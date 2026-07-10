use biscuit_terminal::utils::UnicodeWidthStr;
use unicode_script::{Script, UnicodeScript};

use super::lists::is_list_item_start;

pub fn strip_incidental_newlines(content: &str) -> String {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.is_empty() {
        return normalized;
    }

    let lines: Vec<&str> = normalized.split('\n').collect();
    let line_count = if lines.last() == Some(&"") {
        lines.len().saturating_sub(1)
    } else {
        lines.len()
    };
    let trailing_newline = lines.last() == Some(&"");
    let metadata = LineMetadata::scan(&lines[..line_count]);

    let mut result = String::with_capacity(normalized.len());
    let mut skip_prefix_len = 0;

    for idx in 0..line_count {
        let line = lines[idx];
        if skip_prefix_len > 0 && line.len() >= skip_prefix_len {
            result.push_str(&line[skip_prefix_len..]);
            skip_prefix_len = 0;
        } else {
            result.push_str(line);
        }

        if idx + 1 < line_count {
            let boundary = newline_boundary(&metadata[idx], &metadata[idx + 1]);
            match boundary {
                NewlineBoundary::Preserve => result.push('\n'),
                NewlineBoundary::Collapse { skip_next_prefix } => {
                    let next_line = lines[idx + 1];
                    let next_body = if next_line.len() >= skip_next_prefix {
                        &next_line[skip_next_prefix..]
                    } else {
                        next_line
                    };
                    if let Some(separator) = join_separator(line, next_body) {
                        result.push(separator);
                    }
                    skip_prefix_len = skip_next_prefix;
                }
            }
        } else if trailing_newline {
            result.push('\n');
        }
    }

    result
}

/// Reflows Markdown prose lines to `width` display columns.
///
/// Protected Markdown blocks such as fences, indented code, tables, HTML
/// blocks, and transclusion directive lines are emitted unchanged. List,
/// task-list, and blockquote prefixes are preserved while their body text is
/// wrapped inside the remaining width.
///
/// # Panics
///
/// Panics when `width` is `0`.
pub fn reflow_to_width(content: &str, width: usize) -> String {
    assert!(width > 0, "fixed-width reflow requires a width greater than 0");

    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.is_empty() {
        return normalized;
    }

    let lines: Vec<&str> = normalized.split('\n').collect();
    let line_count = if lines.last() == Some(&"") {
        lines.len().saturating_sub(1)
    } else {
        lines.len()
    };
    let trailing_newline = lines.last() == Some(&"");
    let metadata = LineMetadata::scan(&lines[..line_count]);
    let mut result = String::with_capacity(normalized.len());

    for idx in 0..line_count {
        let line = lines[idx];
        if metadata[idx].blank || metadata[idx].protected {
            result.push_str(line);
        } else {
            let wrapped = reflow_line(line, width);
            result.push_str(&wrapped);
        }

        if idx + 1 < line_count || trailing_newline {
            result.push('\n');
        }
    }

    result
}

#[derive(Debug, Clone)]
struct LineMetadata {
    blank: bool,
    protected: bool,
    list_item: bool,
    inline_code_open_after: bool,
    blockquote_prefix_len: Option<usize>,
    /// Line ends in a CommonMark hard line break (two-or-more trailing spaces or
    /// a trailing unescaped backslash); the newline after it must be preserved.
    hard_break: bool,
}

impl LineMetadata {
    fn scan(lines: &[&str]) -> Vec<Self> {
        let mut metadata = Vec::with_capacity(lines.len());
        let mut fence: Option<String> = None;
        let mut html_block: Option<HtmlBlockEnd> = None;
        let mut inline_code_ticks: Option<usize> = None;
        let mut shell_block_depth = 0usize;

        for line in lines {
            let trimmed = line.trim_start();
            let directive_trimmed = directive_trimmed(line);
            let blank = trimmed.is_empty();
            let was_in_fence = fence.is_some();
            let was_in_html = html_block.is_some();
            let was_in_shell_block = shell_block_depth > 0;
            let starts_fence = fence.is_none().then(|| fence_marker(trimmed)).flatten();
            let starts_html = html_block.is_none().then(|| html_block_end(trimmed)).flatten();
            let starts_shell_block = directive_trimmed.starts_with("::shell-block");
            let ends_shell_block = directive_trimmed.starts_with("::end-block");
            let list_item = is_list_item_start(trimmed);
            let protected = was_in_fence
                || was_in_html
                || was_in_shell_block
                || starts_fence.is_some()
                || starts_html.is_some()
                || is_indented_code_line(line)
                || starts_shell_block
                || is_structural_line(trimmed)
                || is_structural_line(directive_trimmed);

            if !protected {
                update_inline_code_state(line, &mut inline_code_ticks);
            }

            metadata.push(Self {
                blank,
                protected,
                list_item,
                inline_code_open_after: inline_code_ticks.is_some(),
                blockquote_prefix_len: blockquote_prefix_len(line),
                hard_break: is_hard_break_line(line),
            });

            if let Some(marker) = starts_fence {
                fence = Some(marker);
            } else if let Some(marker) = &fence
                && trimmed.starts_with(marker)
            {
                fence = None;
            }

            if let Some(end) = starts_html {
                html_block = Some(end);
            }
            if let Some(end) = html_block
                && end.closes_on(line, blank)
            {
                html_block = None;
            }

            if starts_shell_block {
                shell_block_depth += 1;
            } else if was_in_shell_block && ends_shell_block {
                shell_block_depth = shell_block_depth.saturating_sub(1);
            }
        }

        metadata
    }
}

