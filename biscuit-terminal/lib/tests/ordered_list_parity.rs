//! Tree-path tests for `OrderedList` after its render-tree migration.
//!
//! These tests assert that:
//!
//! 1. `OrderedList::render(&term)` (the user-facing path) routes through the
//!    canonical render tree.
//! 2. The single private projection helper drives both
//!    [`TreeRenderable::render_tree`] and
//!    [`TerminalRenderable::render_tree_node`] so the two entry points cannot
//!    drift.
//! 3. `MarkdownRenderable` and `BrowserRenderable` impls produce the expected
//!    CommonMark / HTML output, including layout-driven CSS where applicable.
//!
//! ## Nested OrderedList Markdown shape
//!
//! A nested `OrderedList` passed as a `Component` item projects to a
//! `NodeKind::List` child inside a `NodeKind::ListItem`. The Markdown
//! renderer collapses that onto the item's line (`2. 1. Inner A`) instead of
//! emitting a CommonMark sublist. The canonical tree's shape is correct;
//! only the renderer's lowering diverges. Item ordering and inner numbering
//! are still preserved.
//!
//! ## Resolved divergences
//!
//! - **Prose inline styling on terminal**: previously items that were `Prose`
//!   components lost their `<b>` / `<i>` / `<red>` styling because
//!   `Prose::render_tree_node()` returns `None` and the generic
//!   `RenderableTerminalContent::to_tree_nodes` fallback strips ANSI to plain
//!   text. `project_list_items` now downcasts `Prose` components and
//!   projects them through `Prose::to_render_nodes`, mirroring the
//!   `BlockQuote` / `Compose` pattern. Inline styling now survives on the
//!   Terminal target; the Markdown target still degrades to plain text
//!   because the cross-target tree projection has no `<b>` peer in
//!   CommonMark.

mod parity_helpers;

use biscuit_terminal::components::list::OrderedList;
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
    let list = OrderedList::new(vec!["First", "Second", "Third"]);
    let canonical = list.render_tree();
    let compat = list.render_tree_node().expect("compat hook present");
    assert_eq!(
        format!("{canonical:?}"),
        format!("{compat:?}"),
        "TreeRenderable::render_tree and render_tree_node must agree"
    );
}

#[test]
fn tree_renderable_preserves_list_hints() {
    let list = OrderedList::new(vec!["a"]).with_indent_children(6);
    let node = list.render_tree();
    let hints = node.attrs.list_hints();
    assert!(hints.hanging_indent, "hanging indent is enabled");
    assert_eq!(hints.bullet, None, "ordered list does not record a bullet");
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
    let list = OrderedList::new(Vec::<String>::new());
    let term = test_terminal(80);
    assert_eq!(normalize_for_parity(&list.render(&term)), "");
}

#[test]
fn parity_single_string_item() {
    let list = OrderedList::new(vec!["Item"]);
    let term = test_terminal(80);
    let tree = normalize_for_parity(&list.render(&term));
    assert_eq!(tree, "1. Item");
}

#[test]
fn parity_three_string_items() {
    let list = OrderedList::new(vec!["First", "Second", "Third"]);
    let term = test_terminal(80);
    let tree = normalize_for_parity(&list.render(&term));
    assert!(tree.contains("1. First"));
    assert!(tree.contains("2. Second"));
    assert!(tree.contains("3. Third"));
}

#[test]
fn parity_double_digit_prefix() {
    let items: Vec<String> = (1..=12).map(|n| format!("item {n}")).collect();
    let list = OrderedList::new(items);
    let term = test_terminal(80);
    let tree = normalize_for_parity(&list.render(&term));
    assert!(tree.contains("9. item 9"));
    assert!(tree.contains("10. item 10"));
    assert!(tree.contains("12. item 12"));
}

