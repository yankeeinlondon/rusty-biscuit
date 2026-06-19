//! Atomic code-block rendering component.
//!
//! [`CodeBlock`] is the public Darkmatter component for rendering a single
//! syntax-highlighted code block. It is independent of Markdown parsing: the
//! terminal and browser render paths fold through the same shared
//! [`render_terminal_code_block`](crate::markdown::output::code_block::render_terminal_code_block)
//! and
//! [`render_html_code_block`](crate::markdown::output::code_block::render_html_code_block)
//! helpers that Markdown fences use, so output stays byte-compatible with a
//! Markdown ` ```rust ` (or ` ```yaml `) fence for the same code, language,
//! and metadata.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::code_block::CodeBlock;
//!
//! let block = CodeBlock::rust("fn main() {}");
//! assert_eq!(block.code(), "fn main() {}");
//! ```

use std::any::Any;
use std::path::Path;
use std::rc::Rc;

use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Layout;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::tree::{
    BrowserRenderOptions, CodeRenderHints, RenderNode, RenderStrictness, TreeRenderable,
    render_browser_node,
};
use thiserror::Error;

use crate::markdown::dsl::{CodeBlockMeta, parse_code_info};
use crate::markdown::highlighting::{CodeBlockMode, ColorMode, ThemePair};
use crate::markdown::language_grammar::LanguageGrammar;
// `TerminalCodeRenderer` is the deprecated render-tree adapter; `CodeBlock` is
// its sanctioned replacement, so wiring it here is the one legitimate direct use.
#[allow(deprecated)]
use crate::markdown::render_tree::TerminalCodeRenderer;

/// Errors that can occur when constructing or working with a [`CodeBlock`].
#[derive(Debug, Error)]
pub enum CodeBlockError {
    /// Failed to read source file from disk.
    #[error("Failed to read code source: {0}")]
    Io(#[from] std::io::Error),

    /// A code-block DSL directive (in the `meta` text) failed to parse.
    #[error("Invalid code-block directive: {0}")]
    InvalidDirective(String),
}

/// Atomic code-block rendering component.
///
/// `CodeBlock` is the public Darkmatter component for rendering one
/// syntax-highlighted code block. It stores **both** the parsed
/// [`CodeBlockMeta`] (which drives rendering decisions such as title,
/// highlight ranges, and line numbering) and the raw fence remainder text
/// (`raw_meta`). The raw text is retained because the Markdown render
/// target re-emits the fence info string verbatim (`{lang} {raw_meta}`)
/// and the TOC tracks the full info string for change detection.
/// Re-deriving the fence text from the parsed struct alone would risk
/// reordering keys or normalizing quoting and whitespace, breaking
/// byte-exact Markdown parity. When a `CodeBlock` is constructed
/// directly (not from a fence), `raw_meta` is `None` and the fence is
/// serialized from `meta`.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeBlock {
    code: String,
    language: Option<LanguageGrammar>,
    meta: CodeBlockMeta,
    raw_meta: Option<String>,
    theme: Option<ThemePair>,
    layout: Layout,
}

