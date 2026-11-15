//! Defaults beneath key-pattern selectors: a child default applies inside an
//! existing object whichever way its key is selected (a literal property, a
//! `<string>` catch-all, `<starting::…>`, `<ending::…>`, `<pattern::…>`, or a
//! raw `patternProperties` / `additionalProperties`), and conflicting
//! defaults beneath a pattern are refused when the schema is prepared.
//!
//! Boundary: the crate's public API (`SimplifiedSchema`, `Prepared`),
//! in-process. F1: every input is in-memory schema text or JSON.

use std::borrow::Cow;
use std::sync::Arc;

use serde_json::{Value, json};
use simplified_schema::{AppliedDefault, Prepared, PropertyPath, SchemaError, SimplifiedSchema, ValidationFailure, ValidationSuccess};

const DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";

fn schema(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).expect("schema text")
}

fn applied(path: &str, value: Value) -> AppliedDefault {
    AppliedDefault { path: PropertyPath::parse(path).expect("path"), value }
}

#[track_caller]
fn borrowed<'a>(result: Result<ValidationSuccess<'a>, ValidationFailure>, input: &Value) -> Vec<AppliedDefault> {
    match result {
        Ok(success @ ValidationSuccess { value: Cow::Borrowed(_), .. }) => {
            assert!(std::ptr::eq(success.value.as_ref(), input), "strict returns the caller's own input");
            success.defaults
        }
        other => panic!("expected the input accepted borrowed, got {other:?}"),
    }
}

#[track_caller]
fn owned(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> (Value, Vec<AppliedDefault>) {
    match result {
        Ok(success @ ValidationSuccess { value: Cow::Owned(_), .. }) => (success.value.into_owned(), success.defaults),
        other => panic!("expected an owned completed copy, got {other:?}"),
    }
}

#[track_caller]
fn unprepared(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> Arc<SchemaError> {
    match result {
        Err(ValidationFailure::Unprepared { error }) => error,
        other => panic!("expected an unprepared schema, got {other:?}"),
    }
}

/// `enabled` defaults to `true` beneath the key `x-settings`, for an absent
/// child and a written `null`, in both modes; `false` is kept; the soft
/// result is stable; an absent parent is not created.
#[track_caller]
fn completes_beneath(settings: &SimplifiedSchema) {
    let expected = [applied("x-settings.enabled", json!(true))];
    for input in [json!({ "x-settings": {} }), json!({ "x-settings": { "enabled": null } })] {
        assert_eq!(borrowed(settings.validate_strict(&input), &input), expected, "strict, input {input}");
        let (value, defaults) = owned(settings.validate_soft(&input));
        assert_eq!(value, json!({ "x-settings": { "enabled": true } }), "soft, input {input}");
        assert_eq!(defaults, expected);
        let again = settings.validate_soft(&value).expect("the completed copy validates");
        assert!(matches!(again.value, Cow::Borrowed(_)) && again.defaults.is_empty(), "a second soft pass changes nothing");
    }
    let kept = json!({ "x-settings": { "enabled": false } });
    assert!(borrowed(settings.validate_soft(&kept), &kept).is_empty(), "false is a value");
    let empty = json!({});
    assert!(borrowed(settings.validate_soft(&empty), &empty).is_empty(), "an absent parent is not created");
}

fn simplified(key: &str) -> SimplifiedSchema {
    schema(&format!("types:\n  Settings:\n    enabled: boolean(default(true))\n$schema:\n  '{key}': Settings\n"))
}

mod selectors {
    use super::*;

    #[test]
    fn a_literal_property_child_is_completed() {
        completes_beneath(&simplified("x-settings"));
    }

    #[test]
    fn a_catch_all_child_is_completed() {
        completes_beneath(&simplified("<string>"));
    }

    #[test]
    fn a_prefix_pattern_child_is_completed() {
        completes_beneath(&simplified("<starting::x->"));
    }

    #[test]
    fn a_suffix_pattern_child_is_completed() {
        completes_beneath(&simplified("<ending::-settings>"));
    }

    #[test]
    fn a_regular_expression_pattern_child_is_completed() {
        completes_beneath(&simplified("<pattern::^x->"));
    }

    #[test]
    fn a_literal_key_is_not_completed_by_a_sibling_pattern() {
        let text = "types:\n  Settings:\n    enabled: boolean(default(true))\n\
                    $schema:\n  x-fixed: \"{ other: number }\"\n  '<starting::x->': Settings\n";
        let input = json!({ "x-fixed": {}, "x-more": {} });

        let (value, defaults) = owned(schema(text).validate_soft(&input));

        assert_eq!(value, json!({ "x-fixed": {}, "x-more": { "enabled": true } }), "literal keys win");
        assert_eq!(defaults, [applied("x-more.enabled", json!(true))]);
    }
}

mod raw_schemas {
    use super::*;

    fn settings() -> Value {
        json!({ "type": "object", "properties": { "enabled": { "type": "boolean", "default": true } } })
    }

    #[test]
    fn a_pattern_properties_child_is_prepared_before_it_is_judged() {
        let raw = json!({ "$schema": DRAFT, "type": "object", "patternProperties": { "^x-": settings() } });

        completes_beneath(&schema(&raw.to_string()));
    }

    #[test]
    fn additional_properties_beside_a_pattern_completes_only_unmatched_keys() {
        let catch_all = json!({ "type": "object", "properties": { "n": { "type": "number", "default": 2 } } });
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "patternProperties": { "^x-": settings() },
            "additionalProperties": catch_all,
        });
        let input = json!({ "x-settings": {}, "other": {} });

        let (value, defaults) = owned(schema(&raw.to_string()).validate_soft(&input));

        assert_eq!(value, json!({ "x-settings": { "enabled": true }, "other": { "n": 2 } }));
        assert_eq!(defaults, [applied("x-settings.enabled", json!(true)), applied("other.n", json!(2))]);
    }

    #[test]
    fn a_declared_property_matching_a_pattern_takes_both_schemas_defaults() {
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "properties": { "x-settings": { "type": "object", "properties": { "level": { "type": "number", "default": 1 } } } },
            "patternProperties": { "^x-": settings() },
        });
        let input = json!({ "x-settings": {} });

        let (value, _) = owned(schema(&raw.to_string()).validate_soft(&input));

        assert_eq!(value, json!({ "x-settings": { "level": 1, "enabled": true } }));
    }

    #[test]
    fn overlapping_patterns_with_equal_defaults_report_once() {
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "patternProperties": { "^x-": settings(), "-settings$": settings() },
        });
        let input = json!({ "x-settings": {} });

        let (_, defaults) = owned(schema(&raw.to_string()).validate_soft(&input));

        assert_eq!(defaults, [applied("x-settings.enabled", json!(true))]);
    }

    #[test]
    fn overlapping_patterns_with_different_defaults_conflict_for_a_key_both_match() {
        let other = json!({ "type": "object", "properties": { "enabled": { "type": "boolean", "default": false } } });
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "patternProperties": { "^x-": settings(), "-settings$": other },
        });
        let overlapping = schema(&raw.to_string());

        let only_one = json!({ "x-a": {}, "b-settings": {} });
        let (value, _) = owned(overlapping.validate_soft(&only_one));
        assert_eq!(value, json!({ "x-a": { "enabled": true }, "b-settings": { "enabled": false } }));
        let both = json!({ "x-settings": {} });
        let error = unprepared(overlapping.validate_strict(&both));
        assert!(matches!(&*error, SchemaError::ConflictingDefaults { property, .. } if property == "enabled"), "{error:?}");
    }
}

