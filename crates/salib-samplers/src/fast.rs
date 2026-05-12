//! FAST search-curve sampler — Saltelli-Tarantola-Chan 1999 (a.k.a.
//! eFAST in the literature). Surfaces under the API name `fast` for
//! `SALib`-affinity per
//! `decisions/2026-04-29-saltelli-fast-sampler.md`.
//!
//! # The design
//!
//! For each factor-of-interest `i ∈ 0..d`, factor `i` gets the
//! maximum frequency `ω_max`; the remaining `d − 1` factors get
//! complementary frequencies bounded above by `ω_max / (2·M)` (so
//! their spectra do not overlap `ω_max`'s harmonic band up to order
//! `M`). The sampler emits `n_per_factor` points along each search
//! curve, stacked as an `(n_per_factor · d, d)` matrix.
//!
//! Search curve transformation (uniform marginal on `[0, 1]`):
//!
//! ```text
//! x[i·N + n, j] = 1/2 + (1/π) · arcsin(sin(ω[i, j] · s_n + φ[i, j]))
//! s_n           = (2π / N) · n,           n ∈ 0..N
//! ```
//!
//! `ω[i, j]` is `ω_max` when `j == i` and one of the `d − 1`
//! complementary frequencies otherwise. `φ[i, j] ~ Uniform[0, 2π]`
//! drawn deterministically from the input `RngState`.
//!
//! # Frequency selection (Saltelli 1999 § 3.2)
//!
//! ```text
//! ω_max = floor((N − 1) / (2 · M))
//! m     = floor(ω_max / (2 · M))
//!
//! if m ≥ d − 1:
//!     complementary[k] = round(linspace(1, m, d − 1)[k])
//! else:
//!     complementary[k] = (k mod m) + 1                         # `SALib` parity
//! ```
//!
//! # Determinism
//!
//! Pure under `(d, n_per_factor, harmonic, RngState)`. Same
//! `RngState` in → bit-identical `FastDesign` out, with
//! `RngState::word_pos` advanced to reflect consumed bytes.
//!
//! # Cost
//!
//! `n_per_factor · d` model evaluations. With `SALib` defaults
//! (`M = 4`, `N = 65`, `ω_max = 8`), 65 · `d` evals — comparable to
//! Morris but with frequency-domain `Sᵀᵢ` recovery rather than
//! finite-difference.
//!
//! # What this module does NOT ship
//!
//! - **Spectral estimator** returning `Sᵢ` and `Sᵀᵢ` — PR 9b.
//! - **Classical Cukier 1973 FAST** (different `G_i` transformation).
//!   Bead-eligible if a historical-reproducibility use case lands.
//! - **RBD-FAST** (Tarantola 2006, Plischke 2010) — different
//!   algorithm operating on given data; PR 10.
//! - **`Sampler` trait `impl`.** FAST output is intrinsically
//!   blocked with load-bearing frequency / phase metadata; exposed
//!   as a free function returning `FastDesign`.

use std::f64::consts::PI;

use ndarray::Array2;
use rand::RngCore;
use salib_core::RngState;

/// Output of [`build_fast_design`].
///
/// `#[non_exhaustive]` — future fields (`recorded_rng_state` for
/// audit replay, `kind: FastKind` if classical Cukier lands,
/// `random_phase: bool` config echo) land non-breaking.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FastDesign {
    /// `(n_per_factor · d, d)` row-major sample matrix. Rows
    /// `[i · n_per_factor, (i + 1) · n_per_factor)` are the samples
    /// for factor-of-interest `i`.
    pub samples: Array2<f64>,
    /// `(d, d)` frequency assignments. `omegas[[i, j]]` is the
    /// frequency assigned to factor `j` when `i` is the
    /// factor-of-interest. `omegas[[i, i]] = ω_max` for every `i`.
    pub omegas: Array2<u32>,
    /// `(d, d)` phase shifts in radians. `phases[[i, j]] ∈ [0, 2π)`.
    pub phases: Array2<f64>,
    /// Number of samples per factor-of-interest. Total samples =
    /// `n_per_factor · d`.
    pub n_per_factor: usize,
    /// Factor count.
    pub d: usize,
    /// Harmonic order `M` (typically `4` per `SALib` default).
    pub harmonic: u32,
}

