//! Darkmatter's [`CodeRenderer`] implementation for the `renderable` render tree.
//!
//! The render-tree terminal and browser renderers fall back to a plain fenced
//! code block when they meet a [`NodeKind::Code`](renderable::tree::NodeKind::Code)
//! node. [`TerminalCodeRenderer`] wires darkmatter's syntax-highlighted
//! code-block path into that hook so render-tree output matches the bespoke
//! [`YamlBlock`](crate::markdown::YamlBlock) (and Markdown code fence) renderer.
//!
//! [`TerminalCodeRenderer`] is the render-tree `CodeRenderer` adapter, not a
//! public rendering surface. It is `#[deprecated]` for direct callers
//! (spec.md:620-624, Phase 4): render code with
//! [`CodeBlock`](crate::markdown::code_block::CodeBlock) and Markdown documents
//! with [`DarkmatterPage`](crate::layout::DarkmatterPage). The crate's own
//! render paths and whitebox tests wire it directly and suppress the warning
//! locally; this module-level `allow` covers the adapter's own impls and tests.
#![allow(deprecated)]

use std::cell::{OnceCell, RefCell};

use biscuit_terminal::components::mermaid::MermaidDiagram;
use biscuit_terminal::terminal::Terminal;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::color::{
    ColorDepth as RenderableColorDepth, ColorMode as RenderableColorMode, TerminalCodeContext,
};
use renderable::tree::{CodeRenderer, NodeAttrs};

use crate::markdown::{
    dsl::{CodeBlockMeta, parse_code_info},
    highlighting::{CodeBlockMode, CodeHighlighter, ColorMode, ThemePair, themes::Theme},
    language_grammar::LanguageGrammar,
    output::code_block::{mode_for_background, render_html_code_block, render_terminal_code_block},
    output::html::HtmlOptions,
    output::terminal::{
        ColorDepth, DimMode, HyperlinkMode, ItalicMode, MermaidMode, TerminalImageMode,
        TerminalOptions, format_header_row,
    },
};

/// Darkmatter's [`CodeRenderer`] hook for the render tree.
///
/// Reproduces the bespoke darkmatter code-block output (header row plus
/// syntax-highlighted body) so the render-tree path stays byte-compatible with
/// the bespoke [`YamlBlock`](crate::markdown::YamlBlock) renderer.
///
/// ## Notes
///
/// The terminal hook produces only the `{header}\n{body}` string; it does not
/// apply layout. The render-tree renderer applies node layout (margins,
/// alignment, word-wrap) separately, so applying it here would double it.
///
/// ## Deprecation
///
/// This adapter is render-tree plumbing, not a public rendering surface
/// (spec.md:620-624). Render code blocks with
/// [`CodeBlock`](crate::markdown::code_block::CodeBlock) and Markdown documents
/// with [`DarkmatterPage`](crate::layout::DarkmatterPage); neither requires a
/// caller to construct or know about `TerminalCodeRenderer`.
#[deprecated(
    since = "0.0.0",
    note = "TerminalCodeRenderer is an internal render-tree adapter, not a public \
            rendering surface. Render code with CodeBlock (e.g. CodeBlock::rust(code), \
            CodeBlock::yaml(code), or CodeBlock::new(code).with_fence_language(lang)) and \
            Markdown documents with DarkmatterPage. If you genuinely need the raw \
            CodeRenderer hook, suppress this warning locally."
)]
#[derive(Debug, Clone)]
pub struct TerminalCodeRenderer {
    terminal: Option<Terminal>,
    code_block_mode: CodeBlockMode,
    /// Explicit theme override from
    /// [`CodeBlock::with_theme`](crate::markdown::code_block::CodeBlock::with_theme)
    /// / `md code-block --theme`. When set it wins over the terminal context's
    /// `code_theme_name` and the carried [`HtmlOptions::code_theme`], so a
    /// direct caller's pinned theme reaches both surfaces.
    theme_override: Option<ThemePair>,
    /// Page-resolved browser options. The browser hook reads the page's
    /// resolved `code_theme`, `color_mode`, `code_block_mode`, and line-number
    /// toggle from here so the highlighted markup uses the same mode/theme
    /// policy as the page frame and the injected `.code-block` stylesheet
    /// (review-1 finding 2). `None` falls back to [`HtmlOptions::default`] —
    /// the direct `CodeBlock` browser path, which emits no page stylesheet to
    /// disagree with.
    html_options: Option<HtmlOptions>,
    /// `CODE_THEME` / `THEME` read once when this renderer is constructed —
    /// the render-scoped environment snapshot (finding 23). Every entry point
    /// builds one renderer per render invocation (both `entrypoints` options
    /// mappers and both [`CodeBlock`](crate::markdown::code_block::CodeBlock)
    /// surfaces), so a later render still observes an environment change while
    /// a 40-fence document reads the environment once instead of once per
    /// fence.
    env_code_theme: Option<ThemePair>,
    /// Terminal theme resolution memoized for the render's
    /// [`TerminalCodeContext`]. The tree renderer hands every code block in one
    /// render the same context, so the first block resolves the surface and the
    /// rest reuse it. The key keeps a renderer that is reused across *different*
    /// contexts (the whitebox tests, which switch pinned theme/mode on one
    /// renderer) resolving each context correctly.
    terminal_surface: RefCell<Option<(TerminalSurfaceKey, CodeSurface)>>,
    /// Browser theme resolution memoized for the render. Unlike the terminal
    /// hook the browser hook takes no per-call context, so its inputs
    /// (`html_options`, `theme_override`, the environment snapshot) are fixed
    /// for the renderer's lifetime and need no key.
    browser_surface: OnceCell<BrowserSurface>,
}

/// The [`TerminalCodeContext`] inputs a [`CodeSurface`] resolution depends on.
///
/// Width, color depth, line numbers, and page surface colors are *not* part of
/// the key: none of them participates in theme or color-mode selection.
type TerminalSurfaceKey = (Option<String>, RenderableColorMode);

/// One render's resolved code-panel theme and color modes (finding 23).
///
/// Produced once per render by
/// [`TerminalCodeRenderer::resolve_terminal_surface`] /
/// [`resolve_browser_surface`](TerminalCodeRenderer::resolve_browser_surface)
/// and reused by every code block, replacing the per-block environment read
/// plus theme/surface resolution chain.
#[derive(Debug, Clone, Copy)]
struct CodeSurface {
    /// The selected theme pair, carried into `TerminalOptions` so the code
    /// block's own padding/header math sees the same pair.
    code_theme: ThemePair,
    /// The concrete theme variant the panel loads.
    theme: Theme,
    /// The surrounding page's color mode, unresolved (`Unknown` preserved) —
    /// the value `TerminalOptions::color_mode` carries.
    page_mode: ColorMode,
    /// The code panel's own color mode (the page mode's inversion under the
    /// default [`CodeBlockMode::Inverse`]).
    panel_mode: ColorMode,
    /// Header/body chrome contrast, keyed off the *resolved theme background*
    /// rather than the requested mode, so single-variant themes still get
    /// readable chrome.
    header_mode: ColorMode,
}

