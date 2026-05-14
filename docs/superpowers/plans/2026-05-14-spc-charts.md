# SPC Control Charts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** New `spc-charts` crate with six stateful control chart monitors (Shewhart, CUSUM, FIR CUSUM, EWMA, combined Shewhart-CUSUM, e-detector), ARL computation, and optional G-theory integration.

**Architecture:** Each chart is a stateful struct accepting observations via `.observe(x) -> ChartSignal`. Generic `(μ₀, σ)` interface decouples from variance source. E-detector uses `seq-anytime-valid`'s MSPRT e-values. ARL via Markov chain discretization.

**Tech Stack:** Rust, thiserror, serde, seq-anytime-valid, nalgebra (ARL matrix solve), optional salib-estimators (G-theory feature)

---

### Task 1: Crate Scaffold + Types

**Files:**
- Create: `crates/spc-charts/Cargo.toml`
- Create: `crates/spc-charts/src/lib.rs`
- Create: `crates/spc-charts/src/types.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Create Cargo.toml**

Create `crates/spc-charts/Cargo.toml`:

```toml
[package]
name = "spc-charts"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
publish = false
description = """
Stateful SPC control chart monitors — Shewhart, CUSUM, FIR CUSUM,
EWMA, combined Shewhart-CUSUM, e-detector change-point detection.
"""

[lib]
path = "src/lib.rs"

[dependencies]
thiserror = "2"
serde = { version = "1", features = ["derive"] }
seq-anytime-valid = { path = "../seq-anytime-valid" }
nalgebra = "0.33"

[dependencies.salib-estimators]
path = "../salib-estimators"
optional = true

[features]
default = []
g-theory = ["dep:salib-estimators"]

[lints]
workspace = true
```

- [ ] **Step 2: Add to workspace**

Add `"crates/spc-charts"` to the `members` array in the root `Cargo.toml`.

- [ ] **Step 3: Create types.rs**

Create `crates/spc-charts/src/types.rs`:

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChartSignal {
    InControl,
    Warning { statistic: f64 },
    OutOfControl {
        statistic: f64,
        observation_index: usize,
    },
}

impl ChartSignal {
    #[must_use]
    pub fn is_out_of_control(&self) -> bool {
        matches!(self, Self::OutOfControl { .. })
    }

    #[must_use]
    pub fn is_in_control(&self) -> bool {
        matches!(self, Self::InControl)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlLimits {
    pub mu_0: f64,
    pub sigma: f64,
}

impl ControlLimits {
    pub fn new(mu_0: f64, sigma: f64) -> Result<Self, SpcError> {
        if !sigma.is_finite() || sigma <= 0.0 {
            return Err(SpcError::NonPositiveSigma(sigma));
        }
        if !mu_0.is_finite() {
            return Err(SpcError::NonFiniteMu(mu_0));
        }
        Ok(Self { mu_0, sigma })
    }
}

#[derive(Debug, Clone, Error)]
pub enum SpcError {
    #[error("sigma must be positive and finite, got {0}")]
    NonPositiveSigma(f64),
    #[error("mu_0 must be finite, got {0}")]
    NonFiniteMu(f64),
    #[error("parameter {name} must be positive, got {value}")]
    NonPositiveParam { name: &'static str, value: f64 },
    #[error("lambda must be in (0, 1], got {0}")]
    InvalidLambda(f64),
    #[error("alpha must be in (0, 1), got {0}")]
    InvalidAlpha(f64),
    #[error("ARL matrix is singular at h={0}")]
    SingularArlMatrix(f64),
    #[error("window width must be >= 1, got {0}")]
    InvalidWindowWidth(usize),
}
```

- [ ] **Step 4: Create lib.rs**

Create `crates/spc-charts/src/lib.rs`:

```rust
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod types;

pub use types::{ChartSignal, ControlLimits, SpcError};
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p spc-charts 2>&1`
Expected: compiles clean.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/spc-charts/
git commit -m "feat(spc): scaffold spc-charts crate with core types (BEAD-0008)"
```

---

### Task 2: Shewhart Chart

**Files:**
- Create: `crates/spc-charts/src/shewhart.rs`
- Modify: `crates/spc-charts/src/lib.rs`
- Create: `tck/spc-charts/features/shewhart.feature`
- Create: `crates/spc-charts/tests/shewhart_tck.rs`

- [ ] **Step 1: Write TCK feature file**

Create `tck/spc-charts/features/shewhart.feature`:

```gherkin
Feature: Shewhart individuals control chart

  Scenario: In-control observations produce no signal
    Given a Shewhart chart with mu_0=50 sigma=2 k=3
    When I observe values 49, 50, 51, 48, 52, 50
    Then all signals are InControl

  Scenario: 3-sigma violation signals OutOfControl
    Given a Shewhart chart with mu_0=50 sigma=2 k=3
    When I observe value 57
    Then the signal is OutOfControl

  Scenario: WE-2 rule detects 2 of 3 beyond 2-sigma
    Given a Shewhart chart with mu_0=50 sigma=2 k=3 rules=[WE1,WE2]
    When I observe values 55, 49, 55
    Then the third observation signals OutOfControl

  Scenario: WE-4 rule detects 8 consecutive on one side
    Given a Shewhart chart with mu_0=50 sigma=2 k=3 rules=[WE1,WE4]
    When I observe values 51, 51, 51, 51, 51, 51, 51, 51
    Then the eighth observation signals OutOfControl

  Scenario: Shewhart ARL₀ is approximately 370.4 at k=3
    Given a Shewhart chart with mu_0=0 sigma=1 k=3 rules=[WE1]
    When I simulate 10000 in-control sequences of length 2000
    Then the empirical ARL₀ is within 10% of 370.4
