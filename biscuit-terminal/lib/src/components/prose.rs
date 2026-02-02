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
/// - `{{reset}}`, `{{reset-fg}}`, `{{reset-bg}}`
/// - `{{normal-font-weight}}`, `{{not-italic}}`, `{{not-underline}}`, `{{not-strikethrough}}`
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

impl Prose {
    /// Create a new Prose instance with the given content.
    pub fn new<T: Into<String>>(content: T) -> Self {
        Prose {
            content: content.into(),
            word_wrap: None,
            left_margin: None,
            right_margin: None,
        }
    }

    /// Set the word wrap strategy.
    pub fn with_word_wrap(mut self, wrap: WordWrap) -> Self {
        self.word_wrap = Some(wrap);
        self
    }

    /// Set the left margin.
    pub fn with_left_margin(mut self, margin: Margin) -> Self {
        self.left_margin = Some(margin);
        self
    }

    /// Set the right margin.
    pub fn with_right_margin(mut self, margin: Margin) -> Self {
        self.right_margin = Some(margin);
        self
    }

    /// Parse and render the content, replacing tokens with ANSI escape codes.
    fn parse_tokens(&self, _term: Option<&Terminal>) -> String {
        let mut result = String::new();
        let mut used_styles = false;
        let mut chars = self.content.chars().peekable();

        while let Some(ch) = chars.next() {
            // Check for atomic tokens: {{token}}
            if ch == '{' && chars.peek() == Some(&'{') {
                chars.next(); // consume second '{'
                let mut token = String::new();
                let mut found_close = false;

                while let Some(c) = chars.next() {
                    if c == '}' && chars.peek() == Some(&'}') {
                        chars.next(); // consume second '}'
                        found_close = true;
                        break;
                    }
                    token.push(c);
                }

                if found_close {
                    if let Some(escape) = atomic_token_to_escape(&token) {
                        result.push_str(escape);
                        used_styles = true;
                    } else {
                        // Unknown token, output as-is
                        result.push_str("{{");
                        result.push_str(&token);
                        result.push_str("}}");
                    }
                } else {
                    // Unclosed token, output as-is
                    result.push_str("{{");
                    result.push_str(&token);
                }
                continue;
            }

            // Check for block tokens: <tag>content</tag>
            if ch == '<' {
                let mut tag_content = String::new();
                let mut found_close = false;

                // Collect until '>'
                while let Some(c) = chars.next() {
                    if c == '>' {
                        found_close = true;
                        break;
                    }
                    tag_content.push(c);
                }

                if found_close && !tag_content.starts_with('/') {
                    // Parse the opening tag
                    if let Some((tag_name, attrs)) = parse_opening_tag(&tag_content) {
                        // Collect content until closing tag
                        let closing_tag = format!("</{}>", tag_name);
                        let mut inner_content = String::new();
                        let mut depth = 1;

                        while depth > 0 {
                            if let Some(c) = chars.next() {
                                inner_content.push(c);
                                // Check for nested opening tags
                                if inner_content.ends_with(&format!("<{}>", tag_name))
                                    || inner_content.ends_with(&format!("<{} ", tag_name)) {
                                    depth += 1;
                                }
                                // Check for closing tag
                                if inner_content.ends_with(&closing_tag) {
                                    depth -= 1;
                                    if depth == 0 {
                                        // Remove the closing tag from content
                                        inner_content.truncate(inner_content.len() - closing_tag.len());
                                    }
                                }
                            } else {
                                break;
                            }
                        }

                        // Recursively parse inner content
                        let inner_prose = Prose::new(inner_content);
                        let parsed_inner = inner_prose.parse_tokens(_term);

                        // Apply the block style
                        if let Some((open, close)) = block_tag_to_escape(&tag_name, &attrs, _term) {
                            result.push_str(&open);
                            result.push_str(&parsed_inner);
                            result.push_str(&close);
                            used_styles = true;
                        } else {
                            // Unknown tag, output as-is
                            result.push('<');
                            result.push_str(&tag_content);
                            result.push('>');
                            result.push_str(&parsed_inner);
                            result.push_str(&closing_tag);
                        }
                        continue;
                    }
                }

                // Not a valid block tag, output as-is
                result.push('<');
                result.push_str(&tag_content);
                if found_close {
                    result.push('>');
                }
                continue;
            }

            result.push(ch);
        }

        // Add reset at the end if styles were used
        if used_styles {
            result.push_str("\x1b[0m");
        }

        result
    }
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

        self.parse_tokens(None)
    }

    fn fallback_render(&self, term: &Terminal, layout: Option<&Layout>) -> String {
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

        self.parse_tokens(Some(term))
    }
}

