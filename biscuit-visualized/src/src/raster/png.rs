use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;

use resvg::usvg;

use super::usvg_options;
use crate::artifact::{Background, RenderRequest};
use crate::fonts::{FontChain, default_chain};

/// Errors that can occur during SVG rasterization.
#[derive(Debug, thiserror::Error)]
pub enum RasterError {
    #[error("SVG parse failed: {0}")]
    SvgParseFailed(String),
    #[error("Render failed: {0}")]
    RenderFailed(String),
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
}

fn parse_svg_tree(svg_data: &str) -> Result<usvg::Tree, RasterError> {
    parse_svg_tree_with(svg_data, default_chain())
}

/// Parses `svg_data`, laying out its text with `chain`'s faces.
fn parse_svg_tree_with(svg_data: &str, chain: &Arc<FontChain>) -> Result<usvg::Tree, RasterError> {
    let opts = usvg_options(chain);
    usvg::Tree::from_str(svg_data, &opts).map_err(|e| RasterError::SvgParseFailed(e.to_string()))
}

/// [`rasterize_svg_to_png_bytes`] with an explicit font chain (tests inject
/// their own font set).
#[cfg(test)]
pub(crate) fn rasterize_svg_to_png_bytes_with(
    svg_data: &str,
    scale: u32,
    chain: &Arc<FontChain>,
) -> Result<Vec<u8>, RasterError> {
    let tree = parse_svg_tree_with(svg_data, chain)?;
    let pixmap = render_tree_to_pixmap(&tree, scale.max(1) as f32, None, None)?;
    encode_png(pixmap)
}

/// Draws `tree` scaled by `scale` onto a canvas whose height is padded up to
/// whole rows of `row_height` pixels; the padding below the drawing is filled
/// with `padding_fill`, or left transparent.
fn render_tree_to_pixmap(
    tree: &usvg::Tree,
    scale: f32,
    row_height: Option<u32>,
    padding_fill: Option<resvg::tiny_skia::Color>,
) -> Result<resvg::tiny_skia::Pixmap, RasterError> {
    let size = tree.size();
    let width = ((size.width() * scale).round() as u32).max(1);
    let content_height = ((size.height() * scale).round() as u32).max(1);
    let height = match row_height {
        Some(row) => content_height.div_ceil(row.max(1)) * row.max(1),
        None => content_height,
    };

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| RasterError::RenderFailed("Failed to create pixmap".to_string()))?;
    if let Some(color) = padding_fill {
        pixmap.fill(color);
    }

    let transform = usvg::Transform::from_scale(scale, scale);
    resvg::render(tree, transform, &mut pixmap.as_mut());

    Ok(pixmap)
}

/// Uniform scale that makes `tree` exactly `target_width` pixels wide.
fn scale_for_width(tree: &usvg::Tree, target_width: u32) -> Result<f32, RasterError> {
    let svg_width = tree.size().width();
    if svg_width <= 0.0 {
        return Err(RasterError::RenderFailed(
            "SVG reports zero or negative width".to_string(),
        ));
    }
    Ok((target_width.max(1) as f32) / svg_width)
}

/// Encodes a pixmap as PNG with zlib level 1 and the `Up` filter.
///
/// The settings are spelled out rather than inherited from tiny-skia's
/// `encode_png`, because they are part of the cache key (`PNG_ENCODING`):
/// a change in tiny-skia's defaults must not change the bytes behind a key.
/// They are chosen for the 50 ms warm render limit: on a large diagram,
/// tiny-skia's zlib default level with adaptive filtering took most of it,
/// and level 1 with `Up` is about five times faster for files about twice as
/// large. fdeflate's faster modes are not used: their output can exceed the
/// raw pixels, and these bytes reach terminals unchanged.
fn encode_png(mut pixmap: resvg::tiny_skia::Pixmap) -> Result<Vec<u8>, RasterError> {
    let (width, height) = (pixmap.width(), pixmap.height());
    // tiny-skia stores premultiplied alpha; PNG wants straight alpha, written
    // over the pixmap in place. Opaque pixels are already straight, and a
    // transparent one is all zeros either way.
    for pixel in pixmap.data_mut().as_chunks_mut::<4>().0 {
        let [red, green, blue, alpha] = *pixel;
        if !matches!(alpha, 0 | 255)
            && let Some(color) = resvg::tiny_skia::PremultipliedColorU8::from_rgba(red, green, blue, alpha)
        {
            let color = color.demultiply();
            *pixel = [color.red(), color.green(), color.blue(), color.alpha()];
        }
    }
    let rgba = pixmap.data();

    let encoding_error = |e: png::EncodingError| RasterError::IoError(io::Error::other(e.to_string()));
    let mut data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut data, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_deflate_compression(png::DeflateCompression::Level(1));
        encoder.set_filter(png::Filter::Up);
        let mut writer = encoder.write_header().map_err(encoding_error)?;
        writer.write_image_data(rgba).map_err(encoding_error)?;
        writer.finish().map_err(encoding_error)?;
    }
    Ok(data)
}