impl CodeBlock {
    /// Constructs a [`CodeBlock`] from a raw code string with no language
    /// (plain text).
    ///
    /// Select a language with [`with_fence_language`](Self::with_fence_language)
    /// or [`with_language`](Self::with_language), or use a typed constructor
    /// such as [`rust`](Self::rust) / [`yaml`](Self::yaml).
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::code_block::CodeBlock;
    ///
    /// let block = CodeBlock::new("fn main() {}").with_fence_language("rust");
    /// assert_eq!(block.code(), "fn main() {}");
    /// ```
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            language: None,
            meta: CodeBlockMeta::default(),
            raw_meta: None,
            theme: None,
            layout: Layout::default(),
        }
    }

    /// Sets the [`LanguageGrammar`] for this block, overriding the token
    /// captured at construction.
    pub fn with_language(mut self, language: LanguageGrammar) -> Self {
        self.language = Some(language);
        self
    }

    /// Sets the language from a raw fence-style token, resolved through
    /// [`LanguageGrammar::from_token_or_plain_text`].
    pub fn with_fence_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(LanguageGrammar::from_token_or_plain_text(language.into()));
        self
    }

    /// Replaces the parsed [`CodeBlockMeta`] for this block.
    pub fn with_meta(mut self, meta: CodeBlockMeta) -> Self {
        self.meta = meta;
        self
    }

    /// Sets the theme override used for this block. When `None` (the
    /// default), the block resolves themes through the page context
    /// (terminal mode or `DarkmatterPage` policy).
    pub fn with_theme(mut self, theme: ThemePair) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Returns a [`CodeBlock`] for YAML content (language token `"yaml"`).
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::code_block::CodeBlock;
    ///
    /// let block = CodeBlock::yaml("foo: 1\nbar: 2");
    /// assert_eq!(block.code(), "foo: 1\nbar: 2");
    /// ```
    pub fn yaml(code: impl Into<String>) -> Self {
        Self::new(code).with_fence_language("yaml")
    }

    /// Returns a [`CodeBlock`] for Rust content (language token `"rust"`).
    pub fn rust(code: impl Into<String>) -> Self {
        Self::new(code).with_fence_language("rust")
    }

    /// Returns a [`CodeBlock`] for JSON content (language token `"json"`).
    pub fn json(code: impl Into<String>) -> Self {
        Self::new(code).with_fence_language("json")
    }

    /// Returns a [`CodeBlock`] for TOML content (language token `"toml"`).
    pub fn toml(code: impl Into<String>) -> Self {
        Self::new(code).with_fence_language("toml")
    }

    /// Reads a file from disk and wraps its contents in a [`CodeBlock`].
    ///
    /// The language is inferred from the filename via
    /// [`LanguageGrammar::from_lossy`]; unsupported paths fall back to
    /// [`LanguageGrammar::PlainText`].
    ///
    /// ## Errors
    ///
    /// Returns [`CodeBlockError::Io`] if the file cannot be read.
    pub fn from_source_file(path: impl AsRef<Path>) -> Result<Self, CodeBlockError> {
        let path = path.as_ref();
        let code = std::fs::read_to_string(path)?;
        let language = LanguageGrammar::from_lossy(path.to_string_lossy());
        Ok(Self::new(code).with_language(language))
    }

    /// Constructs a [`CodeBlock`] from a parsed [`CodeBlockMeta`] and the
    /// raw fence remainder text (the info string beyond the language).
    ///
    /// This is the path the Markdown fold uses: it splits the info string
    /// into `lang` + `raw_meta`, and the spec says `CodeBlock` must keep
    /// the raw text alongside the parsed struct so it can re-emit the
    /// fence verbatim. Callers building a `CodeBlock` from a fence use
    /// this constructor; directly-constructed blocks pass `None` for
    /// `raw_meta` and the constructor serializes the meta instead.
    pub fn from_fence_parts(
        code: impl Into<String>,
        language: Option<impl AsRef<str>>,
        raw_meta: Option<String>,
        meta: CodeBlockMeta,
    ) -> Self {
        let code = code.into();
        let language = language.map(|lang| LanguageGrammar::from_token_or_plain_text(lang));
        Self {
            code,
            language,
            meta,
            raw_meta,
            theme: None,
            layout: Layout::default(),
        }
    }

    /// Returns the code text this block holds.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Consumes the block and returns the stored code text.
    pub fn into_code(self) -> String {
        self.code
    }

    /// Returns a reference to the resolved language grammar, if any.
    pub fn language(&self) -> Option<&LanguageGrammar> {
        self.language.as_ref()
    }

    /// Returns a reference to the parsed [`CodeBlockMeta`].
    pub fn meta(&self) -> &CodeBlockMeta {
        &self.meta
    }

    /// Returns the raw fence remainder (info string beyond the language
    /// token) when this block was constructed from one. `None` for
    /// directly-constructed blocks.
    pub fn raw_meta(&self) -> Option<&str> {
        self.raw_meta.as_deref()
    }

    /// Returns the explicit theme override, if any.
    pub fn theme(&self) -> Option<ThemePair> {
        self.theme
    }

    /// Builds the bare [`NodeKind::Code`](renderable::tree::NodeKind::Code)
    /// node this block projects into the render tree.
    ///
    /// The node carries [`CodeRenderHints`] requesting a header row, the
    /// resolved language label, and syntax highlighting, so a tree renderer
    /// wired with darkmatter's [`TerminalCodeRenderer`] reproduces the
    /// highlighted header-plus-body output. The stored [`Layout`] is
    /// attached when non-default so the renderer applies margins,
    /// alignment, and word-wrap.
    fn code_node(&self) -> RenderNode {
        let lang = self.language.as_ref().map(|g| g.to_string());
        let lang_label = lang.clone().unwrap_or_default();
        let meta = self
            .raw_meta
            .clone()
            .or_else(|| serialize_meta(&self.meta));
        let mut node = RenderNode::code(lang, meta, self.code.clone());
        node.attrs.set_code_hints(&CodeRenderHints {
            header_row: true,
            language_label: Some(lang_label),
            highlight: true,
        });
        if self.layout != Layout::default() {
            node.attrs.set_layout(&self.layout);
        }
        node
    }

    /// Resolves a `CodeBlockMeta` from raw fence info string parts
    /// (`{lang} {raw_meta}`). Used to wire `with_meta` so callers can
    /// pass a parsed meta from a fenced block.
    ///
    /// ## Errors
    ///
    /// Returns [`CodeBlockError::InvalidDirective`] if the info string
    /// contains a malformed DSL directive.
    pub fn meta_from_fence(lang: &str, raw_meta: Option<&str>) -> Result<CodeBlockMeta, CodeBlockError> {
        let info = match raw_meta {
            Some(m) if !m.trim().is_empty() => format!("{lang} {m}"),
            _ => lang.to_string(),
        };
        parse_code_info(&info).map_err(|e| CodeBlockError::InvalidDirective(e.to_string()))
    }
}

impl TreeRenderable for CodeBlock {
    /// Projects the block into a root-wrapped code node.
    ///
    /// The projection is a plain `NodeKind::Code` subtree carrying the
    /// language and raw info-string metadata. **No syntax highlighting
    /// happens here** — terminal and browser target folds remain
    /// responsible for rendering the code panel.
    fn render_tree(&self) -> RenderNode {
        RenderNode::root(vec![self.code_node()])
    }
}

