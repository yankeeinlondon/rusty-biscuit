//! Default preparation, then validation: a property whose value is missing
//! or `null` and that declares a `default(...)` (or a raw JSON Schema
//! `default`) is filled, required or not; `required` asks only whether the
//! key was written or supplied by a default. Strict and soft validation judge
//! the completed value and report the defaults in
//! `ValidationSuccess::defaults`; strict returns the input borrowed, soft an
//! owned copy with the defaults applied.
//!
//! Boundary: the crate's public API (`SimplifiedSchema`, and
//! `defaults::{DefaultPlan, prepare_defaults}` for host-supplied templates),
//! in-process. F1: every input is in-memory schema text or JSON.

use std::borrow::Cow;
use std::collections::HashMap;

use serde_json::{Value, json};
use simplified_schema::slots::{RefuseTemplateReads, SlotAdmissionError, SlotFailure, SlotHost, SlotRequest, TemplateReads};
use simplified_schema::{
    AppliedDefault, PropertyPath, SchemaError, SimplifiedSchema, Slot, ValidationFailure, ValidationProblemCode,
    ValidationSuccess,
};

fn schema(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).expect("schema text")
}

/// The default at `path`, declared at `pointer` in the lowered schema, as
/// this crate reports it (no host-supplied origin).
fn applied(path: &str, value: Value, pointer: &str) -> AppliedDefault {
    AppliedDefault { path: PropertyPath::parse(path).expect("path"), value, pointer: pointer.to_string(), origin: None }
}

/// A default at `path` holding `value`, wherever the schema declares it:
/// it equals any [`AppliedDefault`] with that path and value.
#[derive(Debug)]
struct Placed {
    path: PropertyPath,
    value: Value,
}

impl PartialEq<Placed> for AppliedDefault {
    fn eq(&self, other: &Placed) -> bool {
        self.path == other.path && self.value == other.value
    }
}

fn placed(path: &str, value: Value) -> Placed {
    Placed { path: PropertyPath::parse(path).expect("path"), value }
}

/// The paths of `defaults`, canonically spelled.
fn paths(defaults: &[AppliedDefault]) -> Vec<String> {
    defaults.iter().map(|applied| applied.path.to_string()).collect()
}

#[track_caller]
fn borrowed<'a>(result: Result<ValidationSuccess<'a>, ValidationFailure>, input: &Value) -> ValidationSuccess<'a> {
    match result {
        Ok(success @ ValidationSuccess { value: Cow::Borrowed(_), .. }) => {
            assert!(std::ptr::eq(success.value.as_ref(), input), "the accepted value is the caller's own input");
            assert!(!success.is_modified());
            success
        }
        other => panic!("expected the input accepted borrowed, got {other:?}"),
    }
}