mod conflicts {
    use super::*;

    const SHAPE: &str = "types:\n  Shape:\n    - n: number(default(1))\n      a: string(required)\n    - n: number(default(2))\n      b: string(required)\n";

    #[track_caller]
    fn refused_when_prepared(key: &str) {
        let conflicting = schema(&format!("{SHAPE}$schema:\n  '{key}': Shape\n"));

        assert!(conflicting.to_json_schema().is_err(), "export refuses {key}");
        let error = unprepared(conflicting.validate_strict(&json!({})));
        assert!(matches!(&*error, SchemaError::ConflictingDefaults { property, .. } if property == "n"), "{key}: {error:?}");
    }

    #[test]
    fn a_union_conflict_beneath_a_pattern_is_refused_when_prepared() {
        refused_when_prepared("<starting::x->");
        refused_when_prepared("<pattern::^x->");
    }

    #[test]
    fn a_union_conflict_beneath_a_literal_or_catch_all_is_refused_when_prepared() {
        refused_when_prepared("x-shape");
        refused_when_prepared("<string>");
    }

    #[test]
    fn a_raw_union_conflict_beneath_pattern_properties_fails_prepared_new() {
        let arm = |n: u32| json!({ "type": "object", "properties": { "n": { "type": "number", "default": n } } });
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "patternProperties": { "^x-": { "anyOf": [arm(1), arm(2)] } },
        });
        let dir = tempfile::tempdir().expect("temp dir");

        let error = Prepared::new(Arc::new(raw), None, biscuit_file::FileResolutionContext::new(dir.path())).unwrap_err();

        assert!(matches!(&error, SchemaError::ConflictingDefaults { property, .. } if property == "n"), "{error:?}");
    }
}

/// Every SimplifiedSchema key selector that selects `x-settings`, as a
/// `$schema` key.
const LITERAL: &str = "x-settings";
const CATCH_ALL: &str = "<string>";
const PREFIX: &str = "<starting::x->";
const SUFFIX: &str = "<ending::-settings>";
const REGEX: &str = "<pattern::^x->";

/// `x-settings` declared through `key` with the value type `declaration`.
fn entry(key: &str, declaration: &str) -> SimplifiedSchema {
    schema(&format!("$schema:\n  '{key}': {declaration}\n"))
}

/// The existing `null` entry `x-settings` takes `expected`, reporting
/// `reports`: strict borrows the input, soft writes the completed copy,
/// and a second soft pass changes nothing.
#[track_caller]
fn null_entry_completes(settings: &SimplifiedSchema, expected: Value, reports: &[AppliedDefault]) {
    let input = json!({ "x-settings": null });
    assert_eq!(borrowed(settings.validate_strict(&input), &input), reports, "strict");
    let (value, defaults) = owned(settings.validate_soft(&input));
    assert_eq!(value, json!({ "x-settings": expected }), "soft");
    assert_eq!(defaults, reports, "soft reports");
    let again = settings.validate_soft(&value).expect("the completed copy validates");
    assert!(matches!(again.value, Cow::Borrowed(_)) && again.defaults.is_empty(), "a second soft pass changes nothing");
}

/// The input as `settings` completes it in one soft pass, reported the same
/// by strict validation, with the completed copy stable.
#[track_caller]
fn completes_in_one_pass(settings: &SimplifiedSchema, input: &Value, expected: &Value, reports: &[AppliedDefault]) {
    assert_eq!(borrowed(settings.validate_strict(input), input), reports, "strict");
    let (value, defaults) = owned(settings.validate_soft(input));
    assert_eq!(&value, expected, "soft");
    assert_eq!(defaults, reports, "soft reports");
    let again = settings.validate_soft(&value).expect("the completed copy validates");
    assert!(matches!(again.value, Cow::Borrowed(_)) && again.defaults.is_empty(), "a second soft pass changes nothing");
}

#[track_caller]
fn refused(result: Result<ValidationSuccess<'_>, ValidationFailure>, what: &str) {
    assert!(matches!(result, Err(ValidationFailure::InvalidData { .. })), "{what}: expected invalid data, got {result:?}");
}

/// The selected entry's own default: a `null` scalar entry takes it.
fn scalar_entry_takes_its_default(key: &str) {
    null_entry_completes(&entry(key, "number(default(3))"), json!(3), &[applied("x-settings", json!(3))]);
}

/// A `null` object entry takes its object default and, in the same pass, the
/// default of that object's child; an existing `{}` gets only the child.
fn object_entry_takes_its_default_and_child(key: &str) {
    let settings = entry(key, "\"{ enabled: boolean(default(true)) }(default({}))\"");
    let reports = [applied("x-settings", json!({})), applied("x-settings.enabled", json!(true))];
    null_entry_completes(&settings, json!({ "enabled": true }), &reports);
    let existing = json!({ "x-settings": {} });
    let expected = json!({ "x-settings": { "enabled": true } });
    completes_in_one_pass(&settings, &existing, &expected, &[applied("x-settings.enabled", json!(true))]);
}

/// A default the `null` entry would use is checked against the entry's
/// definition first.
fn used_invalid_default_is_refused(key: &str) {
    let settings = entry(key, "number(default('bad'))");
    let input = json!({ "x-settings": null });
    for (mode, result) in [("strict", settings.validate_strict(&input)), ("soft", settings.validate_soft(&input))] {
        let error = unprepared(result);
        assert!(
            matches!(&*error, SchemaError::InvalidDefault { path, .. } if path.to_string() == "x-settings"),
            "{mode}: {error:?}"
        );
    }
}

