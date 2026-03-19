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
#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub format: OutputFormat,
    pub scale: u32,
    pub transparent_background: bool,
}

impl Default for RenderRequest {
    fn default() -> Self {
        Self {
            format: OutputFormat::Png,
            scale: 2,
            transparent_background: false,
        }
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
