//! Stagnation detection for the EGRI loop.
//!
//! Detects when the loop has stopped making progress by tracking
//! consecutive non-improvements. When the count reaches the threshold,
//! returns `Stagnated` status, signaling the loop should escalate or halt.

use serde::{Deserialize, Serialize};

use crate::ledger::Ledger;

/// Status of stagnation detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StagnationStatus {
    /// Loop is making progress.
    Ok,
    /// Approaching stagnation (count of consecutive non-improvements).
    Warning(usize),
    /// Stagnated — recommend escalation (count of consecutive non-improvements).
    Stagnated(usize),
}

/// Detects when the EGRI loop has stopped making progress.
///
/// Tracks consecutive non-improvements (trials that were not promoted).
/// When the count reaches the threshold, returns `Stagnated` status.
pub struct StagnationDetector {
    threshold: usize,
    warning_ratio: f64,
}

impl StagnationDetector {
    /// Create a new detector with the given stagnation threshold.
    pub fn new(threshold: usize) -> Self {
        Self {
            threshold,
            warning_ratio: 0.5,
        }
    }

    /// Set the warning ratio (fraction of threshold to trigger Warning).
    pub fn with_warning_ratio(mut self, ratio: f64) -> Self {
        self.warning_ratio = ratio.clamp(0.0, 1.0);
        self
    }

    /// Check stagnation status from the ledger.
    pub fn check(&self, ledger: &Ledger) -> StagnationStatus {
        let count = ledger.consecutive_non_improvements();
        if count >= self.threshold {
            StagnationStatus::Stagnated(count)
        } else if count as f64 >= self.threshold as f64 * self.warning_ratio {
            StagnationStatus::Warning(count)
        } else {
            StagnationStatus::Ok
        }
    }

    /// Get the threshold value.
    pub fn threshold(&self) -> usize {
        self.threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use chrono::Utc;

    fn make_record(id: &str, action: Action) -> TrialRecord {
        TrialRecord {
            trial_id: TrialId(id.into()),
            timestamp: Utc::now(),
            parent_state: StateId::baseline(),
            mutation: Mutation {
                operator: "test".into(),
                description: "test mutation".into(),
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
                action,
                reason: "test".into(),
                new_state_id: None,
            },
            strategy_notes: None,
        }
    }

    #[test]
    fn fresh_ledger_ok() {
        let ledger = Ledger::in_memory();
        let detector = StagnationDetector::new(5);
        assert_eq!(detector.check(&ledger), StagnationStatus::Ok);
    }

    #[test]
    fn warning_at_correct_count() {
        let mut ledger = Ledger::in_memory();
        ledger
            .append(make_record("baseline", Action::Promoted))
            .unwrap();
        // Add 3 discards → warning at threshold 5 with ratio 0.5 (trigger at 2.5 → 3)
        for i in 1..=3 {
            ledger
                .append(make_record(&format!("trial-{i:03}"), Action::Discarded))
                .unwrap();
        }
        let detector = StagnationDetector::new(5);
        assert_eq!(detector.check(&ledger), StagnationStatus::Warning(3));
    }

    #[test]
    fn stagnation_at_threshold() {
        let mut ledger = Ledger::in_memory();
        ledger
            .append(make_record("baseline", Action::Promoted))
            .unwrap();
        for i in 1..=5 {
            ledger
                .append(make_record(&format!("trial-{i:03}"), Action::Discarded))
                .unwrap();
        }
        let detector = StagnationDetector::new(5);
        assert_eq!(detector.check(&ledger), StagnationStatus::Stagnated(5));
    }

    #[test]
    fn promotion_resets_counter() {
        let mut ledger = Ledger::in_memory();
        ledger
            .append(make_record("baseline", Action::Promoted))
            .unwrap();
        // 4 discards then a promotion
        for i in 1..=4 {
            ledger
                .append(make_record(&format!("trial-{i:03}"), Action::Discarded))
                .unwrap();
        }
        ledger
            .append(make_record("trial-005", Action::Promoted))
            .unwrap();
        // 1 more discard
        ledger
            .append(make_record("trial-006", Action::Discarded))
            .unwrap();

        let detector = StagnationDetector::new(5);
        // Only 1 consecutive non-improvement after the last promotion
        assert_eq!(detector.check(&ledger), StagnationStatus::Ok);
    }

    #[test]
    fn custom_warning_ratio() {
        let mut ledger = Ledger::in_memory();
        ledger
            .append(make_record("baseline", Action::Promoted))
            .unwrap();
        ledger
            .append(make_record("trial-001", Action::Discarded))
            .unwrap();

        // ratio 0.1 means warning at threshold * 0.1 = 1.0, so 1 discard triggers warning
        let detector = StagnationDetector::new(10).with_warning_ratio(0.1);
        assert_eq!(detector.check(&ledger), StagnationStatus::Warning(1));
    }

    #[test]
    fn threshold_accessor() {
        let detector = StagnationDetector::new(7);
        assert_eq!(detector.threshold(), 7);
    }
}