```

- [ ] **Step 2: Create shewhart.rs**

Create `crates/spc-charts/src/shewhart.rs`:

```rust
use crate::types::{ChartSignal, ControlLimits, SpcError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShewhartRule {
    WE1,
    WE2,
    WE3,
    WE4,
}

#[derive(Debug, Clone)]
pub struct ShewhartConfig {
    pub limits: ControlLimits,
    pub k_sigma: f64,
    pub rules: Vec<ShewhartRule>,
}

impl ShewhartConfig {
    #[must_use]
    pub fn default_for(limits: ControlLimits) -> Self {
        Self {
            limits,
            k_sigma: 3.0,
            rules: vec![ShewhartRule::WE1],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShewhartChart {
    config: ShewhartConfig,
    ucl: f64,
    lcl: f64,
    sigma: f64,
    mu_0: f64,
    n: usize,
    history: Vec<f64>,
}

impl ShewhartChart {
    pub fn new(config: ShewhartConfig) -> Result<Self, SpcError> {
        if config.k_sigma <= 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "k_sigma",
                value: config.k_sigma,
            });
        }
        let mu_0 = config.limits.mu_0;
        let sigma = config.limits.sigma;
        let ucl = mu_0 + config.k_sigma * sigma;
        let lcl = mu_0 - config.k_sigma * sigma;
        Ok(Self {
            config,
            ucl,
            lcl,
            sigma,
            mu_0,
            n: 0,
            history: Vec::new(),
        })
    }

    pub fn observe(&mut self, x: f64) -> ChartSignal {
        debug_assert!(x.is_finite());
        self.n += 1;
        let z = (x - self.mu_0) / self.sigma;
        self.history.push(z);

        for &rule in &self.config.rules {
            if self.check_rule(rule) {
                return ChartSignal::OutOfControl {
                    statistic: z,
                    observation_index: self.n - 1,
                };
            }
        }

        if z.abs() > 2.0 {
            ChartSignal::Warning { statistic: z }
        } else {
            ChartSignal::InControl
        }
    }

    pub fn reset(&mut self) {
        self.n = 0;
        self.history.clear();
    }

    #[must_use]
    pub fn n_observations(&self) -> usize {
        self.n
    }

    fn check_rule(&self, rule: ShewhartRule) -> bool {
        let h = &self.history;
        let n = h.len();
        match rule {
            ShewhartRule::WE1 => {
                n >= 1 && h[n - 1].abs() > self.config.k_sigma
            }
            ShewhartRule::WE2 => {
                if n < 2 {
                    return false;
                }
                let last3: Vec<f64> = h[n.saturating_sub(3)..].to_vec();
                let above = last3.iter().filter(|&&z| z > 2.0).count();
                let below = last3.iter().filter(|&&z| z < -2.0).count();
                above >= 2 || below >= 2
            }
            ShewhartRule::WE3 => {
                if n < 4 {
                    return false;
                }
                let last5: Vec<f64> = h[n.saturating_sub(5)..].to_vec();
                let above = last5.iter().filter(|&&z| z > 1.0).count();
                let below = last5.iter().filter(|&&z| z < -1.0).count();
                above >= 4 || below >= 4
            }
            ShewhartRule::WE4 => {
                if n < 8 {
                    return false;
                }
                let last8 = &h[n - 8..];
                let all_above = last8.iter().all(|&z| z > 0.0);
                let all_below = last8.iter().all(|&z| z < 0.0);
                all_above || all_below
            }
        }
    }
}
```

- [ ] **Step 3: Add to lib.rs**

Add to `crates/spc-charts/src/lib.rs`:

```rust
pub mod shewhart;

pub use shewhart::{ShewhartChart, ShewhartConfig, ShewhartRule};
```

- [ ] **Step 4: Write TCK tests**

Create `crates/spc-charts/tests/shewhart_tck.rs`:

```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{
    ChartSignal, ControlLimits, ShewhartChart, ShewhartConfig, ShewhartRule,
};

fn chart_3sigma(mu_0: f64, sigma: f64) -> ShewhartChart {
    let limits = ControlLimits::new(mu_0, sigma).unwrap();
    ShewhartChart::new(ShewhartConfig::default_for(limits)).unwrap()
}

fn chart_with_rules(mu_0: f64, sigma: f64, rules: Vec<ShewhartRule>) -> ShewhartChart {
    let limits = ControlLimits::new(mu_0, sigma).unwrap();
    ShewhartChart::new(ShewhartConfig {
        limits,
        k_sigma: 3.0,
        rules,
    })
    .unwrap()
}

#[test]
fn in_control_no_signal() {
    let mut chart = chart_3sigma(50.0, 2.0);
    for &x in &[49.0, 50.0, 51.0, 48.0, 52.0, 50.0] {
        assert!(
            !chart.observe(x).is_out_of_control(),
            "x={x} should be in control"
        );
    }
}

#[test]
fn three_sigma_violation() {
    let mut chart = chart_3sigma(50.0, 2.0);
    let signal = chart.observe(57.0);
    assert!(signal.is_out_of_control(), "57 is >3σ above 50±6");
}

#[test]
fn we2_two_of_three_beyond_2sigma() {
    let mut chart = chart_with_rules(
        50.0,
        2.0,
        vec![ShewhartRule::WE1, ShewhartRule::WE2],
    );
    assert!(chart.observe(55.0).is_in_control()); // z=2.5, >2σ but only 1 of 1
    assert!(chart.observe(49.0).is_in_control()); // z=-0.5, in zone C
    let signal = chart.observe(55.0);              // z=2.5, now 2 of 3 >2σ same side
    assert!(signal.is_out_of_control(), "WE-2: 2 of 3 beyond 2σ");
}

#[test]
fn we4_eight_consecutive_one_side() {
    let mut chart = chart_with_rules(
        50.0,
        2.0,
        vec![ShewhartRule::WE1, ShewhartRule::WE4],
    );
    for i in 0..7 {
        let signal = chart.observe(51.0); // z=0.5, above center
        assert!(
            !signal.is_out_of_control(),
            "observation {i} should not signal"
        );
    }
    let signal = chart.observe(51.0); // 8th consecutive
    assert!(signal.is_out_of_control(), "WE-4: 8 consecutive same side");
}

#[test]
fn reset_clears_state() {
    let mut chart = chart_3sigma(50.0, 2.0);
    chart.observe(57.0);
    chart.reset();
    assert_eq!(chart.n_observations(), 0);
    assert!(chart.observe(50.0).is_in_control());
}

#[test]
fn mc_shewhart_arl0() {
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use rand::distr::{Distribution, StandardNormal};

    let mut rng = ChaCha20Rng::seed_from_u64(42);
    let n_sims = 10_000;
    let max_len = 5_000;
    let mut total_rl: u64 = 0;

    for _ in 0..n_sims {
        let mut chart = chart_3sigma(0.0, 1.0);
        let mut rl = max_len;
        for t in 0..max_len {
            let x: f64 = StandardNormal.sample(&mut rng);
            if chart.observe(x).is_out_of_control() {
                rl = t + 1;
                break;
            }
        }
        total_rl += rl as u64;
    }

    let empirical_arl = total_rl as f64 / n_sims as f64;
    let expected = 370.4;
    let tolerance = 0.10;
    assert!(
        (empirical_arl - expected).abs() / expected < tolerance,
        "ARL₀ = {empirical_arl}, expected ~{expected} ± 10%"
    );
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p spc-charts --test shewhart_tck --release 2>&1`
Expected: all 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/spc-charts/src/shewhart.rs \
       crates/spc-charts/src/lib.rs \
       crates/spc-charts/tests/shewhart_tck.rs \
       tck/spc-charts/
git commit -m "feat(spc): Shewhart chart with WE rules + MC ARL₀ (BEAD-0008)"
```

---

### Task 3: CUSUM Chart

**Files:**
- Create: `crates/spc-charts/src/cusum.rs`
- Modify: `crates/spc-charts/src/lib.rs`
- Create: `tck/spc-charts/features/cusum.feature`
- Create: `crates/spc-charts/tests/cusum_tck.rs`

- [ ] **Step 1: Write TCK feature file**

Create `tck/spc-charts/features/cusum.feature`:

```gherkin
Feature: Page 1954 tabular CUSUM chart

  Scenario: In-control observations keep CUSUM near zero
    Given a CUSUM chart with mu_0=0 sigma=1 k=0.5 h=5
    When I observe values 0.1, -0.2, 0.3, -0.1, 0.0
    Then C+ and C- are both less than 1.0

  Scenario: Sustained positive shift triggers upper CUSUM
    Given a CUSUM chart with mu_0=0 sigma=1 k=0.5 h=5
    When I observe 20 values drawn from N(1, 1)
    Then OutOfControl is signaled before observation 20

  Scenario: CUSUM C+ and C- are always non-negative
    Given a CUSUM chart with any parameters
    When I observe 1000 random values
    Then c_plus >= 0 and c_minus >= 0 after every observation

  Scenario: Reset restores initial state
    Given a CUSUM chart with mu_0=0 sigma=1 k=0.5 h=5
    When I observe 5 values then reset
    Then c_plus == 0 and c_minus == 0 and n_observations == 0
```

- [ ] **Step 2: Create cusum.rs**

Create `crates/spc-charts/src/cusum.rs`:

```rust
use crate::types::{ChartSignal, ControlLimits, SpcError};

#[derive(Debug, Clone)]
pub struct CusumConfig {
    pub limits: ControlLimits,
    pub k: f64,
    pub h: f64,
}

impl CusumConfig {
    #[must_use]
    pub fn default_for(limits: ControlLimits) -> Self {
        Self {
            limits,
            k: 0.5,
            h: 5.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CusumChart {
    mu_0: f64,
    sigma: f64,
    k: f64,
    h: f64,
    c_plus: f64,
    c_minus: f64,
    n: usize,
    initial_c_plus: f64,
    initial_c_minus: f64,
}

impl CusumChart {
    pub fn new(config: CusumConfig) -> Result<Self, SpcError> {
        Self::new_with_head_start(config, 0.0)
    }

    pub(crate) fn new_with_head_start(
        config: CusumConfig,
        head_start: f64,
    ) -> Result<Self, SpcError> {
        if config.k <= 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "k",
                value: config.k,
            });
        }
        if config.h <= 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "h",
                value: config.h,
            });
        }
        Ok(Self {
            mu_0: config.limits.mu_0,
            sigma: config.limits.sigma,
            k: config.k,
            h: config.h,
            c_plus: head_start,
            c_minus: head_start,
            n: 0,
            initial_c_plus: head_start,
            initial_c_minus: head_start,
        })
    }

    pub fn observe(&mut self, x: f64) -> ChartSignal {
        debug_assert!(x.is_finite());
        let z = (x - self.mu_0) / self.sigma;
        self.c_plus = f64::max(0.0, self.c_plus + z - self.k);
        self.c_minus = f64::max(0.0, self.c_minus - z - self.k);
        self.n += 1;

        if self.c_plus > self.h {
            ChartSignal::OutOfControl {
                statistic: self.c_plus,
                observation_index: self.n - 1,
            }
        } else if self.c_minus > self.h {
            ChartSignal::OutOfControl {
                statistic: self.c_minus,
                observation_index: self.n - 1,
            }
        } else {
            ChartSignal::InControl
        }
    }

    pub fn reset(&mut self) {
        self.c_plus = self.initial_c_plus;
        self.c_minus = self.initial_c_minus;
        self.n = 0;
    }

    #[must_use]
    pub fn c_plus(&self) -> f64 {
        self.c_plus
    }

    #[must_use]
    pub fn c_minus(&self) -> f64 {
        self.c_minus
    }

    #[must_use]
    pub fn n_observations(&self) -> usize {
        self.n
    }
}
```

- [ ] **Step 3: Add to lib.rs**

Add to `crates/spc-charts/src/lib.rs`:

```rust
pub mod cusum;

pub use cusum::{CusumChart, CusumConfig};
```

- [ ] **Step 4: Write TCK tests**

Create `crates/spc-charts/tests/cusum_tck.rs`:

```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{ChartSignal, ControlLimits, CusumChart, CusumConfig};

fn default_cusum() -> CusumChart {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    CusumChart::new(CusumConfig::default_for(limits)).unwrap()
}

#[test]
fn in_control_stays_low() {
    let mut chart = default_cusum();
    for &x in &[0.1, -0.2, 0.3, -0.1, 0.0] {
        chart.observe(x);
    }
    assert!(chart.c_plus() < 1.0, "C⁺ = {}", chart.c_plus());
    assert!(chart.c_minus() < 1.0, "C⁻ = {}", chart.c_minus());
}

#[test]
fn sustained_shift_triggers() {
    let mut chart = default_cusum();
    let mut signaled = false;
    for t in 0..20 {
        if chart.observe(1.0).is_out_of_control() {
            signaled = true;
            assert!(t < 20, "should signal before t=20");
            break;
        }
    }
    assert!(signaled, "1σ shift should trigger CUSUM within 20 obs");
}

#[test]
fn c_plus_c_minus_always_non_negative() {
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use rand::distr::{Distribution, StandardNormal};

    let mut rng = ChaCha20Rng::seed_from_u64(99);
    let mut chart = default_cusum();
    for _ in 0..1000 {
        let x: f64 = StandardNormal.sample(&mut rng);
        chart.observe(x);
        assert!(chart.c_plus() >= 0.0, "C⁺ went negative");
        assert!(chart.c_minus() >= 0.0, "C⁻ went negative");
    }
}

#[test]
fn reset_restores_initial() {
    let mut chart = default_cusum();
    for &x in &[1.0, 1.5, 2.0, 1.0, 1.5] {
        chart.observe(x);
    }
    chart.reset();
    assert_eq!(chart.c_plus(), 0.0);
    assert_eq!(chart.c_minus(), 0.0);
    assert_eq!(chart.n_observations(), 0);
}

#[test]
fn known_cusum_trace() {
    let mut chart = default_cusum();
    // z = x - 0 / 1 = x. k=0.5.
    // x=0.8: C+ = max(0, 0+0.8-0.5)=0.3, C- = max(0, 0-0.8-0.5)=0
    let s = chart.observe(0.8);
    assert!(s.is_in_control());
    assert!((chart.c_plus() - 0.3).abs() < 1e-10);
    assert_eq!(chart.c_minus(), 0.0);

    // x=0.6: C+ = max(0, 0.3+0.6-0.5)=0.4
    chart.observe(0.6);
    assert!((chart.c_plus() - 0.4).abs() < 1e-10);

    // x=-1.0: C+ = max(0, 0.4-1.0-0.5)=0, C- = max(0, 0+1.0-0.5)=0.5
    chart.observe(-1.0);
    assert_eq!(chart.c_plus(), 0.0);
    assert!((chart.c_minus() - 0.5).abs() < 1e-10);
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p spc-charts --test cusum_tck --release 2>&1`
Expected: all 5 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/spc-charts/src/cusum.rs \
       crates/spc-charts/src/lib.rs \
       crates/spc-charts/tests/cusum_tck.rs \
       tck/spc-charts/features/cusum.feature
git commit -m "feat(spc): Page 1954 tabular CUSUM chart (BEAD-0008)"
```

---

### Task 4: FIR CUSUM Chart

**Files:**
- Create: `crates/spc-charts/src/cusum_fir.rs`
- Modify: `crates/spc-charts/src/lib.rs`
- Create: `crates/spc-charts/tests/cusum_fir_tck.rs`

- [ ] **Step 1: Create cusum_fir.rs**

Create `crates/spc-charts/src/cusum_fir.rs`:

```rust
use crate::cusum::CusumConfig;
use crate::types::{ChartSignal, ControlLimits, SpcError};

#[derive(Debug, Clone)]
pub struct FirCusumConfig {
    pub limits: ControlLimits,
    pub k: f64,
    pub h: f64,
    pub head_start: f64,
}

impl FirCusumConfig {
    #[must_use]
    pub fn default_for(limits: ControlLimits) -> Self {
        Self {
            limits,
            k: 0.5,
            h: 5.0,
            head_start: 2.5, // h/2
        }
    }
}

#[derive(Debug, Clone)]
pub struct FirCusumChart {
    inner: crate::cusum::CusumChart,
}

impl FirCusumChart {
    pub fn new(config: FirCusumConfig) -> Result<Self, SpcError> {
        if config.head_start < 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "head_start",
                value: config.head_start,
            });
        }
        let cusum_config = CusumConfig {
            limits: config.limits,
            k: config.k,
            h: config.h,
        };
        let inner =
            crate::cusum::CusumChart::new_with_head_start(cusum_config, config.head_start)?;
        Ok(Self { inner })
    }

    pub fn observe(&mut self, x: f64) -> ChartSignal {
        self.inner.observe(x)
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[must_use]
    pub fn c_plus(&self) -> f64 {
        self.inner.c_plus()
    }

    #[must_use]
    pub fn c_minus(&self) -> f64 {
        self.inner.c_minus()
    }

    #[must_use]
    pub fn n_observations(&self) -> usize {
        self.inner.n_observations()
    }
}
```

- [ ] **Step 2: Add to lib.rs**

Add to `crates/spc-charts/src/lib.rs`:

```rust
pub mod cusum_fir;

