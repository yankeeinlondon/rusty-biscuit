//! Where each byte of a partially composed body came from.
//!
//! Two maps ride beside the body while the stages that scan it for
//! instructions run:
//!
//! - [`BodyOrigin`] maps the current text back to the loaded document, so a
//!   failing expression can be anchored to its authored span. It is advisory:
//!   when it stops describing the text it is dropped.
//! - [`DataRanges`] records which bytes are **data**: text an expression, a
//!   shell command, a literal escape, or a data value inserted. No stage treats
//!   data as an instruction. It is not advisory: a stage that finds it no
//!   longer describes the text fails rather than treat data as authored.
//!
//! Every stage that rewrites the body reports the [`TextEdit`]s it applied,
//! and [`BodyProvenance::advance`] carries both maps past them. Provenance
//! ends after the transclusion directive parse, the last stage that scans the
//! body for instructions; later stages rewrite whole text and scan nothing.

use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::ops::Range;

use crate::markdown::Markdown;
use crate::markdown::types::{MarkdownError, MarkdownResult};

/// Whether the text an edit wrote is data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditOrigin {
    /// The stage copied authored text (an authored replacement value, or a
    /// deletion).
    Authored,
    /// The stage wrote produced text: an expression result, shell output, a
    /// literal escape, or a data value.
    Data,
    /// The written text is data when the replaced range held any data.
    Inherit,
}

/// One replacement a stage applied to its input text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextEdit {
    /// The replaced byte range of the stage's input.
    pub range: Range<usize>,
    /// Byte length of the text written in its place.
    pub replacement_len: usize,
    /// Whether the written text is data.
    pub origin: EditOrigin,
}

impl TextEdit {
    /// An edit that writes produced text.
    pub(crate) fn data(range: Range<usize>, replacement_len: usize) -> Self {
        Self { range, replacement_len, origin: EditOrigin::Data }
    }

    /// An edit that writes authored text, or deletes.
    pub(crate) fn authored(range: Range<usize>, replacement_len: usize) -> Self {
        Self { range, replacement_len, origin: EditOrigin::Authored }
    }
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

/// The byte ranges of a body that are data.
///
/// Valid only for the exact text it was built for; [`ensure_describes`]
/// guards every scanner that reads it, and a mismatch is an error, never a
/// fallback to "all authored".
///
/// [`ensure_describes`]: Self::ensure_describes
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DataRanges {
    /// Sorted, non-overlapping, non-adjacent, non-empty.
    ranges: Vec<Range<usize>>,
    text_len: usize,
    text_hash: u64,
}

impl DataRanges {
    /// A body with no data.
    pub(crate) fn none_for(text: &str) -> Self {
        Self::covering(text, Vec::new())
    }

    /// A body whose data lies in `ranges` (sorted and non-overlapping).
    pub(crate) fn covering(text: &str, ranges: Vec<Range<usize>>) -> Self {
        let mut merged: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
        for range in ranges.into_iter().filter(|range| !range.is_empty()) {
            match merged.last_mut() {
                Some(last) if last.end >= range.start => last.end = last.end.max(range.end),
                _ => merged.push(range),
            }
        }
        Self {
            ranges: merged,
            text_len: text.len(),
            text_hash: hash_text(text),
        }
    }

    /// Whether this map was built for exactly `text`.
    pub(crate) fn describes(&self, text: &str) -> bool {
        self.text_len == text.len() && self.text_hash == hash_text(text)
    }

    /// Fails when this map was not built for `text`: some stage rewrote the
    /// body without reporting its edits, so which bytes are data is unknown.
    pub(crate) fn ensure_describes(&self, text: &str) -> MarkdownResult<()> {
        if self.describes(text) {
            Ok(())
        } else {
            Err(MarkdownError::Transform(
                "internal error: the body's data ranges no longer describe its text, so \
                 Darkmatter cannot tell inserted data from authored instructions"
                    .to_string(),
            ))
        }
    }

    /// Whether any byte of `range` is data. An empty `range` intersects data
    /// when it sits strictly inside a data range.
    pub(crate) fn intersects(&self, range: &Range<usize>) -> bool {
        let index = self.ranges.partition_point(|data| data.end <= range.start);
        self.ranges.get(index).is_some_and(|data| {
            if range.is_empty() {
                data.start < range.start
            } else {
                data.start < range.end
            }
        })
    }

