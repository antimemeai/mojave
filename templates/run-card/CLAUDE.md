# CLAUDE.md — working notes for Claude Code

This repo holds two **parametric LaTeX report templates** for evaluation
artifacts. They share one proven engine design. Read this before editing.

```
eval-runcards/
├── single-run-card/        One model, one eval run
│   ├── runcard.tex             engine  (do NOT edit per-run)
│   ├── runcard-config.tex      parameters  (edit this)
│   ├── domains.csv             optional per-domain table
│   ├── rawdata.csv             optional arbitrary-length raw dump
│   └── Makefile
├── cross-run-summary/      One eval, many runs aggregated
│   ├── eval-summary.tex        engine  (do NOT edit per-summary)
│   ├── eval-summary-config.tex parameters  (edit this)
│   ├── domain-pooled.csv       optional per-domain pooled table
│   ├── runs.csv                optional arbitrary-length run manifest
│   └── Makefile
├── Makefile                builds both
└── README.md
```

## The golden rule

For a normal job you edit **only the `*-config.tex` file**. The `.tex`
engine and the CSV inputs are the only other moving parts. Never hand-edit
layout to change content.

## How the engine works (both templates, identical core)

- **Key/value store.** `\rcset{key}{value}` defines a field; `\rc{key}`
  prints it. A key that is missing *or* set empty renders a muted em-dash,
  so a partially-filled artifact still compiles and reads sensibly. This is
  intentional — prefer leaving a key empty over deleting it.
- **Values are real LaTeX.** `\texttt{}`, math mode, `\\` line breaks, and
  `\textcolor{}` all work inside a value. Escape `& % # _ $` as usual.
- **Histograms.** `\rcset{...hist}{6,19,54,...}` is a comma list of bin
  **counts**, low→high. `\rchist{key}` renders an auto-scaled sparkbar.
  Empty list → "(no histogram data)" placeholder. Pure numbers only.
- **Disclaimers** have built-in defaults defined in the engine via
  `\@ifundefined`. Override by setting the same `disc.*` key in the config.
- Internal macros use `@`; they live inside one `\makeatletter …
  \makeatother` block. If you add engine macros, keep them in that block.

## CSV inputs — read this, it has bitten us

1. **Header row is auto-skipped.** Every CSV needs a header line; the engine
   reads columns **positionally** (`\csvcoli`, `\csvcolii`, …), not by name.
   Reordering values is fine as long as the header line matches reality.
2. **No commas inside fields. Ever.** The reader uses comma as the
   separator and this `csvsimple` build does not honor quoted commas — a
   comma inside a field silently makes `csvsimple` drop *every* data row
   (symptom: table header prints, body is empty). This is why the run
   manifest stores `ci_low,ci_high` as two columns and the template
   composes `[lo,\,hi]`. Apply the same pattern for any bracketed/list value.
3. **Arbitrary length is free.** The raw dump (`rawdata.csv`) and run
   manifest (`runs.csv`) render as a `longtable` that paginates with a
   repeating header and a "continued on next page" footer. 5 rows or
   50 000 — no template change.
4. **Optional tables self-collapse.** If `domains.csv` /
   `domain-pooled.csv` / the dump file is absent, that block prints a short
   "not supplied" note instead of breaking.

### Current CSV schemas (positional)

- `single-run-card/domains.csv`: `domain,accuracy,ci_low,ci_high,n`
- `single-run-card/rawdata.csv`: `item_id,variant_id,prompt_hash,correct,theta,stop_t`
- `cross-run-summary/domain-pooled.csv`: `domain,mean_acc,sd,runs`
- `cross-run-summary/runs.csv`: `run_id,model,revision,date,accuracy,ci_low,ci_high,n_items`

To change column count/width, edit the single `\csvreader[...]{file.csv}`
block near the end of the relevant engine file (column spec + `table head`
+ the body line that lists `\csvcolN`).

## Building

```
make                 # from repo root: builds both PDFs
make -C single-run-card
make -C cross-run-summary
make clean            # remove aux/log; veryclean also removes PDFs
```

Always **two `pdflatex` passes** (the Makefiles do this): the running
header and `longtable` column widths need the second pass to settle. No
bibtex/biber. Exit status 0 and zero `Overfull` warnings is the bar.

## Portability

The engines load `lmodern`/`microtype` only if a scalable font is present,
so they build on minimal TeX Live too. For best typography install
`texlive-fonts-recommended`. Required packages: `geometry, xcolor, array,
tabularx, booktabs, longtable, enumitem, titlesec, fancyhdr, tikz,
csvsimple, hyperref` (all in `texlive-latex-recommended` +
`texlive-latex-extra` + `texlive-pictures`).

## Adding a third artifact "in the same fashion"

Copy an engine, keep the `\rcset/\rc/\rchist/\fact/\stat` core and the
`\makeatletter` block verbatim, change only: the running-header label, the
palette `rcaccent` colour, the disclaimer defaults, the title block, the
`\section` bodies, and the final `\csvreader` block. Then add a matching
`*-config.tex` and a Makefile mirroring the others. Verify with two passes
and a visual check before declaring done.

## Done-criteria checklist

- [ ] `make` exits 0 for both templates
- [ ] `grep -c Overfull build.log` is 0
- [ ] Page 1 renders title block + first sections
- [ ] Arbitrary-length table paginates with repeating header/footer
- [ ] Empty/missing keys show em-dash, not errors
- [ ] No commas inside any CSV field