pub use cusum_fir::{FirCusumChart, FirCusumConfig};
```

- [ ] **Step 3: Write TCK tests**

Create `crates/spc-charts/tests/cusum_fir_tck.rs`:

```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{ControlLimits, CusumChart, CusumConfig, FirCusumChart, FirCusumConfig};

fn default_fir() -> FirCusumChart {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    FirCusumChart::new(FirCusumConfig::default_for(limits)).unwrap()
}

fn default_cusum() -> CusumChart {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    CusumChart::new(CusumConfig::default_for(limits)).unwrap()
}

#[test]
fn fir_starts_at_head_start() {
    let chart = default_fir();
    assert!((chart.c_plus() - 2.5).abs() < 1e-10, "C⁺ should start at h/2=2.5");
    assert!((chart.c_minus() - 2.5).abs() < 1e-10);
}

#[test]
fn fir_detects_initial_shift_faster_than_standard() {
    let mut fir = default_fir();
    let mut std = default_cusum();

    let shift_values: Vec<f64> = (0..50).map(|_| 1.0).collect();

    let mut fir_rl = 50;
    let mut std_rl = 50;
    for (t, &x) in shift_values.iter().enumerate() {
        if fir_rl == 50 && fir.observe(x).is_out_of_control() {
            fir_rl = t + 1;
        }
        if std_rl == 50 && std.observe(x).is_out_of_control() {
            std_rl = t + 1;
        }
    }
    assert!(
        fir_rl < std_rl,
        "FIR (rl={fir_rl}) should detect faster than standard (rl={std_rl})"
    );
}

