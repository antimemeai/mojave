# ASU Acquisition List -- mojave Advisory 2026-05-30

Consolidated, deduplicated list of all paywalled or access-restricted papers identified across all 11 advisory reports. Patrick has ASU library access (pbeam@asu.edu).

---

## Tier 1: HIGH -- Directly load-bearing for current work

| # | Citation | Where to look | Identified by | Notes |
|---|----------|---------------|---------------|-------|
| 1 | Brennan, R. L. (2001). *Generalizability Theory*. New York: Springer-Verlag. | ASU library Springer catalog | Library Scout, Measurement Theory, Gauge R&R | Zero G-theory coverage in library. Canonical reference for variance decomposition, D-study, unbalanced designs. Highest single acquisition priority. |
| 2 | Shavelson, R. J. & Webb, N. M. (1991). *Generalizability Theory: A Primer*. Sage. | ASU library | Library Scout, Measurement Theory, Gauge R&R | Accessible intro to G-theory. D-study budget optimization. |
| 3 | Saltelli, A. et al. (2008). *Global Sensitivity Analysis: The Primer*. Wiley. | ASU library Wiley catalog | Library Scout | The standard GSA textbook. Not in library despite 17+ GSA papers. |
| 4 | National Academies (2009). "Evaluation of QMU Methodology for Assessing and Certifying the Reliability of the Nuclear Stockpile." ISBN 978-0-309-12094-8. | **Free PDF from NAP:** nap.nationalacademies.org/catalog/12531 | QMU Defense Framework | Free download with NAP account. CR threshold guidance (2:1 to 10:1). |
| 5 | Sharp, D. H. & Wood-Schultz, M. M. (2003). "QMU and the Nuclear Weapons Stockpile." *Los Alamos Science* No. 28. | **Free from LANL website** | QMU Defense Framework | Original confidence ratio definition. Accessible QMU introduction. |
| 6 | Eardley, D. et al. (2005). "Quantification of Margins and Uncertainties." JASON Report JSR-04-330. | OSTI.gov or FAS.org | QMU Defense Framework | Independent JASON advisory panel review of QMU. |
| 7 | Waudby-Smith, I. & Ramdas, A. (2024). "Estimating means of bounded random variables by betting." *Annals of Statistics*. | ASU library / already in library (confirm full text) | Statistical Correctness, Adversary | The correct CS for mojave's MCQ data. Need full text for hedged capital process implementation details. |
| 8 | Iooss, B. & Lemaitre, P. (2015). "A review on global sensitivity analysis methods." In *Uncertainty Management in Simulation-Optimization of Complex Systems*. Springer. | ASU library Springer | Library Scout | Most-cited GSA survey. Missing despite deep GSA collection. |

## Tier 2: MEDIUM -- Needed for upcoming work (next quarter)

| # | Citation | Where to look | Identified by | Notes |
|---|----------|---------------|---------------|-------|
| 9 | Campbell, D. T. & Fiske, D. W. (1959). "Convergent and discriminant validation by the multitrait-multimethod matrix." *Psychological Bulletin* 56(2):81-105. | ASU library APA PsycNET | Library Scout, X-Factor, Measurement Theory | Foundational for BEAD-0011 construct validity dossier. MTMM matrix. |
| 10 | Hernandez-Orallo, J. (2017). *The Measure of All Minds: Evaluating Natural and Artificial Intelligence*. Cambridge University Press. | ASU library CUP | Library Scout, X-Factor, Measurement Theory | Only book on AI measurement theory. Zero coverage for a project called "measurement science for AI agents." |
| 11 | AIAG (2010). *Measurement Systems Analysis Reference Manual*, 4th Edition. | ASU library or interlibrary loan; widely held | X-Factor, Gauge R&R | Canonical MSA reference. ndc formula, P/T ratio, gauge qualification criteria. Not freely available. |
| 12 | Morris, M. D. (1991). "Factorial Sampling Plans for Preliminary Computational Experiments." *Technometrics* 33(2):161-174. | ASU library JSTOR/Taylor & Francis | Library Scout | Original Morris screening method. salib-rs implements it; library has no paper on it. |
| 13 | Pianosi, F. et al. (2015). "A simple and efficient method for global sensitivity analysis based on cumulative distribution functions." *Environmental Modelling & Software* 67:1-11. | ASU library Elsevier | Library Scout | PAWN sensitivity index. Listed in salib-rs estimators. |
| 14 | Helton, J. C., Johnson, J. D. & Oberkampf, W. L. (2004). "An exploration of alternative approaches to the representation of uncertainty in model predictions." *Reliability Engineering & System Safety* 85(1-3):11-71. | ASU library Elsevier | QMU Defense Framework | Key reference for aleatory/epistemic separation in QMU. |
| 15 | Oberkampf, W. L. & Roy, C. J. (2010). *Verification and Validation in Scientific Computing*. Cambridge University Press. | ASU library CUP | QMU Defense Framework | V&V framework underlying QMU credibility assessment. |
| 16 | UK MOD Defence Standard 00-56 Issue 7. "Safety Management Requirements for Defence Systems." | UK MOD publications; may require defense research network access | QMU Defense Framework, X-Factor | Mandates safety cases. Check if ASU has access. |
| 17 | ISO 15026-2:2022. "Systems and software engineering -- Systems and software assurance -- Part 2: Assurance case." | ASU library ISO standards subscription | QMU Defense Framework | Assurance case international standard. |
| 18 | Borgonovo, E. et al. (2024). "Global Sensitivity Analysis via Optimal Transport." *Management Science* 71(5):3809-3828. | ASU library INFORMS | Web Scout | OT-based sensitivity indices. Potential salib-rs extension. |
| 19 | Rasch, G. (1960/1980). *Probabilistic Models for Some Intelligence and Attainment Tests*. University of Chicago Press. | ASU library | Measurement Theory | Foundational for specific objectivity claims in CAT. |
| 20 | Fleiss, J. L. (1971). "Measuring nominal scale agreement among many raters." *Psychological Bulletin* 76(5):378-382. | ASU library APA PsycNET | Library Scout | Fleiss kappa original. mojave implements it; library lacks the source paper. |
| 21 | Krippendorff, K. (2011). "Computing Krippendorff's Alpha-Reliability." *Communication Methods and Measures*. | ASU library Taylor & Francis | Library Scout | Alpha reliability computation. Implemented in mojave but no source paper. |

