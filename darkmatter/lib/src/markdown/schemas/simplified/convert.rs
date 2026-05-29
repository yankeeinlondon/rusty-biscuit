//! SimplifiedSchema → Draft 2020-12 JSON Schema conversion.
//!
//! Lowers a parsed [`SimplifiedSchema`] AST (see [`super::types`]) into a
//! `serde_json::Value` shaped as a Draft 2020-12 JSON Schema. The output is
//! deterministic (keys are emitted into a `serde_json::Map`, whose ordering is
//! stable per the active `serde_json` feature flags) and uses no IO — file
//! references and external resolution are deferred to Phase 3.
//!
//! ## Behavioural Highlights
//!
//! - **Property-level unions** hoist `required` and `default(...)` from every
//!   arm onto the parent property. Arm-local constraints stay arm-local. A
//!   `default(...)` that disagrees across arms is a hard conversion error.
//! - **`x-darkmatter-*` extensions** are emitted as plain JSON Schema
//!   annotation keys (`x-darkmatter-match` for `file(match(...))`,
//!   `x-darkmatter-url-scheme` for `url(scheme(...))`); the custom format
//!   validators consume them in Phase 3.
//! - **Root-level unions** with unresolved [`SchemaArm::FileRef`] arms fail
//!   with [`SchemaError::Convert`] — Phase 3's resolver is expected to inline
//!   them before invoking the converter.
//!
//! See the mapping table in `darkmatter/features/2026-05-11-schemas/spec.md`
//! for the full row-by-row contract; the [`tests::snapshots`] module in this
//! file mirrors that table.

use serde_json::{Map, Value, json};

use super::types::{
    Constraint, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema, SimplifiedType,
};
use crate::markdown::schemas::errors::SchemaError;

/// Draft 2020-12 schema URI emitted on every generated root schema.
pub const DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

/// String spellings a `boolish` field accepts (and that coercion maps to a
/// real boolean). Single source of truth shared with [`super::super::coerce`].
pub(in crate::markdown::schemas) const BOOLISH_VALUES: [&str; 6] =
    ["true", "false", "True", "False", "TRUE", "FALSE"];

/// Pattern the `numberlike` string arm carries — the source for the schema the
/// validator compiles.
pub(super) const NUMBERLIKE_PATTERN: &str = r"^-?\d+(\.\d+)?$";

/// Converts a [`SimplifiedSchema`] into a Draft 2020-12 JSON Schema value.
///
/// ## Errors
///
/// Returns [`SchemaError::Convert`] when:
///
/// - a constraint is applied to a type that doesn't accept it
///   (e.g. `integer` on a `string`),
/// - a property-level union has conflicting `default(...)` values,
/// - a root-level union arm is an unresolved [`SchemaArm::FileRef`].
///
/// ## Examples
///
/// ```ignore
/// # use darkmatter::markdown::schemas::simplified::parse_yaml_schema;
/// # use darkmatter::markdown::schemas::simplified::convert::to_json_schema;
/// let yaml = serde_yaml_ng::from_str("title: 'string(required)'").unwrap();
/// let schema = parse_yaml_schema(&yaml).unwrap();
/// let json = to_json_schema(&schema).unwrap();
/// assert_eq!(json["type"], "object");
/// ```
pub fn to_json_schema(schema: &SimplifiedSchema) -> Result<Value, SchemaError> {
    let mut root = match schema {
        SimplifiedSchema::Single(shape) => shape_to_object_schema(shape)?,
        SimplifiedSchema::Union(arms) => union_to_root_schema(arms)?,
    };
    root.insert("$schema".into(), Value::String(DRAFT_2020_12.into()));
    Ok(Value::Object(root))
}

// ── Shape & root union ────────────────────────────────────────────────────

