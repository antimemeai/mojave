use crate::change::ChangeRecord;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskScore {
    pub task_id: String,
    pub score: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeScoreEntry {
    pub sha: String,
    pub before: Vec<TaskScore>,
    pub after: Vec<TaskScore>,
}

impl ChangeScoreEntry {
    pub fn regressions(&self, threshold: f64) -> Vec<TaskRegression> {
        let mut result = Vec::new();
        for after_score in &self.after {
            if let Some(before_score) = self
                .before
                .iter()
                .find(|b| b.task_id == after_score.task_id)
            {
                let delta = after_score.score - before_score.score;
                if delta < -threshold {
                    result.push(TaskRegression {
                        task_id: after_score.task_id.clone(),
                        before: before_score.score,
                        after: after_score.score,
                        delta,
                    });
                }
            }
        }
        result
    }

    pub fn improvements(&self, threshold: f64) -> Vec<TaskImprovement> {
        let mut result = Vec::new();
        for after_score in &self.after {
            if let Some(before_score) = self
                .before
                .iter()
                .find(|b| b.task_id == after_score.task_id)
            {
                let delta = after_score.score - before_score.score;
                if delta > threshold {
                    result.push(TaskImprovement {
                        task_id: after_score.task_id.clone(),
                        before: before_score.score,
                        after: after_score.score,
                        delta,
                    });
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskRegression {
    pub task_id: String,
    pub before: f64,
    pub after: f64,
    pub delta: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskImprovement {
    pub task_id: String,
    pub before: f64,
    pub after: f64,
    pub delta: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeTaskMatrix {
    entries: Vec<ChangeScoreEntry>,
}

impl ChangeTaskMatrix {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, entry: ChangeScoreEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[ChangeScoreEntry] {
        &self.entries
    }

    pub fn find_by_sha(&self, sha: &str) -> Option<&ChangeScoreEntry> {
        self.entries.iter().find(|e| e.sha == sha)
    }

    pub fn all_regressions(&self, threshold: f64) -> Vec<(&str, Vec<TaskRegression>)> {
        self.entries
            .iter()
            .filter_map(|e| {
                let regs = e.regressions(threshold);
                if regs.is_empty() {
                    None
                } else {
                    Some((e.sha.as_str(), regs))
                }
            })
            .collect()
    }

    pub fn task_history(&self, task_id: &str) -> Vec<(String, f64)> {
        self.entries
            .iter()
            .filter_map(|e| {
                e.after
                    .iter()
                    .find(|s| s.task_id == task_id)
                    .map(|s| (e.sha.clone(), s.score))
            })
            .collect()
    }

    pub fn changes_affecting_task(
        &self,
        task_id: &str,
        threshold: f64,
    ) -> Vec<(&str, &ChangeScoreEntry)> {
        self.entries
            .iter()
            .filter(|e| {
                let before = e.before.iter().find(|s| s.task_id == task_id);
                let after = e.after.iter().find(|s| s.task_id == task_id);
                match (before, after) {
                    (Some(b), Some(a)) => (a.score - b.score).abs() > threshold,
                    _ => false,
                }
            })
            .map(|e| (e.sha.as_str(), e))
            .collect()
    }
}

impl Default for ChangeTaskMatrix {
    fn default() -> Self {
        Self::new()
    }
}

pub fn annotate_change_with_scores(
    _change: &ChangeRecord,
    before: Vec<TaskScore>,
    after: Vec<TaskScore>,
    sha: &str,
) -> ChangeScoreEntry {
    ChangeScoreEntry {
        sha: sha.to_string(),
        before,
        after,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> ChangeScoreEntry {
        ChangeScoreEntry {
            sha: "abc123".into(),
            before: vec![
                TaskScore {
                    task_id: "task-1".into(),
                    score: 0.8,
                },
                TaskScore {
                    task_id: "task-2".into(),
                    score: 0.9,
                },
                TaskScore {
                    task_id: "task-3".into(),
                    score: 0.5,
                },
            ],
            after: vec![
                TaskScore {
                    task_id: "task-1".into(),
                    score: 0.6,
                },
                TaskScore {
                    task_id: "task-2".into(),
                    score: 0.95,
                },
                TaskScore {
                    task_id: "task-3".into(),
                    score: 0.5,
                },
            ],
        }
    }

    #[test]
    fn regressions_detected() {
        let entry = sample_entry();
        let regs = entry.regressions(0.05);
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].task_id, "task-1");
        assert!((regs[0].delta - (-0.2)).abs() < 1e-10);
    }

    #[test]
    fn improvements_detected() {
        let entry = sample_entry();
        let imps = entry.improvements(0.01);
        assert_eq!(imps.len(), 1);
        assert_eq!(imps[0].task_id, "task-2");
    }

    #[test]
    fn no_regressions_below_threshold() {
        let entry = sample_entry();
        let regs = entry.regressions(0.5);
        assert!(regs.is_empty());
    }

    #[test]
    fn unchanged_task_not_in_either() {
        let entry = sample_entry();
        assert!(entry
            .regressions(0.01)
            .iter()
            .all(|r| r.task_id != "task-3"));
        assert!(entry
            .improvements(0.01)
            .iter()
            .all(|i| i.task_id != "task-3"));
    }

    #[test]
    fn matrix_add_and_find() {
        let mut matrix = ChangeTaskMatrix::new();
        matrix.add(sample_entry());
        assert_eq!(matrix.entries().len(), 1);
        assert!(matrix.find_by_sha("abc123").is_some());
        assert!(matrix.find_by_sha("xyz").is_none());
    }

    #[test]
    fn matrix_all_regressions() {
        let mut matrix = ChangeTaskMatrix::new();
        matrix.add(sample_entry());
        let regs = matrix.all_regressions(0.05);
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].0, "abc123");
    }

    #[test]
    fn matrix_task_history() {
        let mut matrix = ChangeTaskMatrix::new();
        matrix.add(sample_entry());
        let history = matrix.task_history("task-1");
        assert_eq!(history.len(), 1);
        assert!((history[0].1 - 0.6).abs() < 1e-10);
    }

    #[test]
    fn matrix_changes_affecting_task() {
        let mut matrix = ChangeTaskMatrix::new();
        matrix.add(sample_entry());
        let affecting = matrix.changes_affecting_task("task-1", 0.05);
        assert_eq!(affecting.len(), 1);
        let not_affecting = matrix.changes_affecting_task("task-3", 0.05);
        assert!(not_affecting.is_empty());
    }

    #[test]
    fn serde_round_trip() {
        let entry = sample_entry();
        let json = serde_json::to_string(&entry).unwrap();
        let back: ChangeScoreEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sha, "abc123");
    }
}
