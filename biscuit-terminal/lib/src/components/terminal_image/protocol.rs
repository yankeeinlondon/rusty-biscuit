//! Protocol dispatch — selects the encoder for a terminal's graphics route
//! (or the text fallback) and stitches the result with cursor-advance logic.

use std::path::Path;

use crate::discovery::detection::{
    GraphicsFormat, GraphicsPlacement, GraphicsRoute, GraphicsTransport, RouteConfidence, RouteReason,
    RouteReasonKind, TerminalApp,
};
use crate::discovery::fonts::CellSize;

use crate::discovery::detection::probe::next_kitty_id;

use super::cursor::{compute_cursor_rows, needs_scroll_compensation};
use super::half_block::half_blocks_permitted;
use super::tmux::{TMUX_MESSAGE_BUDGET, wrap_graphics_for_tmux};
use super::{TerminalImage, TerminalImageError};

/// The pixel encoder a route selects, or why no pixels are sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Encoder {
    Kitty,
    Iterm2,
    Sixel,
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
        GraphicsFormat::Sixel => Encoder::Sixel,
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

/// What [`TerminalImage::render_with_route`] or
/// [`TerminalImage::render_with_half_blocks`] produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageOutput {
    /// Escape sequences that draw the image and advance the cursor past it.
    Pixels(String),
    /// No pixels were sent, because the route has no usable pixel format or
    /// refused the image (for example over tmux's message limit); `text`
    /// describes the image.
    Fallback {
        /// The image's alt text with control characters removed.
        text: String,
        /// Why no pixels were sent.
        reason: RouteReason,
    },
    /// No pixels were sent, for the same reasons as
    /// [`Fallback`](Self::Fallback), and the image is drawn as colored text
    /// instead ([`TerminalImage::render_with_half_blocks`]).
    HalfBlocks {
        /// One line per text row, joined with `\n`, no trailing newline.
        text: String,
        /// Why no pixels were sent; the same reason a
        /// [`Fallback`](Self::Fallback) would carry.
        reason: RouteReason,
    },
}

/// An image rendered into a column of a layout ([`TerminalImage::render_inline`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InlineImage {
    /// Escape sequences that draw the image at the cursor and return the
    /// cursor to where it started; the image covers `rows` lines from there.
    Overlay { sequence: String, rows: u32 },
    /// Lines of text that draw the image (Unicode placeholders), one per
    /// image row; the first also carries the upload.
    Lines(Vec<String>),
}

#[cfg(test)]
impl InlineImage {
    /// The overlay's sequence and rows; panics for [`InlineImage::Lines`].
    pub(super) fn into_overlay(self) -> (String, u32) {
        match self {
            Self::Overlay { sequence, rows } => (sequence, rows),
            Self::Lines(lines) => panic!("expected an overlay, got lines: {lines:?}"),
        }
    }
}

impl ImageOutput {
    /// The text to print: the escape sequences or the fallback text.
    #[must_use]
    pub fn into_text(self) -> String {
        match self {
            Self::Pixels(sequence) => sequence,
            Self::Fallback { text, .. } | Self::HalfBlocks { text, .. } => text,
        }
    }
}

impl TerminalImage {
    /// Renders the image for `term`'s graphics route.
    ///
    /// Everything that can fail (loading, decoding, encoding) happens before
    /// any output is produced. A valid image whose route has no usable pixel
    /// format, or whose transport refuses it, becomes
    /// [`ImageOutput::Fallback`] with the reason; never
    /// [`ImageOutput::HalfBlocks`].
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

