# Design Spec: `seq-anytime-valid` Crate

**BEAD:** BEAD-0004
**Date:** 2026-05-12
**Status:** Draft
**Critical path to:** BEAD-0008 (SPC control charts)

## 1. Purpose

Sequential hypothesis testing and anytime-valid inference primitives for
deciding "is there enough evidence to stop evaluating?" Saves inference
dollars by stopping early when evidence is sufficient, or continuing when
it is not.

Not in scope: bandit algorithms, sequential experimental design, adaptive
sampling strategies, multiple testing correction, visualization.

## 2. Methods

Eight method families spanning three generations of sequential testing.

### 2.1 Classical

| Method | Source | Output |
|--------|--------|--------|
| Wald SPRT (approximate) | Wald 1945 | Accept / Reject / Continue |
| Wald SPRT (conservative) | Wald 1945 | Accept / Reject / Continue |
| Boosted SPRT | Fischer & Ramdas 2024 | Accept / Reject / Continue (tighter) |

**Wald approximate boundaries:** A = (1-beta)/alpha, B = beta/(1-alpha).
These do not guarantee error control due to overshoot (Fischer 2024). The
log-likelihood ratio may cross A or B between observations, making the
actual error rates loose.

**Wald conservative boundaries:** A = 1/alpha, B = beta. Guarantees Type I
error control at alpha via Ville's inequality. Type II controlled at beta.
Conservative in sample size.

**Fischer boosted SPRT:** Truncation function T_alpha(x; M) = x if Mx <= 1/alpha,
else 1/(M*alpha). Applied to likelihood ratio factors to avoid overshooting
1/alpha. Boost factors b_t >= 1 computed greedily to tighten the test
supermartingale while preserving validity. Strictly dominates approximate
SPRT in both error control and sample efficiency.

### 2.2 Group-Sequential

| Method | Source | Output |
|--------|--------|--------|
| Pocock boundaries | Pocock 1977 | Equal critical values at K looks |
| O'Brien-Fleming boundaries | O'Brien & Fleming 1979 | Increasing critical values |
| Lan-DeMets alpha-spending | Lan & DeMets 1983 | Flexible-timing boundaries |

