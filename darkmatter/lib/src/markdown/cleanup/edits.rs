//! Bounded source edits.
//!
//! Cleanup never re-serializes Markdown. Each rule proposes replacements of
//! authored byte ranges, and [`apply`] splices them into the original text, so
//! every byte no rule touched (terminators, BOM-free prose, disabled regions)
//! comes through unchanged.

use std::ops::Range;

use super::protect::Protection;

/// One replacement of a source byte range. An empty range is an insertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceEdit {
    pub range: Range<usize>,
    pub replacement: String,
    /// A re-indentation that a deletion covering it makes moot (the line it
    /// re-indents is joined onto the previous line).
    pub subsumable: bool,
}

impl SourceEdit {
    pub(super) fn replace(range: Range<usize>, replacement: impl Into<String>) -> Self {
        Self { range, replacement: replacement.into(), subsumable: false }
    }

    pub(super) fn insert(at: usize, text: impl Into<String>) -> Self {
        Self::replace(at..at, text)
    }

    pub(super) fn reindent(range: Range<usize>, replacement: impl Into<String>) -> Self {
        Self { range, replacement: replacement.into(), subsumable: true }
    }
}

/// Two edits overlapped, or an edit touched a protected line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EditConflict {
    /// Source offset of the conflicting edit.
    pub offset: usize,
    pub message: String,
}

/// Applies `edits` to `source`.
///
/// Insertions at one offset keep the order they were proposed in and come
/// before a replacement starting there.
///
/// ## Errors
///
/// Returns [`EditConflict`] when two non-insertion edits overlap or an edit
/// touches a protected line; the caller declines rather than guessing.
pub(super) fn apply(source: &str, edits: Vec<SourceEdit>, protection: &Protection) -> Result<String, EditConflict> {
    let covering: Vec<Range<usize>> = edits
        .iter()
        .filter(|edit| !edit.subsumable && !edit.range.is_empty())
        .map(|edit| edit.range.clone())
        .collect();
    let mut edits: Vec<SourceEdit> = edits
        .into_iter()
        .filter(|edit| {
            !(edit.subsumable
                && covering.iter().any(|cover| cover.start <= edit.range.start && edit.range.end <= cover.end))
        })
        .filter(|edit| !edit.range.is_empty() || !edit.replacement.is_empty())
        .collect();
    edits.sort_by_key(|edit| (edit.range.start, !edit.range.is_empty(), edit.range.end));

    for pair in edits.windows(2) {
        if pair[0].range.end > pair[1].range.start {
            return Err(EditConflict {
                offset: pair[1].range.start,
                message: format!("edits at {:?} and {:?} overlap", pair[0].range, pair[1].range),
            });
        }
    }
    if let Some(edit) = edits.iter().find(|edit| protection.touches_line(&edit.range)) {
        return Err(EditConflict {
            offset: edit.range.start,
            message: "an edit touched a protected directive line".to_string(),
        });
    }

    let mut output = String::with_capacity(source.len() + 64);
    let mut cursor = 0usize;
    for edit in &edits {
        output.push_str(&source[cursor..edit.range.start]);
        output.push_str(&edit.replacement);
        cursor = edit.range.end;
    }
    output.push_str(&source[cursor..]);
    Ok(output)
}
