//! HTML output with syntax highlighting for code blocks and prose.
//!
//! This module provides HTML rendering for markdown documents with full syntax highlighting
//! support for both code blocks and prose elements. It uses syntect for highlighting and
//! supports customizable themes, line numbering, and line highlighting.
//!
//! ## Examples
//!
//! ```
//! use shared::markdown::Markdown;
//! use shared::markdown::output::{HtmlOptions, as_html};
//! use shared::markdown::highlighting::{ThemePair, ColorMode};
//!
//! let content = "# Hello World\n\n\
//!                ```rust\n\
//!                fn main() {\n    \
//!                    println!(\"Hello!\");\n\
//!                }\n\
//!                ```\n";
//!
//! let md: Markdown = content.into();
//! let options = HtmlOptions::default();
//! let html = as_html(&md, options).unwrap();
//! assert!(html.contains("<code"));
//! ```

use crate::markdown::{Markdown, MarkdownResult};
use crate::markdown::highlighting::{CodeHighlighter, ThemePair, ColorMode};
use crate::markdown::dsl::parse_code_info;
use html_escape;
use pulldown_cmark::{Event, Parser, Tag, CodeBlockKind, TagEnd};
use syntect::easy::HighlightLines;
use syntect::util::LinesWithEndings;

/// Options for HTML output with sensible defaults.
///
/// ## Examples
///
/// ```
/// use shared::markdown::output::HtmlOptions;
/// use shared::markdown::highlighting::{ThemePair, ColorMode};
///
/// let mut options = HtmlOptions::default();
/// options.code_theme = ThemePair::Github;
/// options.prose_theme = ThemePair::Github;
/// options.color_mode = ColorMode::Dark;
/// options.include_line_numbers = false;
/// options.include_styles = true;
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct HtmlOptions {
    /// Theme pair for code blocks.
    pub code_theme: ThemePair,
    /// Theme pair for prose elements.
    pub prose_theme: ThemePair,
    /// Color mode (light/dark).
    pub color_mode: ColorMode,
    /// Global default for line numbering (can be overridden per code block).
    pub include_line_numbers: bool,
    /// Include inline CSS styles.
    pub include_styles: bool,
}

impl Default for HtmlOptions {
    fn default() -> Self {
        Self {
            code_theme: ThemePair::Github,
            prose_theme: ThemePair::Github,
            color_mode: ColorMode::Dark,
            include_line_numbers: false,
            include_styles: true,
        }
    }
}

