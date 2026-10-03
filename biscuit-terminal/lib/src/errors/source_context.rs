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
/// Used by [`SourceContext::focused_yaml_regions`] and
/// [`SourceContext::focused_yaml_excerpt`] to identify which keys an error
/// involves, so the excerpt can show only those keys plus their structural
/// ancestors (e.g. a `$schema:` parent) instead of the whole frontmatter block.
///
/// When a segment's parent value is a sequence of mappings (`- key: …` items),
/// the segment matches the key in the first item that has it. Use
/// [`in_every_arm`](Self::in_every_arm) to match it in every item instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlKeyPath {
    segments: Vec<String>,
    /// Index of the segment matched in every sequence item of its parent.
    every_arm: Option<usize>,
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
        Self {
            segments,
            every_arm: None,
        }
    }

    /// Build a key path from explicit, root-first segments.
    pub fn new(segments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            segments: segments.into_iter().map(Into::into).collect(),
            every_arm: None,
        }
    }

    /// Build a path to `key` inside every sequence item (union arm) of the
    /// dotted `parent`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::errors::YamlKeyPath;
    ///
    /// // Matches `doc:` in both `- spec: …` and `- design: …` arms of
    /// // a `$schema:` sequence.
    /// let path = YamlKeyPath::in_every_arm("$schema", "doc");
    /// assert_eq!(path.segments(), ["$schema", "doc"]);
    /// ```
    pub fn in_every_arm(parent: impl AsRef<str>, key: impl AsRef<str>) -> Self {
        let parent = Self::dotted(parent);
        let every_arm = Some(parent.segments.len());
        let mut segments = parent.segments;
        segments.extend(Self::dotted(key).segments);
        Self {
            segments,
            every_arm,
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

    /// Select the frontmatter lines that show `keys`: each located key line,
    /// its value, `context` lines either side, and its ancestor header lines.
    ///
    /// Ancestors (a `$schema:` parent, or the `- …` line opening a union arm)
    /// are always included, even outside the window. Overlapping or adjacent
    /// selections merge, so each returned region is one contiguous run and
    /// consecutive regions are separated by at least one unselected line.
    ///
    /// ## Returns
    ///
    /// Regions in ascending line order, with 1-based line numbers absolute
    /// within [`content`](Self::content). Windows are clamped to the
    /// frontmatter block, whose `---` delimiter lines they may include.
    ///
    /// `None` when there is no frontmatter or no requested key resolves.
    /// Unresolved keys are otherwise ignored. There is no whole-block
    /// fallback. It is also `None` when the block uses anchors, aliases, or
    /// merge keys (`&`, `*`, `<<`), which make a partial slice misleading,
    /// unless the selection already holds every line between the delimiters.
    pub fn focused_yaml_regions(
        &self,
        keys: &[YamlKeyPath],
        context: usize,
    ) -> Option<Vec<FocusedRegion>> {
        self.select_regions(context, |lines, _| {
            keys.iter()
                .flat_map(|path| locate_key_regions(lines, path))
                .collect()
        })
    }

    /// Select the frontmatter lines that show the given 1-based `lines`, each
    /// with `context` lines either side and its ancestor header lines.
    ///
    /// This is the counterpart of
    /// [`focused_yaml_regions`](Self::focused_yaml_regions) for a caller that
    /// has already located its target lines (for example, from a path syntax
    /// [`YamlKeyPath`] does not model). Ancestors are the preceding lines that
    /// open an enclosing mapping or sequence item, found by indentation.
    ///
    /// ## Returns
    ///
    /// The same shape as [`focused_yaml_regions`](Self::focused_yaml_regions),
    /// with each requested line highlighted. Lines outside the frontmatter
    /// block are ignored; `None` when none remains, and on the same unsafe-YAML
    /// rule.
    pub fn focused_line_regions(
        &self,
        lines: &[usize],
        context: usize,
    ) -> Option<Vec<FocusedRegion>> {
        self.select_regions(context, |block_lines, first_line| {
            let shapes: Vec<LineShape> = block_lines.iter().map(|l| LineShape::parse(l)).collect();
            lines
                .iter()
                .filter_map(|line| line.checked_sub(first_line))
                .filter(|&idx| idx < block_lines.len())
                .map(|idx| KeyRegion {
                    ancestors: indentation_ancestors(&shapes, idx),
                    target: idx..=idx,
                })
                .collect()
        })
    }

    /// Window, merge, and number the regions `locate` finds in the
    /// frontmatter block.
    ///
    /// `locate` receives the block's lines and the absolute line number of
    /// its first line, and returns block-relative (0-based) matches.
    fn select_regions(
        &self,
        context: usize,
        locate: impl FnOnce(&[&str], usize) -> Vec<KeyRegion>,
    ) -> Option<Vec<FocusedRegion>> {
        let range = self.frontmatter.as_ref()?;
        let block = &self.content[range.clone()];
        let lines: Vec<&str> = block.lines().collect();
        let last = lines.len().checked_sub(1)?;
        // Block index 0 is the source line after `range.start`'s newlines.
        let first_line = self.content[..range.start].matches('\n').count() + 1;

        let mut shown: BTreeSet<usize> = BTreeSet::new();
        let mut highlighted: BTreeSet<usize> = BTreeSet::new();
        for found in locate(&lines, first_line) {
            let start = found.target.start().saturating_sub(context);
            let end = found.target.end().saturating_add(context).min(last);
            shown.extend(start..=end);
            shown.extend(found.ancestors);
            highlighted.insert(*found.target.start());
        }
        if highlighted.is_empty() {
            return None;
        }
        // A slice of an anchored block can hide what an alias expands to; a
        // selection holding every line between the delimiters cannot.
        let covers_block = (1..last).all(|idx| shown.contains(&idx));
        if !covers_block && has_unsafe_yaml_features(block) {
            return None;
        }

        let mut regions: Vec<FocusedRegion> = Vec::new();
        for idx in shown {
            let line = first_line + idx;
            match regions.last_mut() {
                Some(region) if region.end_line + 1 == line => region.end_line = line,
                _ => regions.push(FocusedRegion {
                    start_line: line,
                    end_line: line,
                    highlighted: Vec::new(),
                }),
            }
            if highlighted.contains(&idx)
                && let Some(region) = regions.last_mut()
            {
                region.highlighted.push(line);
            }
        }
        Some(regions)
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
    /// source file.
    ///
    /// ## Notes
    ///
    /// The lines are those of [`focused_yaml_regions`](Self::focused_yaml_regions)
    /// with no surrounding context. Where that returns `None`, this falls back
    /// to a whole-frontmatter numbered excerpt, which keeps the method total —
    /// it never guesses a partial slice.
    pub fn focused_yaml_excerpt(&self, keys: &[YamlKeyPath]) -> Prose {
        match self.focused_yaml_regions(keys, 0) {
            Some(regions) => self.render_regions(&regions),
            None => self.whole_frontmatter_excerpt(),
        }
    }

    /// Render `regions` as one fenced `yaml` block with absolute gutter line
    /// numbers and an elision marker between regions.
    fn render_regions(&self, regions: &[FocusedRegion]) -> Prose {
        let lines: Vec<&str> = self.content.lines().collect();
        let max_line = regions.last().map_or(1, |r| r.end_line);
        let width = max_line.to_string().len().max(1);

        let mut buf = String::from("```yaml\n");
        for (idx, region) in regions.iter().enumerate() {
            if idx > 0 {
                writeln!(buf, "  {blank:>width$} ⋮", blank = "", width = width).unwrap();
            }
            for n in region.start_line..=region.end_line {
                let line = lines.get(n - 1).copied().unwrap_or_default();
                writeln!(buf, "  {n:>width$} │ {line}", width = width).unwrap();
            }
        }
        buf.push_str("```");
        Prose::new(buf)
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

/// One contiguous run of frontmatter lines selected by
/// [`SourceContext::focused_yaml_regions`].
///
/// Line numbers are 1-based and absolute within [`SourceContext::content`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusedRegion {
    /// First line of the run.
    pub start_line: usize,
    /// Last line of the run, inclusive.
    pub end_line: usize,
    /// Lines within the run that name a requested key, ascending.
    pub highlighted: Vec<usize>,
}

/// The lines (0-based into the frontmatter block) involved in showing one
/// match of a key path: the ancestor header lines plus the target key's own
/// value range.
struct KeyRegion {
    /// Ancestor header line indices, root-first, excluding the target key.
    ancestors: Vec<usize>,
    /// Inclusive line-index range of the target key and its multi-line value.
    target: std::ops::RangeInclusive<usize>,
}

/// The mapping-key shape of one frontmatter line.
struct LineShape<'a> {
    indent: isize,
    /// Column of the key: after the `- ` marker on a sequence-item line,
    /// otherwise `indent`.
    key_col: isize,
    key: Option<&'a str>,
    /// The line opens a sequence item (`- …`).
    item: bool,
    /// Neither blank nor a comment.
    meaningful: bool,
}

impl<'a> LineShape<'a> {
    fn parse(line: &'a str) -> Self {
        let indent = indent_of(line);
        let trimmed = line.trim_start();
        let meaningful = !is_blank_or_comment(line);
        let item = trimmed == "-" || trimmed.starts_with("- ");
        if !item {
            return Self {
                indent,
                key_col: indent,
                key: key_name(line),
                item,
                meaningful,
            };
        }
        let rest = &trimmed[1..];
        let body = rest.trim_start();
        // An empty marker line's mapping starts on the next line, two columns in.
        let key_col = if body.is_empty() {
            indent + 2
        } else {
            indent + 1 + (rest.len() - body.len()) as isize
        };
        Self {
            indent,
            key_col,
            key: key_name(body),
            item,
            meaningful,
        }
    }

    /// Whether this line belongs to the value of a key whose key column is
    /// `key_col`: deeper lines, or sequence items at the key's own column.
    fn in_value_of(&self, key_col: isize) -> bool {
        self.indent > key_col || (self.item && self.indent == key_col)
    }
}

/// Locate every match of `path` within `lines`.
///
/// Walks each segment by indentation (the same scheme as Claudine's
/// `locate_property_line`, reproduced here because `biscuit-terminal` is below
/// Claudine in the dependency graph). Each sequence item is its own scope; a
/// segment inside a sequence matches in the first item that has it, or in
/// every item for the path's every-arm segment. Empty when nothing resolves.
fn locate_key_regions(lines: &[&str], path: &YamlKeyPath) -> Vec<KeyRegion> {
    let mut found = Vec::new();
    if !path.segments().is_empty() {
        let shapes: Vec<LineShape> = lines.iter().map(|l| LineShape::parse(l)).collect();
        let mut walk = KeyWalk {
            shapes: &shapes,
            path,
            ancestors: Vec::new(),
            found: &mut found,
        };
        walk.scope(0..lines.len(), 0);
    }
    found
}

struct KeyWalk<'s, 'a> {
    shapes: &'s [LineShape<'a>],
    path: &'s YamlKeyPath,
    ancestors: Vec<usize>,
    found: &'s mut Vec<KeyRegion>,
}

impl KeyWalk<'_, '_> {
    /// Match segment `seg` among the lines of `lines`, which hold either a
    /// mapping or a sequence of items.
    fn scope(&mut self, lines: std::ops::Range<usize>, seg: usize) {
        let Some(first) = lines.clone().find(|&i| self.shapes[i].meaningful) else {
            return;
        };
        if !self.shapes[first].item {
            // The first meaningful line fixes the key column for this level.
            self.mapping(lines, self.shapes[first].key_col, seg);
            return;
        }

        let item_indent = self.shapes[first].indent;
        let markers: Vec<usize> = lines
            .clone()
            .filter(|&i| {
                let shape = &self.shapes[i];
                shape.meaningful && shape.item && shape.indent == item_indent
            })
            .collect();
        let every = self.path.every_arm == Some(seg);
        for (n, &marker) in markers.iter().enumerate() {
            let end = markers.get(n + 1).copied().unwrap_or(lines.end);
            let before = self.found.len();
            self.ancestors.push(marker);
            self.mapping(marker..end, self.shapes[marker].key_col, seg);
            self.ancestors.pop();
            if !every && self.found.len() > before {
                return;
            }
        }
    }

    /// Match segment `seg` among the keys at column `key_col` in `lines`.
    fn mapping(&mut self, lines: std::ops::Range<usize>, key_col: isize, seg: usize) {
        let segment = self.path.segments()[seg].as_str();
        let Some(idx) = lines.clone().find(|&i| {
            let shape = &self.shapes[i];
            shape.meaningful && shape.key_col == key_col && shape.key == Some(segment)
        }) else {
            return;
        };

        if seg + 1 == self.path.segments().len() {
            // The value stops at the first blank or comment line, or the first
            // line outside it.
            let end = (idx + 1..lines.end)
                .take_while(|&i| self.shapes[i].meaningful && self.shapes[i].in_value_of(key_col))
                .last()
                .unwrap_or(idx);
            self.found.push(KeyRegion {
                ancestors: self
                    .ancestors
                    .iter()
                    .copied()
                    .filter(|&a| a != idx)
                    .collect(),
                target: idx..=end,
            });
            return;
        }

        let child_end = (idx + 1..lines.end)
            .find(|&i| self.shapes[i].meaningful && !self.shapes[i].in_value_of(key_col))
            .unwrap_or(lines.end);
        self.ancestors.push(idx);
        self.scope(idx + 1..child_end, seg + 1);
        self.ancestors.pop();
    }
}

/// The header lines enclosing block line `idx`, root-first: each preceding
/// meaningful line that is less indented than the line below it in the chain,
/// or a key line that owns a sequence item at its own indentation. The opening
/// `---` delimiter is never an ancestor.
fn indentation_ancestors(shapes: &[LineShape], idx: usize) -> Vec<usize> {
    let mut indent = shapes[idx].indent;
    let mut item = shapes[idx].item;
    let mut ancestors = Vec::new();
    for candidate in (1..idx).rev() {
        let shape = &shapes[candidate];
        if !shape.meaningful {
            continue;
        }
        if shape.indent < indent || (item && !shape.item && shape.indent == indent) {
            ancestors.push(candidate);
            indent = shape.indent;
            item = shape.item;
        }
    }
    ancestors.reverse();
    ancestors
}

/// Conservatively detect YAML features that make non-contiguous slicing unsafe.
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

/// The byte-width of a line's leading spaces, signed so a key column can sit
/// below every real line.
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

    fn fm_ctx(content: &str) -> SourceContext {
        SourceContext::new(PathBuf::from("/f.md"), PathBuf::from("f.md"), content)
    }

    fn region(start_line: usize, end_line: usize, highlighted: &[usize]) -> FocusedRegion {
        FocusedRegion {
            start_line,
            end_line,
            highlighted: highlighted.to_vec(),
        }
    }

    // Two union arms whose `doc:` keys (lines 8 and 14) sit far apart.
    const UNION_DOC: &str = "---\n$schema:\n  - spec: file(required; match(**/*spec*.md); eager)\n    a: 1\n    b: 2\n    c: 3\n    d: 4\n    doc: file\n  - design: file(required)\n    e: 1\n    f: 2\n    g: 3\n    h: 4\n    doc: file\n---\nbody\n";

    #[test]
    fn focused_regions_mid_file_key_windows_context_and_keeps_ancestor() {
        let content = "---\nagent: codex\n$schema:\n  a: 1\n  b: 2\n  c: 3\n  d: 4\n  e: 5\n  f: 6\n  g: 7\n  h: 8\n  target: x\n  i: 9\n  j: 10\n  k: 11\n  l: 12\nyolo: false\n---\n";

        let regions = fm_ctx(content)
            .focused_yaml_regions(&[YamlKeyPath::dotted("$schema.target")], 3)
            .unwrap();

        assert_eq!(regions, vec![region(3, 3, &[]), region(9, 15, &[12])]);
    }

    #[test]
    fn focused_regions_key_in_both_union_arms_gives_two_regions() {
        let regions = fm_ctx(UNION_DOC)
            .focused_yaml_regions(&[YamlKeyPath::in_every_arm("$schema", "doc")], 1)
            .unwrap();

        assert_eq!(
            regions,
            vec![region(2, 3, &[]), region(7, 9, &[8]), region(13, 15, &[14])]
        );
    }

    #[test]
    fn focused_regions_plain_segment_matches_first_arm_only() {
        let regions = fm_ctx(UNION_DOC)
            .focused_yaml_regions(&[YamlKeyPath::dotted("$schema.doc")], 0)
            .unwrap();

        assert_eq!(regions, vec![region(2, 3, &[]), region(8, 8, &[8])]);
    }

    #[test]
    fn focused_regions_sequence_item_key_does_not_swallow_sibling() {
        // `spec` sits on the `- ` marker line at column 4, the same column as
        // its sibling `a:`, so its value ends on its own line.
        let regions = fm_ctx(UNION_DOC)
            .focused_yaml_regions(&[YamlKeyPath::in_every_arm("$schema", "spec")], 0)
            .unwrap();
        assert_eq!(regions, vec![region(2, 3, &[3])]);

        // A sequence at its parent key's own indentation.
        let flush = "---\n$schema:\n- spec: x\n  doc: y\n- design: z\n  doc: w\nyolo: true\n---\n";
        let regions = fm_ctx(flush)
            .focused_yaml_regions(&[YamlKeyPath::in_every_arm("$schema", "doc")], 0)
            .unwrap();
        assert_eq!(regions, vec![region(2, 6, &[4, 6])]);
    }

    #[test]
    fn focused_regions_every_arm_on_plain_mapping_matches_the_key() {
        let regions = schema_ctx()
            .focused_yaml_regions(&[YamlKeyPath::in_every_arm("$schema", "iteration")], 0)
            .unwrap();

        assert_eq!(regions, vec![region(3, 3, &[]), region(5, 5, &[5])]);
    }

    #[test]
    fn focused_regions_missing_key_gives_none() {
        let ctx = schema_ctx();

        assert_eq!(
            ctx.focused_yaml_regions(&[YamlKeyPath::dotted("nope")], 3),
            None
        );
        assert_eq!(
            ctx.focused_yaml_regions(&[YamlKeyPath::in_every_arm("$schema", "nope")], 3),
            None
        );
        assert_eq!(ctx.focused_yaml_regions(&[], 3), None);
    }

    #[test]
    fn focused_regions_adjacent_and_overlapping_windows_merge() {
        let content = "---\na: 1\nb: 2\nc: 3\nd: 4\ne: 5\nf: 6\ng: 7\n---\n";
        let ctx = fm_ctx(content);

        let adjacent = ctx
            .focused_yaml_regions(&[YamlKeyPath::dotted("a"), YamlKeyPath::dotted("b")], 0)
            .unwrap();
        assert_eq!(adjacent, vec![region(2, 3, &[2, 3])]);

        // Windows 3..=5 and 6..=8 touch, so they merge into one run.
        let touching = ctx
            .focused_yaml_regions(&[YamlKeyPath::dotted("c"), YamlKeyPath::dotted("f")], 1)
            .unwrap();
        assert_eq!(touching, vec![region(3, 8, &[4, 7])]);
    }

    #[test]
    fn focused_regions_clamp_to_block_start_and_end() {
        let content = "---\na: 1\nb: 2\nc: 3\nd: 4\n---\nbody\n";
        let ctx = fm_ctx(content);

        let first = ctx
            .focused_yaml_regions(&[YamlKeyPath::dotted("a")], 3)
            .unwrap();
        assert_eq!(first, vec![region(1, 5, &[2])]);

        let last = ctx
            .focused_yaml_regions(&[YamlKeyPath::dotted("d")], 3)
            .unwrap();
        assert_eq!(last, vec![region(2, 6, &[5])]);
    }

    #[test]
    fn focused_regions_unsafe_yaml_gives_none() {
        for content in [
            "---\nbase: &b\n  a: 1\ntail: 2\n---\n",
            "---\nbase: 1\nother: *b\ntail: 2\n---\n",
            "---\nbase:\n  <<: {a: 1}\ntail: 2\n---\n",
        ] {
            let regions = fm_ctx(content).focused_yaml_regions(&[YamlKeyPath::dotted("tail")], 1);
            assert_eq!(regions, None, "content: {content}");
        }
    }

    #[test]
    fn focused_regions_line_numbers_are_absolute_in_content() {
        let content = "intro\n---\na: 1\nb: 2\n---\n";
        let ctx = SourceContext::with_frontmatter(
            PathBuf::from("/f.md"),
            PathBuf::from("f.md"),
            content,
            Some(6..content.len()),
        );

        let regions = ctx
            .focused_yaml_regions(&[YamlKeyPath::dotted("b")], 0)
            .unwrap();

        assert_eq!(regions, vec![region(4, 4, &[4])]);
    }

    #[test]
    fn focused_regions_unsafe_yaml_fully_covered_renders_the_whole_block() {
        // Nothing is hidden when the window already spans every line.
        let content = "---\nbase: &b\n  a: 1\ntail: *b\n---\n";
        let regions = fm_ctx(content)
            .focused_yaml_regions(&[YamlKeyPath::dotted("tail")], 3)
            .unwrap();
        assert_eq!(regions, vec![region(1, 5, &[4])]);
    }

    #[test]
    fn focused_line_regions_keep_arm_and_parent_ancestors() {
        // Line 14 is arm 2's `doc:`; its ancestors are the `- design:` marker
        // (line 9) and `$schema:` (line 2).
        let regions = fm_ctx(UNION_DOC).focused_line_regions(&[14], 1).unwrap();
        assert_eq!(regions, vec![region(2, 2, &[]), region(9, 9, &[]), region(13, 15, &[14])]);

        // Both arms' marker lines: each is its own target, with `$schema:`.
        let regions = fm_ctx(UNION_DOC).focused_line_regions(&[3, 9], 0).unwrap();
        assert_eq!(regions, vec![region(2, 3, &[3]), region(9, 9, &[9])]);
    }

    #[test]
    fn focused_line_regions_find_the_owner_of_a_flush_sequence() {
        let flush = "---\na: 1\nb: 2\nc: 3\nlist:\n- x\n- y\n- z\n---\n";
        let regions = fm_ctx(flush).focused_line_regions(&[8], 0).unwrap();
        assert_eq!(regions, vec![region(5, 5, &[]), region(8, 8, &[8])]);
    }

    #[test]
    fn focused_line_regions_ignore_lines_outside_the_block() {
        let ctx = schema_ctx();
        assert_eq!(ctx.focused_line_regions(&[0, 500], 3), None);
        assert_eq!(ctx.focused_line_regions(&[], 3), None);
        let no_frontmatter =
            SourceContext::new(PathBuf::from("/a.md"), PathBuf::from("a.md"), "body\n");
        assert_eq!(no_frontmatter.focused_line_regions(&[1], 3), None);
    }

    #[test]
    fn focused_regions_unsafe_yaml_covered_but_for_a_delimiter_is_kept() {
        let content = "---\n# note\nseq: &s\n  - a\nprompt: x\nalias: *s\n---\n";
        let regions = fm_ctx(content).focused_line_regions(&[5], 3).unwrap();
        assert_eq!(regions, vec![region(2, 7, &[5])]);
    }

    #[test]
    fn focused_line_regions_refuse_a_partial_slice_of_anchored_yaml() {
        let content = "---\nbase: &b\n  a: 1\nx: 1\ny: 2\nz: 3\ntail: 2\n---\n";
        assert_eq!(fm_ctx(content).focused_line_regions(&[7], 1), None);
    }
}
