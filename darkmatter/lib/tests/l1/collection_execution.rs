//! Execution of `::map` / `::filter` regions through `Markdown::compose_with`.
//!
//! Every test composes a real document in its own temporary directory through
//! the public compose API. Test functions registered through the expression
//! test hooks log their entries, which proves evaluation order, laziness, and
//! the number of evaluations; they live on the composing thread because
//! callbacks and body expressions are never parallelized.

use std::num::NonZeroU64;
use std::path::Path;

use darkmatter::markdown::compose::collection_blocks::CollectionError;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeContext, ComposeOperation, ComposeOptions};
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::{Value, json};
use tempfile::TempDir;

pub(crate) fn compose_with_options(
    dir: &Path,
    content: &str,
    configure: impl FnOnce(ComposeOptions) -> ComposeOptions,
) -> Result<String, MarkdownError> {
    let path = dir.join("doc.md");
    std::fs::write(&path, content).unwrap();
    let options = configure(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.clone()),
    );
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request_at(dir, options))
        .map(|(composed, _)| composed.content().to_string())
}

pub(crate) fn compose(content: &str) -> Result<String, MarkdownError> {
    compose_configured(content, |options| options)
}

pub(crate) fn compose_configured(
    content: &str,
    configure: impl FnOnce(ComposeOptions) -> ComposeOptions,
) -> Result<String, MarkdownError> {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    compose_with_options(&root, content, configure)
}

#[track_caller]
pub(crate) fn composed(content: &str) -> String {
    compose(content).unwrap_or_else(|error| panic!("{content:?} failed: {error}"))
}

#[track_caller]
pub(crate) fn failed(content: &str) -> MarkdownError {
    match compose(content) {
        Ok(output) => panic!("{content:?} composed:\n{output}"),
        Err(error) => error,
    }
}

pub(crate) fn codes(error: &MarkdownError) -> Vec<String> {
    error_diagnostics(error).into_iter().map(|diagnostic| diagnostic.code).collect()
}

pub(crate) fn collection_error(error: &MarkdownError) -> &CollectionError {
    match error {
        MarkdownError::Collection(inner) => inner,
        other => panic!("expected a collection error, got {other:?}"),
    }
}

/// A document with `data: any[]` holding `items`, composed with `body`.
pub(crate) fn any_items(items: Value, body: &str) -> String {
    format!("---\n$schema:\n  data: any[](required)\ndata: {items}\n---\n{body}")
}

pub(crate) const PEOPLE: &str = "---\n$schema:\n  people: '{ name: string(required), age: number(required), active: boolean(required) }[](required)'\npeople:\n  - { name: ada, age: 30, active: true }\n  - { name: bob, age: 25, active: false }\n  - { name: cy, age: 30, active: true }\n---\n";

#[test]
fn map_without_a_callback_binds_each_item_to_i() {
    let document = "---\n$schema:\n  data: string[](required)\ndata: [foo, bar, baz]\n---\n::map data\n- {{i}}\n::end-map\n";
    assert_eq!(composed(document), "- foo\n- bar\n- baz\n");
}

#[test]
fn map_binds_the_transformed_value_through_the_same_parameter() {
    let document = format!("{PEOPLE}::map people \"p => p.name\"\n- {{{{p}}}}\n::end-map\n");
    assert_eq!(composed(&document), "- ada\n- bob\n- cy\n");
    let upper = format!("{PEOPLE}::map people \"p -> upper(p.name)\"\n- {{{{p}}}}\n::end-map\n");
    assert_eq!(composed(&upper), "- ADA\n- BOB\n- CY\n", "both arrows are the same");
    let identity = format!("{PEOPLE}::map people \"p => p\"\n- {{{{p.name}}}} {{{{p.age}}}}\n::end-map\n");
    assert_eq!(composed(&identity), "- ada 30\n- bob 25\n- cy 30\n");
}