/// Every non-null shape of the defaulted scalar entry is judged as written,
/// never replaced by its default.
fn other_scalar_shapes_are_kept(key: &str) {
    let settings = entry(key, "number(default(3))");
    let zero = json!({ "x-settings": 0 });
    assert!(borrowed(settings.validate_strict(&zero), &zero).is_empty(), "0 is a value, strictly");
    assert!(borrowed(settings.validate_soft(&zero), &zero).is_empty(), "0 is a value, softly");
    for written in [json!(false), json!(""), json!([]), json!({})] {
        let input = json!({ "x-settings": written });
        refused(settings.validate_strict(&input), &format!("strict {input}"));
        refused(settings.validate_soft(&input), &format!("soft {input}"));
    }
    let text = json!({ "x-settings": "3" });
    refused(settings.validate_strict(&text), "strict \"3\"");
    let (value, defaults) = owned(settings.validate_soft(&text));
    assert_eq!(value, json!({ "x-settings": 3 }), "\"3\" converts softly");
    assert!(defaults.is_empty(), "a conversion is not a default");
}

/// An absent entry: a dictionary selector never creates the key; the
/// literal control inserts its named key.
fn an_absent_entry(key: &str) {
    let settings = entry(key, "number(default(3))");
    let empty = json!({});
    if key == LITERAL {
        let (value, defaults) = owned(settings.validate_soft(&empty));
        assert_eq!(value, json!({ "x-settings": 3 }));
        assert_eq!(defaults, [applied("x-settings", json!(3))]);
    } else {
        assert!(borrowed(settings.validate_strict(&empty), &empty).is_empty(), "strict");
        assert!(borrowed(settings.validate_soft(&empty), &empty).is_empty(), "an empty dictionary stays empty");
        let object = entry(key, "\"{ enabled: boolean(default(true)) }(default({}))\"");
        assert!(borrowed(object.validate_soft(&empty), &empty).is_empty(), "an absent object entry stays absent");
    }
}

/// One `#[test]` per selector for each case function above.
macro_rules! per_selector {
    ($($case:ident),* $(,)?) => {$(
        mod $case {
            use super::*;

            #[test]
            fn literal() {
                super::$case(LITERAL);
            }

            #[test]
            fn catch_all() {
                super::$case(CATCH_ALL);
            }

            #[test]
            fn prefix() {
                super::$case(PREFIX);
            }

            #[test]
            fn suffix() {
                super::$case(SUFFIX);
            }

            #[test]
            fn regular_expression() {
                super::$case(REGEX);
            }
        }
    )*};
}

/// A selected entry's own default, whichever selector chooses the key.
mod entry_defaults {
    use super::*;

    per_selector!(
        scalar_entry_takes_its_default,
        object_entry_takes_its_default_and_child,
        used_invalid_default_is_refused,
        other_scalar_shapes_are_kept,
        an_absent_entry,
    );

    fn number() -> Value {
        json!({ "type": "number", "default": 3 })
    }

    #[test]
    fn a_raw_pattern_properties_entry_takes_its_default() {
        let raw = json!({ "$schema": DRAFT, "type": "object", "patternProperties": { "^x-": number() } });

        null_entry_completes(&schema(&raw.to_string()), json!(3), &[applied("x-settings", json!(3))]);
    }

    #[test]
    fn a_raw_pattern_properties_entry_named_in_required_takes_its_default() {
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "required": ["x-settings"],
            "patternProperties": { "^x-": number() },
        });

        null_entry_completes(&schema(&raw.to_string()), json!(3), &[applied("x-settings", json!(3))]);
    }

    /// A name `required` lists is a named key, not one a pattern invents:
    /// the schemas selecting it supply its default when it is missing.
    #[test]
    fn a_missing_required_name_takes_the_default_of_the_schemas_selecting_it() {
        for selector in [json!({ "patternProperties": { "^x-": number() } }), json!({ "additionalProperties": number() })] {
            let mut raw = json!({ "$schema": DRAFT, "type": "object", "required": ["x-settings"] });
            raw.as_object_mut().unwrap().extend(selector.as_object().unwrap().clone());
            let settings = schema(&raw.to_string());

            completes_in_one_pass(&settings, &json!({}), &json!({ "x-settings": 3 }), &[applied("x-settings", json!(3))]);
        }
    }

    #[test]
    fn a_missing_name_no_schema_requires_stays_absent_beside_a_pattern() {
        let raw = json!({ "$schema": DRAFT, "type": "object", "patternProperties": { "^x-": number() } });
        let settings = schema(&raw.to_string());
        let empty = json!({});

        assert!(borrowed(settings.validate_soft(&empty), &empty).is_empty());
    }

    #[test]
    fn a_raw_additional_properties_entry_takes_its_default() {
        let raw = json!({ "$schema": DRAFT, "type": "object", "additionalProperties": number() });

        null_entry_completes(&schema(&raw.to_string()), json!(3), &[applied("x-settings", json!(3))]);
    }

    #[test]
    fn a_raw_literal_matching_a_pattern_takes_the_patterns_default() {
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "properties": { "x-settings": { "type": "number" } },
            "patternProperties": { "^x-": number() },
        });

        null_entry_completes(&schema(&raw.to_string()), json!(3), &[applied("x-settings", json!(3))]);
    }
}

/// Several schemas selecting one key describe it as one property: a parent
/// default from one and a child default from another complete together.
mod combined_selectors {
    use super::*;

    fn parent() -> Value {
        json!({
            "type": "object",
            "properties": {
                "options": { "type": "object", "properties": { "enabled": { "type": "boolean" } }, "default": {} },
            },
        })
    }

    fn nested() -> Value {
        json!({
            "type": "object",
            "properties": {
                "options": { "type": "object", "properties": { "enabled": { "type": "boolean", "default": true } } },
            },
        })
    }

    #[track_caller]
    fn options_complete_in_one_pass(settings: &SimplifiedSchema) {
        let reports = [applied("x-settings.options", json!({})), applied("x-settings.options.enabled", json!(true))];
        let expected = json!({ "x-settings": { "options": { "enabled": true } } });
        completes_in_one_pass(settings, &json!({ "x-settings": {} }), &expected, &reports);
    }

    #[test]
    fn overlapping_simplified_patterns_complete_parent_and_child_in_one_pass() {
        let text = "types:\n  Parent:\n    options: \"{ enabled: boolean }(default({}))\"\n\
                    \x20 Nested:\n    options: \"{ enabled: boolean(default(true)) }\"\n\
                    $schema:\n  '<starting::x->': Parent\n  '<ending::-settings>': Nested\n";

        options_complete_in_one_pass(&schema(text));
    }

