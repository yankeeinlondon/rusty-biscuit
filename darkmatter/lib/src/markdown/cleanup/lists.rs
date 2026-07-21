use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};
use std::ops::Range;

use super::ListSpacingMode;

pub(super) fn extract_list_markers(
    content: &str,
    events: &[(Event, Range<usize>)],
    opaque_body_lines: &[Range<usize>],
) -> Vec<char> {
    let mut markers = Vec::new();
    // Track whether the innermost list is unordered (true) or ordered (false)
    let mut list_type_stack: Vec<bool> = Vec::new();

    for (event, range) in events {
        match event {
            Event::Start(Tag::List(None)) => {
                list_type_stack.push(true); // unordered
            }
            Event::Start(Tag::List(Some(_))) => {
                list_type_stack.push(false); // ordered
            }
            Event::End(TagEnd::List(_)) => {
                list_type_stack.pop();
            }
            Event::Start(Tag::Item)
                if list_type_stack.last() == Some(&true)
                    && !range_starts_in(opaque_body_lines, range) =>
            {
                // Item inside an unordered list - extract its marker from source
                let source_slice = &content[range.start..];
                if let Some(marker) = find_list_marker(source_slice) {
                    markers.push(marker);
                } else {
                    markers.push('*');
                }
            }
            _ => {}
        }
    }

    markers
}

#[derive(Clone, Copy)]
pub(super) struct ListItemContext {
    list_depth: usize,
    blockquote_depth: usize,
}

/// Returns parser-derived container depths for every serialized list item.
///
/// `fix_list_indentation` consumes these contexts in serialization order. The
/// blockquote depth distinguishes quoted markers from marker-looking prose and
/// lets the normalizer rebuild list indentation after blockquote formatting
/// canonicalizes the quote prefix.
pub(super) fn extract_list_item_contexts(
    events: &[(Event, Range<usize>)],
    opaque_body_lines: &[Range<usize>],
) -> Vec<ListItemContext> {
    let mut contexts = Vec::new();
    let mut list_depth = 0usize;
    let mut blockquote_depth = 0usize;

    for (event, range) in events {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth = list_depth.saturating_sub(1),
            Event::Start(Tag::BlockQuote(_)) => blockquote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                blockquote_depth = blockquote_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) if !range_starts_in(opaque_body_lines, range) => {
                contexts.push(ListItemContext {
                    list_depth,
                    blockquote_depth,
                });
            }
            _ => {}
        }
    }

    contexts
}

#[derive(Clone, Copy)]
pub(super) struct AdditionalParagraphContext {
    list_depth: usize,
    blockquote_depth: usize,
    source_body_column: usize,
    source_item_content_column: usize,
}

/// Returns parser-derived ownership for subsequent list-item paragraphs.
///
/// The serializer removes the indentation that keeps a loose paragraph inside
/// a list item, especially after a blockquote prefix. Retaining the item and
/// quote depths alongside the authored body column lets the line normalizer
/// restore that ownership without parsing the serialized output.
pub(super) fn extract_additional_paragraph_contexts(
    content: &str,
    events: &[(Event, Range<usize>)],
    opaque_body_lines: &[Range<usize>],
) -> Vec<AdditionalParagraphContext> {
    let mut contexts = Vec::new();
    let mut item_sources = Vec::<Option<(usize, usize)>>::new();
    let mut list_depth = 0usize;
    let mut blockquote_depth = 0usize;

    for (event, range) in events {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth = list_depth.saturating_sub(1),
            Event::Start(Tag::BlockQuote(_)) => blockquote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                blockquote_depth = blockquote_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) => {
                if range_starts_in(opaque_body_lines, range) {
                    item_sources.push(None);
                    continue;
                }
                let line_start = content[..range.start]
                    .rfind('\n')
                    .map_or(0, |newline| newline + 1);
                let source_body = source_body(&content[line_start..], blockquote_depth);
                let item_column = source_body
                    .bytes()
                    .take_while(|byte| matches!(byte, b' ' | b'\t'))
                    .count();
                let marker_width = list_container_marker_byte_width(&source_body[item_column..])
                    .unwrap_or(0);
                item_sources.push(Some((0, item_column + marker_width)));
            }
            Event::End(TagEnd::Item) => {
                item_sources.pop();
            }
            Event::Start(Tag::Paragraph) => {
                let Some((count, source_item_content_column)) =
                    item_sources.last_mut().and_then(Option::as_mut)
                else {
                    continue;
                };
                *count += 1;
                if *count > 1 {
                    let line_start = content[..range.start]
                        .rfind('\n')
                        .map_or(0, |newline| newline + 1);
                    let source_line = &content[line_start..];
                    contexts.push(AdditionalParagraphContext {
                        list_depth,
                        blockquote_depth,
                        source_body_column: source_body_column(source_line, blockquote_depth),
                        source_item_content_column: *source_item_content_column,
                    });
                }
            }
            _ => {}
        }
    }

    contexts
}

