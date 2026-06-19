//! Parity tests for the `OrderedList` / `UnorderedList` tree projection.
//!
//! These tests verify that `render_tree_node()` produces well-formed
//! `NodeKind::List` / `NodeKind::ListItem` trees, that those trees validate
//! cleanly, and that the tree-rendered terminal output is semantically
//! equivalent to the bespoke component output.

mod parity_helpers;

use biscuit_terminal::components::list::{OrderedList, UnorderedList};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use renderable::tree::{
    HeadingDepth, NodeKind, RenderNode, RenderStrictness, ValidationMode, validate,
};

use parity_helpers::{PARITY_WIDTHS, assert_contains_tokens, strip_ansi, test_terminal};

/// Renders a render-tree node to a terminal string at the given width.
fn render_tree(node: &RenderNode, width: u32) -> String {
    let term = test_terminal(width);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    render_terminal_node(node, &opts)
        .expect("tree render should succeed")
        .output
}

/// Builds a `ListItem` whose body is a single paragraph of text.
fn text_item(text: &str) -> RenderNode {
    RenderNode::list_item(
        None,
        vec![RenderNode::paragraph(vec![RenderNode::text(text)])],
    )
}

// ---------------------------------------------------------------------------
// Structural snapshot
// ---------------------------------------------------------------------------

#[test]
fn ordered_list_projects_to_list_kind() {
    let list = OrderedList::new(vec!["First", "Second", "Third"]);
    let node = list.render_tree_node().expect("ordered list tree node");
    match &node.kind {
        NodeKind::List {
            ordered,
            start,
            children,
        } => {
            assert!(ordered, "ordered list should be ordered");
            assert_eq!(*start, None, "default start is None");
            assert_eq!(children.len(), 3, "three items project to three children");
            for child in children {
                assert!(
                    matches!(child.kind, NodeKind::ListItem { .. }),
                    "each child is a ListItem"
                );
            }
        }
        other => panic!("expected NodeKind::List, got {other:?}"),
    }
}

#[test]
fn unordered_list_projects_to_list_kind() {
    let list = UnorderedList::new(vec!["Apple", "Banana"]);
    let node = list.render_tree_node().expect("unordered list tree node");
    match &node.kind {
        NodeKind::List {
            ordered, children, ..
        } => {
            assert!(!ordered, "unordered list should not be ordered");
            assert_eq!(children.len(), 2, "two items project to two children");
        }
        other => panic!("expected NodeKind::List, got {other:?}"),
    }
}

#[test]
fn unordered_list_records_custom_bullet_hint() {
    let list = UnorderedList::new(vec!["A"]).with_bullet("* ");
    let node = list.render_tree_node().expect("tree node");
    let hints = node.attrs.list_hints();
    assert_eq!(hints.bullet.as_deref(), Some("* "));
}

#[test]
fn unordered_list_omits_default_bullet_hint() {
    let list = UnorderedList::new(vec!["A"]);
    let node = list.render_tree_node().expect("tree node");
    assert_eq!(
        node.attrs.list_hints().bullet,
        None,
        "the default '- ' bullet is not recorded as a hint"
    );
}

#[test]
fn ordered_list_records_indent_children_hint() {
    let list = OrderedList::new(vec!["A"]).with_indent_children(8);
    let node = list.render_tree_node().expect("tree node");
    assert_eq!(node.attrs.list_hints().indent_children, Some(8));
}

#[test]
fn unordered_list_records_disabled_hanging_indent() {
    let list = UnorderedList::new(vec!["A"]).without_hanging_indent();
    let node = list.render_tree_node().expect("tree node");
    assert!(!node.attrs.list_hints().hanging_indent);
}

// ---------------------------------------------------------------------------
// Validity
// ---------------------------------------------------------------------------

#[test]
fn projected_ordered_tree_validates() {
    let list = OrderedList::new(vec!["First", "Second"]);
    let node = list.render_tree_node().expect("tree node");
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "projected ordered list should validate cleanly: {:?}",
        report.errors().collect::<Vec<_>>()
    );
}

#[test]
fn projected_unordered_tree_validates() {
    let list = UnorderedList::new(vec!["Apple", "Banana"]);
    let node = list.render_tree_node().expect("tree node");
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "projected unordered list should validate cleanly: {:?}",
        report.errors().collect::<Vec<_>>()
    );
}

#[test]
fn projected_empty_lists_validate() {
    let ordered = OrderedList::new(Vec::<String>::new());
    let unordered = UnorderedList::new(Vec::<String>::new());
    for node in [
        ordered.render_tree_node().expect("ordered tree node"),
        unordered.render_tree_node().expect("unordered tree node"),
    ] {
        let report = validate(&node, ValidationMode::Full);
        assert!(!report.has_errors(), "empty list should validate cleanly");
    }
}

