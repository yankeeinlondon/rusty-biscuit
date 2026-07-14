//! Phase 1 baseline freeze for the Style Features implementation
//! (`darkmatter/features/2026-07-09-features/`).
//!
//! These preserve the low-level `renderable` contracts the implementation builds
//! on. Components request [`PageFeature`]s and pages dedup them; fragment-only
//! `render_browser_node` output collects those requests but does not resolve or
//! inject assets. Complete-document paths resolve requests through their
//! installed resolver, and Darkmatter installs its production Mermaid resolver.
//! All fixtures are network-free and deterministic.

use renderable::browser::feature::PageFeature;
use renderable::browser::fragment::BrowserFragment;
use renderable::html::HtmlPage;
use renderable::html::tag::BlockTag;
use renderable::tree::{
    BrowserRenderOptions, Diagnostic, Document, DocumentMetadata, GraphicsMode, RenderNode,
    Rendered, SourceRegistry, render_browser_document, render_browser_document_html,
    render_browser_node,
};

// ---------------------------------------------------------------------------
// Deterministic fixtures
// ---------------------------------------------------------------------------

/// A single Mermaid fence render node.
fn mermaid_node(source: &str) -> RenderNode {
    RenderNode::code(Some("mermaid".into()), None, source)
}

/// A document with two distinct Mermaid fences — the dedup fixture. With a
/// Mermaid-owning resolver installed, the two blocks dedup to exactly one
/// injected `<script type="module">` bootstrap (script-only; the palette rides
/// Mermaid `themeVariables`, not CSS — spec acceptance criterion 1). This
/// baseline test instead exercises the *default* resolver, which leaves the
/// feature unresolved.
fn two_mermaid_document() -> Document {
    Document {
        sources: SourceRegistry::default(),
        metadata: DocumentMetadata::default(),
        root: RenderNode::root(vec![
            mermaid_node("graph TD; A --> B"),
            RenderNode::paragraph(vec![RenderNode::text("between")]),
            mermaid_node("sequenceDiagram; A ->> B: hi"),
        ]),
    }
}

/// Interactive Mermaid options at the full `Rich` tier (the only tier that
/// reaches the `<pre class="mermaid">` client-side path).
fn interactive_rich_opts() -> BrowserRenderOptions {
    BrowserRenderOptions {
        graphics_mode: GraphicsMode::Rich,
        mermaid_mode: renderable::tree::BrowserMermaidMode::Interactive,
        ..BrowserRenderOptions::default()
    }
}

// ---------------------------------------------------------------------------
// Low-level Mermaid defaults
// ---------------------------------------------------------------------------

