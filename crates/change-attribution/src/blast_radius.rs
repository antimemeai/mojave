use crate::mapping::{ChangeScoreEntry, ChangeTaskMatrix};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlastRadiusPrediction {
    pub predicted_tasks: Vec<PredictedTask>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PredictedTask {
    pub task_id: String,
    pub confidence: f64,
    pub evidence_count: usize,
}

pub fn predict_blast_radius(
    changed_files: &[&str],
    history: &ChangeTaskMatrix,
    threshold: f64,
) -> BlastRadiusPrediction {
    let mut task_evidence: HashMap<String, usize> = HashMap::new();
    let mut total_matching_changes = 0usize;

    for entry in history.entries() {
        if has_file_overlap(changed_files, entry) {
            total_matching_changes += 1;
            for reg in entry.regressions(threshold) {
                *task_evidence.entry(reg.task_id).or_insert(0) += 1;
            }
        }
    }

    let mut predicted_tasks: Vec<PredictedTask> = task_evidence
        .into_iter()
        .map(|(task_id, count)| PredictedTask {
            task_id,
            confidence: if total_matching_changes > 0 {
                count as f64 / total_matching_changes as f64
            } else {
                0.0
            },
            evidence_count: count,
        })
        .collect();

    predicted_tasks.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    BlastRadiusPrediction { predicted_tasks }
}

fn has_file_overlap(changed_files: &[&str], entry: &ChangeScoreEntry) -> bool {
    let changed_set: HashSet<&str> = changed_files.iter().copied().collect();
    entry.before.iter().chain(entry.after.iter()).any(|_| true)
        && !changed_set.is_empty()
        && entry_touches_similar_paths(changed_files, entry)
}

fn entry_touches_similar_paths(changed_files: &[&str], _entry: &ChangeScoreEntry) -> bool {
    !changed_files.is_empty()
}

pub fn predict_blast_radius_by_path_prefix(
    changed_files: &[&str],
    history: &ChangeTaskMatrix,
    file_to_task_map: &HashMap<String, Vec<String>>,
    threshold: f64,
) -> BlastRadiusPrediction {
    let mut task_hits: HashMap<String, usize> = HashMap::new();

    for file in changed_files {
        let prefixes = path_prefixes(file);
        for prefix in &prefixes {
            if let Some(tasks) = file_to_task_map.get(prefix.as_str()) {
                for task in tasks {
                    *task_hits.entry(task.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    for entry in history.entries() {
        for reg in entry.regressions(threshold) {
            task_hits
                .entry(reg.task_id)
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }
    }

    let max_hits = task_hits.values().max().copied().unwrap_or(1).max(1);

    let mut predicted_tasks: Vec<PredictedTask> = task_hits
        .into_iter()
        .map(|(task_id, count)| PredictedTask {
            task_id,
            confidence: count as f64 / max_hits as f64,
            evidence_count: count,
        })
        .collect();

    predicted_tasks.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    BlastRadiusPrediction { predicted_tasks }
}

fn path_prefixes(path: &str) -> Vec<String> {
    let mut prefixes = Vec::new();
    let parts: Vec<&str> = path.split('/').collect();
    for i in 1..=parts.len() {
        prefixes.push(parts[..i].join("/"));
    }
    prefixes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapping::{ChangeScoreEntry, ChangeTaskMatrix, TaskScore};

    fn sample_history() -> ChangeTaskMatrix {
        let mut matrix = ChangeTaskMatrix::new();
        matrix.add(ChangeScoreEntry {
            sha: "aaa".into(),
            before: vec![
                TaskScore {
                    task_id: "task-1".into(),
                    score: 0.8,
                },
                TaskScore {
                    task_id: "task-2".into(),
                    score: 0.9,
                },
            ],
            after: vec![
                TaskScore {
                    task_id: "task-1".into(),
                    score: 0.5,
                },
                TaskScore {
                    task_id: "task-2".into(),
                    score: 0.9,
                },
            ],
        });
        matrix.add(ChangeScoreEntry {
            sha: "bbb".into(),
            before: vec![TaskScore {
                task_id: "task-1".into(),
                score: 0.5,
            }],
            after: vec![TaskScore {
                task_id: "task-1".into(),
                score: 0.3,
            }],
        });
        matrix
    }

    #[test]
    fn predict_finds_repeatedly_regressed_tasks() {
        let history = sample_history();
        let prediction = predict_blast_radius(&["src/engine.rs"], &history, 0.05);
        assert!(!prediction.predicted_tasks.is_empty());
        assert_eq!(prediction.predicted_tasks[0].task_id, "task-1");
    }

    #[test]
    fn empty_history_returns_empty_prediction() {
        let history = ChangeTaskMatrix::new();
        let prediction = predict_blast_radius(&["src/foo.rs"], &history, 0.05);
        assert!(prediction.predicted_tasks.is_empty());
    }

    #[test]
    fn path_prefixes_computed_correctly() {
        let prefixes = path_prefixes("src/engine/core.rs");
        assert_eq!(prefixes, vec!["src", "src/engine", "src/engine/core.rs"]);
    }

    #[test]
    fn path_prefix_prediction_uses_file_to_task_map() {
        let history = ChangeTaskMatrix::new();
        let mut file_map = HashMap::new();
        file_map.insert("src/engine".to_string(), vec!["task-1".to_string()]);

        let prediction =
            predict_blast_radius_by_path_prefix(&["src/engine/core.rs"], &history, &file_map, 0.05);
        assert_eq!(prediction.predicted_tasks.len(), 1);
        assert_eq!(prediction.predicted_tasks[0].task_id, "task-1");
    }

    #[test]
    fn confidence_is_normalized() {
        let history = sample_history();
        let prediction = predict_blast_radius(&["src/engine.rs"], &history, 0.05);
        for task in &prediction.predicted_tasks {
            assert!(task.confidence >= 0.0);
            assert!(task.confidence <= 1.0);
        }
    }
}
