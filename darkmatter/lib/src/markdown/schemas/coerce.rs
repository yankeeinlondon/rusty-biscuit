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

use std::collections::HashSet;

use jsonschema::Validator;
use serde_json::{Map, Value};

use super::format::{DARKMATTER_JSON_FORMAT, DARKMATTER_YAML_FORMAT};
use super::simplified::convert::{BOOLISH_VALUES, NUMBERLIKE_PATTERN};
use super::validate::{self, build_validator, error_top_level_key};

/// The conversion a recognized property schema asks for.
///
/// Not `Eq`: [`CoercionTarget::LiteralConst`] carries a `serde_json::Value`,
/// which is only `PartialEq` (floats break total equality).
#[derive(Debug, Clone, PartialEq)]
pub enum CoercionTarget {
    /// A string in the boolish set → a real boolean.
    ToBoolean,
    /// A numeric string → a real number.
    ToNumber,
    /// A scalar (number or boolean) → its canonical string form.
    ToString,
    /// A native (mapping/sequence/scalar) value → its YAML string
    /// serialization, for a `yaml` content-format field (Feature D). A value
    /// that is already a string (or null) is left untouched so the `format`
    /// validator checks it directly.
    ToYamlString,
    /// A native value → its strict-JSON string serialization, for a `json`
    /// content-format field (Feature D). A value already a string (or null) is
    /// left untouched.
    ToJsonString,
    /// An array whose elements coerce by the inner target.
    Array(Box<CoercionTarget>),
    /// An inline object fragment: recurse into the named properties. Each
    /// property's value is coerced by the corresponding target; properties
    /// not listed are left untouched.
    ///
    /// The schema shape is `{"type":"object","properties":{...},
    /// "additionalProperties":false}` — the form `convert::inline_object_fragment`
    /// emits. The opaque `object` keyword (no `properties`) is outside the
    /// matrix and yields `None` from `coercion_target`.
    Object(Vec<(String, CoercionTarget)>),
    /// A `literal(x)` const fragment (`{"const": <value>}`) carrying its typed
    /// value. A non-string const reuses the boolish/numberlike conversion but
    /// only commits the coerced value when it equals the const — so an equality
    /// failure (`'3'` against `literal(2)`) never writes back an invalid value.
    /// String consts never coerce, so they never reach a `LiteralConst` target
    /// ([`literal_const_target`] yields `None` for them).
    LiteralConst(Value),
}

/// Result of coercing a whole instance against a schema.
#[derive(Debug, Clone)]
pub struct CoercionOutcome {
    /// The coerced instance (a clone when nothing changed).
    pub value: Value,
    /// Whether any value was actually converted.
    pub changed: bool,
}

/// If `schema` is one of Darkmatter's optional-property wrappers, returns the
/// typed inner arm. Two shapes are recognized:
///
/// - `{"anyOf":[{"type":"null"}, <typed>]}` (2 arms) — every optional scalar,
///   array, inline object, boolish/numberlike, and property-level union.
/// - `{"anyOf":[{"type":"null"}, {"const":""}, <file-typed>]}` (3 arms) —
///   optional scalar `file` preserving the legacy empty-string sentinel.
///
/// The recognizer is exact and only matches the wrappers
/// [`super::simplified::convert`] emits; unrelated `anyOf` schemas return
/// `None`.
pub(super) fn unwrap_nullable_arm(schema: &Value) -> Option<&Value> {
    let obj = schema.as_object()?;
    let arms = obj.get("anyOf").and_then(Value::as_array)?;

    // 2-arm form: null + typed.
    if arms.len() == 2 && arms[0].get("type").and_then(Value::as_str) == Some("null") {
        return Some(&arms[1]);
    }

    // 3-arm form: null + empty const + file-typed.
    if arms.len() == 3
        && arms[0].get("type").and_then(Value::as_str) == Some("null")
        && arms[1].get("const").and_then(Value::as_str) == Some("")
    {
        return Some(&arms[2]);
    }

    None
}

/// Maps a single property's JSON Schema fragment to its coercion target, or
/// `None` when the fragment is outside the coercion matrix.
///
/// Only the specific shapes [`super::simplified::convert`] emits are
/// recognized; anything else (opaque `object`, `any`, bare enums, multi-type
/// `type` arrays, generic unions not matching boolish/numberlike) yields
/// `None` and is left to the validator untouched.
///
/// Inline object fragments (`{"type":"object","properties":{...},
/// "additionalProperties":false}`) are recognized and yield
/// [`CoercionTarget::Object`] so the coercion engine can recurse into
/// recognized named properties while leaving opaque siblings untouched.
///
/// Per-arm coercion for property-level unions is handled by
/// [`coerce_object`], which calls `coerce_property_union` directly when it
/// encounters an `anyOf` at a property position. The specific boolish /
/// numberlike shapes (the only `anyOf` forms Darkmatter itself emits at the
/// property level today) are matched here for the matrix lookup.
pub fn coercion_target(property_schema: &Value) -> Option<CoercionTarget> {
    // Darkmatter optional properties wrap the typed fragment in a nullable
    // `anyOf`. Look through the wrapper so non-null values are coerced against
    // the real typed schema; `null` itself is left untouched by `coerce_value`.
    if let Some(inner) = unwrap_nullable_arm(property_schema) {
        return coercion_target(inner);
    }

    let obj = property_schema.as_object()?;

    // A `literal(x)` fragment is a bare `{"const": <value>}` with no `type`
    // key. A non-string const reuses the scalar conversions with an equality
    // guard; a string const never coerces (yields `None`).
    if let Some(constant) = obj.get("const") {
        return literal_const_target(constant);
    }

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
        // A `string` carrying a content-format seam (Feature D) serializes a
        // native value to that format's string form; every other string type
        // (including `file`, `email`, `url` formats) keeps the plain
        // scalar→string coercion.
        "string" => match obj.get("format").and_then(Value::as_str) {
            Some(DARKMATTER_YAML_FORMAT) => Some(CoercionTarget::ToYamlString),
            Some(DARKMATTER_JSON_FORMAT) => Some(CoercionTarget::ToJsonString),
            _ => Some(CoercionTarget::ToString),
        },
        "array" => {
            let items = obj.get("items")?;
            let inner = coercion_target(items)?;
            Some(CoercionTarget::Array(Box::new(inner)))
        }
        "object" => inline_object_target(obj),
        // "any" and anything else fall outside the matrix.
        _ => None,
    }
}

