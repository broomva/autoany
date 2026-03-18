use serde::{Deserialize, Serialize};

use crate::dead_ends::DeadEndTracker;
use crate::ledger::Ledger;
use crate::strategy::{self, StrategyReport};
use crate::types::Score;

/// Knowledge carried forward from a previous EGRI run.
///
/// Enables cross-run learning: new loops start with awareness of
/// what worked, what failed, and what's known to be a dead end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritedKnowledge {
    /// Strategy report from the previous run.
    pub strategy_report: Option<StrategyReport>,
    /// Known dead ends from the previous run.
    /// Note: This is serialized as the raw dead-end data, not the tracker itself.
    pub dead_end_signatures: Vec<String>,
    /// Best score achieved in the previous run.
    pub best_score: Option<Score>,
    /// Number of trials in the previous run.
    pub previous_trial_count: usize,
}

impl InheritedKnowledge {
    /// Reconstruct inherited knowledge from a previous run's ledger.
    pub fn from_ledger(ledger: &Ledger) -> Self {
        let strategy_report = Some(strategy::distill(ledger));
        let dead_end_tracker = DeadEndTracker::from_ledger(ledger, 3);
        let dead_end_signatures: Vec<String> = dead_end_tracker
            .confirmed()
            .iter()
            .map(|d| d.mutation_signature.clone())
            .collect();
        let best_score = ledger.last_promoted().map(|r| r.outcome.score.clone());
        let previous_trial_count = ledger.trial_count();

        Self {
            strategy_report,
            dead_end_signatures,
            best_score,
            previous_trial_count,
        }
    }

    /// Check if a mutation signature is known to be a dead end from previous runs.
    pub fn is_known_dead_end(&self, signature: &str) -> bool {
        self.dead_end_signatures.iter().any(|s| s == signature)
    }

    /// Get the recommended operator ordering from the previous run.
    pub fn recommended_operators(&self) -> Option<&[String]> {
        self.strategy_report
            .as_ref()
            .map(|r| r.recommended_order.as_slice())
    }

