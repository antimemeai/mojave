use chrono::{DateTime, Utc};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeRecord {
    pub sha: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub files_changed: Vec<FileChange>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileChange {
    pub path: String,
    pub kind: FileChangeKind,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FileChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}

impl ChangeRecord {
    pub fn total_additions(&self) -> u32 {
        self.files_changed.iter().map(|f| f.additions).sum()
    }

    pub fn total_deletions(&self) -> u32 {
        self.files_changed.iter().map(|f| f.deletions).sum()
    }

    pub fn total_churn(&self) -> u32 {
        self.total_additions() + self.total_deletions()
    }

    pub fn changed_paths(&self) -> Vec<&str> {
        self.files_changed.iter().map(|f| f.path.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample_change() -> ChangeRecord {
        ChangeRecord {
            sha: "abc1234".into(),
            author: "dev".into(),
            timestamp: Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap(),
            message: "fix: handle edge case".into(),
            files_changed: vec![
                FileChange {
                    path: "src/engine.rs".into(),
                    kind: FileChangeKind::Modified,
                    additions: 10,
                    deletions: 3,
                },
                FileChange {
                    path: "tests/engine_test.rs".into(),
                    kind: FileChangeKind::Added,
                    additions: 25,
                    deletions: 0,
                },
            ],
        }
    }

    #[test]
    fn total_additions() {
        assert_eq!(sample_change().total_additions(), 35);
    }

    #[test]
    fn total_deletions() {
        assert_eq!(sample_change().total_deletions(), 3);
    }

    #[test]
    fn total_churn() {
        assert_eq!(sample_change().total_churn(), 38);
    }

    #[test]
    fn changed_paths() {
        let change = sample_change();
        let paths = change.changed_paths();
        assert_eq!(paths, vec!["src/engine.rs", "tests/engine_test.rs"]);
    }

    #[test]
    fn serde_round_trip() {
        let change = sample_change();
        let json = serde_json::to_string(&change).unwrap();
        let back: ChangeRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sha, "abc1234");
        assert_eq!(back.files_changed.len(), 2);
    }
}