#[test]
fn map_destructures_object_entries_and_exposes_the_transformed_fields() {
    let document = "---\n$schema:\n  data: '{ foo: string(required), bar: string(required) }(required)'\ndata: { foo: bar, bar: baz }\n---\n::map data \"{key,value} => {key, value: upper(value)}\"\n- {{key}} likes {{value}}\n::end-map\n";
    assert_eq!(composed(document), "- foo likes BAR\n- bar likes BAZ\n");
}

#[test]
fn map_fails_when_the_result_cannot_satisfy_the_input_pattern() {
    let document = any_items(json!([{"a": 1}]), "::map data \"{a} => a\"\n- {{a}}\n::end-map\n");
    let error = failed(&document);
    assert_eq!(codes(&error), ["dm.collection.pattern_mismatch"], "{error}");
    assert_eq!(collection_error(&error).iteration_path().to_string(), "data[0]");
}

#[test]
fn a_pattern_field_that_is_missing_is_not_a_null_field() {
    let present = any_items(json!([{"a": null}]), "::filter data \"{a} => true\"\n- [{{a}}]\n::end-filter\n");
    assert_eq!(composed(&present), "- []\n", "a present null field binds");
    let missing = any_items(json!([{"b": 1}]), "::filter data \"{a} => true\"\n- [{{a}}]\n::end-filter\n");
    assert_eq!(codes(&failed(&missing)), ["dm.collection.pattern_mismatch"]);
}

// ── filter semantics ─────────────────────────────────────────────────────

#[test]
fn filter_retains_the_original_item_not_the_predicate_result() {
    let document = format!("{PEOPLE}::filter people \"p -> p.active\"\n- {{{{p.name}}}} ({{{{p.age}}}})\n::end-filter\n");
    assert_eq!(composed(&document), "- ada (30)\n- cy (30)\n");
}

#[test]
fn an_omitted_filter_predicate_retains_falsey_items_but_identity_removes_them() {
    let items = json!([0, "", false, null, 7]);
    let omitted = any_items(items.clone(), "::filter data\n- <{{i}}>\n::end-filter\n");
    assert_eq!(composed(&omitted), "- <0>\n- <>\n- <false>\n- <>\n- <7>\n");
    let identity = any_items(items, "::filter data \"i => i\"\n- <{{i}}>\n::end-filter\n");
    assert_eq!(composed(&identity), "- <7>\n");
}

#[test]
fn missing_null_and_empty_sources_render_nothing() {
    let missing = "---\n$schema:\n  data: string[]\n---\nbefore\n\n::map data\n- {{i}}\n::end-map\n\nafter\n";
    assert_eq!(composed(missing).trim_end(), "before\n\n\nafter");
    let empty = any_items(json!([]), "before\n\n::map data\n- {{i}}\n::end-map\n\nafter\n");
    assert_eq!(composed(&empty).trim_end(), "before\n\n\nafter");
    let null = "---\n$schema:\n  data: string[]\ndata: null\n---\n::filter data\n- {{i}}\n::end-filter\n";
    assert_eq!(composed(null), "");
}

#[test]
fn a_null_element_is_an_item_while_a_null_collection_is_not() {
    let document = any_items(json!([null, 1]), "::map data\n- <{{i}}>\n::end-map\n");
    assert_eq!(composed(&document), "- <>\n- <1>\n");
}

#[test]
fn a_reached_scalar_source_fails_even_under_the_untyped_recovery() {
    let document = "---\nnote: text\n---\n::map note\n- {{i}}\n::end-map\n";
    let error = compose_configured(document, |options| options.with_allow_untyped_map_variables(true)).unwrap_err();
    assert_eq!(codes(&error), ["dm.collection.invalid_source"], "{error}");
}

