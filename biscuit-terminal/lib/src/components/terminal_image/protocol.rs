//! Protocol dispatch — selects the encoder for a terminal's graphics route
//! (or the text fallback) and stitches the result with cursor-advance logic.

use std::path::Path;

use crate::discovery::detection::{
    GraphicsFormat, GraphicsRoute, GraphicsTransport, RouteConfidence, RouteReason, RouteReasonKind, TerminalApp,
};
use crate::discovery::fonts::CellSize;

use crate::discovery::detection::probe::next_kitty_id;

use super::cursor::{compute_cursor_rows, needs_scroll_compensation};
use super::tmux::{TMUX_MESSAGE_BUDGET, wrap_graphics_for_tmux};
use super::{TerminalImage, TerminalImageError};

/// The pixel encoder a route selects, or why no pixels are sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Encoder {
    Kitty,
    Iterm2,
    Fallback(RouteReason),
}

/// The encoder for `term`'s graphics route.
pub(super) fn encoder_for(term: &crate::terminal::Terminal) -> Encoder {
    let route = &term.graphics_route;
    if route.has_pixels()
        && route.needs_measured_cells
        && let Some(reason) = unmeasured_cells_reason(route, term.cell_size())
    {
        return Encoder::Fallback(reason);
    }
    match route.format {
        // iTerm2 accepts Kitty but ignores `q=2` (its `Gi=…;OK` replies land
        // in the shell's input as junk) and places nothing for a width-only
        // `c=` placement, so a Kitty route to iTerm2 uses iTerm2's own
        // format. Only an explicit `BISCUIT_GRAPHICS=kitty` keeps Kitty.
        GraphicsFormat::Kitty
            if matches!(term.app, TerminalApp::ITerm2)
                && route.confidence != RouteConfidence::Forced =>
        {
            Encoder::Iterm2
        }
        GraphicsFormat::Kitty => Encoder::Kitty,
        GraphicsFormat::Iterm2 => Encoder::Iterm2,
        GraphicsFormat::Sixel => Encoder::Fallback(RouteReason {
            kind: RouteReasonKind::NoSupport,
            message: "the route selects Sixel, whose encoder is not implemented yet (planned)"
                .to_string(),
            fix: None,
        }),
        GraphicsFormat::None => Encoder::Fallback(route.reason.clone()),
    }
}

/// Why a route that needs a measured cell size cannot be used without one.
///
/// Zellij sizes images from its own measurement of the host's cells; the
/// library's estimated cell size cannot stand in for it.
pub(super) fn unmeasured_cells_reason(route: &GraphicsRoute, cell: Option<CellSize>) -> Option<RouteReason> {
    (route.needs_measured_cells && cell.is_none()).then(|| RouteReason {
        kind: RouteReasonKind::Unknown,
        message: "the multiplexer did not report its cell size in pixels, so the image cannot be sized"
            .to_string(),
        fix: None,
    })
}

/// What [`TerminalImage::render_with_route`] produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageOutput {
    /// Escape sequences that draw the image and advance the cursor past it.
    Pixels(String),
    /// The route has no usable pixel format; `text` describes the image.
    Fallback {
        /// The image's alt text with control characters removed.
        text: String,
        /// Why no pixels were sent.
        reason: RouteReason,
    },
}

impl ImageOutput {
    /// The text to print: the escape sequences or the fallback text.
    #[must_use]
    pub fn into_text(self) -> String {
        match self {
            Self::Pixels(sequence) => sequence,
            Self::Fallback { text, .. } => text,
        }
    }
}

