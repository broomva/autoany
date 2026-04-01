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

// ---------------------------------------------------------------------------
// Autoreason / debate types
// ---------------------------------------------------------------------------

/// Configuration for the autoreason debate protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateConfig {
    /// Number of judges in the panel (should be odd for clean majorities).
    #[serde(default = "default_judge_count")]
    pub judge_count: u32,
    /// Consecutive incumbent wins required to declare convergence.
    #[serde(default = "default_convergence_threshold")]
    pub convergence_threshold: u32,
    /// Whether to randomize version labels per judge (strongly recommended).
    #[serde(default = "default_true")]
    pub label_randomization: bool,
    /// Whether to use different LLM providers for different judges.
    #[serde(default)]
    pub model_diversity: bool,
    /// Optional rubric for critique and judging (domain-specific).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rubric: Option<String>,
    /// Maximum tokens per phase (controls cost).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_phase: Option<u32>,
}

fn default_judge_count() -> u32 {
    3
}

fn default_convergence_threshold() -> u32 {
    3
}

fn default_true() -> bool {
    true
}

impl Default for DebateConfig {
    fn default() -> Self {
        Self {
            judge_count: default_judge_count(),
            convergence_threshold: default_convergence_threshold(),
            label_randomization: true,
            model_diversity: false,
            rubric: None,
            max_tokens_per_phase: None,
        }
    }
}

/// Which version won a debate round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Winner {
    /// The incumbent (Version A) — no improvement found.
    Incumbent,
    /// The revision (Version B) — critique-driven improvement.
    Revision,
    /// The synthesis (Version AB) — combined strengths.
    Synthesis,
}

impl std::fmt::Display for Winner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Winner::Incumbent => write!(f, "incumbent"),
            Winner::Revision => write!(f, "revision"),
            Winner::Synthesis => write!(f, "synthesis"),
        }
    }
}

/// Severity of a critique issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    /// Undermines the core argument.
    Critical,
    /// Significant weakness.
    Major,
    /// Improvement opportunity.
    Minor,
}

/// A single issue identified during critique.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueIssue {
    /// Category: "logical_gap", "unsupported_claim", "missing_perspective", etc.
    pub category: String,
    /// How severe the issue is.
    pub severity: IssueSeverity,
    /// Description of the issue.
    pub description: String,
    /// Where in the artifact the issue occurs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Result of the critique phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueResult {
    /// Structured list of identified issues.
    pub issues: Vec<CritiqueIssue>,
    /// Raw critique text (for passing to reviser).
    pub raw_text: String,
}

/// A single judge's vote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeVote {
    /// Judge identifier (for audit trail).
    pub judge_id: String,
    /// Label-to-version mapping this judge saw (for audit trail).
    pub label_map: std::collections::HashMap<String, Winner>,
    /// Ranked preference: first element is the judge's top pick.
    pub ranking: Vec<Winner>,
    /// Free-text justification.
    pub justification: String,
}

/// Result of one complete autoreason round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateRound {
    /// Which round this is (1-indexed).
    pub round_number: u32,
    /// The incumbent content entering this round.
    pub incumbent_content: String,
    /// Critique produced by the adversarial critic.
    pub critique: CritiqueResult,
    /// Revised version addressing the critique.
    pub revision_content: String,
    /// Synthesis of incumbent and revision.
    pub synthesis_content: String,
    /// Individual judge votes.
    pub votes: Vec<JudgeVote>,
    /// Which version won this round.
    pub winner: Winner,
    /// Judge agreement ratio: 1.0 = unanimous, 0.33 = split.
    pub confidence: f64,
    /// The actual content of the winning version.
    pub winning_content: String,
}

