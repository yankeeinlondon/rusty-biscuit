//! Darkmatter's browser [`FeatureResolver`], the single owner of Mermaid
//! browser assets.
//!
//! [`DarkmatterFeatureResolver`] resolves [`PageFeature::MermaidDiagram`] to the
//! theme-aware CSS variables and the inline ESM bootstrap that make interactive
//! Mermaid actually run, and delegates every other feature to
//! [`DefaultFeatureResolver`]. Composition is explicit delegation — one resolver
//! owns a feature request — so there is never an ambiguous merge of two asset
//! bundles (spec "Resolver composition is explicit delegation, not merging").
//!
//! The bootstrap absorbs the logic of the retired standalone Mermaid HTML
//! snippet, so this resolver is the *only* owner of Mermaid browser delivery.

use renderable::browser::feature::{
    DefaultFeatureResolver, FeatureAssets, FeatureContext, FeatureResolveError, FeatureResolver,
    FeatureScript, PageFeature,
};
use renderable::target::RenderTarget;

use crate::markdown::highlighting::{ColorMode, ThemePair};
use crate::mermaid::theme::{MermaidTheme, mermaid_theme_for_syntect};

/// The exact, spec-owned Mermaid version delivered to the browser.
///
/// It is a fully pinned version — **never** a floating major tag such as
/// `mermaid@11` — so the primary (jsDelivr) and fallback (unpkg) dynamic ESM
/// imports load byte-identical code (spec acceptance criterion 11).
pub const MERMAID_VERSION: &str = "11.6.0";

/// The jsDelivr origin the Mermaid ESM module is imported from first.
pub const MERMAID_CDN_PRIMARY_ORIGIN: &str = "https://cdn.jsdelivr.net";

/// The unpkg origin the Mermaid ESM module falls back to.
pub const MERMAID_CDN_FALLBACK_ORIGIN: &str = "https://unpkg.com";

/// Darkmatter's browser feature resolver.
///
/// Owns [`PageFeature::MermaidDiagram`] on the [`RenderTarget::Browser`] target,
/// resolving it to theme-aware CSS variables plus the inline ESM bootstrap
/// derived from the document's resolved theme. Every other feature — and every
/// non-Browser target — is delegated to [`DefaultFeatureResolver`], so the
/// dependency direction stays `darkmatter → renderable` and generic features
/// (e.g. Popover) keep their shared implementation.
///
/// The theme pair is captured at construction from the document's resolved
/// rendering theme; the emitted CSS declares both the light and the dark
/// palette behind a `prefers-color-scheme` query, so the diagram tracks the
/// reader's system theme without a second render.
#[derive(Debug, Clone)]
pub struct DarkmatterFeatureResolver {
    theme_pair: ThemePair,
    default: DefaultFeatureResolver,
}

impl Default for DarkmatterFeatureResolver {
    fn default() -> DarkmatterFeatureResolver {
        DarkmatterFeatureResolver::new(ThemePair::OneHalf)
    }
}

impl DarkmatterFeatureResolver {
    /// Builds a resolver whose Mermaid assets derive from `theme_pair`.
    #[must_use]
    pub fn new(theme_pair: ThemePair) -> DarkmatterFeatureResolver {
        DarkmatterFeatureResolver {
            theme_pair,
            default: DefaultFeatureResolver,
        }
    }

    /// The Mermaid browser assets for this resolver's theme.
    fn mermaid_assets(&self) -> FeatureAssets {
        let light = mermaid_theme_for_syntect(self.theme_pair, ColorMode::Light);
        let dark = mermaid_theme_for_syntect(self.theme_pair, ColorMode::Dark);
        FeatureAssets {
            css: Some(mermaid_feature_css(light, dark).into()),
            js: Some(FeatureScript::Module(mermaid_bootstrap().into())),
            links: Vec::new(),
        }
    }
}

