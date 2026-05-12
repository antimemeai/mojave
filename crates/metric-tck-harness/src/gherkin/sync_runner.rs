//! **Synchronous** scenario runner with step-definition registration.
//!
//! A [`SyncRunner`] maps Gherkin step text (after [`Examples`](crate::gherkin::model::Examples) substitution)
//! to a step-definition closure that mutates a test-scoped `World` and
//! either succeeds or returns a human-readable error. Running a
//! [`Feature`] invokes each scenario's steps in order against a fresh
//! `World`, reporting per-scenario pass/fail with a file+line locator.
//!
//! # Sync-only (explicit)
//!
//! Step-definition closures are synchronous — `Fn(&mut World, &Step)
//! -> Result<(), StepError>`. Scenarios that need to `.await` (Postgres
//! writes via `AuditSink::append_entries`, model calls, RPC, any other
//! async substrate surface) cannot use `SyncRunner` as-is.
//!
//! The plan is to add a sibling `AsyncRunner<W>` in a separate
//! `async_runner.rs` module alongside this one when the first async TCK
//! scenario demands it. Until then, `SyncRunner` stays limited-by-name
//! so no consumer accidentally assumes async support.
//!
//! # Model
//!
//! - The `World` is user-defined and carries any state the scenario
//!   threads across steps (fixtures, accumulated observations, etc.).
//! - Step definitions register with a "step text" string (matched
//!   exactly after substitution) and a synchronous closure
//!   `Fn(&mut World, &Step) -> Result<(), StepError>`.
//! - `Given` / `When` / `Then` / `And` / `But` all use the same
//!   registration space — Gherkin's step-keyword differences are
//!   prose-only here (step definitions don't dispatch by keyword).
//!   The parser preserves [`StepKind`](crate::gherkin::model::StepKind) for diagnostic reporting.
//!
//! # Exact matching vs regex
//!
//! Step texts match **exactly** after [`Examples`](crate::gherkin::model::Examples) substitution. No
//! regex, no parameter extraction à la cucumber-rs. Rationale: substrate
//! TCK scenarios are declarative enough that `Scenario Outline` +
//! `Examples` covers parameterization; regex-based step defs add runtime
//! complexity we don't need for this scope.
//!
//! If substrate TCK grows toward regex-parameterized steps, that's the
//! first natural reason to promote to cucumber-rs.
//!
//! # Reporting
//!
//! Execution returns a [`RunReport`] listing every scenario with its
//! outcome. Failures name the feature, scenario, step text, source
//! file, and line. [`RunReport::assert_all_passed`] is the panic-on-
//! failure convenience for use in `#[test]` functions.

use std::collections::HashMap;

use super::model::{Feature, Scenario, ScenarioKind, Step};

/// Error from a step definition.
#[derive(Debug, Clone)]
pub struct StepError {
    pub message: String,
}

impl StepError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for StepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for StepError {}

/// A step-definition closure boxed for storage in the registry.
type StepFn<W> = Box<dyn Fn(&mut W, &Step) -> Result<(), StepError> + Send + Sync>;

/// Scenario runner parameterized on a user-defined `World` type.
///
/// `World` must be constructible via a factory closure supplied at
/// [`SyncRunner::new`] — the runner constructs a fresh `World` for each
/// scenario so scenarios can't leak state between each other.
pub struct SyncRunner<W> {
    world_factory: Box<dyn Fn() -> W + Send + Sync>,
    steps: HashMap<String, StepFn<W>>,
}

