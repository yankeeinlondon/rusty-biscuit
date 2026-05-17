//! Target-agnostic layout configuration data.
//!
//! This module owns the layout *data* shared across render targets. Terminal
//! ANSI-width application (`apply_layout` / `apply_block_layout`) lives in
//! `biscuit-terminal` as the `LayoutTerminalExt` extension trait.

mod length;
pub use length::{Length, LayoutError};

mod target_value;
pub use target_value::TargetValue;

use crate::color::Color;

pub use crate::wrap_policy::WordWrap;

/// Specifies the horizontal alignment of content within a text block.
///
/// This enum controls how text is positioned horizontally when there's extra
/// horizontal space available (e.g., when the content is shorter than the
/// container width or when margins create unused space).
///
/// ## Per-line vs. block alignment
///
/// `Alignment` only describes *direction* (left/center/right). How it is
/// applied to multi-line content depends on which terminal layout method is
/// used:
///
/// - per-line alignment aligns each line independently — appropriate for
///   prose where each wrapped line is logically standalone.
/// - block alignment aligns all lines as a cohesive block using a single
///   offset derived from the widest line — appropriate for content with
///   horizontal structure that must be preserved (tree connectors, borders,
///   column edges, bullet/number columns).
///
/// ## Examples
///
/// ```
/// use renderable::layout::{Alignment, Layout};
///
/// let layout = Layout {
///     alignment: Alignment::Center,
///     ..Layout::default()
/// };
/// assert_eq!(layout.alignment, Alignment::Center);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

/// Specifies a margin value that can be either fixed or percentage-based.
///
/// Margins control the whitespace around renderable content. They are resolved
/// against the terminal width when rendering occurs.
///
/// ## Variants
///
/// - **`None`**: No margin (zero whitespace).
///
/// - **`Chars(u32)`**: Fixed margin in terminal character cells. The margin
///   is exactly this many characters regardless of terminal width.
///
/// - **`Percent(f32)`**: Percentage of the terminal width. The margin is
///   calculated as `(terminal_width * percent / 100.0)`. Values typically
///   range from 0.0 to 100.0.
///
/// - **`Offset(Box<Margin>, u32)`**: Lazy composition of a base margin plus
///   an additional character offset. Used when nesting components to accumulate
///   margin without resolving percentages prematurely. The `add_chars` method
///   constructs this automatically.
///
/// ## Examples
///
/// ```
/// use renderable::layout::Margin;
///
/// // No margin
/// let none = Margin::None;
///
/// // Fixed 4 character margin
/// let chars = Margin::Chars(4);
///
/// // 10% of terminal width
/// let percent = Margin::Percent(10.0);
///
/// // Combining: 10% + 4 characters
/// let offset = Margin::Percent(10.0).add_chars(4);
///
/// // Chaining: 2 + 3 = 5 characters
/// let chained = Margin::Chars(2).add_chars(3);
/// ```
///
/// ## Resolution
///
/// Margins are resolved to character counts at render time:
///
/// ```
/// use renderable::layout::{Layout, Margin};
///
/// assert_eq!(Layout::resolve_margin(&Margin::Chars(4), 80), 4);
/// assert_eq!(Layout::resolve_margin(&Margin::Percent(10.0), 100), 10);
/// assert_eq!(Layout::resolve_margin(&Margin::None, 80), 0);
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Margin {
    #[default]
    None,
    Chars(u32),
    Percent(f32),
    /// Lazy composition of a base margin plus additional character offset.
    ///
    /// Used when nesting components to accumulate margin without
    /// resolving percentages prematurely.
    Offset(Box<Margin>, u32),
}

impl Margin {
    /// Compose this margin with additional character offset.
    ///
    /// Optimizes common cases:
    /// - `None` + chars → `Chars(chars)`
    /// - `Chars(a)` + chars → `Chars(a + chars)`
    /// - other + chars → `Offset(base, chars)` (defers resolution)
    pub fn add_chars(self, chars: u32) -> Margin {
        if chars == 0 {
            return self;
        }
        match self {
            Margin::None => Margin::Chars(chars),
            Margin::Chars(existing) => Margin::Chars(existing + chars),
            other => Margin::Offset(Box::new(other), chars),
        }
    }
}

