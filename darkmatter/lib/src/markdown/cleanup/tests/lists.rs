use super::*;

// ==================== List Marker Preservation ====================
//
// Cleanup edits authored bytes in place, so list markers are never respelled.

#[test]
fn dash_marker_is_unchanged() {
    let content = "- Item 1\n- Item 2\n- Item 3";
    assert_eq!(clean(content), content);
}

#[test]
fn plus_marker_is_unchanged() {
    let content = "+ Alpha\n+ Beta\n+ Gamma";
    assert_eq!(clean(content), content);
}

#[test]
fn asterisk_marker_is_unchanged() {
    let content = "* One\n* Two\n* Three";
    assert_eq!(clean(content), content);
}

#[test]
fn lists_with_different_markers_are_unchanged() {
    let content = "- Dash item\n\n+ Plus item\n\n* Asterisk item";
    assert_eq!(clean(content), content);
}

#[test]
fn list_markers_in_a_document_are_unchanged() {
    let content = "# Title\n\nSome text.\n\n- First\n- Second\n\nMore text.\n\n+ Alpha\n+ Beta";
    assert_eq!(clean(content), content);
}

#[test]
fn second_item_paragraph_after_heading_is_unchanged() {
    let content = concat!(
        "Top prose.\n",
        "\n",
        "## H2\n",
        "\n",
        "1. one\n",
        "\n",
        "    second paragraph\n",
    );
    assert_eq!(clean(content), content);
}

#[test]
fn list_inside_blockquote_is_unchanged() {
    let content = "> - Quoted item 1\n> - Quoted item 2";
    assert_eq!(clean(content), content);
}

#[test]
fn list_markers_inside_code_block_are_unchanged() {
    assert_eq!(
        clean("```\n* This is code\n- Also code\n+ More code\n```"),
        "```text\n* This is code\n- Also code\n+ More code\n```"
    );
}

// ==================== Blockquote List Markers ====================

#[test]
fn blockquoted_dash_marker_is_unchanged() {
    assert_eq!(clean("> - dash item"), "> - dash item");
}

#[test]
fn blockquoted_star_marker_is_unchanged() {
    assert_eq!(clean("> * star item"), "> * star item");
}

#[test]
fn blockquoted_plus_marker_is_unchanged() {
    assert_eq!(clean("> + plus item"), "> + plus item");
}

#[test]
fn mixed_blockquoted_and_top_level_markers_are_unchanged() {
    let content = "> - bq one\n>\n> - bq two\n\n+ top one\n+ top two";
    assert_eq!(clean(content), content);
}

#[test]
fn nested_blockquoted_marker_is_unchanged() {
    assert_eq!(clean("> > - nested"), "> > - nested");
}

#[test]
fn compact_blockquote_prefix_is_not_respaced() {
    assert_eq!(clean(">> - compact"), ">> - compact");
}

#[test]
fn indented_blockquote_prefix_is_not_stripped() {
    assert_eq!(clean("   > - indented"), "   > - indented");
}

#[test]
fn bullet_inside_blockquoted_backtick_fence_is_unchanged() {
    assert_eq!(clean("> ```\n> * this is code\n> ```\n"), "> ```text\n> * this is code\n> ```\n");
}

#[test]
fn bullet_inside_blockquoted_tilde_fence_is_unchanged() {
    assert_eq!(clean("> ~~~\n> * this is code\n> ~~~\n"), "> ~~~text\n> * this is code\n> ~~~\n");
}

#[test]
fn lists_separated_by_text_are_unchanged() {
    let content = "- Dash list\n\nParagraph text\n\n+ Plus list\n\nMore text\n\n* Star list";
    assert_eq!(clean(content), content);
}

#[test]
fn same_list_markers_are_unchanged() {
    let content = "- Item 1\n- Item 2\n- Item 3\n- Item 4\n- Item 5";
    assert_eq!(clean(content), content);
}

#[test]
fn sequential_ordered_list_is_unchanged() {
    let content = "1. First\n2. Second\n3. Third";
    assert_eq!(clean(content), content);
}

// ==================== List Indentation ====================

#[test]
fn four_space_nesting_is_unchanged_by_default() {
    let content = "- Level 1\n    - Level 2\n        - Level 3";
    assert_eq!(clean(content), content);
}

#[test]
fn two_space_nesting_is_reindented_to_four_by_default() {
    assert_eq!(
        clean("- Level 1\n  - Level 2\n    - Level 3"),
        "- Level 1\n    - Level 2\n        - Level 3"
    );
}

#[test]
fn two_space_nesting_is_unchanged_with_indentation_two() {
    let content = "- Level 1\n  - Level 2\n    - Level 3";
    assert_eq!(clean_indent(content, 2), content);
}

#[test]
fn nested_task_list_is_unchanged() {
    let content = "- [x] Task 1\n- Progress\n    - [ ] Sub-task 1\n    - [ ] Sub-task 2";
    assert_eq!(clean(content), content);
}

