//! Terminal output with ANSI escape codes for markdown rendering.
//!
//! This module provides terminal-based rendering of markdown documents with
//! syntax highlighting using ANSI escape sequences. It supports:
//!
//! - Auto-detection of terminal color depth
//! - Code block syntax highlighting with line numbers
//! - Code block titles with visual prefix
//! - Configurable themes for code and prose
//!
//! ## Examples
//!
//! ```
//! use shared::markdown::Markdown;
//! use shared::markdown::output::terminal::{for_terminal, TerminalOptions};
//!
//! let content = "# Hello World\n\n\
//!                ```rust\n\
//!                fn main() {\n    \
//!                    println!(\"Hello!\");\n\
//!                }\n\
//!                ```\n";
//!
//! let md: Markdown = content.into();
//! let output = for_terminal(&md, TerminalOptions::default()).unwrap();
//! // Output contains ANSI escape codes for terminal display
//! ```

use crate::markdown::{
    dsl::parse_code_info,
    highlighting::{prose::ProseHighlighter, scope_cache::ScopeCache, CodeHighlighter, ColorMode, ThemePair},
    Markdown, MarkdownError,
};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style};
use syntect::parsing::{Scope, SyntaxReference};
use syntect::util::as_24_bit_terminal_escaped;

/// Color depth capability for terminal.
///
/// Represents the level of color support available in a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDepth {
    /// 24-bit true color (16.7M colors).
    TrueColor,
    /// 8-bit color (256 colors).
    Colors256,
    /// 4-bit color (16 colors).
    Colors16,
    /// No color support.
    None,
}

impl ColorDepth {
    /// Auto-detects color depth from terminal capabilities.
    ///
    /// Uses the `color_depth()` function from the terminal module to determine
    /// the maximum color depth supported by the current terminal.
    ///
    /// ## Returns
    ///
    /// - `TrueColor` if terminal supports 16.7M+ colors
    /// - `Colors256` if terminal supports 256 colors
    /// - `Colors16` if terminal supports basic 16 colors
    /// - `None` if no color support detected
    pub fn auto_detect() -> Self {
        let depth = crate::terminal::color_depth();
        if depth >= 16_777_216 {
            Self::TrueColor
        } else if depth >= 256 {
            Self::Colors256
        } else if depth >= 16 {
            Self::Colors16
        } else {
            Self::None
        }
    }
}

/// Options for terminal output with sensible defaults.
///
/// ## Examples
///
/// ```
/// use shared::markdown::highlighting::{ThemePair, ColorMode};
/// use shared::markdown::output::terminal::{TerminalOptions, ColorDepth};
///
/// let mut options = TerminalOptions::default();
/// options.code_theme = ThemePair::Github;
/// options.prose_theme = ThemePair::Github;
/// options.color_mode = ColorMode::Dark;
/// options.include_line_numbers = true;
/// options.color_depth = Some(ColorDepth::TrueColor);
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TerminalOptions {
    /// Theme pair for code blocks.
    pub code_theme: ThemePair,
    /// Theme pair for prose (unused in Phase 9, reserved for future).
    pub prose_theme: ThemePair,
    /// Color mode (light or dark).
    pub color_mode: ColorMode,
    /// Whether to include line numbers in code blocks.
    pub include_line_numbers: bool,
    /// Color depth capability. None = auto-detect.
    pub color_depth: Option<ColorDepth>,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        use crate::markdown::highlighting::{detect_prose_theme, detect_code_theme, detect_color_mode};

        let prose_theme = detect_prose_theme();
        let code_theme = detect_code_theme(prose_theme);
        let color_mode = detect_color_mode();

        Self {
            code_theme,
            prose_theme,
            color_mode,
            include_line_numbers: false,
            color_depth: None,
        }
    }
}

