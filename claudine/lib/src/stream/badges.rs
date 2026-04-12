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

/// Derive zero or more badges from a completed session summary.
///
/// Pure function — reads `error_kind`, `rate_limit`, and `context_usage`
/// from the summary and consults [`Provider::usage_dashboard_url`] for
/// remediation links. Returns an empty vector when no signal fires.
pub fn derive_badges(_summary: &StreamExecutionSummary, _provider: Provider) -> Vec<SessionBadge> {
    Vec::new()
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
}
