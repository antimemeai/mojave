---
id: BEAD-0004
title: Build sequential testing (SPRT / group-sequential) primitives
status: open
priority: high
created: 2026-05-11
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
