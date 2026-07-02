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
//! - **Box painting** — [`Border`] emits box-drawing characters around the
//!   content.
//!
//! A component never hand-writes ANSI; it declares a [`Style`] and the
//! renderer lowers it here.

use renderable::color::{Color, ColorMode as RenderColorMode};
use renderable::layout::{Length, TargetValue};
use renderable::style::{
    Border, BorderLineStyle, BorderSides, BorderWeight, PaintColor, PerMode, Style, UnderlineStyle,
};
use renderable::target::RenderTarget;

use crate::discovery::detection::ColorDepth;
use crate::terminal::Terminal;
use crate::utils::block_constraint::visible_width;
use crate::utils::color::color_terminal::color_code;

/// The SGR sequence that resets every attribute.
pub(crate) const SGR_RESET: &str = "\x1b[0m";

/// Resolved padding in whole terminal cells.
///
/// Padding is painted *inside* the background run by [`paint_text`] (CSS box
/// model: `padding` is covered by `background`). The transparent margin, by
/// contrast, is applied outside the painted box by the layout fold in
/// `render.rs` and is never painted.
#[derive(Clone, Copy, Default)]
pub(crate) struct Padding {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

impl Padding {
    /// No padding on any side.
    pub(crate) const ZERO: Self = Self {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    /// Whether every side is zero.
    fn is_zero(&self) -> bool {
        self.top == 0 && self.right == 0 && self.bottom == 0 && self.left == 0
    }
}

/// Applies a node's [`Style`] to already-rendered terminal `content`.
///
/// Equivalent to [`apply_style_with_padding`] with no padding and a
/// content-hugging box. Kept for callers (headings, the style unit tests) that
/// paint a [`Style`] whose box hugs its content.
pub(crate) fn apply_style(content: &str, style: &Style, term: &Terminal) -> String {
    apply_style_with_padding(content, style, term, None, Padding::ZERO)
}

/// Applies a node's [`Style`] and painted `padding` to rendered `content`.
///
/// The text-appearance layers (`color`, `background`, `emphasis`) and the
/// painted padding box are lowered by [`paint_text`]; [`Border`] then draws a
/// box around the padded result (CSS order: content → padding/background →
/// border).
///
/// `content_box` is the resolved content-box width when a [`Layout`] width mode
/// fixed it (`Some`), or `None` when the box should hug its content. A `Some`
/// width is the **floor** for the visible box: the painted background band and
/// the bordered interior fill it even when the content is narrower, so a
/// `Fixed(20)` or filling `Auto` box paints all of its resolved columns rather
/// than shrinking to the widest text line. `FitContent` passes its already-
/// measured width, so the floor is a no-op there.
///
/// [`Layout`]: renderable::layout::Layout
pub(crate) fn apply_style_with_padding(
    content: &str,
    style: &Style,
    term: &Terminal,
    content_box: Option<u32>,
    padding: Padding,
) -> String {
    let depth = &term.color_depth;
    let mode: RenderColorMode = (&term.color_mode).into();

    let painted = paint_text(content, style, depth, mode, term, padding, content_box);
    match &style.border {
        // The border wraps the padded band, so its interior floor includes the
        // horizontal padding already baked into `painted`.
        Some(border) => {
            let interior_floor = content_box.map(|w| w + padding.left + padding.right);
            render_border(&painted, border, depth, mode, interior_floor)
        }
        None => painted,
    }
}

/// The horizontal columns a [`Style`]'s border adds around its content.
///
/// A drawn left or right edge consumes one column each — the edge glyph only.
/// There is no implicit interior space: inner spacing is owned entirely by
/// [`Layout::padding`](renderable::layout::Layout). The renderer subtracts this
/// from the inner width before rendering the content so the bordered block
/// stays within the available width.
pub(crate) fn border_horizontal_overhead(style: &Style) -> u32 {
    let Some(border) = &style.border else {
        return 0;
    };
    let (_, right, _, left) = resolve_sides(&border.sides);
    u32::from(left) + u32::from(right)
}

/// Wraps each line of `content` with the text-appearance layers and paints the
/// `padding` box.
///
/// The background is the explicit [`Style::background`] color. When `padding` is
/// non-zero the content is painted as a rectangular band — `top`/`bottom`
/// painted rows above and below, and `left`/`right` painted columns inside each
/// content row — reusing the band-pad machinery the deleted `Fill` used to
/// supply. With no padding each line is wrapped individually (no band fill).
///
/// `content_box` floors the painted band width: a `Some(w)` widens the
/// background band to `w` columns even when the text is narrower, so a fixed- or
/// auto-width box paints its full resolved width. The floor only widens the
/// *background* band — with no background the right edge stays ragged, which the
/// left-bordered components (`BlockQuote`, `StatusBlock`) rely on.
fn paint_text(
    content: &str,
    style: &Style,
    depth: &ColorDepth,
    mode: RenderColorMode,
    term: &Terminal,
    padding: Padding,
    content_box: Option<u32>,
) -> String {
    // The opening SGR run: emphasis first, then foreground and background so a
    // single `\x1b[0m` at the end of the line clears the whole run.
    let mut open = emphasis_sgr(style, term);
    if let Some(fg) = style.color.as_ref().and_then(|c| resolve_color(c, mode)) {
        open.push_str(&color_sgr(fg, depth, false).unwrap_or_default());
    }
    let mut has_background = false;
    if let Some(bg) = style
        .background
        .as_ref()
        .and_then(|c| resolve_color(c, mode))
        .and_then(|c| color_sgr(c, depth, true))
    {
        open.push_str(&bg);
        has_background = true;
    }

    // Nothing to do: no appearance run and no padding cells to reserve.
    if open.is_empty() && padding.is_zero() {
        return content.to_string();
    }

    let lines: Vec<&str> = content.split('\n').collect();
    let measured = lines.iter().copied().map(visible_width).max().unwrap_or(0);
    // Floor the band at the resolved content box so a fixed/auto box paints its
    // full width; `None` (content-hugging) leaves the band at the widest line.
    let widest = content_box.map_or(measured, |w| measured.max(w));

    // A floored background paints a uniform rectangle even with no padding, so a
    // `Fixed`/`Auto` box fills every resolved column. Without that floor (no
    // background, or a content-hugging `None` box) the no-padding path wraps each
    // line individually so the right edge stays ragged — the color/emphasis runs
    // and the left-bordered components' (`BlockQuote`, `StatusBlock`) `│ `/`┃ `
    // gap rely on it.
    let band_fill = has_background && content_box.is_some();
    if padding.is_zero() && !band_fill {
        let mut out = String::new();
        for (idx, line) in content.split('\n').enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            if line.is_empty() {
                continue;
            }
            out.push_str(&open);
            out.push_str(line);
            out.push_str(SGR_RESET);
        }
        return out;
    }

