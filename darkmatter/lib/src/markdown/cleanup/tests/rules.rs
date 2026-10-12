//! One test per `style.markdown` rule: its default and every non-default
//! value, with the documented examples as exact expectations.

use super::body_support::{clean_accepted, only, rules_with, structure};
use crate::markdown::cleanup::rules::{
    DefaultLanguage, FixedWidth, Indentation, Leaf, ListInteriorPadding, ListItemPadding, ResolvedMarkdownRules,
};

fn defaults() -> ResolvedMarkdownRules {
    ResolvedMarkdownRules::defaults()
}

#[test]
fn frontmatter_rule_does_not_touch_a_body() {
    // `frontmatter` governs repair of the YAML block only.
    let source = "- a\n  - b\n";
    let off = rules_with(|o| o.frontmatter = Leaf::Value(false));
    assert_eq!(clean_accepted(source, &off), clean_accepted(source, &defaults()));
}

#[test]
fn table_layout_pads_cells_and_the_delimiter_row_by_default() {
    let source = "| a | bb |\n|---|:-:|\n| ccc | d |\n";
    let expected = "| a   | bb  |\n| --- | :-: |\n| ccc | d   |\n";
    assert_eq!(clean_accepted(source, &defaults()), expected);
    assert_eq!(structure(expected, false), structure(source, false));

    let off = rules_with(|o| o.table_layout = Leaf::Value(false));
    assert_eq!(clean_accepted(source, &off), source);
}

#[test]
fn table_layout_keeps_alignment_prefixes_and_wide_characters() {
    // A pipe inside a code span must be escaped in a table cell.
    let source = "> | left | right | none |\n> |:--|--:|---|\n> | 日本 | x | `a\\|b` |\n";
    let expected = "> | left | right | none   |\n> | :--- | ----: | ------ |\n> | 日本 | x     | `a\\|b` |\n";
    let cleaned = clean_accepted(source, &only(|o| o.table_layout = Leaf::Value(true)));
    assert_eq!(cleaned, expected);
}

#[test]
fn table_layout_leaves_tables_holding_expressions_or_extra_cells() {
    let rules = only(|o| o.table_layout = Leaf::Value(true));
    let expression = "| a | b |\n|---|---|\n| {{ x }} | y |\n";
    assert_eq!(clean_accepted(expression, &rules), expression);
    let extra = "| a | b |\n|---|---|\n| 1 | 2 | 3 |\n";
    assert_eq!(clean_accepted(extra, &rules), extra);
    let unescaped_code_pipe = "| a | b |\n|---|---|\n| `x|y` | z |\n";
    assert_eq!(clean_accepted(unescaped_code_pipe, &rules), unescaped_code_pipe);
}

#[test]
fn unwrap_is_off_by_default_and_joins_paragraph_lines_when_on() {
    let source = "one\ntwo\n";
    assert_eq!(clean_accepted(source, &defaults()), source);
    let on = rules_with(|o| o.unwrap = Leaf::Value(true));
    assert_eq!(clean_accepted(source, &on), "one two\n");
}

#[test]
fn unwrap_joins_list_and_quote_paragraphs_and_keeps_hard_breaks() {
    let on = only(|o| o.unwrap = Leaf::Value(true));
    assert_eq!(clean_accepted("- one\n  two\n", &on), "- one two\n");
    assert_eq!(clean_accepted("> one\n> two\n", &on), "> one two\n");
    assert_eq!(clean_accepted("one  \ntwo\nthree\n", &on), "one  \ntwo three\n");
    assert_eq!(clean_accepted("one\\\ntwo\nthree\n", &on), "one\\\ntwo three\n");
    // Trailing spaces at a joined break are not left behind.
    assert_eq!(clean_accepted("one \ntwo\n", &on), "one two\n");
    // Spaceless scripts join with no separator.
    assert_eq!(clean_accepted("日本\n語\n", &on), "日本語\n");
}

#[test]
fn unwrap_never_splits_an_expression_or_joins_into_a_directive() {
    let on = only(|o| o.unwrap = Leaf::Value(true));
    let expression = "Hello {{ name\n }} and\nmore\n";
    assert_eq!(clean_accepted(expression, &on), "Hello {{ name\n }} and more\n");
    let directive = "para one\n::file ./x.md\npara two\n";
    assert_eq!(clean_accepted(directive, &on), directive);
}

