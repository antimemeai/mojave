//! Morris elementary-effects estimator — `μ`, `μ*`, `σ` per factor.
//!
//! Per Morris 1991 + Campolongo 2007's `μ*` extension. Given a
//! `MorrisTrajectories` bundle (`R` trajectories of `d + 1` points
//! each on a `d`-factor problem), evaluate the model at every point
//! and compute the per-factor elementary effects:
//!
//! ```text
//! For trajectory r and step k (where factor j_k is varied):
//!   x_before = trajectories[r, k, :]
//!   x_after  = trajectories[r, k+1, :]
//!   delta    = deltas[r, j_k]
//!   EE_{j_k}^{(r)} = (model(x_after) - model(x_before)) / delta
//!
//! Per factor i, collected over the R trajectories that visit i:
//!   μ_i     = mean over r of EE_i^{(r)}
//!   μ*_i    = mean over r of |EE_i^{(r)}|        (Campolongo 2007)
//!   σ_i     = std over r of EE_i^{(r)}            (sample std, dof = R-1)
//! ```
//!
//! # Why `μ*`
//!
//! The original Morris `μ` averages signed effects, which can mask
//! non-monotonic factors via cancellation (positive and negative
//! effects average toward 0 even when the factor is influential).
//! `μ*` (Campolongo 2007) uses absolute values — `|EE|` — and is the
//! modern standard for screening. We compute all three; the report
//! and downstream Severity-test API consume `μ*` as the screening
//! statistic with `σ` as the non-linearity / interaction indicator.
//!
//! # Determinism
//!
//! Pure under `(trajectories, model)`. All sums route through
//! `salib_core::reduce::tree_*`. No RNG; the trajectory sampler
//! consumed `RngState` upstream.
//!
//! # Cost
//!
//! `R · (d + 1)` model evaluations — this is Morris's signature
//! efficiency claim relative to Sobol' (which costs `N · (d + 2)`
//! with N typically 8192 vs Morris's R typically 10–50).

// Standard Morris notation. `mu` / `sigma` / `r`. Naming purity
// fights the paper cross-reference.
#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::expect_used
)]

use salib_core::{tree_sum, Group};
use salib_samplers::MorrisTrajectories;

/// Errors from Morris estimation. `#[non_exhaustive]`.
///
/// Today's only variant is `ZeroTrajectories`; this is unreachable
/// for `MorrisTrajectories` produced by `build_morris_trajectories`
/// (which rejects `r == 0` at construction). Kept here as a guard
/// for paths that bypass the constructor (deserialization, struct-
/// literal construction inside salib-samplers, etc.).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EmptyError {
    #[error("Morris: cannot estimate effects from zero trajectories")]
    ZeroTrajectories,
}

/// Morris elementary-effects output. `μ`, `μ*`, `σ` per factor.
///
/// `#[non_exhaustive]` — future fields (e.g., per-factor R count if
/// trajectories visit factors unevenly) land non-breaking.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MorrisEffects {
    /// Number of trajectories used.
    pub r: usize,
    /// Factor count.
    pub d: usize,
    /// `μ_i = (1/R) Σ_r EE_i^(r)` — signed mean elementary effect.
    /// Can be near zero for non-monotonic factors via cancellation;
    /// prefer `mu_star` for screening.
    pub mu: Vec<f64>,
    /// `μ*_i = (1/R) Σ_r |EE_i^(r)|` — Campolongo 2007's absolute-
    /// value extension. The screening statistic.
    pub mu_star: Vec<f64>,
    /// `σ_i = std(EE_i)` — sample standard deviation of elementary
    /// effects. High `σ_i` indicates non-linearity or interactions.
    pub sigma: Vec<f64>,
    /// Grouped `μ` per group. `None` for ungrouped analyses.
    pub grouped_mu: Option<Vec<f64>>,
    /// Grouped `μ*` per group. `None` for ungrouped analyses.
    pub grouped_mu_star: Option<Vec<f64>>,
    /// Grouped `σ` per group. `None` for ungrouped analyses.
    pub grouped_sigma: Option<Vec<f64>>,
    /// Group names, parallel to `grouped_mu` / `grouped_mu_star` /
    /// `grouped_sigma`. `None` for ungrouped analyses.
    pub group_names: Option<Vec<String>>,
}

