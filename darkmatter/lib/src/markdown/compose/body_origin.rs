//! Where each byte of a partially composed body came from.
//!
//! Three maps ride beside the body while the stages that scan it for
//! instructions run:
//!
//! - [`BodyOrigin`] maps the current text back to the loaded document, so a
//!   failing expression can be anchored to its authored span. It is advisory:
//!   when it stops describing the text it is dropped.
//! - [`AuthoredBody`] maps it back to the body before any stage rewrote it, so
//!   a shell command's placeholders can be read as authored. Advisory too.
//! - [`DataRanges`] records which bytes are **data**: text an expression, a
//!   shell command, a literal escape, or a data value inserted. No stage treats
//!   data as an instruction. It is not advisory: a stage that finds it no
//!   longer describes the text fails rather than treat data as authored.
//!
//! Every stage that rewrites the body reports the [`TextEdit`]s it applied,
//! and [`BodyProvenance::advance`] carries the maps past them. Provenance
//! ends after the transclusion directive parse, the last stage that scans the
//! body for instructions; later stages rewrite whole text and scan nothing.

use std::borrow::Cow;
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

/// A run of the text a structural expansion wrote that was copied from the
/// text it read.
///
/// Unlike a [`TextEdit`], splices may read the same input range more than
/// once, because repeating a body writes its authored span once per item.
/// Splices are listed in output order and tile the output exactly: text an
/// expansion writes is always a copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Splice {
    /// The run of the input that was copied.
    pub input: Range<usize>,
    /// Where the copy starts in the output.
    pub output: usize,
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

    /// The map of `text` onto itself.
    fn identity(text: &str) -> Self {
        let mut segments = Vec::new();
        push_segment(&mut segments, 0, 0, text.len());
        Self {
            segments,
            text_len: text.len(),
            text_hash: hash_text(text),
        }
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

    /// The map for `output`, the text an expansion produced by writing
    /// `splices` of the text this map describes.
    pub(crate) fn after_splices(&self, splices: &[Splice], output: &str) -> Self {
        let mut segments = Vec::new();
        for splice in splices {
            let first = self.segments.partition_point(|segment| segment.output + segment.len <= splice.input.start);
            for segment in &self.segments[first..] {
                if segment.output >= splice.input.end {
                    break;
                }
                let start = splice.input.start.max(segment.output);
                let end = splice.input.end.min(segment.output + segment.len);
                if start < end {
                    push_segment(
                        &mut segments,
                        splice.output + (start - splice.input.start),
                        segment.origin + (start - segment.output),
                        end - start,
                    );
                }
            }
        }
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

    /// The map for `output`, the text an expansion produced by writing
    /// `splices` of the text this map describes: each copy keeps the data of
    /// the range it copied.
    pub(crate) fn after_splices(&self, splices: &[Splice], output: &str) -> Self {
        let mut ranges = Vec::new();
        for splice in splices {
            let first = self.ranges.partition_point(|data| data.end <= splice.input.start);
            for data in &self.ranges[first..] {
                if data.start >= splice.input.end {
                    break;
                }
                let start = splice.input.start.max(data.start);
                let end = splice.input.end.min(data.end);
                if start < end {
                    let at = splice.output + (start - splice.input.start);
                    ranges.push(at..at + (end - start));
                }
            }
        }
        Self::covering(output, ranges)
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

/// The body as it was before any stage rewrote it, with a map from the
/// current body back into it.
///
/// A shell command's placeholders are read from this text: the stages before
/// shell expansion interpolate them into the current body, where a value's
/// quotes, spaces, or operators would read as command syntax. Advisory like
/// [`BodyOrigin`]: once it stops describing the current text, no authored
/// command can be recovered and a shell site runs only as resolved bytes.
#[derive(Debug, Clone)]
pub(crate) struct AuthoredBody {
    text: std::sync::Arc<str>,
    map: BodyOrigin,
}

impl AuthoredBody {
    fn capture(text: &str) -> Self {
        Self {
            text: text.into(),
            map: BodyOrigin::identity(text),
        }
    }

    /// The body before any stage rewrote it.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// The range of [`text`](Self::text) that `range` of `current` was copied
    /// from, when all of it was and this map describes `current`.
    pub(crate) fn project(&self, current: &str, range: &Range<usize>) -> Option<Range<usize>> {
        self.map.describes(current).then(|| self.map.project(range)).flatten()
    }

    /// Carries the map past an expansion that wrote `splices` of `before`; a
    /// map that does not describe `before` is dropped.
    pub(crate) fn spliced(self, before: &str, splices: &[Splice], after: &str) -> Option<Self> {
        self.map.describes(before).then(|| Self {
            map: self.map.after_splices(splices, after),
            text: self.text,
        })
    }

    /// Carries the map past a stage; a map that does not describe `before`
    /// is dropped.
    pub(crate) fn advanced(self, before: &str, edits: &[TextEdit], after: &str) -> Option<Self> {
        self.map.describes(before).then(|| Self {
            map: self.map.after_edits(edits, after),
            text: self.text,
        })
    }
}

/// Identity of one collection expansion that wrote a run of the body.
///
/// A nested expansion's bytes are charged to every enclosing owner exactly
/// once, so a run records its owner chain outermost first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ExpansionOwner(pub(crate) u64);

/// A run of the current body with the lexical scope and iteration it was
/// copied for.
///
/// Repeating a body range copies the same authored span once per item, so
/// ordinary [`TextEdit`]s (which advance through source runs only once)
/// cannot describe it; each copy carries its own `ScopeRange`.
#[derive(Debug, Clone)]
pub(crate) struct ScopeRange {
    /// The run of the current body.
    pub(crate) range: Range<usize>,
    /// The frame sites in this run read locals from.
    pub(crate) scope: std::sync::Arc<super::expression::LexicalScope>,
    /// The original item positions that produced this run.
    pub(crate) iteration: super::expression::iteration::IterationPath,
    /// The enclosing expansions that own this run's bytes, outermost first.
    pub(crate) owners: Vec<ExpansionOwner>,
    /// The index of the range this one is nested in.
    parent: Option<usize>,
}

/// The [`ScopeRange`]s of a body, in body order.
///
/// Ranges are listed by start with each enclosing range before the ranges
/// inside it, so the innermost range holding an offset is found by a binary
/// search followed by a walk up the enclosing chain.
#[derive(Debug, Clone, Default)]
pub(crate) struct ScopeRanges(Vec<ScopeRange>);

impl ScopeRanges {
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn get(&self, index: usize) -> Option<&ScopeRange> {
        self.0.get(index)
    }

    /// The index of the innermost range containing `offset`, which names the
    /// scope a site at `offset` executes in.
    pub(crate) fn index_at(&self, offset: usize) -> Option<usize> {
        let after = self.0.partition_point(|run| run.range.start <= offset);
        let mut candidate = after.checked_sub(1)?;
        loop {
            let run = &self.0[candidate];
            if run.range.contains(&offset) {
                return Some(candidate);
            }
            // A range that holds `offset` and starts no later than this one
            // must enclose it, so only its ancestors can still match.
            candidate = run.parent?;
        }
    }

    /// The innermost range containing `offset`.
    pub(crate) fn at(&self, offset: usize) -> Option<&ScopeRange> {
        self.index_at(offset).map(|index| &self.0[index])
    }

    /// Charges `bytes` inserted at `offset` once to every collection
    /// expansion that owns the copy there; text outside every copy is free.
    pub(crate) fn charge(
        &self,
        budget: &super::expression::iteration::IterationBudget,
        offset: usize,
        bytes: usize,
    ) -> Result<(), super::expression::iteration::IterationError> {
        let Some(range) = self.at(offset) else {
            return Ok(());
        };
        match (bytes as u64).checked_mul(range.owners.len() as u64) {
            Some(0) => Ok(()),
            Some(charge) => budget.reserve_bytes(charge, &range.iteration),
            None => budget.reserve_bytes(u64::MAX, &range.iteration),
        }
    }

    /// Starts a range at the writer's current offset; `close` ends it.
    pub(crate) fn open(
        &mut self,
        start: usize,
        scope: std::sync::Arc<super::expression::LexicalScope>,
        iteration: super::expression::iteration::IterationPath,
        owners: Vec<ExpansionOwner>,
        parent: Option<usize>,
    ) -> usize {
        self.0.push(ScopeRange { range: start..start, scope, iteration, owners, parent });
        self.0.len() - 1
    }

    /// Ends the range opened as `index` at `end`.
    pub(crate) fn close(&mut self, index: usize, end: usize) {
        self.0[index].range.end = end;
    }

    /// These ranges carried past a stage that applied `edits` (sorted and
    /// non-overlapping) to the text they describe.
    pub(crate) fn after_edits(&self, edits: &[TextEdit]) -> Self {
        if edits.is_empty() || self.0.is_empty() {
            return self.clone();
        }
        Self(
            self.0
                .iter()
                .map(|run| ScopeRange {
                    range: map_offset(edits, run.range.start, false)..map_offset(edits, run.range.end, true),
                    ..run.clone()
                })
                .collect(),
        )
    }
}

/// `offset` of the text `edits` were applied to, as an offset of the result.
/// An offset inside a replaced range maps to the replacement's start, or to
/// its end for `inclusive_end`, so a range that held the replacement still
/// holds it.
fn map_offset(edits: &[TextEdit], offset: usize, inclusive_end: bool) -> usize {
    let mut shift: isize = 0;
    for edit in edits {
        if edit.range.end <= offset {
            shift += edit.replacement_len as isize - edit.range.len() as isize;
        } else if edit.range.start < offset {
            let start = (edit.range.start as isize + shift) as usize;
            return if inclusive_end { start + edit.replacement_len } else { start };
        } else {
            break;
        }
    }
    (offset as isize + shift) as usize
}

/// Builds the text of a structural expansion and the provenance of every
/// byte it writes.
///
/// Everything the writer appends is a copy of authored text, so the result is
/// described by [`Splice`]s plus the [`ScopeRanges`] of the repeated bodies.
/// Every append is charged to the request's byte allowance before it is
/// written: once for each collection expansion that encloses it, so output a
/// nested expansion produced is charged again when it lands in the enclosing
/// one.
pub(crate) struct ExpansionWriter<'a> {
    output: String,
    splices: Vec<Splice>,
    scopes: ScopeRanges,
    budget: &'a super::expression::iteration::IterationBudget,
    owners: Vec<ExpansionOwner>,
    open_ranges: Vec<usize>,
    next_owner: u64,
}

impl<'a> ExpansionWriter<'a> {
    pub(crate) fn new(content_len: usize, budget: &'a super::expression::iteration::IterationBudget) -> Self {
        Self {
            output: String::with_capacity(content_len),
            splices: Vec::new(),
            scopes: ScopeRanges::default(),
            budget,
            owners: Vec::new(),
            open_ranges: Vec::new(),
            next_owner: 0,
        }
    }

    /// Begins one collection invocation's expansion; bytes written until the
    /// matching [`close_expansion`](Self::close_expansion) are charged to it.
    pub(crate) fn open_expansion(&mut self) {
        self.next_owner += 1;
        self.owners.push(ExpansionOwner(self.next_owner));
    }

    pub(crate) fn close_expansion(&mut self) {
        self.owners.pop();
    }

    /// Appends `content[input]`, failing before the write when an enclosing
    /// expansion's byte allowance cannot cover it.
    pub(crate) fn copy(
        &mut self,
        content: &str,
        input: Range<usize>,
        path: &super::expression::iteration::IterationPath,
    ) -> Result<(), super::expression::iteration::IterationError> {
        if input.is_empty() {
            return Ok(());
        }
        let charge = (input.len() as u64).checked_mul(self.owners.len() as u64);
        match charge {
            Some(0) => {}
            Some(charge) => self.budget.reserve_bytes(charge, path)?,
            None => self.budget.reserve_bytes(u64::MAX, path)?,
        }
        self.splices.push(Splice { input: input.clone(), output: self.output.len() });
        self.output.push_str(&content[input]);
        Ok(())
    }

    /// Opens the range one repeated body copy occupies.
    pub(crate) fn open_iteration(
        &mut self,
        scope: std::sync::Arc<super::expression::LexicalScope>,
        iteration: super::expression::iteration::IterationPath,
    ) {
        let parent = self.open_ranges.last().copied();
        let index = self.scopes.open(self.output.len(), scope, iteration, self.owners.clone(), parent);
        self.open_ranges.push(index);
    }

    /// Closes the range [`open_iteration`](Self::open_iteration) opened last.
    pub(crate) fn close_iteration(&mut self) {
        if let Some(index) = self.open_ranges.pop() {
            self.scopes.close(index, self.output.len());
        }
    }

    pub(crate) fn finish(self) -> (String, Vec<Splice>, ScopeRanges) {
        (self.output, self.splices, self.scopes)
    }
}

/// The provenance maps of a body between the stages that scan it.
#[derive(Debug, Clone)]
pub(crate) struct BodyProvenance {
    pub(crate) origin: Option<BodyOrigin>,
    pub(crate) data: DataRanges,
    pub(crate) authored: Option<AuthoredBody>,
    pub(crate) scopes: ScopeRanges,
}

impl BodyProvenance {
    /// The provenance of `markdown`'s body before any stage rewrote it: all of
    /// it is authored.
    pub(crate) fn capture(markdown: &Markdown) -> Self {
        Self {
            origin: BodyOrigin::capture(markdown),
            data: DataRanges::none_for(markdown.content()),
            authored: Some(AuthoredBody::capture(markdown.content())),
            scopes: ScopeRanges::default(),
        }
    }

    /// Carries every map past a stage that rewrote `before` into `after` by
    /// applying `edits`.
    ///
    /// ## Errors
    ///
    /// Fails when the data map does not describe `before`.
    pub(crate) fn advance(&mut self, before: &str, edits: &[TextEdit], after: &str) -> MarkdownResult<()> {
        self.data.ensure_describes(before)?;
        self.scopes = self.scopes.after_edits(edits);
        self.data = self.data.after_edits(edits, after);
        advance(&mut self.origin, before, edits, after);
        self.authored = self.authored.take().and_then(|authored| authored.advanced(before, edits, after));
        Ok(())
    }

    /// Carries every map past a structural expansion that wrote `splices` of
    /// `before` as `after`, and records the scope of each repeated body.
    ///
    /// ## Errors
    ///
    /// Fails when the data map does not describe `before`.
    pub(crate) fn expand(
        &mut self,
        before: &str,
        splices: &[Splice],
        after: &str,
        scopes: ScopeRanges,
    ) -> MarkdownResult<()> {
        self.data.ensure_describes(before)?;
        debug_assert!(self.scopes.is_empty(), "a body is expanded before any scope is recorded");
        self.data = self.data.after_splices(splices, after);
        self.origin = self
            .origin
            .take()
            .filter(|origin| origin.describes(before))
            .map(|origin| origin.after_splices(splices, after));
        self.authored = self.authored.take().and_then(|authored| authored.spliced(before, splices, after));
        self.scopes = scopes;
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
    biscuit_hash::xx_hash(text)
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

    mod expansion {
        use super::*;
        use crate::markdown::compose::expression::LexicalScope;
        use crate::markdown::compose::expression::iteration::{IterationBudget, IterationLimits, IterationPath};
        use std::num::NonZeroU64;

        fn scope() -> std::sync::Arc<LexicalScope> {
            LexicalScope::new(vec![("i".to_string(), serde_json::json!(1))])
        }

        #[test]
        fn a_repeated_range_projects_to_the_same_authored_bytes_each_time() {
            let loaded = "head\nbody {{ x }}\ntail\n";
            let (markdown, origin) = origin_of(loaded);
            let content = markdown.content();
            let body = span_of(content, "body {{ x }}\n");
            let budget = IterationBudget::new(IterationLimits::default());
            let mut writer = ExpansionWriter::new(content.len(), &budget);
            let path = IterationPath::empty();
            writer.copy(content, 0..body.start, &path).unwrap();
            for _ in 0..2 {
                writer.copy(content, body.clone(), &path).unwrap();
            }
            writer.copy(content, body.end..content.len(), &path).unwrap();
            let (output, splices, _) = writer.finish();
            assert_eq!(output, "head\nbody {{ x }}\nbody {{ x }}\ntail\n");
            let carried = origin.after_splices(&splices, &output);
            let authored = span_of(loaded, "{{ x }}");
            let first = span_of(&output, "{{ x }}");
            let second = first.start + body.len()..first.end + body.len();
            assert_eq!(carried.project(&first), Some(authored.clone()));
            assert_eq!(carried.project(&second), Some(authored));
        }

        #[test]
        fn data_inside_a_repeated_range_is_data_in_every_copy() {
            let text = "ab\ncd\n";
            let data = DataRanges::covering(text, std::iter::once(3..5).collect());
            let splices = vec![
                Splice { input: 3..6, output: 0 },
                Splice { input: 3..6, output: 3 },
            ];
            let output = "cd\ncd\n";
            let carried = data.after_splices(&splices, output);
            assert!(carried.intersects(&(0..2)) && carried.intersects(&(3..5)));
            assert!(!carried.intersects(&(2..3)) && !carried.intersects(&(5..6)));
        }

        #[test]
        fn nested_ranges_resolve_to_the_innermost_copy() {
            let budget = IterationBudget::new(IterationLimits::default());
            let mut writer = ExpansionWriter::new(0, &budget);
            let path = IterationPath::empty();
            writer.open_expansion();
            writer.open_iteration(scope(), path.clone());
            writer.copy("abcdefgh", 0..2, &path).unwrap();
            writer.open_iteration(scope(), path.clone());
            writer.copy("abcdefgh", 2..4, &path).unwrap();
            writer.close_iteration();
            writer.copy("abcdefgh", 4..5, &path).unwrap();
            writer.close_iteration();
            writer.close_expansion();
            let (_, _, ranges) = writer.finish();
            assert_eq!(ranges.at(0).unwrap().range, 0..5);
            assert_eq!(ranges.at(3).unwrap().range, 2..4);
            assert_eq!(ranges.at(4).unwrap().range, 0..5, "after the inner copy the outer one holds again");
            assert!(ranges.at(5).is_none());
            assert_eq!(ranges.at(3).unwrap().owners.len(), 1);
        }

        #[test]
        fn ranges_follow_the_edits_of_later_stages() {
            let budget = IterationBudget::new(IterationLimits::default());
            let mut writer = ExpansionWriter::new(0, &budget);
            let path = IterationPath::empty();
            writer.open_expansion();
            writer.open_iteration(scope(), path.clone());
            writer.copy("aa{{x}}bb", 0..9, &path).unwrap();
            writer.close_iteration();
            let (_, _, ranges) = writer.finish();
            let carried = ranges.after_edits(&[TextEdit::data(2..7, 1)]);
            assert_eq!(carried.at(0).unwrap().range, 0..5, "the replacement shrinks the copy");
        }

        #[test]
        fn copies_are_charged_once_per_enclosing_expansion_and_fail_before_the_write() {
            let budget =
                IterationBudget::new(IterationLimits::new(NonZeroU64::new(9).unwrap(), NonZeroU64::new(8).unwrap()));
            let path = IterationPath::empty();
            let mut writer = ExpansionWriter::new(0, &budget);
            writer.copy("xxxx", 0..4, &path).unwrap();
            assert_eq!(budget.usage().bytes, 0, "outside every expansion is free");
            writer.open_expansion();
            writer.open_expansion();
            writer.copy("xxxx", 0..4, &path).unwrap();
            assert_eq!(budget.usage().bytes, 8);
            assert!(writer.copy("x", 0..1, &path).is_err());
            assert_eq!(writer.finish().0.len(), 8, "the failing copy wrote nothing");
        }
    }
}
