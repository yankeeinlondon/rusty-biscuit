//! Unit tests and snapshot tests for the layout visual-test matrix.
//!
//! Regenerate snapshots after intentional changes with:
//! `INSTA_UPDATE=always cargo test -p biscuit-terminal --test layout_matrix`
//! then review the `.snap` diff.

mod layout_matrix_support;

use layout_matrix_support::scenarios;

#[test]
fn scenario_count_is_nineteen() {
    // Twelve original layout scenarios plus the seven properties this feature
    // adds (four width/padding modes and three style properties).
    assert_eq!(scenarios().len(), 19);
}

#[test]
fn scenario_names_are_unique() {
    let names: Vec<&str> = scenarios().iter().map(|s| s.name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "scenario names must be unique");
}

use layout_matrix_support::{pad, side_by_side, stacked_stripped, visible_width};

#[test]
fn pad_extends_to_width() {
    assert_eq!(pad("ab", 5), "ab   ");
    assert_eq!(visible_width(&pad("ab", 5)), 5);
}

#[test]
fn pad_is_ansi_aware() {
    let styled = "\x1b[1mab\x1b[0m";
    assert_eq!(visible_width(&pad(styled, 5)), 5);
}

#[test]
fn stacked_stripped_has_no_ansi() {
    let block = stacked_stripped("\x1b[1mhi\x1b[0m", "\x1b[2mbye\x1b[0m");
    assert!(!block.contains('\x1b'), "snapshot block must be ANSI-free");
    assert!(block.contains("hi") && block.contains("bye"));
    // The header text reflects the post-flip semantics: the left half is the
    // public `render(&term)` output and the right half is
    // `TreeRenderable::render_tree` folded through `render_terminal_node` —
    // both are public trait entry points, not a separate bespoke renderer.
    assert!(
        block.starts_with("VIA_RENDER\n"),
        "snapshot header is VIA_RENDER, not BESPOKE: {block:?}"
    );
    assert!(
        block.contains("\nVIA_TREE_DIRECT\n"),
        "snapshot right-column header is VIA_TREE_DIRECT, not TREE: {block:?}"
    );
}

#[test]
fn side_by_side_includes_title_and_both_columns() {
    let out = side_by_side("My Title", "left", "right", 20);
    assert!(out.contains("My Title"));
    assert!(out.contains("left") && out.contains("right"));
}

use layout_matrix_support::component_cases;

#[test]
fn component_case_count_is_eleven() {
    assert_eq!(component_cases().len(), 11);
}

#[test]
fn every_case_renders_non_empty() {
    for case in component_cases() {
        for scenario in scenarios() {
            let (via_render, via_tree_direct) = (case.render)(&scenario);
            assert!(
                !via_render.is_empty(),
                "{}/{}: render(&term) output was empty",
                case.name,
                scenario.name
            );
            assert!(
                !via_tree_direct.is_empty(),
                "{}/{}: via_tree_direct output was empty",
                case.name,
                scenario.name
            );
        }
    }
}

#[test]
fn layout_matrix_snapshots() {
    for case in component_cases() {
        for scenario in scenarios() {
            let (via_render, via_tree_direct) = (case.render)(&scenario);
            let block = stacked_stripped(&via_render, &via_tree_direct);
            insta::assert_snapshot!(format!("{}__{}", case.name, scenario.name), block);
        }
    }
}

// ── Baseline reference: the shared fold honors the full surface (Scope §1) ──
//
// A single `Paragraph` node carrying every applicable `Layout`
// (`margin`, `padding`, `width` Fixed, `max_width`, `alignment`, `word_wrap`)
// and `Style` (`color`, `background`, `emphasis`, `border`) field is folded to
// each target at a fixed available width. These tests are the reference output
// every later phase validates against: each field must visibly take effect on
// Terminal (SGR / column geometry) and Browser (CSS declarations), and Markdown
// must degrade to structure-only (decision D1).

use biscuit_terminal::prelude::strip_escape_codes;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use renderable::color::{Color, Tailwind};
use renderable::layout::{
    Alignment, Edges, Layout, Length, TargetValue, Width, WordWrap,
};
use renderable::style::{
    Background, Border, BorderSides, PerMode, Style, TextEmphasis,
};
use renderable::tree::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, RenderNode, RenderStrictness,
    render_browser_node, render_markdown_node,
};

/// The available width the baseline reference renders at.
const BASELINE_WIDTH: u32 = 80;