// Hand-rolled Debug: the factory and step closures are opaque (dyn Fn
// has no Debug impl), so we render a summary of the step registry —
// the meaningful diagnostic signal — and mark the opaque fields with
// finish_non_exhaustive.
impl<W> std::fmt::Debug for SyncRunner<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncRunner")
            .field("step_count", &self.steps.len())
            .field("step_texts", &self.steps.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl<W> SyncRunner<W> {
    /// Construct a runner with a factory that builds a fresh `World`
    /// per scenario.
    #[must_use]
    pub fn new<F>(world_factory: F) -> Self
    where
        F: Fn() -> W + Send + Sync + 'static,
    {
        Self {
            world_factory: Box::new(world_factory),
            steps: HashMap::new(),
        }
    }

    /// Register a step definition keyed by exact step text (after
    /// [`Examples`](crate::gherkin::model::Examples) substitution).
    ///
    /// Duplicate registration for the same step text is a programming
    /// error — later registrations overwrite earlier ones. Self-test
    /// `duplicate_step_registration_overwrites` pins the behavior.
    #[must_use = "SyncRunner::step returns a new runner; builder-style chain requires capturing the return"]
    pub fn step<F>(mut self, text: impl Into<String>, handler: F) -> Self
    where
        F: Fn(&mut W, &Step) -> Result<(), StepError> + Send + Sync + 'static,
    {
        self.steps.insert(text.into(), Box::new(handler));
        self
    }

    /// Run every scenario in `feature`, expanding outlines row by row.
    /// Returns a [`RunReport`] enumerating all outcomes.
    #[must_use]
    pub fn run(&self, feature: &Feature) -> RunReport {
        let mut results: Vec<ScenarioResult> = Vec::new();

        for scenario in &feature.scenarios {
            match &scenario.kind {
                ScenarioKind::Plain => {
                    let outcome = self.run_scenario_once(feature, scenario, None);
                    results.push(outcome);
                }
                ScenarioKind::Outline { examples } => {
                    for (row_index, _row) in examples.rows.iter().enumerate() {
                        let outcome = self.run_scenario_once(feature, scenario, Some(row_index));
                        results.push(outcome);
                    }
                }
            }
        }

        RunReport { results }
    }

    fn run_scenario_once(
        &self,
        feature: &Feature,
        scenario: &Scenario,
        outline_row: Option<usize>,
    ) -> ScenarioResult {
        let mut world = (self.world_factory)();

        for step in &scenario.steps {
            let effective_text = match (&scenario.kind, outline_row) {
                (ScenarioKind::Outline { examples }, Some(row_index)) => {
                    examples.substitute(&step.text, row_index)
                }
                _ => step.text.clone(),
            };

            let Some(handler) = self.steps.get(&effective_text) else {
                return ScenarioResult::failed(
                    feature,
                    scenario,
                    step,
                    &effective_text,
                    outline_row,
                    StepError::new(format!(
                        "no step definition registered for: {effective_text}"
                    )),
                );
            };

            if let Err(err) = handler(&mut world, step) {
                return ScenarioResult::failed(
                    feature,
                    scenario,
                    step,
                    &effective_text,
                    outline_row,
                    err,
                );
            }
        }

        ScenarioResult::passed(feature, scenario, outline_row)
    }
}

/// Outcome of running a single scenario (one row of an outline if
/// applicable).
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub feature_name: String,
    pub feature_source: String,
    pub scenario_name: String,
    pub scenario_line: usize,
    pub outline_row: Option<usize>,
    pub outcome: Outcome,
}

/// Per-scenario outcome discriminant.
#[derive(Debug, Clone)]
pub enum Outcome {
    Passed,
    Failed {
        step_text: String,
        step_line: usize,
        error: String,
    },
}

impl ScenarioResult {
    fn passed(feature: &Feature, scenario: &Scenario, outline_row: Option<usize>) -> Self {
        Self {
            feature_name: feature.name.clone(),
            feature_source: feature.source.clone(),
            scenario_name: scenario.name.clone(),
            scenario_line: scenario.line,
            outline_row,
            outcome: Outcome::Passed,
        }
    }

    fn failed(
        feature: &Feature,
        scenario: &Scenario,
        step: &Step,
        effective_text: &str,
        outline_row: Option<usize>,
        error: StepError,
    ) -> Self {
        Self {
            feature_name: feature.name.clone(),
            feature_source: feature.source.clone(),
            scenario_name: scenario.name.clone(),
            scenario_line: scenario.line,
            outline_row,
            outcome: Outcome::Failed {
                step_text: effective_text.to_string(),
                step_line: step.line,
                error: error.message,
            },
        }
    }

    #[must_use]
    pub fn is_passed(&self) -> bool {
        matches!(self.outcome, Outcome::Passed)
    }
}

/// Aggregate of scenario outcomes after a feature run.
#[derive(Debug, Clone)]
pub struct RunReport {
    pub results: Vec<ScenarioResult>,
}

impl RunReport {
    #[must_use]
    pub fn total(&self) -> usize {
        self.results.len()
    }

