use crate::{
    components::renderable::Renderable,
    terminal::Terminal,
    utils::{
        color::Color,
        layout::Layout,
        styling::{FontWeight, Style, Stylist, UnderliningRequest},
    },
};

/// Provides uniform styling support to a block of text
#[derive(Debug)]
pub struct TextBlock {
    content: String,
    font_weight: FontWeight,
    /// optionally specify the foreground/text color
    fg_color: Option<Color>,
    /// optionally specify the background color
    bg_color: Option<Color>,
    italic: bool,
    strikethrough: bool,
    blink: bool,
    underline: UnderliningRequest,
    layout: Layout,
}

impl Default for TextBlock {
    fn default() -> TextBlock {
        TextBlock {
            content: "".to_string(),
            font_weight: FontWeight::Normal,
            fg_color: None,
            bg_color: None,
            italic: false,
            strikethrough: false,
            blink: false,
            underline: UnderliningRequest::None,
            layout: Layout::default(),
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

    /// Enable italics styling.
    pub fn using_italics(&mut self) -> &mut Self {
        self.italic = true;
        self
    }

    /// Enable bold text styling.
    pub fn using_bold_text(&mut self) -> &mut Self {
        self.font_weight = FontWeight::Bold;
        self
    }

    /// Enable strikethrough styling.
    pub fn use_strikethrough_on_content(&mut self) -> &mut Self {
        self.strikethrough = true;
        self
    }

    /// Enable dim text styling.
    pub fn using_dim_text(&mut self) -> &mut Self {
        self.font_weight = FontWeight::Dim;
        self
    }

    /// Set the foreground (text) color.
    pub fn with_foreground_color(&mut self, color: Color) -> &mut Self {
        self.fg_color = Some(color);
        self
    }

    /// Set the background color.
    pub fn with_background_color(&mut self, color: Color) -> &mut Self {
        self.bg_color = Some(color);
        self
    }

    /// Enable blinking text.
    pub fn make_content_blink(&mut self) -> &mut Self {
        self.blink = true;
        self
    }

    /// Set the underline style.
    pub fn with_underline(&mut self, under: UnderliningRequest) -> &mut Self {
        self.underline = under;
        self
    }

    /// Renders to a string with terminal escape sequences applied.
    fn to_terminal(&self, term: &Terminal) -> String {
        let mut content = self.content.clone();
        let _underline = term.underline_support;

        if self.italic && term.supports_italic {
            content = Style::Italic.wrap(content);
        }

        content = self.font_weight.term_wrap(content, term);

        content
    }
}

impl Renderable for TextBlock {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        let content = self.to_terminal(&term);
        self.layout.apply_layout(&content, width)
    }

    fn render(&self, term: &Terminal) -> String {
        let width = term.width();
        let content = self.to_terminal(term);
        self.layout.apply_layout(&content, width)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}