impl MorrisEffects {
    /// Construct from raw vectors. Panics if `mu`, `mu_star`, or
    /// `sigma` have lengths different from `d` — the type's
    /// invariant is that all three per-factor vectors have the same
    /// length matching `dim`.
    ///
    /// # Panics
    ///
    /// On `mu.len() != d`, `mu_star.len() != d`, or `sigma.len() != d`.
    #[must_use]
    pub fn new(r: usize, d: usize, mu: Vec<f64>, mu_star: Vec<f64>, sigma: Vec<f64>) -> Self {
        assert_eq!(mu.len(), d, "MorrisEffects::new: mu.len() != d");
        assert_eq!(mu_star.len(), d, "MorrisEffects::new: mu_star.len() != d");
        assert_eq!(sigma.len(), d, "MorrisEffects::new: sigma.len() != d");
        Self {
            r,
            d,
            mu,
            mu_star,
            sigma,
            grouped_mu: None,
            grouped_mu_star: None,
            grouped_sigma: None,
            group_names: None,
        }
    }

    /// Construct with grouped effects. Same invariants as `new()`,
    /// plus the grouped vectors must all have equal length.
    ///
    /// # Panics
    ///
    /// On length mismatches in per-factor or per-group vectors.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new_grouped(
        r: usize,
        d: usize,
        mu: Vec<f64>,
        mu_star: Vec<f64>,
        sigma: Vec<f64>,
        grouped_mu: Vec<f64>,
        grouped_mu_star: Vec<f64>,
        grouped_sigma: Vec<f64>,
        group_names: Vec<String>,
    ) -> Self {
        assert_eq!(mu.len(), d, "MorrisEffects::new_grouped: mu.len() != d");
        assert_eq!(
            mu_star.len(),
            d,
            "MorrisEffects::new_grouped: mu_star.len() != d"
        );
        assert_eq!(
            sigma.len(),
            d,
            "MorrisEffects::new_grouped: sigma.len() != d"
        );
        let n_groups = group_names.len();
        assert_eq!(
            grouped_mu.len(),
            n_groups,
            "MorrisEffects::new_grouped: grouped_mu.len() != n_groups"
        );
        assert_eq!(
            grouped_mu_star.len(),
            n_groups,
            "MorrisEffects::new_grouped: grouped_mu_star.len() != n_groups"
        );
        assert_eq!(
            grouped_sigma.len(),
            n_groups,
            "MorrisEffects::new_grouped: grouped_sigma.len() != n_groups"
        );
        Self {
            r,
            d,
            mu,
            mu_star,
            sigma,
            grouped_mu: Some(grouped_mu),
            grouped_mu_star: Some(grouped_mu_star),
            grouped_sigma: Some(grouped_sigma),
            group_names: Some(group_names),
        }
    }
}