// ---------------------------------------------------------------------------
// Semantic parity
// ---------------------------------------------------------------------------

#[test]
fn ordered_tree_output_matches_bespoke_content() {
    let list = OrderedList::new(vec!["First item", "Second item", "Third item"]);
    let node = list.render_tree_node().expect("tree node");
    let tree = render_tree(&node, 80);
    assert_contains_tokens(
        &tree,
        &["1.", "First item", "2.", "Second item", "3.", "Third item"],
    );
}

#[test]
fn unordered_tree_output_matches_bespoke_content() {
    let list = UnorderedList::new(vec!["Apple", "Banana", "Cherry"]);
    let node = list.render_tree_node().expect("tree node");
    let tree = render_tree(&node, 80);
    assert_contains_tokens(&tree, &["- Apple", "- Banana", "- Cherry"]);
}

// ---------------------------------------------------------------------------
// Positional parity
// ---------------------------------------------------------------------------

#[test]
fn ordered_tree_output_keeps_item_order() {
    let list = OrderedList::new(vec!["alpha", "beta", "gamma"]);
    let node = list.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));

    let a = plain.find("alpha").expect("alpha present");
    let b = plain.find("beta").expect("beta present");
    let g = plain.find("gamma").expect("gamma present");
    assert!(a < b && b < g, "items keep source order");
}

#[test]
fn unordered_tree_output_aligns_bullets() {
    let list = UnorderedList::new(vec!["one", "two", "three"]);
    let node = list.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));
    for line in plain.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            line.starts_with("- "),
            "every line starts with a bullet: {line:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Ordered prefix-width transitions
// ---------------------------------------------------------------------------

#[test]
fn ordered_prefix_width_crosses_nine_to_ten() {
    let items: Vec<String> = (1..=12).map(|n| format!("item {n}")).collect();
    let list = OrderedList::new(items);
    let node = list.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains("9. item 9"), "single-digit prefix");
    assert!(plain.contains("10. item 10"), "double-digit prefix");
    assert!(plain.contains("12. item 12"), "last item numbered");
}

#[test]
fn ordered_prefix_width_crosses_ninety_nine_to_hundred() {
    // Project an ordered list whose numbering reaches three digits.
    let children: Vec<RenderNode> = (0..3).map(|n| text_item(&format!("v{n}"))).collect();
    let node = RenderNode::list(true, Some(98), children);
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains("98. v0"), "two-digit prefix");
    assert!(plain.contains("99. v1"), "two-digit prefix");
    assert!(plain.contains("100. v2"), "three-digit prefix");
}

#[test]
fn ordered_list_honors_non_default_start() {
    for start in [5u64, 99] {
        let children = vec![text_item("x"), text_item("y")];
        let node = RenderNode::list(true, Some(start), children);
        let plain = strip_ansi(&render_tree(&node, 80));
        assert!(
            plain.contains(&format!("{start}. x")),
            "start={start} first"
        );
        assert!(
            plain.contains(&format!("{}. y", start + 1)),
            "start={start} second"
        );
    }
}

// ---------------------------------------------------------------------------
// Custom bullets
// ---------------------------------------------------------------------------

#[test]
fn custom_ascii_bullet_renders() {
    let list = UnorderedList::new(vec!["one", "two"]).with_bullet("* ");
    let node = list.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains("* one"), "custom '*' bullet on first item");
    assert!(plain.contains("* two"), "custom '*' bullet on second item");
}

#[test]
fn custom_unicode_bullet_renders() {
    let list = UnorderedList::new(vec!["alpha"]).with_bullet("→ ");
    let node = list.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains("→ alpha"), "unicode arrow bullet");
}

// ---------------------------------------------------------------------------
// Checked items
// ---------------------------------------------------------------------------

#[test]
fn checked_item_renders_x_marker() {
    let children = vec![RenderNode::list_item(
        Some(true),
        vec![RenderNode::paragraph(vec![RenderNode::text("done")])],
    )];
    let node = RenderNode::list(false, None, children);
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains("[x] done"), "checked marker: {plain:?}");
}

#[test]
fn unchecked_item_renders_empty_marker() {
    let children = vec![RenderNode::list_item(
        Some(false),
        vec![RenderNode::paragraph(vec![RenderNode::text("todo")])],
    )];
    let node = RenderNode::list(false, None, children);
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains("[ ] todo"), "unchecked marker: {plain:?}");
}

