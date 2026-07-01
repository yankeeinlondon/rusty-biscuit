//! Terminal image rendering with Kitty graphics protocol and iTerm2 fallback.
//!
//! This module provides terminal image display using the Kitty graphics protocol
//! with automatic fallback to iTerm2 inline images or plain text for unsupported
//! terminals.
//!
//! ## Module Layout
//!
//! - [`width`] — `ImageWidth` plus the spec/filepath parsers.
//! - [`kitty`] — Kitty protocol render paths.
//! - [`iterm`] — iTerm2 protocol render paths.
//! - [`cursor`] — cursor row computation and scroll compensation helpers.
//! - [`protocol`] — protocol dispatch (`render_to_terminal`, `render_inline`).
//!
//! ## Width Specification
//!
//! Images can have their width specified using the `|` delimiter:
//!
//! - `filename.jpg|25` - Fixed width of 25 characters
//! - `filename.jpg|50%` - 50% of available terminal width
//! - `filename.jpg|fill` - Fill available width
//! - `filename.jpg` - Default to 50% width

use std::io::Cursor;
use std::path::Path;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use image::{DynamicImage, ImageFormat, ImageReader};

use crate::{
    components::renderable::TerminalRenderable,
    render_tree::render::resolve_cells,
    terminal::Terminal,
    utils::layout::{Alignment, Layout, Length, TargetValue},
};

use renderable::tree::{RenderNode, TreeRenderable};

mod cursor;
mod iterm;
mod kitty;
mod protocol;
mod width;

pub use self::width::{
    ImageWidth, calculate_display_dimensions, parse_filepath_and_width, parse_width_spec,
};

/// Error types for terminal image operations.
///
/// This enum variants cover the full lifecycle of terminal image rendering:
/// file access, validation, loading, encoding, and terminal compatibility.
///
/// ## Error Categories
///
/// - **File Errors**: `FileNotFound`, `InvalidPath`, `IoError` - problems accessing image files
/// - **Validation Errors**: `InvalidWidthSpec`, `PathTraversalBlocked`, `FileTooLarge`, `RemoteUrlBlocked` - security and configuration issues
/// - **Processing Errors**: `ImageLoadError`, `EncodingError` - image decoding and encoding failures
/// - **Runtime Errors**: `UnsupportedTerminal`, `ViuerError` - terminal capability issues
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::terminal_image::TerminalImageError;
///
/// // Pattern matching on error variants
/// let err = TerminalImageError::FileNotFound {
///     path: "/missing/image.png".to_string()
/// };
/// match err {
///     TerminalImageError::FileNotFound { path } => {
///         eprintln!("Image not found: {}", path);
///     }
///     TerminalImageError::UnsupportedTerminal => {
///         eprintln!("Terminal doesn't support images");
///     }
///     _ => {
///         eprintln!("Other error: {}", err);
///     }
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum TerminalImageError {
    /// File does not exist at the specified path.
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    /// Path could not be parsed or resolved.
    #[error("Invalid path '{path}': {reason}")]
    InvalidPath { path: String, reason: String },

    /// Width specification could not be parsed.
    #[error(
        "Invalid width specification '{spec}': expected a number, percentage (e.g., '50%'), or 'fill'"
    )]
    InvalidWidthSpec { spec: String },

    /// I/O error when reading the image file.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Image loading or format error from the image crate.
    #[error("Failed to load image: {0}")]
    ImageLoadError(#[from] image::ImageError),

    /// Base64 or PNG encoding failure.
    #[error("Encoding error: {message}")]
    EncodingError { message: String },

    /// Terminal does not support image rendering.
    #[error("Terminal does not support image rendering")]
    UnsupportedTerminal,

    /// Path traversal attempt detected (security violation).
    #[error("Path traversal blocked: '{path}' is outside allowed base path")]
    PathTraversalBlocked { path: String },

    /// File exceeds maximum allowed size.
    #[error("File too large: {size} bytes exceeds limit of {max_size} bytes")]
    FileTooLarge { size: u64, max_size: u64 },

    /// Remote URLs are not allowed.
    #[error("Remote URLs not allowed: '{url}'")]
    RemoteUrlBlocked { url: String },

    /// viuer rendering error.
    #[error("viuer rendering error: {message}")]
    ViuerError { message: String },
}

