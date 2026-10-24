//! Nested `{{ … }}` templates in schema validation: a template below a
//! top-level key, at a place the schema marks `deferred`, is accepted without
//! judging its value, while its literal siblings and the shape of the
//! collection holding it are still judged, in every validation entry point. A
//! template at an unmarked place is ordinary text. The fixture models a
//! lifecycle stack whose action operands are typed and marked `deferred`.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::validate::build_position_map;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, PendingPolicy, PendingValueReason, SchemaPhase, ValidationOptions,
    ValidationReport,
};
use serde_json::Value;

/// `success` is a list of closed action objects: `pr_list` takes an integer
/// count or a `{ state }` filter, `min` a pair of numbers, each operand
/// marked `deferred` (the union per arm). `no_error` and `strict` are
/// unmarked: a template there is judged as text. `eager` makes the stack
/// judged at launch, as Claudine's lifecycle keys are.
const STACK: &str = concat!(
    "  success: \"{ pr_list: number(integer; min(1); max(100); deferred) | { state: string(deferred) }, ",
    "min: tuple([number(deferred), number(deferred)]), no_error: boolean, strict: number }[](eager)\"\n",
    "  n: number(eager)\n",
);

fn effective() -> EffectiveSchema {
    effective_for(STACK)
}

/// The effective schema of a document whose `$schema` mapping is `schema`
/// (indented entries).
fn effective_for(schema: &str) -> EffectiveSchema {
    let source: Markdown = format!("---\n$schema:\n{schema}---\nbody\n").into();
    DarkmatterSchemas::new(crate::request_support::cwd_context())
        .effective_for(&source)
        .expect("schema resolves")
        .expect("effective schema")
}

fn instance(yaml: &str) -> Value {
    serde_yaml_ng::from_str(yaml).expect("fixture YAML parses")
}

fn options(policy: PendingPolicy) -> ValidationOptions {
    ValidationOptions {
        pending_policy: policy,
        ..ValidationOptions::default()
    }
}

/// Every entry point's report on `yaml`, labelled.
fn reports(yaml: &str) -> Vec<(&'static str, ValidationReport)> {
    reports_against(&effective(), yaml)
}

fn reports_against(schema: &EffectiveSchema, yaml: &str) -> Vec<(&'static str, ValidationReport)> {
    let value = instance(yaml);
    let positions = build_position_map(yaml);
    vec![
        ("validate_with_positions", schema.validate_with_positions(&value, &positions)),
        (
            "validate_with_options(Defer)",
            schema.validate_with_options(&value, &positions, &options(PendingPolicy::Defer)),
        ),
        (
            "validate_with_options(Report)",
            schema.validate_with_options(&value, &positions, &options(PendingPolicy::Report)),
        ),
        (
            "validate_for_phase(Launch)",
            schema
                .validate_for_phase_with_positions(&value, &positions, SchemaPhase::Launch)
                .expect("launch projection builds"),
        ),
    ]
}

fn assert_passes(yaml: &str) {
    assert_passes_against(&effective(), yaml);
}

fn assert_passes_against(schema: &EffectiveSchema, yaml: &str) {
    for (entry, report) in reports_against(schema, yaml) {
        assert!(report.valid, "{entry} rejected\n{yaml}\n{:#?}", report.problems);
    }
}

/// Every entry point reports a problem at `path`, located in the source.
fn assert_fails_at(yaml: &str, path: &str) {
    assert_fails_at_against(&effective(), yaml, path);
}

/// Every entry point reports a problem at `path`, located in the source,
/// except that `validate_with_options(Defer)` mirrors compose's pre-shell
/// gate: a problem whose own authored value holds a template is set aside for
/// the pass that renders it.
fn assert_fails_at_against(schema: &EffectiveSchema, yaml: &str, path: &str) {
    let authored_template = instance(yaml)
        .pointer(path)
        .and_then(Value::as_str)
        .is_some_and(|text| text.contains("{{"));
    for (entry, report) in reports_against(schema, yaml) {
        if authored_template && entry == "validate_with_options(Defer)" {
            assert!(
                report.problems.iter().all(|problem| problem.path != path),
                "{entry}: a template's own problem waits for rendering\n{yaml}\n{:#?}",
                report.problems
            );
            continue;
        }
        let problem = report
            .problems
            .iter()
            .find(|problem| problem.path == path)
            .unwrap_or_else(|| panic!("{entry}: no problem at {path}\n{yaml}\n{:#?}", report.problems));
        assert!(!report.valid, "{entry}");
        assert!(problem.line.is_some() && problem.column.is_some(), "{entry}: unpositioned {problem:#?}");
    }
}

