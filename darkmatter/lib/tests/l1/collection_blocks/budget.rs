//! Request-wide visit and expansion allowances, run for both kinds.

use std::num::NonZeroU64;
use std::path::Path;

use darkmatter::markdown::compose::ComposeOperation;
use darkmatter::markdown::compose::collection_blocks::CollectionKind;
use serde_json::{Value, json};
use tempfile::TempDir;

use super::support::{KINDS, closer, opener, region};
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;

use crate::collection_execution::{any_items, codes, compose_configured, compose_with_options};

fn limit(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).unwrap()
}

/// A raw (cleanup disabled) composition under the given allowances.
fn run(document: &str, visits: Option<u64>, bytes: Option<u64>) -> Result<String, darkmatter::markdown::MarkdownError> {
    compose_configured(document, |options| {
        let mut options = options.disable(ComposeOperation::Cleanup);
        if let Some(visits) = visits {
            options = options.with_max_iteration_visits(limit(visits));
        }
        if let Some(bytes) = bytes {
            options = options.with_max_expansion_bytes(limit(bytes));
        }
        options
    })
}

fn doc(kind: CollectionKind, items: Value, header: &str, body: &str) -> String {
    any_items(items, &region(kind, header, body))
}

#[track_caller]
fn assert_boundary(document: &str, visits: Option<(u64, u64)>, bytes: Option<(u64, u64)>, what: &str) {
    if let Some((fits, fails)) = visits {
        assert!(run(document, Some(fits), None).is_ok(), "{what}: {fits} visits should fit");
        let error = run(document, Some(fails), None).unwrap_err();
        assert_eq!(codes(&error), ["dm.iteration.visit_limit"], "{what}: {error}");
        assert_located(&error, what);
    }
    if let Some((fits, fails)) = bytes {
        assert!(run(document, None, Some(fits)).is_ok(), "{what}: {fits} bytes should fit");
        let error = run(document, None, Some(fails)).unwrap_err();
        assert_eq!(codes(&error), ["dm.iteration.expansion_limit"], "{what}: {error}");
        assert_located(&error, what);
    }
}

/// Every exhaustion names an authored line and the item it was paying for.
#[track_caller]
fn assert_located(error: &darkmatter::markdown::MarkdownError, what: &str) {
    let [record] = error_diagnostics(error).try_into().unwrap_or_else(|records| panic!("{what}: {records:#?}"));
    assert!(record.source.is_some(), "{what}: {record:?}");
    assert!(!record.iteration_path.is_empty(), "{what}: {record:?}");
}

#[test]
fn the_smallest_positive_limits_admit_exactly_one_visit_and_its_bytes_for_both_kinds() {
    for kind in KINDS {
        let one = doc(kind, json!(["a"]), "data", "x\n");
        assert_boundary(&one, None, Some((2, 1)), &format!("{kind:?} one item"));
        assert!(run(&one, Some(1), None).is_ok(), "{kind:?}: one visit fits one item");
        assert!(run(&one, Some(1), Some(2)).is_ok(), "{kind:?}: both smallest allowances together");
        let two = doc(kind, json!(["a", "b"]), "data", "x\n");
        let error = run(&two, Some(1), None).unwrap_err();
        assert_eq!(codes(&error), ["dm.iteration.visit_limit"], "{kind:?}");
    }
}

#[test]
fn visit_and_byte_boundaries_hold_for_both_kinds() {
    for kind in KINDS {
        let four = doc(kind, json!([1, 2, 3, 4]), "data", "- {{i}}\n");
        // Each copy is the 8 authored bytes plus one inserted byte.
        assert_boundary(&four, Some((4, 3)), Some((36, 35)), &format!("{kind:?}"));
    }
}

