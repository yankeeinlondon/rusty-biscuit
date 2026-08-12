//! Shared code-block rendering helpers for terminal and HTML output.
//!
//! This module provides reusable private helpers that both [`super::terminal`]
//! and [`super::html`] use for syntax-highlighted code blocks. Keeping the
//! helpers in one place ensures that [`YamlBlock`](crate::markdown::yaml_block::YamlBlock)
//! and Markdown code fences share the same `syntect` / `two-face` behaviour.

use crate::markdown::{
    MarkdownError, MarkdownResult,
    dsl::CodeBlockMeta,
    highlighting::{CodeHighlighter, ColorMode},
    language_grammar::LanguageGrammar,
    output::html::HtmlOptions,
    output::terminal::TerminalOptions,
};
use std::fmt::Write as _;
use syntect::easy::HighlightLines;
use syntect::highlighting::Color;
use syntect::util::LinesWithEndings;

/// Renders a code block for terminal output with syntax highlighting.
///
/// This is the shared implementation extracted from the previous private
/// `highlight_code` in [`super::terminal`]. It applies syntax highlighting,
/// adds top/bottom padding rows, and optionally renders line numbers and
/// line highlighting based on DSL metadata.
///
/// ## Arguments
///
/// * `code` - Source code to highlight
/// * `language` - Resolved [`LanguageGrammar`] for syntax highlighting
/// * `highlighter` - Code highlighter with loaded syntax set and theme
/// * `options` - Terminal rendering options (includes global line numbering flag)
/// * `meta` - Code block DSL metadata (title, highlight ranges, line numbering override)
/// * `color_mode` - Dark or light mode for computing highlight background colors
/// * `target_width` - Optional component render width. When `Some`, each line
///   pads to exactly `target_width` visible columns using the line background
///   color instead of clearing to the terminal edge with `\x1b[K`. This is
///   what component `Layout.max_width`, `width`, and `padding` need so the
///   code block body honours its resolved width.
///
/// ## Returns
///
/// ANSI-formatted string with:
/// - Top padding row (blank line with theme background)
/// - Code lines with syntax highlighting, optional line numbers, and highlight backgrounds
/// - Bottom padding row (blank line with theme background)
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_terminal_code_block(
    code: &str,
    language: &LanguageGrammar,
    highlighter: &CodeHighlighter,
    options: &TerminalOptions,
    meta: &CodeBlockMeta,
    color_mode: ColorMode,
    target_width: Option<u16>,
    override_bg: Option<Color>,
) -> Result<String, MarkdownError> {
    let syntax = language
        .resolve(highlighter.syntax_set())
        .unwrap_or_else(|_| highlighter.syntax_set().find_syntax_plain_text());
    let theme = highlighter.theme();

    // Use override background when provided (e.g. from page/component style),
    // otherwise fall back to the theme background.
    let bg_color = override_bg.unwrap_or_else(|| theme.settings.background.unwrap_or(Color::BLACK));

    // Use LinesWithEndings to preserve newlines - required for proper multi-line
    // syntax parsing in grammars like bash/shell that track state across lines
    let lines: Vec<&str> = LinesWithEndings::from(code).collect();
    let mut output = String::with_capacity(code.len() * 2);

    // Determine line number width
    let line_number_width = if options.include_line_numbers || meta.line_numbering {
        format!("{}", lines.len()).len()
    } else {
        0
    };

    // Add top padding row
    output.push_str(&emit_padding_row(bg_color, target_width));

    // Create highlighter for this code block
    let mut hl = HighlightLines::new(syntax, theme);

    for (idx, line) in lines.iter().enumerate() {
        let line_number = idx + 1;

        // Determine if this line should be highlighted
        let is_highlighted = meta.highlight.contains(line_number);
        let line_bg = if is_highlighted {
            compute_highlight_bg(bg_color, color_mode)
        } else {
            bg_color
        };

        // Code blocks are self-contained surfaces. Clear inherited text
        // attributes before applying the theme's own background and foreground.
        output.push_str("\x1b[0m");

        // Set background color for the line (applies to gutter and content)
        let _ = write!(output, "\x1b[48;2;{};{};{}m", line_bg.r, line_bg.g, line_bg.b);

        // Visible width used by gutter + content so we can pad correctly when
        // `target_width` is set.
        let mut visible_used: usize = 0;

        // Add line number gutter if enabled (with background already set)
        if line_number_width > 0 {
            // Gray foreground for line numbers, background already set above
            let _ = write!(
                output,
                "\x1b[38;2;128;128;128m{:>width$} │ ",
                line_number,
                width = line_number_width
            );
            // gutter visible width = number digits + " │ " (3 chars: space, │, space)
            visible_used += line_number_width + 3;
        } else {
            // Add left padding (1 character) when no line numbers
            output.push(' ');
            visible_used += 1;
        }

        // Highlight the line and get styled ranges
        let ranges = hl
            .highlight_line(line, highlighter.syntax_set())
            .map_err(|e| MarkdownError::ThemeLoad(format!("Syntax highlighting failed: {}", e)))?;

        // Convert styled ranges to ANSI escape codes, but filter out newlines
        // from the output (we handle line breaks ourselves)
        for (style, text) in &ranges {
            let text_without_newline = text.trim_end_matches('\n');
            if !text_without_newline.is_empty() {
                let _ = write!(
                    output,
                    "\x1b[38;2;{};{};{}m{}",
                    style.foreground.r,
                    style.foreground.g,
                    style.foreground.b,
                    text_without_newline
                );
                visible_used +=
                    biscuit_terminal::utils::block_constraint::visible_width(text_without_newline)
                        as usize;
            }
        }

        // Either pad to exact width with spaces (when constrained) or clear to
        // end of line. Background is still set, so the trailing spaces inherit
        // the line background color.
        match target_width {
            Some(w) => {
                let pad = (w as usize).saturating_sub(visible_used);
                if pad > 0 {
                    output.push_str(&" ".repeat(pad));
                }
                output.push_str("\x1b[0m");
            }
            None => {
                // \x1b[K clears from cursor to end of line using current background
                output.push_str("\x1b[K\x1b[0m");
            }
        }

        // Add newline after each line (including last line, so bottom padding is on its own line)
        output.push('\n');
    }

    // Add bottom padding row (without trailing newline to avoid double spacing)
    match target_width {
        Some(w) => {
            output.push_str(&format!(
                "\x1b[0m\x1b[48;2;{};{};{}m{}\x1b[0m",
                bg_color.r,
                bg_color.g,
                bg_color.b,
                " ".repeat(w as usize)
            ));
        }
        None => {
            output.push_str(&format!(
                "\x1b[0m\x1b[48;2;{};{};{}m\x1b[K\x1b[0m",
                bg_color.r, bg_color.g, bg_color.b
            ));
        }
    }

    Ok(output)
}

