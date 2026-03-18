use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ledger::Ledger;
use crate::types::{Action, Score};

/// Summary of operator performance distilled from a ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyReport {
    /// Operators ranked by success rate: (operator_name, success_rate).
    pub successful_operators: Vec<(String, f64)>,
    /// Common failure patterns: (operator_name, most_common_reason).
    pub failure_patterns: Vec<(String, String)>,
    /// Recommended operator ordering for future runs (most successful first).
    pub recommended_order: Vec<String>,
    /// Total trials analyzed.
    pub total_trials: usize,
    /// Best score achieved.
    pub best_score: Option<Score>,
}

/// Distill strategy insights from a completed loop's ledger.
///
/// Analyzes which operators were most successful, what failure patterns
/// emerged, and produces a recommended ordering for future runs.
pub fn distill(ledger: &Ledger) -> StrategyReport {
    let records = ledger.records();

    // Skip baseline
    let trials: Vec<_> = records
        .iter()
        .filter(|r| r.trial_id.0 != "baseline")
        .collect();

    // Count successes and failures per operator
    let mut operator_successes: HashMap<String, usize> = HashMap::new();
    let mut operator_total: HashMap<String, usize> = HashMap::new();
    let mut failure_reasons: HashMap<String, HashMap<String, usize>> = HashMap::new();

    for record in &trials {
        let op = &record.mutation.operator;
        *operator_total.entry(op.clone()).or_default() += 1;

        if record.decision.action == Action::Promoted {
            *operator_successes.entry(op.clone()).or_default() += 1;
        } else if record.decision.action == Action::Discarded {
            *failure_reasons
                .entry(op.clone())
                .or_default()
                .entry(record.decision.reason.clone())
                .or_default() += 1;
        }
    }

    // Compute success rates
    let mut successful_operators: Vec<(String, f64)> = operator_total
        .iter()
        .map(|(op, total)| {
            let successes = operator_successes.get(op).copied().unwrap_or(0);
            (op.clone(), successes as f64 / *total as f64)
        })
        .collect();
    successful_operators.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Extract most common failure per operator
    let failure_patterns: Vec<(String, String)> = failure_reasons
        .iter()
        .filter_map(|(op, reasons)| {
            reasons
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(reason, _)| (op.clone(), reason.clone()))
        })
        .collect();

    // Recommended order: sort by success rate descending
    let recommended_order: Vec<String> = successful_operators
        .iter()
        .map(|(op, _)| op.clone())
        .collect();

    // Best score from promoted records
    let best_score = ledger.last_promoted().map(|r| r.outcome.score.clone());

    StrategyReport {
        successful_operators,
        failure_patterns,
        recommended_order,
        total_trials: trials.len(),
        best_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Ledger;
    use crate::types::*;
    use chrono::Utc;

    fn make_record(id: &str, operator: &str, action: Action, reason: &str) -> TrialRecord {
        TrialRecord {
            trial_id: TrialId(id.into()),
            timestamp: Utc::now(),
            parent_state: StateId::baseline(),
            mutation: Mutation {
                operator: operator.into(),
                description: format!("{operator} mutation"),
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
                reason: reason.into(),
                new_state_id: None,
            },
            strategy_notes: None,
        }
    }

    fn make_promoted(id: &str, operator: &str, score: f64) -> TrialRecord {
        let mut r = make_record(id, operator, Action::Promoted, "improved");
        r.outcome.score = Score::Scalar(score);
        r
    }

    #[test]
    fn empty_ledger_returns_empty_report() {
        let ledger = Ledger::in_memory();
        let report = distill(&ledger);

        assert!(report.successful_operators.is_empty());
        assert!(report.failure_patterns.is_empty());
        assert!(report.recommended_order.is_empty());
        assert_eq!(report.total_trials, 0);
        assert!(report.best_score.is_none());
    }

    #[test]
    fn single_promotion_gives_full_success_rate() {
        let mut ledger = Ledger::in_memory();
        ledger
            .append(make_record(
                "baseline",
                "none",
                Action::Promoted,
                "baseline",
            ))
            .unwrap();
        ledger
            .append(make_promoted("trial-001", "rewrite", 0.9))
            .unwrap();

        let report = distill(&ledger);

        assert_eq!(report.total_trials, 1);
        assert_eq!(report.successful_operators.len(), 1);
        assert_eq!(report.successful_operators[0].0, "rewrite");
        assert!((report.successful_operators[0].1 - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn mixed_successes_and_failures_computes_correct_rates() {
        let mut ledger = Ledger::in_memory();
        ledger
            .append(make_record(
                "baseline",
                "none",
                Action::Promoted,
                "baseline",
            ))
            .unwrap();

        // op_a: 2 promoted, 1 discarded => 66.7%
        ledger
            .append(make_promoted("trial-001", "op_a", 0.8))
            .unwrap();
        ledger
            .append(make_promoted("trial-002", "op_a", 0.85))
            .unwrap();
        ledger
            .append(make_record(
                "trial-003",
                "op_a",
                Action::Discarded,
                "no improvement",
            ))
            .unwrap();

        // op_b: 0 promoted, 2 discarded => 0%
        ledger
            .append(make_record(
                "trial-004",
                "op_b",
                Action::Discarded,
                "regression",
            ))
            .unwrap();
        ledger
            .append(make_record(
                "trial-005",
                "op_b",
                Action::Discarded,
                "regression",
            ))
            .unwrap();

        let report = distill(&ledger);

        assert_eq!(report.total_trials, 5);

        // op_a should be first (higher success rate)
        assert_eq!(report.successful_operators[0].0, "op_a");
        assert!((report.successful_operators[0].1 - 2.0 / 3.0).abs() < 0.01);

        // op_b should be second (0% success)
        assert_eq!(report.successful_operators[1].0, "op_b");
        assert!((report.successful_operators[1].1 - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn recommended_order_sorted_by_success_rate() {
        let mut ledger = Ledger::in_memory();
        ledger
            .append(make_record(
                "baseline",
                "none",
                Action::Promoted,
                "baseline",
            ))
            .unwrap();

        // fast_op: 1/1 = 100%
        ledger
            .append(make_promoted("trial-001", "fast_op", 0.9))
            .unwrap();

        // slow_op: 1/3 = 33%
        ledger
            .append(make_promoted("trial-002", "slow_op", 0.7))
            .unwrap();
        ledger
            .append(make_record(
                "trial-003",
                "slow_op",
                Action::Discarded,
                "too slow",
            ))
            .unwrap();
        ledger
            .append(make_record(
                "trial-004",
                "slow_op",
                Action::Discarded,
                "too slow",
            ))
            .unwrap();

        let report = distill(&ledger);

        assert_eq!(report.recommended_order[0], "fast_op");
        assert_eq!(report.recommended_order[1], "slow_op");
    }

    #[test]
    fn failure_patterns_captures_most_common_reason() {
        let mut ledger = Ledger::in_memory();
        ledger
            .append(make_record(
                "baseline",
                "none",
                Action::Promoted,
                "baseline",
            ))
            .unwrap();

        // op_x fails with "timeout" 3 times, "crash" 1 time
        ledger
            .append(make_record(
                "trial-001",
                "op_x",
                Action::Discarded,
                "timeout",
            ))
            .unwrap();
        ledger
            .append(make_record(
                "trial-002",
                "op_x",
                Action::Discarded,
                "timeout",
            ))
            .unwrap();
        ledger
            .append(make_record(
                "trial-003",
                "op_x",
                Action::Discarded,
                "timeout",
            ))
            .unwrap();
        ledger
            .append(make_record("trial-004", "op_x", Action::Discarded, "crash"))
            .unwrap();

        let report = distill(&ledger);

        assert_eq!(report.failure_patterns.len(), 1);
        let (op, reason) = &report.failure_patterns[0];
        assert_eq!(op, "op_x");
        assert_eq!(reason, "timeout");
    }
}
