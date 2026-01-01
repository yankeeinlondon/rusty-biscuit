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
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style};
use syntect::parsing::{Scope, SyntaxReference};

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

    // Enable table parsing extension
    let parser = Parser::new_ext(md.content(), Options::ENABLE_TABLES);
    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut code_language = String::new();
    let mut code_info_string = String::new();

    // List tracking
    let mut list_stack: Vec<Option<u64>> = Vec::new(); // None = unordered, Some(start) = ordered

    // Table tracking - buffer entire table for proper rendering
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new(); // All rows including header
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();

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
                // Add blank line before heading (unless at start of output)
                if !output.is_empty() && !output.ends_with("\n\n") {
                    if output.ends_with('\n') {
                        output.push('\n');
                    } else {
                        output.push_str("\n\n");
                    }
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
                output.push_str("\n\n"); // Blank line after heading
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

            // List handling
            Event::Start(Tag::List(start_num)) => {
                // When a nested list starts inside an item, add newline to separate from parent text
                if !list_stack.is_empty() && !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                list_stack.push(start_num);
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                // Add blank line after top-level list ends
                if list_stack.is_empty() {
                    output.push('\n');
                }
            }
            Event::Start(Tag::Item) => {
                // Calculate indentation based on nesting level
                let indent = "  ".repeat(list_stack.len().saturating_sub(1));

                // Get the marker for this item
                if let Some(list_type) = list_stack.last_mut() {
                    match list_type {
                        Some(num) => {
                            // Ordered list: emit number and increment
                            let style = prose_highlighter.base_style();
                            output.push_str(&emit_prose_text(&format!("{}{}. ", indent, num), style));
                            *num += 1;
                        }
                        None => {
                            // Unordered list: emit bullet
                            let style = prose_highlighter.base_style();
                            output.push_str(&emit_prose_text(&format!("{}- ", indent), style));
                        }
                    }
                }
            }
            Event::End(TagEnd::Item) => {
                output.push('\n');
            }

            Event::Start(Tag::Paragraph) => {
                // Don't add extra spacing inside list items
            }
            Event::End(TagEnd::Paragraph) => {
                // Only add double newline for paragraphs outside of lists
                if list_stack.is_empty() {
                    output.push_str("\n\n");
                }
            }

            Event::Text(text) if !in_code_block => {
                if in_table {
                    // Buffer text for table cell
                    current_cell.push_str(&text);
                } else {
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
            }

            Event::Code(code) => {
                if in_table {
                    // Buffer inline code for table cell (mark with special prefix for styling later)
                    current_cell.push_str(&format!("\x00CODE\x00{}\x00/CODE\x00", code));
                } else {
                    // Inline code with styling (no backticks in terminal output)
                    let style = prose_highlighter.style_for_inline_code(&scope_stack);
                    // Use background color from the style if available
                    output.push_str(&emit_inline_code(&code, style));
                }
            }

            Event::SoftBreak => {
                output.push(' ');
            }
            Event::HardBreak => {
                output.push('\n');
            }

            // Table handling - buffer entire table for proper rendering
            Event::Start(Tag::Table(_alignments)) => {
                in_table = true;
                table_rows.clear();
                // Add blank line before table if needed
                if !output.is_empty() && !output.ends_with("\n\n") {
                    if output.ends_with('\n') {
                        output.push('\n');
                    } else {
                        output.push_str("\n\n");
                    }
                }
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
                // Render the buffered table with proper formatting
                output.push_str(&render_table(&table_rows, &prose_highlighter, &scope_stack));
                table_rows.clear();
            }
            Event::Start(Tag::TableHead) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                table_rows.push(current_row.clone());
                current_row.clear();
            }
            Event::Start(Tag::TableRow) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                table_rows.push(current_row.clone());
                current_row.clear();
            }
            Event::Start(Tag::TableCell) => {
                current_cell.clear();
            }
            Event::End(TagEnd::TableCell) => {
                current_row.push(current_cell.clone());
                current_cell.clear();
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

/// Emits inline code with both foreground and background colors.
///
/// Uses the style's background color if available, otherwise uses a subtle gray.
fn emit_inline_code(text: &str, style: Style) -> String {
    let fg = style.foreground;
    let bg = style.background;

    // Check if background is meaningful (not transparent/zero alpha)
    if bg.a > 0 && (bg.r > 0 || bg.g > 0 || bg.b > 0) {
        // Use provided background
        format!(
            "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m{}\x1b[0m",
            bg.r, bg.g, bg.b, fg.r, fg.g, fg.b, text
        )
    } else {
        // Use a subtle dark gray background for contrast
        format!(
            "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m{}\x1b[0m",
            50, 50, 55, fg.r, fg.g, fg.b, text
        )
    }
}

/// Renders a buffered table with box-drawing characters and alternating row colors.
///
/// ## Arguments
///
/// * `rows` - Vector of rows, where each row is a vector of cell strings.
///            First row is treated as header.
/// * `prose_highlighter` - For styling text
/// * `scope_stack` - Current scope stack for styling
fn render_table(
    rows: &[Vec<String>],
    prose_highlighter: &ProseHighlighter,
    scope_stack: &[Scope],
) -> String {
    if rows.is_empty() {
        return String::new();
    }

    // Calculate column widths (max width of each column across all rows)
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut col_widths: Vec<usize> = vec![0; col_count];

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            // Strip our inline code markers for width calculation
            let plain = cell
                .replace("\x00CODE\x00", "")
                .replace("\x00/CODE\x00", "");
            col_widths[i] = col_widths[i].max(plain.chars().count());
        }
    }

    // Ensure minimum column width
    for w in &mut col_widths {
        *w = (*w).max(3);
    }

    let mut output = String::new();

    // Box-drawing characters
    let top_left = '┌';
    let top_right = '┐';
    let bottom_left = '└';
    let bottom_right = '┘';
    let horizontal = '─';
    let vertical = '│';
    let top_tee = '┬';
    let bottom_tee = '┴';
    let left_tee = '├';
    let right_tee = '┤';
    let cross = '┼';

    // Colors - use explicit backgrounds for all rows to ensure consistent rendering
    let border_color = "\x1b[38;2;100;100;110m"; // Gray border
    let header_bg = "\x1b[48;2;60;63;70m"; // Darker header background
    let even_row_bg = "\x1b[48;2;45;48;55m"; // Subtle background for even rows
    let odd_row_bg = "\x1b[48;2;30;33;40m"; // Darker background for odd rows (explicit, not default)
    let reset = "\x1b[0m";

    // Get text style from prose highlighter (used in render_cell_content)
    let _text_style = prose_highlighter.base_style();

    // Top border: ┌───┬───┬───┐
    output.push_str(border_color);
    output.push(top_left);
    for (i, &width) in col_widths.iter().enumerate() {
        for _ in 0..width + 2 {
            output.push(horizontal);
        }
        if i < col_widths.len() - 1 {
            output.push(top_tee);
        }
    }
    output.push(top_right);
    output.push_str(reset);
    output.push('\n');

    // Render rows
    for (row_idx, row) in rows.iter().enumerate() {
        let is_header = row_idx == 0;
        let bg = if is_header {
            header_bg
        } else if row_idx % 2 == 0 {
            even_row_bg
        } else {
            odd_row_bg
        };

        // Row content: │ cell │ cell │ cell │
        // Combine bg + border color for dividers to ensure both are set
        let divider_style = format!("{}{}", bg, border_color);

        // First divider
        output.push_str(&divider_style);
        output.push(vertical);

        for (col_idx, cell) in row.iter().enumerate() {
            let width = col_widths.get(col_idx).copied().unwrap_or(3);

            // Leading space with row background
            output.push_str(bg);
            output.push(' ');

            // Render cell content with styling (pass row background for restoration)
            let rendered = render_cell_content(cell, prose_highlighter, scope_stack, is_header, bg);
            output.push_str(&rendered);

            // Calculate plain text length for padding
            let plain_len = cell
                .replace("\x00CODE\x00", "")
                .replace("\x00/CODE\x00", "")
                .chars()
                .count();

            // Pad to column width with row background
            output.push_str(bg);
            for _ in plain_len..width {
                output.push(' ');
            }
            output.push(' ');

            // Column divider with row background + border color
            output.push_str(&divider_style);
            output.push(vertical);
        }

        // Handle missing cells (pad with empty)
        for col_idx in row.len()..col_count {
            let width = col_widths.get(col_idx).copied().unwrap_or(3);
            output.push_str(bg);
            for _ in 0..width + 2 {
                output.push(' ');
            }
            output.push_str(&divider_style);
            output.push(vertical);
        }

        // Reset at end of row
        output.push_str(reset);

        output.push('\n');

        // After header row, draw separator: ├───┼───┼───┤
        if is_header {
            output.push_str(border_color);
            output.push(left_tee);
            for (i, &width) in col_widths.iter().enumerate() {
                for _ in 0..width + 2 {
                    output.push(horizontal);
                }
                if i < col_widths.len() - 1 {
                    output.push(cross);
                }
            }
            output.push(right_tee);
            output.push_str(reset);
            output.push('\n');
        }
    }

    // Bottom border: └───┴───┴───┘
    output.push_str(border_color);
    output.push(bottom_left);
    for (i, &width) in col_widths.iter().enumerate() {
        for _ in 0..width + 2 {
            output.push(horizontal);
        }
        if i < col_widths.len() - 1 {
            output.push(bottom_tee);
        }
    }
    output.push(bottom_right);
    output.push_str(reset);
    output.push('\n');

    output
}

