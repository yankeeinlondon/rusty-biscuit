//! Tree-path tests for `UnorderedList` after its render-tree migration.
//!
//! These tests mirror `ordered_list_parity.rs` and assert that:
//!
//! 1. `UnorderedList::render(&term)` (the user-facing path) routes through the
//!    canonical render tree.
//! 2. The single private projection helper drives both
//!    [`TreeRenderable::render_tree`] and
//!    [`TerminalRenderable::render_tree_node`] so the two entry points cannot
//!    drift.
//! 3. `MarkdownRenderable` and `BrowserRenderable` impls produce the expected
//!    CommonMark / HTML output.
//!
//! ## Resolved divergences
//!
//! - **Prose inline styling on terminal**: items that are `Prose` components
//!   previously lost their `<b>` / `<i>` / `<red>` styling because
//!   `Prose::render_tree_node()` returns `None` and the generic
//!   `RenderableTerminalContent::to_tree_nodes` fallback strips ANSI to plain
//!   text. `project_list_items` now downcasts `Prose` and projects through
//!   `Prose::to_render_nodes`, mirroring the `BlockQuote` / `Compose` /
//!   `OrderedList` pattern. Inline styling now survives on the Terminal
//!   target; the Markdown target still degrades to plain text because the
//!   cross-target tree projection has no `<b>` peer in CommonMark.

mod parity_helpers;

use biscuit_terminal::components::list::{OrderedList, UnorderedList};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{
    BrowserRenderable, RenderableTerminalContent, TerminalRenderable,
};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use renderable::markdown::MarkdownRenderable;
use renderable::tree::{NodeKind, RenderNode, RenderStrictness, TreeRenderable};
use std::rc::Rc;

use parity_helpers::{strip_ansi, test_terminal};

/// Walks a `RenderNode` tree depth-first looking for a node whose `NodeKind`
/// matches `pred`. Used by nested-component structural assertions.
fn walk_has_kind(node: &RenderNode, pred: impl Fn(&NodeKind) -> bool + Copy) -> bool {
    if pred(&node.kind) {
        return true;
    }
    node.children().iter().any(|c| walk_has_kind(c, pred))
}

// ---------------------------------------------------------------------------
// TreeRenderable / projection helper coherence
// ---------------------------------------------------------------------------

#[test]
fn tree_renderable_and_compat_hook_share_one_projection() {
    // The two tree entry points must produce structurally identical nodes
    // — that is the whole point of the "single private projection helper"
    // migration pattern called out in `lessons-learned.md`.
    let list = UnorderedList::new(vec!["First", "Second", "Third"]);
    let canonical = list.render_tree();
    let compat = list.render_tree_node().expect("compat hook present");
    assert_eq!(
        format!("{canonical:?}"),
        format!("{compat:?}"),
        "TreeRenderable::render_tree and render_tree_node must agree"
    );
}

#[test]
fn tree_renderable_omits_default_bullet_hint() {
    // Default `- ` bullet normalizes to None on the canonical tree so the
    // node does not carry redundant component-level styling.
    let list = UnorderedList::new(vec!["a"]);
    let node = list.render_tree();
    let hints = node.attrs.list_hints();
    assert_eq!(hints.bullet, None, "default `- ` normalizes to None");
    assert!(hints.hanging_indent, "hanging indent is enabled by default");
}

#[test]
fn tree_renderable_preserves_custom_bullet_and_indent_children() {
    let list = UnorderedList::new(vec!["a"])
        .with_bullet("→ ")
        .with_indent_children(Some(6));
    let node = list.render_tree();
    let hints = node.attrs.list_hints();
    assert_eq!(
        hints.bullet.as_deref(),
        Some("→ "),
        "custom bullet flows into the hint"
    );
    assert_eq!(
        hints.indent_children,
        Some(6),
        "indent_children flows into the hint"
    );
}

// ---------------------------------------------------------------------------
// Tree-path rendering
// ---------------------------------------------------------------------------