#[test]
fn the_untyped_recovery_discovers_object_entries_at_runtime() {
    let document = "---\nmap_like:\n  a: 1\n  b: 2\n---\n::map map_like\n- {{i.key}}={{i.value}}\n::end-map\n";
    let output = compose_configured(document, |options| options.with_allow_untyped_map_variables(true)).unwrap();
    assert_eq!(output, "- a=1\n- b=2\n");
}

#[test]
fn objects_iterate_in_authored_key_order() {
    let document = "---\n$schema:\n  data: '{ zeta: string(required), alpha: string(required), mid: string(required) }(required)'\ndata: { zeta: z, alpha: a, mid: m }\n---\n::map data\n- {{i.key}}\n::end-map\n";
    assert_eq!(composed(document), "- zeta\n- alpha\n- mid\n");
}

// ── gates, laziness, and prefix selection ────────────────────────────────

#[test]
fn an_item_rejected_by_selection_cannot_stop_the_sequence() {
    let document = any_items(
        json!([1, 2, 3, 4]),
        "::map data \"i => i\" filter=\"i => i != 3\" until=\"i => i == 3\"\n- {{i}}\n::end-map\n",
    );
    assert_eq!(composed(&document), "- 1\n- 2\n- 4\n", "3 never reaches the stopping predicate");
}

#[test]
fn sorting_is_stable_in_both_directions_over_original_items() {
    let ascending = format!("{PEOPLE}::filter people sort=\"p => p.age\"\n- {{{{i.name}}}}\n::end-filter\n");
    assert_eq!(composed(&ascending), "- bob\n- ada\n- cy\n", "equal keys keep their previous order");
    let descending = format!("{PEOPLE}::filter people sort=\"p => p.age\" sort-order=\"desc\"\n- {{{{i.name}}}}\n::end-filter\n");
    assert_eq!(composed(&descending), "- ada\n- cy\n- bob\n", "descending is stable, not a reversed ascending");
}

#[test]
fn strings_sort_by_code_point_and_numbers_numerically() {
    let strings = any_items(json!(["b", "B", "a"]), "::map data sort=\"s => s\"\n- {{i}}\n::end-map\n");
    assert_eq!(composed(&strings), "- B\n- a\n- b\n");
    let numbers = any_items(json!([10, 9, 2.5]), "::map data sort=\"n => n\"\n- {{i}}\n::end-map\n");
    assert_eq!(composed(&numbers), "- 2.5\n- 9\n- 10\n");
}

#[test]
fn invalid_sort_keys_fail_including_a_single_selected_item() {
    for (name, items) in [
        ("null", json!([null])),
        ("boolean", json!([true])),
        ("object", json!([{"a": 1}])),
        ("array", json!([[1]])),
        ("mixed", json!([1, "a"])),
    ] {
        let document = any_items(items, "::map data sort=\"v => v\"\n- x\n::end-map\n");
        let error = failed(&document);
        assert_eq!(codes(&error), ["dm.collection.invalid_sort_key"], "{name}: {error}");
    }
}

#[test]
fn nested_regions_see_enclosing_bindings_and_shadow_them() {
    let document = "---\n$schema:\n  teams: '{ name: string(required), members: string[](required) }[](required)'\nteams:\n  - { name: red, members: [ann, ben] }\n  - { name: blue, members: [cat] }\n---\n::map teams \"team => team.name\"\n## {{team}}\n::end-map\n\n::filter teams \"team => true\"\n::map team.members \"m => upper(m)\"\n- {{team.name}}: {{m}}\n::end-map\n::end-filter\n";
    assert_eq!(composed(document), "## red\n## blue\n\n- red: ANN\n- red: BEN\n- blue: CAT\n");
}

#[test]
fn an_inner_local_shadows_an_outer_one_and_the_outer_meaning_returns() {
    let document = "---\n$schema:\n  outer: string[](required)\n  inner: string[](required)\nouter: [a, b]\ninner: [x]\n---\n::map outer \"v => v\"\n::map inner \"v => v\"\n- inner {{v}}\n::end-map\n- outer {{v}}\n::end-map\n";
    assert_eq!(composed(document), "- inner x\n- outer a\n- inner x\n- outer b\n");
}

