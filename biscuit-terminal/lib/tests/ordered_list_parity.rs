//! Parity tests for `OrderedList` after its render-tree migration.
//!
//! These tests assert that:
//!
//! 1. `OrderedList::render(&term)` (the user-facing path) routes through the
//!    canonical render tree.
//! 2. The legacy bespoke output (preserved on
//!    [`OrderedList::render_bespoke`]) and the tree output agree on semantic
//!    content (ANSI-stripped, whitespace-normalized).
//! 3. The single private projection helper drives both
//!    [`TreeRenderable::render_tree`] and
//!    [`TerminalRenderable::render_tree_node`] so the two entry points cannot
//!    drift.
//! 4. `MarkdownRenderable` and `BrowserRenderable` impls produce the expected
//!    CommonMark / HTML output, including layout-driven CSS where applicable.
//!
//! ## `KNOWN_DRIFT` ledger
//!
//! The following divergences are accepted at the byte level and tested
//! semantically instead:
//!
//! - **Trailing newline**: the bespoke path emits a trailing `"\n"` after the
//!   last item; the tree renderer does not. Both forms carry the same item
//!   content.
//! - **Empty nested list blank line**: the bespoke path emits a true blank
//!   line (`""`); the tree renderer emits a line padded to the indent width
//!   (`"    "`). Both preserve the visual gap between items.
//! - **Prose styling loss**: items that are `Prose` components keep their
//!   *content* through the tree path but lose inline styling (`<b>` /
//!   `<red>`) because `Prose::render_tree_node()` returns `None`, so the
//!   generic `RenderableTerminalContent::to_tree_nodes` fallback strips
//!   ANSI and embeds plain text. Recovering Prose styling requires the
//!   downcast pattern that `BlockQuote` / `Compose` use and is tracked
//!   separately to keep this migration surgical.

mod parity_helpers;

use biscuit_terminal::components::list::OrderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{
    BrowserRenderable, RenderableTerminalContent, TerminalRenderable,
};
use renderable::markdown::MarkdownRenderable;
use renderable::tree::TreeRenderable;
use std::rc::Rc;

use parity_helpers::{assert_semantic_match, strip_ansi, test_terminal};

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
// Bespoke vs tree parity (default render path now uses tree)
// ---------------------------------------------------------------------------

/// Strips ANSI escape codes and trailing whitespace from each line, then
/// drops trailing blank lines. Both the bespoke and tree paths produce the
/// same item content under this lens, modulo the accepted
/// trailing-newline drift documented at the top of this file.
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

#[test]
fn parity_empty_list() {
    let list = OrderedList::new(Vec::<String>::new());
    let term = test_terminal(80);
    assert_eq!(normalize_for_parity(&list.render_bespoke(&term)), "");
    assert_eq!(normalize_for_parity(&list.render(&term)), "");
}

#[test]
fn parity_single_string_item() {
    let list = OrderedList::new(vec!["Item"]);
    let term = test_terminal(80);
    let bespoke = normalize_for_parity(&list.render_bespoke(&term));
    let tree = normalize_for_parity(&list.render(&term));
    assert_semantic_match(&tree, &bespoke);
    assert_eq!(tree, "1. Item");
}

#[test]
fn parity_three_string_items() {
    let list = OrderedList::new(vec!["First", "Second", "Third"]);
    let term = test_terminal(80);
    let bespoke = normalize_for_parity(&list.render_bespoke(&term));
    let tree = normalize_for_parity(&list.render(&term));
    assert_semantic_match(&tree, &bespoke);
    assert!(tree.contains("1. First"));
    assert!(tree.contains("2. Second"));
    assert!(tree.contains("3. Third"));
}

#[test]
fn parity_double_digit_prefix() {
    let items: Vec<String> = (1..=12).map(|n| format!("item {n}")).collect();
    let list = OrderedList::new(items);
    let term = test_terminal(80);
    let bespoke = normalize_for_parity(&list.render_bespoke(&term));
    let tree = normalize_for_parity(&list.render(&term));
    assert!(tree.contains("9. item 9"));
    assert!(tree.contains("10. item 10"));
    assert!(tree.contains("12. item 12"));
    assert_semantic_match(&tree, &bespoke);
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
    // KNOWN_DRIFT: Per the OrderedList spec, items that are `Prose`
    // components lose styling through the tree path because
    // `Prose::render_tree_node()` returns `None`, so
    // `RenderableTerminalContent::to_tree_nodes` falls back to
    // ANSI-stripped plain text. Content is preserved; inline `<b>` / color
    // styling is not. Recovering styling requires a downcast in the
    // OrderedList projection — tracked separately to keep the migration
    // surgical.
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
fn prose_inline_styling_degrades_in_markdown() {
    let mut list = OrderedList::empty();
    list.add(Prose::new("<b>Bold</b> item"));
    let md = list.render_markdown();
    // Markdown contains the textual content; raw `<b>` tokens are not
    // emitted verbatim (the projection strips them).
    assert!(md.contains("Bold"), "content present: {md:?}");
    assert!(!md.contains("<b>"), "raw style tokens stripped: {md:?}");
}
