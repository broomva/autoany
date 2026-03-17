use crate::error::Result;
use crate::types::{ExecutionResult, Outcome};

/// Scores execution results and checks constraints.
///
/// The evaluator is the most critical component in EGRI.
/// It must be immutable during the loop — never mutate the evaluator
/// and the artifact in the same trial.
pub trait Evaluator {
    /// The artifact type, for access during evaluation if needed.
    type Artifact;

    /// Evaluate execution results and produce a scored outcome.
    fn evaluate(
        &self,
        artifact: &Self::Artifact,
        execution: &ExecutionResult,
    ) -> Result<Outcome>;
}
