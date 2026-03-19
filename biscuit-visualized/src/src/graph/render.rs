use super::{
    dot::{expression_to_dot, render_dot_to_svg, validate_dot},
    error::GraphError,
    expression::GraphExpression,
};
use crate::{
    artifact::{OutputFormat, RenderRequest, RenderedArtifact},
    cache::{file_cache::GRAPH_BACKEND, FileCache, VisualizationKind},
    raster,
};

/// Syntax mode for parsing graph source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphInputSyntax {
    /// Auto-detect syntax from content
    Auto,
    /// Parse as expression syntax
    Expression,
    /// Parse as DOT format
    Dot,
}

/// Graph layout orientation.
///
/// Note: layout-rs only supports TopToBottom and LeftToRight orientations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphOrientation {
    /// Left to right layout
    LeftToRight,
    /// Top to bottom layout (default)
    TopToBottom,
}

impl GraphOrientation {
    fn to_rankdir(&self) -> &'static str {
        match self {
            Self::LeftToRight => "LR",
            Self::TopToBottom => "TB",
        }
    }
}

/// Internal representation of graph source.
#[derive(Debug, Clone)]
pub enum GraphSource {
    Expression(GraphExpression),
    Dot(String),
}

/// A graph diagram that can be rendered to various output formats.
///
/// Supports both a lightweight expression syntax and full DOT format.
/// Handles caching, layout, and rasterization.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_visualized::graph::{GraphDiagram, GraphInputSyntax};
/// use biscuit_visualized::artifact::RenderRequest;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Expression syntax
/// let graph = GraphDiagram::from_expression("a -> b -> c")?;
/// let artifact = graph.render(&RenderRequest::default())?;
///
/// // DOT format
/// let dot = r#"
///     digraph G {
///         A -> B;
///         B -> C;
///     }
/// "#;
/// let graph = GraphDiagram::from_dot(dot)?
///     .with_title("My Graph".to_string());
/// let artifact = graph.render(&RenderRequest::default())?;
/// # Ok(())
/// # }
/// ```
pub struct GraphDiagram {
    source: GraphSource,
    title: Option<String>,
    orientation: GraphOrientation,
}

impl GraphDiagram {
    /// Creates a graph from expression syntax.
    ///
    /// Parses lightweight expression syntax:
    /// - Node identifiers: bare words or `"quoted strings"`
    /// - Directed edge: `->`
    /// - Undirected edge: `--`
    /// - Chain support: `a -> b -> c`
    /// - Statement separators: `;` or newline
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::graph::GraphDiagram;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let graph = GraphDiagram::from_expression("a -> b -> c")?;
    /// let graph = GraphDiagram::from_expression(r#""My App" -- Database"#)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns `GraphError::ExpressionParseFailed` if the syntax is invalid.
    pub fn from_expression(source: impl Into<String>) -> Result<Self, GraphError> {
        let source = source.into();
        let expr = GraphExpression::parse(&source)?;

        Ok(Self {
            source: GraphSource::Expression(expr),
            title: None,
            orientation: GraphOrientation::TopToBottom,
        })
    }

    /// Creates a graph from DOT format.
    ///
    /// Validates the DOT source for unsupported features before storing.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::graph::GraphDiagram;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let dot = r#"
    ///     digraph G {
    ///         A -> B;
    ///         B -> C;
    ///     }
    /// "#;
    /// let graph = GraphDiagram::from_dot(dot)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns `GraphError::UnsupportedDotFeature` if unsupported constructs
    /// are detected (e.g., HTML table labels).
    pub fn from_dot(source: impl Into<String>) -> Result<Self, GraphError> {
        let source = source.into();
        validate_dot(&source)?;

        Ok(Self {
            source: GraphSource::Dot(source),
            title: None,
            orientation: GraphOrientation::TopToBottom,
        })
    }

    /// Parses graph source with syntax detection.
    ///
    /// ## Auto-detection Rules
    ///
    /// 1. If first non-whitespace token is `graph` or `digraph` → DOT
    /// 2. If content contains `{` and `}` → DOT
    /// 3. Otherwise → expression syntax
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::graph::{GraphDiagram, GraphInputSyntax};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let graph = GraphDiagram::parse("a -> b", GraphInputSyntax::Auto)?;
    /// let graph = GraphDiagram::parse("digraph { A -> B }", GraphInputSyntax::Auto)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns an error if parsing fails for the detected or specified syntax.
    pub fn parse(
        source: impl Into<String>,
        syntax: GraphInputSyntax,
    ) -> Result<Self, GraphError> {
        let source = source.into();

        let detected_syntax = match syntax {
            GraphInputSyntax::Auto => {
                let trimmed = source.trim();
                let first_word = trimmed.split_whitespace().next().unwrap_or("");

                if first_word == "graph" || first_word == "digraph" {
                    GraphInputSyntax::Dot
                } else if trimmed.contains('{') && trimmed.contains('}') {
                    GraphInputSyntax::Dot
                } else {
                    GraphInputSyntax::Expression
                }
            }
            other => other,
        };

        match detected_syntax {
            GraphInputSyntax::Expression => Self::from_expression(source),
            GraphInputSyntax::Dot => Self::from_dot(source),
            GraphInputSyntax::Auto => unreachable!("Auto should be resolved by now"),
        }
    }