#[test]
fn rejected_and_stopping_items_are_visits_and_empty_bodies_still_consume_them_for_both_kinds() {
    for kind in KINDS {
        let rejected = doc(kind, json!([1, 2, 3, 4, 5]), "data filter=\"i => i == 1\"", "- {{i}}\n");
        assert_boundary(&rejected, Some((5, 4)), None, &format!("{kind:?} rejected"));
        let stopped = doc(kind, json!([1, 2, 3, 4]), "data until=\"i => i == 3\"", "- {{i}}\n");
        assert_boundary(&stopped, Some((3, 2)), None, &format!("{kind:?} stopped"));
        let sorted = doc(kind, json!([3, 2, 1]), "data sort=\"i => i\" while=\"i => i < 2\"", "- {{i}}\n");
        assert_boundary(&sorted, Some((3, 2)), None, &format!("{kind:?} sorted"));
        let empty_body = doc(kind, json!([1, 2]), "data", "");
        assert_boundary(&empty_body, Some((2, 1)), None, &format!("{kind:?} empty body"));
        assert_eq!(run(&empty_body, Some(2), Some(1)).unwrap(), "", "{kind:?}: an empty body costs no bytes");
    }
}

#[test]
fn bytes_are_utf8_bytes_not_characters() {
    for kind in KINDS {
        // `é\n` is three bytes; two copies are six.
        let document = doc(kind, json!([1, 2]), "data", "é\n");
        assert_boundary(&document, None, Some((6, 5)), &format!("{kind:?}"));
        // An inserted multibyte value is charged at its encoded length.
        let inserted = doc(kind, json!(["é"]), "data", "{{i}}\n");
        // 6 authored bytes (`{{i}}\n`) plus two for the value.
        assert_boundary(&inserted, None, Some((8, 7)), &format!("{kind:?} inserted"));
    }
}

#[test]
fn an_expression_value_is_not_charged_for_the_array_it_was_computed_from() {
    let big: Vec<Value> = (0..100).map(|number| json!(number)).collect();
    for kind in KINDS {
        let items = json!([big.clone(), big.clone(), big.clone()]);
        let document = doc(kind, items, "data", "{{ length(i) }}\n");
        // 16 authored bytes plus the three-byte `100`, three times.
        assert_boundary(&document, None, Some((57, 56)), &format!("{kind:?}"));
    }
}

#[test]
fn a_failure_before_an_excessive_append_returns_no_partial_document() {
    for kind in KINDS {
        let document = doc(kind, json!(["a", "b", "c"]), "data", "- {{i}}\n");
        let error = run(&document, None, Some(10)).unwrap_err();
        assert_eq!(codes(&error), ["dm.iteration.expansion_limit"], "{kind:?}: {error}");
        assert_located(&error, &format!("{kind:?}"));
    }
}

#[test]
fn text_and_stage_passes_outside_every_region_are_never_charged() {
    for kind in KINDS {
        let prose = "unrelated {{ 1 + 1 }} text ".repeat(40);
        let body = format!("{prose}\n::block when=\"true\"\nkept {{{{ fm.note }}}}\n::end-block\n\n{}", region(kind, "data", "x\n"));
        let document = format!(
            "---\n$schema:\n  data: any[](required)\n  note: string(required)\ndata: [1]\nnote: n\nreplace:\n  PLAIN: swapped\n---\nPLAIN {body}"
        );
        assert!(run(&document, Some(1), Some(2)).is_ok(), "{kind:?}");
    }
}

// ── shared allowances across nesting, nested composition, and children ───

#[test]
fn nested_regions_of_every_kind_pair_share_one_allowance_of_each_kind() {
    for outer in KINDS {
        for inner in KINDS {
            let document = format!(
                "---\n$schema:\n  outer: string[](required)\n  inner: string[](required)\nouter: [a, b]\ninner: [x, y]\n---\n{}",
                region(outer, "outer", &region(inner, "inner", "- {{i}}\n")),
            );
            // 2 outer visits + 2 x 2 inner visits.
            assert_boundary(&document, Some((6, 5)), None, &format!("{outer:?}/{inner:?}"));
        }
    }
}

#[test]
fn a_nested_composition_draws_from_the_enclosing_allowance_for_both_kinds() {
    for kind in KINDS {
        let nested = format!(
            "---\\n$schema:\\n  d: 'string[](required)'\\nd: [a, b]\\n---\\n::{} d\\n- {{{{i}}}}\\n::{}\\n",
            kind.opener(),
            kind.closer()
        );
        let document = doc(kind, json!([1, 2]), "data", &format!("{{{{ as_markdown(\"{nested}\") }}}}\n"));
        // 2 outer visits + 2 x 2 nested visits.
        assert_boundary(&document, Some((6, 5)), None, &format!("{kind:?}"));
    }
}

