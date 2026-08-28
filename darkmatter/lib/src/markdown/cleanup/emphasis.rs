use pulldown_cmark::{CowStr, Event, Tag, TagEnd};
use std::ops::Range;

use super::EmphasisStyle;


/// Gets the preferred emphasis style from the `PREFER_ITALICS` environment variable.
///
/// Valid values are `*` (asterisk) or `_` / `__` (underscore). Returns `None` if
/// the variable is not set or has an invalid value.
pub(super) fn get_preferred_emphasis_style() -> Option<EmphasisStyle> {
    std::env::var("PREFER_ITALICS")
        .ok()
        .and_then(|v| match v.trim() {
            "*" => Some(EmphasisStyle::Asterisk),
            "_" | "__" => Some(EmphasisStyle::Underscore),
            _ => None,
        })
}

// Placeholder characters for emphasis markers (private use area - won't be escaped by cmark)
const UNDERSCORE_EMPHASIS_PLACEHOLDER: char = '\u{E000}';
const UNDERSCORE_STRONG_PLACEHOLDER: &str = "\u{E000}\u{E000}";
const ASTERISK_EMPHASIS_PLACEHOLDER: char = '\u{E001}';
const ASTERISK_STRONG_PLACEHOLDER: &str = "\u{E001}\u{E001}";
/// Carries an author-written backslash escape (`\.`, `\*`, `\[`, …) through
/// `cmark` and the unescape passes; see `restore_backslash_placeholders`.
const BACKSLASH_PLACEHOLDER: char = '\u{E002}';

/// Transforms emphasis/strong events into literal text events to preserve original markers.
///
/// Instead of letting cmark normalize all emphasis to a single style, this function
/// replaces Start/End emphasis events with Text events containing placeholder characters.
/// After cmark renders, these placeholders are replaced with the actual markers.
///
/// We use Unicode private use area characters as placeholders because cmark won't escape them.
///
/// ## Parameters
/// - `standardize_emphasis`: If Some, all emphasis (italics) will use this style.
///   Strong (bold) is NEVER standardized - it always preserves the original style.
pub(super) fn preserve_original_emphasis<'a>(
    content: &str,
    events_with_ranges: &[(Event<'a>, Range<usize>)],
    standardize_emphasis: Option<EmphasisStyle>,
) -> Vec<Event<'a>> {
    let mut result = Vec::with_capacity(events_with_ranges.len());

    // Stack to track the markers for matching end tags
    // true = underscore style, false = asterisk style
    let mut style_stack: Vec<bool> = Vec::new();

    for (event, range) in events_with_ranges {
        match event {
            Event::Start(Tag::Emphasis) => {
                // Determine emphasis style: use standardized if specified, otherwise original
                let is_underscore = if let Some(style) = standardize_emphasis {
                    style == EmphasisStyle::Underscore
                } else {
                    range.start < content.len() && content[range.start..].starts_with('_')
                };
                style_stack.push(is_underscore);

                let placeholder = if is_underscore {
                    UNDERSCORE_EMPHASIS_PLACEHOLDER
                } else {
                    ASTERISK_EMPHASIS_PLACEHOLDER
                };
                result.push(Event::Text(CowStr::from(placeholder.to_string())));
            }
            Event::Start(Tag::Strong) => {
                // Strong (bold) ALWAYS preserves original style - never standardized
                let is_underscore =
                    range.start < content.len() && content[range.start..].starts_with('_');
                style_stack.push(is_underscore);

                let placeholder = if is_underscore {
                    UNDERSCORE_STRONG_PLACEHOLDER
                } else {
                    ASTERISK_STRONG_PLACEHOLDER
                };
                result.push(Event::Text(CowStr::from(placeholder)));
            }
            Event::End(TagEnd::Emphasis) => {
                let is_underscore = style_stack.pop().unwrap_or(false);
                let placeholder = if is_underscore {
                    UNDERSCORE_EMPHASIS_PLACEHOLDER
                } else {
                    ASTERISK_EMPHASIS_PLACEHOLDER
                };
                result.push(Event::Text(CowStr::from(placeholder.to_string())));
            }
            Event::End(TagEnd::Strong) => {
                let is_underscore = style_stack.pop().unwrap_or(false);
                let placeholder = if is_underscore {
                    UNDERSCORE_STRONG_PLACEHOLDER
                } else {
                    ASTERISK_STRONG_PLACEHOLDER
                };
                result.push(Event::Text(CowStr::from(placeholder)));
            }
            Event::Text(text) => match preserve_backslash_escapes(content, range, text) {
                Some(preserved) => result.push(Event::Text(CowStr::from(preserved))),
                None => result.push(event.clone()),
            },
            _ => {
                // Pass through all other events unchanged
                result.push(event.clone());
            }
        }
    }

    result
}

