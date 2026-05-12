//! Shapley-effects estimator (Song-Nelson-Staum 2016 Algorithm 1).
//!
//! The compact form of Algorithm 1 (independent-inputs case):
//!
//! ```text
//! 1. Var-only block — sample N_V points from G_K, evaluate Y, get
//!    Ȳ and V̂ar[Y].
//! 2. For ℓ = 1..m (permutations):
//!     - Generate random π ∈ Π(K).
//!     - prevC = 0
//!     - For j = 1..k:
//!         - If j == k: ĉ = V̂ar[Y]                     (boundary)
//!           Else:
//!             ĉ ← double-loop MC of E[Var(Y | X_{-J})]
//!                 J = {π(1), …, π(j)}
//!                 outer N_O samples of X_{-J}
//!                 inner N_I samples of X_J fixed to outer X_{-J}
//!                 — for independent inputs, both reduce to
//!                 independent per-factor draws via Distribution.
//!         - Δ̂ = ĉ − prevC
//!         - Sh_{π(j)} += Δ̂
//!         - prevC = ĉ
//! 3. Sh_i /= m for i = 1..k.
//! ```
//!
//! `Σ Sh_i ≈ V̂ar[Y]` exactly when the per-permutation telescoping
//! `Σ_j Δ̂_j = c(K) − c(∅) = V̂ar[Y] − 0` holds; sampling noise gives
//! a finite-`m` deviation.

#![allow(
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::too_many_arguments
)]

use rand::RngCore;
use salib_core::{tree_sum, tree_var, Distribution, RngState};

/// Shapley-effects estimate for a model on independent inputs.
///
/// `#[non_exhaustive]` — future fields (per-permutation trace,
/// bootstrap CIs, dependent-input metadata) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ShapleyIndices {
    /// `Sh_i` per factor, length `k`. Sums to `var_y` (modulo MC
    /// noise). Each `Sh_i ≥ 0` in expectation; Song 2016 Theorem 3
    /// bounds the variance.
    pub sh: Vec<f64>,
    /// Total variance of `Y` estimated from the var-only block.
    pub var_y: f64,
    /// Number of permutations `m` used. Echo of input.
    pub n_perm: usize,
}

impl ShapleyIndices {
    /// Factor count `k`.
    #[must_use]
    pub fn k(&self) -> usize {
        self.sh.len()
    }
}

/// Errors from [`estimate_shapley`].
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ShapleyError {
    #[error("shapley: factor count k must be ≥ 1, got 0")]
    ZeroFactors,
    #[error("shapley: n_perm must be ≥ 1, got 0")]
    ZeroPermutations,
    #[error("shapley: n_outer must be ≥ 1, got 0")]
    ZeroOuter,
    #[error("shapley: n_inner must be ≥ 2 (sample variance ill-defined for n < 2), got {n_inner}")]
    InsufficientInner { n_inner: usize },
    #[error("shapley: n_var must be ≥ 2, got {n_var}")]
    InsufficientVar { n_var: usize },
}

