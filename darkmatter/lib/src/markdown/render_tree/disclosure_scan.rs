//! Read-only, span-carrying scanner for `::disclosure` blocks.
//!
//! A passive companion to the render-time [`BlockExtensionProcessor`]
//! (`super::block_extension`): it locates each
//! `::disclosure` / `::details` / `::end-disclosure` triple in a source document
//! and reports its byte spans (opener, summary region, body region, closer, and
//! the recognized opener-style token spans) **without** producing render events
//! or changing any render behavior. A language server folds, hovers, and
//! navigates disclosures from this product.
//!
//! Structural malformation keeps raising [`MarkdownError::MalformedDisclosure`],
//! the same error the render path raises, so a passive analyzer and the renderer
//! agree on what "malformed" means: an unmatched `::end-disclosure`, or an
//! opener that never closes.

use crate::markdown::MarkdownError;
use crate::markdown::compose::parse_utils::{find_code_regions, is_in_code_region};
use crate::markdown::render_tree::disclosure_style::disclosure_style_token_spans;
use crate::markdown::span::SourceSpan;

/// Opening marker for a disclosure block.
const DISCLOSURE_OPENER: &str = "::disclosure";
/// Separator between summary and body in a disclosure block.
const DISCLOSURE_DETAILS: &str = "::details";
/// Closing marker for a disclosure block.
const DISCLOSURE_CLOSER: &str = "::end-disclosure";

/// A read-only parse product for one `::disclosure` block.
///
/// Every span is a byte-offset range into the scanned source. `summary_span`
/// and `body_span` are `None` when the corresponding region is empty (e.g. a
/// disclosure with no `::details` divider carries no body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisclosureParse {
    /// Byte span from the opener line through the closer line.
    pub block_span: SourceSpan,
    /// Byte span of the `::disclosure` opener line (trimmed content).
    pub opener_span: SourceSpan,
    /// Byte span of the summary region (opener-line summary plus any lines up to
    /// `::details`), trimmed; `None` when empty.
    pub summary_span: Option<SourceSpan>,
    /// Byte span of the `::details` divider line, when present.
    pub details_span: Option<SourceSpan>,
    /// Byte span of the disclosed body region (between `::details` and the
    /// closer), trimmed; `None` when empty or absent.
    pub body_span: Option<SourceSpan>,
    /// Byte span of the `::end-disclosure` closer line (trimmed content).
    pub closer_span: SourceSpan,
    /// Byte spans of the recognized `key=value` style tokens on the opener line,
    /// in source order.
    pub style_token_spans: Vec<SourceSpan>,
}

/// Frame for an open disclosure while the scanner walks its region.
struct OpenFrame {
    /// Document offset of the opener line start (including indentation).
    block_start: usize,
    /// Trimmed opener line span.
    opener_span: SourceSpan,
    /// Recognized opener-style token spans (document offsets).
    style_token_spans: Vec<SourceSpan>,
    /// Document offset where the summary region begins.
    summary_start: usize,
    /// `::details` line span, once seen.
    details_span: Option<SourceSpan>,
    /// Document offset of the summary region's end once `::details` is seen
    /// (the start of the `::details` line).
    summary_end: Option<usize>,
    /// Document offset where the body region begins (after `::details`).
    body_start: Option<usize>,
}

/// Trims leading/trailing whitespace of `source[start..end]`, returning the
/// tightened span or `None` when the region is empty.
fn trim_span(source: &str, start: usize, end: usize) -> Option<SourceSpan> {
    if start >= end {
        return None;
    }
    let slice = &source[start..end];
    let trimmed = slice.trim();
    if trimmed.is_empty() {
        return None;
    }
    let leading = slice.len() - slice.trim_start().len();
    let s = start + leading;
    Some(s..s + trimmed.len())
}

/// Matches a keyword at the start of `trimmed` at a word boundary (trailing
/// whitespace or end-of-line), tolerating trailing content.
fn matches_keyword(trimmed: &str, keyword: &str) -> bool {
    trimmed
        .strip_prefix(keyword)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
}

