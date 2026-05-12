//! Sparse Polynomial Chaos Expansion via forward-selection solvers
//! with leave-one-out cross-validation stopping (Blatman-Sudret 2011,
//! Blatman 2009 thesis Ch 3-4). Same [`PolynomialChaos`] output type
//! as [`crate::pce::fit_full_pce`] (PR 16b), so
//! [`crate::pce::sobol_indices_from_pce`] works unchanged.
//!
//! # Two solvers, run in parallel for evaluation
//!
//! - [`SparseSolver::Omp`] — **Orthogonal Matching Pursuit**
//!   (Pati-Rezaiifar-Krishnaprasad 1993; Blatman 2009 § 3.2 names
//!   it as a valid PCE alternative). At each step pick the basis
//!   column most correlated with the residual, add to the active
//!   set, refit OLS on the active set, recompute residual, stop on
//!   LOO-CV upturn or `max_terms`. Trivial to implement, robust to
//!   noise; the workhorse.
//!
//! - [`SparseSolver::Lars`] — **Least Angle Regression**
//!   (Efron-Hastie-Johnstone-Tibshirani 2004 § 2). At each step add
//!   the most-correlated column to the active set, then move all
//!   active coefficients along the *equiangular* direction `u_A`
//!   (the unit vector making equal angles with every column of the
//!   sign-flipped active matrix), shrinking the maximal correlation
//!   uniformly until a new column matches it. Stop on LOO-CV upturn
//!   under the LARS-OLS hybrid (refit OLS at each step end for the
//!   error evaluation) per Blatman 2009.
//!
//! Patrick's framing: "we are definitely thunderdoming our own
//! metrics" — both solvers ship as first-class options so a workload
//! can pick the one that fits, and so we can compare on the same
//! Ishigami fixture.
//!
//! # Hyperbolic q-norm truncation
//!
//! Independent of solver choice. [`TruncationScheme::Hyperbolic`]
//! filters the candidate basis to multi-indices with
//! `(Σ αⱼ^q)^{1/q} ≤ p`, suppressing high-interaction terms that
//! sparse methods would discard anyway. Per Blatman-Sudret 2011
//! § 3.2; default `q = 0.75` is their recommendation.
//!
//! # LOO-CV closed form (Allen's PRESS)
//!
//! For OLS on an active basis `Ψ_A` of `k` columns:
//!
//! ```text
//! LOO_err = (1/N) · Σᵢ ((yᵢ - ŷᵢ) / (1 - hᵢᵢ))²
//! ```
//!
//! where `hᵢᵢ` is the i-th diagonal of the hat matrix
//! `H = Ψ_A (Ψ_Aᵀ Ψ_A)⁻¹ Ψ_Aᵀ`. No need to refit `N` times. Computed
//! per step from the current Cholesky factor. Diverges when
//! `1 - hᵢᵢ → 0` (an active row perfectly determined by the basis);
//! we treat `hᵢᵢ ≥ 1 - 1e-10` as a singularity and bail.
//!
//! # Cost
//!
//! - OMP: `O(K · (NP + k³))` for `K` steps and active-set size `k`,
//!   where `P` is the candidate basis size.
//! - LARS: `O(K · (NP + k³))` per step (same complexity; the
//!   equiangular-vector solve is the same `O(k³)` Cholesky).
//!
//! # Output
//!
//! Returns a [`PolynomialChaos`] with `coefficients` and
//! `multi_indices` aligned across the *full* candidate basis —
//! pruned columns get coefficient `0.0`. This keeps the structural
//! contract with [`crate::pce::sobol_indices_from_pce`] identical
//! to the full-OLS path; a follow-up may compact the representation.

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::assigning_clones,
    clippy::manual_let_else,
    clippy::needless_range_loop
)]

use nalgebra::{DMatrix, DVector};
use ndarray::Array2;

use crate::multi_index::{enumerate_hyperbolic, enumerate_total_degree, MultiIndex};
use crate::pce::{PceError, PolynomialChaos};
use crate::polynomial::{evaluate, is_in_canonical_domain, PolynomialFamily};