macro_rules! verdicts {
    ($($name:ident: $yaml:expr => $verdict:tt $(at $path:expr)?;)*) => {
        $(
            #[test]
            fn $name() {
                verdicts!(@check $yaml, $verdict $(, $path)?);
            }
        )*
    };
    (@check $yaml:expr, passes) => { assert_passes($yaml) };
    (@check $yaml:expr, fails, $path:expr) => { assert_fails_at($yaml, $path) };
}

verdicts! {
    // Controls: literals are judged through the shared conversion rules.
    literal_stack_passes:
        "success:\n  - pr_list: 3\n  - min: [1, 2]\n  - no_error: true\n" => passes;
    numeric_text_operand_converts:
        "success:\n  - pr_list: \"3\"\n" => passes;
    non_numeric_text_operand_fails:
        "success:\n  - pr_list: three\n" => fails at "/success/0/pr_list";

    // A recognized template is deferred whatever it holds.
    whole_template_operand_is_deferred:
        "success:\n  - pr_list: \"{{ fm.count }}\"\n" => passes;
    mixed_text_template_operand_is_deferred:
        "success:\n  - pr_list: \"n={{ fm.count }}\"\n" => passes;
    malformed_template_is_judged_as_written:
        "success:\n  - pr_list: \"{{ 1 + }}\"\n" => fails at "/success/0/pr_list";
    unclosed_template_is_a_literal:
        "success:\n  - pr_list: \"{{ fm.count\"\n" => fails at "/success/0/pr_list";
    template_is_never_evaluated:
        "success:\n  - pr_list: \"{{ undefined_function() }}\"\n" => passes;

    // A tuple defers one slot and keeps its shape.
    tuple_defers_one_slot:
        "success:\n  - min: [1, \"{{ fm.x }}\"]\n" => passes;
    tuple_still_judges_a_literal_sibling:
        "success:\n  - min: [three, \"{{ fm.x }}\"]\n" => fails at "/success/0/min/0";
    tuple_still_rejects_an_extra_element:
        "success:\n  - min: [1, \"{{ x }}\", 3]\n" => fails at "/success/0/min/2";
    tuple_still_requires_every_slot:
        "success:\n  - min: [\"{{ x }}\"]\n" => fails at "/success/0/min";

    // A structured operand defers one field and stays closed.
    structured_operand_defers_a_field:
        "success:\n  - pr_list:\n      state: \"{{ fm.s }}\"\n" => passes;
    structured_operand_stays_closed:
        "success:\n  - pr_list:\n      state: \"{{ fm.s }}\"\n      bogus: 1\n" => fails at "/success/0/pr_list";

    // A misspelled verb beside a template is reported, in the same action
    // or in another action of the same list.
    misspelled_verb_beside_a_template_in_the_same_action_fails:
        "success:\n  - pr_list: \"{{ fm.count }}\"\n    no_eror: true\n" => fails at "/success/0";
    misspelled_verb_beside_a_template_in_the_same_list_fails:
        "success:\n  - pr_list: \"{{ fm.count }}\"\n  - no_eror: true\n" => fails at "/success/1";

    // Only a place the schema marks `deferred` defers: the same template
    // anywhere else is text, judged against its declared type, in every entry
    // point (`PendingPolicy::Defer` included).
    template_at_an_unmarked_number_is_judged_as_text:
        "success:\n  - strict: \"{{ fm.count }}\"\n" => fails at "/success/0/strict";
    template_at_an_unmarked_boolean_is_judged_as_text:
        "success:\n  - no_error: \"{{ fm.ok }}\"\n" => fails at "/success/0/no_error";
    unmarked_template_beside_a_marked_one_is_still_judged:
        "success:\n  - pr_list: \"{{ fm.count }}\"\n    strict: \"{{ fm.count }}\"\n" => fails at "/success/0/strict";
}

/// A recursive named type whose `n` is marked `deferred` and whose `m` is not.
const TREE: &str = concat!(
    "  node: \"{ n: number(deferred), m: number, kids: node[] }\"\n",
    "  tree: node\n",
);

