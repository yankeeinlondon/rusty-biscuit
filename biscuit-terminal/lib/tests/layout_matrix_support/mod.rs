//! Shared support for the layout visual-test matrix.
//!
//! Included by both the `layout_matrix` harness example and the
//! `layout_matrix` snapshot test so they render through identical code.
#![allow(dead_code)]

use renderable::layout::{Alignment, Layout, Length, Edges, TargetValue, Width, WordWrap};
use renderable::style::{Background, Border, BorderSides, Style, TextEmphasis};

/// One cell of the matrix: a layout + style configuration applied at a width.
#[derive(Clone)]
pub struct Scenario {
    /// Stable identifier used in harness headers and snapshot names.
    pub name: &'static str,
    /// The full `Layout` applied to the component before rendering.
    pub layout: Layout,
    /// The `Style` exercised by this scenario.
    ///
    /// Empty (`Style::default()`) for the layout-only scenarios. When
    /// non-empty it is injected onto the render-tree node before folding — the
    /// fold surface this feature audits — so a property such as `background`
    /// visibly takes effect in the `VIA_TREE_DIRECT` column. It is *not* pushed
    /// into the bespoke `render(&term)` path, so a style scenario's
    /// `VIA_RENDER` column shows the pre-migration output until a later phase
    /// routes that component through the fold. This asymmetry is intentional:
    /// the baseline pins where the two public surfaces currently diverge.
    pub style: Style,
    /// Terminal width, in columns, the component renders at.
    pub width: u32,
}

