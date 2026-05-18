use rand::{Rng as _, SeedableRng as _};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Separator {
    ColonSpace,
    Newline,
    ArrowSpace,
}

impl Separator {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ColonSpace => ": ",
            Self::Newline => "\n",
            Self::ArrowSpace => "-> ",
        }
    }

    #[must_use]
    pub fn factor_str(self) -> &'static str {
        match self {
            Self::ColonSpace => "colon-space",
            Self::Newline => "newline",
            Self::ArrowSpace => "arrow-space",
        }
    }

    const COUNT: u32 = 3;

    fn from_index(i: u32) -> Self {
        match i % Self::COUNT {
            0 => Self::ColonSpace,
            1 => Self::Newline,
            _ => Self::ArrowSpace,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Casing {
    Original,
    Upper,
    Lower,
}

impl Casing {
    #[must_use]
    pub fn apply(self, label: &str) -> String {
        match self {
            Self::Original => label.to_owned(),
            Self::Upper => label.to_ascii_uppercase(),
            Self::Lower => label.to_ascii_lowercase(),
        }
    }

    #[must_use]
    pub fn factor_str(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Upper => "upper",
            Self::Lower => "lower",
        }
    }

    const COUNT: u32 = 3;

    fn from_index(i: u32) -> Self {
        match i % Self::COUNT {
            0 => Self::Original,
            1 => Self::Upper,
            _ => Self::Lower,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Punctuation {
    Question,
    Period,
    None,
}

impl Punctuation {
    #[must_use]
    pub fn apply(self, s: &str) -> String {
        let trimmed = s.trim_end_matches(['?', '.', '!']);
        let mut out = trimmed.to_owned();
        match self {
            Self::Question => out.push('?'),
            Self::Period => out.push('.'),
            Self::None => {}
        }
        out
    }

    #[must_use]
    pub fn factor_str(self) -> &'static str {
        match self {
            Self::Question => "question",
            Self::Period => "period",
            Self::None => "none",
        }
    }

    const COUNT: u32 = 3;

    fn from_index(i: u32) -> Self {
        match i % Self::COUNT {
            0 => Self::Question,
            1 => Self::Period,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Padding {
    Original,
    QuotesEnclose,
    NewlinesPrepend,
    NewlinesAppend,
    NewlinesBoth,
}

impl Padding {
    #[must_use]
    pub fn apply(self, s: &str) -> String {
        match self {
            Self::Original => s.to_owned(),
            Self::QuotesEnclose => format!("\"{s}\""),
            Self::NewlinesPrepend => format!("\n\n{s}"),
            Self::NewlinesAppend => format!("{s}\n\n"),
            Self::NewlinesBoth => format!("\n\n{s}\n\n"),
        }
    }

    #[must_use]
    pub fn factor_str(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::QuotesEnclose => "quotes-enclose",
            Self::NewlinesPrepend => "newlines-prepend",
            Self::NewlinesAppend => "newlines-append",
            Self::NewlinesBoth => "newlines-both",
        }
    }

    const COUNT: u32 = 5;

    fn from_index(i: u32) -> Self {
        match i % Self::COUNT {
            0 => Self::Original,
            1 => Self::QuotesEnclose,
            2 => Self::NewlinesPrepend,
            3 => Self::NewlinesAppend,
            _ => Self::NewlinesBoth,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FormatAtoms {
    pub separator: Separator,
    pub casing: Casing,
    pub punctuation: Punctuation,
    pub padding: Padding,
}

impl FormatAtoms {
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        let mut rng = ChaCha20Rng::seed_from_u64(seed);
        let s_idx: u32 = rng.random_range(0..Separator::COUNT);
        let c_idx: u32 = rng.random_range(0..Casing::COUNT);
        let p_idx: u32 = rng.random_range(0..Punctuation::COUNT);
        let pad_idx: u32 = rng.random_range(0..Padding::COUNT);
        Self {
            separator: Separator::from_index(s_idx),
            casing: Casing::from_index(c_idx),
            punctuation: Punctuation::from_index(p_idx),
            padding: Padding::from_index(pad_idx),
        }
    }

    #[must_use]
    pub fn signed_confound(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_seed_is_deterministic() {
        let a = FormatAtoms::from_seed(12345);
        let b = FormatAtoms::from_seed(12345);
        assert_eq!(a, b);
    }

    #[test]
    fn from_seed_varies_with_seed() {
        let a = FormatAtoms::from_seed(1);
        let b = FormatAtoms::from_seed(2);
        assert_ne!(a, b);
    }

    #[test]
    fn separator_factor_str_is_kebab_case() {
        assert_eq!(Separator::ColonSpace.factor_str(), "colon-space");
        assert_eq!(Separator::Newline.factor_str(), "newline");
        assert_eq!(Separator::ArrowSpace.factor_str(), "arrow-space");
    }

    #[test]
    fn casing_apply_basic() {
        assert_eq!(Casing::Original.apply("Question"), "Question");
        assert_eq!(Casing::Upper.apply("Question"), "QUESTION");
        assert_eq!(Casing::Lower.apply("Question"), "question");
    }

    #[test]
    fn punctuation_apply_strips_then_appends() {
        assert_eq!(Punctuation::Period.apply("hi?"), "hi.");
        assert_eq!(Punctuation::Question.apply("hi."), "hi?");
        assert_eq!(Punctuation::None.apply("hi?"), "hi");
        assert_eq!(Punctuation::None.apply("hi"), "hi");
    }

    #[test]
    fn padding_apply_variants() {
        assert_eq!(Padding::Original.apply("hi"), "hi");
        assert_eq!(Padding::QuotesEnclose.apply("hi"), "\"hi\"");
        assert_eq!(Padding::NewlinesPrepend.apply("hi"), "\n\nhi");
        assert_eq!(Padding::NewlinesAppend.apply("hi"), "hi\n\n");
        assert_eq!(Padding::NewlinesBoth.apply("hi"), "\n\nhi\n\n");
    }

    #[test]
    fn signed_confound_is_false_for_format_atoms() {
        let atoms = FormatAtoms::from_seed(42);
        assert!(!atoms.signed_confound());
    }
}
