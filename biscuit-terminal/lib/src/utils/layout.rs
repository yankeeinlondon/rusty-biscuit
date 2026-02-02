use crate::{components::renderable::{Renderable, RenderableWrapper}, utils::color::Color};

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
#[derive(Debug,Clone,PartialEq)]
pub enum Margin {
    None,
    Chars(u32),
    Percent(f32),
}

impl Default for Margin {
    fn default() -> Self {
        Margin::None
    }
}

/// The **RowFill** determines if rows in the text block should
/// be padded to ensure that they are always the length of the renderable
/// window.
///
/// This can be useful when you set a background color to be something
/// other than the default color.
#[derive(Debug,Clone,PartialEq)]
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

#[derive(Debug,Clone,PartialEq)]
pub enum MaxWidth {
    None,
    Chars(u32),
    Percent(f32),
}

#[derive(Debug,Clone,PartialEq)]
pub enum WordWrap {
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
    WrapProse(Option<u32>),

    /// If you want to explicitly state the valid "break characters" which
    /// you hope to break on you can use this over the `WrapProse` option.
    BespokeProse(Option<u32>, Vec<char>),

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

impl RenderableWrapper for Layout {
    fn render<T: Into<String>>(&self, content: T) -> String {
        todo!()
    }

    fn fallback_render<T: Into<String>>(&self, content: T, term: &crate::terminal::Terminal) -> String {
        todo!()
    }
}

impl Layout {

    /// Add a new Layout by setting the `word_wrap` policy and optionally
    /// setting the margins (left, right, top, bottom).
    pub fn new(wrap: WordWrap, margin: Option<(Margin,Margin,Margin,Margin)>) -> Self {
        match margin {
            Some(margin) => {
                Layout {
                    word_wrap: wrap,
                    left_margin: margin.0,
                    right_margin: margin.1,
                    top_margin: margin.2,
                    bottom_margin: margin.3,
                    ..Layout::default()
                }
            },
            _ => Layout {
                word_wrap: wrap,
                ..Layout::default()
            }
        }
    }




}
