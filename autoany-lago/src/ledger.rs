//! Lago-compatible ledger for EGRI trial records.
//!
//! Wraps autoany_core's Ledger and adds serialization to/from
//! Lago EventKind::Custom format with "egri." prefix.

use autoany_core::ledger::Ledger;
use autoany_core::types::TrialRecord;
use serde::{Deserialize, Serialize};

/// Event type prefix for EGRI events in Lago.
pub const EGRI_EVENT_PREFIX: &str = "egri.";

/// Lago-compatible event payload for a trial record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgriTrialEvent {
    /// Event type: "egri.trial"
    pub event_type: String,
    /// The full trial record.
    pub trial: TrialRecord,
    /// Session context.
    pub session_id: Option<String>,
}

impl EgriTrialEvent {
    /// Create a new trial event.
    pub fn new(trial: TrialRecord, session_id: Option<String>) -> Self {
        Self {
            event_type: format!("{EGRI_EVENT_PREFIX}trial"),
            trial,
            session_id,
        }
    }

    /// Serialize to the format expected by Lago's EventKind::Custom.
    pub fn to_custom_payload(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// A ledger wrapper that produces Lago-compatible events.
///
/// Wraps autoany_core's in-memory Ledger and adds the ability to
/// export trial records as Lago EventKind::Custom payloads.
pub struct LagoLedger {
    inner: Ledger,
    session_id: Option<String>,
}

impl LagoLedger {
    /// Create a new LagoLedger backed by an in-memory ledger.
    pub fn new(session_id: Option<String>) -> Self {
        Self {
            inner: Ledger::in_memory(),
            session_id,
        }
    }

    /// Create from an existing ledger.
    pub fn from_ledger(ledger: Ledger, session_id: Option<String>) -> Self {
        Self {
            inner: ledger,
            session_id,
        }
    }

    /// Append a trial record and return the Lago-compatible event.
    pub fn append(&mut self, record: TrialRecord) -> autoany_core::Result<EgriTrialEvent> {
        let event = EgriTrialEvent::new(record.clone(), self.session_id.clone());
        self.inner.append(record)?;
        Ok(event)
    }

    /// Get the inner ledger reference.
    pub fn ledger(&self) -> &Ledger {
        &self.inner
    }

    /// Get a mutable reference to the inner ledger.
    pub fn ledger_mut(&mut self) -> &mut Ledger {
        &mut self.inner
    }

    /// Export all records as Lago-compatible events.
    pub fn export_events(&self) -> Vec<EgriTrialEvent> {
        self.inner
            .records()
            .iter()
            .map(|r| EgriTrialEvent::new(r.clone(), self.session_id.clone()))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use autoany_core::types::*;
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
    fn append_produces_correct_event_type() {
        let mut ledger = LagoLedger::new(Some("sess-001".into()));
        let record = make_record("trial-001", Action::Promoted);
        let event = ledger.append(record).unwrap();

        assert_eq!(event.event_type, "egri.trial");
        assert_eq!(event.session_id.as_deref(), Some("sess-001"));
        assert_eq!(event.trial.trial_id.0, "trial-001");
    }

    #[test]
    fn export_events_produces_correct_count() {
        let mut ledger = LagoLedger::new(None);
        ledger
            .append(make_record("baseline", Action::Promoted))
            .unwrap();
        ledger
            .append(make_record("trial-001", Action::Discarded))
            .unwrap();
        ledger
            .append(make_record("trial-002", Action::Promoted))
            .unwrap();

        let events = ledger.export_events();
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| e.event_type == "egri.trial"));
    }

    #[test]
    fn to_custom_payload_is_valid_json() {
        let record = make_record("trial-001", Action::Promoted);
        let event = EgriTrialEvent::new(record, Some("sess-001".into()));
        let payload = event.to_custom_payload();

        assert_eq!(payload["event_type"], "egri.trial");
        assert_eq!(payload["session_id"], "sess-001");
        assert!(payload.get("trial").is_some());
    }

    #[test]
    fn ledger_inner_accessible() {
        let mut lago = LagoLedger::new(None);
        lago.append(make_record("baseline", Action::Promoted))
            .unwrap();

        assert_eq!(lago.ledger().records().len(), 1);
        assert_eq!(lago.ledger().records()[0].trial_id.0, "baseline");
    }

    #[test]
    fn from_existing_ledger() {
        let mut inner = Ledger::in_memory();
        inner
            .append(make_record("baseline", Action::Promoted))
            .unwrap();

        let lago = LagoLedger::from_ledger(inner, Some("sess-x".into()));
        assert_eq!(lago.ledger().records().len(), 1);

        let events = lago.export_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].session_id.as_deref(), Some("sess-x"));
    }
}
