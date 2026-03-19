//! Mermaid diagram rendering for terminals.
//!
//! This module provides a thin adapter over `biscuit-visualized` for rendering
//! Mermaid diagrams in the terminal. It delegates rendering to `biscuit-visualized`
//! and adds terminal-specific display logic using `viuer`.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use biscuit_terminal::components::mermaid::MermaidRenderer;
//!
//! fn example() -> Result<(), biscuit_terminal::components::mermaid::MermaidRenderError> {
//!     let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
//!     renderer.render_for_terminal()?;
//!     Ok(())
//! }
//! ```

use std::path::PathBuf;

use thiserror::Error;

// Re-export types from biscuit-visualized
pub use biscuit_visualized::mermaid::{MermaidConfig, MermaidTheme, QuadrantTheme};

/// Errors that can occur during terminal rendering of Mermaid diagrams.
#[derive(Error, Debug)]
pub enum MermaidRenderError {
    /// Visualization error from biscuit-visualized
    #[error(transparent)]
    Visualization(#[from] biscuit_visualized::mermaid::MermaidError),
    /// Terminal does not support inline images
    #[error("Terminal does not support inline images")]
    NoImageSupport,
    /// Image display failed
    #[error("Image display failed: {0}")]
    DisplayError(String),
}

/// A Mermaid diagram renderer for terminal output.
///
/// This struct wraps `biscuit_visualized::mermaid::MermaidDiagram` and adds
/// terminal-specific display capabilities using `viuer`.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_terminal::components::mermaid::{MermaidRenderer, MermaidTheme, MermaidConfig};
///
/// // Basic usage with default settings (dark theme)
/// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
///     .with_theme(MermaidTheme::Forest)
///     .with_title("My Flowchart")
///     .with_config(MermaidConfig::new().with_point_label_font_size(16));
///
/// // Render to terminal (requires capable terminal)
/// if let Err(e) = renderer.render_for_terminal() {
///     // Fall back to code block on incapable terminals
///     println!("{}", renderer.fallback_code_block());
/// }
/// ```
///
/// ## Supported Diagram Types
///
/// - Flowcharts (flowchart, graph)
/// - Sequence diagrams
/// - Class diagrams
/// - State diagrams
/// - Entity relationship diagrams
/// - Gantt charts
/// - Pie charts
/// - Quadrant charts
/// - And more (see Mermaid documentation)
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
#[derive(Debug, Clone)]
pub struct MermaidRenderer {
    /// The inner diagram from biscuit-visualized
    diagram: biscuit_visualized::mermaid::MermaidDiagram,
    /// Scale factor for output resolution (default: 2)
    scale: u32,
    /// Use transparent background
    transparent_background: bool,
}

impl MermaidRenderer {
    /// Creates a new MermaidRenderer with the given diagram instructions.
    ///
    /// Uses default settings: dark theme.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    /// ```
    pub fn new<S: Into<String>>(instructions: S) -> Self {
        Self {
            diagram: biscuit_visualized::mermaid::MermaidDiagram::new(instructions),
            scale: 2,
            transparent_background: false,
        }
    }

    /// Creates a MermaidRenderer configured for the current terminal.
    ///
    /// Automatically detects color mode and sets appropriate theme.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::for_terminal("flowchart LR\n    A --> B");
    /// // Theme is automatically configured based on terminal color mode
    /// ```
    pub fn for_terminal<S: Into<String>>(instructions: S) -> Self {
        use crate::terminal::Terminal;

        let color_mode = Terminal::color_mode();
        let is_dark = matches!(
            color_mode,
            crate::discovery::detection::ColorMode::Dark
                | crate::discovery::detection::ColorMode::Unknown
        );

        Self {
            diagram: biscuit_visualized::mermaid::MermaidDiagram::new(instructions)
                .with_theme(MermaidTheme::for_color_mode(is_dark)),
            scale: 2,
            transparent_background: true,
        }
    }

    /// Sets the theme for rendering.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::{MermaidRenderer, MermaidTheme};
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_theme(MermaidTheme::Neutral);
    /// ```
    /// Sets the scale factor for output resolution.
    ///
    /// Higher values produce sharper images but larger files.
    /// Default is 2 (good for most modern displays).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_scale(3); // Extra sharp
    /// ```
    pub fn with_scale(mut self, scale: u32) -> Self {
        self.scale = scale.max(1); // Minimum scale of 1
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
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_transparent_background(true);
    /// ```
    pub fn with_transparent_background(mut self, transparent: bool) -> Self {
        self.transparent_background = transparent;
        self
    }

    /// Sets the theme for rendering.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::{MermaidRenderer, MermaidTheme};
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_theme(MermaidTheme::Neutral);
    /// ```
    pub fn with_theme(mut self, theme: MermaidTheme) -> Self {
        self.diagram = self.diagram.with_theme(theme);
        self
    }

    /// Sets the title for the diagram (used for alt text).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_title("My Flowchart");
    /// ```
    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.diagram = self.diagram.with_title(title);
        self
    }

