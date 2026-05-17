//! Unit tests and snapshot tests for the layout visual-test matrix.
//!
//! Regenerate snapshots after intentional changes with:
//! `INSTA_UPDATE=always cargo test -p biscuit-terminal --test layout_matrix`
//! then review the `.snap` diff.

mod layout_matrix_support;

use layout_matrix_support::scenarios;

#[test]
fn scenario_count_is_twelve() {
    assert_eq!(scenarios().len(), 12);
}

#[test]
fn scenario_names_are_unique() {
    let names: Vec<&str> = scenarios().iter().map(|s| s.name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "scenario names must be unique");
}

use layout_matrix_support::{pad, side_by_side, stacked_stripped, visible_width};

#[test]
fn pad_extends_to_width() {
    assert_eq!(pad("ab", 5), "ab   ");
    assert_eq!(visible_width(&pad("ab", 5)), 5);
}

#[test]
fn pad_is_ansi_aware() {
    let styled = "\x1b[1mab\x1b[0m";
    assert_eq!(visible_width(&pad(styled, 5)), 5);
}

#[test]
fn stacked_stripped_has_no_ansi() {
    let block = stacked_stripped("\x1b[1mhi\x1b[0m", "\x1b[2mbye\x1b[0m");
    assert!(!block.contains('\x1b'), "snapshot block must be ANSI-free");
    assert!(block.contains("hi") && block.contains("bye"));
}

#[test]
fn side_by_side_includes_title_and_both_columns() {
    let out = side_by_side("My Title", "left", "right", 20);
    assert!(out.contains("My Title"));
    assert!(out.contains("left") && out.contains("right"));
}

use layout_matrix_support::component_cases;

#[test]
fn component_case_count_is_six() {
    assert_eq!(component_cases().len(), 6);
}

#[test]
fn every_case_renders_non_empty() {
    for case in component_cases() {
        for scenario in scenarios() {
            let (bespoke, tree) = (case.render)(&scenario);
            assert!(
                !bespoke.is_empty(),
                "{}/{}: bespoke output was empty",
                case.name,
                scenario.name
            );
            assert!(
                !tree.is_empty(),
                "{}/{}: tree output was empty",
                case.name,
                scenario.name
            );
        }
    }
}
