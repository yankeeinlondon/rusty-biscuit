use crate::{components::renderable::Renderable, utils::color::Color};

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct BlockQuote {
    /// the content being wrapped in the block quote
    content: String,

    /// if the quote is being attributed to someone
    /// you can add that and it will be placed
    attribution: Option<String>,

    text_color: Option<Color>,
    bg_color: Option<Color>,
    left_block_color: Option<Color>,
}

impl Default for BlockQuote {
    fn default() -> Self {
        BlockQuote {
            content: "".to_string(),
            attribution: None,
            text_color: None,
            bg_color: None,
            left_block_color: None
        }
    }
}

impl BlockQuote {
    /// Create a block quote by passing in the content and _optionally_ an attribution.
    pub fn new<T: Into<String>, U: Into<String>>(content: T, attribution: Option<U>) -> Self {
        Self {
            content: content.into(),
            attribution: match attribution {
                Some(attribution) => Some(attribution.into()),
                _ => None
            },
            ..BlockQuote::default()
        }
    }
}

impl Renderable for BlockQuote {
    fn render(&self, _layout: Option<&crate::utils::layout::Layout>) -> String {
        todo!()
    }

    fn fallback_render(&self, _term: &crate::terminal::Terminal, _layout: Option<&crate::utils::layout::Layout>) -> String {
        todo!()
    }
}
