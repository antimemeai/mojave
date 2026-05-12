//! Scaffold TCK harness — proves Workspace's TCK machinery wires
//! end-to-end against a real `#[test]` invocation.
//!
//! No Workspace-behavior-under-test here; scenarios operate on a
//! trivial `World` (accumulator) so any failure localizes to harness
//! wiring, not Workspace behavior.
//!
//! Real Workspace-behavior scenarios (canonical encoding, chain
//! integrity, sink append-only, perturbation invariance, etc.) land
//! in their own `.feature` files with their own harness files in
//! subsequent phase plans (see `plans/0001-overall.md`).
//!
//! Feature file: `tck/scaffold/features/scaffold.feature`.
//!
//! Ported from substrate's
//! `crates/firecrew-test/tests/audit_seal_scaffold_tck.rs` on
//! 2026-04-28.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use metric_tck_harness::gherkin::{parse_feature, StepError, SyncRunner};

/// Path to the scaffold `.feature` file, resolved at compile time via
/// `CARGO_MANIFEST_DIR` so the test works regardless of where cargo was
/// invoked from.
fn scaffold_feature_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tck")
        .join("scaffold")
        .join("features")
        .join("scaffold.feature")
}

/// Trivial `World` for the scaffold. Records events in insertion order
/// and renders to a comma-separated string for `Then` assertions.
#[derive(Default, Debug)]
struct Accumulator {
    events: Vec<String>,
}

impl Accumulator {
    fn render(&self) -> String {
        self.events.join(", ")
    }
}

#[test]
fn scaffold_feature_runs_end_to_end() {
    let path = scaffold_feature_path();
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read scaffold feature at {}: {e}", path.display()));

    // Source label uses the filename only so error messages stay stable
    // across absolute-path differences.
    let feature =
        parse_feature(&content, "scaffold.feature").expect("scaffold.feature parses cleanly");

    // Sanity: the scaffold feature declares 3 scenarios (2 plain + 1
    // outline with 3 rows). If this count changes, the feature file
    // was edited; update the assertion with an explicit rationale.
    assert_eq!(
        feature.scenarios.len(),
        3,
        "scaffold feature scenario count"
    );

    let runner = SyncRunner::new(Accumulator::default)
        .step("a fresh accumulator", |_w, _| Ok(()))
        // Record-event steps: exact-string match means one step def
        // per distinct event value. Scenario Outline expansion handles
        // the parameterization across rows.
        .step("I record the event \"alpha\"", |w, _| {
            w.events.push("alpha".into());
            Ok(())
        })
        .step("I record the event \"bravo\"", |w, _| {
            w.events.push("bravo".into());
            Ok(())
        })
        .step("I record the event \"foo\"", |w, _| {
            w.events.push("foo".into());
            Ok(())
        })
        .step("I record the event \"bar\"", |w, _| {
            w.events.push("bar".into());
            Ok(())
        })
        .step("I record the event \"x\"", |w, _| {
            w.events.push("x".into());
            Ok(())
        })
        .step("I record the event \"y\"", |w, _| {
            w.events.push("y".into());
            Ok(())
        })
        .step("I record the event \"one\"", |w, _| {
            w.events.push("one".into());
            Ok(())
        })
        .step("I record the event \"two\"", |w, _| {
            w.events.push("two".into());
            Ok(())
        })
        // Assertion steps. One step def per expected value; Outline
        // substitution drives row-by-row matching.
        .step("the accumulator holds \"alpha, bravo\"", |w, _| {
            assert_holds(w, "alpha, bravo")
        })
        .step("the accumulator holds \"\"", |w, _| assert_holds(w, ""))
        .step("the accumulator holds \"foo, bar\"", |w, _| {
            assert_holds(w, "foo, bar")
        })
        .step("the accumulator holds \"x, y\"", |w, _| {
            assert_holds(w, "x, y")
        })
        .step("the accumulator holds \"one, two\"", |w, _| {
            assert_holds(w, "one, two")
        });

    let report = runner.run(&feature);

    // Five total: 2 plain + 3 outline rows.
    assert_eq!(report.total(), 5, "scaffold report total scenario count");

    // Panic-on-failure with a locator-rich summary. This is the usage
    // pattern every subsequent Workspace TCK harness file uses; if
    // it works here it's wired correctly.
    report.assert_all_passed();
}

fn assert_holds(w: &Accumulator, expected: &str) -> Result<(), StepError> {
    let got = w.render();
    if got == expected {
        Ok(())
    } else {
        Err(StepError::new(format!(
            "accumulator holds {got:?}; expected {expected:?}"
        )))
    }
}
