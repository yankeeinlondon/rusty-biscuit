//! Shared support for the inline-content visual-test matrix.
//!
//! Inline components (`InlineContent`, `Status`, `PadLeft`, `PadRight`) carry
//! no block box, so every `Layout` box property is N/A for them. They still
//! honor inherited `Style::color` / `Style::emphasis` and inline
//! `Style::background`. This module provides the scenario list and component
//! builders used by both the harness example and the snapshot test.
#![allow(dead_code)]

use renderable::layout::{Alignment, Edges, Layout, Length, TargetValue, Width};
use renderable::style::{Background, Style, TextEmphasis};

/// One cell of the inline-content matrix: a layout/style configuration applied
/// to an inline component at a fixed terminal width.
#[derive(Clone)]
pub struct InlineScenario {
    /// Stable identifier used in harness headers and snapshot names.
    pub name: &'static str,
    /// The full `Layout` applied to the component before rendering.
    ///
    /// For box scenarios this property must be a no-op (N/A). For style
    /// scenarios the layout is left at its default so the test isolates the
    /// style dimension.
    pub layout: Layout,
    /// `Style` exercised by this scenario.
    ///
    /// Inline components that route through the render tree could have style
    /// injected on the node, but the components in this matrix are
    /// terminal-only; the style is produced by the component's own content
    /// (e.g. `Prose` markup or `StatusState` color).
    pub style: Style,
    /// Terminal width, in columns.
    pub width: u32,
    /// Whether this scenario exercises a style property (Honored) rather than
    /// a box property (N/A).
    pub is_style: bool,
}

