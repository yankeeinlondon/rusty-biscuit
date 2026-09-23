//! Maps byte offsets in a partially composed body back to the loaded document.
//!
//! The stages before body interpolation (text replacement, page blocks, and
//! directive targets) rewrite the body, so a failing expression's scanner span
//! indexes text that no longer matches the file. Each of those stages reports
//! the [`TextEdit`]s it applied, and [`BodyOrigin`] composes them into a map
//! from the current text back to the loaded source. A span projects only when
//! every byte of it was copied from the file; text a stage inserted has no
//! origin.

use std::hash::{Hash, Hasher};
use std::ops::Range;

use crate::markdown::Markdown;

/// One replacement a stage applied to its input text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextEdit {
    /// The replaced byte range of the stage's input.
    pub range: Range<usize>,
    /// Byte length of the text written in its place.
    pub replacement_len: usize,
}

/// A run of `len` bytes at `output` in the current text that was copied from
/// `origin` in the loaded document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Segment {
    output: usize,
    origin: usize,
    len: usize,
}

/// Where each authored byte of a document's current body came from.
///
/// The map is only valid for the exact text it was built for; [`describes`]
/// guards every use, so a body rewritten by an untracked path simply stops
/// projecting rather than projecting wrongly.
///
/// [`describes`]: Self::describes
#[derive(Debug, Clone)]
pub(crate) struct BodyOrigin {
    /// Sorted by `output`, non-overlapping.
    segments: Vec<Segment>,
    text_len: usize,
    text_hash: u64,
}

impl BodyOrigin {
    /// Maps `markdown`'s current body onto the text it was loaded from.
    ///
    /// Parsing rejoins a frontmatter-bearing document's lines with `\n`, so a
    /// `\r` before a `\n` in the loaded text is skipped. The alignment stops at
    /// the first other difference; bytes past it have no origin.
    ///
    /// ## Returns
    ///
    /// `None` when the document was not parsed from text.
    pub(crate) fn capture(markdown: &Markdown) -> Option<Self> {
        let loaded: &str = &markdown.loaded_source()?.text;
        let body_start = match crate::markdown::extract_frontmatter_block(loaded) {
            Ok(Some(block)) => block.body_span.start,
            Ok(None) => 0,
            Err(_) => return None,
        };
        let body = markdown.content();
        let authored = &loaded.as_bytes()[body_start..];
        let mut segments = Vec::new();
        let (mut output, mut origin, mut run_start) = (0, 0, 0);
        while output < body.len() && origin < authored.len() {
            if body.as_bytes()[output] == authored[origin] {
                output += 1;
                origin += 1;
            } else if authored[origin] == b'\r' && authored.get(origin + 1) == Some(&b'\n') {
                push_segment(&mut segments, run_start, body_start + origin - (output - run_start), output - run_start);
                origin += 1;
                run_start = output;
            } else {
                break;
            }
        }
        push_segment(&mut segments, run_start, body_start + origin - (output - run_start), output - run_start);
        Some(Self {
            segments,
            text_len: body.len(),
            text_hash: hash_text(body),
        })
    }

    /// Whether this map was built for exactly `text`.
    pub(crate) fn describes(&self, text: &str) -> bool {
        self.text_len == text.len() && self.text_hash == hash_text(text)
    }

    /// The map for `output`, the text a stage produced by applying `edits`
    /// (sorted and non-overlapping) to the text this map describes.
    pub(crate) fn after_edits(&self, edits: &[TextEdit], output: &str) -> Self {
        // Every kept run of the input lies after the previous one, so one
        // forward pass over the segments serves them all.
        let mut segments = Vec::new();
        let mut next = 0;
        let mut copy = |from: Range<usize>, to: usize, segments: &mut Vec<Segment>| {
            while let Some(segment) = self.segments.get(next) {
                let start = from.start.max(segment.output);
                let end = from.end.min(segment.output + segment.len);
                if start < end {
                    push_segment(segments, to + (start - from.start), segment.origin + (start - segment.output), end - start);
                }
                if segment.output + segment.len > from.end {
                    break;
                }
                next += 1;
            }
        };
        let (mut input, mut written) = (0, 0);
        for edit in edits {
            copy(input..edit.range.start, written, &mut segments);
            written += edit.range.start - input + edit.replacement_len;
            input = edit.range.end;
        }
        copy(input..self.text_len, written, &mut segments);
        Self {
            segments,
            text_len: output.len(),
            text_hash: hash_text(output),
        }
    }

