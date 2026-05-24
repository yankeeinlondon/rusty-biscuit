//! Visual chrome primitives — border, margin, height — shared by the
//! choose components.
//!
//! Phase 1 introduces the library-side types the later CLI phases
//! depend on. The types are intentionally plain data structures plus a
//! thin widget wrapper; they do not mutate any existing public API.
//!
//! ## Module Contents
//!
//! - [`BorderStyle`] maps the CLI `--border-style` enum onto
//!   ratatui's [`Borders`] / [`BorderType`] tuple.
//! - [`Margin`] is a four-sided margin struct with a saturating
//!   `shrink` helper.
//! - [`HeightSpec`] is the parsed form of the CLI `--height` flag.
//! - [`FrameChrome`] / [`FrameChromeConfig`] wrap a stateful widget
//!   with optional border and margin and render both together.
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::core::{BorderStyle, HeightSpec, Margin};
//!
//! let margin = Margin::uniform(2);
//! assert_eq!(margin.top, 2);
//!
//! let height = HeightSpec::Percent(50);
//! assert_eq!(height.resolve(40), 20);
//!
//! let (borders, _border_type) = BorderStyle::Rounded.ratatui_parts();
//! assert!(borders.contains(ratatui::widgets::Borders::TOP));
//! ```

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders, StatefulWidget, Widget},
};

/// The CLI-facing border vocabulary, mapped to ratatui primitives.
///
/// Each variant resolves to a pair of `(Borders, BorderType)` via
/// [`BorderStyle::ratatui_parts`]. `None` resolves to an empty
/// [`Borders`] set, meaning the [`FrameChrome`] draws no border at
/// all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BorderStyle {
    /// No border drawn.
    #[default]
    None,
    /// Rounded corners on all four sides.
    Rounded,
    /// Single-line border with sharp corners on all four sides.
    Sharp,
    /// Thick single-line border on all four sides.
    Bold,
    /// Double-line border on all four sides.
    Double,
    /// Block border (quadrant-outside) on all four sides.
    Block,
    /// Thin block border (quadrant-inside) on all four sides.
    ThinBlock,
    /// Plain border on top and bottom only.
    Horizontal,
    /// Plain border on left and right only.
    Vertical,
    /// A single horizontal rule on top.
    Line,
    /// Plain border on top only.
    Top,
    /// Plain border on bottom only.
    Bottom,
    /// Plain border on left only.
    Left,
    /// Plain border on right only.
    Right,
}

impl BorderStyle {
    /// Returns the `(sides, border_type)` pair this style resolves to.
    ///
    /// [`BorderStyle::None`] returns `(Borders::NONE, BorderType::Plain)`;
    /// callers typically inspect the returned [`Borders`] to decide
    /// whether to draw a block at all.
    pub fn ratatui_parts(self) -> (Borders, BorderType) {
        match self {
            BorderStyle::None => (Borders::NONE, BorderType::Plain),
            BorderStyle::Rounded => (Borders::ALL, BorderType::Rounded),
            BorderStyle::Sharp => (Borders::ALL, BorderType::Plain),
            BorderStyle::Bold => (Borders::ALL, BorderType::Thick),
            BorderStyle::Double => (Borders::ALL, BorderType::Double),
            BorderStyle::Block => (Borders::ALL, BorderType::QuadrantOutside),
            BorderStyle::ThinBlock => (Borders::ALL, BorderType::QuadrantInside),
            BorderStyle::Horizontal => (Borders::TOP | Borders::BOTTOM, BorderType::Plain),
            BorderStyle::Vertical => (Borders::LEFT | Borders::RIGHT, BorderType::Plain),
            BorderStyle::Line | BorderStyle::Top => (Borders::TOP, BorderType::Plain),
            BorderStyle::Bottom => (Borders::BOTTOM, BorderType::Plain),
            BorderStyle::Left => (Borders::LEFT, BorderType::Plain),
            BorderStyle::Right => (Borders::RIGHT, BorderType::Plain),
        }
    }

    /// Returns `true` when this style resolves to a visible border.
    pub fn is_visible(self) -> bool {
        !matches!(self, BorderStyle::None)
    }

