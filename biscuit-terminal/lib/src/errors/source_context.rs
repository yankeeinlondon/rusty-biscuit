//! Resolved source context for an error that originates in a file.
//!
//! [`SourceContext`] provides the file path, content, and frontmatter range
//! needed to render rich, source-aware error diagnostics.

use std::fmt::Write;
use std::path::PathBuf;
use std::sync::Arc;

use crate::components::prose::Prose;

/// Resolved source context for an error that originates in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceContext {
    /// Absolute path used for OSC 8 hyperlinks.
    pub absolute: PathBuf,
    /// Display path (typically relative to repo or cwd) for the visible label.
    pub display: PathBuf,
    /// Full source content. Shared via `Arc` to keep error variants cheap to clone.
    pub content: Arc<str>,
    /// Byte range of frontmatter in `content`, if present.
    pub frontmatter: Option<std::ops::Range<usize>>,
}

impl SourceContext {
    /// Create a new [`SourceContext`] with automatic frontmatter detection.
    pub fn new(absolute: PathBuf, display: PathBuf, content: impl Into<Arc<str>>) -> Self {
        let content: Arc<str> = content.into();
        let frontmatter = detect_frontmatter_range(&content);
        Self {
            absolute,
            display,
            content,
            frontmatter,
        }
    }

    /// Create a new [`SourceContext`] with an explicit frontmatter byte range.
    pub fn with_frontmatter(
        absolute: PathBuf,
        display: PathBuf,
        content: impl Into<Arc<str>>,
        frontmatter: Option<std::ops::Range<usize>>,
    ) -> Self {
        Self {
            absolute,
            display,
            content: content.into(),
            frontmatter,
        }
    }

    /// Render a `<blue><a href=ABSOLUTE>RELATIVE</a></blue>` Prose segment
    /// for use in error headers.
    ///
    /// User-controlled path segments are escaped so they render literally
    /// even when they contain Prose-significant characters (`<`, `>`, `"`,
    /// `{`, etc.).
    pub fn linked_path_prose(&self) -> Prose {
        let abs = self.absolute.to_string_lossy();
        let display = self.display.to_string_lossy();
        let abs_attr = Prose::quoted_attr(&abs);
        let display_escaped = Prose::escape_text(&display);
        Prose::new(format!(
            "<blue><a href={}>{}</a></blue>",
            abs_attr, display_escaped
        ))
    }

    /// Render the frontmatter as a fenced `yaml` code block, or `None` if absent.
    pub fn frontmatter_prose(&self) -> Option<Prose> {
        let range = self.frontmatter.as_ref()?;
        let fm_text = &self.content[range.clone()];
        Some(Prose::new(format!("```yaml\n{}\n```", fm_text)))
    }

    /// Render an excerpt centered on `line` (1-based), with `context` lines
    /// above and below, as a fenced code block tagged with `lang`.
    ///
    /// The offending line is marked with a leading `>` gutter.
    pub fn excerpt_prose(&self, line: usize, context: usize, lang: &str) -> Prose {
        let lines: Vec<&str> = self.content.lines().collect();
        let total = lines.len();
        let start = line.saturating_sub(context + 1).min(total);
        let end = (line + context).min(total);
        let width = end.to_string().len();

        let mut buf = String::from("```");
        buf.push_str(lang);
        buf.push('\n');
        for (idx, l) in lines[start..end].iter().enumerate() {
            let n = start + idx + 1;
            let gutter = if n == line { ">" } else { " " };
            writeln!(buf, "{gutter} {n:>width$} │ {l}", width = width).unwrap();
        }
        buf.push_str("```");
        Prose::new(buf)
    }
}