/// Basis truncation scheme for sparse PCE.
///
/// `#[non_exhaustive]` — additional schemes (e.g., adaptive
/// degree-by-degree per Blatman 2009 § 4) land non-breaking.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum TruncationScheme {
    /// `|α| = Σ αⱼ ≤ max_degree`. The PR 16b default.
    TotalDegree,
    /// Hyperbolic q-norm: `(Σ αⱼ^q)^{1/q} ≤ max_degree`,
    /// `q ∈ (0, 1]`. At `q = 1` reduces to total-degree; at `q < 1`
    /// favors low-interaction terms.
    Hyperbolic { q: f64 },
}

/// Sparse-solver choice. See module docstring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SparseSolver {
    /// Orthogonal Matching Pursuit.
    Omp,
    /// Least Angle Regression (Efron 2004).
    Lars,
}

/// Diagnostic carried through the fit. Not part of
/// [`PolynomialChaos`] so the existing analysis surface stays
/// untouched.
///
/// `#[non_exhaustive]` — future fields (per-step LOO trace,
/// equiangular-vector norms) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SparseFitDiagnostic {
    /// Solver used.
    pub solver: SparseSolver,
    /// Truncation scheme used.
    pub truncation: TruncationScheme,
    /// Number of non-zero coefficients in the final fit.
    pub num_active: usize,
    /// Total candidate basis size before sparse selection.
    pub candidate_basis_size: usize,
    /// Final LOO-CV error (the minimum found during forward
    /// selection — i.e., the model returned).
    pub loo_error: f64,
    /// Step at which the minimum LOO was reached (zero-indexed).
    pub best_step: usize,
}

/// Fit a sparse PCE via the chosen forward-selection solver.
///
/// Same `samples_canonical` / `families` / `max_degree` contract as
/// [`crate::pce::fit_full_pce`]. `max_terms` caps the active set
/// size; if `None`, defaults to `min(N - 1, basis_size)` (the OLS
/// well-posedness limit).
///
/// # Errors
///
/// - [`PceError::ShapeMismatch`] / [`PceError::ZeroD`] /
///   [`PceError::FamiliesDimMismatch`] — input shape errors.
/// - [`PceError::InsufficientSamples`] if `N` is too small to fit
///   even a constant via OLS (`N < 2`).
/// - [`PceError::SingularDesignMatrix`] if the active-set Gram matrix
///   becomes singular (collinearity in the chosen subset).
pub fn fit_sparse_pce(
    samples_canonical: &Array2<f64>,
    y: &[f64],
    families: &[PolynomialFamily],
    max_degree: usize,
    truncation: TruncationScheme,
    solver: SparseSolver,
    max_terms: Option<usize>,
) -> Result<(PolynomialChaos, SparseFitDiagnostic), PceError> {
    let n = samples_canonical.nrows();
    let d = samples_canonical.ncols();
    if d == 0 {
        return Err(PceError::ZeroD);
    }
    if y.len() != n {
        return Err(PceError::ShapeMismatch {
            x_rows: n,
            y_len: y.len(),
        });
    }
    if families.len() != d {
        return Err(PceError::FamiliesDimMismatch {
            families_len: families.len(),
            d,
        });
    }
    if n < 2 {
        return Err(PceError::InsufficientSamples { n, basis_size: 1 });
    }

    // Caller-side canonical-domain debug-assert (mirrors fit_full_pce).
    #[cfg(debug_assertions)]
    {
        for (k, family) in families.iter().enumerate() {
            let xs = samples_canonical.column(k);
            let in_domain = xs.iter().all(|&x| is_in_canonical_domain(*family, x));
            debug_assert!(
                in_domain,
                "sparse PCE: column {k} contains values outside {family:?}'s canonical domain"
            );
        }
    }

    // Build the candidate basis per truncation scheme.
    let multi_indices: Vec<MultiIndex> = match truncation {
        TruncationScheme::TotalDegree => {
            enumerate_total_degree(d, max_degree).map_err(|_| PceError::ZeroD)?
        }
        TruncationScheme::Hyperbolic { q } => {
            enumerate_hyperbolic(d, max_degree, q).map_err(|_| PceError::ZeroD)?
        }
    };
    let basis_size = multi_indices.len();

    // Build the full basis matrix Ψ ∈ R^{N × P}. Sparse solvers will
    // select columns from this; we materialize once.
    let mut psi = DMatrix::<f64>::zeros(n, basis_size);
    for i in 0..n {
        for (j, alpha) in multi_indices.iter().enumerate() {
            let mut value = 1.0;
            for (k, &deg) in alpha.indices.iter().enumerate() {
                value *= evaluate(families[k], deg, samples_canonical[[i, k]]);
            }
            psi[(i, j)] = value;
        }
    }
    let y_vec = DVector::from_iterator(n, y.iter().copied());

    let max_terms_eff = max_terms
        .unwrap_or(usize::MAX)
        .min(basis_size)
        .min(n.saturating_sub(1));
    if max_terms_eff == 0 {
        return Err(PceError::InsufficientSamples { n, basis_size });
    }

    let (active, coefficients_active, diag) = match solver {
        SparseSolver::Omp => omp_forward_select(&psi, &y_vec, max_terms_eff)?,
        SparseSolver::Lars => lars_forward_select(&psi, &y_vec, max_terms_eff)?,
    };

    // Scatter active coefficients back to the full basis.
    let mut coefficients = vec![0.0_f64; basis_size];
    for (idx, &j) in active.iter().enumerate() {
        coefficients[j] = coefficients_active[idx];
    }

    let pce = PolynomialChaos {
        coefficients,
        multi_indices,
        families: families.to_vec(),
        max_degree,
    };

    let diagnostic = SparseFitDiagnostic {
        solver,
        truncation,
        num_active: active.len(),
        candidate_basis_size: basis_size,
        loo_error: diag.best_loo,
        best_step: diag.best_step,
    };

    Ok((pce, diagnostic))
}

