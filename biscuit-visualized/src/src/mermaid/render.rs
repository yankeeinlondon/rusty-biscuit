//! Mermaid rendering through Merman.
//!
//! One diagram goes through these steps (the spec's rendering pipeline):
//!
//! 1. The font-resolution chain ([`default_chain`]) and the styling: the
//!    diagram's [`Palette`] compiled into one Merman theme for its family
//!    ([`design`](super::design)), and the site config handed to Merman
//!    (Mermaid's `base` theme with the font stack, merged with
//!    [`MermaidConfig`]'s `quadrantChart` and `themeVariables`). A source
//!    whose `%%{init}%%` or frontmatter sets theme keys gets no compiled
//!    theme; the palette reaches Merman as `base` theme variables instead, so
//!    the author's keys win (D54).
//! 2. Source rewrites (step 3b): the `\n` → `<br/>` label rewrite, then, for a
//!    gitGraph whose tags collide, a `commitSpacing` directive. The gitGraph
//!    measurement is a Merman `layout_json` call, the only Merman call before
//!    the cache check.
//! 3. The cache check (step 4), keyed on the backend id, the rewritten source,
//!    the output format, a hash of the site config and the compiled design
//!    (so every palette color, R-P6), the font chain's cache
//!    identity for the theme's font stacks, the request's geometry, and the
//!    title.
//! 4. On a miss, Merman's resvg-safe SVG, measured with our measurer, under a
//!    time budget (steps 5–7); then the post-process, the
//!    [accents](super::accents), and the [contrast pass](super::contrast),
//!    the background, and,
//!    for PNG, rasterization (steps 8–9). An SVG for the
//!    [`MermaidTarget::HtmlStatic`] target also gets the HTML static edits
//!    (step 10′); PNG never does.
//! 5. With the [embedded-font opt-in](MermaidDiagram::with_embedded_font)
//!    (HTML static SVG only), labels are measured in the face the page will
//!    embed for their family, as the embedded file draws them (plain glyph
//!    advances from a subset, normal shaping from a face embedded whole), and
//!    the code points each family draws are kept beside the SVG in the cache.
//!    A diagram that draws a weight whose face may not be embedded is laid
//!    out again with that family in Inter, under the first layout's key.
//!
//! Every failure, a time budget exceeded included, is a
//! [`MermaidError::RenderFailed`]; callers show the source as a code block
//! ([`MermaidDiagram::fallback_code_block`]).

use std::borrow::Cow;
use std::sync::Arc;
use std::time::{Duration, Instant};

use merman::svg::{
    MeasurementProfileId, SvgEnvironment, SvgPipeline, TextMeasurementPolicy,
    TextMeasurementProfile, TextMeasurementProfileIdentity, TextMeasurer, TextStyle,
};
use merman::{Engine, OperationControl, RenderError, RenderOutput, Renderer, SvgRequest};

use crate::artifact::{Background, OutputFormat, RenderRequest, RenderedArtifact};
use crate::cache::file_cache::MERMAID_BACKEND;
use crate::cache::{FileCache, VisualizationKind};
use crate::fonts::{FontChain, FontQuery, LabelMeasurer, default_chain};
use crate::raster::png::rasterize_svg_for_request_with;

use super::accents::apply_accents;
use super::config::{MermaidConfig, MermaidTheme};
use super::contrast::{author_text_colors, parse_css_color, readable_text};
use super::design;
use super::embed::glyphs::{UsedGlyphs, collect_used_glyphs};
use super::embed::faces::forced_inter;
use super::embed::page::{DiagramFont, FontRecord};
use super::error::MermaidError;
use super::gitgraph::{GitGraphGeometry, TAG_GAP_EM};
use super::gitgraph_layout::{self, TagLabels};
use super::newline::rewrite_label_newlines;
use super::palette::Palette;
use super::postprocess::{html_static, postprocess};

/// The font stack the site config sets: Mermaid.js 12.2.0's themes'
/// `themeVariables.fontFamily`. It ends in `sans-serif` (R-T9). Setting it
/// explicitly also keeps Merman from drawing in a family its measurement
/// does not ask about.
const THEME_FONT_STACK: &str = r#""trebuchet ms", verdana, arial, sans-serif"#;

/// Mermaid's `themeVariables.fontSize`, in px: one em of tag gap unless the
/// source overrides it.
const THEME_FONT_SIZE: f32 = 16.0;

/// Mermaid's `themeVariables.tagLabelFontSize`, in px.
const TAG_LABEL_FONT_SIZE: f64 = 10.0;

/// The measurement profile Merman records for our measurer.
const MEASUREMENT_PROFILE: &str = "biscuit-visualized";

/// The suffix of the [`FontRecord`] kept beside an embedded-font SVG.
const FONT_RECORD_SUFFIX: &str = "font.json";

/// Who the SVG output is for. The default, [`RasterSafe`](Self::RasterSafe),
/// is drawn by this crate's rasterizer; [`HtmlStatic`](Self::HtmlStatic) is
/// embedded in an HTML page and drawn by the reader's browser.
///
/// A target only shapes SVG output: a PNG is always rasterized from the
/// raster-safe SVG, whatever the target.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_visualized::artifact::{OutputFormat, RenderRequest};
/// use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTarget};
///
/// let diagram = MermaidDiagram::new("flowchart LR\n    A --> B")
///     .with_target(MermaidTarget::HtmlStatic);
/// let request = RenderRequest { format: OutputFormat::Svg, ..RenderRequest::default() };
/// let artifact = diagram.render(&request)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MermaidTarget {
    /// The SVG this crate rasterizes, unchanged by any browser concern.
    #[default]
    RasterSafe,
    /// A static SVG for an HTML page. Every `font-family` starts with the
    /// family of the face the label was measured in, followed by the original
    /// stack, and label viewports let text overflow, because the reader's
    /// fonts are unknowable and a wider face must not be clipped.
    HtmlStatic,
}

/// A diagram's natural size in SVG user units (1 unit = 1 px at scale 1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NaturalSize {
    pub width: f32,
    pub height: f32,
}