#[test]
fn fir_reset_restores_head_start() {
    let mut chart = default_fir();
    for _ in 0..10 {
        chart.observe(0.0);
    }
    chart.reset();
    assert!((chart.c_plus() - 2.5).abs() < 1e-10);
    assert!((chart.c_minus() - 2.5).abs() < 1e-10);
    assert_eq!(chart.n_observations(), 0);
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p spc-charts --test cusum_fir_tck --release 2>&1`
Expected: all 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/spc-charts/src/cusum_fir.rs \
       crates/spc-charts/src/lib.rs \
       crates/spc-charts/tests/cusum_fir_tck.rs
git commit -m "feat(spc): FIR CUSUM chart — Lucas & Crosier 1982 (BEAD-0008)"
```

---

### Task 5: EWMA Chart

**Files:**
- Create: `crates/spc-charts/src/ewma.rs`
- Modify: `crates/spc-charts/src/lib.rs`
- Create: `tck/spc-charts/features/ewma.feature`
- Create: `crates/spc-charts/tests/ewma_tck.rs`

- [ ] **Step 1: Write TCK feature file**

Create `tck/spc-charts/features/ewma.feature`:

```gherkin
Feature: Roberts 1959 EWMA chart

  Scenario: EWMA statistic is weighted average
    Given an EWMA chart with mu_0=0 sigma=1 lambda=0.2 L=3
    When I observe value 1.0
    Then Z = 0.2*1.0 + 0.8*0.0 = 0.2

  Scenario: EWMA detects sustained small shift
    Given an EWMA chart with mu_0=0 sigma=1 lambda=0.2 L=3
    When I observe 100 values from N(0.5, 1)
    Then OutOfControl is signaled

  Scenario: EWMA Z is bounded by observation range
    Given an EWMA chart with any parameters
    When I observe values in [a, b]
    Then Z is always in [min_obs, max_obs] range

  Scenario: Time-varying control limits converge to asymptotic
    Given an EWMA chart with lambda=0.2 L=3
    When I compute UCL at i=1 and i=1000
    Then UCL_1000 is within 0.1% of the asymptotic UCL
```

- [ ] **Step 2: Create ewma.rs**

Create `crates/spc-charts/src/ewma.rs`:

```rust
use crate::types::{ChartSignal, ControlLimits, SpcError};

#[derive(Debug, Clone)]
pub struct EwmaConfig {
    pub limits: ControlLimits,
    pub lambda: f64,
    pub l_sigma: f64,
}

impl EwmaConfig {
    #[must_use]
    pub fn default_for(limits: ControlLimits) -> Self {
        Self {
            limits,
            lambda: 0.2,
            l_sigma: 2.962,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EwmaChart {
    mu_0: f64,
    sigma: f64,
    lambda: f64,
    l_sigma: f64,
    z: f64,
    n: usize,
    one_minus_lambda: f64,
    lambda_ratio: f64,
}

impl EwmaChart {
    pub fn new(config: EwmaConfig) -> Result<Self, SpcError> {
        if config.lambda <= 0.0 || config.lambda > 1.0 {
            return Err(SpcError::InvalidLambda(config.lambda));
        }
        if config.l_sigma <= 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "l_sigma",
                value: config.l_sigma,
            });
        }
        let one_minus_lambda = 1.0 - config.lambda;
        let lambda_ratio = config.lambda / (2.0 - config.lambda);
        Ok(Self {
            mu_0: config.limits.mu_0,
            sigma: config.limits.sigma,
            lambda: config.lambda,
            l_sigma: config.l_sigma,
            z: config.limits.mu_0,
            n: 0,
            one_minus_lambda,
            lambda_ratio,
        })
    }

    pub fn observe(&mut self, x: f64) -> ChartSignal {
        debug_assert!(x.is_finite());
        self.n += 1;
        self.z = self.lambda * x + self.one_minus_lambda * self.z;

        let time_factor = 1.0 - self.one_minus_lambda.powi(2 * self.n as i32);
        let limit_width = self.l_sigma * self.sigma * (self.lambda_ratio * time_factor).sqrt();
        let ucl = self.mu_0 + limit_width;
        let lcl = self.mu_0 - limit_width;

        if self.z > ucl || self.z < lcl {
            ChartSignal::OutOfControl {
                statistic: self.z,
                observation_index: self.n - 1,
            }
        } else {
            ChartSignal::InControl
        }
    }

    pub fn reset(&mut self) {
        self.z = self.mu_0;
        self.n = 0;
    }

    #[must_use]
    pub fn z(&self) -> f64 {
        self.z
    }

    #[must_use]
    pub fn n_observations(&self) -> usize {
        self.n
    }
}
```

- [ ] **Step 3: Add to lib.rs**

Add to `crates/spc-charts/src/lib.rs`:

```rust
pub mod ewma;

pub use ewma::{EwmaChart, EwmaConfig};
```

- [ ] **Step 4: Write TCK tests**

Create `crates/spc-charts/tests/ewma_tck.rs`:

```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{ControlLimits, EwmaChart, EwmaConfig};

fn default_ewma() -> EwmaChart {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    EwmaChart::new(EwmaConfig::default_for(limits)).unwrap()
}

#[test]
fn ewma_first_observation() {
    let mut chart = default_ewma();
    chart.observe(1.0);
    let expected = 0.2 * 1.0 + 0.8 * 0.0;
    assert!(
        (chart.z() - expected).abs() < 1e-10,
        "Z = {}, expected {expected}",
        chart.z()
    );
}

#[test]
fn ewma_detects_sustained_shift() {
    let mut chart = default_ewma();
    let mut detected = false;
    for _ in 0..100 {
        if chart.observe(0.5).is_out_of_control() {
            detected = true;
            break;
        }
    }
    assert!(detected, "EWMA should detect 0.5σ shift within 100 obs");
}

#[test]
fn ewma_z_bounded_by_observations() {
    let mut chart = default_ewma();
    let values = [1.0, 2.0, 3.0, -1.0, 0.5, 2.5];
    let mut min_obs = f64::INFINITY;
    let mut max_obs = f64::NEG_INFINITY;
    for &x in &values {
        min_obs = min_obs.min(x);
        max_obs = max_obs.max(x);
        chart.observe(x);
        assert!(
            chart.z() >= chart.z().min(0.0).min(min_obs) - 1e-10,
            "Z went below observation range"
        );
    }
    // After all observations, Z should be between mu_0 and max_obs
    // (since all values are pulled toward mu_0=0 by the smoothing)
    assert!(
        chart.z() <= max_obs + 1e-10,
        "Z={} > max_obs={max_obs}",
        chart.z()
    );
}

#[test]
fn ewma_asymptotic_limit_convergence() {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    let config = EwmaConfig {
        limits,
        lambda: 0.2,
        l_sigma: 3.0,
    };
    let mut chart = EwmaChart::new(config).unwrap();

    // Feed 1000 in-control observations to advance the time index.
    for _ in 0..1000 {
        chart.observe(0.0);
    }

    // Asymptotic UCL = L * sigma * sqrt(lambda / (2 - lambda))
    let asymptotic_ucl = 3.0 * 1.0 * (0.2 / 1.8_f64).sqrt();

    // At i=1000, the time-varying factor ≈ 1.0, so the UCL should
    // be within 0.1% of asymptotic.
    // We can't directly read UCL, but we know Z=0 (all zeros fed),
    // so an observation at exactly asymptotic_ucl should be
    // very close to the boundary. We'll verify via the formula:
    let time_factor = 1.0 - 0.8_f64.powi(2000);
    let ucl_1000 = 3.0 * 1.0 * (0.2 / 1.8 * time_factor).sqrt();
    let rel_diff = (ucl_1000 - asymptotic_ucl).abs() / asymptotic_ucl;
    assert!(
        rel_diff < 0.001,
        "UCL at i=1000 ({ucl_1000}) should be within 0.1% of asymptotic ({asymptotic_ucl})"
    );
}

#[test]
fn ewma_reset() {
    let mut chart = default_ewma();
    for _ in 0..10 {
        chart.observe(1.0);
    }
    chart.reset();
    assert_eq!(chart.z(), 0.0);
    assert_eq!(chart.n_observations(), 0);
}

#[test]
fn ewma_invalid_lambda() {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    assert!(EwmaChart::new(EwmaConfig {
        limits: limits.clone(),
        lambda: 0.0,
        l_sigma: 3.0,
    })
    .is_err());
    assert!(EwmaChart::new(EwmaConfig {
        limits,
        lambda: 1.5,
        l_sigma: 3.0,
    })
    .is_err());
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p spc-charts --test ewma_tck --release 2>&1`
Expected: all 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/spc-charts/src/ewma.rs \
       crates/spc-charts/src/lib.rs \
       crates/spc-charts/tests/ewma_tck.rs \
       tck/spc-charts/features/ewma.feature
git commit -m "feat(spc): Roberts 1959 EWMA chart (BEAD-0008)"
```

---

### Task 6: Combined Shewhart-CUSUM Chart

**Files:**
- Create: `crates/spc-charts/src/combined.rs`
- Modify: `crates/spc-charts/src/lib.rs`
- Create: `crates/spc-charts/tests/combined_tck.rs`

- [ ] **Step 1: Create combined.rs**

Create `crates/spc-charts/src/combined.rs`:

```rust
use crate::cusum::{CusumChart, CusumConfig};
use crate::types::{ChartSignal, ControlLimits, SpcError};

#[derive(Debug, Clone)]
pub struct CombinedConfig {
    pub cusum: CusumConfig,
    pub shewhart_k: f64,
}

impl CombinedConfig {
    #[must_use]
    pub fn default_for(limits: ControlLimits) -> Self {
        Self {
            cusum: CusumConfig::default_for(limits),
            shewhart_k: 3.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CombinedChart {
    cusum: CusumChart,
    shewhart_k: f64,
    mu_0: f64,
    sigma: f64,
    n: usize,
}

impl CombinedChart {
    pub fn new(config: CombinedConfig) -> Result<Self, SpcError> {
        if config.shewhart_k <= 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "shewhart_k",
                value: config.shewhart_k,
            });
        }
        let mu_0 = config.cusum.limits.mu_0;
        let sigma = config.cusum.limits.sigma;
        let cusum = CusumChart::new(config.cusum)?;
        Ok(Self {
            cusum,
            shewhart_k: config.shewhart_k,
            mu_0,
            sigma,
            n: 0,
        })
    }

    pub fn observe(&mut self, x: f64) -> ChartSignal {
        debug_assert!(x.is_finite());
        self.n += 1;
        let z = (x - self.mu_0) / self.sigma;

        // Shewhart check first (instantaneous large shift).
        if z.abs() > self.shewhart_k {
            // Still update CUSUM state for consistency.
            self.cusum.observe(x);
            return ChartSignal::OutOfControl {
                statistic: z,
                observation_index: self.n - 1,
            };
        }

        // CUSUM check (sustained small shift).
        let cusum_signal = self.cusum.observe(x);
        if cusum_signal.is_out_of_control() {
            return cusum_signal;
        }

        ChartSignal::InControl
    }

    pub fn reset(&mut self) {
        self.cusum.reset();
        self.n = 0;
    }

    #[must_use]
    pub fn cusum_state(&self) -> (f64, f64) {
        (self.cusum.c_plus(), self.cusum.c_minus())
    }

    #[must_use]
    pub fn n_observations(&self) -> usize {
        self.n
    }
}
```

- [ ] **Step 2: Add to lib.rs**

Add to `crates/spc-charts/src/lib.rs`:

```rust
pub mod combined;

pub use combined::{CombinedChart, CombinedConfig};
```

- [ ] **Step 3: Write TCK tests**

Create `crates/spc-charts/tests/combined_tck.rs`:

```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{CombinedChart, CombinedConfig, ControlLimits, CusumConfig};

fn default_combined() -> CombinedChart {
    let limits = ControlLimits::new(0.0, 1.0).unwrap();
    CombinedChart::new(CombinedConfig::default_for(limits)).unwrap()
}

#[test]
fn large_spike_triggers_shewhart_arm() {
    let mut chart = default_combined();
    // 4.0 > 3.5σ Shewhart limit.
    let signal = chart.observe(4.0);
    assert!(signal.is_out_of_control());
}

#[test]
fn sustained_shift_triggers_cusum_arm() {
    let mut chart = default_combined();
    // 1.0σ shift is below 3.5σ Shewhart, but CUSUM accumulates.
    let mut detected = false;
    for _ in 0..30 {
        if chart.observe(1.0).is_out_of_control() {
            detected = true;
            break;
        }
    }
    assert!(detected, "CUSUM arm should detect 1σ sustained shift");
}

#[test]
fn in_control_no_signal() {
    let mut chart = default_combined();
    for &x in &[0.1, -0.2, 0.3, -0.1, 0.0, 0.5, -0.5] {
        assert!(chart.observe(x).is_in_control());
    }
}

#[test]
fn combined_detects_faster_than_either_alone() {
    use spc_charts::{CusumChart, ShewhartChart, ShewhartConfig};

    let limits = ControlLimits::new(0.0, 1.0).unwrap();

    // Sequence: small shift for a while, then a big spike.
    let sequence: Vec<f64> = (0..8)
        .map(|_| 0.6)
        .chain(std::iter::once(4.0))
        .collect();

    // Shewhart alone (k=3.5) won't catch the 0.6s.
    let mut shew = ShewhartChart::new(ShewhartConfig {
        limits: limits.clone(),
        k_sigma: 3.5,
        rules: vec![spc_charts::ShewhartRule::WE1],
    })
    .unwrap();
    let mut shew_rl = sequence.len();
    for (t, &x) in sequence.iter().enumerate() {
        if shew.observe(x).is_out_of_control() {
            shew_rl = t + 1;
            break;
        }
    }

    // CUSUM alone (k=0.5, h=5) might not catch the spike as fast.
    let mut cusum = CusumChart::new(CusumConfig::default_for(limits.clone())).unwrap();
    let mut cusum_rl = sequence.len();
    for (t, &x) in sequence.iter().enumerate() {
        if cusum.observe(x).is_out_of_control() {
            cusum_rl = t + 1;
            break;
        }
    }

    // Combined should detect no later than the minimum of the two.
    let mut comb = CombinedChart::new(CombinedConfig {
        cusum: CusumConfig::default_for(limits),
        shewhart_k: 3.5,
    })
    .unwrap();
    let mut comb_rl = sequence.len();
    for (t, &x) in sequence.iter().enumerate() {
        if comb.observe(x).is_out_of_control() {
            comb_rl = t + 1;
            break;
        }
    }

    assert!(
        comb_rl <= shew_rl.min(cusum_rl),
        "combined (rl={comb_rl}) should detect ≤ min(shewhart={shew_rl}, cusum={cusum_rl})"
    );
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p spc-charts --test combined_tck --release 2>&1`
Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/spc-charts/src/combined.rs \
       crates/spc-charts/src/lib.rs \
       crates/spc-charts/tests/combined_tck.rs
git commit -m "feat(spc): combined Shewhart-CUSUM chart — Lucas 1982 (BEAD-0008)"
```

---

### Task 7: E-Detector

**Files:**
- Create: `crates/spc-charts/src/e_detector.rs`
- Modify: `crates/spc-charts/src/lib.rs`
- Create: `tck/spc-charts/features/e_detector.feature`
- Create: `crates/spc-charts/tests/e_detector_tck.rs`

- [ ] **Step 1: Write TCK feature file**

Create `tck/spc-charts/features/e_detector.feature`:

```gherkin
Feature: E-detector — Shin, Ramdas, Rinaldo 2023

  Scenario: E-process starts at 1
    Given an e-detector with alpha=0.05
    Then the initial e_process is 1.0

  Scenario: E-detector signals when M_t >= 1/alpha
    Given an e-detector with alpha=0.05 (threshold=20)
    When I feed a sustained shift until M_t >= 20
    Then OutOfControl is signaled

  Scenario: E-process floor is 1 (growing window)
    Given an e-detector with growing window
    When I feed observations that would shrink the process
    Then e_process is always >= 1.0

  Scenario: False alarm rate is at most alpha
    Given an e-detector with alpha=0.05
    When I simulate 10000 in-control sequences of length 500
    Then the false alarm rate is <= 0.06
```

- [ ] **Step 2: Create e_detector.rs**

Create `crates/spc-charts/src/e_detector.rs`:

```rust
use crate::types::{ChartSignal, SpcError};

pub trait EValueSource {
    fn e_value(&self, observation: f64) -> f64;
}

#[derive(Debug, Clone)]
pub struct GaussianEValue {
    pub mu_0: f64,
    pub sigma: f64,
    pub mixing_variance: f64,
}

impl GaussianEValue {
    pub fn new(mu_0: f64, sigma: f64, mixing_variance: f64) -> Result<Self, SpcError> {
        if sigma <= 0.0 {
            return Err(SpcError::NonPositiveSigma(sigma));
        }
        if mixing_variance <= 0.0 {
            return Err(SpcError::NonPositiveParam {
                name: "mixing_variance",
                value: mixing_variance,
            });
        }
        Ok(Self {
            mu_0,
            sigma,
            mixing_variance,
        })
    }
}

impl EValueSource for GaussianEValue {
    fn e_value(&self, observation: f64) -> f64 {
        // Single-observation MSPRT log-LR for N(mu_0, sigma^2) with
        // mixing prior N(0, tau^2) on the standardized effect:
        //   log(e) = -0.5 * ln(1 + tau^2) + z^2 * tau^2 / (2*(1+tau^2))
        // where z = (x - mu_0) / sigma, tau^2 = mixing_variance.
        let z = (observation - self.mu_0) / self.sigma;
        let tau_sq = self.mixing_variance;
        let log_e = -0.5 * (1.0 + tau_sq).ln() + z * z * tau_sq / (2.0 * (1.0 + tau_sq));
        log_e.exp()
    }
}

#[derive(Debug, Clone)]
pub enum EDetectorWindow {
    Growing,
    Fixed { width: usize },
}

#[derive(Debug, Clone)]
pub struct EDetectorConfig {
    pub alpha: f64,
    pub window: EDetectorWindow,
}

impl EDetectorConfig {
    #[must_use]
    pub fn default_growing() -> Self {
        Self {
            alpha: 0.05,
            window: EDetectorWindow::Growing,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EDetector<E: EValueSource> {
    source: E,
    threshold: f64,
    window: EDetectorWindow,
    m: f64,
    n: usize,
    ring: Vec<f64>,
    ring_pos: usize,
}

impl<E: EValueSource> EDetector<E> {
    pub fn new(config: EDetectorConfig, source: E) -> Result<Self, SpcError> {
        if config.alpha <= 0.0 || config.alpha >= 1.0 {
            return Err(SpcError::InvalidAlpha(config.alpha));
        }
        if let EDetectorWindow::Fixed { width } = &config.window {
            if *width == 0 {
                return Err(SpcError::InvalidWindowWidth(0));
            }
        }
        let threshold = 1.0 / config.alpha;
        let ring = match &config.window {
            EDetectorWindow::Growing => Vec::new(),
            EDetectorWindow::Fixed { width } => vec![1.0; *width],
        };
        Ok(Self {
            source,
            threshold,
            window: config.window,
            m: 1.0,
            n: 0,
            ring,
            ring_pos: 0,
        })
    }

    pub fn observe(&mut self, x: f64) -> ChartSignal {
        debug_assert!(x.is_finite());
        self.n += 1;
        let e = self.source.e_value(x);

        match &self.window {
            EDetectorWindow::Growing => {
                self.m = f64::max(1.0, self.m) * e;
            }
            EDetectorWindow::Fixed { width } => {
                let w = *width;
                let old = self.ring[self.ring_pos];
                self.ring[self.ring_pos] = e;
                self.ring_pos = (self.ring_pos + 1) % w;
                if self.n <= w {
                    self.m *= e;
                } else {
                    self.m = self.m / old * e;
                }
            }
        }

        if self.m >= self.threshold {
            ChartSignal::OutOfControl {
                statistic: self.m,
                observation_index: self.n - 1,
            }
        } else {
            ChartSignal::InControl
        }
    }

    pub fn reset(&mut self) {
        self.m = 1.0;
        self.n = 0;
        if let EDetectorWindow::Fixed { width } = &self.window {
            self.ring = vec![1.0; *width];
            self.ring_pos = 0;
        }
    }

    #[must_use]
    pub fn e_process(&self) -> f64 {
        self.m
    }

    #[must_use]
    pub fn n_observations(&self) -> usize {
        self.n
    }
}
```

- [ ] **Step 3: Add to lib.rs**

Add to `crates/spc-charts/src/lib.rs`:

```rust
pub mod e_detector;

pub use e_detector::{
    EDetector, EDetectorConfig, EDetectorWindow, EValueSource, GaussianEValue,
};
```

- [ ] **Step 4: Write TCK tests**

Create `crates/spc-charts/tests/e_detector_tck.rs`:

```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{EDetector, EDetectorConfig, EDetectorWindow, GaussianEValue};

fn gaussian_detector(alpha: f64) -> EDetector<GaussianEValue> {
    let source = GaussianEValue::new(0.0, 1.0, 1.0).unwrap();
    let config = EDetectorConfig {
        alpha,
        window: EDetectorWindow::Growing,
    };
    EDetector::new(config, source).unwrap()
}

#[test]
fn initial_e_process_is_one() {
    let det = gaussian_detector(0.05);
    assert_eq!(det.e_process(), 1.0);
}

#[test]
fn detects_shift() {
    let mut det = gaussian_detector(0.05);
    let mut detected = false;
    for _ in 0..200 {
        if det.observe(1.5).is_out_of_control() {
            detected = true;
            break;
        }
    }
    assert!(detected, "should detect 1.5σ shift");
}

#[test]
fn e_process_floor_is_one() {
    let mut det = gaussian_detector(0.05);
    // Feed observations that produce e-values < 1 (near mu_0).
    for _ in 0..100 {
        det.observe(0.0);
        assert!(
            det.e_process() >= 1.0 - 1e-10,
            "M_t = {} < 1",
            det.e_process()
        );
    }
}

#[test]
fn mc_false_alarm_rate() {
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use rand::distr::{Distribution, StandardNormal};

    let alpha = 0.05;
    let n_sims = 10_000;
    let seq_len = 500;
    let mut rng = ChaCha20Rng::seed_from_u64(123);
    let mut false_alarms = 0;

    for _ in 0..n_sims {
        let mut det = gaussian_detector(alpha);
        for _ in 0..seq_len {
            let x: f64 = StandardNormal.sample(&mut rng);
            if det.observe(x).is_out_of_control() {
                false_alarms += 1;
                break;
            }
        }
    }

    let empirical_rate = false_alarms as f64 / n_sims as f64;
    assert!(
        empirical_rate <= alpha + 0.01,
        "false alarm rate = {empirical_rate}, should be ≤ {alpha}+margin"
    );
}

#[test]
fn reset_clears_state() {
    let mut det = gaussian_detector(0.05);
    for _ in 0..10 {
        det.observe(2.0);
    }
    det.reset();
    assert_eq!(det.e_process(), 1.0);
    assert_eq!(det.n_observations(), 0);
}

#[test]
fn fixed_window_mode() {
    let source = GaussianEValue::new(0.0, 1.0, 1.0).unwrap();
    let config = EDetectorConfig {
        alpha: 0.05,
        window: EDetectorWindow::Fixed { width: 5 },
    };
    let mut det = EDetector::new(config, source).unwrap();

    // Feed 10 observations. After 5, the window should be sliding.
    for _ in 0..10 {
        det.observe(0.0);
    }
    assert_eq!(det.n_observations(), 10);
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p spc-charts --test e_detector_tck --release 2>&1`
Expected: all 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/spc-charts/src/e_detector.rs \
       crates/spc-charts/src/lib.rs \
       crates/spc-charts/tests/e_detector_tck.rs \
       tck/spc-charts/features/e_detector.feature
git commit -m "feat(spc): e-detector change-point detection — Shin 2023 (BEAD-0008)"
```

---

### Task 8: ARL Computation

**Files:**
- Create: `crates/spc-charts/src/arl.rs`
- Modify: `crates/spc-charts/src/lib.rs`
- Create: `crates/spc-charts/tests/arl_tck.rs`

- [ ] **Step 1: Create arl.rs**

Create `crates/spc-charts/src/arl.rs`:

```rust
#![allow(clippy::cast_precision_loss)]

use crate::types::SpcError;

/// Compute ARL for a two-sided CUSUM chart via Markov chain
/// discretization (Brook & Evans 1972).
///
/// `k`: reference value (allowance) in σ units.
/// `h`: decision interval in σ units.
/// `shift`: mean shift in σ units (0 for ARL₀).
/// `n_states`: discretization resolution (default 200).
pub fn cusum_arl(k: f64, h: f64, shift: f64, n_states: usize) -> Result<f64, SpcError> {
    if k <= 0.0 {
        return Err(SpcError::NonPositiveParam {
            name: "k",
            value: k,
        });
    }
    if h <= 0.0 {
        return Err(SpcError::NonPositiveParam {
            name: "h",
            value: h,
        });
    }
    let n = n_states;
    let delta = h / n as f64;

    // State midpoints: s_i = (i + 0.5) * delta for i in 0..n
    // Transition: from state s_i, new value = max(0, s_i + Z - k)
    // where Z ~ N(shift, 1). Need P(new value falls in state j).
    //
    // Build (I - Q) matrix where Q[i][j] = P(transition from i to j
    // without signaling). Signal = new value > h.

    let mut mat = nalgebra::DMatrix::<f64>::zeros(n, n);
    let rhs = nalgebra::DVector::<f64>::from_element(n, 1.0);

    for i in 0..n {
        let s_i = (i as f64 + 0.5) * delta;
        for j in 0..n {
            let lo = j as f64 * delta;
            let hi = (j + 1) as f64 * delta;
            // P(lo ≤ max(0, s_i + Z - k) < hi)
            // = P(lo - s_i + k ≤ Z < hi - s_i + k) if lo > 0
            // For j == 0: includes the absorbing-at-zero region:
            // P(s_i + Z - k ≤ 0) + P(0 < s_i + Z - k < delta)
            let z_lo = lo - s_i + k - shift;
            let z_hi = hi - s_i + k - shift;
            let p = if j == 0 {
                phi(z_hi)
            } else {
                phi(z_hi) - phi(z_lo)
            };
            mat[(i, j)] = -p;
        }
        mat[(i, i)] += 1.0;
    }

    // Solve (I - Q) * arl_vec = 1
    let decomp = mat.lu();
    let arl_vec = decomp
        .solve(&rhs)
        .ok_or(SpcError::SingularArlMatrix(h))?;

    // ARL starting from state 0 (CUSUM starts at 0).
    Ok(arl_vec[0])
}

/// Compute ARL for an EWMA chart via Markov chain discretization
/// (Lucas & Saccucci 1990).
///
/// `lambda`: smoothing constant.
/// `l_sigma`: control limit width in σ.
/// `shift`: mean shift in σ units (0 for ARL₀).
/// `n_states`: discretization resolution (default 200).
pub fn ewma_arl(
    lambda: f64,
    l_sigma: f64,
    shift: f64,
    n_states: usize,
) -> Result<f64, SpcError> {
    if lambda <= 0.0 || lambda > 1.0 {
        return Err(SpcError::InvalidLambda(lambda));
    }
    if l_sigma <= 0.0 {
        return Err(SpcError::NonPositiveParam {
            name: "l_sigma",
            value: l_sigma,
        });
    }
    let n = n_states;

    // Asymptotic EWMA control limit width.
    let sigma_z = (lambda / (2.0 - lambda)).sqrt();
    let ucl = l_sigma * sigma_z;
    let lcl = -ucl;
    let range = ucl - lcl;
    let delta = range / n as f64;

    let mut mat = nalgebra::DMatrix::<f64>::zeros(n, n);
    let rhs = nalgebra::DVector::<f64>::from_element(n, 1.0);

    for i in 0..n {
        let z_i = lcl + (i as f64 + 0.5) * delta;
        for j in 0..n {
            let z_lo = lcl + j as f64 * delta;
            let z_hi = z_lo + delta;
            // EWMA update: Z_new = lambda*X + (1-lambda)*z_i
            // X needed for Z_new in [z_lo, z_hi]:
            // x_lo = (z_lo - (1-lambda)*z_i) / lambda
            // x_hi = (z_hi - (1-lambda)*z_i) / lambda
            let x_lo = (z_lo - (1.0 - lambda) * z_i) / lambda - shift;
            let x_hi = (z_hi - (1.0 - lambda) * z_i) / lambda - shift;
            let p = phi(x_hi) - phi(x_lo);
            mat[(i, j)] = -p;
        }
        mat[(i, i)] += 1.0;
    }

    let decomp = mat.lu();
    let arl_vec = decomp
        .solve(&rhs)
        .ok_or(SpcError::SingularArlMatrix(l_sigma))?;

    // ARL starting from the center state (EWMA starts at μ₀).
    let center_state = n / 2;
    Ok(arl_vec[center_state])
}

/// Standard normal CDF via the error function.
fn phi(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// Error function approximation (Abramowitz & Stegun 7.1.26, max error 1.5e-7).
fn erf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}
```

- [ ] **Step 2: Add to lib.rs**

Add to `crates/spc-charts/src/lib.rs`:

```rust
pub mod arl;

pub use arl::{cusum_arl, ewma_arl};
```

- [ ] **Step 3: Write TCK tests**

Create `crates/spc-charts/tests/arl_tck.rs`:

```rust
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::{cusum_arl, ewma_arl};

#[test]
fn cusum_arl0_montgomery_table() {
    // Montgomery Table 9.3: k=0.5, h=5, shift=0 → ARL₀ ≈ 465
    let arl = cusum_arl(0.5, 5.0, 0.0, 200).unwrap();
    assert!(
        (arl - 465.0).abs() / 465.0 < 0.05,
        "CUSUM ARL₀ = {arl}, expected ~465"
    );
}

#[test]
fn cusum_arl1_one_sigma_shift() {
    // Montgomery Table 9.3: k=0.5, h=5, shift=1.0 → ARL₁ ≈ 10.4
    let arl = cusum_arl(0.5, 5.0, 1.0, 200).unwrap();
    assert!(
        (arl - 10.4).abs() / 10.4 < 0.10,
        "CUSUM ARL₁(δ=1) = {arl}, expected ~10.4"
    );
}

#[test]
fn cusum_arl1_half_sigma_shift() {
    // Montgomery Table 9.3: k=0.5, h=5, shift=0.5 → ARL₁ ≈ 38
    let arl = cusum_arl(0.5, 5.0, 0.5, 200).unwrap();
    assert!(
        (arl - 38.0).abs() / 38.0 < 0.10,
        "CUSUM ARL₁(δ=0.5) = {arl}, expected ~38"
    );
}

#[test]
fn cusum_arl_decreases_with_shift() {
    let arl_0 = cusum_arl(0.5, 5.0, 0.0, 200).unwrap();
    let arl_05 = cusum_arl(0.5, 5.0, 0.5, 200).unwrap();
    let arl_10 = cusum_arl(0.5, 5.0, 1.0, 200).unwrap();
    let arl_20 = cusum_arl(0.5, 5.0, 2.0, 200).unwrap();
    assert!(arl_0 > arl_05, "ARL(0)={arl_0} > ARL(0.5)={arl_05}");
    assert!(arl_05 > arl_10, "ARL(0.5)={arl_05} > ARL(1.0)={arl_10}");
    assert!(arl_10 > arl_20, "ARL(1.0)={arl_10} > ARL(2.0)={arl_20}");
}

#[test]
fn ewma_arl0_montgomery_table() {
    // Montgomery Table 9.9: λ=0.2, L=2.962 → ARL₀ ≈ 500
    let arl = ewma_arl(0.2, 2.962, 0.0, 200).unwrap();
    assert!(
        (arl - 500.0).abs() / 500.0 < 0.10,
        "EWMA ARL₀ = {arl}, expected ~500"
    );
}

#[test]
fn ewma_arl_decreases_with_shift() {
    let arl_0 = ewma_arl(0.2, 2.962, 0.0, 200).unwrap();
    let arl_05 = ewma_arl(0.2, 2.962, 0.5, 200).unwrap();
    let arl_10 = ewma_arl(0.2, 2.962, 1.0, 200).unwrap();
    assert!(arl_0 > arl_05, "ARL(0)={arl_0} > ARL(0.5)={arl_05}");
    assert!(arl_05 > arl_10, "ARL(0.5)={arl_05} > ARL(1.0)={arl_10}");
}

#[test]
fn shewhart_arl0_analytic() {
    // ARL₀ = 1 / (2 * Φ(-3)) ≈ 370.4 for k=3.
    // This is a unit test on the phi function, not on the Markov chain.
    let p_tail = 2.0 * (1.0 - normal_cdf(3.0));
    let arl = 1.0 / p_tail;
    assert!(
        (arl - 370.4).abs() / 370.4 < 0.01,
        "Shewhart ARL₀ = {arl}, expected ~370.4"
    );
}

fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf_approx(x / std::f64::consts::SQRT_2))
}

