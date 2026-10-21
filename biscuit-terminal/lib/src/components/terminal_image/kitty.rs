//! Kitty graphics-protocol rendering paths for [`TerminalImage`].

use std::borrow::Cow;

use crate::discovery::detection::probe::next_kitty_id;
use crate::discovery::detection::{GraphicsTransport, TerminalApp};
use crate::discovery::fonts::CellSize;

use super::placeholder::placeholder_rows;
use super::{TerminalImage, TerminalImageError};

/// A Kitty image uploaded with a virtual placement, and the placeholder
/// rows that draw it.
pub(super) struct PlaceholderImage {
    /// The `U=1` upload; it places nothing on its own.
    pub upload: String,
    /// One line of text per image row, already indented by the image's
    /// horizontal offset.
    pub rows: Vec<String>,
}

/// The size and placement of a Kitty image on `term`.
struct KittyGeometry<'a> {
    png_data: Cow<'a, [u8]>,
    width_cells: u32,
    height_cells: u32,
    /// The unrounded height, for terminal-specific cursor correction.
    raw_height: f32,
    x_offset: u32,
    cell_pixels: (u32, u32),
}

impl TerminalImage {
    fn kitty_geometry(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<KittyGeometry<'_>, TerminalImageError> {
        let term_width = term.width().max(1);
        let pixels = self.pixels()?;
        let (pixel_width, pixel_height) = pixels.size();
        let cell = term.cell_size();
        let dims = self.resolve_dimensions_for_image(term_width, cell, pixel_width);
        let target_cells = dims.image_width;

        let (cell_pixel_width, cell_pixel_height) = cell
            .map(|cs| (cs.width.max(1), cs.height.max(1)))
            .unwrap_or((CellSize::FALLBACK.width, CellSize::FALLBACK.height));

        let png_data = pixels.into_png()?;
        let image_aspect = pixel_height as f32 / pixel_width as f32;
        let cell_aspect = cell_pixel_width as f32 / cell_pixel_height as f32;
        let raw_height = target_cells as f32 * image_aspect * cell_aspect;
        let height_cells = super::cursor::covered_rows(
            target_cells,
            (pixel_width, pixel_height),
            (cell_pixel_width, cell_pixel_height),
        );
        Ok(KittyGeometry {
            png_data,
            width_cells: target_cells,
            height_cells,
            raw_height,
            x_offset: dims.x_offset,
            cell_pixels: (cell_pixel_width, cell_pixel_height),
        })
    }