/// One render's resolved browser code-panel state.
#[derive(Debug, Clone)]
struct BrowserSurface {
    /// The page's [`HtmlOptions`] with the theme chain (override → page →
    /// environment snapshot → default) already applied. Held so the hook
    /// borrows it per block instead of cloning the page options per block.
    options: HtmlOptions,
    /// The concrete theme variant the panel loads.
    theme: Theme,
    /// The code panel's own color mode.
    panel_mode: ColorMode,
}

impl TerminalCodeRenderer {
    /// Creates a new [`TerminalCodeRenderer`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_surface_snapshot(None, CodeBlockMode::default())
    }

    /// Builds a renderer and takes its render-scoped environment snapshot.
    ///
    /// Every constructor funnels through here so `CODE_THEME` / `THEME` are
    /// read exactly once per renderer — i.e. once per render invocation.
    fn with_surface_snapshot(terminal: Option<Terminal>, code_block_mode: CodeBlockMode) -> Self {
        Self {
            terminal,
            code_block_mode,
            theme_override: None,
            html_options: None,
            env_code_theme: code_theme_from_env(),
            terminal_surface: RefCell::new(None),
            browser_surface: OnceCell::new(),
        }
    }

    /// Sets the explicit theme override (from `CodeBlock::with_theme` or
    /// `md code-block --theme`). When `Some`, it wins over the terminal
    /// context's `code_theme_name` and the carried browser
    /// [`HtmlOptions::code_theme`].
    #[must_use]
    pub fn with_theme_override(mut self, theme: Option<ThemePair>) -> Self {
        self.theme_override = theme;
        self.invalidate_surfaces();
        self
    }

    /// Carries the page-resolved [`HtmlOptions`] so the browser hook renders
    /// the highlighted markup with the page's `code_theme`, `color_mode`, and
    /// `code_block_mode` rather than `HtmlOptions::default()`.
    #[must_use]
    pub fn with_html_options(mut self, options: HtmlOptions) -> Self {
        self.html_options = Some(options);
        self.invalidate_surfaces();
        self
    }

    /// Drops any memoized surface after a builder changes a resolution input.
    ///
    /// The entry points call the builders before rendering, so this normally
    /// clears nothing; it keeps the memo from outliving the inputs it was
    /// derived from if a caller ever reorders the two.
    fn invalidate_surfaces(&mut self) {
        self.terminal_surface = RefCell::new(None);
        self.browser_surface = OnceCell::new();
    }

    /// Creates a new [`TerminalCodeRenderer`] with a caller-supplied
    /// [`CodeBlockMode`].
    ///
    /// Use this when the renderer's `Terminal` source is not bound and the
    /// caller still wants to control the panel's contrast against the
    /// page — for example, the direct `Markdown::as_terminal(opts)` entry
    /// point, where `opts.code_block_mode` is the only signal available
    /// and the `context.color_mode()` (set from `opts.color_mode`) supplies
    /// the page-side mode.
    #[must_use]
    pub fn new_with_code_block_mode(code_block_mode: CodeBlockMode) -> Self {
        Self::with_surface_snapshot(None, code_block_mode)
    }

    /// Creates a [`TerminalCodeRenderer`] bound to the detected terminal.
    #[must_use]
    pub fn for_terminal(term: &Terminal, code_block_mode: CodeBlockMode) -> Self {
        Self::with_surface_snapshot(Some(term.clone()), code_block_mode)
    }

    /// Returns the render-scoped terminal surface for `context`, resolving it
    /// on the render's first code block and reusing it for the rest.
    fn terminal_surface(&self, context: &TerminalCodeContext) -> CodeSurface {
        let key: TerminalSurfaceKey = (
            context.code_theme_name().map(str::to_string),
            context.color_mode(),
        );
        if let Some((cached_key, surface)) = self.terminal_surface.borrow().as_ref()
            && *cached_key == key
        {
            return *surface;
        }
        let surface = self.resolve_terminal_surface(context);
        *self.terminal_surface.borrow_mut() = Some((key, surface));
        surface
    }

    /// Resolves the theme, page mode, panel mode, and chrome contrast for one
    /// render. Runs once per render — see [`CodeSurface`].
    fn resolve_terminal_surface(&self, context: &TerminalCodeContext) -> CodeSurface {
        #[cfg(test)]
        surface_probe::note_terminal_resolution();

        // The page surface and the code panel must share a single source of
        // truth (Phase 2): a `Terminal` bound to the renderer is the
        // preferred source (it carries the caller's real detected mode); if
        // no terminal is bound (the entry-point path), fall back to the
        // context's `color_mode()` (which the entry point sets from
        // `opts.color_mode` so the page and panel agree). The
        // [`ThemePair::resolve_for_surface`](crate::markdown::highlighting::themes::ThemePair::resolve_for_surface)
        // boundary resolver feeds both surfaces from the same source.
        let page_mode: ColorMode = self
            .terminal
            .as_ref()
            .map(Terminal::color_mode)
            .unwrap_or_else(|| context.color_mode().into());
        // Theme resolution chain (spec: CodeBlock.theme -> page code_theme ->
        // env/default): an explicit `CodeBlock::with_theme(...)` /
        // `md code-block --theme` override wins; then the page-supplied theme
        // name (the page bakes `CODE_THEME` / `THEME` into it at construction);
        // then — for the direct, page-less `CodeBlock` / `md code-block`
        // surface, where the context carries no theme name — the render's
        // `CODE_THEME` / `THEME` environment snapshot; finally the `OneHalf`
        // default. Without the env leg, `THEME=github md code-block …` resolved
        // as `OneHalf` because the direct path pins no context theme name
        // (review-2 finding 1).
        let code_theme = self
            .theme_override
            .or_else(|| context.code_theme_name().map(ThemePair::from_str_or_default))
            .or(self.env_code_theme)
            .unwrap_or(ThemePair::OneHalf);
        let surface = self
            .terminal
            .as_ref()
            .map(crate::markdown::highlighting::Surface::Terminal)
            .unwrap_or(crate::markdown::highlighting::Surface::Mode(page_mode));
        let resolved = code_theme.resolve_for_surface(
            surface,
            Some(code_theme),
            self.code_block_mode,
        );
        let header_mode = mode_for_background(
            CodeHighlighter::from_theme(resolved.theme, resolved.color_mode)
                .theme()
                .settings
                .background
                .unwrap_or(syntect::highlighting::Color::BLACK),
        );
        CodeSurface {
            code_theme,
            theme: resolved.theme,
            page_mode,
            panel_mode: resolved.color_mode,
            header_mode,
        }
    }

    /// Returns the render-scoped browser surface, resolving it on the render's
    /// first code block and reusing it for the rest.
    fn browser_surface(&self) -> &BrowserSurface {
        self.browser_surface
            .get_or_init(|| self.resolve_browser_surface())
    }

    /// Resolves the effective [`HtmlOptions`] and theme variant for one render.
    fn resolve_browser_surface(&self) -> BrowserSurface {
        #[cfg(test)]
        surface_probe::note_browser_resolution();

        // Render with the page's resolved options (code theme, page color mode,
        // code-block mode, line numbers), not `HtmlOptions::default()`: the page
        // frame and the injected `.code-block` stylesheet are built from these
        // same options, so the highlighted markup must agree (review-1 finding
        // 2). A direct `CodeBlock` browser render (no page) carries no options
        // and falls back to the default; a `CodeBlock::with_theme(...)` override
        // wins over the carried theme.
        let mut options = self.html_options.clone().unwrap_or_default();
        // Theme resolution chain (spec: CodeBlock.theme -> page code_theme ->
        // env/default). When page options are carried, `options.code_theme` is
        // already the page-resolved theme (the page bakes `CODE_THEME` /
        // `THEME` in at construction). On the direct, page-less surface no
        // options are carried, so honor the render's `CODE_THEME` / `THEME`
        // snapshot before the `HtmlOptions` default — otherwise a `THEME=github`
        // direct `CodeBlock` HTML render resolved as the default theme
        // (review-2 finding 1). An explicit `with_theme` override still wins.
        if self.html_options.is_none()
            && let Some(env_theme) = self.env_code_theme
        {
            options.code_theme = env_theme;
        }
        if let Some(theme) = self.theme_override {
            options.code_theme = theme;
        }
        // Code blocks contrast against the page: resolve the theme *variant*
        // against the page color mode using the configured `CodeBlockMode`
        // (default `Inverse`: a light code panel on a dark page, and vice
        // versa — Defect D / spec.md:394-397). Single-variant themes
        // (dracula/nord/monokai/vs-dark) are a deliberate no-op.
        let resolved = options.code_theme.resolve_for_surface(
            crate::markdown::highlighting::Surface::Mode(options.color_mode),
            Some(options.code_theme),
            options.code_block_mode,
        );
        BrowserSurface {
            options,
            theme: resolved.theme,
            panel_mode: resolved.color_mode,
        }
    }
}

