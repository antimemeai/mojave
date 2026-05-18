---
id: BEAD-0018
title: mojave-cli — unified CLI entry point
status: closed
priority: high
created: 2026-05-18
---

## Description

Single `mojave` binary providing the CLI entry point to the entire measurement engine. Four subcommands: ingest (Inspect + JSONL → TrialRecords), analyze (batch measurement battery), monitor (streaming analysis), sensitivity (delegates to published salib crate). JSON to stdout, structured errors to stderr. Shell completion via `mojave completions {bash,zsh,fish,...}`. Replaces salib-cli.

## Acceptance

- [x] salib-* crates extracted from workspace, deps point to crates.io 0.1.1
- [x] mojave-cli crate with four subcommands (ingest, analyze, monitor, sensitivity)
- [x] JSON output with hint fields on all Decision objects
- [x] Config loading: YAML file + CLI flag overrides + defaults
- [x] Format auto-detection (Inspect vs JSONL)
- [x] Monitor: stdin mode + file watch mode
- [x] Exit codes: 0 success, 1 error, 2 usage error
- [x] Shell completion support (clap_complete)
- [x] CLI smoke tests (assert_cmd)
- [x] TCK Gherkin feature file
- [x] Clippy zero warnings, rustfmt clean
- [x] Full workspace test suite passes
