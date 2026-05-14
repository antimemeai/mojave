//! Serde deserialization targets that mirror the Inspect AI `EvalLog` JSON schema.
//!
//! These types are **not** part of the mojave domain model — they exist solely as
//! an intermediate representation used by [`crate::inspect::InspectAdapter`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Top-level Inspect `EvalLog` document.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InspectLog {
    /// Log format version (e.g. `"0.3"`).
    #[serde(default)]
    pub version: Option<String>,

    /// The eval specification block.
    pub eval: InspectEvalSpec,

    /// Sampling configuration.
    #[serde(default)]
    pub plan: Option<InspectPlan>,

    /// Individual evaluation samples. May be absent if the log was truncated.
    #[serde(default)]
    pub samples: Option<Vec<InspectSample>>,

    /// Top-level eval metadata (merged into every `TrialRecord.metadata`).
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// `eval` block inside an Inspect log.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InspectEvalSpec {
    /// Opaque run identifier (string). Used to derive `run_id`.
    #[serde(default)]
    pub run_id: Option<String>,

    /// Task identifier (e.g. `"mmlu"`).
    #[serde(default)]
    pub task: Option<String>,

    /// Task ID field (alternate key used in some log versions).
    #[serde(default)]
    pub task_id: Option<String>,

    /// Model under evaluation (e.g. `"openai/gpt-4o"`).
    #[serde(default)]
    pub model: Option<String>,

    /// Dataset info embedded in the eval spec.
    #[serde(default)]
    pub dataset: Option<serde_json::Value>,

    /// Revision info for the task / dataset.
    #[serde(default)]
    pub revision: Option<InspectRevision>,

    /// Generate config used for the eval.
    #[serde(default)]
    pub config: Option<InspectGenerateConfig>,

    /// Eval-level metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// Revision metadata (git commit, branch, etc.).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InspectRevision {
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub origin: Option<String>,
}

/// Model generation configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InspectGenerateConfig {
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub top_p: Option<f32>,
}

/// Inspect `Plan` block (describes the solver pipeline).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InspectPlan {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub steps: Vec<serde_json::Value>,
    #[serde(default)]
    pub finish: Option<serde_json::Value>,
}

/// A single evaluated sample inside an Inspect log.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InspectSample {
    /// Sample identifier — may be a string or an integer in the JSON.
    pub id: serde_json::Value,

    /// UUID assigned by Inspect ≥ 0.3.70 (absent in older logs).
    #[serde(default)]
    pub uuid: Option<String>,

    /// Epoch number (0-indexed repetition index).
    #[serde(default)]
    pub epoch: Option<u32>,

    /// The prompt input to the model.
    #[serde(default)]
    pub input: Option<serde_json::Value>,

    /// Model messages / conversation turns.
    #[serde(default)]
    pub messages: Vec<InspectGradingMessage>,

    /// Per-scorer results. Keys are scorer names.
    #[serde(default)]
    pub scores: Option<BTreeMap<String, InspectScore>>,

    /// When the sample started (RFC 3339).
    #[serde(default)]
    pub started_at: Option<String>,

    /// When the sample completed (RFC 3339).
    #[serde(default)]
    pub completed_at: Option<String>,

    /// Per-sample metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// Score produced by a single scorer for a single sample.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InspectScore {
    /// Raw score value — one of:
    /// - `"C"` / `"I"` / `"N"` / `"P"` (string letter grades)
    /// - `true` / `false` (boolean)
    /// - `0` / `1` (integer aliases for binary)
    /// - `2..=255` (integer grade)
    /// - float (continuous score)
    /// - object (multi-criterion map)
    pub value: serde_json::Value,

    /// Human-readable explanation of the score.
    #[serde(default)]
    pub explanation: Option<String>,

    /// Scorer-level metadata (may contain grading provenance under key `"grading"`).
    #[serde(default)]
    pub metadata: Option<BTreeMap<String, serde_json::Value>>,
}

/// A single message in a grading conversation (used to extract `JudgeConfig`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InspectGradingMessage {
    /// `"user"`, `"assistant"`, or `"system"`.
    #[serde(default)]
    pub role: Option<String>,

    /// Text content of the message.
    #[serde(default)]
    pub content: Option<serde_json::Value>,
}
