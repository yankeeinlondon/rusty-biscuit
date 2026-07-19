//! Literal-text projection for provider-supplied strings embedded in Markdown.
//!
//! Provider titles, branch names, actors, and statuses are attacker-adjacent
//! text: a repository owner controls them and Darkmatter splices them straight
//! into composed Markdown. `collapse_and_escape` is the single boundary that
//! turns such a string into inline content that CommonMark renders verbatim.

/// Characters that can begin, end, or alter an inline construct at *any*
/// column, across every `pulldown_cmark::Options` flag Darkmatter enables
/// anywhere (`markdown::cleanup` parses with `Options::all()` minus smart
/// punctuation and definition lists, which is the widest set in the crate).
///
/// `$` is math, `{`/`}` are heading attributes, `^` is superscript and `~` is
/// strikethrough/subscript: all inert under the render-tree option set but live
/// under cleanup's, so they are escaped unconditionally rather than made to
/// depend on which parser sees the output.
const INLINE_ACTIVE: [char; 16] =
    ['\\', '`', '*', '_', '~', '^', '[', ']', '<', '>', '&', '|', '$', '{', '}', '!'];

/// Collapses every run of whitespace to one space and backslash-escapes the
/// ASCII punctuation that would otherwise be parsed as Markdown, so the result
/// renders as the literal input both as running text and as a `[label](url)`
/// link label.
///
/// ## Notes
///
/// Only [`INLINE_ACTIVE`] is escaped. Two families are deliberately left alone:
///
/// - **Block starters** — `#`, `-`, `+`, `=`, `:`, and `1.`/`1)` markers only
///   open a block at the start of a line. The result here is a single collapsed
///   line that every caller embeds after a literal prefix (`[`, `PR `,
///   `CI job `, or a ` · ` separator), so no escaped value can ever sit at
///   column zero. Escaping them anyway would put a backslash before every `.`
///   and `-` in ordinary prose for no rendered difference, and the projection
///   is specified as compact, noise-free Markdown.
/// - **Delimiters that are only live inside a link destination or title** —
///   `(`, `)`, `"`, `'`. Nothing here is ever emitted as a destination (URLs are
///   passed through untouched by design), and every `[` and `]` is escaped, so
///   no destination context can form around them.
///
/// CommonMark honors a backslash only before ASCII punctuation — before
/// anything else the backslash is itself literal — so every character in
/// [`INLINE_ACTIVE`] is ASCII punctuation and no non-punctuation character is
/// ever prefixed.
pub(super) fn collapse_and_escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut first = true;
    for word in value.split_whitespace() {
        if !first {
            output.push(' ');
        }
        first = false;
        for character in word.chars() {
            if INLINE_ACTIVE.contains(&character) {
                output.push('\\');
            }
            output.push(character);
        }
    }
    output
}

#[cfg(test)]
pub(in crate::markdown::compose) mod harness {
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

    /// The widest option set any Darkmatter parse uses, minus smart
    /// punctuation (never enabled in the crate, and it would rewrite quotes
    /// and dashes that escaping cannot defend against).
    fn strictest_options() -> Options {
        Options::all() - Options::ENABLE_SMART_PUNCTUATION
    }

    /// Parses one paragraph of Markdown, returning the destination of the single
    /// permitted link (if any) and the concatenation of all inline text.
    ///
    /// Panics on any other event. That panic is the discriminating half of every
    /// assertion built on this: a surviving code span or emphasis contributes the
    /// same characters as literal text, so comparing text alone would pass.
    pub fn parse_literal(markdown: &str) -> (Option<String>, String) {
        let mut destination = None;
        let mut text = String::new();
        for event in Parser::new_ext(markdown, strictest_options()) {
            match event {
                Event::Start(Tag::Paragraph)
                | Event::End(TagEnd::Paragraph)
                | Event::End(TagEnd::Link) => {}
                Event::Start(Tag::Link { dest_url, .. }) => {
                    assert!(destination.is_none(), "{markdown:?} produced more than one link");
                    destination = Some(dest_url.to_string());
                }
                Event::Text(chunk) => text.push_str(&chunk),
                other => panic!("{markdown:?} produced non-literal event {other:?}"),
            }
        }
        (destination, text)
    }

