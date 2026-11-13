//! A required property with a `default(...)` passes when absent: strict and
//! soft validation judge the value completed with the default and report it
//! in `ValidationSuccess::defaults`. Strict returns the input borrowed; soft
//! returns an owned copy with the defaults inserted.
//!
//! Boundary: the crate's public API (`SimplifiedSchema`), in-process.

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
    fn an_optional_default_is_never_inserted() {
        let optional = schema("$schema:\n  n: number(default(3))\n");
        let input = json!({});

        assert!(borrowed(optional.validate_strict(&input), &input).defaults.is_empty());
        assert!(borrowed(optional.validate_soft(&input), &input).defaults.is_empty());
    }

    #[test]
    fn an_explicit_null_is_present_and_judged_by_the_declared_type() {
        let retries = schema(RETRIES);
        let input = json!({ "name": "Ada", "retries": null });

        let codes = invalid_codes(retries.validate_strict(&input));

        assert_eq!(codes, [("/retries/".to_string(), ValidationProblemCode::TypeMismatch)]);
        assert!(retries.validate_soft(&input).is_err(), "soft does not default a null either");
    }

    #[test]
    fn a_null_admitting_property_stays_optional_and_is_not_filled() {
        let nullable = schema("$schema:\n  note: [string(required; default(\"x\")), null]\n");
        let input = json!({});

        let success = borrowed(nullable.validate_soft(&input), &input);

        assert!(success.defaults.is_empty());
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

mod unions {
    use super::*;

    #[test]
    fn root_arms_that_complete_the_value_differently_are_ambiguous() {
        let either = schema("$schema:\n  - a: number(required; default(1))\n  - b: string(required; default(\"z\"))\n");

        let error = schema_error(either.validate_strict(&json!({})));

        match error {
            SchemaError::AmbiguousDefaults { path, pointer, arms } => {
                assert_eq!(path, None);
                assert_eq!(pointer, "/anyOf");
                assert_eq!(arms, [0, 1]);
            }
            other => panic!("expected AmbiguousDefaults, got {other:?}"),
        }
    }

    #[test]
    fn a_value_one_arm_accepts_as_written_needs_no_default() {
        let either = schema("$schema:\n  - a: number(required; default(1))\n  - b: string(required; default(\"z\"))\n");
        let input = json!({ "b": "y" });

        assert!(borrowed(either.validate_soft(&input), &input).defaults.is_empty());
    }

    #[test]
    fn equal_candidates_from_several_arms_count_as_one() {
        let same = schema("$schema:\n  - a: number(required; default(1))\n  - a: number(required; default(1))\n    c: string\n");
        let input = json!({});

        let success = borrowed(same.validate_strict(&input), &input);

        assert_eq!(success.defaults, [applied("a", json!(1))]);
    }

    #[test]
    fn a_discriminant_selects_the_arm_whose_defaults_apply() {
        let documents = schema(
            "$schema:\n  - kind: literal(post; required)\n    title: string(required; default(\"untitled\"))\n  - kind: literal(page; required)\n    slug: string(required; default(\"home\"))\n",
        );
        let input = json!({ "kind": "page" });

        let (value, defaults) = owned(documents.validate_soft(&input));

        assert_eq!(value, json!({ "kind": "page", "slug": "home" }));
        assert_eq!(paths(&defaults), ["slug"]);
    }

    #[test]
    fn a_nested_union_ambiguity_is_located_at_its_property() {
        let shape = schema("$schema:\n  shape: \"{ a: number(required; default(1)) } | { b: number(required; default(2)) }\"\n");

        let error = schema_error(shape.validate_strict(&json!({ "shape": {} })));

        match error {
            SchemaError::AmbiguousDefaults { path, pointer, .. } => {
                assert_eq!(path.map(|path| path.to_string()).as_deref(), Some("shape"));
                assert_eq!(pointer, "/properties/shape/anyOf");
            }
            other => panic!("expected AmbiguousDefaults, got {other:?}"),
        }
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
    fn a_root_sibling_default_is_filled_for_the_arm_that_accepts() {
        let rooted = root_with_siblings(json!({}));
        let input = json!({ "kind": "a" });

        let success = borrowed(rooted.validate_strict(&input), &input);

        assert_eq!(success.defaults, [applied("name", json!("anonymous"))]);
    }

    #[test]
    fn root_sibling_assertions_judge_every_arms_candidate() {
        let capped = root_with_siblings(json!({ "maxProperties": 1 }));
        let input = json!({ "kind": "a" });

        let result = capped.validate_strict(&input);

        assert!(matches!(result, Err(ValidationFailure::InvalidData { .. })), "{result:?}");
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
