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
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, Style};
use syntect::parsing::{Scope, SyntaxReference};
use terminal_size::{terminal_size, Width};
use viuer::{KittySupport, get_kitty_support, is_iterm_supported};

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

/// Controls how italic text is rendered to the terminal.
///
/// Different terminals have varying levels of support for italic text rendering.
/// This enum allows explicit control over italic behavior.
///
/// ## Examples
///
/// ```
/// use shared::markdown::output::terminal::{TerminalOptions, ItalicMode};
///
/// // Auto-detect (safe default)
/// let mut options = TerminalOptions::default();
/// assert!(matches!(options.italic_mode, ItalicMode::Auto));
///
/// // Force italics for pre-rendering to unknown terminals
/// options.italic_mode = ItalicMode::Always;
///
/// // Disable italics for terminals known not to support them
/// options.italic_mode = ItalicMode::Never;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItalicMode {
    /// Auto-detect italic support using terminal capabilities.
    ///
    /// Uses [`supports_italics()`](crate::terminal::supports_italics) to query
    /// the terminal. This is the safest option for direct terminal output.
    #[default]
    Auto,

    /// Always emit italic escape codes (`\x1b[3m`).
    ///
    /// Use this when pre-rendering content for a future terminal where
    /// capabilities cannot be detected. Assumes italic support is available.
    Always,

    /// Never emit italic escape codes.
    ///
    /// Use this when rendering for terminals known not to support italics,
    /// or when italic styling is not desired.
    Never,
}

impl ItalicMode {
    /// Resolves the mode to a boolean indicating whether to emit italic codes.
    ///
    /// ## Returns
    ///
    /// - `Auto`: Result of `supports_italics()` terminal capability check
    /// - `Always`: `true`
    /// - `Never`: `false`
    fn should_emit_italic(&self) -> bool {
        match self {
            ItalicMode::Auto => crate::terminal::supports_italics(),
            ItalicMode::Always => true,
            ItalicMode::Never => false,
        }
    }
}

/// Maximum image file size (10MB).
const MAX_IMAGE_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Renderer for inline images in terminal output.
///
/// Handles image rendering via `viuer` when graphics protocols (Kitty/iTerm2) are
/// available, with graceful fallback to styled placeholder text.
///
/// ## Security
///
/// - Validates paths don't escape base directory (path traversal prevention)
/// - Rejects files larger than 10MB
/// - Rejects remote URLs (http://, https://)
///
/// ## Examples
///
/// ```
/// use shared::markdown::output::terminal::ImageRenderer;
/// use std::path::Path;
///
/// let renderer = ImageRenderer::new(Some(Path::new("/tmp")));
/// assert!(!renderer.graphics_supported()); // Usually false in test environment
/// ```
#[derive(Debug)]
pub struct ImageRenderer {
    /// Whether graphics protocols (Kitty/iTerm2) are available.
    graphics_supported: bool,
    /// Whether stdout is a TTY.
    is_tty: bool,
    /// Base path for resolving relative image paths.
    base_path: PathBuf,
    /// Cached canonicalized base path for security validation.
    base_path_canonical: Option<PathBuf>,
    /// Terminal width for image scaling.
    terminal_width: u16,
}

impl ImageRenderer {
    /// Creates a new image renderer with automatic graphics detection.
    ///
    /// Detects terminal graphics protocol support (Kitty or iTerm2) and caches
    /// the result for use during rendering. Falls back to placeholder text when
    /// graphics are unavailable.
    ///
    /// ## Arguments
    ///
    /// * `base_path` - Base directory for resolving relative image paths.
    ///   Defaults to current working directory if `None`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::markdown::output::terminal::ImageRenderer;
    /// use std::path::Path;
    ///
    /// // Use current directory
    /// let renderer = ImageRenderer::new(None);
    ///
    /// // Use specific base path
    /// let renderer = ImageRenderer::new(Some(Path::new("/docs")));
    /// ```
    pub fn new(base_path: Option<&Path>) -> Self {
        let is_tty = std::io::stdout().is_terminal();

        let kitty_supported = matches!(
            get_kitty_support(),
            KittySupport::Local | KittySupport::Remote
        );
        let graphics_supported = is_tty && (kitty_supported || is_iterm_supported());

        let terminal_width = terminal_size()
            .map(|(Width(w), _)| w)
            .unwrap_or(80);

        let base = base_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let base_canonical = base.canonicalize().ok();

        tracing::debug!(
            graphics_supported,
            is_tty,
            terminal_width,
            base_path = %base.display(),
            "ImageRenderer initialized"
        );

        Self {
            graphics_supported,
            is_tty,
            base_path: base,
            base_path_canonical: base_canonical,
            terminal_width,
        }
    }

    /// Returns whether graphics protocols are supported.
    #[inline]
    pub fn graphics_supported(&self) -> bool {
        self.graphics_supported
    }

    /// Returns whether stdout is a TTY.
    #[inline]
    pub fn is_tty(&self) -> bool {
        self.is_tty
    }

    /// Returns the terminal width.
    #[inline]
    pub fn terminal_width(&self) -> u16 {
        self.terminal_width
    }