/// Renders cell content with proper styling for inline code.
///
/// The `row_bg` parameter is the ANSI escape sequence for the row background color,
/// which must be restored after inline code styling to maintain consistent row coloring.
fn render_cell_content(
    cell: &str,
    prose_highlighter: &ProseHighlighter,
    scope_stack: &[Scope],
    is_header: bool,
    row_bg: &str,
) -> String {
    let mut output = String::new();
    let text_style = prose_highlighter.base_style();

    // Bold for headers
    if is_header {
        output.push_str("\x1b[1m");
    }

    const CODE_START: &str = "\x00CODE\x00";
    const CODE_END: &str = "\x00/CODE\x00";

    // Process cell content, handling inline code markers
    let mut pos = 0;

    while pos < cell.len() {
        if let Some(start_offset) = cell[pos..].find(CODE_START) {
            let start = pos + start_offset;

            // Text before code marker
            if start > pos {
                let before = &cell[pos..start];
                output.push_str(&format!(
                    "\x1b[38;2;{};{};{}m{}",
                    text_style.foreground.r, text_style.foreground.g, text_style.foreground.b,
                    before
                ));
            }

            // Find end marker
            let after_start = start + CODE_START.len();
            if let Some(end_offset) = cell[after_start..].find(CODE_END) {
                let code = &cell[after_start..after_start + end_offset];
                // Render inline code with distinct styling
                let code_style = prose_highlighter.style_for_inline_code(scope_stack);
                output.push_str(&emit_inline_code_in_table(code, code_style, row_bg, is_header));
                pos = after_start + end_offset + CODE_END.len();
            } else {
                // No end marker found, treat rest as plain text
                pos = cell.len();
            }
        } else {
            // No more code markers, output remaining text
            let remaining = &cell[pos..];
            if !remaining.is_empty() {
                output.push_str(&format!(
                    "\x1b[38;2;{};{};{}m{}",
                    text_style.foreground.r, text_style.foreground.g, text_style.foreground.b,
                    remaining
                ));
            }
            break;
        }
    }

    if is_header {
        output.push_str("\x1b[22m"); // Reset bold
    }

    output
}

