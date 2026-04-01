//! Comparative evaluation for subjective domains.
//!
//! Unlike [`Evaluator`](crate::evaluator::Evaluator) which scores a single
//! artifact in isolation, `ComparativeEvaluator` compares two artifacts to
//! determine which is better. This is the core abstraction that enables
//! autoreason-style adversarial debate evaluation.

use crate::error::Result;
use crate::types::ComparisonOutcome;

/// Compares two artifacts to determine which is better.
///
/// Used when no objective scalar metric exists. The comparison IS the
/// evaluation — there is no separate scoring step.
///
/// Each call to `compare` should be deterministic in procedure (same debate
/// config, same rubric) even though the LLM outputs may vary. The evaluator
/// itself is immutable — EGRI Law 3 still holds.
///
/// # Context Isolation
///
/// Implementations must ensure that each evaluation phase (critic, reviser,
/// synthesizer, judge) uses a fresh LLM context with no shared conversation
/// history. This is what eliminates sycophancy and anchoring biases.
pub trait ComparativeEvaluator {
    /// The artifact type to compare.
    type Artifact;

    /// Compare the incumbent and candidate artifacts, returning which is better.
    ///
    /// - `task`: the original task description (provides context for judges)
    /// - `incumbent`: the current best artifact
    /// - `candidate`: the proposed replacement
    ///
    /// The returned [`ComparisonOutcome`] contains the winner, confidence
    /// (judge agreement ratio), and the full debate transcript for the ledger.
    fn compare(
        &self,
        task: &str,
        incumbent: &Self::Artifact,
        candidate: &Self::Artifact,
    ) -> Result<ComparisonOutcome>;
}