    #[test]
    fn overlapping_raw_patterns_complete_parent_and_child_in_one_pass() {
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "patternProperties": { "^x-": parent(), "-settings$": nested() },
        });

        options_complete_in_one_pass(&schema(&raw.to_string()));
    }

    #[test]
    fn a_pattern_and_another_members_catch_all_complete_in_one_pass() {
        let raw = json!({
            "$schema": DRAFT,
            "allOf": [
                { "type": "object", "patternProperties": { "^x-": parent() } },
                { "type": "object", "additionalProperties": nested() },
            ],
        });

        options_complete_in_one_pass(&schema(&raw.to_string()));
    }

    #[test]
    fn a_literal_merged_across_all_of_members_completes_in_one_pass() {
        let raw = json!({
            "$schema": DRAFT,
            "allOf": [
                { "type": "object", "properties": { "x-settings": parent() } },
                { "type": "object", "properties": { "x-settings": nested() } },
            ],
        });

        options_complete_in_one_pass(&schema(&raw.to_string()));
    }

    fn settings() -> Value {
        json!({ "type": "object", "properties": { "enabled": { "type": "boolean", "default": true } } })
    }

    #[test]
    fn another_members_catch_all_completes_a_literally_named_key() {
        let raw = json!({
            "$schema": DRAFT,
            "allOf": [
                { "type": "object", "properties": { "x-settings": { "type": "object" } } },
                { "type": "object", "additionalProperties": settings() },
            ],
        });

        completes_beneath(&schema(&raw.to_string()));
    }

    #[test]
    fn another_members_guarded_catch_all_completes_a_key_its_patterns_miss() {
        let raw = json!({
            "$schema": DRAFT,
            "allOf": [
                { "type": "object", "properties": { "x-settings": { "type": "object" } } },
                {
                    "type": "object",
                    "patternProperties": { "^y-": { "type": "object" } },
                    "additionalProperties": settings(),
                },
            ],
        });

        completes_beneath(&schema(&raw.to_string()));
    }

    /// `n` is required by one selected definition and has a `null` default
    /// in another, so the missing key is inserted as `null`.
    #[track_caller]
    fn required_null_is_inserted(settings: &SimplifiedSchema) {
        let reports = [applied("x-settings.n", Value::Null)];
        completes_in_one_pass(settings, &json!({ "x-settings": {} }), &json!({ "x-settings": { "n": null } }), &reports);
    }

    fn requires_n() -> Value {
        json!({ "type": "object", "required": ["n"], "properties": { "n": { "type": ["number", "null"] } } })
    }

    fn null_default_n() -> Value {
        json!({ "type": "object", "properties": { "n": { "type": "null", "default": null } } })
    }

    #[test]
    fn required_and_a_null_default_from_matching_patterns_insert_null() {
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "patternProperties": { "^x-": requires_n(), "-settings$": null_default_n() },
        });

        required_null_is_inserted(&schema(&raw.to_string()));
    }

    #[test]
    fn required_and_a_null_default_from_a_literal_merged_across_all_of_insert_null() {
        let raw = json!({
            "$schema": DRAFT,
            "allOf": [
                { "type": "object", "properties": { "x-settings": requires_n() } },
                { "type": "object", "properties": { "x-settings": null_default_n() } },
            ],
        });

        required_null_is_inserted(&schema(&raw.to_string()));
    }

    #[test]
    fn overlapping_patterns_with_different_entry_defaults_conflict_for_a_null_entry() {
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "patternProperties": {
                "^x-": { "type": "number", "default": 1 },
                "-settings$": { "type": "number", "default": 2 },
            },
        });
        let overlapping = schema(&raw.to_string());

        let error = unprepared(overlapping.validate_soft(&json!({ "x-settings": null })));

        assert!(matches!(&*error, SchemaError::ConflictingDefaults { property, .. } if property == "x-settings"), "{error:?}");
        let written = json!({ "x-settings": 5 });
        assert!(borrowed(overlapping.validate_soft(&written), &written).is_empty(), "an unused conflict is not judged");
    }
}

/// A used default is checked against every definition that applies to its
/// property together, before its children are completed: a default object
/// missing a child another selected definition requires is refused, never
/// repaired by that child's own default.
mod complete_default_checks {
    use super::*;

    /// Both modes refuse the schema with an `InvalidDefault` at `path`.
    #[track_caller]
    fn invalid_default_at(settings: &SimplifiedSchema, input: &Value, path: &str) {
        for (mode, result) in [("strict", settings.validate_strict(input)), ("soft", settings.validate_soft(input))] {
            let error = unprepared(result);
            assert!(
                matches!(&*error, SchemaError::InvalidDefault { path: at, .. } if at.to_string() == path),
                "{mode}, input {input}: expected InvalidDefault at {path}, got {error:?}"
            );
        }
    }

    const REQUIRED_CHILD: &str = "types:\n  Parent:\n    options: \"{ enabled: boolean }(default({}))\"\n\
                                  \x20 Nested:\n    options: \"{ enabled: boolean(required; default(true)) }\"\n\
                                  $schema:\n  '<starting::x->': Parent\n  '<ending::-settings>': Nested\n";

    #[test]
    fn overlapping_simplified_patterns_refuse_a_default_missing_another_selectors_required_child() {
        invalid_default_at(&schema(REQUIRED_CHILD), &json!({ "x-settings": {} }), "x-settings.options");
    }

    /// An object whose `options` defaults to `{}`.
    fn parent() -> Value {
        json!({
            "type": "object",
            "properties": {
                "options": { "type": "object", "properties": { "enabled": { "type": "boolean" } }, "default": {} },
            },
        })
    }

    /// An object whose `options` requires `enabled`, which defaults to `true`.
    fn requires_enabled() -> Value {
        json!({
            "type": "object",
            "properties": {
                "options": {
                    "type": "object",
                    "required": ["enabled"],
                    "properties": { "enabled": { "type": "boolean", "default": true } },
                },
            },
        })
    }

    #[track_caller]
    fn refused_beneath_x_settings(raw: Value) {
        invalid_default_at(&schema(&raw.to_string()), &json!({ "x-settings": {} }), "x-settings.options");
    }

    #[test]
    fn overlapping_raw_patterns_refuse_the_default() {
        refused_beneath_x_settings(json!({
            "$schema": DRAFT,
            "type": "object",
            "patternProperties": { "^x-": parent(), "-settings$": requires_enabled() },
        }));
    }

    /// A nullable wrapper is no union of alternatives for a value that
    /// exists: definitions beneath two nullable selector values still apply
    /// together.
    #[test]
    fn nullable_selector_values_still_refuse_the_default() {
        let nullable = |schema: Value| json!({ "anyOf": [{ "type": "null" }, schema] });
        refused_beneath_x_settings(json!({
            "$schema": DRAFT,
            "type": "object",
            "patternProperties": { "^x-": nullable(parent()), "-settings$": nullable(requires_enabled()) },
        }));
    }

    #[test]
    fn a_literal_property_and_a_matching_pattern_refuse_the_default() {
        refused_beneath_x_settings(json!({
            "$schema": DRAFT,
            "type": "object",
            "properties": { "x-settings": parent() },
            "patternProperties": { "-settings$": requires_enabled() },
        }));
    }

