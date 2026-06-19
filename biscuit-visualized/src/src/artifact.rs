use std::path::PathBuf;

/// Output format for rendered visualizations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Svg,
    Png,
}

impl OutputFormat {
    /// Returns the file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
        }
    }
}

/// Parameters for rendering a visualization.
///
/// When [`target_width`](Self::target_width) is `Some`, the rasterizer renders the
/// SVG directly at that pixel width (aspect-ratio preserved) and the
/// [`scale`](Self::scale) field is ignored. This is the preferred path when the
/// caller knows the display target — e.g. a terminal cell area, a thumbnail
/// bucket — because it avoids the "pick a scale and hope it fits" pattern and
/// produces text rendered fresh at the display resolution instead of resampled
/// from an oversized bitmap.
///
/// When [`target_width`](Self::target_width) is `None`, the rasterizer falls back
/// to the legacy scale-based path: PNG dimensions are `svg_native × scale`.
#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub format: OutputFormat,
    /// HiDPI multiplier applied to the SVG's native dimensions. Used only
    /// when `target_width` is `None`.
    pub scale: u32,
    /// Explicit target width in pixels. When `Some`, takes precedence over
    /// `scale` and height is derived from the SVG's aspect ratio.
    pub target_width: Option<u32>,
    pub transparent_background: bool,
}

impl Default for RenderRequest {
    fn default() -> Self {
        Self {
            format: OutputFormat::Png,
            scale: 2,
            target_width: None,
            transparent_background: false,
        }
    }
}

impl RenderRequest {
    /// Set the target pixel width. The PNG will be rendered at exactly this
    /// width; height is derived from the SVG's aspect ratio.
    pub fn with_target_width(mut self, width: u32) -> Self {
        self.target_width = Some(width);
        self
    }
}

/// Result of a successful render operation.
#[derive(Debug, Clone)]
pub struct RenderedArtifact {
    pub path: PathBuf,
    pub format: OutputFormat,
    pub cache_hit: bool,
    pub alt_text: String,
}
