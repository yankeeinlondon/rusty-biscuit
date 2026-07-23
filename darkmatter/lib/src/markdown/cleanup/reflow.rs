use biscuit_terminal::utils::UnicodeWidthStr;
use pulldown_cmark::{CowStr, Event};
use std::ops::Range;
use unicode_script::{Script, UnicodeScript};

use super::lists::list_marker_byte_width;

mod semantic;

pub(super) fn collapse_incidental_soft_break_events<'a>(
    content: &str,
    events: &[(Event<'a>, Range<usize>)],
) -> Vec<(Event<'a>, Range<usize>)> {
    let semantic = semantic::SoftBreakModel::from_events(content, events);
    let mut boundaries = semantic.boundaries.iter();

    events
        .iter()
        .map(|(event, span)| {
            if matches!(event, Event::SoftBreak) {
                let boundary = boundaries
                    .next()
                    .expect("semantic model records every soft-break event");
                if boundary.eligible {
                    let replacement = materialized_soft_break_separator(content, boundary)
                        .map_or_else(String::new, |separator| separator.to_string());
                    return (Event::Text(CowStr::from(replacement)), span.clone());
                }
            }
            (event.clone(), span.clone())
        })
        .collect()
}

fn materialized_soft_break_separator(
    content: &str,
    boundary: &semantic::SemanticSoftBreak,
) -> Option<char> {
    if boundary.join_separator.is_some() {
        return boundary.join_separator;
    }

    let previous_start = content[..boundary.soft_break.start]
        .rfind(['\n', '\r'])
        .map_or(0, |idx| idx + 1);
    let previous_line = &content[previous_start..boundary.soft_break.start];
    let previous_line = previous_line.strip_suffix(' ')?;
    let next_end = content[boundary.next_prefix.end..]
        .find(['\n', '\r'])
        .map_or(content.len(), |len| boundary.next_prefix.end + len);

    join_separator(
        previous_line,
        &content[boundary.next_prefix.end..next_end],
    )
}

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
    let semantic = semantic::SoftBreakModel::from_content(&normalized);
    let metadata = LineMetadata::scan(&lines[..line_count], 0, Some(&semantic.protected), true);
    // Boundaries arrive in event order, which is source order, so these edits are
    // sorted by `range.start` by construction. The binding is deliberately not
    // `mut`: the legacy pass below binary-searches it to decide ownership, and
    // appending to a vector mid-search destroys the sort order the search
    // requires. Collecting the legacy edits separately makes that class of bug
    // unrepresentable rather than merely avoided.
    let semantic_edits: Vec<StripEdit> = semantic
        .boundaries
        .iter()
        .filter(|boundary| boundary.eligible && boundary.item_depth > 0)
        .map(|boundary| StripEdit {
            range: boundary.soft_break.start..boundary.next_prefix.end,
            separator: boundary.join_separator,
        })
        .collect();
    debug_assert!(
        semantic_edits
            .windows(2)
            .all(|pair| pair[0].range.start < pair[1].range.start),
        "the ownership probe below requires strictly ascending semantic edit starts"
    );

    let mut legacy_edits: Vec<StripEdit> = Vec::new();
    let mut line_start = 0usize;
    for idx in 0..line_count {
        let line = lines[idx];
        if idx + 1 < line_count {
            let newline_offset = line_start + line.len();
            let owned_by_semantic = semantic_edits
                .binary_search_by_key(&newline_offset, |edit| edit.range.start)
                .is_ok();

            // The parser model is authoritative for list prose. The legacy
            // classifier remains the compatibility path for non-list prose.
            if !owned_by_semantic
                && let NewlineBoundary::Collapse { skip_next_prefix } =
                    newline_boundary(&metadata[idx], &metadata[idx + 1])
            {
                let next_line = lines[idx + 1];
                let next_body = if next_line.len() >= skip_next_prefix {
                    &next_line[skip_next_prefix..]
                } else {
                    next_line
                };
                legacy_edits.push(StripEdit {
                    range: newline_offset..newline_offset + 1 + skip_next_prefix,
                    separator: join_separator(line, next_body),
                });
            }
        }

        line_start += line.len();
        if idx + 1 < lines.len() {
            line_start += 1;
        }
    }

    let mut edits = semantic_edits;
    edits.extend(legacy_edits);
    apply_strip_edits(&normalized, edits)
}

#[derive(Debug, Clone)]
struct StripEdit {
    range: Range<usize>,
    separator: Option<char>,
}

