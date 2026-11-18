//! Controlled nulls must remain distinct from refutation, including at promotion.
use autoany_core::Result;
use autoany_core::budget::BudgetController;
use autoany_core::evaluator::Evaluator;
use autoany_core::executor::Executor;
use autoany_core::ledger::Ledger;
use autoany_core::loop_engine::EgriLoop;
use autoany_core::proposer::Proposer;
use autoany_core::selector::{DefaultSelector, Selector};
use autoany_core::spec::PromotionPolicy;
use autoany_core::types::*;

fn evidence(
    control: PositiveControl,
    target: TargetOutcome,
    absence_refutes: bool,
) -> EvaluationEvidence {
    let mut e = EvaluationEvidence {
        claim: "target generates signal".into(),
        conditions: "run-1; detection limit 0.1".into(),
        observation_ref: "raw.json#target".into(),
        positive_control: control,
        control_ref: Some("raw.json#control".into()),
        target_outcome: target,
        falsification_criterion: "absence above declared threshold with live control".into(),
        absence_refutes,
        claim_verdict: ClaimVerdict::Inconclusive,
    };
    e.claim_verdict = e.derived_verdict();
    e
}

fn outcome(e: Option<EvaluationEvidence>) -> Outcome {
    Outcome {
        evidence: e,
        score: Score::Scalar(100.0),
        constraints_passed: true,
        constraint_violations: vec![],
        evaluator_metadata: None,
    }
}

#[test]
fn all_control_target_combinations_preserve_the_inference_boundary() {
    for control in [
        PositiveControl::Pass,
        PositiveControl::Fail,
        PositiveControl::Missing,
        PositiveControl::Error,
        PositiveControl::Skipped,
    ] {
        for target in [
            TargetOutcome::Detected,
            TargetOutcome::NotDetected,
            TargetOutcome::Error,
            TargetOutcome::Skipped,
        ] {
            for falsifies in [true, false] {
                let e = evidence(control, target, falsifies);
                let expected = if control != PositiveControl::Pass {
                    ClaimVerdict::InconclusiveNoPositiveControl
                } else {
                    match target {
                        TargetOutcome::Detected => ClaimVerdict::Detected,
                        TargetOutcome::NotDetected if falsifies => ClaimVerdict::Refuted,
                        TargetOutcome::NotDetected => ClaimVerdict::NotDetected,
                        _ => ClaimVerdict::Inconclusive,
                    }
                };
                assert_eq!(e.derived_verdict(), expected);
                assert_eq!(
                    e.allows_promotion(),
                    matches!(expected, ClaimVerdict::Detected | ClaimVerdict::Refuted)
                );
            }
        }
    }
}

#[test]
fn forged_refutation_and_empty_receipts_do_not_promote() {
    let mut e = evidence(PositiveControl::Missing, TargetOutcome::NotDetected, true);
    e.claim_verdict = ClaimVerdict::Refuted;
    assert!(!e.allows_promotion());
    let mut e = evidence(PositiveControl::Pass, TargetOutcome::NotDetected, true);
    e.control_ref = Some("  ".into());
    assert!(!e.allows_promotion());
}

#[test]
fn legacy_json_still_deserializes_but_explicit_inconclusive_evidence_escalates() {
    let legacy: Outcome =
        serde_json::from_str(r#"{"score":0.0,"constraints_passed":true}"#).unwrap();
    assert!(legacy.evidence_allows_promotion());
    let sel = DefaultSelector::new(PromotionPolicy::KeepIfImproves, Direction::Maximize, None);
    let candidate = outcome(Some(evidence(
        PositiveControl::Missing,
        TargetOutcome::NotDetected,
        true,
    )));
    assert_eq!(
        sel.select(&candidate, &legacy).unwrap().action,
        Action::Escalated
    );
}

struct Propose;
impl Proposer for Propose {
    type Artifact = bool;
    fn propose(&self, _: &bool, _: &Ledger) -> Result<(Mutation, bool)> {
        Ok((
            Mutation {
                operator: "change".into(),
                description: "candidate".into(),
                diff: None,
                hypothesis: None,
            },
            true,
        ))
    }
}
struct Execute;
impl Executor for Execute {
    type Artifact = bool;
    fn execute(&self, _: &bool) -> Result<ExecutionResult> {
        Ok(ExecutionResult {
            duration_secs: 0.0,
            exit_code: 0,
            error: None,
            output: None,
        })
    }
}
struct Evaluate;
impl Evaluator for Evaluate {
    type Artifact = bool;
    fn evaluate(&self, invalid: &bool, _: &ExecutionResult) -> Result<Outcome> {
        Ok(outcome(Some(evidence(
            if *invalid {
                PositiveControl::Missing
            } else {
                PositiveControl::Pass
            },
            TargetOutcome::NotDetected,
            true,
        ))))
    }
}
struct AlwaysPromote;
impl Selector for AlwaysPromote {
    fn select(&self, _: &Outcome, _: &Outcome) -> Result<Decision> {
        Ok(Decision {
            action: Action::Promoted,
            reason: "ignores evidence".into(),
            new_state_id: Some(StateId::new()),
        })
    }
}

#[test]
fn loop_rejects_inconclusive_baseline_and_custom_selector_cannot_bypass_evidence() {
    let mut engine = EgriLoop::new(
        Propose,
        Execute,
        Evaluate,
        AlwaysPromote,
        BudgetController::new(2, None),
        Ledger::in_memory(),
    );
    assert!(engine.baseline(true).is_err());
    assert!(engine.best().is_none());
    assert!(engine.ledger().records().is_empty());
    engine.baseline(false).unwrap();
    let trial = engine.step().unwrap();
    assert_eq!(trial.decision.action, Action::Escalated);
    assert_eq!(
        trial.outcome.evidence.unwrap().claim_verdict,
        ClaimVerdict::InconclusiveNoPositiveControl
    );
    assert_eq!(engine.best(), Some(&false));
    assert_eq!(engine.ledger().records().len(), 2);
}
