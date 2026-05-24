use super::summary::StreamExecutionSummary;

/// Verbosity level derived from wrapper flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    /// Start summary + warnings + completion summary.
    Normal,
    /// Warnings + single compact completion line.
    Quiet,
    /// Nothing.
    Silent,
}

/// Format the session-start summary for stderr.
///
/// Returns `None` if there's insufficient information.
pub fn format_start_summary(summary: &StreamExecutionSummary) -> Option<String> {
    let mut parts = Vec::new();

    // Provider + session ID
    let provider_name = format!("{:?}", summary.provider).to_lowercase();
    if let Some(sid) = &summary.session_id {
        let short_id = if sid.len() > 12 { &sid[..12] } else { sid };
        parts.push(format!("{provider_name} session {short_id}"));
    } else {
        parts.push(format!("{provider_name} session"));
    }

    // Model
    if let Some(model) = &summary.model {
        parts.push(model.clone());
    }

    if summary.session_id.is_none() && summary.model.is_none() {
        return None;
    }

    Some(format!("\u{25b8} {}", parts.join(" \u{00b7} ")))
}

/// Format a warning line for stderr.
pub fn format_warning(message: &str) -> String {
    format!("\u{26a0} {message}")
}

/// Format the completion summary for stderr.
///
/// Returns `None` if there's insufficient information.
pub fn format_completion_summary(summary: &StreamExecutionSummary) -> Option<String> {
    let prefix = if summary.is_error {
        "\u{2717}"
    } else {
        "\u{2713}"
    };

    let mut parts = Vec::new();

    // Duration
    if let Some(ms) = summary.duration_ms {
        parts.push(format_duration(ms));
    }

    // Token usage
    if let Some(usage) = &summary.token_usage {
        let mut token_parts = Vec::new();
        if let Some(input) = usage.input {
            token_parts.push(format!("{} in", format_number(input)));
        }
        if let Some(output) = usage.output {
            token_parts.push(format!("{} out", format_number(output)));
        }
        if let Some(cache) = usage.cache_read
            && cache > 0
        {
            token_parts.push(format!("{} cache", format_number(cache)));
        }
        if !token_parts.is_empty() {
            parts.push(token_parts.join(" / "));
        }
    }

    // Cost
    if let Some(cost) = summary.cost_usd {
        parts.push(format_cost(cost));
    }

    // Tool calls
    if let Some(tc) = summary.tool_calls {
        parts.push(format!("{tc} tool{}", if tc == 1 { "" } else { "s" }));
    }

    // Permission prompts (Codex today; other providers leave None)
    if let Some(pp) = summary.permission_prompts {
        parts.push(format!(
            "{pp} permission prompt{}",
            if pp == 1 { "" } else { "s" }
        ));
    }

    // Error info
    if summary.is_error
        && let Some(msg) = &summary.error_message
    {
        parts.push(msg.clone());
    }

    if parts.is_empty() {
        return None;
    }

    let mut out = format!("{prefix} {}", parts.join(" \u{00b7} "));
    for badge in &summary.badges {
        out.push('\n');
        out.push_str(&format!(
            "\u{26a0} {} \u{2014} {}",
            badge.label, badge.message
        ));
        if let Some(url) = &badge.remediation_url {
            out.push('\n');
            out.push_str(&format!("  \u{2192} {url}"));
        }
    }
    Some(out)
}

/// Format a single compact completion line for `--quiet` mode.
///
/// Returns `None` if there's insufficient information.
pub fn format_compact_completion(summary: &StreamExecutionSummary) -> Option<String> {
    let prefix = if summary.is_error {
        "\u{2717}"
    } else {
        "\u{2713}"
    };

    let mut parts = Vec::new();

    // Duration
    if let Some(ms) = summary.duration_ms {
        parts.push(format_duration(ms));
    }

    // Compact token usage: input→output tokens
    if let Some(usage) = &summary.token_usage
        && let (Some(input), Some(output)) = (usage.input, usage.output)
    {
        parts.push(format!(
            "{}\u{2192}{} tokens",
            format_number(input),
            format_number(output)
        ));
    }

    // Cost
    if let Some(cost) = summary.cost_usd {
        parts.push(format_cost(cost));
    }

    if parts.is_empty() {
        return None;
    }

    let mut out = format!("{prefix} {}", parts.join(" \u{00b7} "));
    if !summary.badges.is_empty() {
        let labels: Vec<&str> = summary.badges.iter().map(|b| b.label.as_str()).collect();
        out.push_str(&format!(" | \u{26a0} {}", labels.join(", ")));
    }
    Some(out)
}

