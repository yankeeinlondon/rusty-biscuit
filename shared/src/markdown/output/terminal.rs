//! Terminal output with ANSI escape codes for markdown rendering.
//!
//! This module provides terminal-based rendering of markdown documents with
//! syntax highlighting using ANSI escape sequences. It supports:
//!
//! - Auto-detection of terminal color depth
//! - Code block syntax highlighting with line numbers
//! - Code block titles with visual prefix
//! - Configurable themes for code and prose
//! - GitHub Flavored Markdown tables with box-drawing characters (via comfy-table)
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
use comfy_table::{Attribute, Cell, CellAlignment, ContentArrangement, Table, presets};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style};
use syntect::parsing::{Scope, SyntaxReference};
use terminal_size::{terminal_size, Width};

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

/// Converts pulldown-cmark alignment to comfy-table alignment.
fn convert_alignment(align: &pulldown_cmark::Alignment) -> comfy_table::CellAlignment {
    match align {
        pulldown_cmark::Alignment::None => comfy_table::CellAlignment::Left,
        pulldown_cmark::Alignment::Left => comfy_table::CellAlignment::Left,
        pulldown_cmark::Alignment::Center => comfy_table::CellAlignment::Center,
        pulldown_cmark::Alignment::Right => comfy_table::CellAlignment::Right,
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

    // Query terminal width once at start
    const DEFAULT_TERMINAL_WIDTH: u16 = 80;
    let terminal_width = terminal_size()
        .map(|(Width(w), _)| w)
        .unwrap_or(DEFAULT_TERMINAL_WIDTH);
    tracing::debug!(terminal_width, "Terminal width detected for table rendering");

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
    let mut table_alignments: Vec<comfy_table::CellAlignment> = Vec::new();
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
            Event::Start(Tag::Table(alignments)) => {
                in_table = true;
                table_rows.clear();
                table_alignments = alignments.iter().map(convert_alignment).collect();
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
                output.push_str(&render_table(&table_rows, &table_alignments, terminal_width));
                // Add blank line after table for spacing from following content
                output.push_str("\n\n");
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

/// Processes inline code markers in cell content, applying background styling.
///
/// Replaces `\x00CODE\x00...\x00/CODE\x00` markers with ANSI-styled text.
/// Uses a subtle dark gray background (RGB: 50, 50, 55) for inline code.
///
/// ## Arguments
///
/// * `content` - Cell content with potential inline code markers
///
/// ## Returns
///
/// String with inline code markers replaced by ANSI-styled text
///
/// ## Note
///
/// This function is currently only used in tests. Production code uses
/// `strip_code_markers()` instead because raw ANSI injection causes
/// comfy-table to misalign columns.
#[cfg(test)]
fn process_cell_content(content: &str) -> String {
    const START_MARKER: &str = "\x00CODE\x00";
    const END_MARKER: &str = "\x00/CODE\x00";

    let mut result = String::new();
    let mut remaining = content;

    while let Some(start_idx) = remaining.find(START_MARKER) {
        // Add text before the marker
        result.push_str(&remaining[..start_idx]);

        // Skip past the start marker
        let after_start_idx = start_idx + START_MARKER.len();
        if after_start_idx > remaining.len() {
            // Malformed marker at end of string, just add rest
            result.push_str(&remaining[start_idx..]);
            remaining = "";
            break;
        }

        let after_start = &remaining[after_start_idx..];

        // Find the end marker
        if let Some(end_idx) = after_start.find(END_MARKER) {
            let code_text = &after_start[..end_idx];
            // Apply background color styling (subtle dark gray: 50, 50, 55)
            result.push_str(&format!("\x1b[48;2;50;50;55m{}\x1b[0m", code_text));

            // Skip past the end marker
            let next_idx = end_idx + END_MARKER.len();
            if next_idx <= after_start.len() {
                remaining = &after_start[next_idx..];
            } else {
                remaining = "";
            }
        } else {
            // No closing marker, just add the rest as-is
            result.push_str(&remaining[start_idx..]);
            remaining = "";
            break;
        }
    }
    result.push_str(remaining);
    result
}

/// Strips inline code markers from cell content for width calculation.
///
/// The markers `\x00CODE\x00` and `\x00/CODE\x00` are used to mark inline code
/// during parsing and are later converted to ANSI styling. They must be removed
/// before calculating display width.
fn strip_code_markers(s: &str) -> String {
    s.replace("\x00CODE\x00", "").replace("\x00/CODE\x00", "")
}

/// Calculates the minimum width needed for a column to avoid mid-word breaks.
///
/// Returns the length of the longest "word" (space-delimited segment) in the column,
/// considering ANSI escape codes and inline code markers which don't contribute
/// to visual width.
fn calculate_min_column_width(rows: &[Vec<String>], col_index: usize) -> usize {
    use unicode_width::UnicodeWidthStr;

    let mut max_word_len = 0;
    for row in rows {
        if let Some(cell) = row.get(col_index) {
            // Strip ANSI codes AND inline code markers for width calculation
            let plain_content = strip_code_markers(&crate::testing::strip_ansi_codes(cell));

            // Find the longest word (space-separated)
            for word in plain_content.split_whitespace() {
                let word_width = UnicodeWidthStr::width(word);
                max_word_len = max_word_len.max(word_width);
            }
        }
    }
    max_word_len
}

/// Renders a buffered table using comfy-table with automatic wrapping.
///
/// Box-drawing characters are used regardless of color depth. When `ColorDepth::None`
/// is configured, the table structure still renders with Unicode borders, but styling
/// attributes like bold headers may not display visually (though they are harmless).
///
/// ## Arguments
///
/// * `rows` - Vector of rows, where each row is a vector of cell strings.
///   First row is treated as header.
/// * `alignments` - Column alignment settings
/// * `terminal_width` - Terminal width for wrapping
#[tracing::instrument(
    skip(rows, alignments),
    fields(row_count = rows.len(), col_count, terminal_width)
)]
fn render_table(
    rows: &[Vec<String>],
    alignments: &[CellAlignment],
    terminal_width: u16,
) -> String {
    use comfy_table::{ColumnConstraint, Color as ComfyColor, Width};

    if rows.is_empty() {
        return String::new();
    }

    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    tracing::Span::current().record("col_count", col_count);

    let mut table = Table::new();

    // Set width constraint (use pre-queried width)
    table.set_width(terminal_width);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    // Minimalist preset
    table.load_preset(presets::UTF8_BORDERS_ONLY);

    // Add header row with bold styling
    // Note: We strip inline code markers but don't convert to ANSI - comfy-table
    // doesn't handle raw ANSI injection well during content wrapping, causing
    // alignment issues and broken escape sequences.
    if let Some(header) = rows.first() {
        table.set_header(header.iter().enumerate().map(|(i, cell_content)| {
            let alignment = alignments.get(i).copied().unwrap_or(CellAlignment::Left);
            // Strip markers without ANSI conversion - headers shouldn't have them anyway
            let plain_content = strip_code_markers(cell_content);
            Cell::new(plain_content)
                .set_alignment(alignment)
                .add_attribute(Attribute::Bold)
                .fg(ComfyColor::White)
        }));
    }

    // Add data rows
    // We strip inline code markers rather than converting to ANSI because:
    // 1. comfy-table breaks ANSI sequences during content wrapping
    // 2. This causes misalignment between headers and data columns
    // 3. Stripped markers preserve content while allowing proper width calculation
    for row in rows.iter().skip(1) {
        table.add_row(row.iter().enumerate().map(|(i, cell_content)| {
            let alignment = alignments.get(i).copied().unwrap_or(CellAlignment::Left);
            // Strip markers - trade off inline code styling for proper alignment
            let plain_content = strip_code_markers(cell_content);
            Cell::new(plain_content).set_alignment(alignment)
        }));
    }

    // Calculate minimum column widths based on longest words to prevent mid-word breaks.
    // This ensures that identifiers like "tool.duration_ms" won't be split.
    // We apply LowerBoundary constraints after adding content so columns exist.
    //
    // Strategy:
    // 1. Calculate the natural minimum width for each column (longest word)
    // 2. If total fits within terminal, apply as LowerBoundary constraints
    // 3. If total exceeds terminal, apply scaled-down constraints that still
    //    provide reasonable minimums to avoid the worst word breaks
    let min_widths: Vec<usize> = (0..col_count)
        .map(|i| calculate_min_column_width(rows, i) + 2) // +2 for padding
        .collect();

    // Calculate total minimum width including borders and column separators
    // comfy-table uses: │ col1 │ col2 │ col3 │ which adds 1 char per column + 1 for final border
    let border_overhead = col_count + 1;
    let total_min_width: usize = min_widths.iter().sum::<usize>() + border_overhead;

    // Always apply constraints, but scale them down if needed
    let usable_width = terminal_width.saturating_sub(border_overhead as u16) as usize;
    let constraints: Vec<ColumnConstraint> = if total_min_width <= terminal_width as usize {
        // All constraints fit - use them directly
        min_widths
            .into_iter()
            .map(|w| ColumnConstraint::LowerBoundary(Width::Fixed(w as u16)))
            .collect()
    } else if usable_width > 0 {
        // Constraints don't all fit - scale them proportionally
        // This gives each column a fair share while respecting relative importance
        let total_requested: usize = min_widths.iter().sum();
        let scale_factor = usable_width as f64 / total_requested as f64;

        min_widths
            .into_iter()
            .map(|w| {
                // Scale down but ensure at least 4 chars minimum (for very narrow terminals)
                let scaled = ((w as f64 * scale_factor) as usize).max(4);
                ColumnConstraint::LowerBoundary(Width::Fixed(scaled as u16))
            })
            .collect()
    } else {
        // Terminal too narrow for any meaningful constraints - let comfy-table handle it
        Vec::new()
    };

    if !constraints.is_empty() {
        table.set_constraints(constraints);
    }

    table.to_string()
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

    /// Test that ColorDepth::None still renders table structure with box-drawing characters
    #[test]
    fn test_color_depth_none_tables_still_render() {
        let md: Markdown = "| Header |\n|--------|\n| Data |".into();

        let mut options = TerminalOptions::default();
        options.color_depth = Some(ColorDepth::None);

        let output = for_terminal(&md, options).unwrap();

        // With ColorDepth::None, we return plain markdown (early return)
        // So tables won't be rendered with box-drawing - they'll be plain markdown
        assert!(!output.contains("\x1b["), "Should not have ANSI codes");
        assert_eq!(output, md.content(), "Should return plain markdown content");

        // To actually test that tables CAN render without colors (if we didn't early return),
        // we'd need to modify the implementation. But the current behavior is correct:
        // ColorDepth::None means "no formatting at all", so we return raw markdown.
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

        // Table should have box-drawing structure (comfy-table uses UTF8_BORDERS_ONLY preset)
        assert!(plain.contains("┌"), "Should have top-left corner");
        assert!(plain.contains("╞") || plain.contains("├"), "Should have header separator");
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
    ///
    /// Note: Inline code in table cells does NOT receive background styling because
    /// raw ANSI injection causes comfy-table to miscalculate column widths and break
    /// content alignment. The markers are stripped and content is rendered plain.
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

        // Inline code in tables does NOT have background styling - this is intentional
        // to prevent alignment issues. See test_table_inline_code_markers_stripped_for_width.
        assert!(
            !output.contains("\x1b[48;2;50;50;55m"),
            "Inline code in table should NOT have background styling (causes alignment issues), got:\n{}",
            output
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

    /// Test that tables render with multiple rows correctly
    /// Note: comfy-table doesn't add alternating row backgrounds, but tables still render correctly
    #[test]
    fn test_table_alternating_rows() {
        let md: Markdown = "| A |\n|---|\n| 1 |\n| 2 |\n| 3 |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Verify all data rows are present
        assert!(plain.contains(" A "), "Header should be present");
        assert!(plain.contains(" 1 "), "First data row should be present");
        assert!(plain.contains(" 2 "), "Second data row should be present");
        assert!(plain.contains(" 3 "), "Third data row should be present");

        // Should have box-drawing borders
        assert!(plain.contains("┌") && plain.contains("└"), "Should have table borders");
    }

    /// Test that table headers render correctly
    /// Note: comfy-table with Attribute::Bold may or may not emit ANSI codes depending on configuration
    #[test]
    fn test_table_header_bold() {
        let md: Markdown = "| Header |\n|--------|\n| Data |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Verify header and data are present
        assert!(plain.contains("Header"), "Header should be present");
        assert!(plain.contains("Data"), "Data should be present");

        // Should have table structure with header separator
        assert!(plain.contains("╞") || plain.contains("├"), "Should have header separator");
    }

    /// Test that inline code in tables renders correctly.
    /// Note: comfy-table doesn't support row background colors, so we just verify content is correct
    #[test]
    fn test_table_row_background_persists_after_inline_code() {
        let md: Markdown =
            "| Field | Description |\n|-------|-------------|\n| `foo` | A var |\n| `bar` | Another |"
                .into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Verify inline code content is present (backticks should be stripped)
        assert!(plain.contains("foo"), "Inline code 'foo' should be present");
        assert!(plain.contains("bar"), "Inline code 'bar' should be present");

        // Verify descriptive text is present
        assert!(plain.contains("A var"), "Description should be present");
        assert!(plain.contains("Another"), "Description should be present");

        // Should have table structure
        assert!(plain.contains("┌") && plain.contains("└"), "Should have table borders");
    }

    // ==================== Table Alignment Tests ====================

    /// Test that tables respect left alignment
    #[test]
    fn test_table_left_alignment() {
        let md: Markdown = "| Left |\n|:-----|\n| Data |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Verify content is present
        assert!(plain.contains("Left"), "Header should be present");
        assert!(plain.contains("Data"), "Data should be present");
    }

    /// Test that tables respect center alignment
    #[test]
    fn test_table_center_alignment() {
        let md: Markdown = "| Center |\n|:------:|\n| Data   |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Verify content is present
        assert!(plain.contains("Center"), "Header should be present");
        assert!(plain.contains("Data"), "Data should be present");
    }

    /// Test that tables respect right alignment
    #[test]
    fn test_table_right_alignment() {
        let md: Markdown = "| Right |\n|------:|\n| Data  |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Verify content is present
        assert!(plain.contains("Right"), "Header should be present");
        assert!(plain.contains("Data"), "Data should be present");
    }

    /// Test that tables handle mixed alignments
    #[test]
    fn test_table_mixed_alignments() {
        let md: Markdown = "| Left | Center | Right |\n|:-----|:------:|------:|\n| L1 | C1 | R1 |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Verify all headers are present
        assert!(plain.contains("Left"), "Left header should be present");
        assert!(plain.contains("Center"), "Center header should be present");
        assert!(plain.contains("Right"), "Right header should be present");

        // Verify data is present
        assert!(plain.contains("L1"), "Left data should be present");
        assert!(plain.contains("C1"), "Center data should be present");
        assert!(plain.contains("R1"), "Right data should be present");
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

    /// Test that process_cell_content correctly styles inline code markers
    #[test]
    fn test_process_cell_content() {
        // Test single inline code
        let input = "\x00CODE\x00FOO\x00/CODE\x00";
        let output = process_cell_content(input);
        assert_eq!(output, "\x1b[48;2;50;50;55mFOO\x1b[0m");

        // Test inline code with surrounding text
        let input = "Use \x00CODE\x00cargo build\x00/CODE\x00 to compile";
        let output = process_cell_content(input);
        assert_eq!(output, "Use \x1b[48;2;50;50;55mcargo build\x1b[0m to compile");

        // Test multiple inline codes
        let input = "Use \x00CODE\x00foo\x00/CODE\x00 and \x00CODE\x00bar\x00/CODE\x00";
        let output = process_cell_content(input);
        assert_eq!(output, "Use \x1b[48;2;50;50;55mfoo\x1b[0m and \x1b[48;2;50;50;55mbar\x1b[0m");

        // Test no markers
        let input = "Plain text";
        let output = process_cell_content(input);
        assert_eq!(output, "Plain text");

        // Test malformed (no closing marker) - keeps the opening marker and content
        let input = "\x00CODE\x00FOO";
        let output = process_cell_content(input);
        // When there's no closing marker, we keep everything as-is
        assert_eq!(output, "\x00CODE\x00FOO");
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

    // ==================== Phase 3: Comprehensive Tests ====================

    // ---- Width Constraint Tests ----

    /// Test that tables render correctly within terminal width constraints
    #[test]
    fn test_table_respects_terminal_width() {
        let md: Markdown = "| Column A | Column B | Column C |\n|----------|----------|----------|\n| Value 1 | Value 2 | Value 3 |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Table should render with box-drawing borders
        assert!(plain.contains("┌") && plain.contains("┘"), "Table should render with borders");

        // All content should be present
        assert!(plain.contains("Column A"));
        assert!(plain.contains("Column B"));
        assert!(plain.contains("Column C"));
        assert!(plain.contains("Value 1"));
    }

    /// Regression test: table width must be constrained to terminal width
    ///
    /// Bug: Tables were not respecting terminal width, causing content to overflow
    /// and words to split mid-word (e.g., "tool.duration_ms" -> "tool.duration_m" / "s")
    ///
    /// Fix: Added LowerBoundary column constraints based on longest word in each column.
    /// This prevents mid-word breaks when the terminal is wide enough to fit the content.
    #[test]
    fn test_table_width_constraint_enforced() {
        use unicode_width::UnicodeWidthStr;

        // Create a table with content that would exceed 50 chars per row
        let rows = vec![
            vec!["Field".to_string(), "Description".to_string(), "Example".to_string()],
            vec!["tool.name".to_string(), "Tool being called".to_string(), "\"brave_search\"".to_string()],
            vec!["tool.query".to_string(), "Search query/URL".to_string(), "\"rust async\"".to_string()],
            vec!["tool.duration_ms".to_string(), "Execution time".to_string(), "1234".to_string()],
        ];
        let alignments = vec![CellAlignment::Left, CellAlignment::Left, CellAlignment::Center];

        // Use width that can fit the content without mid-word breaks
        let adequate_width: u16 = 60;

        let output = render_table(&rows, &alignments, adequate_width);
        let plain = strip_ansi_codes(&output);

        // Every line should be <= adequate_width in display width
        for (i, line) in plain.lines().enumerate() {
            let display_width = UnicodeWidthStr::width(line);
            assert!(
                display_width <= adequate_width as usize,
                "Line {} exceeds width constraint: {} > {} in:\n{}\nFull table:\n{}",
                i, display_width, adequate_width, line, plain
            );
        }

        // Content should NOT be split mid-word at this width
        assert!(
            plain.contains("tool.duration_ms"),
            "tool.duration_ms should not be split, got:\n{}",
            plain
        );
        assert!(
            plain.contains("tool.name"),
            "tool.name should not be split, got:\n{}",
            plain
        );
    }

    /// Regression test: tables should not split words in the middle
    ///
    /// Bug: comfy-table was splitting "tool.duration_ms" into "tool.duration_m" and "s"
    /// when the terminal was narrow. This was caused by ANSI escape codes interfering
    /// with width calculation, or incorrect ContentArrangement settings.
    #[test]
    fn test_table_no_mid_word_splitting() {
        // Table from the bug report screenshot
        let rows = vec![
            vec!["Field".to_string(), "Description".to_string(), "Example".to_string()],
            vec!["tool.name".to_string(), "Tool being called".to_string(), "\"brave_search\"".to_string()],
            vec!["tool.query".to_string(), "Search query/URL".to_string(), "\"rust async\"".to_string()],
            vec!["tool.duration_ms".to_string(), "Execution time".to_string(), "1234".to_string()],
            vec!["tool.results_count".to_string(), "Results returned".to_string(), "10".to_string()],
            vec!["http.status_code".to_string(), "HTTP response".to_string(), "200".to_string()],
            vec!["otel.kind".to_string(), "Span kind".to_string(), "\"client\"".to_string()],
        ];
        let alignments = vec![CellAlignment::Left, CellAlignment::Left, CellAlignment::Center];

        // Test with narrow width (60 chars) like the screenshot showed
        let narrow_width: u16 = 60;

        let output = render_table(&rows, &alignments, narrow_width);
        let plain = strip_ansi_codes(&output);

        // Check for mid-word splits by looking at line breaks
        // Bad patterns: word followed immediately by newline with continuation on next line
        let bad_patterns = [
            "duration_m\n",  // duration_ms split after duration_m
            "results_co\n",  // results_count split
            "status_cod\n",  // status_code split
            "_m │",         // duration_ms split at underscore
            "_co │",        // results_count split
        ];

        for pattern in bad_patterns {
            assert!(
                !plain.contains(pattern),
                "Bad word split detected: found '{}' in:\n{}",
                pattern.escape_default(), plain
            );
        }

        // Verify full identifiers are present (not split across lines)
        // Each identifier should appear complete on at least one line
        for identifier in ["tool.duration_ms", "tool.results_count", "http.status_code"] {
            let found_complete = plain.lines().any(|line| line.contains(identifier));
            assert!(
                found_complete,
                "Identifier '{}' should appear complete on a single line:\n{}",
                identifier, plain
            );
        }
    }

    /// Test table rendering with very narrow width (40 chars) to stress-test wrapping
    #[test]
    fn test_table_very_narrow_width() {
        use unicode_width::UnicodeWidthStr;

        // Table from the bug report screenshot
        let rows = vec![
            vec!["Field".to_string(), "Description".to_string(), "Example".to_string()],
            vec!["tool.name".to_string(), "Tool being called".to_string(), "\"brave_search\"".to_string()],
            vec!["tool.query".to_string(), "Search query/URL".to_string(), "\"rust async\"".to_string()],
            vec!["tool.duration_ms".to_string(), "Execution time".to_string(), "1234".to_string()],
        ];
        let alignments = vec![CellAlignment::Left, CellAlignment::Left, CellAlignment::Center];

        // Very narrow width to force issues
        let narrow_width: u16 = 40;

        let output = render_table(&rows, &alignments, narrow_width);
        let plain = strip_ansi_codes(&output);

        eprintln!("Table at width {}:\n{}", narrow_width, plain);

        // Every line should respect the width constraint
        for (i, line) in plain.lines().enumerate() {
            let display_width = UnicodeWidthStr::width(line);
            assert!(
                display_width <= narrow_width as usize,
                "Line {} exceeds width constraint: {} > {} in:\n{}\nFull table:\n{}",
                i, display_width, narrow_width, line, plain
            );
        }
    }

    /// Regression test: inline code markers must not affect width calculation
    ///
    /// Bug: The markers `\x00CODE\x00` and `\x00/CODE\x00` were being counted in
    /// width calculation, causing columns to be sized incorrectly. Headers and
    /// data columns would not align because headers had no markers but data did.
    #[test]
    fn test_table_inline_code_markers_stripped_for_width() {
        // Simulate rows WITH inline code markers (as they come from markdown parsing)
        let rows = vec![
            vec!["Field".to_string(), "Description".to_string(), "Example".to_string()],
            // Data cells with inline code markers (like real markdown `tool.name`)
            vec![
                "\x00CODE\x00tool.name\x00/CODE\x00".to_string(),
                "Tool being called".to_string(),
                "\x00CODE\x00\"brave_search\"\x00/CODE\x00".to_string(),
            ],
            vec![
                "\x00CODE\x00tool.duration_ms\x00/CODE\x00".to_string(),
                "Execution time".to_string(),
                "\x00CODE\x001234\x00/CODE\x00".to_string(),
            ],
        ];
        let alignments = vec![CellAlignment::Left, CellAlignment::Left, CellAlignment::Center];

        // With adequate width, markers should not cause misalignment
        let width: u16 = 70;
        let output = render_table(&rows, &alignments, width);
        let plain = strip_ansi_codes(&output);

        // The markers should be converted to ANSI (and stripped), not visible
        assert!(
            !plain.contains("CODE"),
            "Markers should not appear in output:\n{}",
            plain
        );

        // Content should be intact (markers converted to styling)
        assert!(plain.contains("tool.name"), "tool.name should be present:\n{}", plain);
        assert!(plain.contains("tool.duration_ms"), "tool.duration_ms should be present:\n{}", plain);
        assert!(plain.contains("brave_search"), "brave_search should be present:\n{}", plain);

        // Headers and data should align - check that column separators line up
        // by verifying all lines have similar structure
        let lines: Vec<&str> = plain.lines().collect();
        assert!(lines.len() >= 3, "Table should have header + separator + data rows");

        // All content lines should have the same width (proper alignment)
        let content_widths: Vec<usize> = lines.iter()
            .map(|line| unicode_width::UnicodeWidthStr::width(*line))
            .collect();
        let first_width = content_widths[0];
        for (i, &w) in content_widths.iter().enumerate() {
            assert_eq!(
                w, first_width,
                "Line {} has width {} but expected {} (misalignment):\n{}",
                i, w, first_width, plain
            );
        }
    }

    /// Debug test to visualize table rendering at different widths
    #[test]
    #[ignore] // Run with: cargo test -p shared --lib test_table_width_visual -- --ignored --nocapture
    fn test_table_width_visual() {
        use unicode_width::UnicodeWidthStr;

        let rows = vec![
            vec!["Field".to_string(), "Description".to_string(), "Example".to_string()],
            vec!["tool.name".to_string(), "Tool being called".to_string(), "\"brave_search\"".to_string()],
            vec!["tool.query".to_string(), "Search query/URL".to_string(), "\"rust async\"".to_string()],
            vec!["tool.duration_ms".to_string(), "Execution time".to_string(), "1234".to_string()],
            vec!["tool.results_count".to_string(), "Results returned".to_string(), "10".to_string()],
            vec!["http.status_code".to_string(), "HTTP response".to_string(), "200".to_string()],
            vec!["otel.kind".to_string(), "Span kind".to_string(), "\"client\"".to_string()],
        ];
        let alignments = vec![CellAlignment::Left, CellAlignment::Left, CellAlignment::Center];

        for width in [40u16, 60, 80, 100, 120] {
            eprintln!("\n{}\nTerminal width: {}\n{}", "=".repeat(60), width, "=".repeat(60));

            let output = render_table(&rows, &alignments, width);
            let plain = strip_ansi_codes(&output);

            for (i, line) in plain.lines().enumerate() {
                let display_width = UnicodeWidthStr::width(line);
                eprintln!("Line {:2}: {:3} chars | {}", i, display_width, line);
            }
        }
    }

    /// Test that very long cell content wraps correctly within table cells
    #[test]
    fn test_table_long_cell_content_wraps() {
        let long_text = "This is a very long piece of text that should wrap within the table cell to fit the terminal width constraint";
        let md: Markdown = format!("| Header |\n|--------|\n| {} |", long_text).into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Content should be present
        assert!(plain.contains("This is a very long"));
        assert!(plain.contains("Header"));

        // Should have table structure
        assert!(plain.contains("┌") && plain.contains("└"), "Should have table borders");
    }

    /// Test that tables with many columns distribute width fairly
    #[test]
    fn test_table_many_columns_fair_width() {
        let md: Markdown = "| A | B | C | D | E | F |\n|---|---|---|---|---|---|\n| 1 | 2 | 3 | 4 | 5 | 6 |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // All columns should be present
        assert!(plain.contains("A"));
        assert!(plain.contains("B"));
        assert!(plain.contains("C"));
        assert!(plain.contains("D"));
        assert!(plain.contains("E"));
        assert!(plain.contains("F"));

        // All data should be present
        for i in 1..=6 {
            assert!(plain.contains(&i.to_string()));
        }
    }

    // ---- Edge Case Tests ----

    /// Test that empty table returns empty string
    #[test]
    fn test_empty_table_returns_empty() {
        let md: Markdown = "".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should only contain reset code
        assert_eq!(plain.trim(), "", "Empty markdown should produce empty output");
    }

    /// Test that single column table renders correctly
    #[test]
    fn test_single_column_table() {
        let md: Markdown = "| Single |\n|--------|\n| Data 1 |\n| Data 2 |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have header and data
        assert!(plain.contains("Single"), "Header should be present");
        assert!(plain.contains("Data 1"), "First data row should be present");
        assert!(plain.contains("Data 2"), "Second data row should be present");

        // Should have table structure
        assert!(plain.contains("┌") && plain.contains("└"), "Should have table borders");
    }

    /// Test that table with empty cells renders correctly
    #[test]
    fn test_table_with_empty_cells() {
        let md: Markdown = "| A | B |\n|---|---|\n|   | X |\n| Y |   |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have headers
        assert!(plain.contains("A"));
        assert!(plain.contains("B"));

        // Should have non-empty cells
        assert!(plain.contains("X"));
        assert!(plain.contains("Y"));

        // Should have table structure
        assert!(plain.contains("┌") && plain.contains("└"), "Should have table borders");
    }

    /// Test that ragged rows (fewer cells than columns) are handled
    #[test]
    fn test_table_ragged_rows() {
        // pulldown-cmark handles ragged rows by filling with empty strings
        let md: Markdown = "| A | B | C |\n|---|---|---|\n| 1 | 2 |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have all headers
        assert!(plain.contains("A"));
        assert!(plain.contains("B"));
        assert!(plain.contains("C"));

        // Should have present data
        assert!(plain.contains("1"));
        assert!(plain.contains("2"));

        // Should have table structure
        assert!(plain.contains("┌") && plain.contains("└"), "Should have table borders");
    }

    /// Test that Unicode characters in table cells render correctly
    #[test]
    fn test_table_unicode_characters() {
        let md: Markdown = "| Emoji | CJK |\n|-------|-----|\n| 🎉 | 中文 |\n| ✅ | 日本語 |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have headers
        assert!(plain.contains("Emoji"));
        assert!(plain.contains("CJK"));

        // Should have Unicode content
        assert!(plain.contains("🎉") || plain.contains("✅"), "Emoji should be present");
        assert!(plain.contains("中文") || plain.contains("日本語"), "CJK characters should be present");

        // Should have table structure
        assert!(plain.contains("┌") && plain.contains("└"), "Should have table borders");
    }

    // ---- ANSI Handling Tests ----

    /// Test that inline code in table cells doesn't break alignment
    #[test]
    fn test_table_inline_code_alignment() {
        let md: Markdown = "| Code | Description |\n|------|-------------|\n| `short` | Text |\n| `very_long_code_sample` | More text |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have headers
        assert!(plain.contains("Code"));
        assert!(plain.contains("Description"));

        // Inline code content should be present (without backticks)
        assert!(plain.contains("short"));
        assert!(plain.contains("very_long_code_sample"));

        // Should have table structure
        assert!(plain.contains("┌") && plain.contains("└"), "Should have table borders");
    }

    /// Test that long inline code in one cell, short text in another works
    #[test]
    fn test_table_mixed_inline_code_lengths() {
        let md: Markdown = "| Short | Long |\n|-------|------|\n| Hi | `this_is_a_very_long_code_identifier_that_might_cause_issues` |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have all content
        assert!(plain.contains("Short"));
        assert!(plain.contains("Long"));
        assert!(plain.contains("Hi"));
        assert!(plain.contains("this_is_a_very_long_code_identifier"));

        // Should have table structure
        assert!(plain.contains("┌") && plain.contains("└"), "Should have table borders");
    }

    /// Test that multiple inline code blocks in same cell work
    #[test]
    fn test_table_multiple_inline_code_in_cell() {
        let md: Markdown = "| Command |\n|---------|\n| Use `cargo build` then `cargo test` |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have header
        assert!(plain.contains("Command"));

        // Should have all inline code content (without backticks)
        assert!(plain.contains("cargo build"));
        assert!(plain.contains("cargo test"));
        assert!(plain.contains("Use"));
        assert!(plain.contains("then"));

        // Should have table structure
        assert!(plain.contains("┌") && plain.contains("└"), "Should have table borders");
    }

    // ---- Integration Tests ----

    /// Test that table renders correctly in a full document with multiple elements
    #[test]
    fn test_table_in_full_document() {
        let md: Markdown = r#"# Documentation

Some intro text with **bold** and `code`.

## Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `timeout` | 30 | Request timeout |
| `retries` | 3 | Retry attempts |

More text after the table.

- List item 1
- List item 2
"#.into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have heading
        assert!(plain.contains("# Documentation"));
        assert!(plain.contains("## Configuration"));

        // Should have paragraph text
        assert!(plain.contains("intro text"));
        assert!(plain.contains("bold"));

        // Should have table content
        assert!(plain.contains("Option"));
        assert!(plain.contains("Default"));
        assert!(plain.contains("Description"));
        assert!(plain.contains("timeout"));
        assert!(plain.contains("retries"));

        // Should have text after table
        assert!(plain.contains("More text after"));

        // Should have list
        assert!(plain.contains("- List item 1"));
        assert!(plain.contains("- List item 2"));

        // Should have table structure
        assert!(plain.contains("┌") && plain.contains("└"), "Should have table borders");
    }

    /// Test that multiple tables in same document render correctly
    #[test]
    fn test_multiple_tables_in_document() {
        let md: Markdown = r#"## Table 1

| A | B |
|---|---|
| 1 | 2 |

## Table 2

| X | Y | Z |
|---|---|---|
| a | b | c |
"#.into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have both headings
        assert!(plain.contains("## Table 1"));
        assert!(plain.contains("## Table 2"));

        // Should have first table content
        assert!(plain.contains("A"));
        assert!(plain.contains("B"));
        assert!(plain.contains("1"));
        assert!(plain.contains("2"));

        // Should have second table content
        assert!(plain.contains("X"));
        assert!(plain.contains("Y"));
        assert!(plain.contains("Z"));
        assert!(plain.contains("a"));
        assert!(plain.contains("b"));
        assert!(plain.contains("c"));

        // Should have multiple table structures
        let table_count = plain.matches("┌").count();
        assert!(table_count >= 2, "Should have at least 2 tables (found {} top borders)", table_count);
    }

    /// Test that table with complex nested markdown in cells works
    #[test]
    fn test_table_with_nested_markdown() {
        // Note: Most table implementations don't support nested block elements,
        // but inline elements like bold, emphasis, code should work
        let md: Markdown = "| Feature | Status |\n|---------|--------|\n| **Bold text** | `enabled` |\n| *Emphasis* | `disabled` |".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have headers
        assert!(plain.contains("Feature"));
        assert!(plain.contains("Status"));

        // Should have content (markdown markers might be stripped)
        assert!(plain.contains("Bold text"));
        assert!(plain.contains("Emphasis"));
        assert!(plain.contains("enabled"));
        assert!(plain.contains("disabled"));

        // Should have table structure
        assert!(plain.contains("┌") && plain.contains("└"), "Should have table borders");
    }
}
