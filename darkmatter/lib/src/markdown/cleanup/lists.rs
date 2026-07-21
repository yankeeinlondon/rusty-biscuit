use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};
use std::ops::Range;

use super::ListSpacingMode;

pub(super) fn extract_list_markers(content: &str, events: &[(Event, Range<usize>)]) -> Vec<char> {
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
            Event::Start(Tag::Item) if list_type_stack.last() == Some(&true) => {
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
) -> Vec<ListItemContext> {
    let mut contexts = Vec::new();
    let mut list_depth = 0usize;
    let mut blockquote_depth = 0usize;

    for (event, _) in events {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth = list_depth.saturating_sub(1),
            Event::Start(Tag::BlockQuote(_)) => blockquote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                blockquote_depth = blockquote_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) => contexts.push(ListItemContext {
                list_depth,
                blockquote_depth,
            }),
            _ => {}
        }
    }

    contexts
}

/// Returns authored columns for subsequent paragraphs in unquoted list items.
pub(super) fn extract_unquoted_additional_paragraph_indents(
    content: &str,
    events: &[(Event, Range<usize>)],
) -> Vec<usize> {
    let mut indents = Vec::new();
    let mut paragraph_counts = Vec::<usize>::new();
    let mut blockquote_depth = 0usize;

    for (event, range) in events {
        match event {
            Event::Start(Tag::BlockQuote(_)) => blockquote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                blockquote_depth = blockquote_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) => paragraph_counts.push(0),
            Event::End(TagEnd::Item) => {
                paragraph_counts.pop();
            }
            Event::Start(Tag::Paragraph) if blockquote_depth == 0 => {
                let Some(count) = paragraph_counts.last_mut() else {
                    continue;
                };
                *count += 1;
                if *count > 1 {
                    let line_start = content[..range.start]
                        .rfind('\n')
                        .map_or(0, |newline| newline + 1);
                    let indent = content[line_start..]
                        .bytes()
                        .take_while(|byte| matches!(byte, b' ' | b'\t'))
                        .count();
                    indents.push(indent);
                }
            }
            _ => {}
        }
    }

    indents
}

