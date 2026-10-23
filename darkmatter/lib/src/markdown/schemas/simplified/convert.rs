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
//! - **`x-darkmatter-*` extensions** include the URL-scheme validator and the
//!   grammar-backed semantic meta-type keywords. `file(match(...))` is
//!   suggestion metadata in a single schema. In a root union each simplified
//!   arm's declared globs lower to `x-darkmatter-match` on its file fragment
//!   and rule that arm out for an existing file outside them (see
//!   [`crate::markdown::schemas::file_match`]).
//! - **Repeated constraints on one atom never overwrite a keyword.** Patterns
//!   (including `not-empty`) are conjunctive, bounds keep the stricter value
//!   per kind, `scheme(...)` lists intersect, and repeated `default(...)`
//!   values must agree. Contradictions are [`SchemaError::Convert`].
//! - **`file` existence posture** is carried by the emitted `format`: bare
//!   `file` lowers to the lazy `darkmatter-file-reference` (syntax-only),
//!   `file(eager)` lowers to the eager `darkmatter-file` (resolve + exists).
//! - **Tuples** lower to `prefixItems` / `minItems` / `items` with no
//!   optional-null wrapper on positions; **`function`** types are
//!   description-only and fail conversion, naming the property path.
//! - **`category(Name)`** lowers to the `x-darkmatter-category` annotation.
//! - **Root-level unions** with unresolved [`SchemaArm::FileRef`] arms fail
//!   with [`SchemaError::Convert`] — Phase 3's resolver is expected to inline
//!   them before invoking the converter.
//!
//! See the mapping table in `darkmatter/features/2026-05-11-schemas/spec.md`
//! for the full row-by-row contract; the [`tests::snapshots`] module in this
//! file mirrors that table.

use serde_json::{Map, Value, json};

use super::alphanumeric::AlphanumericOptions;
use super::types::{
    Constraint, PatternKey, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema,
    SimplifiedType, TupleType, TypeExpr,
};
use crate::markdown::numeric::json_number;
use crate::markdown::compose::expression::parse_condition;
use crate::markdown::schemas::errors::SchemaError;
use crate::markdown::schemas::file_match::{DARKMATTER_MATCH_KEYWORD, file_match_groups};
use crate::markdown::schemas::format::{
    DARKMATTER_INTEGER_KEYWORD,
    DARKMATTER_BOOLISH_FORMAT, DARKMATTER_DATETIME_FORMAT, DARKMATTER_NUMBERLIKE_FORMAT, DARKMATTER_EXPRESSION_FORMAT, DARKMATTER_FILE_FORMAT,
    DARKMATTER_FILE_REFERENCE_FORMAT, DARKMATTER_JSON_FORMAT, DARKMATTER_SCHEMA_KEYWORD,
    DARKMATTER_SHELL_FORMAT, DARKMATTER_TYPE_ANNOTATION,
    DARKMATTER_TIME_FORMAT, DARKMATTER_TYPE_DEFINITION_KEYWORD, DARKMATTER_YAML_FORMAT,
};

/// Draft 2020-12 schema URI emitted on every generated root schema.
pub const DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

/// Converts a [`SimplifiedSchema`] into a Draft 2020-12 JSON Schema value.
///
/// ## Errors
///
/// Returns [`SchemaError::Convert`] when:
///
/// - a constraint is applied to a type that doesn't accept it
///   (e.g. `integer` on a `string`),
/// - a property-level union has conflicting `default(...)` values,
/// - a root-level union arm is an unresolved [`SchemaArm::FileRef`],
/// - the schema reaches a description-only `function` type, directly or
///   through a union, array, tuple, or object (named references are inlined
///   by the resolver before conversion); the error names the property path.
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
    let root = match schema {
        SimplifiedSchema::Single(shape) => shape_to_object_schema(shape)?,
        SimplifiedSchema::Union(arms) => union_to_root_schema(arms)?,
    };
    // `$schema` leads the document, as JSON Schema readers expect, now that
    // maps keep insertion order.
    let mut document = Map::with_capacity(root.len() + 1);
    document.insert("$schema".into(), Value::String(DRAFT_2020_12.into()));
    document.extend(root);
    Ok(Value::Object(document))
}

// ── Shape & root union ────────────────────────────────────────────────────

fn shape_to_object_schema(shape: &SchemaShape) -> Result<Map<String, Value>, SchemaError> {
    // Root shapes default to open (`additionalProperties: true`); a pattern-keyed
    // root switches to closed-object semantics inside `object_body_from_shape`.
    let mut obj = object_body_from_shape("<root>", shape, Value::Bool(true))?;
    reject_block_constraints("<root>", shape, ObjectScope::Root)?;
    apply_object_arity("<root>", &mut obj, shape.constraints.iter())?;
    Ok(obj)
}

