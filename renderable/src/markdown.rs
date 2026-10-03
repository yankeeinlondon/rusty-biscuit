/// A component capable of rendering itself as Markdown output.
///
/// Markdown is a _superset_ of HTML which means, strictly speaking, it can
/// contain as much inline HTML as you like but there are good reasons to
/// keep this to a minimum:
///
/// 1. Every HTML tag makes the authoring experience less ergonomic
/// 2. While many renders might render a few inline elements effectively you should expect inconsistent
pub trait MarkdownRenderable {
    /// Renders the component as a Markdown string (including Markdown body and optionally
    /// YAML Frontmatter).
    ///
    /// - any valid Markdown can be passed through "as is"
    /// - some renderers might offer (on their input) an option to "clean" the markdown to make it more idiomatic
    /// - some renderers might decide to provide an option (on their input) to
    fn render_markdown(&self) -> String;

    /// Renders the component as a Markdown string (including Markdown body and optionally
    /// YAML Frontmatter).
    ///
    /// - Features often supported by markdown renders like:
    ///     - iframes (darkmatter will convert )
    ///     - disclosures (darkmatter will convert ergonomic markdown directives into HTML variant)
    ///     - nicer table rendering (by converting Markdown tables to HTML tables)
    ///
    /// > **Note:** the features supported should _never_ rely on Javascript!
    fn render_markdown_plus(&self) -> String;
}

/// Spells `value` as a Markdown code span whose content parses back to
/// `value`, following the [CommonMark code span] rules.
///
/// The fence is a backtick run one longer than the longest run inside
/// `value` (a single backtick when there is none), so embedded backticks
/// never close the span early. One space of padding goes on each side when
/// `value` begins or ends with a backtick, or begins and ends with a space
/// and is not all spaces; CommonMark strips exactly that padding, so edge
/// spaces and edge backticks survive.
///
/// The content is literal: nothing inside is escaped, because CommonMark
/// gives backslashes no meaning inside a code span. A caller placing the span
/// in a GFM table cell escapes `|` in `value` *before* calling this.
///
/// ## Examples
///
/// ```
/// use renderable::markdown::code_span;
///
/// assert_eq!(code_span("plain"), "`plain`");
/// assert_eq!(code_span("a`b"), "``a`b``");
/// assert_eq!(code_span("`a`"), "`` `a` ``");
/// assert_eq!(code_span("one\ntwo"), "`one two`");
/// assert_eq!(code_span(""), "");
/// ```
///
/// ## Notes
///
/// Two values have no faithful code-span spelling, and the helper makes the
/// loss explicit rather than emitting something that parses differently:
///
/// - **Line endings** (`\n`, `\r\n`, or a lone `\r`) become one space each
///   before fencing, which is what CommonMark would turn them into anyway.
/// - **An empty value** returns an empty string with no fence; an empty
///   backtick pair would not parse as a code span.
///
/// [CommonMark code span]: https://spec.commonmark.org/0.31.2/#code-spans
#[must_use]
pub fn code_span(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    let value = value.replace("\r\n", " ").replace(['\r', '\n'], " ");

    let mut longest_run = 0;
    let mut run = 0;
    for c in value.chars() {
        if c == '`' {
            run += 1;
            longest_run = longest_run.max(run);
        } else {
            run = 0;
        }
    }
    let fence = "`".repeat(longest_run + 1);

    let pad = value.starts_with('`')
        || value.ends_with('`')
        || (value.starts_with(' ') && value.ends_with(' ') && value.bytes().any(|b| b != b' '));
    if pad {
        format!("{fence} {value} {fence}")
    } else {
        format!("{fence}{value}{fence}")
    }
}

#[cfg(test)]
mod tests {
    use super::code_span;

    #[test]
    fn code_span_spec_table() {
        assert_eq!(code_span("plain"), "`plain`");
        assert_eq!(code_span("a`b"), "``a`b``");
        assert_eq!(code_span("`a`"), "`` `a` ``");
    }