#[track_caller]
fn owned(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> (Value, Vec<AppliedDefault>) {
    match result {
        Ok(success @ ValidationSuccess { value: Cow::Owned(_), .. }) => {
            assert!(success.is_modified());
            (success.value.into_owned(), success.defaults)
        }
        other => panic!("expected an owned modified value, got {other:?}"),
    }
}

#[track_caller]
fn schema_error(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> SchemaError {
    match result {
        // A default's error is built per validation, so this result is its
        // only holder.
        Err(ValidationFailure::Unprepared { error }) => std::sync::Arc::try_unwrap(error).expect("one holder"),
        other => panic!("expected a schema error, got {other:?}"),
    }
}

/// The schema error of an unprepared schema, which the schema may share
/// with every later validation.
#[track_caller]
fn shared_schema_error(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> std::sync::Arc<SchemaError> {
    match result {
        Err(ValidationFailure::Unprepared { error }) => error,
        other => panic!("expected a schema error, got {other:?}"),
    }
}

#[track_caller]
fn invalid_codes(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> Vec<(String, ValidationProblemCode)> {
    match result {
        Err(ValidationFailure::InvalidData { problems }) => problems
            .into_iter()
            .map(|problem| (format!("{}/{}", problem.path, problem.property.unwrap_or_default()), problem.code))
            .collect(),
        other => panic!("expected invalid data, got {other:?}"),
    }
}

const RETRIES: &str = "$schema:\n  name: string(required)\n  retries: number(required; default(3))\n  count: number\n";

mod strict_and_soft {
    use super::*;

    #[test]
    fn strict_returns_the_input_borrowed_and_reports_the_default() {
        let retries = schema(RETRIES);
        let input = json!({ "name": "Ada" });

        let success = borrowed(retries.validate_strict(&input), &input);

        assert_eq!(success.defaults, [applied("retries", json!(3), "/properties/retries/default")]);
        assert_eq!(input, json!({ "name": "Ada" }), "the input is never changed");
    }

    #[test]
    fn soft_returns_an_owned_copy_with_the_default_inserted() {
        let retries = schema(RETRIES);
        let input = json!({ "name": "Ada" });

        let (value, defaults) = owned(retries.validate_soft(&input));

        assert_eq!(value, json!({ "name": "Ada", "retries": 3 }));
        assert_eq!(defaults, [applied("retries", json!(3), "/properties/retries/default")]);
    }

    #[test]
    fn soft_borrows_when_nothing_is_converted_or_inserted() {
        let retries = schema(RETRIES);
        let input = json!({ "name": "Ada", "retries": 5 });

        let success = borrowed(retries.validate_soft(&input), &input);

        assert!(success.defaults.is_empty(), "an unconditional pass relies on no default");
    }

    #[test]
    fn soft_converts_and_inserts_in_one_copy() {
        let retries = schema(RETRIES);
        let input = json!({ "name": "Ada", "count": "4" });

        let (value, defaults) = owned(retries.validate_soft(&input));

        assert_eq!(value, json!({ "name": "Ada", "count": 4, "retries": 3 }));
        assert_eq!(paths(&defaults), ["retries"], "a conversion is not a default");
        assert!(retries.validate_strict(&input).is_err(), "strict still refuses the unconverted count");
    }

    #[test]
    fn soft_converts_a_sibling_inside_a_parent_that_needs_a_default() {
        let nested = schema("$schema:\n  meta: \"{ a: number(required; default(1)), n: number }\"\n");
        let input = json!({ "meta": { "n": "4" } });

        let (value, defaults) = owned(nested.validate_soft(&input));

        assert_eq!(value, json!({ "meta": { "n": 4, "a": 1 } }));
        assert_eq!(paths(&defaults), ["meta.a"]);
    }

    #[test]
    fn a_written_null_takes_the_default_of_a_required_property() {
        let retries = schema(RETRIES);
        let input = json!({ "name": "Ada", "retries": null });

        let strict = borrowed(retries.validate_strict(&input), &input);
        assert_eq!(strict.defaults, [placed("retries", json!(3))]);
        let (value, defaults) = owned(retries.validate_soft(&input));
        assert_eq!(value, json!({ "name": "Ada", "retries": 3 }));
        assert_eq!(defaults, [placed("retries", json!(3))]);
    }

    #[test]
    fn a_soft_result_validates_again_unchanged() {
        let retries = schema("$schema:\n  retries: number(required; default(3))\n  count: number(default(1))\n  meta: '{ on: boolean(default(true)) }'\n");
        let input = json!({ "count": "4", "meta": { "on": null } });

        let (first, defaults) = owned(retries.validate_soft(&input));
        assert_eq!(first, json!({ "count": 4, "meta": { "on": true }, "retries": 3 }));
        assert_eq!(paths(&defaults), ["retries", "meta.on"]);

        let again = borrowed(retries.validate_soft(&first), &first);
        assert!(again.defaults.is_empty(), "a completed value needs no default");
        borrowed(retries.validate_strict(&first), &first);
    }

    #[test]
    fn a_required_null_admitting_property_is_filled_when_missing() {
        let nullable = schema("$schema:\n  note: [string(required; default(\"x\")), null]\n");
        let input = json!({});

        let strict = borrowed(nullable.validate_strict(&input), &input);
        assert_eq!(strict.defaults, [placed("note", json!("x"))]);
        let (value, defaults) = owned(nullable.validate_soft(&input));
        assert_eq!(value, json!({ "note": "x" }));
        assert_eq!(defaults, [placed("note", json!("x"))]);
    }

    #[test]
    fn a_required_null_admitting_property_without_a_default_needs_its_key() {
        for declaration in ["note: null(required)", "note: unknown(required)", "note: [number(required), null]"] {
            let nullable = schema(&format!("$schema:\n  {declaration}\n"));

            let codes = invalid_codes(nullable.validate_strict(&json!({})));
            assert_eq!(codes, [("/note".to_string(), ValidationProblemCode::MissingRequired)], "{declaration}");
            let written = json!({ "note": null });
            borrowed(nullable.validate_strict(&written), &written);
            borrowed(nullable.validate_soft(&written), &written);
        }
    }

    #[test]
    fn a_missing_required_property_without_a_default_is_still_missing() {
        let retries = schema(RETRIES);

        let codes = invalid_codes(retries.validate_strict(&json!({})));

        assert_eq!(codes, [("/name".to_string(), ValidationProblemCode::MissingRequired)]);
    }
}

mod nesting {
    use super::*;

    const META: &str = "$schema:\n  meta: \"{ a: number(required; default(1)), b: string }\"\n  req: \"{ a: number(required; default(1)) }(required)\"\n";

    #[test]
    fn a_child_is_filled_inside_a_present_parent() {
        let meta = schema(META);
        let input = json!({ "meta": { "b": "x" }, "req": {} });

        let (value, defaults) = owned(meta.validate_soft(&input));

        assert_eq!(value, json!({ "meta": { "b": "x", "a": 1 }, "req": { "a": 1 } }));
        assert_eq!(paths(&defaults), ["meta.a", "req.a"], "declaration order");
    }

    #[test]
    fn a_child_of_an_absent_parent_is_the_parents_question() {
        let meta = schema(META);

        let codes = invalid_codes(meta.validate_strict(&json!({})));

        assert_eq!(codes, [("/req".to_string(), ValidationProblemCode::MissingRequired)]);
        let input = json!({ "req": { "a": 2 } });
        assert!(borrowed(meta.validate_soft(&input), &input).defaults.is_empty(), "an absent optional parent stays absent");
    }

    #[test]
    fn array_items_are_filled_in_ascending_index_order() {
        let entries = schema("$schema:\n  entries: \"{ id: string(required), on: boolean(required; default(true)) }[]\"\n");
        let input = json!({ "entries": [{ "id": "a" }, { "id": "b", "on": false }, { "id": "c" }] });

        let (value, defaults) = owned(entries.validate_soft(&input));

        assert_eq!(paths(&defaults), ["entries.0.on", "entries.2.on"]);
        assert_eq!(value["entries"], json!([{ "id": "a", "on": true }, { "id": "b", "on": false }, { "id": "c", "on": true }]));
    }

    #[test]
    fn a_recursive_reference_is_filled_at_every_depth_the_value_has() {
        let catalog = schema("kind: schema\ntypes:\n  Node:\n    v: number(required; default(1))\n    next: Node\n");
        let node = catalog.select("Node").expect("Node");
        let input = json!({ "next": { "next": {} } });

        let success = borrowed(node.validate_strict(&input), &input);

        assert_eq!(paths(&success.defaults), ["v", "next.v", "next.next.v"]);
    }

    #[test]
    fn a_key_with_a_dot_or_a_slash_has_a_stable_escaped_path() {
        let odd = schema("$schema:\n  \"a.b/c\": number(required; default(1))\n");
        let input = json!({});

        let success = borrowed(odd.validate_strict(&input), &input);

        assert_eq!(success.defaults[0].path.segments(), ["a.b/c"]);
        assert_eq!(paths(&success.defaults), [r"a\.b/c"]);
    }
}

mod invalid_defaults {
    use super::*;

    #[test]
    fn a_wrong_typed_default_is_a_located_schema_error() {
        let wrong = schema("$schema:\n  n: number(required; default(\"x\"))\n");

        let error = schema_error(wrong.validate_strict(&json!({})));

        match &error {
            SchemaError::InvalidDefault { path, pointer, .. } => {
                assert_eq!(path.to_string(), "n");
                assert_eq!(pointer, "/properties/n/default");
            }
            other => panic!("expected InvalidDefault, got {other:?}"),
        }
        assert!(matches!(schema_error(wrong.validate_soft(&json!({}))), SchemaError::InvalidDefault { .. }));
    }

    #[test]
    fn a_constraint_breaking_default_is_a_schema_error() {
        let low = schema("$schema:\n  n: number(required; min(5); default(3))\n");

        let error = schema_error(low.validate_strict(&json!({})));

        assert!(matches!(error, SchemaError::InvalidDefault { .. }), "{error:?}");
    }

    #[test]
    fn an_unused_invalid_default_is_not_judged() {
        let wrong = schema("$schema:\n  n: number(required; default(\"x\"))\n");
        let input = json!({ "n": 1 });

        assert!(borrowed(wrong.validate_strict(&input), &input).defaults.is_empty());
    }

    #[test]
    fn a_default_object_missing_a_nested_required_child_is_a_schema_error() {
        let raw = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "cfg": {
                    "type": "object",
                    "properties": { "host": { "type": "string" } },
                    "required": ["host"],
                    "default": {}
                }
            },
            "required": ["cfg"]
        });
        let cfg = schema(&raw.to_string());

        let error = schema_error(cfg.validate_strict(&json!({})));

        match error {
            SchemaError::InvalidDefault { pointer, message, .. } => {
                assert_eq!(pointer, "/properties/cfg/default");
                assert!(message.contains("host"), "{message}");
            }
            other => panic!("expected InvalidDefault, got {other:?}"),
        }
    }
}

mod slots {
    use super::*;

    struct Answers(HashMap<String, Value>);

    impl SlotHost for Answers {
        fn admit(&self, _slot: &Slot) -> Result<(), SlotAdmissionError> {
            Ok(())
        }

        fn resolve(&self, request: &SlotRequest<'_>) -> Result<Value, SlotFailure> {
            Ok(self.0[&request.slot.expression].clone())
        }

        fn template_reads(&self) -> std::rc::Rc<dyn TemplateReads> {
            std::rc::Rc::new(RefuseTemplateReads)
        }

        fn is_canceled(&self) -> bool {
            false
        }
    }

    const PENDING: &str = "$schema:\n  n: number(required; default({{ d }}))\n";

    #[test]
    fn a_pending_default_slot_stays_pending() {
        let pending = schema(PENDING);

        let input = json!({});

        let result = pending.validate_soft(&input);

        match result {
            Err(ValidationFailure::Unprepared { error }) => {
                assert!(matches!(*error, SchemaError::UnresolvedSlots(_)), "{error:?}");
            }
            other => panic!("expected the unresolved slot, got {other:?}"),
        }
    }

    #[test]
    fn a_resolved_default_slot_supplies_the_default() {
        let resolved = schema(PENDING)
            .resolve_slots(&Answers(HashMap::from([("d".to_string(), json!(7))])))
            .expect("the slot resolves");
        let input = json!({});

        let success = borrowed(resolved.validate_strict(&input), &input);

        assert_eq!(success.defaults, [applied("n", json!(7), "/properties/n/default")]);
    }
}

/// The specification's presence table, row by row, for a missing key and a
/// written `null`, through both strict and soft validation.
mod presence_table {
    use super::*;

    /// How a row's value ends up: valid as written, valid with `n = 3`, or
    /// one problem code.
    enum Outcome {
        AsWritten,
        Defaulted,
        Problem(&'static str, ValidationProblemCode),
    }

    #[track_caller]
    fn check(declaration: &str, input: Value, outcome: Outcome) {
        let row = schema(&format!("$schema:\n  n: {declaration}\n"));
        match outcome {
            Outcome::AsWritten => {
                assert!(borrowed(row.validate_strict(&input), &input).defaults.is_empty(), "{declaration} {input}");
                assert!(borrowed(row.validate_soft(&input), &input).defaults.is_empty(), "{declaration} {input}");
            }
            Outcome::Defaulted => {
                let strict = borrowed(row.validate_strict(&input), &input);
                assert_eq!(strict.defaults, [placed("n", json!(3))], "{declaration} {input}");
                let (value, defaults) = owned(row.validate_soft(&input));
                assert_eq!(value, json!({ "n": 3 }), "{declaration} {input}");
                assert_eq!(defaults, [placed("n", json!(3))]);
            }
            Outcome::Problem(at, code) => {
                assert_eq!(invalid_codes(row.validate_strict(&input)), [(at.to_string(), code)], "{declaration} {input}");
                assert_eq!(invalid_codes(row.validate_soft(&input)), [(at.to_string(), code)], "{declaration} {input}");
            }
        }
    }

    #[test]
    fn an_optional_property_reads_missing_and_null_alike() {
        check("number", json!({}), Outcome::AsWritten);
        check("number", json!({ "n": null }), Outcome::AsWritten);
    }

    #[test]
    fn an_optional_default_fills_a_missing_key_and_a_null() {
        check("number(default(3))", json!({}), Outcome::Defaulted);
        check("number(default(3))", json!({ "n": null }), Outcome::Defaulted);
    }

    #[test]
    fn a_required_property_needs_its_key_and_then_its_type() {
        check("number(required)", json!({}), Outcome::Problem("/n", ValidationProblemCode::MissingRequired));
        check("number(required)", json!({ "n": null }), Outcome::Problem("/n/", ValidationProblemCode::TypeMismatch));
    }

    #[test]
    fn a_required_default_fills_a_missing_key_and_a_null() {
        check("number(required; default(3))", json!({}), Outcome::Defaulted);
        check("number(required; default(3))", json!({ "n": null }), Outcome::Defaulted);
    }

    #[test]
    fn a_required_null_admitting_property_accepts_a_written_null_only() {
        for declaration in ["null(required)", "unknown(required)", "[\"number(required)\", null]"] {
            check(declaration, json!({}), Outcome::Problem("/n", ValidationProblemCode::MissingRequired));
            check(declaration, json!({ "n": null }), Outcome::AsWritten);
        }
    }

    #[test]
    fn a_supplied_value_always_wins_over_the_default() {
        check("number(default(3))", json!({ "n": 5 }), Outcome::AsWritten);
        check("number(required; default(3))", json!({ "n": 0 }), Outcome::AsWritten);
    }
}

/// Raw JSON Schema `default` keywords apply exactly as `default(...)` does,
/// with the authored assertions and `required` arrays unchanged.
mod raw_schemas {
    use super::*;

    fn raw(properties: Value, required: &[&str]) -> SimplifiedSchema {
        let document = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": properties,
            "required": required,
        });
        schema(&document.to_string())
    }

    #[test]
    fn an_optional_raw_default_fills_a_missing_key_and_a_null() {
        let optional = raw(json!({ "n": { "type": "number", "default": 3 } }), &[]);

        for input in [json!({}), json!({ "n": null })] {
            let strict = borrowed(optional.validate_strict(&input), &input);
            assert_eq!(strict.defaults, [placed("n", json!(3))], "{input}");
            assert_eq!(owned(optional.validate_soft(&input)).0, json!({ "n": 3 }), "{input}");
        }
    }

    #[test]
    fn a_raw_null_without_a_default_is_still_judged_by_the_authored_type() {
        let typed = raw(json!({ "n": { "type": "number" } }), &[]);

        let codes = invalid_codes(typed.validate_strict(&json!({ "n": null })));

        assert_eq!(codes, [("/n/".to_string(), ValidationProblemCode::TypeMismatch)], "no null wrapper is added");
    }

    #[test]
    fn a_required_raw_default_meets_presence_and_keeps_its_assertion() {
        let required = raw(json!({ "n": { "type": "number", "default": 3 } }), &["n"]);
        let input = json!({});

        assert_eq!(borrowed(required.validate_strict(&input), &input).defaults, [placed("n", json!(3))]);
        let exported = required.to_json_schema().expect("exports");
        assert_eq!(exported["required"], json!(["n"]), "the authored required array is kept");
        assert_eq!(exported["properties"]["n"], json!({ "type": "number", "default": 3 }), "no wrapper is added");
    }

    #[test]
    fn a_raw_parent_default_is_checked_then_its_children_are_completed() {
        let nested = raw(
            json!({
                "cfg": {
                    "type": "object",
                    "properties": { "host": { "type": "string", "default": "localhost" }, "port": { "type": "number" } },
                    "default": {}
                }
            }),
            &[],
        );
        let input = json!({});

        let (value, defaults) = owned(nested.validate_soft(&input));

        assert_eq!(value, json!({ "cfg": { "host": "localhost" } }));
        assert_eq!(paths(&defaults), ["cfg", "cfg.host"], "parents before children");
        borrowed(nested.validate_soft(&value), &value);
    }
}

