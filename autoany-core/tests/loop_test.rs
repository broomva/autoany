use autoany_core::budget::BudgetController;
use autoany_core::evaluator::Evaluator;
use autoany_core::executor::Executor;
use autoany_core::ledger::Ledger;
use autoany_core::loop_engine::EgriLoop;
use autoany_core::proposer::Proposer;
use autoany_core::selector::DefaultSelector;
use autoany_core::spec::PromotionPolicy;
use autoany_core::types::*;
use autoany_core::{EgriError, Result};

// --- Domain: optimize a simple function f(x) = -(x-3)^2 + 10 ---
// The artifact is just a single f64 value for x.
// The evaluator computes the function value.
// The proposer perturbs x randomly.
// The selector promotes if the score improves (maximize).

#[derive(Clone, Debug)]
struct SimpleArtifact {
    x: f64,
}

struct SimpleProposer {
    perturbations: Vec<f64>,
    index: std::cell::Cell<usize>,
}

impl Proposer for SimpleProposer {
    type Artifact = SimpleArtifact;

    fn propose(
        &self,
        artifact: &SimpleArtifact,
        _ledger: &Ledger,
    ) -> Result<(Mutation, SimpleArtifact)> {
        let idx = self.index.get();
        let delta = self.perturbations.get(idx).copied().unwrap_or(0.1);
        self.index.set(idx + 1);

        let new_x = artifact.x + delta;
        let mutation = Mutation {
            operator: "perturb".into(),
            description: format!("x += {delta:.2} -> {new_x:.2}"),
            diff: Some(format!("{:.4} -> {:.4}", artifact.x, new_x)),
            hypothesis: Some(format!("moving x by {delta:.2} may improve score")),
        };
        Ok((mutation, SimpleArtifact { x: new_x }))
    }
}

struct SimpleExecutor;

impl Executor for SimpleExecutor {
    type Artifact = SimpleArtifact;

    fn execute(&self, artifact: &SimpleArtifact) -> Result<ExecutionResult> {
        // f(x) = -(x-3)^2 + 10, max at x=3, f(3)=10
        let score = -(artifact.x - 3.0).powi(2) + 10.0;
        Ok(ExecutionResult {
            duration_secs: 0.001,
            exit_code: 0,
            error: None,
            output: Some(serde_json::json!({ "score": score })),
        })
    }
}

struct SimpleEvaluator;

impl Evaluator for SimpleEvaluator {
    type Artifact = SimpleArtifact;