/// Convert an atomic token name to its ANSI escape code.
fn atomic_token_to_escape(token: &str) -> Option<&'static str> {
    match token.to_lowercase().as_str() {
        // Text styles
        "bold" => Some("\x1b[1m"),
        "dim" => Some("\x1b[2m"),
        "italic" => Some("\x1b[3m"),
        "underline" => Some("\x1b[4m"),
        "blink" => Some("\x1b[5m"),
        "reverse" => Some("\x1b[7m"),
        "hidden" => Some("\x1b[8m"),
        "strikethrough" => Some("\x1b[9m"),

        // Reset codes
        "reset" => Some("\x1b[0m"),
        "reset-fg" => Some("\x1b[39m"),
        "reset-bg" => Some("\x1b[49m"),

        // Style-specific reset tokens (kebab-case standard)
        "normal-font-weight" => Some("\x1b[22m"), // Resets bold and dim
        "not-italic" => Some("\x1b[23m"),
        "not-underline" => Some("\x1b[24m"),
        "not-blink" => Some("\x1b[25m"),
        "not-inverse" => Some("\x1b[27m"),
        "not-hidden" => Some("\x1b[28m"),
        "not-strikethrough" => Some("\x1b[29m"),

        // Basic foreground colors
        "black" => Some("\x1b[30m"),
        "red" => Some("\x1b[31m"),
        "green" => Some("\x1b[32m"),
        "yellow" => Some("\x1b[33m"),
        "blue" => Some("\x1b[34m"),
        "magenta" => Some("\x1b[35m"),
        "cyan" => Some("\x1b[36m"),
        "white" => Some("\x1b[37m"),

        // Bright foreground colors
        "bright-black" => Some("\x1b[90m"),
        "bright-red" => Some("\x1b[91m"),
        "bright-green" => Some("\x1b[92m"),
        "bright-yellow" => Some("\x1b[93m"),
        "bright-blue" => Some("\x1b[94m"),
        "bright-magenta" => Some("\x1b[95m"),
        "bright-cyan" => Some("\x1b[96m"),
        "bright-white" => Some("\x1b[97m"),

        // Basic background colors
        "bg-black" => Some("\x1b[40m"),
        "bg-red" => Some("\x1b[41m"),
        "bg-green" => Some("\x1b[42m"),
        "bg-yellow" => Some("\x1b[43m"),
        "bg-blue" => Some("\x1b[44m"),
        "bg-magenta" => Some("\x1b[45m"),
        "bg-cyan" => Some("\x1b[46m"),
        "bg-white" => Some("\x1b[47m"),

        // Bright background colors
        "bg-bright-black" => Some("\x1b[100m"),
        "bg-bright-red" => Some("\x1b[101m"),
        "bg-bright-green" => Some("\x1b[102m"),
        "bg-bright-yellow" => Some("\x1b[103m"),
        "bg-bright-blue" => Some("\x1b[104m"),
        "bg-bright-magenta" => Some("\x1b[105m"),
        "bg-bright-cyan" => Some("\x1b[106m"),
        "bg-bright-white" => Some("\x1b[107m"),

        _ => None,
    }
}

/// Parse an opening tag into its name and attributes.
fn parse_opening_tag(tag_content: &str) -> Option<(String, Vec<(String, String)>)> {
    let tag_content = tag_content.trim();
    if tag_content.is_empty() {
        return None;
    }

    let parts: Vec<&str> = tag_content.splitn(2, |c: char| c.is_whitespace()).collect();
    let tag_name = parts[0].to_lowercase();

    let mut attrs = Vec::new();
    if parts.len() > 1 {
        // Parse attributes
        let attr_str = parts[1];
        let mut current_attr = String::new();
        let mut current_value = String::new();
        let mut in_value = false;
        let mut quote_char: Option<char> = None;

        for c in attr_str.chars() {
            if in_value {
                if let Some(qc) = quote_char {
                    if c == qc {
                        attrs.push((current_attr.clone(), current_value.clone()));
                        current_attr.clear();
                        current_value.clear();
                        in_value = false;
                        quote_char = None;
                    } else {
                        current_value.push(c);
                    }
                } else if c == '"' || c == '\'' {
                    quote_char = Some(c);
                } else if c.is_whitespace() {
                    if !current_value.is_empty() {
                        attrs.push((current_attr.clone(), current_value.clone()));
                        current_attr.clear();
                        current_value.clear();
                    }
                    in_value = false;
                } else {
                    current_value.push(c);
                }
            } else if c == '=' {
                in_value = true;
            } else if !c.is_whitespace() {
                current_attr.push(c);
            }
        }

        // Handle last attribute without closing quote
        if !current_attr.is_empty() || !current_value.is_empty() {
            attrs.push((current_attr, current_value));
        }
    }

    Some((tag_name, attrs))
}

