//! Darkmatter's browser [`FeatureResolver`], the single owner of Mermaid
//! browser assets.
//!
//! [`DarkmatterFeatureResolver`] resolves [`PageFeature::MermaidDiagram`] to the
//! inline ESM bootstrap that makes interactive Mermaid actually run, and
//! delegates every other feature to [`DefaultFeatureResolver`]. Composition is
//! explicit delegation — one resolver owns a feature request — so there is never
//! an ambiguous merge of two asset bundles (spec "Resolver composition is
//! explicit delegation, not merging").
//!
//! The theme reaches the rendered SVG through Mermaid's own configuration: the
//! bootstrap calls `mermaid.initialize` with `theme: 'base'` and a
//! `themeVariables` object built from the resolved palette. That is the only
//! documented Mermaid theming input — arbitrary CSS custom properties are *not*
//! read by Mermaid's rendering engine, so none are emitted. See
//! <https://mermaid.js.org/config/theming.html>.
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
/// resolving it to the inline ESM bootstrap that initializes Mermaid with a
/// `themeVariables` palette derived from the document's resolved theme. Every
/// other feature — and every non-Browser target — is delegated to
/// [`DefaultFeatureResolver`], so the dependency direction stays
/// `darkmatter → renderable` and generic features (e.g. Popover) keep their
/// shared implementation.
///
/// The theme pair is captured at construction; the color mode is read at
/// resolve time from [`FeatureContext::color_mode`]. A **single** palette is
/// resolved for that mode and baked into `mermaid.initialize` — Mermaid renders
/// its SVG once at that fixed palette. There is no live `prefers-color-scheme`
/// switch: Mermaid's SVG colors are chosen at init and would require a JS
/// re-render to change, which v1 does not do.
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

    /// The Mermaid browser assets for this resolver's theme and the resolved
    /// color mode carried by `ctx`.
    ///
    /// The palette is delivered exclusively through Mermaid's `themeVariables`
    /// configuration (baked into the bootstrap's `mermaid.initialize` call), so
    /// there is no `css` slot — Mermaid does not read CSS custom properties.
    fn mermaid_assets(&self, ctx: &FeatureContext) -> FeatureAssets {
        let mode = mermaid_color_mode(ctx.color_mode);
        let theme = mermaid_theme_for_syntect(self.theme_pair, mode);
        FeatureAssets {
            css: None,
            js: Some(FeatureScript::Module(mermaid_bootstrap(theme, mode).into())),
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
                RenderTarget::Browser => Ok(Some(self.mermaid_assets(ctx))),
                RenderTarget::Markdown | RenderTarget::MarkdownPlus | RenderTarget::Terminal => {
                    Ok(None)
                }
            },
            // Every other feature (e.g. Popover) is the generic resolver's.
            _ => self.default.resolve(feature, target, ctx),
        }
    }
}

/// Maps a renderable color mode to the [`ColorMode`] the Mermaid theme lookup
/// uses. `Unknown` is carried through and resolved to the dark palette by
/// [`mermaid_theme_for_syntect`].
fn mermaid_color_mode(mode: renderable::color::ColorMode) -> ColorMode {
    match mode {
        renderable::color::ColorMode::Light => ColorMode::Light,
        renderable::color::ColorMode::Dark => ColorMode::Dark,
        renderable::color::ColorMode::Unknown => ColorMode::Unknown,
    }
}