/// A terminal image component that can be rendered using various protocols.
///
/// This struct represents an image file that can be displayed in terminals
/// supporting inline graphics. It supports the **Kitty graphics protocol**
/// and **iTerm2 inline images**, with automatic protocol selection based on
/// terminal capabilities.
///
/// ## Protocol Support
///
/// | Terminal | Protocol | Notes |
/// |----------|----------|-------|
/// | Kitty    | Kitty    | Primary support, best feature set |
/// | iTerm2   | iTerm2   | Native OSC 1337 handling |
/// | WezTerm  | Kitty    | Requires explicit cell sizing |
/// | Ghostty  | Kitty    | Full support |
/// | Warp     | Kitty    | Uses floor-based row counting |
/// | Other    | None     | Falls back to alt text |
///
/// ## Usage
///
/// Create a `TerminalImage` from a file path and render it:
///
/// ```rust,no_run
/// use biscuit_terminal::components::terminal_image::TerminalImage;
/// use biscuit_terminal::components::renderable::TerminalRenderable;
/// use biscuit_terminal::terminal::Terminal;
/// use std::path::Path;
///
/// // Create from file path
/// let image = TerminalImage::new(Path::new("screenshot.png")).unwrap();
///
/// // Or parse from string with width specification
/// let image = TerminalImage::from_spec("diagram.svg|50%").unwrap();
///
/// // Render to terminal
/// let term = Terminal::new();
/// let output = image.render(&term);
/// print!("{}", output);
/// ```
///
/// ## Width Specification
///
/// Images can be sized using the `|` delimiter:
///
/// ```rust,no_run
/// use biscuit_terminal::components::terminal_image::{TerminalImage, ImageWidth};
/// use std::path::Path;
///
/// // Fixed 25 character width
/// let img = TerminalImage::new(Path::new("img.png")).unwrap()
///     .with_width(ImageWidth::Characters(25));
///
/// // 50% of available terminal width (default)
/// let img = TerminalImage::new(Path::new("img.png")).unwrap()
///     .with_width(ImageWidth::Percent(0.5));
///
/// // Fill all available width
/// let img = TerminalImage::new(Path::new("img.png")).unwrap()
///     .with_width(ImageWidth::Fill);
/// ```
///
/// ## Security
///
/// The `TerminalImage` type validates paths to prevent path traversal attacks.
/// Use `TerminalImageOptions` with a `base_path` to restrict file access.
///
/// ## Layout & Style Contract
///
/// `TerminalImage` is a bespoke image-protocol component (spec C5/D5). The
/// Kitty / iTerm2 / Sixel escape sequences are irreducible — no render-tree
/// `NodeKind` can represent an inline image protocol — so the component
/// emits the protocol bytes directly through `TerminalRenderable::render`.
/// The fold-style box model does not apply.
///
/// The honored subset (spec C5: minimum bar) is the outer placement:
///
/// | Property | Status | Rationale |
/// |----------|--------|-----------|
/// | `Layout::margin` | **Honored** | Reduces `available_width` and seeds `x_offset` via `resolve_dimensions`. |
/// | `Layout::alignment` | **Honored** | Selects left / center / right placement of the image canvas within the slack. |
/// | `Layout::max_width` | **N/A** | The image canvas is sized by `ImageWidth` (the explicit contract for this component). `Layout::width` and `Layout::max_width` are not read by the resolver — `ImageWidth::Fill` / `Percent` / `Characters` is the sole width control. This is the documented TerminalImage-specific carve-out, not a silent no-op. |
/// | `Layout::width` | **N/A** | See `max_width` above — `ImageWidth` is the explicit contract. |
/// | `Layout::padding` | **N/A** | The image protocol has no "padding box"; padding cannot paint protocol bytes. |
/// | `Layout::word_wrap` | **N/A** | An image protocol escape cannot wrap. |
/// | `Style::color` / `emphasis` | **N/A** | The image bytes are not text the SGR can recolor. |
/// | `Style::background` | **N/A** | A block background would have to paint cells *around* the protocol bytes; the protocol owns the cells it covers. |
/// | `Style::border` | **N/A** | Box-drawing glyphs cannot frame an image-protocol escape without corrupting the protocol's cell coverage. |
///
/// Parity for the honored subset is pinned in `terminal_image_parity.rs`
/// via `TerminalImage::resolve_dimensions_for` — the single width/margin
/// calculator both `TerminalImage` and `GraphExpression` share.
#[derive(Debug)]
pub struct TerminalImage {
    /// Fully qualified filename (absolute path).
    pub filename: String,
    /// Relative file path from CWD.
    pub relative: String,

