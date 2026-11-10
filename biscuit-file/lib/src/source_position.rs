//! Byte offset → 1-based line and character column.
//!
//! Every locator in [`data_locations`](crate::data_locations), and every
//! parser error once normalized to a byte offset, is turned into a position
//! here, so all formats count lines and columns the same way:
//!
//! - LF and CRLF end a line; a CRLF's CR never occupies a column, so a CRLF
//!   text and its LF twin give the same position for the same token.
//! - A lone CR (not followed by LF) does not end a line; it is an ordinary
//!   character in the column count.
//! - Columns count `char`s (Unicode scalar values), not bytes.
//!
//! Offsets index the text given. A caller that strips a byte-order mark
//! before parsing passes the stripped text, so line 1 columns match what an
//! editor shows.

/// The 1-based `(line, column)` of `byte_offset` in `text`, column counted in
/// `char`s.
///
/// `byte_offset == text.len()` is the end position, just past the last
/// character.
///
/// ## Examples
///
/// ```rust
/// use biscuit_file::source_position;
///
/// assert_eq!(source_position("a: 1\nnamé: x", 12), Some((2, 7)));
/// assert_eq!(source_position("a\r\nb", 3), Some((2, 1)));
/// assert_eq!(source_position("a\rb", 2), Some((1, 3)));
/// assert_eq!(source_position("é", 1), None);
/// ```
///
/// ## Returns
///
/// `None` when `byte_offset` is past the end of `text` or not on a `char`
/// boundary.
#[must_use]
pub fn source_position(text: &str, byte_offset: usize) -> Option<(usize, usize)> {
    let before = text.get(..byte_offset)?;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let current_line = &before[line_start..];
    let mut column = current_line.chars().count() + 1;
    // At the LF of a CRLF, the CR belongs to the line break, not the line.
    if current_line.ends_with('\r') && text[byte_offset..].starts_with('\n') {
        column -= 1;
    }
    Some((line, column))
}
