//! Sizing a rasterized diagram to the cells it is displayed in.
//!
//! A diagram is rasterized once, at its final display size: exactly as many
//! pixels wide as its columns, and padded to whole rows rather than stretched
//! into them. The PNG then reaches the terminal's encoder unchanged.

use std::path::Path;

use biscuit_terminal::components::terminal_image::{
    EncodedPng, ImageWidth, TerminalImage, TerminalImageError,
};
use biscuit_terminal::discovery::fonts::CellSize;

/// The raster width and row height, in pixels, of `columns` cells.
///
/// Uses the measured cell size, or [`CellSize::FALLBACK`] when the terminal
/// reports none; neither dimension is ever zero. Callers measure on every
/// render, so a resized terminal or font gets a new raster.
pub(crate) fn display_pixels(columns: u32, cell: Option<CellSize>) -> (u32, u32) {
    let cell = cell.unwrap_or(CellSize::FALLBACK);
    (columns.max(1) * cell.width.max(1), cell.height.max(1))
}

/// The PNG at `png_path`, displayed `columns` wide without being decoded or
/// encoded again.
///
/// `png_path` is a rasterizer cache artifact, never a caller-supplied file.
pub(crate) fn display_image(png_path: &Path, columns: u32) -> Result<TerminalImage, TerminalImageError> {
    let png = EncodedPng::new(std::fs::read(png_path)?)?;
    Ok(TerminalImage::from_encoded_png(png).with_width(ImageWidth::Characters(columns)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_become_pixels_through_the_measured_cell() {
        let cell = CellSize { width: 9, height: 18 };
        assert_eq!(display_pixels(30, Some(cell)), (270, 18));
    }

    #[test]
    fn an_unknown_or_zero_cell_never_gives_zero_pixels() {
        assert_eq!(display_pixels(30, None), (30 * CellSize::FALLBACK.width, CellSize::FALLBACK.height));
        assert_eq!(display_pixels(0, Some(CellSize { width: 0, height: 0 })), (1, 1));
    }
}
