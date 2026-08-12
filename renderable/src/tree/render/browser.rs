//! Browser (HTML) renderer for the canonical render tree.
//!
//! [`render_browser_node`] folds a [`RenderNode`] into a typed
//! [`BrowserFragment<Ready>`]; [`render_browser_document`] folds a whole
//! [`Document`] into an [`HtmlPage`].
//!
//! The renderer emits *safe, typed* HTML through the [`BrowserFragment`]
//! builders. Text content is escaped on emit by the fragment renderer, so the
//! only path that produces unescaped markup is [`NodeKind::Html`], governed by
//! [`RawHtmlPolicy`].
//!
//! ## Examples
//!
//! ```
//! use renderable::tree::{HeadingDepth, RenderNode};
//! use renderable::tree::render::{render_browser_node, BrowserRenderOptions};
//!
//! let tree = RenderNode::root(vec![RenderNode::heading(
//!     HeadingDepth::new(2).unwrap(),
//!     vec![RenderNode::text("Title")],
//! )]);
//! let rendered = render_browser_node(&tree, &BrowserRenderOptions::default()).unwrap();
//! assert_eq!(rendered.output.render(), "<div><h2>Title</h2></div>");
//! ```

use std::rc::Rc;

use crate::browser::PageOptions;
use crate::browser::feature::{
    DefaultFeatureResolver, FeatureContext, FeatureResolver, PageFeature, dedup_features,
    resolve_features, serialize_features_head,
};
use crate::browser::fragment::{
    BrowserFragment, ComposableNode, PopoverIdAllocator, PopoverNode, Ready, write_attributes,
};
use crate::html::HtmlPage;
use crate::html::attribute::{ClassDefinition, DomId, HtmlDataAttribute};
use crate::html::tag::{BlockTag, HtmlAttribute, HtmlType, VoidTag};
use crate::tree::attrs::NodeAttrs;
use crate::tree::{HrAlignment, HrKind, HrWeight};
use crate::tree::diagnostic::{Diagnostic, Severity};
use crate::tree::document::Document;
use crate::tree::error::{RenderError, RenderStrictness, Rendered};
use crate::tree::node::{ColumnAlign, HeadingDepth, NodeKind, RenderNode};
use crate::tree::render::CodeRenderer;
use crate::tree::validate::{ValidationError, ValidationMode, validate};

/// How a [`NodeKind::Html`] node is treated by the browser renderer.
///
/// The [`Default`] is [`RawHtmlPolicy::Escape`]: the safe choice. Raw HTML
/// from an untrusted source is a script-injection vector, so the renderer
/// never passes it through verbatim unless the caller explicitly opts in with
/// [`RawHtmlPolicy::Allow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RawHtmlPolicy {
    /// Emit the raw HTML verbatim. The caller vouches for its safety.
    Allow,
    /// Emit the HTML source as escaped text and record a lossy diagnostic.
    /// This is the safe default.
    #[default]
    Escape,
    /// Reject raw HTML. Under [`RenderStrictness::Strict`] this is a
    /// [`RenderError::LossyRejected`]; under [`RenderStrictness::Warn`] /
    /// [`RenderStrictness::Lossy`] it degrades to escaped text (the same
    /// behavior as [`RawHtmlPolicy::Escape`]) so the strictness model stays
    /// consistent.
    Reject,
}

/// Options controlling a browser render.
///
/// The [`Default`] uses [`RenderStrictness::Warn`], [`RawHtmlPolicy::Escape`]
/// (the safe choice — see [`RawHtmlPolicy`]), no [`PageOptions`], no
/// [`CodeRenderer`] (fenced code blocks fall back to plain `<pre><code>`), a
/// [`DefaultFeatureResolver`], and an empty [`FeatureContext`].
pub struct BrowserRenderOptions {
    /// How strictly lossy or unsupported content is treated.
    pub strictness: RenderStrictness,
    /// How [`NodeKind::Html`] nodes are handled.
    pub raw_html: RawHtmlPolicy,
    /// Optional page options applied by [`render_browser_document`].
    pub page: Option<PageOptions>,
    /// Optional hook for bespoke code-block rendering (e.g. syntax
    /// highlighting). When `None`, [`NodeKind::Code`] nodes render as a plain
    /// `<pre><code>` block with a `language-<lang>` class.
    pub code_renderer: Option<Rc<dyn CodeRenderer>>,
    /// The graphics fidelity tier for this render.
    pub graphics_mode: crate::tree::GraphicsMode,
    /// How Mermaid diagrams should be rendered in browser output.
    pub mermaid_mode: crate::tree::BrowserMermaidMode,
    /// Resolves requested [`PageFeature`](crate::browser::feature::PageFeature)s
    /// to their assets. Defaults to [`DefaultFeatureResolver`]; Darkmatter
    /// installs its own on its browser entry points. Shared with [`Rc`] so the
    /// options stay cheap to clone and the trait object stays object-safe.
    pub feature_resolver: Rc<dyn FeatureResolver>,
    /// Renderable-owned context threaded into feature resolution (color mode,
    /// resolved semantic colors).
    pub feature_context: FeatureContext,
    /// When `true`, [`render_browser_document_html`] collects requested features
    /// and returns them in [`Rendered::features`] **without** resolving or
    /// injecting them into the document `<head>`. The caller becomes responsible
    /// for placing the assets (e.g. a body-only embed that injects inline
    /// `<style>`/`<script>` into a wrapper). The default is `false`: the standard
    /// full-document path resolves and injects into `<head>` itself.
    pub defer_feature_injection: bool,
}

impl Default for BrowserRenderOptions {
    fn default() -> BrowserRenderOptions {
        BrowserRenderOptions {
            strictness: RenderStrictness::default(),
            raw_html: RawHtmlPolicy::default(),
            page: None,
            code_renderer: None,
            graphics_mode: crate::tree::GraphicsMode::default(),
            mermaid_mode: crate::tree::BrowserMermaidMode::default(),
            feature_resolver: Rc::new(DefaultFeatureResolver),
            feature_context: FeatureContext::default(),
            defer_feature_injection: false,
        }
    }
}

impl std::fmt::Debug for BrowserRenderOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `dyn CodeRenderer` / `dyn FeatureResolver` are not `Debug`; report
        // only whether a non-default one is installed.
        f.debug_struct("BrowserRenderOptions")
            .field("strictness", &self.strictness)
            .field("raw_html", &self.raw_html)
            .field("page", &self.page)
            .field("code_renderer", &self.code_renderer.is_some())
            .field("graphics_mode", &self.graphics_mode)
            .field("mermaid_mode", &self.mermaid_mode)
            .field("feature_context", &self.feature_context)
            .field("defer_feature_injection", &self.defer_feature_injection)
            .finish()
    }
}

/// Renders a render tree node to a typed [`BrowserFragment<Ready>`].
///
/// The node is validated with [`validate`] first; an error-severity
/// structural finding causes an immediate [`RenderError::InvalidTree`]
/// regardless of [`BrowserRenderOptions::strictness`]. Warning-severity
/// findings are folded into [`Rendered::diagnostics`] under
/// [`RenderStrictness::Warn`] and [`RenderStrictness::Lossy`], and escalate to
/// a [`RenderError::InvalidTree`] under [`RenderStrictness::Strict`].
///
/// ## Returns
///
/// A [`Rendered<BrowserFragment<Ready>>`] carrying the fragment and any
/// non-fatal diagnostics.
///
/// ## Errors
///
/// - [`RenderError::InvalidTree`] if the tree fails structural validation, or
///   if [`RenderStrictness::Strict`] meets a warning-severity validation
///   finding (this includes [`NodeKind::Unsupported`] nodes, whose warning is
///   escalated by the validation gate before the writer runs).
/// - [`RenderError::LossyRejected`] if [`RawHtmlPolicy::Reject`] meets a
///   [`NodeKind::Html`] node under [`RenderStrictness::Strict`].
pub fn render_browser_node(
    node: &RenderNode,
    opts: &BrowserRenderOptions,
) -> Result<Rendered<BrowserFragment<Ready>>, RenderError> {
    let mut writer = gate(node, opts)?;
    let output = writer.render(node)?;
    // The fragment carries its feature requests (interactive Mermaid, prompted
    // links) on the fragments the writer built; roll them up recursively so the
    // side channel mirrors what a whole page would collect.
    let features = output.collect_features();
    Ok(Rendered {
        output,
        diagnostics: writer.diagnostics,
        features,
    })
}

/// Renders a whole [`Document`] to an [`HtmlPage`].
///
/// [`Document::root`] is expected to be a [`NodeKind::Root`]; each of its
/// children becomes a page fragment. When [`BrowserRenderOptions::page`] is
/// present it is applied to the page via [`HtmlPage::apply_page_options`].
///
/// Document frontmatter is **not** wired into page metadata here: the
/// canonical-tree fold does not yet populate
/// [`DocumentMetadata::frontmatter`], so mapping it to `<head>` tags is out of
/// scope for this renderer.
///
/// ## Errors
///
/// Propagates every error from [`render_browser_node`].
///
/// [`DocumentMetadata::frontmatter`]: crate::tree::DocumentMetadata::frontmatter
pub fn render_browser_document(
    doc: &Document,
    opts: &BrowserRenderOptions,
) -> Result<Rendered<HtmlPage>, RenderError> {
    let mut writer = gate(&doc.root, opts)?;

    // The document body is the root's children rendered as page fragments. When
    // the root carries a non-empty `Style` (e.g. a page-level foreground), it is
    // rendered as its own wrapping `<div>` so the style is emitted and inherits
    // to every descendant through CSS — otherwise the root style would be
    // silently discarded. A non-`Root` top-level node always renders as a single
    // wrapping fragment.
    let fragments = match &doc.root.kind {
        NodeKind::Root { children } if root_style_is_empty(&doc.root) => {
            writer.render_each(children)?
        }
        _ => vec![writer.render(&doc.root)?],
    };

    let mut page = HtmlPage::from_fragments(fragments);
    if let Some(page_options) = opts.page.clone() {
        page.apply_page_options(page_options);
    }
    // Install the render's resolver/context so the returned page resolves its
    // features (via `HtmlPage::render`) with the same policy the streaming path
    // uses. Resolving here as well surfaces an unresolved browser feature at
    // this fallible entry point, so the fragment path fails identically to the
    // streaming full-document path (`render_browser_document_html`). This eager
    // call populates the page's memoized head, so the later `render` reuses it
    // and a possibly-impure resolver runs exactly once per feature.
    page.set_feature_resolver(Rc::clone(&opts.feature_resolver));
    page.set_feature_context(opts.feature_context.clone());
    page.resolved_feature_head()?;
    let features = page.features();
    Ok(Rendered {
        output: page,
        diagnostics: writer.diagnostics,
        features,
    })
}

/// Renders a whole [`Document`] directly to a final HTML [`String`].
///
/// This is the **full-document final-string path**: it walks the
/// [`RenderNode`] tree once and streams HTML into a single output buffer
/// instead of building an intermediate [`BrowserFragment`] tree per node and
/// then serializing it. It exists for callers that already own a [`Document`]
/// and need the complete browser output — the render-tree cutover path and the
/// browser perf benchmarks.
///
/// It does **not** replace [`render_browser_document`] /
/// [`render_browser_node`], which remain the API for component composition
/// through [`HtmlPage`] / [`BrowserFragment<Ready>`]. Components still compose
/// through fragments; this is an additional surface for the document renderer.
///
/// The emitted bytes are identical to
/// `render_browser_document(doc, opts)?.output.render()` for supported tree
/// inputs — same validation, diagnostics, page options, metadata, stylesheet /
/// link / script ordering, raw-HTML policy, attributes, escaping, code-renderer
/// hooks, graphics mode, Mermaid mode, styled HR, and semantic wrappers.
///
/// ## Extension islands
///
/// When [`BrowserRenderOptions::code_renderer`] returns a
/// [`BrowserFragment<Ready>`] (a syntax-highlighted code block or a static
/// Mermaid SVG), only that hook result is serialized into the buffer; the whole
/// document is never routed through a fragment tree. Such hook fragments may
/// carry page-level stylesheets, metadata, and dependency links, which roll up
/// into the page `<head>` exactly as they would through [`HtmlPage`].
///
/// ## Errors
///
/// Propagates the same fatal errors as [`render_browser_document`]: a
/// [`RenderError::InvalidTree`] from structural validation (or a strict
/// warning), and any [`RenderError`] raised while walking a node (raw-HTML
/// rejection, an unpromotable strict Mermaid SVG, an unsupported node under
/// [`RenderStrictness::Strict`]).
pub fn render_browser_document_html(
    doc: &Document,
    opts: &BrowserRenderOptions,
) -> Result<Rendered<String>, RenderError> {
    let StreamedDocument {
        body,
        head_page,
        features,
        diagnostics,
    } = stream_browser_document(doc, opts)?;
    let output = assemble_full_document(doc, opts, &head_page, &body, &features)?;
    Ok(Rendered {
        output,
        diagnostics,
        features,
    })
}

/// A [`Document`] rendered in both the standalone full-document form and the
/// embeddable body-fragment form, for callers (like Darkmatter's page frame)
/// that decide between the two **after** rendering — e.g. once the requested
/// [`features`](Rendered::features) are known.
///
/// Produced by [`render_browser_document_body`] from a single streaming pass so
/// the two forms cannot drift.
pub struct BrowserDocumentBody {
    /// The standalone full document (`<!DOCTYPE html><html><head>…</head><body>…
    /// </body></html>`) — byte-identical to [`render_browser_document_html`].
    /// Use this when the output stands alone rather than embedding.
    pub document: String,
    /// The document's inner `<head>` content only — the charset / viewport /
    /// title / microdata / links / design-token `:root` block / `.code-block`
    /// panel stylesheet / scripts [`document`](Self::document) carries between
    /// its `<head>` … `</head>` tags, with **no** deferred
    /// [`features`](Rendered::features) appended (those are the caller's to
    /// place). A caller assembling its own standalone document around
    /// [`body`](Self::body) reuses this as the real `<head>` payload instead of
    /// emitting an empty `<head>`. Empty only when the render produced no head
    /// content at all.
    pub head: String,
    /// The `<body>` inner HTML only: the root's children streamed in order (a
    /// styled root as its own wrapping `<div>`), carrying **no** `<!DOCTYPE>`,
    /// `<html>`, `<head>`, or `<body>` element, so it can be spliced directly
    /// into a host wrapper.
    pub body: String,
    /// The page-level `<style>` / `<script>` assets rolled up from the render
    /// (design-token `:root` block, page and component stylesheets, and script
    /// blocks), already wrapped in their elements so a caller can embed a
    /// self-contained fragment. Empty when the render produced no CSS or JS.
    ///
    /// Requested [`features`](Rendered::features) are **not** part of this
    /// string — an embeddable body has no `<head>` to inject into, so the
    /// features are returned in the side channel for the caller to resolve and
    /// place (see [`BrowserRenderOptions::defer_feature_injection`]).
    pub assets: String,
}

/// Renders a whole [`Document`] to both the standalone document and an
/// embeddable body fragment (plus its rolled-up page-level assets) in one pass.
///
/// This is the embeddable companion to [`render_browser_document_html`]: it
/// streams the body once and returns both [`BrowserDocumentBody::document`]
/// (the full `<!DOCTYPE html>…` form, byte-identical to the full-document path)
/// and [`BrowserDocumentBody::body`] — the same body **without** the
/// surrounding document scaffold, so an embedding caller never nests a full
/// document inside a host element. The page-level `<style>` / `<script>` the
/// full-document path places in `<head>` are also returned separately in
/// [`BrowserDocumentBody::assets`] for inline embedding.
///
/// Requested features are always returned in [`Rendered::features`]. Whether
/// they are injected into [`BrowserDocumentBody::document`]'s `<head>` follows
/// [`BrowserRenderOptions::defer_feature_injection`] exactly as for
/// [`render_browser_document_html`]; they are never part of `body`/`assets`.
///
/// ## Errors
///
/// Propagates the same fatal errors as [`render_browser_document_html`]: a
/// [`RenderError::InvalidTree`] from structural validation (or a strict
/// warning), and any [`RenderError`] raised while streaming a node or resolving
/// a non-deferred feature for the standalone document.
pub fn render_browser_document_body(
    doc: &Document,
    opts: &BrowserRenderOptions,
) -> Result<Rendered<BrowserDocumentBody>, RenderError> {
    let StreamedDocument {
        body,
        head_page,
        features,
        diagnostics,
    } = stream_browser_document(doc, opts)?;

    let document = assemble_full_document(doc, opts, &head_page, &body, &features)?;

    // The document's inner `<head>` content, computed the same way
    // `assemble_full_document` does (same fallback-title source, same
    // `render_head` helper) but **without** the deferred feature assets, which
    // the caller owns. A caller wrapping `body` in its own standalone scaffold
    // reuses this as the real `<head>` instead of emitting an empty one.
    let head = head_page.render_head(tree_first_h1_text(&doc.root).as_deref());

    // Roll up the page-level CSS/JS the full-document path would place in
    // `<head>` so the caller can embed them inline. `stylesheet()` always
    // carries at least the `:root` design-token block; `inline_code()` is empty
    // unless a component contributed a script block.
    let mut assets = String::new();
    let css = head_page.stylesheet();
    if !css.is_empty() {
        assets.push_str("<style>");
        assets.push_str(&css);
        assets.push_str("</style>");
    }
    let js = head_page.inline_code();
    if !js.is_empty() {
        assets.push_str("<script>");
        assets.push_str(&js);
        assets.push_str("</script>");
    }

    Ok(Rendered {
        output: BrowserDocumentBody {
            document,
            head,
            body,
            assets,
        },
        diagnostics,
        features,
    })
}

/// The intermediate product of streaming a [`Document`]'s body: the body HTML,
/// the [`HtmlPage`] seeded with the render's hook fragments and page options
/// (the head-rollup source), the deduped feature requests, and the folded
/// diagnostics. Shared by [`render_browser_document_html`] and
/// [`render_browser_document_body`] so the two paths cannot drift.
struct StreamedDocument {
    body: String,
    head_page: HtmlPage,
    features: Vec<PageFeature>,
    diagnostics: Vec<Diagnostic>,
}

/// Streams a [`Document`]'s body into a single buffer and seeds the head-rollup
/// [`HtmlPage`] from the render's hook fragments and page options.
///
/// The shared front half of both browser document paths; the callers differ
/// only in whether they wrap the body in a full document
/// ([`render_browser_document_html`]) or return it as an embeddable fragment
/// ([`render_browser_document_body`]).
fn stream_browser_document(
    doc: &Document,
    opts: &BrowserRenderOptions,
) -> Result<StreamedDocument, RenderError> {
    let diagnostics = validate_and_collect_diagnostics(&doc.root, opts)?;
    let mut writer = StreamWriter {
        opts,
        diagnostics,
        hook_fragments: Vec::new(),
        features: Vec::new(),
        popover_ids: PopoverIdAllocator::default(),
        buf: String::new(),
    };

    // The body is the root's children streamed in order; a styled root streams
    // as its own wrapping `<div>` so a page-level style inherits to descendants,
    // and a non-`Root` top-level node streams as a single wrapping element —
    // mirroring `render_browser_document`'s body construction.
    match &doc.root.kind {
        NodeKind::Root { children } if root_style_is_empty(&doc.root) => {
            for child in children {
                writer.write(child)?;
            }
        }
        _ => writer.write(&doc.root)?,
    }

    let StreamWriter {
        diagnostics,
        hook_fragments,
        features: collected_features,
        buf: body,
        ..
    } = writer;

    // The `<head>` is built through the same `HtmlPage` head construction so
    // the two paths cannot drift. Only hook fragments contribute head rollups
    // (page-level structural fragments carry no stylesheet / metadata / links),
    // so a page seeded with just those fragments produces an identical head.
    let mut head_page = HtmlPage::from_fragments(hook_fragments);
    if let Some(page_options) = opts.page.clone() {
        head_page.apply_page_options(page_options);
    }

    Ok(StreamedDocument {
        body,
        head_page,
        features: dedup_features(&collected_features),
        diagnostics,
    })
}

/// Assembles the full standalone document string from a streamed body and its
/// head-rollup page, resolving and injecting requested features into `<head>`
/// unless [`BrowserRenderOptions::defer_feature_injection`] is set.
///
/// `first_h1_text` walks fragments, which the streaming path does not build, so
/// the equivalent first-`<h1>` text is computed from the tree and passed to
/// [`HtmlPage::render_head`].
fn assemble_full_document(
    doc: &Document,
    opts: &BrowserRenderOptions,
    head_page: &HtmlPage,
    body: &str,
    features: &[PageFeature],
) -> Result<String, RenderError> {
    let fallback_title = tree_first_h1_text(&doc.root);
    let head = head_page.render_head(fallback_title.as_deref());

    // Resolve the accumulated features once through the render's configured
    // resolver/context and append the serialized assets after the page's
    // authored `<head>` content — the same position (and same helper) the
    // fragment path injects through `HtmlPage::render_head`, so the two paths
    // stay byte-identical. An unresolved browser feature fails the render here.
    //
    // When `defer_feature_injection` is set the caller owns placement (e.g. a
    // body-only embed), so the features are returned unresolved in the side
    // channel and no `<head>` assets are emitted.
    let feature_head = if features.is_empty() || opts.defer_feature_injection {
        String::new()
    } else {
        let resolved = resolve_features(
            features,
            opts.feature_resolver.as_ref(),
            crate::target::RenderTarget::Browser,
            &opts.feature_context,
        )?;
        serialize_features_head(&resolved)
    };
    Ok(format!(
        "<!DOCTYPE html><html><head>{head}{feature_head}</head><body>{body}</body></html>"
    ))
}

/// Validates `node` and builds a [`Writer`], folding (or escalating) every
/// warning-severity validation finding per the strictness model.
fn gate<'a>(node: &RenderNode, opts: &'a BrowserRenderOptions) -> Result<Writer<'a>, RenderError> {
    let diagnostics = validate_and_collect_diagnostics(node, opts)?;
    Ok(Writer { opts, diagnostics })
}

/// Runs the shared validation gate for both the fragment writer
/// ([`gate`]) and the direct document-string writer
/// ([`render_browser_document_html`]).
///
/// Returns the warning-severity validation findings folded into
/// [`Diagnostic`]s per the strictness model, or a fatal [`RenderError`] when an
/// error-severity finding is present (always) or a warning is met under
/// [`RenderStrictness::Strict`].
fn validate_and_collect_diagnostics(
    node: &RenderNode,
    opts: &BrowserRenderOptions,
) -> Result<Vec<Diagnostic>, RenderError> {
    let report = validate(node, ValidationMode::Full);
    if report.has_errors() {
        return Err(ValidationError {
            findings: report.errors().cloned().collect(),
        }
        .into());
    }

    let mut diagnostics = Vec::new();
    for finding in &report.findings {
        if finding.severity != Severity::Warning {
            continue;
        }
        match opts.strictness {
            RenderStrictness::Strict => {
                return Err(RenderError::InvalidTree {
                    findings: report.findings.clone(),
                });
            }
            RenderStrictness::Warn => {
                diagnostics.push(Diagnostic::validation(
                    Severity::Warning,
                    finding.message.clone(),
                    finding.span.clone(),
                ));
            }
            RenderStrictness::Lossy => {}
        }
    }
    Ok(diagnostics)
}

/// Threads render options and accumulating diagnostics through the recursion.
struct Writer<'a> {
    opts: &'a BrowserRenderOptions,
    diagnostics: Vec<Diagnostic>,
}

