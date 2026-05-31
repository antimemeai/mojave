#![allow(clippy::unwrap_used, clippy::expect_used)]

//! TCK integration tests for QMU conformity assessment.
//!
//! Scenarios mirror `tck/eval-orchestrator/features/qmu.feature`.
//! Sources: Pilch et al. 2006 SAND2006-5001, JCGM 106:2012.

use eval_orchestrator::qmu::{ConformityDecision, QmuAssessment};
use eval_orchestrator::{SequentialSummary, SeriesKey};

// ---------------------------------------------------------------------------
// Gate 1: Textbook reproduction
// ---------------------------------------------------------------------------

#[test]
fn cr_clear_acceptance() {
    let a = QmuAssessment::evaluate(0.82, 0.04, 0.70, None).unwrap();
    assert!((a.margin - 0.12).abs() < 1e-10);
    assert!((a.confidence_ratio - 3.0).abs() < 1e-10);
    assert!(matches!(a.decision, ConformityDecision::Accept));
}

#[test]
fn cr_clear_rejection() {
    let a = QmuAssessment::evaluate(0.65, 0.04, 0.70, None).unwrap();
    assert!((a.margin - (-0.05)).abs() < 1e-10);
    assert!((a.confidence_ratio - (-1.25)).abs() < 1e-10);
    assert!(matches!(a.decision, ConformityDecision::Reject));
}

#[test]
fn cr_borderline_investigate() {
    let a = QmuAssessment::evaluate(0.73, 0.04, 0.70, None).unwrap();
    assert!((a.margin - 0.03).abs() < 1e-10);
    assert!((a.confidence_ratio - 0.75).abs() < 1e-10);
    assert!(matches!(a.decision, ConformityDecision::Investigate { .. }));
}

// ---------------------------------------------------------------------------
// JCGM 106 Section 8.3: Guard band decision rules
// ---------------------------------------------------------------------------

#[test]
fn guarded_acceptance_clear() {
    let a = QmuAssessment::evaluate(0.82, 0.04, 0.70, Some(0.04)).unwrap();
    assert!(matches!(a.decision, ConformityDecision::Accept));
}

#[test]
fn guarded_acceptance_marginal_investigate() {
    let a = QmuAssessment::evaluate(0.76, 0.04, 0.70, Some(0.04)).unwrap();
    assert!(matches!(a.decision, ConformityDecision::Investigate { .. }));
}

// ---------------------------------------------------------------------------
// JCGM 106 guard band computation from consumer risk
// ---------------------------------------------------------------------------

#[test]
fn guard_band_iso14253_default() {
    // ISO 14253-1: r=1, g=U, consumer risk ~2.3%
    let g = eval_orchestrator::qmu::jcgm106_guard_band(0.10, 2.0, 0.023).unwrap();
    assert!((g - 0.10).abs() < 0.005, "guard band {g} should be ~0.10");
}

#[test]
fn guard_band_five_percent_risk() {
    // consumer_risk=0.05: r = Phi^-1(0.95)/k = 1.645/2 = 0.8225, g = 0.8225 * 0.10
    let g = eval_orchestrator::qmu::jcgm106_guard_band(0.10, 2.0, 0.05).unwrap();
    assert!(
        (g - 0.0823).abs() < 0.005,
        "guard band {g} should be ~0.0823"
    );
}

// ---------------------------------------------------------------------------
// Gate 3: Property tests
// ---------------------------------------------------------------------------

#[test]
fn cr_monotone_increasing_with_margin() {
    let estimates = [0.65, 0.70, 0.75, 0.80, 0.85];
    let crs: Vec<f64> = estimates
        .iter()
        .map(|&e| QmuAssessment::evaluate(e, 0.04, 0.70, None).unwrap().confidence_ratio)
        .collect();
    for w in crs.windows(2) {
        assert!(
            w[1] > w[0],
            "CR should increase: {:.4} -> {:.4}",
            w[0],
            w[1]
        );
    }
}

