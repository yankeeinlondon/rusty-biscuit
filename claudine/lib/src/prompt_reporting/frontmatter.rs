//! Frontmatter parsing for prompt reporting configuration.

use darkmatter::markdown::Markdown;

use super::types::PromptVerbosity;

/// Parse the `verbosity` property from the frontmatter of a `system-prompt.md`
/// file.
///
/// The raw markdown text is parsed to extract the YAML frontmatter, and the
/// `verbosity` key is read as a string.  Valid values are `silent`, `quiet`,
/// and `verbose` (case-insensitive).
///
/// ## Returns
///
/// Returns `Some(PromptVerbosity)` when the frontmatter contains a valid
/// `verbosity` value, or `None` when the key is missing or the value is
/// unrecognized.
///
/// ## Examples
///
/// ```
/// use claudine::prompt_reporting::{parse_frontmatter_verbosity, PromptVerbosity};
///
/// let text = "---\nverbosity: verbose\n---\n\n# System Prompt\n";
/// assert_eq!(
///     parse_frontmatter_verbosity(text),
///     Some(PromptVerbosity::Verbose)
/// );
///
/// let no_verbosity = "# No frontmatter\n";
/// assert_eq!(parse_frontmatter_verbosity(no_verbosity), None);
/// ```
pub fn parse_frontmatter_verbosity(raw_text: &str) -> Option<PromptVerbosity> {
    let md: Markdown = raw_text.into();
    let value: Option<String> = md.fm_get("verbosity").ok().flatten();
    value.as_deref().and_then(PromptVerbosity::parse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_verbose_lowercase() {
        let text = "---\nverbosity: verbose\n---\n\n# Prompt\n";
        assert_eq!(
            parse_frontmatter_verbosity(text),
            Some(PromptVerbosity::Verbose)
        );
    }

    #[test]
    fn parses_quiet_mixed_case() {
        let text = "---\nverbosity: Quiet\n---\n\n# Prompt\n";
        assert_eq!(
            parse_frontmatter_verbosity(text),
            Some(PromptVerbosity::Quiet)
        );
    }

    #[test]
    fn parses_silent_uppercase() {
        let text = "---\nverbosity: SILENT\n---\n\n# Prompt\n";
        assert_eq!(
            parse_frontmatter_verbosity(text),
            Some(PromptVerbosity::Silent)
        );
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