/// Returns marker-line ordinals that the parser classified as indented code.
///
/// String cleanup passes use these ordinals to distinguish code content such
/// as `- literal` from actual list items without reparsing serialized output.
/// Fenced and blockquoted code is handled by the existing container-specific
/// paths and therefore does not participate in this top-level sequence.
pub(super) fn extract_unquoted_indented_code_marker_ordinals(
    events: &[(Event, Range<usize>)],
) -> Vec<usize> {
    let mut ordinals = Vec::new();
    let mut marker_ordinal = 0usize;
    let mut blockquote_depth = 0usize;
    let mut in_indented_code = false;

    for (event, _) in events {
        match event {
            Event::Start(Tag::BlockQuote(_)) => blockquote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                blockquote_depth = blockquote_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) if blockquote_depth == 0 => marker_ordinal += 1,
            Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) if blockquote_depth == 0 => {
                in_indented_code = true;
            }
            Event::End(TagEnd::CodeBlock) if in_indented_code => in_indented_code = false,
            Event::Text(text) if in_indented_code => {
                for line in text.lines() {
                    if list_marker_byte_width(line.trim_start()).is_some() {
                        ordinals.push(marker_ordinal);
                        marker_ordinal += 1;
                    }
                }
            }
            _ => {}
        }
    }

    ordinals
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
/// Parser-classified indented code marker lines are also protected so they do
/// not consume the marker belonging to a later real list item.
pub(super) fn restore_list_markers(
    output: &mut String,
    markers: &[char],
    indented_code_marker_ordinals: &[usize],
) {
    if markers.is_empty() {
        return;
    }

    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut marker_idx = 0;
    let mut marker_ordinal = 0usize;
    let mut protected_ordinals = indented_code_marker_ordinals.iter().copied().peekable();
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

        let protected = if !prefix.contains('>') && list_marker_byte_width(body).is_some() {
            let protected = protected_ordinals.peek() == Some(&marker_ordinal);
            if protected {
                protected_ordinals.next();
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
/// or more digits is paragraph prose, not a marker. Mirrors
/// `crate::markdown::cleanup::reflow::MAX_ORDERED_MARKER_DIGITS` — that helper
/// is private to a sibling module, so this constant is duplicated here rather
/// than widening the reflow module just to share one number.
const MAX_ORDERED_MARKER_DIGITS: usize = 9;

/// Returns the byte width of the list marker starting at `trimmed`, or `None`
/// when `trimmed` does not begin with a list marker.
///
/// The width includes the trailing space and any task-list bracket:
/// `"- "` = 2, `"* "` = 2, `"+ "` = 2, `"1. "` = 3, `"10. "` = 4,
/// `"- [ ] "` = 6, `"- [x] "` = 6. Digit runs longer than
/// `MAX_ORDERED_MARKER_DIGITS` are not markers.
fn list_marker_byte_width(trimmed: &str) -> Option<usize> {
    let marker = list_container_marker_byte_width(trimmed)?;
    Some(marker + task_marker_byte_width(&trimmed[marker..]).unwrap_or(0))
}

/// Width of the CommonMark list marker, excluding a GFM task checkbox.
///
/// A task checkbox is inline item content. It contributes to the visible body
/// prefix but not to the range in which a nested child marker is valid.
fn list_container_marker_byte_width(trimmed: &str) -> Option<usize> {
    let bytes = trimmed.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    if matches!(bytes[0], b'*' | b'-' | b'+') && bytes[1] == b' ' {
        return Some(2);
    }
    if bytes[0].is_ascii_digit() {
        for (idx, &byte) in bytes.iter().enumerate().skip(1) {
            // `idx` is also the length of the digit run scanned so far.
            if idx > MAX_ORDERED_MARKER_DIGITS {
                return None;
            }
            if byte == b'.' || byte == b')' {
                return (bytes.get(idx + 1) == Some(&b' ')).then_some(idx + 2);
            }
            if !byte.is_ascii_digit() {
                return None;
            }
        }
    }
    None
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
/// item/content columns. Requested columns are constrained to the parent's
/// CommonMark-valid child range. Blank lines do not close the stack because a
/// loose item may contain subsequent paragraphs and child lists; an actual
/// top-level block or parser-derived shallower item closes it instead.
/// Parser-classified indented code is emitted at the serializer's established
/// column and never consumes an item depth, even when its content resembles a
/// list marker.
pub(super) fn fix_list_indentation(
    output: &mut String,
    target_indent: usize,
    item_contexts: &[ListItemContext],
    additional_paragraph_indents: &[usize],
    indented_code_marker_ordinals: &[usize],
) {
    // (orig_item_col, orig_body_col, new_item_col, new_body_col,
    // new_child_content_col) per open list level. Task boxes contribute to
    // body columns but not to the CommonMark child-container column.
    let mut stack: Vec<(usize, usize, usize, usize, usize)> = Vec::new();

    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut in_code_block = false;
    let mut item_contexts = item_contexts.iter().copied().peekable();
    let mut additional_paragraph_indents = additional_paragraph_indents.iter().copied();
    let mut protected_ordinals = indented_code_marker_ordinals.iter().copied().peekable();
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

        if blockquote_depth == 0 && list_marker_byte_width(trimmed).is_some() {
            let protected = protected_ordinals.peek() == Some(&marker_ordinal);
            if protected {
                protected_ordinals.next();
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
            let new_item_col = if depth == 0 {
                0
            } else {
                let parent_child_content = stack
                    .get(depth - 1)
                    .map_or(target_indent * depth, |(_, _, _, _, child_content)| {
                        *child_content
                    });
                // A child marker is valid from its parent's content column
                // through three columns beyond it. Constraining the preferred
                // column prevents both wide parents and large requested steps
                // from turning a nested child into continuation prose or code.
                (target_indent * depth)
                    .clamp(parent_child_content, parent_child_content.saturating_add(3))
            };
            let body_marker_width = list_marker_byte_width(marker_body)
                .expect("container marker was already recognized");
            let new_body_col = new_item_col + body_marker_width;
            let new_child_content_col = new_item_col + container_marker_width;

            if blockquote_depth > 0 {
                for _ in 0..blockquote_depth {
                    result.push_str("> ");
                }
            }
            result.push_str(&" ".repeat(new_item_col));
            result.push_str(marker_body);
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
        } else if current_indent > 0 && line_follows_blank {
            // cmark gives subsequent item paragraphs their own block indent.
            // Pop a completed child block before preserving that paragraph's
            // serializer-chosen container column verbatim.
            while stack
                .last()
                .is_some_and(|(orig_item, _, _, _, _)| *orig_item >= current_indent)
            {
                stack.pop();
            }
            let indent = additional_paragraph_indents
                .next()
                .unwrap_or(current_indent);
            result.push_str(&" ".repeat(indent));
            result.push_str(trimmed);
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
    indented_code_marker_ordinals: &[usize],
) {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() < 2 {
        return;
    }
    let protected_marker_lines = protected_marker_lines(&lines, indented_code_marker_ordinals);

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

fn protected_marker_lines(lines: &[&str], protected_ordinals: &[usize]) -> Vec<bool> {
    let mut protected = vec![false; lines.len()];
    let mut ordinals = protected_ordinals.iter().copied().peekable();
    let mut marker_ordinal = 0usize;
    let mut open_fence: Option<char> = None;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let fence = match trimmed.chars().next() {
            Some('`') if trimmed.starts_with("```") => Some('`'),
            Some('~') if trimmed.starts_with("~~~") => Some('~'),
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
        if open_fence.is_some() || !is_list_item_start(trimmed) {
            continue;
        }

        if ordinals.peek() == Some(&marker_ordinal) {
            protected[idx] = true;
            ordinals.next();
        }
        marker_ordinal += 1;
    }

    protected
}

/// Returns `true` if the line starts a list item (ordered or unordered).
pub(super) fn is_list_item_start(trimmed: &str) -> bool {
    // Unordered: *, -, or + followed by space
    if trimmed.starts_with("* ") || trimmed.starts_with("- ") || trimmed.starts_with("+ ") {
        return true;
    }

    // Ordered: digits followed by . or ) and space
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return false;
    }
    for (i, &b) in bytes.iter().enumerate().skip(1) {
        if b == b'.' || b == b')' {
            return bytes.get(i + 1) == Some(&b' ');
        }
        if !b.is_ascii_digit() {
            return false;
        }
    }
    false
}

/// Returns `true` if the line is indented continuation content within a list.
fn is_list_continuation(line: &str) -> bool {
    let trimmed = line.trim_start();
    !trimmed.is_empty() && line.len() > trimmed.len() && !is_list_item_start(trimmed)
}
