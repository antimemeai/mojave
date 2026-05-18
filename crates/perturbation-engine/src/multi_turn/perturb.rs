use rand::{seq::SliceRandom as _, SeedableRng as _};
use rand_chacha::ChaCha20Rng;

use super::atoms::{ConversationMessage, MultiTurnAtom, MultiTurnPlan, MultiTurnValidationError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedPerturbation {
    pub atom: MultiTurnAtom,
    pub original_history: Vec<ConversationMessage>,
    pub perturbed_history: Vec<ConversationMessage>,
    pub signed_confound: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PerturbationError {
    #[error("multi-turn plan failed validation: {0}")]
    Validation(#[from] MultiTurnValidationError),
}

#[must_use = "returns the perturbed conversation history; discarding it loses the perturbation"]
pub fn apply(plan: &MultiTurnPlan, seed: u64) -> Result<AppliedPerturbation, PerturbationError> {
    plan.validate()?;

    let original_history = plan.history().to_vec();
    let perturbed_history = match plan {
        MultiTurnPlan::Original { history } => history.clone(),
        MultiTurnPlan::TruncateEarly { history, n } => history[*n..].to_vec(),
        MultiTurnPlan::Reorder { history } => {
            let mut out = history.clone();
            let mut rng = ChaCha20Rng::seed_from_u64(seed);
            out.shuffle(&mut rng);
            if out == *history && out.len() > 1 {
                out.rotate_left(1);
            }
            out
        }
        MultiTurnPlan::Inject {
            history,
            position,
            turn,
        } => {
            let mut out = history.clone();
            out.insert(
                *position,
                ConversationMessage {
                    role: turn.role.clone(),
                    content: turn.content.clone(),
                },
            );
            out
        }
    };

    Ok(AppliedPerturbation {
        atom: plan.atom(),
        original_history,
        perturbed_history,
        signed_confound: plan.signed_confound(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_turn::{ConversationMessage, InjectTurn, MultiTurnPlan};

    fn history() -> Vec<ConversationMessage> {
        vec![
            ConversationMessage {
                role: "system".into(),
                content: "S".into(),
            },
            ConversationMessage {
                role: "user".into(),
                content: "U1".into(),
            },
            ConversationMessage {
                role: "assistant".into(),
                content: "A1".into(),
            },
            ConversationMessage {
                role: "user".into(),
                content: "U2".into(),
            },
        ]
    }

    #[test]
    fn original_preserves_history() {
        let applied = apply(&MultiTurnPlan::Original { history: history() }, 7).unwrap();
        assert_eq!(applied.original_history, applied.perturbed_history);
        assert_eq!(applied.atom, MultiTurnAtom::Original);
    }

    #[test]
    fn truncate_early_drops_first_n_turns() {
        let applied = apply(
            &MultiTurnPlan::TruncateEarly {
                history: history(),
                n: 2,
            },
            7,
        )
        .unwrap();
        assert_eq!(applied.perturbed_history.len(), 2);
        assert_eq!(applied.perturbed_history[0].content, "A1");
    }

    #[test]
    fn reorder_shuffles_but_preserves_messages() {
        let applied = apply(&MultiTurnPlan::Reorder { history: history() }, 7).unwrap();
        assert_eq!(applied.perturbed_history.len(), 4);
        let mut original: Vec<_> = applied
            .original_history
            .iter()
            .map(|m| m.content.clone())
            .collect();
        let mut perturbed: Vec<_> = applied
            .perturbed_history
            .iter()
            .map(|m| m.content.clone())
            .collect();
        original.sort();
        perturbed.sort();
        assert_eq!(original, perturbed);
    }

    #[test]
    fn reorder_is_deterministic() {
        let plan = MultiTurnPlan::Reorder { history: history() };
        let a = apply(&plan, 42).unwrap();
        let b = apply(&plan, 42).unwrap();
        assert_eq!(a.perturbed_history, b.perturbed_history);
    }

    #[test]
    fn inject_adds_turn_at_position() {
        let applied = apply(
            &MultiTurnPlan::Inject {
                history: history(),
                position: 1,
                turn: InjectTurn {
                    role: "user".into(),
                    content: "Injected".into(),
                },
            },
            7,
        )
        .unwrap();
        assert_eq!(applied.perturbed_history.len(), 5);
        assert_eq!(applied.perturbed_history[1].content, "Injected");
    }

    #[test]
    fn inject_at_end() {
        let h = history();
        let len = h.len();
        let applied = apply(
            &MultiTurnPlan::Inject {
                history: h,
                position: len,
                turn: InjectTurn {
                    role: "user".into(),
                    content: "Tail".into(),
                },
            },
            7,
        )
        .unwrap();
        assert_eq!(applied.perturbed_history.last().unwrap().content, "Tail");
    }

    #[test]
    fn all_perturbations_are_signed_confounds() {
        let applied = apply(
            &MultiTurnPlan::TruncateEarly {
                history: history(),
                n: 1,
            },
            7,
        )
        .unwrap();
        assert!(applied.signed_confound);
    }

    #[test]
    fn invalid_plan_returns_error() {
        let result = apply(
            &MultiTurnPlan::TruncateEarly {
                history: Vec::new(),
                n: 1,
            },
            7,
        );
        assert!(result.is_err());
    }
}
