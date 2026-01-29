//! HTML output with syntax highlighting for code blocks and prose.
//!
//! This module provides HTML rendering for markdown documents with full syntax highlighting
//! support for both code blocks and prose elements. It uses syntect for highlighting and
//! supports customizable themes, line numbering, and line highlighting.
//!
//! ## Examples
//!
//! ```
//! use darkmatter_lib::markdown::Markdown;
//! use darkmatter_lib::markdown::output::{HtmlOptions, as_html};
//! use darkmatter_lib::markdown::highlighting::{ThemePair, ColorMode};
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

use crate::markdown::dsl::parse_code_info;
use crate::markdown::highlighting::{CodeHighlighter, ColorMode, ThemePair};
use crate::markdown::inline::{InlineEvent, InlineTag, MarkProcessor};
use crate::markdown::output::terminal::MermaidMode;
use crate::markdown::{Markdown, MarkdownResult};
use crate::mermaid::Mermaid;
use crate::render::link::Link;
use html_escape;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::util::LinesWithEndings;

/// Options for HTML output with sensible defaults.
///
/// ## Examples
///
/// ```
/// use darkmatter_lib::markdown::output::HtmlOptions;
/// use darkmatter_lib::markdown::highlighting::{ThemePair, ColorMode};
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
    /// Controls how Mermaid diagrams are rendered.
    ///
    /// - `Off` (default): Show mermaid blocks as syntax-highlighted code
    /// - `Image`: Render as interactive mermaid diagrams (includes mermaid.js)
    /// - `Text`: Show as fenced code blocks (fallback format)
    pub mermaid_mode: MermaidMode,
}

