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

/// Fixes list indentation in the output to match the original style.
///
/// `pulldown-cmark-to-cmark` uses 2-space indentation by default. This function
/// converts it to the specified indentation size (e.g., 4 spaces).
pub(super) fn fix_list_indentation(output: &mut String, target_indent: usize) {
    if target_indent == 2 {
        return; // Already correct
    }

    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut in_code_block = false;

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();

        // Track code blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Don't process code block content
        if in_code_block {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Check if this is a list item
        let is_list_item = trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || is_ordered_list_start(trimmed);

        if is_list_item {
            let current_indent = line.len() - trimmed.len();
            if current_indent > 0 {
                // Calculate nesting level (assuming 2-space input)
                let nesting_level = current_indent / 2;
                // Apply target indentation
                let new_indent = nesting_level * target_indent;
                result.push_str(&" ".repeat(new_indent));
                result.push_str(trimmed);
            } else {
                result.push_str(line);
            }
        } else {
            // For non-list content that's indented (like continuation text),
            // apply the same scaling
            let current_indent = line.len() - trimmed.len();
            if current_indent > 0 && current_indent % 2 == 0 {
                let nesting_level = current_indent / 2;
                let new_indent = nesting_level * target_indent;
                result.push_str(&" ".repeat(new_indent));
                result.push_str(trimmed);
            } else {
                result.push_str(line);
            }
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

/// Checks if a line starts with an ordered list marker (e.g., "1. " or "1) ").
fn is_ordered_list_start(line: &str) -> bool {
    let mut chars = line.chars().peekable();
    let mut has_digit = false;

    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            has_digit = true;
        } else if has_digit && (c == '.' || c == ')') {
            return chars.peek() == Some(&' ');
        } else {
            return false;
        }
    }
    false
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