fn shape_to_object_schema(shape: &SchemaShape) -> Result<Map<String, Value>, SchemaError> {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for (name, def) in &shape.properties {
        let (prop_schema, is_required) = property_def_to_schema(name, def)?;
        if is_required {
            required.push(Value::String(name.clone()));
        }
        properties.insert(name.clone(), prop_schema);
    }

    let mut obj = Map::new();
    obj.insert("type".into(), Value::String("object".into()));
    obj.insert("additionalProperties".into(), Value::Bool(true));
    obj.insert("properties".into(), Value::Object(properties));
    if !required.is_empty() {
        obj.insert("required".into(), Value::Array(required));
    }
    Ok(obj)
}

fn union_to_root_schema(arms: &[SchemaArm]) -> Result<Map<String, Value>, SchemaError> {
    let mut any_of = Vec::with_capacity(arms.len());
    for (idx, arm) in arms.iter().enumerate() {
        match arm {
            SchemaArm::Inline(shape) => {
                any_of.push(Value::Object(shape_to_object_schema(shape)?));
            }
            SchemaArm::FileRef(path) => {
                return Err(SchemaError::Convert {
                    property: format!("<arm[{idx}]>"),
                    message: format!(
                        "root-union file reference `{path}` must be resolved before \
                         conversion (handled by the resolution layer)"
                    ),
                });
            }
        }
    }
    let mut obj = Map::new();
    obj.insert("anyOf".into(), Value::Array(any_of));
    Ok(obj)
}

// ── Property-level dispatch (single vs union) ────────────────────────────

fn property_def_to_schema(name: &str, def: &PropertyDef) -> Result<(Value, bool), SchemaError> {
    match def {
        PropertyDef::Single(atom) => atom_to_schema(name, atom),
        PropertyDef::Union(arms) => union_property_to_schema(name, arms),
    }
}

fn union_property_to_schema(
    name: &str,
    arms: &[PropertyAtom],
) -> Result<(Value, bool), SchemaError> {
    let mut any_of = Vec::with_capacity(arms.len());
    let mut required = false;
    let mut hoisted_default: Option<Value> = None;

    for atom in arms {
        let (arm_value, arm_required) = atom_to_schema(name, atom)?;
        if arm_required {
            required = true;
        }
        // Hoist any `default` key off the arm. `atom_to_schema` always returns
        // a `Value::Object` for the converted fragment.
        let mut arm_value = arm_value;
        if let Value::Object(map) = &mut arm_value
            && let Some(arm_default) = map.remove("default")
        {
            match &hoisted_default {
                Some(existing) if existing != &arm_default => {
                    return Err(SchemaError::Convert {
                        property: name.to_string(),
                        message: format!(
                            "conflicting `default(...)` values across union arms: \
                             {existing} vs {arm_default}"
                        ),
                    });
                }
                Some(_) => { /* identical default — collapse silently */ }
                None => hoisted_default = Some(arm_default),
            }
        }
        any_of.push(arm_value);
    }

    let mut union_schema = Map::new();
    union_schema.insert("anyOf".into(), Value::Array(any_of));
    if let Some(default_val) = hoisted_default {
        union_schema.insert("default".into(), default_val);
    }
    Ok((Value::Object(union_schema), required))
}

// ── Atom → JSON Schema fragment ──────────────────────────────────────────

fn atom_to_schema(name: &str, atom: &PropertyAtom) -> Result<(Value, bool), SchemaError> {
    // Required is hoisted out; default may appear on either constraints
    // (single/value-level) or array_constraints (array-level). The last write
    // wins, mirroring how a user reading left-to-right would expect.
    let mut required = false;
    let mut default_val: Option<Value> = None;
    for c in atom.constraints.iter().chain(atom.array_constraints.iter()) {
        match c {
            Constraint::Required => required = true,
            Constraint::Default(v) => default_val = Some(normalize_json_number(v.clone())),
            _ => {}
        }
    }

    let inner = type_fragment(name, atom.ty, &atom.constraints)?;

    let mut schema = if atom.is_array {
        let mut arr = Map::new();
        arr.insert("type".into(), Value::String("array".into()));
        arr.insert("items".into(), inner);
        apply_array_constraints(name, &mut arr, &atom.array_constraints)?;
        Value::Object(arr)
    } else {
        inner
    };

    // Attach default + description on the outermost object. Every type
    // fragment we emit is an `Object` so this match is exhaustive in practice.
    if let Value::Object(map) = &mut schema {
        if let Some(d) = default_val {
            map.insert("default".into(), d);
        }
        if let Some(desc) = &atom.description {
            map.insert("description".into(), Value::String(desc.clone()));
        }
    }

    Ok((schema, required))
}