/// Estimate Shapley effects on independent inputs via Song 2016
/// Algorithm 1.
///
/// `distributions` defines the per-factor independent input
/// distributions (length `k`). `model` evaluates `Y = η(X)` from a
/// length-`k` slice of factor values. `n_perm` is the permutation-
/// sampling count `m`; `n_outer` is `N_O`; `n_inner` is `N_I`.
/// `n_var` is `N_V` for the variance-only block.
///
/// Recommended budget per Song 2016 Appendix B: `n_inner = 3,
/// n_outer = 1`, `n_perm` consuming the remaining computational
/// budget, `n_var ≥ 1000` for a stable variance estimate.
///
/// # Errors
///
/// - [`ShapleyError::ZeroFactors`] if `distributions.is_empty()`.
/// - [`ShapleyError::ZeroPermutations`] if `n_perm == 0`.
/// - [`ShapleyError::ZeroOuter`] if `n_outer == 0`.
/// - [`ShapleyError::InsufficientInner`] if `n_inner < 2`.
/// - [`ShapleyError::InsufficientVar`] if `n_var < 2`.
pub fn estimate_shapley<F>(
    distributions: &[Distribution],
    mut model: F,
    n_perm: usize,
    n_outer: usize,
    n_inner: usize,
    n_var: usize,
    rng: &mut RngState,
) -> Result<ShapleyIndices, ShapleyError>
where
    F: FnMut(&[f64]) -> f64,
{
    let k = distributions.len();
    if k == 0 {
        return Err(ShapleyError::ZeroFactors);
    }
    if n_perm == 0 {
        return Err(ShapleyError::ZeroPermutations);
    }
    if n_outer == 0 {
        return Err(ShapleyError::ZeroOuter);
    }
    if n_inner < 2 {
        return Err(ShapleyError::InsufficientInner { n_inner });
    }
    if n_var < 2 {
        return Err(ShapleyError::InsufficientVar { n_var });
    }

    let mut chacha = rng.clone().into_chacha();

    // ── Var-only block (Algorithm 1 step 2-3) ────────────────────
    let var_y = sample_var_y(distributions, &mut model, n_var, &mut chacha);

    // ── Permutation sweep (Algorithm 1 step 4) ───────────────────
    let mut sh_sum = vec![0.0_f64; k];
    let mut x_buf = vec![0.0_f64; k]; // reusable buffer for model
    let mut x_outer = vec![0.0_f64; k]; // reusable per-outer X_{-J}
    let mut y_inner = Vec::with_capacity(n_inner);
    let mut var_per_outer = Vec::with_capacity(n_outer);

    for _ell in 0..n_perm {
        let pi = random_permutation(k, &mut chacha);

        let mut prev_c = 0.0_f64;
        for j in 1..=k {
            // The j == k boundary uses Var(Y) directly without
            // re-running the double-loop MC — matches Algorithm 1
            // step 4(c)(i).
            let c_j = if j == k {
                var_y
            } else {
                // Coalition J = {π(0)..π(j-1)} (zero-indexed).
                // Complement: {π(j)..π(k-1)}. We sample X_{-J} (the
                // complement coordinates) for each outer step, then
                // sample X_J (the J coordinates) for each inner step.
                // For independent inputs both are independent draws
                // — the only difference is which coordinates get
                // refreshed at the inner step.
                cost_via_double_loop(
                    distributions,
                    &mut model,
                    &pi[..j], // J = first j elements of pi
                    &pi[j..], // complement of J
                    n_outer,
                    n_inner,
                    &mut chacha,
                    &mut x_buf,
                    &mut x_outer,
                    &mut y_inner,
                    &mut var_per_outer,
                )
            };

            let delta = c_j - prev_c;
            sh_sum[pi[j - 1]] += delta;
            prev_c = c_j;
        }
    }

    let mut sh: Vec<f64> = sh_sum.iter().map(|&s| s / n_perm as f64).collect();
    // Numerical hygiene: tiny negative drift can come from MC noise
    // on a near-zero true Sh_i. Clamp at 0 — Song 2016 § 4.1 notes
    // Sh_i ≥ 0 in expectation and a small negative empirical value
    // is a noise artifact, not signal. (We do NOT renormalize so
    // the user can audit Σ Sh_i vs Var(Y) directly.)
    for v in &mut sh {
        if *v < 0.0 && v.abs() < 1e-10 * var_y.max(1.0) {
            *v = 0.0;
        }
    }

    *rng = RngState::snapshot(&chacha, rng);

    Ok(ShapleyIndices { sh, var_y, n_perm })
}

/// Draw `N_V` independent samples from `G_K`, evaluate `Y`, return
/// sample variance via the workspace-provided `tree_var` reduction.
fn sample_var_y<F>(
    distributions: &[Distribution],
    model: &mut F,
    n_var: usize,
    chacha: &mut rand_chacha::ChaCha20Rng,
) -> f64
where
    F: FnMut(&[f64]) -> f64,
{
    let k = distributions.len();
    let mut x = vec![0.0_f64; k];
    let mut ys = Vec::with_capacity(n_var);
    for _q in 0..n_var {
        for i in 0..k {
            let u = uniform_01(chacha);
            x[i] = distributions[i].quantile(u);
        }
        ys.push(model(&x));
    }
    tree_var(&ys)
}