impl FeatureResolver for DarkmatterFeatureResolver {
    fn resolve(
        &self,
        feature: PageFeature,
        target: RenderTarget,
        ctx: &FeatureContext,
    ) -> Result<Option<FeatureAssets>, FeatureResolveError> {
        match feature {
            // Darkmatter owns Mermaid browser delivery. Non-Browser targets stay
            // asset-free: features never alter terminal or Markdown-family output.
            PageFeature::MermaidDiagram => match target {
                RenderTarget::Browser => Ok(Some(self.mermaid_assets())),
                RenderTarget::Markdown | RenderTarget::MarkdownPlus | RenderTarget::Terminal => {
                    Ok(None)
                }
            },
            // Every other feature (e.g. Popover) is the generic resolver's.
            _ => self.default.resolve(feature, target, ctx),
        }
    }
}

/// Builds the Mermaid theme CSS variables for both color modes.
///
/// Emits a `:root` block for the light palette and a
/// `@media (prefers-color-scheme: dark)` override for the dark palette, so the
/// diagram tracks the reader's system theme. The returned string is the raw CSS
/// body with **no** surrounding `<style>` element — the feature serializer wraps
/// it.
fn mermaid_feature_css(light: &MermaidTheme, dark: &MermaidTheme) -> String {
    let opt = |val: &Option<String>, fallback: &str| -> String {
        val.clone().unwrap_or_else(|| fallback.to_string())
    };

    let vars = |theme: &MermaidTheme, dark_mode: bool| -> String {
        let (text_fb, line_fb, bkg_fb) = if dark_mode {
            ("#ffffff", "#cccccc", "#1e1e1e")
        } else {
            ("#000000", "#333333", "#ececff")
        };
        format!(
            "--mermaid-background:{bg};\
--mermaid-primary-color:{pc};\
--mermaid-secondary-color:{sc};\
--mermaid-tertiary-color:{tc};\
--mermaid-primary-border-color:{pbc};\
--mermaid-secondary-border-color:{sbc};\
--mermaid-tertiary-border-color:{tbc};\
--mermaid-primary-text-color:{ptc};\
--mermaid-secondary-text-color:{stc};\
--mermaid-tertiary-text-color:{ttc};\
--mermaid-line-color:{lc};\
--mermaid-text-color:{txc};\
--mermaid-main-bkg:{mb};\
--mermaid-node-border:{nb}",
            bg = theme.background,
            pc = theme.primary_color,
            sc = opt(&theme.secondary_color, "#6699cc"),
            tc = opt(&theme.tertiary_color, "#99ccff"),
            pbc = opt(&theme.primary_border_color, "#9370db"),
            sbc = opt(&theme.secondary_border_color, "#6699cc"),
            tbc = opt(&theme.tertiary_border_color, "#99ccff"),
            ptc = opt(&theme.primary_text_color, text_fb),
            stc = opt(&theme.secondary_text_color, text_fb),
            ttc = opt(&theme.tertiary_text_color, text_fb),
            lc = opt(&theme.line_color, line_fb),
            txc = opt(&theme.text_color, line_fb),
            mb = opt(&theme.main_bkg, bkg_fb),
            nb = opt(&theme.node_border, "#9370db"),
        )
    };

    format!(
        ":root{{{light_vars}}}@media (prefers-color-scheme: dark){{:root{{{dark_vars}}}}}",
        light_vars = vars(light, false),
        dark_vars = vars(dark, true),
    )
}

