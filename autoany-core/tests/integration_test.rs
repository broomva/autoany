//! Integration tests for the unified EGRI loop with dead-end tracking,
//! stagnation detection, strategy distillation, and cross-run inheritance.

use autoany_core::budget::BudgetController;
use autoany_core::dead_ends::DeadEndTracker;
use autoany_core::evaluator::Evaluator;
use autoany_core::executor::Executor;
use autoany_core::inheritance::InheritedKnowledge;
use autoany_core::ledger::Ledger;
use autoany_core::loop_engine::EgriLoop;
use autoany_core::proposer::Proposer;
use autoany_core::selector::DefaultSelector;
use autoany_core::spec::PromotionPolicy;
use autoany_core::stagnation::{StagnationDetector, StagnationStatus};
use autoany_core::strategy;
use autoany_core::types::*;
use autoany_core::{EgriError, Result};

// --- Domain: optimize f(x) = -(x-3)^2 + 10 ---

#[derive(Clone, Debug)]
struct Artifact {
    x: f64,
}

struct TestProposer {
    perturbations: Vec<f64>,
    index: std::cell::Cell<usize>,
}

impl Proposer for TestProposer {
    type Artifact = Artifact;

    fn propose(&self, artifact: &Artifact, _ledger: &Ledger) -> Result<(Mutation, Artifact)> {
        let idx = self.index.get();
        let delta = self.perturbations.get(idx).copied().unwrap_or(0.1);
        self.index.set(idx + 1);

        let new_x = artifact.x + delta;
        let op = if delta.abs() < 0.01 {
            "noop"
        } else {
            "perturb"
        };
        let mutation = Mutation {
            operator: op.into(),
            description: format!("x += {delta:.2}"),
            diff: None,
            hypothesis: None,
        };
        Ok((mutation, Artifact { x: new_x }))
    }
}

struct TestExecutor;

impl Executor for TestExecutor {
    type Artifact = Artifact;

    fn execute(&self, artifact: &Artifact) -> Result<ExecutionResult> {
        let score = -(artifact.x - 3.0).powi(2) + 10.0;
        Ok(ExecutionResult {
            duration_secs: 0.001,
            exit_code: 0,
            error: None,
            output: Some(serde_json::json!({ "score": score })),
        })
    }
}

struct TestEvaluator;

impl Evaluator for TestEvaluator {
    type Artifact = Artifact;