/// Children `a.md`, `b.md`, `c.md`, each repeating two items in `kind`.
fn write_children(root: &Path, kind: CollectionKind) {
    for name in ["a", "b", "c"] {
        let child = format!(
            "---\n$schema:\n  d: string[](required)\nd: [x, y]\n---\n{} d\n- {name}{{{{i}}}}\n{}\n",
            opener(kind),
            closer(kind)
        );
        std::fs::write(root.join(format!("{name}.md")), child).unwrap();
    }
}

fn run_in(root: &Path, document: &str, visits: u64) -> Result<String, darkmatter::markdown::MarkdownError> {
    compose_with_options(root, document, |options| options.with_max_iteration_visits(limit(visits)))
}

#[test]
fn distinct_children_spend_the_parents_visit_allowance_for_both_kinds() {
    for kind in KINDS {
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        write_children(&root, kind);
        let document = doc(kind, json!(["a.md", "b.md", "c.md"]), "data", "::file {{i}}\n");
        // 3 parent visits + 3 children x 2 visits.
        let output = run_in(&root, &document, 9).unwrap_or_else(|error| panic!("{kind:?}: {error}"));
        assert!(output.contains("ax") && output.contains("cy"), "{kind:?}: {output}");
        let error = run_in(&root, &document, 8).unwrap_err();
        assert_eq!(codes(&error), ["dm.iteration.visit_limit"], "{kind:?}: {error}");
        assert_located(&error, &format!("{kind:?}"));
    }
}

#[test]
fn a_cached_child_is_charged_again_for_every_insertion_and_every_visit() {
    for kind in KINDS {
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        write_children(&root, kind);
        let reused = |count: usize| {
            let names: Vec<_> = vec!["a.md"; count];
            doc(kind, json!(names), "data", "::file ./a.md\n")
        };
        let bytes_fit = |document: &str, bytes: u64| {
            compose_with_options(&root, document, |options| {
                options.disable(ComposeOperation::Cleanup).with_max_expansion_bytes(limit(bytes))
            })
            .is_ok()
        };
        let minimum = |document: &str| -> u64 {
            let (mut low, mut high) = (1_u64, 512_u64);
            assert!(bytes_fit(document, high), "{kind:?}: 512 bytes should fit");
            while low < high {
                let middle = (low + high) / 2;
                if bytes_fit(document, middle) { high = middle } else { low = middle + 1 }
            }
            low
        };
        let (one, three) = (minimum(&reused(1)), minimum(&reused(3)));
        assert_eq!(three, one * 3, "{kind:?}: each insertion of the same child costs what the first did");

        // Visits replay the same way: 3 parent visits + 3 replays of the 2 the child paid.
        let document = reused(3);
        assert!(run_in(&root, &document, 9).is_ok(), "{kind:?}");
        let error = run_in(&root, &document, 8).unwrap_err();
        assert_eq!(codes(&error), ["dm.iteration.visit_limit"], "{kind:?}: {error}");
        assert_located(&error, &format!("{kind:?} replay"));
    }
}

// ── the first budget failure stops the effects after it ──────────────────

#[cfg(feature = "effects-instrumentation")]
#[test]
fn no_body_expression_runs_after_the_allowance_is_spent() {
    use crate::collection_effects::{log, probes};
    for kind in KINDS {
        let (_probe, _boom) = probes();
        let document = doc(kind, json!([1, 2, 3, 4]), "data", "{{ probe('body', i) }}\n");
        // One copy is 22 authored bytes plus one inserted: the second must not run.
        let error = run(&document, None, Some(23)).unwrap_err();
        assert_eq!(codes(&error), ["dm.iteration.expansion_limit"], "{kind:?}");
        let entries = log();
        assert!(entries.len() <= 2, "{kind:?}: {entries:?}");
        assert!(!entries.iter().any(|entry| entry == "body:3" || entry == "body:4"), "{kind:?}: {entries:?}");
    }
}
