use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::badges::SessionBadge;
use super::task_ledger::SubagentOutcome;
use super::token_usage::NormalizedTokenUsage;
use crate::provider_id::Provider;
/// Rate-limit info extracted from provider streams.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RateLimitInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_throttled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_at: Option<DateTime<Utc>>,
}

/// Stderr-derived diagnostics captured from provider logs.
///
/// Populated when a provider emits structured log records on stderr that
/// Claudine can parse and classify. Attached to [`StreamExecutionSummary`]
/// only when at least one structured log line was parsed in the session.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StderrDiagnostics {
    pub log_records_parsed: u32,
    pub rate_limit_events: u32,
    pub malformed_asset_events: u32,
    pub api_failures: u32,
    pub auth_failures: u32,
    pub uncaught_errors: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_reset_at: Option<DateTime<Utc>>,
}

/// Context window pressure info (Kimi-specific, extensible).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContextUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
}

/// Provider-agnostic summary of a structured-stream session.
///
/// Produced by stream parsers and consumed by:
/// - stdout reconstruction
/// - stderr summaries
/// - JSONL logging
/// - reporting ingestion
/// - compose error handling
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamExecutionSummary {
    pub provider: Provider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub assistant_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_status: Option<String>,
    pub exit_code: i32,
    pub is_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_api_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_turns: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<NormalizedTokenUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_prompts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_input_prompts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_usage: Option<ContextUsage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub badges: Vec<SessionBadge>,
    /// Normalized facts for every sub-agent task that did **not** reach a
    /// recognized successful terminal state (see [`TaskLedger`]).
    ///
    /// Empty on a clean run and omitted from JSON, so summaries written before
    /// this field existed still deserialize. When nonempty the run is a
    /// failure regardless of `exit_code`: this is the machine contract behind
    /// `error_kind: "incomplete_subagents"`, and it stays complete even when
    /// the operator-facing headline truncates.
    ///
    /// [`TaskLedger`]: super::task_ledger::TaskLedger
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subagent_outcomes: Vec<SubagentOutcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_summary: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_diagnostics: Option<StderrDiagnostics>,
}

impl Default for StreamExecutionSummary {
    fn default() -> Self {
        Self {
            provider: Provider::Claude,
            session_id: None,
            model: None,
            assistant_text: String::new(),
            provider_status: None,
            exit_code: 0,
            is_error: false,
            error_kind: None,
            error_message: None,
            duration_ms: None,
            duration_api_ms: None,
            num_turns: None,
            token_usage: None,
            cost_usd: None,
            tool_calls: None,
            permission_prompts: None,
            user_input_prompts: None,
            rate_limit: None,
            context_usage: None,
            badges: Vec::new(),
            subagent_outcomes: Vec::new(),
            raw_summary: None,
            stderr_text: None,
            stderr_diagnostics: None,
        }
    }
}

#[cfg(test)]
mod tests;
