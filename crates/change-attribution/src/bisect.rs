use crate::mapping::TaskScore;

#[derive(Debug, Clone)]
pub struct BisectState {
    commits: Vec<String>,
    scores: Vec<Option<Vec<TaskScore>>>,
    target_task: String,
    good_idx: usize,
    bad_idx: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BisectStep {
    Test { sha: String, index: usize },
    Found { sha: String },
    NeedsMoreData,
}

impl BisectState {
    pub fn new(commits: Vec<String>, target_task: String) -> Self {
        let len = commits.len();
        let scores = vec![None; len];
        Self {
            commits,
            scores,
            target_task,
            good_idx: 0,
            bad_idx: len.saturating_sub(1),
        }
    }

    pub fn mark_good(&mut self, sha: &str) {
        if let Some(idx) = self.commits.iter().position(|c| c == sha) {
            if idx > self.good_idx {
                self.good_idx = idx;
            }
        }
    }

    pub fn mark_bad(&mut self, sha: &str) {
        if let Some(idx) = self.commits.iter().position(|c| c == sha) {
            if idx < self.bad_idx {
                self.bad_idx = idx;
            }
        }
    }

    pub fn record_scores(&mut self, sha: &str, scores: Vec<TaskScore>) {
        if let Some(idx) = self.commits.iter().position(|c| c == sha) {
            self.scores[idx] = Some(scores);
        }
    }

    pub fn next_step(&self) -> BisectStep {
        if self.good_idx >= self.bad_idx {
            return if self.bad_idx < self.commits.len() {
                BisectStep::Found {
                    sha: self.commits[self.bad_idx].clone(),
                }
            } else {
                BisectStep::NeedsMoreData
            };
        }

        if self.bad_idx - self.good_idx == 1 {
            return BisectStep::Found {
                sha: self.commits[self.bad_idx].clone(),
            };
        }

        let mid = self.good_idx + (self.bad_idx - self.good_idx) / 2;
        BisectStep::Test {
            sha: self.commits[mid].clone(),
            index: mid,
        }
    }

    pub fn auto_bisect<F>(&mut self, threshold: f64, mut eval_fn: F) -> BisectStep
    where
        F: FnMut(&str) -> Option<Vec<TaskScore>>,
    {
        loop {
            match self.next_step() {
                BisectStep::Test { ref sha, .. } => {
                    let sha_clone = sha.clone();
                    if let Some(scores) = eval_fn(&sha_clone) {
                        let task_score = scores
                            .iter()
                            .find(|s| s.task_id == self.target_task)
                            .map(|s| s.score);

                        self.record_scores(&sha_clone, scores);

                        match task_score {
                            Some(score) if score >= threshold => self.mark_good(&sha_clone),
                            Some(_) => self.mark_bad(&sha_clone),
                            None => return BisectStep::NeedsMoreData,
                        }
                    } else {
                        return BisectStep::NeedsMoreData;
                    }
                }
                step @ (BisectStep::Found { .. } | BisectStep::NeedsMoreData) => return step,
            }
        }
    }

    pub fn remaining_steps(&self) -> usize {
        if self.good_idx >= self.bad_idx {
            return 0;
        }
        let range = self.bad_idx - self.good_idx;
        (range as f64).log2().ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commits(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("commit-{i}")).collect()
    }

    #[test]
    fn initial_step_is_midpoint() {
        let state = BisectState::new(commits(8), "task-1".into());
        match state.next_step() {
            BisectStep::Test { index, .. } => assert_eq!(index, 3),
            other => panic!("expected Test, got {other:?}"),
        }
    }

    #[test]
    fn converges_to_found() {
        let mut state = BisectState::new(commits(8), "task-1".into());
        state.mark_good("commit-0");
        state.mark_bad("commit-7");

        state.mark_good("commit-3");

        state.mark_bad("commit-5");

        match state.next_step() {
            BisectStep::Test { index, .. } => assert_eq!(index, 4),
            other => panic!("expected Test at 4, got {other:?}"),
        }

        state.mark_good("commit-4");

        match state.next_step() {
            BisectStep::Found { sha } => assert_eq!(sha, "commit-5"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn single_commit_range_is_found() {
        let mut state = BisectState::new(commits(4), "task-1".into());
        state.mark_good("commit-1");
        state.mark_bad("commit-2");
        match state.next_step() {
            BisectStep::Found { sha } => assert_eq!(sha, "commit-2"),
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn remaining_steps_decreases() {
        let mut state = BisectState::new(commits(16), "task-1".into());
        let initial = state.remaining_steps();
        assert_eq!(initial, 4);

        state.mark_good("commit-7");
        assert!(state.remaining_steps() < initial);
    }

    #[test]
    fn auto_bisect_finds_regression() {
        let mut state = BisectState::new(commits(8), "task-1".into());
        state.mark_good("commit-0");
        state.mark_bad("commit-7");

        let result = state.auto_bisect(0.7, |sha| {
            let idx: usize = sha.strip_prefix("commit-").unwrap().parse().unwrap();
            Some(vec![TaskScore {
                task_id: "task-1".into(),
                score: if idx < 4 { 0.9 } else { 0.3 },
            }])
        });

        match result {
            BisectStep::Found { sha } => assert_eq!(sha, "commit-4"),
            other => panic!("expected Found commit-4, got {other:?}"),
        }
    }

    #[test]
    fn auto_bisect_handles_missing_data() {
        let mut state = BisectState::new(commits(4), "task-1".into());
        state.mark_good("commit-0");
        state.mark_bad("commit-3");

        let result = state.auto_bisect(0.7, |_| None);
        assert_eq!(result, BisectStep::NeedsMoreData);
    }

    #[test]
    fn empty_commits_returns_needs_more_data() {
        let state = BisectState::new(vec![], "task-1".into());
        assert_eq!(state.next_step(), BisectStep::NeedsMoreData);
    }
}
