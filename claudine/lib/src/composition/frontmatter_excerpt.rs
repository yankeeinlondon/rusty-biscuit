//! Captured frontmatter YAML for rendering an error-attached source excerpt.
//!
//! When a composition error is rooted in a prompt file's YAML frontmatter, the
//! report is far easier to act on if it shows the frontmatter itself with the
//! offending line highlighted. [`FrontmatterExcerpt`] captures the verbatim
//! frontmatter block (delimiters included, so its line numbers match the source
//! file) plus the 1-based line to highlight, and renders it through the
//! [`CodeBlock`] component.
//!
//! The excerpt is attached to an error at the render boundary (see
//! `CompositionError::enrich_frontmatter`) and appended after the primary
//! diagnostic by the CLI's error walker. Rendering is withheld in non-TTY
//! output to avoid exposing frontmatter into pipes, logs, and CI — the same
//! privacy posture the inline-compose / sequence mismatch diagnostic uses.

use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use darkmatter::markdown::CodeBlock;
use darkmatter::markdown::dsl::CodeBlockMeta;

/// A captured frontmatter block plus the line to highlight, ready to render as
/// a YAML [`CodeBlock`] appended to an error report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterExcerpt {
    /// The frontmatter block including its `---` delimiter lines, so a
    /// 1-based line within `block` equals the same line in the source file.
    block: String,
    /// 1-based source-file line to highlight, when the offending property could
    /// be located within the frontmatter. `None` renders a plain numbered block.
    highlight_line: Option<usize>,
    /// Whether stderr was a TTY when the excerpt was captured. Gates rendering:
    /// non-TTY output omits the block entirely.
    stderr_is_tty: bool,
}

impl FrontmatterExcerpt {
    /// Capture an excerpt from a document's full source text.
    ///
    /// `property` is a dotted frontmatter key (e.g. `"success.message"`) to
    /// highlight; when `None` or not locatable, no line is highlighted.
    ///
    /// ## Returns
    ///
    /// `None` when `source_text` has no well-formed frontmatter block.
    pub fn capture(source_text: &str, property: Option<&str>, stderr_is_tty: bool) -> Option<Self> {
        let block = capture_frontmatter_block(source_text)?;
        let highlight_line = property.and_then(|p| locate_property_line(&block, p));
        Some(Self {
            block,
            highlight_line,
            stderr_is_tty,
        })
    }

    /// Capture an excerpt for a schema-body parse failure, mapping a byte span
    /// within the offending property's type-and-constraint string to a source
    /// line.
    ///
    /// `property` is the dotted schema-property path (e.g. `"$schema.spec"`) the
    /// typed `SchemaError::Grammar` failure was attributed to. The byte span is
    /// an offset **into that property's type-and-constraint string** (not the
    /// document), so it is mapped to a source line by counting the line-breaks
    /// the span crosses inside the property's value region. For the usual
    /// single-line type string (`spec: file(required, match(...))`) the span
    /// crosses no line-break and the highlight lands on the property line; for a
    /// multi-line YAML block scalar it advances to the continuation line the span
    /// points into.
    ///
    /// When `property` is `None`, or the property cannot be located, the
    /// highlight falls back to the `$schema` parent line.
    ///
    /// ## Returns
    ///
    /// `None` when `source_text` has no well-formed frontmatter block.
    pub fn capture_schema_span(
        source_text: &str,
        property: Option<&str>,
        span_start: usize,
        stderr_is_tty: bool,
    ) -> Option<Self> {
        let block = capture_frontmatter_block(source_text)?;
        let highlight_line = property
            .and_then(|p| locate_property_line(&block, p))
            .map(|line| line + value_line_offset(&block, line, span_start))
            .or_else(|| locate_property_line(&block, "$schema"));
        Some(Self {
            block,
            highlight_line,
            stderr_is_tty,
        })
    }

