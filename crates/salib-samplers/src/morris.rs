//! Morris trajectory sampler — classic OAT (one-at-a-time) design,
//! per Morris 1991.
//!
//! # The trajectory shape
//!
//! A Morris *trajectory* is a sequence of `d + 1` points where each
//! consecutive pair differs in exactly one coordinate. For a
//! `d`-factor problem with grid levels `p`:
//!
//! 1. Pick a random base point on the grid `{0, 1/(p-1), …, 1}^d`.
//! 2. Pick a random permutation of factor indices `{0, …, d-1}`.
//! 3. For each factor `i` in permuted order: step by `+Δ` in
//!    coordinate `i`, emit the new point.
//!
//! Step size `Δ = p / (2 · (p - 1))` per Saltelli's standard
//! recommendation. For `p = 4`, `Δ = 2/3`.
//!
//! # `R` trajectories
//!
//! Calling `build_morris_trajectories` with R produces R independent
//! trajectories, each `d + 1` points × `d` factors. Total cost is
//! `R · (d + 1)` model evaluations — same as Morris's original
//! design.
//!
//! # Determinism
//!
//! Pure under `(d, r, levels, RngState)`. Same `RngState` in →
//! bit-identical output. Per `decisions/2026-04-28-saltelli-rng-determinism.md`.
//!
//! # What this PR does NOT ship
//!
//! - **Campolongo trajectory optimization** (Campolongo-Cariboni-
//!   Saltelli 2007). Greedy maximin selection of R trajectories from
//!   a pool of M >> R candidates. Reduces estimator variance at the
//!   cost of M² distance computations. Deferred to PR 8.5; classic
//!   OAT is sufficient for the reviewer-affordance contract close.
//! - **Ruano local-search optimization** (Ruano-Ribes-Ferreira-
//!   Conceição 2012). Phase 2 / Phase C work.
//! - **Radial Morris** (per Saltelli's variant). Deferred.

use ndarray::{Array2, Array3};
use rand::RngCore;
use salib_core::{Group, RngState};

/// Output of `build_morris_trajectories`.
///
/// `#[non_exhaustive]` — future fields (e.g., recorded pre-draw
/// `RngState` for audit-replay; `kind: MorrisKind` if Campolongo
/// lands) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MorrisTrajectories {
    /// `(R, d+1, d)` array of points. `trajectories[[r, k, j]]` is
    /// factor `j`'s value at point `k` of trajectory `r`. Point `0`
    /// is the random base; points `1..=d` are the OAT-stepped
    /// successors in the random factor permutation order.
    pub trajectories: Array3<f64>,
    /// `(R, d)` array of per-trajectory per-factor signed Δ values.
    /// `deltas[[r, j]]` is the signed step taken on factor `j` in
    /// trajectory `r` (either `+Δ` or `-Δ` for the standard
    /// `Δ = p / (2·(p-1))`).
    pub deltas: Array2<f64>,
    /// `(R, d)` array of factor permutations. `factor_order[[r, k]]`
    /// is the factor index stepped at position `k` of trajectory
    /// `r`. Useful for the EE estimator to know which factor's
    /// step happened between consecutive points.
    pub factor_order: Array2<usize>,
    /// `(R, n_groups)` array of group permutations for grouped
    /// trajectories. `group_order[[r, k]]` is the GROUP index stepped
    /// at position `k` of trajectory `r`. `None` for ungrouped
    /// (standard) trajectories produced by `build_morris_trajectories`.
    pub group_order: Option<Array2<usize>>,
    /// Number of trajectories.
    pub r: usize,
    /// Factor count.
    pub d: usize,
    /// Grid levels (`p` per Morris's terminology).
    pub levels: u32,
}

/// Errors from `build_morris_trajectories`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MorrisError {
    #[error("Morris: r must be ≥ 1, got 0")]
    ZeroR,
    #[error("Morris: d must be ≥ 1, got 0")]
    ZeroD,
    #[error("Morris: levels must be ≥ 2, got {levels}")]
    LevelsBelowTwo { levels: u32 },
    #[error("Morris: levels must be even (Saltelli convention), got {levels}")]
    LevelsOdd { levels: u32 },
}

