//! Geometry-only two-pane layout primitive.
//!
//! [`SplitPane`] divides a single [`Rect`] into two child rectangles
//! along one axis. Unlike the input components it captures no value and
//! handles no input — its only job is spatial. Call [`SplitPane::split`]
//! and render each child with its own `render_stateful_widget`.
//!
//! ## Examples
//!
//! ```
//! use biscuit_tui::core::{SplitDirection, SplitPane, SplitRatio};
//! use ratatui::layout::Rect;
//!
//! // A 30% sidebar against a 70% main pane, side by side.
//! let (sidebar, main) = SplitPane::new()
//!     .with_direction(SplitDirection::Horizontal)
//!     .with_ratio(SplitRatio::Percent(30))
//!     .split(Rect::new(0, 0, 100, 40));
//! assert_eq!(sidebar.width + main.width, 100);
//! ```

use ratatui::layout::Rect;

/// How a [`SplitPane`] is *asked* to arrange its two child panes.
///
/// This is the caller-facing *input* vocabulary. It includes `Auto`, an
/// intent that is resolved to a concrete axis at split time.
///
/// ## Examples
///
/// ```
/// use biscuit_tui::core::SplitDirection;
///
/// let _auto = SplitDirection::Auto;       // chosen from the area's shape
/// let _horz = SplitDirection::Horizontal; // side-by-side (left | right)
/// let _vert = SplitDirection::Vertical;   // stacked (top / bottom)
/// ```
///
/// ## Notes
///
/// - `Horizontal` maps to ratatui `Direction::Horizontal` (panes side by
///   side); `Vertical` maps to ratatui `Direction::Vertical` (panes
///   stacked). The first pane is the left (Horizontal) or top (Vertical)
///   one.
/// - `Auto` compares the area's raw cells (`width >= height`), so a
///   square area resolves to `Horizontal` via the `>=` tie-break.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SplitDirection {
    /// Pick the direction from the area's shape at split time. A
    /// wider-than-tall area splits `Horizontal` (side-by-side); a
    /// taller-than-wide area splits `Vertical` (stacked). A square area
    /// resolves to `Horizontal`. The default.
    #[default]
    Auto,
    /// Panes sit side-by-side (left | right). The first pane is the
    /// left one. Maps to ratatui `Direction::Horizontal`.
    Horizontal,
    /// Panes stack one over the other (top / bottom). The first pane is
    /// the top one. Maps to ratatui `Direction::Vertical`.
    Vertical,
}

/// The concrete split axis, after [`SplitDirection::Auto`] has been
/// resolved against a specific area.
///
/// Total over the two real axes — there is no `Auto`, so the geometry
/// code never needs an `unreachable!` arm.
///
/// Crate-private: `Auto` resolution is an implementation detail in v1.
/// A future divider glyph or draggable divider should accept a
/// `ResolvedAxis` so the compiler guarantees resolution already
/// happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolvedAxis {
    Horizontal,
    Vertical,
}

impl SplitDirection {
    /// Resolves `Auto` against `area`'s shape; passes explicit
    /// directions through unchanged. Total: never returns an
    /// unresolved value.
    ///
    /// Raw cells are compared (`width >= height`), so a square area
    /// resolves to `Horizontal` via the `>=` tie-break.
    fn resolve(self, area: Rect) -> ResolvedAxis {
        match self {
            SplitDirection::Horizontal => ResolvedAxis::Horizontal,
            SplitDirection::Vertical => ResolvedAxis::Vertical,
            SplitDirection::Auto if area.width >= area.height => ResolvedAxis::Horizontal,
            SplitDirection::Auto => ResolvedAxis::Vertical,
        }
    }
}

