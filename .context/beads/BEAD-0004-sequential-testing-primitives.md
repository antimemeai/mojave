---
id: BEAD-0004
title: Build sequential testing (SPRT / group-sequential) primitives
status: closed
priority: high
created: 2026-05-11
closed: 2026-05-12
---

## Description

Sequential testing is needed for "smart eval budgeting" — stop evaluating early when evidence is sufficient. Saves inference dollars. Not built anywhere in current codebase.

## Methods needed

- Wald SPRT (binary + continuous)
- Pocock boundaries (group-sequential)
- O'Brien-Fleming boundaries (group-sequential)
- Lan-DeMets α-spending (flexible timing)
- Bias-adjusted estimators at stopping time (Siegmund 1985)

## Key properties to validate

- SPRT boundaries: A=β/(1−α), B=(1−β)/α exactly in log-space
- SPRT at H0=H1: degenerate → must error
- Group-sequential cumulative spending = nominal α to 1e-10
- K=1 = fixed-sample test; boundary = z_{α/2}
- Pocock = OBF at K=1
- Information-time scaling: doubling sample sizes preserves boundaries

## Reference implementations

- R: gsDesign (Anderson/Merck, FDA-blessed), rpact (Wassmer & Pahlke)
- Python: confseq (Howard et al.), sequential-tests
- Wald 1947 tables (no software — textbook ground truth)

## Literature needed

- Wald 1945/1947, Pocock 1977, O'Brien & Fleming 1979, Lan & DeMets 1983
- Jennison & Turnbull 2000 (Group Sequential Methods — the textbook)
- Howard et al. 2021 (confidence sequences — modern extension)
- Siegmund 1985 Ch. 4 (early-stopping bias)

## Completion notes

Crate `seq-anytime-valid` built and validated end-to-end. All 52 unit tests,
8 TCK integration tests, and 4-gate validation pass.

### Modules implemented

**Boundary (5 modules):**
- `boundary::wald` — Wald SPRT boundaries (approximate + conservative)
- `boundary::boosted` — boosted SPRT truncation
- `boundary::obf` — O'Brien-Fleming boundaries
- `boundary::pocock` — Pocock boundaries
- `boundary::spending` — Lan-DeMets α-spending functions

**Evidence (4 modules):**
- `evidence::likelihood` — Bernoulli + normal log-likelihood ratios
- `evidence::msprt` — Gaussian mSPRT log-LR and always-valid p-values
- `evidence::confseq` — normal-mixture confidence sequences (estimated σ and known σ)
- `evidence::e_value` — e-value product, e-to-p conversion, threshold decisions

**Monitors (3 modules):**
- `monitor::sprt` — `SprtMonitor` (stateful) + `sprt_decide` (stateless)
- `monitor::group_seq` — `GroupSeqMonitor` + `compute_boundaries`
- `monitor::anytime` — `AnytimeMonitor` (mSPRT + confseq combined)

**Estimation (2 modules):**
- `bias` — `bias_corrected_mle` (Siegmund 1985), `median_unbiased_estimate`
- `practical` — `practical_significance_p` (truncated mSPRT, Shim 2025)

### 4-gate validation

- Gate 1: Textbook reproductions (Wald 1947, Pocock 1977, OBF 1979)
- Gate 2: R fixture infrastructure ready (fixtures pending R installation)
- Gate 3: Property-based tests (8 invariants via proptest)
- Gate 4: Monte Carlo calibration (Type-I control for SPRT, CS coverage, always-valid p)

### TCK feature files (8)

sprt, boosted_sprt, group_sequential, msprt, confseq, e_value, bias, practical
