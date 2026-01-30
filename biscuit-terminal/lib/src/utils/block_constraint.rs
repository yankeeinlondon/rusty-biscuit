use unicode_width::UnicodeWidthChar;

use crate::{
    components::renderable::RenderableWrapper,
    terminal::Terminal,
    utils::color::{BasicColor, Color, RgbColor, WEB_COLOR_LOOKUP},
};

pub enum MaxWidth {
    None,
    Chars(u32),
    Percent(f32),
}

pub enum TextAlignment {
    Left,
    Center,
    Right,
}

/// Allows for fixed or percentage based margins to be added to the
/// block constraint.
pub enum Margin {
    None,
    Chars(u32),
    Percent(f32),
}

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

/// truncates the line with the `truncate_indicator` string used as the closing
/// part of the string and leaving the resultant string length equal to the `width`.
///
/// Note: this truncation must be smart and be aware of
pub fn truncate<T: Into<String>>(content: T, truncate_indicator: &String, width: &u32) -> String {
    let content = content.into();
    if *width == 0 {
        return String::new();
    }

    let indicator_width = visible_width(truncate_indicator);
    if *width <= indicator_width {
        let (head, _) = split_at_visible_width(truncate_indicator, *width);
        return head;
    }

    if visible_width(&content) <= *width {
        return content;
    }

    let target_width = width.saturating_sub(indicator_width);
    let (head, _) = split_at_visible_width(&content, target_width);
    format!("{}{}", head, truncate_indicator)
}

/// The **word_wrap** function follows the following logic:
///
/// 1. Split content into a vector of string so that we can work with
///    lines of text which have no explicit line breaks and each line
///    given an unlimited amount of space to be rendered to would represent
///    a single line of text.
///
/// 2. Iterate over each line of text and:
///
///     - if `plain_text_length(line)` fits into the available width we're done ...
///       the content does not need to be wrapped, truncated, etc.
///     - if we're
pub fn word_wrap<T: Into<String>>(content: T, strategy: WordWrap, width: u32) {
    let lines = split_lines(content);

    let _ = wrap_lines(lines, &strategy, width);
}

/// Describes the policy to use for wrapping text which goes beyond it's
/// allotted length.
pub enum WordWrap {
    /// Will attempt to wrap words on wrap characters (e.g., whitespace,
    /// `-`, etc.) but if unable to find a break character in the text
    /// body then the text will be hyphenated at a hard break point.
    ///
    /// When the word wrapping logic is engaged _start_ looking for a good
    /// place to break the line a certain number of characters before
    /// max-width is reached.
    ///
    /// By default (e.g., when wrap gets `None`) we start looking for a line break
    /// 8 characters before the reaching the end of the line but you can
    /// override that with whatever you want.
    ///
    /// If we are NOT able find a
    WrapProse(Option<u32>),

    /// Instead of "wrapping", we will truncate any content that moves
    /// beyond the
    Truncate(String),
    /// no word wrap, when the end of line (e.g., max-width) is reached
    /// a new line is started but without any `-` or other markings to
    /// indicate a "continuation" and no attempt is made to break at a
    /// clean break character.
    None,
}

/// The **WidthStrategy** determines if rows in the text block should
/// be padded to ensure that they are always the length of the `max_width`.
///
/// This can be useful when you set a background color to be something
/// other than the default color.
pub enum RowFill {
    /// if the background color _is **not**_ the default background color
    /// then each row's width will be extended to the max width for the
    /// text block. Otherwise, no additional padding is provided.
    Auto,
    /// pad each line to be precisely the length of the max width of the
    /// block's constraint
    Fill,
    /// do not add any padding to force the width to match the max width
    /// of the text constraint
    Exact,
}

/// A **BlockConstraint** is used to define the layout constraints
/// for terminal output.
///
/// This can be used to constrain the output to the terminal page,
/// or a subset of the page (such as a "cell" in a table).
pub struct BlockConstraint {
    /// The maximum width allowed for the text in the block.
    ///
    /// - this is often set to the terminal's current width but
    /// - it can be something less than this
    pub max_width: MaxWidth,

    /// the word wrap strategy for the block constraint
    pub word_wrap: WordWrap,

    pub alignment: TextAlignment,

    pub left_margin: Margin,
    pub right_margin: Margin,

    /// always ensure there is a blank line _before_ the block
    pub leading_blank_line: bool,
    /// always ensure there is a blank line _after_ the block
    pub trailing_blank_line: bool,

    pub text_color: Color,
    pub background_color: Color,
    /// the width strategy to use for the given block constraint
    pub row_fill_strategy: RowFill,
}

impl Default for BlockConstraint {
    fn default() -> BlockConstraint {
        BlockConstraint {
            max_width: MaxWidth::None,
            word_wrap: WordWrap::WrapProse(None),
            alignment: TextAlignment::Left,

            left_margin: Margin::None,
            right_margin: Margin::None,

            leading_blank_line: false,
            trailing_blank_line: false,

            text_color: Color::DefaultForeground,
            background_color: Color::DefaultBackground,
            row_fill_strategy: RowFill::Auto,
        }
    }
}

