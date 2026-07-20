use super::*;

mod blockquote;
mod brackets;
mod emphasis;
mod lists;
mod parse_count;
mod reflow;
mod tables;

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

    // ==================== Blank Line Tests ====================

    #[test]
    fn test_cleanup_adds_blank_line_between_header_and_paragraph() {
        // Input has no blank line between header and paragraph
        let content = "# Title\nParagraph text";
        let cleaned = cleanup_content(content);

        // Should have exactly one blank line (two newlines) between header and paragraph
        assert!(
            cleaned.contains("# Title\n\nParagraph"),
            "Expected blank line between header and paragraph, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_adds_blank_line_between_consecutive_headers() {
        let content = "# Header 1\n## Header 2";
        let cleaned = cleanup_content(content);

        // Should have blank line between headers
        assert!(
            cleaned.contains("# Header 1\n\n## Header 2"),
            "Expected blank line between headers, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_adds_blank_line_after_code_block() {
        let content = "```rust\nfn main() {}\n```\nParagraph after";
        let cleaned = cleanup_content(content);

        // Should have blank line after code block
        assert!(
            cleaned.contains("```\n\nParagraph"),
            "Expected blank line after code block, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_adds_blank_line_after_list() {
        // Note: In CommonMark, "* Item\nParagraph" creates a "lazy" paragraph inside the list
        // We need an explicit blank line in input to separate list from paragraph
        let content = "* Item 1\n* Item 2\n\nParagraph after";
        let cleaned = cleanup_content(content);

        // Should have blank line after list
        assert!(
            cleaned.contains("Item 2\n\nParagraph"),
            "Expected blank line after list, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_adds_blank_line_after_blockquote() {
        // Note: In CommonMark, "> Quote\nParagraph" creates a "lazy" paragraph inside blockquote
        // We need an explicit blank line in input to separate blockquote from paragraph
        let content = "> Quote\n\nParagraph after";
        let cleaned = cleanup_content(content);

        // Should have blank line after blockquote
        assert!(
            cleaned.contains("Quote\n\nParagraph"),
            "Expected blank line after blockquote, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_existing_blank_lines() {
        let content = "# Header\n\nSome text\n\n## Subheader";
        let cleaned = cleanup_content(content);

        // Should preserve single blank lines (not double them up)
        assert_eq!(
            count_occurrences(&cleaned, "\n\n\n"),
            0,
            "Should not have triple newlines, got:\n{}",
            cleaned
        );
        assert!(cleaned.contains("# Header\n\nSome text"));
        assert!(cleaned.contains("Some text\n\n## Subheader"));
    }

    #[test]
    fn test_cleanup_does_not_add_excessive_blank_lines() {
        let content = "# Title\nParagraph 1\n\nParagraph 2";
        let cleaned = cleanup_content(content);

        // Count blank lines (consecutive \n\n)
        let blank_line_count = count_occurrences(&cleaned, "\n\n");

        // Should have exactly 2 blank lines: after title and between paragraphs
        assert_eq!(
            blank_line_count, 2,
            "Expected 2 blank lines, got {} in:\n{}",
            blank_line_count, cleaned
        );
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_cleanup_handles_empty_content() {
        let content = "";
        let cleaned = cleanup_content(content);
        assert_eq!(cleaned, "");
    }

    #[test]
    fn test_cleanup_handles_plain_text() {
        let content = "Just plain text";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("Just plain text"));
    }

    #[test]
    fn test_cleanup_ensures_single_trailing_newline() {
        let content = "# Title\n\nParagraph";
        let cleaned = cleanup_content(content);
        let trimmed = cleaned.trim_end_matches('\n');
        let trailing_newlines = cleaned.len() - trimmed.len();

        assert_eq!(
            trailing_newlines, 1,
            "Expected exactly one trailing newline, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_collapses_multiple_trailing_newlines_to_one() {
        let content = "# Title\n\nParagraph\n\n\n";
        let cleaned = cleanup_content(content);
        let trimmed = cleaned.trim_end_matches('\n');
        let trailing_newlines = cleaned.len() - trimmed.len();

        assert_eq!(
            trailing_newlines, 1,
            "Expected trailing newlines to collapse to one, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_handles_multiple_paragraphs() {
        let content = "Para 1\n\nPara 2\n\nPara 3";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("Para 1"));
        assert!(cleaned.contains("Para 2"));
        assert!(cleaned.contains("Para 3"));
    }

    #[test]
    fn test_cleanup_preserves_code_block_content() {
        let content = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("fn main()"));
        assert!(cleaned.contains("println!"));
    }

    #[test]
    fn test_cleanup_preserves_code_block_indentation() {
        // Regression test: indentation inside code blocks must be preserved
        let content =
            "```ts title=\"Greet Function\"\nfunction greet() {\n    console.log(\"hi\")\n}\n```";
        let cleaned = cleanup_content(content);

        // The 4-space indentation before console.log must be preserved
        assert!(
            cleaned.contains("    console.log"),
            "Indentation inside code block should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_code_block_indentation_simple() {
        // Test without attributes - just language
        let content = "```ts\nfunction greet() {\n    console.log(\"hi\")\n}\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("    console.log"),
            "Indentation inside simple code block should be preserved, got:\n{}",
            cleaned
        );
    }

    // ==================== Regression Tests ====================

    #[test]
    fn test_no_hardbreak_in_output() {
        // This is the main regression test for the bug
        let content = "# Header\n\nParagraph\n\n## Another Header";
        let cleaned = cleanup_content(content);

        // HardBreak would render as `\` or `<br>` - neither should appear
        assert!(
            !cleaned.contains("\\"),
            "Should not contain backslash (HardBreak), got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("<br"),
            "Should not contain <br> (HardBreak), got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_code_block_uses_three_backticks() {
        // Regression test: code blocks should use 3 backticks, not 4
        // pulldown-cmark-to-cmark defaults to 4 backticks (code_block_token_count)
        // which is non-standard and causes rendering issues
        let content = "```rust\nfn main() {}\n```";
        let cleaned = cleanup_content(content);

        // Should use exactly 3 backticks for fence
        assert!(
            cleaned.contains("```rust"),
            "Code blocks should start with 3 backticks, got:\n{}",
            cleaned
        );
        // Should NOT have 4 backticks (the buggy default)
        assert!(
            !cleaned.contains("````"),
            "Code blocks should not use 4 backticks, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_code_block_without_language_gets_text_language() {
        // Regression test: code blocks without language should get "text" added
        let content = "```\nsome code\n```";
        let cleaned = cleanup_content(content);

        // Should have "text" as language
        assert!(
            cleaned.starts_with("```text\n") || cleaned.contains("\n```text\n"),
            "Code blocks without language should get 'text' as language, got:\n{}",
            cleaned
        );
        // Should NOT have 4 backticks
        assert!(
            !cleaned.contains("````"),
            "Code blocks should not use 4 backticks, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_code_block_preserves_existing_language() {
        // Ensure code blocks with language are not affected
        let content = "```rust\nfn main() {}\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("```rust\n"),
            "Existing language should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_multiple_code_blocks_without_language() {
        // Multiple empty code blocks should all get "text" language
        let content = "```\nfirst\n```\n\n```\nsecond\n```";
        let cleaned = cleanup_content(content);

        // Count occurrences of "```text"
        let text_count = cleaned.matches("```text").count();
        assert_eq!(
            text_count, 2,
            "Both code blocks should get 'text' language, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_indented_code_blocks_unchanged() {
        // Indented code blocks should remain unchanged (they don't have language specifiers)
        let content = "    indented code\n    more code";
        let cleaned = cleanup_content(content);

        // Should preserve indented code block
        assert!(
            cleaned.contains("indented code"),
            "Indented code should be preserved, got:\n{}",
            cleaned
        );
    }

    // ==================== Smart Quotes Tests (Regression) ====================

    #[test]
    fn test_cleanup_preserves_straight_double_quotes() {
        // Regression test: Normal quotes should NOT be converted to smart quotes
        let content = r#"He said "hello" and "goodbye"."#;
        let cleaned = cleanup_content(content);

        // Should contain straight quotes (ASCII 0x22)
        assert!(
            cleaned.contains('"'),
            "Straight double quotes should be preserved, got:\n{}",
            cleaned
        );
        // Should NOT contain smart quotes (U+201C and U+201D - left and right double quotes)
        assert!(
            !cleaned.contains('\u{201C}') && !cleaned.contains('\u{201D}'),
            "Should not contain smart/curly quotes, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_straight_single_quotes() {
        // Regression test: Single quotes/apostrophes should NOT be converted
        let content = "It's a test and 'quoted text' here.";
        let cleaned = cleanup_content(content);

        // Should contain straight single quote (ASCII 0x27)
        assert!(
            cleaned.contains("'"),
            "Straight single quotes should be preserved, got:\n{}",
            cleaned
        );
        // Should NOT contain smart single quotes (U+2018 and U+2019)
        assert!(
            !cleaned.contains('\u{2018}') && !cleaned.contains('\u{2019}'),
            "Should not contain smart/curly single quotes, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_quotes_in_code() {
        // Quotes inside code blocks should definitely be preserved
        let content = "```\nlet s = \"hello\";\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("\"hello\""),
            "Quotes in code should be preserved, got:\n{}",
            cleaned
        );
    }
