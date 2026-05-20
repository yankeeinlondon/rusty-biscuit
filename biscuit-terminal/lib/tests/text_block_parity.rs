//! Tree-path tests for the `TextBlock` component's IR migration.
//!
//! Coverage splits into:
//!
//! 1. **Structural tests** — projection shape and tree validation.
//! 2. **Tree SGR coverage** — italic, `FontWeight`, layout, foreground/
//!    background color, underline, strikethrough, and blink all surface in
//!    the rendered SGR via the canonical tree path.
//! 3. **Markdown / Browser coverage** — Markdown ignores `Style` by contract;
//!    the browser renderer lowers `Style` to semantic wrappers or inline CSS
//!    per RT-TEXTBLOCK-001.

mod parity_helpers;

use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::text_block::TextBlock;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::utils::color::{BasicColor, Color, RgbColor};
use biscuit_terminal::utils::styling::UnderliningRequest;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::{NodeKind, RenderStrictness, TreeRenderable, ValidationMode, validate};

use parity_helpers::{PARITY_WIDTHS, assert_contains_tokens, strip_ansi, test_terminal};

/// Helper: build an `RgbColor`-bearing `Color` with a basic-color fallback.
fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(RgbColor::new(r, g, b, BasicColor::White))
}

fn render_tree(node: &renderable::tree::RenderNode, width: u32) -> String {
    let term = test_terminal(width);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    render_terminal_node(node, &opts)
        .expect("tree render should succeed")
        .output
}

// ---------------------------------------------------------------------------
// Structural tests (spec §"Tree structure tests": #18-#22)
// ---------------------------------------------------------------------------

#[test]
fn plain_text_projection_is_paragraph_with_one_text_child() {
    let block = TextBlock::new("Hello");
    let node = <TextBlock as TreeRenderable>::render_tree(&block);
    match &node.kind {
        NodeKind::Paragraph { children } => {
            assert_eq!(children.len(), 1, "exactly one text child");
            assert!(
                matches!(&children[0].kind, NodeKind::Text { value } if value == "Hello"),
                "child is a Text node with the content: {:?}",
                children[0].kind
            );
        }
        other => panic!("expected NodeKind::Paragraph, got {other:?}"),
    }
}

#[test]
fn style_on_node_carries_emphasis_color_and_background() {
    let mut block = TextBlock::new("X");
    block
        .using_bold_text()
        .with_foreground_color(Color::BasicColor(BasicColor::Red))
        .with_background_color(Color::BasicColor(BasicColor::Blue));
    let node = <TextBlock as TreeRenderable>::render_tree(&block);
    let style = node.attrs.style().expect("style attr present");
    assert!(style.emphasis.bold, "bold flag carried in emphasis");
    assert!(style.color.is_some(), "foreground color in style");
    assert!(style.background.is_some(), "background color in style");
}

#[test]
fn non_default_layout_is_recorded_on_node() {
    use biscuit_terminal::utils::layout::{Length, TargetValue};
    let mut block = TextBlock::new("X");
    block.layout_mut().margin.left = TargetValue::universal(Length::ch(4));
    let node = <TextBlock as TreeRenderable>::render_tree(&block);
    assert!(node.attrs.layout().is_some(), "layout attr present");
}

#[test]
fn default_layout_is_not_recorded_on_node() {
    let block = TextBlock::new("X");
    let node = <TextBlock as TreeRenderable>::render_tree(&block);
    assert!(
        node.attrs.layout().is_none(),
        "default layout must not seed a layout attr"
    );
}