    #[must_use]
    pub fn passed(&self) -> usize {
        self.results.iter().filter(|r| r.is_passed()).count()
    }

    #[must_use]
    pub fn failed(&self) -> usize {
        self.total() - self.passed()
    }

    /// Panic with a formatted failure summary if any scenario failed.
    /// Intended use: terminal call in a `#[test]` function so a single
    /// `cargo test` invocation reports all failures in one go.
    pub fn assert_all_passed(&self) {
        let failures: Vec<&ScenarioResult> =
            self.results.iter().filter(|r| !r.is_passed()).collect();
        if failures.is_empty() {
            return;
        }
        let mut msg = format!("{} of {} scenarios failed:\n", failures.len(), self.total());
        for result in failures {
            let loc = if result.feature_source.is_empty() {
                format!("line {}", result.scenario_line)
            } else {
                format!("{}:{}", result.feature_source, result.scenario_line)
            };
            let row_suffix = result
                .outline_row
                .map(|r| format!(" [row {r}]"))
                .unwrap_or_default();
            match &result.outcome {
                Outcome::Failed {
                    step_text,
                    step_line,
                    error,
                } => {
                    use std::fmt::Write as _;
                    let _ = writeln!(
                        msg,
                        "  - {} / {}{} @ {} (step line {}): {} — {}",
                        result.feature_name,
                        result.scenario_name,
                        row_suffix,
                        loc,
                        step_line,
                        step_text,
                        error
                    );
                }
                Outcome::Passed => {}
            }
        }
        panic!("{msg}");
    }
}

#[cfg(test)]
mod tests {
    use super::super::parser::parse_feature;
    use super::*;

    #[derive(Default)]
    struct TestWorld {
        events: Vec<String>,
    }

    fn make_runner() -> SyncRunner<TestWorld> {
        SyncRunner::new(TestWorld::default)
            .step("a starting condition", |w, _| {
                w.events.push("starting".to_string());
                Ok(())
            })
            .step("an action happens", |w, _| {
                w.events.push("action".to_string());
                Ok(())
            })
            .step("an outcome follows", |w, _| {
                w.events.push("outcome".to_string());
                Ok(())
            })
    }

    #[test]
    fn plain_scenario_runs_steps_in_order() {
        let src = "\
Feature: minimal

  Scenario: trivial
    Given a starting condition
    When an action happens
    Then an outcome follows
";
        let feature = parse_feature(src, "minimal.feature").unwrap();
        let report = make_runner().run(&feature);
        assert_eq!(report.total(), 1);
        assert_eq!(report.passed(), 1);
        assert_eq!(report.failed(), 0);
        report.assert_all_passed();
    }

    #[test]
    fn scenario_outline_expands_one_result_per_example_row() {
        let src = "\
Feature: outlined

  Scenario Outline: per row
    Given the name is <name>

    Examples:
      | name  |
      | alice |
      | bob   |
      | carol |
";
        let feature = parse_feature(src, "").unwrap();
        let runner = SyncRunner::new(TestWorld::default)
            .step("the name is alice", |_, _| Ok(()))
            .step("the name is bob", |_, _| Ok(()))
            .step("the name is carol", |_, _| Ok(()));
        let report = runner.run(&feature);
        assert_eq!(report.total(), 3, "one result per outline row");
        assert_eq!(report.passed(), 3);
    }

    #[test]
    fn missing_step_definition_fails_with_named_step() {
        let src = "\
Feature: missing-def

  Scenario: undefined step
    Given an unregistered step
";
        let feature = parse_feature(src, "").unwrap();
        let runner: SyncRunner<TestWorld> = SyncRunner::new(TestWorld::default);
        let report = runner.run(&feature);
        assert_eq!(report.passed(), 0);
        assert_eq!(report.failed(), 1);
        let Outcome::Failed {
            step_text, error, ..
        } = &report.results[0].outcome
        else {
            panic!("expected failure");
        };
        assert_eq!(step_text, "an unregistered step");
        assert!(error.contains("no step definition"), "got: {error}");
    }

