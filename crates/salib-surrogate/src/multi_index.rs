//! Multi-index enumeration for PCE basis truncation.
//!
//! A PCE expansion of degree `p` over `d` factors uses multi-indices
//! `α = (α₁, …, α_d) ∈ ℕ^d` with `|α| = α₁ + … + α_d ≤ p`. Each
//! multi-index identifies a tensor-product polynomial
//! `Ψ_α(x) = ∏ᵢ Ψ_{αᵢ}(xᵢ)` in the PCE basis.
//!
//! # Total-degree truncation
//!
//! [`enumerate_total_degree`] generates all multi-indices with
//! `|α| ≤ p`. Cardinality:
//!
//! ```text
//! |{α : |α| ≤ p}| = (d + p)! / (d! · p!)
//! ```
//!
//! For `d = 3, p = 4`: `35` indices. For `d = 10, p = 4`:
//! `1001`. For `d = 20, p = 5`: `53130` — already in the regime
//! where sparse-LARS truncation (PR 16c) becomes essential.
//!
//! # Hyperbolic truncation (deferred)
//!
//! Blatman-Sudret 2011 generalize to "hyperbolic" `q`-norm
//! truncation `|α|_q := (Σ αⱼ^q)^{1/q} ≤ p` with `q ∈ (0, 1]`
//! to favor low-interaction terms. Lands in PR 16c alongside the
//! sparse-LARS solver.

#![allow(clippy::cast_precision_loss)]

/// A multi-index `α ∈ ℕ^d`. `indices[i]` is the polynomial degree
/// in factor `i`'s univariate basis.
///
/// `#[non_exhaustive]` — future fields (e.g., `cached_norm_squared`
/// for performance) land non-breaking.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MultiIndex {
    /// Per-factor polynomial degrees. Length equals factor count `d`.
    pub indices: Vec<usize>,
}

impl MultiIndex {
    /// Construct a multi-index from a slice of per-factor degrees.
    #[must_use]
    pub fn new(indices: Vec<usize>) -> Self {
        Self { indices }
    }

    /// Total degree `|α| = Σ αᵢ`.
    #[must_use]
    pub fn total_degree(&self) -> usize {
        self.indices.iter().sum()
    }

    /// Factor count `d`.
    #[must_use]
    pub fn dim(&self) -> usize {
        self.indices.len()
    }

    /// `true` iff `α = (0, 0, …, 0)` (the constant `Ψ₀ = 1`).
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.indices.iter().all(|&v| v == 0)
    }

    /// `true` iff `α` has exactly one non-zero entry — corresponds
    /// to the "main effect" of a single factor in the PCE.
    #[must_use]
    pub fn is_main_effect(&self) -> bool {
        self.indices.iter().filter(|&&v| v > 0).count() == 1
    }

    /// Active factors — indices `i` where `αᵢ > 0`. Used by
    /// Sudret 2008 Eq 39 to determine which factor's Sobol' index
    /// the multi-index contributes to.
    #[must_use]
    pub fn active_factors(&self) -> Vec<usize> {
        self.indices
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| if v > 0 { Some(i) } else { None })
            .collect()
    }
}

/// Errors from [`enumerate_total_degree`] / [`enumerate_hyperbolic`].
///
/// `Eq` is intentionally not derived because [`InvalidHyperbolicQ`]
/// carries an `f64`. `PartialEq` is sufficient for `assert_eq!` style
/// comparisons in tests.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum MultiIndexError {
    #[error("multi-index: d must be ≥ 1, got 0")]
    ZeroD,
    #[error("multi-index: hyperbolic q must be in (0, 1], got {q}")]
    InvalidHyperbolicQ { q: f64 },
}

/// Enumerate all multi-indices `α ∈ ℕ^d` with `|α| ≤ max_degree`,
/// in lexicographic order. The first element is the zero multi-
/// index `(0, …, 0)`.
///
/// Cardinality of the returned `Vec` is `(d + max_degree)! /
/// (d! · max_degree!)`.
///
/// # Errors
///
/// - [`MultiIndexError::ZeroD`] if `d == 0`.
pub fn enumerate_total_degree(
    d: usize,
    max_degree: usize,
) -> Result<Vec<MultiIndex>, MultiIndexError> {
    if d == 0 {
        return Err(MultiIndexError::ZeroD);
    }
    let mut result = Vec::new();
    let mut current = vec![0_usize; d];
    enumerate_recursive(d, max_degree, 0, 0, &mut current, &mut result);
    Ok(result)
}

