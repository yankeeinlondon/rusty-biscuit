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

/// What paints the area behind a diagram.
///
/// Applies to SVG and PNG output alike. [`Theme`](Self::Theme) is the default
/// and keeps exported artifacts as they have always looked; terminal adapters
/// choose [`Transparent`](Self::Transparent) for Kitty and iTerm2, whose
/// images composite over the terminal, and [`Solid`](Self::Solid) for
/// protocols without alpha.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Background {
    /// The theme's own background color.
    #[default]
    Theme,
    /// No background: unpainted pixels stay fully transparent.
    Transparent,
    /// One opaque color in place of the theme's.
    Solid { r: u8, g: u8, b: u8 },
}

impl Background {
    /// The color as CSS hex (`#rrggbb`), for [`Solid`](Self::Solid) only.
    pub fn solid_hex(&self) -> Option<String> {
        match self {
            Self::Solid { r, g, b } => Some(format!("#{r:02x}{g:02x}{b:02x}")),
            Self::Theme | Self::Transparent => None,
        }
    }

    /// The stable name this background takes in artifact cache keys.
    fn cache_name(&self) -> String {
        match self {
            Self::Theme => "theme".to_string(),
            Self::Transparent => "transparent".to_string(),
            Self::Solid { .. } => self.solid_hex().unwrap_or_default(),
        }
    }
}

/// How PNG artifacts are compressed; part of every PNG artifact's cache key,
/// so changing it never serves bytes encoded the old way.
pub(crate) const PNG_ENCODING: &str = "png@0.17:compression=default,filter=adaptive";

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
    /// Pads a PNG's height up to a whole number of rows this many pixels tall,
    /// so the image fills the terminal cell rectangle it reserves instead of
    /// being stretched into it. The padding sits below the drawing and is
    /// transparent, or the [`Solid`](Background::Solid) color. Ignored for SVG.
    pub row_height: Option<u32>,
    pub background: Background,
}

impl Default for RenderRequest {
    fn default() -> Self {
        Self {
            format: OutputFormat::Png,
            scale: 2,
            target_width: None,
            row_height: None,
            background: Background::Theme,
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

    /// Pad the PNG's height to whole rows `row_height` pixels tall.
    pub fn with_row_height(mut self, row_height: u32) -> Self {
        self.row_height = Some(row_height);
        self
    }

    pub fn with_background(mut self, background: Background) -> Self {
        self.background = background;
        self
    }

    /// Everything in this request that changes an artifact's bytes, for the
    /// renderers' cache keys: geometry, padding, background, and the PNG
    /// encoding.
    pub(crate) fn cache_identity(&self) -> serde_json::Value {
        serde_json::json!({
            "scale": self.scale.max(1),
            "target_width": self.target_width,
            "row_height": self.row_height,
            "background": self.background.cache_name(),
            "png_encoding": (self.format == OutputFormat::Png).then_some(PNG_ENCODING),
        })
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
