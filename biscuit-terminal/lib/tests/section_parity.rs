//! Parity tests for the `Section` component's tree projection.
//!
//! These tests verify that `Section::render_tree_node()` produces a
//! well-formed `NodeKind::Section` tree, that the tree validates cleanly,
//! and that the tree-rendered terminal output is semantically equivalent to
//! the bespoke `Section::render()` output.

mod parity_helpers;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use renderable::tree::{
    NodeKind, RenderStrictness, ValidationMode, validate,
};

use parity_helpers::{PARITY_WIDTHS, assert_contains_tokens, strip_ansi, test_terminal};

/// Builds a section with a heading and two text content items.
fn sample_section() -> Section {
    let mut section = Section::new(HeadingLevel::h2, "Getting Started");
    section
        .push("Welcome to the tutorial.")
        .push("Let's begin with installation.");
    section
}

/// Renders a render-tree node to a terminal string at the given width.
fn render_tree(node: &renderable::tree::RenderNode, width: u32) -> String {
    let term = test_terminal(width);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    render_terminal_node(node, &opts)
        .expect("tree render should succeed")
        .output
}

// ---------------------------------------------------------------------------
// Structural snapshot
// ---------------------------------------------------------------------------

#[test]
fn render_tree_node_produces_section_kind() {
    let section = sample_section();
    let node = section
        .render_tree_node()
        .expect("Section should produce a tree node");

    match &node.kind {
        NodeKind::Section {
            depth,
            heading,
            children,
        } => {
            assert_eq!(depth.get(), 2, "h2 should map to depth 2");
            assert_eq!(heading.len(), 1, "heading holds a single text node");
            assert!(
                matches!(&heading[0].kind, NodeKind::Text { value } if value == "Getting Started"),
                "heading text should be the section title"
            );
            assert_eq!(children.len(), 2, "two content items project to two nodes");
        }
        other => panic!("expected NodeKind::Section, got {other:?}"),
    }
}

#[test]
fn render_tree_node_maps_all_heading_levels() {
    let cases = [
        (HeadingLevel::h1, 1u8),
        (HeadingLevel::h2, 2),
        (HeadingLevel::h3, 3),
        (HeadingLevel::h4, 4),
        (HeadingLevel::h5, 5),
        (HeadingLevel::h6, 6),
    ];
    for (level, expected_depth) in cases {
        let section = Section::new(level, "Title");
        let node = section.render_tree_node().expect("tree node");
        match &node.kind {
            NodeKind::Section { depth, .. } => {
                assert_eq!(depth.get(), expected_depth, "level {level:?} depth");
            }
            other => panic!("expected Section, got {other:?}"),
        }
    }
}

#[test]
fn render_tree_node_with_no_content_has_empty_children() {
    let section = Section::new(HeadingLevel::h1, "Solo Heading");
    let node = section.render_tree_node().expect("tree node");
    match &node.kind {
        NodeKind::Section { children, .. } => {
            assert!(children.is_empty(), "no content means no body children");
        }
        other => panic!("expected Section, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Validity
// ---------------------------------------------------------------------------

#[test]
fn projected_tree_validates_with_no_errors() {
    let section = sample_section();
    let node = section.render_tree_node().expect("tree node");
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "projected Section tree should validate cleanly: {:?}",
        report.errors().collect::<Vec<_>>()
    );
}

#[test]
fn projected_empty_section_validates() {
    let section = Section::new(HeadingLevel::h3, "Empty");
    let node = section.render_tree_node().expect("tree node");
    let report = validate(&node, ValidationMode::Full);
    assert!(
        !report.has_errors(),
        "empty Section should validate cleanly"
    );
}

// ---------------------------------------------------------------------------
// Semantic parity
// ---------------------------------------------------------------------------

#[test]
fn tree_output_contains_same_content_as_bespoke() {
    let section = sample_section();
    let term = test_terminal(80);
    let bespoke = section.render(&term);

    let node = section.render_tree_node().expect("tree node");
    let tree = render_tree(&node, 80);

    let bespoke_plain = strip_ansi(&bespoke);
    let tokens: Vec<&str> = bespoke_plain
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .collect();
    assert_contains_tokens(&tree, &tokens);
}

#[test]
fn tree_output_preserves_heading_prefix() {
    let section = Section::new(HeadingLevel::h2, "My Header");
    let node = section.render_tree_node().expect("tree node");
    let tree = render_tree(&node, 80);
    assert_contains_tokens(&tree, &["## My Header"]);
}

// ---------------------------------------------------------------------------
// Positional parity
// ---------------------------------------------------------------------------

#[test]
fn tree_output_places_heading_before_body() {
    let section = sample_section();
    let node = section.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));

    let heading_pos = plain
        .find("Getting Started")
        .expect("heading present");
    let first_body_pos = plain.find("Welcome").expect("first body item present");
    let second_body_pos = plain
        .find("installation")
        .expect("second body item present");

    assert!(
        heading_pos < first_body_pos,
        "heading must precede the first body item"
    );
    assert!(
        first_body_pos < second_body_pos,
        "body items must keep their order"
    );
}

#[test]
fn tree_output_keeps_body_items_separate() {
    let section = sample_section();
    let node = section.render_tree_node().expect("tree node");
    let plain = strip_ansi(&render_tree(&node, 80));

    // Both body items appear, and the heading line is distinct from the body.
    let lines: Vec<&str> = plain.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        lines.iter().any(|l| l.contains("Getting Started")),
        "heading line present"
    );
    assert!(
        lines.iter().any(|l| l.contains("Welcome")),
        "first body item present"
    );
    assert!(
        lines.iter().any(|l| l.contains("installation")),
        "second body item present"
    );
}

// ---------------------------------------------------------------------------
// Width matrix
// ---------------------------------------------------------------------------

#[test]
fn tree_output_renders_at_all_parity_widths() {
    for &width in PARITY_WIDTHS {
        let section = sample_section();
        let node = section.render_tree_node().expect("tree node");
        let tree = render_tree(&node, width);
        assert_contains_tokens(
            &tree,
            &["Getting Started", "Welcome", "installation"],
        );
    }
}
