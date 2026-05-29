//! Schema-driven frontmatter type coercion.
//!
//! When a frontmatter value is *trivially* the wrong JSON type but
//! unambiguously convertible to the type a property's compiled JSON Schema
//! declares (e.g. the string `"true"` against a `boolean` field), Darkmatter
//! coerces the stored value to the declared type instead of failing
//! validation. Coercion is driven entirely by walking the merged compiled
//! JSON Schema (`EffectiveSchema::json_schema`) — the same shapes
//! [`super::simplified::convert`] emits — so it covers inline `$schema`,
//! baseline-merged fields, raw JSON Schema, and root unions through one path.
//!
//! [`coerce_frontmatter`] is the single source of truth, used by both the
//! library validation path and the compose write-back. It is pure: it builds a
//! coerced copy and never mutates its inputs.
//!
//! See `darkmatter/features/2026-05-28-schema-coercion/design.md` for the
//! recognizer table and union algorithm this implements.

use serde_json::{Map, Value};

use super::simplified::convert::BOOLISH_VALUES;
use super::validate::{self, build_validator};

/// The conversion a recognized property schema asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoercionTarget {
    /// A string in the boolish set → a real boolean.
    ToBoolean,
    /// A numeric string → a real number.
    ToNumber,
    /// A scalar (number or boolean) → its canonical string form.
    ToString,
    /// An array whose elements coerce by the inner target.
    Array(Box<CoercionTarget>),
}

/// Result of coercing a whole instance against a schema.
#[derive(Debug, Clone)]
pub struct CoercionOutcome {
    /// The coerced instance (a clone when nothing changed).
    pub value: Value,
    /// Whether any value was actually converted.
    pub changed: bool,
}

/// Maps a single property's JSON Schema fragment to its coercion target, or
/// `None` when the fragment is outside the coercion matrix.
///
/// Only the specific shapes [`super::simplified::convert`] emits are
/// recognized; anything else (objects, `any`, bare enums, multi-type `type`
/// arrays, generic `string|number` unions) yields `None` and is left to the
/// validator untouched.
pub fn coercion_target(property_schema: &Value) -> Option<CoercionTarget> {
    let obj = property_schema.as_object()?;

    // `anyOf` shapes (boolish / numberlike) are checked before single `type`
    // because the boolish/numberlike fragments have no top-level `type` key.
    if let Some(arms) = obj.get("anyOf").and_then(Value::as_array) {
        return target_from_any_of(arms);
    }

    // A multi-type `type` array (e.g. `["string","null"]`) is outside the
    // matrix: only a single string `type` is recognized.
    let ty = obj.get("type")?.as_str()?;
    match ty {
        "boolean" => Some(CoercionTarget::ToBoolean),
        "number" | "integer" => Some(CoercionTarget::ToNumber),
        "string" => Some(CoercionTarget::ToString),
        "array" => {
            let items = obj.get("items")?;
            let inner = coercion_target(items)?;
            Some(CoercionTarget::Array(Box::new(inner)))
        }
        // "object" and anything else fall outside the matrix.
        _ => None,
    }
}

/// Recognizes the two `anyOf` shapes Darkmatter emits.
///
/// - **boolish**: one arm is `{"type":"boolean"}` and one arm carries an
///   `"enum"` key → [`CoercionTarget::ToBoolean`].
/// - **numberlike**: one arm is `{"type":"number"}` and one arm is
///   `{"type":"string"}` carrying a `"pattern"` → [`CoercionTarget::ToNumber`].
///
/// The `pattern` requirement on the numberlike string arm is what distinguishes
/// it from a generic `string|number` property union (which has no pattern and
/// therefore yields `None`, leaving such unions untouched).
fn target_from_any_of(arms: &[Value]) -> Option<CoercionTarget> {
    let has_type = |t: &str| {
        arms.iter()
            .any(|a| a.get("type").and_then(Value::as_str) == Some(t))
    };
    let has_enum_arm = arms.iter().any(|a| a.get("enum").is_some());
    let has_pattern_string_arm = arms.iter().any(|a| {
        a.get("type").and_then(Value::as_str) == Some("string") && a.get("pattern").is_some()
    });

    if has_type("boolean") && has_enum_arm {
        Some(CoercionTarget::ToBoolean)
    } else if has_type("number") && has_pattern_string_arm {
        Some(CoercionTarget::ToNumber)
    } else {
        None
    }
}

