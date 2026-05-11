//! iTerm2 inline-images protocol rendering paths for [`TerminalImage`].

use std::path::Path;

use super::width::ImageWidth;
use super::{TerminalImage, TerminalImageError};

impl TerminalImage {
    /// Build iTerm2 protocol image output and logical row height for cursor placement.
    ///
    /// Returns a save/restore-wrapped sequence, the ceiled height in cells, and
    /// the raw (unrounded) height for terminal-specific cursor correction.
    pub(super) fn render_iterm2_for_terminal(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<(String, u32, f32), TerminalImageError> {
        let term_width = term.width().max(1);
        let dims = self.resolve_dimensions(term_width);
        let img = self.load_image()?;
        let target_cells = dims.image_width;
        let (cell_pixel_width, cell_pixel_height) = term
            .cell_size()
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

        tracing::trace!(
            file = %self.relative,
            protocol = "iterm2",
            width_cells = target_cells,
            height_cells,
            raw_height,
            x_offset = dims.x_offset,
            cell_size = ?format!("{}x{}", cell_pixel_width, cell_pixel_height),
            "iTerm2 image render"
        );

        Ok((
            format!("\x1b[s{}{}\x1b[u", prefix, image),
            height_cells,
            raw_height,
        ))
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
