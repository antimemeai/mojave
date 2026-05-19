#[derive(Debug, Clone, thiserror::Error)]
pub enum SeqError {
    #[error("degenerate hypotheses: H0 and H1 specify the same parameter value")]
    DegenerateHypotheses,
    #[error("non-finite input: {0}")]
    NonFiniteInput(f64),
    #[error("alpha must be in (0, 1), got {0}")]
    InvalidAlpha(f64),
    #[error("beta must be in (0, 1), got {0}")]
    InvalidBeta(f64),
    #[error("number of looks must be >= 1, got {0}")]
    InvalidLooks(usize),
    #[error("alpha + beta must be < 1")]
    AlphaBetaSum,
    #[error("no observations provided")]
    EmptyObservations,
    #[error("look index {k} out of range [1, {total}]")]
    LookOutOfRange { k: usize, total: usize },
    #[error("mixing distribution variance must be positive, got {0}")]
    InvalidMixingVariance(f64),
    #[error("practical significance delta must be positive, got {0}")]
    InvalidPracticalDelta(f64),
    #[error("Bernoulli observation must be in [0, 1], got {0}")]
    InvalidBernoulliObservation(f64),
    #[error("null proportion p0 must be in (0, 1), got {0}")]
    InvalidNullProportion(f64),
    #[error("Beta shape parameter must be positive, got a={a}, b={b}")]
    InvalidBetaParams { a: f64, b: f64 },
}
