pub enum FontWeight {
    Bold,
    Normal,
    Dim
}

/// provides styling support to a block of text
pub struct TextBlock<T: Into<String>> {
    content: T,
    font_weight: FontWeight,
    fg_color: Color,
    bg_color: Color,
    italic: bool,
    word_wrap: bool,
    margin_right: Option<u32>,
    margin_left: Option<u32>
}

impl Renderable for TextBlock {
    fn render() -> String {
        todo!()
    }
}