**Pocock:** Equal boundaries c_k = c for all k = 1..K. Conservative at early
looks, aggressive at final look. Boundary c determined numerically by
bisection on the K-dimensional equi-coordinate multivariate normal
probability (using Genz's algorithm or direct quadrature for small K).

**O'Brien-Fleming:** Boundaries c_k = C * sqrt(K/k). Very conservative early,
nearly nominal at final look. Preferred when early stopping should require
overwhelming evidence.

**Lan-DeMets alpha-spending:** Generalization that decouples boundary shape
from pre-planned look schedule. Spending function alpha*(t) maps information
fraction t in [0,1] to cumulative alpha spent. Built-in spending functions:
- Pocock-type: alpha*(t) = alpha * ln(1 + (e-1)*t)
- OBF-type: alpha*(t) = 2 - 2*Phi(z_{alpha/2} / sqrt(t))
- Custom: user provides Fn(f64) -> f64

### 2.3 Anytime-Valid

| Method | Source | Output |
|--------|--------|--------|
| Mixture SPRT (mSPRT) | Johari et al. 2022 | Always-valid p-values |
| Confidence sequences | Howard et al. 2021 | Time-uniform confidence intervals |
| E-values / safe testing | Gruenwald et al. 2024 | E-values with optional continuation |

**Mixture SPRT (Johari 2022):** Tests H0: theta = theta_0 vs H1: theta != theta_0
by marginalizing the likelihood ratio over a mixing distribution pi on theta.
For Gaussian data with mixing distribution N(theta_0, tau^2):

  Lambda_n = integral over theta of product_{i=1}^{n} [f_theta(x_i)/f_{theta_0}(x_i)] * pi(theta) d_theta

Always-valid p-value: p_n = 1/Lambda_n. Guarantees P_{theta_0}(exists n: p_n <= alpha) <= alpha.

**Conditions for validity:**
- Simple null (point H0)
- Observations from single-parameter exponential family
- Independence (or conditional independence given filtration)
- Mixing distribution choice affects efficiency, not validity

**Limitation:** Tests of power 1 only -- never accepts H0. Can only reject or
continue. Accepting H0 requires a maximum sample size M with default
acceptance at M.

**Confidence sequences (Howard 2021):** Time-uniform confidence intervals
(C_t)_{t>=1} satisfying P(forall t >= 1: mu in C_t) >= 1 - alpha. Valid at
any data-dependent stopping time without correction for peeking. Two
constructions:
- Normal mixture CS: for sub-Gaussian observations, uses Robbins' Gaussian
  mixture boundary
- Sub-exponential CS: for bounded or light-tailed observations

Only requires finite variance (sub-Gaussian/sub-exponential bound), not a
parametric family. More robust than mSPRT for heavy-tailed score outcomes.

**E-values (Gruenwald 2024):** Nonnegative random variables E with E_P[E] <= 1
for all P in H0. Reject at level alpha when E >= 1/alpha. Key property:
optional continuation -- multiplying independent e-values preserves validity
regardless of whether the decision to continue was data-dependent.

Three interpretations: (1) gambling (Kelly criterion), (2) conservative
p-value via p = 1/E, (3) Bayes factor with special prior.

GRO (growth-rate optimal) e-values maximize log-growth under the
alternative. For simple H0, GRO e-value = Bayes factor with right Haar
prior on H0.

### 2.4 Estimation

| Method | Source | Output |
|--------|--------|--------|
| Bias-adjusted MLE | Siegmund 1985 Ch.4 | Corrected point estimate |
| Practical significance | Shim 2025 | Truncated mSPRT for abs(theta) >= delta |

**Bias-adjusted estimation (Siegmund 1985):** The MLE at stopping time tau
overestimates the true effect size because stopping is correlated with
extreme observations. Siegmund's correction:

  theta_hat_corrected = theta_hat_MLE - bias(tau, theta_hat_MLE)

where the bias term depends on the stopping rule geometry. Two approaches:
- Conditional MLE adjustment (Siegmund Ch.4)
- Median-unbiased estimator (find theta s.t. P_theta(tau <= observed) = 0.5)

**Practical significance (Shim 2025):** Truncated mSPRT that tests for
practically significant effects: H0': theta in (-delta, delta) vs
H1': abs(theta) >= delta. Prevents declaring "significant" when the effect
is real but trivially small. Uses truncated Gaussian mixing distribution
that places no mass in the indifference zone.

## 3. Architecture

Layered by abstraction level. Three layers: boundary math (pure functions),
evidence accumulation (stateless computation), monitors (stateful wrappers).

```
crates/seq-anytime-valid/
  Cargo.toml
  src/
    lib.rs
    types.rs              # Decision, configs, EvidenceSnapshot
    error.rs              # SeqError enum

    boundary/
      mod.rs
      wald.rs             # approximate + conservative boundaries
      boosted.rs          # Fischer 2024 truncation + boost factors
      pocock.rs           # Pocock equal boundaries (numeric solve)
      obf.rs              # O'Brien-Fleming boundaries
      spending.rs         # Lan-DeMets alpha-spending framework

    evidence/
      mod.rs
      likelihood.rs       # log-LR for Bernoulli + Normal
      e_value.rs          # e-variables, product e-values, GRO
      confseq.rs          # confidence sequences (normal mixture, sub-Gaussian)
      msprt.rs            # mixture SPRT, always-valid p-values

    monitor/
      mod.rs
      sprt.rs             # SprtMonitor: feed obs -> Decision
      group_seq.rs        # GroupSeqMonitor: feed batch at look k -> Decision
      anytime.rs          # AnytimeMonitor: feed obs -> p-value + CS

    bias.rs               # Siegmund bias correction
    practical.rs          # Shim truncated mSPRT
```

### 3.1 Key Types

```rust
/// Three-valued stopping decision.
pub enum Decision {
    /// Sufficient evidence to reject H0 (effect detected / regression).
    Reject,
    /// Sufficient evidence to accept H0 (no effect) or max sample reached.
    Accept,
    /// Insufficient evidence; continue sampling.
    Continue,
}

/// Snapshot of accumulated evidence at a point in time.
pub struct EvidenceSnapshot {
    pub log_likelihood_ratio: f64,
    pub n_observations: usize,
    pub always_valid_p: Option<f64>,
    pub confidence_interval: Option<(f64, f64)>,
    pub e_value: Option<f64>,
}
```

### 3.2 Stateless API

Pure functions for batch analysis. Consumer provides all data; function
returns result.

```rust
pub fn sprt_decide(config: &SprtConfig, observations: &[f64]) -> Result<Decision, SeqError>;
pub fn group_seq_boundary(config: &GroupSeqConfig, k: usize) -> Result<f64, SeqError>;
pub fn always_valid_p(config: &MsprtConfig, observations: &[f64]) -> Result<f64, SeqError>;
pub fn confidence_sequence(config: &ConfSeqConfig, observations: &[f64]) -> Result<(f64, f64), SeqError>;
pub fn e_value(config: &EValueConfig, observations: &[f64]) -> Result<f64, SeqError>;
pub fn bias_corrected_estimate(config: &BiasConfig, observations: &[f64], stopping_time: usize) -> Result<f64, SeqError>;
```

### 3.3 Stateful API

Monitors maintain internal state and accept one observation (or one batch)
at a time. Consumer drives the loop.

```rust
let mut monitor = SprtMonitor::new(config);
for obs in stream {
    match monitor.update(obs)? {
        Decision::Reject => break,
        Decision::Accept => break,
        Decision::Continue => {}
    }
}
let snapshot = monitor.snapshot();
```

Group-sequential monitor accepts batches at pre-planned looks:

```rust
let mut monitor = GroupSeqMonitor::new(config);
for (k, batch) in batches.enumerate() {
    match monitor.update_batch(k, &batch)? {
        Decision::Reject => break,
        Decision::Accept if k == config.total_looks - 1 => break,
        _ => {}
    }
}
```

Anytime monitor returns rich evidence at each step:

```rust
let mut monitor = AnytimeMonitor::new(config);
for obs in stream {
    let snapshot = monitor.update(obs)?;
    if snapshot.always_valid_p.unwrap() <= alpha {
        // reject
        break;
    }
}
```

### 3.4 Dependencies

**Runtime:** `thiserror`, `serde` (derive, for config serialization). No `rand`
-- this crate consumes data, does not generate it.

**Dev:** `proptest`, `approx`, `metric-tck-harness` (Gherkin TCK), `rand` +
`rand_distr` (for Gate 4 Monte Carlo only).

**No dependency on `eval-core`.** This is a pure math-primitive crate. The SPC
layer bridges between `eval-core::Outcome` and `seq-anytime-valid` inputs.

## 4. Error Handling

```rust
pub enum SeqError {
    /// H0 and H1 specify the same parameter value.
    DegenerateHypotheses,
    /// Observation is NaN or infinite.
    NonFiniteInput(f64),
    /// Alpha not in (0, 1).
    InvalidAlpha(f64),
    /// Beta not in (0, 1).
    InvalidBeta(f64),
    /// Number of looks K must be >= 1.
    InvalidLooks(usize),
    /// Alpha + beta >= 1 (no valid test exists).
    AlphaBetaSum,
    /// No observations provided.
    EmptyObservations,
    /// Look index k out of range [1, K].
    LookOutOfRange { k: usize, total: usize },
    /// Mixing distribution variance must be positive.
    InvalidMixingVariance(f64),
    /// Practical significance delta must be positive.
    InvalidPracticalDelta(f64),
}
```

All public functions reject invalid inputs at the boundary. No silent NaN
propagation; no panics in library code.

## 5. Validation (4-Gate)

### Gate 1: Textbook Reproductions

| Test | Source | Values |
|------|--------|--------|
| SPRT binomial boundaries | Wald 1945 | p0=0.1, p1=0.2, alpha=0.05, beta=0.10 -> A=9.0, B=0.111 |
| SPRT normal log-LR | Wald 1945 s5.4 | mu0=0, mu1=0.5, sigma=1 -> formula verification |
| Pocock boundaries K=2..5 | Pocock 1977 Table 1 | alpha=0.05 two-sided, known exact values |
| OBF boundaries K=2..5 | O'Brien & Fleming 1979 | alpha=0.05 two-sided, known exact values |
| Bias correction | Siegmund 1985 Ch.4 | Normal mean SPRT at known stopping times |
| Fischer boost | Fischer 2024 Table 1 | Comparison of error guarantees across SPRT variants |

Tolerances: rtol=1e-6, atol=1e-8 for closed-form; rtol=1e-4 for numerically
solved boundaries (Pocock).

### Gate 2: Reference Implementation Cross-Checks (R)

| Our function | R reference | Package (pinned version) |
|--------------|-------------|--------------------------|
| pocock_boundary(K, alpha) | gsDesign::gsDesign(k=K, sfu="Pocock") | gsDesign |
| obf_boundary(K, alpha) | gsDesign::gsDesign(k=K, sfu="OF") | gsDesign |
| spending_boundary(K, alpha, sf) | gsDesign::gsDesign(k=K, sfu=sf) | gsDesign |
| confidence_sequence(obs, alpha) | confseq::cs_mean(obs, alpha) | confseq |

R scripts live in `scripts/r-fixtures/seq-anytime-valid/`. Output JSON
fixtures checked into `tck/seq-anytime-valid/fixtures/`. CI runs against
fixtures; R regeneration is manual/on-demand.

### Gate 3: Property-Based Tests

| Property | Invariant |
|----------|-----------|
| K=1 degenerate | Group-sequential at K=1 = fixed-sample z_{alpha/2} boundary |
| Pocock=OBF at K=1 | Both degenerate to same single boundary |
| H0=H1 | SPRT with identical hypotheses -> SeqError::DegenerateHypotheses |
| Spending exhaustion | Cumulative alpha-spending at t=1.0 = nominal alpha (to 1e-10) |
| Information scaling | Doubling sample sizes preserves boundary in information-time |
| Supermartingale | E-value product under H0: E[E_{n+1} given E_n] <= E_n |
| Observation reorder | SPRT decision invariant to permutation of i.i.d. observations |
| Monotone boundaries | OBF boundaries are non-increasing in look index |
| Spending monotone | alpha*(t) is non-decreasing, alpha*(0) = 0, alpha*(1) = alpha |

### Gate 4: Monte Carlo Calibration

| Calibration | Target | Tolerance | Reps |
|-------------|--------|-----------|------|
| SPRT Type-I under H0 | alpha | MC error (sqrt(alpha*(1-alpha)/N)) | 100,000 |
| SPRT ASN under H0 | Wald's E[N] formula | rtol=0.05 | 100,000 |
| Boosted vs approximate | boosted N <= approximate N | p < 0.01 one-sided | 100,000 |
| Group-seq power | gsDesign reference | rtol=0.02 | 50,000 |
| Always-valid p Type-I | P(exists n: p_n <= alpha) <= alpha | MC error | 100,000 |
| CS coverage | 95% CS covers mu at all n | [0.93, 0.97] | 10,000 |

## 6. TCK Feature Files

```
tck/seq-anytime-valid/features/
  sprt.feature                # Wald SPRT: binary + normal, boundaries, decisions
  boosted_sprt.feature        # Fischer boosted: same scenarios, tighter bounds
  group_sequential.feature    # Pocock, OBF, Lan-DeMets boundaries + decisions
  msprt.feature               # Always-valid p-values, mixing distributions
  confseq.feature             # Confidence sequence width + coverage
  e_value.feature             # E-value accumulation, supermartingale property
  bias.feature                # Siegmund bias correction at stopping time
  practical.feature           # Truncated mSPRT for practical significance
```

TCK harness uses `metric-tck-harness` (homegrown Gherkin parser + SyncRunner),
same pattern as the scaffold and IRR harnesses.

## 7. Literature

| Citation | Role |
|----------|------|
| Wald 1945 | SPRT definition, boundaries, ASN formulas |
| Fischer & Ramdas 2024 | Boosted SPRT, overshoot correction |
| Pocock 1977 | Equal group-sequential boundaries |
| O'Brien & Fleming 1979 | Conservative early / nominal late boundaries |
| Lan & DeMets 1983 | Alpha-spending framework |
| Siegmund 1985 | Bias-adjusted estimation at stopping time |
| Johari et al. 2022 | Mixture SPRT, always-valid p-values |
| Howard et al. 2021 | Confidence sequences, asymptotic CS |
| Gruenwald et al. 2024 | E-values, safe testing, GRO optimality |
| Shim 2025 | Truncated mSPRT for practical significance |
| Koning & van Meer 2026 | Anytime validity via induced sequential tests |

Papers in `../evals_papers/`. Jennison & Turnbull 2000 needed for
group-sequential reference material and bias correction detail.

## 8. Exclusions

- No `eval-core` dependency (pure math crate)
- No async runtime (monitors are synchronous)
- No visualization (orchestration layer concern)
- No multiple testing correction (separate concern for multi-task eval)
- No truncated/curtailed sequential tests (Wald's truncated SPRT)
- No bandit algorithms or adaptive experimental design