/// The relative share of space given to each pane of a [`SplitPane`].
///
/// No variant ever *voluntarily* starves a pane to zero — `Percent` is
/// clamped to `1..=99` and the `*Fixed` variants to `>= 1` on
/// construction. The only case where a pane reaches zero is the
/// genuinely degenerate one documented on [`SplitPane::split`].
///
/// ## Examples
///
/// ```
/// use biscuit_tui::core::SplitRatio;
///
/// assert_eq!(SplitRatio::default(), SplitRatio::Percent(50));
/// assert_eq!(SplitRatio::percent(0), SplitRatio::Percent(1));    // clamped
/// assert_eq!(SplitRatio::percent(100), SplitRatio::Percent(99)); // clamped
/// assert_eq!(SplitRatio::first_fixed(0), SplitRatio::FirstFixed(1));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SplitRatio {
    /// First pane takes this percentage; the second takes `100 - p`.
    /// Clamped into `1..=99` on construction so neither pane is starved.
    Percent(u8),
    /// First pane takes a fixed cell count (clamped to `>= 1` on
    /// construction); the second takes the rest. Useful for a
    /// fixed-width sidebar against a flexible main pane.
    FirstFixed(u16),
    /// Second pane takes a fixed cell count (clamped to `>= 1` on
    /// construction); the first takes the rest. Useful for a
    /// fixed-width detail panel on the right/bottom.
    SecondFixed(u16),
}

impl Default for SplitRatio {
    /// An even (`Percent(50)`) split.
    fn default() -> Self {
        Self::percent(50)
    }
}

impl SplitRatio {
    /// `Percent(p)` clamped into `1..=99` so neither pane is starved.
    pub fn percent(p: u8) -> Self {
        Self::Percent(p.clamp(1, 99))
    }

    /// `FirstFixed(n)` clamped to `>= 1`.
    pub fn first_fixed(n: u16) -> Self {
        Self::FirstFixed(n.max(1))
    }

    /// `SecondFixed(n)` clamped to `>= 1`.
    pub fn second_fixed(n: u16) -> Self {
        Self::SecondFixed(n.max(1))
    }

    /// Re-applies the construction clamps so a raw struct literal
    /// (e.g. `SplitRatio::Percent(0)`) cannot bypass the no-zero-pane
    /// invariant.
    fn normalize(self) -> Self {
        match self {
            Self::Percent(p) => Self::percent(p),
            Self::FirstFixed(n) => Self::first_fixed(n),
            Self::SecondFixed(n) => Self::second_fixed(n),
        }
    }
}

/// Splits a rectangle into two panes along one axis.
///
/// Geometry-only by design: [`SplitPane::split`] computes child
/// rectangles and renders nothing. Call it and render each child with
/// its own `render_stateful_widget` — that is the idiomatic path.
///
/// ## Examples
///
/// ```
/// use biscuit_tui::core::SplitPane;
/// use ratatui::layout::Rect;
///
/// // 50/50 default; direction resolved from the area's shape.
/// let (left, right) = SplitPane::new().split(Rect::new(0, 0, 80, 24));
/// assert_eq!(left.width, 40);
/// assert_eq!(right.width, 40);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SplitPane {
    /// Side-by-side vs. stacked. Defaults to `Auto` (chosen from the
    /// area's shape at split time).
    pub direction: SplitDirection,
    /// Relative share for each pane. Defaults to 50/50.
    pub ratio: SplitRatio,
    /// Cells of empty space reserved *between* the two panes (a
    /// gutter). Defaults to `0`. A value of `1` is the natural home for
    /// a future divider glyph.
    pub gap: u16,
}

