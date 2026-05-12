use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JudgeConfig {
    pub model: String,
    pub family: String,
    pub prompt_template_hash: String,
    pub temperature: f32,
    pub seed: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
#[error("temperature must be finite, got {0}")]
pub struct JudgeConfigError(pub f32);

impl JudgeConfig {
    pub fn new(
        model: String,
        family: String,
        prompt_template_hash: String,
        temperature: f32,
        seed: Option<u64>,
    ) -> Result<Self, JudgeConfigError> {
        if !temperature.is_finite() {
            return Err(JudgeConfigError(temperature));
        }
        Ok(Self {
            model,
            family,
            prompt_template_hash,
            temperature,
            seed,
        })
    }
}