    /// Alternative text for terminals which do not support images.
    pub alt_text: Option<String>,

    /// Raw width specification string (e.g., "|25" or "|50%").
    pub width_raw: Option<String>,

    /// Parsed image width specification.
    pub width: ImageWidth,

    /// Layout configuration for the TerminalRenderable trait (authoritative margins/width).
    layout: Layout,
}

impl Default for TerminalImage {
    fn default() -> Self {
        Self {
            filename: String::new(),
            relative: String::from("."),
            alt_text: None,
            width_raw: None,
            width: ImageWidth::default(),
            layout: Layout::default(),
        }
    }
}

impl TerminalRenderable for TerminalImage {
    /// Fallback render using terminal capabilities.
    ///
    /// Attempts inline rendering; if unsupported, returns an empty string (no alt text).
    fn render(&self, term: &Terminal) -> String {
        match self.render_to_terminal(term) {
            Ok(output) => output,
            Err(e) => {
                tracing::warn!(
                    file = %self.relative,
                    error = %e,
                    "Image render failed, returning empty string"
                );
                String::new()
            }
        }
    }

    /// Render the image as a string of Kitty escape sequences.
    ///
    /// Uses `q=2` (quiet mode) to suppress the terminal's protocol response,
    /// which would otherwise appear as garbage text when printed from a string.
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        match self.render_as_kitty(width) {
            Ok(escape_seq) => escape_seq,
            Err(_) => self.alt_text.clone().unwrap_or_default(),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

impl TreeRenderable for TerminalImage {
    /// Projects the image into a [`NodeKind::Paragraph`] wrapping an inline
    /// [`NodeKind::Image`] and carrying the component's outer-box [`Layout`]
    /// (margin, alignment).
    ///
    /// The Kitty/iTerm2 protocol bytes are irreducible (spec C5/D5); the
    /// structural image node carries placement and degrades to alt text on
    /// targets that cannot render inline images. The `url` carries the image's
    /// basename so Browser/Markdown output is portable; `alt` falls back to
    /// the generated alt-text label.
    ///
    /// [`NodeKind::Image`]: renderable::tree::NodeKind::Image
    /// [`NodeKind::Paragraph`]: renderable::tree::NodeKind::Paragraph
    fn render_tree(&self) -> RenderNode {
        let alt = self.generate_alt_text();
        // Use the basename so tree snapshots are portable across machines;
        // the full filesystem path lives on the `filename` field for the
        // bespoke protocol path.
        let url = Path::new(&self.filename)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let image = RenderNode::image(&url, None, &alt);
        let mut node = RenderNode::paragraph(vec![image]);
        node.attrs.set_layout(&self.layout);
        node
    }
}

/// Computed dimensions for terminal image rendering.
///
/// This struct captures the calculated dimensions after applying margins,
/// width specifications, and alignment. It is returned by `TerminalImage::resolve_dimensions()`
/// to provide all the necessary values for positioning and rendering an image
/// within a terminal of known width.
///
/// ## Fields
///
/// - **`available_width`**: The width remaining after left and right margins are subtracted
///   from the terminal width.
/// - **`image_width`**: The resolved width of the image in terminal character cells.
/// - **`x_offset`**: The horizontal offset where image rendering begins (includes left margin
///   and any alignment adjustment).
/// - **`left_margin`**: The left margin in character cells.
/// - **`right_margin`**: The right margin in character cells.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_terminal::components::terminal_image::{TerminalImage, ImageWidth};
/// use biscuit_terminal::components::renderable::TerminalRenderable;
/// use biscuit_terminal::utils::layout::{Length, TargetValue};
/// use std::path::Path;
///
/// let image = TerminalImage::new(Path::new("test.png")).unwrap()
///     .with_width(ImageWidth::Percent(0.8))
///     .left_margin(TargetValue::universal(Length::ch(10)))
///     .right_margin(TargetValue::universal(Length::ch(10)));
///
/// // Resolve dimensions for a 80-character wide terminal
/// let dims = image.resolve_dimensions(80);
///
/// // available_width = 80 - 10 - 10 = 60
/// // image_width = 60 * 0.8 = 48 (rounded)
/// // x_offset = 10 (left margin, assuming left alignment)
/// assert_eq!(dims.left_margin, 10);
/// assert_eq!(dims.right_margin, 10);
/// ```
///
/// ```rust,no_run
/// use biscuit_terminal::components::terminal_image::TerminalImage;
/// use biscuit_terminal::components::renderable::TerminalRenderable;
/// use biscuit_terminal::utils::layout::{Alignment, Layout, Length, Edges, TargetValue};
/// use std::path::Path;
///
/// let image = TerminalImage::new(Path::new("diagram.svg")).unwrap()
///     .with_layout(Layout {
///         margin: Edges::x(Length::percent(10.0).unwrap()),
///         alignment: Alignment::Center,
///         ..Layout::default()
///     });
///
/// let dims = image.resolve_dimensions(100);
/// // Left and right margins are 10% of 100 = 10 chars each
/// // available_width = 100 - 10 - 10 = 80
/// assert_eq!(dims.available_width, 80);
/// ```
#[derive(Debug, Clone)]
pub struct ResolvedDimensions {
    /// Available width after margins are applied
    pub available_width: u32,
    /// The resolved image width in cells/characters
    pub image_width: u32,
    /// X offset for positioning (includes left margin and alignment)
    pub x_offset: u32,
    /// Left margin in characters
    pub left_margin: u32,
    /// Right margin in characters
    pub right_margin: u32,
}

impl TerminalImage {
    /// Resolve image dimensions based on terminal width, margins, and width specification.
    ///
    /// This helper centralizes the width/margin calculations that were previously
    /// duplicated across multiple rendering methods.
    ///
    /// ## Arguments
    ///
    /// * `term_width` - Total terminal width in characters
    ///
    /// ## Returns
    ///
    /// `ResolvedDimensions` containing the calculated values for rendering.
    pub fn resolve_dimensions(&self, term_width: u32) -> ResolvedDimensions {
        Self::resolve_dimensions_for(&self.width, &self.layout, term_width)
    }

    /// Compute resolved image-area dimensions for an arbitrary `(width, layout)`
    /// pair without needing an existing image on disk.
    ///
    /// Used by callers that want to know the target cell count before
    /// rasterising the image — for example, `GraphExpression::render_to_image`
    /// derives a target pixel width from the cell count so the rasterizer can
    /// render the SVG directly at terminal display resolution.
    pub fn resolve_dimensions_for(
        width: &ImageWidth,
        layout: &Layout,
        term_width: u32,
    ) -> ResolvedDimensions {
        let term_width = term_width.max(1);

        let resolved_left = resolve_cells(&layout.margin.left, term_width);
        let resolved_right = resolve_cells(&layout.margin.right, term_width);

        let available_width = term_width
            .saturating_sub(resolved_left + resolved_right)
            .max(1);

        let image_width = match width {
            ImageWidth::Fill => available_width,
            ImageWidth::Percent(pct) => ((term_width as f32) * pct).round() as u32,
            ImageWidth::Characters(chars) => *chars,
        }
        .clamp(1, available_width);

        let slack = available_width.saturating_sub(image_width);
        let x_offset = match layout.alignment {
            Alignment::Left => resolved_left,
            Alignment::Center => resolved_left + slack / 2,
            Alignment::Right => resolved_left + slack,
        };

        ResolvedDimensions {
            available_width,
            image_width,
            x_offset,
            left_margin: resolved_left,
            right_margin: resolved_right,
        }
    }

    /// Create a new TerminalImage from a file path.
    ///
    /// ## Errors
    ///
    /// Returns `TerminalImageError::FileNotFound` if the file does not exist.
    /// Returns `TerminalImageError::InvalidPath` if the path cannot be canonicalized.
    pub fn new(filepath: &Path) -> Result<Self, TerminalImageError> {
        if !filepath.exists() {
            return Err(TerminalImageError::FileNotFound {
                path: filepath.to_string_lossy().to_string(),
            });
        }

        let absolute_path =
            std::fs::canonicalize(filepath).map_err(|e| TerminalImageError::InvalidPath {
                path: filepath.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

        Ok(Self {
            filename: absolute_path.to_string_lossy().to_string(),
            relative: filepath.to_string_lossy().to_string(),
            ..Default::default()
        })
    }

    /// Create a TerminalImage from a filepath string with optional width specification.
    ///
    /// Parses strings like `"image.png|50%"` or `"photo.jpg|80"`.
    ///
    /// ## Errors
    ///
    /// Returns error if filepath is invalid or width spec cannot be parsed.
    pub fn from_spec(spec: &str) -> Result<Self, TerminalImageError> {
        let (filepath, width_spec) = parse_filepath_and_width(spec)?;
        let path = Path::new(&filepath);

        let mut img = Self::new(path)?;

        if let Some(ref ws) = width_spec {
            img.width = parse_width_spec(ws)?;
            img.width_raw = Some(format!("|{}", ws));
        }

        Ok(img)
    }

    /// Load the image from disk.
    ///
    /// Supports raster formats (PNG, JPEG, GIF, WebP, etc.) via the `image` crate
    /// and SVG files via `resvg` rasterization when the `image` feature is enabled.
    ///
    /// ## Errors
    ///
    /// Returns `TerminalImageError::ImageLoadError` if the image cannot be loaded.
    /// Returns `TerminalImageError::UnsupportedTerminal` for SVG files when the `image`
    /// feature is disabled.
    pub fn load_image(&self) -> Result<DynamicImage, TerminalImageError> {
        if self.is_svg() {
            #[cfg(feature = "image")]
            {
                self.load_svg()
            }
            #[cfg(not(feature = "image"))]
            {
                Err(TerminalImageError::UnsupportedTerminal)
            }
        } else {
            let img = ImageReader::open(&self.filename)?
                .with_guessed_format()?
                .decode()?;
            tracing::debug!(
                file = %self.filename,
                width = img.width(),
                height = img.height(),
                "Loaded raster image"
            );
            Ok(img)
        }
    }

    /// Check if the file is an SVG based on extension.
    pub(super) fn is_svg(&self) -> bool {
        Path::new(&self.filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
    }

    /// Rasterize an SVG file to a `DynamicImage` using resvg.
    #[cfg(feature = "image")]
    fn load_svg(&self) -> Result<DynamicImage, TerminalImageError> {
        let svg_data = std::fs::read(&self.filename)?;
        let tree = resvg::usvg::Tree::from_data(&svg_data, &resvg::usvg::Options::default())
            .map_err(|e| TerminalImageError::EncodingError {
                message: format!("SVG parse error: {e}"),
            })?;

        let size = tree.size();
        let (w, h) = (size.width() as u32, size.height() as u32);
        if w == 0 || h == 0 {
            return Err(TerminalImageError::EncodingError {
                message: "SVG has zero dimensions".to_string(),
            });
        }

        let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or_else(|| {
            TerminalImageError::EncodingError {
                message: format!("Failed to create {w}x{h} pixmap for SVG"),
            }
        })?;

        resvg::render(
            &tree,
            resvg::usvg::Transform::default(),
            &mut pixmap.as_mut(),
        );

        // tiny_skia stores premultiplied alpha — demultiply for image::RgbaImage
        let mut rgba = pixmap.data().to_vec();
        for chunk in rgba.chunks_exact_mut(4) {
            let a = chunk[3] as u16;
            if a > 0 && a < 255 {
                chunk[0] = ((chunk[0] as u16 * 255 + a / 2) / a).min(255) as u8;
                chunk[1] = ((chunk[1] as u16 * 255 + a / 2) / a).min(255) as u8;
                chunk[2] = ((chunk[2] as u16 * 255 + a / 2) / a).min(255) as u8;
            }
        }
        let result = image::RgbaImage::from_raw(w, h, rgba)
            .map(DynamicImage::ImageRgba8)
            .ok_or_else(|| TerminalImageError::EncodingError {
                message: "Failed to create image from SVG rasterization".to_string(),
            });
        if result.is_ok() {
            tracing::debug!(
                file = %self.filename,
                width = w,
                height = h,
                "Loaded SVG image"
            );
        }
        result
    }

    /// Encode a DynamicImage as PNG bytes.
    ///
    /// ## Errors
    ///
    /// Returns `TerminalImageError::EncodingError` if PNG encoding fails.
    pub fn encode_as_png(&self, img: &DynamicImage) -> Result<Vec<u8>, TerminalImageError> {
        let mut buffer = Cursor::new(Vec::new());
        img.write_to(&mut buffer, ImageFormat::Png).map_err(|e| {
            TerminalImageError::EncodingError {
                message: format!("PNG encoding failed: {}", e),
            }
        })?;
        Ok(buffer.into_inner())
    }

    /// Encode bytes as base64 string.
    pub fn encode_as_base64(&self, data: &[u8]) -> String {
        BASE64.encode(data)
    }

    /// Generate alt text from the filename.
    ///
    /// If `alt_text` is set, returns that. Otherwise generates from filename.
    pub fn generate_alt_text(&self) -> String {
        if let Some(ref alt) = self.alt_text {
            return alt.clone();
        }

        // Extract filename from path
        let path = Path::new(&self.filename);
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "image".to_string());

        format!("[Image: {}]", filename)
    }

    /// Set the alt text for this image.
    pub fn with_alt_text(mut self, alt: impl Into<String>) -> Self {
        self.alt_text = Some(alt.into());
        self
    }

    /// Set the width specification for this image.
    pub fn with_width(mut self, width: ImageWidth) -> Self {
        self.width = width;
        self
    }

    /// Set the margins for this image.
    pub fn with_margins(mut self, left: u32, right: u32) -> Self {
        self.layout.margin.left = TargetValue::universal(Length::ch(left));
        self.layout.margin.right = TargetValue::universal(Length::ch(right));
        self
    }

    /// Validate that the path is not a remote URL.
    ///
    /// ## Arguments
    ///
    /// * `allow_remote` - Whether remote URLs are allowed
    ///
    /// ## Errors
    ///
    /// Returns `TerminalImageError::RemoteUrlBlocked` if the filename looks like
    /// a URL and remote URLs are not allowed.
    #[cfg(test)]
    fn validate_not_remote_url(&self, allow_remote: bool) -> Result<(), TerminalImageError> {
        if !allow_remote {
            let lower = self.filename.to_lowercase();
            if lower.starts_with("http://") || lower.starts_with("https://") {
                return Err(TerminalImageError::RemoteUrlBlocked {
                    url: self.filename.clone(),
                });
            }
        }
        Ok(())
    }

    /// Validate that the path does not escape the base path.
    ///
    /// ## Arguments
    ///
    /// * `base_path` - Optional base path for security boundary
    ///
    /// ## Errors
    ///
    /// Returns `TerminalImageError::PathTraversalBlocked` if the file path
    /// escapes the base path after canonicalization.
    #[cfg(test)]
    fn validate_path_traversal(
        &self,
        base_path: &Option<std::path::PathBuf>,
    ) -> Result<(), TerminalImageError> {
        if let Some(base) = base_path {
            // Canonicalize both paths for comparison
            let canonical_file = std::fs::canonicalize(&self.filename).map_err(|e| {
                TerminalImageError::InvalidPath {
                    path: self.filename.clone(),
                    reason: e.to_string(),
                }
            })?;

            let canonical_base =
                std::fs::canonicalize(base).map_err(|e| TerminalImageError::InvalidPath {
                    path: base.to_string_lossy().to_string(),
                    reason: e.to_string(),
                })?;

            if !canonical_file.starts_with(&canonical_base) {
                return Err(TerminalImageError::PathTraversalBlocked {
                    path: self.filename.clone(),
                });
            }
        }
        Ok(())
    }

    /// Validate that the file size is within the allowed limit.
    ///
    /// ## Arguments
    ///
    /// * `max_size` - Maximum allowed file size in bytes
    ///
    /// ## Errors
    ///
    /// Returns `TerminalImageError::FileTooLarge` if the file exceeds the limit.
    #[cfg(test)]
    fn validate_file_size(&self, max_size: u64) -> Result<(), TerminalImageError> {
        let metadata = std::fs::metadata(&self.filename)?;
        let size = metadata.len();

        if size > max_size {
            return Err(TerminalImageError::FileTooLarge { size, max_size });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
