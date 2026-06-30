//! Resolved source context for an error that originates in a file.
//!
//! [`SourceContext`] provides the file path, content, and frontmatter range
//! needed to render rich, source-aware error diagnostics.

use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::Arc;

use crate::components::prose::Prose;

/// A dotted, indentation-aware path to a YAML mapping key within frontmatter.
///
/// Used by [`SourceContext::focused_yaml_excerpt`] to identify which keys an
/// error involves, so the excerpt can show only those keys plus their
/// structural ancestors (e.g. a `$schema:` parent) instead of the whole
/// frontmatter block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlKeyPath {
    segments: Vec<String>,
}

impl YamlKeyPath {
    /// Build a key path from a dotted string such as `"$schema.spec"`.
    ///
    /// Empty segments (from leading/trailing/double dots) are dropped.
    pub fn dotted(path: impl AsRef<str>) -> Self {
        let segments = path
            .as_ref()
            .split('.')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        Self { segments }
    }

    /// Build a key path from explicit, root-first segments.
    pub fn new(segments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            segments: segments.into_iter().map(Into::into).collect(),
        }
    }

    /// The path segments, root-first.
    pub fn segments(&self) -> &[String] {
        &self.segments
    }
}

impl From<&str> for YamlKeyPath {
    fn from(s: &str) -> Self {
        Self::dotted(s)
    }
}

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

    /// Render a focused, structure-aware excerpt of the frontmatter that shows
    /// only the lines for `keys` plus the structural ancestors that give them
    /// context (e.g. a `$schema:` parent), with elision markers (`⋮`) between
    /// non-adjacent regions.
    ///
    /// The involved keys come from the typed error — the receiving
    /// interpolation key plus any frontmatter keys the failing expression
    /// referenced — so the report shows exactly the shape the user must fix
    /// rather than no YAML or the whole block.
    ///
    /// ## Returns
    ///
    /// A fenced `yaml` [`Prose`] block whose 1-based gutter numbers match the
    /// source file (frontmatter opens on line 1).
    ///
    /// ## Notes
    ///
    /// Falls back to a whole-frontmatter numbered excerpt when there is no
    /// frontmatter, `keys` is empty, none of the requested paths resolve, or
    /// the block uses YAML features that prevent safe non-contiguous slicing
    /// (anchors, aliases, merge keys). This keeps the method total — it never
    /// guesses a partial slice.
    pub fn focused_yaml_excerpt(&self, keys: &[YamlKeyPath]) -> Prose {
        self.try_focused_yaml_excerpt(keys)
            .unwrap_or_else(|| self.whole_frontmatter_excerpt())
    }

    /// Attempt the focused slice; `None` triggers the whole-block fallback.
    fn try_focused_yaml_excerpt(&self, keys: &[YamlKeyPath]) -> Option<Prose> {
        if keys.is_empty() {
            return None;
        }
        let range = self.frontmatter.as_ref()?;
        let block = &self.content[range.clone()];
        if has_unsafe_yaml_features(block) {
            return None;
        }
        let lines: Vec<&str> = block.lines().collect();

        // Union each key's ancestor header lines with its own value range.
        let mut shown: BTreeSet<usize> = BTreeSet::new();
        let mut any = false;
        for path in keys {
            if let Some(region) = locate_key_region(&lines, path) {
                any = true;
                shown.extend(region.ancestors);
                shown.extend(region.target);
            }
        }
        if !any {
            return None;
        }
        Some(render_focused(&lines, &shown))
    }

    /// Render the entire frontmatter block as a gutter-numbered `yaml` excerpt.
    ///
    /// Used as the fallback for [`focused_yaml_excerpt`](Self::focused_yaml_excerpt)
    /// and returns an empty [`Prose`] when no frontmatter is present.
    fn whole_frontmatter_excerpt(&self) -> Prose {
        let Some(range) = self.frontmatter.as_ref() else {
            return Prose::new(String::new());
        };
        let block = &self.content[range.clone()];
        let lines: Vec<&str> = block.lines().collect();
        let width = lines.len().to_string().len().max(1);
        let mut buf = String::from("```yaml\n");
        for (idx, line) in lines.iter().enumerate() {
            writeln!(buf, "  {n:>width$} │ {line}", n = idx + 1, width = width).unwrap();
        }
        buf.push_str("```");
        Prose::new(buf)
    }
}

/// The lines (0-based into the frontmatter block) involved in showing one key
/// path: the ancestor header lines plus the target key's own value range.
struct KeyRegion {
    /// Ancestor header line indices, root-first, excluding the target key.
    ancestors: Vec<usize>,
    /// Inclusive line-index range of the target key and its multi-line value.
    target: std::ops::RangeInclusive<usize>,
}