/// Builds an [`CoercionTarget::Object`] from the inline object fragment's
/// recognized `properties` entries. An opaque `object` (no `properties` field)
/// is not an inline object and yields `None`.
fn inline_object_target(obj: &Map<String, Value>) -> Option<CoercionTarget> {
    let props = obj.get("properties").and_then(Value::as_object)?;
    if props.is_empty() {
        // An empty inline object (`{}`) carries no coercions; recognising
        // it as `Object([])` would be a no-op pass-through, so yield `None`
        // to keep the matrix narrow.
        return None;
    }
    let mut inner = Vec::with_capacity(props.len());
    for (k, v) in props {
        if let Some(target) = coercion_target(v) {
            inner.push((k.clone(), target));
        }
    }
    (!inner.is_empty()).then_some(CoercionTarget::Object(inner))
}

/// Recognizes the two `anyOf` shapes Darkmatter emits, matching them *exactly*
/// so unrelated property unions are never coerced.
///
/// - **boolish**: one arm is `{"type":"boolean"}` and one arm is an `"enum"`
///   whose members are exactly [`BOOLISH_VALUES`] → [`CoercionTarget::ToBoolean`].
/// - **numberlike**: one arm is `{"type":"number"}` and one arm is
///   `{"type":"string"}` whose `"pattern"` equals [`NUMBERLIKE_PATTERN`] →
///   [`CoercionTarget::ToNumber`].
///
/// The exact `enum`/`pattern` match is the correctness boundary: a raw JSON
/// Schema union such as `{"anyOf":[{"type":"boolean"},{"enum":["auto"]}]}` or
/// `{"anyOf":[{"type":"number"},{"type":"string","pattern":"^[A-Z]+$"}]}` is
/// *not* something Darkmatter emits, so it yields `None` and is left for the
/// strict validator to accept or reject untouched.
fn target_from_any_of(arms: &[Value]) -> Option<CoercionTarget> {
    let has_type = |t: &str| {
        arms.iter()
            .any(|a| a.get("type").and_then(Value::as_str) == Some(t))
    };
    let has_boolish_enum_arm = arms.iter().any(|a| {
        a.get("enum")
            .and_then(Value::as_array)
            .is_some_and(|members| is_boolish_enum(members))
    });
    let has_numberlike_string_arm = arms.iter().any(|a| {
        a.get("type").and_then(Value::as_str) == Some("string")
            && a.get("pattern").and_then(Value::as_str) == Some(NUMBERLIKE_PATTERN)
    });

    if has_type("boolean") && has_boolish_enum_arm {
        Some(CoercionTarget::ToBoolean)
    } else if has_type("number") && has_numberlike_string_arm {
        Some(CoercionTarget::ToNumber)
    } else {
        None
    }
}

/// Maps a `literal(x)` const value to its coercion target.
///
/// A numeric or boolean const reuses the numberlike/boolish scalar conversion
/// (with the equality guard applied in [`coerce_to_literal`]); a string const
/// (or any other JSON type) never coerces and yields `None`, so a value already
/// holding the exact string is left for the strict `const` validator.
fn literal_const_target(constant: &Value) -> Option<CoercionTarget> {
    match constant {
        Value::Number(_) | Value::Bool(_) => Some(CoercionTarget::LiteralConst(constant.clone())),
        _ => None,
    }
}

/// True when an `anyOf` `enum` arm's members are *exactly* the full boolish
/// spelling set ([`BOOLISH_VALUES`]), order-independent and with no missing or
/// extra members. This is precisely the shape
/// [`super::simplified::convert::boolish_fragment`] emits, so the recognizer
/// matches only Darkmatter's own boolish fragment. Any subset (e.g. `["true"]`
/// or `["true","false"]`), any superset, or any non-boolish member fails the
/// match and is left untouched for the strict validator.
///
/// Subsets must be rejected: a raw JSON Schema such as
/// `{"anyOf":[{"type":"boolean"},{"enum":["true"]}]}` is *not* Darkmatter's
/// shape, and with instance `"false"` strict validation must fail (string, and
/// `"false"` is not in `["true"]`). Recognizing the subset would coerce
/// `"false"` → `false` and falsely validate it against the boolean arm.
fn is_boolish_enum(members: &[Value]) -> bool {
    members.len() == BOOLISH_VALUES.len()
        && BOOLISH_VALUES
            .iter()
            .all(|expected| members.iter().any(|m| m.as_str() == Some(*expected)))
}

/// Builds a coerced copy of `instance` against `json_schema` and reports
/// whether anything changed. Pure: never mutates inputs.
///
/// For a root `anyOf` union, each arm is tried in index order: a per-arm
/// coerced candidate is built and strict-validated, and the first arm whose
/// candidate validates is committed. If no arm validates, the instance is
/// returned unchanged so the existing union error reporting runs.
pub fn coerce_frontmatter(json_schema: &Value, instance: &Value) -> CoercionOutcome {
    coerce_frontmatter_with_pending(json_schema, instance, &HashSet::new())
}

/// Like [`coerce_frontmatter`], but for the compose pre-shell stage where some
/// top-level keys still hold `$(...)` shell expressions and so cannot yet be
/// coerced or validated to their declared types.
///
/// For a root `anyOf` union, an arm is committed when its coerced candidate
/// either fully validates *or* the only remaining validation problems are
/// attributable to keys in `shell_pending` — whose real type is resolved at
/// post-shell re-validation. This lets non-shell fields in the winning arm
/// (e.g. a boolish `flag: "false"`) still coerce and be written back even when a
/// sibling shell-pending typed field (e.g. `n: "$(echo 1)"`) keeps the raw
/// candidate from validating. Non-shell coercions are unaffected: a non-union
/// schema ignores `shell_pending` (its per-property pass already skips
/// uncoercible values), and an empty set reduces this to [`coerce_frontmatter`].
pub fn coerce_frontmatter_with_pending(
    json_schema: &Value,
    instance: &Value,
    shell_pending: &HashSet<String>,
) -> CoercionOutcome {
    if let Some(arms) = json_schema.get("anyOf").and_then(Value::as_array) {
        return coerce_root_union(arms, instance, shell_pending);
    }
    coerce_object(json_schema, instance)
}

