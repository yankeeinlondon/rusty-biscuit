//! Declared conversions through recursive references, in both translators.
//!
//! Before reusable references, the JSON translator expanded a recursive
//! target [`OLD_CYCLE_EXPANSIONS`] times per path and left deeper values
//! validation-only, and the grammar translator failed a recursive named type
//! with `TranslateError::TooDeep`. Every fixture here nests its interesting
//! value below that cutoff, so each test fails if a translator stops
//! following the reference.

use std::path::Path;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::coercion::{FailureReason, MAX_DEPTH, PathSegment, ValueFailure, bind};
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::compose::expression::call_binding::{
    FunctionRegistration, NoNamedTypes, TranslateError, shape_from_property_def, shape_from_simplified_schema_with_definitions,
};
use darkmatter::markdown::schemas::coerce::coerce_frontmatter;
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, resolve_yaml_schema};
use darkmatter::markdown::schemas::shape_from_json_schema;
use darkmatter::markdown::schemas::simplified::PropertyDef;
use darkmatter::markdown::schemas::simplified::grammar::parse_type_definition;
use darkmatter::markdown::schemas::{DarkmatterSchemas, ValidationReport};
use serde_json::{Value, json};
use tempfile::TempDir;

use crate::request_support::request;

/// The recursion levels the JSON translator used to expand.
const OLD_CYCLE_EXPANSIONS: usize = 4;

/// Levels below the old cutoff, still well inside the engine's depth bound.
const DEEP: usize = 9;
const _: () = assert!(DEEP > OLD_CYCLE_EXPANSIONS + 1);

const ITEM: &str = "\"{ value: string(required), children: item[](required) }\"";

fn tree_schema() -> String {
    format!("item: {ITEM}\ntree: item(required)\n")
}

fn resolved(dir: &Path, schema: &str) -> ResolvedSchema {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(schema).expect("the schema is YAML");
    resolve_yaml_schema(&yaml, dir, &FileResolutionContext::new(dir)).expect("the schema resolves")
}

/// A `{ value, children }` chain `depth` levels deep: `filler(level)` at each
/// level above the bottom, `leaf` at the bottom.
fn chain(depth: usize, filler: impl Fn(usize) -> Value, leaf: Value) -> Value {
    (1..depth).fold(json!({"value": leaf, "children": []}), |child, level| {
        json!({"value": filler(level), "children": [child]})
    })
}

fn tree(depth: usize, leaf: Value) -> Value {
    chain(depth, |level| json!(format!("n{level}")), leaf)
}

/// The path of the bottom `value` of `chain(depth, ..)` under `root`.
fn leaf_path(root: &str, depth: usize) -> Vec<PathSegment> {
    let mut path = vec![PathSegment::Key(root.to_string())];
    for _ in 1..depth {
        path.push(PathSegment::Key("children".into()));
        path.push(PathSegment::Index(0));
    }
    path.push(PathSegment::Key("value".into()));
    path
}

fn leaf_pointer(root: &str, depth: usize) -> String {
    format!("/{root}{}/value", "/children/0".repeat(depth - 1))
}

/// Every failure, including those inside union options.
fn all_failures(failures: &[ValueFailure]) -> Vec<&ValueFailure> {
    let mut out = Vec::new();
    for failure in failures {
        out.push(failure);
        if let FailureReason::NoOptionAccepts { options } = &failure.reason {
            for option in options {
                out.extend(all_failures(&option.failures));
            }
        }
    }
    out
}

fn problem_paths(report: &ValidationReport) -> Vec<String> {
    report.problems.iter().map(|problem| problem.path.clone()).collect()
}

#[test]
fn a_declared_conversion_applies_below_the_old_expansion_cutoff() {
    let dir = TempDir::new().expect("tempdir");
    let schema = resolved(dir.path(), &tree_schema());
    let instance = json!({ "tree": tree(DEEP, json!(42)) });

    let outcome = coerce_frontmatter(&schema.json_schema, &instance);

    assert!(outcome.changed, "the deep number converts to text");
    assert_eq!(outcome.value, json!({ "tree": tree(DEEP, json!("42")) }));
    assert_eq!(instance, json!({ "tree": tree(DEEP, json!(42)) }), "the input is never mutated");
}

