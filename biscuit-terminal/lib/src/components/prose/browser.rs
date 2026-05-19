//! Browser emitter: [`BrowserRenderable`] for [`Prose`].
//!
//! Renders the [`ProseDocument`] IR to a `BrowserFragment<Ready>`. Semantic
//! styles map to semantic HTML (`<strong>`, `<em>`, `<s>`); presentational
//! styles map to `<span style="…">`. Browser rendering is infallible —
//! unsupported constructs degrade to escaped literal text.

use std::any::Any;

use renderable::browser::BrowserRenderable;
use renderable::browser::fragment::{BrowserFragment, ComposableNode, Ready};
use renderable::browser::utils::{escape_attribute, escape_text};
use renderable::html::tag::BlockTag;

use super::ir::{ProseNode, ProseStyle};
use super::prose::Prose;
use super::styles::{css_color, resolve_href};

impl BrowserRenderable for Prose {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let mut html = String::new();
        for node in &self.document().children {
            node_to_html(node, &mut html);
        }
        BrowserFragment::new()
            .define_as_block_tag(BlockTag::Span, "prose")
            .add_child(ComposableNode::RawHtml(html))
            .finalize()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Append the HTML for a single node to `out`.
///
/// Text and code-block bodies are HTML-escaped; href attribute values are
/// attribute-escaped. The escaping is what keeps unknown Prose tags — which
/// the parser already lowered to literal [`ProseNode::Text`] — visible and
/// inert across the Browser target.
fn node_to_html(node: &ProseNode, out: &mut String) {
    match node {
        ProseNode::Text(text) => out.push_str(&escape_text(text)),
        ProseNode::CodeBlock { lang, value } => {
            out.push_str("<pre><code");
            if let Some(lang) = lang {
                out.push_str(&format!(
                    " class=\"language-{}\"",
                    escape_attribute(lang)
                ));
            }
            out.push('>');
            out.push_str(&escape_text(value));
            out.push_str("</code></pre>");
        }
        ProseNode::Link { href, children } => {
            let resolved = resolve_href(href);
            if resolved.is_empty() {
                emit_children(children, out);
            } else {
                out.push_str(&format!("<a href=\"{}\">", escape_attribute(&resolved)));
                emit_children(children, out);
                out.push_str("</a>");
            }
        }
        ProseNode::Span { style, children } => {
            let wrappers = span_wrappers(style);
            for (open, _) in &wrappers {
                out.push_str(open);
            }
            emit_children(children, out);
            for (_, close) in wrappers.iter().rev() {
                out.push_str(close);
            }
        }
    }
}

/// Render a node's children into `out`.
fn emit_children(children: &[ProseNode], out: &mut String) {
    for child in children {
        node_to_html(child, out);
    }
}

/// The opening/closing HTML tag pairs for a style.
///
/// Weight / decoration wrappers come from the shared
/// [`TextEmphasis`](renderable::style::TextEmphasis) emitter; the
/// `Prose`-only inverse/hidden/color wrappers are added here. The parser
/// produces single-attribute styles, so in practice this yields one pair;
/// multiple set fields nest cleanly.
fn span_wrappers(style: &ProseStyle) -> Vec<(String, &'static str)> {
    let mut wrappers = style.emphasis.html_wrappers();

    if style.hidden {
        wrappers.push((
            "<span style=\"visibility: hidden\">".to_string(),
            "</span>",
        ));
    }
    if style.inverse {
        wrappers.push(("<span style=\"filter: invert(1)\">".to_string(), "</span>"));
    }
    if let Some(fg) = &style.fg {
        wrappers.push((
            format!("<span style=\"color: {}\">", css_color(fg)),
            "</span>",
        ));
    }
    if let Some(bg) = &style.bg {
        wrappers.push((
            format!("<span style=\"background-color: {}\">", css_color(bg)),
            "</span>",
        ));
    }

    wrappers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn html(input: &str) -> String {
        Prose::new(input).render_html_fragment().render()
    }

    #[test]
    fn plain_text_renders() {
        assert!(html("hello world").contains("hello world"));
    }

    #[test]
    fn bold_uses_semantic_strong() {
        assert!(html("<b>x</b>").contains("<strong>x</strong>"));
    }

    #[test]
    fn italic_uses_semantic_em() {
        assert!(html("<i>x</i>").contains("<em>x</em>"));
    }

    #[test]
    fn nested_bold_italic_nests() {
        assert!(html("<b><i>x</i></b>").contains("<strong><em>x</em></strong>"));
    }

    #[test]
    fn strikethrough_uses_semantic_s() {
        assert!(html("<~>x</~>").contains("<s>x</s>"));
    }

    #[test]
    fn link_renders_anchor() {
        let h = html(r#"<a href="https://example.com">go</a>"#);
        assert!(
            h.contains(r#"<a href="https://example.com">go</a>"#),
            "got: {h}"
        );
    }

    #[test]
    fn link_href_from_quoted_attr_preserves_escaped_delimiter() {
        let attr = Prose::quoted_attr("https://example.com/a>b");
        let h = html(&format!("<a href={attr}>x</a>"));
        assert!(
            h.contains(r#"<a href="https://example.com/a>b">x</a>"#),
            "got: {h}"
        );
    }

    #[test]
    fn link_href_from_quoted_attr_with_both_quote_types_is_attribute_escaped() {
        let attr = Prose::quoted_attr(r#"https://example.com/a>b?q='x'&r="y""#);
        let h = html(&format!("<a href={attr}>x</a>"));
        assert!(
            h.contains(
                r#"<a href="https://example.com/a>b?q=&#39;x&#39;&amp;r=&quot;y&quot;">x</a>"#
            ),
            "got: {h}"
        );
    }

    #[test]
    fn foreground_color_uses_span_style() {
        let h = html("<red>x</red>");
        assert!(h.contains("<span style=\"color: rgb("), "got: {h}");
    }

    #[test]
    fn background_color_uses_span_style() {
        let h = html("<bg-coral>x</bg-coral>");
        assert!(
            h.contains("<span style=\"background-color: rgb("),
            "got: {h}"
        );
    }

    #[test]
    fn rgb_color_renders_exact_channels() {
        assert!(html("<rgb 1,2,3>x</rgb>").contains("rgb(1, 2, 3)"));
    }

    #[test]
    fn code_block_uses_pre_code() {
        assert!(html("```\ncode line\n```").contains("<pre><code>code line</code></pre>"));
    }

    #[test]
    fn code_block_with_tag_delimiters_in_body_is_escaped_and_opaque() {
        // Regression: a fenced body containing the synthetic
        // `</code-block>` tag plus `<red>` markup must render as one
        // escaped `<pre><code>` body — no styled span leaks outside it.
        let h = html("```\n</code-block><red>x</red>\n```");
        assert!(
            h.contains(
                "<pre><code>&lt;/code-block&gt;&lt;red&gt;x&lt;/red&gt;</code></pre>"
            ),
            "got: {h}"
        );
        assert!(!h.contains("<span style=\"color"), "got: {h}");
    }

    #[test]
    fn code_block_with_unknown_tag_and_backslashes_is_escaped_verbatim() {
        let h = html("```\n<unknown> \\* \\\\path\n```");
        assert!(
            h.contains("<pre><code>&lt;unknown&gt; \\* \\\\path</code></pre>"),
            "got: {h}"
        );
    }

    #[test]
    fn unknown_tag_is_escaped_text() {
        let h = html("<unknown>x</unknown>");
        assert!(h.contains("&lt;unknown&gt;x&lt;/unknown&gt;"), "got: {h}");
    }

    #[test]
    fn user_text_is_html_escaped() {
        let h = html(r"\<script\>alert\<\/script\>");
        assert!(h.contains("&lt;script&gt;"), "got: {h}");
        assert!(!h.contains("<script>"), "got: {h}");
    }
}
