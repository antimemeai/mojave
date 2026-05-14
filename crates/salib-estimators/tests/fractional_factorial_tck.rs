#![allow(
    clippy::float_cmp,
    clippy::approx_constant,
    clippy::expect_used,
    clippy::unwrap_used
)]

use salib_core::{Distribution, ProblemBuilder};
use salib_estimators::estimate_fractional_factorial;
use salib_samplers::build_plackett_burman;

/// Linear model: f(x) = 2x1 + 3x2 + 0.5x3 on [-1,+1]^3.
/// Main effects in coded space: effect_i = 2 * coefficient_i (contrast of +1 vs -1).
/// So effect_0 = 4.0, effect_1 = 6.0, effect_2 = 1.0.
#[test]
fn linear_model_recovers_exact_main_effects() {
    let problem = ProblemBuilder::new()
        .factor("x1", Distribution::Uniform { lo: -1.0, hi: 1.0 })
        .factor("x2", Distribution::Uniform { lo: -1.0, hi: 1.0 })
        .factor("x3", Distribution::Uniform { lo: -1.0, hi: 1.0 })
        .build()
        .unwrap();
    let design = build_plackett_burman(3).unwrap();
    let effects =
        estimate_fractional_factorial(&design, &problem, |x| 2.0 * x[0] + 3.0 * x[1] + 0.5 * x[2]);
    assert!(
        (effects.main_effects[0] - 4.0).abs() < 0.01,
        "effect[0] = {}",
        effects.main_effects[0]
    );
    assert!(
        (effects.main_effects[1] - 6.0).abs() < 0.01,
        "effect[1] = {}",
        effects.main_effects[1]
    );
    assert!(
        (effects.main_effects[2] - 1.0).abs() < 0.01,
        "effect[2] = {}",
        effects.main_effects[2]
    );
}

/// Nonlinear model: f(x) = x1^2 + 5*x2 + 0.5*x3 on [-1,+1]^3.
/// X1's quadratic term is even, so its main effect is zero.
/// X2 has the largest main effect (10.0 in coded space).
/// Demonstrates that PB screening correctly ranks factors even with
/// nonlinear (but odd-asymmetric) dominant terms.
#[test]
fn nonlinear_screening_ranks_x2_highest() {
    let problem = ProblemBuilder::new()
        .factor("x1", Distribution::Uniform { lo: -1.0, hi: 1.0 })
        .factor("x2", Distribution::Uniform { lo: -1.0, hi: 1.0 })
        .factor("x3", Distribution::Uniform { lo: -1.0, hi: 1.0 })
        .build()
        .unwrap();
    let design = build_plackett_burman(3).unwrap();
    let effects =
        estimate_fractional_factorial(&design, &problem, |x| x[0] * x[0] + 5.0 * x[1] + 0.5 * x[2]);
    let max_idx = effects
        .main_effects_abs
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap()
        .0;
    assert_eq!(
        max_idx, 1,
        "X2 (index 1) should have largest |effect|, got index {max_idx}"
    );
}

/// 5-factor model where only x1 and x2 matter: inactive factors should have near-zero effects.
#[test]
fn inactive_factors_have_near_zero_effects() {
    let problem = ProblemBuilder::new()
        .factor("x1", Distribution::Uniform { lo: -1.0, hi: 1.0 })
        .factor("x2", Distribution::Uniform { lo: -1.0, hi: 1.0 })
        .factor("x3", Distribution::Uniform { lo: -1.0, hi: 1.0 })
        .factor("x4", Distribution::Uniform { lo: -1.0, hi: 1.0 })
        .factor("x5", Distribution::Uniform { lo: -1.0, hi: 1.0 })
        .build()
        .unwrap();
    let design = build_plackett_burman(5).unwrap();
    let effects = estimate_fractional_factorial(&design, &problem, |x| x[0] - x[1]);
    for j in 2..5 {
        assert!(
            effects.main_effects[j].abs() < 0.5,
            "factor {j} effect = {} should be near zero",
            effects.main_effects[j]
        );
    }
}
