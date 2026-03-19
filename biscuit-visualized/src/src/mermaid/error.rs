/// Errors that can occur during Mermaid diagram rendering.
#[derive(Debug, thiserror::Error)]
pub enum MermaidError {
    /// Mermaid parse/render failure.
    #[error("Mermaid parse/render failure: {0}")]
    RenderFailed(String),
    /// SVG rasterization failed.
    #[error("SVG rasterization failed: {0}")]
    RasterizationFailed(#[from] crate::raster::RasterError),
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