/// Build `r` Morris trajectories for a `d`-factor problem on a grid
/// of `levels` (typically 4 or 8 — even numbers per Saltelli's
/// convention, so the step `Δ = p / (2·(p-1))` lands on a grid
/// point). Step size `Δ = levels / (2·(levels-1))`.
///
/// Output: `MorrisTrajectories` with `(R, d+1, d)` points and
/// `(R, d)` factor-permutation order.
///
/// # Errors
///
/// - `MorrisError::ZeroR` if `r == 0`.
/// - `MorrisError::ZeroD` if `d == 0`.
/// - `MorrisError::LevelsBelowTwo` if `levels < 2`.
/// - `MorrisError::LevelsOdd` if `levels` is odd.
#[allow(clippy::cast_precision_loss, clippy::similar_names)]
pub fn build_morris_trajectories(
    d: usize,
    r: usize,
    levels: u32,
    rng: &mut RngState,
) -> Result<MorrisTrajectories, MorrisError> {
    if r == 0 {
        return Err(MorrisError::ZeroR);
    }
    if d == 0 {
        return Err(MorrisError::ZeroD);
    }
    if levels < 2 {
        return Err(MorrisError::LevelsBelowTwo { levels });
    }
    if !levels.is_multiple_of(2) {
        return Err(MorrisError::LevelsOdd { levels });
    }

    // Δ = p / (2(p-1)) per Saltelli. Lands on a grid point because
    // p is even (so p-1 odd, and p/2 / (p-1) has 2 in numerator).
    let p = f64::from(levels);
    let delta = p / (2.0 * (p - 1.0));

    let mut chacha = rng.clone().into_chacha();
    let grid_step = 1.0 / f64::from(levels - 1);

    // We pick the base point per Morris from the *lower* portion of
    // the grid `{0, 1/(p-1), …, 1/2}` so that adding +Δ stays on a
    // grid point. (For Δ > 1/2, base must be in [0, 1-Δ]. With
    // p=4, Δ=2/3, base must be in {0, 1/3}.) This is the standard
    // Morris construction.
    //
    // Number of valid base levels = (p / 2) — the lower half of the
    // grid. For p=4: 2 valid base levels {0, 1/3}.
    let valid_base_levels = (levels / 2) as usize; // p/2

    let mut traj = Array3::<f64>::zeros((r, d + 1, d));
    let mut deltas_arr = Array2::<f64>::zeros((r, d));
    let mut order_arr = Array2::<usize>::zeros((r, d));

    // Modulo-bias note: both the base-point draw and the Fisher-
    // Yates permutation reduce a `u32` modulo a small bound. The
    // bias is `O(bound / 2³²) ≈ 10⁻⁸` for `bound ≤ 100` — well
    // below MC noise at any realistic Morris workload (R ≤ 1000).
    // Matches the LHS sampler's posture (PR 5 of the roadmap;
    // see `crates/salib-samplers/src/lhs.rs::LhsSampler::unit_sample`).
    for r_idx in 0..r {
        // Random base point: each coordinate chosen from the lower
        // half of the grid.
        let mut base = vec![0.0_f64; d];
        for slot in &mut base {
            #[allow(clippy::cast_possible_truncation)]
            let lvl = (chacha.next_u32() as usize) % valid_base_levels;
            #[allow(clippy::cast_precision_loss)]
            let base_val = (lvl as f64) * grid_step;
            *slot = base_val;
        }

        // Random permutation of factor indices via Fisher-Yates.
        let mut perm: Vec<usize> = (0..d).collect();
        for i in (1..d).rev() {
            #[allow(clippy::cast_possible_truncation)]
            let k = (chacha.next_u32() as usize) % (i + 1);
            perm.swap(i, k);
        }

        // Step direction is fixed +Δ. Saltelli's original Morris
        // 1991 randomizes the sign per factor; we don't, because
        // the random base + random permutation already provides
        // the design's coverage and SALib's modern implementation
        // similarly uses fixed +Δ. Random ±Δ lands when SALib
        // byte-exact differential becomes a goal — see
        // `decisions/2026-04-29-saltelli-morris-estimator.md`
        // § "What this PR does NOT ship."

        // Emit point 0 (base).
        for j in 0..d {
            traj[[r_idx, 0, j]] = base[j];
        }

        // Emit points 1..=d, stepping along perm.
        let mut current = base.clone();
        for (step_idx, &factor_j) in perm.iter().enumerate() {
            current[factor_j] += delta;
            for j in 0..d {
                traj[[r_idx, step_idx + 1, j]] = current[j];
            }
            deltas_arr[[r_idx, factor_j]] = delta;
            order_arr[[r_idx, step_idx]] = factor_j;
        }
    }

    *rng = RngState::snapshot(&chacha, rng);

    Ok(MorrisTrajectories {
        trajectories: traj,
        deltas: deltas_arr,
        factor_order: order_arr,
        group_order: None,
        r,
        d,
        levels,
    })
}