/// Raw JSON Schema cells beyond the presence table: optional null
/// defaults, authored null admission, invalid and conflicting defaults
/// through `Prepared`, and `oneOf` exclusivity over the completed value.
mod raw_schema_inputs {
    use std::sync::Arc;

    use simplified_schema::Prepared;

    use super::*;

    fn raw(document: Value) -> Value {
        let mut document = document;
        document["$schema"] = json!("https://json-schema.org/draft/2020-12/schema");
        document
    }

    fn prepared(document: Value) -> Result<Prepared, SchemaError> {
        let dir = tempfile::tempdir().expect("temp dir");
        Prepared::new(Arc::new(raw(document)), None, biscuit_file::FileResolutionContext::new(dir.path()))
    }

    #[test]
    fn an_optional_raw_null_default_is_a_no_op() {
        let nullable = prepared(json!({
            "type": "object",
            "properties": { "n": { "type": ["number", "null"], "default": null } }
        }))
        .expect("prepares");

        for input in [json!({}), json!({ "n": null })] {
            assert!(borrowed(nullable.validate_strict(&input), &input).defaults.is_empty(), "{input}");
            assert!(borrowed(nullable.validate_soft(&input), &input).defaults.is_empty(), "{input}");
        }
    }

    #[test]
    fn authored_null_admission_and_required_stay_as_written() {
        let nullable = prepared(json!({
            "type": "object",
            "properties": { "n": { "type": ["number", "null"] } },
            "required": ["n"]
        }))
        .expect("prepares");

        let written = json!({ "n": null });
        borrowed(nullable.validate_strict(&written), &written);
        assert_eq!(
            invalid_codes(nullable.validate_strict(&json!({}))),
            [("/n".to_string(), ValidationProblemCode::MissingRequired)]
        );
    }

    #[test]
    fn a_used_invalid_raw_default_is_unprepared_at_its_pointer() {
        let wrong = prepared(json!({
            "type": "object",
            "properties": { "n": { "type": "number", "default": "x" } }
        }))
        .expect("an unused default is not judged");

        let supplied = json!({ "n": 1 });
        borrowed(wrong.validate_strict(&supplied), &supplied);
        for input in [json!({}), json!({ "n": null })] {
            for result in [wrong.validate_strict(&input), wrong.validate_soft(&input)] {
                match schema_error(result) {
                    SchemaError::InvalidDefault { path, pointer, .. } => {
                        assert_eq!((path.to_string().as_str(), pointer.as_str()), ("n", "/properties/n/default"), "{input}");
                    }
                    other => panic!("{input}: expected InvalidDefault, got {other:?}"),
                }
            }
        }
    }

