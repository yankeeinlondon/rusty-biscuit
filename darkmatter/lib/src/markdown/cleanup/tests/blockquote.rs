use super::*;
    // ==================== Blockquote Formatting Tests ====================

    #[test]
    fn test_blockquote_no_leading_space() {
        // Regression test: blockquotes should not have leading space before >
        let content = "> Simple quote";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.starts_with("> "),
            "Blockquote should start with '> ', got:\n{:?}",
            cleaned
        );
        assert!(
            !cleaned.starts_with(" >"),
            "Blockquote should not have leading space, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_no_empty_first_line() {
        // Regression test: blockquotes should not have empty first line
        let content = "> Quote content";
        let cleaned = cleanup_content(content);

        // Should NOT have "> \n> " pattern (empty blockquote line)
        assert!(
            !cleaned.contains("> \n>"),
            "Blockquote should not have empty first line, got:\n{:?}",
            cleaned
        );
        // Should start directly with content
        assert!(
            cleaned.starts_with("> Quote"),
            "Blockquote should start with content, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_multiline() {
        // Incidental wrapping inside a blockquote should keep the quote prefix.
        let content = "> Line 1\n> Line 2\n> Line 3";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> Line 1 Line 2 Line 3"),
            "Blockquote prefix should be preserved after prose collapse, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_nested() {
        // Nested blockquotes should have single space between > markers
        let content = "> > Nested quote";
        let cleaned = cleanup_content(content);

        // Should be "> > " not ">  > " or " >  > "
        assert!(
            cleaned.starts_with("> > Nested"),
            "Nested blockquote should have single space between >, got:\n{:?}",
            cleaned
        );
        assert!(
            !cleaned.contains(">  >"),
            "Nested blockquote should not have double space, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_deeply_nested() {
        // Deeply nested blockquotes
        let content = "> > > Triple nested";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.starts_with("> > > Triple"),
            "Triple nested blockquote should have single spaces, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_after_header() {
        // Blockquotes following headers should be formatted correctly
        let content = "# Header\n\n> Quote after header";
        let cleaned = cleanup_content(content);

        // Should have blank line between header and quote, and proper formatting
        assert!(
            cleaned.contains("# Header\n\n> Quote"),
            "Blockquote after header should have blank line and proper format, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_long_content() {
        // Long blockquotes should not be mangled
        let content = "> Ut faucibus mauris mauris, sed tincidunt augue hendrerit eu. In ultrices ultrices commodo.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.starts_with("> Ut faucibus"),
            "Long blockquote should start correctly, got:\n{:?}",
            cleaned
        );
        assert!(
            cleaned.contains("commodo."),
            "Long blockquote content should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_preserves_content_spaces() {
        // Spaces in blockquote content should be preserved (not just prefix)
        let content = "> Code:   let x = 1";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> Code:   let"),
            "Spaces in blockquote content should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_preserves_blank_separator_before_attribution() {
        // A blank `>` line mid-blockquote is an intentional paragraph break
        // (e.g., separating content from an attribution line) and must survive cleanup.
        let content = "> Some quoted content\n>\n> — Wikipedia";
        let cleaned = cleanup_content(content);

        // The blank `>` line may gain a trailing space from fix_blockquote_line,
        // so check for both forms.
        let has_separator =
            cleaned.contains(">\n> — Wikipedia") || cleaned.contains("> \n> — Wikipedia");
        assert!(
            has_separator,
            "Blank blockquote separator before attribution should be preserved, got:\n{:?}",
            cleaned
        );
    }

