use serde::{Deserialize, Serialize};

use super::badges::SessionBadge;
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

/// Rate-limit info extracted from provider streams.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RateLimitInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_throttled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
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
    pub rate_limit: Option<RateLimitInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_usage: Option<ContextUsage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub badges: Vec<SessionBadge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_summary: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_text: Option<String>,
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
            rate_limit: None,
            context_usage: None,
            badges: Vec::new(),
            raw_summary: None,
            stderr_text: None,
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
            rate_limit: None,
            context_usage: None,
            badges: Vec::new(),
            raw_summary: Some(serde_json::json!({"stop_reason": "end_turn"})),
            stderr_text: Some("stderr text".into()),
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
        };
        let json = serde_json::to_string(&info).unwrap();
        let restored: RateLimitInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, restored);
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
}
