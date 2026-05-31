# Scout: Adversary -- mojave
Date: 2026-05-30

## Key Findings

### 1. Confidence sequences use estimated sigma, which voids the anytime-valid guarantee

The `AnytimeMonitor` (the online monitor used in the real confseq pipeline via `confseq.rs`) computes sigma from the sample via Welford's algorithm (line 67-74 of `anytime.rs`), then feeds it into the Howard et al. 2021 normal-mixture CS formula. The code's own docstring on `normal_mixture_cs` explicitly warns: "using a sample estimate does not preserve the coverage guarantee." The `known_sigma` variant exists but is not used by `AnytimeMonitor`. The confseq pipeline in `mojave-gsa/src/confseq.rs` constructs an `AnytimeMonitor` with `DataFamily::Bernoulli` and `mixing_variance: 1.0` -- these are mSPRT parameters for the always-valid p-value, but the confidence interval emitted by `update()` still uses estimated sigma. The Gate 4 Monte Carlo test (`gate4_monte_carlo.rs` line 86-128) tests coverage only with `normal_mixture_cs_known_sigma` (sigma=1.0 for N(0,1) data) -- it does not test coverage of the `AnytimeMonitor`'s CI output, which is the code path actually used in production. This is a coverage guarantee gap hiding behind a test that validates a different function. Actionable: add a Gate 4 Monte Carlo coverage test for `AnytimeMonitor::update().confidence_interval` with unknown variance.

### 2. Sobol indices are negative and CIs cross unity -- classical diagnostic of insufficient N or model misspecification

All three WMDP analyses show negative first-order Sobol indices (bio: S1_quantization = -0.070, S1_decoding = -0.006; chem: S1_system_prompt = -0.003). Negative S1 values are estimation artifacts that arise when N is too small relative to model complexity or when the output is not square-integrable. The dominant factor (prompt_template) has S1 CI upper bounds exceeding 1.0 in all three benchmarks (bio: 1.034, chem: 1.039, truthfulqa: 1.055). Sum of total-order indices significantly exceeds 1.0 (bio: 1.295, truthfulqa: 1.068, chem: 1.107), which is possible with interactions but at this magnitude suggests the variance decomposition is not fully converged. Per Zhang et al. 2015 (cited in the plan): reliable Sobol indices require N in [10^2, 10^4]. N=512 with k=6 may be marginal. The plan itself says "if dominant-factor CI exceeds 10% of estimate, double N" -- bio's CI width is 44% of the point estimate. This threshold was exceeded and no doubling was done.

### 3. 20 cells in WMDP-Bio have n_samples=0, feeding accuracy=0.0 into the Sobol estimator

Twenty cells in `bio_results_gsa.json` have `n_samples: 0` and `accuracy: 0.0`. These are not true zero-accuracy outcomes; they are missing data coded as zero. The Sobol estimator treats them as real observations. This biases the variance decomposition: zero-accuracy cells artificially inflate the spread (bio aggregate min=0.0, spread=0.832) and corrupt the f(A), f(B), f(A_Bi) evaluation vectors that the Saltelli 2010 estimator requires. The analysis reports `n_cells: 4096` (expected N*(k+2) = 512*8 = 4096), meaning all cells including the empty ones are fed through. The `analyze()` function requires non-null accuracy (`cell.accuracy.with_context(...)`) but treats 0.0 as valid. There is no guard against n_samples=0. Chem has 1 such cell; TruthfulQA has 0. This is a data quality issue that directly affects the headline results.

### 4. The audit chain provides tamper-evidence but not tamper-prevention, and the threat model has a known gap

The hash chain (SHA-256 chaining, canonical JSON encoding) detects retrospective tampering: if someone alters an entry, the hashes break. Ed25519 signing adds origin attestation. However, FUTURE_WORK.md explicitly states: "Signed envelopes from an unsigned binary is theater." The binary itself is not signed (line 42: "Binary signing -- REQUIRED BEFORE PRODUCTION"). Without binary signing, an adversary with write access to the host can replace the mojave binary with one that produces valid-looking chains with fabricated data. The audit chain proves integrity conditional on trusting the tool -- but the tool is currently unverifiable. For defense customers (the stated first market), this is a critical gap: the audit chain is load-bearing for the trust argument, but the foundation of that chain (the binary) is unsigned. COSE_Sign1 attestation exists in `audit-sign` but the attestation only covers individual entries, not the binary producing them.

### 5. The core finding ("prompt template dominates") may be an artifact of how perturbation levels were chosen

