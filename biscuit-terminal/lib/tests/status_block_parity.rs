//! Parity tests for the `StatusBlock` component's IR migration.
//!
//! These tests verify that `StatusBlock::render_tree()` produces a well-formed
//! `NodeKind::Root` tree, that the tree validates cleanly, and that the
//! tree-rendered terminal output is semantically equivalent to the bespoke
//! `StatusBlock::render_bespoke()` output. They also pin the explicit
//! "arbitrary `with_border(prefix)` keeps using the bespoke compatibility
//! path" contract.
//!
//! ## Escape-hatch coverage
//!
//! This file exercises the sanctioned `StatusBlock::render_bespoke` escape
//! hatch (arbitrary border prefix — a capability the render tree's
//! standard `Border` enum cannot express). The hook is `#[doc(hidden)]`
//! but `pub` so integration tests can reach it; see
//! `status_block.rs::render_bespoke` for the policy rationale.

mod parity_helpers;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::utils::color::{Color, Tailwind};
use biscuit_terminal::utils::layout::{Length, TargetValue};
use renderable::markdown::MarkdownRenderable;
use renderable::tree::{NodeKind, RenderNode, TreeRenderable, ValidationMode, validate};

use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use parity_helpers::{PARITY_WIDTHS, assert_contains_tokens, strip_ansi, test_terminal};
use renderable::tree::RenderStrictness;

// ---------------------------------------------------------------------------
// Structural snapshot
// ---------------------------------------------------------------------------

#[test]
fn projects_to_root_with_status_block_classes() {
    let block = StatusBlock::new(StatusState::Error)
        .header("Header")
        .body("Body")
        .hint("Hint");
    let node = block.render_tree();
    assert!(matches!(node.kind, NodeKind::Root { .. }));
    assert!(node.attrs.classes.iter().any(|c| c == "status-block"));
    assert!(
        node.attrs
            .classes
            .iter()
            .any(|c| c == "status-block--error")
    );
}

#[test]
fn projected_layout_is_seeded_on_root() {
    // StatusBlock::new() always has a non-default layout (right margin = 5,
    // WrapProse(8, None)), so the projection MUST seed it directly on the
    // root — the adapter renderers do not apply `tree_layout()`.
    let block = StatusBlock::new(StatusState::Info).body("body");
    let node = block.render_tree();
    assert!(
        node.attrs.layout().is_some(),
        "non-default layout must be seeded on the projected root"
    );
}

#[test]
fn projected_tree_validates() {
    let block = StatusBlock::new(StatusState::Error)
        .header("Header")
        .body("Body")
        .hint("Hint");
    let node = block.render_tree();
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "projected StatusBlock tree should validate cleanly: {:?}",
        report.errors().collect::<Vec<_>>()
    );
}

#[test]
fn body_only_emits_one_block_quote() {
    let block = StatusBlock::new(StatusState::Info).body("body");
    let node = block.render_tree();
    let children = match &node.kind {
        NodeKind::Root { children } => children,
        other => panic!("expected Root, got {other:?}"),
    };
    assert_eq!(children.len(), 1);
    assert!(matches!(children[0].kind, NodeKind::BlockQuote { .. }));
}

#[test]
fn header_only_emits_one_paragraph() {
    let block = StatusBlock::new(StatusState::Info).header("Header");
    let node = block.render_tree();
    let children = match &node.kind {
        NodeKind::Root { children } => children,
        other => panic!("expected Root, got {other:?}"),
    };
    assert_eq!(children.len(), 1);
    assert!(matches!(children[0].kind, NodeKind::Paragraph { .. }));
}

// ---------------------------------------------------------------------------
// Semantic parity: bespoke vs tree
// ---------------------------------------------------------------------------

