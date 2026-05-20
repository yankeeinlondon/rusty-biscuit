//! Tree-path tests for the `Section` component's projection.
//!
//! These tests verify that `Section::render_tree_node()` produces a
//! well-formed `NodeKind::Section` tree, that the tree validates cleanly,
//! and that the tree-rendered terminal output preserves heading and body
//! tokens at every supported width.

mod parity_helpers;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use renderable::tree::{NodeKind, RenderNode, RenderStrictness, ValidationMode, validate};

/// Walks a `RenderNode` tree depth-first looking for a node whose `NodeKind`
/// matches `pred`. Used by nested-component structural assertions.
fn walk_has_kind(node: &RenderNode, pred: impl Fn(&NodeKind) -> bool + Copy) -> bool {
    if pred(&node.kind) {
        return true;
    }
    node.children().iter().any(|c| walk_has_kind(c, pred))
}

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

    let heading_pos = plain.find("Getting Started").expect("heading present");
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
        assert_contains_tokens(&tree, &["Getting Started", "Welcome", "installation"]);
    }
}

// ---------------------------------------------------------------------------
// TreeRenderable canonical adoption
// ---------------------------------------------------------------------------

#[test]
fn tree_renderable_and_render_tree_node_share_one_projection() {
    // The migration pattern: the canonical `TreeRenderable::render_tree`
    // entry point and the terminal compatibility
    // `TerminalRenderable::render_tree_node` hook MUST share one private
    // projection helper so they cannot drift.
    use renderable::tree::TreeRenderable;
    let section = sample_section();
    let canonical = <Section as TreeRenderable>::render_tree(&section);
    let compat = section.render_tree_node().expect("tree node");
    assert_eq!(
        serde_json::to_value(&canonical).unwrap(),
        serde_json::to_value(&compat).unwrap(),
        "canonical and compatibility projections must serialize identically"
    );
}

// ---------------------------------------------------------------------------
// Tree-path heading style coverage
// ---------------------------------------------------------------------------

#[test]
fn heading_styles_survive_tree_render() {
    // h1-h3 use bold, h4-h5 italic, h6 plain.
    let term = test_terminal(80);
    let bold_levels = [HeadingLevel::h1, HeadingLevel::h2, HeadingLevel::h3];
    for level in bold_levels {
        let section = Section::new(level, "Title");
        let out = section.render(&term);
        assert!(
            out.contains("\x1b[1m"),
            "level {level:?} should carry bold SGR: {out:?}"
        );
    }
    let italic_levels = [HeadingLevel::h4, HeadingLevel::h5];
    for level in italic_levels {
        let section = Section::new(level, "Title");
        let out = section.render(&term);
        assert!(
            out.contains("\x1b[3m"),
            "level {level:?} should carry italic SGR: {out:?}"
        );
    }
    let h6_section = Section::new(HeadingLevel::h6, "Title");
    let h6_out = h6_section.render(&term);
    assert!(
        !h6_out.contains("\x1b[1m") && !h6_out.contains("\x1b[3m"),
        "h6 should not carry bold or italic SGR: {h6_out:?}"
    );
}

// ---------------------------------------------------------------------------
// Layout flows through the tree path
// ---------------------------------------------------------------------------

#[test]
fn left_margin_is_honored_through_tree_path() {
    use biscuit_terminal::utils::layout::{Length, TargetValue};
    let mut section = Section::new(HeadingLevel::h1, "Title");
    section.layout_mut().margin.left = TargetValue::universal(Length::ch(4));
    let term = test_terminal(80);
    let out = strip_ansi(&section.render(&term));
    assert!(
        out.lines().next().is_some_and(|first| first.starts_with("    ")),
        "left margin of 4 spaces applied through tree: {out:?}"
    );
}

// ---------------------------------------------------------------------------
// Markdown renderable
// ---------------------------------------------------------------------------

