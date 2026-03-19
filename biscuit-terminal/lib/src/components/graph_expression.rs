//! Graph expression rendering for terminals.
//!
//! This module provides a thin adapter over `biscuit-visualized` for rendering
//! graph diagrams in the terminal. It delegates rendering to `biscuit-visualized`
//! and adds terminal-specific display logic using `viuer`.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use biscuit_terminal::components::graph_expression::{GraphExpressionRenderer, GraphInputSyntax};
//!
//! fn example() -> Result<(), biscuit_terminal::components::graph_expression::GraphRenderError> {
//!     let renderer = GraphExpressionRenderer::parse("a -> b -> c", GraphInputSyntax::Auto)?;
//!     renderer.render_for_terminal()?;
//!     Ok(())
//! }
//! ```

use std::path::PathBuf;

use thiserror::Error;

// Re-export types from biscuit-visualized
pub use biscuit_visualized::graph::{GraphInputSyntax, GraphOrientation};

/// Errors that can occur during terminal rendering of graph diagrams.
#[derive(Error, Debug)]
pub enum GraphRenderError {
    /// Visualization error from biscuit-visualized
    #[error(transparent)]
    Visualization(#[from] biscuit_visualized::graph::GraphError),
    /// Terminal does not support inline images
    #[error("Terminal does not support inline images")]
    NoImageSupport,
    /// Image display failed
    #[error("Image display failed: {0}")]
    DisplayError(String),
}

/// A graph expression renderer for terminal output.
///
/// This struct wraps `biscuit_visualized::graph::GraphDiagram` and adds
/// terminal-specific display capabilities using `viuer`.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_terminal::components::graph_expression::{
///     GraphExpressionRenderer, GraphInputSyntax, GraphOrientation,
/// };
///
/// // Basic usage with default settings
/// let renderer = GraphExpressionRenderer::parse("a -> b -> c", GraphInputSyntax::Auto)?
///     .with_orientation(GraphOrientation::LeftToRight)
///     .with_title("My Graph")
///     .with_scale(2);
///
/// // Render to terminal (requires capable terminal)
/// if let Err(e) = renderer.render_for_terminal() {
///     // Fall back to code block on incapable terminals
///     println!("{}", renderer.fallback_code_block());
/// }
/// # Ok::<(), biscuit_terminal::components::graph_expression::GraphRenderError>(())
/// ```
///
/// ## Supported Input Formats
///
/// - Expression syntax: `a -> b -> c` (directed), `a -- b -- c` (undirected)
/// - DOT format: full Graphviz DOT language
/// - Auto-detection based on content
///
/// ## Terminal Compatibility
///
/// For inline image display, use a capable terminal:
/// - **Kitty** - Full support
/// - **iTerm2** - Full support
/// - **WezTerm** - Full support
/// - **Ghostty** - Full support
/// - **Windows Terminal** - Limited support
///
/// On incompatible terminals, use `fallback_code_block()` for plain text output.
pub struct GraphExpressionRenderer {
    /// The inner diagram from biscuit-visualized
    diagram: biscuit_visualized::graph::GraphDiagram,
    /// Scale factor for output resolution (default: 2)
    scale: u32,
    /// Use transparent background
    transparent_background: bool,
}

impl GraphExpressionRenderer {
    /// Creates a new GraphExpressionRenderer by parsing the given source.
    ///
    /// Uses default settings: scale 2, opaque background.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::graph_expression::{GraphExpressionRenderer, GraphInputSyntax};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let renderer = GraphExpressionRenderer::parse("a -> b", GraphInputSyntax::Auto)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns `GraphRenderError` if parsing fails.
    pub fn parse(
        source: impl Into<String>,
        syntax: GraphInputSyntax,
    ) -> Result<Self, GraphRenderError> {
        let diagram = biscuit_visualized::graph::GraphDiagram::parse(source, syntax)?;

        Ok(Self {
            diagram,
            scale: 2,
            transparent_background: false,
        })
    }

    /// Creates a GraphExpressionRenderer configured for the current terminal.
    ///
    /// Automatically enables transparent background for better terminal integration.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_terminal::components::graph_expression::{GraphExpressionRenderer, GraphInputSyntax};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let renderer = GraphExpressionRenderer::for_terminal("a -> b", GraphInputSyntax::Auto)?;
    /// // Transparent background is automatically enabled
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns `GraphRenderError` if parsing fails.
    pub fn for_terminal(
        source: impl Into<String>,
        syntax: GraphInputSyntax,
    ) -> Result<Self, GraphRenderError> {
        let diagram = biscuit_visualized::graph::GraphDiagram::parse(source, syntax)?;

        Ok(Self {
            diagram,
            scale: 2,
            transparent_background: true,
        })
    }

