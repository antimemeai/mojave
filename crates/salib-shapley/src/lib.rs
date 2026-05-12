//! `salib-shapley` — Shapley effects for global sensitivity
//! analysis (Song-Nelson-Staum 2016).
//!
//! # What Shapley effects measure
//!
//! Shapley effects are a third game-theoretic-flavored sensitivity
//! measure alongside variance-based first-order (`S_i`) and total-
//! order (`S_T_i`) Sobol' indices. The defining property
//! (Song 2016 Eq 10):
//!
//! ```text
//! Σᵢ Sh_i = Var(Y)
//! ```
//!
//! — exactly, even under input dependence or structural interactions.
//! First-order and total-order Sobol' don't have this property under
//! correlation: `Σ S_i` can exceed `Var(Y)` when inputs are positively
//! correlated, and `Σ S_T_i` can fall short when correlation absorbs
//! interaction. Shapley effects are the only semivalue that always
//! sums to the cooperative game's grand coalition cost (Carreras-Giménez
//! 2011).
//!
//! Under independent inputs (our scope), Song 2016 Theorem 2 gives
//! the natural ordering:
//!
//! ```text
//! V_i ≤ Sh_i ≤ V_T_i      (first-order ≤ Shapley ≤ total-order)
//! ```
//!
//! where `V_i = Var(E[Y | X_i])` and `V_T_i = E[Var(Y | X_{-i})]` are
//! the un-normalized variance contributions. Shapley splits the
//! interaction effects evenly across involved factors instead of
//! assigning them entirely to either main-effect or total-effect.
//!
//! # Algorithm — Song-Nelson-Staum 2016 Algorithm 1
//!
//! Direct computation requires evaluating the cost function `c(J) =
//! E[Var(Y | X_{-J})]` for all `2^k − 1` non-empty coalitions `J ⊆ K`,
//! plus `k!` permutation orderings — combinatorial explosion at
//! `k > 10`. The algorithm sidesteps this with two ideas:
//!
//! 1. **Random-permutation sampling** (Castro-Gómez-Cazorla 2009
//!    "ApproShapley"): sample `m` random permutations `π₁, …, π_m`
//!    of `K`, accumulate marginals `Δ_{π(j)}c(π) = c(prefix_j) -
//!    c(prefix_{j-1})` per permutation, average. Per-factor
//!    `Sh_j ≈ (1/m) Σ Δ_j(π_ℓ)`.
//!
//! 2. **Sequential cost evaluation** (Song 2016 § 4.1 Eq 13): walk
//!    `j = 1..k` along each permutation, reusing the previous
//!    iteration's `c(prefix_{j-1})` (cached as `prevC`). Halves the
//!    cost-evaluation budget vs `ApproShapley`'s redundant evaluation.
//!
//! 3. **Double-loop MC for the cost** (Song 2016 § 4.2): each
//!    `c(J) = E_{X_{-J}}[Var_{X_J}(Y | X_{-J})]` is estimated with
//!    `N_O` outer samples of `X_{-J}` and `N_I` inner samples of
//!    `X_J | X_{-J}`. Boundary: `c(K) = Var(Y)` (no conditioning).
//!
//! Total budget: `N_V + m · N_I · N_O · (k − 1)` model evaluations.
//! Song 2016 Appendix B recommends `N_I = 3, N_O = 1`, with `m`
//! consuming the remaining budget.
//!
//! # Independent-inputs scope
//!
//! PR 17 ships independent-input Shapley only. `c(J)`'s outer step
//! "sample `X_{-J}` from the marginal" reduces to independent
//! per-factor draws via [`Distribution::quantile`]; the inner step
//! "sample `X_J | X_{-J}`" reduces to fresh independent draws because
//! `X_J ⊥ X_{-J}` under independence.
//!
//! Dependent-input Shapley via Iman-Conover transformation +
//! conditional sampling is bead-tracked (`workspace-bsj` for the
//! full copula library; `workspace-gos` for given-data k-NN
//! Shapley).
//!
//! # Determinism
//!
//! Same `RngState` in → bit-identical `ShapleyIndices` out. The
//! permutation sequence and all sample draws derive deterministically
//! from the input RNG state.
//!
//! This guarantee assumes the `model` closure is a *pure function* of
//! its argument (or otherwise deterministic across calls). A closure
//! that mutates captured state (counters, accumulators, internal
//! RNGs) defeats the determinism contract — the `RngState` invariant
//! covers only the *sample inputs* the closure receives.
//!
//! # Crate boundaries
//!
//! Depends on `salib-core` (`Distribution`, `RngState`,
//! `tree_sum`/`tree_var`). workspace-agnostic. Sister to
//! `salib-estimators` and `salib-surrogate`; distinct crate
//! because the algorithm structure (permutation walks + double-loop
//! MC + closure-heavy budget) is unlike either direct-MC or
//! surrogate-flow estimators.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod estimator;

pub use estimator::{estimate_shapley, ShapleyError, ShapleyIndices};