/// Strips ANSI escape codes and trailing whitespace from each line, then
/// drops trailing blank lines. Used to normalize tree-rendered output for
/// stable, layout-tolerant assertions.
fn normalize_for_parity(s: &str) -> String {
    let stripped = strip_ansi(s);
    let lines: Vec<String> = stripped
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect();
    let mut end = lines.len();
    while end > 0 && lines[end - 1].is_empty() {
        end -= 1;
    }
    lines[..end].join("\n")
}

/// Renders a render-tree node to a terminal string at the given width,
/// folding it through the canonical `render_terminal_node` path.
fn render_tree(node: &RenderNode, width: u32) -> String {
    let term = test_terminal(width);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    render_terminal_node(node, &opts)
        .expect("tree render should succeed")
        .output
}

/// Widest visible (ANSI-stripped) line in the rendered output.
fn widest_visible_line(rendered: &str) -> usize {
    strip_ansi(rendered)
        .lines()
        .map(|line| line.trim_end().chars().count())
        .max()
        .unwrap_or(0)
}

#[test]
fn parity_empty_list() {
    let list = UnorderedList::new(Vec::<String>::new());
    let term = test_terminal(80);
    assert_eq!(normalize_for_parity(&list.render(&term)), "");
}

#[test]
fn parity_single_string_item() {
    let list = UnorderedList::new(vec!["Item"]);
    let term = test_terminal(80);
    let tree = normalize_for_parity(&list.render(&term));
    assert_eq!(tree, "- Item");
}

#[test]
fn parity_three_string_items() {
    let list = UnorderedList::new(vec!["First", "Second", "Third"]);
    let term = test_terminal(80);
    let tree = normalize_for_parity(&list.render(&term));
    assert!(tree.contains("- First"));
    assert!(tree.contains("- Second"));
    assert!(tree.contains("- Third"));
}

