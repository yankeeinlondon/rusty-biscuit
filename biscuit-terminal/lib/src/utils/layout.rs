use crate::{
    terminal::Terminal,
    utils::{
        block_constraint::{split_lines, visible_width, wrap_lines},
        color::Color,
    },
};

/// A `RenderableWrapper` is a utility which operates at a
/// lower level than a `Renderable` **component** and takes in
/// string content and outputs that string _wrapped_ in some
/// sort of formatting.
pub trait RenderableWrapper {
    fn render<T: Into<String>>(&self, content: T) -> String;

    fn fallback_render<T: Into<String>>(&self, content: T, term: &Terminal) -> String;
}

/// Specifies the horizontal alignment of content within a text block.
///
/// This enum controls how text is positioned horizontally when there's extra
/// horizontal space available (e.g., when the content is shorter than the
/// container width or when margins create unused space).
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::utils::layout::{Alignment, Layout, WordWrap};
///
/// // Left-aligned text (default)
/// let layout = Layout {
///     alignment: Alignment::Left,
///     ..Layout::default()
/// };
/// let result = layout.apply_layout("Hello", 80);
/// assert!(result.starts_with("Hello"));
///
/// // Centered text - content is centered within the available width
/// let layout = Layout {
///     alignment: Alignment::Center,
///     ..Layout::default()
/// };
/// let result = layout.apply_layout("Hi", 80);
/// // With 80 width and 2 char content, 78 chars of padding are added
/// // 39 chars before, 39 chars after
/// assert!(result.starts_with("                                       Hi"));
///
/// // Right-aligned text - content pushed to the right edge
/// let layout = Layout {
///     alignment: Alignment::Right,
///     ..Layout::default()
/// };
/// let result = layout.apply_layout("End", 80);
/// assert!(result.ends_with("End"));
/// ```
///
/// ## Alignment with Margins
///
/// When combined with margins, alignment applies to the content area
/// between the margins:
///
/// ```
/// use biscuit_terminal::utils::layout::{Alignment, Layout, Margin};
///
/// let layout = Layout {
///     left_margin: Margin::Chars(10),
///     right_margin: Margin::Chars(10),
///     alignment: Alignment::Center,
///     ..Layout::default()
/// };
/// // Available width is 60 (80 - 10 - 10), "Hi" is centered in that space
/// let result = layout.apply_layout("Hi", 80);
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
/// use biscuit_terminal::utils::layout::Margin;
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
/// use biscuit_terminal::utils::layout::{Layout, Margin};
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
/// use biscuit_terminal::utils::layout::RowFill;
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
///
/// ```rust
/// use biscuit_terminal::components::prose::Prose;
/// use biscuit_terminal::components::renderable::Renderable;
/// use biscuit_terminal::utils::layout::{Layout, RowFill};
///
/// // Create prose with auto row fill (default behavior)
/// let prose = Prose::new("Hello, world!");
/// let layout = prose.layout().clone();
/// assert_eq!(layout.row_fill_strategy, RowFill::Auto);
///
/// // Explicitly set to always fill
/// let prose = Prose::new("Styled content")
///     .with_layout(Layout {
///         row_fill_strategy: RowFill::Fill,
///         ..Layout::default()
///     });
/// ```
///
/// ```rust
/// use biscuit_terminal::utils::layout::{Layout, RowFill, Margin};
///
/// // Use RowFill::Exact with margins for precise control
/// let layout = Layout {
///     row_fill_strategy: RowFill::Exact,
///     left_margin: Margin::Chars(5),
///     right_margin: Margin::Chars(5),
///     ..Layout::default()
/// };
/// // Each line will have 5 char margins but no extra padding
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