/// Applies `edits` to `content` by walking them from the end of the document
/// into a pre-sized buffer, replacing each removed range with its separator.
///
/// ## Notes
///
/// Callers must supply non-overlapping edits; the `debug_assert` below is the
/// real contract, and a violation is a caller bug worth failing loudly on in
/// tests. An overlap that survives to a release build is nonetheless dropped
/// rather than applied, because slicing `content` backwards would panic — and a
/// formatter that aborts on a document it was asked to format is a worse outcome
/// than one collapse boundary going unapplied.
fn apply_strip_edits(content: &str, mut edits: Vec<StripEdit>) -> String {
    if edits.is_empty() {
        return content.to_string();
    }

    // Widest-first on ties so the retained edit is deterministic when the
    // overlap guard below has to choose between two edits at one offset.
    edits.sort_unstable_by(|left, right| {
        left.range
            .start
            .cmp(&right.range.start)
            .then_with(|| right.range.end.cmp(&left.range.end))
    });
    debug_assert!(edits.windows(2).all(|pair| pair[0].range.end <= pair[1].range.start));
    edits.dedup_by(|later, earlier| later.range.start < earlier.range.end);

    let removed = edits.iter().map(|edit| edit.range.len()).sum::<usize>();
    let inserted = edits
        .iter()
        .filter_map(|edit| edit.separator)
        .map(char::len_utf8)
        .sum::<usize>();
    let mut fragments = Vec::with_capacity(edits.len());
    let mut cursor = content.len();

    for edit in edits.iter().rev() {
        debug_assert!(edit.range.end <= cursor);
        fragments.push((edit.separator, &content[edit.range.end..cursor]));
        cursor = edit.range.start;
    }

    let mut result = String::with_capacity(content.len() - removed + inserted);
    result.push_str(&content[..cursor]);
    for (separator, tail) in fragments.into_iter().rev() {
        if let Some(separator) = separator {
            result.push(separator);
        }
        result.push_str(tail);
    }

    result
}

/// Reflows Markdown prose blocks to `width` display columns.
///
/// Protected Markdown blocks such as fences, indented code, tables, HTML
/// blocks, link-reference definitions, and transclusion directive lines are
/// emitted unchanged. Existing soft breaks inside list-item paragraphs are
/// unwrapped before list, task-list, and blockquote container prefixes are
/// applied to the newly wrapped lines.
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

    let semantic = semantic::ReflowMap::from_content(&normalized);
    let mut result = String::with_capacity(normalized.len());
    let mut cursor = 0usize;

    for block in &semantic.blocks {
        if block.span.start < cursor {
            continue;
        }
        result.push_str(&reflow_physical_lines(
            &normalized[cursor..block.span.start],
            cursor,
            width,
            &semantic.protected,
        ));
        result.push_str(&reflow_list_block(
            &normalized,
            block,
            &semantic.breaks,
            width,
        ));
        cursor = block.span.end;
    }
    result.push_str(&reflow_physical_lines(
        &normalized[cursor..],
        cursor,
        width,
        &semantic.protected,
    ));

    result
}

fn reflow_physical_lines(
    content: &str,
    source_offset: usize,
    width: usize,
    protected_spans: &[Range<usize>],
) -> String {
    if content.is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = content.split('\n').collect();
    let line_count = if lines.last() == Some(&"") {
        lines.len().saturating_sub(1)
    } else {
        lines.len()
    };
    let trailing_newline = lines.last() == Some(&"");
    let metadata = LineMetadata::scan(
        &lines[..line_count],
        source_offset,
        Some(protected_spans),
        false,
    );
    let mut result = String::with_capacity(content.len());

    for idx in 0..line_count {
        let line = lines[idx];
        if metadata[idx].blank || metadata[idx].protected {
            result.push_str(line);
        } else {
            result.push_str(&reflow_line(line, width));
        }

        if idx + 1 < line_count || trailing_newline {
            result.push('\n');
        }
    }

    result
}