    #[test]
    fn conflicting_arm_defaults_fail_prepared_new_before_any_document() {
        for keyword in ["anyOf", "oneOf"] {
            let error = prepared(json!({
                keyword: [
                    { "type": "object", "properties": { "n": { "type": "number", "default": 1 } } },
                    { "type": "object", "properties": { "n": { "type": "number", "default": 2 } } }
                ]
            }))
            .err()
            .unwrap_or_else(|| panic!("{keyword}: prepared"));

            match error {
                SchemaError::ConflictingDefaults { property, declarations, .. } => {
                    assert_eq!(property, "n", "{keyword}");
                    assert_eq!(
                        declarations,
                        [format!("/{keyword}/0/properties/n/default"), format!("/{keyword}/1/properties/n/default")],
                        "{keyword}"
                    );
                }
                other => panic!("{keyword}: expected ConflictingDefaults, got {other:?}"),
            }
        }
    }

    #[test]
    fn one_of_exclusivity_judges_the_completed_value() {
        // Without the default `{}` matches only the first arm; completed to
        // `{ t: 3 }` it matches both, which `oneOf` forbids.
        let exclusive = prepared(json!({
            "oneOf": [
                { "type": "object", "properties": { "t": { "type": "number", "default": 3 } } },
                { "type": "object", "required": ["t"] }
            ]
        }))
        .expect("prepares");

        for input in [json!({}), json!({ "t": null })] {
            let result = exclusive.validate_strict(&input);
            assert!(matches!(result, Err(ValidationFailure::InvalidData { .. })), "{input}: {result:?}");
        }
        let second_arm_only = json!({ "t": "x" });
        borrowed(exclusive.validate_strict(&second_arm_only), &second_arm_only);
    }
}

/// `null` defaults terminate and falsey values are values.
mod null_and_falsey {
    use super::*;

    #[test]
    fn an_optional_null_default_does_nothing() {
        let optional = schema("$schema:\n  n: number(default(null))\n");

        for input in [json!({}), json!({ "n": null })] {
            assert!(borrowed(optional.validate_strict(&input), &input).defaults.is_empty(), "{input}");
            assert!(borrowed(optional.validate_soft(&input), &input).defaults.is_empty(), "{input}");
        }
    }

    #[test]
    fn a_required_null_default_inserts_only_a_missing_key() {
        let nullable = schema("$schema:\n  n: unknown(required; default(null))\n");
        let missing = json!({});

        assert_eq!(borrowed(nullable.validate_strict(&missing), &missing).defaults, [placed("n", Value::Null)]);
        let (value, defaults) = owned(nullable.validate_soft(&missing));
        assert_eq!(value, json!({ "n": null }));
        assert_eq!(defaults, [placed("n", Value::Null)]);

        let written = json!({ "n": null });
        assert!(borrowed(nullable.validate_soft(&written), &written).defaults.is_empty(), "a written null is kept as is");
    }

    #[test]
    fn a_required_number_with_a_null_default_is_an_invalid_default_when_used() {
        let wrong = schema("$schema:\n  n: number(required; default(null))\n");

        let error = schema_error(wrong.validate_strict(&json!({})));

        assert!(matches!(&error, SchemaError::InvalidDefault { pointer, .. } if pointer == "/properties/n/default"), "{error:?}");
        let written = json!({ "n": null });
        assert_eq!(
            invalid_codes(wrong.validate_strict(&written)),
            [("/n/".to_string(), ValidationProblemCode::TypeMismatch)],
            "a written null is not replaced by a null default"
        );
    }

    #[test]
    fn falsey_values_are_never_replaced() {
        let defaults = schema(
            "$schema:\n  n: number(default(3))\n  b: boolean(default(true))\n  s: string(default(\"x\"))\n  l: string[](default([\"x\"]))\n  o: '{ a: number }(default({\"a\": 1}))'\n",
        );
        let input = json!({ "n": 0, "b": false, "s": "", "l": [], "o": {} });

        let success = borrowed(defaults.validate_soft(&input), &input);

        assert!(success.defaults.is_empty(), "{:?}", success.defaults);
    }
}

mod optional_invalid_defaults {
    use super::*;

    #[test]
    fn an_optional_invalid_default_is_an_error_only_when_used() {
        let wrong = schema("$schema:\n  n: number(default(\"x\"))\n");
        let supplied = json!({ "n": 1 });

        borrowed(wrong.validate_strict(&supplied), &supplied);
        for input in [json!({}), json!({ "n": null })] {
            let error = schema_error(wrong.validate_soft(&input));
            match error {
                SchemaError::InvalidDefault { path, pointer, .. } => {
                    assert_eq!(path.to_string(), "n");
                    assert_eq!(pointer, "/properties/n/default");
                }
                other => panic!("expected InvalidDefault, got {other:?}"),
            }
        }
    }

    #[test]
    fn a_parent_default_missing_a_required_child_is_not_repaired() {
        let wrong = schema("$schema:\n  cfg: '{ host: string(required; default(\"h\")) }(default({}))'\n");

        let error = schema_error(wrong.validate_soft(&json!({})));

        assert!(matches!(&error, SchemaError::InvalidDefault { pointer, .. } if pointer == "/properties/cfg/default"), "{error:?}");
    }
}

mod parents {
    use super::*;

    const CFG: &str = "$schema:\n  cfg: '{ on: boolean(default(true)), n: number }'\n";

    #[test]
    fn a_missing_or_null_parent_without_a_default_stays_as_written() {
        let cfg = schema(CFG);

        for input in [json!({}), json!({ "cfg": null })] {
            assert!(borrowed(cfg.validate_soft(&input), &input).defaults.is_empty(), "{input}");
        }
    }

    #[test]
    fn an_existing_parent_has_its_missing_and_null_children_filled() {
        let cfg = schema(CFG);

        for input in [json!({ "cfg": {} }), json!({ "cfg": { "on": null } })] {
            let (value, defaults) = owned(cfg.validate_soft(&input));
            assert_eq!(value, json!({ "cfg": { "on": true } }), "{input}");
            assert_eq!(paths(&defaults), ["cfg.on"]);
        }
    }

    #[test]
    fn a_parent_default_has_its_optional_children_completed() {
        let cfg = schema("$schema:\n  cfg: '{ on: boolean(default(true)) }(default({}))'\n");
        let input = json!({ "cfg": null });

        let strict = borrowed(cfg.validate_strict(&input), &input);
        assert_eq!(strict.defaults, [placed("cfg", json!({})), placed("cfg.on", json!(true))]);
        let (value, _) = owned(cfg.validate_soft(&input));
        assert_eq!(value, json!({ "cfg": { "on": true } }));
    }

    #[test]
    fn list_items_have_their_null_children_filled_by_index() {
        let entries = schema("$schema:\n  entries: \"{ id: string(required), on: boolean(default(true)) }[]\"\n");
        let input = json!({ "entries": [{ "id": "a", "on": null }, { "id": "b", "on": false }] });

        let (value, defaults) = owned(entries.validate_soft(&input));

        assert_eq!(paths(&defaults), ["entries.0.on"]);
        assert_eq!(value["entries"], json!([{ "id": "a", "on": true }, { "id": "b", "on": false }]));
    }

    #[test]
    fn a_local_named_type_carries_its_defaults() {
        let typed = schema("types:\n  Cfg:\n    on: boolean(default(true))\n$schema:\n  cfg: Cfg\n");
        let input = json!({ "cfg": {} });

        let (value, defaults) = owned(typed.validate_soft(&input));

        assert_eq!(value, json!({ "cfg": { "on": true } }));
        assert_eq!(paths(&defaults), ["cfg.on"]);
    }

    #[test]
    fn positional_items_are_never_inserted() {
        let tuple = schema("$schema:\n  pair: 'tuple([number, number(default(2))?])'\n");
        let input = json!({ "pair": [1] });

        assert!(borrowed(tuple.validate_soft(&input), &input).defaults.is_empty(), "an omitted trailing item stays omitted");
    }
}

/// Pending values supplied by a host are not null values to replace.
mod pending_templates {
    use std::sync::Arc;

    use simplified_schema::coerce::NestedTemplates;
    use simplified_schema::coercion::PathSegment;
    use simplified_schema::defaults::{DefaultPlan, prepare_defaults};
    use simplified_schema::validate::ValidatorCache;