struct ForwardSelectDiag {
    best_loo: f64,
    best_step: usize,
}

/// OMP forward selection. Returns the active-column indices (into
/// `psi`'s columns), their OLS coefficients, and the diagnostic.
fn omp_forward_select(
    psi: &DMatrix<f64>,
    y: &DVector<f64>,
    max_terms: usize,
) -> Result<(Vec<usize>, DVector<f64>, ForwardSelectDiag), PceError> {
    let n = psi.nrows();

    // The constant column (index 0 in our enumeration) is always
    // active — every PCE has a mean term. Initialize there.
    let mut active: Vec<usize> = vec![0];
    let mut best_loo = f64::INFINITY;
    let mut best_step = 0;
    let mut best_active: Vec<usize> = active.clone();
    let mut best_beta: DVector<f64> = DVector::zeros(1);

    let mut consecutive_increases = 0_usize;

    for step in 0..max_terms {
        // Refit OLS on the current active set.
        let (beta, hat_diag) = refit_active_ols(psi, y, &active)?;
        let loo = loo_error_from_hat(psi, y, &active, &beta, &hat_diag);

        if loo < best_loo {
            best_loo = loo;
            best_step = step;
            best_active = active.clone();
            best_beta = beta.clone();
            consecutive_increases = 0;
        } else {
            consecutive_increases += 1;
        }

        // Stop if LOO has been increasing for several consecutive
        // steps — local noise can cause one-step bumps; require a
        // sustained trend before giving up.
        if consecutive_increases >= 3 {
            break;
        }

        // Compute residual using current OLS fit.
        let psi_a = active_columns(psi, &active);
        let residual = y - &psi_a * beta;

        if active.len() >= max_terms {
            break;
        }

        // Pick the inactive column most correlated with residual.
        let next_idx = match best_inactive_correlation(psi, &residual, &active) {
            Some(j) => j,
            None => break,
        };
        active.push(next_idx);

        // Guard against numerical underflow on residual norm.
        if residual.norm() < 1e-14 * y.norm().max(1.0) {
            // Fit is already exact on training data — finalize and stop.
            let (beta, hat_diag) = refit_active_ols(psi, y, &active)?;
            let loo = loo_error_from_hat(psi, y, &active, &beta, &hat_diag);
            if loo < best_loo {
                best_loo = loo;
                best_step = step + 1;
                best_active = active.clone();
                best_beta = beta;
            }
            break;
        }
    }

    // Defensive: zero-size active set or all-failures means fall back
    // to constant fit (mean of y).
    if best_active.is_empty() {
        best_active = vec![0];
        best_beta = DVector::from_iterator(1, [y.iter().sum::<f64>() / n as f64]);
    }
    Ok((
        best_active,
        best_beta,
        ForwardSelectDiag {
            best_loo,
            best_step,
        },
    ))
}

