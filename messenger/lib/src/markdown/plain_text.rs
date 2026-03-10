use super::ast::RichNode;

/// Render a rich-text AST to plain text, stripping all formatting.
pub fn render_plain_text(nodes: &[RichNode]) -> String {
    let mut out = String::new();
    render_nodes(&mut out, nodes, true);
    out.trim_end().to_string()
}

fn render_nodes(out: &mut String, nodes: &[RichNode], top_level: bool) {
    for (i, node) in nodes.iter().enumerate() {
        match node {
            RichNode::Text(s) => out.push_str(s),
            RichNode::Bold(children) | RichNode::Italic(children) | RichNode::Strikethrough(children) => {
                render_nodes(out, children, false);
            }
            RichNode::Code(s) => out.push_str(s),
            RichNode::CodeBlock { code, .. } => {
                out.push_str(code);
                if !code.ends_with('\n') {
                    out.push('\n');
                }
            }
            RichNode::Link { children, url, .. } => {
                let text = render_inline(children);
                if text.is_empty() || text == *url {
                    out.push_str(url);
                } else {
                    out.push_str(&text);
                    out.push_str(" (");
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
            RichNode::Heading { children, .. } => {
                render_nodes(out, children, false);
                out.push('\n');
            }
            RichNode::SoftBreak => out.push(' '),
            RichNode::HardBreak => out.push('\n'),
        }
    }
}

fn render_inline(nodes: &[RichNode]) -> String {
    let mut out = String::new();
    render_nodes(&mut out, nodes, false);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse::parse_markdown;

    #[test]
    fn strips_all_formatting() {
        let nodes = parse_markdown("**bold** and _italic_ with `code`");
        let plain = render_plain_text(&nodes);
        assert_eq!(plain, "bold and italic with code");
    }

    #[test]
    fn renders_link_with_url() {
        let nodes = parse_markdown("[click](https://example.com)");
        let plain = render_plain_text(&nodes);
        assert_eq!(plain, "click (https://example.com)");
    }

    #[test]
    fn renders_list() {
        let nodes = parse_markdown("- one\n- two");
        let plain = render_plain_text(&nodes);
        assert_eq!(plain, "- one\n- two");
    }
}
