//! Markdown emitter: [`MarkdownRenderable`] for [`Prose`].
//!
//! Renders the [`ProseDocument`] IR to two Markdown flavors:
//!
//! - [`render_markdown`](MarkdownRenderable::render_markdown) — portable
//!   plain Markdown. Semantic styles (bold, italic, strikethrough, links,
//!   code) map to Markdown sigils; terminal-only presentation (colors,
//!   underline variants, blink, inverse, hidden) degrades to readable inner
//!   text.
//! - [`render_markdown_plus`](MarkdownRenderable::render_markdown_plus) —
//!   the richer target. Semantic styles still use Markdown sigils, but
//!   foreground/background color and underline variants are preserved with
//!   inline HTML (`<span style="…">`). No JavaScript is used.

use renderable::markdown::MarkdownRenderable;

use super::ir::{ProseNode, ProseStyle};
use super::prose::Prose;
use super::styles::{css_color, resolve_href};

impl MarkdownRenderable for Prose {
    fn render_markdown(&self) -> String {
        emit(&self.document().children, false)
    }

    fn render_markdown_plus(&self) -> String {
        emit(&self.document().children, true)
    }
}

/// Render a slice of nodes to a Markdown string.
///
/// `plus` selects MarkdownPlus, which keeps color and underline styling via
/// inline HTML; plain Markdown degrades those to inner text.
fn emit(nodes: &[ProseNode], plus: bool) -> String {
    let mut out = String::new();
    for node in nodes {
        node_to_markdown(node, plus, &mut out);
    }
    out
}

