use crate::utils::color::Color;

#[allow(dead_code)]
pub struct BlockQuote {
    content: Vec<String>,

    text_color: Option<Color>,
    bg_color: Option<Color>,
    left_block_color: Option<Color>,
}