    /// Sets the title for the diagram.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::graph_expression::{GraphExpressionRenderer, GraphInputSyntax};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let renderer = GraphExpressionRenderer::parse("a -> b", GraphInputSyntax::Auto)?
    ///     .with_title("My Graph");
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.diagram = self.diagram.with_title(title);
        self
    }

    /// Sets the graph orientation.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::graph_expression::{
    ///     GraphExpressionRenderer, GraphInputSyntax, GraphOrientation,
    /// };
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let renderer = GraphExpressionRenderer::parse("a -> b", GraphInputSyntax::Auto)?
    ///     .with_orientation(GraphOrientation::LeftToRight);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_orientation(mut self, orientation: GraphOrientation) -> Self {
        self.diagram = self.diagram.with_orientation(orientation);
        self
    }

    /// Enables transparent background for better terminal integration.
    ///
    /// When enabled, the diagram background will be transparent,
    /// allowing it to blend with the terminal's background color.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::graph_expression::{GraphExpressionRenderer, GraphInputSyntax};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let renderer = GraphExpressionRenderer::parse("a -> b", GraphInputSyntax::Auto)?
    ///     .with_transparent_background(true);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_transparent_background(mut self, transparent: bool) -> Self {
        self.transparent_background = transparent;
        self
    }

    /// Sets the scale factor for output resolution.
    ///
    /// Higher values produce sharper images but larger files.
    /// Default is 2 (good for most modern displays).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::graph_expression::{GraphExpressionRenderer, GraphInputSyntax};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let renderer = GraphExpressionRenderer::parse("a -> b", GraphInputSyntax::Auto)?
    ///     .with_scale(3); // Extra sharp
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_scale(mut self, scale: u32) -> Self {
        self.scale = scale.max(1); // Minimum scale of 1
        self
    }

    /// Renders the diagram to a cached PNG file, returning the path and cache hit status.
    ///
    /// This method renders the diagram using biscuit-visualized's caching system.
    ///
    /// ## Returns
    ///
    /// Returns `(PathBuf, bool)` where the bool indicates whether this was a cache hit.
    ///
    /// ## Errors
    ///
    /// Returns error if:
    /// - Diagram rendering fails
    /// - File I/O operations fail
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_terminal::components::graph_expression::{GraphExpressionRenderer, GraphInputSyntax};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let renderer = GraphExpressionRenderer::parse("a -> b", GraphInputSyntax::Auto)?;
    /// let (path, cache_hit) = renderer.render_to_cached_png()?;
    /// println!("Rendered to: {:?} (cache hit: {})", path, cache_hit);
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(skip(self))]
    pub fn render_to_cached_png(&self) -> Result<(PathBuf, bool), GraphRenderError> {
        let request = biscuit_visualized::artifact::RenderRequest {
            format: biscuit_visualized::artifact::OutputFormat::Png,
            scale: self.scale,
            transparent_background: self.transparent_background,
        };

        let artifact = self.diagram.render(&request)?;

        tracing::info!(
            path = ?artifact.path,
            cache_hit = artifact.cache_hit,
            "Rendered graph diagram to PNG"
        );

        Ok((artifact.path, artifact.cache_hit))
    }

    /// Renders the diagram to the terminal.
    ///
    /// This method:
    /// 1. Checks if the terminal supports image rendering
    /// 2. Renders the diagram to PNG using biscuit-visualized
    /// 3. Displays the PNG using viuer
    ///
    /// ## Errors
    ///
    /// Returns `GraphRenderError` if:
    /// - Terminal doesn't support image rendering
    /// - Diagram rendering fails (invalid syntax, etc.)
    /// - Image display fails
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_terminal::components::graph_expression::{GraphExpressionRenderer, GraphInputSyntax};
    ///
    /// # fn example() -> Result<(), biscuit_terminal::components::graph_expression::GraphRenderError> {
    /// let renderer = GraphExpressionRenderer::parse("a -> b", GraphInputSyntax::Auto)?;
    /// renderer.render_for_terminal()?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(skip(self))]
    pub fn render_for_terminal(&self) -> Result<(), GraphRenderError> {
        // Check terminal support
        if !Self::terminal_supports_images() {
            tracing::debug!("Terminal does not support image rendering");
            return Err(GraphRenderError::NoImageSupport);
        }

        // Render to PNG using biscuit-visualized
        let request = biscuit_visualized::artifact::RenderRequest {
            format: biscuit_visualized::artifact::OutputFormat::Png,
            scale: self.scale,
            transparent_background: self.transparent_background,
        };

        let artifact = self.diagram.render(&request)?;

        tracing::info!(path = ?artifact.path, cache_hit = artifact.cache_hit, "Rendered graph diagram");

        // Display with viuer
        let config = viuer::Config {
            absolute_offset: false,
            ..Default::default()
        };

        viuer::print_from_file(&artifact.path, &config)
            .map_err(|e| GraphRenderError::DisplayError(e.to_string()))?;

        tracing::debug!("Displayed graph diagram in terminal");

        Ok(())
    }

    /// Checks if the current terminal supports image rendering.
    ///
    /// Returns `true` if either Kitty or iTerm2 image protocols are supported.
    pub fn terminal_supports_images() -> bool {
        use crate::discovery::detection::ImageSupport;
        use crate::terminal::Terminal;

        let term = Terminal::new();
        !matches!(term.image_support, ImageSupport::None)
    }

    /// Returns a fallback code block string for the diagram.
    ///
    /// This is used when terminal rendering fails or is not supported.
    /// Returns the original graph input formatted as a fenced code block.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::graph_expression::{GraphExpressionRenderer, GraphInputSyntax};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let renderer = GraphExpressionRenderer::parse("a -> b", GraphInputSyntax::Auto)?;
    /// let fallback = renderer.fallback_code_block();
    /// assert!(fallback.contains("```graph-expression"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn fallback_code_block(&self) -> String {
        self.diagram.fallback_code_block()
    }

    /// Prints the fallback code block to stdout.
    ///
    /// This is a convenience method for when terminal rendering fails.
    pub fn print_fallback(&self) {
        println!("{}", self.fallback_code_block());
    }
}

