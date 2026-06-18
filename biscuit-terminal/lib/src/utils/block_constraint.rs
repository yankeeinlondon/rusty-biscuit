use unicode_width::UnicodeWidthChar;

use crate::{terminal::Terminal, utils::word_wrap::truncate, utils::wrap_policy::WordWrap};

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
        _ => {
            if let Some(ch) = content[start + 1..].chars().next() {
                start + 1 + ch.len_utf8()
            } else {
                bytes.len()
            }
        }
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
        // Characters with ambiguous/unknown width (e.g., emoji like ✅, ⛔) typically
        // render as 2 columns in modern terminals. Regular symbols like ✗ return
        // Some(1) from unicode_width and are handled correctly.
        width = width.saturating_add(UnicodeWidthChar::width(ch).unwrap_or(1) as u32);
        idx += ch.len_utf8();
    }

    width
}

/// Splits `content` into its visible body and the run of trailing escape
/// sequences that follow the last visible character.
///
/// The trailing run is exactly the closing envelope a styled inline node
/// appends after its visible label — an SGR reset plus ancestor-style restore,
/// and any OSC8 link close. Truncators ([`truncate`]) keep only the visible
/// prefix and discard the cut tail, which would strip this run and leak the
/// node's color into following content. Callers split it off, truncate the
/// body, then re-append it. Returns `(body, trailing)` with
/// `format!("{body}{trailing}") == content`.
pub fn split_trailing_escapes(content: &str) -> (&str, &str) {
    let bytes = content.as_bytes();
    let mut idx = 0usize;
    let mut last_visible_end = 0usize;

    while idx < content.len() {
        if bytes[idx] == 0x1b {
            idx = escape_sequence_end(content, idx);
            continue;
        }

        let ch = match content[idx..].chars().next() {
            Some(ch) => ch,
            None => break,
        };
        idx += ch.len_utf8();
        last_visible_end = idx;
    }

    content.split_at(last_visible_end)
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
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1) as u32;
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
    let mut early_break: Option<(usize, usize, bool)> = None;

    while idx < content.len() {
        if bytes[idx] == 0x1b {
            idx = escape_sequence_end(content, idx);
            continue;
        }

        let ch = match content[idx..].chars().next() {
            Some(ch) => ch,
            None => break,
        };
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1) as u32;
        let ch_len = ch.len_utf8();

        if ch_width == 0 {
            idx += ch_len;
            continue;
        }

        if visible.saturating_add(ch_width) > width {
            break;
        }

        visible = visible.saturating_add(ch_width);
        if ch.is_whitespace() || ch == '-' {
            if visible >= start_search {
                last_break = Some((idx, ch_len, ch.is_whitespace()));
            } else {
                early_break = Some((idx, ch_len, ch.is_whitespace()));
            }
        }

        idx += ch_len;
    }

    last_break.or(early_break)
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
    let mut early_break: Option<(usize, usize, bool)> = None;

    while idx < content.len() {
        if bytes[idx] == 0x1b {
            idx = escape_sequence_end(content, idx);
            continue;
        }

        let ch = match content[idx..].chars().next() {
            Some(ch) => ch,
            None => break,
        };
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(1) as u32;
        let ch_len = ch.len_utf8();

        if ch_width == 0 {
            idx += ch_len;
            continue;
        }

        if visible.saturating_add(ch_width) > width {
            break;
        }

        visible = visible.saturating_add(ch_width);
        if ch.is_whitespace() || break_chars.contains(&ch) {
            if visible >= start_search {
                last_break = Some((idx, ch_len, ch.is_whitespace()));
            } else {
                early_break = Some((idx, ch_len, ch.is_whitespace()));
            }
        }

        idx += ch_len;
    }

    last_break.or(early_break)
}

