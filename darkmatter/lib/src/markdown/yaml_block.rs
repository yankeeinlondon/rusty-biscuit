//! YAML block component for validated, renderable YAML content.
//!
//! [`YamlBlock`] is a typed wrapper around a validated YAML string that
//! renders through the same terminal and browser code-block highlighting
//! paths used by normal Markdown `yaml` fences.

use std::any::Any;
use std::path::Path;

use biscuit_terminal::components::renderable::{BrowserRenderable, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Layout, LayoutTerminalExt};
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::tree::{CodeRenderHints, RenderNode};
use thiserror::Error;

use crate::markdown::{
    Markdown, MarkdownError,
    dsl::CodeBlockMeta,
    highlighting::{CodeHighlighter, detect_color_mode},
    output::code_block::{render_html_code_block, render_terminal_code_block},
    output::html::HtmlOptions,
    output::terminal::{TerminalOptions, format_header_row},
};

/// Errors that can occur when constructing or working with a [`YamlBlock`].
#[derive(Debug, Error)]
pub enum YamlBlockError {
    /// Failed to read YAML source from disk.
    #[error("Failed to read YAML source: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse YAML.
    #[error("Failed to parse YAML: {0}")]
    YamlParse(#[from] serde_yaml_ng::Error),

    /// Failed to parse markdown frontmatter.
    #[error("Failed to parse markdown frontmatter: {0}")]
    MarkdownParse(#[from] MarkdownError),
}

/// A validated YAML payload that renders as a syntax-highlighted code block.
///
/// `YamlBlock` stores the raw YAML text after validation. It does not retain
/// the parsed `serde_yaml_ng::Value`, keeping the public API small and
/// avoiding dependency leakage.
#[derive(Debug, Clone, PartialEq)]
pub struct YamlBlock {
    yaml: String,
    layout: Layout,
}

impl YamlBlock {
    /// Constructs a [`YamlBlock`] from a raw YAML string.
    ///
    /// The provided YAML is validated by parsing it through `serde_yaml_ng`.
    /// If validation succeeds, the original text is stored unchanged.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::YamlBlock;
    ///
    /// let block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
    /// assert!(block.yaml().contains("foo: 1"));
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns [`YamlBlockError::YamlParse`] if the input fails `serde_yaml_ng` parsing.
    pub fn new<T: Into<String>>(yaml: T) -> Result<Self, YamlBlockError> {
        let yaml = yaml.into();
        validate_yaml(&yaml)?;
        Ok(Self {
            yaml,
            layout: Layout::default(),
        })
    }

    /// Constructs a [`YamlBlock`] from a YAML file on disk.
    ///
    /// Reads the file contents and delegates to [`YamlBlock::new`].
    ///
    /// ## Errors
    ///
    /// Returns [`YamlBlockError::Io`] if the file cannot be read, or
    /// [`YamlBlockError::YamlParse`] if the file contains malformed YAML.
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self, YamlBlockError> {
        let content = std::fs::read_to_string(path)?;
        Self::new(content)
    }

    /// Constructs a [`YamlBlock`] from raw Markdown content, extracting only
    /// the frontmatter.
    ///
    /// The markdown body is ignored. If the document has no frontmatter, the
    /// resulting `YamlBlock` contains an empty mapping (`{}`).
    ///
    /// ## Errors
    ///
    /// Returns [`YamlBlockError::MarkdownParse`] if frontmatter extraction
    /// fails, or [`YamlBlockError::YamlParse`] if the re-serialized frontmatter
    /// fails validation (which should not normally happen).
    ///
    /// ## Notes
    ///
    /// Frontmatter is round-tripped through Darkmatter's structured
    /// `FrontmatterMap` before being re-serialized via `serde_yaml_ng`. As a
    /// result, the rendered YAML payload is **not** a byte-for-byte copy of the
    /// source frontmatter:
    ///
    /// - Comments are dropped (`FrontmatterMap` does not retain comment nodes).
    /// - Custom YAML tags (`!tag`) and anchors/aliases (`&anchor` / `*alias`)
    ///   are lost — values are normalised through `serde_json::Value`.
    /// - Whitespace is normalised to canonical YAML output (single space after
    ///   `:`, no extra blank lines).
    /// - Key order **is** preserved because `FrontmatterMap` is backed by an
    ///   `IndexMap`.
    ///
    /// If you need byte-exact source preservation (comments, anchors, custom
    /// whitespace), call [`YamlBlock::new`] with the raw frontmatter text
    /// instead.
    pub fn from_markdown_content<T: Into<String>>(md: T) -> Result<Self, YamlBlockError> {
        let md = Markdown::try_from_content(md)?;
        let yaml = if md.frontmatter().is_empty() {
            "{}".to_string()
        } else {
            serde_yaml_ng::to_string(md.frontmatter().as_map())?
        };
        validate_yaml(&yaml)?;
        Ok(Self {
            yaml,
            layout: Layout::default(),
        })
    }

    /// Constructs a [`YamlBlock`] from a Markdown file on disk.
    ///
    /// Reads the file contents and delegates to [`YamlBlock::from_markdown_content`].
    ///
    /// ## Errors
    ///
    /// Returns [`YamlBlockError::Io`] if the file cannot be read,
    /// [`YamlBlockError::MarkdownParse`] if frontmatter extraction fails, or
    /// [`YamlBlockError::YamlParse`] if the frontmatter is malformed.
    pub fn from_markdown_file<P: AsRef<Path>>(path: P) -> Result<Self, YamlBlockError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_markdown_content(content)
    }

    /// Returns a reference to the stored YAML text.
    pub fn yaml(&self) -> &str {
        &self.yaml
    }

    /// Consumes the block and returns the stored YAML text.
    pub fn into_yaml(self) -> String {
        self.yaml
    }
}

/// Validates YAML by attempting to parse it as `serde_yaml_ng::Value`.
fn validate_yaml(yaml: &str) -> Result<(), serde_yaml_ng::Error> {
    let _: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml)?;
    Ok(())
}

