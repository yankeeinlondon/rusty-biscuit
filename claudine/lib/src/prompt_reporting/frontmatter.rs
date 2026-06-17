//! Frontmatter parsing for prompt reporting configuration.

use darkmatter::markdown::Markdown;

use super::types::ReportMode;

/// Parse the `verbosity` property from a `system-prompt.md` file's
/// frontmatter as a [`ReportMode`]. Valid values are `silent`, `quiet`, and
/// `verbose` (case-insensitive), which map to [`ReportMode::Silent`],
/// [`ReportMode::Summary`], and [`ReportMode::Full`] respectively. Any other
/// value returns `None`.
pub fn parse_frontmatter_verbosity(raw_text: &str) -> Option<ReportMode> {
    let md: Markdown = raw_text.into();
    let value: Option<String> = md.fm_get("verbosity").ok().flatten();
    value.as_deref().and_then(ReportMode::parse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_verbose_lowercase() {
        let text = "---\nverbosity: verbose\n---\n\n# Prompt\n";
        assert_eq!(parse_frontmatter_verbosity(text), Some(ReportMode::Full));
    }

    #[test]
    fn parses_quiet_mixed_case() {
        let text = "---\nverbosity: Quiet\n---\n\n# Prompt\n";
        assert_eq!(parse_frontmatter_verbosity(text), Some(ReportMode::Summary));
    }

    #[test]
    fn parses_silent_uppercase() {
        let text = "---\nverbosity: SILENT\n---\n\n# Prompt\n";
        assert_eq!(parse_frontmatter_verbosity(text), Some(ReportMode::Silent));
    }

    #[test]
    fn returns_none_when_missing() {
        let text = "# No frontmatter\n\nSome content.\n";
        assert_eq!(parse_frontmatter_verbosity(text), None);
    }

    #[test]
    fn returns_none_for_unrecognized_value() {
        let text = "---\nverbosity: chatty\n---\n\n# Prompt\n";
        assert_eq!(parse_frontmatter_verbosity(text), None);
    }

    #[test]
    fn returns_none_for_non_string_value() {
        let text = "---\nverbosity: 123\n---\n\n# Prompt\n";
        assert_eq!(parse_frontmatter_verbosity(text), None);
    }
}
