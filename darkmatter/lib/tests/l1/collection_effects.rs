//! Evaluation order, laziness, and evaluation counts of `::map` / `::filter`
//! regions, proved with test functions that log their entries.
//!
//! The functions live on the composing thread because callbacks and body
//! expressions are never parallelized. They exist only under the
//! `effects-instrumentation` feature.

use std::cell::RefCell;
use std::sync::Arc;

use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
use darkmatter::markdown::compose::expression::function_test_hooks::{self as hooks, TestFunctionGuard};
use darkmatter::markdown::{MarkdownError, SourceRef};
use serde_json::{Value, json};

use crate::collection_execution::{any_items, codes, collection_error, compose_configured, composed, failed};


thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn log() -> Vec<String> {
    LOG.with(|log| log.borrow().clone())
}

/// `probe(label, value)` logs `label:value` and returns `value`; with a
/// third argument it returns that instead. `boom(label, value)` logs and
/// fails when `value` is `2`.
pub(crate) fn probes() -> (TestFunctionGuard, TestFunctionGuard) {
    LOG.with(|log| log.borrow_mut().clear());
    let probe = FunctionRegistration::builder("probe", "function(parameters(label: string, value: unknown); returns(unknown))")
        .build()
        .expect("signature compiles");
    let boom = FunctionRegistration::builder("boom", "function(parameters(label: string, value: unknown); returns(unknown))")
        .build()
        .expect("signature compiles");
    let probe = hooks::register(
        probe,
        Arc::new(|call| {
            let value = call.args().get(1).cloned().unwrap_or(Value::Null);
            LOG.with(|log| log.borrow_mut().push(format!("{}:{}", call.args().text(0).unwrap_or(""), value)));
            Ok(value)
        }),
    );
    let boom = hooks::register(
        boom,
        Arc::new(|call| {
            let value = call.args().get(1).cloned().unwrap_or(Value::Null);
            LOG.with(|log| log.borrow_mut().push(format!("{}:{}", call.args().text(0).unwrap_or(""), value)));
            if value == json!(2) { Err("boom".to_string()) } else { Ok(value) }
        }),
    );
    (probe, boom)
}

#[test]
fn the_extra_filter_option_runs_only_for_items_the_positional_predicate_keeps() {
    let (_probe, _boom) = probes();
    let document = any_items(
        json!([1, 2, 3, 4]),
        "::filter data \"i => probe('first', i < 3)\" filter=\"i => probe('second', i)\"\n- {{i}}\n::end-filter\n",
    );
    assert_eq!(composed(&document), "- 1\n- 2\n");
    let first_then_second: Vec<_> = log().into_iter().filter(|entry| !entry.starts_with("body")).collect();
    assert_eq!(
        first_then_second,
        ["first:true", "second:1", "first:true", "second:2", "first:false", "first:false"],
        "a falsey positional predicate never reaches the extra filter"
    );
}

// ── sources and absence ──────────────────────────────────────────────────

#[test]
fn a_false_gate_runs_no_source_or_item_callback() {
    let (_probe, _boom) = probes();
    let document = any_items(
        json!([1, 2]),
        "::map data \"i => probe('map', i)\" when=\"probe('gate', false)\" filter=\"i => probe('filter', i)\"\n- {{i}}\n::end-map\n",
    );
    assert_eq!(composed(&document), "");
    assert_eq!(log(), ["gate:false"]);
}

#[test]
fn while_and_until_stop_before_the_boundary_item_and_run_no_later_callback() {
    let (_probe, _boom) = probes();
    let while_ = any_items(json!([1, 2, 3, 4]), "::map data when=\"true\" \"i => i\" while=\"i => probe('w', i < 3)\"\n- {{i}}\n::end-map\n");
    // The positional callback precedes options, so reorder to the grammar.
    let while_ = while_.replace("when=\"true\" \"i => i\" ", "\"i => i\" ");
    assert_eq!(composed(&while_), "- 1\n- 2\n");
    assert_eq!(log(), ["w:true", "w:true", "w:false"], "the stopping item is evaluated, later items are not");

    let (_probe, _boom) = probes();
    let until = any_items(json!([1, 2, 3, 4]), "::map data \"i => i\" until=\"i -> probe('u', i == 3)\"\n- {{i}}\n::end-map\n");
    assert_eq!(composed(&until), "- 1\n- 2\n");
    assert_eq!(log(), ["u:false", "u:false", "u:true"]);
}

#[test]
fn unsorted_selection_prefix_and_map_run_per_item_in_order() {
    let (_probe, _boom) = probes();
    let document = any_items(
        json!([1, 2]),
        "::map data \"i => probe('map', i)\" filter=\"i => probe('filter', true)\" while=\"i => probe('while', true)\"\n- {{i}}\n::end-map\n",
    );
    assert_eq!(composed(&document), "- 1\n- 2\n");
    assert_eq!(log(), ["filter:true", "while:true", "map:1", "filter:true", "while:true", "map:2"]);
}

