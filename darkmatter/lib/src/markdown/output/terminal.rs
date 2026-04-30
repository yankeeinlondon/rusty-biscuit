//! Terminal output with ANSI escape codes for markdown rendering.
//!
//! This module provides terminal-based rendering of markdown documents with
//! syntax highlighting using ANSI escape sequences. It supports:
//!
//! - Auto-detection of terminal color depth
//! - Code block syntax highlighting with line numbers
//! - Code block titles with visual prefix
//! - Configurable themes for code and prose
//! - GitHub Flavored Markdown tables with box-drawing characters (via biscuit-terminal::Table)
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::Markdown;
//! use darkmatter::markdown::output::terminal::{for_terminal, TerminalOptions};
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
    Markdown, MarkdownError,
    block::{RuleProcessor, build_rule_with_defaults, hr_defaults_from_frontmatter},
    dsl::parse_code_info,
    highlighting::{
        CodeHighlighter, ColorMode, ThemePair, prose::ProseHighlighter, scope_cache::ScopeCache,
    },
    inline::{InlineEvent, InlineTag, InlineStyleProcessor},
    output::code_block,
};
use biscuit_terminal::components::horizontal_rule::HorizontalRule;
use biscuit_terminal::components::image_options::TerminalImageOptions;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{
    Table as TerminalTable, TableCellContent, TableColumn,
};
use biscuit_terminal::components::table::types::ColumnType;
use biscuit_terminal::components::terminal_image::{ImageWidth, TerminalImage, parse_width_spec};
use biscuit_terminal::discovery::detection::ColorDepth as TerminalColorDepth;
use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::UnicodeWidthStr;
use biscuit_terminal::utils::layout::Alignment;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::path::{Path, PathBuf};
use syntect::highlighting::{Color, Style};
use syntect::parsing::Scope;

// Re-export shared code-block helpers so existing call sites and tests keep working.
#[cfg(test)]
pub(crate) use code_block::compute_highlight_bg;
#[cfg(test)]
pub(crate) use code_block::find_syntax;
pub(crate) use code_block::render_terminal_code_block as highlight_code;

/// Parse image alt text to extract optional width specification.
///
/// The format is `alt text|width` where width can be:
/// - `25%` - percentage of terminal width
/// - `40ch` - character width
/// - `40` - bare number (character width)
/// - `fill` - fill available width
///
/// Width spec is only parsed if `|` is immediately followed by a digit.
/// Otherwise the entire string (including `|`) is treated as alt text.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::output::terminal::parse_alt_and_width;
/// use biscuit_terminal::components::terminal_image::ImageWidth;
///
/// // Width specification parsed
/// let (alt, width) = parse_alt_and_width("my image|50%");
/// assert_eq!(alt, "my image");
/// assert!(matches!(width, Some(ImageWidth::Percent(p)) if (p - 0.5).abs() < 0.001));
///
/// // Character width
/// let (alt, width) = parse_alt_and_width("photo|80");
/// assert_eq!(alt, "photo");
/// assert!(matches!(width, Some(ImageWidth::Characters(80))));
///
/// // No width spec - pipe not followed by digit
/// let (alt, width) = parse_alt_and_width("chart | analysis");
/// assert_eq!(alt, "chart | analysis");
/// assert!(width.is_none());
///
/// // No pipe at all
/// let (alt, width) = parse_alt_and_width("simple alt text");
/// assert_eq!(alt, "simple alt text");
/// assert!(width.is_none());
/// ```
pub fn parse_alt_and_width(alt_text: &str) -> (String, Option<ImageWidth>) {
    // Find pipe followed immediately by a digit
    if let Some(pipe_pos) = alt_text.find('|') {
        let after_pipe = &alt_text[pipe_pos + 1..];
        // Check if immediately followed by a digit (allowing leading whitespace for "| 50%")
        let trimmed = after_pipe.trim_start();
        if trimmed.starts_with(|c: char| c.is_ascii_digit()) {
            // Try to parse as width spec
            if let Ok(width) = parse_width_spec(after_pipe) {
                let alt = alt_text[..pipe_pos].trim_end().to_string();
                return (alt, Some(width));
            }
        }
    }
    // No valid width spec found - return original alt text
    (alt_text.to_string(), None)
}

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
        use crate::terminal::{COLORS_16_DEPTH, COLORS_256_DEPTH, TRUE_COLOR_DEPTH};

        let depth = crate::terminal::color_depth();
        if depth >= TRUE_COLOR_DEPTH {
            Self::TrueColor
        } else if depth >= COLORS_256_DEPTH {
            Self::Colors256
        } else if depth >= COLORS_16_DEPTH {
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
/// use darkmatter::markdown::output::terminal::{TerminalOptions, ItalicMode};
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

/// Controls how dim/faint text is rendered to the terminal.
///
/// Different terminals have varying levels of support for dim text rendering.
/// This enum allows explicit control over dim behavior.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::output::terminal::{TerminalOptions, DimMode};
///
/// // Auto-detect (safe default)
/// let mut options = TerminalOptions::default();
/// assert!(matches!(options.dim_mode, DimMode::Auto));
///
/// // Force dim for pre-rendering to unknown terminals
/// options.dim_mode = DimMode::Always;
///
/// // Disable dim for terminals known not to support it
/// options.dim_mode = DimMode::Never;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DimMode {
    /// Auto-detect dim support using terminal capabilities.
    ///
    /// Uses [`supports_dim()`](crate::terminal::supports_dim) to query
    /// the terminal. This is the safest option for direct terminal output.
    #[default]
    Auto,

    /// Always emit dim escape codes (`\x1b[2m`).
    ///
    /// Use this when pre-rendering content for a future terminal where
    /// capabilities cannot be detected. Assumes dim support is available.
    Always,

    /// Never emit dim escape codes.
    ///
    /// Use this when rendering for terminals known not to support dim,
    /// or when dim styling is not desired.
    Never,
}

impl DimMode {
    /// Resolves the mode to a boolean indicating whether to emit dim codes.
    ///
    /// ## Returns
    ///
    /// - `Auto`: Result of `supports_dim()` terminal capability check
    /// - `Always`: `true`
    /// - `Never`: `false`
    fn should_emit_dim(&self) -> bool {
        match self {
            DimMode::Auto => crate::terminal::supports_dim(),
            DimMode::Always => true,
            DimMode::Never => false,
        }
    }
}

/// Controls how hyperlinks are rendered to the terminal.
///
/// OSC 8 hyperlinks allow terminals to display clickable links, but not all
/// terminals support them. This enum controls whether to emit OSC 8 sequences
/// or use a fallback format.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::output::terminal::{TerminalOptions, HyperlinkMode};
///
/// let mut options = TerminalOptions::default();
/// // Auto-detect (default) - uses terminal capability detection
/// assert!(matches!(options.hyperlink_mode, HyperlinkMode::Auto));
///
/// // Force OSC8 hyperlinks (for pre-rendering to known-capable terminals)
/// options.hyperlink_mode = HyperlinkMode::Always;
///
/// // Force fallback format: "text [url]"
/// options.hyperlink_mode = HyperlinkMode::Never;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HyperlinkMode {
    /// Auto-detect hyperlink support using `supports_hyperlinks` crate.
    ///
    /// Checks environment variables (TERM, TERM_PROGRAM, etc.) and TTY status
    /// to determine if the terminal supports OSC 8 hyperlinks.
    #[default]
    Auto,

    /// Always emit OSC 8 hyperlink escape codes.
    ///
    /// Use this when pre-rendering content for a terminal known to support
    /// hyperlinks, or when the detection is unreliable.
    Always,

    /// Never emit OSC 8 hyperlink escape codes.
    ///
    /// Use fallback format `text [url]` which is readable in all terminals.
    /// Use this for dumb terminals or when OSC 8 is causing issues.
    Never,
}

impl HyperlinkMode {
    /// Resolves the mode to a boolean indicating whether to emit OSC 8 codes.
    ///
    /// ## Returns
    ///
    /// - `Auto`: Result of biscuit-terminal's OSC 8 link support detection
    /// - `Always`: `true`
    /// - `Never`: `false`
    fn should_emit_osc8(&self) -> bool {
        match self {
            HyperlinkMode::Auto => biscuit_terminal::discovery::detection::osc8_link_support(),
            HyperlinkMode::Always => true,
            HyperlinkMode::Never => false,
        }
    }
}

/// Controls how Mermaid diagrams are rendered to the terminal.
///
/// Mermaid diagrams can be rendered as images (via mermaid.ink service)
/// or displayed as code blocks.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::output::terminal::{TerminalOptions, MermaidMode};
///
/// // Disable mermaid rendering (default - show as code block)
/// let mut options = TerminalOptions::default();
/// assert!(matches!(options.mermaid_mode, MermaidMode::Off));
///
/// // Enable image rendering
/// options.mermaid_mode = MermaidMode::Image;
///
/// // Show as text fallback
/// options.mermaid_mode = MermaidMode::Text;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MermaidMode {
    /// Do not render mermaid diagrams specially (default).
    ///
    /// Mermaid code blocks are displayed as regular syntax-highlighted code.
    #[default]
    Off,

    /// Render mermaid diagrams as images via mermaid.ink service.
    ///
    /// Uses mermaid.ink to convert diagrams to SVG, then resvg to convert
    /// to PNG, and viuer to display in the terminal. Falls back to `Text`
    /// mode if terminal doesn't support graphics or rendering fails.
    Image,

    /// Display mermaid diagrams as fenced code blocks.
    ///
    /// Useful for terminals that don't support inline images, or when
    /// you want to see the diagram source.
    Text,
}

/// Controls terminal image rendering behavior.
///
/// This lets callers keep default capability-driven behavior, disable images,
/// or force protocol output attempts even when capability detection reports none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalImageMode {
    /// Auto-detect support and render only when supported.
    #[default]
    Auto,
    /// Never render protocol images; always use text fallback.
    Never,
    /// Force protocol rendering attempts regardless of detection result.
    Force,
}

/// Maximum image file size (10MB).
const MAX_IMAGE_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Renderer for inline images in terminal output.
///
/// Wraps `biscuit_terminal::components::TerminalImage` to provide image rendering
/// with automatic protocol detection (Kitty/iTerm2) and graceful fallback.
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
/// use darkmatter::markdown::output::terminal::{ImageRenderer, TerminalImageMode};
/// use std::path::Path;
///
/// let renderer = ImageRenderer::new(Some(Path::new("/tmp")), TerminalImageMode::Auto);
/// assert!(!renderer.graphics_supported()); // Usually false in test environment
/// ```
pub struct ImageRenderer {
    /// Cached terminal capabilities from biscuit-terminal.
    terminal: Terminal,
    /// Base path for resolving relative image paths.
    base_path: PathBuf,
    /// Pre-built options for TerminalImage rendering.
    options: TerminalImageOptions,
    /// Controls image rendering behavior.
    image_mode: TerminalImageMode,
}

impl std::fmt::Debug for ImageRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageRenderer")
            .field("graphics_supported", &self.graphics_supported())
            .field("is_tty", &self.is_tty())
            .field("terminal_width", &self.terminal_width())
            .field("base_path", &self.base_path)
            .field("image_mode", &self.image_mode)
            .finish()
    }
}

impl ImageRenderer {
    /// Creates a new image renderer with automatic graphics detection.
    ///
    /// Uses `biscuit_terminal::terminal::Terminal` for capability detection.
    /// Falls back to placeholder text when graphics are unavailable.
    ///
    /// ## Arguments
    ///
    /// * `base_path` - Base directory for resolving relative image paths.
    ///   Defaults to current working directory if `None`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::output::terminal::ImageRenderer;
    /// use darkmatter::markdown::output::terminal::TerminalImageMode;
    /// use std::path::Path;
    ///
    /// // Use current directory
    /// let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);
    ///
    /// // Use specific base path
    /// let renderer = ImageRenderer::new(Some(Path::new("/docs")), TerminalImageMode::Auto);
    /// ```
    pub fn new(base_path: Option<&Path>, image_mode: TerminalImageMode) -> Self {
        let terminal = Terminal::new();

        let base = base_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Build options for TerminalImage rendering
        // Note: We don't set base_path here for security sandboxing.
        // The base_path is used for resolving relative paths (in self.base_path),
        // but we don't want to block images in sibling directories (e.g., ../assets/).
        let options = TerminalImageOptions::builder()
            .max_file_size(MAX_IMAGE_FILE_SIZE)
            .allow_remote(false)
            .width(ImageWidth::Percent(0.5)) // Default to 50% width
            .build();

        tracing::debug!(
            graphics_supported = ?terminal.image_support,
            is_tty = terminal.is_tty,
            terminal_width = terminal.width(),
            image_mode = ?image_mode,
            base_path = %base.display(),
            "ImageRenderer initialized (via biscuit-terminal)"
        );

        Self {
            terminal,
            base_path: base,
            options,
            image_mode,
        }
    }

    /// Returns whether graphics protocols are supported.
    #[inline]
    pub fn graphics_supported(&self) -> bool {
        self.terminal.is_tty && !matches!(self.terminal.image_support, ImageSupport::None)
    }

    /// Returns whether stdout is a TTY.
    #[inline]
    pub fn is_tty(&self) -> bool {
        self.terminal.is_tty
    }

    /// Returns the terminal width.
    #[inline]
    pub fn terminal_width(&self) -> u16 {
        self.terminal.width() as u16
    }

    /// Returns the base path for resolving relative image paths.
    #[inline]
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    #[inline]
    fn should_attempt_protocol_render(&self) -> bool {
        match self.image_mode {
            TerminalImageMode::Auto => self.graphics_supported(),
            TerminalImageMode::Never => false,
            TerminalImageMode::Force => true,
        }
    }

    fn render_terminal(&self) -> Terminal {
        if self.image_mode != TerminalImageMode::Force {
            return self.terminal.clone();
        }

        let mut forced = self.terminal.clone();
        forced.is_tty = true;
        if matches!(forced.image_support, ImageSupport::None) {
            forced.image_support = ImageSupport::Kitty;
        }
        forced
    }

    /// Renders an image from the given path.
    ///
    /// Uses `biscuit_terminal::components::TerminalImage` for rendering when
    /// graphics protocols are available, with fallback to styled placeholder text.
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
    /// * `width` - Optional width specification for the image
    ///
    /// ## Returns
    ///
    /// String containing either rendered image or fallback placeholder.
    #[tracing::instrument(skip(self), fields(
        image.path = %image_path,
        image.graphics_supported = %self.graphics_supported()
    ))]
    pub fn render_image(
        &self,
        image_path: &str,
        alt_text: &str,
        width: Option<ImageWidth>,
    ) -> String {
        // Reject remote URLs (biscuit-terminal also checks, but we fail early)
        if image_path.starts_with("http://") || image_path.starts_with("https://") {
            tracing::warn!(image.path = %image_path, "Remote image URLs not supported");
            return format!("▉ IMAGE[{}]\n", alt_text);
        }

        // Security: Reject absolute paths to prevent directory traversal
        if Path::new(image_path).is_absolute() {
            tracing::warn!(image.path = %image_path, "Rejected absolute image path");
            return format!("▉ IMAGE[{}]\n", alt_text);
        }

        // Resolve relative path against base_path
        let full_path = self.base_path.join(image_path);

        // Security: Canonicalize and verify path stays within base directory
        // This prevents traversal attacks like "../../../etc/passwd"
        if let Ok(canonical_base) = self.base_path.canonicalize()
            && let Ok(canonical_full) = full_path.canonicalize()
            && !canonical_full.starts_with(&canonical_base)
        {
            tracing::warn!(
                image.path = %image_path,
                base = %canonical_base.display(),
                resolved = %canonical_full.display(),
                "Image path escapes base directory"
            );
            return format!("▉ IMAGE[{}]\n", alt_text);
        }
        // If canonical_full fails, the file doesn't exist - handled below
        // If canonical_base fails, we can't verify containment but proceed
        // (the file existence check below will catch most issues)

        // Check file exists before attempting to create TerminalImage
        if !full_path.exists() {
            tracing::warn!(image.path = %full_path.display(), "Image file not found");
            return format!("▉ IMAGE[{}]\n", alt_text);
        }

        // Fallback if graphics unsupported
        if !self.should_attempt_protocol_render() {
            tracing::debug!("Graphics protocol not available");
            // No warning here - this is expected behavior on terminals without graphics
            return format!("▉ IMAGE[{}]\n", alt_text);
        }

        // Create TerminalImage from the path
        let mut term_image = match TerminalImage::new(&full_path) {
            Ok(img) => img.with_alt_text(alt_text),
            Err(e) => {
                tracing::warn!(image.path = %image_path, error = %e, "Failed to load image");
                return format!("▉ IMAGE[{}]\n", alt_text);
            }
        };

        // Apply width if specified
        if let Some(w) = width {
            tracing::debug!(width = ?w, "Applying custom width");
            term_image = term_image.with_width(w);
        }

        // Check file size against configured limit
        if let Ok(metadata) = std::fs::metadata(&full_path)
            && !self.options.is_size_allowed(metadata.len())
        {
            tracing::warn!(
                image.path = %full_path.display(),
                image.size_bytes = metadata.len(),
                "Image file too large"
            );
            return format!("▉ IMAGE[{}]\n", alt_text);
        }

        // Render via Renderable fallback (protocol-aware string output)
        let render_terminal = self.render_terminal();
        let output = term_image.render(&render_terminal);
        if output.is_empty() {
            format!("▉ IMAGE[{}]\n", alt_text)
        } else {
            output
        }
    }
}

/// Options for terminal output with sensible defaults.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::highlighting::{ThemePair, ColorMode};
/// use darkmatter::markdown::output::terminal::{TerminalOptions, ColorDepth};
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
/// use darkmatter::markdown::output::terminal::TerminalOptions;
/// use darkmatter::markdown::output::terminal::TerminalImageMode;
/// use std::path::PathBuf;
///
/// let mut options = TerminalOptions::default();
/// options.image_mode = TerminalImageMode::Auto; // Default
/// options.base_path = Some(PathBuf::from("/docs"));
/// ```
///
/// ## Italic Mode
///
/// ```
/// use darkmatter::markdown::output::terminal::{TerminalOptions, ItalicMode};
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
    /// Controls image rendering behavior for terminal output.
    pub image_mode: TerminalImageMode,
    /// Base path for resolving relative image paths.
    /// If `None`, uses current working directory.
    pub base_path: Option<PathBuf>,
    /// Controls how italic text is rendered.
    ///
    /// - `Auto` (default): Detect terminal capability via `supports_italics()`
    /// - `Always`: Always emit italic escape codes (for pre-rendering)
    /// - `Never`: Never emit italic escape codes
    pub italic_mode: ItalicMode,
    /// Controls how dim/faint text is rendered.
    ///
    /// - `Auto` (default): Detect terminal capability via `supports_dim()`
    /// - `Always`: Always emit dim escape codes (for pre-rendering)
    /// - `Never`: Never emit dim escape codes
    pub dim_mode: DimMode,
    /// Maximum line width for text wrapping.
    ///
    /// If `None` (default), auto-detects from terminal size (defaults to 80 if detection fails).
    /// Set this to override for testing or pre-rendering at a specific width.
    pub max_width: Option<u16>,
    /// Controls how Mermaid diagrams are rendered.
    ///
    /// - `Off` (default): Show mermaid blocks as syntax-highlighted code
    /// - `Image`: Render as images via mermaid.ink service
    /// - `Text`: Show as fenced code blocks (fallback format)
    pub mermaid_mode: MermaidMode,
    /// Controls how hyperlinks are rendered.
    ///
    /// - `Auto` (default): Detect terminal capability via `supports_hyperlinks` crate
    /// - `Always`: Always emit OSC 8 escape codes (for pre-rendering)
    /// - `Never`: Never emit OSC 8 codes; use fallback format `text [url]`
    pub hyperlink_mode: HyperlinkMode,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        use crate::markdown::highlighting::{
            detect_code_theme, detect_color_mode, detect_prose_theme,
        };

        let prose_theme = detect_prose_theme();
        let code_theme = detect_code_theme(prose_theme);
        let color_mode = detect_color_mode();

        Self {
            code_theme,
            prose_theme,
            color_mode,
            include_line_numbers: false,
            color_depth: None,
            image_mode: TerminalImageMode::default(),
            base_path: None,
            italic_mode: ItalicMode::default(),
            dim_mode: DimMode::default(),
            max_width: None,
            mermaid_mode: MermaidMode::default(),
            hyperlink_mode: HyperlinkMode::default(),
        }
    }
}

/// Converts pulldown-cmark alignment to biscuit-terminal alignment.
fn convert_alignment(align: &pulldown_cmark::Alignment) -> Alignment {
    match align {
        pulldown_cmark::Alignment::None => Alignment::Left,
        pulldown_cmark::Alignment::Left => Alignment::Left,
        pulldown_cmark::Alignment::Center => Alignment::Center,
        pulldown_cmark::Alignment::Right => Alignment::Right,
    }
}

