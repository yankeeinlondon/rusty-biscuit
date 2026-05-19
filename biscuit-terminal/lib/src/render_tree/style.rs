//! Terminal lowering for the `renderable` [`Style`] appearance primitive.
//!
//! The terminal tree renderer applies a node's [`Style`] during the fold, the
//! same place it applies [`Layout`](renderable::layout::Layout). This module
//! holds the lowering logic kept out of [`render`](super::render):
//!
//! - **Color / background** — a [`Color`] is degraded to the terminal's
//!   [`ColorDepth`] (truecolor → 256 → 16) and emitted as foreground /
//!   background SGR. [`PerMode`] selects the light- or dark-mode value.
//! - **Emphasis** — reuses [`TextEmphasis::sgr_ops`] for the non-underline
//!   layers and degrades the underline variant against the terminal's
//!   [`UnderlineSupport`](crate::discovery::detection::UnderlineSupport).
//! - **Box painting** — [`Border`] emits box-drawing characters and [`Fill`]
//!   paints a background band — the full available width, the content band,
//!   or an inset band — honoring [`Fill::inset`].
//!
//! A component never hand-writes ANSI; it declares a [`Style`] and the
//! renderer lowers it here.

use renderable::color::{BasicColor, Color, ColorMode as RenderColorMode, RgbColor};
use renderable::layout::{Length, TargetValue};
use renderable::style::{
    Border, BorderLineStyle, BorderSides, BorderWeight, Fill, FillBand, FillIntensity, PerMode,
    Style, UnderlineStyle,
};
use renderable::target::RenderTarget;

use crate::discovery::detection::ColorDepth;
use crate::terminal::Terminal;
use crate::utils::block_constraint::visible_width;
use crate::utils::color::color_terminal::color_code;

/// The SGR sequence that resets every attribute.
pub(crate) const SGR_RESET: &str = "\x1b[0m";

/// Applies a node's [`Style`] to already-rendered terminal `content`.
///
/// The text-appearance layers (`color`, `background`, `emphasis`) wrap each
/// line; [`Fill`] paints a background band; [`Border`] draws a box around the
/// result. `available_width` is the width the `content` was rendered within
/// (already reduced by any border overhead — see
/// [`border_horizontal_overhead`]).
pub(crate) fn apply_style(
    content: &str,
    style: &Style,
    term: &Terminal,
    available_width: u32,
) -> String {
    let depth = &term.color_depth;
    let mode: RenderColorMode = (&term.color_mode).into();

    let painted = paint_text(content, style, depth, mode, term, available_width);
    match &style.border {
        Some(border) => render_border(&painted, border, depth, mode),
        None => painted,
    }
}

/// The horizontal columns a [`Style`]'s border adds around its content.
///
/// A drawn left or right edge consumes two columns each (the edge glyph plus
/// one space of interior padding). The renderer subtracts this from the inner
/// width before rendering the content so the bordered block stays within the
/// available width.
pub(crate) fn border_horizontal_overhead(style: &Style) -> u32 {
    let Some(border) = &style.border else {
        return 0;
    };
    let (_, right, _, left) = resolve_sides(&border.sides);
    u32::from(left) * 2 + u32::from(right) * 2
}

