use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an artifact state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateId(pub String);

impl StateId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn baseline() -> Self {
        Self("baseline".to_string())
    }
}

impl Default for StateId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for StateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a trial.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrialId(pub String);

impl TrialId {
    pub fn new(n: usize) -> Self {
        Self(format!("trial-{n:03}"))
    }

    pub fn baseline() -> Self {
        Self("baseline".to_string())
    }
}

impl std::fmt::Display for TrialId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A score can be scalar or vector (multi-objective).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Score {
    Scalar(f64),
    Vector(std::collections::HashMap<String, f64>),
}

impl Score {
    /// Get the primary scalar score. For vectors, returns None.
    pub fn as_scalar(&self) -> Option<f64> {
        match self {
            Score::Scalar(v) => Some(*v),
            Score::Vector(_) => None,
        }
    }

    /// Get a named metric from a vector score, or the scalar value if named "score".
    pub fn get(&self, name: &str) -> Option<f64> {
        match self {
            Score::Scalar(v) if name == "score" => Some(*v),
            Score::Vector(map) => map.get(name).copied(),
            _ => None,
        }
    }
}

/// Description of a mutation applied to an artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mutation {
    pub operator: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hypothesis: Option<String>,
}

/// Result of executing a candidate artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub duration_secs: f64,
    pub exit_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Raw output from the execution, passed to the evaluator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
}

/// Outcome from evaluation: score + constraint status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub score: Score,
    pub constraints_passed: bool,
    #[serde(default)]
    pub constraint_violations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluator_metadata: Option<serde_json::Value>,
}

/// Decision made about a trial candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Promoted,
    Discarded,
    Branched,
    Escalated,
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Promoted => write!(f, "promoted"),
            Action::Discarded => write!(f, "discarded"),
            Action::Branched => write!(f, "branched"),
            Action::Escalated => write!(f, "escalated"),
        }
    }
}

/// Full decision record for a trial.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub action: Action,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_state_id: Option<StateId>,
}

/// A complete trial record — one row in the ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialRecord {
    pub trial_id: TrialId,
    pub timestamp: DateTime<Utc>,
    pub parent_state: StateId,
    pub mutation: Mutation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionResult>,
    pub outcome: Outcome,
    pub decision: Decision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_notes: Option<String>,
}

/// Optimization direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Minimize,
    Maximize,
}

/// Autonomy mode for the loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyMode {
    Suggestion,
    Sandbox,
    AutoPromote,
    Portfolio,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_scalar_access() {
        let s = Score::Scalar(0.95);
        assert_eq!(s.as_scalar(), Some(0.95));
        assert_eq!(s.get("score"), Some(0.95));
        assert_eq!(s.get("other"), None);
    }

    #[test]
    fn score_vector_access() {
        let mut map = std::collections::HashMap::new();
        map.insert("accuracy".into(), 0.9);
        map.insert("latency".into(), 0.5);
        let s = Score::Vector(map);
        assert_eq!(s.as_scalar(), None);
        assert_eq!(s.get("accuracy"), Some(0.9));
        assert_eq!(s.get("latency"), Some(0.5));
        assert_eq!(s.get("missing"), None);
    }

    #[test]
    fn trial_id_formatting() {
        assert_eq!(TrialId::new(1).to_string(), "trial-001");
        assert_eq!(TrialId::new(42).to_string(), "trial-042");
        assert_eq!(TrialId::baseline().to_string(), "baseline");
    }

    #[test]
    fn state_id_baseline() {
        let s = StateId::baseline();
        assert_eq!(s.0, "baseline");
    }

    #[test]
    fn action_display() {
        assert_eq!(Action::Promoted.to_string(), "promoted");
        assert_eq!(Action::Discarded.to_string(), "discarded");
        assert_eq!(Action::Escalated.to_string(), "escalated");
        assert_eq!(Action::Branched.to_string(), "branched");
    }

    #[test]
    fn score_serde_roundtrip() {
        let scalar = Score::Scalar(3.14);
        let json = serde_json::to_string(&scalar).unwrap();
        let back: Score = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_scalar().unwrap(), 3.14);
    }

    #[test]
    fn action_serde_roundtrip() {
        let json = serde_json::to_string(&Action::Promoted).unwrap();
        assert_eq!(json, "\"promoted\"");
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Action::Promoted);
    }
}