#[test]
fn explicit_local_and_frontmatter_namespaces_are_distinct_from_the_implicit_read() {
    let document = "---\n$schema:\n  title: string(required)\n  data: string[](required)\ntitle: doc\ndata: [a]\n---\n::map data \"title => title\"\n- {{title}} {{local.title}} {{fm.title}}\n::end-map\n";
    assert_eq!(composed(document), "- a a doc\n");
}

#[test]
fn a_local_does_not_leak_to_siblings_or_after_the_region() {
    let document = "---\n$schema:\n  data: string[](required)\ndata: [a]\n---\n::map data\n- {{i}}\n::end-map\n\nafter:{{ local.i }}:\n";
    assert_eq!(composed(document).trim_end(), "- a\n\nafter::");
}

// ── composition with page blocks ─────────────────────────────────────────

#[test]
fn a_conditional_block_inside_a_body_sees_the_iteration_it_is_repeated_for() {
    let document = format!(
        "{PEOPLE}::map people \"p => p\"\n::block when=\"p.active\"\n- {{{{p.name}}}} is active\n::end-block\n::end-map\n"
    );
    assert_eq!(composed(&document), "- ada is active\n- cy is active\n");
}

#[test]
fn page_blocks_outside_collections_behave_as_before() {
    let document = any_items(
        json!([1]),
        "::block when=\"false\"\nhidden\n::end-block\n\n::map data\n- {{i}}\n::end-map\n\n::block when=\"true\"\nshown\n::end-block\n",
    );
    assert_eq!(composed(&document).trim_start(), "- 1\n\nshown\n");
}

// ── operations ───────────────────────────────────────────────────────────

#[test]
fn a_disabled_collection_operation_leaves_the_wrappers_as_text() {
    let document = any_items(json!([1]), "::map data\n- {{i}}\n::end-map\n");
    let output =
        compose_configured(&document, |options| options.disable(ComposeOperation::CollectionBlocks)).unwrap();
    assert!(output.contains("::map data"), "{output}");
}

#[test]
fn inserted_values_are_data_and_authored_expressions_still_run() {
    let document = any_items(
        json!(["::shell echo INJECTED", "::end-map", "```", "::file ./missing.md"]),
        "::map data\n- {{i}} {{ 1 + 1 }}\n::end-map\n",
    );
    assert_eq!(
        composed(&document),
        "- ::shell echo INJECTED 2\n- ::end-map 2\n- ``` 2\n- ::file ./missing.md 2\n"
    );
}

// ── markdown boundaries ──────────────────────────────────────────────────

#[test]
fn repeated_bodies_keep_exact_whitespace_indentation_and_blank_lines() {
    let document = any_items(json!(["a", "b"]), "::map data\n  - {{i}}\n\n  - again\n::end-map\n");
    assert_eq!(composed(&document), "  - a\n\n  - again\n  - b\n\n  - again\n");
}

#[test]
fn an_empty_body_or_source_removes_the_region_without_a_placeholder() {
    let empty_body = any_items(json!(["a"]), "before\n\n::map data\n::end-map\n\nafter\n");
    assert_eq!(composed(&empty_body).trim_end(), "before\n\n\nafter", "the region's own lines go, the surrounding blank lines stay");
}

#[test]
fn block_quoted_regions_repeat_with_their_quote_markers() {
    let document = any_items(json!(["a", "b"]), "> ::map data\n> - {{i}}\n> ::end-map\n");
    assert_eq!(composed(&document), "> - a\n> - b\n");
}

#[test]
fn directive_looking_text_in_code_is_not_a_region() {
    let document = any_items(json!(["a"]), "```md\n::map data\nx\n::end-map\n```\n\n::map data\n- {{i}}\n::end-map\n");
    let output = composed(&document);
    assert!(output.contains("::map data\nx\n::end-map"), "{output}");
    assert!(output.contains("- a"), "{output}");
}