/// A Mermaid diagram for rendering.
///
/// This struct handles rendering Mermaid diagrams to SVG or PNG format with
/// support for caching and customization. Rendering is done by Merman with
/// this crate's text measurer and font chain, so labels are measured in the
/// faces they are drawn in.
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
/// Every family Merman renders, among them flowchart (`flowchart` /
/// `graph`), sequence, class, state, ER, pie, XY chart, quadrant chart,
/// gantt, timeline, journey, mindmap, gitGraph, requirement, C4, block,
/// kanban, architecture, packet, radar, sankey, and treemap.
///
/// ## Notes
///
/// ZenUML, math labels (`$$…$$`, Merman's math support is not built in), a
/// parse error, and an unknown diagram type are render failures, as is a
/// render whose Merman share exceeds the [time budget](Self::with_time_budget).
#[derive(Debug, Clone)]
pub struct MermaidDiagram {
    instructions: String,
    title: Option<String>,
    theme: MermaidTheme,
    palette: Option<Palette>,
    config: MermaidConfig,
    time_budget: Duration,
    target: MermaidTarget,
    embed_font: bool,
}

/// What every Merman call for one diagram shares: the source after step 3b,
/// the site config, and the compiled design for the diagram's family.
struct Prepared {
    source: String,
    site_config: serde_json::Value,
    /// Merman's family id (`flowchart`, `gitGraph`, …), when it detects one.
    family: Option<String>,
    /// The design compiled for this render; `None` when the author's
    /// `%%{init}%%` or frontmatter owns the theme, or no family was detected.
    design: Option<serde_json::Value>,
}

impl MermaidDiagram {
    /// The time budget a diagram gets unless
    /// [`with_time_budget`](Self::with_time_budget) sets another.
    pub const DEFAULT_TIME_BUDGET: Duration = Duration::from_secs(2);

