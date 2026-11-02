//! Shared source-span vocabulary for span-aware parse products.
//!
//! [`Spanned`] and the offset helpers live in `simplified_schema::span` and
//! are re-exported here unchanged, so both paths name the same types and
//! functions. [`SourceSpan`] is `biscuit-file`'s `Range<usize>`.

pub use simplified_schema::span::{SourceSpan, Spanned, line_col_of_offset, line_of_offset};

/// Byte offsets of every `\n` in `content`, in ascending order.
///
/// Precomputed once so [`line_at_offset`] can binary-search instead of
/// rescanning a growing prefix per lookup.
pub(crate) fn newline_offset_table(content: &str) -> Vec<usize> {
    content
        .bytes()
        .enumerate()
        .filter_map(|(i, byte)| (byte == b'\n').then_some(i))
        .collect()
}

/// Returns the 1-indexed line for `offset`, byte-identical to
/// `content[..offset].lines().count() + 1`, using the precomputed
/// [`newline_offset_table`].
///
/// `str::lines` treats a trailing line terminator as *not* starting a new line,
/// so the count is the number of `\n` before `offset` plus one only when the
/// prefix is non-empty and does not end on a `\n`. This differs from a plain
/// newline count when `offset` lands mid-line (e.g. an inline link or a heading
/// nested in a blockquote), which is why the public [`line_of_offset`] is not
/// substitutable for callers that must reproduce `lines().count()`.
pub(crate) fn line_at_offset(newline_offsets: &[usize], content: &str, offset: usize) -> usize {
    let newlines = newline_offsets.partition_point(|&pos| pos < offset);
    let trailing = usize::from(offset > 0 && content.as_bytes()[offset - 1] != b'\n');
    newlines + trailing + 1
}
