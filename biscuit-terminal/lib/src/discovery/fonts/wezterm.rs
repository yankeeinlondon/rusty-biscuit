//! Wezterm Lua config parser.

use super::parser::TerminalFontParser;

pub struct WeztermParser;

impl TerminalFontParser for WeztermParser {
    fn parse_font_name(content: &str) -> Option<String> {
        parse_wezterm_font_name(content)
    }

    fn parse_font_size(content: &str) -> Option<u32> {
        parse_wezterm_font_size(content)
    }
}

/// Parse Wezterm Lua config for font name.
///
/// Looks for patterns like:
/// - `config.font = wezterm.font("JetBrains Mono")`
/// - `config.font = wezterm.font("JetBrains Mono", { weight = "Bold" })`
/// - `config.font = wezterm.font { family = "JetBrains Mono" }`
pub(super) fn parse_wezterm_font_name(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with("--") {
            continue;
        }

        // Look for config.font = wezterm.font(
        if line.contains("config.font") && line.contains("wezterm.font") {
            // Try to extract font name from wezterm.font("Name" or wezterm.font("Name",
            if let Some(start) = line.find("wezterm.font(\"") {
                let after_quote = &line[start + 14..]; // Skip 'wezterm.font("'
                if let Some(end) = after_quote.find('"') {
                    let font_name = &after_quote[..end];
                    if !font_name.is_empty() {
                        return Some(font_name.to_string());
                    }
                }
            }

            // Try alternate pattern: wezterm.font { family = "Name"
            if let Some(start) = line.find("family") {
                let after_family = &line[start..];
                // Find the quoted value after family =
                if let Some(quote_start) = after_family.find('"') {
                    let after_quote = &after_family[quote_start + 1..];
                    if let Some(quote_end) = after_quote.find('"') {
                        let font_name = &after_quote[..quote_end];
                        if !font_name.is_empty() {
                            return Some(font_name.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Parse Wezterm Lua config for font size.
///
/// Looks for: `config.font_size = 13`
pub(super) fn parse_wezterm_font_size(content: &str) -> Option<u32> {
    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with("--") {
            continue;
        }

        // Look for config.font_size = N
        if line.contains("config.font_size")
            && let Some(eq_pos) = line.find('=')
        {
            let value_part = line[eq_pos + 1..].trim();
            // Parse as float first (Lua allows 13.0), then convert to u32
            if let Ok(size) = value_part.parse::<f64>() {
                return Some(size as u32);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wezterm_font_name_basic() {
        let config = r#"
            local wezterm = require("wezterm")
            config.font = wezterm.font("JetBrains Mono")
        "#;
        assert_eq!(
            parse_wezterm_font_name(config),
            Some("JetBrains Mono".to_string())
        );
    }

    #[test]
    fn test_parse_wezterm_font_name_with_options() {
        let config = r#"
            config.font = wezterm.font("Fira Code", { weight = "Bold" })
        "#;
        assert_eq!(
            parse_wezterm_font_name(config),
            Some("Fira Code".to_string())
        );
    }

    #[test]
    fn test_parse_wezterm_font_name_ignores_comments() {
        let config = r#"
            -- config.font = wezterm.font("Commented Out")
            config.font = wezterm.font("Actual Font")
        "#;
        assert_eq!(
            parse_wezterm_font_name(config),
            Some("Actual Font".to_string())
        );
    }

    #[test]
    fn test_parse_wezterm_font_name_no_match() {
        let config = r#"
            config.color_scheme = "Dracula"
        "#;
        assert_eq!(parse_wezterm_font_name(config), None);
    }

    #[test]
    fn test_parse_wezterm_font_size_integer() {
        let config = r#"
            config.font_size = 13
        "#;
        assert_eq!(parse_wezterm_font_size(config), Some(13));
    }

    #[test]
    fn test_parse_wezterm_font_size_float() {
        let config = r#"
            config.font_size = 14.5
        "#;
        assert_eq!(parse_wezterm_font_size(config), Some(14));
    }

    #[test]
    fn test_parse_wezterm_font_size_ignores_comments() {
        let config = r#"
            -- config.font_size = 99
            config.font_size = 12
        "#;
        assert_eq!(parse_wezterm_font_size(config), Some(12));
    }
}