    /// Creates a new Mermaid diagram with default settings.
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
            palette: None,
            config: MermaidConfig::default(),
            time_budget: Self::DEFAULT_TIME_BUDGET,
            target: MermaidTarget::default(),
            embed_font: false,
        }
    }

    /// Styles the diagram with `theme`'s fixed palette ([`Palette::fixed`]),
    /// replacing a palette set by [`with_palette`](Self::with_palette).
    pub fn with_theme(mut self, theme: MermaidTheme) -> Self {
        self.theme = theme;
        self.palette = None;
        self
    }

    /// Styles the diagram with `palette`, for example one built from the
    /// terminal's colors ([`Palette::for_terminal`]).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_visualized::mermaid::{MermaidDiagram, Palette, Rgb, TerminalColors};
    ///
    /// let colors = TerminalColors { background: Rgb::from_hex("#002b36"), ..TerminalColors::default() };
    /// let palette = Palette::for_terminal(&colors, true);
    /// let diagram = MermaidDiagram::new("flowchart LR\n    A --> B").with_palette(palette);
    /// assert_eq!(diagram.palette(), palette);
    /// ```
    pub fn with_palette(mut self, palette: Palette) -> Self {
        self.palette = Some(palette);
        self
    }

    /// The palette the diagram is styled with: the one set by
    /// [`with_palette`](Self::with_palette), else the theme's fixed palette.
    pub fn palette(&self) -> Palette {
        self.palette.unwrap_or_else(|| Palette::fixed(self.theme))
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

    /// Sets how long Merman may work on each request for this diagram
    /// (default [`DEFAULT_TIME_BUDGET`](Self::DEFAULT_TIME_BUDGET), 2 s).
    ///
    /// The budget is a deadline Merman checks at its own checkpoints during
    /// parsing, layout, and SVG output, for SVG renders and gitGraph layout
    /// requests alike. Our post-processing and PNG rasterization run outside
    /// it, so a render can finish later than the budget without failing.
    /// A request that exceeds it fails with [`MermaidError::RenderFailed`]
    /// and logs a warning.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use std::time::Duration;
    /// use biscuit_visualized::mermaid::MermaidDiagram;
    ///
    /// let diagram = MermaidDiagram::new("graph LR; A-->B")
    ///     .with_time_budget(Duration::from_millis(500));
    /// assert_eq!(diagram.time_budget(), Duration::from_millis(500));
    /// ```
    pub fn with_time_budget(mut self, budget: Duration) -> Self {
        self.time_budget = budget;
        self
    }

    /// The time budget of each Merman request (see
    /// [`with_time_budget`](Self::with_time_budget)).
    pub fn time_budget(&self) -> Duration {
        self.time_budget
    }

    /// Selects who the SVG output is for (default
    /// [`MermaidTarget::RasterSafe`]). PNG output ignores it.
    pub fn with_target(mut self, target: MermaidTarget) -> Self {
        self.target = target;
        self
    }

    /// The target of SVG output (see [`with_target`](Self::with_target)).
    pub fn target(&self) -> MermaidTarget {
        self.target
    }

    /// Turns on the embedded-font opt-in (default off), which acts only on
    /// SVG output for [`MermaidTarget::HtmlStatic`]; other output is
    /// unchanged.
    ///
    /// With it on, each requested font family is measured in the face the
    /// HTML page will embed for it: the face the font chain resolves when the
    /// `OS/2` `fsType` of every weight the diagram draws permits embedding,
    /// else the bundled Inter for the whole family. Those weights are known
    /// only after layout, so a diagram that draws a refused weight is laid
    /// out a second time in Inter. Labels are
    /// measured as the embedded file draws them: plain glyph advances for a
    /// subset, which carries no layout tables, and normal shaping (kerning
    /// included) for a face whose `fsType` forbids subsetting, which is
    /// embedded whole. Every label's `font-family` leads with that face's
    /// family.
    /// [`render_with_font`](Self::render_with_font) returns, beside the
    /// artifact, the [`DiagramFont`] the page builds its `@font-face` rules
    /// from ([`page_font`](super::page_font)). The output has its own cache
    /// entries.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use biscuit_visualized::artifact::{OutputFormat, RenderRequest};
    /// use biscuit_visualized::mermaid::{MermaidDiagram, MermaidTarget, page_font};
    ///
    /// let diagram = MermaidDiagram::new("flowchart LR\n    A --> B")
    ///     .with_target(MermaidTarget::HtmlStatic)
    ///     .with_embedded_font(true);
    /// let request = RenderRequest { format: OutputFormat::Svg, ..RenderRequest::default() };
    /// let (artifact, font) = diagram.render_with_font(&request)?;
    /// let markup = page_font(font.as_ref())?.markup();
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_embedded_font(mut self, embed: bool) -> Self {
        self.embed_font = embed;
        self
    }

    /// Whether the embedded-font opt-in is on (see
    /// [`with_embedded_font`](Self::with_embedded_font)).
    pub fn embedded_font(&self) -> bool {
        self.embed_font
    }

    /// Returns the diagram instructions.
    pub fn instructions(&self) -> &str {
        &self.instructions
    }

    /// Renders the diagram to SVG or PNG format.
    ///
    /// A cached artifact for the same rewritten source, design, fonts, and
    /// request is returned without calling Merman's SVG renderer; otherwise
    /// the diagram is rendered, post-processed, rasterized when PNG is
    /// requested, and stored.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The diagram does not parse, its type is unknown or ZenUML, or it has
    ///   a math label
    /// - Merman exceeds the time budget
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
        self.render_in(default_chain(), request)
    }

    /// [`render`](Self::render), also returning the diagram's
    /// [`DiagramFont`] when the output carries the embedded-font opt-in (see
    /// [`with_embedded_font`](Self::with_embedded_font)); `None` otherwise.
    ///
    /// ## Errors
    ///
    /// The errors of [`render`](Self::render).
    pub fn render_with_font(
        &self,
        request: &RenderRequest,
    ) -> Result<(RenderedArtifact, Option<DiagramFont>), MermaidError> {
        self.render_with_font_in(default_chain(), request)
    }

    /// [`render`](Self::render) with `chain` measuring and drawing every
    /// label. The golden tests pass a chain of the bundled faces only, so no
    /// installed font reaches their output.
    pub(crate) fn render_in(
        &self,
        chain: &Arc<FontChain>,
        request: &RenderRequest,
    ) -> Result<RenderedArtifact, MermaidError> {
        self.render_with_font_in(chain, request).map(|(artifact, _)| artifact)
    }

    /// [`render_with_font`](Self::render_with_font) over `chain`.
    pub(crate) fn render_with_font_in(
        &self,
        chain: &Arc<FontChain>,
        request: &RenderRequest,
    ) -> Result<(RenderedArtifact, Option<DiagramFont>), MermaidError> {
        let embed = self.embeds_font(request.format);
        let prepared = self.prepare(chain, embed.then_some(NONE_FORCED))?;
        // A second opt-in pass (below) is stored under this first key, which
        // is the key a later render's hit path computes.
        let cache_key = self.cache_key_for(&prepared, chain, request);

        let cache = FileCache::new();
        if let Some(path) = cache.get(VisualizationKind::Mermaid, &cache_key, request.format) {
            // An embedded-font SVG is served only with its record; without
            // one (an interrupted store) it is rendered again.
            let font = embed.then(|| read_font_record(&cache, &cache_key, chain));
            if let None | Some(Some(_)) = font {
                let artifact = RenderedArtifact {
                    path,
                    format: request.format,
                    cache_hit: true,
                    alt_text: self.alt_text(),
                };
                return Ok((artifact, font.flatten()));
            }
        }

        let target = self.output_target(request.format);
        let measurer = label_measurer(chain, embed.then_some(NONE_FORCED));
        let mut svg = with_background(
            &self.render_svg(&prepared, chain, &measurer, &cache_key, target)?,
            self.background_color(request.background),
        );
        let font = match measurer.embedded_faces() {
            Some(mut faces) => {
                let mut glyphs = used_glyphs(&svg);
                // The weights a diagram draws are known only now: a family
                // that draws a refused weight is laid out again as Inter.
                let forced = forced_inter(&faces, &glyphs);
                if !forced.is_empty() {
                    let prepared = self.prepare(chain, Some(&forced))?;
                    let forced_measurer = label_measurer(chain, Some(&forced));
                    svg = with_background(
                        &self.render_svg(&prepared, chain, &forced_measurer, &cache_key, target)?,
                        self.background_color(request.background),
                    );
                    faces = forced_measurer.embedded_faces().unwrap_or(faces);
                    glyphs = used_glyphs(&svg);
                    debug_assert!(forced_inter(&faces, &glyphs).is_empty(), "the bundled Inter is always embeddable");
                }
                let record = FontRecord::new(&faces, &forced, &glyphs);
                let json = serde_json::to_vec(&record).map_err(|error| MermaidError::RenderFailed(error.to_string()))?;
                // Stored before the SVG, so a stored SVG always has its record.
                cache.store_sidecar(VisualizationKind::Mermaid, &cache_key, OutputFormat::Svg, FONT_RECORD_SUFFIX, &json)?;
                Some(DiagramFont {
                    faces,
                    glyphs,
                    chain: Arc::clone(chain),
                })
            }
            None => None,
        };
        let path = match request.format {
            OutputFormat::Svg => cache.store(
                VisualizationKind::Mermaid,
                &cache_key,
                OutputFormat::Svg,
                svg.as_bytes(),
            )?,
            OutputFormat::Png => {
                record_rasterized_svg(&svg);
                let png_data = rasterize_svg_for_request_with(&svg, request, chain)?;
                cache.store(
                    VisualizationKind::Mermaid,
                    &cache_key,
                    OutputFormat::Png,
                    &png_data,
                )?
            }
        };

        let artifact = RenderedArtifact {
            path,
            format: request.format,
            cache_hit: false,
            alt_text: self.alt_text(),
        };
        Ok((artifact, font))
    }

    /// The diagram's natural size in SVG user units: the root `viewBox` of
    /// the SVG [`render`](Self::render) draws, which is what the rasterizer
    /// scales from.
    ///
    /// Renders (or reads from the cache) the diagram's SVG artifact.
    ///
    /// ## Errors
    ///
    /// The errors of [`render`](Self::render), and
    /// `MermaidError::RenderFailed` when the SVG has no usable `viewBox`.
    pub fn natural_size(&self) -> Result<NaturalSize, MermaidError> {
        let request = RenderRequest {
            format: OutputFormat::Svg,
            ..RenderRequest::default()
        };
        let artifact = self.render(&request)?;
        let svg = std::fs::read_to_string(&artifact.path)?;
        let (_, _, width, height) = root_view_box(&svg).ok_or_else(|| {
            MermaidError::RenderFailed("Merman's SVG has no usable root viewBox".to_string())
        })?;
        Ok(NaturalSize { width, height })
    }

    /// The gitGraph layout that [`render`](Self::render) draws, as commit
    /// parents, lanes, and tag bounds, built from Merman's layout data for
    /// the source after step 3b (so `commit_step` is the written
    /// `commitSpacing` when tags collided). Test support for callers that
    /// must prove label placement without inspecting pixels; not a stable
    /// API.
    ///
    /// `Ok(None)` for any other diagram type. The layout request is bounded
    /// by the [time budget](Self::with_time_budget).
    ///
    /// ## Errors
    ///
    /// `MermaidError::RenderFailed` when the diagram does not parse, Merman
    /// exceeds the time budget, or the layout data is not of the shape this
    /// crate reads.
    #[doc(hidden)]
    pub fn gitgraph_geometry(&self) -> Result<Option<GitGraphGeometry>, MermaidError> {
        let chain = default_chain();
        let prepared = self.prepare(chain, None)?;
        let measurer = LabelMeasurer::text_measurer(chain);
        let Some(json) = self.layout_json(&prepared.source, &prepared, &measurer)? else {
            return Ok(None);
        };
        if !json.contains("\"GitGraphDiagram\"") {
            return Ok(None);
        }
        let (style, tag_gap) = tag_metrics(&json);
        let labels = TagLabels {
            measurer: measurer.as_ref(),
            style: &style,
        };
        let geometry = gitgraph_layout::geometry_from_layout_json(&json, &labels, tag_gap)?;
        Ok(Some(geometry))
    }

    /// Pipeline steps 1–3b: the site config, then the source rewrites. A
    /// gitGraph's tags are measured as the render measures them, so with
    /// `embed` (the requested families forced to Inter) in the faces of the
    /// embedded-font opt-in.
    fn prepare(&self, chain: &Arc<FontChain>, embed: Option<&[String]>) -> Result<Prepared, MermaidError> {
        if is_zenuml(&self.instructions) {
            return Err(MermaidError::RenderFailed(
                "ZenUML diagrams are not rendered; showing the source".to_string(),
            ));
        }
        let source = rewrite_label_newlines(&self.instructions).into_owned();
        let source_theme = source_theme(&source);
        let palette = self.palette();
        let design = match (&source_theme.family, source_theme.author_theme) {
            (Some(family), AuthorTheme::None) => Some(design::design_json(&palette, family)),
            _ => None,
        };
        let site_palette = match (&source_theme.family, &design) {
            (Some(family), Some(design)) => design::wants_site_palette(family, design),
            _ => source_theme.author_theme == AuthorTheme::Variables,
        };
        let mut site_config = self.site_config(&palette, site_palette, source_theme.author_variables.as_ref());
        if let (Some(family), Some(_)) = (&source_theme.family, &design) {
            for (key, value) in design::family_site_variables(family, &palette) {
                site_config["themeVariables"][key] = value;
            }
            // Merman lets site config win over the source's own, so a key the
            // author set in the source is left out.
            for (section, values) in design::family_site_config(family, &palette) {
                let serde_json::Value::Object(values) = values else { continue };
                for (key, value) in values {
                    if source_theme.author_config.pointer(&format!("/{section}/{key}")).is_none() {
                        site_config[&section][key] = value;
                    }
                }
            }
        }
        let mut prepared = Prepared {
            site_config,
            source,
            family: source_theme.family,
            design,
        };
        // A source without tags needs no spacing and gets no Merman call.
        if is_gitgraph(&prepared.source) && prepared.source.contains("tag:") {
            let measurer = label_measurer(chain, embed).into_text_measurer();
            let json = self
                .layout_json(&prepared.source, &prepared, &measurer)?
                .ok_or_else(|| MermaidError::RenderFailed("no Mermaid diagram detected".to_string()))?;
            let (style, tag_gap) = tag_metrics(&json);
            let labels = TagLabels {
                measurer: measurer.as_ref(),
                style: &style,
            };
            let spacing = gitgraph_layout::commit_spacing_for(&prepared.source, &labels, tag_gap, |_| Ok(json))?;
            if let Some(spacing) = spacing {
                prepared.source = gitgraph_layout::with_commit_spacing(&prepared.source, spacing);
            }
        }
        Ok(prepared)
    }

    /// The site config handed to Merman: Mermaid's `base` theme with the font
    /// stack, merged with [`MermaidConfig`]'s Mermaid-shaped JSON, plus the
    /// palette as `base` theme variables when `with_palette`.
    ///
    /// `with_palette` is decided per family ([`design::wants_site_palette`]):
    /// Merman lets explicit site variables win over compiled styles, so they
    /// go only where they cannot erase the design. When the author sets theme
    /// variables, the palette goes there too, beneath the author's keys
    /// (Merman reads the source's own config over the site's); when the author
    /// names a theme, it is left to that theme.
    fn site_config(
        &self,
        palette: &Palette,
        with_palette: bool,
        author_variables: Option<&serde_json::Value>,
    ) -> serde_json::Value {
        let mut config = self
            .config
            .to_json()
            .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
            .filter(serde_json::Value::is_object)
            .unwrap_or_else(|| serde_json::json!({}));
        config["theme"] = serde_json::Value::String("base".to_string());
        if !config["themeVariables"].is_object() {
            config["themeVariables"] = serde_json::json!({});
        }
        let variables = &mut config["themeVariables"];
        variables["fontFamily"] = serde_json::Value::String(THEME_FONT_STACK.to_string());
        if with_palette {
            for (key, color) in base_theme_variables(palette) {
                if variables.get(key).is_none() {
                    variables[key] = serde_json::Value::String(color.to_string());
                }
            }
            variables["darkMode"] = serde_json::Value::Bool(palette.is_dark());
        }
        // Merman lets site variables win over the source's own, the reverse
        // of Mermaid.js; copying the author's on top keeps theirs winning.
        if let Some(serde_json::Value::Object(author)) = author_variables {
            for (key, value) in author {
                variables[key] = value.clone();
            }
        }
        config
    }

    /// Whether `format`'s output carries the embedded-font opt-in: HTML
    /// static SVG only.
    fn embeds_font(&self, format: OutputFormat) -> bool {
        self.embed_font && self.output_target(format) == MermaidTarget::HtmlStatic
    }

    /// The target `format`'s output is made for: PNG is always rasterized
    /// from the raster-safe SVG.
    fn output_target(&self, format: OutputFormat) -> MermaidTarget {
        match format {
            OutputFormat::Svg => self.target,
            OutputFormat::Png => MermaidTarget::RasterSafe,
        }
    }

    /// The cache key for `request` (pipeline step 4).
    fn cache_key_for(&self, prepared: &Prepared, chain: &FontChain, request: &RenderRequest) -> String {
        let design = prepared.design.as_ref().map(serde_json::Value::to_string).unwrap_or_default();
        // Every palette color, not only the design: the accent post-process
        // and the contrast pass read colors the design may round away (R-P6).
        let mut options = serde_json::json!({
            "design": biscuit_hash::xx_hash(&format!("{}|{design}|{:?}", prepared.site_config, self.palette())),
            "fonts": chain.cache_identity(&theme_font_stacks()),
            "title": self.title,
            "request": request.cache_identity(),
        });
        // Only a target that changes the bytes joins the key, so raster-safe
        // keys, and the artifacts already cached under them, stay as they were.
        // The embedded-font opt-in is an output format of its own; the faces
        // it substitutes follow from the resolved faces `fonts` identifies.
        if self.embeds_font(request.format) {
            options["target"] = serde_json::Value::String("html-static+embedded-font".to_string());
        } else if self.output_target(request.format) == MermaidTarget::HtmlStatic {
            options["target"] = serde_json::Value::String("html-static".to_string());
        }
        let options_json = serde_json::to_string(&options).unwrap_or_else(|_| "{}".to_string());
        FileCache::cache_key(
            VisualizationKind::Mermaid,
            &prepared.source,
            &options_json,
            MERMAID_BACKEND,
            request.format,
        )
    }

    /// The cache key [`render`](Self::render) checks for `request`, with the
    /// fonts of `chain`.
    #[cfg(test)]
    pub(crate) fn cache_key_with(
        &self,
        chain: &Arc<FontChain>,
        request: &RenderRequest,
    ) -> Result<String, MermaidError> {
        let prepared = self.prepare(chain, self.embeds_font(request.format).then_some(NONE_FORCED))?;
        Ok(self.cache_key_for(&prepared, chain, request))
    }

    /// The site config [`render`](Self::render) hands Merman.
    #[cfg(test)]
    pub(crate) fn prepared_site_config(&self) -> serde_json::Value {
        self.prepare(default_chain(), None).map(|prepared| prepared.site_config).unwrap_or_default()
    }

    /// The compiled design's JSON [`render`](Self::render) applies, `None`
    /// when the author's theme keys own the styling.
    #[cfg(test)]
    pub(crate) fn prepared_design(&self) -> Option<serde_json::Value> {
        self.prepare(default_chain(), None).ok().and_then(|prepared| prepared.design)
    }

    /// Merman's theme evidence for the SVG [`render`](Self::render) draws:
    /// the evidence status (its `Debug` name) and every diagnostic as
    /// `code:subject:target`.
    #[cfg(test)]
    pub(crate) fn theme_evidence(&self) -> Result<(String, Vec<String>), MermaidError> {
        let chain = default_chain();
        let prepared = self.prepare(chain, None)?;
        let measurer = LabelMeasurer::text_measurer(chain);
        let request = svg_request(measurer, "bv-evidence")?;
        let control = self.control();
        let theme = prepared.design.as_ref().map(design::compile).transpose()?;
        let output = self.call_merman(|| {
            let mut request = merman::RenderRequest::svg(&prepared.source, control.clone(), request);
            if let Some(theme) = theme {
                request = request.with_theme(theme);
            }
            renderer(&prepared).render(request)
        })?;
        let RenderOutput::Svg(Some(svg)) = output else {
            return Err(MermaidError::RenderFailed("no Mermaid diagram detected".to_string()));
        };
        let evidence = svg.evidence();
        let diagnostics = evidence
            .theme_diagnostics()
            .iter()
            .map(|diagnostic| {
                let target = diagnostic.target().map_or("-", |target| target.id());
                format!("{}:{}:{target}", diagnostic.code(), diagnostic.subject())
            })
            .collect();
        Ok((format!("{:?}", evidence.theme_evidence().status()), diagnostics))
    }

    /// The source after pipeline step 3b, as Merman receives it.
    #[cfg(test)]
    pub(crate) fn rewritten_source(&self) -> Result<String, MermaidError> {
        Ok(self.prepare(default_chain(), None)?.source)
    }

    /// Merman's resvg-safe SVG for the prepared source, measured by
    /// `label_measurer` and post-processed for `target`.
    fn render_svg(
        &self,
        prepared: &Prepared,
        chain: &Arc<FontChain>,
        label_measurer: &Arc<LabelMeasurer>,
        cache_key: &str,
        target: MermaidTarget,
    ) -> Result<String, MermaidError> {
        // The HTML static edits ask this same measurer which face each stack
        // was measured in.
        let measurer = Arc::clone(label_measurer).into_text_measurer();
        // Content-addressed: different diagrams always get different ids, and
        // two identical diagrams render identical CSS, so sharing one id is
        // harmless. Keeping ids unique per HTML document (an ordinal) is the
        // HTML caller's concern.
        let diagram_id = format!("bv-{:016x}", biscuit_hash::xx_hash(cache_key));
        let control = self.control();
        let request = svg_request(measurer, &diagram_id)?;
        let theme = prepared.design.as_ref().map(design::compile).transpose()?;
        let output = self.call_merman(|| {
            let mut request = merman::RenderRequest::svg(&prepared.source, control.clone(), request);
            if let Some(theme) = theme {
                request = request.with_theme(theme);
            }
            renderer(prepared).render(request)
        })?;
        match output {
            RenderOutput::Svg(Some(svg)) => {
                let fixed = postprocess(svg.svg());
                let palette = self.palette();
                let accented = match (&prepared.design, &prepared.family) {
                    (Some(_), Some(family)) => apply_accents(&fixed, &palette, family),
                    _ => Cow::Borrowed(fixed.as_ref()),
                };
                // With or without a design: an author color displaces the
                // design, not the library's choice of text color.
                let fixed = readable_text(&accented, &palette, &author_text_colors(&prepared.source), chain).into_owned();
                Ok(match target {
                    MermaidTarget::RasterSafe => fixed,
                    MermaidTarget::HtmlStatic => html_static(&fixed, label_measurer).into_owned(),
                })
            }
            RenderOutput::Svg(None) => Err(MermaidError::RenderFailed("no Mermaid diagram detected".to_string())),
            other => Err(MermaidError::RenderFailed(format!("unexpected Merman output: {other:?}"))),
        }
    }

    /// Merman's layout document for `source`, or `None` when Merman detects
    /// no diagram.
    fn layout_json(
        &self,
        source: &str,
        prepared: &Prepared,
        measurer: &Arc<dyn TextMeasurer + Send + Sync>,
    ) -> Result<Option<String>, MermaidError> {
        let id = format!("bv-{:016x}", biscuit_hash::xx_hash(source));
        let request = svg_request(Arc::clone(measurer), &id)?;
        let control = self.control();
        let written = self.call_merman(|| {
            let output =
                renderer(prepared).render(merman::RenderRequest::layout_json(source, control.clone(), request))?;
            let RenderOutput::LayoutJson(Some(layout)) = output else {
                return Ok(None);
            };
            let mut bytes = Vec::new();
            let written = layout.layout().write_json_controlled(&mut bytes, &control)?;
            Ok(Some(written.map(|()| bytes)))
        })?;
        written
            .transpose()
            .map_err(|error| MermaidError::RenderFailed(format!("gitGraph layout data: {error}")))?
            .map(|bytes| String::from_utf8(bytes).map_err(|error| MermaidError::RenderFailed(error.to_string())))
            .transpose()
    }

    fn control(&self) -> OperationControl {
        OperationControl::new().with_deadline(self.time_budget)
    }

    /// Runs one Merman request, recording its elapsed time and mapping its
    /// error; a cancelled request is the time budget exceeded, logged as a
    /// warning.
    fn call_merman<T>(&self, call: impl FnOnce() -> Result<T, RenderError>) -> Result<T, MermaidError> {
        let started = Instant::now();
        let result = call();
        let elapsed = started.elapsed();
        record_merman_call(elapsed);
        result.map_err(|error| match error {
            RenderError::Cancelled(cancelled) => {
                tracing::warn!(
                    budget_ms = self.time_budget.as_millis() as u64,
                    elapsed_ms = elapsed.as_millis() as u64,
                    "Mermaid render exceeded its time budget; showing the source instead"
                );
                MermaidError::RenderFailed(format!(
                    "Mermaid render exceeded its time budget of {:?}: {cancelled}",
                    self.time_budget
                ))
            }
            other => MermaidError::RenderFailed(other.to_string()),
        })
    }

    /// The color painted behind the diagram, or `None` for no background.
    fn background_color(&self, background: Background) -> Option<String> {
        match background {
            Background::Theme => Some(self.palette().background().to_string()),
            Background::Transparent => None,
            Background::Solid { .. } => background.solid_hex(),
        }
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

/// The measurer for one render: with `embed`, the embedded-font opt-in's,
/// with those requested families forced to Inter.
fn label_measurer(chain: &Arc<FontChain>, embed: Option<&[String]>) -> Arc<LabelMeasurer> {
    Arc::new(match embed {
        Some(forced_inter) => LabelMeasurer::embedding_forcing(chain, forced_inter),
        None => LabelMeasurer::new(chain),
    })
}

/// The opt-in's first pass forces no family to Inter.
const NONE_FORCED: &[String] = &[];

/// The code points `svg` draws, or none (logged) when it cannot be read.
fn used_glyphs(svg: &str) -> UsedGlyphs {
    collect_used_glyphs(svg).unwrap_or_else(|error| {
        tracing::warn!(%error, "the embedded font's characters could not be read from the SVG; it embeds none");
        UsedGlyphs::default()
    })
}

/// The [`DiagramFont`] kept beside the cached SVG `key`, or `None` when the
/// record is missing or unreadable.
fn read_font_record(cache: &FileCache, key: &str, chain: &Arc<FontChain>) -> Option<DiagramFont> {
    let path = cache.sidecar_path(VisualizationKind::Mermaid, key, OutputFormat::Svg, FONT_RECORD_SUFFIX);
    let bytes = std::fs::read(path).ok()?;
    let record: FontRecord = serde_json::from_slice(&bytes)
        .inspect_err(|error| tracing::debug!(%error, "unreadable embedded-font record; rendering again"))
        .ok()?;
    Some(record.into_diagram_font(chain))
}

/// The theme's font stacks as the chain is asked for them: regular and bold.
fn theme_font_stacks() -> [FontQuery; 2] {
    [
        FontQuery::from_css(THEME_FONT_STACK, 400, false),
        FontQuery::from_css(THEME_FONT_STACK, 700, false),
    ]
}

/// The tag label style Merman laid a gitGraph out with and the gap colliding
/// tags get ([`TAG_GAP_EM`] of the font size), read from the layout
/// document's effective config: `themeVariables.fontFamily`,
/// `tagLabelFontSize`, and `fontSize`, so an author's `%%{init}%%` override
/// is measured as Merman draws it. Both sizes follow Merman's own reading
/// (see [`base_font_size`] and [`tag_font_size`]); the layout itself is read
/// strictly by `gitgraph_layout`.
fn tag_metrics(json: &str) -> (TextStyle, f32) {
    let document = serde_json::from_str::<serde_json::Value>(json).ok();
    let variables = document
        .as_ref()
        .and_then(|document| document.pointer("/meta/effective_config/themeVariables"));
    let variable = |name: &str| variables.and_then(|variables| variables.get(name));
    let font_size = base_font_size(variable("fontSize"));
    let family = variable("fontFamily")
        .and_then(serde_json::Value::as_str)
        .filter(|family| !family.trim().is_empty());
    let style = TextStyle {
        font_family: Some(family.unwrap_or(THEME_FONT_STACK).to_string()),
        font_size: tag_font_size(variable("tagLabelFontSize"), font_size),
        font_weight: None,
        font_style: None,
    };
    let gap = TAG_GAP_EM * font_size as f32;
    (style, gap)
}

/// The gitGraph base font size as Merman reads `themeVariables.fontSize`: a
/// number, or a string holding a number with an optional `px`; anything else
/// is the themes' 16 px. Merman clamps it to at least 1 px.
fn base_font_size(value: Option<&serde_json::Value>) -> f64 {
    let parsed = value.and_then(|value| {
        value.as_f64().or_else(|| {
            let text = value.as_str()?.trim();
            text.strip_suffix("px").unwrap_or(text).trim().parse::<f64>().ok()
        })
    });
    parsed
        .filter(|size| size.is_finite())
        .unwrap_or(f64::from(THEME_FONT_SIZE))
        .max(1.0)
}

/// The tag font size as Merman draws it from `themeVariables.tagLabelFontSize`.
/// Merman reads only a string: `px`, `em` and `%` (of the base size),
/// `inherit` and `unset` (the base size), or `0`. A JSON number, a unitless
/// string other than `0`, or anything else draws at the 10 px default, so it
/// is measured at 10 px too.
fn tag_font_size(value: Option<&serde_json::Value>, base: f64) -> f64 {
    let Some(text) = value.and_then(serde_json::Value::as_str).map(str::trim) else {
        return TAG_LABEL_FONT_SIZE;
    };
    let unit = |unit: &str| {
        text.strip_suffix(unit)?
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|size| size.is_finite() && *size >= 0.0)
    };
    if text.eq_ignore_ascii_case("inherit") || text.eq_ignore_ascii_case("unset") {
        base
    } else if let Some(size) = unit("px") {
        size
    } else if let Some(size) = unit("em") {
        size * base
    } else if let Some(size) = unit("%") {
        size * base / 100.0
    } else if text.parse::<f64>().is_ok_and(|size| size == 0.0) {
        0.0
    } else {
        TAG_LABEL_FONT_SIZE
    }
}