/// Emits the same ANSI-highlighted `yaml` code block that a Markdown
/// ` ```yaml ` fence produces (header row + highlighted body) and applies the
/// stored [`Layout`] (margins, alignment, word-wrap) to the result.
impl TerminalRenderable for YamlBlock {
    fn render(&self, term: &Terminal) -> String {
        // Detect color mode fresh so env-var changes between renders are
        // honoured (e.g. dark/light tests that flip COLORFGBG).
        // A YamlBlock is a code block: it contrasts against the page by
        // resolving its theme *variant* against the INVERTED detected mode (see
        // `ColorMode::inverted`), keeping it byte-identical to a Markdown
        // ```yaml``` fence.
        let options = TerminalOptions::default();
        let highlighter = CodeHighlighter::new(options.code_theme, detect_color_mode().inverted());
        // Header/body contrast keys off the resolved theme background so
        // single-variant themes still get readable chrome.
        let color_mode = crate::markdown::output::code_block::mode_for_background(
            highlighter
                .theme()
                .settings
                .background
                .unwrap_or(syntect::highlighting::Color::BLACK),
        );
        let meta = CodeBlockMeta::default();
        let terminal_width = term.width();

        // Resolve the width left for the block after the layout's horizontal
        // margins and any `max_width` cap. Both the header pill (right-aligned
        // within this width) and the body (padded to this width) use it, so the
        // block honours the available width — a right margin or a `max_width`
        // cap pulls the block, and the pill above it, leftward off the physical
        // edge, matching the render-tree path.
        let available = self.layout.content_width(terminal_width);

        // Emit the same header row Markdown's ``` yaml ``` fence emits, so
        // YamlBlock and Markdown stay parity-equivalent in the body region.
        let bg_color = highlighter
            .theme()
            .settings
            .background
            .unwrap_or(syntect::highlighting::Color::BLACK);
        let header = format_header_row(
            meta.title.as_deref(),
            "yaml",
            bg_color,
            color_mode,
            available as u16,
        );

        // Pad the body to the available width rather than clearing to the
        // physical terminal edge (`\x1b[K`); clear-to-EOL ignores the supplied
        // width and can never respect a right margin.
        let body = render_terminal_code_block(
            self.yaml(),
            "yaml",
            &highlighter,
            &options,
            &meta,
            color_mode,
            Some(available as u16),
        )
        .unwrap_or_else(|_| {
            // Fallback: plain text with minimal escaping
            format!("\n{}\n", self.yaml())
        });

        let raw = format!("{header}\n{body}");

        // Apply stored layout (margins, alignment, word-wrap, row-fill).
        self.layout.apply_layout(&raw, terminal_width)
    }

    /// Renders without environment detection by delegating to [`TerminalRenderable::render`]
    /// with a default [`Terminal`]. Once `render` honours the stored layout this
    /// override is behaviourally equivalent to the trait default for a width of
    /// `None`; we keep it explicit so future divergence is intentional.
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