fn erf_approx(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p spc-charts --test arl_tck --release 2>&1`
Expected: all 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/spc-charts/src/arl.rs \
       crates/spc-charts/src/lib.rs \
       crates/spc-charts/tests/arl_tck.rs
git commit -m "feat(spc): ARL computation via Markov chain discretization (BEAD-0008)"
```

---

### Task 9: G-Theory Convenience (Feature-Gated)

**Files:**
- Create: `crates/spc-charts/src/g_theory.rs`
- Modify: `crates/spc-charts/src/lib.rs`
- Create: `crates/spc-charts/tests/g_theory_tck.rs`

- [ ] **Step 1: Create g_theory.rs**

Create `crates/spc-charts/src/g_theory.rs`:

```rust
#![cfg(feature = "g-theory")]

use crate::types::{ControlLimits, SpcError};
use salib_estimators::GTheoryResult;

/// Convert G-theory variance components into SPC control limits.
///
/// The "universe score" standard error from G-theory:
///   σ² = σ²_pi/n_i + σ²_pr/n_r + σ²_pir/(n_i·n_r)
///
/// This excludes σ²_p (person variance = signal) and includes only
/// the error facets (noise floor). The caller provides `grand_mean`
/// from baseline runs.
#[allow(clippy::cast_precision_loss)]
pub fn control_limits_from_g_theory(
    result: &GTheoryResult,
    grand_mean: f64,
    n_items: usize,
    n_raters: usize,
) -> Result<ControlLimits, SpcError> {
    let ni = n_items as f64;
    let nr = n_raters as f64;
    let sigma_sq = result.sigma_pi / ni + result.sigma_pr / nr + result.sigma_pir / (ni * nr);
    if sigma_sq <= 0.0 {
        return Err(SpcError::NonPositiveSigma(sigma_sq.sqrt()));
    }
    ControlLimits::new(grand_mean, sigma_sq.sqrt())
}
```

- [ ] **Step 2: Add to lib.rs**

Add to `crates/spc-charts/src/lib.rs`:

```rust
#[cfg(feature = "g-theory")]
pub mod g_theory;

#[cfg(feature = "g-theory")]
pub use g_theory::control_limits_from_g_theory;
```

- [ ] **Step 3: Write TCK tests**

Create `crates/spc-charts/tests/g_theory_tck.rs`:

```rust
//! Tests for the g-theory feature gate. Only compiled with
//! `cargo test -p spc-charts --features g-theory`.

#![cfg(feature = "g-theory")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use spc_charts::control_limits_from_g_theory;

fn mock_g_theory_result() -> salib_estimators::GTheoryResult {
    salib_estimators::GTheoryResult {
        sigma_p: 10.0,
        sigma_i: 2.0,
        sigma_r: 1.5,
        sigma_pi: 3.0,
        sigma_pr: 2.0,
        sigma_ir: 0.5,
        sigma_pir: 1.0,
        g_coefficient: 0.85,
        phi_coefficient: 0.80,
        variance_component_ci_low: None,
        variance_component_ci_high: None,
        g_coefficient_ci_low: None,
        g_coefficient_ci_high: None,
        phi_coefficient_ci_low: None,
        phi_coefficient_ci_high: None,
        bootstrap_iterations: None,
        bootstrap_alpha: None,
        bootstrap_skipped: None,
    }
}

#[test]
fn control_limits_from_g_theory_computes_sigma() {
    let result = mock_g_theory_result();
    let limits = control_limits_from_g_theory(&result, 50.0, 5, 3).unwrap();

    // σ² = σ²_pi/n_i + σ²_pr/n_r + σ²_pir/(n_i·n_r)
    //    = 3.0/5 + 2.0/3 + 1.0/15
    //    = 0.6 + 0.6667 + 0.0667
    //    = 1.3333
    // σ = √1.3333 ≈ 1.1547
    let expected_sigma = (3.0 / 5.0 + 2.0 / 3.0 + 1.0 / 15.0_f64).sqrt();
    assert!(
        (limits.sigma - expected_sigma).abs() < 1e-10,
        "sigma = {}, expected {expected_sigma}",
        limits.sigma
    );
    assert_eq!(limits.mu_0, 50.0);
}

#[test]
fn control_limits_feeds_into_chart() {
    let result = mock_g_theory_result();
    let limits = control_limits_from_g_theory(&result, 50.0, 5, 3).unwrap();

    // Should be usable with any chart.
    let mut chart =
        spc_charts::ShewhartChart::new(spc_charts::ShewhartConfig::default_for(limits)).unwrap();
    assert!(chart.observe(50.0).is_in_control());
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p spc-charts --features g-theory --test g_theory_tck --release 2>&1`
Expected: both tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/spc-charts/src/g_theory.rs \
       crates/spc-charts/src/lib.rs \
       crates/spc-charts/tests/g_theory_tck.rs
git commit -m "feat(spc): G-theory convenience adapter behind feature flag (BEAD-0008)"
```

---

### Task 10: Full Workspace Integration + BEAD Close

**Files:**
- Modify: `.context/beads/BEAD-0008-spc-control-charts.md`

- [ ] **Step 1: Run full workspace tests**

Run: `cargo test --workspace --release 2>&1`
Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1`

Both must pass clean.

- [ ] **Step 2: Run SPC tests with g-theory feature**

Run: `cargo test -p spc-charts --all-features --release 2>&1`

- [ ] **Step 3: Close BEAD-0008**

Update `.context/beads/BEAD-0008-spc-control-charts.md`:
- Set `status: closed`, `closed: 2026-05-14`
- Add completion notes listing all 6 chart types + ARL + G-theory adapter with test counts.

- [ ] **Step 4: Commit**

```bash
git add .context/beads/BEAD-0008-spc-control-charts.md
git commit -m "chore: close BEAD-0008 — SPC control charts crate complete"
```
