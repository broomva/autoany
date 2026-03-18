//! Dead-end tracking for the EGRI loop.
//!
//! Tracks mutation paths that have failed repeatedly. Once a mutation
//! signature crosses the failure threshold, it is marked as a dead end.
//! The EGRI loop can check this before executing to save budget.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ledger::Ledger;
use crate::types::Action;

/// A mutation path that has failed repeatedly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadEnd {
    /// Composite signature: operator + parent state.
    pub mutation_signature: String,
    /// Most recent failure reason.
    pub failure_reason: String,
    /// Number of times this path has failed.
    pub occurrences: usize,
}

/// Tracks mutation paths that have failed repeatedly.
///
/// Once a mutation signature crosses the failure threshold, it is marked
/// as a dead end. The EGRI loop can check this before executing to save budget.
pub struct DeadEndTracker {
    dead_ends: HashMap<String, DeadEnd>,
    threshold: usize,
}

impl DeadEndTracker {
    /// Create a new tracker with the given failure threshold.
    pub fn new(threshold: usize) -> Self {
        Self {
            dead_ends: HashMap::new(),
            threshold,
        }
    }

    /// Record a failure for a mutation signature.
    pub fn record_failure(&mut self, signature: &str, reason: &str) {
        let entry = self
            .dead_ends
            .entry(signature.to_string())
            .or_insert_with(|| DeadEnd {
                mutation_signature: signature.to_string(),
                failure_reason: reason.to_string(),
                occurrences: 0,
            });
        entry.occurrences += 1;
        entry.failure_reason = reason.to_string();
    }

    /// Check if a mutation signature is a known dead end.
    pub fn is_dead_end(&self, signature: &str) -> bool {
        self.dead_ends
            .get(signature)
            .is_some_and(|d| d.occurrences >= self.threshold)
    }

    /// Get all tracked entries (including those below threshold).
    pub fn all(&self) -> &HashMap<String, DeadEnd> {
        &self.dead_ends
    }

    /// Get only confirmed dead ends (at or above threshold).
    pub fn confirmed(&self) -> Vec<&DeadEnd> {
        self.dead_ends
            .values()
            .filter(|d| d.occurrences >= self.threshold)
            .collect()
    }

    /// Reconstruct a tracker from ledger history.
    ///
    /// Scans all discarded trials and builds failure counts per
    /// `operator:parent_state` signature.
    pub fn from_ledger(ledger: &Ledger, threshold: usize) -> Self {
        let mut tracker = Self::new(threshold);
        for record in ledger.records() {
            if record.decision.action == Action::Discarded {
                let signature = format!("{}:{}", record.mutation.operator, record.parent_state);
                tracker.record_failure(&signature, &record.decision.reason);
            }
        }
        tracker
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_allows_everything() {
        let tracker = DeadEndTracker::new(3);
        assert!(!tracker.is_dead_end("op:state"));
        assert!(tracker.all().is_empty());
        assert!(tracker.confirmed().is_empty());
    }

    #[test]
    fn below_threshold_not_dead() {
        let mut tracker = DeadEndTracker::new(3);
        tracker.record_failure("op:s1", "failed");
        tracker.record_failure("op:s1", "failed again");
        assert!(!tracker.is_dead_end("op:s1"));
        assert_eq!(tracker.all().len(), 1);
        assert!(tracker.confirmed().is_empty());
    }

    #[test]
    fn at_threshold_marks_dead() {
        let mut tracker = DeadEndTracker::new(3);
        tracker.record_failure("op:s1", "fail 1");
        tracker.record_failure("op:s1", "fail 2");
        tracker.record_failure("op:s1", "fail 3");
        assert!(tracker.is_dead_end("op:s1"));
        assert_eq!(tracker.confirmed().len(), 1);
    }

    #[test]
    fn multiple_signatures_tracked_independently() {
        let mut tracker = DeadEndTracker::new(2);
        tracker.record_failure("op_a:s1", "fail");
        tracker.record_failure("op_a:s1", "fail");
        tracker.record_failure("op_b:s1", "fail");
        assert!(tracker.is_dead_end("op_a:s1"));
        assert!(!tracker.is_dead_end("op_b:s1"));
        assert_eq!(tracker.confirmed().len(), 1);
    }

    #[test]
    fn from_ledger_reconstructs() {
        use crate::types::*;
        use chrono::Utc;

        let mut ledger = Ledger::in_memory();
        // Baseline
        ledger
            .append(TrialRecord {
                trial_id: TrialId::baseline(),
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
                    score: Score::Scalar(1.0),
                    constraints_passed: true,
                    constraint_violations: vec![],
                    evaluator_metadata: None,
                },
                decision: Decision {
                    action: Action::Promoted,
                    reason: "baseline".into(),
                    new_state_id: None,
                },
                strategy_notes: None,
            })
            .unwrap();

        // Three discarded trials with same operator + parent
        for i in 1..=3 {
            ledger
                .append(TrialRecord {
                    trial_id: TrialId::new(i),
                    timestamp: Utc::now(),
                    parent_state: StateId::baseline(),
                    mutation: Mutation {
                        operator: "tweak".into(),
                        description: format!("attempt {i}"),
                        diff: None,
                        hypothesis: None,
                    },
                    execution: None,
                    outcome: Outcome {
                        score: Score::Scalar(0.5),
                        constraints_passed: false,
                        constraint_violations: vec![],
                        evaluator_metadata: None,
                    },
                    decision: Decision {
                        action: Action::Discarded,
                        reason: "no improvement".into(),
                        new_state_id: None,
                    },
                    strategy_notes: None,
                })
                .unwrap();
        }

        let tracker = DeadEndTracker::from_ledger(&ledger, 3);
        assert!(tracker.is_dead_end("tweak:baseline"));
        assert_eq!(tracker.confirmed().len(), 1);
    }
}
