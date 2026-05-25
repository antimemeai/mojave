#![allow(clippy::module_name_repetitions)]

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    // Eval lifecycle
    EvalStarted,
    EvalCompleted,
    EvalFailed,

    // Infrastructure
    PodCreated,
    PodReady,
    PodTerminated,
    EndpointVerified,

    // Data provenance
    DatasetLoaded,
    DatasetCached,
    ModelLoaded,
    ScoringCompleted,

    // Artifacts
    RunCardGenerated,
    RunCardSealed,

    // Crypto operations
    KeyGenerated,
    KeyLoaded,
    ChainVerified,
    ChainGenesis,
    AttestationCreated,

    // System
    ConfigChanged,
    CircuitBreakerTripped,
    CircuitBreakerReset,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EvalStarted => "eval.started",
            Self::EvalCompleted => "eval.completed",
            Self::EvalFailed => "eval.failed",
            Self::PodCreated => "pod.created",
            Self::PodReady => "pod.ready",
            Self::PodTerminated => "pod.terminated",
            Self::EndpointVerified => "endpoint.verified",
            Self::DatasetLoaded => "dataset.loaded",
            Self::DatasetCached => "dataset.cached",
            Self::ModelLoaded => "model.loaded",
            Self::ScoringCompleted => "scoring.completed",
            Self::RunCardGenerated => "run_card.generated",
            Self::RunCardSealed => "run_card.sealed",
            Self::KeyGenerated => "key.generated",
            Self::KeyLoaded => "key.loaded",
            Self::ChainVerified => "chain.verified",
            Self::ChainGenesis => "chain.genesis",
            Self::AttestationCreated => "attestation.created",
            Self::ConfigChanged => "config.changed",
            Self::CircuitBreakerTripped => "circuit_breaker.tripped",
            Self::CircuitBreakerReset => "circuit_breaker.reset",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "eval.started" => Some(Self::EvalStarted),
            "eval.completed" => Some(Self::EvalCompleted),
            "eval.failed" => Some(Self::EvalFailed),
            "pod.created" => Some(Self::PodCreated),
            "pod.ready" => Some(Self::PodReady),
            "pod.terminated" => Some(Self::PodTerminated),
            "endpoint.verified" => Some(Self::EndpointVerified),
            "dataset.loaded" => Some(Self::DatasetLoaded),
            "dataset.cached" => Some(Self::DatasetCached),
            "model.loaded" => Some(Self::ModelLoaded),
            "scoring.completed" => Some(Self::ScoringCompleted),
            "run_card.generated" => Some(Self::RunCardGenerated),
            "run_card.sealed" => Some(Self::RunCardSealed),
            "key.generated" => Some(Self::KeyGenerated),
            "key.loaded" => Some(Self::KeyLoaded),
            "chain.verified" => Some(Self::ChainVerified),
            "chain.genesis" => Some(Self::ChainGenesis),
            "attestation.created" => Some(Self::AttestationCreated),
            "config.changed" => Some(Self::ConfigChanged),
            "circuit_breaker.tripped" => Some(Self::CircuitBreakerTripped),
            "circuit_breaker.reset" => Some(Self::CircuitBreakerReset),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::EvalStarted,
            Self::EvalCompleted,
            Self::EvalFailed,
            Self::PodCreated,
            Self::PodReady,
            Self::PodTerminated,
            Self::EndpointVerified,
            Self::DatasetLoaded,
            Self::DatasetCached,
            Self::ModelLoaded,
            Self::ScoringCompleted,
            Self::RunCardGenerated,
            Self::RunCardSealed,
            Self::KeyGenerated,
            Self::KeyLoaded,
            Self::ChainVerified,
            Self::ChainGenesis,
            Self::AttestationCreated,
            Self::ConfigChanged,
            Self::CircuitBreakerTripped,
            Self::CircuitBreakerReset,
        ]
    }
}

impl std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for EventKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for EventKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("unknown event kind: {s}")))
    }
}

impl std::str::FromStr for EventKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("unknown event kind: {s}"))
    }
}
