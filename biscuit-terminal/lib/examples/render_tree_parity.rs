//! Side-by-side visual inspection of the render-tree architecture.
//!
//! For each biscuit-terminal component that opts into the render tree, this
//! example renders it two ways and prints them side by side:
//!
//! - **BESPOKE** — the component's hand-written `TerminalRenderable::render`.
//! - **TREE** — the component projected to a canonical `RenderNode`
//!   (`render_tree_node` / `render_tree`), then folded by the tree renderer
//!   `render_terminal_node`.
//!
//! Spotting drift between the two columns is the point: where the tree path
//! diverges from the bespoke path, the projection or the tree renderer needs
//! attention.
//!
//! Run with:
//!
//! ```text
//! cargo run -p biscuit-terminal --example render_tree_parity
//! ```

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::progress::Progress;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::prelude::strip_escape_codes;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use renderable::tree::{RenderNode, RenderStrictness, TreeRenderable};

/// Render width given to each column, in terminal cells.
const COL: usize = 38;

fn main() {
    println!(
        "\x1b[1mRender-tree parity\x1b[0m \x1b[2m\
         — BESPOKE (hand-written) vs TREE (projected + folded)\x1b[0m"
    );

    // Section — projects via `render_tree_node`.
    let mut section = Section::new(HeadingLevel::h2, "Getting Started");
    section
        .push("Welcome to the tutorial.")
        .push("Let's begin with installation.");
    show("Section", &section, project(&section));

    // UnorderedList — projects via `render_tree_node`.
    let list = UnorderedList::new(vec!["First item", "Second item", "Third item"]);
    show("UnorderedList", &list, project(&list));

    // TwoColumn — projects via `render_tree_node`.
    let columns = TwoColumn::new("Left column content.", "Right column content.");
    show("TwoColumn", &columns, project(&columns));

    // Progress — projects via `render_tree_node`.
    let progress = Progress::new(0.75).with_label("Loading");
    show("Progress", &progress, project(&progress));

    // Table — projects via `render_tree_node`.
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Name"), TableColumn::new("Score")])
        .with_data(vec![
            vec![
                TableCellContent::Text("Ann".into()),
                TableCellContent::Integer(42),
            ],
            vec![
                TableCellContent::Text("Bob".into()),
                TableCellContent::Integer(17),
            ],
        ]);
    show("Table", &table, project(&table));

    // BlockQuote — implements `TreeRenderable::render_tree` directly.
    let quote = BlockQuote::new(
        "The best way to predict the future is to invent it.".into(),
        Some("Alan Kay"),
    );
    show("BlockQuote", &quote, Some(quote.render_tree()));

    println!();
}

/// Projects a component to a `RenderNode` via the `render_tree_node` hook.
fn project(component: &dyn TerminalRenderable) -> Option<RenderNode> {
    component.render_tree_node()
}

/// Renders one component side by side: bespoke output left, tree output right.
fn show(title: &str, component: &dyn TerminalRenderable, node: Option<RenderNode>) {
    let term = Terminal::new_optimistic(COL as u32);

    let bespoke = component.render(&term);
    let tree = match node {
        Some(node) => render_tree(&node),
        None => "<no tree projection>".to_string(),
    };

    print_header(title);
    print_columns("BESPOKE", "TREE", true);
    for (left, right) in zip_lines(&bespoke, &tree) {
        print_columns(&left, &right, false);
    }
}

/// Folds a `RenderNode` into terminal output via the tree renderer.
fn render_tree(node: &RenderNode) -> String {
    let term = Terminal::new_optimistic(COL as u32);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    match render_terminal_node(node, &opts) {
        Ok(rendered) => rendered.output,
        Err(error) => format!("<render error: {error}>"),
    }
}

/// Prints a titled rule spanning both columns.
fn print_header(title: &str) {
    let total = COL * 2 + 3;
    let used = title.chars().count() + 4; // "── " + title + " "
    let trailing = total.saturating_sub(used);
    println!(
        "\n\x1b[1m── {title} \x1b[0m\x1b[2m{}\x1b[0m",
        "─".repeat(trailing)
    );
}

/// Prints one row of the two columns, padding the left to `COL` cells.
fn print_columns(left: &str, right: &str, is_label: bool) {
    let (open, close) = if is_label {
        ("\x1b[1;36m", "\x1b[0m")
    } else {
        ("", "")
    };
    println!(
        "{open}{}{close} \x1b[2m│\x1b[0m {open}{right}{close}",
        pad(left, COL),
    );
}

/// Zips two multi-line strings into aligned `(left, right)` rows.
fn zip_lines(left: &str, right: &str) -> Vec<(String, String)> {
    let left_lines: Vec<&str> = left.lines().collect();
    let right_lines: Vec<&str> = right.lines().collect();
    let rows = left_lines.len().max(right_lines.len());
    (0..rows)
        .map(|i| {
            (
                left_lines.get(i).copied().unwrap_or("").to_string(),
                right_lines.get(i).copied().unwrap_or("").to_string(),
            )
        })
        .collect()
}

/// Pads `s` with trailing spaces to `width` visible cells (ANSI-aware).
fn pad(s: &str, width: usize) -> String {
    let visible = strip_escape_codes(s).chars().count();
    if visible >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - visible))
    }
}