#[test]
fn fixed_width_is_off_by_default() {
    let source = "a long line of prose that would certainly be wrapped at twenty columns\n";
    assert_eq!(clean_accepted(source, &defaults()), source);
    let off = rules_with(|o| o.fixed_width = Leaf::Value(FixedWidth::Off));
    assert_eq!(clean_accepted(source, &off), source);
}

#[test]
fn fixed_width_unwraps_then_rewraps_counting_container_prefixes() {
    let rules = only(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(20)));
    assert_eq!(
        clean_accepted("one two three\nfour five six seven eight\n", &rules),
        "one two three four\nfive six seven eight\n"
    );
    assert_eq!(
        clean_accepted("> one two three four five six\n", &rules),
        "> one two three four\n> five six\n"
    );
    assert_eq!(
        clean_accepted("- one two three four five six\n", &rules),
        "- one two three four\n  five six\n"
    );
}

#[test]
fn fixed_width_eighty_is_the_documented_example() {
    let rules = only(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(80)));
    let source = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n";
    let cleaned = clean_accepted(source, &rules);
    assert_eq!(
        cleaned,
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor\nincididunt ut labore et dolore magna aliqua.\n"
    );
    assert!(cleaned.lines().all(|line| line.chars().count() <= 80));
}

#[test]
fn fixed_width_true_means_one_hundred_twenty_and_implies_unwrap() {
    let words: Vec<String> = (0..40).map(|index| format!("w{index:02}")).collect();
    let source = format!("{}\n{}\n", words[..20].join(" "), words[20..].join(" "));
    let rules = rules_with(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(120)));
    assert_eq!(rules.wrap_columns(), Some(120));
    assert!(rules.joins_soft_breaks(), "a width implies unwrap");
    let cleaned = clean_accepted(&source, &rules);
    let first = cleaned.lines().next().unwrap();
    // 30 three-character words and their 29 separating spaces fill 119 columns.
    assert_eq!(first.split(' ').count(), 30);
    assert!(first.len() <= 120);
}

#[test]
fn fixed_width_keeps_hard_breaks_code_links_and_expressions_whole() {
    let rules = only(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(12)));
    assert_eq!(clean_accepted("aaa bbb ccc  \nddd eee fff\n", &rules), "aaa bbb ccc  \nddd eee fff\n");
    assert_eq!(clean_accepted("x `a b c d e f` y\n", &rules), "x\n`a b c d e f`\ny\n");
    assert_eq!(
        clean_accepted("see [the link](https://example.com \"a title\") now\n", &rules),
        "see [the\nlink](https://example.com \"a title\")\nnow\n"
    );
    assert_eq!(clean_accepted("hi {{ a + b + c }} there\n", &rules), "hi\n{{ a + b + c }}\nthere\n");
}

#[test]
fn fixed_width_never_starts_a_line_with_a_block_marker() {
    let rules = only(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(10)));
    let source = "aaaaaaa - bbb # ccc > ddd 1. eee ::x\n";
    let cleaned = clean_accepted(source, &rules);
    for line in cleaned.lines().skip(1) {
        assert!(!line.starts_with(['-', '#', '>', ':']) && !line.starts_with("1."), "{cleaned:?}");
    }
}

#[test]
fn indentation_defaults_to_four_spaces_per_level() {
    assert_eq!(clean_accepted("- a\n  - b\n    - c\n", &defaults()), "- a\n    - b\n        - c\n");
}

#[test]
fn indentation_two_eight_and_preserve() {
    let source = "- a\n    - b\n        - c\n";
    let two = rules_with(|o| o.indentation = Leaf::Value(Indentation::Two));
    assert_eq!(clean_accepted(source, &two), "- a\n  - b\n    - c\n");
    let eight = rules_with(|o| o.indentation = Leaf::Value(Indentation::Eight));
    assert_eq!(clean_accepted(source, &eight), "-    a\n        -    b\n                - c\n");
    let preserve = rules_with(|o| o.indentation = Leaf::Value(Indentation::Preserve));
    assert_eq!(clean_accepted("- a\n   - b\n", &preserve), "- a\n   - b\n");
}

#[test]
fn indentation_moves_every_line_of_a_nested_item_together() {
    let source = "- a\n  - b\n\n    > quote\n\n    ```\n    code\n    ```\n";
    let cleaned = clean_accepted(source, &only(|o| o.indentation = Leaf::Value(Indentation::Four)));
    assert_eq!(cleaned, "- a\n    - b\n\n      > quote\n\n      ```\n      code\n      ```\n");
    assert_eq!(structure(&cleaned, false), structure(source, false));
}

