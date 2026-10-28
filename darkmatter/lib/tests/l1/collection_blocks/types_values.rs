//! Source types and item values: one execution table for both kinds.
//!
//! Each row composes a real document in its own temporary directory. A row
//! names the schema, the data, one opener tail per kind, and the expected
//! outcome per kind, so a map-versus-filter difference is visible in the row
//! itself.

use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::compose::collection_blocks::CollectionKind;
use tempfile::TempDir;

use super::support::{closer, opener, row_tests};
use crate::collection_execution::{codes, compose_with_options};

#[derive(Clone, Copy)]
enum Out {
    /// Composed output is exactly this text.
    Text(&'static str),
    /// Composition fails with exactly this diagnostic code.
    Code(&'static str),
    /// Same outcome as the map column.
    Same,
}
use Out::{Code, Same, Text};

struct Row {
    name: &'static str,
    /// Lines under `$schema:`.
    schema: &'static str,
    /// Top-level frontmatter lines.
    data: &'static str,
    map: &'static str,
    filter: &'static str,
    body: &'static str,
    map_out: Out,
    filter_out: Out,
    files: &'static [(&'static str, &'static str)],
}

const INVALID_SOURCE: &str = "dm.collection.invalid_source";
const PATTERN: &str = "dm.collection.pattern_mismatch";

const ROWS: &[Row] = &[
    Row {
        name: "declared_array",
        schema: "nums: number[](required)",
        data: "nums: [3, 1]",
        map: "nums",
        filter: "nums",
        body: "- {{i}}\n",
        map_out: Text("- 3\n- 1\n"),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "declared_tuple_yields_each_slot",
        schema: "pair: \"tuple([string(required), number(required)])\"",
        data: "pair: [a, 2]",
        map: "pair",
        filter: "pair",
        body: "- {{i}}\n",
        map_out: Text("- a\n- 2\n"),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "declared_dictionary_yields_key_value_entries",
        schema: "d: \"{ a: number(required), b: number(required) }(required)\"",
        data: "d: { a: 1, b: 2 }",
        map: "d",
        filter: "d",
        body: "- {{i.key}}={{i.value}}\n",
        map_out: Text("- a=1\n- b=2\n"),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "heterogeneous_dictionary_values",
        schema: "d: \"{ a: number(required), b: string(required) }(required)\"",
        data: "d: { a: 1, b: x }",
        map: "d \"{key, value} => {key, value}\"",
        filter: "d \"{key, value} => true\"",
        body: "- {{key}}={{value}}\n",
        map_out: Text("- a=1\n- b=x\n"),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "a_null_dictionary_value_is_still_an_entry",
        schema: "d: \"{ a: any }(required)\"",
        data: "d: { a: null }",
        map: "d \"{key, value} => {key, value}\"",
        filter: "d \"{key, value} => true\"",
        body: "- {{key}}=[{{value}}]\n",
        map_out: Text("- a=[]\n"),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "a_union_of_iterables",
        schema: "u:\n  - number[](required)\n  - string[](required)",
        data: "u: [1, 2]",
        map: "u",
        filter: "u",
        body: "- {{i}}\n",
        map_out: Text("- 1\n- 2\n"),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "a_union_with_a_scalar_needs_a_guard",
        schema: "u:\n  - string(required)\n  - string[](required)",
        data: "u: [a]",
        map: "u",
        filter: "u",
        body: "- {{i}}\n",
        map_out: Code(INVALID_SOURCE),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "a_sufficient_guard_on_the_source_gate",
        schema: "u:\n  - string(required)\n  - string[](required)",
        data: "u: [a]",
        map: "u when=\"is_array(u)\"",
        filter: "u when=\"is_array(u)\"",
        body: "- {{i}}\n",
        map_out: Text("- a\n"),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "an_insufficient_guard_leaves_the_scalar_alternative",
        schema: "u:\n  - string(required)\n  - string[](required)",
        data: "u: [a]",
        map: "u when=\"u\"",
        filter: "u when=\"u\"",
        body: "- {{i}}\n",
        map_out: Code(INVALID_SOURCE),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "an_optional_source_that_is_missing",
        schema: "o: string[]",
        data: "",
        map: "o",
        filter: "o",
        body: "- {{i}}\n",
        map_out: Text(""),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "a_null_optional_source",
        schema: "o: string[]",
        data: "o: null",
        map: "o",
        filter: "o",
        body: "- {{i}}\n",
        map_out: Text(""),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "a_union_with_null_is_iterable_without_a_guard",
        schema: "n:\n  - string[](required)\n  - \"null\"",
        data: "n: null",
        map: "n",
        filter: "n",
        body: "- {{i}}\n",
        map_out: Text(""),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "a_present_null_field_binds",
        schema: "data: any[](required)",
        data: "data: [{ a: null }]",
        map: "data \"{a} => {a}\"",
        filter: "data \"{a} => true\"",
        body: "- [{{a}}]\n",
        map_out: Text("- []\n"),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "a_missing_pattern_field_fails",
        schema: "data: any[](required)",
        data: "data: [{ b: 1 }]",
        map: "data \"{a} => {a}\"",
        filter: "data \"{a} => true\"",
        body: "- [{{a}}]\n",
        map_out: Code(PATTERN),
        filter_out: Same,
        files: &[],
    },
    Row {
        name: "a_scalar_result_cannot_satisfy_an_object_pattern",
        schema: "data: any[](required)",
        data: "data: [{ a: 1 }]",
        map: "data \"{a} => a\"",
        filter: "data \"{a} => a\"",
        body: "- {{a}}\n",
        // The explicit difference: a filter has no result binding to check.
        map_out: Code(PATTERN),
        filter_out: Text("- 1\n"),
        files: &[],
    },
    Row {
        name: "a_named_type_projection_in_the_map_body",
        schema: "people: person[](required)@./types.yaml",
        data: "people:\n  - { name: ada, active: true }\n  - { name: bob, active: false }",
        map: "people \"p => p.name\"",
        filter: "people \"p => p.active\"",
        body: "- {{p}}\n",
        map_out: Text("- ada\n- bob\n"),
        // The body of a filter binds the original person, not the boolean.
        filter_out: Text("- {\"name\":\"ada\",\"active\":true}\n"),
        files: &[(
            "types.yaml",
            "kind: schema\ntypes:\n    person:\n        name: string(required)\n        active: boolean(required)\n",
        )],
    },

];

fn render(row: &Row, kind: CollectionKind) -> String {
    let schema: String = row.schema.lines().map(|line| format!("  {line}\n")).collect();
    let tail = if kind == CollectionKind::Map { row.map } else { row.filter };
    format!(
        "---\n$schema:\n{schema}{}{}---\n{} {tail}\n{}{}\n",
        row.data,
        if row.data.is_empty() { "" } else { "\n" },
        opener(kind),
        row.body,
        closer(kind),
    )
}

fn outcome(row: &Row, kind: CollectionKind) -> Out {
    match (kind, row.filter_out) {
        (CollectionKind::Filter, Same) | (CollectionKind::Map, _) => row.map_out,
        (CollectionKind::Filter, explicit) => explicit,
    }
}

fn run_row(name: &str) {
    let row = ROWS.iter().find(|row| row.name == name).unwrap_or_else(|| panic!("no row {name}"));
    for kind in [CollectionKind::Map, CollectionKind::Filter] {
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        for (file, content) in row.files {
            std::fs::write(root.join(file), content).unwrap();
        }
        let document = render(row, kind);
        let result = compose_with_options(&root, &document, |options: ComposeOptions| options);
        match (outcome(row, kind), result) {
            (Text(want), Ok(got)) => assert_eq!(got, want, "{name} {kind:?}\n{document}"),
            (Code(want), Err(error)) => assert_eq!(codes(&error), [want], "{name} {kind:?}: {error}\n{document}"),
            (Text(_), Err(error)) => panic!("{name} {kind:?} failed: {error}\n{document}"),
            (Code(want), Ok(got)) => panic!("{name} {kind:?} composed, wanted {want}:\n{got}\n{document}"),
            (Same, _) => unreachable!(),
        }
    }
}

row_tests! { ROW_TESTS, run_row;
    declared_array, declared_tuple_yields_each_slot, declared_dictionary_yields_key_value_entries,
    heterogeneous_dictionary_values, a_null_dictionary_value_is_still_an_entry, a_union_of_iterables,
    a_union_with_a_scalar_needs_a_guard, a_sufficient_guard_on_the_source_gate,
    an_insufficient_guard_leaves_the_scalar_alternative, an_optional_source_that_is_missing,
    a_null_optional_source, a_union_with_null_is_iterable_without_a_guard, a_present_null_field_binds,
    a_missing_pattern_field_fails, a_scalar_result_cannot_satisfy_an_object_pattern,
    a_named_type_projection_in_the_map_body,
}

#[test]
fn every_generated_type_test_has_a_table_row_and_back() {
    let table: Vec<_> = ROWS.iter().map(|row| row.name).collect();
    assert_eq!(ROW_TESTS, table.as_slice());
}

// ── recovery for unknown versus known scalar sources ─────────────────────

const UNKNOWN_SOURCE: &str = "---\nitems: [1, 2]\n---\n";
const KNOWN_SCALAR: &str = "---\n$schema:\n  items: string(required)\nitems: text\n---\n";

fn recovery(on: CollectionKind) -> impl FnOnce(ComposeOptions) -> ComposeOptions {
    move |options| match on {
        CollectionKind::Map => options.with_allow_untyped_map_variables(true),
        CollectionKind::Filter => options.with_allow_untyped_filter_variables(true),
    }
}

fn untyped_code(kind: CollectionKind) -> &'static str {
    match kind {
        CollectionKind::Map => "dm.collection.untyped_map_source",
        CollectionKind::Filter => "dm.collection.untyped_filter_source",
    }
}

#[test]
fn an_unknown_source_is_waived_only_by_its_own_kinds_recovery() {
    for kind in [CollectionKind::Map, CollectionKind::Filter] {
        let document = format!("{UNKNOWN_SOURCE}{} items\n- {{{{i}}}}\n{}\n", opener(kind), closer(kind));
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let error = compose_with_options(&root, &document, |options| options).unwrap_err();
        assert_eq!(codes(&error), [untyped_code(kind)], "{kind:?}");

        let wrong = super::support::other(kind);
        let error = compose_with_options(&root, &document, recovery(wrong)).unwrap_err();
        assert_eq!(codes(&error), [untyped_code(kind)], "{kind:?}: the other kind's recovery does not apply");

        let output = compose_with_options(&root, &document, recovery(kind)).unwrap();
        assert_eq!(output, "- 1\n- 2\n", "{kind:?}");
    }
}

#[test]
fn a_known_scalar_is_never_waived_by_either_recovery() {
    for kind in [CollectionKind::Map, CollectionKind::Filter] {
        let document = format!("{KNOWN_SCALAR}{} items\n- {{{{i}}}}\n{}\n", opener(kind), closer(kind));
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let error = compose_with_options(&root, &document, recovery(kind)).unwrap_err();
        assert_eq!(codes(&error), [INVALID_SOURCE], "{kind:?}");
    }
}

#[test]
fn a_recovered_source_discovers_shapes_at_runtime_for_both_kinds() {
    let object = "---\nitems:\n  a: 1\n  b: 2\n---\n";
    for kind in [CollectionKind::Map, CollectionKind::Filter] {
        let document = format!("{object}{} items\n- {{{{i.key}}}}={{{{i.value}}}}\n{}\n", opener(kind), closer(kind));
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let output = compose_with_options(&root, &document, recovery(kind)).unwrap();
        assert_eq!(output, "- a=1\n- b=2\n", "{kind:?}");
    }
}