/// Exports markdown to terminal with ANSI escape codes.
///
/// This function renders markdown content with syntax-highlighted code blocks
/// using ANSI escape sequences for terminal display.
///
/// ## Examples
///
/// ```
/// use shared::markdown::Markdown;
/// use shared::markdown::output::terminal::{for_terminal, TerminalOptions};
///
/// let md: Markdown = "# Hello\n\n```rust\nfn main() {}\n```".into();
/// let output = for_terminal(&md, TerminalOptions::default()).unwrap();
/// ```
///
/// ## Errors
///
/// Returns an error if theme loading fails or syntax highlighting encounters issues.
pub fn for_terminal(md: &Markdown, options: TerminalOptions) -> Result<String, MarkdownError> {
    let color_depth = options.color_depth.unwrap_or_else(ColorDepth::auto_detect);

    // Early return if no color support
    if color_depth == ColorDepth::None {
        return Ok(md.content().to_string());
    }

    let code_highlighter = CodeHighlighter::new(options.code_theme, options.color_mode);

    // Load prose theme for ProseHighlighter
    let prose_syntect_theme = crate::markdown::highlighting::themes::load_theme(
        options.prose_theme,
        options.color_mode,
    );
    let prose_highlighter = ProseHighlighter::new(&prose_syntect_theme);

    let mut output = String::with_capacity(md.content().len() * 2);

    // Track scope stack for prose highlighting (functional style)
    let mut scope_stack: Vec<Scope> = vec![prose_highlighter.base_scope()];

    let parser = Parser::new(md.content());
    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut code_language = String::new();
    let mut code_info_string = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_buffer.clear();

                match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(info) => {
                        code_info_string = info.to_string();
                        code_language = parse_code_info(&code_info_string)
                            .unwrap_or_default()
                            .language;
                    }
                    pulldown_cmark::CodeBlockKind::Indented => {
                        code_language.clear();
                        code_info_string.clear();
                    }
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;

                // Render code block with highlighting
                let meta = parse_code_info(&code_info_string).unwrap_or_default();

                // Add title if present
                if let Some(title) = &meta.title {
                    output.push_str(&format_title(title));
                    output.push('\n');
                }

                // Highlight and render code
                let highlighted = highlight_code(
                    &code_buffer,
                    &code_language,
                    &code_highlighter,
                    &options,
                    &meta,
                )?;
                output.push_str(&highlighted);
                output.push('\n');
            }
            Event::Text(text) if in_code_block => {
                code_buffer.push_str(&text);
            }

            // Prose highlighting with scope tracking
            Event::Start(ref tag @ Tag::Heading { level, .. }) => {
                if let Some(scope) = ScopeCache::global().scope_for_tag(tag) {
                    scope_stack.push(scope);
                }
                let marker = match level {
                    pulldown_cmark::HeadingLevel::H1 => "# ",
                    pulldown_cmark::HeadingLevel::H2 => "## ",
                    pulldown_cmark::HeadingLevel::H3 => "### ",
                    pulldown_cmark::HeadingLevel::H4 => "#### ",
                    pulldown_cmark::HeadingLevel::H5 => "##### ",
                    pulldown_cmark::HeadingLevel::H6 => "###### ",
                };
                let style = prose_highlighter.style_for_tag(tag, &scope_stack);
                output.push_str(&emit_prose_text(marker, style));
            }
            Event::End(TagEnd::Heading(_)) => {
                scope_stack.pop();
                output.push('\n');
            }

            Event::Start(ref tag @ Tag::Strong) => {
                if let Some(scope) = ScopeCache::global().scope_for_tag(tag) {
                    scope_stack.push(scope);
                }
            }
            Event::End(TagEnd::Strong) => {
                scope_stack.pop();
            }

            Event::Start(ref tag @ Tag::Emphasis) => {
                if let Some(scope) = ScopeCache::global().scope_for_tag(tag) {
                    scope_stack.push(scope);
                }
            }
            Event::End(TagEnd::Emphasis) => {
                scope_stack.pop();
            }

            Event::Start(ref tag @ Tag::Link { .. }) => {
                if let Some(scope) = ScopeCache::global().scope_for_tag(tag) {
                    scope_stack.push(scope);
                }
            }
            Event::End(TagEnd::Link) => {
                scope_stack.pop();
            }

            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                output.push_str("\n\n");
            }

            Event::Text(text) if !in_code_block => {
                // Apply current prose styling based on scope stack
                let style = if scope_stack.len() > 1 {
                    // We have nested scopes, compute style from stack
                    // Use a neutral tag reference for computing the style
                    let tag = Tag::Paragraph;
                    prose_highlighter.style_for_tag(&tag, &scope_stack)
                } else {
                    prose_highlighter.base_style()
                };
                output.push_str(&emit_prose_text(&text, style));
            }

            Event::Code(code) => {
                // Inline code with styling
                let style = prose_highlighter.style_for_inline_code(&scope_stack);
                output.push('`');
                output.push_str(&emit_prose_text(&code, style));
                output.push('`');
            }

            Event::SoftBreak => {
                output.push(' ');
            }
            Event::HardBreak => {
                output.push('\n');
            }
            _ => {} // Ignore other events
        }
    }

    // Always emit terminal reset at end
    output.push_str("\x1b[0m");

    Ok(output)
}

