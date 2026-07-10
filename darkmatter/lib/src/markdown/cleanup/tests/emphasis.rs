use super::*;
    // ==================== Emphasis/Italics Tests (Regression) ====================

    #[test]
    fn test_cleanup_preserves_asterisk_emphasis() {
        // Regression test: *asterisk* emphasis should NOT be converted to _underscore_
        let content = "This has *asterisk italics* here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("*asterisk italics*"),
            "Asterisk emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_underscore_emphasis() {
        // Regression test: _underscore_ emphasis should NOT be converted to *asterisk*
        let content = "This has _underscore italics_ here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("_underscore italics_"),
            "Underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_mixed_emphasis_styles() {
        // Both asterisk and underscore styles should be preserved in the same document
        let content = "Mix of *asterisk* and _underscore_ emphasis.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("*asterisk*"),
            "Asterisk emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_underscore_"),
            "Underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_asterisk_strong() {
        // **bold** should be preserved
        let content = "This has **asterisk bold** here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("**asterisk bold**"),
            "Asterisk strong should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_underscore_strong() {
        // __bold__ should be preserved
        let content = "This has __underscore bold__ here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("__underscore bold__"),
            "Underscore strong should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_mixed_strong_styles() {
        // Both asterisk and underscore bold should be preserved
        let content = "Mix of **asterisk bold** and __underscore bold__ text.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("**asterisk bold**"),
            "Asterisk strong should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("__underscore bold__"),
            "Underscore strong should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_nested_emphasis() {
        // Nested emphasis should preserve styles
        let content = "This has **bold with _nested italics_** here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("**"),
            "Bold markers should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_nested italics_"),
            "Nested underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_emphasis_in_code_blocks_unchanged() {
        // Emphasis markers inside code blocks should NOT be modified
        let content = "```\n*not* _emphasis_ **here**\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("*not*"),
            "Emphasis markers in code should be unchanged, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_emphasis_"),
            "Underscore in code should be unchanged, got:\n{}",
            cleaned
        );
    }

    // ==================== Regression Tests for Emphasis Restoration ====================

    #[test]
    fn test_regression_nested_emphasis_preserves_original_styles() {
        // Regression test: nested emphasis like **_text_** should preserve both styles
        // Previously, ***text*** was incorrectly output as ***text___ or similar
        let content = "**_nested_** test";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("**_nested_**"),
            "Nested emphasis should preserve both styles (** and _), got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_literal_underscores_not_converted() {
        // Regression test: literal underscores in words like word_with_underscores
        // should NOT be converted to asterisks or modified in any way
        let content = "Testing word_with_underscores here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("word_with_underscores"),
            "Literal underscores in words should not be modified, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_emphasis_and_literal_underscores_combined() {
        // Regression test: combination of emphasis and literal underscores
        // The underscore emphasis should be preserved AND the literal underscores
        // in word_with_underscores should remain unchanged
        let content = "Testing _emphasis_ inside a word_with_underscores and **_nested_** styles.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("_emphasis_"),
            "Underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("word_with_underscores"),
            "Literal underscores should not be modified, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("**_nested_**"),
            "Nested emphasis should preserve styles, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_triple_emphasis_markers() {
        // Regression test: ***text*** (strong + emphasis) should correctly restore styles
        let content = "***both bold and italic***";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("***both bold and italic***"),
            "Triple emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_mixed_nested_emphasis() {
        // Regression test: mixed nested styles like __*text*__
        let content = "__*mixed nesting*__";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("__*mixed nesting*__"),
            "Mixed nested emphasis should preserve styles, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_multiple_emphasis_with_underscores_in_text() {
        // Regression test: multiple emphasis with underscores in regular text
        let content = "_first_ and snake_case_name and _second_";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("_first_"),
            "First underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("snake_case_name"),
            "Literal underscores in snake_case should not be modified, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_second_"),
            "Second underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_list_marker_not_confused_with_emphasis() {
        // Regression test: asterisk list markers should NOT be treated as emphasis markers
        // Previously, `* list item with _emphasis_` was incorrectly converted to
        // `_ list item with _emphasis*` because the list marker `*` consumed an emphasis marker
        let content = "* list item with _emphasis_";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.starts_with("* "),
            "List marker should remain as asterisk, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_emphasis_"),
            "Emphasis should be preserved as underscore, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_nested_list_markers_not_confused_with_emphasis() {
        // Regression test: nested list markers should not be confused with emphasis
        let content = "* outer _emphasized_\n  * inner _also emphasized_";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("* outer"),
            "Outer list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("* inner") || cleaned.contains("  * inner"),
            "Inner list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_emphasized_"),
            "Outer emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_also emphasized_"),
            "Inner emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_multiple_list_types_with_emphasis() {
        // Regression test: different list markers with emphasis in same document
        let content = "- dash with _em1_\n* asterisk with _em2_\n+ plus with _em3_";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- dash with _em1_"),
            "Dash list with emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("* asterisk with _em2_"),
            "Asterisk list with emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ plus with _em3_"),
            "Plus list with emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