fn union_to_root_schema(arms: &[SchemaArm]) -> Result<Map<String, Value>, SchemaError> {
    let mut any_of = Vec::with_capacity(arms.len());
    for (idx, arm) in arms.iter().enumerate() {
        match arm {
            SchemaArm::Inline(shape) => {
                let mut arm_json = Value::Object(shape_to_object_schema(shape)?);
                attach_root_union_match(&mut arm_json, shape);
                any_of.push(arm_json);
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

/// Emits a root-union arm's declared file globs onto its converted fragments.
/// Called for each simplified arm even when other arms use raw JSON Schema.
///
/// Every `match(...)` group is emitted in one keyword: a single group keeps
/// the flat pattern array, several become an array of pattern arrays (see
/// [`crate::markdown::schemas::file_match`], "Keyword encoding"). One keyword,
/// not an `allOf` of keywords, because root-union arm selection recognizes a
/// glob rejection by a schema path that ends in the keyword.
pub(crate) fn attach_root_union_match(arm_schema: &mut Value, shape: &SchemaShape) {
    let Some(Value::Object(properties)) = arm_schema.get_mut("properties") else {
        return;
    };
    for (name, fragment) in properties.iter_mut() {
        if let Some(groups) = shape.properties.get(name).and_then(file_match_groups) {
            let keyword = match groups.as_slice() {
                [only] => json!(only),
                several => json!(several),
            };
            attach_match_keyword(fragment, &keyword);
        }
    }
}

fn attach_match_keyword(fragment: &mut Value, keyword: &Value) {
    match fragment {
        Value::Object(map) => {
            if map.get("format").and_then(Value::as_str).is_some_and(|format| {
                format == DARKMATTER_FILE_FORMAT || format == DARKMATTER_FILE_REFERENCE_FORMAT
            }) {
                map.insert(DARKMATTER_MATCH_KEYWORD.into(), keyword.clone());
            }
            for key in ["items", "anyOf", "oneOf", "allOf"] {
                if let Some(nested) = map.get_mut(key) {
                    attach_match_keyword(nested, keyword);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                attach_match_keyword(item, keyword);
            }
        }
        _ => {}
    }
}

// ── Property-level dispatch (single vs union) ────────────────────────────

fn property_def_to_schema(name: &str, def: &PropertyDef) -> Result<(Value, bool), SchemaError> {
    match def {
        // A postfixed union reference without `[]` (`U(required)`, `U -> text`),
        // or a union whose flattened postfixed arm hoisted its markers
        // (`[U(required), T]`), is a property union whose markers and
        // description sit on `atom`.
        PropertyDef::Single(atom) => match atom.union_arms() {
            Some(arms) if !atom.is_array => union_property_to_schema(name, arms, Some(atom)),
            _ => atom_to_schema(name, atom),
        },
        PropertyDef::Union(arms) => union_property_to_schema(name, arms, None),
    }
}

/// Lowers a property-level union. `container` is the atom of a postfixed
/// union reference, whose markers apply to the property as a whole and whose
/// description describes the union rather than any arm.
fn union_property_to_schema(
    name: &str,
    arms: &[PropertyAtom],
    container: Option<&PropertyAtom>,
) -> Result<(Value, bool), SchemaError> {
    // Union requiredness must be known *before* building arms: a required
    // union (any arm carries `required`, and no arm admits null) gets neither
    // the property-level `null` arm nor the optional-`file` empty-string
    // tolerance. Deciding this up front lets each arm be built with
    // empty-`file` suppression so a required union containing an
    // otherwise-optional `file` arm still rejects the `""` sentinel.
    let admits_null = container.map_or_else(
        || arms.iter().any(PropertyAtom::admits_null),
        PropertyAtom::admits_null,
    );
    let required = !admits_null
        && container.map_or_else(
            || arms.iter().any(PropertyAtom::marks_required),
            PropertyAtom::marks_required,
        );
    // `generated` is a property-level ownership semantic: if any arm is
    // host-supplied, the static `required` entry is suppressed (spec point 1).
    let any_generated = container.map_or_else(
        || arms.iter().any(PropertyAtom::is_generated),
        PropertyAtom::is_generated,
    );

    // The optional-`file` empty-string arm is allowed only when the union
    // itself is optional.
    let (any_of, hoisted_default) = union_arm_fragments(name, arms, !required)?;

    let mut union_schema = Map::new();
    union_schema.insert("anyOf".into(), Value::Array(any_of));
    if let Some(default_val) = hoisted_default {
        union_schema.insert("default".into(), default_val);
    }
    if let Some(description) = container.and_then(|atom| atom.description.as_ref()) {
        union_schema.insert("description".into(), Value::String(description.clone()));
    }
    // Surface the ownership semantic on the property's schema object so
    // downstream tooling (LSP, completion, runtime validators) discovers it
    // without drilling into arms.
    if any_generated {
        union_schema.insert("x-darkmatter-generated".into(), Value::Bool(true));
    }

    // A union with a `null` or `unknown` arm already accepts null, so it needs
    // no extra `null` arm.
    let schema = if required || admits_null {
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

/// Builds one fragment per union arm and hoists the arms' `default` values,
/// which must agree, off them.
///
/// Each arm uses the bare fragment builder, so an optional `null` wrapper is
/// applied once by the caller, never per arm.
fn union_arm_fragments(
    name: &str,
    arms: &[PropertyAtom],
    allow_optional_empty_file: bool,
) -> Result<(Vec<Value>, Option<Value>), SchemaError> {
    let mut any_of = Vec::with_capacity(arms.len());
    let mut hoisted_default: Option<Value> = None;

    for atom in arms {
        let (mut arm_value, _arm_required) =
            atom_fragment_without_null_wrap(name, atom, allow_optional_empty_file)?;
        // `atom_fragment_without_null_wrap` always returns a `Value::Object`.
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
    Ok((any_of, hoisted_default))
}

// ── Atom → JSON Schema fragment ──────────────────────────────────────────

/// Wraps a finished typed fragment so an optional property also accepts
/// JSON `null` as a sentinel for "absent".
fn wrap_optional_null(inner: Value) -> Value {
    json!({ "anyOf": [ { "type": "null" }, inner ] })
}

fn atom_to_schema(name: &str, atom: &PropertyAtom) -> Result<(Value, bool), SchemaError> {
    let (mut fragment, non_nullable) = atom_fragment_without_null_wrap(name, atom, true)?;
    let generated = atom.is_generated();

    // `null` and `unknown` already accept null and are never required, so they
    // lower bare: `{"type": "null"}` and `{}`.
    if atom.admits_null() {
        return Ok((fragment, false));
    }

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
    let mut category_val: Option<Value> = None;
    if let Value::Object(map) = &mut fragment {
        default_val = map.remove("default");
        description_val = map.remove("description");
        generated_val = map.remove("x-darkmatter-generated");
        example_val = map.remove("x-darkmatter-example");
        category_val = map.remove(DARKMATTER_CATEGORY_KEYWORD);
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
        if let Some(category) = category_val {
            map.insert(DARKMATTER_CATEGORY_KEYWORD.into(), category);
        }
    }

    // An optional atom is never in the parent's static `required` array,
    // regardless of `generated`.
    Ok((schema, false))
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
    // (single/value-level) or array_constraints (array-level). Repeated
    // defaults must agree after number normalization (`3` == `3.0`).
    // `Generated` is similarly hoisted: it only surfaces as the
    // `x-darkmatter-generated: true` annotation, never as a typed fragment key.
    let required = atom.is_required();
    let generated = atom.is_generated();
    let mut default_val: Option<Value> = None;
    // `example(...)` is documentation, not validation: the raw authored
    // reference strings are hoisted onto the `x-darkmatter-example` annotation
    // (resolved into example objects by `resolve.rs`) and never reach a type
    // fragment builder as a constraint.
    let mut examples: Vec<String> = Vec::new();
    let mut category: Option<&str> = None;
    for c in atom.property_constraints() {
        match c {
            Constraint::Category(name) => category = Some(name),
            Constraint::Default(v) => {
                let value = normalize_json_number(v.clone());
                match &default_val {
                    Some(existing) if existing != &value => {
                        return Err(SchemaError::Convert {
                            property: name.to_string(),
                            message: format!(
                                "conflicting `default(...)` values: {existing} vs {value}"
                            ),
                        });
                    }
                    Some(_) => {}
                    None => default_val = Some(value),
                }
            }
            Constraint::Example(refs) => examples.extend(refs.iter().cloned()),
            _ => {}
        }
    }

    // `category(...)` is an annotation on the property, not a constraint any
    // type builder checks.
    let value_constraints: Vec<Constraint> = atom
        .constraints
        .iter()
        .filter(|constraint| !matches!(constraint, Constraint::Category(_)))
        .cloned()
        .collect();
    let inner = match &atom.ty {
        TypeExpr::Primitive(ty) => type_fragment(name, *ty, &value_constraints)?,
        TypeExpr::InlineObject(shape) => inline_object_fragment(name, shape, &value_constraints)?,
        TypeExpr::Tuple(tuple) => tuple_fragment(name, tuple)?,
        TypeExpr::Function(_) => {
            return Err(SchemaError::Convert {
                property: name.to_string(),
                message: "a `function` type is description-only: it describes a callable and \
                          cannot type a value in a document schema"
                    .into(),
            });
        }
        // The alternatives of each item (with `is_array`) or of a union
        // nested as an arm. Items get no `null` wrapper or empty-file arm,
        // like the items of any array.
        TypeExpr::Union(arms) => {
            let (any_of, arm_default) = union_arm_fragments(
                name,
                arms,
                allow_optional_empty_file && !atom.is_array && !required,
            )?;
            if let Some(value) = arm_default {
                // The resolver lifts an array-compatible arm default to the
                // container of `U[]` and rejects any other.
                if atom.is_array {
                    return Err(SchemaError::Convert {
                        property: name.to_string(),
                        message: format!(
                            "an item alternative carries `default({value})`, which cannot \
                             default the array"
                        ),
                    });
                }
                match &default_val {
                    Some(existing) if existing != &value => {
                        return Err(SchemaError::Convert {
                            property: name.to_string(),
                            message: format!(
                                "conflicting `default(...)` values: {existing} vs {value}"
                            ),
                        });
                    }
                    Some(_) => {}
                    None => default_val = Some(value),
                }
            }
            json!({ "anyOf": any_of })
        }
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
        if let Some(category) = category {
            map.insert(DARKMATTER_CATEGORY_KEYWORD.into(), Value::String(category.to_string()));
        }
    }

    Ok((fragment, required))
}

/// The annotation a `category(Name)` lowers to on a data type.
pub const DARKMATTER_CATEGORY_KEYWORD: &str = "x-darkmatter-category";

/// Lowers a tuple to `type: array` with `prefixItems` for its positions,
/// `minItems` for its required positions, and `items` for what may follow:
/// `false` for a fixed length, the spread's item schema otherwise.
///
/// Positions get the bare fragment, without the optional-null wrapper: an
/// optional position may be omitted, but a supplied `null` must match its
/// type. An empty tuple writes no `prefixItems`, which Draft 2020-12 requires
/// to be non-empty.
fn tuple_fragment(name: &str, tuple: &TupleType) -> Result<Value, SchemaError> {
    let mut prefix = Vec::with_capacity(tuple.elements.len());
    for (index, element) in tuple.elements.iter().enumerate() {
        prefix.push(pattern_value_schema(&format!("{name}[{index}]"), &element.def)?);
    }
    let items = match &tuple.spread {
        Some(spread) => {
            let mut item = spread.as_ref().clone();
            item.is_array = false;
            item.array_constraints.clear();
            atom_fragment_without_null_wrap(&format!("{name}[...]"), &item, false)?.0
        }
        None => Value::Bool(false),
    };
    let mut m = Map::new();
    m.insert("type".into(), Value::String("array".into()));
    if !prefix.is_empty() {
        m.insert("prefixItems".into(), Value::Array(prefix));
    }
    m.insert("minItems".into(), json!(tuple.required_len()));
    m.insert("items".into(), items);
    Ok(Value::Object(m))
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
            | Constraint::Eager
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
    reject_block_constraints(name, shape, ObjectScope::Inline)?;
    apply_object_arity(name, &mut obj, shape.constraints.iter().chain(atom_constraints))?;
    Ok(Value::Object(obj))
}

/// Where a `$constraints` block sits, which decides the keys it may hold.
#[derive(Clone, Copy)]
enum ObjectScope {
    /// The block of an object-typed property. Its `required` / `generated`
    /// were moved onto the property's atom by the parser.
    Inline,
    /// The root shape (or a root-union arm), which has no enclosing property.
    Root,
}

/// Rejects `$constraints` keys that have no meaning on the block's object.
/// The block parser accepts every key the grammar knows; only `min-keys` /
/// `max-keys` lower here.
fn reject_block_constraints(
    name: &str,
    shape: &SchemaShape,
    scope: ObjectScope,
) -> Result<(), SchemaError> {
    for c in &shape.constraints {
        if !matches!(c, Constraint::MinKeys(_) | Constraint::MaxKeys(_)) {
            let place = match scope {
                ObjectScope::Inline => "an inline object",
                ObjectScope::Root => "the root object",
            };
            return Err(SchemaError::Convert {
                property: name.to_string(),
                message: format!("`$constraints` key `{}` is not valid on {place}", block_key(c)),
            });
        }
    }
    Ok(())
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
    let mut errors = Vec::new();
    for (prop_name, def) in &shape.properties {
        let qualified_name = if context == "<root>" {
            prop_name.clone()
        } else {
            format!("{context}.{prop_name}")
        };
        match property_def_to_schema(&qualified_name, def) {
            Ok((prop_schema, is_required)) => {
                if is_required {
                    required.push(Value::String(prop_name.clone()));
                }
                properties.insert(prop_name.clone(), prop_schema);
            }
            Err(SchemaError::Aggregate { errors: children }) => errors.extend(children),
            Err(error) => errors.push(error),
        }
    }
    if !errors.is_empty() {
        return Err(if errors.len() == 1 {
            errors.pop().expect("one conversion error")
        } else {
            SchemaError::Aggregate { errors }
        });
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
///
/// A pattern that compiled once is remembered for the process: a named type
/// inlined at many sites (the baseline's lifecycle stacks) emits the same
/// pattern at each, and compiling a long alternation is not free.
fn validate_emitted_pattern(
    context: &str,
    key: &PatternKey,
    emitted: &str,
) -> Result<(), SchemaError> {
    static COMPILED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::OnceLock::new();
    let compiled = COMPILED.get_or_init(Default::default);
    if compiled.lock().expect("compiled pattern set poisoned").contains(emitted) {
        return Ok(());
    }
    let compiles = if has_lookaround(emitted) {
        fancy_regex::Regex::new(emitted).is_ok()
    } else {
        regex::Regex::new(emitted).is_ok()
    };
    if compiles {
        compiled.lock().expect("compiled pattern set poisoned").insert(emitted.to_string());
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
/// arity constraints (Feature C / O-C3). The `$constraints` block and the
/// postfix form are one scope, so repeated bounds from either surface merge
/// as [`Bounds`] do. Other constraints are ignored here: they are hoisted
/// (`required` / `default` / `generated`) or rejected elsewhere.
fn apply_object_arity<'c>(
    name: &str,
    obj: &mut Map<String, Value>,
    constraints: impl Iterator<Item = &'c Constraint>,
) -> Result<(), SchemaError> {
    let mut keys = Bounds::default();
    for c in constraints {
        match c {
            Constraint::MinKeys(n) => keys.lower(*n, c),
            Constraint::MaxKeys(n) => keys.upper(*n, c),
            _ => {}
        }
    }
    keys.emit(name, obj, "minProperties", "maxProperties", |n| json!(n))
}

/// Repeated bounds of one kind on one scope (string length, number, item
/// count, or key count). The larger lower bound and the smaller upper bound
/// win; equal bounds are valid; a lower bound above the upper bound is a
/// [`SchemaError::Convert`] naming both constraints.
///
/// This is the backstop for repeats authored on one atom. Bounds merged from
/// a reused named type are checked earlier by the resolver, which knows
/// where each one came from.
struct Bounds<'c, T> {
    lower: Option<(T, &'c Constraint)>,
    upper: Option<(T, &'c Constraint)>,
}

impl<T> Default for Bounds<'_, T> {
    fn default() -> Self {
        Self { lower: None, upper: None }
    }
}

impl<'c, T: PartialOrd + Copy> Bounds<'c, T> {
    fn lower(&mut self, value: T, constraint: &'c Constraint) {
        if self.lower.is_none_or(|(current, _)| value > current) {
            self.lower = Some((value, constraint));
        }
    }

    fn upper(&mut self, value: T, constraint: &'c Constraint) {
        if self.upper.is_none_or(|(current, _)| value < current) {
            self.upper = Some((value, constraint));
        }
    }

    fn emit(
        self,
        name: &str,
        map: &mut Map<String, Value>,
        lower_key: &str,
        upper_key: &str,
        to_json: impl Fn(T) -> Value,
    ) -> Result<(), SchemaError> {
        if let (Some((low, low_c)), Some((high, high_c))) = (self.lower, self.upper)
            && low > high
        {
            return Err(SchemaError::Convert {
                property: name.to_string(),
                message: format!(
                    "contradictory bounds: `{}` is greater than `{}`",
                    bound_display(low_c),
                    bound_display(high_c)
                ),
            });
        }
        if let Some((low, _)) = self.lower {
            map.insert(lower_key.into(), to_json(low));
        }
        if let Some((high, _)) = self.upper {
            map.insert(upper_key.into(), to_json(high));
        }
        Ok(())
    }
}

/// A `$constraints` entry's key as authored; the item-count variants share
/// the `min` / `max` grammar keyword with other bounds.
fn block_key(c: &Constraint) -> &'static str {
    match c {
        Constraint::MinItems(_) => "min-items",
        Constraint::MaxItems(_) => "max-items",
        other => other.keyword(),
    }
}

/// A bound constraint as authored, for contradiction messages.
fn bound_display(c: &Constraint) -> String {
    let value = match c {
        Constraint::Min(n) | Constraint::Max(n) => number_to_json(*n).to_string(),
        Constraint::MinLen(n)
        | Constraint::MaxLen(n)
        | Constraint::MinItems(n)
        | Constraint::MaxItems(n)
        | Constraint::MinKeys(n)
        | Constraint::MaxKeys(n) => n.to_string(),
        other => return other.keyword().to_string(),
    };
    format!("{}({value})", c.keyword())
}

fn apply_array_constraints(
    name: &str,
    arr: &mut Map<String, Value>,
    constraints: &[Constraint],
) -> Result<(), SchemaError> {
    let mut items = Bounds::default();
    for c in constraints {
        match c {
            Constraint::Required
            | Constraint::Default(_)
            | Constraint::Generated
            | Constraint::Example(_)
            | Constraint::Category(_)
            | Constraint::Eager => {
                // Hoisted to the property level by `atom_to_schema`.
            }
            Constraint::MinItems(n) => items.lower(*n, c),
            Constraint::MaxItems(n) => items.upper(*n, c),
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
    items.emit(name, arr, "minItems", "maxItems", |n| json!(n))
}

// ── Per-type fragment builders ───────────────────────────────────────────

fn type_fragment(
    name: &str,
    ty: SimplifiedType,
    constraints: &[Constraint],
) -> Result<Value, SchemaError> {
    // `eager` is universal timing metadata rather than a presence constraint,
    // so it has no JSON Schema form of its own. Only `file` also assigns it
    // type-specific validation semantics (existence), so other fragment
    // builders need not repeat a no-op match arm.
    let filtered;
    let constraints = if ty == SimplifiedType::File {
        constraints
    } else {
        filtered = constraints
            .iter()
            .filter(|constraint| !matches!(constraint, Constraint::Eager))
            .cloned()
            .collect::<Vec<_>>();
        &filtered
    };
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
        SimplifiedType::TypeDefinition => semantic_type_fragment(
            name,
            "type-definition",
            DARKMATTER_TYPE_DEFINITION_KEYWORD,
            constraints,
        ),
        SimplifiedType::Schema => {
            semantic_type_fragment(name, "schema", DARKMATTER_SCHEMA_KEYWORD, constraints)
        }
        SimplifiedType::Unknown => unknown_fragment(name, constraints),
        SimplifiedType::Null => null_fragment(name, constraints),
        SimplifiedType::Alphanumeric => alphanumeric_fragment(name, constraints),
        SimplifiedType::Shell => shell_fragment(name, constraints),
    }
}

fn string_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    string_fragment_with_base(name, "string", None, constraints)
}

/// A string fragment whose value must also match `base_pattern`, which stays
/// separate from every authored pattern.
fn string_fragment_with_base(
    name: &str,
    type_label: &str,
    base_pattern: Option<String>,
    constraints: &[Constraint],
) -> Result<Value, SchemaError> {
    let mut m = Map::new();
    m.insert("type".into(), Value::String("string".into()));
    let mut length = Bounds::default();
    let mut patterns: Vec<String> = base_pattern.into_iter().collect();
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
            Constraint::MinLen(n) => length.lower(*n, c),
            Constraint::MaxLen(n) => length.upper(*n, c),
            Constraint::NotEmpty => {
                // Lookaround-free: any string containing at least one
                // non-whitespace character. A lookaround-free pattern keeps this
                // schema on jsonschema's linear (ReDoS-safe) `regex` engine —
                // only Feature C pattern-key literal precedence opts a schema
                // into `fancy-regex` — so the previous `^(?!\s*$).+` form (which
                // the linear engine rejects) is not portable and `\S` is used.
                push_unique(&mut patterns, r"\S");
            }
            Constraint::Pattern(p) => push_unique(&mut patterns, p),
            other => return Err(invalid_constraint(name, type_label, other)),
        }
    }
    length.emit(name, &mut m, "minLength", "maxLength", |n| json!(n))?;
    insert_patterns(&mut m, patterns);
    Ok(Value::Object(m))
}

fn push_unique(patterns: &mut Vec<String>, pattern: &str) {
    if !patterns.iter().any(|existing| existing == pattern) {
        patterns.push(pattern.to_string());
    }
}

/// A value must satisfy every pattern. One pattern keeps the plain `pattern`
/// keyword; several become `allOf` entries beside the base fragment's other
/// keywords, never one combined regex.
fn insert_patterns(m: &mut Map<String, Value>, patterns: Vec<String>) {
    match <[String; 1]>::try_from(patterns) {
        Ok([only]) => {
            m.insert("pattern".into(), Value::String(only));
        }
        Err(patterns) if !patterns.is_empty() => {
            m.insert(
                "allOf".into(),
                Value::Array(
                    patterns
                        .into_iter()
                        .map(|pattern| json!({ "pattern": pattern }))
                        .collect(),
                ),
            );
        }
        Err(_) => {}
    }
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
    let mut range = Bounds::default();
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
            Constraint::Min(n) => range.lower(*n, c),
            Constraint::Max(n) => range.upper(*n, c),
            Constraint::Integer => is_integer = true,
            other => return Err(invalid_constraint(name, "number", other)),
        }
    }
    range.emit(name, &mut m, "minimum", "maximum", number_to_json)?;
    m.insert(
        "type".into(),
        Value::String(if is_integer { "integer" } else { "number" }.into()),
    );
    if is_integer {
        m.insert(DARKMATTER_INTEGER_KEYWORD.into(), Value::Bool(true));
    }
    Ok(Value::Object(m))
}

/// `numberlike` accepts a real number or number text and keeps it as sent,
/// so it compiles to both JSON types plus the format backed by the canonical
/// numeric-text parser; a regex cannot express the exactness check.
fn numberlike_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    reject_unsupported(name, "numberlike", constraints, &[])?;
    Ok(json!({ "type": ["number", "string"], "format": DARKMATTER_NUMBERLIKE_FORMAT }))
}

/// `boolish` accepts a real boolean or a boolean word and keeps it as sent.
fn boolish_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    reject_unsupported(name, "boolish", constraints, &[])?;
    Ok(json!({ "type": ["boolean", "string"], "format": DARKMATTER_BOOLISH_FORMAT }))
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
            // Root-union conversion emits `match(...)` after this atom has
            // been converted; a single schema uses it for completion only.
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
    // Repeated `scheme(...)` lists intersect, in the first list's order.
    let mut allowed: Option<Vec<String>> = None;
    let mut authored: Vec<String> = Vec::new();
    for c in constraints {
        match c {
            Constraint::Required
            | Constraint::Default(_)
            | Constraint::Generated
            | Constraint::Example(_) => {}
            Constraint::Scheme(schemes) => {
                authored.push(format!("scheme({})", schemes.join(",")));
                allowed = Some(match allowed {
                    None => schemes.clone(),
                    Some(mut current) => {
                        current.retain(|scheme| schemes.contains(scheme));
                        current
                    }
                });
            }
            other => return Err(invalid_constraint(name, "url", other)),
        }
    }
    if let Some(schemes) = allowed {
        if schemes.is_empty() {
            return Err(SchemaError::Convert {
                property: name.to_string(),
                message: format!(
                    "contradictory constraints: {} share no scheme",
                    authored
                        .iter()
                        .map(|c| format!("`{c}`"))
                        .collect::<Vec<_>>()
                        .join(" and ")
                ),
            });
        }
        m.insert(
            "x-darkmatter-url-scheme".into(),
            Value::Array(schemes.into_iter().map(Value::String).collect()),
        );
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

fn semantic_type_fragment(
    name: &str,
    type_label: &str,
    keyword: &str,
    constraints: &[Constraint],
) -> Result<Value, SchemaError> {
    reject_unsupported(name, type_label, constraints, &[])?;
    let mut fragment = Map::new();
    fragment.insert("type".into(), json!(["string", "object", "array"]));
    fragment.insert(keyword.into(), Value::Bool(true));
    Ok(Value::Object(fragment))
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

fn unknown_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    reject_unsupported(name, "unknown", constraints, &[])?;
    Ok(Value::Object(Map::new()))
}

/// Lowers `null` to `{ "type": "null" }`. A default must itself be null.
fn null_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    reject_unsupported(name, "null", constraints, &[])?;
    if let Some(default) = constraints.iter().find_map(|c| match c {
        Constraint::Default(value) if !value.is_null() => Some(value),
        _ => None,
    }) {
        return Err(SchemaError::Convert {
            property: name.to_string(),
            message: format!("`default({default})` is not valid on `null`; a `null` default must be null"),
        });
    }
    Ok(json!({ "type": "null" }))
}

/// Lowers `alphanumeric` to a string matching its canonical options' regex
/// beside every authored string constraint. The type and its canonical
/// options are recorded as non-validating annotations so a JSON consumer can
/// recover the type identity.
fn alphanumeric_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    let options = constraints
        .iter()
        .filter_map(|c| match c {
            Constraint::Options(options) => Some(options.iter().copied()),
            _ => None,
        })
        .flatten()
        .collect::<AlphanumericOptions>()
        .canonical();
    let rest: Vec<Constraint> = constraints
        .iter()
        .filter(|c| !matches!(c, Constraint::Options(_)))
        .cloned()
        .collect();
    let mut fragment =
        string_fragment_with_base(name, "alphanumeric", Some(options.pattern()), &rest)?;
    check_alphanumeric_values(name, &fragment, &rest)?;
    if let Value::Object(map) = &mut fragment {
        map.insert(
            DARKMATTER_TYPE_ANNOTATION.into(),
            json!(SimplifiedType::Alphanumeric.as_keyword()),
        );
        map.insert(
            "x-darkmatter-options".into(),
            Value::Array(options.keywords().map(|keyword| json!(keyword)).collect()),
        );
    }
    Ok(fragment)
}

/// Fails the load when an `alphanumeric` `default(…)` or `suggest(…)` value is
/// not itself a valid value of the type, judged by the lowered fragment (the
/// option pattern plus every authored string constraint) without coercion.
///
/// Unlike other types' advisory suggestions, which are only linted, an invalid
/// `alphanumeric` suggestion is a schema error.
fn check_alphanumeric_values(
    name: &str,
    fragment: &Value,
    constraints: &[Constraint],
) -> Result<(), SchemaError> {
    let mut values = Vec::new();
    for constraint in constraints {
        match constraint {
            Constraint::Default(value) => values.push(("default", value.clone())),
            Constraint::Suggest(candidates) => values.extend(
                candidates
                    .iter()
                    .map(|candidate| ("suggest", candidate.interpreted.clone())),
            ),
            _ => {}
        }
    }
    if values.is_empty() {
        return Ok(());
    }
    let mut target = fragment.clone();
    if let Value::Object(map) = &mut target {
        map.remove("default");
        map.remove("x-darkmatter-suggest");
    }
    let validator = super::super::validate::build_structural_validator(&target)?;
    match values.into_iter().find(|(_, value)| !validator.is_valid(value)) {
        Some((keyword, value)) => Err(SchemaError::Convert {
            property: name.to_string(),
            message: format!("`{keyword}({value})` is not a valid `alphanumeric` value for this type"),
        }),
        None => Ok(()),
    }
}

/// Lowers `shell` to `{ "type": "string", "format": "darkmatter-shell" }`.
/// Constraint applicability mirrors `expression`. A `default(…)` must be a
/// valid `shell` value after the text conversion supplied values receive: a
/// boolean or number default becomes its text, and a structured or null
/// default fails to load.
fn shell_fragment(name: &str, constraints: &[Constraint]) -> Result<Value, SchemaError> {
    reject_unsupported(name, "shell", constraints, &[])?;
    for c in constraints {
        let Constraint::Default(value) = c else {
            continue;
        };
        let text = match value {
            Value::String(text) => text.clone(),
            Value::Bool(flag) => flag.to_string(),
            Value::Number(number) => number.to_string(),
            Value::Null | Value::Array(_) | Value::Object(_) => {
                return Err(SchemaError::Convert {
                    property: name.to_string(),
                    message: format!(
                        "`default({value})` is not valid on `shell`; a `shell` default must be text"
                    ),
                });
            }
        };
        if text.chars().all(char::is_whitespace) {
            return Err(SchemaError::Convert {
                property: name.to_string(),
                message: format!(
                    "`default({value})` is not a valid `shell` value; a command needs a \
                     non-whitespace character"
                ),
            });
        }
    }
    Ok(json!({ "type": "string", "format": DARKMATTER_SHELL_FORMAT }))
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
            | Constraint::Example(_)
            | Constraint::Eager => {}
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

/// Emits a JSON number in its exact form: an integral value as a JSON
/// integer (`0`, not `0.0`), any other finite value as a float. A
/// non-finite value, which admission rejects, is written as text.
fn number_to_json(n: f64) -> Value {
    json_number(n).map_or_else(|| Value::String(format!("{n}")), Value::Number)
}

/// Normalises numeric values inside a default literal so integral floats
/// render as integers. Recurses into arrays/objects defensively even though
/// the grammar only emits scalars.
fn normalize_json_number(value: Value) -> Value {
    match value {
        // An integer-typed number is already exact.
        Value::Number(n) if n.is_i64() || n.is_u64() => Value::Number(n),
        Value::Number(n) => match n.as_f64().and_then(json_number) {
            Some(number) => Value::Number(number),
            None => Value::Number(n),
        },
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
    fn numberlike_compiles_to_its_format() {
        assert_eq!(
            atom_value("numberlike"),
            json!({ "type": ["number", "string"], "format": DARKMATTER_NUMBERLIKE_FORMAT })
        );
    }

    #[test]
    fn boolish_compiles_to_its_format() {
        assert_eq!(
            atom_value("boolish"),
            json!({ "type": ["boolean", "string"], "format": DARKMATTER_BOOLISH_FORMAT })
        );
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
        // arm. Outside a root union `match(...)` is not emitted
        // (completion reads it from the simplified-schema atom).
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
        let v = crate::markdown::schemas::validate::build_structural_validator(&schema).unwrap();
        assert!(v.is_valid(&json!({ "spec": "" })), "empty optional file must validate");
        assert!(v.is_valid(&json!({ "spec": null })), "null optional file must validate");
        assert!(v.is_valid(&json!({})), "absent optional file must validate");
    }

    #[test]
    fn required_file_rejects_empty_string() {
        let schema = convert("plan: 'file(required)'");
        let v = crate::markdown::schemas::validate::build_structural_validator(&schema).unwrap();
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

        let validator = crate::markdown::schemas::validate::build_structural_validator(&v).unwrap();
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
        // `file` retains the universal constraint's additional existence
        // semantics by selecting the eager `darkmatter-file` format.
        let atom = parse_type_expr("test", "file(eager; required)").unwrap();
        let v = atom_to_schema("test", &atom)
            .expect("file(eager) must convert")
            .0;
        assert_eq!(v["format"], "darkmatter-file");
    }

    #[test]
    fn eager_is_accepted_on_non_file_types_without_altering_json_schema() {
        for (input, expected_type) in [
            ("string(eager)", "string"),
            ("number(eager)", "number"),
            ("object(eager)", "object"),
        ] {
            let atom = parse_type_expr("test", input).unwrap();
            let value = atom_to_schema("test", &atom)
                .unwrap_or_else(|error| panic!("{input} must convert: {error}"))
                .0;
            assert_eq!(value["anyOf"][1]["type"], expected_type);
            assert!(value.get("x-darkmatter-eager").is_none());
        }
    }

    #[test]
    fn file_lazy_eager_required_matrix() {
        // The full 4-cell matrix from the spec's semantics table. `eager` and
        // `required` are orthogonal: `required` governs presence, `eager`
        // governs existence. A present, syntactically valid but not-yet-
        // existing path passes lazy declarations and fails eager ones.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("exists.md"), b"x").unwrap();
        let context = biscuit_file::FileResolutionContext::new(dir.path());

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
            let v = crate::markdown::schemas::validate::build_validator_in_context(
                &schema, None, None, &context,
            )
            .unwrap();
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
    fn file_malformed_reference_is_fatal_under_both_lazy_and_eager() {
        // Laziness defers *existence*, not *syntax*: a malformed reference (the
        // empty string, rejected at `FileReference` parse time) is rejected by
        // both the lazy bare `file` and the eager `file(eager)` declarations.
        let malformed = json!({ "p": "" });

        for decl in ["p: 'file(required)'", "p: 'file(eager; required)'"] {
            let schema = convert(decl);
            let v = crate::markdown::schemas::validate::build_structural_validator(&schema).unwrap();
            assert!(
                !v.is_valid(&malformed),
                "{decl}: malformed reference must be rejected"
            );
        }
    }

    #[test]
    fn file_array_lazy_accepts_missing_item_eager_rejects() {
        // Per-item posture: `file[]` accepts an array whose item is a
        // syntactically valid missing path, while `file(eager)[]` rejects the
        // same missing item (and accepts an existing one).
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("exists.md"), b"x").unwrap();

        let missing_item = json!({ "p": ["./missing.md"] });
        let existing_item = json!({ "p": ["./exists.md"] });

        let lazy = crate::markdown::schemas::validate::build_structural_validator(&convert("p: 'file[]'"))
        .unwrap();
        assert!(
            lazy.is_valid(&missing_item),
            "file[] must accept a missing syntactically valid item"
        );

        let eager = crate::markdown::schemas::validate::build_validator_in_context(
            &convert("p: 'file(eager)[]'"),
            None,
            None,
            &biscuit_file::FileResolutionContext::new(dir.path()),
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
            ("unknown(required)", vec![]),
            ("null(required)", vec!["type"]),
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
            let v = crate::markdown::schemas::validate::build_structural_validator(&schema).unwrap();
            assert!(v.is_valid(&json!({})), "{ty}: absent must validate");
            assert!(v.is_valid(&json!({ "opt": null })), "{ty}: null must validate");
            assert!(
                v.is_valid(&json!({ "opt": valid_value.clone() })),
                "{ty}: valid non-null must validate"
            );

            let required_schema = convert(&format!("req: '{ty}(required)'"));
            let v_req =
                crate::markdown::schemas::validate::build_structural_validator(&required_schema).unwrap();
            assert!(
                !v_req.is_valid(&json!({ "req": null })),
                "{ty}: required must reject null"
            );
        }

        // Enum is tested separately because its members live inside the
        // grammar's parentheses; `required` follows after a `;` separator.
        let schema = convert("opt: enum(red,green)");
        let v = crate::markdown::schemas::validate::build_structural_validator(&schema).unwrap();
        assert!(v.is_valid(&json!({})), "enum: absent must validate");
        assert!(v.is_valid(&json!({ "opt": null })), "enum: null must validate");
        assert!(v.is_valid(&json!({ "opt": "red" })), "enum: valid member must validate");

        let required_enum = convert("req: 'enum(red,green; required)'");
        let v_req = crate::markdown::schemas::validate::build_structural_validator(&required_enum).unwrap();
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
    fn optional_file_accepts_null_and_empty_as_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("x.md");
        std::fs::write(&path, b"x").unwrap();

        let schema = convert("spec: file");
        let v = crate::markdown::schemas::validate::build_validator_in_context(
            &schema,
            None,
            None,
            &biscuit_file::FileResolutionContext::new(dir.path()),
        )
        .unwrap();
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
            let v = crate::markdown::schemas::validate::build_structural_validator(&schema).unwrap();
            assert!(v.is_valid(&json!({})), "{key}: absent must validate");
            assert!(v.is_valid(&json!({ "opt": null })), "{key}: null must validate");
        }
    }

    #[test]
    fn optional_array_accepts_null() {
        let schema = convert("opt: string[]");
        let v = crate::markdown::schemas::validate::build_structural_validator(&schema).unwrap();
        assert!(v.is_valid(&json!({})));
        assert!(v.is_valid(&json!({ "opt": null })));
        assert!(v.is_valid(&json!({ "opt": [] })));
        assert!(v.is_valid(&json!({ "opt": ["a"] })));
    }

    #[test]
    fn optional_property_union_accepts_null() {
        let yaml = "opt:\n  - string\n  - number\n";
        let schema = convert(yaml);
        let v = crate::markdown::schemas::validate::build_structural_validator(&schema).unwrap();
        assert!(v.is_valid(&json!({})));
        assert!(v.is_valid(&json!({ "opt": null })));
        assert!(v.is_valid(&json!({ "opt": "x" })));
        assert!(v.is_valid(&json!({ "opt": 3 })));
    }

    #[test]
    fn optional_constraints_are_bypassed_by_null() {
        let schema = convert("opt: 'string(not-empty; min(5))'");
        let v = crate::markdown::schemas::validate::build_structural_validator(&schema).unwrap();
        assert!(v.is_valid(&json!({ "opt": null })), "null must bypass constraints");
        assert!(!v.is_valid(&json!({ "opt": "" })), "empty string still rejected");
        assert!(!v.is_valid(&json!({ "opt": "ab" })), "too-short string still rejected");
    }

    #[test]
    fn required_string_rejects_null_with_type_problem() {
        use crate::markdown::schemas::validate::{build_structural_validator, collect_problems};

        let schema = convert("req: 'string(required)'");
        let v = build_structural_validator(&schema).unwrap();
        assert!(!v.is_valid(&json!({ "req": null })));
        let problems = collect_problems(&v, &json!({ "req": null }), &PositionMap::new());
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].kind, ValidationProblemKind::Type);
    }

    #[test]
    fn unknown_accepts_null_and_omission_whatever_its_presence_marker() {
        // `required` has no effect on `unknown`/`any`: absence, null, and
        // every native value validate, and the key never enters `required`.
        for declaration in ["opt: any", "opt: unknown", "opt: 'any(required)'", "opt: 'unknown(required)'"] {
            let schema = convert(declaration);
            assert_eq!(schema["properties"]["opt"], json!({}), "{declaration}");
            assert!(schema.get("required").is_none(), "{declaration}");
            let v = crate::markdown::schemas::validate::build_structural_validator(&schema).unwrap();
            assert!(v.is_valid(&json!({})), "{declaration}: absent");
            assert!(v.is_valid(&json!({ "opt": null })), "{declaration}: null");
            assert!(v.is_valid(&json!({ "opt": { "nested": [1, null, "x"] } })), "{declaration}");
        }
    }

    #[test]
    fn null_admitting_properties_lower_bare_and_never_enter_required() {
        for (declaration, expected) in [
            ("p: 'null'", json!({ "type": "null" })),
            ("p: 'null(required)'", json!({ "type": "null" })),
            ("p: 'null(eager; required)'", json!({ "type": "null" })),
            ("p: 'unknown(generated; required)'", json!({ "x-darkmatter-generated": true })),
        ] {
            let schema = convert(declaration);
            assert_eq!(schema["properties"]["p"], expected, "{declaration}");
            assert!(schema.get("required").is_none(), "{declaration}");
        }

        // A union with a null arm is optional and gains no second null arm;
        // constraints on its other arms still apply to non-null values.
        let schema = convert("p:\n  - 'string(required; min(3))'\n  - 'null'");
        assert!(schema.get("required").is_none());
        assert_eq!(schema["properties"]["p"]["anyOf"].as_array().unwrap().len(), 2);
        let v = crate::markdown::schemas::validate::build_structural_validator(&schema).unwrap();
        assert!(v.is_valid(&json!({})));
        assert!(v.is_valid(&json!({ "p": null })));
        assert!(v.is_valid(&json!({ "p": "abc" })));
        assert!(!v.is_valid(&json!({ "p": "ab" })));

        // Item and child nullability leave the container required.
        for declaration in ["p: 'null[](required)'", "p: 'unknown[](required)'", "p: '{ c: null }(required)'"] {
            let schema = convert(declaration);
            assert_eq!(schema["required"], json!(["p"]), "{declaration}");
        }
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
        let v = crate::markdown::schemas::validate::build_structural_validator(&schema).unwrap();
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
        crate::markdown::schemas::validate::build_structural_validator(schema)
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
    fn literal_large_integer_const_is_exact_or_rejected() {
        // A whole number is admitted only when an `f64` holds it exactly; the
        // admitted value reaches `const` as a JSON integer, never rounded.
        let exact: [(&str, Value); 2] = [
            ("literal(9007199254740992)", Value::from(9_007_199_254_740_992_i64)),
            ("literal(9223372036854775808)", Value::from(9_223_372_036_854_775_808_u64)),
        ];
        for (input, expected) in exact {
            assert_eq!(atom_value(input)["const"], expected, "const for {input}");
        }
        for input in [
            "literal(9007199254740993)",
            "literal(9223372036854775807)",
            "literal(18446744073709551615)",
        ] {
            let error = parse_type_expr("test", input).expect_err(input).to_string();
            assert!(error.contains("cannot be represented exactly"), "{input}: {error}");
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
        let v = atom_value("literal(9007199254740992; default(9007199254740992))");
        assert_eq!(v["const"], Value::from(9_007_199_254_740_992_i64));
        assert_eq!(v["default"], Value::from(9_007_199_254_740_992_i64));
    }

    #[test]
    fn large_integer_default_unequal_to_literal_rejected() {
        let mut atom =
            parse_type_expr("test", "literal(9007199254740992; default(9007199254740990))")
                .expect("parse atom");
        atom.constraints.push(Constraint::Required);
        assert!(
            atom_to_schema("test", &atom).is_err(),
            "an unequal large-integer default must fail schema load"
        );
        let error = parse_type_expr("test", "literal(1; default(9007199254740993))")
            .expect_err("an inexact default is rejected")
            .to_string();
        assert!(error.contains("cannot be represented exactly"), "{error}");
    }
}

/// Repeated constraints on one atom: no keyword is overwritten
/// (`2026-10-13-named-type-presence-and-local-names`).
#[cfg(test)]
mod repeated_constraints {
    use super::*;
    use crate::markdown::schemas::simplified::grammar::parse_type_expr;
    use crate::markdown::schemas::simplified::parse_yaml_schema;

    fn lower(input: &str) -> Result<Value, SchemaError> {
        let atom = parse_type_expr("p", input).expect("parse atom");
        atom_fragment_without_null_wrap("p", &atom, false).map(|(fragment, _)| fragment)
    }

    fn convert_message(input: &str) -> String {
        match lower(input) {
            Err(SchemaError::Convert { property, message }) => {
                assert_eq!(property, "p");
                message
            }
            other => panic!("`{input}` must be a conversion error, got {other:?}"),
        }
    }

    fn schema(yaml: &str) -> Result<Value, SchemaError> {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("yaml");
        to_json_schema(&parse_yaml_schema(&value).expect("schema parse"))
    }

    #[test]
    fn several_patterns_become_all_of_entries_on_the_base_fragment() {
        let v = lower("string(not-empty; pattern(^a); pattern(^a); max(9))").unwrap();
        assert_eq!(
            v,
            json!({
                "type": "string",
                "maxLength": 9,
                "allOf": [{ "pattern": r"\S" }, { "pattern": "^a" }]
            })
        );
    }

    #[test]
    fn a_repeated_identical_pattern_stays_a_single_keyword() {
        assert_eq!(
            lower("string(not-empty; not-empty)").unwrap(),
            json!({ "type": "string", "pattern": r"\S" })
        );
    }

    #[test]
    fn repeated_bounds_keep_the_stricter_value_per_kind() {
        let text = lower("string(min(3); min(5); max(80); max(50))").unwrap();
        assert_eq!((&text["minLength"], &text["maxLength"]), (&json!(5), &json!(50)));
        let number = lower("number(min(1.5); min(-2); max(10); max(10))").unwrap();
        assert_eq!((&number["minimum"], &number["maximum"]), (&json!(1.5), &json!(10)));
        let items = lower("string[](min(1); min(2); max(4); max(3))").unwrap();
        assert_eq!((&items["minItems"], &items["maxItems"]), (&json!(2), &json!(3)));
        let keys = lower("{ <string>: any }(min-keys(1); min-keys(2); max-keys(2))").unwrap();
        assert_eq!((&keys["minProperties"], &keys["maxProperties"]), (&json!(2), &json!(2)));
    }

    #[test]
    fn a_lower_bound_above_its_upper_bound_names_both_constraints() {
        for (input, low, high) in [
            ("string(min(10); max(5))", "`min(10)`", "`max(5)`"),
            ("number(max(1); min(1.5))", "`min(1.5)`", "`max(1)`"),
            ("string[](min(4); max(3))", "`min(4)`", "`max(3)`"),
            ("{ <string>: any }(max-keys(1); min-keys(2))", "`min-keys(2)`", "`max-keys(1)`"),
        ] {
            let message = convert_message(input);
            assert!(
                message.contains("contradictory bounds") && message.contains(low) && message.contains(high),
                "{input}: {message}"
            );
        }
    }

    /// A block-form object whose atom also carries postfix key bounds, the
    /// shape a reused named object type takes at a constrained use site.
    fn block_object_with_postfix(block: &str, postfix: Vec<Constraint>) -> Result<Value, SchemaError> {
        let value: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(&format!("p:\n  \"<string>\": any\n  $constraints:\n{block}")).unwrap();
        let SimplifiedSchema::Single(mut shape) = parse_yaml_schema(&value).unwrap() else {
            panic!("single schema");
        };
        let Some(PropertyDef::Single(atom)) = shape.properties.get_mut("p") else {
            panic!("single property");
        };
        atom.constraints.extend(postfix);
        to_json_schema(&SimplifiedSchema::Single(shape))
    }

    #[test]
    fn block_and_postfix_key_bounds_are_one_scope() {
        let v = block_object_with_postfix(
            "    min-keys: 1\n    max-keys: 4\n",
            vec![Constraint::MinKeys(2), Constraint::MaxKeys(5)],
        )
        .unwrap();
        let object = &v["properties"]["p"]["anyOf"][1];
        assert_eq!((&object["minProperties"], &object["maxProperties"]), (&json!(2), &json!(4)));
        let err = block_object_with_postfix("    min-keys: 3\n", vec![Constraint::MaxKeys(2)]).unwrap_err();
        assert!(err.to_string().contains("`min-keys(3)` is greater than `max-keys(2)`"), "{err}");
    }

    #[test]
    fn schemes_intersect_and_an_empty_intersection_is_an_error() {
        let v = lower("url(scheme(http, https); scheme(https, ftp))").unwrap();
        assert_eq!(v["x-darkmatter-url-scheme"], json!(["https"]));
        let message = convert_message("url(scheme(http); scheme(https))");
        assert!(message.contains("`scheme(http)` and `scheme(https)` share no scheme"), "{message}");
    }

    #[test]
    fn repeated_defaults_must_agree_after_number_normalization() {
        assert_eq!(lower("number(default(3); default(3.0))").unwrap()["default"], json!(3));
        let message = convert_message("string(default('3'); default(3))");
        assert!(message.contains("conflicting `default(...)` values"), "{message}");
    }

    #[test]
    fn examples_are_kept_in_authored_order() {
        let v = lower("string(example(./a.md); example(./b.md, ./c.md))").unwrap();
        assert_eq!(v["x-darkmatter-example"], json!(["./a.md", "./b.md", "./c.md"]));
    }

    #[test]
    fn block_presence_flags_mean_the_postfix_on_an_object_property() {
        let block = schema("p:\n  a: string\n  $constraints:\n    required: true\n").unwrap();
        let postfix = schema("p: \"{ a: string }(required)\"\n").unwrap();
        assert_eq!(block, postfix);
        let block = schema("p:\n  a: string\n  $constraints:\n    required: true\n    generated: true\n").unwrap();
        let postfix = schema("p: \"{ a: string }(required; generated)\"\n").unwrap();
        assert_eq!(block, postfix);
    }

    #[test]
    fn block_keys_without_object_meaning_are_rejected() {
        for key in ["not-empty", "unique", "integer"] {
            let err = schema(&format!("p:\n  a: string\n  $constraints:\n    {key}: true\n")).unwrap_err();
            assert!(
                err.to_string().contains(&format!("`$constraints` key `{key}` is not valid on an inline object")),
                "{key}: {err}"
            );
        }
        let err = schema("p:\n  a: string\n  $constraints:\n    min-items: 1\n").unwrap_err();
        assert!(err.to_string().contains("key `min-items` is not valid on an inline object"), "{err}");
        for key in ["required", "generated"] {
            let err = schema(&format!("a: string\n$constraints:\n  {key}: true\n")).unwrap_err();
            assert!(
                err.to_string().contains(&format!("`$constraints` key `{key}` is not valid on the root object")),
                "{key}: {err}"
            );
        }
    }

    #[test]
    fn several_match_groups_emit_nested_arrays_and_one_group_a_flat_array() {
        let v = schema(
            "- {doc: \"file(match('*.md'); match('guide*', '!guide-old*'))\", kind: 'literal(a)'}\n\
             - {doc: \"file(match('*.txt'))\", kind: 'literal(b)'}\n",
        )
        .unwrap();
        let keyword = |arm: usize| v["anyOf"][arm]["properties"]["doc"].to_string();
        assert!(
            keyword(0).contains(r#""x-darkmatter-match":[["*.md"],["guide*","!guide-old*"]]"#),
            "{}",
            keyword(0)
        );
        assert!(keyword(1).contains(r#""x-darkmatter-match":["*.txt"]"#), "{}", keyword(1));
    }
}