#[test]
fn parity_long_item_word_wraps() {
    let list = UnorderedList::new(vec![
        "a fairly long item that definitely wraps onto multiple terminal lines",
    ]);
    let term = test_terminal(30);
    let tree = strip_ansi(&list.render(&term));
    let lines: Vec<&str> = tree.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() > 1, "expected wrapping: {lines:?}");
    assert!(lines[0].starts_with("- "));
    for line in &lines[1..] {
        assert!(
            line.starts_with("  "),
            "continuation aligns under hanging indent: {line:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Custom bullet
// ---------------------------------------------------------------------------

#[test]
fn parity_custom_ascii_bullet() {
    let list = UnorderedList::new(vec!["First", "Second"]).with_bullet("* ");
    let term = test_terminal(80);
    let tree = normalize_for_parity(&list.render(&term));
    assert!(tree.contains("* First"), "custom bullet on tree: {tree:?}");
    assert!(tree.contains("* Second"));
}

#[test]
fn parity_custom_unicode_bullet() {
    let list = UnorderedList::new(vec!["Alpha", "Beta"]).with_bullet("→ ");
    let term = test_terminal(80);
    let tree = normalize_for_parity(&list.render(&term));
    assert!(tree.contains("→ Alpha"), "tree: {tree:?}");
    assert!(tree.contains("→ Beta"));
}

#[test]
fn parity_custom_bullet_wider_glyph_hanging_indent() {
    // A bullet wider than `- ` (visible width 2) — e.g. `*) ` (width 3) —
    // must wrap continuation lines under the wider hanging indent.
    let list = UnorderedList::new(vec![
        "a fairly long item that definitely wraps onto multiple lines",
    ])
    .with_bullet("*) ");
    let term = test_terminal(30);
    let rendered = strip_ansi(&list.render(&term));
    let lines: Vec<&str> = rendered.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() > 1, "expected wrapping: {lines:?}");
    assert!(lines[0].starts_with("*) "), "first line: {lines:?}");
    for line in &lines[1..] {
        assert!(
            line.starts_with("   "),
            "continuation under 3-char bullet must indent ≥3 spaces: {line:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Hanging indent toggle
// ---------------------------------------------------------------------------

#[test]
fn parity_disable_hanging_indent_drops_continuation_indent() {
    // The disabled-hanging-indent flag must survive through the tree path:
    // continuation lines should NOT be indented under the bullet.
    let list = UnorderedList::new(vec![
        "a fairly long item that definitely wraps onto multiple terminal lines",
    ])
    .without_hanging_indent();
    let term = test_terminal(30);
    let rendered = strip_ansi(&list.render(&term));
    let lines: Vec<&str> = rendered.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() > 1, "expected wrapping: {lines:?}");
    assert!(lines[0].starts_with("- "));
    // Continuation lines must not be indented under the bullet.
    let any_continuation_indented = lines[1..].iter().any(|l| l.starts_with("  "));
    assert!(
        !any_continuation_indented,
        "continuation lines must NOT indent when hanging_indent is off: {lines:?}"
    );
}

// ---------------------------------------------------------------------------
// Nested lists
// ---------------------------------------------------------------------------

#[test]
fn parity_nested_unordered_list_indents_block_child() {
    let inner = UnorderedList::new(vec!["Sub A", "Sub B"]);
    let items = vec![
        RenderableTerminalContent::String("Outer".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ];
    let list = UnorderedList::from(items);
    let term = test_terminal(80);
    let tree = strip_ansi(&list.render(&term));
    let lines: Vec<&str> = tree.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines[0], "- Outer");
    assert!(
        lines.iter().any(|l| l.starts_with("  - Sub A")),
        "nested unordered block child indented by 2: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("  - Sub B")),
        "second sub item indented by 2: {lines:?}"
    );
}

#[test]
fn parity_nested_ordered_list_inside_unordered() {
    let inner = OrderedList::new(vec!["First step", "Second step"]);
    let items = vec![
        RenderableTerminalContent::String("Steps".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ];
    let list = UnorderedList::from(items);
    let term = test_terminal(80);
    let tree = strip_ansi(&list.render(&term));
    let lines: Vec<&str> = tree.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines[0], "- Steps");
    assert!(
        lines.iter().any(|l| l.starts_with("  1. First step")),
        "nested ordered renders with number prefix at indent 2: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("  2. Second step")),
        "second numbered item: {lines:?}"
    );
}

#[test]
fn parity_three_level_nesting_compounds_indent() {
    // UL > UL > UL: indent compounds across levels. With the default
    // bullet width of 2, third-level bullets should sit at column 4.
    let leaf = UnorderedList::new(vec!["Leaf"]);
    let mid_items = vec![
        RenderableTerminalContent::String("Mid".to_string()),
        RenderableTerminalContent::Component(Rc::new(leaf)),
    ];
    let mid = UnorderedList::from(mid_items);
    let outer_items = vec![
        RenderableTerminalContent::String("Outer".to_string()),
        RenderableTerminalContent::Component(Rc::new(mid)),
    ];
    let outer = UnorderedList::from(outer_items);
    let term = test_terminal(80);
    let rendered = strip_ansi(&outer.render(&term));
    let lines: Vec<&str> = rendered.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.iter().any(|l| l == &"- Outer"), "L1: {lines:?}");
    assert!(
        lines.iter().any(|l| l.starts_with("  - Mid")),
        "L2 indent 2: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("    - Leaf")),
        "L3 indent 4: {lines:?}"
    );
}

#[test]
fn parity_custom_indent_children() {
    let inner = UnorderedList::new(vec!["Deep"]);
    let outer = UnorderedList::from(vec![
        RenderableTerminalContent::String("Top".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ])
    .with_indent_children(Some(8));
    let term = test_terminal(80);
    let tree = strip_ansi(&outer.render(&term));
    assert!(
        tree.lines().any(|l| l.starts_with("        - Deep")),
        "custom indent of 8 applied: {tree:?}"
    );
}

// ---------------------------------------------------------------------------
// Layout interactions
// ---------------------------------------------------------------------------

#[test]
fn parity_with_left_margin() {
    use biscuit_terminal::utils::layout::{Length, TargetValue};

    let mut list = UnorderedList::new(vec!["First", "Second"]);
    list.layout_mut().margin.left = TargetValue::universal(Length::ch(4));
    let term = test_terminal(80);
    let tree = strip_ansi(&list.render(&term));
    for line in tree.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            line.starts_with("    "),
            "left margin shifts each line: {line:?}"
        );
    }
}

#[test]
fn parity_with_right_margin_narrows_wrap_width() {
    // A right margin must narrow the available content width, forcing items
    // to wrap earlier than they would without it.
    use biscuit_terminal::utils::layout::{Length, TargetValue};

    let item = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
    let plain = UnorderedList::new(vec![item]);
    let mut narrowed = UnorderedList::new(vec![item]);
    narrowed.layout_mut().margin.right = TargetValue::universal(Length::ch(20));

    let term = test_terminal(50);
    let plain_lines = strip_ansi(&plain.render(&term))
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    let narrowed_lines = strip_ansi(&narrowed.render(&term))
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    assert!(
        narrowed_lines > plain_lines,
        "right margin must force earlier wrapping: plain={plain_lines}, narrowed={narrowed_lines}"
    );
}

#[test]
fn parity_with_alignment_set() {
    use biscuit_terminal::utils::layout::Alignment;

    let mut list = UnorderedList::new(vec!["First", "Second"]);
    list.layout_mut().alignment = Alignment::Center;
    let term = test_terminal(80);
    let rendered = strip_ansi(&list.render(&term));
    assert!(rendered.contains("- First"), "got: {rendered:?}");
    assert!(rendered.contains("- Second"), "got: {rendered:?}");
    // Markdown is layout-agnostic.
    let md = list.render_markdown();
    assert!(md.contains("- First"), "md: {md:?}");
    assert!(md.contains("- Second"), "md: {md:?}");
}

// ---------------------------------------------------------------------------
// Mixed inline + block children
// ---------------------------------------------------------------------------

#[test]
fn parity_mixed_inline_and_block_children() {
    let inner = UnorderedList::new(vec!["Apple", "Banana"]);
    let items = vec![
        RenderableTerminalContent::String("Plain string".to_string()),
        RenderableTerminalContent::Component(Rc::new(Prose::new("Inline prose"))),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ];
    let list = UnorderedList::from(items);
    let term = test_terminal(80);
    let tree = strip_ansi(&list.render(&term));
    let lines: Vec<&str> = tree.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.iter().any(|l| l.starts_with("- Plain string")));
    assert!(lines.iter().any(|l| l.starts_with("- Inline prose")));
    assert!(
        lines.iter().any(|l| l.starts_with("  - Apple")),
        "nested unordered block child indented: {lines:?}"
    );
}

