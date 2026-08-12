//! Tree-rendering entry points for darkmatter.
//!
//! These functions are the **internal adapter boundary** between darkmatter's
//! public `Markdown` renderers and the render-tree pipeline shared with
//! `biscuit-terminal` and `renderable`. As of the tree cutover
//! (`renderable/features/2026-06-02-tree-cutover/`), [`render_tree_html`] backs
//! the public [`Markdown::as_html`](crate::markdown::Markdown::as_html) and
//! [`render_tree_terminal`] backs [`Markdown::as_terminal`](crate::markdown::Markdown::as_terminal)
//! (and the default-layout [`DarkmatterPage::render`](crate::layout::DarkmatterPage::render)
//! path). [`render_tree_markdown`] / [`render_tree_markdown_dialect`] drive the
//! parity test suite and benchmarks.
//!
//! The legacy event-stream serializers have been deleted; these tree entry
//! points are the only render path. See
//! `renderable/features/_completed/2026-05-20-darkmatter-tree/entry-point-shape.md`.
//!
//! Visibility: the document entry points the cutover flipped —
//! [`render_tree_html`], [`render_tree_terminal`], [`render_tree_markdown`], and
//! [`render_tree_markdown_dialect`] — are `pub` and re-exported from
//! [`render_tree`](super) so the public `Markdown` / `DarkmatterPage` renderers
//! and the parity benches reach them. [`to_render_document`] and the
//! `*_with_context` internal entry points stay `pub(crate)`: they expose the
//! raw fold / context boundary used only inside the crate.
//!
//! As the internal adapter boundary, these entry points wire the deprecated
//! [`TerminalCodeRenderer`](super::code_renderer::TerminalCodeRenderer) directly;
//! the module-level `allow` keeps that intentional internal use warning-free.
#![allow(deprecated)]

use std::collections::HashMap;
use std::rc::Rc;

use biscuit_terminal::discovery::detection::ColorDepth as TerminalColorDepth;
use biscuit_terminal::render_tree::{
    ImagePlaceholder, TerminalRenderContext, TerminalRenderOptions, render_terminal_document,
};
use biscuit_terminal::terminal::Terminal;
use renderable::browser::feature::{FeatureContext, FeatureResolver, PageFeature};
use renderable::tree::{
    BrowserMermaidMode, BrowserRenderOptions, Diagnostic, Document, GraphicsMode, MarkdownDialect,
    MarkdownRenderOptions, RawHtmlPolicy, RenderStrictness, SourceDescriptor, TerminalMermaidMode,
    render_browser_document_body, render_browser_document_html, render_markdown_document,
};

use renderable::tree::{NodeKind, RenderNode};

use super::code_renderer::TerminalCodeRenderer;
use super::pipeline::PipelineResult;
use crate::markdown::Markdown;
use crate::markdown::inline::HorizontalRuleAttrs;
use crate::markdown::output::{ColorDepth, HtmlOptions, TerminalOptions};

/// Folds a [`Markdown`] into a canonical [`Document`], wiring darkmatter's
/// already extracted frontmatter into [`renderable::tree::DocumentMetadata`].
///
/// Thin wrapper around [`to_render_document_with_context`] with an empty
/// build context (no component policies, no styles, no HR defaults). Used by
/// the Markdown-dialect entry points and by tests that verify base folding
/// behavior.
///
/// ## Returns
///
/// The folded [`Document`] and any non-fatal fold-phase [`Diagnostic`]s
/// (unsupported variants, lossy conversions, malformed structure), or a fatal
/// [`MarkdownError`] raised by the block-extension processor.
pub(crate) fn to_render_document(
    md: &Markdown,
) -> crate::markdown::MarkdownResult<(Document, Vec<Diagnostic>)> {
    let empty_policies = HashMap::new();
    let ctx = super::build_context::TreeBuildContext {
        component_policies: &empty_policies,
        page_color: None,
        page_bg_color: None,
        hyperlink_style: None,
        local_hyperlink_style: None,
        local_image_style: None,
        hr_defaults: None,
    };
    to_render_document_with_context(md, &ctx)
}

/// Folds a [`Markdown`] into a **complete** [`Document`] using the
/// context-aware fold ([`fold_markdown_spanned_with_context`](super::fold::fold_markdown_spanned_with_context))
/// that bakes component policy, colors, text-layout hints, structured
/// directives, and HR defaults into every node during construction.
///
/// The resulting tree needs no post-fold decoration, opacity hints, or
/// attribute injection — it is the final typed render input for every target.
///
/// ## Returns
///
/// The folded [`Document`] and any non-fatal fold-phase [`Diagnostic`]s, or a
/// fatal [`MarkdownError`] raised by the block-extension processor.
pub(crate) fn to_render_document_with_context(
    md: &Markdown,
    ctx: &super::build_context::TreeBuildContext,
) -> crate::markdown::MarkdownResult<(Document, Vec<Diagnostic>)> {
    let source = derive_source(md);
    super::fold::fold_markdown_spanned_with_context(source, md, ctx)
}

