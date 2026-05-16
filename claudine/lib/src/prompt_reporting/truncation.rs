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
/// ```
/// use claudine::prompt_reporting::truncate_front_back;
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
    // Surround `---` with blank lines. Without leading blank, CommonMark
    // interprets "<text>\n---" as a setext h2 heading; the blank line
    // forces it to parse as a thematic break (`HorizontalRule`) instead.
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

/// Strips all leading whitespace from each line of the text.
///
/// ## Examples
///
/// ```
/// use claudine::prompt_reporting::strip_leading_whitespace;
///
/// let text = "  Line 1\n    Line 2\n\tLine 3";
/// let result = strip_leading_whitespace(text);
/// assert_eq!(result, "Line 1\nLine 2\nLine 3");
/// ```
pub fn strip_leading_whitespace(text: &str) -> String {
    text.lines()
        .map(|line| line.trim_start())
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
        assert!(result.contains("Line 1\nLine 2\n\n---\n\nLine 6\nLine 7"), "{result:?}");
    }

    #[test]
    fn blank_first_back_line_gets_skipped() {
        let text = "Line 1\nLine 2\nLine 3\nLine 4\n\nLine 6\nLine 7";
        let result = truncate_front_back(text, 2, 3);
        // back_start should skip the blank at index 4, landing at 5
        assert!(result.contains("Line 1\nLine 2\n\n---\n\nLine 6\nLine 7"), "{result:?}");
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
    fn strips_spaces() {
        let text = "  hello\n    world";
        assert_eq!(strip_leading_whitespace(text), "hello\nworld");
    }

    #[test]
    fn strips_tabs() {
        let text = "\thello\n\t\tworld";
        assert_eq!(strip_leading_whitespace(text), "hello\nworld");
    }

    #[test]
    fn empty_lines_preserved() {
        let text = "hello\n\n  world";
        assert_eq!(strip_leading_whitespace(text), "hello\n\nworld");
    }

    #[test]
    fn empty_string() {
        assert_eq!(strip_leading_whitespace(""), "");
    }
}
