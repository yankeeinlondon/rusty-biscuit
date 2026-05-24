//! Kitty graphics-protocol rendering paths for [`TerminalImage`].

use crate::discovery::detection::TerminalApp;

use super::{TerminalImage, TerminalImageError};

impl TerminalImage {
    /// Build Kitty protocol image output and logical row height for cursor placement.
    ///
    /// Returns a save/restore-wrapped sequence, the ceiled height in cells, and
    /// the raw (unrounded) height for terminal-specific cursor correction.
    pub(super) fn render_kitty_for_terminal(
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

        tracing::trace!(
            file = %self.relative,
            protocol = "kitty",
            width_cells = target_cells,
            height_cells,
            raw_height,
            x_offset = dims.x_offset,
            cell_size = ?format!("{}x{}", cell_pixel_width, cell_pixel_height),
            "Kitty image render"
        );

        Ok((
            format!("\x1b[s{}{}\x1b[u", prefix, image_seq),
            height_cells,
            raw_height,
        ))
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
}
