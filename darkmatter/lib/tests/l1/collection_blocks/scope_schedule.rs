//! Local scopes, selection order, and sort keys, run for both kinds.

use darkmatter::markdown::compose::collection_blocks::CollectionKind;
use serde_json::{Value, json};

use super::support::{KINDS, closer, opener, region, row_tests};
use super::support::composed_raw;
use crate::collection_execution::{any_items, codes, compose, compose_with_options, composed};

/// An `any[]` document of `items` whose region of `kind` has `header` and `body`.
fn any_doc(kind: CollectionKind, items: Value, header: &str, body: &str) -> String {
    any_items(items, &region(kind, header, body))
}

/// The callback that keeps every item unchanged: identity for map, an
/// always-true predicate for filter.
fn keep_all(kind: CollectionKind, param: &str) -> String {
    match kind {
        CollectionKind::Map => format!("{param} => {param}"),
        CollectionKind::Filter => format!("{param} => true"),
    }
}

// ── protected names ──────────────────────────────────────────────────────

const PROTECTED_SLOTS: [&str; 5] = ["", "filter=", "sort=", "while=", "until="];

fn protected_name(name: &str) {
    for kind in KINDS {
        for slot in PROTECTED_SLOTS {
            let header = if slot.is_empty() {
                format!("data \"{name} => 1\"")
            } else {
                format!("data {slot}\"{name} => 1\"")
            };
            let document = any_doc(kind, json!([1]), &header, "- x\n");
            let error = compose(&document).expect_err(&format!("{name} {kind:?} {slot}"));
            assert_eq!(codes(&error), ["dm.expression.invalid_callback"], "{name} {kind:?} {slot}: {error}");
        }
    }
}

const PROTECTED_NAMES: &[&str] = &["fm", "ctx", "env", "current", "local", "null", "true", "false"];

fn protected_name_row(row: &str) {
    protected_name(row.strip_prefix("protected_").unwrap());
}

row_tests! { PROTECTED_TESTS, protected_name_row;
    protected_fm, protected_ctx, protected_env, protected_current, protected_local, protected_null,
    protected_true, protected_false,
}

#[test]
fn every_generated_protected_name_test_is_in_the_table() {
    let generated: Vec<_> = PROTECTED_TESTS.iter().map(|name| name.strip_prefix("protected_").unwrap()).collect();
    assert_eq!(generated, PROTECTED_NAMES);
}

#[test]
fn a_pattern_field_may_not_be_a_protected_name_in_either_kind() {
    for kind in KINDS {
        let document = any_doc(kind, json!([{"ctx": 1}]), "data \"{ ctx } => 1\"", "- x\n");
        assert_eq!(codes(&compose(&document).unwrap_err()), ["dm.expression.invalid_callback"], "{kind:?}");
    }
}

// ── explicit and implicit reads, shadowing, captures ─────────────────────

#[test]
fn explicit_and_implicit_local_reads_agree_in_bodies_and_callbacks_for_both_kinds() {
    for kind in KINDS {
        let header = match kind {
            CollectionKind::Map => "data \"p => local.p\"",
            CollectionKind::Filter => "data \"p => local.p == 'a'\"",
        };
        let document = any_doc(kind, json!(["a", "b"]), header, "- {{p}} {{local.p}}\n");
        let expected = match kind {
            CollectionKind::Map => "- a a\n- b b\n",
            CollectionKind::Filter => "- a a\n",
        };
        assert_eq!(composed(&document), expected, "{kind:?}");
    }
}

#[test]
fn a_present_null_local_shadows_the_frontmatter_property_but_fm_escapes() {
    for kind in KINDS {
        let document = format!(
            "---\n$schema:\n  data: any[](required)\n  v: string(required)\ndata: [null]\nv: outer\n---\n{}",
            region(kind, &format!("data \"{}\"", keep_all(kind, "v")), "- [{{v}}] [{{local.v}}] [{{fm.v}}]\n"),
        );
        assert_eq!(composed(&document), "- [] [] [outer]\n", "{kind:?}");
    }
}

