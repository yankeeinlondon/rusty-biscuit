//! Shared support for the layout visual-test matrix.
//!
//! Included by both the `layout_matrix` harness example and the
//! `layout_matrix` snapshot test so they render through identical code.
#![allow(dead_code)]

use renderable::layout::{Alignment, Layout, Margin, RowFill, WordWrap};

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
