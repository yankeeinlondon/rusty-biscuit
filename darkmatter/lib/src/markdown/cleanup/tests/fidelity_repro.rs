//! The reproductions that motivated guarded cleanup (E1, D1, C1, A1, B1, F1):
//! exact output under the defaults and with every body rule off, and kept
//! structure under every non-default list rule.

use super::body_support::{clean_accepted, only, profiles, rules_with, structure};
use crate::markdown::cleanup::rules::{
    FixedWidth, Indentation, Leaf, ListInteriorPadding, ListItemPadding, ResolvedMarkdownRules,
};
use crate::markdown::cleanup::{CleanOutcome, clean_body};

const E1: &str = "4. step\n\n    > lead:\n    > \n    > - a\n    > - b\n\n5. next\n";
const D1: &str = "1. top\n    - Example:\n\n        ```\n        code\n        ```\n\n        after\n2. next\n";
const C1: &str = "- item:\n\n    > {{message}}\n\n- next\n";
const A1: &str = "::block when=\"a\"\n\n> note\n::end-block\n";
const B1: &str = "1. first\n::block when=\"m\"\n    - nested\n::end-block\n2. second\n";
const F1: &str = "> a\n>\n> b\n";

const REPRODUCTIONS: [(&str, &str); 6] = [("E1", E1), ("D1", D1), ("C1", C1), ("A1", A1), ("B1", B1), ("F1", F1)];

#[test]
fn defaults_leave_every_reproduction_byte_identical_except_d1_gains_a_language() {
    let rules = ResolvedMarkdownRules::defaults();
    for (name, source) in REPRODUCTIONS {
        let expected = if name == "D1" {
            "1. top\n    - Example:\n\n        ```text\n        code\n        ```\n\n        after\n2. next\n"
        } else {
            source
        };
        assert_eq!(clean_accepted(source, &rules), expected, "{name}");
    }
}

#[test]
fn d1_language_is_added_at_column_eight() {
    let cleaned = clean_accepted(D1, &ResolvedMarkdownRules::defaults());
    let fence = cleaned.lines().find(|line| line.contains("```text")).unwrap();
    assert_eq!(fence.find('`'), Some(8));
}

#[test]
fn every_body_rule_off_returns_every_reproduction_exactly() {
    let rules = ResolvedMarkdownRules::all_body_rules_off();
    for (name, source) in REPRODUCTIONS {
        let cleaned = clean_body(source, &rules);
        assert_eq!(cleaned.outcome, CleanOutcome::Unchanged, "{name}");
        assert_eq!(cleaned.text, source, "{name}");
    }
}

#[test]
fn reproductions_keep_their_structure_under_every_rule_value() {
    for (name, source) in REPRODUCTIONS {
        for (profile, rules) in profiles() {
            let cleaned = clean_accepted(source, &rules);
            let padding = rules.permits_tightness_change();
            assert_eq!(
                structure(&cleaned, padding),
                structure(source, padding),
                "{name} under {profile} changed structure: {cleaned:?}"
            );
        }
    }
}

/// The non-default values acceptance criterion 1 names, each set alone over
/// the defaults.
fn named_values() -> [(&'static str, ResolvedMarkdownRules); 10] {
    let interior = |padding| rules_with(move |o| o.list_interior_padding = Leaf::Value(padding));
    [
        ("indentation: 2", rules_with(|o| o.indentation = Leaf::Value(Indentation::Two))),
        ("indentation: 8", rules_with(|o| o.indentation = Leaf::Value(Indentation::Eight))),
        ("indentation: preserve", rules_with(|o| o.indentation = Leaf::Value(Indentation::Preserve))),
        ("unwrap: true", rules_with(|o| o.unwrap = Leaf::Value(true))),
        ("fixed-width: 80", rules_with(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(80)))),
        ("list-item-padding: none", rules_with(|o| o.list_item_padding = Leaf::Value(ListItemPadding::None))),
        ("list-item-padding: one", rules_with(|o| o.list_item_padding = Leaf::Value(ListItemPadding::One))),
        ("list-interior-padding: none", interior(ListInteriorPadding::None)),
        ("list-interior-padding: except-nested-lists", interior(ListInteriorPadding::ExceptNestedLists)),
        ("list-interior-padding: all", interior(ListInteriorPadding::All)),
    ]
}