/// Recursive enumeration. Emits all completions of `current[0..pos]`
/// to a full `d`-vector with total degree `≤ max_degree`, given
/// `current_sum = Σ current[0..pos]`.
fn enumerate_recursive(
    d: usize,
    max_degree: usize,
    pos: usize,
    current_sum: usize,
    current: &mut [usize],
    out: &mut Vec<MultiIndex>,
) {
    if pos == d {
        out.push(MultiIndex {
            indices: current.to_vec(),
        });
        return;
    }
    let remaining_budget = max_degree - current_sum;
    for v in 0..=remaining_budget {
        current[pos] = v;
        enumerate_recursive(d, max_degree, pos + 1, current_sum + v, current, out);
    }
    // No reset needed: parent overwrites current[pos-1] on next
    // iteration; top-level returns directly to caller.
}

/// Cardinality `(d + p)! / (d! · p!)` of the total-degree-truncated
/// PCE basis. Closed-form binomial coefficient — sidesteps the
/// `O(|basis|)` enumeration when only the count is needed.
#[must_use]
pub fn total_degree_basis_size(d: usize, max_degree: usize) -> usize {
    // Compute (d + p choose p) iteratively to avoid overflow on
    // intermediate factorials.
    let n = d + max_degree;
    let k = max_degree.min(d);
    let mut result: u128 = 1;
    for i in 0..k {
        result = result * (n - i) as u128 / (i + 1) as u128;
    }
    result as usize
}

/// Hyperbolic q-norm of a multi-index: `|α|_q = (Σ αⱼ^q)^{1/q}`,
/// `q ∈ (0, 1]`. At `q = 1` this reduces to total-degree
/// `Σ αⱼ`; at `q < 1`, it weights interaction terms more heavily,
/// favoring main-effect indices in the truncated basis (Blatman-
/// Sudret 2011 § 3.2).
#[must_use]
fn hyperbolic_norm(alpha: &[usize], q: f64) -> f64 {
    let sum: f64 = alpha
        .iter()
        .filter(|&&v| v > 0)
        .map(|&v| (v as f64).powf(q))
        .sum();
    if sum == 0.0 {
        0.0
    } else {
        sum.powf(1.0 / q)
    }
}