/// `BrowserRenderOptions::default()` keeps Mermaid as a plain code block and
/// injects no interactive assets. Phase 4/5 keep this low-level default; only
/// Darkmatter's full-page path opts into Interactive.
#[test]
fn default_browser_mermaid_mode_is_code() {
    let out = render_browser_node(&mermaid_node("graph TD; A --> B"), &BrowserRenderOptions::default())
        .expect("render")
        .output
        .render();
    assert!(
        out.contains(r#"<pre><code class="language-mermaid">"#),
        "default mermaid must be a plain code block, got: {out}"
    );
    assert!(
        !out.contains(r#"<pre class="mermaid">"#),
        "default must not emit the interactive container, got: {out}"
    );
    assert!(
        !out.contains("<script"),
        "default must inject no script, got: {out}"
    );
}

/// `render_browser_node` emits the Interactive container and collects its
/// feature request, but this composable fragment path does not resolve or inject
/// assets. An outer document/page renderer owns resolution and bootstrap
/// placement.
#[test]
fn interactive_mermaid_fragment_collects_feature_without_injecting_script() {
    let out = render_browser_node(&mermaid_node("graph TD; A --> B"), &interactive_rich_opts())
        .expect("render")
        .output
        .render();
    assert!(
        out.contains(r#"<pre class="mermaid">"#),
        "interactive must emit the mermaid container, got: {out}"
    );
    assert!(
        !out.contains("<script"),
        "fragment rendering must not inject the outer page's bootstrap, got: {out}"
    );
    assert!(
        !out.contains("jsdelivr") && !out.contains("unpkg") && !out.contains("mermaid.esm"),
        "fragment rendering must not resolve the page-level CDN import, got: {out}"
    );
}

/// Two interactive Mermaid fences in a whole document now *request* the Mermaid
/// feature (Phase 3). The low-level [`DefaultFeatureResolver`] does not own it —
/// only Darkmatter's resolver does (Phase 5) — so both the fragment and
/// streaming full-document paths fail with `UnresolvedFeature` rather than
/// silently emitting an inert `<pre class="mermaid">`. This replaces the Phase 1
/// "injects nothing" baseline: an unresolved browser feature is a hard error
/// (spec acceptance criterion 9).
#[test]
fn two_interactive_mermaid_are_unresolved_by_default_resolver() {
    use renderable::browser::feature::{FeatureResolveError, PageFeature};
    use renderable::target::RenderTarget;
    use renderable::tree::RenderError;

    let doc = two_mermaid_document();
    let opts = interactive_rich_opts();

    for result in [
        render_browser_document(&doc, &opts).err(),
        render_browser_document_html(&doc, &opts).err(),
    ] {
        match result {
            Some(RenderError::FeatureResolution(FeatureResolveError::UnresolvedFeature {
                feature,
                target,
            })) => {
                assert_eq!(feature, PageFeature::MermaidDiagram);
                assert_eq!(target, RenderTarget::Browser);
            }
            other => panic!("expected UnresolvedFeature for interactive mermaid, got {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// No-feature page head
// ---------------------------------------------------------------------------

/// A page whose only component requests no feature keeps the head to its fixed
/// pre-feature shape: charset first, no injected feature `<script>`/CSS.
#[test]
fn no_feature_page_head_injects_no_assets() {
    let body = BrowserFragment::new()
        .define_as_block_tag(BlockTag::P, "")
        .add_child(renderable::browser::fragment::ComposableNode::TextFragment(
            "plain".to_string(),
        ))
        .finalize();
    let html = HtmlPage::from(body).render().expect("no-feature page renders");
    assert!(
        html.starts_with("<!DOCTYPE html><html><head><meta charset=\"utf-8\">"),
        "head must open with the fixed charset meta, got: {html}"
    );
    assert!(
        !html.contains("<script"),
        "a no-feature page injects no script, got: {html}"
    );
    assert!(
        !html.contains(r#"<pre class="mermaid">"#),
        "a no-feature page has no mermaid container, got: {html}"
    );
}

// ---------------------------------------------------------------------------
// Feature dedup rollup (the dedup identity carried forward)
// ---------------------------------------------------------------------------

/// `HtmlPage::features()` de-dups by `PageFeature` variant in first-seen order.
/// This is the deduplication identity the whole feature builds on (spec
/// "Deduplication"): two fragments requesting the same feature collapse to one,
/// and distinct features preserve first-seen order.
#[test]
fn features_rollup_dedups_by_variant_first_seen() {
    let first = BrowserFragment::new()
        .define_as_text_fragment("a")
        .add_feature(PageFeature::MermaidDiagram)
        .finalize();
    let second = BrowserFragment::new()
        .define_as_text_fragment("b")
        .add_feature(PageFeature::DarkMode)
        .finalize();
    let third = BrowserFragment::new()
        .define_as_text_fragment("c")
        .add_feature(PageFeature::MermaidDiagram)
        .finalize();

    let page = HtmlPage::from_fragments(vec![first, second, third]);
    assert_eq!(
        page.features(),
        vec![PageFeature::MermaidDiagram, PageFeature::DarkMode],
        "features must dedup by variant in first-seen order"
    );
}

// ---------------------------------------------------------------------------
// Rendered<T> side channel
// ---------------------------------------------------------------------------

/// `Rendered::map` preserves the diagnostics side channel. Phase 2 adds a
/// parallel `features` channel that `map` must also preserve; this pins the
/// current contract so that addition is proven additive.
#[test]
fn rendered_map_preserves_diagnostics_side_channel() {
    let mut rendered = Rendered::new(2);
    rendered
        .diagnostics
        .push(Diagnostic::lossy("approximated", None));
    let mapped = rendered.map(|n| n * 3);
    assert_eq!(mapped.output, 6);
    assert_eq!(mapped.diagnostics.len(), 1);
}
