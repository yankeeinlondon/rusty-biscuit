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
//!   annotation keys (`x-darkmatter-url-scheme` for `url(scheme(...))`); the
//!   custom format validators consume them. `file(match(...))` is the
//!   exception: `match` is suggestion metadata only, so it is **not** emitted
//!   into the compiled JSON Schema (completion reads the patterns from the
//!   simplified-schema atom instead).
//! - **`file` existence posture** is carried by the emitted `format`: bare
//!   `file` lowers to the lazy `darkmatter-file-reference` (syntax-only),
//!   `file(eager)` lowers to the eager `darkmatter-file` (resolve + exists).
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
    TypeExpr,
};
use crate::markdown::schemas::errors::SchemaError;
use crate::markdown::schemas::format::{DARKMATTER_FILE_FORMAT, DARKMATTER_FILE_REFERENCE_FORMAT};

/// Draft 2020-12 schema URI emitted on every generated root schema.
pub const DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

/// String spellings a `boolish` field accepts (and that coercion maps to a
/// real boolean). Single source of truth shared with [`super::super::coerce`].
pub(in crate::markdown::schemas) const BOOLISH_VALUES: [&str; 6] =
    ["true", "false", "True", "False", "TRUE", "FALSE"];

/// Pattern the `numberlike` string arm carries — the source for the schema the
/// validator compiles. Single source of truth shared with
/// [`super::super::coerce`], which matches it exactly when recognizing a
/// numberlike `anyOf` arm.
pub(in crate::markdown::schemas) const NUMBERLIKE_PATTERN: &str = r"^-?\d+(\.\d+)?$";

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
    // Union requiredness must be known *before* building arms: a required
    // union (any arm carries `required`) gets neither the property-level
    // `null` arm nor the optional-`file` empty-string tolerance. Deciding
    // this up front lets each arm be built with empty-`file` suppression so a
    // required union containing an otherwise-optional `file` arm still rejects
    // the `""` sentinel.
    let required = arms.iter().any(atom_is_required);

    let mut any_of = Vec::with_capacity(arms.len());
    let mut hoisted_default: Option<Value> = None;

    for atom in arms {
        // Use the bare fragment builder so the optional `null` wrapper is
        // applied once at the property level, not per arm. The optional-`file`
        // empty-string arm is allowed only when the union itself is optional.
        let (arm_value, _arm_required) =
            atom_fragment_without_null_wrap(name, atom, !required)?;
        // Hoist any `default` key off the arm. `atom_fragment_without_null_wrap`
        // always returns a `Value::Object` for the converted fragment.
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

    let schema = if required {
        Value::Object(union_schema)
    } else {
        // Lift the null arm into the existing anyOf array so the output
        // stays a single-level `anyOf: [null, arm1, arm2, ...]` rather than
        // nesting the whole union under a second anyOf wrapper.
        let mut arms = union_schema
            .remove("anyOf")
            .expect("union schema has anyOf")
            .as_array()
            .expect("anyOf is an array")
            .clone();
        arms.insert(0, json!({ "type": "null" }));
        union_schema.insert("anyOf".into(), Value::Array(arms));
        Value::Object(union_schema)
    };
    Ok((schema, required))
}

// ── Atom → JSON Schema fragment ──────────────────────────────────────────

/// Wraps a finished typed fragment so an optional property also accepts
/// JSON `null` as a sentinel for "absent".
fn wrap_optional_null(inner: Value) -> Value {
    json!({ "anyOf": [ { "type": "null" }, inner ] })
}

fn atom_to_schema(name: &str, atom: &PropertyAtom) -> Result<(Value, bool), SchemaError> {
    let (mut fragment, required) = atom_fragment_without_null_wrap(name, atom, true)?;

    // Required atoms are emitted unchanged. Optional atoms are wrapped so
    // that JSON `null` validates as "absent". The optional `file` case also
    // preserves the legacy empty-string sentinel.
    if required {
        return Ok((fragment, true));
    }

    // Pull annotations off the fragment so they land on the outer wrapper,
    // not the typed arm. This keeps required-atom output byte-for-byte
    // identical while allowing optional wrappers to carry their own metadata.
    let mut default_val: Option<Value> = None;
    let mut description_val: Option<Value> = None;
    if let Value::Object(map) = &mut fragment {
        default_val = map.remove("default");
        description_val = map.remove("description");
    }

    let is_optional_scalar_file = matches!(&atom.ty, TypeExpr::Primitive(SimplifiedType::File))
        && !atom.is_array;

    let mut schema = if is_optional_scalar_file {
        // The bare fragment is already `[{ "const": "" }, file-typed]`.
        // Prepend the null arm to keep a flat, single-level anyOf.
        let mut arms = fragment
            .get("anyOf")
            .and_then(|a| a.as_array())
            .expect("optional file fragment has anyOf")
            .clone();
        arms.insert(0, json!({ "type": "null" }));
        let mut m = Map::new();
        m.insert("anyOf".into(), Value::Array(arms));
        Value::Object(m)
    } else {
        wrap_optional_null(fragment)
    };

    if let Value::Object(map) = &mut schema {
        if let Some(d) = default_val {
            map.insert("default".into(), d);
        }
        if let Some(d) = description_val {
            map.insert("description".into(), d);
        }
    }

    Ok((schema, false))
}