fn source_body_column(line: &str, blockquote_depth: usize) -> usize {
    source_body(line, blockquote_depth)
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count()
}

fn source_body(line: &str, blockquote_depth: usize) -> &str {
    let bytes = line.as_bytes();
    let mut cursor = 0usize;

    for _ in 0..blockquote_depth {
        let indent_start = cursor;
        while cursor < bytes.len() && bytes[cursor] == b' ' && cursor - indent_start < 3 {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'>') {
            break;
        }
        cursor += 1;
        if bytes.get(cursor) == Some(&b' ') {
            cursor += 1;
        }
    }

    &line[cursor..]
}

#[derive(Clone, Copy)]
pub(super) struct ProtectedMarker {
    ordinal: usize,
    blockquote_depth: usize,
}

/// Returns parser-derived records for marker-looking lines outside Markdown items.
///
/// The records preserve serialization order and blockquote depth so HTML,
/// indented code, and Darkmatter opaque directive bodies cannot consume a
/// later item's parser-derived context or authored unordered marker.
pub(super) fn extract_indented_code_markers(
    events: &[(Event, Range<usize>)],
    opaque_body_lines: &[Range<usize>],
) -> Vec<ProtectedMarker> {
    let mut markers = Vec::new();
    let mut marker_ordinal = 0usize;
    let mut blockquote_depth = 0usize;
    let mut in_indented_code = false;

    for (event, range) in events {
        match event {
            Event::Start(Tag::BlockQuote(_)) => blockquote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                blockquote_depth = blockquote_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) => {
                if range_starts_in(opaque_body_lines, range) {
                    markers.push(ProtectedMarker {
                        ordinal: marker_ordinal,
                        blockquote_depth,
                    });
                }
                marker_ordinal += 1;
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                in_indented_code = true;
            }
            Event::End(TagEnd::CodeBlock) if in_indented_code => in_indented_code = false,
            Event::Text(text) if in_indented_code => {
                for line in text.lines() {
                    if list_marker_byte_width(line.trim_start()).is_some() {
                        markers.push(ProtectedMarker {
                            ordinal: marker_ordinal,
                            blockquote_depth,
                        });
                        marker_ordinal += 1;
                    }
                }
            }
            Event::Html(html) => {
                for line in html.lines() {
                    if list_marker_byte_width(line.trim_start()).is_some() {
                        markers.push(ProtectedMarker {
                            ordinal: marker_ordinal,
                            blockquote_depth,
                        });
                        marker_ordinal += 1;
                    }
                }
            }
            _ => {}
        }
    }

    markers
}

/// Locates physical lines inside opaque Darkmatter directive bodies.
///
/// Pulldown-cmark intentionally knows nothing about Darkmatter directives, so
/// a marker-looking shell command can otherwise become an ordinary Markdown
/// item. The ranges are an overlay on the existing cleanup parse, not another
/// Markdown parse.
pub(super) fn opaque_directive_body_lines(content: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut depth = 0usize;
    let mut offset = 0usize;

    for line_with_ending in content.split_inclusive('\n') {
        let line = line_with_ending.trim_end_matches(['\r', '\n']);
        let body = directive_body(line);
        let starts = body.starts_with("::shell-block");
        let ends = body.starts_with("::end-block");

        if depth > 0 && !ends {
            ranges.push(offset..offset + line.len());
        }
        if starts {
            depth += 1;
        } else if ends {
            depth = depth.saturating_sub(1);
        }

        offset += line_with_ending.len();
    }

    ranges
}