## Tier 3: LOW -- Background, theoretical depth, or future extensions

| # | Citation | Where to look | Identified by | Notes |
|---|----------|---------------|---------------|-------|
| 22 | Sobol, I. M. & Kucherenko, S. (2009). "Derivative based global sensitivity measures and their link with global sensitivity indices." *Mathematics and Computers in Simulation* 79(10):3009-3017. | ASU library Elsevier | Library Scout | DGSM. Listed in salib-rs. |
| 23 | Peters, O. (2019). "The ergodicity problem in economics." *Nature Physics* 15:1216-1221. | ASU library Nature | Measurement Theory, X-Factor | Ergodicity framing for SPC monitoring. |
| 24 | Borgonovo, E. et al. (2025). "Convexity and measures of statistical association." *JRSS-B* 87(4):1281-1304. | ASU library Wiley | Web Scout | Theoretical foundations for GSA measures. |
| 25 | Borgonovo, E. & Plischke, E. (2016). "Sensitivity analysis: A review of recent advances." *European Journal of Operational Research* 248(3):869-887. | ASU library Elsevier | X-Factor | Updated GSA survey. |
| 26 | Garrick, B. J. & Christie, R. F. (2002). "Probabilistic Risk Assessment Practices in the USA for Nuclear Power Plants." *Safety Science* 40(1-4):177-199. | ASU library Elsevier | QMU Defense Framework | Historical PRA context for QMU. |
| 27 | Webb, N. M., Shavelson, R. J. & Harding, E. (2006). "Reliability coefficients and generalizability theory." *Handbook of Statistics* 26:81-124. | ASU library Elsevier | Measurement Theory | Shorter G-theory reference. |
| 28 | Wilrich, P.-T. (2013). "Critical values of Mandel's h and k, the Grubbs and the Cochran test statistic." *AStA Advances in Statistical Analysis* 97:1-10. | ASU library Springer | Gauge R&R | Exact formulae for ISO 5725 outlier detection. |
| 29 | He, Q. et al. (2024). "New roles of Lagrange multiplier method in generalizability theory." PMC11486427. | **Free from PMC** | Gauge R&R | D-study budget optimization via constrained optimization. |
| 30 | Zhang, X.-Y. et al. (2015). "Sobol sensitivity analysis: a tool to guide the development and evaluation of systems pharmacology models." *CPT: Pharmacometrics & Systems Pharmacology* 4(2):69-79. | ASU library or free from PMC | Statistical Correctness | Convergence guidance "N in [10^2, 10^4]." |

## Free / Open-Access Papers (no ASU access needed)

These were identified but are freely downloadable:

| Citation | Source | Status |
|----------|--------|--------|
| National Academies QMU report (2009) | NAP website | Acquire with free NAP account |
| Sharp & Wood-Schultz (2003) Los Alamos Science 28 | LANL public website | Download directly |
| JASON QMU Report JSR-04-330 | OSTI.gov or FAS.org | Search OSTI |
| He et al. (2024) Lagrange method G-theory | PMC open access | Download from PMC |
| NNSA Annual Stockpile Assessment reports | NNSA public website | Download directly |
| Flores et al. (2018) ILS R package | CRAN / journal open access | Check CRAN vignette |

---

## Summary

- **Total unique papers to acquire:** 30
- **Tier 1 (HIGH):** 8 (3 are free)
- **Tier 2 (MEDIUM):** 13
- **Tier 3 (LOW):** 9 (2 are free)
- **Estimated ASU library coverage:** Most journal articles should be accessible via ASU's Elsevier, Springer, Wiley, APA, Taylor & Francis subscriptions. The AIAG MSA Manual and UK MOD Def Stan 00-56 may require special access or interlibrary loan.