#[derive(Debug, Clone, PartialEq)]
pub enum WordWrap {
    /// **WrapProse**`(StartLooking, HangingIndent)`
    ///
    /// Will attempt to wrap words on wrap characters (e.g., whitespace,
    /// `-`, etc.) but if unable to find a break character in the text
    /// body then the text will be hyphenated at a hard break point.
    ///
    /// When the word wrapping logic is engaged _start_ looking for a good
    /// place to break the line a certain number of characters before
    /// max-width is reached.
    ///
    /// By default (e.g., when wrap gets `None`) we start looking for a line break
    /// 8 characters before the reaching the end of the line but you can
    /// override that with whatever you want.
    ///
    /// When a value is provided in the second parameter we use that to
    /// indent lines after the first by that many spaces.
    WrapProse(Option<u32>, Option<u32>),

    /// **BespokeProse**`(StartLooking, BreakChars, HangingIndent)`
    ///
    /// If you want to explicitly state the valid "break characters" which
    /// you hope to break on you can use this over the `WrapProse` option.
    ///
    /// When a value is provided in the third parameter we use that to
    /// indent lines after the first by that many spaces.
    BespokeProse(Option<u32>, Vec<char>, Option<u32>),

    /// Instead of "wrapping", we will truncate any content that moves
    /// beyond the end of the line. You can specify a string -- often an
    /// ellipsis -- which should be added to the end to give a visual queue
    /// that content has been truncated.
    ///
    /// The last "real" character on this line will be a character which
    /// allows the addition of this optional string to be presented to
    /// the terminal without extending beyond the renderable window.
    Truncate(Option<String>),
    /// no word wrap, when the end of line (e.g., max-width) is reached
    /// a new line is started but without any `-` or other markings to
    /// indicate a "continuation" and no attempt is made to break at a
    /// clean break character.
    ///
    /// This is also a hint to components like Table to try other columns
    /// before those marked as NOT having word wrap.
    None,
}

impl Default for WordWrap {
    fn default() -> Self {
        WordWrap::WrapProse(Some(8), None)
    }
}

