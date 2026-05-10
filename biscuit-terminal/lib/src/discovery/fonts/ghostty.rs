//! Ghostty config parser (key=value format).

use super::parser::TerminalFontParser;

pub struct GhosttyParser;

impl TerminalFontParser for GhosttyParser {
    fn parse_font_name(content: &str) -> Option<String> {
        parse_ghostty_font_name(content)
    }

    fn parse_font_size(content: &str) -> Option<u32> {
        parse_ghostty_font_size(content)
    }
}

/// Parse Ghostty config for font name.
///
/// First tries the config file, then falls back to `ghostty +show-config`
/// to get actual running values including defaults.
pub(super) fn parse_ghostty_font_name(content: &str) -> Option<String> {
    // First try parsing the config file
    if let Some(name) = parse_ghostty_config_value(content, "font-family") {
        return Some(name);
    }

    // Fall back to querying Ghostty for its actual config (includes defaults)
    query_ghostty_config("font-family")
}

/// Parse Ghostty config for font size.
///
/// First tries the config file, then falls back to `ghostty +show-config`
/// to get actual running values including defaults.
pub(super) fn parse_ghostty_font_size(content: &str) -> Option<u32> {
    // First try parsing the config file
    if let Some(value) = parse_ghostty_config_value(content, "font-size")
        && let Ok(size) = value.parse::<f64>()
    {
        return Some(size as u32);
    }

    // Fall back to querying Ghostty for its actual config
    if let Some(value) = query_ghostty_config("font-size")
        && let Ok(size) = value.parse::<f64>()
    {
        return Some(size as u32);
    }

    None
}

/// Parse a key-value pair from Ghostty config content.
fn parse_ghostty_config_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with('#') {
            continue;
        }

        // Look for key = value
        if line.starts_with(key)
            && let Some(eq_pos) = line.find('=')
        {
            let value = line[eq_pos + 1..].trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Query Ghostty for a config value using `ghostty +show-config`.
///
/// This returns actual running values including defaults, not just
/// what's in the config file.
fn query_ghostty_config(key: &str) -> Option<String> {
    use std::process::Command;

    let output = Command::new("ghostty")
        .args(["+show-config"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with(key)
            && let Some(eq_pos) = line.find('=')
        {
            let value = line[eq_pos + 1..].trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ghostty_font_name() {
        let config = r#"
            # Ghostty config
            font-family = Iosevka Term
        "#;
        assert_eq!(
            parse_ghostty_font_name(config),
            Some("Iosevka Term".to_string())
        );
    }

    #[test]
    fn test_parse_ghostty_font_name_ignores_comments() {
        let config = r#"
            # font-family = Commented
            font-family = Active
        "#;
        assert_eq!(parse_ghostty_font_name(config), Some("Active".to_string()));
    }

    #[test]
    fn test_parse_ghostty_font_size() {
        let config = r#"
            font-size = 14
        "#;
        assert_eq!(parse_ghostty_font_size(config), Some(14));
    }

    #[test]
    fn test_parse_ghostty_font_size_float() {
        let config = r#"
            font-size = 13.5
        "#;
        assert_eq!(parse_ghostty_font_size(config), Some(13));
    }
}