fn reflow_list_block(
    content: &str,
    block: &semantic::ReflowBlock,
    breaks: &[semantic::ReflowBreak],
    width: usize,
) -> String {
    let first_line_end = content[block.span.clone()]
        .find('\n')
        .map_or(block.span.end, |len| block.span.start + len);
    let prefix = line_reflow_prefix(&content[block.span.start..first_line_end]);
    let mut first_prefix = prefix.first.as_str();
    let mut body = String::new();
    let mut rendered = Vec::new();
    let mut cursor = block.span.start + prefix.first_len;

    for boundary in breaks {
        if boundary.span.start < cursor || block.span.end < boundary.span.end {
            continue;
        }
        body.push_str(&content[cursor..boundary.span.start]);
        match boundary.kind {
            semantic::ReflowBreakKind::Soft(separator) => {
                if let Some(separator) = separator {
                    body.push(separator);
                }
            }
            semantic::ReflowBreakKind::Hard => {
                rendered.push(wrap_text_with_suffix(
                    body.trim(),
                    first_prefix,
                    &prefix.continuation,
                    hard_break_suffix(&content[boundary.span.clone()]),
                    width,
                ));
                body.clear();
                first_prefix = &prefix.continuation;
            }
        }
        cursor = boundary.next_prefix.end;
    }

    body.push_str(&content[cursor..block.span.end]);
    if !body.trim().is_empty() {
        rendered.push(wrap_text(
            body.trim(),
            first_prefix,
            &prefix.continuation,
            width,
        ));
    }

    rendered.join("\n")
}

fn hard_break_suffix(span: &str) -> &str {
    span.trim_end_matches(['\n', '\r'])
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
    fn scan(
        lines: &[&str],
        source_offset: usize,
        parsed_protected_spans: Option<&[Range<usize>]>,
        use_indented_code_fallback: bool,
    ) -> Vec<Self> {
        let mut metadata = Vec::with_capacity(lines.len());
        let mut fence: Option<String> = None;
        let mut html_block: Option<HtmlBlockEnd> = None;
        let mut inline_code_ticks: Option<usize> = None;
        let mut line_offset = source_offset;

        for line in lines {
            let trimmed = line.trim_start();
            let directive_trimmed = directive_trimmed(line);
            let blank = trimmed.is_empty();
            let was_in_fence = fence.is_some();
            let was_in_html = html_block.is_some();
            let starts_fence = fence.is_none().then(|| fence_marker(trimmed)).flatten();
            let starts_html = html_block.is_none().then(|| html_block_end(trimmed)).flatten();
            let list_item = list_marker_byte_width(trimmed).is_some();
            let line_span = line_offset..line_offset + line.len();
            let parsed_protected = parsed_protected_spans.is_some_and(|spans| {
                spans.iter().any(|span| {
                    span.start <= line_span.end && line_span.start < span.end
                })
            });
            let protected = was_in_fence
                || was_in_html
                || starts_fence.is_some()
                || starts_html.is_some()
                || parsed_protected
                || (use_indented_code_fallback && is_indented_code_line(line))
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

            line_offset += line.len() + 1;
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
    let outer_len = blockquote_prefix_len(line)
        .unwrap_or_else(|| line.len() - line.trim_start().len());
    let outer = &line[..outer_len];
    if let Some(marker_len) = list_marker_byte_width(&line[outer_len..]) {
        let first_len = outer_len + marker_len;
        let first = line[..first_len].to_string();
        return ReflowPrefix {
            continuation: format!(
                "{outer}{}",
                " ".repeat(UnicodeWidthStr::width(&line[outer_len..first_len]))
            ),
            first,
            first_len,
        };
    }

    ReflowPrefix {
        first: outer.to_string(),
        continuation: outer.to_string(),
        first_len: outer_len,
    }
}

fn wrap_text(text: &str, first_prefix: &str, continuation_prefix: &str, width: usize) -> String {
    wrap_text_with_suffix(text, first_prefix, continuation_prefix, "", width)
}

fn wrap_text_with_suffix(
    text: &str,
    first_prefix: &str,
    continuation_prefix: &str,
    suffix: &str,
    width: usize,
) -> String {
    let tokens = reflow_tokens(text);
    if tokens.is_empty() {
        return first_prefix.to_string();
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = UnicodeWidthStr::width(first_prefix);
    let mut prefix = first_prefix;

    let last_token = tokens.len().saturating_sub(1);
    for (idx, token) in tokens.into_iter().enumerate() {
        let token_width = UnicodeWidthStr::width(token.as_str());
        let separator_width = usize::from(!current.is_empty());
        let suffix_width = if idx == last_token {
            UnicodeWidthStr::width(suffix)
        } else {
            0
        };
        if !current.is_empty()
            && current_width + separator_width + token_width + suffix_width > width
        {
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
        lines.push(format!("{prefix}{current}{suffix}"));
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