#[derive(Debug, Clone, Copy)]
enum HtmlBlockEnd {
    BlankLine,
    Contains(&'static str),
}

impl HtmlBlockEnd {
    fn closes_on(self, line: &str, blank: bool) -> bool {
        match self {
            Self::BlankLine => blank,
            Self::Contains(needle) => line.contains(needle),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NewlineBoundary {
    Preserve,
    Collapse { skip_next_prefix: usize },
}

fn newline_boundary(current: &LineMetadata, next: &LineMetadata) -> NewlineBoundary {
    // A CommonMark hard line break is structurally significant: keep its newline.
    if current.hard_break {
        return NewlineBoundary::Preserve;
    }

    if current.list_item || next.list_item {
        return if current.list_item && !next.list_item && !next.blank && !next.protected {
            NewlineBoundary::Collapse {
                skip_next_prefix: 0,
            }
        } else {
            NewlineBoundary::Preserve
        };
    }

    if current.blank
        || next.blank
        || current.protected
        || next.protected
        || current.inline_code_open_after
    {
        return NewlineBoundary::Preserve;
    }

    match (current.blockquote_prefix_len, next.blockquote_prefix_len) {
        (Some(current_prefix), Some(next_prefix)) if current_prefix == next_prefix => {
            NewlineBoundary::Collapse {
                skip_next_prefix: next_prefix,
            }
        }
        (Some(_), _) | (_, Some(_)) => NewlineBoundary::Preserve,
        _ => NewlineBoundary::Collapse {
            skip_next_prefix: 0,
        },
    }
}

/// Reports whether `line` ends in a CommonMark hard line break.
///
/// A hard break is two-or-more trailing spaces, or a trailing unescaped
/// backslash (an odd number of trailing backslashes — an even number is a
/// literal escaped backslash, not a break).
fn is_hard_break_line(line: &str) -> bool {
    if line.ends_with("  ") {
        return true;
    }
    let trailing_backslashes = line.chars().rev().take_while(|&c| c == '\\').count();
    trailing_backslashes % 2 == 1
}

/// Decides the separator to insert when collapsing a single newline between two
/// prose lines, keyed off the Unicode Script of the boundary scalars.
///
/// Returns `None` to join with no separator, or `Some(' ')` to insert a single
/// space.
///
/// ## Notes
///
/// - Spaceless scripts (Han, Hiragana, Katakana, Bopomofo, Thai, Lao, Khmer,
///   Myanmar, Tibetan) join with no separator. Hangul is deliberately excluded —
///   Korean uses word spaces, so it is treated as space-delimited.
/// - A script-transition boundary (one side spaceless, the other not, e.g. Han
///   followed by Latin) emits no separator: un-wrapping is neutral, never
///   "pangu" spacing.
/// - A zero-width space (U+200B) on either side joins with no separator.
/// - Otherwise (the space-delimited case) the newline is dropped when the prior
///   line already ends in whitespace, and replaced with a single space when it
///   does not.
fn join_separator(prev_line: &str, next_body: &str) -> Option<char> {
    const ZERO_WIDTH_SPACE: char = '\u{200B}';

    let last = prev_line.chars().next_back();
    let first = next_body.chars().next();

    if last == Some(ZERO_WIDTH_SPACE) || first == Some(ZERO_WIDTH_SPACE) {
        return None;
    }

    // Either both boundary scalars are spaceless-script letters, or the boundary
    // straddles a spaceless/space-delimited transition; both reconstruct with no
    // separator.
    let last_spaceless = last.is_some_and(is_spaceless_letter);
    let first_spaceless = first.is_some_and(is_spaceless_letter);
    if last_spaceless || first_spaceless {
        return None;
    }

    if prev_line.ends_with(char::is_whitespace) {
        None
    } else {
        Some(' ')
    }
}

/// Reports whether `c` is a letter belonging to a curated spaceless script.
///
/// The set is Han, Hiragana, Katakana, Bopomofo, Thai, Lao, Khmer, Myanmar, and
/// Tibetan. Hangul is excluded because Korean is space-delimited. Non-letters
/// (punctuation, symbols, emoji) are never spaceless, so they fall through to
/// the space-delimited join rule.
fn is_spaceless_letter(c: char) -> bool {
    if !c.is_alphabetic() {
        return false;
    }
    matches!(
        c.script(),
        Script::Han
            | Script::Hiragana
            | Script::Katakana
            | Script::Bopomofo
            | Script::Thai
            | Script::Lao
            | Script::Khmer
            | Script::Myanmar
            | Script::Tibetan
    )
}

fn fence_marker(trimmed: &str) -> Option<String> {
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let count = trimmed.chars().take_while(|&c| c == marker).count();
    (count >= 3).then(|| marker.to_string().repeat(count))
}

fn is_indented_code_line(line: &str) -> bool {
    line.starts_with('\t') || line.starts_with("    ")
}

fn is_structural_line(trimmed: &str) -> bool {
    trimmed.starts_with('#')
        || trimmed.starts_with("::")
        || is_table_line(trimmed)
        || trimmed.starts_with("---")
        || trimmed.starts_with("***")
        || trimmed.starts_with("___")
        // `===` (and the `---` handled above) is a setext-heading underline; treat
        // it as structural so the prose line before it keeps its newline.
        || is_setext_underline(trimmed)
}

/// Reports whether `trimmed` is a setext-heading `===` underline.
///
/// A line consisting solely of `=` characters (optionally followed by trailing
/// spaces) underlines a setext H1. The `---` setext H2 form is already matched
/// by the thematic-break check above; only the `=` form needs adding here.
fn is_setext_underline(trimmed: &str) -> bool {
    let underline = trimmed.trim_end();
    !underline.is_empty() && underline.bytes().all(|b| b == b'=')
}

fn directive_trimmed(line: &str) -> &str {
    if let Some(prefix_len) = blockquote_prefix_len(line) {
        return line[prefix_len..].trim_start();
    }

    line.trim_start()
}

fn is_table_line(trimmed: &str) -> bool {
    trimmed.starts_with('|') && trimmed[1..].contains('|')
}

fn html_block_end(trimmed: &str) -> Option<HtmlBlockEnd> {
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("<!--") {
        return Some(HtmlBlockEnd::Contains("-->"));
    }
    if lower.starts_with("<?") {
        return Some(HtmlBlockEnd::Contains("?>"));
    }
    if lower.starts_with("<![cdata[") {
        return Some(HtmlBlockEnd::Contains("]]>"));
    }
    if lower.starts_with("<!") {
        return Some(HtmlBlockEnd::Contains(">"));
    }
    if lower.starts_with("</script")
        || lower.starts_with("</pre")
        || lower.starts_with("</style")
        || lower.starts_with("<script")
        || lower.starts_with("<pre")
        || lower.starts_with("<style")
    {
        return Some(HtmlBlockEnd::BlankLine);
    }

    const BLOCK_TAGS: &[&str] = &[
        "address",
        "article",
        "aside",
        "base",
        "basefont",
        "blockquote",
        "body",
        "caption",
        "center",
        "col",
        "colgroup",
        "dd",
        "details",
        "dialog",
        "dir",
        "div",
        "dl",
        "dt",
        "fieldset",
        "figcaption",
        "figure",
        "footer",
        "form",
        "frame",
        "frameset",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "head",
        "header",
        "hr",
        "html",
        "iframe",
        "legend",
        "li",
        "link",
        "main",
        "menu",
        "menuitem",
        "nav",
        "noframes",
        "ol",
        "optgroup",
        "option",
        "p",
        "param",
        "search",
        "section",
        "summary",
        "table",
        "tbody",
        "td",
        "tfoot",
        "th",
        "thead",
        "title",
        "tr",
        "track",
        "ul",
    ];

    BLOCK_TAGS
        .iter()
        .any(|tag| html_line_starts_with_tag(&lower, tag))
        .then_some(HtmlBlockEnd::BlankLine)
}

fn html_line_starts_with_tag(line: &str, tag: &str) -> bool {
    let Some(rest) = line.strip_prefix("</").or_else(|| line.strip_prefix('<')) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(tag) else {
        return false;
    };
    matches!(rest.as_bytes().first(), Some(b' ' | b'\t' | b'>' | b'/'))
}

fn update_inline_code_state(line: &str, open_ticks: &mut Option<usize>) {
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '`' {
            continue;
        }

        let mut count = 1;
        while chars.peek() == Some(&'`') {
            chars.next();
            count += 1;
        }

        if count >= 3 {
            continue;
        }

        match open_ticks {
            Some(open) if *open == count => *open_ticks = None,
            None => *open_ticks = Some(count),
            _ => {}
        }
    }
}

fn blockquote_prefix_len(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut idx = 0;
    let mut saw_marker = false;

    while idx < bytes.len() {
        while idx < bytes.len() && bytes[idx] == b' ' {
            idx += 1;
        }
        if bytes.get(idx) != Some(&b'>') {
            break;
        }
        saw_marker = true;
        idx += 1;
        if bytes.get(idx) == Some(&b' ') {
            idx += 1;
        }
    }

    saw_marker.then_some(idx)
}

fn reflow_line(line: &str, width: usize) -> String {
    let prefix = line_reflow_prefix(line);
    let body = line[prefix.first_len..].trim();
    if body.is_empty() {
        return line.to_string();
    }

    wrap_text(body, &prefix.first, &prefix.continuation, width)
}

#[derive(Debug, Clone)]
struct ReflowPrefix {
    first: String,
    continuation: String,
    first_len: usize,
}

fn line_reflow_prefix(line: &str) -> ReflowPrefix {
    if let Some(prefix_len) = blockquote_prefix_len(line) {
        let prefix = line[..prefix_len].to_string();
        return ReflowPrefix {
            first: prefix.clone(),
            continuation: prefix,
            first_len: prefix_len,
        };
    }

    let leading_len = line.len() - line.trim_start().len();
    let trimmed = &line[leading_len..];
    if let Some(marker_len) = list_marker_prefix_len(trimmed) {
        let first_len = leading_len + marker_len;
        let first = line[..first_len].to_string();
        return ReflowPrefix {
            continuation: " ".repeat(UnicodeWidthStr::width(first.as_str())),
            first,
            first_len,
        };
    }

    ReflowPrefix {
        first: String::new(),
        continuation: " ".repeat(leading_len),
        first_len: leading_len,
    }
}

fn list_marker_prefix_len(trimmed: &str) -> Option<usize> {
    let marker_len = unordered_marker_prefix_len(trimmed).or_else(|| ordered_marker_prefix_len(trimmed))?;
    let rest = &trimmed[marker_len..];
    task_marker_prefix_len(rest).map_or(Some(marker_len), |task_len| Some(marker_len + task_len))
}

fn unordered_marker_prefix_len(trimmed: &str) -> Option<usize> {
    let mut chars = trimmed.char_indices();
    let (_, marker) = chars.next()?;
    if !matches!(marker, '*' | '-' | '+') {
        return None;
    }
    let (space_idx, space) = chars.next()?;
    (space == ' ').then_some(space_idx + space.len_utf8())
}

fn ordered_marker_prefix_len(trimmed: &str) -> Option<usize> {
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return None;
    }

    for (idx, &byte) in bytes.iter().enumerate().skip(1) {
        if byte == b'.' || byte == b')' {
            return (bytes.get(idx + 1) == Some(&b' ')).then_some(idx + 2);
        }
        if !byte.is_ascii_digit() {
            return None;
        }
    }

    None
}

fn task_marker_prefix_len(rest: &str) -> Option<usize> {
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

fn wrap_text(text: &str, first_prefix: &str, continuation_prefix: &str, width: usize) -> String {
    let tokens = reflow_tokens(text);
    if tokens.is_empty() {
        return first_prefix.to_string();
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = UnicodeWidthStr::width(first_prefix);
    let mut prefix = first_prefix;

    for token in tokens {
        let token_width = UnicodeWidthStr::width(token.as_str());
        let separator_width = usize::from(!current.is_empty());
        if !current.is_empty() && current_width + separator_width + token_width > width {
            lines.push(format!("{prefix}{current}"));
            current.clear();
            prefix = continuation_prefix;
            current_width = UnicodeWidthStr::width(prefix);
        }

        if !current.is_empty() {
            current.push(' ');
            current_width += 1;
        }
        current.push_str(&token);
        current_width += token_width;
    }

    if !current.is_empty() {
        lines.push(format!("{prefix}{current}"));
    }

    lines.join("\n")
}

fn reflow_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }

        if ch == '`' {
            current.push(ch);
            let ticks = consume_matching_chars(&mut chars, '`', &mut current) + 1;
            consume_code_span(&mut chars, ticks, &mut current);
            continue;
        }

        current.push(ch);
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn consume_matching_chars<I>(chars: &mut std::iter::Peekable<I>, target: char, output: &mut String) -> usize
where
    I: Iterator<Item = char>,
{
    let mut count = 0;
    while chars.peek() == Some(&target) {
        output.push(chars.next().unwrap());
        count += 1;
    }
    count
}

fn consume_code_span<I>(chars: &mut std::iter::Peekable<I>, ticks: usize, output: &mut String)
where
    I: Iterator<Item = char>,
{
    let mut run = 0;
    for ch in chars.by_ref() {
        output.push(ch);
        if ch == '`' {
            run += 1;
            if run == ticks {
                break;
            }
        } else {
            run = 0;
        }
    }
}