/// Whether the source's own `%%{init}%%` or frontmatter config styles the
/// diagram (D54).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthorTheme {
    /// No theme keys: the compiled design applies.
    None,
    /// `themeVariables` with a color but no `theme`: the palette as `base`
    /// variables, beneath the author's.
    Variables,
    /// A named `theme`: the author's theme alone.
    Named,
}

/// What the source says about its own styling, read before rendering.
struct SourceTheme {
    family: Option<String>,
    author_theme: AuthorTheme,
    /// The author's `themeVariables` when they set a color
    /// ([`AuthorTheme::Variables`]).
    author_variables: Option<serde_json::Value>,
    /// The source's whole `%%{init}%%`/frontmatter config.
    author_config: serde_json::Value,
}

/// The source's diagram family and author theme keys, from Merman's own
/// preprocessing (directives and frontmatter) and type detection. A source
/// Merman cannot preprocess or detect has no family; its render fails later
/// with Merman's own error.
fn source_theme(source: &str) -> SourceTheme {
    let engine = Engine::new();
    let Ok(preprocessed) = merman::preprocess_diagram(source, engine.registry()) else {
        return SourceTheme {
            family: None,
            author_theme: AuthorTheme::None,
            author_variables: None,
            author_config: serde_json::Value::Null,
        };
    };
    let author = preprocessed.config.as_value();
    let author_theme = if author.get("theme").is_some() {
        AuthorTheme::Named
    } else if author.get("themeVariables").is_some_and(sets_a_color) {
        AuthorTheme::Variables
    } else {
        AuthorTheme::None
    };
    let mut config = preprocessed.config.clone();
    let family = engine
        .registry()
        .detect_type(preprocessed.source.text(), &mut config)
        .ok()
        .and_then(merman::diagram_type_family_id)
        .map(|family| family.as_str().to_string());
    let author_variables = (author_theme == AuthorTheme::Variables).then(|| author["themeVariables"].clone());
    let author_config = author.clone();
    SourceTheme { family, author_theme, author_variables, author_config }
}