    /// Returns `true` when this style draws a bottom-edge segment.
    ///
    /// Used by the standalone runner to decide whether the help-hint
    /// footer should be inlined into the bottom border (via
    /// [`FrameChromeConfig::bottom_label`]) or rendered on its own row.
    pub fn has_bottom(self) -> bool {
        let (sides, _) = self.ratatui_parts();
        sides.contains(Borders::BOTTOM)
    }
}

/// Four-sided margin in terminal cells.
///
/// Margins are applied outside of any border — i.e. they shrink the
/// available area before the border is drawn. Use [`Margin::uniform`]
/// for a single value on all four sides, then optionally override
/// specific sides via struct update syntax.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Margin {
    /// Cells of margin on the top edge.
    pub top: u16,
    /// Cells of margin on the bottom edge.
    pub bottom: u16,
    /// Cells of margin on the left edge.
    pub left: u16,
    /// Cells of margin on the right edge.
    pub right: u16,
}

impl Margin {
    /// Constructs a margin with the same value on all four sides.
    pub const fn uniform(n: u16) -> Self {
        Self {
            top: n,
            bottom: n,
            left: n,
            right: n,
        }
    }

    /// Shrinks `area` by the margin on each side, using saturating
    /// subtraction so overly large margins collapse the rect rather
    /// than wrapping.
    pub fn shrink(self, area: Rect) -> Rect {
        let dx = self.left.min(area.width);
        let dy = self.top.min(area.height);
        let x = area.x.saturating_add(dx);
        let y = area.y.saturating_add(dy);
        let width = area
            .width
            .saturating_sub(dx)
            .saturating_sub(self.right.min(area.width.saturating_sub(dx)));
        let height = area
            .height
            .saturating_sub(dy)
            .saturating_sub(self.bottom.min(area.height.saturating_sub(dy)));
        Rect {
            x,
            y,
            width,
            height,
        }
    }
}

/// Four-sided interior padding in terminal cells.
///
/// Padding is applied inside the border (or directly inside the margin
/// when no border is drawn). It shrinks the area available to the
/// inner widget after the border has claimed its perimeter.
///
/// Defaults to `1` cell on all sides (matches the spec) so widgets do
/// not visually touch the border. Use [`Padding::zero`] (or its
/// [`Padding::none`] alias) when chrome should have no interior
/// spacing at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Padding {
    /// Cells of padding on the top edge.
    pub top: u16,
    /// Cells of padding on the bottom edge.
    pub bottom: u16,
    /// Cells of padding on the left edge.
    pub left: u16,
    /// Cells of padding on the right edge.
    pub right: u16,
}

impl Default for Padding {
    /// Returns `Padding::uniform(1)`.
    ///
    /// This matches the spec's library-level default: chrome adds one
    /// cell of interior padding on every side so widgets do not bump
    /// against the border. Use [`Padding::zero`] for no padding at
    /// all.
    fn default() -> Self {
        Self::uniform(1)
    }
}

impl Padding {
    /// Constructs padding with the same value on all four sides.
    pub const fn uniform(n: u16) -> Self {
        Self {
            top: n,
            bottom: n,
            left: n,
            right: n,
        }
    }

    /// Constructs padding with `0` on every side.
    ///
    /// Prefer this over `Padding { top: 0, .. }` when expressing
    /// "no interior spacing" — it documents the intent and decouples
    /// the call site from the [`Default`] impl, which is `uniform(1)`.
    pub const fn zero() -> Self {
        Self::uniform(0)
    }

    /// Alias for [`Padding::zero`].
    ///
    /// Reads naturally at call sites that want to opt out of any
    /// padding entirely (`Padding::none()`).
    pub const fn none() -> Self {
        Self::zero()
    }

