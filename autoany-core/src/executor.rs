use crate::error::Result;
use crate::types::ExecutionResult;

/// Executes a candidate artifact inside the harness.
///
/// Implementations handle the domain-specific execution:
/// - local process spawning
/// - container execution
/// - API calls
/// - simulator invocations
pub trait Executor {
    /// The artifact type this executor operates on.
    type Artifact;

    /// Execute the candidate artifact and return raw results.
    fn execute(&self, artifact: &Self::Artifact) -> Result<ExecutionResult>;
}
