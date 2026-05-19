# Mojave eval run-card templates

Two parametric LaTeX report templates that share one engine design:

| Template | Question it answers |
|----------|--------------------|
| **`single-run-card/`** | *One model on one eval run* — accuracy + CIs, IRT instrument quality, perturbation sensitivity, retrospective sequential stopping, disclaimers, and an arbitrary-length raw-data dump. |
| **`cross-run-summary/`** | *One eval, many runs* — cohort comparability, pooled accuracy + dispersion, random-effects estimate, ranking with CI-overlap, reproducibility of replicates, cross-run IRT/perturbation stability, and an arbitrary-length run manifest. |

You edit a `*-config.tex` parameter file; the engine handles layout,
sparkbar histograms, optional sub-tables, and arbitrary-length paginated
tables. Missing/empty fields degrade gracefully to an em-dash.

## Quick start

```
make                       # builds both PDFs (two pdflatex passes each)
make -C single-run-card    # just the run card
make -C cross-run-summary  # just the summary
```

Then edit the relevant `*-config.tex` and rebuild. See **`CLAUDE.md`** for
the full engine contract and the CSV rules (notably: no commas inside CSV
fields, and the header row is auto-skipped).

Sample data is included in every CSV so both templates build out-of-the-box
into multi-page PDFs you can inspect immediately.
