//! Kitty config parser (`kitty.conf`).

use super::parser::TerminalFontParser;

pub struct KittyParser;

impl TerminalFontParser for KittyParser {
    fn parse_font_name(content: &str) -> Option<String> {
        parse_kitty_font_name(content)
    }

    fn parse_font_size(content: &str) -> Option<u32> {
        parse_kitty_font_size(content)
    }
}

/// Parse Kitty config for font name.
///
/// Looks for: `font_family FiraCode Nerd Font`
pub(super) fn parse_kitty_font_name(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with('#') {
            continue;
        }

        // Look for font_family <name> (space-separated, not =)
        if let Some(stripped) = line.strip_prefix("font_family") {
            let value = stripped.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Parse Kitty config for font size.
///
/// Looks for: `font_size 14.0`
pub(super) fn parse_kitty_font_size(content: &str) -> Option<u32> {
    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with('#') {
            continue;
        }

        // Look for font_size N (space-separated)
        if let Some(stripped) = line.strip_prefix("font_size") {
            let value = stripped.trim();
            if let Ok(size) = value.parse::<f64>() {
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
    fn test_parse_kitty_font_name() {
        let config = r#"
            # kitty.conf
            font_family FiraCode Nerd Font Mono
        "#;
        assert_eq!(
            parse_kitty_font_name(config),
            Some("FiraCode Nerd Font Mono".to_string())
        );
    }

    #[test]
    fn test_parse_kitty_font_name_ignores_comments() {
        let config = r#"
            #font_family Commented
            font_family Active Font
        "#;
        assert_eq!(
            parse_kitty_font_name(config),
            Some("Active Font".to_string())
        );
    }

    #[test]
    fn test_parse_kitty_font_size() {
        let config = r#"
            font_size 14.0
        "#;
        assert_eq!(parse_kitty_font_size(config), Some(14));
    }

    #[test]
    fn test_parse_kitty_font_size_integer() {
        let config = r#"
            font_size 12
        "#;
        assert_eq!(parse_kitty_font_size(config), Some(12));
    }
}
