use super::agent_status::{AgenticCapLimit, AgenticStatusPlatform, CapEntry, CapWindowSize};
use super::error::AgentStatusError;
use crate::models::metadata::Datetime;

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

/// Parse Claude Code `/status` output (multi-line sections).
///
/// Expected format:
/// ```text
///   Current session
///   ████████████▌                                      25% used
///   Resets 5:59pm (America/Los_Angeles)
///
///   5h limit
///   ██████████                                         20% used
///   Resets 10:30pm (America/Los_Angeles)
/// ```
///
/// ## Errors
///
/// Returns `ParseError` if no cap entries can be extracted.
pub fn parse_claude_code_status(output: &str) -> Result<AgenticCapLimit, AgentStatusError> {
    let lines: Vec<&str> = output.lines().collect();
    let mut caps = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let pct = match extract_pct(line, "used") {
            Some(p) => p,
            None => continue,
        };

        // Look backward for the section label (non-empty, non-bar line)
        let label = find_label_backward(&lines, i);

        // Look forward for reset time
        let resets = find_reset_forward(&lines, i);

        let window_size = parse_window_from_label(&label);

        caps.push(CapEntry {
            label,
            usage: pct / 100.0,
            window_size,
            resets_at: Datetime(resets),
        });
    }

    if caps.is_empty() {
        return Err(AgentStatusError::ParseError(
            "No cap entries found in Claude Code output".to_string(),
        ));
    }

    Ok(AgenticCapLimit { caps })
}

/// Parse Codex `/status` output (single-line box format).
///
/// Expected format:
/// ```text
/// │  5h limit:             [████████████████████] 100% left (resets 22:05)          │
/// │  Weekly limit:         [█████░░░░░░░░░░░░░░░] 23% left (resets 14:03 on 11 Feb) │
/// ```
///
/// ## Errors
///
/// Returns `ParseError` if no cap entries can be extracted.
pub fn parse_codex_status(output: &str) -> Result<AgenticCapLimit, AgentStatusError> {
    let mut caps = Vec::new();

    for line in output.lines() {
        let pct_left = match extract_pct(line, "left") {
            Some(p) => p,
            None => continue,
        };

        // Usage is inverted: 100% left = 0% used
        let usage = (100.0 - pct_left) / 100.0;

        // Extract label: text before the `[` bracket
        let label = extract_codex_label(line);

        // Extract reset time from "(resets ...)"
        let resets = extract_resets_parenthetical(line);

        let window_size = parse_window_from_label(&label);

        caps.push(CapEntry {
            label,
            usage,
            window_size,
            resets_at: Datetime(resets),
        });
    }

    if caps.is_empty() {
        return Err(AgentStatusError::ParseError(
            "No cap entries found in Codex output".to_string(),
        ));
    }

    Ok(AgenticCapLimit { caps })
}

/// Extract a percentage value adjacent to a keyword (e.g., "25% used" or "100% left").
fn extract_pct(line: &str, suffix: &str) -> Option<f32> {
    let lower = line.to_lowercase();
    let pattern = format!("% {suffix}");
    let end_pos = lower.find(&pattern)?;

    // Walk backward from the `%` to find the number
    let before = &line[..end_pos];
    let num_str: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    num_str.parse::<f32>().ok()
}

/// Look backward from a percentage line to find the section label.
fn find_label_backward(lines: &[&str], from: usize) -> String {
    for i in (0..from).rev() {
        let trimmed = lines[i].trim();
        // Skip empty lines, bar-only lines (unicode block chars), and lines with %
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.contains("% used") || trimmed.contains("% left") {
            break; // Hit a previous section
        }
        // Skip lines that are only bar characters
        if trimmed.chars().all(|c| is_bar_char(c) || c.is_whitespace()) {
            continue;
        }
        return trimmed.to_string();
    }
    "unknown".to_string()
}

/// Look forward from a percentage line to find the reset time.
fn find_reset_forward(lines: &[&str], from: usize) -> String {
    for line in &lines[from + 1..] {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_lowercase();
        if let Some(rest) = lower.strip_prefix("resets ") {
            let start = trimmed.len() - rest.len();
            return trimmed[start..].trim().to_string();
        }
        if lower.starts_with("reset ") {
            let start = "reset ".len();
            return trimmed[start..].trim().to_string();
        }
        // If we hit another section label or percentage, stop
        if trimmed.contains("% used") || trimmed.contains("% left") {
            break;
        }
    }
    "unknown".to_string()
}