/// Scans `source` for `::disclosure` blocks and returns them in closing order
/// (innermost blocks precede the outer block that contains them).
///
/// Directives inside fenced or inline code regions are ignored.
///
/// ## Errors
///
/// Returns [`MarkdownError::MalformedDisclosure`] for an `::end-disclosure`
/// without a matching opener, or an opener that reaches end-of-file unclosed.
pub fn scan_disclosures(source: &str) -> Result<Vec<DisclosureParse>, MarkdownError> {
    let code_regions = find_code_regions(source);
    let bytes = source.as_bytes();

    let mut stack: Vec<OpenFrame> = Vec::new();
    let mut parsed: Vec<DisclosureParse> = Vec::new();

    let mut line_start = 0usize;

    for i in 0..=bytes.len() {
        let is_eol = i == bytes.len() || bytes[i] == b'\n';
        if !is_eol {
            continue;
        }

        let text_line_end = if i > line_start && bytes[i - 1] == b'\r' {
            i - 1
        } else {
            i
        };
        let span_end = if i < bytes.len() { i + 1 } else { i };
        let line = &source[line_start..text_line_end];
        let trimmed = line.trim();
        let trimmed_start = line_start + (line.len() - line.trim_start().len());
        let trimmed_end = trimmed_start + trimmed.len();

        if !trimmed.is_empty() && !is_in_code_region(trimmed_start, &code_regions) {
            if matches_keyword(trimmed, DISCLOSURE_OPENER) {
                stack.push(open_frame(
                    line_start,
                    trimmed,
                    trimmed_start,
                    trimmed_end,
                    span_end,
                ));
            } else if matches_keyword(trimmed, DISCLOSURE_DETAILS) {
                if let Some(frame) = stack.last_mut()
                    && frame.details_span.is_none()
                {
                    frame.details_span = Some(trimmed_start..trimmed_end);
                    frame.summary_end = Some(line_start);
                    frame.body_start = Some(span_end);
                }
            } else if matches_keyword(trimmed, DISCLOSURE_CLOSER) {
                let Some(frame) = stack.pop() else {
                    return Err(MarkdownError::MalformedDisclosure {
                        reason: "`::end-disclosure` without a matching `::disclosure`".to_string(),
                        range: trimmed_start..trimmed_end,
                    });
                };
                parsed.push(finalize(source, frame, line_start, trimmed_start, trimmed_end));
            }
        }

        line_start = span_end;
    }

    if let Some(frame) = stack.last() {
        return Err(MarkdownError::MalformedDisclosure {
            reason: "disclosure block is missing `::end-disclosure`".to_string(),
            range: frame.opener_span.clone(),
        });
    }

    Ok(parsed)
}

/// Builds an [`OpenFrame`] from a matched `::disclosure` opener line.
fn open_frame(
    block_start: usize,
    trimmed: &str,
    trimmed_start: usize,
    trimmed_end: usize,
    next_line_start: usize,
) -> OpenFrame {
    // Text after the keyword, with the leading whitespace skipped; `rest_start`
    // is the document offset of that text.
    let after_kw = &trimmed[DISCLOSURE_OPENER.len()..];
    let rest = after_kw.trim_start();
    let rest_start = trimmed_start + DISCLOSURE_OPENER.len() + (after_kw.len() - rest.len());

    let style_token_spans: Vec<SourceSpan> = disclosure_style_token_spans(rest)
        .into_iter()
        .map(|r| rest_start + r.start..rest_start + r.end)
        .collect();

    // Summary text on the opener line begins after the last style token.
    let summary_local_start = style_token_spans
        .last()
        .map(|last| last.end - rest_start)
        .unwrap_or(0);
    let opener_summary = rest[summary_local_start..].trim_start();

    let summary_start = if opener_summary.is_empty() {
        // No summary on the opener line — it begins on the following line.
        next_line_start
    } else {
        trimmed_end - opener_summary.len()
    };

    OpenFrame {
        block_start,
        opener_span: trimmed_start..trimmed_end,
        style_token_spans,
        summary_start,
        details_span: None,
        summary_end: None,
        body_start: None,
    }
}