/// Emits prose text with foreground color only (no background).
fn emit_prose_text(text: &str, style: Style) -> String {
    let fg = style.foreground;
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", fg.r, fg.g, fg.b, text)
}

/// Emits code text with both foreground and background colors.
#[cfg(test)]
fn emit_code_text(text: &str, style: Style, bg_color: Color) -> String {
    let fg = style.foreground;
    format!(
        "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m{}\x1b[0m",
        bg_color.r, bg_color.g, bg_color.b,
        fg.r, fg.g, fg.b,
        text
    )
}

/// Formats a code block title with ANSI codes.
fn format_title(title: &str) -> String {
    // Bold: \x1b[1m
    // Reset: \x1b[0m
    format!("\x1b[1m▌ {}\x1b[0m", title)
}

/// Highlights code with syntax highlighting and optional line numbers.
fn highlight_code(
    code: &str,
    language: &str,
    highlighter: &CodeHighlighter,
    options: &TerminalOptions,
    meta: &crate::markdown::dsl::CodeBlockMeta,
) -> Result<String, MarkdownError> {
    let syntax = find_syntax(language, highlighter.syntax_set())
        .unwrap_or_else(|| highlighter.syntax_set().find_syntax_plain_text());
    let theme = highlighter.theme();

    // Get background color from theme
    let bg_color = theme.settings.background.unwrap_or(Color::BLACK);

    let lines: Vec<&str> = code.lines().collect();
    let mut output = String::with_capacity(code.len() * 2);

    // Determine line number width
    let line_number_width = if options.include_line_numbers || meta.line_numbering {
        format!("{}", lines.len()).len()
    } else {
        0
    };

    // Create highlighter for this code block
    let mut hl = HighlightLines::new(syntax, theme);

    for (idx, line) in lines.iter().enumerate() {
        let line_number = idx + 1;

        // Add line number gutter if enabled
        if line_number_width > 0 {
            // Gray color for line numbers: \x1b[38;2;128;128;128m
            output.push_str(&format!(
                "\x1b[38;2;128;128;128m{:>width$} │\x1b[0m ",
                line_number,
                width = line_number_width
            ));
        }

        // Set background color for the line
        output.push_str(&format!(
            "\x1b[48;2;{};{};{}m",
            bg_color.r, bg_color.g, bg_color.b
        ));

        // Highlight the line and get styled ranges
        let ranges = hl
            .highlight_line(line, highlighter.syntax_set())
            .map_err(|e| MarkdownError::ThemeLoad(format!("Syntax highlighting failed: {}", e)))?;

        // Convert to terminal escape codes (true for background)
        let highlighted = as_24_bit_terminal_escaped(&ranges, false);

        output.push_str(&highlighted);

        // Reset background at end of line
        output.push_str("\x1b[0m");

        // Add newline except for last line
        if idx < lines.len() - 1 {
            output.push('\n');
        }
    }

    Ok(output)
}

