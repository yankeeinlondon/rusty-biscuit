//! Side-by-side visual inspection of darkmatter's render-tree components.
//!
//! `YamlBlock` is the darkmatter component on the render-tree architecture.
//! This example renders it two ways and prints them side by side:
//!
//! - **BESPOKE** — `TerminalRenderable::render`.
//! - **TREE** — `render_tree_node` projected to a `RenderNode`, then folded by
//!   `render_terminal_node`.
//!
//! For the biscuit-terminal components (Section, List, TwoColumn, Progress,
//! Table, BlockQuote) see `cargo run -p biscuit-terminal --example
//! render_tree_parity`.
//!
//! Run with:
//!
//! ```text
//! cargo run -p darkmatter --example render_tree_parity
//! ```

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::prelude::strip_escape_codes;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::YamlBlock;
use renderable::tree::{RenderNode, RenderStrictness};

/// Render width given to each column, in terminal cells.
const COL: usize = 38;

fn main() {
    println!(
        "\x1b[1mRender-tree parity (darkmatter)\x1b[0m \x1b[2m\
         — BESPOKE vs TREE (projected + folded)\x1b[0m"
    );

    let block = YamlBlock::new("name: rusty-biscuit\nversion: 0.1.0\ntags:\n  - cli\n  - terminal")
        .expect("sample YAML is valid");
    show("YamlBlock", &block, block.render_tree_node());

    println!();
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
    let used = title.chars().count() + 4;
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
