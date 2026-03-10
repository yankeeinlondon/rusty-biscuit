use super::ast::RichNode;

/// Render a rich-text AST to Discord Markdown (near pass-through).
pub fn render_discord(nodes: &[RichNode]) -> String {
    let mut out = String::new();
    render_nodes(&mut out, nodes, true);
    out.trim_end().to_string()
}

fn render_nodes(out: &mut String, nodes: &[RichNode], top_level: bool) {
    for (i, node) in nodes.iter().enumerate() {
        match node {
            RichNode::Text(s) => out.push_str(s),
            RichNode::Bold(children) => {
                out.push_str("**");
                render_nodes(out, children, false);
                out.push_str("**");
            }
            RichNode::Italic(children) => {
                out.push('*');
                render_nodes(out, children, false);
                out.push('*');
            }
            RichNode::Strikethrough(children) => {
                out.push_str("~~");
                render_nodes(out, children, false);
                out.push_str("~~");
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
}
