use audit_events::ValidationError;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AuditError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("chain error: {0}")]
    Chain(#[from] audit_chain::seal::SealError),
    #[error("canonical encoding error: {0}")]
    Canonical(#[from] audit_chain::canonical::CanonicalEncodingError),
    #[error("replay error: {0}")]
    Replay(#[from] audit_recover::replay::ReplayError),
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),
    #[error("blob store error: {0}")]
    BlobStore(String),
    #[error("circuit breaker: audit is bypassed")]
    CircuitBreakerActive,
    #[error("detail exceeds max size: {size} bytes (max {max})")]
    DetailTooLarge { size: usize, max: usize },
}