/// Finalizes a popped frame against its `::end-disclosure` closer line.
fn finalize(
    source: &str,
    frame: OpenFrame,
    closer_line_start: usize,
    closer_trimmed_start: usize,
    closer_trimmed_end: usize,
) -> DisclosureParse {
    // When `::details` was seen, the summary ends at the details line and the
    // body runs from after it to the closer. Otherwise the whole region between
    // the opener and closer is summary with no body.
    let summary_end = frame.summary_end.unwrap_or(closer_line_start);
    let summary_span = trim_span(source, frame.summary_start, summary_end);

    let body_span = frame
        .body_start
        .and_then(|start| trim_span(source, start, closer_line_start));

    DisclosureParse {
        block_span: frame.block_start..closer_trimmed_end,
        opener_span: frame.opener_span,
        summary_span,
        details_span: frame.details_span,
        body_span,
        closer_span: closer_trimmed_start..closer_trimmed_end,
        style_token_spans: frame.style_token_spans,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_simple_disclosure() {
        let source = "::disclosure Summary\n::details\nBody text\n::end-disclosure\n";
        let blocks = scan_disclosures(source).unwrap();
        assert_eq!(blocks.len(), 1);
        let b = &blocks[0];
        assert_eq!(&source[b.opener_span.clone()], "::disclosure Summary");
        assert_eq!(&source[b.summary_span.clone().unwrap()], "Summary");
        assert_eq!(&source[b.details_span.clone().unwrap()], "::details");
        assert_eq!(&source[b.body_span.clone().unwrap()], "Body text");
        assert_eq!(&source[b.closer_span.clone()], "::end-disclosure");
        assert!(b.style_token_spans.is_empty());
    }

    #[test]
    fn captures_opener_style_token_spans() {
        let source = "::disclosure max-width=60ch color=red-500 License Agreement\n::details\nx\n::end-disclosure\n";
        let blocks = scan_disclosures(source).unwrap();
        let b = &blocks[0];
        assert_eq!(b.style_token_spans.len(), 2);
        assert_eq!(&source[b.style_token_spans[0].clone()], "max-width=60ch");
        assert_eq!(&source[b.style_token_spans[1].clone()], "color=red-500");
        // Summary is the text after the recognized style tokens.
        assert_eq!(&source[b.summary_span.clone().unwrap()], "License Agreement");
    }

    #[test]
    fn multiline_summary_spans_to_details() {
        let source = "::disclosure\nLine one\nLine two\n::details\nBody\n::end-disclosure\n";
        let b = &scan_disclosures(source).unwrap()[0];
        assert_eq!(&source[b.summary_span.clone().unwrap()], "Line one\nLine two");
    }

    #[test]
    fn nested_disclosures_close_innermost_first() {
        let source = concat!(
            "::disclosure Outer\n",
            "::details\n",
            "::disclosure Inner\n",
            "::details\n",
            "inner body\n",
            "::end-disclosure\n",
            "outer body\n",
            "::end-disclosure\n",
        );
        let blocks = scan_disclosures(source).unwrap();
        assert_eq!(blocks.len(), 2);
        // Inner closes first, so it is emitted first.
        assert_eq!(&source[blocks[0].summary_span.clone().unwrap()], "Inner");
        assert_eq!(&source[blocks[1].summary_span.clone().unwrap()], "Outer");
        // The outer body region contains the entire inner block.
        let outer_body = &source[blocks[1].body_span.clone().unwrap()];
        assert!(outer_body.contains("::disclosure Inner"));
        assert!(outer_body.contains("outer body"));
    }

    #[test]
    fn unmatched_closer_is_malformed() {
        let err = scan_disclosures("::end-disclosure\n").unwrap_err();
        assert!(matches!(err, MarkdownError::MalformedDisclosure { .. }));
    }

    #[test]
    fn unterminated_opener_is_malformed() {
        let err = scan_disclosures("::disclosure Summary\n::details\nbody\n").unwrap_err();
        match err {
            MarkdownError::MalformedDisclosure { reason, .. } => {
                assert!(reason.contains("missing"), "{reason}");
            }
            other => panic!("expected MalformedDisclosure, got {other:?}"),
        }
    }

    #[test]
    fn directives_inside_code_fence_are_ignored() {
        let source = "```\n::disclosure X\n::details\ny\n::end-disclosure\n```\n";
        assert!(scan_disclosures(source).unwrap().is_empty());
    }

    #[test]
    fn disclosure_without_details_has_no_body() {
        let source = "::disclosure Just a summary\n::end-disclosure\n";
        let b = &scan_disclosures(source).unwrap()[0];
        assert_eq!(&source[b.summary_span.clone().unwrap()], "Just a summary");
        assert!(b.details_span.is_none());
        assert!(b.body_span.is_none());
    }
}