impl Default for TerminalCodeRenderer {
    /// Equivalent to [`TerminalCodeRenderer::new`] — in particular it takes the
    /// same render-scoped environment snapshot, which a derived `Default` would
    /// silently skip.
    fn default() -> Self {
        Self::new()
    }
}

/// Reconstructs a [`CodeBlockMeta`] from the language token and the fenced
/// info-string `meta` (the text after the language).
///
/// This is how the render-tree path recovers darkmatter's code-block DSL
/// (`title="…" line-numbering=true highlight=2`): the fold splits the info
/// string into `lang` + `meta`, and this helper re-joins them so
/// [`parse_code_info`] yields the same [`CodeBlockMeta`] the legacy renderer
/// builds. A malformed directive (e.g. an invalid highlight range) degrades to
/// a language-only meta rather than failing the render.
fn build_code_meta(lang: &str, meta: Option<&str>) -> CodeBlockMeta {
    let info = match meta {
        Some(m) if !m.trim().is_empty() => format!("{lang} {m}"),
        _ => lang.to_string(),
    };
    parse_code_info(&info).unwrap_or_else(|_| CodeBlockMeta {
        language: lang.to_string(),
        ..CodeBlockMeta::default()
    })
}

/// Env-derived code theme for the *direct*, page-less render surfaces
/// (`CodeBlock` / `md code-block`).
///
/// Mirrors the page path's `detect_code_theme(detect_prose_theme())`
/// precedence — `CODE_THEME` wins over `THEME` — but returns `None` when
/// neither names a valid theme, so the caller's surface default stands
/// (terminal `OneHalf`, browser `github`). The page path bakes this same
/// env resolution into its options at construction
/// ([`TerminalOptions::default`](crate::markdown::output::terminal::TerminalOptions));
/// the direct path carries no such options, so this hook is the boundary
/// that must honor it (review-2 finding 1).
fn code_theme_from_env() -> Option<ThemePair> {
    #[cfg(test)]
    surface_probe::note_env_read();

    std::env::var("CODE_THEME")
        .ok()
        .and_then(|name| ThemePair::try_from(name.trim()).ok())
        .or_else(|| {
            std::env::var("THEME")
                .ok()
                .and_then(|name| ThemePair::try_from(name.trim()).ok())
        })
}

/// Counting seam proving the finding-23 contract — "resolved once per render",
/// not once per code block — is observable rather than inferred from equal
/// output (which a per-block resolution would also produce).
///
/// Counters are thread-local, so a render on the test's own thread is not
/// disturbed by concurrently running tests.
#[cfg(test)]
mod surface_probe {
    use std::cell::Cell;

