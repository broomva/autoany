//! Arcan-backed executor for EGRI loops.
//!
//! [`ArcanExecutor`] creates an Arcan session, runs a prompt, and maps
//! the Arcan response to [`autoany_core::types::ExecutionResult`].
//!
//! Mirror types are defined locally to avoid a hard dependency on Arcan
//! internals. On HTTP failure, execution returns a failure result (not panic),
//! following the advisory fallthrough pattern from `arcan-aios-adapters`.

use std::time::Duration;

use autoany_core::error::Result;
use autoany_core::types::ExecutionResult;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::warn;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors specific to the Arcan executor adapter.
#[derive(Error, Debug)]
pub enum ArcanExecutorError {
    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Arcan returned a non-success status.
    #[error("Arcan returned status {status}: {body}")]
    BadStatus { status: u16, body: String },

    /// Failed to parse Arcan response.
    #[error("parse error: {0}")]
    Parse(String),
}

// ---------------------------------------------------------------------------
// Mirror types (no dep on Arcan internals)
// ---------------------------------------------------------------------------

/// Policy hint for the Arcan session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcanPolicy {
    /// Allow all tool use.
    #[default]
    Permissive,
    /// Read-only: no file writes or command execution.
    ReadOnly,
    /// Custom policy name.
    Custom(String),
}

/// Configuration for the Arcan executor.
#[derive(Debug, Clone)]
pub struct ArcanExecutorConfig {
    /// Base URL of the Arcan daemon (e.g., `http://localhost:3000`).
    pub base_url: String,
    /// HTTP timeout in seconds.
    pub timeout_secs: u64,
    /// Policy hint for session creation.
    pub policy: ArcanPolicy,
}

impl Default for ArcanExecutorConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3000".to_string(),
            timeout_secs: 120,
            policy: ArcanPolicy::default(),
        }
    }
}

/// Request body for `POST /sessions`.
#[derive(Debug, Serialize)]
struct CreateSessionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    policy: Option<ArcanPolicy>,
}

/// Response from `POST /sessions`.
#[derive(Debug, Deserialize)]
struct CreateSessionResponse {
    session_id: String,
}

/// Request body for `POST /sessions/{id}/runs`.
#[derive(Debug, Serialize)]
struct CreateRunRequest {
    message: String,
}

/// Response from `POST /sessions/{id}/runs`.
#[derive(Debug, Deserialize)]
struct CreateRunResponse {
    #[serde(default)]
    #[allow(dead_code)]
    run_id: Option<String>,
    #[serde(default)]
    output: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<String>,
}

// ---------------------------------------------------------------------------
// ArcanExecutor
// ---------------------------------------------------------------------------

/// Executes EGRI artifacts by delegating to the Arcan agent runtime.
///
/// Creates an Arcan session, submits a prompt via the run API, and maps
/// the response to an [`ExecutionResult`]. HTTP failures produce execution
/// failures rather than panics (advisory semantics).
pub struct ArcanExecutor {
    client: reqwest::Client,
    config: ArcanExecutorConfig,
}

impl ArcanExecutor {
    /// Create a new Arcan executor with the given configuration.
    pub fn new(config: ArcanExecutorConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("failed to build reqwest client");
        Self { client, config }
    }

    /// Create a session and run a prompt, returning an [`ExecutionResult`].
    ///
    /// This is the primary async entry point. The synchronous [`execute`](Self::execute)
    /// method delegates to this via `tokio::runtime::Handle::current().block_on()`.
    pub async fn execute_async(&self, prompt: &str) -> Result<ExecutionResult> {
        let started = std::time::Instant::now();

        // 1. Create session
        let session_id = match self.create_session().await {
            Ok(id) => id,
            Err(e) => {
                warn!(error = %e, "failed to create Arcan session");
                return Ok(ExecutionResult {
                    duration_secs: started.elapsed().as_secs_f64(),
                    exit_code: 1,
                    error: Some(format!("session creation failed: {e}")),
                    output: None,
                });
            }
        };

        // 2. Run prompt
        match self.create_run(&session_id, prompt).await {
            Ok(run_resp) => {
                let duration = started.elapsed().as_secs_f64();
                if let Some(err) = run_resp.error {
                    Ok(ExecutionResult {
                        duration_secs: duration,
                        exit_code: 1,
                        error: Some(err),
                        output: run_resp.output,
                    })
                } else {
                    Ok(ExecutionResult {
                        duration_secs: duration,
                        exit_code: 0,
                        error: None,
                        output: run_resp.output,
                    })
                }
            }
            Err(e) => {
                warn!(error = %e, session_id, "failed to run Arcan prompt");
                Ok(ExecutionResult {
                    duration_secs: started.elapsed().as_secs_f64(),
                    exit_code: 1,
                    error: Some(format!("run failed: {e}")),
                    output: None,
                })
            }
        }
    }