    /// Projects this block into a [`NodeKind::Code`](renderable::tree::NodeKind::Code)
    /// render-tree node tagged with the `yaml` language.
    ///
    /// The node carries [`CodeRenderHints`] requesting a header row, a `yaml`
    /// language label, and syntax highlighting, so a tree renderer wired with
    /// a `CodeRenderer` hook can reproduce the bespoke highlighted output.
    fn render_tree_node(&self) -> Option<RenderNode> {
        let mut node = RenderNode::code(Some("yaml".into()), None, self.yaml.clone());
        node.attrs.set_code_hints(&CodeRenderHints {
            header_row: true,
            language_label: Some("yaml".into()),
            highlight: true,
        });
        if self.layout != Layout::default() {
            node.attrs.set_layout(&self.layout);
        }
        Some(node)
    }
}

/// Emits `<pre><code class="language-yaml">…</code></pre>` inside the standard
/// darkmatter `<div class="code-block">` wrapper, using the same shared
/// `render_html_code_block` helper that Markdown ` ```yaml ` fences use.
/// `BrowserRenderable` does not have layout state to apply; layout for HTML
/// output is handled by surrounding CSS.
impl BrowserRenderable for YamlBlock {
    /// Wraps the highlighted code-block HTML as a [`ComposableNode::RawHtml`]
    /// island.
    ///
    /// The HTML is caller-owned, already-formed markup (highlighted spans,
    /// escaped content); emitting it as a `RawHtml` node tells the renderer
    /// to pass it through verbatim rather than HTML-escaping it.
    ///
    /// [`ComposableNode::RawHtml`]: renderable::browser::fragment::ComposableNode::RawHtml
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        BrowserFragment::new()
            .define_as_raw_html(self.render_browser_html())
            .finalize()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl YamlBlock {
    fn render_browser_html(&self) -> String {
        // Route through HtmlOptions::default() for symmetry with the terminal
        // path. Reuse the resolved color mode and theme rather than re-detecting.
        let options = HtmlOptions::default();
        // Browser code blocks do not invert (terminal-only contrast); see the
        // note in `darkmatter::markdown::output::html::as_html`.
        let highlighter = CodeHighlighter::new(options.code_theme, options.color_mode);
        let meta = CodeBlockMeta::default();

        render_html_code_block(self.yaml(), "yaml", &meta, &highlighter, &options).unwrap_or_else(
            |_| {
                format!(
                    "<pre><code class=\"language-yaml\">{}</code></pre>",
                    html_escape::encode_text(self.yaml())
                )
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::highlighting::{ColorMode, detect_color_mode};
    use biscuit_terminal::utils::layout::{Length, TargetValue};
    use serial_test::serial;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Restores an env var to its prior value, removing it if previously unset.
    ///
    /// Using a small RAII guard keeps env-touching tests symmetric: every
    /// `set_var`/`remove_var` is paired with a deterministic restore at the
    /// end of the test, even on assertion failure.
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
    fn test_new_valid_yaml() {
        let block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
        assert_eq!(block.yaml(), "foo: 1\nbar: 2");
    }

    #[test]
    fn test_new_invalid_yaml() {
        let result = YamlBlock::new("foo: : :");
        assert!(matches!(result, Err(YamlBlockError::YamlParse(_))));
    }

    #[test]
    fn test_from_yaml_file_valid() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "foo: 1").unwrap();
        writeln!(file, "bar: 2").unwrap();

        let block = YamlBlock::from_yaml_file(file.path()).unwrap();
        assert!(block.yaml().contains("foo: 1"));
        assert!(block.yaml().contains("bar: 2"));
    }

    #[test]
    fn test_from_yaml_file_missing() {
        let result = YamlBlock::from_yaml_file("/nonexistent/path/file.yaml");
        assert!(matches!(result, Err(YamlBlockError::Io(_))));
    }

    #[test]
    fn test_from_yaml_file_malformed() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "foo: : :").unwrap();

        let result = YamlBlock::from_yaml_file(file.path());
        assert!(matches!(result, Err(YamlBlockError::YamlParse(_))));
    }

    #[test]
    fn test_from_markdown_content_no_frontmatter() {
        let block = YamlBlock::from_markdown_content("# Hello\n\nWorld").unwrap();
        assert_eq!(block.yaml().trim(), "{}");
    }

    #[test]
    fn test_from_markdown_content_with_frontmatter() {
        let md = "---\nfoo: 1\nbar: 2\n---\n# Hello\n";
        let block = YamlBlock::from_markdown_content(md).unwrap();
        assert!(block.yaml().contains("foo: 1"));
        assert!(block.yaml().contains("bar: 2"));
        // Body should not be included
        assert!(!block.yaml().contains("Hello"));
    }