/// Estimate `c(J) = E_{X_{-J}}[Var_{X_J}(Y | X_{-J})]` via the Song
/// 2016 § 4.2 double-loop Monte Carlo. Independent-inputs case: the
/// "conditional sampling" of `X_J | X_{-J}` reduces to independent
/// draws on `J`'s factors.
#[allow(clippy::too_many_lines)]
fn cost_via_double_loop<F>(
    distributions: &[Distribution],
    model: &mut F,
    j_indices: &[usize],
    complement_indices: &[usize],
    n_outer: usize,
    n_inner: usize,
    chacha: &mut rand_chacha::ChaCha20Rng,
    x_buf: &mut [f64],
    x_outer: &mut [f64],
    y_inner: &mut Vec<f64>,
    var_per_outer: &mut Vec<f64>,
) -> f64
where
    F: FnMut(&[f64]) -> f64,
{
    var_per_outer.clear();
    for _l in 0..n_outer {
        // Outer: sample X_{-J} (the complement).
        for &i in complement_indices {
            let u = uniform_01(chacha);
            x_outer[i] = distributions[i].quantile(u);
        }
        // Inner: for each h, redraw X_J independently (under
        // independence, `X_J | X_{-J}` is the marginal on J).
        y_inner.clear();
        for _h in 0..n_inner {
            // Start from x_outer (carrying complement values).
            for &i in complement_indices {
                x_buf[i] = x_outer[i];
            }
            for &i in j_indices {
                let u = uniform_01(chacha);
                x_buf[i] = distributions[i].quantile(u);
            }
            y_inner.push(model(x_buf));
        }
        var_per_outer.push(tree_var(y_inner));
    }
    let total: f64 = tree_sum(var_per_outer);
    total / n_outer as f64
}

/// Fisher-Yates random permutation of `0..k`.
fn random_permutation(k: usize, chacha: &mut rand_chacha::ChaCha20Rng) -> Vec<usize> {
    let mut perm: Vec<usize> = (0..k).collect();
    for i in (1..k).rev() {
        // Unbiased index in 0..=i.
        let r = chacha.next_u32() as usize;
        let idx = r % (i + 1);
        perm.swap(i, idx);
    }
    perm
}

