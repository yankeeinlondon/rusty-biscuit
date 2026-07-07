//! Text truncation utilities for prompt reporting.

/// Truncates text using the `FrontBack` strategy.
///
/// Shows the first `front_count` lines, an `hr` marker, then the last
/// `back_count` lines. When a boundary line (the last of the front section
/// or the first of the back section) is blank, the section is advanced by
/// one line to ensure non-blank boundary lines.
///
/// If the total line count is less than or equal to `front_count + back_count`,
/// the original text is returned unchanged.
///
/// ## Examples
///
/// ```ignore
/// use claudine::render::prompt::truncate_front_back;
///
/// let text = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6";
/// let result = truncate_front_back(text, 2, 2);
/// assert!(result.contains("Line 1"));
/// assert!(result.contains("Line 2"));
/// assert!(result.contains("Line 5"));
/// assert!(result.contains("Line 6"));
/// assert!(result.contains("---")); // hr marker
/// ```
pub fn truncate_front_back(text: &str, front_count: usize, back_count: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len();

    if total <= front_count + back_count {
        return text.to_string();
    }

    let mut front_end = front_count;
    // Ensure the last line of the front section is not blank
    while front_end > 0 && front_end <= total && lines[front_end - 1].trim().is_empty() {
        front_end -= 1;
    }

    let mut back_start = total.saturating_sub(back_count);
    // Ensure the first line of the back section is not blank
    while back_start < total && lines[back_start].trim().is_empty() {
        back_start += 1;
    }

    // If advancing caused overlap, fall back to the original counts
    if front_end >= back_start {
        front_end = front_count.min(total);
        back_start = total.saturating_sub(back_count);
    }

    let front: Vec<&str> = lines[..front_end].to_vec();
    let back: Vec<&str> = lines[back_start..].to_vec();

    let mut result = String::new();
    for line in front {
        result.push_str(line);
        result.push('\n');
    }
    // The `---` marker is surrounded by blank lines so it reads as a visual
    // separator between the front and back sections. This runs on already
    // rendered rows, so the marker is literal text, not a re-parsed break.
    result.push_str("\n---\n\n");
    for line in back {
        result.push_str(line);
        result.push('\n');
    }

    // Remove trailing newline
    if result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Returns the first `count` lines of `text`, or all of `text` when it has
/// `count` or fewer lines.
pub fn truncate_head(text: &str, count: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= count {
        text.to_string()
    } else {
        lines[..count].join("\n")
    }
}

/// Removes the common leading-whitespace prefix shared by every non-blank
/// line (a "dedent").
///
/// Only the minimum indentation common to all non-blank lines is removed,
/// so *relative* indentation — nested Markdown lists, indented code blocks,
/// continuation lines — is preserved. Blank lines are emitted as-is and are
/// ignored when computing the common prefix.
///
/// The prefix is compared character-by-character, so tabs and spaces are
/// never treated as equivalent: a line indented with a tab and a line
/// indented with spaces share an empty common prefix and the text is
/// returned unchanged.
///
/// ## Examples
///
/// ```ignore
/// use claudine::render::prompt::strip_leading_whitespace;
///
/// // Common 4-space indent is removed; the inner 4-space indent of the
/// // sub-bullet is preserved relative to its parent.
/// let text = "    - parent\n        - child";
/// let result = strip_leading_whitespace(text);
/// assert_eq!(result, "- parent\n    - child");
/// ```
pub fn strip_leading_whitespace(text: &str) -> String {
    fn leading_whitespace(line: &str) -> &str {
        let end = line
            .char_indices()
            .find(|(_, c)| !c.is_whitespace())
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        &line[..end]
    }

    fn common_prefix_len(a: &str, b: &str) -> usize {
        a.char_indices()
            .zip(b.chars())
            .take_while(|((_, x), y)| x == y)
            .last()
            .map(|((i, c), _)| i + c.len_utf8())
            .unwrap_or(0)
    }

    let mut common: Option<&str> = None;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let leading = leading_whitespace(line);
        common = Some(match common {
            Some(prev) => &prev[..common_prefix_len(prev, leading)],
            None => leading,
        });
        if common.map(str::is_empty).unwrap_or(true) {
            break;
        }
    }

    let prefix_len = common.map(str::len).unwrap_or(0);
    if prefix_len == 0 {
        return text.to_string();
    }

    text.lines()
        .map(|line| {
            if line.trim().is_empty() {
                line
            } else {
                &line[prefix_len..]
            }
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- FrontBack truncation tests ---

    #[test]
    fn short_text_unchanged() {
        let text = "a\nb\nc";
        assert_eq!(truncate_front_back(text, 2, 2), text);
    }

    #[test]
    fn exact_boundary_no_truncation() {
        let text = "a\nb\nc\nd";
        assert_eq!(truncate_front_back(text, 2, 2), text);
    }

    #[test]
    fn basic_truncation() {
        // The truncation marker is surrounded by blank lines so CommonMark
        // parses it as a thematic break (HR) rather than a setext heading
        // underline. The marker therefore appears as `\n\n---\n\n`.
        let text = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10";
        let result = truncate_front_back(text, 3, 3);
        assert!(result.contains("1\n2\n3\n\n---\n\n8\n9\n10"), "{result:?}");
    }

    #[test]
    fn blank_last_front_line_gets_skipped() {
        let text = "Line 1\nLine 2\n\nLine 4\nLine 5\nLine 6\nLine 7";
        let result = truncate_front_back(text, 3, 2);
        // front_end should be 2 (skipping the blank line at index 2)
        assert!(
            result.contains("Line 1\nLine 2\n\n---\n\nLine 6\nLine 7"),
            "{result:?}"
        );
    }

    #[test]
    fn blank_first_back_line_gets_skipped() {
        let text = "Line 1\nLine 2\nLine 3\nLine 4\n\nLine 6\nLine 7";
        let result = truncate_front_back(text, 2, 3);
        // back_start should skip the blank at index 4, landing at 5
        assert!(
            result.contains("Line 1\nLine 2\n\n---\n\nLine 6\nLine 7"),
            "{result:?}"
        );
    }

    #[test]
    fn overlap_fallback() {
        // Front and back sections would overlap after blank-line advancement,
        // so the fallback should kick in. The marker is still surrounded by
        // blank lines (extra blanks here are harmless — they just collapse
        // visually).
        let text = "A\n\n\n\nB";
        let result = truncate_front_back(text, 2, 2);
        assert!(result.contains("---"), "{result:?}");
    }

    #[test]
    fn single_line_front_and_back() {
        let text = "a\nb\nc\nd\ne";
        let result = truncate_front_back(text, 1, 1);
        assert_eq!(result, "a\n\n---\n\ne");
    }

    #[test]
    fn front_count_zero() {
        let text = "a\nb\nc\nd\ne";
        let result = truncate_front_back(text, 0, 2);
        assert!(result.contains("---\n\nd\ne"), "{result:?}");
    }

    #[test]
    fn back_count_zero() {
        let text = "a\nb\nc\nd\ne";
        let result = truncate_front_back(text, 2, 0);
        assert!(result.contains("a\nb\n\n---"), "{result:?}");
    }

    #[test]
    fn empty_text() {
        assert_eq!(truncate_front_back("", 5, 5), "");
    }

    #[test]
    fn only_blank_lines() {
        let text = "\n\n\n\n\n";
        let result = truncate_front_back(text, 2, 2);
        // All blank, so front_end collapses to 0, back_start advances past all
        assert!(result.contains("---"));
    }

    // --- strip_leading_whitespace tests ---

    #[test]
    fn dedents_common_space_prefix() {
        // Common indent is two spaces; relative indent of the second line is
        // preserved.
        let text = "  hello\n    world";
        assert_eq!(strip_leading_whitespace(text), "hello\n  world");
    }

    #[test]
    fn dedents_common_tab_prefix() {
        let text = "\thello\n\t\tworld";
        assert_eq!(strip_leading_whitespace(text), "hello\n\tworld");
    }

    #[test]
    fn mixed_tab_and_space_leaves_text_unchanged() {
        // Tab and space share an empty common prefix; the input is returned
        // as-is so neither indentation style is silently rewritten.
        let text = "  hello\n\tworld";
        assert_eq!(strip_leading_whitespace(text), text);
    }

    #[test]
    fn unindented_line_blocks_dedent() {
        // The minimum common indent is empty when any non-blank line has no
        // leading whitespace.
        let text = "hello\n  world";
        assert_eq!(strip_leading_whitespace(text), text);
    }

    #[test]
    fn empty_lines_preserved() {
        // Blank lines are ignored when computing the common prefix and are
        // emitted unchanged.
        let text = "  hello\n\n  world";
        assert_eq!(strip_leading_whitespace(text), "hello\n\nworld");
    }

    #[test]
    fn preserves_nested_list_indentation() {
        // The regression that motivated dedent semantics: a top-level
        // four-space block indent should be stripped, but the bullet's
        // four-space sub-indent must survive so the Markdown parser still
        // sees a nested list.
        let text = "    - parent\n        - child\n        - sibling";
        let result = strip_leading_whitespace(text);
        assert_eq!(result, "- parent\n    - child\n    - sibling");
    }

    #[test]
    fn empty_string() {
        assert_eq!(strip_leading_whitespace(""), "");
    }
}
