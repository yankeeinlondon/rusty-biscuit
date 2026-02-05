use unicode_width::UnicodeWidthChar;

use crate::{terminal::Terminal, utils::layout::WordWrap, utils::word_wrap::truncate};

/// Splits the string content passed in into a vector of string based
/// on any explicit new lines found in the content.
pub fn split_lines<T: Into<String>>(content: T) -> Vec<String> {
    content.into().split('\n').map(|s| s.to_string()).collect()
}

/// The **BlockContent** struct takes a string and converts into
/// a vector of lines by splitting
pub struct BlockContent {
    lines: Vec<String>,
}

impl From<String> for BlockContent {
    fn from(value: String) -> Self {
        BlockContent {
            lines: split_lines(value),
        }
    }
}

impl From<&String> for BlockContent {
    fn from(value: &String) -> Self {
        BlockContent {
            lines: split_lines(value.clone()),
        }
    }
}

impl From<&str> for BlockContent {
    fn from(value: &str) -> Self {
        BlockContent {
            lines: split_lines(value.to_string()),
        }
    }
}

impl BlockContent {
    pub fn new<T: Into<String>>(content: T) -> Self {
        BlockContent::from(content.into())
    }

    /// produces a vector where each element in in the vector
    /// represents a line in the content, and the value represents
    /// the length of the line after all escape codes have been
    /// removed.
    pub fn content_length(self) -> Vec<u32> {
        self.lines
            .into_iter()
            .map(|line| {
                // Strip ANSI escape codes (e.g., \x1b[31m, \x1b[0m)
                // This pattern matches CSI sequences: ESC followed by [ and any characters until a letter
                let stripped = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]")
                    .unwrap()
                    .replace_all(&line, "");
                // Also strip OSC sequences (e.g., \x1b]0;title\x07)
                let stripped = regex::Regex::new(r"\x1b\].*?\x07")
                    .unwrap()
                    .replace_all(&stripped, "");
                stripped.len() as u32
            })
            .collect()
    }
}

/// Converts a vector of strings into a single string
pub fn join_lines<T: Into<String>>(blocks: Vec<T>) -> String {
    blocks
        .into_iter()
        .map(|block| block.into())
        .collect::<Vec<String>>()
        .join("\n")
}

/// Determines the length -- in characters -- of the text being evaluated.
///
/// This must not only strip out all escape codes from the content as a
/// simple first measure, but also consider the length based on grapheme
/// clusters.
pub fn plain_text_length(eval: &str, term: Option<&Terminal>) -> u32 {
    let _ = term;
    visible_width(eval)
}

/// splits content
pub fn split_line<T: Into<String>>(content: T, width: &u32) -> (String, String) {
    let content = content.into();
    split_at_visible_width(&content, *width)
}

fn escape_sequence_end(content: &str, start: usize) -> usize {
    let bytes = content.as_bytes();
    if start >= bytes.len() {
        return bytes.len();
    }
    if bytes[start] != 0x1b {
        return (start + 1).min(bytes.len());
    }
    if start + 1 >= bytes.len() {
        return bytes.len();
    }

    match bytes[start + 1] {
        b'[' => {
            let mut idx = start + 2;
            while idx < bytes.len() {
                let byte = bytes[idx];
                idx += 1;
                if (0x40..=0x7e).contains(&byte) {
                    break;
                }
            }
            idx
        }
        b']' => {
            let mut idx = start + 2;
            while idx < bytes.len() {
                let byte = bytes[idx];
                if byte == 0x07 {
                    idx += 1;
                    break;
                }
                if byte == 0x1b && idx + 1 < bytes.len() && bytes[idx + 1] == b'\\' {
                    idx += 2;
                    break;
                }
                idx += 1;
            }
            idx
        }
        // APC sequences (ESC _ ... ST) - used by Kitty graphics protocol
        b'_' => {
            let mut idx = start + 2;
            while idx < bytes.len() {
                if bytes[idx] == 0x1b && idx + 1 < bytes.len() && bytes[idx + 1] == b'\\' {
                    idx += 2;
                    break;
                }
                idx += 1;
            }
            idx
        }
        _ => (start + 2).min(bytes.len()),
    }
}