#[test]
fn crlf_and_a_closer_without_a_final_newline_compose() {
    let document = any_items(json!(["a", "b"]), "::map data\r\n- {{i}}\r\n::end-map");
    let output = composed(&document);
    assert_eq!(output.replace("\r\n", "\n").trim_end(), "- a\n- b");
}

// ── failure policy ───────────────────────────────────────────────────────

fn limit(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).unwrap()
}

#[test]
fn defaults_are_the_documented_allowances() {
    let options = ComposeOptions::new();
    assert_eq!(options.iteration_limits().max_visits().get(), 100_000);
    assert_eq!(options.iteration_limits().max_expansion_bytes().get(), 16_777_216);
    let tuned = options.with_max_iteration_visits(limit(7)).with_max_expansion_bytes(limit(9));
    assert_eq!((tuned.iteration_limits().max_visits().get(), tuned.iteration_limits().max_expansion_bytes().get()), (7, 9));
}

#[test]
fn an_exactly_full_visit_allowance_succeeds_and_one_more_item_fails() {
    let four = any_items(json!([1, 2, 3, 4]), "::map data\n- {{i}}\n::end-map\n");
    assert!(compose_configured(&four, |options| options.with_max_iteration_visits(limit(4))).is_ok());
    let error = compose_configured(&four, |options| options.with_max_iteration_visits(limit(3))).unwrap_err();
    assert_eq!(codes(&error), ["dm.iteration.visit_limit"], "{error}");
}

#[test]
fn rejected_items_and_the_stopping_item_count_as_visits_and_empty_bodies_consume_them() {
    let filtered = any_items(json!([1, 2, 3, 4, 5]), "::filter data \"i => i == 1\"\n- {{i}}\n::end-filter\n");
    assert!(compose_configured(&filtered, |options| options.with_max_iteration_visits(limit(5))).is_ok());
    assert!(compose_configured(&filtered, |options| options.with_max_iteration_visits(limit(4))).is_err());

    let stopped = any_items(json!([1, 2, 3, 4]), "::map data \"i => i\" until=\"i => i == 3\"\n- {{i}}\n::end-map\n");
    assert!(compose_configured(&stopped, |options| options.with_max_iteration_visits(limit(3))).is_ok());
    assert!(compose_configured(&stopped, |options| options.with_max_iteration_visits(limit(2))).is_err());

    let sorted = any_items(json!([3, 2, 1]), "::map data \"i => i\" sort=\"i => i\" while=\"i => i < 2\"\n- {{i}}\n::end-map\n");
    assert!(compose_configured(&sorted, |options| options.with_max_iteration_visits(limit(3))).is_ok());
    assert!(compose_configured(&sorted, |options| options.with_max_iteration_visits(limit(2))).is_err(), "sorting visits every item");

    let empty_body = any_items(json!([1, 2]), "::map data\n::end-map\n");
    assert!(compose_configured(&empty_body, |options| options.with_max_iteration_visits(limit(2))).is_ok());
    assert!(compose_configured(&empty_body, |options| options.with_max_iteration_visits(limit(1))).is_err());
}

#[test]
fn nested_regions_share_one_visit_allowance() {
    let document = "---\n$schema:\n  outer: string[](required)\n  inner: string[](required)\nouter: [a, b]\ninner: [x, y]\n---\n::map outer\n::map inner\n- {{i}}\n::end-map\n::end-map\n";
    // 2 outer visits + 2 × 2 inner visits.
    assert!(compose_configured(document, |options| options.with_max_iteration_visits(limit(6))).is_ok());
    assert!(compose_configured(document, |options| options.with_max_iteration_visits(limit(5))).is_err());
}