/// The inline ESM bootstrap that loads Mermaid and renders `.mermaid` elements.
///
/// Dynamically imports the [exact `MERMAID_VERSION`](MERMAID_VERSION) from
/// jsDelivr, retrying the identical version from unpkg on failure. Both origins
/// are named literally so a Content Security Policy can permit exactly them.
/// Initialization targets only `.mermaid` elements; a total load failure logs a
/// single concise `console.error` and leaves the escaped diagram source visible
/// (the `<pre class="mermaid">` body), never an empty diagram.
///
/// The returned string is the raw module body with no surrounding `<script>`
/// element — the [`FeatureScript::Module`] renderer wraps it as
/// `<script type="module">`.
fn mermaid_bootstrap() -> String {
    format!(
        "const P='{origin_primary}/npm/mermaid@{version}/dist/mermaid.esm.min.mjs';\
const F='{origin_fallback}/mermaid@{version}/dist/mermaid.esm.min.mjs';\
let m;\
try{{m=(await import(P)).default;}}\
catch(e1){{try{{m=(await import(F)).default;}}\
catch(e2){{console.error('darkmatter: Mermaid failed to load from jsDelivr and unpkg',e1,e2);}}}}\
if(m){{m.initialize({{startOnLoad:false}});await m.run({{querySelector:'.mermaid'}});}}",
        origin_primary = MERMAID_CDN_PRIMARY_ORIGIN,
        origin_fallback = MERMAID_CDN_FALLBACK_ORIGIN,
        version = MERMAID_VERSION,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_mermaid_to_css_and_module_script_on_browser() {
        let resolver = DarkmatterFeatureResolver::default();
        // `FeatureAssets` is intentionally not `Debug`, so match rather than
        // `unwrap`.
        let assets = match resolver.resolve(
            PageFeature::MermaidDiagram,
            RenderTarget::Browser,
            &FeatureContext::default(),
        ) {
            Ok(Some(assets)) => assets,
            Ok(None) => panic!("mermaid must resolve to assets on browser"),
            Err(_) => panic!("mermaid resolution must not error on browser"),
        };
        assert!(assets.css.is_some(), "mermaid resolves to CSS variables");
        assert!(
            matches!(assets.js, Some(FeatureScript::Module(_))),
            "mermaid bootstrap is an ES module, never classic",
        );
        assert!(assets.links.is_empty(), "mermaid assets are inline");
    }

    #[test]
    fn mermaid_bootstrap_names_both_cdn_origins_and_exact_version() {
        let js = mermaid_bootstrap();
        assert!(js.contains(MERMAID_CDN_PRIMARY_ORIGIN), "jsDelivr origin named");
        assert!(js.contains(MERMAID_CDN_FALLBACK_ORIGIN), "unpkg origin named");
        assert!(
            js.contains(&format!("mermaid@{MERMAID_VERSION}")),
            "the exact version is pinned",
        );
        // A floating major tag would be `mermaid@11/…` (no pinned minor/patch);
        // the exact `11.6.0` version legitimately contains `mermaid@11.`.
        assert!(!js.contains("mermaid@11/"), "no floating major tag: {js}");
        assert!(
            js.contains(".mermaid"),
            "initialization targets only .mermaid elements",
        );
        assert!(js.contains("console.error"), "a load failure is logged");
    }

    #[test]
    fn mermaid_css_declares_light_and_dark_palettes() {
        let light = mermaid_theme_for_syntect(ThemePair::OneHalf, ColorMode::Light);
        let dark = mermaid_theme_for_syntect(ThemePair::OneHalf, ColorMode::Dark);
        let css = mermaid_feature_css(light, dark);
        assert!(css.contains("--mermaid-primary-color"), "declares mermaid vars");
        assert!(
            css.contains("prefers-color-scheme: dark"),
            "carries a dark override",
        );
    }

    #[test]
    fn delegates_popover_to_default_resolver() {
        let resolver = DarkmatterFeatureResolver::default();
        let assets = resolver
            .resolve(
                PageFeature::Popover,
                RenderTarget::Browser,
                &FeatureContext::default(),
            )
            .expect("resolve popover")
            .expect("popover has assets on browser");
        assert!(assets.css.is_some(), "delegated Popover keeps its shared CSS");
    }

    #[test]
    fn mermaid_is_asset_free_off_browser() {
        let resolver = DarkmatterFeatureResolver::default();
        for target in [
            RenderTarget::Markdown,
            RenderTarget::MarkdownPlus,
            RenderTarget::Terminal,
        ] {
            let resolved = resolver
                .resolve(PageFeature::MermaidDiagram, target, &FeatureContext::default())
                .expect("resolve");
            assert!(resolved.is_none(), "mermaid is asset-free on {target:?}");
        }
    }
}
