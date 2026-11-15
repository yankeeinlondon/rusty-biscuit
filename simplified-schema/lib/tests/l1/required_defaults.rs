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

fn applied(path: &str, value: Value) -> AppliedDefault {
    AppliedDefault { path: PropertyPath::parse(path).expect("path"), value }
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

        assert_eq!(success.defaults, [applied("retries", json!(3))]);
        assert_eq!(input, json!({ "name": "Ada" }), "the input is never changed");
    }

    #[test]
    fn soft_returns_an_owned_copy_with_the_default_inserted() {
        let retries = schema(RETRIES);
        let input = json!({ "name": "Ada" });

        let (value, defaults) = owned(retries.validate_soft(&input));

        assert_eq!(value, json!({ "name": "Ada", "retries": 3 }));
        assert_eq!(defaults, [applied("retries", json!(3))]);
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
        assert_eq!(strict.defaults, [applied("retries", json!(3))]);
        let (value, defaults) = owned(retries.validate_soft(&input));
        assert_eq!(value, json!({ "name": "Ada", "retries": 3 }));
        assert_eq!(defaults, [applied("retries", json!(3))]);
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
        assert_eq!(strict.defaults, [applied("note", json!("x"))]);
        let (value, defaults) = owned(nullable.validate_soft(&input));
        assert_eq!(value, json!({ "note": "x" }));
        assert_eq!(defaults, [applied("note", json!("x"))]);
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

        assert_eq!(success.defaults, [applied("n", json!(7))]);
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
                assert_eq!(strict.defaults, [applied("n", json!(3))], "{declaration} {input}");
                let (value, defaults) = owned(row.validate_soft(&input));
                assert_eq!(value, json!({ "n": 3 }), "{declaration} {input}");
                assert_eq!(defaults, [applied("n", json!(3))]);
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
            assert_eq!(strict.defaults, [applied("n", json!(3))], "{input}");
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

        assert_eq!(borrowed(required.validate_strict(&input), &input).defaults, [applied("n", json!(3))]);
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

        assert_eq!(borrowed(nullable.validate_strict(&missing), &missing).defaults, [applied("n", Value::Null)]);
        let (value, defaults) = owned(nullable.validate_soft(&missing));
        assert_eq!(value, json!({ "n": null }));
        assert_eq!(defaults, [applied("n", Value::Null)]);

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
        assert_eq!(strict.defaults, [applied("cfg", json!({})), applied("cfg.on", json!(true))]);
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

        assert_eq!(success.defaults, [applied("a", json!(1))]);
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
        assert_eq!(borrowed(either.validate_strict(&missing), &missing).defaults, [applied("timeout", json!(3))]);
        let written = json!({ "timeout": "soon" });
        assert!(borrowed(either.validate_strict(&written), &written).defaults.is_empty(), "already valid, still prepared");
    }

    #[test]
    fn a_default_forbidden_by_every_arm_is_an_ordinary_validation_failure() {
        let exclusive = schema("$schema:\n  - timeout: number(default(3))\n    retries: null\n  - retries: number\n    timeout: null\n");

        let defaulted = json!({});
        assert_eq!(borrowed(exclusive.validate_strict(&defaulted), &defaulted).defaults, [applied("timeout", json!(3))]);
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
        assert_eq!(borrowed(closed.validate_strict(&open_arm), &open_arm).defaults, [applied("extra", json!("x"))]);
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

        assert_eq!(success.defaults, [applied("name", json!("anonymous"))]);
    }

    #[test]
    fn root_sibling_assertions_judge_the_completed_value() {
        let capped = root_with_siblings(json!({ "maxProperties": 1 }));
        let input = json!({ "kind": "a" });

        let result = capped.validate_strict(&input);

        assert!(matches!(result, Err(ValidationFailure::InvalidData { .. })), "{result:?}");
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
    assert_eq!(borrowed(retries.validate_strict(&input), &input).defaults, [applied("retries", json!(3))]);
}

#[test]
fn exported_json_schema_keeps_the_required_assertion() {
    let retries = schema(RETRIES);

    let exported = retries.to_json_schema().expect("exports");

    let required: Vec<&str> = exported["required"].as_array().expect("required").iter().filter_map(Value::as_str).collect();
    assert!(required.contains(&"retries"), "{required:?}");
    assert_eq!(exported["properties"]["retries"]["default"], json!(3), "the default stays an annotation");
}
