use thiserror::Error;

#[derive(Error, Debug)]
pub enum EgriError {
    #[error("budget exhausted: {0}")]
    BudgetExhausted(String),

    #[error("constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    #[error("evaluation failed: {0}")]
    EvaluationFailed(String),

    #[error("no baseline established")]
    NoBaseline,

    #[error("rollback failed: no promoted state available")]
    RollbackFailed,

    #[error("escalation required: {0}")]
    EscalationRequired(String),

    #[error("ledger error: {0}")]
    LedgerError(String),

    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, EgriError>;