impl SplitPane {
    /// Even (50/50) auto-direction split with no gap.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder: set the direction.
    pub fn with_direction(mut self, direction: SplitDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Builder: set the ratio, re-normalized through the construction
    /// clamps so a raw `SplitRatio` literal cannot bypass the invariant.
    pub fn with_ratio(mut self, ratio: SplitRatio) -> Self {
        self.ratio = ratio.normalize();
        self
    }

    /// Builder: set the inter-pane gap in cells.
    pub fn with_gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Computes the two child rectangles for `area`.
    ///
    /// Returns `(first, second)` where `first` is the left pane (for a
    /// resolved `Horizontal`) or the top pane (for a resolved
    /// `Vertical`). On an odd split-axis length the **first** pane
    /// absorbs the spare cell (a 50/50 split of 9 ⇒ first 5, second 4).
    /// Both rects lie within `area`; the cross axis passes through at
    /// `area`'s full extent.
    ///
    /// ## Degenerate inputs
    ///
    /// `split` never panics and never overflows:
    ///
    /// - A zero-sized `area` yields two zero-sized rects.
    /// - A `*Fixed` length ≥ the available axis collapses the flexible
    ///   pane to zero (the only case a pane reaches zero).
    /// - A `gap` ≥ the split-axis length is clamped to that length;
    ///   both panes collapse to zero.
    pub fn split(&self, area: Rect) -> (Rect, Rect) {
        let ratio = self.ratio.normalize();
        let axis = self.direction.resolve(area);

        let axis_len = match axis {
            ResolvedAxis::Horizontal => area.width,
            ResolvedAxis::Vertical => area.height,
        };

        // Clamp the gap to the split-axis length so it can never exceed
        // the area; the channel then sits between the two panes.
        let gap = self.gap.min(axis_len);
        let available = axis_len.saturating_sub(gap);

        // Pane lengths are computed directly rather than via ratatui's
        // layout solver: the spare-cell rule (first absorbs it) and the
        // degenerate collapses above must hold deterministically, and
        // the solver's rounding / Length+Min collision behaviour is not
        // part of this crate's contract.
        let (first_len, second_len) = pane_lengths(ratio, available);

        place(axis, area, first_len, second_len, gap)
    }
}

/// Computes `(first, second)` pane lengths along the split axis for
/// `available` cells (already gap-removed).
fn pane_lengths(ratio: SplitRatio, available: u16) -> (u16, u16) {
    match ratio {
        SplitRatio::Percent(p) => {
            if available == 0 {
                return (0, 0);
            }
            // First absorbs the spare cell via ceiling division.
            let first = ((available as u32 * p as u32).div_ceil(100)) as u16;
            // Hold the no-zero-pane invariant when two panes fit: cap
            // first so the second keeps at least one cell. (available is
            // >= 2 here, so `available - 1` cannot underflow.)
            let first = if available >= 2 {
                first.min(available - 1)
            } else {
                first
            };
            (first, available - first)
        }
        // Fixed clamped to available; the flex pane takes the rest and
        // collapses to zero when the fixed length already fills it.
        SplitRatio::FirstFixed(n) => {
            let first = n.min(available);
            (first, available - first)
        }
        SplitRatio::SecondFixed(n) => {
            let second = n.min(available);
            (available - second, second)
        }
    }
}

/// Positions the two panes inside `area` along the resolved `axis`,
/// leaving a `gap`-cell channel between them. The cross axis passes
/// through at `area`'s full extent.
fn place(
    axis: ResolvedAxis,
    area: Rect,
    first_len: u16,
    second_len: u16,
    gap: u16,
) -> (Rect, Rect) {
    match axis {
        ResolvedAxis::Horizontal => {
            let first = Rect::new(area.x, area.y, first_len, area.height);
            let second_x = area
                .x
                .saturating_add(first_len)
                .saturating_add(gap);
            let second = Rect::new(second_x, area.y, second_len, area.height);
            (first, second)
        }
        ResolvedAxis::Vertical => {
            let first = Rect::new(area.x, area.y, area.width, first_len);
            let second_y = area
                .y
                .saturating_add(first_len)
                .saturating_add(gap);
            let second = Rect::new(area.x, second_y, area.width, second_len);
            (first, second)
        }
    }
}

#[cfg(test)]
mod tests {
    //! Geometry suite covering the §7.1 acceptance invariant and every
    //! enumerated case in §7.2. Pure integer math — no terminal, no
    //! backend — so the contracts hold identically on macOS, Linux, and
    //! Windows.