#[test]
fn ordered_lists_leave_uniform_and_sequential_numbers() {
    for source in ["1. a\n1. b\n1. c\n", "3. a\n3. b\n3. c\n", "3. a\n4. b\n5. c\n", "1. a\n2. b\n3. c\n"] {
        assert_eq!(clean_accepted(source, &defaults()), source);
    }
}

#[test]
fn ordered_lists_renumber_from_the_first_number() {
    assert_eq!(clean_accepted("1. a\n3. b\n8. c\n", &defaults()), "1. a\n2. b\n3. c\n");
    assert_eq!(clean_accepted("2. a\n2. b\n5. c\n", &defaults()), "2. a\n3. b\n4. c\n");
    let off = rules_with(|o| o.ordered_lists = Leaf::Value(false));
    assert_eq!(clean_accepted("1. a\n3. b\n8. c\n", &off), "1. a\n3. b\n8. c\n");
}

#[test]
fn ordered_lists_keep_content_columns_when_a_marker_changes_width() {
    let source = "9.  nine\n\n    more\n9.  ten\n\n    more\n12. eleven\n";
    let cleaned = clean_accepted(source, &only(|o| o.ordered_lists = Leaf::Value(true)));
    assert_eq!(cleaned, "9.  nine\n\n    more\n10. ten\n\n    more\n11. eleven\n");
}

#[test]
fn list_padding_adds_one_blank_line_around_lists() {
    assert_eq!(clean_accepted("Intro:\n- a\n", &defaults()), "Intro:\n\n- a\n");
    assert_eq!(clean_accepted("- a\n# After\n", &defaults()), "- a\n\n# After\n");
    assert_eq!(clean_accepted("Intro:\n\n\n- a\n\n\n\n# After\n", &defaults()), "Intro:\n\n- a\n\n# After\n");
    assert_eq!(clean_accepted("> Intro:\n> - a\n", &defaults()), "> Intro:\n>\n> - a\n");
    let off = rules_with(|o| o.list_padding = Leaf::Value(false));
    assert_eq!(clean_accepted("Intro:\n- a\n", &off), "Intro:\n- a\n");
}

#[test]
fn list_padding_never_pads_a_directive_neighbor_or_a_container_edge() {
    let source = "::block when=\"a\"\n- a\n::end-block\nText\n";
    assert_eq!(clean_accepted(source, &defaults()), source);
    // The first and last block of a document get no padding.
    assert_eq!(clean_accepted("- a\n", &defaults()), "- a\n");
    // A nested list's boundary belongs to list-interior-padding: padding it
    // would make the parent list loose.
    assert_eq!(clean_accepted("- a\n    - b\n- c\n", &defaults()), "- a\n    - b\n- c\n");
}

const SIBLINGS_NONE: &str = "- apples\n- pears\n    - green\n    - red\n- plums\n";
const SIBLINGS_ONE: &str = "- apples\n\n- pears\n    - green\n\n    - red\n\n- plums\n";

#[test]
fn list_item_padding_preserve_is_the_default() {
    assert_eq!(clean_accepted(SIBLINGS_NONE, &defaults()), SIBLINGS_NONE);
    assert_eq!(clean_accepted(SIBLINGS_ONE, &defaults()), SIBLINGS_ONE);
}

#[test]
fn list_item_padding_none_and_one_match_the_documented_example() {
    let none = rules_with(|o| o.list_item_padding = Leaf::Value(ListItemPadding::None));
    let one = rules_with(|o| o.list_item_padding = Leaf::Value(ListItemPadding::One));
    assert_eq!(clean_accepted(SIBLINGS_NONE, &one), SIBLINGS_ONE);
    assert_eq!(clean_accepted(SIBLINGS_ONE, &none), SIBLINGS_NONE);
    assert_eq!(clean_accepted("- a\n\n\n- b\n", &one), "- a\n\n- b\n");
}

const INTERIOR_NONE: &str = "- Install the tool\n    ```sh\n    brew install tool\n    ```\n    - macOS\n    - Linux\n";
const INTERIOR_EXCEPT: &str =
    "- Install the tool\n\n    ```sh\n    brew install tool\n    ```\n    - macOS\n    - Linux\n";
