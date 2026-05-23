//! Stacked visual inspection of darkmatter's render-tree components.
//!
//! `YamlBlock` is the darkmatter component on the render-tree architecture.
//! This example renders it under three layout configurations and prints each
//! one two ways, one after the other, with its layout settings labeled:
//!
//! - **BESPOKE** — `TerminalRenderable::render`.
//! - **TREE** — `render_tree_node` projected to a `RenderNode`, then folded by
//!   `render_terminal_node`.
//!
//! Margins, alignment, and `max_width` are per-component `Layout` settings that
//! both paths honor. Page background is a `DarkmatterPage`-level concern, not a
//! property the `YamlBlock` component carries, so it is labeled N/A here.
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
use biscuit_terminal::utils::layout::{Layout, Length, TargetValue};
use darkmatter::markdown::YamlBlock;
use darkmatter::markdown::render_tree::TerminalCodeRenderer;
use renderable::tree::{RenderNode, RenderStrictness};

/// Render width given to each block, in terminal cells.
const WIDTH: usize = 72;

/// The sample YAML payload rendered under every layout variant.
const SAMPLE: &str = "name: rusty-biscuit\nversion: 0.1.0\ntags:\n  - cli\n  - terminal";

fn main() {
    println!(
        "\x1b[1mRender-tree parity (darkmatter)\x1b[0m \x1b[2m\
         — BESPOKE then TREE (projected + folded), across layout variants\x1b[0m"
    );

    // Variant 1 — defaults: no margins, no width cap.
    show(1, "margin: 0", Layout::default());

    // Variant 2 — left margin 4ch + max-width 50%.
    let mut v2 = Layout::default();
    v2.margin.left = TargetValue::universal(Length::ch(4));
    v2.max_width = Some(TargetValue::universal(Length::Percent(50.0)));
    show(2, "left margin: 4ch  ·  max width: 50%", v2);

    // Variant 3 — left margin 4ch + top/bottom margin 2. Page background is a
    // page-level (`DarkmatterPage`) setting, not part of the component layout.
    let mut v3 = Layout::default();
    v3.margin.left = TargetValue::universal(Length::ch(4));
    v3.margin.top = TargetValue::universal(Length::ch(2));
    v3.margin.bottom = TargetValue::universal(Length::ch(2));
    show(
        3,
        "left margin: 4ch  ·  top/bottom margin: 2  ·  \
         page background: n/a (page-level, not component Layout)",
        v3,
    );

    println!();
}

/// Renders one layout variant stacked: bespoke output first, tree output below.
///
/// `settings` is printed verbatim so the reader knows which `Layout` produced
/// the output beneath it.
fn show(variant: usize, settings: &str, layout: Layout) {
    let term = Terminal::new_optimistic(WIDTH as u32);

    let mut block = YamlBlock::new(SAMPLE).expect("sample YAML is valid");
    *block.layout_mut() = layout;

    let bespoke = block.render(&term);
    let tree = match block.render_tree_node() {
        Some(node) => render_tree(&node),
        None => "<no tree projection>".to_string(),
    };

    print_header(&format!("YamlBlock · variant {variant}"));
    print_settings(settings);
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

/// Prints the dim `layout:` settings line above a variant's rendered blocks.
fn print_settings(settings: &str) {
    println!("\x1b[2mlayout:\x1b[0m {settings}");
}

/// Prints a bold cyan label introducing a rendered block.
fn print_label(label: &str) {
    println!("\x1b[1;36m{label}\x1b[0m");
}