    /// The loaded-document range `range` of the current text was copied from,
    /// when all of it was.
    pub(crate) fn project(&self, range: &Range<usize>) -> Option<Range<usize>> {
        let index = self.segments.partition_point(|segment| segment.output + segment.len <= range.start);
        let segment = self.segments.get(index)?;
        (segment.output <= range.start && range.end <= segment.output + segment.len).then(|| {
            let start = segment.origin + (range.start - segment.output);
            start..start + range.len()
        })
    }
}

/// Carries `origin` past a stage that rewrote `before` into `after` by
/// applying `edits`; an origin that does not describe `before` is dropped.
pub(crate) fn advance(origin: &mut Option<BodyOrigin>, before: &str, edits: &[TextEdit], after: &str) {
    *origin = origin
        .take()
        .filter(|origin| origin.describes(before))
        .map(|origin| origin.after_edits(edits, after));
}

/// Appends a segment, merging it into the previous one when both the output
/// and the origin continue it.
fn push_segment(segments: &mut Vec<Segment>, output: usize, origin: usize, len: usize) {
    if len == 0 {
        return;
    }
    if let Some(last) = segments.last_mut()
        && last.output + last.len == output
        && last.origin + last.len == origin
    {
        last.len += len;
        return;
    }
    segments.push(Segment { output, origin, len });
}

fn hash_text(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin_of(loaded: &str) -> (Markdown, BodyOrigin) {
        let markdown = Markdown::from(loaded);
        let origin = BodyOrigin::capture(&markdown).expect("parsed from text");
        (markdown, origin)
    }

    fn span_of(text: &str, needle: &str) -> Range<usize> {
        let start = text.find(needle).expect("needle present");
        start..start + needle.len()
    }

    #[test]
    fn an_unedited_body_projects_past_the_frontmatter() {
        let loaded = "---\na: 1\n---\nintro\n\nx={{ ctx.os }}\n";
        let (markdown, origin) = origin_of(loaded);
        let span = span_of(markdown.content(), "{{ ctx.os }}");
        assert_eq!(origin.project(&span), Some(span_of(loaded, "{{ ctx.os }}")));
    }

    #[test]
    fn a_crlf_body_skips_each_carriage_return() {
        let loaded = "---\r\na: 1\r\n---\r\none\r\ntwo {{ x }}\r\n";
        let (markdown, origin) = origin_of(loaded);
        let span = span_of(markdown.content(), "{{ x }}");
        assert_eq!(origin.project(&span), Some(span_of(loaded, "{{ x }}")));
    }

    /// A removed region before the expression shifts it; the edit record
    /// still projects it to its authored bytes.
    #[test]
    fn an_edit_before_the_expression_shifts_the_projection() {
        let loaded = "keep\nDROP\n{{ x }}\n";
        let (markdown, origin) = origin_of(loaded);
        let drop = span_of(markdown.content(), "DROP\n");
        let output = "keep\n{{ x }}\n";
        let edited = origin.after_edits(&[TextEdit { range: drop, replacement_len: 0 }], output);
        assert!(edited.describes(output));
        assert_eq!(edited.project(&span_of(output, "{{ x }}")), Some(span_of(loaded, "{{ x }}")));
    }

    /// Text a stage wrote has no origin, even when it spells an expression.
    #[test]
    fn inserted_text_has_no_origin() {
        let loaded = "NAME and {{ y }}\n";
        let (markdown, origin) = origin_of(loaded);
        let output = "{{ x }} and {{ y }}\n";
        let edited = origin.after_edits(
            &[TextEdit { range: span_of(markdown.content(), "NAME"), replacement_len: "{{ x }}".len() }],
            output,
        );
        assert_eq!(edited.project(&span_of(output, "{{ x }}")), None);
        assert_eq!(edited.project(&span_of(output, "{{ y }}")), Some(span_of(loaded, "{{ y }}")));
    }

    /// A span straddling authored and inserted text is not authored.
    #[test]
    fn a_span_straddling_an_edit_does_not_project() {
        let loaded = "{{ a NAME }}\n";
        let (markdown, origin) = origin_of(loaded);
        let output = "{{ a b }}\n";
        let edited = origin.after_edits(
            &[TextEdit { range: span_of(markdown.content(), "NAME"), replacement_len: 1 }],
            output,
        );
        assert_eq!(edited.project(&span_of(output, "{{ a b }}")), None);
    }

    #[test]
    fn a_map_describes_only_its_own_text() {
        let (markdown, origin) = origin_of("body\n");
        assert!(origin.describes(markdown.content()));
        assert!(!origin.describes("bodx\n"));
    }
}