/// Formats a duration in milliseconds as a human-readable string.
pub fn format_duration(ms: u64) -> String {
    let secs = ms as f64 / 1000.0;
    if secs < 10.0 {
        format!("{secs:.1}s")
    } else {
        format!("{:.0}s", secs)
    }
}

/// Formats a token count with K/M suffixes.
pub fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Formats a USD cost with appropriate decimal places.
pub fn format_cost(cost: f64) -> String {
    if cost < 0.01 {
        format!("${cost:.4}")
    } else {
        format!("${cost:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_id::Provider;
    use crate::stream::summary::StreamExecutionSummary;
    use crate::stream::token_usage::NormalizedTokenUsage;

    fn full_summary() -> StreamExecutionSummary {
        StreamExecutionSummary {
            provider: Provider::Claude,
            session_id: Some("sess-abc123def456".into()),
            model: Some("claude-sonnet-4-20250514".into()),
            assistant_text: String::new(),
            provider_status: Some("end_turn".into()),
            exit_code: 0,
            is_error: false,
            error_kind: None,
            error_message: None,
            duration_ms: Some(12345),
            duration_api_ms: Some(11000),
            num_turns: Some(3),
            token_usage: Some(NormalizedTokenUsage {
                input: Some(1234),
                output: Some(567),
                total: Some(1801),
                cache_read: Some(89),
            }),
            cost_usd: Some(0.0042),
            tool_calls: Some(3),
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

    #[test]
    fn normal_mode_start_summary() {
        let summary = full_summary();
        let start = format_start_summary(&summary).unwrap();
        assert!(start.starts_with('\u{25b8}'));
        assert!(start.contains("claude session sess-abc123d"));
        assert!(start.contains("claude-sonnet-4-20250514"));
    }

    #[test]
    fn normal_mode_completion_summary() {
        let summary = full_summary();
        let completion = format_completion_summary(&summary).unwrap();
        assert!(completion.starts_with('\u{2713}'));
        assert!(completion.contains("12s"));
        assert!(completion.contains("1K in"));
        assert!(completion.contains("567 out"));
        assert!(completion.contains("89 cache"));
        assert!(completion.contains("$0.0042"));
        assert!(completion.contains("3 tools"));
    }

    #[test]
    fn quiet_mode_compact_completion() {
        let summary = full_summary();
        let compact = format_compact_completion(&summary).unwrap();
        assert!(compact.starts_with('\u{2713}'));
        assert!(compact.contains("12s"));
        assert!(compact.contains("\u{2192}")); // arrow
        assert!(compact.contains("$0.0042"));
    }

    #[test]
    fn silent_mode_produces_nothing() {
        // Silent mode is handled by the caller — these formatters
        // simply aren't called. Verify the enum exists.
        assert_eq!(Verbosity::Silent, Verbosity::Silent);
    }

    #[test]
    fn missing_fields_gracefully_omitted() {
        let summary = StreamExecutionSummary::default();
        // Start summary returns None without enough info
        assert!(format_start_summary(&summary).is_none());
        // Completion returns None with no data
        assert!(format_completion_summary(&summary).is_none());
        assert!(format_compact_completion(&summary).is_none());
    }

    #[test]
    fn error_summary_shows_error() {
        let summary = StreamExecutionSummary {
            is_error: true,
            error_message: Some("Billing error".into()),
            duration_ms: Some(5000),
            ..Default::default()
        };
        let completion = format_completion_summary(&summary).unwrap();
        assert!(completion.starts_with('\u{2717}')); // ✗
        assert!(completion.contains("Billing error"));
    }

    #[test]
    fn warning_format() {
        let warning = format_warning("Rate limit exceeded");
        assert!(warning.contains("\u{26a0}"));
        assert!(warning.contains("Rate limit exceeded"));
    }

    #[test]
    fn large_numbers_formatted() {
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1K");
        assert_eq!(format_number(1500), "2K");
        assert_eq!(format_number(1_000_000), "1.0M");
        assert_eq!(format_number(2_500_000), "2.5M");
    }

    #[test]
    fn duration_formatting() {
        assert_eq!(format_duration(500), "0.5s");
        assert_eq!(format_duration(9999), "10.0s");
        assert_eq!(format_duration(12345), "12s");
        assert_eq!(format_duration(60000), "60s");
    }

    #[test]
    fn single_tool_no_plural() {
        let summary = StreamExecutionSummary {
            tool_calls: Some(1),
            duration_ms: Some(1000),
            ..Default::default()
        };
        let completion = format_completion_summary(&summary).unwrap();
        assert!(completion.contains("1 tool"));
        assert!(!completion.contains("1 tools"));
    }

    #[test]
    fn completion_summary_renders_single_badge_with_url() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let mut summary = full_summary();
        summary.badges = vec![SessionBadge {
            category: BadgeCategory::Billing,
            severity: BadgeSeverity::Error,
            label: "Billing".into(),
            message: "Insufficient credits".into(),
            remediation_url: Some("https://console.anthropic.com/settings/billing".into()),
        }];
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(rendered.contains("\u{2713}"));
        assert!(rendered.contains("\u{26a0}"));
        assert!(rendered.contains("Billing"));
        assert!(rendered.contains("Insufficient credits"));
        assert!(rendered.contains("https://console.anthropic.com/settings/billing"));
        assert!(rendered.lines().count() >= 3);
    }

    #[test]
    fn completion_summary_renders_badge_without_url() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let mut summary = full_summary();
        summary.badges = vec![SessionBadge {
            category: BadgeCategory::Permission,
            severity: BadgeSeverity::Error,
            label: "Permission".into(),
            message: "Access denied".into(),
            remediation_url: None,
        }];
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(rendered.contains("Permission"));
        assert!(rendered.contains("Access denied"));
        assert!(!rendered.contains("\u{2192}"));
    }

    #[test]
    fn completion_summary_renders_multiple_badges() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let mut summary = full_summary();
        summary.badges = vec![
            SessionBadge {
                category: BadgeCategory::RateLimit,
                severity: BadgeSeverity::Warning,
                label: "Rate Limit".into(),
                message: "Slow down".into(),
                remediation_url: None,
            },
            SessionBadge {
                category: BadgeCategory::ContextPressure,
                severity: BadgeSeverity::Warning,
                label: "Context".into(),
                message: "Context window pressure: 86% used".into(),
                remediation_url: None,
            },
        ];
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(rendered.contains("Rate Limit"));
        assert!(rendered.contains("Slow down"));
        assert!(rendered.contains("Context"));
        assert!(rendered.contains("86%"));
    }

    #[test]
    fn completion_summary_without_badges_is_single_line() {
        let summary = full_summary();
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(!rendered.contains("\u{26a0}"));
        assert_eq!(rendered.lines().count(), 1);
    }

    #[test]
    fn compact_completion_shows_badge_indicator_with_label() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let mut summary = full_summary();
        summary.badges = vec![SessionBadge {
            category: BadgeCategory::Billing,
            severity: BadgeSeverity::Error,
            label: "Billing".into(),
            message: "Insufficient credits".into(),
            remediation_url: None,
        }];
        let rendered = format_compact_completion(&summary).unwrap();
        assert!(rendered.contains("|"));
        assert!(rendered.contains("\u{26a0}"));
        assert!(rendered.contains("Billing"));
        assert!(!rendered.contains("Insufficient credits"));
    }

    #[test]
    fn compact_completion_shows_multiple_badge_labels_comma_separated() {
        use crate::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        let mut summary = full_summary();
        summary.badges = vec![
            SessionBadge {
                category: BadgeCategory::RateLimit,
                severity: BadgeSeverity::Warning,
                label: "Rate Limit".into(),
                message: "Slow down".into(),
                remediation_url: None,
            },
            SessionBadge {
                category: BadgeCategory::ContextPressure,
                severity: BadgeSeverity::Warning,
                label: "Context".into(),
                message: "86%".into(),
                remediation_url: None,
            },
        ];
        let rendered = format_compact_completion(&summary).unwrap();
        assert!(rendered.contains("Rate Limit"));
        assert!(rendered.contains("Context"));
        assert!(rendered.contains(", "));
    }

    #[test]
    fn compact_completion_without_badges_is_single_line() {
        let summary = full_summary();
        let rendered = format_compact_completion(&summary).unwrap();
        assert!(!rendered.contains('|'));
        assert_eq!(rendered.lines().count(), 1);
    }

    #[test]
    fn completion_summary_includes_permission_prompts() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            permission_prompts: Some(3),
            ..Default::default()
        };
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(rendered.contains("3 permission prompts"));
    }

    #[test]
    fn completion_summary_singular_permission_prompt() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(18_000),
            permission_prompts: Some(1),
            ..Default::default()
        };
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(rendered.contains("1 permission prompt"));
        assert!(!rendered.contains("permission prompts"));
    }

    #[test]
    fn completion_summary_omits_permission_when_none() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            ..Default::default()
        };
        let rendered = format_completion_summary(&summary).unwrap();
        assert!(!rendered.contains("permission"));
    }

    #[test]
    fn compact_completion_ignores_permission_prompts() {
        let summary = StreamExecutionSummary {
            duration_ms: Some(18_000),
            permission_prompts: Some(3),
            user_input_prompts: Some(1),
            ..Default::default()
        };
        let rendered = format_compact_completion(&summary).unwrap();
        assert!(!rendered.contains("permission"));
        assert!(!rendered.contains("user input"));
    }
}