/// Builds a coerced copy of `instance` against `json_schema` and reports
/// whether anything changed. Pure: never mutates inputs.
///
/// For a root `anyOf` union, each arm is tried in index order: a per-arm
/// coerced candidate is built and strict-validated, and the first arm whose
/// candidate validates is committed. If no arm validates, the instance is
/// returned unchanged so the existing union error reporting runs.
pub fn coerce_frontmatter(json_schema: &Value, instance: &Value) -> CoercionOutcome {
    if let Some(arms) = json_schema.get("anyOf").and_then(Value::as_array) {
        return coerce_root_union(arms, instance);
    }
    coerce_object(json_schema, instance)
}

/// Root-union pass: commit the first arm whose coerced candidate validates.
fn coerce_root_union(arms: &[Value], instance: &Value) -> CoercionOutcome {
    for arm in arms {
        let candidate = coerce_object(arm, instance).value;
        let wrapped = validate::wrap_arm_as_root_schema(arm);
        // A schema that fails to build cannot be a valid arm; skip it. The
        // surrounding validator already surfaces build failures elsewhere.
        let Ok(validator) = build_validator(&wrapped) else {
            continue;
        };
        if validator.is_valid(&candidate) {
            let changed = &candidate != instance;
            return CoercionOutcome {
                value: candidate,
                changed,
            };
        }
    }
    CoercionOutcome {
        value: instance.clone(),
        changed: false,
    }
}

/// Non-union object pass: coerce each instance property that the schema
/// declares with a recognized target. Properties absent from the schema or
/// instance, or with no recognized target, are untouched.
fn coerce_object(schema: &Value, instance: &Value) -> CoercionOutcome {
    let (Some(props), Some(obj)) = (
        schema.get("properties").and_then(Value::as_object),
        instance.as_object(),
    ) else {
        return CoercionOutcome {
            value: instance.clone(),
            changed: false,
        };
    };

    let mut out: Map<String, Value> = obj.clone();
    let mut changed = false;
    for (name, value) in obj {
        let Some(prop_schema) = props.get(name) else {
            continue;
        };
        let Some(target) = coercion_target(prop_schema) else {
            continue;
        };
        if let Some(coerced) = coerce_value(&target, value) {
            out.insert(name.clone(), coerced);
            changed = true;
        }
    }

    CoercionOutcome {
        value: Value::Object(out),
        changed,
    }
}

/// Applies a scalar/array coercion target to one value.
///
/// Returns `Some(replacement)` when the value was converted, `None` to leave it
/// untouched. The scalar rules only fire on a *mismatched* JSON type, so an
/// already-correctly-typed value yields `None` (this is what makes coercion
/// idempotent).
fn coerce_value(target: &CoercionTarget, value: &Value) -> Option<Value> {
    match target {
        CoercionTarget::ToBoolean => coerce_to_boolean(value),
        CoercionTarget::ToNumber => coerce_to_number(value),
        CoercionTarget::ToString => coerce_to_string(value),
        CoercionTarget::Array(inner) => coerce_array(inner, value),
    }
}

fn coerce_to_boolean(value: &Value) -> Option<Value> {
    let Value::String(s) = value else {
        return None;
    };
    if !BOOLISH_VALUES.contains(&s.as_str()) {
        return None;
    }
    // The boolish set is exactly the `true`/`false` spellings, so a non-`true`
    // member is necessarily a `false` spelling.
    Some(Value::Bool(s.eq_ignore_ascii_case("true")))
}

fn coerce_to_number(value: &Value) -> Option<Value> {
    let Value::String(s) = value else {
        return None;
    };
    if !is_numberlike(s) {
        return None;
    }
    // Integral strings (no `.`) parse as i64 to keep the JSON integer form;
    // otherwise fall back to f64. `is_numberlike` guarantees the grammar, so a
    // parse failure here means the value overflows the target — leave untouched.
    if !s.contains('.') {
        if let Ok(i) = s.parse::<i64>() {
            return Some(Value::Number(i.into()));
        }
    }
    let f = s.parse::<f64>().ok()?;
    serde_json::Number::from_f64(f).map(Value::Number)
}

