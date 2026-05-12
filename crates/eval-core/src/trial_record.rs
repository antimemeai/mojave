use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ulid::Ulid;

use crate::judge_config::JudgeConfig;
use crate::outcome::Outcome;

pub type TrialId = Ulid;
pub type RunId = Ulid;
pub type TaskId = String;
pub type AgentId = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrialRecord {
    pub trial_id: TrialId,
    pub run_id: RunId,
    pub task_id: TaskId,
    pub task_version: Option<String>,
    pub agent_id: AgentId,
    pub agent_version: Option<String>,
    pub judge_config: Option<JudgeConfig>,
    pub seed: Option<u64>,
    /// Seconds since Unix epoch (UTC).
    pub timestamp: i64,
    pub outcome: Outcome,
    pub metadata: BTreeMap<String, serde_json::Value>,
}