/// Wraps each line of `content` with the text-appearance and fill layers.
fn paint_text(
    content: &str,
    style: &Style,
    depth: &ColorDepth,
    mode: RenderColorMode,
    term: &Terminal,
    available_width: u32,
) -> String {
    // The opening SGR run: emphasis first, then foreground and background so a
    // single `\x1b[0m` at the end of the line clears the whole run.
    let mut open = emphasis_sgr(style, term);
    if let Some(fg) = style.color.as_ref().and_then(|c| resolve_color(c, mode)) {
        open.push_str(&color_sgr(fg, depth, false).unwrap_or_default());
    }

    // The background is the explicit `background` color when set, otherwise a
    // non-transparent `fill` contributes one. Both lower to a background SGR.
    let fill = style.fill.as_ref().filter(|f| f.intensity != FillIntensity::Transparent);
    let background = style
        .background
        .as_ref()
        .and_then(|c| resolve_color(c, mode))
        .and_then(|c| color_sgr(c, depth, true))
        .or_else(|| fill.and_then(|f| fill_sgr(f, depth, mode)));
    if let Some(bg) = &background {
        open.push_str(bg);
    }

    // The painted band: a `Fill` selects how much of the available width is
    // painted and where it starts (see [`fill_band`]); without a fill only
    // the content itself is painted.
    let widest = content.split('\n').map(visible_width).max().unwrap_or(0);
    let (band_offset, band_width) = match fill {
        Some(f) => fill_band(f, available_width, widest),
        None => (0, 0),
    };

    if open.is_empty() && band_width == 0 {
        return content.to_string();
    }

    let mut out = String::new();
    for (idx, line) in content.split('\n').enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        if band_offset > 0 {
            // The inset columns sit outside the painted band.
            out.push_str(&" ".repeat(band_offset as usize));
        }
        let pad = band_width.saturating_sub(visible_width(line));
        let body = if pad > 0 {
            format!("{line}{}", " ".repeat(pad as usize))
        } else {
            line.to_string()
        };
        if body.is_empty() || open.is_empty() {
            out.push_str(&body);
        } else {
            out.push_str(&open);
            out.push_str(&body);
            out.push_str(SGR_RESET);
        }
    }
    out
}

/// The opening SGR run for a [`Style`]'s text-appearance layers.
///
/// Emphasis, then foreground, then background — the same order
/// [`paint_text`] opens its run in. [`Border`] and [`Fill`] are box-painting
/// layers with no place in an inline run and are excluded. The result is
/// empty when the style declares no text appearance.
///
/// Used by the inline renderer to apply a [`Span`](renderable::tree::NodeKind::Span)'s
/// declared [`Style`]; pair it with [`SGR_RESET`] to close the run.
pub(crate) fn text_appearance_sgr(style: &Style, term: &Terminal) -> String {
    let depth = &term.color_depth;
    let mode: RenderColorMode = (&term.color_mode).into();
    let mut open = emphasis_sgr(style, term);
    if let Some(fg) = style.color.as_ref().and_then(|c| resolve_color(c, mode)) {
        open.push_str(&color_sgr(fg, depth, false).unwrap_or_default());
    }
    if let Some(bg) = style
        .background
        .as_ref()
        .and_then(|c| resolve_color(c, mode))
        .and_then(|c| color_sgr(c, depth, true))
    {
        open.push_str(&bg);
    }
    open
}

/// Builds the emphasis SGR run for a [`Style`].
///
/// The non-underline layers come from [`TextEmphasis::sgr_ops`]; the underline
/// variant is degraded against the terminal's reported support.
fn emphasis_sgr(style: &Style, term: &Terminal) -> String {
    let mut sgr = String::new();
    for (_, code) in style.emphasis.sgr_ops() {
        sgr.push_str(code);
    }
    if let Some(underline) = style.emphasis.underline
        && let Some(code) = underline_sgr(underline, term)
    {
        sgr.push_str(code);
    }
    sgr
}

/// Degrades an [`UnderlineStyle`] to a code the terminal can render.
///
/// An unsupported variant falls back to a straight underline; a terminal with
/// no underline support at all drops the underline entirely.
fn underline_sgr(underline: UnderlineStyle, term: &Terminal) -> Option<&'static str> {
    let support = &term.underline_support;
    let supported = match underline {
        UnderlineStyle::Straight => support.straight,
        UnderlineStyle::Double => support.double,
        UnderlineStyle::Curly => support.curly,
        UnderlineStyle::Dotted => support.dotted,
        UnderlineStyle::Dashed => support.dashed,
    };
    if supported {
        Some(underline.sgr_open())
    } else if support.straight {
        Some(UnderlineStyle::Straight.sgr_open())
    } else {
        None
    }
}