    #[test]
    fn a_literal_merged_across_all_of_members_refuses_the_default() {
        refused_beneath_x_settings(json!({
            "$schema": DRAFT,
            "allOf": [
                { "type": "object", "properties": { "x-settings": parent() } },
                { "type": "object", "properties": { "x-settings": requires_enabled() } },
            ],
        }));
    }

    #[test]
    fn a_pattern_and_another_members_catch_all_refuse_the_default() {
        refused_beneath_x_settings(json!({
            "$schema": DRAFT,
            "allOf": [
                { "type": "object", "patternProperties": { "^x-": parent() } },
                { "type": "object", "additionalProperties": requires_enabled() },
            ],
        }));
    }

    #[test]
    fn catch_alls_of_separate_all_of_members_refuse_the_default() {
        refused_beneath_x_settings(json!({
            "$schema": DRAFT,
            "allOf": [
                { "type": "object", "additionalProperties": parent() },
                { "type": "object", "additionalProperties": requires_enabled() },
            ],
        }));
    }

    #[test]
    fn reference_targets_joined_by_all_of_refuse_the_default() {
        let raw = json!({
            "$schema": DRAFT,
            "$defs": { "parent": parent(), "nested": requires_enabled() },
            "allOf": [{ "$ref": "#/$defs/parent" }, { "$ref": "#/$defs/nested" }],
        });

        invalid_default_at(&schema(&raw.to_string()), &json!({}), "options");
    }

    #[test]
    fn array_items_joined_by_all_of_refuse_the_default() {
        let raw = json!({
            "$schema": DRAFT,
            "type": "array",
            "allOf": [{ "items": parent() }, { "items": requires_enabled() }],
        });

        invalid_default_at(&schema(&raw.to_string()), &json!([{}]), "0.options");
    }

    #[test]
    fn tuple_positions_joined_by_all_of_refuse_the_default_and_insert_no_position() {
        let raw = json!({
            "$schema": DRAFT,
            "type": "array",
            "allOf": [{ "prefixItems": [parent()] }, { "prefixItems": [requires_enabled()] }],
        });
        let pair = schema(&raw.to_string());

        invalid_default_at(&pair, &json!([{}]), "0.options");
        let empty = json!([]);
        assert!(borrowed(pair.validate_soft(&empty), &empty).is_empty(), "no tuple position is inserted");
    }

    /// The default's type comes from another `allOf` member's definition of
    /// the same property: checked strictly, `"3"` is no number, and it is
    /// not converted into one.
    #[test]
    fn a_scalar_default_is_checked_against_the_type_another_member_declares() {
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "allOf": [
                { "properties": { "timeout": { "default": "3" } } },
                { "properties": { "timeout": { "type": "number" } } },
            ],
        });

        invalid_default_at(&schema(&raw.to_string()), &json!({}), "timeout");
    }

    #[test]
    fn a_complete_definition_in_one_schema_refuses_the_default() {
        let simplified = schema("$schema:\n  x-settings:\n    options: \"{ enabled: boolean(required; default(true)) }(default({}))\"\n");
        invalid_default_at(&simplified, &json!({ "x-settings": {} }), "x-settings.options");
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "properties": {
                "options": {
                    "type": "object",
                    "required": ["enabled"],
                    "properties": { "enabled": { "type": "boolean", "default": true } },
                    "default": {},
                },
            },
        });
        invalid_default_at(&schema(&raw.to_string()), &json!({}), "options");
    }

    #[test]
    fn all_of_inside_the_properties_own_definition_refuses_the_default() {
        let raw = json!({
            "$schema": DRAFT,
            "type": "object",
            "properties": {
                "options": {
                    "allOf": [
                        { "type": "object", "properties": { "enabled": { "type": "boolean" } } },
                        { "required": ["enabled"], "properties": { "enabled": { "default": true } } },
                    ],
                    "default": {},
                },
            },
        });

        invalid_default_at(&schema(&raw.to_string()), &json!({}), "options");
    }

    /// The failure locations of a data refusal.
    #[track_caller]
    fn refused_at(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> Vec<String> {
        match result {
            Err(ValidationFailure::InvalidData { problems }) => problems.into_iter().map(|problem| problem.path).collect(),
            other => panic!("expected invalid data, got {other:?}"),
        }
    }

    /// The written shapes of `options` beneath the edited selector fixture:
    /// a missing or `null` value would take the invalid default; an authored
    /// object is completed; other shapes are judged as written.
    #[test]
    fn options_shapes_beneath_the_required_child_fixture() {
        let settings = schema(REQUIRED_CHILD);
        invalid_default_at(&settings, &json!({ "x-settings": { "options": null } }), "x-settings.options");

        let authored = json!({ "x-settings": { "options": {} } });
        let expected = json!({ "x-settings": { "options": { "enabled": true } } });
        completes_in_one_pass(&settings, &authored, &expected, &[applied("x-settings.options.enabled", json!(true))]);
        let kept = json!({ "x-settings": { "options": { "enabled": false } } });
        assert!(borrowed(settings.validate_soft(&kept), &kept).is_empty(), "false is a value");

        for written in [json!(5), json!(""), json!([])] {
            let input = json!({ "x-settings": { "options": written } });
            for (mode, result) in [("strict", settings.validate_strict(&input)), ("soft", settings.validate_soft(&input))] {
                let paths = refused_at(result);
                assert!(paths.iter().any(|path| path == "/x-settings/options"), "{mode}, input {input}: {paths:?}");
            }
        }
    }
}

/// A used default is also judged against each union imposed beside the
/// definition declaring it, as a group of alternatives: at least one member
/// must accept it. Sibling alternatives of the default's own union are still
/// not consulted (see `required_defaults::unions`).
mod independent_union_checks {
    use std::borrow::Cow;

    use simplified_schema::coerce::NestedTemplates;
    use simplified_schema::defaults::{DefaultPlan, prepare_defaults};
    use simplified_schema::validate::ValidatorCache;

    use super::*;

    const KEYWORDS: [&str; 2] = ["anyOf", "oneOf"];

    /// Both modes refuse the schema with an `InvalidDefault` at `path`.
    #[track_caller]
    fn invalid_default_at(settings: &SimplifiedSchema, input: &Value, path: &str) {
        for (mode, result) in [("strict", settings.validate_strict(input)), ("soft", settings.validate_soft(input))] {
            let error = unprepared(result);
            assert!(
                matches!(&*error, SchemaError::InvalidDefault { path: at, .. } if at.to_string() == path),
                "{mode}, input {input}: expected InvalidDefault at {path}, got {error:?}"
            );
        }
    }

