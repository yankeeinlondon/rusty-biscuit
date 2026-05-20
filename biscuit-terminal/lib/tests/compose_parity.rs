//! Canonical trait parity and nested-component coverage for [`Compose`].
//!
//! Two layers of assertion live in this file:
//!
//! 1. **Tree/hook delegation parity** — [`TreeRenderable::render_tree`] and
//!    the terminal compatibility hook
//!    [`TerminalRenderable::render_tree_node`] must return the same
//!    [`RenderNode`]. The override in `compose.rs` delegates directly to
//!    `<Self as TreeRenderable>::render_tree(self)`, so any future drift
//!    (e.g. someone adding post-processing in only one path) will be caught
//!    by these assertions.
//!
//! 2. **Stage 3a structural nested-component coverage** — every component
//!    that the spec marks as a valid `Compose` part must project to the
//!    expected top-level [`NodeKind`] inside the `Compose` root. These tests
//!    pin the Structural-mode contract that
//!    [`Compose::render_tree`](biscuit_terminal::components::compose::Compose)
//!    inherits via `project_renderable_content(.., ProjectionMode::Structural)`.

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::list::{OrderedList, UnorderedList};
use biscuit_terminal::components::progress::Progress;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::components::text_block::TextBlock;
use biscuit_terminal::components::todo::Todo;
use biscuit_terminal::components::two_column::TwoColumn;
use renderable::tree::{NodeKind, RenderNode, TreeRenderable};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Walks a `RenderNode` tree depth-first looking for a node whose `NodeKind`
/// matches `pred`. Used by nested-component structural assertions.
fn walk_has_kind(node: &RenderNode, pred: impl Fn(&NodeKind) -> bool + Copy) -> bool {
    if pred(&node.kind) {
        return true;
    }
    node.children().iter().any(|c| walk_has_kind(c, pred))
}

/// Returns the top-level children of the `Compose` root node.
fn compose_root_children(node: &RenderNode) -> &[RenderNode] {
    match &node.kind {
        NodeKind::Root { children } => children,
        other => panic!("expected NodeKind::Root, got {other:?}"),
    }
}

/// Builds a `Compose` containing the given component as its only part and
/// returns the projected tree.
fn compose_with<C: TerminalRenderable + 'static>(component: C) -> RenderNode {
    let compose = Compose::new(vec![RenderableTerminalContent::from(component)]);
    <Compose as TreeRenderable>::render_tree(&compose)
}

// ---------------------------------------------------------------------------
// Tree/hook delegation parity
// ---------------------------------------------------------------------------

#[test]
fn render_tree_node_matches_render_tree_for_empty_compose() {
    let compose = Compose::default();
    let from_tree = <Compose as TreeRenderable>::render_tree(&compose);
    let from_hook = <Compose as TerminalRenderable>::render_tree_node(&compose)
        .expect("render_tree_node should return Some");
    assert_eq!(
        from_tree, from_hook,
        "Compose::render_tree_node must delegate to render_tree (empty)"
    );
}

#[test]
fn render_tree_node_matches_render_tree_for_text_only_compose() {
    let mut compose = Compose::default();
    compose.add_text("Hello, ").add_text("world!");
    let from_tree = <Compose as TreeRenderable>::render_tree(&compose);
    let from_hook = <Compose as TerminalRenderable>::render_tree_node(&compose)
        .expect("render_tree_node should return Some");
    assert_eq!(
        from_tree, from_hook,
        "Compose::render_tree_node must delegate to render_tree (text-only)"
    );
}

#[test]
fn render_tree_node_matches_render_tree_for_mixed_compose() {
    let compose = Compose::new(vec![
        RenderableTerminalContent::from("Intro: "),
        RenderableTerminalContent::from(Prose::new("<bold>important</bold> bits")),
        RenderableTerminalContent::from(" and a trailing string."),
    ]);
    let from_tree = <Compose as TreeRenderable>::render_tree(&compose);
    let from_hook = <Compose as TerminalRenderable>::render_tree_node(&compose)
        .expect("render_tree_node should return Some");
    assert_eq!(
        from_tree, from_hook,
        "Compose::render_tree_node must delegate to render_tree (mixed parts)"
    );
}

// ---------------------------------------------------------------------------
// Stage 3a: nested-component fixture coverage
//
// Each fixture in the spec table at
// `renderable/features/2026-05-19-pushing-toward-ir/stage3-spec.md` (line ~208)
// asserts the expected top-level `NodeKind` projected inside the `Compose`
// root for the listed component. The `FileSystem` row is excluded by the
// spec (no idiomatic `Compose` fixture).
// ---------------------------------------------------------------------------

#[test]
fn compose_block_quote_projects_block_quote_node() {
    let node = compose_with(BlockQuote::new("quoted".into(), None::<&str>));
    let children = compose_root_children(&node);
    assert!(
        children
            .iter()
            .any(|c| walk_has_kind(c, |k| matches!(k, NodeKind::BlockQuote { .. }))),
        "Compose<BlockQuote> must carry a structural BlockQuote child: {children:?}"
    );
}