#[test]
fn authored_body_bytes_and_inserted_values_are_charged_before_the_append() {
    // Each copy is `- {{i}}\n` (8 bytes) and its value is one byte.
    let document = any_items(json!(["a", "b"]), "::map data\n- {{i}}\n::end-map\n");
    assert!(compose_configured(&document, |options| options.with_max_expansion_bytes(limit(18))).is_ok());
    let error = compose_configured(&document, |options| options.with_max_expansion_bytes(limit(17))).unwrap_err();
    assert_eq!(codes(&error), ["dm.iteration.expansion_limit"], "{error}");
}

#[test]
fn nested_output_is_charged_again_where_it_lands_in_the_enclosing_expansion() {
    let document = "---\n$schema:\n  outer: string[](required)\n  inner: string[](required)\nouter: [a]\ninner: [x, y]\n---\n::map outer\nA\n::map inner\nb\n::end-map\n::end-map\n";
    // `A\n` once (2) plus two `b\n` copies at two levels (2 × 2 × 2).
    assert!(compose_configured(document, |options| options.with_max_expansion_bytes(limit(10))).is_ok());
    assert!(compose_configured(document, |options| options.with_max_expansion_bytes(limit(9))).is_err());
}

#[test]
fn text_outside_every_region_and_stage_passes_are_not_charged() {
    let document = any_items(json!(["a"]), &format!("{}\n::map data\nx\n::end-map\n", "unrelated text ".repeat(50)));
    assert!(compose_configured(&document, |options| options.with_max_expansion_bytes(limit(2))).is_ok());
}

#[test]
fn budget_exhaustion_cannot_be_absorbed_by_a_nested_composition_fallback() {
    let nested = "---\\n$schema:\\n  d: 'string[](required)'\\nd: [a, b, c]\\n---\\n::map d\\n- {{i}}\\n::end-map\\n";
    let document = any_items(json!([1]), &format!("{{{{ as_markdown(\"{nested}\") || 'fallback' }}}}\n"));
    let error = compose_configured(&document, |options| options.with_max_iteration_visits(limit(2))).unwrap_err();
    assert_eq!(codes(&error), ["dm.iteration.visit_limit"], "{error}");
    let with_room = compose_configured(&document, |options| options.with_max_iteration_visits(limit(10))).unwrap();
    assert!(with_room.contains("- a"), "{with_room}");
}

#[test]
fn nested_composition_shares_the_enclosing_allowance() {
    let nested = "---\\n$schema:\\n  d: 'string[](required)'\\nd: [a, b]\\n---\\n::map d\\n- {{i}}\\n::end-map\\n";
    let document = any_items(json!([1, 2]), &format!("::map data\n{{{{ as_markdown(\"{nested}\") }}}}\n::end-map\n"));
    // 2 outer visits + 2 × 2 nested visits.
    assert!(compose_configured(&document, |options| options.with_max_iteration_visits(limit(6))).is_ok());
    assert!(compose_configured(&document, |options| options.with_max_iteration_visits(limit(5))).is_err());
}

// ── transclusion and shell ───────────────────────────────────────────────

#[test]
fn a_target_expression_reads_the_local_of_its_copy() {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    std::fs::write(root.join("a.md"), "child A\n").unwrap();
    std::fs::write(root.join("b.md"), "child B\n").unwrap();
    let document = "---\n$schema:\n  names: string[](required)\nnames: [a.md, b.md]\n---\n::map names\n::file {{i}}\n::end-map\n";
    let output = compose_with_options(&root, document, |options| options).unwrap();
    assert_eq!(output.matches("child A").count(), 1);
    assert_eq!(output.matches("child B").count(), 1);
    assert!(output.find("child A") < output.find("child B"), "{output}");
}