/// Whether an author's `themeVariables` set a color anywhere (a string, or a
/// comma-separated list, that parses as one). Our designs set colors only, so
/// a size or flag (`pieOuterStrokeWidth`, `darkMode`) cannot conflict with
/// them and keeps the compiled theme.
fn sets_a_color(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(text) => text.split(',').any(|part| parse_css_color(part).is_some()),
        serde_json::Value::Object(map) => map.values().any(sets_a_color),
        serde_json::Value::Array(items) => items.iter().any(sets_a_color),
        _ => false,
    }
}

/// The palette as Mermaid `base` theme variables. Only the root variables
/// the `base` theme derives the rest from: a derived one set here (`mainBkg`,
/// `nodeBorder`) would beat the author's root key (`primaryColor`) it derives
/// from.
fn base_theme_variables(palette: &Palette) -> Vec<(&'static str, super::Rgb)> {
    let [_, _, yellow, ..] = palette.accents();
    vec![
        ("background", palette.background()),
        ("primaryColor", palette.surface()),
        ("primaryTextColor", palette.text()),
        ("primaryBorderColor", palette.border()),
        ("secondaryColor", palette.surface_alt()),
        ("tertiaryColor", palette.surface_alt()),
        ("lineColor", palette.line()),
        ("textColor", palette.text()),
        ("titleColor", palette.text()),
        ("noteBkgColor", palette.tints()[2]),
        ("noteBorderColor", yellow),
        ("noteTextColor", palette.text()),
    ]
}