#[test]
fn mixed_checked_items_render_distinct_markers() {
    let children = vec![
        RenderNode::list_item(
            Some(true),
            vec![RenderNode::paragraph(vec![RenderNode::text("first")])],
        ),
        RenderNode::list_item(
            Some(false),
            vec![RenderNode::paragraph(vec![RenderNode::text("second")])],
        ),
        text_item("third"),
    ];
    let node = RenderNode::list(false, None, children);
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains("[x] first"), "checked");
    assert!(plain.contains("[ ] second"), "unchecked");
    assert!(plain.contains("- third"), "plain item keeps bare bullet");
}

// ---------------------------------------------------------------------------
// Nested block children
// ---------------------------------------------------------------------------

#[test]
fn list_item_with_block_children_indents_without_double_bullet() {
    // An item containing a paragraph followed by a code block.
    let item = RenderNode::list_item(
        None,
        vec![
            RenderNode::paragraph(vec![RenderNode::text("Steps:")]),
            RenderNode::code(Some("rust".into()), None, "let a = 1;"),
        ],
    );
    let node = RenderNode::list(false, None, vec![item]);
    let plain = strip_ansi(&render_tree(&node, 80));

    // The paragraph carries the bullet.
    assert!(
        plain.contains("- Steps:"),
        "paragraph carries bullet: {plain:?}"
    );
    // The code block is indented, not bulleted.
    let code_line = plain
        .lines()
        .find(|l| l.contains("let a = 1;"))
        .expect("code line present");
    assert!(
        code_line.starts_with("  "),
        "code block indented: {code_line:?}"
    );
    assert!(
        !code_line.trim_start().starts_with('-'),
        "code block must not be double-bulleted: {code_line:?}"
    );
}

#[test]
fn ordered_list_item_with_block_child_indents_by_four() {
    let item = RenderNode::list_item(
        None,
        vec![
            RenderNode::paragraph(vec![RenderNode::text("Outer")]),
            RenderNode::list(
                false,
                None,
                vec![text_item("nested a"), text_item("nested b")],
            ),
        ],
    );
    let node = RenderNode::list(true, None, vec![item]);
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(plain.contains("1. Outer"), "outer item numbered");
    let nested = plain
        .lines()
        .find(|l| l.contains("nested a"))
        .expect("nested line present");
    assert!(
        nested.starts_with("    "),
        "ordered block child indented by 4: {nested:?}"
    );
}

// ---------------------------------------------------------------------------
// Hanging indent
// ---------------------------------------------------------------------------

#[test]
fn disabled_hanging_indent_survives_projection_and_render() {
    let list = UnorderedList::new(vec![
        "a long item that should wrap across more than one terminal line for sure",
    ])
    .without_hanging_indent();
    let node = list.render_tree_node().expect("tree node");
    assert!(!node.attrs.list_hints().hanging_indent, "hint preserved");

    let plain = strip_ansi(&render_tree(&node, 30));
    let lines: Vec<&str> = plain.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        lines.len() > 1,
        "content should wrap at width 30: {lines:?}"
    );
    for line in &lines[1..] {
        assert!(
            !line.starts_with(' '),
            "continuation lines are not indented when hanging indent is off: {line:?}"
        );
    }
}

#[test]
fn hanging_indent_aligns_continuation_after_bullet() {
    let list = UnorderedList::new(vec![
        "another fairly long item that wraps onto a second line at narrow width",
    ]);
    let node = list.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 30));
    let lines: Vec<&str> = plain.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() > 1, "content should wrap: {lines:?}");
    assert!(lines[0].starts_with("- "), "first line carries bullet");
    for line in &lines[1..] {
        assert!(
            line.starts_with("  "),
            "continuation aligns after the 2-char bullet: {line:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Width matrix
// ---------------------------------------------------------------------------

#[test]
fn lists_render_at_all_parity_widths() {
    for &width in PARITY_WIDTHS {
        let ordered = OrderedList::new(vec!["First", "Second"]);
        let onode = ordered.render_tree_node().expect("ordered tree node");
        assert_contains_tokens(&render_tree(&onode, width), &["First", "Second"]);

        let unordered = UnorderedList::new(vec!["Apple", "Banana"]);
        let unode = unordered.render_tree_node().expect("unordered tree node");
        assert_contains_tokens(&render_tree(&unode, width), &["Apple", "Banana"]);
    }
}

#[test]
fn nested_section_with_list_validates_and_renders() {
    // A list embedded in a section body — exercises the projection chain.
    let list_node = RenderNode::list(
        true,
        None,
        vec![text_item("step one"), text_item("step two")],
    );
    let section = RenderNode::section(
        HeadingDepth::new(2).unwrap(),
        vec![RenderNode::text("Instructions")],
        vec![list_node],
    );
    let report = validate(&section, ValidationMode::Full);
    assert!(!report.has_errors(), "section with list validates");

    let plain = strip_ansi(&render_tree(&section, 80));
    assert_contains_tokens(
        &plain,
        &["Instructions", "1.", "step one", "2.", "step two"],
    );
}
