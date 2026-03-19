use std::fs;

use crate::artifact::{OutputFormat, RenderRequest, RenderedArtifact};
use crate::cache::file_cache::MERMAID_BACKEND;
use crate::cache::{FileCache, VisualizationKind};
use crate::raster::rasterize_svg;

use super::config::{MermaidConfig, MermaidTheme};
use super::error::MermaidError;

/// A Mermaid diagram for rendering.
///
/// This struct handles rendering Mermaid diagrams to SVG or PNG format with
/// support for caching and customization.
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme};
/// use biscuit_visualized::artifact::RenderRequest;
///
/// // Basic usage with default settings
/// let diagram = MermaidDiagram::new("flowchart LR\n    A --> B")
///     .with_theme(MermaidTheme::Forest)
///     .with_title("My Flowchart");
///
/// // Render to PNG (default)
/// let artifact = diagram.render(&RenderRequest::default()).unwrap();
/// println!("Rendered to: {:?}", artifact.path);
///
/// // Get fallback code block for terminals without image support
/// println!("{}", diagram.fallback_code_block());
/// ```
///
/// ## Supported Diagram Types
///
/// - Flowcharts (`flowchart` / `graph`)
/// - Sequence Diagrams (`sequenceDiagram`)
/// - Class Diagrams (`classDiagram`)
/// - State Diagrams (`stateDiagram-v2`)
/// - ER Diagrams (`erDiagram`)
/// - Pie Charts (`pie`)
/// - XY Charts (`xychart`)
/// - Quadrant Charts (`quadrantChart`)
/// - Gantt (`gantt`)
/// - Timeline (`timeline`)
/// - Journey (`journey`)
/// - Mindmap (`mindmap`)
/// - Git Graph (`gitGraph`)
#[derive(Debug, Clone)]
pub struct MermaidDiagram {
    instructions: String,
    title: Option<String>,
    theme: MermaidTheme,
    config: MermaidConfig,
}

impl MermaidDiagram {
    /// Creates a new Mermaid diagram with default settings.
    ///
    /// ## Arguments
    ///
    /// * `instructions` - The Mermaid diagram syntax
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::mermaid::MermaidDiagram;
    ///
    /// let diagram = MermaidDiagram::new("graph LR; A-->B");
    /// ```
    pub fn new(instructions: impl Into<String>) -> Self {
        Self {
            instructions: instructions.into(),
            title: None,
            theme: MermaidTheme::default(),
            config: MermaidConfig::default(),
        }
    }

    /// Sets the theme for the diagram.
    pub fn with_theme(mut self, theme: MermaidTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Sets the title for the diagram.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the configuration for the diagram.
    pub fn with_config(mut self, config: MermaidConfig) -> Self {
        self.config = config;
        self
    }

    /// Returns the diagram instructions.
    pub fn instructions(&self) -> &str {
        &self.instructions
    }

    /// Renders the diagram to SVG or PNG format.
    ///
    /// This method:
    /// 1. Checks the cache for a previously rendered artifact
    /// 2. If not cached, renders the diagram using mermaid-rs-renderer
    /// 3. If PNG is requested, rasterizes the SVG
    /// 4. Stores the result in the cache
    /// 5. Returns a `RenderedArtifact` with the path to the rendered file
    ///
    /// ## Arguments
    ///
    /// * `request` - Rendering parameters (format, scale, etc.)
    ///
    /// ## Returns
    ///
    /// A `RenderedArtifact` containing the path to the rendered file and metadata.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The diagram syntax is invalid
    /// - SVG rendering fails
    /// - PNG rasterization fails (when PNG format is requested)
    /// - File I/O operations fail
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_visualized::mermaid::MermaidDiagram;
    /// use biscuit_visualized::artifact::{RenderRequest, OutputFormat};
    ///
    /// let diagram = MermaidDiagram::new("graph LR; A-->B");
    ///
    /// // Render to PNG with default settings
    /// let artifact = diagram.render(&RenderRequest::default())?;
    ///
    /// // Render to SVG
    /// let mut request = RenderRequest::default();
    /// request.format = OutputFormat::Svg;
    /// let artifact = diagram.render(&request)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn render(&self, request: &RenderRequest) -> Result<RenderedArtifact, MermaidError> {
        // Build options JSON from config
        let options_json = self.config.to_json().unwrap_or_else(|| "{}".to_string());

        // Build cache key
        let cache_key = FileCache::cache_key(
            VisualizationKind::Mermaid,
            &self.instructions,
            &options_json,
            MERMAID_BACKEND,
            request.format,
        );

        // Check cache
        let cache = FileCache::new();
        if let Some(path) = cache.get(VisualizationKind::Mermaid, &cache_key, request.format) {
            let alt_text = self
                .title
                .clone()
                .unwrap_or_else(|| "Mermaid diagram".to_string());

            return Ok(RenderedArtifact {
                path,
                format: request.format,
                cache_hit: true,
                alt_text,
            });
        }

        // Render SVG using mermaid-rs-renderer
        let svg = self.render_svg()?;

        // Store in cache based on format
        let path = match request.format {
            OutputFormat::Svg => {
                // Store SVG directly
                cache.store(
                    VisualizationKind::Mermaid,
                    &cache_key,
                    OutputFormat::Svg,
                    svg.as_bytes(),
                )?
            }
            OutputFormat::Png => {
                // Write SVG to temp file
                let temp_dir = tempfile::tempdir()?;
                let svg_path = temp_dir.path().join("diagram.svg");
                fs::write(&svg_path, svg.as_bytes())?;

                // Rasterize to PNG
                let png_path = temp_dir.path().join("diagram.png");
                rasterize_svg(&svg_path, &png_path, request.scale)?;

                // Read PNG and store in cache
                let png_data = fs::read(&png_path)?;
                cache.store(
                    VisualizationKind::Mermaid,
                    &cache_key,
                    OutputFormat::Png,
                    &png_data,
                )?
            }
        };

        let alt_text = self
            .title
            .clone()
            .unwrap_or_else(|| "Mermaid diagram".to_string());

        Ok(RenderedArtifact {
            path,
            format: request.format,
            cache_hit: false,
            alt_text,
        })
    }

    /// Renders the diagram to SVG string.
    ///
    /// This is an internal method that calls mermaid-rs-renderer to produce
    /// the SVG output.
    fn render_svg(&self) -> Result<String, MermaidError> {
        // Use mermaid-rs-renderer to render the diagram
        // The library doesn't directly support theme configuration,
        // so we rely on the default rendering
        mermaid_rs_renderer::render(&self.instructions)
            .map_err(|e| MermaidError::RenderFailed(e.to_string()))
    }

    /// Returns a fallback code block representation of the diagram.
    ///
    /// This is useful for environments that don't support image rendering,
    /// such as text-only terminals or markdown viewers.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::mermaid::MermaidDiagram;
    ///
    /// let diagram = MermaidDiagram::new("graph LR; A-->B");
    /// let code_block = diagram.fallback_code_block();
    /// assert!(code_block.starts_with("```mermaid"));
    /// assert!(code_block.ends_with("```"));
    /// ```
    pub fn fallback_code_block(&self) -> String {
        format!("```mermaid\n{}\n```", self.instructions)
    }
}