#[test]
fn indentation_four_reindents_two_space_nesting() {
    assert_eq!(
        clean_indent("- Level 1\n  - Level 2\n    - Level 3", 4),
        "- Level 1\n    - Level 2\n        - Level 3"
    );
}

#[test]
fn indentation_two_reindents_four_space_nesting() {
    assert_eq!(
        clean_indent("- Level 1\n    - Level 2\n        - Level 3", 2),
        "- Level 1\n  - Level 2\n    - Level 3"
    );
}

/// A child of a marker wider than the requested indentation stays at the
/// parent's content column rather than being pushed to a deeper level.
#[test]
fn indentation_four_keeps_child_of_wide_marker_at_content_column() {
    let content = "1234. Parent\n      - Child alpha beta gamma delta epsilon.";
    assert_eq!(clean_indent(content, 4), content);

    let content = "10. Parent\n    - Child alpha beta gamma delta epsilon.";
    assert_eq!(clean_indent(content, 4), content);
}

#[test]
fn loose_list_markers_are_unchanged() {
    let content = "\
- **First**

    Paragraph under first item.

- **Second**

    Paragraph under second item.

- **Third**

    Paragraph under third item.";
    assert_eq!(clean(content), content);
}

// ==================== List Item Padding ====================

#[test]
fn blank_lines_between_siblings_are_kept_by_default() {
    let content = "1. First\n\n2. Second\n\n3. Third\n";
    assert_eq!(clean(content), content);
}

#[test]
fn tight_descent_into_sublist_stays_tight() {
    let content = "\
1. read the lessons:
   - @docs/knowledge/commits.md
2. evaluate all the _staged_ files
";
    let expected = "\
1. read the lessons:
    - @docs/knowledge/commits.md
2. evaluate all the _staged_ files
";
    assert_eq!(clean(content), expected);
}

#[test]
fn sequential_list_starting_at_zero_is_unchanged() {
    let content = "\
0. If no files are staged then exit.
1. read the lessons
2. evaluate the files
3. organize the work
";
    assert_eq!(clean(content), content);
}

#[test]
fn blank_line_between_list_and_prose_is_kept() {
    let content = "\
1. First
   - sub item
2. Second

Some prose after the list.
";
    let expected = "\
1. First
    - sub item
2. Second

Some prose after the list.
";
    assert_eq!(clean(content), expected);
}

#[test]
fn compact_removes_blank_lines_between_items() {
    assert_eq!(clean_with("1. First\n\n2. Second\n\n3. Third\n", compact), "1. First\n2. Second\n3. Third\n");
}

/// `none` owns only sibling boundaries; the blank line before the nested list
/// is interior padding, left as written.
#[test]
fn compact_keeps_interior_blank_before_nested_list() {
    let content = "\
1. read the lessons:

   - @docs/knowledge/commits.md

2. evaluate all the _staged_ files
";
    let expected = "\
1. read the lessons:

    - @docs/knowledge/commits.md
2. evaluate all the _staged_ files
";
    assert_eq!(clean_with(content, compact), expected);
}

#[test]
fn compact_keeps_blank_before_list() {
    assert_eq!(
        clean_with("Some paragraph.\n\n1. First\n\n2. Second\n", compact),
        "Some paragraph.\n\n1. First\n2. Second\n"
    );
}

#[test]
fn loose_adds_blank_lines_between_items() {
    assert_eq!(clean_with("1. First\n2. Second\n3. Third\n", loose), "1. First\n\n2. Second\n\n3. Third\n");
}

#[test]
fn loose_separates_siblings_but_not_an_item_from_its_nested_list() {
    let content = "\
1. read the lessons:
   - @docs/knowledge/commits.md
2. evaluate
3. organize
";
    let expected = "\
1. read the lessons:
    - @docs/knowledge/commits.md

2. evaluate

3. organize
";
    assert_eq!(clean_with(content, loose), expected);
}

#[test]
fn tight_list_is_unchanged() {
    let content = "1. First\n2. Second\n3. Third\n";
    assert_eq!(clean(content), content);
}

#[test]
fn blank_line_between_sublist_and_item_paragraph_is_kept() {
    let content = "\
6. review lessons:
   1. important
   2. not represented

   If both criteria are met then save.
";
    let expected = "\
6. review lessons:
    1. important
    2. not represented

   If both criteria are met then save.
";
    assert_eq!(clean(content), expected);
}

#[test]
fn tight_nested_list_in_document_is_unchanged() {
    let content = "\
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
    assert_eq!(clean(content), content);
}

#[test]
fn tight_siblings_and_descent_are_unchanged() {
    let content = "\
- alpha
- beta
    - gamma
    - delta
";
    assert_eq!(clean(content), content);
}

#[test]
fn blank_line_after_closing_sublist_is_kept() {
    let content = "\
- parent:
    - child

- sibling
";
    assert_eq!(clean(content), content);
}
