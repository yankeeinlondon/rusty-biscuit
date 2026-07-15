//! Structured diagnostic badges emitted after a wrapped session ends.
//!
//! A [`SessionBadge`] surfaces a single human-readable condition such as an
//! authentication failure, billing exhaustion, rate-limit throttle, or
//! context-window pressure. Badges are derived from the fields already
//! present on [`crate::stream::summary::StreamExecutionSummary`] and are
//! consumed by the stderr formatters, the wrapper Prose renderer, and JSONL
//! reporting.

use serde::{Deserialize, Serialize};

use crate::provider_id::Provider;
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

impl BadgeSeverity {
    /// Stable snake_case identifier — matches the serde projection, so the
    /// `err.severity` facet string cannot silently drift from the wire form.
    pub fn as_str(self) -> &'static str {
        match self {
            BadgeSeverity::Info => "info",
            BadgeSeverity::Warning => "warning",
            BadgeSeverity::Error => "error",
        }
    }
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

/// Classify a provider `error_kind` string into its [`BadgeCategory`], if it
/// names a known auth/billing/quota/rate-limit/permission condition.
///
/// Shares the same lookup tables and `Auth > Billing > Quota > RateLimit >
/// Permission` precedence as [`derive_badges`], so the diagnostic facet layer
/// classifies an `error_kind` the same way the operator badge does. Returns
/// `None` for kinds that are not a cap/auth condition (e.g. a runaway/timeout
/// guard label).
pub fn badge_category_for_kind(kind: &str) -> Option<BadgeCategory> {
    if kind_matches(kind, AUTH_KINDS) {
        Some(BadgeCategory::Auth)
    } else if kind_matches(kind, BILLING_KINDS) {
        Some(BadgeCategory::Billing)
    } else if kind_matches(kind, QUOTA_KINDS) {
        Some(BadgeCategory::Quota)
    } else if kind_matches(kind, RATE_LIMIT_KINDS) {
        Some(BadgeCategory::RateLimit)
    } else if kind_matches(kind, PERMISSION_KINDS) {
        Some(BadgeCategory::Permission)
    } else {
        None
    }
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
mod tests;
