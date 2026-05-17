//! Criterion benchmarks for the biscuit-terminal render-tree renderer.
//!
//! These benchmarks exercise [`render_terminal_node`] /
//! [`render_terminal_document`] — the fold of a canonical
//! [`RenderNode`](renderable::tree::RenderNode) tree into terminal output —
//! over programmatically generated stress trees:
//!
//! - a large code block,
//! - a wide-and-tall table,
//! - deeply nested lists,
//! - a link/image-heavy document, and
//! - a **repeated-component-subtree** tree rendered through
//!   [`TreeComponent`], which adapts a [`TreeRenderable`] into a terminal
//!   component. The repeated-subtree case stresses the renderer with many
//!   structurally identical subtrees, the shape produced when a component
//!   tree contains many instances of the same child component.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::render_tree::{
    TerminalRenderOptions, TreeComponent, render_terminal_document, render_terminal_node,
};
use biscuit_terminal::terminal::Terminal;
use renderable::tree::{
    ColumnAlign, Document, HeadingDepth, RenderNode, SourceRegistry, TreeRenderable,
};

// ---------------------------------------------------------------------------
// Stress-tree generators
// ---------------------------------------------------------------------------

/// Builds a tree with one very large code block.
fn large_code_tree() -> RenderNode {
    let mut code = String::new();
    for line in 0..600 {
        code.push_str(&format!(
            "fn generated_{line}() -> usize {{ let value = {line}; value * 2 }}\n"
        ));
    }
    RenderNode::root(vec![
        RenderNode::heading(
            HeadingDepth::new(1).expect("depth 1 is valid"),
            vec![RenderNode::text("Large code block")],
        ),
        RenderNode::code(Some("rust".into()), None, code),
    ])
}

/// Builds a tree with one wide, tall table.
fn large_table_tree() -> RenderNode {
    let columns = 8;
    let rows = 200;
    let align = vec![ColumnAlign::Left; columns];

    let mut table_rows = Vec::with_capacity(rows + 1);
    let header_cells: Vec<RenderNode> = (0..columns)
        .map(|col| RenderNode::table_cell(vec![RenderNode::text(format!("Column {col}"))]))
        .collect();
    table_rows.push(RenderNode::table_row(header_cells));
    for row in 0..rows {
        let cells: Vec<RenderNode> = (0..columns)
            .map(|col| RenderNode::table_cell(vec![RenderNode::text(format!("r{row}c{col} cell"))]))
            .collect();
        table_rows.push(RenderNode::table_row(cells));
    }
    RenderNode::root(vec![RenderNode::table(align, table_rows)])
}

/// Builds a tree with deeply nested unordered lists.
fn deeply_nested_list_tree() -> RenderNode {
    fn nest(level: usize, depth: usize, siblings: usize) -> RenderNode {
        let items: Vec<RenderNode> = (0..siblings)
            .map(|sibling| {
                let mut children = vec![RenderNode::paragraph(vec![RenderNode::text(format!(
                    "level {level} item {sibling} with descriptive text"
                ))])];
                if level + 1 < depth {
                    children.push(nest(level + 1, depth, siblings));
                }
                RenderNode::list_item(None, children)
            })
            .collect();
        RenderNode::list(false, None, items)
    }
    RenderNode::root(vec![nest(0, 14, 3)])
}

/// Builds a tree dense with links and image references.
fn many_links_images_tree() -> RenderNode {
    let paragraphs: Vec<RenderNode> = (0..120)
        .map(|paragraph| {
            RenderNode::paragraph(vec![
                RenderNode::text(format!("Paragraph {paragraph} references ")),
                RenderNode::link(
                    format!("https://example.com/page/{paragraph}"),
                    Some(format!("Title {paragraph}")),
                    vec![RenderNode::text(format!("link {paragraph}"))],
                ),
                RenderNode::text(" and shows "),
                RenderNode::image(
                    format!("https://cdn.example.com/img/{paragraph}.png"),
                    None,
                    format!("alt {paragraph}"),
                ),
                RenderNode::text(" inline."),
            ])
        })
        .collect();
    RenderNode::root(paragraphs)
}

