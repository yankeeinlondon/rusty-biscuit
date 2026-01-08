//! Markdown scope definitions for content isolation.
//!
//! This module defines the [`MarkdownScope`] enum, which specifies which
//! structural elements to extract from a markdown document.

/// Specifies which markdown elements to isolate.
///
/// Each variant targets a specific structural element or content type
/// within a markdown document. The isolation process extracts matching
/// content while preserving the original document structure.
///
/// ## Examples
///
/// ```
/// use shared::isolate::MarkdownScope;
///
/// // Extract all code blocks
/// let scope = MarkdownScope::CodeBlock;
///
/// // Extract only italic text
/// let scope = MarkdownScope::Italicized;
///
/// // Extract link text and URLs
/// let scope = MarkdownScope::Links;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkdownScope {
    /// YAML frontmatter content between the first `---` markers.
    ///
    /// Returns an empty result if no frontmatter is present. The delimiters
    /// themselves are not included in the output.
    Frontmatter,

    /// Prose text outside of code blocks, block quotes, and special elements.
    ///
    /// This captures the main body text of a document, excluding:
    /// - Code blocks (fenced and indented)
    /// - Block quotes
    /// - Headings
    /// - List markers (but list item prose is included)
    /// - Table structure (but cell content may be included)
    Prose,

    /// Fenced or indented code blocks.
    ///
    /// Includes the code content and preserves the info string (language hint)
    /// when available. The fence markers themselves are not included.
    CodeBlock,

    /// Block quote content.
    ///
    /// Extracts the text within block quotes, excluding the `>` markers.
    /// Nested block quotes are flattened into their content.
    BlockQuote,

    /// Heading text content.
    ///
    /// Extracts heading text at all levels, excluding the `#` markers
    /// and any trailing formatting. Both ATX (`#`) and setext (underline)
    /// style headings are captured.
    Heading,

    /// Text with any styling: bold, italic, or strikethrough.
    ///
    /// Captures content wrapped in:
    /// - `**bold**` or `__bold__`
    /// - `*italic*` or `_italic_`
    /// - `~~strikethrough~~`
    ///
    /// The markers are stripped; only the inner text is returned.
    Stylized,

    /// Text styled with italic formatting only.
    ///
    /// Captures content wrapped in `*text*` or `_text_`.
    /// Does not include bold or strikethrough content.
    Italicized,

    /// All content except italic text.
    ///
    /// This is the complement of [`MarkdownScope::Italicized`], returning
    /// everything in the document except content within italic markers.
    NonItalicized,

    /// Link text and destination URLs.
    ///
    /// Captures both inline links `[text](url)` and reference links.
    /// Returns the link text, destination URL, and optional title.
    Links,

    /// Image alt text and source URLs.
    ///
    /// Captures image elements `![alt](src)`, returning the alt text,
    /// source URL, and optional title.
    Images,

    /// Content within list items (ordered and unordered).
    ///
    /// Extracts the text content of list items, excluding the list markers
    /// (`-`, `*`, `+`, or numbers). Nested lists are included.
    Lists,

    /// Table headers and cell content (GFM extension).
    ///
    /// Captures content from GitHub Flavored Markdown tables, including
    /// both header cells and body cells. The pipe delimiters and
    /// alignment indicators are excluded.
    Tables,

    /// Footnote definition content (GFM extension).
    ///
    /// Extracts the content of footnote definitions `[^marker]: content`,
    /// excluding the marker itself. Only the definition content is returned,
    /// not footnote references in the text.
    FootnoteDefinitions,
}

impl MarkdownScope {
    /// Returns all available scope variants.
    ///
    /// Useful for iterating over all possible isolation targets.
    pub fn all() -> &'static [MarkdownScope] {
        &[
            MarkdownScope::Frontmatter,
            MarkdownScope::Prose,
            MarkdownScope::CodeBlock,
            MarkdownScope::BlockQuote,
            MarkdownScope::Heading,
            MarkdownScope::Stylized,
            MarkdownScope::Italicized,
            MarkdownScope::NonItalicized,
            MarkdownScope::Links,
            MarkdownScope::Images,
            MarkdownScope::Lists,
            MarkdownScope::Tables,
            MarkdownScope::FootnoteDefinitions,
        ]
    }

    /// Returns a human-readable name for the scope.
    pub fn name(&self) -> &'static str {
        match self {
            MarkdownScope::Frontmatter => "Frontmatter",
            MarkdownScope::Prose => "Prose",
            MarkdownScope::CodeBlock => "Code Block",
            MarkdownScope::BlockQuote => "Block Quote",
            MarkdownScope::Heading => "Heading",
            MarkdownScope::Stylized => "Stylized",
            MarkdownScope::Italicized => "Italicized",
            MarkdownScope::NonItalicized => "Non-Italicized",
            MarkdownScope::Links => "Links",
            MarkdownScope::Images => "Images",
            MarkdownScope::Lists => "Lists",
            MarkdownScope::Tables => "Tables",
            MarkdownScope::FootnoteDefinitions => "Footnote Definitions",
        }
    }

    /// Returns a brief description of what the scope isolates.
    pub fn description(&self) -> &'static str {
        match self {
            MarkdownScope::Frontmatter => "YAML content between --- markers",
            MarkdownScope::Prose => "Body text outside special elements",
            MarkdownScope::CodeBlock => "Fenced or indented code blocks",
            MarkdownScope::BlockQuote => "Block quote content (excluding > markers)",
            MarkdownScope::Heading => "Heading text (excluding # markers)",
            MarkdownScope::Stylized => "Bold, italic, and strikethrough content",
            MarkdownScope::Italicized => "Italic content only (*text* or _text_)",
            MarkdownScope::NonItalicized => "Everything except italic content",
            MarkdownScope::Links => "Link text and destination URLs",
            MarkdownScope::Images => "Image alt text and source URLs",
            MarkdownScope::Lists => "List item content (ordered and unordered)",
            MarkdownScope::Tables => "Table headers and cell content (GFM)",
            MarkdownScope::FootnoteDefinitions => "Footnote content (excluding [^marker])",
        }
    }
}

impl std::fmt::Display for MarkdownScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_variants_count() {
        // Ensure we have exactly 13 variants
        assert_eq!(MarkdownScope::all().len(), 13);
    }

    #[test]
    fn test_all_variants_unique() {
        let all = MarkdownScope::all();
        for (i, scope) in all.iter().enumerate() {
            for (j, other) in all.iter().enumerate() {
                if i != j {
                    assert_ne!(scope, other, "Duplicate variant at indices {} and {}", i, j);
                }
            }
        }
    }

    #[test]
    fn test_name_non_empty() {
        for scope in MarkdownScope::all() {
            assert!(!scope.name().is_empty(), "{:?} has empty name", scope);
        }
    }

    #[test]
    fn test_description_non_empty() {
        for scope in MarkdownScope::all() {
            assert!(
                !scope.description().is_empty(),
                "{:?} has empty description",
                scope
            );
        }
    }

    #[test]
    fn test_display_matches_name() {
        for scope in MarkdownScope::all() {
            assert_eq!(scope.to_string(), scope.name());
        }
    }

    #[test]
    fn test_copy_clone() {
        let scope = MarkdownScope::CodeBlock;
        let copied = scope;
        let cloned = scope.clone();

        assert_eq!(scope, copied);
        assert_eq!(scope, cloned);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        for scope in MarkdownScope::all() {
            assert!(set.insert(*scope), "Duplicate hash for {:?}", scope);
        }
        assert_eq!(set.len(), 13);
    }
}