/// Reports whether an atom carries the `required` constraint at either the
/// value or array level. Used to decide union requiredness before any arm is
/// built, so the per-arm empty-`file` sentinel can be suppressed for a
/// required union.
fn atom_is_required(atom: &PropertyAtom) -> bool {
    atom.constraints
        .iter()
        .chain(atom.array_constraints.iter())
        .any(|c| matches!(c, Constraint::Required))
}

/// Builds the bare atom fragment without the general optional `null` wrapper.
///
/// The returned fragment carries `default`/`description` annotations so that
/// property-level union arms can reuse the same construction path. Callers
/// that need the public optional wrapper must use [`atom_to_schema`].
///
/// `allow_optional_empty_file` gates the optional-`file` empty-string arm
/// (Decision A). It is `true` for single atoms and optional unions, but
/// `false` for a required union arm so the `""` sentinel never leaks into a
/// required property.
fn atom_fragment_without_null_wrap(
    name: &str,
    atom: &PropertyAtom,
    allow_optional_empty_file: bool,
) -> Result<(Value, bool), SchemaError> {
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

    let inner = match &atom.ty {
        TypeExpr::Primitive(ty) => type_fragment(name, *ty, &atom.constraints)?,
        TypeExpr::InlineObject(shape) => inline_object_fragment(name, shape, &atom.constraints)?,
    };

    // Decision A: a non-`required` scalar `file` field treats an empty string
    // as "absent" so a ternary like `spec: "{{ ... ? path : '' }}"` validates
    // when the optional file is missing. The empty arm wraps the unchanged
    // `darkmatter-file` fragment, so file typing (and SimplifiedSchema-driven
    // completions, which read `atom.ty`, not this JSON Schema) are preserved.
    // Required file fields keep the strict fragment and still reject empty.
    let optional_empty_file = allow_optional_empty_file
        && matches!(&atom.ty, TypeExpr::Primitive(SimplifiedType::File))
        && !atom.is_array
        && !required;

    let mut fragment = if atom.is_array {
        let mut arr = Map::new();
        arr.insert("type".into(), Value::String("array".into()));
        arr.insert("items".into(), inner);
        apply_array_constraints(name, &mut arr, &atom.array_constraints)?;
        Value::Object(arr)
    } else if optional_empty_file {
        json!({ "anyOf": [ { "const": "" }, inner ] })
    } else {
        inner
    };

    // Attach default + description on the fragment. For single atoms these
    // are moved to the optional wrapper by `atom_to_schema`; for union arms
    // `default` is hoisted and `description` stays arm-local.
    if let Value::Object(map) = &mut fragment {
        if let Some(d) = default_val {
            map.insert("default".into(), d);
        }
        if let Some(desc) = &atom.description {
            map.insert("description".into(), Value::String(desc.clone()));
        }
    }

    Ok((fragment, required))
}