    use super::*;
    use ratatui::layout::Rect;

    // --- §7.1 acceptance invariant helper (plan task 2.1) ----------------

    /// Asserts the §7.1 acceptance invariant for `(split, area)`:
    ///
    /// - both child rects lie entirely within `area`;
    /// - the two rects are non-overlapping modulo the `gap` channel;
    /// - along the split axis, `first + gap + second == area.len` (the
    ///   gap is clamped to the axis length, so this holds even in the
    ///   §5.2 degenerate collapses);
    /// - along the cross axis both panes span `area`'s full extent;
    /// - never overflows `area`.
    ///
    /// Calling `split.direction.resolve(area)` exercises the real
    /// resolution rather than a parallel reimplementation.
    fn assert_acceptance_invariant(split: &SplitPane, area: Rect) {
        let (first, second) = split.split(area);
        let axis = split.direction.resolve(area);
        let axis_len = match axis {
            ResolvedAxis::Horizontal => area.width,
            ResolvedAxis::Vertical => area.height,
        };
        // Matches `split()`'s own clamp: a gap past the axis length is
        // held to that length so the channel can never exceed `area`.
        let gap = split.gap.min(axis_len);

        assert_within(first, area, "first");
        assert_within(second, area, "second");

        match axis {
            ResolvedAxis::Horizontal => {
                // Cross axis (height) passes through at full extent.
                assert_eq!(first.y, area.y, "first.y == area.y");
                assert_eq!(second.y, area.y, "second.y == area.y");
                assert_eq!(first.height, area.height, "first cross-axis full extent");
                assert_eq!(second.height, area.height, "second cross-axis full extent");

                assert!(
                    second.x as u32 >= first.x as u32 + first.width as u32,
                    "panes must not overlap",
                );
                let channel = second.x as u32 - (first.x as u32 + first.width as u32);
                assert_eq!(channel, gap as u32, "gap channel sits between panes");
                assert_eq!(
                    first.width as u32 + gap as u32 + second.width as u32,
                    area.width as u32,
                    "split-axis sum == area width",
                );
            }
            ResolvedAxis::Vertical => {
                assert_eq!(first.x, area.x, "first.x == area.x");
                assert_eq!(second.x, area.x, "second.x == area.x");
                assert_eq!(first.width, area.width, "first cross-axis full extent");
                assert_eq!(second.width, area.width, "second cross-axis full extent");

                assert!(
                    second.y as u32 >= first.y as u32 + first.height as u32,
                    "panes must not overlap",
                );
                let channel = second.y as u32 - (first.y as u32 + first.height as u32);
                assert_eq!(channel, gap as u32, "gap channel sits between panes");
                assert_eq!(
                    first.height as u32 + gap as u32 + second.height as u32,
                    area.height as u32,
                    "split-axis sum == area height",
                );
            }
        }
    }

    /// Asserts `rect` lies entirely within `area`. `u32` arithmetic avoids
    /// the `u16` overflow in ratatui's `Rect::right`/`bottom` on inputs
    /// near `u16::MAX`.
    fn assert_within(rect: Rect, area: Rect, ctx: &str) {
        assert!(rect.x >= area.x, "{ctx}: x before area origin");
        assert!(rect.y >= area.y, "{ctx}: y before area origin");
        assert!(
            rect.x as u32 + rect.width as u32 <= area.x as u32 + area.width as u32,
            "{ctx}: right edge past area",
        );
        assert!(
            rect.y as u32 + rect.height as u32 <= area.y as u32 + area.height as u32,
            "{ctx}: bottom edge past area",
        );
    }

    // --- §7.2 direction & ratio tests (plan task 2.2) --------------------

