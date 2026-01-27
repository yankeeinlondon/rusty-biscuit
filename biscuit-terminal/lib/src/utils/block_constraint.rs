use crate::{components::renderable::RenderableWrapper, utils::color::Color};

pub enum MaxWidth {
    None,
    Chars(u32),
    Percent(f32),
}

pub enum TextAlignment {
    Left,
    Center,
    Right,
}

/// Allows for fixed or percentage based margins to be added to the
/// block constraint.
pub enum Margin {
    None,
    Chars(u32),
    Percent(f32),
}

pub enum WordWrap {
    /// start looking for a break-character to start a new line on
    /// 6 characters before the max-width is reached. This eliminates
    /// almost all of the "hard breaks" which require a `-` character
    Normal,
    /// only start looking for break-characters when you reach 3 characters
    ///
    Aggressive,
    /// no word wrap, when the end of line (e.g., max-width) is reached
    /// a new line is started but without any `-` or other markings to
    /// indicate a "continuation"
    None,
}

/// The **WidthStrategy** determines if rows in the text block should
/// be padded to ensure that they are always the length of the `max_width`.
///
/// This can be useful when you set a background color to be something
/// other than the default color.
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

/// A **BlockConstraint** is used to define the layout constraints
/// for terminal output.
///
/// This can be used to constrain the output to the terminal page,
/// or a subset of the page (such as a "cell" in a table).
pub struct BlockConstraint {
    /// The maximum width allowed for the text in the block.
    ///
    /// - this is often set to the terminal's current width but
    /// it can be something less than this
    pub max_width: MaxWidth,

    /// the word wrap strategy for the block constraint
    pub word_wrap: WordWrap,

    pub alignment: TextAlignment,

    pub left_margin: Margin,
    pub right_margin: Margin,

    /// always ensure there is a blank line _before_ the block
    pub leading_blank_line: bool,
    /// always ensure there is a blank line _after_ the block
    pub trailing_blank_line: bool,

    pub text_color: Color,
    pub background_color: Color,
    /// the width strategy to use for the given block constraint
    pub row_fill_strategy: RowFill,
}

impl Default for BlockConstraint {
    fn default() -> BlockConstraint {
        BlockConstraint {
            max_width: MaxWidth::None,
            word_wrap: WordWrap::Normal,
            alignment: TextAlignment::Left,

            left_margin: Margin::None,
            right_margin: Margin::None,

            leading_blank_line: false,
            trailing_blank_line: false,

            text_color: Color::DefaultForeground,
            background_color: Color::DefaultBackground,
            row_fill_strategy: RowFill::Auto,
        }
    }
}

impl RenderableWrapper for BlockConstraint {
    fn render<T: Into<String>>(self, content: T) -> String {
        todo!()
    }

    fn fallback_render<T: Into<String>>(
        self,
        content: T,
        term: &crate::terminal::Terminal,
    ) -> String {
        todo!()
    }
}