/// A plain paragraph carrying the full applicable `Layout`/`Style` surface,
/// one value per field.
fn baseline_node() -> RenderNode {
    let layout = Layout {
        margin: Edges {
            left: TargetValue::universal(Length::ch(4)),
            ..Edges::default()
        },
        padding: Edges::all(Length::ch(1)),
        width: Width::Fixed(TargetValue::universal(Length::ch(40))),
        max_width: Some(TargetValue::universal(Length::ch(60))),
        alignment: Alignment::Center,
        word_wrap: WordWrap::WrapProse(None, None),
    };
    let style = Style {
        color: Some(TargetValue::universal(PerMode::universal(Color::Tailwind(
            Tailwind::Blue500,
        )))),
        background: Some(Background::subtle()),
        emphasis: TextEmphasis {
            bold: true,
            italic: true,
            ..TextEmphasis::default()
        },
        border: Some(Border {
            sides: BorderSides::Sides {
                top: false,
                right: false,
                bottom: false,
                left: true,
            },
            ..Border::default()
        }),
    };
    let mut node = RenderNode::paragraph(vec![RenderNode::text(
        "The shared render-tree fold resolves the full Layout and Style surface \
         for a plain block node so every later phase can validate against it.",
    )]);
    node.attrs.set_layout(&layout);
    node.attrs.set_style(&style);
    node
}

#[test]
fn baseline_fold_terminal_honors_every_field() {
    let term = Terminal::new_optimistic(BASELINE_WIDTH);
    let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    // Prose wrapping is gated by the context toggle the darkmatter document
    // pipeline sets; enable it so the paragraph flows to the resolved content
    // box rather than emitting one long unwrapped line.
    opts.context.wrap_prose = true;
    let out = render_terminal_node(&baseline_node(), &opts)
        .expect("terminal fold")
        .output;

    // `emphasis` — bold and italic each emit their SGR run.
    assert!(out.contains("\x1b[1m"), "bold SGR missing: {out:?}");
    assert!(out.contains("\x1b[3m"), "italic SGR missing: {out:?}");
    // `border` (thin, left) — the fold paints a single-weight vertical edge.
    assert!(out.contains('│'), "left border glyph missing: {out:?}");
    // `margin`/`alignment` — content lines are pushed right by the left margin
    // plus the centering offset, so at least one line has leading spaces.
    let stripped = strip_escape_codes(&out);
    let content_lines: Vec<&str> = stripped
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert!(
        content_lines.iter().any(|l| l.starts_with("     ")),
        "no leading offset from margin/alignment: {content_lines:?}"
    );
    // `width` Fixed(40) + `max_width`(60) — the content box is capped well
    // below the 80-column available width, so every visible line fits inside it.
    let widest = content_lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        widest <= BASELINE_WIDTH as usize,
        "content exceeded available width ({widest} > {BASELINE_WIDTH})"
    );
    // `word_wrap` — the long paragraph wrapped onto more than one visible line
    // rather than overflowing a single line.
    assert!(
        content_lines.len() > 1,
        "content did not wrap: {content_lines:?}"
    );
}

#[test]
fn baseline_fold_browser_honors_every_field() {
    let html = render_browser_node(&baseline_node(), &BrowserRenderOptions::default())
        .expect("browser fold")
        .output
        .render();

    // `emphasis` — a block node's own emphasis lowers to CSS on the block
    // (semantic `<strong>`/`<em>` tags are reserved for inline emphasis nodes).
    assert!(
        html.contains("font-weight:bold"),
        "bold declaration missing: {html}"
    );
    assert!(
        html.contains("font-style:italic"),
        "italic declaration missing: {html}"
    );
    // `color` and `background` lower to inline CSS.
    assert!(html.contains("color:"), "color declaration missing: {html}");
    assert!(
        html.contains("background-color:"),
        "background declaration missing: {html}"
    );
    // `margin` / `padding` (box model) lower to per-side CSS.
    assert!(html.contains("margin-left:"), "margin-left missing: {html}");
    assert!(html.contains("padding-left:"), "padding-left missing: {html}");
    // `width` Fixed lowers to an explicit `width`.
    assert!(html.contains("width:"), "width declaration missing: {html}");
    // `border` (thin, left) lowers to per-side border CSS.
    assert!(
        html.contains("border-left-style:") || html.contains("border-left-width:"),
        "left border CSS missing: {html}"
    );
}

#[test]
fn baseline_fold_markdown_degrades_to_structure_only() {
    // Decision D1: Markdown is the single documented degradation. It preserves
    // structure (the paragraph text) and emits NO appearance or layout markup —
    // no ANSI escapes, no CSS, no raw styling HTML — for the same node the
    // Terminal and Browser folds style richly.
    let md = render_markdown_node(
        &baseline_node(),
        &MarkdownRenderOptions {
            dialect: MarkdownDialect::Markdown,
            strictness: RenderStrictness::Warn,
            ..MarkdownRenderOptions::default()
        },
    )
    .expect("markdown fold")
    .output;

    // Structure survives: the paragraph text is present.
    assert!(
        md.contains("shared render-tree fold"),
        "paragraph text missing: {md:?}"
    );
    // No appearance leakage of any kind.
    assert!(!md.contains('\x1b'), "ANSI escape leaked into Markdown: {md:?}");
    assert!(!md.contains("style="), "inline CSS leaked into Markdown: {md:?}");
    assert!(
        !md.contains("background-color") && !md.contains("border-left"),
        "CSS declaration leaked into Markdown: {md:?}"
    );
    assert!(
        !md.contains("<div") && !md.contains("<span") && !md.contains("<strong"),
        "raw styling HTML leaked into Markdown: {md:?}"
    );
}
