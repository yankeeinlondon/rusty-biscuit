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

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use biscuit_terminal::components::list::{OrderedList, UnorderedList};
use biscuit_terminal::components::progress::Progress;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::components::two_column::TwoColumn;
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
///
/// `nest` recurses into *every* sibling, so the item count is
/// `siblings·(siblings^depth − 1)/(siblings − 1)` — exponential in `depth`.
/// Keep `depth`/`siblings` small: `(10, 2)` yields ~2k list items, the same
/// order of magnitude as the other stress trees. Larger values blow up fast
/// (`(14, 3)` is ~7M items and takes ~24 s per render iteration).
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
    RenderNode::root(vec![nest(0, 10, 2)])
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

fn bench_component_render_path_comparison(c: &mut Criterion) {
    let term = Terminal::new_optimistic(120);
    let opts = TerminalRenderOptions::new(&term, Default::default());
    let progress = Progress::new(0.72)
        .with_label("Indexing")
        .with_bar_width(48);
    let unordered = UnorderedList::new(
        (0..80)
            .map(|i| format!("Unordered item {i} with enough content to exercise wrapping"))
            .collect::<Vec<_>>(),
    );
    let ordered = OrderedList::new(
        (0..80)
            .map(|i| format!("Ordered step {i} with enough content to exercise wrapping"))
            .collect::<Vec<_>>(),
    );
    let mut section = Section::new(HeadingLevel::h2, "Benchmark Section");
    for i in 0..60 {
        section.push(format!(
            "Section paragraph {i} with component content projected into the render tree"
        ));
    }
    let two_column = TwoColumn::new(
        "Left column\nwith multiple lines\nand repeated content",
        "Right column\nwith its own lines\nand repeated content",
    )
    .with_left_percent(0.45)
    .with_gap(4);
    let table = Table::new()
        .with_columns(vec![
            TableColumn::new("Name"),
            TableColumn::new("Status"),
            TableColumn::new("Notes"),
        ])
        .with_data(
            (0..80)
                .map(|i| {
                    vec![
                        TableCellContent::Text(format!("Item {i}")),
                        TableCellContent::Text(if i % 3 == 0 { "Ready" } else { "Queued" }.into()),
                        TableCellContent::Text(format!(
                            "This row has enough text to exercise wrapping and width planning {i}"
                        )),
                    ]
                })
                .collect(),
        );

    let mut group = c.benchmark_group("component_render_path_comparison");

    macro_rules! bench_component {
        ($name:literal, $component:expr) => {{
            group.bench_with_input(
                BenchmarkId::new($name, "bespoke"),
                &$component,
                |b, component| b.iter(|| component.render(black_box(&term))),
            );
            group.bench_with_input(
                BenchmarkId::new($name, "tree"),
                &$component,
                |b, component| {
                    b.iter(|| {
                        let node = component
                            .render_tree_node()
                            .expect("comparison component should support tree rendering");
                        render_terminal_node(black_box(&node), black_box(&opts))
                            .expect("tree rendering should succeed")
                            .output
                    })
                },
            );
        }};
    }

    bench_component!("progress", progress);
    bench_component!("unordered_list_80", unordered);
    bench_component!("ordered_list_80", ordered);
    bench_component!("section_60", section);
    bench_component!("two_column", two_column);
    bench_component!("table_80x3", table);

    group.finish();
}

criterion_group!(
    benches,
    bench_render_node,
    bench_render_document,
    bench_tree_component,
    bench_component_render_path_comparison,
);
criterion_main!(benches);