fn directive_body(line: &str) -> &str {
    let mut body = line.trim_start();
    while let Some(rest) = body.strip_prefix('>') {
        body = rest.trim_start();
    }
    body
}

fn range_starts_in(ranges: &[Range<usize>], range: &Range<usize>) -> bool {
    ranges
        .iter()
        .any(|protected| protected.start <= range.start && range.start < protected.end)
}

/// Finds the first list marker character (*, -, or +) in a source slice.
fn find_list_marker(source: &str) -> Option<char> {
    // Skip leading whitespace and look for the marker
    for c in source.chars() {
        match c {
            '*' | '-' | '+' => return Some(c),
            ' ' | '\t' | '\n' => continue,
            _ => break,
        }
    }
    None
}

/// Restores original list markers in the cleanup output.
///
/// `pulldown-cmark-to-cmark` normalizes every unordered list marker to `*`,
/// including items rendered inside a blockquote. This function walks the
/// rendered output line-by-line and rewrites each normalized `* ` body back to
/// the next authored marker recorded by `extract_list_markers`, preserving the
/// surrounding blockquote prefix and indentation verbatim.
///
/// Each visited unordered-list item — top-level or blockquoted — advances the
/// marker cursor by one, restoring the 1:1 correspondence between extracted
/// markers and rendered `* ` bodies even in mixed-marker documents.
///
/// Apparent bullets inside fenced code blocks (top-level or nested inside a
/// blockquote) are protected: fence state is tracked against the post-prefix
/// line body, and both backtick and tilde fences are recognized.
/// Parser-classified indented code and HTML marker lines are also protected,
/// as are item-looking lines in opaque Darkmatter directive bodies, so none
/// can consume the marker belonging to a later real list item.
pub(super) fn restore_list_markers(
    output: &mut String,
    markers: &[char],
    protected_markers: &[ProtectedMarker],
) {
    if markers.is_empty() {
        return;
    }

    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut marker_idx = 0;
    let mut marker_ordinal = 0usize;
    let mut protected_markers = protected_markers.iter().copied().peekable();
    // Open fence character (`` ` `` or `~`) when currently inside a fenced
    // code block; `None` outside.
    let mut open_fence: Option<char> = None;

    while let Some(line) = lines.next() {
        let (prefix, body) = split_rendered_line(line);

        // Fence detection runs against the post-prefix body so blockquoted
        // fences protect their contents. A fence line is three or more
        // repeated backtick or tilde characters.
        let body_fence_char = match body.chars().next() {
            Some('`') if body.starts_with("```") => Some('`'),
            Some('~') if body.starts_with("~~~") => Some('~'),
            _ => None,
        };

        if let Some(fc) = body_fence_char
            && (open_fence.is_none() || open_fence == Some(fc))
        {
            // Open the fence when outside, close it when the same character
            // reappears. A different fence character inside an open fence is
            // treated as code content below.
            open_fence = if open_fence == Some(fc) { None } else { Some(fc) };
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        if open_fence.is_some() {
            // Inside a fenced code block: emit verbatim.
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        let blockquote_depth = prefix.bytes().filter(|byte| *byte == b'>').count();
        let protected = if list_marker_byte_width(body).is_some() {
            let protected = protected_markers.peek().is_some_and(|marker| {
                marker.ordinal == marker_ordinal && marker.blockquote_depth == blockquote_depth
            });
            if protected {
                protected_markers.next();
            }
            marker_ordinal += 1;
            protected
        } else {
            false
        };
        if protected {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        if body.starts_with("* ") {
            let marker = markers.get(marker_idx).copied().unwrap_or('*');
            marker_idx += 1;

            result.push_str(prefix);
            result.push(marker);
            // Skip the normalized `*` byte and keep the rest of the body
            // (including the trailing space) verbatim.
            result.push_str(&body[1..]);
        } else {
            result.push_str(line);
        }

        if lines.peek().is_some() {
            result.push('\n');
        }
    }

    // Preserve trailing newline if original had one
    if output.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }

    *output = result;
}

/// Splits a rendered cleanup-output line into `(prefix, body)` for marker
/// restoration.
///
/// `restore_list_markers` runs after `fix_blockquote_formatting`, so any
/// blockquote prefix has already been normalized to the canonical `>` + space
/// shape. For a blockquote line, the returned `prefix` carries the exact bytes
/// needed to rebuild the line verbatim — leading indentation, every normalized
/// `>` segment, and the whitespace run between the final `>` and the body — so
/// the restored marker can drop straight into the body. For a non-blockquote
/// line, the helper returns the prior top-level shape: leading whitespace as
/// `prefix` and the first non-whitespace byte as the start of `body`.
fn split_rendered_line(line: &str) -> (&str, &str) {
    let bytes = line.as_bytes();
    let mut idx = 0;

    // Skip leading indentation (also folds the up-to-three-space CommonMark
    // blockquote indent that `fix_blockquote_formatting` may have left behind).
    while idx < bytes.len() && bytes[idx] == b' ' {
        idx += 1;
    }

    // Only treat the line as a blockquote when a `>` follows the leading
    // whitespace; otherwise return the top-level (whitespace) prefix.
    if bytes.get(idx) != Some(&b'>') {
        return (&line[..idx], &line[idx..]);
    }

    // Walk every normalized `>` segment. `fix_blockquote_formatting` emits
    // each `>` followed by exactly one space; consuming the full whitespace
    // run after each `>` is defensive but stays byte-exact for the canonical
    // shape and tolerates a stray double space without splitting short.
    while idx < bytes.len() && bytes[idx] == b'>' {
        idx += 1;
        while idx < bytes.len() && bytes[idx] == b' ' {
            idx += 1;
        }
    }

    (&line[..idx], &line[idx..])
}

/// Detects the authored indentation of the first unquoted nested list item.
///
/// Parser item events are authoritative, so marker-looking indented code does
/// not masquerade as the document's list-indentation style.
pub(super) fn detect_list_indentation(
    content: &str,
    events: &[(Event, Range<usize>)],
) -> usize {
    let mut list_depth = 0usize;
    let mut blockquote_depth = 0usize;

    for (event, range) in events {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth = list_depth.saturating_sub(1),
            Event::Start(Tag::BlockQuote(_)) => blockquote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                blockquote_depth = blockquote_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) if blockquote_depth == 0 && list_depth > 1 => {
                let line_start = content[..range.start]
                    .rfind('\n')
                    .map_or(0, |newline| newline + 1);
                return content[line_start..]
                    .bytes()
                    .take_while(|byte| matches!(byte, b' ' | b'\t'))
                    .count();
            }
            _ => {}
        }
    }

    2
}

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

