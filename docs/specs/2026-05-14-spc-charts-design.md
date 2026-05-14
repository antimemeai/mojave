# SPC Control Charts — Design Spec (BEAD-0008)

> **Scope:** Math primitives only. Stateful control chart monitors
> with generic `(μ₀, σ)` API. No orchestration, no temporal state
> management, no baseline calibration pipeline. Those come later.

## Goal

New workspace crate `spc-charts` implementing the six control chart
methods needed for longitudinal agent-eval tracking: Shewhart,
CUSUM, FIR CUSUM, EWMA, combined Shewhart-CUSUM, and e-detector
change-point detection. Each chart is a stateful monitor that
accepts observations one at a time and emits signals.

## Architecture

### Crate: `spc-charts`

```
crates/spc-charts/
  Cargo.toml
  src/
    lib.rs              re-exports, crate docs
    types.rs            ChartSignal, ControlLimits, common types
    shewhart.rs         Shewhart individuals / X-bar chart
    cusum.rs            Page 1954 tabular CUSUM
    cusum_fir.rs        FIR CUSUM (Lucas & Crosier 1982)
    ewma.rs             Roberts 1959 EWMA
    combined.rs         Shewhart-CUSUM (Lucas 1982)
    e_detector.rs       Shin-Ramdas-Rinaldo 2023 e-detector
    arl.rs              Average Run Length (Markov chain method)
    g_theory.rs         [feature = "g-theory"] GTheoryResult → ControlLimits
```

### Dependencies

```toml
[dependencies]
thiserror = "2"
serde = { version = "1", features = ["derive"] }

# E-detector uses e-value infrastructure from seq-anytime-valid.
seq-anytime-valid = { path = "../seq-anytime-valid" }

[dependencies.salib-estimators]
path = "../salib-estimators"
optional = true

[features]
default = []
g-theory = ["dep:salib-estimators"]
```

## Types (`types.rs`)

