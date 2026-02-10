use super::agent_status::{AgenticCapLimit, AgenticStatusPlatform, CapWindowSize};
use super::error::AgentStatusError;
use crate::models::metadata::Datetime;

/// Parse the output of `claude /status` to extract cap limits.
///
/// Expected output format (subject to change):
/// ```text
/// Claude Code vX.Y.Z
/// Usage: 45% of 5h cap (resets in 2h 15m)
/// Monthly: 12% of monthly cap
/// ```
///
/// ## Errors
///
/// Returns `ParseError` if the output cannot be parsed.
pub fn parse_claude_code_status(output: &str) -> Result<AgenticCapLimit, AgentStatusError> {
    // Parse percentage values from status output
    let short_usage = extract_percentage(output, &["usage:", "cap usage:"])
        .unwrap_or(0.0);
    let long_usage = extract_percentage(output, &["monthly:", "long term:"])
        .unwrap_or(0.0);

    Ok(AgenticCapLimit {
        short_cap_usage: short_usage / 100.0,
        short_cap_until: Datetime("unknown".to_string()),
        short_cap_window_size: CapWindowSize::Hours(5),
        long_cap_usage: long_usage / 100.0,
        long_cap_window_size: CapWindowSize::Monthly,
        long_cap_until: Datetime("unknown".to_string()),
    })
}

/// Parse the output of `codex --status` to extract cap limits.
pub fn parse_codex_status(output: &str) -> Result<AgenticCapLimit, AgentStatusError> {
    let short_usage = extract_percentage(output, &["usage:", "rate limit:"])
        .unwrap_or(0.0);
    let long_usage = extract_percentage(output, &["monthly:", "long term:"])
        .unwrap_or(0.0);

    Ok(AgenticCapLimit {
        short_cap_usage: short_usage / 100.0,
        short_cap_until: Datetime("unknown".to_string()),
        short_cap_window_size: CapWindowSize::Hours(4),
        long_cap_usage: long_usage / 100.0,
        long_cap_window_size: CapWindowSize::Weekly,
        long_cap_until: Datetime("unknown".to_string()),
    })
}

/// Extract a percentage value from text by searching for keywords.
fn extract_percentage(text: &str, keywords: &[&str]) -> Option<f32> {
    let lower = text.to_lowercase();
    for keyword in keywords {
        if let Some(pos) = lower.find(&keyword.to_lowercase()) {
            let after = &text[pos + keyword.len()..];
            // Look for a number followed by %
            for word in after.split_whitespace() {
                let cleaned = word.trim_end_matches('%');
                if let Ok(val) = cleaned.parse::<f32>() {
                    return Some(val);
                }
            }
        }
    }
    None
}

/// Dispatch to the correct parser based on platform.
pub fn parse_status_output(
    platform: AgenticStatusPlatform,
    output: &str,
) -> Result<AgenticCapLimit, AgentStatusError> {
    match platform {
        AgenticStatusPlatform::ClaudeCode => parse_claude_code_status(output),
        AgenticStatusPlatform::Codex => parse_codex_status(output),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_percentage() {
        assert_eq!(extract_percentage("usage: 45% of cap", &["usage:"]), Some(45.0));
        assert_eq!(extract_percentage("Monthly: 12% used", &["monthly:"]), Some(12.0));
        assert_eq!(extract_percentage("no match here", &["usage:"]), None);
    }

    #[test]
    fn test_parse_claude_code_status() {
        let output = "Claude Code v2.1.0\nUsage: 75% of 5h cap\nMonthly: 30% of monthly cap";
        let result = parse_claude_code_status(output);
        assert!(result.is_ok());
        let limit = result.unwrap();
        assert!((limit.short_cap_usage - 0.75).abs() < 0.01);
        assert!((limit.long_cap_usage - 0.30).abs() < 0.01);
    }

    #[test]
    fn test_parse_codex_status() {
        let output = "Codex v1.0\nUsage: 50% rate limited\nMonthly: 20% used";
        let result = parse_codex_status(output);
        assert!(result.is_ok());
        let limit = result.unwrap();
        assert!((limit.short_cap_usage - 0.50).abs() < 0.01);
        assert!((limit.long_cap_usage - 0.20).abs() < 0.01);
    }

    #[test]
    fn test_parse_dispatch() {
        let output = "Usage: 60%\nMonthly: 25%";
        let result = parse_status_output(AgenticStatusPlatform::ClaudeCode, output);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_empty_output() {
        let result = parse_claude_code_status("");
        assert!(result.is_ok());
        let limit = result.unwrap();
        assert_eq!(limit.short_cap_usage, 0.0);
    }
}