/// Resolves a `TargetValue<PerMode<Color>>` to a concrete terminal [`Color`].
fn resolve_color(tv: &TargetValue<PerMode<Color>>, mode: RenderColorMode) -> Option<Color> {
    tv.resolve(RenderTarget::Terminal)
        .map(|per_mode| *per_mode.resolve(mode))
}

/// Lowers a [`Color`] to a foreground or background SGR escape.
///
/// The color is degraded to `depth`: truecolor uses `38;2;r;g;b`, a 256-color
/// terminal uses the `38;5;n` color cube, and a 16-color terminal uses the
/// color's basic-palette fallback. A terminal with no color support, or a
/// terminal-default / reset color, yields `None`.
///
/// Shared with component code (e.g. `Progress`, `Table` striping) so every
/// declared [`Color`] degrades through the same capability-aware path.
pub(crate) fn color_sgr(color: Color, depth: &ColorDepth, background: bool) -> Option<String> {
    if matches!(depth, ColorDepth::None) {
        return None;
    }
    match color {
        Color::DefaultForeground | Color::DefaultBackground | Color::Reset => None,
        Color::BasicColor(basic) => Some(basic_sgr(basic, depth, background)),
        Color::Rgb(_) | Color::Web(_) | Color::Tailwind(_) => {
            let (rgb, fallback) = rgb_and_fallback(color)?;
            Some(rgb_sgr(rgb, fallback, depth, background))
        }
    }
}

/// The SGR escape for a [`BasicColor`] in the 16-color palette.
///
/// On an 8-color terminal a bright variant is degraded to its non-bright
/// counterpart.
fn basic_sgr(
    basic: renderable::color::BasicColor,
    depth: &ColorDepth,
    background: bool,
) -> String {
    let mut code = u16::from(color_code(basic));
    if matches!(depth, ColorDepth::Minimal) && code >= 90 {
        // No bright palette on an 8-color terminal: 90..97 -> 30..37.
        code -= 60;
    }
    if background {
        // Background codes sit ten above the matching foreground code
        // (30->40, 90->100).
        code += 10;
    }
    format!("\x1b[{code}m")
}

/// The SGR escape for an RGB triple degraded to `depth`.
fn rgb_sgr(
    rgb: (u8, u8, u8),
    fallback: renderable::color::BasicColor,
    depth: &ColorDepth,
    background: bool,
) -> String {
    let (r, g, b) = rgb;
    let lead = if background { 48 } else { 38 };
    match depth {
        ColorDepth::TrueColor => format!("\x1b[{lead};2;{r};{g};{b}m"),
        ColorDepth::Enhanced => format!("\x1b[{lead};5;{}m", color_cube_index(r, g, b)),
        ColorDepth::Basic | ColorDepth::Minimal => basic_sgr(fallback, depth, background),
        // `color_sgr` rejects `ColorDepth::None` before this is reached.
        ColorDepth::None => String::new(),
    }
}

/// Maps an RGB triple to the nearest index in the 256-color 6×6×6 cube.
fn color_cube_index(r: u8, g: u8, b: u8) -> u16 {
    let quantize = |v: u8| -> u16 { (f32::from(v) / 255.0 * 5.0).round() as u16 };
    16 + 36 * quantize(r) + 6 * quantize(g) + quantize(b)
}

/// Returns the RGB triple and basic-palette fallback for a non-basic [`Color`].
fn rgb_and_fallback(color: Color) -> Option<((u8, u8, u8), renderable::color::BasicColor)> {
    match color {
        Color::Rgb(rgb) => Some(((rgb.red(), rgb.green(), rgb.blue()), rgb.fallback())),
        Color::Web(web) => renderable::color::WEB_COLOR_LOOKUP
            .get(&web)
            .map(|rgb| ((rgb.red(), rgb.green(), rgb.blue()), rgb.fallback())),
        Color::Tailwind(tw) => tw
            .to_hdr_color()
            .map(|hdr| ((hdr.red(), hdr.green(), hdr.blue()), hdr.fallback())),
        Color::BasicColor(_)
        | Color::DefaultForeground
        | Color::DefaultBackground
        | Color::Reset => None,
    }
}

