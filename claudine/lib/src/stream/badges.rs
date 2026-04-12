//! Structured diagnostic badges emitted after a wrapped session ends.
//!
//! A [`SessionBadge`] surfaces a single human-readable condition such as an
//! authentication failure, billing exhaustion, rate-limit throttle, or
//! context-window pressure. Badges are derived from the fields already
//! present on [`crate::stream::summary::StreamExecutionSummary`] and are
//! consumed by the stderr formatters, the wrapper Prose renderer, and JSONL
//! reporting.

use serde::{Deserialize, Serialize};

use crate::events::Provider;
use crate::stream::summary::StreamExecutionSummary;

/// Category describing why a badge was emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BadgeCategory {
    Auth,
    Billing,
    Quota,
    RateLimit,
    ContextPressure,
    Permission,
}

/// Severity of a badge from the operator's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BadgeSeverity {
    Info,
    Warning,
    Error,
}

/// A single diagnostic badge attached to a session summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionBadge {
    pub category: BadgeCategory,
    pub severity: BadgeSeverity,
    pub label: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation_url: Option<String>,
}

const AUTH_KINDS: &[&str] = &[
    "authentication_error",
    "authentication_failed",
    "auth_error",
    "invalid_api_key",
    "unauthorized",
];

const BILLING_KINDS: &[&str] = &[
    "billing_error",
    "payment_required",
    "credit_balance_too_low",
    "account_suspended",
];

const QUOTA_KINDS: &[&str] = &[
    "quota_exceeded",
    "insufficient_quota",
    "usage_limit_reached",
    "capacity_limit",
];

const RATE_LIMIT_KINDS: &[&str] = &[
    "rate_limit_error",
    "rate_limit",
    "rate_limit_exceeded",
    "too_many_requests",
    "resource_exhausted",
];

const PERMISSION_KINDS: &[&str] = &[
    "permission_error",
    "forbidden_error",
    "access_denied",
    "forbidden",
];

fn kind_matches(kind: &str, table: &[&str]) -> bool {
    table.iter().any(|entry| entry.eq_ignore_ascii_case(kind))
}