impl Writer<'_> {
    /// Renders a single node and its subtree to a [`BrowserFragment<Ready>`].
    ///
    /// A node's bold / italic / strikethrough [`Style`](crate::style::Style)
    /// emphasis is lowered differently by element category: an **inline** node
    /// has its fragment wrapped in the matching semantic elements
    /// (`<strong>`, `<em>`, `<s>`); a **block** node lowers the same emphasis
    /// to inline CSS via [`node_attributes`], because wrapping a block element
    /// inside `<strong>`/`<em>`/`<s>` is invalid HTML. The color, background,
    /// underline, dim, blink, and inverse layers always lower to CSS.
    fn render(&mut self, node: &RenderNode) -> Result<BrowserFragment<Ready>, RenderError> {
        let fragment = self.render_kind(node)?;
        if is_inline_node_kind(&node.kind) {
            Ok(wrap_style_emphasis(&node.attrs, fragment))
        } else {
            // Block emphasis is already lowered to CSS by `node_attributes`.
            Ok(fragment)
        }
    }

    /// Renders a single node by its [`NodeKind`], without applying the
    /// semantic emphasis wrappers of a declared [`Style`](crate::style::Style).
    fn render_kind(&mut self, node: &RenderNode) -> Result<BrowserFragment<Ready>, RenderError> {
        match &node.kind {
            NodeKind::Root { children } => self.block(BlockTag::Div, &node.attrs, children),
            NodeKind::Heading { depth, children } => {
                self.block(heading_tag(*depth), &node.attrs, children)
            }
            NodeKind::Section {
                depth,
                heading,
                children,
            } => self.render_section(node, *depth, heading, children),
            NodeKind::Paragraph { children } => {
                // A projected `Progress` widget carries `ProgressHints`. The
                // browser emits a semantic CSS progress bar before falling
                // back to normal paragraph rendering.
                if let Some(hints) = node.attrs.progress_hints_ref() {
                    self.render_progress(node, hints, children)
                } else {
                    self.block(BlockTag::P, &node.attrs, children)
                }
            }
            NodeKind::BlockQuote { children } => {
                if let Some(hints) = node.attrs.columns_hints_ref() {
                    self.render_columns(node, hints, children)
                } else {
                    self.block(BlockTag::Blockquote, &node.attrs, children)
                }
            }
            NodeKind::List {
                ordered,
                start,
                children,
            } => self.render_list(node, *ordered, *start, children),
            NodeKind::ListItem { checked, children } => {
                self.render_list_item(node, *checked, children)
            }
            NodeKind::Code { lang, meta, value } => {
                self.render_code_block(node, lang.as_deref(), meta.as_deref(), value)
            }
            NodeKind::ThematicBreak => Ok(self.render_thematic_break(&node.attrs)),
            NodeKind::Table { align, children } => self.render_table(node, align, children),
            NodeKind::TableRow { children } => self.block(BlockTag::Tr, &node.attrs, children),
            NodeKind::TableCell { children } => self.block(BlockTag::Td, &node.attrs, children),
            NodeKind::FootnoteDefinition {
                identifier,
                children,
            } => self.render_footnote_definition(node, identifier, children),
            NodeKind::Text { value } => Ok(text_fragment(value)),
            NodeKind::Emphasis { children } => self.block(BlockTag::Em, &node.attrs, children),
            NodeKind::Strong { children } => self.block(BlockTag::Strong, &node.attrs, children),
            NodeKind::Delete { children } => self.block(BlockTag::S, &node.attrs, children),
            NodeKind::Span { children } => self.block(BlockTag::Span, &node.attrs, children),
            NodeKind::InlineCode { value } => Ok(self.render_inline_code(node, value)),
            NodeKind::Link {
                url,
                title,
                children,
            } => self.render_link(node, url, title.as_deref(), children),
            NodeKind::Image { url, title, alt } => {
                Ok(self.render_image(node, url, title.as_deref(), alt))
            }
            NodeKind::FootnoteReference { identifier } => {
                Ok(self.render_footnote_reference(node, identifier))
            }
            // A soft break is a single space between phrasing runs.
            NodeKind::SoftBreak => Ok(text_fragment(" ")),
            NodeKind::HardBreak => Ok(self.void(VoidTag::Br, &node.attrs)),
            NodeKind::Html { value, block } => self.render_html(node, value, *block),
            NodeKind::Extended {
                token, children, ..
            } => self.render_extended(node, token, children),
            NodeKind::Unsupported { label } => self.render_unsupported(node, label),
            NodeKind::Disclosure { summary, children, .. } => {
                self.render_disclosure(node, summary, children)
            }
        }
    }

    /// Renders a disclosure block as native `\u003cdetails\u003e`/\u003csummary\u003e` HTML.
    ///
    /// The summary is rendered as phrasing content inside `\u003csummary\u003e`; the
    /// disclosed body is rendered as block children inside `\u003cdetails\u003e`. No
    /// JavaScript is emitted — the native elements provide collapse/expand.
    fn render_disclosure(
        &mut self,
        node: &RenderNode,
        summary: &[RenderNode],
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let mut details = BrowserFragment::new().define_as_block_tag(BlockTag::Details, "");
        for attr in node_attributes(&node.attrs, false) {
            details = details.add_attribute(attr);
        }

        let mut summary_fragment =
            BrowserFragment::new().define_as_block_tag(BlockTag::Summary, "");
        for child in summary {
            summary_fragment = summary_fragment.add_component(self.render(child)?);
        }
        details = details.add_component(summary_fragment.finalize());

        for child in children {
            details = details.add_component(self.render(child)?);
        }

        Ok(details.finalize())
    }

    /// Renders a [`NodeKind::Extended`] node.
    ///
    /// Built-in tokens lower to their semantic browser form: `mark` recovers
    /// the `<mark>` element (closing the legacy `<span class="mark">` fidelity
    /// regression) and `dim` becomes a `<span>` carrying the shared dim visual
    /// policy (`opacity:0.6`, matching [`Style`]'s dim emphasis). An
    /// unrecognized token falls back to a neutral `<span class="extended-{token}">`
    /// wrapper whose class lets a stylesheet target it; the nested `children`
    /// are preserved in every case.
    ///
    /// [`Style`]: crate::style::Style
    fn render_extended(
        &mut self,
        node: &RenderNode,
        token: &str,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        match token {
            "mark" => self.block(BlockTag::Mark, &node.attrs, children),
            "dim" => self.block_with_extra_style(BlockTag::Span, &node.attrs, "opacity:0.6", children),
            _ => {
                let mut attrs = node.attrs.clone();
                attrs.classes.push(format!("extended-{token}"));
                self.block(BlockTag::Span, &attrs, children)
            }
        }
    }

    /// Renders every node in `children` to its own fragment.
    fn render_each(
        &mut self,
        children: &[RenderNode],
    ) -> Result<Vec<BrowserFragment<Ready>>, RenderError> {
        let mut out = Vec::with_capacity(children.len());
        for child in children {
            out.push(self.render(child)?);
        }
        Ok(out)
    }

    /// Builds a block-tag fragment, attaching `attrs` and rendered children.
    fn block(
        &mut self,
        tag: BlockTag,
        attrs: &NodeAttrs,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let inline = is_inline_block_tag(&tag);
        let mut fragment = BrowserFragment::new().define_as_block_tag(tag, "");
        for attr in node_attributes(attrs, inline) {
            fragment = fragment.add_attribute(attr);
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Builds an inline block-tag fragment that also carries an `extra_css`
    /// inline `style` declaration, merging it onto any `style` produced from
    /// the node's attrs rather than overwriting it.
    ///
    /// Used for tokens (such as `dim`) whose browser lowering has no dedicated
    /// element and is expressed purely as inline CSS.
    fn block_with_extra_style(
        &mut self,
        tag: BlockTag,
        attrs: &NodeAttrs,
        extra_css: &str,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let inline = is_inline_block_tag(&tag);
        let mut fragment = BrowserFragment::new().define_as_block_tag(tag, "");
        let mut style_emitted = false;
        for attr in node_attributes(attrs, inline) {
            match attr {
                HtmlAttribute::Other(key, value) if key == "style" => {
                    fragment = fragment.add_attribute(HtmlAttribute::Other(
                        "style".into(),
                        format!("{value};{extra_css}"),
                    ));
                    style_emitted = true;
                }
                other => fragment = fragment.add_attribute(other),
            }
        }
        if !style_emitted {
            fragment = fragment
                .add_attribute(HtmlAttribute::Other("style".into(), extra_css.to_string()));
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Builds a void-tag fragment carrying `attrs`.
    fn void(&self, tag: VoidTag, attrs: &NodeAttrs) -> BrowserFragment<Ready> {
        let inline = is_inline_void_tag(&tag);
        let mut fragment = BrowserFragment::new().define_as_void_tag(tag);
        for attr in node_attributes(attrs, inline) {
            fragment = fragment.add_attribute(attr);
        }
        fragment.finalize()
    }

    /// Builds the `<hr>` or styled SVG for a [`NodeKind::ThematicBreak`].
    ///
    /// Under [`GraphicsMode::Off`] the node degrades to a plain `<hr>`
    /// carrying any typed [`ThematicBreakAttrs`] as `data-hr-*` attributes so
    /// the styled rule is still user-observable. Under [`GraphicsMode::Vector`]
    /// and [`GraphicsMode::Rich`] those attrs drive an inline SVG whose
    /// primitives reference CSS custom properties (`--hr-weight`,
    /// `--hr-color`, `--hr-width`) so page-level overrides take effect.
    ///
    /// [`ThematicBreakAttrs`]: crate::tree::ThematicBreakAttrs
    fn render_thematic_break(&self, attrs: &NodeAttrs) -> BrowserFragment<Ready> {
        use crate::tree::GraphicsMode;

        let hr = attrs.thematic_break_ref();
        match self.opts.graphics_mode {
            GraphicsMode::Off => {
                let mut fragment = BrowserFragment::new().define_as_void_tag(VoidTag::Hr);
                for attr in node_attributes(attrs, is_inline_void_tag(&VoidTag::Hr)) {
                    fragment = fragment.add_attribute(attr);
                }
                for (key, value) in hr_data_attr_pairs(hr) {
                    fragment = fragment.add_attribute(HtmlAttribute::Data(
                        HtmlDataAttribute::new(format!("hr-{key}")),
                        value.to_string(),
                    ));
                }
                fragment.finalize()
            }
            GraphicsMode::Vector | GraphicsMode::Rich => {
                let svg = crate::tree::graphics::horizontal_rule_svg(
                    hr.and_then(|h| h.kind),
                    hr.and_then(|h| h.weight),
                    hr.and_then(|h| h.alignment),
                    hr.and_then(|h| h.width.as_deref()),
                    hr.and_then(|h| h.color.as_deref()),
                    "0",
                    "0",
                );
                // Spec C5/C6: the rule owns a block box, so the node's outer
                // `Layout` must place and size it on Browser exactly as on
                // Terminal. The SVG carries only the rule's intrinsic
                // `RuleAlignment` centering (its own `margin:auto`); wrapping it
                // in a block box keeps that from fighting the outer margin over a
                // single `margin` shorthand. Only `margin` / `width` / `max_width`
                // are honored — padding is N/A (a rule has no padding box) — and a
                // default/absent layout adds no wrapper (see `hr_outer_box_css`).
                let html = match attrs.layout_ref().and_then(hr_outer_box_css) {
                    Some(box_css) => format!("<div style=\"{box_css}\">{svg}</div>"),
                    None => svg,
                };
                BrowserFragment::new().define_as_raw_html(html).finalize()
            }
        }
    }

    /// Renders a section as `<section>` containing a heading tag and body.
    fn render_section(
        &mut self,
        node: &RenderNode,
        depth: HeadingDepth,
        heading: &[RenderNode],
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::Section, "");
        for attr in node_attributes(&node.attrs, false) {
            fragment = fragment.add_attribute(attr);
        }

        // Render the heading as h1-h6.
        let mut heading_fragment =
            BrowserFragment::new().define_as_block_tag(heading_tag(depth), "");
        for child in heading {
            heading_fragment = heading_fragment.add_component(self.render(child)?);
        }
        fragment = fragment.add_component(heading_fragment.finalize());

        // Render the body children.
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a two-column block quote as a `<div class="columns">` flex
    /// container holding two `<div class="column">` children.
    ///
    /// The flat child list is split at [`ColumnsHints::left_count`]; the left
    /// children fill the first column `<div>`, the rest fill the second. The
    /// `columns` / `column` classes are preserved as external-CSS hooks. The
    /// container additionally gets inline `display:flex` and `gap` CSS, plus
    /// any layout CSS from [`NodeAttrs::layout`]. The left column carries the
    /// width CSS from [`ColumnsHints::left_width`]; the right column flexes to
    /// fill the remaining width.
    ///
    /// [`ColumnsHints::left_count`]: crate::tree::ColumnsHints::left_count
    /// [`ColumnsHints::left_width`]: crate::tree::ColumnsHints::left_width
    fn render_columns(
        &mut self,
        node: &RenderNode,
        hints: &crate::tree::ColumnsHints,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let split = hints.left_count.min(children.len());
        let (left, right) = children.split_at(split);

        // The node's `id` / `class` / layout / `Style` attributes are lowered
        // through the shared `node_attributes` helper so user-supplied class
        // hooks and node `Style` are preserved alongside the literal `columns`
        // class and the inline `display`/`gap` flex CSS.
        let mut container = BrowserFragment::new().define_as_block_tag(BlockTag::Div, "");
        let attributes = node_attributes(&node.attrs, false);
        let mut class_emitted = false;
        let mut style_emitted = false;
        for attr in attributes {
            match attr {
                // Merge the literal `columns` class ahead of any user classes.
                HtmlAttribute::Class(class) => {
                    container = container.add_attribute(HtmlAttribute::Class(
                        ClassDefinition::new(format!("columns {}", class.as_str())),
                    ));
                    class_emitted = true;
                }
                // Merge node layout / `Style` CSS into the flex container
                // `style` without overwriting the `display`/`gap` declarations.
                HtmlAttribute::Other(key, value) if key == "style" => {
                    container = container.add_attribute(HtmlAttribute::Other(
                        "style".into(),
                        super::shared::columns_container_css(hints, &value),
                    ));
                    style_emitted = true;
                }
                other => container = container.add_attribute(other),
            }
        }
        if !class_emitted {
            container =
                container.add_attribute(HtmlAttribute::Class(ClassDefinition::new("columns")));
        }
        if !style_emitted {
            container = container.add_attribute(HtmlAttribute::Other(
                "style".into(),
                super::shared::columns_container_css(hints, ""),
            ));
        }

        for (group, column_css) in [
            (left, super::shared::left_column_css(hints.left_width)),
            (right, super::shared::right_column_css().to_string()),
        ] {
            let mut column = BrowserFragment::new().define_as_block_tag(BlockTag::Div, "");
            column = column.add_attribute(HtmlAttribute::Class(ClassDefinition::new("column")));
            column = column.add_attribute(HtmlAttribute::Other("style".into(), column_css));
            for child in group {
                column = column.add_component(self.render(child)?);
            }
            container = container.add_component(column.finalize());
        }
        Ok(container.finalize())
    }

    /// Renders a projected `Progress` widget as a semantic CSS progress bar.
    ///
    /// The HTML carries `role="progressbar"` plus ARIA value attributes and
    /// `progress-*` classes. Color slots lower to inline `background-color`;
    /// non-default glyphs and brackets are preserved as `data-*` attributes.
    /// Node [`Layout`](crate::layout::Layout) is lowered onto the outer
    /// element through the existing browser layout lowering.
    fn render_progress(
        &mut self,
        node: &RenderNode,
        hints: &crate::tree::ProgressHints,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        // The accessible label is recovered from the paragraph's fallback
        // text — the same `"{label} {pct}%"` shape the terminal renderer uses.
        // The *plain text* of the children is collected (not their rendered
        // HTML) so the label is escaped exactly once by `progress_html`.
        let fallback_text = plain_text(children);
        let layout_css = node
            .attrs
            .layout_ref()
            .map(layout_to_css)
            .filter(|css| !css.is_empty())
            .unwrap_or_default();
        let html = super::shared::progress_html(hints, &fallback_text, &layout_css);
        Ok(BrowserFragment::new().define_as_raw_html(html).finalize())
    }

    /// Renders a list as `<ol>` (respecting `start`) or `<ul>`.
    fn render_list(
        &mut self,
        node: &RenderNode,
        ordered: bool,
        start: Option<u64>,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let tag = if ordered { BlockTag::Ol } else { BlockTag::Ul };
        let mut fragment = BrowserFragment::new().define_as_block_tag(tag, "");
        // A typed list marker policy lowers per its browser fidelity:
        //
        // - `None` is *faithful* — the browser genuinely has a no-marker list
        //   (`list-style:none`), so it degrades silently with no diagnostic.
        // - `TreeConnectors` is *lossy* — the browser cannot draw terminal
        //   box-drawing connector geometry, so it degrades to a plain
        //   no-marker list and reports the loss per the strictness model.
        let marker_css = match node.attrs.list_marker_policy() {
            crate::tree::ListMarkerPolicy::Default => None,
            crate::tree::ListMarkerPolicy::None => Some("list-style:none"),
            crate::tree::ListMarkerPolicy::TreeConnectors => {
                let message = "list marker policy 'tree_connectors' has no \
                               browser equivalent; degraded to a plain \
                               no-marker list"
                    .to_string();
                match self.opts.strictness {
                    RenderStrictness::Strict => {
                        return Err(RenderError::LossyRejected { message });
                    }
                    RenderStrictness::Warn => {
                        self.diagnostics
                            .push(Diagnostic::lossy(message, Some(node.span.clone())));
                    }
                    RenderStrictness::Lossy => {}
                }
                Some("list-style:none")
            }
        };
        let attributes = node_attributes(&node.attrs, false);
        let has_style = attributes
            .iter()
            .any(|a| matches!(a, HtmlAttribute::Other(k, _) if k == "style"));
        for attr in attributes {
            // Merge the marker-policy CSS into the existing style attribute
            // rather than emitting a second, conflicting one.
            match (&attr, marker_css) {
                (HtmlAttribute::Other(key, value), Some(extra)) if key == "style" => {
                    fragment = fragment.add_attribute(HtmlAttribute::Other(
                        "style".into(),
                        format!("{value};{extra}"),
                    ));
                }
                _ => fragment = fragment.add_attribute(attr),
            }
        }
        // When no style attribute was emitted, add one for the marker policy.
        if let Some(extra) = marker_css.filter(|_| !has_style) {
            fragment =
                fragment.add_attribute(HtmlAttribute::Other("style".into(), extra.to_string()));
        }
        // `<ol start="N">` controls the first item's number.
        if ordered && let Some(start) = start.filter(|s| *s != 1) {
            fragment =
                fragment.add_attribute(HtmlAttribute::Other("start".into(), start.to_string()));
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a list item as `<li>`. A task item (`checked` is `Some`) gets a
    /// leading disabled `<input type=checkbox>` and the `task-list-item` class.
    fn render_list_item(
        &mut self,
        node: &RenderNode,
        checked: Option<bool>,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let mut attrs = node.attrs.clone();
        if checked.is_some() {
            attrs.classes.push("task-list-item".to_string());
        }

        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::Li, "");
        for attr in node_attributes(&attrs, false) {
            fragment = fragment.add_attribute(attr);
        }
        if let Some(checked) = checked {
            fragment = fragment.add_component(checkbox(checked));
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a fenced code block.
    ///
    /// When [`BrowserRenderOptions::code_renderer`] is set, the hook is given
    /// the language, body, info-string `meta`, and node attributes; a `Some`
    /// result is used verbatim. A `None` result (or no hook) falls back to the
    /// built-in plain `<pre><code>` rendering, with the language carried as a
    /// `language-<lang>` class on the `<code>` element.
    ///
    /// ## Mermaid promotion
    ///
    /// Mermaid code blocks are promoted based on [`BrowserRenderOptions::graphics_mode`]
    /// (the fidelity ceiling) and [`BrowserRenderOptions::mermaid_mode`] (the
    /// opt-in):
    ///
    /// | `graphics_mode` | `mermaid_mode`   | Result                                          |
    /// |-----------------|------------------|-------------------------------------------------|
    /// | `Off`           | any              | Plain `<pre><code>` (no promotion)              |
    /// | `Vector`/`Rich` | `Code`           | Plain `<pre><code class="language-mermaid">`    |
    /// | `Vector`        | `Interactive`    | Degrades to `StaticSvg` — `Vector` permits no scripts |
    /// | `Rich`          | `Interactive`    | `<pre class="mermaid">` (client-side mermaid.js) |
    /// | `Vector`/`Rich` | `StaticSvg`      | Static `<svg>` via [`CodeRenderer::render_browser_mermaid`]; falls back |
    ///
    /// `Interactive` is a script-capable presentation, so it sits above the
    /// `Vector` ceiling ("no scripts"). A `Vector` + `Interactive` request is
    /// therefore capped down to `StaticSvg`; only `Rich` reaches the mermaid.js
    /// path.
    ///
    /// `StaticSvg` promotion never runs through the generic
    /// [`CodeRenderer::render_browser_code`] hook: it uses the dedicated,
    /// fallible [`CodeRenderer::render_browser_mermaid`] hook so a `None`
    /// (failed SVG generation) is observable as a failure rather than being
    /// hidden behind a hook's own code-block fallback, and strictness can apply
    /// to it. Every non-promotion outcome (`Off`, `Code`, or a `StaticSvg`
    /// failure that degrades under `Warn`/`Lossy`) instead routes through
    /// [`Self::render_code_fallback`], which consults `render_browser_code`
    /// first so the original code-block presentation — `title`, line numbers,
    /// highlights — is preserved (the spec's lossless-fallback contract). That
    /// hook's contract forbids promoting `lang="mermaid"`, so consulting it for
    /// the fallback cannot re-introduce an SVG.
    ///
    /// ## Errors
    ///
    /// Under [`RenderStrictness::Strict`] a `StaticSvg` promotion that cannot
    /// produce an SVG returns [`RenderError::LossyRejected`]; under
    /// [`RenderStrictness::Warn`] it records a lossy diagnostic and falls back
    /// to the plain code block.
    fn render_code_block(
        &mut self,
        node: &RenderNode,
        lang: Option<&str>,
        meta: Option<&str>,
        value: &str,
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        use crate::tree::{BrowserMermaidMode, GraphicsMode};

        let is_mermaid = lang
            .map(|l| l.eq_ignore_ascii_case("mermaid"))
            .unwrap_or(false);

        if is_mermaid {
            // Off caps promotion. The block still renders through the lossless
            // code fallback (title / line numbers / highlights preserved); the
            // generic `render_browser_code` hook cannot re-promote mermaid, so
            // consulting it here is safe.
            if self.opts.graphics_mode == GraphicsMode::Off {
                return Ok(self.render_code_fallback(node, lang, meta, value));
            }
            // `Vector` permits no scripts, so an `Interactive` request degrades
            // to the static SVG form; only `Rich` reaches the mermaid.js path.
            let mermaid_mode = match (self.opts.graphics_mode, self.opts.mermaid_mode) {
                (GraphicsMode::Vector, BrowserMermaidMode::Interactive) => {
                    BrowserMermaidMode::StaticSvg
                }
                (_, mode) => mode,
            };
            return match mermaid_mode {
                BrowserMermaidMode::Interactive => Ok(self.render_mermaid_interactive(node, value)),
                BrowserMermaidMode::StaticSvg => {
                    if let Some(renderer) = &self.opts.code_renderer
                        && let Some(fragment) =
                            renderer.render_browser_mermaid(value, meta, &node.attrs)
                    {
                        return Ok(fragment);
                    }
                    // No hook, or SVG generation failed: degrade per strictness.
                    // The degrade path uses the lossless code fallback so the
                    // failed diagram keeps its code-block metadata.
                    match self.opts.strictness {
                        RenderStrictness::Strict => Err(RenderError::LossyRejected {
                            message: "Mermaid static SVG promotion failed".to_string(),
                        }),
                        RenderStrictness::Warn => {
                            self.note_lossy(
                                "Mermaid static SVG promotion failed; rendered as code block",
                                node,
                            );
                            Ok(self.render_code_fallback(node, lang, meta, value))
                        }
                        RenderStrictness::Lossy => {
                            Ok(self.render_code_fallback(node, lang, meta, value))
                        }
                    }
                }
                BrowserMermaidMode::Code => Ok(self.render_code_fallback(node, lang, meta, value)),
            };
        }

        Ok(self.render_code_fallback(node, lang, meta, value))
    }

    /// Renders a code block through the generic [`CodeRenderer::render_browser_code`]
    /// hook when one is installed, falling back to the plain `<pre><code>`
    /// block when there is no hook or it declines (`None`).
    ///
    /// This is the lossless fallback for both ordinary code blocks and Mermaid
    /// blocks whose promotion is disabled (`Off`/`Code`) or has failed: the
    /// spec requires the original code-block presentation — `title`, line
    /// numbers, highlights — which the hook reproduces. Mermaid is never
    /// re-promoted here; the hook's contract forbids promoting `lang="mermaid"`
    /// (the dedicated [`CodeRenderer::render_browser_mermaid`] hook owns
    /// promotion), so this path cannot emit an SVG.
    fn render_code_fallback(
        &self,
        node: &RenderNode,
        lang: Option<&str>,
        meta: Option<&str>,
        value: &str,
    ) -> BrowserFragment<Ready> {
        if let Some(renderer) = &self.opts.code_renderer
            && let Some(fragment) = renderer.render_browser_code(lang, value, meta, &node.attrs)
        {
            return fragment;
        }
        self.render_plain_code_block(node, lang, value)
    }

    /// Renders a Mermaid code block as an interactive `<pre class="mermaid">`.
    ///
    /// The diagram source is escaped by the fragment renderer so it is safe
    /// to embed directly. Node attributes (id, classes, layout, style) are
    /// preserved and merged with the required `mermaid` class.
    fn render_mermaid_interactive(
        &self,
        node: &RenderNode,
        value: &str,
    ) -> BrowserFragment<Ready> {
        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::Pre, "");
        fragment = fragment.add_attribute(HtmlAttribute::Class(ClassDefinition::new("mermaid")));
        for attr in node_attributes(&node.attrs, false) {
            fragment = fragment.add_attribute(attr);
        }
        // The interactive form is inert without a client-side bootstrap; request
        // the Mermaid feature so the page assembler injects it. Static SVG /
        // code renderings are self-contained and request nothing.
        fragment
            .add_feature(PageFeature::MermaidDiagram)
            .add_child(ComposableNode::TextFragment(value.to_string()))
            .finalize()
    }

    /// Renders a plain `<pre><code>` code block, used as the universal fallback
    /// when promotion is disabled, unsupported, or fails.
    fn render_plain_code_block(
        &self,
        node: &RenderNode,
        lang: Option<&str>,
        value: &str,
    ) -> BrowserFragment<Ready> {
        let mut code = BrowserFragment::new().define_as_block_tag(BlockTag::Code, "");
        if let Some(lang) = lang.filter(|lang| !lang.is_empty()) {
            code = code.add_attribute(HtmlAttribute::Class(ClassDefinition::new(format!(
                "language-{lang}"
            ))));
        }
        let code = code.add_child(ComposableNode::TextFragment(value.to_string()));

        let mut pre = BrowserFragment::new().define_as_block_tag(BlockTag::Pre, "");
        for attr in node_attributes(&node.attrs, false) {
            pre = pre.add_attribute(attr);
        }
        pre.add_component(code.finalize()).finalize()
    }

    /// Renders inline code as `<code>` carrying escaped text.
    fn render_inline_code(&self, node: &RenderNode, value: &str) -> BrowserFragment<Ready> {
        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::Code, "");
        for attr in node_attributes(&node.attrs, true) {
            fragment = fragment.add_attribute(attr);
        }
        fragment
            .add_child(ComposableNode::TextFragment(value.to_string()))
            .finalize()
    }

    /// Renders a GFM table. The first child row is the header (`<thead>` with
    /// `<th>` cells); the rest go into `<tbody>`. Per-column [`ColumnAlign`]
    /// is applied as a `text-align` inline style on every cell.
    fn render_table(
        &mut self,
        node: &RenderNode,
        align: &[ColumnAlign],
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let mut table = BrowserFragment::new().define_as_block_tag(BlockTag::Table, "");
        for attr in node_attributes(&node.attrs, false) {
            table = table.add_attribute(attr);
        }

        // A table title/caption is emitted as a `<caption>` before `<thead>`
        // and `<tbody>`. An empty or whitespace-only title is ignored.
        if let Some(title) = node.attrs.table_title_ref()
            && !title.trim().is_empty()
        {
            let caption = BrowserFragment::new()
                .define_as_block_tag(BlockTag::Caption, "")
                .add_component(text_fragment(title.trim()))
                .finalize();
            table = table.add_component(caption);
        }

        let mut rows = children.iter();
        if let Some(header) = rows.next() {
            let header_row = self.render_table_row(header, align, true)?;
            let thead = BrowserFragment::new()
                .define_as_block_tag(BlockTag::Thead, "")
                .add_component(header_row)
                .finalize();
            table = table.add_component(thead);
        }

        let mut tbody = BrowserFragment::new().define_as_block_tag(BlockTag::Tbody, "");
        for row in rows {
            tbody = tbody.add_component(self.render_table_row(row, align, false)?);
        }
        table = table.add_component(tbody.finalize());
        Ok(table.finalize())
    }

    /// Renders a table row; `header` selects `<th>` vs `<td>` cells.
    fn render_table_row(
        &mut self,
        row: &RenderNode,
        align: &[ColumnAlign],
        header: bool,
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let cells = match &row.kind {
            NodeKind::TableRow { children } => children.as_slice(),
            // Validation guarantees table children are rows; this is a
            // defensive empty fallback for trees bypassing the gate.
            _ => &[],
        };
        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::Tr, "");
        for attr in node_attributes(&row.attrs, false) {
            fragment = fragment.add_attribute(attr);
        }
        for (index, cell) in cells.iter().enumerate() {
            fragment = fragment.add_component(self.render_table_cell(
                cell,
                align.get(index).copied().unwrap_or(ColumnAlign::None),
                header,
            )?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a table cell as `<th>` or `<td>`, applying column alignment.
    fn render_table_cell(
        &mut self,
        cell: &RenderNode,
        align: ColumnAlign,
        header: bool,
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let children = match &cell.kind {
            NodeKind::TableCell { children } => children.as_slice(),
            _ => &[],
        };
        let tag = if header { BlockTag::Th } else { BlockTag::Td };
        let mut fragment = BrowserFragment::new().define_as_block_tag(tag, "");
        // Column alignment merges into the cell's existing `style` attribute
        // (from node `Style` / `Layout`) rather than emitting a second,
        // conflicting `style` — the same merge the columns and list paths use.
        let align_css = align_value(align).map(|value| format!("text-align:{value}"));
        let mut style_emitted = false;
        for attr in node_attributes(&cell.attrs, false) {
            match (&attr, &align_css) {
                (HtmlAttribute::Other(key, value), Some(extra)) if key == "style" => {
                    fragment = fragment.add_attribute(HtmlAttribute::Other(
                        "style".into(),
                        format!("{value};{extra}"),
                    ));
                    style_emitted = true;
                }
                _ => fragment = fragment.add_attribute(attr),
            }
        }
        if let Some(extra) = align_css.filter(|_| !style_emitted) {
            fragment = fragment.add_attribute(HtmlAttribute::Other("style".into(), extra));
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a footnote definition as a `<div>` carrying the identifier as
    /// its `id` (`fn-<identifier>`).
    fn render_footnote_definition(
        &mut self,
        node: &RenderNode,
        identifier: &str,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::Div, "");
        fragment =
            fragment.add_attribute(HtmlAttribute::Id(DomId::new(format!("fn-{identifier}"))));
        fragment = fragment.add_attribute(HtmlAttribute::Class(ClassDefinition::new(
            "footnote-definition",
        )));
        for attr in node_attributes(&node.attrs, false) {
            fragment = fragment.add_attribute(attr);
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a footnote reference as an `<a>` anchor to the definition.
    fn render_footnote_reference(
        &self,
        node: &RenderNode,
        identifier: &str,
    ) -> BrowserFragment<Ready> {
        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::A, "");
        for attr in node_attributes(&node.attrs, true) {
            fragment = fragment.add_attribute(attr);
        }
        fragment
            .add_attribute(HtmlAttribute::Other(
                "href".into(),
                format!("#fn-{identifier}"),
            ))
            .add_child(ComposableNode::TextFragment(identifier.to_string()))
            .finalize()
    }

    /// Renders a hyperlink as `<a href title>`.
    ///
    /// A Darkmatter-lowered prompt (`data-prompt`) promotes the link to the
    /// accessible popover structure via [`Self::render_prompted_link`]; a plain
    /// link renders as a bare anchor and requests no feature.
    fn render_link(
        &mut self,
        node: &RenderNode,
        url: &str,
        title: Option<&str>,
        children: &[RenderNode],
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        if let Some(prompt) = link_prompt(&node.attrs) {
            let prompt = prompt.to_string();
            return self.render_prompted_link(node, url, title, children, &prompt);
        }

        let mut fragment = BrowserFragment::new().define_as_block_tag(BlockTag::A, "");
        for attr in node_attributes(&node.attrs, true) {
            fragment = fragment.add_attribute(attr);
        }
        // `href` carries an arbitrary (possibly relative) URL, which the
        // typed `HtmlAttribute::Href(Url)` cannot represent — the `Other`
        // outlet keeps the value escaped without forcing absolute-URL parsing.
        fragment = fragment.add_attribute(HtmlAttribute::Other("href".into(), url.to_string()));
        if let Some(title) = title {
            fragment = fragment.add_attribute(HtmlAttribute::Title(title.to_string()));
        }
        for child in children {
            fragment = fragment.add_component(self.render(child)?);
        }
        Ok(fragment.finalize())
    }

    /// Renders a prompted (enhanced) link as an accessible popover structure.
    ///
    /// The emitted shape is a `<span class="dm-popover-wrapper">` holding the
    /// navigable anchor followed by a `<span … popover="hint">` prompt. The
    /// anchor keeps every existing attribute and its real `href`; the internal
    /// `data-prompt` transport is consumed here and never re-emitted.
    ///
    /// The prompt's document-unique id is **not** allocated here: this returns a
    /// typed [`PopoverNode`] carrying the id-independent pieces plus a readable
    /// slug base, and the id is allocated when the composed page renders (see
    /// [`PopoverNode`] / [`HtmlPage::render`](crate::html::HtmlPage::render)).
    /// That defers allocation to the true final-document assembly point, so two
    /// prompted links rendered independently and composed via
    /// [`HtmlPage::from_fragments`](crate::html::HtmlPage::from_fragments) get
    /// distinct ids instead of colliding (spec criterion 7). The shared Popover
    /// CSS (requested here as [`PageFeature::Popover`]) reveals the prompt on
    /// `:hover` / `:focus-within` so it stays keyboard reachable even where
    /// native interest/popover behavior is absent.
    fn render_prompted_link(
        &mut self,
        node: &RenderNode,
        url: &str,
        title: Option<&str>,
        children: &[RenderNode],
        prompt: &str,
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        let mut anchor_children = Vec::with_capacity(children.len());
        for child in children {
            anchor_children.push(ComposableNode::Component(Box::new(self.render(child)?)));
        }

        let popover = PopoverNode {
            id_base: popover_id_base(url),
            anchor_attrs: prompted_anchor_base_attributes(node, url, title),
            anchor_children,
            prompt_text: prompt.to_string(),
        };

        Ok(BrowserFragment::new()
            .define_as_popover(popover)
            .add_feature(PageFeature::Popover)
            .finalize())
    }

    /// Renders an image as `<img src alt title>`.
    fn render_image(
        &self,
        node: &RenderNode,
        url: &str,
        title: Option<&str>,
        alt: &str,
    ) -> BrowserFragment<Ready> {
        let mut fragment = BrowserFragment::new().define_as_void_tag(VoidTag::Img);
        for attr in node_attributes(&node.attrs, true) {
            fragment = fragment.add_attribute(attr);
        }
        // `src` carries an arbitrary (possibly relative) URL — see the note
        // on `render_link` for why the `Other` outlet is used.
        fragment = fragment
            .add_attribute(HtmlAttribute::Other("src".into(), url.to_string()))
            .add_attribute(HtmlAttribute::Alt(alt.to_string()));
        if let Some(title) = title {
            fragment = fragment.add_attribute(HtmlAttribute::Title(title.to_string()));
        }
        fragment.finalize()
    }

    /// Renders a [`NodeKind::Html`] node per [`RawHtmlPolicy`].
    ///
    /// | Policy   | Strict                     | Warn / Lossy            |
    /// |----------|----------------------------|-------------------------|
    /// | `Allow`  | raw verbatim               | raw verbatim            |
    /// | `Escape` | escaped text + diagnostic* | escaped text + diag.*   |
    /// | `Reject` | `RenderError::LossyRejected` | escaped text + diag.* |
    ///
    /// *No diagnostic is recorded under [`RenderStrictness::Lossy`].
    fn render_html(
        &mut self,
        node: &RenderNode,
        value: &str,
        _block: bool,
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        match self.opts.raw_html {
            RawHtmlPolicy::Allow => Ok(BrowserFragment::new()
                .define_as_raw_html(value.to_string())
                .finalize()),
            RawHtmlPolicy::Escape => {
                self.note_lossy("raw HTML emitted as escaped text", node);
                Ok(text_fragment(value))
            }
            RawHtmlPolicy::Reject => match self.opts.strictness {
                RenderStrictness::Strict => Err(RenderError::LossyRejected {
                    message: "raw HTML rejected by RawHtmlPolicy::Reject".to_string(),
                }),
                RenderStrictness::Warn | RenderStrictness::Lossy => {
                    self.note_lossy("raw HTML rejected and emitted as escaped text", node);
                    Ok(text_fragment(value))
                }
            },
        }
    }

    /// Renders an [`NodeKind::Unsupported`] node according to strictness.
    fn render_unsupported(
        &mut self,
        node: &RenderNode,
        label: &str,
    ) -> Result<BrowserFragment<Ready>, RenderError> {
        match self.opts.strictness {
            // Defensive fallback: trees entered through `render_browser_node`
            // are preempted by the validation gate, which escalates the
            // `Unsupported` warning to `InvalidTree` first.
            RenderStrictness::Strict => Err(RenderError::Unsupported {
                label: label.to_string(),
            }),
            RenderStrictness::Warn => {
                self.diagnostics.push(Diagnostic::unsupported(
                    format!("unsupported content dropped: {label}"),
                    Some(node.span.clone()),
                ));
                Ok(BrowserFragment::new()
                    .define_as_raw_html(format!("<!-- unsupported: {label} -->"))
                    .finalize())
            }
            RenderStrictness::Lossy => Ok(BrowserFragment::new()
                .define_as_text_fragment("")
                .finalize()),
        }
    }

    /// Records a lossy diagnostic, unless strictness is [`RenderStrictness::Lossy`].
    fn note_lossy(&mut self, message: &str, node: &RenderNode) {
        if self.opts.strictness != RenderStrictness::Lossy {
            self.diagnostics.push(Diagnostic::lossy(
                message.to_string(),
                Some(node.span.clone()),
            ));
        }
    }
}

/// Threads render options, accumulating diagnostics, hook fragments, and the
/// output buffer through the direct document-string recursion.
///
/// Unlike [`Writer`], which builds a [`BrowserFragment`] per node, this writer
/// streams HTML straight into [`buf`](StreamWriter::buf). Each method mirrors
/// the corresponding [`Writer`] method so the bytes match: it reuses
/// [`node_attributes`] and [`write_attributes`] for opening tags, and the
/// shared SVG / progress / column helpers for the raw-HTML islands.
struct StreamWriter<'a> {
    opts: &'a BrowserRenderOptions,
    diagnostics: Vec<Diagnostic>,
    /// Fragments returned by [`BrowserRenderOptions::code_renderer`], retained
    /// so their page-level stylesheet / metadata / dependency-link rollups feed
    /// the `<head>` exactly as they would through [`HtmlPage`].
    hook_fragments: Vec<BrowserFragment<Ready>>,
    /// Feature requests accumulated in document order. Unlike the fragment
    /// writer, this path builds no per-node fragment, so features requested at
    /// the streamed nodes (interactive Mermaid, prompted links) plus those
    /// merged from hook fragments are accumulated here for one deduplicated
    /// resolution by the document assembler.
    features: Vec<PageFeature>,
    /// Document-scoped allocator for prompted-link popover ids, shared in spirit
    /// with the fragment [`Writer`] so both paths derive the same deterministic,
    /// collision-free ids in document order.
    popover_ids: PopoverIdAllocator,
    buf: String,
}

impl StreamWriter<'_> {
    /// Streams a node and its subtree, applying the semantic emphasis wrappers
    /// of a declared [`Style`](crate::style::Style) for inline nodes — the
    /// streaming analogue of [`Writer::render`] + [`wrap_style_emphasis`].
    fn write(&mut self, node: &RenderNode) -> Result<(), RenderError> {
        if is_inline_node_kind(&node.kind)
            && let Some(style) = node.attrs.style_ref().filter(|s| !s.is_empty())
        {
            let emphasis = style.emphasis;
            // Outermost-first so the nesting renders `<strong><em><s>…`,
            // matching `wrap_style_emphasis`'s innermost-first construction.
            if emphasis.bold {
                self.buf.push_str("<strong>");
            }
            if emphasis.italic {
                self.buf.push_str("<em>");
            }
            if emphasis.strikethrough {
                self.buf.push_str("<s>");
            }
            self.write_kind(node)?;
            if emphasis.strikethrough {
                self.buf.push_str("</s>");
            }
            if emphasis.italic {
                self.buf.push_str("</em>");
            }
            if emphasis.bold {
                self.buf.push_str("</strong>");
            }
            return Ok(());
        }
        self.write_kind(node)
    }

    /// Streams a single node by its [`NodeKind`] — the streaming analogue of
    /// [`Writer::render_kind`].
    fn write_kind(&mut self, node: &RenderNode) -> Result<(), RenderError> {
        match &node.kind {
            NodeKind::Root { children } => self.block(BlockTag::Div, &node.attrs, children),
            NodeKind::Heading { depth, children } => {
                self.block(heading_tag(*depth), &node.attrs, children)
            }
            NodeKind::Section {
                depth,
                heading,
                children,
            } => self.write_section(node, *depth, heading, children),
            NodeKind::Paragraph { children } => {
                if let Some(hints) = node.attrs.progress_hints_ref() {
                    self.write_progress(node, hints, children);
                    Ok(())
                } else {
                    self.block(BlockTag::P, &node.attrs, children)
                }
            }
            NodeKind::BlockQuote { children } => {
                if let Some(hints) = node.attrs.columns_hints_ref() {
                    self.write_columns(node, hints, children)
                } else {
                    self.block(BlockTag::Blockquote, &node.attrs, children)
                }
            }
            NodeKind::List {
                ordered,
                start,
                children,
            } => self.write_list(node, *ordered, *start, children),
            NodeKind::ListItem { checked, children } => {
                self.write_list_item(node, *checked, children)
            }
            NodeKind::Code { lang, meta, value } => {
                self.write_code_block(node, lang.as_deref(), meta.as_deref(), value)
            }
            NodeKind::ThematicBreak => {
                self.write_thematic_break(&node.attrs);
                Ok(())
            }
            NodeKind::Table { align, children } => self.write_table(node, align, children),
            NodeKind::TableRow { children } => self.block(BlockTag::Tr, &node.attrs, children),
            NodeKind::TableCell { children } => self.block(BlockTag::Td, &node.attrs, children),
            NodeKind::FootnoteDefinition {
                identifier,
                children,
            } => self.write_footnote_definition(node, identifier, children),
            NodeKind::Text { value } => {
                self.push_text(value);
                Ok(())
            }
            NodeKind::Emphasis { children } => self.block(BlockTag::Em, &node.attrs, children),
            NodeKind::Strong { children } => self.block(BlockTag::Strong, &node.attrs, children),
            NodeKind::Delete { children } => self.block(BlockTag::S, &node.attrs, children),
            NodeKind::Span { children } => self.block(BlockTag::Span, &node.attrs, children),
            NodeKind::InlineCode { value } => {
                self.write_inline_code(node, value);
                Ok(())
            }
            NodeKind::Link {
                url,
                title,
                children,
            } => self.write_link(node, url, title.as_deref(), children),
            NodeKind::Image { url, title, alt } => {
                self.write_image(node, url, title.as_deref(), alt);
                Ok(())
            }
            NodeKind::FootnoteReference { identifier } => {
                self.write_footnote_reference(node, identifier);
                Ok(())
            }
            NodeKind::SoftBreak => {
                self.buf.push(' ');
                Ok(())
            }
            NodeKind::HardBreak => {
                self.void(VoidTag::Br, &node.attrs);
                Ok(())
            }
            NodeKind::Html { value, block } => self.write_html(node, value, *block),
            NodeKind::Extended {
                token, children, ..
            } => self.write_extended(node, token, children),
            NodeKind::Unsupported { label } => self.write_unsupported(node, label),
            NodeKind::Disclosure { summary, children, .. } => {
                self.write_disclosure(node, summary, children)
            }
        }
    }

    /// Streams a disclosure block as native `\u003cdetails\u003e`/\u003csummary\u003e` HTML.
    fn write_disclosure(
        &mut self,
        node: &RenderNode,
        summary: &[RenderNode],
        children: &[RenderNode],
    ) -> Result<(), RenderError> {
        let inline = is_inline_block_tag(&BlockTag::Details);
        self.open_block(&BlockTag::Details, &node_attributes(&node.attrs, inline));

        self.open_block(&BlockTag::Summary, &[]);
        for child in summary {
            self.write(child)?;
        }
        self.close_block(&BlockTag::Summary);

        for child in children {
            self.write(child)?;
        }

        self.close_block(&BlockTag::Details);
        Ok(())
    }

    /// Streams an open tag, child subtrees, and a close tag for a block
    /// element — the streaming analogue of [`Writer::block`].
    fn block(
        &mut self,
        tag: BlockTag,
        attrs: &NodeAttrs,
        children: &[RenderNode],
    ) -> Result<(), RenderError> {
        let inline = is_inline_block_tag(&tag);
        self.open_block(&tag, &node_attributes(attrs, inline));
        for child in children {
            self.write(child)?;
        }
        self.close_block(&tag);
        Ok(())
    }

    /// Writes `<{tag}{attributes}>` for a block element. The empty `base_class`
    /// matches the fragment writer's `define_as_block_tag(tag, "")`.
    fn open_block(&mut self, tag: &BlockTag, attributes: &[HtmlAttribute]) {
        self.buf.push('<');
        self.buf.push_str(tag.name());
        write_attributes(&mut self.buf, attributes, Some(""));
        self.buf.push('>');
    }

    /// Writes `</{tag}>`.
    fn close_block(&mut self, tag: &BlockTag) {
        self.buf.push_str("</");
        self.buf.push_str(tag.name());
        self.buf.push('>');
    }

    /// Writes `<{tag}{attributes}>` for a void element (no `base_class`).
    fn void(&mut self, tag: VoidTag, attrs: &NodeAttrs) {
        let inline = is_inline_void_tag(&tag);
        self.write_void_tag(&tag, &node_attributes(attrs, inline));
    }

    /// Writes a void element from an explicit attribute list.
    fn write_void_tag(&mut self, tag: &VoidTag, attributes: &[HtmlAttribute]) {
        self.buf.push('<');
        self.buf.push_str(tag.name());
        write_attributes(&mut self.buf, attributes, None);
        self.buf.push('>');
    }

    /// Appends HTML-escaped text — the streaming analogue of [`text_fragment`].
    fn push_text(&mut self, value: &str) {
        self.buf
            .push_str(&crate::browser::utils::escape_text(value));
    }

    /// Serializes a hook-returned fragment into the buffer and retains it for
    /// the `<head>` rollup. See [`StreamWriter::hook_fragments`].
    ///
    /// A hook fragment may itself request features; merge them at this document
    /// position so the streaming path collects nested/hook features the same
    /// way the fragment path's page rollup does.
    fn push_hook_fragment(&mut self, fragment: BrowserFragment<Ready>) {
        self.features.extend(fragment.collect_features());
        self.buf.push_str(&fragment.render());
        self.hook_fragments.push(fragment);
    }

    /// Streaming analogue of [`Writer::render_section`].
    fn write_section(
        &mut self,
        node: &RenderNode,
        depth: HeadingDepth,
        heading: &[RenderNode],
        children: &[RenderNode],
    ) -> Result<(), RenderError> {
        self.open_block(&BlockTag::Section, &node_attributes(&node.attrs, false));
        // The heading element carries no attributes (matching the fragment
        // writer, which builds it with an empty attribute list).
        let heading_tag = heading_tag(depth);
        self.open_block(&heading_tag, &[]);
        for child in heading {
            self.write(child)?;
        }
        self.close_block(&heading_tag);
        for child in children {
            self.write(child)?;
        }
        self.close_block(&BlockTag::Section);
        Ok(())
    }

    /// Streaming analogue of [`Writer::render_progress`].
    fn write_progress(
        &mut self,
        node: &RenderNode,
        hints: &crate::tree::ProgressHints,
        children: &[RenderNode],
    ) {
        let fallback_text = plain_text(children);
        let layout_css = node
            .attrs
            .layout_ref()
            .map(layout_to_css)
            .filter(|css| !css.is_empty())
            .unwrap_or_default();
        self.buf
            .push_str(&super::shared::progress_html(hints, &fallback_text, &layout_css));
    }

    /// Streaming analogue of [`Writer::render_columns`].
    fn write_columns(
        &mut self,
        node: &RenderNode,
        hints: &crate::tree::ColumnsHints,
        children: &[RenderNode],
    ) -> Result<(), RenderError> {
        let split = hints.left_count.min(children.len());
        let (left, right) = children.split_at(split);

        // Build the container's attribute list in the same order the fragment
        // writer pushes it, then serialize it byte-identically.
        let mut container_attrs: Vec<HtmlAttribute> = Vec::new();
        let mut class_emitted = false;
        let mut style_emitted = false;
        for attr in node_attributes(&node.attrs, false) {
            match attr {
                HtmlAttribute::Class(class) => {
                    container_attrs.push(HtmlAttribute::Class(ClassDefinition::new(format!(
                        "columns {}",
                        class.as_str()
                    ))));
                    class_emitted = true;
                }
                HtmlAttribute::Other(key, value) if key == "style" => {
                    container_attrs.push(HtmlAttribute::Other(
                        "style".into(),
                        super::shared::columns_container_css(hints, &value),
                    ));
                    style_emitted = true;
                }
                other => container_attrs.push(other),
            }
        }
        if !class_emitted {
            container_attrs.push(HtmlAttribute::Class(ClassDefinition::new("columns")));
        }
        if !style_emitted {
            container_attrs.push(HtmlAttribute::Other(
                "style".into(),
                super::shared::columns_container_css(hints, ""),
            ));
        }
        self.open_block(&BlockTag::Div, &container_attrs);

        for (group, column_css) in [
            (left, super::shared::left_column_css(hints.left_width)),
            (right, super::shared::right_column_css().to_string()),
        ] {
            self.open_block(
                &BlockTag::Div,
                &[
                    HtmlAttribute::Class(ClassDefinition::new("column")),
                    HtmlAttribute::Other("style".into(), column_css),
                ],
            );
            for child in group {
                self.write(child)?;
            }
            self.close_block(&BlockTag::Div);
        }
        self.close_block(&BlockTag::Div);
        Ok(())
    }

    /// Streaming analogue of [`Writer::render_list`].
    fn write_list(
        &mut self,
        node: &RenderNode,
        ordered: bool,
        start: Option<u64>,
        children: &[RenderNode],
    ) -> Result<(), RenderError> {
        let tag = if ordered { BlockTag::Ol } else { BlockTag::Ul };
        let marker_css = match node.attrs.list_marker_policy() {
            crate::tree::ListMarkerPolicy::Default => None,
            crate::tree::ListMarkerPolicy::None => Some("list-style:none"),
            crate::tree::ListMarkerPolicy::TreeConnectors => {
                let message = "list marker policy 'tree_connectors' has no \
                               browser equivalent; degraded to a plain \
                               no-marker list"
                    .to_string();
                match self.opts.strictness {
                    RenderStrictness::Strict => {
                        return Err(RenderError::LossyRejected { message });
                    }
                    RenderStrictness::Warn => {
                        self.diagnostics
                            .push(Diagnostic::lossy(message, Some(node.span.clone())));
                    }
                    RenderStrictness::Lossy => {}
                }
                Some("list-style:none")
            }
        };

        let mut attrs: Vec<HtmlAttribute> = Vec::new();
        let base = node_attributes(&node.attrs, false);
        let has_style = base
            .iter()
            .any(|a| matches!(a, HtmlAttribute::Other(k, _) if k == "style"));
        for attr in base {
            match (&attr, marker_css) {
                (HtmlAttribute::Other(key, value), Some(extra)) if key == "style" => {
                    attrs.push(HtmlAttribute::Other(
                        "style".into(),
                        format!("{value};{extra}"),
                    ));
                }
                _ => attrs.push(attr),
            }
        }
        if let Some(extra) = marker_css.filter(|_| !has_style) {
            attrs.push(HtmlAttribute::Other("style".into(), extra.to_string()));
        }
        if ordered && let Some(start) = start.filter(|s| *s != 1) {
            attrs.push(HtmlAttribute::Other("start".into(), start.to_string()));
        }

        self.open_block(&tag, &attrs);
        for child in children {
            self.write(child)?;
        }
        self.close_block(&tag);
        Ok(())
    }

    /// Streaming analogue of [`Writer::render_list_item`].
    fn write_list_item(
        &mut self,
        node: &RenderNode,
        checked: Option<bool>,
        children: &[RenderNode],
    ) -> Result<(), RenderError> {
        let mut attrs = node.attrs.clone();
        if checked.is_some() {
            attrs.classes.push("task-list-item".to_string());
        }
        self.open_block(&BlockTag::Li, &node_attributes(&attrs, false));
        if let Some(checked) = checked {
            self.write_checkbox(checked);
        }
        for child in children {
            self.write(child)?;
        }
        self.close_block(&BlockTag::Li);
        Ok(())
    }

    /// Writes a disabled `<input type=checkbox>` — the streaming analogue of
    /// [`checkbox`].
    fn write_checkbox(&mut self, checked: bool) {
        self.write_void_tag(
            &VoidTag::Input,
            &[
                HtmlAttribute::Type(HtmlType::Checkbox),
                HtmlAttribute::Disabled(true),
                HtmlAttribute::Checked(checked),
            ],
        );
    }

    /// Streaming analogue of [`Writer::render_code_block`].
    fn write_code_block(
        &mut self,
        node: &RenderNode,
        lang: Option<&str>,
        meta: Option<&str>,
        value: &str,
    ) -> Result<(), RenderError> {
        use crate::tree::{BrowserMermaidMode, GraphicsMode};

        let is_mermaid = lang
            .map(|l| l.eq_ignore_ascii_case("mermaid"))
            .unwrap_or(false);

        if is_mermaid {
            if self.opts.graphics_mode == GraphicsMode::Off {
                self.write_code_fallback(node, lang, meta, value);
                return Ok(());
            }
            let mermaid_mode = match (self.opts.graphics_mode, self.opts.mermaid_mode) {
                (GraphicsMode::Vector, BrowserMermaidMode::Interactive) => {
                    BrowserMermaidMode::StaticSvg
                }
                (_, mode) => mode,
            };
            match mermaid_mode {
                BrowserMermaidMode::Interactive => self.write_mermaid_interactive(node, value),
                BrowserMermaidMode::StaticSvg => {
                    if let Some(renderer) = &self.opts.code_renderer
                        && let Some(fragment) =
                            renderer.render_browser_mermaid(value, meta, &node.attrs)
                    {
                        self.push_hook_fragment(fragment);
                        return Ok(());
                    }
                    match self.opts.strictness {
                        RenderStrictness::Strict => {
                            return Err(RenderError::LossyRejected {
                                message: "Mermaid static SVG promotion failed".to_string(),
                            });
                        }
                        RenderStrictness::Warn => {
                            self.note_lossy(
                                "Mermaid static SVG promotion failed; rendered as code block",
                                node,
                            );
                            self.write_code_fallback(node, lang, meta, value);
                        }
                        RenderStrictness::Lossy => {
                            self.write_code_fallback(node, lang, meta, value);
                        }
                    }
                }
                BrowserMermaidMode::Code => self.write_code_fallback(node, lang, meta, value),
            }
            return Ok(());
        }

        self.write_code_fallback(node, lang, meta, value);
        Ok(())
    }

    /// Streaming analogue of [`Writer::render_code_fallback`].
    fn write_code_fallback(
        &mut self,
        node: &RenderNode,
        lang: Option<&str>,
        meta: Option<&str>,
        value: &str,
    ) {
        if let Some(renderer) = &self.opts.code_renderer
            && let Some(fragment) = renderer.render_browser_code(lang, value, meta, &node.attrs)
        {
            self.push_hook_fragment(fragment);
            return;
        }
        self.write_plain_code_block(node, lang, value);
    }

    /// Streaming analogue of [`Writer::render_mermaid_interactive`].
    fn write_mermaid_interactive(&mut self, node: &RenderNode, value: &str) {
        let mut attrs: Vec<HtmlAttribute> =
            vec![HtmlAttribute::Class(ClassDefinition::new("mermaid"))];
        attrs.extend(node_attributes(&node.attrs, false));
        // Request the Mermaid feature at the same semantic branch as the
        // fragment writer, in document position (deduped at resolution).
        self.features.push(PageFeature::MermaidDiagram);
        self.open_block(&BlockTag::Pre, &attrs);
        self.push_text(value);
        self.close_block(&BlockTag::Pre);
    }

    /// Streaming analogue of [`Writer::render_plain_code_block`].
    fn write_plain_code_block(&mut self, node: &RenderNode, lang: Option<&str>, value: &str) {
        self.open_block(&BlockTag::Pre, &node_attributes(&node.attrs, false));
        let code_attrs: Vec<HtmlAttribute> = match lang.filter(|lang| !lang.is_empty()) {
            Some(lang) => vec![HtmlAttribute::Class(ClassDefinition::new(format!(
                "language-{lang}"
            )))],
            None => Vec::new(),
        };
        self.open_block(&BlockTag::Code, &code_attrs);
        self.push_text(value);
        self.close_block(&BlockTag::Code);
        self.close_block(&BlockTag::Pre);
    }

    /// Streaming analogue of [`Writer::render_inline_code`].
    fn write_inline_code(&mut self, node: &RenderNode, value: &str) {
        self.open_block(&BlockTag::Code, &node_attributes(&node.attrs, true));
        self.push_text(value);
        self.close_block(&BlockTag::Code);
    }

    /// Streaming analogue of [`Writer::render_table`].
    fn write_table(
        &mut self,
        node: &RenderNode,
        align: &[ColumnAlign],
        children: &[RenderNode],
    ) -> Result<(), RenderError> {
        self.open_block(&BlockTag::Table, &node_attributes(&node.attrs, false));

        if let Some(title) = node.attrs.table_title_ref()
            && !title.trim().is_empty()
        {
            self.open_block(&BlockTag::Caption, &[]);
            self.push_text(title.trim());
            self.close_block(&BlockTag::Caption);
        }

        let mut rows = children.iter();
        if let Some(header) = rows.next() {
            self.open_block(&BlockTag::Thead, &[]);
            self.write_table_row(header, align, true)?;
            self.close_block(&BlockTag::Thead);
        }

        self.open_block(&BlockTag::Tbody, &[]);
        for row in rows {
            self.write_table_row(row, align, false)?;
        }
        self.close_block(&BlockTag::Tbody);
        self.close_block(&BlockTag::Table);
        Ok(())
    }

    /// Streaming analogue of [`Writer::render_table_row`].
    fn write_table_row(
        &mut self,
        row: &RenderNode,
        align: &[ColumnAlign],
        header: bool,
    ) -> Result<(), RenderError> {
        let cells = match &row.kind {
            NodeKind::TableRow { children } => children.as_slice(),
            _ => &[],
        };
        self.open_block(&BlockTag::Tr, &node_attributes(&row.attrs, false));
        for (index, cell) in cells.iter().enumerate() {
            self.write_table_cell(
                cell,
                align.get(index).copied().unwrap_or(ColumnAlign::None),
                header,
            )?;
        }
        self.close_block(&BlockTag::Tr);
        Ok(())
    }

    /// Streaming analogue of [`Writer::render_table_cell`].
    fn write_table_cell(
        &mut self,
        cell: &RenderNode,
        align: ColumnAlign,
        header: bool,
    ) -> Result<(), RenderError> {
        let children = match &cell.kind {
            NodeKind::TableCell { children } => children.as_slice(),
            _ => &[],
        };
        let tag = if header { BlockTag::Th } else { BlockTag::Td };
        let align_css = align_value(align).map(|value| format!("text-align:{value}"));
        let mut attrs: Vec<HtmlAttribute> = Vec::new();
        let mut style_emitted = false;
        for attr in node_attributes(&cell.attrs, false) {
            match (&attr, &align_css) {
                (HtmlAttribute::Other(key, value), Some(extra)) if key == "style" => {
                    attrs.push(HtmlAttribute::Other(
                        "style".into(),
                        format!("{value};{extra}"),
                    ));
                    style_emitted = true;
                }
                _ => attrs.push(attr),
            }
        }
        if let Some(extra) = align_css.filter(|_| !style_emitted) {
            attrs.push(HtmlAttribute::Other("style".into(), extra));
        }
        self.open_block(&tag, &attrs);
        for child in children {
            self.write(child)?;
        }
        self.close_block(&tag);
        Ok(())
    }

    /// Streaming analogue of [`Writer::render_footnote_definition`].
    fn write_footnote_definition(
        &mut self,
        node: &RenderNode,
        identifier: &str,
        children: &[RenderNode],
    ) -> Result<(), RenderError> {
        let mut attrs: Vec<HtmlAttribute> = vec![
            HtmlAttribute::Id(DomId::new(format!("fn-{identifier}"))),
            HtmlAttribute::Class(ClassDefinition::new("footnote-definition")),
        ];
        attrs.extend(node_attributes(&node.attrs, false));
        self.open_block(&BlockTag::Div, &attrs);
        for child in children {
            self.write(child)?;
        }
        self.close_block(&BlockTag::Div);
        Ok(())
    }

    /// Streaming analogue of [`Writer::render_footnote_reference`].
    fn write_footnote_reference(&mut self, node: &RenderNode, identifier: &str) {
        let mut attrs = node_attributes(&node.attrs, true);
        attrs.push(HtmlAttribute::Other(
            "href".into(),
            format!("#fn-{identifier}"),
        ));
        self.open_block(&BlockTag::A, &attrs);
        self.push_text(identifier);
        self.close_block(&BlockTag::A);
    }

    /// Streaming analogue of [`Writer::render_link`].
    fn write_link(
        &mut self,
        node: &RenderNode,
        url: &str,
        title: Option<&str>,
        children: &[RenderNode],
    ) -> Result<(), RenderError> {
        if let Some(prompt) = link_prompt(&node.attrs) {
            let prompt = prompt.to_string();
            return self.write_prompted_link(node, url, title, children, &prompt);
        }

        let mut attrs = node_attributes(&node.attrs, true);
        attrs.push(HtmlAttribute::Other("href".into(), url.to_string()));
        if let Some(title) = title {
            attrs.push(HtmlAttribute::Title(title.to_string()));
        }
        self.open_block(&BlockTag::A, &attrs);
        for child in children {
            self.write(child)?;
        }
        self.close_block(&BlockTag::A);
        Ok(())
    }

    /// Streaming analogue of [`Writer::render_prompted_link`]; emits
    /// byte-identical wrapper/anchor/prompt markup and requests
    /// [`PageFeature::Popover`] in document position.
    fn write_prompted_link(
        &mut self,
        node: &RenderNode,
        url: &str,
        title: Option<&str>,
        children: &[RenderNode],
        prompt: &str,
    ) -> Result<(), RenderError> {
        let id = self.popover_ids.allocate(&popover_id_base(url));
        self.features.push(PageFeature::Popover);

        self.open_block(
            &BlockTag::Span,
            &[HtmlAttribute::Class(ClassDefinition::new(
                "dm-popover-wrapper",
            ))],
        );

        self.open_block(&BlockTag::A, &prompted_anchor_attributes(node, url, title, &id));
        for child in children {
            self.write(child)?;
        }
        self.close_block(&BlockTag::A);

        self.open_block(&BlockTag::Span, &prompted_prompt_attributes(&id));
        self.push_text(prompt);
        self.close_block(&BlockTag::Span);

        self.close_block(&BlockTag::Span);
        Ok(())
    }

    /// Streaming analogue of [`Writer::render_image`].
    fn write_image(&mut self, node: &RenderNode, url: &str, title: Option<&str>, alt: &str) {
        let mut attrs = node_attributes(&node.attrs, true);
        attrs.push(HtmlAttribute::Other("src".into(), url.to_string()));
        attrs.push(HtmlAttribute::Alt(alt.to_string()));
        if let Some(title) = title {
            attrs.push(HtmlAttribute::Title(title.to_string()));
        }
        self.write_void_tag(&VoidTag::Img, &attrs);
    }

    /// Streaming analogue of [`Writer::render_thematic_break`].
    fn write_thematic_break(&mut self, attrs: &NodeAttrs) {
        use crate::tree::GraphicsMode;

        let hr = attrs.thematic_break_ref();
        match self.opts.graphics_mode {
            GraphicsMode::Off => {
                let mut out = node_attributes(attrs, is_inline_void_tag(&VoidTag::Hr));
                for (key, value) in hr_data_attr_pairs(hr) {
                    out.push(HtmlAttribute::Data(
                        HtmlDataAttribute::new(format!("hr-{key}")),
                        value.to_string(),
                    ));
                }
                self.write_void_tag(&VoidTag::Hr, &out);
            }
            GraphicsMode::Vector | GraphicsMode::Rich => {
                let svg = crate::tree::graphics::horizontal_rule_svg(
                    hr.and_then(|h| h.kind),
                    hr.and_then(|h| h.weight),
                    hr.and_then(|h| h.alignment),
                    hr.and_then(|h| h.width.as_deref()),
                    hr.and_then(|h| h.color.as_deref()),
                    "0",
                    "0",
                );
                self.buf.push_str(&svg);
            }
        }
    }

    /// Streaming analogue of [`Writer::render_html`].
    fn write_html(
        &mut self,
        node: &RenderNode,
        value: &str,
        _block: bool,
    ) -> Result<(), RenderError> {
        match self.opts.raw_html {
            RawHtmlPolicy::Allow => self.buf.push_str(value),
            RawHtmlPolicy::Escape => {
                self.note_lossy("raw HTML emitted as escaped text", node);
                self.push_text(value);
            }
            RawHtmlPolicy::Reject => match self.opts.strictness {
                RenderStrictness::Strict => {
                    return Err(RenderError::LossyRejected {
                        message: "raw HTML rejected by RawHtmlPolicy::Reject".to_string(),
                    });
                }
                RenderStrictness::Warn | RenderStrictness::Lossy => {
                    self.note_lossy("raw HTML rejected and emitted as escaped text", node);
                    self.push_text(value);
                }
            },
        }
        Ok(())
    }

    /// Streaming analogue of [`Writer::render_extended`].
    fn write_extended(
        &mut self,
        node: &RenderNode,
        token: &str,
        children: &[RenderNode],
    ) -> Result<(), RenderError> {
        match token {
            "mark" => self.block(BlockTag::Mark, &node.attrs, children),
            "dim" => self.block_with_extra_style(BlockTag::Span, &node.attrs, "opacity:0.6", children),
            _ => {
                let mut attrs = node.attrs.clone();
                attrs.classes.push(format!("extended-{token}"));
                self.block(BlockTag::Span, &attrs, children)
            }
        }
    }

    /// Streaming analogue of [`Writer::block_with_extra_style`].
    fn block_with_extra_style(
        &mut self,
        tag: BlockTag,
        attrs: &NodeAttrs,
        extra_css: &str,
        children: &[RenderNode],
    ) -> Result<(), RenderError> {
        let inline = is_inline_block_tag(&tag);
        let mut out: Vec<HtmlAttribute> = Vec::new();
        let mut style_emitted = false;
        for attr in node_attributes(attrs, inline) {
            match attr {
                HtmlAttribute::Other(key, value) if key == "style" => {
                    out.push(HtmlAttribute::Other(
                        "style".into(),
                        format!("{value};{extra_css}"),
                    ));
                    style_emitted = true;
                }
                other => out.push(other),
            }
        }
        if !style_emitted {
            out.push(HtmlAttribute::Other("style".into(), extra_css.to_string()));
        }
        self.open_block(&tag, &out);
        for child in children {
            self.write(child)?;
        }
        self.close_block(&tag);
        Ok(())
    }

    /// Streaming analogue of [`Writer::render_unsupported`].
    fn write_unsupported(&mut self, node: &RenderNode, label: &str) -> Result<(), RenderError> {
        match self.opts.strictness {
            RenderStrictness::Strict => Err(RenderError::Unsupported {
                label: label.to_string(),
            }),
            RenderStrictness::Warn => {
                self.diagnostics.push(Diagnostic::unsupported(
                    format!("unsupported content dropped: {label}"),
                    Some(node.span.clone()),
                ));
                self.buf.push_str(&format!("<!-- unsupported: {label} -->"));
                Ok(())
            }
            // Lossy emits nothing (the fragment writer emits an empty text node).
            RenderStrictness::Lossy => Ok(()),
        }
    }

    /// Records a lossy diagnostic, unless strictness is [`RenderStrictness::Lossy`]
    /// — the streaming analogue of [`Writer::note_lossy`].
    fn note_lossy(&mut self, message: &str, node: &RenderNode) {
        if self.opts.strictness != RenderStrictness::Lossy {
            self.diagnostics.push(Diagnostic::lossy(
                message.to_string(),
                Some(node.span.clone()),
            ));
        }
    }
}

/// Returns the concatenated text content of the first `<h1>` the browser
/// renderer would emit for `root`, or `None` when the tree produces no `<h1>`.
///
/// This mirrors [`HtmlPage::first_h1_text`] on the equivalent fragment tree so
/// [`render_browser_document_html`] can supply the same `<title>` fallback
/// without building fragments. An `<h1>` is produced by a depth-1
/// [`NodeKind::Heading`] or by a depth-1 [`NodeKind::Section`]'s heading; the
/// scan is document-order and descends container children.
/// Whether the document root carries no renderable [`Style`](crate::style::Style).
///
/// When `true` the document folds emit the root's children directly as page
/// fragments; when `false` the root is rendered as its own wrapping `<div>` so
/// its style is emitted and inherits to every descendant.
fn root_style_is_empty(root: &RenderNode) -> bool {
    root.attrs.style_ref().is_none_or(|s| s.is_empty())
}

fn tree_first_h1_text(root: &RenderNode) -> Option<String> {
    match &root.kind {
        NodeKind::Heading { depth, children } if depth.get() == 1 => {
            let mut out = String::new();
            for child in children {
                collect_h1_text(child, &mut out);
            }
            Some(out)
        }
        NodeKind::Section {
            depth,
            heading,
            children,
        } => {
            if depth.get() == 1 {
                let mut out = String::new();
                for child in heading {
                    collect_h1_text(child, &mut out);
                }
                return Some(out);
            }
            children.iter().find_map(tree_first_h1_text)
        }
        _ => root.children().iter().find_map(tree_first_h1_text),
    }
}

/// Collects the text content of an inline subtree the way
/// [`crate::html::find_first_h1_text`]'s `collect_text` does over the rendered
/// fragment tree: text and inline-code values contribute, a soft break is a
/// space, void elements (images, hard breaks) and raw HTML contribute nothing,
/// and every other inline container descends into its children.
fn collect_h1_text(node: &RenderNode, out: &mut String) {
    match &node.kind {
        NodeKind::Text { value } | NodeKind::InlineCode { value } => out.push_str(value),
        NodeKind::FootnoteReference { identifier } => out.push_str(identifier),
        NodeKind::SoftBreak => out.push(' '),
        // Void / raw / leaf elements emit no text node to collect.
        NodeKind::Image { .. }
        | NodeKind::HardBreak
        | NodeKind::Html { .. }
        | NodeKind::ThematicBreak
        | NodeKind::Code { .. }
        | NodeKind::Unsupported { .. } => {}
        _ => {
            for child in node.children() {
                collect_h1_text(child, out);
            }
        }
    }
}

/// Maps a heading depth `1..=6` to its `<h1>`..`<h6>` block tag.
fn heading_tag(depth: HeadingDepth) -> BlockTag {
    match depth.get() {
        1 => BlockTag::H1,
        2 => BlockTag::H2,
        3 => BlockTag::H3,
        4 => BlockTag::H4,
        5 => BlockTag::H5,
        // `HeadingDepth` is constrained to `1..=6`; `6` is the only
        // remaining value.
        _ => BlockTag::H6,
    }
}

/// The `text-align` value for a column alignment, or `None` for
/// [`ColumnAlign::None`].
fn align_value(align: ColumnAlign) -> Option<&'static str> {
    match align {
        ColumnAlign::Left => Some("left"),
        ColumnAlign::Center => Some("center"),
        ColumnAlign::Right => Some("right"),
        ColumnAlign::None => None,
    }
}

/// Returns `true` for [`NodeKind`] variants that are inline (phrasing-level).
///
/// Inline nodes have their bold / italic / strikethrough `Style` emphasis
/// lowered to semantic element wrappers; block nodes lower the same emphasis
/// to CSS, because wrapping a block element in `<strong>`/`<em>`/`<s>` is
/// invalid HTML.
fn is_inline_node_kind(kind: &NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Text { .. }
            | NodeKind::Emphasis { .. }
            | NodeKind::Strong { .. }
            | NodeKind::Delete { .. }
            | NodeKind::Span { .. }
            | NodeKind::InlineCode { .. }
            | NodeKind::Link { .. }
            | NodeKind::Image { .. }
            | NodeKind::FootnoteReference { .. }
            | NodeKind::SoftBreak
            | NodeKind::HardBreak
    )
}

/// Collects the plain text content of a phrasing subtree.
///
/// Used to recover a `Progress` widget's accessible label from its paragraph
/// children without HTML-escaping noise — the collected text is escaped
/// exactly once, by `progress_html`.
fn plain_text(nodes: &[RenderNode]) -> String {
    fn collect(node: &RenderNode, out: &mut String) {
        match &node.kind {
            NodeKind::Text { value } | NodeKind::InlineCode { value } => out.push_str(value),
            NodeKind::Image { alt, .. } => out.push_str(alt),
            NodeKind::SoftBreak | NodeKind::HardBreak => out.push(' '),
            _ => {
                for child in node.children() {
                    collect(child, out);
                }
            }
        }
    }
    let mut out = String::new();
    for node in nodes {
        collect(node, &mut out);
    }
    out
}

/// Returns `true` for [`BlockTag`] variants that represent inline elements.
fn is_inline_block_tag(tag: &BlockTag) -> bool {
    matches!(
        tag,
        BlockTag::Em
            | BlockTag::Strong
            | BlockTag::S
            | BlockTag::Span
            | BlockTag::Code
            | BlockTag::A
            | BlockTag::Mark
    )
}

/// Returns `true` for [`VoidTag`] variants that represent inline elements.
fn is_inline_void_tag(tag: &VoidTag) -> bool {
    matches!(tag, VoidTag::Br | VoidTag::Img)
}

/// Collects the set `(suffix, value)` pairs for a thematic break's `data-hr-*`
/// degradation attributes, in a fixed key order shared by the fragment and
/// streaming HR paths so both emit byte-identical `<hr>` output.
fn hr_data_attr_pairs(
    hr: Option<&crate::tree::ThematicBreakAttrs>,
) -> Vec<(&'static str, &str)> {
    let Some(hr) = hr else {
        return Vec::new();
    };
    [
        ("kind", hr.kind.map(HrKind::as_str)),
        ("alignment", hr.alignment.map(HrAlignment::as_str)),
        ("weight", hr.weight.map(HrWeight::as_str)),
        ("width", hr.width.as_deref()),
        ("color", hr.color.as_deref()),
    ]
    .into_iter()
    .filter_map(|(key, value)| value.map(|v| (key, v)))
    .collect()
}

/// Translates a node's [`NodeAttrs`] into HTML attributes: `id` to the `id`
/// attribute, `classes` to a `class` attribute, and a stored
/// [`Layout`](crate::layout::Layout) plus the CSS-bearing layers of a stored
/// [`Style`](crate::style::Style) to an inline `style` attribute.
///
/// Layout is skipped for inline nodes; the validation gate rejects a layout on
/// an inline node as an error, and the renderer drops it per D5.
/// `Style` is applied to both block nodes and inline `Span` nodes. The
/// layout and style declarations share a single `style` attribute and never
/// overwrite each other.
fn node_attributes(attrs: &NodeAttrs, inline: bool) -> Vec<HtmlAttribute> {
    let mut out = Vec::new();
    if let Some(id) = &attrs.id {
        out.push(HtmlAttribute::Id(DomId::new(id.clone())));
    }
    if !attrs.classes.is_empty() {
        out.push(HtmlAttribute::Class(ClassDefinition::new(
            attrs.classes.join(" "),
        )));
    }

    let mut decls: Vec<String> = Vec::new();
    // The renderable width contract is content-box: `width` is the content box
    // and `padding` / `border` are added around it. A page stylesheet may ship a
    // global `* { box-sizing: border-box }` reset, which would silently
    // reinterpret the lowered `width` as a border-box width. Emitting
    // `box-sizing:content-box` on every node that lowers a non-default width,
    // padding, or border keeps the contract under any such reset.
    let mut needs_content_box = false;
    if !inline && let Some(layout) = attrs.layout_ref() {
        let css = layout_to_css(layout);
        if !css.is_empty() {
            needs_content_box |= layout_lowers_box(layout);
            decls.push(css);
        }
    }
    // `Style` text-appearance/box-color CSS rides on both block and inline
    // nodes. Bold / italic / strikethrough emphasis lowers to CSS here for
    // block nodes; for inline nodes it is applied as semantic wrappers by
    // `wrap_style_emphasis` instead, so the CSS form is suppressed.
    if let Some(style) = attrs.style_ref().filter(|s| !s.is_empty()) {
        let css = style_css_declarations(style, !inline);
        if !css.is_empty() {
            needs_content_box |= style_draws_border(style);
            decls.push(css);
        }
    }
    if needs_content_box {
        // Prepend so the contract is set before the width/padding it governs.
        decls.insert(0, "box-sizing:content-box".into());
    }
    // Validated `inline_style` (browser-only author CSS, already merged over
    // frontmatter defaults upstream) is the author's per-node intent: it
    // *replaces* the derived `Layout` / `Style` declaration for any property it
    // sets, then its remaining declarations append last. Replacing — rather than
    // appending a shadowing duplicate — keeps the overridden frontmatter value
    // out of the attribute entirely (the spec's per-node-wins contract). Derived
    // declarations whose property `inline_style` does not set are untouched, so
    // intentional internal pairs (e.g. an explicit margin plus an `auto`
    // centering margin) are preserved. It is a validated `CssStyle`, so no
    // unparsed CSS can be injected here.
    let inline_decls: Vec<String> = attrs
        .browser_ref()
        .and_then(|browser| browser.inline_style.as_ref())
        .map(css_style_declarations)
        .unwrap_or_default();
    if !inline_decls.is_empty() {
        let overridden: std::collections::HashSet<String> =
            inline_decls.iter().map(|d| css_property_of(d)).collect();
        let mut merged: Vec<String> = decls
            .into_iter()
            .flat_map(|group| {
                group
                    .split(';')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|decl| !overridden.contains(&css_property_of(decl)))
            .collect();
        merged.extend(inline_decls);
        decls = merged;
    }
    if !decls.is_empty() {
        out.push(HtmlAttribute::Other("style".into(), decls.join(";")));
    }
    push_browser_attributes(&mut out, attrs);
    out
}

/// The property name of a `name:value` CSS declaration (the text before the
/// first `:`, trimmed). Used to let a validated `inline_style` override the
/// derived `Layout` / `Style` declaration for the same property.
fn css_property_of(decl: &str) -> String {
    decl.split(':').next().unwrap_or("").trim().to_string()
}

/// Lowers a validated [`CssStyle`](crate::stylesheet::CssStyle) to compact
/// `name:value` declaration strings matching the inline-`style` form the
/// browser renderer emits for `Layout` / `Style`.
///
/// [`CssStyle::to_css`](crate::stylesheet::CssStyle::to_css) produces the
/// human-spaced `name: value;` form (one declaration per line); this collapses
/// each line to `name:value` so it merges cleanly into the joined `style`
/// attribute.
fn css_style_declarations(style: &crate::stylesheet::CssStyle) -> Vec<String> {
    style
        .to_css()
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim().trim_end_matches(';');
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.replacen(": ", ":", 1))
            }
        })
        .collect()
}

/// The Darkmatter-lowered prompt carried by a link node, if any.
///
/// The prompt is transported as a browser `data-prompt` attribute. Its presence
/// promotes the link to the accessible popover structure and makes it request
/// [`PageFeature::Popover`]; the value becomes the escaped prompt content.
fn link_prompt(attrs: &NodeAttrs) -> Option<&str> {
    attrs.browser_ref().and_then(|browser| {
        browser
            .data_attrs
            .iter()
            .find(|(name, _)| name.as_str() == "prompt")
            .map(|(_, value)| value.as_str())
    })
}

/// Builds the id-independent anchor attributes for a prompted link.
///
/// Every existing node attribute is preserved except the internal `data-prompt`
/// transport, which is consumed into the popover markup and never emitted, then
/// the real `href` and optional `title`. The id-dependent `interestfor` /
/// `aria-describedby` are appended separately once the popover id is known —
/// eagerly by [`prompted_anchor_attributes`] (streaming path) or at render by
/// [`PopoverNode`] (fragment path) — so both browser paths stay byte-identical.
fn prompted_anchor_base_attributes(
    node: &RenderNode,
    url: &str,
    title: Option<&str>,
) -> Vec<HtmlAttribute> {
    let mut attrs: Vec<HtmlAttribute> = node_attributes(&node.attrs, true)
        .into_iter()
        .filter(|attr| !matches!(attr, HtmlAttribute::Other(key, _) if key == "data-prompt"))
        .collect();
    attrs.push(HtmlAttribute::Other("href".into(), url.to_string()));
    if let Some(title) = title {
        attrs.push(HtmlAttribute::Title(title.to_string()));
    }
    attrs
}

/// Builds the full anchor attribute list for a prompted link, appending the
/// `id`-dependent association attributes to
/// [`prompted_anchor_base_attributes`]. Used by the streaming writer, which
/// allocates the id eagerly.
fn prompted_anchor_attributes(
    node: &RenderNode,
    url: &str,
    title: Option<&str>,
    id: &str,
) -> Vec<HtmlAttribute> {
    let mut attrs = prompted_anchor_base_attributes(node, url, title);
    // `interestfor` is the progressive-enhancement invoker where supported;
    // `aria-describedby` is the always-on accessible association.
    attrs.push(HtmlAttribute::Other("interestfor".into(), id.to_string()));
    attrs.push(HtmlAttribute::Other(
        "aria-describedby".into(),
        id.to_string(),
    ));
    attrs
}

/// Builds the id-independent prompt-element attributes for a prompted link:
/// the `dm-popover-prompt` class, `popover="hint"`, and `role="note"`. The
/// `id` is prepended once allocated (see [`prompted_prompt_attributes`] /
/// [`PopoverNode`]).
fn prompted_prompt_base_attributes() -> Vec<HtmlAttribute> {
    vec![
        HtmlAttribute::Class(ClassDefinition::new("dm-popover-prompt")),
        HtmlAttribute::Other("popover".into(), "hint".to_string()),
        HtmlAttribute::Other("role".into(), "note".to_string()),
    ]
}

/// Builds the full attribute list for a prompted link's prompt element:
/// `id` first, then [`prompted_prompt_base_attributes`]. Used by the streaming
/// writer, which allocates the id eagerly.
fn prompted_prompt_attributes(id: &str) -> Vec<HtmlAttribute> {
    let mut attrs = Vec::with_capacity(4);
    attrs.push(HtmlAttribute::Id(DomId::new(id.to_string())));
    attrs.extend(prompted_prompt_base_attributes());
    attrs
}

/// Derives a readable, stable id slug from a link target: ASCII alphanumerics
/// are lowercased and every other run collapses to a single `-`, bounded to a
/// modest length. Falls back to `link` when nothing survives.
fn popover_id_base(url: &str) -> String {
    let mut base = String::new();
    let mut last_dash = false;
    for ch in url.chars() {
        if base.len() >= 40 {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            base.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            base.push('-');
            last_dash = true;
        }
    }
    let trimmed = base.trim_matches('-');
    if trimmed.is_empty() {
        "link".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Appends the typed browser attributes carried by [`NodeAttrs::browser`].
///
/// The kind-specific `link` / `image` sub-groups are emitted unconditionally
/// from whichever group is present — validation guarantees a `link` group only
/// rides a [`NodeKind::Link`] and an `image` group only a [`NodeKind::Image`],
/// so no node-kind check is needed here. `data_attrs` and `aria_attrs` emit in
/// the deterministic [`BTreeMap`](std::collections::BTreeMap) key order with a
/// fixed `data-` / `aria-` prefix, so an arbitrary attribute (`onclick`,
/// `href`, `src`, `style`) can never be injected through them. `inline_style`
/// is handled by [`node_attributes`] so it coalesces into the single `style`
/// attribute.
fn push_browser_attributes(out: &mut Vec<HtmlAttribute>, attrs: &NodeAttrs) {
    let Some(browser) = attrs.browser_ref() else {
        return;
    };
    if let Some(link) = &browser.link {
        if let Some(target) = &link.target {
            out.push(HtmlAttribute::Target(target.as_str().to_string()));
        }
        if !link.rel.is_empty() {
            let rel = link
                .rel
                .iter()
                .map(|r| r.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            out.push(HtmlAttribute::Other("rel".into(), rel));
        }
        if let Some(download) = &link.download {
            out.push(HtmlAttribute::Other("download".into(), download.clone()));
        }
    }
    if let Some(image) = &browser.image {
        if let Some(loading) = image.loading {
            out.push(HtmlAttribute::Other(
                "loading".into(),
                loading.as_str().to_string(),
            ));
        }
        if let Some(decoding) = image.decoding {
            out.push(HtmlAttribute::Other(
                "decoding".into(),
                decoding.as_str().to_string(),
            ));
        }
    }
    for (name, value) in &browser.data_attrs {
        out.push(HtmlAttribute::Other(
            format!("data-{}", name.as_str()),
            value.clone(),
        ));
    }
    for (name, value) in &browser.aria_attrs {
        out.push(HtmlAttribute::Other(
            format!("aria-{}", name.as_str()),
            value.clone(),
        ));
    }
}

/// Whether a [`Layout`](crate::layout::Layout) lowers a non-default `width` or
/// `padding` for the browser — the box-model declarations whose width contract
/// is content-box. Mirrors the emission conditions in [`layout_to_css`].
fn layout_lowers_box(layout: &crate::layout::Layout) -> bool {
    use crate::layout::{Length, Width};
    use crate::target::RenderTarget;
    // The default `Edges` resolve to `Some(Length::Zero)` (not `None`), so a
    // zero side is a no-op padding that does not widen the box.
    let p = &layout.padding;
    let padded = [&p.top, &p.right, &p.bottom, &p.left]
        .into_iter()
        .any(|tv| !matches!(tv.resolve(RenderTarget::Browser), None | Some(Length::Zero)));
    let sized = match &layout.width {
        Width::Auto => false,
        Width::FitContent => true,
        Width::Fixed(tv) => tv.resolve(RenderTarget::Browser).is_some(),
    };
    padded || sized
}

/// Whether a [`Style`](crate::style::Style) draws at least one border edge — the
/// case where `border` widens the box and the content-box contract matters.
fn style_draws_border(style: &crate::style::Style) -> bool {
    use crate::style::BorderSides;
    style
        .border
        .as_ref()
        .is_some_and(|b| !matches!(b.sides, BorderSides::None))
}

/// Lowers the CSS-bearing layers of a [`Style`](crate::style::Style) to an
/// inline CSS declaration string.
///
/// Always covers `color`, `background` (as `background-color`), and the
/// underline / dim / blink / inverse emphasis layers. When `emit_emphasis` is `true`
/// (a block node) the bold / italic / strikethrough layers are *also* lowered
/// here, to `font-weight` / `font-style` / `text-decoration-line` — wrapping a
/// block element in `<strong>`/`<em>`/`<s>` would be invalid HTML. When it is
/// `false` (an inline node) those three layers are left to the semantic
/// wrappers applied by [`wrap_style_emphasis`].
///
/// `border` lowers to the CSS `border-*` matrix via [`lower_border`]. The
/// deleted `fill` field no longer exists; a painted box is `background`.
fn style_css_declarations(style: &crate::style::Style, emit_emphasis: bool) -> String {
    use crate::color::ColorMode;
    use crate::target::RenderTarget;

    let mut decls: Vec<String> = Vec::new();

    // Both color slots and the border share one lowering: `PaintColor` →
    // validated `CssColor` (`rgb()` / `rgba()` / keyword) via the shared
    // `paint_to_css_color`, so alpha survives and the CSS form stays consistent
    // with the MarkdownPlus emitter.
    let resolve_color = |tv: &crate::layout::TargetValue<
        crate::style::PerMode<crate::style::PaintColor>,
    >|
     -> Option<String> {
        tv.resolve(RenderTarget::Browser)
            .map(|per_mode| *per_mode.resolve(ColorMode::Dark))
            .and_then(super::shared::paint_to_css_color)
            .map(|css| css.to_string())
    };

    if let Some(color) = style.color.as_ref().and_then(&resolve_color) {
        decls.push(format!("color:{color}"));
    }
    if let Some(bg) = style.background.as_ref().and_then(&resolve_color) {
        decls.push(format!("background-color:{bg}"));
    }
    if let Some(underline) = style.emphasis.underline {
        // `css_declaration()` is human-spaced (`prop: value; prop: value`);
        // normalize to the compact `prop:value;prop:value` form used here.
        decls.push(
            underline
                .css_declaration()
                .replace("; ", ";")
                .replace(": ", ":"),
        );
    }
    if style.emphasis.dim {
        decls.push("opacity:0.6".into());
    }
    if style.emphasis.blink {
        decls.push("text-decoration:blink".into());
    }
    if style.emphasis.inverse {
        decls.push("filter:invert(1)".into());
    }
    // Block emphasis: lower bold / italic / strikethrough to CSS rather than
    // to the semantic element wrappers used for inline nodes.
    if emit_emphasis {
        if style.emphasis.bold {
            decls.push("font-weight:bold".into());
        }
        if style.emphasis.italic {
            decls.push("font-style:italic".into());
        }
        if style.emphasis.strikethrough {
            decls.push("text-decoration-line:line-through".into());
        }
    }
    if let Some(border) = style.border.as_ref() {
        lower_border(border, resolve_color, &mut decls);
    }
    decls.join(";")
}

/// Lowers a [`Border`](crate::style::Border) to inline CSS declarations.
///
/// `weight` maps to a pixel `border-width` (`Thin`→`1px`, `Medium`→`2px`,
/// `Thick`→`3px`); `line_style` to the matching `border-style` keyword; and
/// `color` through the shared `PerMode` → CSS color path.
/// [`BorderSides::All`] emits the `border-{width,style,color}` shorthands;
/// [`BorderSides::Sides`] emits only the enabled per-side `border-{side}-*`
/// declarations; [`BorderSides::None`] emits no edges. `radius` lowers to
/// `border-radius` regardless of which sides are drawn.
///
/// [`BorderSides::All`]: crate::style::BorderSides::All
/// [`BorderSides::Sides`]: crate::style::BorderSides::Sides
/// [`BorderSides::None`]: crate::style::BorderSides::None
fn lower_border(
    border: &crate::style::Border,
    resolve_color: impl Fn(
        &crate::layout::TargetValue<crate::style::PerMode<crate::style::PaintColor>>,
    ) -> Option<String>,
    decls: &mut Vec<String>,
) {
    use crate::style::{BorderLineStyle, BorderSides, BorderWeight};
    use crate::target::RenderTarget;

    let width = match border.weight {
        BorderWeight::Thin => "1px",
        BorderWeight::Medium => "2px",
        BorderWeight::Thick => "3px",
    };
    let line_style = match border.line_style {
        BorderLineStyle::Solid => "solid",
        BorderLineStyle::Dashed => "dashed",
        BorderLineStyle::Dotted => "dotted",
        BorderLineStyle::Double => "double",
    };
    let color = border.color.as_ref().and_then(resolve_color);

    match border.sides {
        BorderSides::All => {
            decls.push(format!("border-width:{width}"));
            decls.push(format!("border-style:{line_style}"));
            if let Some(color) = &color {
                decls.push(format!("border-color:{color}"));
            }
        }
        BorderSides::None => {}
        BorderSides::Sides {
            top,
            right,
            bottom,
            left,
        } => {
            for (enabled, side) in [
                (top, "top"),
                (right, "right"),
                (bottom, "bottom"),
                (left, "left"),
            ] {
                if enabled {
                    decls.push(format!("border-{side}-width:{width}"));
                    decls.push(format!("border-{side}-style:{line_style}"));
                    if let Some(color) = &color {
                        decls.push(format!("border-{side}-color:{color}"));
                    }
                }
            }
        }
    }
    if let Some(len) = border.radius.as_ref().and_then(|r| r.resolve(RenderTarget::Browser)) {
        decls.push(format!("border-radius:{}", css_len(len, false)));
    }
}

/// Wraps `fragment` in semantic emphasis tags (`<strong>`, `<em>`, `<s>`) for
/// a node's declared [`Style`](crate::style::Style), preserving nesting.
///
/// This applies only the bold / italic / strikethrough layers and is invoked
/// only for **inline** nodes — wrapping a block element in these tags is
/// invalid HTML, so a block node lowers the same layers to CSS through
/// [`style_css_declarations`]. The underline / dim / blink / inverse layers
/// are always applied as CSS by [`style_css_declarations`].
fn wrap_style_emphasis(
    attrs: &NodeAttrs,
    fragment: BrowserFragment<Ready>,
) -> BrowserFragment<Ready> {
    let Some(style) = attrs.style_ref().filter(|s| !s.is_empty()) else {
        return fragment;
    };
    let emphasis = style.emphasis;
    let mut wrapped = fragment;
    // Innermost-first so the rendered nesting is `<strong><em><s>…`.
    if emphasis.strikethrough {
        wrapped = BrowserFragment::new()
            .define_as_block_tag(BlockTag::S, "")
            .add_component(wrapped)
            .finalize();
    }
    if emphasis.italic {
        wrapped = BrowserFragment::new()
            .define_as_block_tag(BlockTag::Em, "")
            .add_component(wrapped)
            .finalize();
    }
    if emphasis.bold {
        wrapped = BrowserFragment::new()
            .define_as_block_tag(BlockTag::Strong, "")
            .add_component(wrapped)
            .finalize();
    }
    wrapped
}

/// Lowers a [`Layout`](crate::layout::Layout) to an inline CSS declaration
/// string for the browser target.
///
/// `margin` and `padding` edge sides are resolved with
/// [`TargetValue::resolve`] for [`RenderTarget::Browser`]. Vertical sides
/// (`top` / `bottom`) lower a [`Length::Ch`] to `lh` (line-height units);
/// horizontal sides lower it to `ch`. [`Width::FitContent`] emits
/// `width:fit-content` and [`Width::Fixed`] emits an explicit `width`;
/// [`Width::Auto`] omits the property. A `max_width` adds `margin-left` /
/// `margin-right: auto` per the node's [`Alignment`]. [`WordWrap::None`] adds
/// `white-space:nowrap`; any wrapping variant adds `overflow-wrap:break-word`.
/// Lowers a [`Length`](crate::layout::Length) to a CSS dimension. A
/// [`Length::Ch`] becomes `lh` (line-height units) when `vertical`, otherwise
/// `ch`; the other variants are target-independent.
fn css_len(len: &crate::layout::Length, vertical: bool) -> String {
    use crate::layout::Length;
    match len {
        Length::Zero => "0".into(),
        Length::Ch(n) if vertical => format!("{n}lh"),
        Length::Ch(n) => format!("{n}ch"),
        Length::Percent(p) => format!("{p}%"),
        Length::Css(sizing) => sizing.to_string(),
    }
}

/// Lowers the outer box of a thematic-break node to CSS for its SVG wrapper.
///
/// Emits `margin`, `width`, `max_width`, and the alignment auto-margins — the
/// properties the matrix marks Honored for `HorizontalRule`. Padding and
/// word-wrap are intentionally excluded: a rule has no padding box (matrix N/A)
/// and cannot wrap. Returns `None` when the layout neither positions nor sizes
/// the box (every margin side resolves to zero and `width` is `Auto` with no
/// `max_width`), so a rule with a default or absent layout renders the bare SVG
/// with no wrapper element.
fn hr_outer_box_css(layout: &crate::layout::Layout) -> Option<String> {
    use crate::layout::{Alignment, Length, TargetValue, Width};
    use crate::target::RenderTarget;

    fn resolve(tv: &TargetValue<Length>) -> Option<&Length> {
        tv.resolve(RenderTarget::Browser)
    }

    let m = &layout.margin;
    let mut decls: Vec<String> = Vec::new();
    let mut meaningful = false;
    for (tv, prop, vertical) in [
        (&m.top, "margin-top", true),
        (&m.bottom, "margin-bottom", true),
        (&m.left, "margin-left", false),
        (&m.right, "margin-right", false),
    ] {
        if let Some(l) = resolve(tv) {
            decls.push(format!("{prop}:{}", css_len(l, vertical)));
            meaningful |= !matches!(l, Length::Zero);
        }
    }
    match &layout.width {
        Width::Auto => {}
        Width::FitContent => {
            decls.push("width:fit-content".into());
            meaningful = true;
        }
        Width::Fixed(tv) => {
            if let Some(l) = resolve(tv) {
                decls.push(format!("width:{}", css_len(l, false)));
                meaningful = true;
            }
        }
    }
    if let Some(mw) = layout.max_width.as_ref().and_then(resolve) {
        decls.push(format!("max-width:{}", css_len(mw, false)));
        meaningful = true;
        // A capped width centers/anchors via auto margins, mirroring the outer
        // box handling for every other block node in `layout_to_css`.
        match layout.alignment {
            Alignment::Center => {
                decls.push("margin-left:auto".into());
                decls.push("margin-right:auto".into());
            }
            Alignment::Right => decls.push("margin-left:auto".into()),
            Alignment::Left => {}
        }
    }
    meaningful.then(|| decls.join(";"))
}

fn layout_to_css(layout: &crate::layout::Layout) -> String {
    use crate::layout::{Alignment, Length, TargetValue, Width, WordWrap};
    use crate::target::RenderTarget;

    fn resolve(tv: &TargetValue<Length>) -> Option<&Length> {
        tv.resolve(RenderTarget::Browser)
    }

    let m = &layout.margin;
    let mut decls: Vec<String> = Vec::new();
    if let Some(l) = resolve(&m.top) {
        decls.push(format!("margin-top:{}", css_len(l, true)));
    }
    if let Some(l) = resolve(&m.bottom) {
        decls.push(format!("margin-bottom:{}", css_len(l, true)));
    }
    if let Some(l) = resolve(&m.left) {
        decls.push(format!("margin-left:{}", css_len(l, false)));
    }
    if let Some(l) = resolve(&m.right) {
        decls.push(format!("margin-right:{}", css_len(l, false)));
    }
    let p = &layout.padding;
    if let Some(l) = resolve(&p.top) {
        decls.push(format!("padding-top:{}", css_len(l, true)));
    }
    if let Some(l) = resolve(&p.bottom) {
        decls.push(format!("padding-bottom:{}", css_len(l, true)));
    }
    if let Some(l) = resolve(&p.left) {
        decls.push(format!("padding-left:{}", css_len(l, false)));
    }
    if let Some(l) = resolve(&p.right) {
        decls.push(format!("padding-right:{}", css_len(l, false)));
    }
    match &layout.width {
        Width::Auto => {}
        Width::FitContent => decls.push("width:fit-content".into()),
        Width::Fixed(tv) => {
            if let Some(l) = resolve(tv) {
                decls.push(format!("width:{}", css_len(l, false)));
            }
        }
    }
    if let Some(mw) = layout.max_width.as_ref().and_then(resolve) {
        decls.push(format!("max-width:{}", css_len(mw, false)));
        match layout.alignment {
            Alignment::Center => {
                decls.push("margin-left:auto".into());
                decls.push("margin-right:auto".into());
            }
            Alignment::Right => {
                decls.push("margin-left:auto".into());
            }
            Alignment::Left => {}
        }
    }
    match layout.word_wrap {
        WordWrap::None => decls.push("white-space:nowrap".into()),
        WordWrap::WrapProse(..) | WordWrap::BespokeProse(..) | WordWrap::Truncate(_) => {
            decls.push("overflow-wrap:break-word".into());
        }
    }
    decls.join(";")
}

/// Builds a finalized text fragment. The fragment renderer HTML-escapes the
/// content on emit.
fn text_fragment(value: &str) -> BrowserFragment<Ready> {
    BrowserFragment::new()
        .define_as_text_fragment(value.to_string())
        .finalize()
}

/// Builds a disabled `<input type=checkbox>` reflecting a task item's state.
fn checkbox(checked: bool) -> BrowserFragment<Ready> {
    BrowserFragment::new()
        .define_as_void_tag(VoidTag::Input)
        .add_attribute(HtmlAttribute::Type(HtmlType::Checkbox))
        .add_attribute(HtmlAttribute::Disabled(true))
        .add_attribute(HtmlAttribute::Checked(checked))
        .finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::document::{DocumentMetadata, FrontmatterFormat};
    use crate::tree::node::HeadingDepth;
    use crate::tree::{Frontmatter, SourceRegistry};

    fn opts(strictness: RenderStrictness, raw_html: RawHtmlPolicy) -> BrowserRenderOptions {
        BrowserRenderOptions {
            strictness,
            raw_html,
            page: None,
            ..BrowserRenderOptions::default()
        }
    }

    fn render(node: &RenderNode) -> Rendered<BrowserFragment<Ready>> {
        render_browser_node(node, &BrowserRenderOptions::default()).expect("render")
    }

    fn html(node: &RenderNode) -> String {
        render(node).output.render()
    }

    fn html_with_graphics_mode(node: &RenderNode, mode: crate::tree::GraphicsMode) -> String {
        let opts = BrowserRenderOptions {
            graphics_mode: mode,
            ..BrowserRenderOptions::default()
        };
        render_browser_node(node, &opts).expect("render").output.render()
    }

    #[test]
    fn default_options_use_warn_and_escape() {
        let opts = BrowserRenderOptions::default();
        assert_eq!(opts.strictness, RenderStrictness::Warn);
        assert_eq!(opts.raw_html, RawHtmlPolicy::Escape);
        assert!(opts.page.is_none());
    }

    #[test]
    fn paragraph_and_heading() {
        let para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        assert_eq!(html(&para), "<p>hi</p>");

        let heading = RenderNode::heading(
            HeadingDepth::new(3).unwrap(),
            vec![RenderNode::text("Title")],
        );
        assert_eq!(html(&heading), "<h3>Title</h3>");
    }

    #[test]
    fn emphasis_strong_delete() {
        let em = RenderNode::emphasis(vec![RenderNode::text("e")]);
        let st = RenderNode::strong(vec![RenderNode::text("s")]);
        let de = RenderNode::delete(vec![RenderNode::text("d")]);
        assert_eq!(html(&em), "<em>e</em>");
        assert_eq!(html(&st), "<strong>s</strong>");
        assert_eq!(html(&de), "<s>d</s>");
    }

    #[test]
    fn extended_unknown_token_falls_back_to_classed_span() {
        // An unrecognized token wraps its children in a neutral
        // `<span class="extended-{token}">` that a stylesheet can target.
        let node = RenderNode::extended("custom-token", vec![RenderNode::text("hi")], None);
        assert_eq!(html(&node), "<span class=\"extended-custom-token\">hi</span>");

        // Nested inline content is preserved, and any author classes are kept
        // alongside the generated `extended-{token}` class.
        let mut nested = RenderNode::extended(
            "custom-token",
            vec![RenderNode::strong(vec![RenderNode::text("b")])],
            None,
        );
        nested.attrs.classes.push("authored".to_string());
        assert_eq!(
            html(&nested),
            "<span class=\"authored extended-custom-token\"><strong>b</strong></span>"
        );
    }

    #[test]
    fn extended_mark_recovers_semantic_mark_element() {
        // `mark` lowers to the semantic `<mark>` element, not a classed span —
        // this recovers the legacy `<span class="mark">` fidelity regression.
        let node = RenderNode::extended("mark", vec![RenderNode::text("hi")], None);
        let out = html(&node);
        assert_eq!(out, "<mark>hi</mark>");
        assert!(!out.contains("class=\"mark\""));
        assert!(!out.contains("extended-mark"));

        // Nested inline content is preserved inside the `<mark>`.
        let nested = RenderNode::extended(
            "mark",
            vec![RenderNode::strong(vec![RenderNode::text("b")])],
            None,
        );
        assert_eq!(html(&nested), "<mark><strong>b</strong></mark>");
    }

    #[test]
    fn extended_dim_lowers_to_opacity_span() {
        // `dim` has no dedicated element; it lowers to a span carrying the
        // shared dim visual policy (`opacity:0.6`).
        let node = RenderNode::extended("dim", vec![RenderNode::text("hi")], None);
        assert_eq!(html(&node), "<span style=\"opacity:0.6\">hi</span>");
    }

    #[test]
    fn text_is_escaped() {
        let para = RenderNode::paragraph(vec![RenderNode::text("a < b & c")]);
        assert_eq!(html(&para), "<p>a &lt; b &amp; c</p>");
    }

    #[test]
    fn unordered_and_ordered_lists() {
        let ul = RenderNode::list(
            false,
            None,
            vec![
                RenderNode::list_item(None, vec![RenderNode::text("a")]),
                RenderNode::list_item(None, vec![RenderNode::text("b")]),
            ],
        );
        assert_eq!(html(&ul), "<ul><li>a</li><li>b</li></ul>");

        let ol = RenderNode::list(
            true,
            Some(3),
            vec![RenderNode::list_item(None, vec![RenderNode::text("x")])],
        );
        assert_eq!(html(&ol), r#"<ol start="3"><li>x</li></ol>"#);
    }

    #[test]
    fn task_list_item_emits_disabled_checkbox() {
        let item = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(
                Some(true),
                vec![RenderNode::text("done")],
            )],
        );
        let out = html(&item);
        assert!(out.contains(r#"class="task-list-item""#), "{out}");
        assert!(out.contains(r#"type="checkbox""#), "{out}");
        assert!(out.contains("disabled"), "{out}");
        assert!(out.contains("checked"), "{out}");
    }

    #[test]
    fn code_block_and_inline_code() {
        let block = RenderNode::code(Some("rust".into()), None, "let a = 1;");
        assert_eq!(
            html(&block),
            r#"<pre><code class="language-rust">let a = 1;</code></pre>"#
        );

        let inline = RenderNode::inline_code("x < y");
        assert_eq!(html(&inline), "<code>x &lt; y</code>");
    }

    #[test]
    fn mermaid_off_mode_renders_as_plain_code_block() {
        let block = RenderNode::code(Some("mermaid".into()), None, "graph TD; A to B");
        let opts = BrowserRenderOptions {
            graphics_mode: crate::tree::GraphicsMode::Off,
            ..BrowserRenderOptions::default()
        };
        let out = render_browser_node(&block, &opts).unwrap().output.render();
        assert!(
            out.contains(r#"<pre><code class="language-mermaid">"#),
            "expected plain code block under Off mode, got: {out}"
        );
        assert!(out.contains("graph TD"), "source must survive: {out}");
    }

    #[test]
    fn mermaid_code_mode_renders_as_plain_code_block() {
        let block = RenderNode::code(Some("mermaid".into()), None, "graph TD; A to B");
        let opts = BrowserRenderOptions {
            mermaid_mode: crate::tree::BrowserMermaidMode::Code,
            ..BrowserRenderOptions::default()
        };
        let out = render_browser_node(&block, &opts).unwrap().output.render();
        assert!(
            out.contains(r#"<pre><code class="language-mermaid">"#),
            "expected plain code block under Code mode, got: {out}"
        );
        assert!(out.contains("graph TD"), "source must survive: {out}");
    }

    #[test]
    fn mermaid_interactive_mode_emits_pre_class_mermaid() {
        let block = RenderNode::code(Some("mermaid".into()), None, "graph TD; A to B");
        let opts = BrowserRenderOptions {
            mermaid_mode: crate::tree::BrowserMermaidMode::Interactive,
            ..BrowserRenderOptions::default()
        };
        let out = render_browser_node(&block, &opts).unwrap().output.render();
        assert!(
            out.contains(r#"<pre class="mermaid">"#),
            "expected interactive mermaid pre, got: {out}"
        );
        assert!(out.contains("graph TD"), "source must survive: {out}");
    }

    #[test]
    fn mermaid_interactive_escapes_html_in_source() {
        let block = RenderNode::code(
            Some("mermaid".into()),
            None,
            "graph TD; A[script node] to B",
        );
        let opts = BrowserRenderOptions {
            mermaid_mode: crate::tree::BrowserMermaidMode::Interactive,
            ..BrowserRenderOptions::default()
        };
        let out = render_browser_node(&block, &opts).unwrap().output.render();
        assert!(
            out.contains("script node"),
            "text must survive escaping: {out}"
        );
    }

    #[test]
    fn mermaid_static_svg_tries_code_renderer_then_fallback() {
        use std::rc::Rc;

        struct SvgCodeRenderer;
        impl crate::tree::CodeRenderer for SvgCodeRenderer {
            fn render_terminal_code(
                &self,
                _lang: Option<&str>,
                _value: &str,
                _meta: Option<&str>,
                _attrs: &crate::tree::NodeAttrs,
                _context: crate::color::TerminalCodeContext,
            ) -> Option<String> {
                None
            }
            fn render_browser_code(
                &self,
                _lang: Option<&str>,
                _value: &str,
                _meta: Option<&str>,
                _attrs: &crate::tree::NodeAttrs,
            ) -> Option<BrowserFragment<Ready>> {
                None
            }
            fn render_browser_mermaid(
                &self,
                _value: &str,
                _meta: Option<&str>,
                _attrs: &crate::tree::NodeAttrs,
            ) -> Option<BrowserFragment<Ready>> {
                Some(
                    BrowserFragment::new()
                        .define_as_raw_html("<svg></svg>".to_string())
                        .finalize(),
                )
            }
        }

        let block = RenderNode::code(Some("mermaid".into()), None, "graph TD; A to B");
        let opts = BrowserRenderOptions {
            mermaid_mode: crate::tree::BrowserMermaidMode::StaticSvg,
            code_renderer: Some(Rc::new(SvgCodeRenderer)),
            ..BrowserRenderOptions::default()
        };
        let out = render_browser_node(&block, &opts).unwrap().output.render();
        assert!(
            out.contains("<svg>"),
            "expected SVG from code_renderer hook, got: {out}"
        );

        // Without the hook, falls back to plain code block.
        let opts2 = BrowserRenderOptions {
            mermaid_mode: crate::tree::BrowserMermaidMode::StaticSvg,
            ..BrowserRenderOptions::default()
        };
        let out2 = render_browser_node(&block, &opts2).unwrap().output.render();
        assert!(
            out2.contains(r#"<pre><code class="language-mermaid">"#),
            "expected fallback code block, got: {out2}"
        );
    }

    #[test]
    fn non_mermaid_code_ignores_mermaid_mode() {
        let block = RenderNode::code(Some("rust".into()), None, "let a = 1;");
        let opts = BrowserRenderOptions {
            mermaid_mode: crate::tree::BrowserMermaidMode::Interactive,
            ..BrowserRenderOptions::default()
        };
        let out = render_browser_node(&block, &opts).unwrap().output.render();
        assert!(
            out.contains(r#"<pre><code class="language-rust">"#),
            "non-mermaid code must not be affected by mermaid_mode, got: {out}"
        );
    }

    #[test]
    fn mermaid_preserves_node_attrs_in_interactive_mode() {
        let mut block = RenderNode::code(Some("mermaid".into()), None, "graph TD");
        block.attrs.id = Some("diagram-1".into());
        let opts = BrowserRenderOptions {
            mermaid_mode: crate::tree::BrowserMermaidMode::Interactive,
            ..BrowserRenderOptions::default()
        };
        let out = render_browser_node(&block, &opts).unwrap().output.render();
        assert!(out.contains(r#"id="diagram-1""#), "id must be preserved: {out}");
        assert!(
            out.contains(r#"class="mermaid""#),
            "mermaid class must be present: {out}"
        );
    }

    /// Review-2 finding 1: `Interactive` is script-capable and so sits above the
    /// `Vector` ceiling ("no scripts"). Under `Vector`, an `Interactive` request
    /// must NOT emit the client-side mermaid.js `<pre class="mermaid">`; it
    /// degrades to the static SVG rung. Only `Rich` reaches mermaid.js.
    #[test]
    fn mermaid_interactive_under_vector_degrades_to_static_svg() {
        use std::rc::Rc;

        let block = RenderNode::code(Some("mermaid".into()), None, "graph TD");

        // With a StaticSvg-capable hook, the degraded rung produces SVG.
        let opts = BrowserRenderOptions {
            graphics_mode: crate::tree::GraphicsMode::Vector,
            mermaid_mode: crate::tree::BrowserMermaidMode::Interactive,
            code_renderer: Some(Rc::new(MermaidSvgHook)),
            ..BrowserRenderOptions::default()
        };
        let out = render_browser_node(&block, &opts).unwrap().output.render();
        assert!(
            !out.contains(r#"<pre class="mermaid">"#),
            "Vector must not reach the interactive mermaid.js path, got: {out}"
        );
        assert!(
            out.contains("<svg"),
            "Vector + Interactive must degrade to static SVG, got: {out}"
        );

        // Under `Rich`, the same request DOES reach mermaid.js.
        let rich = BrowserRenderOptions {
            graphics_mode: crate::tree::GraphicsMode::Rich,
            mermaid_mode: crate::tree::BrowserMermaidMode::Interactive,
            ..BrowserRenderOptions::default()
        };
        let rich_out = render_browser_node(&block, &rich).unwrap().output.render();
        assert!(
            rich_out.contains(r#"<pre class="mermaid">"#),
            "Rich + Interactive must reach mermaid.js, got: {rich_out}"
        );
    }

    #[test]
    fn mermaid_static_svg_with_off_graphics_mode_fallback() {
        let block = RenderNode::code(Some("mermaid".into()), None, "graph TD");
        let opts = BrowserRenderOptions {
            graphics_mode: crate::tree::GraphicsMode::Off,
            mermaid_mode: crate::tree::BrowserMermaidMode::StaticSvg,
            ..BrowserRenderOptions::default()
        };
        let out = render_browser_node(&block, &opts).unwrap().output.render();
        assert!(
            out.contains(r#"<pre><code class="language-mermaid">"#),
            "Off mode must suppress StaticSvg, got: {out}"
        );
    }

    /// A code renderer whose Mermaid hook always emits SVG — exactly the shape
    /// darkmatter installs.
    struct MermaidSvgHook;
    impl crate::tree::CodeRenderer for MermaidSvgHook {
        fn render_terminal_code(
            &self,
            _lang: Option<&str>,
            _value: &str,
            _meta: Option<&str>,
            _attrs: &crate::tree::NodeAttrs,
            _context: crate::color::TerminalCodeContext,
        ) -> Option<String> {
            None
        }
        fn render_browser_code(
            &self,
            _lang: Option<&str>,
            _value: &str,
            _meta: Option<&str>,
            _attrs: &crate::tree::NodeAttrs,
        ) -> Option<BrowserFragment<Ready>> {
            None
        }
        fn render_browser_mermaid(
            &self,
            _value: &str,
            _meta: Option<&str>,
            _attrs: &crate::tree::NodeAttrs,
        ) -> Option<BrowserFragment<Ready>> {
            Some(
                BrowserFragment::new()
                    .define_as_raw_html("<svg></svg>".to_string())
                    .finalize(),
            )
        }
    }

    /// A **synthetic** resolver that attaches both a CSS block and a module
    /// script to `MermaidDiagram`, delegating everything else to
    /// [`DefaultFeatureResolver`]. It exercises the pipeline's generic
    /// feature-asset dedup path (CSS + JS deduped, first-seen order, byte-identical
    /// fragment/streaming output — spec criterion 10) without depending on
    /// Darkmatter.
    ///
    /// The CSS below is a **pipeline probe, not production Mermaid output**:
    /// production Darkmatter resolves `MermaidDiagram` to a *script-only* bundle
    /// (`css: None`) and delivers the palette through Mermaid `themeVariables`,
    /// because Mermaid does not read CSS custom properties. This fixture pairs a
    /// throwaway CSS rule with the script purely so the CSS-dedup path has an
    /// asset to dedup; it does not model production Mermaid CSS.
    struct SyntheticFeatureResolver;
    impl crate::browser::feature::FeatureResolver for SyntheticFeatureResolver {
        fn resolve(
            &self,
            feature: PageFeature,
            target: crate::target::RenderTarget,
            ctx: &FeatureContext,
        ) -> Result<
            Option<crate::browser::feature::FeatureAssets>,
            crate::browser::feature::FeatureResolveError,
        > {
            use crate::browser::feature::{FeatureAssets, FeatureScript};
            use crate::target::RenderTarget;
            match (target, feature) {
                (RenderTarget::Browser, PageFeature::MermaidDiagram) => Ok(Some(FeatureAssets {
                    // Synthetic dedup probe — NOT production Mermaid CSS (which
                    // is `None`; see the resolver doc above).
                    css: Some(".synthetic-probe{display:block}".into()),
                    js: Some(FeatureScript::Module("import('mermaid')".into())),
                    links: Vec::new(),
                })),
                _ => DefaultFeatureResolver.resolve(feature, target, ctx),
            }
        }
    }

    /// Finding 3 (review-1): under `GraphicsMode::Off`, a `lang="mermaid"` block
    /// must render as a plain code block even when a Mermaid-aware code renderer
    /// (which would otherwise emit SVG via `render_browser_mermaid`) is
    /// installed. The Off branch must short-circuit before consulting any hook.
    #[test]
    fn mermaid_off_bypasses_svg_emitting_code_renderer() {
        use std::rc::Rc;

        let block = RenderNode::code(Some("mermaid".into()), None, "graph TD; A to B");
        let opts = BrowserRenderOptions {
            graphics_mode: crate::tree::GraphicsMode::Off,
            // Default mermaid_mode (Code) — and a hook that would emit SVG.
            code_renderer: Some(Rc::new(MermaidSvgHook)),
            ..BrowserRenderOptions::default()
        };
        let out = render_browser_node(&block, &opts).unwrap().output.render();
        assert!(
            !out.contains("<svg"),
            "Off must not emit SVG even with an SVG-capable hook installed, got: {out}"
        );
        assert!(
            out.contains(r#"<pre><code class="language-mermaid">"#),
            "Off must degrade to a plain code block, got: {out}"
        );
    }

    /// Finding 7 (review-1): a `StaticSvg` promotion that cannot produce an SVG
    /// (no hook installed) must reject under `Strict` and degrade with a lossy
    /// diagnostic under `Warn`.
    #[test]
    fn mermaid_static_svg_failure_honors_strictness() {
        let block = RenderNode::code(Some("mermaid".into()), None, "graph TD");

        let strict = BrowserRenderOptions {
            mermaid_mode: crate::tree::BrowserMermaidMode::StaticSvg,
            strictness: RenderStrictness::Strict,
            ..BrowserRenderOptions::default()
        };
        let err = render_browser_node(&block, &strict)
            .err()
            .expect("strict StaticSvg failure must reject");
        assert!(
            matches!(err, RenderError::LossyRejected { .. }),
            "expected LossyRejected; got {err:?}",
        );

        let warn = BrowserRenderOptions {
            mermaid_mode: crate::tree::BrowserMermaidMode::StaticSvg,
            strictness: RenderStrictness::Warn,
            ..BrowserRenderOptions::default()
        };
        let rendered = render_browser_node(&block, &warn).expect("warn renders");
        assert!(
            rendered.output.render().contains(r#"<pre><code class="language-mermaid">"#),
            "warn StaticSvg failure must degrade to a code block",
        );
        assert!(
            !rendered.diagnostics.is_empty(),
            "warn StaticSvg failure must surface a diagnostic",
        );
    }

    /// A code renderer whose `render_browser_code` hook reproduces the rich
    /// code-block presentation (here a sentinel `code-block-title` wrapper) and
    /// whose Mermaid hook always declines — exactly the shape that exercises the
    /// lossless fallback contract: the generic hook carries title / line-number
    /// / highlight metadata, but never promotes mermaid to SVG.
    struct RichCodeHook;
    impl crate::tree::CodeRenderer for RichCodeHook {
        fn render_terminal_code(
            &self,
            _lang: Option<&str>,
            _value: &str,
            _meta: Option<&str>,
            _attrs: &crate::tree::NodeAttrs,
            _context: crate::color::TerminalCodeContext,
        ) -> Option<String> {
            None
        }
        fn render_browser_code(
            &self,
            _lang: Option<&str>,
            value: &str,
            meta: Option<&str>,
            _attrs: &crate::tree::NodeAttrs,
        ) -> Option<BrowserFragment<Ready>> {
            let title = meta.unwrap_or("untitled");
            Some(
                BrowserFragment::new()
                    .define_as_raw_html(format!(
                        "<figure class=\"code-block\"><figcaption class=\"code-block-title\">{title}</figcaption><pre><code>{value}</code></pre></figure>"
                    ))
                    .finalize(),
            )
        }
        fn render_browser_mermaid(
            &self,
            _value: &str,
            _meta: Option<&str>,
            _attrs: &crate::tree::NodeAttrs,
        ) -> Option<BrowserFragment<Ready>> {
            None
        }
    }

    /// Review-3 finding 1: a non-promoted Mermaid block (`Off`, `Code`, or a
    /// degraded `StaticSvg` failure) must keep its full code-block presentation
    /// by routing through the generic `render_browser_code` hook — not
    /// short-circuiting to a bare `<pre><code>` that drops `title` /
    /// line-numbering / highlight metadata.
    #[test]
    fn mermaid_non_promotion_preserves_code_block_metadata_via_hook() {
        use std::rc::Rc;

        let block = RenderNode::code(
            Some("mermaid".into()),
            Some("title=\"Diagram\"".into()),
            "graph TD; A --> B",
        );

        // Each non-promotion outcome must reach the rich hook fallback.
        let cases = [
            // (graphics_mode, mermaid_mode) — all are non-promotion outcomes.
            (
                crate::tree::GraphicsMode::Off,
                crate::tree::BrowserMermaidMode::StaticSvg,
            ),
            (
                crate::tree::GraphicsMode::Rich,
                crate::tree::BrowserMermaidMode::Code,
            ),
            // StaticSvg with a hook that declines mermaid promotion → degrades.
            (
                crate::tree::GraphicsMode::Rich,
                crate::tree::BrowserMermaidMode::StaticSvg,
            ),
        ];

        for (graphics_mode, mermaid_mode) in cases {
            let opts = BrowserRenderOptions {
                graphics_mode,
                mermaid_mode,
                strictness: RenderStrictness::Warn,
                code_renderer: Some(Rc::new(RichCodeHook)),
                ..BrowserRenderOptions::default()
            };
            let out = render_browser_node(&block, &opts).unwrap().output.render();
            assert!(
                out.contains("code-block-title") && out.contains("Diagram"),
                "{graphics_mode:?}/{mermaid_mode:?} must preserve title metadata via the \
                 render_browser_code hook, got: {out}",
            );
            assert!(
                !out.contains("<svg"),
                "{graphics_mode:?}/{mermaid_mode:?} must not promote to SVG, got: {out}",
            );
        }
    }

    /// Companion to the above: with no code renderer installed, the same
    /// non-promotion outcomes still degrade cleanly to the plain `<pre><code>`
    /// block — the hook is consulted first but its absence is not an error.
    #[test]
    fn mermaid_non_promotion_falls_back_to_plain_code_without_hook() {
        let block = RenderNode::code(Some("mermaid".into()), None, "graph TD; A --> B");
        for (graphics_mode, mermaid_mode) in [
            (
                crate::tree::GraphicsMode::Off,
                crate::tree::BrowserMermaidMode::StaticSvg,
            ),
            (
                crate::tree::GraphicsMode::Rich,
                crate::tree::BrowserMermaidMode::Code,
            ),
            (
                crate::tree::GraphicsMode::Rich,
                crate::tree::BrowserMermaidMode::StaticSvg,
            ),
        ] {
            let opts = BrowserRenderOptions {
                graphics_mode,
                mermaid_mode,
                strictness: RenderStrictness::Warn,
                ..BrowserRenderOptions::default()
            };
            let out = render_browser_node(&block, &opts).unwrap().output.render();
            assert!(
                out.contains(r#"<pre><code class="language-mermaid">"#),
                "{graphics_mode:?}/{mermaid_mode:?} must fall back to a plain code block, got: {out}",
            );
        }
    }

    #[test]
    fn link_and_image() {
        let link = RenderNode::link(
            "https://example.com",
            Some("Site".into()),
            vec![RenderNode::text("here")],
        );
        assert_eq!(
            html(&link),
            r#"<a href="https://example.com" title="Site">here</a>"#
        );

        let image = RenderNode::image("img.png", None, "alt text");
        assert_eq!(html(&image), r#"<img src="img.png" alt="alt text">"#);
    }

    #[test]
    fn link_emits_typed_browser_attributes() {
        let mut link = RenderNode::link("https://example.com/x", None, vec![RenderNode::text("go")]);
        let browser = crate::tree::BrowserAttrs {
            link: Some(crate::tree::LinkBrowserAttrs {
                target: Some(crate::tree::LinkTarget::Blank),
                rel: vec![
                    crate::tree::LinkRelation::NoOpener,
                    crate::tree::LinkRelation::NoReferrer,
                ],
                download: Some("file.txt".into()),
            }),
            ..Default::default()
        };
        link.attrs.set_browser(&browser);
        assert_eq!(
            html(&link),
            r#"<a target="_blank" rel="noopener noreferrer" download="file.txt" href="https://example.com/x">go</a>"#
        );
    }

    #[test]
    fn image_emits_typed_browser_attributes() {
        let mut image = RenderNode::image("p.png", None, "alt");
        let browser = crate::tree::BrowserAttrs {
            image: Some(crate::tree::ImageBrowserAttrs {
                loading: Some(crate::tree::ImageLoading::Lazy),
                decoding: Some(crate::tree::ImageDecoding::Async),
            }),
            ..Default::default()
        };
        image.attrs.set_browser(&browser);
        assert_eq!(
            html(&image),
            r#"<img loading="lazy" decoding="async" src="p.png" alt="alt">"#
        );
    }

    #[test]
    fn node_emits_data_and_aria_attributes_in_deterministic_order_and_escaped() {
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        let mut browser = crate::tree::BrowserAttrs::default();
        // Insert out of sorted order; BTreeMap emits them sorted.
        browser
            .data_attrs
            .insert(crate::tree::DataAttrName::new("z-last").unwrap(), "1".into());
        browser.data_attrs.insert(
            crate::tree::DataAttrName::new("prompt").unwrap(),
            "explain < this".into(),
        );
        browser.aria_attrs.insert(
            crate::tree::AriaAttrName::new("label").unwrap(),
            "info".into(),
        );
        para.attrs.set_browser(&browser);
        assert_eq!(
            html(&para),
            r#"<p data-prompt="explain &lt; this" data-z-last="1" aria-label="info">x</p>"#
        );
    }

    #[test]
    fn inline_style_merges_with_node_style_into_a_single_attribute() {
        use crate::style::{PaintColor, PerMode, Style};
        let mut span = RenderNode::span(vec![], vec![RenderNode::text("x")]);
        // Node `Style` sets a foreground color; inline_style sets a background.
        span.attrs.set_style(&Style {
            color: Some(crate::layout::TargetValue::universal(PerMode::universal(
                PaintColor::new(crate::color::Color::Tailwind(crate::color::Tailwind::Inherit)),
            ))),
            ..Default::default()
        });
        let browser = crate::tree::BrowserAttrs {
            inline_style: Some(crate::stylesheet::CssStyle::new().add(
                crate::stylesheet::CssColorProp::BackgroundColor,
                crate::stylesheet::CssColor::rgb(0x33, 0x66, 0x99),
            )),
            ..Default::default()
        };
        span.attrs.set_browser(&browser);
        // A single `style` attribute carries both the node-`Style` color and the
        // inline_style background, inline_style last so it wins in source order.
        assert_eq!(
            html(&span),
            r#"<span style="color:inherit;background-color:rgb(51, 102, 153)">x</span>"#
        );
    }

    /// The validated `data-*` / `aria-*` name newtypes make it impossible to
    /// inject a duplicate/replacement `href`, `src`, raw `style`, or
    /// event-handler attribute through the typed extension maps — the spec's
    /// "no attribute injection" contract.
    #[test]
    fn attr_name_newtypes_reject_injection_vectors() {
        for forbidden in ["href", "src", "style", "onclick", "onload"] {
            // A `data-`/`aria-` prefix is always prepended, so even an accepted
            // name cannot become a bare `href`/`src`/`style`/`onclick`. The
            // names here are themselves valid suffixes, proving the prefix — not
            // name rejection — is the guard.
            let data = crate::tree::DataAttrName::new(forbidden).unwrap();
            assert_eq!(data.as_str(), forbidden);
        }
        // Names that *could* break out of the attribute token are rejected.
        assert!(crate::tree::DataAttrName::new("on click").is_err());
        assert!(crate::tree::DataAttrName::new("href=\"x\"").is_err());
        assert!(crate::tree::AriaAttrName::new("foo>bar").is_err());
    }

    #[test]
    fn thematic_break_and_breaks() {
        // Under GraphicsMode::Off a plain break degrades to <hr>.
        assert_eq!(
            html_with_graphics_mode(&RenderNode::thematic_break(), crate::tree::GraphicsMode::Off),
            "<hr>"
        );
        // Under the default Rich mode a plain break produces the default SVG.
        let svg = html(&RenderNode::thematic_break());
        assert!(
            svg.starts_with("<svg "),
            "expected SVG under Rich mode, got: {svg}"
        );
        assert_eq!(html(&RenderNode::hard_break()), "<br>");
        assert_eq!(html(&RenderNode::soft_break()), " ");
    }

    /// Spec C5/C6: a thematic break owns a block box, so its outer `Layout`
    /// (`margin` / `width` / `max_width`) must position and size it on Browser —
    /// the SVG core carries only the rule's intrinsic centering. Padding is N/A
    /// (a rule has no padding box) and a default/absent layout adds no wrapper.
    #[test]
    fn thematic_break_honors_outer_box_layout() {
        use crate::layout::{Edges, Layout, Length, TargetValue, Width};

        // No layout: bare SVG, no wrapper.
        assert!(
            html(&RenderNode::thematic_break()).starts_with("<svg "),
            "a rule with no outer layout must render the bare SVG"
        );

        // margin: wrapped in a block box that carries the outer margin.
        let mut ruled = RenderNode::thematic_break();
        ruled.attrs.set_layout(&Layout {
            margin: Edges {
                left: TargetValue::universal(Length::ch(4)),
                ..Edges::default()
            },
            ..Layout::default()
        });
        let out = html(&ruled);
        assert!(
            out.starts_with("<div style=\"") && out.contains("margin-left:4ch"),
            "outer margin must wrap the rule: {out}"
        );
        assert!(out.contains("<svg "), "the SVG core survives the wrapper: {out}");

        // max_width: honored on the outer box.
        let mut capped = RenderNode::thematic_break();
        capped.attrs.set_layout(&Layout {
            max_width: Some(TargetValue::universal(Length::ch(40))),
            ..Layout::default()
        });
        assert!(
            html(&capped).contains("max-width:40ch"),
            "max_width must be honored on the outer box"
        );

        // width: honored on the outer box.
        let mut sized = RenderNode::thematic_break();
        sized.attrs.set_layout(&Layout {
            width: Width::Fixed(TargetValue::universal(Length::Percent(50.0))),
            ..Layout::default()
        });
        assert!(
            html(&sized).contains("width:50%"),
            "Layout::width must be honored on the outer box"
        );

        // padding is N/A: a padding-only layout adds no wrapper.
        let mut padded = RenderNode::thematic_break();
        padded.attrs.set_layout(&Layout {
            padding: Edges::all(Length::ch(1)),
            ..Layout::default()
        });
        assert!(
            html(&padded).starts_with("<svg "),
            "padding is N/A for a rule and must not introduce a wrapper box"
        );
    }

    /// Review-4 finding 2: under GraphicsMode::Off a `<hr>` produced from
    /// darkmatter's HR-attribute fold must surface its typed
    /// [`ThematicBreakAttrs`](crate::tree::ThematicBreakAttrs) as `data-hr-*`
    /// HTML attributes so the styled rule is still user-observable. Under
    /// Vector/Rich those attrs drive an SVG instead.
    #[test]
    fn thematic_break_surfaces_darkmatter_hr_hints_as_data_attrs() {
        use crate::tree::{HrKind, HrWeight, ThematicBreakAttrs};
        let mut hr = RenderNode::thematic_break();
        hr.attrs.set_thematic_break(&ThematicBreakAttrs {
            kind: Some(HrKind::Waves),
            weight: Some(HrWeight::Thick),
            ..Default::default()
        });

        let off = html_with_graphics_mode(&hr, crate::tree::GraphicsMode::Off);
        assert!(
            off.contains(r#"data-hr-kind="waves""#),
            "expected data-hr-kind attribute: {off}",
        );
        assert!(
            off.contains(r#"data-hr-weight="thick""#),
            "expected data-hr-weight attribute: {off}",
        );

        let rich = html(&hr);
        assert!(
            rich.starts_with("<svg "),
            "expected SVG under Rich mode, got: {rich}"
        );
        assert!(
            rich.contains("stroke-dasharray") || rich.contains("M0 20"),
            "expected styled SVG content: {rich}"
        );
    }

    #[test]
    fn block_quote() {
        let bq =
            RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text("quote")])]);
        assert_eq!(html(&bq), "<blockquote><p>quote</p></blockquote>");
    }

    #[test]
    fn span_carries_classes() {
        let span = RenderNode::span(vec!["hl".into(), "big".into()], vec![RenderNode::text("x")]);
        assert_eq!(html(&span), r#"<span class="hl big">x</span>"#);
    }

    #[test]
    fn node_id_maps_to_id_attribute() {
        let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
        para.attrs.id = Some("intro".into());
        assert_eq!(html(&para), r#"<p id="intro">x</p>"#);
    }

    #[test]
    fn table_header_row_becomes_thead() {
        let table = RenderNode::table(
            vec![ColumnAlign::Left, ColumnAlign::Right],
            vec![
                RenderNode::table_row(vec![
                    RenderNode::table_cell(vec![RenderNode::text("H1")]),
                    RenderNode::table_cell(vec![RenderNode::text("H2")]),
                ]),
                RenderNode::table_row(vec![
                    RenderNode::table_cell(vec![RenderNode::text("a")]),
                    RenderNode::table_cell(vec![RenderNode::text("b")]),
                ]),
            ],
        );
        let out = html(&table);
        assert!(out.starts_with("<table><thead><tr>"), "{out}");
        assert!(
            out.contains(r#"<th style="text-align:left">H1</th>"#),
            "{out}"
        );
        assert!(
            out.contains(r#"<th style="text-align:right">H2</th>"#),
            "{out}"
        );
        assert!(out.contains("<tbody><tr>"), "{out}");
        assert!(
            out.contains(r#"<td style="text-align:left">a</td>"#),
            "{out}"
        );
        assert!(out.ends_with("</tbody></table>"), "{out}");
    }

    #[test]
    fn footnotes() {
        let reference = RenderNode::footnote_reference("1");
        assert_eq!(html(&reference), r##"<a href="#fn-1">1</a>"##);

        let definition = RenderNode::footnote_definition(
            "1",
            vec![RenderNode::paragraph(vec![RenderNode::text("note")])],
        );
        // The fragment renderer emits the merged `class` ahead of `id`.
        assert_eq!(
            html(&definition),
            r#"<div class="footnote-definition" id="fn-1"><p>note</p></div>"#
        );
    }

    #[test]
    fn root_wraps_children_in_a_div() {
        let root = RenderNode::root(vec![
            RenderNode::heading(HeadingDepth::new(1).unwrap(), vec![RenderNode::text("T")]),
            RenderNode::paragraph(vec![RenderNode::text("body")]),
        ]);
        assert_eq!(html(&root), "<div><h1>T</h1><p>body</p></div>");
    }

    #[test]
    fn raw_html_allow_emits_verbatim() {
        let node = RenderNode::html("<b>raw</b>", false);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Warn, RawHtmlPolicy::Allow))
                .expect("render");
        assert_eq!(rendered.output.render(), "<b>raw</b>");
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn raw_html_escape_emits_escaped_text_with_diagnostic() {
        let node = RenderNode::html("<b>raw</b>", false);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Warn, RawHtmlPolicy::Escape))
                .expect("render");
        assert_eq!(rendered.output.render(), "&lt;b&gt;raw&lt;/b&gt;");
        assert_eq!(rendered.diagnostics.len(), 1);
    }

    #[test]
    fn raw_html_escape_is_silent_under_lossy() {
        let node = RenderNode::html("<b>raw</b>", false);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Lossy, RawHtmlPolicy::Escape))
                .expect("render");
        assert_eq!(rendered.output.render(), "&lt;b&gt;raw&lt;/b&gt;");
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn raw_html_reject_errors_under_strict() {
        let node = RenderNode::html("<b>raw</b>", false);
        let result = render_browser_node(
            &node,
            &opts(RenderStrictness::Strict, RawHtmlPolicy::Reject),
        );
        assert!(matches!(result, Err(RenderError::LossyRejected { .. })));
    }

    #[test]
    fn raw_html_reject_degrades_under_warn() {
        let node = RenderNode::html("<b>raw</b>", false);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Warn, RawHtmlPolicy::Reject))
                .expect("render");
        assert_eq!(rendered.output.render(), "&lt;b&gt;raw&lt;/b&gt;");
        assert_eq!(rendered.diagnostics.len(), 1);
    }

    #[test]
    fn unsupported_fails_in_strict_mode() {
        // The validation gate escalates the `Unsupported` warning to
        // `InvalidTree` before the node-level path is reached.
        let node = RenderNode::root(vec![RenderNode::unsupported("custom")]);
        let result = render_browser_node(
            &node,
            &opts(RenderStrictness::Strict, RawHtmlPolicy::Escape),
        );
        assert!(matches!(result, Err(RenderError::InvalidTree { .. })));
    }

    #[test]
    fn unsupported_emits_comment_in_warn_mode() {
        let node = RenderNode::root(vec![RenderNode::unsupported("custom")]);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Warn, RawHtmlPolicy::Escape))
                .expect("render");
        assert_eq!(
            rendered.output.render(),
            "<div><!-- unsupported: custom --></div>"
        );
        // One validation-warning diagnostic plus one renderer Unsupported
        // diagnostic.
        assert_eq!(rendered.diagnostics.len(), 2);
    }

    #[test]
    fn unsupported_emits_nothing_in_lossy_mode() {
        let node = RenderNode::root(vec![RenderNode::unsupported("custom")]);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Lossy, RawHtmlPolicy::Escape))
                .expect("render");
        assert_eq!(rendered.output.render(), "<div></div>");
        assert!(rendered.diagnostics.is_empty());
    }

    #[test]
    fn diagnostics_surface_in_rendered_result() {
        let node = RenderNode::root(vec![RenderNode::unsupported("thing")]);
        let rendered =
            render_browser_node(&node, &opts(RenderStrictness::Warn, RawHtmlPolicy::Escape))
                .expect("render");
        assert!(rendered.diagnostics.iter().any(|d| {
            d.kind == crate::tree::DiagnosticKind::Validation
                && d.message.contains("Unsupported node")
        }));
    }

    #[test]
    fn invalid_tree_errors_before_output() {
        let bad = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::table_cell(
            vec![RenderNode::text("x")],
        )])]);
        for strictness in [
            RenderStrictness::Strict,
            RenderStrictness::Warn,
            RenderStrictness::Lossy,
        ] {
            let result = render_browser_node(&bad, &opts(strictness, RawHtmlPolicy::Escape));
            assert!(matches!(result, Err(RenderError::InvalidTree { .. })));
        }
    }

    #[test]
    fn document_renders_root_children_as_page_fragments() {
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![
                RenderNode::heading(
                    HeadingDepth::new(1).unwrap(),
                    vec![RenderNode::text("Title")],
                ),
                RenderNode::paragraph(vec![RenderNode::text("Body")]),
            ]),
        };
        let rendered =
            render_browser_document(&doc, &BrowserRenderOptions::default()).expect("render");
        let html = rendered.output.render().expect("render");
        assert!(
            html.contains("<body><h1>Title</h1><p>Body</p></body>"),
            "{html}"
        );
    }

    #[test]
    fn document_body_is_a_fragment_with_no_document_scaffold() {
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![
                RenderNode::heading(
                    HeadingDepth::new(1).unwrap(),
                    vec![RenderNode::text("Title")],
                ),
                RenderNode::paragraph(vec![RenderNode::text("Body")]),
            ]),
        };
        let opts = BrowserRenderOptions::default();
        let rendered = render_browser_document_body(&doc, &opts).expect("render");

        // The body is a bare fragment: rendered markdown, no document scaffold.
        assert_eq!(rendered.output.body, "<h1>Title</h1><p>Body</p>");
        for forbidden in ["<!DOCTYPE", "<html", "<head", "<body"] {
            assert!(
                !rendered.output.body.contains(forbidden),
                "body fragment must not carry `{forbidden}`, got: {}",
                rendered.output.body
            );
        }

        // The standalone `document` field stays byte-identical to the
        // full-document path so callers that don't embed keep the same output.
        let full = render_browser_document_html(&doc, &opts).expect("render");
        assert_eq!(rendered.output.document, full.output);
    }

    #[test]
    fn document_body_assets_carry_page_stylesheet_without_a_head() {
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("x")])]),
        };
        let mut sheet = crate::stylesheet::Stylesheet::new();
        sheet.push(crate::stylesheet::CssRule::new(
            ".panel",
            crate::stylesheet::CssStyle::new(),
        ));
        let opts = BrowserRenderOptions {
            page: Some(PageOptions {
                stylesheet: Some(sheet),
                css_variables: Some(vec![("primary".into(), "#336699".into())]),
                external_stylesheet: None,
                external_code: None,
            }),
            ..BrowserRenderOptions::default()
        };
        let rendered = render_browser_document_body(&doc, &opts).expect("render");

        // The page-level CSS the full-document path would place in `<head>` is
        // returned in `assets`, wrapped in a `<style>` for inline embedding —
        // not inside a `<head>`.
        assert!(
            rendered.output.assets.starts_with("<style>")
                && rendered.output.assets.contains("--primary: #336699;")
                && rendered.output.assets.contains(".panel"),
            "assets must embed the page stylesheet inline, got: {}",
            rendered.output.assets
        );
        assert!(
            !rendered.output.assets.contains("<head"),
            "assets carry no `<head>`, got: {}",
            rendered.output.assets
        );
    }

    #[test]
    fn document_applies_page_options() {
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("x")])]),
        };
        let mut sheet = crate::stylesheet::Stylesheet::new();
        sheet.push(crate::stylesheet::CssRule::new(
            "body",
            crate::stylesheet::CssStyle::new(),
        ));
        let opts = BrowserRenderOptions {
            strictness: RenderStrictness::Warn,
            raw_html: RawHtmlPolicy::Escape,
            page: Some(PageOptions {
                stylesheet: Some(sheet),
                css_variables: Some(vec![("primary".into(), "#336699".into())]),
                external_stylesheet: None,
                external_code: None,
            }),
            ..BrowserRenderOptions::default()
        };
        let rendered = render_browser_document(&doc, &opts).expect("render");
        let html = rendered.output.render().expect("render");
        assert!(html.contains("--primary: #336699;"), "{html}");
    }

    #[test]
    fn document_ignores_frontmatter() {
        // Frontmatter is intentionally not wired into page metadata.
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata {
                frontmatter: Some(Frontmatter {
                    format: FrontmatterFormat::Yaml,
                    raw: "title: Ignored".into(),
                }),
            },
            root: RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("Body")])]),
        };
        let rendered =
            render_browser_document(&doc, &BrowserRenderOptions::default()).expect("render");
        let html = rendered.output.render().expect("render");
        assert!(html.contains("<body><p>Body</p></body>"), "{html}");
        assert!(!html.contains("Ignored"), "{html}");
    }

    /// Builds a `Document` whose root carries a foreground `Style`.
    fn doc_with_root_color() -> Document {
        use crate::style::{PaintColor, PerMode, Style};
        let mut root = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("Body")])]);
        root.attrs.set_style(&Style {
            color: Some(crate::layout::TargetValue::universal(PerMode::universal(
                PaintColor::new(crate::color::Color::Tailwind(crate::color::Tailwind::Red500)),
            ))),
            ..Style::default()
        });
        Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root,
        }
    }

    #[test]
    fn document_root_foreground_wraps_body_for_inheritance() {
        // Review-1 finding 3: a styled root must be rendered as a wrapping
        // element so its foreground is emitted and inherits to descendants —
        // not silently discarded. Both document folds must agree.
        let doc = doc_with_root_color();
        let opts = BrowserRenderOptions::default();

        let via_page = render_browser_document(&doc, &opts)
            .expect("render")
            .output
            .render()
            .expect("render");
        let direct = render_browser_document_html(&doc, &opts)
            .expect("render")
            .output;

        for (label, html) in [("fragment", &via_page), ("direct", &direct)] {
            assert!(
                html.contains("<body><div style=\"color:rgb(251, 44, 54)\"><p>Body</p></div></body>"),
                "{label}: root foreground must wrap the body in an inheriting div; got:\n{html}"
            );
        }
        assert_eq!(via_page, direct, "both folds must emit identical HTML");
    }

    #[test]
    fn document_unstyled_root_emits_children_directly() {
        // No-regression guard: an unstyled root still streams its children with
        // no extra wrapping div.
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("Body")])]),
        };
        let opts = BrowserRenderOptions::default();
        let html = render_browser_document_html(&doc, &opts).expect("render").output;
        assert!(html.contains("<body><p>Body</p></body>"), "{html}");
    }

    // ── render_browser_document_html: direct document-string renderer ────────

    /// A diverse single-document corpus exercising prose, headings, emphasis,
    /// inline code, lists (incl. ordered `start` and task items), a GFM table
    /// with alignment, links, images, block quotes, nested styling, footnotes,
    /// and a `mark`/`dim` extension — every structural node the direct writer
    /// must keep byte-compatible with the fragment-page path.
    fn parity_corpus() -> Vec<RenderNode> {
        vec![
            RenderNode::heading(
                HeadingDepth::new(1).unwrap(),
                vec![
                    RenderNode::text("Doc "),
                    RenderNode::strong(vec![RenderNode::text("Title")]),
                ],
            ),
            RenderNode::paragraph(vec![
                RenderNode::text("Prose with "),
                RenderNode::emphasis(vec![RenderNode::text("em")]),
                RenderNode::text(", "),
                RenderNode::inline_code("code < x"),
                RenderNode::text(" & a "),
                RenderNode::link(
                    "https://example.com/a?b=1&c=2",
                    Some("Tip".into()),
                    vec![RenderNode::text("link")],
                ),
                RenderNode::text("."),
            ]),
            RenderNode::list(
                false,
                None,
                vec![
                    RenderNode::list_item(None, vec![RenderNode::text("a")]),
                    RenderNode::list_item(Some(true), vec![RenderNode::text("done")]),
                    RenderNode::list_item(Some(false), vec![RenderNode::text("todo")]),
                ],
            ),
            RenderNode::list(
                true,
                Some(3),
                vec![RenderNode::list_item(None, vec![RenderNode::text("x")])],
            ),
            RenderNode::table(
                vec![ColumnAlign::Left, ColumnAlign::Right],
                vec![
                    RenderNode::table_row(vec![
                        RenderNode::table_cell(vec![RenderNode::text("H1")]),
                        RenderNode::table_cell(vec![RenderNode::text("H2")]),
                    ]),
                    RenderNode::table_row(vec![
                        RenderNode::table_cell(vec![RenderNode::text("a & b")]),
                        RenderNode::table_cell(vec![RenderNode::text("c")]),
                    ]),
                ],
            ),
            RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text("quote")])]),
            RenderNode::paragraph(vec![
                RenderNode::image("img.png", Some("T".into()), "alt < text"),
                RenderNode::soft_break(),
                RenderNode::extended("mark", vec![RenderNode::text("hi")], None),
                RenderNode::hard_break(),
                RenderNode::extended("dim", vec![RenderNode::text("lo")], None),
            ]),
            RenderNode::code(Some("rust".into()), None, "let a = 1 < 2;"),
            RenderNode::thematic_break(),
            RenderNode::footnote_definition(
                "1",
                vec![RenderNode::paragraph(vec![RenderNode::text("note")])],
            ),
            // A paragraph carrying typed browser attrs (inline style merged with
            // node Style, plus data-* / aria-* maps) — exercises the generic
            // browser-attr lowering in both writers.
            {
                let mut para = RenderNode::paragraph(vec![RenderNode::text("attrs")]);
                let mut browser = crate::tree::BrowserAttrs {
                    inline_style: Some(crate::stylesheet::CssStyle::new().add(
                        crate::stylesheet::CssColorProp::Color,
                        crate::stylesheet::CssColor::rgb(0x33, 0x66, 0x99),
                    )),
                    ..Default::default()
                };
                browser.data_attrs.insert(
                    crate::tree::DataAttrName::new("prompt").unwrap(),
                    "explain < this".into(),
                );
                browser.aria_attrs.insert(
                    crate::tree::AriaAttrName::new("label").unwrap(),
                    "info".into(),
                );
                para.attrs.set_browser(&browser);
                para
            },
            // A link carrying typed link browser attrs and an image carrying
            // typed image browser attrs — exercises the kind-specific lowering.
            RenderNode::paragraph(vec![
                {
                    let mut link = RenderNode::link(
                        "https://example.com/x",
                        None,
                        vec![RenderNode::text("go")],
                    );
                    let browser = crate::tree::BrowserAttrs {
                        link: Some(crate::tree::LinkBrowserAttrs {
                            target: Some(crate::tree::LinkTarget::Blank),
                            rel: vec![
                                crate::tree::LinkRelation::NoOpener,
                                crate::tree::LinkRelation::NoReferrer,
                            ],
                            download: Some("file.txt".into()),
                        }),
                        ..Default::default()
                    };
                    link.attrs.set_browser(&browser);
                    link
                },
                {
                    let mut image = RenderNode::image("p.png", None, "alt");
                    let browser = crate::tree::BrowserAttrs {
                        image: Some(crate::tree::ImageBrowserAttrs {
                            loading: Some(crate::tree::ImageLoading::Lazy),
                            decoding: Some(crate::tree::ImageDecoding::Async),
                        }),
                        ..Default::default()
                    };
                    image.attrs.set_browser(&browser);
                    image
                },
            ]),
        ]
    }

    /// The direct document-string renderer must emit byte-identical output to
    /// `render_browser_document(...).output.render()` across the shared corpus,
    /// for every graphics mode (the HR/SVG branch differs by mode).
    #[test]
    fn document_html_matches_fragment_page_bytes() {
        for mode in [
            crate::tree::GraphicsMode::Off,
            crate::tree::GraphicsMode::Vector,
            crate::tree::GraphicsMode::Rich,
        ] {
            let doc = Document {
                sources: SourceRegistry::default(),
                metadata: DocumentMetadata::default(),
                root: RenderNode::root(parity_corpus()),
            };
            let opts = BrowserRenderOptions {
                graphics_mode: mode,
                ..BrowserRenderOptions::default()
            };
            let direct = render_browser_document_html(&doc, &opts).expect("direct render");
            let via_page = render_browser_document(&doc, &opts)
                .expect("fragment render")
                .output
                .render()
                .expect("fragment render");
            assert_eq!(direct.output, via_page, "byte mismatch under {mode:?}");
        }
    }

    /// The `<title>` must fall back to the first `<h1>` text exactly as the
    /// fragment-page path does, including when the heading carries nested
    /// inline markup.
    #[test]
    fn document_html_title_falls_back_to_first_h1() {
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![
                RenderNode::heading(
                    HeadingDepth::new(1).unwrap(),
                    vec![
                        RenderNode::text("Hello "),
                        RenderNode::strong(vec![RenderNode::text("World")]),
                    ],
                ),
                RenderNode::paragraph(vec![RenderNode::text("body")]),
            ]),
        };
        let opts = BrowserRenderOptions::default();
        let direct = render_browser_document_html(&doc, &opts).expect("render");
        assert!(
            direct.output.contains("<title>Hello World</title>"),
            "{}",
            direct.output
        );
        let via_page = render_browser_document(&doc, &opts)
            .expect("render")
            .output
            .render()
            .expect("render");
        assert_eq!(direct.output, via_page);
    }

    /// Page options (stylesheet + CSS variables) must roll into the `<head>`
    /// identically on the direct path.
    #[test]
    fn document_html_applies_page_options() {
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("x")])]),
        };
        let mut sheet = crate::stylesheet::Stylesheet::new();
        sheet.push(crate::stylesheet::CssRule::new(
            "body",
            crate::stylesheet::CssStyle::new(),
        ));
        let opts = BrowserRenderOptions {
            page: Some(PageOptions {
                stylesheet: Some(sheet),
                css_variables: Some(vec![("primary".into(), "#336699".into())]),
                external_stylesheet: None,
                external_code: None,
            }),
            ..BrowserRenderOptions::default()
        };
        let direct = render_browser_document_html(&doc, &opts).expect("render");
        assert!(direct.output.contains("--primary: #336699;"), "{}", direct.output);
        let via_page = render_browser_document(&doc, &opts)
            .expect("render")
            .output
            .render()
            .expect("render");
        assert_eq!(direct.output, via_page);
    }

    /// A code-renderer hook fragment is an extension island: only its result is
    /// serialized into the body, and its page-level rollups (here a metadata
    /// title) must reach the `<head>` exactly as through `HtmlPage`.
    #[test]
    fn document_html_serializes_code_renderer_hook_and_rolls_up_head() {
        use crate::microdata::MicrodataKey;
        use std::rc::Rc;

        struct TitleCodeHook;
        impl crate::tree::CodeRenderer for TitleCodeHook {
            fn render_terminal_code(
                &self,
                _lang: Option<&str>,
                _value: &str,
                _meta: Option<&str>,
                _attrs: &crate::tree::NodeAttrs,
                _context: crate::color::TerminalCodeContext,
            ) -> Option<String> {
                None
            }
            fn render_browser_code(
                &self,
                _lang: Option<&str>,
                _value: &str,
                _meta: Option<&str>,
                _attrs: &crate::tree::NodeAttrs,
            ) -> Option<BrowserFragment<Ready>> {
                Some(
                    BrowserFragment::new()
                        .define_as_raw_html("<pre class=\"hl\">highlighted</pre>")
                        .add_metadata_keypair(MicrodataKey::Title, "From Hook")
                        .finalize(),
                )
            }
        }

        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![RenderNode::code(Some("rust".into()), None, "fn x() {}")]),
        };
        let opts = BrowserRenderOptions {
            code_renderer: Some(Rc::new(TitleCodeHook)),
            ..BrowserRenderOptions::default()
        };
        let direct = render_browser_document_html(&doc, &opts).expect("render");
        assert!(
            direct.output.contains("<pre class=\"hl\">highlighted</pre>"),
            "hook body must be serialized: {}",
            direct.output
        );
        assert!(
            direct.output.contains("<title>From Hook</title>"),
            "hook metadata must roll up into head: {}",
            direct.output
        );
        let via_page = render_browser_document(&doc, &opts)
            .expect("render")
            .output
            .render()
            .expect("render");
        assert_eq!(direct.output, via_page);
    }

    /// Diagnostics and fatal validation behavior must match the fragment-page
    /// path across strictness modes.
    #[test]
    fn document_html_shares_strictness_and_diagnostics() {
        // Warn: a raw-HTML node under the escape policy records one diagnostic
        // on both paths.
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![RenderNode::html("<b>raw</b>", false)]),
        };
        let warn = opts(RenderStrictness::Warn, RawHtmlPolicy::Escape);
        let direct = render_browser_document_html(&doc, &warn).expect("render");
        let via_page = render_browser_document(&doc, &warn).expect("render");
        assert_eq!(direct.output, via_page.output.render().expect("render"));
        assert_eq!(direct.diagnostics.len(), via_page.diagnostics.len());
        assert_eq!(direct.diagnostics.len(), 1);

        // Strict: an unsupported node fails the validation gate identically.
        let bad = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![RenderNode::unsupported("custom")]),
        };
        let strict = opts(RenderStrictness::Strict, RawHtmlPolicy::Escape);
        assert!(matches!(
            render_browser_document_html(&bad, &strict),
            Err(RenderError::InvalidTree { .. })
        ));
    }

    /// Every non-fatal [`RawHtmlPolicy`] / [`RenderStrictness`] combination must
    /// produce byte-identical output and the same diagnostic count on the direct
    /// document-string path and the fragment-page path.
    #[test]
    fn document_html_raw_html_policy_parity() {
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![
                RenderNode::paragraph(vec![RenderNode::text("before")]),
                RenderNode::html("<b>raw & <i>markup</i></b>", true),
                RenderNode::paragraph(vec![RenderNode::text("after")]),
            ]),
        };
        for raw in [
            RawHtmlPolicy::Allow,
            RawHtmlPolicy::Escape,
            RawHtmlPolicy::Reject,
        ] {
            // `Reject` under `Strict` is the only fatal combination; the rest
            // degrade to escaped text and stay comparable byte-for-byte.
            for strictness in [RenderStrictness::Warn, RenderStrictness::Lossy] {
                let o = opts(strictness, raw);
                let direct = render_browser_document_html(&doc, &o).expect("direct render");
                let via_page = render_browser_document(&doc, &o).expect("fragment render");
                assert_eq!(
                    direct.output,
                    via_page.output.render().expect("fragment render"),
                    "byte mismatch under {raw:?}/{strictness:?}"
                );
                assert_eq!(
                    direct.diagnostics.len(),
                    via_page.diagnostics.len(),
                    "diagnostic count mismatch under {raw:?}/{strictness:?}"
                );
            }
        }
    }

    /// Mermaid promotion must produce byte-identical output on both paths across
    /// every `graphics_mode` × `mermaid_mode` pairing — covering the static-SVG
    /// hook island, the interactive `<pre class="mermaid">` form, and the
    /// lossless code fallback.
    #[test]
    fn document_html_mermaid_mode_parity() {
        use crate::tree::{BrowserMermaidMode, GraphicsMode};
        use std::rc::Rc;

        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(vec![RenderNode::code(
                Some("mermaid".into()),
                None,
                "graph TD; A-->B",
            )]),
        };
        for graphics_mode in [GraphicsMode::Off, GraphicsMode::Vector, GraphicsMode::Rich] {
            for mermaid_mode in [
                BrowserMermaidMode::Code,
                BrowserMermaidMode::StaticSvg,
                BrowserMermaidMode::Interactive,
            ] {
                let o = BrowserRenderOptions {
                    graphics_mode,
                    mermaid_mode,
                    code_renderer: Some(Rc::new(MermaidSvgHook)),
                    // Own `MermaidDiagram` so the interactive rung resolves; the
                    // two paths must still inject byte-identical assets.
                    feature_resolver: Rc::new(SyntheticFeatureResolver),
                    ..BrowserRenderOptions::default()
                };
                let direct = render_browser_document_html(&doc, &o).expect("direct render");
                let via_page = render_browser_document(&doc, &o).expect("fragment render");
                assert_eq!(
                    direct.output,
                    via_page.output.render().expect("fragment render"),
                    "byte mismatch under {graphics_mode:?}/{mermaid_mode:?}"
                );
            }
        }
    }

    /// Builds a document from a list of top-level nodes.
    fn document_of(children: Vec<RenderNode>) -> Document {
        Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root: RenderNode::root(children),
        }
    }

    /// A link node carrying a Darkmatter-lowered `data-prompt` transport.
    fn prompted_link(url: &str, prompt: &str) -> RenderNode {
        let mut link = RenderNode::link(url.to_string(), None, vec![RenderNode::text("link")]);
        link.attrs.browser_mut_or_default().data_attrs.insert(
            crate::tree::DataAttrName::new("prompt").unwrap(),
            prompt.to_string(),
        );
        link
    }

    /// Generic feature-asset dedup (spec criterion 10): two feature requests for
    /// the same feature resolve to exactly one CSS block and one module script,
    /// deduped and in first-seen order, on both the fragment and streaming paths —
    /// with byte-identical output.
    ///
    /// The CSS asserted here is [`SyntheticFeatureResolver`]'s throwaway probe
    /// rule, **not** a production Mermaid CSS block: production Mermaid is
    /// script-only (`css: None`; palette via `themeVariables`). Criterion 1's
    /// production Mermaid dedup lives in Darkmatter's
    /// `style_features_phase5::two_mermaid_blocks_inject_one_module_script`.
    #[test]
    fn two_feature_requests_inject_one_css_one_script_on_both_paths() {
        use crate::tree::{BrowserMermaidMode, GraphicsMode};

        let doc = document_of(vec![
            RenderNode::code(Some("mermaid".into()), None, "graph TD; A-->B"),
            RenderNode::paragraph(vec![RenderNode::text("between")]),
            RenderNode::code(Some("mermaid".into()), None, "sequenceDiagram; A->>B: hi"),
        ]);
        let opts = BrowserRenderOptions {
            graphics_mode: GraphicsMode::Rich,
            mermaid_mode: BrowserMermaidMode::Interactive,
            feature_resolver: Rc::new(SyntheticFeatureResolver),
            ..BrowserRenderOptions::default()
        };

        let streaming = render_browser_document_html(&doc, &opts).expect("streaming");
        let fragment = render_browser_document(&doc, &opts).expect("fragment");

        assert_eq!(streaming.features, vec![PageFeature::MermaidDiagram]);
        assert_eq!(fragment.features, vec![PageFeature::MermaidDiagram]);

        let streamed_html = streaming.output;
        let fragment_html = fragment.output.render().expect("fragment render");
        assert_eq!(streamed_html, fragment_html, "paths must be byte-identical");

        for html in [&streamed_html, &fragment_html] {
            assert_eq!(
                html.matches(r#"<script type="module">"#).count(),
                1,
                "exactly one module script: {html}"
            );
            assert_eq!(
                html.matches(".synthetic-probe{display:block}").count(),
                1,
                "one deduped feature CSS block: {html}"
            );
            assert_eq!(
                html.matches(r#"<pre class="mermaid">"#).count(),
                2,
                "both containers render: {html}"
            );
        }
    }

    /// A requested but unresolved browser feature fails both full-document
    /// paths with the feature and target named (spec acceptance criterion 9).
    #[test]
    fn interactive_mermaid_unresolved_by_default_resolver_errors() {
        use crate::tree::{BrowserMermaidMode, GraphicsMode};

        let doc = document_of(vec![RenderNode::code(
            Some("mermaid".into()),
            None,
            "graph TD; A-->B",
        )]);
        let opts = BrowserRenderOptions {
            graphics_mode: GraphicsMode::Rich,
            mermaid_mode: BrowserMermaidMode::Interactive,
            ..BrowserRenderOptions::default()
        };

        for result in [
            render_browser_document_html(&doc, &opts).err(),
            render_browser_document(&doc, &opts).err(),
        ] {
            match result {
                Some(RenderError::FeatureResolution(
                    crate::browser::feature::FeatureResolveError::UnresolvedFeature {
                        feature,
                        target,
                    },
                )) => {
                    assert_eq!(feature, PageFeature::MermaidDiagram);
                    assert_eq!(target, crate::target::RenderTarget::Browser);
                }
                other => panic!("expected UnresolvedFeature, got {other:?}"),
            }
        }
    }

    /// Non-interactive Mermaid renderings (`Off`, `Code`, and the
    /// `Vector` + `Interactive` static degrade) request no feature and inject
    /// no interactive assets (spec acceptance criterion 4).
    #[test]
    fn non_interactive_mermaid_requests_no_feature() {
        use crate::tree::{BrowserMermaidMode, GraphicsMode};

        let doc = document_of(vec![RenderNode::code(
            Some("mermaid".into()),
            None,
            "graph TD; A-->B",
        )]);
        for cases in [
            (GraphicsMode::Off, BrowserMermaidMode::Interactive),
            (GraphicsMode::Rich, BrowserMermaidMode::Code),
            (GraphicsMode::Vector, BrowserMermaidMode::StaticSvg),
            // Vector caps Interactive to the static SVG rung — no script.
            (GraphicsMode::Vector, BrowserMermaidMode::Interactive),
        ] {
            let opts = BrowserRenderOptions {
                graphics_mode: cases.0,
                mermaid_mode: cases.1,
                code_renderer: Some(Rc::new(MermaidSvgHook)),
                feature_resolver: Rc::new(SyntheticFeatureResolver),
                ..BrowserRenderOptions::default()
            };
            let streaming = render_browser_document_html(&doc, &opts).expect("streaming");
            let fragment = render_browser_document(&doc, &opts).expect("fragment");
            assert!(
                streaming.features.is_empty(),
                "{cases:?} streaming must request no feature"
            );
            assert!(
                fragment.features.is_empty(),
                "{cases:?} fragment must request no feature"
            );
            assert!(
                !streaming.output.contains(r#"<script type="module">"#),
                "{cases:?} must inject no module script: {}",
                streaming.output
            );
        }
    }

    /// A plain link requests no feature; a prompted link requests Popover and
    /// injects its CSS once (no script), on both paths (spec criterion 7).
    #[test]
    fn prompted_link_requests_popover_plain_link_does_not() {
        let plain = document_of(vec![RenderNode::paragraph(vec![RenderNode::link(
            "https://example.com".to_string(),
            None,
            vec![RenderNode::text("plain")],
        )])]);
        let plain_out = render_browser_document_html(&plain, &BrowserRenderOptions::default())
            .expect("plain render");
        assert!(plain_out.features.is_empty(), "plain link requests nothing");
        assert!(
            !plain_out.output.contains("dm-popover"),
            "no popover CSS for a plain link: {}",
            plain_out.output
        );

        let prompted = document_of(vec![
            RenderNode::paragraph(vec![prompted_link("https://example.com", "go home")]),
            RenderNode::paragraph(vec![prompted_link("https://example.com", "go home")]),
        ]);
        let streaming =
            render_browser_document_html(&prompted, &BrowserRenderOptions::default())
                .expect("streaming");
        let fragment =
            render_browser_document(&prompted, &BrowserRenderOptions::default()).expect("fragment");

        assert_eq!(streaming.features, vec![PageFeature::Popover]);
        assert_eq!(fragment.features, vec![PageFeature::Popover]);
        assert_eq!(
            streaming.output,
            fragment.output.render().expect("fragment render"),
            "byte parity"
        );
        assert_eq!(
            streaming.output.matches(".dm-popover-wrapper{").count(),
            1,
            "popover CSS injected exactly once: {}",
            streaming.output
        );
        assert!(
            !streaming.output.contains("<script"),
            "popover is CSS-only: {}",
            streaming.output
        );
    }

    /// A prompted link emits the accessible wrapper/anchor/prompt structure:
    /// real `href` preserved, the internal `data-prompt` transport dropped, the
    /// prompt escaped and associated through `interestfor` / `aria-describedby`
    /// and the popover `id`.
    #[test]
    fn prompted_link_emits_accessible_popover_markup() {
        let doc = document_of(vec![RenderNode::paragraph(vec![prompted_link(
            "https://example.com/docs",
            "explain this",
        )])]);
        let out = render_browser_document_html(&doc, &BrowserRenderOptions::default())
            .expect("render")
            .output;

        assert!(
            out.contains(r#"<span class="dm-popover-wrapper">"#),
            "wrapper present: {out}"
        );
        assert!(
            out.contains(r#"href="https://example.com/docs""#),
            "real href preserved: {out}"
        );
        assert!(
            !out.contains("data-prompt"),
            "internal transport attribute is not emitted: {out}"
        );
        assert!(
            out.contains(r#"interestfor="dm-popover-https-example-com-docs""#),
            "interestfor names the id: {out}"
        );
        assert!(
            out.contains(r#"aria-describedby="dm-popover-https-example-com-docs""#),
            "aria association present: {out}"
        );
        assert!(
            out.contains(
                r#"<span class="dm-popover-prompt" id="dm-popover-https-example-com-docs" popover="hint" role="note">explain this</span>"#
            ),
            "prompt element carries id/class/popover/role and escaped text: {out}"
        );
    }

    /// Two identical prompted links get distinct, deterministic ids so the
    /// `aria-describedby` / popover association never collides (spec criterion 7).
    #[test]
    fn repeated_prompted_links_get_unique_ids() {
        let doc = document_of(vec![
            RenderNode::paragraph(vec![prompted_link("https://example.com", "one")]),
            RenderNode::paragraph(vec![prompted_link("https://example.com", "two")]),
        ]);
        let streaming = render_browser_document_html(&doc, &BrowserRenderOptions::default())
            .expect("streaming")
            .output;
        let fragment = render_browser_document(&doc, &BrowserRenderOptions::default())
            .expect("fragment")
            .output
            .render()
            .expect("fragment render");

        assert_eq!(streaming, fragment, "byte parity across paths");
        assert!(
            streaming.contains(r#"id="dm-popover-https-example-com""#),
            "first link uses the bare base id: {streaming}"
        );
        assert!(
            streaming.contains(r#"id="dm-popover-https-example-com-1""#),
            "second identical link gets an occurrence suffix: {streaming}"
        );
    }

    /// Two identical prompted links rendered as **separate** fragments and then
    /// composed with [`HtmlPage::from_fragments`] still get document-unique ids,
    /// and each anchor's `interestfor` / `aria-describedby` names its OWN prompt.
    ///
    /// This is the collision the finding targeted: independently rendering each
    /// prompted-link node reset the per-writer id allocator, so both fragments
    /// baked the same `dm-popover-<base>` id. Deferring allocation to the page's
    /// final render (one allocator threaded across every fragment) fixes it
    /// (spec criterion 7).
    #[test]
    fn composed_prompted_link_fragments_get_unique_ids() {
        let opts = BrowserRenderOptions::default();
        let first = render_browser_node(
            &RenderNode::paragraph(vec![prompted_link("https://example.com", "one")]),
            &opts,
        )
        .expect("first fragment")
        .output;
        let second = render_browser_node(
            &RenderNode::paragraph(vec![prompted_link("https://example.com", "two")]),
            &opts,
        )
        .expect("second fragment")
        .output;

        let html = HtmlPage::from_fragments(vec![first, second])
            .render()
            .expect("compose render");

        // Distinct ids across the two independently-rendered fragments.
        assert!(
            html.contains(r#"id="dm-popover-https-example-com""#),
            "first composed link uses the bare base id: {html}"
        );
        assert!(
            html.contains(r#"id="dm-popover-https-example-com-1""#),
            "second composed link gets an occurrence suffix: {html}"
        );

        // Each prompt span keeps its own id and matching text.
        assert!(
            html.contains(
                r#"<span class="dm-popover-prompt" id="dm-popover-https-example-com" popover="hint" role="note">one</span>"#
            ),
            "first prompt keeps the base id and its text: {html}"
        );
        assert!(
            html.contains(
                r#"<span class="dm-popover-prompt" id="dm-popover-https-example-com-1" popover="hint" role="note">two</span>"#
            ),
            "second prompt keeps the suffixed id and its text: {html}"
        );

        // Each anchor associates with its OWN prompt id (interestfor and
        // aria-describedby both name the same occurrence).
        assert!(
            html.contains(
                r#"interestfor="dm-popover-https-example-com" aria-describedby="dm-popover-https-example-com""#
            ),
            "first anchor points at its own prompt: {html}"
        );
        assert!(
            html.contains(
                r#"interestfor="dm-popover-https-example-com-1" aria-describedby="dm-popover-https-example-com-1""#
            ),
            "second anchor points at its own prompt: {html}"
        );
    }

    /// Composing duplicate prompted-link fragments keeps every prompt HTML-escaped
    /// and still yields document-unique ids.
    #[test]
    fn composed_duplicate_prompted_links_escape_and_stay_unique() {
        let opts = BrowserRenderOptions::default();
        let render_one = || {
            render_browser_node(
                &RenderNode::paragraph(vec![prompted_link(
                    "https://example.com",
                    "<b>hi</b>",
                )]),
                &opts,
            )
            .expect("fragment")
            .output
        };

        let html = HtmlPage::from_fragments(vec![render_one(), render_one()])
            .render()
            .expect("compose render");

        assert_eq!(
            html.matches("&lt;b&gt;hi&lt;/b&gt;").count(),
            2,
            "both duplicate prompts stay escaped: {html}"
        );
        assert!(
            !html.contains("<b>hi</b>"),
            "raw prompt markup must not survive: {html}"
        );
        assert!(
            html.contains(r#"id="dm-popover-https-example-com""#)
                && html.contains(r#"id="dm-popover-https-example-com-1""#),
            "duplicate links still get unique ids: {html}"
        );
    }

    /// Hostile HTML in a prompt is escaped in the emitted prompt content.
    #[test]
    fn prompted_link_escapes_hostile_prompt() {
        let doc = document_of(vec![RenderNode::paragraph(vec![prompted_link(
            "https://example.com",
            "<script>alert(1)</script>",
        )])]);
        let out = render_browser_document_html(&doc, &BrowserRenderOptions::default())
            .expect("render")
            .output;
        assert!(
            !out.contains("<script>alert(1)</script>"),
            "raw script must not survive: {out}"
        );
        assert!(
            out.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
            "prompt content is escaped: {out}"
        );
    }

    /// A prompted link preserves its ordinary navigation attributes (class,
    /// target) alongside the popover association.
    #[test]
    fn prompted_link_preserves_navigation_attributes() {
        let mut link =
            RenderNode::link("https://example.com".to_string(), None, vec![RenderNode::text("go")]);
        link.attrs.id = None;
        link.attrs.classes.push("nav".to_string());
        {
            let browser = link.attrs.browser_mut_or_default();
            browser.data_attrs.insert(
                crate::tree::DataAttrName::new("prompt").unwrap(),
                "hint".to_string(),
            );
            browser.link.get_or_insert_with(Default::default).target =
                Some(crate::tree::LinkTarget::Blank);
        }
        let doc = document_of(vec![RenderNode::paragraph(vec![link])]);
        let out = render_browser_document_html(&doc, &BrowserRenderOptions::default())
            .expect("render")
            .output;
        assert!(out.contains(r#"class="nav""#), "class preserved: {out}");
        assert!(out.contains(r#"target="_blank""#), "target preserved: {out}");
        assert!(out.contains("interestfor="), "still enhanced: {out}");
        assert!(!out.contains("data-prompt"), "transport dropped: {out}");
    }

    #[test]
    fn section_renders_as_section_element() {
        let section = RenderNode::section(
            HeadingDepth::new(2).unwrap(),
            vec![RenderNode::text("Title")],
            vec![RenderNode::paragraph(vec![RenderNode::text("Body")])],
        );
        let out = html(&section);
        assert!(out.starts_with("<section>"), "{out}");
        assert!(out.contains("<h2>Title</h2>"), "{out}");
        assert!(out.contains("<p>Body</p>"), "{out}");
        assert!(out.ends_with("</section>"), "{out}");
    }

    #[test]
    fn section_heading_uses_correct_depth() {
        for depth in 1..=6 {
            let section = RenderNode::section(
                HeadingDepth::new(depth).unwrap(),
                vec![RenderNode::text("T")],
                vec![],
            );
            let out = html(&section);
            assert!(out.contains(&format!("<h{depth}>T</h{depth}>")), "{out}");
        }
    }

    #[test]
    fn section_with_styled_heading() {
        let section = RenderNode::section(
            HeadingDepth::new(1).unwrap(),
            vec![
                RenderNode::text("Hello "),
                RenderNode::strong(vec![RenderNode::text("World")]),
            ],
            vec![],
        );
        let out = html(&section);
        assert!(
            out.contains("<h1>Hello <strong>World</strong></h1>"),
            "{out}"
        );
    }

    #[test]
    fn section_carries_node_attrs() {
        let mut section = RenderNode::section(
            HeadingDepth::new(2).unwrap(),
            vec![RenderNode::text("Title")],
            vec![],
        );
        section.attrs.id = Some("intro".into());
        section.attrs.classes = vec!["featured".into()];
        let out = html(&section);
        assert!(out.contains(r#"id="intro""#), "{out}");
        assert!(out.contains(r#"class="featured""#), "{out}");
    }

    #[test]
    fn renderer_lowers_layout_to_css() {
        use crate::layout::{Alignment, Layout, Length, Edges, TargetValue};

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            margin: Edges::x(Length::ch(2)),
            alignment: Alignment::Center,
            max_width: Some(TargetValue::universal(Length::Percent(80.0))),
            ..Layout::default()
        });
        let root = RenderNode::root(vec![para]);

        let rendered = render_browser_node(&root, &BrowserRenderOptions::default()).unwrap();
        let html = rendered.output.render();
        assert!(
            html.contains("margin-left:2ch") || html.contains("margin-left: 2ch"),
            "{html}"
        );
        assert!(
            html.contains("max-width:80%") || html.contains("max-width: 80%"),
            "{html}"
        );
        assert!(
            html.contains("margin-left:auto")
                || html.contains("margin-right:auto")
                || html.contains("margin: 0 auto"),
            "{html}"
        );
    }

    #[test]
    fn renderer_lowers_vertical_margin_to_lh() {
        use crate::layout::{Layout, Length, Edges};

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            margin: Edges::y(Length::ch(1)),
            ..Layout::default()
        });
        let root = RenderNode::root(vec![para]);
        let html = render_browser_node(&root, &BrowserRenderOptions::default())
            .unwrap()
            .output
            .render();
        assert!(
            html.contains("1lh"),
            "vertical Ch margin must lower to lh: {html}"
        );
    }

    #[test]
    fn layout_to_css_emits_padding() {
        use crate::layout::{Edges, Layout, Length};

        let css = layout_to_css(&Layout {
            padding: Edges::x(Length::ch(3)),
            ..Default::default()
        });
        assert!(
            css.contains("padding-left:3ch") && css.contains("padding-right:3ch"),
            "{css}"
        );
    }

    #[test]
    fn layout_to_css_emits_width_modes() {
        use crate::layout::{Layout, Length, TargetValue, Width};

        assert!(
            layout_to_css(&Layout {
                width: Width::FitContent,
                ..Default::default()
            })
            .contains("width:fit-content")
        );
        let fixed = layout_to_css(&Layout {
            width: Width::Fixed(TargetValue::universal(Length::ch(60))),
            ..Default::default()
        });
        assert!(fixed.contains("width:60ch"), "{fixed}");
        // Auto omits an explicit width.
        assert!(!layout_to_css(&Layout::default()).contains("width:"));
    }

    #[test]
    fn border_lowers_full_matrix() {
        use crate::color::{Color, Tailwind};
        use crate::layout::{Length, TargetValue};
        use crate::style::{
            Border, BorderLineStyle, BorderSides, BorderWeight, PerMode, Style,
        };

        let style = Style {
            border: Some(Border {
                weight: BorderWeight::Thick,
                line_style: BorderLineStyle::Dashed,
                sides: BorderSides::Sides {
                    top: false,
                    right: false,
                    bottom: false,
                    left: true,
                },
                radius: Some(TargetValue::universal(Length::ch(1))),
                color: Some(TargetValue::universal(PerMode::universal(Color::Tailwind(
                    Tailwind::Indigo500,
                )))),
            }),
            ..Default::default()
        };
        let css = style_css_declarations(&style, true);
        assert!(css.contains("border-left-style:dashed"), "{css}");
        assert!(css.contains("border-left-width:"), "{css}");
        assert!(css.contains("border-radius:"), "{css}");
        assert!(
            css.contains("border-left-color:") || css.contains("border-color:"),
            "{css}"
        );
    }

    #[test]
    fn border_all_sides_uses_shorthand() {
        use crate::style::{Border, BorderSides, Style};

        let style = Style {
            border: Some(Border {
                sides: BorderSides::All,
                ..Default::default()
            }),
            ..Default::default()
        };
        let css = style_css_declarations(&style, true);
        assert!(css.contains("border-style:solid"), "{css}");
    }

    #[test]
    fn width_and_padding_emit_box_sizing_content_box() {
        use crate::layout::{Edges, Layout, Length, TargetValue, Width};

        // A node lowering an explicit `width` + `padding` must carry
        // `box-sizing:content-box` so a global `border-box` reset cannot
        // reinterpret the width contract.
        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            width: Width::Fixed(TargetValue::universal(Length::ch(20))),
            padding: Edges::x(Length::ch(2)),
            ..Default::default()
        });
        let out = html(&para);
        assert!(out.contains("box-sizing:content-box"), "{out}");
        assert!(out.contains("width:20ch"), "{out}");
        assert!(out.contains("padding-left:2ch"), "{out}");
    }

    #[test]
    fn border_emits_box_sizing_content_box() {
        use crate::style::{Border, BorderSides, Style};

        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_style(&Style {
            border: Some(Border {
                sides: BorderSides::All,
                ..Default::default()
            }),
            ..Default::default()
        });
        let out = html(&para);
        assert!(out.contains("box-sizing:content-box"), "{out}");
        assert!(out.contains("border-style:solid"), "{out}");
    }

    #[test]
    fn plain_node_omits_box_sizing() {
        use crate::layout::{Edges, Layout, Length};

        // Margin alone is a transparent outer layer with no width contract, so
        // it must not drag in a `box-sizing` declaration.
        let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        para.attrs.set_layout(&Layout {
            margin: Edges::x(Length::ch(2)),
            ..Default::default()
        });
        let out = html(&para);
        assert!(!out.contains("box-sizing"), "{out}");
    }

    // ── RT-PROGRESS-001: browser progress ──────────────────────────────────

    fn progress_para(text: &str, hints: crate::tree::ProgressHints) -> RenderNode {
        let mut para = RenderNode::paragraph(vec![RenderNode::text(text)]);
        para.attrs.set_progress_hints(&hints);
        para
    }

    #[test]
    fn progress_emits_progressbar_role_and_aria() {
        let node = progress_para(
            "Loading 60%",
            crate::tree::ProgressHints {
                value: 0.6,
                ..Default::default()
            },
        );
        let out = html(&node);
        assert!(out.contains(r#"role="progressbar""#), "{out}");
        assert!(out.contains(r#"aria-valuemin="0""#), "{out}");
        assert!(out.contains(r#"aria-valuemax="100""#), "{out}");
        assert!(out.contains(r#"aria-valuenow="60""#), "{out}");
        assert!(out.contains(r#"aria-label="Loading""#), "{out}");
    }

    #[test]
    fn progress_uses_stable_classes_and_clamps_value() {
        let node = progress_para(
            "150%",
            crate::tree::ProgressHints {
                value: 1.5,
                ..Default::default()
            },
        );
        let out = html(&node);
        assert!(out.contains(r#"class="progress""#), "{out}");
        assert!(out.contains("progress-track"), "{out}");
        assert!(out.contains("progress-filled"), "{out}");
        assert!(out.contains("progress-percentage"), "{out}");
        // The value clamps to 1.0 → 100%.
        assert!(out.contains("width:100%"), "{out}");
    }

    #[test]
    fn progress_track_width_uses_bar_width_in_ch() {
        let node = progress_para(
            "25%",
            crate::tree::ProgressHints {
                value: 0.25,
                bar_width: 30,
                ..Default::default()
            },
        );
        let out = html(&node);
        assert!(out.contains("width:30ch"), "{out}");
    }

    #[test]
    fn progress_lowers_color_slots_to_background_color() {
        use crate::color::{BasicColor, Color};
        let node = progress_para(
            "50%",
            crate::tree::ProgressHints {
                value: 0.5,
                filled_color: Some(Color::BasicColor(BasicColor::Green)),
                empty_color: Some(Color::BasicColor(BasicColor::Red)),
                ..Default::default()
            },
        );
        let out = html(&node);
        // Two distinct background-color declarations.
        assert_eq!(out.matches("background-color:#").count(), 2, "{out}");
    }

    #[test]
    fn progress_preserves_custom_glyphs_in_data_attributes() {
        let node = progress_para(
            "10%",
            crate::tree::ProgressHints {
                value: 0.1,
                fill_char: '#',
                left_bracket: '(',
                right_bracket: ')',
                ..Default::default()
            },
        );
        let out = html(&node);
        assert!(out.contains(r##"data-fill-char="#""##), "{out}");
        assert!(out.contains(r#"data-left-bracket="(""#), "{out}");
        assert!(out.contains(r#"data-right-bracket=")""#), "{out}");
    }

    #[test]
    fn progress_preserves_bracket_color_as_data_attribute() {
        use crate::color::{BasicColor, Color};
        let node = progress_para(
            "50%",
            crate::tree::ProgressHints {
                value: 0.5,
                bracket_color: Some(Color::BasicColor(BasicColor::Cyan)),
                ..Default::default()
            },
        );
        let out = html(&node);
        assert!(
            out.contains("data-bracket-color=\"#"),
            "bracket color preserved as data attribute: {out}"
        );
    }

    #[test]
    fn progress_omits_bracket_color_attribute_when_unset() {
        let node = progress_para(
            "50%",
            crate::tree::ProgressHints {
                value: 0.5,
                ..Default::default()
            },
        );
        let out = html(&node);
        assert!(
            !out.contains("data-bracket-color"),
            "no bracket color attribute when unset: {out}"
        );
    }

    #[test]
    fn progress_applies_node_layout_to_outer_element() {
        use crate::layout::{Layout, Length, Edges};
        let mut node = progress_para(
            "40%",
            crate::tree::ProgressHints {
                value: 0.4,
                ..Default::default()
            },
        );
        node.attrs.set_layout(&Layout {
            margin: Edges::x(Length::ch(3)),
            ..Layout::default()
        });
        let out = html(&node);
        assert!(out.contains("margin-left:3ch"), "{out}");
    }

    #[test]
    fn plain_paragraph_without_progress_unchanged() {
        let para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
        assert_eq!(html(&para), "<p>hi</p>");
    }

    /// Extracts the attribute list of the `<span class="progress-filled">`
    /// element so its `style` attributes can be asserted exactly.
    fn progress_filled_attrs(out: &str) -> &str {
        let after = out
            .split(r#"class="progress-filled""#)
            .nth(1)
            .expect("progress-filled span");
        &after[..after.find('>').expect("filled span close")]
    }

    #[test]
    fn progress_filled_color_and_width_share_one_style_attribute() {
        use crate::color::{BasicColor, Color};
        let node = progress_para(
            "50%",
            crate::tree::ProgressHints {
                value: 0.5,
                filled_color: Some(Color::BasicColor(BasicColor::Green)),
                ..Default::default()
            },
        );
        let out = html(&node);
        let filled = progress_filled_attrs(&out);
        // Exactly one `style` attribute, carrying both width and color.
        assert_eq!(filled.matches("style=").count(), 1, "{out}");
        assert!(filled.contains("width:50%"), "{out}");
        assert!(filled.contains("background-color:#"), "{out}");
    }

    #[test]
    fn progress_filled_without_color_has_single_width_style() {
        let node = progress_para(
            "25%",
            crate::tree::ProgressHints {
                value: 0.25,
                ..Default::default()
            },
        );
        let out = html(&node);
        let filled = progress_filled_attrs(&out);
        assert_eq!(filled.matches("style=").count(), 1, "{out}");
        assert!(filled.contains("width:25%"), "{out}");
        assert!(!filled.contains("background-color"), "{out}");
    }

    #[test]
    fn progress_label_with_special_chars_is_escaped_once() {
        let node = progress_para(
            r#"A < B & "C" 50%"#,
            crate::tree::ProgressHints {
                value: 0.5,
                ..Default::default()
            },
        );
        let out = html(&node);
        assert!(
            out.contains(r#"aria-label="A &lt; B &amp; &quot;C&quot;""#),
            "{out}"
        );
        // The label must be escaped exactly once — never double-escaped.
        assert!(!out.contains("&amp;lt;"), "double-escaped label: {out}");
    }

    #[test]
    fn progress_label_survives_value_clamp_mismatch() {
        // The fallback text says 150% but the value clamps to 100%; the label
        // is stripped by token shape, not by the clamped percentage, so it
        // does not disappear.
        let node = progress_para(
            "Loading 150%",
            crate::tree::ProgressHints {
                value: 1.5,
                ..Default::default()
            },
        );
        let out = html(&node);
        assert!(out.contains(r#"aria-label="Loading""#), "{out}");
        assert!(out.contains(r#"aria-valuenow="100""#), "{out}");
    }

    // ── RT-TEXTBLOCK-001: browser Style lowering ───────────────────────────

    fn styled_para(style: crate::style::Style) -> RenderNode {
        let mut para = RenderNode::paragraph(vec![RenderNode::text("text")]);
        para.attrs.set_style(&style);
        para
    }

    fn universal_color(
        c: crate::color::Color,
    ) -> crate::layout::TargetValue<crate::style::PerMode<crate::style::PaintColor>> {
        crate::layout::TargetValue::universal(crate::style::PerMode::universal(c))
    }

    #[test]
    fn style_foreground_lowers_to_css_color() {
        use crate::color::{BasicColor, Color};
        let node = styled_para(crate::style::Style {
            color: Some(universal_color(Color::BasicColor(BasicColor::Red))),
            ..Default::default()
        });
        let out = html(&node);
        assert!(out.contains("color:rgb("), "{out}");
    }

    #[test]
    fn style_foreground_alpha_lowers_to_rgba() {
        use crate::color::{BasicColor, Color, RgbColor};
        use crate::style::{Opacity, PaintColor, PerMode};
        let paint = PaintColor::new(Color::Rgb(RgbColor::new(255, 0, 0, BasicColor::Red)))
            .with_opacity(Opacity::from_percent(50).unwrap());
        let node = styled_para(crate::style::Style {
            color: Some(crate::layout::TargetValue::universal(PerMode::universal(
                paint,
            ))),
            ..Default::default()
        });
        let out = html(&node);
        // A single declaration carrying the alpha — no opaque pre-declaration.
        assert!(out.contains("color:rgba(255, 0, 0,"), "{out}");
        assert_eq!(out.matches("color:").count(), 1, "{out}");
    }

    #[test]
    fn style_background_lowers_to_background_color() {
        use crate::color::{BasicColor, Color};
        let node = styled_para(crate::style::Style {
            background: Some(universal_color(Color::BasicColor(BasicColor::Blue))),
            ..Default::default()
        });
        let out = html(&node);
        assert!(out.contains("background-color:rgb("), "{out}");
    }

    #[test]
    fn style_emphasis_on_block_node_lowers_to_css() {
        use crate::style::{Style, TextEmphasis};
        // A styled *block* node lowers bold / italic / strikethrough to CSS —
        // wrapping a `<p>` in `<strong>`/`<em>`/`<s>` is invalid HTML.
        let node = styled_para(Style {
            emphasis: TextEmphasis {
                bold: true,
                italic: true,
                strikethrough: true,
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(
            html(&node),
            r#"<p style="font-weight:bold;font-style:italic;text-decoration-line:line-through">text</p>"#
        );
    }

    #[test]
    fn style_emphasis_on_inline_span_uses_semantic_wrappers() {
        use crate::style::{Style, TextEmphasis};
        // A styled *inline* `Span` keeps the semantic emphasis wrappers, with
        // `<strong>` outermost so the nesting is preserved.
        let mut span = RenderNode::span(vec![], vec![RenderNode::text("x")]);
        span.attrs.set_style(&Style {
            emphasis: TextEmphasis {
                bold: true,
                italic: true,
                strikethrough: true,
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(
            html(&span),
            "<strong><em><s><span>x</span></s></em></strong>"
        );
    }

    #[test]
    fn style_underline_variants_lower_with_css_declaration() {
        use crate::style::{Style, TextEmphasis, UnderlineStyle};
        for variant in [
            UnderlineStyle::Straight,
            UnderlineStyle::Double,
            UnderlineStyle::Curly,
            UnderlineStyle::Dotted,
            UnderlineStyle::Dashed,
        ] {
            let node = styled_para(Style {
                emphasis: TextEmphasis {
                    underline: Some(variant),
                    ..Default::default()
                },
                ..Default::default()
            });
            let out = html(&node);
            assert!(
                out.contains("text-decoration:underline"),
                "{variant:?}: {out}"
            );
        }
    }

    #[test]
    fn style_dim_lowers_to_opacity() {
        use crate::style::{Style, TextEmphasis};
        let node = styled_para(Style {
            emphasis: TextEmphasis {
                dim: true,
                ..Default::default()
            },
            ..Default::default()
        });
        assert!(html(&node).contains("opacity:0.6"));
    }

    #[test]
    fn style_blink_lowers_to_text_decoration_blink() {
        use crate::style::{Style, TextEmphasis};
        let node = styled_para(Style {
            emphasis: TextEmphasis {
                blink: true,
                ..Default::default()
            },
            ..Default::default()
        });
        assert!(html(&node).contains("text-decoration:blink"));
    }

    #[test]
    fn style_inverse_lowers_to_invert_filter() {
        use crate::style::{Style, TextEmphasis};
        let node = styled_para(Style {
            emphasis: TextEmphasis {
                inverse: true,
                ..Default::default()
            },
            ..Default::default()
        });
        assert!(html(&node).contains("filter:invert(1)"));
    }

    #[test]
    fn style_and_layout_share_one_style_attribute() {
        use crate::color::{BasicColor, Color};
        use crate::layout::{Layout, Length, Edges};
        let mut para = RenderNode::paragraph(vec![RenderNode::text("text")]);
        para.attrs.set_style(&crate::style::Style {
            color: Some(universal_color(Color::BasicColor(BasicColor::Red))),
            ..Default::default()
        });
        para.attrs.set_layout(&Layout {
            margin: Edges::x(Length::ch(2)),
            ..Layout::default()
        });
        let out = html(&RenderNode::root(vec![para]));
        // A single style attribute carrying both layout and style CSS.
        assert_eq!(out.matches("style=").count(), 1, "{out}");
        assert!(out.contains("margin-left:2ch"), "{out}");
        assert!(out.contains("color:rgb("), "{out}");
    }

    #[test]
    fn style_applies_to_inline_span() {
        use crate::color::{BasicColor, Color};
        let mut span = RenderNode::span(vec![], vec![RenderNode::text("x")]);
        span.attrs.set_style(&crate::style::Style {
            color: Some(universal_color(Color::BasicColor(BasicColor::Red))),
            ..Default::default()
        });
        let out = html(&span);
        assert!(out.contains("color:rgb("), "{out}");
    }

    #[test]
    fn paragraph_without_style_unchanged() {
        let para = RenderNode::paragraph(vec![RenderNode::text("plain")]);
        assert_eq!(html(&para), "<p>plain</p>");
    }

    // ── RT-TWOCOLUMN-001: browser columns CSS ──────────────────────────────

    fn columns_bq(hints: crate::tree::ColumnsHints, children: Vec<RenderNode>) -> RenderNode {
        let mut bq = RenderNode::block_quote(children);
        bq.attrs.set_columns_hints(&hints);
        bq
    }

    #[test]
    fn columns_default_emits_flex_container_and_classes() {
        let node = columns_bq(
            crate::tree::ColumnsHints {
                left_count: 1,
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("L")]),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        let out = html(&node);
        assert!(out.contains(r#"<div class="columns""#), "{out}");
        assert!(out.contains("display:flex"), "{out}");
        assert!(out.contains("gap:3ch"), "{out}");
        assert_eq!(out.matches(r#"class="column""#).count(), 2, "{out}");
    }

    #[test]
    fn columns_fixed_left_width_lowers_to_flex_and_max_width() {
        let node = columns_bq(
            crate::tree::ColumnsHints {
                left_count: 1,
                left_width: crate::tree::ColumnWidthKind::Fixed(40),
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("L")]),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        let out = html(&node);
        assert!(out.contains("flex:0 0 40ch;max-width:40ch"), "{out}");
        assert!(out.contains("flex:1 1 0"), "{out}");
    }

    #[test]
    fn columns_percent_left_width_clamps_and_lowers_to_flex() {
        let node = columns_bq(
            crate::tree::ColumnsHints {
                left_count: 1,
                left_width: crate::tree::ColumnWidthKind::Percent(1.5),
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("L")]),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        let out = html(&node);
        // The fraction clamps to 1.0 → 100%.
        assert!(out.contains("flex:0 0 100%"), "{out}");
    }

    #[test]
    fn columns_custom_gap_lowers_to_gap_ch() {
        let node = columns_bq(
            crate::tree::ColumnsHints {
                gap: 8,
                ..Default::default()
            },
            vec![],
        );
        assert!(html(&node).contains("gap:8ch"));
    }

    #[test]
    fn columns_layout_and_column_css_coexist() {
        use crate::layout::{Layout, Length, Edges};
        let mut node = columns_bq(crate::tree::ColumnsHints::default(), vec![]);
        node.attrs.set_layout(&Layout {
            margin: Edges::x(Length::ch(2)),
            ..Layout::default()
        });
        let out = html(&node);
        assert!(out.contains("display:flex"), "{out}");
        assert!(out.contains("margin-left:2ch"), "{out}");
    }

    #[test]
    fn columns_empty_left_and_right() {
        let node = columns_bq(
            crate::tree::ColumnsHints {
                left_count: 0,
                ..Default::default()
            },
            vec![],
        );
        let out = html(&node);
        assert_eq!(out.matches(r#"class="column""#).count(), 2, "{out}");
    }

    #[test]
    fn columns_preserve_user_supplied_classes() {
        // A columns node carrying user classes keeps them as external-CSS
        // hooks, merged after the literal `columns` class.
        let mut node = columns_bq(
            crate::tree::ColumnsHints {
                left_count: 1,
                ..Default::default()
            },
            vec![
                RenderNode::paragraph(vec![RenderNode::text("L")]),
                RenderNode::paragraph(vec![RenderNode::text("R")]),
            ],
        );
        node.attrs.classes = vec!["hero".into(), "wide".into()];
        let out = html(&node);
        assert!(out.contains(r#"class="columns hero wide""#), "{out}");
        // The flex CSS still rides on the container alongside the user classes.
        assert!(out.contains("display:flex"), "{out}");
    }

    #[test]
    fn block_quote_without_columns_renders_normally() {
        let bq = RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text("q")])]);
        assert_eq!(html(&bq), "<blockquote><p>q</p></blockquote>");
    }

    // ── RT-TABLE-001: browser caption ──────────────────────────────────────

    #[test]
    fn table_title_emitted_as_caption_before_thead() {
        let mut table = RenderNode::table(
            vec![ColumnAlign::Left],
            vec![RenderNode::table_row(vec![RenderNode::table_cell(vec![
                RenderNode::text("H"),
            ])])],
        );
        table.attrs.set_table_title("Sales <2024>");
        let out = html(&table);
        assert!(
            out.contains("<table><caption>Sales &lt;2024&gt;</caption><thead>"),
            "{out}"
        );
    }

    #[test]
    fn whitespace_only_table_title_emits_no_caption() {
        let mut table = RenderNode::table(
            vec![ColumnAlign::Left],
            vec![RenderNode::table_row(vec![RenderNode::table_cell(vec![
                RenderNode::text("H"),
            ])])],
        );
        table.attrs.set_table_title("  ");
        assert!(!html(&table).contains("<caption>"));
    }

    #[test]
    fn table_cell_alignment_merges_with_existing_style() {
        use crate::color::{BasicColor, Color};
        // A body cell carrying a foreground `Style` and a column alignment
        // must emit exactly one `style` attribute holding both.
        let mut cell = RenderNode::table_cell(vec![RenderNode::text("a")]);
        cell.attrs.set_style(&crate::style::Style {
            color: Some(universal_color(Color::BasicColor(BasicColor::Red))),
            ..Default::default()
        });
        let table = RenderNode::table(
            vec![ColumnAlign::Right],
            vec![
                RenderNode::table_row(vec![RenderNode::table_cell(vec![RenderNode::text("H")])]),
                RenderNode::table_row(vec![cell]),
            ],
        );
        let out = html(&table);
        let after = out.split("<td").nth(1).expect("body cell");
        let td_attrs = &after[..after.find('>').expect("td close")];
        assert_eq!(td_attrs.matches("style=").count(), 1, "{out}");
        assert!(td_attrs.contains("text-align:right"), "{out}");
        assert!(td_attrs.contains("color:rgb("), "{out}");
    }

    // ── RT-FILESYSTEM-001: browser marker policy ───────────────────────────

    #[test]
    fn list_marker_policy_none_suppresses_bullet_via_css() {
        let mut list = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(None, vec![RenderNode::text("a")])],
        );
        list.attrs
            .set_list_marker_policy(crate::tree::ListMarkerPolicy::None);
        let out = html(&list);
        assert!(out.contains("list-style:none"), "{out}");
        // No terminal box-drawing connector text.
        assert!(!out.contains("├"), "{out}");
    }

    #[test]
    fn list_marker_policy_tree_connectors_degrades_to_native_list() {
        let mut list = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(None, vec![RenderNode::text("a")])],
        );
        list.attrs
            .set_list_marker_policy(crate::tree::ListMarkerPolicy::TreeConnectors);
        let out = html(&list);
        assert!(out.starts_with("<ul"), "{out}");
        assert!(!out.contains("└──"), "{out}");
        assert!(out.contains("list-style:none"), "{out}");
    }

    #[test]
    fn default_list_marker_policy_emits_plain_list() {
        let list = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(None, vec![RenderNode::text("a")])],
        );
        assert_eq!(html(&list), "<ul><li>a</li></ul>");
    }

    fn marker_policy_list(policy: crate::tree::ListMarkerPolicy) -> RenderNode {
        let mut list = RenderNode::list(
            false,
            None,
            vec![RenderNode::list_item(None, vec![RenderNode::text("a")])],
        );
        list.attrs.set_list_marker_policy(policy);
        list
    }

    #[test]
    fn marker_policy_none_is_faithful_with_no_diagnostic() {
        // `None` maps faithfully to the browser's own no-marker list, so it
        // degrades silently in every strictness mode.
        for strictness in [
            RenderStrictness::Strict,
            RenderStrictness::Warn,
            RenderStrictness::Lossy,
        ] {
            let list = marker_policy_list(crate::tree::ListMarkerPolicy::None);
            let rendered = render_browser_node(&list, &opts(strictness, RawHtmlPolicy::Escape))
                .expect("None marker policy is faithful in every mode");
            assert!(rendered.output.render().contains("list-style:none"));
            assert!(
                rendered.diagnostics.is_empty(),
                "None is faithful — no diagnostic: {strictness:?}"
            );
        }
    }

    #[test]
    fn marker_policy_tree_connectors_rejected_under_strict() {
        let list = marker_policy_list(crate::tree::ListMarkerPolicy::TreeConnectors);
        let result = render_browser_node(
            &list,
            &opts(RenderStrictness::Strict, RawHtmlPolicy::Escape),
        );
        assert!(matches!(result, Err(RenderError::LossyRejected { .. })));
    }

    #[test]
    fn marker_policy_tree_connectors_degrades_with_diagnostic_under_warn() {
        let list = marker_policy_list(crate::tree::ListMarkerPolicy::TreeConnectors);
        let rendered =
            render_browser_node(&list, &opts(RenderStrictness::Warn, RawHtmlPolicy::Escape))
                .expect("Warn degrades TreeConnectors");
        assert!(rendered.output.render().contains("list-style:none"));
        assert_eq!(rendered.diagnostics.len(), 1, "one lossy diagnostic");
    }

    #[test]
    fn marker_policy_tree_connectors_silent_under_lossy() {
        let list = marker_policy_list(crate::tree::ListMarkerPolicy::TreeConnectors);
        let rendered =
            render_browser_node(&list, &opts(RenderStrictness::Lossy, RawHtmlPolicy::Escape))
                .expect("Lossy degrades TreeConnectors silently");
        assert!(rendered.output.render().contains("list-style:none"));
        assert!(rendered.diagnostics.is_empty());
    }

    // ── RT-TODO-001: browser task-state ────────────────────────────────────

    #[test]
    fn task_state_cancelled_and_in_progress_keep_portable_checkbox() {
        use crate::tree::{TaskHints, TaskState};
        for state in [TaskState::Cancelled, TaskState::InProgress] {
            let mut item = RenderNode::list_item(Some(false), vec![RenderNode::text("t")]);
            item.attrs.set_task_hints(&TaskHints { state });
            let list = RenderNode::list(false, None, vec![item]);
            let out = html(&list);
            assert!(out.contains(r#"type="checkbox""#), "{state:?}: {out}");
            assert!(
                out.contains(r#"class="task-list-item""#),
                "{state:?}: {out}"
            );
            assert!(out.contains("disabled"), "{state:?}: {out}");
            // Terminal-only task glyphs must never leak into browser output.
            for glyph in ['✔', '✗', '⏺', '☐', '⊝'] {
                assert!(
                    !out.contains(glyph),
                    "{state:?} leaked terminal glyph {glyph}: {out}"
                );
            }
        }
    }

    // ── RT-COMPOSE-001: browser sequence join ──────────────────────────────

    #[test]
    fn root_with_sequence_join_renders_children_in_div() {
        // The browser ignores the sequence-join policy (it has no block
        // separators to suppress); children still render in order in a `div`.
        let mut root = RenderNode::root(vec![
            RenderNode::text("inline"),
            RenderNode::paragraph(vec![RenderNode::text("para")]),
        ]);
        root.attrs
            .set_sequence_join(crate::tree::SequenceJoin::None);
        let rendered =
            render_browser_node(&root, &BrowserRenderOptions::default()).expect("render");
        assert_eq!(rendered.output.render(), "<div>inline<p>para</p></div>");
    }

    #[test]
    fn document_with_sequence_join_root_renders_through_entry_point() {
        let mut root = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("x")])]);
        root.attrs
            .set_sequence_join(crate::tree::SequenceJoin::None);
        let doc = Document {
            sources: SourceRegistry::default(),
            metadata: DocumentMetadata::default(),
            root,
        };
        let rendered =
            render_browser_document(&doc, &BrowserRenderOptions::default()).expect("render");
        let html = rendered.output.render().expect("render");
        assert!(html.contains("<body><p>x</p></body>"), "{html}");
    }

    #[test]
    fn nested_sections_render_correctly() {
        let tree = RenderNode::root(vec![RenderNode::section(
            HeadingDepth::new(1).unwrap(),
            vec![RenderNode::text("Parent")],
            vec![RenderNode::section(
                HeadingDepth::new(2).unwrap(),
                vec![RenderNode::text("Child")],
                vec![RenderNode::paragraph(vec![RenderNode::text("Content")])],
            )],
        )]);
        let out = html(&tree);
        assert!(
            out.contains(
                "<section><h1>Parent</h1><section><h2>Child</h2><p>Content</p></section></section>"
            ),
            "{out}"
        );
    }
}