    use super::*;

    #[test]
    fn a_value_below_a_template_is_never_filled() {
        let lowered = schema("$schema:\n  cfg: '{ on: boolean(default(true)) }'\n  n: number(default(3))\n")
            .to_json_schema()
            .expect("lowers");
        let plan = DefaultPlan::compile(Arc::clone(&lowered), &ValidatorCache::new()).expect("compiles");
        let templates = NestedTemplates::from_template_paths(vec![vec![PathSegment::Key("cfg".into())]]);
        let input = json!({ "cfg": {} });

        let prepared = prepare_defaults(&plan, Cow::Borrowed(&input), &templates).expect("prepares");

        assert_eq!(paths(&prepared.defaults), ["n"], "the template's subtree is pending");
        assert_eq!(prepared.candidate.as_ref(), &json!({ "cfg": {}, "n": 3 }));
    }
}

mod unions {
    use super::*;

    #[test]
    fn equal_defaults_on_several_arms_count_as_one() {
        let same = schema("$schema:\n  - a: number(required; default(1))\n  - a: number(required; default(1))\n    c: string\n");
        let input = json!({});

        let success = borrowed(same.validate_strict(&input), &input);

        assert_eq!(success.defaults, [applied("a", json!(1), "/anyOf/0/properties/a/default")]);
    }

    #[test]
    fn different_defaults_on_root_arms_fail_preparation_even_when_unused() {
        let text = "$schema:\n  - a: number(default(1))\n    b: string(required)\n  - a: number(default(2))\n    c: string(required)\n";
        let conflicting = schema(text);

        let exported = conflicting.to_json_schema();
        assert!(exported.is_err(), "export refuses the schema");
        let input = json!({ "a": 5, "b": "x" });
        match &*shared_schema_error(conflicting.validate_strict(&input)) {
            SchemaError::ConflictingDefaults { pointer, property, declarations } => {
                assert_eq!(pointer.as_str(), "/anyOf/0");
                assert_eq!(property.as_str(), "a");
                assert_eq!(*declarations, ["/anyOf/0/properties/a/default", "/anyOf/1/properties/a/default"]);
            }
            other => panic!("expected ConflictingDefaults, got {other:?}"),
        }
    }

    #[test]
    fn different_defaults_on_nested_object_arms_are_located_at_their_property() {
        let shape = schema("$schema:\n  shape: \"{ a: number(required; default(1)) } | { a: number(required; default(2)) }\"\n");

        match &*shared_schema_error(shape.validate_strict(&json!({}))) {
            SchemaError::ConflictingDefaults { pointer, property, .. } => {
                assert_eq!(pointer.as_str(), "/properties/shape/anyOf/1", "the first object arm after the null wrapper");
                assert_eq!(property.as_str(), "a");
            }
            other => panic!("expected ConflictingDefaults, got {other:?}"),
        }
    }

    #[test]
    fn different_defaults_in_a_raw_schema_fail_preparation() {
        let raw = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "anyOf": [
                { "type": "object", "properties": { "n": { "type": "number", "default": 1 } } },
                { "type": "object", "properties": { "n": { "type": "number", "default": 2 } } }
            ]
        });

        let error = shared_schema_error(schema(&raw.to_string()).validate_soft(&json!({ "n": 4 })));

        assert!(matches!(*error, SchemaError::ConflictingDefaults { .. }), "{error:?}");
    }

    #[test]
    fn a_default_on_one_arm_fills_whichever_arm_matches() {
        let either = schema("$schema:\n  - timeout: number(default(3))\n  - timeout: string\n");

        let missing = json!({});
        assert_eq!(borrowed(either.validate_strict(&missing), &missing).defaults, [placed("timeout", json!(3))]);
        let written = json!({ "timeout": "soon" });
        assert!(borrowed(either.validate_strict(&written), &written).defaults.is_empty(), "already valid, still prepared");
    }

    /// A nested default on one arm is checked only against the definitions
    /// along that arm: the other arm's required child is an alternative,
    /// not a constraint the default must meet.
    #[test]
    fn a_nested_default_on_one_arm_is_not_checked_against_the_other_arm() {
        let either = schema(
            "types:\n  Loose:\n    o: \"{ e: boolean }(default({}))\"\n  Strict:\n    o: \"{ e: boolean(required) }\"\n$schema:\n  - x: Loose\n  - x: Strict\n",
        );
        let input = json!({ "x": {} });

        assert_eq!(borrowed(either.validate_strict(&input), &input).defaults, [placed("x.o", json!({}))]);
        let (value, _) = owned(either.validate_soft(&input));
        assert_eq!(value, json!({ "x": { "o": {} } }));
    }

    /// Definitions joined by `allOf` inside one arm all apply to a default
    /// declared on that arm.
    #[test]
    fn a_default_is_checked_against_every_definition_along_its_arm() {
        let raw = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "anyOf": [
                {
                    "allOf": [
                        { "type": "object", "properties": { "o": { "type": "object", "default": {} } } },
                        { "type": "object", "properties": { "o": { "required": ["e"], "properties": { "e": { "default": true } } } } }
                    ]
                },
                { "type": "object", "properties": { "o": { "type": "string" } } }
            ]
        });
        let split = schema(&raw.to_string());

        for result in [split.validate_strict(&json!({})), split.validate_soft(&json!({}))] {
            let error = shared_schema_error(result);
            assert!(matches!(&*error, SchemaError::InvalidDefault { path, .. } if path.to_string() == "o"), "{error:?}");
        }
    }

    #[test]
    fn a_default_forbidden_by_every_arm_is_an_ordinary_validation_failure() {
        let exclusive = schema("$schema:\n  - timeout: number(default(3))\n    retries: null\n  - retries: number\n    timeout: null\n");

        let defaulted = json!({});
        assert_eq!(borrowed(exclusive.validate_strict(&defaulted), &defaulted).defaults, [placed("timeout", json!(3))]);
        let retries = json!({ "retries": 5 });
        let result = exclusive.validate_strict(&retries);
        assert!(matches!(result, Err(ValidationFailure::InvalidData { .. })), "the exclusion still judges the completed value: {result:?}");
    }

    #[test]
    fn a_closed_arm_rejects_a_default_another_arm_supplies() {
        let raw = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "anyOf": [
                {
                    "type": "object",
                    "properties": { "mode": { "const": "a" }, "extra": { "type": "string", "default": "x" } },
                    "required": ["mode"]
                },
                {
                    "type": "object",
                    "properties": { "mode": { "const": "b" } },
                    "required": ["mode"],
                    "additionalProperties": false
                }
            ]
        });
        let closed = schema(&raw.to_string());

        let open_arm = json!({ "mode": "a" });
        assert_eq!(borrowed(closed.validate_strict(&open_arm), &open_arm).defaults, [placed("extra", json!("x"))]);
        let closed_arm = json!({ "mode": "b" });
        let result = closed.validate_strict(&closed_arm);
        assert!(matches!(result, Err(ValidationFailure::InvalidData { .. })), "the default is not dropped: {result:?}");
        let written = json!({ "mode": "b", "extra": null });
        assert!(closed.validate_strict(&written).is_err(), "a written null is replaced too");
    }

    #[test]
    fn defaults_of_every_arm_apply_and_the_discriminant_still_selects() {
        let documents = schema(
            "$schema:\n  - kind: literal(post; required)\n    title: string(required; default(\"untitled\"))\n  - kind: literal(page; required)\n    slug: string(required; default(\"home\"))\n",
        );
        let input = json!({ "kind": "page" });

        let (value, defaults) = owned(documents.validate_soft(&input));

        assert_eq!(value, json!({ "kind": "page", "title": "untitled", "slug": "home" }), "no default is chosen by arm");
        assert_eq!(paths(&defaults), ["title", "slug"]);
        let wrong = json!({ "kind": "essay" });
        assert!(documents.validate_strict(&wrong).is_err(), "the discriminant still judges the completed value");
    }

    /// A root union whose root requires `name` beside arms that discriminate
    /// on `kind`, with `extra` root assertions.
    fn root_with_siblings(extra: Value) -> SimplifiedSchema {
        let mut raw = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": { "name": { "type": "string", "default": "anonymous" } },
            "required": ["name"],
            "anyOf": [
                { "properties": { "kind": { "const": "a" } }, "required": ["kind"] },
                { "properties": { "kind": { "const": "b" } }, "required": ["kind"] }
            ]
        });
        if let (Some(raw), Some(extra)) = (raw.as_object_mut(), extra.as_object()) {
            raw.extend(extra.clone());
        }
        schema(&raw.to_string())
    }

    #[test]
    fn a_root_sibling_default_is_filled() {
        let rooted = root_with_siblings(json!({}));
        let input = json!({ "kind": "a" });

        let success = borrowed(rooted.validate_strict(&input), &input);

        assert_eq!(success.defaults, [applied("name", json!("anonymous"), "/properties/name/default")]);
    }

    #[test]
    fn root_sibling_assertions_judge_the_completed_value() {
        let capped = root_with_siblings(json!({ "maxProperties": 1 }));
        let input = json!({ "kind": "a" });

        let result = capped.validate_strict(&input);

        assert!(matches!(result, Err(ValidationFailure::InvalidData { .. })), "{result:?}");
    }
}