    /// Shrinks `area` by the padding on each side, using saturating
    /// subtraction so overly large padding collapses the rect rather
    /// than wrapping.
    pub fn shrink(self, area: Rect) -> Rect {
        let dx = self.left.min(area.width);
        let dy = self.top.min(area.height);
        let x = area.x.saturating_add(dx);
        let y = area.y.saturating_add(dy);
        let width = area
            .width
            .saturating_sub(dx)
            .saturating_sub(self.right.min(area.width.saturating_sub(dx)));
        let height = area
            .height
            .saturating_sub(dy)
            .saturating_sub(self.bottom.min(area.height.saturating_sub(dy)));
        Rect {
            x,
            y,
            width,
            height,
        }
    }
}

/// The parsed form of the CLI `--height` flag.
///
/// Both variants are treated as a *maximum* — when the live terminal is
/// smaller than the requested height, the inline viewport is clamped to
/// the available rows so the prompt never overflows the screen.
///
/// ## Variants
///
/// - [`HeightSpec::Cells`] is an absolute cell count, clamped to the
///   terminal height when resolved. Resolved once at prompt start;
///   ratatui's own `autoresize` clamps the inline viewport to the live
///   terminal height on subsequent resizes, so the cap never exceeds
///   the terminal even after a shrink.
/// - [`HeightSpec::Percent`] is a percentage of the terminal height,
///   clamped to a floor of 3 rows so the list always has room for a
///   header plus two rows. Re-resolved on every resize event by
///   [`crate::core::run_standalone`], so the inline viewport tracks
///   the requested fraction of the terminal as it grows or shrinks
///   mid-prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeightSpec {
    /// An absolute cell count, treated as a maximum.
    Cells(u16),
    /// A percentage of the current terminal height (1..=100), tracked
    /// dynamically across resize events.
    Percent(u8),
}

impl HeightSpec {
    /// Resolves this spec against the current terminal row count.
    ///
    /// `term_rows` is the total rows reported by
    /// `crossterm::terminal::size`. Percentages clamp to a floor of 3
    /// rows (so there is always room for the list plus an error row)
    /// and to a ceiling of `term_rows`. Cells clamp to the ceiling.
    pub fn resolve(&self, term_rows: u16) -> u16 {
        match self {
            HeightSpec::Cells(n) => (*n).min(term_rows),
            HeightSpec::Percent(p) => {
                let raw = (term_rows as u32 * (*p as u32)) / 100;
                raw.clamp(3, term_rows as u32) as u16
            }
        }
    }
}

/// Configuration consumed by [`FrameChrome`].
///
/// Produced by the CLI layer from the shared `ChooseChromeArgs`
/// struct. Later phases will flesh out parsing; Phase 1 just fixes
/// the shape so other call sites can be written against it.
#[derive(Debug, Clone)]
pub struct FrameChromeConfig {
    /// Border style; [`BorderStyle::None`] draws no border.
    pub border: BorderStyle,
    /// Optional title rendered in the top-left of the border.
    pub border_label: Option<String>,
    /// Optional title rendered in the bottom-left of the border.
    ///
    /// Used by the standalone runner to inline the help-hint footer
    /// inside the bottom border (matching how `border_label` inlines a
    /// title in the top border) so the hint never overwrites the
    /// border corners. Ignored when the resolved border has no
    /// [`Borders::BOTTOM`] segment.
    pub bottom_label: Option<String>,
    /// Margin applied outside of the border.
    pub margin: Margin,
    /// Style applied to the border glyphs.
    pub border_style: Style,
    /// Interior padding applied inside the border (or directly inside
    /// the margin when no border is drawn).
    pub padding: Padding,
    /// Preserve the final render after the prompt exits.
    ///
    /// When `false` (the default) the standalone runner clears the
    /// inline viewport on exit so the terminal reclaims the space
    /// (fzf-style). When `true` the final render is left in place and
    /// the cursor is positioned on a fresh line just below the chrome.
    pub show_on_exit: bool,
}

impl Default for FrameChromeConfig {
    /// The library-default config: no border, no margin, padding of
    /// `1` cell on every side.
    ///
    /// Padding is written as `Padding::uniform(1)` even though it
    /// matches `Padding::default()` — the explicit value documents the
    /// invariant at the call site so a reader does not have to follow
    /// the [`Default`] impl on [`Padding`] to understand the chrome's
    /// resting state.
    fn default() -> Self {
        Self {
            border: BorderStyle::default(),
            border_label: None,
            bottom_label: None,
            margin: Margin::default(),
            border_style: Style::default(),
            padding: Padding::uniform(1),
            show_on_exit: false,
        }
    }
}

