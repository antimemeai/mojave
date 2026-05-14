use eval_core::{Outcome, TrialRecord};

use crate::types::{IngestWarning, WarningKind};

/// Unix epoch for 2020-01-01T00:00:00Z.
const EPOCH_2020: i64 = 1_577_836_800;

/// Validate a single [`TrialRecord`] for ingestion integrity.
///
/// Returns `Ok(())` if the record passes all checks, or an [`IngestWarning`]
/// describing the first failure encountered.  The caller is responsible for
/// collecting multiple warnings by invoking this once per record.
///
/// # Parameters
/// - `record`       — the record to check.
/// - `source_index` — zero-based position in the source file (for diagnostics).
/// - `source_id`    — human-readable ID such as sample UUID (for diagnostics).
/// - `now`          — current Unix epoch seconds; timestamp must not exceed `now + 86400`.
pub fn validate_record(
    record: &TrialRecord,
    source_index: Option<usize>,
    source_id: Option<String>,
    now: i64,
) -> Result<(), IngestWarning> {
    let mk_warn = |kind: WarningKind| IngestWarning {
        source_index,
        source_id: source_id.clone(),
        kind,
    };

    if record.task_id.is_empty() {
        return Err(mk_warn(WarningKind::EmptyTaskId));
    }

    if record.agent_id.is_empty() {
        return Err(mk_warn(WarningKind::EmptyAgentId));
    }

    if record.timestamp < EPOCH_2020 {
        return Err(mk_warn(WarningKind::TimestampTooOld(record.timestamp)));
    }

    let deadline = now + 86_400;
    if record.timestamp > deadline {
        return Err(mk_warn(WarningKind::TimestampInFuture(record.timestamp)));
    }

    match &record.outcome {
        Outcome::Score(v) => {
            if !v.is_finite() {
                return Err(mk_warn(WarningKind::NonFiniteScore(*v)));
            }
        }
        Outcome::MultiCriterion(map) => {
            for (key, value) in map {
                if !value.is_finite() {
                    return Err(mk_warn(WarningKind::NonFiniteCriterion {
                        key: key.clone(),
                        value: *value,
                    }));
                }
            }
        }
        Outcome::Binary(_) | Outcome::Graded(_) => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use std::collections::BTreeMap;

    use eval_core::{Outcome, TrialRecord};
    use ulid::Ulid;

    use super::*;

    fn make_record(task_id: &str, agent_id: &str, timestamp: i64, outcome: Outcome) -> TrialRecord {
        TrialRecord {
            trial_id: Ulid::nil(),
            run_id: Ulid::nil(),
            task_id: task_id.to_owned(),
            task_version: None,
            agent_id: agent_id.to_owned(),
            agent_version: None,
            judge_config: None,
            seed: None,
            timestamp,
            outcome,
            metadata: BTreeMap::new(),
        }
    }

    const NOW: i64 = 1_748_000_000; // arbitrary "now" in 2025

    #[test]
    fn valid_record_passes() {
        let r = make_record("task1", "agent1", 1_700_000_000, Outcome::Binary(true));
        assert!(validate_record(&r, None, None, NOW).is_ok());
    }

    #[test]
    fn empty_task_id_fails() {
        let r = make_record("", "agent1", 1_700_000_000, Outcome::Binary(true));
        let w = validate_record(&r, None, None, NOW).unwrap_err();
        assert_eq!(w.kind, WarningKind::EmptyTaskId);
    }

    #[test]
    fn empty_agent_id_fails() {
        let r = make_record("task1", "", 1_700_000_000, Outcome::Binary(true));
        let w = validate_record(&r, None, None, NOW).unwrap_err();
        assert_eq!(w.kind, WarningKind::EmptyAgentId);
    }

    #[test]
    fn timestamp_too_old_fails() {
        let r = make_record("task1", "agent1", 1_000_000_000, Outcome::Binary(true));
        let w = validate_record(&r, None, None, NOW).unwrap_err();
        assert!(matches!(w.kind, WarningKind::TimestampTooOld(_)));
    }

    #[test]
    fn timestamp_in_future_fails() {
        let future = NOW + 90_000;
        let r = make_record("task1", "agent1", future, Outcome::Binary(true));
        let w = validate_record(&r, None, None, NOW).unwrap_err();
        assert!(matches!(w.kind, WarningKind::TimestampInFuture(_)));
    }

    #[test]
    fn non_finite_score_fails() {
        let r = make_record("task1", "agent1", 1_700_000_000, Outcome::Score(f64::NAN));
        let w = validate_record(&r, None, None, NOW).unwrap_err();
        assert!(matches!(w.kind, WarningKind::NonFiniteScore(_)));
    }

    #[test]
    fn non_finite_multi_criterion_fails() {
        let mut map = BTreeMap::new();
        map.insert("x".to_owned(), f64::INFINITY);
        let r = make_record(
            "task1",
            "agent1",
            1_700_000_000,
            Outcome::MultiCriterion(map),
        );
        let w = validate_record(&r, None, None, NOW).unwrap_err();
        assert!(matches!(w.kind, WarningKind::NonFiniteCriterion { .. }));
    }
}
