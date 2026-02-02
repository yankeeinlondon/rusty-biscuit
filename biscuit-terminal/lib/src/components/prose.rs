use crate::{
    components::renderable::Renderable,
    terminal::Terminal,
    utils::layout::{Layout, Margin, WordWrap}
};

/// Prose content allows plain text to be passed in and that content will be parsed
/// for two kinds of tokens:
///
/// ## Atomic Tokens
///
/// Atomic tokens will be of the form `{{token}}` and the prose
/// parser does a simple lookup table on the atomic token and
/// replaces it with an escape code.
///
/// Examples include:
///
/// - `{{bold}}`, `{{dim}}`
/// - `{{italic}}`, `{{underline}}`, `{{strikethrough}}`
/// - `{{red}}`, `{{blue}}`, `{{bright-red}}`, etc.
/// - `{{bg-red}}`, `{{bg-blue}}`, etc.
/// - `{{reset}}`, `{{reset_fg}}`, `{{reset_bg}}`
///
/// The key characteristic of these atomic tokens is that they don't clean up
/// after themselves and require the caller to use the `{{reset}}` token whenever
/// they want to return to a known/default state.
///
/// **Note:** a `{{reset}}` is _always_ added to the end of a prose section which
/// has used at least one atomic token. This is just to be sure that styles do not
/// bleed out.
///
/// ## Block Tokens
///
/// Block tokens use an _HTML-like_ syntax but are really just a tiny subset of HTML's
/// vast catalog of tags. A block tag, in contrast to an atomic token, has a clear
/// start and stop token and like HTML we use the nomenclature of `<tag>content</tag>`.
///
/// Supported block tokens are:
///
/// - `<i>content</i>` for italic text
/// - `<b>content</b>` for bold text
/// - `<u>content</u>` for underlined text
/// - `<uu>content</uu>` for double-underlined text
/// - `<~>content</~>` for strikethrough content
/// - `<a href="...">content</a>` for an OSC8 link to a file or URL
/// - `<rgb 125,67,45>content</rgb>` for RGB colored foreground text
/// - `<red>content</red>` for named color foreground text
/// - `<clipboard>fallback</clipboard>` injects clipboard content or fallback
///
#[derive(Debug)]
pub struct Prose {
    /// the raw content as received
    content: String,

    word_wrap: Option<WordWrap>,
    /// Optionally force a fixed number of blank characters at the
    /// start of each line to create a "left margin"
    left_margin: Option<Margin>,
    /// Optionally force a fixed number of blank characters at the
    /// end of each line to create a "right margin" effect
    right_margin: Option<Margin>,
}

impl Default for Prose {
    fn default() -> Prose {
        Prose {
            content: "".to_string(),
            word_wrap: None,
            left_margin: None,
            right_margin: None,
        }
    }
}

impl Renderable for Prose {
    fn render(&self, layout: Option<&Layout>) -> String {
        let _layout = match layout {
            Some(layout) => {
              Layout {
                  word_wrap: match &self.word_wrap {
                      Some(wrap) => wrap.clone(),
                      _ => layout.word_wrap.clone()
                  },
                  left_margin: match &self.left_margin {
                      Some(margin) => margin.clone(),
                      _ => layout.left_margin.clone()
                  },
                  right_margin: match &self.right_margin {
                      Some(margin) => margin.clone(),
                      _ => layout.right_margin.clone()
                  },
                  top_margin: layout.top_margin.clone(),
                  bottom_margin: layout.bottom_margin.clone(),
                  alignment: layout.alignment,
                  row_fill_strategy: layout.row_fill_strategy.clone(),
                  page_bg_color: layout.page_bg_color.clone(),
              }
            },
            _ => {
                Layout {
                  word_wrap: self.word_wrap.clone().unwrap_or(WordWrap::None),
                  left_margin: self.left_margin.clone().unwrap_or_default(),
                  right_margin: self.right_margin.clone().unwrap_or_default(),
                  ..Layout::default()
                }
            }
        };
        // TODO: Implement actual rendering logic using the layout

    }

    fn fallback_render(&self, _term: &Terminal, _layout: Option<&Layout>) -> String {
        todo!()
    }
}