/// Row padding strategy for text block rendering.
///
/// Determines whether rows in a text block should be padded to match the
/// maximum width of the renderable area. This is particularly useful when
/// rendering styled content with background colors to ensure proper visual
/// alignment.
///
/// ## Variants
///
/// - **`Auto`** (default): Pad rows only when background color is not the default.
///   Rows are extended to the max width of the text block if a custom background
///   color is set; otherwise, no padding is added.
///
/// - **`Fill`**: Always pad each line to precisely the max width of the block's
///   constraint, regardless of background color.
///
/// - **`Exact`**: Do not add any padding. Rows maintain their natural width,
///   matching only the content length.
///
/// ## Examples
///
/// ```rust
/// use renderable::layout::RowFill;
///
/// // Auto-fill only when using custom background colors
/// let fill_auto = RowFill::Auto;
///
/// // Always fill to max width
/// let fill_always = RowFill::Fill;
///
/// // Never add padding
/// let no_fill = RowFill::Exact;
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub enum RowFill {
    /// if the background color _is **not**_ the default background color
    ///
    /// > then each row's width will be extended to the max width for the
    /// > text block. Otherwise, no additional padding is provided.
    #[default]
    Auto,
    /// pad each line to be precisely the length of the max width of the
    /// block's constraint
    Fill,
    /// do not add any padding to force the width to match the max width
    /// of the text constraint
    Exact,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MaxWidth {
    None,
    Chars(u32),
    Percent(f32),
}

/// Layout configuration for renderable components.
///
/// Controls margins, alignment, word-wrapping, and background color for
/// rendered content. Every terminal component has an associated `Layout`
/// that determines how it appears.
///
/// ## Fields
///
/// - **`left_margin`**, **`right_margin`**, **`top_margin`**, **`bottom_margin`**:
///   Control whitespace around the content. See [`Margin`] for specification options.
///
/// - **`alignment`**: Horizontal alignment within the available width
///   ([`Alignment::Left`], [`Alignment::Center`], or [`Alignment::Right`]).
///
/// - **`row_fill_strategy`**: Whether to pad rows to fill the available width
///   (useful for background colors). See [`RowFill`].
///
/// - **`word_wrap`**: How to handle lines that exceed the available width.
///   See [`WordWrap`] for options like wrap-with-hyphenation, truncate, or none.
///
/// - **`page_bg_color`**: Optional background color for the entire content area.
///
/// ## Examples
///
/// ```
/// use renderable::layout::{Layout, Margin, Alignment, WordWrap};
///
/// // Centered content with margins
/// let layout = Layout {
///     left_margin: Margin::Chars(4),
///     right_margin: Margin::Chars(4),
///     alignment: Alignment::Center,
///     ..Default::default()
/// };
///
/// // Word-wrapped content at 50% width
/// let wrapped = Layout {
///     left_margin: Margin::Percent(25.0),
///     right_margin: Margin::Percent(25.0),
///     word_wrap: WordWrap::WrapProse(Some(8), Some(4)),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// how much whitespace is required to the _left_ of this text block
    pub left_margin: Margin,
    /// how much whitespace is required to the _right_ of this text block
    pub right_margin: Margin,
    /// how many blank lines should precede the first content?
    ///
    /// Most components will ignore these instructions and leave this to
    /// the `Compose` component which has a more macro page-level responsibility
    pub top_margin: Margin,
    /// how many blank lines should follow the final content?
    ///
    /// Most components will ignore these instructions and leave this to
    /// the `Compose` component which has a more macro page-level responsibility
    pub bottom_margin: Margin,

    /// how should the text block be aligned relative to the renderable window
    pub alignment: Alignment,
    /// determines whether rows of text should be padded with characters to
    /// ensure
    pub row_fill_strategy: RowFill,
    /// how should we handle a line of text which
    /// extends beyond the length of the available
    /// space
    pub word_wrap: WordWrap,

    /// If the overall layout should use a backing color then
    /// this can be added here. Underlying components are free to
    /// change this for their rendering.
    ///
    /// This setting changes the "default background color" while also ensuring
    /// that a background is used (versus allowing the desktop ... which might
    /// have an image as a background). In most cases you should leave this as
    /// `None` but when you do set it you likely will want to set the `row_fill_strategy`
    /// to "fill".
    ///
    /// > Note: setting this property to `Some<Color::DefaultBackgroundColor>` WILL make a change
    /// > because instead of rendering text on top of a transparent background, you are now explicitly
    /// > rendering it onto an opaque background color and masking anything the terminal may have been
    /// > rendering underneath it.
    pub page_bg_color: Option<Color>,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            left_margin: Margin::default(),
            right_margin: Margin::default(),
            top_margin: Margin::default(),
            bottom_margin: Margin::default(),
            alignment: Alignment::default(),
            row_fill_strategy: RowFill::default(),
            word_wrap: WordWrap::None,
            page_bg_color: None,
        }
    }
}