/// Renders a code block for HTML output with syntax highlighting.
///
/// This is the shared implementation extracted from the previous private
/// `highlight_code_block` in [`super::html`]. It produces a `<div class="code-block">`
/// wrapper containing an optional title, and either a `<table>` with line numbers
/// or a `<pre><code>` block without them.
///
/// ## Arguments
///
/// * `code` - Source code to highlight
/// * `language` - Resolved [`LanguageGrammar`] for syntax highlighting and
///   the HTML `language-*` class
/// * `meta` - Code block DSL metadata
/// * `highlighter` - Code highlighter with loaded syntax set and theme
/// * `options` - HTML rendering options
///
/// ## Returns
///
/// HTML string containing the highlighted code block.
pub(crate) fn render_html_code_block(
    code: &str,
    language: &LanguageGrammar,
    meta: &CodeBlockMeta,
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

    // Resolve syntax through the same LanguageGrammar path as the terminal renderer.
    let syntax = language
        .resolve(highlighter.syntax_set())
        .unwrap_or_else(|_| highlighter.syntax_set().find_syntax_plain_text());

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

            let _ = write!(
                output,
                r#"<tr{}><td class="ln-gutter"><span class="ln">{}</span></td><td class="code-content">"#,
                if is_highlighted { r#" class="highlighted""# } else { "" },
                line_num
            );

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
                let _ = write!(
                    output,
                    r#"<span style="color: #{:02x}{:02x}{:02x};">{}</span>"#,
                    fg.r,
                    fg.g,
                    fg.b,
                    html_escape::encode_text(text)
                );
            }

            output.push_str("</td></tr>\n");
        }

        output.push_str("</tbody></table>\n");
    } else {
        // Simple pre/code block without line numbers
        output.push_str("<pre><code");
        let language_label = language.to_string();
        if !language_label.is_empty() {
            output.push_str(&format!(
                r#" class="language-{}""#,
                html_escape::encode_text(&language_label)
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
                let _ = write!(
                    output,
                    r#"<span style="color: #{:02x}{:02x}{:02x};">{}</span>"#,
                    fg.r,
                    fg.g,
                    fg.b,
                    html_escape::encode_text(text)
                );
            }
        }

        output.push_str("</code></pre>\n");
    }

    output.push_str("</div>\n");

    Ok(output)
}

