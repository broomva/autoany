use crate::error::Result;
use crate::spec::PromotionPolicy;
use crate::types::{Action, Decision, Direction, Outcome, Score, StateId};

/// Decides whether to promote, discard, branch, or escalate a candidate.
pub trait Selector {
    fn select(&self, candidate_score: &Outcome, best_score: &Outcome) -> Result<Decision>;
}

/// Default selector implementing standard promotion policies.
pub struct DefaultSelector {
    pub policy: PromotionPolicy,
    pub direction: Direction,
    pub threshold: Option<f64>,
}

impl DefaultSelector {
    pub fn new(policy: PromotionPolicy, direction: Direction, threshold: Option<f64>) -> Self {
        Self {
            policy,
            direction,
            threshold,
        }
    }

    fn is_improvement(&self, candidate: f64, best: f64) -> bool {
        let improved = match self.direction {
            Direction::Maximize => candidate > best,
            Direction::Minimize => candidate < best,
        };

        if let Some(threshold) = self.threshold {
            let delta = (candidate - best).abs();
            improved && delta >= threshold
        } else {
            improved
        }
    }
}

impl Selector for DefaultSelector {
    fn select(&self, candidate: &Outcome, best: &Outcome) -> Result<Decision> {
        // Constraint check always comes first
        if !candidate.constraints_passed {
            return Ok(Decision {
                action: Action::Discarded,
                reason: format!(
                    "constraint violation: {}",
                    candidate.constraint_violations.join(", ")
                ),
                new_state_id: None,
            });
        }

        match self.policy {
            PromotionPolicy::KeepIfImproves => {
                let (c_score, b_score) = match (&candidate.score, &best.score) {
                    (Score::Scalar(c), Score::Scalar(b)) => (*c, *b),
                    _ => {
                        return Ok(Decision {
                            action: Action::Escalated,
                            reason: "vector scores require Pareto or manual selection".into(),
                            new_state_id: None,
                        });
                    }
                };

                if self.is_improvement(c_score, b_score) {
                    let new_state = StateId::new();
                    Ok(Decision {
                        action: Action::Promoted,
                        reason: format!("improved {b_score:.4} -> {c_score:.4}"),
                        new_state_id: Some(new_state),
                    })
                } else {
                    Ok(Decision {
                        action: Action::Discarded,
                        reason: format!("no improvement ({c_score:.4} vs best {b_score:.4})"),
                        new_state_id: None,
                    })
                }
            }
            PromotionPolicy::Threshold => {
                let c_score = match &candidate.score {
                    Score::Scalar(c) => *c,
                    _ => {
                        return Ok(Decision {
                            action: Action::Escalated,
                            reason: "threshold policy requires scalar score".into(),
                            new_state_id: None,
                        });
                    }
                };

                let threshold = self.threshold.unwrap_or(0.0);
                let meets = match self.direction {
                    Direction::Maximize => c_score >= threshold,
                    Direction::Minimize => c_score <= threshold,
                };

                if meets {
                    Ok(Decision {
                        action: Action::Promoted,
                        reason: format!("meets threshold {threshold:.4} (score: {c_score:.4})"),
                        new_state_id: Some(StateId::new()),
                    })
                } else {
                    Ok(Decision {
                        action: Action::Discarded,
                        reason: format!("below threshold {threshold:.4} (score: {c_score:.4})"),
                        new_state_id: None,
                    })
                }
            }
            PromotionPolicy::HumanGate => Ok(Decision {
                action: Action::Escalated,
                reason: "human review required".into(),
                new_state_id: None,
            }),
            PromotionPolicy::Pareto => {
                // Pareto requires vector scores — full implementation deferred
                Ok(Decision {
                    action: Action::Escalated,
                    reason: "Pareto selection not yet implemented".into(),
                    new_state_id: None,
                })
            }
            PromotionPolicy::Comparative => {
                // Comparative evaluation is handled by DebateLoop, not DefaultSelector.
                // If this is reached, the problem is misconfigured.
                Ok(Decision {
                    action: Action::Escalated,
                    reason: "comparative policy requires DebateLoop, not EgriLoop".into(),
                    new_state_id: None,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_outcome(score: f64, passed: bool) -> Outcome {
        Outcome {
            score: Score::Scalar(score),
            constraints_passed: passed,
            constraint_violations: if passed {
                vec![]
            } else {
                vec!["violated".into()]
            },
            evaluator_metadata: None,
        }
    }

    #[test]
    fn maximize_promotes_improvement() {
        let sel = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
        let best = make_outcome(0.7, true);
        let candidate = make_outcome(0.9, true);
        let d = sel.select(&candidate, &best).unwrap();
        assert_eq!(d.action, Action::Promoted);
    }

    #[test]
    fn maximize_discards_regression() {
        let sel = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
        let best = make_outcome(0.9, true);
        let candidate = make_outcome(0.7, true);
        let d = sel.select(&candidate, &best).unwrap();
        assert_eq!(d.action, Action::Discarded);
    }

    #[test]
    fn minimize_promotes_lower() {
        let sel = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Minimize, None);
        let best = make_outcome(0.5, true);
        let candidate = make_outcome(0.3, true);
        let d = sel.select(&candidate, &best).unwrap();
        assert_eq!(d.action, Action::Promoted);
    }

    #[test]
    fn constraint_violation_always_discards() {
        let sel = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
        let best = make_outcome(0.5, true);
        let candidate = make_outcome(0.9, false); // better score but violated
        let d = sel.select(&candidate, &best).unwrap();
        assert_eq!(d.action, Action::Discarded);
    }

    #[test]
    fn threshold_with_minimum_delta() {
        let sel = DefaultSelector::new(
            PromotionPolicy::KeepIfImproves,
            Direction::Maximize,
            Some(0.1),
        );
        let best = make_outcome(0.7, true);

        // Tiny improvement below threshold
        let d = sel.select(&make_outcome(0.75, true), &best).unwrap();
        assert_eq!(d.action, Action::Discarded);

        // Improvement above threshold
        let d = sel.select(&make_outcome(0.85, true), &best).unwrap();
        assert_eq!(d.action, Action::Promoted);
    }

    #[test]
    fn human_gate_always_escalates() {
        let sel = DefaultSelector::new(PromotionPolicy::HumanGate, Direction::Maximize, None);
        let d = sel
            .select(&make_outcome(0.9, true), &make_outcome(0.5, true))
            .unwrap();
        assert_eq!(d.action, Action::Escalated);
    }

    #[test]
    fn threshold_policy_maximize() {
        let sel = DefaultSelector::new(PromotionPolicy::Threshold, Direction::Maximize, Some(0.8));
        let best = make_outcome(0.5, true);

        let d = sel.select(&make_outcome(0.85, true), &best).unwrap();
        assert_eq!(d.action, Action::Promoted);

        let d = sel.select(&make_outcome(0.7, true), &best).unwrap();
        assert_eq!(d.action, Action::Discarded);
    }
}