fn apply_array_constraints(
    name: &str,
    arr: &mut Map<String, Value>,
    constraints: &[Constraint],
) -> Result<(), SchemaError> {
    for c in constraints {
        match c {
            Constraint::Required | Constraint::Default(_) => {
                // Hoisted to the property level by `atom_to_schema`.
            }
            Constraint::MinItems(n) => {
                arr.insert("minItems".into(), json!(*n));
            }
            Constraint::MaxItems(n) => {
                arr.insert("maxItems".into(), json!(*n));
            }
            Constraint::Unique => {
                arr.insert("uniqueItems".into(), Value::Bool(true));
            }
            other => {
                return Err(SchemaError::Convert {
                    property: name.to_string(),
                    message: format!(
                        "constraint `{}` is not valid at the array level",
                        other.keyword()
                    ),
                });
            }
        }
    }
    Ok(())
}

// ── Per-type fragment builders ───────────────────────────────────────────

fn type_fragment(
    name: &str,
    ty: SimplifiedType,
    constraints: &[Constraint],
) -> Result<Value, SchemaError> {
    match ty {
        SimplifiedType::String => string_fragment(name, constraints),
        SimplifiedType::Date => date_family_fragment(name, "date", constraints),
        SimplifiedType::DateTime => date_family_fragment(name, "date-time", constraints),
        SimplifiedType::Time => date_family_fragment(name, "time", constraints),
        SimplifiedType::Number => number_fragment(name, constraints),
        SimplifiedType::NumberLike => numberlike_fragment(name, constraints),
        SimplifiedType::Boolean => simple_typed_fragment(name, "boolean", constraints),
        SimplifiedType::Boolish => boolish_fragment(name, constraints),
        SimplifiedType::Object => simple_typed_fragment(name, "object", constraints),
        SimplifiedType::File => file_fragment(name, constraints),
        SimplifiedType::Enum => enum_fragment(name, constraints),
        SimplifiedType::Url => url_fragment(name, constraints),
        SimplifiedType::Email => email_fragment(name, constraints),
        SimplifiedType::Any => any_fragment(name, constraints),
    }
}

fn string_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    let mut m = Map::new();
    m.insert("type".into(), Value::String("string".into()));
    for c in constraints {
        match c {
            Constraint::Required | Constraint::Default(_) => {}
            Constraint::MinLen(n) => {
                m.insert("minLength".into(), json!(*n));
            }
            Constraint::MaxLen(n) => {
                m.insert("maxLength".into(), json!(*n));
            }
            Constraint::NotEmpty => {
                // Anchored, lookaround-free: any string containing at least
                // one non-whitespace character. The Rust `regex` crate (used
                // by jsonschema for ReDoS-safe pattern evaluation) rejects
                // `(?!...)` lookahead, so the previous `^(?!\s*$).+` form is
                // not portable.
                m.insert("pattern".into(), Value::String(r"\S".into()));
            }
            Constraint::Pattern(p) => {
                m.insert("pattern".into(), Value::String(p.clone()));
            }
            other => return Err(invalid_constraint(name, "string", other)),
        }
    }
    Ok(Value::Object(m))
}

fn date_family_fragment(
    name: &str,
    fmt: &str,
    constraints: &[Constraint],
) -> Result<Value, SchemaError> {
    reject_unsupported(name, fmt, constraints, &[])?;
    Ok(json!({ "type": "string", "format": fmt }))
}

