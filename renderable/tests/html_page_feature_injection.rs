//! Feature injection on the public `HtmlPage` render paths.
//!
//! Regression cover for review-1: `HtmlPage::render` must resolve and inject a
//! component's requested browser features — or fail loudly — on *every* public
//! constructor (`From`, `from_fragments`, and the `BrowserRenderable` promotion
//! path), not only through `render_browser_document`. Before the fix these paths
//! rendered the precomputed (empty) feature head and silently dropped every
//! requested dependency.

use std::cell::Cell;
use std::rc::Rc;

use renderable::browser::feature::{
    DefaultFeatureResolver, FeatureAssets, FeatureContext, FeatureResolveError, FeatureResolver,
    PageFeature,
};
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::html::HtmlPage;
use renderable::html::tag::BlockTag;
use renderable::target::RenderTarget;
use renderable::tree::{
    BrowserMermaidMode, BrowserRenderOptions, Document, DocumentMetadata, GraphicsMode, RenderNode,
    SourceRegistry, render_browser_document,
};

/// A component fragment requesting a single browser feature.
fn fragment_requesting(feature: PageFeature) -> BrowserFragment<Ready> {
    BrowserFragment::new()
        .define_as_text_fragment("body")
        .add_feature(feature)
        .finalize()
}

/// `HtmlPage::from(fragment).render()` resolves the component's Popover request
/// through the default resolver and injects the shared CSS — the exact dead-end
/// the feature spec set out to remove.
#[test]
fn direct_html_page_injects_default_popover_asset() {
    let page = HtmlPage::from(fragment_requesting(PageFeature::Popover));
    let html = page
        .render()
        .expect("popover resolves under the default resolver");
    assert!(
        html.contains(".dm-popover-wrapper{"),
        "default resolver must inject popover CSS on the direct path: {html}"
    );
}

/// A feature requested by only one of several `from_fragments` siblings is
/// injected exactly once, in a single `<style>` block.
#[test]
fn popover_css_injected_exactly_once_across_fragments() {
    let page = HtmlPage::from_fragments(vec![
        fragment_requesting(PageFeature::Popover),
        fragment_requesting(PageFeature::Popover),
    ]);
    let html = page.render().expect("popover resolves");
    assert_eq!(
        html.matches(".dm-popover-wrapper{").count(),
        1,
        "popover CSS must be injected exactly once: {html}"
    );
}

/// A feature requested by a *nested* component bubbles up through composition
/// and is injected on the direct render path.
#[test]
fn nested_fragment_feature_bubbles_up_and_injects() {
    let child = fragment_requesting(PageFeature::Popover);
    let parent = BrowserFragment::new()
        .define_as_block_tag(BlockTag::Div, "wrap")
        .add_component(child)
        .finalize();
    let page = HtmlPage::from_fragments(vec![parent]);
    let html = page.render().expect("nested popover resolves");
    assert!(
        html.contains(".dm-popover-wrapper{"),
        "a feature requested by a nested component must inject: {html}"
    );
}

/// An installed custom resolver is honored on the direct render path: it owns a
/// feature the default resolver cannot resolve and its assets are injected.
#[test]
fn custom_resolver_is_honored_on_direct_render() {
    struct DarkModeResolver;
    impl FeatureResolver for DarkModeResolver {
        fn resolve(
            &self,
            feature: PageFeature,
            target: RenderTarget,
            ctx: &FeatureContext,
        ) -> Result<Option<FeatureAssets>, FeatureResolveError> {
            match (feature, target) {
                (PageFeature::DarkMode, RenderTarget::Browser) => {
                    Ok(Some(FeatureAssets::css(".dm-dark{color:#eee}")))
                }
                _ => DefaultFeatureResolver.resolve(feature, target, ctx),
            }
        }
    }

    let mut page = HtmlPage::from(fragment_requesting(PageFeature::DarkMode));
    page.set_feature_resolver(Rc::new(DarkModeResolver));
    let html = page.render().expect("custom resolver owns DarkMode");
    assert!(
        html.contains(".dm-dark{color:#eee}"),
        "custom resolver assets must be injected: {html}"
    );
}

/// A `FeatureResolver` that counts how many times it is asked to resolve a
/// browser feature. Used to prove the `render_browser_document` path resolves
/// each deduplicated feature exactly once across the eager entry-point check and
/// the final `HtmlPage::render`.
struct CountingResolver {
    calls: Rc<Cell<usize>>,
}

impl FeatureResolver for CountingResolver {
    fn resolve(
        &self,
        feature: PageFeature,
        target: RenderTarget,
        ctx: &FeatureContext,
    ) -> Result<Option<FeatureAssets>, FeatureResolveError> {
        match (feature, target) {
            (PageFeature::MermaidDiagram, RenderTarget::Browser) => {
                self.calls.set(self.calls.get() + 1);
                // Synthetic CSS so injection is observable — production Mermaid is
                // script-only (`css: None`); this fixture only counts resolver calls.
                Ok(Some(FeatureAssets::css(".dm-mermaid{}")))
            }
            _ => DefaultFeatureResolver.resolve(feature, target, ctx),
        }
    }
}

/// The `render_browser_document` path resolves each deduplicated feature exactly
/// once. `render_browser_document` eagerly resolves to surface unresolved
/// features at its fallible entry point, then the returned page's `render`
/// resolves again for the `<head>`; the memoized head must collapse those two
/// into a single resolver invocation per feature. Before memoization the counter
/// would read 2. Two Mermaid fences dedup to one `MermaidDiagram` feature, so the
/// expected count is exactly one.
#[test]
fn feature_resolution_runs_once_per_deduplicated_feature() {
    let calls = Rc::new(Cell::new(0usize));
    let doc = Document {
        sources: SourceRegistry::default(),
        metadata: DocumentMetadata::default(),
        root: RenderNode::root(vec![
            RenderNode::code(Some("mermaid".into()), None, "graph TD; A --> B"),
            RenderNode::code(Some("mermaid".into()), None, "sequenceDiagram; A ->> B: hi"),
        ]),
    };
    let opts = BrowserRenderOptions {
        graphics_mode: GraphicsMode::Rich,
        mermaid_mode: BrowserMermaidMode::Interactive,
        feature_resolver: Rc::new(CountingResolver {
            calls: Rc::clone(&calls),
        }),
        ..BrowserRenderOptions::default()
    };

    let rendered = render_browser_document(&doc, &opts).expect("mermaid resolves");
    assert_eq!(
        calls.get(),
        1,
        "the eager entry-point resolution must populate the cache: {}",
        calls.get()
    );

    let html = rendered.output.render().expect("final render resolves");
    assert!(
        html.contains(".dm-mermaid{}"),
        "resolved mermaid CSS must be injected on the final render: {html}"
    );
    assert_eq!(
        calls.get(),
        1,
        "the final render must reuse the memoized head, not resolve a second time"
    );
}

/// A requested browser feature the installed resolver cannot resolve fails the
/// render with `UnresolvedFeature`; a declared browser dependency is never
/// silently dropped. `DefaultFeatureResolver` owns only Popover on Browser, so
/// DarkMode is unresolved.
#[test]
fn unresolved_browser_feature_fails_the_render() {
    let page = HtmlPage::from(fragment_requesting(PageFeature::DarkMode));
    let err = match page.render() {
        Err(err) => err,
        Ok(html) => panic!("an unresolved browser feature must fail the render, got: {html}"),
    };
    assert_eq!(
        err,
        FeatureResolveError::UnresolvedFeature {
            feature: PageFeature::DarkMode,
            target: RenderTarget::Browser,
        }
    );
}