    /// `text` with every data byte masked: ASCII whitespace becomes a space
    /// and every other byte becomes `x`. Offsets are unchanged and the result
    /// is valid UTF-8 (whole characters are replaced). A line break in data
    /// becomes a space, so data can neither start a line nor contain a
    /// directive, a fence, or an expression in the masked view.
    pub(crate) fn masked<'a>(&self, text: &'a str) -> Cow<'a, str> {
        if self.ranges.is_empty() {
            return Cow::Borrowed(text);
        }
        let mut bytes = text.as_bytes().to_vec();
        for range in &self.ranges {
            for byte in &mut bytes[range.clone()] {
                *byte = if byte.is_ascii_whitespace() { b' ' } else { b'x' };
            }
        }
        Cow::Owned(String::from_utf8(bytes).expect("masking replaces whole characters"))
    }

    /// The map for `output`, the text a stage produced by applying `edits`
    /// (sorted and non-overlapping) to the text this map describes.
    pub(crate) fn after_edits(&self, edits: &[TextEdit], output: &str) -> Self {
        let mut ranges = Vec::new();
        let mut next = 0;
        // Copies the data inside the kept input run `from` to output offset `to`.
        let mut copy = |from: Range<usize>, to: usize, ranges: &mut Vec<Range<usize>>| {
            while let Some(data) = self.ranges.get(next) {
                let start = from.start.max(data.start);
                let end = from.end.min(data.end);
                if start < end {
                    ranges.push(to + (start - from.start)..to + (end - from.start));
                }
                if data.end > from.end {
                    break;
                }
                next += 1;
            }
        };
        let (mut input, mut written) = (0, 0);
        for edit in edits {
            copy(input..edit.range.start, written, &mut ranges);
            written += edit.range.start - input;
            let is_data = match edit.origin {
                EditOrigin::Authored => false,
                EditOrigin::Data => true,
                EditOrigin::Inherit => self.intersects(&edit.range),
            };
            if is_data {
                ranges.push(written..written + edit.replacement_len);
            }
            written += edit.replacement_len;
            input = edit.range.end;
        }
        copy(input..self.text_len, written, &mut ranges);
        Self::covering(output, ranges)
    }
}

/// Both provenance maps of a body between the stages that scan it.
#[derive(Debug, Clone)]
pub(crate) struct BodyProvenance {
    pub(crate) origin: Option<BodyOrigin>,
    pub(crate) data: DataRanges,
}

impl BodyProvenance {
    /// The provenance of `markdown`'s body before any stage rewrote it: all of
    /// it is authored.
    pub(crate) fn capture(markdown: &Markdown) -> Self {
        Self {
            origin: BodyOrigin::capture(markdown),
            data: DataRanges::none_for(markdown.content()),
        }
    }

    /// Carries both maps past a stage that rewrote `before` into `after` by
    /// applying `edits`.
    ///
    /// ## Errors
    ///
    /// Fails when the data map does not describe `before`.
    pub(crate) fn advance(&mut self, before: &str, edits: &[TextEdit], after: &str) -> MarkdownResult<()> {
        self.data.ensure_describes(before)?;
        self.data = self.data.after_edits(edits, after);
        advance(&mut self.origin, before, edits, after);
        Ok(())
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

/// Applies `(range, text)` replacements (non-overlapping, any order) to
/// `content`, returning the [`TextEdit`]s they made with `origin`, in order.
///
/// A replacement that swaps a whole directive line, terminator included, for
/// text that also ends in a line break keeps that line break as authored text:
/// it ends the replaced line rather than being produced, so the next authored
/// line still starts an authored line.
pub(crate) fn apply_replacements_with_edits(
    content: &mut String,
    mut replacements: Vec<(Range<usize>, String)>,
    origin: EditOrigin,
) -> Vec<TextEdit> {
    replacements.sort_by_key(|(range, _)| range.start);
    let mut edits = Vec::with_capacity(replacements.len());
    for (range, text) in &replacements {
        let keeps_terminator = origin != EditOrigin::Authored
            && text.ends_with('\n')
            && content[range.clone()].ends_with('\n');
        if keeps_terminator {
            edits.push(TextEdit { range: range.start..range.end - 1, replacement_len: text.len() - 1, origin });
            edits.push(TextEdit::authored(range.end - 1..range.end, 1));
        } else {
            edits.push(TextEdit { range: range.clone(), replacement_len: text.len(), origin });
        }
    }
    for (range, text) in replacements.into_iter().rev() {
        content.replace_range(range, &text);
    }
    edits
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
        let edited = origin.after_edits(&[TextEdit::authored(drop, 0)], output);
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
            &[TextEdit::data(span_of(markdown.content(), "NAME"), "{{ x }}".len())],
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
            &[TextEdit::data(span_of(markdown.content(), "NAME"), 1)],
            output,
        );
        assert_eq!(edited.project(&span_of(output, "{{ a b }}")), None);
    }

    mod data_ranges {
        use super::*;
        use proptest::prelude::*;