const INTERIOR_ALL: &str =
    "- Install the tool\n\n    ```sh\n    brew install tool\n    ```\n\n    - macOS\n    - Linux\n";

#[test]
fn list_interior_padding_matches_the_documented_example_from_every_starting_point() {
    let value = |padding| rules_with(move |o| o.list_interior_padding = Leaf::Value(padding));
    let cases = [
        (ListInteriorPadding::None, INTERIOR_NONE),
        (ListInteriorPadding::ExceptNestedLists, INTERIOR_EXCEPT),
        (ListInteriorPadding::All, INTERIOR_ALL),
    ];
    for (padding, expected) in cases {
        for source in [INTERIOR_NONE, INTERIOR_EXCEPT, INTERIOR_ALL] {
            assert_eq!(clean_accepted(source, &value(padding)), expected, "{padding:?} from {source:?}");
        }
    }
    let preserve = value(ListInteriorPadding::Preserve);
    for source in [INTERIOR_NONE, INTERIOR_EXCEPT, INTERIOR_ALL] {
        assert_eq!(clean_accepted(source, &preserve), source);
        assert_eq!(clean_accepted(source, &defaults()), source);
    }
}

#[test]
fn required_separators_survive_none_under_both_padding_rules() {
    let source = "- a\n\n    nested paragraph\n    - x\n\n    after the nested list\n\n- b\n";
    let item_none = rules_with(|o| o.list_item_padding = Leaf::Value(ListItemPadding::None));
    assert_eq!(
        clean_accepted(source, &item_none),
        "- a\n\n    nested paragraph\n    - x\n\n    after the nested list\n- b\n"
    );
    let interior_none = rules_with(|o| o.list_interior_padding = Leaf::Value(ListInteriorPadding::None));
    assert_eq!(clean_accepted(source, &interior_none), source);
}

#[test]
fn default_language_labels_unlabeled_fences_with_text() {
    assert_eq!(clean_accepted("```\nx\n```\n", &defaults()), "```text\nx\n```\n");
    assert_eq!(clean_accepted("~~~\nx\n~~~\n", &defaults()), "~~~text\nx\n~~~\n");
    // Existing info strings stay exact, and indented code gets no fence.
    assert_eq!(clean_accepted("```rust {a}\nx\n```\n", &defaults()), "```rust {a}\nx\n```\n");
    assert_eq!(clean_accepted("    indented\n", &defaults()), "    indented\n");
}

#[test]
fn default_language_custom_and_empty_values() {
    let shell = rules_with(|o| o.default_language = Leaf::Value(DefaultLanguage::new("sh").unwrap()));
    assert_eq!(clean_accepted("```\nx\n```\n", &shell), "```sh\nx\n```\n");
    let none = rules_with(|o| o.default_language = Leaf::Value(DefaultLanguage::new("").unwrap()));
    assert_eq!(clean_accepted("```\nx\n```\n", &none), "```\nx\n```\n");
}

/// Minimal reproductions of rule bugs the repository corpus exposed. Each was
/// refused by the guard before its fix; each must now format.
mod corpus_regressions {
    use super::super::body_support::{clean_accepted, only, structure};
    use crate::markdown::cleanup::rules::{Indentation, Leaf};

    fn four() -> crate::markdown::cleanup::rules::ResolvedMarkdownRules {
        only(|o| o.indentation = Leaf::Value(Indentation::Four))
    }

    #[test]
    fn a_lazy_line_inside_a_code_span_keeps_its_indentation() {
        // The parser strips a lazy line only to the container it satisfies,
        // so moving it would put a space into the code span.
        let source = "1. top\n   1. item `bt\n   prose` end\n";
        let cleaned = clean_accepted(source, &four());
        assert_eq!(cleaned, "1. top\n    1. item `bt\n   prose` end\n");
        assert_eq!(structure(&cleaned, false), structure(source, false));
    }

    #[test]
    fn items_authored_at_different_columns_each_reach_the_target() {
        let source = "1. parent\n\n   text\n\n     - first\n     - second\n\n   - third\n";
        let cleaned = clean_accepted(source, &four());
        assert_eq!(cleaned, "1. parent\n\n   text\n\n    - first\n    - second\n\n    - third\n");
        assert_eq!(structure(&cleaned, false), structure(source, false));
    }