#[test]
fn underline_color_is_dropped_from_projection() {
    // `UnderliningRequest::*(Some(color))` maps shape only — the underline
    // color is intentionally dropped. The component should not invent an ad
    // hoc data hint to preserve it; that is a future renderer-wide feature.
    //
    // Parameterised over every `UnderliningRequest` variant that carries a
    // color so the drop policy is pinned for the whole enum, not just one
    // representative variant.
    use renderable::style::UnderlineStyle;

    fn case(label: &str, req: UnderliningRequest, expected_shape: UnderlineStyle) {
        let mut block = TextBlock::new("X");
        block.with_underline(req);
        let node = <TextBlock as TreeRenderable>::render_tree(&block);
        let style = node.attrs.style().expect("style attr present");
        assert_eq!(
            style.emphasis.underline,
            Some(expected_shape),
            "{label}: underline shape preserved"
        );

        // The component must not invent an ad-hoc data hint to smuggle the
        // dropped underline color through `NodeAttrs`. Serialize the attrs and
        // assert no key suggesting "underline color" survives. The data-hint
        // surface uses kebab-case keys like `data-underline-color`.
        let attrs_json =
            serde_json::to_value(&node.attrs).expect("NodeAttrs serializes to JSON");
        let attrs_text = attrs_json.to_string().to_ascii_lowercase();
        assert!(
            !attrs_text.contains("underline-color")
                && !attrs_text.contains("underline_color"),
            "{label}: no underline-color hint should appear in NodeAttrs: {attrs_text}"
        );
    }

    let red = || Color::BasicColor(BasicColor::Red);
    case(
        "Straight",
        UnderliningRequest::Straight(Some(red())),
        UnderlineStyle::Straight,
    );
    case(
        "Double",
        UnderliningRequest::Double(Some(red())),
        UnderlineStyle::Double,
    );
    case(
        "Curly",
        UnderliningRequest::Curly(Some(red())),
        UnderlineStyle::Curly,
    );
    case(
        "Dotted",
        UnderliningRequest::Dotted(Some(red())),
        UnderlineStyle::Dotted,
    );
    case(
        "Dashed",
        UnderliningRequest::Dashed(Some(red())),
        UnderlineStyle::Dashed,
    );
}

#[test]
fn projected_tree_validates_with_no_errors() {
    let mut block = TextBlock::new("Body");
    block.using_bold_text();
    let node = <TextBlock as TreeRenderable>::render_tree(&block);
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "projected TextBlock tree should validate cleanly: {:?}",
        report.errors().collect::<Vec<_>>()
    );
}

#[test]
fn tree_renderable_and_render_tree_node_share_one_projection() {
    let mut block = TextBlock::new("X");
    block
        .using_bold_text()
        .with_foreground_color(Color::BasicColor(BasicColor::Red));
    let canonical = <TextBlock as TreeRenderable>::render_tree(&block);
    let compat = block.render_tree_node().expect("tree node");
    assert_eq!(
        serde_json::to_value(&canonical).unwrap(),
        serde_json::to_value(&compat).unwrap(),
        "canonical and compatibility projections must serialize identically"
    );
}

// ---------------------------------------------------------------------------
// Tree-path SGR coverage
//
// Every stored style field (italic, FontWeight, layout, fg/bg color,
// underline, strikethrough, blink) must surface through the canonical tree
// path. These tests pin tree-path behavior directly.
// ---------------------------------------------------------------------------

#[test]
fn plain_text_renders_through_tree() {
    let block = TextBlock::new("Hello World");
    let term = test_terminal(80);
    let tree = strip_ansi(&block.render(&term));
    assert!(tree.contains("Hello World"));
}

#[test]
fn bold_emits_bold_sgr_through_tree() {
    let mut block = TextBlock::new("Bold");
    block.using_bold_text();
    let term = test_terminal(80);
    assert!(block.render(&term).contains("\x1b[1m"), "tree bold SGR");
}

#[test]
fn dim_emits_dim_sgr_through_tree() {
    let mut block = TextBlock::new("Dim");
    block.using_dim_text();
    let term = test_terminal(80);
    assert!(block.render(&term).contains("\x1b[2m"), "tree dim SGR");
}

#[test]
fn italic_emits_italic_sgr_through_tree() {
    let mut block = TextBlock::new("It");
    block.using_italics();
    let term = test_terminal(80);
    assert!(block.render(&term).contains("\x1b[3m"), "tree italic SGR");
}

#[test]
fn bold_plus_italic_emits_both_sgrs_through_tree() {
    let mut block = TextBlock::new("Both");
    block.using_bold_text().using_italics();
    let term = test_terminal(80);
    let tree = block.render(&term);
    assert!(tree.contains("\x1b[1m") && tree.contains("\x1b[3m"));
}

