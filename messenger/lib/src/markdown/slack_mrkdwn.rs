use super::ast::{RichNode, split_edges};

/// Render a rich-text AST to Slack mrkdwn format.
pub fn render_slack_mrkdwn(nodes: &[RichNode]) -> String {
    let mut out = String::new();
    render_nodes(&mut out, nodes, true, None);
    out.trim_end().to_string()
}

/// Renders `nodes` into `out`. `following` is the character written right
/// after them, if one is known.
fn render_nodes(out: &mut String, nodes: &[RichNode], top_level: bool, following: Option<char>) {
    for (i, node) in nodes.iter().enumerate() {
        let after = || first_char(&nodes[i + 1..]).or(following);
        match node {
            RichNode::Text(s) => out.push_str(&escape_control(s)),
            RichNode::Bold(children) => render_delimited(out, '*', children, after()),
            RichNode::Italic(children) => render_delimited(out, '_', children, after()),
            RichNode::Strikethrough(children) => render_delimited(out, '~', children, after()),
            RichNode::Code(s) => {
                out.push('`');
                out.push_str(&escape_control(s));
                out.push('`');
            }
            RichNode::CodeBlock { code, .. } => {
                out.push_str("```\n");
                out.push_str(&escape_control(code));
                if !code.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str("```");
                out.push('\n');
            }
            RichNode::Link { url, children } => {
                let text = render_inline(children, Some('>'));
                if text.is_empty() || text == *url {
                    out.push('<');
                    out.push_str(url);
                    out.push('>');
                } else {
                    out.push('<');
                    out.push_str(url);
                    out.push('|');
                    out.push_str(&text);
                    out.push('>');
                }
            }
            RichNode::List { ordered, items } => {
                for (idx, item) in items.iter().enumerate() {
                    if *ordered {
                        out.push_str(&format!("{}. ", idx + 1));
                    } else {
                        out.push_str("• ");
                    }
                    render_nodes(out, item, false, Some('\n'));
                    out.push('\n');
                }
            }
            RichNode::Paragraph(children) => {
                render_nodes(out, children, false, None);
                if top_level && i + 1 < nodes.len() {
                    out.push_str("\n\n");
                }
            }
            RichNode::Heading { children, .. } => {
                // Slack has no heading syntax — render as bold
                out.push('*');
                render_nodes(out, children, false, Some('*'));
                out.push('*');
                out.push('\n');
            }
            RichNode::SoftBreak => out.push(' '),
            RichNode::HardBreak => out.push('\n'),
        }
    }
}

/// Writes `children` between `delimiter` runs, with edge whitespace and
/// breaks outside them so each run touches the text it marks.
///
/// Slack only formats a run at a word boundary: an alphanumeric outside the
/// opener or the closer (`a*b*c`), or the same delimiter character there,
/// leaves the runs as literal characters. No other spelling exists in
/// mrkdwn, so such a wrapper is written as its text alone, as the plain-text
/// renderer writes every wrapper.
fn render_delimited(
    out: &mut String,
    delimiter: char,
    children: &[RichNode],
    following: Option<char>,
) {
    let (leading, content, trailing) = split_edges(children);
    render_nodes(out, &leading, false, None);
    let after = first_char(&trailing).or(following);
    let at_boundary =
        |side: Option<char>| side.is_none_or(|c| !c.is_alphanumeric() && c != delimiter);
    if at_boundary(out.chars().next_back()) && at_boundary(after) {
        // The body starts after the opener, so a nested wrapper at its start
        // sees that character as its neighbor.
        let mut body = String::from(delimiter);
        render_nodes(&mut body, &content, false, Some(delimiter));
        if body.len() > delimiter.len_utf8() {
            out.push_str(&body);
            out.push(delimiter);
        }
    } else {
        render_nodes(out, &content, false, after);
    }
    render_nodes(out, &trailing, false, following);
}

/// The first character `nodes` write, as far as a preceding delimiter run
/// is concerned. A wrapper counts as its first content character, because
/// it may itself be written without delimiters.
fn first_char(nodes: &[RichNode]) -> Option<char> {
    for node in nodes {
        let first = match node {
            RichNode::Text(text) => text.chars().next(),
            RichNode::Bold(children)
            | RichNode::Italic(children)
            | RichNode::Strikethrough(children)
            | RichNode::Paragraph(children) => first_char(children),
            RichNode::Code(_) | RichNode::CodeBlock { .. } => Some('`'),
            RichNode::Link { .. } => Some('<'),
            RichNode::Heading { .. } => Some('*'),
            RichNode::List { .. } | RichNode::HardBreak => Some('\n'),
            RichNode::SoftBreak => Some(' '),
        };
        if first.is_some() {
            return first;
        }
    }
    None
}