/// A marked place inside a named, recursive type defers at every depth; an
/// unmarked sibling in the same definition is judged.
#[test]
fn a_marked_place_inside_a_recursive_type_defers_at_every_depth() {
    let schema = effective_for(TREE);
    assert_passes_against(&schema, "tree:\n  n: \"{{ x }}\"\n");
    assert_passes_against(
        &schema,
        "tree:\n  n: 1\n  kids:\n    - n: 2\n      kids:\n        - n: \"{{ fm.deep }}\"\n",
    );
    assert_fails_at_against(&schema, "tree:\n  kids:\n    - m: \"{{ x }}\"\n", "/tree/kids/0/m");
    assert_fails_at_against(&schema, "tree:\n  kids:\n    - n: three\n", "/tree/kids/0/n");
}

/// A union marks per alternative: `number(deferred) | boolean` defers a
/// template through its marked arm, while `number | boolean`, which no arm
/// marks, judges it and rejects it.
#[test]
fn a_marked_union_arm_defers_where_an_unmarked_union_would_fail() {
    let schema = effective_for("  pick: \"{ marked: number(deferred) | boolean, unmarked: number | boolean }\"\n");
    assert_passes_against(&schema, "pick:\n  marked: \"{{ x }}\"\n");
    assert_fails_at_against(&schema, "pick:\n  unmarked: \"{{ x }}\"\n", "/pick/unmarked");
    // The marked union still judges a literal through every arm.
    assert_fails_at_against(&schema, "pick:\n  marked: three\n", "/pick/marked");
}

/// The located position of a converted-text failure is the operand's line.
#[test]
fn a_failing_operand_is_located_on_its_line() {
    let yaml = "success:\n  - min: [1, 2]\n  - pr_list: three\n";
    for (entry, report) in reports(yaml) {
        let problem = &report.problems[0];
        assert_eq!((problem.path.as_str(), problem.line), ("/success/1/pr_list", Some(3)), "{entry}");
    }
}

/// A whole top-level template keeps its pending-key rule: deferred under
/// `Defer`, reported under `Report`, listed either way.
#[test]
fn a_top_level_template_keeps_its_pending_rule() {
    let schema = effective();
    let yaml = "n: \"{{ x }}\"\n";
    let value = instance(yaml);
    let positions = build_position_map(yaml);

    let deferred = schema.validate_with_options(&value, &positions, &options(PendingPolicy::Defer));
    assert!(deferred.valid, "{:#?}", deferred.problems);
    let reported = schema.validate_with_options(&value, &positions, &options(PendingPolicy::Report));
    assert!(!reported.valid);
    assert_eq!(reported.problems[0].path, "/n");
    for report in [&deferred, &reported] {
        let pending: Vec<_> = report.pending.iter().map(|p| (p.key.as_str(), p.reason)).collect();
        assert_eq!(pending, [("n", PendingValueReason::UnresolvedTemplate)]);
    }
    // Without options a top-level template is judged as written, as before.
    assert!(!schema.validate_with_positions(&value, &positions).valid);
}

/// A list holding a template is still listed as pending, but only its
/// templates are deferred.
#[test]
fn a_collection_holding_a_template_is_listed_but_not_dropped_wholesale() {
    let schema = effective();
    let yaml = "success:\n  - pr_list: \"{{ fm.count }}\"\n  - no_eror: true\n";
    let value = instance(yaml);
    let report = schema.validate_with_options(&value, &build_position_map(yaml), &options(PendingPolicy::Defer));
    let pending: Vec<_> = report.pending.iter().map(|p| (p.key.as_str(), p.reason)).collect();
    assert_eq!(pending, [("success", PendingValueReason::UnresolvedTemplate)]);
    let paths: Vec<&str> = report.problems.iter().map(|problem| problem.path.as_str()).collect();
    assert_eq!(paths, ["/success/1"], "{:#?}", report.problems);
}

/// Completion validates the instance as parsed, and defers nested templates
/// the same way.
#[test]
fn completion_phase_defers_a_nested_template() {
    let schema = effective();
    let deferred = instance("success:\n  - pr_list: \"{{ fm.count }}\"\n    min: [1, \"{{ x }}\"]\n");
    let report = schema.validate_for_phase(&deferred, SchemaPhase::Completion).expect("phase builds");
    assert!(report.valid, "{:#?}", report.problems);

    let sibling = instance("success:\n  - min: [three, \"{{ x }}\"]\n");
    let report = schema.validate_for_phase(&sibling, SchemaPhase::Completion).expect("phase builds");
    let paths: Vec<&str> = report.problems.iter().map(|problem| problem.path.as_str()).collect();
    assert_eq!(paths, ["/success/0/min/0"], "{:#?}", report.problems);
}