// ---------------------------------------------------------------------------
// From<Prose> conversion
// ---------------------------------------------------------------------------

#[test]
fn parity_from_single_prose() {
    let list: UnorderedList = UnorderedList::from(Prose::new("Just one"));
    let term = test_terminal(80);
    let tree = normalize_for_parity(&list.render(&term));
    assert!(tree.contains("- Just one"), "tree: {tree:?}");
}

// ---------------------------------------------------------------------------
// Very narrow widths
// ---------------------------------------------------------------------------

#[test]
fn parity_very_narrow_width_preserves_content() {
    // At a very narrow terminal width (10 cols), content must still be
    // present — bullets, items, no panics. We accept whatever wrap shape
    // both paths produce; only content survival is required.
    let list = UnorderedList::new(vec!["alpha beta", "gamma delta"]);
    let term = test_terminal(10);
    let tree = strip_ansi(&list.render(&term));
    assert!(tree.contains("alpha"), "alpha survives narrow: {tree:?}");
    assert!(tree.contains("beta"), "beta survives narrow: {tree:?}");
    assert!(tree.contains("gamma"), "gamma survives narrow: {tree:?}");
    assert!(tree.contains("delta"), "delta survives narrow: {tree:?}");
}

// ---------------------------------------------------------------------------
// Post-construction bullet change
// ---------------------------------------------------------------------------

