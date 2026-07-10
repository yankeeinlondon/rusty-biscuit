use super::*;
    // ==================== Bracket Escaping Tests ====================

    #[test]
    fn test_bracket_not_escaped_progress_indicator() {
        // Regression test: progress indicators should not be escaped
        let content =
            "- [0%] started\n- [25%] progress\n- [50%] halfway\n- [75%] almost\n- [100%] done";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("[0%]"),
            "Progress indicator [0%] should not be escaped, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\\[0%\\]"),
            "Progress indicator should not be escaped, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("[25%]"),
            "Progress indicator [25%] should not be escaped, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_bracket_not_escaped_bang_indicator() {
        // Regression test: [!] blocked indicator should not be escaped
        let content = "- [!] blocked task";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("[!]"),
            "Blocked indicator [!] should not be escaped, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\\[!\\]"),
            "Blocked indicator should not be escaped, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_bracket_preserved_in_actual_links() {
        // Actual links should still work
        let content = "Check out [this link](https://example.com)";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("[this link](https://example.com)"),
            "Links should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_task_list_checkboxes_not_escaped() {
        // Task list checkboxes should never be escaped
        let content = "- [x] done\n- [ ] pending";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("[x]"),
            "Checkbox [x] should not be escaped, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("[ ]"),
            "Checkbox [ ] should not be escaped, got:\n{}",
            cleaned
        );
    }