    /// Whether there is any inherited knowledge.
    pub fn is_empty(&self) -> bool {
        self.strategy_report.is_none()
            && self.dead_end_signatures.is_empty()
            && self.best_score.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_ledger_empty_produces_empty_knowledge() {
        let ledger = Ledger::in_memory();
        let knowledge = InheritedKnowledge::from_ledger(&ledger);

        // strategy_report is always Some (even if empty), so is_empty checks
        // dead_end_signatures and best_score too
        assert!(knowledge.dead_end_signatures.is_empty());
        assert!(knowledge.best_score.is_none());
        assert_eq!(knowledge.previous_trial_count, 0);
    }

    #[test]
    fn from_ledger_with_trials_produces_strategy_report() {
        use crate::types::*;
        use chrono::Utc;

        let mut ledger = Ledger::in_memory();
        ledger
            .append(TrialRecord {
                trial_id: TrialId("baseline".into()),
                timestamp: Utc::now(),
                parent_state: StateId::baseline(),
                mutation: Mutation {
                    operator: "none".into(),
                    description: "baseline".into(),
                    diff: None,
                    hypothesis: None,
                },
                execution: None,
                outcome: Outcome {
                    score: Score::Scalar(0.5),
                    constraints_passed: true,
                    constraint_violations: vec![],
                    evaluator_metadata: None,
                },
                decision: Decision {
                    action: Action::Promoted,
                    reason: "baseline".into(),
                    new_state_id: Some(StateId::baseline()),
                },
                strategy_notes: None,
            })
            .unwrap();

        ledger
            .append(TrialRecord {
                trial_id: TrialId("trial-001".into()),
                timestamp: Utc::now(),
                parent_state: StateId::baseline(),
                mutation: Mutation {
                    operator: "rewrite".into(),
                    description: "rewrite mutation".into(),
                    diff: None,
                    hypothesis: None,
                },
                execution: None,
                outcome: Outcome {
                    score: Score::Scalar(0.8),
                    constraints_passed: true,
                    constraint_violations: vec![],
                    evaluator_metadata: None,
                },
                decision: Decision {
                    action: Action::Promoted,
                    reason: "improved".into(),
                    new_state_id: Some(StateId::new()),
                },
                strategy_notes: None,
            })
            .unwrap();

        let knowledge = InheritedKnowledge::from_ledger(&ledger);

        assert!(knowledge.strategy_report.is_some());
        let report = knowledge.strategy_report.as_ref().unwrap();
        assert_eq!(report.total_trials, 1);
        assert!(knowledge.best_score.is_some());
        assert_eq!(knowledge.previous_trial_count, 1);
    }

    #[test]
    fn is_known_dead_end_checks_correctly() {
        let knowledge = InheritedKnowledge {
            strategy_report: None,
            dead_end_signatures: vec!["op_a::param_x".into(), "op_b::param_y".into()],
            best_score: None,
            previous_trial_count: 0,
        };

        assert!(knowledge.is_known_dead_end("op_a::param_x"));
        assert!(knowledge.is_known_dead_end("op_b::param_y"));
        assert!(!knowledge.is_known_dead_end("op_c::param_z"));
    }

    #[test]
    fn recommended_operators_returns_ordered_list() {
        let report = StrategyReport {
            successful_operators: vec![("fast_op".into(), 1.0), ("slow_op".into(), 0.5)],
            failure_patterns: vec![],
            recommended_order: vec!["fast_op".into(), "slow_op".into()],
            total_trials: 3,
            best_score: Some(Score::Scalar(0.9)),
        };

        let knowledge = InheritedKnowledge {
            strategy_report: Some(report),
            dead_end_signatures: vec![],
            best_score: Some(Score::Scalar(0.9)),
            previous_trial_count: 3,
        };

        let ops = knowledge.recommended_operators().unwrap();
        assert_eq!(ops, &["fast_op", "slow_op"]);
    }

    #[test]
    fn is_empty_works_correctly() {
        let empty = InheritedKnowledge {
            strategy_report: None,
            dead_end_signatures: vec![],
            best_score: None,
            previous_trial_count: 0,
        };
        assert!(empty.is_empty());

        let with_dead_ends = InheritedKnowledge {
            strategy_report: None,
            dead_end_signatures: vec!["sig".into()],
            best_score: None,
            previous_trial_count: 0,
        };
        assert!(!with_dead_ends.is_empty());

        let with_score = InheritedKnowledge {
            strategy_report: None,
            dead_end_signatures: vec![],
            best_score: Some(Score::Scalar(0.5)),
            previous_trial_count: 0,
        };
        assert!(!with_score.is_empty());

        let with_report = InheritedKnowledge {
            strategy_report: Some(StrategyReport {
                successful_operators: vec![],
                failure_patterns: vec![],
                recommended_order: vec![],
                total_trials: 0,
                best_score: None,
            }),
            dead_end_signatures: vec![],
            best_score: None,
            previous_trial_count: 0,
        };
        assert!(!with_report.is_empty());
    }

    #[test]
    fn serde_roundtrip() {
        let knowledge = InheritedKnowledge {
            strategy_report: Some(StrategyReport {
                successful_operators: vec![("op_a".into(), 0.75)],
                failure_patterns: vec![("op_b".into(), "timeout".into())],
                recommended_order: vec!["op_a".into(), "op_b".into()],
                total_trials: 10,
                best_score: Some(Score::Scalar(0.95)),
            }),
            dead_end_signatures: vec!["dead::sig".into()],
            best_score: Some(Score::Scalar(0.95)),
            previous_trial_count: 10,
        };

        let json = serde_json::to_string(&knowledge).unwrap();
        let back: InheritedKnowledge = serde_json::from_str(&json).unwrap();

        assert_eq!(back.dead_end_signatures, knowledge.dead_end_signatures);
        assert_eq!(back.previous_trial_count, knowledge.previous_trial_count);
        assert!(back.strategy_report.is_some());
        let report = back.strategy_report.unwrap();
        assert_eq!(report.total_trials, 10);
        assert_eq!(report.recommended_order, vec!["op_a", "op_b"]);
    }
}