/// Rescales nested list indentation in `output` to step by `target_indent`
/// columns per nesting level.
///
/// Parser-depth-based rather than `current_indent / 2` because the prior formula
/// silently destroyed list structure whenever a marker was wider than one
/// character. Under a `10. ` parent, for instance, `pulldown-cmark-to-cmark`
/// correctly indents a depth-1 child to column 4 (the parent's content
/// column); `4 / 2 = 2` then synthesized depth 2 and pushed the child to
/// column 8 under `--indent 4`, which pulldown-cmark reads as lazy
/// continuation prose on the next parse and absorbs into the parent.
///
/// The algorithm consumes parser-derived item depths in the same order cmark
/// serializes them, then tracks each open list level's original and rescaled
/// item/content columns. For an eight-column step, a parent that owns a nested
/// list uses CommonMark's maximum four-space marker padding. This makes the
/// exact child column valid even for narrow unordered, ordered, and task
/// markers without changing the parsed tree. Other requested columns remain
/// constrained to the parent's CommonMark-valid child range. Blank lines do
/// not close the stack because a loose item may contain subsequent paragraphs
/// and child lists; an actual top-level block or parser-derived shallower item
/// closes it instead.
/// Parser-classified indented code is emitted at the serializer's established
/// column and never consumes an item depth, even when its content resembles a
/// list marker.
pub(super) fn fix_list_indentation(
    output: &mut String,
    target_indent: usize,
    item_contexts: &[ListItemContext],
    additional_paragraph_contexts: &[AdditionalParagraphContext],
    protected_markers: &[ProtectedMarker],
) {
    // (orig_item_col, orig_body_col, new_item_col, new_body_col,
    // new_child_content_col) per open list level. Task boxes contribute to
    // body columns but not to the CommonMark child-container column.
    let mut stack: Vec<(usize, usize, usize, usize, usize)> = Vec::new();

    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut in_code_block = false;
    let mut item_contexts = item_contexts.iter().copied().peekable();
    let mut additional_paragraph_contexts =
        additional_paragraph_contexts.iter().copied().peekable();
    let mut protected_markers = protected_markers.iter().copied().peekable();
    let mut marker_ordinal = 0usize;
    let mut follows_blank = false;
    let mut active_blockquote_depth = 0usize;

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let current_indent = line.len() - trimmed.len();
        let (container_prefix, container_body) = split_rendered_line(line);
        let blockquote_depth = container_prefix.bytes().filter(|byte| *byte == b'>').count();
        let marker_body = if blockquote_depth > 0 {
            container_body
        } else {
            trimmed
        };

        if blockquote_depth > 0 && container_body.is_empty() {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            follows_blank = true;
            continue;
        }

        if trimmed.is_empty() {
            if current_indent > 0
                && let Some((_, _, _, new_body, _)) = stack.last().copied()
            {
                let base = (target_indent * stack.len()).max(new_body);
                result.push_str(&" ".repeat(current_indent.max(base)));
            } else {
                result.push_str(line);
            }
            if lines.peek().is_some() {
                result.push('\n');
            }
            follows_blank = true;
            continue;
        }
        let line_follows_blank = follows_blank;
        follows_blank = false;

        // Fenced code blocks and their contents are passed through verbatim.
        // The list-nesting stack is unchanged: cleanup has already serialized
        // a fence at the correct column for its containing item, and a
        // top-level fence does not reset anything.
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }
        if in_code_block {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        if list_marker_byte_width(marker_body).is_some() {
            let protected = protected_markers.peek().is_some_and(|marker| {
                marker.ordinal == marker_ordinal && marker.blockquote_depth == blockquote_depth
            });
            if protected {
                protected_markers.next();
            }
            marker_ordinal += 1;
            if protected {
                result.push_str(line);
                if lines.peek().is_some() {
                    result.push('\n');
                }
                continue;
            }
        }

        let item_context = item_contexts
            .peek()
            .filter(|context| context.blockquote_depth == blockquote_depth)
            .copied();
        if let Some((container_marker_width, context)) =
            list_container_marker_byte_width(marker_body).zip(item_context)
        {
            item_contexts.next();
            if active_blockquote_depth != blockquote_depth {
                stack.clear();
                active_blockquote_depth = blockquote_depth;
            }

            let depth = context.list_depth.saturating_sub(1);
            stack.truncate(depth);
            let owns_nested_list = item_contexts.peek().is_some_and(|next| {
                next.blockquote_depth == context.blockquote_depth
                    && next.list_depth > context.list_depth
            });
            let marker_token_width = list_container_marker_token_byte_width(marker_body)
                .expect("container marker was already recognized");
            let source_marker_padding = container_marker_width - marker_token_width;
            let marker_padding = if target_indent == 8 && owns_nested_list {
                4
            } else {
                source_marker_padding
            };
            let new_item_col = if depth == 0 {
                0
            } else {
                let parent_child_content = stack
                    .get(depth - 1)
                    .map_or(target_indent * depth, |(_, _, _, _, child_content)| {
                        *child_content
                    });
                // A child marker is valid from its parent's content column
                // through three columns beyond it. Constraining the requested
                // column prevents both wide parents and large requested steps
                // from turning a nested child into continuation prose or code.
                (target_indent * depth)
                    .clamp(parent_child_content, parent_child_content.saturating_add(3))
            };
            let body_marker_width = list_marker_byte_width(marker_body)
                .expect("container marker was already recognized")
                - source_marker_padding
                + marker_padding;
            let new_body_col = new_item_col + body_marker_width;
            let new_child_content_col =
                new_item_col + marker_token_width + marker_padding;

            if blockquote_depth > 0 {
                for _ in 0..blockquote_depth {
                    result.push_str("> ");
                }
            }
            result.push_str(&" ".repeat(new_item_col));
            result.push_str(&marker_body[..marker_token_width]);
            result.push_str(&" ".repeat(marker_padding));
            result.push_str(&marker_body[container_marker_width..]);
            stack.push((
                if blockquote_depth > 0 {
                    new_item_col
                } else {
                    current_indent
                },
                if blockquote_depth > 0 {
                    new_body_col
                } else {
                    current_indent + body_marker_width
                },
                new_item_col,
                new_body_col,
                new_child_content_col,
            ));
        } else if line_follows_blank
            && let Some(context) = additional_paragraph_contexts
                .peek()
                .filter(|context| context.blockquote_depth == blockquote_depth)
                .copied()
        {
            additional_paragraph_contexts.next();
            stack.truncate(context.list_depth);
            let relative_offset = context
                .source_body_column
                .saturating_sub(context.source_item_content_column);
            let continuation_column = stack
                .get(context.list_depth.saturating_sub(1))
                .map_or(context.source_body_column, |(_, _, _, _, new_content)| {
                    new_content + relative_offset
                });
            if blockquote_depth > 0 {
                for _ in 0..blockquote_depth {
                    result.push_str("> ");
                }
                result.push_str(&" ".repeat(continuation_column));
                result.push_str(container_body);
            } else {
                result.push_str(&" ".repeat(continuation_column));
                result.push_str(trimmed);
            }
        } else if current_indent > 0 {
            // Continuation prose or indented content belonging to the deepest
            // open item. A continuation line at column C cannot belong to an
            // item whose own marker was at column ≥ C, so those levels are
            // popped before the offset is computed.
            while stack
                .last()
                .is_some_and(|(orig_item, _, _, _, _)| *orig_item >= current_indent)
            {
                stack.pop();
            }

            if stack.is_empty() {
                // Top-level indented content that is not inside any list.
                result.push_str(line);
            } else {
                let (_, orig_body, _, new_body, _) = *stack.last().unwrap();
                // `target_indent * depth` mirrors how list-item markers are
                // rescaled, so a `- Alpha\n  beta` parent-then-continuation
                // at cmark's content column lands at `target_indent` rather
                // than the bare marker-width content column. The `max`
                // guarantees the result is at least the parent's rescaled
                // content column, which wide markers (`1234. `, content col
                // 6) need to stay CommonMark-valid.
                let depth = stack.len();
                let base = (target_indent * depth).max(new_body);
                // Preserve the relative offset from the containing item's
                // original content column: indented code (typically +4) keeps
                // its extra indentation, plain continuation prose keeps offset 0.
                let offset = current_indent.saturating_sub(orig_body);
                let new_indent = base + offset;
                result.push_str(&" ".repeat(new_indent));
                result.push_str(trimmed);
            }
        } else if blockquote_depth == 0 {
            // Top-level non-list line: clear the stack and pass through.
            stack.clear();
            active_blockquote_depth = 0;
            result.push_str(line);
        } else {
            // Quoted prose keeps the parser-derived list stack alive across
            // blank quote lines in loose items. Its serialized bytes are
            // otherwise already canonical.
            result.push_str(line);
        }

        if lines.peek().is_some() {
            result.push('\n');
        }
    }

    // Preserve trailing newline
    if output.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }

    *output = result;
}

