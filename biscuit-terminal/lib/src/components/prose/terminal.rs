//! Terminal emitter: renders the [`ProseDocument`] IR to ANSI/OSC8 output.
//!
//! This is the behavioral oracle for `Prose`. All terminal capability
//! decisions — OSC8 support, double-underline degradation — happen here,
//! never in the parser.

use renderable::color::{BasicColor, Color};
use renderable::style::UnderlineStyle;

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
    if let Some(fg) = &style.fg {
        ops.push((StyleLayer::Foreground, fg_escape(fg)));
    }
    if let Some(bg) = &style.bg {
        ops.push((StyleLayer::Background, bg_escape(bg)));
    }

    ops
}

/// Foreground SGR opening escape for a color.
fn fg_escape(color: &Color) -> String {
    match color {
        Color::BasicColor(c) => basic_fg(*c).to_string(),
        other => match other.to_rgb() {
            Some((r, g, b)) => format!("\x1b[38;2;{};{};{}m", r, g, b),
            None => "\x1b[39m".to_string(),
        },
    }
}

/// Background SGR opening escape for a color.
fn bg_escape(color: &Color) -> String {
    match color {
        Color::BasicColor(c) => basic_bg(*c).to_string(),
        other => match other.to_rgb() {
            Some((r, g, b)) => format!("\x1b[48;2;{};{};{}m", r, g, b),
            None => "\x1b[49m".to_string(),
        },
    }
}

/// ANSI foreground code for a 16-color basic color.
fn basic_fg(c: BasicColor) -> &'static str {
    match c {
        BasicColor::Black => "\x1b[30m",
        BasicColor::Red => "\x1b[31m",
        BasicColor::Green => "\x1b[32m",
        BasicColor::Yellow => "\x1b[33m",
        BasicColor::Blue => "\x1b[34m",
        BasicColor::Magenta => "\x1b[35m",
        BasicColor::Cyan => "\x1b[36m",
        BasicColor::White => "\x1b[37m",
        BasicColor::BrightBlack => "\x1b[90m",
        BasicColor::BrightRed => "\x1b[91m",
        BasicColor::BrightGreen => "\x1b[92m",
        BasicColor::BrightYellow => "\x1b[93m",
        BasicColor::BrightBlue => "\x1b[94m",
        BasicColor::BrightMagenta => "\x1b[95m",
        BasicColor::BrightCyan => "\x1b[96m",
        BasicColor::BrightWhite => "\x1b[97m",
    }
}

/// ANSI background code for a 16-color basic color.
fn basic_bg(c: BasicColor) -> &'static str {
    match c {
        BasicColor::Black => "\x1b[40m",
        BasicColor::Red => "\x1b[41m",
        BasicColor::Green => "\x1b[42m",
        BasicColor::Yellow => "\x1b[43m",
        BasicColor::Blue => "\x1b[44m",
        BasicColor::Magenta => "\x1b[45m",
        BasicColor::Cyan => "\x1b[46m",
        BasicColor::White => "\x1b[47m",
        BasicColor::BrightBlack => "\x1b[100m",
        BasicColor::BrightRed => "\x1b[101m",
        BasicColor::BrightGreen => "\x1b[102m",
        BasicColor::BrightYellow => "\x1b[103m",
        BasicColor::BrightBlue => "\x1b[104m",
        BasicColor::BrightMagenta => "\x1b[105m",
        BasicColor::BrightCyan => "\x1b[106m",
        BasicColor::BrightWhite => "\x1b[107m",
    }
}
