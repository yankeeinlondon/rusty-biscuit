use std::fs;
use std::io;
use std::path::Path;

use resvg::usvg;

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
    // Read SVG file
    let svg_data = fs::read_to_string(svg_path)?;

    // Parse SVG with default options
    let opts = usvg::Options::default();
    let tree = usvg::Tree::from_str(&svg_data, &opts)
        .map_err(|e| RasterError::SvgParseFailed(e.to_string()))?;

    // Calculate output dimensions
    let size = tree.size();
    let width = (size.width() * scale as f32) as u32;
    let height = (size.height() * scale as f32) as u32;

    // Create pixmap for rendering
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| RasterError::RenderFailed("Failed to create pixmap".to_string()))?;

    // Create transform for scaling
    let transform = usvg::Transform::from_scale(scale as f32, scale as f32);

    // Render SVG to pixmap
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Save as PNG
    pixmap
        .save_png(png_path)
        .map_err(|e| RasterError::IoError(io::Error::other(e.to_string())))?;

    Ok(())
}
