use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjectTurn {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum MultiTurnAtom {
    Original,
    TruncateEarly,
    Reorder,
    Inject,
}

impl MultiTurnAtom {
    #[must_use]
    pub fn factor_str(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::TruncateEarly => "truncate-early",
            Self::Reorder => "reorder",
            Self::Inject => "inject",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "atom", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum MultiTurnPlan {
    Original {
        history: Vec<ConversationMessage>,
    },
    TruncateEarly {
        history: Vec<ConversationMessage>,
        n: usize,
    },
    Reorder {
        history: Vec<ConversationMessage>,
    },
    Inject {
        history: Vec<ConversationMessage>,
        position: usize,
        turn: InjectTurn,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MultiTurnValidationError {
    #[error("multi-turn plan requires a non-empty history")]
    EmptyHistory,
    #[error("truncate-early requires n > 0 and n < history.len()")]
    InvalidTruncateCount,
    #[error("inject position must be <= history.len()")]
    InvalidInjectPosition,
    #[error("inject turn requires non-empty role and content")]
    InvalidInjectTurn,
    #[error("conversation messages require non-empty role and content")]
    InvalidMessage,
}

impl MultiTurnPlan {
    #[must_use]
    pub fn atom(&self) -> MultiTurnAtom {
        match self {
            Self::Original { .. } => MultiTurnAtom::Original,
            Self::TruncateEarly { .. } => MultiTurnAtom::TruncateEarly,
            Self::Reorder { .. } => MultiTurnAtom::Reorder,
            Self::Inject { .. } => MultiTurnAtom::Inject,
        }
    }

    #[must_use]
    pub fn history(&self) -> &[ConversationMessage] {
        match self {
            Self::Original { history }
            | Self::TruncateEarly { history, .. }
            | Self::Reorder { history }
            | Self::Inject { history, .. } => history,
        }
    }

    pub fn validate(&self) -> Result<(), MultiTurnValidationError> {
        let history = self.history();
        if history.is_empty() {
            return Err(MultiTurnValidationError::EmptyHistory);
        }
        if history
            .iter()
            .any(|m| m.role.trim().is_empty() || m.content.trim().is_empty())
        {
            return Err(MultiTurnValidationError::InvalidMessage);
        }
        match self {
            Self::Original { .. } | Self::Reorder { .. } => Ok(()),
            Self::TruncateEarly { n, history } => {
                if *n == 0 || *n >= history.len() {
                    Err(MultiTurnValidationError::InvalidTruncateCount)
                } else {
                    Ok(())
                }
            }
            Self::Inject {
                history,
                position,
                turn,
            } => {
                if *position > history.len() {
                    return Err(MultiTurnValidationError::InvalidInjectPosition);
                }
                if turn.role.trim().is_empty() || turn.content.trim().is_empty() {
                    return Err(MultiTurnValidationError::InvalidInjectTurn);
                }
                Ok(())
            }
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

    fn history() -> Vec<ConversationMessage> {
        vec![
            ConversationMessage {
                role: "system".into(),
                content: "You are a helpful assistant".into(),
            },
            ConversationMessage {
                role: "user".into(),
                content: "Enumerate the host".into(),
            },
            ConversationMessage {
                role: "assistant".into(),
                content: "I will inspect the host".into(),
            },
        ]
    }

    #[test]
    fn plan_validation_rejects_empty_history() {
        let plan = MultiTurnPlan::Reorder {
            history: Vec::new(),
        };
        assert_eq!(
            plan.validate().unwrap_err(),
            MultiTurnValidationError::EmptyHistory
        );
    }

    #[test]
    fn truncate_validation_rejects_too_large_n() {
        let plan = MultiTurnPlan::TruncateEarly {
            history: history(),
            n: 3,
        };
        assert_eq!(
            plan.validate().unwrap_err(),
            MultiTurnValidationError::InvalidTruncateCount
        );
    }

    #[test]
    fn truncate_validation_rejects_zero_n() {
        let plan = MultiTurnPlan::TruncateEarly {
            history: history(),
            n: 0,
        };
        assert_eq!(
            plan.validate().unwrap_err(),
            MultiTurnValidationError::InvalidTruncateCount
        );
    }

    #[test]
    fn inject_validation_rejects_out_of_bounds() {
        let plan = MultiTurnPlan::Inject {
            history: history(),
            position: 4,
            turn: InjectTurn {
                role: "user".into(),
                content: "hi".into(),
            },
        };
        assert_eq!(
            plan.validate().unwrap_err(),
            MultiTurnValidationError::InvalidInjectPosition
        );
    }

    #[test]
    fn inject_validation_rejects_empty_turn() {
        let plan = MultiTurnPlan::Inject {
            history: history(),
            position: 1,
            turn: InjectTurn {
                role: "user".into(),
                content: "  ".into(),
            },
        };
        assert_eq!(
            plan.validate().unwrap_err(),
            MultiTurnValidationError::InvalidInjectTurn
        );
    }

    #[test]
    fn valid_original_plan() {
        let plan = MultiTurnPlan::Original { history: history() };
        assert!(plan.validate().is_ok());
        assert_eq!(plan.atom(), MultiTurnAtom::Original);
    }

    #[test]
    fn signed_confound_is_always_true() {
        let plan = MultiTurnPlan::Original { history: history() };
        assert!(plan.signed_confound());
    }

    #[test]
    fn validation_rejects_blank_role() {
        let plan = MultiTurnPlan::Original {
            history: vec![ConversationMessage {
                role: "".into(),
                content: "hello".into(),
            }],
        };
        assert_eq!(
            plan.validate().unwrap_err(),
            MultiTurnValidationError::InvalidMessage
        );
    }
}
