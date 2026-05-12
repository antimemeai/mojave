use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricLevel {
    Nominal,
    Ordinal,
    Interval,
    Ratio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationTriple {
    pub item_id: String,
    pub annotator_id: String,
    /// Currently u32; interval/ratio scales with fractional or negative
    /// values will require a future generalization.
    pub label: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RatingMatrix {
    pub items: Vec<String>,
    pub raters: Vec<String>,
    /// ratings[item_idx][rater_idx] = Some(label) or None if missing
    pub ratings: Vec<Vec<Option<u32>>>,
}

#[derive(Debug, thiserror::Error)]
pub enum RatingMatrixError {
    #[error("duplicate annotation for item {item_id:?} by rater {rater_id:?}")]
    DuplicateAnnotation { item_id: String, rater_id: String },
}

impl RatingMatrix {
    pub fn from_triples(triples: &[AnnotationTriple]) -> Result<Self, RatingMatrixError> {
        let mut item_set: BTreeMap<&str, usize> = BTreeMap::new();
        let mut rater_set: BTreeMap<&str, usize> = BTreeMap::new();

        for t in triples {
            let next_item = item_set.len();
            item_set.entry(&t.item_id).or_insert(next_item);
            let next_rater = rater_set.len();
            rater_set.entry(&t.annotator_id).or_insert(next_rater);
        }

        let mut items: Vec<(usize, String)> =
            item_set.iter().map(|(&k, &v)| (v, k.to_string())).collect();
        items.sort();
        let items: Vec<String> = items.into_iter().map(|(_, s)| s).collect();

        let mut raters: Vec<(usize, String)> = rater_set
            .iter()
            .map(|(&k, &v)| (v, k.to_string()))
            .collect();
        raters.sort();
        let raters: Vec<String> = raters.into_iter().map(|(_, s)| s).collect();

        let mut ratings = vec![vec![None; rater_set.len()]; item_set.len()];
        for t in triples {
            let i = item_set[t.item_id.as_str()];
            let j = rater_set[t.annotator_id.as_str()];
            if ratings[i][j].is_some() {
                return Err(RatingMatrixError::DuplicateAnnotation {
                    item_id: t.item_id.clone(),
                    rater_id: t.annotator_id.clone(),
                });
            }
            ratings[i][j] = Some(t.label);
        }

        Ok(Self {
            items,
            raters,
            ratings,
        })
    }

    pub fn n_items(&self) -> usize {
        self.items.len()
    }

    pub fn n_raters(&self) -> usize {
        self.raters.len()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[must_use]
pub struct IrrResult {
    pub statistic_name: String,
    pub value: f64,
    pub ci_lower: Option<f64>,
    pub ci_upper: Option<f64>,
    pub n_items: usize,
    pub n_raters: usize,
    pub metric_level: Option<MetricLevel>,
}