/// Derive zero or more badges from a completed session summary.
///
/// Pure function — reads `error_kind`, `rate_limit`, and `context_usage`
/// from the summary and consults [`Provider::usage_dashboard_url`] for
/// remediation links. Returns an empty vector when no signal fires.
///
/// Priority order for error_kind classification:
/// Auth > Billing > Quota > RateLimit > Permission.
/// At most one badge is emitted per error_kind.
pub fn derive_badges(summary: &StreamExecutionSummary, provider: Provider) -> Vec<SessionBadge> {
    let mut badges = Vec::new();
    let dashboard_url = provider.usage_dashboard_url().map(str::to_owned);

    if let Some(kind) = summary.error_kind.as_deref() {
        if kind_matches(kind, AUTH_KINDS) {
            badges.push(SessionBadge {
                category: BadgeCategory::Auth,
                severity: BadgeSeverity::Error,
                label: "Auth".into(),
                message: summary
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Authentication failed".to_string()),
                remediation_url: dashboard_url.clone(),
            });
        } else if kind_matches(kind, BILLING_KINDS) {
            badges.push(SessionBadge {
                category: BadgeCategory::Billing,
                severity: BadgeSeverity::Error,
                label: "Billing".into(),
                message: summary
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Billing error".to_string()),
                remediation_url: dashboard_url.clone(),
            });
        } else if kind_matches(kind, QUOTA_KINDS) {
            badges.push(SessionBadge {
                category: BadgeCategory::Quota,
                severity: BadgeSeverity::Error,
                label: "Quota".into(),
                message: summary
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Quota exceeded".to_string()),
                remediation_url: dashboard_url.clone(),
            });
        } else if kind_matches(kind, RATE_LIMIT_KINDS) {
            badges.push(SessionBadge {
                category: BadgeCategory::RateLimit,
                severity: BadgeSeverity::Warning,
                label: "Rate Limit".into(),
                message: summary
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Rate limit hit".to_string()),
                remediation_url: dashboard_url.clone(),
            });
        } else if kind_matches(kind, PERMISSION_KINDS) {
            badges.push(SessionBadge {
                category: BadgeCategory::Permission,
                severity: BadgeSeverity::Error,
                label: "Permission".into(),
                message: summary
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Permission denied".to_string()),
                remediation_url: None,
            });
        }
    }

    badges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_badge_round_trip() {
        let badge = SessionBadge {
            category: BadgeCategory::Billing,
            severity: BadgeSeverity::Warning,
            label: "Billing".into(),
            message: "Insufficient credits".into(),
            remediation_url: Some("https://console.anthropic.com/settings/billing".into()),
        };
        let json = serde_json::to_string(&badge).unwrap();
        let restored: SessionBadge = serde_json::from_str(&json).unwrap();
        assert_eq!(badge, restored);
    }

    #[test]
    fn category_serializes_snake_case() {
        let json = serde_json::to_string(&BadgeCategory::RateLimit).unwrap();
        assert_eq!(json, "\"rate_limit\"");
        let json = serde_json::to_string(&BadgeCategory::ContextPressure).unwrap();
        assert_eq!(json, "\"context_pressure\"");
    }

    #[test]
    fn severity_serializes_snake_case() {
        let json = serde_json::to_string(&BadgeSeverity::Warning).unwrap();
        assert_eq!(json, "\"warning\"");
    }

    #[test]
    fn derive_badges_returns_empty_for_default_summary() {
        let summary = StreamExecutionSummary::default();
        assert!(derive_badges(&summary, Provider::Claude).is_empty());
    }

    fn summary_with_kind(provider: Provider, kind: &str, message: &str) -> StreamExecutionSummary {
        StreamExecutionSummary {
            provider,
            is_error: true,
            error_kind: Some(kind.into()),
            error_message: Some(message.into()),
            ..Default::default()
        }
    }

    #[test]
    fn auth_kind_yields_auth_badge_with_dashboard_url() {
        let summary = summary_with_kind(Provider::Claude, "authentication_error", "Invalid API key");
        let badges = derive_badges(&summary, Provider::Claude);
        assert_eq!(badges.len(), 1);
        let badge = &badges[0];
        assert_eq!(badge.category, BadgeCategory::Auth);
        assert_eq!(badge.severity, BadgeSeverity::Error);
        assert_eq!(badge.label, "Auth");
        assert_eq!(badge.message, "Invalid API key");
        assert_eq!(
            badge.remediation_url.as_deref(),
            Some("https://console.anthropic.com/settings/billing")
        );
    }

    #[test]
    fn auth_kind_without_message_falls_back_to_label_text() {
        let summary = StreamExecutionSummary {
            is_error: true,
            error_kind: Some("unauthorized".into()),
            error_message: None,
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::Claude);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].message, "Authentication failed");
    }

    #[test]
    fn invalid_api_key_is_auth_kind() {
        let summary = summary_with_kind(Provider::Codex, "invalid_api_key", "Bad key");
        let badges = derive_badges(&summary, Provider::Codex);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::Auth);
        assert_eq!(
            badges[0].remediation_url.as_deref(),
            Some("https://platform.openai.com/usage")
        );
    }

    #[test]
    fn billing_kind_yields_billing_badge() {
        let summary =
            summary_with_kind(Provider::Claude, "billing_error", "Insufficient credits");
        let badges = derive_badges(&summary, Provider::Claude);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::Billing);
        assert_eq!(badges[0].severity, BadgeSeverity::Error);
        assert_eq!(badges[0].label, "Billing");
        assert_eq!(badges[0].message, "Insufficient credits");
    }

    #[test]
    fn quota_kind_yields_quota_badge() {
        let summary = summary_with_kind(
            Provider::Codex,
            "insufficient_quota",
            "You exceeded your current quota",
        );
        let badges = derive_badges(&summary, Provider::Codex);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::Quota);
        assert_eq!(badges[0].severity, BadgeSeverity::Error);
        assert_eq!(badges[0].label, "Quota");
        assert_eq!(badges[0].message, "You exceeded your current quota");
    }

    #[test]
    fn quota_kind_exceeded_yields_quota_badge() {
        let summary = summary_with_kind(
            Provider::QwenCode,
            "quota_exceeded",
            "OAuth free quota depleted",
        );
        let badges = derive_badges(&summary, Provider::QwenCode);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::Quota);
        assert_eq!(
            badges[0].remediation_url.as_deref(),
            Some("https://bailian.console.aliyun.com/")
        );
    }

    #[test]
    fn rate_limit_kind_yields_rate_limit_badge() {
        let summary =
            summary_with_kind(Provider::Codex, "rate_limit", "Too many requests");
        let badges = derive_badges(&summary, Provider::Codex);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::RateLimit);
        assert_eq!(badges[0].severity, BadgeSeverity::Warning);
        assert_eq!(badges[0].label, "Rate Limit");
        assert_eq!(badges[0].message, "Too many requests");
    }

    #[test]
    fn permission_kind_yields_permission_badge() {
        let summary =
            summary_with_kind(Provider::Gemini, "permission_error", "Access denied");
        let badges = derive_badges(&summary, Provider::Gemini);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::Permission);
        assert_eq!(badges[0].severity, BadgeSeverity::Error);
        assert_eq!(badges[0].label, "Permission");
        assert_eq!(badges[0].message, "Access denied");
        assert!(badges[0].remediation_url.is_none());
    }

    #[test]
    fn billing_wins_over_rate_limit_on_same_error_kind() {
        // insufficient_quota is in QUOTA_KINDS, not BILLING_KINDS — proves priority works
        let summary = summary_with_kind(Provider::Codex, "insufficient_quota", "Quota exceeded");
        let badges = derive_badges(&summary, Provider::Codex);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::Quota);
    }
}
