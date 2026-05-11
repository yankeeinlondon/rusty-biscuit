//! HTML output with syntax highlighting for code blocks and prose.
//!
//! This module provides HTML rendering for markdown documents with full syntax highlighting
//! support for both code blocks and prose elements. It uses syntect for highlighting and
//! supports customizable themes, line numbering, and line highlighting.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::Markdown;
//! use darkmatter::markdown::output::{HtmlOptions, as_html};
//! use darkmatter::markdown::highlighting::{ThemePair, ColorMode};
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

use crate::markdown::block::{
    RuleProcessor, build_rule_with_defaults, hr_defaults_from_frontmatter,
};
use crate::markdown::dsl::parse_code_info;
use crate::markdown::highlighting::{CodeHighlighter, ColorMode, ThemePair};
use crate::markdown::inline::{InlineEvent, InlineStyleProcessor, InlineTag};
use crate::markdown::output::code_block;
use crate::markdown::output::terminal::MermaidMode;
use crate::markdown::{Markdown, MarkdownResult};
use crate::mermaid::Mermaid;
use crate::render::{ImageRef, Link};
use biscuit_terminal::components::horizontal_rule::HorizontalRule;
use biscuit_terminal::components::renderable::BrowserRenderable;
use html_escape;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// Options for HTML output with sensible defaults.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::output::HtmlOptions;
/// use darkmatter::markdown::highlighting::{ThemePair, ColorMode};
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
    /// Overrides for horizontal-rule CSS custom properties.
    ///
    /// When non-empty, each emitted `<svg>` for a [`HorizontalRule`] is run
    /// through
    /// [`BrowserRenderable::render_to_browser_with_inline_variables`],
    /// which substitutes `var(--hr-*)` tokens with concrete values. Keys
    /// match the CSS variable names *without* the `--` prefix (e.g.,
    /// `hr-weight`, `hr-color`, `hr-width`).
    ///
    /// An empty map (the default) means "no overrides" — the generated SVG
    /// keeps its `var(--hr-*, …)` expressions so page-level CSS or
    /// downstream code can override them.
    pub hr_css_variables: std::collections::HashMap<String, String>,
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
            hr_css_variables: std::collections::HashMap::new(),
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
/// use darkmatter::markdown::Markdown;
/// use darkmatter::markdown::output::{HtmlOptions, as_html};
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
    let hr_defaults = hr_defaults_from_frontmatter(md);

    // Create highlighter for code blocks
    let code_highlighter = CodeHighlighter::new(options.code_theme, options.color_mode);

    // Include styles if requested
    if options.include_styles {
        output.push_str(&generate_styles(&code_highlighter, &options));
    }

    // Parse markdown content with GFM strikethrough extension and wrap with MarkProcessor
    // and RuleProcessor for horizontal rules with attributes
    let preprocessed = crate::markdown::inline::preprocess_escaped_markers(md.content());
    let parser = Parser::new_ext(&preprocessed, Options::ENABLE_STRIKETHROUGH);
    let events = RuleProcessor::new(InlineStyleProcessor::new(parser));

    // Track state for code blocks
    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut code_lang = String::new();
    let mut code_info = String::new();
    let mut has_mermaid = false;
    let mut in_image = false;
    let mut current_image_alt = String::new();
    let mut current_image_src = String::new();
    let mut current_image_title = String::new();

    for event in events {
        match event {
            // Handle custom inline tags (highlight/mark)
            InlineEvent::Start(InlineTag::Mark) => {
                output.push_str("<mark>");
            }
            InlineEvent::End(InlineTag::Mark) => {
                output.push_str("</mark>");
            }
            InlineEvent::Start(InlineTag::Dim) => {
                output.push('⌄');
            }
            InlineEvent::End(InlineTag::Dim) => {
                output.push('⌄');
            }
            // Handle horizontal rule with attributes
            InlineEvent::HorizontalRule(attrs) => {
                // Create HorizontalRule from attributes via the shared builder
                // so terminal and HTML renderers stay consistent (Phase 5).
                let rule = build_rule_with_defaults(hr_defaults.as_ref(), &attrs);
                output.push_str(&render_rule_browser(&rule, &options.hr_css_variables));
                output.push('\n');
            }
            // Phase 5 (B4): bare `---` / `***` / `___` lines surface as
            // pulldown-cmark `Event::Rule`. Handle them explicitly so the
            // browser output gets a default SVG instead of falling through
            // the catch-all arm.
            InlineEvent::Standard(Event::Rule) => {
                let rule = build_rule_with_defaults(
                    hr_defaults.as_ref(),
                    &crate::markdown::inline::HorizontalRuleAttrs::default(),
                );
                output.push_str(&render_rule_browser(&rule, &options.hr_css_variables));
                output.push('\n');
            }
            // Handle standard pulldown-cmark events
            InlineEvent::Standard(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))) => {
                in_code_block = true;
                code_info = info.to_string();
                code_buffer.clear();
                code_lang.clear();
            }
            InlineEvent::Standard(Event::End(TagEnd::CodeBlock)) if in_code_block => {
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
                    let highlighted = code_block::render_html_code_block(
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
                // Parse title for structured content (class, style, prompt, etc.).
                // We use a placeholder display since we're streaming; actual text follows.
                let link = Link::with_title_parsed("", &*dest_url, &title).ok();

                // Build anchor tag with parsed attributes
                let mut attrs = format!(r#"href="{}""#, html_escape::encode_text(&dest_url));

                if let Some(link) = link.as_ref() {
                    if let Some(class) = link.class() {
                        attrs.push_str(&format!(r#" class="{}""#, html_escape::encode_text(class)));
                    }
                    if let Some(style) = link.style_css() {
                        attrs
                            .push_str(&format!(r#" style="{}""#, html_escape::encode_text(&style)));
                    }
                    if let Some(target) = link.target_attr() {
                        attrs.push_str(&format!(
                            r#" target="{}""#,
                            html_escape::encode_text(&target)
                        ));
                    }
                    if let Some(title) = link.title_plain() {
                        attrs
                            .push_str(&format!(r#" title="{}""#, html_escape::encode_text(&title)));
                    }
                    if let Some(prompt) = link.prompt() {
                        attrs.push_str(&format!(
                            r#" data-prompt="{}""#,
                            html_escape::encode_text(prompt)
                        ));
                    }
                    for (key, value) in link.data() {
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
                if in_image {
                    current_image_alt.push('`');
                    current_image_alt.push_str(&text);
                    current_image_alt.push('`');
                } else {
                    output.push_str(&format!("<code>{}</code>", html_escape::encode_text(&text)));
                }
            }
            InlineEvent::Standard(Event::Start(Tag::Image {
                dest_url, title, ..
            })) => {
                in_image = true;
                current_image_alt.clear();
                current_image_src = dest_url.to_string();
                current_image_title = title.to_string();
            }
            InlineEvent::Standard(Event::End(TagEnd::Image)) => {
                if in_image {
                    if let Some(image_ref) = image_ref_from_parts(
                        &current_image_alt,
                        &current_image_src,
                        &current_image_title,
                    ) {
                        output.push_str(&image_ref.to_html());
                    } else {
                        output.push_str(&format!(
                            r#"<img src="{}" alt="{}" />"#,
                            html_escape::encode_text(&current_image_src),
                            html_escape::encode_text(&current_image_alt)
                        ));
                    }
                }

                in_image = false;
                current_image_alt.clear();
                current_image_src.clear();
                current_image_title.clear();
            }
            InlineEvent::Standard(Event::Text(text)) if !in_code_block => {
                if in_image {
                    current_image_alt.push_str(&text);
                } else {
                    output.push_str(html_escape::encode_text(&text).as_ref());
                }
            }
            InlineEvent::Standard(Event::SoftBreak) => {
                if in_image {
                    current_image_alt.push(' ');
                } else {
                    output.push('\n');
                }
            }
            InlineEvent::Standard(Event::HardBreak) => {
                if in_image {
                    current_image_alt.push('\n');
                } else {
                    output.push_str("<br>\n");
                }
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

/// Renders a [`HorizontalRule`] to browser SVG, optionally substituting
/// `var(--hr-*)` custom properties with caller-provided overrides.
///
/// ## Notes
///
/// When `vars` is `Some`, each `var(--name)` token in the default SVG is
/// replaced via
/// [`BrowserRenderable::render_to_browser_with_inline_variables`]. When
/// `vars` is `None`, the SVG keeps its `var(--…)` expressions so page-level
/// CSS (or downstream post-processing) can control the appearance.
fn render_rule_browser(
    rule: &HorizontalRule,
    vars: &std::collections::HashMap<String, String>,
) -> String {
    if vars.is_empty() {
        rule.render_to_browser()
    } else {
        rule.render_to_browser_with_inline_variables(vars)
    }
}

fn image_ref_from_parts(alt: &str, src: &str, title: &str) -> Option<ImageRef> {
    let markdown = build_markdown_image_literal(alt, src, title);

    if let Ok(image_ref) = ImageRef::try_from(markdown.as_str()) {
        return Some(image_ref);
    }

    let mut image_ref = ImageRef::new(src, alt).ok()?;
    if !title.trim().is_empty() {
        image_ref = image_ref.with_title(title);
    }
    Some(image_ref)
}

fn build_markdown_image_literal(alt: &str, src: &str, title: &str) -> String {
    let alt = escape_markdown_image_alt(alt);
    let src = escape_markdown_image_url(src);

    if title.trim().is_empty() {
        return format!("![{alt}]({src})");
    }

    let title = escape_markdown_title(title);
    format!("![{alt}]({src} \"{title}\")")
}

fn escape_markdown_image_alt(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn escape_markdown_image_url(value: &str) -> String {
    value.replace('(', "%28").replace(')', "%29")
}

fn escape_markdown_title(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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
        // Styles are normalized through typed CSS parsing.
        assert!(html.contains(r#"style="color: red;""#));
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
    fn test_as_html_image_basic() {
        let md: Markdown = "![Diagram](./diagram.png)".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();

        assert!(
            html.contains(r#"src="./diagram.png""#),
            "HTML was: {}",
            html
        );
        assert!(html.contains(r#"alt="Diagram""#), "HTML was: {}", html);
        assert!(html.contains("<img "), "HTML was: {}", html);
    }

    #[test]
    fn test_as_html_image_width_hint_via_imageref() {
        let md: Markdown = "![My chart|40%](./chart.png)".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();

        assert!(html.contains(r#"alt="My chart""#), "HTML was: {}", html);
        assert!(
            html.contains(r#"style="width: 40%;""#),
            "HTML was: {}",
            html
        );
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

    // Dim tests
    #[test]
    fn test_html_dim_renders_as_literal() {
        let md: Markdown = "This is ⌄dimmed⌄ text.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(
            html.contains("⌄dimmed⌄"),
            "Should preserve ⌄ delimiters as literal, got: {}",
            html
        );
        assert!(!html.contains("<dim>"), "Should not have <dim> tag");
    }

    #[test]
    fn test_html_dim_with_nested_strong() {
        let md: Markdown = "⌄dim and **strong**⌄".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(
            html.contains("<p>⌄dim and <strong>strong</strong>⌄</p>"),
            "Should preserve delimiters around nested HTML, got: {}",
            html
        );
    }

    #[test]
    fn test_html_dim_in_inline_code() {
        let md: Markdown = "Use `⌄code⌄` syntax.".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(html.contains("<code>"), "Should contain code tag");
        assert!(
            html.contains("⌄code⌄"),
            "Should preserve ⌄ in inline code, got: {}",
            html
        );
    }

    #[test]
    fn test_html_dim_in_fenced_code() {
        let content = "```\n⌄dim\n```";
        let md: Markdown = content.into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(
            html.contains("⌄dim"),
            "Should preserve ⌄ in fenced code, got: {}",
            html
        );
    }

    #[test]
    fn test_html_dim_escaping() {
        let md: Markdown = "⌄<script>alert('xss')</script>⌄".into();
        let html = as_html(&md, HtmlOptions::default()).unwrap();
        assert!(
            !html.contains("<script>alert"),
            "Should escape script inside dim span"
        );
        assert!(
            html.contains("&lt;script&gt;") || html.contains("&#60;script&#62;"),
            "Should have escaped entities inside dim span, got: {}",
            html
        );
        assert!(html.contains("⌄"), "Should preserve ⌄ delimiters");
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