/// The full inline-content scenario list.
///
/// Box scenarios exercise one `Layout` box property in isolation and are
/// expected to be no-ops for inline components. Style scenarios exercise one
/// `Style` property and are expected to be honored.
pub fn inline_scenarios() -> Vec<InlineScenario> {
    vec![
        InlineScenario {
            name: "baseline",
            layout: Layout::default(),
            style: Style::default(),
            width: 80,
            is_style: false,
        },
        // ── Box properties: N/A for inline components ──
        InlineScenario {
            name: "left_margin_4",
            layout: Layout {
                margin: Edges {
                    left: TargetValue::universal(Length::ch(4)),
                    ..Edges::default()
                },
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
            is_style: false,
        },
        InlineScenario {
            name: "right_margin_4",
            layout: Layout {
                margin: Edges {
                    right: TargetValue::universal(Length::ch(4)),
                    ..Edges::default()
                },
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
            is_style: false,
        },
        InlineScenario {
            name: "top_margin_2",
            layout: Layout {
                margin: Edges {
                    top: TargetValue::universal(Length::ch(2)),
                    ..Edges::default()
                },
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
            is_style: false,
        },
        InlineScenario {
            name: "bottom_margin_2",
            layout: Layout {
                margin: Edges {
                    bottom: TargetValue::universal(Length::ch(2)),
                    ..Edges::default()
                },
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
            is_style: false,
        },
        InlineScenario {
            name: "align_center",
            layout: Layout {
                alignment: Alignment::Center,
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
            is_style: false,
        },
        InlineScenario {
            name: "align_right",
            layout: Layout {
                alignment: Alignment::Right,
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
            is_style: false,
        },
        InlineScenario {
            name: "max_width_40",
            layout: Layout {
                max_width: Some(TargetValue::universal(Length::ch(40))),
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
            is_style: false,
        },
        InlineScenario {
            name: "width_auto_fill",
            layout: Layout {
                width: Width::Auto,
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
            is_style: false,
        },
        InlineScenario {
            name: "width_fit_content",
            layout: Layout {
                width: Width::FitContent,
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
            is_style: false,
        },
        InlineScenario {
            name: "width_fixed_pct_50",
            layout: Layout {
                width: Width::Fixed(TargetValue::universal(Length::Percent(50.0))),
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
            is_style: false,
        },
        InlineScenario {
            name: "padding_all_1",
            layout: Layout {
                padding: Edges::all(Length::ch(1)),
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
            is_style: false,
        },
        // ── Style properties: Honored for inline components ──
        InlineScenario {
            name: "color_red",
            layout: Layout::default(),
            style: Style {
                color: Some(TargetValue::universal(renderable::style::PerMode::universal(
                    renderable::color::Color::BasicColor(renderable::color::BasicColor::Red),
                ))),
                ..Style::default()
            },
            width: 80,
            is_style: true,
        },
        InlineScenario {
            name: "emphasis_bold",
            layout: Layout::default(),
            style: Style {
                emphasis: TextEmphasis {
                    bold: true,
                    ..TextEmphasis::default()
                },
                ..Style::default()
            },
            width: 80,
            is_style: true,
        },
        InlineScenario {
            name: "background_inline",
            layout: Layout::default(),
            style: Style {
                background: Some(Background::subtle()),
                ..Style::default()
            },
            width: 80,
            is_style: true,
        },
    ]
}

use biscuit_terminal::components::inline_content::InlineContent;
use biscuit_terminal::components::pad::{PadLeft, PadRight};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;

/// Renders a [`TerminalRenderable`] component at the given width.
pub fn render_inline<C>(component: &C, width: u32) -> String
where
    C: TerminalRenderable,
{
    let term = Terminal::new_optimistic(width);
    component.render(&term)
}

/// A named inline component with a closure that builds it under a scenario.
pub struct InlineComponentCase {
    /// Component name, used in harness headers and snapshot names.
    pub name: &'static str,
    /// Builds the component for the given scenario and renders it, returning
    /// the ANSI-retained output.
    pub render: Box<dyn Fn(&InlineScenario) -> String>,
}

/// The pure inline components exercised by the inline-content matrix.
///
/// Each component is built so that style scenarios produce the relevant style
/// through the component's own content:
///
/// - `InlineContent` / `PadLeft` / `PadRight`: wrap a `Prose` span so color,
///   emphasis, and inline background lower to SGR.
/// - `Status`: color comes from `StatusState`; emphasis comes from the
///   `from_prose` constructor.
pub fn inline_component_cases() -> Vec<InlineComponentCase> {
    vec![
        InlineComponentCase {
            name: "InlineContent",
            render: Box::new(|s| {
                let component = match s.name {
                    "baseline" => InlineContent::from("inline content"),
                    "left_margin_4" | "right_margin_4" | "top_margin_2"
                    | "bottom_margin_2" | "align_center" | "align_right"
                    | "max_width_40" | "width_auto_fill" | "width_fit_content"
                    | "width_fixed_pct_50" | "padding_all_1" => {
                        let mut c = InlineContent::from("inline content");
                        *c.layout_mut() = s.layout.clone();
                        c
                    }
                    "color_red" => InlineContent::from("plain ").with(Prose::new("<red>red</red>")),
                    "emphasis_bold" => {
                        InlineContent::from("plain ").with(Prose::new("<b>bold</b>"))
                    }
                    "background_inline" => InlineContent::from("plain ")
                        .with(Prose::new("<bg-coral>bg</bg-coral>")),
                    other => panic!("unknown inline scenario: {other}"),
                };
                render_inline(&component, s.width)
            }),
        },
        InlineComponentCase {
            name: "Status",
            render: Box::new(|s| {
                let component = match s.name {
                    "baseline" => Status::new("status"),
                    "left_margin_4" | "right_margin_4" | "top_margin_2"
                    | "bottom_margin_2" | "align_center" | "align_right"
                    | "max_width_40" | "width_auto_fill" | "width_fit_content"
                    | "width_fixed_pct_50" | "padding_all_1" => {
                        let mut c = Status::new("status");
                        *c.layout_mut() = s.layout.clone();
                        c
                    }
                    "color_red" => Status::new("status").state(StatusState::Error),
                    "emphasis_bold" => {
                        Status::from_prose("status <b>bold</b>").state(StatusState::Info)
                    }
                    "background_inline" => Status::new("status"),
                    other => panic!("unknown inline scenario: {other}"),
                };
                render_inline(&component, s.width)
            }),
        },
        InlineComponentCase {
            name: "PadLeft",
            render: Box::new(|s| {
                let component = match s.name {
                    "baseline" => PadLeft::new("pad", 10),
                    "left_margin_4" | "right_margin_4" | "top_margin_2"
                    | "bottom_margin_2" | "align_center" | "align_right"
                    | "max_width_40" | "width_auto_fill" | "width_fit_content"
                    | "width_fixed_pct_50" | "padding_all_1" => {
                        let mut c = PadLeft::new("pad", 10);
                        *c.layout_mut() = s.layout.clone();
                        c
                    }
                    "color_red" => PadLeft::new(Prose::new("<red>red</red>"), 10),
                    "emphasis_bold" => PadLeft::new(Prose::new("<b>bold</b>"), 10),
                    "background_inline" => PadLeft::new(Prose::new("<bg-coral>bg</bg-coral>"), 10),
                    other => panic!("unknown inline scenario: {other}"),
                };
                render_inline(&component, s.width)
            }),
        },
        InlineComponentCase {
            name: "PadRight",
            render: Box::new(|s| {
                let component = match s.name {
                    "baseline" => PadRight::new("pad", 10),
                    "left_margin_4" | "right_margin_4" | "top_margin_2"
                    | "bottom_margin_2" | "align_center" | "align_right"
                    | "max_width_40" | "width_auto_fill" | "width_fit_content"
                    | "width_fixed_pct_50" | "padding_all_1" => {
                        let mut c = PadRight::new("pad", 10);
                        *c.layout_mut() = s.layout.clone();
                        c
                    }
                    "color_red" => PadRight::new(Prose::new("<red>red</red>"), 10),
                    "emphasis_bold" => PadRight::new(Prose::new("<b>bold</b>"), 10),
                    "background_inline" => {
                        PadRight::new(Prose::new("<bg-coral>bg</bg-coral>"), 10)
                    }
                    other => panic!("unknown inline scenario: {other}"),
                };
                render_inline(&component, s.width)
            }),
        },
    ]
}