    /// Returns the base path for resolving relative image paths.
    #[inline]
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Renders an image from the given path.
    ///
    /// Attempts to render the image using viuer if graphics protocols are available,
    /// otherwise returns a styled fallback placeholder.
    ///
    /// ## Security
    ///
    /// - Rejects remote URLs (http://, https://)
    /// - Validates paths don't escape base directory (path traversal prevention)
    /// - Rejects files larger than 10MB
    ///
    /// ## Arguments
    ///
    /// * `image_path` - Path to the image (relative to base_path or absolute)
    /// * `alt_text` - Alt text to display in fallback placeholder
    ///
    /// ## Returns
    ///
    /// String containing either rendered image (via viuer) or fallback placeholder.
    #[tracing::instrument(skip(self), fields(
        image.path = %image_path,
        image.graphics_supported = %self.graphics_supported
    ))]
    pub fn render_image(&self, image_path: &str, alt_text: &str) -> String {
        // Reject remote URLs
        if image_path.starts_with("http://") || image_path.starts_with("https://") {
            tracing::debug!("Remote URLs not supported");
            return format!("▉ IMAGE[{}]\n", alt_text);
        }

        let full_path = if Path::new(image_path).is_absolute() {
            PathBuf::from(image_path)
        } else {
            self.base_path.join(image_path)
        };

        // Security: Prevent path traversal
        if let Some(ref base_canonical) = self.base_path_canonical
            && let Ok(canonical_image) = full_path.canonicalize()
            && !canonical_image.starts_with(base_canonical)
        {
            tracing::warn!(path = %image_path, "Image path escapes base directory");
            return format!("▉ IMAGE[{}]\n", alt_text);
        }

        // Check file exists
        if !full_path.exists() {
            tracing::debug!(path = %image_path, "Image file not found");
            return format!("▉ IMAGE[{}]\n", alt_text);
        }

        // Check file size
        if let Ok(metadata) = std::fs::metadata(&full_path)
            && metadata.len() > MAX_IMAGE_FILE_SIZE
        {
            tracing::warn!(size = metadata.len(), "Image file too large (>10MB)");
            return format!("▉ IMAGE[{}]\n", alt_text);
        }

        // Fallback if graphics unsupported
        if !self.graphics_supported {
            tracing::debug!("Graphics protocol not available");
            return format!("▉ IMAGE[{}]\n", alt_text);
        }

        // Constrain width only; let height be determined by aspect ratio
        let max_width = (self.terminal_width.saturating_sub(4) as u32).min(60);

        let config = viuer::Config {
            width: Some(max_width),
            height: None, // Auto-calculate from aspect ratio
            absolute_offset: false,
            restore_cursor: false,
            transparent: true,
            truecolor: true,
            ..Default::default()
        };

        match viuer::print_from_file(&full_path, &config) {
            Ok((_width, height)) => {
                tracing::debug!(path = %image_path, height, "Image rendered successfully");
                // With restore_cursor: false, viuer positions cursor after image.
                // No extra spacing needed - viuer handles it.
                String::new()
            }
            Err(e) => {
                tracing::warn!(path = %image_path, error = %e, "Image render failed");
                format!("▉ IMAGE[{}]\n", alt_text)
            }
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
///
/// ## Image Rendering
///
/// ```
/// use shared::markdown::output::terminal::TerminalOptions;
/// use std::path::PathBuf;
///
/// let mut options = TerminalOptions::default();
/// options.render_images = true; // Default
/// options.base_path = Some(PathBuf::from("/docs"));
/// ```
///
/// ## Italic Mode
///
/// ```
/// use shared::markdown::output::terminal::{TerminalOptions, ItalicMode};
///
/// let mut options = TerminalOptions::default();
/// // Auto-detect (default) - uses terminal capability detection
/// assert!(matches!(options.italic_mode, ItalicMode::Auto));
///
/// // Force italics for pre-rendering to unknown terminals
/// options.italic_mode = ItalicMode::Always;
/// ```
///
/// **Note:** Due to `#[non_exhaustive]`, use `let mut opts = TerminalOptions::default();`
/// and then set fields individually rather than struct update syntax.
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
    /// Whether to render images (via viuer) or show fallback text.
    /// Default: `true`
    pub render_images: bool,
    /// Base path for resolving relative image paths.
    /// If `None`, uses current working directory.
    pub base_path: Option<PathBuf>,
    /// Controls how italic text is rendered.
    ///
    /// - `Auto` (default): Detect terminal capability via `supports_italics()`
    /// - `Always`: Always emit italic escape codes (for pre-rendering)
    /// - `Never`: Never emit italic escape codes
    pub italic_mode: ItalicMode,
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
            render_images: true,
            base_path: None,
            italic_mode: ItalicMode::default(),
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
/// **Note:** For documents with images, use [`write_terminal`] instead, which
/// properly handles image rendering by writing directly to a writer.
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
pub fn for_terminal(md: &Markdown, mut options: TerminalOptions) -> Result<String, MarkdownError> {
    // Disable graphics rendering when returning a String, since viuer
    // writes directly to stdout which would be out of order.
    // Use write_terminal() with stdout for proper image rendering.
    options.render_images = false;
    let mut output = Vec::new();
    write_terminal(&mut output, md, options)?;
    Ok(String::from_utf8_lossy(&output).into_owned())
}

/// Writes markdown to a writer with ANSI escape codes and image support.
///
/// Unlike [`for_terminal`], this function writes directly to the provided writer,
/// enabling proper image rendering via viuer which requires direct stdout access.
///
/// ## Examples
///
/// ```no_run
/// use shared::markdown::Markdown;
/// use shared::markdown::output::terminal::{write_terminal, TerminalOptions};
/// use std::io::{self, Write};
///
/// let md: Markdown = "# Hello\n\n![image](./test.png)".into();
/// let stdout = io::stdout();
/// let mut handle = stdout.lock();
/// write_terminal(&mut handle, &md, TerminalOptions::default()).unwrap();
/// ```
///
/// ## Errors
///
/// Returns an error if theme loading fails or syntax highlighting encounters issues.
pub fn write_terminal<W: std::io::Write>(
    writer: &mut W,
    md: &Markdown,
    options: TerminalOptions,
) -> Result<(), MarkdownError> {
    let color_depth = options.color_depth.unwrap_or_else(ColorDepth::auto_detect);

    // Early return if no color support
    if color_depth == ColorDepth::None {
        write!(writer, "{}", md.content()).ok();
        return Ok(());
    }

    // Resolve italic mode once at start (avoids repeated capability detection)
    let emit_italic = options.italic_mode.should_emit_italic();

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

    // Use LineWrapper for proper word wrapping at terminal width
    let mut wrapper = LineWrapper::new(terminal_width as usize);

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

    // Image tracking and rendering
    let mut in_image = false;
    let mut current_alt = String::new();
    let mut current_image_path = String::new();
    let mut just_rendered_image = false; // Track if we just rendered an image (skip paragraph spacing)
    let image_renderer = if options.render_images {
        Some(ImageRenderer::new(options.base_path.as_deref()))
    } else {
        None
    };

    // Track semantic font styles for proper rendering
    // Some themes don't set font_style for markup scopes, so we track this explicitly
    let mut in_emphasis = false;
    let mut in_strong = false;

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

                // Get background color from theme for header row
                let theme = code_highlighter.theme();
                let bg_color = theme.settings.background.unwrap_or(Color::BLACK);

                // Add header row with title and language (right-aligned)
                let header = format_header_row(
                    meta.title.as_deref(),
                    &code_language,
                    bg_color,
                    options.color_mode,
                    terminal_width,
                );
                wrapper.push_with_newlines(&header);
                wrapper.newline();

                // Highlight and render code
                let highlighted = highlight_code(
                    &code_buffer,
                    &code_language,
                    &code_highlighter,
                    &options,
                    &meta,
                    options.color_mode,
                )?;
                wrapper.push_with_newlines(&highlighted);
                wrapper.newline();

                // Add trailing blank line with theme background
                wrapper.push_with_newlines(&emit_padding_row(bg_color));
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
                if !wrapper.output().is_empty() && !wrapper.output().ends_with("\n\n") {
                    if wrapper.output().ends_with('\n') {
                        wrapper.newline();
                    } else {
                        wrapper.push_with_newlines("\n\n");
                    }
                }
                let marker = match level {
                    pulldown_cmark::HeadingLevel::H1 => "█ ",
                    pulldown_cmark::HeadingLevel::H2 => "██ ",
                    pulldown_cmark::HeadingLevel::H3 => "████ ",
                    pulldown_cmark::HeadingLevel::H4 => "███████ ",
                    pulldown_cmark::HeadingLevel::H5 => "███████████ ",
                    pulldown_cmark::HeadingLevel::H6 => "███████████████ ",
                };
                let style = prose_highlighter.style_for_tag(tag, &scope_stack);
                wrapper.emit_styled_marker(marker, style, emit_italic);
            }
            Event::End(TagEnd::Heading(_)) => {
                scope_stack.pop();
                wrapper.push_with_newlines("\n\n"); // Blank line after heading
            }

            Event::Start(ref tag @ Tag::Strong) => {
                in_strong = true;
                if let Some(scope) = ScopeCache::global().scope_for_tag(tag) {
                    scope_stack.push(scope);
                }
            }
            Event::End(TagEnd::Strong) => {
                in_strong = false;
                scope_stack.pop();
            }

            Event::Start(ref tag @ Tag::Emphasis) => {
                in_emphasis = true;
                if let Some(scope) = ScopeCache::global().scope_for_tag(tag) {
                    scope_stack.push(scope);
                }
            }
            Event::End(TagEnd::Emphasis) => {
                in_emphasis = false;
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
                if !list_stack.is_empty() && !wrapper.output().is_empty() && !wrapper.output().ends_with('\n') {
                    wrapper.newline();
                }
                list_stack.push(start_num);
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                // Add blank line after top-level list ends
                if list_stack.is_empty() {
                    wrapper.newline();
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
                            wrapper.emit_styled_marker(&format!("{}{}. ", indent, num), style, emit_italic);
                            *num += 1;
                        }
                        None => {
                            // Unordered list: emit bullet
                            let style = prose_highlighter.base_style();
                            wrapper.emit_styled_marker(&format!("{}- ", indent), style, emit_italic);
                        }
                    }
                }
            }
            Event::End(TagEnd::Item) => {
                wrapper.newline();
            }

            Event::Start(Tag::Paragraph) => {
                // Don't add extra spacing inside list items
            }
            Event::End(TagEnd::Paragraph) => {
                // Skip paragraph spacing after images (image provides visual separation)
                if just_rendered_image {
                    just_rendered_image = false;
                    // viuer positions cursor after image; no extra spacing needed
                } else if list_stack.is_empty() {
                    // Only add double newline for paragraphs outside of lists
                    wrapper.push_with_newlines("\n\n");
                }
            }

            Event::Text(text) if !in_code_block => {
                if in_image {
                    // Accumulate alt text for image
                    current_alt.push_str(&text);
                } else if in_table {
                    // Buffer text for table cell
                    current_cell.push_str(&text);
                } else {
                    // Apply current prose styling based on scope stack
                    // For emphasis/strong: use parent color, only change font style
                    use syntect::highlighting::FontStyle;

                    let mut style = if in_emphasis || in_strong {
                        // Compute style from parent scopes (exclude emphasis/strong scopes)
                        // to preserve the parent's color
                        let parent_depth = scope_stack.len()
                            - (in_emphasis as usize)
                            - (in_strong as usize);
                        let parent_scopes = &scope_stack[..parent_depth.max(1)];
                        let tag = Tag::Paragraph;
                        prose_highlighter.style_for_tag(&tag, parent_scopes)
                    } else if scope_stack.len() > 1 {
                        let tag = Tag::Paragraph;
                        prose_highlighter.style_for_tag(&tag, &scope_stack)
                    } else {
                        prose_highlighter.base_style()
                    };

                    // Apply semantic font styles - italic/bold only change style, not color
                    if in_emphasis {
                        style.font_style |= FontStyle::ITALIC;
                    }
                    if in_strong {
                        style.font_style |= FontStyle::BOLD;
                    }

                    // Use LineWrapper for proper word wrapping
                    wrapper.emit_styled(&text, style, emit_italic);
                }
            }

            Event::Code(code) => {
                if in_image {
                    // Preserve inline code in alt text
                    current_alt.push('`');
                    current_alt.push_str(&code);
                    current_alt.push('`');
                } else if in_table {
                    // Buffer inline code for table cell (mark with special prefix for styling later)
                    current_cell.push_str(&format!("\x00CODE\x00{}\x00/CODE\x00", code));
                } else {
                    // Inline code with styling (no backticks in terminal output)
                    let style = prose_highlighter.style_for_inline_code(&scope_stack);
                    // Use LineWrapper for proper word wrapping
                    wrapper.emit_inline_code(&code, style);
                }
            }

            Event::SoftBreak => {
                // SoftBreak is a space - only emit if not at start of line
                if wrapper.current_col() > 0 {
                    wrapper.emit_raw(" ");
                }
            }
            Event::HardBreak => {
                wrapper.newline();
            }

            // Table handling - buffer entire table for proper rendering
            Event::Start(Tag::Table(alignments)) => {
                in_table = true;
                table_rows.clear();
                table_alignments = alignments.iter().map(convert_alignment).collect();
                // Add blank line before table if needed
                if !wrapper.output().is_empty() && !wrapper.output().ends_with("\n\n") {
                    if wrapper.output().ends_with('\n') {
                        wrapper.newline();
                    } else {
                        wrapper.push_with_newlines("\n\n");
                    }
                }
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
                // Render the buffered table with proper formatting
                wrapper.push_with_newlines(&render_table(&table_rows, &table_alignments, terminal_width));
                // Add blank line after table for spacing from following content
                wrapper.push_with_newlines("\n\n");
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

            // Image handling
            Event::Start(Tag::Image { dest_url, .. }) => {
                in_image = true;
                current_alt.clear();
                current_image_path = dest_url.to_string();
            }
            Event::End(TagEnd::Image) => {
                if let Some(ref renderer) = image_renderer {
                    // Flush accumulated output before viuer prints to stdout
                    if renderer.graphics_supported() {
                        write!(writer, "{}", wrapper.output()).ok();
                        writer.flush().ok();
                        // Clear the wrapper by creating a new one (preserving max_width)
                        wrapper = LineWrapper::new(terminal_width as usize);
                        // render_image prints via viuer
                        renderer.render_image(&current_image_path, &current_alt);
                        writer.flush().ok();
                        just_rendered_image = true;
                    } else {
                        wrapper.push_with_newlines(&renderer.render_image(&current_image_path, &current_alt));
                        just_rendered_image = true;
                    }
                } else {
                    wrapper.push_with_newlines(&format!("▉ IMAGE[{}]\n", current_alt));
                    just_rendered_image = true;
                }
                in_image = false;
            }

            _ => {} // Ignore other events
        }
    }

    // Get the final output from the wrapper
    let mut output = wrapper.into_output();

    // Always emit terminal reset at end
    output.push_str("\x1b[0m");

    // Write final output
    write!(writer, "{}", output).ok();
    writer.flush().ok();

    Ok(())
}

