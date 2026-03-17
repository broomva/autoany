use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{EgriError, Result};
use crate::types::{Action, TrialRecord};

/// Append-only trial ledger.
///
/// Records every trial: mutations, scores, decisions, and lineage.
/// The ledger is the memory of the EGRI loop.
pub struct Ledger {
    records: Vec<TrialRecord>,
    file_path: Option<PathBuf>,
}

impl Ledger {
    /// Create an in-memory ledger.
    pub fn in_memory() -> Self {
        Self {
            records: Vec::new(),
            file_path: None,
        }
    }

    /// Create a ledger backed by a JSONL file.
    pub fn with_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Load existing entries if file exists
        let records = if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(serde_json::from_str)
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| EgriError::LedgerError(format!("parse error: {e}")))?
        } else {
            Vec::new()
        };

        Ok(Self {
            records,
            file_path: Some(path),
        })
    }

    /// Append a trial record.
    pub fn append(&mut self, record: TrialRecord) -> Result<()> {
        // Write to file if backed
        if let Some(path) = &self.file_path {
            let mut file = OpenOptions::new().create(true).append(true).open(path)?;
            let line = serde_json::to_string(&record)?;
            writeln!(file, "{line}")?;
        }

        self.records.push(record);
        Ok(())
    }

    /// Get all records.
    pub fn records(&self) -> &[TrialRecord] {
        &self.records
    }

    /// Number of trials recorded (excluding baseline).
    pub fn trial_count(&self) -> usize {
        self.records
            .iter()
            .filter(|r| r.trial_id.0 != "baseline")
            .count()
    }

    /// Get the last promoted record.
    pub fn last_promoted(&self) -> Option<&TrialRecord> {
        self.records
            .iter()
            .rev()
            .find(|r| r.decision.action == Action::Promoted)
    }

    /// Count of consecutive non-improvements (for escalation triggers).
    pub fn consecutive_non_improvements(&self) -> usize {
        self.records
            .iter()
            .rev()
            .take_while(|r| r.decision.action != Action::Promoted)
            .count()
    }

    /// Get records filtered by action type.
    pub fn by_action(&self, action: Action) -> Vec<&TrialRecord> {
        self.records
            .iter()
            .filter(|r| r.decision.action == action)
            .collect()
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
    fn in_memory_ledger_append_and_read() {
        let mut ledger = Ledger::in_memory();
        assert_eq!(ledger.records().len(), 0);

        ledger
            .append(make_record("baseline", Action::Promoted))
            .unwrap();
        ledger
            .append(make_record("trial-001", Action::Discarded))
            .unwrap();

        assert_eq!(ledger.records().len(), 2);
        assert_eq!(ledger.trial_count(), 1); // excludes baseline
    }

    #[test]
    fn last_promoted() {
        let mut ledger = Ledger::in_memory();
        ledger
            .append(make_record("baseline", Action::Promoted))
            .unwrap();
        ledger
            .append(make_record("trial-001", Action::Discarded))
            .unwrap();
        ledger
            .append(make_record("trial-002", Action::Promoted))
            .unwrap();
        ledger
            .append(make_record("trial-003", Action::Discarded))
            .unwrap();

        let last = ledger.last_promoted().unwrap();
        assert_eq!(last.trial_id.0, "trial-002");
    }

    #[test]
    fn consecutive_non_improvements() {
        let mut ledger = Ledger::in_memory();
        ledger
            .append(make_record("baseline", Action::Promoted))
            .unwrap();
        ledger
            .append(make_record("trial-001", Action::Promoted))
            .unwrap();
        ledger
            .append(make_record("trial-002", Action::Discarded))
            .unwrap();
        ledger
            .append(make_record("trial-003", Action::Discarded))
            .unwrap();
        ledger
            .append(make_record("trial-004", Action::Discarded))
            .unwrap();

        assert_eq!(ledger.consecutive_non_improvements(), 3);
    }

    #[test]
    fn by_action_filter() {
        let mut ledger = Ledger::in_memory();
        ledger
            .append(make_record("baseline", Action::Promoted))
            .unwrap();
        ledger
            .append(make_record("trial-001", Action::Discarded))
            .unwrap();
        ledger
            .append(make_record("trial-002", Action::Promoted))
            .unwrap();

        assert_eq!(ledger.by_action(Action::Promoted).len(), 2);
        assert_eq!(ledger.by_action(Action::Discarded).len(), 1);
        assert_eq!(ledger.by_action(Action::Escalated).len(), 0);
    }

    #[test]
    fn file_backed_ledger_persistence() {
        let dir = std::env::temp_dir().join("autoany_test_ledger");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_ledger.jsonl");
        let _ = std::fs::remove_file(&path);

        // Write
        {
            let mut ledger = Ledger::with_file(&path).unwrap();
            ledger
                .append(make_record("baseline", Action::Promoted))
                .unwrap();
            ledger
                .append(make_record("trial-001", Action::Discarded))
                .unwrap();
        }

        // Re-read
        {
            let ledger = Ledger::with_file(&path).unwrap();
            assert_eq!(ledger.records().len(), 2);
            assert_eq!(ledger.records()[0].trial_id.0, "baseline");
            assert_eq!(ledger.records()[1].trial_id.0, "trial-001");
        }

        let _ = std::fs::remove_file(&path);
    }
}
