//! Sizing a rasterized diagram to the cells it is displayed in.
//!
//! A diagram is rasterized once, at its final display size: exactly as many
//! pixels wide as its columns, and padded to whole rows rather than stretched
//! into them. The PNG then reaches the terminal's encoder unchanged.

use std::path::Path;

use biscuit_terminal::components::terminal_image::{
    BackgroundSource, EncodedPng, ImageWidth, SixelBackground, TerminalImage, TerminalImageError,
};
use biscuit_terminal::discovery::detection::GraphicsFormat;
use biscuit_terminal::discovery::fonts::CellSize;
use biscuit_terminal::terminal::Terminal;

use crate::artifact::Background;

/// The background to rasterize a diagram on for `term`'s graphics route.
///
/// Sixel has no transparency, so a [`Transparent`](Background::Transparent)
/// diagram is drawn on the terminal's measured background instead, or on the
/// theme's own opaque background when the terminal reported none. Every
/// other route and every other background is kept as requested. The
/// artifact cache keys on the background, so the two never share a PNG.
pub(crate) fn display_background(requested: Background, term: &Terminal) -> Background {
    if requested != Background::Transparent || term.graphics_route.format != GraphicsFormat::Sixel {
        return requested;
    }
    let background = SixelBackground::for_terminal(term);
    match background.source {
        BackgroundSource::Measured => Background::Solid {
            r: background.color.r,
            g: background.color.g,
            b: background.color.b,
        },
        BackgroundSource::Estimated => Background::Theme,
    }
}

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
    use biscuit_terminal::discovery::detection::{GraphicsRoute, ImageSupport};
    use biscuit_terminal::discovery::osc_queries::RgbValue;

    fn terminal(support: ImageSupport, background: Option<RgbValue>) -> Terminal {
        Terminal {
            graphics_route: GraphicsRoute::from(support),
            background_color: background,
            ..Terminal::default()
        }
    }

    const MEASURED: RgbValue = RgbValue { r: 1, g: 2, b: 3 };

    #[test]
    fn sixel_draws_a_transparent_diagram_on_the_measured_background() {
        let term = terminal(ImageSupport::Sixel, Some(MEASURED));
        assert_eq!(
            display_background(Background::Transparent, &term),
            Background::Solid { r: 1, g: 2, b: 3 }
        );
    }

    #[test]
    fn sixel_without_a_measured_background_uses_the_theme_background() {
        let term = terminal(ImageSupport::Sixel, None);
        assert_eq!(display_background(Background::Transparent, &term), Background::Theme);
    }

    #[test]
    fn other_routes_and_explicit_backgrounds_are_kept() {
        for support in [ImageSupport::Kitty, ImageSupport::ITerm, ImageSupport::None] {
            let term = terminal(support, Some(MEASURED));
            assert_eq!(display_background(Background::Transparent, &term), Background::Transparent);
        }
        let sixel = terminal(ImageSupport::Sixel, Some(MEASURED));
        let solid = Background::Solid { r: 9, g: 9, b: 9 };
        assert_eq!(display_background(Background::Theme, &sixel), Background::Theme);
        assert_eq!(display_background(solid, &sixel), solid);
    }

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