#[test]
fn callbacks_capture_frontmatter_and_enclosing_locals() {
    for kind in KINDS {
        let predicate = "\"y => y != x\"";
        let projection = "\"y => y\"";
        let inner_callback = if kind == CollectionKind::Map { projection } else { predicate };
        let document = format!(
            "---\n$schema:\n  outer: number[](required)\n  inner: number[](required)\nouter: [1, 2]\ninner: [1, 2]\n---\n{}",
            region(
                CollectionKind::Filter,
                "outer \"x => true\"",
                &region(kind, &format!("inner {inner_callback}"), "- {{x}}:{{y}}\n"),
            ),
        );
        let expected = match kind {
            CollectionKind::Map => "- 1:1\n- 1:2\n- 2:1\n- 2:2\n",
            CollectionKind::Filter => "- 1:2\n- 2:1\n",
        };
        assert_eq!(composed(&document), expected, "{kind:?}");
    }
}

#[test]
fn a_source_named_like_its_parameter_is_read_in_the_enclosing_scope() {
    for kind in KINDS {
        let callback = if kind == CollectionKind::Map { "\"data => data\"" } else { "\"data => true\"" };
        let document = any_doc(kind, json!(["a", "b"]), &format!("data {callback}"), "- {{data}}\n");
        assert_eq!(composed(&document), "- a\n- b\n", "{kind:?}");
    }
}

#[test]
fn an_inner_source_is_read_through_the_outer_local_with_the_same_spelling() {
    for kind in KINDS {
        let callback = if kind == CollectionKind::Map { "\"team => team\"" } else { "\"team => true\"" };
        let document = format!(
            "---\n$schema:\n  teams: '{{ members: string[](required) }}[](required)'\nteams: [{{ members: [a, b] }}]\n---\n{}",
            region(kind, "teams \"team => team\"", &region(kind, &format!("team.members {callback}"), "- {{team}}\n")),
        );
        let document = if kind == CollectionKind::Filter {
            document.replacen("\"team => team\"", "\"team => true\"", 1)
        } else {
            document
        };
        assert_eq!(composed(&document), "- a\n- b\n", "{kind:?}");
    }
}

#[test]
fn each_option_binds_its_own_name_and_only_the_positional_reaches_the_body() {
    for kind in KINDS {
        let positional = if kind == CollectionKind::Map { "\"p => p * 10\"" } else { "\"p => p > 1\"" };
        let document = any_doc(
            kind,
            json!([4, 3, 2, 1]),
            &format!("data {positional} filter=\"q => q < 4\" sort=\"r => r\" while=\"s => s < 4\""),
            "- {{p}}\n",
        );
        let expected = match kind {
            // The map's positional transforms rather than selects: `filter=` drops 4, sorting orders the rest.
            CollectionKind::Map => "- 10\n- 20\n- 30\n",
            CollectionKind::Filter => "- 2\n- 3\n",
        };
        assert_eq!(composed(&document), expected, "{kind:?}");
    }
}

// ── nesting cross product ────────────────────────────────────────────────

#[test]
fn every_nesting_of_the_two_kinds_restores_the_outer_local_after_the_inner_region() {
    for outer in KINDS {
        for inner in KINDS {
            let document = format!(
                "---\n$schema:\n  a: string[](required)\n  b: string[](required)\na: [x, y]\nb: [1]\n---\n{}",
                region(
                    outer,
                    &format!("a \"{}\"", keep_all(outer, "i")),
                    &format!("{}after-{{{{i}}}}[{{{{j}}}}]\n", region(inner, &format!("b \"{}\"", keep_all(inner, "j")), "{{i}}{{j}}\n")),
                ),
            );
            assert_eq!(composed_raw(&document), "x1\nafter-x[]\ny1\nafter-y[]\n", "{outer:?}/{inner:?}");
        }
    }
}

