use super::*;

/// Cleanup is a Markdown → Markdown transform, so an author's backslash
/// escapes must reach the next Markdown consumer intact.

#[test]
fn punctuation_escapes_in_paragraph_text_survive() {
    for content in [
        "a\\-b", "a\\.b", "a\\_b", "a\\*b", "a\\[b\\]", "a\\\\b", "a\\#b", "2 \\* 3 \\+ 4",
    ] {
        assert_eq!(clean(content), content, "escape lost for {content:?}");
    }
}

#[test]
fn windows_path_with_punctuation_initial_segment_survives() {
    let content = "PLAIN=C:\\Users\\x\\Temp\\.tmpAbc\\repo and C:\\repo\\.claudine\\x";
    assert_eq!(clean(content), content);
}

#[test]
fn escapes_inside_code_are_not_doubled() {
    let content = "```text\nC:\\Temp\\.tmp \\* x\n```\n\nspan `a\\.b` here";
    assert_eq!(clean(content), content);
}

#[test]
fn escapes_in_list_items_and_headings_survive() {
    let content = "# Title \\#1\n\n- item a\\-b\n- item `code` a\\.b";
    assert_eq!(clean(content), content);
}

#[test]
fn non_escape_backslashes_are_unchanged() {
    let content = "C:\\Users\\ken\\repo";
    assert_eq!(clean(content), content);
}

#[test]
fn hard_break_backslash_is_unchanged() {
    let content = "line one\\\nline two";
    assert_eq!(clean(content), content);
    assert_eq!(clean_unwrapped(content), content);
}