/// Supplied values that are not null: each cell is one mutation of a
/// valid `{ n: … }`. Strict refuses every one at its JSON Pointer; soft
/// accepts only what the conversion engine converts, never fills a default
/// over a written value, and its result validates again unchanged. The input
/// is never edited.
mod input_shapes {
    use super::*;

    const NUMBER: &str = "number(required; default(3))";
    const LIST: &str = "number[](required; default([1]))";
    const OBJECT: &str = "'{ a: number }(required; default({\"a\": 1}))'";
    const ITEMS: &str = "'{ id: string(required), on: boolean(default(true)) }[](required)'";

    /// What soft validation makes of a cell.
    enum Soft {
        /// Refused, with problems at these pointers.
        Refused(&'static [&'static str]),
        /// Accepted as an owned copy whose `n` is this value, reporting these
        /// default paths.
        Converted(Value, &'static [&'static str]),
    }

    #[track_caller]
    fn pointers(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> Vec<String> {
        match result {
            Err(ValidationFailure::InvalidData { problems }) => {
                let mut pointers: Vec<String> = problems.into_iter().map(|problem| problem.path).collect();
                pointers.sort();
                pointers
            }
            other => panic!("expected invalid data, got {other:?}"),
        }
    }

    fn cells() -> Vec<(&'static str, Value, &'static [&'static str], Soft)> {
        use Soft::*;
        vec![
            // Native wrong types and non-coercible text.
            (NUMBER, json!(true), &["/n"], Refused(&["/n"])),
            (NUMBER, json!([1]), &["/n"], Refused(&["/n"])),
            (NUMBER, json!({}), &["/n"], Refused(&["/n"])),
            (NUMBER, json!("abc"), &["/n"], Refused(&["/n"])),
            (NUMBER, json!(""), &["/n"], Refused(&["/n"])),
            (NUMBER, json!("  "), &["/n"], Refused(&["/n"])),
            (NUMBER, json!("4"), &["/n"], Converted(json!(4), &[])),
            (OBJECT, json!("x"), &["/n"], Refused(&["/n"])),
            (OBJECT, json!({ "a": "x" }), &["/n/a"], Refused(&["/n/a"])),
            (OBJECT, json!({ "a": "1" }), &["/n/a"], Converted(json!({ "a": 1 }), &[])),
            // A scalar for a list, then one and every invalid item; a null
            // item is a value of the wrong type, never a missing default.
            (LIST, json!("1"), &["/n"], Refused(&["/n"])),
            (LIST, json!([1, "x"]), &["/n/1"], Refused(&["/n/1"])),
            (LIST, json!(["x", "y"]), &["/n/0", "/n/1"], Refused(&["/n/0", "/n/1"])),
            (LIST, json!([null]), &["/n/0"], Refused(&["/n/0"])),
            (LIST, json!(["1", 2]), &["/n/0"], Converted(json!([1, 2]), &[])),
            // Object items: an item's defaults fill only beside convertible
            // siblings, and an invalid item blocks the whole copy.
            (ITEMS, json!([{ "id": "a" }, { "id": {} }]), &["/n/1/id"], Refused(&["/n/1/id"])),
            (ITEMS, json!([{ "id": [] }, { "id": {} }]), &["/n/0/id", "/n/1/id"], Refused(&["/n/0/id", "/n/1/id"])),
            (ITEMS, json!([{ "id": "a", "on": "maybe" }, { "id": "b" }]), &["/n/0/on"], Refused(&["/n/0/on"])),
            (
                ITEMS,
                json!([{ "id": "a", "on": "yes" }, { "id": "b" }]),
                &["/n/0/on"],
                Converted(json!([{ "id": "a", "on": true }, { "id": "b", "on": true }]), &["n.1.on"]),
            ),
        ]
    }

    #[test]
    fn supplied_values_are_judged_at_their_pointer_and_never_replaced_by_the_default() {
        for (declaration, value, strict, soft) in cells() {
            let row = schema(&format!("$schema:\n  n: {declaration}\n"));
            let input = json!({ "n": value });
            let written = input.clone();
            let label = format!("{declaration} {input}");

            assert_eq!(pointers(row.validate_strict(&input)), strict, "{label}: strict");
            match soft {
                Soft::Refused(at) => assert_eq!(pointers(row.validate_soft(&input)), at, "{label}: soft"),
                Soft::Converted(expected, defaults) => {
                    let (value, reported) = owned(row.validate_soft(&input));
                    assert_eq!(value, json!({ "n": expected }), "{label}");
                    assert_eq!(paths(&reported), defaults, "{label}");
                    let again = borrowed(row.validate_soft(&value), &value);
                    assert!(again.defaults.is_empty(), "{label}: the soft result needs nothing more");
                    borrowed(row.validate_strict(&value), &value);
                }
            }
            assert_eq!(input, written, "{label}: the input is never changed");
        }
    }

    #[test]
    fn required_falsey_values_are_values() {
        let falsey = schema(
            "$schema:\n  n: number(required; default(3))\n  b: boolean(required; default(true))\n  s: string(required; default(\"x\"))\n  l: string[](required; default([\"x\"]))\n  o: '{ a: number }(required; default({\"a\": 1}))'\n",
        );
        let input = json!({ "n": 0, "b": false, "s": "", "l": [], "o": {} });

        assert!(borrowed(falsey.validate_strict(&input), &input).defaults.is_empty());
        assert!(borrowed(falsey.validate_soft(&input), &input).defaults.is_empty());
    }

    #[test]
    fn a_failure_elsewhere_still_counts_the_default_toward_presence() {
        let retries = schema(RETRIES);
        let input = json!({ "name": [] });

        let problems = invalid_codes(retries.validate_strict(&input));

        assert_eq!(problems, [("/name/".to_string(), ValidationProblemCode::TypeMismatch)], "no missing `retries`");
        assert_eq!(invalid_codes(retries.validate_soft(&input)), problems);
        assert_eq!(input, json!({ "name": [] }));
    }
}

/// A plain JSON Schema validator reads the exported schema's `required`
/// without applying defaults; this library prepares them first.
#[test]
fn a_plain_validator_still_rejects_an_absent_defaulted_required_property() {
    let retries = schema("$schema:\n  retries: number(required; default(3))\n");
    let exported = retries.to_json_schema().expect("exports");
    let plain = jsonschema::validator_for(&exported).expect("a plain validator compiles the export");
    let input = json!({});

    assert!(!plain.is_valid(&input), "JSON Schema's `default` fills nothing");
    assert_eq!(borrowed(retries.validate_strict(&input), &input).defaults, [placed("retries", json!(3))]);
}

/// An existing object value is completed under every schema JSON Schema
/// applies to it: its literal property, each `patternProperties` entry whose
/// pattern matches its key, or `additionalProperties` when neither covers
/// the key. Each row keeps the child contract and input fixed and changes
/// only the container that selects the child.
mod object_children {
    use super::*;

    const CHILD: &str = "\"{ retries: number(required; default(3)) }\"";

    /// A schema whose root object selects `CHILD` through `selector`, with
    /// any `extra` lines after it.
    fn selected(selector: &str, extra: &str) -> SimplifiedSchema {
        schema(&format!("$schema:\n  \"{selector}\": {CHILD}\n{extra}"))
    }

    /// Strict accepts `input` borrowed and reports the one default at
    /// `path`, declared at `pointer`; soft inserts it.
    #[track_caller]
    fn completes(subject: &SimplifiedSchema, input: &Value, path: &str, pointer: &str, completed: &Value) {
        let success = borrowed(subject.validate_strict(input), input);
        assert_eq!(success.defaults, [applied(path, json!(3), pointer)]);

        let (value, defaults) = owned(subject.validate_soft(input));
        assert_eq!(&value, completed);
        assert_eq!(defaults, [applied(path, json!(3), pointer)]);
    }

    macro_rules! completes_under {
        ($($name:ident: $subject:expr, $input:expr, $path:expr, $pointer:expr, $completed:expr;)+) => {$(
            #[test]
            fn $name() {
                completes(&$subject, &$input, $path, $pointer, &$completed);
            }
        )+};
    }

    completes_under! {
        a_prefix_pattern_key: selected("<starting::x->", ""), json!({ "x-build": {} }),
            "x-build.retries", r"/patternProperties/^x\-/properties/retries/default",
            json!({ "x-build": { "retries": 3 } });
        a_suffix_pattern_key: selected("<ending::-build>", ""), json!({ "x-build": {} }),
            "x-build.retries", r"/patternProperties/\-build$/properties/retries/default",
            json!({ "x-build": { "retries": 3 } });
        a_regex_pattern_key: selected("<pattern::^x-.*$>", ""), json!({ "x-build": {} }),
            "x-build.retries", "/patternProperties/^x-.*$/properties/retries/default",
            json!({ "x-build": { "retries": 3 } });
        a_catch_all_key_beside_an_unrelated_pattern: selected("<string>", "  \"<starting::other->\": string\n"),
            json!({ "x-build": {} }), "x-build.retries", "/additionalProperties/properties/retries/default",
            json!({ "x-build": { "retries": 3 } });
        a_literal_key: selected("x-build", ""), json!({ "x-build": {} }),
            "x-build.retries", "/properties/x-build/anyOf/1/properties/retries/default",
            json!({ "x-build": { "retries": 3 } });
        a_catch_all_key_without_patterns: selected("<string>", ""), json!({ "x-build": {} }),
            "x-build.retries", "/additionalProperties/properties/retries/default",
            json!({ "x-build": { "retries": 3 } });
        a_literal_key_beside_an_unrelated_pattern: selected("x-build", "  \"<starting::other->\": string\n"),
            json!({ "x-build": {} }), "x-build.retries", "/properties/x-build/anyOf/1/properties/retries/default",
            json!({ "x-build": { "retries": 3 } });
        an_array_item: schema(&format!("$schema:\n  entries: {}\n", CHILD.replace("}\"", "}[]\""))),
            json!({ "entries": [{}] }), "entries.0.retries", "/properties/entries/anyOf/1/items/properties/retries/default",
            json!({ "entries": [{ "retries": 3 }] });
        a_tuple_item: schema(r#"{ "$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object",
            "properties": { "entries": { "type": "array", "prefixItems": [
                { "type": "object", "properties": { "retries": { "type": "number", "default": 3 } }, "required": ["retries"] }
            ] } } }"#),
            json!({ "entries": [{}] }), "entries.0.retries", "/properties/entries/prefixItems/0/properties/retries/default",
            json!({ "entries": [{ "retries": 3 }] });
        a_local_reference: schema(r##"{ "$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object",
            "$defs": { "Job": { "type": "object", "properties": { "retries": { "type": "number", "default": 3 } }, "required": ["retries"] } },
            "properties": { "x-build": { "$ref": "#/$defs/Job" } } }"##),
            json!({ "x-build": {} }), "x-build.retries", "/$defs/Job/properties/retries/default",
            json!({ "x-build": { "retries": 3 } });
        an_all_of_member: schema(r#"{ "$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object",
            "properties": { "x-build": { "allOf": [
                { "type": "object", "properties": { "retries": { "type": "number", "default": 3 } }, "required": ["retries"] }
            ] } } }"#),
            json!({ "x-build": {} }), "x-build.retries", "/properties/x-build/allOf/0/properties/retries/default",
            json!({ "x-build": { "retries": 3 } });
    }

    #[test]
    fn an_explicit_value_under_a_pattern_key_is_kept() {
        let prefixed = selected("<starting::x->", "");
        let input = json!({ "x-build": { "retries": 5 } });

        assert!(borrowed(prefixed.validate_strict(&input), &input).defaults.is_empty());
        assert!(borrowed(prefixed.validate_soft(&input), &input).defaults.is_empty());
    }

    #[test]
    fn a_value_breaking_the_pattern_child_contract_is_still_invalid() {
        let prefixed = selected("<starting::x->", "");
        let input = json!({ "x-build": { "retries": "many" } });

        let result = prefixed.validate_strict(&input);

        assert!(matches!(result, Err(ValidationFailure::InvalidData { .. })), "{result:?}");
    }

    #[test]
    fn a_key_a_literal_and_a_pattern_both_cover_takes_both_schemas_defaults() {
        let both = schema(
            r#"{ "$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object",
            "properties": { "x-build": { "type": "object",
                "properties": { "retries": { "type": "number", "default": 3 } }, "required": ["retries"] } },
            "patternProperties": { "^x-": { "type": "object",
                "properties": { "timeout": { "type": "number", "default": 60 } }, "required": ["timeout"] } } }"#,
        );
        let input = json!({ "x-build": {} });

        let (value, defaults) = owned(both.validate_soft(&input));

        assert_eq!(value, json!({ "x-build": { "retries": 3, "timeout": 60 } }));
        assert_eq!(paths(&defaults), ["x-build.retries", "x-build.timeout"]);
        assert_eq!(defaults[1].pointer, "/patternProperties/^x-/properties/timeout/default");
    }

    #[test]
    fn every_pattern_a_key_matches_applies() {
        let two = schema(
            r#"{ "$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object",
            "patternProperties": {
                "^x-": { "type": "object", "properties": { "retries": { "type": "number", "default": 3 } }, "required": ["retries"] },
                "-build$": { "type": "object", "properties": { "timeout": { "type": "number", "default": 60 } }, "required": ["timeout"] } } }"#,
        );
        let input = json!({ "x-build": {}, "x-test": {} });

        let (value, defaults) = owned(two.validate_soft(&input));

        assert_eq!(value, json!({ "x-build": { "retries": 3, "timeout": 60 }, "x-test": { "retries": 3 } }));
        assert_eq!(paths(&defaults), ["x-build.retries", "x-build.timeout", "x-test.retries"]);
    }

    #[test]
    fn the_catch_all_does_not_judge_a_key_a_pattern_covers() {
        let mixed = selected("<string>", "  \"<starting::other->\": \"{ n: number }\"\n");
        let input = json!({ "other-job": {}, "x-build": {} });

        let (value, defaults) = owned(mixed.validate_soft(&input));

        assert_eq!(value, json!({ "other-job": {}, "x-build": { "retries": 3 } }));
        assert_eq!(paths(&defaults), ["x-build.retries"]);
    }

    /// The walk never meets such a pattern on its own: the schema's
    /// validator refuses it first, with the same error the walk would give.
    #[test]
    fn a_pattern_that_does_not_compile_is_a_schema_error() {
        let broken = schema(
            r#"{ "$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object",
            "patternProperties": { "(": { "type": "object", "properties": { "n": { "type": "number", "default": 1 } }, "required": ["n"] } } }"#,
        );

        let input = json!({ "(": {} });

        let result = broken.validate_strict(&input);

        match result {
            Err(ValidationFailure::Unprepared { error }) => {
                assert!(matches!(error.as_ref(), SchemaError::BuildValidator { .. }), "{error:?}");
            }
            other => panic!("expected the schema refused, got {other:?}"),
        }
    }
}

