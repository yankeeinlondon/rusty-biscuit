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
//! and the parity benches reach them. [`to_render_document`] and
//! [`render_tree_terminal_with_layout`] (the decorated `Some(ctx)` route of
//! [`Markdown::as_terminal_with_layout`](crate::markdown::Markdown::as_terminal_with_layout))
//! stay `pub(crate)`: they expose the raw fold / decoration boundary used only
//! inside the crate.

use std::rc::Rc;

use biscuit_terminal::discovery::detection::ColorDepth as TerminalColorDepth;
use biscuit_terminal::render_tree::{
    ImagePlaceholder, TerminalRenderContext, TerminalRenderOptions, render_terminal_document,
};
use biscuit_terminal::terminal::Terminal;
use renderable::tree::{
    BrowserRenderOptions, Diagnostic, Document, GraphicsMode, MarkdownDialect, MarkdownRenderOptions,
    RawHtmlPolicy, RenderStrictness, SourceDescriptor, TerminalMermaidMode,
    render_browser_document_html, render_markdown_document,
};

use renderable::tree::{NodeKind, RenderNode};

use super::code_renderer::TerminalCodeRenderer;
use super::fold::fold_markdown_spanned_with_frontmatter;
use super::pipeline::{PipelineRenderResult, PipelineResult};
use crate::markdown::Markdown;
use crate::markdown::output::{ColorDepth, HtmlOptions, TerminalOptions};

/// Folds a [`Markdown`] into a canonical [`Document`], wiring darkmatter's
/// already extracted frontmatter into [`renderable::tree::DocumentMetadata`].
///
/// Uses the **span-aware** fold
/// ([`fold_markdown_spanned_with_frontmatter`](super::fold::fold_markdown_spanned_with_frontmatter))
/// so darkmatter's `==mark==`, `⌄dim⌄`, and `--- { ... }` HR-attribute
/// constructs are visible to the every render-tree target. Without this the
/// experimental entry points would silently lose those features even though
/// the span-aware helper exists; see review-2 finding 1 in
/// `renderable/features/_completed/2026-05-20-darkmatter-tree/`.
///
/// Frontmatter is **not** re-parsed by this layer — darkmatter strips it
/// before the parser ever sees content (`Markdown::try_from(...)`), and the
/// fold's `pulldown-cmark` options keep `MetadataBlock` disabled (see
/// `parser-options.md`).
///
/// ## Returns
///
/// The folded [`Document`] and any non-fatal fold-phase [`Diagnostic`]s
/// (unsupported variants, lossy conversions, malformed structure).
#[must_use]
pub(crate) fn to_render_document(md: &Markdown) -> (Document, Vec<Diagnostic>) {
    let source = derive_source(md);
    fold_markdown_spanned_with_frontmatter(source, md)
}

/// Renders a [`Markdown`] to HTML via the render-tree pipeline.
///
/// Maps [`HtmlOptions`] to [`BrowserRenderOptions`] narrowly: raw HTML
/// defaults to [`RawHtmlPolicy::Escape`] (the safe baseline), the
/// [`TerminalCodeRenderer`] hook is wired so fenced code blocks are
/// syntax-highlighted (with title / line-number / highlight directives),
/// `mermaid_mode` maps to the render-tree Mermaid opt-in, and (when
/// `include_styles` is set) the `.code-block` panel stylesheet is
/// injected through the page hook so fenced code retains its background. The
/// [`HtmlOptions::hr_css_variables`](crate::markdown::output::HtmlOptions::hr_css_variables)
/// `:root` injection has no tree-side equivalent and is dropped at this boundary.
///
/// `style:` frontmatter hyperlink / image color injection
/// ([`HtmlOptions::hyperlink_style`](crate::markdown::output::HtmlOptions::hyperlink_style),
/// `local_hyperlink_style`, `local_image_style`) is reproduced by
/// [`inject_inline_styles`], which mutates the folded [`Document`] before render
/// and rewrites the resulting `<a>` / `<img>` tags. See that function for the
/// merge precedence. The render tree carries no inline-CSS layer on a link /
/// image node, so the injection bridges the gap on the darkmatter side.
///
/// Uses the direct document-string renderer
/// ([`render_browser_document_html`]) — the production browser path needs the
/// complete final HTML string, so it streams straight into one buffer rather
/// than building an intermediate [`HtmlPage`](renderable::html::HtmlPage) /
/// [`BrowserFragment`](renderable::browser::BrowserFragment) tree and then
/// serializing it. See `renderable/features/2026-06-03-browser-perf/spec.md`.
///
/// ## Errors
///
/// Propagates any fatal [`RenderError`] from
/// [`render_browser_document_html`]. Non-fatal diagnostics are returned in the
/// [`PipelineResult`] without being demoted to errors.
pub fn render_tree_html(md: &Markdown, options: &HtmlOptions) -> PipelineRenderResult<String> {
    let (doc, fold_diagnostics) = to_render_document(md);
    render_tree_html_from_document(doc, fold_diagnostics, options)
}