#[test]
fn parity_bullet_changed_after_construction_updates_items() {
    // Building with the default bullet and then calling `with_bullet`
    // should update the bullet on every existing item, and the hanging
    // indent on inline component items should reflect the new bullet's
    // visible width.
    let items = vec![
        RenderableTerminalContent::String("plain string".to_string()),
        RenderableTerminalContent::Component(Rc::new(Prose::new(
            "a fairly long prose item that wraps onto multiple lines under the bullet",
        ))),
    ];
    let list = UnorderedList::from(items).with_bullet("*) ");
    let term = test_terminal(30);
    let rendered = strip_ansi(&list.render(&term));
    let lines: Vec<&str> = rendered.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        lines.iter().any(|l| l.starts_with("*) plain string")),
        "string item uses new bullet: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("*) a fairly long")),
        "prose item uses new bullet: {lines:?}"
    );
    // Any continuation line under the prose item must indent under the new
    // 3-char bullet (the bullet-change path force-updates the hanging
    // indent on inline component items).
    let prose_first = lines
        .iter()
        .position(|l| l.starts_with("*) a fairly long"))
        .expect("prose item present");
    for line in &lines[prose_first + 1..] {
        if line.starts_with("*) ") {
            break;
        }
        assert!(
            line.starts_with("   "),
            "continuation under 3-char bullet must indent ≥3 spaces: {line:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Width-mode slack sink (style-everywhere Phase 2, Task 2.3)
//
// A list's `Layout.width` resolves the outer box, which narrows
// `available_width`; the item body TEXT column then wraps to that width while
// the bullet / hanging-indent stays fixed. So the item body text column is
// the documented slack sink (spec decision D2). Lists HUG naturally: a short
// item stays short, a long item wraps to the available width.
// ---------------------------------------------------------------------------

/// A ~200-char sentence that must wrap at any realistic terminal width.
const LONG_ITEM: &str = "This is a deliberately long unordered-list item whose \
single sentence keeps going well past any reasonable terminal column so that \
the render tree is forced to wrap the item body text onto several successive \
lines at the available content width.";

#[test]
fn width_auto_body_wraps_to_available() {
    let layout = biscuit_terminal::utils::layout::Layout {
        width: biscuit_terminal::utils::layout::Width::Auto,
        ..Default::default()
    };
    let list = UnorderedList::new(vec![LONG_ITEM]).with_layout(layout);
    let node = list.render_tree();
    let rendered = render_tree(&node, 80);
    let widest = widest_visible_line(&rendered);
    let line_count = strip_ansi(&rendered)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    assert!(widest <= 80, "body must wrap within width 80: widest={widest}");
    assert!(
        line_count > 1,
        "body must use the full width to wrap onto multiple lines: {rendered:?}"
    );
}

#[test]
fn width_fixed_percent_50_does_not_double_apply() {
    use biscuit_terminal::utils::layout::{Length, TargetValue, Width};

    let layout = biscuit_terminal::utils::layout::Layout {
        width: Width::Fixed(TargetValue::universal(Length::Percent(50.0))),
        ..Default::default()
    };
    let list = UnorderedList::new(vec![LONG_ITEM]).with_layout(layout);
    let node = list.render_tree();
    let rendered = render_tree(&node, 80);
    let widest = widest_visible_line(&rendered);
    // 50% of width 80 = 40 cells for the outer box. A widest of ~20 would mean
    // the 50% was resolved twice (box=40, then wrapped to 50% of 40 = 20) —
    // the double-application bug this guards against.
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
fn width_fit_content_hugs_short_content() {
    use biscuit_terminal::utils::layout::Width;

    let layout = biscuit_terminal::utils::layout::Layout {
        width: Width::FitContent,
        ..Default::default()
    };
    let list = UnorderedList::new(vec!["Ab", "Cd"]).with_layout(layout);
    let node = list.render_tree();
    let rendered = render_tree(&node, 80);
    let widest = widest_visible_line(&rendered);
    assert!(
        widest < 80,
        "FitContent hugs short content and does not pad to full width: widest={widest}"
    );
}

#[test]
fn width_fixed_full_wraps_body_to_available() {
    // Width::Fixed(100%) is the explicit "fill the parent's available width"
    // contract (spec C2). The list body wraps within the resolved content
    // width; the bullet / hanging indent stays fixed (D2 slack sink).
    use biscuit_terminal::utils::layout::{Length, TargetValue, Width};

    let layout = biscuit_terminal::utils::layout::Layout {
        width: Width::Fixed(TargetValue::universal(Length::Percent(100.0))),
        ..Default::default()
    };
    let list = UnorderedList::new(vec![LONG_ITEM]).with_layout(layout);
    let node = list.render_tree();
    let rendered = render_tree(&node, 80);
    let widest = widest_visible_line(&rendered);
    assert!(
        widest <= 80,
        "Fixed(100%) wraps the body within width 80: widest={widest}"
    );
    let line_count = strip_ansi(&rendered)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    assert!(
        line_count > 1,
        "Fixed(100%) must use the full width so the long item wraps: {rendered:?}"
    );
}

#[test]
fn bullet_and_hanging_indent_stay_fixed_under_width_modes() {
    // D2 slack sink: the bullet ("- ") and the hanging indent are fixed, so
    // they keep their geometry across width modes. Only the item body text
    // column absorbs slack by wrapping.
    use biscuit_terminal::utils::layout::Width;

    for layout in [
        biscuit_terminal::utils::layout::Layout::default(),
        biscuit_terminal::utils::layout::Layout {
            width: Width::Fixed(biscuit_terminal::utils::layout::TargetValue::universal(
                biscuit_terminal::utils::layout::Length::Percent(50.0),
            )),
            ..Default::default()
        },
    ] {
        let list = UnorderedList::new(vec!["Body line that is wide enough to wrap under narrow widths."])
            .with_layout(layout);
        let node = list.render_tree();
        let rendered = render_tree(&node, 80);
        let stripped = strip_ansi(&rendered);
        let first = stripped
            .lines()
            .next()
            .expect("at least one rendered line");
        assert!(
            first.starts_with("- "),
            "bullet is fixed at \"- \" under any width mode: {first:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Optimistic render parity
// ---------------------------------------------------------------------------

#[test]
fn optimistic_render_matches_default_terminal_render() {
    let list = UnorderedList::new(vec!["First", "Second", "Third"]);
    let term = test_terminal(80);
    let optimistic = strip_ansi(&list.render_optimistic(Some(80)));
    let explicit = strip_ansi(&list.render(&term));
    assert_eq!(
        optimistic, explicit,
        "render() at width 80 == optimistic(80)"
    );
}

// ---------------------------------------------------------------------------
// MarkdownRenderable
// ---------------------------------------------------------------------------

#[test]
fn markdown_empty_list_is_empty() {
    let list = UnorderedList::new(Vec::<String>::new());
    assert!(list.render_markdown().is_empty());
}

#[test]
fn markdown_single_item_uses_dash_prefix() {
    let list = UnorderedList::new(vec!["Step"]);
    let md = list.render_markdown();
    assert!(md.contains("- Step"), "got: {md:?}");
}

#[test]
fn markdown_three_items_use_dash_prefix_in_order() {
    let list = UnorderedList::new(vec!["First", "Second", "Third"]);
    let md = list.render_markdown();
    let first = md.find("- First").expect("first item present");
    let second = md.find("- Second").expect("second item present");
    let third = md.find("- Third").expect("third item present");
    assert!(first < second && second < third, "ordering: {md:?}");
}

#[test]
fn markdown_plus_matches_portable_markdown() {
    // UnorderedList is a structural container with no color/border/fill of
    // its own, so MarkdownPlus must produce the same output as portable
    // Markdown.
    let list = UnorderedList::new(vec!["First", "Second"]);
    assert_eq!(list.render_markdown(), list.render_markdown_plus());
}

#[test]
fn markdown_ignores_custom_bullet() {
    // Markdown's portable contract uses `- ` regardless of the terminal
    // bullet. This pins that the bullet hint does not leak into the
    // Markdown target.
    let list = UnorderedList::new(vec!["Alpha", "Beta"]).with_bullet("→ ");
    let md = list.render_markdown();
    assert!(md.contains("- Alpha"), "portable dash: {md:?}");
    assert!(md.contains("- Beta"), "portable dash: {md:?}");
    assert!(!md.contains("→ "), "no custom bullet leak: {md:?}");
}

#[test]
fn markdown_layout_does_not_change_output() {
    use biscuit_terminal::utils::layout::{Length, TargetValue};

    let plain = UnorderedList::new(vec!["A", "B"]);
    let mut with_margin = UnorderedList::new(vec!["A", "B"]);
    with_margin.layout_mut().margin.left = TargetValue::universal(Length::ch(4));
    assert_eq!(
        plain.render_markdown(),
        with_margin.render_markdown(),
        "Markdown ignores layout (locked by tree-renderer contract)"
    );
}

// ---------------------------------------------------------------------------
// BrowserRenderable
// ---------------------------------------------------------------------------

#[test]
fn html_empty_list_emits_empty_ul() {
    let list = UnorderedList::new(Vec::<String>::new());
    let html = list.render_html_fragment().render();
    assert!(html.contains("<ul"), "got: {html:?}");
    assert!(html.contains("</ul>"), "got: {html:?}");
}

#[test]
fn html_single_item_wraps_in_li() {
    let list = UnorderedList::new(vec!["Item"]);
    let html = list.render_html_fragment().render();
    assert!(html.contains("<ul"), "ul present: {html:?}");
    assert!(html.contains("<li>"), "li present: {html:?}");
    assert!(html.contains("Item"), "content present: {html:?}");
    assert!(html.contains("</li>"), "li closed: {html:?}");
    assert!(html.contains("</ul>"), "ul closed: {html:?}");
}

#[test]
fn html_three_items_emit_three_li_elements() {
    let list = UnorderedList::new(vec!["First", "Second", "Third"]);
    let html = list.render_html_fragment().render();
    let li_count = html.matches("<li>").count();
    assert_eq!(li_count, 3, "three <li> elements: {html:?}");
    assert!(html.contains("First"));
    assert!(html.contains("Second"));
    assert!(html.contains("Third"));
}

#[test]
fn html_nested_unordered_list_nests_ul() {
    let inner = UnorderedList::new(vec!["Inner"]);
    let outer = UnorderedList::from(vec![
        RenderableTerminalContent::String("Outer".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ]);
    let html = outer.render_html_fragment().render();
    let first_ul = html.find("<ul").expect("outer ul present");
    let second_ul = html[first_ul + 3..].find("<ul").expect("nested ul present");
    let _ = second_ul;
    assert!(html.contains("Outer"));
    assert!(html.contains("Inner"));
}

#[test]
fn html_nested_ordered_list_emits_ol() {
    let inner = OrderedList::new(vec!["Apple", "Banana"]);
    let outer = UnorderedList::from(vec![
        RenderableTerminalContent::String("Fruits".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ]);
    let html = outer.render_html_fragment().render();
    assert!(html.contains("<ul"), "outer unordered: {html:?}");
    assert!(html.contains("<ol"), "inner ordered: {html:?}");
    assert!(html.contains("Apple"));
    assert!(html.contains("Banana"));
}

// ---------------------------------------------------------------------------
// Cross-target: Prose styling survives terminal, degrades for Markdown
// ---------------------------------------------------------------------------

#[test]
fn prose_item_content_survives_terminal_render() {
    let mut list = UnorderedList::empty();
    list.add(Prose::new("<b>Bold</b> item"));
    let term = test_terminal(80);
    let rendered = list.render(&term);
    assert!(
        strip_ansi(&rendered).contains("Bold item"),
        "content survives: {rendered:?}"
    );
}

#[test]
fn prose_inline_styling_survives_terminal_render() {
    // Pin: after the `project_list_items` Prose-downcast fix (mirroring
    // BlockQuote / Compose / OrderedList), inline `<b>` / `<i>` / `<red>`
    // styling must survive on the Terminal target. We assert on the bold
    // SGR open sequence; the exact close/reset bytes depend on the
    // renderer's pairing strategy.
    let mut list = UnorderedList::empty();
    list.add(Prose::new("<b>foo</b>"));
    let term = test_terminal(80);
    let rendered = list.render(&term);
    assert!(
        rendered.contains("\x1b[1m"),
        "bold SGR must be present (regression for Prose-downcast fix): {rendered:?}"
    );
    assert!(
        strip_ansi(&rendered).contains("foo"),
        "content survives alongside styling: {rendered:?}"
    );
}

#[test]
fn prose_inline_styling_degrades_in_markdown() {
    let mut list = UnorderedList::empty();
    list.add(Prose::new("<b>Bold</b> item"));
    let md = list.render_markdown();
    assert!(md.contains("Bold"), "content present: {md:?}");
    assert!(!md.contains("<b>"), "raw style tokens stripped: {md:?}");
}

// ---------------------------------------------------------------------------
// Stage 3a: structural nested-component projection inside list items
//
// `project_list_items` runs `project_renderable_content` in
// `ProjectionMode::Structural`. A nested block component inside an item
// must therefore appear as the matching `NodeKind` (Section, BlockQuote,
// Table, List) inside the projected `NodeKind::ListItem` — not flatten to
// plain text.
// ---------------------------------------------------------------------------

/// Returns the `ListItem` children of a projected `NodeKind::List`.
fn list_item_children(node: &RenderNode) -> &[RenderNode] {
    match &node.kind {
        NodeKind::List { children, .. } => children,
        other => panic!("expected NodeKind::List, got {other:?}"),
    }
}

#[test]
fn nested_section_inside_unordered_list_item_projects_structural_section_node() {
    use biscuit_terminal::components::section::{HeadingLevel, Section};

    let nested = Section::new(HeadingLevel::h3, "Nested Title");
    let list = UnorderedList::from(vec![
        RenderableTerminalContent::String("First".to_string()),
        RenderableTerminalContent::Component(Rc::new(nested)),
    ]);
    let node = list.render_tree();
    let items = list_item_children(&node);

    assert!(
        items
            .iter()
            .any(|item| walk_has_kind(item, |k| matches!(k, NodeKind::Section { .. }))),
        "nested Section must appear as a structural Section node inside a ListItem: {items:?}"
    );
}

#[test]
fn nested_block_quote_inside_unordered_list_item_projects_structural_block_quote_node() {
    use biscuit_terminal::components::block_quote::BlockQuote;

    let nested = BlockQuote::new("quoted body".into(), None::<&str>);
    let list = UnorderedList::from(vec![
        RenderableTerminalContent::String("Lead".to_string()),
        RenderableTerminalContent::Component(Rc::new(nested)),
    ]);
    let node = list.render_tree();
    let items = list_item_children(&node);

    assert!(
        items
            .iter()
            .any(|item| walk_has_kind(item, |k| matches!(k, NodeKind::BlockQuote { .. }))),
        "nested BlockQuote must appear as a structural BlockQuote node inside a ListItem: \
         {items:?}"
    );
}

#[test]
fn nested_table_inside_unordered_list_item_projects_structural_table_node() {
    use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};

    let table = Table::new()
        .with_columns(vec![TableColumn::new("A"), TableColumn::new("B")])
        .with_data(vec![vec![
            TableCellContent::Text("x".into()),
            TableCellContent::Text("y".into()),
        ]]);
    let list = UnorderedList::from(vec![
        RenderableTerminalContent::String("Row".to_string()),
        RenderableTerminalContent::Component(Rc::new(table)),
    ]);
    let node = list.render_tree();
    let items = list_item_children(&node);

    assert!(
        items
            .iter()
            .any(|item| walk_has_kind(item, |k| matches!(k, NodeKind::Table { .. }))),
        "nested Table must appear as a structural Table node inside a ListItem: {items:?}"
    );
}

#[test]
fn nested_ordered_list_inside_unordered_list_item_projects_ordered_list_node() {
    let inner = OrderedList::new(vec!["Inner A", "Inner B"]);
    let list = UnorderedList::from(vec![
        RenderableTerminalContent::String("Outer".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ]);
    let node = list.render_tree();
    let items = list_item_children(&node);

    assert!(
        items
            .iter()
            .any(|item| walk_has_kind(item, |k| matches!(k, NodeKind::List { ordered: true, .. }))),
        "nested OrderedList must appear as a structural ordered List node inside a ListItem: \
         {items:?}"
    );
}

#[test]
fn nested_unordered_list_inside_unordered_list_item_projects_unordered_list_node() {
    let inner = UnorderedList::new(vec!["Apple", "Banana"]);
    let list = UnorderedList::from(vec![
        RenderableTerminalContent::String("Fruits".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ]);
    let node = list.render_tree();
    let items = list_item_children(&node);

    assert!(
        items.iter().any(|item| walk_has_kind(item, |k| matches!(
            k,
            NodeKind::List { ordered: false, .. }
        ))),
        "nested UnorderedList must appear as a structural unordered List node inside a \
         ListItem: {items:?}"
    );
}
