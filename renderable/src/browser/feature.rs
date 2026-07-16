//! Page-feature identity, resolution, and asset serialization.
//!
//! A *feature* is a declarable dependency bundle (CSS and/or JavaScript) that a
//! renderable component may request for a browser render. [`PageFeature`] is the
//! type-safe identity and the deduplication key; [`FeatureResolver`] maps a
//! requested feature to its concrete [`FeatureAssets`] for a given
//! [`RenderTarget`]; and the [`resolve_features`] / [`serialize_features_head`] /
//! [`serialize_features_body`] helpers turn a deduplicated feature list into
//! injectable markup.
//!
//! The dependency direction is `darkmatter → renderable`: [`FeatureContext`]
//! carries only renderable-owned values (color mode, resolved semantic colors)
//! so a resolver in another crate can derive theme-aware assets without this
//! crate depending on it.

use std::borrow::Cow;

use thiserror::Error;

use crate::color::ColorMode;
use crate::html::tag::link::LinkTag;
use crate::target::RenderTarget;

/// **PageFeature**
///
/// - Enumerates all of the reusable features which can be added to a
///   HTML page.
/// - Each feature is the identity used to look up the JavaScript, CSS, and
///   `<link>` dependencies that feature requires, and the key used to
///   deduplicate repeated requests in first-seen order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageFeature {
    CopyToClipboard,
    PasteFromClipboard,
    CollapseNestedLists,
    ExpandNestedLists,
    ShowDialog,
    MermaidDiagram,
    DarkMode,
    /// A hover/focus-reachable prompt attached to a navigable link. The
    /// resolver supplies only shared CSS; the emitting component owns the
    /// accessible markup and unique-ID allocation.
    Popover,
}

impl PageFeature {
    /// The stable, kebab-case identity of this feature.
    ///
    /// Used in diagnostics and in the debug-only `data-darkmatter-features`
    /// attribute. It is a stable wire/debug string, not a display label, and
    /// must not change without a compatibility review.
    pub fn name(self) -> &'static str {
        match self {
            PageFeature::CopyToClipboard => "copy-to-clipboard",
            PageFeature::PasteFromClipboard => "paste-from-clipboard",
            PageFeature::CollapseNestedLists => "collapse-nested-lists",
            PageFeature::ExpandNestedLists => "expand-nested-lists",
            PageFeature::ShowDialog => "show-dialog",
            PageFeature::MermaidDiagram => "mermaid-diagram",
            PageFeature::DarkMode => "dark-mode",
            PageFeature::Popover => "popover",
        }
    }
}

/// A feature's resolved browser assets.
///
/// All three slots are optional: an inline `css` block, an inline `js` script,
/// and any `<link>` dependencies. V1 features are inline-only (`links` empty);
/// the field exists so a future head-linked feature can be expressed and so a
/// body-only render can reject it with [`FeatureResolveError::HeadRequired`].
#[derive(Default)]
pub struct FeatureAssets {
    /// Inline CSS for this feature, emitted inside a single `<style>` block.
    pub css: Option<Cow<'static, str>>,
    /// Inline JavaScript for this feature. The [`FeatureScript`] kind selects
    /// classic vs. `type="module"` so an ESM bootstrap is never mis-wrapped.
    pub js: Option<FeatureScript>,
    /// `<link>` dependencies. These require a document `<head>`, so they are
    /// rejected in a body-only render.
    pub links: Vec<LinkTag>,
}

impl FeatureAssets {
    /// Feature assets that are a single inline CSS block.
    pub fn css(css: impl Into<Cow<'static, str>>) -> FeatureAssets {
        FeatureAssets {
            css: Some(css.into()),
            ..FeatureAssets::default()
        }
    }
}