    #[test]
    fn step_error_is_reported_with_locator() {
        let src = "\
Feature: step-error

  Scenario: deliberate fail
    Given a handler that returns an error
";
        let feature = parse_feature(src, "err.feature").unwrap();
        let runner = SyncRunner::new(TestWorld::default)
            .step("a handler that returns an error", |_, _| {
                Err(StepError::new("planned failure"))
            });
        let report = runner.run(&feature);
        let result = &report.results[0];
        assert!(!result.is_passed());
        let Outcome::Failed {
            step_line, error, ..
        } = &result.outcome
        else {
            panic!("expected failure");
        };
        assert_eq!(*step_line, 4);
        assert_eq!(error, "planned failure");
    }

    #[test]
    fn scenario_failure_stops_scenario_not_feature() {
        let src = "\
Feature: stop-at-scenario

  Scenario: first fails
    Given a handler that fails

  Scenario: second passes
    Given a passing step
";
        let feature = parse_feature(src, "").unwrap();
        let runner = SyncRunner::new(TestWorld::default)
            .step("a handler that fails", |_, _| Err(StepError::new("nope")))
            .step("a passing step", |_, _| Ok(()));
        let report = runner.run(&feature);
        assert_eq!(report.total(), 2);
        assert_eq!(report.passed(), 1);
        assert_eq!(report.failed(), 1);
        // Order preserved: first is the failure.
        assert!(!report.results[0].is_passed());
        assert!(report.results[1].is_passed());
    }

    #[test]
    fn world_is_fresh_per_scenario() {
        // If world state leaks between scenarios, the second scenario's
        // assertion will see leaked events from the first.
        let src = "\
Feature: isolation

  Scenario: first
    Given the world starts empty

  Scenario: second
    Given the world starts empty
";
        let feature = parse_feature(src, "").unwrap();
        let runner = SyncRunner::new(TestWorld::default).step("the world starts empty", |w, _| {
            if w.events.is_empty() {
                w.events.push("visited".to_string());
                Ok(())
            } else {
                Err(StepError::new("world leaked from previous scenario"))
            }
        });
        let report = runner.run(&feature);
        assert_eq!(report.passed(), 2);
    }

    #[test]
    fn duplicate_step_registration_overwrites() {
        let runner = SyncRunner::new(TestWorld::default)
            .step("same text", |_, _| Err(StepError::new("first")))
            .step("same text", |_, _| Err(StepError::new("second")));
        let src = "\
Feature: dup

  Scenario: dup
    Given same text
";
        let feature = parse_feature(src, "").unwrap();
        let report = runner.run(&feature);
        let Outcome::Failed { error, .. } = &report.results[0].outcome else {
            panic!("expected failure");
        };
        assert_eq!(error, "second", "later registration must win");
    }

    #[test]
    fn and_and_but_share_step_registration_with_preceding_keyword() {
        // Gherkin's And/But inherit semantic meaning from the previous
        // step's keyword. Our runner matches step text exactly, so And
        // and But also work as long as their text is registered.
        let src = "\
Feature: and-but

  Scenario: mixed
    Given first
    And second
    When third
    But fourth
    Then fifth
";
        let feature = parse_feature(src, "").unwrap();
        let runner = SyncRunner::new(TestWorld::default)
            .step("first", |_, _| Ok(()))
            .step("second", |_, _| Ok(()))
            .step("third", |_, _| Ok(()))
            .step("fourth", |_, _| Ok(()))
            .step("fifth", |_, _| Ok(()));
        let report = runner.run(&feature);
        report.assert_all_passed();
    }

    #[test]
    fn assert_all_passed_panics_on_failure_with_locator() {
        let src = "\
Feature: panic-check

  Scenario: with failure
    Given a failing step
";
        let feature = parse_feature(src, "fail.feature").unwrap();
        let runner = SyncRunner::new(TestWorld::default)
            .step("a failing step", |_, _| Err(StepError::new("the reason")));
        let report = runner.run(&feature);
        // assert_all_passed should panic; catch it and check the message.
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| report.assert_all_passed()));
        let err = result.expect_err("assert_all_passed should have panicked");
        let msg = err
            .downcast::<String>()
            .map(|s| *s)
            .or_else(|e| e.downcast::<&'static str>().map(|s| s.to_string()))
            .unwrap_or_else(|_| String::from("<non-string panic>"));
        assert!(msg.contains("fail.feature"), "got: {msg}");
        assert!(msg.contains("panic-check"), "got: {msg}");
        assert!(msg.contains("the reason"), "got: {msg}");
    }
}