/// Derives the effective color mode of a code panel from its resolved theme
/// background, by perceived luminance.
///
/// Code blocks resolve their theme *variant* via [`CodeBlockMode`](crate::markdown::highlighting::CodeBlockMode)
/// (default `Inverse`, i.e. the opposite of the page mode for contrast), and a
/// few theme names are single-variant and do not invert. Downstream contrast
/// decisions — the header pill's text color and the
/// highlighted-line background math — must therefore key off the panel's
/// **actual** background, not the requested mode, so a single-variant dark theme
/// still gets light header text (and vice versa).
///
/// Returns [`ColorMode::Light`] when the background is perceptually light
/// (Rec. 601 luma > 127), otherwise [`ColorMode::Dark`].
#[inline]
pub(crate) fn mode_for_background(bg: Color) -> ColorMode {
    let luma = 0.299 * f32::from(bg.r) + 0.587 * f32::from(bg.g) + 0.114 * f32::from(bg.b);
    if luma > 127.0 {
        ColorMode::Light
    } else {
        ColorMode::Dark
    }
}

/// Computes a highlighted background color based on the theme background and color mode.
///
/// Uses warmer tones (more red/green) to create a visual highlight effect.
#[inline]
pub(crate) fn compute_highlight_bg(theme_bg: Color, color_mode: ColorMode) -> Color {
    use crate::markdown::output::terminal::adjust_background;
    adjust_background(theme_bg, color_mode, (30, 25, 0), (20, 15, 0))
}