    /// Sets the diagram title.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::graph::GraphDiagram;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let graph = GraphDiagram::from_expression("a -> b")?
    ///     .with_title("My Graph".to_string());
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the graph orientation.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::graph::{GraphDiagram, GraphOrientation};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let graph = GraphDiagram::from_expression("a -> b")?
    ///     .with_orientation(GraphOrientation::LeftToRight);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_orientation(mut self, orientation: GraphOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Returns the DOT representation of the graph source.
    ///
    /// If the source is expression syntax, converts it to DOT.
    /// Applies orientation settings.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::graph::GraphDiagram;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let graph = GraphDiagram::from_expression("a -> b")?;
    /// let dot = graph.source_as_dot();
    /// assert!(dot.contains("digraph"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn source_as_dot(&self) -> String {
        let mut dot = match &self.source {
            GraphSource::Expression(expr) => {
                // Determine if directed based on edges
                let directed = expr
                    .edges
                    .first()
                    .map(|e| matches!(e.kind, super::expression::EdgeKind::Directed))
                    .unwrap_or(true);
                expression_to_dot(expr, directed)
            }
            GraphSource::Dot(dot_source) => dot_source.clone(),
        };

        // Apply orientation by injecting rankdir attribute
        dot = self.apply_orientation_to_dot(&dot);

        // Apply title if present
        if let Some(title) = &self.title {
            dot = self.apply_title_to_dot(&dot, title);
        }

        dot
    }

    fn apply_orientation_to_dot(&self, dot: &str) -> String {
        let rankdir = self.orientation.to_rankdir();

        // Find the opening brace and insert rankdir attribute
        if let Some(brace_pos) = dot.find('{') {
            let mut result = String::new();
            result.push_str(&dot[..=brace_pos]);
            result.push_str(&format!("\n    rankdir={};\n", rankdir));
            result.push_str(&dot[brace_pos + 1..]);
            result
        } else {
            dot.to_string()
        }
    }

    fn apply_title_to_dot(&self, dot: &str, title: &str) -> String {
        let escaped_title = title.replace('"', "\\\"");

        // Find the opening brace and insert label attribute
        if let Some(brace_pos) = dot.find('{') {
            let mut result = String::new();
            result.push_str(&dot[..=brace_pos]);
            result.push_str(&format!("\n    label=\"{}\";\n", escaped_title));
            result.push_str(&dot[brace_pos + 1..]);
            result
        } else {
            dot.to_string()
        }
    }

    /// Renders the graph to the requested output format.
    ///
    /// Uses caching to avoid re-rendering identical graphs. If the rendered
    /// artifact exists in cache, returns it immediately. Otherwise, renders
    /// to SVG, optionally rasterizes to PNG, and stores in cache.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_visualized::graph::GraphDiagram;
    /// use biscuit_visualized::artifact::{RenderRequest, OutputFormat};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let graph = GraphDiagram::from_expression("a -> b")?;
    ///
    /// let request = RenderRequest {
    ///     format: OutputFormat::Png,
    ///     scale: 2,
    ///     transparent_background: false,
    /// };
    ///
    /// let artifact = graph.render(&request)?;
    /// println!("Rendered to: {:?}", artifact.path);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - DOT parsing fails
    /// - SVG rendering fails
    /// - PNG rasterization fails (when format is PNG)
    /// - Cache storage fails
    pub fn render(&self, request: &RenderRequest) -> Result<RenderedArtifact, GraphError> {
        let cache = FileCache::new();

        // Convert to DOT
        let dot_source = self.source_as_dot();

        // Build options JSON
        let options = serde_json::json!({
            "orientation": self.orientation.to_rankdir(),
            "title": self.title,
        });
        let options_json = serde_json::to_string(&options).unwrap_or_default();

        // Build cache key
        let cache_key = FileCache::cache_key(
            VisualizationKind::Graph,
            &dot_source,
            &options_json,
            GRAPH_BACKEND,
            request.format,
        );

        // Check cache
        if let Some(path) = cache.get(VisualizationKind::Graph, &cache_key, request.format) {
            return Ok(RenderedArtifact {
                path,
                format: request.format,
                cache_hit: true,
                alt_text: self.generate_alt_text(),
            });
        }

        // Render DOT to SVG
        let svg_content = render_dot_to_svg(&dot_source)?;

        // Handle output format
        let (final_content, final_format) = match request.format {
            OutputFormat::Svg => (svg_content.as_bytes().to_vec(), OutputFormat::Svg),
            OutputFormat::Png => {
                // Create temporary SVG file
                let temp_svg = tempfile::NamedTempFile::new()?;
                std::fs::write(temp_svg.path(), &svg_content)?;

                // Create temporary PNG file
                let temp_png = tempfile::NamedTempFile::new()?;

                // Rasterize
                raster::rasterize_svg(temp_svg.path(), temp_png.path(), request.scale)?;

                // Read PNG content
                let png_content = std::fs::read(temp_png.path())?;
                (png_content, OutputFormat::Png)
            }
        };

        // Store in cache
        let path = cache.store(
            VisualizationKind::Graph,
            &cache_key,
            final_format,
            &final_content,
        )?;

        Ok(RenderedArtifact {
            path,
            format: final_format,
            cache_hit: false,
            alt_text: self.generate_alt_text(),
        })
    }

    fn generate_alt_text(&self) -> String {
        let node_count = match &self.source {
            GraphSource::Expression(expr) => expr.nodes.len(),
            GraphSource::Dot(dot) => {
                // Rough estimate from DOT source
                dot.lines().filter(|l| l.contains("->") || l.contains("--")).count()
            }
        };

        if let Some(title) = &self.title {
            format!("{} (graph with {} nodes)", title, node_count)
        } else {
            format!("Graph with {} nodes", node_count)
        }
    }
}
