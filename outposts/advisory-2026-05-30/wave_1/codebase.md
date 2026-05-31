# Scout: Codebase -- mojave
Date: 2026-05-30

## Key Findings

1. **The core product is a measurement engine with five mathematical pillars, each implemented from textbook formulas in Rust.** The pillars are: (a) Inter-Rater Reliability (IRR) -- Cohen, Fleiss, Krippendorff, Gwet, Dawid-Skene, Bland-Altman, preference leakage; (b) Sequential/Anytime-Valid Testing -- mSPRT, e-values, confidence sequences (Howard et al. 2021), group sequential boundaries; (c) Statistical Process Control (SPC) -- Shewhart, CUSUM, EWMA, FIR-CUSUM, e-detector (Shin et al.); (d) Computerized Adaptive Testing (CAT) -- 2PL IRT, MLE/EAP ability estimation, MaxInfo/Minimax item selection; (e) Global Sensitivity Analysis -- Saltelli/Sobol', Borgonovo delta, via the external `salib-rs` crate. Total: ~29,800 lines of Rust across 17 workspace crates, with 50 integration test files and 77 Gherkin TCK feature specs.

2. **The audit chain is a custom tamper-evident provenance ledger, not blockchain.** It uses SHA-256 hash chains with a domain-tagged construction: `SHA256("mojave-audit-v1\0" || canonical_json(entry) || parent_hash)`. The canonical JSON encoder is hand-written (sorted keys, no whitespace, floats rejected, integer-only numbers) -- this is a load-bearing invariant. A genesis sentinel entry binds the chain to a `ModelIdentity` (name, provider, quantization, hash). The Python audit writer (`scripts/audit.py`) reimplements this same canonical encoding and hash construction, creating a cross-language compatibility contract that could break silently.

3. **The eval-orchestrator is the integration brain that auto-detects which measurement instruments to apply.** It uses simple heuristics: multiple judges -> IRR, >=10 repeated observations -> Sequential, >=2 distinct run IDs -> SPC. The `Monitor` struct is the streaming variant -- it serializes/deserializes via serde but replays all observations through fresh chart/monitor instances on each push (O(n) per observation). This replay-on-push design is explicitly acknowledged in comments as "acceptable for eval workloads" but will break at scale. The monitor accumulates unbounded `Vec<f64>` for sequential observations and chart observations per series, with no compaction or windowing.

4. **The perturbation engine + eval-design crate implement anti-gaming via randomized perturbation schedules.** Each eval run gets a ChaCha20-seeded assignment of items to perturbation families (format, paraphrase, multi-turn) with a control group. The perturbation_schedule module generates coverage reports across runs. The CAT module implements game-theoretic unpredictability via top-k randomization in item selection (select top-k by Fisher information, then shuffle). This is a distinctive design choice -- most CAT implementations pick the single best item. The perturbation engine has three families (format atoms, paraphrase atoms, multi-turn atoms) but the paraphrase and multi-turn modules are stub-level -- `signed_confound()` always returns `true`.

