use super::*;
use biscuit_terminal::utils::UnicodeWidthStr;

    // ==================== Incidental Newline Tests ====================

    #[test]
    fn strip_incidental_newlines_drops_after_trailing_whitespace() {
        let content = "Wrapped line \ncontinues here";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "Wrapped line continues here");
    }

    #[test]
    fn strip_incidental_newlines_replaces_with_space_without_trailing_whitespace() {
        let content = "Wrapped line\ncontinues here";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "Wrapped line continues here");
    }

    #[test]
    fn strip_incidental_newlines_preserves_blank_lines() {
        let content = "First paragraph\n\nSecond paragraph\n\n\nThird paragraph";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_fenced_code_verbatim() {
        let content = "Before\n```rust\nlet value = 1;\nlet other = 2;\n```\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(
            stripped,
            "Before\n```rust\nlet value = 1;\nlet other = 2;\n```\nAfter"
        );
    }

    #[test]
    fn strip_incidental_newlines_preserves_indented_code_verbatim() {
        let content = "Before\n\n    let value = 1;\n    let other = 2;\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_newline_inside_inline_code_span() {
        let content = "Before `code\nspan` after";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_html_block_verbatim() {
        let content = "<div>\n<span>One</span>\n<span>Two</span>\n</div>\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_blockquote_prefix() {
        let content = "> Wrapped quote\n> continues here\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "> Wrapped quote continues here\n\nAfter");
    }

    #[test]
    fn strip_incidental_newlines_preserves_table_rows() {
        let content = "| A | B |\n|---|---|\n| 1 | 2 |\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_list_marker() {
        let content = "- Wrapped item\ncontinues here\n- Second item";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "- Wrapped item continues here\n- Second item");
    }

    #[test]
    fn strip_incidental_newlines_preserves_transclusion_directives() {
        let content = "::file ./README.md\n::code ./src/main.rs\n::shell echo hi\n::disclosure Details\n::details\nWrapped\ntext\n::end-disclosure\n";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(
            stripped,
            "::file ./README.md\n::code ./src/main.rs\n::shell echo hi\n::disclosure Details\n::details\nWrapped text\n::end-disclosure\n"
        );
    }

    #[test]
    fn strip_incidental_newlines_preserves_shell_block_bodies() {
        let content = "::shell-block\necho a\necho b\n::end-block\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_blockquoted_shell_block_bodies() {
        let content = "> ::shell-block\n> echo a\n> echo b\n> ::end-block\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_treats_crlf_and_lf_as_newlines() {
        let content = "One\r\ntwo\nthree\r\n\nFour";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "One two three\n\nFour");
    }

    // ---- Structural safety: hard line breaks and setext headings ----

    #[test]
    fn strip_incidental_newlines_preserves_two_space_hard_break() {
        let content = "Roses are red  \nviolets are blue";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_trailing_backslash_hard_break() {
        let content = "Roses are red\\\nviolets are blue";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_collapses_single_trailing_space_not_hard_break() {
        // A single trailing space is incidental, not a hard break: the newline drops.
        let content = "Roses are red \nviolets are blue";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "Roses are red violets are blue");
    }

    #[test]
    fn strip_incidental_newlines_preserves_setext_h1_underline() {
        let content = "Heading\n===\nbody continues";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_setext_h2_underline() {
        let content = "Heading\n---\nbody continues";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    // ---- Join separator by Unicode Script ----

    #[test]
    fn strip_incidental_newlines_joins_han_without_separator() {
        let content = "\u{6F22}\n\u{5B57}";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "\u{6F22}\u{5B57}");
    }

    #[test]
    fn strip_incidental_newlines_joins_thai_without_separator() {
        let content = "\u{0E20}\u{0E32}\u{0E29}\u{0E32}\n\u{0E44}\u{0E17}\u{0E22}";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "\u{0E20}\u{0E32}\u{0E29}\u{0E32}\u{0E44}\u{0E17}\u{0E22}");
    }

    #[test]
    fn strip_incidental_newlines_keeps_space_between_hangul() {
        // Hangul is space-delimited (excluded from the spaceless set).
        let content = "\u{D55C}\n\u{AE00}";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "\u{D55C} \u{AE00}");
    }

    #[test]
    fn strip_incidental_newlines_joins_han_to_latin_without_separator() {
        // Script transition: neutral reconstruction, never a pangu space.
        let content = "\u{6F22}\ntext";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "\u{6F22}text");
    }

    #[test]
    fn strip_incidental_newlines_keeps_space_between_emoji() {
        // Emoji are not letters, so they are not mis-joined as CJK.
        let content = "\u{1F600}\n\u{1F601}";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "\u{1F600} \u{1F601}");
    }

    #[test]
    fn strip_incidental_newlines_joins_after_cjk_punctuation() {
        // A line ending in `。` joined with a following ideograph: no separator.
        let content = "\u{6F22}\u{3002}\n\u{5B57}";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "\u{6F22}\u{3002}\u{5B57}");
    }

    #[test]
    fn strip_incidental_newlines_no_separator_around_zwsp() {
        let trailing = "foo\u{200B}\nbar";
        assert_eq!(strip_incidental_newlines(trailing), "foo\u{200B}bar");

        let leading = "foo\n\u{200B}bar";
        assert_eq!(strip_incidental_newlines(leading), "foo\u{200B}bar");
    }

    // ==================== Fixed-Width Reflow Tests ====================

    #[test]
    fn reflow_to_width_wraps_ascii_paragraphs_at_common_widths() {
        let content = "This paragraph has enough words to wrap cleanly at several common editor widths without needing to split any individual word.";

        for width in [20, 40, 80, 100] {
            let reflowed = cleanup_to_fixed_width(content, width);
            assert!(
                reflowed
                    .lines()
                    .all(|line| UnicodeWidthStr::width(line) <= width),
                "line exceeded width {width}:\n{reflowed}"
            );
        }
    }

    #[test]
    fn reflow_to_width_uses_unicode_display_columns() {
        let content = "café naïve résumé words";
        let reflowed = cleanup_to_fixed_width(content, 12);

        assert_eq!(reflowed, "café naïve\nrésumé words");
        assert!(reflowed.lines().all(|line| UnicodeWidthStr::width(line) <= 12));
    }

    #[test]
    fn reflow_to_width_keeps_long_words_on_one_line() {
        let content = "supercalifragilisticexpialidocious";
        let reflowed = cleanup_to_fixed_width(content, 10);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_keeps_han_run_atomic_and_overflows() {
        // A contiguous Han run is a single whitespace-free token. v1 treats it
        // like a long URL: never split between ideographs, allowed to overflow.
        let content = "\u{6F22}".repeat(10);
        let width = 8;
        assert!(UnicodeWidthStr::width(content.as_str()) > width);

        let reflowed = cleanup_to_fixed_width(&content, width);

        assert_eq!(reflowed, content);
        assert!(!reflowed.contains('\n'));
    }

    #[test]
    fn reflow_to_width_keeps_thai_run_atomic_and_overflows() {
        // Thai is spaceless but non-Han; the atomic-overflow contract holds for
        // the whole curated spaceless set, not just CJK ideographs.
        let content = "\u{0E2A}\u{0E27}\u{0E31}\u{0E2A}\u{0E14}\u{0E35}\u{0E0A}\u{0E32}\u{0E27}\u{0E42}\u{0E25}\u{0E01}";
        let width = 6;
        assert!(UnicodeWidthStr::width(content) > width);

        let reflowed = cleanup_to_fixed_width(content, width);

        assert_eq!(reflowed, content);
        assert!(!reflowed.contains('\n'));
    }

    #[test]
    fn reflow_to_width_handles_empty_document() {
        assert_eq!(cleanup_to_fixed_width("", 80), "");
    }

    #[test]
    fn reflow_to_width_preserves_code_only_document() {
        let content = "```rust\nlet value = \"a very long line that stays untouched\";\n```\n";
        let reflowed = cleanup_to_fixed_width(content, 20);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_preserves_mixed_list_marker_alignment() {
        let content = "- short marker has enough text to wrap around the target width\n1. ordered marker has enough text to wrap around the target width\n- [ ] task marker has enough text to wrap around the target width";
        let reflowed = cleanup_to_fixed_width(content, 28);

        assert_eq!(
            reflowed,
            "- short marker has enough\n  text to wrap around the\n  target width\n1. ordered marker has enough\n   text to wrap around the\n   target width\n- [ ] task marker has enough\n      text to wrap around\n      the target width"
        );
    }

    #[test]
    fn reflow_to_width_preserves_table_rows() {
        let content = "| A | B |\n|---|---|\n| a very long cell | another long cell |\n";
        let reflowed = cleanup_to_fixed_width(content, 12);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_keeps_blockquote_prefixes() {
        let content = "> This quote has enough text to wrap while preserving the quote prefix";
        let reflowed = cleanup_to_fixed_width(content, 24);

        assert_eq!(
            reflowed,
            "> This quote has enough\n> text to wrap while\n> preserving the quote\n> prefix"
        );
    }

    #[test]
    fn reflow_to_width_preserves_transclusion_directives() {
        let content = "::file ./a/path/that/should/not/be/wrapped.md\n::code ./src/main.rs\n";
        let reflowed = cleanup_to_fixed_width(content, 12);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_preserves_shell_block_bodies() {
        let content = "::shell-block\necho a\necho b\n::end-block\n";
        let reflowed = cleanup_to_fixed_width(content, 8);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_preserves_html_blocks() {
        let content = "<div class=\"options\">\n<span>Very long HTML content stays untouched.</span>\n</div>\n";
        let reflowed = cleanup_to_fixed_width(content, 12);

        assert_eq!(reflowed, content);
    }

    #[test]
    #[should_panic(expected = "fixed-width cleanup requires a width greater than 0")]
    fn cleanup_to_fixed_width_rejects_zero_width() {
        let _ = cleanup_to_fixed_width("content", 0);
    }