/// LARS forward selection (Efron 2004 § 2). Returns the active set,
/// LARS-OLS-hybrid coefficients (refit OLS on the final active set),
/// and the diagnostic.
///
/// Efron 2004 eq. (1.1) standardizes columns to unit `ℓ²` norm and
/// centers `y`. Our PCE basis has natively-orthogonal-but-not-unit-
/// norm columns (`||Ψ_α|| ≈ √(N · ⟨Ψ_α, Ψ_α⟩)`), so the equiangular
/// geometry is wrong without rescaling. We standardize internally
/// for path-following only — column selection, equiangular vector,
/// step sizes — then refit OLS on the *un-standardized* columns at
/// each step end (the LARS-OLS hybrid Blatman 2009 § 3.4 advocates).
/// Coefficients returned are in the un-standardized (PCE-natural)
/// scale.
fn lars_forward_select(
    psi: &DMatrix<f64>,
    y: &DVector<f64>,
    max_terms: usize,
) -> Result<(Vec<usize>, DVector<f64>, ForwardSelectDiag), PceError> {
    let n = psi.nrows();
    let p = psi.ncols();

    // ── Standardize for LARS path-following ──────────────────────
    //
    // Center y. Drop the constant column (j = 0) from the LARS
    // selection pool — its job is captured by the y-centering. The
    // OLS-hybrid refit on the un-standardized columns will pull the
    // mean back in via the constant column, which we manually keep
    // in the active set throughout.
    #[allow(clippy::cast_precision_loss)]
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let y_centered = y.map(|v| v - y_mean);

    // Per-column ℓ² norms for j ≥ 1; column 0 (constant) is excluded.
    let mut col_norms = vec![1.0_f64; p];
    for j in 1..p {
        col_norms[j] = psi.column(j).norm();
        if col_norms[j] < 1e-14 {
            // A degenerate-zero column (all samples at a polynomial
            // root) shouldn't happen in practice but if it does, give
            // it a sentinel norm so divisions don't NaN.
            col_norms[j] = 1.0;
        }
    }

    // The constant column always lives in `active` (un-standardized
    // pool index 0); the LARS path operates on standardized columns
    // 1..p only. We track active selections in the un-standardized
    // index space.
    let mut active: Vec<usize> = vec![0];
    // LARS-internal active set (subset of {1..p}, in standardized
    // column space). Distinct from `active` because `active`
    // includes the constant.
    let mut active_lars: Vec<usize> = Vec::new();

    // μ̂ is the LARS path estimate of y_centered, in standardized
    // column space.
    let mut mu = DVector::<f64>::zeros(n);

    let mut best_loo = f64::INFINITY;
    let mut best_step = 0;
    let mut best_active: Vec<usize> = active.clone();
    let mut best_beta: DVector<f64> = {
        let psi_a = active_columns(psi, &active);
        solve_ols(&psi_a, y)?
    };

    let mut consecutive_increases = 0_usize;

    for step in 0..max_terms {
        // Evaluate LARS-OLS hybrid: refit OLS on the
        // un-standardized current active set (constant + LARS picks).
        let (beta, hat_diag) = refit_active_ols(psi, y, &active)?;
        let loo = loo_error_from_hat(psi, y, &active, &beta, &hat_diag);
        if loo < best_loo {
            best_loo = loo;
            best_step = step;
            best_active = active.clone();
            best_beta = beta;
            consecutive_increases = 0;
        } else {
            consecutive_increases += 1;
        }
        if consecutive_increases >= 3 {
            break;
        }
        // `active_lars.len() + 1 >= p` rather than `>= p - 1` to
        // avoid underflow if `p == 0` (unreachable in practice but
        // belt-and-suspenders).
        if active.len() >= max_terms || active_lars.len() + 1 >= p {
            break;
        }

        // Compute current standardized correlations c = Ψ̃ᵀ(ỹ - μ̂).
        let residual = &y_centered - &mu;
        // c[j] for j ≥ 1 is the standardized correlation (j = 0 is
        // skipped). We allocate a length-p vector and zero out j = 0.
        let mut c = DVector::<f64>::zeros(p);
        for j in 1..p {
            c[j] = psi.column(j).dot(&residual) / col_norms[j];
        }

        // Pick next column: max |c[j]| among j ∉ active_lars.
        let active_lars_set: std::collections::HashSet<usize> =
            active_lars.iter().copied().collect();
        let next_idx = {
            let mut best: Option<(usize, f64)> = None;
            for j in 1..p {
                if active_lars_set.contains(&j) {
                    continue;
                }
                let v = c[j].abs();
                if v.is_finite() && best.is_none_or(|(_, prev)| v > prev) {
                    best = Some((j, v));
                }
            }
            match best {
                Some((j, v)) if v > 1e-14 => j,
                _ => break,
            }
        };
        active_lars.push(next_idx);
        active.push(next_idx);

        // Build sign-flipped standardized active matrix X̃_A.
        // s_j = sign(c[j]) using the c computed *before* adding
        // next_idx (so next_idx's sign comes from its own current
        // correlation).
        let signs: Vec<f64> = active_lars
            .iter()
            .map(|&j| if c[j] >= 0.0 { 1.0 } else { -1.0 })
            .collect();
        let mut psi_a_std_signed = DMatrix::<f64>::zeros(n, active_lars.len());
        for (col, (&j, &s)) in active_lars.iter().zip(signs.iter()).enumerate() {
            let v = psi.column(j).clone_owned() * (s / col_norms[j]);
            psi_a_std_signed.set_column(col, &v);
        }

        // Gram G_A = X̃_Aᵀ X̃_A.
        let g_a = psi_a_std_signed.transpose() * &psi_a_std_signed;
        let cholesky = match g_a.clone().cholesky() {
            Some(c) => c,
            None => return Err(PceError::SingularDesignMatrix),
        };
        let ones = DVector::<f64>::from_element(active_lars.len(), 1.0);
        let g_inv_ones = cholesky.solve(&ones);
        let denom = ones.dot(&g_inv_ones);
        if !(denom.is_finite() && denom > 1e-15) {
            return Err(PceError::SingularDesignMatrix);
        }
        let a_a = denom.powf(-0.5); // Efron eq. 2.5
        let w_a = a_a * &g_inv_ones; // eq. 2.6
        let u_a = &psi_a_std_signed * &w_a; // eq. 2.6

        // a = X̃ᵀ u_A — but only j ∉ active_lars matters for the step.
        // Recompute c_max (the active-set absolute correlation) from
        // the LARS-internal pool, *after* adding next_idx (so it
        // matches the new max).
        let c_max: f64 = active_lars
            .iter()
            .map(|&j| c[j].abs())
            .fold(0.0_f64, f64::max);

        let mut gamma_hat = f64::INFINITY;
        let active_lars_set2: std::collections::HashSet<usize> =
            active_lars.iter().copied().collect();
        for j in 1..p {
            if active_lars_set2.contains(&j) {
                continue;
            }
            let aj = psi.column(j).dot(&u_a) / col_norms[j];
            for &candidate in &[(c_max - c[j]) / (a_a - aj), (c_max + c[j]) / (a_a + aj)] {
                if candidate > 1e-12 && candidate < gamma_hat {
                    gamma_hat = candidate;
                }
            }
        }
        // Last-step fallback: if no inactive column gives a finite
        // positive γ̂ (we've effectively exhausted), use the OLS
        // distance |c_max / A_A| to walk fully along u_A — Efron's
        // "final" γ̂ that drives μ̂ to the LS fit on the active set.
        if !gamma_hat.is_finite() {
            gamma_hat = c_max / a_a;
        }

        // Update μ̂ along the equiangular direction by γ̂ · u_A.
        mu += gamma_hat * &u_a;
    }

    Ok((
        best_active,
        best_beta,
        ForwardSelectDiag {
            best_loo,
            best_step,
        },
    ))
}

