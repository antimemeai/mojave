#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};
use seq_anytime_valid::boundary::{obf, pocock};

fn feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("seq-anytime-valid")
        .join("features")
        .join("group_sequential.feature")
}

#[derive(Default, Debug)]
struct GroupSeqWorld {
    obf_boundaries: Option<Vec<f64>>,
    pocock_boundaries: Option<Vec<f64>>,
    // for the K=1 comparison scenario
    pocock_k1: Option<f64>,
    obf_k1: Option<f64>,
}

#[test]
fn group_sequential_feature_runs_end_to_end() {
    let path = feature_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read feature: {e}"));
    let feature =
        parse_feature(&content, "group_sequential.feature").expect("feature parses cleanly");

    let runner = SyncRunner::new(GroupSeqWorld::default)
        // --- Given steps ---
        .step("a 5-look OBF design at alpha = 0.05", |w, _| {
            w.obf_boundaries =
                Some(obf::boundaries(5, 0.05).map_err(|e| StepError::new(e.to_string()))?);
            Ok(())
        })
        .step("a 1-look OBF design at alpha = 0.05", |w, _| {
            let bs = obf::boundaries(1, 0.05).map_err(|e| StepError::new(e.to_string()))?;
            w.obf_k1 = Some(bs[0]);
            w.obf_boundaries = Some(bs);
            Ok(())
        })
        .step("a 3-look Pocock design at alpha = 0.05", |w, _| {
            w.pocock_boundaries =
                Some(pocock::boundaries(3, 0.05).map_err(|e| StepError::new(e.to_string()))?);
            Ok(())
        })
        .step("a 1-look Pocock design at alpha = 0.05", |w, _| {
            let bs = pocock::boundaries(1, 0.05).map_err(|e| StepError::new(e.to_string()))?;
            w.pocock_k1 = Some(bs[0]);
            w.pocock_boundaries = Some(bs);
            Ok(())
        })
        // --- When steps ---
        .step("I compute all boundaries", |_, _| {
            // boundaries already computed in Given
            Ok(())
        })
        .step("I compute boundary at look 1", |_, _| {
            // boundary already computed in Given
            Ok(())
        })
        .step("I compute both boundaries", |_, _| {
            // both already computed in Given
            Ok(())
        })
        // --- Then steps ---
        .step("boundary at look 1 > boundary at look 5", |w, _| {
            let bs = w
                .obf_boundaries
                .as_ref()
                .ok_or_else(|| StepError::new("OBF boundaries not computed"))?;
            if bs.len() < 5 {
                return Err(StepError::new(format!(
                    "expected at least 5 boundaries, got {}",
                    bs.len()
                )));
            }
            let b1 = bs[0];
            let b5 = bs[4];
            if b1 > b5 {
                Ok(())
            } else {
                Err(StepError::new(format!(
                    "expected boundary[0]={b1} > boundary[4]={b5}"
                )))
            }
        })
        .step("the boundary is approximately 1.96", |w, _| {
            let b = w
                .obf_k1
                .ok_or_else(|| StepError::new("OBF K=1 boundary not computed"))?;
            let expected = 1.96_f64;
            let tol = 0.05;
            if (b - expected).abs() <= tol {
                Ok(())
            } else {
                Err(StepError::new(format!(
                    "expected boundary ~ {expected}, got {b} (tol={tol})"
                )))
            }
        })
        .step("all boundaries are equal", |w, _| {
            let bs = w
                .pocock_boundaries
                .as_ref()
                .ok_or_else(|| StepError::new("Pocock boundaries not computed"))?;
            if bs.is_empty() {
                return Err(StepError::new("no Pocock boundaries to compare"));
            }
            let first = bs[0];
            for (i, &b) in bs.iter().enumerate() {
                if (b - first).abs() > 1e-10 {
                    return Err(StepError::new(format!(
                        "Pocock boundary[{i}]={b} != boundary[0]={first}"
                    )));
                }
            }
            Ok(())
        })
        .step("they are equal", |w, _| {
            let pc = w
                .pocock_k1
                .ok_or_else(|| StepError::new("Pocock K=1 boundary not computed"))?;
            let ob = w
                .obf_k1
                .ok_or_else(|| StepError::new("OBF K=1 boundary not computed"))?;
            let tol = 0.05;
            if (pc - ob).abs() <= tol {
                Ok(())
            } else {
                Err(StepError::new(format!(
                    "Pocock K=1={pc} and OBF K=1={ob} differ by {} (tol={tol})",
                    (pc - ob).abs()
                )))
            }
        });

    let report = runner.run(&feature);
    report.assert_all_passed();
}
