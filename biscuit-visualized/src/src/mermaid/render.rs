//! Mermaid rendering through Merman.
//!
//! One diagram goes through these steps (the spec's rendering pipeline):
//!
//! 1. The font-resolution chain ([`default_chain`]) and the site config handed
//!    to Merman: the built-in theme named by [`MermaidTheme`] merged with
//!    [`MermaidConfig`]'s `quadrantChart` and `themeVariables`.
//! 2. Source rewrites (step 3b): the `\n` → `<br/>` label rewrite, then, for a
//!    gitGraph whose tags collide, a `commitSpacing` directive. The gitGraph
//!    measurement is a Merman `layout_json` call, the only Merman call before
//!    the cache check.
//! 3. The cache check (step 4), keyed on the backend id, the rewritten source,
//!    the output format, a hash of the site config, the font chain's cache
//!    identity for the theme's font stacks, the request's geometry, and the
//!    title.
//! 4. On a miss, Merman's resvg-safe SVG, measured with our measurer, under a
//!    time budget (steps 5–7); then the post-process, the background, and,
//!    for PNG, rasterization (steps 8–9). An SVG for the
//!    [`MermaidTarget::HtmlStatic`] target also gets the HTML static edits
//!    (step 10′); PNG never does.
//!
//! Every failure, a time budget exceeded included, is a
//! [`MermaidError::RenderFailed`]; callers show the source as a code block
//! ([`MermaidDiagram::fallback_code_block`]).

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
use crate::raster::png::rasterize_svg_for_request;

use super::config::{MermaidConfig, MermaidTheme};
use super::error::MermaidError;
use super::gitgraph::{GitGraphGeometry, TAG_GAP_EM};
use super::gitgraph_layout::{self, TagLabels};
use super::newline::rewrite_label_newlines;
use super::postprocess::{html_static, postprocess};

/// The font stack every built-in Merman theme sets (`themeVariables.fontFamily`
/// of Mermaid.js 12.2.0's `default`, `dark`, `forest`, and `neutral`).
const THEME_FONT_STACK: &str = r#""trebuchet ms", verdana, arial, sans-serif"#;

/// The built-in themes' `themeVariables.fontSize`, in px: one em of tag gap
/// unless the source overrides it.
const THEME_FONT_SIZE: f32 = 16.0;

/// The built-in themes' `themeVariables.tagLabelFontSize`, in px.
const TAG_LABEL_FONT_SIZE: f64 = 10.0;

/// The measurement profile Merman records for our measurer.
const MEASUREMENT_PROFILE: &str = "biscuit-visualized";

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
    config: MermaidConfig,
    time_budget: Duration,
    target: MermaidTarget,
}

/// What every Merman call for one diagram shares: the source after step 3b
/// and the site config.
struct Prepared {
    source: String,
    site_config: serde_json::Value,
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
            config: MermaidConfig::default(),
            time_budget: Self::DEFAULT_TIME_BUDGET,
            target: MermaidTarget::default(),
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
        let chain = default_chain();
        let prepared = self.prepare(chain)?;
        let cache_key = self.cache_key_for(&prepared, chain, request);

        let cache = FileCache::new();
        if let Some(path) = cache.get(VisualizationKind::Mermaid, &cache_key, request.format) {
            return Ok(RenderedArtifact {
                path,
                format: request.format,
                cache_hit: true,
                alt_text: self.alt_text(),
            });
        }