/// Emits inline code within a table cell, restoring row background after.
///
/// Unlike regular `emit_inline_code`, this version restores the row background
/// and header bold state instead of doing a full reset, ensuring consistent
/// row coloring throughout the table.
fn emit_inline_code_in_table(text: &str, style: Style, row_bg: &str, is_header: bool) -> String {
    let fg = style.foreground;
    let bg = style.background;

    // Determine the code background color
    let code_bg = if bg.a > 0 && (bg.r > 0 || bg.g > 0 || bg.b > 0) {
        format!("\x1b[48;2;{};{};{}m", bg.r, bg.g, bg.b)
    } else {
        // Use a subtle dark gray background for contrast
        "\x1b[48;2;50;50;55m".to_string()
    };

    // Build the restore sequence: row background + bold if header
    let restore = if is_header {
        format!("{}\x1b[1m", row_bg)
    } else {
        row_bg.to_string()
    };

    format!(
        "{}\x1b[38;2;{};{};{}m{}{}",
        code_bg, fg.r, fg.g, fg.b, text, restore
    )
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

    // Use LinesWithEndings to preserve newlines - required for proper multi-line
    // syntax parsing in grammars like bash/shell that track state across lines
    use syntect::util::LinesWithEndings;
    let lines: Vec<&str> = LinesWithEndings::from(code).collect();
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

        // Convert styled ranges to ANSI escape codes, but filter out newlines
        // from the output (we handle line breaks ourselves)
        for (style, text) in &ranges {
            let text_without_newline = text.trim_end_matches('\n');
            if !text_without_newline.is_empty() {
                output.push_str(&format!(
                    "\x1b[38;2;{};{};{}m{}",
                    style.foreground.r, style.foreground.g, style.foreground.b,
                    text_without_newline
                ));
            }
        }

        // Clear to end of line with background color, then reset
        // \x1b[K clears from cursor to end of line using current background
        output.push_str("\x1b[K\x1b[0m");

        // Add newline except for last line
        if idx < lines.len() - 1 {
            output.push('\n');
        }
    }

    Ok(output)
}

