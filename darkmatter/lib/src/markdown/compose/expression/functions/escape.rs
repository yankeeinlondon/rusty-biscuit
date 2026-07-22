//! Markdown projection for provider-supplied strings.
//!
//! Provider titles, branch names, actors, statuses, and links are
//! attacker-adjacent: a repository owner controls them and Darkmatter splices
//! them straight into composed Markdown. Two boundaries divide that work by
//! the syntax position the value lands in — [`collapse_and_escape`] for inline
//! content CommonMark must render verbatim, and [`markdown_destination`] for a
//! link destination.

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
///   `(`, `)`, `"`, `'`. Nothing here is ever emitted as a destination
///   ([`markdown_destination`] owns that position), and every `[` and `]` is
///   escaped, so no destination context can form around them.
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

/// Bytes that must not appear literally in an inline link destination.
///
/// CommonMark accepts a bare destination only as a run of characters with no
/// ASCII whitespace, no ASCII control, and balanced parentheses; `<`, `>`, and
/// `\` are added so no autolink, raw-HTML span, or escape can begin inside one.
/// Everything above the printable-ASCII range is encoded too, so the emitted
/// destination is pure ASCII regardless of what a producer hands over.
fn destination_hostile(byte: u8) -> bool {
    byte <= b' ' || byte >= 0x7F || matches!(byte, b'(' | b')' | b'<' | b'>' | b'\\')
}

/// Turns a provider-supplied URL into a Markdown link destination, or drops it.
///
/// ## Returns
///
/// A destination safe to interpolate into `[label](…)` verbatim, or `None` when
/// the value is not an absolute `http(s)` URL — a non-web scheme degrades to
/// the link-less projection instead of emitting a `javascript:` or `data:`
/// destination that a downstream renderer would honor.
///
/// ## Notes
///
/// Sniff already refuses hostile links at the provider boundary
/// (`remote::web_link`); this is the second half of that pair and the one that
/// owns Markdown syntax, so the formatters cannot emit a structurally broken
/// destination even if a future producer skips the first.
///
/// Residual hostile bytes are percent-encoded rather than wrapped in an
/// angle-bracket `<…>` destination. Percent-encoding is transparent under
/// RFC 3986 — the origin server receives the same bytes — and keeps the
/// specified compact `[label](url)` shape. The `<…>` form would still need
/// backslash escaping for `<`, `>`, and `\` inside it, so it adds syntax
/// without removing the escaping problem.
pub(super) fn markdown_destination(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url.trim()).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    let serialized = parsed.to_string();
    let mut output = String::with_capacity(serialized.len());
    for byte in serialized.bytes() {
        if destination_hostile(byte) {
            output.push_str(&format!("%{byte:02X}"));
        } else {
            output.push(char::from(byte));
        }
    }
    Some(output)
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

    /// Destinations a downstream renderer would execute, read from disk, or
    /// resolve against the wrong base.
    const HOSTILE_DESTINATIONS: &[&str] = &[
        "javascript:alert(1)",
        "JavaScript:alert(1)",
        "data:text/html;base64,PHNjcmlwdD4=",
        "file:///etc/passwd",
        "vbscript:msgbox(1)",
        "mailto:alice@evil.example",
        "//evil.example/x",
        "/acme/project/pull/1",
        "pull/1",
        "not a url",
        "",
        "   ",
    ];

    #[test]
    fn only_absolute_web_urls_become_destinations() {
        for raw in HOSTILE_DESTINATIONS {
            assert_eq!(markdown_destination(raw), None, "accepted {raw:?}");
        }
        assert!(markdown_destination("https://provider.example/pr/1").is_some());
        assert!(markdown_destination("http://provider.example:8443/pr/1").is_some());
    }

    #[test]
    fn destination_hostile_bytes_are_percent_encoded() {
        assert_eq!(
            markdown_destination("https://provider.example/pr/1?t=a (b) c").as_deref(),
            Some("https://provider.example/pr/1?t=a%20%28b%29%20c")
        );
        assert_eq!(
            markdown_destination("https://provider.example/a\tb\nc").as_deref(),
            Some("https://provider.example/abc"),
            "the URL parser strips tabs and newlines before this ever sees them"
        );
    }

    /// Percent-encoding is transparent: an origin server decodes the escapes
    /// back to the bytes the provider published, so nothing is retargeted.
    ///
    /// Asserted by decoding rather than by `Url` equality, because the URL
    /// parser preserves existing escapes verbatim — `%28` and `(` are two
    /// distinct `Url` values for the same request.
    #[test]
    fn encoding_preserves_the_target() {
        let raw = "https://provider.example/a(b)/pr/1?t=c(d)e";
        let decoded = markdown_destination(raw)
            .unwrap()
            .replace("%28", "(")
            .replace("%29", ")");
        assert_eq!(decoded, url::Url::parse(raw).unwrap().to_string());
    }

    proptest! {
        /// Whatever a producer supplies, an accepted destination is printable
        /// ASCII with no space, no paren, and no autolink or escape opener —
        /// which is exactly CommonMark's bare-destination grammar, so
        /// `[label](dest)` can only ever parse as one link.
        #[test]
        fn accepted_destinations_are_always_inert(
            tail in "[\\x00-\\x7f]{0,32}"
        ) {
            let raw = format!("https://provider.example/{tail}");
            if let Some(destination) = markdown_destination(&raw) {
                prop_assert!(
                    destination.bytes().all(|byte| !destination_hostile(byte)),
                    "{destination:?}"
                );
                let markdown = format!("[label]({destination})");
                let (parsed, text) = super::harness::parse_literal(&markdown);
                prop_assert_eq!(parsed.as_deref(), Some(destination.as_str()));
                prop_assert_eq!(text, "label");
            }
        }
    }
}