#[test]
fn exported_json_schema_keeps_the_required_assertion() {
    let retries = schema(RETRIES);

    let exported = retries.to_json_schema().expect("exports");

    let required: Vec<&str> = exported["required"].as_array().expect("required").iter().filter_map(Value::as_str).collect();
    assert!(required.contains(&"retries"), "{required:?}");
    assert_eq!(exported["properties"]["retries"]["default"], json!(3), "the default stays an annotation");
}

/// A check the structural default check cannot run (an eager file's
/// existence) still judges the completed value; its problem belongs to the
/// schema default, never to an authored position.
#[cfg(feature = "file")]
mod eager_file_defaults {
    use std::path::Path;

    use simplified_schema::{SchemaSource, ValidationProblem};

    use super::*;

    const EAGER_DEFAULT: &str = "$schema:\n  doc: file(required; eager; default(./note.md))\n";

    /// The schema `text`, written to `schema.yaml` in `dir` and bound to a
    /// context there, so its file values resolve from `dir`.
    fn bound(dir: &Path, text: &str) -> SimplifiedSchema {
        let path = dir.join("schema.yaml");
        std::fs::write(&path, text).expect("schema written");
        SimplifiedSchema::try_from_yaml(SchemaSource::from(path.as_path()).bound(biscuit_file::FileResolutionContext::new(dir)))
            .expect("schema file")
    }