fn renderer(prepared: &Prepared) -> Renderer {
    Renderer::new().with_engine(
        Engine::new().with_site_config(merman::MermaidConfig::from_value(prepared.site_config.clone())),
    )
}

fn svg_request(
    measurer: Arc<dyn TextMeasurer + Send + Sync>,
    diagram_id: &str,
) -> Result<SvgRequest, MermaidError> {
    let failed = |error: &dyn std::fmt::Display| MermaidError::RenderFailed(error.to_string());
    let profile = MeasurementProfileId::new(MEASUREMENT_PROFILE).map_err(|error| failed(&error))?;
    let identity = TextMeasurementProfileIdentity::new(profile, "1").map_err(|error| failed(&error))?;
    let mut request = SvgRequest {
        environment: SvgEnvironment::deterministic().with_text_measurement_policy(
            TextMeasurementPolicy::uniform(TextMeasurementProfile::new(identity, measurer)),
        ),
        ..SvgRequest::default()
    };
    request.options.diagram_id = Some(diagram_id.to_string());
    request.pipeline = Some(SvgPipeline::resvg_safe());
    Ok(request)
}

/// The diagram header: the first word of the first line that is not blank,
/// a `%%` comment or directive, or YAML frontmatter.
fn header_keyword(source: &str) -> Option<&str> {
    let mut in_frontmatter = false;
    let mut in_directive = false;
    let mut first = true;
    for line in source.lines().map(str::trim) {
        let at_start = std::mem::replace(&mut first, false);
        if in_frontmatter {
            in_frontmatter = line != "---";
            continue;
        }
        if in_directive {
            in_directive = !line.contains("}%%");
            continue;
        }
        if line.is_empty() {
            first = at_start;
            continue;
        }
        if at_start && line == "---" {
            in_frontmatter = true;
            continue;
        }
        if line.starts_with("%%{") {
            in_directive = !line.contains("}%%");
            continue;
        }
        if line.starts_with("%%") {
            continue;
        }
        return line.split(|c: char| c.is_whitespace() || c == ':' || c == ';').next();
    }
    None
}

