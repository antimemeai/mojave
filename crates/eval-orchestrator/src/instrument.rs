use crate::config::AnalysisConfig;
use crate::types::{Decision, SeriesKey};
use eval_core::TrialRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstrumentId {
    Irr,
    Sequential,
    Spc,
}

impl InstrumentId {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "irr" => Some(Self::Irr),
            "sequential" => Some(Self::Sequential),
            "spc" => Some(Self::Spc),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Irr => "irr",
            Self::Sequential => "sequential",
            Self::Spc => "spc",
        }
    }
}

pub trait Instrument {
    fn id(&self) -> InstrumentId;

    fn analyze(
        &self,
        series: &SeriesKey,
        records: &[TrialRecord],
        config: &AnalysisConfig,
    ) -> Vec<Decision>;
}