impl RenderableWrapper for BlockConstraint {
    fn render<T: Into<String>>(self, _content: T) -> String {
        self.render_with_term(_content.into(), None)
    }

    fn fallback_render<T: Into<String>>(
        self,
        _content: T,
        _term: &crate::terminal::Terminal,
    ) -> String {
        self.render_with_term(_content.into(), Some(_term))
    }
}

impl BlockConstraint {
    fn render_with_term(self, content: String, term: Option<&Terminal>) -> String {
        let terminal_width = Terminal::width();
        let max_width = resolve_max_width(&self.max_width, terminal_width);
        let left_margin = resolve_margin(&self.left_margin, max_width);
        let right_margin = resolve_margin(&self.right_margin, max_width);
        let content_width = max_width.saturating_sub(left_margin + right_margin);

        let mut lines = wrap_lines(split_lines(content), &self.word_wrap, content_width);
        if self.leading_blank_line {
            lines.insert(0, String::new());
        }
        if self.trailing_blank_line {
            lines.push(String::new());
        }

        let fill_rows = match self.row_fill_strategy {
            RowFill::Fill => true,
            RowFill::Auto => !is_default_background(&self.background_color),
            RowFill::Exact => false,
        };

        let (prefix, suffix) = color_wrapper(&self.text_color, &self.background_color, term);
        let mut rendered_lines = Vec::with_capacity(lines.len());

        for line in lines {
            let line_length = visible_width(&line);
            let available = content_width.saturating_sub(line_length);
            let left_padding = match self.alignment {
                TextAlignment::Left => 0,
                TextAlignment::Center => available / 2,
                TextAlignment::Right => available,
            };
            let right_padding = if fill_rows {
                available.saturating_sub(left_padding)
            } else {
                0
            };

            let mut row = String::new();
            if left_margin > 0 {
                row.push_str(&" ".repeat(left_margin as usize));
            }
            if left_padding > 0 {
                row.push_str(&" ".repeat(left_padding as usize));
            }
            row.push_str(&line);
            if right_padding > 0 {
                row.push_str(&" ".repeat(right_padding as usize));
            }
            if right_margin > 0 {
                row.push_str(&" ".repeat(right_margin as usize));
            }

            if prefix.is_empty() {
                rendered_lines.push(row);
            } else {
                rendered_lines.push(format!("{}{}{}", prefix, row, suffix));
            }
        }

        join_lines(rendered_lines)
    }
}

fn resolve_max_width(max_width: &MaxWidth, term_width: u32) -> u32 {
    match max_width {
        MaxWidth::None => term_width,
        MaxWidth::Chars(value) => *value,
        MaxWidth::Percent(value) => percent_of(term_width, *value),
    }
}

fn resolve_margin(margin: &Margin, width: u32) -> u32 {
    match margin {
        Margin::None => 0,
        Margin::Chars(value) => *value,
        Margin::Percent(value) => percent_of(width, *value),
    }
}

fn percent_of(total: u32, percent: f32) -> u32 {
    if total == 0 {
        return 0;
    }
    let normalized = if percent <= 1.0 {
        percent
    } else {
        percent / 100.0
    };
    let value = (total as f32 * normalized).floor() as u32;
    value.min(total)
}

fn is_default_background(color: &Color) -> bool {
    matches!(color, Color::DefaultBackground | Color::Reset)
}

fn basic_color_code(color: BasicColor) -> u8 {
    match color {
        BasicColor::Black => 30,
        BasicColor::Red => 31,
        BasicColor::Green => 32,
        BasicColor::Yellow => 33,
        BasicColor::Blue => 34,
        BasicColor::Magenta => 35,
        BasicColor::Cyan => 36,
        BasicColor::White => 37,
        BasicColor::BrightBlack => 90,
        BasicColor::BrightRed => 91,
        BasicColor::BrightGreen => 92,
        BasicColor::BrightYellow => 93,
        BasicColor::BrightBlue => 94,
        BasicColor::BrightMagenta => 95,
        BasicColor::BrightCyan => 96,
        BasicColor::BrightWhite => 97,
    }
}

fn rgb_to_256_index(rgb: &RgbColor) -> u8 {
    let r = rgb.red() as f32;
    let g = rgb.green() as f32;
    let b = rgb.blue() as f32;
    ((r / 256.0 * 36.0).floor() as u8)
        + ((g / 256.0 * 6.0).floor() as u8)
        + ((b / 256.0 * 1.0).floor() as u8)
        + 16
}