/// Convert a block tag to its opening and closing ANSI escape codes.
fn block_tag_to_escape(
    tag_name: &str,
    attrs: &[(String, String)],
    term: Option<&Terminal>,
) -> Option<(String, String)> {
    match tag_name {
        "b" => Some(("\x1b[1m".to_string(), "\x1b[22m".to_string())),
        "i" => Some(("\x1b[3m".to_string(), "\x1b[23m".to_string())),
        "u" => Some(("\x1b[4m".to_string(), "\x1b[24m".to_string())),
        "uu" => Some(("\x1b[21m".to_string(), "\x1b[24m".to_string())), // Double underline
        "~" => Some(("\x1b[9m".to_string(), "\x1b[29m".to_string())),   // Strikethrough
        "dim" => Some(("\x1b[2m".to_string(), "\x1b[22m".to_string())),
        "blink" => Some(("\x1b[5m".to_string(), "\x1b[25m".to_string())),
        "reverse" => Some(("\x1b[7m".to_string(), "\x1b[27m".to_string())),

        // OSC8 hyperlinks
        "a" => {
            let href = attrs.iter()
                .find(|(k, _)| k == "href")
                .map(|(_, v)| v.as_str())
                .unwrap_or("");

            // Check if terminal supports OSC8
            let supports_osc8 = term.map(|t| t.osc_link_support).unwrap_or(true);
            if supports_osc8 && !href.is_empty() {
                Some((
                    format!("\x1b]8;;{}\x1b\\", href),
                    "\x1b]8;;\x1b\\".to_string(),
                ))
            } else {
                // Fallback: just show the content (no link)
                Some((String::new(), String::new()))
            }
        }

        // RGB colors
        "rgb" => {
            // Parse RGB from tag name like "rgb 125,67,45"
            let rgb_str = attrs.iter()
                .find(|(k, _)| k.is_empty())
                .map(|(_, v)| v.as_str())
                .unwrap_or("");

            if let Some((r, g, b)) = parse_rgb(rgb_str) {
                Some((
                    format!("\x1b[38;2;{};{};{}m", r, g, b),
                    "\x1b[39m".to_string(),
                ))
            } else {
                None
            }
        }

        // Named colors
        "black" => Some(("\x1b[30m".to_string(), "\x1b[39m".to_string())),
        "red" => Some(("\x1b[31m".to_string(), "\x1b[39m".to_string())),
        "green" => Some(("\x1b[32m".to_string(), "\x1b[39m".to_string())),
        "yellow" => Some(("\x1b[33m".to_string(), "\x1b[39m".to_string())),
        "blue" => Some(("\x1b[34m".to_string(), "\x1b[39m".to_string())),
        "magenta" => Some(("\x1b[35m".to_string(), "\x1b[39m".to_string())),
        "cyan" => Some(("\x1b[36m".to_string(), "\x1b[39m".to_string())),
        "white" => Some(("\x1b[37m".to_string(), "\x1b[39m".to_string())),

        // Clipboard - returns empty escapes, actual clipboard handling would be done externally
        "clipboard" => Some((String::new(), String::new())),

        _ => None,
    }
}

/// Parse an RGB string like "125,67,45" into (r, g, b).
fn parse_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() == 3 {
        let r = parts[0].trim().parse::<u8>().ok()?;
        let g = parts[1].trim().parse::<u8>().ok()?;
        let b = parts[2].trim().parse::<u8>().ok()?;
        Some((r, g, b))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_bold_token() {
        let prose = Prose::new("Hello {{bold}}world{{reset}}!");
        let result = prose.render(None);
        assert_eq!(result, "Hello \x1b[1mworld\x1b[0m!\x1b[0m");
    }

    #[test]
    fn test_atomic_color_token() {
        let prose = Prose::new("{{red}}Error{{reset}}");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[31mError\x1b[0m\x1b[0m");
    }

    #[test]
    fn test_block_bold_tag() {
        let prose = Prose::new("<b>bold text</b>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[1mbold text\x1b[22m\x1b[0m");
    }

    #[test]
    fn test_block_italic_tag() {
        let prose = Prose::new("<i>italic text</i>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[3mitalic text\x1b[23m\x1b[0m");
    }

    #[test]
    fn test_block_underline_tag() {
        let prose = Prose::new("<u>underlined</u>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[4munderlined\x1b[24m\x1b[0m");
    }

    #[test]
    fn test_nested_block_tags() {
        let prose = Prose::new("<b><i>bold italic</i></b>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[1m\x1b[3mbold italic\x1b[23m\x1b[0m\x1b[22m\x1b[0m");
    }

    #[test]
    fn test_osc8_link() {
        let prose = Prose::new("<a href=\"https://example.com\">link</a>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\\x1b[0m");
    }

    #[test]
    fn test_plain_text_no_reset() {
        let prose = Prose::new("Plain text with no styles");
        let result = prose.render(None);
        assert_eq!(result, "Plain text with no styles");
    }

    #[test]
    fn test_background_color() {
        let prose = Prose::new("{{bg-red}}highlight{{reset}}");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[41mhighlight\x1b[0m\x1b[0m");
    }

    #[test]
    fn test_strikethrough_block() {
        let prose = Prose::new("<~>deleted</~>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[9mdeleted\x1b[29m\x1b[0m");
    }

    #[test]
    fn test_named_color_block() {
        let prose = Prose::new("<red>error message</red>");
        let result = prose.render(None);
        assert_eq!(result, "\x1b[31merror message\x1b[39m\x1b[0m");
    }
}