/// Detect the byte range of YAML frontmatter delimited by `---` lines.
///
/// Handles both LF (`\n`) and CRLF (`\r\n`) line endings, and correctly
/// bounds the returned range to `content.len()` when the file ends
/// immediately after the closing `---` delimiter.
fn detect_frontmatter_range(content: &str) -> Option<std::ops::Range<usize>> {
    // Use `split_inclusive('\n')` so each yielded segment contains its
    // trailing newline (if any). This naturally accounts for CRLF because
    // the `\r` remains part of the segment, and the segment length reflects
    // the actual byte count consumed.
    let mut lines = content.split_inclusive('\n').enumerate().peekable();

    // First line must be exactly `---` (ignoring line endings)
    let (first_idx, first_line) = lines.next()?;
    let first_trimmed = first_line
        .trim_end_matches('\n')
        .trim_end_matches('\r')
        .trim();
    if first_trimmed != "---" {
        return None;
    }

    // Find closing `---`
    let mut closing_idx = None;
    for (idx, line) in lines {
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r').trim();
        if trimmed == "---" {
            closing_idx = Some(idx);
            break;
        }
    }
    let closing_idx = closing_idx?;

    // Calculate byte positions from the same split iterator
    let mut pos = 0usize;
    let mut start_byte = None;
    let mut end_byte = None;

    for (idx, line) in content.split_inclusive('\n').enumerate() {
        let line_start = pos;
        let line_end = pos + line.len();

        if idx == first_idx {
            start_byte = Some(line_start);
        }
        if idx == closing_idx {
            end_byte = Some(line_end);
            break;
        }

        pos = line_end;
    }

    Some(start_byte?..end_byte?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_path_prose_includes_href() {
        let ctx = SourceContext::new(
            PathBuf::from("/abs/path.md"),
            PathBuf::from("path.md"),
            "content",
        );
        let prose = ctx.linked_path_prose();
        assert!(prose.content().contains("href=\"/abs/path.md\""));
        assert!(prose.content().contains("<a href="));
        assert!(prose.content().contains(">path.md</a>"));
    }

    #[test]
    fn frontmatter_detection_basic() {
        let content = "---\ntitle: Test\n---\n# Body\n";
        let ctx = SourceContext::new(PathBuf::from("/test.md"), PathBuf::from("test.md"), content);
        assert!(ctx.frontmatter.is_some());
        let range = ctx.frontmatter.unwrap();
        assert_eq!(&content[range], "---\ntitle: Test\n---\n");
    }

    #[test]
    fn frontmatter_detection_none_when_missing() {
        let ctx = SourceContext::new(
            PathBuf::from("/test.md"),
            PathBuf::from("test.md"),
            "# No frontmatter\n",
        );
        assert!(ctx.frontmatter.is_none());
    }

    #[test]
    fn frontmatter_prose_renders_yaml_block() {
        let content = "---\ntitle: Test\n---\n# Body\n";
        let ctx = SourceContext::new(PathBuf::from("/test.md"), PathBuf::from("test.md"), content);
        let prose = ctx.frontmatter_prose().unwrap();
        assert!(prose.content().starts_with("```yaml"));
        assert!(prose.content().contains("title: Test"));
    }

    #[test]
    fn excerpt_prose_gutters_offending_line() {
        let content = "line 1\nline 2\nline 3\nline 4\nline 5\n";
        let ctx = SourceContext::new(PathBuf::from("/test.md"), PathBuf::from("test.md"), content);
        let prose = ctx.excerpt_prose(3, 1, "md");
        let text = prose.content();
        assert!(text.contains("> 3 │ line 3"));
        assert!(text.contains("  2 │ line 2"));
        assert!(text.contains("  4 │ line 4"));
    }

    #[test]
    fn excerpt_prose_near_start() {
        let content = "line 1\nline 2\nline 3\n";
        let ctx = SourceContext::new(PathBuf::from("/test.md"), PathBuf::from("test.md"), content);
        let prose = ctx.excerpt_prose(1, 2, "md");
        let text = prose.content();
        assert!(text.contains("> 1 │ line 1"));
        assert!(text.contains("  2 │ line 2"));
        assert!(text.contains("  3 │ line 3"));
    }

    #[test]
    fn frontmatter_detection_ends_at_eof() {
        // File ends immediately after the closing `---` with no trailing newline.
        let content = "---\ntitle: Test\n---";
        let ctx = SourceContext::new(PathBuf::from("/test.md"), PathBuf::from("test.md"), content);
        assert!(ctx.frontmatter.is_some());
        let range = ctx.frontmatter.as_ref().unwrap().clone();
        assert_eq!(&content[range], content);
        // frontmatter_prose must not panic
        let prose = ctx.frontmatter_prose().unwrap();
        assert!(prose.content().starts_with("```yaml"));
        assert!(prose.content().contains("title: Test"));
    }

    #[test]
    fn frontmatter_detection_crlf() {
        let content = "---\r\ntitle: Test\r\n---\r\n# Body\r\n";
        let ctx = SourceContext::new(PathBuf::from("/test.md"), PathBuf::from("test.md"), content);
        assert!(ctx.frontmatter.is_some());
        let range = ctx.frontmatter.unwrap();
        assert_eq!(&content[range], "---\r\ntitle: Test\r\n---\r\n");
    }

    #[test]
    fn linked_path_prose_escapes_special_chars() {
        let ctx = SourceContext::new(
            PathBuf::from("/abs/path\u{003c}weird\u{003e}"),
            PathBuf::from("path\u{003c}weird\u{003e}.md"),
            "content",
        );
        let prose = ctx.linked_path_prose();
        let text = prose.content();
        // `>` in path must be escaped so it doesn't close the <a> tag early
        assert!(
            text.contains("path\\<weird\\>.md"),
            "expected escaped angle brackets in display text, got: {text}"
        );
        // `>` in href must be escaped too
        assert!(
            text.contains("/abs/path\\<weird\\>"),
            "expected escaped angle brackets in href, got: {text}"
        );
        // The tag structure must remain intact
        assert!(text.contains("<a href="), "tag structure broken: {text}");
    }
}
