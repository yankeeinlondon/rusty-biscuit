//! Passive runtime-phase projection for resolved SimplifiedSchema values.

use super::simplified::{Constraint, DefinitionTable, PropertyAtom, PropertyDef, SchemaRoot};
use super::{SchemaError, resolve};
use serde_json::{Map, Value};

/// The revision of a phase projection: [`lower`], [`merge_with_effective`],
/// and [`make_passive`] together. A
/// [`LayeredIdentity`](crate::validate::LayeredIdentity) folds it in;
/// increment it with any change to what a projection produces.
pub const PROJECTION_REVISION: u32 = 3;

/// Runtime seam at which a resolved schema instance is validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaPhase {
    /// The stabilized instance immediately before provider launch.
    Launch,
    /// The final instance after every content-producing actor has run.
    Completion,
}

/// Lowers a resolved schema to Draft 2020-12 JSON Schema with `phase`'s
/// presence rules.
///
/// Only each object's `required` array changes. [`SchemaPhase::Launch`]
/// lists a required property only when it is also `eager`;
/// [`SchemaPhase::Completion`] lists every required property, `generated`
/// included, since every content-producing actor has run. Value fragments
/// are lowered from the authored markers exactly as
/// [`to_json_schema_with_definitions`](crate::simplified::to_json_schema_with_definitions)
/// lowers them, so a required number that launch leaves unlisted still
/// rejects a supplied `null`. The rule applies inside nested objects, union
/// arms, and every definition body under `$defs`: a body carries no
/// presence of its own, so one lowered body serves an eager use and a
/// deferred use alike.
///
/// ## Errors
///
/// Whatever the ordinary lowering reports for `schema`, including
/// [`SchemaError::UnresolvedSlots`] for an unresolved template.
pub fn lower(schema: &SchemaRoot, definitions: &DefinitionTable, phase: SchemaPhase) -> Result<Value, SchemaError> {
    crate::simplified::convert::lower_document(schema, definitions, Some(phase))
}

/// Replaces the document SimplifiedSchema portion of an effective schema while
/// preserving baseline and trigger-only properties already merged around it.
///
/// Definitions sit at the root of both schemas. The document was the
/// receiving side of every merge that built `effective`, so its definitions
/// kept their keys there, and the projection, lowered from the same table,
/// reaches the same keys: each projected body replaces its unprojected
/// counterpart, and the definitions only a baseline or trigger layer reaches
/// are kept under their merged keys.
pub fn merge_with_effective(
    effective: &Value,
    projected_document: Value,
) -> Result<Value, SchemaError> {
    let mut merged = match (
        effective.get("anyOf").and_then(Value::as_array),
        projected_document.get("anyOf").and_then(Value::as_array),
    ) {
        (Some(effective_arms), Some(projected_arms))
            if effective_arms.len() == projected_arms.len() =>
        {
            let mut root = projected_document.as_object().cloned().unwrap_or_default();
            let arms = effective_arms
                .iter()
                .zip(projected_arms)
                .map(|(effective_arm, projected_arm)| merge_layer(effective_arm, projected_arm.clone()))
                .collect::<Result<Vec<_>, _>>()?;
            root.insert("anyOf".into(), Value::Array(arms));
            Value::Object(root)
        }
        // A root union below an object document (a registered host's union
        // layer): every arm already holds the document's declarations, so
        // the projected document replaces them arm by arm. Its definitions
        // join the root, where the arms' references resolve.
        (Some(effective_arms), None) => {
            let mut projected_document = projected_document;
            let projected_defs = projected_document.as_object_mut().and_then(|document| document.shift_remove("$defs"));
            let mut root = effective.as_object().cloned().unwrap_or_default();
            let arms = effective_arms
                .iter()
                .map(|effective_arm| merge_layer(effective_arm, projected_document.clone()))
                .collect::<Result<Vec<_>, _>>()?;
            root.insert("anyOf".into(), Value::Array(arms));
            if let Some(Value::Object(projected_defs)) = projected_defs {
                let Value::Object(defs) = root.entry("$defs").or_insert_with(|| Value::Object(Map::new())) else {
                    unreachable!("an effective schema's `$defs` is an object");
                };
                for (key, body) in projected_defs {
                    defs.insert(key, body);
                }
            }
            Value::Object(root)
        }
        (None, None) => merge_layer(effective, projected_document)?,
        // A mismatched root shape cannot be a layer-preserving projection.
        // Falling back to the document projection is safer than pairing arms
        // incorrectly; this only arises for mixed raw/Simplified root unions,
        // whose `EffectiveSchema::simplified` is already `None`.
        _ => return Ok(projected_document),
    };
    if let (Some(Value::Object(layer_defs)), Some(root)) = (effective.get("$defs"), merged.as_object_mut()) {
        let Value::Object(defs) = root.entry("$defs").or_insert_with(|| Value::Object(Map::new())) else {
            unreachable!("a lowered document's `$defs` is an object");
        };
        for (key, body) in layer_defs {
            defs.entry(key.clone()).or_insert_with(|| body.clone());
        }
    }
    Ok(merged)
}

/// An effective schema (or one arm of it) as the lower layer of the phase
/// merge. Its definitions are already at the merged root, so only its root
/// shape is checked before the property merge.
fn merge_layer(effective: &Value, projected: Value) -> Result<Value, SchemaError> {
    let object = effective.as_object().ok_or_else(|| SchemaError::Baseline {
        message: "the effective schema must be an object schema".into(),
        source: None,
    })?;
    resolve::validate_simple_object_root(object)?;
    resolve::merge_layer(effective, projected)
}

/// Removes the one validation format that probes the filesystem. Phase
/// validation checks the already-stabilized working instance; eager file
/// existence remains the responsibility of ordinary schema preparation.
pub fn make_passive(schema: &mut Value) {
    match schema {
        Value::Object(map) => {
            if map.get("format").and_then(Value::as_str)
                == Some(super::format::DARKMATTER_FILE_FORMAT)
            {
                map.insert(
                    "format".into(),
                    Value::String(super::format::DARKMATTER_FILE_REFERENCE_FORMAT.into()),
                );
            }
            for value in map.values_mut() {
                make_passive(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                make_passive(value);
            }
        }
        _ => {}
    }
}

/// Whether a property's definition sets `eager` validation timing on the
/// property itself (see [`atom_has_property_eager`]); a property-level union
/// is eager when any arm is.
pub(crate) fn property_eager(def: &PropertyDef) -> bool {
    match def {
        PropertyDef::Single(atom) => atom_has_property_eager(atom),
        PropertyDef::Union(atoms) => atoms.iter().any(atom_has_property_eager),
    }
}

/// Whether `atom` sets the property's validation timing.
///
/// `eager` is property timing in the property's own constraint list: the
/// postfix `[](…)` list of an array, the value list otherwise, so the
/// item-level `eager` of `file(eager)[]` times the items, not the property.
/// A union type's arms count whether or not it is a list (`U(eager)`, `U[]`),
/// the way a property-level union's arms do and the way
/// [`PropertyAtom::is_required`] counts them: an arm's markers belong to the
/// property that holds the union.
fn atom_has_property_eager(atom: &PropertyAtom) -> bool {
    let constraints = if atom.is_array {
        &atom.array_constraints
    } else {
        &atom.constraints
    };
    constraints
        .iter()
        .any(|constraint| matches!(constraint, Constraint::Eager))
        || atom
            .union_arms()
            .is_some_and(|arms| arms.iter().any(atom_has_property_eager))
}