/// The full scenario list — one layout or style dimension varied at a time.
pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "baseline",
            layout: Layout::default(),
            style: Style::default(),
            width: 80,
        },
        Scenario {
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
        },
        Scenario {
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
        },
        Scenario {
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
        },
        Scenario {
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
        },
        Scenario {
            name: "left_margin_pct_10",
            layout: Layout {
                margin: Edges {
                    left: TargetValue::universal(Length::Percent(10.0)),
                    ..Edges::default()
                },
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
        },
        Scenario {
            name: "align_center",
            layout: Layout {
                alignment: Alignment::Center,
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
        },
        Scenario {
            name: "align_right",
            layout: Layout {
                alignment: Alignment::Right,
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
        },
        Scenario {
            name: "max_width_40",
            layout: Layout {
                max_width: Some(TargetValue::universal(Length::ch(40))),
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
        },
        Scenario {
            name: "word_wrap_prose",
            layout: Layout {
                word_wrap: WordWrap::WrapProse(None, None),
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
        },
        Scenario {
            name: "width_40",
            layout: Layout::default(),
            style: Style::default(),
            width: 40,
        },
        Scenario {
            name: "width_120",
            layout: Layout::default(),
            style: Style::default(),
            width: 120,
        },
        // ── The seven properties this feature adds to the locked baseline. ──
        // Four layout modes exercised in isolation on the content box:
        Scenario {
            name: "width_auto_fill",
            layout: Layout {
                width: Width::Auto,
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
        },
        Scenario {
            name: "width_fit_content",
            layout: Layout {
                width: Width::FitContent,
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
        },
        Scenario {
            name: "width_fixed_pct_50",
            layout: Layout {
                width: Width::Fixed(TargetValue::universal(Length::Percent(50.0))),
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
        },
        Scenario {
            name: "padding_all_1",
            layout: Layout {
                padding: Edges::all(Length::ch(1)),
                ..Layout::default()
            },
            style: Style::default(),
            width: 80,
        },
        // Three style properties exercised in isolation (injected on the fold):
        Scenario {
            name: "background_subtle",
            layout: Layout::default(),
            style: Style {
                background: Some(Background::subtle()),
                ..Style::default()
            },
            width: 80,
        },
        Scenario {
            name: "border_thin_left",
            layout: Layout::default(),
            style: Style {
                border: Some(Border {
                    sides: BorderSides::Sides {
                        top: false,
                        right: false,
                        bottom: false,
                        left: true,
                    },
                    ..Border::default()
                }),
                ..Style::default()
            },
            width: 80,
        },
        Scenario {
            name: "emphasis_bold_italic",
            layout: Layout::default(),
            style: Style {
                emphasis: TextEmphasis {
                    bold: true,
                    italic: true,
                    ..TextEmphasis::default()
                },
                ..Style::default()
            },
            width: 80,
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
/// The left column shows the result of the component's public
/// `render(&term)` call. The right column shows the result of folding
/// `TreeRenderable::render_tree(component)` through `render_terminal_node`
/// — the other public trait entry point. After the Stage 2 IR flip both
/// halves typically route through the canonical tree, so the harness is an
/// *informational* view that highlights any regression in either public
/// surface, not an oracle.
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
        "\x1b[1;36m{}\x1b[0m \x1b[2m│\x1b[0m \x1b[1;36mVIA_TREE_DIRECT\x1b[0m\n",
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
/// `via_render` is the output of the component's public `render(&term)` call.
/// `tree` is the result of folding `TreeRenderable::render_tree(component)`
/// through `render_terminal_node` — the other public trait entry point. After
/// the Stage 2 IR flip both halves typically agree by construction; the
/// snapshot captures both so a regression in either path is immediately
/// visible.
pub fn stacked_stripped(via_render: &str, tree: &str) -> String {
    format!(
        "VIA_RENDER\n{}\n---\nVIA_TREE_DIRECT\n{}",
        strip_escape_codes(via_render).trim_end(),
        strip_escape_codes(tree).trim_end(),
    )
}

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::list::{OrderedList, UnorderedList};
use biscuit_terminal::components::progress::Progress;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::components::section::{HeadingLevel, Section};
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::components::text_block::TextBlock;
use biscuit_terminal::components::todo::{Todo, TodoState};
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use renderable::tree::{RenderNode, RenderStrictness, TreeRenderable};

/// A boxed closure that builds a component under a [`Scenario`] and renders
/// it both ways, returning `(via_render_output, via_tree_direct_output)`.
///
/// `via_render_output` is the result of the component's public
/// `render(&term)` call. `via_tree_direct_output` is the result of folding
/// `TreeRenderable::render_tree(component)` through `render_terminal_node` —
/// the other public trait entry point. After the IR flip in Stage 2, both
/// halves typically agree by construction; the pair is preserved so any
/// regression in either public surface shows up as a snapshot diff without
/// needing a separate harness.
type RenderFn = Box<dyn Fn(&Scenario) -> (String, String)>;

/// A named component with a closure that builds it under a scenario and
/// renders both `render(&term)` (the public `TerminalRenderable` entry
/// point) and an explicit `TreeRenderable::render_tree` fold.
pub struct ComponentCase {
    /// Component name, used in harness headers and snapshot names.
    pub name: &'static str,
    /// Returns `(via_render_output, via_tree_direct_output)`, both with ANSI
    /// retained.
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

/// Renders `component` both ways: through `TerminalRenderable::render(&term)`
/// for the left column and through `TreeRenderable::render_tree` +
/// `render_terminal_node` for the right column. ANSI is retained.
///
/// A non-empty `style` (a style scenario such as `background_subtle`) is merged
/// onto the projected render-tree node before folding, so the property takes
/// effect only in the tree column — the fold surface this feature audits. The
/// bespoke `render(&term)` column is left untouched (see [`Scenario::style`]).
fn render_both<C>(component: &C, width: u32, style: &Style) -> (String, String)
where
    C: TerminalRenderable + TreeRenderable,
{
    let term = Terminal::new_optimistic(width);
    let via_render = component.render(&term);
    let mut node = component.render_tree();
    if !style.is_empty() {
        merge_scenario_style(&mut node, style);
    }
    let via_tree_direct = render_tree_string(&node, width);
    (via_render, via_tree_direct)
}

/// Merges a scenario's `Style` onto `node`, letting the scenario win per field
/// while preserving any appearance the component already projected.
///
/// The scenario styles set exactly one property in isolation, so the merge is a
/// simple per-field override: the scenario's `color` / `background` / `border`
/// replace the node's when present, and the scenario `emphasis` is unioned onto
/// the node's via [`TextEmphasis::inherited_from`].
fn merge_scenario_style(node: &mut RenderNode, style: &Style) {
    let base = node.attrs.style_ref().cloned().unwrap_or_default();
    let merged = Style {
        color: style.color.clone().or(base.color),
        background: style.background.clone().or(base.background),
        emphasis: style.emphasis.inherited_from(&base.emphasis),
        border: style.border.clone().or(base.border),
    };
    node.attrs.set_style(&merged);
}

/// The eleven default-case biscuit-terminal components on the render-tree
/// architecture.
///
/// Each component is exercised in its default configuration only — escape-hatch
/// knobs (`BlockQuote::with_border(arbitrary)`, `StatusBlock::border(arbitrary)`,
/// `Table::prefer_cursor_alignment`, `TwoColumn` with `TerminalImage` content)
/// and `FileSystem::render` are deliberately excluded; those paths are
/// covered by dedicated parity fixtures, not this matrix.
pub fn component_cases() -> Vec<ComponentCase> {
    vec![
        ComponentCase {
            name: "BlockQuote",
            render: Box::new(|s| {
                let quote = BlockQuote::new(
                    "The best way to predict the future is to invent it.".into(),
                    Some("Alan Kay"),
                )
                .with_layout(s.layout.clone());
                render_both(&quote, s.width, &s.style)
            }),
        },
        ComponentCase {
            name: "Compose",
            render: Box::new(|s| {
                let compose = Compose::new(vec![
                    RenderableTerminalContent::from("First line of composed output."),
                    RenderableTerminalContent::from("Second line of composed output."),
                ])
                .with_layout(s.layout.clone());
                render_both(&compose, s.width, &s.style)
            }),
        },
        ComponentCase {
            name: "OrderedList",
            render: Box::new(|s| {
                let list = OrderedList::new(vec!["First item", "Second item", "Third item"])
                    .with_layout(s.layout.clone());
                render_both(&list, s.width, &s.style)
            }),
        },
        ComponentCase {
            name: "Progress",
            render: Box::new(|s| {
                let progress = Progress::new(0.75)
                    .with_label("Loading")
                    .with_layout(s.layout.clone());
                render_both(&progress, s.width, &s.style)
            }),
        },
        ComponentCase {
            name: "Section",
            render: Box::new(|s| {
                let mut section = Section::new(HeadingLevel::h2, "Getting Started");
                section
                    .push("Welcome to the tutorial.")
                    .push("Let's begin with installation.");
                let section = section.with_layout(s.layout.clone());
                render_both(&section, s.width, &s.style)
            }),
        },
        ComponentCase {
            name: "StatusBlock",
            render: Box::new(|s| {
                let block = StatusBlock::new(StatusState::Error)
                    .header("Shell Expansion Failed")
                    .body("Missing closing brace in template directive.")
                    .hint("Check the template syntax and retry.")
                    .with_layout(s.layout.clone());
                render_both(&block, s.width, &s.style)
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
                render_both(&table, s.width, &s.style)
            }),
        },
        ComponentCase {
            name: "TextBlock",
            render: Box::new(|s| {
                let block = TextBlock::new(
                    "TextBlock applies uniform block styling to a single piece of content.",
                )
                .with_layout(s.layout.clone());
                render_both(&block, s.width, &s.style)
            }),
        },
        ComponentCase {
            name: "Todo",
            render: Box::new(|s| {
                let todo = Todo::new("Review pull request #42")
                    .with_state(TodoState::InProgress)
                    .with_layout(s.layout.clone());
                render_both(&todo, s.width, &s.style)
            }),
        },
        ComponentCase {
            name: "TwoColumn",
            render: Box::new(|s| {
                let columns = TwoColumn::new("Left column content.", "Right column content.")
                    .with_layout(s.layout.clone());
                render_both(&columns, s.width, &s.style)
            }),
        },
        ComponentCase {
            name: "UnorderedList",
            render: Box::new(|s| {
                let list = UnorderedList::new(vec!["First item", "Second item", "Third item"])
                    .with_layout(s.layout.clone());
                render_both(&list, s.width, &s.style)
            }),
        },
    ]
}