/// Stack the columns of `psi` indexed by `active` into a dense
/// `(N, |A|)` matrix.
fn active_columns(psi: &DMatrix<f64>, active: &[usize]) -> DMatrix<f64> {
    let mut out = DMatrix::<f64>::zeros(psi.nrows(), active.len());
    for (col, &j) in active.iter().enumerate() {
        out.set_column(col, &psi.column(j).clone_owned());
    }
    out
}

/// OLS via Cholesky on the normal equations.
fn solve_ols(psi_a: &DMatrix<f64>, y: &DVector<f64>) -> Result<DVector<f64>, PceError> {
    let xtx = psi_a.transpose() * psi_a;
    let xty = psi_a.transpose() * y;
    let cholesky = xtx.cholesky().ok_or(PceError::SingularDesignMatrix)?;
    Ok(cholesky.solve(&xty))
}

/// OLS coefficients + hat-matrix diagonal for the active set. The
/// hat-matrix diagonal is `hᵢᵢ = ψᵢᵀ (Ψ_Aᵀ Ψ_A)⁻¹ ψᵢ` where `ψᵢ`
/// is the i-th row of `Ψ_A`.
fn refit_active_ols(
    psi: &DMatrix<f64>,
    y: &DVector<f64>,
    active: &[usize],
) -> Result<(DVector<f64>, Vec<f64>), PceError> {
    let psi_a = active_columns(psi, active);
    let xtx = psi_a.transpose() * &psi_a;
    let xty = psi_a.transpose() * y;
    let cholesky = xtx.cholesky().ok_or(PceError::SingularDesignMatrix)?;
    let beta = cholesky.solve(&xty);

    // Hat-matrix diagonal: h_ii = ψᵢᵀ (XᵀX)⁻¹ ψᵢ. Per row, solve
    // (XᵀX) z = ψᵢ, then h_ii = ψᵢ · z.
    let mut hat_diag = Vec::with_capacity(psi_a.nrows());
    for i in 0..psi_a.nrows() {
        let row = psi_a.row(i).transpose().clone_owned();
        let z = cholesky.solve(&row);
        hat_diag.push(row.dot(&z));
    }
    Ok((beta, hat_diag))
}

