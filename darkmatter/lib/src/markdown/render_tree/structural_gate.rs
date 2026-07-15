//! Structural performance gate for the **real styled Darkmatter** render path
//! (tree-features spec, "Performance and Ergonomics").
//!
//! Phases 4–5 proved the zero-extension-bag invariant against a synthetic
//! hand-built [`Document`](renderable::tree::Document) in `renderable` and
//! `biscuit-terminal`. This module proves the same invariant end-to-end through
//! the production Darkmatter entry points: a `style:`-frontmatter document is
//! parsed, the full `style:` pipeline is applied to a
//! [`DarkmatterPage`](crate::layout::DarkmatterPage), and the resulting
//! [`TreeBuildContext`](super::build_context::TreeBuildContext) folds a complete
//! typed tree that both target renderers consume.
//!
//! The gate has two halves that defend each other:
//!
//! - **Coverage** ([`styled_corpus_populates_every_typed_group`]): the built
//!   tree must actually carry alpha paint, component policy, text-layout hints,
//!   and typed browser attrs *before* any fold runs. Without this, the
//!   zero-access assertion below could pass vacuously on an empty corpus.
//! - **Zero renderable-owned bag access**
//!   ([`styled_terminal_path_does_zero_renderable_owned_hint_roundtrips`] /
//!   `…_browser_…`): folding that same corpus through the real
//!   `render_tree_{terminal,html}_with_context` entry points must perform
//!   **zero** `renderable.*` `NodeAttrs::data` round-trips. Every first-class
//!   hint — including thematic-break styling, which now rides the typed
//!   [`thematic_break`](renderable::tree::NodeAttrs::thematic_break) field — is
//!   read from a typed field, so the styled production path touches the bag for
//!   neither the renderable-owned *nor* the extension counter. A deliberate
//!   post-render probe proves the counter is still live.
//!
//! The counter is renderable's `hint-access-counter` instrumentation, enabled
//! for this crate's test build via the `[dev-dependencies]` renderable feature
//! re-declaration. It is a thread-local, so every `reset`/`render`/`read` trio
//! must stay on one thread — which it does, since each test runs synchronously.
//!
//! What the zero-access gate directly proves is the single observable it
//! measures: the styled production fold performs **zero `renderable.*`
//! `NodeAttrs::data` bag accesses**. Two of the closeout spec's three structural
//! performance properties (spec section 6) reduce to exactly that observable:
//!
//! - **Zero first-class extension-bag access** *is* the counter's first slot.
//! - **Zero per-node formatted hint keys** follows directly: the only
//!   `format!("{ns}.{key}")` calls live inside `NodeAttrs::{set,get,remove}_hint`,
//!   which are exactly what the counter records — zero accesses means zero
//!   per-node key formatting on the fold path.
//!
//! The third property — **zero typed-attr serde round-trips** — is **not**
//! mechanically certified by this counter, and the gate does not claim it is.
//! The counter only observes the `data` bag; nothing here would catch code that
//! serialized a typed attr *directly* (e.g. `serde_json::to_value(layout_ref)`)
//! without touching the bag. What upholds the no-serde-on-the-hot-path property
//! is the convention that every first-class hint is read through its typed
//! accessor (`layout_ref`, `style_ref`, `text_layout_ref`, `browser_ref`,
//! `thematic_break_ref`, …), none of which call `serde` — a code-review
//! invariant, not a type-level one. The counter is a strong *backstop*: the most
//! likely regression (re-routing a hint through the `data` bag, which does
//! serialize) would trip it. Closing the residual gap would require
//! instrumenting serde conversions on the production fold and a same-corpus
//! before/after Criterion baseline (see `performance-record.md`).

use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_document,
};
use biscuit_terminal::terminal::Terminal;
use renderable::tree::{
    BrowserRenderOptions, Document, HintNamespace, MarkdownDialect, MarkdownRenderOptions,
    NodeAttrs, NodeKind, RenderNode, hint_accesses, render_browser_document,
    render_browser_document_html, render_markdown_document, render_markdown_node,
    reset_hint_accesses,
};

use super::build_context::TreeBuildContext;
use super::entrypoints::{
    render_tree_html_with_context, render_tree_terminal_with_context, resolve_hr_defaults,
    to_render_document_with_context,
};
use crate::layout::DarkmatterPage;
use crate::markdown::Markdown;
use crate::markdown::output::{HtmlOptions, TerminalOptions};
use crate::style::{
    BespokeStyleOverrides, ComponentStyleOverrides, HrStyleOverrides, ListStyleOverrides,
    PageStyleOverrides, apply_bespoke_style, apply_color_style, apply_component_style,
    apply_hr_style, apply_list_style, apply_page_style, from_frontmatter,
};