/// Rasterizes SVG content to PNG bytes as a [`RenderRequest`] asks: at its
/// target width (or scale), padded to its row height, with padding painted for
/// a [`Background::Solid`] and transparent otherwise.
///
/// Draws text with the process's default chain.
#[cfg(test)]
pub(crate) fn rasterize_svg_for_request(
    svg_data: &str,
    request: &RenderRequest,
) -> Result<Vec<u8>, RasterError> {
    rasterize_svg_for_request_with(svg_data, request, default_chain())
}

/// [`rasterize_svg_for_request`], drawing text with `chain`'s faces: the
/// chain whose identity is in the artifact's cache key (and, for Mermaid,
/// the one its labels were measured with).
///
/// The renderers' one path from SVG to PNG, so geometry and padding agree with
/// the request's cache identity.
pub(crate) fn rasterize_svg_for_request_with(
    svg_data: &str,
    request: &RenderRequest,
    chain: &Arc<FontChain>,
) -> Result<Vec<u8>, RasterError> {
    let tree = parse_svg_tree_with(svg_data, chain)?;
    let scale = match request.target_width {
        Some(width) => scale_for_width(&tree, width)?,
        None => request.scale.max(1) as f32,
    };
    let padding_fill = match request.background {
        Background::Solid { r, g, b } => Some(resvg::tiny_skia::Color::from_rgba8(r, g, b, 255)),
        Background::Theme | Background::Transparent => None,
    };
    let pixmap = render_tree_to_pixmap(&tree, scale, request.row_height, padding_fill)?;
    encode_png(pixmap)
}

/// Rasterizes SVG content to PNG bytes at the SVG's native dimensions
/// multiplied by `scale`.
///
/// Useful when you want a fixed HiDPI multiplier (e.g. `scale = 2` for Retina
/// displays). When you instead know the *target pixel width*,
/// [`rasterize_svg_to_png`] is the more direct API — it takes the width
/// in pixels and computes the necessary scale internally, preserving the
/// SVG's aspect ratio.
///
/// ## Errors
///
/// Returns an error if the SVG cannot be parsed or PNG encoding fails.
pub fn rasterize_svg_to_png_bytes(svg_data: &str, scale: u32) -> Result<Vec<u8>, RasterError> {
    let tree = parse_svg_tree(svg_data)?;
    let pixmap = render_tree_to_pixmap(&tree, scale.max(1) as f32, None, None)?;
    encode_png(pixmap)
}

/// Rasterizes SVG content to PNG bytes at a target *pixel width*.
///
/// The output PNG is exactly `width` pixels wide; height is derived from the
/// SVG's aspect ratio so the result is geometrically faithful. Internally
/// `resvg` is given a fractional uniform scale — there is no intermediate
/// PNG that gets resampled, so the rasterized text and shapes are rendered
/// fresh at the target resolution.
///
/// Prefer this over [`rasterize_svg_to_png_bytes`] when you know the display
/// size you want (e.g. a terminal cell area, a layout container, an image
/// thumbnail). It avoids the "pick an arbitrary `scale` factor" guessing
/// game and produces output sized for the display rather than for an
/// abstract DPI multiplier.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_visualized::raster::rasterize_svg_to_png;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let svg = r#"<svg width="100" height="50" xmlns="http://www.w3.org/2000/svg">
///     <rect width="100" height="50" fill="red"/>
/// </svg>"#;
///
/// // PNG will be 800 pixels wide; height 400 (aspect ratio preserved)
/// let png = rasterize_svg_to_png(svg, 800)?;
/// # Ok(())
/// # }
/// ```
///
/// ## Errors
///
/// Returns an error if the SVG cannot be parsed, reports a zero or negative
/// native width, or if PNG encoding fails.
pub fn rasterize_svg_to_png(svg_data: &str, width: u32) -> Result<Vec<u8>, RasterError> {
    let tree = parse_svg_tree(svg_data)?;
    let pixmap = render_tree_to_pixmap(&tree, scale_for_width(&tree, width)?, None, None)?;
    encode_png(pixmap)
}

/// Rasterizes an SVG file to PNG format.
///
/// Reads an SVG file, parses it, and renders it to a PNG file at the specified
/// scale factor. The output dimensions are the SVG's natural dimensions multiplied
/// by the scale factor.
///
/// ## Arguments
///
/// * `svg_path` - Path to the input SVG file
/// * `png_path` - Path where the output PNG will be written
/// * `scale` - Scale factor for rendering (2 = 2x resolution)
///
/// ## Examples
///
/// ```rust,no_run
/// use std::path::Path;
/// use biscuit_visualized::raster::rasterize_svg;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// rasterize_svg(
///     Path::new("diagram.svg"),
///     Path::new("diagram.png"),
///     2, // 2x scale for high-DPI displays
/// )?;
/// # Ok(())
/// # }
/// ```
///
/// ## Errors
///
/// Returns an error if:
/// - The SVG file cannot be read
/// - The SVG content is invalid or cannot be parsed
/// - The rendering operation fails
/// - The PNG file cannot be written
pub fn rasterize_svg(svg_path: &Path, png_path: &Path, scale: u32) -> Result<(), RasterError> {
    let svg_data = fs::read_to_string(svg_path)?;
    let png_data = rasterize_svg_to_png_bytes(&svg_data, scale)?;
    fs::write(png_path, png_data)?;

    Ok(())
}
