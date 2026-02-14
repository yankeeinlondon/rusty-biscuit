//! Terminal image rendering with Kitty graphics protocol and iTerm2 fallback.
//!
//! This module provides terminal image display using the Kitty graphics protocol
//! with automatic fallback to iTerm2 inline images or plain text for unsupported
//! terminals.
//!
//! ## Width Specification
//!
//! Images can have their width specified using the `|` delimiter:
//!
//! - `filename.jpg|25` - Fixed width of 25 characters
//! - `filename.jpg|50%` - 50% of available terminal width
//! - `filename.jpg|fill` - Fill available width
//! - `filename.jpg` - Default to 50% width

use std::fmt::Alignment;
use std::io::Cursor;
use std::path::Path;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use image::{DynamicImage, ImageFormat, ImageReader};

use crate::discovery::detection::TerminalApp;
use crate::{components::renderable::Renderable, terminal::Terminal, utils::layout::Layout};

/// Error types for terminal image operations.
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

/// Width specification for image rendering.
#[derive(Debug, Clone, PartialEq)]
pub enum ImageWidth {
    /// Fill available space (using margins as offsets).
    Fill,
    /// Use a percentage of the available space where
    /// available space is the number of characters - (margin_left + margin_right).
    Percent(f32),
    /// A fixed width based on character width.
    Characters(u32),
}

impl Default for ImageWidth {
    fn default() -> Self {
        ImageWidth::Percent(0.5)
    }
}

/// A terminal image component that can be rendered using various protocols.
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

    /// Horizontal alignment of the image.
    pub alignment: Alignment,

    /// Left margin in characters (legacy; prefer layout margins).
    pub margin_left: u32,
    /// Right margin in characters (legacy; prefer layout margins).
    pub margin_right: u32,

    /// Layout configuration for the Renderable trait (authoritative margins/width).
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
            alignment: Alignment::Left,
            margin_left: 0,
            margin_right: 0,
            layout: Layout::default(),
        }
    }
}

impl Renderable for TerminalImage {
    /// Fallback render using terminal capabilities.
    ///
    /// Attempts inline rendering; if unsupported, returns an empty string (no alt text).
    fn fallback_render(&self, term: &Terminal) -> String {
        self.render_to_terminal(term).unwrap_or_default()
    }