Prompt_template has 5 levels including the pathological "bare" template (no instructions, just the question). If "bare" produces near-zero accuracy (as the min_accuracy=0.0 data suggests), then prompt_template's dominance in the Sobol decomposition (S1 ~0.85-0.93) is largely driven by one extreme level. This is a construct validity question: the finding "prompt template explains 85% of variance" is technically correct but potentially misleading if it means "including a broken prompt causes bad results." The perturbation design treats all levels as equally interesting, but the Saltelli design's uniform sampling means the extreme "bare" level gets the same weight as realistic production configurations. A practitioner hearing "prompt template is the dominant factor" might conclude prompt engineering is the key lever, when the real finding may be "don't use a completely bare prompt." This conflation of sensitivity (what drives variance) with actionability (what you should tune) is a known limitation of GSA when applied outside engineering domains.

### 6. Confidence sequence early-stopping for binary accuracy data uses the wrong distributional family

The `AnytimeMonitor` in `confseq.rs` uses a Gaussian mSPRT (`gaussian_msprt_log_lr`) for data that is actually Bernoulli (binary correct/incorrect per item). The Gaussian mSPRT assumes known variance sigma^2=1.0 and is designed for continuous data. Binary accuracy data has variance p(1-p), which depends on the unknown parameter being estimated. The confseq pipeline shuffles item-level binary outcomes and monitors CI width -- but the CI formula assumes sub-Gaussian data with known variance. For accuracy near 0 or 1 (many cells show accuracy <0.1 or >0.8), the Gaussian approximation is poor. The `BernoulliMonitor` exists in the codebase (using Beta-mixture mSPRT from Johari et al. 2022), and the plan explicitly recommends "betting-based confidence sequence for every cell where the score is a bounded mean" (Waudby-Smith & Ramdas 2024). The current implementation does not use either -- it uses the Gaussian path. The plan also cites Howard et al.'s confseq library and says "do not implement uniform boundaries from scratch," but the implementation does exactly that.

### 7. salib-rs is an ambitious strategic bet with single-developer bus-factor risk

salib-rs (published as `salib` 0.1.1 on crates.io) is positioned as a strict superset of Python SALib. The CLAUDE.md declares this "NON-NEGOTIABLE." This means: (a) every bug in salib-rs is a mojave-only bug that the SALib community will not help find; (b) every new SA method in the literature must be implemented from scratch rather than consumed; (c) the entire statistical foundation of the product depends on one person's implementation of complex estimators (Saltelli 2010, Jansen 1999, Borgonovo delta, PCE, HDMR, etc.). The 4-gate validation is thorough when exercised, but Gate 2 (R cross-checks) relies on fixture files that may not exist -- `gate2_r_crosscheck.rs` silently skips if fixtures are missing ("SKIP: OBF fixtures not found"). If the R fixture generation script (`scripts/r-fixtures/`) is not run regularly, Gate 2 is theater for seq-anytime-valid, and potentially for other crates too.

### 8. The Rust/Python boundary creates a trust-but-don't-verify gap for the Python calibration layer

The architecture cleanly separates Rust (correctness) from Python (offline calibration). But the Python layer wraps py-irt (Bayesian IRT via Pyro), deepirtools (IWAVE), and semopy (SEM/CFA) -- none of which are under mojave's control. These libraries produce item pool parameters, factor structures, and CFA models that the Rust engine trusts as inputs. The 4-gate validation methodology applies to the Rust crates. The Python tests (31 total) are thinner: `test_irt.py`, `test_factors.py`, `test_cfa.py`, `test_schema.py`, `test_cli.py`. If py-irt produces miscalibrated item parameters, the Rust CAT engine (`eval-design`) will select items based on wrong discrimination/difficulty values. The JSON boundary is clean architecturally but it means Rust has no way to verify that the Python-produced parameters are statistically correct -- it trusts the JSON.

### 9. Scaling: N(k+2) Saltelli design cost is quadratic in axes, and multi-model plans compound it

