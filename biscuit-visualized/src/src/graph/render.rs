use super::{
    dot::{dot_node_count, expression_to_dot, render_dot_to_svg, validate_dot},
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
    /// Auto-detect syntax from content.
    Auto,
    /// Parse as expression syntax.
    Expression,
    /// Parse as DOT format.
    Dot,
}

impl GraphInputSyntax {
    /// Returns the CLI-friendly spelling for this syntax.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Expression => "expression",
            Self::Dot => "dot",
        }
    }
}

/// Graph layout orientation.
///
/// `layout-rs` currently supports left-to-right and top-to-bottom layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphOrientation {
    /// Left to right layout.
    LeftToRight,
    /// Top to bottom layout (default).
    TopToBottom,
}

impl GraphOrientation {
    fn to_rankdir(self) -> &'static str {
        match self {
            Self::LeftToRight => "LR",
            Self::TopToBottom => "TB",
        }
    }

    /// Returns the CLI-friendly spelling for this orientation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LeftToRight => "left-to-right",
            Self::TopToBottom => "top-to-bottom",
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
/// use biscuit_visualized::artifact::RenderRequest;
/// use biscuit_visualized::graph::{GraphDiagram, GraphInputSyntax};
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
/// let graph = GraphDiagram::from_dot(dot)?.with_title("My Graph".to_string());
/// let artifact = graph.render(&RenderRequest::default())?;
/// # Ok(())
/// # }
/// ```
pub struct GraphDiagram {
    source: GraphSource,
    source_text: String,
    syntax: GraphInputSyntax,
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
    /// ## Errors
    ///
    /// Returns `GraphError::ExpressionParseFailed` if the syntax is invalid.
    pub fn from_expression(source: impl Into<String>) -> Result<Self, GraphError> {
        let source = source.into();
        let expr = GraphExpression::parse(&source)?;

        Ok(Self {
            source: GraphSource::Expression(expr),
            source_text: source,
            syntax: GraphInputSyntax::Expression,
            title: None,
            orientation: GraphOrientation::TopToBottom,
        })
    }

    /// Creates a graph from DOT format.
    ///
    /// Validates the DOT source for unsupported features before storing.
    pub fn from_dot(source: impl Into<String>) -> Result<Self, GraphError> {
        let source = source.into();
        validate_dot(&source)?;

        Ok(Self {
            source: GraphSource::Dot(source.clone()),
            source_text: source,
            syntax: GraphInputSyntax::Dot,
            title: None,
            orientation: GraphOrientation::TopToBottom,
        })
    }

