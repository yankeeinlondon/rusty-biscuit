//! Byte-preserving line indentation shared across compose stages.
//!
//! Both `::toc-linking` and the `::shell` / `::shell-block` directives need to
//! re-indent a multi-line replacement string so that generated lines stay
//! nested under the container the directive appeared in. This module holds the
//! single implementation those stages share.

/// Prefixes every line of `text` with an effective indent.
///
/// The effective indent is `indent` when it is non-empty, otherwise
/// `inferred_indent` (its fallback). When the effective indent or `text` is
/// empty, `text` is returned unchanged.
///
/// The prefix is inserted at the start of the text and after every interior
/// newline. A trailing newline does not gain a prefix because no content line
/// follows it, so final newlines, trailing whitespace, tabs, and blank interior
/// lines are all preserved byte-for-byte.
///
/// ## Examples
///
/// ```ignore
/// assert_eq!(indent_text("a\nb", "  ", None), "  a\n  b");
/// assert_eq!(indent_text("a\n", "  ", None), "  a\n");
/// assert_eq!(indent_text("a", "", Some("\t")), "\ta");
/// assert_eq!(indent_text("", "  ", None), "");
/// ```
pub(crate) fn indent_text(text: &str, indent: &str, inferred_indent: Option<&str>) -> String {
    let effective_indent = if indent.is_empty() {
        inferred_indent.unwrap_or("")
    } else {
        indent
    };

    if effective_indent.is_empty() || text.is_empty() {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len() + effective_indent.len());
    out.push_str(effective_indent);

    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        out.push(ch);
        // Only re-indent when another character follows the newline, so a
        // trailing newline is preserved without spawning an indent-only line.
        if ch == '\n' && chars.peek().is_some() {
            out.push_str(effective_indent);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_line_text_is_prefixed_per_line() {
        assert_eq!(
            indent_text("first\nsecond\nthird", "    ", None),
            "    first\n    second\n    third"
        );
    }

    #[test]
    fn blank_interior_lines_receive_the_indent() {
        // A blank separator line between two output lines must still be nested,
        // otherwise it could break out of the surrounding list/blockquote.
        assert_eq!(indent_text("one\n\ntwo", "  ", None), "  one\n  \n  two");
    }

    #[test]
    fn trailing_newline_is_preserved_without_indent_only_line() {
        assert_eq!(indent_text("one\ntwo\n", "  ", None), "  one\n  two\n");
    }

    #[test]
    fn trailing_whitespace_is_preserved() {
        assert_eq!(indent_text("one  \ntwo\t", "  ", None), "  one  \n  two\t");
    }

    #[test]
    fn tab_indentation_is_applied_verbatim() {
        assert_eq!(indent_text("a\nb", "\t", None), "\ta\n\tb");
    }

    #[test]
    fn single_line_text_is_prefixed() {
        assert_eq!(indent_text("solo", "  ", None), "  solo");
    }

    #[test]
    fn empty_text_returns_empty() {
        assert_eq!(indent_text("", "    ", None), "");
    }

    #[test]
    fn empty_indent_returns_text_unchanged() {
        assert_eq!(indent_text("one\ntwo", "", None), "one\ntwo");
    }

    #[test]
    fn inferred_indent_is_used_when_indent_is_empty() {
        assert_eq!(indent_text("a\nb", "", Some("  ")), "  a\n  b");
    }

    #[test]
    fn explicit_indent_wins_over_inferred() {
        assert_eq!(indent_text("a", "    ", Some("\t")), "    a");
    }

    #[test]
    fn empty_indent_and_empty_inferred_returns_text_unchanged() {
        assert_eq!(indent_text("a\nb", "", Some("")), "a\nb");
    }
}