/// Estimate Morris elementary effects from a trajectory bundle and
/// a model. Pure function.
///
/// `model` is called `R · (d + 1)` times.
///
/// # Errors
///
/// Returns `EmptyError` if `trajectories.r == 0` (zero trajectories
/// is degenerate; can't compute mean over zero samples).
#[allow(clippy::many_single_char_names)]
pub fn estimate_morris_effects<F>(
    trajectories: &MorrisTrajectories,
    model: F,
) -> Result<MorrisEffects, EmptyError>
where
    F: Fn(&[f64]) -> f64,
{
    let r = trajectories.r;
    let d = trajectories.d;

    if r == 0 {
        return Err(EmptyError::ZeroTrajectories);
    }

    // Per-factor accumulator of elementary effects across all
    // trajectories. Each factor is visited exactly once per
    // trajectory (Morris's OAT property), so each `ees[i]` has
    // length R.
    let mut ees: Vec<Vec<f64>> = vec![Vec::with_capacity(r); d];

    // Walk every trajectory; for each consecutive (point_k,
    // point_{k+1}) pair, the factor stepped is
    // `factor_order[r_idx, k]`.
    let mut row_buf = vec![0.0_f64; d];
    for r_idx in 0..r {
        // Evaluate model at point 0.
        for (j, slot) in row_buf.iter_mut().enumerate() {
            *slot = trajectories.trajectories[[r_idx, 0, j]];
        }
        let mut prev_y = model(&row_buf);

        for k in 0..d {
            // Step k advances factor `factor_j`.
            let factor_j = trajectories.factor_order[[r_idx, k]];
            let delta = trajectories.deltas[[r_idx, factor_j]];

            for (j, slot) in row_buf.iter_mut().enumerate() {
                *slot = trajectories.trajectories[[r_idx, k + 1, j]];
            }
            let cur_y = model(&row_buf);

            let ee = (cur_y - prev_y) / delta;
            ees[factor_j].push(ee);

            prev_y = cur_y;
        }
    }

    // OAT-property invariant: each factor visited exactly once per
    // trajectory ⇒ each `ees[i]` has length R. `MorrisTrajectories`
    // produced by `build_morris_trajectories` always satisfies this;
    // a malformed cross-crate construction (the struct is
    // `#[non_exhaustive]` to external crates but open within the
    // saltelli workspace) would silently produce wrong μ otherwise.
    debug_assert!(
        ees.iter().all(|v| v.len() == r),
        "OAT invariant violated: per-factor EE counts {:?} != r = {r}",
        ees.iter().map(Vec::len).collect::<Vec<_>>()
    );

    // Per-factor reduction.
    let r_f = r as f64;
    let mut mu = Vec::with_capacity(d);
    let mut mu_star = Vec::with_capacity(d);
    let mut sigma = Vec::with_capacity(d);

    for ee_i in &ees {
        // μ_i = mean.
        let mu_val = tree_sum(ee_i) / r_f;
        mu.push(mu_val);

        // μ*_i = mean of abs.
        let abs_ee: Vec<f64> = ee_i.iter().map(|x| x.abs()).collect();
        let mu_star_val = tree_sum(&abs_ee) / r_f;
        mu_star.push(mu_star_val);

        // σ_i = sample std with Bessel correction (R-1).
        // For R == 1, σ is undefined; we return 0.0 by convention.
        let sigma_val = if r > 1 {
            let centered_sq: Vec<f64> = ee_i.iter().map(|x| (x - mu_val).powi(2)).collect();
            let var = tree_sum(&centered_sq) / (r_f - 1.0);
            var.sqrt()
        } else {
            0.0
        };
        sigma.push(sigma_val);
    }

    Ok(MorrisEffects::new(r, d, mu, mu_star, sigma))
}