/// A `style:`-frontmatter document exercising every typed group the gate
/// claims to cover, expanded to the closeout spec's performance-corpus list
/// (spec section 6) so the production styled path reads every typed branch:
///
/// - **alpha background paint:** `block-quote.bg-color: red-500/50`;
/// - **alpha foreground paint:** `block-quote.color: slate-200/50`;
/// - **page-inheriting color:** `page.color: slate-200` (attached to the root);
/// - **page-frame padding/max-width:** `page.top-padding` / `page.left-padding`
///   / `page.max-width` (viewport concerns owned by the page frame);
/// - **component policy layout (alignment + fixed/max width):**
///   `table.alignment: center` + `table.width: 40`, `block-quote.max-width: 50`;
/// - **exact text layout:** `hyperlinks.width: 20`;
/// - **max text layout:** `li.max-width: 40` + `li.alignment: right`;
/// - **ordered-list layout:** `ol.max-width: 40` (an `ol` in the body);
/// - **image text layout + color:** `images.local-style: { color, width }`;
/// - **typed browser link attrs / data-* / inline style:** the structured link
///   directive (`class`, `target`, `prompt`, custom `data-*`, `style`);
/// - **typed browser image inline style:** the per-image `style='…'` directive.
const STYLED_CORPUS: &str = "\
---
style:
    page:
        color: slate-200
        top-padding: 1
        left-padding: 2
        max-width: 100
    block-quote:
        bg-color: red-500/50
        color: slate-200/50
        max-width: 50
    table:
        alignment: center
        width: 40
    hyperlinks:
        width: 20
    li:
        alignment: right
        max-width: 40
    ol:
        max-width: 40
    images:
        local-style:
            color: red-500
            width: 40
---

> A quoted paragraph.

| A | B |
|---|---|
| 1 | 2 |

- Item one
- Item two

1. First
2. Second

[Click](https://example.com \"class='btn' style='color: blue;' target='_blank' prompt='Run it' data-id='42'\")

![A](./local.png \"style='color: green;'\")

---
";

/// Parses [`STYLED_CORPUS`], extracting its `style:` frontmatter.
fn corpus_md() -> Markdown {
    Markdown::try_from_content(STYLED_CORPUS).expect("parse styled corpus")
}

/// Builds a fully styled [`DarkmatterPage`] from [`STYLED_CORPUS`] by running
/// the same `style:` pipeline the production CLI / page paths run.
fn styled_page(md: &Markdown) -> DarkmatterPage {
    let term = Terminal::new_optimistic(80);
    let page = DarkmatterPage::new(&term);
    let (style, _warnings) = from_frontmatter(md.frontmatter()).expect("parse style");

    let page =
        apply_page_style(page, &style, PageStyleOverrides::default()).expect("apply_page_style");
    let page = apply_component_style(page, &style, ComponentStyleOverrides::default())
        .expect("apply_component_style");
    let page =
        apply_list_style(page, &style, ListStyleOverrides::default()).expect("apply_list_style");
    let page = apply_hr_style(page, &style, HrStyleOverrides::default()).expect("apply_hr_style");
    let page = apply_color_style(page, &style).expect("apply_color_style");
    apply_bespoke_style(page, &style, BespokeStyleOverrides::default(), None)
        .expect("apply_bespoke_style")
}

/// Builds the construction-time [`TreeBuildContext`] from a styled page exactly
/// as [`DarkmatterPage::render`] does, so the tree the gate inspects for
/// coverage is byte-for-byte the tree the renderers fold below. The owned HR
/// defaults are passed in because the context borrows them.
fn build_context<'a>(
    page: &'a DarkmatterPage,
    hr_defaults_owned: &'a Option<crate::markdown::inline::HorizontalRuleAttrs>,
) -> TreeBuildContext<'a> {
    TreeBuildContext {
        component_policies: page.component_policies(),
        page_color: page.page_color().copied(),
        page_bg_color: page.page_bg_color().copied(),
        hyperlink_style: page.hyperlink_style(),
        local_hyperlink_style: page.local_hyperlink_style(),
        local_image_style: page.local_image_style(),
        hr_defaults: hr_defaults_owned.as_ref(),
    }
}

