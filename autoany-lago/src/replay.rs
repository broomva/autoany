//! Reconstruct a Ledger from Lago journal custom events.
//!
//! Enables cross-run inheritance: replay EGRI events from Lago's
//! journal to reconstruct the trial history of a previous run.

use autoany_core::ledger::Ledger;

use crate::ledger::{EGRI_EVENT_PREFIX, EgriTrialEvent};

/// Reconstruct a Ledger from a sequence of Lago custom event payloads.
///
/// Filters for events with the "egri." prefix, deserializes trial records,
/// and rebuilds the in-memory ledger.
pub fn replay_from_events(events: &[serde_json::Value]) -> autoany_core::Result<Ledger> {
    let mut ledger = Ledger::in_memory();

    for event in events {
        // Check for EGRI event prefix
        let event_type = event
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !event_type.starts_with(EGRI_EVENT_PREFIX) {
            continue;
        }

        // Deserialize the trial event
        if let Ok(egri_event) = serde_json::from_value::<EgriTrialEvent>(event.clone()) {
            ledger.append(egri_event.trial)?;
        } else {
            tracing::warn!(event_type, "failed to deserialize EGRI event, skipping");
        }
    }

    Ok(ledger)
}

/// Replay EGRI trials from all sessions in a hive task.
///
/// Filters events by the `"egri."` prefix and `hive_task_id` metadata,
/// then reconstructs a merged ledger from all agents' trials.
pub fn replay_hive_history(
    events: &[serde_json::Value],
    hive_task_id: &str,
) -> autoany_core::Result<Ledger> {
    let mut ledger = Ledger::in_memory();

    for event in events {
        // Check for hive_task_id match in metadata (if present)
        let meta_match = event
            .get("metadata")
            .and_then(|m| m.get("hive_task_id"))
            .and_then(|v| v.as_str())
            .is_some_and(|id| id == hive_task_id);

        // Also check session_id field in the event payload itself
        let payload_match = event
            .get("hive_task_id")
            .and_then(|v| v.as_str())
            .is_some_and(|id| id == hive_task_id);

        if !meta_match && !payload_match {
            continue;
        }

        // Check for EGRI event prefix
        let event_type = event
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !event_type.starts_with(EGRI_EVENT_PREFIX) {
            continue;
        }

        if let Ok(egri_event) = serde_json::from_value::<EgriTrialEvent>(event.clone()) {
            ledger.append(egri_event.trial)?;
        } else {
            tracing::warn!(
                event_type,
                "failed to deserialize EGRI hive event, skipping"
            );
        }
    }

    Ok(ledger)
}

