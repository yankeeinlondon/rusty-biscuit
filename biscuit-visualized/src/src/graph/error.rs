/// Errors that can occur during graph operations.
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Expression parse error: {0}")]
    ExpressionParseFailed(String),
    #[error("Mixed directed and undirected edges are not supported in the same expression")]
    MixedEdgeKinds,
    #[error("DOT parse error: {0}")]
    DotParseFailed(String),
    #[error("Unsupported DOT feature: {0}")]
    UnsupportedDotFeature(String),
    #[error("Graph rendering failed: {0}")]
    RenderFailed(String),
    #[error("SVG rasterization failed: {0}")]
    RasterizationFailed(#[from] crate::raster::RasterError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