    thread_local! {
        static ENV_READS: Cell<usize> = const { Cell::new(0) };
        static TERMINAL_RESOLUTIONS: Cell<usize> = const { Cell::new(0) };
        static BROWSER_RESOLUTIONS: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn note_env_read() {
        ENV_READS.with(|c| c.set(c.get() + 1));
    }

    pub(super) fn note_terminal_resolution() {
        TERMINAL_RESOLUTIONS.with(|c| c.set(c.get() + 1));
    }

    pub(super) fn note_browser_resolution() {
        BROWSER_RESOLUTIONS.with(|c| c.set(c.get() + 1));
    }

    /// Runs `body` with the counters zeroed and returns
    /// `(env_reads, terminal_resolutions, browser_resolutions)` alongside its
    /// value.
    pub(super) fn counted<T>(body: impl FnOnce() -> T) -> (T, usize, usize, usize) {
        ENV_READS.with(|c| c.set(0));
        TERMINAL_RESOLUTIONS.with(|c| c.set(0));
        BROWSER_RESOLUTIONS.with(|c| c.set(0));
        let value = body();
        (
            value,
            ENV_READS.with(Cell::get),
            TERMINAL_RESOLUTIONS.with(Cell::get),
            BROWSER_RESOLUTIONS.with(Cell::get),
        )
    }
}

impl CodeRenderer for TerminalCodeRenderer {
    fn render_terminal_code(
        &self,
        lang: Option<&str>,
        value: &str,
        meta: Option<&str>,
        attrs: &NodeAttrs,
        context: TerminalCodeContext,
    ) -> Option<String> {
        // No-color contract (see [`CodeRenderer`]): on `ColorDepth::None` the
        // syntax-highlighted output would emit ANSI color SGRs, so return
        // `None` and let the tree renderer's plain fallback run. This matches
        // the legacy terminal renderer, which short-circuits to unformatted
        // content when color is disabled.
        if context.color_depth() == RenderableColorDepth::None {
            return None;
        }
        let hints = attrs.code_hints();
        // Theme, page mode, panel mode, and chrome contrast are resolved once
        // per render and reused by every block (finding 23).
        let surface = self.terminal_surface(&context);
        let highlighter = CodeHighlighter::from_theme(surface.theme, surface.panel_mode);
        let color_mode = surface.header_mode;
        let code_theme = surface.code_theme;
        let page_mode = surface.page_mode;
        let code_meta = build_code_meta(lang.unwrap_or(""), meta);
        let language = lang.unwrap_or("");
        let grammar = LanguageGrammar::from_token_or_plain_text(language);
        let options = TerminalOptions {
            code_theme,
            prose_theme: code_theme,
            // `page_mode` is the resolved page color mode the code panel
            // inverted against. Threading it through `TerminalOptions`
            // here keeps `render_terminal_code_block`'s padding/header
            // math consistent with the panel background.
            color_mode: page_mode,
            include_line_numbers: context.line_numbers(),
            color_depth: Some(ColorDepth::TrueColor),
            image_mode: TerminalImageMode::Never,
            base_path: None,
            italic_mode: ItalicMode::Auto,
            dim_mode: DimMode::Auto,
            max_width: None,
            mermaid_mode: MermaidMode::Off,
            hyperlink_mode: HyperlinkMode::Auto,
            hr_defaults: None,
            code_block_mode: self.code_block_mode,
        };

        // Body: the syntax-highlighted code block, padded to the available
        // width. `context.width()` is already the post-margin content width
        // (the tree renderer narrows it in `render_with_layout` before this
        // hook runs and re-applies the left margin afterward), so padding to it
        // — rather than clearing to the physical edge with `\x1b[K` — keeps the
        // block within its margins and the right-aligned language pill flush
        // with the block's right edge. `code_meta` carries any `line-numbering`
        // / `highlight` directive so the body honors them.
        let body = render_terminal_code_block(
            value,
            &grammar,
            &highlighter,
            &options,
            &code_meta,
            color_mode,
            Some(context.width() as u16),
            None,
        )
        .ok()?;

        // Header: emitted when the projection requested one (e.g. `YamlBlock`)
        // or when the info string carried a `title`.
        if hints.header_row || code_meta.title.is_some() {
            let label = hints.language_label.as_deref().unwrap_or(language);
            let bg_color = highlighter
                .theme()
                .settings
                .background
                .unwrap_or(syntect::highlighting::Color::BLACK);
            let header = format_header_row(
                code_meta.title.as_deref(),
                label,
                bg_color,
                color_mode,
                context.width() as u16,
            );
            Some(format!("{header}\n{body}"))
        } else {
            Some(body)
        }
    }

    fn render_browser_code(
        &self,
        lang: Option<&str>,
        value: &str,
        meta: Option<&str>,
        _attrs: &NodeAttrs,
    ) -> Option<BrowserFragment<Ready>> {
        // Mermaid is intentionally NOT handled here. The render tree routes
        // `lang="mermaid"` to `render_browser_mermaid`, whose `None` return is
        // an observable promotion failure the renderer can apply strictness to.
        // Catching the SVG failure here and returning a code-block fragment
        // would hide that failure behind `Some(_)` and bypass strictness.
        // The effective options and theme variant are resolved once per render
        // and borrowed here, rather than cloned and re-resolved per block
        // (finding 23).
        let surface = self.browser_surface();
        let highlighter = CodeHighlighter::from_theme(surface.theme, surface.panel_mode);
        let code_meta = build_code_meta(lang.unwrap_or(""), meta);
        let grammar = LanguageGrammar::from_token_or_plain_text(lang.unwrap_or(""));

        // `code_meta` carries any `title` / `line-numbering` / `highlight`
        // directive so the HTML reproduces the legacy renderer's title block,
        // line-number table, and highlighted-line markup.
        let html =
            render_html_code_block(value, &grammar, &code_meta, &highlighter, &surface.options)
                .ok()?;
        Some(BrowserFragment::new().define_as_raw_html(html).finalize())
    }