#[test]
fn markdown_renders_heading_and_body() {
    use renderable::markdown::MarkdownRenderable;
    let section = sample_section();
    let md = section.render_markdown();
    assert!(md.contains("## Getting Started"), "heading prefix: {md:?}");
    assert!(md.contains("Welcome to the tutorial."), "body item 1: {md:?}");
    assert!(
        md.contains("Let's begin with installation."),
        "body item 2: {md:?}"
    );
}

#[test]
fn markdown_empty_section_is_just_heading() {
    use renderable::markdown::MarkdownRenderable;
    let section = Section::new(HeadingLevel::h1, "Solo");
    let md = section.render_markdown();
    assert_eq!(md.trim(), "# Solo");
}

#[test]
fn markdown_heading_level_maps_to_hash_count() {
    use renderable::markdown::MarkdownRenderable;
    let cases = [
        (HeadingLevel::h1, "# "),
        (HeadingLevel::h2, "## "),
        (HeadingLevel::h3, "### "),
        (HeadingLevel::h4, "#### "),
        (HeadingLevel::h5, "##### "),
        (HeadingLevel::h6, "###### "),
    ];
    for (level, expected_prefix) in cases {
        let section = Section::new(level, "Title");
        let md = section.render_markdown();
        assert!(
            md.starts_with(expected_prefix),
            "level {level:?} prefix: {md:?}"
        );
    }
}

#[test]
fn markdown_and_markdown_plus_match_for_pure_section() {
    // Section's structure is pure CommonMark — Markdown and MarkdownPlus
    // should produce identical output when content is plain text.
    use renderable::markdown::MarkdownRenderable;
    let section = sample_section();
    assert_eq!(section.render_markdown(), section.render_markdown_plus());
}

#[test]
fn markdown_ignores_layout_by_contract() {
    // Layout is intentionally not lowered to Markdown output — CommonMark
    // has no portable layout primitive.
    use biscuit_terminal::utils::layout::{Length, TargetValue};
    use renderable::markdown::MarkdownRenderable;
    let mut section = Section::new(HeadingLevel::h2, "Header");
    section.push("Body.");
    let without_layout = section.render_markdown();
    section.layout_mut().margin.left = TargetValue::universal(Length::ch(8));
    let with_layout = section.render_markdown();
    assert_eq!(
        without_layout, with_layout,
        "layout must not change Markdown output"
    );
}

// ---------------------------------------------------------------------------
// Browser renderable
// ---------------------------------------------------------------------------

#[test]
fn browser_emits_section_element_with_heading_tag() {
    use renderable::browser::BrowserRenderable;
    let section = sample_section();
    let html = section.render_html_fragment().render();
    assert!(html.contains("<section"), "section element: {html:?}");
    assert!(html.contains("</section>"), "section close: {html:?}");
    assert!(html.contains("<h2"), "h2 heading tag: {html:?}");
    assert!(html.contains("Getting Started"), "title text: {html:?}");
}

#[test]
fn browser_heading_tag_tracks_level() {
    use renderable::browser::BrowserRenderable;
    let cases = [
        (HeadingLevel::h1, "<h1"),
        (HeadingLevel::h2, "<h2"),
        (HeadingLevel::h3, "<h3"),
        (HeadingLevel::h4, "<h4"),
        (HeadingLevel::h5, "<h5"),
        (HeadingLevel::h6, "<h6"),
    ];
    for (level, expected_tag) in cases {
        let section = Section::new(level, "Title");
        let html = section.render_html_fragment().render();
        assert!(
            html.contains(expected_tag),
            "level {level:?} should emit {expected_tag}: {html:?}"
        );
    }
}

#[test]
fn browser_empty_section_omits_body_paragraph() {
    use renderable::browser::BrowserRenderable;
    let section = Section::new(HeadingLevel::h1, "Solo");
    let html = section.render_html_fragment().render();
    assert!(html.contains("<h1"), "heading: {html:?}");
    assert!(html.contains("Solo"), "title: {html:?}");
    assert!(
        !html.contains("<p>") && !html.contains("<p "),
        "no paragraph in empty body: {html:?}"
    );
}

