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

/// The **TextAlignment** enumeration allows for
/// terminal components to express how they should
/// _align_ to the terminal window once the layout
/// for the text block has been configured.
///
/// - by default `TextAlignment::Left` is chosen as
///   this is the most common expectation for callers
///   as well as the easiest to implement
/// - even though the `TextAlignment::Left` has less
///   dependencies to being rendered than the other alignments
///   it still needs to know that the `left_margin` is.
/// - both `TextAlignment::Right` and `TextAlignment::Center`
///   are only able to be expressed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum Alignment {
    Left,
    Center,
    Right,
}

impl Default for Alignment {
    fn default() -> Self {
        Alignment::Left
    }
}

/// The **Margin** allows for a fixed or percentage based margins to be
/// added to the renderable component.
#[derive(Debug, Clone, PartialEq)]
pub enum Margin {
    None,
    Chars(u32),
    Percent(f32),
    /// Lazy composition of a base margin plus additional character offset.
    ///
    /// Used when nesting components to accumulate margin without
    /// resolving percentages prematurely.
    Offset(Box<Margin>, u32),
}

impl Default for Margin {
    fn default() -> Self {
        Margin::None
    }
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

/// The **RowFill** determines if rows in the text block should
/// be padded to ensure that they are always the length of the renderable
/// window.
///
/// This can be useful when you set a background color to be something
/// other than the default color.
#[derive(Debug, Clone, PartialEq)]
pub enum RowFill {
    /// if the background color _is **not**_ the default background color
    /// then each row's width will be extended to the max width for the
    /// text block. Otherwise, no additional padding is provided.
    Auto,
    /// pad each line to be precisely the length of the max width of the
    /// block's constraint
    Fill,
    /// do not add any padding to force the width to match the max width
    /// of the text constraint
    Exact,
}

impl Default for RowFill {
    fn default() -> Self {
        RowFill::Auto
    }
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
    None,
}

impl Default for WordWrap {
    fn default() -> Self {
        WordWrap::WrapProse(Some(8), None)
    }
}

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
    /// because instead of rendering text on top of a transparent background, you are now explicitly
    /// rendering it onto an opaque background color and masking anything the terminal may have been
    /// rendering underneath it.
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
            Margin::Offset(base, chars) => {
                Self::resolve_margin(base, terminal_width) + chars
            }
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

        // Split content into lines
        let lines = split_lines(content);

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

        // Remove trailing newline
        if result.ends_with('\n') {
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
}
