use crate::error::Result;
use crate::types::ExecutionResult;

/// Checks hard constraints on execution results.
///
/// Constraint violations cause immediate rejection — these are not tradeoffs.
pub trait ConstraintChecker {
    /// Check all constraints. Returns list of violations (empty = all passed).
    fn check(&self, execution: &ExecutionResult) -> Result<Vec<String>>;
}

/// Default constraint checker that enforces runtime budget and exit code.
pub struct RuntimeConstraint {
    pub max_duration_secs: f64,
}

impl ConstraintChecker for RuntimeConstraint {
    fn check(&self, execution: &ExecutionResult) -> Result<Vec<String>> {
        let mut violations = Vec::new();
        if execution.duration_secs > self.max_duration_secs {
            violations.push(format!(
                "runtime {:.2}s exceeds limit {:.2}s",
                execution.duration_secs, self.max_duration_secs
            ));
        }
        if execution.exit_code != 0 {
            violations.push(format!("non-zero exit code: {}", execution.exit_code));
        }
        Ok(violations)
    }
}
