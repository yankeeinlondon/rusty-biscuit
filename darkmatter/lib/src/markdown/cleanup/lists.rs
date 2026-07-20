use pulldown_cmark::{Event, Tag, TagEnd};
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
pub(super) fn restore_list_markers(output: &mut String, markers: &[char]) {
    if markers.is_empty() {
        return;
    }

    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut marker_idx = 0;
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

/// Fixes blockquote formatting issues introduced by pulldown-cmark-to-cmark v18.
///
/// The library adds:
/// 1. A leading space before `>` (e.g., ` > ` instead of `> `)
/// 2. An empty blockquote line at the start of each blockquote
/// 3. Extra spaces in nested blockquotes (e.g., `>  > ` instead of `> > `)
///
/// This function corrects these issues to produce standard markdown.
pub(super) fn detect_list_indentation(content: &str) -> usize {
    let mut in_code_block = false;

    for line in content.lines() {
        let trimmed = line.trim_start();

        // Skip code blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        // Look for indented list items
        let indent = line.len() - trimmed.len();
        if indent > 0
            && (trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ "))
        {
            // Found a nested list item - return its indentation
            return indent;
        }

        // Also check for numbered lists
        if indent > 0 {
            let mut chars = trimmed.chars().peekable();
            let mut is_numbered = false;
            while let Some(c) = chars.next() {
                if c.is_ascii_digit() {
                    continue;
                } else if (c == '.' || c == ')') && chars.peek() == Some(&' ') {
                    is_numbered = true;
                    break;
                } else {
                    break;
                }
            }
            if is_numbered {
                return indent;
            }
        }
    }

    // Default to 2 if no nested lists found
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
    let bytes = trimmed.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    if matches!(bytes[0], b'*' | b'-' | b'+') && bytes[1] == b' ' {
        let marker = 2;
        return Some(marker + task_marker_byte_width(&trimmed[marker..]).unwrap_or(0));
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
/// Stack-based rather than `current_indent / 2` because the prior formula
/// silently destroyed list structure whenever a marker was wider than one
/// character. Under a `10. ` parent, for instance, `pulldown-cmark-to-cmark`
/// correctly indents a depth-1 child to column 4 (the parent's content
/// column); `4 / 2 = 2` then synthesized depth 2 and pushed the child to
/// column 8 under `--indent 4`, which pulldown-cmark reads as lazy
/// continuation prose on the next parse and absorbs into the parent.
///
/// The algorithm tracks each open list level's original and rescaled
/// item/content columns. A child is recognized as depth N+1 only when its
/// marker column is at least its parent's content column (the CommonMark
/// rule), so depth is derived from actual nesting rather than from the
/// absolute column divided by two. Requested columns are constrained to the
/// parent's CommonMark-valid child range, and continuation prose or indented
/// code preserves its offset relative to the rescaled content column.
pub(super) fn fix_list_indentation(output: &mut String, target_indent: usize) {
    if target_indent == 2 {
        return; // cmark's default for narrow markers is already 2-space.
    }

    // (orig_item_col, orig_content_col, new_item_col, new_content_col) per
    // open list level. `orig_*` is the column in the cmark-serialized input;
    // `new_*` is the rescaled column written to the output.
    let mut stack: Vec<(usize, usize, usize, usize)> = Vec::new();

    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut in_code_block = false;

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();

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

        let current_indent = line.len() - trimmed.len();

        if let Some(marker_width) = list_marker_byte_width(trimmed) {
            // Pop every level whose item column is at or past the current
            // column: those are siblings or shallower levels we have exited.
            while stack
                .last()
                .is_some_and(|(orig_item, _, _, _)| *orig_item >= current_indent)
            {
                stack.pop();
            }

            let depth = stack.len();
            let new_item_col = if depth == 0 {
                0
            } else {
                let (_, _, _, parent_new_content) = stack[depth - 1];
                // A child marker is valid from its parent's content column
                // through three columns beyond it. Constraining the preferred
                // column prevents both wide parents and large requested steps
                // from turning a nested child into continuation prose or code.
                (target_indent * depth)
                    .clamp(parent_new_content, parent_new_content.saturating_add(3))
            };
            let new_content_col = new_item_col + marker_width;

            result.push_str(&" ".repeat(new_item_col));
            result.push_str(trimmed);
            stack.push((
                current_indent,
                current_indent + marker_width,
                new_item_col,
                new_content_col,
            ));
        } else if current_indent > 0 {
            // Continuation prose or indented content belonging to the deepest
            // open item. A continuation line at column C cannot belong to an
            // item whose own marker was at column ≥ C, so those levels are
            // popped before the offset is computed.
            while stack
                .last()
                .is_some_and(|(orig_item, _, _, _)| *orig_item >= current_indent)
            {
                stack.pop();
            }

            if stack.is_empty() {
                // Top-level indented content that is not inside any list.
                result.push_str(line);
            } else {
                let (_, orig_content, _, new_content) = *stack.last().unwrap();
                // `target_indent * depth` mirrors how list-item markers are
                // rescaled, so a `- Alpha\n  beta` parent-then-continuation
                // at cmark's content column lands at `target_indent` rather
                // than the bare marker-width content column. The `max`
                // guarantees the result is at least the parent's rescaled
                // content column, which wide markers (`1234. `, content col
                // 6) need to stay CommonMark-valid.
                let depth = stack.len();
                let base = (target_indent * depth).max(new_content);
                // Preserve the relative offset from the containing item's
                // original content column: indented code (typically +4) keeps
                // its extra indentation, plain continuation prose keeps offset 0.
                let offset = current_indent.saturating_sub(orig_content);
                let new_indent = base + offset;
                result.push_str(&" ".repeat(new_indent));
                result.push_str(trimmed);
            }
        } else {
            // Top-level non-list line: clear the stack and pass through.
            stack.clear();
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

/// Unescapes unnecessarily escaped brackets in the output.
///
/// `pulldown-cmark-to-cmark` escapes `[` and `]` characters that could potentially
/// be interpreted as link syntax. This function unescapes patterns like `\[0%\]`
/// that are clearly not links (no `](` following them).
pub(super) fn normalize_list_spacing(output: &mut String, mode: ListSpacingMode) {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() < 2 {
        return;
    }

    // Phase 1: strip blank lines between list items to get a clean baseline.
    // Only strip when both the previous non-blank line and the next non-blank
    // line are list item starts. Blank lines before continuation prose or
    // between a list item and non-item content are preserved.
    let mut stripped: Vec<&str> = Vec::with_capacity(lines.len());
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
            let next_is_item = j < lines.len() && is_list_item_start(lines[j].trim_start());

            // Only strip if both sides are list items (not continuation prose)
            if prev_is_item && next_is_item {
                i = j;
                continue;
            }
        }
        stripped.push(lines[i]);
        i += 1;
    }

    // Phase 2: insert blank lines based on mode.
    // Track indentation level of list items to detect transitions.
    let mut result = String::with_capacity(output.len() + 64);
    let mut prev_item_indent: Option<usize> = None;
    let mut in_list_run = false; // true when previous line(s) were list items
    let mut prev_was_blank = false;
    let mut had_continuation = false; // true when continuation content seen since last item

    for (idx, line) in stripped.iter().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let is_item = is_list_item_start(trimmed);
        let is_cont = is_list_continuation(line);

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
