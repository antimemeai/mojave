# Perturbation Engine: Proposed Future Families

Research report — 2026-05-18

## Inventory of what exists

The current perturbation engine covers three families:

- **Format atoms** (surface text): Separator, Casing, Punctuation, Padding — `signed_confound: false`
- **Paraphrase atoms** (semantic rewording): Model × Strength — `signed_confound: true`
- **Multi-turn atoms** (conversation history): Original, TruncateEarly, Reorder, Inject — `signed_confound: true`

All seeded via ChaCha20Rng, map to flat `factor_<atom>` keys for sensitivity analysis.

---

## Family 1: Positional Bias Atoms

**Dimension:** Position of correct/relevant information within the context window.

**Grounding:** "Lost in the middle" phenomenon (Liu et al., 2023) — LLMs attend disproportionately to beginning/end of context. A model whose score depends on where answer-bearing content appears exhibits a measurement artifact.

| Atom | signed_confound |
|------|-----------------|
| PositionOriginal | false |
| AnswerContextToTop | true |
| AnswerContextToMiddle | true |
| AnswerContextToBottom | true |
| DistractorDensityLow (1-2 irrelevant passages) | true |
| DistractorDensityHigh (5-10 irrelevant passages) | true |

**Measurement theory:** High sensitivity to `factor_answer_position` means the eval measures attention-pattern artifacts, not the construct. Analogous to "method effects" in multitrait-multimethod matrices (Campbell & Fiske, 1959).

**Complexity:** Low-medium. Pure text transform for passage rearrangement. Distractor variants need a pre-generated distractor corpus.

---

## Family 2: Anchoring and Priming Atoms

**Dimension:** Suggestive content placed before the task that could bias the response.

**Grounding:** Jones & Steinhardt (2022) on cognitive biases in LLMs; Shi et al. (2023) on irrelevant context degrading accuracy. Anchoring bias analogous to Tversky & Kahneman (1974).

| Atom | signed_confound |
|------|-----------------|
| AnchorNone | false |
| NumericalAnchorHigh ("95% of experts...") | true |
| NumericalAnchorLow ("5% of experts...") | true |
| AuthorityPrime ("According to Harvard...") | true |
| NegativePrime ("This is extremely difficult...") | true |
| PositivePrime ("This is straightforward...") | true |

**Measurement theory:** Anchoring sensitivity is pure nuisance variance. High Shapley attribution to anchoring atoms means scores are confounded with priming susceptibility.

**Complexity:** Low. Templated text prepends parameterized by seed.

---

## Family 3: Instruction-Following Fidelity Atoms

**Dimension:** Whether the model follows meta-instructions about output format and constraints.

**Grounding:** IFEval (Zhou et al., 2023) — instruction-following is distinct from task completion. For agentic evals, constraint adherence is safety-critical.

| Atom | signed_confound |
|------|-----------------|
| ConstraintOriginal | false |
| FormatConstraintAdded ("Respond in exactly 3 bullet points") | true |
| NegativeConstraint ("Do not mention X") | true |
| LengthConstraint ("Under 25 words") | true |
| ConflictingConstraint | true |

**Measurement theory:** Tests whether the eval measures task-completion or the conjunction of task-completion AND instruction-compliance. Maps to the item-construction principle of specifying stem and key precisely (Haladyna, Downing & Rodriguez, 2002).

**Complexity:** Low-medium. Pure text appends; scoring integration requires constraint-checking post-processor.

---

## Family 4: Tool-Use and API Perturbation Atoms

**Dimension:** The tool/API environment available to an agentic system.

**Grounding:** Kapoor et al.'s Agentic Benchmark Checklist (NeurIPS 2025); Ruan et al. (2023) on tool reliability affecting agentic task completion.

| Atom | signed_confound |
|------|-----------------|
| ToolOriginal | false |
| ToolLatency (2-5s delay) | true |
| ToolTransientFailure (503 on first call) | true |
| ToolSchemaVariation (rename field, add field) | true |
| ToolPartialResult (truncated/paginated) | true |
| ToolRemoved (force alternative path) | true |

**Measurement theory:** In G-theory, the tool environment is a facet. High tool-perturbation sensitivity means poor generalizability — the score doesn't generalize to messy real-world tool environments.

**Complexity:** High. Requires tool-environment proxy (architecturally homologous to format-spread-proxy but operating on tool responses).

---

## Family 5: Numerical and Quantitative Perturbation Atoms

**Dimension:** Sensitivity to specific numerical values, units, and magnitudes.

**Grounding:** Stolfo et al. (2023) — changing numbers in math problems while preserving structure reveals pattern-matching vs. reasoning. Critical for defense/financial applications.

| Atom | signed_confound |
|------|-----------------|
| NumOriginal | false |
| MagnitudeShift (×10, ×0.1 with answer adjustment) | false |
| UnitConversion (meters→feet with answer adjustment) | false |
| PrecisionIncrease ("12" → "12.000") | false |
| IrrelevantNumber (inject irrelevant numerical statement) | true |
| OffByOne (+/- 1 with answer adjustment) | false |

