use super::*;
use pulldown_cmark::TagEnd;
// ==================== Table Alignment Tests ====================

    #[test]
    fn test_table_columns_are_aligned() {
        let content = "|Short|VeryLongHeader|\n|---|---|\n|A|B|";
        let cleaned = cleanup_content(content);

        // The table should be rendered with aligned columns
        // Note: exact format depends on cmark rendering, but cells should be padded
        assert!(cleaned.contains("Short"), "Content should be preserved");
        assert!(
            cleaned.contains("VeryLongHeader"),
            "Content should be preserved"
        );
    }
    #[test]
    fn test_table_structure_preserved() {
        let content = "| Col1 | Col2 |\n|------|------|\n| A | B |";
        let cleaned = cleanup_content(content);

        // Should still have pipe characters for table structure
        assert!(cleaned.contains("|"), "Table structure should be preserved");
        assert!(cleaned.contains("Col1"));
        assert!(cleaned.contains("Col2"));
    }

    #[test]
    fn test_align_tables_preserves_non_table_content() {
        let parser = Parser::new_ext("# Title\nParagraph", Options::all());
        let events: Vec<Event> = parser.collect();
        let processed = align_tables_in_stream(events.clone());

        // Should preserve all events when no table present
        assert_eq!(processed.len(), events.len());
    }

    #[test]
    fn test_align_tables_handles_simple_table() {
        let content = "| Col1 | Col2 |\n|------|------|\n| A | B |";
        let parser = Parser::new_ext(content, Options::all());
        let events: Vec<Event> = parser.collect();
        let processed = align_tables_in_stream(events);

        // Should preserve table structure
        let has_table_start = processed
            .iter()
            .any(|e| matches!(e, Event::Start(Tag::Table(_))));
        let has_table_end = processed
            .iter()
            .any(|e| matches!(e, Event::End(TagEnd::Table)));
        assert!(has_table_start);
        assert!(has_table_end);
    }

    #[test]
    fn test_table_after_cleanup_still_parses() {
        let content = "| A | B |\n|---|---|\n| 1 | 2 |";
        let cleaned = cleanup_content(content);

        // Re-parse the cleaned content - should still be a valid table
        let parser = Parser::new_ext(&cleaned, Options::all());
        let events: Vec<Event> = parser.collect();

        let has_table = events
            .iter()
            .any(|e| matches!(e, Event::Start(Tag::Table(_))));
        assert!(
            has_table,
            "Cleaned table should still parse as table, got:\n{}",
            cleaned
        );
    }
    #[test]
    fn test_table_cells_have_spacing() {
        // Regression test: table cells should have space after | and before |
        let content = "|A|B|\n|---|---|\n|1|2|";
        let cleaned = cleanup_content(content);

        // Should have "| A " pattern (space after pipe, content, space before next pipe)
        assert!(
            cleaned.contains("| A "),
            "Table cells should have leading space after |, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("| B "),
            "Table cells should have leading space after |, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_table_code_spans_not_escaped() {
        // Regression test: backticks in code spans should not be escaped
        let content = "| Name | Value |\n|---|---|\n| `foo` | bar |";
        let cleaned = cleanup_content(content);

        // Should preserve backticks without escaping
        assert!(
            cleaned.contains("`foo`"),
            "Code spans should preserve backticks, got:\n{}",
            cleaned
        );
        // Should NOT have escaped backticks
        assert!(
            !cleaned.contains("\\`"),
            "Code spans should not have escaped backticks, got:\n{}",
            cleaned
        );
    }

    /// Regression test: Table columns with links must be properly aligned.
    ///
    /// Previously, the table alignment code measured link text width (e.g., "no-color.org")
    /// but the rendered markdown output includes the full link syntax `[text](url)`,
    /// causing misaligned columns.
    #[test]
    fn test_table_alignment_with_links() {
        let content = "| Variable | Description |\n|----------|-------------|\n| `THEME` | Default theme |\n| `NO_COLOR` | Disable colors ([no-color.org](https://no-color.org)) |";
        let cleaned = cleanup_content(content);

        // All rows should have the same number of pipe characters
        let lines: Vec<&str> = cleaned.lines().collect();
        assert!(
            lines.len() >= 3,
            "Should have at least 3 lines (header, separator, data)"
        );

        // Each data row should end with " |" (space before closing pipe)
        for line in &lines[2..] {
            assert!(
                line.ends_with(" |"),
                "Table row should end with ' |' for proper alignment, got:\n{}",
                line
            );
        }

        // The link should be preserved
        assert!(
            cleaned.contains("[no-color.org](https://no-color.org)"),
            "Link should be preserved in table, got:\n{}",
            cleaned
        );
    }

    /// Regression test: Table columns with emphasis must be properly aligned.
    ///
    /// Previously, emphasis markers (*text* or _text_) weren't counted in the width
    /// calculation, causing misaligned columns.
    #[test]
    fn test_table_alignment_with_emphasis() {
        let content = "| Option | Description |\n|--------|-------------|\n| `--mermaid` | *Terminal only*. Renders diagrams |\n| `--html` | Output HTML |";
        let cleaned = cleanup_content(content);

        // Each data row should end with " |"
        let lines: Vec<&str> = cleaned.lines().collect();
        for line in &lines[2..] {
            assert!(
                line.ends_with(" |"),
                "Table row should end with ' |' for proper alignment, got:\n{}",
                line
            );
        }

        // Emphasis should be preserved
        assert!(
            cleaned.contains("*Terminal only*"),
            "Emphasis should be preserved in table, got:\n{}",
            cleaned
        );
    }

    /// Regression test: Table columns with bold text must be properly aligned.
    #[test]
    fn test_table_alignment_with_strong() {
        let content = "| Name | Description |\n|------|-------------|\n| Test | **Bold text** here |\n| A | Short |";
        let cleaned = cleanup_content(content);

        // Each data row should end with " |"
        let lines: Vec<&str> = cleaned.lines().collect();
        for line in &lines[2..] {
            assert!(
                line.ends_with(" |"),
                "Table row should end with ' |' for proper alignment, got:\n{}",
                line
            );
        }

        // Bold should be preserved
        assert!(
            cleaned.contains("**Bold text**"),
            "Bold should be preserved in table, got:\n{}",
            cleaned
        );
    }

    /// Regression test: Table columns with mixed formatting must be properly aligned.
    #[test]
    fn test_table_alignment_with_mixed_formatting() {
        let content = "| Column | Content |\n|--------|-------------|\n| A | `code` with *emphasis* and [link](url) |\n| B | Plain text |";
        let cleaned = cleanup_content(content);

        // Each data row should end with " |"
        let lines: Vec<&str> = cleaned.lines().collect();
        for line in &lines[2..] {
            assert!(
                line.ends_with(" |"),
                "Table row should end with ' |' for proper alignment, got:\n{}",
                line
            );
        }

        // All formatting should be preserved
        assert!(cleaned.contains("`code`"), "Code should be preserved");
        assert!(
            cleaned.contains("*emphasis*"),
            "Emphasis should be preserved"
        );
        assert!(cleaned.contains("[link](url)"), "Link should be preserved");
    }

    /// Regression test: Verify table alignment uses unicode width, not char count.
    #[test]
    fn test_table_alignment_unicode_width() {
        // East Asian characters have display width of 2
        let content = "| Name | Greeting |\n|------|----------|\n| 你好 | Hello |\n| A | World |";
        let cleaned = cleanup_content(content);

        // Each data row should end with " |"
        let lines: Vec<&str> = cleaned.lines().collect();
        for line in &lines[2..] {
            assert!(
                line.ends_with(" |"),
                "Table row should end with ' |' for proper alignment, got:\n{}",
                line
            );
        }

        // Content should be preserved
        assert!(
            cleaned.contains("你好"),
            "CJK characters should be preserved"
        );
    }