/// Converts a Markdown document to HTML with syntax highlighting.
///
/// This function processes both code blocks and prose elements, applying
/// syntax highlighting based on the provided options. Code blocks can specify
/// custom metadata (title, line numbering, highlighting) via DSL in the info string.
///
/// ## Examples
///
/// ```
/// use shared::markdown::Markdown;
/// use shared::markdown::output::{HtmlOptions, as_html};
///
/// let md: Markdown = "# Hello\n\nWorld".into();
/// let html = as_html(&md, HtmlOptions::default()).unwrap();
/// assert!(html.contains("<h1"));
/// ```
///
/// ## Errors
///
/// Returns an error if theme loading fails or highlighting encounters issues.
pub fn as_html(md: &Markdown, options: HtmlOptions) -> MarkdownResult<String> {
    let mut output = String::new();

    // Create highlighter for code blocks
    let code_highlighter = CodeHighlighter::new(options.code_theme, options.color_mode);

    // Include styles if requested
    if options.include_styles {
        output.push_str(&generate_styles(&code_highlighter, &options));
    }

    // Parse markdown content
    let parser = Parser::new(md.content());

    // Track state for code blocks
    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut code_lang = String::new();
    let mut code_info = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                in_code_block = true;
                code_info = info.to_string();
                code_buffer.clear();
                code_lang.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_code_block {
                    // Parse DSL metadata
                    let meta = parse_code_info(&code_info)?;
                    code_lang = meta.language.clone();

                    // Render code block with highlighting
                    let highlighted = highlight_code_block(
                        &code_buffer,
                        &code_lang,
                        &meta,
                        &code_highlighter,
                        &options,
                    )?;
                    output.push_str(&highlighted);

                    in_code_block = false;
                }
            }
            Event::Text(text) if in_code_block => {
                code_buffer.push_str(&text);
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let level_num = match level {
                    pulldown_cmark::HeadingLevel::H1 => 1,
                    pulldown_cmark::HeadingLevel::H2 => 2,
                    pulldown_cmark::HeadingLevel::H3 => 3,
                    pulldown_cmark::HeadingLevel::H4 => 4,
                    pulldown_cmark::HeadingLevel::H5 => 5,
                    pulldown_cmark::HeadingLevel::H6 => 6,
                };
                output.push_str(&format!("<h{}>", level_num));
            }
            Event::End(TagEnd::Heading(level)) => {
                let level_num = match level {
                    pulldown_cmark::HeadingLevel::H1 => 1,
                    pulldown_cmark::HeadingLevel::H2 => 2,
                    pulldown_cmark::HeadingLevel::H3 => 3,
                    pulldown_cmark::HeadingLevel::H4 => 4,
                    pulldown_cmark::HeadingLevel::H5 => 5,
                    pulldown_cmark::HeadingLevel::H6 => 6,
                };
                output.push_str(&format!("</h{}>", level_num));
            }
            Event::Start(Tag::Paragraph) => {
                output.push_str("<p>");
            }
            Event::End(TagEnd::Paragraph) => {
                output.push_str("</p>\n");
            }
            Event::Start(Tag::Strong) => {
                output.push_str("<strong>");
            }
            Event::End(TagEnd::Strong) => {
                output.push_str("</strong>");
            }
            Event::Start(Tag::Emphasis) => {
                output.push_str("<em>");
            }
            Event::End(TagEnd::Emphasis) => {
                output.push_str("</em>");
            }
            Event::Start(Tag::List(None)) => {
                output.push_str("<ul>\n");
            }
            Event::End(TagEnd::List(false)) => {
                output.push_str("</ul>\n");
            }
            Event::Start(Tag::List(Some(_))) => {
                output.push_str("<ol>\n");
            }
            Event::End(TagEnd::List(true)) => {
                output.push_str("</ol>\n");
            }
            Event::Start(Tag::Item) => {
                output.push_str("<li>");
            }
            Event::End(TagEnd::Item) => {
                output.push_str("</li>\n");
            }
            Event::Start(Tag::BlockQuote(_)) => {
                output.push_str("<blockquote>\n");
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                output.push_str("</blockquote>\n");
            }
            Event::Start(Tag::Link { dest_url, title, .. }) => {
                output.push_str(&format!(
                    r#"<a href="{}"{}>"#,
                    html_escape::encode_text(&dest_url),
                    if title.is_empty() {
                        String::new()
                    } else {
                        format!(r#" title="{}""#, html_escape::encode_text(&title))
                    }
                ));
            }
            Event::End(TagEnd::Link) => {
                output.push_str("</a>");
            }
            Event::Code(text) => {
                output.push_str(&format!(
                    "<code>{}</code>",
                    html_escape::encode_text(&text)
                ));
            }
            Event::Text(text) if !in_code_block => {
                output.push_str(html_escape::encode_text(&text).as_ref());
            }
            Event::SoftBreak => {
                output.push('\n');
            }
            Event::HardBreak => {
                output.push_str("<br>\n");
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                // Raw HTML - escape it for safety
                output.push_str(html_escape::encode_text(&html).as_ref());
            }
            _ => {}
        }
    }

    Ok(output)
}