/// Locate the ancestor header lines and value range for `path` within `lines`.
///
/// Walks each dotted segment by indentation (the same scheme as Claudine's
/// `locate_property_line`, reproduced here because `biscuit-terminal` is below
/// Claudine in the dependency graph; the two converge in a later phase), then
/// extends the final segment's range over any more-deeply-indented value lines.
/// Returns `None` when any segment cannot be found.
fn locate_key_region(lines: &[&str], path: &YamlKeyPath) -> Option<KeyRegion> {
    let segments = path.segments();
    if segments.is_empty() {
        return None;
    }

    let mut ancestors: Vec<usize> = Vec::new();
    let mut target_line: Option<usize> = None;
    // Top-level keys sit at indent >= 0; the synthetic root indent is below 0.
    let mut parent_indent: isize = -1;
    let mut lo = 0usize;

    for (seg_idx, segment) in segments.iter().enumerate() {
        let mut level_indent: Option<isize> = None;
        let mut matched: Option<usize> = None;

        for (idx, line) in lines.iter().enumerate().skip(lo) {
            if is_blank_or_comment(line) {
                continue;
            }
            let indent = indent_of(line);
            // A line at or shallower than the parent ends the parent's scope.
            if indent <= parent_indent {
                break;
            }
            // The first meaningful line fixes the indentation for this level.
            let level = *level_indent.get_or_insert(indent);
            if indent != level {
                continue;
            }
            if key_name(line) == Some(segment.as_str()) {
                matched = Some(idx);
                break;
            }
        }

        let idx = matched?;
        if seg_idx + 1 == segments.len() {
            target_line = Some(idx);
        } else {
            ancestors.push(idx);
        }
        parent_indent = indent_of(lines[idx]);
        lo = idx + 1;
    }

    let target_start = target_line?;
    let target_indent = indent_of(lines[target_start]);
    // Extend over the value: subsequent non-blank lines more indented than the
    // key belong to its value (a nested mapping or multi-line scalar).
    let mut target_end = target_start;
    for (idx, line) in lines.iter().enumerate().skip(target_start + 1) {
        if is_blank_or_comment(line) {
            break;
        }
        if indent_of(line) > target_indent {
            target_end = idx;
        } else {
            break;
        }
    }

    Some(KeyRegion {
        ancestors,
        target: target_start..=target_end,
    })
}

/// Render the `shown` line indices as a fenced `yaml` block with gutter line
/// numbers, inserting an elision marker between non-adjacent regions.
fn render_focused(lines: &[&str], shown: &BTreeSet<usize>) -> Prose {
    let max_line = shown.iter().max().map(|&i| i + 1).unwrap_or(1);
    let width = max_line.to_string().len().max(1);

    let mut buf = String::from("```yaml\n");
    let mut prev: Option<usize> = None;
    for &idx in shown {
        if let Some(p) = prev
            && idx > p + 1
        {
            // Gap between regions: align the marker under the gutter separator.
            writeln!(buf, "  {blank:>width$} ⋮", blank = "", width = width).unwrap();
        }
        writeln!(buf, "  {n:>width$} │ {line}", n = idx + 1, line = lines[idx], width = width)
            .unwrap();
        prev = Some(idx);
    }
    buf.push_str("```");
    Prose::new(buf)
}

/// Conservatively detect YAML features that make non-contiguous slicing unsafe,
/// signalling the caller to fall back to a whole-block excerpt.
fn has_unsafe_yaml_features(block: &str) -> bool {
    block.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.contains("<<:")
            || trimmed.contains(": &")
            || trimmed.contains(": *")
            || trimmed.starts_with("- &")
            || trimmed.starts_with("- *")
    })
}

/// The byte-width of a line's leading spaces, signed for comparison against the
/// synthetic `-1` root indent.
fn indent_of(line: &str) -> isize {
    (line.len() - line.trim_start().len()) as isize
}

fn is_blank_or_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.is_empty() || trimmed.starts_with('#')
}

