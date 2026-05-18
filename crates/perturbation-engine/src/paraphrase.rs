use rand::{Rng as _, SeedableRng as _};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ParaphraseModel {
    Mini,
    Standard,
    Frontier,
}

impl ParaphraseModel {
    #[must_use]
    pub fn factor_str(self) -> &'static str {
        match self {
            Self::Mini => "mini",
            Self::Standard => "standard",
            Self::Frontier => "frontier",
        }
    }

    const COUNT: u32 = 3;

    fn from_index(i: u32) -> Self {
        match i % Self::COUNT {
            0 => Self::Mini,
            1 => Self::Standard,
            _ => Self::Frontier,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ParaphraseStrength {
    Mild,
    Moderate,
    Aggressive,
}

impl ParaphraseStrength {
    #[must_use]
    pub fn factor_str(self) -> &'static str {
        match self {
            Self::Mild => "mild",
            Self::Moderate => "moderate",
            Self::Aggressive => "aggressive",
        }
    }

    const COUNT: u32 = 3;

    fn from_index(i: u32) -> Self {
        match i % Self::COUNT {
            0 => Self::Mild,
            1 => Self::Moderate,
            _ => Self::Aggressive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ParaphraseAtoms {
    pub model: ParaphraseModel,
    pub strength: ParaphraseStrength,
}

impl ParaphraseAtoms {
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        let mut rng = ChaCha20Rng::seed_from_u64(seed);
        let model_idx: u32 = rng.random_range(0..ParaphraseModel::COUNT);
        let strength_idx: u32 = rng.random_range(0..ParaphraseStrength::COUNT);
        Self {
            model: ParaphraseModel::from_index(model_idx),
            strength: ParaphraseStrength::from_index(strength_idx),
        }
    }

    #[must_use]
    pub fn signed_confound(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_seed_is_deterministic() {
        assert_eq!(ParaphraseAtoms::from_seed(7), ParaphraseAtoms::from_seed(7));
    }

    #[test]
    fn from_seed_varies_with_seed() {
        let a = ParaphraseAtoms::from_seed(1);
        let b = ParaphraseAtoms::from_seed(2);
        assert_ne!(a, b);
    }

    #[test]
    fn factor_strings_are_stable() {
        assert_eq!(ParaphraseModel::Mini.factor_str(), "mini");
        assert_eq!(ParaphraseModel::Standard.factor_str(), "standard");
        assert_eq!(ParaphraseModel::Frontier.factor_str(), "frontier");
        assert_eq!(ParaphraseStrength::Mild.factor_str(), "mild");
        assert_eq!(ParaphraseStrength::Moderate.factor_str(), "moderate");
        assert_eq!(ParaphraseStrength::Aggressive.factor_str(), "aggressive");
    }

    #[test]
    fn signed_confound_is_true() {
        let atoms = ParaphraseAtoms::from_seed(42);
        assert!(atoms.signed_confound());
    }
}
