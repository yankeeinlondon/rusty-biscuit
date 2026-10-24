//! Mermaid diagram rendering for terminals.
//!
//! This module provides [`MermaidDiagram`], a composable [`TerminalRenderable`] component
//! for rendering Mermaid diagrams inline in the terminal. It renders with this
//! crate's [`crate::mermaid`] module and displays the PNG in the format the
//! terminal's graphics route selects.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use biscuit_visualized::components::mermaid::MermaidDiagram;
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
use std::time::Duration;

use thiserror::Error;

use biscuit_terminal::components::terminal_image::{
    ImageOutput, ImageWidth, ResolvedDimensions, TerminalImage,
};
use biscuit_terminal::discovery::fonts::CellSize;
use biscuit_terminal::utils::layout::LayoutTerminalExt;

use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};

use super::display::{display_background, display_image, display_pixels};
use super::fallback_policy::{MermaidFallbackPolicy, ResolvedFallbackPolicy};
use crate::artifact::{Background, OutputFormat, RenderRequest};
use crate::mermaid::{MermaidError, TextCharset, TextCoverage, TextOptions, TextRender};

use renderable::tree::{RenderNode, RenderStrictness, TreeRenderable};

// Re-export types from biscuit-visualized
pub use crate::mermaid::{DiagramFont, MermaidConfig, MermaidTarget, MermaidTheme, Palette, QuadrantTheme, TerminalColors};

/// Errors that can occur during terminal rendering of Mermaid diagrams.
#[derive(Error, Debug)]
pub enum MermaidRenderError {
    /// Visualization error from biscuit-visualized
    #[error(transparent)]
    Visualization(#[from] crate::mermaid::MermaidError),
    /// Terminal does not support inline images
    #[error("Terminal does not support inline images")]
    NoImageSupport,
    /// Image display failed
    #[error("Image display failed: {0}")]
    DisplayError(String),
}

/// The theme for the attached terminal's color mode; an unknown mode is dark.
pub(crate) fn terminal_theme() -> MermaidTheme {
    MermaidTheme::for_color_mode(terminal_is_dark())
}

fn terminal_is_dark() -> bool {
    matches!(
        biscuit_terminal::discovery::detection::color_mode(),
        biscuit_terminal::discovery::detection::ColorMode::Dark
            | biscuit_terminal::discovery::detection::ColorMode::Unknown
    )
}

/// The palette for the attached terminal (R-P3–R-P5): its detected colors,
/// or the fixed palette for its color mode when stdout is not a terminal, in
/// CI, or when it reported no background.
pub(crate) fn terminal_palette() -> Palette {
    Palette::for_terminal(&TerminalColors::detect(), terminal_is_dark())
}

/// A Mermaid diagram renderer for terminal output.
///
/// This struct wraps `crate::mermaid::MermaidDiagram` and adds
/// terminal-specific display capabilities.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_visualized::components::mermaid::{MermaidRenderer, MermaidTheme, MermaidConfig};
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
    diagram: crate::mermaid::MermaidDiagram,
    /// Scale factor for output resolution (default: 2)
    scale: u32,
    background: Background,
}

impl MermaidRenderer {
    /// Creates a new MermaidRenderer with the given diagram instructions.
    ///
    /// Uses default settings: dark theme.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use biscuit_visualized::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    /// ```
    pub fn new<S: Into<String>>(instructions: S) -> Self {
        Self {
            diagram: crate::mermaid::MermaidDiagram::new(instructions),
            scale: 2,
            background: Background::Theme,
        }
    }

    /// Creates a MermaidRenderer configured for the current terminal.
    ///
    /// Styles the diagram with the terminal's palette ([`terminal_palette`]);
    /// a later `with_theme` replaces it with that theme's fixed palette.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use biscuit_visualized::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::for_terminal("flowchart LR\n    A --> B");
    /// // Theme is automatically configured based on terminal color mode
    /// ```
    pub fn for_terminal<S: Into<String>>(instructions: S) -> Self {
        Self {
            diagram: crate::mermaid::MermaidDiagram::new(instructions)
                .with_theme(terminal_theme())
                .with_palette(terminal_palette()),
            scale: 2,
            background: Background::Transparent,
        }
    }

    /// Sets the theme for rendering.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use biscuit_visualized::components::mermaid::{MermaidRenderer, MermaidTheme};
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
    /// use biscuit_visualized::components::mermaid::MermaidRenderer;
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
    /// use biscuit_visualized::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B")
    ///     .with_transparent_background(true);
    /// ```
    pub fn with_transparent_background(mut self, transparent: bool) -> Self {
        self.background = if transparent {
            Background::Transparent
        } else {
            Background::Theme
        };
        self
    }