/// Finds syntax definition by language identifier.
///
/// Searches in the following order:
/// 1. By file extension (e.g., "rs", "py", "js")
/// 2. By exact name (e.g., "Rust", "Python")
/// 3. By case-insensitive name match (e.g., "rust" -> "Rust")
/// 4. By common alias mapping (e.g., "shell" -> "bash", "c++" -> "cpp")
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

    // Try by exact name
    if let Some(syntax) = syntax_set.find_syntax_by_name(language) {
        return Some(syntax);
    }

    // Try case-insensitive name match
    let language_lower = language.to_lowercase();
    for syntax in syntax_set.syntaxes() {
        if syntax.name.to_lowercase() == language_lower {
            return Some(syntax);
        }
    }

    // Try common aliases that differ from extension/name
    let alias = match language_lower.as_str() {
        "shell" | "zsh" => "bash",
        "c++" => "cpp",
        "dockerfile" => "Dockerfile",
        "makefile" | "make" => "Makefile",
        "javascript" => "js",
        "typescript" => "ts",
        "python3" => "py",
        _ => return None,
    };

    // Try alias as extension first, then as name
    syntax_set
        .find_syntax_by_extension(alias)
        .or_else(|| syntax_set.find_syntax_by_name(alias))
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
        // Inline code should NOT have backticks in terminal output
        assert!(plain.contains("cargo build"));
        assert!(!plain.contains("`cargo build`"), "Backticks should be removed in terminal output");
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

        // Inline code SHOULD have background color for contrast
        let bg_count = output.matches("\x1b[48;2;").count();
        assert!(bg_count > 0, "Inline code should have background color for contrast");

        let plain = strip_ansi_codes(&output);
        // No backticks in terminal output
        assert!(plain.contains("cargo build"));
        assert!(!plain.contains("`"), "Backticks should be removed in terminal output");
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

    #[test]
    fn test_for_terminal_heading_has_blank_line_after() {
        let md: Markdown = "# Heading\n\nParagraph text.".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Heading should be followed by blank line before paragraph
        assert!(plain.contains("# Heading\n\nParagraph text."));
    }

    #[test]
    fn test_for_terminal_heading_has_blank_line_before() {
        let md: Markdown = "Some text.\n\n## Heading".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Should have blank line between paragraph and heading
        assert!(plain.contains("Some text.\n\n## Heading"));
    }

    #[test]
    fn test_for_terminal_heading_after_list_has_blank_line() {
        let md: Markdown = "- Item one\n- Item two\n\n### Heading".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Should have blank line between list and heading
        assert!(plain.contains("- Item two\n\n### Heading"));
    }

    #[test]
    fn test_for_terminal_first_heading_no_leading_blank() {
        let md: Markdown = "# First Heading".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // First heading should not have leading blank lines
        assert!(plain.starts_with("# First Heading"));
    }

    #[test]
    fn test_for_terminal_unordered_list() {
        let md: Markdown = "- First item\n- Second item\n- Third item".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Each item should be on its own line with bullet marker
        assert!(plain.contains("- First item\n"));
        assert!(plain.contains("- Second item\n"));
        assert!(plain.contains("- Third item\n"));
    }

    #[test]
    fn test_for_terminal_ordered_list() {
        let md: Markdown = "1. First item\n2. Second item\n3. Third item".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Each item should be on its own line with number marker
        assert!(plain.contains("1. First item\n"));
        assert!(plain.contains("2. Second item\n"));
        assert!(plain.contains("3. Third item\n"));
    }

    #[test]
    fn test_for_terminal_nested_list() {
        let md: Markdown = "- Parent item\n  - Child item\n  - Another child\n- Second parent".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Parent item should have newline before nested list starts
        assert!(plain.contains("- Parent item\n"), "Parent should end with newline, got:\n{}", plain);
        assert!(plain.contains("  - Child item\n"));
        assert!(plain.contains("  - Another child\n"));
        assert!(plain.contains("- Second parent\n"));
    }

    #[test]
    fn test_for_terminal_list_with_inline_code() {
        let md: Markdown = "- Use `cargo build`\n- Run `cargo test`".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // List items should be separate lines
        assert!(plain.contains("- Use cargo build\n"));
        assert!(plain.contains("- Run cargo test\n"));
    }

    // ==================== Regression Tests ====================

    /// Regression test: nested lists must render each item on its own line
    /// Bug: nested list items were concatenated horizontally instead of vertically
    #[test]
    fn test_nested_list_items_on_separate_lines() {
        let md: Markdown = "- Parent\n  - Child\n  - Child2".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Each item must be on its own line - not concatenated like "- Parent  - Child"
        assert!(
            !plain.contains("Parent  - Child"),
            "Nested list items should not be concatenated, got:\n{}",
            plain
        );
        assert!(
            plain.contains("- Parent\n"),
            "Parent item should have newline before nested list, got:\n{}",
            plain
        );
        assert!(plain.contains("  - Child\n"));
        assert!(plain.contains("  - Child2\n"));
    }

    /// Regression test: deeply nested lists with inline code render correctly
    /// Bug: lists like "- Use ### (H3)\n  - Example:\n    - `## Env`" rendered on one line
    #[test]
    fn test_deeply_nested_list_with_inline_code() {
        let md: Markdown = r#"- Use ### Heading (H3) only for subsections
  - Example:
    - `## Environment Variables`
    - `### Priority Order`
    - `### Fallback Behavior`"#.into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Each level should be on its own line with proper indentation
        assert!(
            plain.contains("- Use ### Heading (H3) only for subsections\n"),
            "Top level item should end with newline, got:\n{}",
            plain
        );
        assert!(
            plain.contains("  - Example:\n"),
            "Second level should have 2-space indent, got:\n{}",
            plain
        );
        assert!(
            plain.contains("    - ## Environment Variables\n"),
            "Third level should have 4-space indent, got:\n{}",
            plain
        );
        // Items should NOT be concatenated horizontally
        assert!(
            !plain.contains("subsections  - Example"),
            "Items should not be on same line, got:\n{}",
            plain
        );
    }

    /// Regression test: triple-nested lists render with correct indentation
    #[test]
    fn test_triple_nested_list_indentation() {
        let md: Markdown = "- Level 1\n  - Level 2\n    - Level 3\n    - Level 3b\n  - Level 2b\n- Level 1b".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Verify each level has correct indentation and newlines
        assert!(plain.contains("- Level 1\n"), "Level 1 should end with newline");
        assert!(plain.contains("  - Level 2\n"), "Level 2 should have 2-space indent");
        assert!(plain.contains("    - Level 3\n"), "Level 3 should have 4-space indent");
        assert!(plain.contains("    - Level 3b\n"), "Level 3b should have 4-space indent");
        assert!(plain.contains("  - Level 2b\n"), "Level 2b should return to 2-space indent");
        assert!(plain.contains("- Level 1b\n"), "Level 1b should return to no indent");
    }

    // ==================== Table Regression Tests ====================

    /// Regression test: tables must render with rows on separate lines
    /// Bug: table rows were concatenated horizontally instead of vertically
    #[test]
    fn test_table_rows_on_separate_lines() {
        let md: Markdown = "| A | B |\n|---|---|\n| 1 | 2 |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Table should have box-drawing characters
        assert!(
            plain.contains("┌") && plain.contains("┐"),
            "Table should have top border, got:\n{}",
            plain
        );
        assert!(
            plain.contains("└") && plain.contains("┘"),
            "Table should have bottom border, got:\n{}",
            plain
        );
        // Header and data should be present
        assert!(plain.contains("A"), "Header cell A missing");
        assert!(plain.contains("B"), "Header cell B missing");
        assert!(plain.contains("1"), "Data cell 1 missing");
        assert!(plain.contains("2"), "Data cell 2 missing");
        // Multiple lines (not concatenated)
        assert!(
            plain.lines().count() >= 5,
            "Table should have multiple lines (top border, header, separator, data, bottom border), got:\n{}",
            plain
        );
    }

    /// Regression test: tables with multiple rows render correctly
    #[test]
    fn test_table_multiple_rows() {
        let md: Markdown = "| Col1 | Col2 |\n|------|------|\n| A | B |\n| C | D |\n| E | F |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Table should have box-drawing structure
        assert!(plain.contains("┌"), "Should have top-left corner");
        assert!(plain.contains("├"), "Should have header separator");
        assert!(plain.contains("└"), "Should have bottom-left corner");

        // Verify content is present
        assert!(plain.contains("Col1"), "Header Col1 missing");
        assert!(plain.contains("Col2"), "Header Col2 missing");
        assert!(plain.contains(" A "), "Data A missing");
        assert!(plain.contains(" E "), "Data E missing");

        // Should have multiple data rows (at least 7 lines: top, header, sep, 3 data, bottom)
        let line_count = plain.lines().count();
        assert!(
            line_count >= 7,
            "Should have at least 7 lines, got {} in:\n{}",
            line_count, plain
        );
    }

    /// Regression test: tables with inline code in cells render correctly
    #[test]
    fn test_table_with_inline_code() {
        let md: Markdown = "| Variable | Description |\n|----------|-------------|\n| `FOO` | A variable |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Inline code should be present (without backticks in terminal)
        assert!(
            plain.contains("FOO"),
            "Inline code content should be present, got:\n{}",
            plain
        );
        // Table should have box-drawing borders
        assert!(
            plain.contains("┌") && plain.contains("┘"),
            "Table should have box-drawing borders, got:\n{}",
            plain
        );
    }

    /// Regression test: table after heading has proper spacing
    #[test]
    fn test_table_after_heading_spacing() {
        let md: Markdown = "## Environment Variables\n\n| Var | Desc |\n|-----|------|\n| A | B |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Should have heading followed by table with box-drawing
        assert!(
            plain.contains("## Environment Variables"),
            "Heading should be present, got:\n{}",
            plain
        );
        assert!(
            plain.contains("┌"),
            "Table should have box-drawing borders after heading, got:\n{}",
            plain
        );
        // Table should be separated from heading
        assert!(
            plain.contains("Variables\n\n┌"),
            "Table should have blank line after heading, got:\n{}",
            plain
        );
    }

    /// Test that tables have alternating row backgrounds (via ANSI codes)
    #[test]
    fn test_table_alternating_rows() {
        let md: Markdown = "| A |\n|---|\n| 1 |\n| 2 |\n| 3 |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Should contain background color codes for alternating rows
        // Header has one bg, even rows another, odd rows no bg
        let bg_count = output.matches("\x1b[48;2;").count();
        assert!(
            bg_count >= 2,
            "Should have background colors for header and alternating rows, got {} occurrences",
            bg_count
        );
    }

    /// Test that table headers are bold
    #[test]
    fn test_table_header_bold() {
        let md: Markdown = "| Header |\n|--------|\n| Data |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Should contain bold code (\x1b[1m) for header
        assert!(
            output.contains("\x1b[1m"),
            "Header should be bold, output:\n{}",
            output
        );
    }

    /// Regression test: row background persists after inline code styling.
    ///
    /// This bug caused row backgrounds to reset to terminal default after
    /// inline code because `emit_inline_code` used `\x1b[0m` (full reset).
    /// The fix uses `emit_inline_code_in_table` which restores the row background.
    #[test]
    fn test_table_row_background_persists_after_inline_code() {
        let md: Markdown =
            "| Field | Description |\n|-------|-------------|\n| `foo` | A var |\n| `bar` | Another |"
                .into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // The row background colors
        let even_row_bg = "\x1b[48;2;45;48;55m"; // Used for row index 2 (even)

        // Split output into lines and check body rows
        let lines: Vec<&str> = output.lines().collect();
        // Line 0: top border
        // Line 1: header row
        // Line 2: header separator
        // Line 3: first body row (row_idx=1, odd - no bg)
        // Line 4: second body row (row_idx=2, even - has bg)

        // The even row (second body row) should have background color that persists
        // after inline code. Check that the bg appears AFTER inline code content.
        let even_row = lines.get(4).unwrap_or(&"");
        let bar_pos = even_row.find("bar");
        let bg_after_bar = even_row.rfind(even_row_bg);

        assert!(
            bar_pos.is_some() && bg_after_bar.is_some() && bg_after_bar > bar_pos,
            "Row background should be restored after inline code 'bar', got row:\n{}",
            even_row
        );

        // Also verify we're NOT using full reset (\x1b[0m) right after inline code text
        // within the table cell content. The pattern `bar\x1b[0m` would indicate the bug.
        // Instead, we should see the row background restored.
        assert!(
            !even_row.contains("bar\x1b[0m "),
            "Should not have full reset immediately after inline code, got:\n{}",
            even_row
        );
    }

    // ==================== Code Block Syntax Highlighting Regression Tests ====================

    /// Regression test: find_syntax must handle case-insensitive language names.
    ///
    /// Bug: `find_syntax("rust", ...)` returned None because it only tried
    /// extension match ("rust" is not an extension) and exact name match
    /// ("rust" != "Rust"). This caused code blocks with `\`\`\`rust` to fall back
    /// to plain text with no syntax highlighting.
    #[test]
    fn test_find_syntax_case_insensitive() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);

        // These should all find the Rust syntax
        assert!(find_syntax("rust", highlighter.syntax_set()).is_some(), "lowercase 'rust' should find Rust syntax");
        assert!(find_syntax("Rust", highlighter.syntax_set()).is_some(), "exact 'Rust' should find Rust syntax");
        assert!(find_syntax("RUST", highlighter.syntax_set()).is_some(), "uppercase 'RUST' should find Rust syntax");
        assert!(find_syntax("rs", highlighter.syntax_set()).is_some(), "extension 'rs' should find Rust syntax");

        // Python
        assert!(find_syntax("python", highlighter.syntax_set()).is_some(), "lowercase 'python' should find Python syntax");
        assert!(find_syntax("Python", highlighter.syntax_set()).is_some(), "exact 'Python' should find Python syntax");
        assert!(find_syntax("py", highlighter.syntax_set()).is_some(), "extension 'py' should find Python syntax");
    }

    /// Regression test: find_syntax must handle common aliases.
    ///
    /// Bug: Common language aliases like "shell" weren't mapped to their
    /// actual syntax definitions like "bash".
    #[test]
    fn test_find_syntax_aliases() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);

        // Bash aliases
        assert!(find_syntax("bash", highlighter.syntax_set()).is_some(), "'bash' should find Bash syntax");
        assert!(find_syntax("sh", highlighter.syntax_set()).is_some(), "'sh' should find Bash syntax");
        assert!(find_syntax("shell", highlighter.syntax_set()).is_some(), "'shell' alias should find Bash syntax");

        // JavaScript/TypeScript
        assert!(find_syntax("js", highlighter.syntax_set()).is_some(), "'js' should find JavaScript syntax");
        assert!(find_syntax("javascript", highlighter.syntax_set()).is_some(), "'javascript' alias should find JS syntax");
        assert!(find_syntax("ts", highlighter.syntax_set()).is_some(), "'ts' should find TypeScript syntax");
        assert!(find_syntax("typescript", highlighter.syntax_set()).is_some(), "'typescript' alias should find TS syntax");
    }

    /// Regression test: code blocks must have syntax highlighting with multiple colors.
    ///
    /// Bug: Even when the syntax was found, all tokens had the same foreground color
    /// because the highlighting wasn't being applied correctly.
    #[test]
    fn test_code_block_has_multiple_colors() {
        let md: Markdown = "```rust\nfn main() {\n    let x = 5;\n}\n```".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Count distinct foreground colors in the output
        // Foreground colors have format: \x1b[38;2;R;G;Bm
        let mut colors = std::collections::HashSet::new();
        let mut i = 0;
        let bytes = output.as_bytes();

        while i < bytes.len() {
            // Look for foreground color escape sequence
            if i + 7 < bytes.len()
                && bytes[i] == 0x1b
                && bytes[i + 1] == b'['
                && bytes[i + 2] == b'3'
                && bytes[i + 3] == b'8'
                && bytes[i + 4] == b';'
                && bytes[i + 5] == b'2'
                && bytes[i + 6] == b';'
            {
                // Extract the color values until 'm'
                let start = i + 7;
                let mut end = start;
                while end < bytes.len() && bytes[end] != b'm' {
                    end += 1;
                }
                if end < bytes.len() {
                    let color_str = &output[start..end];
                    colors.insert(color_str.to_string());
                }
                i = end;
            }
            i += 1;
        }

        // Rust code with fn, let, numbers should have at least 3 different colors
        // (keywords, identifiers, numbers typically get different colors)
        assert!(
            colors.len() >= 3,
            "Code block should have at least 3 different foreground colors for proper syntax highlighting, found {} colors: {:?}",
            colors.len(),
            colors
        );
    }

    /// Regression test: code block background must extend to end of line.
    ///
    /// Bug: Background color stopped at end of content instead of filling
    /// to terminal edge. Fixed by adding \x1b[K (clear to EOL) after content.
    #[test]
    fn test_code_block_background_extends_to_eol() {
        let md: Markdown = "```rust\nfn main() {}\n```".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // The output should contain \x1b[K (clear to end of line) for each code line
        // This ensures the background color extends to the terminal edge
        assert!(
            output.contains("\x1b[K"),
            "Code block should use clear-to-EOL (\\x1b[K) to extend background to terminal edge"
        );

        // Verify the pattern is: background color -> content -> [K -> reset
        // The [K should come after the highlighted content and before the reset
        let has_eol_pattern = output.contains("\x1b[K\x1b[0m");
        assert!(
            has_eol_pattern,
            "Clear-to-EOL should be followed by reset: expected '\\x1b[K\\x1b[0m' pattern"
        );
    }

    /// Regression test: bash/shell syntax highlighting requires LinesWithEndings.
    ///
    /// Bug: Using `.lines()` strips newlines, but syntect's bash grammar requires
    /// newlines to properly track parser state across lines. Without newlines,
    /// everything gets the same "comment" color because the shebang line's comment
    /// state bleeds into subsequent lines.
    #[test]
    fn test_bash_highlighting_uses_lines_with_endings() {
        let md: Markdown = "```bash\n#!/bin/bash\necho \"Hello\"\n```".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Count distinct foreground colors - bash code should have multiple
        // (shebang/comments get one color, echo gets another, string gets another)
        let mut colors = std::collections::HashSet::new();
        let mut i = 0;
        let bytes = output.as_bytes();

        while i < bytes.len() {
            if i + 7 < bytes.len()
                && bytes[i] == 0x1b
                && bytes[i + 1] == b'['
                && bytes[i + 2] == b'3'
                && bytes[i + 3] == b'8'
                && bytes[i + 4] == b';'
                && bytes[i + 5] == b'2'
                && bytes[i + 6] == b';'
            {
                let start = i + 7;
                let mut end = start;
                while end < bytes.len() && bytes[end] != b'm' {
                    end += 1;
                }
                if end < bytes.len() {
                    let color_str = &output[start..end];
                    colors.insert(color_str.to_string());
                }
                i = end;
            }
            i += 1;
        }

        // With proper LinesWithEndings, bash should have at least 2 different colors:
        // - One for comments/shebang
        // - One for echo command or strings
        // Without LinesWithEndings, everything would be the same gray comment color
        assert!(
            colors.len() >= 2,
            "Bash code should have at least 2 different colors (comments vs commands), found {} colors: {:?}. \
            This likely means LinesWithEndings is not being used for syntax parsing.",
            colors.len(),
            colors
        );
    }
}
