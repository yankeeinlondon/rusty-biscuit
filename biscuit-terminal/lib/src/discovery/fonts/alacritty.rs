//! Alacritty TOML config parser.

use super::parser::TerminalFontParser;

pub struct AlacrittyParser;

impl TerminalFontParser for AlacrittyParser {
    fn parse_font_name(content: &str) -> Option<String> {
        parse_alacritty_font_name(content)
    }

    fn parse_font_size(content: &str) -> Option<u32> {
        parse_alacritty_font_size(content)
    }
}

/// Parse Alacritty TOML config for font name.
///
/// Looks for:
/// ```toml
/// [font.normal]
/// family = "JetBrains Mono"
/// ```
pub(super) fn parse_alacritty_font_name(content: &str) -> Option<String> {
    let mut in_font_normal = false;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with('#') {
            continue;
        }

        // Track sections - we only care about [font.normal]
        if line.starts_with('[') {
            in_font_normal = line == "[font.normal]";
            continue;
        }

        // Look for family = "Name" in font.normal section
        if in_font_normal
            && line.starts_with("family")
            && let Some(eq_pos) = line.find('=')
        {
            let value = line[eq_pos + 1..].trim();
            // Remove quotes
            let value = value.trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Parse Alacritty TOML config for font size.
///
/// Looks for:
/// ```toml
/// [font]
/// size = 12
/// ```
pub(super) fn parse_alacritty_font_size(content: &str) -> Option<u32> {
    let mut in_font_section = false;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments
        if line.starts_with('#') {
            continue;
        }

        // Track sections - we want [font] but not [font.normal] etc.
        if line.starts_with('[') {
            // [font] section but not subsections
            in_font_section = line == "[font]";
            continue;
        }

        // Look for size = N in [font] section
        if in_font_section
            && line.starts_with("size")
            && let Some(eq_pos) = line.find('=')
        {
            let value = line[eq_pos + 1..].trim();
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
    fn test_parse_alacritty_font_name() {
        let config = r#"
            [font.normal]
            family = "JetBrains Mono"
            style = "Regular"
        "#;
        assert_eq!(
            parse_alacritty_font_name(config),
            Some("JetBrains Mono".to_string())
        );
    }

    #[test]
    fn test_parse_alacritty_font_name_single_quotes() {
        let config = r#"
            [font.normal]
            family = 'Fira Code'
        "#;
        assert_eq!(
            parse_alacritty_font_name(config),
            Some("Fira Code".to_string())
        );
    }

    #[test]
    fn test_parse_alacritty_font_name_ignores_other_sections() {
        let config = r#"
            [font.bold]
            family = "Bold Font"

            [font.normal]
            family = "Normal Font"
        "#;
        assert_eq!(
            parse_alacritty_font_name(config),
            Some("Normal Font".to_string())
        );
    }

    #[test]
    fn test_parse_alacritty_font_size() {
        let config = r#"
            [font]
            size = 12
        "#;
        assert_eq!(parse_alacritty_font_size(config), Some(12));
    }

    #[test]
    fn test_parse_alacritty_font_size_float() {
        let config = r#"
            [font]
            size = 11.5
        "#;
        assert_eq!(parse_alacritty_font_size(config), Some(11));
    }

    #[test]
    fn test_parse_alacritty_font_size_not_in_subsection() {
        let config = r#"
            [font.normal]
            size = 99

            [font]
            size = 12
        "#;
        // Should only find the one in [font], not [font.normal]
        assert_eq!(parse_alacritty_font_size(config), Some(12));
    }
}
