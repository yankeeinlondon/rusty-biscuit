//! Sixel rendering paths for [`TerminalImage`].
//!
//! Sixel has no partial transparency. A fully transparent pixel is left
//! unset, so the terminal's own background shows through it; a partially
//! transparent one is blended onto the best-known background
//! ([`SixelBackground`]) and drawn opaque. Quantization is
//! deterministic: Wu's method, a 256-color palette, and no dithering, so a
//! fixed image always encodes to the same bytes and crisp diagrams gain no
//! dithering noise.
//!
//! The developer guide is `biscuit-terminal/docs/components/terminal_image.md`.

use icy_sixel::{BackgroundMode, EncodeOptions, PixelAspectRatio, QuantizeMethod, SixelImage};
use image::DynamicImage;

use crate::discovery::detection::ColorMode;
use crate::discovery::fonts::CellSize;
use crate::discovery::osc_queries::{ColorSource, RgbValue};
use crate::terminal::Terminal;

use super::encoded_png::ImagePixels;
use super::{TerminalImage, TerminalImageError};

/// The most colors a Sixel image's palette holds.
pub const SIXEL_PALETTE_COLORS: u16 = 256;

/// The best-known terminal background, and whether the terminal reported
/// it.
///
/// Sixel and half-block images blend their partially transparent pixels onto
/// `color`; fully transparent pixels are never painted with it. A
/// `Measured` color is exact; an `Estimated` one can differ from the
/// terminal's real background, so only antialiased edges can show it.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::terminal_image::{BackgroundSource, SixelBackground};
/// use biscuit_terminal::discovery::detection::ColorMode;
/// use biscuit_terminal::terminal::Terminal;
///
/// let term = Terminal {
///     background_color: None,
///     color_mode: ColorMode::Light,
///     ..Terminal::default()
/// };
/// let background = SixelBackground::for_terminal(&term);
/// assert_eq!(background.source, BackgroundSource::Estimated);
/// assert_eq!((background.color.r, background.color.g, background.color.b), (255, 255, 255));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SixelBackground {
    pub color: RgbValue,
    pub source: BackgroundSource,
}

/// Whether a [`SixelBackground`] was measured or estimated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundSource {
    /// The terminal answered the background-color query (OSC 11):
    /// [`Terminal::background_source`] is [`ColorSource::Measured`].
    Measured,
    /// The terminal reported no color: its `COLORFGBG` or per-app default
    /// when it has one, else white for a light color mode and black for a
    /// dark or unknown one.
    Estimated,
}

impl SixelBackground {
    /// The terminal's measured background; otherwise its estimated
    /// background, or the light/dark estimate when it has none.
    #[must_use]
    pub fn for_terminal(term: &Terminal) -> Self {
        if let Some(color) = term.background_color {
            let source = match term.background_source {
                ColorSource::Measured => BackgroundSource::Measured,
                ColorSource::Estimated => BackgroundSource::Estimated,
            };
            return Self { color, source };
        }
        let level = if term.color_mode == ColorMode::Light { 255 } else { 0 };
        Self {
            color: RgbValue {
                r: level,
                g: level,
                b: level,
            },
            source: BackgroundSource::Estimated,
        }
    }
}

/// A Sixel image and the rows it covers.
pub(super) struct SixelImageOutput {
    /// Reserves the rows, then draws the image with the cursor saved and
    /// restored, so the cursor ends where it started.
    pub sequence: String,
    pub rows: u32,
}

impl TerminalImage {
    /// The image as Sixel for `term`, `columns` wide and offset `x_offset`
    /// columns.
    ///
    /// The rows are reserved with index (`ESC D`) before the image is drawn:
    /// at the bottom of the viewport the terminal scrolls then, never under a
    /// saved cursor. No private mode is set, so Sixel scrolling and palette
    /// modes stay as the terminal had them.
    pub(super) fn render_sixel(
        &self,
        pixels: ImagePixels<'_>,
        columns: u32,
        x_offset: u32,
        term: &Terminal,
    ) -> Result<SixelImageOutput, TerminalImageError> {
        let (pixel_width, pixel_height) = pixels.size();
        let cell = term
            .cell_size()
            .map(|cs| (cs.width.max(1), cs.height.max(1)))
            .unwrap_or((CellSize::FALLBACK.width, CellSize::FALLBACK.height));
        let rows = super::cursor::covered_rows(columns, (pixel_width, pixel_height), cell);
        let width = columns.max(1) * cell.0;
        let height = scaled_height(width, (pixel_width, pixel_height));
        let background = SixelBackground::for_terminal(term);
        tracing::debug!(
            file = %self.relative,
            width,
            height,
            rows,
            background = ?background,
            "Sixel image render"
        );
        let sixel = encode_sixel(&pixels.into_image()?, (width, height), rows * cell.1, background.color)?;
        let prefix = if x_offset > 0 {
            format!("\x1b[{x_offset}C")
        } else {
            String::new()
        };
        let sequence = format!("{}\x1b[{rows}A\x1b[s{prefix}{sixel}\x1b[u", "\x1bD".repeat(rows as usize));
        Ok(SixelImageOutput { sequence, rows })
    }

