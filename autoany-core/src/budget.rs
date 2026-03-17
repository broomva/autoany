use std::time::{Duration, Instant};

use crate::error::{EgriError, Result};
use crate::spec::Budget as BudgetSpec;

/// Enforces budget limits. Fails closed — never allows "one more try."
pub struct BudgetController {
    max_trials: usize,
    total_time: Option<Duration>,
    trials_used: usize,
    start_time: Option<Instant>,
}

impl BudgetController {
    pub fn from_spec(spec: &BudgetSpec) -> Self {
        Self {
            max_trials: spec.max_trials,
            total_time: spec.total_time_s.map(Duration::from_secs),
            trials_used: 0,
            start_time: None,
        }
    }

    pub fn new(max_trials: usize, total_time: Option<Duration>) -> Self {
        Self {
            max_trials,
            total_time,
            trials_used: 0,
            start_time: None,
        }
    }

    /// Start the budget clock. Call once before the loop begins.
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Check if budget allows another trial. Returns Err if exhausted.
    pub fn check(&self) -> Result<()> {
        if self.trials_used >= self.max_trials {
            return Err(EgriError::BudgetExhausted(format!(
                "trial limit reached ({}/{})",
                self.trials_used, self.max_trials
            )));
        }

        if let (Some(limit), Some(start)) = (self.total_time, self.start_time) {
            let elapsed = start.elapsed();
            if elapsed >= limit {
                return Err(EgriError::BudgetExhausted(format!(
                    "time limit reached ({:.1}s / {:.1}s)",
                    elapsed.as_secs_f64(),
                    limit.as_secs_f64()
                )));
            }
        }

        Ok(())
    }

    /// Record that a trial was consumed.
    pub fn consume(&mut self) {
        self.trials_used += 1;
    }

    /// How many trials remain.
    pub fn remaining(&self) -> usize {
        self.max_trials.saturating_sub(self.trials_used)
    }

    /// How many trials have been used.
    pub fn used(&self) -> usize {
        self.trials_used
    }

    /// Total trials allowed.
    pub fn max_trials(&self) -> usize {
        self.max_trials
    }
}
