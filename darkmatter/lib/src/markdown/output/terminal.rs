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
//! use darkmatter::markdown::output::TerminalOptions;
//!
//! let content = "# Hello World\n\n\
//!                ```rust\n\
//!                fn main() {\n    \
//!                    println!(\"Hello!\");\n\
//!                }\n\
//!                ```\n";
//!
//! let md: Markdown = content.into();
//! let output = md.as_terminal(TerminalOptions::default()).unwrap();
//! // Output contains ANSI escape codes for terminal display
//! ```

use crate::markdown::highlighting::{ColorMode, ThemePair};
#[cfg(test)]
use crate::markdown::output::code_block;
use biscuit_terminal::components::image_options::TerminalImageOptions;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::terminal_image::{ImageWidth, TerminalImage, parse_width_spec};
use biscuit_terminal::discovery::detection::ColorDepth as TerminalColorDepth;
use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::terminal::Terminal;
use std::path::{Path, PathBuf};
use syntect::highlighting::Color;

// Re-export shared code-block helpers so darkmatter tests keep working.
#[cfg(test)]
pub(crate) use code_block::compute_highlight_bg;
#[cfg(test)]
pub(crate) use code_block::find_syntax;
#[cfg(test)]
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

impl From<TerminalColorDepth> for ColorDepth {
    /// Project a biscuit-terminal [`ColorDepth`](TerminalColorDepth) onto the
    /// darkmatter renderer's coarser palette using the same thresholds as
    /// [`ColorDepth::auto_detect`].
    ///
    /// Darkmatter has no 8-color variant, so biscuit's `Minimal` (8 colors)
    /// falls below the 16-color floor and maps to [`ColorDepth::None`] — the
    /// same projection [`auto_detect`](Self::auto_detect) makes via
    /// [`crate::terminal::color_depth`]. Preserving that mapping is what lets
    /// a [`crate::layout::DarkmatterPage`] built from a real
    /// [`biscuit_terminal::terminal::Terminal`] render byte-for-byte
    /// identically to `for_terminal(&md, TerminalOptions::default())`.
    fn from(depth: TerminalColorDepth) -> Self {
        match depth {
            TerminalColorDepth::TrueColor => ColorDepth::TrueColor,
            TerminalColorDepth::Enhanced => ColorDepth::Colors256,
            TerminalColorDepth::Basic => ColorDepth::Colors16,
            TerminalColorDepth::Minimal | TerminalColorDepth::None => ColorDepth::None,
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

        // Render via TerminalRenderable fallback (protocol-aware string output)
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
    /// Resolved HR defaults from `style.hr` frontmatter (or `DarkmatterPage`
    /// builder calls). When present, terminal rendering uses these as the
    /// default horizontal-rule style instead of reading the deprecated top-level
    /// `hr:` frontmatter block.
    pub hr_defaults: Option<crate::markdown::inline::HorizontalRuleAttrs>,
}

static DETECTED_COLOR_MODE: std::sync::OnceLock<ColorMode> = std::sync::OnceLock::new();

impl Default for TerminalOptions {
    fn default() -> Self {
        use crate::markdown::highlighting::{
            detect_code_theme, detect_color_mode, detect_prose_theme,
        };

        let prose_theme = detect_prose_theme();
        let code_theme = detect_code_theme(prose_theme);
        let color_mode = *DETECTED_COLOR_MODE.get_or_init(detect_color_mode);

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
            hr_defaults: None,
        }
    }
}

/// Converts pulldown-cmark alignment to biscuit-terminal alignment.
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
        ColorMode::Dark | ColorMode::Unknown => (255, 255, 255), // WHITE
        ColorMode::Light => (0, 0, 0),                           // BLACK
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
            "\x1b[0m\x1b[1m\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m {} \x1b[0m",
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
            "\x1b[0m\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m {} \x1b[0m",
            bg_color.r, bg_color.g, bg_color.b, text_color.0, text_color.1, text_color.2, lang
        ));
    }

    output
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
        ColorMode::Dark | ColorMode::Unknown => Color {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::highlighting::CodeHighlighter;
    use crate::testing::strip_ansi_codes;

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
            hr_defaults: None,
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
    fn default_code_theme_uses_page_theme_pair() {
        let options = TerminalOptions::default();
        assert_eq!(
            options.code_theme, options.prose_theme,
            "code blocks should use the page ThemePair and let the renderer invert the color mode",
        );
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
        let result = highlight_code(
            code,
            "rust",
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
    fn test_code_block_padding() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let options = test_options();
        let meta = crate::markdown::dsl::CodeBlockMeta::default();

        let code = "fn main() {}";
        let result = highlight_code(
            code,
            "rust",
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

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
        let result = highlight_code(
            code,
            "rust",
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

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
        let result = highlight_code(
            code,
            "rust",
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

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
        let result = highlight_code(
            code,
            "rust",
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

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
        let result = highlight_code(
            code,
            "rust",
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

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
        let result = highlight_code(
            code,
            "rust",
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

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
        let result = highlight_code(
            code,
            "rust",
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

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
        let result = highlight_code(
            code,
            "rust",
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

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
    fn test_line_numbers_have_background_color() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let mut options = test_options();
        options.include_line_numbers = true;
        let meta = crate::markdown::dsl::CodeBlockMeta::default();

        let code = "let x = 1;";
        let result = highlight_code(
            code,
            "rust",
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

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
        let result = highlight_code(
            code,
            "rust",
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

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
        let result = highlight_code(
            code,
            "rust",
            &highlighter,
            &options,
            &meta,
            ColorMode::Dark,
            None,
            None,
        );

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

    #[test]
    fn test_image_renderer_new_with_none_base_path() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);

        // Should use current directory as base
        let expected_base = std::env::current_dir().unwrap_or_default();
        assert_eq!(renderer.base_path(), expected_base);

        // Terminal width should be either detected or default 80
        assert!(renderer.terminal_width() >= 10);
    }

    #[test]
    fn test_image_renderer_new_with_base_path() {
        let base = std::env::temp_dir();
        let renderer = ImageRenderer::new(Some(&base), TerminalImageMode::Auto);

        assert_eq!(renderer.base_path(), base);
    }

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

    #[test]
    fn test_image_renderer_with_nonexistent_base() {
        let nonexistent = std::path::PathBuf::from("/this/path/does/not/exist/for/sure");
        let renderer = ImageRenderer::new(Some(&nonexistent), TerminalImageMode::Auto);

        // Should use the path even if it doesn't exist
        assert_eq!(renderer.base_path(), nonexistent);

        // base_path_canonical should be None
        // (we can't directly test this since it's private, but the struct should be valid)
    }

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

    #[test]
    fn test_image_renderer_force_mode_attempts_protocol_render() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Force);
        assert!(renderer.should_attempt_protocol_render());

        let forced_terminal = renderer.render_terminal();
        assert!(forced_terminal.is_tty);
        assert!(!matches!(forced_terminal.image_support, ImageSupport::None));
    }

    #[test]
    fn test_image_renderer_never_mode_disables_protocol_render() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Never);
        assert!(!renderer.should_attempt_protocol_render());
    }

    #[test]
    fn test_parse_alt_and_width_percentage() {
        let (alt, width) = parse_alt_and_width("my image|50%");
        assert_eq!(alt, "my image");
        assert!(matches!(width, Some(ImageWidth::Percent(p)) if (p - 0.5).abs() < 0.001));
    }

    #[test]
    fn test_parse_alt_and_width_characters() {
        let (alt, width) = parse_alt_and_width("photo|80");
        assert_eq!(alt, "photo");
        assert!(matches!(width, Some(ImageWidth::Characters(80))));
    }

    #[test]
    fn test_parse_alt_and_width_ch_suffix() {
        let (alt, width) = parse_alt_and_width("diagram|40ch");
        assert_eq!(alt, "diagram");
        assert!(matches!(width, Some(ImageWidth::Characters(40))));
    }

    #[test]
    fn test_parse_alt_and_width_fill() {
        // 'fill' doesn't start with a digit, so the entire string is treated as alt text
        let (alt, width) = parse_alt_and_width("banner|fill");
        assert_eq!(alt, "banner|fill");
        assert!(width.is_none());
    }

    #[test]
    fn test_parse_alt_and_width_no_pipe() {
        let (alt, width) = parse_alt_and_width("simple alt text");
        assert_eq!(alt, "simple alt text");
        assert!(width.is_none());
    }

    #[test]
    fn test_parse_alt_and_width_pipe_not_digit() {
        let (alt, width) = parse_alt_and_width("chart | analysis");
        assert_eq!(alt, "chart | analysis");
        assert!(width.is_none());
    }

    #[test]
    fn test_parse_alt_and_width_pipe_letter() {
        let (alt, width) = parse_alt_and_width("A|B comparison");
        assert_eq!(alt, "A|B comparison");
        assert!(width.is_none());
    }

    #[test]
    fn test_parse_alt_and_width_invalid_spec() {
        // 0 is invalid, so fallback to full alt text
        let (alt, width) = parse_alt_and_width("image|0");
        assert_eq!(alt, "image|0");
        assert!(width.is_none());
    }

    #[test]
    fn test_parse_alt_and_width_whitespace_before_number() {
        let (alt, width) = parse_alt_and_width("image| 50%");
        assert_eq!(alt, "image");
        assert!(matches!(width, Some(ImageWidth::Percent(p)) if (p - 0.5).abs() < 0.001));
    }

    #[test]
    fn test_parse_alt_and_width_trims_alt() {
        let (alt, width) = parse_alt_and_width("my image |25%");
        assert_eq!(alt, "my image");
        assert!(matches!(width, Some(ImageWidth::Percent(p)) if (p - 0.25).abs() < 0.001));
    }

    #[test]
    fn test_parse_alt_and_width_empty_alt() {
        let (alt, width) = parse_alt_and_width("|50%");
        assert_eq!(alt, "");
        assert!(matches!(width, Some(ImageWidth::Percent(p)) if (p - 0.5).abs() < 0.001));
    }

    #[test]
    fn test_render_image_rejects_http_url() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);
        let result = renderer.render_image("http://example.com/image.png", "Alt", None);

        assert!(result.contains("▉ IMAGE[Alt]"));
    }

    #[test]
    fn test_render_image_rejects_https_url() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);
        let result = renderer.render_image("https://example.com/image.png", "Alt", None);

        assert!(result.contains("▉ IMAGE[Alt]"));
    }

    #[test]
    fn test_render_image_missing_file() {
        let renderer = ImageRenderer::new(Some(&std::env::temp_dir()), TerminalImageMode::Auto);
        let result = renderer.render_image("nonexistent_file_12345.png", "Missing", None);

        assert!(result.contains("▉ IMAGE[Missing]"));
    }

    #[test]
    fn test_render_image_relative_path() {
        let renderer = ImageRenderer::new(Some(&std::env::temp_dir()), TerminalImageMode::Auto);
        // This file doesn't exist, so we get fallback, but path is resolved correctly
        let result = renderer.render_image("subdir/image.png", "Test", None);

        assert!(result.contains("▉ IMAGE[Test]"));
    }

    #[test]
    fn test_render_image_empty_alt() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);
        let result = renderer.render_image("http://example.com/x.png", "", None);

        assert!(result.contains("▉ IMAGE[]"));
    }

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

    #[test]
    fn test_render_image_fallback_ends_with_newline() {
        let renderer = ImageRenderer::new(None, TerminalImageMode::Auto);
        let result = renderer.render_image("http://example.com/x.png", "Test", None);

        assert!(result.ends_with('\n'));
    }

    #[test]
    fn test_terminal_options_image_defaults() {
        // This test specifically checks TerminalOptions::default() image behavior
        let options = TerminalOptions::default();

        assert!(matches!(options.image_mode, TerminalImageMode::Auto));
        assert!(options.base_path.is_none());
    }

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

    #[test]
    fn test_hyperlink_mode_default_is_auto() {
        // Default should be Auto mode
        let options = TerminalOptions::default();
        assert!(
            matches!(options.hyperlink_mode, HyperlinkMode::Auto),
            "Default hyperlink_mode should be Auto"
        );
    }
}