#[test]
fn a_conditional_block_gates_a_nested_collection_on_both_sides() {
    for outer in KINDS {
        for inner in KINDS {
            let document = format!(
                "---\n$schema:\n  a: number[](required)\n  b: number[](required)\na: [1, 2]\nb: [7]\n---\n{}",
                region(
                    outer,
                    &format!("a \"{}\"", keep_all(outer, "i")),
                    &format!(
                        "::block when=\"i > 1\"\n{}::end-block\n",
                        region(inner, &format!("b \"{}\"", keep_all(inner, "j")), "{{i}}-{{j}}\n"),
                    ),
                ),
            );
            assert_eq!(composed(&document), "2-7\n", "{outer:?}/{inner:?}");
        }
    }
}

// ── sort keys ────────────────────────────────────────────────────────────

#[test]
fn strings_sort_by_code_point_not_by_locale_for_both_kinds_and_orders() {
    for kind in KINDS {
        for (order, expected) in [("asc", "- Z\n- a\n- e\n- z\n- é\n"), ("desc", "- é\n- z\n- e\n- a\n- Z\n")] {
            let document = any_doc(
                kind,
                json!(["é", "z", "a", "Z", "e"]),
                &format!("data \"{}\" sort=\"s => s\" sort-order=\"{order}\"", keep_all(kind, "s")),
                "- {{ s }}\n",
            );
            assert_eq!(composed(&document), expected, "{kind:?} {order}");
        }
    }
}

