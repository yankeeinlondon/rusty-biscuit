use super::ast::RichNode;

/// Render a rich-text AST to Slack mrkdwn format.
pub fn render_slack_mrkdwn(nodes: &[RichNode]) -> String {
    let mut out = String::new();
    render_nodes(&mut out, nodes, true);
    out.trim_end().to_string()
}

fn render_nodes(out: &mut String, nodes: &[RichNode], top_level: bool) {
    for (i, node) in nodes.iter().enumerate() {
        match node {
            RichNode::Text(s) => out.push_str(s),
            RichNode::Bold(children) => {
                out.push('*');
                render_nodes(out, children, false);
                out.push('*');
            }
            RichNode::Italic(children) => {
                out.push('_');
                render_nodes(out, children, false);
                out.push('_');
            }
            RichNode::Strikethrough(children) => {
                out.push('~');
                render_nodes(out, children, false);
                out.push('~');
            }
            RichNode::Code(s) => {
                out.push('`');
                out.push_str(s);
                out.push('`');
            }
            RichNode::CodeBlock { code, .. } => {
                out.push_str("```\n");
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
                // Slack has no heading syntax — render as bold
                out.push('*');
                render_nodes(out, children, false);
                out.push('*');
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
    fn renders_heading_as_bold() {
        let nodes = parse_markdown("## Status");
        let result = render_slack_mrkdwn(&nodes);
        assert_eq!(result, "*Status*");
    }
}