    #[track_caller]
    fn invalid(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> Vec<ValidationProblem> {
        match result {
            Err(ValidationFailure::InvalidData { problems }) => problems,
            other => panic!("expected invalid data, got {other:?}"),
        }
    }

    #[track_caller]
    fn assert_attributed_to_the_default(problems: &[ValidationProblem]) {
        let [problem] = problems else { panic!("one problem expected, got {problems:?}") };
        assert_eq!(problem.code, ValidationProblemCode::InvalidFileReference);
        assert_eq!(problem.path, "/doc");
        let default = problem.default.as_ref().expect("the problem names its default");
        assert_eq!(default.path.to_string(), "doc");
        assert_eq!(default.pointer, "/properties/doc/default");
        assert_eq!(default.value, json!("./note.md"));
        assert!(problem.message.contains("schema default at schema location `/properties/doc/default`"), "{}", problem.message);
        assert_eq!((problem.line, problem.column), (None, None), "no authored text holds a default");
    }

    #[test]
    fn strict_attributes_a_missing_eager_default_target_to_the_default() {
        let dir = tempfile::tempdir().expect("temp dir");
        let eager = bound(dir.path(), EAGER_DEFAULT);

        let problems = invalid(eager.validate_strict(&json!({})));

        assert_attributed_to_the_default(&problems);
    }

    #[test]
    fn soft_attributes_a_missing_eager_default_target_to_the_default() {
        let dir = tempfile::tempdir().expect("temp dir");
        let eager = bound(dir.path(), EAGER_DEFAULT);

        let problems = invalid(eager.validate_soft(&json!({})));

        assert_attributed_to_the_default(&problems);
    }

    #[test]
    fn an_eager_default_whose_target_exists_is_accepted_and_reported() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("note.md"), "note\n").expect("target written");
        let eager = bound(dir.path(), EAGER_DEFAULT);
        let input = json!({});

        let success = borrowed(eager.validate_strict(&input), &input);
        let (value, defaults) = owned(eager.validate_soft(&input));

        assert_eq!(success.defaults, [applied("doc", json!("./note.md"), "/properties/doc/default")]);
        assert_eq!(value, json!({ "doc": "./note.md" }));
        assert_eq!(paths(&defaults), ["doc"]);
    }

    #[test]
    fn a_lazy_file_default_is_accepted_whether_or_not_its_target_exists() {
        let dir = tempfile::tempdir().expect("temp dir");
        let lazy = bound(dir.path(), "$schema:\n  doc: file(required; default(./note.md))\n");
        let input = json!({});

        assert_eq!(paths(&borrowed(lazy.validate_strict(&input), &input).defaults), ["doc"]);
        assert_eq!(owned(lazy.validate_soft(&input)).1.len(), 1);
        std::fs::write(dir.path().join("note.md"), "note\n").expect("target written");
        assert_eq!(paths(&borrowed(lazy.validate_strict(&input), &input).defaults), ["doc"]);
    }

    #[test]
    fn a_nested_eager_default_is_attributed_inside_its_present_parent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let nested = bound(dir.path(), "$schema:\n  meta:\n    doc: file(required; eager; default(./note.md))\n");

        let problems = invalid(nested.validate_strict(&json!({ "meta": {} })));

        let [problem] = problems.as_slice() else { panic!("one problem expected, got {problems:?}") };
        assert_eq!(problem.path, "/meta/doc");
        let default = problem.default.as_ref().expect("attributed");
        // An optional parent lowers to `null | object`; the pointer names the
        // object arm that declares the default.
        assert_eq!(default.pointer, "/properties/meta/anyOf/1/properties/doc/default");
    }

    #[test]
    fn an_authored_missing_eager_file_stays_an_ordinary_problem() {
        let dir = tempfile::tempdir().expect("temp dir");
        let eager = bound(dir.path(), EAGER_DEFAULT);

        let problems = invalid(eager.validate_strict(&json!({ "doc": "./absent.md" })));

        let [problem] = problems.as_slice() else { panic!("one problem expected, got {problems:?}") };
        assert_eq!(problem.path, "/doc");
        assert!(problem.default.is_none(), "an authored value is not a default's");
        assert!(!problem.message.contains("schema default"), "{}", problem.message);
    }

    /// Expansion inlines a `Name@file` type without a trace of its file;
    /// resolution records the file declaring each default the type brings
    /// in, so a problem with it names the file to repair. A default the
    /// schema declares itself, or one inside a list's items, is not listed.
    #[test]
    fn resolution_records_the_file_declaring_each_imported_default() {
        use simplified_schema::resolve::{DefaultFiles, resolve_yaml_schema};
        use simplified_schema::embedded::ShippedSources;

        let dir = tempfile::tempdir().expect("temp dir");
        let types = dir.path().join("types.yaml");
        std::fs::write(
            &types,
            "kind: schema\ntypes:\n  Leaf: string(required; default(a))\n  Thing:\n    leaf: Leaf\n    own: number(required; default(1))\n",
        )
        .expect("types written");
        std::fs::write(
            dir.path().join("schema.yaml"),
            "$schema:\n  doc: Thing@./types.yaml\n  mine: string(required; default(m))\n",
        )
        .expect("schema written");
        let types = simplified_schema::resolve::canonical_path(&types);
        let resolve = |yaml: &str| {
            let declaration: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("test YAML parses");
            let context = biscuit_file::FileResolutionContext::new(dir.path());
            resolve_yaml_schema(&declaration, dir.path(), &context, ShippedSources::NONE).expect("schema resolves")
        };
        let key = |path: &[&str]| path.iter().map(ToString::to_string).collect::<Vec<_>>();
        let expected: DefaultFiles =
            [(key(&["doc", "leaf"]), types.clone()), (key(&["doc", "own"]), types.clone())].into_iter().collect();

        let inline = resolve("doc: Thing@./types.yaml\nlocal: string(required; default(x))\nitems: Thing[]@./types.yaml\n");
        let referenced = resolve("./schema.yaml");

        assert_eq!(inline.default_files, expected);
        assert_eq!(referenced.default_files, expected, "the referenced file's own `mine` default is its origin's");
    }
}
