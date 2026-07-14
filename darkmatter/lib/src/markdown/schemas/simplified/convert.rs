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
    Constraint, PatternKey, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema,
    SimplifiedType, TypeExpr,
};
use crate::markdown::compose::expression::parse_condition;
use crate::markdown::schemas::errors::SchemaError;
use crate::markdown::schemas::format::{
    DARKMATTER_DATETIME_FORMAT, DARKMATTER_EXPRESSION_FORMAT, DARKMATTER_FILE_FORMAT,
    DARKMATTER_FILE_REFERENCE_FORMAT, DARKMATTER_JSON_FORMAT, DARKMATTER_TIME_FORMAT,
    DARKMATTER_YAML_FORMAT,
};

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
    // Root shapes default to open (`additionalProperties: true`); a pattern-keyed
    // root switches to closed-object semantics inside `object_body_from_shape`.
    let mut obj = object_body_from_shape("<root>", shape, Value::Bool(true))?;
    apply_object_arity(&mut obj, &shape.constraints);
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
    // `generated` is a property-level ownership semantic: if any arm is
    // host-supplied, the static `required` entry is suppressed (spec point 1).
    let any_generated = arms.iter().any(atom_is_generated);

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
    // Surface the ownership semantic on the property's schema object so
    // downstream tooling (LSP, completion, runtime validators) discovers it
    // without drilling into arms.
    if any_generated {
        union_schema.insert("x-darkmatter-generated".into(), Value::Bool(true));
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
    // `generated` suppresses the static `required` entry so authored documents
    // validate cleanly when the host has not yet supplied the value (spec
    // point 1). The non-nullable type semantics of `required` are preserved
    // because the null-arm decision above keys off `required`, not off
    // `static_required`.
    let static_required = required && !any_generated;
    Ok((schema, static_required))
}

// ── Atom → JSON Schema fragment ──────────────────────────────────────────

/// Wraps a finished typed fragment so an optional property also accepts
/// JSON `null` as a sentinel for "absent".
fn wrap_optional_null(inner: Value) -> Value {
    json!({ "anyOf": [ { "type": "null" }, inner ] })
}