    #[test]
    fn code_span_fence_is_longer_than_the_longest_backtick_run() {
        assert_eq!(code_span("a``b"), "```a``b```");
        assert_eq!(code_span("a`b``c"), "```a`b``c```");
        assert_eq!(code_span("a`````b"), "``````a`````b``````");
        // A long run at the edge gets both the longer fence and the padding.
        assert_eq!(code_span("``x"), "``` ``x ```");
        assert_eq!(code_span("x``"), "``` x`` ```");
        assert_eq!(code_span("`"), "`` ` ``");
    }

    #[test]
    fn code_span_edge_spaces() {
        // Begins and ends with a space: padded so CommonMark's one-space strip
        // leaves the original spaces.
        assert_eq!(code_span(" a "), "`  a  `");
        assert_eq!(code_span("  a  "), "`   a   `");
        // A space on one edge only is not stripped by CommonMark: no padding.
        assert_eq!(code_span(" a"), "` a`");
        assert_eq!(code_span("a "), "`a `");
        // A space on one edge and a backtick on the other pads for the backtick.
        assert_eq!(code_span(" a`"), "``  a` ``");
    }

    #[test]
    fn code_span_all_spaces_is_not_padded() {
        // CommonMark never strips an all-space span, so padding would add spaces.
        assert_eq!(code_span(" "), "` `");
        assert_eq!(code_span("   "), "`   `");
    }

    #[test]
    fn code_span_line_endings_become_spaces() {
        assert_eq!(code_span("a\nb"), "`a b`");
        assert_eq!(code_span("a\r\nb"), "`a b`");
        assert_eq!(code_span("a\rb"), "`a b`");
        assert_eq!(code_span("a\n\nb"), "`a  b`");
        // Normalization happens before the padding decision.
        assert_eq!(code_span("\na\n"), "`  a  `");
        assert_eq!(code_span("\n"), "` `");
    }

    /// Parses `markdown` with a real CommonMark parser and returns the text of
    /// every code span, plus whether anything other than code spans appeared
    /// inside the paragraph.
    fn parsed_code_spans(markdown: &str) -> (Vec<String>, bool) {
        use pulldown_cmark::{Event, Parser, Tag, TagEnd};
        let mut spans = Vec::new();
        let mut other = false;
        for event in Parser::new(markdown) {
            match event {
                Event::Code(code) => spans.push(code.into_string()),
                Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph) => {}
                _ => other = true,
            }
        }
        (spans, other)
    }

    #[test]
    fn code_span_parses_back_with_commonmark() {
        for value in [
            "plain", "a`b", "`a`", "``x", "x``", "`", " a ", " a", "a ", " a`", " ", "   ",
            r"a\_b", "[desc](ref)", "*not emphasis*", "a``b```c",
        ] {
            let (spans, other) = parsed_code_spans(&code_span(value));
            assert_eq!(spans, vec![value.to_string()], "{value:?} -> {}", code_span(value));
            assert!(!other, "{value:?}");
        }
    }

    proptest::proptest! {
        /// Any non-empty value round-trips through CommonMark as one code span
        /// whose text is the value with each line ending turned into a space.
        #[test]
        fn code_span_round_trips_any_value(value in r"[ a`\\\n\r|*_\[\]()<>&é]{1,12}") {
            let expected = value.replace("\r\n", " ").replace(['\r', '\n'], " ");
            let markdown = code_span(&value);
            let (spans, other) = parsed_code_spans(&markdown);
            proptest::prop_assert_eq!(spans, vec![expected], "markdown: {:?}", markdown);
            proptest::prop_assert!(!other, "markdown: {:?}", markdown);
        }
    }

    #[test]
    fn code_span_empty_has_no_fence() {
        assert_eq!(code_span(""), "");
    }

    #[test]
    fn code_span_content_is_literal() {
        assert_eq!(code_span(r"a\_b"), r"`a\_b`");
        assert_eq!(code_span("[desc](ref)"), "`[desc](ref)`");
        assert_eq!(code_span("<b>&amp;"), "`<b>&amp;`");
        assert_eq!(code_span("a|b"), "`a|b`");
        assert_eq!(code_span("ünï`cödé"), "``ünï`cödé``");
    }
}
