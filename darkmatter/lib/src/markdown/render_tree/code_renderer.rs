//! Darkmatter's [`CodeRenderer`] implementation for the `renderable` render tree.
//!
//! The render-tree terminal and browser renderers fall back to a plain fenced
//! code block when they meet a [`NodeKind::Code`](renderable::tree::NodeKind::Code)
//! node. [`TerminalCodeRenderer`] wires darkmatter's syntax-highlighted
//! code-block path into that hook so render-tree output matches the bespoke
//! [`YamlBlock`](crate::markdown::YamlBlock) (and Markdown code fence) renderer.

use biscuit_terminal::components::mermaid::MermaidDiagram;
use biscuit_terminal::terminal::Terminal;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::color::{ColorDepth as RenderableColorDepth, TerminalCodeContext};
use renderable::tree::{CodeRenderer, NodeAttrs};

use crate::markdown::{
    dsl::{CodeBlockMeta, parse_code_info},
    highlighting::{CodeBlockMode, CodeHighlighter, ColorMode, ThemePair},
    output::code_block::{render_html_code_block, render_terminal_code_block},
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
#[derive(Debug, Default, Clone)]
pub struct TerminalCodeRenderer {
    terminal: Option<Terminal>,
    code_block_mode: CodeBlockMode,
}

impl TerminalCodeRenderer {
    /// Creates a new [`TerminalCodeRenderer`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            terminal: None,
            code_block_mode: CodeBlockMode::default(),
        }
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
        Self {
            terminal: None,
            code_block_mode,
        }
    }

    /// Creates a [`TerminalCodeRenderer`] bound to the detected terminal.
    #[must_use]
    pub fn for_terminal(term: &Terminal, code_block_mode: CodeBlockMode) -> Self {
        Self {
            terminal: Some(term.clone()),
            code_block_mode,
        }
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
        let code_theme = match context.code_theme_name() {
            Some(name) => ThemePair::from_str_or_default(name),
            None => ThemePair::OneHalf,
        };
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
        let highlighter = CodeHighlighter::from_theme(resolved.theme, resolved.color_mode);
        // Header/body contrast keys off the resolved theme background, not the
        // requested mode, so single-variant themes still get readable chrome.
        let color_mode = crate::markdown::output::code_block::mode_for_background(
            highlighter
                .theme()
                .settings
                .background
                .unwrap_or(syntect::highlighting::Color::BLACK),
        );
        let code_meta = build_code_meta(lang.unwrap_or(""), meta);
        let language = lang.unwrap_or("");
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
            language,
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
        let options = HtmlOptions::default();
        // TODO: honor code_block_mode for browser (terminal-only for now).
        // Code blocks contrast against the page: resolve the theme *variant*
        // against the INVERTED color mode (a light code panel on a dark page,
        // and vice versa). This matches `darkmatter::markdown::output::html::as_html`
        // and `YamlBlock`'s browser path so render-tree HTML, legacy `as_html`,
        // and `YamlBlock` agree (Defect D). Single-variant themes (dracula/nord/
        // monokai/vs-dark) are a deliberate no-op.
        let resolved = options.code_theme.resolve_for_surface(
            crate::markdown::highlighting::Surface::Mode(options.color_mode),
            Some(options.code_theme),
            crate::markdown::highlighting::CodeBlockMode::default(),
        );
        let highlighter = CodeHighlighter::from_theme(resolved.theme, resolved.color_mode);
        let code_meta = build_code_meta(lang.unwrap_or(""), meta);
        let language = lang.unwrap_or("");

        // `code_meta` carries any `title` / `line-numbering` / `highlight`
        // directive so the HTML reproduces the legacy renderer's title block,
        // line-number table, and highlighted-line markup.
        let html =
            render_html_code_block(value, language, &code_meta, &highlighter, &options).ok()?;
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
    use renderable::color::ColorDepth;
    use renderable::color::ColorMode as RenderableColorMode;
    use renderable::tree::CodeRenderHints;
    use syntect::easy::HighlightLines;
    use syntect::highlighting::Color;

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
        let syntax = highlighter
            .syntax_set()
            .find_syntax_by_extension("yaml")
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
        let inverted = render_html_code_block(
            code,
            "rust",
            &meta,
            &CodeHighlighter::from_theme(inverted_resolved.theme, inverted_resolved.color_mode),
            &opts,
        )
        .expect("inverted highlight");
        let non_inverted = render_html_code_block(
            code,
            "rust",
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