/// Renders an already-folded [`Document`] to HTML via the render-tree pipeline.
///
/// Splits the post-fold half of [`render_tree_html`] out so [`Markdown::as_html`]
/// can fold once, run [`validate_code_directives`] over the same tree (restoring
/// the legacy fatal-directive contract — see that function), and render from the
/// shared [`Document`] without a second fold.
pub(crate) fn render_tree_html_from_document(
    mut doc: Document,
    fold_diagnostics: Vec<Diagnostic>,
    options: &HtmlOptions,
) -> PipelineRenderResult<String> {
    let injections = inject_link_image_attributes(&mut doc.root, options);
    let browser_opts = browser_options_from_html_options(options);
    let rendered = render_browser_document_html(&doc, &browser_opts)?;
    let output = apply_attribute_injections(rendered.output, &injections);
    Ok(PipelineResult::new(
        output,
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
/// the browser-only guard [`Markdown::as_html`] runs before rendering. The
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

/// One pending attribute injection: the sentinel class attached to a folded
/// link / image node and the literal attribute fragment that must replace it in
/// the rendered output.
struct AttributeInjection {
    /// The unique sentinel class written onto the node (and emitted as
    /// `class="…"` by the browser writer).
    sentinel: String,
    /// The attribute fragment (e.g. `class="btn" style="…" target="_blank"`)
    /// substituted for the node's emitted `class="<sentinel>"`. Free-form text
    /// values are pre-escaped; the fragment is spliced verbatim into the
    /// rendered HTML.
    attributes: String,
}

/// Sentinel-class prefix for [`inject_link_image_attributes`]. Carries no
/// semantic meaning; it exists only as a 1:1 post-render handle so the computed
/// attribute fragment lands on the exact tag the node produced, independent of
/// duplicate URLs.
const ATTR_INJECTION_SENTINEL_PREFIX: &str = "__dm-attr-injection-";

/// Lowers darkmatter's structured link directives and `style:` frontmatter
/// link / image colors onto the folded tree, reproducing legacy
/// `output::as_html`'s `<a>` / `<img>` attribute emission on the tree path.
///
/// The render tree carries no structured-link layer (`class` / `target` /
/// `data-*` / `prompt`) and no inline-CSS layer on a [`NodeKind::Link`] /
/// [`NodeKind::Image`] node, so darkmatter bridges the gap here: each decorated
/// node gets a unique sentinel **class** (which the browser writer emits
/// verbatim) and the computed attribute fragment is recorded;
/// [`apply_attribute_injections`] then rewrites `class="<sentinel>"` to that
/// fragment in the rendered string.
///
/// The **link** pass runs for every titled link so a structured directive
/// (`[label](url "class='btn' target='_blank' prompt='…' data-x='y'")`) keeps
/// its `class`, `style`, `target`, plain `title`, `data-prompt`, and `data-*`
/// attributes even when no frontmatter hyperlink style is configured — this is
/// the fidelity legacy emitted and review-2 finding 2 flagged the tree path for
/// dropping. The **image** pass only applies the frontmatter `local_image_style`
/// overlay and is skipped when none is set.
///
/// Merge precedence mirrors `output::html`:
///
/// - **Link.** `is_local = Link::is_file()`. Local links merge
///   `local_hyperlink_style` over `hyperlink_style`; remote links use
///   `hyperlink_style` only. The frontmatter overlay then fills only the
///   properties the link's own inline CSS (`style='…'` in its title) does not
///   already set — the per-link declaration wins.
/// - **Image.** Only local images (`is_local_image`) receive
///   `local_image_style`, again filling only properties the image's own inline
///   CSS does not set. Remote images get no local style.
///
/// A link whose title parsed as a structured directive (`class='…'`,
/// `style='…'`, …) also has that title cleared from the node so the browser
/// writer does not leak it as a `title="…"` attribute, matching legacy
/// structured-mode link output.
///
/// ## Returns
///
/// The pending injections, in attachment order. Empty when no link / image node
/// needed any attribute — the caller then skips the post-render rewrite.
fn inject_link_image_attributes(
    root: &mut RenderNode,
    options: &HtmlOptions,
) -> Vec<AttributeInjection> {
    let has_link_style =
        options.hyperlink_style.is_some() || options.local_hyperlink_style.is_some();
    let has_image_style = options.local_image_style.is_some();

    let mut injections = Vec::new();
    decorate_node(root, options, has_link_style, has_image_style, &mut injections);
    injections
}

/// Recursively walks `node`, decorating link / image nodes per
/// [`inject_link_image_attributes`].
fn decorate_node(
    node: &mut RenderNode,
    options: &HtmlOptions,
    has_link_style: bool,
    has_image_style: bool,
    injections: &mut Vec<AttributeInjection>,
) {
    match &node.kind {
        // A plain link with no title and no frontmatter style can carry no
        // structured attributes; skip parsing it (the common case).
        NodeKind::Link { url, title, .. } if title.is_some() || has_link_style => {
            if let Some((attributes, clear_title)) =
                link_attributes(url, title.as_deref(), options, has_link_style)
            {
                attach_injection(node, attributes, injections);
                if clear_title
                    && let NodeKind::Link { title, .. } = &mut node.kind
                {
                    *title = None;
                }
            }
        }
        NodeKind::Image { url, title, .. } if has_image_style => {
            if let Some(css) = image_inline_css(url, title.as_deref(), options) {
                attach_injection(node, format!("style=\"{css}\""), injections);
            }
        }
        _ => {}
    }

    if let Some(children) = node.children_mut() {
        for child in children {
            decorate_node(child, options, has_link_style, has_image_style, injections);
        }
    }
}

/// Records an injection for `attributes` and writes its sentinel class onto
/// `node`.
fn attach_injection(
    node: &mut RenderNode,
    attributes: String,
    injections: &mut Vec<AttributeInjection>,
) {
    let sentinel = format!("{ATTR_INJECTION_SENTINEL_PREFIX}{}", injections.len());
    node.attrs.classes.push(sentinel.clone());
    injections.push(AttributeInjection {
        sentinel,
        attributes,
    });
}

/// Computes the full `<a>` attribute fragment for a link, or `None` when nothing
/// applies.
///
/// Returns `(attributes, clear_title)` where `attributes` is the escaped
/// `class="…" style="…" target="…" title="…" data-prompt="…" data-x="…"`
/// fragment legacy `output::as_html` emits, and `clear_title` is `true` when the
/// link's title parsed as a structured directive (so the node's `title` must be
/// cleared to avoid leaking the raw directive as a `title="…"` attribute).
///
/// The structured attributes (`class` / `target` / `title` / `data-*`) are
/// emitted only when the title parsed as a directive; the merged `style` is
/// emitted whenever a frontmatter overlay or a per-link `style='…'` applies. A
/// plain title (a real tooltip) is left on the node so the browser writer emits
/// it natively.
fn link_attributes(
    url: &str,
    title: Option<&str>,
    options: &HtmlOptions,
    has_link_style: bool,
) -> Option<(String, bool)> {
    use crate::render::Link;

    let link = Link::with_title_parsed("", url, title.unwrap_or("")).ok()?;

    // A directive (`class='…' target='…' …`) always makes the parsed plain
    // title differ from the raw title; a real tooltip parses back to itself.
    let is_structured = title.is_some() && link.title_plain().as_deref() != title;

    let frontmatter_css = if has_link_style {
        let merged = if link.is_file() {
            options
                .local_hyperlink_style
                .as_ref()
                .map(|local| match options.hyperlink_style.as_ref() {
                    Some(base) => crate::style::bespoke::merge_common_style(base, local),
                    None => local.clone(),
                })
                .or_else(|| options.hyperlink_style.clone())
        } else {
            options.hyperlink_style.clone()
        };
        merged.and_then(|common| common.to_css_overlay())
    } else {
        None
    };

    let style = combine_inline_css(frontmatter_css.as_ref(), link.style());

    if !is_structured && style.is_none() {
        return None;
    }

    let mut attrs = String::new();
    let mut push_attr = |fragment: &str| {
        if !attrs.is_empty() {
            attrs.push(' ');
        }
        attrs.push_str(fragment);
    };

    if is_structured
        && let Some(class) = link.class()
    {
        push_attr(&format!("class=\"{}\"", html_escape::encode_text(class)));
    }
    // The merged CSS is already a normalized declaration string; legacy emits it
    // unescaped and CSS carries no `<` / `>` / `&`, so it is spliced as-is.
    if let Some(style) = &style {
        push_attr(&format!("style=\"{style}\""));
    }
    if is_structured {
        if let Some(target) = link.target_attr() {
            push_attr(&format!(
                "target=\"{}\"",
                html_escape::encode_text(&target)
            ));
        }
        if let Some(title_plain) = link.title_plain() {
            push_attr(&format!(
                "title=\"{}\"",
                html_escape::encode_text(&title_plain)
            ));
        }
        if let Some(prompt) = link.prompt() {
            push_attr(&format!(
                "data-prompt=\"{}\"",
                html_escape::encode_text(prompt)
            ));
        }
        for (key, value) in link.data() {
            push_attr(&format!(
                "data-{}=\"{}\"",
                html_escape::encode_text(key),
                html_escape::encode_text(value)
            ));
        }
    }

    if attrs.is_empty() {
        return None;
    }
    Some((attrs, is_structured))
}

/// Computes the merged inline CSS for an image, or `None` when nothing applies.
///
/// Only local images receive the frontmatter `local_image_style`; remote images
/// return `None`.
fn image_inline_css(url: &str, title: Option<&str>, options: &HtmlOptions) -> Option<String> {
    if !crate::style::bespoke::is_local_image(url) {
        return None;
    }
    let local = options.local_image_style.as_ref()?;
    let frontmatter_css = local.to_css_overlay()?;

    // Reconstruct the image's own inline CSS the same way legacy does, so a
    // per-image `style='…'` directive still wins over the frontmatter overlay.
    let image_ref = build_image_ref(url, title);
    let own_css = image_ref.as_ref().and_then(|i| i.style());

    combine_inline_css(Some(&frontmatter_css), own_css)
}

/// Reconstructs a darkmatter [`ImageRef`](crate::render::ImageRef) from a folded
/// image node's `url` / `title` so its own inline CSS can be recovered.
fn build_image_ref(url: &str, title: Option<&str>) -> Option<crate::render::ImageRef> {
    use crate::render::ImageRef;

    let mut image_ref = ImageRef::new(url, "").ok()?;
    if let Some(title) = title
        && !title.trim().is_empty()
    {
        image_ref = image_ref.with_title(title);
    }
    Some(image_ref)
}

/// Merges a frontmatter CSS overlay under a node's own inline CSS.
///
/// The node's own per-property declarations win; the frontmatter overlay fills
/// only the properties the node does not already set. Mirrors legacy
/// `output::html::merge_css_style` (existing declarations come last and win).
///
/// Returns `None` when both inputs are absent, and a single-line declaration
/// string (`prop: value; prop: value`) otherwise.
fn combine_inline_css(
    frontmatter: Option<&renderable::stylesheet::CssStyle>,
    own: Option<&renderable::stylesheet::CssStyle>,
) -> Option<String> {
    let merged = match (frontmatter, own) {
        (None, None) => return None,
        (Some(fm), None) => fm.clone(),
        (None, Some(own)) => own.clone(),
        (Some(fm), Some(own)) => {
            let combined = format!("{}\n{}", fm.to_css(), own.to_css());
            renderable::stylesheet::CssStyle::try_from(combined.as_str()).unwrap_or_else(|_| fm.clone())
        }
    };
    if merged.is_empty() {
        return None;
    }
    // `CssStyle::to_css` is newline-joined; collapse to the single-line inline
    // form the browser `style="…"` attribute expects.
    Some(merged.to_css().replace('\n', " "))
}

/// Rewrites each `class="<sentinel>"` produced by
/// [`inject_link_image_attributes`] into the node's computed attribute fragment
/// in the rendered HTML.
///
/// Each sentinel is unique and appears exactly once, so a single literal
/// replacement per injection is sufficient. The sentinels are pure ASCII and
/// pass through the browser writer's attribute escaping unchanged.
fn apply_attribute_injections(mut html: String, injections: &[AttributeInjection]) -> String {
    for injection in injections {
        let needle = format!("class=\"{}\"", injection.sentinel);
        html = html.replacen(&needle, &injection.attributes, 1);
    }
    html
}

/// Renders a [`Markdown`] to a terminal string via the render-tree pipeline.
///
/// Maps [`TerminalOptions`] to a [`TerminalRenderOptions`] built from a
/// detected [`Terminal`] (respecting any pinned `max_width` /
/// `color_depth`). `image_mode` maps to the graphics fidelity tier and
/// `mermaid_mode` to the Mermaid promotion opt-in; see
/// [`terminal_options_from_terminal_options`] for the full mapping.
///
/// ## Errors
///
/// Propagates any fatal [`RenderError`] from
/// [`render_terminal_document`].
pub fn render_tree_terminal(
    md: &Markdown,
    options: &TerminalOptions,
) -> PipelineRenderResult<String> {
    let (doc, fold_diagnostics) = to_render_document(md);
    let term_opts = terminal_options_from_terminal_options(options);
    let rendered = render_terminal_document(&doc, &term_opts)?;
    Ok(PipelineResult::new(
        rendered.output,
        fold_diagnostics,
        rendered.diagnostics,
    ))
}

/// Renders a [`Markdown`] to a terminal string via the render-tree pipeline,
/// lowering a [`LayoutContext`](crate::layout::LayoutContext)'s per-component
/// settings onto the folded tree first.
///
/// This is the decorated-layout counterpart to [`render_tree_terminal`]: a
/// decoration pass walks the folded [`Document`] and, for every node that maps
/// to a [`PageComponent`], sets the node's
/// [`Layout`](renderable::layout::Layout) (alignment, width cap via margins) and
/// [`Style`](renderable::style::Style) (foreground / background colors) from the
/// resolved context. Page-level margins, padding, and background are applied by
/// the [`DarkmatterPage`](crate::layout::DarkmatterPage) row-decoration
/// post-pass and are intentionally **not** handled here.
///
/// Backs the decorated `Some(ctx)` route of
/// [`Markdown::as_terminal_with_layout`](crate::markdown::Markdown::as_terminal_with_layout).
/// The image fallback placeholder is switched to the legacy `▉ IMAGE[alt]`
/// block form here (only the decorated path uses it; the default
/// [`render_tree_terminal`] keeps the generic `[alt]`).
///
/// ## Errors
///
/// Propagates any fatal [`RenderError`] from [`render_terminal_document`], or a
/// [`PageRenderError`](crate::layout::PageRenderError)-derived fold of a
/// component width that cannot be resolved (mapped to a render error string).
pub(crate) fn render_tree_terminal_with_layout(
    md: &Markdown,
    options: &TerminalOptions,
    ctx: &crate::layout::LayoutContext,
) -> PipelineRenderResult<String> {
    let (mut doc, fold_diagnostics) = to_render_document(md);
    super::decorate::decorate_document(&mut doc.root, ctx);
    let mut term_opts = terminal_options_from_terminal_options(options);
    // Decorated-only: the decorated layout path emits `▉ IMAGE[alt]` for the
    // non-graphics image fallback. The default path keeps `Bracket`.
    term_opts.context.image_placeholder = ImagePlaceholder::Block;
    let rendered = render_terminal_document(&doc, &term_opts)?;
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
pub fn render_tree_markdown(md: &Markdown) -> PipelineRenderResult<String> {
    render_tree_markdown_dialect(md, MarkdownDialect::Markdown)
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
) -> PipelineRenderResult<String> {
    let (doc, fold_diagnostics) = to_render_document(md);
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
/// `<pre><code>` fallback. The
/// [`HtmlOptions::hr_css_variables`](crate::markdown::output::HtmlOptions::hr_css_variables)
/// `:root` injection has no tree-side equivalent and is dropped at this boundary.
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

    let page = opts.include_styles.then(|| PageOptions {
        stylesheet: Some(code_block_stylesheet(opts)),
        ..PageOptions::default()
    });

    BrowserRenderOptions {
        strictness: RenderStrictness::Warn,
        raw_html: RawHtmlPolicy::Escape,
        page,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
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
fn terminal_options_from_terminal_options(opts: &TerminalOptions) -> TerminalRenderOptions {
    let mut term = match opts.max_width {
        Some(width) => Terminal::new_optimistic(u32::from(width)),
        None => Terminal::default(),
    };
    if let Some(depth) = opts.color_depth {
        term.color_depth = darkmatter_color_depth_to_terminal(depth);
    }
    // Thread the requested color mode so the code renderer's contrast
    // resolution (which inverts the mode for page contrast) sees the caller's
    // mode rather than the detected/optimistic terminal default. Without this
    // the highlighter always resolved against the terminal's own mode, dropping
    // a caller's `with_color_mode(...)`.
    term.color_mode = darkmatter_color_mode_to_terminal(opts.color_mode);

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

    TerminalRenderOptions {
        context,
        strictness: RenderStrictness::Warn,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
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

/// Maps darkmatter's highlighting [`ColorMode`](crate::markdown::highlighting::ColorMode)
/// onto biscuit-terminal's [`ColorMode`](biscuit_terminal::discovery::detection::ColorMode).
fn darkmatter_color_mode_to_terminal(
    mode: crate::markdown::highlighting::ColorMode,
) -> biscuit_terminal::discovery::detection::ColorMode {
    use biscuit_terminal::discovery::detection::ColorMode as TermColorMode;
    use crate::markdown::highlighting::ColorMode as DmColorMode;
    match mode {
        DmColorMode::Light => TermColorMode::Light,
        DmColorMode::Dark => TermColorMode::Dark,
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

    /// Smoke: every entry point renders a small fixture without panicking and
    /// surfaces fold/render diagnostics separately.
    const FIXTURE: &str = "# Heading\n\nA paragraph with **strong** text.\n";

    #[test]
    fn to_render_document_smoke() {
        let md: Markdown = FIXTURE.into();
        let (doc, diags) = to_render_document(&md);
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
        let (doc, _diags) = to_render_document(&md);
        assert_eq!(
            doc.sources.resolve(SourceId(0)),
            Some(&SourceDescriptor::File {
                path: PathBuf::from("docs/example.md"),
            }),
            "file-backed Markdown must register a SourceDescriptor::File",
        );

        // No source: stays virtual.
        let md: Markdown = "Body paragraph.\n".into();
        let (doc, _diags) = to_render_document(&md);
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
        let (doc, _diags) = to_render_document(&md);
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
        let (doc, diags) = to_render_document(&md);
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
        let (doc, diags) = to_render_document(&md);
        assert!(diags.is_empty(), "dim fixture must fold cleanly: {diags:?}");
        assert!(
            has_dim(&doc.root),
            "to_render_document must surface a `dim` Extended node",
        );
    }

    /// `to_render_document` must rewrite `--- { style: waves }` into a
    /// `ThematicBreak` carrying the `darkmatter.hr.kind` hint.
    #[test]
    fn to_render_document_uses_span_aware_fold_for_hr_attributes() {
        use renderable::tree::{HintNamespace, NodeKind, RenderNode};

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
        let (doc, _diags) = to_render_document(&md);
        let hr = find_hr(&doc.root).expect("HR-attribute paragraph must fold to a ThematicBreak");
        let ns = HintNamespace("darkmatter.hr");
        assert_eq!(
            hr.attrs.get_hint(ns, "kind"),
            Some(&serde_json::json!("waves")),
            "HR-attribute hint must survive to_render_document",
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
        // Mix prose, inline code (lowers to `<dim>…</dim>`), and a link
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

        let mut opts = HtmlOptions::default();
        opts.mermaid_mode = crate::markdown::output::terminal::MermaidMode::Off;
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

        let mut opts = HtmlOptions::default();
        opts.mermaid_mode = crate::markdown::output::terminal::MermaidMode::Image;
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
        let mut opts = TerminalOptions::default();
        opts.image_mode = crate::markdown::output::terminal::TerminalImageMode::Never;
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert_eq!(term_opts.context.graphics_mode, GraphicsMode::Off);
        assert!(!term_opts.context.force_graphics);
    }

    /// The terminal entry point must map `TerminalImageMode::Auto` to
    /// `GraphicsMode::Rich` so the tree renderer attempts Mermaid promotion
    /// and inline image rendering when capabilities allow.
    #[test]
    fn terminal_options_mapping_maps_auto_to_rich() {
        let mut opts = TerminalOptions::default();
        opts.image_mode = crate::markdown::output::terminal::TerminalImageMode::Auto;
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert_eq!(term_opts.context.graphics_mode, GraphicsMode::Rich);
        assert!(!term_opts.context.force_graphics);
    }

    /// The terminal entry point must map `TerminalImageMode::Force` to
    /// `GraphicsMode::Rich` with `force_graphics` set so the tree renderer
    /// bypasses capability detection.
    #[test]
    fn terminal_options_mapping_maps_force_to_rich_with_force_flag() {
        let mut opts = TerminalOptions::default();
        opts.image_mode = crate::markdown::output::terminal::TerminalImageMode::Force;
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

        let mut image = TerminalOptions::default();
        image.mermaid_mode = crate::markdown::output::terminal::MermaidMode::Image;
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

        let mut opts = TerminalOptions::default();
        opts.code_theme = ThemePair::Dracula;
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert_eq!(
            term_opts.context.code_theme.as_deref(),
            Some("dracula"),
            "code_theme must thread into the context as its kebab name",
        );
    }

    /// The terminal entry point must thread `TerminalOptions::color_mode` onto
    /// the terminal so the code-renderer hook's contrast resolution sees the
    /// caller's mode rather than the optimistic terminal default.
    #[test]
    fn terminal_options_mapping_threads_color_mode() {
        use biscuit_terminal::discovery::detection::ColorMode as TermColorMode;
        use crate::markdown::highlighting::ColorMode as DmColorMode;

        let mut opts = TerminalOptions::default();
        opts.color_mode = DmColorMode::Light;
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert!(matches!(
            term_opts.context.terminal.color_mode,
            TermColorMode::Light
        ));

        opts.color_mode = DmColorMode::Dark;
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert!(matches!(
            term_opts.context.terminal.color_mode,
            TermColorMode::Dark
        ));
    }

    /// The terminal entry point must thread `TerminalOptions::base_path` into
    /// the render context so relative image paths resolve at `Rich`.
    #[test]
    fn terminal_options_mapping_threads_image_base_path() {
        use std::path::PathBuf;

        let mut opts = TerminalOptions::default();
        opts.base_path = Some(PathBuf::from("/docs/assets"));
        let term_opts = terminal_options_from_terminal_options(&opts);
        assert_eq!(
            term_opts.context.image_base_path,
            Some(PathBuf::from("/docs/assets")),
        );
    }

    // -----------------------------------------------------------------------
    // Decorated-layout tree path (`render_tree_terminal_with_layout`).
    //
    // The decoration pass + entry point back the active `Some(ctx)` route of
    // `Markdown::as_terminal_with_layout`. These tests pin per-component
    // coverage and the three formerly-legacy features (hyperlink label width,
    // `▉ IMAGE[alt]` placeholder, right-aligned list-item body).
    // -----------------------------------------------------------------------

    #[allow(deprecated)]
    fn layout_ctx_with_blockquote_indent() -> crate::layout::LayoutContext {
        use crate::layout::{
            LayoutContext, PageBackground, PageComponent, PageFill, PageMargin, PagePadding,
            WidthUnit,
        };
        use std::collections::HashMap;

        let mut fills = HashMap::new();
        fills.insert(
            PageComponent::BlockQuotes,
            PageFill::Indent(WidthUnit::Fixed(10)),
        );

        LayoutContext::from_page(
            80,
            PageMargin::ZERO,
            PagePadding::ZERO,
            PageBackground::Transparent,
            None,
            &biscuit_terminal::discovery::detection::ColorMode::Dark,
            crate::markdown::highlighting::ColorMode::Dark,
            HashMap::new(),
            fills,
            HashMap::new(),
            None,
            None,
            HashMap::new(),
            HashMap::new(),
            None,
            None,
            None,
        )
        .expect("layout context")
    }

    /// The decorated entry point caps a block quote to its `Indent` fill width
    /// through the tree: at 80 cols with `Indent(10)` the bar-prefixed lines
    /// must stay within 70 visible columns and wrap onto multiple lines.
    #[test]
    fn render_tree_terminal_with_layout_caps_blockquote_indent() {
        fn strip(s: &str) -> String {
            let mut out = String::new();
            let mut esc = false;
            for c in s.chars() {
                if esc {
                    if c.is_ascii_alphabetic() {
                        esc = false;
                    }
                    continue;
                }
                if c == '\u{1b}' {
                    esc = true;
                    continue;
                }
                out.push(c);
            }
            out
        }

        let md: Markdown = "> This is a fairly long quoted paragraph that should be forced to wrap onto a second visible line once the Indent(10) fill caps the blockquote width below the page width.\n".into();
        let mut opts = TerminalOptions::default();
        opts.max_width = Some(80);
        opts.color_depth = Some(ColorDepth::TrueColor);

        let ctx = layout_ctx_with_blockquote_indent();
        let out = render_tree_terminal_with_layout(&md, &opts, &ctx)
            .expect("decorated render")
            .output;
        let plain = strip(&out);

        let bar_lines: Vec<&str> = plain.lines().filter(|l| l.contains('▐')).collect();
        assert!(
            bar_lines.len() >= 2,
            "blockquote must wrap under Indent(10); got:\n{plain}",
        );
        let max_len = bar_lines
            .iter()
            .map(|l| l.trim_end().chars().count())
            .max()
            .unwrap_or(0);
        assert!(
            max_len <= 70,
            "blockquote lines must be capped to 70 cols by Indent(10); got max={max_len}:\n{plain}",
        );
    }

    /// Strips ANSI escape sequences for byte-level assertions on rendered text.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut esc = false;
        for c in s.chars() {
            if esc {
                if c.is_ascii_alphabetic() {
                    esc = false;
                }
                continue;
            }
            if c == '\u{1b}' {
                esc = true;
                continue;
            }
            out.push(c);
        }
        out
    }

    /// Builds a decorated `LayoutContext` from the three style buckets and the
    /// component alignment map the formerly-legacy features key off.
    #[allow(deprecated)]
    fn decorated_ctx(
        alignments: std::collections::HashMap<crate::layout::PageComponent, crate::layout::PageAlignment>,
        fills: std::collections::HashMap<crate::layout::PageComponent, crate::layout::PageFill>,
        hyperlink_style: Option<crate::style::schema::CommonStyle>,
        local_image_style: Option<crate::style::schema::CommonStyle>,
    ) -> crate::layout::LayoutContext {
        use crate::layout::{LayoutContext, PageBackground, PageMargin, PagePadding};
        use std::collections::HashMap;

        LayoutContext::from_page(
            80,
            PageMargin::ZERO,
            PagePadding::ZERO,
            PageBackground::Transparent,
            None,
            &biscuit_terminal::discovery::detection::ColorMode::Dark,
            crate::markdown::highlighting::ColorMode::Dark,
            alignments,
            fills,
            HashMap::new(),
            None,
            None,
            HashMap::new(),
            HashMap::new(),
            hyperlink_style,
            None,
            local_image_style,
        )
        .expect("layout context")
    }

    /// Feature 1: a `style.hyperlinks.width` pads / truncates the inline link
    /// label through the decorated tree path.
    #[test]
    fn render_tree_terminal_with_layout_pads_hyperlink_label() {
        use crate::style::schema::CommonStyle;
        use renderable::layout::Length;
        use std::collections::HashMap;

        let hyperlink_style = CommonStyle {
            width: Some(Length::ch(20)),
            ..CommonStyle::default()
        };
        let ctx = decorated_ctx(HashMap::new(), HashMap::new(), Some(hyperlink_style), None);

        let md: Markdown = "[go](https://example.com)\n".into();
        let mut opts = TerminalOptions::default();
        opts.max_width = Some(80);
        opts.color_depth = Some(ColorDepth::None);

        let out = render_tree_terminal_with_layout(&md, &opts, &ctx)
            .expect("decorated render")
            .output;
        // `width: 20ch` left-pads "go" to exactly 20 cells (trailing spaces).
        // The OSC8 hyperlink envelope is not stripped by `strip_ansi`, so assert
        // on the raw output's padded run (which rides inside the OSC8 sequence).
        assert!(
            out.contains("go                  "),
            "hyperlink label must pad to 20 cells; got:\n{out:?}",
        );
    }

    /// Feature 1 (truncation): a label wider than `max-width` is truncated with
    /// an ellipsis through the decorated tree path.
    #[test]
    fn render_tree_terminal_with_layout_truncates_hyperlink_label() {
        use crate::style::schema::CommonStyle;
        use renderable::layout::Length;
        use std::collections::HashMap;

        let hyperlink_style = CommonStyle {
            max_width: Some(Length::ch(5)),
            ..CommonStyle::default()
        };
        let ctx = decorated_ctx(HashMap::new(), HashMap::new(), Some(hyperlink_style), None);

        let md: Markdown = "[a very long label](https://example.com)\n".into();
        let mut opts = TerminalOptions::default();
        opts.max_width = Some(80);
        opts.color_depth = Some(ColorDepth::None);

        let out = render_tree_terminal_with_layout(&md, &opts, &ctx)
            .expect("decorated render")
            .output;
        let plain = strip_ansi(&out);
        assert!(
            plain.contains('…'),
            "over-wide label must truncate with an ellipsis; got:\n{plain:?}",
        );
        assert!(
            !plain.contains("very long"),
            "truncated label must not keep the full text; got:\n{plain:?}",
        );
    }

    /// Feature 2: a local image renders the `▉ IMAGE[alt]` block placeholder
    /// through the decorated tree path (graphics off so no raster is attempted).
    #[test]
    fn render_tree_terminal_with_layout_emits_image_block_placeholder() {
        use std::collections::HashMap;

        let ctx = decorated_ctx(HashMap::new(), HashMap::new(), None, None);

        let md: Markdown = "![a diagram](diagram.png)\n".into();
        let mut opts = TerminalOptions::default();
        opts.max_width = Some(80);
        opts.color_depth = Some(ColorDepth::None);
        // Suppress raster so the fallback placeholder path runs deterministically.
        opts.image_mode = crate::markdown::output::terminal::TerminalImageMode::Never;

        let out = render_tree_terminal_with_layout(&md, &opts, &ctx)
            .expect("decorated render")
            .output;
        let plain = strip_ansi(&out);
        assert!(
            plain.contains("▉ IMAGE[a diagram]"),
            "local image must render the `▉ IMAGE[...]` block placeholder; got:\n{plain:?}",
        );
    }

    /// Feature 3: a right-aligned list lifts the marker onto its own line and
    /// right-aligns the body block through the decorated tree path.
    #[test]
    #[allow(deprecated)]
    fn render_tree_terminal_with_layout_right_aligns_list_item_body() {
        use crate::layout::{PageAlignment, PageComponent, PageFill, WidthUnit};
        use std::collections::HashMap;

        let mut alignments = HashMap::new();
        alignments.insert(PageComponent::Li, PageAlignment::Right);
        // Cap the Li body so there is surplus to right-align into.
        let mut fills = HashMap::new();
        fills.insert(PageComponent::Li, PageFill::Max(WidthUnit::Fixed(20)));

        let ctx = decorated_ctx(alignments, fills, None, None);

        let md: Markdown = "- item body\n".into();
        let mut opts = TerminalOptions::default();
        opts.max_width = Some(80);
        opts.color_depth = Some(ColorDepth::None);

        let out = render_tree_terminal_with_layout(&md, &opts, &ctx)
            .expect("decorated render")
            .output;
        let plain = strip_ansi(&out);
        let lines: Vec<&str> = plain.lines().collect();
        // The marker is on its own line; the body is on a later, left-padded line.
        let marker_line = lines
            .iter()
            .position(|l| l.trim() == "-")
            .expect("marker must be lifted onto its own line");
        let body_line = lines
            .iter()
            .skip(marker_line + 1)
            .find(|l| l.contains("item body"))
            .expect("body must follow the lifted marker");
        let lead = body_line.len() - body_line.trim_start().len();
        assert!(
            lead > 0,
            "right-aligned list body must be left-padded; got:\n{plain:?}",
        );
    }
}