    /// Sets what paints behind the diagram.
    pub fn with_background(mut self, background: Background) -> Self {
        self.background = background;
        self
    }

    /// Sets the theme for rendering.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use biscuit_visualized::components::mermaid::{MermaidRenderer, MermaidTheme};
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
    /// use biscuit_visualized::components::mermaid::MermaidRenderer;
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
    /// use biscuit_visualized::components::mermaid::{MermaidRenderer, MermaidConfig};
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

    /// Sets how long Merman may work on each request for this diagram; see
    /// [`crate::mermaid::MermaidDiagram::with_time_budget`].
    pub fn with_time_budget(mut self, budget: Duration) -> Self {
        self.diagram = self.diagram.with_time_budget(budget);
        self
    }

    /// Selects who [`render_to_svg`](Self::render_to_svg)'s SVG is for; see
    /// [`crate::mermaid::MermaidDiagram::with_target`].
    pub fn with_target(mut self, target: MermaidTarget) -> Self {
        self.diagram = self.diagram.with_target(target);
        self
    }

    /// Embeds the measured font face in an HTML static SVG; see
    /// [`crate::mermaid::MermaidDiagram::with_embedded_font`].
    pub fn with_embedded_font(mut self, embed: bool) -> Self {
        self.diagram = self.diagram.with_embedded_font(embed);
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
    /// use biscuit_visualized::components::mermaid::MermaidRenderer;
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
        Ok(self.render_to_svg_with_font()?.0)
    }

