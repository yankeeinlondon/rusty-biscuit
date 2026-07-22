//! Shared source-span vocabulary for span-aware parse products.
//!
//! Spans are byte-offset ranges into the exact source text handed to the
//! parser — no line-ending normalization is applied, so offsets remain valid
//! against the caller's original document (including CRLF content).
//!
//! [`SourceSpan`] itself is owned by `biscuit-file` (the shared span
//! vocabulary for the workspace) and re-exported here unchanged; both paths
//! name the same `Range<usize>`.

pub use biscuit_file::SourceSpan;

/// A value paired with the source span it was parsed from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    /// The parsed value.
    pub value: T,
    /// Byte span of the source text the value was parsed from.
    pub span: SourceSpan,
}

impl<T> Spanned<T> {
    /// Creates a spanned value.
    pub fn new(value: T, span: SourceSpan) -> Self {
        Self { value, span }
    }

    /// Maps the inner value, preserving the span.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            value: f(self.value),
            span: self.span,
        }
    }
}

/// Returns the 1-indexed line number containing byte `offset` in `source`.
///
/// Offsets past the end of `source` are clamped to the final line. Lines are
/// delimited by `\n`; a `\r\n` sequence therefore counts as a single line
/// break.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::span::line_of_offset;
///
/// let source = "first\nsecond\n";
/// assert_eq!(line_of_offset(source, 0), 1);
/// assert_eq!(line_of_offset(source, 6), 2);
/// ```
pub fn line_of_offset(source: &str, offset: usize) -> usize {
    let offset = offset.min(source.len());
    source[..offset].bytes().filter(|&b| b == b'\n').count() + 1
}

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

/// Returns the 1-indexed `(line, column)` for byte `offset` in `source`.
///
/// The column is a **byte** column within the line (1-indexed), not a
/// character or UTF-16 column; encoding-aware projection is the caller's
/// responsibility. LF, CRLF, and lone CR are recognized as line endings.
/// Offsets past the end of `source` are clamped.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::span::line_col_of_offset;
///
/// let source = "first\nsecond\n";
/// assert_eq!(line_col_of_offset(source, 0), (1, 1));
/// assert_eq!(line_col_of_offset(source, 8), (2, 3));
/// ```
pub fn line_col_of_offset(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let bytes = source.as_bytes();
    let mut line = 1;
    let mut line_start = 0;
    let mut index = 0;

    while index < offset {
        match bytes[index] {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                if index + 2 > offset {
                    break;
                }
                index += 2;
                line += 1;
                line_start = index;
            }
            b'\r' | b'\n' => {
                index += 1;
                line += 1;
                line_start = index;
            }
            _ => index += 1,
        }
    }

    (line, offset - line_start + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_of_offset_first_line() {
        assert_eq!(line_of_offset("hello\nworld", 0), 1);
        assert_eq!(line_of_offset("hello\nworld", 5), 1);
    }

    #[test]
    fn test_line_of_offset_after_newline() {
        assert_eq!(line_of_offset("hello\nworld", 6), 2);
        assert_eq!(line_of_offset("hello\nworld", 11), 2);
    }

    #[test]
    fn test_line_of_offset_clamps_past_end() {
        assert_eq!(line_of_offset("hello\nworld", 999), 2);
    }

    #[test]
    fn test_line_of_offset_crlf_counts_single_break() {
        let source = "a\r\nb\r\nc";
        assert_eq!(line_of_offset(source, 0), 1);
        assert_eq!(line_of_offset(source, 3), 2);
        assert_eq!(line_of_offset(source, 6), 3);
    }

    #[test]
    fn test_line_col_of_offset_basic() {
        let source = "ab\ncd\n";
        assert_eq!(line_col_of_offset(source, 0), (1, 1));
        assert_eq!(line_col_of_offset(source, 1), (1, 2));
        assert_eq!(line_col_of_offset(source, 3), (2, 1));
        assert_eq!(line_col_of_offset(source, 4), (2, 2));
    }

    #[test]
    fn test_line_col_of_offset_empty_source() {
        assert_eq!(line_col_of_offset("", 0), (1, 1));
        assert_eq!(line_col_of_offset("", 5), (1, 1));
    }

    #[test]
    fn test_line_col_of_offset_handles_crlf_and_lone_cr() {
        assert_eq!(line_col_of_offset("ab\r\ncd", 4), (2, 1));
        assert_eq!(line_col_of_offset("ab\rcd", 3), (2, 1));
        assert_eq!(line_col_of_offset("ab\rcd", 4), (2, 2));
    }

    #[test]
    fn test_line_col_of_offset_is_byte_column() {
        // 'é' is two bytes; the byte column after it is 3, not 2.
        let source = "é!";
        assert_eq!(line_col_of_offset(source, 2), (1, 3));
    }

    #[test]
    fn test_spanned_new_and_map() {
        let spanned = Spanned::new("42", 3..5);
        let mapped = spanned.map(|value| value.parse::<u32>().unwrap());
        assert_eq!(mapped, Spanned::new(42, 3..5));
    }
}