#[test]
fn e1_c1_d1_produce_exact_text_under_every_named_rule_value() {
    // D1 under the defaults: only the fence gains `text`.
    const D1_LABELED: &str =
        "1. top\n    - Example:\n\n        ```text\n        code\n        ```\n\n        after\n2. next\n";
    // Expected text per value, in `named_values` order. A quote inside an item
    // is not a nested list, so no indentation value moves E1 or C1.
    let expected: [(&str, &str, [&str; 10]); 3] = [
        (
            "E1",
            E1,
            [
                E1,
                E1,
                E1,
                E1,
                E1,
                // The outer list stays loose: the item's interior blank line remains.
                "4. step\n\n    > lead:\n    > \n    > - a\n    > - b\n5. next\n",
                // The quoted list's siblings get one blank line inside the quote,
                // with no trailing space after its marker.
                "4. step\n\n    > lead:\n    > \n    > - a\n    >\n    > - b\n\n5. next\n",
                "4. step\n    > lead:\n    > \n    > - a\n    > - b\n\n5. next\n",
                E1,
                E1,
            ],
        ),
        (
            "C1",
            C1,
            [
                C1,
                C1,
                C1,
                C1,
                C1,
                "- item:\n\n    > {{message}}\n- next\n",
                C1,
                "- item:\n    > {{message}}\n\n- next\n",
                C1,
                C1,
            ],
        ),
        (
            "D1",
            D1,
            [
                // `1.` sets a minimum continuation of 3, so 2 becomes 3, and the
                // fence and the paragraph move with their item.
                "1. top\n   - Example:\n\n       ```text\n       code\n       ```\n\n       after\n2. next\n",
                // 8 is out of reach of a one-space `1.` marker, so the marker
                // gains padding.
                "1.   top\n        - Example:\n\n            ```text\n            code\n            ```\n\n            after\n2. next\n",
                D1_LABELED,
                D1_LABELED,
                D1_LABELED,
                D1_LABELED,
                "1. top\n    - Example:\n\n        ```text\n        code\n        ```\n\n        after\n\n2. next\n",
                // The blank line before the nested paragraph is required and stays.
                "1. top\n    - Example:\n        ```text\n        code\n        ```\n\n        after\n2. next\n",
                D1_LABELED,
                "1. top\n\n    - Example:\n\n        ```text\n        code\n        ```\n\n        after\n2. next\n",
            ],
        ),
    ];
    for (name, source, outputs) in expected {
        for ((value, rules), output) in named_values().iter().zip(outputs) {
            assert_eq!(clean_accepted(source, rules), output, "{name} under {value}");
        }
    }
}

#[test]
fn indentation_preserve_unwrap_and_width_leave_the_list_reproductions_alone() {
    let profiles = [
        rules_with(|o| o.indentation = Leaf::Value(Indentation::Preserve)),
        rules_with(|o| o.unwrap = Leaf::Value(true)),
        rules_with(|o| o.fixed_width = Leaf::Value(FixedWidth::Columns(80))),
    ];
    for rules in &profiles {
        assert_eq!(clean_accepted(C1, rules), C1);
        assert_eq!(clean_accepted(E1, rules), E1);
        assert_eq!(clean_accepted(B1, rules), B1);
    }
}

#[test]
fn multi_digit_markers_set_the_minimum_continuation_indent() {
    // `10.` is four columns wide: a 2-space request cannot be used literally.
    let source = "10. parent\n    - child\n";
    let two = rules_with(|o| o.indentation = Leaf::Value(Indentation::Two));
    assert_eq!(clean_accepted(source, &two), source);
    // 8 fits within three columns of `10. `'s content column with one more space.
    let eight = rules_with(|o| o.indentation = Leaf::Value(Indentation::Eight));
    assert_eq!(clean_accepted(source, &eight), "10.  parent\n        - child\n");
}

#[test]
fn code_children_move_with_their_item_when_the_requested_indent_is_clamped() {
    let source = "10. parent\n\n    ```\n    code\n    ```\n\n    - child\n\n          indented code\n";
    let rules = only(|o| o.indentation = Leaf::Value(Indentation::Two));
    // The nested list stays at the clamped column 4; its indented code keeps
    // its relative indentation.
    assert_eq!(clean_accepted(source, &rules), source);

    let source = "- parent\n  - child\n\n        indented code\n";
    let rules = only(|o| o.indentation = Leaf::Value(Indentation::Four));
    let cleaned = clean_accepted(source, &rules);
    assert_eq!(cleaned, "- parent\n    - child\n\n          indented code\n");
    assert_eq!(structure(&cleaned, false), structure(source, false));
}

#[test]
fn b1_directive_lines_stay_fixed_while_the_list_spanning_them_is_reindented() {
    let rules = rules_with(|o| o.indentation = Leaf::Value(Indentation::Two));
    assert_eq!(
        clean_accepted(B1, &rules),
        "1. first\n::block when=\"m\"\n   - nested\n::end-block\n2. second\n"
    );
}