#[test]
fn layout_left_margin_applied_through_tree() {
    use biscuit_terminal::utils::layout::{Length, TargetValue};
    let mut block = TextBlock::new("Hello");
    block.layout_mut().margin.left = TargetValue::universal(Length::ch(4));
    let term = test_terminal(80);
    let tree_plain = strip_ansi(&block.render(&term));
    assert!(
        tree_plain.starts_with("    "),
        "tree left margin: {tree_plain:?}"
    );
}

#[test]
fn empty_content_produces_empty_output_through_tree() {
    let block = TextBlock::new("");
    let term = test_terminal(80);
    let tree_plain = strip_ansi(&block.render(&term));
    assert!(tree_plain.trim().is_empty(), "tree: {tree_plain:?}");
}

#[test]
fn unicode_content_preserved_through_tree() {
    let block = TextBlock::new("héllo 世界 ✨");
    let term = test_terminal(80);
    assert!(strip_ansi(&block.render(&term)).contains("héllo 世界 ✨"));
}

#[test]
fn fg_basic_color_activates_through_tree() {
    let mut block = TextBlock::new("Red");
    block.with_foreground_color(Color::BasicColor(BasicColor::Red));
    let term = test_terminal(80);
    let tree = block.render(&term);
    // The tree path now emits a foreground color SGR. The exact code depends
    // on degradation; assert any 30/90-series open is present.
    assert!(
        tree.contains("\x1b[31m") || tree.contains("\x1b[38;"),
        "tree path emits a foreground color SGR: {tree:?}"
    );
}

#[test]
fn fg_rgb_color_uses_shared_degradation_path() {
    let mut block = TextBlock::new("Rgb");
    block.with_foreground_color(rgb(255, 128, 0));
    let term = test_terminal(80);
    let tree = block.render(&term);
    // The shared color path emits truecolor or 256-color SGR depending on
    // capability; the optimistic terminal advertises TrueColor.
    assert!(
        tree.contains("\x1b[38;2;255;128;0m") || tree.contains("\x1b[38;5;"),
        "tree RGB color SGR: {tree:?}"
    );
}

#[test]
fn bg_color_activates_through_tree() {
    let mut block = TextBlock::new("Bg");
    block.with_background_color(Color::BasicColor(BasicColor::Blue));
    let term = test_terminal(80);
    let tree = block.render(&term);
    assert!(
        tree.contains("\x1b[44m") || tree.contains("\x1b[48;"),
        "tree bg SGR: {tree:?}"
    );
}

#[test]
fn straight_underline_activates_through_tree() {
    let mut block = TextBlock::new("U");
    block.with_underline(UnderliningRequest::Straight(None));
    let term = test_terminal(80);
    let tree = block.render(&term);
    assert!(
        tree.contains("\x1b[4m") || tree.contains("\x1b[4:"),
        "tree underline SGR: {tree:?}"
    );
}

#[test]
fn curly_underline_lowers_to_an_underline_sgr() {
    let mut block = TextBlock::new("U");
    block.with_underline(UnderliningRequest::Curly(None));
    let term = test_terminal(80);
    let tree = block.render(&term);
    // The renderer may degrade curly to straight depending on capability,
    // but in either case some underline SGR must be present.
    assert!(
        tree.contains("\x1b[4m") || tree.contains("\x1b[4:"),
        "tree curly-or-degraded underline SGR: {tree:?}"
    );
}

#[test]
fn strikethrough_activates_through_tree() {
    let mut block = TextBlock::new("S");
    block.use_strikethrough_on_content();
    let term = test_terminal(80);
    let tree = block.render(&term);
    assert!(
        tree.contains("\x1b[9m"),
        "tree strikethrough SGR: {tree:?}"
    );
}

#[test]
fn blink_activates_through_tree() {
    let mut block = TextBlock::new("B");
    block.make_content_blink();
    let term = test_terminal(80);
    let tree = block.render(&term);
    assert!(tree.contains("\x1b[5m"), "tree blink SGR: {tree:?}");
}