/// Root-union pass: commit the first arm whose coerced candidate is accepted.
fn coerce_root_union(
    arms: &[Value],
    instance: &Value,
    shell_pending: &HashSet<String>,
) -> CoercionOutcome {
    for arm in arms {
        let candidate = coerce_object(arm, instance).value;
        let wrapped = validate::wrap_arm_as_root_schema(arm);
        // A schema that fails to build cannot be a valid arm; skip it. The
        // surrounding validator already surfaces build failures elsewhere.
        let Ok(validator) = build_validator(&wrapped, None, None) else {
            continue;
        };
        if arm_accepts(&validator, &candidate, shell_pending) {
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

/// Whether a root-union arm's coerced candidate is good enough to commit.
///
/// A fully valid candidate is always accepted. Otherwise — only when
/// `shell_pending` is non-empty — the arm is still accepted if every residual
/// validation problem points at a pending `$(...)` top-level key. Those keys
/// are re-validated and coerced after shell expansion, so deferring them here
/// is sound; a problem on any non-pending key (including a missing required
/// discriminator) rejects the arm so a genuinely wrong arm is never committed.
fn arm_accepts(validator: &Validator, candidate: &Value, shell_pending: &HashSet<String>) -> bool {
    if validator.is_valid(candidate) {
        return true;
    }
    if shell_pending.is_empty() {
        return false;
    }
    let mut saw_problem = false;
    for err in validator.iter_errors(candidate) {
        saw_problem = true;
        match error_top_level_key(&err) {
            Some(key) if shell_pending.contains(&key) => {}
            _ => return false,
        }
    }
    saw_problem
}

/// Non-union object pass: coerce each instance property that the schema
/// declares with a recognized target. Properties absent from the schema or
/// instance, or with no recognized target, are untouched.
///
/// For each declared property:
/// 1. [`coercion_target`] is consulted first. It recognises the single-type
///    forms (`string`, `boolean`, …) plus the specific boolish / numberlike
///    `anyOf` shapes — and inline object fragments with recursable
///    properties (Phase 2). When it returns a target, that target is used.
/// 2. If `coercion_target` returns `None` but the property schema is an
///    `anyOf` of multiple arms, [`coerce_property_union`] is tried: each
///    arm gets a coerced candidate, the candidate is validated against the
///    arm, and the original value is replaced only when exactly one arm's
///    candidate validates. The spec's "no guessing" rule (zero or multiple
///    matches leave the value untouched) is enforced there.
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
        let effective_schema = unwrap_nullable_arm(prop_schema).unwrap_or(prop_schema);

        // 1. Specific shape: single-type, inline object, or boolish/numberlike
        //    anyOf. `coercion_target` returns a single coherent target so the
        //    per-arm pass below is unnecessary.
        if let Some(target) = coercion_target(effective_schema) {
            if let Some(coerced) = coerce_value(&target, value) {
                out.insert(name.clone(), coerced);
                changed = true;
            }
            continue;
        }
        // 2. Property-level union with per-arm coercion. Only reached when the
        //    `anyOf` is not a boolish / numberlike / specific-shape match.
        if let Some(arm_schemas) = effective_schema.get("anyOf").and_then(Value::as_array)
            && let Some(coerced) = coerce_property_union(arm_schemas, value)
        {
            out.insert(name.clone(), coerced);
            changed = true;
        }
    }

    CoercionOutcome {
        value: Value::Object(out),
        changed,
    }
}

/// Per-arm coercion for a property-level union schema.
///
/// For each arm:
/// 1. Compute the arm's [`CoercionTarget`] via [`coercion_target`].
/// 2. Build a candidate value by applying the target (or, if the arm is
///    outside the matrix, fall back to the original value).
/// 3. Validate the candidate against the arm schema.
///
/// The candidate is committed only when **exactly one** arm's candidate
/// validates — preserving the spec's "no guessing" rule for ambiguous
/// unions (multiple matching arms) and zero-match unions (no arm can
/// accept the value). An arm whose validator fails to compile is skipped
/// without affecting the outcome.
fn coerce_property_union(arms: &[Value], value: &Value) -> Option<Value> {
    let mut valid_candidates: Vec<Value> = Vec::new();
    for arm in arms {
        let candidate = match coercion_target(arm) {
            Some(target) => coerce_value(&target, value).unwrap_or_else(|| value.clone()),
            None => value.clone(),
        };
        let Ok(validator) = build_validator(arm, None, None) else {
            continue;
        };
        if validator.is_valid(&candidate) {
            valid_candidates.push(candidate);
        }
    }
    if valid_candidates.len() == 1 {
        valid_candidates.pop()
    } else {
        None
    }
}

/// Coerces a single value against a single property schema, returning a coerced
/// copy (or a clone when nothing coerces). Pure: never mutates the input.
///
/// Mirrors the per-arm coercion [`coerce_property_union`] applies, but for a lone
/// value/schema pair rather than a property inside an object — e.g. an
/// `example(...)` artifact's `returns` against its annotated target, where the
/// schema describes the value directly (`{"type":"string","format":"darkmatter-yaml"}`)
/// instead of an object with `properties`. A schema outside the coercion matrix
/// (or an already-correctly-typed value) yields a plain clone, so this is safe to
/// run before a strict validator.
pub fn coerce_value_against_schema(schema: &Value, value: &Value) -> Value {
    match coercion_target(schema) {
        Some(target) => coerce_value(&target, value).unwrap_or_else(|| value.clone()),
        None => value.clone(),
    }
}

/// Applies a scalar/array/object coercion target to one value.
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
        CoercionTarget::ToYamlString => coerce_to_yaml_string(value),
        CoercionTarget::ToJsonString => coerce_to_json_string(value),
        CoercionTarget::Array(inner) => coerce_array(inner, value),
        CoercionTarget::Object(props) => coerce_inline_object(props, value),
        CoercionTarget::LiteralConst(constant) => coerce_to_literal(constant, value),
    }
}

/// Coerces a value toward a `literal(x)` const, committing the coerced value
/// **only** when it equals the const. Reuses the boolish/numberlike scalar
/// conversion selected by the const's JSON type; a value that coerces to a
/// different scalar (an equality failure, e.g. `'3'` against `literal(2)`) is
/// left untouched so no invalid value is written back. String consts never
/// reach here ([`literal_const_target`] rejects them).
fn coerce_to_literal(constant: &Value, value: &Value) -> Option<Value> {
    let coerced = match constant {
        Value::Number(_) => coerce_to_number(value)?,
        Value::Bool(_) => coerce_to_boolean(value)?,
        _ => return None,
    };
    (&coerced == constant).then_some(coerced)
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
    // Parse through serde_json's own number model so the coerced value is
    // exactly what the validator would have seen had the literal been written
    // bare: integral strings become `i64`/`u64` (preserving values above
    // `i64::MAX`), decimals become `f64`. `is_numberlike` already guaranteed the
    // `^-?\d+(\.\d+)?$` shape, so this only returns `None` for literals too
    // large even for `f64`, which then stay strings for the validator to reject.
    serde_json::from_str::<serde_json::Number>(s)
        .ok()
        .map(Value::Number)
}

fn coerce_to_string(value: &Value) -> Option<Value> {
    match value {
        Value::Number(n) => Some(Value::String(n.to_string())),
        Value::Bool(b) => Some(Value::String(b.to_string())),
        _ => None,
    }
}