/// Normalizes blank-line spacing between serialized list items.
///
/// Parser-classified indented code is excluded from the marker heuristic so
/// the blank line that establishes the code block remains intact.
pub(super) fn normalize_list_spacing(
    output: &mut String,
    mode: ListSpacingMode,
    protected_markers: &[ProtectedMarker],
) {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() < 2 {
        return;
    }
    let protected_marker_lines = protected_marker_lines(&lines, protected_markers);

    // Phase 1: strip blank lines between list items to get a clean baseline.
    // Only strip when both the previous non-blank line and the next non-blank
    // line are list item starts. Blank lines before continuation prose or
    // between a list item and non-item content are preserved.
    let mut stripped: Vec<(&str, bool)> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim().is_empty() && i > 0 {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }

            // Find the last non-blank line before the gap
            let prev_is_item =
                is_list_item_start(lines[i - 1].trim_start()) || is_list_continuation(lines[i - 1]);
            let next_is_item = j < lines.len()
                && !protected_marker_lines[j]
                && is_list_item_start(lines[j].trim_start());

            // Only strip if both sides are list items (not continuation prose)
            if prev_is_item && next_is_item {
                i = j;
                continue;
            }
        }
        stripped.push((lines[i], protected_marker_lines[i]));
        i += 1;
    }

    // Phase 2: insert blank lines based on mode.
    // Track indentation level of list items to detect transitions.
    let mut result = String::with_capacity(output.len() + 64);
    let mut prev_item_indent: Option<usize> = None;
    let mut in_list_run = false; // true when previous line(s) were list items
    let mut prev_was_blank = false;
    let mut had_continuation = false; // true when continuation content seen since last item

    for (idx, (line, protected_marker)) in stripped.iter().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let is_item = !protected_marker && is_list_item_start(trimmed);
        let is_cont = *protected_marker || is_list_continuation(line);

        if !prev_was_blank && idx > 0 {
            if is_item {
                // List item: check if we need a blank line before it
                let need_blank = match mode {
                    ListSpacingMode::Loose => true,
                    ListSpacingMode::Normal => {
                        if let Some(prev) = prev_item_indent {
                            // Descents and same-level siblings stay tight; loose items
                            // and shallower returns keep their separating blank.
                            indent < prev || had_continuation
                        } else {
                            false
                        }
                    }
                    ListSpacingMode::Compact => false,
                };
                if need_blank {
                    result.push('\n');
                }
            } else if is_cont && !trimmed.is_empty() && in_list_run {
                // Non-item continuation after a run of list items =
                // prose following a (sub-)list. Needs a blank line.
                if let Some(prev) = prev_item_indent
                    && indent <= prev
                {
                    result.push('\n');
                }
            }
        }

        if is_item {
            prev_item_indent = Some(indent);
            in_list_run = true;
            had_continuation = false;
        } else if !trimmed.is_empty() {
            if is_cont {
                had_continuation = true;
            }
            if !is_cont {
                prev_item_indent = None;
            }
            in_list_run = false;
        }

        prev_was_blank = trimmed.is_empty();
        result.push_str(line);
        result.push('\n');
    }

    if !output.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    *output = result;
}