/// Whether each typed group is populated somewhere in the built tree.
#[derive(Default, Debug)]
struct Coverage {
    layout: bool,
    alpha_paint: bool,
    component_policy: bool,
    text_layout: bool,
    browser_link: bool,
    browser_inline_style: bool,
    browser_data_attrs: bool,
}

/// Collects [`Coverage`] over the whole tree, proving the corpus is not vacuous.
fn coverage(root: &RenderNode) -> Coverage {
    use renderable::color::ColorMode;
    use renderable::style::Opacity;
    use renderable::target::RenderTarget;

    fn has_alpha(node: &RenderNode) -> bool {
        let Some(style) = node.attrs.style_ref() else {
            return false;
        };
        // A node carries alpha paint if either color slot resolves to a
        // non-opaque PaintColor for any concrete (target, mode).
        [style.color.as_ref(), style.background.as_ref()]
            .into_iter()
            .flatten()
            .any(|tv| {
                tv.resolve(RenderTarget::Terminal)
                    .map(|per_mode| per_mode.resolve(ColorMode::Dark))
                    .is_some_and(|paint| paint.opacity != Opacity::OPAQUE)
            })
    }

    fn walk(node: &RenderNode, cov: &mut Coverage) {
        cov.layout |= node.attrs.layout_ref().is_some();
        cov.alpha_paint |= has_alpha(node);
        // Component policy is observable as a baked Style or Layout on a
        // container node the fold attaches policy to.
        if matches!(
            node.kind,
            NodeKind::Table { .. } | NodeKind::BlockQuote { .. } | NodeKind::List { .. }
        ) {
            cov.component_policy |=
                node.attrs.style_ref().is_some() || node.attrs.layout_ref().is_some();
        }
        cov.text_layout |= node.attrs.text_layout_ref().is_some();
        if let Some(browser) = node.attrs.browser_ref() {
            cov.browser_link |= browser.link.is_some();
            cov.browser_inline_style |= browser.inline_style.is_some();
            cov.browser_data_attrs |= !browser.data_attrs.is_empty();
        }
        for child in node.children() {
            walk(child, cov);
        }
    }

    let mut cov = Coverage::default();
    walk(root, &mut cov);
    cov
}

/// Folds [`STYLED_CORPUS`] to a complete [`Document`] through the real styled
/// build context.
fn build_styled_document() -> Document {
    let md = corpus_md();
    let page = styled_page(&md);
    let hr_owned = resolve_hr_defaults(&md, &page.hr_defaults());
    let ctx = build_context(&page, &hr_owned);
    let (doc, diags) = to_render_document_with_context(&md, &ctx).expect("fold must succeed");
    assert!(diags.is_empty(), "styled corpus must fold cleanly: {diags:?}");
    doc
}

/// Production-entry structural assertion (closeout spec section 5, AC6): a
/// styled Darkmatter source's *initial* `Document` — before any fold runs —
/// already carries layout, paint, text-layout, and browser attrs as typed
/// fields. This both proves the "complete typed Document" thesis and guards the
/// zero-access gate below from passing vacuously on an empty corpus.
#[test]
fn styled_corpus_populates_every_typed_group() {
    let doc = build_styled_document();
    let cov = coverage(&doc.root);
    assert!(cov.layout, "corpus must carry typed Layout: {cov:?}");
    assert!(cov.alpha_paint, "corpus must carry alpha paint: {cov:?}");
    assert!(
        cov.component_policy,
        "corpus must carry baked component policy: {cov:?}"
    );
    assert!(cov.text_layout, "corpus must carry text-layout hints: {cov:?}");
    assert!(
        cov.browser_link,
        "corpus must carry typed browser link attrs: {cov:?}"
    );
    assert!(
        cov.browser_inline_style,
        "corpus must carry browser inline_style: {cov:?}"
    );
    assert!(
        cov.browser_data_attrs,
        "corpus must carry browser data_attrs: {cov:?}"
    );
}