/// Append the Markdown for a single node to `out`.
fn node_to_markdown(node: &ProseNode, plus: bool, out: &mut String) {
    match node {
        ProseNode::Text(text) => out.push_str(&escape_markdown(text)),
        ProseNode::CodeBlock { lang, value } => {
            out.push_str("```");
            if let Some(lang) = lang {
                out.push_str(lang);
            }
            out.push('\n');
            out.push_str(value);
            if !value.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```");
        }
        ProseNode::Link { href, children } => {
            let resolved = resolve_href(href);
            let desc = emit(children, plus);
            if resolved.is_empty() {
                out.push_str(&desc);
            } else {
                // The description's Markdown-significant characters
                // (including `[` and `]`) are already escaped by
                // `escape_markdown` when each text node was emitted, so
                // `desc` needs no further escaping here.
                out.push('[');
                out.push_str(&desc);
                out.push_str("](");
                out.push_str(&escape_markdown_destination(&resolved));
                out.push(')');
            }
        }
        ProseNode::Span { style, children } => {
            let inner = emit(children, plus);
            out.push_str(&wrap_span(style, &inner, plus));
        }
    }
}

/// Wrap `inner` in the Markdown sigils — and, for MarkdownPlus, the inline
/// HTML — that carry `style`.
///
/// Plain Markdown wraps only the styles it can carry portably (bold, italic,
/// strikethrough); colors and underline variants pass through as readable
/// inner text. MarkdownPlus additionally preserves foreground/background
/// color and underline variants with `<span style="…">`.
fn wrap_span(style: &ProseStyle, inner: &str, plus: bool) -> String {
    let mut result = inner.to_string();

    if style.emphasis.strikethrough {
        result = format!("~~{}~~", result);
    }
    if style.emphasis.italic {
        result = format!("_{}_", result);
    }
    if style.emphasis.bold {
        result = format!("**{}**", result);
    }

    if plus {
        if let Some(underline) = style.emphasis.underline {
            result = format!(
                "<span style=\"{}\">{}</span>",
                underline.css_declaration(),
                result
            );
        }
        if let Some(fg) = &style.fg {
            result = format!("<span style=\"color: {}\">{}</span>", css_color(fg), result);
        }
        if let Some(bg) = &style.bg {
            result = format!(
                "<span style=\"background-color: {}\">{}</span>",
                css_color(bg),
                result
            );
        }
    }

    result
}

/// Escape an href so it survives intact as a CommonMark link destination.
///
/// A *bare* destination may not contain ASCII whitespace or control
/// characters, and any parentheses must be backslash-escaped — otherwise
/// the first unescaped `)` truncates the destination and the remainder
/// leaks out as literal text. When the href contains whitespace it is
/// emitted in *angle-bracket* form (`<…>`), which permits spaces but
/// forbids unescaped `<` and `>`; line endings, invalid in either form,
/// degrade to spaces.
fn escape_markdown_destination(href: &str) -> String {
    if href.chars().any(|c| c.is_ascii_whitespace()) {
        let mut out = String::with_capacity(href.len() + 2);
        out.push('<');
        for c in href.chars() {
            match c {
                '\\' | '<' | '>' => {
                    out.push('\\');
                    out.push(c);
                }
                '\n' | '\r' | '\t' => out.push(' '),
                _ => out.push(c),
            }
        }
        out.push('>');
        out
    } else {
        let mut out = String::with_capacity(href.len());
        for c in href.chars() {
            match c {
                '\\' | '(' | ')' => {
                    out.push('\\');
                    out.push(c);
                }
                _ => out.push(c),
            }
        }
        out
    }
}

/// Escape Markdown-significant characters so literal text keeps its meaning.
fn escape_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md(input: &str) -> String {
        Prose::new(input).render_markdown()
    }

    fn md_plus(input: &str) -> String {
        Prose::new(input).render_markdown_plus()
    }

    #[test]
    fn plain_text_passes_through() {
        assert_eq!(md("hello world"), "hello world");
    }

    #[test]
    fn bold_uses_double_asterisk() {
        assert_eq!(md("<b>x</b>"), "**x**");
    }

    #[test]
    fn italic_uses_underscore() {
        assert_eq!(md("<i>x</i>"), "_x_");
    }

    #[test]
    fn nested_bold_italic_nests() {
        assert_eq!(md("<b><i>x</i></b>"), "**_x_**");
    }

    #[test]
    fn strikethrough_uses_gfm_tilde() {
        assert_eq!(md("<~>x</~>"), "~~x~~");
    }

    #[test]
    fn link_uses_markdown_syntax() {
        assert_eq!(
            md(r#"<a href="https://example.com">go</a>"#),
            "[go](https://example.com)"
        );
    }

    #[test]
    fn link_destination_escapes_close_paren() {
        assert_eq!(
            md(r#"<a href="https://example.com/a)b">go</a>"#),
            r"[go](https://example.com/a\)b)"
        );
    }

    #[test]
    fn link_destination_escapes_open_paren() {
        assert_eq!(
            md(r#"<a href="https://example.com/a(b">go</a>"#),
            r"[go](https://example.com/a\(b)"
        );
    }

    #[test]
    fn link_destination_escapes_backslash() {
        assert_eq!(
            md(r#"<a href="https://example.com/a\b">go</a>"#),
            r"[go](https://example.com/a\\b)"
        );
    }

    #[test]
    fn link_destination_with_whitespace_uses_angle_brackets() {
        assert_eq!(
            md(r#"<a href="https://example.com/a b">go</a>"#),
            "[go](<https://example.com/a b>)"
        );
    }

    #[test]
    fn link_description_with_close_bracket_is_escaped_once() {
        assert_eq!(
            md(r#"<a href="https://example.com">a]b</a>"#),
            r"[a\]b](https://example.com)"
        );
    }

    #[test]
    fn foreground_color_degrades_to_inner_text() {
        assert_eq!(md("<red>important</red>"), "important");
    }

    #[test]
    fn background_color_degrades_to_inner_text() {
        assert_eq!(md("<bg-coral>note</bg-coral>"), "note");
    }

    #[test]
    fn rgb_color_degrades_to_inner_text() {
        assert_eq!(md("<rgb 1,2,3>colored</rgb>"), "colored");
    }

    #[test]
    fn underline_degrades_to_inner_text() {
        assert_eq!(md("<u>x</u>"), "x");
    }

    #[test]
    fn code_block_uses_fence() {
        assert_eq!(md("```rust\nlet x = 1;\n```"), "```rust\nlet x = 1;\n```");
    }

    #[test]
    fn code_block_with_tag_delimiters_in_body_stays_in_fence() {
        // Regression: a fenced body containing the synthetic
        // `</code-block>` tag plus `<red>` markup must round-trip
        // verbatim inside the fence — not collapse to an empty fence
        // with escaped tail text.
        assert_eq!(
            md("```\n</code-block><red>x</red>\n```"),
            "```\n</code-block><red>x</red>\n```",
        );
    }

    #[test]
    fn code_block_with_unknown_tag_and_backslashes_is_verbatim() {
        assert_eq!(
            md("```\n<unknown> \\* \\\\path\n```"),
            "```\n<unknown> \\* \\\\path\n```",
        );
    }

    #[test]
    fn unknown_tag_is_escaped_text() {
        assert_eq!(md("<unknown>x</unknown>"), r"\<unknown\>x\</unknown\>");
    }

    #[test]
    fn markdown_sigils_in_literal_text_are_escaped() {
        assert_eq!(md(r"\*not bold\*"), r"\*not bold\*");
    }

    // ── MarkdownPlus: richer styling preserved via inline HTML ───────

    #[test]
    fn markdown_plus_keeps_semantic_styles_as_markdown() {
        assert_eq!(md_plus("<b>x</b>"), "**x**");
        assert_eq!(md_plus("<i>x</i>"), "_x_");
        assert_eq!(md_plus("<~>x</~>"), "~~x~~");
    }

    #[test]
    fn markdown_plus_preserves_foreground_color() {
        let out = md_plus("<red>x</red>");
        assert_eq!(out, "<span style=\"color: rgb(128, 0, 0)\">x</span>");
    }

    #[test]
    fn markdown_plus_preserves_background_color() {
        let out = md_plus("<bg-coral>x</bg-coral>");
        assert!(
            out.starts_with("<span style=\"background-color: rgb("),
            "got: {out}"
        );
        assert!(out.ends_with("x</span>"), "got: {out}");
    }

    #[test]
    fn markdown_plus_preserves_rgb_color() {
        assert_eq!(
            md_plus("<rgb 1,2,3>x</rgb>"),
            "<span style=\"color: rgb(1, 2, 3)\">x</span>"
        );
    }

    #[test]
    fn markdown_plus_preserves_underline_variants() {
        assert_eq!(
            md_plus("<u>x</u>"),
            "<span style=\"text-decoration: underline\">x</span>"
        );
        assert_eq!(
            md_plus("<curly-underline>x</curly-underline>"),
            "<span style=\"text-decoration: underline; \
             text-decoration-style: wavy\">x</span>"
        );
    }

    #[test]
    fn markdown_plus_uses_no_javascript() {
        let out = md_plus("<red>x</red> <bg-blue>y</bg-blue> <u>z</u>");
        assert!(!out.contains("javascript:"), "got: {out}");
        assert!(!out.contains("<script"), "got: {out}");
        assert!(!out.contains("onclick"), "got: {out}");
    }

    #[test]
    fn markdown_plus_combines_color_with_semantic_bold() {
        assert_eq!(
            md_plus("<red><b>x</b></red>"),
            "<span style=\"color: rgb(128, 0, 0)\">**x**</span>"
        );
    }
}
