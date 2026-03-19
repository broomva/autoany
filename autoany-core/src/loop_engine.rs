use chrono::Utc;
use tracing::{info, warn};

use crate::budget::BudgetController;
use crate::error::{EgriError, Result};
use crate::evaluator::Evaluator;
use crate::executor::Executor;
use crate::ledger::Ledger;
use crate::promotion::PromotionController;
use crate::proposer::Proposer;
use crate::selector::Selector;
use crate::types::*;

/// The EGRI loop engine — orchestrates the full recursive improvement cycle.
///
/// ```text
/// Π = (X, M, H, E, J, C, B, P, L)
///
/// while budget remains:
///   m = propose(x_t, L)
///   x' = apply(m, x_t)
///   result = execute(x')
///   outcome = evaluate(x', result)
///   decision = select(outcome, best)
///   apply_decision(decision, x')
///   append(L, trial_record)
/// ```
pub struct EgriLoop<A, P, X, E, S>
where
    A: Clone,
    P: Proposer<Artifact = A>,
    X: Executor<Artifact = A>,
    E: Evaluator<Artifact = A>,
    S: Selector,
{
    proposer: P,
    executor: X,
    evaluator: E,
    selector: S,
    budget: BudgetController,
    promotion: PromotionController<A>,
    ledger: Ledger,
    best_outcome: Option<Outcome>,
}

/// Summary of a completed loop.
#[derive(Debug)]
pub struct LoopSummary {
    pub total_trials: usize,
    pub promoted_count: usize,
    pub discarded_count: usize,
    pub escalated_count: usize,
    pub baseline_score: Option<Score>,
    pub final_score: Option<Score>,
}

impl<A, P, X, E, S> EgriLoop<A, P, X, E, S>
where
    A: Clone,
    P: Proposer<Artifact = A>,
    X: Executor<Artifact = A>,
    E: Evaluator<Artifact = A>,
    S: Selector,
{
    pub fn new(
        proposer: P,
        executor: X,
        evaluator: E,
        selector: S,
        budget: BudgetController,
        ledger: Ledger,
    ) -> Self {
        Self {
            proposer,
            executor,
            evaluator,
            selector,
            budget,
            promotion: PromotionController::new(),
            ledger,
            best_outcome: None,
        }
    }

    /// Establish the baseline. Must be called before `run`.
    pub fn baseline(&mut self, artifact: A) -> Result<Outcome> {
        info!("establishing baseline");

        let exec_result = self.executor.execute(&artifact)?;
        let outcome = self.evaluator.evaluate(&artifact, &exec_result)?;

        self.promotion.set_baseline(artifact);
        self.best_outcome = Some(outcome.clone());

        let record = TrialRecord {
            trial_id: TrialId::baseline(),
            timestamp: Utc::now(),
            parent_state: StateId::baseline(),
            mutation: Mutation {
                operator: "none".into(),
                description: "baseline measurement".into(),
                diff: None,
                hypothesis: None,
            },
            execution: Some(exec_result),
            outcome: outcome.clone(),
            decision: Decision {
                action: Action::Promoted,
                reason: "baseline establishment".into(),
                new_state_id: Some(StateId::baseline()),
            },
            strategy_notes: None,
        };

        self.ledger.append(record)?;
        info!(score = ?outcome.score, "baseline established");

        Ok(outcome)
    }

    /// Run a single trial. Returns the trial record.
    pub fn step(&mut self) -> Result<TrialRecord> {
        self.budget.check()?;

        let best_outcome = self.best_outcome.as_ref().ok_or(EgriError::NoBaseline)?;

        let current = self
            .promotion
            .current()
            .ok_or(EgriError::NoBaseline)?
            .clone();

        let parent_state = self
            .promotion
            .current_state_id()
            .cloned()
            .unwrap_or_else(StateId::baseline);

        // Propose
        let (mutation, candidate) = self.proposer.propose(&current, &self.ledger)?;
        info!(operator = %mutation.operator, desc = %mutation.description, "proposed mutation");

        // Execute
        let exec_result = self.executor.execute(&candidate);
        let exec_result = match exec_result {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, "execution failed");
                self.budget.consume();
                let record = TrialRecord {
                    trial_id: TrialId::new(self.budget.used()),
                    timestamp: Utc::now(),
                    parent_state,
                    mutation,
                    execution: None,
                    outcome: Outcome {
                        score: Score::Scalar(0.0),
                        constraints_passed: false,
                        constraint_violations: vec![format!("execution failed: {e}")],
                        evaluator_metadata: None,
                    },
                    decision: Decision {
                        action: Action::Discarded,
                        reason: format!("execution failed: {e}"),
                        new_state_id: None,
                    },
                    strategy_notes: None,
                };
                self.ledger.append(record.clone())?;
                return Ok(record);
            }
        };

        // Evaluate
        let outcome = self.evaluator.evaluate(&candidate, &exec_result)?;

        // Select
        let decision = self.selector.select(&outcome, best_outcome)?;

        info!(
            score = ?outcome.score,
            action = %decision.action,
            reason = %decision.reason,
            "trial complete"
        );

        // Apply decision
        if decision.action == Action::Promoted {
            self.best_outcome = Some(outcome.clone());
        }
        self.promotion.apply_decision(&decision, candidate);

        self.budget.consume();

        let record = TrialRecord {
            trial_id: TrialId::new(self.budget.used()),
            timestamp: Utc::now(),
            parent_state,
            mutation,
            execution: Some(exec_result),
            outcome,
            decision,
            strategy_notes: None,
        };

        self.ledger.append(record.clone())?;
        Ok(record)
    }

    /// Run the full loop until budget is exhausted or an escalation occurs.
    pub fn run(&mut self) -> Result<LoopSummary> {
        self.budget.start();

        loop {
            match self.step() {
                Ok(record) => {
                    if record.decision.action == Action::Escalated {
                        info!(reason = %record.decision.reason, "escalation — halting loop");
                        break;
                    }
                }
                Err(EgriError::BudgetExhausted(msg)) => {
                    info!(reason = %msg, "budget exhausted — loop complete");
                    break;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(self.summary())
    }

    /// Get current loop summary.
    pub fn summary(&self) -> LoopSummary {
        let records = self.ledger.records();
        let baseline_record = records.first();
        let last_promoted = self.ledger.last_promoted();

        LoopSummary {
            total_trials: self.ledger.trial_count(),
            promoted_count: self.ledger.by_action(Action::Promoted).len(),
            discarded_count: self.ledger.by_action(Action::Discarded).len(),
            escalated_count: self.ledger.by_action(Action::Escalated).len(),
            baseline_score: baseline_record.map(|r| r.outcome.score.clone()),
            final_score: last_promoted.map(|r| r.outcome.score.clone()),
        }
    }

    /// Access the ledger.
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// Access the current best artifact.
    pub fn best(&self) -> Option<&A> {
        self.promotion.best()
    }

    /// Current best score for hive reporting.
    pub fn best_score(&self) -> Option<&Score> {
        self.best_outcome.as_ref().map(|o| &o.score)
    }

    /// Inject trial records from another agent's history (cross-pollination).
    ///
    /// These records are appended to the ledger for the proposer to learn from
    /// but do not affect the current promotion state.
    pub fn inject_history(&mut self, records: Vec<TrialRecord>) -> Result<()> {
        for record in records {
            self.ledger.append(record)?;
        }
        Ok(())
    }

    /// Rollback to last promoted state.
    pub fn rollback(&mut self) -> Result<&A> {
        self.promotion.rollback()
    }
}