pub fn visible_width(content: &str) -> u32 {
    let mut width = 0u32;
    let mut idx = 0usize;
    let bytes = content.as_bytes();

    while idx < content.len() {
        if bytes[idx] == 0x1b {
            idx = escape_sequence_end(content, idx);
            continue;
        }

        let ch = match content[idx..].chars().next() {
            Some(ch) => ch,
            None => break,
        };
        width = width.saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u32);
        idx += ch.len_utf8();
    }

    width
}

pub fn split_at_visible_width(content: &str, width: u32) -> (String, String) {
    if width == 0 {
        return (String::new(), content.to_string());
    }

    let mut head = String::new();
    let mut pending = String::new();
    let mut visible = 0u32;
    let mut idx = 0usize;
    let bytes = content.as_bytes();

    while idx < content.len() {
        if bytes[idx] == 0x1b {
            let end = escape_sequence_end(content, idx);
            pending.push_str(&content[idx..end]);
            idx = end;
            continue;
        }

        let ch = match content[idx..].chars().next() {
            Some(ch) => ch,
            None => break,
        };
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0) as u32;
        let ch_len = ch.len_utf8();

        if visible.saturating_add(ch_width) > width {
            if visible == 0 {
                head.push_str(&pending);
                pending.clear();
                head.push(ch);
                idx += ch_len;
                return (head, content[idx..].to_string());
            }

            let mut tail = String::new();
            tail.push_str(&pending);
            tail.push_str(&content[idx..]);
            return (head, tail);
        }

        head.push_str(&pending);
        pending.clear();
        head.push(ch);
        visible = visible.saturating_add(ch_width);
        idx += ch_len;
    }

    head.push_str(&pending);
    (head, String::new())
}

fn trim_leading_whitespace_preserve_escapes(content: &str) -> String {
    let mut idx = 0usize;
    let mut prefix = String::new();
    let bytes = content.as_bytes();

    while idx < content.len() {
        if bytes[idx] == 0x1b {
            let end = escape_sequence_end(content, idx);
            prefix.push_str(&content[idx..end]);
            idx = end;
            continue;
        }

        let ch = match content[idx..].chars().next() {
            Some(ch) => ch,
            None => break,
        };
        if ch.is_whitespace() {
            idx += ch.len_utf8();
            continue;
        }
        break;
    }

    format!("{}{}", prefix, &content[idx..])
}

fn find_break_position(
    content: &str,
    width: u32,
    search_offset: u32,
) -> Option<(usize, usize, bool)> {
    let start_search = width.saturating_sub(search_offset);
    let mut visible = 0u32;
    let mut idx = 0usize;
    let bytes = content.as_bytes();
    let mut last_break: Option<(usize, usize, bool)> = None;

    while idx < content.len() {
        if bytes[idx] == 0x1b {
            idx = escape_sequence_end(content, idx);
            continue;
        }

        let ch = match content[idx..].chars().next() {
            Some(ch) => ch,
            None => break,
        };
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0) as u32;
        let ch_len = ch.len_utf8();

        if ch_width == 0 {
            idx += ch_len;
            continue;
        }

        if visible.saturating_add(ch_width) > width {
            break;
        }

        visible = visible.saturating_add(ch_width);
        if visible >= start_search && (ch.is_whitespace() || ch == '-') {
            last_break = Some((idx, ch_len, ch.is_whitespace()));
        }

        idx += ch_len;
    }

    last_break
}

fn find_bespoke_break_position(
    content: &str,
    width: u32,
    search_offset: u32,
    break_chars: &[char],
) -> Option<(usize, usize, bool)> {
    let start_search = width.saturating_sub(search_offset);
    let mut visible = 0u32;
    let mut idx = 0usize;
    let bytes = content.as_bytes();
    let mut last_break: Option<(usize, usize, bool)> = None;

    while idx < content.len() {
        if bytes[idx] == 0x1b {
            idx = escape_sequence_end(content, idx);
            continue;
        }

        let ch = match content[idx..].chars().next() {
            Some(ch) => ch,
            None => break,
        };
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0) as u32;
        let ch_len = ch.len_utf8();

        if ch_width == 0 {
            idx += ch_len;
            continue;
        }

        if visible.saturating_add(ch_width) > width {
            break;
        }

        visible = visible.saturating_add(ch_width);
        if visible >= start_search && (ch.is_whitespace() || break_chars.contains(&ch)) {
            last_break = Some((idx, ch_len, ch.is_whitespace()));
        }

        idx += ch_len;
    }

    last_break
}