/// Outcome of a comparative evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonOutcome {
    /// Which version won.
    pub winner: Winner,
    /// Judge agreement ratio: 1.0 = unanimous, 0.33 = split.
    pub confidence: f64,
    /// Full debate record (for ledger).
    pub round: DebateRound,
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
        let scalar = Score::Scalar(3.125);
        let json = serde_json::to_string(&scalar).unwrap();
        let back: Score = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_scalar().unwrap(), 3.125);
    }

    #[test]
    fn action_serde_roundtrip() {
        let json = serde_json::to_string(&Action::Promoted).unwrap();
        assert_eq!(json, "\"promoted\"");
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Action::Promoted);
    }

    // --- Autoreason / debate type tests ---

    #[test]
    fn debate_config_defaults() {
        let cfg = DebateConfig::default();
        assert_eq!(cfg.judge_count, 3);
        assert_eq!(cfg.convergence_threshold, 3);
        assert!(cfg.label_randomization);
        assert!(!cfg.model_diversity);
        assert!(cfg.rubric.is_none());
        assert!(cfg.max_tokens_per_phase.is_none());
    }

    #[test]
    fn debate_config_serde_roundtrip() {
        let cfg = DebateConfig {
            judge_count: 5,
            convergence_threshold: 4,
            label_randomization: false,
            model_diversity: true,
            rubric: Some("Evaluate clarity and coherence.".into()),
            max_tokens_per_phase: Some(2000),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: DebateConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.judge_count, 5);
        assert_eq!(back.convergence_threshold, 4);
        assert!(!back.label_randomization);
        assert!(back.model_diversity);
        assert_eq!(
            back.rubric.as_deref(),
            Some("Evaluate clarity and coherence.")
        );
        assert_eq!(back.max_tokens_per_phase, Some(2000));
    }

    #[test]
    fn winner_serde_roundtrip() {
        for winner in [Winner::Incumbent, Winner::Revision, Winner::Synthesis] {
            let json = serde_json::to_string(&winner).unwrap();
            let back: Winner = serde_json::from_str(&json).unwrap();
            assert_eq!(back, winner);
        }
    }

    #[test]
    fn winner_display() {
        assert_eq!(Winner::Incumbent.to_string(), "incumbent");
        assert_eq!(Winner::Revision.to_string(), "revision");
        assert_eq!(Winner::Synthesis.to_string(), "synthesis");
    }

    #[test]
    fn issue_severity_serde_roundtrip() {
        for sev in [
            IssueSeverity::Critical,
            IssueSeverity::Major,
            IssueSeverity::Minor,
        ] {
            let json = serde_json::to_string(&sev).unwrap();
            let back: IssueSeverity = serde_json::from_str(&json).unwrap();
            assert_eq!(back, sev);
        }
    }

    #[test]
    fn critique_result_serde_roundtrip() {
        let critique = CritiqueResult {
            issues: vec![
                CritiqueIssue {
                    category: "logical_gap".into(),
                    severity: IssueSeverity::Critical,
                    description: "Missing causal link between X and Y.".into(),
                    location: Some("paragraph 3".into()),
                },
                CritiqueIssue {
                    category: "unsupported_claim".into(),
                    severity: IssueSeverity::Minor,
                    description: "No citation for the 30% figure.".into(),
                    location: None,
                },
            ],
            raw_text: "The argument has two main issues...".into(),
        };
        let json = serde_json::to_string(&critique).unwrap();
        let back: CritiqueResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.issues.len(), 2);
        assert_eq!(back.issues[0].category, "logical_gap");
        assert_eq!(back.issues[0].severity, IssueSeverity::Critical);
        assert_eq!(back.issues[1].location, None);
        assert_eq!(back.raw_text, critique.raw_text);
    }

    #[test]
    fn judge_vote_serde_roundtrip() {
        let mut label_map = std::collections::HashMap::new();
        label_map.insert("Alpha".into(), Winner::Incumbent);
        label_map.insert("Beta".into(), Winner::Revision);
        label_map.insert("Gamma".into(), Winner::Synthesis);

        let vote = JudgeVote {
            judge_id: "judge-0".into(),
            label_map,
            ranking: vec![Winner::Revision, Winner::Synthesis, Winner::Incumbent],
            justification: "Beta is the strongest because...".into(),
        };
        let json = serde_json::to_string(&vote).unwrap();
        let back: JudgeVote = serde_json::from_str(&json).unwrap();
        assert_eq!(back.judge_id, "judge-0");
        assert_eq!(back.ranking[0], Winner::Revision);
        assert_eq!(back.label_map.len(), 3);
    }

    #[test]
    fn comparison_outcome_serde_roundtrip() {
        let round = DebateRound {
            round_number: 1,
            incumbent_content: "Version A text.".into(),
            critique: CritiqueResult {
                issues: vec![],
                raw_text: "No significant issues.".into(),
            },
            revision_content: "Version B text.".into(),
            synthesis_content: "Version AB text.".into(),
            votes: vec![],
            winner: Winner::Incumbent,
            confidence: 1.0,
            winning_content: "Version A text.".into(),
        };
        let outcome = ComparisonOutcome {
            winner: Winner::Incumbent,
            confidence: 1.0,
            round,
        };
        let json = serde_json::to_string(&outcome).unwrap();
        let back: ComparisonOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(back.winner, Winner::Incumbent);
        assert!((back.confidence - 1.0).abs() < f64::EPSILON);
        assert_eq!(back.round.round_number, 1);
        assert_eq!(back.round.winning_content, "Version A text.");
    }
}