/// Uniform `[0, 1)` from a `u32` draw — same conversion the LHS
/// sampler uses for byte-identical RNG-state semantics.
fn uniform_01(chacha: &mut rand_chacha::ChaCha20Rng) -> f64 {
    let u32_norm = 1.0_f64 / (f64::from(u32::MAX) + 1.0);
    f64::from(chacha.next_u32()) * u32_norm
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;

    fn standard_normal_factors(k: usize) -> Vec<Distribution> {
        (0..k)
            .map(|_| Distribution::Normal {
                mu: 0.0,
                sigma: 1.0,
            })
            .collect()
    }

    fn uniform_factors(k: usize, lo: f64, hi: f64) -> Vec<Distribution> {
        (0..k).map(|_| Distribution::Uniform { lo, hi }).collect()
    }

    // ── Validation ────────────────────────────────────────────────

    #[test]
    fn zero_factors_errors() {
        let mut rng = RngState::from_seed([0; 32]);
        let err = estimate_shapley(&[], |_x: &[f64]| 0.0, 100, 1, 3, 1000, &mut rng).unwrap_err();
        assert_eq!(err, ShapleyError::ZeroFactors);
    }

    #[test]
    fn zero_perm_errors() {
        let mut rng = RngState::from_seed([0; 32]);
        let err = estimate_shapley(
            &standard_normal_factors(2),
            |_x: &[f64]| 0.0,
            0,
            1,
            3,
            1000,
            &mut rng,
        )
        .unwrap_err();
        assert_eq!(err, ShapleyError::ZeroPermutations);
    }

    #[test]
    fn insufficient_inner_errors() {
        let mut rng = RngState::from_seed([0; 32]);
        let err = estimate_shapley(
            &standard_normal_factors(2),
            |_x: &[f64]| 0.0,
            10,
            1,
            1,
            1000,
            &mut rng,
        )
        .unwrap_err();
        assert!(matches!(err, ShapleyError::InsufficientInner { .. }));
    }

    // ── Linear-additive closed form ──────────────────────────────

    #[test]
    fn linear_additive_independent_normals_recovers_squared_coefficients() {
        // Y = X_1 + 2·X_2 + 3·X_3, X_i ~ N(0, 1) independent.
        // Var(Y) = 1 + 4 + 9 = 14.
        // Under independence + linearity, Sh_i = a_i² (no
        // interactions, so first-order = total = Shapley).
        let mut rng = RngState::from_seed([42; 32]);
        let result = estimate_shapley(
            &standard_normal_factors(3),
            |x: &[f64]| x[0] + 2.0 * x[1] + 3.0 * x[2],
            500,  // n_perm
            1,    // n_outer
            3,    // n_inner
            4000, // n_var
            &mut rng,
        )
        .unwrap();
        // Loose tolerance: MC noise on Sh at this budget is ~5%.
        assert!(
            (result.var_y - 14.0).abs() < 1.5,
            "var_y = {} (analytic 14)",
            result.var_y
        );
        let expected = [1.0, 4.0, 9.0];
        for (i, &want) in expected.iter().enumerate() {
            assert!(
                (result.sh[i] - want).abs() < 1.5,
                "Sh_{i} = {:.3}, want {} (linear-additive)",
                result.sh[i],
                want
            );
        }
        // Σ Sh_i ≈ Var(Y) — Song 2016 Eq 10.
        let sum: f64 = result.sh.iter().sum();
        assert!(
            (sum - result.var_y).abs() < 0.5,
            "Σ Sh_i = {sum:.3}, Var(Y) = {:.3}",
            result.var_y
        );
    }

    // ── Determinism ──────────────────────────────────────────────

    #[test]
    fn same_rng_yields_identical_indices() {
        let mut rng_a = RngState::from_seed([7; 32]);
        let mut rng_b = RngState::from_seed([7; 32]);
        let a = estimate_shapley(
            &uniform_factors(2, 0.0, 1.0),
            |x: &[f64]| x[0] + x[1],
            64,
            1,
            3,
            500,
            &mut rng_a,
        )
        .unwrap();
        let b = estimate_shapley(
            &uniform_factors(2, 0.0, 1.0),
            |x: &[f64]| x[0] + x[1],
            64,
            1,
            3,
            500,
            &mut rng_b,
        )
        .unwrap();
        assert_eq!(a.sh, b.sh);
        assert_eq!(a.var_y, b.var_y);
    }

    #[test]
    fn rng_state_advances_across_call() {
        let mut rng = RngState::from_seed([0; 32]);
        let before_pos = rng.word_pos;
        let _ = estimate_shapley(
            &uniform_factors(2, 0.0, 1.0),
            |x: &[f64]| x[0] + x[1],
            32,
            1,
            3,
            300,
            &mut rng,
        )
        .unwrap();
        assert!(
            rng.word_pos > before_pos,
            "RngState should advance: before={before_pos}, after={}",
            rng.word_pos
        );
    }

    // ── Permutation generator sanity ─────────────────────────────

    #[test]
    fn random_permutation_is_a_permutation() {
        use rand::SeedableRng;
        let mut chacha = rand_chacha::ChaCha20Rng::from_seed([1; 32]);
        for k in 1..=8 {
            let perm = random_permutation(k, &mut chacha);
            assert_eq!(perm.len(), k);
            let mut sorted = perm.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, (0..k).collect::<Vec<_>>());
        }
    }
}