/// The default indent, in terminal columns, for a [`FillBand::Indented`]
/// band that carries no explicit [`Fill::inset`].
const DEFAULT_INDENT: u32 = 4;

/// Resolves a [`Fill`]'s [`inset`](Fill::inset) to terminal columns.
///
/// `Ch` is a column count; `Percent` is taken against `available_width`;
/// `Zero`, a missing inset, and the non-universal `Css` unit yield `0`.
fn resolve_inset_columns(fill: &Fill, available_width: u32) -> u32 {
    let Some(length) = fill
        .inset
        .as_ref()
        .and_then(|tv| tv.resolve(RenderTarget::Terminal))
    else {
        return 0;
    };
    match length {
        Length::Zero | Length::Css(_) => 0,
        Length::Ch(n) => *n,
        Length::Percent(pct) => ((available_width as f32) * (pct / 100.0)).round() as u32,
    }
}

/// The leading unpainted columns and painted width of a [`Fill`]'s band.
///
/// - [`FillBand::Full`] paints the whole available width; an explicit
///   [`inset`](Fill::inset) narrows it symmetrically.
/// - [`FillBand::Padded`] paints just the content band, leaving the
///   surrounding padding unpainted; an inset offsets that band.
/// - [`FillBand::Indented`] insets the band from both edges — by the explicit
///   [`inset`](Fill::inset), or [`DEFAULT_INDENT`] when none is set.
///
/// The painted width is never narrower than `widest` (the content), so an
/// inset can never clip the text it sits behind.
fn fill_band(fill: &Fill, available_width: u32, widest: u32) -> (u32, u32) {
    let inset = resolve_inset_columns(fill, available_width);
    match fill.band {
        FillBand::Full => {
            let width = available_width
                .saturating_sub(inset.saturating_mul(2))
                .max(widest);
            (inset, width)
        }
        FillBand::Padded => (inset, widest),
        FillBand::Indented => {
            let indent = if fill.inset.is_some() {
                inset
            } else {
                DEFAULT_INDENT
            };
            let width = available_width
                .saturating_sub(indent.saturating_mul(2))
                .max(widest);
            (indent, width)
        }
    }
}

/// The background SGR escape contributed by a [`Fill`].
///
/// An explicit fill color is degraded to `depth`; without one, a default tint
/// is chosen for the background mode, deepened for [`FillIntensity::Pronounced`].
/// Both the explicit and implicit-tint paths route through [`color_sgr`], so
/// the tint honors the terminal's [`ColorDepth`] — truecolor terminals get the
/// 24-bit escape, lesser terminals get the degraded fallback, and a terminal
/// with no color support yields `None`.
fn fill_sgr(fill: &Fill, depth: &ColorDepth, mode: RenderColorMode) -> Option<String> {
    if let Some(color) = fill.color.as_ref().and_then(|c| resolve_color(c, mode)) {
        return color_sgr(color, depth, true);
    }
    // The implicit tint as an RGB triple plus a basic-palette fallback, so the
    // degradation path is the same one authored colors take.
    let tint = match (mode, fill.intensity) {
        (RenderColorMode::Light, FillIntensity::Pronounced) => {
            RgbColor::new(215, 215, 220, BasicColor::White)
        }
        (RenderColorMode::Light, _) => RgbColor::new(235, 235, 238, BasicColor::White),
        (_, FillIntensity::Pronounced) => RgbColor::new(50, 50, 56, BasicColor::Black),
        (_, _) => RgbColor::new(30, 30, 34, BasicColor::Black),
    };
    color_sgr(Color::Rgb(tint), depth, true)
}

/// Resolves a [`BorderSides`] to `(top, right, bottom, left)` booleans.
fn resolve_sides(sides: &BorderSides) -> (bool, bool, bool, bool) {
    match sides {
        BorderSides::All => (true, true, true, true),
        BorderSides::None => (false, false, false, false),
        BorderSides::Sides {
            top,
            right,
            bottom,
            left,
        } => (*top, *right, *bottom, *left),
    }
}