// ---------------------------------------------------------------------------
// Prose content: structured inline projection (not flattened text)
// ---------------------------------------------------------------------------

/// A `Section` pushed a `Prose` child must project that child as **structured
/// inline nodes** (Strong/Emphasis/Span — whatever the prose markup demands),
/// not as a flat `Text` blob. This protects against an accidental regression
/// to the ANSI-strip-and-flatten fallback that the BlockQuote / Compose /
/// OrderedList / UnorderedList migrations consolidated into
/// `project_renderable_content` with `ProjectionMode::Structural`.
#[test]
fn render_tree_node_projects_prose_child_as_structured_inline() {
    use biscuit_terminal::components::prose::Prose;
    use renderable::tree::NodeKind;

    let mut section = Section::new(HeadingLevel::h2, "Styled");
    section.push(Prose::new("<bold>Bold</bold> tail"));

    let node = section.render_tree_node().expect("tree node");
    let children = match &node.kind {
        NodeKind::Section { children, .. } => children,
        other => panic!("expected NodeKind::Section, got {other:?}"),
    };

    // The structural projection must not flatten Prose to a single Text node.
    // We expect at least one non-Text inline node corresponding to the
    // `<bold>Bold</bold>` segment — Strong is the canonical mapping for pure
    // bold emphasis, but Span-with-bold-style would also be acceptable; we
    // assert "not flat text" rather than pinning a specific inline kind.
    let has_structured_inline = children
        .iter()
        .any(|child| !matches!(child.kind, NodeKind::Text { .. }));
    assert!(
        has_structured_inline,
        "Prose child must project to structured inline nodes, not flat text: {children:?}"
    );

    // And the literal "Bold" text must still appear somewhere in the
    // projection so the structural assertion above is not a false-positive
    // for an empty inline wrapper.
    let plain = strip_ansi(&render_tree(&node, 80));
    assert!(
        plain.contains("Bold"),
        "Prose content text must survive projection: {plain:?}"
    );
    assert!(
        plain.contains("tail"),
        "Prose trailing text must survive projection: {plain:?}"
    );
}

/// The terminal target must lower the structured inline projection back to
/// SGR — a `<bold>` segment inside the section's `Prose` child should appear
/// as a bold-open escape (`\x1b[1m`) in the terminal output, not as plain
/// text.
#[test]
fn prose_child_inline_emphasis_survives_terminal_render() {
    use biscuit_terminal::components::prose::Prose;

    let mut section = Section::new(HeadingLevel::h2, "Styled");
    section.push(Prose::new("<bold>Bold</bold> tail"));

    let term = test_terminal(80);
    let out = section.render(&term);
    assert!(
        out.contains("\x1b[1m"),
        "Prose bold emphasis must lower to SGR in terminal output: {out:?}"
    );
}

// ---------------------------------------------------------------------------
// Nested Component content: block structure preserved across targets
// ---------------------------------------------------------------------------

/// A `Section` pushed a nested block component (here, another `Section`) must
/// carry that child's canonical block structure into the projected tree
/// rather than flattening it to plain text. This validates the
/// `ProjectionMode::Structural` path for non-Prose block-capable children.
#[test]
fn render_tree_node_preserves_nested_block_component_structure() {
    use renderable::tree::NodeKind;

    let nested = Section::new(HeadingLevel::h3, "Subsection");
    let mut outer = Section::new(HeadingLevel::h2, "Outer");
    outer.push(nested);

    let node = outer.render_tree_node().expect("tree node");
    let children = match &node.kind {
        NodeKind::Section { children, .. } => children,
        other => panic!("expected NodeKind::Section, got {other:?}"),
    };

    // The nested section must appear as a structured `Section` child (with
    // its own depth and heading), not as a flat text node.
    let nested_kind = children
        .iter()
        .find_map(|child| match &child.kind {
            NodeKind::Section { depth, heading, .. } => Some((depth.get(), heading.clone())),
            _ => None,
        })
        .expect("nested Section must appear as a Section child");

    assert_eq!(nested_kind.0, 3, "nested section depth preserved");
    assert!(
        matches!(&nested_kind.1[0].kind, NodeKind::Text { value } if value == "Subsection"),
        "nested heading text preserved: {:?}",
        nested_kind.1
    );
}

