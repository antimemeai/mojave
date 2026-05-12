---
date: 2026-05-11
summary: Initial session — extracted old project, assessed state, designed architecture, launched deep research
---

## What happened

1. Extracted encrypted tarball containing previous project artifacts
2. Reviewed all contents: crates (saltelli-* and prior project crates), TCK specs, ADRs, notes, campaign configs, conex adapters, measurement theory note
3. Discussed product vision: longitudinal SPC for agent evaluation, not a benchmark or dashboard
4. Assessed market: defense first (user has access), regulated industries follow, broad market 12-18 months
5. Identified core value prop: "questions you don't know to ask" + ablation engine on customer infra
6. Established JSMNTL methodology (extreme rigor, TCK-first, 4-gate validation, lit reviews, beads)
7. Designed math core architecture: salib-rs repack + irr + seq-test + reliability + prereg (all Rust)
8. Designed system architecture: Rust everything, Python thin shell, bincode internal, JSON edge
9. Moved old artifacts to quarantine (gitignored), set up repo structure
10. Grabbed 36 papers (29 arXiv + 7 foundational) — 5 paywalled papers need ASU library trip
11. Prepared deep research prompt for sky Claude — now running

## Key decisions

- Project nameless for now
- saltelli-* → repack as salib-rs (standalone publishable Rust GSA library)
- Prior project crates → quarantine, shop from as needed
- All logic in Rust, Python is thin user shell
- Build math core first (IRR, seq-test, reliability, prereg), then pivot hard to orchestration
- Nice-to-haves tracked as beads, not lost

## State at end of session

- Repo initialized, first commit made
- Design spec written, awaiting sky Claude deep research feedback
- 14 beads open
- Papers in ../evals_papers/ (36 grabbed, 5 paywalled pending)
- dat/ still contains saltelli-* crates and TCK specs (not yet moved to crates/)
- Pre-commit hooks not yet set up (BEAD-0001)

## Next session priorities

1. Incorporate sky Claude deep research findings
2. Move saltelli-* crates from dat/ to proper workspace location
3. Set up pre-commit hooks (BEAD-0001)
4. Begin salib-rs repack or first new crate (IRR likely — most commercially legible)
5. Each crate starts with: lit review → written sub-plan → TCK specs → implementation

## ASU library trip list

1. Pocock 1977 — Biometrika 64(2):191-199. DOI: 10.1093/biomet/64.2.191
2. O'Brien & Fleming 1979 — Biometrics 35(3):549-556. DOI: 10.2307/2530245
3. Lan & DeMets 1983 — Biometrika 70(3):659-663. DOI: 10.1093/biomet/70.3.659
4. Fleiss 1971 — Psychological Bulletin 76(5):378-382. DOI: 10.1037/h0031619
5. Brennan 2001 — Generalizability Theory (Springer book). ISBN: 978-0-387-95227-8