/// The box-drawing glyphs for a border weight and line style.
struct BorderGlyphs {
    top_left: char,
    top_right: char,
    bottom_left: char,
    bottom_right: char,
    horizontal: char,
    vertical: char,
}

/// Selects the box-drawing glyph set for a [`Border`].
///
/// [`BorderLineStyle::Double`] uses the double-line set; otherwise the weight
/// chooses thin or heavy corners and the line style chooses the edge glyph.
/// [`BorderWeight::Medium`] renders as thin — the terminal box-drawing set has
/// no medium weight.
fn border_glyphs(weight: BorderWeight, line_style: BorderLineStyle) -> BorderGlyphs {
    if line_style == BorderLineStyle::Double {
        return BorderGlyphs {
            top_left: '╔',
            top_right: '╗',
            bottom_left: '╚',
            bottom_right: '╝',
            horizontal: '═',
            vertical: '║',
        };
    }
    let heavy = weight == BorderWeight::Thick;
    let (horizontal, vertical) = match (heavy, line_style) {
        (true, BorderLineStyle::Dashed) => ('┅', '┇'),
        (true, BorderLineStyle::Dotted) => ('┉', '┋'),
        (true, _) => ('━', '┃'),
        (false, BorderLineStyle::Dashed) => ('┄', '┆'),
        (false, BorderLineStyle::Dotted) => ('┈', '┊'),
        (false, _) => ('─', '│'),
    };
    if heavy {
        BorderGlyphs {
            top_left: '┏',
            top_right: '┓',
            bottom_left: '┗',
            bottom_right: '┛',
            horizontal,
            vertical,
        }
    } else {
        BorderGlyphs {
            top_left: '┌',
            top_right: '┐',
            bottom_left: '└',
            bottom_right: '┘',
            horizontal,
            vertical,
        }
    }
}

/// Draws a [`Border`] around already-painted `content`.
///
/// Each enabled side is drawn with the glyph set selected by the border's
/// weight and line style; a corner is drawn only where its two sides meet.
/// The border color, when set, is degraded to `depth` and applied to the
/// glyphs.
fn render_border(
    content: &str,
    border: &Border,
    depth: &ColorDepth,
    mode: RenderColorMode,
) -> String {
    let (top, right, bottom, left) = resolve_sides(&border.sides);
    if !(top || right || bottom || left) {
        return content.to_string();
    }

    let glyphs = border_glyphs(border.weight, border.line_style);
    let color = border
        .color
        .as_ref()
        .and_then(|c| resolve_color(c, mode))
        .and_then(|c| color_sgr(c, depth, false));
    let paint = |text: &str| -> String {
        match &color {
            Some(sgr) => format!("{sgr}{text}{SGR_RESET}"),
            None => text.to_string(),
        }
    };

    let lines: Vec<&str> = content.split('\n').collect();
    let widest = lines.iter().copied().map(visible_width).max().unwrap_or(0);

    // The interior is the content padded to a uniform width, plus one space of
    // padding inside each drawn vertical edge.
    let interior = widest + u32::from(left) + u32::from(right);

    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 2);

    if top {
        out.push(horizontal_rule(
            &glyphs,
            interior,
            left,
            right,
            true,
            &paint,
        ));
    }
    for line in &lines {
        // Pad the content to a uniform width only when a right edge needs the
        // alignment; a left-only bar would otherwise emit trailing whitespace.
        let pad = if right {
            widest.saturating_sub(visible_width(line))
        } else {
            0
        };
        let mut row = String::new();
        if left {
            row.push_str(&paint(&glyphs.vertical.to_string()));
            row.push(' ');
        }
        row.push_str(line);
        row.push_str(&" ".repeat(pad as usize));
        if right {
            row.push(' ');
            row.push_str(&paint(&glyphs.vertical.to_string()));
        }
        out.push(row);
    }
    if bottom {
        out.push(horizontal_rule(
            &glyphs,
            interior,
            left,
            right,
            false,
            &paint,
        ));
    }
    out.join("\n")
}