#[test]
fn all_stored_styles_combined_emit_their_sgr_layers() {
    let mut block = TextBlock::new("All");
    block
        .using_bold_text()
        .using_italics()
        .use_strikethrough_on_content()
        .make_content_blink()
        .with_underline(UnderliningRequest::Straight(None))
        .with_foreground_color(Color::BasicColor(BasicColor::Red))
        .with_background_color(Color::BasicColor(BasicColor::Blue));
    let term = test_terminal(80);
    let tree = block.render(&term);

    // Each layer must surface in the rendered SGR.
    assert!(tree.contains("\x1b[1m"), "bold: {tree:?}");
    assert!(tree.contains("\x1b[3m"), "italic: {tree:?}");
    assert!(tree.contains("\x1b[9m"), "strikethrough: {tree:?}");
    assert!(tree.contains("\x1b[5m"), "blink: {tree:?}");
    assert!(
        tree.contains("\x1b[4m") || tree.contains("\x1b[4:"),
        "underline: {tree:?}"
    );
    assert!(
        tree.contains("\x1b[31m") || tree.contains("\x1b[38;"),
        "fg: {tree:?}"
    );
    assert!(
        tree.contains("\x1b[44m") || tree.contains("\x1b[48;"),
        "bg: {tree:?}"
    );
}

// ---------------------------------------------------------------------------
// Width matrix — visible content must survive at every parity width
// ---------------------------------------------------------------------------

#[test]
fn tree_output_renders_at_all_parity_widths() {
    for &width in PARITY_WIDTHS {
        let block = TextBlock::new("Content");
        let node = <TextBlock as TreeRenderable>::render_tree(&block);
        let out = render_tree(&node, width);
        assert_contains_tokens(&out, &["Content"]);
    }
}

// ---------------------------------------------------------------------------
// Markdown (spec §"Markdown Test Strategy")
//
// Markdown ignores Style by contract — every styled variant emits plain text.
// ---------------------------------------------------------------------------

#[test]
fn markdown_renders_plain_text_regardless_of_style() {
    let mut block = TextBlock::new("Hello");
    block
        .using_bold_text()
        .using_italics()
        .use_strikethrough_on_content()
        .with_foreground_color(Color::BasicColor(BasicColor::Red))
        .with_background_color(Color::BasicColor(BasicColor::Blue))
        .with_underline(UnderliningRequest::Straight(None));
    let md = block.render_markdown();
    assert_eq!(md.trim(), "Hello", "md ignores style: {md:?}");
}

#[test]
fn markdown_plus_renders_plain_text_regardless_of_style() {
    let mut block = TextBlock::new("Hello");
    block.using_bold_text();
    let md = block.render_markdown_plus();
    assert_eq!(md.trim(), "Hello", "md+ ignores style: {md:?}");
}

#[test]
fn markdown_empty_text_block_is_empty() {
    let block = TextBlock::new("");
    let md = block.render_markdown();
    assert!(md.trim().is_empty(), "empty: {md:?}");
}

#[test]
fn markdown_unicode_preserved() {
    let block = TextBlock::new("héllo 世界");
    let md = block.render_markdown();
    assert!(md.contains("héllo 世界"));
}

// ---------------------------------------------------------------------------
// Browser (spec §"Browser Test Strategy", post RT-TEXTBLOCK-001)
// ---------------------------------------------------------------------------

#[test]
fn browser_emits_paragraph_for_plain_text() {
    let block = TextBlock::new("Hello");
    let html = block.render_html_fragment().render();
    assert!(html.contains("<p"), "paragraph element: {html:?}");
    assert!(html.contains("Hello"), "content present: {html:?}");
}

#[test]
fn browser_bold_preserves_bold_semantics() {
    // Per RT-TEXTBLOCK-001: "Lower TextEmphasis.bold ... to semantic wrappers
    // (<strong>, <em>, <s>) or equivalent valid HTML that preserves nesting."
    // For block-level Paragraph nodes, the browser renderer lowers emphasis
    // to CSS on the element itself (`<p style="font-weight:bold">`); for
    // inline nodes it uses `<strong>`. Either is conformant — we just need
    // bold semantics to be preserved.
    let mut block = TextBlock::new("Hello");
    block.using_bold_text();
    let html = block.render_html_fragment().render();
    assert!(
        html.contains("<strong") || html.contains("font-weight:bold"),
        "bold semantics preserved: {html:?}"
    );
}