fn atom_to_schema(name: &str, atom: &PropertyAtom) -> Result<(Value, bool), SchemaError> {
    let (mut fragment, non_nullable) = atom_fragment_without_null_wrap(name, atom, true)?;
    let generated = atom_is_generated(atom);

    // Non-nullable atoms (those carrying `Required`) are emitted unchanged.
    // Optional atoms are wrapped so that JSON `null` validates as "absent".
    // The optional `file` case also preserves the legacy empty-string sentinel.
    // The null-wrap decision keys off `Required` presence, NOT off membership
    // in the parent's `required` array — so `string(generated; required)` still
    // lowers to a bare non-nullable `string` (spec semantics point 4).
    if non_nullable {
        // `generated` suppresses the static `required` entry so an authored
        // document validates cleanly when the host has not yet supplied the
        // value (spec semantics point 1). The non-nullable type is preserved,
        // so a present, wrongly-typed host-supplied value still fails type
        // validation (point 3).
        return Ok((fragment, !generated));
    }

    // Pull annotations off the fragment so they land on the outer wrapper,
    // not the typed arm. This keeps required-atom output byte-for-byte
    // identical while allowing optional wrappers to carry their own metadata.
    let mut default_val: Option<Value> = None;
    let mut description_val: Option<Value> = None;
    let mut generated_val: Option<Value> = None;
    let mut example_val: Option<Value> = None;
    if let Value::Object(map) = &mut fragment {
        default_val = map.remove("default");
        description_val = map.remove("description");
        generated_val = map.remove("x-darkmatter-generated");
        example_val = map.remove("x-darkmatter-example");
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
        if let Some(g) = generated_val {
            map.insert("x-darkmatter-generated".into(), g);
        }
        if let Some(e) = example_val {
            map.insert("x-darkmatter-example".into(), e);
        }
    }

    // An optional atom is never in the parent's static `required` array,
    // regardless of `generated`.
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

/// Reports whether an atom carries the `generated` constraint at either the
/// value or array level. Used to suppress the static `required` entry for
/// host-supplied properties (spec semantics point 1) while preserving the
/// non-nullable type semantics of `required` (point 4).
fn atom_is_generated(atom: &PropertyAtom) -> bool {
    atom.constraints
        .iter()
        .chain(atom.array_constraints.iter())
        .any(|c| matches!(c, Constraint::Generated))
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
    // `Generated` is similarly hoisted: it only surfaces as the
    // `x-darkmatter-generated: true` annotation, never as a typed fragment key.
    let mut required = false;
    let mut generated = false;
    let mut default_val: Option<Value> = None;
    // `example(...)` is documentation, not validation: the raw authored
    // reference strings are hoisted onto the `x-darkmatter-example` annotation
    // (resolved into example objects by `resolve.rs`) and never reach a type
    // fragment builder as a constraint.
    let mut examples: Vec<String> = Vec::new();
    for c in atom.constraints.iter().chain(atom.array_constraints.iter()) {
        match c {
            Constraint::Required => required = true,
            Constraint::Generated => generated = true,
            Constraint::Default(v) => default_val = Some(normalize_json_number(v.clone())),
            Constraint::Example(refs) => examples.extend(refs.iter().cloned()),
            _ => {}
        }
    }

    let inner = match &atom.ty {
        TypeExpr::Primitive(ty) => type_fragment(name, *ty, &atom.constraints)?,
        TypeExpr::InlineObject(shape) => inline_object_fragment(name, shape, &atom.constraints)?,
        // An imported type must be inline-expanded by the resolver before
        // conversion, just as a root-union `FileRef` arm is. Reaching the
        // converter with one unresolved is a hard error (resolution lands in
        // Phase 4).
        TypeExpr::Imported {
            name: ty_name,
            reference,
        } => {
            return Err(SchemaError::Convert {
                property: name.to_string(),
                message: format!(
                    "imported type `{ty_name}@{reference}` must be resolved before \
                     conversion (handled by the resolution layer)"
                ),
            });
        }
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
    // `default` is hoisted and `description` stays arm-local. The
    // `x-darkmatter-generated` annotation rides along the same path so it
    // lands on the property's schema object regardless of wrapping.
    if let Value::Object(map) = &mut fragment {
        if let Some(d) = default_val {
            map.insert("default".into(), d);
        }
        if let Some(desc) = &atom.description {
            map.insert("description".into(), Value::String(desc.clone()));
        }
        if generated {
            map.insert("x-darkmatter-generated".into(), Value::Bool(true));
        }
        if !examples.is_empty() {
            map.insert(
                "x-darkmatter-example".into(),
                Value::Array(examples.into_iter().map(Value::String).collect()),
            );
        }
    }

    Ok((fragment, required))
}

/// Builds the exact non-null schema fragment used to lint one atom's
/// suggestions.
pub(super) fn suggestion_target_schema(
    name: &str,
    atom: &PropertyAtom,
) -> Result<Value, SchemaError> {
    let (fragment, _) = atom_fragment_without_null_wrap(name, atom, false)?;
    let mut target = if atom.is_array {
        fragment
            .get("items")
            .cloned()
            .expect("array atoms always carry an items schema")
    } else {
        fragment
    };
    if let Value::Object(map) = &mut target {
        map.remove("x-darkmatter-suggest");
        map.remove("x-darkmatter-example");
        map.remove("x-darkmatter-generated");
        map.remove("default");
        map.remove("description");
    }
    Ok(target)
}

/// Lowers an inline object `SchemaShape` to a Draft 2020-12 object fragment.
///
/// The fragment defaults to `additionalProperties: false` (Decision #7):
/// authors reach for an inline object specifically to constrain the shape,
/// so silently accepting extra keys would defeat their intent. A `<string>`
/// catch-all pattern key (Feature C) overrides that default in
/// `object_body_from_shape`, lowering `additionalProperties` to the catch-all's
/// value schema instead. Nested
/// `required` constraints live on the fragment, not the parent property, so
/// `union_property_to_schema`'s hoisting logic cannot accidentally lift them
/// out of an inline object arm (Risk Mitigation Checkpoint 4).
fn inline_object_fragment(
    name: &str,
    shape: &SchemaShape,
    atom_constraints: &[Constraint],
) -> Result<Value, SchemaError> {
    // Inline object atoms accept the universal `required` / `default` /
    // `generated` constraints (hoisted elsewhere) plus the object-arity
    // `min-keys` / `max-keys` (Feature C / O-C3). Reject anything else with the
    // same wording primitive fragment builders use.
    for c in atom_constraints {
        match c {
            Constraint::Required
            | Constraint::Default(_)
            | Constraint::Generated
            | Constraint::Example(_)
            | Constraint::MinKeys(_)
            | Constraint::MaxKeys(_) => {}
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

    let mut obj = object_body_from_shape(name, shape, Value::Bool(false))?;
    // Object arity from both authoring surfaces desugars to the same emitted
    // keys: the reserved `$constraints` block (`shape.constraints`) and the
    // postfix `{ … }(min-keys(1))` form (`atom_constraints`).
    apply_object_arity(&mut obj, &shape.constraints);
    apply_object_arity(&mut obj, atom_constraints);
    Ok(Value::Object(obj))
}

/// Builds the `type` / `additionalProperties` / `properties` /
/// `patternProperties` / `required` portion of an object schema from a shape's
/// literal and pattern keys (Feature C).
///
/// `default_additional` is the `additionalProperties` value used when the shape
/// has **no** pattern keys (root shapes default to open `true`; inline objects
/// default to closed `false`). When pattern keys are present the closed-object
/// rule (O-C1) overrides it: a `<string>` catch-all lowers to
/// `additionalProperties: <valueSchema>`, and any other pattern set lowers to
/// `additionalProperties: false`.
fn object_body_from_shape(
    context: &str,
    shape: &SchemaShape,
    default_additional: Value,
) -> Result<Map<String, Value>, SchemaError> {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for (prop_name, def) in &shape.properties {
        let (prop_schema, is_required) = property_def_to_schema(prop_name, def)?;
        if is_required {
            required.push(Value::String(prop_name.clone()));
        }
        properties.insert(prop_name.clone(), prop_schema);
    }

    // Literal names feed the negative-lookahead exclusion so a key that also
    // names a declared property is validated only by that property's schema
    // (literal keys win — O-C1).
    let literal_names: Vec<&str> = shape.properties.keys().map(String::as_str).collect();
    let mut catch_all: Option<Value> = None;
    let mut pattern_properties = Map::new();
    for pk in &shape.pattern_keys {
        match &pk.key {
            PatternKey::CatchAll => {
                catch_all = Some(pattern_value_schema(context, &pk.def)?);
            }
            other => {
                let emitted = emitted_pattern_property(context, other, &literal_names)?;
                let value_schema = pattern_value_schema(context, &pk.def)?;
                pattern_properties.insert(emitted, value_schema);
            }
        }
    }

    let additional = if let Some(value) = catch_all {
        value
    } else if shape.pattern_keys.is_empty() {
        default_additional
    } else {
        Value::Bool(false)
    };

    let mut obj = Map::new();
    obj.insert("type".into(), Value::String("object".into()));
    obj.insert("additionalProperties".into(), additional);
    obj.insert("properties".into(), Value::Object(properties));
    if !pattern_properties.is_empty() {
        obj.insert(
            "patternProperties".into(),
            Value::Object(pattern_properties),
        );
    }
    if !required.is_empty() {
        obj.insert("required".into(), Value::Array(required));
    }
    Ok(obj)
}

/// Lowers a pattern-key value definition to its JSON Schema. Dictionary values
/// are not "optional properties", so they emit the bare typed fragment (no
/// nullable wrapper) — `<string>: number` yields `additionalProperties:
/// { "type": "number" }`, not a null-tolerant `anyOf`.
fn pattern_value_schema(context: &str, def: &PropertyDef) -> Result<Value, SchemaError> {
    match def {
        PropertyDef::Single(atom) => Ok(atom_fragment_without_null_wrap(context, atom, false)?.0),
        PropertyDef::Union(arms) => {
            let mut any_of = Vec::with_capacity(arms.len());
            for atom in arms {
                any_of.push(atom_fragment_without_null_wrap(context, atom, false)?.0);
            }
            Ok(json!({ "anyOf": any_of }))
        }
    }
}

/// Lowers a non-catch-all [`PatternKey`] to the ECMA-262 regex emitted under
/// `patternProperties`, applying literal-key precedence (O-C1) by excluding any
/// literal property names that could otherwise double-validate.
///
/// ## Errors
///
/// Returns [`SchemaError::Convert`] when the emitted pattern (after literal
/// exclusion) is not a valid regex under the engine schema validation uses.
fn emitted_pattern_property(
    context: &str,
    key: &PatternKey,
    literal_names: &[&str],
) -> Result<String, SchemaError> {
    let core = match key {
        PatternKey::Starting(prefix) => format!("^{}", regex::escape(prefix)),
        PatternKey::Ending(suffix) => format!("{}$", regex::escape(suffix)),
        PatternKey::Pattern(re) => re.clone(),
        // The catch-all lowers to `additionalProperties`, never here.
        PatternKey::CatchAll => unreachable!("catch-all handled before patternProperties"),
    };
    let emitted = wrap_literal_exclusion(&core, literal_names);
    validate_emitted_pattern(context, key, &emitted)?;
    Ok(emitted)
}

/// Wraps an emitted regex with a start-anchored negative lookahead that
/// subtracts the literal key set, so a literal property never also matches a
/// sibling pattern (O-C1). Returns `core` unchanged when there are no literal
/// keys to exclude (keeping the lookaround-free pattern on the linear engine).
fn wrap_literal_exclusion(core: &str, literal_names: &[&str]) -> String {
    if literal_names.is_empty() {
        return core.to_string();
    }
    let alternation = literal_names
        .iter()
        .map(|name| regex::escape(name))
        .collect::<Vec<_>>()
        .join("|");
    let exclusion = format!("(?!(?:{alternation})$)");
    match core.strip_prefix('^') {
        // Start-anchored cores (`^PREFIX`) splice the exclusion after the
        // anchor: `^(?!(?:LITS)$)PREFIX`.
        Some(rest) => format!("^{exclusion}{rest}"),
        // Search cores (`SUFFIX$`, a raw `<pattern::RE>`) keep their match-
        // anywhere semantics: anchor the exclusion at the string start, then
        // allow any prefix before the original pattern.
        None => format!("^{exclusion}.*{core}"),
    }
}

/// Verifies an emitted pattern compiles under the same regex engine schema
/// validation uses: `fancy-regex` when the pattern carries a lookaround
/// (Feature C literal precedence), otherwise the linear `regex` engine.
fn validate_emitted_pattern(
    context: &str,
    key: &PatternKey,
    emitted: &str,
) -> Result<(), SchemaError> {
    let compiles = if has_lookaround(emitted) {
        fancy_regex::Regex::new(emitted).is_ok()
    } else {
        regex::Regex::new(emitted).is_ok()
    };
    if compiles {
        Ok(())
    } else {
        Err(SchemaError::Convert {
            property: context.to_string(),
            message: format!(
                "pattern key `{}` cannot be lowered into a valid `patternProperties` \
                 regex `{emitted}`",
                pattern_key_display(key)
            ),
        })
    }
}

/// Reconstructs a pattern key's authored `<...>` spelling for error messages.
fn pattern_key_display(key: &PatternKey) -> String {
    match key {
        PatternKey::CatchAll => "<string>".to_string(),
        PatternKey::Starting(prefix) => format!("<starting::{prefix}>"),
        PatternKey::Ending(suffix) => format!("<ending::{suffix}>"),
        PatternKey::Pattern(re) => format!("<pattern::{re}>"),
    }
}

/// Reports whether a regex string carries a lookaround construct
/// (`(?!`, `(?=`, `(?<`). Such patterns require the backtracking `fancy-regex`
/// engine; the default linear `regex` engine rejects them.
pub(in crate::markdown::schemas) fn has_lookaround(pattern: &str) -> bool {
    pattern.contains("(?!") || pattern.contains("(?=") || pattern.contains("(?<")
}

/// Emits `minProperties` / `maxProperties` from `min-keys` / `max-keys` object
/// arity constraints (Feature C / O-C3). Other constraints are ignored here:
/// they are hoisted (`required` / `default` / `generated`) or rejected on the
/// per-type / array-level paths.
fn apply_object_arity(obj: &mut Map<String, Value>, constraints: &[Constraint]) {
    for c in constraints {
        match c {
            Constraint::MinKeys(n) => {
                obj.insert("minProperties".into(), json!(*n));
            }
            Constraint::MaxKeys(n) => {
                obj.insert("maxProperties".into(), json!(*n));
            }
            _ => {}
        }
    }
}

fn apply_array_constraints(
    name: &str,
    arr: &mut Map<String, Value>,
    constraints: &[Constraint],
) -> Result<(), SchemaError> {
    for c in constraints {
        match c {
            Constraint::Required
            | Constraint::Default(_)
            | Constraint::Generated
            | Constraint::Example(_) => {
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
        // `datetime` / `time` lower to darkmatter custom formats (offset
        // optional) rather than the built-in RFC 3339 `date-time` / `time`,
        // which reject the offset-less local values ISO 8601 allows. `date` has
        // no offset component, so it keeps the built-in format.
        SimplifiedType::DateTime => {
            date_family_fragment(name, DARKMATTER_DATETIME_FORMAT, constraints)
        }
        SimplifiedType::Time => date_family_fragment(name, DARKMATTER_TIME_FORMAT, constraints),
        SimplifiedType::Number => number_fragment(name, constraints),
        SimplifiedType::NumberLike => numberlike_fragment(name, constraints),
        SimplifiedType::Boolean => simple_typed_fragment(name, "boolean", constraints),
        SimplifiedType::Boolish => boolish_fragment(name, constraints),
        SimplifiedType::Object => simple_typed_fragment(name, "object", constraints),
        SimplifiedType::File => file_fragment(name, constraints),
        SimplifiedType::Enum => enum_fragment(name, constraints),
        SimplifiedType::Url => url_fragment(name, constraints),
        SimplifiedType::Email => email_fragment(name, constraints),
        // Content-format string types: a string whose content must parse as
        // YAML / strict JSON. Lower to `{ "type": "string", "format": … }` so
        // the custom format validators (`super::super::format`) parse the value;
        // native mappings/sequences/scalars are serialized to a string by the
        // coercion pass before validation.
        SimplifiedType::Yaml => content_format_fragment(name, "yaml", DARKMATTER_YAML_FORMAT, constraints),
        SimplifiedType::Json => content_format_fragment(name, "json", DARKMATTER_JSON_FORMAT, constraints),
        // `literal` lowers to a typed `const`; the optional-nullable wrapper and
        // the `literal(x)[]` array `items` placement fall out of the shared atom
        // path. `expression` is the third content-format string type, lowering
        // to `{ type: string, format: darkmatter-expression }` (parse-only, never
        // evaluated).
        SimplifiedType::Literal => literal_fragment(name, constraints),
        SimplifiedType::Expression => expression_fragment(name, constraints),
        SimplifiedType::Any => any_fragment(name, constraints),
    }
}

fn string_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    let mut m = Map::new();
    m.insert("type".into(), Value::String("string".into()));
    for c in constraints {
        match c {
            Constraint::Required
            | Constraint::Default(_)
            | Constraint::Generated
            | Constraint::Example(_) => {}
            Constraint::Suggest(candidates) => {
                m.insert(
                    "x-darkmatter-suggest".into(),
                    Value::Array(
                        candidates
                            .iter()
                            .map(|candidate| candidate.interpreted.clone())
                            .collect(),
                    ),
                );
            }
            Constraint::MinLen(n) => {
                m.insert("minLength".into(), json!(*n));
            }
            Constraint::MaxLen(n) => {
                m.insert("maxLength".into(), json!(*n));
            }
            Constraint::NotEmpty => {
                // Lookaround-free: any string containing at least one
                // non-whitespace character. A lookaround-free pattern keeps this
                // schema on jsonschema's linear (ReDoS-safe) `regex` engine —
                // only Feature C pattern-key literal precedence opts a schema
                // into `fancy-regex` — so the previous `^(?!\s*$).+` form (which
                // the linear engine rejects) is not portable and `\S` is used.
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
            Constraint::Required
            | Constraint::Default(_)
            | Constraint::Generated
            | Constraint::Example(_) => {}
            Constraint::Suggest(candidates) => {
                m.insert(
                    "x-darkmatter-suggest".into(),
                    Value::Array(
                        candidates
                            .iter()
                            .map(|candidate| candidate.interpreted.clone())
                            .collect(),
                    ),
                );
            }
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
            Constraint::Required
            | Constraint::Default(_)
            | Constraint::Generated
            | Constraint::Example(_) => {}
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
            Constraint::Required
            | Constraint::Default(_)
            | Constraint::Generated
            | Constraint::Example(_) => {}
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
            Constraint::Required
            | Constraint::Default(_)
            | Constraint::Generated
            | Constraint::Example(_) => {}
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

/// Lowers a content-format string type (`yaml` / `json`) to a string carrying
/// the darkmatter content-format seam. `type_label` is the SimplifiedSchema
/// keyword used in constraint-rejection messages; `format` is the emitted
/// `format` value the custom validator keys on. Constraints beyond the universal
/// set are deferred (O-D2 `yaml(schema(...))` is future work).
fn content_format_fragment(
    name: &str,
    type_label: &str,
    format: &str,
    constraints: &[Constraint],
) -> Result<Value, SchemaError> {
    reject_unsupported(name, type_label, constraints, &[])?;
    Ok(json!({ "type": "string", "format": format }))
}

/// Lowers a `literal(value)` atom to a JSON Schema `const` fragment.
///
/// The typed scalar comes from the required [`Constraint::LiteralValue`] the
/// grammar attaches; the value is emitted verbatim (numbers normalized so an
/// integral float renders as `2`, not `2.0`). Only the universal constraints
/// and an **equal** `default(...)` are permitted — a `default` that disagrees
/// with the const is always an authoring bug and fails schema load
/// ([`SchemaError::Convert`]); `suggest` and unrelated constraints are rejected
/// (most are already refused by the grammar's positional-value path).
///
/// The optional-nullable wrapper and the `literal(x)[]` array `items` placement
/// are handled by the shared atom path ([`atom_fragment_without_null_wrap`] /
/// [`atom_to_schema`]), so this only builds the bare `{ "const": … }` core.
fn literal_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    let mut literal_value: Option<Value> = None;
    let mut default_value: Option<Value> = None;
    for c in constraints {
        match c {
            // Hoisted to the property level by the shared atom path.
            Constraint::Required | Constraint::Generated | Constraint::Example(_) => {}
            Constraint::LiteralValue(v) => literal_value = Some(normalize_json_number(v.clone())),
            Constraint::Default(v) => default_value = Some(normalize_json_number(v.clone())),
            other => return Err(invalid_constraint(name, "literal", other)),
        }
    }
    let value = literal_value.ok_or_else(|| SchemaError::Convert {
        property: name.to_string(),
        message: "`literal` requires a value".into(),
    })?;
    if let Some(default) = default_value
        && default != value
    {
        return Err(SchemaError::Convert {
            property: name.to_string(),
            message: format!("`default({default})` must equal the literal value `{value}`"),
        });
    }
    let mut m = Map::new();
    m.insert("const".into(), value);
    Ok(Value::Object(m))
}

/// Lowers an `expression` atom to `{ "type": "string", "format":
/// "darkmatter-expression" }` — the third content-format string type alongside
/// `yaml` / `json`. Constraint applicability mirrors those types (only the
/// universal set; `suggest` and string constraints are rejected by
/// [`reject_unsupported`]). A `default(...)` must itself parse as a Darkmatter
/// expression, checked here at schema-load time.
fn expression_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    reject_unsupported(name, "expression", constraints, &[])?;
    for c in constraints {
        if let Constraint::Default(v) = c
            && parse_condition(&expression_default_source(v)).is_err()
        {
            return Err(SchemaError::Convert {
                property: name.to_string(),
                message: format!("`default({v})` is not a valid Darkmatter expression"),
            });
        }
    }
    Ok(json!({ "type": "string", "format": DARKMATTER_EXPRESSION_FORMAT }))
}

/// Renders an expression `default(...)` value as the source text handed to the
/// parser. Native boolean/number defaults are valid degenerate expressions
/// (`true`, `3`); a string default parses as-authored.
fn expression_default_source(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn any_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    reject_unsupported(name, "any", constraints, &[])?;
    Ok(Value::Object(Map::new()))
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// For types that only accept the universal `required` / `default` /
/// `generated` / `example(...)` constraints, reject anything else.
/// `_extra_allowed` is reserved for future extensions.
fn reject_unsupported(
    name: &str,
    type_label: &str,
    constraints: &[Constraint],
    _extra_allowed: &[&str],
) -> Result<(), SchemaError> {
    for c in constraints {
        match c {
            Constraint::Required
            | Constraint::Default(_)
            | Constraint::Generated
            | Constraint::Example(_) => {}
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
        // An already-integer-typed number (the parser preserves i64/u64 for a
        // bare literal like `9007199254740993`) is exact — passing it through
        // `as_f64()` would round it, so return it untouched. Only a genuine
        // floating representation (the parser's `default(3)` emits `3.0`) is
        // canonicalized to integer form.
        Value::Number(n) if n.is_i64() || n.is_u64() => Value::Number(n),
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
        assert_eq!(atom_value("datetime")["format"], "darkmatter-datetime");
        assert_eq!(atom_value("time")["format"], "darkmatter-time");
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

    // ── `generated` constraint semantics (Phase 2) ──────────────────────────

    /// Snapshot: a `generated; required` property is absent from the emitted
    /// `required` array, carries `x-darkmatter-generated: true`, and keeps a
    /// bare non-nullable type (no `null` anyOf arm).
    #[test]
    fn generated_required_suppresses_static_required_and_emits_annotation() {
        let v = convert("ctx_today: 'string(generated; required)'");
        // (1) absent from the required array.
        assert!(
            v.get("required").is_none(),
            "generated property must not appear in the static `required` array: {v:?}"
        );
        let prop = &v["properties"]["ctx_today"];
        // (2) carries the annotation.
        assert_eq!(prop["x-darkmatter-generated"], true);
        // (3) the type remains non-nullable (no anyOf null arm).
        assert_eq!(prop["type"], "string");
        assert!(
            prop.get("anyOf").is_none(),
            "generated+required must not get the optional null wrap: {prop:?}"
        );
    }

    /// `generated` without `required` emits a nullable type (the `null` arm is
    /// present), keeps the annotation on the outer wrapper, and is absent from
    /// the `required` array (already implied by optionality).
    #[test]
    fn generated_without_required_emits_nullable_type_with_annotation() {
        let v = convert("ctx_maybe: 'string(generated)'");
        assert!(v.get("required").is_none());
        let prop = &v["properties"]["ctx_maybe"];
        assert_eq!(prop["x-darkmatter-generated"], true);
        // (4) nullable type as before.
        let arms = prop["anyOf"].as_array().expect("nullable wrapper present");
        assert_eq!(arms[0]["type"], "null");
        assert_eq!(arms[1]["type"], "string");
    }

    /// A non-`generated` required property still lands in the `required` array
    /// (regression guard: the suppression is gated on `Generated`, not a
    /// side effect of the convert-path refactor).
    #[test]
    fn required_without_generated_still_emits_static_required() {
        let v = convert("title: 'string(required)'");
        let required = v["required"].as_array().expect("required array");
        assert_eq!(required, &vec![Value::String("title".into())]);
        let prop = &v["properties"]["title"];
        assert!(prop.get("x-darkmatter-generated").is_none());
    }

    /// Inside an inline object, a `generated; required` inner property is
    /// absent from the inner object's `required` array, carries the annotation,
    /// and remains non-nullable. Mirrors the `ctx.today` motivating case.
    #[test]
    fn generated_required_inside_inline_object_stays_non_nullable_and_unrequired() {
        // Uses the Phase 1 nested mapping form: `ctx` lowers to an inline
        // object atom with one inner property `today`.
        let v = convert("ctx:\n  today: \"date(generated; required)\"");
        // The outer property `ctx` is itself optional (no outer `required`).
        assert!(
            v.get("required").is_none(),
            "outer ctx must not be required: {v:?}"
        );
        let ctx = &v["properties"]["ctx"]["anyOf"][1];
        // Inner `today` is NOT in the inner `required` array.
        assert!(
            ctx.get("required").is_none(),
            "inner generated property must not be required: {ctx:?}"
        );
        let today = &ctx["properties"]["today"];
        assert_eq!(today["x-darkmatter-generated"], true);
        assert_eq!(today["type"], "string");
        assert_eq!(today["format"], "date");
        assert!(today.get("anyOf").is_none());
    }

    /// End-to-end validation: an authored document omitting a
    /// `generated; required` property validates cleanly; a present but
    /// wrongly-typed value still fails type validation.
    #[test]
    fn generated_required_validates_when_absent_and_type_checks_when_present() {
        let schema = convert("ctx_today: 'string(generated; required)'");
        let v = crate::markdown::schemas::validate::build_validator(&schema, None, None).unwrap();
        // Absent — validates (spec semantics point 1).
        assert!(v.is_valid(&json!({})), "absent generated property must validate");
        // Present and correctly typed — validates.
        assert!(v.is_valid(&json!({ "ctx_today": "2026-07-04" })));
        // Present but wrongly-typed — fails (spec semantics point 3).
        assert!(
            !v.is_valid(&json!({ "ctx_today": 42 })),
            "present wrongly-typed generated value must fail type validation"
        );
    }

    /// A property-level union with a `generated; required` arm suppresses the
    /// static `required` entry, emits the annotation on the union schema, and
    /// keeps the union non-nullable (no `null` arm).
    #[test]
    fn generated_required_union_arm_suppresses_static_required() {
        let yaml = r#"
ctx_kind:
  - "string(generated; required)"
  - "number"
"#;
        let v = convert(yaml);
        assert!(
            v.get("required").is_none(),
            "union with a generated arm must not be statically required: {v:?}"
        );
        let prop = &v["properties"]["ctx_kind"];
        assert_eq!(prop["x-darkmatter-generated"], true);
        let arms = prop["anyOf"].as_array().unwrap();
        // Non-nullable (no null arm).
        assert!(
            arms.iter().all(|a| a["type"] != "null"),
            "union with a required arm must not carry a null arm: {arms:?}"
        );
    }

    /// `generated` is accepted on every type without conversion error. Guards
    /// that the per-type fragment builders all skip the universal `Generated`
    /// constraint rather than reporting it as invalid.
    #[test]
    fn generated_is_accepted_on_every_type() {
        for input in [
            "string(generated)",
            "number(generated)",
            "boolean(generated)",
            "date(generated)",
            "datetime(generated)",
            "time(generated)",
            "numberlike(generated)",
            "boolish(generated)",
            "object(generated)",
            "enum(a, b; generated)",
            "url(generated)",
            "email(generated)",
            "any(generated)",
            "string(generated)[](min(1))",
            "{ foo: string }(generated)",
        ] {
            let atom = parse_type_expr("test", input)
                .unwrap_or_else(|e| panic!("parse failed for `{input}`: {e:?}"));
            atom_to_schema("test", &atom)
                .unwrap_or_else(|e| panic!("convert failed for `{input}`: {e:?}"));
        }
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

/// Schema-plus composition primitives
/// (`darkmatter/features/2026-07-08-schema-plus/`). These pin the JSON Schema
/// lowering for pattern keys, object-arity constraints, and content-format
/// types. The Feature C (pattern-key / object-arity) tests are active as of
/// Phase 3; the Feature D (`yaml` / `json`) tests remain `#[ignore]`-gated
/// until Phase 5 lands. Run the ignored ones with
/// `cargo nextest run -p darkmatter --run-ignored ignored-only schema_plus`.
#[cfg(test)]
mod schema_plus_phase1 {
    use super::*;
    use crate::markdown::schemas::simplified::{grammar::parse_type_expr, parse_yaml_schema};

    /// Parses one required inline/primitive type expression and lowers it to
    /// its typed JSON Schema fragment (no optional-null wrapper).
    fn atom_value(input: &str) -> Value {
        let mut atom = parse_type_expr("test", input).expect("parse atom");
        atom.constraints.push(Constraint::Required);
        atom_to_schema("test", &atom).expect("convert").0
    }

    /// Lowers one type expression as an **optional** property (the standard
    /// nullable `anyOf` wrapper is applied).
    fn optional_atom_value(input: &str) -> Value {
        let atom = parse_type_expr("test", input).expect("parse atom");
        atom_to_schema("test", &atom).expect("convert").0
    }

    /// Compiles a `$schema`-body YAML mapping to its full JSON Schema.
    fn convert(schema_body: &str) -> Value {
        let v: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(schema_body).expect("yaml parse");
        to_json_schema(&parse_yaml_schema(&v).expect("schema parse")).expect("convert")
    }

    // ── Feature C — pattern keys + object arity ──────────────────────────

    #[test]
    fn catch_all_lowers_to_additional_properties() {
        let v = atom_value("{ <string>: number }");
        assert_eq!(v["type"], "object");
        assert_eq!(
            v["additionalProperties"]["type"], "number",
            "catch-all value type must lower to additionalProperties: {v}"
        );
        let props = v["properties"].as_object();
        assert!(
            props.is_none_or(|p| p.is_empty()),
            "catch-all key must not appear as a literal property: {v}"
        );
    }

    #[test]
    fn pattern_key_lowers_with_literal_precedence() {
        let v = atom_value("{ x-kind: string, <pattern::[0-9]$>: number }");
        // The literal key stays a declared property. `x-kind` is optional here,
        // so its typed arm lives under the nullable `anyOf` wrapper.
        assert_eq!(v["properties"]["x-kind"]["anyOf"][1]["type"], "string");
        // The pattern lowers to patternProperties (with literal-key exclusion).
        let pattern_props = v["patternProperties"]
            .as_object()
            .expect("pattern key must lower to patternProperties");
        assert_eq!(pattern_props.len(), 1);
        let (emitted, value_schema) = pattern_props.iter().next().unwrap();
        // The emitted regex carries the negative-lookahead exclusion for the
        // literal key set (`x-kind`), so a literal never double-validates.
        assert!(
            emitted.contains("(?!"),
            "emitted pattern must exclude literal keys: {emitted}"
        );
        assert_eq!(value_schema["type"], "number");
        // A pattern-keyed object without a catch-all is closed.
        assert_eq!(v["additionalProperties"], false);
    }

    #[test]
    fn object_arity_lowers_to_property_counts() {
        let v = atom_value("{ <string>: any }(min-keys(1); max-keys(1))");
        assert_eq!(v["minProperties"], 1);
        assert_eq!(v["maxProperties"], 1);
    }

    #[test]
    fn mixed_literal_and_pattern_keys_lower_together() {
        // A literal property and a pattern key coexist: the literal stays under
        // `properties`, the pattern under `patternProperties`, and the object is
        // closed (no catch-all).
        let v = atom_value("{ name: string(required), <starting::x->: number }");
        assert_eq!(v["type"], "object");
        assert_eq!(v["additionalProperties"], false);
        assert_eq!(v["properties"]["name"]["type"], "string");
        assert_eq!(v["required"], json!(["name"]));
        let pattern_props = v["patternProperties"].as_object().unwrap();
        assert_eq!(pattern_props.len(), 1);
        let (emitted, schema) = pattern_props.iter().next().unwrap();
        assert!(
            emitted.contains("(?!"),
            "emitted pattern must exclude the literal `name`: {emitted}"
        );
        assert_eq!(schema["type"], "number");
    }

    #[test]
    fn multiple_pattern_keys_each_lower_to_pattern_properties() {
        let v = atom_value("{ <starting::a->: string, <ending::-z>: number }");
        let pattern_props = v["patternProperties"].as_object().unwrap();
        assert_eq!(
            pattern_props.len(),
            2,
            "each pattern key gets its own patternProperties entry: {v}"
        );
        assert_eq!(v["additionalProperties"], false);
    }

    #[test]
    fn catch_all_plus_specific_pattern_key() {
        // `<string>` catch-all lowers to additionalProperties; the specific
        // pattern key still lowers to patternProperties.
        let v = atom_value("{ <starting::x->: number, <string>: any }");
        assert!(
            v["additionalProperties"].as_object().unwrap().is_empty(),
            "catch-all `any` lowers to additionalProperties: {{}}: {v}"
        );
        let pattern_props = v["patternProperties"].as_object().unwrap();
        assert_eq!(pattern_props.len(), 1);
        let (_, schema) = pattern_props.iter().next().unwrap();
        assert_eq!(schema["type"], "number");
    }

    #[test]
    fn invalid_pattern_wrapping_is_a_conversion_error() {
        // A `<pattern::RE>` that is not a valid regex cannot be lowered into a
        // `patternProperties` entry; conversion fails loudly rather than
        // shipping a schema the validator would reject at build time.
        let atom = parse_type_expr("dict", "{ <pattern::*>: string }").expect("parse");
        let err = atom_to_schema("dict", &atom).unwrap_err();
        match err {
            SchemaError::Convert { message, .. } => {
                assert!(message.contains("pattern key"), "{message}");
            }
            other => panic!("expected Convert, got {other:?}"),
        }
    }

    #[test]
    fn min_keys_on_non_object_atom_is_rejected() {
        // Object arity is only meaningful on objects; `min-keys` on a scalar
        // atom is a conversion error, mirroring the per-type constraint guard.
        let atom = parse_type_expr("test", "string(min-keys(1))").expect("parse");
        let err = atom_to_schema("test", &atom).unwrap_err();
        match err {
            SchemaError::Convert { message, .. } => {
                assert!(message.contains("min-keys"), "{message}");
                assert!(message.contains("string"), "{message}");
            }
            other => panic!("expected Convert, got {other:?}"),
        }
    }

    #[test]
    fn max_keys_at_array_level_is_rejected() {
        // `max-keys` in an array-level constraint position is ambiguous and
        // rejected — object arity attaches to object items, not the array.
        let atom = parse_type_expr("test", "object[](max-keys(1))").expect("parse");
        let err = atom_to_schema("test", &atom).unwrap_err();
        match err {
            SchemaError::Convert { message, .. } => {
                assert!(message.contains("array level"), "{message}");
            }
            other => panic!("expected Convert, got {other:?}"),
        }
    }

    // ── End-to-end validation checkpoint (Phase 3) ───────────────────────

    /// Converts a full `$schema` document to compiled JSON Schema.
    fn schema_of(yaml: &str) -> Value {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("yaml parse");
        to_json_schema(&parse_yaml_schema(&value).expect("schema parse")).expect("convert")
    }

    fn validator_of(schema: &Value) -> jsonschema::Validator {
        crate::markdown::schemas::validate::build_validator(schema, None, None)
            .expect("build validator")
    }

    #[test]
    fn catch_all_document_enforces_value_type() {
        let schema = schema_of("dict:\n  \"<string>\": number");
        let v = validator_of(&schema);
        assert!(v.is_valid(&json!({ "dict": { "a": 1, "b": 2 } })), "any string key with a number value validates");
        assert!(
            !v.is_valid(&json!({ "dict": { "a": "x" } })),
            "catch-all value type is enforced"
        );
    }

    #[test]
    fn pattern_only_document_stays_closed_and_typed() {
        // No literal keys → no lookaround → the schema stays on the linear
        // engine. `<ending::_id>` keys must carry string values; other keys are
        // rejected (closed object).
        let schema = schema_of("codes:\n  \"<ending::_id>\": string");
        let pattern_props = schema["properties"]["codes"]["anyOf"][1]["patternProperties"]
            .as_object()
            .unwrap();
        assert!(
            pattern_props.keys().all(|k| !k.contains("(?!")),
            "no literals → no lookahead wrapping: {schema}"
        );
        let v = validator_of(&schema);
        assert!(v.is_valid(&json!({ "codes": { "user_id": "u1" } })));
        assert!(!v.is_valid(&json!({ "codes": { "user_id": 5 } })), "value type enforced");
        assert!(
            !v.is_valid(&json!({ "codes": { "name": "x" } })),
            "key not matching the pattern is rejected (closed object)"
        );
    }

    #[test]
    fn literal_precedence_document_validates_via_fancy_regex() {
        // `x-kind` (a number property) would also match the `<starting::x->`
        // string pattern; literal-key precedence excludes it so it is validated
        // only by its own number schema, while other `x-` keys must be strings.
        let schema = schema_of("config:\n  \"x-kind\": number\n  \"<starting::x->\": string");
        let v = validator_of(&schema);
        assert!(
            v.is_valid(&json!({ "config": { "x-kind": 5 } })),
            "literal key wins: `x-kind` is validated only as a number"
        );
        assert!(
            v.is_valid(&json!({ "config": { "x-trace": "on" } })),
            "a pattern-matched key satisfies the string pattern schema"
        );
        assert!(
            !v.is_valid(&json!({ "config": { "x-trace": 5 } })),
            "a pattern-matched key must satisfy the string pattern schema"
        );
        assert!(
            !v.is_valid(&json!({ "config": { "other": 1 } })),
            "a key matching neither literal nor pattern is rejected (closed)"
        );
    }

    #[test]
    fn exact_one_key_dictionary_validates_only_single_pairs() {
        // The ratified `parameter` fixture shape: one `<string>: any` pair.
        let schema = schema_of(
            "parameter:\n  \"<string>\": any\n  $constraints:\n    min-keys: 1\n    max-keys: 1",
        );
        let inner = &schema["properties"]["parameter"]["anyOf"][1];
        assert_eq!(inner["minProperties"], 1);
        assert_eq!(inner["maxProperties"], 1);
        let v = validator_of(&schema);
        assert!(
            v.is_valid(&json!({ "parameter": { "temperature": 0.7 } })),
            "exactly one pair validates"
        );
        assert!(
            !v.is_valid(&json!({ "parameter": {} })),
            "zero keys fail min-keys"
        );
        assert!(
            !v.is_valid(&json!({ "parameter": { "a": 1, "b": 2 } })),
            "two keys fail max-keys"
        );
    }

    // ── Feature D — content-format string types ──────────────────────────

    #[test]
    fn yaml_type_lowers_to_content_format() {
        let v = atom_value("yaml");
        assert_eq!(v["type"], "string");
        assert_eq!(v["format"], "darkmatter-yaml");
    }

    #[test]
    fn json_type_lowers_to_content_format() {
        let v = atom_value("json");
        assert_eq!(v["type"], "string");
        assert_eq!(v["format"], "darkmatter-json");
    }

    // ── literal / expression (2026-07-12-literal-expression, Phase 3) ─────

    #[test]
    fn literal_string_lowers_to_const() {
        let v = atom_value("literal(spec)");
        assert_eq!(v["const"], "spec");
    }

    #[test]
    fn literal_number_and_boolean_preserve_scalar_type() {
        let n = atom_value("literal(2)");
        assert_eq!(n["const"], json!(2));
        assert!(n["const"].is_number());

        let b = atom_value("literal(false)");
        assert_eq!(b["const"], json!(false));
        assert!(b["const"].is_boolean());

        // Quoting and leading zeros opt out of typing → string const.
        assert_eq!(atom_value("literal('2')")["const"], json!("2"));
        assert_eq!(atom_value("literal(007)")["const"], json!("007"));
    }

    #[test]
    fn literal_array_places_const_under_items() {
        let v = atom_value("literal(auto)[]");
        assert_eq!(v["type"], "array");
        assert_eq!(v["items"]["const"], "auto");
    }

    #[test]
    fn optional_literal_wraps_const_in_nullable_any_of() {
        // A non-required literal accepts missing/null via the shared wrapper.
        let v = optional_atom_value("literal(spec)");
        assert_eq!(v["anyOf"][0]["type"], "null");
        assert_eq!(v["anyOf"][1]["const"], "spec");
    }

    #[test]
    fn literal_equal_default_loads_and_mismatch_errors() {
        // An equal default is fine.
        assert!(parse_type_expr("kind", "literal(spec; default(spec))")
            .and_then(|a| atom_to_schema("kind", &a))
            .is_ok());
        // A default that violates its own const fails conversion.
        let atom = parse_type_expr("kind", "literal(spec; default(other))").expect("parse");
        let err = atom_to_schema("kind", &atom).expect_err("mismatched default must fail");
        assert!(matches!(err, SchemaError::Convert { .. }));
    }

    #[test]
    fn literal_rejects_unrelated_constraint() {
        // `min(1)` is meaningless on a literal identity and must be rejected at
        // conversion (the grammar admits it; convert is the enforcement point).
        let atom = parse_type_expr("v", "literal(2; min(1))").expect("parse");
        let err = atom_to_schema("v", &atom).expect_err("min not valid on literal");
        assert!(matches!(err, SchemaError::Convert { .. }));
    }

    #[test]
    fn expression_lowers_to_string_with_format() {
        let v = atom_value("expression");
        assert_eq!(v["type"], "string");
        assert_eq!(v["format"], "darkmatter-expression");
    }

    #[test]
    fn expression_default_must_parse() {
        // A parseable default loads.
        let ok = parse_type_expr("when", "expression(default('is_agent()'))").expect("parse");
        assert!(atom_to_schema("when", &ok).is_ok());
        // An unparseable default fails at schema load.
        let bad = parse_type_expr("when", "expression(default('(('))").expect("parse");
        let err = atom_to_schema("when", &bad).expect_err("bad default must fail");
        assert!(matches!(err, SchemaError::Convert { .. }));
    }

    #[test]
    fn property_union_mixes_literal_with_atom() {
        // `[literal(auto), number(min(1))]` — a literal arm beside a numeric arm.
        let v = convert("width:\n  - literal(auto)\n  - 'number(min(1))'\n");
        let arms = v["properties"]["width"]["anyOf"].as_array().expect("anyOf");
        // Optional union prepends a null arm; the literal and number arms follow.
        assert!(arms.iter().any(|a| a["const"] == "auto"));
        assert!(arms.iter().any(|a| a["type"] == "number"));
    }

    #[test]
    fn inline_object_discriminant_carries_const() {
        // A tagged-union arm: the required `kind` discriminant lowers to a const.
        let v = convert(
            "event: '{ kind: literal(created; required), path: file(required) }'\n",
        );
        let event = &v["properties"]["event"];
        // Optional inline object → nullable wrapper; the object is the typed arm.
        let obj = &event["anyOf"][1];
        assert_eq!(obj["properties"]["kind"]["const"], "created");
        assert!(
            obj["required"]
                .as_array()
                .is_some_and(|r| r.iter().any(|k| k == "kind")),
            "the required discriminant is listed: {obj}"
        );
    }

    #[test]
    fn literal_large_integer_const_is_exact() {
        // Bare integer literals beyond f64's 2^53 exact range must reach `const`
        // verbatim — `normalize_json_number` must not round them through f64.
        let cases: [(&str, Value); 4] = [
            ("literal(9007199254740993)", Value::from(9_007_199_254_740_993_i64)),
            ("literal(9223372036854775807)", Value::from(i64::MAX)),
            ("literal(9223372036854775808)", Value::from(9_223_372_036_854_775_808_u64)),
            ("literal(18446744073709551615)", Value::from(u64::MAX)),
        ];
        for (input, expected) in cases {
            assert_eq!(atom_value(input)["const"], expected, "const for {input}");
        }
    }

    #[test]
    fn integral_float_default_still_tidies() {
        // Guard against a regression: an integral `default(3)` still emits `3`,
        // not `3.0`.
        assert_eq!(atom_value("number(default(3))")["default"], Value::from(3_i64));
    }

    #[test]
    fn large_integer_default_equal_to_literal_accepted() {
        // An equal large-integer `default(...)` must compile (values compared
        // exactly), and the const survives.
        let v = atom_value("literal(9007199254740993; default(9007199254740993))");
        assert_eq!(v["const"], Value::from(9_007_199_254_740_993_i64));
        assert_eq!(v["default"], Value::from(9_007_199_254_740_993_i64));
    }

    #[test]
    fn large_integer_default_unequal_to_literal_rejected() {
        let mut atom =
            parse_type_expr("test", "literal(9007199254740993; default(9007199254740992))")
                .expect("parse atom");
        atom.constraints.push(Constraint::Required);
        assert!(
            atom_to_schema("test", &atom).is_err(),
            "an off-by-one large-integer default must fail schema load"
        );
    }
}