impl TerminalRenderable for CodeBlock {
    /// Renders the code block as an ANSI-styled terminal string.
    ///
    /// Routes through darkmatter's [`TerminalCodeRenderer`], the same
    /// shared `render_terminal_code_block` helper Markdown fences use,
    /// so a [`CodeBlock::yaml(...)]` produces output byte-for-byte equal
    /// to a Markdown ` ```yaml ` fence for the same code, language, and
    /// metadata.
    #[allow(deprecated)]
    fn render(&self, term: &Terminal) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn)
            .with_code_renderer(Rc::new(
                TerminalCodeRenderer::for_terminal(term, CodeBlockMode::default())
                    .with_theme_override(self.theme),
            ));
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "CodeBlock",
                    error = %error,
                    "render_terminal_node failed; emitting plain fallback"
                );
                format!("\n{}\n", self.code())
            }
        }
    }

    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        self.render(&term)
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn is_block_level(&self) -> bool {
        true
    }

    /// Exposes the bare code-node projection through the canonical
    /// [`TerminalRenderable::render_tree_node`] hook so a parent component
    /// that embeds a `CodeBlock` consumes it structurally.
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.code_node())
    }
}

impl BrowserRenderable for CodeBlock {
    /// Renders the code block as a `<pre><code class="language-…">…</code></pre>`
    /// HTML fragment through darkmatter's [`TerminalCodeRenderer`], the same
    /// `render_html_code_block` path Markdown fences use, so a
    /// [`CodeBlock::yaml(...)]` produces output byte-for-byte equal to a
    /// Markdown ` ```yaml ` fence for the same code, language, and
    /// metadata.
    #[allow(deprecated)]
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = BrowserRenderOptions {
            code_renderer: Some(Rc::new(
                TerminalCodeRenderer::new().with_theme_override(self.theme),
            )),
            ..Default::default()
        };
        match render_browser_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "CodeBlock",
                    error = %error,
                    "render_browser_node failed; emitting escaped plain fallback"
                );
                BrowserFragment::new()
                    .define_as_text_fragment(self.code().to_string())
                    .finalize()
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Internal helper that returns a snapshot of how a [`CodeBlock`] should be
/// rendered for a given terminal context, without going through the render
/// tree. This is what the tree's [`TerminalCodeRenderer`] uses today; the
/// `CodeBlock` API now mirrors that shape so callers can render a block
/// directly without a `RenderNode` detour.
///
/// Returns `(header_row_string_or_none, body_string)`. Either field may be
/// `None` to indicate the component should fall through to a plain render.
#[allow(dead_code, clippy::too_many_arguments)]
pub(crate) fn render_code_block_for_terminal(
    code: &str,
    language: &str,
    meta: &CodeBlockMeta,
    highlighter: &crate::markdown::highlighting::CodeHighlighter,
    options: &super::output::terminal::TerminalOptions,
    page_mode: ColorMode,
    code_block_mode: CodeBlockMode,
    width: Option<u16>,
) -> Result<(Option<String>, String), crate::markdown::MarkdownError> {
    let _ = code_block_mode.resolve(page_mode);
    let grammar = LanguageGrammar::from_token_or_plain_text(language);
    let color_mode = super::output::code_block::mode_for_background(
        highlighter
            .theme()
            .settings
            .background
            .unwrap_or(syntect::highlighting::Color::BLACK),
    );
    let body = super::output::code_block::render_terminal_code_block(
        code,
        &grammar,
        highlighter,
        options,
        meta,
        color_mode,
        width,
        None,
    )?;
    let header = if meta.title.is_some() {
        let bg = highlighter
            .theme()
            .settings
            .background
            .unwrap_or(syntect::highlighting::Color::BLACK);
        Some(super::output::terminal::format_header_row(
            meta.title.as_deref(),
            language,
            bg,
            color_mode,
            width.unwrap_or(80),
        ))
    } else {
        None
    };
    Ok((header, body))
}