    /// An object whose `options` defaults to `{}`.
    fn parent() -> Value {
        json!({ "type": "object", "properties": { "options": { "type": "object", "default": {} } } })
    }

    /// An object whose `options` requires `enabled`, which defaults to `true`.
    fn requires_enabled() -> Value {
        json!({
            "type": "object",
            "properties": {
                "options": {
                    "type": "object",
                    "required": ["enabled"],
                    "properties": { "enabled": { "type": "boolean", "default": true } },
                },
            },
        })
    }

    /// An object whose `options` requires `mode`, which has no default.
    fn requires_mode() -> Value {
        json!({
            "type": "object",
            "properties": {
                "options": { "type": "object", "required": ["mode"], "properties": { "mode": { "type": "string" } } },
            },
        })
    }

    /// `requires_enabled` or `requires_mode`, joined by `keyword`.
    fn nested(keyword: &str) -> Value {
        json!({ keyword: [requires_enabled(), requires_mode()] })
    }

    #[track_caller]
    fn raw(document: Value) -> SimplifiedSchema {
        schema(&document.to_string())
    }

    #[test]
    fn simplified_named_alternatives_refuse_a_default_neither_accepts() {
        let text = "types:\n  Parent:\n    options: object(default({}))\n  Nested:\n\
                    \x20   - options: \"{ enabled: boolean(required; default(true)) }\"\n\
                    \x20   - options: \"{ mode: string(required) }\"\n\
                    $schema:\n  '<starting::x->': Parent\n  '<ending::-settings>': Nested\n";

        invalid_default_at(&schema(text), &json!({ "x-settings": {} }), "x-settings.options");
    }

    #[test]
    fn overlapping_raw_patterns_refuse_the_default() {
        for keyword in KEYWORDS {
            let settings = raw(json!({
                "$schema": DRAFT,
                "type": "object",
                "patternProperties": { "^x-": parent(), "-settings$": nested(keyword) },
            }));
            invalid_default_at(&settings, &json!({ "x-settings": {} }), "x-settings.options");
        }
    }

    #[test]
    fn a_literal_and_a_matching_patterns_union_refuse_the_default() {
        for keyword in KEYWORDS {
            let settings = raw(json!({
                "$schema": DRAFT,
                "type": "object",
                "properties": { "x-settings": parent() },
                "patternProperties": { "-settings$": nested(keyword) },
            }));
            invalid_default_at(&settings, &json!({ "x-settings": {} }), "x-settings.options");
        }
    }

    #[test]
    fn a_literal_merged_across_all_of_refuses_the_default() {
        for keyword in KEYWORDS {
            let settings = raw(json!({
                "$schema": DRAFT,
                "allOf": [
                    { "type": "object", "properties": { "x-settings": parent() } },
                    { "type": "object", "properties": { "x-settings": nested(keyword) } },
                ],
            }));
            invalid_default_at(&settings, &json!({ "x-settings": {} }), "x-settings.options");
        }
    }

    #[test]
    fn a_pattern_and_another_members_catch_all_union_refuse_the_default() {
        for keyword in KEYWORDS {
            let settings = raw(json!({
                "$schema": DRAFT,
                "allOf": [
                    { "type": "object", "patternProperties": { "^x-": parent() } },
                    { "type": "object", "additionalProperties": nested(keyword) },
                ],
            }));
            invalid_default_at(&settings, &json!({ "x-settings": {} }), "x-settings.options");
        }
    }

    #[test]
    fn separate_catch_alls_across_all_of_refuse_the_default() {
        for keyword in KEYWORDS {
            let settings = raw(json!({
                "$schema": DRAFT,
                "allOf": [
                    { "type": "object", "additionalProperties": parent() },
                    { "type": "object", "additionalProperties": nested(keyword) },
                ],
            }));
            invalid_default_at(&settings, &json!({ "x-settings": {} }), "x-settings.options");
        }
    }

    #[test]
    fn reference_targets_across_all_of_refuse_the_default() {
        for keyword in KEYWORDS {
            let settings = raw(json!({
                "$schema": DRAFT,
                "$defs": { "parent": parent(), "nested": nested(keyword) },
                "allOf": [{ "$ref": "#/$defs/parent" }, { "$ref": "#/$defs/nested" }],
            }));
            invalid_default_at(&settings, &json!({}), "options");
        }
    }

    #[test]
    fn joined_array_items_refuse_the_default() {
        for keyword in KEYWORDS {
            let list = raw(json!({
                "$schema": DRAFT,
                "type": "array",
                "allOf": [{ "items": parent() }, { "items": nested(keyword) }],
            }));
            invalid_default_at(&list, &json!([{}]), "0.options");
        }
    }

    #[test]
    fn joined_tuple_positions_refuse_the_default_and_insert_no_position() {
        for keyword in KEYWORDS {
            let pair = raw(json!({
                "$schema": DRAFT,
                "type": "array",
                "allOf": [{ "prefixItems": [parent()] }, { "prefixItems": [nested(keyword)] }],
            }));
            invalid_default_at(&pair, &json!([{}]), "0.options");
            let empty = json!([]);
            assert!(borrowed(pair.validate_soft(&empty), &empty).is_empty(), "{keyword}: no tuple position is inserted");
        }
    }

    /// `options` defaults to `{}` in the root's own properties, beside a root
    /// union whose alternatives each require a child of it.
    fn common_root(keyword: &str) -> Value {
        json!({
            "$schema": DRAFT,
            "type": "object",
            "properties": { "options": { "type": "object", "default": {} } },
            keyword: [requires_enabled(), requires_mode()],
        })
    }

    #[test]
    fn a_common_default_beside_a_root_union_is_refused() {
        for keyword in KEYWORDS {
            invalid_default_at(&raw(common_root(keyword)), &json!({}), "options");
        }
    }

    #[test]
    fn a_default_on_one_union_is_judged_against_another_independent_union() {
        for keyword in KEYWORDS {
            let other = json!({ "type": "object", "properties": { "other": { "type": "string" } } });
            let settings = raw(json!({
                "$schema": DRAFT,
                "allOf": [{ keyword: [parent(), other] }, nested(keyword)],
            }));
            invalid_default_at(&settings, &json!({}), "options");
        }
    }

    #[test]
    fn a_scalar_default_beside_a_union_is_refused_without_conversion() {
        for keyword in KEYWORDS {
            let timeout = raw(json!({
                "$schema": DRAFT,
                "properties": { "timeout": { "default": "3" } },
                keyword: [
                    { "properties": { "timeout": { "type": "number" } } },
                    { "properties": { "timeout": { "type": "boolean" } } },
                ],
            }));
            invalid_default_at(&timeout, &json!({}), "timeout");
        }
    }