/// Errors from [`build_fast_design`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FastError {
    #[error("FAST: d must be ≥ 1, got 0")]
    ZeroD,
    #[error("FAST: harmonic must be ≥ 1, got 0")]
    ZeroHarmonic,
    #[error(
        "FAST: n_per_factor must satisfy n_per_factor ≥ 4·harmonic² + 1 \
         (got n_per_factor={n_per_factor}, harmonic={harmonic}, \
         minimum={minimum}); else m = floor(ω_max / (2·harmonic)) collapses \
         to 0 and the complementary frequency budget vanishes"
    )]
    InsufficientSamples {
        n_per_factor: usize,
        harmonic: u32,
        minimum: usize,
    },
}

/// Build a Saltelli-Tarantola-Chan 1999 FAST search-curve design for
/// a `d`-factor problem with `n_per_factor` samples per factor-of-
/// interest at harmonic order `harmonic` (typically `4`).
///
/// Total cost: `n_per_factor · d` model evaluations.
///
/// # Errors
///
/// - [`FastError::ZeroD`] if `d == 0`.
/// - [`FastError::ZeroHarmonic`] if `harmonic == 0`.
/// - [`FastError::InsufficientSamples`] if `n_per_factor < 4 · harmonic² + 1`
///   (would yield `m = floor(ω_max / (2·M)) = 0`, no bandwidth budget for
///   the complementary set).
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
pub fn build_fast_design(
    d: usize,
    n_per_factor: usize,
    harmonic: u32,
    rng: &mut RngState,
) -> Result<FastDesign, FastError> {
    if d == 0 {
        return Err(FastError::ZeroD);
    }
    if harmonic == 0 {
        return Err(FastError::ZeroHarmonic);
    }
    // n ≥ 4M² + 1 ⇒ ω_max ≥ 2M ⇒ m ≥ 1. Guarantees that `ω_max` is
    // strictly the maximum entry per row of `omegas` (no ties with
    // complementary entries) and that the linspace / cycling
    // branches in `complementary_frequencies` always produce at
    // least one valid frequency.
    let minimum = 4 * (harmonic as usize) * (harmonic as usize) + 1;
    if n_per_factor < minimum {
        return Err(FastError::InsufficientSamples {
            n_per_factor,
            harmonic,
            minimum,
        });
    }

    let omega_max = ((n_per_factor - 1) / (2 * harmonic as usize)) as u32;
    let complementary = complementary_frequencies(d, omega_max, harmonic);

    let mut chacha = rng.clone().into_chacha();

    // Phase draws: d² uniform[0, 2π) values.
    let mut phases = Array2::<f64>::zeros((d, d));
    for i in 0..d {
        for j in 0..d {
            phases[[i, j]] = uniform_unit(&mut chacha) * 2.0 * PI;
        }
    }

    // Frequency assignment per factor-of-interest. Row `i`: column
    // `i` gets `ω_max`; the other columns receive the `d − 1`
    // complementary frequencies in their natural order. By
    // construction `complementary.len() == d − 1`.
    let mut omegas = Array2::<u32>::zeros((d, d));
    debug_assert_eq!(complementary.len(), d.saturating_sub(1));
    for i in 0..d {
        let mut comp = complementary.iter().copied();
        for j in 0..d {
            omegas[[i, j]] = if j == i {
                omega_max
            } else {
                #[allow(clippy::expect_used)]
                comp.next().expect("complementary length is d − 1")
            };
        }
    }

    // Search curve evaluation. Block i occupies rows
    // [i·N, (i+1)·N).
    let n = n_per_factor;
    let mut samples = Array2::<f64>::zeros((n * d, d));
    let two_pi_over_n = 2.0 * PI / (n as f64);
    for i in 0..d {
        for n_idx in 0..n {
            let s = two_pi_over_n * (n_idx as f64);
            let row = i * n + n_idx;
            for j in 0..d {
                let omega = f64::from(omegas[[i, j]]);
                let phi = phases[[i, j]];
                let arg = omega * s + phi;
                samples[[row, j]] = 0.5 + (1.0 / PI) * (arg.sin()).asin();
            }
        }
    }

    *rng = RngState::snapshot(&chacha, rng);

    Ok(FastDesign {
        samples,
        omegas,
        phases,
        n_per_factor: n,
        d,
        harmonic,
    })
}