    /// Sets additional Mermaid configuration options.
    ///
    /// These options are passed to the renderer via configuration.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::{MermaidRenderer, MermaidConfig};
    ///
    /// let config = MermaidConfig::new()
    ///     .with_point_label_font_size(16)
    ///     .with_point_radius(10);
    ///
    /// let renderer = MermaidRenderer::new("quadrantChart\n    A: [0.5, 0.5]")
    ///     .with_config(config);
    /// ```
    pub fn with_config(mut self, config: MermaidConfig) -> Self {
        self.diagram = self.diagram.with_config(config);
        self
    }

    /// Returns the diagram instructions.
    pub fn instructions(&self) -> &str {
        self.diagram.instructions()
    }

    /// Returns a fallback code block string for the diagram.
    ///
    /// This is used when terminal rendering fails or is not supported.
    /// Returns the instructions formatted as a fenced code block.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    /// let fallback = renderer.fallback_code_block();
    /// assert!(fallback.contains("```mermaid"));
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

    /// Checks if the current terminal supports image rendering.
    ///
    /// Returns `true` if either Kitty or iTerm2 image protocols are supported.
    pub fn terminal_supports_images() -> bool {
        use crate::discovery::detection::ImageSupport;
        use crate::terminal::Terminal;

        let term = Terminal::new();
        !matches!(term.image_support, ImageSupport::None)
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
    /// Returns `MermaidRenderError` if:
    /// - Terminal doesn't support image rendering
    /// - Diagram rendering fails (invalid syntax, etc.)
    /// - Image display fails
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// fn example() -> Result<(), biscuit_terminal::components::mermaid::MermaidRenderError> {
    ///     let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    ///     renderer.render_for_terminal()?;
    ///     Ok(())
    /// }
    /// ```
    #[tracing::instrument(skip(self))]
    pub fn render_for_terminal(&self) -> Result<(), MermaidRenderError> {
        // Check terminal support
        if !Self::terminal_supports_images() {
            tracing::debug!("Terminal does not support image rendering");
            return Err(MermaidRenderError::NoImageSupport);
        }

        // Render to PNG using biscuit-visualized
        let request = biscuit_visualized::artifact::RenderRequest {
            format: biscuit_visualized::artifact::OutputFormat::Png,
            scale: self.scale,
            transparent_background: self.transparent_background,
        };

        let artifact = self.diagram.render(&request)?;

        tracing::info!(path = ?artifact.path, cache_hit = artifact.cache_hit, "Rendered diagram");

        // Display with viuer
        let config = viuer::Config {
            absolute_offset: false,
            ..Default::default()
        };

        viuer::print_from_file(&artifact.path, &config)
            .map_err(|e| MermaidRenderError::DisplayError(e.to_string()))?;

        tracing::debug!("Displayed diagram in terminal");

        Ok(())
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
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    /// let (path, cache_hit) = renderer.render_to_cached_png()?;
    /// println!("Rendered to: {:?} (cache hit: {})", path, cache_hit);
    /// # Ok::<(), biscuit_terminal::components::mermaid::MermaidRenderError>(())
    /// ```
    #[tracing::instrument(skip(self))]
    pub fn render_to_cached_png(&self) -> Result<(PathBuf, bool), MermaidRenderError> {
        let request = biscuit_visualized::artifact::RenderRequest {
            format: biscuit_visualized::artifact::OutputFormat::Png,
            scale: self.scale,
            transparent_background: self.transparent_background,
        };

        let artifact = self.diagram.render(&request)?;

        tracing::info!(path = ?artifact.path, cache_hit = artifact.cache_hit, "Rendered diagram to PNG");

        Ok((artifact.path, artifact.cache_hit))
    }
}

impl From<String> for MermaidRenderer {
    fn from(instructions: String) -> Self {
        Self::new(instructions)
    }
}

impl From<&str> for MermaidRenderer {
    fn from(instructions: &str) -> Self {
        Self::new(instructions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mermaid_renderer_new() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
        assert_eq!(renderer.instructions(), "flowchart LR\n    A --> B");
    }

    #[test]
    fn test_mermaid_renderer_with_title() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B").with_title("My Flowchart");
        // Title is stored internally, no direct getter in the adapter
        assert_eq!(renderer.instructions(), "flowchart LR\n    A --> B");
    }

    #[test]
    fn test_mermaid_renderer_from_string() {
        let instructions = String::from("flowchart LR\n    A --> B");
        let renderer = MermaidRenderer::from(instructions.clone());
        assert_eq!(renderer.instructions(), "flowchart LR\n    A --> B");
    }

    #[test]
    fn test_mermaid_renderer_from_str() {
        let renderer = MermaidRenderer::from("flowchart LR\n    A --> B");
        assert_eq!(renderer.instructions(), "flowchart LR\n    A --> B");
    }

    #[test]
    fn test_mermaid_renderer_clone() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B").with_title("Test");
        let cloned = renderer.clone();
        assert_eq!(renderer.instructions(), cloned.instructions());
    }

    #[test]
    fn test_fallback_code_block() {
        let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
        let output = renderer.fallback_code_block();
        assert!(output.starts_with("```mermaid\n"));
        assert!(output.ends_with("\n```"));
        assert!(output.contains("A --> B"));
    }
}
