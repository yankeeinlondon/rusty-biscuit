use crate::components::renderable::Renderable;

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
pub struct Prose {
    /// the raw content as received
    content: String,
    /// the content after having been parsed for template
    parsed_content: Option<String>,

    /// Whether the **word wrap** feature is turned on.
    /// When on, an attempt to create clean line breaks
    /// at natural word breakpoints will be made.
    word_wrap: bool,
    /// Optionally force a fixed number of blank characters at the
    /// start of each line to create a "left margin"
    margin_left: Option<u32>,
    /// Optionally force a fixed number of blank characters at the
    /// end of each line to create a "right margin" effect
    margin_right: Option<u32>,
}

impl Default for Prose {
    fn default() -> Prose {
        Prose {
            content: "".to_string(),
            parsed_content: None,
            word_wrap: true,
            margin_left: None,
            margin_right: None,
        }
    }
}

impl Renderable for Prose {
    fn render() -> String {
        todo!()
    }

    fn fallback_render(term: &crate::terminal::Terminal) -> String {
        todo!()
    }
}