    /// Capture a near-miss frontmatter excerpt by line number.
    ///
    /// Recognizes a matched dash-only (`----`+) fence pair at the top of the
    /// document and returns the block with the requested line highlighted.
    /// This is used for errors like `FrontmatterFenceMismatch` where the
    /// offending token is the delimiter itself, not a YAML property.
    ///
    /// ## Returns
    ///
    /// `None` when `source_text` has no matched near-miss dash-only fence pair.
    pub fn capture_line(source_text: &str, line: usize, stderr_is_tty: bool) -> Option<Self> {
        let block = capture_near_miss_frontmatter_block(source_text)?;
        Some(Self {
            block,
            highlight_line: Some(line),
            stderr_is_tty,
        })
    }

    /// Render the excerpt as a trailing appendix for an error report.
    ///
    /// Returns an empty string in non-TTY output (privacy gating). Otherwise
    /// returns the YAML [`CodeBlock`] prefixed with a blank-line separator. SGR
    /// and OSC 8 escapes are stripped when the terminal has no color depth, so
    /// redirected / `NO_COLOR` output stays plain text.
    pub fn render_appendix(&self, term: &Terminal) -> String {
        if !self.stderr_is_tty {
            return String::new();
        }
        let mut meta = CodeBlockMeta {
            line_numbering: true,
            ..CodeBlockMeta::default()
        };
        if let Some(line) = self.highlight_line {
            meta.highlight.add_line(line);
        }
        let rendered = CodeBlock::yaml(self.block.clone()).with_meta(meta).render(term);
        let body = if matches!(term.color_depth, ColorDepth::None) {
            strip_escape_codes(&rendered)
        } else {
            rendered
        };
        format!("\n\n{}", body.trim_end_matches('\n'))
    }
}

#[cfg(test)]
impl FrontmatterExcerpt {
    /// Test-only accessor for the captured highlight line.
    pub fn highlight_line(&self) -> Option<usize> {
        self.highlight_line
    }
}

/// Capture the frontmatter block **including** its `---` delimiter lines.
///
/// Unlike [`capture_frontmatter_yaml`](super::mismatch::capture_frontmatter_yaml),
/// which returns only the interior, this keeps the delimiters so a rendered
/// [`CodeBlock`]'s 1-based line numbers line up with the source file (whose
/// frontmatter always opens on line 1). A single trailing line-ending after the
/// closing delimiter is stripped; all interior line-endings are preserved.
///
/// ## Returns
///
/// `None` when `text` has no well-formed frontmatter block (no leading `---`
/// line, or no closing `---` line).
pub fn capture_frontmatter_block(text: &str) -> Option<String> {
    let mut lines = text.split_inclusive('\n');

    let opening = lines.next()?;
    if opening.trim() != "---" {
        return None;
    }

    let mut end = opening.len();
    for line in lines {
        end += line.len();
        if line.trim() == "---" {
            let block = &text[..end];
            return Some(block.strip_suffix('\n').unwrap_or(block).to_string());
        }
    }

    None
}

/// Capture a near-miss frontmatter block **including** its `----`+ delimiter lines.
///
/// This is the counterpart to [`capture_frontmatter_block`] for the malformed
/// dash-only fence case. It recognizes a matched pair of four-or-more dash
/// fences (`----`/`-----`/...) and returns the full block, delimiters included,
/// so line 1 in the captured block equals line 1 in the source file.
///
/// ## Returns
///
/// `None` when `text` has no matched near-miss dash-only fence pair at the top
/// of the document.
pub fn capture_near_miss_frontmatter_block(text: &str) -> Option<String> {
    let mut lines = text.split_inclusive('\n');

    let opening = lines.next()?;
    let opening_trimmed = opening.trim();
    if !is_dash_only_fence(opening_trimmed) || opening_trimmed.len() < 4 {
        return None;
    }

    let mut end = opening.len();
    for line in lines {
        end += line.len();
        if line.trim() == opening_trimmed {
            let block = &text[..end];
            return Some(block.strip_suffix('\n').unwrap_or(block).to_string());
        }
    }

    None
}

/// Returns `true` when `text` is non-empty and contains only `-` characters.
fn is_dash_only_fence(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b == b'-')
}

