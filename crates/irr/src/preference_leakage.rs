/// Preference Leakage Score (PLS) for LLM-as-a-judge bias detection.
///
/// Measures how much judge models favor student models from related
/// data generators. Implements equations 5-6 from Li et al. 2025
/// (ICLR 2026).
///
/// WR(i,j) = win rate of student model i judged by model j.
/// PLS(i,j) = [(WR(i,i)-AVG(i,j))/AVG(i,j) + (WR(j,j)-AVG(j,i))/AVG(j,i)] / 2
/// AVG(i,j) = [WR(i,i) + WR(i,j)] / 2
use crate::types::{PlsPair, PreferenceLeakageResult, RegimeMean, RelatednessRegime};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum PlsError {
    #[error("empty win-rate matrix")]
    EmptyData,
    #[error("win-rate matrix is not square")]
    NotSquare,
    #[error("model {0} not found in family map")]
    UnknownModel(String),
    #[error("win rate at [{row}][{col}] = {value} is not a finite value in [0, 1]")]
    InvalidWinRate { row: usize, col: usize, value: f64 },
    #[error("degenerate AVG = 0 for pair ({model_i}, {model_j}): PLS is undefined")]
    DegenerateAverage { model_i: String, model_j: String },
}

pub fn compute_pls(
    models: &[String],
    win_rates: &[Vec<f64>],
    family_map: &BTreeMap<String, String>,
) -> Result<PreferenceLeakageResult, PlsError> {
    let n = models.len();
    if n == 0 {
        return Err(PlsError::EmptyData);
    }
    if win_rates.len() != n || win_rates.iter().any(|r| r.len() != n) {
        return Err(PlsError::NotSquare);
    }

    for (i, row) in win_rates.iter().enumerate() {
        for (j, &wr) in row.iter().enumerate() {
            if !wr.is_finite() || !(0.0..=1.0).contains(&wr) {
                return Err(PlsError::InvalidWinRate {
                    row: i,
                    col: j,
                    value: wr,
                });
            }
        }
    }

    for m in models {
        if !family_map.contains_key(m) {
            return Err(PlsError::UnknownModel(m.clone()));
        }
    }

    let mut pairs = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            let family_i = &family_map[&models[i]];
            let family_j = &family_map[&models[j]];

            let regime = if family_i == family_j {
                RelatednessRegime::SameFamily
            } else {
                RelatednessRegime::CrossFamily
            };

            // Li et al. 2025, eq 6: AVG(i,j) = [WR(i,i) + WR(i,j)] / 2
            let avg_ij = (win_rates[i][i] + win_rates[i][j]) / 2.0;
            let avg_ji = (win_rates[j][j] + win_rates[j][i]) / 2.0;

            if avg_ij.abs() < 1e-15 || avg_ji.abs() < 1e-15 {
                return Err(PlsError::DegenerateAverage {
                    model_i: models[i].clone(),
                    model_j: models[j].clone(),
                });
            }

            // Li et al. 2025, eq 5
            let term_i = (win_rates[i][i] - avg_ij) / avg_ij;
            let term_j = (win_rates[j][j] - avg_ji) / avg_ji;
            let pls = (term_i + term_j) / 2.0;

            pairs.push(PlsPair {
                model_i: models[i].clone(),
                model_j: models[j].clone(),
                pls,
                regime,
            });
        }
    }

    let mut regime_sums: BTreeMap<RelatednessRegime, (f64, usize)> = BTreeMap::new();
    for pair in &pairs {
        let entry = regime_sums.entry(pair.regime).or_insert((0.0, 0));
        entry.0 += pair.pls;
        entry.1 += 1;
    }

    let regime_means: Vec<RegimeMean> = regime_sums
        .into_iter()
        .map(|(regime, (sum, count))| RegimeMean {
            regime,
            mean_pls: sum / count as f64,
            n_pairs: count,
        })
        .collect();

    let global_mean_pls = if pairs.is_empty() {
        0.0
    } else {
        pairs.iter().map(|p| p.pls).sum::<f64>() / pairs.len() as f64
    };

    Ok(PreferenceLeakageResult {
        pls_scores: pairs,
        regime_means,
        global_mean_pls,
    })
}
