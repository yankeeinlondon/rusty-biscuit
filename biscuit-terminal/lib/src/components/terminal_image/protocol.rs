//! Protocol dispatch — selects the Kitty or iTerm2 rendering path for a
//! given [`Terminal`] and stitches the result with cursor-advance logic.

use std::path::Path;

use crate::discovery::detection::TerminalApp;

use super::cursor::{compute_cursor_rows, needs_scroll_compensation};
use super::{TerminalImage, TerminalImageError};

impl TerminalImage {
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
    #[tracing::instrument(skip(self, term), fields(file = %self.relative))]
    pub(super) fn render_to_terminal(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<String, TerminalImageError> {
        use crate::discovery::detection::ImageSupport;

        let protocol = match term.image_support {
            ImageSupport::Kitty if matches!(term.app, TerminalApp::ITerm2) => "iterm2",
            ImageSupport::Kitty => "kitty",
            ImageSupport::ITerm => "iterm2",
            ImageSupport::None => {
                tracing::trace!(file = %self.relative, "No image support, returning empty");
                return Ok(String::new());
            }
        };

        let (sequence, height_cells, raw_height) = match term.image_support {
            // iTerm2 can advertise Kitty, but native OSC 1337 handling is
            // more predictable for cursor placement.
            ImageSupport::Kitty if matches!(term.app, TerminalApp::ITerm2) => {
                self.render_iterm2_for_terminal(term)
            }
            ImageSupport::Kitty => self.render_kitty_for_terminal(term),
            ImageSupport::ITerm => self.render_iterm2_for_terminal(term),
            ImageSupport::None => unreachable!(),
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
        let cursor_rows = compute_cursor_rows(&term.app, raw_height, height_cells);

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
        let needs_compensation =
            needs_scroll_compensation(&term.app, cursor_rows, term.height(), &self.relative);

        let suffix = if needs_compensation { "\n" } else { "" };

        tracing::debug!(
            file = %self.relative,
            protocol,
            cursor_rows,
            needs_scroll_compensation = needs_compensation,
            "Image rendered to terminal"
        );

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
        let (cell_pixel_width, cell_pixel_height) = term
            .cell_size()
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
}
