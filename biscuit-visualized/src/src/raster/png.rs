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

fn render_svg_to_pixmap(
    svg_data: &str,
    scale: u32,
) -> Result<resvg::tiny_skia::Pixmap, RasterError> {
    let mut opts = usvg::Options::default();
    *opts.fontdb_mut() = cloned_system_font_database();
    let tree = usvg::Tree::from_str(svg_data, &opts)
        .map_err(|e| RasterError::SvgParseFailed(e.to_string()))?;

    let scale = scale.max(1);
    let size = tree.size();
    let width = ((size.width() * scale as f32).round() as u32).max(1);
    let height = ((size.height() * scale as f32).round() as u32).max(1);

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| RasterError::RenderFailed("Failed to create pixmap".to_string()))?;

    let transform = usvg::Transform::from_scale(scale as f32, scale as f32);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Ok(pixmap)
}

/// Rasterizes SVG content to PNG bytes.
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