#[test]
fn a_transclusion_condition_reads_the_local_of_its_copy() {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    std::fs::write(root.join("child.md"), "child\n").unwrap();
    let document = "---\n$schema:\n  flags: boolean[](required)\nflags: [true, false, true]\n---\n::map flags\n::file ./child.md when=\"i\"\n::end-map\n";
    let output = compose_with_options(&root, document, |options| options).unwrap();
    assert_eq!(output.matches("child").count(), 2, "{output}");
}

#[test]
fn a_collection_failure_in_a_child_stays_fatal_through_lenient_transclusion() {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    std::fs::write(root.join("child.md"), "---\nnote: text\n---\n::map note\n- {{i}}\n::end-map\n").unwrap();
    let document = "before\n\n::file ./child.md\n";
    let error = compose_with_options(&root, document, |options| {
        options.with_ignore_invalid_references(Some(true)).with_allow_untyped_map_variables(true)
    })
    .unwrap_err();
    assert_eq!(codes(&error), ["dm.collection.invalid_source"], "{error}");
}

// ── shell stages ─────────────────────────────────────────────────────────

mod shell {
    use std::sync::{Arc, Mutex};

    use darkmatter::markdown::compose::shell_expansion::types::{
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
    };
    use darkmatter::markdown::compose::ShellExpansionError;

    use super::*;

    /// Records each approval request and allows it.
    struct Recorder(Mutex<Vec<String>>);

    impl ShellApprovalHandler for Recorder {
        fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
            let recorded = match (request.offered.exact, request.template) {
                (false, Some(template)) => format!("template {}", template.display),
                _ => request.normalized_exact,
            };
            self.0.lock().unwrap().push(recorded);
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    fn run(content: &str) -> (Result<String, MarkdownError>, Vec<String>) {
        let recorder = Arc::new(Recorder(Mutex::default()));
        let dir = TempDir::new().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let handler = recorder.clone();
        let output = compose_with_options(&root, content, |options| {
            options.with_shell_policy_root(&root).with_shell_working_directory(&root).with_shell_approval_handler(handler)
        });
        let commands = recorder.0.lock().unwrap().clone();
        (output, commands)
    }

    #[test]
    fn a_shell_directive_in_a_body_runs_once_per_item_with_the_local_of_its_copy() {
        let document = any_items(json!(["one", "two"]), "::map data\n::shell echo {{ i }}\n::end-map\n");
        let (output, commands) = run(&document);
        let output = output.unwrap_or_else(|error| panic!("{error}"));
        assert!(output.contains("one") && output.contains("two"), "{output}");
        assert!(output.find("one") < output.find("two"), "{output}");
        assert!(!commands.is_empty(), "the shell stage asked for approval");
    }