    /// Parses graph source with syntax detection.
    ///
    /// ## Auto-detection Rules
    ///
    /// 1. If first non-whitespace token is `graph` or `digraph` -> DOT
    /// 2. If content contains `{` and `}` -> DOT
    /// 3. Otherwise -> expression syntax
    pub fn parse(source: impl Into<String>, syntax: GraphInputSyntax) -> Result<Self, GraphError> {
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
            GraphInputSyntax::Auto => unreachable!("Auto should be resolved before parsing"),
        }
    }

    /// Returns the original graph source text.
    pub fn source(&self) -> &str {
        &self.source_text
    }

    /// Returns the resolved syntax for this diagram.
    pub fn syntax(&self) -> GraphInputSyntax {
        self.syntax
    }

    /// Sets the diagram title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the graph orientation.
    pub fn with_orientation(mut self, orientation: GraphOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Returns the DOT representation of the graph source.
    ///
    /// If the source is expression syntax, converts it to DOT.
    /// Applies orientation settings.
    pub fn source_as_dot(&self) -> String {
        let mut dot = match &self.source {
            GraphSource::Expression(expr) => {
                let directed = expr
                    .edges
                    .first()
                    .map(|edge| matches!(edge.kind, super::expression::EdgeKind::Directed))
                    .unwrap_or(true);
                expression_to_dot(expr, directed)
            }
            GraphSource::Dot(dot_source) => dot_source.clone(),
        };

        dot = self.apply_orientation_to_dot(&dot);

        if let Some(title) = &self.title {
            dot = self.apply_title_to_dot(&dot, title);
        }

        dot
    }

    /// Returns a fenced code block using the original source syntax.
    pub fn fallback_code_block(&self) -> String {
        let info_string = match self.syntax {
            GraphInputSyntax::Dot => "dot",
            GraphInputSyntax::Expression | GraphInputSyntax::Auto => "graph-expression",
        };

        format!("```{info_string}\n{}\n```", self.source_text)
    }

    fn apply_orientation_to_dot(&self, dot: &str) -> String {
        let rankdir = self.orientation.to_rankdir();

        if let Some(existing_rankdir) = dot.find("rankdir=") {
            let mut end = existing_rankdir;
            while end < dot.len() {
                let ch = dot.as_bytes()[end] as char;
                if ch == ';' || ch == '\n' {
                    break;
                }
                end += 1;
            }

            let mut result = String::new();
            result.push_str(&dot[..existing_rankdir]);
            result.push_str(&format!("rankdir={rankdir}"));
            result.push_str(&dot[end..]);
            return result;
        }

        if let Some(brace_pos) = dot.find('{') {
            let mut result = String::new();
            result.push_str(&dot[..=brace_pos]);
            result.push_str(&format!("\n    rankdir={rankdir};\n"));
            result.push_str(&dot[brace_pos + 1..]);
            result
        } else {
            dot.to_string()
        }
    }

    fn apply_title_to_dot(&self, dot: &str, title: &str) -> String {
        let escaped_title = title.replace('"', "\\\"");

        if let Some(brace_pos) = dot.find('{') {
            let mut result = String::new();
            result.push_str(&dot[..=brace_pos]);
            result.push_str(&format!("\n    label=\"{escaped_title}\";\n"));
            result.push_str(&dot[brace_pos + 1..]);
            result
        } else {
            dot.to_string()
        }
    }

    /// Renders the graph to the requested output format.
    pub fn render(&self, request: &RenderRequest) -> Result<RenderedArtifact, GraphError> {
        let cache = FileCache::new();
        let cache_key = self.cache_key(request);

        if let Some(path) = cache.get(VisualizationKind::Graph, &cache_key, request.format) {
            return Ok(RenderedArtifact {
                path,
                format: request.format,
                cache_hit: true,
                alt_text: self.generate_alt_text(),
            });
        }

        let dot_source = self.source_as_dot();
        let svg_content = apply_graph_background(
            &render_dot_to_svg(&dot_source)?,
            request.transparent_background,
        );

        let (final_content, final_format) = match request.format {
            OutputFormat::Svg => (svg_content.into_bytes(), OutputFormat::Svg),
            OutputFormat::Png => (
                raster::rasterize_svg_to_png_bytes(&svg_content, request.scale)?,
                OutputFormat::Png,
            ),
        };

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

    fn cache_key(&self, request: &RenderRequest) -> String {
        let options_json = serde_json::to_string(&serde_json::json!({
            "syntax": self.syntax.as_str(),
            "orientation": self.orientation.as_str(),
            "title": self.title,
            "scale": request.scale.max(1),
            "transparent_background": request.transparent_background,
        }))
        .unwrap_or_else(|_| "{}".to_string());

        FileCache::cache_key(
            VisualizationKind::Graph,
            &self.source_text,
            &options_json,
            GRAPH_BACKEND,
            request.format,
        )
    }

    fn generate_alt_text(&self) -> String {
        let node_count = match &self.source {
            GraphSource::Expression(expr) => expr.nodes.len(),
            GraphSource::Dot(dot) => dot_node_count(dot).unwrap_or(0),
        };

        if let Some(title) = &self.title {
            format!("{title} (graph with {node_count} nodes)")
        } else {
            format!("Graph with {node_count} nodes")
        }
    }
}

fn apply_graph_background(svg: &str, transparent_background: bool) -> String {
    if transparent_background {
        return svg.to_string();
    }

    let Some(svg_start) = svg.find("<svg") else {
        return svg.to_string();
    };
    let Some(open_end) = svg[svg_start..].find('>') else {
        return svg.to_string();
    };

    let insert_pos = svg_start + open_end + 1;
    let mut output = String::with_capacity(svg.len() + 48);
    output.push_str(&svg[..insert_pos]);
    output.push_str(r##"<rect width="100%" height="100%" fill="#ffffff"/>"##);
    output.push_str(&svg[insert_pos..]);
    output
}