        let svg = self.render_svg(&prepared, chain, &cache_key, self.output_target(request.format))?;
        let svg = with_background(&svg, self.background_color(request.background));
        let path = match request.format {
            OutputFormat::Svg => cache.store(
                VisualizationKind::Mermaid,
                &cache_key,
                OutputFormat::Svg,
                svg.as_bytes(),
            )?,
            OutputFormat::Png => {
                record_rasterized_svg(&svg);
                let png_data = rasterize_svg_for_request(&svg, request)?;
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
        let prepared = self.prepare(chain)?;
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

    /// Pipeline steps 1–3b: the site config, then the source rewrites.
    fn prepare(&self, chain: &Arc<FontChain>) -> Result<Prepared, MermaidError> {
        if is_zenuml(&self.instructions) {
            return Err(MermaidError::RenderFailed(
                "ZenUML diagrams are not rendered; showing the source".to_string(),
            ));
        }
        let mut prepared = Prepared {
            source: rewrite_label_newlines(&self.instructions).into_owned(),
            site_config: self.site_config(),
        };
        // A source without tags needs no spacing and gets no Merman call.
        if is_gitgraph(&prepared.source) && prepared.source.contains("tag:") {
            let measurer = LabelMeasurer::text_measurer(chain);
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

    /// The site config handed to Merman: the built-in theme named by
    /// [`MermaidTheme`] (an explicit theme also keeps Merman from drawing in a
    /// family its measurement does not ask about) merged with
    /// [`MermaidConfig`]'s Mermaid-shaped JSON.
    fn site_config(&self) -> serde_json::Value {
        let mut config = self
            .config
            .to_json()
            .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
            .filter(serde_json::Value::is_object)
            .unwrap_or_else(|| serde_json::json!({}));
        config["theme"] = serde_json::Value::String(self.theme.as_str().to_string());
        config
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
        let mut options = serde_json::json!({
            "design": biscuit_hash::xx_hash(&prepared.site_config.to_string()),
            "fonts": chain.cache_identity(&theme_font_stacks()),
            "title": self.title,
            "request": request.cache_identity(),
        });
        // Only a target that changes the bytes joins the key, so raster-safe
        // keys, and the artifacts already cached under them, stay as they were.
        if self.output_target(request.format) == MermaidTarget::HtmlStatic {
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
        let prepared = self.prepare(chain)?;
        Ok(self.cache_key_for(&prepared, chain, request))
    }

    /// The source after pipeline step 3b, as Merman receives it.
    #[cfg(test)]
    pub(crate) fn rewritten_source(&self) -> Result<String, MermaidError> {
        Ok(self.prepare(default_chain())?.source)
    }

    /// Merman's resvg-safe SVG for the prepared source, post-processed for
    /// `target`.
    fn render_svg(
        &self,
        prepared: &Prepared,
        chain: &Arc<FontChain>,
        cache_key: &str,
        target: MermaidTarget,
    ) -> Result<String, MermaidError> {
        // The HTML static edits ask this same measurer which face each stack
        // was measured in.
        let label_measurer = Arc::new(LabelMeasurer::new(chain));
        let measurer = Arc::clone(&label_measurer).into_text_measurer();
        // Content-addressed: different diagrams always get different ids, and
        // two identical diagrams render identical CSS, so sharing one id is
        // harmless. Keeping ids unique per HTML document (an ordinal) is the
        // HTML caller's concern.
        let diagram_id = format!("bv-{:016x}", biscuit_hash::xx_hash(cache_key));
        let control = self.control();
        let request = svg_request(measurer, &diagram_id)?;
        let output = self.call_merman(|| {
            renderer(prepared).render(merman::RenderRequest::svg(&prepared.source, control.clone(), request))
        })?;
        match output {
            RenderOutput::Svg(Some(svg)) => {
                let fixed = postprocess(svg.svg());
                Ok(match target {
                    MermaidTarget::RasterSafe => fixed.into_owned(),
                    MermaidTarget::HtmlStatic => html_static(&fixed, &label_measurer).into_owned(),
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
            Background::Theme => Some(theme_background(self.theme).to_string()),
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
/// is measured as Merman draws it. A value that is absent or not a size takes
/// the built-in themes' default; the layout itself is read strictly by
/// `gitgraph_layout`.
fn tag_metrics(json: &str) -> (TextStyle, f32) {
    let document = serde_json::from_str::<serde_json::Value>(json).ok();
    let variables = document
        .as_ref()
        .and_then(|document| document.pointer("/meta/effective_config/themeVariables"));
    let variable = |name: &str| variables.and_then(|variables| variables.get(name));
    let px = |value: &serde_json::Value| {
        value.as_f64().or_else(|| {
            value
                .as_str()
                .and_then(|text| text.trim().trim_end_matches("px").trim().parse::<f64>().ok())
        })
    };
    let font_size = variable("fontSize").and_then(px).filter(|size| *size > 0.0);
    let tag_size = variable("tagLabelFontSize").and_then(px).filter(|size| *size > 0.0);
    let family = variable("fontFamily")
        .and_then(serde_json::Value::as_str)
        .filter(|family| !family.trim().is_empty());
    let style = TextStyle {
        font_family: Some(family.unwrap_or(THEME_FONT_STACK).to_string()),
        font_size: tag_size.unwrap_or(TAG_LABEL_FONT_SIZE),
        font_weight: None,
        font_style: None,
    };
    let gap = TAG_GAP_EM * font_size.map_or(THEME_FONT_SIZE, |size| size as f32);
    (style, gap)
}

/// The built-in theme's `themeVariables.background` (Mermaid.js 12.2.0, as
/// Merman's generated theme table records it). Merman leaves the root
/// unpainted, so [`Background::Theme`] paints this color itself.
fn theme_background(theme: MermaidTheme) -> &'static str {
    match theme {
        MermaidTheme::Dark => "#333",
        MermaidTheme::Default | MermaidTheme::Forest => "white",
        MermaidTheme::Neutral => "#ffffff",
    }
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