#[test]
fn final_validation_still_runs_below_the_old_expansion_cutoff() {
    let dir = TempDir::new().expect("tempdir");
    let schemas = DarkmatterSchemas::new(FileResolutionContext::new(dir.path()));
    let document: Markdown = format!("---\n$schema:\n  item: {ITEM}\n  tree: item(required)\n---\nbody\n").into();
    let effective = schemas.effective_for(&document).expect("the schema resolves").expect("an effective schema");

    let convertible = effective.validate(&json!({ "tree": tree(DEEP, json!(42)) }));
    assert!(convertible.valid, "{:#?}", convertible.problems);

    let report = effective.validate(&json!({ "tree": tree(DEEP, json!({ "not": "text" })) }));
    assert!(!report.valid);
    assert!(problem_paths(&report).contains(&leaf_pointer("tree", DEEP)), "{:#?}", report.problems);
}

#[test]
fn composition_writes_back_a_deep_recursive_conversion() {
    let dir = TempDir::new().expect("tempdir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    let path = root.join("doc.md");
    let instance = serde_json::to_string(&tree(DEEP, json!(42))).expect("JSON");
    std::fs::write(&path, format!("---\n$schema:\n  item: {ITEM}\n  tree: item(required)\ntree: {instance}\n---\nBody\n"))
        .expect("write document");
    let markdown = Markdown::try_from(path.as_path()).expect("document loads");

    let composed = match markdown.compose_with(&request(ComposeOptions::new().with_source_file(&path))) {
        Ok((composed, _)) => serde_json::to_value(composed.frontmatter().as_map()).expect("frontmatter JSON"),
        Err(MarkdownError::SchemaValidationFailed { problems, .. }) => panic!("{problems:#?}"),
        Err(other) => panic!("{other}"),
    };

    assert_eq!(composed["tree"], tree(DEEP, json!("42")));
}

#[test]
fn a_non_convertible_deep_value_leaves_the_whole_input_unchanged() {
    let dir = TempDir::new().expect("tempdir");
    let schema = resolved(dir.path(), &tree_schema());
    let instance = json!({ "tree": {
        "value": "root",
        "children": [tree(DEEP, json!(42)), tree(DEEP, json!({ "not": "text" }))],
    }});

    let outcome = coerce_frontmatter(&schema.json_schema, &instance);
    assert!(!outcome.changed, "a sibling's failure keeps the convertible branch as written");
    assert_eq!(outcome.value, instance);

    // The failure is the deep value's own type error, found by walking the
    // reference, not a validation-only cutoff above it.
    let shape = shape_from_json_schema(&schema.json_schema);
    let error = bind(&shape, &instance).expect_err("the object leaf has no conversion to text");
    let mut bad_leaf = vec![
        PathSegment::Key("tree".into()),
        PathSegment::Key("children".into()),
        PathSegment::Index(1),
    ];
    bad_leaf.extend(leaf_path("tree", DEEP).into_iter().skip(1));
    let failures = all_failures(&error.failures);
    assert!(
        failures.iter().any(|failure| failure.path == bad_leaf && failure.reason == FailureReason::NoConversion),
        "{error}"
    );
    assert!(
        !failures.iter().any(|failure| matches!(failure.reason, FailureReason::AssertionFailed { .. })),
        "no validation-only cutoff: {error}"
    );
}

#[test]
fn an_ambiguous_union_reached_through_a_reference_is_not_coerced() {
    let dir = TempDir::new().expect("tempdir");
    let schema = resolved(
        dir.path(),
        "pair: \"{ a: number, b: boolean } | { a: boolean, b: number }\"\n\
         node: \"{ value: pair(required), children: node[](required) }\"\n\
         tree: node(required)\n",
    );
    let filler = |_| json!({ "a": 1, "b": true });

    let reachable = json!({ "tree": chain(DEEP, filler, json!({ "a": "1", "b": true })) });
    let converted = coerce_frontmatter(&schema.json_schema, &reachable);
    assert!(converted.changed, "one reachable option converts below the old cutoff");
    assert_eq!(converted.value, json!({ "tree": chain(DEEP, filler, json!({ "a": 1, "b": true })) }));

    let ambiguous = json!({ "tree": chain(DEEP, filler, json!({ "a": "1", "b": "1" })) });
    let outcome = coerce_frontmatter(&schema.json_schema, &ambiguous);
    assert!(!outcome.changed, "an ambiguous union is never coerced");
    assert_eq!(outcome.value, ambiguous);
    let error = bind(&shape_from_json_schema(&schema.json_schema), &ambiguous).expect_err("ambiguous");
    assert!(
        all_failures(&error.failures).iter().any(|failure| failure.path == leaf_path("tree", DEEP)
            && matches!(failure.reason, FailureReason::AmbiguousUnion { .. })),
        "{error}"
    );
}

#[test]
fn a_value_deeper_than_the_engine_bound_fails_with_resource_limit() {
    let dir = TempDir::new().expect("tempdir");
    let schema = resolved(dir.path(), &tree_schema());
    // Each level is an object and a list, so 40 levels nest 79 values deep.
    let instance = json!({ "tree": tree(40, json!(42)) });
    let grammar = shape_from_simplified_schema_with_definitions(
        schema.simplified.as_ref().expect("a simplified schema"),
        &NoNamedTypes,
        &schema.definitions,
    )
    .expect("the grammar translator follows the reference");

    for (translator, shape) in [("JSON", shape_from_json_schema(&schema.json_schema)), ("grammar", grammar)] {
        let error = bind(&shape, &instance).expect_err("deeper than the engine walks");
        assert!(
            all_failures(&error.failures)
                .iter()
                .any(|failure| failure.reason == FailureReason::ResourceLimit { max_depth: MAX_DEPTH }),
            "{translator}: {error}"
        );
    }
    let outcome = coerce_frontmatter(&schema.json_schema, &instance);
    assert!(!outcome.changed);
    assert_eq!(outcome.value, instance);
}

fn def(text: &str) -> PropertyDef {
    parse_type_definition("p", text).expect("parses")
}

#[test]
fn a_recursive_named_type_binds_deep_function_arguments() {
    let resolver = |name: &str, _reference: &str| (name == "Tree").then(|| def("{ value: string, children: Tree[] }"));
    let registration = FunctionRegistration::builder("walk", "function(parameters(tree: Tree); returns(boolean))")
        .build_with(&resolver)
        .expect("a recursive named type translates");

    let args = [tree(DEEP, json!(42))];
    let call = registration.bind_call(&args).expect("binds");
    assert_eq!(call.args().get(0), Some(&tree(DEEP, json!("42"))));

    let bad_args = [tree(DEEP, json!({ "not": "text" }))];
    let deep_failure = registration.bind_call(&bad_args);
    assert!(deep_failure.is_err(), "a deep value is still checked");
}

#[test]
fn a_named_type_that_never_descends_still_fails_too_deep() {
    let resolver = |name: &str, _reference: &str| (name == "Loop").then(|| def("Loop | string"));
    let result = shape_from_property_def(&def("Loop"), &resolver);
    assert!(matches!(result, Err(TranslateError::TooDeep { .. })), "{result:?}");
}

#[test]
fn a_raw_descending_cycle_converts_at_any_depth() {
    let schema = json!({
        "$defs": { "node": {
            "type": "object",
            "properties": { "n": { "type": "number" }, "next": { "$ref": "#/$defs/node" } },
        }},
        "type": "object",
        "properties": { "list": { "$ref": "#/$defs/node" } },
    });
    let list = |depth: usize, n: Value| (1..depth).fold(json!({ "n": n }), |next, _| json!({ "n": 0, "next": next }));

    let instance = json!({ "list": list(DEEP, json!("7")) });
    let bound = bind(&shape_from_json_schema(&schema), &instance).expect("binds");

    assert_eq!(bound.value(), &json!({ "list": list(DEEP, json!(7)) }));
}

#[test]
fn a_raw_cycle_that_never_descends_stays_validation_only() {
    let schema = json!({
        "$defs": { "a": { "anyOf": [{ "type": "number" }, { "$ref": "#/$defs/a" }] } },
        "type": "object",
        "properties": { "v": { "$ref": "#/$defs/a" } },
    });
    let shape = shape_from_json_schema(&schema);

    // The self-reference is a passive check of its target, which the
    // complete schema judges: it matches as written, so nothing converts,
    // exactly as under the old expansion cutoff, whose deepest level was the
    // same check.
    for value in [json!("3"), json!("pear")] {
        let instance = json!({ "v": value });
        let bound = bind(&shape, &instance).expect("the validation-only option keeps it");
        assert!(bound.is_kept(), "{instance} is kept as written");
    }
    let description = shape.describe();
    assert!(description.contains("the schema at #/$defs/a"), "{description}");
}

#[test]
fn a_deep_missing_value_does_not_block_a_sibling_conversion() {
    let dir = TempDir::new().expect("tempdir");
    let schema = resolved(dir.path(), &format!("{}count: number\n", tree_schema()));
    let mut broken = tree(DEEP, json!(42));
    let mut bottom = &mut broken;
    for _ in 1..DEEP {
        bottom = &mut bottom["children"][0];
    }
    bottom.as_object_mut().expect("an object").remove("children");
    let instance = json!({ "tree": broken, "count": "3" });

    let outcome = coerce_frontmatter(&schema.json_schema, &instance);

    assert!(outcome.changed, "presence is relaxed inside the recursive body too");
    assert_eq!(outcome.value["count"], json!(3));
    assert_eq!(outcome.value["tree"], instance["tree"], "the broken property stays as written");
}