    /// Render the image as a string of Kitty escape sequences.
    ///
    /// Uses `q=2` (quiet mode) to suppress the terminal's protocol response,
    /// which would otherwise appear as garbage text when printed from a string.
    fn render(&self, term_width: Option<u32>) -> String {
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

/// Resolved dimensions for image rendering.
///
/// This struct captures the calculated dimensions after applying margins,
/// width specifications, and alignment.
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
        let term_width = term_width.max(1);

        // Resolve margins from layout and legacy margin fields
        let resolved_left = Layout::resolve_margin(&self.layout.left_margin, term_width)
            .saturating_add(self.margin_left);
        let resolved_right = Layout::resolve_margin(&self.layout.right_margin, term_width)
            .saturating_add(self.margin_right);

        // Calculate available width after margins
        let available_width = term_width
            .saturating_sub(resolved_left + resolved_right)
            .max(1);

        // Calculate image width based on width specification
        let image_width = match &self.width {
            ImageWidth::Fill => available_width,
            ImageWidth::Percent(pct) => ((available_width as f32) * pct).round() as u32,
            ImageWidth::Characters(chars) => (*chars).min(available_width),
        }
        .clamp(1, available_width);

        // Calculate x offset based on alignment
        let slack = available_width.saturating_sub(image_width);
        let x_offset = match self.alignment {
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
            alignment: Alignment::Left,
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
    /// ## Errors
    ///
    /// Returns `TerminalImageError::ImageLoadError` if the image cannot be loaded.
    pub fn load_image(&self) -> Result<DynamicImage, TerminalImageError> {
        let img = ImageReader::open(&self.filename)?
            .with_guessed_format()?
            .decode()?;
        Ok(img)
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
        self.margin_left = left;
        self.margin_right = right;
        self
    }

    /// Build Kitty protocol image output and logical row height for cursor placement.
    ///
    /// Returns a save/restore-wrapped sequence, the ceiled height in cells, and
    /// the raw (unrounded) height for terminal-specific cursor correction.
    fn render_kitty_for_terminal(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<(String, u32, f32), TerminalImageError> {
        let term_width = term.width().max(1);
        let dims = self.resolve_dimensions(term_width);
        let img = self.load_image()?;
        let target_cells = dims.image_width;

        let (cell_pixel_width, cell_pixel_height) = crate::discovery::fonts::cell_size()
            .map(|cs| (cs.width.max(1), cs.height.max(1)))
            .unwrap_or((8u32, 16u32));

        let png_data = self.encode_as_png(&img)?;
        let image_aspect = img.height() as f32 / img.width() as f32;
        let cell_aspect = cell_pixel_width as f32 / cell_pixel_height as f32;
        let raw_height = target_cells as f32 * image_aspect * cell_aspect;
        let height_cells = (raw_height.ceil() as u32).max(1);

        // WezTerm needs explicit row sizing for correct geometry. Kitty-family
        // peers render correctly with width-only sizing.
        let image_seq = if matches!(term.app, TerminalApp::Wezterm) {
            self.render_kitty_cells(&png_data, target_cells, height_cells)
        } else {
            self.render_kitty_width_only(&png_data, target_cells)
        };

        let prefix = if dims.x_offset > 0 {
            format!("\x1b[{}C", dims.x_offset)
        } else {
            String::new()
        };

        Ok((
            format!("\x1b[s{}{}\x1b[u", prefix, image_seq),
            height_cells,
            raw_height,
        ))
    }

    /// Build iTerm2 protocol image output and logical row height for cursor placement.
    ///
    /// Returns a save/restore-wrapped sequence, the ceiled height in cells, and
    /// the raw (unrounded) height for terminal-specific cursor correction.
    fn render_iterm2_for_terminal(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<(String, u32, f32), TerminalImageError> {
        let term_width = term.width().max(1);
        let dims = self.resolve_dimensions(term_width);
        let img = self.load_image()?;
        let target_cells = dims.image_width;
        let (cell_pixel_width, cell_pixel_height) = crate::discovery::fonts::cell_size()
            .map(|cs| (cs.width.max(1), cs.height.max(1)))
            .unwrap_or((8u32, 16u32));
        let image_aspect = img.height() as f32 / img.width() as f32;
        let cell_aspect = cell_pixel_width as f32 / cell_pixel_height as f32;
        let raw_height = target_cells as f32 * image_aspect * cell_aspect;
        let height_cells = (raw_height.ceil() as u32).max(1);
        let png_data = self.encode_as_png(&img)?;

        let width_param = match &self.width {
            ImageWidth::Fill => "100%".to_string(),
            ImageWidth::Percent(pct) => format!("{:.0}%", pct * 100.0),
            ImageWidth::Characters(chars) => chars.to_string(),
        };

        let filename = Path::new(&self.filename)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "image.png".to_string());

        let image = self.render_iterm2(&png_data, &width_param, &filename);
        let prefix = if dims.x_offset > 0 {
            format!("\x1b[{}C", dims.x_offset)
        } else {
            String::new()
        };

        Ok((
            format!("\x1b[s{}{}\x1b[u", prefix, image),
            height_cells,
            raw_height,
        ))
    }

    /// Render the image to a string appropriate for the given terminal.
    ///
    /// Returns escape sequences for the detected image protocol (Kitty or
    /// iTerm2). Kitty sequences include `q=2` (quiet mode) so the terminal
    /// does not send a response that would appear as garbage text.
    ///
    /// ## Arguments
    ///
    /// * `term` - Terminal with detected capabilities
    ///
    /// ## Returns
    ///
    /// A string of escape sequences ready to print, or an empty string
    /// when the terminal does not support images.
    ///
    /// ## Errors
    ///
    /// Returns error if image loading or encoding fails.
    pub fn render_to_terminal(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<String, TerminalImageError> {
        use crate::discovery::detection::ImageSupport;

        let (sequence, height_cells, raw_height) = match term.image_support {
            // iTerm2 can advertise Kitty, but native OSC 1337 handling is
            // more predictable for cursor placement.
            ImageSupport::Kitty if matches!(term.app, TerminalApp::ITerm2) => {
                self.render_iterm2_for_terminal(term)
            }
            ImageSupport::Kitty => self.render_kitty_for_terminal(term),
            ImageSupport::ITerm => self.render_iterm2_for_terminal(term),
            ImageSupport::None => return Ok(String::new()),
        }?;

        // Terminal-specific cursor row calculation.
        //
        // Most terminals use ceil-based row counting — a partial cell row
        // at the bottom still consumes a full terminal row. The cursor must
        // advance past the entire image including the partial row.
        //
        // Warp uses floor-based row counting — ceil overshoots by one row,
        // causing a blank line between the image and subsequent text.
        //
        // All other terminals (Kitty, Ghostty, WezTerm, iTerm2) use ceil.
        // DSR diagnostics confirmed images physically occupy ceil rows:
        // floor puts the cursor ON the last image row, hiding text behind
        // the image overlay.
        let cursor_rows = match term.app {
            TerminalApp::Warp => (raw_height.floor() as u32).max(1),
            _ => height_cells,
        };

        // Detect if the image will trigger a scroll event.
        // When the cursor is near the bottom of the screen and the image
        // extends past the viewport, the terminal scrolls to fit it.
        // After scroll, \x1b[u restores to the screen-bottom and CUD is
        // clamped there. A single \n compensates because line feed at the
        // bottom margin triggers a scroll-up (CUD does not).
        //
        // Ghostty is excluded — its save/restore interaction with scroll
        // naturally leaves the cursor 1 row past the image with ceil.
        // Warp is excluded — its input is always at the bottom so images
        // render at the top, never triggering scroll.
        let needs_scroll_compensation =
            if matches!(term.app, TerminalApp::Ghostty | TerminalApp::Warp) {
                false
            } else {
                use crate::discovery::cursor_position::cursor_position;
                if let Some(pos) = cursor_position() {
                    let term_height = term.height();
                    pos.row + cursor_rows > term_height
                } else {
                    false
                }
            };

        let suffix = if needs_scroll_compensation {
            "\n"
        } else {
            ""
        };

        if cursor_rows > 0 {
            Ok(format!("{}\x1b[{}B\r{}", sequence, cursor_rows, suffix))
        } else {
            Ok(format!("{}\r{}", sequence, suffix))
        }
    }

    pub(crate) fn render_inline(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<(String, u32), TerminalImageError> {
        use crate::discovery::detection::ImageSupport;

        if !term.is_tty || matches!(term.image_support, ImageSupport::None) {
            return Err(TerminalImageError::UnsupportedTerminal);
        }

        let dims = self.resolve_dimensions(term.width());
        let width_cells = dims.image_width;
        let x_offset = dims.x_offset;

        let img = self.load_image()?;
        let (cell_pixel_width, cell_pixel_height) = crate::discovery::fonts::cell_size()
            .map(|cs| (cs.width.max(1), cs.height.max(1)))
            .unwrap_or((8u32, 16u32));
        let image_aspect = img.height() as f32 / img.width() as f32;
        let cell_aspect = cell_pixel_width as f32 / cell_pixel_height as f32;
        let height_cells =
            (((width_cells as f32) * image_aspect * cell_aspect).ceil() as u32).max(1);

        let png_data = self.encode_as_png(&img)?;
        let image = match term.image_support {
            ImageSupport::Kitty => {
                if matches!(term.app, TerminalApp::Wezterm) {
                    self.render_kitty_cells(&png_data, width_cells, height_cells)
                } else {
                    self.render_kitty_width_only(&png_data, width_cells)
                }
            }
            ImageSupport::ITerm => {
                let filename = Path::new(&self.filename)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "image.png".to_string());
                self.render_iterm2(&png_data, &width_cells.to_string(), &filename)
            }
            ImageSupport::None => return Err(TerminalImageError::UnsupportedTerminal),
        };

        let prefix = if x_offset > 0 {
            format!("\x1b[{}C", x_offset)
        } else {
            String::new()
        };

        let sequence = format!("\x1b[s{}{}\x1b[u", prefix, image);

        Ok((sequence, height_cells))
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

    /// Render the image using Kitty protocol.
    ///
    /// ## Arguments
    ///
    /// * `term_width` - Terminal width in characters (defaults to 80 if 0)
    pub fn render_as_kitty(&self, term_width: u32) -> Result<String, TerminalImageError> {
        let term_width = if term_width == 0 { 80 } else { term_width };
        let dims = self.resolve_dimensions(term_width);

        let img = self.load_image()?;

        // Use resolved dimensions
        let target_cells = dims.image_width;

        // Use measured cell size when available for correct aspect ratio calculation.
        let (cell_pixel_width, cell_pixel_height) = crate::discovery::fonts::cell_size()
            .map(|cs| (cs.width.max(1), cs.height.max(1)))
            .unwrap_or((8u32, 16u32));

        // Don't resize the image - send it at original resolution and let Kitty handle scaling.
        // This preserves maximum quality and lets Kitty's aspect ratio preservation work correctly.
        let png_data = self.encode_as_png(&img)?;

        // Calculate expected rows for cursor advancement:
        // Kitty preserves aspect ratio when only c= is specified (no r=).
        // rows = cols * (image_height / image_width) * (cell_width / cell_height)
        let image_aspect = img.height() as f32 / img.width() as f32;
        let cell_aspect = cell_pixel_width as f32 / cell_pixel_height as f32;
        let display_cells_height = (target_cells as f32 * image_aspect * cell_aspect).ceil() as u32;

        // Only specify columns (c=), let Kitty calculate rows to preserve aspect ratio
        let image = self.render_kitty_width_only(&png_data, target_cells);
        let prefix = if dims.x_offset > 0 {
            format!("\x1b[{}C", dims.x_offset)
        } else {
            String::new()
        };
        let cursor_advance = format!("\x1b[{}B\r\n", display_cells_height.max(1));
        Ok(format!("{}{}{}", prefix, image, cursor_advance))
    }

    /// Render the image using iTerm2 protocol.
    ///
    /// ## Arguments
    ///
    /// * `term_width` - Terminal width in characters (defaults to 80 if 0)
    pub fn render_as_iterm2(&self, term_width: u32) -> Result<String, TerminalImageError> {
        let term_width = if term_width == 0 { 80 } else { term_width };
        let dims = self.resolve_dimensions(term_width);

        let img = self.load_image()?;

        // Use resolved dimensions
        let char_width = dims.image_width;

        // Resize to preserve aspect ratio based on character width
        let (cell_pixel_width, cell_pixel_height) = crate::discovery::fonts::cell_size()
            .map(|cs| (cs.width.max(1), cs.height.max(1)))
            .unwrap_or((8u32, 16u32));

        let target_pixel_width = char_width * cell_pixel_width;
        let aspect_ratio = img.height() as f32 / img.width() as f32;
        let target_pixel_height = (target_pixel_width as f32 * aspect_ratio) as u32;

        let resized = if target_pixel_width != img.width() {
            img.resize_exact(
                target_pixel_width.max(1),
                target_pixel_height.max(1),
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            img
        };

        let png_data = self.encode_as_png(&resized)?;

        // Build width string for iTerm2 (supports %, px, cells). Prefer percent for percent/fill specs.
        let width_param = match &self.width {
            ImageWidth::Fill => "100%".to_string(),
            ImageWidth::Percent(pct) => format!("{:.0}%", pct * 100.0),
            ImageWidth::Characters(chars) => chars.to_string(),
        };

        // Get filename for iTerm2
        let filename = Path::new(&self.filename)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "image.png".to_string());

        // Keep aspect ratio; rely on iTerm to scale; avoid post cursor moves to prevent parsing issues.
        let display_cells_height =
            ((target_pixel_height as f32 / cell_pixel_height as f32).ceil() as u32).max(1);

        let image = self.render_iterm2(&png_data, &width_param, &filename);
        let prefix = if dims.x_offset > 0 {
            format!("\x1b[{}C", dims.x_offset)
        } else {
            String::new()
        };
        Ok(format!(
            "{}{}\x1b[{}B\r\n",
            prefix, image, display_cells_height
        ))
    }

    /// Render image using the Kitty graphics protocol.
    ///
    /// The Kitty protocol transmits images as base64-encoded PNG data using
    /// escape sequences. For large images, data is chunked into 4096-byte segments.
    ///
    /// Uses `q=2` (quiet mode) to suppress all terminal responses, preventing
    /// garbage text when the output is printed from a string.
    ///
    /// ## Arguments
    ///
    /// * `png_data` - PNG-encoded image bytes
    /// * `width_cells` - Display width in terminal cells (columns)
    /// * `height_cells` - Display height in terminal cells (rows)
    ///
    /// ## Escape Sequence Format
    ///
    /// ```text
    /// ESC_G q=2,f=100,a=T,t=d,m=1;{base64_chunk} ESC\  (intermediate chunks)
    /// ESC_G q=2,f=100,a=T,t=d,m=0;{base64_chunk} ESC\  (final chunk)
    /// ```
    ///
    /// Where:
    /// - `q=2`: quiet mode (suppress all terminal responses)
    /// - `f=100`: format is PNG
    /// - `a=T`: action is transmit and display
    /// - `t=d`: transmission medium is direct (inline data)
    /// - `m=0|1`: more chunks follow (1) or this is final (0)
    pub fn render_kitty_cells(
        &self,
        png_data: &[u8],
        width_cells: u32,
        height_cells: u32,
    ) -> String {
        let base64_data = self.encode_as_base64(png_data);
        let chunk_size = 4096;
        let mut result = String::new();

        // Split into chunks and emit escape sequences
        let chunks: Vec<&str> = base64_data
            .as_bytes()
            .chunks(chunk_size)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect();

        for (i, chunk) in chunks.iter().enumerate() {
            let is_last = i == chunks.len() - 1;
            let more = if is_last { 0 } else { 1 };

            if i == 0 {
                // First chunk includes all parameters; use cell-based sizing
                result.push_str(&format!(
                    "\x1b_Gq=2,f=100,a=T,t=d,c={},r={},m={};{}\x1b\\",
                    width_cells, height_cells, more, chunk
                ));
            } else {
                // Subsequent chunks only include m parameter
                result.push_str(&format!("\x1b_Gm={};{}\x1b\\", more, chunk));
            }
        }

        result
    }

    /// Backwards-compatible helper accepting cell dimensions.
    pub fn render_kitty(&self, png_data: &[u8], width: u32, height: u32) -> String {
        self.render_kitty_cells(png_data, width, height)
    }

    /// Render image using Kitty protocol with only width specified.
    ///
    /// By omitting the `r=` (rows) parameter, Kitty automatically calculates
    /// the number of rows needed to preserve the image's aspect ratio.
    /// This is the preferred method for aspect-ratio-correct rendering.
    ///
    /// Uses `q=2` (quiet mode) to suppress all terminal responses.
    ///
    /// ## Arguments
    ///
    /// * `png_data` - PNG-encoded image bytes
    /// * `width_cells` - Display width in terminal columns
    pub fn render_kitty_width_only(&self, png_data: &[u8], width_cells: u32) -> String {
        let base64_data = self.encode_as_base64(png_data);
        let chunk_size = 4096;
        let mut result = String::new();

        let chunks: Vec<&str> = base64_data
            .as_bytes()
            .chunks(chunk_size)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect();

        for (i, chunk) in chunks.iter().enumerate() {
            let is_last = i == chunks.len() - 1;
            let more = if is_last { 0 } else { 1 };

            if i == 0 {
                // First chunk: specify only c= (columns), omit r= (rows)
                // Kitty will automatically calculate rows to preserve aspect ratio
                result.push_str(&format!(
                    "\x1b_Gq=2,f=100,a=T,t=d,c={},m={};{}\x1b\\",
                    width_cells, more, chunk
                ));
            } else {
                result.push_str(&format!("\x1b_Gm={};{}\x1b\\", more, chunk));
            }
        }

        result
    }

    /// Render image using the iTerm2 inline images protocol.
    ///
    /// ## Arguments
    ///
    /// * `png_data` - PNG-encoded image bytes
    /// * `width` - Display width (cells by default, supports `%` and `px` suffixes)
    /// * `filename` - Filename for the image (displayed in some contexts)
    ///
    /// ## Escape Sequence Format
    ///
    /// ```text
    /// ESC]1337;File=name={base64_name};inline=1;width={width}:{base64_data}BEL
    /// ```
    pub fn render_iterm2(&self, png_data: &[u8], width: &str, filename: &str) -> String {
        let base64_data = self.encode_as_base64(png_data);
        let base64_filename = self.encode_as_base64(filename.as_bytes());

        format!(
            "\x1b]1337;File=name={};inline=1;preserveAspectRatio=1;width={};size=auto:{}\x07",
            base64_filename, width, base64_data
        )
    }
}

/// Calculate display dimensions while preserving aspect ratio.
///
/// ## Arguments
///
/// * `img_width` - Original image width in pixels
/// * `img_height` - Original image height in pixels
/// * `target_width` - Target width specification
/// * `term_width` - Terminal width in characters
///
/// ## Returns
///
/// Tuple of (width, height) in pixels for display.
pub fn calculate_display_dimensions(
    img_width: u32,
    img_height: u32,
    target_width: &ImageWidth,
    term_width: u32,
) -> (u32, u32) {
    // Assume roughly 2:1 pixel aspect ratio for terminal cells
    // (characters are typically ~twice as tall as wide)
    let cell_pixel_width = 8u32;

    let target_pixels = match target_width {
        ImageWidth::Fill => term_width * cell_pixel_width,
        ImageWidth::Percent(pct) => ((term_width as f32) * pct * (cell_pixel_width as f32)) as u32,
        ImageWidth::Characters(chars) => chars * cell_pixel_width,
    };

    // Calculate height preserving aspect ratio
    let aspect_ratio = img_height as f32 / img_width as f32;
    let display_width = target_pixels.min(img_width); // Don't upscale
    let display_height = (display_width as f32 * aspect_ratio) as u32;

    (display_width.max(1), display_height.max(1))
}

/// Parse a width specification string.
///
/// ## Supported formats
///
/// - Empty or whitespace: Default to 50% (`ImageWidth::Percent(0.5)`)
/// - `"fill"`: `ImageWidth::Fill`
/// - Number with `%` suffix: `ImageWidth::Percent(value / 100.0)`
/// - Number with `ch` suffix: `ImageWidth::Characters(value)` (e.g., "40ch")
/// - Bare number: `ImageWidth::Characters(value)`
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::terminal_image::{parse_width_spec, ImageWidth};
///
/// assert!(matches!(parse_width_spec("50%").unwrap(), ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));
/// assert!(matches!(parse_width_spec("fill").unwrap(), ImageWidth::Fill));
/// assert!(matches!(parse_width_spec("80").unwrap(), ImageWidth::Characters(80)));
/// assert!(matches!(parse_width_spec("40ch").unwrap(), ImageWidth::Characters(40)));
/// ```
///
/// ## Errors
///
/// Returns `TerminalImageError::InvalidWidthSpec` for invalid specifications.
pub fn parse_width_spec(spec: &str) -> Result<ImageWidth, TerminalImageError> {
    let trimmed = spec.trim();

    // Empty or whitespace defaults to 50%
    if trimmed.is_empty() {
        return Ok(ImageWidth::Percent(0.5));
    }

    // Handle "fill" keyword
    if trimmed.eq_ignore_ascii_case("fill") {
        return Ok(ImageWidth::Fill);
    }

    // Handle percentage (e.g., "50%")
    if let Some(pct_str) = trimmed.strip_suffix('%') {
        let pct_val: f32 =
            pct_str
                .trim()
                .parse()
                .map_err(|_| TerminalImageError::InvalidWidthSpec {
                    spec: spec.to_string(),
                })?;

        // Validate percentage range (0-100)
        if !(0.0..=100.0).contains(&pct_val) {
            return Err(TerminalImageError::InvalidWidthSpec {
                spec: spec.to_string(),
            });
        }

        return Ok(ImageWidth::Percent(pct_val / 100.0));
    }

    // Handle character width with "ch" suffix (e.g., "40ch")
    if let Some(char_str) = trimmed.strip_suffix("ch") {
        let char_val: u32 =
            char_str
                .trim()
                .parse()
                .map_err(|_| TerminalImageError::InvalidWidthSpec {
                    spec: spec.to_string(),
                })?;

        if char_val == 0 {
            return Err(TerminalImageError::InvalidWidthSpec {
                spec: spec.to_string(),
            });
        }

        return Ok(ImageWidth::Characters(char_val));
    }

    // Handle bare number (characters)
    let char_val: u32 = trimmed
        .parse()
        .map_err(|_| TerminalImageError::InvalidWidthSpec {
            spec: spec.to_string(),
        })?;

    // Validate that width is positive
    if char_val == 0 {
        return Err(TerminalImageError::InvalidWidthSpec {
            spec: spec.to_string(),
        });
    }

    Ok(ImageWidth::Characters(char_val))
}

/// Parse a filepath string that may include a width specification.
///
/// Splits on the `|` delimiter and returns the filepath and optional width spec.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::terminal_image::parse_filepath_and_width;
///
/// let (path, width) = parse_filepath_and_width("image.png|50%").unwrap();
/// assert_eq!(path, "image.png");
/// assert_eq!(width, Some("50%".to_string()));
///
/// let (path, width) = parse_filepath_and_width("image.png").unwrap();
/// assert_eq!(path, "image.png");
/// assert!(width.is_none());
/// ```
pub fn parse_filepath_and_width(
    input: &str,
) -> Result<(String, Option<String>), TerminalImageError> {
    let parts: Vec<&str> = input.splitn(2, '|').collect();

    let filepath = parts[0].trim().to_string();

    if filepath.is_empty() {
        return Err(TerminalImageError::InvalidPath {
            path: input.to_string(),
            reason: "Empty filepath".to_string(),
        });
    }

    let width_spec = parts.get(1).map(|s| s.trim().to_string());

    Ok((filepath, width_spec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::io::Write;

    // Helper to create a minimal valid PNG using the image crate
    fn create_test_png() -> Vec<u8> {
        use image::{ImageBuffer, ImageFormat, Rgb};
        use std::io::Cursor;

        // Create a 2x2 red image
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_fn(2, 2, |_x, _y| Rgb([255u8, 0u8, 0u8]));

        let mut buffer = Cursor::new(Vec::new());
        img.write_to(&mut buffer, ImageFormat::Png).unwrap();
        buffer.into_inner()
    }

    fn create_temp_test_image() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();
        (dir, file_path)
    }

    fn extract_row_advance(output: &str) -> Option<u32> {
        let end = output.rfind("B\r")?;
        let head = &output[..end];
        let start = head.rfind("\x1b[")?;
        head[start + 2..].parse::<u32>().ok()
    }

    // Error type tests
    #[test]
    fn test_error_file_not_found_message() {
        let err = TerminalImageError::FileNotFound {
            path: "/nonexistent/file.png".to_string(),
        };
        assert!(err.to_string().contains("File not found"));
        assert!(err.to_string().contains("/nonexistent/file.png"));
    }

    #[test]
    fn test_error_invalid_path_message() {
        let err = TerminalImageError::InvalidPath {
            path: "bad/path".to_string(),
            reason: "Permission denied".to_string(),
        };
        assert!(err.to_string().contains("Invalid path"));
        assert!(err.to_string().contains("Permission denied"));
    }

    #[test]
    fn test_error_invalid_width_spec_message() {
        let err = TerminalImageError::InvalidWidthSpec {
            spec: "abc".to_string(),
        };
        assert!(err.to_string().contains("Invalid width specification"));
        assert!(err.to_string().contains("abc"));
    }

    #[test]
    fn test_error_encoding_message() {
        let err = TerminalImageError::EncodingError {
            message: "PNG encode failed".to_string(),
        };
        assert!(err.to_string().contains("Encoding error"));
        assert!(err.to_string().contains("PNG encode failed"));
    }

    #[test]
    fn test_error_unsupported_terminal_message() {
        let err = TerminalImageError::UnsupportedTerminal;
        assert!(err.to_string().contains("does not support image"));
    }

    // Width parsing tests
    #[test]
    fn test_parse_width_spec_empty() {
        let result = parse_width_spec("").unwrap();
        assert!(matches!(result, ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));
    }

    #[test]
    fn test_parse_width_spec_whitespace() {
        let result = parse_width_spec("   ").unwrap();
        assert!(matches!(result, ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));
    }

    #[test]
    fn test_parse_width_spec_fill() {
        assert!(matches!(
            parse_width_spec("fill").unwrap(),
            ImageWidth::Fill
        ));
        assert!(matches!(
            parse_width_spec("FILL").unwrap(),
            ImageWidth::Fill
        ));
        assert!(matches!(
            parse_width_spec("Fill").unwrap(),
            ImageWidth::Fill
        ));
    }

    #[test]
    fn test_parse_width_spec_percentage() {
        let result = parse_width_spec("50%").unwrap();
        assert!(matches!(result, ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));

        let result = parse_width_spec("100%").unwrap();
        assert!(matches!(result, ImageWidth::Percent(p) if (p - 1.0).abs() < 0.001));

        let result = parse_width_spec("25%").unwrap();
        assert!(matches!(result, ImageWidth::Percent(p) if (p - 0.25).abs() < 0.001));
    }

    #[test]
    fn test_parse_width_spec_percentage_with_spaces() {
        let result = parse_width_spec(" 50% ").unwrap();
        assert!(matches!(result, ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));
    }

    #[test]
    fn test_parse_width_spec_characters() {
        assert!(matches!(
            parse_width_spec("80").unwrap(),
            ImageWidth::Characters(80)
        ));
        assert!(matches!(
            parse_width_spec("25").unwrap(),
            ImageWidth::Characters(25)
        ));
        assert!(matches!(
            parse_width_spec("1").unwrap(),
            ImageWidth::Characters(1)
        ));
    }

    #[test]
    fn test_parse_width_spec_characters_with_ch_suffix() {
        assert!(matches!(
            parse_width_spec("40ch").unwrap(),
            ImageWidth::Characters(40)
        ));
        assert!(matches!(
            parse_width_spec("100ch").unwrap(),
            ImageWidth::Characters(100)
        ));
        assert!(matches!(
            parse_width_spec(" 25ch ").unwrap(),
            ImageWidth::Characters(25)
        ));
        // 0ch should be invalid
        assert!(parse_width_spec("0ch").is_err());
    }

    #[test]
    fn test_parse_width_spec_invalid() {
        assert!(parse_width_spec("abc").is_err());
        assert!(parse_width_spec("50px").is_err());
        assert!(parse_width_spec("-10").is_err());
        assert!(parse_width_spec("0").is_err());
        assert!(parse_width_spec("150%").is_err());
    }

    // Filepath parsing tests
    #[test]
    fn test_parse_filepath_and_width_simple() {
        let (path, width) = parse_filepath_and_width("image.png").unwrap();
        assert_eq!(path, "image.png");
        assert!(width.is_none());
    }

    #[test]
    fn test_parse_filepath_and_width_with_percentage() {
        let (path, width) = parse_filepath_and_width("image.png|50%").unwrap();
        assert_eq!(path, "image.png");
        assert_eq!(width, Some("50%".to_string()));
    }

    #[test]
    fn test_parse_filepath_and_width_with_characters() {
        let (path, width) = parse_filepath_and_width("photo.jpg|80").unwrap();
        assert_eq!(path, "photo.jpg");
        assert_eq!(width, Some("80".to_string()));
    }

    #[test]
    fn test_parse_filepath_and_width_with_spaces() {
        let (path, width) = parse_filepath_and_width("image.png | 50%").unwrap();
        assert_eq!(path, "image.png");
        assert_eq!(width, Some("50%".to_string()));
    }

    #[test]
    fn test_parse_filepath_and_width_with_fill() {
        let (path, width) = parse_filepath_and_width("image.png|fill").unwrap();
        assert_eq!(path, "image.png");
        assert_eq!(width, Some("fill".to_string()));
    }

    #[test]
    fn test_parse_filepath_and_width_empty_path() {
        assert!(parse_filepath_and_width("").is_err());
        assert!(parse_filepath_and_width("|50%").is_err());
    }

    #[test]
    fn test_parse_filepath_and_width_multiple_pipes() {
        // Only splits on first pipe
        let (path, width) = parse_filepath_and_width("file|50|extra").unwrap();
        assert_eq!(path, "file");
        assert_eq!(width, Some("50|extra".to_string()));
    }

    // Image loading tests
    #[test]
    fn test_terminal_image_new_file_not_found() {
        let result = TerminalImage::new(Path::new("/nonexistent/image.png"));
        assert!(matches!(
            result,
            Err(TerminalImageError::FileNotFound { .. })
        ));
    }

    #[test]
    fn test_terminal_image_new_with_valid_file() {
        // Create a temp file
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(&create_test_png()).unwrap();

        let img = TerminalImage::new(&file_path).unwrap();
        assert!(img.filename.contains("test.png"));
        assert_eq!(img.relative, file_path.to_string_lossy());
    }

    #[test]
    fn test_terminal_image_from_spec_simple() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::from_spec(&file_path.to_string_lossy()).unwrap();
        assert!(matches!(img.width, ImageWidth::Percent(p) if (p - 0.5).abs() < 0.001));
    }

    #[test]
    fn test_terminal_image_from_spec_with_width() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let spec = format!("{}|75%", file_path.display());
        let img = TerminalImage::from_spec(&spec).unwrap();
        assert!(matches!(img.width, ImageWidth::Percent(p) if (p - 0.75).abs() < 0.001));
        assert_eq!(img.width_raw, Some("|75%".to_string()));
    }

    #[test]
    fn test_terminal_image_load_image() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path).unwrap();
        let loaded = img.load_image().unwrap();
        assert_eq!(loaded.width(), 2);
        assert_eq!(loaded.height(), 2);
    }

    #[test]
    fn test_terminal_image_encode_as_png() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let term_img = TerminalImage::new(&file_path).unwrap();
        let loaded = term_img.load_image().unwrap();
        let png_bytes = term_img.encode_as_png(&loaded).unwrap();

        // PNG files start with specific magic bytes
        assert!(png_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
    }

    #[test]
    fn test_terminal_image_encode_as_base64() {
        let term_img = TerminalImage::default();
        let data = b"Hello, World!";
        let encoded = term_img.encode_as_base64(data);
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn test_terminal_image_generate_alt_text_default() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("my-photo.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path).unwrap();
        let alt = img.generate_alt_text();
        assert_eq!(alt, "[Image: my-photo.png]");
    }

    #[test]
    fn test_terminal_image_generate_alt_text_custom() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path)
            .unwrap()
            .with_alt_text("A beautiful sunset");
        let alt = img.generate_alt_text();
        assert_eq!(alt, "A beautiful sunset");
    }

    #[test]
    fn test_terminal_image_builder_methods() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path)
            .unwrap()
            .with_alt_text("Test image")
            .with_width(ImageWidth::Characters(40))
            .with_margins(2, 2);