/// Saltelli 1999 § 3.2 complementary frequency selection.
/// Returns `d − 1` frequencies, all `≤ ω_max / (2·M)`.
///
/// When `m = floor(ω_max / (2·M)) ≥ d − 1`, frequencies are
/// `linspace(1, m, d-1)` (rounded to integers); pairwise distinct.
///
/// When `m < d − 1`, frequencies cycle `1..=m`; collisions occur.
/// This matches `SALib`'s `fast_sampler.py` behavior. The estimator
/// (PR 9b) tolerates collisions because the FFT bins are still
/// well-separated.
fn complementary_frequencies(d: usize, omega_max: u32, harmonic: u32) -> Vec<u32> {
    if d <= 1 {
        return Vec::new();
    }
    let comp_count = d - 1;
    let m = omega_max / (2 * harmonic);
    debug_assert!(
        m >= 1,
        "complementary_frequencies invariant violated: m = {m} < 1; \
         caller must enforce n_per_factor ≥ 4·harmonic² + 1"
    );
    let mut result = vec![0u32; comp_count];
    if (m as usize) >= comp_count {
        // linspace(1, m, comp_count) rounded to nearest integer.
        if comp_count == 1 {
            result[0] = 1;
        } else {
            #[allow(clippy::cast_precision_loss)]
            let denom = (comp_count - 1) as f64;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            for (k, slot) in result.iter_mut().enumerate() {
                #[allow(clippy::cast_precision_loss)]
                let v = 1.0 + (k as f64) * (f64::from(m) - 1.0) / denom;
                *slot = v.round() as u32;
            }
        }
    } else {
        // Cycle 1..=m. Collisions inherent.
        for (k, slot) in result.iter_mut().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let v = ((k as u32) % m) + 1;
            *slot = v;
        }
    }
    result
}

/// Draw a uniform `[0, 1)` from the chacha RNG via the canonical
/// 53-bit-mantissa construction. Phase shifts deserve full
/// `f64` precision; the LHS sampler uses 32-bit randomness because
/// stratification dominates there.
#[allow(clippy::cast_precision_loss)]
fn uniform_unit(rng: &mut rand_chacha::ChaCha20Rng) -> f64 {
    (rng.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::cast_precision_loss)]
mod tests {
    use super::*;

    const SEED: [u8; 32] = [0; 32];

    fn build(d: usize, n: usize, m: u32) -> FastDesign {
        let mut rng = RngState::from_seed(SEED);
        build_fast_design(d, n, m, &mut rng).expect("valid params")
    }

    // ── Validation ────────────────────────────────────────────────

    #[test]
    fn zero_d_errors() {
        let mut rng = RngState::from_seed(SEED);
        let err = build_fast_design(0, 65, 4, &mut rng).unwrap_err();
        assert_eq!(err, FastError::ZeroD);
    }

    #[test]
    fn zero_harmonic_errors() {
        let mut rng = RngState::from_seed(SEED);
        let err = build_fast_design(4, 65, 0, &mut rng).unwrap_err();
        assert_eq!(err, FastError::ZeroHarmonic);
    }

    #[test]
    fn insufficient_samples_errors() {
        // n < 4·M² + 1 = 65 with M=4.
        let mut rng = RngState::from_seed(SEED);
        let err = build_fast_design(4, 64, 4, &mut rng).unwrap_err();
        assert_eq!(
            err,
            FastError::InsufficientSamples {
                n_per_factor: 64,
                harmonic: 4,
                minimum: 65,
            }
        );
    }

    #[test]
    fn minimum_n_succeeds() {
        // n = 4·M² + 1 = 65 with M=4 ⇒ ω_max = 8, m = 1.
        let design = build(4, 65, 4);
        assert_eq!(design.omegas[[0, 0]], 8);
    }

    // ── Output shape ──────────────────────────────────────────────

    #[test]
    fn output_shape_matches_n_per_factor_times_d() {
        let design = build(6, 129, 4);
        assert_eq!(design.samples.shape(), &[129 * 6, 6]);
        assert_eq!(design.omegas.shape(), &[6, 6]);
        assert_eq!(design.phases.shape(), &[6, 6]);
    }

    #[test]
    fn samples_in_unit_interval() {
        let design = build(6, 129, 4);
        for &v in &design.samples {
            assert!(
                (0.0..=1.0).contains(&v),
                "sample {v} not in [0, 1] — search curve transformation broken"
            );
        }
    }

    // ── Frequency assignment ──────────────────────────────────────

    #[test]
    fn factor_of_interest_holds_max_frequency_in_its_row() {
        let design = build(6, 129, 4);
        for i in 0..6 {
            let omega_i = design.omegas[[i, i]];
            for j in 0..6 {
                if j != i {
                    assert!(
                        design.omegas[[i, j]] < omega_i,
                        "row {i}: ω[{i},{j}] = {} should be < ω_max = {omega_i}",
                        design.omegas[[i, j]]
                    );
                }
            }
        }
    }