    fn evaluate(
        &self,
        _artifact: &SimpleArtifact,
        execution: &ExecutionResult,
    ) -> Result<Outcome> {
        let score = execution
            .output
            .as_ref()
            .and_then(|o| o.get("score"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| EgriError::EvaluationFailed("no score in output".into()))?;

        Ok(Outcome {
            score: Score::Scalar(score),
            constraints_passed: true,
            constraint_violations: vec![],
            evaluator_metadata: None,
        })
    }
}

#[test]
fn test_full_egri_loop() {
    // Start at x=0, f(0) = -9 + 10 = 1
    // Perturbations: move toward x=3 in steps
    let proposer = SimpleProposer {
        perturbations: vec![1.0, 1.0, 1.0, 0.5, -0.5, 0.1, -0.1],
        index: std::cell::Cell::new(0),
    };

    let executor = SimpleExecutor;
    let evaluator = SimpleEvaluator;
    let selector = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
    let budget = BudgetController::new(7, None);
    let ledger = Ledger::in_memory();

    let mut egri = EgriLoop::new(proposer, executor, evaluator, selector, budget, ledger);

    // Phase 2: Establish baseline
    let baseline = egri.baseline(SimpleArtifact { x: 0.0 }).unwrap();
    assert_eq!(baseline.score.as_scalar().unwrap(), 1.0); // f(0) = 1

    // Phase 5: Run the loop
    let summary = egri.run().unwrap();

    println!("Total trials: {}", summary.total_trials);
    println!("Promoted: {}", summary.promoted_count);
    println!("Discarded: {}", summary.discarded_count);
    println!("Baseline: {:?}", summary.baseline_score);
    println!("Final: {:?}", summary.final_score);

    // Should have improved from baseline
    let final_score = summary
        .final_score
        .as_ref()
        .unwrap()
        .as_scalar()
        .unwrap();
    assert!(final_score > 1.0, "should improve from baseline of 1.0");

    // The best artifact should be close to x=3 (the optimum)
    let best = egri.best().unwrap();
    assert!(
        (best.x - 3.0).abs() < 1.0,
        "best x={:.2} should be near 3.0",
        best.x
    );

    // Ledger should have entries
    assert!(egri.ledger().records().len() > 1);

    // Print ledger
    println!("\n--- Ledger ---");
    for r in egri.ledger().records() {
        println!(
            "{}: score={:?} action={} reason={}",
            r.trial_id, r.outcome.score, r.decision.action, r.decision.reason
        );
    }
}

#[test]
fn test_budget_enforcement() {
    let proposer = SimpleProposer {
        perturbations: vec![0.1; 100],
        index: std::cell::Cell::new(0),
    };
    let selector = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
    let budget = BudgetController::new(3, None);
    let ledger = Ledger::in_memory();

    let mut egri = EgriLoop::new(
        proposer,
        SimpleExecutor,
        SimpleEvaluator,
        selector,
        budget,
        ledger,
    );

    egri.baseline(SimpleArtifact { x: 0.0 }).unwrap();
    let summary = egri.run().unwrap();

    assert_eq!(summary.total_trials, 3, "should stop after budget of 3");
}

#[test]
fn test_rollback() {
    let proposer = SimpleProposer {
        perturbations: vec![1.0],
        index: std::cell::Cell::new(0),
    };
    let selector = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
    let budget = BudgetController::new(5, None);
    let ledger = Ledger::in_memory();

    let mut egri = EgriLoop::new(
        proposer,
        SimpleExecutor,
        SimpleEvaluator,
        selector,
        budget,
        ledger,
    );

    egri.baseline(SimpleArtifact { x: 0.0 }).unwrap();
    egri.step().unwrap(); // x=0 -> x=1, f(1)=6, promoted

    let rolled_back = egri.rollback().unwrap();
    assert!(
        (rolled_back.x - 1.0).abs() < 0.001,
        "rollback should return to last promoted state (x=1.0)"
    );
}

#[test]
fn test_constraint_violation_discards() {
    // Evaluator that fails constraints when score > 9
    struct StrictEvaluator;
    impl Evaluator for StrictEvaluator {
        type Artifact = SimpleArtifact;
        fn evaluate(&self, _a: &SimpleArtifact, exec: &ExecutionResult) -> Result<Outcome> {
            let score = exec.output.as_ref().unwrap()["score"].as_f64().unwrap();
            let passes = score <= 9.0;
            Ok(Outcome {
                score: Score::Scalar(score),
                constraints_passed: passes,
                constraint_violations: if passes {
                    vec![]
                } else {
                    vec!["score exceeds safety limit of 9.0".into()]
                },
                evaluator_metadata: None,
            })
        }
    }

    let proposer = SimpleProposer {
        perturbations: vec![1.0, 1.0, 1.0], // x: 0->1->2->3
        index: std::cell::Cell::new(0),
    };
    let selector = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
    let budget = BudgetController::new(3, None);
    let ledger = Ledger::in_memory();

    let mut egri = EgriLoop::new(
        proposer,
        SimpleExecutor,
        StrictEvaluator,
        selector,
        budget,
        ledger,
    );

    egri.baseline(SimpleArtifact { x: 0.0 }).unwrap();
    let summary = egri.run().unwrap();

    // x=3 gives f(3)=10 which violates constraint, so should be discarded
    assert!(summary.discarded_count > 0);

    // Print to see what happened
    for r in egri.ledger().records() {
        println!(
            "{}: score={:?} action={} violations={:?}",
            r.trial_id, r.outcome.score, r.decision.action, r.outcome.constraint_violations
        );
    }
}