/// Lowers an inline object `SchemaShape` to a Draft 2020-12 object fragment.
///
/// The fragment always emits `additionalProperties: false` (Decision #7):
/// authors reach for an inline object specifically to constrain the shape,
/// so silently accepting extra keys would defeat their intent. Nested
/// `required` constraints live on the fragment, not the parent property, so
/// `union_property_to_schema`'s hoisting logic cannot accidentally lift them
/// out of an inline object arm (Risk Mitigation Checkpoint 4).
fn inline_object_fragment(
    name: &str,
    shape: &SchemaShape,
    atom_constraints: &[Constraint],
) -> Result<Value, SchemaError> {
    // Inline object atoms accept only the universal `required` / `default`
    // constraints. Reject anything else with the same wording primitive
    // fragment builders use.
    for c in atom_constraints {
        match c {
            Constraint::Required | Constraint::Default(_) => {}
            other => {
                return Err(SchemaError::Convert {
                    property: name.to_string(),
                    message: format!(
                        "constraint `{}` is not valid on an inline object",
                        other.keyword()
                    ),
                });
            }
        }
    }

    let mut properties = Map::new();
    let mut required = Vec::new();

    for (prop_name, def) in &shape.properties {
        let (prop_schema, is_required) = property_def_to_schema(prop_name, def)?;
        if is_required {
            required.push(Value::String(prop_name.clone()));
        }
        properties.insert(prop_name.clone(), prop_schema);
    }

    let mut obj = Map::new();
    obj.insert("type".into(), Value::String("object".into()));
    obj.insert("additionalProperties".into(), Value::Bool(false));
    obj.insert("properties".into(), Value::Object(properties));
    if !required.is_empty() {
        obj.insert("required".into(), Value::Array(required));
    }
    Ok(Value::Object(obj))
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
    // `eager` flips the emitted `format` so the build-time existence decision
    // is legible to the (sibling-blind) jsonschema format closure: bare `file`
    // lowers to the lazy `darkmatter-file-reference` (syntax-only), while
    // `file(eager)` lowers to the eager `darkmatter-file` (resolve + exists).
    let eager = constraints.iter().any(|c| matches!(c, Constraint::Eager));
    let format = if eager {
        DARKMATTER_FILE_FORMAT
    } else {
        DARKMATTER_FILE_REFERENCE_FORMAT
    };
    let mut m = Map::new();
    m.insert("type".into(), Value::String("string".into()));
    m.insert("format".into(), Value::String(format.into()));
    for c in constraints {
        match c {
            Constraint::Required | Constraint::Default(_) => {}
            // Consumed above to pick the format; nothing further to emit.
            Constraint::Eager => {}
            // `match(...)` is suggestion metadata only — it shapes completion
            // candidates via the simplified-schema atom (`Constraint::Match` →
            // `CompletionKind::File`), never validation — so it is no longer
            // emitted into the compiled JSON Schema.
            Constraint::Match(_) => {}
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
    use crate::markdown::schemas::ValidationProblemKind;
    use crate::markdown::schemas::simplified::{grammar::parse_type_expr, parse_yaml_schema};
    use crate::markdown::schemas::validate::PositionMap;

    fn parse(yaml: &str) -> SimplifiedSchema {
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("yaml parse");
        parse_yaml_schema(&v).expect("schema parse")
    }

    fn convert(yaml: &str) -> Value {
        to_json_schema(&parse(yaml)).expect("convert")
    }

    fn atom_value(input: &str) -> Value {
        let mut atom = parse_type_expr("test", input).expect("parse atom");
        // Shape tests focus on the typed fragment, so force the atom to be
        // required. Optional wrapping is exercised separately.
        atom.constraints.push(Constraint::Required);
        atom_to_schema("test", &atom).expect("convert").0
    }

    fn optional_atom_value(input: &str) -> Value {
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
    fn bare_file_emits_reference_format_and_drops_match_extension() {
        // Bare `file` is lazy: it lowers to `darkmatter-file-reference`. An
        // optional `file` field wraps that fragment in an `anyOf` with an
        // empty-string arm (Decision A); the file shape lives in the third
        // arm. `match(...)` is suggestion metadata only and is no longer
        // emitted into the compiled JSON Schema (completion reads it from the
        // simplified-schema atom).
        let v = optional_atom_value("file(match('*.md', '!_*.md'))");
        let file_arm = &v["anyOf"][2];
        assert_eq!(file_arm["type"], "string");
        assert_eq!(file_arm["format"], "darkmatter-file-reference");
        assert!(
            file_arm.get("x-darkmatter-match").is_none(),
            "match metadata must not reach the compiled JSON Schema: {file_arm}"
        );
    }

    #[test]
    fn optional_file_wraps_empty_string_arm() {
        let v = optional_atom_value("file");
        // Arm 0 admits null, arm 1 admits the empty string ("absent"), and
        // arm 2 is the (lazy) file shape.
        assert_eq!(v["anyOf"][0]["type"], "null");
        assert_eq!(v["anyOf"][1]["const"], "");
        assert_eq!(v["anyOf"][2]["format"], "darkmatter-file-reference");
    }

    #[test]
    fn required_file_is_not_empty_wrapped() {
        // A required `file` field keeps the strict, unwrapped fragment so an
        // empty string is still rejected. Bare `file` is lazy, so the format
        // is `darkmatter-file-reference`.
        let v = atom_value("file(required)");
        assert_eq!(v["type"], "string");
        assert_eq!(v["format"], "darkmatter-file-reference");
        assert!(v.get("anyOf").is_none(), "required file must not be empty-wrapped");
    }

    #[test]
    fn optional_file_accepts_empty_string_as_absent() {
        // End-to-end: the converted schema validates an empty-string optional
        // `file` value as absent, and an omitted key (or explicit null) is
        // likewise valid.
        let schema = convert("spec: file");
        let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
        assert!(v.is_valid(&json!({ "spec": "" })), "empty optional file must validate");
        assert!(v.is_valid(&json!({ "spec": null })), "null optional file must validate");
        assert!(v.is_valid(&json!({})), "absent optional file must validate");
    }

    #[test]
    fn required_file_rejects_empty_string() {
        let schema = convert("plan: 'file(required)'");
        let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
        assert!(!v.is_valid(&json!({ "plan": "" })), "empty required file must fail");
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
        // Bare `file[]` is lazy per item; array constraints stay on the
        // wrapper. `match(...)` is no longer emitted into the compiled JSON
        // Schema.
        let v = atom_value("file(match('*.png'))[](min(1))");
        assert_eq!(v["type"], "array");
        assert_eq!(v["items"]["format"], "darkmatter-file-reference");
        assert!(v["items"].get("x-darkmatter-match").is_none());
        assert_eq!(v["minItems"], 1);
    }

    #[test]
    fn file_array_lowers_eager_per_item() {
        // `eager` is an item-level constraint: `file(eager)[]` carries the
        // eager `darkmatter-file` format on `items`, while array constraints
        // (`min`, …) stay on the array wrapper. Bare `file[]` stays lazy per
        // item.
        let v = atom_value("file(eager)[](min(1))");
        assert_eq!(v["type"], "array");
        assert_eq!(v["items"]["format"], "darkmatter-file");
        assert_eq!(v["minItems"], 1);

        let v = atom_value("file[]");
        assert_eq!(v["items"]["format"], "darkmatter-file-reference");
    }

    #[test]
    fn description_lands_on_outer_schema() {
        let v = optional_atom_value("string -> The author's full name");
        assert_eq!(v["description"], "The author's full name");

        let v = optional_atom_value("string[] -> A list of tags");
        assert_eq!(v["anyOf"][1]["type"], "array");
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
    fn required_union_with_file_arm_rejects_empty_string() {
        // A required union (here via `number(required)`) must not inherit the
        // optional-`file` empty-string sentinel from its `file` arm: the
        // property is required, so `""` is not "absent" and must fail.
        let yaml = r#"
asset:
  - file
  - "number(required)"
"#;
        let v = convert(yaml);
        assert_eq!(v["required"][0], "asset");
        let arms = v["properties"]["asset"]["anyOf"].as_array().unwrap();
        // No null arm (required) and no empty-string `const` arm anywhere.
        assert!(
            arms.iter().all(|a| a["type"] != "null"),
            "required union must not carry a null arm: {arms:?}"
        );
        assert!(
            arms.iter().all(|a| a.get("const") != Some(&json!(""))),
            "required union must not carry an empty-string file arm: {arms:?}"
        );

        let validator = crate::markdown::schemas::validate::build_validator(&v, None, None).unwrap();
        assert!(
            !validator.is_valid(&json!({ "asset": "" })),
            "required union must reject the empty-string sentinel"
        );
        assert!(
            !validator.is_valid(&json!({ "asset": null })),
            "required union must reject null"
        );
        assert!(
            validator.is_valid(&json!({ "asset": 42 })),
            "required union must still accept a valid number arm"
        );
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
    fn eager_is_accepted_on_file_and_emits_eager_format() {
        // `eager` is a file-only constraint; on `file` it converts cleanly and
        // flips the emitted format to the eager `darkmatter-file`.
        let atom = parse_type_expr("test", "file(eager; required)").unwrap();
        let v = atom_to_schema("test", &atom)
            .expect("file(eager) must convert")
            .0;
        assert_eq!(v["format"], "darkmatter-file");
    }

    #[test]
    fn eager_on_non_file_types_is_fatal() {
        // D2: `eager` applied to any non-`file` type aborts schema preparation
        // with a Convert error that names the offending type and constraint.
        for input in ["string(eager)", "number(eager)"] {
            let atom = parse_type_expr("test", input).unwrap();
            let err = atom_to_schema("test", &atom).unwrap_err();
            match err {
                SchemaError::Convert { property, message } => {
                    assert_eq!(property, "test");
                    assert!(message.contains("eager"), "{input}: {message}");
                    assert!(
                        message.contains("string") || message.contains("number"),
                        "{input}: error must name the offending type: {message}"
                    );
                }
                other => panic!("expected Convert for {input}, got {other:?}"),
            }
        }
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_lazy_eager_required_matrix() {
        // The full 4-cell matrix from the spec's semantics table. `eager` and
        // `required` are orthogonal: `required` governs presence, `eager`
        // governs existence. A present, syntactically valid but not-yet-
        // existing path passes lazy declarations and fails eager ones.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("exists.md"), b"x").unwrap();
        let _cwd = FileFormatCwdGuard::enter(dir.path());

        let present_missing = json!({ "p": "./missing.md" });
        let present_existing = json!({ "p": "./exists.md" });
        let absent = json!({});
        let null = json!({ "p": null });

        // (declaration, absent_ok, null_ok, present_missing_ok, present_existing_ok)
        let cases: &[(&str, bool, bool, bool, bool)] = &[
            // lazy + optional: presence not required, existence not checked.
            ("p: file", true, true, true, true),
            // lazy + required: must be present, existence still not checked.
            ("p: 'file(required)'", false, false, true, true),
            // eager + optional: absent OK; if present, must exist.
            ("p: 'file(eager)'", true, true, false, true),
            // eager + required: must be present AND exist.
            ("p: 'file(eager; required)'", false, false, false, true),
        ];

        for (decl, absent_ok, null_ok, present_missing_ok, present_existing_ok) in cases {
            let schema = convert(decl);
            let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
            assert_eq!(v.is_valid(&absent), *absent_ok, "{decl}: absent");
            assert_eq!(v.is_valid(&null), *null_ok, "{decl}: null");
            assert_eq!(
                v.is_valid(&present_missing),
                *present_missing_ok,
                "{decl}: present-but-missing"
            );
            assert_eq!(
                v.is_valid(&present_existing),
                *present_existing_ok,
                "{decl}: present-and-existing"
            );
        }
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_malformed_reference_is_fatal_under_both_lazy_and_eager() {
        // Laziness defers *existence*, not *syntax*: a malformed reference (the
        // empty string, rejected at `FileReference` parse time) is rejected by
        // both the lazy bare `file` and the eager `file(eager)` declarations.
        let dir = tempfile::tempdir().expect("tempdir");
        let _cwd = FileFormatCwdGuard::enter(dir.path());
        let malformed = json!({ "p": "" });

        for decl in ["p: 'file(required)'", "p: 'file(eager; required)'"] {
            let schema = convert(decl);
            let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
            assert!(
                !v.is_valid(&malformed),
                "{decl}: malformed reference must be rejected"
            );
        }
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn file_array_lazy_accepts_missing_item_eager_rejects() {
        // Per-item posture: `file[]` accepts an array whose item is a
        // syntactically valid missing path, while `file(eager)[]` rejects the
        // same missing item (and accepts an existing one).
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("exists.md"), b"x").unwrap();
        let _cwd = FileFormatCwdGuard::enter(dir.path());

        let missing_item = json!({ "p": ["./missing.md"] });
        let existing_item = json!({ "p": ["./exists.md"] });

        let lazy = crate::markdown::schemas::validate::build_validator(
            &convert("p: 'file[]'"),
            None,
            None,
        )
        .unwrap();
        assert!(
            lazy.is_valid(&missing_item),
            "file[] must accept a missing syntactically valid item"
        );

        let eager = crate::markdown::schemas::validate::build_validator(
            &convert("p: 'file(eager)[]'"),
            None,
            None,
        )
        .unwrap();
        assert!(
            !eager.is_valid(&missing_item),
            "file(eager)[] must reject a missing item"
        );
        assert!(
            eager.is_valid(&existing_item),
            "file(eager)[] must accept an existing item"
        );
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

    // ── Inline object conversion (Phase 2) ───────────────────────────────

    #[test]
    fn inline_object_compiles_to_additional_properties_false() {
        // Decision #7: every inline object fragment carries
        // `additionalProperties: false` so a user-facing inline object
        // restricts the shape, unlike the root schema's `true` default.
        let v = atom_value("{ foo: string, bar: number }");
        assert_eq!(v["type"], "object");
        assert_eq!(v["additionalProperties"], false);
        let props = v["properties"].as_object().unwrap();
        assert!(props.contains_key("foo"));
        assert!(props.contains_key("bar"));
    }

    #[test]
    fn inline_object_collects_required_from_inner_properties() {
        // `host: string(required)` puts `host` on the inline object's
        // `required` array; the *outer* atom's `Required` (if any) stays
        // at the parent level and does not show up here.
        let v = atom_value("{ host: string(required) }");
        let required = v["required"].as_array().unwrap();
        assert_eq!(required, &vec![Value::String("host".into())]);
    }

    #[test]
    fn inline_object_does_not_hoist_inner_required_to_property_level() {
        // Risk Mitigation Checkpoint #4: inner `required` stays inside
        // the inline object fragment, not at the property's parent. The
        // document schema's `required` array must not pick up `host`.
        let yaml = r#"
config: "{ host: string(required) }"
"#;
        let v = convert(yaml);
        // The root document's `required` array is absent — the outer atom
        // has no `Required` constraint of its own, and the inner one must
        // not be lifted out.
        assert!(
            v.get("required").is_none(),
            "root required must not include inline object inner requireds: {v:?}"
        );
        // But the inner fragment carries it. Because the outer atom is
        // optional, reach through the nullable wrapper to the typed arm.
        let config = &v["properties"]["config"]["anyOf"][1];
        let required = config["required"].as_array().unwrap();
        assert_eq!(required, &vec![Value::String("host".into())]);
    }

    #[test]
    fn inline_object_array_wraps_items_with_array_constraints() {
        // `{ foo: string }[](min(1))` becomes an array whose `items`
        // carries the inline object fragment and whose `minItems` is set
        // on the array level — not on the inner `items` schema.
        let v = atom_value("{ foo: string }[](min(1); max(3); unique)");
        assert_eq!(v["type"], "array");
        let items = &v["items"];
        assert_eq!(items["type"], "object");
        assert_eq!(items["additionalProperties"], false);
        assert_eq!(v["minItems"], 1);
        assert_eq!(v["maxItems"], 3);
        assert_eq!(v["uniqueItems"], true);
        // The inner `items` must NOT carry array-level keys.
        assert!(items.get("minItems").is_none());
        assert!(items.get("maxItems").is_none());
        assert!(items.get("uniqueItems").is_none());
    }

    #[test]
    fn inline_object_outer_required_is_on_property_not_fragment() {
        // `{ host: string }(required)` is a single-value inline object
        // with the *containing property* marked required. The inner
        // fragment's `required` array must be absent (no inner `host`
        // is required), and the document's `required` array must
        // contain `config`.
        let yaml = r#"
config: "{ host: string }(required)"
"#;
        let v = convert(yaml);
        let required = v["required"].as_array().unwrap();
        assert_eq!(required, &vec![Value::String("config".into())]);
        let config = &v["properties"]["config"];
        assert!(
            config.get("required").is_none(),
            "outer required must not leak into the fragment: {config:?}"
        );
    }

    #[test]
    fn nested_inline_object_emits_additional_properties_false_at_every_level() {
        // Each inline object layer (outer and inner) must carry
        // `additionalProperties: false` — the decision applies at every
        // level, not just the top one.
        let v = atom_value("{ outer: { inner: string } }");
        let outer = &v;
        assert_eq!(outer["additionalProperties"], false);
        let outer_props = outer["properties"].as_object().unwrap();
        let inner = &outer_props["outer"]["anyOf"][1];
        assert_eq!(inner["type"], "object");
        assert_eq!(inner["additionalProperties"], false);
    }

    #[test]
    fn inline_object_rejects_non_universal_atom_constraint() {
        // Inline object atoms accept only `required` and `default(...)`.
        // `string(min(1))` after the closing brace is rejected as
        // illegal on an inline object, mirroring the per-type
        // `invalid_constraint` wording in the existing fragment builders.
        let atom = parse_type_expr("test", "{ host: string }(min(1))").unwrap();
        let err = atom_to_schema("test", &atom).unwrap_err();
        match err {
            SchemaError::Convert { property, message } => {
                assert_eq!(property, "test");
                assert!(message.contains("inline object"), "{message}");
            }
            other => panic!("expected Convert, got {other:?}"),
        }
    }

    #[test]
    fn empty_inline_object_compiles_with_additional_properties_false() {
        // `{}` is a legal inline object; the fragment carries an empty
        // `properties` map and `additionalProperties: false` (so a value
        // is still constrained to be an empty object, not a free-form
        // map of arbitrary keys).
        let v = atom_value("{}");
        assert_eq!(v["type"], "object");
        assert_eq!(v["additionalProperties"], false);
        let props = v["properties"].as_object().unwrap();
        assert!(props.is_empty());
        assert!(v.get("required").is_none());
    }

    #[test]
    fn inline_object_preserves_descriptions_on_inner_properties() {
        // Per-property `-> description` is preserved on the inner
        // property fragments, not hoisted to the parent.
        let v = atom_value("{ foo: string(required) -> The foo }");
        let foo = &v["properties"]["foo"];
        assert_eq!(foo["description"], "The foo");
        // And it does not appear on the outer fragment.
        assert!(v.get("description").is_none());
    }

    // ── Backward compatibility: v1 schemas must still parse identically ─

    #[test]
    fn v1_opaque_object_still_compiles_without_additional_properties_false() {
        // The opaque `object` keyword (Decision #7) keeps its existing
        // behavior: `{ "type": "object" }` with NO `additionalProperties`.
        let v = atom_value("object");
        assert_eq!(v["type"], "object");
        assert!(
            v.get("additionalProperties").is_none(),
            "opaque object must not gain `additionalProperties: false`: {v:?}"
        );
    }

    #[test]
    fn v1_opaque_object_array_still_compiles_without_additional_properties_false() {
        // And the array-of-objects form keeps the same shape: a `type:
        // array` whose `items.type` is `object` and which carries no
        // `additionalProperties` anywhere.
        let v = atom_value("object[]");
        assert_eq!(v["type"], "array");
        assert_eq!(v["items"]["type"], "object");
        assert!(v["items"].get("additionalProperties").is_none());
        assert!(v.get("additionalProperties").is_none());
    }

    #[test]
    fn required_scalar_atoms_are_byte_for_byte_unchanged() {
        // Snapshot-style check on required scalar atoms: the JSON shape
        // emitted by the converter must match what the pre-nullable codebase
        // produced, byte for byte. Optional variants are covered separately.
        for (input, expected_keys) in [
            ("string(required)", vec!["type"]),
            ("number(required)", vec!["type"]),
            ("boolean(required)", vec!["type"]),
            (
                "string(required; min(5); max(80))",
                vec!["type", "minLength", "maxLength"],
            ),
            ("date(required)", vec!["type", "format"]),
            ("datetime(required)", vec!["type", "format"]),
            ("time(required)", vec!["type", "format"]),
            ("email(required)", vec!["type", "format"]),
            ("any(required)", vec![]),
            (
                "string(pattern(^[a-z]+$))[](min(1); max(5); unique; required)",
                vec!["type", "items", "minItems", "maxItems", "uniqueItems"],
            ),
        ] {
            let v = atom_value(input);
            let obj = v.as_object().unwrap();
            let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
            keys.sort();
            let mut expected: Vec<&str> = expected_keys.clone();
            expected.sort();
            assert_eq!(
                keys, expected,
                "required atom `{input}` has drifted: got keys {keys:?}"
            );
        }
    }

    #[test]
    fn optional_scalar_atoms_emit_null_wrap() {
        // Every optional scalar (and array) is now wrapped in a nullable
        // `anyOf` whose first arm is `{"type":"null"}`.
        for input in [
            "string",
            "number",
            "boolean",
            "string(min(5); max(80))",
            "date",
            "datetime",
            "time",
            "email",
            "any",
            "string(pattern(^[a-z]+$))[](min(1); max(5); unique)",
        ] {
            let v = optional_atom_value(input);
            let obj = v.as_object().unwrap();
            assert_eq!(
                obj.keys().collect::<Vec<_>>(),
                vec!["anyOf"],
                "optional atom `{input}` should emit only an anyOf wrapper"
            );
            let arms = v["anyOf"].as_array().unwrap();
            assert_eq!(
                arms[0]["type"],
                "null",
                "optional atom `{input}` should lead with a null arm"
            );
        }
    }

    // ── Acceptance tests for nullable optional properties (Phase 1) ────────

    #[test]
    fn optional_primitives_accept_null_as_absent() {
        let cases: &[(&str, Value)] = &[
            ("string", json!("hello")),
            ("number", json!(42)),
            ("boolean", json!(true)),
            ("date", json!("2026-06-18")),
            ("datetime", json!("2026-06-18T12:00:00Z")),
            ("time", json!("12:00:00Z")),
            ("email", json!("a@example.com")),
            ("url", json!("https://example.com")),
            ("object", json!({ "x": 1 })),
        ];
        for (ty, valid_value) in cases {
            let schema = convert(&format!("opt: {ty}"));
            let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
            assert!(v.is_valid(&json!({})), "{ty}: absent must validate");
            assert!(v.is_valid(&json!({ "opt": null })), "{ty}: null must validate");
            assert!(
                v.is_valid(&json!({ "opt": valid_value.clone() })),
                "{ty}: valid non-null must validate"
            );

            let required_schema = convert(&format!("req: '{ty}(required)'"));
            let v_req =
                crate::markdown::schemas::validate::build_validator(&required_schema, None, None).unwrap();
            assert!(
                !v_req.is_valid(&json!({ "req": null })),
                "{ty}: required must reject null"
            );
        }

        // Enum is tested separately because its members live inside the
        // grammar's parentheses; `required` follows after a `;` separator.
        let schema = convert("opt: enum(red,green)");
        let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
        assert!(v.is_valid(&json!({})), "enum: absent must validate");
        assert!(v.is_valid(&json!({ "opt": null })), "enum: null must validate");
        assert!(v.is_valid(&json!({ "opt": "red" })), "enum: valid member must validate");

        let required_enum = convert("req: 'enum(red,green; required)'");
        let v_req = crate::markdown::schemas::validate::build_validator(&required_enum, None, None).unwrap();
        assert!(
            !v_req.is_valid(&json!({ "req": null })),
            "enum: required must reject null"
        );
        assert!(
            v_req.is_valid(&json!({ "req": "red" })),
            "enum: required valid member must validate"
        );
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn optional_file_accepts_null_and_empty_as_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("x.md");
        std::fs::write(&path, b"x").unwrap();
        let _cwd = FileFormatCwdGuard::enter(dir.path());

        let schema = convert("spec: file");
        let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
        assert!(v.is_valid(&json!({})), "absent optional file must validate");
        assert!(v.is_valid(&json!({ "spec": null })), "null optional file must validate");
        assert!(v.is_valid(&json!({ "spec": "" })), "empty optional file must validate");
        assert!(
            v.is_valid(&json!({ "spec": "./x.md" })),
            "existing optional file must validate"
        );
    }

    #[test]
    fn optional_object_and_inline_object_accept_null() {
        for (key, ty) in [("object", "object"), ("inline", "'{ foo: string }'")] {
            let schema = convert(&format!("opt: {ty}"));
            let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
            assert!(v.is_valid(&json!({})), "{key}: absent must validate");
            assert!(v.is_valid(&json!({ "opt": null })), "{key}: null must validate");
        }
    }

    #[test]
    fn optional_array_accepts_null() {
        let schema = convert("opt: string[]");
        let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
        assert!(v.is_valid(&json!({})));
        assert!(v.is_valid(&json!({ "opt": null })));
        assert!(v.is_valid(&json!({ "opt": [] })));
        assert!(v.is_valid(&json!({ "opt": ["a"] })));
    }

    #[test]
    fn optional_property_union_accepts_null() {
        let yaml = "opt:\n  - string\n  - number\n";
        let schema = convert(yaml);
        let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
        assert!(v.is_valid(&json!({})));
        assert!(v.is_valid(&json!({ "opt": null })));
        assert!(v.is_valid(&json!({ "opt": "x" })));
        assert!(v.is_valid(&json!({ "opt": 3 })));
    }

    #[test]
    fn optional_constraints_are_bypassed_by_null() {
        let schema = convert("opt: 'string(not-empty; min(5))'");
        let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
        assert!(v.is_valid(&json!({ "opt": null })), "null must bypass constraints");
        assert!(!v.is_valid(&json!({ "opt": "" })), "empty string still rejected");
        assert!(!v.is_valid(&json!({ "opt": "ab" })), "too-short string still rejected");
    }

    #[test]
    fn required_string_rejects_null_with_type_problem() {
        use crate::markdown::schemas::validate::{collect_problems, build_validator};

        let schema = convert("req: 'string(required)'");
        let v = build_validator(&schema, None, None).unwrap();
        assert!(!v.is_valid(&json!({ "req": null })));
        let problems = collect_problems(&v, &json!({ "req": null }), &PositionMap::new());
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].kind, ValidationProblemKind::Type);
    }

    #[test]
    fn optional_any_accepts_null_and_required_any_is_presence_only() {
        // Compatibility preservation: `any` already includes every JSON value,
        // so the optional wrapper adds null and the required wrapper is only
        // a presence check.
        let schema = convert("opt: any");
        let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
        assert!(v.is_valid(&json!({})));
        assert!(v.is_valid(&json!({ "opt": null })));
        assert!(v.is_valid(&json!({ "opt": { "nested": [1, 2, 3] } })));

        let required_schema = convert("req: 'any(required)'");
        let v_req =
            crate::markdown::schemas::validate::build_validator(&required_schema, None, None).unwrap();
        assert!(!v_req.is_valid(&json!({})), "required any rejects absent");
        assert!(
            v_req.is_valid(&json!({ "req": null })),
            "required any accepts null (presence only)"
        );
    }

    #[test]
    fn optional_default_and_description_land_on_outer_wrapper() {
        let v = optional_atom_value("string(default(hello)) -> A greeting");
        assert_eq!(v["default"], "hello");
        assert_eq!(v["description"], "A greeting");
        // The typed arm should not carry them.
        let typed_arm = &v["anyOf"][1];
        assert!(typed_arm.get("default").is_none());
        assert!(typed_arm.get("description").is_none());
    }

    #[test]
    fn optional_union_property_is_single_level_nullable_any_of() {
        let yaml = "opt:\n  - string\n  - number\n";
        let v = convert(yaml);
        let arms = v["properties"]["opt"]["anyOf"].as_array().unwrap();
        assert_eq!(arms.len(), 3, "expected [null, string, number]: {arms:?}");
        assert_eq!(arms[0]["type"], "null");
        assert_eq!(arms[1]["type"], "string");
        assert_eq!(arms[2]["type"], "number");
    }

    /// Temporary CWD guard so `darkmatter-file` validation sees deterministic
    /// relative paths. Serialised with `darkmatter_file_cwd` to match the
    /// convention used in `validate::tests`.
    struct FileFormatCwdGuard {
        prior: std::path::PathBuf,
    }

    impl FileFormatCwdGuard {
        fn enter(dir: &std::path::Path) -> Self {
            let prior = std::env::current_dir().expect("read CWD");
            std::env::set_current_dir(dir).expect("set CWD");
            Self { prior }
        }
    }

    impl Drop for FileFormatCwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.prior);
        }
    }
}
