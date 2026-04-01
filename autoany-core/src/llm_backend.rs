//! Stateless LLM call abstraction for autoreason debate.
//!
//! Each call to [`LlmBackend::generate`] is independent — no conversation
//! history is carried between calls. This enforces the context isolation
//! that makes autoreason's adversarial debate effective against sycophancy.

use crate::error::Result;

/// Abstraction over LLM API calls.
///
/// Each call is stateless — no conversation history. This enforces the
/// context isolation that makes autoreason work. Every debate phase
/// (critic, reviser, synthesizer, judge) gets a fresh context.
///
/// # Implementation Notes
///
/// Implementors should:
/// - Never maintain conversation state between calls
/// - Support configurable model selection for judge diversity
/// - Handle rate limiting and retries internally
/// - Return errors for API failures (do not silently fall back)
pub trait LlmBackend: Send + Sync {
    /// Generate a completion from a system prompt and user message.
    ///
    /// Each call is independent — no shared conversation state.
    fn generate(&self, system: &str, user: &str) -> Result<String>;

    /// Generate with a specific model (for judge diversity).
    ///
    /// When `model_diversity` is enabled in [`DebateConfig`](crate::types::DebateConfig),
    /// different judges can use different models to reduce correlated biases.
    ///
    /// The default implementation ignores the model parameter and delegates
    /// to [`generate`](Self::generate).
    fn generate_with_model(&self, model: &str, system: &str, user: &str) -> Result<String> {
        let _ = model;
        self.generate(system, user)
    }
}
