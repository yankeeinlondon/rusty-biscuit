//! iTerm2 macOS preference queries.

#[cfg(target_os = "macos")]
use super::parser::TerminalFontParser;

#[cfg(target_os = "macos")]
pub struct ITerm2Parser;

#[cfg(target_os = "macos")]
impl TerminalFontParser for ITerm2Parser {
    fn parse_font_name(_content: &str) -> Option<String> {
        // iTerm2 uses macOS preferences instead of a config file.
        query_iterm2_font_name()
    }

    fn parse_font_size(_content: &str) -> Option<u32> {
        query_iterm2_font_size()
    }
}

/// Query iTerm2 for font name using macOS `defaults` command.
///
/// iTerm2 stores font settings in macOS preferences as "Normal Font" = "FontName Size".
/// For example: "Monaco 12" or "JetBrainsMono Nerd Font 14".
#[cfg(target_os = "macos")]
pub(super) fn query_iterm2_font_name() -> Option<String> {
    use std::process::Command;

    // Query the "New Bookmarks" array which contains profile settings
    let output = Command::new("defaults")
        .args(["read", "com.googlecode.iterm2", "New Bookmarks"])
        .output()
        .ok()?;

    if !output.status.success() {
        tracing::debug!("query_iterm2_font_name(): defaults read failed");
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_iterm2_font_setting(&stdout, true)
}

/// Query iTerm2 for font size using macOS `defaults` command.
#[cfg(target_os = "macos")]
pub(super) fn query_iterm2_font_size() -> Option<u32> {
    use std::process::Command;

    let output = Command::new("defaults")
        .args(["read", "com.googlecode.iterm2", "New Bookmarks"])
        .output()
        .ok()?;

    if !output.status.success() {
        tracing::debug!("query_iterm2_font_size(): defaults read failed");
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_iterm2_font_setting(&stdout, false).and_then(|s| s.parse::<u32>().ok())
}

/// Parse iTerm2 font setting from `defaults read` output.
///
/// The output format is a plist-like structure with "Normal Font" = "FontName Size".
/// We extract either the font name (without size) or just the size.
///
/// ## Arguments
/// * `content` - The output from `defaults read`
/// * `extract_name` - If true, extract font name; if false, extract size
#[cfg(target_os = "macos")]
fn parse_iterm2_font_setting(content: &str, extract_name: bool) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();

        // Look for "Normal Font" = "FontName Size";
        if line.contains("\"Normal Font\"") {
            // Extract the value between the last pair of quotes
            if let Some(eq_pos) = line.find('=') {
                let value_part = line[eq_pos + 1..].trim();
                // Remove surrounding quotes and semicolon
                let value = value_part.trim_matches(|c| c == '"' || c == ';' || c == ' ');

                if value.is_empty() {
                    return None;
                }

                // Split "FontName Size" - the size is the last space-separated token
                // Handle fonts like "JetBrainsMono Nerd Font 14"
                if let Some(last_space) = value.rfind(' ') {
                    let potential_size = &value[last_space + 1..];
                    // Check if the last part is a number (the size)
                    if potential_size.parse::<f64>().is_ok() {
                        if extract_name {
                            // Return everything before the size
                            return Some(value[..last_space].to_string());
                        } else {
                            // Return just the size
                            return Some(potential_size.to_string());
                        }
                    }
                }

                // If we couldn't parse size, return the whole thing as name
                if extract_name {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iterm2_font_setting_basic() {
        let content = r#"
            (
                {
                    "Normal Font" = "Monaco 12";
                }
            )
        "#;
        assert_eq!(
            parse_iterm2_font_setting(content, true),
            Some("Monaco".to_string())
        );
        assert_eq!(
            parse_iterm2_font_setting(content, false),
            Some("12".to_string())
        );
    }

    #[test]
    fn test_parse_iterm2_font_setting_nerd_font() {
        let content = r#"
            (
                {
                    "Normal Font" = "JetBrainsMono Nerd Font 14";
                }
            )
        "#;
        assert_eq!(
            parse_iterm2_font_setting(content, true),
            Some("JetBrainsMono Nerd Font".to_string())
        );
        assert_eq!(
            parse_iterm2_font_setting(content, false),
            Some("14".to_string())
        );
    }

    #[test]
    fn test_parse_iterm2_font_setting_with_spaces() {
        let content = r#"
            (
                {
                    "Normal Font" = "SF Mono 13";
                }
            )
        "#;
        assert_eq!(
            parse_iterm2_font_setting(content, true),
            Some("SF Mono".to_string())
        );
        assert_eq!(
            parse_iterm2_font_setting(content, false),
            Some("13".to_string())
        );
    }

    #[test]
    fn test_parse_iterm2_font_setting_float_size() {
        let content = r#"
            (
                {
                    "Normal Font" = "Menlo 12.5";
                }
            )
        "#;
        assert_eq!(
            parse_iterm2_font_setting(content, true),
            Some("Menlo".to_string())
        );
        // Size parsing handles floats
        assert_eq!(
            parse_iterm2_font_setting(content, false),
            Some("12.5".to_string())
        );
    }

    #[test]
    fn test_parse_iterm2_font_setting_no_match() {
        let content = r#"
            (
                {
                    "Other Setting" = "value";
                }
            )
        "#;
        assert_eq!(parse_iterm2_font_setting(content, true), None);
        assert_eq!(parse_iterm2_font_setting(content, false), None);
    }
}
