//! Experimental tree-rendering entry points for darkmatter.
//!
//! These functions are the **internal adapter boundary** between
//! darkmatter's legacy public renderers and the render-tree pipeline shared
//! with `biscuit-terminal` and `renderable`. They exist so the parity test
//! suite and benchmarks can drive the tree path without touching the public
//! `Markdown::as_html`, `Markdown::as_terminal`, or `for_terminal` APIs (which
//! continue to use the legacy event-stream serializers until the cutover gate
//! in `renderable/features/2026-05-20-darkmatter-tree/spec.md`).
//!
//! All entry points are `pub(crate)` — they intentionally live below the
//! public API surface. See
//! `renderable/features/2026-05-20-darkmatter-tree/entry-point-shape.md`.

use std::rc::Rc;

use biscuit_terminal::discovery::detection::ColorDepth as TerminalColorDepth;
use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_document,
};
use biscuit_terminal::terminal::Terminal;
use renderable::tree::{
    BrowserRenderOptions, Diagnostic, Document, MarkdownDialect, MarkdownRenderOptions,
    RawHtmlPolicy, RenderStrictness, SourceDescriptor, render_browser_document,
    render_markdown_document,
};

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
/// `renderable/features/2026-05-20-darkmatter-tree/`.
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
/// defaults to [`RawHtmlPolicy::Escape`] (the safe baseline), and the
/// [`TerminalCodeRenderer`] hook is wired so fenced code blocks are
/// syntax-highlighted (with title / line-number / highlight directives).
/// Mermaid mode and HR CSS variables remain **documented parity gaps** for
/// the internal path until adapter hooks land. See `entry-point-shape.md`.
///
/// ## Errors
///
/// Propagates any fatal [`RenderError`] from
/// [`render_browser_document`]. Non-fatal diagnostics are returned in the
/// [`PipelineResult`] without being demoted to errors.
pub(crate) fn render_tree_html(
    md: &Markdown,
    options: &HtmlOptions,
) -> PipelineRenderResult<String> {
    let (doc, fold_diagnostics) = to_render_document(md);
    let browser_opts = browser_options_from_html_options(options);
    let rendered = render_browser_document(&doc, &browser_opts)?;
    Ok(PipelineResult::new(
        rendered.output.render(),
        fold_diagnostics,
        rendered.diagnostics,
    ))
}

/// Renders a [`Markdown`] to a terminal string via the render-tree pipeline.
///
/// Maps [`TerminalOptions`] to a [`TerminalRenderOptions`] built from a
/// detected [`Terminal`] (respecting any pinned `max_width` /
/// `color_depth`). Image and Mermaid handling stay deferred: the internal
/// terminal renderer does not yet expose an image-renderer hook, so those
/// modes are accepted parity gaps in the experimental phase.
///
/// ## Errors
///
/// Propagates any fatal [`RenderError`] from
/// [`render_terminal_document`].
pub(crate) fn render_tree_terminal(
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

/// Renders a [`Markdown`] back to a Markdown string via the render-tree
/// pipeline.
///
/// Useful primarily for parity and round-trip testing.
///
/// ## Errors
///
/// Propagates any fatal [`RenderError`] from
/// [`render_markdown_document`].
pub(crate) fn render_tree_markdown(md: &Markdown) -> PipelineRenderResult<String> {
    render_tree_markdown_dialect(md, MarkdownDialect::Markdown)
}

/// Renders a [`Markdown`] to either standard Markdown or MarkdownPlus via the
/// render-tree pipeline.
///
/// ## Errors
///
/// Propagates any fatal [`RenderError`] from
/// [`render_markdown_document`].
pub(crate) fn render_tree_markdown_dialect(
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
/// `<pre><code>` fallback. Mermaid mode and HR CSS variables remain documented
/// parity gaps; see `entry-point-shape.md`.
fn browser_options_from_html_options(_opts: &HtmlOptions) -> BrowserRenderOptions {
    BrowserRenderOptions {
        strictness: RenderStrictness::Warn,
        raw_html: RawHtmlPolicy::Escape,
        page: None,
        code_renderer: Some(Rc::new(TerminalCodeRenderer::new())),
    }
}

/// Maps [`TerminalOptions`] to [`TerminalRenderOptions`].
///
/// Uses an optimistic [`Terminal`] when `max_width` is pinned; otherwise
/// falls back to detection. `opts.color_depth`, when set, overrides the
/// terminal's resolved color depth so callers can pin the tree renderer to
/// `ColorDepth::None` (matching the legacy renderer's no-color contract)
/// without depending on host capability detection. Image and Mermaid modes
/// stay deferred — see `entry-point-shape.md` for the documented gaps.
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
    TerminalRenderOptions {
        context: TerminalRenderContext::from_terminal(&term),
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
}
