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

/// Color theme for graph rendering.
///
/// Controls the colors of nodes, edges, and text to ensure legibility
/// against different background colors (e.g., dark vs light terminals).
#[derive(Debug, Clone)]
pub struct GraphColorTheme {
    /// Surface/background color for opaque renders.
    pub surface_color: String,
    /// Node border color (hex, e.g. "#ffffff")
    pub node_color: String,
    /// Node fill color (hex)
    pub node_fill: String,
    /// Node text color (hex)
    pub font_color: String,
    /// Edge/arrow color (hex)
    pub edge_color: String,
    /// Font family for node/edge labels (CSS font-family value)
    pub font_family: String,
}

impl GraphColorTheme {
    /// Theme suitable for dark backgrounds.
    pub fn dark() -> Self {
        Self {
            surface_color: "#020617".to_string(),
            node_color: "#aaaaaa".to_string(),
            node_fill: "#1f2937".to_string(),
            font_color: "#e0e0e0".to_string(),
            edge_color: "#cccccc".to_string(),
            font_family: "Helvetica, Arial, sans-serif".to_string(),
        }
    }

    /// Theme suitable for light backgrounds.
    pub fn light() -> Self {
        Self {
            surface_color: "#f8fafc".to_string(),
            node_color: "#333333".to_string(),
            node_fill: "#ffffff".to_string(),
            font_color: "#000000".to_string(),
            edge_color: "#333333".to_string(),
            font_family: "Helvetica, Arial, sans-serif".to_string(),
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
    color_theme: Option<GraphColorTheme>,
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
            color_theme: None,
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
            color_theme: None,
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

                if first_word == "graph"
                    || first_word == "digraph"
                    || (trimmed.contains('{') && trimmed.contains('}'))
                {
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

    /// Sets the color theme for graph rendering.
    ///
    /// When set, DOT graph-level attributes are injected to control
    /// node fill, border, text, and edge colors for legibility.
    pub fn with_color_theme(mut self, theme: GraphColorTheme) -> Self {
        self.color_theme = Some(theme);
        self
    }

    /// Overrides the font family in the color theme.
    ///
    /// If no color theme is set, this has no effect (font styling
    /// is only applied when a color theme is active).
    pub fn with_font_family(mut self, font_family: impl Into<String>) -> Self {
        if let Some(theme) = &mut self.color_theme {
            theme.font_family = font_family.into();
        }
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

        if let Some(theme) = &self.color_theme {
            dot = self.apply_color_theme_to_dot(&dot, theme);
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

    fn apply_color_theme_to_dot(&self, dot: &str, theme: &GraphColorTheme) -> String {
        if let Some(brace_pos) = dot.find('{') {
            let mut result = String::new();
            result.push_str(&dot[..=brace_pos]);
            result.push_str(&format!(
                "\n    node [color=\"{}\" fillcolor=\"{}\" fontcolor=\"{}\" style=filled];\n    edge [color=\"{}\" fontcolor=\"{}\"];\n",
                theme.node_color, theme.node_fill, theme.font_color, theme.edge_color, theme.edge_color
            ));
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
        let mut svg_content = render_dot_to_svg(&dot_source)?;

        // Trim excess padding from layout-rs output
        svg_content = trim_svg_padding(&svg_content, 20.0);

        svg_content = apply_graph_background(
            &svg_content,
            request.transparent_background,
            self.color_theme.as_ref(),
        );

        // layout-rs ignores DOT fontcolor/fontname, so apply via SVG post-processing
        if let Some(theme) = &self.color_theme {
            svg_content = apply_text_style(&svg_content, &theme.font_color, &theme.font_family);
        }

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
        let theme_key = self.color_theme.as_ref().map(|t| {
            format!(
                "{}/{}/{}/{}/{}/{}",
                t.surface_color,
                t.node_color,
                t.node_fill,
                t.font_color,
                t.edge_color,
                t.font_family
            )
        });
        let options_json = serde_json::to_string(&serde_json::json!({
            "syntax": self.syntax.as_str(),
            "orientation": self.orientation.as_str(),
            "title": self.title,
            "scale": request.scale.max(1),
            "transparent_background": request.transparent_background,
            "color_theme": theme_key,
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

fn apply_graph_background(
    svg: &str,
    transparent_background: bool,
    theme: Option<&GraphColorTheme>,
) -> String {
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
    let fill = theme
        .map(|theme| theme.surface_color.as_str())
        .unwrap_or("#ffffff");
    output.push_str(&format!(
        r#"<rect width="100%" height="100%" fill="{fill}"/>"#
    ));
    output.push_str(&svg[insert_pos..]);
    output
}

/// Computes a tight viewBox from SVG element positions and replaces the
/// original `<svg>` dimensions.
///
/// layout-rs sizes the canvas from `grow_window` calls (element positions +
/// 5px). The node coordinates themselves already include layout-rs's internal
/// 60px inter-node padding, so the content is well-spaced. This function
/// scans all `cx`/`cy`/`rx`/`ry` (ellipses) and path `d` attributes to find
/// the actual content bounding box, then adds a small `pad` around it.
fn trim_svg_padding(svg: &str, pad: f64) -> String {
    let mut min_x: f64 = f64::MAX;
    let mut min_y: f64 = f64::MAX;
    let mut max_x: f64 = f64::MIN;
    let mut max_y: f64 = f64::MIN;

    let parse_attr_f64 = |haystack: &str, attr: &str| -> Option<f64> {
        let needle = format!("{attr}=\"");
        let pos = haystack.find(&needle)? + needle.len();
        let end = haystack[pos..].find('"')? + pos;
        haystack[pos..end].parse().ok()
    };

    // Scan ellipses: <ellipse cx="..." cy="..." rx="..." ry="..."/>
    for chunk in svg.split("<ellipse ").skip(1) {
        let tag_end = chunk.find("/>").unwrap_or(chunk.len());
        let tag = &chunk[..tag_end];
        if let (Some(cx), Some(cy), Some(rx), Some(ry)) = (
            parse_attr_f64(tag, "cx"),
            parse_attr_f64(tag, "cy"),
            parse_attr_f64(tag, "rx"),
            parse_attr_f64(tag, "ry"),
        ) {
            min_x = min_x.min(cx - rx);
            min_y = min_y.min(cy - ry);
            max_x = max_x.max(cx + rx);
            max_y = max_y.max(cy + ry);
        }
    }

    // Scan paths for arrow endpoints: d="M x y C x1 y1, x2 y2, x3 y3"
    for chunk in svg.split("<path ").skip(1) {
        let tag_end = chunk.find("/>").unwrap_or(chunk.len());
        let tag = &chunk[..tag_end];
        if let Some(d_start) = tag.find("d=\"") {
            let d_val_start = d_start + 3;
            let d_val_end = tag[d_val_start..].find('"').unwrap_or(0) + d_val_start;
            let d_val = &tag[d_val_start..d_val_end];
            // Extract x/y pairs from path data
            let nums: Vec<f64> = d_val
                .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
                .filter_map(|t| t.trim().parse::<f64>().ok())
                .collect();
            for pair in nums.chunks(2) {
                if pair.len() == 2 {
                    min_x = min_x.min(pair[0]);
                    max_x = max_x.max(pair[0]);
                    min_y = min_y.min(pair[1]);
                    max_y = max_y.max(pair[1]);
                }
            }
        }
    }

    if min_x >= max_x || min_y >= max_y {
        return svg.to_string();
    }

    let vb_x = (min_x - pad).max(0.0);
    let vb_y = (min_y - pad).max(0.0);
    let vb_w = max_x - min_x + 2.0 * pad;
    let vb_h = max_y - min_y + 2.0 * pad;

    let Some(svg_start) = svg.find("<svg") else {
        return svg.to_string();
    };
    let Some(tag_end) = svg[svg_start..].find('>') else {
        return svg.to_string();
    };

    let new_tag = format!(
        "<svg width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\" xmlns=\"http://www.w3.org/2000/svg\">",
        vb_w.round() as u32,
        vb_h.round() as u32,
        vb_x, vb_y, vb_w, vb_h
    );

    let mut result = String::with_capacity(svg.len());
    result.push_str(&svg[..svg_start]);
    result.push_str(&new_tag);
    result.push_str(&svg[svg_start + tag_end + 1..]);
    result
}

/// Applies text fill color and font-family to the SVG.
///
/// layout-rs ignores DOT `fontcolor` and `fontname` attributes, always
/// emitting `font-family: Times, serif`. This function patches both via
/// string replacement on the generated SVG.
fn apply_text_style(svg: &str, color: &str, font_family: &str) -> String {
    let svg = svg.replace("<text ", &format!("<text fill=\"{color}\" "));
    svg.replace(
        "font-family: Times, serif;",
        &format!("font-family: {font_family};"),
    )
}