#[test]
fn compose_section_projects_section_node() {
    let mut section = Section::new(HeadingLevel::h2, "Title");
    section.push("body");
    let node = compose_with(section);
    let children = compose_root_children(&node);
    assert!(
        children.iter().any(|c| walk_has_kind(c, |k| {
            matches!(k, NodeKind::Section { depth, .. } if depth.get() == 2)
        })),
        "Compose<Section> must carry a structural Section child at depth 2: {children:?}"
    );
}

#[test]
fn compose_table_projects_table_node() {
    let table = Table::new()
        .with_columns(vec![TableColumn::new("A"), TableColumn::new("B")])
        .with_data(vec![vec![
            TableCellContent::Text("x".into()),
            TableCellContent::Text("y".into()),
        ]]);
    let node = compose_with(table);
    let children = compose_root_children(&node);
    assert!(
        children
            .iter()
            .any(|c| walk_has_kind(c, |k| matches!(k, NodeKind::Table { .. }))),
        "Compose<Table> must carry a structural Table child: {children:?}"
    );
}

#[test]
fn compose_ordered_list_projects_ordered_list_node() {
    let list = OrderedList::new(vec!["a", "b"]);
    let node = compose_with(list);
    let children = compose_root_children(&node);
    assert!(
        children
            .iter()
            .any(|c| walk_has_kind(c, |k| matches!(k, NodeKind::List { ordered: true, .. }))),
        "Compose<OrderedList> must carry a structural ordered List child: {children:?}"
    );
}

#[test]
fn compose_unordered_list_projects_unordered_list_node() {
    let list = UnorderedList::new(vec!["a", "b"]);
    let node = compose_with(list);
    let children = compose_root_children(&node);
    assert!(
        children
            .iter()
            .any(|c| walk_has_kind(c, |k| matches!(k, NodeKind::List { ordered: false, .. }))),
        "Compose<UnorderedList> must carry a structural unordered List child: {children:?}"
    );
}

#[test]
fn compose_progress_projects_paragraph_node() {
    let node = compose_with(Progress::new(0.5).with_label("p"));
    let children = compose_root_children(&node);
    assert!(
        children
            .iter()
            .any(|c| walk_has_kind(c, |k| matches!(k, NodeKind::Paragraph { .. }))),
        "Compose<Progress> must carry a structural Paragraph child: {children:?}"
    );
}

#[test]
fn compose_status_block_projects_root_or_paragraph_child() {
    // StatusBlock projects into a composite root subtree (Paragraph header +
    // optional BlockQuote body). We assert on the inner shape since
    // `project_renderable_content` flattens a nested `Root` into its
    // children rather than introducing a second `Root`.
    let block = StatusBlock::new(StatusState::Warning)
        .header("warn header")
        .body(vec![Prose::new("body msg")]);
    let node = compose_with(block);
    let children = compose_root_children(&node);
    assert!(
        children
            .iter()
            .any(|c| walk_has_kind(c, |k| matches!(k, NodeKind::Paragraph { .. }))),
        "Compose<StatusBlock> must carry the StatusBlock's structural Paragraph header: \
         {children:?}"
    );
}

#[test]
fn compose_text_block_projects_paragraph_node() {
    let node = compose_with(TextBlock::new("text"));
    let children = compose_root_children(&node);
    assert!(
        children
            .iter()
            .any(|c| walk_has_kind(c, |k| matches!(k, NodeKind::Paragraph { .. }))),
        "Compose<TextBlock> must carry a structural Paragraph child: {children:?}"
    );
}

#[test]
fn compose_todo_projects_list_node() {
    let node = compose_with(Todo::new("task"));
    let children = compose_root_children(&node);
    assert!(
        children
            .iter()
            .any(|c| walk_has_kind(c, |k| matches!(k, NodeKind::List { .. }))),
        "Compose<Todo> must carry a structural List child: {children:?}"
    );
}

#[test]
fn compose_two_column_projects_block_quote_columns_carrier() {
    // TwoColumn projects to a NodeKind::BlockQuote carrier with
    // `ColumnsHints` attached to its attrs. We assert on the discriminant;
    // the columns-hints contract is covered by `two_column_parity.rs`.
    let node = compose_with(TwoColumn::new("L", "R"));
    let children = compose_root_children(&node);
    assert!(
        children
            .iter()
            .any(|c| walk_has_kind(c, |k| matches!(k, NodeKind::BlockQuote { .. }))),
        "Compose<TwoColumn> must carry a structural BlockQuote columns carrier: {children:?}"
    );
}

#[test]
fn compose_nested_compose_projects_inner_structural_child() {
    // A `Compose` whose only part is another `Compose` must structurally
    // surface the inner Compose's children. `project_renderable_content`
    // flattens a nested `NodeKind::Root` into its children rather than
    // introducing a second `Root`. We use an inner part with a clearly
    // identifiable `NodeKind` (a Section) so the discriminant check is
    // unambiguous.
    let mut inner = Compose::default();
    inner.add_heading("Inner", 2);
    let outer = compose_with(inner);
    let children = compose_root_children(&outer);
    assert!(
        children
            .iter()
            .any(|c| walk_has_kind(c, |k| matches!(k, NodeKind::Section { .. }))),
        "nested Compose<Compose<Section>> must surface the inner Section child: {children:?}"
    );
}