fn is_zenuml(source: &str) -> bool {
    header_keyword(source).is_some_and(|keyword| keyword.eq_ignore_ascii_case("zenuml"))
}

fn is_gitgraph(source: &str) -> bool {
    header_keyword(source) == Some("gitGraph")
}

/// The root `<svg>` element's `viewBox` as `(min_x, min_y, width, height)`,
/// when it has four numbers and a positive size.
fn root_view_box(svg: &str) -> Option<(f32, f32, f32, f32)> {
    let start = svg.find("<svg")?;
    let tag = &svg[start..start + svg[start..].find('>')?];
    let value_start = tag.find("viewBox=\"")? + "viewBox=\"".len();
    let value = &tag[value_start..value_start + tag[value_start..].find('"')?];
    let numbers: Vec<f32> = value
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|part| !part.is_empty())
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    match numbers.as_slice() {
        &[x, y, width, height] if width > 0.0 && height > 0.0 => Some((x, y, width, height)),
        _ => None,
    }
}

/// `svg` with a rectangle of `color` covering its root `viewBox`, painted
/// first; unchanged for `None` or an SVG without a usable `viewBox`.
fn with_background(svg: &str, color: Option<String>) -> String {
    let (Some(color), Some((x, y, width, height))) = (color, root_view_box(svg)) else {
        return svg.to_string();
    };
    let Some(open_end) = svg.find("<svg").and_then(|start| svg[start..].find('>').map(|end| start + end + 1)) else {
        return svg.to_string();
    };
    // An inline style, so no family stylesheet rule for `rect` repaints it.
    let rect = format!(
        r#"<rect x="{x}" y="{y}" width="{width}" height="{height}" fill="{color}" style="fill:{color};stroke:none"/>"#
    );
    let mut out = String::with_capacity(svg.len() + rect.len());
    out.push_str(&svg[..open_end]);
    out.push_str(&rect);
    out.push_str(&svg[open_end..]);
    out
}