pub(super) fn protected_marker_lines(
    lines: &[&str],
    protected_markers: &[ProtectedMarker],
) -> Vec<bool> {
    let mut protected = vec![false; lines.len()];
    let mut protected_markers = protected_markers.iter().copied().peekable();
    let mut marker_ordinal = 0usize;
    let mut open_fence: Option<char> = None;

    for (idx, line) in lines.iter().enumerate() {
        let (prefix, body) = split_rendered_line(line);
        let blockquote_depth = prefix.bytes().filter(|byte| *byte == b'>').count();
        let fence = match body.chars().next() {
            Some('`') if body.starts_with("```") => Some('`'),
            Some('~') if body.starts_with("~~~") => Some('~'),
            _ => None,
        };
        if let Some(fence) = fence
            && (open_fence.is_none() || open_fence == Some(fence))
        {
            open_fence = if open_fence == Some(fence) {
                None
            } else {
                Some(fence)
            };
            continue;
        }
        if open_fence.is_some() || !is_list_item_start(body) {
            continue;
        }

        if protected_markers.peek().is_some_and(|marker| {
            marker.ordinal == marker_ordinal && marker.blockquote_depth == blockquote_depth
        }) {
            protected[idx] = true;
            protected_markers.next();
        }
        marker_ordinal += 1;
    }

    protected
}

/// Returns `true` if the line starts a list item (ordered or unordered).
pub(super) fn is_list_item_start(trimmed: &str) -> bool {
    list_marker_byte_width(trimmed).is_some()
}

/// Returns `true` if the line is indented continuation content within a list.
fn is_list_continuation(line: &str) -> bool {
    let trimmed = line.trim_start();
    !trimmed.is_empty() && line.len() > trimmed.len() && !is_list_item_start(trimmed)
}
