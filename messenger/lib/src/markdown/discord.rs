use super::ast::{RichNode, split_edges};

/// Render a rich-text AST to Discord Markdown, escaping literal text.
pub fn render_discord(nodes: &[RichNode]) -> String {
    let mut out = String::new();
    render_nodes(&mut out, nodes, true);
    out.trim_end().to_string()
}

fn render_nodes(out: &mut String, nodes: &[RichNode], top_level: bool) {
    for (i, node) in nodes.iter().enumerate() {
        match node {
            RichNode::Text(s) => {
                let line_start = out.is_empty() || out.ends_with('\n');
                out.push_str(&escape_text(s, line_start));
            }
            RichNode::Bold(children) => render_delimited(out, "**", children),
            RichNode::Italic(children) => render_delimited(out, "*", children),
            RichNode::Strikethrough(children) => render_delimited(out, "~~", children),
            // A backtick inside the code would end a single-backtick span.
            RichNode::Code(s) if s.contains('`') => {
                out.push_str("`` ");
                out.push_str(s);
                out.push_str(" ``");
            }
            RichNode::Code(s) => {
                out.push('`');
                out.push_str(s);
                out.push('`');
            }
            RichNode::CodeBlock { language, code } => {
                out.push_str("```");
                if let Some(lang) = language {
                    out.push_str(lang);
                }
                out.push('\n');
                out.push_str(code);
                if !code.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str("```");
                out.push('\n');
            }
            RichNode::Link { url, children } => {
                let text = render_inline(children);
                if text.is_empty() || text == *url {
                    out.push_str(url);
                } else {
                    out.push('[');
                    out.push_str(&text);
                    out.push_str("](");
                    out.push_str(url);
                    out.push(')');
                }
            }
            RichNode::List { ordered, items } => {
                for (idx, item) in items.iter().enumerate() {
                    if *ordered {
                        out.push_str(&format!("{}. ", idx + 1));
                    } else {
                        out.push_str("- ");
                    }
                    render_nodes(out, item, false);
                    out.push('\n');
                }
            }
            RichNode::Paragraph(children) => {
                render_nodes(out, children, false);
                if top_level && i + 1 < nodes.len() {
                    out.push_str("\n\n");
                }
            }
            RichNode::Heading { level, children } => {
                for _ in 0..*level {
                    out.push('#');
                }
                out.push(' ');
                render_nodes(out, children, false);
                out.push('\n');
            }
            RichNode::SoftBreak => out.push(' '),
            RichNode::HardBreak => out.push('\n'),
        }
    }
}

/// Writes `children` between `delimiter` runs, with edge whitespace and
/// breaks outside them so each run touches the text it marks.
fn render_delimited(out: &mut String, delimiter: &str, children: &[RichNode]) {
    let (leading, content, trailing) = split_edges(children);
    render_nodes(out, &leading, false);
    let body = render_inline(&content);
    if !body.is_empty() {
        out.push_str(delimiter);
        out.push_str(&body);
        out.push_str(delimiter);
    }
    render_nodes(out, &trailing, false);
}

fn render_inline(nodes: &[RichNode]) -> String {
    let mut out = String::new();
    render_nodes(&mut out, nodes, false);
    out
}