    #[test]
    fn test_from_markdown_content_malformed_frontmatter() {
        let md = "---\nfoo: : :\n---\n# Hello\n";
        let result = YamlBlock::from_markdown_content(md);
        assert!(matches!(result, Err(YamlBlockError::MarkdownParse(_))));
    }

    #[test]
    fn test_from_markdown_file_valid() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "---").unwrap();
        writeln!(file, "title: Test").unwrap();
        writeln!(file, "---").unwrap();
        writeln!(file, "# Content").unwrap();

        let block = YamlBlock::from_markdown_file(file.path()).unwrap();
        assert!(block.yaml().contains("title: Test"));
        assert!(!block.yaml().contains("Content"));
    }

    #[test]
    fn test_from_markdown_file_missing() {
        let result = YamlBlock::from_markdown_file("/nonexistent/path/file.md");
        assert!(matches!(result, Err(YamlBlockError::Io(_))));
    }

    #[test]
    fn test_yaml_accessor() {
        let block = YamlBlock::new("key: value").unwrap();
        assert_eq!(block.yaml(), "key: value");
    }

    #[test]
    fn test_into_yaml() {
        let block = YamlBlock::new("key: value").unwrap();
        assert_eq!(block.into_yaml(), "key: value");
    }

    #[test]
    fn test_empty_yaml() {
        let result = YamlBlock::new("");
        assert!(
            result.is_ok(),
            "empty YAML should be accepted: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_renderable_traits() {
        let block = YamlBlock::new("foo: 1").unwrap();
        assert!(block.is_block_level());
        // as_any should return the YamlBlock
        let any_ref = TerminalRenderable::as_any(&block);
        assert!(any_ref.downcast_ref::<YamlBlock>().is_some());
    }

    #[test]
    fn test_browser_renderable_traits() {
        let block = YamlBlock::new("foo: 1").unwrap();
        let any_ref = BrowserRenderable::as_any(&block);
        assert!(any_ref.downcast_ref::<YamlBlock>().is_some());
    }

    #[test]
    fn test_terminal_render_contains_ansi_and_content() {
        let block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
        let term = Terminal::new();
        let output = TerminalRenderable::render(&block, &term);

        // Should contain ANSI escape codes for syntax highlighting
        assert!(
            output.contains("\x1b["),
            "Terminal output should contain ANSI codes"
        );

        // Should contain the YAML content when ANSI is stripped
        let plain = crate::testing::strip_ansi_codes(&output);
        assert!(
            plain.contains("foo: 1"),
            "Terminal output should contain YAML content"
        );
        assert!(
            plain.contains("bar: 2"),
            "Terminal output should contain YAML content"
        );
    }

    /// §3.1 — Strict body-substring parity test (terminal).
    ///
    /// The Markdown YAML fence wrapper adds outer paragraph spacing on top of
    /// the same shared `render_terminal_code_block` helper that `YamlBlock`
    /// uses. After stripping ANSI from both outputs, the YamlBlock's plain
    /// body lines must appear contiguously inside the Markdown-rendered plain
    /// output. We also assert both outputs contain the post-Phase-1 header
    /// substring (`" yaml "`) so the header-row promise stays enforced.
    #[test]
    fn test_terminal_render_parity_with_markdown_yaml_fence() {
        let yaml_content = "foo: 1\nbar: 2";

        // Render via YamlBlock
        let block = YamlBlock::new(yaml_content).unwrap();
        let term = Terminal::new();
        let block_output = TerminalRenderable::render(&block, &term);

        // Render via Markdown YAML fence
        let md_content = format!("```yaml\n{}\n```", yaml_content);
        let md: Markdown = md_content.into();
        let md_output = md.as_terminal(TerminalOptions::default()).unwrap();

        // Header parity: both must contain the " yaml " language label.
        assert!(
            block_output.contains(" yaml "),
            "YamlBlock terminal output should contain the ' yaml ' header label. Got: {block_output:?}"
        );
        assert!(
            md_output.contains(" yaml "),
            "Markdown fence terminal output should contain the ' yaml ' header label. Got: {md_output:?}"
        );

        // Strip ANSI from both, then assert contiguous-substring parity on the
        // body lines (the lines that contain the actual YAML payload). We
        // tolerate wrapper differences (extra trailing newlines, surrounding
        // blank lines) but reject structural drift in the body.
        //
        // Trailing whitespace is trimmed per line before comparison: `YamlBlock`
        // pads each line to the available width, whereas the Markdown fence
        // clears to the terminal edge with `\x1b[K` (no trailing spaces once
        // stripped). The padding spaces are invisible formatting, not a body
        // difference, so they are not part of the parity contract.
        let trim_trailing = |s: &str| {
            s.lines()
                .map(|l| l.trim_end())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let block_plain = trim_trailing(&crate::testing::strip_ansi_codes(&block_output));
        let md_plain = trim_trailing(&crate::testing::strip_ansi_codes(&md_output));

        for needle in ["foo: 1", "bar: 2"] {
            assert!(
                block_plain.contains(needle),
                "YamlBlock plain output missing {needle:?}: {block_plain:?}"
            );
            assert!(
                md_plain.contains(needle),
                "Markdown plain output missing {needle:?}: {md_plain:?}"
            );
        }

        // Find the contiguous body region from YamlBlock (from "foo: 1" to
        // end of "bar: 2") and assert that exact slice appears verbatim in
        // the Markdown output.
        let body_start = block_plain
            .find("foo: 1")
            .expect("foo: 1 in YamlBlock plain");
        let body_end_offset = block_plain[body_start..]
            .find("bar: 2")
            .expect("bar: 2 after foo: 1 in YamlBlock plain");
        let body_end = body_start + body_end_offset + "bar: 2".len();
        let yaml_body_slice = &block_plain[body_start..body_end];

        assert!(
            md_plain.contains(yaml_body_slice),
            "Markdown plain output should contain YamlBlock's body slice verbatim.\n  slice: {yaml_body_slice:?}\n  md_plain: {md_plain:?}"
        );
    }

    #[test]
    fn test_browser_render_contains_language_yaml() {
        let block = YamlBlock::new("foo: 1").unwrap();
        let html = block.render_browser_html();

        // Should contain language-yaml class
        assert!(
            html.contains("language-yaml"),
            "Browser output should contain language-yaml class. Got: {}",
            html
        );
    }

    #[test]
    fn test_browser_render_escapes_html_in_yaml() {
        let block = YamlBlock::new("script: \"<script>alert('xss')</script>\"").unwrap();
        let html = block.render_browser_html();

        // Should escape HTML special characters
        assert!(
            html.contains("&lt;script&gt;") || html.contains("&#60;script&#62;"),
            "Browser output should escape <script> tags. Got: {}",
            html
        );
    }

    /// §3.1 — Strict body-substring parity test (browser).
    ///
    /// Both `YamlBlock::render_browser_html` and the Markdown YAML fence route
    /// through `render_html_code_block`, so they must emit byte-identical
    /// highlighted span sequences for the YAML payload. We extract the
    /// `<pre><code class="language-yaml">` opener through the final-key span
    /// (the longest contiguous prefix shared between both renderings, before
    /// the Markdown wrapper appends a trailing `\n` span that the raw
    /// `YamlBlock` payload does not contain) and assert it appears verbatim
    /// in the Markdown HTML output.
    ///
    /// We also assert both outputs contain the `language-yaml` class and the
    /// canonical `<pre><code…>` opener byte-for-byte.
    #[test]
    fn test_browser_render_parity_with_markdown_yaml_fence() {
        let yaml_content = "foo: 1\nbar: 2";

        // Render via YamlBlock
        let block = YamlBlock::new(yaml_content).unwrap();
        let block_html = block.render_browser_html();

        // Render via Markdown YAML fence
        let md_content = format!("```yaml\n{}\n```", yaml_content);
        let md: Markdown = md_content.into();
        let md_html = md.as_html(HtmlOptions::default()).unwrap();

        // Both must contain the canonical <pre><code class="language-yaml"> opener.
        let pre_open = "<pre><code class=\"language-yaml\">";
        assert!(
            block_html.contains(pre_open),
            "YamlBlock HTML missing canonical opener: {block_html:?}"
        );
        assert!(
            md_html.contains(pre_open),
            "Markdown HTML missing canonical opener: {md_html:?}"
        );

        // Extract YamlBlock's <pre>…</pre> segment in full.
        let pre_close = "</code></pre>";
        let block_pre_start = block_html
            .find(pre_open)
            .expect("YamlBlock HTML should contain the opener");
        let block_pre_end_offset = block_html[block_pre_start..]
            .find(pre_close)
            .expect("YamlBlock HTML should contain </code></pre>");
        let block_pre_segment_full =
            &block_html[block_pre_start..block_pre_start + block_pre_end_offset + pre_close.len()];

        // The YamlBlock segment may differ from the Markdown segment by a
        // single trailing `<span>…\n</span>` span (Markdown wrappers append a
        // newline to the fenced content; raw YamlBlock without trailing `\n`
        // does not). Compare contiguous substrings: trim the YamlBlock
        // segment back to just before its closing `</code></pre>` and assert
        // that prefix appears verbatim in the Markdown HTML.
        let block_pre_inner = block_pre_segment_full
            .strip_suffix(pre_close)
            .expect("segment ends with </code></pre>");

        assert!(
            md_html.contains(block_pre_inner),
            "Markdown HTML should contain the YamlBlock's <pre>…<last-span> prefix verbatim.\n  prefix: {block_pre_inner:?}\n  md_html: {md_html:?}"
        );
    }

    /// §2.2 — `COLORFGBG` set to a dark background should produce dark mode.
    ///
    /// `COLORFGBG="15;0"` means foreground=15 (white) on background=0 (black);
    /// background < 7 ⇒ Dark per `detect_color_mode`'s contract.
    #[test]
    #[serial]
    fn test_dark_mode_via_colorfgbg() {
        let _no_color_guard = EnvVarGuard::capture("NO_COLOR");
        let _colorfgbg_guard = EnvVarGuard::capture("COLORFGBG");

        unsafe { std::env::remove_var("NO_COLOR") };
        unsafe { std::env::set_var("COLORFGBG", "15;0") };

        assert_eq!(detect_color_mode(), ColorMode::Dark);

        let block = YamlBlock::new("key: value").unwrap();
        let dark_out = TerminalRenderable::render(&block, &Terminal::default());
        assert!(
            dark_out.contains("\x1b["),
            "Dark-mode render should still emit ANSI escape sequences"
        );
    }

    /// §2.2 — `COLORFGBG` set to a light background should produce light mode.
    ///
    /// `COLORFGBG="0;15"` means foreground=0 (black) on background=15 (white);
    /// background ≥ 7 ⇒ Light per `detect_color_mode`'s contract.
    #[test]
    #[serial]
    fn test_light_mode_via_colorfgbg() {
        let _no_color_guard = EnvVarGuard::capture("NO_COLOR");
        let _colorfgbg_guard = EnvVarGuard::capture("COLORFGBG");

        unsafe { std::env::remove_var("NO_COLOR") };
        unsafe { std::env::set_var("COLORFGBG", "0;15") };

        assert_eq!(detect_color_mode(), ColorMode::Light);

        let block = YamlBlock::new("key: value").unwrap();
        let light_out = TerminalRenderable::render(&block, &Terminal::default());
        assert!(
            light_out.contains("\x1b["),
            "Light-mode render should still emit ANSI escape sequences"
        );
    }

    /// §2.2 — Light vs dark mode must produce visibly different output.
    ///
    /// Different background colors flow through `CodeHighlighter::new`, so the
    /// emitted SGR background sequences (`\x1b[48;2;…m`) should differ. This
    /// confirms that `TerminalRenderable::render` actually exercises the color-mode
    /// branch and isn't ignoring the env var.
    #[test]
    #[serial]
    fn test_dark_and_light_render_differ() {
        let _no_color_guard = EnvVarGuard::capture("NO_COLOR");
        let _colorfgbg_guard = EnvVarGuard::capture("COLORFGBG");
        // Pin a *paired* code theme so the assertion is meaningful. A code block
        // resolves its theme against the INVERTED terminal mode for page
        // contrast, so a paired theme yields opposite concrete variants (and
        // thus different backgrounds) between dark and light terminals.
        // Single-variant themes (the ambient default in some environments)
        // legitimately render identically under both modes — keying header text
        // off the resolved background, not the requested mode — so they cannot
        // demonstrate this property.
        let _code_theme_guard = EnvVarGuard::capture("CODE_THEME");

        unsafe { std::env::remove_var("NO_COLOR") };
        unsafe { std::env::set_var("CODE_THEME", "github") };

        let block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
        let term = Terminal::default();

        unsafe { std::env::set_var("COLORFGBG", "15;0") };
        let dark_out = TerminalRenderable::render(&block, &term);

        unsafe { std::env::set_var("COLORFGBG", "0;15") };
        let light_out = TerminalRenderable::render(&block, &term);

        assert_ne!(
            dark_out, light_out,
            "Dark and light renders should differ in their ANSI background sequences"
        );
    }

    /// §3.2 — Layout left margin must be applied to the rendered output.
    ///
    /// Sets a 4-cell left margin on the block's layout and asserts every
    /// non-empty rendered line begins with the expected indent. This is the
    /// behavioural test for Phase 1 §1.1 (rewriting `render` to honour the
    /// stored layout).
    #[test]
    fn test_left_margin_is_applied() {
        let mut block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
        block.layout_mut().margin.left = TargetValue::universal(Length::ch(4));

        let out = TerminalRenderable::render(&block, &Terminal::default());
        let plain = crate::testing::strip_ansi_codes(&out);

        for line in plain.lines().filter(|l| !l.trim().is_empty()) {
            assert!(
                line.starts_with("    "),
                "expected 4-space left margin on every non-empty line, got: {line:?}"
            );
        }
    }

    /// The code block must honour the available terminal width instead of
    /// clearing to the physical terminal edge with `\x1b[K`. Every rendered
    /// line (header, body, padding rows) must be exactly the available width,
    /// which keeps the right-aligned language pill flush with the block's
    /// right edge regardless of the physical terminal size.
    #[test]
    fn test_render_pads_to_available_width_not_clear_to_eol() {
        let block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
        let term = Terminal::new_optimistic(40);
        let out = TerminalRenderable::render(&block, &term);

        assert!(
            !out.contains("\x1b[K"),
            "code block must not flood to the physical edge with clear-to-EOL: {out:?}"
        );

        let plain = crate::testing::strip_ansi_codes(&out);
        for line in plain.lines().filter(|l| !l.is_empty()) {
            assert_eq!(
                line.chars().count(),
                40,
                "every non-empty line must pad to the available width (40): {line:?}"
            );
        }
    }

    /// A right margin must shrink both the code-block background and the
    /// right-aligned language pill, moving them leftward off the right edge.
    #[test]
    fn test_right_margin_shrinks_block_and_pill() {
        let mut block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
        block.layout_mut().margin.right = TargetValue::universal(Length::ch(10));

        let term = Terminal::new_optimistic(40);
        let out = TerminalRenderable::render(&block, &term);
        let plain = crate::testing::strip_ansi_codes(&out);

        // available = 40 - 10 = 30; with no left margin every non-empty line
        // is exactly 30 columns wide, so the block (and the pill above it) end
        // 10 cells short of the physical right edge.
        for line in plain.lines().filter(|l| !l.is_empty()) {
            assert_eq!(
                line.chars().count(),
                30,
                "right margin must shrink the block to the available width (30): {line:?}"
            );
        }
    }

    /// A `max_width` cap must bound the block (and its pill) to the capped
    /// width, matching the render-tree path which applies the cap in
    /// `render_with_layout`.
    #[test]
    fn test_max_width_caps_block() {
        let mut block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
        block.layout_mut().max_width = Some(TargetValue::universal(Length::Percent(50.0)));

        let term = Terminal::new_optimistic(80);
        let out = TerminalRenderable::render(&block, &term);
        let plain = crate::testing::strip_ansi_codes(&out);

        // 50% of 80 = 40; every non-empty line is exactly 40 columns wide.
        for line in plain.lines().filter(|l| !l.is_empty()) {
            assert_eq!(
                line.chars().count(),
                40,
                "max_width 50% must cap the block to 40 columns: {line:?}"
            );
        }
    }

    /// §3.3 — `render_optimistic` smoke test.
    ///
    /// Confirms the optimistic-render path produces ANSI-bearing output that
    /// contains the YAML content. This documents that the override added in
    /// Phase 1 §1.4 stays behaviourally aligned with `render`.
    #[test]
    fn test_render_optimistic_smoke() {
        let block = YamlBlock::new("foo: 1").unwrap();
        let out = TerminalRenderable::render_optimistic(&block, None);
        assert!(out.contains("\x1b["), "optimistic render should emit ANSI");
        assert!(
            crate::testing::strip_ansi_codes(&out).contains("foo: 1"),
            "optimistic render should contain the YAML content"
        );
    }

    /// §3.3 — `render` honours a narrow `Terminal` width.
    ///
    /// Constructs an optimistic terminal at width 40 and asserts the rendered
    /// output is non-empty and ANSI-bearing. The trait-default `render_in_width`
    /// (provided by `TerminalRenderable`) delegates to `render` with this terminal.
    #[test]
    fn test_render_in_width_smoke() {
        let block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
        let term = Terminal::new_optimistic(40);
        let out = TerminalRenderable::render(&block, &term);

        assert!(
            !out.is_empty(),
            "render at narrow width should not be empty"
        );
        assert!(
            out.contains("\x1b["),
            "render at narrow width should still emit ANSI"
        );
    }

    /// §3.4 — File constructor produces identical state to string constructor.
    #[test]
    fn test_from_yaml_file_matches_new() {
        let yaml = "foo: 1\nbar: 2\n";
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let from_file = YamlBlock::from_yaml_file(file.path()).unwrap();
        let from_string = YamlBlock::new(yaml).unwrap();
        assert_eq!(from_file, from_string);
        assert_eq!(from_file.yaml(), from_string.yaml());
    }

    /// §3.5 — `from_markdown_content` preserves frontmatter key order.
    ///
    /// The underlying `FrontmatterMap` is an `IndexMap`, so the original key
    /// order survives the parse/serialize round-trip even when keys are not
    /// alphabetical.
    #[test]
    fn test_from_markdown_content_preserves_key_order() {
        let md = "---\nb: 1\na: 2\nc: 3\n---\n# body\n";
        let block = YamlBlock::from_markdown_content(md).unwrap();
        let yaml = block.yaml();
        let pos_b = yaml.find("b:").expect("b key present");
        let pos_a = yaml.find("a:").expect("a key present");
        let pos_c = yaml.find("c:").expect("c key present");
        assert!(
            pos_b < pos_a && pos_a < pos_c,
            "expected b < a < c in: {yaml}"
        );
    }

    /// §2.5 — Documents the lossy reserialization behaviour: comments are
    /// dropped because `FrontmatterMap` (`IndexMap<String, Value>`) does not
    /// retain comment nodes. Callers needing byte-exact source should use
    /// `YamlBlock::new` with the raw frontmatter text instead.
    #[test]
    fn test_from_markdown_content_drops_comments() {
        let md = "---\n# leading comment\nfoo: 1\n---\nbody\n";
        let block = YamlBlock::from_markdown_content(md).unwrap();
        assert!(
            !block.yaml().contains("leading comment"),
            "comments must be dropped after FrontmatterMap round-trip; got: {}",
            block.yaml()
        );
        assert!(block.yaml().contains("foo: 1"));
    }

    /// §2.5 — Documents that whitespace inside frontmatter is normalized to
    /// canonical YAML output (single space after `:`). Callers needing
    /// byte-exact whitespace should use `YamlBlock::new` directly.
    #[test]
    fn test_from_markdown_content_normalizes_whitespace() {
        let md = "---\nfoo:    1\n---\n";
        let block = YamlBlock::from_markdown_content(md).unwrap();
        assert!(
            block.yaml().contains("foo: 1"),
            "whitespace should be normalized to canonical 'foo: 1'; got: {}",
            block.yaml()
        );
        assert!(
            !block.yaml().contains("foo:    1"),
            "original quad-space whitespace should not survive; got: {}",
            block.yaml()
        );
    }

    #[test]
    fn test_browser_render_structure() {
        let block = YamlBlock::new("nested:\n  key: value").unwrap();
        let html = block.render_browser_html();

        // Should contain pre and code tags
        assert!(
            html.contains("<pre>"),
            "Browser output should contain <pre> tag"
        );
        assert!(
            html.contains("<code"),
            "Browser output should contain <code> tag"
        );
        assert!(
            html.contains("</code>"),
            "Browser output should contain </code> tag"
        );
        assert!(
            html.contains("</pre>"),
            "Browser output should contain </pre> tag"
        );

        // Should contain the YAML content (may be inside syntax-highlighting spans)
        assert!(
            html.contains("nested"),
            "HTML should contain 'nested'. Full HTML: {}",
            html
        );
        assert!(
            html.contains("key"),
            "HTML should contain 'key'. Full HTML: {}",
            html
        );
        assert!(
            html.contains("value"),
            "HTML should contain 'value'. Full HTML: {}",
            html
        );
    }

    #[test]
    fn test_yaml_block_render_is_block_level() {
        let block = YamlBlock::new("foo: 1").unwrap();
        assert!(
            TerminalRenderable::is_block_level(&block),
            "YamlBlock should be block-level"
        );
    }

    #[test]
    fn test_terminal_render_empty_yaml() {
        let block = YamlBlock::new("").unwrap();
        let term = Terminal::new();
        let output = TerminalRenderable::render(&block, &term);

        // Should still produce valid output (padding rows at minimum)
        assert!(
            output.contains("\x1b["),
            "Empty YAML should still produce ANSI output"
        );
    }

    #[test]
    fn test_browser_render_empty_yaml() {
        let block = YamlBlock::new("").unwrap();
        let html = block.render_browser_html();

        // Should contain the code block structure even for empty YAML
        assert!(
            html.contains("language-yaml"),
            "Empty YAML should still have language class"
        );
        assert!(
            html.contains("<pre>"),
            "Empty YAML should still have <pre> tag"
        );
        assert!(
            html.contains("<code"),
            "Empty YAML should still have <code> tag"
        );
    }
}