/// Builds a tree composed of many repeated, structurally identical subtrees.
///
/// Each repeated subtree is a heading + paragraph + three-item list; the tree
/// holds 200 of them, the shape a component tree takes when it contains many
/// instances of the same child component.
fn repeated_subtree() -> RenderNode {
    fn unit(index: usize) -> Vec<RenderNode> {
        vec![
            RenderNode::heading(
                HeadingDepth::new(3).expect("depth 3 is valid"),
                vec![RenderNode::text(format!("Repeated unit {index}"))],
            ),
            RenderNode::paragraph(vec![
                RenderNode::text("A paragraph with "),
                RenderNode::strong(vec![RenderNode::text("emphasis")]),
                RenderNode::text(" inside the repeated subtree."),
            ]),
            RenderNode::list(
                false,
                None,
                (0..3)
                    .map(|item| {
                        RenderNode::list_item(
                            None,
                            vec![RenderNode::paragraph(vec![RenderNode::text(format!(
                                "list item {item}"
                            ))])],
                        )
                    })
                    .collect(),
            ),
        ]
    }
    let children: Vec<RenderNode> = (0..200).flat_map(unit).collect();
    RenderNode::root(children)
}

/// A [`TreeRenderable`] wrapper over a pre-built tree, so the
/// repeated-subtree case can be driven through [`TreeComponent`].
#[derive(Debug)]
struct PrebuiltTree(RenderNode);

impl TreeRenderable for PrebuiltTree {
    fn render_tree(&self) -> RenderNode {
        self.0.clone()
    }
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

/// Renders each stress tree to a terminal string via [`render_terminal_node`].
fn bench_render_node(c: &mut Criterion) {
    let trees = [
        ("large_code_block", large_code_tree()),
        ("large_table", large_table_tree()),
        ("deeply_nested_lists", deeply_nested_list_tree()),
        ("many_links_images", many_links_images_tree()),
        ("repeated_subtree", repeated_subtree()),
    ];
    let opts = TerminalRenderOptions::default();

    let mut group = c.benchmark_group("render_tree_terminal_node");
    for (name, tree) in &trees {
        group.bench_function(*name, |b| {
            b.iter(|| render_terminal_node(black_box(tree), &opts))
        });
    }
    group.finish();
}

/// Renders each stress tree wrapped in a [`Document`] via
/// [`render_terminal_document`].
fn bench_render_document(c: &mut Criterion) {
    let trees = [
        ("large_code_block", large_code_tree()),
        ("large_table", large_table_tree()),
        ("deeply_nested_lists", deeply_nested_list_tree()),
        ("many_links_images", many_links_images_tree()),
        ("repeated_subtree", repeated_subtree()),
    ];
    let opts = TerminalRenderOptions::default();

    let mut group = c.benchmark_group("render_tree_terminal_document");
    for (name, tree) in &trees {
        let doc = Document {
            root: tree.clone(),
            metadata: Default::default(),
            sources: SourceRegistry::default(),
        };
        group.bench_function(*name, |b| {
            b.iter(|| render_terminal_document(black_box(&doc), &opts))
        });
    }
    group.finish();
}

/// Renders the repeated-subtree tree through [`TreeComponent`].
///
/// [`TreeComponent`] adapts a [`TreeRenderable`] into a terminal component;
/// this benchmark measures the adapter path that real component trees use.
fn bench_tree_component(c: &mut Criterion) {
    let term = Terminal::new_optimistic(120);
    let component = TreeComponent::new(PrebuiltTree(repeated_subtree()));

    let mut group = c.benchmark_group("render_tree_component");
    group.bench_function("repeated_subtree_render", |b| {
        b.iter(|| component.render(black_box(&term)))
    });
    group.bench_function("repeated_subtree_render_plain", |b| {
        b.iter(|| component.render_plain(black_box(&term)))
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_render_node,
    bench_render_document,
    bench_tree_component,
);
criterion_main!(benches);