/// Locate the 1-based line of a dotted frontmatter property within a captured
/// `block` (delimiters included).
///
/// Resolves nested keys by indentation: each segment is matched among the
/// sibling keys that share the indentation level established by the first
/// non-blank line inside the parent's scope. Blank and comment lines are
/// skipped. Returns `None` when any segment cannot be found.
///
/// ## Examples
///
/// ```ignore
/// // block line 1 is `---`; `success.message` resolves to its file line.
/// assert_eq!(locate_property_line(block, "success.message"), Some(16));
/// ```
pub fn locate_property_line(block: &str, dotted_property: &str) -> Option<usize> {
    let lines: Vec<&str> = block.lines().collect();
    let mut found: Option<usize> = None;
    // Top-level keys sit above any nesting, so the first parent indent is below
    // zero — every real key has indent >= 0 and thus deeper than the root.
    let mut parent_indent: isize = -1;
    let mut lo = 0usize;

    for segment in dotted_property.split('.') {
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
            if key_name(line) == Some(segment) {
                matched = Some(idx);
                break;
            }
        }

        let idx = matched?;
        found = Some(idx);
        parent_indent = indent_of(lines[idx]);
        lo = idx + 1;
    }

    found.map(|idx| idx + 1)
}

/// Map a byte offset within a property's type-and-constraint string to the
/// number of source line-breaks it crosses, so a schema-grammar `span` can
/// advance the highlight onto the right continuation line of a multi-line value.
///
/// `property_line` is the 1-based source line of the `key:` entry inside `block`.
/// The property's value text is everything after the first `:` on that line plus
/// any deeper-indented continuation lines (a YAML block scalar). The returned
/// offset is `0` for the common single-line type string, since its value text
/// holds no line-break before `span_start`.
fn value_line_offset(block: &str, property_line: usize, span_start: usize) -> usize {
    let lines: Vec<&str> = block.lines().collect();
    let Some(key_idx) = property_line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return 0;
    };
    let key_indent = indent_of(lines[key_idx]);

    // Reconstruct the value text exactly as the schema lexer saw it: the inline
    // remainder after `key:`, then each deeper-indented continuation line joined
    // by the `\n` the lexer's span counts against.
    let inline = lines[key_idx]
        .split_once(':')
        .map_or("", |(_, rest)| rest.trim_start());
    let mut value = String::from(inline);
    for line in &lines[key_idx + 1..] {
        if is_blank_or_comment(line) {
            value.push('\n');
            continue;
        }
        if indent_of(line) <= key_indent {
            break;
        }
        value.push('\n');
        value.push_str(line.trim_start());
    }

    // Clamp to a char boundary at or below `span_start` so a span landing mid
    // UTF-8 sequence (e.g. a non-ASCII description) never panics the slice.
    let mut end = span_start.min(value.len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].matches('\n').count()
}

/// The byte-width of a line's leading spaces, as a signed value for comparison
/// against the synthetic `-1` root indent.
fn indent_of(line: &str) -> isize {
    (line.len() - line.trim_start().len()) as isize
}

fn is_blank_or_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.is_empty() || trimmed.starts_with('#')
}

