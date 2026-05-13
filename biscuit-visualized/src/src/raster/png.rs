use std::fs;
use std::io;
use std::path::Path;

use resvg::usvg;

use super::cloned_system_font_database;

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
    let mut opts = usvg::Options::default();
    *opts.fontdb_mut() = cloned_system_font_database();
    usvg::Tree::from_str(svg_data, &opts).map_err(|e| RasterError::SvgParseFailed(e.to_string()))
}

fn render_tree_to_pixmap(
    tree: &usvg::Tree,
    scale_x: f32,
    scale_y: f32,
) -> Result<resvg::tiny_skia::Pixmap, RasterError> {
    let size = tree.size();
    let width = ((size.width() * scale_x).round() as u32).max(1);
    let height = ((size.height() * scale_y).round() as u32).max(1);

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| RasterError::RenderFailed("Failed to create pixmap".to_string()))?;

    let transform = usvg::Transform::from_scale(scale_x, scale_y);
    resvg::render(tree, transform, &mut pixmap.as_mut());

    Ok(pixmap)
}

fn render_svg_to_pixmap(
    svg_data: &str,
    scale: u32,
) -> Result<resvg::tiny_skia::Pixmap, RasterError> {
    let tree = parse_svg_tree(svg_data)?;
    let scale = scale.max(1) as f32;
    render_tree_to_pixmap(&tree, scale, scale)
}

fn render_svg_to_pixmap_at_width(
    svg_data: &str,
    target_width: u32,
) -> Result<resvg::tiny_skia::Pixmap, RasterError> {
    let tree = parse_svg_tree(svg_data)?;
    let svg_width = tree.size().width();
    if svg_width <= 0.0 {
        return Err(RasterError::RenderFailed(
            "SVG reports zero or negative width".to_string(),
        ));
    }
    // Uniform scale so the rasterized PNG preserves the SVG's aspect ratio
    // exactly. Callers who need non-uniform scaling can compute it themselves
    // and use the scale-based API.
    let scale = (target_width.max(1) as f32) / svg_width;
    render_tree_to_pixmap(&tree, scale, scale)
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
    let pixmap = render_svg_to_pixmap(svg_data, scale)?;
    pixmap
        .encode_png()
        .map_err(|e| RasterError::IoError(io::Error::other(e.to_string())))
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
    let pixmap = render_svg_to_pixmap_at_width(svg_data, width)?;
    pixmap
        .encode_png()
        .map_err(|e| RasterError::IoError(io::Error::other(e.to_string())))
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