// ---------------------------------------------------------------------------
// Stage 3a: structural projection of additional nested block components
// (BlockQuote-in-Section, Table-in-Section). These tests pin that the
// canonical tree carries the nested component's `NodeKind` rather than
// flattening to text.
// ---------------------------------------------------------------------------

#[test]
fn nested_block_quote_in_section_projects_structural_block_quote_node() {
    use biscuit_terminal::components::block_quote::BlockQuote;

    let mut outer = Section::new(HeadingLevel::h2, "Quote Holder");
    outer.push(BlockQuote::new("nested quoted".into(), None::<&str>));

    let node = outer.render_tree_node().expect("tree node");
    let children = match &node.kind {
        NodeKind::Section { children, .. } => children,
        other => panic!("expected NodeKind::Section, got {other:?}"),
    };

    assert!(
        children
            .iter()
            .any(|c| walk_has_kind(c, |k| matches!(k, NodeKind::BlockQuote { .. }))),
        "nested BlockQuote must appear as a structural BlockQuote node: {children:?}"
    );
}

#[test]
fn nested_table_in_section_projects_structural_table_node() {
    use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};

    let table = Table::new()
        .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Value")])
        .with_data(vec![vec![
            TableCellContent::Text("alpha".into()),
            TableCellContent::Text("beta".into()),
        ]]);

    let mut outer = Section::new(HeadingLevel::h2, "Table Holder");
    outer.push(table);

    let node = outer.render_tree_node().expect("tree node");
    let children = match &node.kind {
        NodeKind::Section { children, .. } => children,
        other => panic!("expected NodeKind::Section, got {other:?}"),
    };

    assert!(
        children
            .iter()
            .any(|c| walk_has_kind(c, |k| matches!(k, NodeKind::Table { .. }))),
        "nested Table must appear as a structural Table node: {children:?}"
    );
}

#[test]
fn nested_block_component_renders_across_all_targets() {
    use renderable::browser::BrowserRenderable;
    use renderable::markdown::MarkdownRenderable;

    let nested = Section::new(HeadingLevel::h3, "Subsection");
    let mut outer = Section::new(HeadingLevel::h2, "Outer");
    outer.push(nested);

    // Terminal — both headings appear with their respective Markdown-style
    // prefixes (`##` and `###`).
    let term = test_terminal(80);
    let terminal_out = strip_ansi(&outer.render(&term));
    assert!(
        terminal_out.contains("## Outer"),
        "outer heading present in terminal: {terminal_out:?}"
    );
    assert!(
        terminal_out.contains("### Subsection"),
        "nested heading present in terminal: {terminal_out:?}"
    );

    // Markdown — same hash prefixes, separated by blank lines.
    let md = outer.render_markdown();
    assert!(md.contains("## Outer"), "outer heading in md: {md:?}");
    assert!(
        md.contains("### Subsection"),
        "nested heading in md: {md:?}"
    );

    // Browser — semantic `<h2>` and `<h3>` tags inside `<section>` elements.
    let html = outer.render_html_fragment().render();
    assert!(html.contains("<section"), "section element: {html:?}");
    assert!(html.contains("<h2"), "outer h2 tag: {html:?}");
    assert!(html.contains("<h3"), "nested h3 tag: {html:?}");
    assert!(html.contains("Outer"), "outer title: {html:?}");
    assert!(html.contains("Subsection"), "nested title: {html:?}");
}