/// A feature's inline script, tagged with its module kind.
///
/// A typed kind is required because wrapping ESM in the classic `<script>`
/// path would make a dynamic `import(...)` bootstrap invalid.
pub enum FeatureScript {
    /// A classic script — emitted as `<script>…</script>`.
    Classic(Cow<'static, str>),
    /// An ES module script — emitted as `<script type="module">…</script>`.
    Module(Cow<'static, str>),
}

impl FeatureScript {
    /// Renders this script to its `<script>` markup.
    pub fn render(&self) -> String {
        match self {
            FeatureScript::Classic(code) => format!("<script>{code}</script>"),
            FeatureScript::Module(code) => format!(r#"<script type="module">{code}</script>"#),
        }
    }
}

/// Renderable-owned context supplied to a [`FeatureResolver`].
///
/// Carries only values a resolver in any crate can use to derive theme-aware
/// assets without depending on Darkmatter. V1 includes the resolved color mode
/// and the resolved semantic colors as `(token-name, css-value)` pairs (empty
/// until a caller — e.g. Darkmatter's full-page path — populates them).
#[derive(Debug, Clone, Default)]
pub struct FeatureContext {
    /// The page's resolved color mode.
    pub color_mode: ColorMode,
    /// Resolved semantic colors as `(name, css-value)` pairs, in declaration
    /// order. Empty when the caller supplies none.
    pub semantic_colors: Vec<(String, String)>,
}

/// The object-safe resolution seam mapping a requested feature to its assets.
///
/// A resolver maps `(PageFeature, RenderTarget, &FeatureContext)` to either
/// resolved [`FeatureAssets`], `Ok(None)` when the feature intentionally has no
/// assets for the target, or a [`FeatureResolveError`]. Composition is explicit
/// delegation — one resolver owns a given feature request — not merging two
/// independently returned bundles.
pub trait FeatureResolver {
    /// Resolves `feature` for `target` under `ctx`.
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(assets))` when the feature resolves to assets.
    /// - `Ok(None)` when the feature intentionally has no assets for `target`
    ///   (e.g. every feature on Markdown/MarkdownPlus/Terminal in v1).
    ///
    /// ## Errors
    ///
    /// - [`FeatureResolveError::UnresolvedFeature`] when a Browser feature is
    ///   requested but this resolver has no assets for it. Silently dropping a
    ///   browser dependency is forbidden.
    fn resolve(
        &self,
        feature: PageFeature,
        target: RenderTarget,
        ctx: &FeatureContext,
    ) -> Result<Option<FeatureAssets>, FeatureResolveError>;
}

/// The generic resolver shipped by `renderable`.
///
/// Resolves [`PageFeature::Popover`] to shared inline CSS on the Browser
/// target, returns `Ok(None)` for every feature on the intentionally asset-free
/// Markdown / MarkdownPlus / Terminal targets, and returns
/// [`FeatureResolveError::UnresolvedFeature`] for any other Browser feature.
///
/// Feature-specific resolvers (e.g. Darkmatter's Mermaid resolver) own their
/// feature and delegate every other variant here.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultFeatureResolver;

impl FeatureResolver for DefaultFeatureResolver {
    fn resolve(
        &self,
        feature: PageFeature,
        target: RenderTarget,
        _ctx: &FeatureContext,
    ) -> Result<Option<FeatureAssets>, FeatureResolveError> {
        match target {
            // Markdown-family and terminal output never receive feature assets
            // in v1: the feature is intentionally asset-free for these targets.
            RenderTarget::Markdown | RenderTarget::MarkdownPlus | RenderTarget::Terminal => {
                Ok(None)
            }
            RenderTarget::Browser => match feature {
                PageFeature::Popover => Ok(Some(FeatureAssets::css(POPOVER_CSS))),
                _ => Err(FeatureResolveError::UnresolvedFeature { feature, target }),
            },
        }
    }
}

/// A failure raised while resolving or placing a feature's assets.
///
/// Both variants name the offending `feature` and `target` so the failure is
/// actionable.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FeatureResolveError {
    /// A Browser feature was requested but the active resolver produced no
    /// assets for it. Dropping a browser dependency silently is forbidden.
    #[error("no resolver produced assets for feature '{}' on the {target:?} target", feature.name())]
    UnresolvedFeature {
        /// The feature that could not be resolved.
        feature: PageFeature,
        /// The render target the request was made for.
        target: RenderTarget,
    },
    /// A body-only render requested a feature that resolves to `<link>`
    /// dependencies, whose document-head semantics an embedder cannot
    /// guarantee.
    #[error("feature '{}' requires a document <head> and cannot be injected into a body-only {target:?} render", feature.name())]
    HeadRequired {
        /// The feature whose assets require a `<head>`.
        feature: PageFeature,
        /// The render target the request was made for.
        target: RenderTarget,
    },
}

/// Deduplicates `features` by variant, preserving first-seen order.
#[must_use]
pub fn dedup_features(features: &[PageFeature]) -> Vec<PageFeature> {
    let mut out: Vec<PageFeature> = Vec::new();
    for &feature in features {
        if !out.contains(&feature) {
            out.push(feature);
        }
    }
    out
}

/// Resolves a feature list to `(feature, assets)` pairs in first-seen order.
///
/// The input is deduplicated by variant before resolution, so a feature
/// requested twice resolves once. Features that resolve to `Ok(None)` are
/// dropped from the result; the pairs are returned in first-seen order so the
/// serializers below can preserve deterministic asset ordering.
///
/// ## Errors
///
/// Propagates any [`FeatureResolveError`] the resolver raises (e.g.
/// [`FeatureResolveError::UnresolvedFeature`] for an unresolved Browser
/// feature).
pub fn resolve_features(
    features: &[PageFeature],
    resolver: &dyn FeatureResolver,
    target: RenderTarget,
    ctx: &FeatureContext,
) -> Result<Vec<(PageFeature, FeatureAssets)>, FeatureResolveError> {
    let mut out = Vec::new();
    for feature in dedup_features(features) {
        if let Some(assets) = resolver.resolve(feature, target, ctx)? {
            out.push((feature, assets));
        }
    }
    Ok(out)
}

/// Serializes resolved feature assets for injection into a document `<head>`.
///
/// Assets are emitted in first-seen feature order; within one feature the order
/// is `<link>`, then `<style>`, then `<script>`. Callers append this after the
/// page's authored links / styles / scripts, so feature code can rely on its
/// own declarations without changing existing output when no feature is
/// requested.
#[must_use]
pub fn serialize_features_head(resolved: &[(PageFeature, FeatureAssets)]) -> String {
    let mut out = String::new();
    for (_feature, assets) in resolved {
        for link in &assets.links {
            out.push_str(&link.render());
        }
        push_inline_assets(&mut out, assets);
    }
    out
}

/// Serializes resolved feature assets for a body-only render.
///
/// Emits `<style>`/`<script>` in first-seen feature order (no `<link>`s, which
/// require a `<head>`). The caller places the result before the body inside a
/// wrapper.
///
/// ## Errors
///
/// Returns [`FeatureResolveError::HeadRequired`] if any resolved feature
/// carries `<link>` dependencies, which cannot be injected into a body-only
/// fragment.
pub fn serialize_features_body(
    resolved: &[(PageFeature, FeatureAssets)],
) -> Result<String, FeatureResolveError> {
    let mut out = String::new();
    for (feature, assets) in resolved {
        if !assets.links.is_empty() {
            return Err(FeatureResolveError::HeadRequired {
                feature: *feature,
                target: RenderTarget::Browser,
            });
        }
        push_inline_assets(&mut out, assets);
    }
    Ok(out)
}

/// Appends a feature's inline `<style>` then `<script>` to `out`.
fn push_inline_assets(out: &mut String, assets: &FeatureAssets) {
    if let Some(css) = &assets.css
        && !css.is_empty()
    {
        out.push_str("<style>");
        out.push_str(css);
        out.push_str("</style>");
    }
    if let Some(js) = &assets.js {
        out.push_str(&js.render());
    }
}

/// Shared CSS for [`PageFeature::Popover`].
///
/// A CSS-only progressive enhancement: the prompt is hidden by default and
/// revealed on hover or keyboard focus of the wrapped link, with a
/// `prefers-reduced-motion` override.
///
/// The prompt element carries the `popover="hint"` attribute so browsers that
/// support the interest/popover invokers get native behavior. Those browsers'
/// UA stylesheet also sets `[popover]{display:none}`, which would defeat the
/// CSS-only fallback; the explicit `display:block` on `.dm-popover-prompt`
/// overrides it (author rules beat the UA sheet), so visibility is governed by
/// this stylesheet's `:hover` / `:focus-within` toggle everywhere. The light
/// panel uses the shared semantic tokens (with light literal fallbacks when the
/// page defines none); under a dark `prefers-color-scheme` the panel switches to
/// a dark palette so a floating tooltip stays legible against an OS-dark page —
/// the semantic tokens are theme-invariant today, so this override, not the
/// tokens, provides dark-mode contrast.
///
/// ## Viewport safety
///
/// The base rule anchors the panel's inline-start edge to the link
/// (`left:0`) and lets it grow to `max-content`, so a link near the **right**
/// edge would overflow the viewport. Two guards prevent that:
///
/// - `max-width:min(20rem,calc(100vw - 1rem))` caps the panel so it can never
///   be wider than the viewport (long prompts wrap instead of overflowing);
/// - the `@supports (position-try-fallbacks:flip-inline)` block re-anchors the
///   panel to the link with CSS anchor positioning and adds `flip-inline`
///   (plus `flip-block`) fallbacks, so a browser that supports it flips the
///   panel to the opposite side when it would otherwise overflow — keeping the
///   bounding box on-screen at either edge.
///
/// Verified in Chromium (Chrome 125+ ships anchor positioning; the geometry is
/// asserted by the `browser_popover_*` tests). Firefox and WebKit have **not**
/// been executed against this CSS; they fall back to the `max-width`-capped,
/// left-anchored layout, which stays on-screen for ordinary inline links but
/// is not edge-flip-verified there.
const POPOVER_CSS: &str = "\
.dm-popover-wrapper{position:relative;display:inline-block;anchor-scope:--dm-popover}\
.dm-popover-wrapper a{anchor-name:--dm-popover}\
.dm-popover-prompt{display:block;position:absolute;left:0;right:auto;top:100%;\
z-index:30;max-width:min(20rem,calc(100vw - 1rem));width:max-content;margin-top:.25rem;\
padding:var(--space-2,.5rem);border-radius:.25rem;\
font-size:.875rem;line-height:1.4;white-space:normal;\
background:var(--color-bg,#fff);color:var(--color-fg,#1a1a1a);\
border:1px solid var(--color-border,#d0d0d0);\
box-shadow:0 2px 8px rgba(0,0,0,.4);\
visibility:hidden;opacity:0;transition:opacity .12s ease}\
@supports (position-try-fallbacks:flip-inline){\
.dm-popover-prompt{position:fixed;left:auto;right:auto;top:auto;bottom:auto;\
position-anchor:--dm-popover;position-area:bottom span-right;\
position-try-fallbacks:flip-inline,flip-block}}\
@media (prefers-color-scheme:dark){.dm-popover-prompt{\
background:#111;color:#eee;border-color:#444}}\
.dm-popover-wrapper:hover .dm-popover-prompt,\
.dm-popover-wrapper:focus-within .dm-popover-prompt{visibility:visible;opacity:1}\
@media(prefers-reduced-motion:reduce){.dm-popover-prompt{transition:none}}";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::attribute::rel::LinkRel;

    #[test]
    fn feature_names_are_stable_kebab_case() {
        assert_eq!(PageFeature::Popover.name(), "popover");
        assert_eq!(PageFeature::MermaidDiagram.name(), "mermaid-diagram");
        assert_eq!(PageFeature::CopyToClipboard.name(), "copy-to-clipboard");
    }

    #[test]
    fn default_resolver_resolves_popover_css_on_browser() {
        let ctx = FeatureContext::default();
        let assets = DefaultFeatureResolver
            .resolve(PageFeature::Popover, RenderTarget::Browser, &ctx)
            .expect("resolve")
            .expect("popover has assets on browser");
        assert!(assets.css.is_some(), "popover resolves to CSS");
        assert!(assets.js.is_none(), "popover is CSS-only in v1");
        assert!(assets.links.is_empty(), "popover is inline");
    }

    #[test]
    fn default_resolver_bypasses_non_browser_targets() {
        let ctx = FeatureContext::default();
        for target in [
            RenderTarget::Markdown,
            RenderTarget::MarkdownPlus,
            RenderTarget::Terminal,
        ] {
            let resolved = DefaultFeatureResolver
                .resolve(PageFeature::Popover, target, &ctx)
                .expect("resolve");
            assert!(
                resolved.is_none(),
                "{target:?} is intentionally asset-free (Ok(None))"
            );
        }
    }

    #[test]
    fn default_resolver_errors_on_unimplemented_browser_feature() {
        let ctx = FeatureContext::default();
        for feature in [PageFeature::MermaidDiagram, PageFeature::DarkMode] {
            // `FeatureAssets` is intentionally not `Debug` (it carries a
            // non-`Debug` `LinkTag`), so match rather than `expect_err`.
            let err = match DefaultFeatureResolver.resolve(feature, RenderTarget::Browser, &ctx) {
                Err(err) => err,
                Ok(_) => panic!("unimplemented browser feature must error"),
            };
            assert_eq!(
                err,
                FeatureResolveError::UnresolvedFeature {
                    feature,
                    target: RenderTarget::Browser,
                }
            );
            // The failure names both the feature and the target.
            let msg = err.to_string();
            assert!(msg.contains(feature.name()), "message names feature: {msg}");
            assert!(msg.contains("Browser"), "message names target: {msg}");
        }
    }

    #[test]
    fn resolve_features_dedups_first_seen() {
        let ctx = FeatureContext::default();
        let resolved = resolve_features(
            &[
                PageFeature::Popover,
                PageFeature::Popover,
                PageFeature::Popover,
            ],
            &DefaultFeatureResolver,
            RenderTarget::Browser,
            &ctx,
        )
        .expect("resolve");
        assert_eq!(resolved.len(), 1, "duplicate feature requests collapse");
        assert_eq!(resolved[0].0, PageFeature::Popover);
    }

    #[test]
    fn classic_and_module_scripts_render_distinctly() {
        assert_eq!(
            FeatureScript::Classic("run()".into()).render(),
            "<script>run()</script>"
        );
        assert_eq!(
            FeatureScript::Module("import('x')".into()).render(),
            r#"<script type="module">import('x')</script>"#
        );
    }

    #[test]
    fn head_serialization_orders_link_css_script_per_feature() {
        let assets = FeatureAssets {
            css: Some("a{}".into()),
            js: Some(FeatureScript::Module("m".into())),
            links: vec![LinkTag::new(LinkRel::Stylesheet, "x.css")],
        };
        let head = serialize_features_head(&[(PageFeature::Popover, assets)]);
        let link_at = head.find("<link").expect("link present");
        let style_at = head.find("<style>").expect("style present");
        let script_at = head.find("<script").expect("script present");
        assert!(link_at < style_at, "link before style: {head}");
        assert!(style_at < script_at, "style before script: {head}");
    }

    #[test]
    fn body_serialization_rejects_head_only_links() {
        let assets = FeatureAssets {
            css: None,
            js: None,
            links: vec![LinkTag::new(LinkRel::Stylesheet, "x.css")],
        };
        let err = serialize_features_body(&[(PageFeature::MermaidDiagram, assets)])
            .expect_err("links in a body-only render must be rejected");
        assert_eq!(
            err,
            FeatureResolveError::HeadRequired {
                feature: PageFeature::MermaidDiagram,
                target: RenderTarget::Browser,
            }
        );
    }

    #[test]
    fn body_serialization_emits_inline_css_only() {
        let resolved = resolve_features(
            &[PageFeature::Popover],
            &DefaultFeatureResolver,
            RenderTarget::Browser,
            &FeatureContext::default(),
        )
        .expect("resolve");
        let body = serialize_features_body(&resolved).expect("no links to reject");
        assert!(body.contains("<style>"), "popover CSS present: {body}");
        assert!(!body.contains("<link"), "no links in body-only: {body}");
    }
}