    /// [`render_sixel`](Self::render_sixel) at the width `term` resolves
    /// for this image.
    pub(super) fn render_sixel_for_terminal(&self, term: &Terminal) -> Result<SixelImageOutput, TerminalImageError> {
        let pixels = self.pixels()?;
        let dims = self.resolve_dimensions_for_image(term.width().max(1), term.cell_size(), pixels.size().0);
        self.render_sixel(pixels, dims.image_width, dims.x_offset, term)
    }
}

/// `width` scaled by the image's aspect ratio, rounded, at least 1.
fn scaled_height(width: u32, (pixel_width, pixel_height): (u32, u32)) -> u32 {
    let pixel_width = u64::from(pixel_width.max(1));
    let height = (u64::from(width) * u64::from(pixel_height) + pixel_width / 2) / pixel_width;
    u32::try_from(height).unwrap_or(u32::MAX).max(1)
}

/// Encodes `image` resized to `size` and padded below to `padded_height`
/// pixels (whole cell rows), as one Sixel DCS sequence.
///
/// The DCS declares transparent background mode (`P2=1`): a pixel with no
/// sixel bit in any color keeps whatever the terminal shows there. Fully
/// transparent pixels and the padding get no bits; partially transparent
/// pixels are blended onto `background` and drawn opaque.
pub(super) fn encode_sixel(
    image: &DynamicImage,
    (width, height): (u32, u32),
    padded_height: u32,
    background: RgbValue,
) -> Result<String, TerminalImageError> {
    let resized;
    let image = if (image.width(), image.height()) == (width, height) {
        image
    } else {
        resized = image.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
        &resized
    };
    let padded_height = padded_height.max(height);
    let source = image.to_rgba8();
    let mut canvas = Vec::with_capacity(width as usize * padded_height as usize * 4);
    // icy_sixel sets no bits for alpha below 128; every pixel it receives is
    // either fully transparent or opaque, so only alpha 0 is left unset.
    const UNSET: [u8; 4] = [0, 0, 0, 0];
    for y in 0..padded_height {
        for x in 0..width {
            if y >= height {
                canvas.extend_from_slice(&UNSET);
                continue;
            }
            match source.get_pixel(x, y).0 {
                [_, _, _, 0] => canvas.extend_from_slice(&UNSET),
                [r, g, b, a] => canvas.extend_from_slice(&[
                    blend(r, background.r, a),
                    blend(g, background.g, a),
                    blend(b, background.b, a),
                    255,
                ]),
            }
        }
    }
    let options = EncodeOptions {
        max_colors: SIXEL_PALETTE_COLORS,
        diffusion: 0.0,
        quantize_method: QuantizeMethod::Wu,
    };
    SixelImage::try_from_rgba(canvas, width as usize, padded_height as usize)
        .map(|sixel| {
            sixel
                .with_aspect_ratio(PixelAspectRatio::Square)
                .with_background_mode(BackgroundMode::Transparent)
        })
        .and_then(|sixel| sixel.encode_with(&options))
        .map_err(|e| TerminalImageError::EncodingError {
            message: format!("Sixel encoding failed: {e}"),
        })
}

/// `over` composited onto `under` at `alpha`, rounded.
pub(super) fn blend(over: u8, under: u8, alpha: u8) -> u8 {
    let alpha = u32::from(alpha);
    ((u32::from(over) * alpha + u32::from(under) * (255 - alpha) + 127) / 255) as u8
}

#[cfg(test)]
mod tests;
