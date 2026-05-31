# Library Outpost Update -- mojave

**Date:** 2026-05-30
**Advisory:** advisory-2026-05-30

Recommended update to the neurotic_library outpost file for the mojave project.

---

## New Recommendations for Library Acquisition

### Critical gaps discovered during advisory

These gaps were not previously identified in the mojave outpost and are now understood to be load-bearing:

1. **Generalizability Theory (zero coverage).** mojave's variance decomposition is a reinvention of G-theory applied to AI evaluation. The library has zero G-theory papers. This is the most significant gap.
   - Brennan (2001) *Generalizability Theory* -- canonical textbook, HIGH
   - Shavelson & Webb (1991) *Generalizability Theory: A Primer* -- accessible intro, HIGH
   - Webb, Shavelson & Harding (2006) "Reliability coefficients and generalizability theory" -- chapter, MEDIUM

2. **GSA textbook coverage.** 17+ GSA papers but no primary textbook or survey.
   - Saltelli et al. (2008) *Global Sensitivity Analysis: The Primer* -- HIGH
   - Iooss & Lemaitre (2015) "A review on global sensitivity analysis methods" -- HIGH

3. **Nuclear QMU literature.** mojave is structurally isomorphic to QMU but the library has no QMU papers.
   - Pilch et al. (2006) SAND2006-5001 -- acquired, in intake
   - National Academies (2009) QMU evaluation -- free from NAP, HIGH
   - Sharp & Wood-Schultz (2003) Los Alamos Science 28 -- free from LANL, HIGH
   - Eardley et al. (2005) JASON JSR-04-330 -- free from OSTI, HIGH

4. **Measurement System Analysis.** mojave implements IRR but not gauge discrimination.
   - AIAG MSA Reference Manual 4th ed (2010) -- MEDIUM, may need ILL

5. **AI measurement theory.** Zero coverage of the only book on the topic.
   - Hernandez-Orallo (2017) *The Measure of All Minds* -- MEDIUM

6. **MTMM / construct validity operationalization.**
   - Campbell & Fiske (1959) MTMM matrix -- MEDIUM
   - Already have Cronbach-Meehl (1955) and Borsboom (2004)

7. **IRR source papers.** mojave implements Fleiss kappa and Krippendorff alpha but the library lacks the original papers.
   - Fleiss (1971) -- MEDIUM
   - Krippendorff (2011) -- MEDIUM

8. **Morris screening and PAWN.** Implemented in salib-rs, no source papers.
   - Morris (1991) -- MEDIUM
   - Pianosi et al. (2015) -- MEDIUM

### Fulfilled requests from this advisory

The following papers were acquired during the advisory and are now in intake:

| Paper | Status | Fulfilled by |
|-------|--------|-------------|
| Pilch2006 QMU White Paper | ACQUIRED | X-Factor Scout |
| Takeshita2026 Bootstrap ISO 5725 | ACQUIRED | X-Factor Scout |
| Mari2005 Foundations of Measurement | ACQUIRED | X-Factor Scout |
| Keller2026 NIST AI 800-3 | ACQUIRED | Web Scout |
| Wang2025 Measuring Noises of LLM Evals | ACQUIRED | Web Scout |
| Rabanser2026 Science of Agent Reliability | ACQUIRED | Web Scout |
| Kao2025 Constant-Size Crypto Evidence | ACQUIRED | Web Scout |
| Kao2025 Post-Quantum Audit Evidence | ACQUIRED | Web Scout |
| Mazo2024 New Paradigm for GSA | ACQUIRED | Web Scout |
| Koning2025 Anytime Validity is Free | ACQUIRED | Web Scout |
| Chen2026 Efficient Inference Noisy Judge | ACQUIRED | Web Scout |
| BHI2026 Benchmark Health Index | ACQUIRED | Web Scout |
| Mitra2026 Spark-LLM-Eval | ACQUIRED | Web Scout |
| SafetyBenchBenchmark2026 | ACQUIRED | Web Scout |
| Ndzomga2026 Efficient Benchmarking Agents | ACQUIRED | Web Scout |

---

## Notes on Library Strengths Identified

The advisory confirmed that the neurotic_library has exceptional coverage in several areas directly serving mojave:

- **Confidence sequences / SAVI:** Near-complete. Ramdas 2022, 2023, 2025; Howard 2020, 2021; Koning 2026; Waudby-Smith 2024; Shin 2023; Vovk/Wang 2021; Gruenwald 2019; Wald 1945.
- **Psychometrics / IRT:** Deep. Cronbach-Meehl through Borsboom through Freiesleben 2026. Lalor 2022 (py-irt), Burkner 2021, Chalmers 2012 (mirt).
- **LLM-as-Judge:** Comprehensive. 12+ papers including bias, sensitivity, self-preference, position bias.
- **Data contamination / WMDP:** Strong. WMDP original, 10+ contamination papers, unlearning collection.
- **Metrology / GUM / VIM:** Complete official suite plus Possolo and Taylor/Kuyatt supplements.
- **Benchmark critique:** Coherent collection supporting mojave's thesis (Bowman 2021, Hidden Measurement Error 2026, Madaan 2024, etc.).
- **GSA papers:** 17+ covering Saltelli, Sobol, Owen, Borgonovo, Janon, Plischke, RBD-FAST, PCE.

### Field gaps the library cannot fill (because the literature doesn't exist)

The advisory confirmed these genuine gaps in the published literature:
- No published work applies Sobol variance decomposition to LLM evaluation pipelines (mojave is first)
- No framework combines GSA + audit chains + confidence sequences
- No application of QMU outside nuclear/waste domains
- No paper on measurement uncertainty of MCQ evaluations using GUM methodology
- No application of G-theory to LLM evaluation

---

## What the Library Should Acquire Next (beyond this advisory)

Looking ahead to mojave's medium-term roadmap:

1. **Equating / test linking:** If CAT is deployed and items don't fit Rasch, equating becomes necessary.
   - Battauz (2015) equateIRT -- check if already in library
   - Kolen & Brennan (2014) *Test Equating, Scaling, and Linking* textbook

2. **Differential Item Functioning (DIF):** Critical for cross-model fairness analysis.
   - Lord (1980) chi-square DIF
   - Holland & Wainer (1993) *Differential Item Functioning*
   - FairDIF (2026, Springer AI and Ethics)

3. **Mechanism design for evaluation:** If BEAD-0010 (game-theoretic eval design) proceeds.
   - Myerson (1981) mechanism design
   - Incentive-compatible testing literature

4. **Post-quantum cryptography:** If audit chain longevity exceeds 10 years (defense contracts).
   - NIST PQC standardization documents (FIPS 203, 204, 205)

5. **Interlaboratory standards:** If ISO 5725 reporting is implemented.
   - ISO 5725:1994 Parts 1-6
   - ISO 17025:2017 (testing laboratory accreditation)
