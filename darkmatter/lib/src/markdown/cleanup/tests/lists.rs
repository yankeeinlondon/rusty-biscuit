use super::*;
    // ==================== List Marker Preservation Tests ====================

    #[test]
    fn test_list_marker_dash_preserved() {
        let content = "- Item 1\n- Item 2\n- Item 3";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- Item 1"),
            "Dash list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("- Item 2"),
            "Dash list marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_plus_preserved() {
        let content = "+ Alpha\n+ Beta\n+ Gamma";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("+ Alpha"),
            "Plus list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ Beta"),
            "Plus list marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_asterisk_preserved() {
        let content = "* One\n* Two\n* Three";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("* One"),
            "Asterisk list marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_multiple_lists_different_markers() {
        let content = "- Dash item\n\n+ Plus item\n\n* Asterisk item";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- Dash"),
            "First list should use dash, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ Plus"),
            "Second list should use plus, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("* Asterisk"),
            "Third list should use asterisk, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_with_header_and_text() {
        // Regression test: List markers preserved in real document context
        let content = "# Title\n\nSome text.\n\n- First\n- Second\n\nMore text.\n\n+ Alpha\n+ Beta";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- First"),
            "Dash markers should be preserved in document, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ Alpha"),
            "Plus markers should be preserved in document, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn top_level_blocks_do_not_consume_later_list_paragraph_context() {
        let content = concat!(
            "Top prose.\n",
            "\n",
            "## H2\n",
            "\n",
            "1. one\n",
            "\n",
            "    second paragraph\n",
        );
        let expected = concat!(
            "Top prose.\n",
            "\n",
            "## H2\n",
            "\n",
            "1. one\n",
            "    \n",
            "    second paragraph\n",
        );

        assert_eq!(cleanup_content(content), expected);
    }

    #[test]
    fn test_list_inside_blockquote() {
        // List markers inside blockquotes should be preserved
        let content = "> - Quoted item 1\n> - Quoted item 2";
        let cleaned = cleanup_content(content);

        // The structure should be preserved
        assert!(
            cleaned.contains("Quoted item 1"),
            "Blockquote list content should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_not_changed_in_code_block() {
        // List markers inside code blocks should not be modified
        let content = "```\n* This is code\n- Also code\n+ More code\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("* This is code"),
            "Asterisk in code should not change, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("- Also code"),
            "Dash in code should not change, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ More code"),
            "Plus in code should not change, got:\n{}",
            cleaned
        );
    }

    // ==================== Blockquote List Marker Preservation Tests ====================
    //
    // These tests lock down the fix for the bug where `pulldown-cmark-to-cmark`
    // normalizes every unordered bullet to `*` and `restore_list_markers` only
    // matched bare `* ` lines, missing items inside a `> ` blockquote prefix.
    // See `fixes/2026-06-20-unordered-and-quoted/` for the spec.

    #[test]
    fn cleanup_preserves_dash_marker_for_blockquoted_list_item() {
        // `> - item` MUST round-trip with the authored `-` marker.
        let content = "> - dash item";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> - dash item"),
            "Blockquoted `-` marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("> * dash item"),
            "Blockquoted `-` marker should not be normalized to `*`, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn cleanup_preserves_star_marker_for_blockquoted_list_item() {
        // `> * item` keeps its authored `*` marker (no regression for the one
        // shape that already matched by accident).
        let content = "> * star item";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> * star item"),
            "Blockquoted `*` marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn cleanup_preserves_plus_marker_for_blockquoted_list_item() {
        // `> + item` MUST round-trip with the authored `+` marker.
        let content = "> + plus item";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> + plus item"),
            "Blockquoted `+` marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("> * plus item"),
            "Blockquoted `+` marker should not be normalized to `*`, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn cleanup_preserves_authored_markers_for_mixed_blockquote_and_top_level_lists() {
        // Locks down the latent alignment defect: `extract_list_markers` already
        // records blockquote items, but `restore_list_markers` skipped those
        // lines and let top-level items consume the wrong marker slots. Using
        // different markers on each list makes the mis-alignment observable.
        //
        // The blank `>` line inside the blockquote keeps the items as separate
        // list entries (otherwise `strip_incidental_newlines` would join them).
        let content = "> - bq one\n>\n> - bq two\n\n+ top one\n+ top two";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> - bq one"),
            "Blockquote list should keep its authored `-` marker, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("> - bq two"),
            "Blockquote list should keep its authored `-` marker for every item, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ top one"),
            "Top-level list should keep its authored `+` marker, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ top two"),
            "Top-level list should keep its authored `+` marker for every item, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("> * bq"),
            "Blockquote items must not leak the cmark-normalized `*` marker, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n- top"),
            "Top-level items must not inherit the blockquote's `-` marker, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn cleanup_preserves_marker_for_nested_blockquoted_list_item() {
        // `> > - item` keeps `-` after cleanup's existing `> > ` normalization.
        let content = "> > - nested";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> > - nested"),
            "Nested blockquote should keep its authored `-` marker, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("> > * nested"),
            "Nested blockquote should not be normalized to `*`, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn cleanup_preserves_marker_for_compact_blockquoted_list_item() {
        // Compact input `>> - item` is normalized to `> > - item` by
        // `fix_blockquote_formatting`; restoration MUST still preserve `-`.
        let content = ">> - compact";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> > - compact"),
            "Compact blockquote prefix should normalize and keep `-`, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("> > * compact"),
            "Compact blockquote should not be normalized to `*`, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn cleanup_preserves_marker_for_indented_blockquoted_list_item() {
        // Up to three leading spaces are valid CommonMark blockquote indentation;
        // cleanup strips that indent while preserving the authored `-`.
        let content = "   > - indented";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> - indented"),
            "Indented blockquote should keep its authored `-` marker, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("> * indented"),
            "Indented blockquote should not be normalized to `*`, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn restore_list_markers_protects_blockquoted_backtick_fence_content() {
        // `restore_list_markers` MUST NOT rewrite apparent bullets inside a
        // fenced code block nested inside a blockquote. We exercise the helper
        // directly with synthetic post-cmark output because `strip_incidental_newlines`
        // collapses single-line blockquoted fences before they reach cmark, so
        // the protection is not observable through `cleanup_content`.
        let input = "> ```\n> * this is code\n> ```\n";
        let mut buffer = input.to_string();
        restore_list_markers(&mut buffer, &['-'], &[]);

        assert_eq!(
            buffer, input,
            "Bullet inside a blockquoted backtick fence must not be restored, got:\n{}",
            buffer
        );
    }

    #[test]
    fn restore_list_markers_protects_blockquoted_tilde_fence_content() {
        // Same protection as the backtick case but for tilde fences, which
        // CommonMark accepts and which the cmark pipeline can emit.
        let input = "> ~~~\n> * this is code\n> ~~~\n";
        let mut buffer = input.to_string();
        restore_list_markers(&mut buffer, &['-'], &[]);

        assert_eq!(
            buffer, input,
            "Bullet inside a blockquoted tilde fence must not be restored, got:\n{}",
            buffer
        );
    }

    #[test]
    fn test_mixed_list_types_separated_by_text() {
        // Multiple lists with different markers separated by paragraph text
        let content = "- Dash list\n\nParagraph text\n\n+ Plus list\n\nMore text\n\n* Star list";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- Dash"),
            "Dash list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ Plus"),
            "Plus list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("* Star"),
            "Star list marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_consistency_in_same_list() {
        // All items in the same list should use the same marker
        let content = "- Item 1\n- Item 2\n- Item 3\n- Item 4\n- Item 5";
        let cleaned = cleanup_content(content);

        // All items should have dash markers
        let dash_count =
            cleaned.matches("\n- ").count() + if cleaned.starts_with("- ") { 1 } else { 0 };
        assert_eq!(
            dash_count, 5,
            "All 5 items should use dash marker, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_ordered_list_numbers_are_incremented() {
        let content = "1. First\n2. Second\n3. Third";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("1. First"),
            "First item should be numbered 1, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("2. Second"),
            "Second item should be numbered 2, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("3. Third"),
            "Third item should be numbered 3, got:\n{}",
            cleaned
        );
    }

    // ==================== List Indentation Tests ====================

    #[test]
    fn test_nested_list_preserves_4_space_indentation() {
        // Regression test: 4-space indentation should be preserved
        let content = "- Level 1\n    - Level 2\n        - Level 3";
        let cleaned = cleanup_content(content);

        // Should have 4-space indentation for nested items
        assert!(
            cleaned.contains("\n    - Level 2"),
            "4-space indentation should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n        - Level 3"),
            "8-space indentation should be preserved, got:\n{}",
            cleaned
        );
        // Tight nested lists must not gain a spurious blank line before children.
        assert!(
            !cleaned.contains("\n\n    - Level 2"),
            "no blank line before a tight child, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n        - Level 3"),
            "no blank line before a tight grandchild, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_nested_list_normalizes_to_4_space_indentation() {
        // Default cleanup normalizes to 4-space indentation
        let content = "- Level 1\n  - Level 2\n    - Level 3";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("\n    - Level 2"),
            "Default indentation should be 4 spaces, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n    - Level 2"),
            "no blank line before a tight child, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_nested_list_preserves_2_space_with_explicit_indent() {
        // Explicit 2-space indent via cleanup_content_with_indent
        let content = "- Level 1\n  - Level 2\n    - Level 3";
        let cleaned = cleanup_content_with_indent(content, 2);

        assert!(
            cleaned.contains("\n  - Level 2"),
            "2-space indentation should be preserved when explicitly requested, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n    - Level 3"),
            "4-space indentation should be preserved at level 2, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n  - Level 2"),
            "no blank line before a tight child, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n    - Level 3"),
            "no blank line before a tight grandchild, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_nested_list_in_todo_list() {
        // Regression test for nested TODO list items
        let content = "- [x] Task 1\n- Progress\n    - [ ] Sub-task 1\n    - [ ] Sub-task 2";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("\n    - [ ] Sub-task 1"),
            "Nested TODO items should preserve 4-space indentation, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_with_indent_forces_4_space_indentation() {
        let content = "- Level 1\n  - Level 2\n    - Level 3";
        let cleaned = cleanup_content_with_indent(content, 4);

        assert!(
            cleaned.contains("\n    - Level 2"),
            "Nested list should use 4-space indentation, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n        - Level 3"),
            "Nested list should use 8-space indentation at level 2, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n    - Level 2"),
            "no blank line before a tight child, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n        - Level 3"),
            "no blank line before a tight grandchild, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_with_indent_forces_2_space_indentation() {
        let content = "- Level 1\n    - Level 2\n        - Level 3";
        let cleaned = cleanup_content_with_indent(content, 2);

        assert!(
            cleaned.contains("\n  - Level 2"),
            "Nested list should use 2-space indentation, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n    - Level 3"),
            "Nested list should use 4-space indentation at level 2, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n  - Level 2"),
            "no blank line before a tight child, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n    - Level 3"),
            "no blank line before a tight grandchild, got:\n{}",
            cleaned
        );
    }

    /// Regression for the wide-marker defect reviewed in H1 of
    /// `fixes/2026-07-13-fixed-width-lists/review-2.md`. The pre-stack
    /// `fix_list_indentation` derived nesting depth as `current_indent / 2`
    /// from the absolute column, which silently invented depth whenever the
    /// parent marker was wider than one character. Under a `1234. ` parent
    /// (cmark content col 6), a depth-1 child was miscounted as depth 3 and
    /// pushed to col 12 with `--indent 4`, then absorbed into the parent's
    /// prose on the next cleanup pass.
    ///
    /// The stack-based replacement tracks each open level's actual item
    /// column, so depth is derived from real nesting rather than from the
    /// absolute column. The child stays at cmark's canonical position (the
    /// parent's content column) and the cleanup is idempotent.
    #[test]
    fn fix_list_indentation_handles_wide_markers_without_inventing_depth() {
        // `1234. ` parent (marker width 6) with a `- ` child. cmark serializes
        // the child at column 6 — the parent's content column. The rescaled
        // child must stay at column 6 (the parent's content column), not be
        // invented as depth 3 and pushed to column 12.
        let content = "1234. Parent\n      - Child alpha beta gamma delta epsilon.";
        let first = cleanup_content_with_indent(content, 4);

        assert!(
            first.contains("\n      - Child"),
            "depth-1 child under a 6-wide marker must stay at column 6, got:\n{first}"
        );
        assert!(
            !first.contains("\n            - Child"),
            "depth-1 child must not be invented as depth 3 (column 12), got:\n{first}"
        );

        // `10. ` parent (marker width 4) with a `- ` child at column 4.
        // The rescaled child stays at column 4 — `max(target*1, 4) = 4`.
        let content_wide_4 = "10. Parent\n    - Child alpha beta gamma delta epsilon.";
        let first_wide_4 = cleanup_content_with_indent(content_wide_4, 4);
        assert!(
            first_wide_4.contains("\n    - Child"),
            "depth-1 child under a 4-wide marker must stay at column 4, got:\n{first_wide_4}"
        );
        assert!(
            !first_wide_4.contains("\n        - Child"),
            "depth-1 child must not be invented as depth 2 (column 8), got:\n{first_wide_4}"
        );

        // Idempotence: running cleanup a second time on the first-pass output
        // is byte-identical. The pre-stack implementation flattened the
        // structure entirely on the second pass.
        let second = cleanup_content_with_indent(&first, 4);
        assert_eq!(
            first, second,
            "cleanup_content_with_indent(_, 4) must be idempotent on wide markers\n\
             first:\n{first}\nsecond:\n{second}"
        );

        let second_wide_4 = cleanup_content_with_indent(&first_wide_4, 4);
        assert_eq!(
            first_wide_4, second_wide_4,
            "cleanup_content_with_indent(_, 4) must be idempotent on `10. ` markers\n\
             first:\n{first_wide_4}\nsecond:\n{second_wide_4}"
        );
    }

    #[test]
    fn test_loose_list_markers_preserved() {
        // Regression test: loose lists (items separated by blank lines with paragraph
        // content) should preserve all markers. Previously, only the first item kept
        // its original marker; subsequent items reverted to '*'.
        let content = "\
- **First**

    Paragraph under first item.

- **Second**

    Paragraph under second item.

- **Third**

    Paragraph under third item.";
        let cleaned = cleanup_content(content);

        let dash_count =
            cleaned.matches("\n- ").count() + if cleaned.starts_with("- ") { 1 } else { 0 };
        assert_eq!(
            dash_count, 3,
            "All 3 loose list items should use dash marker, got:\n{}",
            cleaned
        );
    }

    // ==================== List Spacing Mode Tests ====================

    #[test]
    fn normal_no_blank_lines_between_same_level_items() {
        let input = "1. First\n\n2. Second\n\n3. Third\n";
        let cleaned = cleanup_content(input);
        assert!(
            cleaned.contains("1. First\n2. Second\n3. Third"),
            "Normal mode: no blank lines between same-level items, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn normal_descent_into_sublist_is_tight() {
        let input = "\
1. read the lessons:
   - @docs/knowledge/commits.md
2. evaluate all the _staged_ files
";
        let cleaned = cleanup_content(input);
        // Descending into a tight sub-list must not insert a blank line.
        assert!(
            cleaned.contains("lessons:\n    - @docs"),
            "Normal: tight descent into sub-list stays tight, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn normal_return_from_sublist_inserts_blank() {
        let input = "\
1. read the lessons:
   - @docs/knowledge/commits.md
2. evaluate all the _staged_ files
";
        let cleaned = cleanup_content(input);
        // Returning to the shallower parent level still separates with a blank line.
        assert!(
            cleaned.contains("commits.md\n\n2."),
            "Normal: blank line when returning from sub-list, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn normal_no_blank_between_same_level_without_transition() {
        let input = "\
0. If no files are staged then exit.
1. read the lessons
2. evaluate the files
3. organize the work
";
        let cleaned = cleanup_content(input);
        assert!(
            cleaned.contains("exit.\n1.") || cleaned.contains("exit.\n0."),
            "Normal: no blank lines between same-level items, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn normal_blank_line_after_list_before_prose() {
        let input = "\
1. First
   - sub item
2. Second

Some prose after the list.
";
        let cleaned = cleanup_content(input);
        assert!(
            cleaned.contains("2. Second\n\nSome prose"),
            "Normal: blank line between list end and prose, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn compact_removes_all_blank_lines_between_items() {
        let input = "1. First\n\n2. Second\n\n3. Third\n";
        let cleaned = cleanup_content_compact(input);
        assert!(
            cleaned.contains("1. First\n2. Second\n3. Third"),
            "Compact: no blank lines between items, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn compact_with_nested_list() {
        let input = "\
1. read the lessons:

   - @docs/knowledge/commits.md

2. evaluate all the _staged_ files
";
        let cleaned = cleanup_content_compact(input);
        assert!(
            !cleaned.contains("\n\n2."),
            "Compact: no blank line before item 2, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn compact_preserves_blank_before_non_list() {
        let input = "Some paragraph.\n\n1. First\n\n2. Second\n";
        let cleaned = cleanup_content_compact(input);
        assert!(
            cleaned.contains("paragraph.\n\n1."),
            "Compact: preserve blank between paragraph and list, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn loose_adds_blank_lines_between_all_items() {
        let input = "1. First\n2. Second\n3. Third\n";
        let cleaned = cleanup_content_loose(input);
        assert!(
            cleaned.contains("1. First\n\n2. Second\n\n3. Third"),
            "Loose: blank lines between all items, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn loose_with_nested_list() {
        let input = "\
1. read the lessons:
   - @docs/knowledge/commits.md
2. evaluate
3. organize
";
        let cleaned = cleanup_content_loose(input);
        assert!(
            cleaned.contains("\n\n2."),
            "Loose: blank line before item 2, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n\n3."),
            "Loose: blank line before item 3, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn normal_loose_list_preserves_blank_lines_between_items() {
        // Loose lists (items with continuation paragraphs) should have blank
        // lines between items in Normal mode, not just in Loose mode.
        let input = "\
- **First**

    Paragraph under first item.

- **Second**

    Paragraph under second item.

- **Third**

    Paragraph under third item.
";
        let cleaned = cleanup_content(input);

        assert!(
            cleaned.contains("first item.\n\n- **Second**"),
            "Normal: blank line before second item in loose list, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("second item.\n\n- **Third**"),
            "Normal: blank line before third item in loose list, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn tight_list_stays_tight_in_normal_mode() {
        let input = "1. First\n2. Second\n3. Third\n";
        let cleaned = cleanup_content(input);
        assert!(
            cleaned.contains("1. First\n2. Second\n3. Third"),
            "Normal: tight list stays tight, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn normal_blank_line_between_sublist_and_prose() {
        let input = "\
6. review lessons:
   1. important
   2. not represented

   If both criteria are met then save.
";
        let cleaned = cleanup_content(input);
        assert!(
            cleaned.contains("represented\n\n   If both"),
            "Normal: blank line between sub-list and prose continuation, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn tight_nested_list_stays_tight_after_cleanup() {
        // Regression test modeled on the incident's `## Closure` section.
        let input = "\
## Closure

- Save your review suggestions to \"path.md\"
- Save the following frontmatter properties on \"path.md\":
    - based on your review suggestions indicate whether you think this feature is **ready for production**
    - set the `agent` frontmatter property to \"codex\"
    - set the `model` frontmatter property as \"gpt-4o\"
    - set the `created` frontmatter property to \"2026-06-18\"

**Next steps:**

- verify the file
";
        let cleaned = cleanup_content(input);

        assert!(
            cleaned.contains("properties on \"path.md\":\n    - based on"),
            "tight nested list: parent must be directly followed by child, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("properties on \"path.md\":\n\n    - based on"),
            "tight nested list: no blank line before the first child, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n    - set the `agent` frontmatter property"),
            "nested children must keep the configured 4-space indent, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn tight_siblings_stay_tight() {
        let input = "\
- alpha
- beta
    - gamma
    - delta
";
        let cleaned = cleanup_content(input);

        assert!(
            cleaned.contains("- alpha\n- beta\n    - gamma\n    - delta"),
            "Normal: same-level siblings and descents stay tight, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n- beta"),
            "Normal: no blank line between same-level siblings, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n    - gamma"),
            "Normal: no blank line before a tight child, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn closing_a_sublist_inserts_blank() {
        let input = "\
- parent:
    - child

- sibling
";
        let cleaned = cleanup_content(input);

        assert!(
            cleaned.contains("parent:\n    - child"),
            "Normal: descent into sub-list stays tight, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("parent:\n\n    - child"),
            "Normal: no blank line between parent and child, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("child\n\n- sibling"),
            "Normal: returning to a shallower level inserts a blank line, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn loose_list_keeps_blank_lines_in_normal_mode() {
        // Focused guard: loose (continuation) items must still separate with blanks.
        let input = "\
- **First**

    Paragraph under first item.

- **Second**
";
        let cleaned = cleanup_content(input);

        assert!(
            cleaned.contains("Paragraph under first item.\n\n- **Second**"),
            "Normal: loose list items keep their blank-line separator, got:\n{}",
            cleaned
        );
    }