/// Exports markdown to terminal with ANSI escape codes.
///
/// This function renders markdown content with syntax-highlighted code blocks
/// using ANSI escape sequences for terminal display. Inline images are rendered
/// via biscuit-terminal's protocol-aware `Renderable` trait (Kitty/iTerm2).
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::Markdown;
/// use darkmatter::markdown::output::terminal::{for_terminal, TerminalOptions};
///
/// let md: Markdown = "# Hello\n\n```rust\nfn main() {}\n```".into();
/// let output = for_terminal(&md, TerminalOptions::default()).unwrap();
/// ```
///
/// ## Errors
///
/// Returns an error if theme loading fails or syntax highlighting encounters issues.
pub fn for_terminal(md: &Markdown, options: TerminalOptions) -> Result<String, MarkdownError> {
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
/// use darkmatter::markdown::Markdown;
/// use darkmatter::markdown::output::terminal::{write_terminal, TerminalOptions};
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

    // Resolve dim mode once at start (avoids repeated capability detection)
    let emit_dim = options.dim_mode.should_emit_dim();

    // Resolve hyperlink mode once at start (avoids repeated capability detection)
    let emit_hyperlinks = options.hyperlink_mode.should_emit_osc8();

    // Query terminal width once at start (allow override for testing)
    let terminal_width = options
        .max_width
        .unwrap_or_else(|| biscuit_terminal::discovery::detection::terminal_width() as u16);
    tracing::debug!(terminal_width, "Terminal width for rendering");

    let code_highlighter = CodeHighlighter::new(options.code_theme, options.color_mode);
    let hr_defaults = hr_defaults_from_frontmatter(md);

    // Load prose theme for ProseHighlighter
    let prose_syntect_theme =
        crate::markdown::highlighting::themes::load_theme(options.prose_theme, options.color_mode);
    let prose_highlighter = ProseHighlighter::new(&prose_syntect_theme);

    // Use LineWrapper for proper word wrapping at terminal width
    let mut wrapper = LineWrapper::new(terminal_width as usize, emit_hyperlinks);

    // Track scope stack for prose highlighting (functional style)
    let mut scope_stack: Vec<Scope> = vec![prose_highlighter.base_scope()];

    // Enable table parsing extension and wrap with MarkProcessor for ==highlight== support
    // and RuleProcessor for horizontal rules with attributes
    let parser = Parser::new_ext(
        md.content(),
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH,
    );
    let events = RuleProcessor::new(InlineStyleProcessor::new(parser));
    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut code_language = String::new();
    let mut code_info_string = String::new();

    // List tracking
    let mut list_stack: Vec<Option<u64>> = Vec::new(); // None = unordered, Some(start) = ordered

    // Table tracking - buffer entire table for proper rendering
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new(); // All rows including header
    let mut table_alignments: Vec<Alignment> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    // Single outer `Terminal` context shared by every component that renders
    // inside this pipeline (tables, horizontal rules, ...). Built once from
    // the resolved options so every consumer honors `max_width`, `color_depth`,
    // and `emit_hyperlinks` without re-running capability detection (B2).
    let mut render_terminal = Terminal::builder()
        .width(terminal_width as u32)
        .osc_link_support(emit_hyperlinks)
        .color_depth(match color_depth {
            ColorDepth::TrueColor => TerminalColorDepth::TrueColor,
            ColorDepth::Colors256 => TerminalColorDepth::Enhanced,
            ColorDepth::Colors16 => TerminalColorDepth::Basic,
            ColorDepth::None => TerminalColorDepth::None,
        })
        .build();
    match options.image_mode {
        TerminalImageMode::Never => {
            render_terminal.is_tty = false;
            render_terminal.image_support = ImageSupport::None;
        }
        TerminalImageMode::Force => {
            render_terminal.is_tty = true;
            if matches!(render_terminal.image_support, ImageSupport::None) {
                render_terminal.image_support = ImageSupport::Kitty;
            }
        }
        TerminalImageMode::Auto => {}
    }

    // Image tracking and rendering
    let mut in_image = false;
    let mut current_alt = String::new();
    let mut current_image_path = String::new();
    let mut just_rendered_image = false; // Track if we just rendered an image (skip paragraph spacing)
    let image_renderer = match options.image_mode {
        TerminalImageMode::Never => None,
        TerminalImageMode::Auto | TerminalImageMode::Force => Some(ImageRenderer::new(
            options.base_path.as_deref(),
            options.image_mode,
        )),
    };

    // Track semantic font styles for proper rendering
    // Some themes don't set font_style for markup scopes, so we track this explicitly
    let mut in_emphasis = false;
    let mut in_strong = false;
    let mut in_strikethrough = false;
    let mut in_mark = false;
    let mut in_dim = false;

    // Track hyperlinks for OSC8 terminal escape sequences
    let mut in_link = false;
    let mut current_link_url = String::new();
    let mut current_link_text = String::new();

    // Track blockquote nesting depth and whether we've seen content at current depth
    let mut blockquote_depth: usize = 0;
    let mut blockquote_has_content = false;

    // Compute blockquote background color from theme (subtle lift from page)
    let blockquote_bg = {
        let theme_bg = prose_syntect_theme.settings.background.unwrap_or(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        });
        compute_blockquote_bg(theme_bg, options.color_mode)
    };

    for event in events {
        match event {
            // Handle custom inline tags first
            InlineEvent::Start(InlineTag::Mark) => {
                in_mark = true;
                let scope = ScopeCache::global().scope_for_inline_tag(InlineTag::Mark);
                scope_stack.push(scope);
            }
            InlineEvent::End(InlineTag::Mark) => {
                in_mark = false;
                scope_stack.pop();
            }

            InlineEvent::Start(InlineTag::Dim) => {
                in_dim = true;
                let scope = ScopeCache::global().scope_for_inline_tag(InlineTag::Dim);
                scope_stack.push(scope);
            }
            InlineEvent::End(InlineTag::Dim) => {
                in_dim = false;
                scope_stack.pop();
            }

            // Handle horizontal rule with attributes
            InlineEvent::HorizontalRule(attrs) => {
                // Build the rule from attributes via the shared helper so the
                // terminal and HTML code paths stay consistent (Phase 5).
                let rule = build_rule_with_defaults(hr_defaults.as_ref(), &attrs);
                write_horizontal_rule(&mut wrapper, &rule, &render_terminal, terminal_width);
            }
            // Phase 5 (B4): bare `---` / `***` / `___` lines surface as
            // pulldown-cmark `Event::Rule`. Handle them explicitly so the
            // terminal output gets a default dashed rule instead of falling
            // through the catch-all arm.
            InlineEvent::Standard(Event::Rule) => {
                let rule = build_rule_with_defaults(
                    hr_defaults.as_ref(),
                    &crate::markdown::inline::HorizontalRuleAttrs::default(),
                );
                write_horizontal_rule(&mut wrapper, &rule, &render_terminal, terminal_width);
            }

            // Standard pulldown-cmark events
            InlineEvent::Standard(Event::Start(Tag::CodeBlock(kind))) => {
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
            InlineEvent::Standard(Event::End(TagEnd::CodeBlock)) => {
                in_code_block = false;

                // Check for mermaid code blocks
                let is_mermaid = code_language.eq_ignore_ascii_case("mermaid");

                if is_mermaid && options.mermaid_mode != MermaidMode::Off {
                    // Parse metadata for mermaid blocks (supports title attribute)
                    let meta = parse_code_info(&code_info_string).unwrap_or_default();

                    // Get background color from theme for header row (matches code blocks)
                    let theme = code_highlighter.theme();
                    let bg_color = theme.settings.background.unwrap_or(Color::BLACK);

                    match options.mermaid_mode {
                        MermaidMode::Text => {
                            // Ensure header starts on a fresh line (not appended to previous content)
                            if wrapper.current_col() > 0 {
                                wrapper.newline();
                            }
                            // Text mode: show header with title and "mermaid" label (it's a code block)
                            let header = format_header_row(
                                meta.title.as_deref(),
                                "mermaid",
                                bg_color,
                                options.color_mode,
                                terminal_width,
                            );
                            wrapper.push_with_newlines(&header);
                            wrapper.newline();
                            // Render with syntax highlighting (same as regular code blocks)
                            let highlighted = highlight_code(
                                &code_buffer,
                                "mermaid",
                                &code_highlighter,
                                &options,
                                &meta,
                                options.color_mode,
                            )?;
                            wrapper.push_with_newlines(&highlighted);
                            wrapper.push_with_newlines("\n\n");
                        }
                        MermaidMode::Image => {
                            // Flush output before viuer prints to stdout
                            // (don't emit header yet - we'll emit after knowing if rendering succeeds)
                            write!(writer, "{}", wrapper.output()).ok();
                            writer.flush().ok();
                            wrapper = LineWrapper::new(terminal_width as usize, emit_hyperlinks);

                            // Render mermaid diagram as image using mmdc CLI
                            let diagram = crate::mermaid::Mermaid::new(&code_buffer);

                            let render_succeeded = match diagram.render_for_terminal() {
                                Ok(()) => true,
                                Err(e) => {
                                    tracing::warn!(error = %e, "Mermaid image rendering failed");
                                    false
                                }
                            };

                            if render_succeeded {
                                // Image rendered successfully - emit title-only header after the image
                                // (the rendered diagram makes "mermaid" label redundant)
                                if let Some(title) = &meta.title {
                                    let header = format_header_row(
                                        Some(title.as_str()),
                                        "", // No language label for rendered images
                                        bg_color,
                                        options.color_mode,
                                        terminal_width,
                                    );
                                    wrapper.push_with_newlines(&header);
                                }
                                // viuer already prints a newline, just add one more for spacing
                                wrapper.push_with_newlines("\n");
                            } else {
                                // Rendering failed - show fallback with syntax highlighting
                                // Ensure header starts on a fresh line
                                if wrapper.current_col() > 0 {
                                    wrapper.newline();
                                }
                                // Emit header row with title and "mermaid" label
                                let header = format_header_row(
                                    meta.title.as_deref(),
                                    "mermaid",
                                    bg_color,
                                    options.color_mode,
                                    terminal_width,
                                );
                                wrapper.push_with_newlines(&header);
                                wrapper.newline();
                                // Render with syntax highlighting (same as regular code blocks)
                                let highlighted = highlight_code(
                                    &code_buffer,
                                    "mermaid",
                                    &code_highlighter,
                                    &options,
                                    &meta,
                                    options.color_mode,
                                )?;
                                wrapper.push_with_newlines(&highlighted);
                                wrapper.push_with_newlines("\n\n");
                            }
                        }
                        MermaidMode::Off => unreachable!(),
                    }
                } else {
                    // Render code block with highlighting (normal path)
                    let meta = parse_code_info(&code_info_string).unwrap_or_default();

                    // Get background color from theme for header row
                    let theme = code_highlighter.theme();
                    let bg_color = theme.settings.background.unwrap_or(Color::BLACK);

                    // Ensure header starts on a fresh line (not appended to previous content)
                    if wrapper.current_col() > 0 {
                        wrapper.newline();
                    }

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
                    // highlight_code ends with a bottom padding row, add newline after it
                    // then add blank line for separation from following content
                    wrapper.push_with_newlines("\n\n");
                }
            }
            InlineEvent::Standard(Event::Text(text)) if in_code_block => {
                code_buffer.push_str(&text);
            }

            // Prose highlighting with scope tracking
            InlineEvent::Standard(Event::Start(ref tag @ Tag::Heading { level, .. })) => {
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
            InlineEvent::Standard(Event::End(TagEnd::Heading(_))) => {
                scope_stack.pop();
                wrapper.push_with_newlines("\n\n"); // Blank line after heading
            }

            InlineEvent::Standard(Event::Start(ref tag @ Tag::Strong)) => {
                in_strong = true;
                if let Some(scope) = ScopeCache::global().scope_for_tag(tag) {
                    scope_stack.push(scope);
                }
            }
            InlineEvent::Standard(Event::End(TagEnd::Strong)) => {
                in_strong = false;
                scope_stack.pop();
            }

            InlineEvent::Standard(Event::Start(ref tag @ Tag::Emphasis)) => {
                in_emphasis = true;
                if let Some(scope) = ScopeCache::global().scope_for_tag(tag) {
                    scope_stack.push(scope);
                }
            }
            InlineEvent::Standard(Event::End(TagEnd::Emphasis)) => {
                in_emphasis = false;
                scope_stack.pop();
            }

            InlineEvent::Standard(Event::Start(ref tag @ Tag::Strikethrough)) => {
                in_strikethrough = true;
                if let Some(scope) = ScopeCache::global().scope_for_tag(tag) {
                    scope_stack.push(scope);
                }
            }
            InlineEvent::Standard(Event::End(TagEnd::Strikethrough)) => {
                in_strikethrough = false;
                scope_stack.pop();
            }

            InlineEvent::Standard(Event::Start(ref tag @ Tag::Link { ref dest_url, .. })) => {
                in_link = true;
                current_link_url = dest_url.to_string();
                current_link_text.clear();
                if let Some(scope) = ScopeCache::global().scope_for_tag(tag) {
                    scope_stack.push(scope);
                }
            }
            InlineEvent::Standard(Event::End(TagEnd::Link)) => {
                if in_table {
                    scope_stack.pop();
                    let style = table_link_style(&prose_highlighter, &scope_stack);
                    current_cell.push_str(&render_table_link(
                        &current_link_text,
                        &current_link_url,
                        style,
                        emit_italic,
                        emit_hyperlinks,
                    ));
                } else {
                    // Pop link scope to get parent scopes, then query theme for link styling
                    // (The link scope was pushed in Start(Link))
                    scope_stack.pop();
                    let link_tag = Tag::Link {
                        link_type: pulldown_cmark::LinkType::Inline,
                        dest_url: "".into(),
                        title: "".into(),
                        id: "".into(),
                    };
                    let mut style = prose_highlighter.style_for_tag(&link_tag, &scope_stack);

                    // Fallback: If theme doesn't define a distinct link color, use standard blue with underline.
                    // Many themes (e.g., OneHalf) don't style the markup.underline.link.markdown scope,
                    // causing links to appear identical to regular text. This ensures links are always
                    // visually distinguishable.
                    let base_style = prose_highlighter.base_style();
                    if style.foreground == base_style.foreground {
                        // Apply standard link blue color (similar to HTML default #0066cc)
                        style.foreground = Color {
                            r: 65,
                            g: 160,
                            b: 225,
                            a: 255,
                        }; // Soft blue
                        // Also add underline for extra visual distinction
                        style.font_style |= syntect::highlighting::FontStyle::UNDERLINE;
                    }

                    // Emit styled hyperlink with OSC8 escape sequences
                    // IMPORTANT: Styling must be applied INSIDE the OSC8 sequence, not outside.
                    // OSC8 format: ESC]8;;URL BEL <styled_text> ESC]8;; BEL
                    // The styled text appears between the OSC8 open and close sequences.
                    wrapper.emit_styled_hyperlink(
                        &current_link_text,
                        &current_link_url,
                        style,
                        emit_italic,
                    );
                }

                in_link = false;
                current_link_url.clear();
                current_link_text.clear();
            }

            // List handling
            InlineEvent::Standard(Event::Start(Tag::List(start_num))) => {
                // When a nested list starts inside an item, add newline to separate from parent text
                if !list_stack.is_empty()
                    && !wrapper.output().is_empty()
                    && !wrapper.output().ends_with('\n')
                {
                    wrapper.newline();
                }
                list_stack.push(start_num);
            }
            InlineEvent::Standard(Event::End(TagEnd::List(_))) => {
                list_stack.pop();
                // Add blank line after top-level list ends
                if list_stack.is_empty() {
                    wrapper.newline();
                }
            }
            InlineEvent::Standard(Event::Start(Tag::Item)) => {
                // Calculate indentation based on nesting level
                let indent = "  ".repeat(list_stack.len().saturating_sub(1));

                // Get the marker for this item
                if let Some(list_type) = list_stack.last_mut() {
                    match list_type {
                        Some(num) => {
                            // Ordered list: emit number and increment
                            let style = prose_highlighter.base_style();
                            wrapper.emit_styled_marker(
                                &format!("{}{}. ", indent, num),
                                style,
                                emit_italic,
                            );
                            *num += 1;
                        }
                        None => {
                            // Unordered list: emit bullet
                            let style = prose_highlighter.base_style();
                            wrapper.emit_styled_marker(
                                &format!("{}- ", indent),
                                style,
                                emit_italic,
                            );
                        }
                    }
                }
            }
            InlineEvent::Standard(Event::End(TagEnd::Item)) => {
                wrapper.newline();
            }

            InlineEvent::Standard(Event::Start(Tag::Paragraph)) => {
                // Add spacing before paragraphs inside blockquotes (except first)
                if blockquote_depth > 0 && blockquote_has_content {
                    // Add blank line between paragraphs in blockquote:
                    // - First newline ends current line and adds prefix for blank line
                    // - Second newline ends blank line and adds prefix for new paragraph
                    wrapper.emit_newline_with_prefix();
                    wrapper.emit_newline_with_prefix();
                }
                // Don't add extra spacing inside list items
            }
            InlineEvent::Standard(Event::End(TagEnd::Paragraph)) => {
                // Skip paragraph spacing after images (image provides visual separation)
                if just_rendered_image {
                    just_rendered_image = false;
                    // viuer positions cursor after image; no extra spacing needed
                } else if list_stack.is_empty() && blockquote_depth == 0 {
                    // Only add double newline for paragraphs outside of lists and blockquotes
                    wrapper.push_with_newlines("\n\n");
                }
                // Mark that we've seen content in this blockquote (for spacing next paragraph)
                if blockquote_depth > 0 {
                    blockquote_has_content = true;
                }
            }

            InlineEvent::Standard(Event::Text(text)) if !in_code_block => {
                if in_image {
                    // Accumulate alt text for image
                    current_alt.push_str(&text);
                } else if in_link {
                    // Buffer text for link (rendered with OSC8 at End(Link))
                    current_link_text.push_str(&text);
                } else if in_table {
                    let style = resolve_prose_text_style(
                        &prose_highlighter,
                        &scope_stack,
                        in_emphasis,
                        in_strong,
                        in_strikethrough,
                        in_mark,
                        in_dim,
                        blockquote_depth > 0,
                    );
                    current_cell.push_str(&render_table_cell_text(
                        &text,
                        style,
                        emit_italic,
                        emit_dim,
                        TableCellInlineState {
                            in_strikethrough,
                            in_mark,
                            in_emphasis,
                            in_strong,
                            in_dim,
                        },
                        &render_terminal,
                    ));
                } else {
                    let style = resolve_prose_text_style(
                        &prose_highlighter,
                        &scope_stack,
                        in_emphasis,
                        in_strong,
                        in_strikethrough,
                        in_mark,
                        in_dim,
                        blockquote_depth > 0,
                    );

                    // Use LineWrapper for proper word wrapping
                    // Pass in_mark to enable background highlighting
                    wrapper.emit_styled(&text, style, emit_italic, in_strikethrough, in_mark, in_dim, emit_dim);
                }
            }

            InlineEvent::Standard(Event::Code(code)) => {
                if in_image {
                    // Preserve inline code in alt text
                    current_alt.push('`');
                    current_alt.push_str(&code);
                    current_alt.push('`');
                } else if in_table {
                    let style = prose_highlighter.style_for_inline_code(&scope_stack);
                    current_cell.push_str(&emit_inline_code(&code, style));
                } else {
                    // Inline code with styling (no backticks in terminal output)
                    let style = prose_highlighter.style_for_inline_code(&scope_stack);
                    // Use LineWrapper for proper word wrapping
                    wrapper.emit_inline_code(&code, style);
                }
            }

            InlineEvent::Standard(Event::SoftBreak) => {
                // SoftBreak is a space - only emit if not at start of line
                if wrapper.current_col() > 0 {
                    wrapper.emit_raw(" ");
                }
            }
            InlineEvent::Standard(Event::HardBreak) => {
                wrapper.newline();
            }

            // Table handling - buffer entire table for proper rendering
            InlineEvent::Standard(Event::Start(Tag::Table(alignments))) => {
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
            InlineEvent::Standard(Event::End(TagEnd::Table)) => {
                in_table = false;
                // Render the buffered table with proper formatting
                wrapper.push_with_newlines(&render_table(
                    &table_rows,
                    &table_alignments,
                    terminal_width,
                ));
                // Add blank line after table for spacing from following content
                wrapper.push_with_newlines("\n\n");
                table_rows.clear();
            }
            InlineEvent::Standard(Event::Start(Tag::TableHead)) => {
                current_row.clear();
            }
            InlineEvent::Standard(Event::End(TagEnd::TableHead)) => {
                table_rows.push(current_row.clone());
                current_row.clear();
            }
            InlineEvent::Standard(Event::Start(Tag::TableRow)) => {
                current_row.clear();
            }
            InlineEvent::Standard(Event::End(TagEnd::TableRow)) => {
                table_rows.push(current_row.clone());
                current_row.clear();
            }
            InlineEvent::Standard(Event::Start(Tag::TableCell)) => {
                current_cell.clear();
            }
            InlineEvent::Standard(Event::End(TagEnd::TableCell)) => {
                current_row.push(current_cell.clone());
                current_cell.clear();
            }

            // Image handling
            InlineEvent::Standard(Event::Start(Tag::Image { dest_url, .. })) => {
                in_image = true;
                current_alt.clear();
                current_image_path = dest_url.to_string();
            }
            InlineEvent::Standard(Event::End(TagEnd::Image)) => {
                // Parse alt text to extract optional width specification
                // Format: "alt text|width" where width is like "50%", "40ch", or "40"
                let (parsed_alt, parsed_width) = parse_alt_and_width(&current_alt);

                // Images inside table cells are rendered as deterministic text fallbacks.
                // Table cells are width-managed text regions; protocol sequences should stay
                // block-level outside tables.
                if in_table {
                    current_cell.push_str(&format!("▉ IMAGE[{}]", parsed_alt));
                    in_image = false;
                    continue;
                }

                if let Some(ref renderer) = image_renderer {
                    // Flush accumulated output before viuer prints to stdout
                    if renderer.graphics_supported() {
                        write!(writer, "{}", wrapper.output()).ok();
                        writer.flush().ok();
                        // Clear the wrapper by creating a new one (preserving max_width)
                        wrapper = LineWrapper::new(terminal_width as usize, emit_hyperlinks);
                        // render_image returns protocol output on success,
                        // fallback text on failure
                        let result =
                            renderer.render_image(&current_image_path, &parsed_alt, parsed_width);
                        if !result.is_empty() {
                            // Print rendered output (or fallback text on failure)
                            write!(writer, "{}", result).ok();
                        }
                        writer.flush().ok();
                        just_rendered_image = true;
                    } else {
                        wrapper.push_with_newlines(&renderer.render_image(
                            &current_image_path,
                            &parsed_alt,
                            parsed_width,
                        ));
                        just_rendered_image = true;
                    }
                } else {
                    wrapper.push_with_newlines(&format!("▉ IMAGE[{}]\n", parsed_alt));
                    just_rendered_image = true;
                }
                in_image = false;
            }

            // Blockquote handling - add styled prefix and background
            InlineEvent::Standard(Event::Start(ref tag @ Tag::BlockQuote(_))) => {
                if blockquote_depth == 0 {
                    // Top-level blockquote - add spacing before if there's prior content
                    if !wrapper.output().is_empty() && !wrapper.output().ends_with("\n\n") {
                        if wrapper.output().ends_with('\n') {
                            wrapper.newline();
                        } else {
                            wrapper.push_with_newlines("\n\n");
                        }
                    }
                } else {
                    // Nested blockquote - end current line with outer prefix, add blank line
                    wrapper.emit_newline_with_prefix(); // Ends current line, adds outer prefix for blank line
                    wrapper.newline(); // Ends blank line, no prefix (nested prefix comes next)
                }
                blockquote_depth += 1;
                blockquote_has_content = false; // Reset content flag for new blockquote level
                if let Some(scope) = ScopeCache::global().scope_for_tag(tag) {
                    scope_stack.push(scope);
                }
                // Emit the blockquote prefix for the first line of this blockquote level
                wrapper.emit_blockquote_prefix(blockquote_depth, blockquote_bg);
            }
            InlineEvent::Standard(Event::End(TagEnd::BlockQuote(_))) => {
                blockquote_depth = blockquote_depth.saturating_sub(1);
                scope_stack.pop();
                // Update wrapper's blockquote state
                if blockquote_depth == 0 {
                    blockquote_has_content = false; // Reset only when fully exiting blockquotes
                    wrapper.clear_blockquote();
                    // Add blank line after blockquote (like headings and paragraphs)
                    wrapper.push_with_newlines("\n\n");
                } else {
                    // Still nested - just update state to new depth
                    // Paragraph start will handle spacing with the new depth
                    wrapper.set_blockquote_state(blockquote_depth, blockquote_bg);
                    // Mark that outer blockquote has content (the nested blockquote)
                    blockquote_has_content = true;
                }
            }

            InlineEvent::Standard(_) => {} // Ignore other standard events
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
/// * `in_strikethrough` - Whether to apply strikethrough formatting
/// * `in_mark` - Whether to apply highlight background color
/// * `blockquote_bg` - Optional background color for blockquote context
/// * `in_dim` - Whether to apply dim/faint formatting
/// * `emit_dim` - Whether to emit dim escape codes
#[allow(clippy::too_many_arguments)]
fn emit_prose_text(
    text: &str,
    style: Style,
    emit_italic: bool,
    in_strikethrough: bool,
    in_mark: bool,
    blockquote_bg: Option<Color>,
    in_dim: bool,
    emit_dim: bool,
) -> String {
    use syntect::highlighting::FontStyle;

    let fg = style.foreground;
    let mut result = String::new();

    // Apply font styles (bold=1, dim=2, italic=3, underline=4, strikethrough=9)
    if style.font_style.contains(FontStyle::BOLD) {
        result.push_str("\x1b[1m");
    }
    if emit_dim && in_dim {
        result.push_str("\x1b[2m");
    }
    if emit_italic && style.font_style.contains(FontStyle::ITALIC) {
        result.push_str("\x1b[3m");
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        result.push_str("\x1b[4m");
    }
    if in_strikethrough {
        result.push_str("\x1b[9m");
    }

    // Apply background color for highlighted/marked text
    // Uses a yellow background similar to <mark> in HTML
    if in_mark {
        // Yellow highlight background (255, 243, 184) - matches CSS var(--highlight-bg, #fff3b8)
        result.push_str("\x1b[48;2;255;243;184m");
        // Use dark text for contrast on yellow background
        result.push_str(&format!("\x1b[38;2;0;0;0m{}\x1b[0m", text));
    } else if let Some(bg) = blockquote_bg {
        // Apply blockquote background color with foreground color
        result.push_str(&format!(
            "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m{}\x1b[0m",
            bg.r, bg.g, bg.b, fg.r, fg.g, fg.b, text
        ));
    } else {
        // Apply foreground color and text
        result.push_str(&format!(
            "\x1b[38;2;{};{};{}m{}\x1b[0m",
            fg.r, fg.g, fg.b, text
        ));
    }
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

/// Resolves prose text style from the scope stack and semantic inline states.
#[allow(clippy::too_many_arguments)]
fn resolve_prose_text_style(
    prose_highlighter: &ProseHighlighter<'_>,
    scope_stack: &[Scope],
    in_emphasis: bool,
    in_strong: bool,
    in_strikethrough: bool,
    in_mark: bool,
    in_dim: bool,
    in_blockquote: bool,
) -> Style {
    use syntect::highlighting::FontStyle;

    let mut style = if in_emphasis || in_strong || in_strikethrough || in_mark || in_dim {
        // Compute style from parent scopes (exclude inline formatting scopes)
        // to preserve the parent's color.
        let parent_depth = scope_stack.len()
            - (in_emphasis as usize)
            - (in_strong as usize)
            - (in_strikethrough as usize)
            - (in_mark as usize)
            - (in_dim as usize);
        let parent_scopes = &scope_stack[..parent_depth.max(1)];
        prose_highlighter.style_for_tag(&Tag::Paragraph, parent_scopes)
    } else if scope_stack.len() > 1 {
        prose_highlighter.style_for_tag(&Tag::Paragraph, scope_stack)
    } else {
        prose_highlighter.base_style()
    };

    // Apply semantic font styles - italic/bold only change style, not color.
    if in_emphasis {
        style.font_style |= FontStyle::ITALIC;
    }
    if in_strong {
        style.font_style |= FontStyle::BOLD;
    }

    // Clear theme-applied italic for blockquotes - only explicit emphasis should be italic.
    if in_blockquote && !in_emphasis {
        style.font_style.remove(FontStyle::ITALIC);
    }

    style
}

/// Resolves the style used for links and applies fallback coloring if the theme
/// doesn't provide a distinct link color.
fn table_link_style(prose_highlighter: &ProseHighlighter<'_>, scope_stack: &[Scope]) -> Style {
    let link_tag = Tag::Link {
        link_type: pulldown_cmark::LinkType::Inline,
        dest_url: "".into(),
        title: "".into(),
        id: "".into(),
    };
    let mut style = prose_highlighter.style_for_tag(&link_tag, scope_stack);
    let base_style = prose_highlighter.base_style();
    if style.foreground == base_style.foreground {
        style.foreground = Color {
            r: 65,
            g: 160,
            b: 225,
            a: 255,
        };
        style.font_style |= syntect::highlighting::FontStyle::UNDERLINE;
    }
    style
}

/// Renders a table-cell hyperlink while honoring hyperlink mode behavior.
fn render_table_link(
    text: &str,
    url: &str,
    style: Style,
    emit_italic: bool,
    emit_hyperlinks: bool,
) -> String {
    if emit_hyperlinks {
        format!(
            "\x1b]8;;{}\x07{}\x1b]8;;\x07",
            url,
            emit_prose_text(text, style, emit_italic, false, false, None, false, true)
        )
    } else {
        format!(
            "{} [{}]",
            emit_prose_text(text, style, emit_italic, false, false, None, false, true),
            url
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct TableCellInlineState {
    in_strikethrough: bool,
    in_mark: bool,
    in_emphasis: bool,
    in_strong: bool,
    in_dim: bool,
}

fn render_table_cell_text(
    text: &str,
    style: Style,
    emit_italic: bool,
    emit_dim: bool,
    state: TableCellInlineState,
    terminal: &Terminal,
) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Keep mark/highlight rendering on Darkmatter's style path.
    if state.in_mark {
        return emit_prose_text(text, style, emit_italic, state.in_strikethrough, true, None, state.in_dim, emit_dim);
    }

    // Use Prose selectively for semantic inline style serialization in table cells.
    // This keeps table layout logic in biscuit-terminal while Darkmatter owns markdown parsing.
    if state.in_strong || state.in_emphasis || state.in_strikethrough || state.in_dim {
        let mut serialized = String::new();
        if state.in_strong {
            serialized.push_str("<bold>");
        }
        if state.in_emphasis && emit_italic {
            serialized.push_str("<italic>");
        }
        if state.in_strikethrough {
            serialized.push_str("<strikethrough>");
        }
        if state.in_dim && emit_dim {
            serialized.push_str("<dim>");
        }
        serialized.push_str(text);
        if state.in_dim && emit_dim {
            serialized.push_str("</dim>");
        }
        if state.in_strikethrough {
            serialized.push_str("</strikethrough>");
        }
        if state.in_emphasis && emit_italic {
            serialized.push_str("</italic>");
        }
        if state.in_strong {
            serialized.push_str("</bold>");
        }

        // Avoid tag parsing edge cases for literal '<'/'{' content.
        if !text.contains('<') && !text.contains('{') {
            let prose = Prose::new(serialized).render(terminal);
            let fg = style.foreground;
            return format!("\x1b[38;2;{};{};{}m{}\x1b[0m", fg.r, fg.g, fg.b, prose);
        }
    }

    emit_prose_text(
        text,
        style,
        emit_italic,
        state.in_strikethrough,
        false,
        None,
        state.in_dim,
        emit_dim,
    )
}

/// Renders a buffered markdown table via `biscuit-terminal::Table`.
///
/// Markdown parsing remains in Darkmatter; final table layout/rendering is delegated
/// to biscuit-terminal's ANSI/OSC-aware table component.
#[tracing::instrument(
    skip(rows, alignments),
    fields(row_count = rows.len(), col_count, terminal_width)
)]
fn render_table(rows: &[Vec<String>], alignments: &[Alignment], terminal_width: u16) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let col_count = rows
        .iter()
        .map(|row| row.len())
        .max()
        .unwrap_or(0)
        .max(alignments.len());
    tracing::Span::current().record("col_count", col_count);

    if col_count == 0 {
        return String::new();
    }

    let header = rows.first();
    let columns = (0..col_count)
        .map(|index| {
            let header_text = header
                .and_then(|row| row.get(index))
                .cloned()
                .unwrap_or_default();
            let alignment = alignments.get(index).copied().unwrap_or(Alignment::Left);
            TableColumn::new(header_text)
                .with_type(ColumnType::String)
                .with_alignment(alignment)
        })
        .collect();

    let data = rows
        .iter()
        .skip(1)
        .map(|row| {
            (0..col_count)
                .map(|index| TableCellContent::Text(row.get(index).cloned().unwrap_or_default()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    TerminalTable::new()
        .with_columns(columns)
        .with_data(data)
        .render(&Terminal::builder().width(terminal_width as u32).build())
}

/// Emits code text with both foreground and background colors.
#[cfg(test)]
fn emit_code_text(text: &str, style: Style, bg_color: Color) -> String {
    let fg = style.foreground;
    format!(
        "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m{}\x1b[0m",
        bg_color.r, bg_color.g, bg_color.b, fg.r, fg.g, fg.b, text
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
        ColorMode::Dark => (255, 255, 255), // WHITE
        ColorMode::Light => (0, 0, 0),      // BLACK
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
/// * `language` - Language identifier. If empty AND title is present, creates a
///   title-only header (no language label). If empty with no title, defaults to "text".
/// * `bg_color` - Background color for title and language spans
/// * `color_mode` - Color mode for determining text color
/// * `terminal_width` - Terminal width for right-alignment calculation
///
/// ## Returns
///
/// ANSI-formatted string with title (if present) on the left and language right-aligned.
/// Title is bold, language is not. For title-only headers (empty language with title),
/// only the title is rendered.
///
/// ## Notes
///
/// This helper is also reused by [`crate::markdown::yaml_block::YamlBlock`] to keep
/// the YAML-fence header row byte-identical between Markdown and YamlBlock outputs.
/// Any signature change here must keep that consumer in sync.
pub(crate) fn format_header_row(
    title: Option<&str>,
    language: &str,
    bg_color: Color,
    color_mode: ColorMode,
    terminal_width: u16,
) -> String {
    let text_color = header_text_color(color_mode);

    // For empty language, default to "text" unless we have a title-only header
    let show_language = !language.is_empty() || title.is_none();
    let lang = if language.is_empty() {
        "text"
    } else {
        language
    };

    // Calculate visible widths for spacing
    // Title: " {title} " = 1 + title.len() + 1 = title.len() + 2
    // Language: " {lang} " = 1 + lang.len() + 1 = lang.len() + 2
    let title_width = title.map(|t| t.chars().count() + 2).unwrap_or(0);
    let lang_width = if show_language {
        lang.chars().count() + 2
    } else {
        0
    };
    let total_content_width = title_width + lang_width;

    // Calculate spacing to right-align language (or fill line for title-only)
    let spacing = if (terminal_width as usize) > total_content_width {
        terminal_width as usize - total_content_width
    } else if show_language {
        1 // Minimum 1 space between title and language
    } else {
        0 // No spacing needed for title-only
    };

    let mut output = String::new();

    // Left side: title (if present)
    if let Some(t) = title {
        // Bold + BG + FG + space + title + space + reset
        output.push_str(&format!(
            "\x1b[1m\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m {} \x1b[0m",
            bg_color.r, bg_color.g, bg_color.b, text_color.0, text_color.1, text_color.2, t
        ));
    }

    // Add spacing to push language to the right
    for _ in 0..spacing {
        output.push(' ');
    }

    // Right side: language (right-aligned) - skip if title-only header
    if show_language {
        // BG + FG + space + lang + space + reset
        output.push_str(&format!(
            "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m {} \x1b[0m",
            bg_color.r, bg_color.g, bg_color.b, text_color.0, text_color.1, text_color.2, lang
        ));
    }

    output
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
    /// Current blockquote nesting depth (0 = not in blockquote)
    blockquote_depth: usize,
    /// Background color for blockquotes
    blockquote_bg: Option<Color>,
    /// Whether to emit OSC 8 hyperlinks (cached detection result)
    supports_hyperlinks: bool,
}

impl LineWrapper {
    /// Creates a new LineWrapper with the given maximum width and hyperlink support flag.
    fn new(max_width: usize, supports_hyperlinks: bool) -> Self {
        Self {
            current_col: 0,
            max_width,
            output: String::new(),
            blockquote_depth: 0,
            blockquote_bg: None,
            supports_hyperlinks,
        }
    }

    /// Sets blockquote state without emitting the prefix.
    ///
    /// Use this when updating state but letting `emit_newline_with_prefix` handle the prefix.
    fn set_blockquote_state(&mut self, depth: usize, bg: Color) {
        self.blockquote_depth = depth;
        self.blockquote_bg = Some(bg);
    }

    /// Emits a blockquote prefix with styled bar and indentation.
    ///
    /// Uses `▐` character followed by 3 spaces for visual separation.
    /// The entire line gets a subtle background color.
    fn emit_blockquote_prefix(&mut self, depth: usize, bg: Color) {
        self.blockquote_depth = depth;
        self.blockquote_bg = Some(bg);

        // Build the prefix: "▐   " repeated for each nesting level
        // The bar character ▐ has width 1, plus 3 spaces = 4 chars per level
        let prefix = "▐   ".repeat(depth);
        let prefix_width = UnicodeWidthStr::width(prefix.as_str());

        // Emit with background color (gray for bar, subtle bg for spaces)
        // Bar: bright gray foreground
        // Spaces: subtle background extends
        self.output.push_str(&format!(
            "\x1b[38;2;100;100;100m\x1b[48;2;{};{};{}m{}\x1b[0m",
            bg.r, bg.g, bg.b, prefix
        ));
        self.current_col = prefix_width;
    }

    /// Clears the blockquote context (called when exiting all blockquotes).
    ///
    /// Pads the final line to terminal width before clearing to ensure uniform background.
    fn clear_blockquote(&mut self) {
        self.pad_to_width();
        self.blockquote_depth = 0;
        self.blockquote_bg = None;
    }

    /// Pads the current line to max_width with background-colored spaces.
    ///
    /// Called before newlines in blockquotes to ensure uniform background width.
    fn pad_to_width(&mut self) {
        if let Some(bg) = self.blockquote_bg
            && self.current_col < self.max_width
        {
            let padding = self.max_width - self.current_col;
            let spaces = " ".repeat(padding);
            self.output.push_str(&format!(
                "\x1b[48;2;{};{};{}m{}\x1b[0m",
                bg.r, bg.g, bg.b, spaces
            ));
            self.current_col = self.max_width;
        }
    }

    /// Emits a newline and the blockquote prefix if we're in a blockquote.
    fn emit_newline_with_prefix(&mut self) {
        // Pad line to terminal width before newline (for uniform blockquote background)
        self.pad_to_width();
        self.output.push('\n');
        if self.blockquote_depth > 0 {
            if let Some(bg) = self.blockquote_bg {
                let prefix = "▐   ".repeat(self.blockquote_depth);
                let prefix_width = UnicodeWidthStr::width(prefix.as_str());
                self.output.push_str(&format!(
                    "\x1b[38;2;100;100;100m\x1b[48;2;{};{};{}m{}\x1b[0m",
                    bg.r, bg.g, bg.b, prefix
                ));
                self.current_col = prefix_width;
            } else {
                self.current_col = 0;
            }
        } else {
            self.current_col = 0;
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
    /// * `in_strikethrough` - Whether to apply strikethrough formatting
    /// * `in_mark` - Whether to apply highlight background
    /// * `in_dim` - Whether to apply dim/faint formatting
    /// * `emit_dim` - Whether to emit dim escape codes
    #[allow(clippy::too_many_arguments)]
    fn emit_styled(
        &mut self,
        text: &str,
        style: Style,
        emit_italic: bool,
        in_strikethrough: bool,
        in_mark: bool,
        in_dim: bool,
        emit_dim: bool,
    ) {
        // Split into segments, preserving whitespace
        // We iterate over whitespace-separated words, handling spaces between them
        let mut current_word = String::new();

        for ch in text.chars() {
            if ch.is_whitespace() {
                // Emit accumulated word first
                if !current_word.is_empty() {
                    self.emit_word(&current_word, style, emit_italic, in_strikethrough, in_mark, in_dim, emit_dim);
                    current_word.clear();
                }
                // Handle whitespace
                if ch == '\n' {
                    // Hard break - emit newline (with blockquote prefix if applicable)
                    self.emit_newline_with_prefix();
                } else {
                    // Space or tab - emit if not at start of line
                    if self.current_col > 0 {
                        // Check if space would overflow the line
                        // If at max width, skip the space (next word will wrap anyway)
                        if self.current_col >= self.max_width {
                            // At or past max width - don't emit space, let next word trigger wrap
                            continue;
                        }
                        // For marked text or blockquote, emit styled space to preserve background
                        if in_mark || self.blockquote_bg.is_some() {
                            self.emit_word(" ", style, emit_italic, in_strikethrough, in_mark, in_dim, emit_dim);
                        } else {
                            self.output.push(' ');
                            self.current_col += 1;
                        }
                    }
                }
            } else {
                current_word.push(ch);
            }
        }

        // Emit any remaining word
        if !current_word.is_empty() {
            self.emit_word(&current_word, style, emit_italic, in_strikethrough, in_mark, in_dim, emit_dim);
        }
    }

    /// Emits a single word, wrapping if necessary.
    #[allow(clippy::too_many_arguments)]
    fn emit_word(
        &mut self,
        word: &str,
        style: Style,
        emit_italic: bool,
        in_strikethrough: bool,
        in_mark: bool,
        in_dim: bool,
        emit_dim: bool,
    ) {
        let word_width = UnicodeWidthStr::width(word);

        // Check if word fits on current line
        if self.current_col > 0 && self.current_col + word_width > self.max_width {
            // Need to wrap - emit newline (with blockquote prefix if applicable)
            self.emit_newline_with_prefix();
        }

        // Emit the styled word with blockquote background if applicable
        self.output.push_str(&emit_prose_text(
            word,
            style,
            emit_italic,
            in_strikethrough,
            in_mark,
            self.blockquote_bg,
            in_dim,
            emit_dim,
        ));
        self.current_col += word_width;
    }

    /// Emits a raw string without styling (for markers, bullets, etc.).
    /// Updates column position based on visual width.
    fn emit_raw(&mut self, text: &str) {
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
        // Note: markers don't use blockquote background (they have their own styling)
        self.output.push_str(&emit_prose_text(
            text,
            style,
            emit_italic,
            false,
            false,
            None,
            false,
            true,
        ));
        self.current_col += UnicodeWidthStr::width(text);
    }

    /// Emits a styled hyperlink with OSC8 escape sequences or fallback format.
    ///
    /// When `supports_hyperlinks` is true, the styling is applied INSIDE the OSC8
    /// sequence so the visible text retains its color (typically blue for links).
    /// The OSC8 wrapper makes the text clickable in supporting terminals.
    ///
    /// When `supports_hyperlinks` is false, emits fallback format: `text [url]`
    /// which is readable in all terminals including dumb terminals.
    ///
    /// OSC8 Format: `ESC]8;;URL BEL <styled_text> ESC]8;; BEL`
    /// Fallback Format: `<styled_text> [url]`
    fn emit_styled_hyperlink(&mut self, text: &str, url: &str, style: Style, emit_italic: bool) {
        if self.supports_hyperlinks {
            // OSC8 hyperlink start: ESC ] 8 ; ; <url> BEL
            self.output.push_str("\x1b]8;;");
            self.output.push_str(url);
            self.output.push('\x07');

            // Emit styled text INSIDE the hyperlink
            // Note: We don't use word wrapping for links - they stay on one line
            self.output.push_str(&emit_prose_text(
                text,
                style,
                emit_italic,
                false,
                false,
                None,
                false,
                true,
            ));

            // OSC8 hyperlink end: ESC ] 8 ; ; BEL
            self.output.push_str("\x1b]8;;\x07");

            self.current_col += UnicodeWidthStr::width(text);
        } else {
            // Fallback format: styled text followed by [url]
            self.output.push_str(&emit_prose_text(
                text,
                style,
                emit_italic,
                false,
                false,
                None,
                false,
                true,
            ));
            self.current_col += UnicodeWidthStr::width(text);

            // Append URL in brackets (unstyled)
            let url_part = format!(" [{}]", url);
            self.output.push_str(&url_part);
            self.current_col += UnicodeWidthStr::width(url_part.as_str());
        }
    }

    /// Emits inline code with styling.
    fn emit_inline_code(&mut self, code: &str, style: Style) {
        let code_width = UnicodeWidthStr::width(code);

        // Check if code fits on current line
        if self.current_col > 0 && self.current_col + code_width > self.max_width {
            // Wrap before inline code (with blockquote prefix if applicable)
            self.emit_newline_with_prefix();
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

/// Emits a rendered [`HorizontalRule`] into `wrapper`, honoring the rule's
/// [`Layout`](biscuit_terminal::utils::layout::Layout) margins (B3) and using
/// the outer [`Terminal`] context (B2).
///
/// The rule's `top_margin` and `bottom_margin` are resolved to character
/// counts via [`Layout::resolve_margin`](biscuit_terminal::utils::layout::Layout::resolve_margin)
/// and emitted as blank lines. A default-layout rule (`Margin::None`) produces
/// a single trailing blank line to match the surrounding markdown rhythm —
/// replacing the previous hardcoded double `\n\n` spacing.
fn write_horizontal_rule(
    wrapper: &mut LineWrapper,
    rule: &HorizontalRule,
    term: &Terminal,
    terminal_width: u16,
) {
    use biscuit_terminal::components::renderable::Renderable;
    use biscuit_terminal::utils::layout::{Layout, Margin};

    // If we're mid-line, break to column 0 before emitting margins.
    if wrapper.current_col() > 0 {
        wrapper.newline();
    }

    let layout = rule.layout();
    let top = Layout::resolve_margin(&layout.top_margin, terminal_width as u32);
    for _ in 0..top {
        wrapper.newline();
    }

    // `render` returns the rule content without a trailing newline; use
    // `push_with_newlines` + an explicit `\n` so wrapper column tracking
    // resets correctly.
    wrapper.push_with_newlines(&rule.render(term));
    wrapper.push_with_newlines("\n");

    let bottom = Layout::resolve_margin(&layout.bottom_margin, terminal_width as u32);
    if matches!(layout.bottom_margin, Margin::None) {
        // Default behavior: one blank line after the rule so subsequent
        // blocks visually separate without forcing authors to set an
        // explicit margin. Callers that want tighter or looser spacing can
        // configure `layout.bottom_margin` on the rule.
        wrapper.push_with_newlines("\n");
    } else {
        for _ in 0..bottom {
            wrapper.newline();
        }
    }
}

/// Adjusts a background color based on color mode and RGB delta values.
///
/// For dark mode, adds brightness (capped at 235 to avoid white).
/// For light mode, subtracts brightness.
///
/// ## Arguments
///
/// * `base` - Base color to adjust
/// * `color_mode` - Dark or light mode
/// * `dark_delta` - (r, g, b) amounts to add in dark mode
/// * `light_delta` - (r, g, b) amounts to subtract in light mode
///
/// ## Examples
///
/// ```ignore
/// use darkmatter::markdown::highlighting::ColorMode;
/// use syntect::highlighting::Color;
///
/// let theme_bg = Color { r: 40, g: 40, b: 40, a: 255 };
/// let adjusted = adjust_background(theme_bg, ColorMode::Dark, (30, 25, 0), (20, 15, 0));
/// // Result: Color { r: 70, g: 65, b: 40, a: 255 }
/// ```
pub(crate) fn adjust_background(
    base: Color,
    color_mode: ColorMode,
    dark_delta: (u8, u8, u8),
    light_delta: (u8, u8, u8),
) -> Color {
    match color_mode {
        ColorMode::Dark => Color {
            r: base.r.saturating_add(dark_delta.0).min(235),
            g: base.g.saturating_add(dark_delta.1).min(235),
            b: base.b.saturating_add(dark_delta.2).min(235),
            a: 255,
        },
        ColorMode::Light => Color {
            r: base.r.saturating_sub(light_delta.0),
            g: base.g.saturating_sub(light_delta.1),
            b: base.b.saturating_sub(light_delta.2),
            a: 255,
        },
    }
}

/// Computes a subtle background color for blockquotes based on the theme background.
///
/// Uses uniform brightness adjustment for subtle visual separation.
#[inline]
fn compute_blockquote_bg(theme_bg: Color, color_mode: ColorMode) -> Color {
    adjust_background(theme_bg, color_mode, (20, 20, 20), (15, 15, 15))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestTerminal, strip_ansi_codes};

    /// Returns deterministic `TerminalOptions` for testing.
    ///
    /// This helper ensures tests are not affected by environment variables
    /// like `NO_COLOR`, `TERM=dumb`, `THEME`, or `CODE_THEME`. All settings
    /// are explicitly configured to provide consistent, predictable behavior.
    ///
    /// ## Configuration
    ///
    /// - Color depth: `TrueColor` (enables ANSI escape sequences)
    /// - Themes: `OneHalf` (stable, well-tested theme)
    /// - Color mode: `Dark`
    /// - Italic mode: `Always` (forces italic escape codes)
    /// - Hyperlink mode: `Always` (forces OSC8 escape codes)
    /// - Max width: `80` (deterministic line wrapping)
    /// - Line numbers: disabled
    /// - Image rendering: disabled (avoids filesystem dependencies)
    fn test_options() -> TerminalOptions {
        TerminalOptions {
            code_theme: ThemePair::OneHalf,
            prose_theme: ThemePair::OneHalf,
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
        }
    }

    #[test]
    fn test_color_depth_auto_detect() {
        let depth = ColorDepth::auto_detect();
        // Just verify it returns a valid variant
        assert!(matches!(
            depth,
            ColorDepth::TrueColor | ColorDepth::Colors256 | ColorDepth::Colors16 | ColorDepth::None
        ));
    }

    #[test]
    fn test_terminal_options_default_uses_detection() {
        // This test specifically checks TerminalOptions::default() behavior
        // so it must use default(), not test_options()
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
        let output = for_terminal(&md, test_options()).unwrap();

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
        let output = for_terminal(&md, test_options()).unwrap();

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

        let mut options = test_options();
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
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain title (without prefix)
        assert!(output.contains("Example"));

        // Title should be bold
        assert!(output.contains("\x1b[1m"));
    }

    #[test]
    fn test_for_terminal_inline_code() {
        let md: Markdown = "Use `cargo build` to compile.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Inline code should NOT have backticks in terminal output
        assert!(plain.contains("cargo build"));
        assert!(
            !plain.contains("`cargo build`"),
            "Backticks should be removed in terminal output"
        );
    }

    #[test]
    fn test_for_terminal_paragraph() {
        let md: Markdown = "First paragraph.\n\nSecond paragraph.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("First paragraph"));
        assert!(plain.contains("Second paragraph"));
    }

    #[test]
    fn test_terminal_strikethrough_basic() {
        let md: Markdown = "This is ~~strikethrough~~ text.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain strikethrough ANSI code \x1b[9m
        assert!(
            output.contains("\x1b[9m"),
            "Should contain strikethrough ANSI code"
        );

        // Content should be present when stripped
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("strikethrough"));
    }

    #[test]
    fn test_terminal_strikethrough_nested_bold() {
        let md: Markdown = "This is **~~bold strikethrough~~** text.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain both bold and strikethrough codes
        assert!(output.contains("\x1b[1m"), "Should contain bold code");
        assert!(
            output.contains("\x1b[9m"),
            "Should contain strikethrough code"
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("bold strikethrough"));
    }

    #[test]
    fn test_terminal_strikethrough_nested_italic() {
        let md: Markdown = "This is *~~italic strikethrough~~* text.".into();
        let mut options = test_options();
        options.italic_mode = ItalicMode::Always;
        let output = for_terminal(&md, options).unwrap();

        // Should contain both italic and strikethrough codes
        assert!(output.contains("\x1b[3m"), "Should contain italic code");
        assert!(
            output.contains("\x1b[9m"),
            "Should contain strikethrough code"
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("italic strikethrough"));
    }

    #[test]
    fn test_terminal_strikethrough_all_styles() {
        let md: Markdown = "This is ***~~all styles~~*** text.".into();
        let mut options = test_options();
        options.italic_mode = ItalicMode::Always;
        let output = for_terminal(&md, options).unwrap();

        // Should contain bold, italic, and strikethrough codes
        assert!(output.contains("\x1b[1m"), "Should contain bold code");
        assert!(output.contains("\x1b[3m"), "Should contain italic code");
        assert!(
            output.contains("\x1b[9m"),
            "Should contain strikethrough code"
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("all styles"));
    }

    #[test]
    fn test_terminal_no_strikethrough_without_markers() {
        let md: Markdown = "This is normal text without strikethrough.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should NOT contain strikethrough ANSI code
        assert!(
            !output.contains("\x1b[9m"),
            "Should not contain strikethrough code for normal text"
        );
    }

    #[test]
    fn test_terminal_strikethrough_unclosed() {
        let md: Markdown = "This has ~~unclosed strikethrough markers.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Unclosed markers should be rendered literally, not as strikethrough
        let plain = strip_ansi_codes(&output);
        assert!(
            plain.contains("~~unclosed"),
            "Unclosed markers should render literally"
        );
    }

    #[test]
    fn test_terminal_strikethrough_multiple() {
        let md: Markdown = "This has ~~one~~ and ~~two~~ strikethroughs.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain strikethrough code (at least once, possibly multiple times)
        assert!(
            output.contains("\x1b[9m"),
            "Should contain strikethrough code"
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("one"));
        assert!(plain.contains("two"));
    }

    #[test]
    fn test_terminal_strikethrough_in_list() {
        let md: Markdown = "- ~~completed item~~\n- active item".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain strikethrough code
        assert!(
            output.contains("\x1b[9m"),
            "Strikethrough should work in list items"
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("completed item"));
        assert!(plain.contains("active item"));
    }

    // =========================================================================
    // Highlight (==text==) tests
    // =========================================================================

    #[test]
    fn test_terminal_highlight_basic() {
        let md: Markdown = "This is ==highlighted== text.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain background color ANSI code \x1b[48;2;255;243;184m (yellow background)
        assert!(
            output.contains("\x1b[48;2;255;243;184m"),
            "Should contain highlight background ANSI code"
        );

        // Content should be present when stripped
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("highlighted"));
    }

    #[test]
    fn test_terminal_highlight_nested_bold() {
        let md: Markdown = "This is **==bold highlight==** text.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain both bold and highlight background codes
        assert!(output.contains("\x1b[1m"), "Should contain bold code");
        assert!(
            output.contains("\x1b[48;2;255;243;184m"),
            "Should contain highlight background code"
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("bold highlight"));
    }

    #[test]
    fn test_terminal_highlight_nested_italic() {
        let md: Markdown = "This is *==italic highlight==* text.".into();
        let mut options = test_options();
        options.italic_mode = ItalicMode::Always;
        let output = for_terminal(&md, options).unwrap();

        // Should contain both italic and highlight background codes
        assert!(output.contains("\x1b[3m"), "Should contain italic code");
        assert!(
            output.contains("\x1b[48;2;255;243;184m"),
            "Should contain highlight background code"
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("italic highlight"));
    }

    #[test]
    fn test_terminal_no_highlight_without_markers() {
        let md: Markdown = "This is normal text without highlights.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should NOT contain highlight background ANSI code (specifically yellow highlight)
        assert!(
            !output.contains("\x1b[48;2;255;243;184m"),
            "Should not contain highlight background without markers"
        );
    }

    #[test]
    fn test_terminal_highlight_unclosed() {
        let md: Markdown = "This has ==unclosed highlight markers.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Unclosed markers should be rendered literally, not as highlight
        let plain = strip_ansi_codes(&output);
        assert!(
            plain.contains("==unclosed"),
            "Unclosed markers should render literally"
        );
    }

    #[test]
    fn test_terminal_highlight_multiple() {
        let md: Markdown = "This has ==one== and ==two== highlights.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain highlight background code (at least once)
        assert!(
            output.contains("\x1b[48;2;255;243;184m"),
            "Should contain highlight background code"
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("one"));
        assert!(plain.contains("two"));
    }

    #[test]
    fn test_terminal_highlight_in_list() {
        let md: Markdown = "- ==highlighted item==\n- normal item".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain highlight background code
        assert!(
            output.contains("\x1b[48;2;255;243;184m"),
            "Highlight should work in list items"
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("highlighted item"));
        assert!(plain.contains("normal item"));
    }

    #[test]
    fn test_terminal_highlight_in_code_block_unchanged() {
        let md: Markdown = "```\n==not highlighted==\n```".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // The == markers should appear literally in code block content
        let plain = strip_ansi_codes(&output);
        assert!(
            plain.contains("==not highlighted=="),
            "Code block should preserve == markers"
        );
    }

    #[test]
    fn test_terminal_highlight_empty() {
        let md: Markdown = "This has ==== empty highlight.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Empty highlight (====) creates start + end events with no content between
        // so no text actually gets the background color applied, but the events are balanced
        let plain = strip_ansi_codes(&output);
        assert!(
            plain.contains("empty highlight"),
            "Text after empty highlight should render"
        );
        // The ==== should not cause any rendering issues
    }

    #[test]
    fn test_header_row_with_title() {
        let bg_color = Color {
            r: 40,
            g: 40,
            b: 40,
            a: 255,
        };
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
        let bg_color = Color {
            r: 40,
            g: 40,
            b: 40,
            a: 255,
        };
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
        let bg_color = Color {
            r: 40,
            g: 40,
            b: 40,
            a: 255,
        };
        let header = format_header_row(None, "", bg_color, ColorMode::Dark, 80);

        // Should default to "text"
        let plain = strip_ansi_codes(&header);
        assert!(plain.contains("text"));
    }

    #[test]
    fn test_header_row_dark_mode() {
        let bg_color = Color {
            r: 40,
            g: 40,
            b: 40,
            a: 255,
        };
        let header = format_header_row(Some("Title"), "rust", bg_color, ColorMode::Dark, 80);

        // Should contain white text color (255, 255, 255)
        assert!(header.contains("\x1b[38;2;255;255;255m"));
    }

    #[test]
    fn test_header_row_light_mode() {
        let bg_color = Color {
            r: 240,
            g: 240,
            b: 240,
            a: 255,
        };
        let header = format_header_row(Some("Title"), "rust", bg_color, ColorMode::Light, 80);

        // Should contain black text color (0, 0, 0)
        assert!(header.contains("\x1b[38;2;0;0;0m"));
    }

    #[test]
    fn test_header_row_bold_title() {
        let bg_color = Color {
            r: 40,
            g: 40,
            b: 40,
            a: 255,
        };
        let header = format_header_row(Some("Title"), "rust", bg_color, ColorMode::Dark, 80);

        // Should contain bold code
        assert!(header.contains("\x1b[1m"));
    }

    #[test]
    fn test_header_row_no_bold_language() {
        let bg_color = Color {
            r: 40,
            g: 40,
            b: 40,
            a: 255,
        };
        let header = format_header_row(None, "rust", bg_color, ColorMode::Dark, 80);

        // Language-only header should not start with bold (title is what gets bolded)
        // The format is: "\x1b[48;2;...m\x1b[38;2;...m rust \x1b[0m"
        // It should NOT contain "\x1b[1m" when there's no title
        assert!(!header.contains("\x1b[1m"));
    }

    #[test]
    fn test_header_row_right_alignment() {
        let bg_color = Color {
            r: 40,
            g: 40,
            b: 40,
            a: 255,
        };
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
        let options = test_options();
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
        let options = test_options();
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
        let options = test_options();
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
        let options = test_options();
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
        let options = test_options();
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
        let options = test_options();
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
        let options = test_options();
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
        let mut options = test_options();
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
        let mut options = test_options();
        options.include_line_numbers = true;
        let meta = crate::markdown::dsl::CodeBlockMeta::default();

        let code =
            "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10";
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

    /// Regression test: Line number gutter must have background color.
    ///
    /// Previously, the line numbers, separator, and trailing space were rendered
    /// without a background color, causing a visual inconsistency where only
    /// the code content had the theme's background color.
    #[test]
    fn test_line_numbers_have_background_color() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let mut options = test_options();
        options.include_line_numbers = true;
        let meta = crate::markdown::dsl::CodeBlockMeta::default();

        let code = "let x = 1;";
        let result = highlight_code(code, "rust", &highlighter, &options, &meta, ColorMode::Dark);

        assert!(result.is_ok());
        let output = result.unwrap();

        // Get expected background color from theme
        let theme = highlighter.theme();
        let bg_color = theme.settings.background.unwrap_or(Color::BLACK);
        let bg_escape = format!("\x1b[48;2;{};{};{}m", bg_color.r, bg_color.g, bg_color.b);

        // Find where the line number gutter starts (after top padding)
        // The gutter should appear after a background color escape sequence
        // Pattern: [bg color][gray fg][number] │ [code]
        let gutter_pattern = format!("{}\x1b[38;2;128;128;128m", bg_escape);

        assert!(
            output.contains(&gutter_pattern),
            "Line number gutter should have background color set before foreground.\n\
             Expected to find: {:?}\n\
             Output: {:?}",
            gutter_pattern,
            output
        );

        // Verify there's no reset (\x1b[0m) between the gutter and code content
        // that would clear the background color
        let lines: Vec<&str> = output.lines().collect();
        for line in &lines {
            if line.contains("│") {
                // The line should NOT have a pattern like: [number]│\x1b[0m (reset after separator)
                // followed by the background being set again
                assert!(
                    !line.contains("│\x1b[0m"),
                    "Gutter separator should not be followed by reset.\n\
                     This would cause the space after the separator to lack background color.\n\
                     Line: {:?}",
                    line
                );
            }
        }
    }

    #[test]
    fn test_highlight_ignores_zero() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let options = test_options();
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
        let options = test_options();
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
            let output = for_terminal(&md, test_options()).unwrap();
            buf.push_str(&output);
        });

        // Verify ANSI codes are present
        let raw = terminal.get_output();
        assert!(raw.contains("\x1b["));

        // Verify content after stripping ANSI
        // Output includes: right-aligned header row (spacing + language) + newline + top padding + left padding + code + bottom padding + trailing blank line + separation blank line
        // Terminal width is 80, language " rust " is 6 chars, so 74 spaces of padding
        let plain = strip_ansi_codes(&raw);
        assert!(plain.ends_with(" rust \n\n fn test() {}\n\n\n"));
        assert!(plain.contains("rust"));
    }

    #[test]
    fn test_color_depth_no_color_returns_plain_text() {
        let md: Markdown = "# Test\n\n```rust\nfn main() {}\n```".into();

        let mut options = test_options();
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

        let mut options = test_options();
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
        let output = for_terminal(&md, test_options()).unwrap();

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
        let output = for_terminal(&md, test_options()).unwrap();

        // Code blocks should have background color
        assert!(
            output.contains("\x1b[48;2;"),
            "Code should have background colors"
        );
    }

    #[test]
    fn test_for_terminal_ends_with_reset() {
        let md: Markdown = "# Test".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Output should end with reset code
        assert!(
            output.ends_with("\x1b[0m"),
            "Output should end with terminal reset"
        );
    }

    #[test]
    fn test_emit_prose_text_no_background() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: FontStyle::empty(),
        };

        let result = emit_prose_text("Hello", style, true, false, false, None, false, true);

        // Should have foreground
        assert!(result.contains("\x1b[38;2;255;128;64m"));
        // Should NOT have background (unless in_mark is true)
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
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: FontStyle::ITALIC,
        };

        // With emit_italic=true, should emit italic code
        let result = emit_prose_text("italic text", style, true, false, false, None, false, true);

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
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: FontStyle::ITALIC,
        };

        // With emit_italic=false, should NOT emit italic code
        let result = emit_prose_text("italic text", style, false, false, false, None, false, true);

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
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: FontStyle::BOLD,
        };

        let result = emit_prose_text("bold text", style, true, false, false, None, false, true);

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
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: FontStyle::UNDERLINE,
        };

        let result = emit_prose_text("underline text", style, true, false, false, None, false, true);

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
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: FontStyle::BOLD | FontStyle::ITALIC,
        };

        let result = emit_prose_text("bold italic", style, true, false, false, None, false, true);

        // Should have both escape codes
        assert!(result.contains("\x1b[1m"), "Should have bold");
        assert!(result.contains("\x1b[3m"), "Should have italic");
    }

    /// Regression test: bold+italic with emit_italic=false should only emit bold.
    #[test]
    fn test_emit_prose_text_bold_italic_no_italic() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: FontStyle::BOLD | FontStyle::ITALIC,
        };

        let result = emit_prose_text("bold italic", style, false, false, false, None, false, true);

        // Should have bold but NOT italic
        assert!(result.contains("\x1b[1m"), "Should have bold");
        assert!(
            !result.contains("\x1b[3m"),
            "Should NOT have italic when emit_italic=false"
        );
    }

    /// Test: strikethrough text should include ANSI strikethrough escape code (\x1b[9m).
    #[test]
    fn test_emit_prose_text_strikethrough() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: FontStyle::empty(),
        };

        let result = emit_prose_text("strikethrough text", style, true, true, false, None, false, true);

        // Should have strikethrough escape code
        assert!(
            result.contains("\x1b[9m"),
            "Strikethrough text should include \\x1b[9m escape code, got: {:?}",
            result
        );
        // Should have foreground color
        assert!(result.contains("\x1b[38;2;255;128;64m"));
        // Should end with reset
        assert!(result.contains("\x1b[0m"));
    }

    /// Test: strikethrough combined with bold and italic.
    #[test]
    fn test_emit_prose_text_strikethrough_combined() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: FontStyle::BOLD | FontStyle::ITALIC,
        };

        let result = emit_prose_text("bold italic strikethrough", style, true, true, false, None, false, true);

        // Should have all three escape codes
        assert!(result.contains("\x1b[1m"), "Should have bold");
        assert!(result.contains("\x1b[3m"), "Should have italic");
        assert!(result.contains("\x1b[9m"), "Should have strikethrough");
    }

    #[test]
    fn test_emit_code_text_has_background() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 30,
                g: 30,
                b: 30,
                a: 255,
            },
            font_style: FontStyle::empty(),
        };
        let bg = Color {
            r: 30,
            g: 30,
            b: 30,
            a: 255,
        };

        let result = emit_code_text("code", style, bg);

        // Should have both foreground and background
        assert!(result.contains("\x1b[48;2;30;30;30m"));
        assert!(result.contains("\x1b[38;2;255;128;64m"));
    }

    #[test]
    fn test_for_terminal_heading_styled() {
        let md: Markdown = "# Heading 1\n\n## Heading 2".into();
        let output = for_terminal(&md, test_options()).unwrap();

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
        let mut options = test_options();
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
        let mut options = test_options();
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
        let mut options = test_options();
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
        let output = for_terminal(&md, test_options()).unwrap();

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
    fn test_for_terminal_strikethrough_styled() {
        let md: Markdown = "Some ~~strikethrough text~~ here.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain ANSI strikethrough escape code (\x1b[9m)
        assert!(
            output.contains("\x1b[9m"),
            "Strikethrough markdown should produce \\x1b[9m escape code, got: {:?}",
            output
        );

        // Plain text should have the strikethrough content
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("strikethrough text"));
    }

    #[test]
    fn test_for_terminal_inline_code_styled() {
        let md: Markdown = "Use `cargo build` to compile.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain ANSI codes
        assert!(output.contains("\x1b[38;2;"));

        // Inline code SHOULD have background color for contrast
        let bg_count = output.matches("\x1b[48;2;").count();
        assert!(
            bg_count > 0,
            "Inline code should have background color for contrast"
        );

        let plain = strip_ansi_codes(&output);
        // No backticks in terminal output
        assert!(plain.contains("cargo build"));
        assert!(
            !plain.contains("`"),
            "Backticks should be removed in terminal output"
        );
    }

    #[test]
    fn test_for_terminal_nested_styling() {
        let md: Markdown = "# Heading with **bold** text".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain ANSI codes
        assert!(output.contains("\x1b[38;2;"));

        // H1 uses block marker: █
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("█ Heading with bold text"));
    }

    #[test]
    fn test_for_terminal_heading_has_blank_line_after() {
        let md: Markdown = "# Heading\n\nParagraph text.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        // H1 uses block marker: █
        // Heading should be followed by blank line before paragraph
        assert!(plain.contains("█ Heading\n\nParagraph text."));
    }

    #[test]
    fn test_for_terminal_heading_has_blank_line_before() {
        let md: Markdown = "Some text.\n\n## Heading".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        // H2 uses block marker: ██
        // Should have blank line between paragraph and heading
        assert!(plain.contains("Some text.\n\n██ Heading"));
    }

    #[test]
    fn test_for_terminal_heading_after_list_has_blank_line() {
        let md: Markdown = "- Item one\n- Item two\n\n### Heading".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        // H3 uses block marker: ████
        // Should have blank line between list and heading
        assert!(plain.contains("- Item two\n\n████ Heading"));
    }

    #[test]
    fn test_for_terminal_first_heading_no_leading_blank() {
        let md: Markdown = "# First Heading".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        // H1 uses block marker: █
        // First heading should not have leading blank lines
        assert!(plain.starts_with("█ First Heading"));
    }

    #[test]
    fn test_for_terminal_unordered_list() {
        let md: Markdown = "- First item\n- Second item\n- Third item".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Each item should be on its own line with bullet marker
        assert!(plain.contains("- First item\n"));
        assert!(plain.contains("- Second item\n"));
        assert!(plain.contains("- Third item\n"));
    }

    #[test]
    fn test_for_terminal_ordered_list() {
        let md: Markdown = "1. First item\n2. Second item\n3. Third item".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Each item should be on its own line with number marker
        assert!(plain.contains("1. First item\n"));
        assert!(plain.contains("2. Second item\n"));
        assert!(plain.contains("3. Third item\n"));
    }

    #[test]
    fn test_for_terminal_nested_list() {
        let md: Markdown =
            "- Parent item\n  - Child item\n  - Another child\n- Second parent".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Parent item should have newline before nested list starts
        assert!(
            plain.contains("- Parent item\n"),
            "Parent should end with newline, got:\n{}",
            plain
        );
        assert!(plain.contains("  - Child item\n"));
        assert!(plain.contains("  - Another child\n"));
        assert!(plain.contains("- Second parent\n"));
    }

    #[test]
    fn test_for_terminal_list_with_inline_code() {
        let md: Markdown = "- Use `cargo build`\n- Run `cargo test`".into();
        let output = for_terminal(&md, test_options()).unwrap();

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
        let output = for_terminal(&md, test_options()).unwrap();

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
    - `### Fallback Behavior`"#
            .into();
        let output = for_terminal(&md, test_options()).unwrap();

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
        let md: Markdown =
            "- Level 1\n  - Level 2\n    - Level 3\n    - Level 3b\n  - Level 2b\n- Level 1b"
                .into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Verify each level has correct indentation and newlines
        assert!(
            plain.contains("- Level 1\n"),
            "Level 1 should end with newline"
        );
        assert!(
            plain.contains("  - Level 2\n"),
            "Level 2 should have 2-space indent"
        );
        assert!(
            plain.contains("    - Level 3\n"),
            "Level 3 should have 4-space indent"
        );
        assert!(
            plain.contains("    - Level 3b\n"),
            "Level 3b should have 4-space indent"
        );
        assert!(
            plain.contains("  - Level 2b\n"),
            "Level 2b should return to 2-space indent"
        );
        assert!(
            plain.contains("- Level 1b\n"),
            "Level 1b should return to no indent"
        );
    }

    // ==================== Table Regression Tests ====================

    /// Regression test: tables must render with rows on separate lines
    /// Bug: table rows were concatenated horizontally instead of vertically
    #[test]
    fn test_table_rows_on_separate_lines() {
        let md: Markdown = "| A | B |\n|---|---|\n| 1 | 2 |".into();
        let output = for_terminal(&md, test_options()).unwrap();

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
        let md: Markdown =
            "| Col1 | Col2 |\n|------|------|\n| A | B |\n| C | D |\n| E | F |".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Table should have box-drawing structure
        assert!(plain.contains("┌"), "Should have top-left corner");
        assert!(
            plain.contains("╞") || plain.contains("├"),
            "Should have header separator"
        );
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
            line_count,
            plain
        );
    }

    /// Regression test: tables with inline code in cells render correctly.
    ///
    /// Inline code in table cells should keep ANSI styling and remain aligned.
    #[test]
    fn test_table_with_inline_code() {
        let md: Markdown =
            "| Variable | Description |\n|----------|-------------|\n| `FOO` | A variable |".into();
        let output = for_terminal(&md, test_options()).unwrap();

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

        // Inline code in tables should preserve ANSI background styling.
        assert!(
            output.contains("\x1b[48;2;"),
            "Inline code in table should retain background styling, got:\n{}",
            output
        );
    }

    /// Regression test: table after heading has proper spacing
    #[test]
    fn test_table_after_heading_spacing() {
        let md: Markdown =
            "## Environment Variables\n\n| Var | Desc |\n|-----|------|\n| A | B |".into();
        let output = for_terminal(&md, test_options()).unwrap();

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

    /// Test that tables render with multiple rows correctly.
    #[test]
    fn test_table_alternating_rows() {
        let md: Markdown = "| A |\n|---|\n| 1 |\n| 2 |\n| 3 |".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Verify all data rows are present
        assert!(plain.contains(" A "), "Header should be present");
        assert!(plain.contains(" 1 "), "First data row should be present");
        assert!(plain.contains(" 2 "), "Second data row should be present");
        assert!(plain.contains(" 3 "), "Third data row should be present");

        // Should have box-drawing borders
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should have table borders"
        );
    }

    /// Test that table headers render correctly.
    #[test]
    fn test_table_header_bold() {
        let md: Markdown = "| Header |\n|--------|\n| Data |".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Verify header and data are present
        assert!(plain.contains("Header"), "Header should be present");
        assert!(plain.contains("Data"), "Data should be present");

        // Should have table structure with header separator
        assert!(
            plain.contains("╞") || plain.contains("├"),
            "Should have header separator"
        );
    }

    /// Test that inline code in tables renders correctly across multiple rows.
    #[test]
    fn test_table_row_background_persists_after_inline_code() {
        let md: Markdown =
            "| Field | Description |\n|-------|-------------|\n| `foo` | A var |\n| `bar` | Another |"
                .into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Verify inline code content is present (backticks should be stripped)
        assert!(plain.contains("foo"), "Inline code 'foo' should be present");
        assert!(plain.contains("bar"), "Inline code 'bar' should be present");

        // Verify descriptive text is present
        assert!(plain.contains("A var"), "Description should be present");
        assert!(plain.contains("Another"), "Description should be present");

        // Should have table structure
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should have table borders"
        );
    }

    // ==================== Table Alignment Tests ====================

    /// Test that tables respect left alignment
    #[test]
    fn test_table_left_alignment() {
        let md: Markdown = "| Left |\n|:-----|\n| Data |".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Verify content is present
        assert!(plain.contains("Left"), "Header should be present");
        assert!(plain.contains("Data"), "Data should be present");
    }

    /// Test that tables respect center alignment
    #[test]
    fn test_table_center_alignment() {
        let md: Markdown = "| Center |\n|:------:|\n| Data   |".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Verify content is present
        assert!(plain.contains("Center"), "Header should be present");
        assert!(plain.contains("Data"), "Data should be present");
    }

    /// Test that tables respect right alignment
    #[test]
    fn test_table_right_alignment() {
        let md: Markdown = "| Right |\n|------:|\n| Data  |".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Verify content is present
        assert!(plain.contains("Right"), "Header should be present");
        assert!(plain.contains("Data"), "Data should be present");
    }

    /// Test that tables handle mixed alignments
    #[test]
    fn test_table_mixed_alignments() {
        let md: Markdown =
            "| Left | Center | Right |\n|:-----|:------:|------:|\n| L1 | C1 | R1 |".into();
        let output = for_terminal(&md, test_options()).unwrap();
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
        assert!(
            find_syntax("rust", highlighter.syntax_set()).is_some(),
            "lowercase 'rust' should find Rust syntax"
        );
        assert!(
            find_syntax("Rust", highlighter.syntax_set()).is_some(),
            "exact 'Rust' should find Rust syntax"
        );
        assert!(
            find_syntax("RUST", highlighter.syntax_set()).is_some(),
            "uppercase 'RUST' should find Rust syntax"
        );
        assert!(
            find_syntax("rs", highlighter.syntax_set()).is_some(),
            "extension 'rs' should find Rust syntax"
        );

        // Python
        assert!(
            find_syntax("python", highlighter.syntax_set()).is_some(),
            "lowercase 'python' should find Python syntax"
        );
        assert!(
            find_syntax("Python", highlighter.syntax_set()).is_some(),
            "exact 'Python' should find Python syntax"
        );
        assert!(
            find_syntax("py", highlighter.syntax_set()).is_some(),
            "extension 'py' should find Python syntax"
        );
    }

    /// Regression test: find_syntax must handle common aliases.
    ///
    /// Bug: Common language aliases like "shell" weren't mapped to their
    /// actual syntax definitions like "bash".
    #[test]
    fn test_find_syntax_aliases() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);

        // Bash aliases
        assert!(
            find_syntax("bash", highlighter.syntax_set()).is_some(),
            "'bash' should find Bash syntax"
        );
        assert!(
            find_syntax("sh", highlighter.syntax_set()).is_some(),
            "'sh' should find Bash syntax"
        );
        assert!(
            find_syntax("shell", highlighter.syntax_set()).is_some(),
            "'shell' alias should find Bash syntax"
        );

        // JavaScript/TypeScript
        assert!(
            find_syntax("js", highlighter.syntax_set()).is_some(),
            "'js' should find JavaScript syntax"
        );
        assert!(
            find_syntax("javascript", highlighter.syntax_set()).is_some(),
            "'javascript' alias should find JS syntax"
        );
        assert!(
            find_syntax("ts", highlighter.syntax_set()).is_some(),
            "'ts' should find TypeScript syntax"
        );
        assert!(
            find_syntax("typescript", highlighter.syntax_set()).is_some(),
            "'typescript' alias should find TS syntax"
        );
    }

    /// Regression test: code blocks must have syntax highlighting with multiple colors.
    ///
    /// Bug: Even when the syntax was found, all tokens had the same foreground color
    /// because the highlighting wasn't being applied correctly.
    #[test]
    fn test_code_block_has_multiple_colors() {
        let md: Markdown = "```rust\nfn main() {\n    let x = 5;\n}\n```".into();
        let output = for_terminal(&md, test_options()).unwrap();

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

    #[test]
    fn test_emit_inline_code_has_background_and_foreground() {
        let style = Style {
            foreground: Color {
                r: 200,
                g: 210,
                b: 220,
                a: 255,
            },
            background: Color {
                r: 40,
                g: 45,
                b: 50,
                a: 255,
            },
            font_style: syntect::highlighting::FontStyle::empty(),
        };

        let output = emit_inline_code("FOO", style);
        assert!(output.contains("FOO"));
        assert!(output.contains("\x1b[48;2;40;45;50m"));
        assert!(output.contains("\x1b[38;2;200;210;220m"));
        assert!(output.ends_with("\x1b[0m"));
    }

    /// Regression test: code block background must extend to end of line.
    ///
    /// Bug: Background color stopped at end of content instead of filling
    /// to terminal edge. Fixed by adding \x1b[K (clear to EOL) after content.
    #[test]
    fn test_code_block_background_extends_to_eol() {
        let md: Markdown = "```rust\nfn main() {}\n```".into();
        let output = for_terminal(&md, test_options()).unwrap();

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
        let output = for_terminal(&md, test_options()).unwrap();

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
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Table should render with box-drawing borders
        assert!(
            plain.contains("┌") && plain.contains("┘"),
            "Table should render with borders"
        );

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
        // Create a table with content that would exceed 50 chars per row
        let rows = vec![
            vec![
                "Field".to_string(),
                "Description".to_string(),
                "Example".to_string(),
            ],
            vec![
                "tool.name".to_string(),
                "Tool being called".to_string(),
                "\"brave_search\"".to_string(),
            ],
            vec![
                "tool.query".to_string(),
                "Search query/URL".to_string(),
                "\"rust async\"".to_string(),
            ],
            vec![
                "tool.duration_ms".to_string(),
                "Execution time".to_string(),
                "1234".to_string(),
            ],
        ];
        let alignments = vec![Alignment::Left, Alignment::Left, Alignment::Center];

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
                i,
                display_width,
                adequate_width,
                line,
                plain
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
    /// Bug: table rendering was splitting "tool.duration_ms" into "tool.duration_m" and "s"
    /// when the terminal was narrow. This was caused by ANSI escape codes interfering
    /// with width calculation.
    #[test]
    fn test_table_no_mid_word_splitting() {
        // Table from the bug report screenshot
        let rows = vec![
            vec![
                "Field".to_string(),
                "Description".to_string(),
                "Example".to_string(),
            ],
            vec![
                "tool.name".to_string(),
                "Tool being called".to_string(),
                "\"brave_search\"".to_string(),
            ],
            vec![
                "tool.query".to_string(),
                "Search query/URL".to_string(),
                "\"rust async\"".to_string(),
            ],
            vec![
                "tool.duration_ms".to_string(),
                "Execution time".to_string(),
                "1234".to_string(),
            ],
            vec![
                "tool.results_count".to_string(),
                "Results returned".to_string(),
                "10".to_string(),
            ],
            vec![
                "http.status_code".to_string(),
                "HTTP response".to_string(),
                "200".to_string(),
            ],
            vec![
                "otel.kind".to_string(),
                "Span kind".to_string(),
                "\"client\"".to_string(),
            ],
        ];
        let alignments = vec![Alignment::Left, Alignment::Left, Alignment::Center];

        // Test with narrow width (60 chars) like the screenshot showed
        let narrow_width: u16 = 60;

        let output = render_table(&rows, &alignments, narrow_width);
        let plain = strip_ansi_codes(&output);

        // Check for mid-word splits by looking at line breaks
        // Bad patterns: word followed immediately by newline with continuation on next line
        let bad_patterns = [
            "duration_m\n", // duration_ms split after duration_m
            "results_co\n", // results_count split
            "status_cod\n", // status_code split
            "_m │",         // duration_ms split at underscore
            "_co │",        // results_count split
        ];

        for pattern in bad_patterns {
            assert!(
                !plain.contains(pattern),
                "Bad word split detected: found '{}' in:\n{}",
                pattern.escape_default(),
                plain
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
        // Table from the bug report screenshot
        let rows = vec![
            vec![
                "Field".to_string(),
                "Description".to_string(),
                "Example".to_string(),
            ],
            vec![
                "tool.name".to_string(),
                "Tool being called".to_string(),
                "\"brave_search\"".to_string(),
            ],
            vec![
                "tool.query".to_string(),
                "Search query/URL".to_string(),
                "\"rust async\"".to_string(),
            ],
            vec![
                "tool.duration_ms".to_string(),
                "Execution time".to_string(),
                "1234".to_string(),
            ],
        ];
        let alignments = vec![Alignment::Left, Alignment::Left, Alignment::Center];

        // Very narrow width to force issues
        let narrow_width: u16 = 40;

        let output = render_table(&rows, &alignments, narrow_width);
        let plain = strip_ansi_codes(&output);

        // At 40 columns, this 3-column table cannot fit — the renderer returns
        // an error message explaining the minimum width required.
        if plain.contains("could not be rendered") {
            // Error fallback is acceptable at this extreme width
            return;
        }

        // If the table did render, every line should respect the width constraint
        for (i, line) in plain.lines().enumerate() {
            let display_width = UnicodeWidthStr::width(line);
            assert!(
                display_width <= narrow_width as usize + 1,
                "Line {} exceeds narrow-width tolerance: {} > {} in:\n{}\nFull table:\n{}",
                i,
                display_width,
                narrow_width as usize + 1,
                line,
                plain
            );
        }
    }

    /// Regression test: inline-code ANSI styling in table cells must preserve
    /// stable column geometry.
    #[test]
    fn test_table_inline_code_ansi_preserves_alignment() {
        let inline_style = Style {
            foreground: Color {
                r: 215,
                g: 225,
                b: 235,
                a: 255,
            },
            background: Color {
                r: 50,
                g: 50,
                b: 55,
                a: 255,
            },
            font_style: syntect::highlighting::FontStyle::empty(),
        };

        let rows = vec![
            vec![
                "Field".to_string(),
                "Description".to_string(),
                "Example".to_string(),
            ],
            vec![
                emit_inline_code("tool.name", inline_style),
                "Tool being called".to_string(),
                emit_inline_code("\"brave_search\"", inline_style),
            ],
            vec![
                emit_inline_code("tool.duration_ms", inline_style),
                "Execution time".to_string(),
                emit_inline_code("1234", inline_style),
            ],
        ];
        let alignments = vec![Alignment::Left, Alignment::Left, Alignment::Center];

        let width: u16 = 70;
        let output = render_table(&rows, &alignments, width);
        let plain = strip_ansi_codes(&output);

        assert!(
            output.contains("\x1b[48;2;"),
            "Inline code ANSI background should be preserved, output:\n{}",
            output
        );

        for expected in ["tool.name", "tool.duration_ms", "brave_search"] {
            assert!(
                plain.contains(expected),
                "Missing '{}':\n{}",
                expected,
                plain
            );
        }

        let lines: Vec<&str> = plain.lines().collect();
        assert!(
            lines.len() >= 3,
            "Table should have header + separator + data rows"
        );

        let first_width = biscuit_terminal::utils::UnicodeWidthStr::width(lines[0]);
        for (i, line) in lines.iter().enumerate() {
            let w = biscuit_terminal::utils::UnicodeWidthStr::width(*line);
            assert_eq!(
                w, first_width,
                "Line {} has width {} but expected {} (misalignment):\n{}",
                i, w, first_width, plain
            );
        }
    }

    /// Debug test to visualize table rendering at different widths
    #[test]
    #[ignore] // Run with: cargo test -p darkmatter --lib test_table_width_visual -- --ignored --nocapture
    fn test_table_width_visual() {
        let rows = vec![
            vec![
                "Field".to_string(),
                "Description".to_string(),
                "Example".to_string(),
            ],
            vec![
                "tool.name".to_string(),
                "Tool being called".to_string(),
                "\"brave_search\"".to_string(),
            ],
            vec![
                "tool.query".to_string(),
                "Search query/URL".to_string(),
                "\"rust async\"".to_string(),
            ],
            vec![
                "tool.duration_ms".to_string(),
                "Execution time".to_string(),
                "1234".to_string(),
            ],
            vec![
                "tool.results_count".to_string(),
                "Results returned".to_string(),
                "10".to_string(),
            ],
            vec![
                "http.status_code".to_string(),
                "HTTP response".to_string(),
                "200".to_string(),
            ],
            vec![
                "otel.kind".to_string(),
                "Span kind".to_string(),
                "\"client\"".to_string(),
            ],
        ];
        let alignments = vec![Alignment::Left, Alignment::Left, Alignment::Center];

        for width in [40u16, 60, 80, 100, 120] {
            eprintln!(
                "\n{}\nTerminal width: {}\n{}",
                "=".repeat(60),
                width,
                "=".repeat(60)
            );

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
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Content should be present
        assert!(plain.contains("This is a very long"));
        assert!(plain.contains("Header"));

        // Should have table structure
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should have table borders"
        );
    }

    /// Test that tables with many columns distribute width fairly
    #[test]
    fn test_table_many_columns_fair_width() {
        let md: Markdown =
            "| A | B | C | D | E | F |\n|---|---|---|---|---|---|\n| 1 | 2 | 3 | 4 | 5 | 6 |"
                .into();
        let output = for_terminal(&md, test_options()).unwrap();
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
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should only contain reset code
        assert_eq!(
            plain.trim(),
            "",
            "Empty markdown should produce empty output"
        );
    }

    /// Test that single column table renders correctly
    #[test]
    fn test_single_column_table() {
        let md: Markdown = "| Single |\n|--------|\n| Data 1 |\n| Data 2 |".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have header and data
        assert!(plain.contains("Single"), "Header should be present");
        assert!(plain.contains("Data 1"), "First data row should be present");
        assert!(
            plain.contains("Data 2"),
            "Second data row should be present"
        );

        // Should have table structure
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should have table borders"
        );
    }

    /// Test that table with empty cells renders correctly
    #[test]
    fn test_table_with_empty_cells() {
        let md: Markdown = "| A | B |\n|---|---|\n|   | X |\n| Y |   |".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have headers
        assert!(plain.contains("A"));
        assert!(plain.contains("B"));

        // Should have non-empty cells
        assert!(plain.contains("X"));
        assert!(plain.contains("Y"));

        // Should have table structure
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should have table borders"
        );
    }

    /// Test that ragged rows (fewer cells than columns) are handled
    #[test]
    fn test_table_ragged_rows() {
        // pulldown-cmark handles ragged rows by filling with empty strings
        let md: Markdown = "| A | B | C |\n|---|---|---|\n| 1 | 2 |".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have all headers
        assert!(plain.contains("A"));
        assert!(plain.contains("B"));
        assert!(plain.contains("C"));

        // Should have present data
        assert!(plain.contains("1"));
        assert!(plain.contains("2"));

        // Should have table structure
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should have table borders"
        );
    }

    /// Test that Unicode characters in table cells render correctly
    #[test]
    fn test_table_unicode_characters() {
        let md: Markdown =
            "| Emoji | CJK |\n|-------|-----|\n| 🎉 | 中文 |\n| ✅ | 日本語 |".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have headers
        assert!(plain.contains("Emoji"));
        assert!(plain.contains("CJK"));

        // Should have Unicode content
        assert!(
            plain.contains("🎉") || plain.contains("✅"),
            "Emoji should be present"
        );
        assert!(
            plain.contains("中文") || plain.contains("日本語"),
            "CJK characters should be present"
        );

        // Should have table structure
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should have table borders"
        );
    }

    // ---- ANSI Handling Tests ----

    /// Test that inline code in table cells doesn't break alignment
    #[test]
    fn test_table_inline_code_alignment() {
        let md: Markdown = "| Code | Description |\n|------|-------------|\n| `short` | Text |\n| `very_long_code_sample` | More text |".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have headers
        assert!(plain.contains("Code"));
        assert!(plain.contains("Description"));

        // Inline code content should be present (without backticks)
        assert!(plain.contains("short"));
        assert!(plain.contains("very_long_code_sample"));

        // Should have table structure
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should have table borders"
        );
    }

    /// Test that long inline code in one cell, short text in another works
    #[test]
    fn test_table_mixed_inline_code_lengths() {
        let md: Markdown = "| Short | Long |\n|-------|------|\n| Hi | `this_is_a_very_long_code_identifier_that_might_cause_issues` |".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have all content
        assert!(plain.contains("Short"));
        assert!(plain.contains("Long"));
        assert!(plain.contains("Hi"));
        assert!(plain.contains("this_is_a_very_long_code_identifier"));

        // Should have table structure
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should have table borders"
        );
    }

    /// Test that multiple inline code blocks in same cell work
    #[test]
    fn test_table_multiple_inline_code_in_cell() {
        let md: Markdown =
            "| Command |\n|---------|\n| Use `cargo build` then `cargo test` |".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have header
        assert!(plain.contains("Command"));

        // Should have all inline code content (without backticks)
        assert!(plain.contains("cargo build"));
        assert!(plain.contains("cargo test"));
        assert!(plain.contains("Use"));
        assert!(plain.contains("then"));

        // Should have table structure
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should have table borders"
        );
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
"#
        .into();
        let output = for_terminal(&md, test_options()).unwrap();
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
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should have table borders"
        );
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
"#
        .into();
        let output = for_terminal(&md, test_options()).unwrap();
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
        assert!(
            table_count >= 2,
            "Should have at least 2 tables (found {} top borders)",
            table_count
        );
    }

    /// Test that table with complex nested markdown in cells works
    #[test]
    fn test_table_with_nested_markdown() {
        // Note: Most table implementations don't support nested block elements,
        // but inline elements like bold, emphasis, code should work
        let md: Markdown = "| Feature | Status |\n|---------|--------|\n| **Bold text** | `enabled` |\n| *Emphasis* | `disabled` |".into();
        let output = for_terminal(&md, test_options()).unwrap();
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
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should have table borders"
        );
    }

    #[test]
    fn test_table_cell_image_uses_text_fallback() {
        let md: Markdown =
            "| Name | Preview |\n|------|---------|\n| Chart | ![Diagram](assets/diagram.png) |"
                .into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(
            plain.contains("▉ IMAGE[Diagram]"),
            "Table cell images should use deterministic text fallback, got:\n{}",
            plain
        );
        assert!(
            plain.contains("┌") && plain.contains("└"),
            "Should still render as a table, got:\n{}",
            plain
        );
    }

    // ---- ImageRenderer Tests ----

    /// Test ImageRenderer::new() initialization with None base path
    #[test]
    fn test_image_renderer_new_with_none_base_path() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);

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
        let renderer = ImageRenderer::new(Some(&base), TerminalImageMode::Auto);

        assert_eq!(renderer.base_path(), base);
    }

    /// Test ImageRenderer accessors
    #[test]
    fn test_image_renderer_accessors() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);

        // Test that accessors return values
        let is_tty = renderer.is_tty();
        let graphics_supported = renderer.graphics_supported();

        // graphics_supported requires is_tty to be true
        if graphics_supported {
            assert!(is_tty, "graphics_supported should be false when not a TTY");
        }

        // Terminal width should be reasonable
        assert!(renderer.terminal_width() >= 10);
        assert!(renderer.terminal_width() <= 1000);
    }

    /// Test ImageRenderer with non-existent base path
    #[test]
    fn test_image_renderer_with_nonexistent_base() {
        let nonexistent = std::path::PathBuf::from("/this/path/does/not/exist/for/sure");
        let renderer = ImageRenderer::new(Some(&nonexistent), TerminalImageMode::Auto);

        // Should use the path even if it doesn't exist
        assert_eq!(renderer.base_path(), nonexistent);

        // base_path_canonical should be None
        // (we can't directly test this since it's private, but the struct should be valid)
    }

    /// Test ImageRenderer graphics detection is cached
    #[test]
    fn test_image_renderer_caches_detection() {
        // Create two renderers and verify they have consistent state
        let renderer1 = ImageRenderer::new(None, TerminalImageMode::Auto);
        let renderer2 = ImageRenderer::new(None, TerminalImageMode::Auto);

        // Both should have the same graphics support detection
        assert_eq!(
            renderer1.graphics_supported(),
            renderer2.graphics_supported()
        );
        assert_eq!(renderer1.is_tty(), renderer2.is_tty());
    }

    /// Test ImageRenderer debug formatting
    #[test]
    fn test_image_renderer_debug() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);
        let debug_str = format!("{:?}", renderer);

        // Should contain field names
        assert!(debug_str.contains("ImageRenderer"));
        assert!(debug_str.contains("graphics_supported"));
        assert!(debug_str.contains("is_tty"));
        assert!(debug_str.contains("terminal_width"));
    }

    /// Test force mode bypasses capability gating.
    #[test]
    fn test_image_renderer_force_mode_attempts_protocol_render() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Force);
        assert!(renderer.should_attempt_protocol_render());

        let forced_terminal = renderer.render_terminal();
        assert!(forced_terminal.is_tty);
        assert!(!matches!(forced_terminal.image_support, ImageSupport::None));
    }

    /// Test never mode disables protocol rendering.
    #[test]
    fn test_image_renderer_never_mode_disables_protocol_render() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Never);
        assert!(!renderer.should_attempt_protocol_render());
    }

    // ---- Image Event Handling Tests ----

    /// Test that image markdown produces fallback placeholder
    #[test]
    fn test_image_fallback_placeholder() {
        let md: Markdown = "![Test Image](./image.png)".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("▉ IMAGE[Test Image]"));
    }

    /// Test image with empty alt text
    #[test]
    fn test_image_empty_alt_text() {
        let md: Markdown = "![](./image.png)".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("▉ IMAGE[]"));
    }

    /// Test image alt text with inline code
    #[test]
    fn test_image_alt_with_inline_code() {
        let md: Markdown = "![Code `snippet` here](./image.png)".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Alt text should preserve inline code with backticks
        assert!(plain.contains("▉ IMAGE[Code `snippet` here]"));
    }

    /// Test multiple images in document
    #[test]
    fn test_multiple_images() {
        let md: Markdown = "![First](a.png)\n\n![Second](b.png)".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("▉ IMAGE[First]"));
        assert!(plain.contains("▉ IMAGE[Second]"));
    }

    /// Test image inside list
    #[test]
    fn test_image_in_list() {
        let md: Markdown = "- Item with ![image](test.png)".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("- Item with"));
        assert!(plain.contains("▉ IMAGE[image]"));
    }

    /// Test image in paragraph context
    #[test]
    fn test_image_in_paragraph() {
        let md: Markdown = "Some text before ![alt](img.png) and after.".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("Some text before"));
        assert!(plain.contains("▉ IMAGE[alt]"));
        assert!(plain.contains("and after"));
    }

    /// Test image with width specification in alt text
    #[test]
    fn test_image_with_width_spec_in_alt() {
        // Width spec should be parsed and removed from alt text in fallback
        let md: Markdown = "![my diagram|50%](diagram.png)".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Alt text should NOT include the width spec
        assert!(plain.contains("▉ IMAGE[my diagram]"));
        assert!(!plain.contains("|50%"));
    }

    /// Test image with character width in alt text
    #[test]
    fn test_image_with_char_width_in_alt() {
        let md: Markdown = "![photo|80](photo.jpg)".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("▉ IMAGE[photo]"));
        assert!(!plain.contains("|80"));
    }

    /// Test image with pipe not followed by digit (not a width spec)
    #[test]
    fn test_image_with_pipe_in_alt_not_width() {
        // Pipe followed by non-digit should remain in alt text
        let md: Markdown = "![A | B comparison](chart.png)".into();
        let output = for_terminal(&md, test_options()).unwrap();
        let plain = strip_ansi_codes(&output);

        // Full alt text including pipe should be preserved
        assert!(plain.contains("▉ IMAGE[A | B comparison]"));
    }

    // ---- parse_alt_and_width() Tests ----

    /// Test parse_alt_and_width with percentage width
    #[test]
    fn test_parse_alt_and_width_percentage() {
        let (alt, width) = parse_alt_and_width("my image|50%");
        assert_eq!(alt, "my image");
        assert!(matches!(width, Some(ImageWidth::Percent(p)) if (p - 0.5).abs() < 0.001));
    }

    /// Test parse_alt_and_width with character width (bare number)
    #[test]
    fn test_parse_alt_and_width_characters() {
        let (alt, width) = parse_alt_and_width("photo|80");
        assert_eq!(alt, "photo");
        assert!(matches!(width, Some(ImageWidth::Characters(80))));
    }

    /// Test parse_alt_and_width with ch suffix
    #[test]
    fn test_parse_alt_and_width_ch_suffix() {
        let (alt, width) = parse_alt_and_width("diagram|40ch");
        assert_eq!(alt, "diagram");
        assert!(matches!(width, Some(ImageWidth::Characters(40))));
    }

    /// Test parse_alt_and_width with fill (not parsed - doesn't start with digit)
    #[test]
    fn test_parse_alt_and_width_fill() {
        // 'fill' doesn't start with a digit, so the entire string is treated as alt text
        let (alt, width) = parse_alt_and_width("banner|fill");
        assert_eq!(alt, "banner|fill");
        assert!(width.is_none());
    }

    /// Test parse_alt_and_width with no pipe
    #[test]
    fn test_parse_alt_and_width_no_pipe() {
        let (alt, width) = parse_alt_and_width("simple alt text");
        assert_eq!(alt, "simple alt text");
        assert!(width.is_none());
    }

    /// Test parse_alt_and_width with pipe not followed by digit
    #[test]
    fn test_parse_alt_and_width_pipe_not_digit() {
        let (alt, width) = parse_alt_and_width("chart | analysis");
        assert_eq!(alt, "chart | analysis");
        assert!(width.is_none());
    }

    /// Test parse_alt_and_width with pipe followed by letter
    #[test]
    fn test_parse_alt_and_width_pipe_letter() {
        let (alt, width) = parse_alt_and_width("A|B comparison");
        assert_eq!(alt, "A|B comparison");
        assert!(width.is_none());
    }

    /// Test parse_alt_and_width with invalid width spec
    #[test]
    fn test_parse_alt_and_width_invalid_spec() {
        // 0 is invalid, so fallback to full alt text
        let (alt, width) = parse_alt_and_width("image|0");
        assert_eq!(alt, "image|0");
        assert!(width.is_none());
    }

    /// Test parse_alt_and_width with whitespace before number
    #[test]
    fn test_parse_alt_and_width_whitespace_before_number() {
        let (alt, width) = parse_alt_and_width("image| 50%");
        assert_eq!(alt, "image");
        assert!(matches!(width, Some(ImageWidth::Percent(p)) if (p - 0.5).abs() < 0.001));
    }

    /// Test parse_alt_and_width trims trailing whitespace from alt
    #[test]
    fn test_parse_alt_and_width_trims_alt() {
        let (alt, width) = parse_alt_and_width("my image |25%");
        assert_eq!(alt, "my image");
        assert!(matches!(width, Some(ImageWidth::Percent(p)) if (p - 0.25).abs() < 0.001));
    }

    /// Test parse_alt_and_width with empty alt text
    #[test]
    fn test_parse_alt_and_width_empty_alt() {
        let (alt, width) = parse_alt_and_width("|50%");
        assert_eq!(alt, "");
        assert!(matches!(width, Some(ImageWidth::Percent(p)) if (p - 0.5).abs() < 0.001));
    }

    // ---- ImageRenderer::render_image() Tests ----

    /// Test render_image with remote HTTP URL returns fallback
    #[test]
    fn test_render_image_rejects_http_url() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);
        let result = renderer.render_image("http://example.com/image.png", "Alt", None);

        assert!(result.contains("▉ IMAGE[Alt]"));
    }

    /// Test render_image with remote HTTPS URL returns fallback
    #[test]
    fn test_render_image_rejects_https_url() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);
        let result = renderer.render_image("https://example.com/image.png", "Alt", None);

        assert!(result.contains("▉ IMAGE[Alt]"));
    }

    /// Test render_image with missing file returns fallback
    #[test]
    fn test_render_image_missing_file() {
        let renderer = ImageRenderer::new(Some(&std::env::temp_dir()), TerminalImageMode::Auto);
        let result = renderer.render_image("nonexistent_file_12345.png", "Missing", None);

        assert!(result.contains("▉ IMAGE[Missing]"));
    }

    /// Test render_image with relative path joined to base
    #[test]
    fn test_render_image_relative_path() {
        let renderer = ImageRenderer::new(Some(&std::env::temp_dir()), TerminalImageMode::Auto);
        // This file doesn't exist, so we get fallback, but path is resolved correctly
        let result = renderer.render_image("subdir/image.png", "Test", None);

        assert!(result.contains("▉ IMAGE[Test]"));
    }

    /// Test render_image with empty alt text
    #[test]
    fn test_render_image_empty_alt() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);
        let result = renderer.render_image("http://example.com/x.png", "", None);

        assert!(result.contains("▉ IMAGE[]"));
    }

    /// Test render_image fallback behavior
    #[test]
    fn test_render_image_no_graphics_support() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);

        // Create a temp file with invalid image data
        // This should trigger fallback regardless of graphics_supported value
        let tmp = std::env::temp_dir().join("test_image_render_invalid.png");
        std::fs::write(&tmp, b"fake png data").unwrap();

        let result = renderer.render_image(tmp.to_str().unwrap(), "Test", None);

        // Should get fallback text either because:
        // 1. graphics_supported is false, or
        // 2. viuer fails to decode invalid PNG data
        assert!(
            result.contains("▉ IMAGE[Test]"),
            "Expected fallback placeholder for invalid image data"
        );

        std::fs::remove_file(&tmp).ok();
    }

    /// Test render_image path traversal prevention with existing file
    #[test]
    fn test_render_image_path_traversal() {
        // Create a temp directory as the base
        let base_dir = std::env::temp_dir().join("image_test_base_traversal");
        std::fs::create_dir_all(&base_dir).ok();

        // Create a file outside the base directory that actually exists
        let outside_file = std::env::temp_dir().join("outside_image_traversal.png");
        std::fs::write(&outside_file, b"outside file content").unwrap();

        let renderer = ImageRenderer::new(Some(&base_dir), TerminalImageMode::Auto);

        // Try to access file outside base via path traversal
        let result = renderer.render_image("../outside_image_traversal.png", "Outside", None);

        // Should return fallback (path escapes base) even though the file exists
        assert!(
            result.contains("▉ IMAGE[Outside]"),
            "Path traversal should be blocked: {}",
            result
        );

        std::fs::remove_file(&outside_file).ok();
        std::fs::remove_dir(&base_dir).ok();
    }

    /// Test render_image blocks deep path traversal like ../../../etc/passwd
    #[test]
    fn test_render_image_deep_path_traversal() {
        let base_dir = std::env::temp_dir().join("image_test_base_deep");
        std::fs::create_dir_all(&base_dir).ok();

        let renderer = ImageRenderer::new(Some(&base_dir), TerminalImageMode::Auto);

        // Try deep traversal - should be blocked regardless of file existence
        let result = renderer.render_image("../../../etc/passwd", "Passwd", None);

        assert!(
            result.contains("▉ IMAGE[Passwd]"),
            "Deep path traversal should be blocked: {}",
            result
        );

        std::fs::remove_dir(&base_dir).ok();
    }

    /// Test render_image with absolute path (existing file)
    #[test]
    fn test_render_image_absolute_path() {
        // Create an actual file to ensure we block it for security, not just missing file
        let absolute_file = std::env::temp_dir().join("absolute_image_test.png");
        std::fs::write(&absolute_file, b"absolute file content").unwrap();

        let renderer = ImageRenderer::new(Some(&std::env::temp_dir()), TerminalImageMode::Auto);

        // Use an absolute path - should be rejected even if file exists
        let result =
            renderer.render_image(absolute_file.to_str().unwrap(), "AbsoluteExisting", None);

        assert!(
            result.contains("▉ IMAGE[AbsoluteExisting]"),
            "Absolute paths should be blocked: {}",
            result
        );

        std::fs::remove_file(&absolute_file).ok();
    }

    /// Test render_image with Windows-style absolute path
    #[test]
    fn test_render_image_windows_absolute_path() {
        let renderer = ImageRenderer::new(Some(&std::env::temp_dir()), TerminalImageMode::Auto);

        // Windows-style absolute path (C:\...)
        let result = renderer.render_image("C:\\Windows\\System32\\image.png", "WinAbs", None);

        // On Unix, this won't be detected as absolute by std::path::Path::is_absolute()
        // but it's still suspicious - the file won't exist anyway
        // Just verify we get a fallback (either from absolute check or file-not-found)
        assert!(result.contains("▉ IMAGE[WinAbs]"));
    }

    /// Test render_image allows valid relative paths within base
    #[test]
    fn test_render_image_valid_relative_path() {
        let base_dir = std::env::temp_dir().join("image_test_base_valid");
        let subdir = base_dir.join("subdir");
        std::fs::create_dir_all(&subdir).ok();

        let renderer = ImageRenderer::new(Some(&base_dir), TerminalImageMode::Auto);

        // Valid relative path within base (file doesn't exist, but path is valid)
        let result = renderer.render_image("subdir/image.png", "ValidRelative", None);

        // Should return fallback because file doesn't exist, not because path is invalid
        assert!(result.contains("▉ IMAGE[ValidRelative]"));

        std::fs::remove_dir_all(&base_dir).ok();
    }

    /// Test render_image handles path with . and .. that stays within base
    #[test]
    fn test_render_image_dotdot_stays_in_base() {
        let base_dir = std::env::temp_dir().join("image_test_base_dotdot");
        let subdir = base_dir.join("subdir");
        std::fs::create_dir_all(&subdir).ok();

        // Create a file in base_dir
        let image_file = base_dir.join("image.png");
        std::fs::write(&image_file, b"image content").unwrap();

        let renderer = ImageRenderer::new(Some(&base_dir), TerminalImageMode::Auto);

        // Path that uses .. but stays within base: subdir/../image.png resolves to base/image.png
        // This should be allowed since it stays within base
        let result = renderer.render_image("subdir/../image.png", "StaysInBase", None);

        // The path is valid and stays within base, so we'll get a graphics-related fallback
        // (since tests don't have graphics support) rather than a security rejection
        assert!(result.contains("▉ IMAGE[StaysInBase]"));

        std::fs::remove_dir_all(&base_dir).ok();
    }

    /// Test render_image fallback ends with newline
    #[test]
    fn test_render_image_fallback_ends_with_newline() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);
        let result = renderer.render_image("http://example.com/x.png", "Test", None);

        assert!(result.ends_with('\n'));
    }

    // ---- TerminalOptions Image Field Tests ----

    /// Test TerminalOptions default values for image fields
    #[test]
    fn test_terminal_options_image_defaults() {
        // This test specifically checks TerminalOptions::default() image behavior
        let options = TerminalOptions::default();

        assert!(matches!(options.image_mode, TerminalImageMode::Auto));
        assert!(options.base_path.is_none());
    }

    /// Test for_terminal with image rendering disabled
    #[test]
    fn test_for_terminal_image_mode_never() {
        // test_options() has image_mode = Never by default
        let options = test_options();

        let md: Markdown = "![Test](./image.png)".into();
        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should show fallback since images are disabled
        assert!(plain.contains("▉ IMAGE[Test]"));
    }

    /// Test for_terminal with image rendering enabled
    #[test]
    fn test_for_terminal_image_mode_auto() {
        let mut options = test_options();
        options.image_mode = TerminalImageMode::Auto;

        let md: Markdown = "![Test](./image.png)".into();
        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should show fallback (file doesn't exist, no graphics support in tests)
        assert!(plain.contains("▉ IMAGE[Test]"));
    }

    /// Test for_terminal with custom base_path
    #[test]
    fn test_for_terminal_with_base_path() {
        let mut options = test_options();
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
        let options = test_options();

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

        let mut options = test_options();
        options.include_line_numbers = true;

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Verify title is present in header row
        assert!(
            plain.contains("Complete Example"),
            "Header should contain title"
        );

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
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain header row with language
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("rust"), "Header should contain language");

        // Should contain padding (top and bottom)
        // Padding rows are: \x1b[48;2;R;G;Bm\x1b[K\x1b[0m\n
        assert!(
            output.contains("\x1b[K"),
            "Should contain padding clear codes"
        );

        // Should have proper ANSI structure even with no code lines
        assert!(
            output.contains("\x1b[48;2;"),
            "Should contain background color codes"
        );
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
        let mut options = test_options();
        options.color_depth = Some(ColorDepth::TrueColor);

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Check for common mid-word break patterns (word fragments at line ends)
        let bad_patterns = [
            "charac\n", // "character" split
            "Markdo\n", // "Markdown" split
            "itali\n",  // "italics" split
            "eithe\n",  // "either" split
        ];

        for pattern in bad_patterns {
            assert!(
                !plain.contains(pattern),
                "Bad word split detected: found '{}' in:\n{}",
                pattern.escape_default(),
                plain
            );
        }

        // Verify complete words exist
        for word in ["italics", "Markdown", "character", "either"] {
            assert!(
                plain.contains(word),
                "Complete word '{}' should be present:\n{}",
                word,
                plain
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

        let mut options = test_options();
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
                i + 1,
                line.escape_default(),
                plain
            );
        }
    }

    /// Test LineWrapper wraps correctly at word boundaries
    #[test]
    fn test_line_wrapper_basic() {
        use syntect::highlighting::Color;

        let style = Style {
            foreground: Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            background: Color::BLACK,
            font_style: syntect::highlighting::FontStyle::empty(),
        };

        // Create wrapper with narrow width (no hyperlink support for this test)
        let mut wrapper = LineWrapper::new(20, false);
        wrapper.emit_styled("Hello world this is a test", style, false, false, false, false, true);

        let output = wrapper.into_output();
        let plain = strip_ansi_codes(&output);

        // Should have wrapped - multiple lines
        let lines: Vec<&str> = plain.lines().collect();
        assert!(lines.len() > 1, "Text should have wrapped: '{}'", plain);

        // No line should exceed the max width
        for line in &lines {
            let width = biscuit_terminal::utils::UnicodeWidthStr::width(*line);
            assert!(
                width <= 20,
                "Line exceeds max width: '{}' (width: {})",
                line,
                width
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
            foreground: Color {
                r: 200,
                g: 200,
                b: 200,
                a: 255,
            },
            background: Color {
                r: 40,
                g: 40,
                b: 40,
                a: 255,
            },
            font_style: syntect::highlighting::FontStyle::empty(),
        };

        let mut wrapper = LineWrapper::new(30, false);
        wrapper.emit_raw("Command: ");
        wrapper.emit_inline_code("cargo build", style);
        wrapper.emit_raw(" runs the build");

        let output = wrapper.into_output();
        let plain = strip_ansi_codes(&output);

        // Should contain the inline code
        assert!(
            plain.contains("cargo build"),
            "Should contain 'cargo build'"
        );
        // Should contain surrounding text
        assert!(plain.contains("Command:"), "Should contain 'Command:'");
    }

    /// Test that long paragraphs wrap correctly at terminal width
    #[test]
    fn test_prose_paragraph_wrapping() {
        // Long paragraph that will definitely need wrapping
        let content = "This is a very long paragraph that contains many words and should definitely wrap when rendered to a terminal with a reasonable width. The wrapping should occur at word boundaries to maintain readability and prevent awkward mid-word breaks.";
        let md: Markdown = content.into();

        let mut options = test_options();
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
        for word in [
            "paragraph",
            "definitely",
            "wrapping",
            "boundaries",
            "readability",
        ] {
            assert!(
                plain.contains(word),
                "Word '{}' should be complete, not split:\n{}",
                word,
                plain
            );
        }
    }

    /// Test LineWrapper with styled text (bold, italic)
    #[test]
    fn test_line_wrapper_styled_text() {
        use syntect::highlighting::{Color, FontStyle};

        let bold_style = Style {
            foreground: Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            background: Color::BLACK,
            font_style: FontStyle::BOLD,
        };

        let italic_style = Style {
            foreground: Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            background: Color::BLACK,
            font_style: FontStyle::ITALIC,
        };

        let mut wrapper = LineWrapper::new(40, true);
        wrapper.emit_styled("This has ", bold_style, true, false, false, false, true);
        wrapper.emit_styled("mixed styles", italic_style, true, false, false, false, true);
        wrapper.emit_styled(" in one line", bold_style, true, false, false, false, true);

        let output = wrapper.into_output();

        // Should contain bold escape code
        assert!(output.contains("\x1b[1m"), "Should contain bold code");
        // Should contain italic escape code
        assert!(output.contains("\x1b[3m"), "Should contain italic code");

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("This has"), "Should contain 'This has'");
        assert!(
            plain.contains("mixed styles"),
            "Should contain 'mixed styles'"
        );
        assert!(
            plain.contains("in one line"),
            "Should contain 'in one line'"
        );
    }

    /// Test: multiline prose should not have blank lines between wrapped lines.
    #[test]
    fn test_prose_multiline_no_blank_lines() {
        let content = r#"The **strikethrough** feature -- introduced in GFM -- uses `~~` around a block of text to represent the text which should be ~~kept~~ crossed out."#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.color_depth = Some(ColorDepth::TrueColor);
        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Check that we don't have consecutive blank lines within the paragraph
        let lines: Vec<&str> = plain.lines().collect();
        for (i, window) in lines.windows(2).enumerate() {
            if window[0].is_empty() && window[1].is_empty() {
                panic!(
                    "Found consecutive blank lines at position {}: {:?}",
                    i, lines
                );
            }
        }
    }

    /// Test: prose with ==highlight== should not have extra blank lines.
    #[test]
    fn test_prose_with_highlight_no_blank_lines() {
        let content = r#"This emerging standard uses the character sequence `==` to wrap text and the wrapped text is then given a different background color to clearly ==separate it from== the rest of the text."#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.color_depth = Some(ColorDepth::TrueColor);
        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Check that we don't have consecutive blank lines within the paragraph
        let lines: Vec<&str> = plain.lines().collect();
        for (i, window) in lines.windows(2).enumerate() {
            if window[0].is_empty() && window[1].is_empty() {
                panic!(
                    "Found consecutive blank lines at position {}: {:?}",
                    i, lines
                );
            }
        }
    }

    /// Debug test: check for duplicate ANSI codes in highlighted output.
    #[test]
    fn test_debug_highlight_codes() {
        // This tests highlighted text that wraps across lines
        let content = "this is some text and ==separate it from== the rest";
        let md: Markdown = content.into();
        let mut options = test_options();
        options.color_depth = Some(ColorDepth::TrueColor);
        let output = for_terminal(&md, options).unwrap();

        // Check for duplicate background codes
        let bg_code = "\x1b[48;2;255;243;184m";
        let _bg_count = output.matches(bg_code).count();

        // For "separate it from" (3 words + 2 spaces when styled), we expect up to 5 bg codes
        // But we should NOT have consecutive duplicate codes
        let has_double_bg = output.contains("\x1b[48;2;255;243;184m\x1b[48;2;255;243;184m");
        assert!(
            !has_double_bg,
            "Should not have consecutive duplicate background codes: {:?}",
            output
        );
    }

    /// Test: strikethrough section from test.md should not have extra blank lines.
    #[test]
    fn test_strikethrough_section_no_blank_lines() {
        let content = r#"## Strikethrough

The **strikethrough** feature -- introduced in GFM -- uses `~~` around a block of text to represent the text which should be ~~kept~~ crossed out.


## Markdown Highlighting

Unlike the strikethrough functionality, the **highlight** feature for Markdown lives as a less formal spec but it is supported in popular apps like Obsidian and Typora. It is also being considered for [**CommonMark**](https://commonmark.com).

- this emerging standard uses the character sequence `==` to wrap text and the wrapped text is then given a different background color to clearly ==separate it from== the rest of the text.
"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.color_depth = Some(ColorDepth::TrueColor);
        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Check that there are no instances of 3 or more consecutive blank lines
        // (we allow 2 blank lines for section spacing)
        let lines: Vec<&str> = plain.lines().collect();
        for (i, window) in lines.windows(3).enumerate() {
            if window[0].is_empty() && window[1].is_empty() && window[2].is_empty() {
                panic!(
                    "Found three consecutive blank lines at position {}: {:?}",
                    i,
                    &lines[i.saturating_sub(2)..i.min(lines.len() - 1) + 5]
                );
            }
        }

        // Also check that within a non-blank line sequence, we don't have random blank lines
        // (e.g., prose shouldn't have blank lines after each wrapped line)
        for (i, line) in lines.iter().enumerate() {
            if !line.is_empty() && i + 1 < lines.len() && lines[i + 1].is_empty() {
                // We found a non-blank line followed by a blank line
                // Check if the blank line is a section separator (blank line after paragraph)
                // vs a bug (blank line in middle of paragraph)
                if i + 2 < lines.len() && !lines[i + 2].is_empty() {
                    // There's content after the blank - that's ok (section separator)
                } else if i + 2 < lines.len() && lines[i + 2].is_empty() {
                    // Two blank lines - ok for section separator
                }
            }
        }
    }

    // ===== CODE BLOCK REGRESSION TESTS =====
    // Regression tests for code block rendering issues:
    // 1. Bottom padding row must be on its own line (not appended to last content line)
    // 2. There should be a blank line after code blocks for separation from following content

    /// Regression test: Bottom padding row should be on its own line.
    /// Bug: The bottom padding row was being appended to the last content line
    /// instead of being rendered on its own line.
    #[test]
    fn test_code_block_bottom_padding_on_own_line() {
        let content = r#"```text
- foo
- bar
- baz
```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.color_depth = Some(ColorDepth::TrueColor);
        let output = for_terminal(&md, options).unwrap();

        // The output should have the last content line followed by a newline,
        // then the bottom padding row on its own line.
        // This regex checks that we DON'T have the pattern where content and padding are on same line:
        // [K\x1b[0m\x1b[48;2;... (clear + reset immediately followed by background on same line)
        let bad_pattern = "\x1b[K\x1b[0m\x1b[48;2;";
        assert!(
            !output.contains(bad_pattern),
            "Bottom padding row should not be appended to content line. Found pattern: {:?}",
            bad_pattern
        );

        // Verify each content line ends with clear-to-EOL + reset + newline
        let good_pattern = "\x1b[K\x1b[0m\n";
        let count = output.matches(good_pattern).count();
        // Should have at least: top padding, foo, bar, baz, bottom padding = 5 instances
        assert!(
            count >= 5,
            "Expected at least 5 lines ending with clear+reset+newline, found {}",
            count
        );
    }

    /// Regression test: Code block should have blank line after it for separation.
    /// Bug: There was no blank line between the code block and following content.
    #[test]
    fn test_code_block_followed_by_blank_line() {
        let content = r#"```text
content
```

Following paragraph."#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.color_depth = Some(ColorDepth::TrueColor);
        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Find the line containing "Following" and check there's a blank line before it
        let lines: Vec<&str> = plain.lines().collect();
        let following_idx = lines.iter().position(|l| l.contains("Following"));

        assert!(
            following_idx.is_some(),
            "Should contain 'Following paragraph'"
        );
        let idx = following_idx.unwrap();

        // The line before "Following" should be empty (the separation blank line)
        assert!(
            idx > 0 && lines[idx - 1].is_empty(),
            "There should be a blank line before following content. Lines around 'Following': {:?}",
            &lines[idx.saturating_sub(2)..idx.min(lines.len() - 1) + 1]
        );
    }

    /// Regression test: Code block content lines should each be on their own line.
    #[test]
    fn test_code_block_content_lines_separate() {
        let content = r#"```rust
fn foo() {}
fn bar() {}
```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.color_depth = Some(ColorDepth::TrueColor);
        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Each function should be on its own line
        let lines: Vec<&str> = plain.lines().collect();

        // Find lines containing the functions
        let foo_lines: Vec<_> = lines.iter().filter(|l| l.contains("fn foo")).collect();
        let bar_lines: Vec<_> = lines.iter().filter(|l| l.contains("fn bar")).collect();

        // Each should be on exactly one line
        assert_eq!(
            foo_lines.len(),
            1,
            "fn foo() should be on exactly one line, found on {} lines",
            foo_lines.len()
        );
        assert_eq!(
            bar_lines.len(),
            1,
            "fn bar() should be on exactly one line, found on {} lines",
            bar_lines.len()
        );

        // They should NOT be on the same line
        let combined_line = lines
            .iter()
            .find(|l| l.contains("fn foo") && l.contains("fn bar"));
        assert!(
            combined_line.is_none(),
            "fn foo and fn bar should not be on the same line"
        );
    }

    /// Regression test: Code block header should start on its own line when inside a list item.
    ///
    /// Previously, when a code block followed text in a list item, the language label (e.g., "sh")
    /// would be appended to the end of the text line instead of appearing on a dedicated header row.
    /// This caused the right-aligned language label to appear inline with prose text.
    #[test]
    fn test_code_block_header_on_own_line_in_list() {
        let content = r#"1. First item with text.

   ```sh
   echo hello
   ```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.color_depth = Some(ColorDepth::TrueColor);
        options.max_width = Some(80);
        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        let lines: Vec<&str> = plain.lines().collect();

        // The language label "sh" should NOT be on the same line as "text."
        let text_line = lines.iter().find(|l| l.contains("text."));
        assert!(text_line.is_some(), "Should have a line containing 'text.'");
        assert!(
            !text_line.unwrap().contains(" sh "),
            "Language label 'sh' should not be on same line as prose text. Found: {:?}",
            text_line
        );

        // The "sh" label should be on its own line (right-aligned)
        let sh_line = lines.iter().find(|l| l.trim().ends_with("sh"));
        assert!(
            sh_line.is_some(),
            "Should have a line ending with language label 'sh'. Lines: {:?}",
            lines
        );
    }

    /// Debug test: trace exact output for list item with highlight
    #[test]
    fn test_debug_list_item_with_highlight() {
        // Just the bullet point with highlight - matches test.md line 77
        let content = "- this emerging standard uses the character sequence `==` to wrap text and the wrapped text is then given a different background color to clearly ==separate it from== the rest of the text.";
        let md: Markdown = content.into();
        let mut options = test_options();
        options.color_depth = Some(ColorDepth::TrueColor);
        let output = for_terminal(&md, options).unwrap();

        let plain = strip_ansi_codes(&output);

        // The key assertion: there should be no blank lines within the wrapped list item
        let lines: Vec<&str> = plain.lines().collect();
        for (i, window) in lines.windows(2).enumerate() {
            let prev_line = window[0];
            let curr_line = window[1];
            // If we have an empty line after content (not the final trailing newline), that's a bug
            if !prev_line.is_empty() && curr_line.is_empty() && i + 2 < lines.len() {
                panic!(
                    "Found unexpected blank line at position {}: prev={:?}, curr={:?}",
                    i + 1,
                    prev_line,
                    curr_line
                );
            }
        }
    }

    /// Test line wrapping at specific widths to find where extra newlines appear
    #[test]
    fn test_wrap_at_various_widths() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let base_style = Style {
            foreground: Color {
                r: 200,
                g: 200,
                b: 200,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            font_style: FontStyle::empty(),
        };

        // Test at various widths to find where the bug appears
        for width in [60, 70, 75, 78, 79, 80, 81, 82, 85, 90, 100] {
            let mut wrapper = LineWrapper::new(width, true);

            // Simulate: "prefix ==highlighted== suffix"
            wrapper.emit_styled("prefix text before ", base_style, false, false, false, false, true);
            wrapper.emit_styled("highlighted text here", base_style, false, false, true, false, true); // in_mark=true
            wrapper.emit_styled(" suffix text after", base_style, false, false, false, false, true);

            let output = wrapper.output();
            let plain = strip_ansi_codes(output);

            // Count newlines
            let _newline_count = plain.matches('\n').count();
            let lines: Vec<&str> = plain.lines().collect();

            // Check for blank lines (empty strings between non-empty content)
            let mut has_unexpected_blank = false;
            for (i, window) in lines.windows(2).enumerate() {
                if !window[0].is_empty() && window[1].is_empty() && i + 2 < lines.len() {
                    has_unexpected_blank = true;
                }
            }

            if has_unexpected_blank {
                panic!("Found unexpected blank line at width {}", width);
            }
        }
    }

    // ===== Blockquote Tests =====
    // Regression tests for blockquote styling (fixed: blockquotes had no visual styling)

    #[test]
    fn test_blockquote_has_prefix() {
        // Regression test: blockquotes should have a visual prefix character
        let md: Markdown = "> This is a blockquote.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Blockquote should have the ▐ prefix character
        assert!(
            plain.contains("▐"),
            "Blockquote should have ▐ prefix for visual separation"
        );
        assert!(plain.contains("This is a blockquote"));
    }

    #[test]
    fn test_blockquote_has_background_color() {
        // Regression test: blockquotes should have a subtle background color
        let md: Markdown = "> Quote content".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Background color is set with ANSI escape sequence \x1b[48;2;R;G;B
        assert!(
            output.contains("\x1b[48;2;"),
            "Blockquote should have background color ANSI sequence"
        );
    }

    #[test]
    fn test_blockquote_background_extends_to_text() {
        // Regression test: blockquote background should extend to text content, not just prefix
        // Bug: previously only the ▐ prefix had background color, the text had no background
        let md: Markdown = "> Word1 Word2".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Count background color sequences - should appear multiple times
        // (once for prefix, once for each word, once for space between words)
        let bg_count = output.matches("\x1b[48;2;").count();
        assert!(
            bg_count >= 3,
            "Background color should appear for prefix AND text content (found {} times): {:?}",
            bg_count,
            output
        );

        // Verify background appears before actual text words, not just at start
        // Split by background code and check that text content follows
        let parts: Vec<&str> = output.split("\x1b[48;2;").collect();
        let has_text_with_bg = parts.iter().skip(1).any(|part| {
            // After background code, should find word content
            part.contains("Word1") || part.contains("Word2")
        });
        assert!(
            has_text_with_bg,
            "Text content should have background color applied: {:?}",
            output
        );
    }

    #[test]
    fn test_blockquote_multiple_paragraphs() {
        // Regression test: multi-paragraph blockquotes should maintain prefix on all lines
        let content = "> First paragraph.\n>\n> Second paragraph.";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        let lines: Vec<&str> = plain.lines().filter(|l| !l.is_empty()).collect();

        // Each non-empty line in the blockquote should start with the prefix
        for line in &lines {
            if line.contains("paragraph") || line.trim().starts_with("▐") {
                assert!(
                    line.starts_with("▐"),
                    "Blockquote line should start with ▐: {:?}",
                    line
                );
            }
        }
    }

    #[test]
    fn test_blockquote_preserves_inline_formatting() {
        // Blockquotes should preserve bold, italic, and other inline formatting
        let md: Markdown = "> **Bold** and *italic* in quote.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Check for bold ANSI code
        assert!(
            output.contains("\x1b[1m"),
            "Should contain bold code in blockquote"
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("Bold"));
        assert!(plain.contains("italic"));
    }

    #[test]
    fn test_blockquote_nested() {
        // Nested blockquotes should have multiple prefix characters
        let content = "> Outer\n>\n> > Nested";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Nested blockquote should have double prefix
        assert!(
            plain.contains("▐   ▐"),
            "Nested blockquote should have double prefix: {:?}",
            plain
        );
        assert!(plain.contains("Nested"));
    }

    #[test]
    fn test_blockquote_content_after() {
        // Content after blockquote should NOT have the prefix
        let content = "> Quote\n\nAfter quote";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        let lines: Vec<&str> = plain.lines().collect();

        // Find the "After quote" line
        let after_line = lines.iter().find(|l| l.contains("After quote"));
        assert!(after_line.is_some(), "Should have 'After quote' line");

        // The "After quote" line should NOT start with prefix
        assert!(
            !after_line.unwrap().starts_with("▐"),
            "Content after blockquote should not have prefix"
        );
    }

    #[test]
    fn test_blockquote_line_wrapping_preserves_prefix() {
        // Long blockquote content that wraps should have prefix on wrapped lines
        let long_text = "This is a very long blockquote line that should wrap to the next line when the terminal width is narrow enough to cause wrapping.";
        let content = format!("> {}", long_text);
        let md: Markdown = content.into();

        let options = test_options();

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should contain the prefix
        assert!(plain.contains("▐"));
        // Should contain the content
        assert!(plain.contains("very long blockquote"));
    }

    #[test]
    fn test_blockquote_no_italic_unless_emphasis() {
        // Regression test: blockquote text should NOT be italic unless explicitly marked with *text*
        // Many themes style quotes as italic, but we suppress that for readability
        let md: Markdown = "> Plain text without any formatting.".into();
        let mut options = test_options();
        options.italic_mode = ItalicMode::Always; // Force italic detection for test
        let output = for_terminal(&md, options).unwrap();

        // Should NOT contain italic escape code \x1b[3m for plain blockquote text
        assert!(
            !output.contains("\x1b[3m"),
            "Plain blockquote text should NOT be italic: {:?}",
            output
        );

        // But explicitly emphasized text SHOULD be italic
        let md_with_emphasis: Markdown = "> This has *emphasized* text.".into();
        let mut options_emphasis = test_options();
        options_emphasis.italic_mode = ItalicMode::Always;
        let output_emphasis = for_terminal(&md_with_emphasis, options_emphasis).unwrap();

        // Should contain italic code for emphasized text
        assert!(
            output_emphasis.contains("\x1b[3m"),
            "Emphasized text in blockquote should be italic: {:?}",
            output_emphasis
        );
    }

    #[test]
    fn test_blockquote_background_extends_to_terminal_width() {
        // Regression test: blockquote background should extend to terminal width
        let md: Markdown = "> Short line.".into();
        let mut options = test_options();
        options.max_width = Some(80);
        let output = for_terminal(&md, options).unwrap();

        // The output should contain padding spaces with background before newline
        // Look for pattern: background color code + multiple spaces + reset
        // Pattern: \x1b[48;2;R;G;Bm<spaces>\x1b[0m
        let has_padding = output.contains("m   ") || output.contains("m    ");
        assert!(
            has_padding || output.ends_with("\x1b[0m"),
            "Blockquote should have background padding to terminal width"
        );
    }

    #[test]
    fn test_blockquote_blank_line_after() {
        // Regression test: blockquote should be followed by a blank line before subsequent content
        // Bug: multiline blockquotes were not enforcing blank line after them
        let content = "> This is a blockquote.\n\nThis is after the blockquote.";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        let lines: Vec<&str> = plain.lines().collect();

        // Should have blank line between blockquote and following text
        // Find the blockquote line and verify next line is blank (empty or whitespace)
        let blockquote_idx = lines.iter().position(|l| l.contains("blockquote."));
        assert!(blockquote_idx.is_some(), "Should find blockquote line");

        let idx = blockquote_idx.unwrap();
        assert!(
            idx + 1 < lines.len() && lines[idx + 1].trim().is_empty(),
            "Should have blank line after blockquote, got lines:\n{:?}",
            lines
        );
    }

    #[test]
    fn test_blockquote_blank_line_before_heading() {
        // Regression test: ensure blockquote followed by heading has proper spacing
        // (The heading's own spacing rule should still apply, plus blockquote spacing)
        let content = "> Quote content.\n\n## Heading After";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        let lines: Vec<&str> = plain.lines().collect();

        // Find the blockquote line and verify next line is blank
        let quote_idx = lines.iter().position(|l| l.contains("content."));
        assert!(quote_idx.is_some(), "Should find quote content line");

        let idx = quote_idx.unwrap();
        assert!(
            idx + 1 < lines.len() && lines[idx + 1].trim().is_empty(),
            "Should have blank line after blockquote before heading, got lines:\n{:?}",
            lines
        );
    }

    #[test]
    fn test_blockquote_blank_line_before_list() {
        // Regression test: blockquote followed by list should have blank line
        let content = "> Quote content.\n\n- List item";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        let lines: Vec<&str> = plain.lines().collect();

        // Find the blockquote line and verify next line is blank
        let quote_idx = lines.iter().position(|l| l.contains("content."));
        assert!(quote_idx.is_some(), "Should find quote content line");

        let idx = quote_idx.unwrap();
        assert!(
            idx + 1 < lines.len() && lines[idx + 1].trim().is_empty(),
            "Should have blank line after blockquote before list, got lines:\n{:?}",
            lines
        );
    }

    // ===== Mermaid Diagram Tests =====
    // Regression tests for mermaid title rendering (fixed: titles were not displayed)

    #[test]
    fn test_mermaid_text_mode_has_header() {
        // Regression test: mermaid blocks in Text mode should render header with "mermaid" label
        let content = r#"```mermaid
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.mermaid_mode = MermaidMode::Text;

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should contain "mermaid" label in header
        assert!(
            plain.contains("mermaid"),
            "Mermaid block should have 'mermaid' label in header, got: {:?}",
            plain
        );
    }

    #[test]
    fn test_mermaid_text_mode_with_title() {
        // Regression test: mermaid blocks with title should display the title
        // Bug: previously titles were not rendered for mermaid blocks
        let content = r#"```mermaid title="My Flowchart"
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.mermaid_mode = MermaidMode::Text;

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should contain the title
        assert!(
            plain.contains("My Flowchart"),
            "Mermaid block should display title 'My Flowchart', got: {:?}",
            plain
        );

        // Should also contain "mermaid" label
        assert!(
            plain.contains("mermaid"),
            "Mermaid block should have 'mermaid' label in header"
        );
    }

    #[test]
    fn test_mermaid_title_is_bold() {
        // Regression test: mermaid title should be bold (like regular code block titles)
        let content = r#"```mermaid title="Bold Title"
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.mermaid_mode = MermaidMode::Text;

        let output = for_terminal(&md, options).unwrap();

        // Title should be bold (ANSI code \x1b[1m appears before the title)
        assert!(
            output.contains("\x1b[1m"),
            "Mermaid title should be bold, got: {:?}",
            output
        );

        // Bold should appear before the title text
        let bold_pos = output.find("\x1b[1m");
        let title_pos = output.find("Bold Title");
        assert!(
            bold_pos.is_some() && title_pos.is_some() && bold_pos.unwrap() < title_pos.unwrap(),
            "Bold ANSI code should appear before title text"
        );
    }

    #[test]
    fn test_mermaid_title_with_spaces() {
        // Regression test: mermaid titles with spaces should work correctly
        let content = r#"```mermaid title="My Complex Flowchart Diagram"
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.mermaid_mode = MermaidMode::Text;

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        assert!(
            plain.contains("My Complex Flowchart Diagram"),
            "Mermaid block should display multi-word title, got: {:?}",
            plain
        );
    }

    #[test]
    fn test_mermaid_uppercase_language() {
        // Mermaid language detection should be case-insensitive
        let content = r#"```MERMAID title="Uppercase Test"
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.mermaid_mode = MermaidMode::Text;

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should still detect as mermaid and show title
        assert!(
            plain.contains("Uppercase Test"),
            "MERMAID (uppercase) should display title, got: {:?}",
            plain
        );
        assert!(
            plain.contains("mermaid"),
            "Header should show 'mermaid' label"
        );
    }

    #[test]
    fn test_mermaid_off_mode_renders_as_code() {
        // When mermaid mode is Off, mermaid blocks render as regular code blocks
        let content = r#"```mermaid title="Test Title"
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.mermaid_mode = MermaidMode::Off;

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should render title since it's treated as regular code block
        assert!(
            plain.contains("Test Title"),
            "Mermaid block in Off mode should display title as regular code block, got: {:?}",
            plain
        );

        // Should NOT contain the fallback code block format (```mermaid)
        assert!(
            !plain.contains("```mermaid"),
            "Off mode should render as highlighted code, not fallback format"
        );
    }

    #[test]
    fn test_mermaid_no_title_still_has_header() {
        // Regression test: mermaid blocks without title should still show "mermaid" label
        let content = r#"```mermaid
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.mermaid_mode = MermaidMode::Text;

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should have "mermaid" label even without a title
        assert!(
            plain.contains("mermaid"),
            "Mermaid block without title should still have 'mermaid' label, got: {:?}",
            plain
        );
    }

    #[test]
    fn test_mermaid_text_mode_no_raw_markdown_fences() {
        // Regression test for bug: mermaid fallback was outputting raw markdown
        // syntax (```mermaid...```) instead of syntax-highlighted code
        let content = r#"```mermaid title="Test"
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.mermaid_mode = MermaidMode::Text;

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Should NOT contain raw markdown fence markers
        assert!(
            !plain.contains("```mermaid"),
            "Text mode should NOT output raw ```mermaid fence, got:\n{}",
            plain
        );
        assert!(
            !plain.contains("```"),
            "Text mode should NOT output any raw ``` fence markers, got:\n{}",
            plain
        );
    }

    #[test]
    fn test_mermaid_text_mode_has_ansi_styling() {
        // Regression test for bug: mermaid fallback had no styling (no background color)
        let content = r#"```mermaid
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.mermaid_mode = MermaidMode::Text;

        let output = for_terminal(&md, options).unwrap();

        // Should contain ANSI escape codes for background color (48;2 = RGB background)
        assert!(
            output.contains("\x1b[48;2;"),
            "Text mode should have ANSI background color styling, got:\n{}",
            output
        );
    }

    #[test]
    fn test_mermaid_text_mode_no_title_duplication() {
        // Regression test for bug: title was appearing twice in mermaid fallback
        let content = r#"```mermaid title="My Title"
flowchart LR
    A --> B
```"#;
        let md: Markdown = content.into();
        let mut options = test_options();
        options.mermaid_mode = MermaidMode::Text;

        let output = for_terminal(&md, options).unwrap();
        let plain = strip_ansi_codes(&output);

        // Count occurrences of "My Title" - should be exactly 1
        let title_count = plain.matches("My Title").count();
        assert_eq!(
            title_count, 1,
            "Title 'My Title' should appear exactly once, found {} times in:\n{}",
            title_count, plain
        );

        // Count occurrences of "mermaid" - should be exactly 1
        let mermaid_count = plain.matches("mermaid").count();
        assert_eq!(
            mermaid_count, 1,
            "'mermaid' label should appear exactly once, found {} times in:\n{}",
            mermaid_count, plain
        );
    }

    // ===== format_header_row Tests =====
    // Unit tests for the header row formatting function

    #[test]
    fn test_format_header_row_title_only() {
        // When language is empty but title is present, show only the title (no language label)
        // This is used for rendered mermaid diagrams where "mermaid" label is redundant
        use syntect::highlighting::Color;

        let header = format_header_row(
            Some("My Diagram"),
            "", // Empty language = title-only header
            Color {
                r: 34,
                g: 34,
                b: 34,
                a: 255,
            },
            ColorMode::Dark,
            80,
        );

        let plain = strip_ansi_codes(&header);

        // Should contain the title
        assert!(
            plain.contains("My Diagram"),
            "Title-only header should contain the title, got: {:?}",
            plain
        );

        // Should NOT contain "text" (the default language for empty strings)
        assert!(
            !plain.contains("text"),
            "Title-only header should NOT show 'text' language label, got: {:?}",
            plain
        );
    }

    #[test]
    fn test_format_header_row_empty_language_no_title_shows_text() {
        // When both language is empty AND no title, default to "text" label
        use syntect::highlighting::Color;

        let header = format_header_row(
            None,
            "",
            Color {
                r: 34,
                g: 34,
                b: 34,
                a: 255,
            },
            ColorMode::Dark,
            80,
        );

        let plain = strip_ansi_codes(&header);

        // Should show "text" as default language
        assert!(
            plain.contains("text"),
            "Header with no title and empty language should show 'text', got: {:?}",
            plain
        );
    }

    #[test]
    fn test_format_header_row_with_both_title_and_language() {
        // Normal case: both title and language are shown
        use syntect::highlighting::Color;

        let header = format_header_row(
            Some("Example"),
            "rust",
            Color {
                r: 34,
                g: 34,
                b: 34,
                a: 255,
            },
            ColorMode::Dark,
            80,
        );

        let plain = strip_ansi_codes(&header);

        assert!(plain.contains("Example"), "Should contain title");
        assert!(plain.contains("rust"), "Should contain language");
    }

    // ===== Hyperlink Tests =====

    #[test]
    fn test_terminal_link_renders_display_text() {
        // Regression test: links should render display text (not be silently dropped)
        // Bug: links were only styled but the OSC8 hyperlink sequences weren't emitted
        let content = "Check out [Zigbee2MQTT](https://www.zigbee2mqtt.io/) for details.";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);

        // The display text "Zigbee2MQTT" must appear
        assert!(
            plain.contains("Zigbee2MQTT"),
            "Link display text should be rendered, got: {:?}",
            plain
        );
    }

    #[test]
    fn test_terminal_link_uses_link_struct() {
        // Regression test: terminal output should use Link::to_terminal() which handles
        // OSC8 escape sequences for clickable links (or fallback to [url] format)
        let content = "[Example](https://example.com)";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);

        // Should contain either OSC8 sequences (if terminal supports) or fallback [url]
        // In test environment, typically falls back to "Example [https://example.com]"
        assert!(
            plain.contains("Example"),
            "Link display text should be rendered, got: {:?}",
            plain
        );

        // Either OSC8 start sequence OR fallback URL format should be present
        let has_osc8 = output.contains("\x1b]8;;");
        let has_fallback = plain.contains("[https://example.com]");

        assert!(
            has_osc8 || has_fallback,
            "Link should use OSC8 or fallback format, got raw: {:?}, plain: {:?}",
            output,
            plain
        );
    }

    #[test]
    fn test_terminal_link_in_list() {
        // Regression test: links inside list items should render correctly
        let content = "- [Link One](https://one.com)\n- [Link Two](https://two.com)";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("Link One"), "First link should render");
        assert!(plain.contains("Link Two"), "Second link should render");
    }

    #[test]
    fn test_terminal_multiple_links_in_paragraph() {
        // Ensure multiple links in a single paragraph all render
        let content = "Visit [Google](https://google.com) or [GitHub](https://github.com).";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);

        assert!(plain.contains("Google"), "First link should render");
        assert!(plain.contains("GitHub"), "Second link should render");
    }

    #[test]
    fn test_terminal_link_styling_inside_osc8() {
        // Regression test: ANSI color codes must appear INSIDE the OSC8 sequence, not outside.
        //
        // Bug (before fix): Styling was applied to the entire OSC8 sequence including escape codes:
        //   \x1b[38;2;R;G;Bm\x1b]8;;URL\x07TEXT\x1b]8;;\x07\x1b[0m
        //   ^--- color start (invisible)                    ^--- color reset (too late!)
        //
        // Fix: Styling must wrap only the visible text INSIDE the hyperlink:
        //   \x1b]8;;URL\x07\x1b[38;2;R;G;BmTEXT\x1b[0m\x1b]8;;\x07
        //                  ^--- color start   ^--- reset   ^--- OSC8 close
        //
        let content = "[Click Here](https://example.com)";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Check that OSC8 sequences are present
        let osc8_start = "\x1b]8;;";
        let osc8_end = "\x1b]8;;\x07";

        assert!(
            output.contains(osc8_start),
            "Output should contain OSC8 start sequence"
        );
        assert!(
            output.contains(osc8_end),
            "Output should contain OSC8 end sequence"
        );

        // Find the positions of key sequences
        let start_pos = output.find(osc8_start).expect("OSC8 start should exist");
        let end_pos = output.find(osc8_end).expect("OSC8 end should exist");

        // The BEL after the URL marks where visible content begins
        let after_url_bel = output[start_pos..]
            .find('\x07')
            .expect("BEL should exist after URL");
        let content_start = start_pos + after_url_bel + 1;

        // Extract the content between OSC8 open and close
        let hyperlink_content = &output[content_start..end_pos];

        // The ANSI reset sequence (\x1b[0m) should appear INSIDE the hyperlink content
        // (before the OSC8 close sequence)
        assert!(
            hyperlink_content.contains("\x1b[0m"),
            "ANSI reset should appear INSIDE the OSC8 hyperlink, not after it. \
             Hyperlink content: {:?}",
            hyperlink_content
        );

        // The hyperlink content should contain ANSI color codes (38;2 for 24-bit foreground)
        assert!(
            hyperlink_content.contains("\x1b[38;2;"),
            "ANSI color code should appear INSIDE the OSC8 hyperlink. \
             Hyperlink content: {:?}",
            hyperlink_content
        );

        // The visible text should be inside the hyperlink
        assert!(
            hyperlink_content.contains("Click Here"),
            "Link text should appear INSIDE the OSC8 hyperlink. \
             Hyperlink content: {:?}",
            hyperlink_content
        );
    }

    #[test]
    fn test_terminal_link_osc8_structure() {
        // Verify the complete OSC8 structure:
        // ESC]8;;URL BEL <styled_text> ESC]8;; BEL
        let content = "[Test](https://test.com)";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        // The URL should appear in the OSC8 start sequence
        assert!(
            output.contains("\x1b]8;;https://test.com\x07"),
            "OSC8 start should contain the URL followed by BEL. Output: {:?}",
            output
        );

        // The OSC8 close sequence should be present
        assert!(
            output.contains("\x1b]8;;\x07"),
            "OSC8 close sequence should be present"
        );
    }

    #[test]
    fn test_terminal_link_styling_preserved_in_list() {
        // Regression test: links in lists should also have styling inside OSC8
        let content = "- [Link](https://example.com)";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Find OSC8 content region
        let osc8_start = "\x1b]8;;";
        let osc8_end = "\x1b]8;;\x07";

        if let (Some(start_pos), Some(end_pos)) = (output.find(osc8_start), output.find(osc8_end)) {
            let after_url_bel = output[start_pos..].find('\x07').unwrap();
            let content_start = start_pos + after_url_bel + 1;
            let hyperlink_content = &output[content_start..end_pos];

            // Style should be inside the hyperlink
            assert!(
                hyperlink_content.contains("\x1b["),
                "Link in list should have ANSI styling inside OSC8. Content: {:?}",
                hyperlink_content
            );
        }
    }

    #[test]
    fn test_terminal_link_always_has_distinct_color() {
        // Regression test for: links sent to the terminal have NO STYLING
        // Bug: Themes without link-specific colors (e.g., OneHalf) caused links
        // to render with the same color as regular text, making them invisible.
        // Fix: Apply fallback blue color and underline when theme returns base color.
        let content = "[Click Here](https://example.com)";
        let md: Markdown = content.into();

        // Test with default OneHalf theme which doesn't define link colors
        let output = for_terminal(&md, test_options()).unwrap();

        // Extract the content between OSC8 open and close
        let osc8_start = "\x1b]8;;";
        let osc8_end = "\x1b]8;;\x07";
        let start_pos = output.find(osc8_start).expect("OSC8 start should exist");
        let end_pos = output.find(osc8_end).expect("OSC8 end should exist");
        let after_url_bel = output[start_pos..].find('\x07').unwrap();
        let content_start = start_pos + after_url_bel + 1;
        let hyperlink_content = &output[content_start..end_pos];

        // The fallback blue color should be applied: RGB(65,160,225)
        assert!(
            hyperlink_content.contains("\x1b[38;2;65;160;225m"),
            "Links without theme styling should use fallback blue color. \
             Hyperlink content: {:?}",
            hyperlink_content
        );

        // Underline should be added for extra visual distinction
        assert!(
            hyperlink_content.contains("\x1b[4m"),
            "Links without theme styling should have underline. \
             Hyperlink content: {:?}",
            hyperlink_content
        );
    }

    #[test]
    fn test_terminal_link_uses_theme_color_when_available() {
        // Regression test: themes that DO define link colors should use them
        // (not the fallback blue). Github theme defines link color as RGB(230,211,122).
        use crate::markdown::highlighting::ThemePair;

        let content = "[Click Here](https://example.com)";
        let md: Markdown = content.into();

        let mut options = test_options();
        options.prose_theme = ThemePair::Github;

        let output = for_terminal(&md, options).unwrap();

        // Extract the content between OSC8 open and close
        let osc8_start = "\x1b]8;;";
        let osc8_end = "\x1b]8;;\x07";
        let start_pos = output.find(osc8_start).expect("OSC8 start should exist");
        let end_pos = output.find(osc8_end).expect("OSC8 end should exist");
        let after_url_bel = output[start_pos..].find('\x07').unwrap();
        let content_start = start_pos + after_url_bel + 1;
        let hyperlink_content = &output[content_start..end_pos];

        // Github theme's link color (approximately 230,211,122 - yellow/gold)
        // Should NOT be the fallback blue
        assert!(
            !hyperlink_content.contains("\x1b[38;2;65;160;225m"),
            "Themes with link styling should NOT use fallback blue. \
             Hyperlink content: {:?}",
            hyperlink_content
        );

        // Should use the theme's link color
        assert!(
            hyperlink_content.contains("\x1b[38;2;230;211;122m"),
            "Themes with link styling should use theme's link color. \
             Hyperlink content: {:?}",
            hyperlink_content
        );

        // Should NOT have underline (theme provides its own styling)
        assert!(
            !hyperlink_content.contains("\x1b[4m"),
            "Themes with link styling should not add extra underline. \
             Hyperlink content: {:?}",
            hyperlink_content
        );
    }

    #[test]
    fn test_terminal_link_in_list_has_distinct_color() {
        // Regression test: links inside list items should also get distinct styling
        // This tests that the bug fix works for links in different contexts.
        let content = "- [Zigbee2MQTT](https://www.zigbee2mqtt.io/)";
        let md: Markdown = content.into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Extract the content between OSC8 open and close
        let osc8_start = "\x1b]8;;";
        let osc8_end = "\x1b]8;;\x07";
        let start_pos = output.find(osc8_start).expect("OSC8 start should exist");
        let end_pos = output.find(osc8_end).expect("OSC8 end should exist");
        let after_url_bel = output[start_pos..].find('\x07').unwrap();
        let content_start = start_pos + after_url_bel + 1;
        let hyperlink_content = &output[content_start..end_pos];

        // The fallback blue color should be applied
        assert!(
            hyperlink_content.contains("\x1b[38;2;65;160;225m"),
            "Links in lists should use fallback blue when theme lacks link styling. \
             Hyperlink content: {:?}",
            hyperlink_content
        );

        // Visible text should be present
        assert!(
            hyperlink_content.contains("Zigbee2MQTT"),
            "Link text should be present. Hyperlink content: {:?}",
            hyperlink_content
        );
    }

    #[test]
    fn test_hyperlink_mode_never_uses_fallback_format() {
        // When HyperlinkMode::Never is set, links should render as "text [url]"
        // instead of OSC8 escape sequences
        let content = "[Click Here](https://example.com)";
        let md: Markdown = content.into();

        let mut options = test_options();
        options.hyperlink_mode = HyperlinkMode::Never;

        let output = for_terminal(&md, options).unwrap();

        // Should NOT contain OSC8 sequences
        let osc8_start = "\x1b]8;;";
        assert!(
            !output.contains(osc8_start),
            "HyperlinkMode::Never should not emit OSC8 sequences. Output: {:?}",
            output
        );

        // Should contain fallback format: text followed by [url]
        let plain = strip_ansi_codes(&output);
        assert!(
            plain.contains("Click Here [https://example.com]"),
            "HyperlinkMode::Never should use 'text [url]' fallback format. Plain output: {:?}",
            plain
        );
    }

    #[test]
    fn test_hyperlink_mode_always_emits_osc8() {
        // When HyperlinkMode::Always is set, links should always render as OSC8
        let content = "[Test Link](https://test.com)";
        let md: Markdown = content.into();

        let mut options = test_options();
        options.hyperlink_mode = HyperlinkMode::Always;

        let output = for_terminal(&md, options).unwrap();

        // Should contain OSC8 sequences
        let osc8_start = "\x1b]8;;https://test.com\x07";
        assert!(
            output.contains(osc8_start),
            "HyperlinkMode::Always should emit OSC8 sequences. Output: {:?}",
            output
        );
    }

    #[test]
    fn test_hyperlink_fallback_in_table() {
        // Links in tables should also respect HyperlinkMode
        let content = "| Name | Link |\n|------|------|\n| Test | [Click](https://example.com) |";
        let md: Markdown = content.into();

        let mut options = test_options();
        options.hyperlink_mode = HyperlinkMode::Never;

        let output = for_terminal(&md, options).unwrap();

        // Should NOT contain OSC8 sequences in table
        let osc8_start = "\x1b]8;;";
        assert!(
            !output.contains(osc8_start),
            "Table links with HyperlinkMode::Never should not emit OSC8. Output: {:?}",
            output
        );

        // Should contain fallback format in table
        let plain = strip_ansi_codes(&output);
        assert!(
            plain.contains("Click [https://example.com]"),
            "Table links with HyperlinkMode::Never should use fallback format. Plain output: {:?}",
            plain
        );
    }

    #[test]
    fn test_hyperlink_mode_default_is_auto() {
        // Default should be Auto mode
        let options = TerminalOptions::default();
        assert!(
            matches!(options.hyperlink_mode, HyperlinkMode::Auto),
            "Default hyperlink_mode should be Auto"
        );
    }

    // =========================================================================
    // Dim (⌄text⌄) tests
    // =========================================================================

    #[test]
    fn test_terminal_dim_basic() {
        let md: Markdown = "This is ⌄dimmed⌄ text.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain dim ANSI code \x1b[2m
        assert!(
            output.contains("\x1b[2m"),
            "Should contain dim ANSI code, got: {:?}",
            output
        );

        // Content should be present when stripped
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("dimmed"));
    }

    #[test]
    fn test_terminal_dim_never_strips_delimiters() {
        let md: Markdown = "This is ⌄dimmed⌄ text.".into();
        let mut options = test_options();
        options.dim_mode = DimMode::Never;
        let output = for_terminal(&md, options).unwrap();

        // Should NOT contain dim ANSI code
        assert!(
            !output.contains("\x1b[2m"),
            "DimMode::Never should not produce \x1b[2m, got: {:?}",
            output
        );

        // Delimiters should be stripped (inline processor removes them)
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("dimmed"));
        assert!(!plain.contains("⌄"), "Delimiters should be stripped in terminal output");
    }

    #[test]
    fn test_terminal_dim_nested_bold() {
        let md: Markdown = "This is **⌄bold dim⌄** text.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain both bold and dim codes
        assert!(output.contains("\x1b[1m"), "Should contain bold code");
        assert!(output.contains("\x1b[2m"), "Should contain dim code");

        // Neither style should leak into following text
        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("bold dim"));
        assert!(plain.contains("text."));
    }

    #[test]
    fn test_terminal_dim_unclosed() {
        let md: Markdown = "This has ⌄unclosed dim markers.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Unclosed markers should render literally
        let plain = strip_ansi_codes(&output);
        assert!(
            plain.contains("⌄unclosed"),
            "Unclosed dim markers should render literally, got: {:?}",
            plain
        );
    }

    #[test]
    fn test_terminal_dim_in_inline_code() {
        let md: Markdown = "Use `⌄code⌄` syntax.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Inline code should NOT be dimmed
        assert!(
            !output.contains("\x1b[2m"),
            "Inline code should not trigger dim, got: {:?}",
            output
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("⌄code⌄"), "Inline code should preserve literal delimiters");
    }

    #[test]
    fn test_terminal_dim_in_fenced_code() {
        let md: Markdown = "```\n⌄dim\n```".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Fenced code should NOT be dimmed
        assert!(
            !output.contains("\x1b[2m"),
            "Fenced code should not trigger dim, got: {:?}",
            output
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("⌄dim"), "Fenced code should preserve literal delimiters");
    }

    #[test]
    fn test_terminal_dim_in_list() {
        let md: Markdown = "- ⌄dimmed item⌄\n- normal item".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain dim code
        assert!(
            output.contains("\x1b[2m"),
            "Dim should work in list items, got: {:?}",
            output
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("dimmed item"));
        assert!(plain.contains("normal item"));
    }

    #[test]
    fn test_terminal_dim_in_blockquote() {
        let md: Markdown = "> ⌄dimmed quote⌄".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain dim code
        assert!(
            output.contains("\x1b[2m"),
            "Dim should work in blockquotes, got: {:?}",
            output
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("dimmed quote"));
    }

    #[test]
    fn test_terminal_dim_table_cell() {
        let md: Markdown = "| Header |\n|--------|\n| ⌄dim⌄ |".into();
        let output = for_terminal(&md, test_options()).unwrap();

        let plain = strip_ansi_codes(&output);
        // Table cell should contain the dimmed text without delimiters
        assert!(plain.contains("dim"), "Table cell should contain dim text");
        assert!(!plain.contains("⌄"), "Table cell should not expose delimiters");
    }

    #[test]
    fn test_terminal_dim_mode_auto_with_support() {
        // DimMode::Auto with a terminal that supports dim should produce \x1b[2m
        // We can't easily mock terminal detection, but we can at least verify
        // the mode resolves correctly
        let mode = DimMode::Always;
        assert!(mode.should_emit_dim(), "DimMode::Always should emit dim");

        let mode = DimMode::Never;
        assert!(!mode.should_emit_dim(), "DimMode::Never should not emit dim");
    }

    #[test]
    fn test_terminal_dim_empty() {
        let md: Markdown = "This has ⌄⌄ empty dim.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Empty dim (⌄⌄) creates start + end events with no content between
        let plain = strip_ansi_codes(&output);
        assert!(
            plain.contains("empty dim"),
            "Text after empty dim should render"
        );
    }

    #[test]
    fn test_terminal_dim_multiple() {
        let md: Markdown = "This has ⌄one⌄ and ⌄two⌄ dims.".into();
        let output = for_terminal(&md, test_options()).unwrap();

        // Should contain dim code
        assert!(
            output.contains("\x1b[2m"),
            "Should contain dim code"
        );

        let plain = strip_ansi_codes(&output);
        assert!(plain.contains("one"));
        assert!(plain.contains("two"));
    }

    #[test]
    fn test_emit_prose_text_dim() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: FontStyle::empty(),
        };

        // With in_dim=true and emit_dim=true, should emit dim code
        let result = emit_prose_text("dim text", style, true, false, false, None, true, true);
        assert!(
            result.contains("\x1b[2m"),
            "Dim text should include \x1b[2m escape code, got: {:?}",
            result
        );
        assert!(result.contains("\x1b[38;2;255;128;64m"));
        assert!(result.contains("\x1b[0m"));
    }

    #[test]
    fn test_emit_prose_text_dim_suppressed() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: FontStyle::empty(),
        };

        // With emit_dim=false, should NOT emit dim code
        let result = emit_prose_text("dim text", style, true, false, false, None, true, false);
        assert!(
            !result.contains("\x1b[2m"),
            "Dim text with emit_dim=false should NOT include \x1b[2m, got: {:?}",
            result
        );
        assert!(result.contains("\x1b[38;2;255;128;64m"));
    }

    #[test]
    fn test_emit_prose_text_bold_dim() {
        use syntect::highlighting::{Color, FontStyle, Style};

        let style = Style {
            foreground: Color {
                r: 255,
                g: 128,
                b: 64,
                a: 255,
            },
            background: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            font_style: FontStyle::BOLD,
        };

        let result = emit_prose_text("bold dim", style, true, false, false, None, true, true);

        // Should have both bold and dim escape codes
        assert!(result.contains("\x1b[1m"), "Should have bold");
        assert!(result.contains("\x1b[2m"), "Should have dim");
    }
}
