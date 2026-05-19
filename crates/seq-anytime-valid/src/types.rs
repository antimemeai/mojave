use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Reject,
    Accept,
    Continue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSnapshot {
    pub log_likelihood_ratio: f64,
    pub n_observations: usize,
    pub always_valid_p: Option<f64>,
    pub confidence_interval: Option<(f64, f64)>,
    pub e_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprtConfig {
    pub theta_0: f64,
    pub theta_1: f64,
    pub alpha: f64,
    pub beta: f64,
    pub variant: SprtVariant,
    pub family: DataFamily,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SprtVariant {
    Approximate,
    Conservative,
    Boosted,
}

/// Exponential family for sequential test statistics.
///
/// Note: `DataFamily` deliberately does not implement `Eq` because the
/// `Normal` variant contains an `Option<f64>`, and `f64` is not `Eq`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DataFamily {
    Bernoulli,
    Normal { known_variance: Option<f64> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupSeqConfig {
    pub total_looks: usize,
    pub alpha: f64,
    pub beta: f64,
    pub boundary_type: BoundaryType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BoundaryType {
    Pocock,
    OBrienFleming,
    LanDeMets(SpendingFunctionType),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpendingFunctionType {
    PocockType,
    OBrienFlemingType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsprtConfig {
    pub theta_0: f64,
    pub mixing_variance: f64,
    pub family: DataFamily,
    pub max_samples: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BernoulliMsprtConfig {
    pub p0: f64,
    pub beta_a: f64,
    pub beta_b: f64,
    pub max_samples: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfSeqConfig {
    pub alpha: f64,
    pub bound_type: ConfSeqBoundType,
}

/// Confidence-sequence bound type.
///
/// Note: `ConfSeqBoundType` deliberately does not implement `Eq` because
/// `SubGaussian` contains an `f64` field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ConfSeqBoundType {
    NormalMixture,
    SubGaussian { bound: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EValueConfig {
    pub theta_0: f64,
    pub family: DataFamily,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasConfig {
    pub theta_0: f64,
    pub theta_1: f64,
    pub alpha: f64,
    pub beta: f64,
    pub family: DataFamily,
}