    /// The image drawn with Unicode placeholders: the upload followed by
    /// each placeholder row and a line break, leaving the cursor at the start
    /// of the line below the image. `None` when the grid cannot be encoded
    /// (see [`placeholder_rows`]); the caller then places the image
    /// ordinarily.
    pub(super) fn render_kitty_placeholders_for_terminal(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<Option<String>, TerminalImageError> {
        let geometry = self.kitty_geometry(term)?;
        let Some(image) = self.render_kitty_placeholders(
            &geometry.png_data,
            geometry.width_cells,
            geometry.height_cells,
            geometry.x_offset,
        ) else {
            return Ok(None);
        };
        tracing::trace!(
            file = %self.relative,
            protocol = "kitty",
            placement = "placeholders",
            width_cells = geometry.width_cells,
            height_cells = geometry.height_cells,
            "Kitty image render"
        );
        let mut output = image.upload;
        for row in image.rows {
            output.push_str(&row);
            output.push_str("\r\n");
        }
        Ok(Some(output))
    }

    /// A `U=1` upload of `png_data` over `width_cells` x `height_cells` with
    /// a fresh image ID, and its placeholder rows indented by `x_offset`
    /// spaces. `None` when the grid cannot be encoded.
    pub(super) fn render_kitty_placeholders(
        &self,
        png_data: &[u8],
        width_cells: u32,
        height_cells: u32,
        x_offset: u32,
    ) -> Option<PlaceholderImage> {
        let id = next_kitty_id();
        let rows = placeholder_rows(id, width_cells, height_cells)?;
        // Spaces rather than cursor movement: a column layout measures the
        // rows as text.
        let indent = " ".repeat(x_offset as usize);
        Some(PlaceholderImage {
            upload: self.kitty_chunks(
                png_data,
                &format!("q=2,i={id},f=100,a=T,U=1,t=d,c={width_cells},r={height_cells}"),
            ),
            rows: rows.into_iter().map(|row| format!("{indent}{row}")).collect(),
        })
    }

    /// Build Kitty protocol image output and logical row height for cursor placement.
    ///
    /// Returns a save/restore-wrapped sequence, the ceiled height in cells, and
    /// the raw (unrounded) height for terminal-specific cursor correction.
    pub(super) fn render_kitty_for_terminal(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<(String, u32, f32), TerminalImageError> {
        let KittyGeometry {
            png_data,
            width_cells: target_cells,
            height_cells,
            raw_height,
            x_offset,
            cell_pixels: (cell_pixel_width, cell_pixel_height),
        } = self.kitty_geometry(term)?;

        // WezTerm needs explicit row sizing for correct geometry. Kitty-family
        // peers render correctly with width-only sizing. Through tmux the
        // outer terminal is not `term.app`, so rows are always explicit, and
        // the upload carries its own ID.
        let image_seq = if term.graphics_route.transport == GraphicsTransport::Tmux {
            self.render_kitty_with_id(&png_data, target_cells, height_cells, next_kitty_id())
        } else if matches!(term.app, TerminalApp::Wezterm) {
            self.render_kitty_cells(&png_data, target_cells, height_cells)
        } else {
            self.render_kitty_width_only(&png_data, target_cells)
        };

        let prefix = if x_offset > 0 {
            format!("\x1b[{}C", x_offset)
        } else {
            String::new()
        };

        tracing::trace!(
            file = %self.relative,
            protocol = "kitty",
            width_cells = target_cells,
            height_cells,
            raw_height,
            x_offset,
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
        let pixels = self.pixels()?;
        let (pixel_width, pixel_height) = pixels.size();
        let cell = crate::discovery::fonts::cell_size();
        let dims = self.resolve_dimensions_for_image(term_width, cell, pixel_width);

        // Use resolved dimensions
        let target_cells = dims.image_width;

        // Use measured cell size when available for correct aspect ratio calculation.
        let (cell_pixel_width, cell_pixel_height) = cell
            .map(|cs| (cs.width.max(1), cs.height.max(1)))
            .unwrap_or((CellSize::FALLBACK.width, CellSize::FALLBACK.height));

        // Don't resize the image - send it at original resolution and let Kitty handle scaling.
        // This preserves maximum quality and lets Kitty's aspect ratio preservation work correctly.
        let png_data = pixels.png()?;

        // Kitty preserves aspect ratio when only c= is specified (no r=).
        let display_cells_height = super::cursor::covered_rows(
            target_cells,
            (pixel_width, pixel_height),
            (cell_pixel_width, cell_pixel_height),
        );

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
        self.kitty_chunks(
            png_data,
            &format!("q=2,f=100,a=T,t=d,c={width_cells},r={height_cells}"),
        )
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
        // Only `c=` (columns): Kitty computes the rows from the aspect ratio.
        self.kitty_chunks(png_data, &format!("q=2,f=100,a=T,t=d,c={width_cells}"))
    }

    /// [`render_kitty_cells`](Self::render_kitty_cells) with image ID `id`
    /// (`i=`) on the upload. Quiet mode still suppresses every reply.
    pub(super) fn render_kitty_with_id(
        &self,
        png_data: &[u8],
        width_cells: u32,
        height_cells: u32,
        id: u32,
    ) -> String {
        self.kitty_chunks(
            png_data,
            &format!("q=2,i={id},f=100,a=T,t=d,c={width_cells},r={height_cells}"),
        )
    }

    /// `png_data` as base64 Kitty chunks of at most 4096 bytes: the first
    /// carries `params`, every chunk carries `m=` (1 while more follow).
    fn kitty_chunks(&self, png_data: &[u8], params: &str) -> String {
        let base64_data = self.encode_as_base64(png_data);
        let chunks: Vec<&str> = base64_data
            .as_bytes()
            .chunks(4096)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect();
        let mut result = String::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let more = u8::from(i + 1 < chunks.len());
            if i == 0 {
                result.push_str(&format!("\x1b_G{params},m={more};{chunk}\x1b\\"));
            } else {
                result.push_str(&format!("\x1b_Gm={more};{chunk}\x1b\\"));
            }
        }
        result
    }
}
