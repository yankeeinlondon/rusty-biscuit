use super::*;

/// Cleanup is a Markdown → Markdown transform, so an author's backslash
/// escapes must reach the next Markdown consumer intact. Spec:
/// `darkmatter/fixes/2026-08-27-preserve-backslash-escapes/spec.md`.

#[test]
fn punctuation_escapes_in_paragraph_text_survive() {
    for content in [
        "a\\-b", "a\\.b", "a\\_b", "a\\*b", "a\\[b\\]", "a\\\\b", "a\\#b", "2 \\* 3 \\+ 4",
    ] {
        let cleaned = cleanup_content(content);
        assert_eq!(cleaned.trim_end(), content, "escape lost for {content:?}");
    }
}

#[test]
fn windows_path_with_punctuation_initial_segment_survives() {
    let content = "PLAIN=C:\\Users\\x\\Temp\\.tmpAbc\\repo and C:\\repo\\.claudine\\x";
    let cleaned = cleanup_content(content);
    assert_eq!(cleaned.trim_end(), content);
}

#[test]
fn escapes_inside_code_are_not_doubled() {
    let content = "```text\nC:\\Temp\\.tmp \\* x\n```\n\nspan `a\\.b` here";
    let cleaned = cleanup_content(content);
    assert!(cleaned.contains("C:\\Temp\\.tmp \\* x"), "got:\n{cleaned}");
    assert!(cleaned.contains("`a\\.b`"), "got:\n{cleaned}");
    assert!(!cleaned.contains("\\\\."), "backslash doubled:\n{cleaned}");
}

#[test]
fn escapes_in_list_items_and_headings_survive() {
    let content = "# Title \\#1\n\n- item a\\-b\n- item `code` a\\.b";
    let cleaned = cleanup_content(content);
    assert!(cleaned.contains("Title \\#1"), "got:\n{cleaned}");
    assert!(cleaned.contains("item a\\-b"), "got:\n{cleaned}");
    assert!(cleaned.contains("a\\.b"), "got:\n{cleaned}");
}

#[test]
fn non_escape_backslashes_are_unchanged() {
    let content = "C:\\Users\\ken\\repo";
    assert_eq!(cleanup_content(content).trim_end(), content);
}

#[test]
fn hard_break_backslash_is_unaffected() {
    // cmark renders a hard break as two trailing spaces; the point is that the
    // break survives and no placeholder or stray backslash leaks.
    let content = "line one\\\nline two";
    let cleaned = cleanup_content(content);
    assert!(cleaned.contains("line one  \nline two"), "got:\n{cleaned}");
    assert!(!cleaned.contains('\u{E002}') && !cleaned.contains('\\'), "got:\n{cleaned}");
}
