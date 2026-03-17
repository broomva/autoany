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
                .map(|l| serde_json::from_str(l))
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
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
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