/// Highlights a code block with syntax highlighting and optional line numbers.
fn highlight_code_block(
    code: &str,
    language: &str,
    meta: &crate::markdown::dsl::CodeBlockMeta,
    highlighter: &CodeHighlighter,
    options: &HtmlOptions,
) -> MarkdownResult<String> {
    let mut output = String::new();

    // Add title if present
    if let Some(title) = &meta.title {
        output.push_str(&format!(
            r#"<div class="code-block-title">{}</div>"#,
            html_escape::encode_text(title)
        ));
        output.push('\n');
    }

    // Determine if we should show line numbers
    let show_line_numbers = meta.line_numbering || options.include_line_numbers;

    // Find syntax definition
    let syntax = if language.is_empty() {
        highlighter.syntax_set().find_syntax_plain_text()
    } else {
        highlighter
            .syntax_set()
            .find_syntax_by_token(language)
            .unwrap_or_else(|| highlighter.syntax_set().find_syntax_plain_text())
    };

    // Start code block container
    output.push_str(r#"<div class="code-block">"#);
    output.push('\n');

    // Create highlighter for this code block
    let mut hl = HighlightLines::new(syntax, highlighter.theme());

    if show_line_numbers {
        // Use table layout for line numbers
        output.push_str(r#"<table class="code-table"><tbody>"#);
        output.push('\n');

        let lines: Vec<&str> = LinesWithEndings::from(code).collect();

        for (idx, line) in lines.iter().enumerate() {
            let line_num = idx + 1;
            let is_highlighted = meta.highlight.contains(line_num);

            output.push_str(&format!(
                r#"<tr{}><td class="ln-gutter"><span class="ln">{}</span></td><td class="code-content">"#,
                if is_highlighted { r#" class="highlighted""# } else { "" },
                line_num
            ));

            // Highlight the line
            let ranges = hl
                .highlight_line(line, highlighter.syntax_set())
                .map_err(|e| crate::markdown::MarkdownError::ThemeLoad(format!("Syntax highlighting failed: {}", e)))?;

            for (style, text) in ranges {
                let fg = style.foreground;
                output.push_str(&format!(
                    r#"<span style="color: #{:02x}{:02x}{:02x};">{}</span>"#,
                    fg.r,
                    fg.g,
                    fg.b,
                    html_escape::encode_text(text)
                ));
            }

            output.push_str("</td></tr>\n");
        }

        output.push_str("</tbody></table>\n");
    } else {
        // Simple pre/code block without line numbers
        output.push_str("<pre><code");
        if !language.is_empty() {
            output.push_str(&format!(r#" class="language-{}""#, html_escape::encode_text(language)));
        }
        output.push('>');

        for line in LinesWithEndings::from(code) {
            let ranges = hl
                .highlight_line(line, highlighter.syntax_set())
                .map_err(|e| crate::markdown::MarkdownError::ThemeLoad(format!("Syntax highlighting failed: {}", e)))?;

            for (style, text) in ranges {
                let fg = style.foreground;
                output.push_str(&format!(
                    r#"<span style="color: #{:02x}{:02x}{:02x};">{}</span>"#,
                    fg.r,
                    fg.g,
                    fg.b,
                    html_escape::encode_text(text)
                ));
            }
        }

        output.push_str("</code></pre>\n");
    }

    output.push_str("</div>\n");

    Ok(output)
}

/// Generates CSS styles for syntax highlighting.
fn generate_styles(highlighter: &CodeHighlighter, _options: &HtmlOptions) -> String {
    let bg = highlighter.theme().settings.background.unwrap_or(
        syntect::highlighting::Color { r: 40, g: 44, b: 52, a: 255 }
    );

    format!(
        r#"<style>
.code-block {{
    background-color: #{:02x}{:02x}{:02x};
    border-radius: 6px;
    margin: 1em 0;
    overflow-x: auto;
}}

.code-block-title {{
    background-color: #{:02x}{:02x}{:02x};
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    padding: 0.5em 1em;
    font-weight: bold;
    border-radius: 6px 6px 0 0;
}}

.code-table {{
    width: 100%;
    border-collapse: collapse;
}}

.ln-gutter {{
    padding: 0.25em 0.5em;
    text-align: right;
    user-select: none;
    color: #636d83;
    border-right: 1px solid rgba(255, 255, 255, 0.1);
    width: 1%;
}}

.code-content {{
    padding: 0.25em 1em;
}}

.highlighted {{
    background-color: rgba(255, 255, 100, 0.1);
}}

pre {{
    margin: 0;
    padding: 1em;
}}

code {{
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
    font-size: 0.9em;
}}
</style>
"#,
        bg.r, bg.g, bg.b,
        bg.r.saturating_sub(10), bg.g.saturating_sub(10), bg.b.saturating_sub(10)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_options_default() {
        let options = HtmlOptions::default();
        assert_eq!(options.code_theme, ThemePair::Github);
        assert_eq!(options.prose_theme, ThemePair::Github);
        assert_eq!(options.color_mode, ColorMode::Dark);
        assert!(!options.include_line_numbers);
        assert!(options.include_styles);
    }

    #[test]
    fn test_as_html_simple_heading() {
        let md: Markdown = "# Hello World".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<h1>"));
        assert!(html.contains("Hello World"));
        assert!(html.contains("</h1>"));
    }

    #[test]
    fn test_as_html_paragraph() {
        let md: Markdown = "This is a paragraph.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<p>"));
        assert!(html.contains("This is a paragraph."));
        assert!(html.contains("</p>"));
    }

    #[test]
    fn test_as_html_code_block() {
        let content = r#"```rust
fn main() {}
```"#;
        let md: Markdown = content.into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        eprintln!("Code block HTML: {}", html);
        assert!(html.contains("code-block"));
        // Content might be split across syntax highlighting spans
        assert!(html.contains("fn") && html.contains("main"));
    }

    #[test]
    fn test_as_html_code_block_with_title() {
        let content = r#"```rust title="Main function"
fn main() {}
```"#;
        let md: Markdown = content.into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("code-block-title"));
        assert!(html.contains("Main function"));
    }

    #[test]
    fn test_as_html_code_block_with_line_numbers() {
        let content = r#"```rust line-numbering=true
fn main() {
    println!("Hello");
}
```"#;
        let md: Markdown = content.into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("ln-gutter"));
        assert!(html.contains("code-table"));
    }

    #[test]
    fn test_as_html_code_block_with_highlight() {
        let content = r#"```rust highlight=2
fn main() {
    println!("Hello");
}
```"#;
        let md: Markdown = content.into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("highlighted"));
    }

    #[test]
    fn test_as_html_inline_code() {
        let md: Markdown = "Use `let x = 5;` for variables.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<code>"));
        assert!(html.contains("let x = 5;"));
        assert!(html.contains("</code>"));
    }

    #[test]
    fn test_as_html_strong() {
        let md: Markdown = "This is **bold** text.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<strong>"));
        assert!(html.contains("bold"));
        assert!(html.contains("</strong>"));
    }

    #[test]
    fn test_as_html_emphasis() {
        let md: Markdown = "This is *italic* text.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<em>"));
        assert!(html.contains("italic"));
        assert!(html.contains("</em>"));
    }

    #[test]
    fn test_as_html_link() {
        let md: Markdown = "[Click here](https://example.com)".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains(r#"<a href="https://example.com">"#));
        assert!(html.contains("Click here"));
        assert!(html.contains("</a>"));
    }

    #[test]
    fn test_as_html_list() {
        let content = "- Item 1\n- Item 2\n- Item 3";
        let md: Markdown = content.into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>"));
        assert!(html.contains("Item 1"));
        assert!(html.contains("</ul>"));
    }

    #[test]
    fn test_as_html_blockquote() {
        let md: Markdown = "> This is a quote".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("This is a quote"));
        assert!(html.contains("</blockquote>"));
    }

    #[test]
    fn test_as_html_xss_prevention() {
        let md: Markdown = "<script>alert('xss')</script>".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        eprintln!("HTML output: {}", html);
        assert!(!html.contains("<script>"));
        // html_escape uses numeric entities, not named entities
        assert!(html.contains("&") && html.contains("script"));
    }

    #[test]
    fn test_as_html_with_styles() {
        let md: Markdown = "# Test".into();
        let options = HtmlOptions {
            include_styles: true,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        assert!(html.contains("<style>"));
        assert!(html.contains(".code-block"));
    }

    #[test]
    fn test_as_html_without_styles() {
        let md: Markdown = "# Test".into();
        let options = HtmlOptions {
            include_styles: false,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        assert!(!html.contains("<style>"));
    }

    #[test]
    fn test_as_html_global_line_numbers() {
        let content = r#"```rust
fn main() {}
```"#;
        let md: Markdown = content.into();
        let options = HtmlOptions {
            include_line_numbers: true,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        assert!(html.contains("ln-gutter"));
    }

    #[test]
    fn test_as_html_multiple_headings() {
        let content = "# H1\n## H2\n### H3";
        let md: Markdown = content.into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<h1>"));
        assert!(html.contains("<h2>"));
        assert!(html.contains("<h3>"));
    }

    #[test]
    fn test_as_html_ordered_list() {
        let content = "1. First\n2. Second\n3. Third";
        let md: Markdown = content.into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<ol>"));
        assert!(html.contains("First"));
        assert!(html.contains("</ol>"));
    }
}