**Measurement theory:** `MagnitudeShift` and `UnitConversion` are "parallel forms" — if scores change, the eval measures surface-pattern sensitivity, not mathematical reasoning. `IrrelevantNumber` probes numerical anchoring.

**Complexity:** Medium. Some atoms need structured numerical annotation in eval inputs.

---

## Family 6: Persona and Register Atoms

**Dimension:** Social register, persona, and communicative style.

**Grounding:** Gupta et al. (2024) on persona-assigned LLM biases; Salewski et al. (2023) on impersonation effects; Sclar et al. (2024, the FormatSpread paper).

| Atom | signed_confound |
|------|-----------------|
| RegisterOriginal | false |
| RegisterFormal | true |
| RegisterCasual | true |
| RegisterTechnical (heavy jargon) | true |
| RegisterNonNative (non-native speaker patterns) | true |
| AuthorityPersona ("You are an expert in X") | true |

**Measurement theory:** Register sensitivity is differential item functioning (DIF) — the item functions differently for different communication styles. DIF analysis (Holland & Wainer, 1993) is core psychometric detection of item bias.

**Complexity:** Medium. Register rewrites require model calls (shares paraphrase-proxy infra). AuthorityPersona is pure text prepend.

---

## Family 7: Temporal and Version Reference Atoms

**Dimension:** Sensitivity to temporal framing, version numbers, and recency cues.

**Grounding:** Targets eval contamination — if changing "Python 3.11" to "Python 3.14" in an identical problem degrades performance, the model is retrieving memorized answers. Connected to contamination literature (Jacovi et al., 2023; Oren et al., 2024).

| Atom | signed_confound |
|------|-----------------|
| TemporalOriginal | false |
| DateShiftFuture ("2024" → "2027") | true |
| DateShiftPast | true |
| VersionBump (increment version numbers) | true |
| RecencyCueAdd ("as of today") | true |
| RecencyCueRemove (strip temporal qualifiers) | false |

**Measurement theory:** Tests whether items measure "ability to reason about X" or "recall of specific formulation of X from training data." Contaminated items exhibit artificially low IRT difficulty parameters.

**Complexity:** Low. Regex-based date/version substitution.

---

## Family 8: Chain-of-Thought and Reasoning Structure Atoms

**Dimension:** Whether performance depends on explicit reasoning scaffolding vs. inherent task difficulty.

**Grounding:** Wei et al. (2022) on CoT prompting; Wang et al. (2023) on self-consistency.

| Atom | signed_confound |
|------|-----------------|
| CoTOriginal | false |
| CoTElicit ("Let's think step by step") | true |
| CoTSuppress ("Answer directly without explanation") | true |
| CoTMisleadingHint (plausible but incorrect reasoning hint) | true |
| CoTDecompose (break into sub-questions) | true |

**Measurement theory:** If CoT elicitation changes model rankings, the eval has a differential-boost problem — formally analogous to treatment-by-aptitude interaction (Cronbach & Snow, 1977). `CoTMisleadingHint` detects sycophancy/authority-following.

**Complexity:** Low-medium. Most are text appends; `CoTDecompose` may need model call.

---

## Family 9: Example and Few-Shot Perturbation Atoms

**Dimension:** Sensitivity to number, quality, and ordering of in-context examples.

**Grounding:** Min et al. (2022) — label correctness matters less than format in few-shot; Lu et al. (2022) — example ordering can flip accuracy from near-zero to near-perfect.

| Atom | signed_confound |
|------|-----------------|
| FewShotOriginal | false |
| FewShotShuffle (reorder examples) | false |
| FewShotRemoveOne | true |
| FewShotAddDistractor (correct label, different category) | true |
| FewShotLabelFlip (flip one label) | true |
| FewShotZeroShot (remove all examples) | true |

**Measurement theory:** `FewShotShuffle` is arguably the single most diagnostic atom possible — if example ordering changes scores, the eval is measuring a prompt-engineering artifact, full stop. In classical test theory, shuffle variance is pure error variance.

**Complexity:** Low for structured inputs.

---

## Family 10: Adversarial Distractor Atoms

**Dimension:** Robustness to plausible-but-wrong distractors and red herrings.

**Grounding:** Psychometric item-construction literature (Haladyna, Downing & Rodriguez, 2002; Rodriguez, 2005); adversarial NLP (Wallace et al., 2019; Jia & Liang, 2017).

| Atom | signed_confound |
|------|-----------------|
| DistractorOriginal | false |
| PlausibleDistractorInject | true |
| NearMissDistractor (differs in one critical detail) | true |
| ConfidenceDistractor (incorrect with high confidence framing) | true |
| PartialTruthDistractor (partially correct but misleading) | true |

**Measurement theory:** Directly operationalizes IRT item discrimination (`a` parameter). Injecting quality distractors and measuring score drop estimates whether eval items have adequate discrimination power.

**Complexity:** Medium-high. Best when distractors are pre-authored per task with psychometric intent.