    /// Asserts `escaped` renders as `expected` when embedded in running text.
    pub fn renders_literally(escaped: &str, expected: &str) {
        let markdown = format!("before {escaped} after");
        let (destination, text) = parse_literal(&markdown);
        assert_eq!(destination, None, "{markdown:?} grew a link");
        assert_eq!(text, format!("before {expected} after"));
    }

    /// Asserts `escaped` survives the stricter link-label context: the link
    /// still spans the whole label and points at `url`.
    pub fn renders_literally_as_link_label(escaped: &str, expected: &str, url: &str) {
        let markdown = format!("[{escaped}]({url})");
        let (destination, label) = parse_literal(&markdown);
        assert_eq!(destination.as_deref(), Some(url), "link destination changed for {markdown:?}");
        assert_eq!(label, expected, "link label changed for {markdown:?}");
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::harness::{renders_literally, renders_literally_as_link_label};
    use super::*;

    fn collapsed(value: &str) -> String {
        value.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// One case per punctuation class that can alter inline parsing.
    const HOSTILE: &[&str] = &[
        "**urgent**",
        "`code`",
        "_name_",
        "~~struck~~",
        "^sup^",
        "<script>alert(1)</script>",
        "<img src=x onerror=alert(1)>",
        "&amp; &#65;",
        "a | b | c",
        "$x^2$",
        "# heading {#id .cls}",
        "![img](https://evil.example/x.png)",
        "[label](https://evil.example)",
        "unbalanced [ open",
        "unbalanced ] close",
        "trailing backslash \\",
        r"already \[escaped\] text",
        r"\*not bold\*",
        "1. list",
        "- bullet",
        "  indented code",
        "auto <https://evil.example>",
        "footnote [^1]",
    ];

    #[test]
    fn every_punctuation_class_renders_as_literal_text() {
        for raw in HOSTILE {
            renders_literally(&collapse_and_escape(raw), &collapsed(raw));
        }
    }

    #[test]
    fn every_punctuation_class_survives_a_link_label() {
        for raw in HOSTILE {
            renders_literally_as_link_label(
                &collapse_and_escape(raw),
                &collapsed(raw),
                "https://provider.example/pr/1",
            );
        }
    }

    /// Escaping is idempotent in *rendered* terms: text that already carries
    /// backslashes must come back with those backslashes intact rather than
    /// being consumed as escapes or doubled into visible noise.
    #[test]
    fn already_escaped_text_is_not_double_mangled() {
        let raw = r"already \[escaped\]";
        let once = collapse_and_escape(raw);
        renders_literally(&once, raw);
        renders_literally(&collapse_and_escape(&once), &once);
    }

    #[test]
    fn whitespace_collapses_to_single_spaces() {
        assert_eq!(collapse_and_escape("  Fix \t the \n\n parser  "), "Fix the parser");
        assert_eq!(collapse_and_escape("   "), "");
    }

    /// Sentence punctuation carries no backslash: over-escaping would be the
    /// placeholder noise the projection rules forbid.
    #[test]
    fn prose_punctuation_is_left_unescaped() {
        assert_eq!(
            collapse_and_escape("Fix v1.2 (again): don't, won't; why? 50% @ #4 - end"),
            "Fix v1.2 (again): don't, won't; why? 50% @ #4 - end"
        );
    }

    proptest! {
        /// The property the hand-written table only samples: for arbitrary
        /// printable-ASCII provider text, the escaped form parses back to the
        /// collapsed original with no Markdown structure at all.
        #[test]
        fn arbitrary_provider_text_round_trips(raw in "[ -~\t\n]{0,48}") {
            let escaped = collapse_and_escape(&raw);
            renders_literally(&escaped, &collapsed(&raw));
            renders_literally_as_link_label(&escaped, &collapsed(&raw), "https://provider.example/x");
        }
    }
}
