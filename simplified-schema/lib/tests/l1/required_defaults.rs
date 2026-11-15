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

/// The default at `path`, declared at `pointer` in the lowered schema, as
/// this crate reports it (no host-supplied origin).
fn applied(path: &str, value: Value, pointer: &str) -> AppliedDefault {
    AppliedDefault { path: PropertyPath::parse(path).expect("path"), value, pointer: pointer.to_string(), origin: None }
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

        assert_eq!(success.defaults, [applied("n", json!(7), "/properties/n/default")]);
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

    const EITHER: &str = "$schema:\n  - a: number(required; default(1))\n  - b: string(required; default(\"z\"))\n";
    const NESTED_EITHER: &str =
        "$schema:\n  shape: \"{ a: number(required; default(1)) } | { b: string(required; default(z)) }\"\n";

    /// The arms of an [`SchemaError::AmbiguousDefaults`], after asserting
    /// where it is located.
    #[track_caller]
    fn ambiguity_at(error: SchemaError, expected_path: Option<&str>, expected_pointer: &str) -> Vec<usize> {
        match error {
            SchemaError::AmbiguousDefaults { path, pointer, arms } => {
                assert_eq!(path.map(|path| path.to_string()).as_deref(), expected_path);
                assert_eq!(pointer, expected_pointer);
                arms
            }
            other => panic!("expected AmbiguousDefaults, got {other:?}"),
        }
    }

    #[test]
    fn a_value_one_arm_accepts_as_written_is_ambiguous_with_an_arm_that_completes_it() {
        let either = schema(EITHER);
        let input = json!({ "b": "y" });

        let strict = schema_error(either.validate_strict(&input));
        let soft = schema_error(either.validate_soft(&input));

        assert_eq!(ambiguity_at(strict, None, "/anyOf"), [0, 1]);
        assert_eq!(ambiguity_at(soft, None, "/anyOf"), [0, 1]);
    }

    /// `shape`'s alternatives as raw JSON Schema objects that, unlike
    /// SimplifiedSchema's always-closed nested objects, admit each other's
    /// keys.
    fn open_nested_either() -> SimplifiedSchema {
        schema(
            &json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "shape": {
                        "anyOf": [
                            { "type": "object", "properties": { "a": { "type": "number", "default": 1 } }, "required": ["a"] },
                            { "type": "object", "properties": { "b": { "type": "string", "default": "z" } }, "required": ["b"] }
                        ]
                    }
                }
            })
            .to_string(),
        )
    }

    #[test]
    fn a_nested_value_one_open_arm_accepts_as_written_is_ambiguous_at_its_property() {
        let nested = open_nested_either();
        let input = json!({ "shape": { "b": "y" } });

        let strict = schema_error(nested.validate_strict(&input));
        let soft = schema_error(nested.validate_soft(&input));

        assert_eq!(ambiguity_at(strict, Some("shape"), "/properties/shape/anyOf"), [0, 1]);
        assert_eq!(ambiguity_at(soft, Some("shape"), "/properties/shape/anyOf"), [0, 1]);
    }

    #[test]
    fn a_closed_nested_arm_completed_with_a_default_cannot_take_the_other_arms_keys() {
        let nested = schema(NESTED_EITHER);
        let input = json!({ "shape": { "b": "y" } });

        assert!(borrowed(nested.validate_strict(&input), &input).defaults.is_empty());
        assert!(borrowed(nested.validate_soft(&input), &input).defaults.is_empty());
    }

    #[test]
    fn an_empty_nested_value_with_differing_candidates_is_ambiguous_in_both_modes() {
        let nested = schema(NESTED_EITHER);
        let input = json!({ "shape": {} });

        let strict = schema_error(nested.validate_strict(&input));
        let soft = schema_error(nested.validate_soft(&input));

        // Arm 0 is the `null` an optional property admits.
        assert_eq!(ambiguity_at(strict, Some("shape"), "/properties/shape/anyOf"), [1, 2]);
        assert_eq!(ambiguity_at(soft, Some("shape"), "/properties/shape/anyOf"), [1, 2]);
    }

    #[test]
    fn arms_without_defaults_accept_a_value_as_written() {
        let plain = schema("$schema:\n  - a: number(required)\n  - b: string(required)\n");
        let input = json!({ "b": "y" });

        assert!(borrowed(plain.validate_strict(&input), &input).defaults.is_empty());
        assert!(borrowed(plain.validate_soft(&input), &input).defaults.is_empty());
    }

    #[test]
    fn an_arm_that_cannot_hold_the_value_does_not_compete_with_it() {
        let text_or_shape = schema("$schema:\n  shape: \"string | { a: number(required; default(1)) }\"\n");
        let input = json!({ "shape": "round" });

        assert!(borrowed(text_or_shape.validate_strict(&input), &input).defaults.is_empty());
        assert!(borrowed(text_or_shape.validate_soft(&input), &input).defaults.is_empty());
    }

    #[test]
    fn a_distinguishing_literal_keeps_a_complete_value_unambiguous() {
        let documents = schema(
            "$schema:\n  - kind: literal(post; required)\n    title: string(required; default(\"untitled\"))\n  - kind: literal(page; required)\n    slug: string(required; default(\"home\"))\n",
        );
        let input = json!({ "kind": "page", "slug": "about" });

        assert!(borrowed(documents.validate_strict(&input), &input).defaults.is_empty());
        assert!(borrowed(documents.validate_soft(&input), &input).defaults.is_empty());
    }

    #[test]
    fn equal_candidates_from_several_arms_count_as_one() {
        let same = schema("$schema:\n  - a: number(required; default(1))\n  - a: number(required; default(1))\n    c: string\n");
        let input = json!({});

        let success = borrowed(same.validate_strict(&input), &input);

        assert_eq!(success.defaults, [applied("a", json!(1), "/anyOf/0/properties/a/default")]);
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

        assert_eq!(success.defaults, [applied("name", json!("anonymous"), "/properties/name/default")]);
    }

    #[test]
    fn root_sibling_assertions_judge_every_arms_candidate() {
        let capped = root_with_siblings(json!({ "maxProperties": 1 }));
        let input = json!({ "kind": "a" });

        let result = capped.validate_strict(&input);

        assert!(matches!(result, Err(ValidationFailure::InvalidData { .. })), "{result:?}");
    }
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