    #[test]
    fn a_shell_block_in_a_body_repeats_as_a_template_and_closes_on_its_own_closer() {
        let document = any_items(json!(["one", "two"]), "::map data\n::shell-block\necho first {{ i }}\n::end-block\nafter {{ i }}\n::end-map\n");
        let (output, _) = run(&document);
        let output = output.unwrap_or_else(|error| panic!("{error}"));
        assert!(output.contains("first one") && output.contains("first two"), "{output}");
        assert!(output.contains("after one") && output.contains("after two"), "{output}");
    }
}

#[test]
fn text_an_earlier_stage_inserted_stays_data_in_every_copy() {
    // The interpolated replacement value is data; the authored one is not.
    let document = "---\n$schema:\n  data: string[](required)\ndata: [a, b]\ncmd: echo DATA\nreplace:\n  PRODUCED: \"{{ cmd }}\"\n---\n::map data\n- {{i}} PRODUCED\n::end-map\n";
    assert_eq!(composed(document), "- a echo DATA\n- b echo DATA\n");
}

#[test]
fn callback_runtime_composition_shares_expression_and_directive_visits() {
    let document = "---\n$schema:\n  data: number[](required)\ndata: [1, 2]\n---\n::map data\n{{ map([i], n => n) }}\n::end-map\n{{ filter(data, n -> false) }}\n";
    assert_eq!(compose_configured(document, |o| o.with_max_iteration_visits(limit(6))).unwrap(), "[1] [2] []");
    let error = compose_configured(document, |o| o.with_max_iteration_visits(limit(5))).unwrap_err();
    assert_eq!(codes(&error), ["dm.iteration.visit_limit"], "{error}");
    let diagnostics = error_diagnostics(&error);
    assert!(diagnostics[0].source.is_some());
}

#[test]
fn callback_runtime_real_file_reads_use_the_enclosing_document_context() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("person.md"), "---\nname: Ada\n---\n").unwrap();
    let document = "{{ map(find_files('./person.md'), file => frontmatter(file).name) }}\n";
    let output = compose_with_options(dir.path(), document, |o| o).unwrap();
    assert_eq!(output, "[\"Ada\"]\n");
    let error = compose_with_options(dir.path(), "{{ map(['./missing.md'], file => frontmatter(file)) }}", |o| o).unwrap_err();
    let MarkdownError::Interpolation { cause, .. } = &error else { panic!("{error:?}") };
    assert!(matches!(cause.as_ref(), darkmatter::markdown::compose::expression::ExpressionError::Callback { .. }));
    let darkmatter::markdown::compose::expression::ExpressionError::FileReference(diagnostic) = cause.without_callbacks() else { panic!("{cause:?}") };
    assert_eq!(diagnostic.reference, "./missing.md");
    assert!(cause.is_authoring_fatal());
    assert!(error_diagnostics(&error)[0].source.is_some());
}

#[test]
fn callback_runtime_composition_preserves_free_capture_absence_observations() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("doc.md");
    let text = "{{ map([true], p => p ? missing : null) }}";
    std::fs::write(&path, text).unwrap();
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), "")).with_source_file(path.clone());
    let (_, report) = Markdown::try_from(path.as_path()).unwrap().compose_with(&crate::request_support::request_at(dir.path(), options)).unwrap();
    assert!(report.warnings.iter().any(|warning| warning.message.contains("missing")), "{:?}", report.warnings);
}

#[test]
fn callback_runtime_child_exhaustion_cannot_be_hidden_by_a_fallback() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("child.md"), "{{ map([1, 2], n => n) }}").unwrap();
    let child = compose_with_options(dir.path(), "::file ./child.md\n", |o| o.with_max_iteration_visits(limit(1))).unwrap_err();
    assert_eq!(codes(&child), ["dm.iteration.visit_limit"]);
    let nested = r#"{{ as_markdown("{{ map([1, 2], n => n) }}") || 'hidden' }}"#;
    let error = compose_with_options(dir.path(), nested, |o| o.with_max_iteration_visits(limit(1))).unwrap_err();
    assert_eq!(codes(&error), ["dm.iteration.visit_limit"], "{error}");
    assert!(error.iteration_failure().is_some());
}

#[test]
fn callback_runtime_nested_failure_reports_every_original_position() {
    let error = failed("{{ map({team: [1]}, {value} => filter(value, n => frontmatter('./missing.md'))) }}");
    let diagnostic = &error_diagnostics(&error)[0];
    assert_eq!(diagnostic.iteration_path.to_string(), "map[team] → filter[0]");
    assert!(diagnostic.source.is_some());
    let MarkdownError::Interpolation { cause, .. } = error else { panic!("{error:?}") };
    assert!(matches!(cause.without_callbacks(), darkmatter::markdown::compose::expression::ExpressionError::FileReference(_)));
}

#[test]
fn callback_runtime_schema_tuples_and_yaml_representations_keep_their_values() {
    for values in ["[Ada, 2]", "[Ada, '2']"] {
        let text = format!("---\n$schema:\n  pair: tuple([string, number])\npair: {values}\n---\n{{{{ map(pair, item => item) }}}}");
        assert_eq!(composed(&text), "[\"Ada\",2]", "{values}");
    }
}