    #[test]
    fn even_split_halves_exactly() {
        let (a, b) = SplitPane::new().split(Rect::new(0, 0, 80, 24));
        assert_eq!(a.width, 40);
        assert_eq!(b.width, 40);
        // Cross axis passes through at full extent.
        assert_eq!(a.height, 24);
        assert_eq!(b.height, 24);
        assert_eq!(a.x, 0);
        assert_eq!(b.x, 40);
        assert_eq!(a.y, 0);
        assert_eq!(b.y, 0);
    }

    #[test]
    fn odd_axis_gives_first_pane_the_spare_cell() {
        // 9 cells at 50/50 ⇒ first 5, second 4 (spec §4.3).
        let (a, b) = SplitPane::new().split(Rect::new(0, 0, 9, 5));
        assert_eq!(a.width, 5);
        assert_eq!(b.width, 4);
        assert_eq!(a.width + b.width, 9);
    }

    #[test]
    fn explicit_vertical_splits_the_height_axis() {
        let split = SplitPane::new().with_direction(SplitDirection::Vertical);
        let (top, bottom) = split.split(Rect::new(0, 0, 20, 7));
        // Odd height: first (top) absorbs the spare.
        assert_eq!(top.height, 4);
        assert_eq!(bottom.height, 3);
        // Cross axis (width) passes through fully.
        assert_eq!(top.width, 20);
        assert_eq!(bottom.width, 20);
        assert_eq!(top.y, 0);
        assert_eq!(bottom.y, 4);
    }

    #[test]
    fn square_area_resolves_auto_to_horizontal() {
        // Auto on a square area ⇒ Horizontal (the `>=` tie-break).
        let (a, b) = SplitPane::new().split(Rect::new(0, 0, 8, 8));
        assert_eq!(a.height, 8, "square area splits along width");
        assert_eq!(a.width, 4);
        assert_eq!(b.width, 4);
    }

    #[test]
    fn degenerate_zero_area_yields_two_zero_rects() {
        let (a, b) = SplitPane::new().split(Rect::new(0, 0, 0, 0));
        assert_eq!((a.width, a.height), (0, 0));
        assert_eq!((b.width, b.height), (0, 0));
    }

    #[test]
    fn degenerate_fixed_ge_available_collapses_flex_to_zero() {
        let split = SplitPane::new().with_ratio(SplitRatio::FirstFixed(50));
        let (first, second) = split.split(Rect::new(0, 0, 10, 4));
        assert_eq!(first.width, 10, "fixed clamped to available");
        assert_eq!(second.width, 0, "flex collapses to zero");
        assert_eq!(first.height, 4);
    }

    #[test]
    fn degenerate_gap_ge_axis_length_collapses_both_panes() {
        let split = SplitPane::new().with_ratio(SplitRatio::Percent(50)).with_gap(100);
        let (a, b) = split.split(Rect::new(0, 0, 10, 4));
        assert_eq!(a.width, 0);
        assert_eq!(b.width, 0);
    }

    #[test]
    fn degenerate_one_cell_axis_never_panics() {
        // 1×N and tiny areas must stay within `area` and never overflow.
        let (a, b) = SplitPane::new().split(Rect::new(0, 0, 1, 5));
        assert!(a.x + a.width <= 1);
        assert!(b.x + b.width <= 1);
    }

    #[test]
    fn gap_lands_between_panes_and_is_absorbed_by_flex() {
        // FirstFixed(24) + gap 1 on 100 cells ⇒ fixed 24, gap 1, flex 75.
        let split = SplitPane::new()
            .with_direction(SplitDirection::Horizontal)
            .with_ratio(SplitRatio::FirstFixed(24))
            .with_gap(1);
        let (first, second) = split.split(Rect::new(0, 0, 100, 10));
        assert_eq!(first.width, 24, "fixed pane keeps its exact n");
        assert_eq!(first.x, 0);
        assert_eq!(second.x, 25, "second starts after fixed + gap");
        assert_eq!(second.width, 75, "flexible pane absorbs the gap");
        assert_eq!(first.width + 1 + second.width, 100);
    }