/// Replaces emphasis placeholders with actual markers.
pub(super) fn restore_emphasis_placeholders(output: &mut String) {
    // Fast path: no placeholders to restore, avoid the allocation entirely.
    if !output.contains(UNDERSCORE_EMPHASIS_PLACEHOLDER)
        && !output.contains(ASTERISK_EMPHASIS_PLACEHOLDER)
    {
        return;
    }

    // Single forward scan replacing all four placeholders at once. A doubled
    // placeholder run is the strong marker (`__` / `**`); a lone one is the
    // emphasis marker (`_` / `*`). Greedy left-to-right pairing is byte-identical
    // to the previous strong-then-emphasis `str::replace` sequence, but does it
    // in one pass with one allocation instead of four.
    let mut result = String::with_capacity(output.len());
    let mut chars = output.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            UNDERSCORE_EMPHASIS_PLACEHOLDER => {
                if chars.peek() == Some(&UNDERSCORE_EMPHASIS_PLACEHOLDER) {
                    chars.next();
                    result.push_str("__");
                } else {
                    result.push('_');
                }
            }
            ASTERISK_EMPHASIS_PLACEHOLDER => {
                if chars.peek() == Some(&ASTERISK_EMPHASIS_PLACEHOLDER) {
                    chars.next();
                    result.push_str("**");
                } else {
                    result.push('*');
                }
            }
            _ => result.push(c),
        }
    }
    *output = result;
}

/// Unescapes underscores and asterisks that cmark escaped in plain text.
///
/// cmark escapes `_` to `\_` and `*` to `\*` when they appear in text that could
/// potentially be interpreted as emphasis markers. However, since we handle all
/// emphasis via placeholders, these escapes are unnecessary.
///
/// This function removes the escape backslashes, but only outside of code blocks/spans.
pub(super) fn unescape_emphasis_chars(output: &mut String) {
    // Only process if there are escaped characters
    if !output.contains("\\_") && !output.contains("\\*") {
        return;
    }

    let mut result = String::with_capacity(output.len());
    let mut chars = output.chars().peekable();
    let mut in_code_block = false;
    let mut in_inline_code = false;

    while let Some(c) = chars.next() {
        // Track code blocks
        if c == '`' {
            let mut backtick_count = 1;
            while chars.peek() == Some(&'`') {
                chars.next();
                backtick_count += 1;
            }

            if backtick_count >= 3 {
                in_code_block = !in_code_block;
            } else if !in_code_block {
                // Toggle inline code (simplified - doesn't handle all edge cases)
                in_inline_code = !in_inline_code;
            }

            for _ in 0..backtick_count {
                result.push('`');
            }
            continue;
        }

        // Don't unescape inside code
        if in_code_block || in_inline_code {
            result.push(c);
            continue;
        }

        // Unescape \_ and \*
        if c == '\\' {
            match chars.peek() {
                Some('_') | Some('*') => {
                    // Skip the backslash, push the unescaped character
                    result.push(chars.next().unwrap());
                }
                _ => {
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }

    *output = result;
}


/// Re-attaches an author-written escape to a `Text` event, as
/// [`BACKSLASH_PLACEHOLDER`] + the event text, or returns `None` when the
/// event was not escaped.
///
/// The parser resolves CommonMark escapes before handing back `Event::Text`,
/// and its source range for the escaped character starts *after* the
/// backslash (`a\-b` yields `Text("-b")` at `2..4`), so the backslash lives in
/// the gap before the event. An event is an escape when its text starts with
/// ASCII punctuation, its range really does start with that character, and the
/// source immediately before the range ends in an odd run of backslashes (an
/// even run is escaped backslashes, `a\\-b`). Code blocks never match: their
/// text keeps its backslashes, so the character before a range is never a
/// stray `\`.
fn preserve_backslash_escapes(content: &str, range: &Range<usize>, text: &str) -> Option<String> {
    let first = text.chars().next()?;
    if !first.is_ascii_punctuation() {
        return None;
    }
    let before = content.get(..range.start)?;
    if !content.get(range.clone())?.starts_with(first) {
        return None;
    }
    let run = before.bytes().rev().take_while(|b| *b == b'\\').count();
    (run % 2 == 1).then(|| format!("{BACKSLASH_PLACEHOLDER}{text}"))
}

/// Restores author-written backslashes carried through by
/// `preserve_backslash_escapes`. Must run after `unescape_emphasis_chars` and
/// `unescape_brackets`, so an author's `\*` (now `␣*`, possibly re-escaped by
/// `cmark` to `␣\*` and reduced back to `␣*`) ends as `\*` rather than `*`.
pub(super) fn restore_backslash_placeholders(output: &mut String) {
    if output.contains(BACKSLASH_PLACEHOLDER) {
        *output = output.replace(BACKSLASH_PLACEHOLDER, "\\");
    }
}
