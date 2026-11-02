//! Passive location of a Markdown document's leading frontmatter block.
//!
//! This module finds the `---`-fenced YAML block at the top of a document and
//! reports byte spans against the exact source text. It does not parse the
//! YAML into values and knows nothing about Markdown rendering, so a crate that
//! only needs the frontmatter envelope (a schema reader, a hasher) can use it
//! without a Markdown dependency.

use crate::SourceSpan;

/// Byte-accurate location of a document's frontmatter block.
///
/// Produced by [`extract_frontmatter_block`]. All spans index into the exact
/// source text passed in: original line endings (including CRLF) are
/// preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterExtraction<'a> {
    /// Raw YAML text between the delimiters: every complete source line after
    /// the opening `---` up to (not including) the closing `---` line,
    /// terminators intact. Empty for empty frontmatter.
    pub yaml: &'a str,
    /// Byte span of [`Self::yaml`] within the source.
    pub yaml_span: SourceSpan,
    /// Byte span of the whole frontmatter block, from the start of the
    /// opening `---` line through the closing `---` line's terminator (or end
    /// of input when the closing delimiter is the final line). A recognized
    /// UTF-8 BOM immediately before the opening delimiter is included.
    pub block_span: SourceSpan,
    /// Byte span of the document body following the block: everything from
    /// `block_span.end` to the end of the source. Empty for body-less
    /// documents.
    pub body_span: SourceSpan,
    /// 1-indexed source line of the opening `---` delimiter (always `1`).
    pub opening_line: usize,
    /// 1-indexed source line of the closing `---` delimiter.
    pub closing_line: usize,
    /// 1-indexed source line of the first YAML line (`2` for ordinary
    /// frontmatter, since line 1 is the opening delimiter). This is the base
    /// line for projecting YAML-relative positions back into the document.
    pub yaml_base_line: usize,
}

/// Why a document's leading fence could not be read as frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrontmatterEnvelopeError {
    /// The document opens with a dash-only run longer than `---` that encloses
    /// a YAML mapping, so the author almost certainly meant frontmatter.
    #[error("frontmatter fence `{found}` on line {line} must be exactly `---`")]
    FenceMismatch {
        /// The offending fence text, e.g. `"----"`.
        found: String,
        /// 1-indexed line of the offending fence (always `1`).
        line: usize,
    },
}

/// One source line located by [`line_spans`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineSpan {
    /// Byte offset of the line's first character.
    pub start: usize,
    /// Byte offset just past the line's content, before its terminator.
    pub content_end: usize,
    /// Byte offset just past the line's terminator (equal to `content_end`
    /// for a final unterminated line).
    pub end: usize,
}

/// Splits `source` into lines, recognizing LF, CRLF, and lone-CR terminators
/// without losing byte offsets. A trailing terminator produces no empty final
/// line.
pub fn line_spans(source: &str) -> Vec<LineSpan> {
    let bytes = source.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut idx = 0;

    while idx < bytes.len() {
        let terminator_len = match bytes[idx] {
            b'\n' => 1,
            b'\r' if bytes.get(idx + 1) == Some(&b'\n') => 2,
            b'\r' => 1,
            _ => {
                idx += 1;
                continue;
            }
        };
        lines.push(LineSpan {
            start,
            content_end: idx,
            end: idx + terminator_len,
        });
        idx += terminator_len;
        start = idx;
    }

    if start < source.len() {
        lines.push(LineSpan {
            start,
            content_end: source.len(),
            end: source.len(),
        });
    }

    lines
}

/// Detects a near-miss frontmatter fence at the start of a document.
///
/// `lines` are the document's line contents (terminators removed, a leading
/// UTF-8 BOM already stripped from the first). Returns the offending fence
/// (e.g. `"----"`) when **all** of the following hold:
///
/// 1. The first line's trimmed content is a dash-only run of length `>= 4`.
/// 2. A later line's trimmed content is the **same** dash-only run.
/// 3. The strict interior between the two fences is non-empty and parses as
///    a YAML **mapping** with at least one key.
///
/// Otherwise the document has no frontmatter: a leading `----` thematic break
/// followed by prose is left untouched.
pub fn detect_near_miss_fence(lines: &[&str]) -> Option<String> {
    let first = lines.first()?.trim();
    if !is_dash_only_fence(first) || first.len() < 4 {
        return None;
    }

    let closing_idx = lines
        .iter()
        .skip(1)
        .position(|line| line.trim() == first)
        .map(|idx| idx + 1)?;

    let interior = &lines[1..closing_idx];
    if interior.is_empty() {
        return None;
    }

    let yaml_content = interior.join("\n");
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&yaml_content) {
        Ok(serde_yaml_ng::Value::Mapping(map)) if !map.is_empty() => Some(first.to_string()),
        _ => None,
    }
}