impl TerminalImage {
    /// Renders the image for `term`'s graphics route.
    ///
    /// Everything that can fail (loading, decoding, encoding) happens before
    /// any output is produced. A valid image whose route has no usable pixel
    /// format becomes [`ImageOutput::Fallback`] with the route's reason.
    ///
    /// ## Errors
    ///
    /// Returns the load error for a missing, unreadable, or corrupt image,
    /// whether or not the route has pixels.
    pub fn render_with_route(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<ImageOutput, TerminalImageError> {
        match encoder_for(term) {
            Encoder::Fallback(reason) => {
                // Validate as the pixel path would, so a corrupt image stays
                // an error instead of becoming alt text.
                self.pixels()?;
                Ok(ImageOutput::Fallback {
                    text: strip_controls(&self.generate_alt_text()),
                    reason,
                })
            }
            encoder => self.render_to_terminal(term, &encoder),
        }
    }

    /// Render the image to escape sequences for `encoder`, advancing the
    /// cursor past it. Through tmux, an image whose passthrough message is
    /// over tmux's limit becomes a fallback before any row is reserved.
    #[tracing::instrument(skip(self, term, encoder), fields(file = %self.relative))]
    fn render_to_terminal(
        &self,
        term: &crate::terminal::Terminal,
        encoder: &Encoder,
    ) -> Result<ImageOutput, TerminalImageError> {
        let (protocol, (sequence, height_cells, raw_height)) = match encoder {
            Encoder::Kitty => ("kitty", self.render_kitty_for_terminal(term)?),
            Encoder::Iterm2 => ("iterm2", self.render_iterm2_for_terminal(term)?),
            Encoder::Fallback(_) => unreachable!("fallback routes are handled by the caller"),
        };
        let sequence = match for_transport(term, protocol, sequence) {
            Ok(sequence) => sequence,
            Err(reason) => {
                return Ok(ImageOutput::Fallback {
                    text: strip_controls(&self.generate_alt_text()),
                    reason,
                });
            }
        };

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

        let output = if cursor_rows > 0 {
            format!("{}\x1b[{}B\r{}", sequence, cursor_rows, suffix)
        } else {
            format!("{}\r{}", sequence, suffix)
        };
        Ok(ImageOutput::Pixels(output))
    }

    pub(crate) fn render_inline(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<(String, u32), TerminalImageError> {
        if !term.is_tty {
            return Err(TerminalImageError::UnsupportedTerminal);
        }
        let encoder = encoder_for(term);
        if matches!(encoder, Encoder::Fallback(_)) {
            return Err(TerminalImageError::UnsupportedTerminal);
        }

        let pixels = self.pixels()?;
        let (pixel_width, pixel_height) = pixels.size();
        let cell = term.cell_size();
        let dims = self.resolve_dimensions_for_image(term.width(), cell, pixel_width);
        let width_cells = dims.image_width;
        let x_offset = dims.x_offset;

        let (cell_pixel_width, cell_pixel_height) = cell
            .map(|cs| (cs.width.max(1), cs.height.max(1)))
            .unwrap_or((CellSize::FALLBACK.width, CellSize::FALLBACK.height));
        let height_cells = super::cursor::covered_rows(
            width_cells,
            (pixel_width, pixel_height),
            (cell_pixel_width, cell_pixel_height),
        );

        let png_data = pixels.png()?;
        let tmux = term.graphics_route.transport == GraphicsTransport::Tmux;
        let image = match encoder {
            Encoder::Kitty => {
                if tmux {
                    self.render_kitty_with_id(&png_data, width_cells, height_cells, next_kitty_id())
                } else if matches!(term.app, TerminalApp::Wezterm) {
                    self.render_kitty_cells(&png_data, width_cells, height_cells)
                } else {
                    self.render_kitty_width_only(&png_data, width_cells)
                }
            }
            Encoder::Iterm2 => {
                let filename = Path::new(&self.filename)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "image.png".to_string());
                self.render_iterm2(&png_data, &width_cells.to_string(), &filename)
            }
            Encoder::Fallback(_) => return Err(TerminalImageError::UnsupportedTerminal),
        };

        let prefix = if x_offset > 0 {
            format!("\x1b[{}C", x_offset)
        } else {
            String::new()
        };

        let sequence = format!("\x1b[s{}{}\x1b[u", prefix, image);
        let protocol = if matches!(encoder, Encoder::Kitty) { "kitty" } else { "iterm2" };
        let sequence = for_transport(term, protocol, sequence)
            .map_err(|_| TerminalImageError::UnsupportedTerminal)?;

        Ok((sequence, height_cells))
    }
}

/// `sequence` as the route's transport carries it: unchanged when direct,
/// each graphics control wrapped for tmux passthrough otherwise. A wrapped
/// message over tmux's limit is refused with a size-limit reason; the image
/// is never split across messages.
pub(super) fn for_transport(
    term: &crate::terminal::Terminal,
    protocol: &str,
    sequence: String,
) -> Result<String, RouteReason> {
    if term.graphics_route.transport != GraphicsTransport::Tmux {
        return Ok(sequence);
    }
    let wrapped = wrap_graphics_for_tmux(&sequence);
    if wrapped.largest_message > TMUX_MESSAGE_BUDGET {
        return Err(RouteReason {
            kind: RouteReasonKind::BlockedTransport,
            message: format!(
                "the {protocol} image is {} bytes once wrapped for tmux passthrough, over tmux's {TMUX_MESSAGE_BUDGET}-byte message limit",
                wrapped.largest_message
            ),
            fix: None,
        });
    }
    Ok(wrapped.text)
}

/// Removes control characters so alt text and file names print as literal
/// text and cannot inject terminal controls.
fn strip_controls(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}