/// Builds a top or bottom border rule.
///
/// `interior` is the column count between (and excluding) the vertical edges;
/// a corner glyph is used at an end only when that vertical edge is drawn.
fn horizontal_rule(
    glyphs: &BorderGlyphs,
    interior: u32,
    left: bool,
    right: bool,
    is_top: bool,
    paint: &impl Fn(&str) -> String,
) -> String {
    let (left_corner, right_corner) = if is_top {
        (glyphs.top_left, glyphs.top_right)
    } else {
        (glyphs.bottom_left, glyphs.bottom_right)
    };
    let run = interior - u32::from(left) - u32::from(right);
    let mut rule = String::new();
    if left {
        rule.push(left_corner);
    }
    rule.push_str(&glyphs.horizontal.to_string().repeat(run as usize));
    if right {
        rule.push(right_corner);
    }
    paint(&rule)
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::color::{BasicColor, RgbColor, Tailwind};
    use renderable::style::{BorderSides, TextEmphasis};

    fn truecolor_term() -> Terminal {
        Terminal::new_optimistic(80)
    }

    #[test]
    fn foreground_color_lowers_to_truecolor_sgr() {
        let style = Style {
            color: Some(TargetValue::universal(PerMode::universal(Color::Rgb(
                RgbColor::new(255, 0, 0, BasicColor::Red),
            )))),
            ..Style::default()
        };
        let out = apply_style("hello", &style, &truecolor_term(), 10);
        assert!(out.contains("\x1b[38;2;255;0;0m"));
        assert!(out.contains("hello"));
        assert!(out.ends_with(SGR_RESET));
    }

    #[test]
    fn rgb_degrades_to_256_color_cube() {
        let mut term = truecolor_term();
        term.color_depth = ColorDepth::Enhanced;
        let style = Style {
            color: Some(TargetValue::universal(PerMode::universal(Color::Rgb(
                RgbColor::new(255, 0, 0, BasicColor::Red),
            )))),
            ..Style::default()
        };
        let out = apply_style("x", &style, &term, 5);
        assert!(out.contains("\x1b[38;5;196m"), "got {out:?}");
    }

    #[test]
    fn rgb_degrades_to_basic_fallback_on_16_color() {
        let mut term = truecolor_term();
        term.color_depth = ColorDepth::Basic;
        let style = Style {
            color: Some(TargetValue::universal(PerMode::universal(Color::Rgb(
                RgbColor::new(255, 0, 0, BasicColor::Red),
            )))),
            ..Style::default()
        };
        let out = apply_style("x", &style, &term, 5);
        assert!(out.contains("\x1b[31m"), "got {out:?}");
    }

    #[test]
    fn no_color_support_emits_no_color() {
        let mut term = truecolor_term();
        term.color_depth = ColorDepth::None;
        let style = Style {
            color: Some(TargetValue::universal(PerMode::universal(Color::Rgb(
                RgbColor::new(255, 0, 0, BasicColor::Red),
            )))),
            ..Style::default()
        };
        let out = apply_style("x", &style, &term, 5);
        assert_eq!(out, "x");
    }

    #[test]
    fn per_mode_adaptive_resolves_for_dark_terminal() {
        let mut term = truecolor_term();
        term.color_mode = crate::discovery::detection::ColorMode::Dark;
        let style = Style {
            color: Some(TargetValue::universal(PerMode::adaptive(
                Color::BasicColor(BasicColor::Black),
                Color::BasicColor(BasicColor::White),
            ))),
            ..Style::default()
        };
        let out = apply_style("x", &style, &term, 5);
        // Dark terminal resolves to the white branch (fg code 37).
        assert!(out.contains("\x1b[37m"), "got {out:?}");
    }

    #[test]
    fn emphasis_lowers_to_sgr() {
        let style = Style {
            emphasis: TextEmphasis {
                bold: true,
                italic: true,
                ..Default::default()
            },
            ..Style::default()
        };
        let out = apply_style("x", &style, &truecolor_term(), 5);
        assert!(out.contains("\x1b[1m"));
        assert!(out.contains("\x1b[3m"));
    }

    #[test]
    fn underline_double_degrades_when_unsupported() {
        let mut term = truecolor_term();
        term.underline_support.double = false;
        term.underline_support.straight = true;
        let style = Style {
            emphasis: TextEmphasis {
                underline: Some(UnderlineStyle::Double),
                ..Default::default()
            },
            ..Style::default()
        };
        let out = apply_style("x", &style, &term, 5);
        assert!(out.contains(UnderlineStyle::Straight.sgr_open()));
        assert!(!out.contains("\x1b[4:2m"));
    }

    #[test]
    fn background_color_lowers_to_background_sgr() {
        let style = Style {
            background: Some(TargetValue::universal(PerMode::universal(
                Color::BasicColor(BasicColor::Blue),
            ))),
            ..Style::default()
        };
        let out = apply_style("x", &style, &truecolor_term(), 5);
        assert!(out.contains("\x1b[44m"), "got {out:?}");
    }

    #[test]
    fn implicit_fill_tint_degrades_with_color_depth() {
        // The implicit (color-less) fill tint must honor `ColorDepth` rather
        // than emitting a truecolor escape unconditionally.
        let style = Style {
            fill: Some(Fill {
                intensity: FillIntensity::Subtle,
                band: renderable::style::FillBand::Full,
                ..Fill::default()
            }),
            ..Style::default()
        };

        let mut truecolor = truecolor_term();
        truecolor.color_depth = ColorDepth::TrueColor;
        assert!(
            apply_style("hi", &style, &truecolor, 10).contains("\x1b[48;2;"),
            "truecolor terminal should get a 24-bit fill escape"
        );

        let mut enhanced = truecolor_term();
        enhanced.color_depth = ColorDepth::Enhanced;
        let out = apply_style("hi", &style, &enhanced, 10);
        assert!(
            out.contains("\x1b[48;5;") && !out.contains("\x1b[48;2;"),
            "256-color terminal should get a color-cube fill escape, got {out:?}"
        );

        let mut basic = truecolor_term();
        basic.color_depth = ColorDepth::Basic;
        let out = apply_style("hi", &style, &basic, 10);
        assert!(
            !out.contains("\x1b[48;2;") && !out.contains("\x1b[48;5;"),
            "16-color terminal should not get a truecolor or 256 fill escape, got {out:?}"
        );

        let mut none = truecolor_term();
        none.color_depth = ColorDepth::None;
        let out = apply_style("hi", &style, &none, 10);
        assert!(
            !out.contains("\x1b[48;"),
            "a terminal with no color support must emit no fill escape, got {out:?}"
        );
    }

    #[test]
    fn fill_paints_a_background_band() {
        let style = Style {
            fill: Some(Fill {
                intensity: FillIntensity::Subtle,
                band: renderable::style::FillBand::Full,
                ..Fill::default()
            }),
            ..Style::default()
        };
        let out = apply_style("hi", &style, &truecolor_term(), 10);
        // The band pads the two-column content out to the available width.
        assert!(out.contains("\x1b[48;2;"), "got {out:?}");
        assert!(visible_width(out.split('\n').next().unwrap()) >= 10);
    }

    #[test]
    fn fill_band_padded_paints_only_the_content_width() {
        let style = Style {
            fill: Some(Fill {
                intensity: FillIntensity::Subtle,
                band: FillBand::Padded,
                ..Fill::default()
            }),
            ..Style::default()
        };
        let out = apply_style("hi", &style, &truecolor_term(), 20);
        let first = out.split('\n').next().unwrap();
        // No leading inset; the band is the two-column content, not 20.
        assert!(!first.starts_with(' '), "got {first:?}");
        assert_eq!(visible_width(first), 2, "got {first:?}");
    }

    #[test]
    fn fill_band_indented_insets_from_both_edges() {
        let style = Style {
            fill: Some(Fill {
                intensity: FillIntensity::Subtle,
                band: FillBand::Indented,
                ..Fill::default()
            }),
            ..Style::default()
        };
        let out = apply_style("hi", &style, &truecolor_term(), 20);
        let first = out.split('\n').next().unwrap();
        // `DEFAULT_INDENT` (4) leading unpainted columns, then a band inset
        // from both edges: 20 - 2*4 = 12.
        assert!(first.starts_with("    "), "got {first:?}");
        assert!(!first.starts_with("     "), "got {first:?}");
        assert_eq!(visible_width(first), DEFAULT_INDENT + 12, "got {first:?}");
    }

    #[test]
    fn fill_inset_offsets_and_narrows_a_full_band() {
        let style = Style {
            fill: Some(Fill {
                intensity: FillIntensity::Subtle,
                band: FillBand::Full,
                inset: Some(TargetValue::universal(Length::ch(3))),
                ..Fill::default()
            }),
            ..Style::default()
        };
        let out = apply_style("hi", &style, &truecolor_term(), 20);
        let first = out.split('\n').next().unwrap();
        // 3 leading inset columns; the band narrows to 20 - 2*3 = 14.
        assert!(first.starts_with("   "), "got {first:?}");
        assert!(!first.starts_with("    "), "got {first:?}");
        assert_eq!(visible_width(first), 3 + 14, "got {first:?}");
    }

    #[test]
    fn fill_band_indented_explicit_inset_overrides_the_default() {
        let style = Style {
            fill: Some(Fill {
                intensity: FillIntensity::Subtle,
                band: FillBand::Indented,
                inset: Some(TargetValue::universal(Length::ch(2))),
                ..Fill::default()
            }),
            ..Style::default()
        };
        let out = apply_style("hi", &style, &truecolor_term(), 20);
        let first = out.split('\n').next().unwrap();
        assert!(
            first.starts_with("  ") && !first.starts_with("   "),
            "got {first:?}"
        );
    }

    #[test]
    fn border_all_sides_draws_a_box() {
        let style = Style {
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            ..Style::default()
        };
        let out = apply_style("hi", &style, &truecolor_term(), 4);
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with('┌') && lines[0].ends_with('┐'));
        assert!(lines[1].starts_with('│') && lines[1].ends_with('│'));
        assert!(lines[2].starts_with('└') && lines[2].ends_with('┘'));
    }

    #[test]
    fn border_left_only_draws_a_bar() {
        let style = Style {
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
        };
        let out = apply_style("a\nbb", &style, &truecolor_term(), 6);
        for line in out.split('\n') {
            assert!(line.starts_with('│'), "got {line:?}");
        }
    }

    #[test]
    fn border_thick_double_use_distinct_glyphs() {
        let thick = border_glyphs(BorderWeight::Thick, BorderLineStyle::Solid);
        assert_eq!(thick.top_left, '┏');
        let double = border_glyphs(BorderWeight::Thin, BorderLineStyle::Double);
        assert_eq!(double.top_left, '╔');
    }

    #[test]
    fn border_color_is_applied_to_glyphs() {
        let style = Style {
            border: Some(Border {
                sides: BorderSides::All,
                color: Some(TargetValue::universal(PerMode::universal(
                    Color::BasicColor(BasicColor::Green),
                ))),
                ..Border::default()
            }),
            ..Style::default()
        };
        let out = apply_style("x", &style, &truecolor_term(), 3);
        assert!(out.contains("\x1b[32m┌"), "got {out:?}");
    }

    #[test]
    fn border_overhead_counts_drawn_vertical_edges() {
        let all = Style {
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            ..Style::default()
        };
        assert_eq!(border_horizontal_overhead(&all), 4);

        let left_only = Style {
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
        };
        assert_eq!(border_horizontal_overhead(&left_only), 2);

        assert_eq!(border_horizontal_overhead(&Style::default()), 0);
    }

    #[test]
    fn tailwind_color_lowers_through_per_mode() {
        let style = Style {
            color: Some(TargetValue::universal(PerMode::universal(Color::Tailwind(
                Tailwind::Blue500,
            )))),
            ..Style::default()
        };
        let out = apply_style("x", &style, &truecolor_term(), 5);
        assert!(out.contains("\x1b[38;2;"), "got {out:?}");
    }
}