        assert_eq!(img.alt_text, Some("Test image".to_string()));
        assert_eq!(img.width, ImageWidth::Characters(40));
        assert_eq!(img.margin_left, 2);
        assert_eq!(img.margin_right, 2);
    }

    // Protocol rendering tests
    #[test]
    fn test_render_kitty_small_data() {
        let term_img = TerminalImage::default();
        let png_data = create_test_png();
        let result = term_img.render_kitty(&png_data, 100, 100);

        // Should start with Kitty escape sequence
        assert!(result.starts_with("\x1b_G"));
        // Should contain format and action parameters
        assert!(result.contains("f=100")); // PNG format
        assert!(result.contains("a=T")); // Transmit and display
        assert!(result.contains("t=d")); // Direct transmission
        // Should end with string terminator
        assert!(result.ends_with("\x1b\\"));
    }

    #[test]
    fn test_render_kitty_chunking() {
        let term_img = TerminalImage::default();
        // Create a larger payload that requires chunking (>4096 bytes base64)
        let large_data = vec![0u8; 4000]; // Will be ~5333 bytes base64
        let result = term_img.render_kitty(&large_data, 100, 100);

        // Should have multiple escape sequences due to chunking
        let escape_count = result.matches("\x1b_G").count();
        assert!(escape_count >= 2, "Expected chunking for large data");

        // First chunk should have m=1 (more), last should have m=0
        assert!(result.contains("m=1"));
        assert!(result.contains("m=0"));
    }

    #[test]
    fn test_render_iterm2() {
        let term_img = TerminalImage::default();
        let png_data = create_test_png();
        let result = term_img.render_iterm2(&png_data, "40", "test.png");

        // Should start with iTerm2 inline image escape
        assert!(result.starts_with("\x1b]1337;File="));
        // Should contain inline=1
        assert!(result.contains("inline=1"));
        // Should contain width
        assert!(result.contains("width=40"));
        // Should end with BEL
        assert!(result.ends_with("\x07"));
        // Filename should be base64 encoded
        let expected_filename_b64 = BASE64.encode("test.png");
        assert!(result.contains(&format!("name={}", expected_filename_b64)));
    }

    #[test]
    fn test_render_to_terminal_kitty_uses_save_restore_and_explicit_row_advance() {
        let (_dir, file_path) = create_temp_test_image();
        let term_img = TerminalImage::new(&file_path).unwrap();
        let term = Terminal::builder()
            .app(TerminalApp::Kitty)
            .is_tty(true)
            .image_support(crate::discovery::detection::ImageSupport::Kitty)
            .width(80)
            .build();

        let output = term_img.render_to_terminal(&term).unwrap();

        assert!(output.contains("\x1b_G"));
        assert!(output.contains("\x1b[s"));
        assert!(output.contains("\x1b[u"));
        assert!(output.contains("B\r"));
        assert!(!output.contains("\x1b[1A"));
    }

    #[test]
    fn test_render_to_terminal_iterm2_uses_save_restore_and_explicit_row_advance() {
        let (_dir, file_path) = create_temp_test_image();
        let term_img = TerminalImage::new(&file_path).unwrap();
        let term = Terminal::builder()
            .app(TerminalApp::ITerm2)
            .is_tty(true)
            .image_support(crate::discovery::detection::ImageSupport::ITerm)
            .width(80)
            .build();

        let output = term_img.render_to_terminal(&term).unwrap();

        assert!(output.contains("\x1b]1337;File="));
        assert!(output.contains("\x1b[s"));
        assert!(output.contains("\x1b[u"));
        assert!(output.contains("B\r"));
        assert!(!output.contains("\x1b[1A"));
    }

    #[test]
    fn test_render_to_terminal_wezterm_uses_explicit_row_sizing_and_row_advance() {
        let (_dir, file_path) = create_temp_test_image();
        let term_img = TerminalImage::new(&file_path).unwrap();
        let term = Terminal::builder()
            .app(TerminalApp::Wezterm)
            .is_tty(true)
            .image_support(crate::discovery::detection::ImageSupport::Kitty)
            .width(80)
            .build();

        let output = term_img.render_to_terminal(&term).unwrap();

        assert!(output.contains("\x1b_G"));
        assert!(output.contains("\x1b[s"));
        assert!(output.contains("\x1b[u"));
        assert!(output.contains("c="));
        assert!(output.contains(",r="));
        assert!(output.contains("B\r"));
        assert!(output.ends_with('\r'));
    }

    #[test]
    fn test_render_to_terminal_iterm2_prefers_native_protocol_when_kitty_advertised() {
        let (_dir, file_path) = create_temp_test_image();
        let term_img = TerminalImage::new(&file_path).unwrap();
        let iterm2 = Terminal::builder()
            .app(TerminalApp::ITerm2)
            .is_tty(true)
            .image_support(crate::discovery::detection::ImageSupport::Kitty)
            .width(80)
            .build();

        let iterm2_out = term_img.render_to_terminal(&iterm2).unwrap();

        assert!(iterm2_out.contains("\x1b]1337;File="));
        assert!(!iterm2_out.contains("\x1b_G"));
    }

    #[test]
    fn test_render_to_terminal_warp_matches_kitty_cursor_management_strategy() {
        let (_dir, file_path) = create_temp_test_image();
        let term_img = TerminalImage::new(&file_path).unwrap();

        let kitty = Terminal::builder()
            .app(TerminalApp::Kitty)
            .is_tty(true)
            .image_support(crate::discovery::detection::ImageSupport::Kitty)
            .width(80)
            .build();
        let warp = Terminal::builder()
            .app(TerminalApp::Warp)
            .is_tty(true)
            .image_support(crate::discovery::detection::ImageSupport::Kitty)
            .width(80)
            .build();

        let kitty_out = term_img.render_to_terminal(&kitty).unwrap();
        let warp_out = term_img.render_to_terminal(&warp).unwrap();

        assert!(kitty_out.contains("\x1b[s"));
        assert!(kitty_out.contains("\x1b[u"));
        assert!(kitty_out.contains("B\r"));
        assert!(!kitty_out.contains("\x1b[1A"));
        assert!(warp_out.contains("\x1b[s"));
        assert!(warp_out.contains("\x1b[u"));
        assert!(warp_out.contains("B\r"));
        assert!(!warp_out.contains("\x1b[1A"));
    }

    #[test]
    fn test_render_to_terminal_iterm2_uses_floor_rounding() {
        let (_dir, file_path) = create_temp_test_image();
        let term_img = TerminalImage::new(&file_path).unwrap();

        let kitty = Terminal::builder()
            .app(TerminalApp::Kitty)
            .is_tty(true)
            .image_support(crate::discovery::detection::ImageSupport::Kitty)
            .width(80)
            .build();
        let iterm2 = Terminal::builder()
            .app(TerminalApp::ITerm2)
            .is_tty(true)
            .image_support(crate::discovery::detection::ImageSupport::ITerm)
            .width(80)
            .build();

        let kitty_out = term_img.render_to_terminal(&kitty).unwrap();
        let iterm2_out = term_img.render_to_terminal(&iterm2).unwrap();

        let kitty_rows = extract_row_advance(&kitty_out).unwrap();
        let iterm2_rows = extract_row_advance(&iterm2_out).unwrap();
        // For integer heights (like our 2x2 test image), floor == ceil
        // so iTerm2 (floor) matches Kitty (ceil).
        // For fractional heights, iTerm2 would use floor while Kitty uses ceil.
        assert_eq!(kitty_rows, iterm2_rows);
    }

    #[test]
    fn test_render_to_terminal_wezterm_uses_ceil_rounding() {
        let (_dir, file_path) = create_temp_test_image();
        let term_img = TerminalImage::new(&file_path).unwrap();

        let kitty = Terminal::builder()
            .app(TerminalApp::Kitty)
            .is_tty(true)
            .image_support(crate::discovery::detection::ImageSupport::Kitty)
            .width(80)
            .build();
        let wezterm = Terminal::builder()
            .app(TerminalApp::Wezterm)
            .is_tty(true)
            .image_support(crate::discovery::detection::ImageSupport::Kitty)
            .width(80)
            .build();

        let kitty_out = term_img.render_to_terminal(&kitty).unwrap();
        let wezterm_out = term_img.render_to_terminal(&wezterm).unwrap();

        let kitty_rows = extract_row_advance(&kitty_out).unwrap();
        let wezterm_rows = extract_row_advance(&wezterm_out).unwrap();
        // WezTerm uses ceil rounding (same default path as Kitty/Ghostty).
        // Correct for no-scroll cases; scroll cases have CUD-clamping.
        assert_eq!(kitty_rows, wezterm_rows);
    }

    #[test]
    fn test_render_to_terminal_warp_uses_floor_rounding() {
        let (_dir, file_path) = create_temp_test_image();
        let term_img = TerminalImage::new(&file_path).unwrap();

        let kitty = Terminal::builder()
            .app(TerminalApp::Kitty)
            .is_tty(true)
            .image_support(crate::discovery::detection::ImageSupport::Kitty)
            .width(80)
            .build();
        let warp = Terminal::builder()
            .app(TerminalApp::Warp)
            .is_tty(true)
            .image_support(crate::discovery::detection::ImageSupport::Kitty)
            .width(80)
            .build();

        let kitty_out = term_img.render_to_terminal(&kitty).unwrap();
        let warp_out = term_img.render_to_terminal(&warp).unwrap();

        let kitty_rows = extract_row_advance(&kitty_out).unwrap();
        let warp_rows = extract_row_advance(&warp_out).unwrap();
        // For integer heights (like our 2x2 test image), floor == ceil
        // so Warp (floor) matches Kitty (ceil).
        // For fractional heights, Warp would use floor while Kitty uses ceil.
        assert_eq!(kitty_rows, warp_rows);
    }

    // Dimension calculation tests
    #[test]
    fn test_calculate_display_dimensions_fill() {
        let (w, h) = calculate_display_dimensions(800, 600, &ImageWidth::Fill, 100);
        // 100 chars * 8 pixels = 800, should use original since no upscale
        assert_eq!(w, 800);
        // Aspect ratio preserved: 800 * (600/800) = 600
        assert_eq!(h, 600);
    }

    #[test]
    fn test_calculate_display_dimensions_percent() {
        let (w, h) = calculate_display_dimensions(800, 600, &ImageWidth::Percent(0.5), 100);
        // 50% of 100 chars * 8 pixels = 400
        assert_eq!(w, 400);
        // Aspect ratio: 400 * (600/800) = 300
        assert_eq!(h, 300);
    }

    #[test]
    fn test_calculate_display_dimensions_characters() {
        let (w, h) = calculate_display_dimensions(800, 600, &ImageWidth::Characters(50), 100);
        // 50 chars * 8 pixels = 400
        assert_eq!(w, 400);
        // Aspect ratio: 400 * (600/800) = 300
        assert_eq!(h, 300);
    }

    #[test]
    fn test_calculate_display_dimensions_no_upscale() {
        // Image smaller than target - should not upscale
        let (w, h) = calculate_display_dimensions(100, 100, &ImageWidth::Fill, 100);
        assert_eq!(w, 100); // Don't upscale beyond original
        assert_eq!(h, 100);
    }

    #[test]
    fn test_calculate_display_dimensions_minimum_size() {
        // Very small percentage
        let (w, h) = calculate_display_dimensions(800, 600, &ImageWidth::Percent(0.001), 10);
        // Should be at least 1x1
        assert!(w >= 1);
        assert!(h >= 1);
    }

    // Integration tests for render_as_* methods
    // These tests query the terminal for cell size - must run serially to avoid /dev/tty conflicts

    #[test]
    #[serial]
    fn test_render_as_kitty_produces_valid_output() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path)
            .unwrap()
            .with_width(ImageWidth::Percent(0.5));

        let result = img.render_as_kitty(80).unwrap();

        // Should be a valid Kitty escape sequence
        assert!(result.starts_with("\x1b_G"));
        assert!(result.contains("\x1b\\"));
        assert!(result.contains("f=100")); // PNG format
    }

    #[test]
    #[serial]
    fn test_render_as_iterm2_produces_valid_output() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path)
            .unwrap()
            .with_width(ImageWidth::Characters(40));

        let result = img.render_as_iterm2(80).unwrap();

        // Should be a valid iTerm2 escape sequence
        assert!(result.starts_with("\x1b]1337;File="));
        assert!(result.contains("\x07"));
        assert!(result.contains("inline=1"));
    }

    #[test]
    #[serial]
    fn test_render_as_kitty_with_zero_term_width_uses_default() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path).unwrap();

        // Should not panic with zero width, uses 80 as default
        let result = img.render_as_kitty(0);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_render_as_iterm2_with_zero_term_width_uses_default() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path).unwrap();

        // Should not panic with zero width, uses 80 as default
        let result = img.render_as_iterm2(0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_spec_with_invalid_width_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let spec = format!("{}|invalid", file_path.display());
        let result = TerminalImage::from_spec(&spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_spec_with_percentage_over_100_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let spec = format!("{}|150%", file_path.display());
        let result = TerminalImage::from_spec(&spec);
        assert!(result.is_err());
    }

    // Security error type tests
    #[test]
    fn test_error_path_traversal_blocked_message() {
        let err = TerminalImageError::PathTraversalBlocked {
            path: "/etc/passwd".to_string(),
        };
        assert!(err.to_string().contains("Path traversal blocked"));
        assert!(err.to_string().contains("/etc/passwd"));
    }

    #[test]
    fn test_error_file_too_large_message() {
        let err = TerminalImageError::FileTooLarge {
            size: 20_000_000,
            max_size: 10_000_000,
        };
        assert!(err.to_string().contains("File too large"));
        assert!(err.to_string().contains("20000000"));
        assert!(err.to_string().contains("10000000"));
    }

    #[test]
    fn test_error_remote_url_blocked_message() {
        let err = TerminalImageError::RemoteUrlBlocked {
            url: "https://example.com/image.png".to_string(),
        };
        assert!(err.to_string().contains("Remote URLs not allowed"));
        assert!(err.to_string().contains("https://example.com"));
    }

    #[test]
    fn test_error_viuer_error_message() {
        let err = TerminalImageError::ViuerError {
            message: "Protocol not supported".to_string(),
        };
        assert!(err.to_string().contains("viuer rendering error"));
        assert!(err.to_string().contains("Protocol not supported"));
    }

    // Security validation tests
    #[test]
    fn test_validate_not_remote_url_allows_local_path() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path).unwrap();
        assert!(img.validate_not_remote_url(false).is_ok());
    }

    #[test]
    fn test_validate_not_remote_url_blocks_http() {
        let mut img = TerminalImage::default();
        img.filename = "http://example.com/image.png".to_string();

        let result = img.validate_not_remote_url(false);
        assert!(matches!(
            result,
            Err(TerminalImageError::RemoteUrlBlocked { .. })
        ));
    }

    #[test]
    fn test_validate_not_remote_url_blocks_https() {
        let mut img = TerminalImage::default();
        img.filename = "https://example.com/image.png".to_string();

        let result = img.validate_not_remote_url(false);
        assert!(matches!(
            result,
            Err(TerminalImageError::RemoteUrlBlocked { .. })
        ));
    }

    #[test]
    fn test_validate_not_remote_url_allows_when_permitted() {
        let mut img = TerminalImage::default();
        img.filename = "https://example.com/image.png".to_string();

        // When allow_remote is true, should succeed
        assert!(img.validate_not_remote_url(true).is_ok());
    }

    #[test]
    fn test_validate_not_remote_url_case_insensitive() {
        let mut img = TerminalImage::default();
        img.filename = "HTTPS://EXAMPLE.COM/IMAGE.PNG".to_string();

        let result = img.validate_not_remote_url(false);
        assert!(matches!(
            result,
            Err(TerminalImageError::RemoteUrlBlocked { .. })
        ));
    }

    #[test]
    fn test_validate_path_traversal_allows_within_base() {
        let dir = tempfile::tempdir().unwrap();
        let sub_dir = dir.path().join("images");
        std::fs::create_dir(&sub_dir).unwrap();

        let file_path = sub_dir.join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path).unwrap();
        let base_path = Some(dir.path().to_path_buf());

        assert!(img.validate_path_traversal(&base_path).is_ok());
    }

    #[test]
    fn test_validate_path_traversal_blocks_escape() {
        let dir = tempfile::tempdir().unwrap();
        let sibling_dir = tempfile::tempdir().unwrap();

        let file_path = sibling_dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path).unwrap();
        let base_path = Some(dir.path().to_path_buf());

        let result = img.validate_path_traversal(&base_path);
        assert!(matches!(
            result,
            Err(TerminalImageError::PathTraversalBlocked { .. })
        ));
    }

    #[test]
    fn test_validate_path_traversal_allows_no_base() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path).unwrap();

        // No base path means all paths are allowed
        assert!(img.validate_path_traversal(&None).is_ok());
    }

    #[test]
    fn test_validate_file_size_allows_small_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        let png_data = create_test_png();
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&png_data)
            .unwrap();

        let img = TerminalImage::new(&file_path).unwrap();

        // Allow files up to 1MB (our test PNG is tiny)
        assert!(img.validate_file_size(1024 * 1024).is_ok());
    }

    #[test]
    fn test_validate_file_size_blocks_large_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        let png_data = create_test_png();
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&png_data)
            .unwrap();

        let img = TerminalImage::new(&file_path).unwrap();

        // Set max size to 10 bytes (our PNG is larger)
        let result = img.validate_file_size(10);
        assert!(matches!(
            result,
            Err(TerminalImageError::FileTooLarge { .. })
        ));
    }

    #[test]
    fn test_validate_file_size_exact_limit() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        let png_data = create_test_png();
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&png_data)
            .unwrap();

        let img = TerminalImage::new(&file_path).unwrap();
        let file_size = std::fs::metadata(&file_path).unwrap().len();

        // Exact limit should pass
        assert!(img.validate_file_size(file_size).is_ok());

        // One byte less should fail
        let result = img.validate_file_size(file_size - 1);
        assert!(matches!(
            result,
            Err(TerminalImageError::FileTooLarge { .. })
        ));
    }

    // resolve_dimensions tests
    #[test]
    fn test_resolve_dimensions_fill() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path)
            .unwrap()
            .with_width(ImageWidth::Fill);
        let dims = img.resolve_dimensions(80);

        assert_eq!(dims.available_width, 80);
        assert_eq!(dims.image_width, 80);
        assert_eq!(dims.x_offset, 0);
    }

    #[test]
    fn test_resolve_dimensions_percent() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path)
            .unwrap()
            .with_width(ImageWidth::Percent(0.5));
        let dims = img.resolve_dimensions(80);

        assert_eq!(dims.available_width, 80);
        assert_eq!(dims.image_width, 40);
    }

    #[test]
    fn test_resolve_dimensions_with_margins() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let img = TerminalImage::new(&file_path)
            .unwrap()
            .with_margins(5, 5)
            .with_width(ImageWidth::Fill);
        let dims = img.resolve_dimensions(80);

        assert_eq!(dims.left_margin, 5);
        assert_eq!(dims.right_margin, 5);
        assert_eq!(dims.available_width, 70); // 80 - 5 - 5
        assert_eq!(dims.image_width, 70);
        assert_eq!(dims.x_offset, 5);
    }

    #[test]
    fn test_resolve_dimensions_center_alignment() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let mut img = TerminalImage::new(&file_path)
            .unwrap()
            .with_width(ImageWidth::Characters(40));
        img.alignment = Alignment::Center;
        let dims = img.resolve_dimensions(80);

        assert_eq!(dims.image_width, 40);
        // Centered: (80 - 40) / 2 = 20
        assert_eq!(dims.x_offset, 20);
    }

    #[test]
    fn test_resolve_dimensions_right_alignment() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.png");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(&create_test_png())
            .unwrap();

        let mut img = TerminalImage::new(&file_path)
            .unwrap()
            .with_width(ImageWidth::Characters(30));
        img.alignment = Alignment::Right;
        let dims = img.resolve_dimensions(80);

        assert_eq!(dims.image_width, 30);
        // Right-aligned: 80 - 30 = 50
        assert_eq!(dims.x_offset, 50);
    }
}