---

## Family 11: Output Judge Perturbation Atoms

**Dimension:** Sensitivity of the scoring/judgment mechanism itself.

**Grounding:** "Rating Roulette" (EMNLP 2025) — LLM judges have lower within-judge than between-judge consistency; SAGE framework shows ~25% inconsistency in difficult cases; Li et al.'s preference-leakage work shows family-correlated bias.

| Atom | signed_confound |
|------|-----------------|
| JudgeOriginal | false |
| JudgeRubricShuffle (reorder criteria) | false |
| JudgeScaleInversion (reverse scale, invert post-hoc) | false |
| JudgeFamilySwap (different model family as judge) | true |
| JudgePromptParaphrase (paraphrase judge system prompt) | false |
| JudgeTemperatureBump (0 → 0.3) | true |

**Measurement theory:** Directly operationalizes G-theory variance decomposition of `σ²(judge_config)`. `JudgeRubricShuffle` and `JudgeScaleInversion` are semantically equivalent transforms — any variance they introduce is pure measurement error.

**Complexity:** Medium. Requires access to judge configuration.

---

## Summary Matrix

| # | Family | Atoms | Primary measurement question | Complexity |
|---|--------|-------|------------------------------|------------|
| 1 | Positional Bias | 6 | Does position in context affect scores? | Low-Med |
| 2 | Anchoring/Priming | 6 | Is the model anchored by irrelevant content? | Low |
| 3 | Instruction Fidelity | 5 | Task completion or constraint compliance? | Low-Med |
| 4 | Tool/API Perturbation | 6 | Robust to tool-environment degradation? | High |
| 5 | Numerical/Quantitative | 6 | Reasoning or pattern-matching on numbers? | Medium |
| 6 | Persona/Register | 6 | Does communication style affect scores? | Medium |
| 7 | Temporal/Version | 6 | Reasoning or memorized answer retrieval? | Low |
| 8 | Chain-of-Thought | 5 | Capability or scaffolding dependence? | Low-Med |
| 9 | Few-Shot Examples | 6 | How much is example-engineering artifact? | Low |
| 10 | Adversarial Distractors | 5 | Does the eval have adequate discrimination? | Med-High |
| 11 | Judge Perturbation | 6 | How much variance is measurement noise? | Medium |

## Recommended Implementation Priority

**Tier 1 — Highest diagnostic value, lowest cost:**
- Family 9 (Few-Shot) — `FewShotShuffle` is the single most diagnostic atom possible
- Family 7 (Temporal/Version) — contamination detection
- Family 2 (Anchoring/Priming) — templated text transforms

**Tier 2 — High value, medium cost:**
- Family 1 (Positional Bias) — foundational for construct validity
- Family 8 (Chain-of-Thought) — scaffolding dependence
- Family 5 (Numerical/Quantitative) — reasoning vs. pattern-matching
- Family 11 (Judge Perturbation) — measurement instrument noise

**Tier 3 — High value, requires infrastructure:**
- Family 6 (Persona/Register) — shares paraphrase proxy infra
- Family 3 (Instruction Fidelity) — needs constraint-checking
- Family 10 (Adversarial Distractors) — best with pre-authored distractors
- Family 4 (Tool/API) — most complex but uniquely valuable for agentic evals

## What This Does Not Cover

- **Multimodal perturbations** (image/audio) — distinct engineering domain
- **Adversarial attacks** (jailbreaks, prompt injection) — security not measurement
- **Data contamination detection** (canary tokens, membership inference) — own domain
- **Perturbation composition rules** — factorial designs across families deferred

## Key References

- Liu et al. (2023), "Lost in the Middle" — Positional bias
- Shi et al. (2023), "LLMs Can Be Easily Distracted by Irrelevant Context" — Anchoring
- Stolfo et al. (2023), "A Causal Framework to Quantify Robustness of Mathematical Reasoning" — Numerical
- Jones & Steinhardt (2022), "Capturing Failures via Human Cognitive Biases" — Cognitive biases
- Zhou et al. (2023), IFEval — Instruction-following
- Min et al. (2022), "Rethinking the Role of Demonstrations" — Few-shot
- Lu et al. (2022), "Fantastically Ordered Prompts" — Example ordering
- Wei et al. (2022), "Chain-of-Thought Prompting" — CoT scaffolding
- Gupta et al. (2024), "Bias Runs Deep: Persona-Assigned LLMs" — Persona/register
- Haladyna, Downing & Rodriguez (2002), "MC Item-Writing Guidelines" — Distractor design
- Holland & Wainer (1993), "Differential Item Functioning" — DIF analysis
- Campbell & Fiske (1959), "Multitrait-Multimethod Matrix" — Construct validity
- Ruan et al. (2023), "LM-Emulated Sandbox" — Tool-environment
- Wallace et al. (2019), "Universal Adversarial Triggers" — Adversarial robustness
- Jia & Liang (2017), "Adversarial Examples for Reading Comprehension" — Distractor injection