    #[test]
    fn complementary_frequencies_below_harmonic_bandwidth() {
        // Saltelli 1999 invariant: every complementary frequency
        // below ω_max / (2·M).
        let design = build(6, 129, 4);
        let omega_max = design.omegas[[0, 0]];
        let bound = omega_max / (2 * design.harmonic);
        for i in 0..6 {
            for j in 0..6 {
                if j != i {
                    assert!(
                        design.omegas[[i, j]] <= bound,
                        "ω[{i},{j}] = {} exceeds bandwidth bound {bound}",
                        design.omegas[[i, j]]
                    );
                }
            }
        }
    }

    #[test]
    fn complementary_distinct_when_m_large() {
        // d=6, n=129, M=4 → ω_max=16, m=2. m=2 < d-1=5 → cycling
        // branch with collisions. So this test covers d=4 where
        // m=2 ≥ d-1=3? Let me try larger n.
        // For ω_max ≥ 2·M·(d-1), m ≥ d-1 ⇒ linspace branch.
        // n=2·M·ω_max+1, so n=2·4·(2·4·5)+1 = 321 gets m=5.
        let design = build(6, 321, 4);
        for i in 0..6 {
            let row: Vec<u32> = (0..6)
                .filter(|&j| j != i)
                .map(|j| design.omegas[[i, j]])
                .collect();
            let mut sorted = row.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(
                sorted.len(),
                row.len(),
                "row {i}: complementary frequencies should be distinct in linspace regime, got {row:?}"
            );
        }
    }

    // ── Phase distribution ────────────────────────────────────────

    #[test]
    fn phases_in_zero_to_two_pi() {
        let design = build(6, 129, 4);
        for &phi in &design.phases {
            assert!((0.0..2.0 * PI).contains(&phi), "phase {phi} not in [0, 2π)");
        }
    }

    // ── Determinism ───────────────────────────────────────────────

    #[test]
    fn same_seed_yields_bit_identical_design() {
        let a = build(6, 129, 4);
        let b = build(6, 129, 4);
        assert_eq!(a.samples, b.samples);
        assert_eq!(a.omegas, b.omegas);
        assert_eq!(a.phases, b.phases);
    }

    #[test]
    fn different_seed_yields_different_phases() {
        let mut rng_a = RngState::from_seed([0; 32]);
        let mut rng_b = RngState::from_seed([1; 32]);
        let a = build_fast_design(6, 129, 4, &mut rng_a).unwrap();
        let b = build_fast_design(6, 129, 4, &mut rng_b).unwrap();
        // Frequencies are deterministic from (d, n, M) — same in
        // both. Phases differ.
        assert_eq!(a.omegas, b.omegas);
        assert_ne!(a.phases, b.phases);
    }

    #[test]
    fn rng_state_advances_on_build() {
        let mut rng = RngState::from_seed(SEED);
        let before = rng.word_pos;
        let _ = build_fast_design(6, 129, 4, &mut rng).unwrap();
        let after = rng.word_pos;
        assert!(
            after > before,
            "RngState word_pos should advance: before={before}, after={after}"
        );
    }

    // ── Search curve transformation ───────────────────────────────

    #[test]
    fn search_curve_at_zero_phase_starts_at_half() {
        // With phase=0 the curve at s=0 is 0.5 + (1/π)·arcsin(sin(0))
        // = 0.5. We don't run with phase=0 in production, but
        // verifying the formula directly is useful sanity.
        let arg: f64 = 0.0;
        let v = 0.5 + (1.0 / PI) * (arg.sin()).asin();
        assert!((v - 0.5).abs() < 1e-15);
    }

    #[test]
    fn search_curve_uniform_marginal_approximately() {
        // With n=1024 samples across the search curve, each column
        // (factor) should be approximately uniform on [0, 1] —
        // arcsin-of-sin is a bijection for any frequency. We use a
        // bin-count test rather than KS for simplicity.
        let design = build(4, 1024, 4);
        for j in 0..4 {
            let mut bins = [0_usize; 10];
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            for row in 0..design.samples.shape()[0] {
                let v = design.samples[[row, j]].clamp(0.0, 0.999_999_9);
                let b = (v * 10.0) as usize;
                bins[b] += 1;
            }
            let total = design.samples.shape()[0] as f64;
            let expected = total / 10.0;
            for (b, &c) in bins.iter().enumerate() {
                let dev = (c as f64 - expected).abs() / expected;
                // Allow 25% deviation per bin — search curves with
                // d=4 blocks of 1024 each have structured (not
                // i.i.d.) marginals; this is a sanity check, not a
                // KS test.
                assert!(
                    dev < 0.25,
                    "factor {j} bin {b}: count {c} deviates {dev:.2} from expected {expected:.0}"
                );
            }
        }
    }
}