/// Folding the styled corpus through the real terminal entry point must not
/// round-trip any hint through [`NodeAttrs::data`]. Component policy, alpha
/// paint, text-layout, and thematic-break styling all ride typed fields, so the
/// styled production path performs **zero** bag round-trips of either kind. A
/// deliberate post-render probe proves the counter is live, so the zero is real
/// and not a silently disabled counter.
#[test]
fn styled_terminal_path_does_zero_renderable_owned_hint_roundtrips() {
    let md = corpus_md();
    let page = styled_page(&md);
    let hr_owned = resolve_hr_defaults(&md, &page.hr_defaults());
    let ctx = build_context(&page, &hr_owned);
    let opts = TerminalOptions {
        max_width: Some(80),
        ..TerminalOptions::default()
    };

    reset_hint_accesses();
    let _ = render_tree_terminal_with_context(&md, &opts, &ctx).expect("terminal render");
    let (renderable_owned, extension) = hint_accesses();

    assert_eq!(
        renderable_owned, 0,
        "the styled terminal path must read every first-class hint from a typed field",
    );
    assert_eq!(
        extension, 0,
        "now that HR styling rides the typed `thematic_break` field, the styled \
         terminal path performs zero extension-bag round-trips too",
    );

    // Liveness guard: the zeros above must be real, not a silently disabled
    // counter. A deliberate probe must still register on the extension slot.
    reset_hint_accesses();
    let probe = NodeAttrs::default();
    let _ = probe.get_hint(HintNamespace("liveness.probe"), "k");
    assert!(
        hint_accesses().1 > 0,
        "the hint-access counter must be live (probe should have incremented it)",
    );
}

/// Folding the styled corpus through the real browser entry point must likewise
/// perform zero renderable-owned bag round-trips: structured directives,
/// inline style, and `data-*` are all typed browser attrs.
#[test]
fn styled_browser_path_does_zero_renderable_owned_hint_roundtrips() {
    let md = corpus_md();
    let page = styled_page(&md);
    let hr_owned = resolve_hr_defaults(&md, &page.hr_defaults());
    let ctx = build_context(&page, &hr_owned);
    let opts = HtmlOptions::default();

    reset_hint_accesses();
    let _ = render_tree_html_with_context(&md, &opts, &ctx).expect("html render");
    let (renderable_owned, _extension) = hint_accesses();

    assert_eq!(
        renderable_owned, 0,
        "the styled browser path must read every first-class hint from a typed field",
    );
}

/// Rendering one built tree through **all four targets** — Terminal (at two
/// widths), Browser, Markdown, and MarkdownPlus — must not mutate the input
/// tree: each renderer resolves width-dependent text-layout against its own
/// fold context, never by rewriting the stored tree. Pins the spec's
/// immutable-tree contract (closeout spec section 5, AC6) across every target
/// and width.
#[test]
fn rendering_does_not_mutate_the_tree_across_targets_and_widths() {
    let doc = build_styled_document();
    let pristine = doc.clone();

    let terminal_at = |width: u32| {
        let context = TerminalRenderContext::from_terminal(&Terminal::new_optimistic(width));
        let opts = TerminalRenderOptions {
            context,
            ..TerminalRenderOptions::default()
        };
        render_terminal_document(&doc, &opts).expect("terminal render")
    };

    let narrow = terminal_at(40);
    let wide = terminal_at(100);
    // Width-dependent text layout must actually depend on width somewhere in
    // the corpus, so the immutability claim is non-trivial.
    assert_ne!(
        narrow.output, wide.output,
        "the corpus must render differently at 40 vs 100 cols",
    );

    let _ = render_browser_document_html(&doc, &BrowserRenderOptions::default())
        .expect("browser render");

    for dialect in [MarkdownDialect::Markdown, MarkdownDialect::MarkdownPlus] {
        let opts = MarkdownRenderOptions {
            dialect,
            ..MarkdownRenderOptions::default()
        };
        let _ = render_markdown_document(&doc, &opts).expect("markdown render");
    }

    assert_eq!(doc, pristine, "rendering must not mutate the input tree");
}

/// The two browser surfaces must agree on style and attributes for the styled
/// production corpus: the streaming final-string path
/// (`render_browser_document_html`) and the fragment-tree path
/// (`render_browser_document(...).output.render()`) emit byte-identical HTML, so
/// a component composing through `BrowserFragment`s sees exactly what the
/// document fold emits (closeout spec section 5, AC6: "browser fragment and
/// streaming output agree for style and attributes"). The shared synthetic
/// corpus is also pinned in `renderable`; this pins the real Darkmatter entry.
#[test]
fn styled_browser_fragment_and_streaming_paths_agree() {
    let doc = build_styled_document();
    let opts = BrowserRenderOptions::default();

    let streamed = render_browser_document_html(&doc, &opts)
        .expect("streaming browser render")
        .output;
    let via_fragments = render_browser_document(&doc, &opts)
        .expect("fragment browser render")
        .output
        .render()
        .expect("fragment browser render");

    assert_eq!(
        streamed, via_fragments,
        "the streaming and fragment browser paths must emit identical HTML",
    );
}