/// Reconstruct a Ledger from a sequence of [`EgriTrialEvent`]s.
pub fn replay_from_trial_events(events: &[EgriTrialEvent]) -> autoany_core::Result<Ledger> {
    let mut ledger = Ledger::in_memory();
    for event in events {
        ledger.append(event.trial.clone())?;
    }
    Ok(ledger)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{EgriTrialEvent, LagoLedger};
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
    fn replay_filters_non_egri_events() {
        let events = vec![
            serde_json::json!({"event_type": "autonomic.heartbeat", "data": {}}),
            serde_json::json!({"event_type": "system.startup", "data": {}}),
            EgriTrialEvent::new(make_record("trial-001", Action::Promoted), None)
                .to_custom_payload(),
        ];

        let ledger = replay_from_events(&events).unwrap();
        assert_eq!(ledger.records().len(), 1);
        assert_eq!(ledger.records()[0].trial_id.0, "trial-001");
    }

    #[test]
    fn replay_from_trial_events_reconstructs_ledger() {
        let events = vec![
            EgriTrialEvent::new(make_record("baseline", Action::Promoted), None),
            EgriTrialEvent::new(make_record("trial-001", Action::Discarded), None),
            EgriTrialEvent::new(make_record("trial-002", Action::Promoted), None),
        ];

        let ledger = replay_from_trial_events(&events).unwrap();
        assert_eq!(ledger.records().len(), 3);
        assert_eq!(ledger.records()[0].trial_id.0, "baseline");
        assert_eq!(ledger.records()[1].trial_id.0, "trial-001");
        assert_eq!(ledger.records()[2].trial_id.0, "trial-002");
    }

    #[test]
    fn roundtrip_append_export_replay() {
        // 1. Append records to LagoLedger
        let mut lago = LagoLedger::new(Some("sess-round".into()));
        lago.append(make_record("baseline", Action::Promoted))
            .unwrap();
        lago.append(make_record("trial-001", Action::Discarded))
            .unwrap();
        lago.append(make_record("trial-002", Action::Promoted))
            .unwrap();

        // 2. Export as events
        let events = lago.export_events();
        assert_eq!(events.len(), 3);

        // 3. Convert to JSON payloads (simulating Lago storage)
        let payloads: Vec<serde_json::Value> =
            events.iter().map(|e| e.to_custom_payload()).collect();

        // 4. Replay from payloads
        let replayed = replay_from_events(&payloads).unwrap();
        assert_eq!(replayed.records().len(), 3);

        // 5. Verify records match
        let original = lago.ledger().records();
        let replayed_records = replayed.records();
        for (orig, repl) in original.iter().zip(replayed_records.iter()) {
            assert_eq!(orig.trial_id, repl.trial_id);
            assert_eq!(orig.decision.action, repl.decision.action);
            assert_eq!(orig.mutation.operator, repl.mutation.operator);
        }
    }

    #[test]
    fn replay_skips_malformed_egri_events() {
        let events = vec![
            // Valid EGRI event
            EgriTrialEvent::new(make_record("trial-001", Action::Promoted), None)
                .to_custom_payload(),
            // Malformed: correct prefix but missing trial field
            serde_json::json!({"event_type": "egri.trial", "bad_field": true}),
            // Valid EGRI event
            EgriTrialEvent::new(make_record("trial-002", Action::Discarded), None)
                .to_custom_payload(),
        ];

        let ledger = replay_from_events(&events).unwrap();
        // Only the two valid events should be replayed
        assert_eq!(ledger.records().len(), 2);
        assert_eq!(ledger.records()[0].trial_id.0, "trial-001");
        assert_eq!(ledger.records()[1].trial_id.0, "trial-002");
    }

    #[test]
    fn replay_empty_events() {
        let ledger = replay_from_events(&[]).unwrap();
        assert!(ledger.records().is_empty());

        let ledger = replay_from_trial_events(&[]).unwrap();
        assert!(ledger.records().is_empty());
    }

    #[test]
    fn replay_hive_history_filters_by_task_id() {
        let mut event1 = EgriTrialEvent::new(make_record("trial-001", Action::Promoted), None)
            .to_custom_payload();
        // Add hive_task_id at top level
        event1
            .as_object_mut()
            .unwrap()
            .insert("hive_task_id".into(), serde_json::json!("HIVE-A"));

        let mut event2 = EgriTrialEvent::new(make_record("trial-002", Action::Discarded), None)
            .to_custom_payload();
        event2
            .as_object_mut()
            .unwrap()
            .insert("hive_task_id".into(), serde_json::json!("HIVE-B"));

        let mut event3 = EgriTrialEvent::new(make_record("trial-003", Action::Promoted), None)
            .to_custom_payload();
        event3
            .as_object_mut()
            .unwrap()
            .insert("hive_task_id".into(), serde_json::json!("HIVE-A"));

        let events = vec![event1, event2, event3];

        let ledger = super::replay_hive_history(&events, "HIVE-A").unwrap();
        assert_eq!(ledger.records().len(), 2);
        assert_eq!(ledger.records()[0].trial_id.0, "trial-001");
        assert_eq!(ledger.records()[1].trial_id.0, "trial-003");
    }

    #[test]
    fn replay_hive_history_empty_when_no_match() {
        let ledger = super::replay_hive_history(&[], "HIVE-X").unwrap();
        assert!(ledger.records().is_empty());
    }
}