/// Resolves HR defaults from the supplied options. Root-level `hr:`
/// frontmatter is no longer read; only `style.hr.*` and explicit
/// `hr_defaults` options are honored.
pub(crate) fn resolve_hr_defaults(
    _md: &Markdown,
    options_hr_defaults: &Option<HorizontalRuleAttrs>,
) -> Option<HorizontalRuleAttrs> {
    options_hr_defaults.clone()
}
/// Lowers [`HtmlOptions::hr_css_variables`](crate::markdown::output::HtmlOptions::hr_css_variables)
/// to a sorted [`PageOptions::css_variables`](renderable::browser::PageOptions::css_variables)
/// override list, or `None` when nothing safe remains.
///
/// Mirrors the legacy `generate_hr_root_block`: keys are passed without the
/// `--` prefix (the page renderer adds it), sorted for deterministic output,
/// and any entry whose key or value contains `<` or a newline is skipped so it
/// cannot break out of the emitted `<style>` element.
fn hr_root_variables(
    vars: &std::collections::HashMap<String, String>,
) -> Option<Vec<(String, String)>> {
    let is_safe = |s: &str| !s.contains('<') && !s.contains('\n');
    let mut entries: Vec<(String, String)> = vars
        .iter()
        .filter(|(key, value)| is_safe(key) && is_safe(value))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    if entries.is_empty() {
        return None;
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Some(entries)
}

/// Renders a [`Markdown`] to HTML via the context-aware render-tree pipeline.
///
/// Builds a [`TreeBuildContext`](super::build_context::TreeBuildContext) from
/// [`HtmlOptions`] (hyperlink / image styles and HR defaults) and folds the
/// markdown into a complete [`Document`] that carries all typed intent. The
/// browser renderer then performs a single fold — no post-fold decoration,
/// opacity injection, or attribute rewriting is needed.
///
/// HR defaults resolve from [`HtmlOptions::hr_defaults`] only; root-level `hr:`
/// frontmatter is no longer read.
///
/// Uses the direct document-string renderer
/// ([`render_browser_document_html`]) — the production browser path needs the
/// complete final HTML string, so it streams straight into one buffer rather
/// than building an intermediate [`HtmlPage`](renderable::html::HtmlPage) /
/// [`BrowserFragment`](renderable::browser::BrowserFragment) tree and then
/// serializing it.
///
/// ## Errors
///
/// Returns [`MarkdownError::InvalidLineRange`](crate::markdown::MarkdownError::InvalidLineRange)
/// when a fenced code block carries a malformed DSL directive (the fatal
/// browser-path contract; see [`validate_code_directives`]), or
/// [`MarkdownError::RenderTree`](crate::markdown::MarkdownError::RenderTree)
/// wrapping any fatal render error from [`render_browser_document_html`].
/// Non-fatal diagnostics are returned in the [`PipelineResult`] without being
/// demoted to errors.
pub fn render_tree_html(
    md: &Markdown,
    options: &HtmlOptions,
) -> crate::markdown::MarkdownResult<PipelineResult<String>> {
    let empty_policies = HashMap::new();
    let hr_defaults_owned = resolve_hr_defaults(md, &options.hr_defaults);
    let build_ctx = super::build_context::TreeBuildContext {
        component_policies: &empty_policies,
        page_color: None,
        page_bg_color: None,
        hyperlink_style: options.hyperlink_style.as_ref(),
        local_hyperlink_style: options.local_hyperlink_style.as_ref(),
        local_image_style: options.local_image_style.as_ref(),
        hr_defaults: hr_defaults_owned.as_ref(),
    };
    render_tree_html_with_context(md, options, &build_ctx)
}

/// Renders a [`Markdown`] to HTML via the context-aware fold, using the full
/// page-level policy carried in `build_ctx`.
///
/// This is the production entry point for
/// [`DarkmatterPage::render_to_browser`](crate::layout::DarkmatterPage::render_to_browser):
/// it folds with component policies, page colors, hyperlink / image styles, and
/// HR defaults baked into every node, then renders directly.
///
/// ## Errors
///
/// Returns [`MarkdownError::InvalidLineRange`](crate::markdown::MarkdownError::InvalidLineRange)
/// for a malformed fenced code-block directive, or
/// [`MarkdownError::RenderTree`](crate::markdown::MarkdownError::RenderTree)
/// wrapping a fatal browser render error.
pub(crate) fn render_tree_html_with_context(
    md: &Markdown,
    options: &HtmlOptions,
    build_ctx: &super::build_context::TreeBuildContext,
) -> crate::markdown::MarkdownResult<PipelineResult<String>> {
    let (doc, fold_diagnostics) = to_render_document_with_context(md, build_ctx)?;
    // Fatal browser-path preflight over the same folded tree the renderer
    // consumes: a malformed code-block directive is a hard error, not a silent
    // degrade. Runs on the single fold — no second traversal of the source.
    validate_code_directives(&doc.root)?;
    let browser_opts = browser_options_from_html_options(options);
    let rendered = render_browser_document_html(&doc, &browser_opts)?;
    Ok(PipelineResult::new(
        rendered.output,
        fold_diagnostics,
        rendered.diagnostics,
    ))
}

/// A browser render for Darkmatter's full-page path, keeping the standalone
/// document, the embeddable body fragment, and the requested page features
/// separate so the caller can place feature assets **inside** an embeddable
/// wrapper rather than in the document `<head>`.
///
/// See [`render_tree_html_page_body`].
pub(crate) struct BrowserPageRender {
    /// The standalone full document (`<!DOCTYPE html>…`), for the no-wrapper
    /// path where the page frame adds nothing so the output stands alone.
    pub document: String,
    /// The document's inner `<head>` content (charset / viewport / title /
    /// design-token `:root` block / `.code-block` panel stylesheet), with no
    /// deferred feature assets. The decorated standalone-document path reuses
    /// this as the real `<head>` payload — wrapping [`body`](Self::body) in a
    /// frame while the head keeps its charset/tokens/panel CSS — rather than
    /// emitting an empty `<head>`.
    pub head: String,
    /// The `<body>` inner HTML fragment — no `<!DOCTYPE>`/`<html>`/`<head>`/
    /// `<body>` — for embedding inside the `<div class="darkmatter-page">`
    /// wrapper.
    pub body: String,
    /// The rolled-up page-level `<style>`/`<script>` assets (design-token
    /// `:root` block, `.code-block` panel stylesheet, component styles) the
    /// wrapper embeds inline before the body so the fragment is self-contained.
    /// Empty when the render produced none.
    pub assets: String,
    /// The deduplicated features the components requested, in first-seen order.
    pub features: Vec<PageFeature>,
}

/// Renders a [`Markdown`] to browser HTML for
/// [`DarkmatterPage::render_to_browser`](crate::layout::DarkmatterPage::render_to_browser),
/// **collecting** requested features instead of injecting them into `<head>`.
///
/// This is the feature-aware, body-only companion to
/// [`render_tree_html_with_context`]. Two things differ from the standalone
/// [`Markdown::as_html`](crate::markdown::Markdown::as_html) path:
///
/// 1. **Interactive Mermaid is the default.** A bare `mermaid` fence lowers to
///    the interactive `<pre class="mermaid">` container (requesting
///    [`PageFeature::MermaidDiagram`]); [`MermaidMode::Image`] stays an explicit
///    static-SVG opt-in and [`MermaidMode::Text`] an explicit code opt-in. The
///    supplied `graphics_mode` still caps the result — [`GraphicsMode::Off`]
///    renders code and [`GraphicsMode::Vector`] caps Interactive to static SVG —
///    so a caller who disables graphics keeps the side-effect-free code form.
/// 2. **Features are deferred.** [`BrowserRenderOptions::defer_feature_injection`]
///    is set, so the collected features are returned in
///    [`BrowserPageRender::features`] and **no** assets are written to `<head>`;
///    the caller resolves them through `resolver`/`feature_context` and injects
///    inline `<style>`/`<script>` into the page wrapper (spec "Body-only
///    renders").
///
/// The render is produced through the body-fragment path
/// ([`render_browser_document_body`]) so [`BrowserPageRender::body`] is the
/// `<body>` inner HTML with no document scaffold, ready to embed inside the
/// page wrapper. [`BrowserPageRender::document`] carries the standalone
/// full-document form for the no-wrapper path where the page frame adds nothing.
///
/// ## Errors
///
/// Returns [`MarkdownError::InvalidLineRange`](crate::markdown::MarkdownError::InvalidLineRange)
/// for a malformed fenced code-block directive, or
/// [`MarkdownError::RenderTree`](crate::markdown::MarkdownError::RenderTree)
/// wrapping a fatal browser render error.
pub(crate) fn render_tree_html_page_body(
    md: &Markdown,
    options: &HtmlOptions,
    build_ctx: &super::build_context::TreeBuildContext,
    resolver: Rc<dyn FeatureResolver>,
    feature_context: FeatureContext,
    graphics_mode: GraphicsMode,
) -> crate::markdown::MarkdownResult<PipelineResult<BrowserPageRender>> {
    let (doc, fold_diagnostics) = to_render_document_with_context(md, build_ctx)?;
    validate_code_directives(&doc.root)?;

    let mut browser_opts = browser_options_from_html_options(options);
    // Darkmatter's full-page browser default is interactive Mermaid; explicit
    // Image / Text choices keep their static-SVG / code opt-in shapes.
    browser_opts.mermaid_mode = match options.mermaid_mode {
        crate::markdown::output::terminal::MermaidMode::Off => BrowserMermaidMode::Interactive,
        crate::markdown::output::terminal::MermaidMode::Image => BrowserMermaidMode::StaticSvg,
        crate::markdown::output::terminal::MermaidMode::Text => BrowserMermaidMode::Code,
    };
    browser_opts.graphics_mode = graphics_mode;
    browser_opts.feature_resolver = resolver;
    browser_opts.feature_context = feature_context;
    // The page owns feature placement (body-only wrapper), so collect the
    // requests without resolving/injecting them into `<head>`.
    browser_opts.defer_feature_injection = true;

    let rendered = render_browser_document_body(&doc, &browser_opts)?;
    Ok(PipelineResult::new(
        BrowserPageRender {
            document: rendered.output.document,
            head: rendered.output.head,
            body: rendered.output.body,
            assets: rendered.output.assets,
            features: rendered.features,
        },
        fold_diagnostics,
        rendered.diagnostics,
    ))
}

/// Validates every fenced code block's DSL directive in a folded [`Document`],
/// returning the first fatal parse error.
///
/// Restores the legacy `output::as_html` contract (`parse_code_info(...)?`): a
/// malformed directive — e.g. an invalid highlight range like `highlight=1-2-3`
/// — is a **hard error** on the browser path, not a silent degrade. The render
/// tree's [`TerminalCodeRenderer`](super::code_renderer::TerminalCodeRenderer)
/// degrades malformed directives to language-only metadata (matching the legacy
/// *terminal* renderer, which used `unwrap_or_default`), so this preflight is
/// the browser-only guard the HTML entry points run before rendering. The
/// reconstructed info string matches
/// [`build_code_meta`](super::code_renderer)'s so validation and rendering parse
/// identical input.
///
/// ## Errors
///
/// Returns the [`MarkdownError`] (typically
/// [`InvalidLineRange`](crate::markdown::MarkdownError::InvalidLineRange)) that
/// [`parse_code_info`](crate::markdown::dsl::parse_code_info) raises for the
/// first malformed directive, in document order.
pub(crate) fn validate_code_directives(root: &RenderNode) -> Result<(), crate::markdown::MarkdownError> {
    use crate::markdown::dsl::parse_code_info;

    if let NodeKind::Code { lang, meta, .. } = &root.kind {
        let lang = lang.as_deref().unwrap_or("");
        let info = match meta.as_deref() {
            Some(m) if !m.trim().is_empty() => format!("{lang} {m}"),
            _ => lang.to_string(),
        };
        parse_code_info(&info)?;
    }
    for child in root.children() {
        validate_code_directives(child)?;
    }
    Ok(())
}

/// Renders a [`Markdown`] to a terminal string via the context-aware
/// render-tree pipeline.
///
/// Builds a [`TreeBuildContext`](super::build_context::TreeBuildContext) from
/// [`TerminalOptions`] (hyperlink / image styles and HR defaults) and folds the
/// markdown into a complete [`Document`] in a single pass — no post-fold
/// decoration or `darkmatter.li` hint is needed.
///
/// HR defaults resolve from [`TerminalOptions::hr_defaults`] only; root-level
/// `hr:` frontmatter is no longer read.
///
/// ## Errors
///
/// Propagates any fatal [`RenderError`] from
/// [`render_terminal_document`].
pub fn render_tree_terminal(
    md: &Markdown,
    options: &TerminalOptions,
) -> crate::markdown::MarkdownResult<PipelineResult<String>> {
    let empty_policies = HashMap::new();
    let hr_defaults_owned = resolve_hr_defaults(md, &options.hr_defaults);
    let build_ctx = super::build_context::TreeBuildContext {
        component_policies: &empty_policies,
        page_color: None,
        page_bg_color: None,
        hyperlink_style: None,
        local_hyperlink_style: None,
        local_image_style: None,
        hr_defaults: hr_defaults_owned.as_ref(),
    };
    render_tree_terminal_with_context(md, options, &build_ctx)
}

/// Renders a [`Markdown`] to a terminal string via the context-aware fold,
/// using the full page-level policy carried in `build_ctx`.
///
/// This is the production entry point for
/// [`DarkmatterPage::render`](crate::layout::DarkmatterPage::render): it folds
/// with component policies, page colors, hyperlink / image styles, and HR
/// defaults baked into every node during construction.
///
/// The image fallback placeholder is set to [`ImagePlaceholder::Block`] — only
/// the page-decorated path uses the `▉ IMAGE[alt]` block form; the direct
/// [`render_tree_terminal`] keeps the generic `[alt]`.
///
/// ## Errors
///
/// Propagates any fatal [`RenderError`] from [`render_terminal_document`].
pub(crate) fn render_tree_terminal_with_context(
    md: &Markdown,
    options: &TerminalOptions,
    build_ctx: &super::build_context::TreeBuildContext,
) -> crate::markdown::MarkdownResult<PipelineResult<String>> {
    let (doc, fold_diagnostics) = to_render_document_with_context(md, build_ctx)?;
    let mut term_opts = terminal_options_from_terminal_options(options);
    term_opts.context.image_placeholder = ImagePlaceholder::Block;
    let rendered = render_terminal_document(&doc, &term_opts)?;
    Ok(PipelineResult::new(
        rendered.output,
        fold_diagnostics,
        rendered.diagnostics,
    ))
}

/// Renders an already-built [`Document`] for [`DarkmatterPage::render`](crate::layout::DarkmatterPage::render),
/// pinning the content-box width **independently** of terminal capability
/// selection.
///
/// The page builds its `Document` once and passes the same owned tree here, so
/// there is no second construction fold (acceptance criterion 3: one tree build,
/// one target fold).
///
/// Unlike [`render_tree_terminal_with_context`], this keeps the content-box
/// width and the terminal capability profile **independent** instead of
/// encoding both through [`TerminalOptions::max_width`].
///
/// `max_width` selects the optimistic pre-render [`Terminal`] — carrying both a
/// width *and* a full capability profile (TrueColor, OSC8). The page splits the
/// two concerns:
///
///   * `optimistic_capabilities` selects the capability profile. The page sets
///     it for *deliberate frame geometry only* — never a matched component
///     policy — so a centered table or other matched layout cannot promote
///     unrelated content to optimistic TrueColor + OSC8 the ambient terminal
///     never advertised (review-5 finding 1). A geometry frame still reaches the
///     optimistic profile faithfully; a no-geometry page (matched policy or not)
///     renders at the ambient capabilities a no-policy page would.
///   * `content_width` is pinned afterwards regardless of that profile, so a
///     no-geometry page can render at the ambient width independent of the
///     selected capabilities — a split `max_width` alone cannot express. Painted
///     construction color still rides `options.color_depth` (the captured
///     depth), applied by the adapter over the selected base.
///
/// Color depth, when the page paints construction color, still rides
/// `options.color_depth` and is applied by the adapter over the selected base.
///
/// ## Errors
///
/// Returns any fatal fold-time [`MarkdownError`], including [`RenderError`]
/// from [`render_terminal_document`] converted to
/// [`MarkdownError::RenderTree`](crate::markdown::MarkdownError::RenderTree).
pub(crate) fn render_page_terminal_document(
    doc: &Document,
    fold_diagnostics: Vec<Diagnostic>,
    options: &TerminalOptions,
    content_width: u16,
    optimistic_capabilities: bool,
) -> crate::markdown::MarkdownResult<PipelineResult<String>> {
    // Select the capability profile via the adapter's optimistic/ambient base,
    // then pin the content width independently on top.
    let mut capability_options = options.clone();
    capability_options.max_width = optimistic_capabilities.then_some(content_width);
    let mut term_opts = terminal_options_from_terminal_options(&capability_options);
    // Pin only the content-box width. Set it on both the context and its
    // terminal snapshot so a component reading either agrees on the width,
    // independent of the capability profile selected above.
    let width = u32::from(content_width);
    term_opts.context.terminal.fixed_width = Some(width);
    term_opts.context.width = width;
    term_opts.context.available_width = width;
    term_opts.context.image_placeholder = ImagePlaceholder::Block;
    let rendered = render_terminal_document(doc, &term_opts)
        .map_err(crate::markdown::MarkdownError::RenderTree)?;
    Ok(PipelineResult::new(
        rendered.output,
        fold_diagnostics,
        rendered.diagnostics,
    ))
}

/// Renders a [`Markdown`] back to a Markdown string via the render-tree
/// pipeline.
///
/// Useful primarily for parity and round-trip testing.
///
/// ## Errors
///
/// Propagates any fatal [`RenderError`] from
/// [`render_markdown_document`].
pub fn render_tree_markdown(md: &Markdown) -> crate::markdown::MarkdownResult<PipelineResult<String>> {
    render_tree_markdown_dialect(md, MarkdownDialect::Markdown)
}

/// Renders a [`Markdown`] to MarkdownPlus via the render-tree pipeline,
/// using the provided construction-time context so component policy and
/// disclosure inline styles are preserved.
///
/// ## Errors
///
/// Propagates any fatal [`RenderError`] from
/// [`render_markdown_document`].
pub(crate) fn render_tree_markdown_plus_with_context(
    md: &Markdown,
    ctx: &super::build_context::TreeBuildContext,
) -> crate::markdown::MarkdownResult<PipelineResult<String>> {
    let (doc, fold_diagnostics) = to_render_document_with_context(md, ctx)?;
    let opts = MarkdownRenderOptions {
        dialect: MarkdownDialect::MarkdownPlus,
        strictness: RenderStrictness::Warn,
        style: None,
    };
    let rendered = render_markdown_document(&doc, &opts)?;
    Ok(PipelineResult::new(
        rendered.output,
        fold_diagnostics,
        rendered.diagnostics,
    ))
}

/// Renders a [`Markdown`] to either standard Markdown or MarkdownPlus via the
/// render-tree pipeline.
///
/// ## Errors
///
/// Propagates any fatal [`RenderError`] from
/// [`render_markdown_document`].
pub fn render_tree_markdown_dialect(
    md: &Markdown,
    dialect: MarkdownDialect,
) -> crate::markdown::MarkdownResult<PipelineResult<String>> {
    let (doc, fold_diagnostics) = to_render_document(md)?;
    let opts = MarkdownRenderOptions {
        dialect,
        strictness: RenderStrictness::Warn,
        style: None,
    };
    let rendered = render_markdown_document(&doc, &opts)?;
    Ok(PipelineResult::new(
        rendered.output,
        fold_diagnostics,
        rendered.diagnostics,
    ))
}

/// Maps [`HtmlOptions`] to [`BrowserRenderOptions`].
///
/// The [`TerminalCodeRenderer`] hook is wired in so fenced code blocks
/// reproduce darkmatter's syntax-highlighted HTML (title block, line-number
/// table, highlighted-line markup) rather than the render tree's plain
/// `<pre><code>` fallback.
///
/// [`HtmlOptions::hr_css_variables`](crate::markdown::output::HtmlOptions::hr_css_variables)
/// are lowered (via [`hr_root_variables`]) onto
/// [`PageOptions::css_variables`](renderable::browser::PageOptions::css_variables),
/// which the page renderer emits as `--{key}: {value};` declarations in its
/// `:root` block. The HR SVG keeps its literal `var(--hr-*, fallback)`
/// expressions, so the declared values resolve against them in the browser —
/// reproducing the legacy `:root` override contract. A page is built whenever
/// either the code-block stylesheet or an HR override applies, so the override
/// still takes effect when `include_styles` is unset.
///
/// When `opts.include_styles` is set, the page hook
/// ([`BrowserRenderOptions::page`]) carries a [`renderable::stylesheet::Stylesheet`]
/// with the `.code-block` panel-background rule (see
/// [`code_block_stylesheet`]). The render tree's full-page renderer emits the
/// design-token `:root` stylesheet but no `.code-block` rule, so without this
/// the syntax-highlighted `<div class="code-block">` would have no background.
/// The page's default stylesheet is empty (the `:root` tokens ride a separate
/// `css_variables` channel), so supplying one via `page_options` augments
/// rather than replaces the page styles.
///
/// Maps `mermaid_mode` to the render-tree [`BrowserMermaidMode`] so the
/// tree browser path honors the same Mermaid opt-in contract as the legacy
/// renderer.
///
/// Legacy `MermaidMode::Image` maps to [`BrowserMermaidMode::StaticSvg`]: the
/// spec's promoted browser form is a pre-rendered static `<svg>`. The
/// client-side mermaid.js path ([`BrowserMermaidMode::Interactive`]) is an
/// orthogonal, default-off browser opt-in and is not reachable from the legacy
/// enum.
fn browser_options_from_html_options(opts: &HtmlOptions) -> BrowserRenderOptions {
    use renderable::browser::PageOptions;
    use renderable::tree::BrowserMermaidMode;

    let mermaid_mode = match opts.mermaid_mode {
        crate::markdown::output::terminal::MermaidMode::Off => BrowserMermaidMode::Code,
        crate::markdown::output::terminal::MermaidMode::Text => BrowserMermaidMode::Code,
        crate::markdown::output::terminal::MermaidMode::Image => BrowserMermaidMode::StaticSvg,
    };

    let stylesheet = opts.include_styles.then(|| code_block_stylesheet(opts));
    let css_variables = hr_root_variables(&opts.hr_css_variables);
    let page = (stylesheet.is_some() || css_variables.is_some()).then(|| PageOptions {
        stylesheet,
        css_variables,
        ..PageOptions::default()
    });

    BrowserRenderOptions {
        strictness: RenderStrictness::Warn,
        raw_html: RawHtmlPolicy::Escape,
        page,
        // Carry the caller's resolved `HtmlOptions` (code theme, page color
        // mode, code-block mode, line numbers) into the code renderer so the
        // highlighted markup uses the same mode/theme policy as the page frame
        // and the `.code-block` stylesheet, rather than `HtmlOptions::default()`
        // (review-1 finding 2).
        code_renderer: Some(Rc::new(
            TerminalCodeRenderer::new().with_html_options(opts.clone()),
        )),
        mermaid_mode,
        ..Default::default()
    }
}

/// Builds the `.code-block` panel-background stylesheet for the browser path.
///
/// The background color resolves through
/// [`code_block_background_hex`](crate::markdown::output::html::code_block_background_hex),
/// which applies the same `CodeHighlighter` / inverted-color-mode path the code
/// renderer uses, so the injected `.code-block` rule and the highlighted code
/// markup agree on the value (Defect D: code blocks invert their theme variant
/// for page contrast). Only the load-bearing `.code-block` background rule is injected;
/// the remaining cosmetic rules (`.code-block-title`, gutter, `pre`/`code`, …)
/// are emitted inline by the code-renderer hook's own markup.
fn code_block_stylesheet(opts: &HtmlOptions) -> renderable::stylesheet::Stylesheet {
    use renderable::stylesheet::{CssColor, CssColorProp, CssRule, CssStyle, Stylesheet};

    let hex = crate::markdown::output::html::code_block_background_hex(opts);
    let mut sheet = Stylesheet::new();
    if let Ok(color) = CssColor::hex(&hex) {
        let style = CssStyle::new().add(CssColorProp::BackgroundColor, color);
        sheet.push(CssRule::new(".code-block", style));
    }
    sheet
}

/// Maps [`TerminalOptions`] to [`TerminalRenderOptions`].
///
/// Uses an optimistic [`Terminal`] when `max_width` is pinned; otherwise
/// falls back to detection. `opts.color_depth`, when set, overrides the
/// terminal's resolved color depth so callers can pin the tree renderer to
/// `ColorDepth::None` (matching the legacy renderer's no-color contract)
/// without depending on host capability detection. `image_mode` and
/// `mermaid_mode` map onto the context's graphics tier and Mermaid promotion
/// opt-in (see the inline mapping below).
///
/// The [`TerminalCodeRenderer`] hook is always wired in so fenced code blocks
/// reproduce darkmatter's syntax-highlighted code-block output (header row
/// plus highlighted body) rather than the render tree's plain-fence fallback.
/// Closes review-10 finding 2: previously the entry point passed
/// `code_renderer: None`, so the user-observable tree terminal path exercised
/// the generic renderable fallback instead of the darkmatter code path the
/// renderer was built for.
///
/// Closes review-5 finding 1: previously the tree entry point silently
/// dropped `TerminalOptions::color_depth`, so `ColorDepth::None` callers and
/// the `migration/terminal_no_color` benchmark group both measured a
/// TrueColor tree context.
///
/// Phase 2 (centralize theme resolution) removes the `term.color_mode =
/// opts.color_mode` rebuild that created an independent code-panel mode. The
/// `Terminal` constructed here is the single source of truth for the page
/// surface and the nested code-block panel: `code_block_mode.resolve(term.
/// color_mode())` produces the panel variant, and `term.color_mode()` itself
/// is the page variant. Callers who want a specific mode should construct a
/// `Terminal` with that mode (the page path through
/// [`DarkmatterPage::render`](crate::layout::DarkmatterPage::render) already
/// captures the terminal; direct [`Markdown::as_terminal`](crate::markdown::Markdown::as_terminal)
/// callers inherit the default-detection `Terminal::default()`). The
/// `opts.color_mode` field is preserved for backward compatibility with the
/// page path (where it is the [`ColorMode::Unknown`] fallback in
/// [`LayoutContext::from_page`](crate::layout::context::LayoutContext::from_page))
/// but no longer overrides the terminal here.
fn terminal_options_from_terminal_options(opts: &TerminalOptions) -> TerminalRenderOptions {
    let mut term = Terminal::default();
    if let Some(width) = opts.max_width {
        if term.is_tty {
            term.fixed_width = Some(u32::from(width));
        } else {
            term = Terminal::new_optimistic(u32::from(width));
        }
    }
    if let Some(depth) = opts.color_depth {
        term.color_depth = darkmatter_color_depth_to_terminal(depth);
    }

    // Map legacy image_mode onto the graphics fidelity tier so the tree
    // renderer honors the same opt-in / never / force contract as the legacy
    // terminal renderer. Mermaid promotion and inline image rendering both
    // key off this field.
    let graphics_mode = match opts.image_mode {
        crate::markdown::output::terminal::TerminalImageMode::Never => {
            GraphicsMode::Off
        }
        crate::markdown::output::terminal::TerminalImageMode::Auto => {
            GraphicsMode::Rich
        }
        crate::markdown::output::terminal::TerminalImageMode::Force => {
            GraphicsMode::Rich
        }
    };

    // Map the legacy Mermaid opt-in onto the terminal Mermaid promotion mode.
    // `GraphicsMode` is only the ceiling: a fence is promoted to a raster image
    // solely when the caller opted in via `MermaidMode::Image`. `Off` / `Text`
    // keep Mermaid as code, preserving the public default.
    let mermaid_mode = match opts.mermaid_mode {
        crate::markdown::output::terminal::MermaidMode::Image => TerminalMermaidMode::Image,
        crate::markdown::output::terminal::MermaidMode::Off
        | crate::markdown::output::terminal::MermaidMode::Text => TerminalMermaidMode::Code,
    };

    let mut context = TerminalRenderContext::from_terminal(&term);
    context.graphics_mode = graphics_mode;
    context.mermaid_mode = mermaid_mode;
    context.image_base_path = opts.base_path.clone();
    // The darkmatter document pipeline wraps top-level prose to the content
    // width, matching the legacy `for_terminal` renderer.
    context.wrap_prose = true;
    // Carry the requested code theme as its canonical kebab name so the code
    // renderer hook resolves the same `ThemePair` a `with_code_theme(...)`
    // caller asked for, rather than always painting the default theme.
    context.code_theme = Some(opts.code_theme.kebab_name().to_string());
    // Forward the page line-number toggle so fenced code blocks render their
    // gutter, matching the legacy renderer's `include_line_numbers`.
    context.line_numbers = opts.include_line_numbers;
    // Block quotes use the legacy quarter-block bar (`▐   `, 4 cols) rather
    // than the shared component's `│ ` border, matching `for_terminal`.
    context.blockquote_prefix = Some("▐   ".to_string());
    if opts.image_mode == crate::markdown::output::terminal::TerminalImageMode::Force {
        context.force_graphics = true;
    }
    // Phase 2 (centralize theme resolution): the `Terminal` above carries the
    // capability profile (width, color depth, is_tty). The page surface and
    // the nested code-block panel must resolve against the *caller's*
    // `opts.color_mode` — for the page path that is the captured
    // `DarkmatterPage` terminal's mode; for direct `Markdown::as_terminal`
    // callers it is the requested mode. Setting `context.color_mode` here
    // and binding an *unbound* code renderer below keeps the page surface
    // and the code panel on the same source of truth, removing the
    // pre-Phase-2 dual-source defect (an env-only `Terminal::default()` mode
    // disagreed with the caller-supplied `opts.color_mode`).
    context.color_mode = opts.color_mode;

    // Inline code spans take their foreground/background from the prose theme
    // reduced to the page's color mode — the same source the syntect code panel
    // uses — so a `code` span sits on a subtle theme-derived band instead of
    // reverse-video. Resolved here (syntect lives in darkmatter) and forwarded
    // to the generic terminal renderer, which cannot load themes itself.
    let inline_mode = opts.color_mode.resolve_unknown();
    let (inline_fg, inline_bg) =
        crate::markdown::highlighting::themes::inline_code_colors(opts.prose_theme, inline_mode);
    context.inline_code_color = Some(inline_fg);
    context.inline_code_background = Some(inline_bg);

    TerminalRenderOptions {
        context,
        strictness: RenderStrictness::Warn,
        // Bind a code renderer with no terminal: the renderer's
        // `terminal_mode` falls back to `context.color_mode()` (set from
        // `opts.color_mode` above), so the code panel inverts the caller's
        // mode — the same source feeds the page and the panel. The
        // caller's `opts.code_block_mode` is forwarded so direct
        // `Markdown::as_terminal(opts)` callers (e.g. `md schema about
        // --code-block ...`) still control the panel's contrast against
        // the page.
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new_with_code_block_mode(
            opts.code_block_mode,
        ))),
    }
}

