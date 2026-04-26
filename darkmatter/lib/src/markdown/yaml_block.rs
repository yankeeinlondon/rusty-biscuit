//! YAML block component for validated, renderable YAML content.
//!
//! [`YamlBlock`] is a typed wrapper around a validated YAML string that
//! renders through the same terminal and browser code-block highlighting
//! paths used by normal Markdown `yaml` fences.

use std::any::Any;
use std::path::Path;

use biscuit_terminal::components::renderable::{BrowserRenderable, Renderable};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Layout;
use thiserror::Error;

use crate::markdown::{
    Markdown, MarkdownError,
    dsl::CodeBlockMeta,
    highlighting::{CodeHighlighter, ThemePair, detect_color_mode},
    output::code_block::{render_html_code_block, render_terminal_code_block},
    output::html::HtmlOptions,
    output::terminal::TerminalOptions,
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

impl Renderable for YamlBlock {
    fn render(&self, _term: &Terminal) -> String {
        let color_mode = detect_color_mode();
        let highlighter = CodeHighlighter::new(ThemePair::Github, color_mode);
        let options = TerminalOptions::default();
        let meta = CodeBlockMeta::default();

        render_terminal_code_block(self.yaml(), "yaml", &highlighter, &options, &meta, color_mode)
            .unwrap_or_else(|_| {
                // Fallback: plain text with minimal escaping
                format!("\n{}\n", self.yaml())
            })
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
}

impl BrowserRenderable for YamlBlock {
    fn render_to_browser(&self) -> String {
        let color_mode = detect_color_mode();
        let highlighter = CodeHighlighter::new(ThemePair::Github, color_mode);
        let options = HtmlOptions::default();
        let meta = CodeBlockMeta::default();

        render_html_code_block(self.yaml(), "yaml", &meta, &highlighter, &options)
            .unwrap_or_else(|_| {
                format!(
                    "<pre><code class=\"language-yaml\">{}</code></pre>",
                    html_escape::encode_text(self.yaml())
                )
            })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

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
        assert!(result.is_ok(), "empty YAML should be accepted: {:?}", result.err());
    }

    #[test]
    fn test_renderable_traits() {
        let block = YamlBlock::new("foo: 1").unwrap();
        assert!(block.is_block_level());
        // as_any should return the YamlBlock
        let any_ref = Renderable::as_any(&block);
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
        let output = Renderable::render(&block, &term);

        // Should contain ANSI escape codes for syntax highlighting
        assert!(output.contains("\x1b["), "Terminal output should contain ANSI codes");

        // Should contain the YAML content when ANSI is stripped
        let plain = crate::testing::strip_ansi_codes(&output);
        assert!(plain.contains("foo: 1"), "Terminal output should contain YAML content");
        assert!(plain.contains("bar: 2"), "Terminal output should contain YAML content");
    }

    #[test]
    fn test_terminal_render_parity_with_markdown_yaml_fence() {
        let yaml_content = "foo: 1\nbar: 2";

        // Render via YamlBlock
        let block = YamlBlock::new(yaml_content).unwrap();
        let term = Terminal::new();
        let block_output = Renderable::render(&block, &term);

        // Render via Markdown YAML fence
        let md_content = format!("```yaml\n{}\n```", yaml_content);
        let md: Markdown = md_content.into();
        let md_output = md.as_terminal(TerminalOptions::default()).unwrap();

        // Both should contain ANSI codes
        assert!(block_output.contains("\x1b["), "YamlBlock should produce ANSI output");
        assert!(md_output.contains("\x1b["), "Markdown YAML fence should produce ANSI output");

        // Both should contain the YAML content
        let block_plain = crate::testing::strip_ansi_codes(&block_output);
        let md_plain = crate::testing::strip_ansi_codes(&md_output);
        assert!(block_plain.contains("foo: 1"));
        assert!(md_plain.contains("foo: 1"));
    }

    #[test]
    fn test_browser_render_contains_language_yaml() {
        let block = YamlBlock::new("foo: 1").unwrap();
        let html = BrowserRenderable::render_to_browser(&block);

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
        let html = BrowserRenderable::render_to_browser(&block);

        // Should escape HTML special characters
        assert!(
            html.contains("&lt;script&gt;") || html.contains("&#60;script&#62;"),
            "Browser output should escape <script> tags. Got: {}",
            html
        );
    }

    #[test]
    fn test_browser_render_parity_with_markdown_yaml_fence() {
        let yaml_content = "foo: 1\nbar: 2";

        // Render via YamlBlock
        let block = YamlBlock::new(yaml_content).unwrap();
        let block_html = BrowserRenderable::render_to_browser(&block);

        // Render via Markdown YAML fence
        let md_content = format!("```yaml\n{}\n```", yaml_content);
        let md: Markdown = md_content.into();
        let md_html = md.as_html(HtmlOptions::default()).unwrap();

        // Both should contain language-yaml
        assert!(block_html.contains("language-yaml"), "YamlBlock should produce language-yaml");
        assert!(md_html.contains("language-yaml"), "Markdown fence should produce language-yaml");

        // Both should contain the YAML content (individual words, since syntax highlighting may split them across spans)
        assert!(block_html.contains("foo"), "YamlBlock should contain YAML content");
        assert!(md_html.contains("foo"), "Markdown fence should contain YAML content");
        assert!(block_html.contains("bar"), "YamlBlock should contain YAML content");
        assert!(md_html.contains("bar"), "Markdown fence should contain YAML content");
    }

    #[test]
    fn test_terminal_render_with_light_color_mode() {
        let block = YamlBlock::new("key: value").unwrap();
        let term = Terminal::new();
        let output = Renderable::render(&block, &term);

        // Should produce ANSI output regardless of detected color mode
        assert!(output.contains("\x1b["), "Terminal output should contain ANSI codes");

        // Content should be present
        let plain = crate::testing::strip_ansi_codes(&output);
        assert!(plain.contains("key: value"));
    }

    #[test]
    fn test_terminal_render_with_dark_color_mode() {
        let block = YamlBlock::new("key: value").unwrap();
        let term = Terminal::new();
        let output = Renderable::render(&block, &term);

        // Should produce ANSI output
        assert!(output.contains("\x1b["), "Terminal output should contain ANSI codes");

        // Content should be present
        let plain = crate::testing::strip_ansi_codes(&output);
        assert!(plain.contains("key: value"));
    }

    #[test]
    fn test_browser_render_structure() {
        let block = YamlBlock::new("nested:\n  key: value").unwrap();
        let html = BrowserRenderable::render_to_browser(&block);

        // Should contain pre and code tags
        assert!(html.contains("<pre>"), "Browser output should contain <pre> tag");
        assert!(html.contains("<code"), "Browser output should contain <code> tag");
        assert!(html.contains("</code>"), "Browser output should contain </code> tag");
        assert!(html.contains("</pre>"), "Browser output should contain </pre> tag");

        // Should contain the YAML content (may be inside syntax-highlighting spans)
        assert!(html.contains("nested"), "HTML should contain 'nested'. Full HTML: {}", html);
        assert!(html.contains("key"), "HTML should contain 'key'. Full HTML: {}", html);
        assert!(html.contains("value"), "HTML should contain 'value'. Full HTML: {}", html);
    }

    #[test]
    fn test_yaml_block_render_is_block_level() {
        let block = YamlBlock::new("foo: 1").unwrap();
        assert!(Renderable::is_block_level(&block), "YamlBlock should be block-level");
    }

    #[test]
    fn test_terminal_render_empty_yaml() {
        let block = YamlBlock::new("").unwrap();
        let term = Terminal::new();
        let output = Renderable::render(&block, &term);

        // Should still produce valid output (padding rows at minimum)
        assert!(output.contains("\x1b["), "Empty YAML should still produce ANSI output");
    }

    #[test]
    fn test_browser_render_empty_yaml() {
        let block = YamlBlock::new("").unwrap();
        let html = BrowserRenderable::render_to_browser(&block);

        // Should contain the code block structure even for empty YAML
        assert!(html.contains("language-yaml"), "Empty YAML should still have language class");
        assert!(html.contains("<pre>"), "Empty YAML should still have <pre> tag");
        assert!(html.contains("<code"), "Empty YAML should still have <code> tag");
    }
}