/// Layout configuration for renderable components.
///
/// Controls margins, alignment, word-wrapping, and background color for
/// terminal-rendered content. Every component that implements [`Renderable`]
/// has an associated `Layout` that determines how it appears in the terminal.
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
/// use biscuit_terminal::utils::layout::{Layout, Margin, Alignment, WordWrap};
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
///
/// ## Usage with Components
///
/// Most components provide builder methods to configure layout:
///
/// ```
/// use biscuit_terminal::prelude::*;
/// use biscuit_terminal::utils::layout::{Margin, Alignment, Layout};
/// use biscuit_terminal::components::section::{Section, HeadingLevel};
///
/// let section = Section::new(HeadingLevel::h2, "Title")
///     .left_margin(Margin::Chars(2))
///     .alignment(Alignment::Center);
/// ```
#[derive(Debug, Clone)]
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

    /// Apply the layout to content and render.
    pub fn apply_layout(&self, content: &str, terminal_width: u32) -> String {
        let left = Self::resolve_margin(&self.left_margin, terminal_width);
        let right = Self::resolve_margin(&self.right_margin, terminal_width);

        // Calculate available width for content
        let available_width = terminal_width.saturating_sub(left).saturating_sub(right);
        if available_width == 0 {
            return String::new();
        }

        // Remember if content ended with a newline so we can preserve it.
        // Without this, split_lines produces an empty trailing element that
        // gets padded with left_margin spaces — leaving invisible trailing
        // whitespace that triggers zsh's missing-newline indicator (%).
        let had_trailing_newline = content.ends_with('\n');

        // Split content into lines
        let mut lines = split_lines(content);

        // Remove the empty trailing element produced by a trailing newline;
        // we'll restore the newline at the end instead.
        if had_trailing_newline
            && let Some(last) = lines.last()
            && last.is_empty()
        {
            lines.pop();
        }

        // Apply word wrapping
        let wrapped_lines = match &self.word_wrap {
            WordWrap::None => lines,
            _ => wrap_lines(lines, &self.word_wrap, available_width),
        };

        // Build result with margins and alignment
        let left_padding = " ".repeat(left as usize);
        let mut result = String::new();

        // Determine if we should fill rows
        let should_fill = match &self.row_fill_strategy {
            RowFill::Fill => true,
            RowFill::Auto => self.page_bg_color.is_some(),
            RowFill::Exact => false,
        };

        for line in wrapped_lines {
            let line_width = visible_width(&line);
            let padding_needed = available_width.saturating_sub(line_width);

            // Apply alignment
            let (pre_align, post_align) = match self.alignment {
                Alignment::Left => (0, padding_needed),
                Alignment::Right => (padding_needed, 0),
                Alignment::Center => {
                    let left_pad = padding_needed / 2;
                    let right_pad = padding_needed - left_pad;
                    (left_pad, right_pad)
                }
            };

            result.push_str(&left_padding);
            result.push_str(&" ".repeat(pre_align as usize));
            result.push_str(&line);

            if should_fill {
                result.push_str(&" ".repeat(post_align as usize));
            }

            result.push('\n');
        }

        // Remove trailing newline (unless content originally ended with one,
        // which was stripped above and should be preserved)
        if !had_trailing_newline && result.ends_with('\n') {
            result.pop();
        }

        result
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
    fn test_layout_left_margin() {
        let layout = Layout {
            left_margin: Margin::Chars(4),
            ..Layout::default()
        };
        let result = layout.apply_layout("Hello", 80);
        assert!(result.starts_with("    Hello"));
    }

    #[test]
    fn test_layout_center_alignment() {
        let layout = Layout {
            alignment: Alignment::Center,
            ..Layout::default()
        };
        let result = layout.apply_layout("Hi", 80);
        // With 80 width and 2 char content, should have padding
        assert!(result.contains("Hi"));
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
    fn test_layout_trailing_newline_not_padded() {
        // Content ending with \n (e.g., image escape sequences with scroll
        // compensation) should preserve the trailing newline without adding
        // left-margin padding to the empty trailing line.
        let layout = Layout {
            left_margin: Margin::Chars(4),
            ..Layout::default()
        };
        let result = layout.apply_layout("hello\n", 80);
        // Should end with \n, not with trailing spaces
        assert!(
            result.ends_with('\n'),
            "Trailing newline should be preserved"
        );
        assert!(
            !result.ends_with("    \n"),
            "Empty trailing line should not be padded"
        );
        assert_eq!(result, "    hello\n");
    }

    #[test]
    fn test_layout_no_trailing_newline_unchanged() {
        // Content without trailing newline should not gain one
        let layout = Layout {
            left_margin: Margin::Chars(4),
            ..Layout::default()
        };
        let result = layout.apply_layout("hello", 80);
        assert!(!result.ends_with('\n'));
        assert_eq!(result, "    hello");
    }

    #[test]
    fn test_layout_implements_debug() {
        let layout = Layout::default();
        let debug_str = format!("{:?}", layout);
        assert!(debug_str.contains("Layout"));
    }

    #[test]
    fn test_word_wrap_applied() {
        let layout = Layout {
            word_wrap: WordWrap::WrapProse(None, None),
            ..Layout::default()
        };
        // With default 80 width, a long line should wrap
        let long_text = "a".repeat(100);
        let result = layout.apply_layout(&long_text, 80);
        // Should have been split into multiple lines
        assert!(result.contains('\n') || result.len() <= 80);
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_layout_never_panics(
            text in ".*",
            terminal_width in 1..=500u32,
            margin_left in 0..=100u32,
            margin_right in 0..=100u32,
            indentation in 0..=100u32
        ) {
            let layout = Layout {
                left_margin: Margin::Chars(margin_left),
                right_margin: Margin::Chars(margin_right),
                word_wrap: WordWrap::WrapProse(Some(indentation), None),
                ..Default::default()
            };
            let _ = layout.apply_layout(&text, terminal_width);
            let _ = layout.available_width(terminal_width);
        }

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