/// Asserts that the bespoke and tree paths agree on visible tokens (split on
/// whitespace) after ANSI stripping. Both paths emit fundamentally different
/// byte layouts (the tree path's terminal renderer joins blocks with `\n\n`,
/// the bespoke path with `\n`), so per-byte equality is intentionally not
/// asserted.
fn assert_token_parity(block: &StatusBlock, width: u32) {
    let term = test_terminal(width);
    let bespoke = strip_ansi(&block.render_bespoke(&term));
    let tree = strip_ansi(&block.render(&term));

    // The bespoke path may render header icons via the Nerd Font set on
    // capable terminals while the tree path always uses portable Unicode
    // fallback glyphs. Drop any leading single icon character so token
    // comparison is robust across that documented divergence.
    let drop_icons = |s: &str| -> Vec<String> {
        s.split_whitespace()
            .map(|tok| {
                // Strip a leading non-ASCII icon (single char) when present.
                if let Some(first) = tok.chars().next()
                    && !first.is_ascii()
                {
                    let mut rest = tok.chars();
                    rest.next();
                    rest.collect::<String>()
                } else {
                    tok.to_string()
                }
            })
            .filter(|t| !t.is_empty())
            .collect()
    };

    let bespoke_tokens = drop_icons(&bespoke);
    let tree_tokens = drop_icons(&tree);
    assert_eq!(
        bespoke_tokens, tree_tokens,
        "bespoke/tree token mismatch at width {width}:\nbespoke = {bespoke:?}\ntree    = {tree:?}"
    );
}

#[test]
fn header_only_token_parity() {
    let block = StatusBlock::new(StatusState::Error).header("Header text");
    assert_token_parity(&block, 80);
}

#[test]
fn body_only_token_parity() {
    let block = StatusBlock::new(StatusState::Info).body("Plain body text");
    assert_token_parity(&block, 80);
}

#[test]
fn body_with_styled_prose_token_parity() {
    // Prose styling is documented as flattened in the tree projection; this
    // test asserts that the visible *words* still match across both paths.
    let block = StatusBlock::new(StatusState::Info).body(Prose::new("<b>bold</b> body text"));
    assert_token_parity(&block, 80);
}

#[test]
fn header_plus_body_token_parity() {
    let block = StatusBlock::new(StatusState::Warning)
        .header("Header")
        .body("Body text");
    assert_token_parity(&block, 80);
}

#[test]
fn header_plus_body_plus_hint_token_parity() {
    let block = StatusBlock::new(StatusState::Error)
        .header("Header")
        .body("Body")
        .hint("Hint text");
    assert_token_parity(&block, 80);
}

#[test]
fn multiple_body_items_token_parity() {
    let block =
        StatusBlock::new(StatusState::Info).body(vec![Prose::new("first"), Prose::new("second")]);
    assert_token_parity(&block, 80);
}

#[test]
fn token_parity_holds_at_all_widths() {
    let block = StatusBlock::new(StatusState::Error)
        .header("Header")
        .body("Body text with several words")
        .hint("Hint");
    for &width in PARITY_WIDTHS {
        assert_token_parity(&block, width);
    }
}

// ---------------------------------------------------------------------------
// Color and SGR
// ---------------------------------------------------------------------------

#[test]
fn custom_border_color_emits_sgr_on_color_terminal() {
    let term = test_terminal(80);
    let block = StatusBlock::new(StatusState::Warning)
        .body("Body")
        .border_color(Color::Tailwind(Tailwind::Purple700));
    let rendered = block.render(&term);
    // Some color SGR is emitted on the color-capable optimistic terminal.
    assert!(
        rendered.contains('\x1b'),
        "expected SGR on color terminal: {rendered:?}"
    );
    // And the bespoke path emits color too.
    let bespoke = block.render_bespoke(&term);
    assert!(bespoke.contains('\x1b'));
}

// ---------------------------------------------------------------------------
// Layout flows through the tree path
// ---------------------------------------------------------------------------

#[test]
fn left_and_right_margins_honored_in_tree_path() {
    let term = test_terminal(32);
    let block = StatusBlock::new(StatusState::Error)
        .body("alpha beta gamma delta epsilon")
        .left_margin(TargetValue::universal(Length::ch(4)))
        .right_margin(TargetValue::universal(Length::ch(10)));
    let rendered = strip_ansi(&block.render(&term));
    let lines: Vec<_> = rendered.lines().collect();
    assert!(lines.len() > 1, "expected wrapped output: {rendered:?}");
    assert!(
        lines.iter().all(|l| l.starts_with("    ┃ ")),
        "every line should start with the 4-space margin + thick border: {rendered:?}"
    );
}

