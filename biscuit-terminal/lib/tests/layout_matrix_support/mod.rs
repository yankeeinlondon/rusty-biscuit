//! Shared support for the layout visual-test matrix.
//!
//! Included by both the `layout_matrix` harness example and the
//! `layout_matrix` snapshot test so they render through identical code.
#![allow(dead_code)]

use renderable::layout::{Alignment, Layout, Length, Margin, TargetValue, WordWrap};

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
                margin: Margin {
                    left: TargetValue::universal(Length::ch(4)),
                    ..Margin::default()
                },
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "right_margin_4",
            layout: Layout {
                margin: Margin {
                    right: TargetValue::universal(Length::ch(4)),
                    ..Margin::default()
                },
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "top_margin_2",
            layout: Layout {
                margin: Margin {
                    top: TargetValue::universal(Length::ch(2)),
                    ..Margin::default()
                },
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "bottom_margin_2",
            layout: Layout {
                margin: Margin {
                    bottom: TargetValue::universal(Length::ch(2)),
                    ..Margin::default()
                },
                ..Layout::default()
            },
            width: 80,
        },
        Scenario {
            name: "left_margin_pct_10",
            layout: Layout {
                margin: Margin {
                    left: TargetValue::universal(Length::Percent(10.0)),
                    ..Margin::default()
                },
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
            name: "max_width_40",
            layout: Layout {
                max_width: Some(TargetValue::universal(Length::ch(40))),
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

use biscuit_terminal::prelude::strip_escape_codes;

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
///
/// The left column is padded to `width` cells — the scenario's render width —
/// so the divider lines up with the right edge of the bespoke output.
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

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::progress::Progress;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use renderable::tree::{RenderNode, RenderStrictness, TreeRenderable};

/// A boxed closure that builds a component under a [`Scenario`] and renders
/// it both ways, returning `(bespoke_output, tree_output)`.
type RenderFn = Box<dyn Fn(&Scenario) -> (String, String)>;

/// A named component with a closure that builds it under a scenario and
/// renders both the bespoke and tree paths.
pub struct ComponentCase {
    /// Component name, used in harness headers and snapshot names.
    pub name: &'static str,
    /// Returns `(bespoke_output, tree_output)`, both with ANSI retained.
    pub render: RenderFn,
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

/// All six biscuit-terminal components on the render-tree architecture.
pub fn component_cases() -> Vec<ComponentCase> {
    vec![
        ComponentCase {
            name: "Section",
            render: Box::new(|s| {
                let mut section = Section::new(HeadingLevel::h2, "Getting Started");
                section
                    .push("Welcome to the tutorial.")
                    .push("Let's begin with installation.");
                let section = section.with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let bespoke = section.render(&term);
                let tree = section
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (bespoke, tree)
            }),
        },
        ComponentCase {
            name: "UnorderedList",
            render: Box::new(|s| {
                let list = UnorderedList::new(vec!["First item", "Second item", "Third item"])
                    .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let bespoke = list.render(&term);
                let tree = list
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (bespoke, tree)
            }),
        },
        ComponentCase {
            name: "TwoColumn",
            render: Box::new(|s| {
                let columns = TwoColumn::new("Left column content.", "Right column content.")
                    .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let bespoke = columns.render(&term);
                let tree = columns
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (bespoke, tree)
            }),
        },
        ComponentCase {
            name: "Progress",
            render: Box::new(|s| {
                let progress = Progress::new(0.75)
                    .with_label("Loading")
                    .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let bespoke = progress.render(&term);
                let tree = progress
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (bespoke, tree)
            }),
        },
        ComponentCase {
            name: "Table",
            render: Box::new(|s| {
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
                    ])
                    .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let bespoke = table.render(&term);
                let tree = table
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (bespoke, tree)
            }),
        },
        ComponentCase {
            name: "BlockQuote",
            render: Box::new(|s| {
                let quote = BlockQuote::new(
                    "The best way to predict the future is to invent it.".into(),
                    Some("Alan Kay"),
                )
                .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let bespoke = quote.render(&term);
                let tree = render_tree_string(&quote.render_tree(), s.width);
                (bespoke, tree)
            }),
        },
    ]
}