fn number_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    let mut m = Map::new();
    let mut is_integer = false;
    for c in constraints {
        match c {
            Constraint::Required | Constraint::Default(_) => {}
            Constraint::Min(n) => {
                m.insert("minimum".into(), number_to_json(*n));
            }
            Constraint::Max(n) => {
                m.insert("maximum".into(), number_to_json(*n));
            }
            Constraint::Integer => is_integer = true,
            other => return Err(invalid_constraint(name, "number", other)),
        }
    }
    m.insert(
        "type".into(),
        Value::String(if is_integer { "integer" } else { "number" }.into()),
    );
    Ok(Value::Object(m))
}

fn numberlike_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    reject_unsupported(name, "numberlike", constraints, &[])?;
    Ok(json!({
        "anyOf": [
            { "type": "number" },
            { "type": "string", "pattern": NUMBERLIKE_PATTERN }
        ]
    }))
}

fn boolish_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    reject_unsupported(name, "boolish", constraints, &[])?;
    let enum_members: Vec<Value> = BOOLISH_VALUES
        .iter()
        .map(|s| Value::String((*s).into()))
        .collect();
    Ok(json!({
        "anyOf": [
            { "type": "boolean" },
            { "enum": enum_members }
        ]
    }))
}

fn simple_typed_fragment(
    name: &str,
    ty: &str,
    constraints: &[Constraint],
) -> Result<Value, SchemaError> {
    reject_unsupported(name, ty, constraints, &[])?;
    Ok(json!({ "type": ty }))
}

fn file_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    let mut m = Map::new();
    m.insert("type".into(), Value::String("string".into()));
    m.insert("format".into(), Value::String("darkmatter-file".into()));
    for c in constraints {
        match c {
            Constraint::Required | Constraint::Default(_) => {}
            Constraint::Match(globs) => {
                m.insert(
                    "x-darkmatter-match".into(),
                    Value::Array(globs.iter().cloned().map(Value::String).collect()),
                );
            }
            other => return Err(invalid_constraint(name, "file", other)),
        }
    }
    Ok(Value::Object(m))
}

fn enum_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    let mut members: Option<Vec<String>> = None;
    for c in constraints {
        match c {
            Constraint::Required | Constraint::Default(_) => {}
            Constraint::Members(m) => members = Some(m.clone()),
            other => return Err(invalid_constraint(name, "enum", other)),
        }
    }
    let members = members.ok_or_else(|| SchemaError::Convert {
        property: name.to_string(),
        message: "`enum` requires at least one member".into(),
    })?;
    let mut m = Map::new();
    m.insert(
        "enum".into(),
        Value::Array(members.into_iter().map(Value::String).collect()),
    );
    Ok(Value::Object(m))
}

fn url_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    let mut m = Map::new();
    m.insert("type".into(), Value::String("string".into()));
    m.insert("format".into(), Value::String("uri".into()));
    for c in constraints {
        match c {
            Constraint::Required | Constraint::Default(_) => {}
            Constraint::Scheme(schemes) => {
                m.insert(
                    "x-darkmatter-url-scheme".into(),
                    Value::Array(schemes.iter().cloned().map(Value::String).collect()),
                );
            }
            other => return Err(invalid_constraint(name, "url", other)),
        }
    }
    Ok(Value::Object(m))
}

fn email_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    reject_unsupported(name, "email", constraints, &[])?;
    Ok(json!({ "type": "string", "format": "email" }))
}

fn any_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    reject_unsupported(name, "any", constraints, &[])?;
    Ok(Value::Object(Map::new()))
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// For types that only accept the universal `required` / `default` constraints,
/// reject anything else. `_extra_allowed` is reserved for future extensions.
fn reject_unsupported(
    name: &str,
    type_label: &str,
    constraints: &[Constraint],
    _extra_allowed: &[&str],
) -> Result<(), SchemaError> {
    for c in constraints {
        match c {
            Constraint::Required | Constraint::Default(_) => {}
            other => return Err(invalid_constraint(name, type_label, other)),
        }
    }
    Ok(())
}