/// Allen's PRESS statistic — closed-form leave-one-out CV error.
/// `LOO = (1/N) · Σ ((yᵢ - ŷᵢ) / (1 - hᵢᵢ))²`.
fn loo_error_from_hat(
    psi: &DMatrix<f64>,
    y: &DVector<f64>,
    active: &[usize],
    beta: &DVector<f64>,
    hat_diag: &[f64],
) -> f64 {
    let psi_a = active_columns(psi, active);
    let yhat = &psi_a * beta;
    let n = y.len();
    let mut acc = 0.0_f64;
    let mut effective_n = 0_usize;
    for i in 0..n {
        let denom = 1.0 - hat_diag[i];
        // Guard the singular-row case: hᵢᵢ ≈ 1 means `Ψ_A` perfectly
        // determines yᵢ, so leave-one-out residual is undefined.
        // Skip from the average rather than diverging.
        if denom.abs() < 1e-10 {
            continue;
        }
        let resid = y[i] - yhat[i];
        acc += (resid / denom).powi(2);
        effective_n += 1;
    }
    if effective_n == 0 {
        return f64::INFINITY;
    }
    acc / effective_n as f64
}

/// Pick the inactive column index with the largest absolute inner
/// product against `residual`. Returns `None` if every inactive
/// column is numerically uncorrelated.
fn best_inactive_correlation(
    psi: &DMatrix<f64>,
    residual: &DVector<f64>,
    active: &[usize],
) -> Option<usize> {
    let p = psi.ncols();
    let active_set: std::collections::HashSet<usize> = active.iter().copied().collect();
    let mut best: Option<(usize, f64)> = None;
    for j in 0..p {
        if active_set.contains(&j) {
            continue;
        }
        let col = psi.column(j);
        let dot = col.dot(residual).abs();
        if dot.is_finite() && best.is_none_or(|(_, prev)| dot > prev) {
            best = Some((j, dot));
        }
    }
    best.and_then(|(j, v)| if v > 1e-14 { Some(j) } else { None })
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;

    fn linspace_unit_to_canonical(n: usize, d: usize) -> Array2<f64> {
        let mut x = Array2::<f64>::zeros((n, d));
        for j in 0..d {
            let mut perm: Vec<usize> = (0..n).collect();
            let mut state: u64 = 0x9E37_79B9_7F4A_7C15_u64.wrapping_mul((j as u64).wrapping_add(1));
            for i in (1..n).rev() {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                #[allow(clippy::cast_possible_truncation)]
                let k = (state >> 33) as usize % (i + 1);
                perm.swap(i, k);
            }
            for i in 0..n {
                let unit = (perm[i] as f64 + 0.5) / (n as f64);
                x[[i, j]] = 2.0 * unit - 1.0;
            }
        }
        x
    }

    // ── Validation ────────────────────────────────────────────────

    #[test]
    fn omp_zero_d_errors() {
        let x = Array2::<f64>::zeros((10, 0));
        let y = vec![0.0; 10];
        let err = fit_sparse_pce(
            &x,
            &y,
            &[],
            3,
            TruncationScheme::TotalDegree,
            SparseSolver::Omp,
            None,
        )
        .unwrap_err();
        assert_eq!(err, PceError::ZeroD);
    }

    #[test]
    fn lars_shape_mismatch_errors() {
        let x = Array2::<f64>::zeros((10, 3));
        let y = vec![0.0; 5];
        let err = fit_sparse_pce(
            &x,
            &y,
            &[PolynomialFamily::Legendre; 3],
            3,
            TruncationScheme::TotalDegree,
            SparseSolver::Lars,
            None,
        )
        .unwrap_err();
        assert!(matches!(err, PceError::ShapeMismatch { .. }));
    }

    // ── OMP recovery on closed-form polynomial models ────────────

    #[test]
    fn omp_fits_constant() {
        let n = 64;
        let x = linspace_unit_to_canonical(n, 2);
        let y = vec![5.0; n];
        let (pce, _) = fit_sparse_pce(
            &x,
            &y,
            &[PolynomialFamily::Legendre; 2],
            3,
            TruncationScheme::TotalDegree,
            SparseSolver::Omp,
            None,
        )
        .unwrap();
        assert!((pce.mean() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn omp_picks_out_sparse_additive_active_factors() {
        // Y = ξ_0 + 0.5·ξ_2 + 2·ξ_4 on d = 5. Sparse PCE should pick
        // only the (1,0,0,0,0), (0,0,1,0,0), (0,0,0,0,1) main-effects.
        let n = 256;
        let x = linspace_unit_to_canonical(n, 5);
        let y: Vec<f64> = (0..n)
            .map(|i| x[[i, 0]] + 0.5 * x[[i, 2]] + 2.0 * x[[i, 4]])
            .collect();
        let (pce, diag) = fit_sparse_pce(
            &x,
            &y,
            &[PolynomialFamily::Legendre; 5],
            4,
            TruncationScheme::TotalDegree,
            SparseSolver::Omp,
            None,
        )
        .unwrap();
        // Coefficient at (1,0,0,0,0) should be 1.0; at (0,0,1,0,0)
        // should be 0.5; at (0,0,0,0,1) should be 2.0.
        let find = |target: Vec<usize>| {
            pce.multi_indices
                .iter()
                .position(|a| a.indices == target)
                .map(|i| pce.coefficients[i])
                .unwrap()
        };
        assert!((find(vec![1, 0, 0, 0, 0]) - 1.0).abs() < 1e-3);
        assert!((find(vec![0, 0, 1, 0, 0]) - 0.5).abs() < 1e-3);
        assert!((find(vec![0, 0, 0, 0, 1]) - 2.0).abs() < 1e-3);
        // Sparse: total active terms way under the 126-term basis.
        assert!(
            diag.num_active <= 10,
            "OMP kept {} terms, expected ≤ 10",
            diag.num_active
        );
    }

    // ── LARS recovery ────────────────────────────────────────────

    #[test]
    fn lars_picks_out_sparse_additive_active_factors() {
        let n = 256;
        let x = linspace_unit_to_canonical(n, 5);
        let y: Vec<f64> = (0..n)
            .map(|i| x[[i, 0]] + 0.5 * x[[i, 2]] + 2.0 * x[[i, 4]])
            .collect();
        let (pce, diag) = fit_sparse_pce(
            &x,
            &y,
            &[PolynomialFamily::Legendre; 5],
            4,
            TruncationScheme::TotalDegree,
            SparseSolver::Lars,
            None,
        )
        .unwrap();
        let find = |target: Vec<usize>| {
            pce.multi_indices
                .iter()
                .position(|a| a.indices == target)
                .map(|i| pce.coefficients[i])
                .unwrap()
        };
        // Coefficients should be near closed-form.
        assert!((find(vec![1, 0, 0, 0, 0]) - 1.0).abs() < 1e-3);
        assert!((find(vec![0, 0, 1, 0, 0]) - 0.5).abs() < 1e-3);
        assert!((find(vec![0, 0, 0, 0, 1]) - 2.0).abs() < 1e-3);
        // LARS may keep a couple more terms than OMP near the LOO
        // minimum (equiangular movement is gentler than greedy).
        assert!(diag.num_active <= 15, "LARS kept {} terms", diag.num_active);
    }

    // ── Hyperbolic truncation ────────────────────────────────────

    #[test]
    fn hyperbolic_truncation_used_at_q_0p75() {
        let n = 256;
        let x = linspace_unit_to_canonical(n, 5);
        let y: Vec<f64> = (0..n).map(|i| x[[i, 0]] + 2.0 * x[[i, 4]]).collect();
        let (_pce, diag) = fit_sparse_pce(
            &x,
            &y,
            &[PolynomialFamily::Legendre; 5],
            4,
            TruncationScheme::Hyperbolic { q: 0.75 },
            SparseSolver::Omp,
            None,
        )
        .unwrap();
        // Hyperbolic at d=5, p=4, q=0.75 should be substantially
        // smaller than total-degree (126).
        assert!(
            diag.candidate_basis_size < 126,
            "hyperbolic basis size = {} should be < 126",
            diag.candidate_basis_size
        );
    }

    // ── Determinism ──────────────────────────────────────────────

    #[test]
    fn omp_is_deterministic() {
        let n = 128;
        let x = linspace_unit_to_canonical(n, 3);
        let y: Vec<f64> = (0..n).map(|i| x[[i, 0]] + x[[i, 1]] + x[[i, 2]]).collect();
        let a = fit_sparse_pce(
            &x,
            &y,
            &[PolynomialFamily::Legendre; 3],
            3,
            TruncationScheme::TotalDegree,
            SparseSolver::Omp,
            None,
        )
        .unwrap()
        .0;
        let b = fit_sparse_pce(
            &x,
            &y,
            &[PolynomialFamily::Legendre; 3],
            3,
            TruncationScheme::TotalDegree,
            SparseSolver::Omp,
            None,
        )
        .unwrap()
        .0;
        assert_eq!(a.coefficients, b.coefficients);
    }

    #[test]
    fn lars_is_deterministic() {
        let n = 128;
        let x = linspace_unit_to_canonical(n, 3);
        let y: Vec<f64> = (0..n).map(|i| x[[i, 0]] + x[[i, 1]] + x[[i, 2]]).collect();
        let a = fit_sparse_pce(
            &x,
            &y,
            &[PolynomialFamily::Legendre; 3],
            3,
            TruncationScheme::TotalDegree,
            SparseSolver::Lars,
            None,
        )
        .unwrap()
        .0;
        let b = fit_sparse_pce(
            &x,
            &y,
            &[PolynomialFamily::Legendre; 3],
            3,
            TruncationScheme::TotalDegree,
            SparseSolver::Lars,
            None,
        )
        .unwrap()
        .0;
        assert_eq!(a.coefficients, b.coefficients);
    }
}