fn ansi_color_sequence(
    color: &Color,
    is_background: bool,
    term: Option<&Terminal>,
) -> Option<String> {
    let prefix = if is_background { "48" } else { "38" };
    match color {
        Color::DefaultForeground if !is_background => Some("\x1b[39m".to_string()),
        Color::DefaultBackground if is_background => Some("\x1b[49m".to_string()),
        Color::Reset => Some("\x1b[0m".to_string()),
        Color::Basic(basic) => {
            let code = basic_color_code(*basic) + if is_background { 10 } else { 0 };
            Some(format!("\x1b[{}m", code))
        }
        Color::Rgb(rgb) => {
            let depth = term
                .map(|t| t.color_depth.clone())
                .unwrap_or(crate::discovery::detection::ColorDepth::TrueColor);
            match depth {
                crate::discovery::detection::ColorDepth::TrueColor => Some(format!(
                    "\x1b[{};2;{};{};{}m",
                    prefix,
                    rgb.red(),
                    rgb.green(),
                    rgb.blue()
                )),
                crate::discovery::detection::ColorDepth::Enhanced => {
                    Some(format!("\x1b[{};5;{}m", prefix, rgb_to_256_index(rgb)))
                }
                _ => Some(format!(
                    "\x1b[{}m",
                    basic_color_code(rgb.fallback()) + if is_background { 10 } else { 0 }
                )),
            }
        }
        Color::Web(web) => {
            let rgb = WEB_COLOR_LOOKUP.get(web).copied().unwrap_or(RgbColor::new(
                128,
                128,
                128,
                BasicColor::White,
            ));
            let depth = term
                .map(|t| t.color_depth.clone())
                .unwrap_or(crate::discovery::detection::ColorDepth::TrueColor);
            match depth {
                crate::discovery::detection::ColorDepth::TrueColor => Some(format!(
                    "\x1b[{};2;{};{};{}m",
                    prefix,
                    rgb.red(),
                    rgb.green(),
                    rgb.blue()
                )),
                crate::discovery::detection::ColorDepth::Enhanced => {
                    Some(format!("\x1b[{};5;{}m", prefix, rgb_to_256_index(&rgb)))
                }
                _ => Some(format!(
                    "\x1b[{}m",
                    basic_color_code(rgb.fallback()) + if is_background { 10 } else { 0 }
                )),
            }
        }
        _ => None,
    }
}

fn color_wrapper(fg: &Color, bg: &Color, term: Option<&Terminal>) -> (String, String) {
    let mut prefix = String::new();
    let mut used = false;

    if matches!(fg, Color::Reset) || matches!(bg, Color::Reset) {
        prefix.push_str("\x1b[0m");
        used = true;
    }

    if !matches!(fg, Color::Reset)
        && let Some(seq) = ansi_color_sequence(fg, false, term) {
            prefix.push_str(&seq);
            used = true;
        }

    if !matches!(bg, Color::Reset)
        && let Some(seq) = ansi_color_sequence(bg, true, term) {
            prefix.push_str(&seq);
            used = true;
        }

    let suffix = if used {
        "\x1b[0m".to_string()
    } else {
        String::new()
    };
    (prefix, suffix)
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
        _ => (start + 2).min(bytes.len()),
    }
}

fn visible_width(content: &str) -> u32 {
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

fn split_at_visible_width(content: &str, width: u32) -> (String, String) {
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

fn wrap_lines(lines: Vec<String>, strategy: &WordWrap, width: u32) -> Vec<String> {
    if width == 0 {
        return lines.into_iter().map(|_| String::new()).collect();
    }

    let mut wrapped = Vec::new();
    for line in lines {
        let mut remaining = line;

        loop {
            if visible_width(&remaining) <= width {
                wrapped.push(remaining);
                break;
            }

            match strategy {
                WordWrap::Truncate(indicator) => {
                    wrapped.push(truncate(remaining, indicator, &width));
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
                WordWrap::WrapProse(offset) => {
                    let search_offset = offset.unwrap_or(8);
                    if let Some((break_idx, break_len, is_whitespace)) =
                        find_break_position(&remaining, width, search_offset)
                    {
                        let split_at = if is_whitespace {
                            break_idx
                        } else {
                            break_idx + break_len
                        };
                        let head = remaining[..split_at].to_string();
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
                    } else if width <= 1 {
                        let (head, tail) = split_at_visible_width(&remaining, width);
                        wrapped.push(head);
                        if tail.is_empty() {
                            break;
                        }
                        remaining = tail;
                    } else {
                        let hyphen_width = width.saturating_sub(1);
                        let (mut head, tail) = split_at_visible_width(&remaining, hyphen_width);
                        head.push('-');
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
            &WordWrap::WrapProse(None),
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
            &WordWrap::WrapProse(None),
            5,
        );
        assert_eq!(
            lines,
            vec!["abcd-".to_string(), "efgh-".to_string(), "ij".to_string()]
        );
    }

    #[test]
    fn wrap_lines_truncate_strategy() {
        let lines = wrap_lines(
            vec!["abcdef".to_string()],
            &WordWrap::Truncate("..".to_string()),
            4,
        );
        assert_eq!(lines, vec!["ab..".to_string()]);
    }
}
