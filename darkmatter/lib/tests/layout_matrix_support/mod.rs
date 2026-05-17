//! Shared support for darkmatter's layout visual-test matrix.
//!
//! Scoped to `YamlBlock`, the one darkmatter component on the render-tree
//! architecture. Included by both the `layout_matrix` harness example and the
//! `layout_matrix` snapshot test so they render through identical code.
#![allow(dead_code)]

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::prelude::strip_escape_codes;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::YamlBlock;
use renderable::layout::{Alignment, Layout, Margin, RowFill, WordWrap};
use renderable::tree::{RenderNode, RenderStrictness};

/// One cell of the matrix: a layout configuration applied at a width.
#[derive(Clone)]
pub struct Scenario {
    /// Stable identifier used in harness headers and snapshot names.
    pub name: &'static str,
    /// The full `Layout` applied to the component before rendering.
    pub layout: Layout,
    /// Terminal width, in columns, the component renders at.
    pub width: u32,
}

/// The full scenario list — one layout dimension varied at a time.
pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "baseline",
            layout: Layout::default(),
            width: 80,
        },
        Scenario {
            name: "left_margin_4",
            layout: Layout {
                left_margin: Margin::Chars(4),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "right_margin_4",
            layout: Layout {
                right_margin: Margin::Chars(4),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "top_margin_2",
            layout: Layout {
                top_margin: Margin::Chars(2),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "bottom_margin_2",
            layout: Layout {
                bottom_margin: Margin::Chars(2),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "left_margin_pct_10",
            layout: Layout {
                left_margin: Margin::Percent(10.0),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "align_center",
            layout: Layout {
                alignment: Alignment::Center,
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "align_right",
            layout: Layout {
                alignment: Alignment::Right,
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "row_fill_fill",
            layout: Layout {
                row_fill_strategy: RowFill::Fill,
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "word_wrap_prose",
            layout: Layout {
                word_wrap: WordWrap::WrapProse(None, None),
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "width_40",
            layout: Layout::default(),
            width: 40,
        },
        Scenario {
            name: "width_120",
            layout: Layout::default(),
            width: 120,
        },
    ]
}

/// Visible (ANSI-stripped) width of a string, in characters.
pub fn visible_width(s: &str) -> usize {
    strip_escape_codes(s).chars().count()
}

/// Pads `s` with trailing spaces to `width` visible cells (ANSI-aware).
pub fn pad(s: &str, width: usize) -> String {
    let visible = visible_width(s);
    if visible >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - visible))
    }
}

/// Formats a bespoke/tree pair side by side, ANSI retained, for the harness.
pub fn side_by_side(title: &str, bespoke: &str, tree: &str, width: u32) -> String {
    let col = width as usize;
    let bespoke_lines: Vec<&str> = bespoke.lines().collect();
    let tree_lines: Vec<&str> = tree.lines().collect();
    let rows = bespoke_lines.len().max(tree_lines.len());

    let mut out = format!("\n\x1b[1m── {title} ──\x1b[0m\n");
    out.push_str(&format!(
        "\x1b[1;36m{}\x1b[0m \x1b[2m│\x1b[0m \x1b[1;36mTREE\x1b[0m\n",
        pad("BESPOKE", col),
    ));
    for i in 0..rows {
        let left = bespoke_lines.get(i).copied().unwrap_or("");
        let right = tree_lines.get(i).copied().unwrap_or("");
        out.push_str(&format!("{} \x1b[2m│\x1b[0m {right}\n", pad(left, col)));
    }
    out
}

/// Formats a bespoke/tree pair as a stacked, ANSI-stripped block for snapshots.
pub fn stacked_stripped(bespoke: &str, tree: &str) -> String {
    format!(
        "BESPOKE\n{}\n---\nTREE\n{}",
        strip_escape_codes(bespoke).trim_end(),
        strip_escape_codes(tree).trim_end(),
    )
}

/// A named component with a closure that builds it under a scenario and
/// renders both the bespoke and tree paths.
pub struct ComponentCase {
    /// Component name, used in harness headers and snapshot names.
    pub name: &'static str,
    /// Returns `(bespoke_output, tree_output)`, both with ANSI retained.
    pub render: Box<dyn Fn(&Scenario) -> (String, String)>,
}

/// Folds a `RenderNode` into terminal output at the given width.
fn render_tree_string(node: &RenderNode, width: u32) -> String {
    let term = Terminal::new_optimistic(width);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    match render_terminal_node(node, &opts) {
        Ok(rendered) => rendered.output,
        Err(error) => format!("<render error: {error}>"),
    }
}

/// The darkmatter components on the render-tree architecture (`YamlBlock`).
pub fn component_cases() -> Vec<ComponentCase> {
    vec![ComponentCase {
        name: "YamlBlock",
        render: Box::new(|s| {
            let block = YamlBlock::new(
                "name: rusty-biscuit\nversion: 0.1.0\ntags:\n  - cli\n  - terminal",
            )
            .expect("sample YAML is valid")
            .with_layout(s.layout.clone());
            let term = Terminal::new_optimistic(s.width);
            let bespoke = block.render(&term);
            let tree = block
                .render_tree_node()
                .map(|node| render_tree_string(&node, s.width))
                .unwrap_or_else(|| "<no tree projection>".to_string());
            (bespoke, tree)
        }),
    }]
}