#[cfg(test)]
thread_local! {
    static MERMAN_CALLS: std::cell::RefCell<Vec<Duration>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg_attr(not(test), allow(unused_variables))]
fn record_merman_call(elapsed: Duration) {
    #[cfg(test)]
    MERMAN_CALLS.with(|calls| calls.borrow_mut().push(elapsed));
}

#[cfg(test)]
thread_local! {
    static RASTERIZED_SVGS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg_attr(not(test), allow(unused_variables))]
fn record_rasterized_svg(svg: &str) {
    #[cfg(test)]
    RASTERIZED_SVGS.with(|svgs| svgs.borrow_mut().push(svg.to_string()));
}

/// Every SVG this thread handed the rasterizer for a PNG render since the
/// last call, oldest first, and clears the record.
#[cfg(test)]
pub(crate) fn take_rasterized_svgs() -> Vec<String> {
    RASTERIZED_SVGS.with(|svgs| std::mem::take(&mut *svgs.borrow_mut()))
}

/// The elapsed time of every Merman request this thread made since the last
/// call, oldest first, and clears the record.
#[cfg(test)]
pub(crate) fn take_merman_calls() -> Vec<Duration> {
    MERMAN_CALLS.with(|calls| std::mem::take(&mut *calls.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_keyword_skips_comments_directives_and_frontmatter() {
        assert_eq!(header_keyword("zenuml\n A.b()"), Some("zenuml"));
        assert_eq!(header_keyword("\n%% c\n%%{init: {}}%%\ngitGraph\n"), Some("gitGraph"));
        assert_eq!(header_keyword("---\ntitle: x\n---\ngitGraph LR:\n"), Some("gitGraph"));
        assert_eq!(header_keyword("%%{init:\n {}\n}%%\nzenuml\n"), Some("zenuml"));
        assert_eq!(header_keyword("graph LR; A-->B"), Some("graph"));
        assert_eq!(header_keyword(""), None);
    }

    #[test]
    fn root_view_box_reads_the_root_element_only() {
        let svg = r#"<svg id="a" viewBox="0 0 370 64"><svg viewBox="0 0 1 1"/></svg>"#;
        assert_eq!(root_view_box(svg), Some((0.0, 0.0, 370.0, 64.0)));
        assert_eq!(root_view_box(r#"<svg viewBox="0 0 0 5"/>"#), None);
        assert_eq!(root_view_box("<svg/>"), None);
    }

    #[test]
    fn with_background_paints_the_view_box_first() {
        let svg = r#"<svg viewBox="-1 2 30 40"><style/><g/></svg>"#;
        let painted = with_background(svg, Some("#123456".to_string()));
        assert_eq!(
            painted,
            r##"<svg viewBox="-1 2 30 40"><rect x="-1" y="2" width="30" height="40" fill="#123456" style="fill:#123456;stroke:none"/><style/><g/></svg>"##
        );
        assert_eq!(with_background(svg, None), svg);
    }
}
