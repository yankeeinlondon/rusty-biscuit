//! Integration tests for disclosure-block rendering across all targets.
//!
//! These tests exercise the full render pipeline from Darkmatter Markdown
//! source through the Terminal, Markdown, MarkdownPlus, Browser, and JSON
//! targets, including nested disclosures.

use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::{ColorDepth, HtmlOptions, TerminalOptions};
use darkmatter::markdown::render_tree::{render_tree_markdown, render_tree_markdown_dialect};
use darkmatter::style::{
    DisclosureStyleOverrides, apply_color_style, apply_disclosure_style, from_frontmatter,
};
use renderable::tree::MarkdownDialect;

/// Truecolor SGR foreground for Tailwind `red-500` (`#fb2c36`).
const RED_500_FG: &str = "\u{1b}[38;2;251;44;54m";

/// Returns a disclosure fixture with phrasing summary and a single-paragraph body.
fn fixture() -> Markdown {
    Markdown::new(
        "::disclosure\nLicense *Agreement*\n::details\nKeep your **hands** off.\n::end-disclosure\n".to_string(),
    )
}

/// Returns a disclosure fixture with a nested disclosure in the body.
fn nested_fixture() -> Markdown {
    Markdown::new(
        "::disclosure\nOuter\n::details\nOuter body.\n\n::disclosure\nInner\n::details\nInner body.\n::end-disclosure\n\n::end-disclosure\n".to_string(),
    )
}

#[test]
fn fold_produces_disclosure_node() {
    let md = fixture();
    let doc = md.as_document().expect("fold must succeed");

    let children = doc.root.children();
    assert_eq!(children.len(), 1, "document should contain one disclosure");

    let NodeKind::Disclosure { summary, children, .. } = &children[0].kind else {
        panic!("expected Disclosure node, got {:?}", children[0].kind);
    };

    assert!(!summary.is_empty(), "summary must not be empty");
    assert!(!children.is_empty(), "body must not be empty");

    let mut summary_text = String::new();
    collect_text(summary, &mut summary_text);
    assert!(summary_text.contains("License"), "summary must contain 'License'");
    assert!(summary_text.contains("Agreement"), "summary must contain 'Agreement'");

    let mut body_text = String::new();
    collect_text(children, &mut body_text);
    assert!(body_text.contains("Keep your"), "body must contain 'Keep your'");
    assert!(body_text.contains("hands"), "body must contain 'hands'");
}

#[test]
fn markdown_target_emits_dsl_verbatim() {
    let md = fixture();
    let rendered = render_tree_markdown(&md).expect("markdown render must succeed");

    let output = rendered.output;
    assert!(output.contains("::disclosure"), "must contain opener: {output}");
    assert!(output.contains("::details"), "must contain separator: {output}");
    assert!(output.contains("::end-disclosure"), "must contain closer: {output}");
    assert!(output.contains("License _Agreement_"), "must preserve summary markdown: {output}");
    assert!(output.contains("Keep your **hands** off."), "must preserve body markdown: {output}");
    assert!(
        !output.contains("<details>"),
        "markdown target must not emit HTML: {output}"
    );
}

#[test]
fn markdown_plus_target_wraps_with_details_summary() {
    let md = fixture();
    let rendered = render_tree_markdown_dialect(&md, MarkdownDialect::MarkdownPlus)
        .expect("markdown-plus render must succeed");

    let output = rendered.output;
    assert!(output.contains("<details>"), "must open details: {output}");
    assert!(output.contains("<summary>"), "must open summary: {output}");
    assert!(output.contains("</summary>"), "must close summary: {output}");
    assert!(output.contains("</details>"), "must close details: {output}");
    assert!(
        output.contains("License _Agreement_"),
        "summary must render inline markdown: {output}"
    );
    assert!(
        output.contains("Keep your **hands** off."),
        "body must render block markdown: {output}"
    );
}

#[test]
fn browser_target_uses_native_details_summary() {
    let md = fixture();
    let html = md.as_html(HtmlOptions::default()).expect("html render must succeed");

    assert!(html.contains("<details>"), "must open details: {html}");
    assert!(html.contains("<summary>"), "must open summary: {html}");
    assert!(html.contains("</summary>"), "must close summary: {html}");
    assert!(html.contains("</details>"), "must close details: {html}");
    assert!(
        html.contains("License <em>Agreement</em>"),
        "summary must render inline HTML: {html}"
    );
    assert!(
        html.contains("Keep your <strong>hands</strong> off."),
        "body must render block HTML: {html}"
    );
    assert!(!html.contains("<script"), "must not include JavaScript: {html}");
}