    fn render_browser_mermaid(
        &self,
        value: &str,
        _meta: Option<&str>,
        _attrs: &NodeAttrs,
    ) -> Option<BrowserFragment<Ready>> {
        // `None` on failure is the contract: it surfaces the promotion failure
        // to the render tree so strictness can reject or diagnose it, rather
        // than silently degrading to a code block here.
        let svg = MermaidDiagram::new(value).render_to_svg().ok()?;
        // The SVG is emitted unescaped as raw HTML, so it must pass an explicit
        // allowlist sanitizer first — the render tree must not depend on the
        // upstream renderer (or any future string override) staying safe. A
        // sanitizer that cannot parse the SVG returns `None`, which the renderer
        // treats as a promotion failure rather than emitting unsafe markup.
        let safe = super::svg_sanitizer::sanitize_svg(&svg)?;
        Some(BrowserFragment::new().define_as_raw_html(safe).finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;
    use renderable::color::ColorDepth;
    use renderable::color::ColorMode as RenderableColorMode;
    use renderable::tree::CodeRenderHints;
    use serial_test::serial;
    use syntect::easy::HighlightLines;
    use syntect::highlighting::Color;

    /// Restores an env var to its prior value, removing it if previously unset.
    struct EnvVarGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvVarGuard {
        fn capture(key: &'static str) -> Self {
            Self {
                key,
                prev: std::env::var(key).ok(),
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    /// Five fences in three languages — the multi-block document the
    /// finding-23 "one snapshot per render" assertions render.
    const MULTI_BLOCK_DOC: &str = "\
# Multi

```rust
fn a() -> usize { 1 }
```

```yaml
first: 1
```

```rust
fn b() -> usize { 2 }
```

```json
{ \"k\": 3 }
```

```yaml
last: 4
```
";

    fn multi_block_terminal_options() -> TerminalOptions {
        TerminalOptions {
            // Pin color depth: a `ColorDepth::None` context short-circuits the
            // hook before it resolves anything, which would make a
            // "resolved once" assertion pass vacuously.
            color_depth: Some(crate::markdown::output::terminal::ColorDepth::TrueColor),
            max_width: Some(80),
            ..TerminalOptions::default()
        }
    }

    fn yaml_attrs() -> NodeAttrs {
        let mut attrs = NodeAttrs::default();
        attrs.set_code_hints(&CodeRenderHints {
            header_row: true,
            language_label: Some("yaml".into()),
            highlight: true,
        });
        attrs
    }

    fn fg_sgr(color: Color) -> String {
        format!("\x1b[38;2;{};{};{}m", color.r, color.g, color.b)
    }

    fn bg_sgr(color: Color) -> String {
        format!("\x1b[48;2;{};{};{}m", color.r, color.g, color.b)
    }

    fn one_half_yaml_color(mode: ColorMode, line: &str, token: &str) -> Color {
        let highlighter = CodeHighlighter::new(ThemePair::OneHalf, mode);
        let syntax = LanguageGrammar::yaml()
            .resolve(highlighter.syntax_set())
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

    fn one_half_background(mode: ColorMode) -> Color {
        CodeHighlighter::new(ThemePair::OneHalf, mode)
            .theme()
            .settings
            .background
            .expect("theme background")
    }

    /// The `render_code_heavy` manifest fixture's 40 fenced blocks as
    /// `(language, body)` pairs — the same frozen bytes `benches/phase8_render.rs`
    /// renders, entered at the code hook instead of at `as_terminal` / `as_html`.
    fn code_heavy_blocks() -> Vec<(String, String)> {
        use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

        let text = crate::perf_harness::fixture_text("render_code_heavy");
        let mut blocks = Vec::new();
        let mut current: Option<(String, String)> = None;
        for event in Parser::new_ext(&text, Options::all()) {
            match event {
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                    let language = info.split_whitespace().next().unwrap_or_default().to_string();
                    current = Some((language, String::new()));
                }
                Event::Text(text) => {
                    if let Some((_, body)) = current.as_mut() {
                        body.push_str(&text);
                    }
                }
                Event::End(TagEnd::CodeBlock) => {
                    if let Some(block) = current.take() {
                        blocks.push(block);
                    }
                }
                _ => {}
            }
        }
        blocks
    }

    /// Finding 23's retained raw-sample harness — the per-observation vectors the
    /// benchmark contract requires ("Raw samples (mandatory)"; "Harnesses are
    /// retained, not deleted").
    ///
    /// The pre-F23 shape is reconstructed without a pinned code copy and without
    /// a production seam: a renderer that is *rebuilt per block* reads the
    /// environment and resolves the surface once per block, which is exactly what
    /// the per-block resolution did; the F23 renderer is built once per render
    /// and resolves once. The `surface_probe` counters assert that reconstruction
    /// is real (40 vs 1) rather than assumed, and the byte-equality gate runs
    /// before any timing.
    ///
    /// Entering at the code hook rather than at `as_terminal` / `as_html` gives
    /// F23 its *best* case: the fixture's prose, layout, and frame work is
    /// excluded, so the hoisted resolution is a larger share here than in any
    /// real render. A null result at this boundary therefore bounds the null
    /// result at the entry points.
    ///
    /// The single-block control is F23's measurement floor: at one block the
    /// two shapes do identical work (one construction, one resolution, one
    /// highlight), so whatever separates them is the harness's own noise.
    #[test]
    #[ignore = "measurement harness; run explicitly with DM_PERF_RAW_DIR set"]
    #[serial]
    fn f23_code_surface_raw_samples() {
        use crate::perf_harness::Harness;
        use std::hint::black_box;

        // 300 samples: at 100 the two arms' 95% CIs abutted, which cannot
        // separate a ~1% effect from the control's own ±0.2% floor.
        let Some(render_harness) = Harness::from_env(300, 1) else {
            return;
        };
        let control_harness = Harness::from_env(300, 20).expect("DM_PERF_RAW_DIR still set");

        let _theme = EnvVarGuard::capture("THEME");
        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        let _no_color = EnvVarGuard::capture("NO_COLOR");
        // Pin the theme signal: the measured resolution chain must not depend on
        // the invoking shell's environment.
        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("CODE_THEME");
            std::env::set_var("THEME", "github");
        }

        let blocks = code_heavy_blocks();
        assert_eq!(
            blocks.len(),
            40,
            "render_code_heavy is the frozen 40-fence manifest fixture",
        );
        let context =
            || TerminalCodeContext::new(120, ColorDepth::TrueColor, RenderableColorMode::Dark);

        let terminal_baseline = || {
            let mut out = String::new();
            for (language, body) in &blocks {
                out.push_str(
                    &TerminalCodeRenderer::new()
                        .render_terminal_code(
                            Some(language),
                            body,
                            None,
                            &NodeAttrs::default(),
                            context(),
                        )
                        .expect("baseline terminal block"),
                );
            }
            out
        };
        let terminal_candidate = || {
            let renderer = TerminalCodeRenderer::new();
            let mut out = String::new();
            for (language, body) in &blocks {
                out.push_str(
                    &renderer
                        .render_terminal_code(
                            Some(language),
                            body,
                            None,
                            &NodeAttrs::default(),
                            context(),
                        )
                        .expect("candidate terminal block"),
                );
            }
            out
        };

        let (baseline_out, baseline_env, baseline_resolutions, _) =
            surface_probe::counted(terminal_baseline);
        let (candidate_out, candidate_env, candidate_resolutions, _) =
            surface_probe::counted(terminal_candidate);
        assert_eq!(
            baseline_out, candidate_out,
            "F23 terminal baseline and candidate must produce byte-identical output",
        );
        assert_eq!(
            (baseline_env, baseline_resolutions),
            (40, 40),
            "the baseline must read the environment and resolve once per block, or it does not \
             represent the pre-F23 shape",
        );
        assert_eq!(
            (candidate_env, candidate_resolutions),
            (1, 1),
            "the candidate must take one snapshot and one resolution per render",
        );

        let browser_baseline = || {
            let mut out = String::new();
            for (language, body) in &blocks {
                out.push_str(
                    &TerminalCodeRenderer::new()
                        .render_browser_code(Some(language), body, None, &NodeAttrs::default())
                        .expect("baseline browser block")
                        .render(),
                );
            }
            out
        };
        let browser_candidate = || {
            let renderer = TerminalCodeRenderer::new();
            let mut out = String::new();
            for (language, body) in &blocks {
                out.push_str(
                    &renderer
                        .render_browser_code(Some(language), body, None, &NodeAttrs::default())
                        .expect("candidate browser block")
                        .render(),
                );
            }
            out
        };

        let (baseline_html, baseline_env, _, baseline_resolutions) =
            surface_probe::counted(browser_baseline);
        let (candidate_html, candidate_env, _, candidate_resolutions) =
            surface_probe::counted(browser_candidate);
        assert_eq!(
            baseline_html, candidate_html,
            "F23 browser baseline and candidate must produce byte-identical output",
        );
        assert_eq!(
            (baseline_env, baseline_resolutions),
            (40, 40),
            "the browser baseline must resolve once per block",
        );
        assert_eq!(
            (candidate_env, candidate_resolutions),
            (1, 1),
            "the browser candidate must resolve once per render",
        );

        let (control_language, control_body) = &blocks[0];
        let control_baseline = || {
            TerminalCodeRenderer::new()
                .render_terminal_code(
                    Some(control_language),
                    control_body,
                    None,
                    &NodeAttrs::default(),
                    context(),
                )
                .expect("control baseline block")
        };
        let control_candidate = || {
            let renderer = TerminalCodeRenderer::new();
            renderer
                .render_terminal_code(
                    Some(control_language),
                    control_body,
                    None,
                    &NodeAttrs::default(),
                    context(),
                )
                .expect("control candidate block")
        };
        assert_eq!(
            control_baseline(),
            control_candidate(),
            "F23 control shapes must produce byte-identical output",
        );

        println!("f23: terminal (40 fences)");
        render_harness.interleaved_pair(
            "f23-terminal-code-heavy-baseline",
            || {
                black_box(terminal_baseline());
            },
            "f23-terminal-code-heavy-candidate",
            || {
                black_box(terminal_candidate());
            },
        );
        println!("f23: browser (40 fences)");
        render_harness.interleaved_pair(
            "f23-browser-code-heavy-baseline",
            || {
                black_box(browser_baseline());
            },
            "f23-browser-code-heavy-candidate",
            || {
                black_box(browser_candidate());
            },
        );
        println!("f23: control (1 fence — identical work on both shapes)");
        control_harness.interleaved_pair(
            "f23-control-single-block-baseline",
            || {
                black_box(control_baseline());
            },
            "f23-control-single-block-candidate",
            || {
                black_box(control_candidate());
            },
        );
    }

    /// Finding 23: one terminal render resolves the code theme/environment
    /// **once**, however many fences the document holds. Counted rather than
    /// inferred from equal output, which a per-block resolution would produce
    /// too.
    #[test]
    #[serial]
    fn terminal_render_resolves_code_surface_once_per_render() {
        let _theme = EnvVarGuard::capture("THEME");
        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        let _no_color = EnvVarGuard::capture("NO_COLOR");
        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("CODE_THEME");
            std::env::set_var("THEME", "github");
        }
        let md: Markdown = MULTI_BLOCK_DOC.into();
        let opts = multi_block_terminal_options();

        let (rendered, env_reads, terminal_resolutions, _) =
            surface_probe::counted(|| md.as_terminal(opts).expect("terminal render"));

        let plain = crate::testing::strip_ansi_codes(&rendered);
        for token in ["fn a()", "first: 1", "fn b()", "\"k\": 3", "last: 4"] {
            assert!(
                plain.contains(token),
                "all five fences must render through the code hook; {token:?} missing from:\n{plain}",
            );
        }
        assert_eq!(
            terminal_resolutions, 1,
            "five fences in one render must share one resolved code surface",
        );
        assert_eq!(
            env_reads, 1,
            "the render's CODE_THEME/THEME snapshot must be taken once, not once per fence",
        );
    }

    /// Finding 23, browser half: one HTML render resolves the effective
    /// `HtmlOptions` + theme variant once and every fence borrows it.
    #[test]
    #[serial]
    fn browser_render_resolves_code_surface_once_per_render() {
        let _theme = EnvVarGuard::capture("THEME");
        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        unsafe {
            std::env::remove_var("CODE_THEME");
            std::env::set_var("THEME", "github");
        }
        let md: Markdown = MULTI_BLOCK_DOC.into();

        let (html, env_reads, _, browser_resolutions) =
            surface_probe::counted(|| md.as_html(HtmlOptions::default()).expect("html render"));

        assert_eq!(
            html.matches("language-").count(),
            5,
            "all five fences must render through the code hook:\n{html}",
        );
        assert_eq!(
            browser_resolutions, 1,
            "five fences in one render must share one resolved browser surface",
        );
        assert_eq!(
            env_reads, 1,
            "the render's CODE_THEME/THEME snapshot must be taken once, not once per fence",
        );
    }

    /// Finding 23's dynamic half: the snapshot is scoped to **one render**, so
    /// a later render observes an environment change. Each `as_terminal` call
    /// builds its own renderer, and therefore its own snapshot.
    #[test]
    #[serial]
    fn separate_terminal_renders_observe_theme_environment_change() {
        let _theme = EnvVarGuard::capture("THEME");
        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        let _no_color = EnvVarGuard::capture("NO_COLOR");
        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("CODE_THEME");
        }
        let md: Markdown = MULTI_BLOCK_DOC.into();

        unsafe { std::env::set_var("THEME", "github") };
        let github = md
            .as_terminal(multi_block_terminal_options())
            .expect("github render");
        unsafe { std::env::set_var("THEME", "dracula") };
        let dracula = md
            .as_terminal(multi_block_terminal_options())
            .expect("dracula render");

        assert_ne!(
            github, dracula,
            "a THEME change between renders must be observed by the later render",
        );
    }

    /// The same contract on the direct, page-less surface — the one where the
    /// renderer's own environment snapshot (rather than the page's baked
    /// options) selects the theme. A snapshot that outlived its render, or a
    /// process-wide cache, would return the stale theme here.
    #[test]
    #[serial]
    fn separate_direct_renders_observe_code_theme_environment_change() {
        let _theme = EnvVarGuard::capture("THEME");
        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        unsafe { std::env::remove_var("THEME") };
        let code = "fn demo() -> usize { 42 }";
        // No `code_theme_name` on the context: the direct surface, where the
        // environment snapshot is the only theme signal.
        let context =
            || TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Dark);

        unsafe { std::env::set_var("CODE_THEME", "github") };
        let github = TerminalCodeRenderer::new()
            .render_terminal_code(Some("rust"), code, None, &NodeAttrs::default(), context())
            .expect("github render");
        let github_html = TerminalCodeRenderer::new()
            .render_browser_code(Some("rust"), code, None, &NodeAttrs::default())
            .expect("github html")
            .render();

        unsafe { std::env::set_var("CODE_THEME", "dracula") };
        let dracula = TerminalCodeRenderer::new()
            .render_terminal_code(Some("rust"), code, None, &NodeAttrs::default(), context())
            .expect("dracula render");
        let dracula_html = TerminalCodeRenderer::new()
            .render_browser_code(Some("rust"), code, None, &NodeAttrs::default())
            .expect("dracula html")
            .render();

        assert_ne!(
            github, dracula,
            "a CODE_THEME change must reach a newly constructed renderer's terminal output",
        );
        assert_ne!(
            github_html, dracula_html,
            "a CODE_THEME change must reach a newly constructed renderer's HTML output",
        );
    }

    /// A pinned `code_theme_name` on the context must reach the highlighter so
    /// two different themes produce different terminal code panels. Without the
    /// theme-name carrier the hook always painted the default theme, dropping a
    /// caller's `with_code_theme(...)`.
    #[test]
    fn terminal_code_honors_pinned_theme_name() {
        let renderer = TerminalCodeRenderer::new();
        let code = "fn demo() -> usize { 42 }";

        let github = renderer
            .render_terminal_code(
                Some("rust"),
                code,
                None,
                &NodeAttrs::default(),
                TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Dark)
                    .with_code_theme_name(Some("github".into())),
            )
            .expect("github render");
        let dracula = renderer
            .render_terminal_code(
                Some("rust"),
                code,
                None,
                &NodeAttrs::default(),
                TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Dark)
                    .with_code_theme_name(Some("dracula".into())),
            )
            .expect("dracula render");