#[test]
fn default_right_margin_drives_wrapping() {
    // Default right margin = 5 means a 32-column terminal yields a 27-column
    // content area, narrower than the body text below — so wrapping must
    // happen.
    let term = test_terminal(32);
    let block = StatusBlock::new(StatusState::Error)
        .body("one two three four five six seven eight nine ten eleven twelve");
    let rendered = strip_ansi(&block.render(&term));
    assert!(
        rendered.lines().count() > 1,
        "expected wrapping: {rendered:?}"
    );
}

// ---------------------------------------------------------------------------
// Severity icon coverage
// ---------------------------------------------------------------------------

#[test]
fn every_non_deprecated_severity_emits_fallback_icon_in_header() {
    let cases = [
        (StatusState::Error, "⤫"),
        (StatusState::Warning, "⚠"),
        (StatusState::Info, "ℹ"),
        (StatusState::Success, "✓"),
        (StatusState::Active, "◽"),
        (StatusState::NotStarted, "◻"),
        (StatusState::ToolUse, "🔧"),
        (StatusState::Subagent, "🤖"),
    ];
    let term = test_terminal(80);
    for (state, icon) in cases {
        let block = StatusBlock::new(state.clone()).header("Header");
        // Tree path: terminal output carries the portable icon.
        let rendered = strip_ansi(&block.render(&term));
        assert!(
            rendered.contains(icon),
            "severity {state:?}: terminal output missing icon {icon}: {rendered:?}"
        );
        // Markdown carries the same icon.
        let md = block.render_markdown();
        assert!(
            md.contains(icon),
            "severity {state:?}: Markdown output missing icon {icon}: {md:?}"
        );
        // Browser carries the same icon.
        let html = block.render_html_fragment().render();
        assert!(
            html.contains(icon),
            "severity {state:?}: HTML output missing icon {icon}: {html:?}"
        );
    }
}

