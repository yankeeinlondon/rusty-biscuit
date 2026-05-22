//! Stacked visual inspection of darkmatter's render-tree components.
//!
//! `YamlBlock` is the darkmatter component on the render-tree architecture.
//! This example renders it two ways and prints them one after the other:
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

use std::rc::Rc;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::YamlBlock;
use darkmatter::markdown::render_tree::TerminalCodeRenderer;
use renderable::tree::{RenderNode, RenderStrictness};

/// Render width given to each block, in terminal cells.
const WIDTH: usize = 72;

fn main() {
    println!(
        "\x1b[1mRender-tree parity (darkmatter)\x1b[0m \x1b[2m\
         — BESPOKE then TREE (projected + folded)\x1b[0m"
    );

    let block = YamlBlock::new("name: rusty-biscuit\nversion: 0.1.0\ntags:\n  - cli\n  - terminal")
        .expect("sample YAML is valid");
    show("YamlBlock", &block, block.render_tree_node());

    println!();
}

/// Renders one component stacked: bespoke output first, tree output below.
fn show(title: &str, component: &dyn TerminalRenderable, node: Option<RenderNode>) {
    let term = Terminal::new_optimistic(WIDTH as u32);

    let bespoke = component.render(&term);
    let tree = match node {
        Some(node) => render_tree(&node),
        None => "<no tree projection>".to_string(),
    };

    print_header(title);
    print_label("BESPOKE");
    println!("{bespoke}");
    print_label("TREE");
    println!("{tree}");
}

/// Folds a `RenderNode` into terminal output via the tree renderer.
///
/// Wires darkmatter's [`TerminalCodeRenderer`] so fenced code blocks
/// syntax-highlight exactly as the production `render_tree_terminal` entry
/// point does — without the hook the tree renderer falls back to a plain,
/// dim, literal-fence projection that does not match the bespoke output.
fn render_tree(node: &RenderNode) -> String {
    let term = Terminal::new_optimistic(WIDTH as u32);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn)
        .with_code_renderer(Rc::new(TerminalCodeRenderer::new()));
    match render_terminal_node(node, &opts) {
        Ok(rendered) => rendered.output,
        Err(error) => format!("<render error: {error}>"),
    }
}

/// Prints a titled rule spanning the full block width.
fn print_header(title: &str) {
    let used = title.chars().count() + 4;
    let trailing = WIDTH.saturating_sub(used);
    println!(
        "\n\x1b[1m── {title} \x1b[0m\x1b[2m{}\x1b[0m",
        "─".repeat(trailing)
    );
}

/// Prints a bold cyan label introducing a rendered block.
fn print_label(label: &str) {
    println!("\x1b[1;36m{label}\x1b[0m");
}