const BAD_KEY_FAMILIES: &[(&str, &str)] = &[
    ("null", "[null]"),
    ("boolean", "[true]"),
    ("object", r#"[{"a": 1}]"#),
    ("array", "[[1]]"),
    ("mixed", r#"[1, "a"]"#),
    ("two_nulls", "[null, null]"),
    ("number_then_null", "[1, null]"),
];

fn bad_key_family(name: &str) {
    let (_, items) = BAD_KEY_FAMILIES.iter().find(|(row, _)| *row == name).unwrap();
    let items: Value = serde_json::from_str(items).unwrap();
    for kind in KINDS {
        for order in ["asc", "desc"] {
            let header = format!("data \"{}\" sort=\"v => v\" sort-order=\"{order}\"", keep_all(kind, "v"));
            let document = any_doc(kind, items.clone(), &header, "- x\n");
            let error = compose(&document).expect_err(&format!("{name} {kind:?} {order}"));
            assert_eq!(codes(&error), ["dm.collection.invalid_sort_key"], "{name} {kind:?} {order}: {error}");
        }
    }
}

fn bad_key_family_row(row: &str) {
    bad_key_family(row.strip_prefix("key_").unwrap());
}

row_tests! { KEY_TESTS, bad_key_family_row;
    key_null, key_boolean, key_object, key_array, key_mixed, key_two_nulls, key_number_then_null,
}

#[test]
fn every_generated_sort_key_test_is_in_the_table() {
    let table: Vec<_> = BAD_KEY_FAMILIES.iter().map(|(name, _)| *name).collect();
    let generated: Vec<_> = KEY_TESTS.iter().map(|name| name.strip_prefix("key_").unwrap()).collect();
    assert_eq!(generated, table);
}

#[test]
fn a_bad_key_on_an_unselected_item_is_never_computed() {
    for kind in KINDS {
        let positional = format!("\"{}\"", keep_all(kind, "v"));
        let document = any_doc(
            kind,
            json!([null, 2, 1]),
            &format!("data {positional} filter=\"v => v\" sort=\"v => v\""),
            "- {{ v }}\n",
        );
        assert_eq!(composed(&document), "- 1\n- 2\n", "{kind:?}");
    }
}

#[test]
fn a_single_selected_item_with_a_bad_key_still_fails_after_selection() {
    for kind in KINDS {
        let positional = format!("\"{}\"", keep_all(kind, "v"));
        let document = any_doc(
            kind,
            json!([1, true, 2]),
            &format!("data {positional} filter=\"v => v == true\" sort=\"v => v\""),
            "- x\n",
        );
        assert_eq!(codes(&compose(&document).unwrap_err()), ["dm.collection.invalid_sort_key"], "{kind:?}");
    }
}

#[test]
fn a_missing_field_key_is_invalid_and_a_present_number_key_sorts() {
    for kind in KINDS {
        let callback = if kind == CollectionKind::Map { "\"o => o\"" } else { "\"o => true\"" };
        let sorted = any_doc(kind, json!([{"n": 2}, {"n": 1}]), &format!("data {callback} sort=\"o => o.n\""), "- {{ o.n }}\n");
        assert_eq!(composed(&sorted), "- 1\n- 2\n", "{kind:?}");
        let missing = any_doc(kind, json!([{"n": 2}, {"m": 1}]), &format!("data {callback} sort=\"o => o.n\""), "- x\n");
        assert_eq!(codes(&compose(&missing).unwrap_err()), ["dm.collection.invalid_sort_key"], "{kind:?}");
    }
}

#[test]
fn the_region_text_survives_every_row_builder() {
    // Guards `region`/`opener`/`closer` against drifting from the directive names.
    for kind in KINDS {
        assert_eq!(region(kind, "x", "b\n"), format!("{} x\nb\n{}\n", opener(kind), closer(kind)));
    }
}

// ── selection facts across destructured and object spellings ─────────────

/// Items whose `members` only a selection guard proves to be an array.
const MEMBERS: &str = "---\n$schema:\n  data: \"{ members: any(required), other: any(required) }[](required)\"\ndata: [{members: [a, b], other: x}]\n---\n";

/// The selection option for `slot` that retains items where `test` holds:
/// `until=` stops at the first item where it holds, so it takes the negation.
fn retaining(slot: &str, param: &str, test: &str) -> String {
    match slot {
        "until" => format!("until=\"{param} => !({test})\""),
        _ => format!("{slot}=\"{param} => {test}\""),
    }
}

/// Composes a `kind` region over [`MEMBERS`] with `positional`, a retaining
/// `slot` guard, and a nested `::map inner` body.
fn guarded_members(kind: CollectionKind, positional: &str, slot: &str, param: &str, test: &str, inner: &str) -> String {
    let header = format!("data \"{positional}\" {}", retaining(slot, param, test));
    format!("{MEMBERS}{}", region(kind, &header, &region(CollectionKind::Map, inner, "- {{i}}\n")))
}

/// One test per selection slot: each guard fact, recorded under one spelling
/// of an original-item field, must prove the nested source under the other.
macro_rules! guard_identity_rows {
    ($($row:ident: $kind:expr, $positional:literal, $param:literal, $test:literal, $inner:literal;)+) => {$(
        mod $row {
            use super::*;

            fn run(slot: &str) {
                let document = guarded_members($kind, $positional, slot, $param, $test, $inner);
                match compose(&document) {
                    Ok(output) => assert_eq!(output, "- a\n- b\n", "{slot}: {document}"),
                    Err(error) => panic!("{slot}: {:?} {error}\n{document}", codes(&error)),
                }
            }

            #[test]
            fn filter_slot() {
                run("filter");
            }

            #[test]
            fn while_slot() {
                run("while");
            }

            #[test]
            fn until_slot() {
                run("until");
            }
        }
    )+};
}

guard_identity_rows! {
    destructured_filter_body_with_an_object_guard:
        CollectionKind::Filter, "{members} => true", "p", "is_array(p.members)", "members";
    object_filter_body_with_a_destructured_guard:
        CollectionKind::Filter, "p => true", "{members}", "is_array(members)", "p.members";
    destructured_map_result_with_an_object_guard:
        CollectionKind::Map, "{members} => {members}", "p", "is_array(p.members)", "members";
    object_filter_body_with_a_differently_named_object_guard:
        CollectionKind::Filter, "p => true", "q", "is_array(q.members)", "p.members";
    destructured_filter_body_with_a_destructured_guard:
        CollectionKind::Filter, "{members} => true", "{members}", "is_array(members)", "members";
    destructured_map_result_with_a_destructured_guard:
        CollectionKind::Map, "{members} => {members}", "{members}", "is_array(members)", "members";
}

#[test]
fn a_map_result_field_named_like_a_guarded_input_field_is_a_new_binding() {
    for slot in ["filter", "while", "until"] {
        let positional = "{members, other} => {members: other, other: members}";
        let renamed = guarded_members(CollectionKind::Map, positional, slot, "p", "is_array(p.members)", "other");
        assert_eq!(composed(&renamed), "- a\n- b\n", "{slot}: the mapping expression carries the proven type");
        let same_name = guarded_members(CollectionKind::Map, positional, slot, "p", "is_array(p.members)", "members");
        assert_eq!(
            codes(&compose(&same_name).unwrap_err()),
            ["dm.collection.untyped_map_source"],
            "{slot}: the result's `members` holds the unproven input `other`"
        );
    }
}

#[test]
fn an_inner_destructured_field_does_not_inherit_the_outer_items_guard_facts() {
    let document = concat!(
        "---\n$schema:\n  data: \"{ members: any(required), groups: { members: any(required) }[](required) }[](required)\"\n",
        "data: [{members: [a, b], groups: [{members: [c]}]}]\n---\n",
        "::filter data \"{members, groups} => true\" filter=\"p => is_array(p.members)\"\n",
        "::filter groups \"{members} => true\"\n",
        "::map members\n- {{i}}\n::end-map\n",
        "::end-filter\n",
        "::end-filter\n",
    );
    assert_eq!(codes(&compose(document).unwrap_err()), ["dm.collection.untyped_map_source"]);
    let outer_read = document.replace("::filter groups \"{members} => true\"", "::filter groups \"{other} => true\"");
    let outer_read = outer_read.replace("groups: { members: any(required) }", "groups: { other: any(required) }");
    let outer_read = outer_read.replace("groups: [{members: [c]}]", "groups: [{other: c}]");
    assert_eq!(composed(&outer_read), "- a\n- b\n", "the outer field stays proven where nothing shadows it");
}

#[test]
fn a_file_field_checked_through_an_object_parameter_is_read_through_a_destructured_one() {
    use darkmatter::markdown::compose::expression::UNCERTAIN_CALL_CODE;
    use darkmatter::markdown::compose::policy::BlockingPolicy;

    let dir = tempfile::TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    std::fs::write(root.join("a.md"), "---\ntitle: Beta\n---\nA\n").unwrap();
    std::fs::write(root.join("b.md"), "---\ntitle: Alpha\n---\nB\n").unwrap();
    for slot in ["filter", "while", "until"] {
        let header = format!(
            "docs \"{{path}} => {{path: frontmatter(path, 'title')}}\" sort=\"{{path}} => frontmatter(path, 'title')\" {}",
            retaining(slot, "p", "p.path != null"),
        );
        let document = format!(
            "---\n$schema:\n  docs: \"{{ path: file }}[](required)\"\ndocs: [{{path: ./a.md}}, {{path: ./b.md}}]\n---\n{}",
            region(CollectionKind::Map, &header, "- {{path}}\n"),
        );
        let output = compose_with_options(&root, &document, |options| {
            options.with_blocking_policy(BlockingPolicy::default().deny_code(UNCERTAIN_CALL_CODE))
        })
        .unwrap_or_else(|error| panic!("{slot}: {error}\n{document}"));
        assert_eq!(output, "- Alpha\n- Beta\n", "{slot}");
    }
}