pub fn wrap_lines(lines: Vec<String>, strategy: &WordWrap, width: u32) -> Vec<String> {
    if width == 0 {
        return lines.into_iter().map(|_| String::new()).collect();
    }

    let mut wrapped: Vec<String> = Vec::new();
    for line in lines {
        let original_width = visible_width(&line);
        let mut remaining = line;

        loop {
            // If only escape sequences remain (0 visible width but non-empty),
            // append them to the previous line so they don't become a blank row.
            // This happens when a word-wrap split lands just before closing
            // sequences like \x1b[0m or \x1b]8;;\x1b\\.
            if !remaining.is_empty() && visible_width(&remaining) == 0 {
                if let Some(last) = wrapped.last_mut() {
                    last.push_str(&remaining);
                }
                break;
            }

            // Check if remaining text fits - apply hanging indent if applicable.
            // For continuation lines with hanging indent, the effective available
            // width is reduced by the indent so the prepended spaces don't overflow.
            {
                let (indent, is_continuation) = match strategy {
                    WordWrap::WrapProse(_, hanging_indent)
                    | WordWrap::BespokeProse(_, _, hanging_indent) => {
                        let indent = hanging_indent.unwrap_or(0) as usize;
                        let is_cont =
                            !wrapped.is_empty() || visible_width(&remaining) != original_width;
                        (indent, is_cont)
                    }
                    _ => (0, false),
                };
                let fits_width = if is_continuation && indent > 0 {
                    width.saturating_sub(indent as u32)
                } else {
                    width
                };
                if visible_width(&remaining) <= fits_width {
                    let final_text = if is_continuation && indent > 0 {
                        format!("{}{}", " ".repeat(indent), remaining)
                    } else {
                        remaining
                    };
                    wrapped.push(final_text);
                    break;
                }
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
                        let tail = trim_leading_whitespace_preserve_escapes(&remaining[split_at..]);
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
                        // Always trim leading whitespace so that breaking on
                        // non-whitespace chars (e.g. comma in "a, b") doesn't
                        // carry a stray space into the next line.
                        let tail = trim_leading_whitespace_preserve_escapes(&remaining[split_at..]);
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

/// Ensures each line in a multi-line split is ANSI-self-contained.
///
/// When text containing ANSI escape sequences is word-wrapped, a color or
/// hyperlink opened on one line may carry across the split boundary. This
/// causes "color bleed" when lines are rendered side-by-side (e.g. in table
/// cells sharing a row with other columns).
///
/// This function:
/// 1. Appends `\x1b[0m` (and OSC8 close) at the end of each non-last line
///    that has active styles
/// 2. Re-opens those styles at the start of the next line
pub fn sanitize_wrapped_lines(lines: Vec<String>) -> Vec<String> {
    if lines.len() <= 1 {
        return lines;
    }

    let last_idx = lines.len() - 1;
    let mut result = Vec::with_capacity(lines.len());
    let mut carry_fg: Option<String> = None;
    let mut carry_osc8: Option<String> = None;

    for (i, line) in lines.into_iter().enumerate() {
        let mut current = String::new();

        // Re-apply carried state from previous line
        if let Some(ref osc8) = carry_osc8 {
            current.push_str(osc8);
        }
        if let Some(ref fg) = carry_fg {
            current.push_str(fg);
        }
        current.push_str(&line);

        // Scan the full line (with carried prefixes) for active state
        let (new_fg, new_osc8) = active_ansi_state(&current);

        // Close active state at end of non-last lines
        if i < last_idx && (new_fg.is_some() || new_osc8.is_some()) {
            if new_osc8.is_some() {
                current.push_str("\x1b]8;;\x1b\\");
            }
            current.push_str("\x1b[0m");
        }

        carry_fg = new_fg;
        carry_osc8 = new_osc8;
        result.push(current);
    }

    result
}

/// The set of SGR attributes still open at a point in a string.
///
/// Tracking the **full** set — emphasis plus foreground and background — is what
/// keeps a bold or background run (not just a foreground color) from bleeding
/// past a wrapped/multiline split into padding, borders, or the next line.
#[derive(Default)]
struct SgrState {
    bold: bool,
    dim: bool,
    italic: bool,
    /// The active underline param token (`"4"`, `"21"`, or an ITU `"4:2"`), or
    /// `None` when no underline is open. Stored verbatim so the subtype survives
    /// a re-open.
    underline: Option<String>,
    blink: bool,
    inverse: bool,
    strikethrough: bool,
    /// Active foreground color params, e.g. `["31"]` or `["38","2","255","0","0"]`.
    fg: Vec<String>,
    /// Active background color params.
    bg: Vec<String>,
}

impl SgrState {
    /// The single `\x1b[…m` run that re-applies every active attribute, or
    /// `None` when nothing is open.
    fn reopen_run(&self) -> Option<String> {
        let mut codes: Vec<String> = Vec::new();
        if self.bold {
            codes.push("1".into());
        }
        if self.dim {
            codes.push("2".into());
        }
        if self.italic {
            codes.push("3".into());
        }
        if let Some(u) = &self.underline {
            codes.push(u.clone());
        }
        if self.blink {
            codes.push("5".into());
        }
        if self.inverse {
            codes.push("7".into());
        }
        if self.strikethrough {
            codes.push("9".into());
        }
        codes.extend(self.fg.iter().cloned());
        codes.extend(self.bg.iter().cloned());
        (!codes.is_empty()).then(|| format!("\x1b[{}m", codes.join(";")))
    }
}

/// Number of `;`-separated tokens an extended-color introducer consumes,
/// starting at its `38`/`48` token: `…;5;n` → 3, `…;2;r;g;b` → 5. Falls back to
/// 1 for a malformed form so the parser always advances.
fn extended_color_span(tokens: &[&str]) -> usize {
    match tokens.get(1).copied() {
        Some("5") => 3.min(tokens.len()),
        Some("2") => 5.min(tokens.len()),
        _ => 1,
    }
}

/// Folds one CSI SGR parameter list into `state`.
fn apply_sgr_params(params: &str, state: &mut SgrState) {
    // `\x1b[m` (empty params) is shorthand for a full reset.
    if params.is_empty() {
        *state = SgrState::default();
        return;
    }
    let tokens: Vec<&str> = params.split(';').collect();
    let mut i = 0;
    while i < tokens.len() {
        let tok = tokens[i];
        // ITU sub-parameter underline form (`4:0` off, `4:2` double, …).
        if tok.starts_with("4:") {
            state.underline = (tok != "4:0").then(|| tok.to_string());
            i += 1;
            continue;
        }
        let Ok(n) = tok.parse::<u32>() else {
            i += 1;
            continue;
        };
        match n {
            0 => *state = SgrState::default(),
            1 => state.bold = true,
            2 => state.dim = true,
            3 => state.italic = true,
            4 => state.underline = Some("4".into()),
            5 => state.blink = true,
            7 => state.inverse = true,
            9 => state.strikethrough = true,
            21 => state.underline = Some("21".into()),
            22 => {
                state.bold = false;
                state.dim = false;
            }
            23 => state.italic = false,
            24 => state.underline = None,
            25 => state.blink = false,
            27 => state.inverse = false,
            29 => state.strikethrough = false,
            30..=37 | 90..=97 => state.fg = vec![tok.to_string()],
            39 => state.fg.clear(),
            40..=47 | 100..=107 => state.bg = vec![tok.to_string()],
            49 => state.bg.clear(),
            38 => {
                let span = extended_color_span(&tokens[i..]);
                state.fg = tokens[i..i + span].iter().map(ToString::to_string).collect();
                i += span;
                continue;
            }
            48 => {
                let span = extended_color_span(&tokens[i..]);
                state.bg = tokens[i..i + span].iter().map(ToString::to_string).collect();
                i += span;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
}

/// Scans content for active (unclosed) ANSI SGR and OSC8 link state.
///
/// Returns `(active_sgr_reopen_run, active_osc8_open_sequence)`, where the first
/// is a single `\x1b[…m` re-applying every still-open SGR attribute (emphasis,
/// foreground, background) or `None` when no SGR attribute is open.
fn active_ansi_state(content: &str) -> (Option<String>, Option<String>) {
    let mut state = SgrState::default();
    let mut osc8: Option<String> = None;
    let bytes = content.as_bytes();
    let mut idx = 0;

    while idx < content.len() {
        if bytes[idx] != 0x1b {
            idx += 1;
            continue;
        }

        let end = escape_sequence_end(content, idx);
        let seq = &content[idx..end];

        if seq.starts_with("\x1b[") && seq.ends_with('m') {
            apply_sgr_params(&seq[2..seq.len() - 1], &mut state);
        }
        // OSC8 link: `\x1b]8;;url\x1b\\` — a non-empty URL opens, empty closes.
        else if let Some(inner) = seq.strip_prefix("\x1b]8;;") {
            let url_empty = inner == "\x1b\\" || inner == "\x07" || inner.is_empty();
            osc8 = (!url_empty).then(|| seq.to_string());
        }

        idx = end;
    }

    (state.reopen_run(), osc8)
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
    fn visible_width_handles_emoji_with_ambiguous_width() {
        // Emoji like ✅ (U+2705) may return None from unicode_width.
        // We default to 1 for unknown characters.
        // Note: actual terminal rendering varies - some show emoji as 1 wide, others as 2.
        let checkmark = "\u{2705}"; // ✅
        let width = visible_width(checkmark);
        assert!(width >= 1, "Emoji should have width >= 1, got {}", width);

        // Ballot X (U+2717) - this is a dingbat symbol.
        // unicode_width returns Some(1) for this.
        let ballot_x = "\u{2717}"; // ✗
        let width = visible_width(ballot_x);
        assert_eq!(width, 1, "Ballot X should have width 1, got {}", width);
    }

    #[test]
    fn split_trailing_escapes_separates_closing_envelope() {
        let content = "\x1b[31mred text\x1b[0m";
        let (body, trailing) = split_trailing_escapes(content);
        assert_eq!(body, "\x1b[31mred text");
        assert_eq!(trailing, "\x1b[0m");
        assert_eq!(format!("{body}{trailing}"), content);
    }

    #[test]
    fn split_trailing_escapes_handles_osc8_and_sgr_close() {
        let content = "\x1b]8;;url\x1b\\\x1b[31mlink\x1b[0m\x1b]8;;\x1b\\";
        let (body, trailing) = split_trailing_escapes(content);
        assert_eq!(body, "\x1b]8;;url\x1b\\\x1b[31mlink");
        assert_eq!(trailing, "\x1b[0m\x1b]8;;\x1b\\");
    }

    #[test]
    fn split_trailing_escapes_no_trailing_run() {
        let content = "\x1b[31mred";
        let (body, trailing) = split_trailing_escapes(content);
        assert_eq!(body, content);
        assert_eq!(trailing, "");
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
    fn wrap_lines_hanging_indent_does_not_overflow_on_fits_check() {
        // Regression: remaining text that fits within `width` but NOT within
        // `width - indent` was accepted by the early-exit check and then
        // prepended with indent spaces, producing a line wider than `width`.
        let lines = wrap_lines(
            vec!["aaa bbb ccc ddd eee".to_string()],
            &WordWrap::BespokeProse(Some(50), vec![' '], Some(4)),
            12,
        );
        // width=12, indent=4 → effective=8 for continuation lines.
        // "aaa bbb ccc" (11 visible) fits the first line (no indent).
        // "ddd eee" (7 visible) fits within effective=8 → "    ddd eee" (11 visible ≤ 12).
        // Without the fix, "ddd eee" (7) passed the `<= 12` check and became
        // "    ddd eee" (11) — correct here, but with tighter values it overflowed.
        for line in &lines {
            assert!(
                visible_width(line) <= 12,
                "line overflows width: {:?} (visible={})",
                line,
                visible_width(line),
            );
        }
    }

    #[test]
    fn wrap_lines_hanging_indent_remaining_exceeds_effective_width() {
        // The remaining text fits in `width` but NOT in `width - indent`,
        // so it must be wrapped further rather than emitted as-is.
        let lines = wrap_lines(
            vec!["aaaa bbbb cccccccc".to_string()],
            &WordWrap::BespokeProse(Some(50), vec![' '], Some(6)),
            14,
        );
        // width=14, indent=6 → effective=8 for continuation.
        // First line: "aaaa bbbb" (9) fits width=14.
        // Remaining: "cccccccc" (8) fits width=14 but NOT effective=8 → must still wrap.
        // With indent: "      cccccccc" = 14 visible = exactly fits.
        // But if remaining were 9 chars it would overflow without the fix.
        for line in &lines {
            assert!(
                visible_width(line) <= 14,
                "line overflows width: {:?} (visible={})",
                line,
                visible_width(line),
            );
        }
    }

    #[test]
    fn wrap_lines_prefers_word_break_over_hyphenation() {
        // When the last word is longer than search_offset, the algorithm should
        // fall back to an earlier break point rather than hyphenating mid-word.
        // "hello immediate-mode rendering" at width=25 with search_offset=8:
        // search zone starts at col 17. The space before "immediate-mode" is at
        // col 6 (before the zone), but should still be used as a fallback.
        let lines = wrap_lines(
            vec!["hello immediate-mode rendering".to_string()],
            &WordWrap::WrapProse(Some(8), None),
            25,
        );
        assert_eq!(
            lines,
            vec!["hello immediate-mode".to_string(), "rendering".to_string(),]
        );
    }

    #[test]
    fn wrap_bespoke_prefers_word_break_over_hyphenation() {
        // Same test for BespokeProse: should use early break rather than hyphenating.
        let lines = wrap_lines(
            vec!["hello immediate-mode rendering".to_string()],
            &WordWrap::BespokeProse(Some(8), vec![' '], None),
            25,
        );
        assert_eq!(
            lines,
            vec!["hello immediate-mode".to_string(), "rendering".to_string(),]
        );
    }

    #[test]
    fn wrap_bespoke_breaks_on_hyphen_and_underscore() {
        // With '-' and '_' as break chars, long hyphenated/underscored words
        // break at those characters instead of being force-hyphenated.
        let lines = wrap_lines(
            vec!["cross-platform terminal applications".to_string()],
            &WordWrap::BespokeProse(Some(8), vec![' ', '-', '_'], None),
            20,
        );
        assert_eq!(
            lines,
            vec![
                "cross-platform".to_string(),
                "terminal".to_string(),
                "applications".to_string(),
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

    #[test]
    fn sanitize_single_line_unchanged() {
        let lines = vec!["\x1b[31mred text\x1b[0m".to_string()];
        let result = sanitize_wrapped_lines(lines.clone());
        assert_eq!(result, lines);
    }

    #[test]
    fn sanitize_no_escapes_unchanged() {
        let lines = vec!["hello".to_string(), "world".to_string()];
        let result = sanitize_wrapped_lines(lines.clone());
        assert_eq!(result, lines);
    }

    #[test]
    fn sanitize_resets_fg_at_line_break() {
        // Simulate a colored string split mid-color
        let lines = vec![
            "\x1b[38;2;59;130;246mhello-wor".to_string(),
            "ld\x1b[39m\x1b[0m".to_string(),
        ];
        let result = sanitize_wrapped_lines(lines);
        // Line 1 should end with reset
        assert!(
            result[0].ends_with("\x1b[0m"),
            "Line 1 should end with reset"
        );
        // Line 2 should re-open the color
        assert!(
            result[1].starts_with("\x1b[38;2;59;130;246m"),
            "Line 2 should re-open foreground color"
        );
    }

    #[test]
    fn sanitize_resets_osc8_at_line_break() {
        let lines = vec![
            "\x1b]8;;https://example.com\x1b\\hello-wor".to_string(),
            "ld\x1b]8;;\x1b\\".to_string(),
        ];
        let result = sanitize_wrapped_lines(lines);
        // Line 1 should close OSC8 and reset
        assert!(
            result[0].contains("\x1b]8;;\x1b\\"),
            "Line 1 should close OSC8"
        );
        assert!(
            result[0].ends_with("\x1b[0m"),
            "Line 1 should end with SGR reset"
        );
        // Line 2 should re-open OSC8
        assert!(
            result[1].starts_with("\x1b]8;;https://example.com\x1b\\"),
            "Line 2 should re-open OSC8 link"
        );
    }

    #[test]
    fn sanitize_handles_osc8_plus_color() {
        let lines = vec![
            "\x1b]8;;https://hf.co/org/model\x1b\\\x1b[38;2;37;99;235morg/\x1b[39m\x1b[38;2;59;130;246mmodel-name-th".to_string(),
            "at-is-long\x1b[39m\x1b[0m\x1b]8;;\x1b\\\x1b[0m".to_string(),
        ];
        let result = sanitize_wrapped_lines(lines);
        // Line 1: should close both OSC8 and color
        assert!(result[0].ends_with("\x1b[0m"));
        assert!(result[0].contains("\x1b]8;;\x1b\\"));
        // Line 2: should re-open OSC8 and the active blue-500 color
        assert!(
            result[1].starts_with("\x1b]8;;https://hf.co/org/model\x1b\\\x1b[38;2;59;130;246m")
        );
    }

    #[test]
    fn sanitize_no_action_when_styles_closed() {
        // All styles properly closed within each line
        let lines = vec![
            "\x1b[31mred\x1b[39m\x1b[0m".to_string(),
            "plain text".to_string(),
        ];
        let result = sanitize_wrapped_lines(lines.clone());
        assert_eq!(result, lines);
    }

    #[test]
    fn wrap_lines_no_trailing_blank_from_escape_sequences() {
        // Simulates an OSC8 link with colored text that wraps.
        // After the last visible char, only closing escapes remain.
        // These must NOT become a separate (blank) line.
        let osc_open = "\x1b]8;;https://example.com\x1b\\";
        let osc_close = "\x1b]8;;\x1b\\";
        let fg_open = "\x1b[38;2;59;130;246m";
        let fg_reset = "\x1b[0m";

        // "hello world" = 11 visible chars, wrapped at width 6
        let content = format!("{osc_open}{fg_open}hello world{fg_reset}{osc_close}");
        let lines = vec![content];
        let result = wrap_lines(lines, &WordWrap::WrapProse(None, None), 6);

        // Should be exactly 2 lines, not 3
        assert_eq!(result.len(), 2, "got lines: {:?}", result);
        // Both lines should have visible content
        assert!(
            visible_width(&result[0]) > 0,
            "line 0 should have visible content"
        );
        assert!(
            visible_width(&result[1]) > 0,
            "line 1 should have visible content"
        );
    }

    #[test]
    fn wrap_lines_closing_escapes_appended_to_last_content_line() {
        // When the break falls right before closing escapes, they should
        // be appended to the previous line, not create a new one.
        let fg = "\x1b[31m";
        let reset = "\x1b[0m";

        // "abcdef" = 6 visible chars, wrapped at width 3 → 3 lines
        let content = format!("{fg}abcdef{reset}");
        let lines = vec![content];
        let result = wrap_lines(lines, &WordWrap::WrapProse(None, None), 3);

        assert_eq!(result.len(), 3, "got lines: {:?}", result);
        // The reset should be on the last content line, not a separate line
        let last = result.last().unwrap();
        assert!(last.contains(reset), "last line should contain the reset");
        assert!(
            visible_width(last) > 0,
            "last line should have visible content"
        );
    }
}