/// Maps darkmatter's [`ColorDepth`] onto biscuit-terminal's
/// [`TerminalColorDepth`].
///
/// Mirrors the mapping used by the legacy terminal renderer
/// (`darkmatter::markdown::output::terminal`) so the tree path honors the
/// same `TerminalOptions::color_depth` contract.
fn darkmatter_color_depth_to_terminal(depth: ColorDepth) -> TerminalColorDepth {
    match depth {
        ColorDepth::TrueColor => TerminalColorDepth::TrueColor,
        ColorDepth::Colors256 => TerminalColorDepth::Enhanced,
        ColorDepth::Colors16 => TerminalColorDepth::Basic,
        ColorDepth::None => TerminalColorDepth::None,
    }
}

/// Derives a [`SourceDescriptor`] from `md.source()`.
///
/// A file-backed darkmatter document maps to [`SourceDescriptor::File`] so the
/// source kind survives into every [`SourceLocation`](renderable::tree::SourceLocation)
/// emitted through [`to_render_document`] (the path diagnostics and
/// source-aware tools consume). URL and unknown sources have no file backing,
/// so they fold to a [`SourceDescriptor::Virtual`] named after the URL (or a
/// generic `"darkmatter"` placeholder).
fn derive_source(md: &Markdown) -> SourceDescriptor {
    use crate::markdown::compose::ComposeSource;
    match md.source() {
        Some(ComposeSource::File(path)) => SourceDescriptor::File {
            path: path.to_path_buf(),
        },
        Some(ComposeSource::Url(url)) => SourceDescriptor::Virtual {
            name: url.to_string(),
        },
        _ => SourceDescriptor::Virtual {
            name: "darkmatter".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::highlighting::{CodeHighlighter, ThemePair};
    use syntect::easy::HighlightLines;
    use syntect::highlighting::Color;

    /// Smoke: every entry point renders a small fixture without panicking and
    /// surfaces fold/render diagnostics separately.
    const FIXTURE: &str = "# Heading\n\nA paragraph with **strong** text.\n";

    fn fg_sgr(color: Color) -> String {
        format!("\x1b[38;2;{};{};{}m", color.r, color.g, color.b)
    }

    fn bg_sgr(color: Color) -> String {
        format!("\x1b[48;2;{};{};{}m", color.r, color.g, color.b)
    }

    fn one_half_yaml_color(
        mode: crate::markdown::highlighting::ColorMode,
        line: &str,
        token: &str,
    ) -> Color {
        let highlighter = CodeHighlighter::new(ThemePair::OneHalf, mode);
        let syntax = highlighter
            .syntax_set()
            .find_syntax_by_extension("yaml")
            .expect("yaml syntax");
        let mut hl = HighlightLines::new(syntax, highlighter.theme());
        let token_start = line
            .find(token)
            .unwrap_or_else(|| panic!("missing token {token:?} in source line {line:?}"));
        let mut offset = 0;
        hl.highlight_line(line, highlighter.syntax_set())
            .expect("highlight line")
            .into_iter()
            .find_map(|(style, text)| {
                let end = offset + text.len();
                let matched = (offset..end).contains(&token_start);
                offset = end;
                matched.then_some(style.foreground)
            })
            .unwrap_or_else(|| panic!("missing token {token:?} in highlighted line {line:?}"))
    }

    fn one_half_background(mode: crate::markdown::highlighting::ColorMode) -> Color {
        CodeHighlighter::new(ThemePair::OneHalf, mode)
            .theme()
            .settings
            .background
            .expect("theme background")
    }

    fn assert_yaml_colors(output: &str, expected_mode: crate::markdown::highlighting::ColorMode) {
        let background = one_half_background(expected_mode);
        let object_key = one_half_yaml_color(expected_mode, "  foo: string", "foo");
        let string_value = one_half_yaml_color(expected_mode, "  foo: string", "string");

        assert!(
            output.contains(&bg_sgr(background)),
            "expected OneHalf {:?} background RGB({},{},{}), raw:\n{output:?}",
            expected_mode,
            background.r,
            background.g,
            background.b,
        );
        assert!(
            output.contains(&format!("{}foo", fg_sgr(object_key))),
            "expected OneHalf {:?} YAML key RGB({},{},{}), raw:\n{output:?}",
            expected_mode,
            object_key.r,
            object_key.g,
            object_key.b,
        );
        assert!(
            output.contains(&format!("{}string", fg_sgr(string_value))),
            "expected OneHalf {:?} YAML scalar RGB({},{},{}), raw:\n{output:?}",
            expected_mode,
            string_value.r,
            string_value.g,
            string_value.b,
        );
    }

    fn terminal_opts_for_pipeline(
        mode: crate::markdown::highlighting::ColorMode,
    ) -> TerminalOptions {
        TerminalOptions {
            code_theme: ThemePair::OneHalf,
            prose_theme: ThemePair::OneHalf,
            color_mode: mode,
            color_depth: Some(ColorDepth::TrueColor),
            max_width: Some(80),
            ..TerminalOptions::default()
        }
    }

    #[test]
    fn to_render_document_smoke() {
        let md: Markdown = FIXTURE.into();
        let (doc, diags) = to_render_document(&md).expect("fold must succeed");
        assert!(diags.is_empty(), "smoke fixture must fold cleanly");
        assert!(!doc.root.children().is_empty(), "fold produced no nodes");
    }

    #[test]
    fn render_tree_html_smoke() {
        let md: Markdown = FIXTURE.into();
        let result = render_tree_html(&md, &HtmlOptions::default()).expect("html render");
        assert!(result.output.contains("<h1"));
        assert!(result.output.contains("strong"));
    }

    #[test]
    fn render_tree_terminal_smoke() {
        let md: Markdown = FIXTURE.into();
        // Pin max_width so the test doesn't depend on the host terminal.
        let opts = TerminalOptions {
            max_width: Some(80),
            ..TerminalOptions::default()
        };
        let result = render_tree_terminal(&md, &opts).expect("terminal render");
        assert!(result.output.contains("Heading"));
    }

    #[test]
    fn render_tree_terminal_inline_code_uses_theme_background() {
        // Regression guard: inline code derives its background from the prose
        // theme (reduced to the page color mode), not reverse-video.
        let md: Markdown = "Use the `review.md` file.".into();
        let opts = terminal_opts_for_pipeline(crate::markdown::highlighting::ColorMode::Dark);
        let result = render_tree_terminal(&md, &opts).expect("terminal render");

        let bg = one_half_background(crate::markdown::highlighting::ColorMode::Dark);
        assert!(
            result
                .output
                .contains(&format!("\x1b[48;2;{};{};{}m", bg.r, bg.g, bg.b)),
            "inline code should paint the OneHalf dark background band, raw:\n{:?}",
            result.output,
        );
        assert!(
            !result.output.contains("\x1b[7m"),
            "inline code must not use reverse video, raw:\n{:?}",
            result.output,
        );
    }

    #[test]
    fn render_tree_markdown_smoke() {
        let md: Markdown = FIXTURE.into();
        let result = render_tree_markdown(&md).expect("markdown render");
        assert!(result.output.contains("Heading"));
        assert!(result.output.contains("strong"));
    }

    #[test]
    fn render_tree_markdown_plus_smoke() {
        let md: Markdown = FIXTURE.into();
        let result = render_tree_markdown_dialect(&md, MarkdownDialect::MarkdownPlus)
            .expect("markdown-plus render");
        assert!(result.output.contains("Heading"));
    }

    /// A file-backed darkmatter document must resolve to a
    /// [`SourceDescriptor::File`] (carrying its path), not be downgraded to a
    /// [`SourceDescriptor::Virtual`]. URL / unknown sources stay virtual.
    /// Pins the fix for review-12 finding 2.
    #[test]
    fn derive_source_preserves_file_backing() {
        use crate::markdown::compose::ComposeSource;
        use renderable::tree::SourceId;
        use std::path::PathBuf;

        // File-backed: must surface as `SourceDescriptor::File`.
        let md: Markdown = Markdown::from("Body paragraph.\n")
            .with_source(ComposeSource::File(PathBuf::from("docs/example.md")));
        let (doc, _diags) = to_render_document(&md).expect("fold must succeed");
        assert_eq!(
            doc.sources.resolve(SourceId(0)),
            Some(&SourceDescriptor::File {
                path: PathBuf::from("docs/example.md"),
            }),
            "file-backed Markdown must register a SourceDescriptor::File",
        );

        // No source: stays virtual.
        let md: Markdown = "Body paragraph.\n".into();
        let (doc, _diags) = to_render_document(&md).expect("fold must succeed");
        assert_eq!(
            doc.sources.resolve(SourceId(0)),
            Some(&SourceDescriptor::Virtual {
                name: "darkmatter".to_string(),
            }),
            "sourceless Markdown must register a virtual descriptor",
        );
    }

    #[test]
    fn frontmatter_attaches_via_entry_point() {
        let raw = "---\ntitle: Smoke\n---\n\nBody paragraph.\n";
        let md: Markdown = raw.into();
        let (doc, _diags) = to_render_document(&md).expect("fold must succeed");
        let fm = doc
            .metadata
            .frontmatter
            .as_ref()
            .expect("frontmatter must thread through to_render_document");
        assert!(fm.raw.contains("title: Smoke"));
    }

    #[test]
    fn pipeline_result_keeps_fold_and_render_streams_separate() {
        let md: Markdown = FIXTURE.into();
        let result = render_tree_html(&md, &HtmlOptions::default()).expect("html render");
        // The streams are independent vectors; even when both are empty for a
        // clean fixture, the type-level separation is what the spec
        // (diagnostic-model.md) requires.
        assert!(result.is_clean(), "smoke fixture must be diagnostics-free");
    }

    // -----------------------------------------------------------------------
    // Span-aware-entrypoint coverage (review-2 finding 1): mark/dim/HR-attribute
    // constructs must be visible through every target entry point, not just
    // through the lower-level `fold_markdown_spanned_with_frontmatter` helper.
    // -----------------------------------------------------------------------

    /// `to_render_document` must produce an `Extended { token: "mark" }` node
    /// for `==highlighted==`, proving the entry point uses the span-aware fold.
    #[test]
    fn to_render_document_uses_span_aware_fold_for_mark() {
        use renderable::tree::NodeKind;

        fn has_mark(node: &renderable::tree::RenderNode) -> bool {
            if matches!(&node.kind, NodeKind::Extended { token, .. } if token == "mark") {
                return true;
            }
            node.children().iter().any(has_mark)
        }

        let md: Markdown = "plain ==highlighted== after\n".into();
        let (doc, diags) = to_render_document(&md).expect("fold must succeed");
        assert!(
            diags.is_empty(),
            "mark fixture must fold cleanly: {diags:?}"
        );
        assert!(
            has_mark(&doc.root),
            "to_render_document must surface a `mark` Extended node — entry point did not use the span-aware fold",
        );
    }

    /// `to_render_document` must produce an `Extended { token: "dim" }` node
    /// for `⌄dimmed⌄`, proving the entry point uses the span-aware fold.
    #[test]
    fn to_render_document_uses_span_aware_fold_for_dim() {
        use renderable::tree::NodeKind;

        fn has_dim(node: &renderable::tree::RenderNode) -> bool {
            if matches!(&node.kind, NodeKind::Extended { token, .. } if token == "dim") {
                return true;
            }
            node.children().iter().any(has_dim)
        }

        let md: Markdown = "normal \u{2304}dimmed\u{2304} after\n".into();
        let (doc, diags) = to_render_document(&md).expect("fold must succeed");
        assert!(diags.is_empty(), "dim fixture must fold cleanly: {diags:?}");
        assert!(
            has_dim(&doc.root),
            "to_render_document must surface a `dim` Extended node",
        );
    }

    /// `to_render_document` must rewrite `--- { style: waves }` into a
    /// `ThematicBreak` carrying the typed `thematic_break.kind`.
    #[test]
    fn to_render_document_uses_span_aware_fold_for_hr_attributes() {
#[cfg(test)]
use renderable::tree::{NodeKind, RenderNode};

        fn find_hr(node: &RenderNode) -> Option<&RenderNode> {
            if matches!(node.kind, NodeKind::ThematicBreak) {
                return Some(node);
            }
            for child in node.children() {
                if let Some(found) = find_hr(child) {
                    return Some(found);
                }
            }
            None
        }

        let md: Markdown = "--- { style: waves }\n".into();
        let (doc, _diags) = to_render_document(&md).expect("fold must succeed");
        let hr = find_hr(&doc.root).expect("HR-attribute paragraph must fold to a ThematicBreak");
        assert_eq!(
            hr.attrs.thematic_break_ref().and_then(|h| h.kind),
            Some(renderable::tree::HrKind::Waves),
            "HR-attribute styling must survive to_render_document",
        );
    }

    /// The terminal entry point must keep the mark text and surface ANSI
    /// styling — proving the span-aware fold's `Extended { token: "mark" }`
    /// reaches the terminal renderer through `render_tree_terminal`.
    #[test]
    fn render_tree_terminal_preserves_mark_text() {
        let md: Markdown = "before ==highlighted== after\n".into();
        let opts = TerminalOptions {
            max_width: Some(80),
            ..TerminalOptions::default()
        };
        let result = render_tree_terminal(&md, &opts).expect("terminal render");
        assert!(
            result.output.contains("highlighted"),
            "mark text must survive the terminal entry point",
        );
    }

    /// The HTML entry point must keep the mark text inside a `<mark>` element
    /// — proving the span-aware fold's `Extended { token: "mark" }` reaches
    /// the browser renderer through `render_tree_html`.
    #[test]
    fn render_tree_html_preserves_mark_text() {
        let md: Markdown = "before ==highlighted== after\n".into();
        let result = render_tree_html(&md, &HtmlOptions::default()).expect("html render");
        assert!(
            result.output.contains("highlighted"),
            "mark text must survive the HTML entry point",
        );
    }

    /// The Markdown round-trip entry point must keep the mark text and re-emit
    /// it (the exact MarkdownPlus form is renderer-dependent; the visible word
    /// must survive).
    #[test]
    fn render_tree_markdown_preserves_mark_text() {
        let md: Markdown = "before ==highlighted== after\n".into();
        let result = render_tree_markdown(&md).expect("markdown render");
        assert!(
            result.output.contains("highlighted"),
            "mark text must survive the Markdown entry point",
        );
    }

    /// The HTML entry point must lower `Style.emphasis.dim` to inline CSS so
    /// the visual dim styling reaches the browser surface — closes review-3
    /// finding 1 for the browser target.
    #[test]
    fn render_tree_html_emits_dim_styling_for_dim_span() {
        let md: Markdown = "normal \u{2304}dimmed\u{2304} after\n".into();
        let result = render_tree_html(&md, &HtmlOptions::default()).expect("html render");
        assert!(
            result.output.contains("dimmed"),
            "dim text must survive the HTML entry point",
        );
        assert!(
            result.output.contains("opacity:0.6") || result.output.contains("opacity: 0.6"),
            "HTML output must lower Style.emphasis.dim to `opacity:0.6`; raw output:\n{}",
            result.output,
        );
    }

    /// Review-5 finding 1: `TerminalOptions::color_depth = None` must
    /// propagate into the tree's terminal context so styled content
    /// renders without any ANSI **color** SGRs. Previously the entry
    /// point silently ignored `color_depth` and built a TrueColor
    /// optimistic terminal regardless. Style SGRs (bold/italic/strike)
    /// are orthogonal — biscuit-terminal emits those without color, and
    /// stripping them is the responsibility of an explicit
    /// "no-formatting" mode, not the color-depth knob.
    ///
    /// The companion
    /// [`terminal_options_mapping_pins_color_depth_translation`] test
    /// pins the option-to-context wiring; this test pins the
    /// observable rendered-bytes contract.
    #[test]
    fn render_tree_terminal_color_depth_none_emits_no_color_sgrs() {
        // Mix prose, inline code (lowers to reverse video), and a link
        // so multiple color-capable paths are exercised in one fixture.
        let md: Markdown =
            "# Heading\n\nText with `code` and [link](https://example.com).\n".into();

        let no_color_opts = TerminalOptions {
            max_width: Some(80),
            color_depth: Some(ColorDepth::None),
            ..TerminalOptions::default()
        };
        let no_color_result = render_tree_terminal(&md, &no_color_opts).expect("no-color render");
        let no_color_raw = &no_color_result.output;
        // Visible text must survive even when colors are stripped.
        assert!(
            no_color_raw.contains("Heading"),
            "heading dropped: {no_color_raw:?}"
        );
        assert!(
            no_color_raw.contains("code"),
            "code text dropped: {no_color_raw:?}"
        );
        assert!(
            no_color_raw.contains("link"),
            "link text dropped: {no_color_raw:?}"
        );
        assert!(
            !contains_color_sgr(no_color_raw),
            "ColorDepth::None must produce no ANSI color SGRs; raw output:\n{no_color_raw:?}",
        );
    }

    /// Returns `true` if `raw` contains any ANSI color SGR sequence
    /// (foreground or background, basic, bright, 256, or RGB). Style
    /// SGRs (bold, italic, strikethrough, …) deliberately don't match.
    fn contains_color_sgr(raw: &str) -> bool {
        // Scan every CSI introducer and inspect the parameter list.
        let mut rest = raw;
        while let Some(idx) = rest.find("\u{1b}[") {
            let after = &rest[idx + 2..];
            let end = after.find('m').unwrap_or(after.len());
            let params = &after[..end];
            for param in params.split(';') {
                if is_color_sgr_param(param) {
                    return true;
                }
            }
            rest = &after[end.min(after.len())..];
        }
        false
    }

    /// Returns `true` for a single SGR parameter that names a color
    /// attribute. Style attributes (bold=1, italic=3, strike=9, …) and
    /// resets (0, 22, 23, 29, 39, 49) are not colors.
    fn is_color_sgr_param(param: &str) -> bool {
        match param.parse::<u32>() {
            // 30–37 basic FG, 40–47 basic BG, 38/48 extended, 90–97
            // bright FG, 100–107 bright BG.
            Ok(n) => matches!(n, 30..=37 | 38 | 40..=47 | 48 | 90..=97 | 100..=107),
            Err(_) => false,
        }
    }

    /// The terminal entry point's mapping helper must lower every
    /// darkmatter `ColorDepth` variant onto the matching biscuit-terminal
    /// `ColorDepth`. This pins the conversion table so a silent reordering
    /// of either enum is caught at test time.
    #[test]
    fn terminal_options_mapping_pins_color_depth_translation() {
        for (input, expected) in [
            (ColorDepth::TrueColor, TerminalColorDepth::TrueColor),
            (ColorDepth::Colors256, TerminalColorDepth::Enhanced),
            (ColorDepth::Colors16, TerminalColorDepth::Basic),
            (ColorDepth::None, TerminalColorDepth::None),
        ] {
            let opts = TerminalOptions {
                max_width: Some(80),
                color_depth: Some(input),
                ..TerminalOptions::default()
            };
            let term_opts = terminal_options_from_terminal_options(&opts);
            assert_eq!(
                term_opts.context.terminal.color_depth, expected,
                "{input:?} must map to {expected:?}",
            );
        }
    }

    /// When `color_depth` is unset, the entry point keeps the underlying
    /// optimistic / detected terminal's color depth — no spurious downgrade.
    #[test]
    fn terminal_options_mapping_leaves_unspecified_color_depth_alone() {
        let opts = TerminalOptions {
            max_width: Some(80),
            color_depth: None,
            ..TerminalOptions::default()
        };
        let term_opts = terminal_options_from_terminal_options(&opts);
        // `Terminal::new_optimistic` advertises `TrueColor`; without a
        // pinned `color_depth` that must reach the tree context unchanged.
        assert_eq!(
            term_opts.context.terminal.color_depth,
            TerminalColorDepth::TrueColor
        );
    }

    /// The terminal entry point must emit the dim SGR sequence (`ESC [ 2 m`)
    /// for a dim span — closes review-3 finding 1 for the terminal target.
    #[test]
    fn render_tree_terminal_emits_dim_sgr_for_dim_span() {
        let md: Markdown = "normal \u{2304}dimmed\u{2304} after\n".into();
        let opts = TerminalOptions {
            max_width: Some(80),
            ..TerminalOptions::default()
        };
        let result = render_tree_terminal(&md, &opts).expect("terminal render");
        assert!(
            result.output.contains("dimmed"),
            "dim text must survive the terminal entry point",
        );
        // The dim SGR open code is `ESC [ 2 m`. It may appear standalone or
        // combined with other layers in a single `ESC [ ... m` sequence
        // (e.g. `\x1b[0;2m`); accept any position within a combined run.
        let raw = &result.output;
        let has_dim_sgr = raw.contains("\u{1b}[2m")
            || raw.contains("\u{1b}[2;")
            || raw.contains(";2m")
            || raw.contains(";2;");
        assert!(
            has_dim_sgr,
            "terminal output must include dim SGR (\\x1b[2m); raw output:\n{raw:?}",
        );
    }

    // -----------------------------------------------------------------------
    // Review-6 finding 1: the safe raw-HTML escape default must be observable
    // through the Darkmatter HTML entry point, not just through the lower
    // browser renderer. The parity test (`render_tree_parity_raw_html`)
    // deliberately overrides the tree side to `RawHtmlPolicy::Allow`, so the
    // production safety default would otherwise be unpinned at the adapter
    // boundary. DMTR-9 requires raw Markdown HTML to escape by default.
    // -----------------------------------------------------------------------

    /// Block-level raw HTML must be escaped through `render_tree_html` so
    /// `<div>` / `<script>` source bytes never reach the rendered output as a
    /// live tag.
    #[test]
    fn render_tree_html_escapes_raw_block_html_by_default() {
        let md: Markdown =
            "<div class=\"evil\">block content</div>\n\nTrailing paragraph.\n".into();
        let result = render_tree_html(&md, &HtmlOptions::default()).expect("html render");
        let out = &result.output;
        assert!(
            out.contains("&lt;div"),
            "block raw HTML must be escaped to `&lt;div`; raw output:\n{out}",
        );
        assert!(
            !out.contains("<div class=\"evil\">"),
            "live `<div class=\"evil\">` must not appear in output; raw output:\n{out}",
        );
        // The visible body text must still survive, escaped alongside its tag.
        assert!(out.contains("block content"));
        assert!(out.contains("Trailing paragraph"));
    }

    /// Inline raw HTML (e.g. `<script>` inside a paragraph) must also be
    /// escaped by default — the Browser renderer treats inline `Html` nodes
    /// the same as block `Html` nodes under `RawHtmlPolicy::Escape`.
    #[test]
    fn render_tree_html_escapes_inline_raw_html_by_default() {
        let md: Markdown = "Inline <script>alert('xss')</script> here.\n".into();
        let result = render_tree_html(&md, &HtmlOptions::default()).expect("html render");
        let out = &result.output;
        assert!(
            out.contains("&lt;script") || out.contains("&lt;SCRIPT"),
            "inline raw HTML must be escaped to `&lt;script`; raw output:\n{out}",
        );
        assert!(
            !out.contains("<script>"),
            "live `<script>` tag must not appear in output; raw output:\n{out}",
        );
        // The surrounding prose must survive.
        assert!(out.contains("Inline"));
        assert!(out.contains("here"));
    }

    /// The mapping helper itself must default `raw_html` to
    /// [`RawHtmlPolicy::Escape`] regardless of the [`HtmlOptions`] passed in —
    /// pins the adapter conversion so a silent flip to `Allow` is caught here
    /// even when the rendered-bytes tests above don't run.
    #[test]
    fn browser_options_mapping_defaults_raw_html_to_escape() {
        let opts = browser_options_from_html_options(&HtmlOptions::default());
        assert_eq!(opts.raw_html, RawHtmlPolicy::Escape);
    }

    // -----------------------------------------------------------------------
    // Review-4 finding 1: page-level HR defaults (`hr_defaults`) and the
    // `hr_css_variables` `:root` override must be consumed by the tree entry
    // points, not silently dropped at the adapter boundary.
    // -----------------------------------------------------------------------

    /// `hr_root_variables` sorts keys for deterministic output and drops any
    /// entry whose key or value could break out of the `<style>` element.
    #[test]
    fn hr_root_variables_sorts_and_filters_unsafe_entries() {
        use std::collections::HashMap;

        let mut vars = HashMap::new();
        vars.insert("hr-width".to_string(), "42%".to_string());
        vars.insert("hr-color".to_string(), "red".to_string());
        vars.insert("hr-evil".to_string(), "</style>".to_string());

        let out = hr_root_variables(&vars).expect("safe entries remain");
        assert_eq!(
            out,
            vec![
                ("hr-color".to_string(), "red".to_string()),
                ("hr-width".to_string(), "42%".to_string()),
            ],
            "entries must be key-sorted and the `<`-bearing value dropped",
        );

        assert!(
            hr_root_variables(&HashMap::new()).is_none(),
            "an empty map emits no override list",
        );
    }

    /// `hr_css_variables` must be lowered onto the page `css_variables` channel
    /// even when `include_styles` is unset, so the `:root` HR override still
    /// reaches the browser (the legacy contract emitted it regardless of
    /// `include_styles`).
    #[test]
    fn browser_options_mapping_lowers_hr_css_variables_without_styles() {
        use std::collections::HashMap;

        let mut vars = HashMap::new();
        vars.insert("hr-width".to_string(), "42%".to_string());
        let opts = HtmlOptions {
            include_styles: false,
            hr_css_variables: vars,
            ..HtmlOptions::default()
        };
        let browser = browser_options_from_html_options(&opts);
        let page = browser
            .page
            .expect("a page must be built to carry the HR override");
        assert!(
            page.stylesheet.is_none(),
            "no code-block stylesheet is injected when include_styles is false",
        );
        assert_eq!(
            page.css_variables,
            Some(vec![("hr-width".to_string(), "42%".to_string())]),
        );
    }

    /// A bare `---` rendered through `render_tree_terminal` must adopt the
    /// `hr_defaults` kind — proving the terminal entry point consumes the
    /// page-level HR default.
    #[test]
    fn render_tree_terminal_applies_hr_defaults_to_bare_rule() {
        let md: Markdown = "---\n".into();
        let opts = TerminalOptions {
            max_width: Some(40),
            color_depth: Some(ColorDepth::None),
            image_mode: crate::markdown::output::terminal::TerminalImageMode::Never,
            hr_defaults: Some(HorizontalRuleAttrs {
                kind: Some("dots".into()),
                ..HorizontalRuleAttrs::default()
            }),
            ..TerminalOptions::default()
        };
        let out = render_tree_terminal(&md, &opts).expect("terminal render").output;
        // The default dashed rule uses `╌`/`-`; a dots default switches the
        // glyph to `·` (or the ASCII `.` fallback).
        assert!(
            out.contains('·') || out.contains('.'),
            "bare rule must adopt the `dots` HR default; got:\n{out:?}",
        );
    }

    /// A bare `---` rendered through `render_tree_html` must adopt the
    /// `hr_defaults` width / color / weight — proving the HTML entry point
    /// consumes the page-level HR default.
    #[test]
    fn render_tree_html_applies_hr_defaults_to_bare_rule() {
        let md: Markdown = "---\n".into();
        let opts = HtmlOptions {
            hr_defaults: Some(HorizontalRuleAttrs {
                kind: Some("waves".into()),
                weight: Some("thick".into()),
                color: Some("red".into()),
                width: Some("50%".into()),
                ..HorizontalRuleAttrs::default()
            }),
            ..HtmlOptions::default()
        };
        let html = render_tree_html(&md, &opts).expect("html render").output;
        assert!(html.contains(r#"width="50%""#), "{html}");
        assert!(html.contains("--hr-color: red"), "{html}");
        assert!(html.contains("--hr-weight: 8"), "thick weight ⇒ 8px: {html}");
    }

    /// `render_tree_html` (the page-declares-variables contract). The HR SVG
    /// keeps its literal `var(--hr-color, …)` / `var(--hr-weight, …)`
    /// expressions, so the declared override resolves against them in the
    /// browser.
    #[test]
    fn render_tree_html_emits_hr_css_variable_root_override() {
        let md: Markdown = "--- { style: dashes }\n".into();

        // Without overrides: the :root block carries no `--hr-width: 42%`.
        let default_html = render_tree_html(&md, &HtmlOptions::default())
            .expect("html render")
            .output;
        assert!(
            !default_html.contains("--hr-width: 42%"),
            "default render must not declare the override value: {default_html}",
        );

        // With an override: the :root block declares `--hr-width: 42%` and the
        // SVG keeps its literal `var(--hr-*, …)` expressions.
        let mut vars = std::collections::HashMap::new();
        vars.insert("hr-width".to_string(), "42%".to_string());
        let opts = HtmlOptions {
            hr_css_variables: vars,
            ..HtmlOptions::default()
        };
        let html = render_tree_html(&md, &opts).expect("html render").output;
        assert!(
            html.contains(":root{") && html.contains("--hr-width: 42%"),
            "override must emit a :root declaration --hr-width: 42%: {html}",
        );
        assert!(
            html.contains("var(--hr-color,") && html.contains("var(--hr-weight,"),
            "SVG must keep its literal var(--hr-*, …) expressions: {html}",
        );
    }

    /// Review-10 finding 2: the terminal entry point must wire darkmatter's
    /// [`TerminalCodeRenderer`] so fenced code blocks are syntax-highlighted
    /// (darkmatter's code path) rather than rendered through the render tree's
    /// plain dim-fence fallback. This pins the option-construction wiring.
    #[test]
    fn terminal_options_wire_the_darkmatter_code_renderer() {
        let opts = terminal_options_from_terminal_options(&TerminalOptions::default());
        assert!(
            opts.code_renderer.is_some(),
            "tree terminal entry point must wire the darkmatter code renderer",
        );
    }

    /// End-to-end: a fenced code block rendered through `render_tree_terminal`
    /// must carry ANSI styling produced by the darkmatter code path. The
    /// plain render-tree fallback emits only `<dim>`-wrapped text; the
    /// darkmatter highlighter emits per-token color SGRs, so the presence of
    /// a color SGR proves the code renderer is the one in use.
    #[test]
    fn render_tree_terminal_syntax_highlights_code_blocks() {
        let md: Markdown = "```rust\nfn demo() -> usize { 42 }\n```\n".into();
        let opts = TerminalOptions {
            max_width: Some(80),
            color_depth: Some(ColorDepth::TrueColor),
            ..TerminalOptions::default()
        };
        let result = render_tree_terminal(&md, &opts).expect("terminal render");
        // The code text must survive.
        assert!(
            result.output.contains("demo"),
            "code text dropped: {:?}",
            result.output
        );
        // The darkmatter highlighter emits foreground color SGRs (e.g.
        // `\x1b[38;2;…m`); the plain fallback never emits a color SGR.
        assert!(
            result.output.contains("\u{1b}[38;2;") || result.output.contains("\u{1b}[38;5;"),
            "code block must be syntax-highlighted by the wired code renderer; raw:\n{:?}",
            result.output,
        );
    }

    /// Review-11 finding 1: a fenced code block rendered through
    /// `render_tree_terminal` with `ColorDepth::None` must emit **no** ANSI
    /// color SGRs. The darkmatter `TerminalCodeRenderer` short-circuits to
    /// `None` on `ColorDepth::None`, so the tree renderer's plain (no-color)
    /// fallback runs instead of the syntax-highlighted body — matching the
    /// legacy renderer's no-formatting contract.
    #[test]
    fn render_tree_terminal_code_block_color_depth_none_emits_no_color_sgrs() {
        let md: Markdown = "```rust\nfn demo() -> usize { 42 }\n```\n".into();
        let opts = TerminalOptions {
            max_width: Some(80),
            color_depth: Some(ColorDepth::None),
            ..TerminalOptions::default()
        };
        let result = render_tree_terminal(&md, &opts).expect("terminal render");
        // The code text must survive even with color stripped.
        assert!(
            result.output.contains("demo"),
            "code text dropped: {:?}",
            result.output,
        );
        assert!(
            !contains_color_sgr(&result.output),
            "ColorDepth::None must produce no ANSI color SGRs for a code block; raw:\n{:?}",
            result.output,
        );
    }

    /// The HTML entry point must map `MermaidMode::Off` to
    /// `BrowserMermaidMode::Code` so the tree browser path honors the same
    /// default as the legacy renderer.
    #[test]
    fn browser_options_mapping_maps_mermaid_off_to_code() {
        use renderable::tree::BrowserMermaidMode;

        let opts = HtmlOptions {
            mermaid_mode: crate::markdown::output::terminal::MermaidMode::Off,
            ..Default::default()
        };
        let browser_opts = browser_options_from_html_options(&opts);
        assert_eq!(browser_opts.mermaid_mode, BrowserMermaidMode::Code);
    }

    /// The HTML entry point must map `MermaidMode::Image` to
    /// `BrowserMermaidMode::StaticSvg`: the spec's promoted browser form is a
    /// pre-rendered static `<svg>`, not the orthogonal interactive mermaid.js
    /// path (which stays a separate, default-off browser opt-in).
    #[test]
    fn browser_options_mapping_maps_mermaid_image_to_static_svg() {
        use renderable::tree::BrowserMermaidMode;

        let opts = HtmlOptions {
            mermaid_mode: crate::markdown::output::terminal::MermaidMode::Image,
            ..Default::default()
        };
        let browser_opts = browser_options_from_html_options(&opts);
        assert_eq!(browser_opts.mermaid_mode, BrowserMermaidMode::StaticSvg);
    }

    /// Review-3 finding 1: a `lang="mermaid"` fence carrying code-block DSL
    /// (`title` / `line-numbering` / `highlight`) that is NOT promoted —
    /// `MermaidMode::Off` maps to `BrowserMermaidMode::Code` — must still render
    /// the full darkmatter code-block presentation through the wired
    /// `render_browser_code` hook, not collapse to a bare `<pre><code>` that
    /// drops the metadata. This is the spec's lossless-fallback contract,
    /// exercised end-to-end with the real darkmatter code renderer.
    #[test]
    fn render_tree_html_mermaid_off_preserves_code_block_metadata() {
        let md: Markdown =
            "```mermaid title=\"Flow\" line-numbering=true highlight=2\nflowchart LR\n    A --> B\n```\n"
                .into();
        let opts = HtmlOptions {
            mermaid_mode: crate::markdown::output::terminal::MermaidMode::Off,
            ..HtmlOptions::default()
        };

        let result = render_tree_html(&md, &opts).expect("html render");
        let out = &result.output;

        assert!(
            out.contains("code-block-title") && out.contains("Flow"),
            "non-promoted mermaid must keep its title block; raw output:\n{out}",
        );
        assert!(
            out.contains("code-table") && out.contains("ln-gutter"),
            "non-promoted mermaid must keep its line-number table; raw output:\n{out}",
        );
        assert!(
            out.contains("highlighted"),
            "non-promoted mermaid must keep its highlighted-line markup; raw output:\n{out}",
        );
        assert!(
            !out.contains("<svg"),
            "MermaidMode::Off must not promote to SVG; raw output:\n{out}",
        );
    }

    /// The terminal entry point must map `TerminalImageMode::Never` to
    /// `GraphicsMode::Off` so the tree renderer suppresses both inline images
    /// and Mermaid promotion.
    #[test]
    fn terminal_options_mapping_maps_never_to_off() {
        let opts = TerminalOptions {
            image_mode: crate::markdown::output::terminal::TerminalImageMode::Never,
            ..Default::default()
        };
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert_eq!(term_opts.context.graphics_mode, GraphicsMode::Off);
        assert!(!term_opts.context.force_graphics);
    }

    /// The terminal entry point must map `TerminalImageMode::Auto` to
    /// `GraphicsMode::Rich` so the tree renderer attempts Mermaid promotion
    /// and inline image rendering when capabilities allow.
    #[test]
    fn terminal_options_mapping_maps_auto_to_rich() {
        let opts = TerminalOptions {
            image_mode: crate::markdown::output::terminal::TerminalImageMode::Auto,
            ..Default::default()
        };
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert_eq!(term_opts.context.graphics_mode, GraphicsMode::Rich);
        assert!(!term_opts.context.force_graphics);
    }

    /// The terminal entry point must map `TerminalImageMode::Force` to
    /// `GraphicsMode::Rich` with `force_graphics` set so the tree renderer
    /// bypasses capability detection.
    #[test]
    fn terminal_options_mapping_maps_force_to_rich_with_force_flag() {
        let opts = TerminalOptions {
            image_mode: crate::markdown::output::terminal::TerminalImageMode::Force,
            ..Default::default()
        };
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert_eq!(term_opts.context.graphics_mode, GraphicsMode::Rich);
        assert!(term_opts.context.force_graphics);
    }

    /// `MermaidMode::Image` must opt the terminal context into Mermaid
    /// promotion; the default (`Off`) must keep Mermaid as code so a fence is
    /// not promoted just because `GraphicsMode` is `Rich`.
    #[test]
    fn terminal_options_mapping_maps_mermaid_opt_in() {
        use renderable::tree::TerminalMermaidMode;

        let image = TerminalOptions {
            mermaid_mode: crate::markdown::output::terminal::MermaidMode::Image,
            ..Default::default()
        };
        assert_eq!(
            terminal_options_from_terminal_options(&image)
                .context
                .mermaid_mode,
            TerminalMermaidMode::Image,
        );

        // Default opts: Mermaid stays code even though graphics defaults to Rich.
        let default = terminal_options_from_terminal_options(&TerminalOptions::default());
        assert_eq!(default.context.mermaid_mode, TerminalMermaidMode::Code);
        assert_eq!(default.context.graphics_mode, GraphicsMode::Rich);
    }

    /// The terminal entry point must carry `TerminalOptions::code_theme` into
    /// the render context as its canonical kebab name so the code-renderer hook
    /// resolves the same `ThemePair` the caller pinned.
    #[test]
    fn terminal_options_mapping_threads_code_theme_name() {
        use crate::markdown::highlighting::ThemePair;

        let opts = TerminalOptions {
            code_theme: ThemePair::Dracula,
            ..Default::default()
        };
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert_eq!(
            term_opts.context.code_theme.as_deref(),
            Some("dracula"),
            "code_theme must thread into the context as its kebab name",
        );
    }

    /// Phase 2 (centralize theme resolution): the terminal entry point must
    /// keep the `Terminal`'s own `color_mode()` as the single source of
    /// truth for the page surface and the nested code-block panel. Setting
    /// `TerminalOptions::color_mode` no longer overrides the terminal — that
    /// field is now consumed only by the page path's
    /// `LayoutContext::from_page` as the fallback for `Unknown`. The code
    /// renderer's contrast resolution inverts the terminal's mode, so a dark
    /// terminal always produces a light panel and a light terminal always
    /// produces a dark panel — the same source feeds both.
    #[test]
    fn terminal_options_mapping_keeps_terminal_color_mode_as_source_of_truth() {
        use biscuit_terminal::discovery::detection::ColorMode as TermColorMode;
        use crate::markdown::highlighting::ColorMode as DmColorMode;

        // `opts.color_mode = Light` does not override the default-detected
        // terminal mode (which is `Dark` in this test environment).
        let opts = TerminalOptions {
            color_mode: DmColorMode::Light,
            ..Default::default()
        };
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert!(
            matches!(term_opts.context.terminal.color_mode, TermColorMode::Dark),
            "terminal entry point must keep the default terminal mode; got {:?}",
            term_opts.context.terminal.color_mode
        );

        // `opts.color_mode = Dark` likewise leaves the default `Dark` terminal
        // mode in place. (In a CI / non-TTY environment where detection
        // returns `Unknown`, the page path's `LayoutContext::from_page` is
        // the only place that fallback takes effect; the entry point's
        // terminal context is still the terminal's own mode.)
        let opts = TerminalOptions {
            color_mode: DmColorMode::Dark,
            ..Default::default()
        };
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert!(
            matches!(term_opts.context.terminal.color_mode, TermColorMode::Dark),
            "terminal entry point must keep the default terminal mode; got {:?}",
            term_opts.context.terminal.color_mode
        );
    }

    #[test]
    fn terminal_options_mapping_preserves_code_pipeline_inputs_for_both_modes() {
        use biscuit_terminal::discovery::detection::ColorMode as TermColorMode;
        use crate::markdown::highlighting::ColorMode as DmColorMode;

        // Phase 2: the entry point's context color mode is set from
        // `opts.color_mode` (the caller's request — the page path sets
        // it from the captured `DarkmatterPage` terminal, direct
        // `Markdown::as_terminal` callers set it from their own
        // request). The pipeline-correctness assertions (code theme
        // threaded as kebab name, truecolor preserved, code renderer
        // wired) are orthogonal to which mode is in effect; the mode
        // comparison below pins the *request* rather than the
        // terminal's own default.
        for dm_mode in [DmColorMode::Dark, DmColorMode::Light] {
            let opts = terminal_opts_for_pipeline(dm_mode);
            let mapped = terminal_options_from_terminal_options(&opts);
            let term_mode = match dm_mode {
                DmColorMode::Dark => TermColorMode::Dark,
                DmColorMode::Light => TermColorMode::Light,
                DmColorMode::Unknown => TermColorMode::Unknown,
            };

            assert_eq!(
                mapped.context.code_theme.as_deref(),
                Some("one-half"),
                "stage TerminalOptions -> TerminalRenderContext dropped the code theme for {dm_mode:?}",
            );
            assert_eq!(
                mapped.context.color_depth,
                biscuit_terminal::discovery::detection::ColorDepth::TrueColor,
                "stage TerminalOptions -> TerminalRenderContext dropped truecolor for {dm_mode:?}",
            );
            assert!(
                std::mem::discriminant(&mapped.context.color_mode)
                    == std::mem::discriminant(&term_mode),
                "stage TerminalOptions -> TerminalRenderContext mapped color mode incorrectly for {dm_mode:?}: {:?}",
                mapped.context.color_mode,
            );
            assert!(
                mapped.code_renderer.is_some(),
                "stage TerminalOptions -> TerminalRenderOptions dropped the code renderer for {dm_mode:?}",
            );
        }
    }

    #[test]
    fn render_tree_terminal_inverts_code_theme_against_terminal_source() {
        // Phase 2: the entry point's single source of truth for the page
        // surface and the nested code panel is `opts.color_mode` (which
        // the page path sets from the captured terminal's mode and
        // direct `Markdown::as_terminal` callers set from their request).
        // The `Terminal` constructed for the capability profile is
        // transport only — its own `color_mode` is not used for theme
        // resolution. The code panel inverts `opts.color_mode` under the
        // default `CodeBlockMode::Inverse`, so a dark request yields a
        // light panel and vice versa — the same source feeds the page
        // and the panel.
        use crate::markdown::highlighting::ColorMode as DmColorMode;

        let md: Markdown = "```yaml\n$schema:\n  foo: string\n```\n".into();

        let dark_output = render_tree_terminal(&md, &terminal_opts_for_pipeline(DmColorMode::Dark))
            .expect("dark terminal render")
            .output;
        assert_yaml_colors(&dark_output, DmColorMode::Light);

        let light_output =
            render_tree_terminal(&md, &terminal_opts_for_pipeline(DmColorMode::Light))
                .expect("light terminal render")
                .output;
        // A light request yields a light page, so the panel inverts to
        // dark — the cross-surface invariant.
        assert_yaml_colors(&light_output, DmColorMode::Dark);
    }

    #[test]
    fn markdown_as_terminal_inverts_code_theme_against_terminal_source() {
        use crate::markdown::highlighting::ColorMode as DmColorMode;

        let md: Markdown = "```yaml\n$schema:\n  foo: string\n```\n".into();

        let dark_output = md
            .as_terminal(terminal_opts_for_pipeline(DmColorMode::Dark))
            .expect("dark as_terminal render");
        assert_yaml_colors(&dark_output, DmColorMode::Light);

        let light_output = md
            .as_terminal(terminal_opts_for_pipeline(DmColorMode::Light))
            .expect("light as_terminal render");
        // Same rationale as the tree-entry-point test: the caller's
        // `color_mode` is the source, and the panel inverts it.
        assert_yaml_colors(&light_output, DmColorMode::Dark);
    }

    /// The terminal entry point must thread `TerminalOptions::base_path` into
    /// the render context so relative image paths resolve at `Rich`.
    #[test]
    fn terminal_options_mapping_threads_image_base_path() {
        use std::path::PathBuf;

        let opts = TerminalOptions {
            base_path: Some(PathBuf::from("/docs/assets")),
            ..Default::default()
        };
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert_eq!(
            term_opts.context.image_base_path,
            Some(PathBuf::from("/docs/assets")),
        );
    }
}