    #[test]
    fn default_preparation_refuses_before_completing_children() {
        for keyword in KEYWORDS {
            let plan = DefaultPlan::compile(Arc::new(common_root(keyword)), &ValidatorCache::new()).expect("compiles");
            let input = json!({});

            let error = prepare_defaults(&plan, Cow::Borrowed(&input), &NestedTemplates::default()).unwrap_err();

            assert!(
                matches!(&error, SchemaError::InvalidDefault { path, .. } if path.to_string() == "options"),
                "{keyword}: {error:?}"
            );
        }
    }

    #[test]
    fn prepared_verdicts_refuse_the_default() {
        for keyword in KEYWORDS {
            let dir = tempfile::tempdir().expect("temp dir");
            let prepared = Prepared::new(Arc::new(common_root(keyword)), None, biscuit_file::FileResolutionContext::new(dir.path()))
                .expect("prepared");
            let input = json!({});
            for (mode, result) in [("strict", prepared.validate_strict(&input)), ("soft", prepared.validate_soft(&input))] {
                let error = unprepared(result);
                assert!(
                    matches!(&*error, SchemaError::InvalidDefault { path, .. } if path.to_string() == "options"),
                    "{keyword}, {mode}: {error:?}"
                );
            }
        }
    }

    /// One member of the independent union accepts the default: it is
    /// applied and its children completed.
    #[test]
    fn an_independent_union_member_accepting_the_default_lets_it_apply() {
        let optional_mode = json!({
            "type": "object",
            "properties": { "options": { "type": "object", "properties": { "mode": { "type": "string" } } } },
        });
        let settings = raw(json!({
            "$schema": DRAFT,
            "type": "object",
            "properties": { "options": { "type": "object", "default": {} } },
            "anyOf": [requires_enabled(), optional_mode],
        }));
        let input = json!({});
        let expected = [applied("options", json!({})), applied("options.enabled", json!(true))];

        assert_eq!(borrowed(settings.validate_strict(&input), &input), expected);
        let (value, defaults) = owned(settings.validate_soft(&input));
        assert_eq!(value, json!({ "options": { "enabled": true } }));
        assert_eq!(defaults, expected);
    }

    /// A member that says nothing about the key accepts any default for it.
    #[test]
    fn an_independent_union_member_without_the_key_lets_the_default_apply() {
        for keyword in KEYWORDS {
            let other = json!({ "type": "object", "properties": { "other": { "type": "string" } }, "required": ["other"] });
            let settings = raw(json!({
                "$schema": DRAFT,
                "type": "object",
                "properties": { "options": { "type": "object", "default": {} } },
                keyword: [requires_mode(), other],
            }));
            let input = json!({ "other": "x" });

            assert_eq!(borrowed(settings.validate_strict(&input), &input), [applied("options", json!({}))], "{keyword}");
        }
    }

    /// The union inside the property's own definition is judged by that
    /// definition's validator.
    #[test]
    fn a_union_inside_the_properties_own_definition_refuses_the_default() {
        for keyword in KEYWORDS {
            let alternative = |required: &str| json!({ "type": "object", "required": [required] });
            let settings = raw(json!({
                "$schema": DRAFT,
                "type": "object",
                "properties": {
                    "options": {
                        "default": {},
                        "properties": { "enabled": { "type": "boolean", "default": true } },
                        keyword: [alternative("enabled"), alternative("mode")],
                    },
                },
            }));
            invalid_default_at(&settings, &json!({}), "options");
        }
    }

    /// An authored object is completed by its children's defaults; a
    /// written value is kept; a second soft pass changes nothing.
    #[test]
    fn authored_values_beside_an_independent_union_are_still_completed() {
        let settings = raw(common_root("anyOf"));
        let authored = json!({ "options": {} });
        let expected = [applied("options.enabled", json!(true))];

        assert_eq!(borrowed(settings.validate_strict(&authored), &authored), expected);
        let (value, defaults) = owned(settings.validate_soft(&authored));
        assert_eq!(value, json!({ "options": { "enabled": true } }));
        assert_eq!(defaults, expected);
        let again = settings.validate_soft(&value).expect("the completed copy validates");
        assert!(matches!(again.value, Cow::Borrowed(_)) && again.defaults.is_empty(), "a second soft pass changes nothing");
        let kept = json!({ "options": { "enabled": false } });
        assert!(borrowed(settings.validate_soft(&kept), &kept).is_empty(), "false is a value");
    }

    /// A member that cannot hold the object containing the key, or denies
    /// the key, is no alternative for a default inside that object.
    mod members_that_cannot_hold_the_key {
        use super::*;

        /// Every alternative that excludes an object holding `options`,
        /// each beside `requires_enabled`: plain types, the static forms
        /// that say the same, closed objects, and references to them.
        fn excluding() -> Vec<Value> {
            vec![
                json!({ "type": "string" }),
                json!({ "type": "number" }),
                json!({ "type": "boolean" }),
                json!({ "type": "array" }),
                json!({ "type": "null" }),
                json!({ "type": ["string", "number"] }),
                json!({ "const": "auto" }),
                json!({ "enum": ["auto", 3] }),
                json!(false),
                json!({ "$ref": "#/$defs/string" }),
                json!({ "$ref": "#/$defs/null" }),
                json!({ "type": "object", "additionalProperties": false }),
                json!({ "$ref": "#/$defs/closed" }),
            ]
        }

        /// The referenced alternatives, placed at the root of each schema.
        fn definitions() -> Value {
            json!({
                "string": { "type": "string" },
                "null": { "type": "null" },
                "closed": { "type": "object", "properties": { "other": { "type": "string" } }, "additionalProperties": false },
                "object": { "type": "object" },
            })
        }