pub fn wrap_lines(lines: Vec<String>, strategy: &WordWrap, width: u32) -> Vec<String> {
    if width == 0 {
        return lines.into_iter().map(|_| String::new()).collect();
    }

    let mut wrapped = Vec::new();
    for line in lines {
        let original_width = visible_width(&line);
        let mut remaining = line;

        loop {
            // Check if remaining text fits - apply hanging indent if applicable
            if visible_width(&remaining) <= width {
                let final_text = match strategy {
                    WordWrap::WrapProse(_, hanging_indent)
                    | WordWrap::BespokeProse(_, _, hanging_indent) => {
                        let indent = hanging_indent.unwrap_or(0) as usize;
                        let is_continuation =
                            !wrapped.is_empty() || visible_width(&remaining) != original_width;
                        if is_continuation && indent > 0 {
                            format!("{}{}", " ".repeat(indent), remaining)
                        } else {
                            remaining
                        }
                    }
                    _ => remaining,
                };
                wrapped.push(final_text);
                break;
            }

            match strategy {
                WordWrap::Truncate(indicator) => {
                    let default_indicator = String::from("…");
                    let indicator_ref = indicator.as_ref().unwrap_or(&default_indicator);
                    wrapped.push(truncate(remaining, indicator_ref, &width));
                    break;
                }
                WordWrap::None => {
                    let (head, tail) = split_at_visible_width(&remaining, width);
                    wrapped.push(head);
                    if tail.is_empty() {
                        break;
                    }
                    remaining = tail;
                }
                WordWrap::WrapProse(offset, hanging_indent) => {
                    let search_offset = offset.unwrap_or(8);
                    let indent = hanging_indent.unwrap_or(0) as usize;
                    let is_continuation =
                        !wrapped.is_empty() || visible_width(&remaining) != original_width;
                    let effective_width = if is_continuation && indent > 0 {
                        width.saturating_sub(indent as u32)
                    } else {
                        width
                    };

                    if let Some((break_idx, break_len, is_whitespace)) =
                        find_break_position(&remaining, effective_width, search_offset)
                    {
                        let split_at = if is_whitespace {
                            break_idx
                        } else {
                            break_idx + break_len
                        };
                        let mut head = remaining[..split_at].to_string();
                        if is_continuation && indent > 0 {
                            head = format!("{}{}", " ".repeat(indent), head);
                        }
                        let tail = if is_whitespace {
                            trim_leading_whitespace_preserve_escapes(
                                &remaining[break_idx + break_len..],
                            )
                        } else {
                            remaining[split_at..].to_string()
                        };
                        wrapped.push(head);
                        if tail.is_empty() {
                            break;
                        }
                        remaining = tail;
                    } else if effective_width <= 1 {
                        let (head, tail) = split_at_visible_width(&remaining, effective_width);
                        let mut final_head = head;
                        if is_continuation && indent > 0 {
                            final_head = format!("{}{}", " ".repeat(indent), final_head);
                        }
                        wrapped.push(final_head);
                        if tail.is_empty() {
                            break;
                        }
                        remaining = tail;
                    } else {
                        let hyphen_width = effective_width.saturating_sub(1);
                        let (mut head, tail) = split_at_visible_width(&remaining, hyphen_width);
                        head.push('-');
                        if is_continuation && indent > 0 {
                            head = format!("{}{}", " ".repeat(indent), head);
                        }
                        wrapped.push(head);
                        if tail.is_empty() {
                            break;
                        }
                        remaining = tail;
                    }
                }
                WordWrap::BespokeProse(offset, break_chars, hanging_indent) => {
                    let search_offset = offset.unwrap_or(8);
                    let indent = hanging_indent.unwrap_or(0) as usize;
                    let is_continuation =
                        !wrapped.is_empty() || visible_width(&remaining) != original_width;
                    let effective_width = if is_continuation && indent > 0 {
                        width.saturating_sub(indent as u32)
                    } else {
                        width
                    };

                    if let Some((break_idx, break_len, is_whitespace)) = find_bespoke_break_position(
                        &remaining,
                        effective_width,
                        search_offset,
                        break_chars,
                    ) {
                        let split_at = if is_whitespace {
                            break_idx
                        } else {
                            break_idx + break_len
                        };
                        let mut head = remaining[..split_at].to_string();
                        if is_continuation && indent > 0 {
                            head = format!("{}{}", " ".repeat(indent), head);
                        }
                        let tail = if is_whitespace {
                            trim_leading_whitespace_preserve_escapes(
                                &remaining[break_idx + break_len..],
                            )
                        } else {
                            remaining[split_at..].to_string()
                        };
                        wrapped.push(head);
                        if tail.is_empty() {
                            break;
                        }
                        remaining = tail;
                    } else if effective_width <= 1 {
                        let (head, tail) = split_at_visible_width(&remaining, effective_width);
                        let mut final_head = head;
                        if is_continuation && indent > 0 {
                            final_head = format!("{}{}", " ".repeat(indent), final_head);
                        }
                        wrapped.push(final_head);
                        if tail.is_empty() {
                            break;
                        }
                        remaining = tail;
                    } else {
                        let hyphen_width = effective_width.saturating_sub(1);
                        let (mut head, tail) = split_at_visible_width(&remaining, hyphen_width);
                        head.push('-');
                        if is_continuation && indent > 0 {
                            head = format!("{}{}", " ".repeat(indent), head);
                        }
                        wrapped.push(head);
                        if tail.is_empty() {
                            break;
                        }
                        remaining = tail;
                    }
                }
            }
        }
    }

    wrapped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_width_strips_escape_codes() {
        let content = "\x1b[31mred\x1b[0m\x1b]8;;https://example.com\x07link\x1b]8;;\x07";
        assert_eq!(visible_width(content), 7);
    }

    #[test]
    fn visible_width_strips_kitty_graphics() {
        let content = "\x1b_Gf=100,a=T,t=d,c=10,m=0;AAAA\x1b\\text";
        assert_eq!(visible_width(content), 4);
    }

    #[test]
    fn plain_text_length_respects_unicode_width() {
        let content = "\u{4F60}\u{597D}".to_string();
        assert_eq!(plain_text_length(&content, None), 4);
    }

    #[test]
    fn split_line_preserves_content_and_width() {
        let content = "\x1b[31mred\x1b[0m blue";
        let (head, tail) = split_line(content, &3);
        assert_eq!(visible_width(&head), 3);
        assert_eq!(format!("{}{}", head, tail), content);
    }

    #[test]
    fn truncate_respects_indicator_and_width() {
        let indicator = "...".to_string();
        let result = truncate("hello world", &indicator, &8);
        assert_eq!(result, "hello...");
        assert_eq!(visible_width(&result), 8);
    }

    #[test]
    fn truncate_handles_small_width() {
        let indicator = "...".to_string();
        let result = truncate("abcdef", &indicator, &2);
        assert_eq!(result, "..");
        assert_eq!(visible_width(&result), 2);
    }

    #[test]
    fn wrap_lines_none_breaks_hard() {
        let lines = wrap_lines(vec!["abcdef".to_string()], &WordWrap::None, 3);
        assert_eq!(lines, vec!["abc".to_string(), "def".to_string()]);
    }

    #[test]
    fn wrap_lines_wrapprose_breaks_on_space() {
        let lines = wrap_lines(
            vec!["hello world friend".to_string()],
            &WordWrap::WrapProse(None, None),
            10,
        );
        assert_eq!(
            lines,
            vec![
                "hello".to_string(),
                "world".to_string(),
                "friend".to_string()
            ]
        );
    }

    #[test]
    fn wrap_lines_wrapprose_hyphenates_long_words() {
        let lines = wrap_lines(
            vec!["abcdefghij".to_string()],
            &WordWrap::WrapProse(None, None),
            5,
        );
        assert_eq!(
            lines,
            vec!["abcd-".to_string(), "efgh-".to_string(), "ij".to_string()]
        );
    }

    #[test]
    fn wrap_lines_wrapprose_with_hanging_indent() {
        let lines = wrap_lines(
            vec!["hello world friend".to_string()],
            &WordWrap::WrapProse(None, Some(2)),
            10,
        );
        assert_eq!(
            lines,
            vec![
                "hello".to_string(),
                "  world".to_string(),
                "  friend".to_string()
            ]
        );
    }

    #[test]
    fn wrap_lines_truncate_strategy() {
        let lines = wrap_lines(
            vec!["abcdef".to_string()],
            &WordWrap::Truncate(Some("..".to_string())),
            4,
        );
        assert_eq!(lines, vec!["ab..".to_string()]);
    }
}