/// Serializes a native value to a YAML string for a `yaml` content-format
/// field. Values that are already a string (the `format` validator checks them
/// directly) or `null` (the optional-property "absent" sentinel) are left
/// untouched. Serialization of a `serde_json::Value` never fails in practice;
/// on the theoretical failure the value is left untouched so the `type: string`
/// check surfaces a validation error rather than silently passing.
fn coerce_to_yaml_string(value: &Value) -> Option<Value> {
    if value.is_string() || value.is_null() {
        return None;
    }
    let yaml = serde_yaml_ng::to_string(value).ok()?;
    // Trim the single trailing newline serde_yaml_ng appends so the written-back
    // scalar stays tidy; interior newlines (multi-line mappings) are preserved.
    Some(Value::String(yaml.trim_end_matches('\n').to_string()))
}

/// Serializes a native value to a strict-JSON string for a `json` content-format
/// field. Mirrors [`coerce_to_yaml_string`]'s string/null passthrough.
fn coerce_to_json_string(value: &Value) -> Option<Value> {
    if value.is_string() || value.is_null() {
        return None;
    }
    let json = serde_json::to_string(value).ok()?;
    Some(Value::String(json))
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

/// Recursively coerces the named properties of an inline object value.
///
/// Only the properties listed in `props` are inspected; properties in
/// `value` that the schema does not declare are passed through untouched.
/// The returned value is `Some(coerced_object)` only when at least one
/// listed property was actually converted — this keeps the recursion
/// idempotent and lets the caller set [`CoercionOutcome::changed`] cheaply.
fn coerce_inline_object(
    props: &[(String, CoercionTarget)],
    value: &Value,
) -> Option<Value> {
    let Value::Object(obj) = value else {
        return None;
    };
    let mut out = obj.clone();
    let mut changed = false;
    for (name, inner_target) in props {
        let Some(field) = obj.get(name) else {
            continue;
        };
        if let Some(coerced) = coerce_value(inner_target, field) {
            out.insert(name.clone(), coerced);
            changed = true;
        }
    }
    changed.then_some(Value::Object(out))
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
// `3.14` appears as a plain decimal-string fixture, not as an approximation of
// `PI`; the `approx_constant` lint is a false positive here.
#[allow(clippy::approx_constant)]
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

    #[test]
    fn unrelated_boolean_enum_union_is_none() {
        // A raw JSON Schema union of `boolean` with an arbitrary enum is NOT the
        // boolish shape Darkmatter emits — the enum members are not the boolish
        // spellings — so it must yield `None` and never coerce `"true"` → true.
        let frag = json!({"anyOf": [{"type": "boolean"}, {"enum": ["auto"]}]});
        assert_eq!(coercion_target(&frag), None);
        // A superset enum (boolish spellings plus extras) is also not exact.
        let superset = json!({
            "anyOf": [
                {"type": "boolean"},
                {"enum": ["true", "false", "True", "False", "TRUE", "FALSE", "maybe"]}
            ]
        });
        assert_eq!(coercion_target(&superset), None);
    }

    #[test]
    fn boolish_subset_enum_union_is_none() {
        // A boolish *subset* enum is NOT the full six-spelling shape
        // `boolish_fragment` emits. Recognizing it would coerce a value like
        // `"false"` against `{"anyOf":[{"type":"boolean"},{"enum":["true"]}]}`
        // into `false` and falsely validate it against the boolean arm, even
        // though strict validation should reject the string `"false"`.
        let single = json!({"anyOf": [{"type": "boolean"}, {"enum": ["true"]}]});
        assert_eq!(coercion_target(&single), None);
        let pair = json!({"anyOf": [{"type": "boolean"}, {"enum": ["true", "false"]}]});
        assert_eq!(coercion_target(&pair), None);
    }

    #[test]
    fn unrelated_number_string_pattern_union_is_none() {
        // A number/string union whose string arm carries a *different* pattern
        // is not the numberlike shape; coercing `"42"` here would defeat the
        // string arm's intentional `^[A-Z]+$` rejection of digits.
        let frag = json!({
            "anyOf": [{"type": "number"}, {"type": "string", "pattern": "^[A-Z]+$"}]
        });
        assert_eq!(coercion_target(&frag), None);
    }

    // ── literal const recognizer & coercion (Phase 4) ───────────────────

    #[test]
    fn recognizes_number_and_boolean_const() {
        match coercion_target(&json!({"const": 2})) {
            Some(CoercionTarget::LiteralConst(v)) => assert_eq!(v, json!(2)),
            other => panic!("expected LiteralConst(2), got {other:?}"),
        }
        match coercion_target(&json!({"const": false})) {
            Some(CoercionTarget::LiteralConst(v)) => assert_eq!(v, json!(false)),
            other => panic!("expected LiteralConst(false), got {other:?}"),
        }
    }

    #[test]
    fn string_const_is_not_a_coercion_target() {
        // A string literal never coerces, so its const yields no target.
        assert_eq!(coercion_target(&json!({"const": "spec"})), None);
    }

    #[test]
    fn number_const_coerces_matching_string_only() {
        // `'2'` matches literal(2) → coerces to number 2.
        assert_eq!(
            coerce_value(&CoercionTarget::LiteralConst(json!(2)), &json!("2")),
            Some(json!(2))
        );
        // `'3'` coerces to number 3 which does not equal const 2 → untouched
        // (write-back only on a validating value).
        assert_eq!(
            coerce_value(&CoercionTarget::LiteralConst(json!(2)), &json!("3")),
            None
        );
        // Non-numberlike string → untouched.
        assert_eq!(
            coerce_value(&CoercionTarget::LiteralConst(json!(2)), &json!("two")),
            None
        );
    }

    #[test]
    fn boolean_const_coerces_matching_spelling_only() {
        assert_eq!(
            coerce_value(&CoercionTarget::LiteralConst(json!(false)), &json!("false")),
            Some(json!(false))
        );
        // `'true'` coerces to `true` which is not the const `false` → untouched.
        assert_eq!(
            coerce_value(&CoercionTarget::LiteralConst(json!(false)), &json!("true")),
            None
        );
    }

    #[test]
    fn literal_const_object_pass_and_array_form() {
        // Optional literal wraps the const in a nullable `anyOf`; the object
        // pass looks through the wrapper and coerces the string form.
        let schema = json!({
            "type": "object",
            "properties": {
                "version": {"anyOf": [{"type": "null"}, {"const": 2}]},
                "modes": {"anyOf": [
                    {"type": "null"},
                    {"type": "array", "items": {"const": 3}}
                ]}
            }
        });
        let instance = json!({ "version": "2", "modes": ["3", "4"] });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["version"], json!(2));
        // `'3'` equals the item const → number 3; `'4'` does not → left as-is.
        assert_eq!(outcome.value["modes"], json!([3, "4"]));
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
    // `3.14` is decimal-coercion test data, not an approximation of π.
    #[allow(clippy::approx_constant)]
    fn string_to_number_integer_and_decimal() {
        assert_eq!(coerce_to_number(&json!("42")), Some(json!(42)));
        assert_eq!(coerce_to_number(&json!("-7")), Some(json!(-7)));
        assert_eq!(coerce_to_number(&json!("3.14")), Some(json!(3.14)));
        assert_eq!(coerce_to_number(&json!("-0.5")), Some(json!(-0.5)));
    }

    #[test]
    // `3.14` is decimal-coercion test data, not an approximation of π.
    #[allow(clippy::approx_constant)]
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

    #[test]
    // `3.14` is decimal-coercion test data, not an approximation of π.
    #[allow(clippy::approx_constant)]
    fn large_integral_strings_match_serde_json_number_model() {
        // Coercion mirrors serde_json's number parsing: `i64::MAX` stays i64,
        // `i64::MAX + 1` becomes u64 (rather than being rejected), and the
        // coerced value equals what the bare literal would deserialize to.
        assert_eq!(
            coerce_to_number(&json!("9223372036854775807")), // i64::MAX
            Some(json!(9_223_372_036_854_775_807_i64))
        );
        assert_eq!(
            coerce_to_number(&json!("9223372036854775808")), // i64::MAX + 1
            Some(json!(9_223_372_036_854_775_808_u64))
        );
        assert_eq!(coerce_to_number(&json!("3.14")), Some(json!(3.14)));
    }

    // ── scalar coercion: string ─────────────────────────────────────────

    #[test]
    // `3.14` is float-to-string test data, not an approximation of π.
    #[allow(clippy::approx_constant)]
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
            "opaque": {"k": "v"},      // skipped at the coercion_target guard: object is not a recognized target
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
                "flag": {"anyOf": [
                    {"type": "boolean"},
                    {"enum": ["true", "false", "True", "False", "TRUE", "FALSE"]}
                ]},
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
    fn root_union_commits_arm_when_only_pending_keys_block_it() {
        // An arm declaring both a shell-pending number (`n`) and a non-shell
        // boolean (`flag`). With `n` deferred via `shell_pending`, the arm is
        // committed and `flag` coerces; `n` keeps its literal `$(...)` form.
        let schema = json!({
            "anyOf": [
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": {
                        "kind": {"type": "string"},
                        "n": {"type": "number"},
                        "flag": {"type": "boolean"}
                    },
                    "required": ["kind"]
                },
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": { "other": {"type": "string"} },
                    "required": ["other"]
                }
            ]
        });
        let instance = json!({ "kind": "implement", "n": "$(echo 1)", "flag": "false" });
        let pending: HashSet<String> = ["n".to_string()].into_iter().collect();
        let outcome = coerce_frontmatter_with_pending(&schema, &instance, &pending);
        assert!(outcome.changed);
        assert_eq!(outcome.value["flag"], json!(false));
        assert_eq!(outcome.value["n"], json!("$(echo 1)"));
    }

    #[test]
    fn root_union_without_pending_set_still_requires_full_validation() {
        // Same shape, but no key is marked pending: `n: "$(echo 1)"` is a string
        // against a `number` arm, so no arm fully validates and the instance is
        // returned unchanged (the existing union error reporting then runs).
        let schema = json!({
            "anyOf": [
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": {
                        "kind": {"type": "string"},
                        "n": {"type": "number"},
                        "flag": {"type": "boolean"}
                    },
                    "required": ["kind"]
                }
            ]
        });
        let instance = json!({ "kind": "implement", "n": "$(echo 1)", "flag": "false" });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    #[test]
    fn root_union_pending_does_not_mask_non_pending_failure() {
        // `flag` is non-pending and holds an uncoercible array, so even though
        // `n` is pending the arm has a residual non-pending problem on `/flag`
        // and must not be committed.
        let schema = json!({
            "anyOf": [
                {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": {
                        "kind": {"type": "string"},
                        "n": {"type": "number"},
                        "flag": {"type": "boolean"}
                    },
                    "required": ["kind"]
                }
            ]
        });
        let instance = json!({ "kind": "implement", "n": "$(echo 1)", "flag": [1, 2] });
        let pending: HashSet<String> = ["n".to_string()].into_iter().collect();
        let outcome = coerce_frontmatter_with_pending(&schema, &instance, &pending);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
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

    // ── Inline object coercion (Phase 2) ────────────────────────────────

    #[test]
    fn recognizes_inline_object_fragment() {
        // The shape `convert::inline_object_fragment` emits:
        // `{"type":"object","properties":{...},"additionalProperties":false}`.
        let frag = json!({
            "type": "object",
            "properties": {
                "enabled": {"type": "boolean"},
                "retries": {"type": "number"}
            },
            "additionalProperties": false
        });
        match coercion_target(&frag) {
            Some(CoercionTarget::Object(props)) => {
                assert_eq!(props.len(), 2);
                assert!(props.iter().any(|(k, t)| k == "enabled" && *t == CoercionTarget::ToBoolean));
                assert!(props.iter().any(|(k, t)| k == "retries" && *t == CoercionTarget::ToNumber));
            }
            other => panic!("expected Object, got {other:?}"),
        }
    }

    #[test]
    fn opaque_object_without_properties_is_not_recognised() {
        // The opaque `object` keyword (no `properties`) is outside the
        // matrix — coercion must not recurse into it.
        assert_eq!(coercion_target(&json!({"type": "object"})), None);
    }

    #[test]
    fn inline_object_with_unrecognized_property_keeps_recognized_targets() {
        let frag = json!({
            "type": "object",
            "properties": {
                "ok": {"type": "boolean"},
                "weird": {"type": "object"} // opaque nested object, not in matrix
            },
            "additionalProperties": false
        });
        assert_eq!(
            coercion_target(&frag),
            Some(CoercionTarget::Object(vec![(
                "ok".to_string(),
                CoercionTarget::ToBoolean
            )]))
        );
    }

    #[test]
    fn inline_object_value_coerces_declared_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "properties": {
                        "enabled": {"type": "boolean"},
                        "retries": {"type": "number"}
                    },
                    "additionalProperties": false
                }
            }
        });
        let instance = json!({
            "config": {
                "enabled": "true",
                "retries": "3",
                "untouched": "x"
            }
        });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["config"]["enabled"], json!(true));
        assert_eq!(outcome.value["config"]["retries"], json!(3));
        // Properties not declared by the schema are left untouched.
        assert_eq!(outcome.value["config"]["untouched"], json!("x"));
    }

    #[test]
    fn inline_object_value_coerces_mixed_recognized_and_opaque_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "properties": {
                        "enabled": {"type": "boolean"},
                        "metadata": {"type": "object"}
                    },
                    "additionalProperties": false
                }
            }
        });
        let instance = json!({
            "config": {
                "enabled": "true",
                "metadata": { "source": "user" }
            }
        });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["config"]["enabled"], json!(true));
        assert_eq!(
            outcome.value["config"]["metadata"],
            json!({ "source": "user" })
        );
    }

    #[test]
    fn inline_object_value_with_no_coercible_fields_unchanged() {
        // Every nested property is already the right type — no coercion
        // happens, and the function reports `changed: false`.
        let schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "properties": {
                        "enabled": {"type": "boolean"},
                        "label": {"type": "string"}
                    },
                    "additionalProperties": false
                }
            }
        });
        let instance = json!({
            "config": {
                "enabled": true,
                "label": "ready"
            }
        });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    #[test]
    fn inline_object_value_must_be_object_to_recurse() {
        // The inline object fragment expects an object value; any other
        // JSON type is left alone and the validator handles it.
        let schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "properties": {
                        "enabled": {"type": "boolean"}
                    },
                    "additionalProperties": false
                }
            }
        });
        let instance = json!({ "config": "not-an-object" });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    #[test]
    fn array_of_inline_objects_coerces_per_item() {
        // `{ enabled: boolean }[]` produces an array target whose inner
        // target is `Object([("enabled", ToBoolean)])`. Each item's
        // `enabled` is coerced independently.
        let schema = json!({
            "type": "object",
            "properties": {
                "authors": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "active": {"type": "boolean"},
                            "score": {"type": "number"}
                        },
                        "additionalProperties": false
                    }
                }
            }
        });
        let instance = json!({
            "authors": [
                { "active": "true", "score": "4.5" },
                { "active": "false", "score": "-1" }
            ]
        });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["authors"][0]["active"], json!(true));
        assert_eq!(outcome.value["authors"][0]["score"], json!(4.5));
        assert_eq!(outcome.value["authors"][1]["active"], json!(false));
        assert_eq!(outcome.value["authors"][1]["score"], json!(-1));
    }

    #[test]
    fn array_of_inline_objects_coerces_mixed_recognized_and_opaque_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "authors": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "active": {"type": "boolean"},
                            "metadata": {"type": "object"}
                        },
                        "additionalProperties": false
                    }
                }
            }
        });
        let instance = json!({
            "authors": [
                { "active": "true", "metadata": { "role": "admin" } },
                { "active": "false", "metadata": { "role": "reader" } }
            ]
        });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["authors"][0]["active"], json!(true));
        assert_eq!(outcome.value["authors"][1]["active"], json!(false));
        assert_eq!(
            outcome.value["authors"][0]["metadata"],
            json!({ "role": "admin" })
        );
    }

    // ── Per-arm coercion for property-level unions (Phase 2) ───────────

    #[test]
    fn per_arm_coercion_commits_unambiguous_inline_object_arm() {
        // Schema: an inline object arm + a string arm. The value is an
        // object with a numeric string for `count`; only the inline
        // object arm's coerced candidate validates.
        let schema = json!({
            "type": "object",
            "properties": {
                "metadata": {
                    "anyOf": [
                        {
                            "type": "object",
                            "properties": {
                                "key": {"type": "string"},
                                "count": {"type": "number"}
                            },
                            "additionalProperties": false
                        },
                        {"type": "string"}
                    ]
                }
            }
        });
        let instance = json!({
            "metadata": { "key": "visits", "count": "42" }
        });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["metadata"]["key"], json!("visits"));
        assert_eq!(outcome.value["metadata"]["count"], json!(42));
    }

    #[test]
    fn per_arm_coercion_handles_inline_object_arm_with_opaque_sibling() {
        let schema = json!({
            "type": "object",
            "properties": {
                "metadata": {
                    "anyOf": [
                        {
                            "type": "object",
                            "properties": {
                                "kind": {"const": "config"},
                                "enabled": {"type": "boolean"},
                                "details": {"type": "object"}
                            },
                            "required": ["kind", "enabled", "details"],
                            "additionalProperties": false
                        },
                        {"type": "string"}
                    ]
                }
            }
        });
        let instance = json!({
            "metadata": {
                "kind": "config",
                "enabled": "true",
                "details": { "source": "user" }
            }
        });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["metadata"]["enabled"], json!(true));
        assert_eq!(outcome.value["metadata"]["kind"], json!("config"));
        assert_eq!(
            outcome.value["metadata"]["details"],
            json!({ "source": "user" })
        );
    }

    #[test]
    fn per_arm_coercion_leaves_zero_match_value_untouched() {
        // An array value is not coercible into the inline object arm
        // (it's not an object) and not coercible into the string arm
        // (it's not a scalar). Per-arm coercion produces no candidate,
        // so the original value is left for the validator to reject.
        let schema = json!({
            "type": "object",
            "properties": {
                "metadata": {
                    "anyOf": [
                        {
                            "type": "object",
                            "properties": {
                                "key": {"type": "string"},
                                "count": {"type": "number"}
                            },
                            "additionalProperties": false
                        },
                        {"type": "string"}
                    ]
                }
            }
        });
        let instance = json!({ "metadata": ["a", "b"] });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    #[test]
    fn per_arm_coercion_leaves_ambiguous_value_untouched() {
        // `"42"` is itself a string and matches the string arm directly;
        // `"42"` also coerces to `42` and matches the number arm. Two
        // arms validate after coercion, so the value is left alone
        // (no guessing).
        let schema = json!({
            "type": "object",
            "properties": {
                "count": {
                    "anyOf": [
                        {"type": "number"},
                        {"type": "string"}
                    ]
                }
            }
        });
        let instance = json!({ "count": "42" });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    // ── Nullable wrapper recognition (Phase 2) ───────────────────────────

    #[test]
    fn nullable_wrapper_string_yields_to_string() {
        assert_eq!(
            coercion_target(&json!({"anyOf": [{"type": "null"}, {"type": "string"}]})),
            Some(CoercionTarget::ToString)
        );
    }

    #[test]
    fn nullable_wrapper_number_yields_to_number() {
        assert_eq!(
            coercion_target(&json!({"anyOf": [{"type": "null"}, {"type": "number"}]})),
            Some(CoercionTarget::ToNumber)
        );
    }

    #[test]
    fn nullable_wrapper_boolean_yields_to_boolean() {
        assert_eq!(
            coercion_target(&json!({"anyOf": [{"type": "null"}, {"type": "boolean"}]})),
            Some(CoercionTarget::ToBoolean)
        );
    }

    #[test]
    fn nullable_wrapper_file_yields_to_string() {
        // Bare optional `file` lowers to the lazy `darkmatter-file-reference`
        // inside the 3-arm nullable wrapper; coercion only cares that the file
        // arm is a string, so the result is `ToString` regardless of which
        // file format the arm carries.
        let frag = json!({
            "anyOf": [
                {"type": "null"},
                {"const": ""},
                {"type": "string", "format": "darkmatter-file-reference"}
            ]
        });
        assert_eq!(coercion_target(&frag), Some(CoercionTarget::ToString));
    }

    #[test]
    fn nullable_wrapper_array_yields_array_target() {
        let frag = json!({
            "anyOf": [
                {"type": "null"},
                {"type": "array", "items": {"type": "boolean"}}
            ]
        });
        assert_eq!(
            coercion_target(&frag),
            Some(CoercionTarget::Array(Box::new(CoercionTarget::ToBoolean)))
        );
    }

    #[test]
    fn nullable_wrapper_inline_object_yields_object_target() {
        let frag = json!({
            "anyOf": [
                {"type": "null"},
                {
                    "type": "object",
                    "properties": {"enabled": {"type": "boolean"}},
                    "additionalProperties": false
                }
            ]
        });
        match coercion_target(&frag) {
            Some(CoercionTarget::Object(props)) => {
                assert_eq!(props, vec![("enabled".to_string(), CoercionTarget::ToBoolean)]);
            }
            other => panic!("expected Object target, got {other:?}"),
        }
    }

    #[test]
    fn nullable_wrapper_boolish_yields_to_boolean() {
        let frag = json!({
            "anyOf": [
                {"type": "null"},
                {
                    "anyOf": [
                        {"type": "boolean"},
                        {"enum": ["true", "false", "True", "False", "TRUE", "FALSE"]}
                    ]
                }
            ]
        });
        assert_eq!(coercion_target(&frag), Some(CoercionTarget::ToBoolean));
    }

    #[test]
    fn nullable_wrapper_numberlike_yields_to_number() {
        let frag = json!({
            "anyOf": [
                {"type": "null"},
                {
                    "anyOf": [
                        {"type": "number"},
                        {"type": "string", "pattern": r"^-?\d+(\.\d+)?$"}
                    ]
                }
            ]
        });
        assert_eq!(coercion_target(&frag), Some(CoercionTarget::ToNumber));
    }

    #[test]
    fn nullable_wrapper_unrelated_union_is_none() {
        let unrelated = json!({"anyOf": [{"type": "null"}, {"type": "object"}]});
        assert_eq!(coercion_target(&unrelated), None);
    }

    #[test]
    fn optional_scalar_coerces_through_null_wrapper_and_preserves_null() {
        let schema = json!({
            "type": "object",
            "properties": {
                "s": {"anyOf": [{"type": "null"}, {"type": "string"}]},
                "n": {"anyOf": [{"type": "null"}, {"type": "number"}]},
                "b": {"anyOf": [{"type": "null"}, {"type": "boolean"}]}
            }
        });
        let instance = json!({ "s": 7, "n": "42", "b": "true", "missing": null });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["s"], json!("7"));
        assert_eq!(outcome.value["n"], json!(42));
        assert_eq!(outcome.value["b"], json!(true));
        assert_eq!(outcome.value["missing"], Value::Null);
    }

    #[test]
    fn optional_string_null_is_preserved_unchanged() {
        let schema = json!({
            "type": "object",
            "properties": {
                "opt": {"anyOf": [{"type": "null"}, {"type": "string"}]}
            }
        });
        let instance = json!({ "opt": null });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, instance);
    }

    #[test]
    fn optional_file_null_and_empty_are_preserved() {
        let schema = json!({
            "type": "object",
            "properties": {
                "file": {
                    "anyOf": [
                        {"type": "null"},
                        {"const": ""},
                        {"type": "string", "format": "darkmatter-file-reference"}
                    ]
                }
            }
        });
        let null_instance = json!({ "file": null });
        let outcome = coerce_frontmatter(&schema, &null_instance);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, null_instance);

        let empty_instance = json!({ "file": "" });
        let outcome2 = coerce_frontmatter(&schema, &empty_instance);
        assert!(!outcome2.changed);
        assert_eq!(outcome2.value, empty_instance);
    }

    #[test]
    fn optional_boolish_and_numberlike_coerce_through_null_wrapper() {
        let schema = json!({
            "type": "object",
            "properties": {
                "flag": {
                    "anyOf": [
                        {"type": "null"},
                        {
                            "anyOf": [
                                {"type": "boolean"},
                                {"enum": ["true", "false", "True", "False", "TRUE", "FALSE"]}
                            ]
                        }
                    ]
                },
                "n": {
                    "anyOf": [
                        {"type": "null"},
                        {
                            "anyOf": [
                                {"type": "number"},
                                {"type": "string", "pattern": r"^-?\d+(\.\d+)?$"}
                            ]
                        }
                    ]
                }
            }
        });
        let instance = json!({ "flag": "false", "n": "99" });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["flag"], json!(false));
        assert_eq!(outcome.value["n"], json!(99));
    }

    #[test]
    fn optional_array_coerces_element_wise_and_preserves_null() {
        let schema = json!({
            "type": "object",
            "properties": {
                "tags": {
                    "anyOf": [
                        {"type": "null"},
                        {"type": "array", "items": {"type": "boolean"}}
                    ]
                }
            }
        });
        let instance = json!({ "tags": ["true", "false"] });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["tags"], json!([true, false]));

        let null_instance = json!({ "tags": null });
        let outcome2 = coerce_frontmatter(&schema, &null_instance);
        assert!(!outcome2.changed);
        assert_eq!(outcome2.value, null_instance);
    }

    #[test]
    fn optional_inline_object_coerces_properties_and_preserves_null() {
        let schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "anyOf": [
                        {"type": "null"},
                        {
                            "type": "object",
                            "properties": {
                                "enabled": {"type": "boolean"},
                                "retries": {"type": "number"}
                            },
                            "additionalProperties": false
                        }
                    ]
                }
            }
        });
        let instance = json!({ "config": { "enabled": "true", "retries": "3" } });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert_eq!(outcome.value["config"]["enabled"], json!(true));
        assert_eq!(outcome.value["config"]["retries"], json!(3));

        let null_instance = json!({ "config": null });
        let outcome2 = coerce_frontmatter(&schema, &null_instance);
        assert!(!outcome2.changed);
        assert_eq!(outcome2.value, null_instance);
    }

    #[test]
    fn optional_property_level_union_runs_per_arm_coercion() {
        let schema = json!({
            "type": "object",
            "properties": {
                "count": {
                    "anyOf": [
                        {"type": "null"},
                        {"anyOf": [{"type": "number"}, {"type": "string"}]}
                    ]
                }
            }
        });

        let ambiguous = json!({ "count": "42" });
        let outcome = coerce_frontmatter(&schema, &ambiguous);
        assert!(!outcome.changed);
        assert_eq!(outcome.value, ambiguous);

        let unambiguous = json!({ "count": true });
        let outcome2 = coerce_frontmatter(&schema, &unambiguous);
        assert!(outcome2.changed);
        assert_eq!(outcome2.value["count"], json!("true"));

        let null_instance = json!({ "count": null });
        let outcome3 = coerce_frontmatter(&schema, &null_instance);
        assert!(!outcome3.changed);
        assert_eq!(outcome3.value, null_instance);
    }

    // ── Content-format string types (Feature D, Phase 5) ────────────────

    #[test]
    fn recognizes_yaml_and_json_content_formats() {
        assert_eq!(
            coercion_target(&json!({"type": "string", "format": "darkmatter-yaml"})),
            Some(CoercionTarget::ToYamlString)
        );
        assert_eq!(
            coercion_target(&json!({"type": "string", "format": "darkmatter-json"})),
            Some(CoercionTarget::ToJsonString)
        );
        // A string with any other (or no) format keeps the plain ToString
        // coercion so `file`/`email`/`url` fields are unaffected.
        assert_eq!(
            coercion_target(&json!({"type": "string", "format": "email"})),
            Some(CoercionTarget::ToString)
        );
    }

    #[test]
    fn yaml_string_and_null_are_left_untouched() {
        // An already-string value flows straight to the `format` validator; a
        // null value is the optional-property "absent" sentinel.
        assert_eq!(coerce_to_yaml_string(&json!("title: Foo")), None);
        assert_eq!(coerce_to_yaml_string(&Value::Null), None);
        assert_eq!(coerce_to_json_string(&json!("{}")), None);
        assert_eq!(coerce_to_json_string(&Value::Null), None);
    }

    #[test]
    fn native_mapping_coerces_to_yaml_and_json_strings() {
        let mapping = json!({ "title": "Foo", "tags": ["a", "b"] });
        let yaml = coerce_to_yaml_string(&mapping).expect("yaml coercion");
        // The serialized string round-trips through a YAML parse.
        assert!(serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml.as_str().unwrap()).is_ok());
        assert!(!yaml.as_str().unwrap().ends_with('\n'), "trailing newline trimmed");

        let jsonv = coerce_to_json_string(&mapping).expect("json coercion");
        // Compare by re-parse rather than exact bytes: serde_json map ordering
        // depends on build features.
        let reparsed: Value = serde_json::from_str(jsonv.as_str().unwrap()).expect("valid json");
        assert_eq!(reparsed, mapping);
    }

    #[test]
    fn native_sequence_coerces_to_content_format_string() {
        let seq = json!([1, 2, 3]);
        let yaml = coerce_to_yaml_string(&seq).expect("yaml coercion");
        assert!(serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml.as_str().unwrap()).is_ok());
        assert_eq!(coerce_to_json_string(&seq), Some(json!("[1,2,3]")));
    }

    #[test]
    fn native_scalar_coerces_to_content_format_string() {
        // Numbers and booleans serialize to their format-string form.
        assert_eq!(coerce_to_json_string(&json!(42)), Some(json!("42")));
        assert_eq!(coerce_to_json_string(&json!(true)), Some(json!("true")));
        let yaml = coerce_to_yaml_string(&json!(42)).expect("yaml coercion");
        assert!(serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml.as_str().unwrap()).is_ok());
    }

    #[test]
    fn content_format_coercion_is_idempotent() {
        let schema = json!({
            "type": "object",
            "properties": {
                "frontmatter": {"type": "string", "format": "darkmatter-yaml"}
            }
        });
        let instance = json!({ "frontmatter": { "title": "Foo" } });
        let first = coerce_frontmatter(&schema, &instance);
        assert!(first.changed);
        let second = coerce_frontmatter(&schema, &first.value);
        assert!(!second.changed, "second run should be a no-op");
        assert_eq!(first.value, second.value);
    }

    #[test]
    fn coerce_value_against_schema_serializes_native_values() {
        // A lone value/schema pair (an example artifact's `returns` against its
        // annotated target) serializes a native mapping/sequence to the format's
        // string form, so a `type: string` validator then accepts it.
        let yaml_target = json!({ "type": "string", "format": "darkmatter-yaml" });
        let mapping = coerce_value_against_schema(&yaml_target, &json!({ "title": "Foo" }));
        assert!(mapping.is_string(), "native mapping must coerce to a yaml string");

        let json_target = json!({ "type": "string", "format": "darkmatter-json" });
        assert_eq!(
            coerce_value_against_schema(&json_target, &json!([1, 2, 3])),
            json!("[1,2,3]")
        );
    }

    #[test]
    fn coerce_value_against_schema_passes_through_string_and_out_of_matrix() {
        // An already-string value is returned unchanged (no double-encode), and a
        // schema outside the coercion matrix (opaque `object`) yields a plain clone.
        let yaml_target = json!({ "type": "string", "format": "darkmatter-yaml" });
        assert_eq!(
            coerce_value_against_schema(&yaml_target, &json!("title: Foo")),
            json!("title: Foo")
        );
        let opaque = json!({ "type": "object" });
        let value = json!({ "k": "v" });
        assert_eq!(coerce_value_against_schema(&opaque, &value), value);
    }

    #[test]
    fn native_content_format_object_pass_coerces_and_is_non_mutating() {
        // The object pass serializes a native mapping for a `yaml` field on a
        // fresh copy; the caller's instance is never mutated.
        let schema = json!({
            "type": "object",
            "properties": {
                "frontmatter": {"type": "string", "format": "darkmatter-yaml"}
            }
        });
        let instance = json!({ "frontmatter": { "title": "Foo" } });
        let outcome = coerce_frontmatter(&schema, &instance);
        assert!(outcome.changed);
        assert!(outcome.value["frontmatter"].is_string());
        // Non-mutating: the original instance still holds the native mapping.
        assert_eq!(instance, json!({ "frontmatter": { "title": "Foo" } }));
    }
}
