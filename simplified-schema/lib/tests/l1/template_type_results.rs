//! Template-type resolution results: what a `{{ … }}` slot standing for a
//! property's whole type becomes once the host answers it.
//!
//! Boundary: the crate's public API, in-process. Each template is resolved
//! through `SimplifiedSchema::resolve_slots` with a stub host that answers
//! slots from a table and reads no file. A positive case is proved by
//! equivalence with the static schema an author could have written: the
//! same JSON Schema and the same strict verdicts on a valid and an invalid
//! document. A rejected value is proved by its `SlotResultKind`, the slot it
//! names, and the property it is reported at.

use std::cell::Cell;
use std::rc::Rc;

use serde_json::{Value, json};
use simplified_schema::slots::{
    RefuseTemplateReads, SlotAdmissionError, SlotFailure, SlotHost, SlotRequest, SlotResolutionError, TemplateReads,
};
use simplified_schema::{Requiredness, SchemaError, SimplifiedSchema, Slot, SlotRelated, SlotResultKind};

// ── helpers ──────────────────────────────────────────────────────────────

/// Answers each slot by expression and counts evaluations.
struct Answers {
    values: Vec<(&'static str, Value)>,
    calls: Cell<usize>,
}

impl Answers {
    fn new(values: &[(&'static str, Value)]) -> Self {
        Self { values: values.to_vec(), calls: Cell::new(0) }
    }
}

impl SlotHost for Answers {
    fn admit(&self, _: &Slot) -> Result<(), SlotAdmissionError> {
        Ok(())
    }

    fn resolve(&self, request: &SlotRequest<'_>) -> Result<Value, SlotFailure> {
        self.calls.set(self.calls.get() + 1);
        let expression = request.slot.expression.as_str();
        let (_, value) = self
            .values
            .iter()
            .find(|(name, _)| *name == expression)
            .unwrap_or_else(|| panic!("the stub has no answer for `{expression}`"));
        Ok(value.clone())
    }

    fn template_reads(&self) -> Rc<dyn TemplateReads> {
        Rc::new(RefuseTemplateReads)
    }
}

/// A standalone schema document whose `$schema` mapping is `body`.
fn document(body: &str) -> String {
    let indented: Vec<String> = body.lines().map(|line| format!("  {line}")).collect();
    format!("$schema:\n{}\n", indented.join("\n"))
}

fn parse(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("{error}\n{text}"))
}

/// Resolves the template document `text` against `host`.
fn resolve(text: &str, host: &Answers) -> Result<SimplifiedSchema, SlotResolutionError> {
    let template = parse(text);
    assert!(!template.is_resolved(), "expected a template:\n{text}");
    template.resolve_slots(host)
}

fn json_schema(schema: &SimplifiedSchema) -> Value {
    (*schema.to_json_schema().expect("the schema exports")).clone()
}

/// The template `template` with `answers` resolves to exactly what the
/// static document `concrete` declares, and both judge `valid` and
/// `invalid` alike (and correctly).
#[track_caller]
fn assert_equivalent(template: &str, answers: &[(&'static str, Value)], concrete: &str, valid: &Value, invalid: &Value) {
    let host = Answers::new(answers);
    let resolved = resolve(template, &host).unwrap_or_else(|error| panic!("{template}: {error}"));
    let expected = parse(concrete);
    assert!(expected.is_resolved(), "the control is static:\n{concrete}");
    assert_eq!(json_schema(&resolved), json_schema(&expected), "{template} vs {concrete}");
    assert!(resolved.validate_strict(valid).is_ok(), "{template} accepts {valid}");
    assert!(expected.validate_strict(valid).is_ok(), "{concrete} accepts {valid}");
    assert!(resolved.validate_strict(invalid).is_err(), "{template} rejects {invalid}");
    assert!(expected.validate_strict(invalid).is_err(), "{concrete} rejects {invalid}");
}

/// The `SchemaError::SlotResult` a failed resolution reports.
#[track_caller]
fn slot_result(result: Result<SimplifiedSchema, SlotResolutionError>) -> (Slot, SlotResultKind, Option<String>, Vec<SlotRelated>) {
    match result {
        Err(SlotResolutionError::Schema(error)) => match *error {
            SchemaError::SlotResult { slot, kind, property, related } => (*slot, kind, property, related),
            other => panic!("expected a slot result error, got {other:?}"),
        },
        Err(other) => panic!("expected a schema error, got {other:?}"),
        Ok(schema) => panic!("expected a slot result error, resolution succeeded; export: {:?}", schema.to_json_schema()),
    }
}

/// The `status: "{{ t }}"` template answered with `value`.
fn status_template(value: Value) -> Result<SimplifiedSchema, SlotResolutionError> {
    resolve(&document("status: \"{{ t }}\""), &Answers::new(&[("t", value)]))
}

/// A `NotATypeDefinition` at `status` naming slot `t`; returns the wrapped
/// definition error's message.
#[track_caller]
fn assert_not_a_type_definition(value: Value) -> String {
    let (slot, kind, property, _) = slot_result(status_template(value.clone()));
    assert_eq!(slot.expression, "t", "{value}");
    assert_eq!(property.as_deref(), Some("status"), "{value}");
    match kind {
        SlotResultKind::NotATypeDefinition(error) => error.to_string(),
        other => panic!("{value}: expected NotATypeDefinition, got {other:?}"),
    }
}

fn required(schema: &SimplifiedSchema) -> Vec<String> {
    json_schema(schema)["required"]
        .as_array()
        .map(|names| names.iter().map(|name| name.as_str().expect("names are strings").to_owned()).collect())
        .unwrap_or_default()
}

// ── control ──────────────────────────────────────────────────────────────

#[test]
fn enum_value_equals_static_enum() {
    assert_equivalent(
        &document("status: \"{{ t }}\""),
        &[("t", json!("enum(draft, published)"))],
        &document("status: enum(draft, published)"),
        &json!({ "status": "draft" }),
        &json!({ "status": "archived" }),
    );
}

// ── values that are not a type definition ────────────────────────────────

#[test]
fn non_string_non_array_values_are_not_type_definitions() {
    for (value, found) in [(json!(null), "null"), (json!({ "a": 1 }), "mapping"), (json!(1), "number"), (json!(true), "boolean")]
    {
        let message = assert_not_a_type_definition(value.clone());
        assert!(
            message.contains("expected a type definition (a string or an array)") && message.contains(found),
            "{value}: {message}"
        );
    }
}

#[test]
fn empty_array_keeps_the_ordinary_union_error() {
    let message = assert_not_a_type_definition(json!([]));
    assert!(message.contains("property-level union must have at least one arm"), "{message}");
}

#[test]
fn empty_string_keeps_the_ordinary_definition_error() {
    let message = assert_not_a_type_definition(json!(""));
    assert!(message.contains("expected a type name"), "{message}");
}

#[test]
fn array_with_a_non_string_element_is_rejected_not_filtered() {
    for value in [json!(["string", 123]), json!([123])] {
        let (slot, _, property, _) = slot_result(status_template(value.clone()));
        assert_eq!(slot.expression, "t", "{value}");
        assert_eq!(property.as_deref(), Some("status"), "{value}");
    }
}

// ── null and list forms ──────────────────────────────────────────────────

#[test]
fn null_string_is_the_null_type() {
    assert_equivalent(
        &document("status: \"{{ t }}\""),
        &[("t", json!("null"))],
        &document("status: \"null\""),
        &json!({ "status": null }),
        &json!({ "status": "x" }),
    );
}

#[test]
fn array_holding_json_null_is_the_null_type_list_form() {
    assert_equivalent(
        &document("status: \"{{ t }}\""),
        &[("t", json!([null]))],
        &document("status: [null]"),
        &json!({ "status": null }),
        &json!({ "status": 1 }),
    );
}

#[test]
fn string_encoded_list_is_a_union() {
    assert_equivalent(
        &document("status: \"{{ t }}\""),
        &[("t", json!("[string, number]"))],
        &document("status: [string, number]"),
        &json!({ "status": 3 }),
        &json!({ "status": true }),
    );
}

#[test]
fn array_result_flattens_into_a_yaml_list_union() {
    assert_equivalent(
        &document("status: [string, \"{{ t }}\"]"),
        &[("t", json!(["number", "boolean"]))],
        &document("status: [string, number, boolean]"),
        &json!({ "status": true }),
        &json!({ "status": [] }),
    );
}

#[test]
fn array_result_flattens_into_a_pipe_union() {
    assert_equivalent(
        &document("status: \"string | {{ t }}\""),
        &[("t", json!(["number", "boolean"]))],
        &document("status: [string, number, boolean]"),
        &json!({ "status": 1 }),
        &json!({ "status": [] }),
    );
}

#[test]
fn array_suffix_applies_to_the_whole_returned_union() {
    assert_equivalent(
        &document("status: \"{{ t }}[]\""),
        &[("t", json!(["number", "boolean"]))],
        &document("status: (number | boolean)[]"),
        &json!({ "status": [1, true] }),
        &json!({ "status": ["x"] }),
    );
}

// ── requiredness ─────────────────────────────────────────────────────────

#[test]
fn required_suffix_over_null_admitting_types_stays_optional() {
    for value in [json!("null"), json!("unknown"), json!("string | null")] {
        let host = Answers::new(&[("t", value.clone())]);
        let resolved = resolve(&document("status: \"{{ t }}(required)\""), &host)
            .unwrap_or_else(|error| panic!("{value}: {error}"));
        assert!(!required(&resolved).contains(&"status".to_owned()), "{value}: {}", json_schema(&resolved));
        assert!(resolved.validate_strict(&json!({})).is_ok(), "{value}: an absent status is accepted");
    }
}

#[test]
fn required_suffix_over_string_is_required() {
    assert_equivalent(
        &document("status: \"{{ t }}(required)\""),
        &[("t", json!("string"))],
        &document("status: string(required)"),
        &json!({ "status": "x" }),
        &json!({}),
    );
}

#[test]
fn nullable_items_do_not_make_the_array_nullable() {
    let host = Answers::new(&[("t", json!("string | null"))]);
    let resolved = resolve(&document("status: \"{{ t }}[](required)\""), &host).expect("resolves");
    assert_eq!(required(&resolved), vec!["status".to_owned()]);
    assert!(resolved.validate_strict(&json!({ "status": ["a", null] })).is_ok(), "items admit null");
    assert!(resolved.validate_strict(&json!({ "status": null })).is_err(), "the array is not nullable");
    assert!(resolved.validate_strict(&json!({})).is_err(), "the array is required");
}

// ── suffix merge ─────────────────────────────────────────────────────────

#[test]
fn conflicting_default_is_a_suffix_conflict() {
    let host = Answers::new(&[("t", json!("string(default(a))"))]);
    let result = resolve(&document("status: \"{{ t }}(default(b))\""), &host);
    let rendered = match &result {
        Err(error) => error.to_string(),
        Ok(_) => String::new(),
    };
    let (slot, kind, property, related) = slot_result(result);
    assert_eq!(slot.expression, "t");
    assert_eq!(property.as_deref(), Some("status"));
    let reason = match kind {
        SlotResultKind::SuffixConflict(reason) => reason,
        other => panic!("expected SuffixConflict, got {other:?}"),
    };
    assert!(
        related.iter().any(|entry| entry.message.contains("the suffix written after the slot")),
        "related: {related:?}"
    );
    for text in [&reason, &rendered] {
        assert!(!text.contains("{{ template type }}"), "the internal name leaks: {text}");
    }
}

#[test]
fn non_null_over_a_null_admitting_result_is_an_invalid_expansion() {
    let host = Answers::new(&[("t", json!("string | null"))]);
    let (slot, kind, property, _) = slot_result(resolve(&document("status: \"{{ t }}(non-null)\""), &host));
    assert_eq!(slot.expression, "t");
    assert_eq!(property.as_deref(), Some("status"));
    assert!(matches!(kind, SlotResultKind::InvalidExpansion(_)), "{kind:?}");
}

#[test]
fn template_constraint_in_the_suffix_still_resolves() {
    assert_equivalent(
        &document("status: \"{{ t }}(default({{ d }}))\""),
        &[("t", json!("string")), ("d", json!("x"))],
        &document("status: string(default(x))"),
        &json!({ "status": "y" }),
        &json!({ "status": 1 }),
    );
}

#[test]
fn array_and_constraint_suffix_merge_like_a_named_type() {
    assert_equivalent(
        &document("status: \"{{ t }}[](min(1))\""),
        &[("t", json!("enum(draft, published)"))],
        &document("status: enum(draft, published)[](min(1))"),
        &json!({ "status": ["draft"] }),
        &json!({ "status": [] }),
    );
}

#[test]
fn suffix_constraint_over_a_type_without_it_is_rejected() {
    let host = Answers::new(&[("t", json!("boolean"))]);
    let (slot, kind, property, _) = slot_result(resolve(&document("status: \"{{ t }}(pattern('^a'))\""), &host));
    assert_eq!(slot.expression, "t");
    assert_eq!(property.as_deref(), Some("status"));
    assert!(matches!(kind, SlotResultKind::SuffixConflict(_)), "{kind:?}");
}

#[test]
fn description_suffix_survives_expansion() {
    assert_equivalent(
        &document("status: \"{{ t }} -> where it is\""),
        &[("t", json!("string"))],
        &document("status: string -> where it is"),
        &json!({ "status": "x" }),
        &json!({ "status": 1 }),
    );
    let host = Answers::new(&[("t", json!("string"))]);
    let resolved = resolve(&document("status: \"{{ t }} -> where it is\""), &host).expect("resolves");
    assert!(json_schema(&resolved).to_string().contains("where it is"), "{}", json_schema(&resolved));
}

// ── positions ────────────────────────────────────────────────────────────

#[test]
fn inline_object_property_position() {
    assert_equivalent(
        &document("status: \"{ a: {{ t }} }\""),
        &[("t", json!("number"))],
        &document("status: \"{ a: number }\""),
        &json!({ "status": { "a": 1 } }),
        &json!({ "status": { "a": "x" } }),
    );
}

#[test]
fn nested_mapping_property_position() {
    assert_equivalent(
        &document("meta:\n  status: \"{{ t }}\""),
        &[("t", json!("enum(a, b)"))],
        &document("meta:\n  status: enum(a, b)"),
        &json!({ "meta": { "status": "a" } }),
        &json!({ "meta": { "status": "c" } }),
    );
}

#[test]
fn nested_mapping_reports_the_property_path_for_a_wrong_shape() {
    let host = Answers::new(&[("t", json!({ "a": 1 }))]);
    let (slot, kind, property, _) = slot_result(resolve(&document("meta:\n  status: \"{{ t }}\""), &host));
    assert_eq!(slot.expression, "t");
    assert!(property.as_deref().is_some_and(|path| path.contains("status")), "{property:?}");
    assert!(matches!(kind, SlotResultKind::NotATypeDefinition(_)), "{kind:?}");
}

#[test]
fn named_type_body_position() {
    assert_equivalent(
        "kind: schema\n$schema:\n  status: State@this\ntypes:\n  State: \"{{ t }}\"\n",
        &[("t", json!("enum(on, off)"))],
        "kind: schema\n$schema:\n  status: State@this\ntypes:\n  State: enum(on, off)\n",
        &json!({ "status": "on" }),
        &json!({ "status": "dim" }),
    );
}

#[test]
fn tuple_element_position() {
    assert_equivalent(
        &document("status: \"tuple([number, {{ t }}])\""),
        &[("t", json!("string"))],
        &document("status: tuple([number, string])"),
        &json!({ "status": [1, "x"] }),
        &json!({ "status": [1, 2] }),
    );
}

#[test]
fn grouped_union_with_array_suffix_position() {
    assert_equivalent(
        &document("status: \"({{ t }} | string)[]\""),
        &[("t", json!("number | boolean"))],
        &document("status: (number | boolean | string)[]"),
        &json!({ "status": [1, true, "x"] }),
        &json!({ "status": [null] }),
    );
}

// ── slot identity ────────────────────────────────────────────────────────

#[test]
fn named_type_template_used_twice_is_evaluated_once() {
    let host = Answers::new(&[("t", json!("enum(on, off)"))]);
    let resolved = resolve(
        "kind: schema\n$schema:\n  first: State@this\n  second: State@this\ntypes:\n  State: \"{{ t }}\"\n",
        &host,
    )
    .expect("resolves");
    assert_eq!(host.calls.get(), 1, "one slot, one evaluation");
    assert!(resolved.validate_strict(&json!({ "first": "on", "second": "off" })).is_ok());
    assert!(resolved.validate_strict(&json!({ "first": "dim" })).is_err(), "first has the type");
    assert!(resolved.validate_strict(&json!({ "second": "dim" })).is_err(), "second has the type");
}

// ── nested grouped unions ────────────────────────────────────────────────
//
// A grouped union holding a template type is substituted at every depth a
// type may be written, not only as a property's whole type. Each case is the
// template answered with `string` against the same text with `string`
// written in place of the slot.

/// The status template `body` (with `{{ t }}` for the slot) answered with
/// `string`, against the same document with `string` written there.
#[track_caller]
fn assert_string_equivalent(body: &str, valid: &Value, invalid: &Value) {
    let template = document(body);
    let concrete = document(&body.replace("{{ t }}", "string"));
    assert_equivalent(&template, &[("t", json!("string"))], &concrete, valid, invalid);
    let resolved = resolve(&template, &Answers::new(&[("t", json!("string"))])).expect("resolves");
    assert!(resolved.is_resolved(), "lowering receives a fully substituted type: {body}");
}

#[test]
fn bare_slot_equals_static() {
    assert_string_equivalent("status: \"{{ t }}\"", &json!({ "status": "x" }), &json!({ "status": 1 }));
}

#[test]
fn bare_slot_with_array_suffix_equals_static() {
    assert_string_equivalent("status: \"{{ t }}[]\"", &json!({ "status": ["x"] }), &json!({ "status": [1] }));
}

#[test]
fn whole_property_grouped_union_equals_static() {
    assert_string_equivalent(
        "status: \"({{ t }} | boolean)[]\"",
        &json!({ "status": ["x", true] }),
        &json!({ "status": [1] }),
    );
}

#[test]
fn direct_union_arm_equals_static() {
    assert_string_equivalent("status: \"number | {{ t }}\"", &json!({ "status": "x" }), &json!({ "status": true }));
}

#[test]
fn grouped_union_as_a_union_arm_equals_static() {
    assert_string_equivalent(
        "status: \"number | ({{ t }} | boolean)[]\"",
        &json!({ "status": ["x", false] }),
        &json!({ "status": [1] }),
    );
}

#[test]
fn grouped_union_inside_a_nested_group_equals_static() {
    assert_string_equivalent(
        "status: \"(number | ({{ t }} | boolean)[])[]\"",
        &json!({ "status": [1, ["x", true]] }),
        &json!({ "status": [[1]] }),
    );
}

#[test]
fn ordinary_object_property_equals_static() {
    assert_string_equivalent("status: \"{ a: {{ t }} }\"", &json!({ "status": { "a": "x" } }), &json!({ "status": { "a": 1 } }));
}

#[test]
fn grouped_union_arm_in_an_object_property_equals_static() {
    assert_string_equivalent(
        "status: \"{ a: number | ({{ t }} | boolean)[] }\"",
        &json!({ "status": { "a": ["x"] } }),
        &json!({ "status": { "a": [1] } }),
    );
}

#[test]
fn grouped_union_arm_in_a_pattern_key_property_equals_static() {
    assert_string_equivalent(
        "status:\n  \"<string>\": \"number | ({{ t }} | boolean)[]\"",
        &json!({ "status": { "k": ["x"] } }),
        &json!({ "status": { "k": [1] } }),
    );
}

#[test]
fn direct_tuple_element_equals_static() {
    assert_string_equivalent("status: \"tuple([number, {{ t }}])\"", &json!({ "status": [1, "x"] }), &json!({ "status": [1, 2] }));
}

#[test]
fn grouped_union_arm_in_a_tuple_element_equals_static() {
    assert_string_equivalent(
        "status: \"tuple([number | ({{ t }} | boolean)[]])\"",
        &json!({ "status": [["x"]] }),
        &json!({ "status": [[1]] }),
    );
}

#[test]
fn grouped_union_arm_in_a_named_type_used_twice_equals_static() {
    let body = "kind: schema\n$schema:\n  first: State@this\n  second: State@this\ntypes:\n  State: \"number | ({{ t }} | boolean)[]\"\n";
    let host = Answers::new(&[("t", json!("string"))]);
    let resolved = resolve(body, &host).expect("resolves");
    assert!(resolved.is_resolved());
    assert_eq!(host.calls.get(), 1, "one slot, one evaluation");
    let expected = parse(&body.replace("{{ t }}", "string"));
    assert_eq!(json_schema(&resolved), json_schema(&expected));
    for schema in [&resolved, &expected] {
        assert!(schema.validate_strict(&json!({ "first": ["x"], "second": 1 })).is_ok());
        assert!(schema.validate_strict(&json!({ "first": [1] })).is_err(), "first has the type");
        assert!(schema.validate_strict(&json!({ "second": [1] })).is_err(), "second has the type");
    }
}

#[test]
fn nested_grouped_union_keeps_its_array_constraints() {
    assert_string_equivalent(
        "status: \"number | ({{ t }} | boolean)[](min(2))\"",
        &json!({ "status": ["x", true] }),
        &json!({ "status": ["x"] }),
    );
}

#[test]
fn nested_grouped_union_takes_a_returned_null_arm_as_an_item() {
    assert_equivalent(
        &document("status: \"number | ({{ t }} | boolean)[]\""),
        &[("t", json!("string | null"))],
        &document("status: \"number | (string | null | boolean)[]\""),
        &json!({ "status": ["x", null] }),
        &json!({ "status": [1] }),
    );
}

#[test]
fn nested_grouped_union_hoists_a_postfixed_returned_union_marker() {
    // The suffix `(required)` on a slot answered with a union is a postfixed
    // union arm: its marker hoists to the group, as `U(required)` does.
    let template = "kind: schema\n$schema:\n  status: \"number | ({{ t }}(required) | boolean)[]\"\n";
    let concrete = "kind: schema\n$schema:\n  status: \"number | (U(required) | boolean)[]\"\ntypes:\n  U: string | null\n";
    assert_equivalent(template, &[("t", json!("string | null"))], concrete, &json!({ "status": ["x", null] }), &json!({}));
    let resolved = resolve(template, &Answers::new(&[("t", json!("string | null"))])).expect("resolves");
    assert_eq!(required(&resolved), vec!["status".to_owned()]);
}

#[test]
fn nested_template_constraint_resolves() {
    assert_equivalent(
        &document("status: \"number | (enum({{ e }}) | boolean)[]\""),
        &[("e", json!(["a", "b"]))],
        &document("status: \"number | (enum(a, b) | boolean)[]\""),
        &json!({ "status": ["a", true] }),
        &json!({ "status": ["c"] }),
    );
}

#[test]
fn non_null_after_nested_substitution_is_a_located_invalid_expansion() {
    // The static `null | (string(non-null) | boolean)[]` is a grammar error;
    // the template is not, until the returned `non-null` joins the group.
    assert!(SimplifiedSchema::try_from(document("status: \"null | (string(non-null) | boolean)[]\"").as_str()).is_err());
    let host = Answers::new(&[("t", json!("string(non-null)"))]);
    let (slot, kind, property, _) = slot_result(resolve(&document("status: \"null | ({{ t }} | boolean)[]\""), &host));
    assert_eq!(slot.expression, "t");
    assert_eq!(property.as_deref(), Some("status"));
    assert!(matches!(kind, SlotResultKind::InvalidExpansion(_)), "{kind:?}");
}

#[test]
fn a_slot_in_an_item_union_leaves_presence_pending_until_it_resolves() {
    // The returned type's `required` marker hoists from an item alternative
    // to the property, exactly as the static text's does.
    for body in ["status: \"({{ t }} | boolean)[]\"", "status: \"number | ({{ t }} | boolean)[]\""] {
        let template = parse(&document(body));
        let status = template.property("status").expect("status is declared");
        assert_eq!(status.requiredness(), Requiredness::Pending, "{body}");
        assert!(!status.is_required(), "{body}: the compatibility boolean is false while pending");
        assert_equivalent(
            &document(body),
            &[("t", json!("string(required)"))],
            &document(&body.replace("{{ t }}", "string(required)")),
            &json!({ "status": ["x"] }),
            &json!({}),
        );
        let resolved = resolve(&document(body), &Answers::new(&[("t", json!("string(required)"))])).expect("resolves");
        assert_eq!(resolved.property("status").expect("declared").requiredness(), Requiredness::Required, "{body}");
    }
}