/// Extract the mapping key from a `key: value` line, or `None` when the line is
/// not a plain mapping entry (e.g. a list item or a bare scalar).
fn key_name(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('-') {
        return None;
    }
    let (key, _) = trimmed.split_once(':')?;
    let key = key.trim();
    Some(key.trim_matches(|c| c == '"' || c == '\''))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: written as one literal (no `\`-newline continuations) because those
    // would strip the leading YAML indentation that the nesting tests rely on.
    const DOC: &str = "---\ntitle: Example\niteration: 1\nsuccess:\n    message: \"done at {{review_file}}\"\n    effect: cheer\nfailure:\n    message: \"failed\"\n---\nbody\n";

    #[test]
    fn block_includes_delimiters_so_lines_match_file() {
        let block = capture_frontmatter_block(DOC).unwrap();
        assert!(block.starts_with("---\n"));
        assert!(block.ends_with("\n---"));
        // Line 1 is the opening delimiter, matching the source file.
        assert_eq!(block.lines().next(), Some("---"));
    }

    #[test]
    fn block_none_without_closing_delimiter() {
        assert_eq!(capture_frontmatter_block("---\ntitle: x\nbody\n"), None);
    }

    #[test]
    fn block_none_without_opening_delimiter() {
        assert_eq!(capture_frontmatter_block("title: x\n"), None);
    }

    #[test]
    fn locate_top_level_key() {
        let block = capture_frontmatter_block(DOC).unwrap();
        // `iteration:` is the 3rd line of the file (after `---`, `title:`).
        assert_eq!(locate_property_line(&block, "iteration"), Some(3));
    }

    #[test]
    fn locate_nested_key() {
        let block = capture_frontmatter_block(DOC).unwrap();
        // `success.message` is on line 5.
        assert_eq!(locate_property_line(&block, "success.message"), Some(5));
    }

    #[test]
    fn locate_nested_key_under_second_parent() {
        let block = capture_frontmatter_block(DOC).unwrap();
        // `failure.message` is on line 8 — must not match `success.message`.
        assert_eq!(locate_property_line(&block, "failure.message"), Some(8));
    }

    #[test]
    fn locate_absent_key_is_none() {
        let block = capture_frontmatter_block(DOC).unwrap();
        assert_eq!(locate_property_line(&block, "missing"), None);
        assert_eq!(locate_property_line(&block, "success.absent"), None);
    }

    #[test]
    fn locate_does_not_match_value_substring() {
        let block = capture_frontmatter_block(DOC).unwrap();
        // `message` appears as a value-bearing key under success/failure but
        // never as a top-level key.
        assert_eq!(locate_property_line(&block, "message"), None);
    }

    #[test]
    fn capture_returns_none_without_frontmatter() {
        assert!(FrontmatterExcerpt::capture("no frontmatter", Some("x"), true).is_none());
    }

    #[test]
    fn appendix_empty_when_not_tty() {
        let excerpt = FrontmatterExcerpt::capture(DOC, Some("success.message"), false).unwrap();
        let term = Terminal::new_optimistic(80);
        assert_eq!(excerpt.render_appendix(&term), "");
    }

    #[test]
    fn appendix_shows_yaml_when_tty() {
        let excerpt = FrontmatterExcerpt::capture(DOC, Some("success.message"), true).unwrap();
        let term = Terminal::new_optimistic(80);
        let rendered = strip_escape_codes(excerpt.render_appendix(&term));
        assert!(rendered.contains("message"), "got: {rendered}");
        assert!(rendered.contains("iteration"), "got: {rendered}");
    }

    #[test]
    fn appendix_plain_when_no_color() {
        let excerpt = FrontmatterExcerpt::capture(DOC, Some("success.message"), true).unwrap();
        let term = Terminal::builder()
            .width(80)
            .color_depth(ColorDepth::None)
            .build();
        let rendered = excerpt.render_appendix(&term);
        assert!(
            !rendered.contains('\x1b'),
            "plain appendix must have no escape bytes; got: {rendered:?}"
        );
    }

    // An inline `$schema` mapping whose `spec` type-string has a bad constraint
    // separator (`,` instead of `;`). `spec` is the 3rd file line.
    const SCHEMA_DOC: &str =
        "---\n$schema:\n    spec: file(required, match(**/*spec*.md))\nspec: \"x\"\n---\nbody\n";

    #[test]
    fn schema_span_highlights_offending_property_line() {
        // The span points into the single-line `spec` type string, so it must
        // land on the property's own line (line 3), not the `$schema` parent.
        let excerpt =
            FrontmatterExcerpt::capture_schema_span(SCHEMA_DOC, Some("$schema.spec"), 13, true)
                .unwrap();
        assert_eq!(excerpt.highlight_line, Some(3));
    }

    #[test]
    fn schema_span_does_not_highlight_unrelated_line() {
        // The top-level `spec: "x"` value on line 4 must never be highlighted in
        // place of the `$schema.spec` type-string line.
        let excerpt =
            FrontmatterExcerpt::capture_schema_span(SCHEMA_DOC, Some("$schema.spec"), 13, true)
                .unwrap();
        assert_ne!(excerpt.highlight_line, Some(4));
    }

    #[test]
    fn schema_span_falls_back_to_schema_parent_without_property() {
        // A structural failure with no real property name falls back to the
        // `$schema:` parent line (line 2).
        let excerpt =
            FrontmatterExcerpt::capture_schema_span(SCHEMA_DOC, None, 0, true).unwrap();
        assert_eq!(excerpt.highlight_line, Some(2));
    }

    #[test]
    fn value_line_offset_zero_for_single_line_value() {
        let block = capture_frontmatter_block(SCHEMA_DOC).unwrap();
        // Any in-range span into the single-line `spec` value crosses no newline.
        assert_eq!(value_line_offset(&block, 3, 0), 0);
        assert_eq!(value_line_offset(&block, 3, 13), 0);
    }

    #[test]
    fn value_line_offset_counts_newlines_across_continuation_lines() {
        // Defensive mechanic: when a value's reconstructed text spans physical
        // lines (a YAML block scalar), the offset counts the line-breaks the span
        // crosses. Real SimplifiedSchema type strings are single-line, so this
        // path returns 0 in practice; the test pins the multi-line arithmetic.
        let doc = "---\n$schema:\n    spec: a\n      b\n      c\n---\nbody\n";
        let block = capture_frontmatter_block(doc).unwrap();
        // Reconstructed value text for `spec` is "a\nb\nc"; a span past the first
        // newline lands one continuation line down, past the second lands two.
        assert_eq!(value_line_offset(&block, 3, 0), 0);
        assert_eq!(value_line_offset(&block, 3, "a\nb".len()), 1);
        assert_eq!(value_line_offset(&block, 3, "a\nb\nc".len()), 2);
    }

    #[test]
    fn schema_span_appendix_withheld_when_not_tty() {
        let excerpt =
            FrontmatterExcerpt::capture_schema_span(SCHEMA_DOC, Some("$schema.spec"), 13, false)
                .unwrap();
        let term = Terminal::new_optimistic(80);
        assert_eq!(excerpt.render_appendix(&term), "");
    }

    const NEAR_MISS_DOC: &str = "----\nname: cross-platform\ndescription: near-miss fence\n----\n# Body\n";

    #[test]
    fn capture_line_recognizes_four_dash_fence() {
        let excerpt = FrontmatterExcerpt::capture_line(NEAR_MISS_DOC, 1, true).unwrap();
        assert_eq!(excerpt.highlight_line, Some(1));
        assert!(excerpt.block.starts_with("----\n"), "block must include opening fence");
        assert!(excerpt.block.ends_with("\n----"), "block must include closing fence");
    }

    #[test]
    fn capture_line_none_for_plain_prose() {
        assert!(FrontmatterExcerpt::capture_line("no frontmatter here\n", 1, true).is_none());
    }

    #[test]
    fn capture_line_none_for_valid_three_dash_fence() {
        assert!(FrontmatterExcerpt::capture_line(DOC, 1, true).is_none());
    }

    #[test]
    fn capture_line_appendix_empty_when_not_tty() {
        let excerpt = FrontmatterExcerpt::capture_line(NEAR_MISS_DOC, 1, false).unwrap();
        let term = Terminal::new_optimistic(80);
        assert_eq!(excerpt.render_appendix(&term), "");
    }

    #[test]
    fn capture_line_appendix_highlights_fence_line() {
        let excerpt = FrontmatterExcerpt::capture_line(NEAR_MISS_DOC, 1, true).unwrap();
        assert_eq!(excerpt.highlight_line, Some(1));
        let term = Terminal::new_optimistic(80);
        let rendered = strip_escape_codes(excerpt.render_appendix(&term));
        assert!(rendered.contains("name:"), "yaml block missing: {rendered}");
        assert!(rendered.contains("----"), "fence line missing: {rendered}");
    }
}