    // Painted box. With a `background` the box is painted as a rectangle: every
    // row is widened to a uniform `widest` and `top`/`bottom` emit full-width
    // painted blank rows, so the background fills a solid block (reviving the
    // band-pad machinery the deleted `Fill` used to supply). Without a
    // background the right edge stays ragged — left/right padding reserve only
    // the requested cells and no stray trailing whitespace is emitted, which is
    // what the left-bordered components (`BlockQuote`, `StatusBlock`) need to
    // restore their `│ `/`┃ ` inner gap via `Layout::padding`.
    let left_pad = " ".repeat(padding.left as usize);
    let right_pad = " ".repeat(padding.right as usize);

    let wrap = |inner: String| -> String {
        if open.is_empty() {
            inner
        } else {
            format!("{open}{inner}{SGR_RESET}")
        }
    };

    let blank_row = if has_background {
        wrap(" ".repeat((widest + padding.left + padding.right) as usize))
    } else {
        String::new()
    };

    let mut out: Vec<String> =
        Vec::with_capacity(lines.len() + (padding.top + padding.bottom) as usize);
    for _ in 0..padding.top {
        out.push(blank_row.clone());
    }
    for line in &lines {
        let extend = if has_background {
            widest.saturating_sub(visible_width(line))
        } else {
            0
        };
        out.push(wrap(format!(
            "{left_pad}{line}{}{right_pad}",
            " ".repeat(extend as usize),
        )));
    }
    for _ in 0..padding.bottom {
        out.push(blank_row.clone());
    }
    out.join("\n")
}

/// The opening SGR run for a [`Style`]'s text-appearance layers.
///
/// Emphasis, then foreground, then background — the same order
/// [`paint_text`] opens its run in. [`Border`] is a box-painting layer with no
/// place in an inline run and is excluded. The result is empty when the style
/// declares no text appearance.
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

/// Closes an inline appearance run, restoring the ancestor `parent` appearance.
///
/// `open` is the run produced by [`text_appearance_sgr`] for the styled node.
/// When it is empty — no text appearance, or every requested text appearance is
/// unsupported — the run opened nothing, so the close is also empty: no stray
/// [`SGR_RESET`] is emitted. Otherwise the run is reset and the `parent`
/// appearance re-applied so text after the node keeps the inherited
/// color/emphasis.
pub(crate) fn appearance_close(open: &str, parent: &Style, term: &Terminal) -> String {
    if open.is_empty() {
        return String::new();
    }
    format!("{}{}", SGR_RESET, text_appearance_sgr(parent, term))
}

/// Builds the emphasis SGR run for a [`Style`].
///
/// The non-underline layers come from [`TextEmphasis::sgr_ops`]; the underline
/// variant is degraded against the terminal's reported support. A
/// [`ColorDepth::None`] terminal still suppresses non-underline emphasis, but
/// underline support is a separate capability and can remain available when
/// color is unavailable.
fn emphasis_sgr(style: &Style, term: &Terminal) -> String {
    let mut sgr = String::new();
    if !matches!(term.color_depth, ColorDepth::None) {
        for (_, code) in style.emphasis.sgr_ops() {
            sgr.push_str(code);
        }
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

/// Resolves a `TargetValue<PerMode<PaintColor>>` to a concrete terminal
/// [`Color`], intentionally discarding the [`PaintColor`] alpha.
///
/// Terminal cells cannot composite a partial alpha, so the opacity is a
/// documented degradation here — the underlying color is painted at full
/// strength at every [`ColorDepth`].
fn resolve_color(tv: &TargetValue<PerMode<PaintColor>>, mode: RenderColorMode) -> Option<Color> {
    tv.resolve(RenderTarget::Terminal)
        .map(|per_mode| per_mode.resolve(mode).color)
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
fn basic_sgr(basic: renderable::color::BasicColor, depth: &ColorDepth, background: bool) -> String {
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

/// Whether a [`Border`] requests rounded (arc) corners.
///
/// A terminal corner is a single glyph, so a radius is binary: any non-zero
/// [`Border::radius`] selects the light-arc corner set. The arc glyphs exist
/// only for the thin/light box-drawing weight — a heavy or double border has
/// no arc variant and keeps square corners (see [`border_glyphs`]).
fn border_is_rounded(border: &Border) -> bool {
    border
        .radius
        .as_ref()
        .and_then(|tv| tv.resolve(RenderTarget::Terminal))
        .is_some_and(|length| match length {
            Length::Zero => false,
            Length::Ch(n) => *n > 0,
            Length::Percent(pct) => *pct > 0.0,
            // A target-native CSS radius is a positive request; round.
            Length::Css(_) => true,
        })
}

/// Selects the box-drawing glyph set for a [`Border`].
///
/// [`BorderLineStyle::Double`] uses the double-line set; otherwise the weight
/// chooses thin or heavy corners and the line style chooses the edge glyph.
/// [`BorderWeight::Medium`] renders as thin — the terminal box-drawing set has
/// no medium weight.
///
/// `rounded` selects the light-arc corner glyphs (`╭╮╰╯`). The arc set exists
/// only for the thin/light weight, so a heavy or double border keeps its
/// square corners regardless of `rounded`.
fn border_glyphs(weight: BorderWeight, line_style: BorderLineStyle, rounded: bool) -> BorderGlyphs {
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
    } else if rounded {
        BorderGlyphs {
            top_left: '╭',
            top_right: '╮',
            bottom_left: '╰',
            bottom_right: '╯',
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
/// glyphs. The edge glyphs hug the content directly — any inner spacing is the
/// painted [`Layout::padding`](renderable::layout::Layout) band already baked
/// into `content`, not an implicit space inside the border.
///
/// `interior_floor` is the resolved content box (plus its horizontal padding)
/// when a [`Layout`](renderable::layout::Layout) fixed the width: the bordered
/// interior fills it even when the content is narrower, so a `Fixed`/`Auto` box
/// encloses its full resolved width. `None` hugs the content. The floor only
/// widens the interior when a right edge (or a top/bottom rule) makes it
/// observable — a left-only bar stays ragged.
fn render_border(
    content: &str,
    border: &Border,
    depth: &ColorDepth,
    mode: RenderColorMode,
    interior_floor: Option<u32>,
) -> String {
    let (top, right, bottom, left) = resolve_sides(&border.sides);
    if !(top || right || bottom || left) {
        return content.to_string();
    }

    let glyphs = border_glyphs(border.weight, border.line_style, border_is_rounded(border));
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
    let measured = lines.iter().copied().map(visible_width).max().unwrap_or(0);

    // The interior is the content padded to a uniform width — no implicit space
    // inside the drawn edges. Inner spacing, when wanted, comes from the painted
    // `Layout::padding` band already baked into `content`. A fixed/auto box
    // floors the interior at its resolved width so the edges enclose every
    // resolved column rather than shrinking to the widest text line.
    let interior = interior_floor.map_or(measured, |w| measured.max(w));

    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 2);

    if top {
        out.push(horizontal_rule(
            &glyphs, interior, left, right, true, &paint,
        ));
    }
    for line in &lines {
        // Pad the content to a uniform width only when a right edge needs the
        // alignment; a left-only bar would otherwise emit trailing whitespace.
        let pad = if right {
            interior.saturating_sub(visible_width(line))
        } else {
            0
        };
        let mut row = String::new();
        if left {
            row.push_str(&paint(&glyphs.vertical.to_string()));
        }
        row.push_str(line);
        row.push_str(&" ".repeat(pad as usize));
        if right {
            row.push_str(&paint(&glyphs.vertical.to_string()));
        }
        out.push(row);
    }
    if bottom {
        out.push(horizontal_rule(
            &glyphs, interior, left, right, false, &paint,
        ));
    }
    out.join("\n")
}

/// Builds a top or bottom border rule.
///
/// `interior` is the column count between (and excluding) the vertical edges;
/// the horizontal run spans it in full so the rule lines up with the content
/// row. A corner glyph is used at an end only when that vertical edge is drawn.
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
    let run = interior;
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
        let out = apply_style("hello", &style, &truecolor_term());
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
        let out = apply_style("x", &style, &term);
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
        let out = apply_style("x", &style, &term);
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
        let out = apply_style("x", &style, &term);
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
        let out = apply_style("x", &style, &term);
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
        let out = apply_style("x", &style, &truecolor_term());
        assert!(out.contains("\x1b[1m"));
        assert!(out.contains("\x1b[3m"));
    }

    #[test]
    fn inverse_lowers_to_sgr_7() {
        let style = Style {
            emphasis: TextEmphasis {
                inverse: true,
                ..Default::default()
            },
            ..Style::default()
        };
        let out = apply_style("x", &style, &truecolor_term());
        assert!(out.contains("\x1b[7m"), "got {out:?}");
        assert!(out.ends_with(SGR_RESET));
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
        let out = apply_style("x", &style, &term);
        assert!(out.contains(UnderlineStyle::Straight.sgr_open()));
        assert!(!out.contains("\x1b[4:2m"));
    }

    #[test]
    fn underline_degrades_when_color_depth_is_none() {
        let mut term = truecolor_term();
        term.color_depth = ColorDepth::None;
        term.underline_support.double = false;
        term.underline_support.straight = true;
        let style = Style {
            emphasis: TextEmphasis {
                underline: Some(UnderlineStyle::Double),
                ..Default::default()
            },
            ..Style::default()
        };
        let out = apply_style("x", &style, &term);
        assert_eq!(out, "\x1b[4mx\x1b[0m");
    }

    #[test]
    fn background_color_lowers_to_background_sgr() {
        let style = Style {
            background: Some(TargetValue::universal(PerMode::universal(
                Color::BasicColor(BasicColor::Blue),
            ))),
            ..Style::default()
        };
        let out = apply_style("x", &style, &truecolor_term());
        assert!(out.contains("\x1b[44m"), "got {out:?}");
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
        let out = apply_style("hi", &style, &truecolor_term());
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with('┌') && lines[0].ends_with('┐'));
        assert!(lines[1].starts_with('│') && lines[1].ends_with('│'));
        assert!(lines[2].starts_with('└') && lines[2].ends_with('┘'));
    }

    #[test]
    fn border_rules_match_content_row_width() {
        // Regression: the top/bottom rules must be exactly as wide as the
        // content row so the corners line up with the vertical edges. With no
        // implicit interior gap the edges hug the content directly.
        let style = Style {
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            ..Style::default()
        };
        let out = apply_style("hello world", &style, &truecolor_term());
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(lines.len(), 3);
        let widths: Vec<u32> = lines.iter().copied().map(visible_width).collect();
        assert_eq!(
            widths[0], widths[1],
            "top rule must match content row width, got {widths:?}"
        );
        assert_eq!(
            widths[2], widths[1],
            "bottom rule must match content row width, got {widths:?}"
        );
        // │ + 11 + │ = 13 columns (no interior gap).
        assert_eq!(widths[1], 13, "got {widths:?}");
    }

    #[test]
    fn rounded_border_rules_match_content_row_width() {
        let style = Style {
            border: Some(Border {
                sides: BorderSides::All,
                radius: Some(TargetValue::universal(Length::ch(1))),
                ..Border::default()
            }),
            ..Style::default()
        };
        let out = apply_style("the quick brown fox", &style, &truecolor_term());
        let lines: Vec<&str> = out.split('\n').collect();
        let widths: Vec<u32> = lines.iter().copied().map(visible_width).collect();
        assert_eq!(widths[0], widths[1], "got {widths:?}");
        assert_eq!(widths[2], widths[1], "got {widths:?}");
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
        let out = apply_style("a\nbb", &style, &truecolor_term());
        for line in out.split('\n') {
            assert!(line.starts_with('│'), "got {line:?}");
        }
    }

    #[test]
    fn border_thick_double_use_distinct_glyphs() {
        let thick = border_glyphs(BorderWeight::Thick, BorderLineStyle::Solid, false);
        assert_eq!(thick.top_left, '┏');
        let double = border_glyphs(BorderWeight::Thin, BorderLineStyle::Double, false);
        assert_eq!(double.top_left, '╔');
    }

    #[test]
    fn border_radius_selects_arc_corners() {
        let style = Style {
            border: Some(Border {
                sides: BorderSides::All,
                radius: Some(TargetValue::universal(Length::ch(1))),
                ..Border::default()
            }),
            ..Style::default()
        };
        let out = apply_style("x", &style, &truecolor_term());
        let lines: Vec<&str> = out.split('\n').collect();
        assert!(
            lines[0].starts_with('╭') && lines[0].ends_with('╮'),
            "got {:?}",
            lines[0]
        );
        assert!(
            lines[2].starts_with('╰') && lines[2].ends_with('╯'),
            "got {:?}",
            lines[2]
        );
    }

    #[test]
    fn border_zero_radius_keeps_square_corners() {
        let style = Style {
            border: Some(Border {
                sides: BorderSides::All,
                radius: Some(TargetValue::universal(Length::Zero)),
                ..Border::default()
            }),
            ..Style::default()
        };
        let out = apply_style("x", &style, &truecolor_term());
        assert!(out.split('\n').next().unwrap().starts_with('┌'));
    }

    #[test]
    fn border_radius_ignored_for_heavy_and_double() {
        // The light-arc corner set has no heavy or double variant.
        let heavy = border_glyphs(BorderWeight::Thick, BorderLineStyle::Solid, true);
        assert_eq!(heavy.top_left, '┏');
        let double = border_glyphs(BorderWeight::Thin, BorderLineStyle::Double, true);
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
        let out = apply_style("x", &style, &truecolor_term());
        assert!(out.contains("\x1b[32m┌"), "got {out:?}");
    }

    #[test]
    fn border_overhead_counts_drawn_vertical_edges() {
        // One column per drawn vertical edge — no implicit interior gap.
        let all = Style {
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            ..Style::default()
        };
        assert_eq!(border_horizontal_overhead(&all), 2);

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
        assert_eq!(border_horizontal_overhead(&left_only), 1);

        assert_eq!(border_horizontal_overhead(&Style::default()), 0);
    }

    #[test]
    fn border_reserves_only_drawn_cells_no_implicit_gap() {
        // A left+right border with no padding: content sits directly inside the
        // edges with no implicit interior space.
        let style = Style {
            border: Some(Border {
                sides: BorderSides::All,
                ..Border::default()
            }),
            ..Style::default()
        };
        assert_eq!(border_horizontal_overhead(&style), 2);
        let out = apply_style("ab", &style, &truecolor_term());
        let row = out.split('\n').find(|l| l.contains("ab")).unwrap();
        // `│ab│` — the glyphs hug the content, no space between edge and text.
        assert!(row.contains("│ab│"), "no implicit interior space, got {row:?}");
    }

    #[test]
    fn tailwind_color_lowers_through_per_mode() {
        let style = Style {
            color: Some(TargetValue::universal(PerMode::universal(Color::Tailwind(
                Tailwind::Blue500,
            )))),
            ..Style::default()
        };
        let out = apply_style("x", &style, &truecolor_term());
        assert!(out.contains("\x1b[38;2;"), "got {out:?}");
    }

    #[test]
    fn foreground_alpha_degrades_to_the_opaque_color_at_every_depth() {
        use renderable::style::{Opacity, PaintColor};
        // A half-transparent foreground paints exactly the same SGR as its
        // opaque counterpart: the terminal discards alpha at every color depth.
        let red = Color::Rgb(RgbColor::new(255, 0, 0, BasicColor::Red));
        let translucent = Style {
            color: Some(TargetValue::universal(PerMode::universal(
                PaintColor::new(red).with_opacity(Opacity::new(128)),
            ))),
            ..Style::default()
        };
        let opaque = Style {
            color: Some(TargetValue::universal(PerMode::universal(red))),
            ..Style::default()
        };
        for depth in [
            ColorDepth::TrueColor,
            ColorDepth::Enhanced,
            ColorDepth::Basic,
            ColorDepth::Minimal,
        ] {
            let mut term = truecolor_term();
            term.color_depth = depth;
            assert_eq!(
                apply_style("x", &translucent, &term),
                apply_style("x", &opaque, &term),
                "alpha changed output at {depth:?}"
            );
        }
    }
}