    #[test]
    fn a_quotes_bare_marker_line_moves_with_the_quote() {
        let source = "1. a\n   - b\n\n     > one\n     >\n     > two\n";
        let cleaned = clean_accepted(source, &four());
        assert_eq!(cleaned, "1. a\n    - b\n\n      > one\n      >\n      > two\n");
        assert_eq!(structure(&cleaned, false), structure(source, false));
    }

    #[test]
    fn a_list_crossed_by_a_multi_line_expression_is_left_as_authored() {
        // `{{{` inside inline code opens a literal the finder closes lines
        // later; re-indenting inside it would change its spelling.
        let source = "- a\n  - `rg -F '{{{'`;\n  - x }}} y\n";
        assert_eq!(clean_accepted(source, &four()), source);
    }

    fn two() -> crate::markdown::cleanup::rules::ResolvedMarkdownRules {
        only(|o| o.indentation = Leaf::Value(Indentation::Two))
    }

    #[test]
    fn moving_a_list_left_never_captures_the_parents_next_block() {
        // The parser's list range ends inside the next line's indentation; the
        // paragraph after the list must stay the parent item's.
        let source = "- a\n    - b\n    - c\n\n    para\n- d\n";
        let cleaned = clean_accepted(source, &two());
        assert_eq!(cleaned, "- a\n   - b\n   - c\n\n    para\n- d\n");
        assert_eq!(structure(&cleaned, false), structure(source, false));
    }

    #[test]
    fn whitespace_lines_inside_code_move_with_the_code() {
        let source = "1. a\n\n     ```sh\n     one\n       \n     two\n     ```\n";
        let cleaned = clean_accepted(source, &two());
        assert_eq!(structure(&cleaned, false), structure(source, false));
        let empty_line = "- a\n\n  ```\n  x\n\n  y\n  ```\n";
        assert!(!clean_accepted(empty_line, &four()).contains("\n    \n"), "an empty line stays empty");
    }

    #[test]
    fn a_lazy_line_that_would_join_the_item_blocks_a_left_move() {
        let source = "- a\n    - b `x\n    y` z\n";
        let cleaned = clean_accepted(source, &two());
        assert_eq!(structure(&cleaned, false), structure(source, false));
        assert_eq!(cleaned, source);
    }

    #[test]
    fn a_lazy_line_joined_by_unwrap_does_not_block_the_move() {
        let rules = only(|o| {
            o.indentation = Leaf::Value(Indentation::Four);
            o.unwrap = Leaf::Value(true);
        });
        let source = "1. one\n      - run it\n      Note: then check\n";
        // `clean_accepted` also proves a second run changes nothing.
        assert_eq!(clean_accepted(source, &rules), "1. one\n    - run it Note: then check\n");
    }

    #[test]
    fn tab_indented_items_are_padded_once() {
        let rules = only(|o| o.list_item_padding = Leaf::Value(crate::markdown::cleanup::rules::ListItemPadding::One));
        let source = "- a\n\n    para\n\n\t- x\n\t- y\n\t- z\n";
        assert_eq!(clean_accepted(source, &rules), "- a\n\n    para\n\n\t- x\n\n\t- y\n\n\t- z\n");
    }
}

mod corpus_regressions_other_rules {
    use super::super::body_support::{clean_accepted, only};
    use crate::markdown::cleanup::rules::{FixedWidth, Leaf, ListInteriorPadding, ListItemPadding};

    #[test]
    fn joining_keeps_an_escape_that_starts_the_next_line() {
        let unwrap = only(|o| o.unwrap = Leaf::Value(true));
        assert_eq!(clean_accepted("> **api**\n> \\<name\\>\n", &unwrap), "> **api** \\<name\\>\n");
        let width = only(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(12)));
        assert_eq!(clean_accepted("> **api** \\<name\\> more\n", &width), "> **api**\n> \\<name\\>\n> more\n");
    }

    #[test]
    fn two_quotes_in_an_item_keep_their_separating_blank_line() {
        let none = only(|o| o.list_interior_padding = Leaf::Value(ListInteriorPadding::None));
        let source = "- item\n\n  > first\n\n  > second\n";
        assert_eq!(clean_accepted(source, &none), "- item\n  > first\n\n  > second\n");
    }

    #[test]
    fn no_blank_line_is_added_after_an_unclosed_fence() {
        let one = only(|o| o.list_item_padding = Leaf::Value(ListItemPadding::One));
        let source = "- a\n  ```\n  x\n- b\n";
        assert_eq!(clean_accepted(source, &one), source);
    }
}
