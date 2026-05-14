//! TCK harness for grouped-factor Morris support.
//!
//! Maps Gherkin scenarios from
//! `tck/salib/grouped-factors/features/grouped_factors.feature`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use salib_core::{Group, RngState};
use salib_estimators::{estimate_grouped_morris_effects, estimate_morris_effects};
use salib_samplers::{build_grouped_morris_trajectories, build_morris_trajectories};

const SEED: [u8; 32] = [0x42; 32];

// ── Scenario: Grouped trajectory has n_groups+1 points ────────────

#[test]
fn grouped_trajectory_shape() {
    let groups = vec![
        Group {
            name: "A".into(),
            factor_indices: vec![0, 1],
        },
        Group {
            name: "B".into(),
            factor_indices: vec![2, 3],
        },
    ];
    let mut rng = RngState::from_seed(SEED);
    let traj = build_grouped_morris_trajectories(&groups, 4, 10, 4, &mut rng).unwrap();
    // n_groups=2 => 3 points per trajectory
    assert_eq!(traj.trajectories.shape(), &[10, 3, 4]);
    assert!(traj.group_order.is_some());
    let group_order = traj.group_order.as_ref().unwrap();
    assert_eq!(group_order.shape(), &[10, 2]);
}

// ── Scenario: Grouped Morris ranks group B higher ─────────────────
//
// f(x) = x1 + x2 + 3*x3 + 3*x4
// Group A = {x1, x2} (coefficients 1, 1)
// Group B = {x3, x4} (coefficients 3, 3)
// Group B should have larger mu_star than group A.

#[test]
fn grouped_morris_ranks_b_higher() {
    let groups = vec![
        Group {
            name: "A".into(),
            factor_indices: vec![0, 1],
        },
        Group {
            name: "B".into(),
            factor_indices: vec![2, 3],
        },
    ];
    let mut rng = RngState::from_seed(SEED);
    let traj = build_grouped_morris_trajectories(&groups, 4, 100, 4, &mut rng).unwrap();

    let effects =
        estimate_grouped_morris_effects(&traj, &groups, |x| x[0] + x[1] + 3.0 * x[2] + 3.0 * x[3])
            .unwrap();

    let grouped_mu_star = effects
        .grouped_mu_star
        .as_ref()
        .expect("should have grouped_mu_star");
    // Group B (index 1) should have larger mu_star than group A (index 0)
    assert!(
        grouped_mu_star[1] > grouped_mu_star[0],
        "group B mu_star ({}) should be > group A ({})",
        grouped_mu_star[1],
        grouped_mu_star[0]
    );

    // Also verify group names are correct
    let names = effects
        .group_names
        .as_ref()
        .expect("should have group_names");
    assert_eq!(names, &["A", "B"]);
}

// ── Scenario: Ungrouped equals singleton groups (Morris identity) ─
//
// When each group contains exactly one factor, the grouped Morris
// analysis should produce mu_star values equal to the ungrouped
// analysis (within tolerance, same seed).

#[test]
fn ungrouped_equals_singleton_groups() {
    let d = 3;

    // Ungrouped run.
    let mut rng_ungrouped = RngState::from_seed(SEED);
    let traj_ungrouped = build_morris_trajectories(d, 50, 4, &mut rng_ungrouped).unwrap();
    let effects_ungrouped =
        estimate_morris_effects(&traj_ungrouped, |x| 2.0 * x[0] + 3.0 * x[1] + 5.0 * x[2]).unwrap();

    // Grouped run with singleton groups.
    let singleton_groups = vec![
        Group {
            name: "g0".into(),
            factor_indices: vec![0],
        },
        Group {
            name: "g1".into(),
            factor_indices: vec![1],
        },
        Group {
            name: "g2".into(),
            factor_indices: vec![2],
        },
    ];
    let mut rng_grouped = RngState::from_seed(SEED);
    let traj_grouped =
        build_grouped_morris_trajectories(&singleton_groups, d, 50, 4, &mut rng_grouped).unwrap();
    let effects_grouped = estimate_grouped_morris_effects(&traj_grouped, &singleton_groups, |x| {
        2.0 * x[0] + 3.0 * x[1] + 5.0 * x[2]
    })
    .unwrap();

    // Both use the same seed. With singleton groups, the grouped
    // sampler should produce the same trajectories as the ungrouped
    // sampler (same Fisher-Yates over d items). The grouped mu_star
    // per group should match the ungrouped per-factor mu_star.
    let grouped_mu_star = effects_grouped
        .grouped_mu_star
        .as_ref()
        .expect("should have grouped_mu_star");

    for (i, (gms, ums)) in grouped_mu_star
        .iter()
        .zip(effects_ungrouped.mu_star.iter())
        .enumerate()
    {
        let diff = (gms - ums).abs();
        assert!(
            diff < 0.01,
            "factor {i}: grouped mu_star ({gms}) vs ungrouped mu_star ({ums}) differ by {diff}",
        );
    }
}