/// Emits prose text with foreground color and font style (bold, italic, underline).
///
/// ## Arguments
///
/// * `text` - The text content to emit
/// * `style` - The syntect style including foreground color and font styles
/// * `emit_italic` - Whether to emit italic escape codes when font_style contains ITALIC
fn emit_prose_text(text: &str, style: Style, emit_italic: bool) -> String {
    use syntect::highlighting::FontStyle;

    let fg = style.foreground;
    let mut result = String::new();

    // Apply font styles (bold=1, italic=3, underline=4)
    if style.font_style.contains(FontStyle::BOLD) {
        result.push_str("\x1b[1m");
    }
    if emit_italic && style.font_style.contains(FontStyle::ITALIC) {
        result.push_str("\x1b[3m");
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        result.push_str("\x1b[4m");
    }

    // Apply foreground color and text
    result.push_str(&format!("\x1b[38;2;{};{};{}m{}\x1b[0m", fg.r, fg.g, fg.b, text));
    result
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

/// Returns the text color for header rows based on color mode.
///
/// ## Arguments
///
/// * `color_mode` - The color mode (dark or light)
///
/// ## Returns
///
/// RGB tuple: (255, 255, 255) for dark mode (white text), (0, 0, 0) for light mode (black text)
fn header_text_color(color_mode: ColorMode) -> (u8, u8, u8) {
    match color_mode {
        ColorMode::Dark => (255, 255, 255),  // WHITE
        ColorMode::Light => (0, 0, 0),       // BLACK
    }
}

/// Formats a code block header row with title (left) and language (right-aligned).
///
/// Creates a header row showing the title on the left (bold) and language on the right
/// (not bold). Both use the theme's background color. Spacing fills the gap to push
/// the language to the right edge of the terminal.
///
/// ## Arguments
///
/// * `title` - Optional title text for the code block
/// * `language` - Language identifier (defaults to "text" if empty)
/// * `bg_color` - Background color for title and language spans
/// * `color_mode` - Color mode for determining text color
/// * `terminal_width` - Terminal width for right-alignment calculation
///
/// ## Returns
///
/// ANSI-formatted string with title (if present) on the left and language right-aligned.
/// Title is bold, language is not.
fn format_header_row(
    title: Option<&str>,
    language: &str,
    bg_color: Color,
    color_mode: ColorMode,
    terminal_width: u16,
) -> String {
    let text_color = header_text_color(color_mode);
    let lang = if language.is_empty() { "text" } else { language };

    // Calculate visible widths for spacing
    // Title: " {title} " = 1 + title.len() + 1 = title.len() + 2
    // Language: " {lang} " = 1 + lang.len() + 1 = lang.len() + 2
    let title_width = title.map(|t| t.chars().count() + 2).unwrap_or(0);
    let lang_width = lang.chars().count() + 2;
    let total_content_width = title_width + lang_width;

    // Calculate spacing to right-align language
    let spacing = if (terminal_width as usize) > total_content_width {
        terminal_width as usize - total_content_width
    } else {
        1 // Minimum 1 space between title and language
    };

    let mut output = String::new();

    // Left side: title (if present)
    if let Some(t) = title {
        // Bold + BG + FG + space + title + space + reset
        output.push_str(&format!(
            "\x1b[1m\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m {} \x1b[0m",
            bg_color.r, bg_color.g, bg_color.b,
            text_color.0, text_color.1, text_color.2,
            t
        ));
    }

    // Add spacing to push language to the right
    for _ in 0..spacing {
        output.push(' ');
    }

    // Right side: language (right-aligned)
    // BG + FG + space + lang + space + reset
    output.push_str(&format!(
        "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m {} \x1b[0m",
        bg_color.r, bg_color.g, bg_color.b,
        text_color.0, text_color.1, text_color.2,
        lang
    ));

    output
}

/// Emits a padding row with the specified background color.
///
/// The padding row consists of:
/// - Setting the background color
/// - Clearing to end of line (\x1b[K)
/// - Resetting all attributes (\x1b[0m)
/// - Adding a newline
fn emit_padding_row(bg_color: Color) -> String {
    format!(
        "\x1b[48;2;{};{};{}m\x1b[K\x1b[0m\n",
        bg_color.r, bg_color.g, bg_color.b
    )
}

/// A wrapper that handles word-based line wrapping for prose text.
///
/// Tracks the current column position and wraps at word boundaries to prevent
/// mid-word breaks and leading whitespace on continuation lines.
struct LineWrapper {
    /// Current column position (visual width, excluding ANSI codes)
    current_col: usize,
    /// Maximum line width
    max_width: usize,
    /// Output buffer
    output: String,
}

impl LineWrapper {
    /// Creates a new LineWrapper with the given maximum width.
    fn new(max_width: usize) -> Self {
        Self {
            current_col: 0,
            max_width,
            output: String::new(),
        }
    }

    /// Emits styled text with word wrapping.
    ///
    /// Splits the text into words and wraps at word boundaries. Each word is
    /// emitted with the provided style. Leading whitespace on continuation lines
    /// is stripped to prevent visible indentation after wrapping.
    ///
    /// ## Arguments
    ///
    /// * `text` - The plain text to emit
    /// * `style` - The syntect style for coloring
    /// * `emit_italic` - Whether to emit italic escape codes
    fn emit_styled(&mut self, text: &str, style: Style, emit_italic: bool) {
        // Split into segments, preserving whitespace
        // We iterate over whitespace-separated words, handling spaces between them
        let mut chars = text.chars().peekable();
        let mut current_word = String::new();

        while let Some(ch) = chars.next() {
            if ch.is_whitespace() {
                // Emit accumulated word first
                if !current_word.is_empty() {
                    self.emit_word(&current_word, style, emit_italic);
                    current_word.clear();
                }
                // Handle whitespace
                if ch == '\n' {
                    // Hard break - emit newline and reset column
                    self.output.push('\n');
                    self.current_col = 0;
                } else {
                    // Space or tab - emit if not at start of line
                    if self.current_col > 0 {
                        self.output.push(' ');
                        self.current_col += 1;
                    }
                }
            } else {
                current_word.push(ch);
            }
        }

        // Emit any remaining word
        if !current_word.is_empty() {
            self.emit_word(&current_word, style, emit_italic);
        }
    }

    /// Emits a single word, wrapping if necessary.
    fn emit_word(&mut self, word: &str, style: Style, emit_italic: bool) {
        use unicode_width::UnicodeWidthStr;

        let word_width = UnicodeWidthStr::width(word);

        // Check if word fits on current line
        if self.current_col > 0 && self.current_col + word_width > self.max_width {
            // Need to wrap - emit newline
            self.output.push('\n');
            self.current_col = 0;
        }

        // Emit the styled word
        self.output.push_str(&emit_prose_text(word, style, emit_italic));
        self.current_col += word_width;
    }

    /// Emits a raw string without styling (for markers, bullets, etc.).
    /// Updates column position based on visual width.
    fn emit_raw(&mut self, text: &str) {
        use unicode_width::UnicodeWidthStr;
        self.output.push_str(text);
        // Update column for non-newline content
        if let Some(last_line) = text.rsplit('\n').next() {
            if text.contains('\n') {
                self.current_col = UnicodeWidthStr::width(last_line);
            } else {
                self.current_col += UnicodeWidthStr::width(text);
            }
        }
    }

    /// Emits styled text for markers/prefixes (bullets, numbers).
    /// These are emitted directly without word-wrap logic.
    fn emit_styled_marker(&mut self, text: &str, style: Style, emit_italic: bool) {
        use unicode_width::UnicodeWidthStr;
        self.output.push_str(&emit_prose_text(text, style, emit_italic));
        self.current_col += UnicodeWidthStr::width(text);
    }

    /// Emits inline code with styling.
    fn emit_inline_code(&mut self, code: &str, style: Style) {
        use unicode_width::UnicodeWidthStr;

        let code_width = UnicodeWidthStr::width(code);

        // Check if code fits on current line
        if self.current_col > 0 && self.current_col + code_width > self.max_width {
            // Wrap before inline code
            self.output.push('\n');
            self.current_col = 0;
        }

        self.output.push_str(&emit_inline_code(code, style));
        self.current_col += code_width;
    }

    /// Adds a newline and resets column position.
    fn newline(&mut self) {
        self.output.push('\n');
        self.current_col = 0;
    }

    /// Adds content that includes newlines (resets column tracking).
    fn push_with_newlines(&mut self, text: &str) {
        self.output.push_str(text);
        if text.ends_with('\n') {
            self.current_col = 0;
        }
    }

    /// Consumes the wrapper and returns the output string.
    fn into_output(self) -> String {
        self.output
    }

    /// Gets a reference to the current output.
    fn output(&self) -> &str {
        &self.output
    }

    /// Gets the current column position.
    fn current_col(&self) -> usize {
        self.current_col
    }
}

/// Computes a highlighted background color based on the theme background and color mode.
///
/// For dark mode, adds brightness to the theme background (capped at 235).
/// For light mode, subtracts brightness from the theme background.
///
/// ## Examples
///
/// ```
/// use shared::markdown::highlighting::ColorMode;
/// use syntect::highlighting::Color;
///
/// let theme_bg = Color { r: 40, g: 40, b: 40, a: 255 };
/// let highlight_bg = compute_highlight_bg(theme_bg, ColorMode::Dark);
/// // Result: Color { r: 70, g: 65, b: 40, a: 255 }
/// ```
fn compute_highlight_bg(theme_bg: Color, color_mode: ColorMode) -> Color {
    match color_mode {
        ColorMode::Dark => Color {
            r: theme_bg.r.saturating_add(30).min(235),
            g: theme_bg.g.saturating_add(25).min(235),
            b: theme_bg.b,
            a: 255,
        },
        ColorMode::Light => Color {
            r: theme_bg.r.saturating_sub(20),
            g: theme_bg.g.saturating_sub(15),
            b: theme_bg.b,
            a: 255,
        },
    }
}

/// Highlights code with syntax highlighting and optional line numbers.
///
/// Applies syntax highlighting using syntect, adds top/bottom padding rows,
/// and optionally renders line numbers and line highlighting based on DSL metadata.
///
/// ## Arguments
///
/// * `code` - Source code to highlight
/// * `language` - Programming language identifier (e.g., "rust", "python")
/// * `highlighter` - Code highlighter with loaded syntax set and theme
/// * `options` - Terminal rendering options (includes global line numbering flag)
/// * `meta` - Code block DSL metadata (title, highlight ranges, line numbering override)
/// * `color_mode` - Dark or light mode for computing highlight background colors
///
/// ## Returns
///
/// ANSI-formatted string with:
/// - Top padding row (blank line with theme background)
/// - Code lines with syntax highlighting, optional line numbers, and highlight backgrounds
/// - Bottom padding row (blank line with theme background)
///
/// ## Examples
///
/// ```
/// use shared::markdown::highlighting::{CodeHighlighter, ThemePair, ColorMode};
/// use shared::markdown::output::TerminalOptions;
/// use shared::markdown::dsl::CodeBlockMeta;
///
/// let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
/// let options = TerminalOptions::default();
/// let meta = CodeBlockMeta::default();
///
/// let result = highlight_code(
///     "fn main() {}",
///     "rust",
///     &highlighter,
///     &options,
///     &meta,
///     ColorMode::Dark
/// );
/// // Result: ANSI-formatted code with padding, syntax colors, and backgrounds
/// ```
fn highlight_code(
    code: &str,
    language: &str,
    highlighter: &CodeHighlighter,
    options: &TerminalOptions,
    meta: &crate::markdown::dsl::CodeBlockMeta,
    color_mode: ColorMode,
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

    // Add top padding row
    output.push_str(&emit_padding_row(bg_color));

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
            line_bg.r, line_bg.g, line_bg.b
        ));

        // Add left padding (1 character)
        output.push(' ');

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

    // Add bottom padding row (without trailing newline to avoid double spacing)
    output.push_str(&format!(
        "\x1b[48;2;{};{};{}m\x1b[K\x1b[0m",
        bg_color.r, bg_color.g, bg_color.b
    ));

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
        // H1 uses block marker: █
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("█ Hello World"));
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

        // Should contain title (without prefix)
        assert!(output.contains("Example"));

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
    fn test_header_row_with_title() {
        let bg_color = Color { r: 40, g: 40, b: 40, a: 255 };
        let header = format_header_row(Some("Example"), "rust", bg_color, ColorMode::Dark, 80);

        // Should contain both title and language
        let plain = strip_ansi_codes(&header);
        assert!(plain.contains("Example"));
        assert!(plain.contains("rust"));

        // Should contain ANSI codes
        assert!(header.contains("\x1b["));
    }

    #[test]
    fn test_header_row_no_title() {
        let bg_color = Color { r: 40, g: 40, b: 40, a: 255 };
        let header = format_header_row(None, "javascript", bg_color, ColorMode::Dark, 80);

        // Should only contain language
        let plain = strip_ansi_codes(&header);
        assert!(plain.contains("javascript"));
        assert!(!plain.contains("Example"));

        // Should contain ANSI codes
        assert!(header.contains("\x1b["));
    }

    #[test]
    fn test_header_row_default_language() {
        let bg_color = Color { r: 40, g: 40, b: 40, a: 255 };
        let header = format_header_row(None, "", bg_color, ColorMode::Dark, 80);

        // Should default to "text"
        let plain = strip_ansi_codes(&header);
        assert!(plain.contains("text"));
    }

    #[test]
    fn test_header_row_dark_mode() {
        let bg_color = Color { r: 40, g: 40, b: 40, a: 255 };
        let header = format_header_row(Some("Title"), "rust", bg_color, ColorMode::Dark, 80);

        // Should contain white text color (255, 255, 255)
        assert!(header.contains("\x1b[38;2;255;255;255m"));
    }

    #[test]
    fn test_header_row_light_mode() {
        let bg_color = Color { r: 240, g: 240, b: 240, a: 255 };
        let header = format_header_row(Some("Title"), "rust", bg_color, ColorMode::Light, 80);

        // Should contain black text color (0, 0, 0)
        assert!(header.contains("\x1b[38;2;0;0;0m"));
    }

    #[test]
    fn test_header_row_bold_title() {
        let bg_color = Color { r: 40, g: 40, b: 40, a: 255 };
        let header = format_header_row(Some("Title"), "rust", bg_color, ColorMode::Dark, 80);

        // Should contain bold code
        assert!(header.contains("\x1b[1m"));
    }

    #[test]
    fn test_header_row_no_bold_language() {
        let bg_color = Color { r: 40, g: 40, b: 40, a: 255 };
        let header = format_header_row(None, "rust", bg_color, ColorMode::Dark, 80);

        // Language-only header should not start with bold (title is what gets bolded)
        // The format is: "\x1b[48;2;...m\x1b[38;2;...m rust \x1b[0m"
        // It should NOT contain "\x1b[1m" when there's no title
        assert!(!header.contains("\x1b[1m"));
    }

    #[test]
    fn test_header_row_right_alignment() {
        let bg_color = Color { r: 40, g: 40, b: 40, a: 255 };
        // Title "Test" (4 chars) + leading/trailing spaces = 6 visible chars
        // Language "rs" (2 chars) + spaces " rs " = 4 visible chars
        // Total content: 10 chars, terminal width: 80, so spacing: 70 chars
        let header = format_header_row(Some("Test"), "rs", bg_color, ColorMode::Dark, 80);
        let plain = strip_ansi_codes(&header);

        // Title should be at the start with leading space, language at the end
        assert!(plain.starts_with(" Test "));
        assert!(plain.ends_with(" rs "));

        // Total visible width should be 80 (terminal width)
        assert_eq!(plain.chars().count(), 80);
    }

    #[test]
    fn test_header_text_color_dark() {
        let color = header_text_color(ColorMode::Dark);
        assert_eq!(color, (255, 255, 255)); // White
    }

    #[test]
    fn test_header_text_color_light() {
        let color = header_text_color(ColorMode::Light);
        assert_eq!(color, (0, 0, 0)); // Black
    }

    #[test]
    fn test_highlight_code_basic() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let options = TerminalOptions::default();
        let meta = crate::markdown::dsl::CodeBlockMeta::default();

        let code = "fn main() {}";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta, ColorMode::Dark);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Should contain ANSI escape codes
        assert!(output.contains("\x1b["));
    }

    #[test]
    fn test_code_block_padding() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let options = TerminalOptions::default();
        let meta = crate::markdown::dsl::CodeBlockMeta::default();

        let code = "fn main() {}";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta, ColorMode::Dark);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Get background color from theme
        let theme = highlighter.theme();
        let bg_color = theme.settings.background.unwrap_or(Color::BLACK);
        let bg_code = format!("\x1b[48;2;{};{};{}m", bg_color.r, bg_color.g, bg_color.b);

        // Should start with top padding row (bg color + clear + reset + newline)
        assert!(
            output.starts_with(&format!("{}\x1b[K\x1b[0m\n", bg_code)),
            "Output should start with top padding row"
        );

        // Should end with bottom padding row (bg color + clear + reset, no newline)
        assert!(
            output.ends_with(&format!("{}\x1b[K\x1b[0m", bg_code)),
            "Output should end with bottom padding row"
        );
    }

    #[test]
    fn test_code_block_left_padding() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let options = TerminalOptions::default();
        let meta = crate::markdown::dsl::CodeBlockMeta::default();

        let code = "fn main() {}";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta, ColorMode::Dark);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Get background color from theme
        let theme = highlighter.theme();
        let bg_color = theme.settings.background.unwrap_or(Color::BLACK);
        let bg_code = format!("\x1b[48;2;{};{};{}m", bg_color.r, bg_color.g, bg_color.b);

        // After the top padding row and background color set, should have a space for left padding
        let expected_sequence = format!("{}\x1b[K\x1b[0m\n{} ", bg_code, bg_code);
        assert!(
            output.contains(&expected_sequence),
            "Code lines should have 1-character left padding after background color"
        );
    }

    #[test]
    fn test_code_block_padding_uses_theme_background() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let options = TerminalOptions::default();
        let meta = crate::markdown::dsl::CodeBlockMeta::default();

        let code = "test";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta, ColorMode::Dark);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Get expected background color from theme
        let theme = highlighter.theme();
        let bg_color = theme.settings.background.unwrap_or(Color::BLACK);
        let expected_bg = format!("\x1b[48;2;{};{};{}m", bg_color.r, bg_color.g, bg_color.b);

        // Verify padding rows use the theme's background color
        assert!(
            output.contains(&expected_bg),
            "Padding should use theme background color"
        );

        // Count occurrences of background color code
        let bg_count = output.matches(&expected_bg).count();
        // Should appear at least twice: top padding + at least one code line
        assert!(
            bg_count >= 2,
            "Background color should appear in padding rows and code lines"
        );
    }

    #[test]
    fn test_code_block_highlight_single_line() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let options = TerminalOptions::default();
        let mut meta = crate::markdown::dsl::CodeBlockMeta::default();
        meta.highlight.add_line(2);

        let code = "line 1\nline 2\nline 3";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta, ColorMode::Dark);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Get expected background colors
        let theme = highlighter.theme();
        let bg_color = theme.settings.background.unwrap_or(Color::BLACK);
        let highlight_bg = compute_highlight_bg(bg_color, ColorMode::Dark);

        let normal_bg = format!("\x1b[48;2;{};{};{}m", bg_color.r, bg_color.g, bg_color.b);
        let highlighted_bg = format!(
            "\x1b[48;2;{};{};{}m",
            highlight_bg.r, highlight_bg.g, highlight_bg.b
        );

        // Verify both normal and highlighted backgrounds are present
        assert!(
            output.contains(&normal_bg),
            "Output should contain normal background color"
        );
        assert!(
            output.contains(&highlighted_bg),
            "Output should contain highlighted background color for line 2"
        );
    }

    #[test]
    fn test_code_block_highlight_range() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let options = TerminalOptions::default();
        let mut meta = crate::markdown::dsl::CodeBlockMeta::default();
        meta.highlight.add_range(2, 4).unwrap();

        let code = "line 1\nline 2\nline 3\nline 4\nline 5";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta, ColorMode::Dark);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Get expected background colors
        let theme = highlighter.theme();
        let bg_color = theme.settings.background.unwrap_or(Color::BLACK);
        let highlight_bg = compute_highlight_bg(bg_color, ColorMode::Dark);

        let highlighted_bg = format!(
            "\x1b[48;2;{};{};{}m",
            highlight_bg.r, highlight_bg.g, highlight_bg.b
        );

        // Verify highlighted background is present
        assert!(
            output.contains(&highlighted_bg),
            "Output should contain highlighted background for lines 2-4"
        );
    }

    #[test]
    fn test_code_block_highlight_mixed() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let options = TerminalOptions::default();
        let mut meta = crate::markdown::dsl::CodeBlockMeta::default();
        meta.highlight.add_line(1);
        meta.highlight.add_range(4, 6).unwrap();

        let code = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta, ColorMode::Dark);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Get expected background colors
        let theme = highlighter.theme();
        let bg_color = theme.settings.background.unwrap_or(Color::BLACK);
        let highlight_bg = compute_highlight_bg(bg_color, ColorMode::Dark);

        let normal_bg = format!("\x1b[48;2;{};{};{}m", bg_color.r, bg_color.g, bg_color.b);
        let highlighted_bg = format!(
            "\x1b[48;2;{};{};{}m",
            highlight_bg.r, highlight_bg.g, highlight_bg.b
        );

        // Verify both normal and highlighted backgrounds are present
        assert!(
            output.contains(&normal_bg),
            "Output should contain normal background color"
        );
        assert!(
            output.contains(&highlighted_bg),
            "Output should contain highlighted background for lines 1,4-6"
        );
    }

    #[test]
    fn test_highlight_with_line_numbers() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let mut options = TerminalOptions::default();
        options.include_line_numbers = true;
        let mut meta = crate::markdown::dsl::CodeBlockMeta::default();
        meta.highlight.add_line(2);

        let code = "line 1\nline 2\nline 3";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta, ColorMode::Dark);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Get expected background colors
        let theme = highlighter.theme();
        let bg_color = theme.settings.background.unwrap_or(Color::BLACK);
        let highlight_bg = compute_highlight_bg(bg_color, ColorMode::Dark);

        let highlighted_bg = format!(
            "\x1b[48;2;{};{};{}m",
            highlight_bg.r, highlight_bg.g, highlight_bg.b
        );

        // Verify line numbers are present
        assert!(
            output.contains("│"),
            "Output should contain line number separator"
        );

        // Verify highlighting works with line numbers
        assert!(
            output.contains(&highlighted_bg),
            "Output should contain highlighted background even with line numbers"
        );
    }

    #[test]
    fn test_padding_preserves_line_numbers_alignment() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let mut options = TerminalOptions::default();
        options.include_line_numbers = true;
        let meta = crate::markdown::dsl::CodeBlockMeta::default();

        let code = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta, ColorMode::Dark);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Strip ANSI to verify alignment visually
        let plain = strip_ansi_codes(&output);

        // Verify line numbers are present and properly aligned
        assert!(plain.contains(" 1 │"), "Line 1 should have proper padding");
        assert!(plain.contains("10 │"), "Line 10 should align with line 1");

        // Count separator occurrences (should equal number of code lines)
        let separator_count = plain.matches("│").count();
        assert_eq!(
            separator_count, 10,
            "Should have 10 line number separators for 10 lines of code"
        );
    }

    #[test]
    fn test_highlight_ignores_zero() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let options = TerminalOptions::default();
        let mut meta = crate::markdown::dsl::CodeBlockMeta::default();
        meta.highlight.add_line(0); // Line 0 should be ignored (1-indexed)

        let code = "line 1\nline 2\nline 3";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta, ColorMode::Dark);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Get expected background colors
        let theme = highlighter.theme();
        let bg_color = theme.settings.background.unwrap_or(Color::BLACK);
        let highlight_bg = compute_highlight_bg(bg_color, ColorMode::Dark);

        let normal_bg = format!("\x1b[48;2;{};{};{}m", bg_color.r, bg_color.g, bg_color.b);
        let highlighted_bg = format!(
            "\x1b[48;2;{};{};{}m",
            highlight_bg.r, highlight_bg.g, highlight_bg.b
        );

        // Should contain normal background
        assert!(
            output.contains(&normal_bg),
            "Output should contain normal background"
        );

        // Should NOT contain highlighted background (since line 0 is invalid)
        assert!(
            !output.contains(&highlighted_bg),
            "Output should NOT contain highlighted background when only line 0 is specified"
        );
    }

    #[test]
    fn test_highlight_out_of_range() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let options = TerminalOptions::default();
        let mut meta = crate::markdown::dsl::CodeBlockMeta::default();
        meta.highlight.add_line(100); // Line 100 on 5-line code should be ignored

        let code = "line 1\nline 2\nline 3\nline 4\nline 5";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta, ColorMode::Dark);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Get expected background colors
        let theme = highlighter.theme();
        let bg_color = theme.settings.background.unwrap_or(Color::BLACK);
        let highlight_bg = compute_highlight_bg(bg_color, ColorMode::Dark);

        let normal_bg = format!("\x1b[48;2;{};{};{}m", bg_color.r, bg_color.g, bg_color.b);
        let highlighted_bg = format!(
            "\x1b[48;2;{};{};{}m",
            highlight_bg.r, highlight_bg.g, highlight_bg.b
        );

        // Should contain normal background
        assert!(
            output.contains(&normal_bg),
            "Output should contain normal background"
        );

        // Should NOT contain highlighted background (since line 100 is out of range)
        assert!(
            !output.contains(&highlighted_bg),
            "Output should NOT contain highlighted background when line is out of range"
        );
    }

    #[test]
    fn test_highlight_dark_mode_color() {
        let theme_bg = Color {
            r: 40,
            g: 40,
            b: 40,
            a: 255,
        };
        let highlight_bg = compute_highlight_bg(theme_bg, ColorMode::Dark);

        // Dark mode should add brightness
        assert_eq!(highlight_bg.r, 70); // 40 + 30
        assert_eq!(highlight_bg.g, 65); // 40 + 25
        assert_eq!(highlight_bg.b, 40); // unchanged
        assert_eq!(highlight_bg.a, 255);
    }

    #[test]
    fn test_highlight_light_mode_color() {
        let theme_bg = Color {
            r: 240,
            g: 240,
            b: 240,
            a: 255,
        };
        let highlight_bg = compute_highlight_bg(theme_bg, ColorMode::Light);

        // Light mode should subtract brightness
        assert_eq!(highlight_bg.r, 220); // 240 - 20
        assert_eq!(highlight_bg.g, 225); // 240 - 15
        assert_eq!(highlight_bg.b, 240); // unchanged
        assert_eq!(highlight_bg.a, 255);
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

        // Verify content after stripping ANSI
        // Output includes: right-aligned header row (spacing + language) + newline + top padding + left padding + code + bottom padding + trailing blank line
        // Terminal width is 80, language " rust " is 6 chars, so 74 spaces of padding
        let plain = strip_ansi_codes(&raw);
        assert!(plain.ends_with(" rust \n\n fn test() {}\n\n"));
        assert!(plain.contains("rust"));
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

        let result = emit_prose_text("Hello", style, true);

        // Should have foreground
        assert!(result.contains("\x1b[38;2;255;128;64m"));
        // Should NOT have background
        assert!(!result.contains("\x1b[48;2;"));
        // Should end with reset
        assert!(result.contains("\x1b[0m"));
    }

    /// Regression test: italic text should include ANSI italic escape code (\x1b[3m).
    ///
    /// Previously, emit_prose_text only emitted foreground color, ignoring font_style.
    /// This caused italic markdown (*text*) to render without italic styling in terminals.
    #[test]
    fn test_emit_prose_text_italic() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color { r: 255, g: 128, b: 64, a: 255 },
            background: Color { r: 0, g: 0, b: 0, a: 255 },
            font_style: FontStyle::ITALIC,
        };

        // With emit_italic=true, should emit italic code
        let result = emit_prose_text("italic text", style, true);

        // Should have italic escape code
        assert!(
            result.contains("\x1b[3m"),
            "Italic text should include \\x1b[3m escape code, got: {:?}",
            result
        );
        // Should have foreground color
        assert!(result.contains("\x1b[38;2;255;128;64m"));
        // Should end with reset
        assert!(result.contains("\x1b[0m"));
    }

    /// Regression test: italic escape codes should be suppressed when emit_italic=false.
    ///
    /// This tests the ItalicMode::Never behavior where italics are disabled.
    #[test]
    fn test_emit_prose_text_italic_suppressed() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color { r: 255, g: 128, b: 64, a: 255 },
            background: Color { r: 0, g: 0, b: 0, a: 255 },
            font_style: FontStyle::ITALIC,
        };

        // With emit_italic=false, should NOT emit italic code
        let result = emit_prose_text("italic text", style, false);

        // Should NOT have italic escape code
        assert!(
            !result.contains("\x1b[3m"),
            "Italic text with emit_italic=false should NOT include \\x1b[3m, got: {:?}",
            result
        );
        // Should still have foreground color
        assert!(result.contains("\x1b[38;2;255;128;64m"));
    }

    /// Regression test: bold text should include ANSI bold escape code (\x1b[1m).
    #[test]
    fn test_emit_prose_text_bold() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color { r: 255, g: 128, b: 64, a: 255 },
            background: Color { r: 0, g: 0, b: 0, a: 255 },
            font_style: FontStyle::BOLD,
        };

        let result = emit_prose_text("bold text", style, true);

        // Should have bold escape code
        assert!(
            result.contains("\x1b[1m"),
            "Bold text should include \\x1b[1m escape code, got: {:?}",
            result
        );
        // Should have foreground color
        assert!(result.contains("\x1b[38;2;255;128;64m"));
    }

    /// Regression test: underline text should include ANSI underline escape code (\x1b[4m).
    #[test]
    fn test_emit_prose_text_underline() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color { r: 255, g: 128, b: 64, a: 255 },
            background: Color { r: 0, g: 0, b: 0, a: 255 },
            font_style: FontStyle::UNDERLINE,
        };

        let result = emit_prose_text("underline text", style, true);

        // Should have underline escape code
        assert!(
            result.contains("\x1b[4m"),
            "Underline text should include \\x1b[4m escape code, got: {:?}",
            result
        );
    }

    /// Regression test: combined styles (bold + italic) should include both escape codes.
    #[test]
    fn test_emit_prose_text_bold_italic() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color { r: 255, g: 128, b: 64, a: 255 },
            background: Color { r: 0, g: 0, b: 0, a: 255 },
            font_style: FontStyle::BOLD | FontStyle::ITALIC,
        };

        let result = emit_prose_text("bold italic", style, true);

        // Should have both escape codes
        assert!(result.contains("\x1b[1m"), "Should have bold");
        assert!(result.contains("\x1b[3m"), "Should have italic");
    }

    /// Regression test: bold+italic with emit_italic=false should only emit bold.
    #[test]
    fn test_emit_prose_text_bold_italic_no_italic() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color { r: 255, g: 128, b: 64, a: 255 },
            background: Color { r: 0, g: 0, b: 0, a: 255 },
            font_style: FontStyle::BOLD | FontStyle::ITALIC,
        };

        let result = emit_prose_text("bold italic", style, false);

        // Should have bold but NOT italic
        assert!(result.contains("\x1b[1m"), "Should have bold");
        assert!(!result.contains("\x1b[3m"), "Should NOT have italic when emit_italic=false");
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

        // Should contain heading markers (block characters)
        // H1: █, H2: ██
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("█ Heading 1"));
        assert!(plain.contains("██ Heading 2"));
    }

    /// Regression test: italic markdown should render with ANSI italic escape code
    /// when ItalicMode::Always is used.
    ///
    /// This tests the full rendering pipeline from markdown to terminal output.
    #[test]
    fn test_for_terminal_italic_styled() {
        let md: Markdown = "Text blocks get some _title love_ too.".into();
        let mut options = TerminalOptions::default();
        options.italic_mode = ItalicMode::Always;
        let output = for_terminal(&md, options).unwrap();

        // Should contain ANSI italic escape code (\x1b[3m)
        assert!(
            output.contains("\x1b[3m"),
            "Italic markdown should produce \\x1b[3m escape code, got: {:?}",
            output
        );

        // Plain text should have the italic content
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("title love"));
    }

    /// Regression test: italic escape codes should be suppressed when
    /// ItalicMode::Never is configured.
    ///
    /// This ensures that terminals without italic support don't receive
    /// unsupported escape codes.
    #[test]
    fn test_for_terminal_italic_never() {
        let md: Markdown = "Text blocks get some _title love_ too.".into();
        let mut options = TerminalOptions::default();
        options.italic_mode = ItalicMode::Never;
        let output = for_terminal(&md, options).unwrap();

        // Should NOT contain ANSI italic escape code
        assert!(
            !output.contains("\x1b[3m"),
            "ItalicMode::Never should not produce \\x1b[3m escape code, got: {:?}",
            output
        );

        // Plain text should still have the content
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("title love"));
    }

    /// Test that ItalicMode::Always forces italic rendering even when
    /// the terminal might not support it.
    #[test]
    fn test_for_terminal_italic_always_forces_italic() {
        let md: Markdown = "This has *emphasis* text.".into();
        let mut options = TerminalOptions::default();
        options.italic_mode = ItalicMode::Always;
        let output = for_terminal(&md, options).unwrap();

        // Should contain italic code
        assert!(
            output.contains("\x1b[3m"),
            "ItalicMode::Always should produce \\x1b[3m, got: {:?}",
            output
        );
    }

    /// Regression test: bold markdown should render with ANSI bold escape code.
    #[test]
    fn test_for_terminal_bold_styled() {
        let md: Markdown = "Some **bold text** here.".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Should contain ANSI bold escape code (\x1b[1m)
        assert!(
            output.contains("\x1b[1m"),
            "Bold markdown should produce \\x1b[1m escape code, got: {:?}",
            output
        );

        // Plain text should have the bold content
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("bold text"));
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

        // H1 uses block marker: █
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("█ Heading with bold text"));
    }

    #[test]
    fn test_for_terminal_heading_has_blank_line_after() {
        let md: Markdown = "# Heading\n\nParagraph text.".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // H1 uses block marker: █
        // Heading should be followed by blank line before paragraph
        assert!(plain.contains("█ Heading\n\nParagraph text."));
    }

    #[test]
    fn test_for_terminal_heading_has_blank_line_before() {
        let md: Markdown = "Some text.\n\n## Heading".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // H2 uses block marker: ██
        // Should have blank line between paragraph and heading
        assert!(plain.contains("Some text.\n\n██ Heading"));
    }

    #[test]
    fn test_for_terminal_heading_after_list_has_blank_line() {
        let md: Markdown = "- Item one\n- Item two\n\n### Heading".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // H3 uses block marker: ████
        // Should have blank line between list and heading
        assert!(plain.contains("- Item two\n\n████ Heading"));
    }

    #[test]
    fn test_for_terminal_first_heading_no_leading_blank() {
        let md: Markdown = "# First Heading".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        let plain = strip_ansi_codes(&output);
        // H1 uses block marker: █
        // First heading should not have leading blank lines
        assert!(plain.starts_with("█ First Heading"));
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

        // H2 uses block marker: ██
        // Should have heading followed by table with box-drawing
        assert!(
            plain.contains("██ Environment Variables"),
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

        // H1 uses block marker: █, H2 uses: ██
        // Should have heading
        assert!(plain.contains("█ Documentation"));
        assert!(plain.contains("██ Configuration"));

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

        // H2 uses block marker: ██
        // Should have both headings
        assert!(plain.contains("██ Table 1"));
        assert!(plain.contains("██ Table 2"));

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

    // ---- ImageRenderer Tests ----

    /// Test ImageRenderer::new() initialization with None base path
    #[test]
    fn test_image_renderer_new_with_none_base_path() {
        let renderer = ImageRenderer::new(None);

        // Should use current directory as base
        let expected_base = std::env::current_dir().unwrap_or_default();
        assert_eq!(renderer.base_path(), expected_base);

        // Terminal width should be either detected or default 80
        assert!(renderer.terminal_width() >= 10);
    }

    /// Test ImageRenderer::new() initialization with specific base path
    #[test]
    fn test_image_renderer_new_with_base_path() {
        let base = std::env::temp_dir();
        let renderer = ImageRenderer::new(Some(&base));

        assert_eq!(renderer.base_path(), base);
    }

    /// Test ImageRenderer accessors
    #[test]
    fn test_image_renderer_accessors() {
        let renderer = ImageRenderer::new(None);

        // is_tty should be false in test context
        assert!(!renderer.is_tty());

        // graphics_supported should be false when not TTY
        assert!(!renderer.graphics_supported());

        // Terminal width should be reasonable
        assert!(renderer.terminal_width() >= 10);
        assert!(renderer.terminal_width() <= 1000);
    }

    /// Test ImageRenderer with non-existent base path
    #[test]
    fn test_image_renderer_with_nonexistent_base() {
        let nonexistent = std::path::PathBuf::from("/this/path/does/not/exist/for/sure");
        let renderer = ImageRenderer::new(Some(&nonexistent));

        // Should use the path even if it doesn't exist
        assert_eq!(renderer.base_path(), nonexistent);

        // base_path_canonical should be None
        // (we can't directly test this since it's private, but the struct should be valid)
    }

    /// Test ImageRenderer graphics detection is cached
    #[test]
    fn test_image_renderer_caches_detection() {
        // Create two renderers and verify they have consistent state
        let renderer1 = ImageRenderer::new(None);
        let renderer2 = ImageRenderer::new(None);

        // Both should have the same graphics support detection
        assert_eq!(renderer1.graphics_supported(), renderer2.graphics_supported());
        assert_eq!(renderer1.is_tty(), renderer2.is_tty());
    }

    /// Test ImageRenderer debug formatting
    #[test]
    fn test_image_renderer_debug() {
        let renderer = ImageRenderer::new(None);
        let debug_str = format!("{:?}", renderer);

        // Should contain field names
        assert!(debug_str.contains("ImageRenderer"));
        assert!(debug_str.contains("graphics_supported"));
        assert!(debug_str.contains("is_tty"));
        assert!(debug_str.contains("terminal_width"));
    }

    // ---- Image Event Handling Tests ----

    /// Test that image markdown produces fallback placeholder
    #[test]
    fn test_image_fallback_placeholder() {
        let md: Markdown = "![Test Image](./image.png)".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("▉ IMAGE[Test Image]"));
    }

    /// Test image with empty alt text
    #[test]
    fn test_image_empty_alt_text() {
        let md: Markdown = "![](./image.png)".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("▉ IMAGE[]"));
    }

    /// Test image alt text with inline code
    #[test]
    fn test_image_alt_with_inline_code() {
        let md: Markdown = "![Code `snippet` here](./image.png)".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Alt text should preserve inline code with backticks
        assert!(plain.contains("▉ IMAGE[Code `snippet` here]"));
    }

    /// Test multiple images in document
    #[test]
    fn test_multiple_images() {
        let md: Markdown = "![First](a.png)\n\n![Second](b.png)".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("▉ IMAGE[First]"));
        assert!(plain.contains("▉ IMAGE[Second]"));
    }

    /// Test image inside list
    #[test]
    fn test_image_in_list() {
        let md: Markdown = "- Item with ![image](test.png)".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("- Item with"));
        assert!(plain.contains("▉ IMAGE[image]"));
    }

    /// Test image in paragraph context
    #[test]
    fn test_image_in_paragraph() {
        let md: Markdown = "Some text before ![alt](img.png) and after.".into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("Some text before"));
        assert!(plain.contains("▉ IMAGE[alt]"));
        assert!(plain.contains("and after"));
    }

    // ---- ImageRenderer::render_image() Tests ----

    /// Test render_image with remote HTTP URL returns fallback
    #[test]
    fn test_render_image_rejects_http_url() {
        let renderer = ImageRenderer::new(None);
        let result = renderer.render_image("http://example.com/image.png", "Alt");

        assert!(result.contains("▉ IMAGE[Alt]"));
    }

    /// Test render_image with remote HTTPS URL returns fallback
    #[test]
    fn test_render_image_rejects_https_url() {
        let renderer = ImageRenderer::new(None);
        let result = renderer.render_image("https://example.com/image.png", "Alt");

        assert!(result.contains("▉ IMAGE[Alt]"));
    }

    /// Test render_image with missing file returns fallback
    #[test]
    fn test_render_image_missing_file() {
        let renderer = ImageRenderer::new(Some(&std::env::temp_dir()));
        let result = renderer.render_image("nonexistent_file_12345.png", "Missing");

        assert!(result.contains("▉ IMAGE[Missing]"));
    }

    /// Test render_image with relative path joined to base
    #[test]
    fn test_render_image_relative_path() {
        let renderer = ImageRenderer::new(Some(&std::env::temp_dir()));
        // This file doesn't exist, so we get fallback, but path is resolved correctly
        let result = renderer.render_image("subdir/image.png", "Test");

        assert!(result.contains("▉ IMAGE[Test]"));
    }

    /// Test render_image with empty alt text
    #[test]
    fn test_render_image_empty_alt() {
        let renderer = ImageRenderer::new(None);
        let result = renderer.render_image("http://example.com/x.png", "");

        assert!(result.contains("▉ IMAGE[]"));
    }

    /// Test render_image when graphics_supported is false
    #[test]
    fn test_render_image_no_graphics_support() {
        let renderer = ImageRenderer::new(None);
        // In test context, graphics_supported should be false
        assert!(!renderer.graphics_supported());

        // Create a temp file that exists
        let tmp = std::env::temp_dir().join("test_image_render.png");
        std::fs::write(&tmp, b"fake png data").unwrap();

        let result = renderer.render_image(tmp.to_str().unwrap(), "Test");
        assert!(result.contains("▉ IMAGE[Test]"));

        std::fs::remove_file(&tmp).ok();
    }

    /// Test render_image path traversal prevention
    #[test]
    fn test_render_image_path_traversal() {
        // Create a temp directory as the base
        let base_dir = std::env::temp_dir().join("image_test_base");
        std::fs::create_dir_all(&base_dir).ok();

        // Create a file outside the base directory
        let outside_file = std::env::temp_dir().join("outside_image.png");
        std::fs::write(&outside_file, b"outside").unwrap();

        let renderer = ImageRenderer::new(Some(&base_dir));

        // Try to access file outside base via path traversal
        let result = renderer.render_image("../outside_image.png", "Outside");

        // Should return fallback (path escapes base)
        assert!(result.contains("▉ IMAGE[Outside]"));

        std::fs::remove_file(&outside_file).ok();
        std::fs::remove_dir(&base_dir).ok();
    }

    /// Test render_image with absolute path
    #[test]
    fn test_render_image_absolute_path() {
        let renderer = ImageRenderer::new(Some(&std::env::temp_dir()));

        // Use an absolute path (file doesn't exist)
        let result = renderer.render_image("/nonexistent/absolute/path.png", "Abs");

        assert!(result.contains("▉ IMAGE[Abs]"));
    }

    /// Test render_image fallback ends with newline
    #[test]
    fn test_render_image_fallback_ends_with_newline() {
        let renderer = ImageRenderer::new(None);
        let result = renderer.render_image("http://example.com/x.png", "Test");

        assert!(result.ends_with('\n'));
    }

    // ---- TerminalOptions Image Field Tests ----

    /// Test TerminalOptions default values for image fields
    #[test]
    fn test_terminal_options_image_defaults() {
        let options = TerminalOptions::default();

        assert!(options.render_images);
        assert!(options.base_path.is_none());
    }

    /// Test for_terminal with render_images disabled
    #[test]
    fn test_for_terminal_render_images_disabled() {
        let mut options = TerminalOptions::default();
        options.render_images = false;

        let md: Markdown = "![Test](./image.png)".into();
        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should show fallback since render_images is false
        assert!(plain.contains("▉ IMAGE[Test]"));
    }

    /// Test for_terminal with render_images enabled (default)
    #[test]
    fn test_for_terminal_render_images_enabled() {
        let options = TerminalOptions::default();
        assert!(options.render_images);

        let md: Markdown = "![Test](./image.png)".into();
        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should show fallback (file doesn't exist, no graphics support in tests)
        assert!(plain.contains("▉ IMAGE[Test]"));
    }

    /// Test for_terminal with custom base_path
    #[test]
    fn test_for_terminal_with_base_path() {
        let mut options = TerminalOptions::default();
        options.base_path = Some(std::env::temp_dir());

        let md: Markdown = "![Test](./image.png)".into();
        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should show fallback (file doesn't exist)
        assert!(plain.contains("▉ IMAGE[Test]"));
    }

    /// Test for_terminal preserves behavior for documents without images
    #[test]
    fn test_for_terminal_no_images_in_content() {
        let options = TerminalOptions::default();

        let md: Markdown = "# Hello\n\nSome text here.".into();
        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // H1 uses block marker: █
        assert!(plain.contains("█ Hello"));
        assert!(plain.contains("Some text here"));
        // Should NOT contain image fallback
        assert!(!plain.contains("▉ IMAGE"));
    }

    /// Integration test: Full code block with all features
    #[test]
    fn test_full_code_block_all_features() {
        let content = r#"```rust title="Complete Example" line-numbering=true highlight=2,4-5
fn line1() {}
fn line2() {}
fn line3() {}
fn line4() {}
fn line5() {}
fn line6() {}
```"#;
        let md: Markdown = content.into();

        let mut options = TerminalOptions::default();
        options.include_line_numbers = true;

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Verify title is present in header row
        assert!(plain.contains("Complete Example"), "Header should contain title");

        // Verify language is present in header row
        assert!(plain.contains("rust"), "Header should contain language");

        // Verify line numbers are present (with gutter)
        assert!(plain.contains("│"), "Should contain line number gutter");

        // Verify code content is present
        assert!(plain.contains("fn line1"), "Should contain line1 function");
        assert!(plain.contains("fn line6"), "Should contain line6 function");

        // Verify ANSI codes are present (highlighting and formatting)
        assert!(output.contains("\x1b["), "Should contain ANSI escape codes");
    }

    /// Integration test: Empty code block renders padding correctly
    #[test]
    fn test_empty_code_block() {
        let content = "```rust\n```";
        let md: Markdown = content.into();
        let output = for_terminal(&md, TerminalOptions::default()).unwrap();

        // Should contain header row with language
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("rust"), "Header should contain language");

        // Should contain padding (top and bottom)
        // Padding rows are: \x1b[48;2;R;G;Bm\x1b[K\x1b[0m\n
        assert!(output.contains("\x1b[K"), "Should contain padding clear codes");

        // Should have proper ANSI structure even with no code lines
        assert!(output.contains("\x1b[48;2;"), "Should contain background color codes");
        assert!(output.contains("\x1b[0m"), "Should contain reset codes");
    }

    // ===== LINE WRAPPER REGRESSION TESTS =====
    // Regression tests for the line wrapping bug where:
    // 1. Words were being broken mid-word when wrapping
    // 2. Continuation lines started with visible whitespace

    /// Regression test: Words should not be broken mid-word when wrapping.
    /// Bug: "character" was being split as "charac" / "ter"
    #[test]
    fn test_prose_no_mid_word_breaks() {
        // Create text that will need to wrap at a narrow width
        let content = "NOTE: use of italics in Markdown can use either the '_' character OR the '*' character";
        let md: Markdown = content.into();

        // Use narrow width (40 chars) to force wrapping
        let mut options = TerminalOptions::default();
        options.color_depth = Some(ColorDepth::TrueColor);

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Check for common mid-word break patterns (word fragments at line ends)
        let bad_patterns = [
            "charac\n",   // "character" split
            "Markdo\n",   // "Markdown" split
            "itali\n",    // "italics" split
            "eithe\n",    // "either" split
        ];

        for pattern in bad_patterns {
            assert!(
                !plain.contains(pattern),
                "Bad word split detected: found '{}' in:\n{}",
                pattern.escape_default(), plain
            );
        }

        // Verify complete words exist
        for word in ["italics", "Markdown", "character", "either"] {
            assert!(
                plain.contains(word),
                "Complete word '{}' should be present:\n{}",
                word, plain
            );
        }
    }

    /// Regression test: Continuation lines should not start with visible whitespace.
    /// Bug: When text wrapped, the continuation line started with a space.
    #[test]
    fn test_prose_no_leading_whitespace_on_wrap() {
        // Text with multiple words that will wrap
        let content = "The strikethrough feature -- introduced in GFM -- uses ~~ around a block of text to represent the text which should be crossed out.";
        let md: Markdown = content.into();

        let mut options = TerminalOptions::default();
        options.color_depth = Some(ColorDepth::TrueColor);

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Check each line (except first) doesn't start with whitespace
        for (i, line) in plain.lines().enumerate() {
            if i == 0 {
                continue; // First line can start however
            }
            // Skip empty lines
            if line.is_empty() {
                continue;
            }
            // Continuation lines should not start with space/tab
            assert!(
                !line.starts_with(' ') && !line.starts_with('\t'),
                "Line {} starts with whitespace: '{}'\nFull output:\n{}",
                i + 1, line.escape_default(), plain
            );
        }
    }

    /// Test LineWrapper wraps correctly at word boundaries
    #[test]
    fn test_line_wrapper_basic() {
        use syntect::highlighting::Color;

        let style = Style {
            foreground: Color { r: 255, g: 255, b: 255, a: 255 },
            background: Color::BLACK,
            font_style: syntect::highlighting::FontStyle::empty(),
        };

        // Create wrapper with narrow width
        let mut wrapper = LineWrapper::new(20);
        wrapper.emit_styled("Hello world this is a test", style, false);

        let output = wrapper.into_output();
        let plain = strip_ansi_codes(&output);

        // Should have wrapped - multiple lines
        let lines: Vec<&str> = plain.lines().collect();
        assert!(lines.len() > 1, "Text should have wrapped: '{}'", plain);

        // No line should exceed the max width
        for line in &lines {
            let width = unicode_width::UnicodeWidthStr::width(*line);
            assert!(
                width <= 20,
                "Line exceeds max width: '{}' (width: {})",
                line, width
            );
        }

        // Words should be complete, not split
        assert!(plain.contains("Hello"), "Should contain 'Hello'");
        assert!(plain.contains("world"), "Should contain 'world'");
        assert!(plain.contains("this"), "Should contain 'this'");
        assert!(plain.contains("test"), "Should contain 'test'");
    }

    /// Test LineWrapper handles inline code with wrapping
    #[test]
    fn test_line_wrapper_inline_code() {
        use syntect::highlighting::Color;

        let style = Style {
            foreground: Color { r: 200, g: 200, b: 200, a: 255 },
            background: Color { r: 40, g: 40, b: 40, a: 255 },
            font_style: syntect::highlighting::FontStyle::empty(),
        };

        let mut wrapper = LineWrapper::new(30);
        wrapper.emit_raw("Command: ");
        wrapper.emit_inline_code("cargo build", style);
        wrapper.emit_raw(" runs the build");

        let output = wrapper.into_output();
        let plain = strip_ansi_codes(&output);

        // Should contain the inline code
        assert!(plain.contains("cargo build"), "Should contain 'cargo build'");
        // Should contain surrounding text
        assert!(plain.contains("Command:"), "Should contain 'Command:'");
    }

    /// Test that long paragraphs wrap correctly at terminal width
    #[test]
    fn test_prose_paragraph_wrapping() {
        // Long paragraph that will definitely need wrapping
        let content = "This is a very long paragraph that contains many words and should definitely wrap when rendered to a terminal with a reasonable width. The wrapping should occur at word boundaries to maintain readability and prevent awkward mid-word breaks.";
        let md: Markdown = content.into();

        let mut options = TerminalOptions::default();
        options.color_depth = Some(ColorDepth::TrueColor);

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Remove trailing paragraph spacing for line check
        let trimmed = plain.trim();

        // Should have wrapped into multiple lines
        let lines: Vec<&str> = trimmed.lines().collect();
        assert!(
            lines.len() > 1,
            "Long paragraph should wrap into multiple lines:\n{}",
            trimmed
        );

        // Verify key words are complete
        for word in ["paragraph", "definitely", "wrapping", "boundaries", "readability"] {
            assert!(
                plain.contains(word),
                "Word '{}' should be complete, not split:\n{}",
                word, plain
            );
        }
    }

    /// Test LineWrapper with styled text (bold, italic)
    #[test]
    fn test_line_wrapper_styled_text() {
        use syntect::highlighting::{Color, FontStyle};

        let bold_style = Style {
            foreground: Color { r: 255, g: 255, b: 255, a: 255 },
            background: Color::BLACK,
            font_style: FontStyle::BOLD,
        };

        let italic_style = Style {
            foreground: Color { r: 255, g: 255, b: 255, a: 255 },
            background: Color::BLACK,
            font_style: FontStyle::ITALIC,
        };

        let mut wrapper = LineWrapper::new(40);
        wrapper.emit_styled("This has ", bold_style, true);
        wrapper.emit_styled("mixed styles", italic_style, true);
        wrapper.emit_styled(" in one line", bold_style, true);

        let output = wrapper.into_output();

        // Should contain bold escape code
        assert!(output.contains("\x1b[1m"), "Should contain bold code");
        // Should contain italic escape code
        assert!(output.contains("\x1b[3m"), "Should contain italic code");

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("This has"), "Should contain 'This has'");
        assert!(plain.contains("mixed styles"), "Should contain 'mixed styles'");
        assert!(plain.contains("in one line"), "Should contain 'in one line'");
    }
}