    #[test]
    fn spare_cell_survives_an_odd_gap() {
        // Odd gap (1) + odd remaining axis under Percent(50):
        // available = 9 - 1 = 8 (even) here, but use axis 11 ⇒ available
        // 10 (even). Force an odd available: axis 10, gap 1 ⇒ available
        // 9 (odd) ⇒ first absorbs the spare (5), second 4, plus the gap.
        let split = SplitPane::new()
            .with_direction(SplitDirection::Horizontal)
            .with_ratio(SplitRatio::Percent(50))
            .with_gap(1);
        let (first, second) = split.split(Rect::new(0, 0, 10, 4));
        assert_eq!(first.width, 5, "first absorbs the spare cell");
        assert_eq!(second.width, 4);
        assert_eq!(first.width + 1 + second.width, 10);
    }

    #[test]
    fn explicit_horizontal_splits_the_width_axis() {
        let split = SplitPane::new().with_direction(SplitDirection::Horizontal);
        let (left, right) = split.split(Rect::new(0, 0, 20, 7));
        assert_eq!(left.width, 10);
        assert_eq!(right.width, 10);
        // Cross axis (height) passes through fully.
        assert_eq!(left.height, 7);
        assert_eq!(right.height, 7);
        assert_eq!(left.x, 0);
        assert_eq!(right.x, 10);
    }

    // --- §7.2 Auto resolution tests (plan task 2.3) ----------------------

    #[test]
    fn auto_wide_area_resolves_to_horizontal() {
        // width(80) > height(24) ⇒ Horizontal (side-by-side).
        let (first, second) = SplitPane::new().split(Rect::new(0, 0, 80, 24));
        // Horizontal ⇒ panes differ on x, share full height.
        assert_eq!(first.height, 24);
        assert_eq!(second.height, 24);
        assert!(second.x > first.x);
        assert_eq!(first.y, second.y);
    }

    #[test]
    fn auto_tall_area_resolves_to_vertical() {
        // width(20) < height(50) ⇒ Vertical (stacked).
        let (first, second) = SplitPane::new().split(Rect::new(0, 0, 20, 50));
        assert_eq!(first.width, 20);
        assert_eq!(second.width, 20);
        assert!(second.y > first.y);
        assert_eq!(first.x, second.x);
    }

    // --- §7.2 fixed-ratio tests (plan task 2.4) --------------------------

    #[test]
    fn first_fixed_honors_exact_length_and_flexes_other() {
        let split = SplitPane::new()
            .with_direction(SplitDirection::Horizontal)
            .with_ratio(SplitRatio::FirstFixed(20));
        let (first, second) = split.split(Rect::new(0, 0, 100, 10));
        assert_eq!(first.width, 20, "fixed pane gets its exact n");
        assert_eq!(second.width, 80, "other pane flexes to the remainder");
    }

    #[test]
    fn second_fixed_honors_exact_length_and_flexes_other() {
        let split = SplitPane::new()
            .with_direction(SplitDirection::Horizontal)
            .with_ratio(SplitRatio::SecondFixed(20));
        let (first, second) = split.split(Rect::new(0, 0, 100, 10));
        assert_eq!(first.width, 80, "other pane flexes to the remainder");
        assert_eq!(second.width, 20, "fixed pane gets its exact n");
    }

    #[test]
    fn fixed_zero_clamps_to_one_on_construction() {
        assert_eq!(SplitRatio::first_fixed(0), SplitRatio::FirstFixed(1));
        assert_eq!(SplitRatio::second_fixed(0), SplitRatio::SecondFixed(1));
    }