impl FrameChromeConfig {
    /// Returns `true` when neither the border, margin, nor padding
    /// would affect layout. Callers can use this to skip wrapping a
    /// widget in [`FrameChrome`] entirely.
    ///
    /// Padding is compared against [`Padding::zero`] rather than
    /// [`Padding::default`] because the library default is
    /// `uniform(1)` — a configuration with the default padding still
    /// affects layout and must not be treated as empty.
    pub fn is_empty(&self) -> bool {
        !self.border.is_visible()
            && self.margin == Margin::default()
            && self.padding == Padding::zero()
    }
}

/// A stateful wrapper that draws an optional border and margin around
/// an inner widget.
///
/// [`FrameChrome`] implements [`StatefulWidget`], forwarding to the
/// inner widget's [`StatefulWidget::State`] unchanged. The inner
/// widget renders inside the rectangle that remains after the margin
/// has been subtracted, (if any) the border has claimed its own
/// single-cell perimeter, and padding has been applied inside the
/// border.
pub struct FrameChrome<'a, W> {
    /// The widget drawn inside the frame.
    pub inner: W,
    /// Optional border spec: `(sides, border_type, top_label, bottom_label)`.
    pub border: Option<(Borders, BorderType, Option<&'a str>, Option<&'a str>)>,
    /// Margin applied outside of the border.
    pub margin: Margin,
    /// Style applied to the border glyphs.
    pub border_style: Style,
    /// Interior padding applied inside the border.
    pub padding: Padding,
}

impl<'a, W> FrameChrome<'a, W> {
    /// Wraps `inner` with no chrome (margin zero, border none, padding
    /// zero). Used as the no-op fallback when
    /// `FrameChromeConfig::is_empty()` returns true but the caller
    /// still wants to go through the wrapper for type uniformity.
    ///
    /// Note: padding is set to [`Padding::zero`] (not
    /// [`Padding::default`], which is `uniform(1)`) so the bare
    /// wrapper truly applies no chrome.
    pub fn bare(inner: W) -> Self {
        Self {
            inner,
            border: None,
            margin: Margin::default(),
            border_style: Style::default(),
            padding: Padding::zero(),
        }
    }

    /// Builds a frame from `inner` and a `config`.
    ///
    /// The config's `border_label` is borrowed by the returned frame,
    /// so the config must outlive the frame.
    pub fn from_config(inner: W, config: &'a FrameChromeConfig) -> Self {
        let border = if config.border.is_visible() {
            let (sides, ty) = config.border.ratatui_parts();
            Some((
                sides,
                ty,
                config.border_label.as_deref(),
                config.bottom_label.as_deref(),
            ))
        } else {
            None
        };
        Self {
            inner,
            border,
            margin: config.margin,
            border_style: config.border_style,
            padding: config.padding,
        }
    }
}

