use super::*;

// ==================== Emphasis (Regression) ====================
//
// Cleanup never restyles emphasis: authored delimiters come out as written.

#[test]
fn emphasis_spellings_are_unchanged() {
    for content in [
        "This has *asterisk italics* here.",
        "This has _underscore italics_ here.",
        "Mix of *asterisk* and _underscore_ emphasis.",
        "This has **asterisk bold** here.",
        "This has __underscore bold__ here.",
        "Mix of **asterisk bold** and __underscore bold__ text.",
        "This has **bold with _nested italics_** here.",
        "**_nested_** test",
        "***both bold and italic***",
        "__*mixed nesting*__",
    ] {
        assert_eq!(clean(content), content);
    }
}

#[test]
fn emphasis_markers_in_code_block_are_unchanged() {
    assert_eq!(clean("```\n*not* _emphasis_ **here**\n```"), "```text\n*not* _emphasis_ **here**\n```");
}

#[test]
fn intraword_underscores_are_unchanged() {
    for content in [
        "Testing word_with_underscores here.",
        "Testing _emphasis_ inside a word_with_underscores and **_nested_** styles.",
        "_first_ and snake_case_name and _second_",
    ] {
        assert_eq!(clean(content), content);
    }
}

#[test]
fn asterisk_list_marker_is_not_confused_with_emphasis() {
    let content = "* list item with _emphasis_";
    assert_eq!(clean(content), content);
}

#[test]
fn nested_asterisk_list_markers_are_not_confused_with_emphasis() {
    assert_eq!(
        clean("* outer _emphasized_\n  * inner _also emphasized_"),
        "* outer _emphasized_\n    * inner _also emphasized_"
    );
}

/// Each marker change starts a new list; `list-padding` separates adjacent
/// lists with one blank line.
#[test]
fn adjacent_lists_with_emphasis_keep_markers() {
    assert_eq!(
        clean("- dash with _em1_\n* asterisk with _em2_\n+ plus with _em3_"),
        "- dash with _em1_\n\n* asterisk with _em2_\n\n+ plus with _em3_"
    );
}
