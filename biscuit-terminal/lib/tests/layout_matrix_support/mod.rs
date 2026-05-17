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
