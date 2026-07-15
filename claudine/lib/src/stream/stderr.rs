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
mod tests;