#[test]
fn terminal_target_renders_summary_and_dim_italic_body() {
    let md = fixture();
    let mut options = TerminalOptions::default();
    options.color_depth = Some(ColorDepth::TrueColor);
    let rendered = md.as_terminal(options).expect("terminal render must succeed");

    assert!(rendered.contains("License"), "must contain summary text: {rendered}");
    assert!(rendered.contains("Agreement"), "must contain summary text: {rendered}");
    assert!(rendered.contains("Keep your"), "must contain body text: {rendered}");
    assert!(rendered.contains("hands"), "must contain body text: {rendered}");

    // Dim and italic SGR escapes should appear somewhere in the body region.
    assert!(rendered.contains("\u{001b}[2m"), "body must contain dim escape: {rendered}");
    assert!(rendered.contains("\u{001b}[3m"), "body must contain italic escape: {rendered}");
}

/// Inline opener style tokens on the same line as `::disclosure` must be parsed
/// off the summary and captured on the node — they must not leak into the
/// rendered summary text. Regression for the `split_disclosure_directives`
/// boundary that previously stranded the tokens as summary content.
#[test]
fn inline_opener_style_is_parsed_off_the_summary() {
    let md = Markdown::new(
        "::disclosure max-width=30ch color=red-500 alignment=center License Terms\n::details\nBody.\n::end-disclosure\n".to_string(),
    );
    let doc = md.as_document().expect("fold must succeed");
    let NodeKind::Disclosure { summary, style, .. } = &doc.root.children()[0].kind else {
        panic!("expected Disclosure node");
    };

    let style = style.as_ref().expect("inline opener style must be captured");
    assert!(style.layout.is_some(), "max-width/alignment must lower to layout: {style:?}");
    assert!(style.color.is_some(), "color must be captured: {style:?}");

    let mut summary_text = String::new();
    collect_text(summary, &mut summary_text);
    assert_eq!(
        summary_text.trim(),
        "License Terms",
        "style tokens must not leak into summary: {summary_text:?}"
    );
}

/// Terminal rendering must honor inline opener `alignment` and `max-width`:
/// centered alignment indents the rendered lines, and `max-width` wraps the
/// body into multiple quoted lines. Inline color capture is asserted by
/// `inline_opener_style_is_parsed_off_the_summary`.
#[test]
fn terminal_target_honors_inline_opener_style() {
    let md = Markdown::new(
        "::disclosure color=red-500 alignment=center max-width=24ch A Title\n::details\nThis disclosed body is comfortably longer than twenty-four columns wide.\n::end-disclosure\n".to_string(),
    );
    let mut term = Terminal::new_optimistic(80);
    term.color_depth = biscuit_terminal::discovery::detection::ColorDepth::TrueColor;
    let rendered = DarkmatterPage::new(&term).render(&md).expect("render must succeed");

    let title_line = rendered
        .lines()
        .find(|l| l.contains("A Title"))
        .expect("summary line present");
    assert!(
        title_line.starts_with(' '),
        "centered summary must be indented: {title_line:?}"
    );

    let quoted_body_lines = rendered.lines().filter(|l| l.contains('│')).count();
    assert!(
        quoted_body_lines >= 2,
        "max-width must wrap the body into multiple quoted lines: {rendered:?}"
    );
}

/// Terminal rendering must honor `style.disclosure.*` frontmatter applied
/// through the same `apply_disclosure_style` + `apply_color_style` sequence the
/// CLI runs: the body wraps to the frontmatter `max-width` and the summary
/// carries the frontmatter `color`.
#[test]
fn terminal_target_honors_frontmatter_disclosure_style() {
    let md = Markdown::try_from_content(
        "---\nstyle:\n  disclosure:\n    color: red-500\n    max-width: 24ch\n    alignment: center\n---\n::disclosure\nFrontmatter Summary\n::details\nThis disclosed body is comfortably longer than twenty-four columns wide.\n::end-disclosure\n",
    )
    .expect("frontmatter parse must succeed");

    let term = Terminal::new_optimistic(80);
    let (style, _warnings) = from_frontmatter(md.frontmatter()).expect("style parse must succeed");
    let page = DarkmatterPage::new(&term);
    let page = apply_disclosure_style(page, &style, DisclosureStyleOverrides::default())
        .expect("disclosure style apply must succeed");
    let page = apply_color_style(page, &style).expect("color style apply must succeed");
    let rendered = page.render(&md).expect("render must succeed");

    assert!(rendered.contains(RED_500_FG), "summary must carry red-500 fg: {rendered:?}");

    let quoted_body_lines = rendered.lines().filter(|l| l.contains('│')).count();
    assert!(
        quoted_body_lines >= 2,
        "max-width must wrap the body into multiple quoted lines: {rendered:?}"
    );
}

