use crate::{
    components::renderable::{Renderable, RenderableWrapper},
    terminal::Terminal,
    utils::{
        color::Color,
        layout::Layout,
        styling::{FontWeight, Style, Stylist, UnderliningRequest}
    }
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
            underline: UnderliningRequest::None
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

    pub fn using_italics(&mut self) -> &Self {
        self.italic = true;
        self
    }

    pub fn using_bold_text(&mut self) -> &Self {
        self.font_weight = FontWeight::Bold;
        self
    }

    pub fn use_strikethrough_on_content(&mut self) -> &Self {
        self.strikethrough = true;
        self
    }

    pub fn using_dim_text(&mut self) -> &Self {
        self.font_weight = FontWeight::Dim;
        self
    }

    pub fn with_foreground_color(&mut self, color: Color) -> &Self {
        self.fg_color = Some(color);
        self
    }

    pub fn with_background_color(&mut self, color: Color) -> &Self {
        self.fg_color = Some(color);
        self
    }

    pub fn make_content_blink(&mut self) -> &Self {
        self.blink = true;
        self
    }

    /// Renders to the terminal without
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
    fn render(&self, layout: Option<&Layout>) -> String {
        let term = Terminal::new_tty();
        match layout {
            Some(layout) => {
                layout.render(self.to_terminal(&term))
            },
            _ => {
                self.to_terminal(&term)
            }
        }
    }

    fn fallback_render(&self, term: &Terminal, layout: Option<&Layout>) -> String {
        match layout {
            Some(layout) => {
                layout.render(self.to_terminal(term))
            },
            _ => {
                self.to_terminal(term)
            }
        }
    }
}