/// Serializes a [`CodeBlockMeta`] back into the fence-info-string format the
/// [`parse_code_info`](crate::markdown::dsl::parse_code_info) DSL accepts:
/// `key="value" key=val …`.
///
/// Returns `None` when the meta carries no directives the parser recognizes,
/// so a freshly-constructed `CodeBlock` without `title` / `line-numbering` /
/// `highlight` does not emit a stray meta string on the projected node.
fn serialize_meta(meta: &CodeBlockMeta) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(title) = &meta.title {
        parts.push(format!("title=\"{}\"", title.replace('"', "\\\"")));
    }
    if meta.line_numbering {
        parts.push("line-numbering=true".to_string());
    }
    if !meta.highlight.is_empty() {
        parts.push(format!("highlight={}", meta.highlight));
    }
    // Custom keys: round-trip unknown directives verbatim.
    for (k, v) in &meta.custom {
        if v.contains(' ') || v.contains('"') {
            parts.push(format!("{k}=\"{}\"", v.replace('"', "\\\"")));
        } else {
            parts.push(format!("{k}={v}"));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::utils::layout::{Length, TargetValue};
    use serial_test::serial;
    use std::io::Write;
    use tempfile::NamedTempFile;

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
                Some(v) => unsafe { std::env::set_var(self.key, v) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    #[test]
    fn new_stores_code_without_language() {
        let block = CodeBlock::new("foo: 1");
        assert_eq!(block.code(), "foo: 1");
        assert!(block.language().is_none());
    }

    #[test]
    fn new_with_fence_language_resolves_grammar() {
        let block = CodeBlock::new("fn main() {}").with_fence_language("rust");
        assert_eq!(block.code(), "fn main() {}");
        assert!(matches!(block.language(), Some(LanguageGrammar::Rust)));
    }

    #[test]
    fn yaml_constructor_sets_yaml_grammar() {
        let block = CodeBlock::yaml("foo: 1");
        assert!(matches!(block.language(), Some(LanguageGrammar::Yaml)));
    }

    #[test]
    fn rust_constructor_sets_rust_grammar() {
        let block = CodeBlock::rust("fn main() {}");
        assert!(matches!(block.language(), Some(LanguageGrammar::Rust)));
    }

    #[test]
    fn json_constructor_sets_json_grammar() {
        let block = CodeBlock::json("{\"k\":1}");
        assert!(matches!(block.language(), Some(LanguageGrammar::Json)));
    }

    #[test]
    fn toml_constructor_sets_toml_grammar() {
        let block = CodeBlock::toml("k = 1");
        assert!(matches!(block.language(), Some(LanguageGrammar::Toml)));
    }

    #[test]
    fn with_language_overrides_constructor() {
        let block = CodeBlock::yaml("foo: 1").with_language(LanguageGrammar::Json);
        assert!(matches!(block.language(), Some(LanguageGrammar::Json)));
    }

    #[test]
    fn with_fence_language_resolves_grammar() {
        let block = CodeBlock::new("fn main() {}").with_fence_language("rust");
        assert!(matches!(block.language(), Some(LanguageGrammar::Rust)));
    }

    #[test]
    fn with_fence_language_handles_yml_alias() {
        let block = CodeBlock::new("foo: 1").with_fence_language("yml");
        assert!(matches!(block.language(), Some(LanguageGrammar::Yaml)));
    }

    #[test]
    fn with_meta_replaces_meta() {
        let meta = CodeBlockMeta {
            title: Some("Demo".to_string()),
            ..Default::default()
        };
        let block = CodeBlock::rust("fn main() {}").with_meta(meta.clone());
        assert_eq!(block.meta().title, Some("Demo".to_string()));
    }

    #[test]
    fn with_theme_stores_override() {
        let block = CodeBlock::rust("fn main() {}").with_theme(ThemePair::Github);
        assert_eq!(block.theme(), Some(ThemePair::Github));
    }

    /// review-1 finding 1: `with_theme` must reach the terminal renderer and
    /// change the resolved output — `github` and `nord` are distinct themes, so
    /// their highlighted SGR colors must differ. A stored-but-ignored override
    /// (the original defect) would make these equal.
    #[test]
    fn with_theme_changes_terminal_output() {
        let term = Terminal::new_optimistic(80);
        let code = "fn demo() -> usize { 42 }";
        let github =
            TerminalRenderable::render(&CodeBlock::rust(code).with_theme(ThemePair::Github), &term);
        let nord =
            TerminalRenderable::render(&CodeBlock::rust(code).with_theme(ThemePair::Nord), &term);
        assert_ne!(
            github, nord,
            "a CodeBlock theme override must change the resolved terminal output",
        );
    }

    /// review-1 finding 1: `with_theme` must also reach the browser renderer and
    /// change the emitted `<span style="color: …">` syntax colors.
    #[test]
    fn with_theme_changes_html_output() {
        let code = "fn demo() -> usize { 42 }";
        let github = BrowserRenderable::render_html_fragment(
            &CodeBlock::rust(code).with_theme(ThemePair::Github),
        )
        .render();
        let nord = BrowserRenderable::render_html_fragment(
            &CodeBlock::rust(code).with_theme(ThemePair::Nord),
        )
        .render();
        assert_ne!(
            github, nord,
            "a CodeBlock theme override must change the resolved HTML output",
        );
    }

    /// review-2 finding 1: a direct `CodeBlock` with no `with_theme` override
    /// must honor the `THEME` env var on the terminal surface — the page path
    /// bakes env into its options at construction, but the direct, page-less
    /// surface carries no theme name, so the render hook is the boundary that
    /// must consult env. Two distinct `THEME` values must yield distinct output.
    #[test]
    #[serial]
    fn terminal_honors_theme_env_when_no_override() {
        let _no_color = EnvVarGuard::capture("NO_COLOR");
        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        let _theme = EnvVarGuard::capture("THEME");
        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("CODE_THEME");
        }
        let term = Terminal::new_optimistic(80);
        let code = "fn demo() -> usize { 42 }";

        unsafe { std::env::set_var("THEME", "github") };
        let github = TerminalRenderable::render(&CodeBlock::rust(code), &term);
        unsafe { std::env::set_var("THEME", "nord") };
        let nord = TerminalRenderable::render(&CodeBlock::rust(code), &term);
        unsafe { std::env::remove_var("THEME") };

        assert_ne!(
            github, nord,
            "THEME must drive a direct CodeBlock's terminal theme when no with_theme override is set",
        );
    }

    /// review-2 finding 1: `CODE_THEME` must also drive a direct `CodeBlock`'s
    /// terminal theme (it wins over `THEME`, matching the page path's
    /// `detect_code_theme` precedence).
    #[test]
    #[serial]
    fn terminal_honors_code_theme_env_when_no_override() {
        let _no_color = EnvVarGuard::capture("NO_COLOR");
        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        let _theme = EnvVarGuard::capture("THEME");
        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("THEME");
        }
        let term = Terminal::new_optimistic(80);
        let code = "fn demo() -> usize { 42 }";

        unsafe { std::env::set_var("CODE_THEME", "dracula") };
        let dracula = TerminalRenderable::render(&CodeBlock::rust(code), &term);
        unsafe { std::env::set_var("CODE_THEME", "github") };
        let github = TerminalRenderable::render(&CodeBlock::rust(code), &term);
        unsafe { std::env::remove_var("CODE_THEME") };

        assert_ne!(
            dracula, github,
            "CODE_THEME must drive a direct CodeBlock's terminal theme when no with_theme override is set",
        );
    }

    /// review-2 finding 1: a `with_theme` override must win over `THEME`, so the
    /// resolved output is independent of the env value.
    #[test]
    #[serial]
    fn terminal_override_wins_over_theme_env() {
        let _no_color = EnvVarGuard::capture("NO_COLOR");
        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        let _theme = EnvVarGuard::capture("THEME");
        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("CODE_THEME");
        }
        let term = Terminal::new_optimistic(80);
        let code = "fn demo() -> usize { 42 }";

        unsafe { std::env::set_var("THEME", "github") };
        let with_env = TerminalRenderable::render(&CodeBlock::rust(code).with_theme(ThemePair::Nord), &term);
        unsafe { std::env::set_var("THEME", "nord") };
        let other_env = TerminalRenderable::render(&CodeBlock::rust(code).with_theme(ThemePair::Nord), &term);
        unsafe { std::env::remove_var("THEME") };

        assert_eq!(
            with_env, other_env,
            "an explicit with_theme override must ignore THEME, so output is env-independent",
        );
    }

    /// review-2 finding 1: the browser surface must honor `THEME` for a direct
    /// `CodeBlock` (no page options carried) when no `with_theme` override is
    /// set — two distinct `THEME` values must yield distinct HTML.
    #[test]
    #[serial]
    fn browser_honors_theme_env_when_no_override() {
        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        let _theme = EnvVarGuard::capture("THEME");
        unsafe { std::env::remove_var("CODE_THEME") };
        let code = "fn demo() -> usize { 42 }";

        unsafe { std::env::set_var("THEME", "github") };
        let github = BrowserRenderable::render_html_fragment(&CodeBlock::rust(code)).render();
        unsafe { std::env::set_var("THEME", "nord") };
        let nord = BrowserRenderable::render_html_fragment(&CodeBlock::rust(code)).render();
        unsafe { std::env::remove_var("THEME") };

        assert_ne!(
            github, nord,
            "THEME must drive a direct CodeBlock's HTML theme when no with_theme override is set",
        );
    }

    /// review-2 finding 1: `CODE_THEME` must drive a direct `CodeBlock`'s HTML
    /// theme when no `with_theme` override is set.
    #[test]
    #[serial]
    fn browser_honors_code_theme_env_when_no_override() {
        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        let _theme = EnvVarGuard::capture("THEME");
        unsafe { std::env::remove_var("THEME") };
        let code = "fn demo() -> usize { 42 }";

        unsafe { std::env::set_var("CODE_THEME", "dracula") };
        let dracula = BrowserRenderable::render_html_fragment(&CodeBlock::rust(code)).render();
        unsafe { std::env::set_var("CODE_THEME", "github") };
        let github = BrowserRenderable::render_html_fragment(&CodeBlock::rust(code)).render();
        unsafe { std::env::remove_var("CODE_THEME") };

        assert_ne!(
            dracula, github,
            "CODE_THEME must drive a direct CodeBlock's HTML theme when no with_theme override is set",
        );
    }

    /// review-2 finding 1: a `with_theme` override must win over `THEME` on the
    /// browser surface too.
    #[test]
    #[serial]
    fn browser_override_wins_over_theme_env() {
        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        let _theme = EnvVarGuard::capture("THEME");
        unsafe { std::env::remove_var("CODE_THEME") };
        let code = "fn demo() -> usize { 42 }";

        unsafe { std::env::set_var("THEME", "github") };
        let with_env =
            BrowserRenderable::render_html_fragment(&CodeBlock::rust(code).with_theme(ThemePair::Nord)).render();
        unsafe { std::env::set_var("THEME", "nord") };
        let other_env =
            BrowserRenderable::render_html_fragment(&CodeBlock::rust(code).with_theme(ThemePair::Nord)).render();
        unsafe { std::env::remove_var("THEME") };

        assert_eq!(
            with_env, other_env,
            "an explicit with_theme override must ignore THEME on the browser surface too",
        );
    }

    #[test]
    fn from_source_file_reads_and_infers_extension() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"fn main() {}").unwrap();

        let block = CodeBlock::from_source_file(file.path()).unwrap();
        assert_eq!(block.code(), "fn main() {}");
        // Tempfile names end in `.tmp`; the language should fall through
        // to a token variant and resolve via alias or extension.
        assert!(block.language().is_some());
    }

    #[test]
    fn from_source_file_missing_returns_io_error() {
        let result = CodeBlock::from_source_file("/nonexistent/path/file.rs");
        assert!(matches!(result, Err(CodeBlockError::Io(_))));
    }

    #[test]
    fn code_node_carries_language_and_hints() {
        let block = CodeBlock::yaml("foo: 1");
        let node = block.code_node();
        match &node.kind {
            renderable::tree::NodeKind::Code { lang, meta, value } => {
                assert_eq!(lang.as_deref(), Some("yaml"));
                assert!(meta.is_none());
                assert_eq!(value, "foo: 1");
            }
            _ => panic!("expected Code node"),
        }
        let hints = node.attrs.code_hints();
        assert!(hints.header_row);
        assert!(hints.highlight);
        assert_eq!(hints.language_label.as_deref(), Some("yaml"));
    }

    #[test]
    fn render_tree_projects_root_with_code_child() {
        let block = CodeBlock::rust("fn main() {}");
        let tree = <CodeBlock as TreeRenderable>::render_tree(&block);
        match &tree.kind {
            renderable::tree::NodeKind::Root { children } => {
                assert_eq!(children.len(), 1);
            }
            _ => panic!("expected Root node"),
        }
    }

    #[test]
    fn render_optimistic_emits_ansi() {
        let block = CodeBlock::yaml("foo: 1");
        let out = TerminalRenderable::render_optimistic(&block, None);
        assert!(out.contains("\x1b["), "expected ANSI styling; got {out:?}");
        let plain = crate::testing::strip_ansi_codes(&out);
        assert!(plain.contains("foo: 1"));
    }

    #[test]
    fn render_in_width_still_emits_ansi() {
        let block = CodeBlock::yaml("foo: 1");
        let term = Terminal::new_optimistic(40);
        let out = TerminalRenderable::render(&block, &term);
        assert!(!out.is_empty());
        assert!(out.contains("\x1b["));
    }

    #[test]
    fn render_html_fragment_contains_language_class() {
        let block = CodeBlock::yaml("foo: 1");
        let html = BrowserRenderable::render_html_fragment(&block).render();
        assert!(html.contains("language-yaml"), "html: {html}");
    }

    #[test]
    fn render_html_fragment_escapes_special_chars() {
        let block = CodeBlock::yaml("script: \"<script>alert('xss')</script>\"");
        let html = BrowserRenderable::render_html_fragment(&block).render();
        assert!(
            html.contains("&lt;script&gt;") || html.contains("&#60;script&#62;"),
            "html: {html}"
        );
    }

    #[test]
    fn is_block_level_is_true() {
        let block = CodeBlock::rust("fn main() {}");
        assert!(TerminalRenderable::is_block_level(&block));
    }

    #[test]
    fn as_any_returns_code_block() {
        let block = CodeBlock::rust("fn main() {}");
        let any_ref = TerminalRenderable::as_any(&block);
        assert!(any_ref.downcast_ref::<CodeBlock>().is_some());
    }

    #[test]
    fn left_margin_is_applied() {
        let mut block = CodeBlock::yaml("foo: 1\nbar: 2");
        block.layout_mut().margin.left = TargetValue::universal(Length::ch(4));

        let out = TerminalRenderable::render(&block, &Terminal::default());
        let plain = crate::testing::strip_ansi_codes(&out);
        for line in plain.lines().filter(|l| !l.trim().is_empty()) {
            assert!(
                line.starts_with("    "),
                "expected 4-space left margin, got: {line:?}"
            );
        }
    }

    /// `CodeBlock` is block-level and the `YamlBlock` parity tests
    /// reuse this guarantee, so guard it explicitly.
    #[test]
    #[serial]
    fn dark_and_light_render_differ() {
        let _no_color = EnvVarGuard::capture("NO_COLOR");
        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        unsafe { std::env::remove_var("NO_COLOR") };
        unsafe { std::env::set_var("CODE_THEME", "github") };

        let block = CodeBlock::yaml("foo: 1\nbar: 2");

        let mut dark = Terminal::new_optimistic(80);
        dark.color_mode = biscuit_terminal::discovery::detection::ColorMode::Dark;
        let dark_out = TerminalRenderable::render(&block, &dark);

        let mut light = Terminal::new_optimistic(80);
        light.color_mode = biscuit_terminal::discovery::detection::ColorMode::Light;
        let light_out = TerminalRenderable::render(&block, &light);

        assert_ne!(dark_out, light_out, "dark/light renders must differ");
    }

    /// Empty blocks still produce a valid highlighted code block (matching
    /// the empty-YAML `YamlBlock` parity expectation).
    #[test]
    fn empty_code_still_renders() {
        let block = CodeBlock::yaml("");
        let out = TerminalRenderable::render(&block, &Terminal::new_optimistic(80));
        assert!(out.contains("\x1b["));
        let html = BrowserRenderable::render_html_fragment(&block).render();
        assert!(html.contains("language-yaml"));
    }

    /// A `CodeBlock::rust(...)` carrying a `title` produces a terminal
    /// header containing the title.
    #[test]
    fn title_reaches_terminal_header() {
        let meta = CodeBlockMeta {
            title: Some("Demo".to_string()),
            ..Default::default()
        };
        let block = CodeBlock::rust("fn main() {}").with_meta(meta);
        let out = TerminalRenderable::render(&block, &Terminal::new_optimistic(80));
        let plain = crate::testing::strip_ansi_codes(&out);
        assert!(plain.contains("Demo"), "expected title in terminal output: {plain:?}");
    }

    /// The browser fragment must wrap a rust block in `<pre><code class="language-rust">`.
    #[test]
    fn browser_emits_rust_language_class() {
        let block = CodeBlock::rust("fn main() {}");
        let html = BrowserRenderable::render_html_fragment(&block).render();
        assert!(html.contains("language-rust"), "html: {html}");
    }

    /// `from_source_file` for a `.yaml` file should resolve the YAML
    /// language through the extension alias.
    #[test]
    fn from_source_file_yaml_extension() {
        let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
        file.write_all(b"foo: 1\n").unwrap();
        let block = CodeBlock::from_source_file(file.path()).unwrap();
        assert!(matches!(block.language(), Some(LanguageGrammar::Yaml)));
    }

    /// Phase 3.1: a fenced ` ```rust ` block in a markdown document must
    /// route through `CodeBlock`'s shared helper, so the markdown-rendered
    /// output is byte-for-byte equal to a direct `CodeBlock::rust(...).render`
    /// for the same code, language, and metadata.
    #[test]
    #[serial]
    fn fenced_rust_block_routes_through_code_block() {
        use crate::markdown::output::terminal::TerminalOptions;
        use crate::markdown::Markdown;

        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        let _theme = EnvVarGuard::capture("THEME");
        unsafe {
            std::env::remove_var("CODE_THEME");
            std::env::remove_var("THEME");
        }

        let code = "fn main() {}\n";

        // Direct: `CodeBlock::rust(code).render(term)`
        let block = CodeBlock::rust(code);
        let term = Terminal::new_optimistic(80);
        let direct = TerminalRenderable::render(&block, &term);

        // Via markdown fence: same code, same language, same terminal.
        let md: Markdown = format!("```rust\n{code}```\n").into();
        let opts = TerminalOptions {
            color_depth: Some(crate::markdown::output::terminal::ColorDepth::TrueColor),
            ..Default::default()
        };
        let fenced = md.as_terminal(opts).unwrap();

        assert_eq!(
            direct, fenced,
            "fenced rust block must equal CodeBlock::rust(...).render"
        );
    }

    /// Phase 3.1: a fenced ` ```rust ` block in a markdown document must
    /// produce browser output byte-for-byte equal to a direct
    /// `CodeBlock::rust(...).render_html_fragment`.
    ///
    /// Strengthened (review-4 finding 2 / gap 1): the prior assertion only
    /// checked that each output *contained* a `<pre><code class="language-rust">`
    /// wrapper, which can pass even if the highlighted bodies diverge. We now
    /// extract the `.code-block` fragment from each surface and assert it is
    /// byte-for-byte identical.
    ///
    /// `#[serial]` + env guards: this case exercises the default-theme routing,
    /// and the direct browser path reads `CODE_THEME` / `THEME` at render time.
    /// Without isolation it races the serial tests that set `CODE_THEME`, so the
    /// two surfaces could resolve different default themes. The metadata/theme
    /// parity tests below pin an explicit theme and need no such guard.
    #[test]
    #[serial]
    fn fenced_rust_block_browser_routes_through_code_block() {
        use crate::markdown::Markdown;

        let _code_theme = EnvVarGuard::capture("CODE_THEME");
        let _theme = EnvVarGuard::capture("THEME");
        unsafe {
            std::env::remove_var("CODE_THEME");
            std::env::remove_var("THEME");
        }

        let code = "fn main() {}\n";

        let block = CodeBlock::rust(code);
        let direct = BrowserRenderable::render_html_fragment(&block).render();

        let md: Markdown = format!("```rust\n{code}```\n").into();
        let html_options = crate::markdown::output::html::HtmlOptions::default();
        let fenced = md.as_html(html_options).unwrap();

        assert_eq!(
            extract_code_block_fragment(&direct),
            extract_code_block_fragment(&fenced),
            "fenced rust block must produce a byte-identical .code-block fragment to \
             CodeBlock::rust(...).render_html_fragment;\n  direct: {direct}\n  fenced: {fenced}"
        );
    }

    /// Finding 2 / gap 4 (review-4): the spec's parity guarantee
    /// ("`CodeBlock`-direct output equals fenced-code-in-`DarkmatterPage` output
    /// for the same code, language, metadata, theme, and surface") was never
    /// verified for a metadata- and theme-bearing block on the same surface.
    /// This drives `title` + `line-numbering` + `highlight` + an explicit
    /// paired-theme override through both the direct `CodeBlock` terminal path
    /// and the fenced-in-`DarkmatterPage` terminal path on the same `Terminal`,
    /// and asserts byte-for-byte equality.
    #[test]
    fn fenced_block_with_metadata_and_theme_equals_direct_terminal() {
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;
        use crate::markdown::output::terminal::ColorDepth;

        let code = "fn a() {}\nfn b() {}\nfn c() {}\n";
        // A paired theme so the inverse-mode resolution is observable and the
        // override genuinely differs from the surface default.
        let theme = ThemePair::from_str_or_default("one-half");
        let raw_meta = "title=\"Demo Snippet\" line-numbering=true highlight=2";
        let meta = CodeBlock::meta_from_fence("rust", Some(raw_meta)).unwrap();

        let term = Terminal::new_optimistic(80);

        // Direct: CodeBlock with metadata + explicit theme override.
        let block = CodeBlock::rust(code).with_meta(meta).with_theme(theme);
        let direct = TerminalRenderable::render(&block, &term);

        // Fenced inside a default-layout DarkmatterPage with the same code,
        // metadata, theme, and terminal. `with_page_code_theme` / `with_color_depth`
        // do not flip `is_default_layout`, so this stays on the zero-config path
        // that is byte-for-byte equal to `Markdown::as_terminal`.
        let md: Markdown = format!("```rust {raw_meta}\n{code}```\n").into();
        let fenced = DarkmatterPage::new(&term)
            .with_color_depth(ColorDepth::TrueColor)
            .with_page_code_theme(theme)
            .render(&md)
            .unwrap();

        assert_eq!(
            direct, fenced,
            "direct CodeBlock with title/line-numbering/highlight/theme must equal the \
             fenced block rendered through DarkmatterPage on the same terminal surface"
        );
    }

    /// Finding 2 / gap 4 (review-4): the browser counterpart to
    /// [`fenced_block_with_metadata_and_theme_equals_direct_terminal`]. Drives
    /// the same metadata + paired-theme override through the direct `CodeBlock`
    /// browser path and `DarkmatterPage::render_to_browser`, then compares the
    /// extracted `.code-block` fragment byte-for-byte (the page wrapper and its
    /// stylesheet are excluded — only the code-panel markup is the parity
    /// oracle).
    #[test]
    fn fenced_block_with_metadata_and_theme_equals_direct_browser() {
        use crate::layout::DarkmatterPage;
        use crate::markdown::Markdown;

        let code = "fn a() {}\nfn b() {}\nfn c() {}\n";
        let theme = ThemePair::from_str_or_default("one-half");
        let raw_meta = "title=\"Demo Snippet\" line-numbering=true highlight=2";
        let meta = CodeBlock::meta_from_fence("rust", Some(raw_meta)).unwrap();

        // Direct: CodeBlock with metadata + explicit theme override.
        let block = CodeBlock::rust(code).with_meta(meta).with_theme(theme);
        let direct = BrowserRenderable::render_html_fragment(&block).render();

        // Fenced through DarkmatterPage::render_to_browser. The page resolves
        // its browser mode from the captured `Terminal` (new_optimistic =>
        // Dark), matching the direct path's HtmlOptions::default() Dark mode, so
        // the inverse-resolved paired theme lands on the same variant.
        let md: Markdown = format!("```rust {raw_meta}\n{code}```\n").into();
        let term = Terminal::new_optimistic(80);
        let page_html = DarkmatterPage::new(&term)
            .with_page_code_theme(theme)
            .render_to_browser(&md)
            .unwrap();

        assert_eq!(
            extract_code_block_fragment(&direct),
            extract_code_block_fragment(&page_html),
            "direct CodeBlock browser fragment must equal the fenced-in-DarkmatterPage \
             .code-block fragment;\n  direct: {direct}\n  page: {page_html}"
        );
    }

    /// Extracts the `.code-block` HTML fragment (optionally preceded by its
    /// `.code-block-title`) so direct-vs-fenced parity can be asserted on the
    /// stable code-panel markup, independent of any surrounding page wrapper or
    /// stylesheet. The code-block `<div>` has no nested `<div>` children (its
    /// body is a `<table>` or `<pre>`), so the first `</div>` after it is its
    /// close.
    fn extract_code_block_fragment(html: &str) -> String {
        let block_marker = "<div class=\"code-block\">";
        let block_start = html
            .find(block_marker)
            .unwrap_or_else(|| panic!("code-block div not found in: {html}"));
        let title_marker = "<div class=\"code-block-title\">";
        let start = html[..block_start]
            .rfind(title_marker)
            .unwrap_or(block_start);
        let close_rel = html[block_start..]
            .find("</div>")
            .unwrap_or_else(|| panic!("code-block close not found in: {html}"));
        let end = block_start + close_rel + "</div>".len();
        html[start..end].to_string()
    }
}
