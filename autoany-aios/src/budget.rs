//! Budget synchronization between Arcan's BudgetState and autoany's BudgetController.

use serde::Deserialize;
use tracing::warn;

/// Arcan budget state (mirrored locally).
#[derive(Debug, Clone, Deserialize)]
pub struct ArcanBudgetState {
    /// Remaining tokens in the budget.
    pub tokens_remaining: u64,
    /// Remaining time in milliseconds.
    pub time_remaining_ms: u64,
    /// Remaining cost in USD.
    pub cost_remaining_usd: f64,
    /// Remaining tool call count.
    pub tool_calls_remaining: u32,
    /// Remaining error budget.
    pub error_budget_remaining: u32,
}

/// Fetches Arcan budget state from the daemon.
///
/// Returns `None` on any failure (advisory semantics — budget fetch
/// failures should not block the EGRI loop).
pub async fn fetch_budget(base_url: &str, session_id: &str) -> Option<ArcanBudgetState> {
    let client = reqwest::Client::new();
    let url = format!("{base_url}/sessions/{session_id}/state");
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            // Extract budget from state response
            let body: serde_json::Value = resp.json().await.ok()?;
            serde_json::from_value(body.get("state")?.get("budget")?.clone()).ok()
        }
        Ok(_) | Err(_) => {
            warn!("failed to fetch arcan budget for session {session_id}");
            None
        }
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
    async fn fetch_budget_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/sessions/sess-001/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "state": {
                    "budget": {
                        "tokens_remaining": 50000,
                        "time_remaining_ms": 300000,
                        "cost_remaining_usd": 0.75,
                        "tool_calls_remaining": 20,
                        "error_budget_remaining": 3
                    }
                }
            })))
            .mount(&server)
            .await;

        let budget = fetch_budget(&server.uri(), "sess-001").await;
        let budget = budget.expect("should parse budget");
        assert_eq!(budget.tokens_remaining, 50000);
        assert_eq!(budget.time_remaining_ms, 300000);
        assert!((budget.cost_remaining_usd - 0.75).abs() < f64::EPSILON);
        assert_eq!(budget.tool_calls_remaining, 20);
        assert_eq!(budget.error_budget_remaining, 3);
    }

    #[tokio::test]
    async fn fetch_budget_server_error_returns_none() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/sessions/sess-001/state"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let budget = fetch_budget(&server.uri(), "sess-001").await;
        assert!(budget.is_none());
    }

    #[tokio::test]
    async fn fetch_budget_missing_budget_field_returns_none() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/sessions/sess-001/state"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"state": {"mode": "active"}})),
            )
            .mount(&server)
            .await;

        let budget = fetch_budget(&server.uri(), "sess-001").await;
        assert!(budget.is_none());
    }

    #[tokio::test]
    async fn fetch_budget_unreachable_returns_none() {
        let budget = fetch_budget("http://127.0.0.1:1", "sess-001").await;
        assert!(budget.is_none());
    }
}