/// Builds Mermaid's `themeVariables` object body from a resolved palette.
///
/// The keys are Mermaid's documented base-theme theme-variable names (camelCase)
/// — the only inputs Mermaid's theming engine reads — and each value is a
/// single-quoted JS string. Missing optional palette entries fall back to
/// mode-appropriate defaults. The returned string is the object *body* (no
/// surrounding braces); the bootstrap wraps it.
fn mermaid_theme_variables(theme: &MermaidTheme, dark_mode: bool) -> String {
    // Colors here originate from static, library-owned palettes, but escape
    // defensively so a future palette source can never break out of the JS
    // string literal.
    let js = |val: &str| -> String { val.replace('\\', "\\\\").replace('\'', "\\'") };
    let opt = |val: &Option<String>, fallback: &str| -> String {
        js(val.as_deref().unwrap_or(fallback))
    };

    let (text_fb, line_fb, bkg_fb) = if dark_mode {
        ("#ffffff", "#cccccc", "#1e1e1e")
    } else {
        ("#000000", "#333333", "#ececff")
    };

    format!(
        "background:'{bg}',\
primaryColor:'{pc}',\
secondaryColor:'{sc}',\
tertiaryColor:'{tc}',\
primaryBorderColor:'{pbc}',\
secondaryBorderColor:'{sbc}',\
tertiaryBorderColor:'{tbc}',\
primaryTextColor:'{ptc}',\
secondaryTextColor:'{stc}',\
tertiaryTextColor:'{ttc}',\
lineColor:'{lc}',\
textColor:'{txc}',\
mainBkg:'{mb}',\
nodeBorder:'{nb}'",
        bg = js(&theme.background),
        pc = js(&theme.primary_color),
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
}

/// The inline ESM bootstrap that loads Mermaid and renders `.mermaid` elements.
///
/// Dynamically imports the [exact `MERMAID_VERSION`](MERMAID_VERSION) from
/// jsDelivr, retrying the identical version from unpkg on failure. Both origins
/// are named literally so a Content Security Policy can permit exactly them.
/// Initialization uses `theme: 'base'` plus a `themeVariables` object built from
/// `theme` — Mermaid's only documented theming input — so the rendered SVG
/// follows the resolved palette. `mode` selects the fallback colors for palette
/// entries the theme leaves unset. Initialization targets only `.mermaid`
/// elements; a total load failure logs a single concise `console.error` and
/// leaves the escaped diagram source visible (the `<pre class="mermaid">` body),
/// never an empty diagram.
///
/// The returned string is the raw module body with no surrounding `<script>`
/// element — the [`FeatureScript::Module`] renderer wraps it as
/// `<script type="module">`.
fn mermaid_bootstrap(theme: &MermaidTheme, mode: ColorMode) -> String {
    let dark_mode = !matches!(mode, ColorMode::Light);
    let theme_variables = mermaid_theme_variables(theme, dark_mode);
    format!(
        "const P='{origin_primary}/npm/mermaid@{version}/dist/mermaid.esm.min.mjs';\
const F='{origin_fallback}/mermaid@{version}/dist/mermaid.esm.min.mjs';\
let m;\
try{{m=(await import(P)).default;}}\
catch(e1){{try{{m=(await import(F)).default;}}\
catch(e2){{console.error('darkmatter: Mermaid failed to load from jsDelivr and unpkg',e1,e2);}}}}\
if(m){{m.initialize({{startOnLoad:false,theme:'base',themeVariables:{{{theme_variables}}}}});\
await m.run({{querySelector:'.mermaid'}});}}",
        origin_primary = MERMAID_CDN_PRIMARY_ORIGIN,
        origin_fallback = MERMAID_CDN_FALLBACK_ORIGIN,
        version = MERMAID_VERSION,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_mermaid_to_module_script_without_css_on_browser() {
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
        // The palette rides Mermaid's `themeVariables`, never CSS custom
        // properties (which Mermaid does not read), so there is no CSS slot.
        assert!(
            assets.css.is_none(),
            "mermaid emits no CSS — the theme is delivered via themeVariables",
        );
        assert!(
            matches!(assets.js, Some(FeatureScript::Module(_))),
            "mermaid bootstrap is an ES module, never classic",
        );
        assert!(assets.links.is_empty(), "mermaid assets are inline");
    }

    #[test]
    fn mermaid_bootstrap_names_both_cdn_origins_and_exact_version() {
        let theme = mermaid_theme_for_syntect(ThemePair::OneHalf, ColorMode::Dark);
        let js = mermaid_bootstrap(theme, ColorMode::Dark);
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
            js.contains("querySelector:'.mermaid'"),
            "initialization targets only .mermaid elements",
        );
        assert!(js.contains("console.error"), "a load failure is logged");
    }

    #[test]
    fn mermaid_bootstrap_initializes_base_theme_with_palette_theme_variables() {
        let theme = mermaid_theme_for_syntect(ThemePair::OneHalf, ColorMode::Dark);
        let js = mermaid_bootstrap(theme, ColorMode::Dark);

        // Mermaid's theming engine only reads `theme: 'base'` + `themeVariables`.
        assert!(js.contains("theme:'base'"), "the base theme is selected: {js}");
        assert!(
            js.contains("themeVariables:{"),
            "a themeVariables object is passed: {js}",
        );

        // The resolved palette colors appear as themeVariables *inputs* …
        assert!(
            js.contains(&format!("primaryColor:'{}'", theme.primary_color)),
            "the resolved primary color is a themeVariable input: {js}",
        );
        assert!(
            js.contains(&format!("background:'{}'", theme.background)),
            "the resolved background is a themeVariable input: {js}",
        );

        // … and never as dangling `--mermaid-*` custom properties nothing reads.
        assert!(
            !js.contains("--mermaid-"),
            "no dead --mermaid-* custom properties are emitted: {js}",
        );
    }

    #[test]
    fn mermaid_bootstrap_resolves_one_palette_per_color_mode() {
        let light = mermaid_theme_for_syntect(ThemePair::OneHalf, ColorMode::Light);
        let dark = mermaid_theme_for_syntect(ThemePair::OneHalf, ColorMode::Dark);
        let light_js = mermaid_bootstrap(light, ColorMode::Light);
        let dark_js = mermaid_bootstrap(dark, ColorMode::Dark);

        // Exactly one palette is baked per mode — no `prefers-color-scheme`
        // override, which Mermaid could not act on anyway.
        assert!(
            light_js.contains(&format!("background:'{}'", light.background))
                && !light_js.contains(&format!("background:'{}'", dark.background)),
            "the light bootstrap carries only the light background: {light_js}",
        );
        assert!(
            dark_js.contains(&format!("background:'{}'", dark.background))
                && !dark_js.contains(&format!("background:'{}'", light.background)),
            "the dark bootstrap carries only the dark background: {dark_js}",
        );
        assert!(
            !light_js.contains("prefers-color-scheme"),
            "no media-query palette switch is emitted: {light_js}",
        );
    }

    #[test]
    fn mermaid_color_mode_carries_unknown_to_dark_palette() {
        // `Unknown` resolves through `mermaid_theme_for_syntect` to the dark
        // palette, matching the renderable color-mode default.
        let dark = mermaid_theme_for_syntect(ThemePair::OneHalf, ColorMode::Dark);
        let resolved = mermaid_theme_for_syntect(
            ThemePair::OneHalf,
            mermaid_color_mode(renderable::color::ColorMode::Unknown),
        );
        assert_eq!(resolved.background, dark.background);
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