/// Estimate grouped Morris elementary effects from a trajectory
/// bundle produced by `build_grouped_morris_trajectories` and a
/// model.
///
/// For each step `k = 0..n_groups-1`, identifies which GROUP was
/// stepped (from `group_order`), computes the elementary effect
/// `EE_g = (Y_{k+1} - Y_k) / Δ` (using the Δ of the first member
/// factor in the group), and aggregates `μ`, `μ*`, `σ` per group.
///
/// Also computes per-factor effects (same as `estimate_morris_effects`
/// but using a representative Δ per factor from the grouped
/// trajectory).
///
/// # Errors
///
/// Returns `EmptyError::ZeroTrajectories` if `trajectories.r == 0`.
#[allow(clippy::many_single_char_names)]
pub fn estimate_grouped_morris_effects<F>(
    trajectories: &MorrisTrajectories,
    groups: &[Group],
    model: F,
) -> Result<MorrisEffects, EmptyError>
where
    F: Fn(&[f64]) -> f64,
{
    let r = trajectories.r;
    let d = trajectories.d;
    let n_groups = groups.len();

    if r == 0 {
        return Err(EmptyError::ZeroTrajectories);
    }

    let group_order = trajectories
        .group_order
        .as_ref()
        .expect("estimate_grouped_morris_effects requires group_order (use build_grouped_morris_trajectories)");

    // Per-group accumulator of elementary effects.
    let mut group_ees: Vec<Vec<f64>> = vec![Vec::with_capacity(r); n_groups];

    // Per-factor accumulators: for grouped Morris, each factor's EE
    // is the same as the group's EE (since all factors in the group
    // move simultaneously, the individual factor contribution cannot
    // be separated). We record the group-level EE for each factor.
    let mut factor_ees: Vec<Vec<f64>> = vec![Vec::with_capacity(r); d];

    let mut row_buf = vec![0.0_f64; d];
    for r_idx in 0..r {
        // Evaluate model at point 0.
        for (j, slot) in row_buf.iter_mut().enumerate() {
            *slot = trajectories.trajectories[[r_idx, 0, j]];
        }
        let mut prev_y = model(&row_buf);

        for k in 0..n_groups {
            let group_idx = group_order[[r_idx, k]];
            let group = &groups[group_idx];

            // Use the delta of the first factor in the group as the
            // representative delta for computing the group EE.
            let rep_factor = group.factor_indices[0];
            let delta = trajectories.deltas[[r_idx, rep_factor]];

            for (j, slot) in row_buf.iter_mut().enumerate() {
                *slot = trajectories.trajectories[[r_idx, k + 1, j]];
            }
            let cur_y = model(&row_buf);

            let ee = (cur_y - prev_y) / delta;
            group_ees[group_idx].push(ee);

            // Record the same EE for each factor in the group.
            for &factor_j in &group.factor_indices {
                factor_ees[factor_j].push(ee);
            }

            prev_y = cur_y;
        }
    }

    let r_f = r as f64;

    // Per-factor reduction.
    let mut mu = Vec::with_capacity(d);
    let mut mu_star = Vec::with_capacity(d);
    let mut sigma = Vec::with_capacity(d);

    for ee_i in &factor_ees {
        let mu_val = if ee_i.is_empty() {
            0.0
        } else {
            tree_sum(ee_i) / r_f
        };
        mu.push(mu_val);

        let abs_ee: Vec<f64> = ee_i.iter().map(|x| x.abs()).collect();
        let mu_star_val = if abs_ee.is_empty() {
            0.0
        } else {
            tree_sum(&abs_ee) / r_f
        };
        mu_star.push(mu_star_val);

        let sigma_val = if ee_i.len() > 1 {
            let centered_sq: Vec<f64> = ee_i.iter().map(|x| (x - mu_val).powi(2)).collect();
            let var = tree_sum(&centered_sq) / (r_f - 1.0);
            var.sqrt()
        } else {
            0.0
        };
        sigma.push(sigma_val);
    }

    // Per-group reduction.
    let mut grouped_mu = Vec::with_capacity(n_groups);
    let mut grouped_mu_star = Vec::with_capacity(n_groups);
    let mut grouped_sigma = Vec::with_capacity(n_groups);

    for ee_g in &group_ees {
        let mu_val = tree_sum(ee_g) / r_f;
        grouped_mu.push(mu_val);

        let abs_ee: Vec<f64> = ee_g.iter().map(|x| x.abs()).collect();
        let mu_star_val = tree_sum(&abs_ee) / r_f;
        grouped_mu_star.push(mu_star_val);

        let sigma_val = if r > 1 {
            let centered_sq: Vec<f64> = ee_g.iter().map(|x| (x - mu_val).powi(2)).collect();
            let var = tree_sum(&centered_sq) / (r_f - 1.0);
            var.sqrt()
        } else {
            0.0
        };
        grouped_sigma.push(sigma_val);
    }

    let group_names: Vec<String> = groups.iter().map(|g| g.name.clone()).collect();

    Ok(MorrisEffects::new_grouped(
        r,
        d,
        mu,
        mu_star,
        sigma,
        grouped_mu,
        grouped_mu_star,
        grouped_sigma,
        group_names,
    ))
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::cast_precision_loss)]
mod tests {
    use super::*;
    use salib_core::RngState;
    use salib_samplers::build_morris_trajectories;

    fn fresh_rng() -> RngState {
        RngState::from_seed([0x42; 32])
    }

    fn morris_traj(d: usize, r: usize) -> MorrisTrajectories {
        let mut rng = fresh_rng();
        build_morris_trajectories(d, r, 4, &mut rng).unwrap()
    }

    fn assert_close(got: f64, want: f64, tol: f64, ctx: &str) {
        assert!(
            (got - want).abs() <= tol,
            "{ctx}: got {got}, want {want}, |Δ|={}",
            (got - want).abs()
        );
    }

    // ── Output shape ────────────────────────────────────────────────

    #[test]
    fn effects_have_correct_dim() {
        let t = morris_traj(5, 20);
        let e = estimate_morris_effects(&t, |x| x.iter().sum::<f64>()).unwrap();
        assert_eq!(e.d, 5);
        assert_eq!(e.r, 20);
        assert_eq!(e.mu.len(), 5);
        assert_eq!(e.mu_star.len(), 5);
        assert_eq!(e.sigma.len(), 5);
    }

    // ── Linear model: μ_i exactly recovers the coefficient ─────────

