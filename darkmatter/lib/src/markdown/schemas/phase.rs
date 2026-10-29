//! Passive runtime-phase projection for resolved SimplifiedSchema values.

use super::simplified::{
    Constraint, DefinitionTable, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema, TypeExpr,
};
use super::{SchemaError, resolve};
use serde_json::{Map, Value};

/// Runtime seam at which a resolved schema instance is validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaPhase {
    /// The stabilized instance immediately before provider launch.
    Launch,
    /// The final instance after every content-producing actor has run.
    Completion,
}

pub(super) fn project(schema: &SimplifiedSchema, phase: SchemaPhase) -> SimplifiedSchema {
    match schema {
        SimplifiedSchema::Single(shape) => SimplifiedSchema::Single(project_shape(shape, phase)),
        SimplifiedSchema::Union(arms) => SimplifiedSchema::Union(
            arms.iter()
                .map(|arm| match arm {
                    SchemaArm::Inline(shape) => SchemaArm::Inline(project_shape(shape, phase)),
                    SchemaArm::FileRef(path) => SchemaArm::FileRef(path.clone()),
                })
                .collect(),
        ),
    }
}

/// Projects every definition body of `definitions` for `phase`, under the
/// same keys, so the projected document's references reach projected bodies.
///
/// A body carries no presence or timing of its own (each use site holds
/// them), so projecting it touches only the properties nested inside it,
/// each timed by its own markers. One projected body therefore serves an
/// eager use and a deferred use alike; the two differ only at their sites.
pub(super) fn project_definitions(definitions: &DefinitionTable, phase: SchemaPhase) -> DefinitionTable {
    definitions.map_bodies(|body| {
        let mut body = body.clone();
        project_def(&mut body, phase);
        body
    })
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
pub(super) fn merge_with_effective(
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
pub(super) fn make_passive(schema: &mut Value) {
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

fn project_shape(shape: &SchemaShape, phase: SchemaPhase) -> SchemaShape {
    let mut projected = shape.clone();
    for def in projected.properties.values_mut() {
        project_def(def, phase);
    }
    projected
}

fn project_def(def: &mut PropertyDef, phase: SchemaPhase) {
    match def {
        PropertyDef::Single(atom) => project_atom(atom, phase, atom_has_property_eager(atom)),
        PropertyDef::Union(atoms) => {
            let eager = atoms.iter().any(atom_has_property_eager);
            for atom in atoms {
                project_atom(atom, phase, eager);
            }
        }
    }
}

/// Rewrites one property atom's presence markers for `phase`.
///
/// The arms of a [`TypeExpr::Union`] (`U[]`, `U(required)`) hold presence
/// markers that hoist to the property, so they are read into the property's
/// authored presence, stripped from the arms, and re-applied once on the
/// container. Every other arm constraint, including an inherited
/// `file(eager)`, is left alone: phase projection changes presence, never an
/// arm's file-existence semantics. Nested object fields, whether in an
/// inline-object type or in a union arm, are projected as their own shape.
fn project_atom(atom: &mut PropertyAtom, phase: SchemaPhase, property_eager: bool) {
    let authored_required = atom.is_required();
    match &mut atom.ty {
        TypeExpr::InlineObject(shape) => *shape = project_shape(shape, phase),
        TypeExpr::Union(arms) => arms.iter_mut().for_each(|arm| strip_union_arm(arm, phase)),
        TypeExpr::Primitive(_)
        | TypeExpr::Imported { .. }
        | TypeExpr::Tuple(_)
        | TypeExpr::Function(_)
        | TypeExpr::Ref(_) => {}
    }
    strip_presence(atom);

    let phase_required = match phase {
        SchemaPhase::Launch => property_eager && authored_required,
        SchemaPhase::Completion => authored_required,
    };
    if phase_required {
        if atom.is_array {
            atom.array_constraints.push(Constraint::Required);
        } else {
            atom.constraints.push(Constraint::Required);
        }
    }
}

/// Removes a union arm's hoisted presence markers, recursing into the item
/// arms of an array-of-union arm, and projects an object arm's own fields.
fn strip_union_arm(arm: &mut PropertyAtom, phase: SchemaPhase) {
    match &mut arm.ty {
        TypeExpr::InlineObject(shape) => *shape = project_shape(shape, phase),
        TypeExpr::Union(arms) => arms.iter_mut().for_each(|arm| strip_union_arm(arm, phase)),
        TypeExpr::Primitive(_)
        | TypeExpr::Imported { .. }
        | TypeExpr::Tuple(_)
        | TypeExpr::Function(_)
        | TypeExpr::Ref(_) => {}
    }
    strip_presence(arm);
}

fn strip_presence(atom: &mut PropertyAtom) {
    atom.constraints
        .retain(|constraint| !matches!(constraint, Constraint::Required | Constraint::Generated));
    atom.array_constraints
        .retain(|constraint| !matches!(constraint, Constraint::Required | Constraint::Generated));
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