fn invalid_constraint(property: &str, ty: &str, c: &Constraint) -> SchemaError {
    SchemaError::Convert {
        property: property.to_string(),
        message: format!("constraint `{}` is not valid on `{ty}`", c.keyword()),
    }
}

/// Emit a JSON number that prefers integer form when the value is integral.
/// This keeps snapshots tidy (`0` instead of `0.0`).
fn number_to_json(n: f64) -> Value {
    if n.is_finite() && n.fract() == 0.0 && (i64::MIN as f64..=i64::MAX as f64).contains(&n) {
        json!(n as i64)
    } else if let Some(num) = serde_json::Number::from_f64(n) {
        Value::Number(num)
    } else {
        Value::String(format!("{n}"))
    }
}

/// Normalises numeric values inside a default literal so integral floats
/// (the form the parser emits for `default(3)`) render as `3` rather than
/// `3.0`. Recurses into arrays/objects defensively even though v1 grammar
/// only emits scalars.
fn normalize_json_number(value: Value) -> Value {
    match value {
        Value::Number(n) => {
            if let Some(f) = n.as_f64()
                && f.is_finite()
                && f.fract() == 0.0
                && (i64::MIN as f64..=i64::MAX as f64).contains(&f)
            {
                json!(f as i64)
            } else {
                Value::Number(n)
            }
        }
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_json_number).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, normalize_json_number(v)))
                .collect(),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::simplified::{grammar::parse_type_expr, parse_yaml_schema};

    fn parse(yaml: &str) -> SimplifiedSchema {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("yaml parse");
        parse_yaml_schema(&v).expect("schema parse")
    }

    fn convert(yaml: &str) -> Value {
        to_json_schema(&parse(yaml)).expect("convert")
    }

    fn atom_value(input: &str) -> Value {
        let atom = parse_type_expr("test", input).expect("parse atom");
        atom_to_schema("test", &atom).expect("convert").0
    }

    #[test]
    fn single_shape_has_root_object_and_schema_uri() {
        let v = convert("name: string");
        assert_eq!(v["$schema"], DRAFT_2020_12);
        assert_eq!(v["type"], "object");
        assert_eq!(v["additionalProperties"], true);
        assert!(v.get("properties").is_some());
        assert!(v.get("required").is_none(), "no required props expected");
    }

    #[test]
    fn required_property_is_listed() {
        let v = convert("name: 'string(required)'");
        let required = v["required"].as_array().expect("required array");
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "name");
    }

    #[test]
    fn string_min_max_become_lengths() {
        let v = atom_value("string(min(5); max(80))");
        assert_eq!(v["minLength"], 5);
        assert_eq!(v["maxLength"], 80);
    }

    #[test]
    fn not_empty_emits_pattern() {
        let v = atom_value("string(not-empty)");
        assert_eq!(v["pattern"], r"\S");
    }

    #[test]
    fn pattern_is_preserved() {
        let v = atom_value("string(pattern(^[a-z]+$))");
        assert_eq!(v["pattern"], "^[a-z]+$");
    }

    #[test]
    fn date_family_emits_format() {
        assert_eq!(atom_value("date")["format"], "date");
        assert_eq!(atom_value("datetime")["format"], "date-time");
        assert_eq!(atom_value("time")["format"], "time");
    }

    #[test]
    fn number_min_max_are_values_and_integer_flips_type() {
        let v = atom_value("number(min(0); max(100))");
        assert_eq!(v["type"], "number");
        assert_eq!(v["minimum"], 0);
        assert_eq!(v["maximum"], 100);

        let v = atom_value("number(integer; min(1))");
        assert_eq!(v["type"], "integer");
        assert_eq!(v["minimum"], 1);
    }

    #[test]
    fn numberlike_is_any_of() {
        let v = atom_value("numberlike");
        let arms = v["anyOf"].as_array().expect("anyOf");
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0]["type"], "number");
        assert_eq!(arms[1]["type"], "string");
        assert_eq!(arms[1]["pattern"], r"^-?\d+(\.\d+)?$");
    }

    #[test]
    fn boolish_is_any_of() {
        let v = atom_value("boolish");
        let arms = v["anyOf"].as_array().expect("anyOf");
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0]["type"], "boolean");
        assert!(arms[1]["enum"].is_array());
    }

    #[test]
    fn object_is_typed() {
        let v = atom_value("object");
        assert_eq!(v["type"], "object");
    }

    #[test]
    fn file_emits_format_and_match_extension() {
        let v = atom_value("file(match('*.md', '!_*.md'))");
        assert_eq!(v["type"], "string");
        assert_eq!(v["format"], "darkmatter-file");
        let globs = v["x-darkmatter-match"].as_array().unwrap();
        assert_eq!(globs[0], "*.md");
        assert_eq!(globs[1], "!_*.md");
    }

    #[test]
    fn enum_emits_enum_array() {
        let v = atom_value("enum(red,green,blue)");
        let members = v["enum"].as_array().unwrap();
        assert_eq!(members.len(), 3);
        assert_eq!(members[0], "red");
    }

    #[test]
    fn url_emits_uri_format_and_scheme_extension() {
        let v = atom_value("url(scheme(https, http))");
        assert_eq!(v["format"], "uri");
        let schemes = v["x-darkmatter-url-scheme"].as_array().unwrap();
        assert_eq!(schemes[0], "https");
        assert_eq!(schemes[1], "http");
    }

    #[test]
    fn email_emits_email_format() {
        let v = atom_value("email");
        assert_eq!(v["format"], "email");
    }

    #[test]
    fn any_is_empty_object() {
        let v = atom_value("any");
        assert!(v.as_object().unwrap().is_empty());
    }

    #[test]
    fn array_suffix_wraps_items() {
        let v = atom_value("string[]");
        assert_eq!(v["type"], "array");
        assert_eq!(v["items"]["type"], "string");
    }

    #[test]
    fn array_constraints_become_min_max_unique_items() {
        let v = atom_value("string(pattern(^[a-z]+$))[](min(1); max(5); unique)");
        assert_eq!(v["type"], "array");
        assert_eq!(v["items"]["pattern"], "^[a-z]+$");
        assert_eq!(v["minItems"], 1);
        assert_eq!(v["maxItems"], 5);
        assert_eq!(v["uniqueItems"], true);
    }

    #[test]
    fn file_array_with_min_items() {
        let v = atom_value("file(match('*.png'))[](min(1))");
        assert_eq!(v["type"], "array");
        assert_eq!(v["items"]["format"], "darkmatter-file");
        assert_eq!(v["minItems"], 1);
    }

    #[test]
    fn description_lands_on_outer_schema() {
        let v = atom_value("string -> The author's full name");
        assert_eq!(v["description"], "The author's full name");

        let v = atom_value("string[] -> A list of tags");
        assert_eq!(v["type"], "array");
        assert_eq!(v["description"], "A list of tags");
    }

    #[test]
    fn default_emitted_at_property_level_for_atoms() {
        let v = atom_value("string(default(hello))");
        assert_eq!(v["default"], "hello");

        let v = atom_value("number(default(3))");
        assert_eq!(v["default"], 3);

        let v = atom_value("boolean(default(true))");
        assert_eq!(v["default"], true);
    }

    #[test]
    fn union_property_emits_any_of_and_hoists_required_default() {
        let yaml = r#"
title:
  - "string(required; not-empty)"
  - "number"
"#;
        let v = convert(yaml);
        let required = v["required"].as_array().unwrap();
        assert_eq!(required[0], "title");

        let title = &v["properties"]["title"];
        let arms = title["anyOf"].as_array().unwrap();
        assert_eq!(arms.len(), 2);
        // not-empty stays on the string arm
        assert_eq!(arms[0]["pattern"], r"\S");
        // required is NOT emitted on the arm
        assert!(arms[0].get("required").is_none());
    }

    #[test]
    fn union_property_with_matching_defaults_collapses() {
        let yaml = r#"
flag:
  - "boolean(default(true))"
  - "boolish(default(true))"
"#;
        let v = convert(yaml);
        let flag = &v["properties"]["flag"];
        assert_eq!(flag["default"], true);
        // No arm should still carry the default.
        for arm in flag["anyOf"].as_array().unwrap() {
            assert!(arm.get("default").is_none(), "arm retained default: {arm}");
        }
    }

    #[test]
    fn union_property_with_conflicting_defaults_errors() {
        let yaml = r#"
flag:
  - "boolean(default(true))"
  - "boolish(default(false))"
"#;
        let err = to_json_schema(&parse(yaml)).unwrap_err();
        match err {
            SchemaError::Convert { property, message } => {
                assert_eq!(property, "flag");
                assert!(message.contains("conflicting"), "{message}");
            }
            other => panic!("expected Convert, got {other:?}"),
        }
    }

    #[test]
    fn root_union_inline_arms_become_any_of() {
        let yaml = r#"
- title: 'string(required)'
  body:  string
- name:  'string(required)'
"#;
        let v = convert(yaml);
        assert_eq!(v["$schema"], DRAFT_2020_12);
        let arms = v["anyOf"].as_array().unwrap();
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0]["type"], "object");
        assert_eq!(arms[0]["properties"]["title"]["type"], "string");
        assert_eq!(arms[0]["required"][0], "title");
        assert_eq!(arms[1]["properties"]["name"]["type"], "string");
    }

    #[test]
    fn root_union_with_unresolved_file_ref_errors() {
        let yaml = "- ./schemas/post.yaml\n- title: string\n";
        let err = to_json_schema(&parse(yaml)).unwrap_err();
        match err {
            SchemaError::Convert { property, message } => {
                assert!(property.starts_with("<arm[0]>"), "{property}");
                assert!(message.contains("must be resolved"), "{message}");
            }
            other => panic!("expected Convert, got {other:?}"),
        }
    }

    #[test]
    fn invalid_constraint_for_type_errors() {
        // `integer` is a number-only constraint but parses on a string atom.
        let atom = parse_type_expr("test", "string(integer)").unwrap();
        let err = atom_to_schema("test", &atom).unwrap_err();
        match err {
            SchemaError::Convert { property, message } => {
                assert_eq!(property, "test");
                assert!(message.contains("string"));
                assert!(message.contains("integer"));
            }
            other => panic!("expected Convert, got {other:?}"),
        }
    }

    #[test]
    fn array_level_invalid_constraint_errors() {
        // `pattern` is not legal at the array level.
        let atom = parse_type_expr("test", "string[](pattern(x))").unwrap();
        let err = atom_to_schema("test", &atom).unwrap_err();
        match err {
            SchemaError::Convert { message, .. } => {
                assert!(message.contains("array level"), "{message}");
            }
            other => panic!("expected Convert, got {other:?}"),
        }
    }

    #[test]
    fn property_order_is_preserved_in_declaration_order_only_when_serde_json_does() {
        // The AST preserves declaration order via IndexMap. Whether the
        // resulting `properties` JSON object preserves declaration order is a
        // function of the active `serde_json` features (default = sorted by
        // key, `preserve_order` = insertion). The converter simply inserts in
        // declaration order; this test documents that contract.
        let yaml = "zeta: string\nalpha: number\nmiddle: boolean";
        let v = convert(yaml);
        let props = v["properties"].as_object().unwrap();
        // All three properties round-trip regardless of ordering choice.
        assert!(props.contains_key("zeta"));
        assert!(props.contains_key("alpha"));
        assert!(props.contains_key("middle"));
    }
}
