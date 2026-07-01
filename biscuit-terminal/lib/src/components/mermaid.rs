//! Mermaid diagram rendering for terminals.
//!
//! This module provides [`MermaidDiagram`], a composable [`TerminalRenderable`] component
//! for rendering Mermaid diagrams inline in the terminal. It delegates rendering
//! to `biscuit-visualized` and displays via the terminal's image protocol.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use biscuit_terminal::components::mermaid::MermaidDiagram;
//! use biscuit_terminal::terminal::Terminal;
//!
//! let diagram = MermaidDiagram::new("flowchart LR\n    A --> B");
//! let term = Terminal::new();
//!
//! // Fallible path with metadata
//! match diagram.try_render(&term) {
//!     Ok(result) => print!("{}", result.output),
//!     Err(e) => eprintln!("Render failed: {}", e),
//! }
//! ```

use std::path::PathBuf;

use thiserror::Error;

use crate::components::renderable::TerminalRenderable;
use crate::utils::layout::LayoutTerminalExt;

use renderable::tree::{RenderNode, TreeRenderable};

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
/// ```ignore
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
///
/// ## Note
///
/// For composable rendering, prefer [`MermaidDiagram`] which implements
/// [`TerminalRenderable`] and provides proper error handling via
/// [`try_render()`](MermaidDiagram::try_render).
#[derive(Debug, Clone)]
pub(crate) struct MermaidRenderer {
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
    /// ```ignore
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
    /// ```ignore
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::for_terminal("flowchart LR\n    A --> B");
    /// // Theme is automatically configured based on terminal color mode
    /// ```
    pub fn for_terminal<S: Into<String>>(instructions: S) -> Self {
        let color_mode = crate::discovery::detection::color_mode();
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
    /// ```ignore
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
    /// ```ignore
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
    /// ```ignore
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
    /// ```ignore
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
    /// ```ignore
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
    /// ```ignore
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
    /// ```ignore
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    /// let fallback = renderer.fallback_code_block();
    /// assert!(fallback.contains("```mermaid"));
    /// ```
    pub fn fallback_code_block(&self) -> String {
        self.diagram.fallback_code_block()
    }

    /// Renders the diagram to an SVG string.
    ///
    /// Delegates to `biscuit-visualized`'s Mermaid renderer and reads the
    /// cached SVG file into a string. This is the browser static-SVG path
    /// for Mermaid promotion.
    ///
    /// ## Errors
    ///
    /// Returns error if diagram rendering or file I/O fails.
    pub fn render_to_svg(&self) -> Result<String, MermaidRenderError> {
        let request = biscuit_visualized::artifact::RenderRequest {
            format: biscuit_visualized::artifact::OutputFormat::Svg,
            scale: self.scale,
            target_width: None,
            transparent_background: self.transparent_background,
        };
        let artifact = self.diagram.render(&request)?;
        std::fs::read_to_string(&artifact.path)
            .map_err(|e| MermaidRenderError::DisplayError(e.to_string()))
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
    /// ```ignore
    /// use biscuit_terminal::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    /// let (path, cache_hit) = renderer.render_to_cached_png()?;
    /// println!("Rendered to: {:?} (cache hit: {})", path, cache_hit);
    /// # Ok::<(), biscuit_terminal::components::mermaid::MermaidRenderError>(())
    /// ```
    /// Renders the diagram to a cached PNG at the specified target pixel width.
    ///
    /// Height is derived from the SVG's aspect ratio. The rasterizer renders
    /// the SVG fresh at `target_width` pixels — no oversampling, no
    /// downstream downscaling.
    ///
    /// ## Errors
    ///
    /// Returns error if diagram rendering or file I/O fails.
    #[tracing::instrument(skip(self))]
    pub fn render_to_cached_png_at_width(
        &self,
        target_width: u32,
    ) -> Result<(PathBuf, bool), MermaidRenderError> {
        let request = biscuit_visualized::artifact::RenderRequest {
            format: biscuit_visualized::artifact::OutputFormat::Png,
            scale: self.scale,
            target_width: Some(target_width),
            transparent_background: self.transparent_background,
        };

        let artifact = self.diagram.render(&request)?;

        tracing::info!(
            path = ?artifact.path,
            cache_hit = artifact.cache_hit,
            target_width,
            "Rendered diagram to PNG at target width"
        );

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

/// A composable Mermaid diagram that implements [`TerminalRenderable`].
///
/// `MermaidDiagram` wraps [`MermaidRenderer`] and makes diagrams usable
/// anywhere a `TerminalRenderable` is expected — inside `Section`, `Compose`,
/// lists, or any other container component.
///
/// The render pipeline is:
/// 1. Render Mermaid instructions to a cached PNG via `MermaidRenderer`
/// 2. Load the PNG into a [`TerminalImage`]
/// 3. Render the image inline using the terminal's image protocol
///
/// When image rendering is unavailable (no terminal support, render
/// failure, etc.) the component falls back to a fenced code block
/// of the Mermaid source.
///
/// ## Examples
///
/// ```rust
/// use biscuit_terminal::components::mermaid::{MermaidDiagram, MermaidTheme};
/// use biscuit_terminal::components::renderable::TerminalRenderable;
/// use biscuit_terminal::components::terminal_image::ImageWidth;
///
/// let diagram = MermaidDiagram::new("flowchart LR\n    A --> B --> C")
///     .with_theme(MermaidTheme::Forest)
///     .with_title("My Flow")
///     .with_width(ImageWidth::Percent(0.6));
///
/// /// Use in any TerminalRenderable context
/// let term = biscuit_terminal::terminal::Terminal::default();
/// let output = diagram.render(&term);
/// ```
///
/// ## Layout & Style Contract
///
/// `MermaidDiagram` is a bespoke image-protocol component (spec C5/D5). The
/// external Mermaid → PNG rendering is irreducible — no render-tree
/// `NodeKind` can represent a rasterized diagram — so the component emits
/// the rendered image through `TerminalImage` and the underlying image
/// protocol (Kitty / iTerm2 / Sixel). The fold-style box model does not
/// apply to the rendered image itself; the box placement is shared.
///
/// The honored subset (spec C5: minimum bar) is the outer placement,
/// resolved by `TerminalImage::resolve_dimensions_for`:
///
/// | Property | Status | Rationale |
/// |----------|--------|-----------|
/// | `Layout::margin` | **Honored** | Reduces `available_width` before the image width is resolved, so a wider margin shrinks the rendered canvas. |
/// | `Layout::alignment` | **Honored** | Selects left / center / right placement of the rendered image within the slack (applied via `apply_block_layout`). |
/// | `Layout::width`, `Layout::max_width` | **N/A** | The rendered canvas is sized by `ImageWidth` (the explicit contract for this component). Reusing `Layout::width` would create two competing width controls. This is the documented MermaidDiagram-specific carve-out, not a silent no-op. |
/// | `Layout::padding` | **N/A** | The rendered image has no padding box. |
/// | `Layout::word_wrap` | **N/A** | A rasterized diagram cannot wrap. |
/// | `Style::color` / `emphasis` | **N/A** | The diagram is rasterized pixels, not text the SGR can recolor. Theme selection happens via `MermaidTheme`. |
/// | `Style::background` | **N/A** | A block background would have to paint cells *around* the protocol bytes; the protocol owns the cells it covers. Use `with_transparent_background(false)` to control the rasterizer's own background. |
/// | `Style::border` | **N/A** | Box-drawing glyphs cannot frame an image-protocol escape without corrupting the protocol's cell coverage. |
///
/// Parity for the honored subset is pinned in `mermaid_parity.rs`.
#[derive(Debug, Clone)]
pub struct MermaidDiagram {
    renderer: MermaidRenderer,
    width: super::terminal_image::ImageWidth,
    layout: crate::utils::layout::Layout,
}

impl MermaidDiagram {
    /// Creates a new `MermaidDiagram` with the given Mermaid instructions.
    ///
    /// Automatically configures theme based on terminal color mode and
    /// uses a transparent background. Default width is 50%.
    pub fn new<S: Into<String>>(instructions: S) -> Self {
        Self {
            renderer: MermaidRenderer::for_terminal(instructions),
            width: super::terminal_image::ImageWidth::Percent(0.5),
            layout: crate::utils::layout::Layout::default(),
        }
    }

    /// Sets the display width for the rendered image.
    pub fn with_width(mut self, width: super::terminal_image::ImageWidth) -> Self {
        self.width = width;
        self
    }

    /// Sets the Mermaid theme.
    pub fn with_theme(mut self, theme: MermaidTheme) -> Self {
        self.renderer = self.renderer.with_theme(theme);
        self
    }

    /// Sets the diagram title.
    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.renderer = self.renderer.with_title(title);
        self
    }

    /// Sets the Mermaid configuration.
    pub fn with_config(mut self, config: MermaidConfig) -> Self {
        self.renderer = self.renderer.with_config(config);
        self
    }

    /// Sets the render scale factor (default: 2).
    pub fn with_scale(mut self, scale: u32) -> Self {
        self.renderer = self.renderer.with_scale(scale);
        self
    }

    /// Enables or disables transparent background.
    pub fn with_transparent_background(mut self, transparent: bool) -> Self {
        self.renderer = self.renderer.with_transparent_background(transparent);
        self
    }

    /// Uses inverted colors (solid background with opposite theme).
    pub fn inverted(mut self, is_dark_mode: bool) -> Self {
        let theme = MermaidTheme::for_color_mode(is_dark_mode).inverse();
        self.renderer = self
            .renderer
            .with_theme(theme)
            .with_transparent_background(false);
        self
    }

    /// Returns the underlying Mermaid instructions.
    pub fn instructions(&self) -> &str {
        self.renderer.instructions()
    }

    /// Returns the fallback code block for the diagram.
    pub fn fallback_code_block(&self) -> String {
        self.renderer.fallback_code_block()
    }

    /// Renders the diagram to an SVG string.
    ///
    /// Delegates to [`MermaidRenderer::render_to_svg`].
    pub fn render_to_svg(&self) -> Result<String, MermaidRenderError> {
        self.renderer.render_to_svg()
    }

    /// Renders the diagram to a terminal-displayable string.
    ///
    /// This is the **fallible** render path. Use this when you need to
    /// handle errors explicitly rather than silently falling back.
    ///
    /// ## Returns
    ///
    /// On success, returns a [`MermaidRenderResult`] containing the rendered
    /// output string along with metadata (PNG path, cache hit status).
    ///
    /// ## Errors
    ///
    /// Returns [`MermaidRenderError`] if:
    /// - The Mermaid instructions fail to render (invalid syntax, etc.)
    /// - The PNG file cannot be loaded as a terminal image
    /// - The terminal does not support inline images
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_terminal::components::mermaid::MermaidDiagram;
    /// use biscuit_terminal::terminal::Terminal;
    ///
    /// let diagram = MermaidDiagram::new("flowchart LR\n    A --> B");
    /// let term = Terminal::new();
    /// match diagram.try_render(&term) {
    ///     Ok(result) => {
    ///         print!("{}", result.output);
    ///         eprintln!("cache hit: {}", result.cache_hit);
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Render failed: {}", e);
    ///         println!("{}", diagram.fallback_code_block());
    ///     }
    /// }
    /// ```
    #[tracing::instrument(skip(self, term))]
    pub fn try_render(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<MermaidRenderResult, MermaidRenderError> {
        let (output, png_path, cache_hit, width_cells) = self.render_to_image(term)?;
        // Diagrams are inherently block-cohesive (image escapes or fenced code
        // fallback) — align as a block.
        let output = self.layout.apply_block_layout(&output, term.width());

        Ok(MermaidRenderResult {
            output,
            png_path,
            cache_hit,
            width_cells,
        })
    }

    /// Renders the diagram to a terminal image string without layout applied.
    ///
    /// Returns the raw image output on success, or the fallback code block
    /// on failure.
    fn render_raw(&self, term: &crate::terminal::Terminal) -> String {
        match self.render_to_image(term) {
            Ok((output, _, _, _)) => output,
            Err(_) => self.renderer.fallback_code_block(),
        }
    }

    /// Core render pipeline: Mermaid → PNG → terminal image string.
    ///
    /// Returns `(output, png_path, cache_hit, width_cells)` on success.
    /// `width_cells` is the resolved image width in terminal columns,
    /// after applying margins and the configured [`ImageWidth`] spec.
    fn render_to_image(
        &self,
        term: &crate::terminal::Terminal,
    ) -> Result<(String, PathBuf, bool, u32), MermaidRenderError> {
        // Translate the configured display width (cells) into pixels using the
        // terminal's detected cell size, and have the rasterizer render the
        // SVG once at that exact pixel width. No oversampling, no downstream
        // downscaling — text glyphs are rasterised at the display resolution.
        let dims = super::terminal_image::TerminalImage::resolve_dimensions_for(
            &self.width,
            &self.layout,
            term.width(),
        );
        let cell_pixel_width = term.cell_size().map(|cs| cs.width.max(1)).unwrap_or(8u32);
        let target_width_px = (dims.image_width.max(1)) * cell_pixel_width;

        let (png_path, cache_hit) = self
            .renderer
            .render_to_cached_png_at_width(target_width_px)?;

        let term_image = super::terminal_image::TerminalImage::new(&png_path)
            .map_err(|e| MermaidRenderError::DisplayError(e.to_string()))?
            .with_width(self.width.clone());

        let width_cells = dims.image_width;

        let output = term_image.render(term);
        if output.is_empty() {
            return Err(MermaidRenderError::NoImageSupport);
        }

        Ok((output, png_path, cache_hit, width_cells))
    }
}

/// Successful render result from [`MermaidDiagram::try_render()`].
#[derive(Debug, Clone)]
pub struct MermaidRenderResult {
    /// The rendered terminal output string.
    pub output: String,
    /// Path to the cached PNG file.
    pub png_path: PathBuf,
    /// Whether the PNG was served from cache.
    pub cache_hit: bool,
    /// Number of pane columns the rendered image occupies, after
    /// resolving margins and the configured width spec (percent,
    /// fill, or absolute cells).
    pub width_cells: u32,
}

impl super::renderable::TerminalRenderable for MermaidDiagram {
    fn render(&self, term: &crate::terminal::Terminal) -> String {
        let content = self.render_raw(term);
        self.layout.apply_block_layout(&content, term.width())
    }

    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = crate::terminal::Terminal::new_optimistic(width);
        let content = self.render_raw(&term);
        self.layout.apply_block_layout(&content, width)
    }

    fn is_block_level(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &crate::utils::layout::Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut crate::utils::layout::Layout {
        &mut self.layout
    }
}

impl TreeRenderable for MermaidDiagram {
    /// Projects the rendered diagram into a [`NodeKind::Paragraph`] wrapping
    /// an inline [`NodeKind::Image`] and carrying the component's outer-box
    /// layout (margin, alignment).
    ///
    /// The externally rendered Mermaid PNG is irreducible (spec C5/D5); the
    /// structural image node carries placement and alt text with the Mermaid
    /// source, so Browser/Markdown targets degrade to a readable placeholder.
    /// The `url` is left empty because the rendered PNG is produced on demand
    /// and has no stable address until rendered to cache.
    ///
    /// [`NodeKind::Image`]: renderable::tree::NodeKind::Image
    /// [`NodeKind::Paragraph`]: renderable::tree::NodeKind::Paragraph
    fn render_tree(&self) -> RenderNode {
        let alt = self.fallback_code_block();
        let image = RenderNode::image("", None, &alt);
        let mut node = RenderNode::paragraph(vec![image]);
        node.attrs.set_layout(&self.layout);
        node
    }
}

impl From<String> for MermaidDiagram {
    fn from(instructions: String) -> Self {
        Self::new(instructions)
    }
}

impl From<&str> for MermaidDiagram {
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

    // ── MermaidDiagram tests ──────────────────────────────────

    #[test]
    fn diagram_new_preserves_instructions() {
        let diagram = MermaidDiagram::new("flowchart LR\n    A --> B");
        assert_eq!(diagram.instructions(), "flowchart LR\n    A --> B");
    }

    #[test]
    fn diagram_from_string() {
        let diagram = MermaidDiagram::from("pie\n    Dogs: 50".to_string());
        assert_eq!(diagram.instructions(), "pie\n    Dogs: 50");
    }

    #[test]
    fn diagram_from_str() {
        let diagram = MermaidDiagram::from("pie\n    Dogs: 50");
        assert_eq!(diagram.instructions(), "pie\n    Dogs: 50");
    }

    #[test]
    fn diagram_builder_methods_chain() {
        use crate::components::terminal_image::ImageWidth;

        let diagram = MermaidDiagram::new("flowchart LR\n    X --> Y")
            .with_title("Test")
            .with_theme(MermaidTheme::Neutral)
            .with_width(ImageWidth::Percent(0.75))
            .with_scale(3)
            .with_transparent_background(false);
        assert_eq!(diagram.instructions(), "flowchart LR\n    X --> Y");
    }

    #[test]
    fn diagram_is_block_level() {
        let diagram = MermaidDiagram::new("flowchart LR\n    A --> B");
        assert!(diagram.is_block_level());
    }

    #[test]
    fn diagram_clone() {
        let diagram = MermaidDiagram::new("flowchart LR\n    A --> B").with_title("Clone Test");
        let cloned = diagram.clone();
        assert_eq!(diagram.instructions(), cloned.instructions());
    }

    /// Locks in the `width_cells` resolution path: when the renderer
    /// succeeds, `MermaidRenderResult.width_cells` must reflect the
    /// resolved `ImageWidth` spec.
    #[test]
    fn diagram_try_render_populates_width_cells() {
        use crate::components::terminal_image::ImageWidth;

        let diagram = MermaidDiagram::new("pie\n    A: 1").with_width(ImageWidth::Percent(0.5));
        let term = crate::terminal::Terminal::new_optimistic(80);

        match diagram.try_render(&term) {
            Ok(result) => {
                // Default layout has zero margins, so 50% of 80 cols = 40.
                assert_eq!(
                    result.width_cells, 40,
                    "50% of an 80-column terminal should resolve to 40 cells"
                );
            }
            Err(e) => {
                // If the host lacks Mermaid rendering dependencies, skip
                // the assertion rather than fail — the Level-2 tests
                // exercise the full pipeline.
                eprintln!("Mermaid render unavailable in unit-test env: {e}");
            }
        }
    }

    /// `render_to_svg` must produce an SVG string containing expected Mermaid
    /// elements, or gracefully skip when the host lacks rendering deps.
    #[test]
    fn diagram_render_to_svg_produces_svg() {
        let diagram = MermaidDiagram::new("flowchart LR\n    A --> B");
        match diagram.render_to_svg() {
            Ok(svg) => {
                assert!(
                    svg.contains("<svg"),
                    "render_to_svg must produce an <svg> element, got: {svg}"
                );
                assert!(
                    svg.contains("</svg>"),
                    "render_to_svg must close the <svg> element, got: {svg}"
                );
            }
            Err(e) => {
                // If the host lacks Mermaid rendering dependencies, skip
                // rather than fail — the Level-2 tests exercise the full
                // pipeline.
                eprintln!("Mermaid SVG render unavailable in unit-test env: {e}");
            }
        }
    }
}
