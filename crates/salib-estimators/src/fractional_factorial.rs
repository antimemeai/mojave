//! Fractional factorial effects estimator — Plackett-Burman screening.
//!
//! Estimates main effects from a Plackett-Burman two-level design.
//! For each factor *i*, the main effect is the difference in mean
//! response between the high (+1) and low (-1) levels:
//!
//! ```text
//! effect_i = mean(Y | x_i = +1) - mean(Y | x_i = -1)
//! ```
//!
//! The design's coded {-1, +1} entries are mapped to the physical
//! factor bounds via `x_physical = lo + (x_coded + 1)/2 * (hi - lo)`.
//!
//! # References
//!
//! - Plackett, R. L. & Burman, J. P. (1946). "The Design of Optimum
//!   Multifactorial Experiments." *Biometrika*, 33(4), 305–325.

use salib_core::Problem;
use salib_samplers::PlackettBurmanDesign;

/// Results of a fractional factorial screening analysis.
#[derive(Debug, Clone)]
pub struct FractionalFactorialEffects {
    /// Number of factors.
    pub dim: usize,
    /// Number of design runs (rows in the PB matrix).
    pub n_runs: usize,
    /// Main effect for each factor (signed).
    pub main_effects: Vec<f64>,
    /// Absolute value of each main effect (for ranking).
    pub main_effects_abs: Vec<f64>,
}

/// Estimate main effects from a Plackett-Burman design.
///
/// For each factor i, main_effect_i = mean(Y where x_i = +1) - mean(Y where x_i = -1).
/// The model function receives inputs mapped from the design's {-1,+1} coding
/// to the problem's factor bounds: x_physical = lo + (x_coded + 1)/2 * (hi - lo).
pub fn estimate_fractional_factorial<F>(
    design: &PlackettBurmanDesign,
    problem: &Problem,
    model: F,
) -> FractionalFactorialEffects
where
    F: Fn(&[f64]) -> f64,
{
    let n = design.n_runs;
    let d = design.dim;

    // Evaluate model at each design point, mapping coded -> physical.
    let mut x_phys = vec![0.0_f64; d];
    let mut y = Vec::with_capacity(n);
    for i in 0..n {
        for (j, xp) in x_phys.iter_mut().enumerate() {
            let coded = design.matrix[[i, j]];
            let (lo, hi) = problem.factors()[j].distribution.support();
            *xp = lo + (coded + 1.0) / 2.0 * (hi - lo);
        }
        y.push(model(&x_phys));
    }

    // Main effect for factor j = mean(Y where x_j=+1) - mean(Y where x_j=-1).
    let main_effects: Vec<f64> = (0..d)
        .map(|j| {
            let mut sum_plus = 0.0_f64;
            let mut count_plus = 0usize;
            let mut sum_minus = 0.0_f64;
            let mut count_minus = 0usize;
            for (i, &yi) in y.iter().enumerate() {
                if design.matrix[[i, j]] > 0.0 {
                    sum_plus += yi;
                    count_plus += 1;
                } else {
                    sum_minus += yi;
                    count_minus += 1;
                }
            }
            sum_plus / count_plus as f64 - sum_minus / count_minus as f64
        })
        .collect();

    let main_effects_abs: Vec<f64> = main_effects.iter().map(|e| e.abs()).collect();

    FractionalFactorialEffects {
        dim: d,
        n_runs: n,
        main_effects,
        main_effects_abs,
    }
}