fn is_dash_only_fence(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b == b'-')
}

/// Locates a document's frontmatter block without parsing the YAML.
///
/// Frontmatter must start at line 1 with a trimmed `---`, optionally preceded
/// by one UTF-8 BOM; the closing delimiter is the next trimmed `---`; a
/// missing closing delimiter means the document has no frontmatter. LF, CRLF,
/// and lone-CR terminators are preserved in every span.
///
/// ## Examples
///
/// ```
/// use biscuit_file::frontmatter::extract_frontmatter_block;
///
/// let source = "---\ntitle: Hello\n---\n\n# Body\n";
/// let extraction = extract_frontmatter_block(source).unwrap().unwrap();
/// assert_eq!(extraction.yaml, "title: Hello\n");
/// assert_eq!(extraction.yaml_base_line, 2);
/// assert_eq!(&source[extraction.body_span.clone()], "\n# Body\n");
/// ```
///
/// ## Returns
///
/// `Ok(None)` when the document has no frontmatter block (it does not start
/// with `---`, or the closing delimiter is missing).
///
/// ## Errors
///
/// Returns [`FrontmatterEnvelopeError::FenceMismatch`] for a near-miss fence
/// (see [`detect_near_miss_fence`]).
pub fn extract_frontmatter_block(
    source: &str,
) -> Result<Option<FrontmatterExtraction<'_>>, FrontmatterEnvelopeError> {
    let line_spans = line_spans(source);
    let line_content = |idx: usize| -> &str {
        let line = line_spans[idx];
        &source[line.start..line.content_end]
    };
    let opening = line_spans
        .first()
        .map(|_| line_content(0).strip_prefix('\u{feff}').unwrap_or(line_content(0)));

    if opening.is_none_or(|line| line.trim() != "---") {
        let mut lines: Vec<&str> = line_spans
            .iter()
            .map(|line| &source[line.start..line.content_end])
            .collect();
        if let Some(first) = lines.first_mut() {
            *first = first.strip_prefix('\u{feff}').unwrap_or(first);
        }
        if let Some(found) = detect_near_miss_fence(&lines) {
            return Err(FrontmatterEnvelopeError::FenceMismatch { found, line: 1 });
        }
        return Ok(None);
    }

    let Some(closing_idx) = (1..line_spans.len()).find(|&idx| line_content(idx).trim() == "---")
    else {
        return Ok(None);
    };

    let yaml_span = line_spans[1].start..line_spans[closing_idx].start;
    let block_span = 0..line_spans[closing_idx].end;
    let body_span = block_span.end..source.len();

    Ok(Some(FrontmatterExtraction {
        yaml: &source[yaml_span.clone()],
        yaml_span,
        block_span,
        body_span,
        opening_line: 1,
        closing_line: closing_idx + 1,
        yaml_base_line: 2,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_frontmatter_is_none() {
        assert_eq!(extract_frontmatter_block("# Hi\n\nBody.\n").unwrap(), None);
        assert_eq!(extract_frontmatter_block("").unwrap(), None);
    }

    #[test]
    fn ordinary_block_spans() {
        let source = "---\ntitle: X\n---\n\n# Hi\n";
        let extraction = extract_frontmatter_block(source).unwrap().unwrap();

        assert_eq!(extraction.yaml, "title: X\n");
        assert_eq!(extraction.yaml_span, 4..13);
        assert_eq!(&source[extraction.yaml_span.clone()], extraction.yaml);
        assert_eq!(extraction.block_span, 0..17);
        assert_eq!(extraction.body_span, 17..source.len());
        assert_eq!(&source[extraction.body_span.clone()], "\n# Hi\n");
        assert_eq!(extraction.opening_line, 1);
        assert_eq!(extraction.closing_line, 3);
        assert_eq!(extraction.yaml_base_line, 2);
    }

    #[test]
    fn empty_block() {
        let source = "---\n---\nbody\n";
        let extraction = extract_frontmatter_block(source).unwrap().unwrap();

        assert_eq!(extraction.yaml, "");
        assert_eq!(extraction.yaml_span, 4..4);
        assert_eq!(extraction.block_span, 0..8);
        assert_eq!(&source[extraction.body_span.clone()], "body\n");
        assert_eq!(extraction.closing_line, 2);
        assert_eq!(extraction.yaml_base_line, 2);
    }

    #[test]
    fn crlf_delimiters_keep_terminators() {
        let source = "---\r\ntitle: X\r\n---\r\nbody";
        let extraction = extract_frontmatter_block(source).unwrap().unwrap();

        assert_eq!(extraction.yaml, "title: X\r\n");
        assert_eq!(extraction.yaml_span, 5..15);
        assert_eq!(extraction.block_span, 0..20);
        assert_eq!(&source[extraction.body_span.clone()], "body");
        assert_eq!(extraction.opening_line, 1);
        assert_eq!(extraction.closing_line, 3);
    }

    #[test]
    fn utf8_bom_is_inside_the_block() {
        let source = "\u{feff}---\ntitle: X\n---\nbody";
        let extraction = extract_frontmatter_block(source).unwrap().unwrap();

        assert_eq!(extraction.yaml, "title: X\n");
        assert_eq!(extraction.yaml_span, 7..16);
        assert_eq!(extraction.block_span, 0..20);
        assert_eq!(&source[extraction.body_span.clone()], "body");
    }

    #[test]
    fn lone_cr_delimiters() {
        let source = "---\rtitle: X\r---\rbody";
        let extraction = extract_frontmatter_block(source).unwrap().unwrap();

        assert_eq!(extraction.yaml, "title: X\r");
        assert_eq!(extraction.yaml_span, 4..13);
        assert_eq!(extraction.block_span, 0..17);
        assert_eq!(&source[extraction.body_span.clone()], "body");
        assert_eq!(extraction.closing_line, 3);
    }

    #[test]
    fn bom_does_not_weaken_delimiter_rules() {
        assert_eq!(
            extract_frontmatter_block("\u{feff}---x\ntitle: X\n---\nbody").unwrap(),
            None
        );
    }

    #[test]
    fn near_miss_fence_is_a_typed_error() {
        let err = extract_frontmatter_block("----\na: 1\n----\nbody").unwrap_err();
        assert_eq!(
            err,
            FrontmatterEnvelopeError::FenceMismatch {
                found: "----".to_string(),
                line: 1
            }
        );
    }

    #[test]
    fn dash_run_without_a_yaml_mapping_is_not_frontmatter() {
        // A thematic break around prose, an empty interior, and an unmatched
        // run are all body text.
        assert_eq!(extract_frontmatter_block("----\njust prose\n----\n").unwrap(), None);
        assert_eq!(extract_frontmatter_block("----\n----\n").unwrap(), None);
        assert_eq!(extract_frontmatter_block("----\na: 1\n-----\n").unwrap(), None);
    }

    #[test]
    fn missing_closing_delimiter_is_none() {
        assert_eq!(extract_frontmatter_block("---\ntitle: X\n").unwrap(), None);
        assert_eq!(extract_frontmatter_block("---").unwrap(), None);
    }

    #[test]
    fn junk_after_the_closer_is_body() {
        let source = "---\na: 1\n---\n}}} not: [yaml\n";
        let extraction = extract_frontmatter_block(source).unwrap().unwrap();
        assert_eq!(extraction.yaml, "a: 1\n");
        assert_eq!(&source[extraction.body_span.clone()], "}}} not: [yaml\n");
    }

    #[test]
    fn bodyless_document() {
        let source = "---\na: 1\n---";
        let extraction = extract_frontmatter_block(source).unwrap().unwrap();

        assert_eq!(extraction.yaml, "a: 1\n");
        assert_eq!(extraction.yaml_span, 4..9);
        assert_eq!(extraction.block_span, 0..source.len());
        assert_eq!(extraction.body_span, source.len()..source.len());
        assert_eq!(extraction.closing_line, 3);
    }

    #[test]
    fn line_spans_cover_every_terminator() {
        let spans = line_spans("a\r\nb\rc\nd");
        let starts: Vec<_> = spans.iter().map(|s| (s.start, s.content_end, s.end)).collect();
        assert_eq!(starts, vec![(0, 1, 3), (3, 4, 5), (5, 6, 7), (7, 8, 8)]);
        assert!(line_spans("").is_empty());
        assert_eq!(line_spans("x\n").len(), 1);
    }
}