#[test]
fn every_callback_finishes_before_any_body_expression_runs() {
    let (_probe, _boom) = probes();
    let document = any_items(json!([1, 2]), "::map data \"i => probe('cb', i)\"\n- {{ probe('body', i) }}\n::end-map\n");
    assert_eq!(composed(&document).lines().count(), 2);
    let log = log();
    let first_body = log.iter().position(|entry| entry.starts_with("body")).expect("bodies ran");
    assert_eq!(&log[..first_body], ["cb:1", "cb:2"], "{log:?}");
}

// ── sorting ──────────────────────────────────────────────────────────────

#[test]
fn sort_keys_are_computed_once_from_selected_items_and_precede_the_prefix() {
    let (_probe, _boom) = probes();
    let document = any_items(
        json!([3, 1, 2, 9]),
        "::map data \"i => i\" filter=\"i => i < 9\" sort=\"i => probe('key', i)\" while=\"i => probe('while', i < 3)\"\n- {{i}}\n::end-map\n",
    );
    assert_eq!(composed(&document), "- 1\n- 2\n");
    assert_eq!(log(), ["key:3", "key:1", "key:2", "while:true", "while:true", "while:false"]);
}

#[test]
fn an_empty_selection_never_evaluates_the_sort_selector() {
    let (_probe, _boom) = probes();
    let document = any_items(json!([1, 2]), "::map data filter=\"i => false\" sort=\"i => probe('key', i)\"\n- {{i}}\n::end-map\n");
    assert_eq!(composed(&document), "");
    assert!(log().is_empty());
}

// ── locals ───────────────────────────────────────────────────────────────

#[test]
fn a_collection_inside_a_conditional_block_runs_only_when_the_outer_gate_holds() {
    let (_probe, _boom) = probes();
    let skipped = any_items(
        json!([1]),
        "::block when=\"false\"\n::map data \"i => probe('cb', i)\"\n- {{i}}\n::end-map\n::end-block\n",
    );
    assert_eq!(composed(&skipped), "");
    assert!(log().is_empty());
    let taken = any_items(json!([1]), "::block when=\"true\"\n::map data \"i => i\"\n- {{i}}\n::end-map\n::end-block\n");
    assert_eq!(composed(&taken), "- 1\n");
}

#[test]
fn the_frontmatter_surface_never_executes_collection_callbacks() {
    let (_probe, _boom) = probes();
    let document = any_items(json!([1]), "::map data \"i => probe('cb', i)\"\n- {{i}}\n::end-map\n");
    let output = compose_configured(&document, |options| options.only_frontmatter_surface()).unwrap();
    assert!(output.contains("::map data"), "{output}");
    assert!(log().is_empty());
}

// ── data provenance ──────────────────────────────────────────────────────

#[test]
fn a_callback_failure_is_fatal_and_names_the_original_item() {
    let (_probe, _boom) = probes();
    let document = any_items(json!([1, 2, 3]), "::map data \"i => boom('cb', i)\"\n- {{i}}\n::end-map\n");
    let error = failed(&document);
    assert_eq!(codes(&error), ["dm.collection.callback_failed"], "{error}");
    assert_eq!(collection_error(&error).iteration_path().to_string(), "data[1]");
    assert_eq!(log(), ["cb:1", "cb:2"], "no callback runs after the failure");
}

#[test]
fn the_failing_position_after_sorting_is_the_original_index() {
    let (_probe, _boom) = probes();
    let document = any_items(json!([3, 2, 1]), "::map data \"i => i\" sort=\"i => boom('key', i)\"\n- {{i}}\n::end-map\n");
    let error = failed(&document);
    assert_eq!(collection_error(&error).iteration_path().to_string(), "data[1]");
}

#[test]
fn nested_failures_report_every_enclosing_position() {
    let (_probe, _boom) = probes();
    let document = "---\n$schema:\n  teams: '{ ns: number[](required) }[](required)'\nteams: [{ ns: [1] }, { ns: [1, 2] }]\n---\n::map teams \"t => t.ns\"\n::map t \"n => boom('n', n)\"\n- {{n}}\n::end-map\n::end-map\n";
    let error = failed(document);
    assert_eq!(collection_error(&error).iteration_path().to_string(), "teams[1] → t[1]");
}

#[test]
fn a_failure_inside_a_repeated_body_keeps_its_authored_location() {
    let (_probe, _boom) = probes();
    let document = any_items(json!([1, 2]), "::map data\n- {{ boom('body', i) }}\n::end-map\n");
    let error = failed(&document);
    let located = match &error {
        MarkdownError::Interpolation { source, .. } => match source.as_ref() {
            SourceRef::OnDiskSpan { span, .. } => Some(span.line()),
            _ => None,
        },
        _ => None,
    };
    assert_eq!(located, Some(7), "the copy's expression points at the authored body line: {error:?}");
}

// ── budgets ──────────────────────────────────────────────────────────────