/// Escapes a literal text value so Discord shows it as written: formatting
/// comes only from the AST's wrapper nodes. A backslash goes before each
/// character Discord reads as inline syntax (`\`, `*`, `~`, `` ` ``, `|`,
/// `[`, `]`, `<`, and a `_` not inside a word), and, when the text starts a
/// line, before a heading, quote, or list marker there.
fn escape_text(text: &str, line_start: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 4);
    let mut at_line_start = line_start;
    let mut block_escape_at = None;
    for (index, &c) in chars.iter().enumerate() {
        if at_line_start && c != ' ' {
            block_escape_at = block_marker(&chars[index..]).map(|offset| index + offset);
        }
        at_line_start = c == '\n' || (at_line_start && c == ' ');
        let in_word = |side: Option<&char>| side.is_some_and(|c| c.is_alphanumeric());
        let inline = match c {
            '\\' | '*' | '~' | '`' | '|' | '[' | ']' | '<' => true,
            '_' => {
                !(in_word(index.checked_sub(1).map(|i| &chars[i])) && in_word(chars.get(index + 1)))
            }
            _ => false,
        };
        if inline || block_escape_at == Some(index) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Where a backslash keeps `rest`, at the start of a line, from beginning a
/// Discord heading (`#`, `-#`), quote (`>`), or list item (`-`, `*`, `+`,
/// `1.`): the offset of the marker character to escape.
fn block_marker(rest: &[char]) -> Option<usize> {
    let marker_end = |at: usize| rest.get(at).is_none_or(|c| *c == ' ');
    match rest.first()? {
        '>' => Some(0),
        '#' => marker_end(rest.iter().take_while(|c| **c == '#').count()).then_some(0),
        '-' if rest.get(1) == Some(&'#') => Some(0),
        '-' | '*' | '+' => marker_end(1).then_some(0),
        c if c.is_ascii_digit() => {
            let run = rest.iter().take_while(|c| c.is_ascii_digit()).count();
            (rest.get(run) == Some(&'.') && marker_end(run + 1)).then_some(run)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse::parse_markdown;

    #[test]
    fn renders_bold_and_italic() {
        let nodes = parse_markdown("**bold** and _italic_");
        let result = render_discord(&nodes);
        assert_eq!(result, "**bold** and *italic*");
    }

    #[test]
    fn renders_code_block() {
        let nodes = parse_markdown("```rust\nfn main() {}\n```");
        let result = render_discord(&nodes);
        assert_eq!(result, "```rust\nfn main() {}\n```");
    }

    #[test]
    fn renders_strikethrough() {
        let nodes = parse_markdown("~~deleted~~");
        let result = render_discord(&nodes);
        assert_eq!(result, "~~deleted~~");
    }

    #[test]
    fn escaped_markdown_source_stays_literal_text() {
        // The parser removes the source escapes, so the text holds literal
        // syntax characters that must not become formatting again.
        for (source, expected) in [
            (r"\*\*literal\*\*", r"\*\*literal\*\*"),
            (r"\_literal\_ and snake_case", r"\_literal\_ and snake_case"),
            (r"\[literal\](https://x.io)", r"\[literal\](https://x.io)"),
            (r"\<em>literal\</em>", r"\<em>literal\</em>"),
            (r"\~\~gone\~\~ a\|b\|c \`tick\`", r"\~\~gone\~\~ a\|b\|c \`tick\`"),
            (r"C:\\dir", r"C:\\dir"),
            ("&copy; stays decoded as ©", "© stays decoded as ©"),
        ] {
            assert_eq!(render_discord(&parse_markdown(source)), expected, "{source:?}");
        }
    }

    #[test]
    fn text_starting_a_line_cannot_start_a_block() {
        for (source, expected) in [
            (r"\# heading", r"\# heading"),
            (r"\> quote", r"\> quote"),
            (r"\- item", r"\- item"),
            (r"\+ item", r"\+ item"),
            (r"1\. item", r"1\. item"),
            ("a  \n\\- after a hard break", "a\n\\- after a hard break"),
            ("x # not at a line start", "x # not at a line start"),
        ] {
            assert_eq!(render_discord(&parse_markdown(source)), expected, "{source:?}");
        }
    }

    #[test]
    fn code_containing_a_backtick_keeps_it() {
        assert_eq!(render_discord(&parse_markdown("``a`b``")), "`` a`b ``");
        assert_eq!(render_discord(&parse_markdown("`*a*`")), "`*a*`");
    }

    type Wrap = fn(Vec<RichNode>) -> RichNode;

    /// Each wrapper with its Discord delimiter.
    fn wrappers() -> [(Wrap, &'static str); 3] {
        [
            (RichNode::Bold, "**"),
            (RichNode::Italic, "*"),
            (RichNode::Strikethrough, "~~"),
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
        // Discord's `*` italic does not open before whitespace or close after
        // it; `**` and `~~` are moved the same way for one rule.
        for (wrap, delimiter) in wrappers() {
            for (edge, shown) in edges() {
                let leading = vec![RichNode::Paragraph(vec![
                    RichNode::Text("a".into()),
                    wrap(vec![edge.clone(), RichNode::Text("b".into())]),
                ])];
                assert_eq!(
                    render_discord(&leading),
                    format!("a{shown}{delimiter}b{delimiter}"),
                    "{edge:?}"
                );
                let trailing = vec![RichNode::Paragraph(vec![
                    wrap(vec![RichNode::Text("b".into()), edge.clone()]),
                    RichNode::Text("c".into()),
                ])];
                assert_eq!(
                    render_discord(&trailing),
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
        assert_eq!(render_discord(&nested), "x ***a*** y");
        let blank = vec![RichNode::Paragraph(vec![
            RichNode::Text("x".into()),
            RichNode::Italic(vec![RichNode::Text(" ".into())]),
            RichNode::Text("y".into()),
        ])];
        assert_eq!(render_discord(&blank), "x y");
    }

    #[test]
    fn parsed_emphasis_with_an_entity_space_keeps_its_delimiters_on_text() {
        // `&#32;` is whitespace only after parsing, so the source is emphasis.
        let nodes = parse_markdown("a *&#32;b* c");
        assert_eq!(render_discord(&nodes), "a  *b* c");
    }

    #[test]
    fn delimiters_inside_words_and_around_punctuation_are_kept() {
        // Discord's `*`, `**`, and `~~` have no word-boundary or punctuation
        // rule (only `_` italics need word boundaries, and they are not
        // written), so these spellings already format.
        for (wrap, delimiter) in wrappers() {
            for (before, body, after) in [("a", "b", "c"), ("a", "(b)", "c"), ("a", "(b)", " c")] {
                let nodes = vec![RichNode::Paragraph(vec![
                    RichNode::Text(before.into()),
                    wrap(vec![RichNode::Text(body.into())]),
                    RichNode::Text(after.into()),
                ])];
                assert_eq!(
                    render_discord(&nodes),
                    format!("{before}{delimiter}{body}{delimiter}{after}")
                );
            }
        }
    }
}