/// Extract the mapping key from a `key: value` line, or `None` when the line is
/// not a plain mapping entry (a list item or bare scalar).
fn key_name(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('-') {
        return None;
    }
    let (key, _) = trimmed.split_once(':')?;
    Some(key.trim().trim_matches(|c| c == '"' || c == '\''))
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

    // Frontmatter matching the spec's reference example: `spec` and `iteration`
    // are nested under a `$schema:` structural parent, surrounded by unrelated
    // top-level keys that the focused excerpt must exclude.
    const SCHEMA_DOC: &str = "---\nagent: \"codex\"\n$schema:\n    spec: \"features/spec.md\"\n    iteration: \"frontmatter(spec, 'n')\"\nyolo: false\nphases: 8\n---\n# Body\n";

    fn schema_ctx() -> SourceContext {
        SourceContext::new(
            PathBuf::from("/p.md"),
            PathBuf::from("p.md"),
            SCHEMA_DOC,
        )
    }

    #[test]
    fn yaml_key_path_dotted_splits_and_trims() {
        let path = YamlKeyPath::dotted("$schema.spec");
        assert_eq!(path.segments(), ["$schema", "spec"]);
        // Leading/trailing/double dots drop empty segments.
        assert_eq!(YamlKeyPath::dotted(".a..b.").segments(), ["a", "b"]);
    }

    #[test]
    fn focused_excerpt_includes_schema_parent() {
        // The P5 validation checkpoint: focusing on the two nested keys must
        // surface their `$schema:` parent plus both children, and nothing else.
        let ctx = schema_ctx();
        let prose = ctx.focused_yaml_excerpt(&[
            YamlKeyPath::dotted("$schema.spec"),
            YamlKeyPath::dotted("$schema.iteration"),
        ]);
        let text = prose.content();
        assert!(text.contains("$schema:"), "missing parent: {text}");
        assert!(text.contains("spec:"), "missing spec: {text}");
        assert!(text.contains("iteration:"), "missing iteration: {text}");
        // Unrelated siblings must not appear.
        assert!(!text.contains("agent:"), "leaked agent: {text}");
        assert!(!text.contains("yolo:"), "leaked yolo: {text}");
        assert!(!text.contains("phases:"), "leaked phases: {text}");
    }

    #[test]
    fn focused_excerpt_line_numbers_match_file() {
        // `$schema:` is file line 3, `spec:` line 4, `iteration:` line 5.
        let ctx = schema_ctx();
        let prose = ctx.focused_yaml_excerpt(&[YamlKeyPath::dotted("$schema.spec")]);
        let text = prose.content();
        assert!(text.contains("3 │ $schema:"), "got: {text}");
        assert!(text.contains("4 │     spec:"), "got: {text}");
    }

    #[test]
    fn focused_excerpt_elides_between_nonadjacent_regions() {
        // Two non-adjacent top-level keys: an elision marker separates them and
        // the intervening unrelated keys are excluded.
        let ctx = schema_ctx();
        let prose = ctx.focused_yaml_excerpt(&[
            YamlKeyPath::dotted("agent"),
            YamlKeyPath::dotted("phases"),
        ]);
        let text = prose.content();
        assert!(text.contains("agent:"), "got: {text}");
        assert!(text.contains("phases:"), "got: {text}");
        assert!(text.contains('⋮'), "missing elision marker: {text}");
        assert!(!text.contains("yolo:"), "leaked intervening key: {text}");
    }

    #[test]
    fn focused_excerpt_falls_back_to_whole_block_on_missing_key() {
        // No requested key resolves → whole-block fallback shows everything.
        let ctx = schema_ctx();
        let prose = ctx.focused_yaml_excerpt(&[YamlKeyPath::dotted("nonexistent")]);
        let text = prose.content();
        assert!(text.contains("agent:"), "fallback should show all: {text}");
        assert!(text.contains("yolo:"), "fallback should show all: {text}");
        assert!(!text.contains('⋮'), "fallback is contiguous: {text}");
    }

    #[test]
    fn focused_excerpt_falls_back_on_empty_keys() {
        let ctx = schema_ctx();
        let prose = ctx.focused_yaml_excerpt(&[]);
        assert!(prose.content().contains("agent:"));
    }

    #[test]
    fn focused_excerpt_falls_back_on_anchors() {
        // Anchors/aliases are unsafe to slice non-contiguously → whole block.
        let content = "---\nbase: &b\n    a: 1\nother: *b\ntail: 2\n---\nbody\n";
        let ctx = SourceContext::new(PathBuf::from("/a.md"), PathBuf::from("a.md"), content);
        let prose = ctx.focused_yaml_excerpt(&[YamlKeyPath::dotted("tail")]);
        let text = prose.content();
        assert!(text.contains("base:"), "anchor fallback shows all: {text}");
        assert!(text.contains("other:"), "anchor fallback shows all: {text}");
        assert!(!text.contains('⋮'), "fallback is contiguous: {text}");
    }

    #[test]
    fn focused_excerpt_empty_without_frontmatter() {
        let ctx = SourceContext::new(
            PathBuf::from("/b.md"),
            PathBuf::from("b.md"),
            "# no frontmatter\n",
        );
        assert_eq!(ctx.focused_yaml_excerpt(&[YamlKeyPath::dotted("x")]).content(), "");
    }

    #[test]
    fn focused_excerpt_includes_multiline_value() {
        // A nested key block is captured in full as the target's value range.
        let ctx = schema_ctx();
        let prose = ctx.focused_yaml_excerpt(&[YamlKeyPath::dotted("$schema")]);
        let text = prose.content();
        assert!(text.contains("$schema:"), "got: {text}");
        assert!(text.contains("spec:"), "nested child missing: {text}");
        assert!(text.contains("iteration:"), "nested child missing: {text}");
        assert!(!text.contains("yolo:"), "leaked sibling: {text}");
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