impl<W> StatefulWidget for FrameChrome<'_, W>
where
    W: StatefulWidget,
{
    type State = W::State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = self.margin.shrink(area);
        let inner_area = if let Some((sides, ty, top_label, bottom_label)) = self.border {
            let mut block = Block::default()
                .borders(sides)
                .border_type(ty)
                .style(self.border_style);
            if let Some(l) = top_label {
                block = block.title(l.to_string());
            }
            if let Some(l) = bottom_label
                && sides.contains(Borders::BOTTOM)
            {
                block = block.title_bottom(l.to_string());
            }
            let inner = block.inner(area);
            block.render(area, buf);
            self.padding.shrink(inner)
        } else {
            self.padding.shrink(area)
        };
        self.inner.render(inner_area, buf, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn margin_uniform() {
        let m = Margin::uniform(3);
        assert_eq!(m.top, 3);
        assert_eq!(m.bottom, 3);
        assert_eq!(m.left, 3);
        assert_eq!(m.right, 3);
    }

    #[test]
    fn margin_per_side_overrides() {
        let m = Margin {
            top: 0,
            ..Margin::uniform(2)
        };
        assert_eq!(m.top, 0);
        assert_eq!(m.bottom, 2);
        assert_eq!(m.left, 2);
        assert_eq!(m.right, 2);
    }

    #[test]
    fn margin_shrink_subtracts_each_side() {
        let area = Rect::new(0, 0, 20, 10);
        let shrunk = Margin::uniform(2).shrink(area);
        assert_eq!(shrunk.x, 2);
        assert_eq!(shrunk.y, 2);
        assert_eq!(shrunk.width, 16);
        assert_eq!(shrunk.height, 6);
    }

    #[test]
    fn margin_shrink_saturates_when_too_large() {
        let area = Rect::new(0, 0, 4, 2);
        let shrunk = Margin::uniform(10).shrink(area);
        assert_eq!(shrunk.width, 0);
        assert_eq!(shrunk.height, 0);
    }

    #[test]
    fn height_spec_percent_clamps_to_floor_3() {
        assert_eq!(HeightSpec::Percent(10).resolve(10), 3);
        assert_eq!(HeightSpec::Percent(1).resolve(40), 3);
    }

    #[test]
    fn height_spec_percent_standard_case() {
        assert_eq!(HeightSpec::Percent(50).resolve(40), 20);
        assert_eq!(HeightSpec::Percent(25).resolve(40), 10);
    }

    #[test]
    fn height_spec_percent_100_resolves_to_term_rows() {
        assert_eq!(HeightSpec::Percent(100).resolve(40), 40);
    }

    #[test]
    fn height_spec_cells_caps_to_term_rows() {
        assert_eq!(HeightSpec::Cells(100).resolve(40), 40);
        assert_eq!(HeightSpec::Cells(20).resolve(40), 20);
    }

    #[test]
    fn border_style_none_is_default() {
        assert_eq!(BorderStyle::default(), BorderStyle::None);
        assert!(!BorderStyle::None.is_visible());
        assert!(BorderStyle::Rounded.is_visible());
    }

    #[test]
    fn border_style_maps_all_variants() {
        let (sides, ty) = BorderStyle::Rounded.ratatui_parts();
        assert_eq!(sides, Borders::ALL);
        assert_eq!(ty, BorderType::Rounded);

        let (sides, _) = BorderStyle::Horizontal.ratatui_parts();
        assert!(sides.contains(Borders::TOP));
        assert!(sides.contains(Borders::BOTTOM));
        assert!(!sides.contains(Borders::LEFT));

        let (sides, _) = BorderStyle::Vertical.ratatui_parts();
        assert!(sides.contains(Borders::LEFT));
        assert!(sides.contains(Borders::RIGHT));
        assert!(!sides.contains(Borders::TOP));

        let (sides, _) = BorderStyle::Top.ratatui_parts();
        assert_eq!(sides, Borders::TOP);

        let (sides, _) = BorderStyle::Line.ratatui_parts();
        assert_eq!(sides, Borders::TOP);

        let (sides, _) = BorderStyle::Bottom.ratatui_parts();
        assert_eq!(sides, Borders::BOTTOM);

        let (sides, _) = BorderStyle::Left.ratatui_parts();
        assert_eq!(sides, Borders::LEFT);

        let (sides, _) = BorderStyle::Right.ratatui_parts();
        assert_eq!(sides, Borders::RIGHT);

        let (_, ty) = BorderStyle::Bold.ratatui_parts();
        assert_eq!(ty, BorderType::Thick);

        let (_, ty) = BorderStyle::Double.ratatui_parts();
        assert_eq!(ty, BorderType::Double);

        let (_, ty) = BorderStyle::Block.ratatui_parts();
        assert_eq!(ty, BorderType::QuadrantOutside);

        let (_, ty) = BorderStyle::ThinBlock.ratatui_parts();
        assert_eq!(ty, BorderType::QuadrantInside);
    }

    #[test]
    fn frame_chrome_config_default_padding_is_uniform_one() {
        let config = FrameChromeConfig::default();
        assert_eq!(config.padding, Padding::uniform(1));
    }

    #[test]
    fn frame_chrome_config_is_empty_only_when_padding_is_zero() {
        // The library-level default padding is now `uniform(1)`, so a
        // config built from `Default::default()` is *not* empty — the
        // padding alone affects layout. Explicit `Padding::zero()`
        // restores the empty state.
        let default_config = FrameChromeConfig::default();
        assert!(!default_config.is_empty());

        let zeroed = FrameChromeConfig {
            padding: Padding::zero(),
            ..Default::default()
        };
        assert!(zeroed.is_empty());
    }

    #[test]
    fn frame_chrome_config_is_empty_uses_padding_none_alias() {
        let zeroed = FrameChromeConfig {
            padding: Padding::none(),
            ..Default::default()
        };
        assert!(zeroed.is_empty());
    }

    #[test]
    fn frame_chrome_config_with_border_is_not_empty() {
        let config = FrameChromeConfig {
            border: BorderStyle::Rounded,
            ..Default::default()
        };
        assert!(!config.is_empty());
    }

    #[test]
    fn frame_chrome_config_with_margin_is_not_empty() {
        let config = FrameChromeConfig {
            margin: Margin::uniform(1),
            ..Default::default()
        };
        assert!(!config.is_empty());
    }

    /// A minimal stateful widget used to exercise [`FrameChrome`]'s
    /// render path.
    struct Marker;

    #[derive(Default)]
    struct MarkerState {
        last_area: Option<Rect>,
    }

    impl StatefulWidget for Marker {
        type State = MarkerState;

        fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
            state.last_area = Some(area);
            if area.width > 0 && area.height > 0 {
                buf.set_string(area.x, area.y, "X", Style::default());
            }
        }
    }

    #[test]
    fn frame_chrome_renders_block_then_inner() {
        let config = FrameChromeConfig {
            border: BorderStyle::Rounded,
            border_label: Some("Title".to_string()),
            bottom_label: None,
            margin: Margin::uniform(1),
            border_style: Style::default().fg(Color::Cyan),
            padding: Padding::zero(),
            show_on_exit: false,
        };
        let frame = FrameChrome::from_config(Marker, &config);

        let area = Rect::new(0, 0, 10, 6);
        let mut buf = Buffer::empty(area);
        let mut state = MarkerState::default();
        frame.render(area, &mut buf, &mut state);

        // The inner widget received the rect left over after the
        // margin (1 on each side) and the border (1 on each side).
        let inner = state.last_area.expect("inner widget rendered");
        assert_eq!(inner.x, 2);
        assert_eq!(inner.y, 2);
        assert_eq!(inner.width, 6);
        assert_eq!(inner.height, 2);
    }

    #[test]
    fn frame_chrome_bare_applies_no_chrome() {
        let frame = FrameChrome::bare(Marker);
        let area = Rect::new(0, 0, 10, 6);
        let mut buf = Buffer::empty(area);
        let mut state = MarkerState::default();
        frame.render(area, &mut buf, &mut state);

        let inner = state.last_area.expect("inner widget rendered");
        assert_eq!(inner, area);
    }

    #[test]
    fn frame_chrome_margin_only_still_renders_inner() {
        let config = FrameChromeConfig {
            margin: Margin::uniform(1),
            padding: Padding::zero(),
            ..Default::default()
        };
        let frame = FrameChrome::from_config(Marker, &config);

        let area = Rect::new(0, 0, 10, 6);
        let mut buf = Buffer::empty(area);
        let mut state = MarkerState::default();
        frame.render(area, &mut buf, &mut state);

        let inner = state.last_area.expect("inner widget rendered");
        assert_eq!(inner.x, 1);
        assert_eq!(inner.y, 1);
        assert_eq!(inner.width, 8);
        assert_eq!(inner.height, 4);
    }

    #[test]
    fn border_label_truncates_when_narrow() {
        // ratatui's Block silently truncates the title to the
        // available border width; we render into a buffer and sanity
        // check that a long label does not cause a panic or leak into
        // the inner area.
        let config = FrameChromeConfig {
            border: BorderStyle::Rounded,
            border_label: Some("A very long title indeed".to_string()),
            padding: Padding::zero(),
            ..Default::default()
        };
        let frame = FrameChrome::from_config(Marker, &config);

        let area = Rect::new(0, 0, 8, 4);
        let mut buf = Buffer::empty(area);
        let mut state = MarkerState::default();
        frame.render(area, &mut buf, &mut state);

        let inner = state.last_area.expect("inner widget rendered");
        // Inner shrinks by one cell on every side for the border.
        assert_eq!(inner.x, 1);
        assert_eq!(inner.y, 1);
        assert_eq!(inner.width, 6);
        assert_eq!(inner.height, 2);
    }

    #[test]
    fn frame_chrome_padding_shrinks_inner_even_without_border() {
        let config = FrameChromeConfig {
            padding: Padding::uniform(2),
            ..Default::default()
        };
        let frame = FrameChrome::from_config(Marker, &config);

        let area = Rect::new(0, 0, 10, 6);
        let mut buf = Buffer::empty(area);
        let mut state = MarkerState::default();
        frame.render(area, &mut buf, &mut state);

        let inner = state.last_area.expect("inner widget rendered");
        assert_eq!(inner.x, 2);
        assert_eq!(inner.y, 2);
        assert_eq!(inner.width, 6);
        assert_eq!(inner.height, 2);
    }

    #[test]
    fn frame_chrome_border_plus_padding_shrinks_inner() {
        let config = FrameChromeConfig {
            border: BorderStyle::Rounded,
            padding: Padding::uniform(1),
            ..Default::default()
        };
        let frame = FrameChrome::from_config(Marker, &config);

        let area = Rect::new(0, 0, 10, 6);
        let mut buf = Buffer::empty(area);
        let mut state = MarkerState::default();
        frame.render(area, &mut buf, &mut state);

        let inner = state.last_area.expect("inner widget rendered");
        // margin 0 + border 1 + padding 1 = 2 on each side
        assert_eq!(inner.x, 2);
        assert_eq!(inner.y, 2);
        assert_eq!(inner.width, 6);
        assert_eq!(inner.height, 2);
    }

    #[test]
    fn padding_uniform() {
        let p = Padding::uniform(3);
        assert_eq!(p.top, 3);
        assert_eq!(p.bottom, 3);
        assert_eq!(p.left, 3);
        assert_eq!(p.right, 3);
    }

    #[test]
    fn padding_shrink_subtracts_each_side() {
        let area = Rect::new(0, 0, 20, 10);
        let shrunk = Padding::uniform(2).shrink(area);
        assert_eq!(shrunk.x, 2);
        assert_eq!(shrunk.y, 2);
        assert_eq!(shrunk.width, 16);
        assert_eq!(shrunk.height, 6);
    }

    #[test]
    fn padding_shrink_saturates_when_too_large() {
        let area = Rect::new(0, 0, 4, 2);
        let shrunk = Padding::uniform(10).shrink(area);
        assert_eq!(shrunk.width, 0);
        assert_eq!(shrunk.height, 0);
    }

    #[test]
    fn padding_default_is_uniform_one() {
        // Phase 2 of the choose-one improvements feature: the
        // library-level default for `Padding` is `uniform(1)` so
        // chrome-bearing widgets never visually touch the border.
        assert_eq!(Padding::default(), Padding::uniform(1));
        let p = Padding::default();
        assert_eq!(p.top, 1);
        assert_eq!(p.bottom, 1);
        assert_eq!(p.left, 1);
        assert_eq!(p.right, 1);
    }

    #[test]
    fn padding_zero_is_all_zeros() {
        let p = Padding::zero();
        assert_eq!(p.top, 0);
        assert_eq!(p.bottom, 0);
        assert_eq!(p.left, 0);
        assert_eq!(p.right, 0);
        assert_eq!(p, Padding::uniform(0));
        // Confirms the Default impl is *not* zero: Padding::default()
        // and Padding::zero() must now diverge.
        assert_ne!(Padding::default(), Padding::zero());
    }

    #[test]
    fn padding_none_alias_matches_zero() {
        assert_eq!(Padding::none(), Padding::zero());
    }

    #[test]
    fn padding_default_shrinks_rect_by_one_on_each_side() {
        let area = Rect::new(0, 0, 10, 6);
        let shrunk = Padding::default().shrink(area);
        assert_eq!(shrunk.x, 1);
        assert_eq!(shrunk.y, 1);
        assert_eq!(shrunk.width, 8);
        assert_eq!(shrunk.height, 4);
    }

    #[test]
    fn border_style_has_bottom_only_for_styles_that_draw_bottom_segment() {
        // Styles that include the BOTTOM segment in `ratatui_parts`.
        for style in [
            BorderStyle::Rounded,
            BorderStyle::Sharp,
            BorderStyle::Bold,
            BorderStyle::Double,
            BorderStyle::Block,
            BorderStyle::ThinBlock,
            BorderStyle::Horizontal,
            BorderStyle::Bottom,
        ] {
            assert!(
                style.has_bottom(),
                "{style:?} should report has_bottom() = true",
            );
        }
        // Styles that do not draw a bottom edge.
        for style in [
            BorderStyle::None,
            BorderStyle::Vertical,
            BorderStyle::Line,
            BorderStyle::Top,
            BorderStyle::Left,
            BorderStyle::Right,
        ] {
            assert!(
                !style.has_bottom(),
                "{style:?} should report has_bottom() = false",
            );
        }
    }

    #[test]
    fn frame_chrome_default_show_on_exit_is_false() {
        // The library default preserves backward-compatible behaviour
        // (clear-on-exit happens when `show_on_exit == false`).
        assert!(!FrameChromeConfig::default().show_on_exit);
        assert!(FrameChromeConfig::default().bottom_label.is_none());
    }

    #[test]
    fn frame_chrome_renders_bottom_label_inside_bottom_border() {
        // The bottom label should appear on the bottom border row,
        // leaving the corner glyphs at column 0 and column area.right - 1
        // intact.
        let config = FrameChromeConfig {
            border: BorderStyle::Rounded,
            border_label: None,
            bottom_label: Some("Hint".to_string()),
            padding: Padding::zero(),
            ..Default::default()
        };
        let frame = FrameChrome::from_config(Marker, &config);

        let area = Rect::new(0, 0, 16, 4);
        let mut buf = Buffer::empty(area);
        let mut state = MarkerState::default();
        frame.render(area, &mut buf, &mut state);

        let bottom_row = (0..area.width)
            .map(|x| buf[(x, area.height - 1)].symbol().to_string())
            .collect::<String>();
        assert!(
            bottom_row.contains("Hint"),
            "bottom border row should contain the label; got {bottom_row:?}",
        );
        // Corners must survive — neither column 0 nor the rightmost
        // column should be a space on the bottom row.
        assert_ne!(
            buf[(0, area.height - 1)].symbol(),
            " ",
            "bottom-left corner glyph clobbered",
        );
        assert_ne!(
            buf[(area.width - 1, area.height - 1)].symbol(),
            " ",
            "bottom-right corner glyph clobbered",
        );
    }

    #[test]
    fn frame_chrome_ignores_bottom_label_when_no_bottom_border() {
        // BorderStyle::Top draws only a top border, so a bottom_label
        // must be silently ignored — there is no bottom row to host it.
        let config = FrameChromeConfig {
            border: BorderStyle::Top,
            bottom_label: Some("Hint".to_string()),
            padding: Padding::zero(),
            ..Default::default()
        };
        let frame = FrameChrome::from_config(Marker, &config);

        let area = Rect::new(0, 0, 16, 4);
        let mut buf = Buffer::empty(area);
        let mut state = MarkerState::default();
        frame.render(area, &mut buf, &mut state);

        let bottom_row = (0..area.width)
            .map(|x| buf[(x, area.height - 1)].symbol().to_string())
            .collect::<String>();
        assert!(
            !bottom_row.contains("Hint"),
            "bottom_label should not render when border has no bottom segment; got {bottom_row:?}",
        );
    }
}
