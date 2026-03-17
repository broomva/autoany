use crate::error::{EgriError, Result};
use crate::types::{Action, Decision, StateId};

/// Manages artifact state promotion and rollback.
pub struct PromotionController<A: Clone> {
    best_artifact: Option<A>,
    best_state_id: Option<StateId>,
    current_artifact: Option<A>,
    current_state_id: Option<StateId>,
}

impl<A: Clone> PromotionController<A> {
    pub fn new() -> Self {
        Self {
            best_artifact: None,
            best_state_id: None,
            current_artifact: None,
            current_state_id: None,
        }
    }

    /// Set the baseline artifact. Must be called before the loop.
    pub fn set_baseline(&mut self, artifact: A) {
        let state_id = StateId::baseline();
        self.best_artifact = Some(artifact.clone());
        self.best_state_id = Some(state_id.clone());
        self.current_artifact = Some(artifact);
        self.current_state_id = Some(state_id);
    }

    /// Apply a decision: promote or discard the candidate.
    pub fn apply_decision(&mut self, decision: &Decision, candidate: A) {
        match decision.action {
            Action::Promoted => {
                let state_id = decision.new_state_id.clone().unwrap_or_default();
                self.best_artifact = Some(candidate.clone());
                self.best_state_id = Some(state_id.clone());
                self.current_artifact = Some(candidate);
                self.current_state_id = Some(state_id);
            }
            Action::Discarded | Action::Escalated => {
                // Restore current to best
                if let Some(best) = &self.best_artifact {
                    self.current_artifact = Some(best.clone());
                    self.current_state_id = self.best_state_id.clone();
                }
            }
            Action::Branched => {
                // Keep current as-is for branching — caller handles branch logic
            }
        }
    }

    /// Rollback to the last promoted state.
    pub fn rollback(&mut self) -> Result<&A> {
        match &self.best_artifact {
            Some(artifact) => {
                self.current_artifact = Some(artifact.clone());
                self.current_state_id = self.best_state_id.clone();
                Ok(self.current_artifact.as_ref().unwrap())
            }
            None => Err(EgriError::RollbackFailed),
        }
    }

    pub fn current(&self) -> Option<&A> {
        self.current_artifact.as_ref()
    }

    pub fn best(&self) -> Option<&A> {
        self.best_artifact.as_ref()
    }

    pub fn current_state_id(&self) -> Option<&StateId> {
        self.current_state_id.as_ref()
    }

    pub fn best_state_id(&self) -> Option<&StateId> {
        self.best_state_id.as_ref()
    }
}

impl<A: Clone> Default for PromotionController<A> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::StateId;

    #[test]
    fn baseline_sets_both_current_and_best() {
        let mut pc: PromotionController<String> = PromotionController::new();
        assert!(pc.current().is_none());
        assert!(pc.best().is_none());

        pc.set_baseline("v0".into());
        assert_eq!(pc.current(), Some(&"v0".into()));
        assert_eq!(pc.best(), Some(&"v0".into()));
        assert_eq!(pc.current_state_id().unwrap().0, "baseline");
    }

    #[test]
    fn promote_updates_both() {
        let mut pc: PromotionController<String> = PromotionController::new();
        pc.set_baseline("v0".into());

        let decision = Decision {
            action: Action::Promoted,
            reason: "improved".into(),
            new_state_id: Some(StateId("s1".into())),
        };
        pc.apply_decision(&decision, "v1".into());

        assert_eq!(pc.current(), Some(&"v1".into()));
        assert_eq!(pc.best(), Some(&"v1".into()));
        assert_eq!(pc.best_state_id().unwrap().0, "s1");
    }

    #[test]
    fn discard_restores_to_best() {
        let mut pc: PromotionController<String> = PromotionController::new();
        pc.set_baseline("v0".into());

        let decision = Decision {
            action: Action::Discarded,
            reason: "no improvement".into(),
            new_state_id: None,
        };
        pc.apply_decision(&decision, "v_bad".into());

        assert_eq!(pc.current(), Some(&"v0".into()));
        assert_eq!(pc.best(), Some(&"v0".into()));
    }

    #[test]
    fn rollback_returns_best() {
        let mut pc: PromotionController<String> = PromotionController::new();
        pc.set_baseline("v0".into());

        let promote = Decision {
            action: Action::Promoted,
            reason: "better".into(),
            new_state_id: Some(StateId("s1".into())),
        };
        pc.apply_decision(&promote, "v1".into());

        let rolled = pc.rollback().unwrap();
        assert_eq!(rolled, &"v1".to_string());
    }

    #[test]
    fn rollback_without_baseline_fails() {
        let mut pc: PromotionController<String> = PromotionController::new();
        assert!(pc.rollback().is_err());
    }
}