/// Enumerate all multi-indices `α ∈ ℕ^d` with `|α|_q ≤ max_degree`,
/// where `|α|_q = (Σ αⱼ^q)^{1/q}` is the hyperbolic q-norm
/// (Blatman-Sudret 2011 § 3.2). At `q = 1` reduces to
/// [`enumerate_total_degree`]; at `q < 1` keeps fewer interaction
/// terms (favoring sparsity for high-`d` PCE workloads).
///
/// The implementation enumerates the total-degree basis and filters
/// — total-degree is a strict superset of hyperbolic for `q ≤ 1`,
/// so this is correct and `O(|total-degree basis|)` work for the
/// enumeration, which is acceptable since hyperbolic is the
/// regime where total-degree is feasible to enumerate but ill-
/// suited as a sparse-PCE basis.
///
/// # Errors
///
/// - [`MultiIndexError::ZeroD`] if `d == 0`.
/// - [`MultiIndexError::InvalidHyperbolicQ`] if `q ∉ (0, 1]`.
pub fn enumerate_hyperbolic(
    d: usize,
    max_degree: usize,
    q: f64,
) -> Result<Vec<MultiIndex>, MultiIndexError> {
    if d == 0 {
        return Err(MultiIndexError::ZeroD);
    }
    if !(q.is_finite() && q > 0.0 && q <= 1.0) {
        return Err(MultiIndexError::InvalidHyperbolicQ { q });
    }
    let total = enumerate_total_degree(d, max_degree)?;
    #[allow(clippy::cast_precision_loss)]
    let bound = max_degree as f64;
    let kept: Vec<MultiIndex> = total
        .into_iter()
        .filter(|mi| hyperbolic_norm(&mi.indices, q) <= bound + 1e-12)
        .collect();
    Ok(kept)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    // ── MultiIndex basics ────────────────────────────────────────

    #[test]
    fn total_degree_sums_indices() {
        let mi = MultiIndex::new(vec![2, 0, 3]);
        assert_eq!(mi.total_degree(), 5);
        assert_eq!(mi.dim(), 3);
    }

    #[test]
    fn zero_multi_index_detection() {
        assert!(MultiIndex::new(vec![0, 0, 0]).is_zero());
        assert!(!MultiIndex::new(vec![1, 0, 0]).is_zero());
    }

    #[test]
    fn main_effect_detection() {
        assert!(MultiIndex::new(vec![3, 0, 0]).is_main_effect());
        assert!(MultiIndex::new(vec![0, 0, 5]).is_main_effect());
        // Constant is NOT a main effect (no factor active).
        assert!(!MultiIndex::new(vec![0, 0, 0]).is_main_effect());
        // Interactions are NOT main effects.
        assert!(!MultiIndex::new(vec![1, 1, 0]).is_main_effect());
    }

    #[test]
    fn active_factors_returns_nonzero_positions() {
        assert_eq!(
            MultiIndex::new(vec![2, 0, 3, 0]).active_factors(),
            vec![0, 2]
        );
        assert_eq!(
            MultiIndex::new(vec![0, 0, 0]).active_factors(),
            Vec::<usize>::new()
        );
        assert_eq!(MultiIndex::new(vec![5]).active_factors(), vec![0]);
    }

    // ── Enumeration ──────────────────────────────────────────────

    #[test]
    fn zero_d_errors() {
        assert_eq!(
            enumerate_total_degree(0, 3).unwrap_err(),
            MultiIndexError::ZeroD
        );
    }

    #[test]
    fn count_matches_binomial_formula() {
        for d in 1..=6 {
            for p in 0..=5 {
                let basis = enumerate_total_degree(d, p).unwrap();
                let expected = total_degree_basis_size(d, p);
                assert_eq!(
                    basis.len(),
                    expected,
                    "d={d}, p={p}: enumerated {} vs formula {}",
                    basis.len(),
                    expected
                );
            }
        }
    }

    #[test]
    fn count_specific_cases() {
        // (d + p)! / (d! · p!).
        // d=3, p=4: 7!/(3!4!) = 35.
        assert_eq!(enumerate_total_degree(3, 4).unwrap().len(), 35);
        // d=2, p=2: 4!/(2!2!) = 6.
        assert_eq!(enumerate_total_degree(2, 2).unwrap().len(), 6);
        // d=5, p=0: 1 (just the constant).
        assert_eq!(enumerate_total_degree(5, 0).unwrap().len(), 1);
        // d=1, p=10: 11.
        assert_eq!(enumerate_total_degree(1, 10).unwrap().len(), 11);
    }

    #[test]
    fn first_index_is_zero_multi_index() {
        let basis = enumerate_total_degree(4, 3).unwrap();
        assert!(basis[0].is_zero());
    }

    #[test]
    fn no_index_exceeds_max_degree() {
        let basis = enumerate_total_degree(3, 4).unwrap();
        for mi in &basis {
            assert!(
                mi.total_degree() <= 4,
                "found multi-index with |α| = {} > 4: {:?}",
                mi.total_degree(),
                mi
            );
        }
    }

    #[test]
    fn enumeration_is_unique() {
        let basis = enumerate_total_degree(3, 3).unwrap();
        for i in 0..basis.len() {
            for j in (i + 1)..basis.len() {
                assert_ne!(
                    basis[i], basis[j],
                    "duplicate at i={i}, j={j}: {:?}",
                    basis[i]
                );
            }
        }
    }

    #[test]
    fn lex_order_first_index_varies_slowest() {
        // In our recursive enumeration, position 0 varies slowest
        // (outermost loop), so the basis groups by position-0 value.
        let basis = enumerate_total_degree(3, 2).unwrap();
        // Expected (lex over (a, b, c) with a + b + c ≤ 2):
        //   (0,0,0), (0,0,1), (0,0,2),
        //   (0,1,0), (0,1,1), (0,2,0),
        //   (1,0,0), (1,0,1), (1,1,0), (2,0,0)
        // = 10 indices.
        assert_eq!(basis.len(), 10);
        assert_eq!(basis[0].indices, vec![0, 0, 0]);
        assert_eq!(basis[1].indices, vec![0, 0, 1]);
        assert_eq!(basis[2].indices, vec![0, 0, 2]);
    }

    // ── Sanity: counting binomials directly ─────────────────────

    #[test]
    fn total_degree_basis_size_matches_pascal() {
        // (d + p choose p) = (d + p choose d). Symmetry check.
        assert_eq!(total_degree_basis_size(3, 5), total_degree_basis_size(5, 3));
        assert_eq!(total_degree_basis_size(0, 5), 1); // (5 choose 5) = 1
        assert_eq!(total_degree_basis_size(5, 0), 1);
        assert_eq!(total_degree_basis_size(10, 4), 1001);
    }

    // ── Hyperbolic q-norm truncation ─────────────────────────────

    #[test]
    fn hyperbolic_at_q_one_equals_total_degree() {
        // |α|_1 = Σ α_j = total degree.
        for d in 1..=5 {
            for p in 0..=4 {
                let total = enumerate_total_degree(d, p).unwrap();
                let hyper = enumerate_hyperbolic(d, p, 1.0).unwrap();
                assert_eq!(
                    total.len(),
                    hyper.len(),
                    "d={d}, p={p}: total {} vs hyper@q=1 {}",
                    total.len(),
                    hyper.len()
                );
            }
        }
    }

    #[test]
    fn hyperbolic_at_q_below_one_is_subset_of_total_degree() {
        // For q ∈ (0, 1) and any α ≠ 0, |α|_q ≥ |α|_1 (since α_j^q
        // grows faster than α_j when α_j > 1). So hyperbolic basis
        // ⊆ total-degree basis.
        for q in [0.25, 0.5, 0.75] {
            let total = enumerate_total_degree(5, 4).unwrap();
            let hyper = enumerate_hyperbolic(5, 4, q).unwrap();
            assert!(
                hyper.len() <= total.len(),
                "q={q}: hyper {} > total {}",
                hyper.len(),
                total.len()
            );
            // Containment: every hyper index is in total.
            for h in &hyper {
                assert!(total.contains(h), "q={q}: hyper {h:?} not in total");
            }
        }
    }

    #[test]
    fn hyperbolic_at_small_q_keeps_only_main_effects_and_constant() {
        // At q → 0, |α|_q → |support(α)| = number of active factors
        // for α ≠ 0. Concretely at q = 0.01, |α|_q for α = (1, 1, 0)
        // is (2)^100 ≫ p, so two-factor interactions are excluded
        // even at moderate p.
        let kept = enumerate_hyperbolic(5, 4, 0.01).unwrap();
        for mi in &kept {
            // Either constant or main effect.
            assert!(
                mi.is_zero() || mi.is_main_effect(),
                "q≈0: kept non-main-effect {mi:?}"
            );
        }
    }

    #[test]
    fn hyperbolic_q_out_of_range_errors() {
        assert!(matches!(
            enumerate_hyperbolic(3, 4, 0.0).unwrap_err(),
            MultiIndexError::InvalidHyperbolicQ { .. }
        ));
        assert!(matches!(
            enumerate_hyperbolic(3, 4, 1.5).unwrap_err(),
            MultiIndexError::InvalidHyperbolicQ { .. }
        ));
        assert!(matches!(
            enumerate_hyperbolic(3, 4, f64::NAN).unwrap_err(),
            MultiIndexError::InvalidHyperbolicQ { .. }
        ));
    }

    #[test]
    fn hyperbolic_zero_d_errors() {
        assert_eq!(
            enumerate_hyperbolic(0, 3, 0.75).unwrap_err(),
            MultiIndexError::ZeroD
        );
    }

    #[test]
    fn hyperbolic_first_index_is_zero() {
        let basis = enumerate_hyperbolic(4, 3, 0.75).unwrap();
        assert!(basis[0].is_zero());
    }

    #[test]
    fn hyperbolic_substantially_smaller_at_high_d() {
        // The Blatman-Sudret pay-off: at d=10, p=4, total-degree = 1001;
        // hyperbolic q=0.5 should be < 200.
        let total = total_degree_basis_size(10, 4);
        let hyper = enumerate_hyperbolic(10, 4, 0.5).unwrap().len();
        assert_eq!(total, 1001);
        assert!(
            hyper < 200,
            "d=10, p=4, q=0.5: hyper basis = {hyper}, expected < 200"
        );
    }
}