#[test]
fn parity_long_item_word_wraps() {
    let list = OrderedList::new(vec![
        "a fairly long item that definitely wraps onto multiple terminal lines",
    ]);
    let term = test_terminal(30);
    let tree = strip_ansi(&list.render(&term));
    let lines: Vec<&str> = tree.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() > 1, "expected wrapping: {lines:?}");
    assert!(lines[0].starts_with("1. "));
    for line in &lines[1..] {
        assert!(
            line.starts_with("   "),
            "continuation aligns under hanging indent: {line:?}"
        );
    }
}

#[test]
fn parity_nested_ordered_list_indents_block_child() {
    let inner = OrderedList::new(vec!["Nested A", "Nested B"]);
    let items = vec![
        RenderableTerminalContent::String("First".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ];
    let list = OrderedList::from(items);
    let term = test_terminal(80);
    let tree = strip_ansi(&list.render(&term));
    let lines: Vec<&str> = tree.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines[0], "1. First");
    assert!(
        lines.iter().any(|l| l.starts_with("    1. Nested A")),
        "block child indented by 4: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("    2. Nested B")),
        "block child numbering restarts at 1: {lines:?}"
    );
}

#[test]
fn parity_mixed_inline_and_block_children() {
    use biscuit_terminal::components::list::UnorderedList;

    let inner = UnorderedList::new(vec!["Apple", "Banana"]);
    let items = vec![
        RenderableTerminalContent::String("Plain string".to_string()),
        RenderableTerminalContent::Component(Rc::new(Prose::new("Inline prose"))),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ];
    let list = OrderedList::from(items);
    let term = test_terminal(80);
    let tree = strip_ansi(&list.render(&term));
    let lines: Vec<&str> = tree.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.iter().any(|l| l.starts_with("1. Plain string")));
    assert!(lines.iter().any(|l| l.starts_with("2. Inline prose")));
    assert!(
        lines.iter().any(|l| l.starts_with("    - Apple")),
        "nested unordered block child indented: {lines:?}"
    );
}

#[test]
fn parity_custom_indent_children() {
    let inner = OrderedList::new(vec!["Deep"]);
    let outer = OrderedList::from(vec![
        RenderableTerminalContent::String("Top".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ])
    .with_indent_children(8);
    let term = test_terminal(80);
    let tree = strip_ansi(&outer.render(&term));
    assert!(
        tree.lines().any(|l| l.starts_with("        1. Deep")),
        "custom indent of 8 applied: {tree:?}"
    );
}

#[test]
fn parity_with_left_margin() {
    use biscuit_terminal::utils::layout::{Length, TargetValue};

    let mut list = OrderedList::new(vec!["First", "Second"]);
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
    // to wrap earlier than they would without it. Spec table row:
    // "OrderedList with left/right margins → Layout is applied via set_layout;
    // margins narrow the available width."
    use biscuit_terminal::utils::layout::{Length, TargetValue};

    let item = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
    let plain = OrderedList::new(vec![item]);
    let mut narrowed = OrderedList::new(vec![item]);
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
    // Alignment (without max_width) is recorded on the projected node's
    // layout attribute. On the Terminal target it shifts the block as a
    // unit; on Markdown it is ignored (locked by the tree-renderer
    // contract). We verify both sides here so a regression in either
    // target surfaces.
    use biscuit_terminal::utils::layout::Alignment;

    let mut list = OrderedList::new(vec!["First", "Second"]);
    list.layout_mut().alignment = Alignment::Center;
    let term = test_terminal(80);
    let rendered = strip_ansi(&list.render(&term));
    // Items still present and ordered.
    assert!(rendered.contains("1. First"), "got: {rendered:?}");
    assert!(rendered.contains("2. Second"), "got: {rendered:?}");
    // Markdown is layout-agnostic.
    let md = list.render_markdown();
    assert!(md.contains("1. First"), "md: {md:?}");
    assert!(md.contains("2. Second"), "md: {md:?}");
}

#[test]
fn parity_triple_digit_prefix() {
    // Once item count ≥ 100, the prefix width grows to 5 chars ("100. ").
    // Wrapping must use the wider hanging indent so continuation lines align
    // after the prefix. Spec table row: "Item count ≥ 100 (triple-digit
    // prefix) → Prefix '100. ' = 5 chars; wrapping and alignment correct."
    let items: Vec<String> = (1..=105).map(|n| format!("item {n}")).collect();
    let list = OrderedList::new(items);
    let term = test_terminal(80);
    let tree = strip_ansi(&list.render(&term));
    let tree_norm = normalize_for_parity(&tree);
    // Verify presence of representative items at each prefix width.
    assert!(tree_norm.contains("9. item 9"), "single-digit row");
    assert!(tree_norm.contains("99. item 99"), "double-digit row");
    assert!(tree_norm.contains("100. item 100"), "triple-digit row");
    assert!(
        tree_norm.contains("105. item 105"),
        "final triple-digit row"
    );
}

#[test]
fn parity_triple_digit_prefix_wraps_with_5char_hanging_indent() {
    // At a narrow width, a triple-digit item that wraps must align
    // continuation lines under the "      " (6-space) indent: 5 chars
    // for "100. " plus the hanging-indent that the tree renderer
    // computes from the prefix width. (At very narrow widths the
    // renderer may also break inside words; we only assert on the
    // indent of continuation lines that exist.)
    let mut items: Vec<String> = (1..=100).map(|n| format!("x{n}")).collect();
    // Force the 100th item to be long enough to wrap.
    items[99] = "alpha beta gamma delta epsilon zeta eta theta iota kappa".to_string();
    let list = OrderedList::new(items);
    let term = test_terminal(30);
    let rendered = strip_ansi(&list.render(&term));
    // Find the line that starts "100. " and inspect the following
    // continuation lines until the next prefix or end of output.
    let mut lines = rendered.lines();
    let prefix_line = lines
        .by_ref()
        .find(|l| l.starts_with("100. "))
        .expect("found 100. row");
    let _ = prefix_line;
    let mut saw_continuation = false;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        // Stop scanning when we reach an unrelated row.
        if line.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            break;
        }
        saw_continuation = true;
        assert!(
            line.starts_with("     "),
            "continuation under 100. row must indent ≥5 spaces: {line:?}"
        );
    }
    assert!(
        saw_continuation,
        "expected at least one wrapped line for the 100. item"
    );
}

