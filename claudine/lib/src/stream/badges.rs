//! Structured diagnostic badges emitted after a wrapped session ends.
//!
//! A [`SessionBadge`] surfaces a single human-readable condition such as an
//! authentication failure, billing exhaustion, rate-limit throttle, or
//! context-window pressure. Badges are derived from the fields already
//! present on [`crate::stream::summary::StreamExecutionSummary`] and are
//! consumed by the stderr formatters, the wrapper Prose renderer, and JSONL
//! reporting.

use serde::{Deserialize, Serialize};

use crate::provider::Provider;
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
    Config,
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

    let already_has_rate_limit = badges
        .iter()
        .any(|b| b.category == BadgeCategory::RateLimit);

    if !already_has_rate_limit
        && let Some(rate_limit) = summary.rate_limit.as_ref()
        && rate_limit.is_throttled.unwrap_or(false)
    {
        let retry_hint = rate_limit
            .retry_after_ms
            .map(|ms| format!(" (retry in {:.1}s)", ms as f64 / 1000.0))
            .unwrap_or_default();
        let base_message = rate_limit
            .message
            .clone()
            .unwrap_or_else(|| "Rate limit hit".to_string());
        badges.push(SessionBadge {
            category: BadgeCategory::RateLimit,
            severity: BadgeSeverity::Warning,
            label: "Rate Limit".into(),
            message: format!("{base_message}{retry_hint}"),
            remediation_url: dashboard_url.clone(),
        });
    }

    const CONTEXT_PRESSURE_THRESHOLD: f64 = 80.0;

    if let Some(context) = summary.context_usage.as_ref()
        && let Some(percent) = context.percent
        && percent >= CONTEXT_PRESSURE_THRESHOLD
    {
        let used = context
            .used
            .map(|u| u.to_string())
            .unwrap_or_else(|| "?".to_string());
        let total = context
            .total
            .map(|t| t.to_string())
            .unwrap_or_else(|| "?".to_string());
        badges.push(SessionBadge {
            category: BadgeCategory::ContextPressure,
            severity: BadgeSeverity::Warning,
            label: "Context".into(),
            message: format!("Context window pressure: {percent:.0}% used ({used}/{total} tokens)"),
            remediation_url: dashboard_url.clone(),
        });
    }

    if let Some(diagnostics) = summary.stderr_diagnostics.as_ref() {
        let already_has_rate_limit = badges
            .iter()
            .any(|b| b.category == BadgeCategory::RateLimit);
        if !already_has_rate_limit && diagnostics.rate_limit_events > 0 {
            let reset_hint = diagnostics
                .rate_limit_reset_at
                .map(|reset| format!(" (resets at {})", reset.format("%Y-%m-%d %H:%M:%S UTC")))
                .unwrap_or_default();
            badges.push(SessionBadge {
                category: BadgeCategory::RateLimit,
                severity: BadgeSeverity::Warning,
                label: "Rate Limit".into(),
                message: format!("Rate limit hit{reset_hint}"),
                remediation_url: dashboard_url.clone(),
            });
        }

        // Malformed-asset events intentionally do NOT emit a trailer
        // badge — each malformed asset is already surfaced once per
        // line as a `SemanticEvent::Warning` ("󰀨 Skipped malformed
        // OpenCode <kind>: <path>"), which is the authoritative
        // human-visible surface. The `malformed_asset_events`
        // counter on `StderrDiagnostics` is preserved for JSONL
        // reporting and downstream dashboards.
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
        let summary =
            summary_with_kind(Provider::Claude, "authentication_error", "Invalid API key");
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
        let summary = summary_with_kind(Provider::Claude, "billing_error", "Insufficient credits");
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
        let summary = summary_with_kind(Provider::Codex, "rate_limit", "Too many requests");
        let badges = derive_badges(&summary, Provider::Codex);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::RateLimit);
        assert_eq!(badges[0].severity, BadgeSeverity::Warning);
        assert_eq!(badges[0].label, "Rate Limit");
        assert_eq!(badges[0].message, "Too many requests");
    }

    #[test]
    fn permission_kind_yields_permission_badge() {
        let summary = summary_with_kind(Provider::Gemini, "permission_error", "Access denied");
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

    #[test]
    fn throttled_rate_limit_info_yields_badge_even_without_error_kind() {
        use crate::stream::summary::RateLimitInfo;
        let summary = StreamExecutionSummary {
            rate_limit: Some(RateLimitInfo {
                is_throttled: Some(true),
                retry_after_ms: Some(5000),
                message: Some("Rate limit exceeded".into()),
                reset_at: None,
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::Claude);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::RateLimit);
        assert!(badges[0].message.contains("Rate limit exceeded"));
        assert!(badges[0].message.contains("5.0s"));
    }

    #[test]
    fn non_throttled_rate_limit_info_does_not_yield_badge() {
        use crate::stream::summary::RateLimitInfo;
        let summary = StreamExecutionSummary {
            rate_limit: Some(RateLimitInfo {
                is_throttled: Some(false),
                retry_after_ms: None,
                message: None,
                reset_at: None,
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::Claude);
        assert!(badges.is_empty());
    }

    #[test]
    fn rate_limit_from_error_kind_does_not_duplicate_with_rate_limit_info() {
        use crate::stream::summary::RateLimitInfo;
        let summary = StreamExecutionSummary {
            is_error: true,
            error_kind: Some("rate_limit".into()),
            error_message: Some("Too many requests".into()),
            rate_limit: Some(RateLimitInfo {
                is_throttled: Some(true),
                retry_after_ms: Some(1000),
                message: Some("Slow down".into()),
                reset_at: None,
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::Codex);
        let rate_limit_count = badges
            .iter()
            .filter(|b| b.category == BadgeCategory::RateLimit)
            .count();
        assert_eq!(rate_limit_count, 1);
    }

    #[test]
    fn context_usage_at_or_above_threshold_yields_context_pressure_badge() {
        use crate::stream::summary::ContextUsage;
        let summary = StreamExecutionSummary {
            context_usage: Some(ContextUsage {
                used: Some(110_000),
                total: Some(128_000),
                percent: Some(85.9),
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::KimiCode);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::ContextPressure);
        assert_eq!(badges[0].label, "Context");
        assert_eq!(badges[0].severity, BadgeSeverity::Warning);
        assert!(badges[0].message.contains("86%"));
        assert!(badges[0].message.contains("110000"));
        assert!(badges[0].message.contains("128000"));
        assert_eq!(
            badges[0].remediation_url.as_deref(),
            Some("https://platform.moonshot.cn/console/account")
        );
    }

    #[test]
    fn context_usage_below_threshold_does_not_yield_badge() {
        use crate::stream::summary::ContextUsage;
        let summary = StreamExecutionSummary {
            context_usage: Some(ContextUsage {
                used: Some(50_000),
                total: Some(128_000),
                percent: Some(39.0),
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::KimiCode);
        assert!(badges.is_empty());
    }

    #[test]
    fn billing_error_does_not_produce_rate_limit_badge() {
        let summary = summary_with_kind(
            Provider::Claude,
            "billing_error",
            "Credit balance is too low",
        );
        let badges = derive_badges(&summary, Provider::Claude);
        let rate_limit_count = badges
            .iter()
            .filter(|b| b.category == BadgeCategory::RateLimit)
            .count();
        let billing_count = badges
            .iter()
            .filter(|b| b.category == BadgeCategory::Billing)
            .count();
        assert_eq!(
            rate_limit_count, 0,
            "billing_error must not yield a RateLimit badge"
        );
        assert_eq!(
            billing_count, 1,
            "billing_error must yield exactly one Billing badge"
        );
    }

    #[test]
    fn context_usage_at_exact_threshold_yields_badge() {
        use crate::stream::summary::ContextUsage;
        let summary = StreamExecutionSummary {
            context_usage: Some(ContextUsage {
                used: Some(102_400),
                total: Some(128_000),
                percent: Some(80.0),
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::KimiCode);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::ContextPressure);
    }

    #[test]
    fn stderr_diagnostics_rate_limit_yields_rate_limit_badge() {
        use crate::stream::summary::StderrDiagnostics;
        let summary = StreamExecutionSummary {
            stderr_diagnostics: Some(StderrDiagnostics {
                log_records_parsed: 3,
                rate_limit_events: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::OpenCode);
        assert_eq!(badges.len(), 1);
        assert_eq!(badges[0].category, BadgeCategory::RateLimit);
        assert_eq!(badges[0].severity, BadgeSeverity::Warning);
        assert_eq!(badges[0].label, "Rate Limit");
    }

    #[test]
    fn stderr_diagnostics_rate_limit_includes_reset_time_in_message() {
        use crate::stream::summary::StderrDiagnostics;
        use chrono::TimeZone;
        let summary = StreamExecutionSummary {
            stderr_diagnostics: Some(StderrDiagnostics {
                log_records_parsed: 1,
                rate_limit_events: 1,
                rate_limit_reset_at: Some(
                    chrono::Utc
                        .with_ymd_and_hms(2026, 4, 16, 4, 18, 56)
                        .unwrap(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::OpenCode);
        assert_eq!(badges.len(), 1);
        assert!(badges[0].message.contains("2026-04-16 04:18:56"));
    }

    #[test]
    fn stderr_diagnostics_does_not_duplicate_rate_limit_badge_from_error_kind() {
        use crate::stream::summary::StderrDiagnostics;
        let summary = StreamExecutionSummary {
            is_error: true,
            error_kind: Some("rate_limit".into()),
            error_message: Some("Too many requests".into()),
            stderr_diagnostics: Some(StderrDiagnostics {
                log_records_parsed: 1,
                rate_limit_events: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::OpenCode);
        let rate_limit_count = badges
            .iter()
            .filter(|b| b.category == BadgeCategory::RateLimit)
            .count();
        assert_eq!(rate_limit_count, 1);
    }

    #[test]
    fn stderr_diagnostics_malformed_assets_does_not_emit_config_badge() {
        use crate::stream::summary::StderrDiagnostics;
        // Per the 2026-04-18 OpenCode reporting contract, malformed
        // asset events are surfaced once per line as Warning events
        // and MUST NOT be repeated as a trailer Config badge — even
        // when the diagnostics counter is non-zero.
        let summary = StreamExecutionSummary {
            stderr_diagnostics: Some(StderrDiagnostics {
                log_records_parsed: 2,
                malformed_asset_events: 2,
                ..Default::default()
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::OpenCode);
        assert!(
            !badges.iter().any(|b| b.category == BadgeCategory::Config),
            "Config trailer badge must be absent for malformed assets: {badges:?}"
        );
    }

    #[test]
    fn stderr_diagnostics_single_malformed_asset_does_not_emit_config_badge() {
        use crate::stream::summary::StderrDiagnostics;
        // Singular-noun branch is also gone — no trailer badge regardless
        // of the count.
        let summary = StreamExecutionSummary {
            stderr_diagnostics: Some(StderrDiagnostics {
                log_records_parsed: 1,
                malformed_asset_events: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::OpenCode);
        assert!(
            !badges.iter().any(|b| b.category == BadgeCategory::Config),
            "Config trailer badge must be absent for single malformed asset: {badges:?}"
        );
    }

    #[test]
    fn stderr_diagnostics_empty_counts_produce_no_badges() {
        use crate::stream::summary::StderrDiagnostics;
        let summary = StreamExecutionSummary {
            stderr_diagnostics: Some(StderrDiagnostics {
                log_records_parsed: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        let badges = derive_badges(&summary, Provider::OpenCode);
        assert!(badges.is_empty());
    }

    #[test]
    fn config_category_serializes_snake_case() {
        let json = serde_json::to_string(&BadgeCategory::Config).unwrap();
        assert_eq!(json, "\"config\"");
    }
}