/// Extract the label from a Codex single-line format (text before `[`).
fn extract_codex_label(line: &str) -> String {
    if let Some(bracket_pos) = line.find('[') {
        let before = &line[..bracket_pos];
        // Strip box-drawing chars and whitespace
        let cleaned: String = before
            .chars()
            .filter(|c| !is_box_char(*c))
            .collect::<String>()
            .trim()
            .trim_end_matches(':')
            .trim()
            .to_string();
        if !cleaned.is_empty() {
            return cleaned;
        }
    }
    "unknown".to_string()
}

/// Extract reset time from parenthetical "(resets ...)" on the same line.
fn extract_resets_parenthetical(line: &str) -> String {
    let lower = line.to_lowercase();
    if let Some(start) = lower.find("(resets ") {
        let after = &line[start + 8..]; // skip "(resets "
        if let Some(end) = after.find(')') {
            return after[..end].trim().to_string();
        }
    }
    "unknown".to_string()
}

/// Parse a `CapWindowSize` from a label string.
fn parse_window_from_label(label: &str) -> CapWindowSize {
    let lower = label.to_lowercase();

    // Check for "Nh" pattern (e.g., "5h limit")
    for word in lower.split_whitespace() {
        if let Some(Ok(n)) = word.strip_suffix('h').map(|s| s.parse::<u8>()) {
            return CapWindowSize::Hours(n);
        }
    }

    if lower.contains("session") {
        return CapWindowSize::Session;
    }
    if lower.contains("daily") || lower.contains("day") {
        return CapWindowSize::Daily;
    }
    if lower.contains("week") {
        return CapWindowSize::Weekly;
    }
    if lower.contains("month") {
        return CapWindowSize::Monthly;
    }

    // Default to session if nothing matches
    CapWindowSize::Session
}

/// Check if a character is a Unicode block/bar character used in progress bars.
fn is_bar_char(c: char) -> bool {
    matches!(c, '\u{2580}'..='\u{259F}')
}