#[test]
fn browser_italic_preserves_italic_semantics() {
    let mut block = TextBlock::new("Hello");
    block.using_italics();
    let html = block.render_html_fragment().render();
    assert!(
        html.contains("<em") || html.contains("font-style:italic"),
        "italic semantics preserved: {html:?}"
    );
}

#[test]
fn browser_strikethrough_preserves_strike_semantics() {
    let mut block = TextBlock::new("Hello");
    block.use_strikethrough_on_content();
    let html = block.render_html_fragment().render();
    assert!(
        html.contains("<s>")
            || html.contains("<s ")
            || html.contains("line-through"),
        "strike semantics preserved: {html:?}"
    );
}

#[test]
fn browser_fg_color_lowers_to_css_color() {
    let mut block = TextBlock::new("Red");
    block.with_foreground_color(rgb(255, 0, 0));
    let html = block.render_html_fragment().render();
    assert!(html.contains("color:"), "css color: {html:?}");
}

#[test]
fn browser_bg_color_lowers_to_css_background_color() {
    let mut block = TextBlock::new("Bg");
    block.with_background_color(rgb(0, 0, 255));
    let html = block.render_html_fragment().render();
    assert!(
        html.contains("background-color:"),
        "css background-color: {html:?}"
    );
}

#[test]
fn browser_underline_lowers_to_text_decoration() {
    let mut block = TextBlock::new("U");
    block.with_underline(UnderliningRequest::Straight(None));
    let html = block.render_html_fragment().render();
    assert!(
        html.contains("text-decoration"),
        "css text-decoration: {html:?}"
    );
}

#[test]
fn browser_curly_underline_uses_wavy_decoration_style() {
    let mut block = TextBlock::new("U");
    block.with_underline(UnderliningRequest::Curly(None));
    let html = block.render_html_fragment().render();
    assert!(html.contains("wavy"), "wavy underline: {html:?}");
}

#[test]
fn browser_dim_lowers_to_opacity() {
    let mut block = TextBlock::new("D");
    block.using_dim_text();
    let html = block.render_html_fragment().render();
    assert!(html.contains("opacity"), "opacity for dim: {html:?}");
}

#[test]
fn browser_blink_lowers_to_text_decoration_blink() {
    let mut block = TextBlock::new("B");
    block.make_content_blink();
    let html = block.render_html_fragment().render();
    assert!(html.contains("blink"), "blink decoration: {html:?}");
}

#[test]
fn browser_style_plus_layout_share_one_style_attribute() {
    use biscuit_terminal::utils::layout::{Length, TargetValue};
    let mut block = TextBlock::new("X");
    block.with_foreground_color(rgb(255, 0, 0));
    block.layout_mut().margin.left = TargetValue::universal(Length::ch(2));
    let html = block.render_html_fragment().render();
    // Both color and margin-left should appear on the `<p>` element's
    // single style attribute.
    assert!(html.contains("color:"), "color in style: {html:?}");
    assert!(
        html.contains("margin-left"),
        "margin-left in style: {html:?}"
    );
}

#[test]
fn browser_empty_text_block_emits_empty_paragraph() {
    let block = TextBlock::new("");
    let html = block.render_html_fragment().render();
    // An empty paragraph is still a paragraph element.
    assert!(html.contains("<p"), "paragraph element: {html:?}");
}

#[test]
fn browser_escapes_html_sensitive_content() {
    let block = TextBlock::new("<script>alert('x')</script>");
    let html = block.render_html_fragment().render();
    assert!(
        !html.contains("<script>"),
        "raw script must not survive: {html:?}"
    );
    assert!(
        html.contains("&lt;script&gt;") || html.contains("&lt;"),
        "must escape '<': {html:?}"
    );
}

#[test]
fn browser_no_style_emits_plain_paragraph() {
    let block = TextBlock::new("Plain");
    let html = block.render_html_fragment().render();
    // Without any Style or Layout, the paragraph must not carry a style
    // attribute (RT-TEXTBLOCK-001: "Preserve current plain paragraph output
    // when no Style is present.").
    assert!(html.contains("<p"));
    // The paragraph open tag should be just `<p>` — no inline style.
    assert!(
        html.contains("<p>") || !html.contains("style="),
        "plain paragraph: {html:?}"
    );
}