    /// Renders a plain image for `term`: like
    /// [`render_with_route`](Self::render_with_route), but an image with no
    /// usable pixel format is drawn as half-blocks
    /// ([`ImageOutput::HalfBlocks`]) when the terminal has truecolor and
    /// Unicode, stdout is a terminal, and graphics were not disabled or
    /// misconfigured. Otherwise the result is the alt-text fallback.
    ///
    /// Diagrams use `render_with_route`: their fallback is their source.
    ///
    /// ## Errors
    ///
    /// The same as [`render_with_route`](Self::render_with_route).
    pub fn render_with_half_blocks(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<ImageOutput, TerminalImageError> {
        let output = match encoder_for(term) {
            Encoder::Fallback(reason) => {
                let pixels = self.pixels()?;
                if half_blocks_permitted(term, &reason) {
                    let text = self.render_half_blocks(pixels, term)?.join("\n");
                    return Ok(ImageOutput::HalfBlocks { text, reason });
                }
                ImageOutput::Fallback {
                    text: strip_controls(&self.generate_alt_text()),
                    reason,
                }
            }
            encoder => self.render_to_terminal(term, &encoder)?,
        };
        Ok(match output {
            // A pixel route refused late, such as an image over tmux's
            // message limit.
            ImageOutput::Fallback { reason, .. } if half_blocks_permitted(term, &reason) => {
                let text = self.render_half_blocks(self.pixels()?, term)?.join("\n");
                ImageOutput::HalfBlocks { text, reason }
            }
            output => output,
        })
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
        if matches!(encoder, Encoder::Kitty)
            && term.graphics_route.placement == GraphicsPlacement::Placeholders
            && let Some(sequence) = self.render_kitty_placeholders_for_terminal(term)?
        {
            // The grid is ordinary text that already ends on the line below
            // the image, so no cursor rows are reserved.
            return Ok(match for_transport(term, "kitty", sequence) {
                Ok(sequence) => ImageOutput::Pixels(sequence),
                Err(reason) => ImageOutput::Fallback {
                    text: strip_controls(&self.generate_alt_text()),
                    reason,
                },
            });
        }
        if matches!(encoder, Encoder::Sixel) {
            let image = self.render_sixel_for_terminal(term)?;
            // The rows are already reserved, so the image never scrolls the
            // viewport under the saved cursor and needs no compensation.
            return Ok(match for_transport(term, "sixel", image.sequence) {
                Ok(sequence) => ImageOutput::Pixels(format!("{sequence}\x1b[{}B\r", image.rows)),
                Err(reason) => ImageOutput::Fallback {
                    text: strip_controls(&self.generate_alt_text()),
                    reason,
                },
            });
        }
        let (protocol, (sequence, height_cells, raw_height)) = match encoder {
            Encoder::Kitty => ("kitty", self.render_kitty_for_terminal(term)?),
            Encoder::Iterm2 => ("iterm2", self.render_iterm2_for_terminal(term)?),
            Encoder::Sixel | Encoder::Fallback(_) => unreachable!("handled above or by the caller"),
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

    /// Renders the image into a layout column `term.width()` cells wide.
    ///
    /// When the route has no usable pixel format, or the wrapped image is
    /// over tmux's message limit, the image is drawn as half-block
    /// [`InlineImage::Lines`] where [`TerminalImage::render_with_half_blocks`]
    /// would draw them.
    ///
    /// ## Errors
    ///
    /// [`TerminalImageError::UnsupportedTerminal`] when stdout is not a
    /// terminal or neither pixels nor half-blocks can be drawn; otherwise the
    /// load or encoding error.
    pub(crate) fn render_inline(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<InlineImage, TerminalImageError> {
        if !term.is_tty {
            return Err(TerminalImageError::UnsupportedTerminal);
        }
        let reason = match encoder_for(term) {
            Encoder::Fallback(reason) => reason,
            encoder => match self.render_inline_pixels(term, encoder)? {
                Ok(image) => return Ok(image),
                Err(reason) => reason,
            },
        };
        if !half_blocks_permitted(term, &reason) {
            return Err(TerminalImageError::UnsupportedTerminal);
        }
        Ok(InlineImage::Lines(self.render_half_blocks(self.pixels()?, term)?))
    }

    /// [`render_inline`](Self::render_inline) for a pixel `encoder`, or the
    /// reason the transport refused the image.
    fn render_inline_pixels(
        &self,
        term: &crate::terminal::Terminal,
        encoder: Encoder,
    ) -> Result<Result<InlineImage, RouteReason>, TerminalImageError> {
        let pixels = self.pixels()?;
        let (pixel_width, pixel_height) = pixels.size();
        let cell = term.cell_size();
        let dims = self.resolve_dimensions_for_image(term.width(), cell, pixel_width);
        let width_cells = dims.image_width;
        let x_offset = dims.x_offset;
        if matches!(encoder, Encoder::Sixel) {
            let image = self.render_sixel(pixels, width_cells, x_offset, term)?;
            return Ok(for_transport(term, "sixel", image.sequence).map(|sequence| InlineImage::Overlay {
                sequence,
                rows: image.rows,
            }));
        }

        let (cell_pixel_width, cell_pixel_height) = cell
            .map(|cs| (cs.width.max(1), cs.height.max(1)))
            .unwrap_or((CellSize::FALLBACK.width, CellSize::FALLBACK.height));
        let height_cells = super::cursor::covered_rows(
            width_cells,
            (pixel_width, pixel_height),
            (cell_pixel_width, cell_pixel_height),
        );

        let png_data = pixels.png()?;
        if matches!(encoder, Encoder::Kitty)
            && term.graphics_route.placement == GraphicsPlacement::Placeholders
            && let Some(image) =
                self.render_kitty_placeholders(&png_data, width_cells, height_cells, x_offset)
        {
            let upload = match for_transport(term, "kitty", image.upload) {
                Ok(upload) => upload,
                Err(reason) => return Ok(Err(reason)),
            };
            let mut lines = image.rows;
            if let Some(first) = lines.first_mut() {
                first.insert_str(0, &upload);
            }
            return Ok(Ok(InlineImage::Lines(lines)));
        }
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
            Encoder::Sixel | Encoder::Fallback(_) => unreachable!("handled above or by the caller"),
        };

        let prefix = if x_offset > 0 {
            format!("\x1b[{}C", x_offset)
        } else {
            String::new()
        };

        let sequence = format!("\x1b[s{}{}\x1b[u", prefix, image);
        let protocol = if matches!(encoder, Encoder::Kitty) { "kitty" } else { "iterm2" };
        Ok(for_transport(term, protocol, sequence).map(|sequence| InlineImage::Overlay {
            sequence,
            rows: height_cells,
        }))
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