impl TryFrom<String> for GraphExpressionRenderer {
    type Error = GraphRenderError;

    fn try_from(source: String) -> Result<Self, Self::Error> {
        Self::parse(source, GraphInputSyntax::Auto)
    }
}

impl TryFrom<&str> for GraphExpressionRenderer {
    type Error = GraphRenderError;

    fn try_from(source: &str) -> Result<Self, Self::Error> {
        Self::parse(source, GraphInputSyntax::Auto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_expression_renderer_parse() {
        let renderer = GraphExpressionRenderer::parse("a -> b -> c", GraphInputSyntax::Auto);
        assert!(renderer.is_ok());
    }

    #[test]
    fn test_graph_expression_renderer_with_title() {
        let renderer = GraphExpressionRenderer::parse("a -> b", GraphInputSyntax::Auto)
            .unwrap()
            .with_title("Test Graph");
        // Title is stored internally, can verify through rendering
        let dot = renderer.diagram.source_as_dot();
        assert!(dot.contains("label=\"Test Graph\""));
    }

    #[test]
    fn test_graph_expression_renderer_from_string() {
        let source = String::from("a -> b -> c");
        let renderer = GraphExpressionRenderer::try_from(source).unwrap();
        let dot = renderer.diagram.source_as_dot();
        assert!(dot.contains("digraph"));
    }

    #[test]
    fn test_graph_expression_renderer_from_str() {
        let renderer = GraphExpressionRenderer::try_from("a -> b -> c").unwrap();
        let dot = renderer.diagram.source_as_dot();
        assert!(dot.contains("digraph"));
    }

    #[test]
    fn test_fallback_code_block() {
        let renderer = GraphExpressionRenderer::parse("a -> b", GraphInputSyntax::Auto).unwrap();
        let output = renderer.fallback_code_block();
        assert!(output.starts_with("```graph-expression\n"));
        assert!(output.ends_with("\n```"));
        assert!(output.contains("a -> b"));
    }

    #[test]
    fn test_fallback_code_block_uses_dot_info_string_for_dot_input() {
        let renderer =
            GraphExpressionRenderer::parse("digraph { A -> B; }", GraphInputSyntax::Dot).unwrap();
        let output = renderer.fallback_code_block();
        assert!(output.starts_with("```dot\n"));
        assert!(output.contains("digraph { A -> B; }"));
    }
}