/// Escapes Slack's three control characters, which it reads as markup in
/// every position, code included (`<https://x|label>`, `<!channel>`), so a
/// literal value stays literal. mrkdwn has no escape for `*`, `_`, `~`, or
/// `` ` ``: a literal formatting spelling in text still formats.
fn escape_control(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn render_inline(nodes: &[RichNode], following: Option<char>) -> String {
    let mut out = String::new();
    render_nodes(&mut out, nodes, false, following);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse::parse_markdown;

    #[test]
    fn renders_bold_and_italic() {
        let nodes = parse_markdown("**bold** and _italic_");
        let result = render_slack_mrkdwn(&nodes);
        assert_eq!(result, "*bold* and _italic_");
    }

    #[test]
    fn renders_link() {
        let nodes = parse_markdown("[click](https://example.com)");
        let result = render_slack_mrkdwn(&nodes);
        assert_eq!(result, "<https://example.com|click>");
    }

    #[test]
    fn renders_list_with_bullets() {
        let nodes = parse_markdown("- one\n- two");
        let result = render_slack_mrkdwn(&nodes);
        assert_eq!(result, "• one\n• two");
    }

    #[test]
    fn renders_code_block_without_language() {
        let nodes = parse_markdown("```rust\nfn main() {}\n```");
        let result = render_slack_mrkdwn(&nodes);
        // Slack doesn't support language tags in code blocks
        assert_eq!(result, "```\nfn main() {}\n```");
    }

    #[test]
    fn control_characters_in_text_and_code_stay_literal() {
        let nodes = parse_markdown(r"a \<!channel> & \<https://x|y> `<b>`");
        assert_eq!(
            render_slack_mrkdwn(&nodes),
            "a &lt;!channel&gt; &amp; &lt;https://x|y&gt; `&lt;b&gt;`"
        );
        let link = parse_markdown(r"[a \<b> & c](https://x.io)");
        assert_eq!(
            render_slack_mrkdwn(&link),
            "<https://x.io|a &lt;b&gt; &amp; c>"
        );
    }

    #[test]
    fn renders_heading_as_bold() {
        let nodes = parse_markdown("## Status");
        let result = render_slack_mrkdwn(&nodes);
        assert_eq!(result, "*Status*");
    }

    type Wrap = fn(Vec<RichNode>) -> RichNode;

    /// Each wrapper with its Slack delimiter.
    fn wrappers() -> [(Wrap, &'static str); 3] {
        [
            (RichNode::Bold, "*"),
            (RichNode::Italic, "_"),
            (RichNode::Strikethrough, "~"),
        ]
    }

    /// Each edge node with the text it renders as.
    fn edges() -> [(RichNode, &'static str); 4] {
        [
            (RichNode::Text(" ".into()), " "),
            (RichNode::Text("\t".into()), "\t"),
            (RichNode::SoftBreak, " "),
            (RichNode::HardBreak, "\n"),
        ]
    }

    #[test]
    fn edge_whitespace_and_breaks_are_written_outside_the_delimiters() {
        // Slack does not open a delimiter before whitespace or close one
        // after it.
        for (wrap, delimiter) in wrappers() {
            for (edge, shown) in edges() {
                let leading = vec![RichNode::Paragraph(vec![
                    RichNode::Text("a".into()),
                    wrap(vec![edge.clone(), RichNode::Text("b".into())]),
                ])];
                assert_eq!(
                    render_slack_mrkdwn(&leading),
                    format!("a{shown}{delimiter}b{delimiter}"),
                    "{edge:?}"
                );
                let trailing = vec![RichNode::Paragraph(vec![
                    wrap(vec![RichNode::Text("b".into()), edge.clone()]),
                    RichNode::Text("c".into()),
                ])];
                assert_eq!(
                    render_slack_mrkdwn(&trailing),
                    format!("{delimiter}b{delimiter}{shown}c"),
                    "{edge:?}"
                );
            }
        }
    }

    #[test]
    fn nested_and_whitespace_only_wrappers_keep_delimiters_on_text() {
        let nested = vec![RichNode::Paragraph(vec![
            RichNode::Text("x".into()),
            RichNode::Bold(vec![RichNode::Italic(vec![RichNode::Text(" a ".into())])]),
            RichNode::Text("y".into()),
        ])];
        assert_eq!(render_slack_mrkdwn(&nested), "x *_a_* y");
        let blank = vec![RichNode::Paragraph(vec![
            RichNode::Text("x".into()),
            RichNode::Italic(vec![RichNode::Text(" ".into())]),
            RichNode::Text("y".into()),
        ])];
        assert_eq!(render_slack_mrkdwn(&blank), "x y");
    }

    #[test]
    fn parsed_emphasis_with_an_entity_space_keeps_its_delimiters_on_text() {
        // `&#32;` is whitespace only after parsing, so the source is emphasis.
        let nodes = parse_markdown("a *&#32;b* c");
        assert_eq!(render_slack_mrkdwn(&nodes), "a  _b_ c");
    }

    fn paragraph(nodes: Vec<RichNode>) -> Vec<RichNode> {
        vec![RichNode::Paragraph(nodes)]
    }

    fn text(value: &str) -> RichNode {
        RichNode::Text(value.into())
    }

    #[test]
    fn a_wrapper_against_a_word_character_is_written_unstyled() {
        // Slack formats a run only at a word boundary, and mrkdwn has no
        // other spelling.
        for (wrap, delimiter) in wrappers() {
            let cases = [
                (
                    vec![text("a"), wrap(vec![text("b")]), text("c")],
                    "abc".to_string(),
                ),
                (
                    vec![text("a"), wrap(vec![text("b")]), text(" c")],
                    "ab c".to_string(),
                ),
                (
                    vec![text("a "), wrap(vec![text("b")]), text("c")],
                    "a bc".to_string(),
                ),
                (
                    vec![text("a"), wrap(vec![text("(b)")]), text("c")],
                    "a(b)c".to_string(),
                ),
                (
                    vec![text("a "), wrap(vec![text("(b)")]), text(" c")],
                    format!("a {delimiter}(b){delimiter} c"),
                ),
                (
                    vec![text("("), wrap(vec![text("b")]), text(")")],
                    format!("({delimiter}b{delimiter})"),
                ),
                (
                    vec![
                        text("a "),
                        wrap(vec![text("b")]),
                        RichNode::Code("c".into()),
                    ],
                    format!("a {delimiter}b{delimiter}`c`"),
                ),
            ];
            for (nodes, expected) in cases {
                assert_eq!(
                    render_slack_mrkdwn(&paragraph(nodes)),
                    expected,
                    "{delimiter}"
                );
            }
        }
    }

    #[test]
    fn a_wrapper_beside_its_own_delimiter_is_written_unstyled() {
        let nested = paragraph(vec![
            text("x "),
            RichNode::Bold(vec![RichNode::Bold(vec![text("b")])]),
            text(" y"),
        ]);
        assert_eq!(render_slack_mrkdwn(&nested), "x *b* y");
        let heading = vec![RichNode::Heading {
            level: 2,
            children: vec![RichNode::Bold(vec![text("b")])],
        }];
        assert_eq!(render_slack_mrkdwn(&heading), "*b*");
        let literal = paragraph(vec![
            text("x *"),
            RichNode::Bold(vec![text("b")]),
            text(" y"),
        ]);
        assert_eq!(render_slack_mrkdwn(&literal), "x *b y");
    }

    #[test]
    fn a_wrapper_inside_a_word_wrapper_sees_the_outer_word() {
        // An outer wrapper written without runs leaves its content against
        // the outer word, so a nested wrapper is unstyled too.
        let nodes = paragraph(vec![
            text("a"),
            RichNode::Bold(vec![RichNode::Italic(vec![text("b")])]),
            text(" c"),
        ]);
        assert_eq!(render_slack_mrkdwn(&nodes), "ab c");
        let nodes = paragraph(vec![
            text("a "),
            RichNode::Bold(vec![text("x"), RichNode::Italic(vec![text("b")])]),
            text(" c"),
        ]);
        assert_eq!(render_slack_mrkdwn(&nodes), "a *xb* c");
    }
}