### `ChartSignal`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ChartSignal {
    InControl,
    Warning { statistic: f64 },
    OutOfControl { statistic: f64, observation_index: usize },
}
```

Three-valued: `InControl` means the process is behaving normally.
`Warning` means the statistic is elevated but below the action
limit (Shewhart 2-sigma zone, EWMA warning level). `OutOfControl`
means the action limit was breached — the observation index where
the signal fired is included for downstream attribution.

### `ControlLimits`

```rust
#[derive(Debug, Clone)]
pub struct ControlLimits {
    pub mu_0: f64,
    pub sigma: f64,
}
```

Generic in-control parameters. The caller maps whatever variance
source they have (G-theory, sample std dev, domain knowledge) into
these two numbers. Each chart type defines its own derived limits
(UCL/LCL for Shewhart, decision interval h for CUSUM, etc.) from
these base parameters plus chart-specific tuning constants.

### `SpcError`

```rust
#[derive(Debug, Clone, Error)]
pub enum SpcError {
    #[error("sigma must be positive, got {0}")]
    NonPositiveSigma(f64),
    #[error("parameter {name} must be positive, got {value}")]
    NonPositiveParam { name: &'static str, value: f64 },
    #[error("lambda must be in (0, 1], got {0}")]
    InvalidLambda(f64),
    #[error("ARL matrix is singular at h={0}")]
    SingularArlMatrix(f64),
}
```

## Chart 1: Shewhart (`shewhart.rs`)

### Theory

Shewhart (1931) individuals chart. Plot each observation against
control limits `μ₀ ± k·σ`. Default `k = 3` (3-sigma rule).
Optimal for detecting large shifts (≥ 2σ). Poor at small sustained
shifts — that's what CUSUM/EWMA are for.

### Supplementary rules (Western Electric / Nelson)

Beyond the basic 3-sigma violation, the following zone-based rules
improve sensitivity to non-random patterns:

| Rule | Description |
|------|-------------|
| WE-1 | 1 point beyond 3σ (same as basic Shewhart) |
| WE-2 | 2 of 3 consecutive points beyond 2σ, same side |
| WE-3 | 4 of 5 consecutive points beyond 1σ, same side |
| WE-4 | 8 consecutive points on one side of center |

All four are standard in Montgomery (2012) §5.4. The chart stores
a configurable rule set; the default enables only WE-1.

### API

```rust
pub struct ShewhartConfig {
    pub limits: ControlLimits,
    pub k_sigma: f64,           // default 3.0
    pub rules: Vec<ShewhartRule>,  // default [WE1]
}

pub enum ShewhartRule { WE1, WE2, WE3, WE4 }

pub struct ShewhartChart { /* config + ring buffer for zone rules */ }

impl ShewhartChart {
    pub fn new(config: ShewhartConfig) -> Result<Self, SpcError>;
    pub fn observe(&mut self, x: f64) -> ChartSignal;
    pub fn reset(&mut self);
    pub fn n_observations(&self) -> usize;
}
```

### Formulas

```
UCL = μ₀ + k·σ
LCL = μ₀ − k·σ
Zone A: (μ₀ + 2σ, μ₀ + 3σ) and (μ₀ − 3σ, μ₀ − 2σ)
Zone B: (μ₀ + σ, μ₀ + 2σ)   and (μ₀ − 2σ, μ₀ − σ)
Zone C: (μ₀ − σ, μ₀ + σ)
```

Signal: `OutOfControl` if any enabled rule fires. `Warning` if the
observation is in Zone A but no rule fires.

## Chart 2: CUSUM (`cusum.rs`)

### Theory

Page (1954) tabular CUSUM. Accumulates deviations from target in
both directions. Detects sustained shifts as small as 0.5σ with
properly tuned parameters.

### API

```rust
pub struct CusumConfig {
    pub limits: ControlLimits,
    pub k: f64,    // reference value (allowance), default 0.5 (in σ units)
    pub h: f64,    // decision interval, default 5.0 (in σ units)
}

pub struct CusumChart {
    // Tracks C⁺ (upper) and C⁻ (lower) cumulative sums.
}

impl CusumChart {
    pub fn new(config: CusumConfig) -> Result<Self, SpcError>;
    pub fn observe(&mut self, x: f64) -> ChartSignal;
    pub fn reset(&mut self);
    pub fn c_plus(&self) -> f64;
    pub fn c_minus(&self) -> f64;
    pub fn n_observations(&self) -> usize;
}
```

### Formulas

Two-sided tabular CUSUM (Montgomery 2012 §9.1):

```
z_i = (x_i − μ₀) / σ           standardized observation

C⁺_i = max(0, C⁺_{i−1} + z_i − k)    upper CUSUM
C⁻_i = max(0, C⁻_{i−1} − z_i − k)    lower CUSUM

C⁺_0 = C⁻_0 = 0

Signal: OutOfControl if C⁺_i > h  or  C⁻_i > h
```

Default `(k=0.5, h=5)` gives ARL₀ ≈ 465 (in-control) and detects
a 1σ shift with ARL₁ ≈ 10.4 (Montgomery Table 9.3).

## Chart 3: FIR CUSUM (`cusum_fir.rs`)

### Theory

Lucas & Crosier (1982). Standard CUSUM starts at zero — it takes
time to accumulate evidence for an initial out-of-control state.
FIR (Fast Initial Response) CUSUM initializes at a nonzero head
start, typically `C⁺_0 = C⁻_0 = h/2`. This makes the chart
sensitive to shifts present from the start (e.g., a newly deployed
model that was never in-control).

### API

Same as `CusumChart` but with an additional `head_start` parameter:

```rust
pub struct FirCusumConfig {
    pub limits: ControlLimits,
    pub k: f64,
    pub h: f64,
    pub head_start: f64,   // default h/2
}

pub struct FirCusumChart { /* same internal structure as CusumChart */ }
```

### Formulas

```
C⁺_0 = C⁻_0 = head_start     (typically h/2)

C⁺_i = max(0, C⁺_{i−1} + z_i − k)
C⁻_i = max(0, C⁻_{i−1} − z_i − k)

Signal: same as standard CUSUM
```

After a signal and reset, C⁺ and C⁻ reset to `head_start` (not
zero) — the FIR property persists across restarts.

## Chart 4: EWMA (`ewma.rs`)

### Theory

Roberts (1959). Exponentially weighted moving average. Smooths
observations with weight λ ∈ (0, 1]. Good at detecting small to
moderate sustained shifts. λ = 0.2 is the standard default.

### API

```rust
pub struct EwmaConfig {
    pub limits: ControlLimits,
    pub lambda: f64,     // smoothing constant, default 0.2
    pub l_sigma: f64,    // control limit width in σ, default 3.0
}

pub struct EwmaChart {
    // Tracks EWMA statistic Z_i.
}

impl EwmaChart {
    pub fn new(config: EwmaConfig) -> Result<Self, SpcError>;
    pub fn observe(&mut self, x: f64) -> ChartSignal;
    pub fn reset(&mut self);
    pub fn z(&self) -> f64;
    pub fn n_observations(&self) -> usize;
}
```

### Formulas

EWMA statistic (Montgomery 2012 §9.2):

```
Z_0 = μ₀
Z_i = λ·x_i + (1 − λ)·Z_{i−1}

UCL_i = μ₀ + L·σ·√(λ/(2−λ) · (1 − (1−λ)^(2i)))
LCL_i = μ₀ − L·σ·√(λ/(2−λ) · (1 − (1−λ)^(2i)))

Signal: OutOfControl if Z_i > UCL_i or Z_i < LCL_i
```

Note: control limits are time-varying (widen toward the asymptotic
value). The `(1 − (1−λ)^(2i))` term converges to 1 quickly. The
asymptotic limit width is `L·σ·√(λ/(2−λ))`.

Common tunings (Montgomery Table 9.9):
- `(λ=0.05, L=2.615)` — ARL₀ ≈ 500, best for shifts ≤ 0.5σ
- `(λ=0.10, L=2.814)` — ARL₀ ≈ 500
- `(λ=0.20, L=2.962)` — ARL₀ ≈ 500, general default
- `(λ=0.40, L=3.054)` — ARL₀ ≈ 500, moderate shifts

## Chart 5: Combined Shewhart-CUSUM (`combined.rs`)

### Theory

Lucas (1982). Run CUSUM and Shewhart simultaneously on the same
observations. The CUSUM catches small sustained shifts; the
Shewhart catches isolated large excursions that CUSUM's
accumulation would dilute. Signal if either chart signals.

### API

```rust
pub struct CombinedConfig {
    pub cusum: CusumConfig,
    pub shewhart_k: f64,    // Shewhart limit in σ, default 3.5
    // Note: wider than standard 3σ because CUSUM handles moderate shifts.
    // Lucas 1982 recommends 3.5σ for the combined chart.
}

pub struct CombinedChart {
    // Internal CusumChart + Shewhart check
}

impl CombinedChart {
    pub fn new(config: CombinedConfig) -> Result<Self, SpcError>;
    pub fn observe(&mut self, x: f64) -> ChartSignal;
    pub fn reset(&mut self);
    pub fn cusum_state(&self) -> (&f64, &f64);  // (C⁺, C⁻)
    pub fn n_observations(&self) -> usize;
}
```

### Formulas

```
On each observation x_i:
  1. Compute z_i = (x_i − μ₀) / σ
  2. Update CUSUM: C⁺_i, C⁻_i (standard tabular)
  3. Check Shewhart: |z_i| > shewhart_k?
  4. Signal if CUSUM signals OR Shewhart signals.
     Source field indicates which triggered.
```

## Chart 6: E-Detector (`e_detector.rs`)

### Theory

Shin, Ramdas, Rinaldo (2023) "E-detectors: A Nonparametric
Framework for Sequential Change Detection." The anytime-valid
generalization of CUSUM. Instead of accumulating standardized
deviations, accumulates e-values — test martingales under the
null. Change is declared when the running maximum of the e-process
exceeds threshold `1/α`.

Key advantages over classical CUSUM:
- Anytime-valid Type-I guarantee (not just ARL-based).
- Nonparametric: works with any e-value, not just Gaussian.
- Naturally composes with the e-value infrastructure in
  `seq-anytime-valid`.

### API

```rust
pub struct EDetectorConfig {
    pub alpha: f64,              // significance level, default 0.05
    pub window: EDetectorWindow, // growing or fixed-width
}

pub enum EDetectorWindow {
    /// Growing window: M_t = max(1, M_{t-1}) · e_t
    Growing,
    /// Fixed-width window of size w: restart after w observations.
    Fixed { width: usize },
}

pub struct EDetector<E: EValueSource> {
    // Accumulates e-process, tracks running maximum.
}

/// Trait for pluggable e-value computation.
pub trait EValueSource {
    fn e_value(&self, observation: f64) -> f64;
}

/// Gaussian location e-value: tests H0: μ = μ₀ vs H1: μ ≠ μ₀.
pub struct GaussianEValue {
    pub mu_0: f64,
    pub sigma: f64,
    pub mixing_variance: f64,   // τ² for the MSPRT mixture
}

impl EDetector<E> {
    pub fn new(config: EDetectorConfig, source: E) -> Result<Self, SpcError>;
    pub fn observe(&mut self, x: f64) -> ChartSignal;
    pub fn reset(&mut self);
    pub fn e_process(&self) -> f64;
    pub fn n_observations(&self) -> usize;
}
```

### Formulas

E-process (Shin et al. 2023 §2):

```
M_0 = 1
M_t = max(1, M_{t−1}) · e_t       (growing window)

where e_t is the per-observation e-value from the source.

Signal: OutOfControl if M_t ≥ 1/α

For the fixed-width variant:
  M_t = ∏_{i=t−w+1}^{t} e_i        (product over last w observations)
  Signal: OutOfControl if M_t ≥ 1/α
```

The `GaussianEValue` source computes:
```
e_t = exp(gaussian_msprt_log_lr(x_t, μ₀, σ, τ²))
```
using `seq_anytime_valid::msprt::gaussian_msprt_log_lr`.

## ARL Computation (`arl.rs`)

### Theory

Average Run Length is the expected number of observations before
a signal fires. ARL₀ is the in-control ARL (want: large). ARL₁
is the out-of-control ARL at a given shift δ (want: small).

For CUSUM and EWMA, exact ARL can be computed via Markov chain
discretization of the chart statistic's state space (Brook &
Evans 1972; Lucas & Saccucci 1990).

### API

```rust
/// Compute ARL₀ and ARL₁ for a CUSUM chart.
pub fn cusum_arl(k: f64, h: f64, shift: f64, n_states: usize)
    -> Result<f64, SpcError>;

/// Compute ARL₀ and ARL₁ for an EWMA chart.
pub fn ewma_arl(lambda: f64, l_sigma: f64, shift: f64, n_states: usize)
    -> Result<f64, SpcError>;
```

`n_states` controls the Markov chain discretization resolution
(default 200). Higher = more accurate but O(n³) for the matrix
solve.

### Method

Discretize the chart statistic's range `[0, h]` (CUSUM) or
`[LCL, UCL]` (EWMA) into `n_states` intervals. Build transition
probability matrix `P` where `P[i][j]` is the probability of
transitioning from state `i` to state `j` under the specified
shift. The ARL vector satisfies `(I − P) · ARL = 1`, solved via
LU decomposition.

## G-Theory Convenience (`g_theory.rs`, feature-gated)

```rust
#[cfg(feature = "g-theory")]
pub fn control_limits_from_g_theory(
    result: &salib_estimators::GTheoryResult,
    grand_mean: f64,
    n_items: usize,
    n_raters: usize,
) -> ControlLimits {
    // G-theory "universe score" standard error:
    //   σ² = σ²_p + σ²_pi/n_i + σ²_pr/n_r + σ²_pir/(n_i·n_r)
    // where n_i and n_r are the number of items and raters in the
    // original study design (extractable from the GTheoryResult's
    // input dimensions). σ²_p is the person variance (signal);
    // the remaining terms are error facets (noise floor).
    let sigma_sq = result.sigma_pi / n_items as f64
        + result.sigma_pr / n_raters as f64
        + result.sigma_pir / (n_items * n_raters) as f64;
    ControlLimits {
        mu_0: grand_mean,
        sigma: sigma_sq.sqrt(),
    }
}
```

This maps G-theory's variance decomposition to the `(μ₀, σ)` pair
that all charts accept. The caller provides the grand mean from
baseline runs; the G-theory result provides the noise-floor σ.

## Error Handling

All constructors validate parameters and return `Result<Self, SpcError>`.

`.observe()` returns `ChartSignal` directly (not `Result`) for
ergonomic streaming use. Finite inputs are a documented
precondition: `debug_assert!(x.is_finite())` in each chart's
`.observe()` catches violations in development; release builds
treat non-finite as a caller bug (same convention as
`salib_core::tree_sum`).

## Validation (4-Gate)

### Gate 1: Textbook Reproductions

| Test | Source | Expectation |
|------|--------|-------------|
| CUSUM ARL₀ at (k=0.5, h=5) | Montgomery Table 9.3 | ARL₀ ≈ 465 |
| CUSUM ARL₁ at δ=1σ | Montgomery Table 9.3 | ARL₁ ≈ 10.4 |
| EWMA ARL₀ at (λ=0.2, L=2.962) | Montgomery Table 9.9 | ARL₀ ≈ 500 |
| Shewhart ARL₀ at k=3 | Theory: 1/(2·Φ(−3)) | ARL₀ ≈ 370.4 |
| CUSUM ARL₁ at δ=0.5σ | Montgomery Table 9.3 | ARL₁ ≈ 38.0 |

### Gate 2: Reference Implementation Cross-Checks

- **R `qcc`** (Scrucca 2004, pin v2.7): Shewhart limits, CUSUM/EWMA statistics on shared datasets.
- **R `spc`** (Knoth 2023): exact ARL for CUSUM (`xcusum.arl`) and EWMA (`xewma.arl`).
- **Python `pyspc`** or `spccharter`: cross-check chart statistics on shared datasets.

Tolerance: `rtol = 1e-3` for ARL, `rtol = 1e-6` for chart statistics.

### Gate 3: Property-Based Tests

- CUSUM/EWMA ARL₀ is monotonically decreasing in shift magnitude.
- Shewhart with WE-1 only: ARL₀ = 1 / P(|Z| > k).
- CUSUM C⁺, C⁻ are always ≥ 0 (non-negative by construction).
- EWMA Z_i is always between min(observations) and max(observations) (weighted average property).
- All charts: `reset()` followed by in-control observations should not signal.
- E-detector: M_t ≥ 1 always (max(1, ·) floor).

### Gate 4: Monte Carlo Calibration

- Simulate 10,000 in-control sequences of length 1000. Empirical ARL₀ should match theoretical within 5%.
- Simulate 10,000 sequences with a 1σ shift at t=50. Empirical detection delay should match ARL₁ within 10%.
- E-detector: empirical false-alarm rate at threshold 1/α should be ≤ α (anytime-valid guarantee).

## Non-Goals (Deferred)

- Multivariate charts (T², MEWMA) — future bead if needed.
- Baseline calibration pipeline (Phase I analysis) — orchestration layer.
- Change attribution (which code change caused the shift) — orchestration.
- Temporal state persistence — orchestration.
- Visualization / plotting — Python/web layer.
- Subgroup charts (X-bar/R, X-bar/S) — add when needed; individuals charts cover the agent-eval use case.
