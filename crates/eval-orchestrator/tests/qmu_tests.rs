#![allow(clippy::unwrap_used, clippy::expect_used)]

//! TCK integration tests for QMU conformity assessment.
//!
//! Scenarios mirror `tck/eval-orchestrator/features/qmu.feature`.
//! Sources: Pilch et al. 2006 SAND2006-5001, JCGM 106:2012.

use eval_orchestrator::qmu::{ConformityDecision, QmuAssessment};

// ---------------------------------------------------------------------------
// Gate 1: Textbook reproduction
// ---------------------------------------------------------------------------

#[test]
fn cr_clear_acceptance() {
    let a = QmuAssessment::evaluate(0.82, 0.04, 0.70, None);
    assert!((a.margin - 0.12).abs() < 1e-10);
    assert!((a.confidence_ratio - 3.0).abs() < 1e-10);
    assert!(matches!(a.decision, ConformityDecision::Accept));
}

#[test]
fn cr_clear_rejection() {
    let a = QmuAssessment::evaluate(0.65, 0.04, 0.70, None);
    assert!((a.margin - (-0.05)).abs() < 1e-10);
    assert!((a.confidence_ratio - (-1.25)).abs() < 1e-10);
    assert!(matches!(a.decision, ConformityDecision::Reject));
}

#[test]
fn cr_borderline_investigate() {
    let a = QmuAssessment::evaluate(0.73, 0.04, 0.70, None);
    assert!((a.margin - 0.03).abs() < 1e-10);
    assert!((a.confidence_ratio - 0.75).abs() < 1e-10);
    assert!(matches!(a.decision, ConformityDecision::Investigate { .. }));
}

// ---------------------------------------------------------------------------
// JCGM 106 Section 8.3: Guard band decision rules
// ---------------------------------------------------------------------------

#[test]
fn guarded_acceptance_clear() {
    let a = QmuAssessment::evaluate(0.82, 0.04, 0.70, Some(0.04));
    assert!(matches!(a.decision, ConformityDecision::Accept));
}

#[test]
fn guarded_acceptance_marginal_investigate() {
    let a = QmuAssessment::evaluate(0.76, 0.04, 0.70, Some(0.04));
    assert!(matches!(a.decision, ConformityDecision::Investigate { .. }));
}

// ---------------------------------------------------------------------------
// JCGM 106 guard band computation from consumer risk
// ---------------------------------------------------------------------------

#[test]
fn guard_band_iso14253_default() {
    // ISO 14253-1: r=1, g=U, consumer risk ~2.3%
    let g = eval_orchestrator::qmu::jcgm106_guard_band(0.10, 2.0, 0.023);
    assert!((g - 0.10).abs() < 0.005, "guard band {g} should be ~0.10");
}

#[test]
fn guard_band_five_percent_risk() {
    // consumer_risk=0.05: r = Phi^-1(0.95)/k = 1.645/2 = 0.8225, g = 0.8225 * 0.10
    let g = eval_orchestrator::qmu::jcgm106_guard_band(0.10, 2.0, 0.05);
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
        .map(|&e| QmuAssessment::evaluate(e, 0.04, 0.70, None).confidence_ratio)
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
        .map(|&u| QmuAssessment::evaluate(0.80, u, 0.70, None).confidence_ratio)
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
    let a = QmuAssessment::evaluate(0.80, 0.0, 0.70, None);
    assert!(a.confidence_ratio.is_infinite() && a.confidence_ratio > 0.0);
    assert!(matches!(a.decision, ConformityDecision::Accept));
}

#[test]
fn guard_band_zero_equals_no_guard_band() {
    let with_zero = QmuAssessment::evaluate(0.75, 0.04, 0.70, Some(0.0));
    let without = QmuAssessment::evaluate(0.75, 0.04, 0.70, None);
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
    let a = QmuAssessment::evaluate(0.70, 0.04, 0.70, None);
    assert!((a.margin).abs() < 1e-10);
    assert!((a.confidence_ratio).abs() < 1e-10);
    assert!(matches!(a.decision, ConformityDecision::Investigate { .. }));
}

#[test]
fn negative_margin_with_guard_band() {
    // estimate=0.65, U=0.04 → CI upper = 0.69 < threshold=0.70 → Reject
    let a = QmuAssessment::evaluate(0.65, 0.04, 0.70, Some(0.02));
    assert!(a.margin < 0.0);
    assert!(matches!(a.decision, ConformityDecision::Reject));
}

#[test]
fn negative_margin_ci_straddles_threshold() {
    // estimate=0.68, U=0.04 → CI = [0.64, 0.72], straddles threshold=0.70
    let a = QmuAssessment::evaluate(0.68, 0.04, 0.70, Some(0.02));
    assert!(a.margin < 0.0);
    assert!(matches!(a.decision, ConformityDecision::Investigate { .. }));
}

#[test]
fn serde_roundtrip() {
    let a = QmuAssessment::evaluate(0.82, 0.04, 0.70, None);
    let json = serde_json::to_string(&a).unwrap();
    let back: QmuAssessment = serde_json::from_str(&json).unwrap();
    assert!((back.confidence_ratio - a.confidence_ratio).abs() < 1e-10);
}