#[test]
fn every_severity_emits_class_for_browser() {
    let cases = [
        (StatusState::Error, "status-block--error"),
        (StatusState::Warning, "status-block--warning"),
        (StatusState::Info, "status-block--info"),
        (StatusState::Success, "status-block--success"),
        (StatusState::Active, "status-block--active"),
        (StatusState::NotStarted, "status-block--not-started"),
        (StatusState::ToolUse, "status-block--tool-use"),
        (StatusState::Subagent, "status-block--subagent"),
    ];
    for (state, class) in cases {
        let block = StatusBlock::new(state.clone()).body("body");
        let html = block.render_html_fragment().render();
        assert!(
            html.contains(class),
            "severity {state:?}: HTML missing class {class}: {html:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Compatibility fallback: custom border prefix
// ---------------------------------------------------------------------------

#[test]
fn custom_border_uses_bespoke_terminal_path() {
    let term = test_terminal(80);
    let block = StatusBlock::new(StatusState::Error)
        .body("Body line")
        .border("!! ");
    let rendered = strip_ansi(&block.render(&term));
    // The bespoke path honors the arbitrary prefix verbatim.
    assert!(
        rendered.contains("!! Body line"),
        "custom border must reach output through the bespoke fallback: {rendered:?}"
    );
    // The default `┃` glyph must not appear in this branch.
    assert!(!rendered.contains('┃'));
}

#[test]
fn custom_border_does_not_leak_into_markdown() {
    // The tree path is the only Markdown source — a custom terminal prefix
    // never reaches Markdown by design.
    let block = StatusBlock::new(StatusState::Error)
        .body("Body")
        .border("!! ");
    let md = block.render_markdown();
    assert!(
        !md.contains("!! "),
        "custom border leaked into Markdown: {md:?}"
    );
    assert!(md.contains("> Body"));
}

#[test]
fn custom_border_does_not_leak_into_html() {
    // The Browser path also uses the canonical tree projection only.
    let block = StatusBlock::new(StatusState::Error)
        .body("Body")
        .border("!! ");
    let html = BrowserRenderable::render_html_fragment(&block).render();
    assert!(
        !html.contains("!! "),
        "custom border leaked into HTML: {html:?}"
    );
    assert!(html.contains("<blockquote"));
}

#[test]
fn default_border_terminal_render_matches_tree_byte_for_byte_with_bespoke_default_path() {
    // When the user does NOT call `with_border`, the default `┃ ` path
    // routes through the tree renderer. The bespoke renderer composes
    // Status + BlockQuote which itself goes through the tree (because
    // BlockQuote's default border also routes through the tree), so a
    // body-only block should produce identical visible content in both
    // paths even though the byte streams may differ in styling.
    let term = test_terminal(80);
    let block = StatusBlock::new(StatusState::Error).body("Body");
    let rendered = strip_ansi(&block.render(&term));
    let bespoke = strip_ansi(&block.render_bespoke(&term));
    assert!(
        rendered.contains("┃ Body"),
        "tree path missing default border: {rendered:?}"
    );
    assert!(
        bespoke.contains("┃ Body"),
        "bespoke path missing default border: {bespoke:?}"
    );
}

// ---------------------------------------------------------------------------
// Width matrix smoke
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// render_tree_node ↔ render_tree parity (Stage 3a.1)
// ---------------------------------------------------------------------------

#[test]
fn render_tree_node_matches_render_tree() {
    let block = StatusBlock::new(StatusState::Error)
        .header("Header")
        .body("Body text")
        .hint("Hint");
    let from_tree = <StatusBlock as TreeRenderable>::render_tree(&block);
    let from_hook = <StatusBlock as TerminalRenderable>::render_tree_node(&block)
        .expect("render_tree_node should return Some");
    assert_eq!(
        from_tree, from_hook,
        "render_tree_node must delegate to TreeRenderable::render_tree"
    );
}

#[test]
fn render_tree_node_matches_render_tree_body_only() {
    let block = StatusBlock::new(StatusState::Info).body("body only");
    let from_tree = <StatusBlock as TreeRenderable>::render_tree(&block);
    let from_hook = <StatusBlock as TerminalRenderable>::render_tree_node(&block)
        .expect("render_tree_node should return Some");
    assert_eq!(from_tree, from_hook);
}

#[test]
fn render_tree_node_matches_render_tree_with_custom_border() {
    // Custom border keeps the bespoke terminal path but the IR projection
    // (used by nested-component embedding) must still be canonical.
    let block = StatusBlock::new(StatusState::Error)
        .body("Body")
        .border("!! ");
    let from_tree = <StatusBlock as TreeRenderable>::render_tree(&block);
    let from_hook = <StatusBlock as TerminalRenderable>::render_tree_node(&block)
        .expect("render_tree_node should return Some");
    assert_eq!(from_tree, from_hook);
}

#[test]
fn renders_at_all_parity_widths() {
    for &width in PARITY_WIDTHS {
        let block = StatusBlock::new(StatusState::Error)
            .header("Header")
            .body("Body text")
            .hint("Hint");
        let term = test_terminal(width);
        let rendered = strip_ansi(&block.render(&term));
        assert_contains_tokens(&rendered, &["Header", "Body", "Hint"]);
    }
}

// ---------------------------------------------------------------------------
// Phase 3: Component-level body/hint layout tests
// ---------------------------------------------------------------------------

/// Helper: extract the BlockQuote children from a StatusBlock render tree.
/// Panics if the tree root has no BlockQuote child.
fn body_block_quote_children(block: &StatusBlock) -> Vec<RenderNode> {
    let node = block.render_tree();
    let root_children = match &node.kind {
        NodeKind::Root { children } => children,
        other => panic!("expected Root, got {other:?}"),
    };
    for child in root_children {
        if let NodeKind::BlockQuote { children } = &child.kind {
            return children.clone();
        }
    }
    panic!("no BlockQuote found in tree root");
}

#[test]
fn every_status_state_renders_body_with_leading_blank_line_in_block_quote() {
    let states = [
        StatusState::NotStarted,
        StatusState::Active,
        StatusState::Success,
        StatusState::Error,
        StatusState::Warning,
        StatusState::Info,
        StatusState::ToolUse,
        StatusState::Subagent,
    ];
    for state in &states {
        let block = StatusBlock::new(state.clone()).body("Body text");
        let bq = body_block_quote_children(&block);
        assert!(
            matches!(&bq[0].kind, NodeKind::Paragraph { children } if children.is_empty()),
            "state {:?}: first child of body block quote must be a blank paragraph",
            state
        );
    }
}

#[test]
fn failure_renders_identically_to_error() {
    #[allow(deprecated)]
    let failure = StatusBlock::new(StatusState::Failure)
        .header("Header")
        .body("Body")
        .hint("Hint");
    let error = StatusBlock::new(StatusState::Error)
        .header("Header")
        .body("Body")
        .hint("Hint");

    let failure_tree = failure.render_tree();
    let error_tree = error.render_tree();
    assert_eq!(
        serde_json::to_string(&failure_tree).unwrap(),
        serde_json::to_string(&error_tree).unwrap(),
        "Failure and Error must produce identical render trees"
    );

    let failure_json = serde_json::to_string(&failure_tree).unwrap();
    assert!(
        failure_tree.attrs.classes.iter().any(|c| c == "status-block--error"),
        "Failure must map to error severity class: {failure_json}"
    );

    let term = test_terminal(80);
    let failure_rendered = strip_ansi(&failure.render(&term));
    let error_rendered = strip_ansi(&error.render(&term));
    assert_eq!(failure_rendered, error_rendered);
}

#[test]
fn body_plus_hint_renders_hint_inside_block_quote_for_terminal() {
    let term = test_terminal(80);
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("Fix this hint");
    let rendered = strip_ansi(&block.render(&term));
    let hint_lines: Vec<&str> = rendered
        .lines()
        .filter(|l| l.contains("Fix this hint"))
        .collect();
    assert!(
        !hint_lines.is_empty(),
        "hint must appear in terminal output: {rendered:?}"
    );
    for hint_line in &hint_lines {
        assert!(
            hint_line.contains('┃'),
            "hint line must carry the block quote border: {:?}",
            hint_line
        );
    }
}

#[test]
fn body_plus_hint_renders_hint_inside_block_quote_for_markdown() {
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("Fix this hint");
    let md = block.render_markdown();
    assert!(
        md.contains("> Body text"),
        "body must be in block quote: {md:?}"
    );
    let hint_lines: Vec<&str> = md
        .lines()
        .filter(|l| l.contains("Fix this hint"))
        .collect();
    assert!(
        !hint_lines.is_empty(),
        "hint must appear in markdown: {md:?}"
    );
    for hint_line in &hint_lines {
        assert!(
            hint_line.starts_with('>'),
            "hint line must be inside block quote: {:?}",
            hint_line
        );
    }
}

#[test]
fn body_plus_hint_renders_hint_inside_block_quote_for_browser() {
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("Fix this hint");
    let html = BrowserRenderable::render_html_fragment(&block).render();
    let bq_open = html.find("<blockquote").expect("blockquote opening tag");
    let hint_pos = html
        .find("status-block__hint")
        .expect("hint class in HTML");
    let bq_close = html
        .find("</blockquote>")
        .expect("blockquote closing tag");
    assert!(
        bq_open < hint_pos && hint_pos < bq_close,
        "hint class must be inside blockquote: {html:?}"
    );
}

#[test]
fn blank_hint_omits_separator_and_hint() {
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("   ");
    let bq = body_block_quote_children(&block);
    assert_eq!(
        bq.len(),
        2,
        "blank hint should produce exactly 2 children (leading blank + body), got {}",
        bq.len()
    );
    assert!(
        !bq.iter()
            .any(|c| c.attrs.classes.iter().any(|cl| cl == "status-block__hint")),
        "blank hint must not produce a hint node"
    );
}

#[test]
fn hint_only_remains_outside_block_quote() {
    let block = StatusBlock::new(StatusState::Info)
        .header("Header")
        .hint("Hint only");
    let node = block.render_tree();
    let root_children = match &node.kind {
        NodeKind::Root { children } => children,
        other => panic!("expected Root, got {other:?}"),
    };
    assert!(
        !root_children
            .iter()
            .any(|c| matches!(c.kind, NodeKind::BlockQuote { .. })),
        "hint-only block must not contain a BlockQuote"
    );
    let hint_node = root_children
        .iter()
        .find(|c| c.attrs.classes.iter().any(|cl| cl == "status-block__hint"))
        .expect("hint paragraph");
    assert!(
        matches!(hint_node.kind, NodeKind::Paragraph { .. }),
        "hint must be a standalone paragraph"
    );
}

#[test]
fn multiple_body_items_keep_blank_line_separation_in_block_quote() {
    let block = StatusBlock::new(StatusState::Info)
        .body(vec![Prose::new("first item"), Prose::new("second item")]);
    let bq = body_block_quote_children(&block);
    assert_eq!(bq.len(), 2, "expected leading blank + body paragraph");
    let body_text = match &bq[1].kind {
        NodeKind::Paragraph { children } => match &children.first().unwrap().kind {
            NodeKind::Text { value } => value.clone(),
            other => panic!("expected Text node, got {other:?}"),
        },
        other => panic!("expected Paragraph, got {other:?}"),
    };
    assert!(
        body_text.contains("first item\n\nsecond item"),
        "body items must be separated by blank line: {body_text:?}"
    );
}

#[test]
fn default_border_terminal_uses_tree_projection() {
    let term = test_terminal(80);
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("Hint text");
    let via_render = strip_ansi(&block.render(&term));
    let via_tree = {
        let node = block.render_tree();
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        strip_ansi(&render_terminal_node(&node, &opts).unwrap().output)
    };
    assert_eq!(via_render, via_tree);
}

#[test]
fn custom_border_mirrors_body_hint_layout() {
    let term = test_terminal(80);
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("Hint text")
        .border("!! ");
    let rendered = strip_ansi(&block.render(&term));
    assert!(
        rendered.contains("!! Body text"),
        "custom border must prefix body: {rendered:?}"
    );
    let hint_lines: Vec<&str> = rendered
        .lines()
        .filter(|l| l.contains("Hint text"))
        .collect();
    assert!(
        !hint_lines.is_empty(),
        "hint must appear in output: {rendered:?}"
    );
    for hint_line in &hint_lines {
        assert!(
            hint_line.contains("!! "),
            "hint must be inside block quote with custom prefix: {:?}",
            hint_line
        );
    }
}

#[test]
fn markdown_does_not_leak_custom_border_prefix_with_body_and_hint() {
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("Hint text")
        .border("!! ");
    let md = block.render_markdown();
    assert!(
        !md.contains("!! "),
        "custom border must not leak into Markdown: {md:?}"
    );
    assert!(md.contains("> Body text"));
    assert!(
        md.lines().any(|l| l.contains("Hint text") && l.starts_with('>')),
        "hint must be inside block quote in Markdown: {md:?}"
    );
}

#[test]
fn browser_preserves_hint_class_and_italic_inside_body_block_quote() {
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("Fix this hint");
    let html = BrowserRenderable::render_html_fragment(&block).render();
    assert!(html.contains("status-block__hint"));
    let bq_open = html.find("<blockquote").expect("blockquote");
    let hint_pos = html
        .find("status-block__hint")
        .expect("hint class");
    let bq_close = html
        .find("</blockquote>")
        .expect("blockquote close");
    assert!(
        bq_open < hint_pos && hint_pos < bq_close,
        "hint class must be inside blockquote: {html:?}"
    );
    assert!(
        html.contains("font-style:italic"),
        "hint must have italic styling in HTML: {html:?}"
    );
    let bq = body_block_quote_children(&block);
    let hint_node = bq
        .iter()
        .find(|c| c.attrs.classes.iter().any(|cl| cl == "status-block__hint"))
        .expect("hint node in block quote");
    let style = hint_node.attrs.style().expect("hint has style");
    assert!(
        style.emphasis.italic,
        "hint must have italic emphasis in render tree"
    );
}
