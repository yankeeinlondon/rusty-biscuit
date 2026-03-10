use super::ast::RichNode;

/// Render a rich-text AST to Telegram HTML format.
pub fn render_telegram_html(nodes: &[RichNode]) -> String {
    let mut out = String::new();
    render_nodes(&mut out, nodes, true);
    out.trim_end().to_string()
}

fn render_nodes(out: &mut String, nodes: &[RichNode], top_level: bool) {
    for (i, node) in nodes.iter().enumerate() {
        match node {
            RichNode::Text(s) => out.push_str(&escape_html(s)),
            RichNode::Bold(children) => {
                out.push_str("<b>");
                render_nodes(out, children, false);
                out.push_str("</b>");
            }
            RichNode::Italic(children) => {
                out.push_str("<i>");
                render_nodes(out, children, false);
                out.push_str("</i>");
            }
            RichNode::Strikethrough(children) => {
                out.push_str("<s>");
                render_nodes(out, children, false);
                out.push_str("</s>");
            }
            RichNode::Code(s) => {
                out.push_str("<code>");
                out.push_str(&escape_html(s));
                out.push_str("</code>");
            }
            RichNode::CodeBlock { language, code } => {
                if let Some(lang) = language {
                    out.push_str(&format!("<pre><code class=\"language-{}\">", escape_html(lang)));
                } else {
                    out.push_str("<pre><code>");
                }
                out.push_str(&escape_html(code));
                out.push_str("</code></pre>");
                out.push('\n');
            }
            RichNode::Link { url, children } => {
                let text = render_inline(children);
                out.push_str("<a href=\"");
                out.push_str(&escape_html(url));
                out.push_str("\">");
                if text.is_empty() {
                    out.push_str(&escape_html(url));
                } else {
                    out.push_str(&text);
                }
                out.push_str("</a>");
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
                out.push_str("<b>");
                render_nodes(out, children, false);
                out.push_str("</b>");
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

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse::parse_markdown;

    #[test]
    fn renders_bold_and_italic() {
        let nodes = parse_markdown("**bold** and _italic_");
        let result = render_telegram_html(&nodes);
        assert_eq!(result, "<b>bold</b> and <i>italic</i>");
    }

    #[test]
    fn renders_code() {
        let nodes = parse_markdown("use `foo`");
        let result = render_telegram_html(&nodes);
        assert_eq!(result, "use <code>foo</code>");
    }

    #[test]
    fn escapes_html_entities() {
        let nodes = parse_markdown("a < b & c > d");
        let result = render_telegram_html(&nodes);
        assert_eq!(result, "a &lt; b &amp; c &gt; d");
    }

    #[test]
    fn renders_link() {
        let nodes = parse_markdown("[click](https://example.com)");
        let result = render_telegram_html(&nodes);
        assert_eq!(result, "<a href=\"https://example.com\">click</a>");
    }
}