        /// Per-byte reference model: `true` marks a data byte.
        fn apply_model(model: &[bool], edits: &[TextEdit]) -> Vec<bool> {
            let mut out = Vec::new();
            let mut input = 0;
            for edit in edits {
                out.extend_from_slice(&model[input..edit.range.start]);
                let is_data = match edit.origin {
                    EditOrigin::Authored => false,
                    EditOrigin::Data => true,
                    EditOrigin::Inherit => {
                        model[edit.range.clone()].iter().any(|b| *b)
                            || (edit.range.is_empty()
                                && edit.range.start > 0
                                && edit.range.start < model.len()
                                && model[edit.range.start - 1]
                                && model[edit.range.start])
                    }
                };
                out.extend(std::iter::repeat_n(is_data, edit.replacement_len));
                input = edit.range.end;
            }
            out.extend_from_slice(&model[input..]);
            out
        }

        fn model_of(data: &DataRanges, len: usize) -> Vec<bool> {
            (0..len).map(|at| data.intersects(&(at..at + 1))).collect()
        }

        proptest! {
            /// Carrying ranges through edits agrees with the per-byte model.
            #[test]
            fn after_edits_matches_a_per_byte_model(
                text in "[a-z\n]{0,24}",
                marks in proptest::collection::vec(any::<bool>(), 24),
                raw_edits in proptest::collection::vec((0usize..24, 0usize..4, 0usize..5, 0u8..3), 0..4),
            ) {
                let len = text.len();
                let ranges: Vec<Range<usize>> = (0..len).filter(|at| marks[*at]).map(|at| at..at + 1).collect();
                let data = DataRanges::covering(&text, ranges);
                let model = model_of(&data, len);

                // Non-overlapping, sorted edits inside the text.
                let mut edits = Vec::new();
                let mut cursor = 0;
                for (start, width, replacement_len, origin) in raw_edits {
                    let start = cursor.max(start.min(len));
                    let end = (start + width).min(len);
                    if start < cursor || start > len { continue; }
                    let origin = [EditOrigin::Authored, EditOrigin::Data, EditOrigin::Inherit][origin as usize];
                    edits.push(TextEdit { range: start..end, replacement_len, origin });
                    cursor = end.max(start + 1);
                }
                let mut output = String::new();
                let mut input = 0;
                for edit in &edits {
                    output.push_str(&text[input..edit.range.start]);
                    output.push_str(&"r".repeat(edit.replacement_len));
                    input = edit.range.end;
                }
                output.push_str(&text[input..]);

                let carried = data.after_edits(&edits, &output);
                prop_assert!(carried.describes(&output));
                prop_assert_eq!(model_of(&carried, output.len()), apply_model(&model, &edits));
            }
        }

        #[test]
        fn the_masked_view_hides_data_structure_and_keeps_offsets() {
            let text = "a\n```\n::shell x é\nb";
            let data = DataRanges::covering(text, std::iter::once(1..text.len() - 2).collect());
            let masked = data.masked(text);
            assert_eq!(masked.len(), text.len());
            assert_eq!(&masked[..1], "a");
            assert!(!masked.contains("```") && !masked.contains("::shell"), "{masked}");
            assert_eq!(masked.matches('\n').count(), 1, "only the authored line break survives");
            assert!(masked.ends_with("\nb"));
        }

        #[test]
        fn a_map_for_other_text_fails_closed() {
            let data = DataRanges::none_for("body\n");
            assert!(data.ensure_describes("body\n").is_ok());
            let error = data.ensure_describes("bodx\n").unwrap_err();
            assert!(error.to_string().contains("no longer describe"), "{error}");
        }

        #[test]
        fn an_empty_range_inside_data_intersects_it() {
            let data = DataRanges::covering("abcdef", std::iter::once(1..4).collect());
            assert!(data.intersects(&(2..2)));
            assert!(!data.intersects(&(1..1)), "a boundary is not inside");
            assert!(!data.intersects(&(4..6)));
            assert!(data.intersects(&(0..2)));
        }

        /// A produced replacement of a whole line keeps the line break that
        /// ends it authored, so the next authored line still starts a line.
        #[test]
        fn a_replaced_line_keeps_its_terminator_authored() {
            let mut content = "::shell x\n::file y\n".to_string();
            let edits = apply_replacements_with_edits(
                &mut content,
                vec![(0..10, "out\n".to_string())],
                EditOrigin::Data,
            );
            assert_eq!(content, "out\n::file y\n");
            let data = DataRanges::none_for("::shell x\n::file y\n").after_edits(&edits, &content);
            assert!(data.intersects(&(0..3)));
            assert!(!data.intersects(&(3..4)), "the terminator stays authored");
        }
    }

    #[test]
    fn a_map_describes_only_its_own_text() {
        let (markdown, origin) = origin_of("body\n");
        assert!(origin.describes(markdown.content()));
        assert!(!origin.describes("bodx\n"));
    }
}