    #[test]
    fn with_ratio_renormalizes_raw_zero_literals() {
        // Raw enum literals that would starve a pane are re-normalized by
        // `with_ratio`, so callers cannot bypass the invariant by skipping
        // the constructors.
        let percent = SplitPane::new().with_ratio(SplitRatio::Percent(0));
        assert_eq!(percent.ratio, SplitRatio::Percent(1));

        let first = SplitPane::new().with_ratio(SplitRatio::FirstFixed(0));
        assert_eq!(first.ratio, SplitRatio::FirstFixed(1));

        let second = SplitPane::new().with_ratio(SplitRatio::SecondFixed(0));
        assert_eq!(second.ratio, SplitRatio::SecondFixed(1));
    }

    #[test]
    fn split_renormalizes_a_raw_zero_ratio_field() {
        // `split()` defends itself: a direct struct field assignment (which
        // skips the builder) is still re-normalized before geometry, so the
        // no-zero-pane invariant cannot be bypassed.
        let mut split = SplitPane::new();
        split.ratio = SplitRatio::Percent(0);
        let (first, second) = split.split(Rect::new(0, 0, 100, 10));
        assert!(first.width >= 1, "Percent(0) normalized; first not starved");
        assert!(second.width >= 1, "second not starved");
    }

    // --- §7.2 gap tests (plan task 2.6) ----------------------------------

    #[test]
    fn gap_reduces_total_before_percent_division() {
        // Percent(50) + gap 2 on 80 cells ⇒ available 78 ⇒ 39 / 39 plus
        // the 2-cell channel ⇒ 39 + 2 + 39 == 80.
        let split = SplitPane::new()
            .with_direction(SplitDirection::Horizontal)
            .with_ratio(SplitRatio::Percent(50))
            .with_gap(2);
        let (first, second) = split.split(Rect::new(0, 0, 80, 10));
        assert_eq!(first.width, 39);
        assert_eq!(second.width, 39);
        assert_eq!(second.x, first.x + first.width + 2);
        assert_eq!(first.width + 2 + second.width, 80);
    }

    // --- §7.2 Percent boundary clamping (plan task 2.7) ------------------

    #[test]
    fn percent_zero_clamps_to_one() {
        assert_eq!(SplitRatio::percent(0), SplitRatio::Percent(1));
    }

    #[test]
    fn percent_one_hundred_clamps_to_ninety_nine() {
        assert_eq!(SplitRatio::percent(100), SplitRatio::Percent(99));
    }

    // --- §7.2 acceptance-invariant sweep (plan task 2.8) -----------------

    #[test]
    fn acceptance_invariant_holds_across_representative_spread() {
        // Representative spread per §7.2: tiny, wide, tall, square, odd.
        let areas = [
            Rect::new(0, 0, 0, 0),
            Rect::new(0, 0, 1, 1),
            Rect::new(0, 0, 1, 5),
            Rect::new(0, 0, 5, 1),
            Rect::new(0, 0, 2, 2),
            Rect::new(0, 0, 3, 3),
            Rect::new(0, 0, 8, 8),
            Rect::new(0, 0, 9, 9),
            Rect::new(0, 0, 80, 24),
            Rect::new(0, 0, 81, 24),
            Rect::new(0, 0, 20, 50),
            Rect::new(0, 0, 100, 10),
            Rect::new(0, 0, 10, 100),
        ];
        let directions = [
            SplitDirection::Auto,
            SplitDirection::Horizontal,
            SplitDirection::Vertical,
        ];
        let ratios = [
            SplitRatio::Percent(50),
            SplitRatio::Percent(30),
            SplitRatio::Percent(70),
            SplitRatio::FirstFixed(5),
            SplitRatio::SecondFixed(5),
        ];
        // gap ∈ {0, 1, large}: `large` exercises the clamp-to-axis path.
        let gaps = [0u16, 1u16, 1000u16];

        for area in areas {
            for &direction in &directions {
                for &ratio in &ratios {
                    for &gap in &gaps {
                        let split = SplitPane::new()
                            .with_direction(direction)
                            .with_ratio(ratio)
                            .with_gap(gap);
                        assert_acceptance_invariant(&split, area);
                    }
                }
            }
        }
    }
}