impl Default for HtmlOptions {
    fn default() -> Self {
        Self {
            code_theme: ThemePair::Github,
            prose_theme: ThemePair::Github,
            color_mode: ColorMode::Dark,
            include_line_numbers: false,
            include_styles: true,
            mermaid_mode: MermaidMode::default(),
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
/// use darkmatter_lib::markdown::Markdown;
/// use darkmatter_lib::markdown::output::{HtmlOptions, as_html};
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

    // Parse markdown content with GFM strikethrough extension and wrap with MarkProcessor
    let parser = Parser::new_ext(md.content(), Options::ENABLE_STRIKETHROUGH);
    let events = MarkProcessor::new(parser);

    // Track state for code blocks
    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut code_lang = String::new();
    let mut code_info = String::new();
    let mut has_mermaid = false;

    for event in events {
        match event {
            // Handle custom inline tags (highlight/mark)
            InlineEvent::Start(InlineTag::Mark) => {
                output.push_str("<mark>");
            }
            InlineEvent::End(InlineTag::Mark) => {
                output.push_str("</mark>");
            }
            // Handle standard pulldown-cmark events
            InlineEvent::Standard(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))) => {
                in_code_block = true;
                code_info = info.to_string();
                code_buffer.clear();
                code_lang.clear();
            }
            InlineEvent::Standard(Event::End(TagEnd::CodeBlock)) => {
                if in_code_block {
                    // Parse DSL metadata
                    let meta = parse_code_info(&code_info)?;
                    code_lang = meta.language.clone();

                    // Check for mermaid code blocks
                    let is_mermaid = code_lang.eq_ignore_ascii_case("mermaid");

                    if is_mermaid && options.mermaid_mode != MermaidMode::Off {
                        match options.mermaid_mode {
                            MermaidMode::Image => {
                                // Render as interactive mermaid diagram
                                has_mermaid = true;
                                let diagram = Mermaid::new(&code_buffer);
                                if let Some(title) = &meta.title {
                                    let diagram = diagram.with_title(title.clone());
                                    let html = diagram.render_for_html();
                                    output.push_str(&html.body);
                                    output.push('\n');
                                } else {
                                    let html = diagram.render_for_html();
                                    output.push_str(&html.body);
                                    output.push('\n');
                                }
                            }
                            MermaidMode::Text => {
                                // Render as fenced code block (fallback format)
                                output.push_str("<pre><code class=\"language-mermaid\">");
                                output.push_str(&html_escape::encode_text(&code_buffer));
                                output.push_str("</code></pre>\n");
                            }
                            MermaidMode::Off => unreachable!(),
                        }
                    } else {
                        // Render code block with highlighting
                        let highlighted = highlight_code_block(
                            &code_buffer,
                            &code_lang,
                            &meta,
                            &code_highlighter,
                            &options,
                        )?;
                        output.push_str(&highlighted);
                    }

                    in_code_block = false;
                }
            }
            InlineEvent::Standard(Event::Text(text)) if in_code_block => {
                code_buffer.push_str(&text);
            }
            InlineEvent::Standard(Event::Start(Tag::Heading { level, .. })) => {
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
            InlineEvent::Standard(Event::End(TagEnd::Heading(level))) => {
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
            InlineEvent::Standard(Event::Start(Tag::Paragraph)) => {
                output.push_str("<p>");
            }
            InlineEvent::Standard(Event::End(TagEnd::Paragraph)) => {
                output.push_str("</p>\n");
            }
            InlineEvent::Standard(Event::Start(Tag::Strong)) => {
                output.push_str("<strong>");
            }
            InlineEvent::Standard(Event::End(TagEnd::Strong)) => {
                output.push_str("</strong>");
            }
            InlineEvent::Standard(Event::Start(Tag::Emphasis)) => {
                output.push_str("<em>");
            }
            InlineEvent::Standard(Event::End(TagEnd::Emphasis)) => {
                output.push_str("</em>");
            }
            InlineEvent::Standard(Event::Start(Tag::Strikethrough)) => {
                output.push_str("<del>");
            }
            InlineEvent::Standard(Event::End(TagEnd::Strikethrough)) => {
                output.push_str("</del>");
            }
            InlineEvent::Standard(Event::Start(Tag::List(None))) => {
                output.push_str("<ul>\n");
            }
            InlineEvent::Standard(Event::End(TagEnd::List(false))) => {
                output.push_str("</ul>\n");
            }
            InlineEvent::Standard(Event::Start(Tag::List(Some(_)))) => {
                output.push_str("<ol>\n");
            }
            InlineEvent::Standard(Event::End(TagEnd::List(true))) => {
                output.push_str("</ol>\n");
            }
            InlineEvent::Standard(Event::Start(Tag::Item)) => {
                output.push_str("<li>");
            }
            InlineEvent::Standard(Event::End(TagEnd::Item)) => {
                output.push_str("</li>\n");
            }
            InlineEvent::Standard(Event::Start(Tag::BlockQuote(_))) => {
                output.push_str("<blockquote>\n");
            }
            InlineEvent::Standard(Event::End(TagEnd::BlockQuote(_))) => {
                output.push_str("</blockquote>\n");
            }
            InlineEvent::Standard(Event::Start(Tag::Link {
                dest_url, title, ..
            })) => {
                // Parse title for structured content (class, style, prompt, etc.)
                // We use a placeholder display since we're streaming; actual text follows
                let link = Link::with_title_parsed("", &*dest_url, &title);

                // Build anchor tag with parsed attributes
                let mut attrs = format!(r#"href="{}""#, html_escape::encode_text(&dest_url));

                if let Some(class) = link.class() {
                    attrs.push_str(&format!(r#" class="{}""#, html_escape::encode_text(class)));
                }
                if let Some(style) = link.style() {
                    attrs.push_str(&format!(r#" style="{}""#, html_escape::encode_text(style)));
                }
                if let Some(target) = link.target() {
                    attrs.push_str(&format!(
                        r#" target="{}""#,
                        html_escape::encode_text(target)
                    ));
                }
                if let Some(title) = link.title() {
                    attrs.push_str(&format!(r#" title="{}""#, html_escape::encode_text(title)));
                }
                if let Some(prompt) = link.prompt() {
                    attrs.push_str(&format!(
                        r#" data-prompt="{}""#,
                        html_escape::encode_text(prompt)
                    ));
                }
                if let Some(data) = link.data() {
                    for (key, value) in data {
                        attrs.push_str(&format!(
                            r#" data-{}="{}""#,
                            html_escape::encode_text(key),
                            html_escape::encode_text(value)
                        ));
                    }
                }

                output.push_str(&format!("<a {}>", attrs));
            }
            InlineEvent::Standard(Event::End(TagEnd::Link)) => {
                output.push_str("</a>");
            }
            InlineEvent::Standard(Event::Code(text)) => {
                output.push_str(&format!("<code>{}</code>", html_escape::encode_text(&text)));
            }
            InlineEvent::Standard(Event::Text(text)) if !in_code_block => {
                output.push_str(html_escape::encode_text(&text).as_ref());
            }
            InlineEvent::Standard(Event::SoftBreak) => {
                output.push('\n');
            }
            InlineEvent::Standard(Event::HardBreak) => {
                output.push_str("<br>\n");
            }
            InlineEvent::Standard(Event::Html(html) | Event::InlineHtml(html)) => {
                // Raw HTML - escape it for safety
                output.push_str(html_escape::encode_text(&html).as_ref());
            }
            _ => {}
        }
    }

    // Add mermaid.js script if we rendered any mermaid diagrams
    if has_mermaid {
        output.push_str(r#"<script type="module">
  import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';
  mermaid.registerIconPacks([
    { name: 'fa7-brands', loader: () => fetch('https://unpkg.com/@iconify-json/fa7-brands@1/icons.json').then(r => r.json()) },
    { name: 'lucide', loader: () => fetch('https://unpkg.com/@iconify-json/lucide@1/icons.json').then(r => r.json()) },
    { name: 'carbon', loader: () => fetch('https://unpkg.com/@iconify-json/carbon@1/icons.json').then(r => r.json()) },
    { name: 'system-uicons', loader: () => fetch('https://unpkg.com/@iconify-json/system-uicons@1/icons.json').then(r => r.json()) }
  ]);
  mermaid.initialize({ startOnLoad: true });
</script>
"#);
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
                .map_err(|e| {
                    crate::markdown::MarkdownError::ThemeLoad(format!(
                        "Syntax highlighting failed: {}",
                        e
                    ))
                })?;

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
            output.push_str(&format!(
                r#" class="language-{}""#,
                html_escape::encode_text(language)
            ));
        }
        output.push('>');

        for line in LinesWithEndings::from(code) {
            let ranges = hl
                .highlight_line(line, highlighter.syntax_set())
                .map_err(|e| {
                    crate::markdown::MarkdownError::ThemeLoad(format!(
                        "Syntax highlighting failed: {}",
                        e
                    ))
                })?;

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
    let bg = highlighter
        .theme()
        .settings
        .background
        .unwrap_or(syntect::highlighting::Color {
            r: 40,
            g: 44,
            b: 52,
            a: 255,
        });

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

mark {{
    background-color: var(--highlight-bg, #fff3b8);
    color: var(--highlight-fg, inherit);
    padding: 0.1em 0.2em;
    border-radius: 2px;
}}
</style>
"#,
        bg.r,
        bg.g,
        bg.b,
        bg.r.saturating_sub(10),
        bg.g.saturating_sub(10),
        bg.b.saturating_sub(10)
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
    fn test_as_html_link_with_title() {
        // Title mode - plain title becomes title attribute
        let md: Markdown = r#"[Click here](https://example.com "A tooltip")"#.into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains(r#"href="https://example.com""#));
        assert!(html.contains(r#"title="A tooltip""#));
        assert!(html.contains("Click here"));
    }

    #[test]
    fn test_as_html_link_structured_mode() {
        // Structured mode - parses class, style, etc.
        let md: Markdown =
            r#"[Click here](https://example.com "class='btn' style='color:red'")"#.into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains(r#"href="https://example.com""#));
        assert!(html.contains(r#"class="btn""#));
        assert!(html.contains(r#"style="color:red""#));
        // Should NOT have title attribute in structured mode (unless title= key is used)
        assert!(!html.contains("title="));
        assert!(html.contains("Click here"));
    }

    #[test]
    fn test_as_html_link_structured_mode_with_prompt() {
        let md: Markdown =
            r#"[Hover me](https://example.com "prompt='Click for more info'")"#.into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains(r#"data-prompt="Click for more info""#));
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

    #[test]
    fn test_html_strikethrough_basic() {
        let md: Markdown = "This is ~~strikethrough~~ text.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<del>"), "Should contain opening del tag");
        assert!(html.contains("</del>"), "Should contain closing del tag");
        assert!(html.contains("strikethrough"));
    }

    #[test]
    fn test_html_strikethrough_nested() {
        let md: Markdown = "This is **~~bold strikethrough~~** text.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<strong>"), "Should contain strong tag");
        assert!(html.contains("<del>"), "Should contain del tag");
        assert!(html.contains("</del>"), "Should contain closing del tag");
        assert!(
            html.contains("</strong>"),
            "Should contain closing strong tag"
        );
        assert!(html.contains("bold strikethrough"));
    }

    #[test]
    fn test_html_no_strikethrough_without_markers() {
        let md: Markdown = "This is normal text without strikethrough.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(
            !html.contains("<del>"),
            "Should not contain del tag for normal text"
        );
    }

    #[test]
    fn test_html_strikethrough_unclosed() {
        let md: Markdown = "This has ~~unclosed strikethrough markers.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        // Unclosed markers should be rendered literally, not as strikethrough
        assert!(
            html.contains("~~unclosed"),
            "Unclosed markers should render literally"
        );
    }

    #[test]
    fn test_html_strikethrough_multiple() {
        let md: Markdown = "This has ~~one~~ and ~~two~~ strikethroughs.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        // Should contain multiple del tags
        let del_count = html.matches("<del>").count();
        assert!(
            del_count >= 2,
            "Should contain at least 2 del tags for multiple strikethroughs"
        );
        assert!(html.contains("one"));
        assert!(html.contains("two"));
    }

    #[test]
    fn test_html_strikethrough_preserves_other_styles() {
        let md: Markdown = "This has **bold** and ~~strikethrough~~ text.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<strong>"), "Should contain strong tag");
        assert!(
            html.contains("</strong>"),
            "Should contain closing strong tag"
        );
        assert!(html.contains("<del>"), "Should contain del tag");
        assert!(html.contains("</del>"), "Should contain closing del tag");
        assert!(html.contains("bold"));
        assert!(html.contains("strikethrough"));
    }

    // Highlight/Mark tests
    #[test]
    fn test_html_highlight_basic() {
        let md: Markdown = "This is ==highlighted== text.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<mark>"), "Should contain opening mark tag");
        assert!(html.contains("</mark>"), "Should contain closing mark tag");
        assert!(html.contains("highlighted"));
    }

    #[test]
    fn test_html_highlight_multiple() {
        let md: Markdown = "This has ==one== and ==two== highlights.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        let mark_count = html.matches("<mark>").count();
        assert!(
            mark_count >= 2,
            "Should contain at least 2 mark tags, got: {}",
            mark_count
        );
        assert!(html.contains("one"));
        assert!(html.contains("two"));
    }

    #[test]
    fn test_html_highlight_nested_bold() {
        let md: Markdown = "This is **==bold highlight==** text.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<strong>"), "Should contain strong tag");
        assert!(html.contains("<mark>"), "Should contain mark tag");
        assert!(html.contains("bold highlight"));
    }

    #[test]
    fn test_html_highlight_nested_italic() {
        let md: Markdown = "This is *==italic highlight==* text.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<em>"), "Should contain em tag");
        assert!(html.contains("<mark>"), "Should contain mark tag");
        assert!(html.contains("italic highlight"));
    }

    #[test]
    fn test_html_highlight_no_markers() {
        let md: Markdown = "This is normal text without highlights.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(
            !html.contains("<mark>"),
            "Should not contain mark tag for normal text"
        );
    }

    #[test]
    fn test_html_highlight_unclosed() {
        let md: Markdown = "This has ==unclosed highlight text.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        // Unclosed markers should be rendered literally
        assert!(
            html.contains("==unclosed") || html.contains("=="),
            "Unclosed markers should render literally"
        );
    }

    #[test]
    fn test_html_highlight_in_inline_code() {
        let md: Markdown = "Use `==code==` syntax.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        // Should have code tag but not process == inside code
        assert!(html.contains("<code>"), "Should contain code tag");
        assert!(
            html.contains("==code=="),
            "Should preserve == in inline code"
        );
    }

    #[test]
    fn test_html_highlight_css_included() {
        let md: Markdown = "==test==".into();
        let options = HtmlOptions {
            include_styles: true,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        assert!(html.contains("mark {"), "CSS should include mark selector");
        assert!(
            html.contains("--highlight-bg"),
            "CSS should include CSS variable"
        );
    }

    #[test]
    fn test_html_highlight_preserves_other_styles() {
        let md: Markdown = "**bold** and ==highlight== and *italic*".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<strong>"), "Should preserve strong");
        assert!(html.contains("<mark>"), "Should have mark");
        assert!(html.contains("<em>"), "Should preserve em");
    }

    // Mermaid rendering tests - regression tests for mermaid code block rendering bug
    #[test]
    fn test_mermaid_off_renders_as_code_block() {
        // With MermaidMode::Off (default), mermaid blocks are syntax-highlighted code
        let content = r#"```mermaid
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let options = HtmlOptions {
            mermaid_mode: MermaidMode::Off,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        // Should render as normal code block, not as mermaid diagram
        assert!(html.contains("code-block"), "Should have code-block class");
        assert!(
            !html.contains("class=\"mermaid\""),
            "Should not have mermaid class"
        );
        assert!(
            !html.contains("mermaid.initialize"),
            "Should not include mermaid.js"
        );
    }

    #[test]
    fn test_mermaid_image_renders_as_diagram() {
        // Regression test: MermaidMode::Image should render as interactive diagram
        let content = r#"```mermaid
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let options = HtmlOptions {
            mermaid_mode: MermaidMode::Image,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        // Should render as mermaid pre element for mermaid.js
        assert!(
            html.contains("class=\"mermaid\""),
            "Should have mermaid class for mermaid.js"
        );
        assert!(html.contains("role=\"img\""), "Should have ARIA role");
        assert!(html.contains("aria-label="), "Should have ARIA label");
        // Should include mermaid.js script
        assert!(
            html.contains("mermaid.initialize"),
            "Should include mermaid initialization"
        );
        assert!(
            html.contains("cdn.jsdelivr.net/npm/mermaid"),
            "Should include mermaid CDN"
        );
    }

    #[test]
    fn test_mermaid_text_renders_as_code_block() {
        // MermaidMode::Text renders as plain code block (fallback)
        let content = r#"```mermaid
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let options = HtmlOptions {
            mermaid_mode: MermaidMode::Text,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        // Should render as pre/code with language-mermaid class
        assert!(
            html.contains("language-mermaid"),
            "Should have language-mermaid class"
        );
        assert!(html.contains("flowchart"), "Should contain diagram source");
        assert!(
            !html.contains("mermaid.initialize"),
            "Should not include mermaid.js"
        );
    }

    #[test]
    fn test_mermaid_with_title() {
        // Mermaid blocks with title metadata
        let content = r#"```mermaid title="My Flowchart"
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let options = HtmlOptions {
            mermaid_mode: MermaidMode::Image,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        assert!(
            html.contains("title=\"My Flowchart\""),
            "Should include title attribute"
        );
    }

    #[test]
    fn test_mermaid_multiple_diagrams() {
        // Multiple mermaid diagrams should all render and only include script once
        let content = r#"```mermaid
flowchart LR
    A --> B
```

Some text.

```mermaid
sequenceDiagram
    A->>B: Hello
```"#;
        let md: Markdown = content.into();
        let options = HtmlOptions {
            mermaid_mode: MermaidMode::Image,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        // Both diagrams should render
        let mermaid_count = html.matches("class=\"mermaid\"").count();
        assert_eq!(mermaid_count, 2, "Should have 2 mermaid diagrams");
        // Script should only appear once (at the end)
        let script_count = html.matches("mermaid.initialize").count();
        assert_eq!(script_count, 1, "Should have only 1 mermaid script");
    }

    #[test]
    fn test_mermaid_escapes_xss() {
        // Mermaid content should be HTML-escaped
        let content = r#"```mermaid
flowchart LR
    A["<script>alert('xss')</script>"] --> B
```"#;
        let md: Markdown = content.into();
        let options = HtmlOptions {
            mermaid_mode: MermaidMode::Image,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        // Should escape script tags
        assert!(
            !html.contains("<script>alert"),
            "Should escape XSS in mermaid content"
        );
        assert!(
            html.contains("&lt;script&gt;") || html.contains("&#60;script&#62;"),
            "Should have escaped entities"
        );
    }

    #[test]
    fn test_mermaid_case_insensitive() {
        // Language detection should be case-insensitive
        let content = r#"```MERMAID
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let options = HtmlOptions {
            mermaid_mode: MermaidMode::Image,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        assert!(
            html.contains("class=\"mermaid\""),
            "Should detect MERMAID (uppercase)"
        );
    }

    #[test]
    fn test_mermaid_mixed_with_regular_code() {
        // Document with both mermaid and regular code blocks
        let content = r#"```rust
fn main() {}
```

```mermaid
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let options = HtmlOptions {
            mermaid_mode: MermaidMode::Image,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        // Should have both: syntax-highlighted rust and mermaid diagram
        assert!(html.contains("code-block"), "Should have rust code block");
        assert!(
            html.contains("class=\"mermaid\""),
            "Should have mermaid diagram"
        );
        assert!(
            html.contains("mermaid.initialize"),
            "Should include mermaid script"
        );
    }

    #[test]
    fn test_mermaid_no_script_when_no_diagrams() {
        // When no mermaid diagrams exist, don't include the script
        let content = r#"# Hello

```rust
fn main() {}
```"#;
        let md: Markdown = content.into();
        let options = HtmlOptions {
            mermaid_mode: MermaidMode::Image,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();
        assert!(
            !html.contains("mermaid.initialize"),
            "Should not include mermaid script when no diagrams"
        );
    }

    #[test]
    fn test_mermaid_has_icon_pack_registration() {
        // Verify icon packs are registered before mermaid.initialize()
        let content = r#"```mermaid
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let options = HtmlOptions {
            mermaid_mode: MermaidMode::Image,
            ..Default::default()
        };
        let html = as_html(&md, options).unwrap();

        // Verify registerIconPacks is present
        assert!(
            html.contains("mermaid.registerIconPacks"),
            "Should include registerIconPacks call"
        );

        // Verify all 4 icon packs are registered
        assert!(
            html.contains("@iconify-json/fa7-brands"),
            "Should register fa7-brands pack"
        );
        assert!(
            html.contains("@iconify-json/lucide"),
            "Should register lucide pack"
        );
        assert!(
            html.contains("@iconify-json/carbon"),
            "Should register carbon pack"
        );
        assert!(
            html.contains("@iconify-json/system-uicons"),
            "Should register system-uicons pack"
        );

        // Verify registerIconPacks comes before initialize
        let register_pos = html
            .find("registerIconPacks")
            .expect("registerIconPacks should exist");
        let initialize_pos = html
            .find("mermaid.initialize")
            .expect("initialize should exist");
        assert!(
            register_pos < initialize_pos,
            "registerIconPacks should come before initialize"
        );
    }
}