/// Check if a character is a box-drawing character.
fn is_box_char(c: char) -> bool {
    matches!(c, '│' | '┃' | '║' | '┊' | '┆' | '╎' | '╏')
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- extract_pct ---

    #[test]
    fn test_extract_pct_used() {
        assert_eq!(extract_pct("  25% used", "used"), Some(25.0));
        assert_eq!(extract_pct("100% used", "used"), Some(100.0));
        assert_eq!(extract_pct("  0% used", "used"), Some(0.0));
    }

    #[test]
    fn test_extract_pct_left() {
        assert_eq!(extract_pct("100% left", "left"), Some(100.0));
        assert_eq!(extract_pct("  23% left (resets 14:03)", "left"), Some(23.0));
    }

    #[test]
    fn test_extract_pct_none() {
        assert_eq!(extract_pct("no percentage here", "used"), None);
        assert_eq!(extract_pct("", "used"), None);
    }

    // --- parse_window_from_label ---

    #[test]
    fn test_parse_window_hours() {
        assert_eq!(parse_window_from_label("5h limit"), CapWindowSize::Hours(5));
        assert_eq!(parse_window_from_label("4h cap"), CapWindowSize::Hours(4));
        assert_eq!(parse_window_from_label("1h limit"), CapWindowSize::Hours(1));
    }

    #[test]
    fn test_parse_window_named() {
        assert_eq!(parse_window_from_label("Current session"), CapWindowSize::Session);
        assert_eq!(parse_window_from_label("Weekly limit"), CapWindowSize::Weekly);
        assert_eq!(parse_window_from_label("Monthly cap"), CapWindowSize::Monthly);
        assert_eq!(parse_window_from_label("Daily limit"), CapWindowSize::Daily);
    }

    #[test]
    fn test_parse_window_fallback() {
        assert_eq!(parse_window_from_label("unknown thing"), CapWindowSize::Session);
    }

    // --- Claude Code parser ---

    #[test]
    fn test_parse_claude_code_multi_section() {
        let output = "\
  Current session
  ████████████▌                                      25% used
  Resets 5:59pm (America/Los_Angeles)

  5h limit
  ██████████                                         20% used
  Resets 10:30pm (America/Los_Angeles)
";
        let result = parse_claude_code_status(output);
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        let limit = result.unwrap();
        assert_eq!(limit.caps.len(), 2);

        assert_eq!(limit.caps[0].label, "Current session");
        assert!((limit.caps[0].usage - 0.25).abs() < 0.01);
        assert_eq!(limit.caps[0].window_size, CapWindowSize::Session);
        assert!(limit.caps[0].resets_at.0.contains("5:59pm"));

        assert_eq!(limit.caps[1].label, "5h limit");
        assert!((limit.caps[1].usage - 0.20).abs() < 0.01);
        assert_eq!(limit.caps[1].window_size, CapWindowSize::Hours(5));
        assert!(limit.caps[1].resets_at.0.contains("10:30pm"));
    }

    #[test]
    fn test_parse_claude_code_four_sections() {
        let output = "\
  Current session
  ██                                                  4% used
  Resets 6:00pm

  5h limit
  █████                                              10% used
  Resets 11:00pm

  Daily limit
  ███████████████                                    30% used
  Resets tomorrow 12:00am

  Weekly limit
  ██████████████████████████████████████████████████  99% used
  Resets Mon 12:00am
";
        let result = parse_claude_code_status(output);
        assert!(result.is_ok());
        let limit = result.unwrap();
        assert_eq!(limit.caps.len(), 4);
        assert_eq!(limit.caps[0].window_size, CapWindowSize::Session);
        assert_eq!(limit.caps[1].window_size, CapWindowSize::Hours(5));
        assert_eq!(limit.caps[2].window_size, CapWindowSize::Daily);
        assert_eq!(limit.caps[3].window_size, CapWindowSize::Weekly);
    }

    #[test]
    fn test_parse_claude_code_empty() {
        let result = parse_claude_code_status("");
        assert!(result.is_err());
    }

    // --- Codex parser ---

    #[test]
    fn test_parse_codex_single_line() {
        let output = "\
│  5h limit:             [████████████████████] 100% left (resets 22:05)          │
│  Weekly limit:         [█████░░░░░░░░░░░░░░░] 23% left (resets 14:03 on 11 Feb) │
";
        let result = parse_codex_status(output);
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        let limit = result.unwrap();
        assert_eq!(limit.caps.len(), 2);

        assert_eq!(limit.caps[0].label, "5h limit");
        assert!((limit.caps[0].usage - 0.0).abs() < 0.01); // 100% left = 0% used
        assert_eq!(limit.caps[0].window_size, CapWindowSize::Hours(5));
        assert_eq!(limit.caps[0].resets_at.0, "22:05");

        assert_eq!(limit.caps[1].label, "Weekly limit");
        assert!((limit.caps[1].usage - 0.77).abs() < 0.01); // 23% left = 77% used
        assert_eq!(limit.caps[1].window_size, CapWindowSize::Weekly);
        assert_eq!(limit.caps[1].resets_at.0, "14:03 on 11 Feb");
    }

    #[test]
    fn test_parse_codex_empty() {
        let result = parse_codex_status("");
        assert!(result.is_err());
    }

    // --- dispatch ---

    #[test]
    fn test_parse_dispatch_claude() {
        let output = "  Current session\n  25% used\n  Resets 5pm\n";
        let result = parse_status_output(AgenticStatusPlatform::ClaudeCode, output);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_dispatch_codex() {
        let output = "│  5h limit:  [██████] 50% left (resets 10:00) │\n";
        let result = parse_status_output(AgenticStatusPlatform::Codex, output);
        assert!(result.is_ok());
    }

    // --- extract_codex_label ---

    #[test]
    fn test_extract_codex_label() {
        assert_eq!(
            extract_codex_label("│  5h limit:             [████] 100% left"),
            "5h limit"
        );
        assert_eq!(
            extract_codex_label("│  Weekly limit:         [█████░░░] 23% left"),
            "Weekly limit"
        );
    }

    // --- extract_resets_parenthetical ---

    #[test]
    fn test_extract_resets_parenthetical() {
        assert_eq!(
            extract_resets_parenthetical("100% left (resets 22:05)"),
            "22:05"
        );
        assert_eq!(
            extract_resets_parenthetical("23% left (resets 14:03 on 11 Feb)"),
            "14:03 on 11 Feb"
        );
        assert_eq!(
            extract_resets_parenthetical("no resets here"),
            "unknown"
        );
    }
}