#[test]
fn cr_monotone_decreasing_with_uncertainty() {
    let uncertainties = [0.02, 0.04, 0.06, 0.08];
    let crs: Vec<f64> = uncertainties
        .iter()
        .map(|&u| QmuAssessment::evaluate(0.80, u, 0.70, None).unwrap().confidence_ratio)
        .collect();
    for w in crs.windows(2) {
        assert!(
            w[1] < w[0],
            "CR should decrease: {:.4} -> {:.4}",
            w[0],
            w[1]
        );
    }
}

#[test]
fn zero_uncertainty_infinite_cr() {
    let a = QmuAssessment::evaluate(0.80, 0.0, 0.70, None).unwrap();
    assert!(a.confidence_ratio.is_infinite() && a.confidence_ratio > 0.0);
    assert!(matches!(a.decision, ConformityDecision::Accept));
}

#[test]
fn guard_band_zero_equals_no_guard_band() {
    let with_zero = QmuAssessment::evaluate(0.75, 0.04, 0.70, Some(0.0)).unwrap();
    let without = QmuAssessment::evaluate(0.75, 0.04, 0.70, None).unwrap();
    assert_eq!(
        std::mem::discriminant(&with_zero.decision),
        std::mem::discriminant(&without.decision)
    );
    assert!((with_zero.confidence_ratio - without.confidence_ratio).abs() < 1e-10);
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn estimate_exactly_at_threshold() {
    let a = QmuAssessment::evaluate(0.70, 0.04, 0.70, None).unwrap();
    assert!((a.margin).abs() < 1e-10);
    assert!((a.confidence_ratio).abs() < 1e-10);
    assert!(matches!(a.decision, ConformityDecision::Investigate { .. }));
}

#[test]
fn negative_margin_with_guard_band() {
    // estimate=0.65, U=0.04 -> CI upper = 0.69 < threshold=0.70 -> Reject
    let a = QmuAssessment::evaluate(0.65, 0.04, 0.70, Some(0.02)).unwrap();
    assert!(a.margin < 0.0);
    assert!(matches!(a.decision, ConformityDecision::Reject));
}

#[test]
fn negative_margin_ci_straddles_threshold() {
    // estimate=0.68, U=0.04 -> CI = [0.64, 0.72], straddles threshold=0.70
    let a = QmuAssessment::evaluate(0.68, 0.04, 0.70, Some(0.02)).unwrap();
    assert!(a.margin < 0.0);
    assert!(matches!(a.decision, ConformityDecision::Investigate { .. }));
}

#[test]
fn serde_roundtrip() {
    let a = QmuAssessment::evaluate(0.82, 0.04, 0.70, None).unwrap();
    let json = serde_json::to_string(&a).unwrap();
    let back: QmuAssessment = serde_json::from_str(&json).unwrap();
    assert!((back.confidence_ratio - a.confidence_ratio).abs() < 1e-10);
}

// ---------------------------------------------------------------------------
// C-R2: Negative expanded_uncertainty rejected
// ---------------------------------------------------------------------------

#[test]
fn evaluate_rejects_negative_uncertainty() {
    let result = QmuAssessment::evaluate(0.80, -0.04, 0.70, None);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// C2: Pipeline composition -- from_pipeline()
// ---------------------------------------------------------------------------

fn make_sequential_summary(ci_lo: f64, ci_hi: f64) -> SequentialSummary {
    SequentialSummary {
        series: SeriesKey {
            task_id: "task".into(),
            agent_id: "agent".into(),
            scorer: None,
        },
        n_observations: 200,
        current_estimate: (ci_lo + ci_hi) / 2.0,
        ci: (ci_lo, ci_hi),
        evidence: 100.0,
        stopped: true,
    }
}

#[test]
fn from_pipeline_clear_accept() {
    // CI = (0.78, 0.86) -> estimate=0.82, U=0.04, threshold=0.70
    // margin=0.12, CR=3.0, CI lower bound 0.78 > 0.70 -> Accept
    let summary = make_sequential_summary(0.78, 0.86);
    let a = QmuAssessment::from_pipeline(&summary, 0.70, None).unwrap();
    assert!((a.estimate - 0.82).abs() < 1e-10);
    assert!((a.expanded_uncertainty - 0.04).abs() < 1e-10);
    assert!((a.margin - 0.12).abs() < 1e-10);
    assert!((a.confidence_ratio - 3.0).abs() < 1e-10);
    assert!(matches!(a.decision, ConformityDecision::Accept));
}

#[test]
fn from_pipeline_with_guard_band_investigate() {
    // CI = (0.68, 0.76) -> estimate=0.72, U=0.04, threshold=0.70, guard=0.04
    // acceptance_limit=0.74, estimate-U=0.68 < 0.74 -> Investigate
    let summary = make_sequential_summary(0.68, 0.76);
    let a = QmuAssessment::from_pipeline(&summary, 0.70, Some(0.04)).unwrap();
    assert!((a.estimate - 0.72).abs() < 1e-10);
    assert!((a.expanded_uncertainty - 0.04).abs() < 1e-10);
    assert!(matches!(a.decision, ConformityDecision::Investigate { .. }));
}

#[test]
fn from_pipeline_clear_reject() {
    // CI = (0.60, 0.68) -> estimate=0.64, U=0.04, threshold=0.70
    // estimate+U=0.68 < 0.70 -> Reject
    let summary = make_sequential_summary(0.60, 0.68);
    let a = QmuAssessment::from_pipeline(&summary, 0.70, None).unwrap();
    assert!((a.margin - (-0.06)).abs() < 1e-10);
    assert!(matches!(a.decision, ConformityDecision::Reject));
}

#[test]
fn from_pipeline_degenerate_ci() {
    // CI = (0.80, 0.80) -> estimate=0.80, U=0.0, threshold=0.70
    // Zero uncertainty, positive margin -> Accept with infinite CR
    let summary = make_sequential_summary(0.80, 0.80);
    let a = QmuAssessment::from_pipeline(&summary, 0.70, None).unwrap();
    assert!((a.expanded_uncertainty).abs() < 1e-10);
    assert!(a.confidence_ratio.is_infinite() && a.confidence_ratio > 0.0);
    assert!(matches!(a.decision, ConformityDecision::Accept));
}

#[test]
fn from_pipeline_matches_evaluate() {
    // from_pipeline should produce identical results to evaluate with
    // the same derived parameters.
    let summary = make_sequential_summary(0.75, 0.85);
    let from_pipe = QmuAssessment::from_pipeline(&summary, 0.70, Some(0.02)).unwrap();
    let direct = QmuAssessment::evaluate(0.80, 0.05, 0.70, Some(0.02)).unwrap();
    assert!((from_pipe.estimate - direct.estimate).abs() < 1e-10);
    assert!(
        (from_pipe.expanded_uncertainty - direct.expanded_uncertainty).abs() < 1e-10
    );
    assert!((from_pipe.margin - direct.margin).abs() < 1e-10);
    assert!((from_pipe.confidence_ratio - direct.confidence_ratio).abs() < 1e-10);
    assert_eq!(
        std::mem::discriminant(&from_pipe.decision),
        std::mem::discriminant(&direct.decision)
    );
}

// ---------------------------------------------------------------------------
// C-R1: from_pipeline rejects inverted CIs
// ---------------------------------------------------------------------------

#[test]
fn from_pipeline_rejects_inverted_ci() {
    let summary = make_sequential_summary(0.90, 0.70); // ci_hi < ci_lo
    let result = QmuAssessment::from_pipeline(&summary, 0.70, None);
    assert!(result.is_err());
}