5. **The mojave-gsa binary is the Saltelli study driver and is the most production-exercised component.** Recent commits (PRs #15-17) show a complete 6-factor Saltelli perturbation study: prompt_template (5 levels), system_prompt (4), n_shot_frac (4), choice_order (2), decoding (3), quantization (2). The analysis pipeline computes first-order (Saltelli 2010 Eq c) and total-order (Jansen 1999 Eq f) Sobol' indices with bootstrap CIs, plus Borgonovo delta indices. It uses `tree_sum` from salib-core for numerically stable summation. The confseq module does retrospective CI-width stopping analysis using the seq-anytime-valid crate's permutation infrastructure.

6. **The change-attribution crate implements git-bisect-for-eval-regressions and blast-radius prediction.** `BisectState` does binary search over commits with an auto_bisect function that takes an eval function and threshold. `BlastRadiusPrediction` maps changed files to historically-regressed tasks. However, the `has_file_overlap` function is a stub -- `entry_touches_similar_paths` always returns `!changed_files.is_empty()`, meaning every history entry matches any non-empty change set. This makes the current blast-radius predictor useless for real change-level attribution.

7. **The Python eval pipeline (`scripts/v2/`) is built on UK AI Safety Institute's Inspect framework.** The MCQ task wrapper (`mcq_task.py`) applies all 6 perturbation axes to MCQ benchmarks (WMDP bio/chem/cyber). It uses Inspect's `multiple_choice` solver with 5 prompt templates and 4 system prompts. The n-shot exemplar pool is seed-pinned with nested levels. The runner (`run_mcq.py`) uses work-stealing across RunPod GPU pods serving Qwen2.5-7B-Instruct via vLLM. The pipeline generates LaTeX run cards with Sobol'-aware sensitivity decomposition sections.

8. **The 4-gate validation methodology is deeply embedded in the test suite.** Gate 1 (textbook reproductions): golden canonical tests, Krippendorff textbook examples. Gate 2 (reference cross-checks): `gate2_r_crosscheck.rs` tests sequential boundaries against R `gsDesign` package. Gate 3 (property-based): `gate3_properties.rs` tests invariants like monotonicity of e-values. Gate 4 (Monte Carlo calibration): `gate4_monte_carlo.rs`, `gate4_bootstrap_calibration.rs`, `gate4_prevalence_sweep.rs`, `gate4_gwet_kappa_paradox.rs`. The IRR crate has 5 Gate-4 tests checking calibration across parameter sweeps -- this is extremely unusual rigor for a startup codebase.

9. **The audit-sign crate uses Ed25519 (PKCS#8 DER/PEM) for attestation signing.** The `AuditSigner` trait abstracts key management with `KeyRef` supporting in-memory, file, and environment variable sources. Attestations are written as CBOR files per sequence number. The signing is optional (signer can be `None`) which means the default deployment path has no cryptographic guarantees -- the hash chain provides tamper evidence but not authenticity.

10. **Workspace-level lint configuration enforces `deny(clippy::unwrap_used, clippy::expect_used)` with test-only exemptions.** Combined with `#![forbid(unsafe_code)]` on most crates, this produces a codebase with zero unsafe blocks and zero unwrap/expect in production paths. All error handling goes through `thiserror` with `#[non_exhaustive]` enums. This is unusually disciplined for a research-stage project and creates a high bar for contributor code.

11. **The dependency on salib-rs is structural and non-negotiable per CLAUDE.md.** mojave-gsa depends on `salib-core`, `salib-samplers`, and `salib-estimators` at version 0.1 from crates.io. The salib crate lives at `../salib/` as a sibling repo. This means any GSA capability gap blocks mojave feature work. The spc-charts crate has an optional `g-theory` feature that depends on `salib-estimators::GTheoryResult`, creating a bidirectional coupling between the measurement engine and the GSA library.

12. **The project is on branch `test/neurotic_library` with HEAD detached.** The most recent commit is a merge of PR #17 (genesis sentinel feature). The commit history shows rapid, disciplined development: genesis sentinel was implemented across 10 commits touching 7 crates with a proper plan/spec/implement/review cycle. The commits are atomic and descriptive (e.g., `feat(audit-chain): SealedAuditEntry enum with genesis sentinel`).

## Questions for Deep Investigation

1. **Python-Rust canonical JSON parity**: The Python `audit.py` reimplements canonical JSON encoding. Has cross-language hash agreement ever been verified with a shared test vector suite? One difference: the Python writer uses `GENESIS_SENTINEL = bytes(32)` (all zeros) for the first entry's parent hash, but the Rust `ChainHead::new()` creates a proper Genesis variant with no parent hash at all. The Python chain uses a flat `{"base": ..., "parent_hash": ..., "entry_hash": ...}` structure while Rust uses `{"type": "Genesis", ...}` or `{"type": "Chained", ...}`. Are these chains actually mutually verifiable?

2. **Monitor replay cost**: The streaming `Monitor` replays ALL observations through fresh chart/monitor instances on every push. For a 10,000-observation eval, that is O(n^2) total work. At what observation count does this become a practical bottleneck? Is there a plan to checkpoint chart state?

3. **Perturbation engine completeness**: The paraphrase and multi-turn perturbation families are stubs. The format family has atoms (walk/atoms) but no actual transformation logic -- it defines what to perturb, not how. Where does the actual perturbation execution happen? Is it done externally by the Python pipeline?

4. **CAT session game-theoretic claims**: The minimax selection claims "game-theoretic unpredictability" via top-k shuffling, but the shuffle uses a ChaCha20 RNG seeded from the session config. If the seed is known (and it is, since configs are pre-registered), an adversary could predict the selection. Is this intentional (audit trail) or an oversight?

5. **Blast radius stub**: The `has_file_overlap` and `entry_touches_similar_paths` functions in `blast_radius.rs` are placeholders. What is the planned implementation? Git diff parsing? AST-level dependency tracing?

6. **spc-charts g-theory bridge**: The g-theory module converts G-theory variance components to SPC control limits. This is a novel connection (using measurement generalizability to set control chart parameters). Has this been validated against any reference? The formula `sigma^2 = sigma_pi/n_i + sigma_pr/n_r + sigma_pir/(n_i*n_r)` is the standard absolute SE formula from Brennan's G-theory -- correct but only valid for balanced designs.

7. **Eval-orchestrator instrument detection**: The heuristic thresholds (>=10 for sequential, >=2 runs for SPC) are hard-coded. Should these be configurable? The current defaults mean a 9-observation eval skips sequential testing entirely.

## Gaps Identified

1. **No IRT calibration crate**: The eval-design crate has 2PL ability estimation (MLE + EAP) and Fisher information, but no item parameter estimation (no MMLE, no EM, no MCMC for item calibration). Item difficulty and discrimination must be provided externally. This means mojave cannot self-calibrate item pools from response data -- it can only consume pre-calibrated parameters.

2. **No differential item functioning (DIF)**: There is no mechanism to detect whether items behave differently across model families (which is the eval analog of demographic DIF). This is critical for cross-model comparisons in the defense market.

3. **No visualization or reporting layer in Rust**: All reporting goes through the Python pipeline to LaTeX. The mojave CLI outputs JSON to stdout. There is no built-in way to generate human-readable summaries, plots, or dashboards from the Rust side.

4. **No network/distributed architecture**: The monitor is in-process only. There is no gRPC/REST API, no message queue integration, no distributed chain consensus. The file-lock-based concurrency (`fs2::FileExt::lock_exclusive`) is single-machine only.

5. **No adversarial robustness testing for the perturbation engine**: The perturbation schedule generator creates random assignments, but there is no analysis of whether the schedule is actually robust against an adversary who can observe some perturbation-response pairs and infer the pattern.

6. **Missing TCK-to-test automation for several crates**: change-attribution, perturbation-engine, eval-design, and audit-emit have no TCK feature files. The metric-tck-harness exists but coverage appears limited to the math crates.

7. **No confidence intervals on IRR statistics**: Cohen's kappa, Fleiss' kappa, and Krippendorff's alpha all return `ci_lower: None, ci_upper: None`. The bootstrap module exists but is not wired into the main IRR statistics. This means the orchestrator's IRR decisions are based on point estimates without uncertainty quantification.

## Leads

1. **Rasch/IRT item calibration**: Linacre (1994) joint MLE, de Ayala (2009) textbook for MMLE-EM. `mirt` R package as reference implementation. Would close the self-calibration gap.

2. **DIF detection methods**: Lord (1980) chi-square, Mantel-Haenszel, logistic regression DIF. The `difR` R package has reference implementations. Directly relevant to cross-model fairness analysis.

3. **Game-theoretic evaluation design**: The BEAD-0010 file references this. Abernethy et al. (2011) "A New Approach to Item Pool Stratification"; Vovk et al. (2005) "Algorithmic Learning in a Random World" for e-value theory underlying the sequential testing.

4. **E-detector for change detection**: Shin et al. (2022) "E-detectors: A Nonparametric Framework for Online Changepoint Detection" -- already implemented in spc-charts. The growing window variant is implemented but the theoretical guarantees (false alarm rate control) should be validated against Shin's Theorem 1.

5. **G-theory for LLM evaluation**: Shavelson & Webb (1991) "Generalizability Theory: A Primer". The bridge from G-theory to SPC limits is novel and worth a methods paper. Also relevant: Brennan (2001) "Generalizability Theory" for unbalanced designs.

6. **Preference leakage**: Li et al. (2025) "Preference Leakage" (ICLR 2026) -- already implemented. The implementation follows equations 5-6 faithfully. Could be extended with bootstrap CIs for the PLS scores.

7. **salib-rs as standalone contribution**: The 77 TCK feature files for salib alone represent a significant quality benchmark for GSA software. A methods paper comparing salib-rs against Python SALib on the Ishigami/Sobol-G test functions with bit-deterministic reproduction would be publishable.

8. **Anytime-valid inference**: Grunwald et al. (2024) "Safe Testing"; Ramdas et al. (2023) survey on e-values and anytime-valid methods. The implementation uses Howard et al. (2021) stitched boundaries -- should verify against the `confseq` R/Python package.

## Acquisitions

(No papers downloaded during this scout -- all referenced papers are known titles for targeted acquisition in wave 2.)