    fn evaluate(&self, _artifact: &Artifact, execution: &ExecutionResult) -> Result<Outcome> {
        let score = execution
            .output
            .as_ref()
            .and_then(|o| o.get("score"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| EgriError::EvaluationFailed("no score".into()))?;

        Ok(Outcome {
            score: Score::Scalar(score),
            constraints_passed: true,
            constraint_violations: vec![],
            evaluator_metadata: None,
        })
    }
}

/// Test 1: Dead-end prevention saves budget
#[test]
fn dead_end_tracking_prevents_wasted_trials() {
    // Run an initial loop that will produce failures
    let proposer = TestProposer {
        // Move away from optimum repeatedly with same operator
        perturbations: vec![-5.0, -5.0, -5.0, -5.0, -5.0],
        index: std::cell::Cell::new(0),
    };
    let selector = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
    let budget = BudgetController::new(5, None);
    let ledger = Ledger::in_memory();

    let mut egri = EgriLoop::new(
        proposer,
        TestExecutor,
        TestEvaluator,
        selector,
        budget,
        ledger,
    );

    egri.baseline(Artifact { x: 3.0 }).unwrap();
    egri.run().unwrap();

    // Build dead-end tracker from the ledger
    let tracker = DeadEndTracker::from_ledger(egri.ledger(), 3);

    // The "perturb:baseline" signature should be a dead end (5 failures)
    assert!(
        !tracker.confirmed().is_empty(),
        "should have at least one confirmed dead end"
    );
}

/// Test 2: Stagnation detection triggers correctly
#[test]
fn stagnation_detection_escalation() {
    // All perturbations move away → all discarded → stagnation
    let proposer = TestProposer {
        perturbations: vec![-10.0; 10],
        index: std::cell::Cell::new(0),
    };
    let selector = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
    let budget = BudgetController::new(10, None);
    let ledger = Ledger::in_memory();

    let mut egri = EgriLoop::new(
        proposer,
        TestExecutor,
        TestEvaluator,
        selector,
        budget,
        ledger,
    );

    egri.baseline(Artifact { x: 3.0 }).unwrap();
    egri.run().unwrap();

    let detector = StagnationDetector::new(5);
    let status = detector.check(egri.ledger());

    assert!(
        matches!(status, StagnationStatus::Stagnated(_)),
        "10 consecutive failures should trigger stagnation, got: {status:?}"
    );
}

/// Test 3: Strategy distillation produces accurate report
#[test]
fn strategy_distillation_accuracy() {
    // Mix of good and bad perturbations
    let proposer = TestProposer {
        perturbations: vec![1.0, 1.0, 1.0, -5.0, -5.0],
        index: std::cell::Cell::new(0),
    };
    let selector = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
    let budget = BudgetController::new(5, None);
    let ledger = Ledger::in_memory();

    let mut egri = EgriLoop::new(
        proposer,
        TestExecutor,
        TestEvaluator,
        selector,
        budget,
        ledger,
    );

    egri.baseline(Artifact { x: 0.0 }).unwrap();
    egri.run().unwrap();

    let report = strategy::distill(egri.ledger());

    assert_eq!(report.total_trials, 5);
    assert!(
        report
            .successful_operators
            .iter()
            .any(|(_, rate)| *rate > 0.0),
        "some operators should have successes"
    );
    assert!(!report.recommended_order.is_empty());
    assert!(report.best_score.is_some());
}

/// Test 4: Cross-run inheritance carries forward knowledge
#[test]
fn cross_run_inheritance() {
    // Run 1: produce failures to create dead ends
    let proposer = TestProposer {
        perturbations: vec![-5.0, -5.0, -5.0, 1.0, 1.0],
        index: std::cell::Cell::new(0),
    };
    let selector = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
    let budget = BudgetController::new(5, None);
    let ledger = Ledger::in_memory();

    let mut egri = EgriLoop::new(
        proposer,
        TestExecutor,
        TestEvaluator,
        selector,
        budget,
        ledger,
    );

    egri.baseline(Artifact { x: 0.0 }).unwrap();
    egri.run().unwrap();

    // Extract knowledge from run 1
    let knowledge = InheritedKnowledge::from_ledger(egri.ledger());

    assert!(!knowledge.is_empty(), "should have inherited knowledge");
    assert!(
        knowledge.strategy_report.is_some(),
        "should have strategy report"
    );
    assert!(
        knowledge.best_score.is_some(),
        "should have best score from run 1"
    );
    assert!(
        knowledge.previous_trial_count > 0,
        "should record previous trial count"
    );

    // The knowledge should inform run 2
    if let Some(operators) = knowledge.recommended_operators() {
        assert!(
            !operators.is_empty(),
            "should have operator recommendations"
        );
    }

    // Serde roundtrip
    let json = serde_json::to_string(&knowledge).unwrap();
    let roundtripped: InheritedKnowledge = serde_json::from_str(&json).unwrap();
    assert_eq!(
        roundtripped.previous_trial_count,
        knowledge.previous_trial_count
    );
}

/// Test 5: Full pipeline — run → distill → inherit → verify
#[test]
fn full_pipeline_run_distill_inherit() {
    // Run 1
    let proposer = TestProposer {
        perturbations: vec![1.0, 1.0, 0.5, 0.25, -3.0],
        index: std::cell::Cell::new(0),
    };
    let selector = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
    let budget = BudgetController::new(5, None);
    let ledger = Ledger::in_memory();

    let mut egri = EgriLoop::new(
        proposer,
        TestExecutor,
        TestEvaluator,
        selector,
        budget,
        ledger,
    );

    egri.baseline(Artifact { x: 0.0 }).unwrap();
    let summary1 = egri.run().unwrap();

    // Distill
    let report = strategy::distill(egri.ledger());
    assert_eq!(report.total_trials, summary1.total_trials);

    // Inherit
    let knowledge = InheritedKnowledge::from_ledger(egri.ledger());

    // Stagnation check on run 1
    let detector = StagnationDetector::new(5);
    let status = detector.check(egri.ledger());
    // Should NOT be stagnated since we had promotions
    assert!(
        !matches!(status, StagnationStatus::Stagnated(_)),
        "run with promotions should not be stagnated"
    );

    // Dead-end check
    let tracker = DeadEndTracker::from_ledger(egri.ledger(), 3);
    // With only 5 trials, unlikely to have 3+ failures of same operator
    // but the tracker should still be constructible
    assert!(tracker.all().len() <= 5);

    // Verify inheritance captures the best score
    let best_score = knowledge.best_score.unwrap().as_scalar().unwrap();
    let baseline_score = summary1.baseline_score.unwrap().as_scalar().unwrap();
    assert!(
        best_score >= baseline_score,
        "inherited best score should be at least as good as baseline"
    );
}