/// Markdown-dialect degradation on the styled production corpus (closeout spec
/// section 5, AC8). Portable [`MarkdownDialect::Markdown`] drops paint,
/// geometry, and browser-only attributes; [`MarkdownDialect::MarkdownPlus`]
/// stays within its documented HTML dialect policy and never becomes a second
/// browser renderer — its links and images keep their plain Markdown spelling
/// because the structured browser attrs have no MarkdownPlus form.
///
/// The assertions run on the rendered **body** ([`render_markdown_node`] over
/// the root), not the whole document, so the preserved raw frontmatter — which
/// legitimately contains `color:` / `bg-color:` style keys — does not
/// false-positive the "no paint" checks.
#[test]
fn markdown_dialects_degrade_within_policy() {
    let doc = build_styled_document();

    // Paint, inline CSS, browser attrs, and inline HTML tags that must never
    // survive the portable-Markdown body.
    const FORBIDDEN: &[&str] = &[
        "rgb(", "rgba(", "style=", "color:", "background", "data-", "target=",
        "class=", "<a ", "<img", "<span",
    ];

    let portable = render_markdown_node(&doc.root, &MarkdownRenderOptions::default())
        .expect("portable markdown render")
        .output;
    for needle in FORBIDDEN {
        assert!(
            !portable.contains(needle),
            "portable Markdown must drop `{needle}`. body:\n{portable}",
        );
    }
    assert!(
        portable.contains("[Click](https://example.com)"),
        "the link must degrade to its plain Markdown spelling. body:\n{portable}",
    );
    assert!(
        portable.contains("![A](./local.png)"),
        "the image must degrade to its plain Markdown spelling. body:\n{portable}",
    );

    let plus_opts = MarkdownRenderOptions {
        dialect: MarkdownDialect::MarkdownPlus,
        ..MarkdownRenderOptions::default()
    };
    let plus = render_markdown_node(&doc.root, &plus_opts)
        .expect("markdown-plus render")
        .output;
    // MarkdownPlus must not widen a link/image into a browser `<a>`/`<img>`:
    // those browser-only attrs have no MarkdownPlus form, so both stay plain.
    assert!(
        plus.contains("[Click](https://example.com)") && !plus.contains("<a "),
        "MarkdownPlus must keep the link plain (no browser <a>). body:\n{plus}",
    );
    assert!(
        plus.contains("![A](./local.png)") && !plus.contains("<img"),
        "MarkdownPlus must keep the image plain (no browser <img>). body:\n{plus}",
    );
}

/// `InheritedStyle` is the sole text-appearance inheritance path (closeout spec
/// section 5: "`InheritedStyle` is the sole text-appearance inheritance
/// contract"). Proven end-to-end on the browser fold, where a second
/// inheritance path would surface as a re-declared or leaked property:
///
/// - the page foreground color is declared exactly once (on the wrapping
///   element) and reaches descendants through the CSS cascade — the
///   block-quote's child paragraph never re-declares `color`;
/// - box-paint never inherits: the block-quote's `background-color` does not
///   appear on its child paragraph, which carries no `style` at all.
///
/// The `InheritedStyle` resolver's field-level contract (only `color` +
/// `emphasis` inherit; `background` / `border` / geometry do not) is unit-tested
/// in `renderable::tree::inherit`; this pins the same contract through the real
/// styled production fold.
#[test]
fn inherited_style_is_the_sole_text_appearance_path() {
    let doc = build_styled_document();
    let html = render_browser_document_html(&doc, &BrowserRenderOptions::default())
        .expect("browser render")
        .output;

    // The page color (`slate-200`) is declared once and inherits via the
    // cascade; a second push-down path would re-emit it on descendants.
    assert_eq!(
        html.matches("color:rgb(226, 232, 240)").count(),
        1,
        "the page color must be declared once and inherit via the cascade. html:\n{html}",
    );
    // The block-quote's child paragraph carries no style of its own: neither the
    // inherited foreground nor the non-inheriting background leaks onto it.
    assert!(
        html.contains("<p>A quoted paragraph.</p>"),
        "the block-quote child paragraph must carry no inherited/leaked style. html:\n{html}",
    );
}