/// Finds syntax definition by language identifier.
fn find_syntax<'a>(
    language: &str,
    syntax_set: &'a syntect::parsing::SyntaxSet,
) -> Option<&'a SyntaxReference> {
    if language.is_empty() {
        return None;
    }

    // Try by extension first (common case)
    if let Some(syntax) = syntax_set.find_syntax_by_extension(language) {
        return Some(syntax);
    }

    // Try by name
    syntax_set.find_syntax_by_name(language)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{strip_ansi_codes, TestTerminal};

    #[test]
    fn test_color_depth_auto_detect() {
        let depth = ColorDepth::auto_detect();
        // Just verify it returns a valid variant
        assert!(matches!(
            depth,
            ColorDepth::TrueColor
                | ColorDepth::Colors256
                | ColorDepth::Colors16
                | ColorDepth::None
        ));
    }

    #[test]
    fn test_terminal_options_default_uses_detection() {
        // Default options should use environment detection
        let options = TerminalOptions::default();

        // Should have valid themes (not checking specific values since they depend on env)
        assert!(ThemePair::all().contains(&options.prose_theme));
        assert!(ThemePair::all().contains(&options.code_theme));
        assert!(!options.include_line_numbers);
        assert!(options.color_depth.is_none());
    }

    #[test]
    fn test_for_terminal_simple_heading() {
        let md: Markdown = "# Hello World".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Strip ANSI codes and check content
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("# Hello World"));
    }

    #[test]
    fn test_for_terminal_code_block() {
        let content = r#"# Test

```rust
fn main() {
    println!("Hello!");
}
```
"#;
        let md: Markdown = content.into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Should contain ANSI codes
        assert!(output.contains("\x1b["));

        // Content should be present when stripped
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("fn main"));
        assert!(plain.contains("println"));
    }

    #[test]
    fn test_for_terminal_with_line_numbers() {
        let content = "```rust\nfn test() {}\n```";
        let md: Markdown = content.into();

        let mut options = TerminalOptions::default();
        options.include_line_numbers = true;

        let output = for_terminal(&md, options).unwrap();

        // Should contain line number gutter (│)
        assert!(output.contains("│"));
    }

    #[test]
    fn test_for_terminal_code_block_with_title() {
        let content = r#"```rust title="Example"
fn main() {}
```"#;
        let md: Markdown = content.into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Should contain title with prefix
        assert!(output.contains("▌ Example"));

        // Title should be bold
        assert!(output.contains("\x1b[1m"));
    }

    #[test]
    fn test_for_terminal_inline_code() {
        let md: Markdown = "Use `cargo build` to compile.".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("`cargo build`"));
    }

    #[test]
    fn test_for_terminal_paragraph() {
        let md: Markdown = "First paragraph.\n\nSecond paragraph.".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("First paragraph"));
        assert!(plain.contains("Second paragraph"));
    }

    #[test]
    fn test_format_title() {
        let title = format_title("Test Title");

        // Should contain bold ANSI code
        assert!(title.contains("\x1b[1m"));
        // Should contain reset code
        assert!(title.contains("\x1b[0m"));
        // Should contain prefix
        assert!(title.contains("▌"));

        let plain = strip_ansi_codes(&title);
        assert_eq!(plain, "▌ Test Title");
    }

    #[test]
    fn test_highlight_code_basic() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let options = TerminalOptions::default();
        let meta = crate::markdown::dsl::CodeBlockMeta::default();

        let code = "fn main() {}";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Should contain ANSI escape codes
        assert!(output.contains("\x1b["));
    }

    #[test]
    fn test_find_syntax_by_extension() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let syntax = find_syntax("rs", highlighter.syntax_set());

        assert!(syntax.is_some());
        assert_eq!(syntax.unwrap().name, "Rust");
    }

    #[test]
    fn test_find_syntax_unknown_language() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let syntax = find_syntax("unknown_language", highlighter.syntax_set());

        // Should return None for unknown languages
        assert!(syntax.is_none());
    }

    #[test]
    fn test_for_terminal_with_test_terminal() {
        let terminal = TestTerminal::new();

        terminal.run(|buf| {
            let md: Markdown = "```rust\nfn test() {}\n```".into();
            let output = for_terminal(&md, TerminalOptions::default()).unwrap();
            buf.push_str(&output);
        });

        // Verify ANSI codes are present
        let raw = terminal.get_output();
        assert!(raw.contains("\x1b["));

        // Verify content after stripping ANSI (output includes trailing newline)
        terminal.assert_output("fn test() {}\n");
    }

    #[test]
    fn test_color_depth_no_color_returns_plain_text() {
        let md: Markdown = "# Test\n\n```rust\nfn main() {}\n```".into();

        let mut options = TerminalOptions::default();
        options.color_depth = Some(ColorDepth::None);

        let output = for_terminal(&md, options).unwrap();

        // Should not contain ANSI codes
        assert!(!output.contains("\x1b["));

        // Should return plain content
        assert_eq!(output, md.content());
    }

    #[test]
    fn test_for_terminal_prose_no_background() {
        let md: Markdown = "# Hello World\n\nSome **bold** text.".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Should contain foreground color codes (38;2)
        assert!(output.contains("\x1b[38;2;"));

        // Prose should NOT contain background color codes (48;2)
        // Split at code blocks and check prose sections
        let prose_section = output.split("```").next().unwrap_or(&output);
        // Count background codes - should be minimal for prose
        let bg_count = prose_section.matches("\x1b[48;2;").count();
        assert_eq!(bg_count, 0, "Prose should not have background colors");
    }

    #[test]
    fn test_for_terminal_code_has_background() {
        let md: Markdown = "```rust\nfn main() {}\n```".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Code blocks should have background color
        assert!(output.contains("\x1b[48;2;"), "Code should have background colors");
    }

    #[test]
    fn test_for_terminal_ends_with_reset() {
        let md: Markdown = "# Test".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Output should end with reset code
        assert!(output.ends_with("\x1b[0m"), "Output should end with terminal reset");
    }

    #[test]
    fn test_emit_prose_text_no_background() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color { r: 255, g: 128, b: 64, a: 255 },
            background: Color { r: 0, g: 0, b: 0, a: 255 },
            font_style: FontStyle::empty(),
        };

        let result = emit_prose_text("Hello", style);

        // Should have foreground
        assert!(result.contains("\x1b[38;2;255;128;64m"));
        // Should NOT have background
        assert!(!result.contains("\x1b[48;2;"));
        // Should end with reset
        assert!(result.contains("\x1b[0m"));
    }

    #[test]
    fn test_emit_code_text_has_background() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color { r: 255, g: 128, b: 64, a: 255 },
            background: Color { r: 30, g: 30, b: 30, a: 255 },
            font_style: FontStyle::empty(),
        };
        let bg = Color { r: 30, g: 30, b: 30, a: 255 };

        let result = emit_code_text("code", style, bg);

        // Should have both foreground and background
        assert!(result.contains("\x1b[48;2;30;30;30m"));
        assert!(result.contains("\x1b[38;2;255;128;64m"));
    }

    #[test]
    fn test_for_terminal_heading_styled() {
        let md: Markdown = "# Heading 1\n\n## Heading 2".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Should contain ANSI codes for styling
        assert!(output.contains("\x1b[38;2;"));

        // Should contain heading markers
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("# Heading 1"));
        assert!(plain.contains("## Heading 2"));
    }

    #[test]
    fn test_for_terminal_inline_code_styled() {
        let md: Markdown = "Use `cargo build` to compile.".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Should contain ANSI codes
        assert!(output.contains("\x1b[38;2;"));

        // Should NOT have background for inline code (prose style)
        let bg_count = output.matches("\x1b[48;2;").count();
        assert_eq!(bg_count, 0, "Inline code should not have background colors in prose");

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("`cargo build`"));
    }

    #[test]
    fn test_for_terminal_nested_styling() {
        let md: Markdown = "# Heading with **bold** text".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Should contain ANSI codes
        assert!(output.contains("\x1b[38;2;"));

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("# Heading with bold text"));
    }
}