/// Emits a padding row with the specified background color.
///
/// The padding row consists of:
/// - Setting the background color
/// - Either painting exactly `target_width` spaces, or clearing to end of line
///   (`\x1b[K`) when no width constraint applies
/// - Resetting all attributes (\x1b[0m)
/// - Adding a newline
#[inline]
pub(crate) fn emit_padding_row(bg_color: Color, target_width: Option<u16>) -> String {
    match target_width {
        Some(w) => format!(
            "\x1b[0m\x1b[48;2;{};{};{}m{}\x1b[0m\n",
            bg_color.r,
            bg_color.g,
            bg_color.b,
            " ".repeat(w as usize)
        ),
        None => format!(
            "\x1b[0m\x1b[48;2;{};{};{}m\x1b[K\x1b[0m\n",
            bg_color.r, bg_color.g, bg_color.b
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::output::terminal::{
        ColorDepth, DimMode, HyperlinkMode, ItalicMode, MermaidMode, TerminalImageMode,
        TerminalOptions,
    };
    use crate::testing::strip_ansi_codes;

    fn test_options() -> TerminalOptions {
        TerminalOptions {
            code_theme: crate::markdown::highlighting::ThemePair::OneHalf,
            prose_theme: crate::markdown::highlighting::ThemePair::OneHalf,
            color_mode: ColorMode::Dark,
            include_line_numbers: false,
            color_depth: Some(ColorDepth::TrueColor),
            image_mode: TerminalImageMode::Never,
            base_path: None,
            italic_mode: ItalicMode::Always,
            dim_mode: DimMode::Always,
            max_width: Some(80),
            mermaid_mode: MermaidMode::Off,
            hyperlink_mode: HyperlinkMode::Always,
            hr_defaults: None,
            code_block_mode: crate::markdown::highlighting::CodeBlockMode::default(),
        }
    }

    #[test]
    fn test_render_terminal_code_block_basic() {
        let highlighter = CodeHighlighter::new(
            crate::markdown::highlighting::ThemePair::Github,
            ColorMode::Dark,
        );
        let options = test_options();
        let meta = CodeBlockMeta::default();

        let code = "fn main() {}";
        let result = render_terminal_code_block(
            code,
            &LanguageGrammar::rust(),
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

        assert!(result.is_ok());
        let output = result.unwrap();

        // Should contain ANSI escape codes
        assert!(output.contains("\x1b["));
    }

    #[test]
    fn test_render_terminal_code_block_yaml() {
        let highlighter = CodeHighlighter::new(
            crate::markdown::highlighting::ThemePair::Github,
            ColorMode::Dark,
        );
        let options = test_options();
        let meta = CodeBlockMeta::default();

        let code = "foo: 1\nbar: 2";
        let result = render_terminal_code_block(
            code,
            &LanguageGrammar::yaml(),
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        let plain = strip_ansi_codes(&output);

        // Should contain the YAML content
        assert!(plain.contains("foo: 1"));
        assert!(plain.contains("bar: 2"));
    }

    #[test]
    fn test_render_terminal_code_block_light_mode() {
        let highlighter = CodeHighlighter::new(
            crate::markdown::highlighting::ThemePair::Github,
            ColorMode::Light,
        );
        let options = test_options();
        let meta = CodeBlockMeta::default();

        let code = "test: value";
        let result = render_terminal_code_block(
            code,
            &LanguageGrammar::yaml(),
            &highlighter,
            &options,
            &meta,
            ColorMode::Light,
            None,
            None,
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("\x1b["));
    }

    #[test]
    fn test_render_terminal_code_block_dark_mode() {
        let highlighter = CodeHighlighter::new(
            crate::markdown::highlighting::ThemePair::Github,
            ColorMode::Dark,
        );
        let options = test_options();
        let meta = CodeBlockMeta::default();

        let code = "test: value";
        let result = render_terminal_code_block(
            code,
            &LanguageGrammar::yaml(),
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("\x1b["));
    }

    #[test]
    fn test_render_terminal_code_block_clears_inherited_text_attributes() {
        let highlighter = CodeHighlighter::new(
            crate::markdown::highlighting::ThemePair::OneHalf,
            ColorMode::Light,
        );
        let options = test_options();
        let meta = CodeBlockMeta::default();

        let output = render_terminal_code_block(
            "foo: string\n",
            &LanguageGrammar::yaml(),
            &highlighter,
            &options,
            &meta,
            ColorMode::Light,
            Some(40),
            None,
        )
        .expect("render code block");

        assert!(
            output.contains("\x1b[0m\x1b[48;2;"),
            "code rows must reset inherited SGR state before applying themed code colors: {output:?}"
        );
    }

    #[test]
    fn test_render_html_code_block_yaml() {
        let highlighter = CodeHighlighter::new(
            crate::markdown::highlighting::ThemePair::Github,
            ColorMode::Dark,
        );
        let options = HtmlOptions::default();
        let meta = CodeBlockMeta::default();

        let code = "foo: 1\nbar: <script>";
        let result = render_html_code_block(code, &LanguageGrammar::yaml(), &meta, &highlighter, &options);

        assert!(result.is_ok());
        let html = result.unwrap();

        // Should contain language class
        assert!(html.contains("language-yaml"));
        // Should escape HTML in scalar content
        assert!(html.contains("&lt;script&gt;") || html.contains("&#60;script&#62;"));
    }

    #[test]
    fn test_compute_highlight_bg_dark() {
        let theme_bg = Color {
            r: 40,
            g: 40,
            b: 40,
            a: 255,
        };
        let highlight_bg = compute_highlight_bg(theme_bg, ColorMode::Dark);
        assert_eq!(highlight_bg.r, 70); // 40 + 30
        assert_eq!(highlight_bg.g, 65); // 40 + 25
        assert_eq!(highlight_bg.b, 40); // unchanged
    }

    #[test]
    fn test_compute_highlight_bg_light() {
        let theme_bg = Color {
            r: 240,
            g: 240,
            b: 240,
            a: 255,
        };
        let highlight_bg = compute_highlight_bg(theme_bg, ColorMode::Light);
        assert_eq!(highlight_bg.r, 220); // 240 - 20
        assert_eq!(highlight_bg.g, 225); // 240 - 15
        assert_eq!(highlight_bg.b, 240); // unchanged
    }

    #[test]
    fn test_emit_padding_row() {
        let bg = Color {
            r: 50,
            g: 60,
            b: 70,
            a: 255,
        };
        let row = emit_padding_row(bg, None);
        assert!(row.contains("\x1b[48;2;50;60;70m"));
        assert!(row.contains("\x1b[K"));
        assert!(row.contains("\x1b[0m"));
        assert!(row.ends_with('\n'));
    }

    #[test]
    fn test_emit_padding_row_with_target_width() {
        let bg = Color {
            r: 50,
            g: 60,
            b: 70,
            a: 255,
        };
        let row = emit_padding_row(bg, Some(20));
        assert!(row.contains("\x1b[48;2;50;60;70m"));
        // Should not use clear-to-EOL when constrained.
        assert!(!row.contains("\x1b[K"));
        // Should contain exactly 20 spaces.
        assert!(row.contains(&" ".repeat(20)));
        assert!(row.contains("\x1b[0m"));
        assert!(row.ends_with('\n'));
    }

    #[test]
    fn test_render_terminal_code_block_with_target_width_pads_spaces() {
        let highlighter = CodeHighlighter::new(
            crate::markdown::highlighting::ThemePair::Github,
            ColorMode::Dark,
        );
        let options = test_options();
        let meta = CodeBlockMeta::default();

        let code = "ok";
        let result = render_terminal_code_block(
            code,
            &LanguageGrammar::rust(),
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            Some(20),
            None,
        )
        .unwrap();

        // Constrained output should not rely on \x1b[K (clear-to-EOL).
        assert!(!result.contains("\x1b[K"));
        let plain = strip_ansi_codes(&result);
        // Each non-empty line should be exactly 20 visible columns wide.
        for line in plain.lines().filter(|l| !l.is_empty()) {
            assert_eq!(
                line.chars().count(),
                20,
                "expected exactly 20 visible chars on line: {line:?}"
            );
        }
    }
}