/// Build `r` grouped Morris trajectories for a `d`-factor problem.
///
/// In grouped Morris, factors in the same group are perturbed
/// simultaneously. Instead of `d + 1` points per trajectory (one
/// step per factor), there are `n_groups + 1` points (one step per
/// group). Each step perturbs ALL factors in the selected group by
/// their respective `+-Δ` values.
///
/// Output: `MorrisTrajectories` with `(R, n_groups+1, d)` points,
/// `(R, d)` deltas, `(R, d)` factor_order (records which factor was
/// stepped, flattened), and `(R, n_groups)` group_order (records
/// which GROUP was stepped at each position).
///
/// # Errors
///
/// Same as `build_morris_trajectories`: `MorrisError::ZeroR`,
/// `ZeroD`, `LevelsBelowTwo`, `LevelsOdd`.
#[allow(clippy::cast_precision_loss, clippy::similar_names)]
pub fn build_grouped_morris_trajectories(
    groups: &[Group],
    d: usize,
    r: usize,
    levels: u32,
    rng: &mut RngState,
) -> Result<MorrisTrajectories, MorrisError> {
    if r == 0 {
        return Err(MorrisError::ZeroR);
    }
    if d == 0 {
        return Err(MorrisError::ZeroD);
    }
    if levels < 2 {
        return Err(MorrisError::LevelsBelowTwo { levels });
    }
    if !levels.is_multiple_of(2) {
        return Err(MorrisError::LevelsOdd { levels });
    }

    let n_groups = groups.len();

    // Δ = p / (2(p-1)) per Saltelli.
    let p = f64::from(levels);
    let delta = p / (2.0 * (p - 1.0));

    let mut chacha = rng.clone().into_chacha();
    let grid_step = 1.0 / f64::from(levels - 1);
    let valid_base_levels = (levels / 2) as usize;

    let mut traj = Array3::<f64>::zeros((r, n_groups + 1, d));
    let mut deltas_arr = Array2::<f64>::zeros((r, d));
    // factor_order: (R, d) — records which factor was stepped for
    // each of the d factor-level steps (some factors are stepped
    // simultaneously within a group, but we record group-level
    // order in group_order).
    let mut order_arr = Array2::<usize>::zeros((r, d));
    let mut group_order_arr = Array2::<usize>::zeros((r, n_groups));

    for r_idx in 0..r {
        // Random base point: each coordinate from the lower half.
        let mut base = vec![0.0_f64; d];
        for slot in &mut base {
            #[allow(clippy::cast_possible_truncation)]
            let lvl = (chacha.next_u32() as usize) % valid_base_levels;
            #[allow(clippy::cast_precision_loss)]
            let base_val = (lvl as f64) * grid_step;
            *slot = base_val;
        }

        // Random permutation of GROUP indices via Fisher-Yates.
        let mut perm: Vec<usize> = (0..n_groups).collect();
        for i in (1..n_groups).rev() {
            #[allow(clippy::cast_possible_truncation)]
            let k = (chacha.next_u32() as usize) % (i + 1);
            perm.swap(i, k);
        }

        // Emit point 0 (base).
        for j in 0..d {
            traj[[r_idx, 0, j]] = base[j];
        }

        // Emit points 1..=n_groups, stepping all factors in each
        // group simultaneously.
        let mut current = base.clone();
        for (step_idx, &group_idx) in perm.iter().enumerate() {
            let group = &groups[group_idx];
            for &factor_j in &group.factor_indices {
                current[factor_j] += delta;
                deltas_arr[[r_idx, factor_j]] = delta;
                order_arr[[r_idx, factor_j]] = step_idx;
            }
            for j in 0..d {
                traj[[r_idx, step_idx + 1, j]] = current[j];
            }
            group_order_arr[[r_idx, step_idx]] = group_idx;
        }
    }

    *rng = RngState::snapshot(&chacha, rng);

    Ok(MorrisTrajectories {
        trajectories: traj,
        deltas: deltas_arr,
        factor_order: order_arr,
        group_order: Some(group_order_arr),
        r,
        d,
        levels,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn fresh_rng() -> RngState {
        RngState::from_seed([0x42; 32])
    }

    // ── Validation ──────────────────────────────────────────────────

    #[test]
    fn zero_r_returns_error() {
        let mut rng = fresh_rng();
        let err = build_morris_trajectories(3, 0, 4, &mut rng).unwrap_err();
        assert_eq!(err, MorrisError::ZeroR);
    }

    #[test]
    fn zero_d_returns_error() {
        let mut rng = fresh_rng();
        let err = build_morris_trajectories(0, 10, 4, &mut rng).unwrap_err();
        assert_eq!(err, MorrisError::ZeroD);
    }

    #[test]
    fn levels_one_returns_error() {
        let mut rng = fresh_rng();
        let err = build_morris_trajectories(3, 10, 1, &mut rng).unwrap_err();
        assert_eq!(err, MorrisError::LevelsBelowTwo { levels: 1 });
    }

    #[test]
    fn levels_odd_returns_error() {
        let mut rng = fresh_rng();
        let err = build_morris_trajectories(3, 10, 5, &mut rng).unwrap_err();
        assert_eq!(err, MorrisError::LevelsOdd { levels: 5 });
    }

    // ── Output shape ────────────────────────────────────────────────

    #[test]
    fn output_shape_is_r_by_d_plus_one_by_d() {
        let mut rng = fresh_rng();
        let t = build_morris_trajectories(5, 10, 4, &mut rng).unwrap();
        assert_eq!(t.trajectories.shape(), &[10, 6, 5]);
        assert_eq!(t.deltas.shape(), &[10, 5]);
        assert_eq!(t.factor_order.shape(), &[10, 5]);
        assert_eq!(t.r, 10);
        assert_eq!(t.d, 5);
        assert_eq!(t.levels, 4);
    }

    // ── OAT property ────────────────────────────────────────────────

    #[test]
    fn consecutive_points_differ_in_exactly_one_factor() {
        // Load-bearing Morris property: each pair of consecutive
        // points along a trajectory differs in exactly one
        // coordinate (the OAT step).
        let mut rng = fresh_rng();
        let t = build_morris_trajectories(8, 50, 4, &mut rng).unwrap();
        for r_idx in 0..t.r {
            for k in 0..t.d {
                let mut differ_count = 0;
                for j in 0..t.d {
                    let before = t.trajectories[[r_idx, k, j]];
                    let after = t.trajectories[[r_idx, k + 1, j]];
                    if (before - after).abs() > 1e-12 {
                        differ_count += 1;
                    }
                }
                assert_eq!(
                    differ_count, 1,
                    "trajectory {r_idx} step {k}: {differ_count} factors changed"
                );
            }
        }
    }

    #[test]
    fn factor_order_visits_each_factor_exactly_once() {
        let mut rng = fresh_rng();
        let t = build_morris_trajectories(6, 20, 4, &mut rng).unwrap();
        for r_idx in 0..t.r {
            let mut visited: Vec<usize> = (0..t.d).map(|k| t.factor_order[[r_idx, k]]).collect();
            visited.sort_unstable();
            let expected: Vec<usize> = (0..t.d).collect();
            assert_eq!(visited, expected, "trajectory {r_idx} factor_order");
        }
    }

    // ── Step size ───────────────────────────────────────────────────

    #[test]
    fn step_size_matches_saltelli_formula_at_levels_four() {
        // p=4: Δ = 4 / (2·3) = 2/3.
        let mut rng = fresh_rng();
        let t = build_morris_trajectories(3, 10, 4, &mut rng).unwrap();
        let expected_delta = 2.0 / 3.0;
        for r_idx in 0..t.r {
            for j in 0..t.d {
                let d_val = t.deltas[[r_idx, j]];
                assert!(
                    (d_val.abs() - expected_delta).abs() < 1e-12,
                    "delta = {d_val}, expected ±{expected_delta}"
                );
            }
        }
    }

    // ── Output range ────────────────────────────────────────────────

    #[test]
    fn all_points_in_unit_interval() {
        // With base in lower half + positive Δ, all points stay in
        // [0, 1] modulo FP. At higher even p, Δ may not be exactly
        // representable in f64 (e.g. p=10 ⇒ Δ = 5/9), so allow a
        // small tolerance — the construction guarantees `|x - 1| ≤ ε`
        // at the upper edge.
        let cases = [(5usize, 30usize, 4u32), (3, 30, 6), (3, 30, 8), (3, 30, 10)];
        let eps = 1e-12_f64;
        for (d, r, levels) in cases {
            let mut rng = fresh_rng();
            let t = build_morris_trajectories(d, r, levels, &mut rng).unwrap();
            for &v in &t.trajectories {
                assert!(
                    v >= -eps && v <= 1.0 + eps,
                    "out-of-range {v} at (d={d}, r={r}, levels={levels})"
                );
            }
        }
    }

    // ── Determinism ─────────────────────────────────────────────────

    #[test]
    fn same_rngstate_produces_identical_trajectories() {
        let mut r1 = fresh_rng();
        let mut r2 = fresh_rng();
        let t1 = build_morris_trajectories(4, 20, 4, &mut r1).unwrap();
        let t2 = build_morris_trajectories(4, 20, 4, &mut r2).unwrap();
        assert_eq!(t1.trajectories, t2.trajectories);
        assert_eq!(t1.deltas, t2.deltas);
        assert_eq!(t1.factor_order, t2.factor_order);
    }

    #[test]
    fn distinct_streams_produce_different_trajectories() {
        let mut r1 = RngState::from_parts([0; 32], 1, 0);
        let mut r2 = RngState::from_parts([0; 32], 2, 0);
        let t1 = build_morris_trajectories(4, 20, 4, &mut r1).unwrap();
        let t2 = build_morris_trajectories(4, 20, 4, &mut r2).unwrap();
        assert_ne!(t1.trajectories, t2.trajectories);
    }

    #[test]
    fn unit_sample_advances_rng_word_pos() {
        let mut rng = fresh_rng();
        let initial = rng.word_pos;
        let _ = build_morris_trajectories(4, 10, 4, &mut rng).unwrap();
        assert!(rng.word_pos > initial);
    }

    // ── Grid alignment ──────────────────────────────────────────────

    #[test]
    fn base_points_are_on_lower_half_grid() {
        // With levels=4, valid base levels are {0, 1/3}. Each
        // coordinate of point 0 should be one of those values.
        let mut rng = fresh_rng();
        let t = build_morris_trajectories(5, 30, 4, &mut rng).unwrap();
        for r_idx in 0..t.r {
            for j in 0..t.d {
                let base = t.trajectories[[r_idx, 0, j]];
                let on_grid = base == 0.0 || (base - 1.0 / 3.0).abs() < 1e-12;
                assert!(on_grid, "base[{r_idx},{j}] = {base} not in {{0, 1/3}}");
            }
        }
    }

    #[test]
    fn level_six_yields_three_valid_base_levels() {
        // p=6: valid base levels = p/2 = 3, namely {0, 1/5, 2/5}.
        // Δ = 6/(2·5) = 3/5.
        let mut rng = fresh_rng();
        let t = build_morris_trajectories(3, 30, 6, &mut rng).unwrap();
        for r_idx in 0..t.r {
            for j in 0..t.d {
                let base = t.trajectories[[r_idx, 0, j]];
                let on_grid =
                    base == 0.0 || (base - 0.2).abs() < 1e-12 || (base - 0.4).abs() < 1e-12;
                assert!(on_grid, "base[{r_idx},{j}] = {base}");
            }
        }
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn r_one_produces_one_trajectory() {
        let mut rng = fresh_rng();
        let t = build_morris_trajectories(3, 1, 4, &mut rng).unwrap();
        assert_eq!(t.r, 1);
        assert_eq!(t.trajectories.shape(), &[1, 4, 3]);
    }

    #[test]
    fn d_one_produces_two_point_trajectories() {
        // d=1 ⇒ trajectory has d+1 = 2 points.
        let mut rng = fresh_rng();
        let t = build_morris_trajectories(1, 10, 4, &mut rng).unwrap();
        assert_eq!(t.trajectories.shape(), &[10, 2, 1]);
    }
}