#[test]
fn json_target_exports_native_disclosure_node() {
    let md = fixture();
    let doc = md.as_document().expect("fold must succeed");
    let json = serde_json::to_string_pretty(&doc).expect("json serialize must succeed");

    assert!(
        json.contains("\"disclosure\"") || json.contains("\"Disclosure\""),
        "json must contain disclosure kind: {json}"
    );
    assert!(json.contains("summary"), "json must contain summary field: {json}");
    assert!(json.contains("children"), "json must contain children field: {json}");
    assert!(json.contains("License"), "json must preserve summary text: {json}");
    assert!(json.contains("Keep your"), "json must preserve body text: {json}");
}

#[test]
fn markdown_target_renders_nested_disclosures() {
    let md = nested_fixture();
    let rendered = render_tree_markdown(&md).expect("markdown render must succeed");
    let output = rendered.output;

    // Two openers, two separators, two closers.
    assert_eq!(output.matches("::disclosure").count(), 2, "expected two disclosures: {output}");
    assert_eq!(output.matches("::details").count(), 2, "expected two details: {output}");
    assert_eq!(
        output.matches("::end-disclosure").count(),
        2,
        "expected two closers: {output}"
    );
    assert!(output.contains("Outer body."), "must contain outer body: {output}");
    assert!(output.contains("Inner body."), "must contain inner body: {output}");
}

#[test]
fn markdown_plus_target_renders_nested_disclosures() {
    let md = nested_fixture();
    let rendered = render_tree_markdown_dialect(&md, MarkdownDialect::MarkdownPlus)
        .expect("markdown-plus render must succeed");
    let output = rendered.output;

    assert_eq!(output.matches("<details>").count(), 2, "expected two details: {output}");
    assert_eq!(output.matches("<summary>").count(), 2, "expected two summaries: {output}");
    assert!(output.contains("Outer body."), "must contain outer body: {output}");
    assert!(output.contains("Inner body."), "must contain inner body: {output}");
}

#[test]
fn browser_target_renders_nested_disclosures() {
    let md = nested_fixture();
    let html = md.as_html(HtmlOptions::default()).expect("html render must succeed");

    assert_eq!(html.matches("<details>").count(), 2, "expected two details: {html}");
    assert_eq!(html.matches("<summary>").count(), 2, "expected two summaries: {html}");
    assert!(html.contains("Outer body."), "must contain outer body: {html}");
    assert!(html.contains("Inner body."), "must contain inner body: {html}");
}

#[test]
fn terminal_target_renders_nested_disclosures() {
    let md = nested_fixture();
    let rendered = md.as_terminal(TerminalOptions::default()).expect("terminal render must succeed");

    assert!(rendered.contains("Outer"), "must contain outer summary: {rendered}");
    assert!(rendered.contains("Inner"), "must contain inner summary: {rendered}");
    assert!(rendered.contains("Outer body."), "must contain outer body: {rendered}");
    assert!(rendered.contains("Inner body."), "must contain inner body: {rendered}");
}

#[test]
fn json_target_exports_nested_disclosures() {
    let md = nested_fixture();
    let doc = md.as_document().expect("fold must succeed");
    let json = serde_json::to_string_pretty(&doc).expect("json serialize must succeed");

    assert_eq!(
        json.matches("\"disclosure\"").count() + json.matches("\"Disclosure\"").count(),
        2,
        "expected two disclosure nodes: {json}"
    );
    assert!(json.contains("Outer body."), "must contain outer body: {json}");
    assert!(json.contains("Inner body."), "must contain inner body: {json}");
}

/// Recursively collects plain text from a slice of nodes.
fn collect_text(nodes: &[renderable::tree::RenderNode], out: &mut String) {
    for node in nodes {
        match &node.kind {
            renderable::tree::NodeKind::Text { value }
            | renderable::tree::NodeKind::InlineCode { value } => out.push_str(value),
            _ => collect_text(node.children(), out),
        }
    }
}

use renderable::tree::NodeKind;
