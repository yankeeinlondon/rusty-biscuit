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

/// Formats a `render`/tree pair side by side, ANSI retained, for the harness.
///
/// Both halves now route through the tree path (every component is flipped),
/// so the left column shows the result of the public `render(&term)` call and
/// the right column shows the result of explicitly folding the projected
/// `RenderNode` through `render_terminal_node`. The two columns therefore
/// agree by construction; the harness is now an *informational* view that
/// highlights any regression in the public render surface, not an oracle.
///
/// The left column is padded to `width` cells — the scenario's render width —
/// so the divider lines up with the right edge of the rendered output.
pub fn side_by_side(title: &str, via_render: &str, tree: &str, width: u32) -> String {
    let col = width as usize;
    let render_lines: Vec<&str> = via_render.lines().collect();
    let tree_lines: Vec<&str> = tree.lines().collect();
    let rows = render_lines.len().max(tree_lines.len());

    let mut out = format!("\n\x1b[1m── {title} ──\x1b[0m\n");
    out.push_str(&format!(
        "\x1b[1;36m{}\x1b[0m \x1b[2m│\x1b[0m \x1b[1;36mTREE\x1b[0m\n",
        pad("VIA_RENDER", col),
    ));
    for i in 0..rows {
        let left = render_lines.get(i).copied().unwrap_or("");
        let right = tree_lines.get(i).copied().unwrap_or("");
        out.push_str(&format!("{} \x1b[2m│\x1b[0m {right}\n", pad(left, col)));
    }
    out
}

/// Formats a `render`/tree pair as a stacked, ANSI-stripped block for snapshots.
///
/// `via_render` is the output of the component's public `render(&term)` call —
/// after the IR flip, every component routes that through the tree renderer
/// internally. `tree` is the same projection rendered explicitly via
/// `render_terminal_node`. The two halves agree by construction; the snapshot
/// captures both so a regression in either path is immediately visible.
pub fn stacked_stripped(via_render: &str, tree: &str) -> String {
    format!(
        "VIA_RENDER\n{}\n---\nTREE\n{}",
        strip_escape_codes(via_render).trim_end(),
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
/// it both ways, returning `(via_render_output, tree_output)`.
///
/// After the IR flip in Stage 2, every component's public `render(&term)`
/// itself routes through the tree path, so the two halves agree by
/// construction. The pair is preserved so any regression in the public render
/// surface (e.g. a bespoke-only fallback regressing on layout) shows up as a
/// snapshot diff without needing a separate harness.
type RenderFn = Box<dyn Fn(&Scenario) -> (String, String)>;

/// A named component with a closure that builds it under a scenario and
/// renders both `render(&term)` (the public API, which after the Stage 2 flip
/// routes through the tree internally) and an explicit tree fold.
pub struct ComponentCase {
    /// Component name, used in harness headers and snapshot names.
    pub name: &'static str,
    /// Returns `(via_render_output, tree_output)`, both with ANSI retained.
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
                let via_render = section.render(&term);
                let tree = section
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (via_render, tree)
            }),
        },
        ComponentCase {
            name: "UnorderedList",
            render: Box::new(|s| {
                let list = UnorderedList::new(vec!["First item", "Second item", "Third item"])
                    .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let via_render = list.render(&term);
                let tree = list
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (via_render, tree)
            }),
        },
        ComponentCase {
            name: "TwoColumn",
            render: Box::new(|s| {
                let columns = TwoColumn::new("Left column content.", "Right column content.")
                    .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let via_render = columns.render(&term);
                let tree = columns
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (via_render, tree)
            }),
        },
        ComponentCase {
            name: "Progress",
            render: Box::new(|s| {
                let progress = Progress::new(0.75)
                    .with_label("Loading")
                    .with_layout(s.layout.clone());
                let term = Terminal::new_optimistic(s.width);
                let via_render = progress.render(&term);
                let tree = progress
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (via_render, tree)
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
                let via_render = table.render(&term);
                let tree = table
                    .render_tree_node()
                    .map(|node| render_tree_string(&node, s.width))
                    .unwrap_or_else(|| "<no tree projection>".to_string());
                (via_render, tree)
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
                let via_render = quote.render(&term);
                let tree = render_tree_string(&quote.render_tree(), s.width);
                (via_render, tree)
            }),
        },
    ]
}