fn coerce_to_string(value: &Value) -> Option<Value> {
    match value {
        Value::Number(n) => Some(Value::String(n.to_string())),
        Value::Bool(b) => Some(Value::String(b.to_string())),
        _ => None,
    }
}

fn coerce_array(inner: &CoercionTarget, value: &Value) -> Option<Value> {
    let Value::Array(items) = value else {
        return None;
    };
    let mut out = Vec::with_capacity(items.len());
    let mut changed = false;
    for item in items {
        match coerce_value(inner, item) {
            Some(coerced) => {
                out.push(coerced);
                changed = true;
            }
            None => out.push(item.clone()),
        }
    }
    changed.then_some(Value::Array(out))
}

/// Mirrors `^-?\d+(\.\d+)?$` exactly without compiling a runtime regex: an
/// optional leading `-`, one or more digits, and an optional `.` followed by
/// one or more digits.
fn is_numberlike(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    if bytes.first() == Some(&b'-') {
        i = 1;
    }
    let int_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == int_start {
        return false; // no integer digits
    }
    if i == bytes.len() {
        return true; // integer form
    }
    if bytes[i] != b'.' {
        return false;
    }
    i += 1;
    let frac_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i > frac_start && i == bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── coercion_target recognizer ──────────────────────────────────────

    #[test]
    fn recognizes_boolean() {
        assert_eq!(
            coercion_target(&json!({"type": "boolean"})),
            Some(CoercionTarget::ToBoolean)
        );
    }

    #[test]
    fn recognizes_boolish_any_of() {
        let frag = json!({
            "anyOf": [
                {"type": "boolean"},
                {"enum": ["true", "false", "True", "False", "TRUE", "FALSE"]}
            ]
        });
        assert_eq!(coercion_target(&frag), Some(CoercionTarget::ToBoolean));
    }

    #[test]
    fn recognizes_number_and_integer() {
        assert_eq!(
            coercion_target(&json!({"type": "number"})),
            Some(CoercionTarget::ToNumber)
        );
        assert_eq!(
            coercion_target(&json!({"type": "integer"})),
            Some(CoercionTarget::ToNumber)
        );
    }

    #[test]
    fn recognizes_numberlike_any_of() {
        let frag = json!({
            "anyOf": [
                {"type": "number"},
                {"type": "string", "pattern": r"^-?\d+(\.\d+)?$"}
            ]
        });
        assert_eq!(coercion_target(&frag), Some(CoercionTarget::ToNumber));
    }

    #[test]
    fn recognizes_string_with_and_without_constraints() {
        assert_eq!(
            coercion_target(&json!({"type": "string"})),
            Some(CoercionTarget::ToString)
        );
        assert_eq!(
            coercion_target(&json!({"type": "string", "format": "email"})),
            Some(CoercionTarget::ToString)
        );
        assert_eq!(
            coercion_target(&json!({"type": "string", "pattern": "x", "minLength": 1})),
            Some(CoercionTarget::ToString)
        );
    }

    #[test]
    fn recognizes_typed_array() {
        let frag = json!({"type": "array", "items": {"type": "boolean"}});
        assert_eq!(
            coercion_target(&frag),
            Some(CoercionTarget::Array(Box::new(CoercionTarget::ToBoolean)))
        );
    }

    #[test]
    fn array_with_unrecognized_items_is_none() {
        let frag = json!({"type": "array", "items": {"type": "object"}});
        assert_eq!(coercion_target(&frag), None);
    }

    #[test]
    fn object_any_bare_enum_and_multitype_are_none() {
        assert_eq!(coercion_target(&json!({"type": "object"})), None);
        assert_eq!(coercion_target(&json!({})), None);
        assert_eq!(coercion_target(&json!({"enum": ["a", "b"]})), None);
        assert_eq!(coercion_target(&json!({"type": ["string", "null"]})), None);
    }

    #[test]
    fn generic_string_number_union_is_none() {
        // No `pattern` on the string arm → not numberlike → left untouched.
        let frag = json!({"anyOf": [{"type": "number"}, {"type": "string"}]});
        assert_eq!(coercion_target(&frag), None);
    }

    // ── scalar coercion: boolean ────────────────────────────────────────

    #[test]
    fn string_to_boolean_all_spellings() {
        for (s, expected) in [
            ("true", true),
            ("True", true),
            ("TRUE", true),
            ("false", false),
            ("False", false),
            ("FALSE", false),
        ] {
            assert_eq!(
                coerce_to_boolean(&json!(s)),
                Some(Value::Bool(expected)),
                "spelling {s}"
            );
        }
    }

    #[test]
    fn ambiguous_strings_not_coerced_to_boolean() {
        for s in ["yes", "no", "on", "off", "1", "0"] {
            assert_eq!(coerce_to_boolean(&json!(s)), None, "value {s}");
        }
    }

    #[test]
    fn non_string_not_coerced_to_boolean() {
        assert_eq!(coerce_to_boolean(&json!(1)), None);
        assert_eq!(coerce_to_boolean(&json!(true)), None);
        assert_eq!(coerce_to_boolean(&json!(null)), None);
        assert_eq!(coerce_to_boolean(&json!([1])), None);
    }

    // ── scalar coercion: number ─────────────────────────────────────────

    #[test]
    fn string_to_number_integer_and_decimal() {
        assert_eq!(coerce_to_number(&json!("42")), Some(json!(42)));
        assert_eq!(coerce_to_number(&json!("-7")), Some(json!(-7)));
        assert_eq!(coerce_to_number(&json!("3.14")), Some(json!(3.14)));
        assert_eq!(coerce_to_number(&json!("-0.5")), Some(json!(-0.5)));
    }

    #[test]
    fn decimal_string_against_integer_field_still_produces_float() {
        // The engine just produces the number; the integer check runs later.
        assert_eq!(coerce_to_number(&json!("3.14")), Some(json!(3.14)));
    }

    #[test]
    fn non_numeric_string_not_coerced_to_number() {
        for s in ["abc", "1.2.3", "", "-", ".5", "5.", "1e3", " 1", "1 "] {
            assert_eq!(coerce_to_number(&json!(s)), None, "value {s:?}");
        }
    }

    #[test]
    fn non_string_not_coerced_to_number() {
        assert_eq!(coerce_to_number(&json!(42)), None);
        assert_eq!(coerce_to_number(&json!(null)), None);
        assert_eq!(coerce_to_number(&json!([1])), None);
    }

    // ── scalar coercion: string ─────────────────────────────────────────

    #[test]
    fn number_and_boolean_to_string() {
        assert_eq!(coerce_to_string(&json!(42)), Some(json!("42")));
        assert_eq!(coerce_to_string(&json!(3.14)), Some(json!("3.14")));
        assert_eq!(coerce_to_string(&json!(true)), Some(json!("true")));
        assert_eq!(coerce_to_string(&json!(false)), Some(json!("false")));
    }

    #[test]
    fn string_null_array_object_not_coerced_to_string() {
        assert_eq!(coerce_to_string(&json!("hi")), None);
        assert_eq!(coerce_to_string(&json!(null)), None);
        assert_eq!(coerce_to_string(&json!([1])), None);
        assert_eq!(coerce_to_string(&json!({"a": 1})), None);
    }

    // ── out-of-matrix object/string never coerced ───────────────────────

    #[test]
    fn string_to_object_and_object_to_string_untouched() {
        // string field holding an object → ToString leaves object untouched.
        assert_eq!(coerce_value(&CoercionTarget::ToString, &json!({"a": 1})), None);
        // object field is not a recognized target at all (covered above), and a
        // string value has no scalar target that would touch it.
        assert_eq!(coerce_value(&CoercionTarget::ToString, &json!("text")), None);
    }

    // ── typed arrays ────────────────────────────────────────────────────

    #[test]
    fn boolean_array_coerces_element_wise() {
        let target = CoercionTarget::Array(Box::new(CoercionTarget::ToBoolean));
        assert_eq!(
            coerce_value(&target, &json!(["true", "false"])),
            Some(json!([true, false]))
        );
    }

    #[test]
    fn mixed_array_leaves_uncoercible_element() {
        let target = CoercionTarget::Array(Box::new(CoercionTarget::ToBoolean));
        // "nope" is not boolish → kept as the string; "true" → real boolean.
        assert_eq!(
            coerce_value(&target, &json!(["true", "nope"])),
            Some(json!([true, "nope"]))
        );
    }

    #[test]
    fn array_with_no_coercible_elements_is_untouched() {
        let target = CoercionTarget::Array(Box::new(CoercionTarget::ToBoolean));
        assert_eq!(coerce_value(&target, &json!(["nope", "maybe"])), None);
        assert_eq!(coerce_value(&target, &json!("not-an-array")), None);
    }

    // ── non-union object pass ───────────────────────────────────────────

    #[test]
    fn object_pass_coerces_declared_props_and_tracks_changed() {
        let schema = json!({
            "type": "object",
            "properties": {
                "flag": {"type": "boolean"},
                "count": {"type": "number"},
                "label": {"type": "string"},
                "opaque": {"type": "object"}
            }
        });
        let instance = json!({
            "flag": "true",
            "count": "42",
            "label": 7,
            "opaque": {"k": "v"},      // recognized-but-untouchable (object value)
            "undeclared": "left-alone" // not in schema
        });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["flag"], json!(true));
        assert_eq!(outcome.value["count"], json!(42));
        assert_eq!(outcome.value["label"], json!("7"));
        assert_eq!(outcome.value["opaque"], json!({"k": "v"}));
        assert_eq!(outcome.value["undeclared"], json!("left-alone"));
    }

    #[test]
    fn object_pass_reports_unchanged_when_nothing_coerces() {
        let schema = json!({
            "type": "object",
            "properties": { "flag": {"type": "boolean"} }
        });
        let instance = json!({ "flag": true, "other": "x" });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    #[test]
    fn boolish_normalizes_and_numberlike_normalizes() {
        let schema = json!({
            "type": "object",
            "properties": {
                "flag": {"anyOf": [{"type": "boolean"}, {"enum": ["true", "false"]}]},
                "n": {"anyOf": [{"type": "number"}, {"type": "string", "pattern": r"^-?\d+(\.\d+)?$"}]}
            }
        });
        let instance = json!({ "flag": "true", "n": "42" });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert_eq!(outcome.value["flag"], json!(true));
        assert_eq!(outcome.value["n"], json!(42));
    }

    // ── root union ──────────────────────────────────────────────────────

    fn implement_like_union() -> Value {
        // Mirrors prompts/implement.md: three arms typing the has_* trio as
        // boolean, each with a different required string discriminator.
        json!({
            "anyOf": [
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": {
                        "kind": {"type": "string"},
                        "has_spec": {"type": "boolean"},
                        "has_plan": {"type": "boolean"},
                        "has_review": {"type": "boolean"}
                    },
                    "required": ["kind"]
                },
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": { "other": {"type": "string"} },
                    "required": ["other"]
                },
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": { "third": {"type": "string"} },
                    "required": ["third"]
                }
            ]
        })
    }

    #[test]
    fn root_union_commits_first_validating_arm_coercions() {
        let schema = implement_like_union();
        let instance = json!({
            "kind": "implement",
            "has_spec": "true",
            "has_plan": "false",
            "has_review": "false"
        });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["has_spec"], json!(true));
        assert_eq!(outcome.value["has_plan"], json!(false));
        assert_eq!(outcome.value["has_review"], json!(false));
    }

    #[test]
    fn root_union_returns_unchanged_when_no_arm_validates() {
        let schema = implement_like_union();
        // Missing every arm's required discriminator → no arm validates.
        let instance = json!({ "has_spec": "true" });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    // ── idempotence ─────────────────────────────────────────────────────

    #[test]
    fn coercion_is_idempotent() {
        let schema = json!({
            "type": "object",
            "properties": {
                "flag": {"type": "boolean"},
                "count": {"type": "number"},
                "tags": {"type": "array", "items": {"type": "boolean"}}
            }
        });
        let instance = json!({ "flag": "true", "count": "42", "tags": ["true", "false"] });

        let first = coerce_frontmatter(&schema, &instance);
        assert!(first.changed);

        let second = coerce_frontmatter(&schema, &first.value);
        assert!(!second.changed, "second run should be a no-op");
        assert_eq!(first.value, second.value);
    }

    #[test]
    fn null_array_object_untouched_against_scalar_targets() {
        let schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "boolean"},
                "b": {"type": "number"},
                "c": {"type": "string"}
            }
        });
        let instance = json!({ "a": null, "b": [1, 2], "c": {"k": "v"} });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }
}
