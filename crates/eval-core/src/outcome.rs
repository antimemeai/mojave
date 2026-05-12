use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Outcome {
    Binary(bool),
    Score(f64),
    Graded(u8),
    MultiCriterion(BTreeMap<String, f64>),
}

#[derive(Debug, thiserror::Error)]
pub enum OutcomeError {
    #[error("score must be finite, got {0}")]
    NonFiniteScore(f64),
    #[error("multi-criterion value for {key} must be finite, got {value}")]
    NonFiniteCriterion { key: String, value: f64 },
}

impl Outcome {
    pub fn score(value: f64) -> Result<Self, OutcomeError> {
        if !value.is_finite() {
            return Err(OutcomeError::NonFiniteScore(value));
        }
        Ok(Self::Score(value))
    }

    pub fn multi_criterion(criteria: BTreeMap<String, f64>) -> Result<Self, OutcomeError> {
        for (key, value) in &criteria {
            if !value.is_finite() {
                return Err(OutcomeError::NonFiniteCriterion {
                    key: key.clone(),
                    value: *value,
                });
            }
        }
        Ok(Self::MultiCriterion(criteria))
    }
}
