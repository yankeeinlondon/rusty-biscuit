use crate::artifact::{OutputFormat, RenderRequest, RenderedArtifact};
use crate::cache::file_cache::MERMAID_BACKEND;
use crate::cache::{FileCache, VisualizationKind};
use crate::raster::rasterize_svg_to_png_bytes;

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
/// use biscuit_visualized::artifact::RenderRequest;
/// use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTheme};
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
    /// use biscuit_visualized::artifact::{OutputFormat, RenderRequest};
    /// use biscuit_visualized::mermaid::MermaidDiagram;
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
        let options_json = self.render_options_json(request);
        let cache_key = FileCache::cache_key(
            VisualizationKind::Mermaid,
            &self.instructions,
            &options_json,
            MERMAID_BACKEND,
            request.format,
        );

        let cache = FileCache::new();
        if let Some(path) = cache.get(VisualizationKind::Mermaid, &cache_key, request.format) {
            return Ok(RenderedArtifact {
                path,
                format: request.format,
                cache_hit: true,
                alt_text: self.alt_text(),
            });
        }

        let svg = self.render_svg(request)?;
        let path = match request.format {
            OutputFormat::Svg => cache.store(
                VisualizationKind::Mermaid,
                &cache_key,
                OutputFormat::Svg,
                svg.as_bytes(),
            )?,
            OutputFormat::Png => {
                let png_data = rasterize_svg_to_png_bytes(&svg, request.scale)?;
                cache.store(
                    VisualizationKind::Mermaid,
                    &cache_key,
                    OutputFormat::Png,
                    &png_data,
                )?
            }
        };

        Ok(RenderedArtifact {
            path,
            format: request.format,
            cache_hit: false,
            alt_text: self.alt_text(),
        })
    }

    fn render_options_json(&self, request: &RenderRequest) -> String {
        let config_value = self
            .config
            .to_json()
            .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        serde_json::to_string(&serde_json::json!({
            "theme": self.theme.as_str(),
            "title": self.title,
            "config": config_value,
            "scale": request.scale.max(1),
            "transparent_background": request.transparent_background,
        }))
        .unwrap_or_else(|_| "{}".to_string())
    }

    fn render_svg(&self, request: &RenderRequest) -> Result<String, MermaidError> {
        let parsed = mermaid_rs_renderer::parse_mermaid(&self.instructions)
            .map_err(|err| MermaidError::RenderFailed(err.to_string()))?;

        let theme = self.build_theme(request.transparent_background);
        let layout_config = mermaid_rs_renderer::LayoutConfig::default();
        let layout = mermaid_rs_renderer::compute_layout(&parsed.graph, &theme, &layout_config);
        let svg = mermaid_rs_renderer::render_svg(&layout, &theme, &layout_config);

        Ok(self.apply_svg_overrides(svg))
    }

    fn build_theme(&self, transparent_background: bool) -> mermaid_rs_renderer::Theme {
        let mut theme = match self.theme {
            MermaidTheme::Default => mermaid_rs_renderer::Theme::mermaid_default(),
            MermaidTheme::Dark => dark_theme(),
            MermaidTheme::Forest => forest_theme(),
            MermaidTheme::Neutral => neutral_theme(),
        };

        if let Some(size) = self.config.point_label_font_size {
            theme.font_size = size as f32;
        }

        if transparent_background {
            theme.background = "none".to_string();
            theme.edge_label_background = "none".to_string();
        }

        theme
    }

    fn apply_svg_overrides(&self, svg: String) -> String {
        let mut output = svg;

        if let Some(fill) = &self.config.quadrant1_fill {
            output = replace_quadrant_fill(&output, "#ECECFF", fill);
        }
        if let Some(fill) = &self.config.quadrant2_fill {
            output = replace_quadrant_fill(&output, "#f1f1ff", fill);
        }
        if let Some(fill) = &self.config.quadrant3_fill {
            output = replace_quadrant_fill(&output, "#f6f6ff", fill);
        }
        if let Some(fill) = &self.config.quadrant4_fill {
            output = replace_quadrant_fill(&output, "#fbfbff", fill);
        }
        if let Some(radius) = self.config.point_radius {
            output = replace_circle_radius(&output, radius);
        }

        output
    }

    fn alt_text(&self) -> String {
        self.title
            .clone()
            .unwrap_or_else(|| "Mermaid diagram".to_string())
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

fn replace_quadrant_fill(svg: &str, current_fill: &str, replacement_fill: &str) -> String {
    svg.replacen(
        &format!("fill=\"{}\"", current_fill),
        &format!("fill=\"{}\"", replacement_fill),
        1,
    )
}

fn replace_circle_radius(svg: &str, radius: u32) -> String {
    let replacement = format!(" r=\"{}\"", radius.max(1));
    let mut output = String::with_capacity(svg.len());
    let mut remaining = svg;

    while let Some(circle_start) = remaining.find("<circle") {
        let (before_circle, circle_and_after) = remaining.split_at(circle_start);
        output.push_str(before_circle);

        if let Some(tag_end) = circle_and_after.find('>') {
            let (circle_tag, after_circle) = circle_and_after.split_at(tag_end + 1);
            output.push_str(&circle_tag.replacen(" r=\"5\"", &replacement, 1));
            remaining = after_circle;
        } else {
            output.push_str(circle_and_after);
            remaining = "";
        }
    }

    output.push_str(remaining);
    output
}

fn dark_theme() -> mermaid_rs_renderer::Theme {
    let mut theme = mermaid_rs_renderer::Theme::modern();
    theme.primary_color = "#1e293b".to_string();
    theme.primary_text_color = "#e2e8f0".to_string();
    theme.primary_border_color = "#64748b".to_string();
    theme.line_color = "#94a3b8".to_string();
    theme.secondary_color = "#334155".to_string();
    theme.tertiary_color = "#0f172a".to_string();
    theme.edge_label_background = "rgba(15,23,42,0.92)".to_string();
    theme.cluster_background = "#111827".to_string();
    theme.cluster_border = "#475569".to_string();
    theme.background = "#020617".to_string();
    theme.sequence_actor_fill = "#0f172a".to_string();
    theme.sequence_actor_border = "#64748b".to_string();
    theme.sequence_actor_line = "#94a3b8".to_string();
    theme.sequence_note_fill = "#422006".to_string();
    theme.sequence_note_border = "#f59e0b".to_string();
    theme.sequence_activation_fill = "#1e293b".to_string();
    theme.sequence_activation_border = "#64748b".to_string();
    theme.text_color = "#e2e8f0".to_string();
    theme.pie_title_text_color = "#e2e8f0".to_string();
    theme.pie_section_text_color = "#e2e8f0".to_string();
    theme.pie_legend_text_color = "#e2e8f0".to_string();
    theme.pie_stroke_color = "#cbd5e1".to_string();
    theme.pie_outer_stroke_color = "#64748b".to_string();
    theme
}

fn forest_theme() -> mermaid_rs_renderer::Theme {
    let mut theme = mermaid_rs_renderer::Theme::mermaid_default();
    theme.primary_color = "#dcfce7".to_string();
    theme.primary_text_color = "#14532d".to_string();
    theme.primary_border_color = "#16a34a".to_string();
    theme.line_color = "#166534".to_string();
    theme.secondary_color = "#bbf7d0".to_string();
    theme.tertiary_color = "#ecfdf5".to_string();
    theme.edge_label_background = "#f7fee7".to_string();
    theme.cluster_background = "#d1fae5".to_string();
    theme.cluster_border = "#34d399".to_string();
    theme.background = "#f0fdf4".to_string();
    theme.sequence_actor_fill = "#dcfce7".to_string();
    theme.sequence_actor_border = "#16a34a".to_string();
    theme.sequence_actor_line = "#166534".to_string();
    theme.sequence_note_fill = "#fef3c7".to_string();
    theme.sequence_note_border = "#d97706".to_string();
    theme.sequence_activation_fill = "#bbf7d0".to_string();
    theme.sequence_activation_border = "#16a34a".to_string();
    theme.text_color = "#14532d".to_string();
    theme
}

fn neutral_theme() -> mermaid_rs_renderer::Theme {
    let mut theme = mermaid_rs_renderer::Theme::modern();
    theme.primary_color = "#f5f5f5".to_string();
    theme.primary_text_color = "#262626".to_string();
    theme.primary_border_color = "#737373".to_string();
    theme.line_color = "#525252".to_string();
    theme.secondary_color = "#e5e5e5".to_string();
    theme.tertiary_color = "#ffffff".to_string();
    theme.edge_label_background = "#fafafa".to_string();
    theme.cluster_background = "#ededed".to_string();
    theme.cluster_border = "#a3a3a3".to_string();
    theme.background = "#fafafa".to_string();
    theme.sequence_actor_fill = "#f5f5f5".to_string();
    theme.sequence_actor_border = "#737373".to_string();
    theme.sequence_actor_line = "#525252".to_string();
    theme.sequence_note_fill = "#f5f5f4".to_string();
    theme.sequence_note_border = "#a8a29e".to_string();
    theme.sequence_activation_fill = "#e7e5e4".to_string();
    theme.sequence_activation_border = "#737373".to_string();
    theme.text_color = "#171717".to_string();
    theme
}