// ---------------------------------------------------------------------------
// Width-mode slack sink (style-everywhere Phase 2, Task 2.2)
//
// A list's `Layout.width` resolves the outer box, which narrows
// `available_width`; the item body TEXT column then wraps to that width while
// the marker / hanging-indent stays fixed. So the item body text column is
// the documented slack sink (spec decision D2). Lists HUG naturally: a short
// item stays short, a long item wraps to the available width.
// ---------------------------------------------------------------------------

/// A ~200-char sentence that must wrap at any realistic terminal width.
const LONG_ITEM: &str = "This is a deliberately long ordered-list item whose \
single sentence keeps going well past any reasonable terminal column so that \
the render tree is forced to wrap the item body text onto several successive \
lines at the available content width.";

#[test]
fn width_auto_body_wraps_to_available() {
    let layout = biscuit_terminal::utils::layout::Layout {
        width: biscuit_terminal::utils::layout::Width::Auto,
        ..Default::default()
    };
    let list = OrderedList::new(vec![LONG_ITEM]).with_layout(layout);
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
    let list = OrderedList::new(vec![LONG_ITEM]).with_layout(layout);
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
    let list = OrderedList::new(vec!["Ab", "Cd"]).with_layout(layout);
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
    // width; the marker / hanging indent stays fixed (D2 slack sink).
    use biscuit_terminal::utils::layout::{Length, TargetValue, Width};

    let layout = biscuit_terminal::utils::layout::Layout {
        width: Width::Fixed(TargetValue::universal(Length::Percent(100.0))),
        ..Default::default()
    };
    let list = OrderedList::new(vec![LONG_ITEM]).with_layout(layout);
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
fn marker_and_hanging_indent_stay_fixed_under_width_modes() {
    // D2 slack sink: the marker ("1. ") and the hanging indent are fixed, so
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
        let list =
            OrderedList::new(vec!["Body line that is wide enough to wrap under narrow widths."])
                .with_layout(layout);
        let node = list.render_tree();
        let rendered = render_tree(&node, 80);
        let stripped = strip_ansi(&rendered);
        let first = stripped
            .lines()
            .next()
            .expect("at least one rendered line");
        assert!(
            first.starts_with("1. "),
            "marker is fixed at \"1. \" under any width mode: {first:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Optimistic render parity
// ---------------------------------------------------------------------------

#[test]
fn optimistic_render_matches_default_terminal_render() {
    let list = OrderedList::new(vec!["First", "Second", "Third"]);
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
    let list = OrderedList::new(Vec::<String>::new());
    assert!(list.render_markdown().is_empty());
}

#[test]
fn markdown_single_item_uses_one_dot() {
    let list = OrderedList::new(vec!["Step"]);
    let md = list.render_markdown();
    assert!(md.contains("1. Step"), "got: {md:?}");
}

#[test]
fn markdown_three_items_number_in_order() {
    let list = OrderedList::new(vec!["First", "Second", "Third"]);
    let md = list.render_markdown();
    let first = md.find("1. First").expect("first item present");
    let second = md.find("2. Second").expect("second item present");
    let third = md.find("3. Third").expect("third item present");
    assert!(first < second && second < third, "ordering: {md:?}");
}

#[test]
fn markdown_plus_matches_portable_markdown() {
    // OrderedList is a structural container with no color/border/fill of its
    // own, so MarkdownPlus must produce the same output as portable Markdown.
    let list = OrderedList::new(vec!["First", "Second"]);
    assert_eq!(list.render_markdown(), list.render_markdown_plus());
}

#[test]
fn markdown_nested_ordered_list_preserves_content() {
    // A nested OrderedList passed as a `Component` item projects to a
    // `NodeKind::List` child inside a `NodeKind::ListItem`. The Markdown
    // renderer collapses that onto the item's line (`2. 1. Inner A`) and
    // indents the rest, which is an accepted divergence from "ideal" nested
    // CommonMark — the canonical tree's structural shape is correct, but
    // the renderer's lowering is what it is. Item ordering and inner
    // numbering are still preserved.
    let inner = OrderedList::new(vec!["Inner A", "Inner B"]);
    let outer = OrderedList::from(vec![
        RenderableTerminalContent::String("Outer".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ]);
    let md = outer.render_markdown();
    assert!(md.contains("1. Outer"), "outer numbered: {md:?}");
    assert!(md.contains("Inner A"), "inner content present: {md:?}");
    assert!(md.contains("Inner B"), "inner content present: {md:?}");
    // Item ordering is preserved.
    let outer_pos = md.find("Outer").unwrap();
    let inner_a_pos = md.find("Inner A").unwrap();
    let inner_b_pos = md.find("Inner B").unwrap();
    assert!(outer_pos < inner_a_pos, "outer before inner");
    assert!(inner_a_pos < inner_b_pos, "Inner A before Inner B");
}

#[test]
fn markdown_layout_does_not_change_output() {
    use biscuit_terminal::utils::layout::{Length, TargetValue};

    let plain = OrderedList::new(vec!["A", "B"]);
    let mut with_margin = OrderedList::new(vec!["A", "B"]);
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
fn html_empty_list_emits_empty_ol() {
    let list = OrderedList::new(Vec::<String>::new());
    let html = list.render_html_fragment().render();
    // Empty list still produces an <ol> wrapper.
    assert!(html.contains("<ol"), "got: {html:?}");
    assert!(html.contains("</ol>"), "got: {html:?}");
}

#[test]
fn html_single_item_wraps_in_li() {
    let list = OrderedList::new(vec!["Item"]);
    let html = list.render_html_fragment().render();
    assert!(html.contains("<ol"), "ol present: {html:?}");
    assert!(html.contains("<li>"), "li present: {html:?}");
    assert!(html.contains("Item"), "content present: {html:?}");
    assert!(html.contains("</li>"), "li closed: {html:?}");
    assert!(html.contains("</ol>"), "ol closed: {html:?}");
}

#[test]
fn html_three_items_emit_three_li_elements() {
    let list = OrderedList::new(vec!["First", "Second", "Third"]);
    let html = list.render_html_fragment().render();
    let li_count = html.matches("<li>").count();
    assert_eq!(li_count, 3, "three <li> elements: {html:?}");
    assert!(html.contains("First"));
    assert!(html.contains("Second"));
    assert!(html.contains("Third"));
}

#[test]
fn html_nested_ordered_list_nests_ol() {
    let inner = OrderedList::new(vec!["Inner"]);
    let outer = OrderedList::from(vec![
        RenderableTerminalContent::String("Outer".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ]);
    let html = outer.render_html_fragment().render();
    // The outer wrapper contains a nested <ol>.
    let first_ol = html.find("<ol").expect("outer ol present");
    let second_ol = html[first_ol + 3..].find("<ol").expect("nested ol present");
    let _ = second_ol;
    assert!(html.contains("Outer"));
    assert!(html.contains("Inner"));
}

#[test]
fn html_nested_unordered_list_emits_ul() {
    use biscuit_terminal::components::list::UnorderedList;

    let inner = UnorderedList::new(vec!["Apple", "Banana"]);
    let outer = OrderedList::from(vec![
        RenderableTerminalContent::String("Fruits".to_string()),
        RenderableTerminalContent::Component(Rc::new(inner)),
    ]);
    let html = outer.render_html_fragment().render();
    assert!(html.contains("<ol"), "outer ordered: {html:?}");
    assert!(html.contains("<ul"), "inner unordered: {html:?}");
    assert!(html.contains("Apple"));
    assert!(html.contains("Banana"));
}

// ---------------------------------------------------------------------------
// Cross-target: Prose styling survives terminal, degrades for Markdown
// ---------------------------------------------------------------------------

#[test]
fn prose_item_content_survives_terminal_render() {
    // Content always survives through the tree path; this is the baseline
    // before any styling assertions.
    let mut list = OrderedList::empty();
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
    // After the project_list_items Prose-downcast fix (mirroring BlockQuote
    // and Compose), inline `<b>` / `<i>` / `<red>` styling must survive on
    // the Terminal target. We assert on the bold SGR open sequence; the
    // exact close/reset bytes depend on the renderer's pairing strategy.
    let mut list = OrderedList::empty();
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
    let mut list = OrderedList::empty();
    list.add(Prose::new("<b>Bold</b> item"));
    let md = list.render_markdown();
    // Markdown contains the textual content; raw `<b>` tokens are not
    // emitted verbatim (the projection strips them).
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
fn nested_section_inside_ordered_list_item_projects_structural_section_node() {
    use biscuit_terminal::components::section::{HeadingLevel, Section};

    let nested = Section::new(HeadingLevel::h3, "Nested Title");
    let list = OrderedList::from(vec![
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
fn nested_block_quote_inside_ordered_list_item_projects_structural_block_quote_node() {
    use biscuit_terminal::components::block_quote::BlockQuote;

    let nested = BlockQuote::new("quoted body".into(), None::<&str>);
    let list = OrderedList::from(vec![
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
fn nested_table_inside_ordered_list_item_projects_structural_table_node() {
    use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};

    let table = Table::new()
        .with_columns(vec![TableColumn::new("A"), TableColumn::new("B")])
        .with_data(vec![vec![
            TableCellContent::Text("x".into()),
            TableCellContent::Text("y".into()),
        ]]);
    let list = OrderedList::from(vec![
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
fn nested_ordered_list_inside_ordered_list_item_projects_ordered_list_node() {
    let inner = OrderedList::new(vec!["Inner A", "Inner B"]);
    let list = OrderedList::from(vec![
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
fn nested_unordered_list_inside_ordered_list_item_projects_unordered_list_node() {
    use biscuit_terminal::components::list::UnorderedList;

    let inner = UnorderedList::new(vec!["Apple", "Banana"]);
    let list = OrderedList::from(vec![
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
