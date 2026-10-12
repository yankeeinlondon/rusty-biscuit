//! List-marker widths for the standalone reflow helpers.

/// Maximum digit count for a CommonMark ordered-list marker.
///
/// See <https://spec.commonmark.org/0.31.2/#ordered-list-marker>: a run of ten
/// or more digits is paragraph prose, not a marker.
const MAX_ORDERED_MARKER_DIGITS: usize = 9;

/// Returns the byte width of the list marker starting at `trimmed`, or `None`
/// when `trimmed` does not begin with a list marker.
///
/// The width includes the trailing space and any task-list bracket:
/// `"- "` = 2, `"* "` = 2, `"+ "` = 2, `"1. "` = 3, `"10. "` = 4,
/// `"- [ ] "` = 6, `"- [x] "` = 6. Digit runs longer than
/// `MAX_ORDERED_MARKER_DIGITS` are not markers.
pub(super) fn list_marker_byte_width(trimmed: &str) -> Option<usize> {
    let marker = list_container_marker_byte_width(trimmed)?;
    Some(marker + task_marker_byte_width(&trimmed[marker..]).unwrap_or(0))
}

/// Width of the CommonMark list marker, including its one-to-four-space
/// padding and excluding a GFM task checkbox.
///
/// A task checkbox is inline item content. It contributes to the visible body
/// prefix but not to the range in which a nested child marker is valid.
fn list_container_marker_byte_width(trimmed: &str) -> Option<usize> {
    let token_width = list_container_marker_token_byte_width(trimmed)?;
    let padding_width = commonmark_marker_padding(&trimmed[token_width..])?;
    Some(token_width + padding_width)
}

/// Width of a list marker's punctuation, excluding its required padding.
fn list_container_marker_token_byte_width(trimmed: &str) -> Option<usize> {
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    if matches!(bytes[0], b'*' | b'-' | b'+') {
        return Some(1);
    }
    if bytes[0].is_ascii_digit() {
        for (idx, &byte) in bytes.iter().enumerate().skip(1) {
            // `idx` is also the length of the digit run scanned so far.
            if idx > MAX_ORDERED_MARKER_DIGITS {
                return None;
            }
            if byte == b'.' || byte == b')' {
                return Some(idx + 1);
            }
            if !byte.is_ascii_digit() {
                return None;
            }
        }
    }
    None
}

/// CommonMark permits one through four spaces between a marker and its body.
fn commonmark_marker_padding(rest: &str) -> Option<usize> {
    let padding = rest.bytes().take_while(|byte| *byte == b' ').count();
    match padding {
        0 => None,
        1..=4 => Some(padding),
        _ => Some(1),
    }
}

/// Width of the `[ ] ` / `[x] ` / `[X] ` task-list bracket suffix.
fn task_marker_byte_width(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if bytes.len() >= 4
        && bytes[0] == b'['
        && matches!(bytes[1], b' ' | b'x' | b'X')
        && bytes[2] == b']'
        && bytes[3] == b' '
    {
        Some(4)
    } else {
        None
    }
}

