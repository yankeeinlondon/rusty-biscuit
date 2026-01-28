use crate::{components::renderable::Renderable, utils::color::Color};

pub enum FontWeight {
    Bold,
    Normal,
    Dim,
}

/// Provides uniform styling support to a block of text
/// by wrapping the passed in block with
#[allow(dead_code)]
pub struct TextBlock {
    content: String,
    font_weight: FontWeight,
    fg_color: Color,
    bg_color: Color,
    italic: bool,
    word_wrap: bool,
    margin_right: u32,
    margin_left: u32,
}

impl Renderable for TextBlock {
    fn render() -> String {
        todo!()
    }

    fn fallback_render(_term: &crate::terminal::Terminal) -> String {
        todo!()
    }
}

impl Default for TextBlock {
    fn default() -> TextBlock {
        TextBlock {
            content: "".to_string(),
            font_weight: FontWeight::Normal,
            fg_color: Color::DefaultForeground,
            bg_color: Color::DefaultBackground,
            italic: false,
            word_wrap: true,
            margin_right: 0,
            margin_left: 0,
        }
    }
}

impl TextBlock {
    pub fn new<T: Into<String>>(content: T) -> Self {
        TextBlock {
            content: content.into(),
            ..Default::default()
        }
    }
}