    /// Synchronous execution entry point.
    ///
    /// Delegates to [`execute_async`](Self::execute_async) via the current
    /// Tokio runtime handle. Panics if called outside a Tokio runtime.
    pub fn execute(&self, prompt: &str) -> Result<ExecutionResult> {
        tokio::runtime::Handle::current().block_on(self.execute_async(prompt))
    }

    /// Create a new Arcan session.
    async fn create_session(&self) -> std::result::Result<String, ArcanExecutorError> {
        let url = format!("{}/sessions", self.config.base_url);
        let body = CreateSessionRequest {
            policy: Some(self.config.policy.clone()),
        };

        let resp = self.client.post(&url).json(&body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ArcanExecutorError::BadStatus {
                status: status.as_u16(),
                body,
            });
        }

        let session: CreateSessionResponse = resp.json().await?;
        Ok(session.session_id)
    }

    /// Create a run within an existing session.
    async fn create_run(
        &self,
        session_id: &str,
        message: &str,
    ) -> std::result::Result<CreateRunResponse, ArcanExecutorError> {
        let url = format!("{}/sessions/{session_id}/runs", self.config.base_url);
        let body = CreateRunRequest {
            message: message.to_string(),
        };

        let resp = self.client.post(&url).json(&body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ArcanExecutorError::BadStatus {
                status: status.as_u16(),
                body,
            });
        }

        let run_resp: CreateRunResponse = resp.json().await?;
        Ok(run_resp)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn successful_session_and_run() {
        let server = MockServer::start().await;

        // Mock session creation
        Mock::given(method("POST"))
            .and(path("/sessions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"session_id": "sess-001"})),
            )
            .mount(&server)
            .await;

        // Mock run creation
        Mock::given(method("POST"))
            .and(path("/sessions/sess-001/runs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "run_id": "run-001",
                "output": {"result": "optimized"},
                "error": null
            })))
            .mount(&server)
            .await;

        let config = ArcanExecutorConfig {
            base_url: server.uri(),
            timeout_secs: 10,
            policy: ArcanPolicy::Permissive,
        };
        let executor = ArcanExecutor::new(config);

        let result = executor.execute_async("optimize this").await.unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.error.is_none());
        assert!(result.output.is_some());
        assert!(result.duration_secs >= 0.0);
    }

    #[tokio::test]
    async fn session_creation_failure_returns_error_result() {
        let server = MockServer::start().await;

        // Mock session creation failure
        Mock::given(method("POST"))
            .and(path("/sessions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .mount(&server)
            .await;

        let config = ArcanExecutorConfig {
            base_url: server.uri(),
            timeout_secs: 10,
            policy: ArcanPolicy::Permissive,
        };
        let executor = ArcanExecutor::new(config);

        let result = executor.execute_async("optimize this").await.unwrap();
        assert_eq!(result.exit_code, 1);
        assert!(result.error.is_some());
        assert!(
            result
                .error
                .as_ref()
                .unwrap()
                .contains("session creation failed")
        );
    }

    #[tokio::test]
    async fn run_failure_returns_error_result() {
        let server = MockServer::start().await;

        // Mock successful session creation
        Mock::given(method("POST"))
            .and(path("/sessions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"session_id": "sess-002"})),
            )
            .mount(&server)
            .await;

        // Mock run failure
        Mock::given(method("POST"))
            .and(path("/sessions/sess-002/runs"))
            .respond_with(ResponseTemplate::new(502).set_body_string("bad gateway"))
            .mount(&server)
            .await;

        let config = ArcanExecutorConfig {
            base_url: server.uri(),
            timeout_secs: 10,
            policy: ArcanPolicy::Permissive,
        };
        let executor = ArcanExecutor::new(config);

        let result = executor.execute_async("optimize this").await.unwrap();
        assert_eq!(result.exit_code, 1);
        assert!(result.error.is_some());
        assert!(result.error.as_ref().unwrap().contains("run failed"));
    }

    #[tokio::test]
    async fn run_with_error_in_response() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/sessions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"session_id": "sess-003"})),
            )
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/sessions/sess-003/runs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "run_id": "run-003",
                "output": null,
                "error": "tool execution timed out"
            })))
            .mount(&server)
            .await;

        let config = ArcanExecutorConfig {
            base_url: server.uri(),
            timeout_secs: 10,
            policy: ArcanPolicy::Permissive,
        };
        let executor = ArcanExecutor::new(config);

        let result = executor.execute_async("optimize this").await.unwrap();
        assert_eq!(result.exit_code, 1);
        assert_eq!(result.error.as_deref(), Some("tool execution timed out"));
    }

    #[tokio::test]
    async fn unreachable_server_returns_error_result() {
        let config = ArcanExecutorConfig {
            base_url: "http://127.0.0.1:1".to_string(),
            timeout_secs: 1,
            policy: ArcanPolicy::Permissive,
        };
        let executor = ArcanExecutor::new(config);

        let result = executor.execute_async("optimize this").await.unwrap();
        assert_eq!(result.exit_code, 1);
        assert!(result.error.is_some());
    }
}
