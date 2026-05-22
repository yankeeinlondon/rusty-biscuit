//! Unit tests and snapshot tests for darkmatter's layout visual-test matrix.
//!
//! Regenerate snapshots after intentional changes with:
//! `INSTA_UPDATE=always cargo test -p darkmatter --test layout_matrix`
//! then review the `.snap` diff.

mod layout_matrix_support;

use layout_matrix_support::{component_cases, scenarios, stacked_stripped};

#[test]
fn scenario_count_is_eleven() {
    // The legacy `row_fill_fill` scenario was dropped: `RowFill` has no
    // equivalent on the new `renderable::layout::Layout`.
    assert_eq!(scenarios().len(), 11);
}

#[test]
fn component_case_count_is_one() {
    assert_eq!(component_cases().len(), 1);
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

#[test]
fn stacked_stripped_has_no_ansi() {
    let block = stacked_stripped("\x1b[1mhi\x1b[0m", "\x1b[2mbye\x1b[0m");
    assert!(!block.contains('\x1b'), "snapshot block must be ANSI-free");
}

#[test]
fn layout_matrix_snapshots() {
    for case in component_cases() {
        for scenario in scenarios() {
            let (bespoke, tree) = (case.render)(&scenario);
            let block = stacked_stripped(&bespoke, &tree);
            insta::assert_snapshot!(format!("{}__{}", case.name, scenario.name), block);
        }
    }
}
