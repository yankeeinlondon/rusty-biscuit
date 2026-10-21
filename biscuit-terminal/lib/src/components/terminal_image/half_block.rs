//! Half-block rendering: a plain image with no usable pixel route drawn as
//! text, two image pixels per cell (`▀` with a truecolor foreground for the
//! upper pixel and background for the lower one).
//!
//! Half-blocks are a text fallback, not an image protocol: they never count
//! as image support on the route, and diagrams never use them.
//!
//! The developer guide is `biscuit-terminal/docs/components/terminal_image.md`.

use image::DynamicImage;

use crate::discovery::detection::{ColorDepth, RouteReason, RouteReasonKind};
use crate::discovery::fonts::CellSize;
use crate::discovery::osc_queries::RgbValue;
use crate::terminal::Terminal;

use super::encoded_png::ImagePixels;
use super::sixel::{SixelBackground, blend};
use super::{TerminalImage, TerminalImageError};

/// The glyph whose foreground is a cell's upper pixel.
const UPPER_HALF_BLOCK: char = '▀';

/// Whether `term` may draw an image whose pixel route failed for `reason`
/// as half-blocks.
///
/// Needs a terminal on stdout with truecolor and Unicode. Never when the
/// user disabled graphics (`BISCUIT_GRAPHICS=none`), an override variable is
/// invalid (a configuration error is not guessed around), or stdout is not
/// a terminal, whatever the color settings claim.
pub(super) fn half_blocks_permitted(term: &Terminal, reason: &RouteReason) -> bool {
    term.is_tty
        && term.color_depth == ColorDepth::TrueColor
        && term.supports_unicode
        && !matches!(
            reason.kind,
            RouteReasonKind::UserDisabled | RouteReasonKind::InvalidInput | RouteReasonKind::NotATerminal
        )
}

impl TerminalImage {
    /// The image as half-block lines for `term`, one per text row, each
    /// indented by the layout's offset and ending with an SGR reset.
    pub(super) fn render_half_blocks(
        &self,
        pixels: ImagePixels<'_>,
        term: &Terminal,
    ) -> Result<Vec<String>, TerminalImageError> {
        let (pixel_width, pixel_height) = pixels.size();
        let dims = self.resolve_dimensions_for_image(term.width().max(1), term.cell_size(), pixel_width);
        let cell = term.cell_size().unwrap_or(CellSize::FALLBACK);
        let rows = half_block_pixel_rows(dims.image_width, (pixel_width, pixel_height), cell);
        let background = SixelBackground::for_terminal(term);
        tracing::debug!(
            file = %self.relative,
            columns = dims.image_width,
            pixel_rows = rows,
            background = ?background,
            "Half-block image render"
        );
        let indent = " ".repeat(dims.x_offset as usize);
        Ok(encode_half_blocks(&pixels.into_image()?, (dims.image_width, rows), background.color)
            .into_iter()
            .map(|line| format!("{indent}{line}"))
            .collect())
    }
}

/// How many image pixel rows `columns` cells show, keeping the aspect ratio:
/// a half-block pixel is one cell wide and half a cell tall. At least 1.
fn half_block_pixel_rows(columns: u32, (pixel_width, pixel_height): (u32, u32), cell: CellSize) -> u32 {
    let numerator = 2 * u64::from(columns.max(1)) * u64::from(cell.width.max(1)) * u64::from(pixel_height);
    let denominator = u64::from(pixel_width.max(1)) * u64::from(cell.height.max(1));
    let rows = (numerator + denominator / 2) / denominator;
    u32::try_from(rows).unwrap_or(u32::MAX).max(1)
}

/// `image` resized to `columns` x `pixel_rows`, composited onto
/// `background`, as half-block lines. An odd last pixel row gets
/// `background` as its lower half. A color is written only when it changes
/// from the previous cell.
pub(super) fn encode_half_blocks(
    image: &DynamicImage,
    (columns, pixel_rows): (u32, u32),
    background: RgbValue,
) -> Vec<String> {
    let resized;
    let image = if (image.width(), image.height()) == (columns, pixel_rows) {
        image
    } else {
        resized = image.resize_exact(columns, pixel_rows, image::imageops::FilterType::Triangle);
        &resized
    };
    let source = image.to_rgba8();
    let pixel = |x: u32, y: u32| -> (u8, u8, u8) {
        if y >= pixel_rows {
            return (background.r, background.g, background.b);
        }
        let [r, g, b, a] = source.get_pixel(x, y).0;
        (blend(r, background.r, a), blend(g, background.g, a), blend(b, background.b, a))
    };

    (0..pixel_rows.div_ceil(2))
        .map(|row| {
            let mut line = String::new();
            let mut previous = None;
            for x in 0..columns {
                let colors = (pixel(x, row * 2), pixel(x, row * 2 + 1));
                if previous != Some(colors) {
                    let ((fr, fg, fb), (br, bg, bb)) = colors;
                    line.push_str(&format!("\x1b[38;2;{fr};{fg};{fb};48;2;{br};{bg};{bb}m"));
                    previous = Some(colors);
                }
                line.push(UPPER_HALF_BLOCK);
            }
            line.push_str("\x1b[0m");
            line
        })
        .collect()
}

#[cfg(test)]
mod tests;