        /// Each structural route with `requires_enabled` or `alternative`
        /// joined by `keyword`: the schema, its input, and where the parent
        /// default stands.
        fn routes(keyword: &str, alternative: &Value) -> Vec<(Value, Value, &'static str)> {
            let union = json!({ keyword: [requires_enabled(), alternative] });
            let settings = json!({ "x-settings": {} });
            let route = |mut body: Value, input: Value, path| {
                body["$schema"] = json!(DRAFT);
                body["$defs"] = definitions();
                body["$defs"]["parent"] = parent();
                body["$defs"]["union"] = union.clone();
                (body, input, path)
            };
            vec![
                route(json!({ "type": "object", "patternProperties": { "^x-": parent(), "-settings$": union } }), settings.clone(), "x-settings.options"),
                route(
                    json!({ "type": "object", "properties": { "x-settings": parent() }, "patternProperties": { "-settings$": union } }),
                    settings.clone(),
                    "x-settings.options",
                ),
                route(
                    json!({ "allOf": [
                        { "type": "object", "properties": { "x-settings": parent() } },
                        { "type": "object", "properties": { "x-settings": union } },
                    ] }),
                    settings.clone(),
                    "x-settings.options",
                ),
                route(
                    json!({ "allOf": [
                        { "type": "object", "patternProperties": { "^x-": parent() } },
                        { "type": "object", "additionalProperties": union },
                    ] }),
                    settings.clone(),
                    "x-settings.options",
                ),
                route(
                    json!({ "allOf": [
                        { "type": "object", "additionalProperties": parent() },
                        { "type": "object", "additionalProperties": union },
                    ] }),
                    settings,
                    "x-settings.options",
                ),
                route(
                    json!({ "allOf": [{ "$ref": "#/$defs/parent" }, { "$ref": "#/$defs/union" }] }),
                    json!({}),
                    "options",
                ),
                route(json!({ "type": "array", "allOf": [{ "items": parent() }, { "items": union }] }), json!([{}]), "0.options"),
                route(json!({ "type": "array", "allOf": [{ "prefixItems": [parent()] }, { "prefixItems": [union] }] }), json!([{}]), "0.options"),
                route(
                    json!({ "type": "object", "properties": { "options": { "type": "object", "default": {} } }, keyword: [requires_enabled(), alternative] }),
                    json!({}),
                    "options",
                ),
                route(
                    json!({ "allOf": [{ keyword: [parent(), { "type": "string" }] }, union] }),
                    json!({}),
                    "options",
                ),
            ]
        }

        #[test]
        fn a_simplified_string_shorthand_refuses_the_default() {
            let text = "types:\n  Parent:\n    options: object(default({}))\n  Nested:\n\
                        \x20   - options: \"{ enabled: boolean(required; default(true)) }\"\n\
                        \x20   - string\n\
                        $schema:\n  '<starting::x->': Parent\n  '<ending::-settings>': Nested\n";

            invalid_default_at(&schema(text), &json!({ "x-settings": {} }), "x-settings.options");
        }

        #[test]
        fn every_route_refuses_a_default_beside_a_member_that_cannot_hold_the_key() {
            for keyword in KEYWORDS {
                for alternative in excluding() {
                    for (body, input, path) in routes(keyword, &alternative) {
                        let settings = raw(body.clone());
                        for (mode, result) in [("strict", settings.validate_strict(&input)), ("soft", settings.validate_soft(&input))] {
                            let error = unprepared(result);
                            assert!(
                                matches!(&*error, SchemaError::InvalidDefault { path: at, .. } if at.to_string() == path),
                                "{keyword}, {mode}, alternative {alternative}, schema {body}: got {error:?}"
                            );
                        }
                    }
                }
            }
        }

        #[test]
        fn a_scalar_default_beside_a_string_member_is_refused_before_conversion() {
            for keyword in KEYWORDS {
                let timeout = raw(json!({
                    "$schema": DRAFT,
                    "properties": { "timeout": { "default": "3" } },
                    keyword: [{ "type": "object", "required": ["timeout"], "properties": { "timeout": { "type": "number" } } }, { "type": "string" }],
                }));
                invalid_default_at(&timeout, &json!({}), "timeout");
            }
        }

        fn string_member(keyword: &str) -> Value {
            json!({
                "$schema": DRAFT,
                "type": "object",
                "properties": { "options": { "type": "object", "default": {} } },
                keyword: [requires_enabled(), { "type": "string" }],
            })
        }

        #[test]
        fn public_preparation_and_prepared_verdicts_refuse_the_default() {
            for keyword in KEYWORDS {
                let plan = DefaultPlan::compile(Arc::new(string_member(keyword)), &ValidatorCache::new()).expect("compiles");
                let input = json!({});
                let error = prepare_defaults(&plan, Cow::Borrowed(&input), &NestedTemplates::default()).unwrap_err();
                assert!(matches!(&error, SchemaError::InvalidDefault { path, .. } if path.to_string() == "options"), "{keyword}: {error:?}");

                let dir = tempfile::tempdir().expect("temp dir");
                let prepared =
                    Prepared::new(Arc::new(string_member(keyword)), None, biscuit_file::FileResolutionContext::new(dir.path()))
                        .expect("prepared");
                for (mode, result) in [("strict", prepared.validate_strict(&input)), ("soft", prepared.validate_soft(&input))] {
                    let error = unprepared(result);
                    assert!(
                        matches!(&*error, SchemaError::InvalidDefault { path, .. } if path.to_string() == "options"),
                        "{keyword}, {mode}: {error:?}"
                    );
                }
            }
        }

        /// An authored object beside the string member is still completed,
        /// and a written value is kept.
        #[test]
        fn authored_values_beside_a_string_member_are_still_completed() {
            for keyword in KEYWORDS {
                let settings = raw(string_member(keyword));
                let authored = json!({ "options": {} });
                let (value, defaults) = owned(settings.validate_soft(&authored));
                assert_eq!(value, json!({ "options": { "enabled": true } }), "{keyword}");
                assert_eq!(defaults, [applied("options.enabled", json!(true))], "{keyword}");
                let kept = json!({ "options": { "enabled": false } });
                assert!(borrowed(settings.validate_strict(&kept), &kept).is_empty(), "{keyword}: false is a value");
            }
        }

        /// A member that admits an object and says nothing about the key is
        /// an unconstrained alternative: the default applies.
        #[test]
        fn object_admitting_members_still_let_the_default_apply() {
            for alternative in [
                json!({ "type": "object" }),
                json!({ "type": ["object", "string"] }),
                json!({ "$ref": "#/$defs/object" }),
                json!({ "type": "object", "additionalProperties": false, "patternProperties": { "^opt": {} } }),
            ] {
                let settings = raw(json!({
                    "$schema": DRAFT,
                    "$defs": definitions(),
                    "type": "object",
                    "properties": { "options": { "type": "object", "default": {} } },
                    "anyOf": [requires_enabled(), alternative],
                }));
                let input = json!({});
                assert_eq!(
                    borrowed(settings.validate_strict(&input), &input),
                    [applied("options", json!({})), applied("options.enabled", json!(true))],
                    "alternative {alternative}"
                );
            }
        }

        /// A member that declares the key `false`, by name or by pattern,
        /// already refuses the default through its own definition.
        #[test]
        fn explicit_false_definitions_refuse_the_default() {
            for keyword in KEYWORDS {
                for alternative in [
                    json!({ "type": "object", "properties": { "options": false } }),
                    json!({ "type": "object", "patternProperties": { "^options$": false } }),
                ] {
                    let settings = raw(json!({
                        "$schema": DRAFT,
                        "type": "object",
                        "properties": { "options": { "type": "object", "default": {} } },
                        keyword: [requires_enabled(), alternative],
                    }));
                    invalid_default_at(&settings, &json!({}), "options");
                }
            }
        }
    }
}
