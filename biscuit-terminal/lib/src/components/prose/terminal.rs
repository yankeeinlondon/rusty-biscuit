//! Terminal emitter: renders the [`ProseDocument`] IR to ANSI/OSC8 output.
//!
//! This is the behavioral oracle for `Prose`. All terminal capability
//! decisions — OSC8 support, double-underline degradation — happen here,
//! never in the parser.

use renderable::style::UnderlineStyle;

use crate::discovery::detection::ColorDepth;
use crate::render_tree::style::color_sgr;
use crate::terminal::Terminal;

use super::ir::{ProseDocument, ProseNode, ProseStyle};
use super::styles::{StyleLayer, StyleState, degraded_double_underline_open, resolve_href};

/// Render a parsed Prose document to a terminal string.
///
/// Emits a single trailing `\x1b[0m` when any style escape was produced.
pub(super) fn render(doc: &ProseDocument, term: Option<&Terminal>) -> String {
    let mut state = StyleState::default();
    let mut out = String::new();
    render_nodes(&doc.children, term, &mut state, &mut out);
    if state.used_styles {
        out.push_str("\x1b[0m");
    }
    out
}

/// Render a slice of nodes into `out`.
fn render_nodes(
    nodes: &[ProseNode],
    term: Option<&Terminal>,
    state: &mut StyleState,
    out: &mut String,
) {
    for node in nodes {
        match node {
            ProseNode::Text(text) => out.push_str(text),
            ProseNode::CodeBlock { value, .. } => render_code_block(value, state, out),
            ProseNode::Link { href, children } => render_link(href, children, term, state, out),
            ProseNode::Span { style, children } => render_span(style, children, term, state, out),
        }
    }
}

/// Render a fenced code block as dim, 2-space-indented text.
///
/// Each code line is closed with a hard `\x1b[0m` so the dim attribute
/// never bleeds. When the block is nested inside an active span, that
/// reset would also clear the enclosing span's attributes, leaving
/// following sibling text unstyled — so the enclosing layers are
/// re-emitted from [`StyleState`] once the block is complete.
fn render_code_block(value: &str, state: &mut StyleState, out: &mut String) {
    state.used_styles = true;
    for (idx, line) in value.lines().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        out.push_str("\x1b[2m  ");
        out.push_str(line);
        out.push_str("\x1b[0m");
    }
    state.reapply_active_layers(out);
}

/// Render a hyperlink: OSC8 when supported, Markdown fallback otherwise.
fn render_link(
    href: &str,
    children: &[ProseNode],
    term: Option<&Terminal>,
    state: &mut StyleState,
    out: &mut String,
) {
    let resolved = resolve_href(href);
    if resolved.is_empty() {
        // No usable href — render the description only.
        render_nodes(children, term, state, out);
        return;
    }

    let supports_osc8 = term.map(|t| t.osc_link_support).unwrap_or(true);
    if supports_osc8 {
        state.used_styles = true;
        out.push_str(&format!("\x1b]8;;{}\x1b\\", resolved));
        render_nodes(children, term, state, out);
        out.push_str("\x1b]8;;\x1b\\");
    } else {
        // Markdown fallback: `[description](href)`. The closing `]` of any
        // bracket inside the description is escaped so downstream CommonMark
        // parsers do not mis-resolve the link.
        state.used_styles = true;
        let mut inner = String::new();
        render_nodes(children, term, state, &mut inner);
        out.push('[');
        out.push_str(&inner.replace(']', "\\]"));
        out.push_str(&format!("]({})", resolved));
    }
}

/// Render a styled span, applying per-layer push/pop so a closing span
/// restores the parent's value rather than issuing a nuclear reset.
fn render_span(
    style: &ProseStyle,
    children: &[ProseNode],
    term: Option<&Terminal>,
    state: &mut StyleState,
    out: &mut String,
) {
    let ops = style_to_ops(style, term);
    if ops.is_empty() {
        // Fully suppressed style (e.g. `<double-underline>` with no
        // underline support): render the inner content with no escapes.
        render_nodes(children, term, state, out);
        return;
    }

    let mut applied: Vec<(StyleLayer, Option<String>)> = Vec::with_capacity(ops.len());
    for (layer, open) in &ops {
        let prev = state.set(*layer, open);
        out.push_str(open);
        applied.push((*layer, prev));
    }

    render_nodes(children, term, state, out);

    for (layer, prev) in applied.into_iter().rev() {
        state.restore(layer, prev);
        out.push_str(state.close_code(layer));
    }
}

/// Translate a [`ProseStyle`] into ordered (layer, opening-escape) pairs.
///
/// Weight / decoration escapes come from the shared
/// [`TextEmphasis`](renderable::style::TextEmphasis) emitter; only the
/// capability-aware underline degradation and the `Prose`-only
/// inverse/hidden/color layers are decided here. A `<double-underline>`
/// request with no terminal underline support contributes no op — the
/// caller then renders the inner content plain.
fn style_to_ops(style: &ProseStyle, term: Option<&Terminal>) -> Vec<(StyleLayer, String)> {
    let mut ops: Vec<(StyleLayer, String)> = Vec::new();

    for (layer, open) in style.emphasis.sgr_ops() {
        ops.push((StyleLayer::from_emphasis(layer), open.to_string()));
    }
    if style.inverse {
        ops.push((StyleLayer::Inverse, "\x1b[7m".to_string()));
    }
    if style.hidden {
        ops.push((StyleLayer::Hidden, "\x1b[8m".to_string()));
    }
    if let Some(kind) = style.emphasis.underline {
        let open = match kind {
            UnderlineStyle::Double => degraded_double_underline_open(term).map(str::to_string),
            other => Some(other.sgr_open().to_string()),
        };
        if let Some(open) = open {
            ops.push((StyleLayer::Underline, open));
        }
    }
    // Colors degrade through the shared capability-aware path so a declared
    // color (basic, RGB, web, or Tailwind) is emitted at the terminal's depth —
    // truecolor, 256-cube, 16-color fallback, or nothing under no-color. When
    // `term` is absent (optimistic render) we assume full truecolor. Skipping
    // the op entirely under no-color keeps `StyleState` from emitting a reset
    // for a layer that was never opened.
    let depth = term.map_or(ColorDepth::TrueColor, |t| t.color_depth);
    if let Some(fg) = &style.fg
        && let Some(open) = color_sgr(*fg, &depth, false)
    {
        ops.push((StyleLayer::Foreground, open));
    }
    if let Some(bg) = &style.bg
        && let Some(open) = color_sgr(*bg, &depth, true)
    {
        ops.push((StyleLayer::Background, open));
    }

    ops
}
