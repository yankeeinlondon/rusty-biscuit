use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::badges::SessionBadge;
use super::token_usage::NormalizedTokenUsage;
use crate::provider::Provider;
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
            raw_summary: None,
            stderr_text: None,
            stderr_diagnostics: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_sensible() {
        let summary = StreamExecutionSummary::default();
        assert_eq!(summary.provider, Provider::Claude);
        assert_eq!(summary.assistant_text, "");
        assert_eq!(summary.exit_code, 0);
        assert!(!summary.is_error);
        assert!(summary.session_id.is_none());
        assert!(summary.model.is_none());
        assert!(summary.token_usage.is_none());
        assert!(summary.cost_usd.is_none());
    }

    #[test]
    fn serde_round_trip_full() {
        let summary = StreamExecutionSummary {
            provider: Provider::Gemini,
            session_id: Some("sess-123".into()),
            model: Some("gemini-2.5-pro".into()),
            assistant_text: "Hello world".into(),
            provider_status: Some("end_turn".into()),
            exit_code: 0,
            is_error: false,
            error_kind: None,
            error_message: None,
            duration_ms: Some(12345),
            duration_api_ms: Some(11000),
            num_turns: Some(3),
            token_usage: Some(NormalizedTokenUsage {
                input: Some(1000),
                output: Some(500),
                total: Some(1500),
                cache_read: Some(200),
            }),
            cost_usd: Some(0.0042),
            tool_calls: Some(5),
            permission_prompts: None,
            user_input_prompts: None,
            rate_limit: None,
            context_usage: None,
            badges: Vec::new(),
            raw_summary: Some(serde_json::json!({"stop_reason": "end_turn"})),
            stderr_text: Some("stderr text".into()),
            stderr_diagnostics: None,
        };
        let json = serde_json::to_string(&summary).unwrap();
        let restored: StreamExecutionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(summary, restored);
    }

    #[test]
    fn serde_round_trip_minimal() {
        let summary = StreamExecutionSummary::default();
        let json = serde_json::to_string(&summary).unwrap();
        let restored: StreamExecutionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(summary, restored);
    }

    #[test]
    fn serde_skips_none_fields() {
        let summary = StreamExecutionSummary::default();
        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("session_id"));
        assert!(!json.contains("model"));
        assert!(!json.contains("token_usage"));
        assert!(!json.contains("cost_usd"));
        assert!(!json.contains("rate_limit"));
        assert!(!json.contains("context_usage"));
        assert!(!json.contains("raw_summary"));
        assert!(!json.contains("stderr_text"));
    }

    #[test]
    fn rate_limit_info_round_trip() {
        let info = RateLimitInfo {
            is_throttled: Some(true),
            retry_after_ms: Some(5000),
            message: Some("Rate limit exceeded".into()),
            reset_at: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        let restored: RateLimitInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, restored);
    }

    #[test]
    fn rate_limit_info_round_trip_with_reset_at() {
        use chrono::TimeZone;
        let info = RateLimitInfo {
            is_throttled: Some(true),
            retry_after_ms: None,
            message: Some("Usage limit reached".into()),
            reset_at: Some(Utc.with_ymd_and_hms(2026, 4, 16, 4, 18, 56).unwrap()),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"reset_at\""));
        let restored: RateLimitInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, restored);
    }

    #[test]
    fn rate_limit_info_skips_reset_at_when_none() {
        let info = RateLimitInfo {
            is_throttled: Some(true),
            retry_after_ms: None,
            message: None,
            reset_at: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(!json.contains("reset_at"));
    }

    #[test]
    fn stderr_diagnostics_default_is_all_zero() {
        let diagnostics = StderrDiagnostics::default();
        assert_eq!(diagnostics.log_records_parsed, 0);
        assert_eq!(diagnostics.rate_limit_events, 0);
        assert_eq!(diagnostics.malformed_asset_events, 0);
        assert_eq!(diagnostics.api_failures, 0);
        assert_eq!(diagnostics.auth_failures, 0);
        assert_eq!(diagnostics.uncaught_errors, 0);
        assert!(diagnostics.rate_limit_reset_at.is_none());
    }

    #[test]
    fn stderr_diagnostics_round_trip_full() {
        use chrono::TimeZone;
        let diagnostics = StderrDiagnostics {
            log_records_parsed: 12,
            rate_limit_events: 1,
            malformed_asset_events: 2,
            api_failures: 1,
            auth_failures: 0,
            uncaught_errors: 3,
            rate_limit_reset_at: Some(Utc.with_ymd_and_hms(2026, 4, 16, 4, 18, 56).unwrap()),
        };
        let json = serde_json::to_string(&diagnostics).unwrap();
        let restored: StderrDiagnostics = serde_json::from_str(&json).unwrap();
        assert_eq!(diagnostics, restored);
    }

    #[test]
    fn stderr_diagnostics_skips_reset_at_when_none() {
        let diagnostics = StderrDiagnostics {
            log_records_parsed: 1,
            ..Default::default()
        };
        let json = serde_json::to_string(&diagnostics).unwrap();
        assert!(!json.contains("rate_limit_reset_at"));
    }

    #[test]
    fn summary_stderr_diagnostics_skipped_when_none() {
        let summary = StreamExecutionSummary::default();
        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("stderr_diagnostics"));
    }

    #[test]
    fn summary_stderr_diagnostics_round_trips_when_set() {
        let summary = StreamExecutionSummary {
            stderr_diagnostics: Some(StderrDiagnostics {
                log_records_parsed: 4,
                malformed_asset_events: 2,
                ..Default::default()
            }),
            ..Default::default()
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"stderr_diagnostics\""));
        let restored: StreamExecutionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.stderr_diagnostics, summary.stderr_diagnostics);
    }

    #[test]
    fn context_usage_round_trip() {
        let usage = ContextUsage {
            used: Some(50000),
            total: Some(128000),
            percent: Some(39.0625),
        };
        let json = serde_json::to_string(&usage).unwrap();
        let restored: ContextUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(usage, restored);
    }

    #[test]
    fn summary_with_error_state() {
        let summary = StreamExecutionSummary {
            is_error: true,
            error_kind: Some("billing_error".into()),
            error_message: Some("Insufficient credits".into()),
            exit_code: 1,
            ..Default::default()
        };
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("billing_error"));
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("billing_error"));
    }

    #[test]
    fn badges_default_is_empty_vec() {
        let summary = StreamExecutionSummary::default();
        assert!(summary.badges.is_empty());
    }

    #[test]
    fn empty_badges_vec_is_skipped_in_serde_output() {
        let summary = StreamExecutionSummary::default();
        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("badges"));
    }

    #[test]
    fn non_empty_badges_vec_round_trips() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let summary = StreamExecutionSummary {
            badges: vec![SessionBadge {
                category: BadgeCategory::Billing,
                severity: BadgeSeverity::Error,
                label: "Billing".into(),
                message: "Insufficient credits".into(),
                remediation_url: Some("https://console.anthropic.com/settings/billing".into()),
            }],
            ..Default::default()
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("Insufficient credits"));
        let restored: StreamExecutionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.badges.len(), 1);
        assert_eq!(restored.badges[0].category, BadgeCategory::Billing);
    }

    #[test]
    fn permission_counters_round_trip() {
        let summary = StreamExecutionSummary {
            permission_prompts: Some(3),
            user_input_prompts: Some(1),
            ..Default::default()
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"permission_prompts\":3"));
        assert!(json.contains("\"user_input_prompts\":1"));
        let restored: StreamExecutionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.permission_prompts, Some(3));
        assert_eq!(restored.user_input_prompts, Some(1));
    }

    #[test]
    fn permission_counters_skip_none() {
        let summary = StreamExecutionSummary::default();
        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("permission_prompts"));
        assert!(!json.contains("user_input_prompts"));
    }
}