// ── Additional property: grouped trajectory OAT on groups ─────────

#[test]
fn grouped_trajectory_steps_groups_not_individual_factors() {
    // Verify that consecutive points in a grouped trajectory can
    // differ in MORE than one factor (because all factors in the
    // stepped group move simultaneously).
    let groups = vec![
        Group {
            name: "A".into(),
            factor_indices: vec![0, 1],
        },
        Group {
            name: "B".into(),
            factor_indices: vec![2, 3],
        },
    ];
    let mut rng = RngState::from_seed(SEED);
    let traj = build_grouped_morris_trajectories(&groups, 4, 20, 4, &mut rng).unwrap();

    // At each step, exactly the factors in the stepped group should
    // change. Count how many factors differ between consecutive
    // points — it should equal the group size.
    let group_order = traj.group_order.as_ref().unwrap();
    for r_idx in 0..traj.r {
        for k in 0..groups.len() {
            let group_idx = group_order[[r_idx, k]];
            let group_size = groups[group_idx].factor_indices.len();
            let mut differ_count = 0;
            for j in 0..traj.d {
                let before = traj.trajectories[[r_idx, k, j]];
                let after = traj.trajectories[[r_idx, k + 1, j]];
                if (before - after).abs() > 1e-12 {
                    differ_count += 1;
                }
            }
            assert_eq!(
                differ_count, group_size,
                "trajectory {r_idx} step {k}: {differ_count} factors changed, expected {group_size} (group {group_idx})"
            );
        }
    }
}

// ── Grouped trajectory: group_order visits each group exactly once ─

#[test]
fn group_order_visits_each_group_exactly_once() {
    let groups = vec![
        Group {
            name: "A".into(),
            factor_indices: vec![0],
        },
        Group {
            name: "B".into(),
            factor_indices: vec![1, 2],
        },
        Group {
            name: "C".into(),
            factor_indices: vec![3],
        },
    ];
    let mut rng = RngState::from_seed(SEED);
    let traj = build_grouped_morris_trajectories(&groups, 4, 20, 4, &mut rng).unwrap();
    let group_order = traj.group_order.as_ref().unwrap();
    for r_idx in 0..traj.r {
        let mut visited: Vec<usize> = (0..groups.len()).map(|k| group_order[[r_idx, k]]).collect();
        visited.sort_unstable();
        let expected: Vec<usize> = (0..groups.len()).collect();
        assert_eq!(
            visited, expected,
            "trajectory {r_idx} group_order doesn't visit each group exactly once"
        );
    }
}

// ── Determinism ───────────────────────────────────────────────────

#[test]
fn grouped_trajectory_is_deterministic() {
    let groups = vec![
        Group {
            name: "A".into(),
            factor_indices: vec![0, 1],
        },
        Group {
            name: "B".into(),
            factor_indices: vec![2, 3],
        },
    ];
    let mut rng1 = RngState::from_seed(SEED);
    let mut rng2 = RngState::from_seed(SEED);
    let t1 = build_grouped_morris_trajectories(&groups, 4, 20, 4, &mut rng1).unwrap();
    let t2 = build_grouped_morris_trajectories(&groups, 4, 20, 4, &mut rng2).unwrap();
    assert_eq!(t1.trajectories, t2.trajectories);
    assert_eq!(t1.deltas, t2.deltas);
    assert_eq!(t1.group_order, t2.group_order);
}