        assert_ne!(
            github, dracula,
            "distinct pinned code themes must produce distinct highlighted output",
        );
    }

    /// A pinned `color_mode` must reach the highlighter: the same theme resolves
    /// a different concrete variant for Light vs Dark (github is a paired
    /// theme), so the rendered panels must differ.
    #[test]
    fn terminal_code_honors_pinned_color_mode() {
        let renderer = TerminalCodeRenderer::new();
        let code = "fn demo() -> usize { 42 }";

        let dark = renderer
            .render_terminal_code(
                Some("rust"),
                code,
                None,
                &NodeAttrs::default(),
                TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Dark)
                    .with_code_theme_name(Some("github".into())),
            )
            .expect("dark render");
        let light = renderer
            .render_terminal_code(
                Some("rust"),
                code,
                None,
                &NodeAttrs::default(),
                TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Light)
                    .with_code_theme_name(Some("github".into())),
            )
            .expect("light render");

        assert_ne!(
            dark, light,
            "the requested color mode must reach the highlighter (github is a paired theme)",
        );
    }

    #[test]
    fn terminal_code_inverts_theme_for_dark_pages() {
        let renderer = TerminalCodeRenderer::new();
        let rendered = renderer
            .render_terminal_code(
                Some("yaml"),
                "$schema:\n  # a string type\n  foo: string\n",
                None,
                &yaml_attrs(),
                TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Dark)
                    .with_code_theme_name(Some("one-half".into()))
                    .with_page_surface(Some((192, 202, 245)), Some((26, 27, 38))),
            )
            .expect("rendered code block");

        let background = one_half_background(ColorMode::Light);
        let comment = one_half_yaml_color(ColorMode::Light, "  # a string type", "#");
        let object_key = one_half_yaml_color(ColorMode::Light, "  foo: string", "foo");
        let string_value = one_half_yaml_color(ColorMode::Light, "  foo: string", "string");

        assert!(
            rendered.contains(&bg_sgr(background)),
            "a dark page should use the exact OneHalf light background RGB({},{},{}):\n{rendered:?}",
            background.r,
            background.g,
            background.b,
        );
        assert!(
            rendered.contains(&format!("{}#", fg_sgr(comment))),
            "comment should use the exact OneHalf light YAML comment RGB({},{},{}):\n{rendered:?}",
            comment.r,
            comment.g,
            comment.b,
        );
        assert!(
            rendered.contains(&format!("{}foo", fg_sgr(object_key))),
            "object key should use the exact OneHalf light YAML key RGB({},{},{}):\n{rendered:?}",
            object_key.r,
            object_key.g,
            object_key.b,
        );
        assert!(
            rendered.contains(&format!("{}string", fg_sgr(string_value))),
            "string value should use the exact OneHalf light YAML scalar RGB({},{},{}):\n{rendered:?}",
            string_value.r,
            string_value.g,
            string_value.b,
        );
    }

    #[test]
    fn terminal_code_inverts_theme_for_light_pages() {
        let renderer = TerminalCodeRenderer::new();
        let rendered = renderer
            .render_terminal_code(
                Some("yaml"),
                "$schema:\n  foo: string\n",
                None,
                &yaml_attrs(),
                TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Light)
                    .with_code_theme_name(Some("one-half".into()))
                    .with_page_surface(Some((101, 123, 131)), Some((253, 246, 227))),
            )
            .expect("rendered code block");

        let background = one_half_background(ColorMode::Dark);
        let object_key = one_half_yaml_color(ColorMode::Dark, "  foo: string", "foo");
        let string_value = one_half_yaml_color(ColorMode::Dark, "  foo: string", "string");

        assert!(
            rendered.contains(&bg_sgr(background)),
            "a light page should use the exact OneHalf dark background RGB({},{},{}):\n{rendered:?}",
            background.r,
            background.g,
            background.b,
        );
        assert!(
            rendered.contains(&format!("{}foo", fg_sgr(object_key))),
            "object key should use the exact OneHalf dark YAML key RGB({},{},{}):\n{rendered:?}",
            object_key.r,
            object_key.g,
            object_key.b,
        );
        assert!(
            rendered.contains(&format!("{}string", fg_sgr(string_value))),
            "string value should use the exact OneHalf dark YAML scalar RGB({},{},{}):\n{rendered:?}",
            string_value.r,
            string_value.g,
            string_value.b,
        );
    }

    /// `CodeBlockMode` controls which OneHalf variant the terminal code panel
    /// uses, independent of the page color mode. `Dark` forces the dark
    /// background, `Light` forces the light one, and `Same` matches the page.
    #[test]
    fn terminal_code_block_mode_selects_variant() {
        use biscuit_terminal::terminal::Terminal;

        let code = "$schema:\n  foo: string\n";
        let dark_bg = bg_sgr(one_half_background(ColorMode::Dark));
        let light_bg = bg_sgr(one_half_background(ColorMode::Light));
        assert_ne!(dark_bg, light_bg, "OneHalf must be a paired theme");

        // A dark page: capture which variant each mode resolves.
        let term = Terminal::new_optimistic(80);

        let render = |mode: CodeBlockMode| {
            TerminalCodeRenderer::for_terminal(&term, mode)
                .render_terminal_code(
                    Some("yaml"),
                    code,
                    None,
                    &yaml_attrs(),
                    TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Dark)
                        .with_code_theme_name(Some("one-half".into())),
                )
                .expect("render")
        };

        // Dark forces dark variant; Light forces light variant.
        assert!(render(CodeBlockMode::Dark).contains(&dark_bg));
        assert!(!render(CodeBlockMode::Dark).contains(&light_bg));
        assert!(render(CodeBlockMode::Light).contains(&light_bg));
        assert!(!render(CodeBlockMode::Light).contains(&dark_bg));

        // Inverse on a dark page -> light panel.
        assert!(render(CodeBlockMode::Inverse).contains(&light_bg));
        // Same on a dark page -> dark panel.
        assert!(render(CodeBlockMode::Same).contains(&dark_bg));
    }

    #[test]
    fn terminal_code_emits_header_and_highlighted_body() {
        let renderer = TerminalCodeRenderer::new();
        let context =
            TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Dark);
        let out = renderer
            .render_terminal_code(Some("yaml"), "foo: 1\nbar: 2", None, &yaml_attrs(), context)
            .expect("renders");

        assert!(out.contains(" yaml "), "header label missing: {out:?}");
        assert!(out.contains("\x1b["), "expected ANSI styling");
        let plain = crate::testing::strip_ansi_codes(&out);
        assert!(plain.contains("foo: 1") && plain.contains("bar: 2"));
    }

    #[test]
    fn terminal_code_omits_header_when_not_requested() {
        let renderer = TerminalCodeRenderer::new();
        let context =
            TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Dark);
        let out = renderer
            .render_terminal_code(Some("yaml"), "foo: 1", None, &NodeAttrs::default(), context)
            .expect("renders");
        assert!(!out.contains(" yaml "), "header should be absent: {out:?}");
    }

    /// Finding 1 (review-11): a `ColorDepth::None` context must short-circuit
    /// to `None` so the tree renderer's plain (no-ANSI) fallback runs instead
    /// of the syntax-highlighted body, matching the legacy renderer's
    /// no-formatting contract.
    #[test]
    fn terminal_code_returns_none_for_color_depth_none() {
        let renderer = TerminalCodeRenderer::new();
        let context = TerminalCodeContext::new(80, ColorDepth::None, RenderableColorMode::Dark);
        let out =
            renderer.render_terminal_code(Some("rust"), "fn x() {}", None, &yaml_attrs(), context);
        assert!(
            out.is_none(),
            "ColorDepth::None must return None so the plain fallback runs; got {out:?}",
        );
    }

    /// A `title="…"` info-string directive must surface as a header row on the
    /// terminal path even without `CodeRenderHints::header_row` (the Markdown
    /// fold does not set hints, only `meta`).
    #[test]
    fn terminal_code_emits_title_header_from_meta() {
        let renderer = TerminalCodeRenderer::new();
        let context =
            TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Dark);
        let out = renderer
            .render_terminal_code(
                Some("rust"),
                "fn demo() {}",
                Some("title=\"Demo Snippet\""),
                &NodeAttrs::default(),
                context,
            )
            .expect("renders");
        let plain = crate::testing::strip_ansi_codes(&out);
        assert!(
            plain.contains("Demo Snippet"),
            "info-string title must reach the terminal header; got {plain:?}",
        );
    }

    #[test]
    fn browser_code_emits_language_class() {
        let renderer = TerminalCodeRenderer::new();
        let fragment = renderer
            .render_browser_code(Some("yaml"), "foo: 1", None, &NodeAttrs::default())
            .expect("renders");
        assert!(fragment.render().contains("language-yaml"));
    }

    /// Defect D (review-1 finding 2): the render-tree browser hook must invert
    /// the code theme for page contrast, exactly like `as_html` and `YamlBlock`.
    /// `HtmlOptions::default()` is a `github` (paired) theme on a `Dark` page, so
    /// the panel must resolve to github-*light*, not github-dark. The fragment carries no stylesheet, so the inversion shows up
    /// in the `<span style="color: …">` syntax colors. We render the same
    /// `render_html_code_block` helper with the inverted (correct) and
    /// non-inverted (buggy) highlighters and assert the hook reproduces the
    /// inverted one — independent of concrete hex values.
    #[test]
    fn browser_code_inverts_theme_for_page_contrast() {
        let opts = HtmlOptions::default();
        let code = "fn main() {}";
        let meta = build_code_meta("rust", None);

        let inverted_resolved = opts.code_theme.resolve_for_surface(
            crate::markdown::highlighting::Surface::Mode(opts.color_mode),
            Some(opts.code_theme),
            crate::markdown::highlighting::CodeBlockMode::default(),
        );
        let rust_grammar = LanguageGrammar::rust();
        let inverted = render_html_code_block(
            code,
            &rust_grammar,
            &meta,
            &CodeHighlighter::from_theme(inverted_resolved.theme, inverted_resolved.color_mode),
            &opts,
        )
        .expect("inverted highlight");
        let non_inverted = render_html_code_block(
            code,
            &rust_grammar,
            &meta,
            &CodeHighlighter::new(opts.code_theme, opts.color_mode),
            &opts,
        )
        .expect("non-inverted highlight");
        assert_ne!(
            inverted, non_inverted,
            "github is a paired theme; inverted vs non-inverted output must differ",
        );

        let html = TerminalCodeRenderer::new()
            .render_browser_code(Some("rust"), code, None, &NodeAttrs::default())
            .expect("renders")
            .render();

        assert!(
            html.contains(inverted.trim()),
            "render-tree HTML on a dark page must use the INVERTED (light) github theme; got:\n{html}",
        );
        assert!(
            !html.contains(non_inverted.trim()),
            "render-tree HTML must not use the non-inverted (dark) github theme; got:\n{html}",
        );
    }

    /// A `title` / `line-numbering` / `highlight` info-string directive must
    /// reproduce the legacy renderer's title block, line-number table, and
    /// highlighted-line markup on the browser path.
    #[test]
    fn browser_code_emits_title_and_line_numbers_from_meta() {
        let renderer = TerminalCodeRenderer::new();
        let fragment = renderer
            .render_browser_code(
                Some("rust"),
                "fn a() {}\nfn b() {}",
                Some("title=\"Demo Snippet\" line-numbering=true highlight=2"),
                &NodeAttrs::default(),
            )
            .expect("renders");
        let html = fragment.render();
        assert!(
            html.contains("code-block-title") && html.contains("Demo Snippet"),
            "info-string title must reach HTML; got {html}",
        );
        assert!(
            html.contains("code-table") && html.contains("ln-gutter"),
            "line-numbering must produce the line-number table; got {html}",
        );
        assert!(
            html.contains("highlighted"),
            "highlight directive must mark the highlighted line; got {html}",
        );
    }

    /// Mermaid promotion lives in the dedicated `render_browser_mermaid` hook,
    /// which returns `Some(svg)` on success and `None` on failure — the renderer
    /// applies strictness to that `None`. `render_browser_code` must NOT special-
    /// case mermaid; doing so would hide an SVG failure behind a code-block
    /// fallback and bypass strictness (review-2 finding 2).
    #[test]
    fn browser_mermaid_returns_svg_or_none_never_silent_code_block() {
        let renderer = TerminalCodeRenderer::new();

        match renderer.render_browser_mermaid(
            "flowchart LR\n    A --> B",
            None,
            &NodeAttrs::default(),
        ) {
            Some(f) => {
                let html = f.render();
                assert!(
                    html.contains("<svg") && html.contains("</svg>"),
                    "successful promotion must be well-formed SVG; got: {html}"
                );
            }
            None => {
                // Host lacks the Mermaid toolchain: a `None` is the correct
                // failure signal — the renderer degrades per strictness.
            }
        }
    }

    /// `render_browser_code` must treat `lang="mermaid"` as an ordinary code
    /// block (no SVG promotion), so the only promotion path is the fallible
    /// `render_browser_mermaid` hook.
    #[test]
    fn browser_code_does_not_promote_mermaid() {
        let renderer = TerminalCodeRenderer::new();
        let html = renderer
            .render_browser_code(
                Some("mermaid"),
                "flowchart LR\n    A --> B",
                None,
                &NodeAttrs::default(),
            )
            .expect("renders a code block")
            .render();
        assert!(
            !html.contains("<svg"),
            "render_browser_code must not promote mermaid to SVG; got: {html}"
        );
    }
}