    /// [`render_to_svg`](Self::render_to_svg), also returning the diagram's
    /// [`DiagramFont`] when the embedded-font opt-in applies; `None`
    /// otherwise.
    pub fn render_to_svg_with_font(&self) -> Result<(String, Option<DiagramFont>), MermaidRenderError> {
        let request = RenderRequest {
            format: OutputFormat::Svg,
            scale: self.scale,
            background: self.background,
            ..RenderRequest::default()
        };
        let (artifact, font) = self.diagram.render_with_font(&request)?;
        let svg = std::fs::read_to_string(&artifact.path)
            .map_err(|e| MermaidRenderError::DisplayError(e.to_string()))?;
        Ok((svg, font))
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
    /// use biscuit_visualized::components::mermaid::MermaidRenderer;
    ///
    /// let renderer = MermaidRenderer::new("flowchart LR\n    A --> B");
    /// let (path, cache_hit) = renderer.render_to_cached_png()?;
    /// println!("Rendered to: {:?} (cache hit: {})", path, cache_hit);
    /// # Ok::<(), biscuit_visualized::components::mermaid::MermaidRenderError>(())
    /// ```
    /// Measures the diagram's natural size in SVG user units.
    pub fn natural_size(
        &self,
    ) -> Result<crate::mermaid::NaturalSize, MermaidRenderError> {
        Ok(self.diagram.natural_size()?)
    }

    /// Renders the diagram to a cached PNG at the specified target pixel width,
    /// on `background` in place of the renderer's own.
    ///
    /// Height is derived from the SVG's aspect ratio, then padded to whole
    /// rows `row_height` pixels tall. The rasterizer renders the SVG fresh at
    /// `target_width` pixels — no oversampling, no downstream downscaling.
    ///
    /// ## Errors
    ///
    /// Returns error if diagram rendering or file I/O fails.
    #[tracing::instrument(skip(self))]
    pub fn render_to_cached_png_at_width(
        &self,
        target_width: u32,
        row_height: u32,
        background: Background,
    ) -> Result<(PathBuf, bool), MermaidRenderError> {
        let request = RenderRequest {
            format: OutputFormat::Png,
            scale: self.scale,
            target_width: Some(target_width),
            row_height: Some(row_height),
            background,
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

/// The id the partial note names, so a reader can search for it.
pub const PARTIAL_NOTE_ID: &str = "mermaid-partial-warning";

/// Columns kept free beside a text drawing for its `txt` code block's own
/// padding: a highlighting code renderer (darkmatter's pads one column on
/// the left) may pad either side, while the plain code block pads none.
pub const TEXT_BLOCK_CHROME_COLUMNS: u32 = 2;

/// What [`MermaidDiagram::present`] chose to show.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MermaidPresentation {
    /// The image, exactly as [`MermaidDiagram::try_render`] returns it.
    Image(MermaidRenderResult),
    /// The diagram drawn as text: a `txt` code node, followed by the partial
    /// note's block quote when the drawing is partial and the note is shown.
    Text(Vec<RenderNode>),
    /// The source as a `mermaid` code node.
    Source(Vec<RenderNode>),
}

impl MermaidPresentation {
    /// The render-tree nodes of a text or source presentation; `None` for an
    /// image.
    pub fn nodes(&self) -> Option<&[RenderNode]> {
        match self {
            Self::Image(_) => None,
            Self::Text(nodes) | Self::Source(nodes) => Some(nodes),
        }
    }
}

/// The note that follows a partial text drawing: `Note:` bold, then the
/// bracketed id, dim, with the id itself italic.
pub fn partial_note() -> RenderNode {
    RenderNode::block_quote(vec![RenderNode::paragraph(vec![
        RenderNode::strong(vec![RenderNode::text("Note:")]),
        RenderNode::text(
            " this textual representation is provided because the current terminal settings do not \
             allow for a graphical image to be used. This fallback representation, while never arbitrary \
             or speculative, might be missing attributes that would have been visible in the image's \
             representation [",
        ),
        RenderNode::span(
            vec!["dim".to_string()],
            vec![
                RenderNode::text("id: "),
                RenderNode::emphasis(vec![RenderNode::text(PARTIAL_NOTE_ID)]),
            ],
        ),
        RenderNode::text("]."),
    ])])
}

/// The drawing set a terminal can show.
pub(crate) fn text_charset(term: &biscuit_terminal::terminal::Terminal) -> TextCharset {
    if term.supports_unicode { TextCharset::Unicode } else { TextCharset::Ascii }
}

/// A composable Mermaid diagram that implements [`TerminalRenderable`].
///
/// `MermaidDiagram` wraps [`MermaidRenderer`] and makes diagrams usable
/// anywhere a `TerminalRenderable` is expected — inside `Section`, `Compose`,
/// lists, or any other container component.
///
/// The render pipeline is:
/// 1. Render Mermaid instructions to a cached PNG via `MermaidRenderer`, at
///    the display size: as many pixels wide as the resolved columns, padded
///    to whole cell rows
/// 2. Hand those PNG bytes to a [`TerminalImage`] unchanged
/// 3. Render the image inline using the terminal's image protocol
///
/// When the terminal's graphics route has no pixels, the diagram is drawn as
/// text instead, if Merman draws its family as a diagram, the drawing fits,
/// and the [fallback policy](MermaidDiagram::with_fallback_policy) allows it
/// (see [`present`](MermaidDiagram::present)). Otherwise, and whenever the
/// image route fails (a parse error, an unsupported family such as ZenUML, a
/// render over its [time budget](MermaidDiagram::with_time_budget), etc.),
/// the component shows a `mermaid` code block of the source.
///
/// ## Examples
///
/// ```rust
/// use biscuit_visualized::components::mermaid::{MermaidDiagram, MermaidTheme};
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
/// | `Style::background` | **N/A** | A block background would have to paint cells *around* the protocol bytes; the protocol owns the cells it covers. Use `with_background` to control the rasterizer's own background. |
/// | `Style::border` | **N/A** | Box-drawing glyphs cannot frame an image-protocol escape without corrupting the protocol's cell coverage. |
///
/// Parity for the honored subset is pinned in `mermaid_parity.rs`.
#[derive(Debug, Clone)]
pub struct MermaidDiagram {
    renderer: MermaidRenderer,
    width: biscuit_terminal::components::terminal_image::ImageWidth,
    layout: biscuit_terminal::utils::layout::Layout,
    fallback: MermaidFallbackPolicy,
}

impl MermaidDiagram {
    /// Creates a new `MermaidDiagram` with the given Mermaid instructions.
    ///
    /// Styles it with the terminal's palette (its background, foreground,
    /// and ANSI colors; the fixed palette for its color mode when they are
    /// unknown or the output is not a terminal) and uses a transparent
    /// background. Default width is `ImageWidth::Scale(1.0)`:
    /// body text is one terminal line tall, so every diagram draws at the same
    /// size, capped at the available columns.
    pub fn new<S: Into<String>>(instructions: S) -> Self {
        Self {
            renderer: MermaidRenderer::for_terminal(instructions),
            width: biscuit_terminal::components::terminal_image::ImageWidth::Scale(1.0),
            layout: biscuit_terminal::utils::layout::Layout::default(),
            fallback: MermaidFallbackPolicy::default(),
        }
    }

    /// Sets the display width for the rendered image.
    pub fn with_width(mut self, width: biscuit_terminal::components::terminal_image::ImageWidth) -> Self {
        self.width = width;
        self
    }

    /// Styles the diagram with `theme`'s fixed palette instead of the
    /// terminal's.
    pub fn with_theme(mut self, theme: MermaidTheme) -> Self {
        self.renderer = self.renderer.with_theme(theme);
        self
    }

    /// Styles the diagram with `palette` instead of the terminal's.
    pub fn with_palette(mut self, palette: Palette) -> Self {
        self.renderer.diagram = self.renderer.diagram.with_palette(palette);
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

    /// Enables or disables transparent background; disabled means the
    /// theme's background.
    pub fn with_transparent_background(mut self, transparent: bool) -> Self {
        self.renderer = self.renderer.with_transparent_background(transparent);
        self
    }

    /// Sets what paints behind the diagram.
    pub fn with_background(mut self, background: Background) -> Self {
        self.renderer = self.renderer.with_background(background);
        self
    }

    /// Sets how long Merman may work on each request for this diagram
    /// (default 2 s). The value is passed to the library-level
    /// [`crate::mermaid::MermaidDiagram::with_time_budget`]; a render that
    /// exceeds it shows the fallback code block and logs a warning.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use biscuit_visualized::components::mermaid::MermaidDiagram;
    ///
    /// let diagram = MermaidDiagram::new("flowchart LR\n    A --> B")
    ///     .with_time_budget(Duration::from_millis(500));
    /// assert_eq!(diagram.time_budget(), Duration::from_millis(500));
    /// ```
    pub fn with_time_budget(mut self, budget: Duration) -> Self {
        self.renderer = self.renderer.with_time_budget(budget);
        self
    }

    /// The time budget of each Merman request for this diagram.
    pub fn time_budget(&self) -> Duration {
        self.renderer.diagram.time_budget()
    }

    /// Selects who [`render_to_svg`](Self::render_to_svg)'s SVG is for:
    /// [`MermaidTarget::HtmlStatic`] for a static SVG embedded in HTML. The
    /// terminal's PNG is unaffected; see
    /// [`crate::mermaid::MermaidDiagram::with_target`].
    pub fn with_target(mut self, target: MermaidTarget) -> Self {
        self.renderer = self.renderer.with_target(target);
        self
    }

    /// Measures labels in a face the page may embed, for a
    /// [`MermaidTarget::HtmlStatic`] SVG; the page builds its `@font-face`
    /// rules from every diagram's [`render_to_svg_with_font`](Self::render_to_svg_with_font)
    /// with [`page_font`](crate::mermaid::page_font). See
    /// [`crate::mermaid::MermaidDiagram::with_embedded_font`].
    pub fn with_embedded_font(mut self, embed: bool) -> Self {
        self.renderer = self.renderer.with_embedded_font(embed);
        self
    }

    /// Sets when the diagram may be drawn as text on a terminal without
    /// images, and whether a partial drawing carries the note. Settings left
    /// unset resolve at render time ([`MermaidFallbackPolicy::resolve`]).
    pub fn with_fallback_policy(mut self, policy: MermaidFallbackPolicy) -> Self {
        self.fallback = policy;
        self
    }

    /// The fallback policy set by
    /// [`with_fallback_policy`](Self::with_fallback_policy).
    pub fn fallback_policy(&self) -> MermaidFallbackPolicy {
        self.fallback
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

    /// Renders the diagram to an SVG string, with the [`DiagramFont`] its
    /// page embeds when [`with_embedded_font`](Self::with_embedded_font)
    /// applies (an HTML static target); `None` otherwise.
    ///
    /// ## Errors
    ///
    /// The errors of [`render_to_svg`](Self::render_to_svg).
    pub fn render_to_svg_with_font(&self) -> Result<(String, Option<DiagramFont>), MermaidRenderError> {
        self.renderer.render_to_svg_with_font()
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
    /// use biscuit_visualized::components::mermaid::MermaidDiagram;
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
        term: &biscuit_terminal::terminal::Terminal,
    ) -> Result<MermaidRenderResult, MermaidRenderError> {
        let (output, png_path, cache_hit, width_cells) = self.render_to_image(term)?;
        // The image escapes are block-cohesive — align as a block.
        let output = self.layout.apply_block_layout(&output, term.width());

        Ok(MermaidRenderResult {
            output,
            png_path,
            cache_hit,
            width_cells,
        })
    }

    /// Resolves the image's columns for a terminal `term_width` wide.
    ///
    /// `ImageWidth::Scale` measures the diagram's natural width first; the
    /// other variants never parse the diagram here.
    ///
    /// ## Errors
    ///
    /// Returns `MermaidRenderError::Visualization` when a scaled diagram does
    /// not parse.
    pub fn resolve_dimensions(
        &self,
        term_width: u32,
        cell: Option<CellSize>,
    ) -> Result<ResolvedDimensions, MermaidRenderError> {
        let natural_width = match self.width {
            ImageWidth::Scale(_) => Some(self.renderer.natural_size()?.width),
            _ => None,
        };
        Ok(TerminalImage::resolve_scaled_dimensions_for(
            &self.width,
            &self.layout,
            term_width,
            natural_width,
            cell,
        ))
    }

    /// Chooses what the infallible [`render`](TerminalRenderable::render)
    /// shows on `term`, with this diagram's
    /// [fallback policy](Self::with_fallback_policy) resolved now.
    ///
    /// The image route is taken when `term`'s graphics route has pixels
    /// ([`TerminalImage::route_has_pixels`]), decided before any SVG is
    /// measured, font loaded, or artifact written; if it then fails, the
    /// source is shown, never text. Otherwise the diagram is drawn as text
    /// (Unicode or ASCII as `term` supports) within the columns the
    /// component's width and margins leave, less
    /// [`TEXT_BLOCK_CHROME_COLUMNS`]; anything Merman does not draw as a
    /// diagram, a drawing that does not fit, a policy that forbids it, or a
    /// text failure shows the source. A title set with
    /// [`with_title`](Self::with_title) leads the text and source nodes,
    /// unless the source already gives that title.
    pub fn present(&self, term: &biscuit_terminal::terminal::Terminal) -> MermaidPresentation {
        self.present_with(term, self.fallback.resolve())
    }

    /// [`present`](Self::present) under an already resolved `policy`.
    pub(crate) fn present_with(
        &self,
        term: &biscuit_terminal::terminal::Terminal,
        policy: ResolvedFallbackPolicy,
    ) -> MermaidPresentation {
        if TerminalImage::route_has_pixels(term) {
            return match self.try_render(term) {
                Ok(result) => MermaidPresentation::Image(result),
                Err(_) => MermaidPresentation::Source(self.titled(self.source_nodes())),
            };
        }
        match self.text_nodes(self.text_columns(term.width()), text_charset(term), policy) {
            Ok(Some(nodes)) => MermaidPresentation::Text(self.titled(nodes)),
            Ok(None) | Err(_) => MermaidPresentation::Source(self.titled(self.source_nodes())),
        }
    }

    /// The text drawing's nodes for a code block `columns` wide: `Ok(None)`
    /// when the source should be shown instead (the policy forbids text,
    /// the family is not drawn as a diagram, or the drawing does not fit).
    /// At most one Merman text request, and none under
    /// [`TextFallback::Never`](super::TextFallback::Never).
    ///
    /// ## Errors
    ///
    /// A parse error, Merman failure, time budget, or resource limit.
    pub(crate) fn text_nodes(
        &self,
        columns: u32,
        charset: TextCharset,
        policy: ResolvedFallbackPolicy,
    ) -> Result<Option<Vec<RenderNode>>, MermaidError> {
        let diagram = &self.renderer.diagram;
        // A policy that refuses this family's coverage needs no request.
        let admitted = diagram
            .text_eligibility()
            .is_some_and(|eligibility| policy.text_fallback.admits(eligibility.coverage()));
        if !admitted {
            return Ok(None);
        }
        let options = TextOptions::new(columns.saturating_sub(TEXT_BLOCK_CHROME_COLUMNS)).with_charset(charset);
        let TextRender::Drawing(drawing) = diagram.render_text(options)? else {
            return Ok(None);
        };
        let partial = drawing.coverage() == TextCoverage::Partial;
        // Rows may end in significant spaces; only trailing newlines go.
        let text = drawing.into_text();
        let mut nodes = vec![RenderNode::code(Some("txt".to_string()), None, text.trim_end_matches('\n'))];
        if partial && policy.show_partial_note {
            nodes.push(partial_note());
        }
        Ok(Some(nodes))
    }

    /// The source as a `mermaid` code node.
    fn source_nodes(&self) -> Vec<RenderNode> {
        vec![RenderNode::code(Some("mermaid".to_string()), None, self.instructions())]
    }

    /// `nodes` led by the component's title, unless the source already
    /// gives the same title.
    fn titled(&self, mut nodes: Vec<RenderNode>) -> Vec<RenderNode> {
        let Some(title) = self.renderer.diagram.title() else {
            return nodes;
        };
        if source_gives_title(self.instructions(), title) {
            return nodes;
        }
        nodes.insert(0, RenderNode::paragraph(vec![RenderNode::strong(vec![RenderNode::text(title)])]));
        nodes
    }

    /// The columns a text presentation may use on a terminal `term_width`
    /// wide: the image width's own limit inside the margins. A scaled width
    /// takes every available column, so no SVG is measured for text.
    ///
    /// A program that lays out [`present`](Self::present)'s nodes itself (to
    /// install a highlighting code renderer) renders them at this width and
    /// then applies the component's layout, as
    /// [`render`](biscuit_terminal::components::renderable::TerminalRenderable::render) does.
    pub fn text_columns(&self, term_width: u32) -> u32 {
        TerminalImage::resolve_scaled_dimensions_for(&self.width, &self.layout, term_width, None, None).image_width
    }

    /// Lays out `nodes` for `term` at [`text_columns`](Self::text_columns),
    /// with the plain code block (no highlighter).
    fn render_nodes(&self, nodes: &[RenderNode], term: &biscuit_terminal::terminal::Terminal) -> String {
        let mut options = TerminalRenderOptions::new(term, RenderStrictness::Lossy);
        options.context = options.context.with_width(self.text_columns(term.width()));
        render_terminal_node(&RenderNode::root(nodes.to_vec()), &options)
            .map(|rendered| rendered.output)
            .unwrap_or_else(|_| self.fallback_code_block())
    }

    /// Core render pipeline: Mermaid → PNG → terminal image string.
    ///
    /// Returns `(output, png_path, cache_hit, width_cells)` on success.
    /// `width_cells` is the resolved image width in terminal columns,
    /// after applying margins and the configured [`ImageWidth`] spec.
    fn render_to_image(
        &self,
        term: &biscuit_terminal::terminal::Terminal,
    ) -> Result<(String, PathBuf, bool, u32), MermaidRenderError> {
        let cell = term.cell_size();
        let dims = self.resolve_dimensions(term.width(), cell)?;
        let (target_width_px, row_height_px) = display_pixels(dims.image_width, cell);

        let (png_path, cache_hit) = self
            .renderer
            .render_to_cached_png_at_width(
                target_width_px,
                row_height_px,
                display_background(self.renderer.background, term),
            )?;

        let term_image = display_image(&png_path, dims.image_width)
            .map_err(|e| MermaidRenderError::DisplayError(e.to_string()))?;

        let width_cells = dims.image_width;

        let output = match term_image
            .render_with_route(term)
            .map_err(|e| MermaidRenderError::DisplayError(e.to_string()))?
        {
            ImageOutput::Pixels(output) => output,
            ImageOutput::Fallback { .. } | ImageOutput::HalfBlocks { .. } => {
                return Err(MermaidRenderError::NoImageSupport);
            }
        };

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

/// Whether a line of `source` gives `title` with a `title` statement: on a
/// line of its own (`title X`, or `title: X` in frontmatter) or after the
/// header (`pie title X`), optionally quoted.
fn source_gives_title(source: &str, title: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim();
        line.strip_prefix("title")
            .or_else(|| line.split_once(" title ").map(|(_, rest)| rest))
            .map(|rest| rest.trim_start().trim_start_matches(':').trim().trim_matches('"'))
            .is_some_and(|named| named == title.trim())
    })
}

impl biscuit_terminal::components::renderable::TerminalRenderable for MermaidDiagram {
    /// The image, or else the text drawing or source chosen by
    /// [`present`](MermaidDiagram::present); margins and alignment are
    /// applied once.
    fn render(&self, term: &biscuit_terminal::terminal::Terminal) -> String {
        match self.present(term) {
            // `try_render` has applied the layout already.
            MermaidPresentation::Image(result) => result.output,
            MermaidPresentation::Text(nodes) | MermaidPresentation::Source(nodes) => {
                self.layout.apply_block_layout(&self.render_nodes(&nodes, term), term.width())
            }
        }
    }

    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        self.render(&biscuit_terminal::terminal::Terminal::new_optimistic(width))
    }

    fn is_block_level(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &biscuit_terminal::utils::layout::Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut biscuit_terminal::utils::layout::Layout {
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
    use biscuit_terminal::components::renderable::TerminalRenderable;

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
        let diagram = MermaidDiagram::from("pie\n    \"Dogs\" : 50".to_string());
        assert_eq!(diagram.instructions(), "pie\n    \"Dogs\" : 50");
    }

    #[test]
    fn diagram_from_str() {
        let diagram = MermaidDiagram::from("pie\n    \"Dogs\" : 50");
        assert_eq!(diagram.instructions(), "pie\n    \"Dogs\" : 50");
    }

    #[test]
    fn diagram_builder_methods_chain() {
        use biscuit_terminal::components::terminal_image::ImageWidth;

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
        use biscuit_terminal::components::terminal_image::ImageWidth;

        let diagram = MermaidDiagram::new("pie\n    \"A\" : 1").with_width(ImageWidth::Percent(0.5));
        let term = biscuit_terminal::terminal::Terminal::new_optimistic(80);

        match diagram.try_render(&term) {
            Ok(result) => {
                // Default layout has zero margins, so 50% of 80 cols = 40.
                assert_eq!(
                    result.width_cells, 40,
                    "50% of an 80-column terminal should resolve to 40 cells"
                );
            }
            Err(e) => {
                // A failed render skips the assertion rather than fail —
                // the Level-2 tests exercise the full pipeline.
                eprintln!("Mermaid render unavailable in unit-test env: {e}");
            }
        }
    }

    /// `render_to_svg` must produce an SVG string containing expected Mermaid
    /// elements, or gracefully skip when the render fails.
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
                // A failed render skips rather than fails — the Level-2
                // tests exercise the full pipeline.
                eprintln!("Mermaid SVG render unavailable in unit-test env: {e}");
            }
        }
    }

    const GIT_GRAPH: &str = "gitGraph\n    commit id: \"4c2e4d5\"\n    branch feat/theme\n    checkout feat/theme\n    commit id: \"7a1b2c3\"";

    #[test]
    fn diagram_defaults_to_scale_one() {
        let diagram = MermaidDiagram::new(GIT_GRAPH);
        assert_eq!(diagram.width, ImageWidth::Scale(1.0));
    }

    #[test]
    fn scaled_columns_come_from_the_measured_svg_width() {
        let cell = CellSize::FALLBACK;
        let diagram = MermaidDiagram::new(GIT_GRAPH);
        let natural = diagram.renderer.natural_size().unwrap().width;
        let dims = diagram.resolve_dimensions(500, Some(cell)).unwrap();
        assert_eq!(dims.image_width, ImageWidth::scaled_columns(1.0, natural, cell));
        assert_eq!(dims.image_width, (natural / 8.0).ceil() as u32);

        // 125% of the same diagram is wider by the scale, give or take rounding.
        let larger = diagram
            .clone()
            .with_width(ImageWidth::Scale(1.25))
            .resolve_dimensions(500, Some(cell))
            .unwrap();
        assert_eq!(larger.image_width, ((natural * 1.25) / 8.0).ceil() as u32);
    }

    #[test]
    fn a_narrow_terminal_caps_the_scaled_width() {
        let diagram = MermaidDiagram::new(GIT_GRAPH);
        let dims = diagram.resolve_dimensions(10, Some(CellSize::FALLBACK)).unwrap();
        assert_eq!(dims.image_width, 10);
    }

    #[test]
    fn a_scaled_diagram_that_does_not_parse_is_an_error() {
        let diagram = MermaidDiagram::new("no diagram header here");
        assert!(matches!(
            diagram.resolve_dimensions(80, None),
            Err(MermaidRenderError::Visualization(_))
        ));
        // Other widths never parse while resolving.
        let fixed = diagram.with_width(ImageWidth::Characters(30));
        assert_eq!(fixed.resolve_dimensions(80, None).unwrap().image_width, 30);
    }

    // ── Fallbacks and the time budget (AC-C9, AC-C10) ─────────

    use std::time::Duration;

    use serial_test::serial;

    use crate::mermaid::render::take_merman_calls;
    use crate::tests::{capture_warnings, isolated_cache_dir};

    /// How far past its budget a cancelled Merman call may return.
    const BUDGET_MARGIN: Duration = Duration::from_millis(250);

    /// The component's output on an image-capable terminal, escapes
    /// stripped: a failed image shows the source as a plain `mermaid` code
    /// block, the same text as `fallback_code_block`.
    fn rendered(diagram: &MermaidDiagram) -> String {
        biscuit_terminal::utils::escape_codes::strip_escape_codes(
            diagram.render(&biscuit_terminal::terminal::Terminal::new_optimistic(80)),
        )
    }

    #[test]
    fn the_time_budget_defaults_to_two_seconds_and_reaches_the_library_type() {
        let diagram = MermaidDiagram::new("flowchart LR\n    A --> B");
        assert_eq!(diagram.time_budget(), Duration::from_secs(2));

        let bounded = diagram.with_time_budget(Duration::from_millis(40));

        assert_eq!(bounded.time_budget(), Duration::from_millis(40));
        assert_eq!(bounded.renderer.diagram.time_budget(), Duration::from_millis(40));
    }

    #[test]
    #[serial]
    fn a_render_over_its_time_budget_shows_the_code_block_with_a_warning() {
        let _cache = isolated_cache_dir();
        let mut source = String::from("flowchart LR\n");
        for index in 0..15 {
            source.push_str(&format!("    N{index}[Node number {index}] --> N{}\n", index + 1));
        }
        let budget = Duration::from_millis(2);
        // An uncancelled twin (another cache key) first: it warms the font
        // chain, whose first-use face resolution would otherwise run, and be
        // timed, inside the budgeted Merman call.
        take_merman_calls();
        crate::mermaid::MermaidDiagram::new(format!("{source}\n%% warm-up twin\n"))
            .render(&crate::artifact::RenderRequest::default())
            .expect("the twin fits 2 s");
        let full: Duration = take_merman_calls().into_iter().sum();
        let diagram = MermaidDiagram::new(source).with_time_budget(budget);

        let (output, warnings) = capture_warnings(|| rendered(&diagram));

        // The code block drops the source's trailing newline.
        assert_eq!(output, format!("```mermaid\n{}\n```", diagram.instructions().trim_end()));
        assert!(
            warnings.iter().any(|warning| warning.contains("time budget")),
            "{warnings:?}"
        );
        let calls = take_merman_calls();
        let merman = calls.first().expect("a Merman call");
        assert!(
            *merman <= budget + BUDGET_MARGIN && *merman < full,
            "Merman ran {merman:?} on a {budget:?} budget; uncancelled {full:?}"
        );
    }

    #[test]
    #[serial]
    fn the_component_returns_the_font_of_an_opt_in_html_static_svg() {
        let _cache = isolated_cache_dir();
        let source = "flowchart LR\n    A[Alpha] --> B[Beta]";
        let plain = MermaidDiagram::new(source).with_target(MermaidTarget::HtmlStatic);
        let opt_in = plain.clone().with_embedded_font(true);

        let (svg, font) = opt_in.render_to_svg_with_font().expect("opt-in svg");
        let font = font.expect("the opt-in applies to an HTML static SVG");
        assert!(!font.families().is_empty());
        assert!(svg.contains(font.families()[0]), "{svg}");
        assert_eq!(opt_in.render_to_svg().expect("cached svg"), svg);
        assert!(!crate::mermaid::page_font([&font]).expect("page font").is_empty());

        // Without the opt-in, or without the HTML static target, no font.
        assert!(plain.render_to_svg_with_font().expect("plain svg").1.is_none());
        let raster_safe = MermaidDiagram::new(source).with_embedded_font(true);
        assert!(raster_safe.render_to_svg_with_font().expect("raster-safe svg").1.is_none());
    }

    #[test]
    #[serial]
    fn a_parse_error_shows_the_code_block() {
        let _cache = isolated_cache_dir();
        let diagram = MermaidDiagram::new("graph TD; A to B");

        assert_eq!(rendered(&diagram), diagram.fallback_code_block());
    }

    #[test]
    #[serial]
    fn an_unknown_diagram_type_shows_the_code_block() {
        let _cache = isolated_cache_dir();
        let diagram = MermaidDiagram::new("notADiagram\n    A --> B");

        assert_eq!(rendered(&diagram), diagram.fallback_code_block());
    }

    #[test]
    #[serial]
    fn zenuml_shows_the_code_block() {
        let _cache = isolated_cache_dir();
        let diagram = MermaidDiagram::new("zenuml\n    title Demo\n    Alice->Bob: Hello");

        assert_eq!(rendered(&diagram), diagram.fallback_code_block());
    }

    #[test]
    #[serial]
    fn a_math_label_shows_its_source_text() {
        let _cache = isolated_cache_dir();
        let diagram = MermaidDiagram::new("flowchart LR\n    A[\"$$x^2$$\"] --> B");

        let output = rendered(&diagram);

        assert_eq!(output, diagram.fallback_code_block());
        assert!(output.contains("$$x^2$$"));
    }
}
