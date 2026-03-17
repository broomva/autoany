use serde::{Deserialize, Serialize};

use crate::types::{AutonomyMode, Direction};

/// Problem specification — the compiled EGRI instance definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemSpec {
    pub name: String,
    pub objective: Objective,
    #[serde(default)]
    pub constraints: Vec<String>,
    pub artifacts: Artifacts,
    pub execution: Execution,
    pub budget: Budget,
    pub promotion: Promotion,
    pub autonomy: Autonomy,
    #[serde(default)]
    pub search: Option<Search>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Objective {
    pub metric: String,
    pub direction: Direction,
    #[serde(default)]
    pub baseline: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifacts {
    pub mutable: Vec<ArtifactEntry>,
    pub immutable: Vec<ImmutableEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactEntry {
    pub path: String,
    #[serde(rename = "type")]
    pub artifact_type: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmutableEntry {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    pub backend: String,
    #[serde(default)]
    pub command: String,
    #[serde(default = "default_timeout")]
    pub timeout_s: u64,
    #[serde(default = "default_true")]
    pub sandbox: bool,
}

fn default_timeout() -> u64 {
    300
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    #[serde(default = "default_trials")]
    pub max_trials: usize,
    #[serde(default = "default_timeout")]
    pub time_per_trial_s: u64,
    pub total_time_s: Option<u64>,
    pub token_budget: Option<u64>,
    pub cost_budget: Option<f64>,
}

fn default_trials() -> usize {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Promotion {
    pub policy: PromotionPolicy,
    pub threshold: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionPolicy {
    KeepIfImproves,
    Pareto,
    Threshold,
    HumanGate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Autonomy {
    pub mode: AutonomyMode,
    #[serde(default)]
    pub escalation_triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Search {
    #[serde(default = "default_proposer")]
    pub proposer: String,
    #[serde(default)]
    pub strategy_notes: String,
}

fn default_proposer() -> String {
    "llm".to_string()
}
