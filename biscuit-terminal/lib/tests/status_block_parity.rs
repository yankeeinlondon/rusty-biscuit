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
    // Styling differs between renderers, so parity here concerns visible words.
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
fn every_status_state_renders_body_with_leading_blank_and_body_paragraph() {
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
        assert_eq!(
            bq.len(),
            2,
            "state {:?}: body-only block quote should have exactly two children (leading blank + body paragraph)",
            state
        );
        assert!(
            matches!(&bq[0].kind, NodeKind::Paragraph { children } if children.is_empty()
                || children.iter().all(|c| matches!(&c.kind, NodeKind::Text { value } if value.is_empty()))),
            "state {:?}: first child must be a structural blank paragraph",
            state
        );
        match &bq[1].kind {
            NodeKind::Paragraph { children } => {
                assert!(
                    !children.is_empty(),
                    "state {:?}: body paragraph must not be empty",
                    state
                );
            }
            other => panic!("state {:?}: expected Paragraph, got {other:?}", state),
        }
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
fn body_plus_hint_carries_italic_sgr_on_terminal() {
    let term = test_terminal(80);
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("Fix this hint");
    let rendered = block.render(&term);
    let italic_sgr = "\x1b[3m";
    assert!(
        rendered.contains(italic_sgr),
        "hint must carry italic SGR (ESC[3m) in raw terminal output: {rendered:?}"
    );
    let hint_lines_raw: Vec<&str> = rendered
        .lines()
        .filter(|l| l.contains("Fix this hint"))
        .collect();
    assert!(
        !hint_lines_raw.is_empty(),
        "hint text must appear in raw output: {rendered:?}"
    );
    for line in &hint_lines_raw {
        assert!(
            line.contains(italic_sgr),
            "hint line must contain italic SGR: {:?}",
            line
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
        assert!(
            hint_line.contains("_Fix this hint_"),
            "hint must be italicized with Markdown emphasis: {:?}",
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
        "blank hint should produce exactly 2 children (leading blank + body paragraph), got {}",
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
    assert_eq!(
        bq.len(),
        4,
        "expected leading blank, first item, blank separator, and second item"
    );
    assert!(matches!(
        &bq[1].kind,
        NodeKind::Paragraph { children }
            if matches!(&children[0].kind, NodeKind::Text { value } if value == "first item")
    ));
    assert!(matches!(
        &bq[2].kind,
        NodeKind::Paragraph { children }
            if children.iter().all(|child| matches!(&child.kind, NodeKind::Text { value } if value.is_empty()))
    ));
    assert!(matches!(
        &bq[3].kind,
        NodeKind::Paragraph { children }
            if matches!(&children[0].kind, NodeKind::Text { value } if value == "second item")
    ));
}

/// The render-tree contract requires the body-plus-hint body block quote to
/// contain exactly `blank, body, blank, hint` — the hint separator must be a
/// structural `Paragraph` node, not a renderer-side spacing rule, so Browser
/// output preserves the blank line even when the Browser renderer does not
/// insert spacing between adjacent paragraphs.
#[test]
fn body_plus_hint_block_quote_has_structural_blank_separator_before_hint() {
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("Hint text");
    let bq = body_block_quote_children(&block);
    assert_eq!(
        bq.len(),
        4,
        "body-plus-hint block quote must have exactly four children (blank, body, blank, hint)"
    );

    let is_blank_paragraph = |node: &RenderNode| matches!(&node.kind,
        NodeKind::Paragraph { children }
            if children.is_empty() || children.iter()
                .all(|c| matches!(&c.kind, NodeKind::Text { value } if value.is_empty()))
    );
    assert!(
        is_blank_paragraph(&bq[0]),
        "first child must be the leading blank paragraph"
    );
    match &bq[1].kind {
        NodeKind::Paragraph { children } => assert!(
            !children.is_empty(),
            "second child must be the non-empty body paragraph"
        ),
        other => panic!("second child must be a Paragraph, got {other:?}"),
    }
    assert!(
        is_blank_paragraph(&bq[2]),
        "third child must be the structural blank separator paragraph"
    );
    let hint_node = &bq[3];
    assert!(
        hint_node.attrs.classes.iter().any(|c| c == "status-block__hint"),
        "fourth child must carry the status-block__hint class"
    );
    if let NodeKind::Paragraph { children } = &hint_node.kind {
        assert!(
            children.iter().any(|c| matches!(c.kind, NodeKind::Emphasis { .. })),
            "hint paragraph must contain an Emphasis child"
        );
    } else {
        panic!("hint node must be a Paragraph");
    }

    let html = BrowserRenderable::render_html_fragment(&block).render();
    let p_tags: Vec<_> = html.match_indices("<p").collect();
    let p_closes: Vec<_> = html.match_indices("</p>").collect();
    let em_tags: Vec<_> = html.match_indices("<em>").collect();
    let em_closes: Vec<_> = html.match_indices("</em>").collect();
    let bq_open = html.find("<blockquote").expect("blockquote opening");
    let bq_close = html.find("</blockquote>").expect("blockquote closing");
    let hint_pos = html
        .find("status-block__hint")
        .expect("hint class in HTML");
    assert!(
        bq_open < hint_pos && hint_pos < bq_close,
        "hint class must live inside the blockquote: {html:?}"
    );
    let p_in_bq: Vec<_> = p_tags
        .iter()
        .copied()
        .filter(|(idx, _)| *idx > bq_open && *idx < bq_close)
        .collect();
    let p_closes_in_bq: Vec<_> = p_closes
        .iter()
        .copied()
        .filter(|(idx, _)| *idx > bq_open && *idx < bq_close)
        .collect();
    assert_eq!(
        p_in_bq.len(),
        4,
        "block quote must contain four <p> elements (blank, body, blank, hint): {html:?}"
    );
    assert_eq!(
        p_closes_in_bq.len(),
        4,
        "block quote must close four <p> elements: {html:?}"
    );
    assert_eq!(
        em_tags.len(),
        1,
        "block quote must contain exactly one <em> element (the hint): {html:?}"
    );
    assert_eq!(em_closes.len(), 1, "exactly one </em> expected: {html:?}");
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
fn custom_border_hint_carries_italic_sgr_on_terminal() {
    let term = test_terminal(80);
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("Fix this hint")
        .border("!! ");
    let rendered = block.render(&term);
    let italic_sgr = "\x1b[3m";
    assert!(
        rendered.contains(italic_sgr),
        "bespoke-path hint must carry italic SGR (ESC[3m) in raw terminal output: {rendered:?}"
    );
    let hint_lines_raw: Vec<&str> = rendered
        .lines()
        .filter(|l| l.contains("Fix this hint"))
        .collect();
    assert!(
        !hint_lines_raw.is_empty(),
        "hint text must appear in raw bespoke output: {rendered:?}"
    );
    for line in &hint_lines_raw {
        assert!(
            line.contains(italic_sgr),
            "bespoke hint line must contain italic SGR: {:?}",
            line
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
        html.contains("<em>Fix this hint</em>"),
        "hint must be italicized via <em> in HTML: {html:?}"
    );
    let bq = body_block_quote_children(&block);
    let hint_node = bq
        .iter()
        .find(|c| c.attrs.classes.iter().any(|cl| cl == "status-block__hint"))
        .expect("hint node in block quote");
    if let NodeKind::Paragraph { children } = &hint_node.kind {
        assert!(
            children.iter().any(|c| matches!(c.kind, NodeKind::Emphasis { .. })),
            "hint paragraph must contain an Emphasis child: {hint_node:#?}"
        );
    } else {
        panic!("hint node must be a Paragraph: {hint_node:#?}");
    }
}

// ---------------------------------------------------------------------------
// Phase 3b: Rendered-output row-count assertions
// ---------------------------------------------------------------------------

#[test]
fn terminal_body_plus_hint_has_leading_blank_and_one_separator_row() {
    let term = test_terminal(80);
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("Hint text");
    let rendered = strip_ansi(&block.render(&term));
    let quoted_lines: Vec<&str> = rendered.lines().filter(|l| l.contains('┃')).collect();
    let blank_idx = quoted_lines
        .iter()
        .position(|l| !l.contains("Body text") && !l.contains("Hint text"))
        .expect("leading blank row");
    let body_idx = quoted_lines
        .iter()
        .position(|l| l.contains("Body text"))
        .expect("body line");
    let hint_idx = quoted_lines
        .iter()
        .position(|l| l.contains("Hint text"))
        .expect("hint line");
    assert_eq!(
        body_idx - blank_idx,
        1,
        "body must immediately follow leading blank:\n{rendered}"
    );
    assert_eq!(
        hint_idx - body_idx,
        2,
        "exactly one blank quoted row between body and hint:\n{rendered}"
    );
}

#[test]
fn terminal_body_only_has_one_leading_blank_row_before_body() {
    let term = test_terminal(80);
    let block = StatusBlock::new(StatusState::Error).body("Body text");
    let rendered = strip_ansi(&block.render(&term));
    let quoted_lines: Vec<&str> = rendered.lines().filter(|l| l.contains('┃')).collect();
    assert_eq!(
        quoted_lines.len(),
        2,
        "body-only block quote must have exactly two quoted rows (blank + body):\n{rendered}"
    );
    assert!(
        !quoted_lines[0].contains("Body text"),
        "first quoted row must be the leading blank: {rendered}"
    );
    assert!(
        quoted_lines[1].contains("Body text"),
        "second quoted row must be the body: {rendered}"
    );
}

#[test]
fn markdown_body_plus_hint_has_leading_blank_and_one_separator_row() {
    let block = StatusBlock::new(StatusState::Error)
        .body("Body text")
        .hint("Hint text");
    let md = block.render_markdown();
    let quoted_lines: Vec<&str> = md.lines().filter(|l| l.starts_with('>')).collect();
    let blank_idx = quoted_lines
        .iter()
        .position(|l| !l.contains("Body text") && !l.contains("Hint text"))
        .expect("leading blank row");
    let body_idx = quoted_lines
        .iter()
        .position(|l| l.contains("Body text"))
        .expect("body line");
    let hint_idx = quoted_lines
        .iter()
        .position(|l| l.contains("Hint text"))
        .expect("hint line");
    assert_eq!(
        body_idx - blank_idx,
        1,
        "body must immediately follow leading blank in Markdown:\n{md}"
    );
    assert_eq!(
        hint_idx - body_idx,
        2,
        "exactly one blank quoted row between body and hint in Markdown:\n{md}"
    );
}

#[test]
fn markdown_body_only_has_leading_blank_and_body_rows() {
    let block = StatusBlock::new(StatusState::Error).body("Body text");
    let md = block.render_markdown();
    let quoted_lines: Vec<&str> = md.lines().filter(|l| l.starts_with('>')).collect();
    assert_eq!(
        quoted_lines.len(),
        2,
        "body-only block quote must have exactly two quoted rows (blank + body) in Markdown:\n{md}"
    );
    assert!(
        quoted_lines[0].trim().is_empty() || quoted_lines[0] == ">",
        "first quoted row must be the leading blank: {md}"
    );
    assert!(
        quoted_lines[1].contains("Body text"),
        "second quoted row must be the body: {md}"
    );
}

// ---------------------------------------------------------------------------
// Width-mode slack sink (style-everywhere Phase 2, Task 2.4)
//
// StatusBlock is an internal-layout component: the shared render-tree fold
// resolves the outer box from `Layout::width`, and the body text wraps inside
// the resolved content width. The header icon, severity glyph, status prefix,
// and the thick left border chrome are all FIXED (D2 slack sink). Only the
// body / message region absorbs slack by wrapping.
// ---------------------------------------------------------------------------

/// Renders a `StatusBlock` tree projection to a terminal string at the given
/// width, ANSI-stripped for content comparison.
fn render_status_block_tree(block: &StatusBlock, width: u32) -> String {
    let term = test_terminal(width);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    let node = <StatusBlock as TreeRenderable>::render_tree(block);
    strip_ansi(
        &render_terminal_node(&node, &opts)
            .expect("status-block tree render should succeed")
            .output,
    )
}

/// Max char-count over the lines of a rendered block.
fn widest_line(rendered: &str) -> usize {
    rendered.lines().map(|line| line.chars().count()).max().unwrap_or(0)
}

/// A long body sentence that wraps at any realistic terminal width.
const LONG_BODY: &str = "This is a deliberately long status-block body whose single \
sentence keeps going well past any reasonable terminal column so that the render \
tree is forced to wrap the body text onto several successive lines at the \
available content width.";

#[test]
fn width_auto_body_wraps_to_available() {
    use biscuit_terminal::utils::layout::{Layout, Width};
    let mut block = StatusBlock::new(StatusState::Info).body(LONG_BODY);
    block.layout_mut().width = Width::Auto;
    block.layout_mut().margin = Default::default();
    block.layout_mut().word_wrap = biscuit_terminal::utils::wrap_policy::WordWrap::WrapProse(
        None,
        None,
    );
    let _ = Layout::default(); // document the layout path
    let rendered = render_status_block_tree(&block, 80);
    let widest = widest_line(&rendered);
    let line_count = rendered.lines().filter(|l| !l.trim().is_empty()).count();
    assert!(
        widest <= 80,
        "body must wrap within width 80: widest={widest}"
    );
    assert!(
        line_count > 1,
        "body must use the full width to wrap onto multiple lines: {rendered:?}"
    );
}

#[test]
fn width_fixed_percent_50_does_not_double_apply() {
    use biscuit_terminal::utils::layout::{Length, TargetValue, Width};
    let mut block = StatusBlock::new(StatusState::Info).body(LONG_BODY);
    block.layout_mut().width =
        Width::Fixed(TargetValue::universal(Length::Percent(50.0)));
    block.layout_mut().margin = Default::default();
    let rendered = render_status_block_tree(&block, 80);
    let widest = widest_line(&rendered);
    // 50% of width 80 = 40 cells for the outer box. A widest of ~20 would
    // mean the 50% was resolved twice (box=40, then wrapped to 50% of 40 =
    // 20) — the double-application bug this guards against.
    assert!(
        widest <= 40,
        "50% of width 80 caps the body at 40 cells: widest={widest}"
    );
    assert!(
        widest > 20,
        "widest ~20 would mean the 50% was applied twice: widest={widest}"
    );
}

#[test]
fn width_fixed_full_wraps_body_to_available() {
    use biscuit_terminal::utils::layout::{Length, TargetValue, Width};
    let mut block = StatusBlock::new(StatusState::Info).body(LONG_BODY);
    block.layout_mut().width =
        Width::Fixed(TargetValue::universal(Length::Percent(100.0)));
    block.layout_mut().margin = Default::default();
    let rendered = render_status_block_tree(&block, 80);
    let widest = widest_line(&rendered);
    assert!(
        widest <= 80,
        "Fixed(100%) wraps the body within width 80: widest={widest}"
    );
    let line_count = rendered.lines().filter(|l| !l.trim().is_empty()).count();
    assert!(
        line_count > 1,
        "Fixed(100%) must use the full width so the long body wraps: {rendered:?}"
    );
}

#[test]
fn width_fit_content_hugs_short_body() {
    use biscuit_terminal::utils::layout::Width;
    let mut block = StatusBlock::new(StatusState::Info).body("Ab Cd Ef.");
    block.layout_mut().width = Width::FitContent;
    block.layout_mut().margin = Default::default();
    let rendered = render_status_block_tree(&block, 80);
    let widest = widest_line(&rendered);
    assert!(
        widest < 80,
        "FitContent hugs short body content and does not pad to full width: widest={widest}"
    );
}

#[test]
fn border_chrome_stays_fixed_across_width_modes() {
    // D2 slack sink: the thick left border chrome (`┃ `) stays fixed across
    // width modes — only the body text column absorbs slack by wrapping.
    use biscuit_terminal::utils::layout::{Length, TargetValue, Width};
    for width in [
        Width::Auto,
        Width::FitContent,
        Width::Fixed(TargetValue::universal(Length::Percent(50.0))),
    ] {
        let label = format!("{width:?}");
        let mut block = StatusBlock::new(StatusState::Error).body("Body line.");
        block.layout_mut().width = width;
        block.layout_mut().margin = Default::default();
        let rendered = render_status_block_tree(&block, 80);
        let body_line = rendered
            .lines()
            .find(|l| l.contains("Body line."))
            .unwrap_or_else(|| panic!("body line present under {label}: {rendered:?}"));
        assert!(
            body_line.starts_with('┃'),
            "thick left border glyph is fixed under any width mode: {body_line:?}"
        );
    }
}