    #[test]
    fn purely_linear_model_recovers_coefficients_exactly() {
        // Y = Σ b_i x_i with b = [1, 2, 3, 4, 5].
        // EE_i = b_i (constant for every step), so μ_i = b_i, σ_i = 0.
        let t = morris_traj(5, 30);
        let e = estimate_morris_effects(&t, |x| {
            x.iter()
                .enumerate()
                .map(|(i, xi)| (i as f64 + 1.0) * xi)
                .sum::<f64>()
        })
        .unwrap();
        for i in 0..5 {
            #[allow(clippy::cast_precision_loss)]
            let want = i as f64 + 1.0;
            assert_close(e.mu[i], want, 1e-10, &format!("mu[{i}]"));
            assert_close(e.mu_star[i], want, 1e-10, &format!("mu_star[{i}]"));
            assert_close(e.sigma[i], 0.0, 1e-10, &format!("sigma[{i}]"));
        }
    }

    // ── μ* ≥ |μ| identity ────────────────────────────────────────────

    #[test]
    fn mu_star_at_least_absolute_mu() {
        // Identity: |mean(x)| ≤ mean(|x|). Always.
        let t = morris_traj(5, 30);
        let e = estimate_morris_effects(&t, |x| (x[0] - 0.5).powi(3) + x[1].sin() + x[2] * x[3])
            .unwrap();
        for i in 0..5 {
            assert!(
                e.mu_star[i] >= e.mu[i].abs() - 1e-12,
                "μ*[{i}] = {} should be ≥ |μ[{i}]| = {}",
                e.mu_star[i],
                e.mu[i].abs()
            );
        }
    }

    // ── Constant model: zero everything ─────────────────────────────

    #[test]
    fn constant_model_yields_zero_effects() {
        let t = morris_traj(4, 10);
        let e = estimate_morris_effects(&t, |_x| 7.0).unwrap();
        for i in 0..4 {
            assert_eq!(e.mu[i], 0.0);
            assert_eq!(e.mu_star[i], 0.0);
            assert_eq!(e.sigma[i], 0.0);
        }
    }

    // ── Determinism ─────────────────────────────────────────────────

    #[test]
    fn same_inputs_produce_identical_effects() {
        let t = morris_traj(4, 20);
        let e1 = estimate_morris_effects(&t, |x| x.iter().sum::<f64>()).unwrap();
        let e2 = estimate_morris_effects(&t, |x| x.iter().sum::<f64>()).unwrap();
        assert_eq!(e1, e2);
    }

    // ── Quadratic factor introduces σ > 0 ───────────────────────────

    #[test]
    fn quadratic_factor_has_nonzero_sigma() {
        // Y = x_0² + x_1. EE_0 depends on the trajectory base point;
        // σ_0 > 0. EE_1 is constant (= 1); σ_1 = 0.
        let t = morris_traj(2, 50);
        let e = estimate_morris_effects(&t, |x| x[0].powi(2) + x[1]).unwrap();
        assert!(e.sigma[0] > 0.1, "σ_0 = {}", e.sigma[0]);
        assert_close(e.sigma[1], 0.0, 1e-10, "σ_1");
    }

    // ── Empty trajectories ──────────────────────────────────────────
    //
    // The `EmptyError::ZeroTrajectories` path is unreachable via the
    // public `build_morris_trajectories` constructor (which rejects
    // `r == 0`). The error variant guards against deserialization /
    // future-cross-crate paths that bypass the constructor; testing
    // it requires synthesizing a `MorrisTrajectories` value, which
    // is `#[non_exhaustive]` and can only be constructed inside its
    // owning crate. Coverage of this path lives in
    // salib-samplers's own tests.

    // ── R=1 handles σ degenerate case ───────────────────────────────

    #[test]
    fn r_one_yields_zero_sigma_by_convention() {
        // With R=1, sample std is undefined (division by R-1=0). We
        // return 0.0 by convention.
        let t = morris_traj(3, 1);
        let e = estimate_morris_effects(&t, |x| x[0].powi(2) + x[1] + x[2]).unwrap();
        assert_eq!(e.r, 1);
        for i in 0..3 {
            assert_eq!(e.sigma[i], 0.0);
        }
    }

    // ── Effects of unused factor are zero ───────────────────────────

    #[test]
    fn unused_factor_has_zero_effects() {
        // Y depends only on x_0; x_1 should have μ=0, μ*=0, σ=0.
        let t = morris_traj(2, 30);
        let e = estimate_morris_effects(&t, |x| x[0] * 5.0).unwrap();
        assert_close(e.mu[0], 5.0, 1e-10, "μ_0");
        assert_close(e.mu[1], 0.0, 1e-10, "μ_1 (unused)");
        assert_close(e.mu_star[1], 0.0, 1e-10, "μ*_1");
        assert_close(e.sigma[1], 0.0, 1e-10, "σ_1");
    }
}