Current WMDP runs: 3 benchmarks x 4096 cells = 12,288 GPU evaluations on one 7B model. The v2 plan targets 3 models (7B + 72B + DeepSeek V4 Pro) across 5 benchmarks. At N=1024, k=6: 8,192 cells per (model, benchmark). With 3 models and 5 benchmarks: 8,192 x 15 = 122,880 cells. At ~$0.30/hr per pod and assuming ~30 seconds per cell for a 7B model, this is manageable. But for 72B models, inference is ~10x slower and requires H100s (~$2-3/hr). The 72B cost estimate: 122,880 / 3 models * 1 (just 72B) * 10x slowdown = ~400,000 pod-seconds = ~111 pod-hours at ~$2.50/hr = ~$278 for the 72B model alone. Feasible, but if N needs doubling (per the plan's own threshold, which was already exceeded), costs double. Adding more axes (the plan mentions "agentic-harness variance decomposition" with scaffold as a factor) increases k, making the (k+2) multiplier worse. The cost function is O(N * k) per (model, benchmark), and the plan is adding in both dimensions.

### 10. WMDP as measurement target creates regulatory and ethical positioning risk

The project explicitly targets WMDP (Weapons of Mass Destruction Proxy benchmark) as its first empirical showcase, and explicitly links to NIST CAISI regulatory use ("NIST CAISI cited WMDP-style hazardous-capability batteries in its Sept 2025 DeepSeek and Dec 2025 Kimi K2 evaluations"). This creates a double-edged positioning: (a) demonstrating that WMDP scores are highly sensitive to prompt template (the core finding) undermines confidence in WMDP-based regulatory decisions, which could make mojave politically useful to regulators but threatening to labs whose compliance depends on WMDP scores; (b) the project must handle item-level WMDP data carefully -- the plan says "aggregate findings open, item-level data dataset-hygiene" -- but the Sobol decomposition requires running every item under every perturbation combination, which means mojave has demonstrated exactly which prompt configurations cause a model to answer hazardous knowledge questions correctly vs. incorrectly. This is dual-use information that must be handled with care.

## Questions for Deep Investigation

1. **Coverage calibration of AnytimeMonitor CI**: Run a Gate 4 Monte Carlo test of `AnytimeMonitor::update().confidence_interval` with Bernoulli(p) data. What is the empirical coverage? Does it hold the 1-alpha guarantee?

2. **Impact of n_samples=0 cells on Sobol indices**: Rerun the bio analysis excluding the 20 zero-sample cells. How much do S1 and ST change? Does the prompt_template dominance hold or is it inflated by the extreme zeros?

3. **"Bare" prompt as leverage point**: What fraction of the prompt_template variance is attributable to the "bare" level alone? Run a leave-one-level-out analysis.

4. **R cross-check fixture freshness**: When were the R fixtures last generated? Are they present in the CI environment? How many Gate 2 tests are silently skipping?

5. **Bernoulli vs Gaussian mSPRT**: What is the Type I error rate difference between the current Gaussian mSPRT and the correct Bernoulli mSPRT for binary accuracy data at p=0.2 (the low end of WMDP accuracies)?

6. **Key custody for defense customers**: How will Ed25519 signing keys be managed in a defense deployment? The current `KeyRef::Env` pattern puts the key in an environment variable. Is this acceptable under NIST 800-171 / CMMC?

## Gaps Identified

1. **No test of the production CI code path** -- Gate 4 tests validate `normal_mixture_cs_known_sigma` but `AnytimeMonitor` uses estimated sigma.
2. **No data quality gate before Sobol analysis** -- cells with n_samples=0 are treated as accuracy=0.0.
3. **No sensitivity analysis of the sensitivity analysis** -- no leave-one-level-out or influence analysis to check whether findings are driven by outlier levels.
4. **Binary signing not implemented** -- required before defense customer deployment per FUTURE_WORK.md.
5. **R cross-check fixtures may be stale or absent** -- Gate 2 tests skip silently.
6. **No Bernoulli-specific confidence sequence in the production pipeline** -- uses Gaussian approximation for binary data.
7. **No documentation of how salib-rs estimator implementations were independently verified** -- the 4-gate strategy is described but the actual Gate 1/Gate 2 evidence (specific paper tables, R package versions, frozen CSVs) is not systematically cataloged in the repository.

## Leads

1. **Waudby-Smith & Ramdas 2024 betting CS** -- the enter_mojave_plan explicitly recommends this for [0,1]-bounded scores. It is cited but not implemented. This would fix Finding 6.
2. **Saltelli 2010 Eq (d) S2 indices** -- second-order indices are implemented (BEAD-0016) but not used in the WMDP analysis. They would quantify the prompt_template x quantization interaction that sum_ST > 1.0 suggests.
3. **confseq library** (Howard et al.) -- cited in the plan as "use this library; do not implement uniform boundaries from scratch." The current implementation does implement from scratch. Consider wrapping confseq via FFI or reimplementing with explicit verification.
4. **Partial replication of Sclar et al. 2024** (ICLR prompt sensitivity) -- their methodology and findings directly validate or conflict with mojave's WMDP results. A direct comparison would strengthen the paper.
5. **NIST AI 600-1 (Jan 2025) alignment** -- if defense is the first market, alignment with NIST's AI Risk Management Framework would be more actionable than general measurement-science framing.

## Acquisitions

None. This is an adversary assessment, not a shopping run.