impl Layout {
    /// Add a new Layout by setting the `word_wrap` policy and optionally
    /// setting the margins (left, right, top, bottom).
    pub fn new(wrap: WordWrap, margin: Option<(Margin, Margin, Margin, Margin)>) -> Self {
        match margin {
            Some(margin) => Layout {
                word_wrap: wrap,
                left_margin: margin.0,
                right_margin: margin.1,
                top_margin: margin.2,
                bottom_margin: margin.3,
                ..Layout::default()
            },
            _ => Layout {
                word_wrap: wrap,
                ..Layout::default()
            },
        }
    }

    /// Resolve a margin to a number of characters given a terminal width.
    pub fn resolve_margin(margin: &Margin, terminal_width: u32) -> u32 {
        match margin {
            Margin::None => 0,
            Margin::Chars(chars) => *chars,
            Margin::Percent(pct) => ((terminal_width as f32) * pct / 100.0).round() as u32,
            Margin::Offset(base, chars) => Self::resolve_margin(base, terminal_width) + chars,
        }
    }

    /// Calculate the available content width after accounting for margins.
    pub fn available_width(&self, terminal_width: u32) -> u32 {
        let left = Self::resolve_margin(&self.left_margin, terminal_width);
        let right = Self::resolve_margin(&self.right_margin, terminal_width);
        terminal_width.saturating_sub(left).saturating_sub(right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_default() {
        let layout = Layout::default();
        assert_eq!(layout.alignment, Alignment::Left);
        assert_eq!(layout.word_wrap, WordWrap::None);
    }

    #[test]
    fn test_margin_resolve_chars() {
        let margin = Margin::Chars(10);
        assert_eq!(Layout::resolve_margin(&margin, 100), 10);
    }

    #[test]
    fn test_margin_resolve_percent() {
        let margin = Margin::Percent(10.0);
        assert_eq!(Layout::resolve_margin(&margin, 100), 10);
    }

    #[test]
    fn test_margin_resolve_none() {
        let margin = Margin::None;
        assert_eq!(Layout::resolve_margin(&margin, 100), 0);
    }

    #[test]
    fn test_margin_resolve_offset() {
        let margin = Margin::Percent(10.0).add_chars(4);
        assert_eq!(Layout::resolve_margin(&margin, 100), 14);
    }

    #[test]
    fn test_margin_resolve_nested_offset() {
        // Percent(10%) + 4 + 2 → at width 100: 10 + 4 + 2 = 16
        let margin = Margin::Percent(10.0).add_chars(4).add_chars(2);
        assert_eq!(Layout::resolve_margin(&margin, 100), 16);
    }

    #[test]
    fn test_margin_add_chars_none_becomes_chars() {
        assert_eq!(Margin::None.add_chars(4), Margin::Chars(4));
    }

    #[test]
    fn test_margin_add_chars_chars_combines() {
        assert_eq!(Margin::Chars(2).add_chars(4), Margin::Chars(6));
    }

    #[test]
    fn test_margin_add_chars_zero_is_noop() {
        assert_eq!(Margin::Chars(5).add_chars(0), Margin::Chars(5));
        assert_eq!(Margin::None.add_chars(0), Margin::None);
        assert_eq!(Margin::Percent(10.0).add_chars(0), Margin::Percent(10.0));
    }

    #[test]
    fn test_margin_add_chars_percent_becomes_offset() {
        let result = Margin::Percent(10.0).add_chars(4);
        assert_eq!(result, Margin::Offset(Box::new(Margin::Percent(10.0)), 4));
    }

    #[test]
    fn test_available_width_with_char_margins() {
        let layout = Layout {
            left_margin: Margin::Chars(10),
            right_margin: Margin::Chars(10),
            ..Layout::default()
        };
        assert_eq!(layout.available_width(80), 60);
    }

    #[test]
    fn test_available_width_with_percent_margins() {
        let layout = Layout {
            left_margin: Margin::Percent(10.0),
            right_margin: Margin::Percent(10.0),
            ..Layout::default()
        };
        assert_eq!(layout.available_width(100), 80);
    }

    #[test]
    fn test_available_width_saturates_to_zero() {
        let layout = Layout {
            left_margin: Margin::Chars(50),
            right_margin: Margin::Chars(50),
            ..Layout::default()
        };
        assert_eq!(layout.available_width(80), 0);
    }

    #[test]
    fn test_layout_implements_debug() {
        let layout = Layout::default();
        let debug_str = format!("{:?}", layout);
        assert!(debug_str.contains("Layout"));
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_available_width_is_bounded(
            terminal_width in 1..=1000u32,
            margin_left in 0..=100u32,
            margin_right in 0..=100u32
        ) {
            let layout = Layout {
                left_margin: Margin::Chars(margin_left),
                right_margin: Margin::Chars(margin_right),
                ..Default::default()
            };
            let result = layout.available_width(terminal_width);
            prop_assert!(result <= terminal_width.saturating_sub(margin_left).saturating_sub(margin_right));
        }
    }
}
