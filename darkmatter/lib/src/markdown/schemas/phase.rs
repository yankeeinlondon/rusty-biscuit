//! Passive runtime-phase projection for resolved SimplifiedSchema values.

use super::simplified::{Constraint, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema, TypeExpr};
use super::{SchemaError, resolve};
use serde_json::Value;

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

/// Replaces the document SimplifiedSchema portion of an effective schema while
/// preserving baseline and trigger-only properties already merged around it.
pub(super) fn merge_with_effective(
    effective: &Value,
    projected_document: Value,
) -> Result<Value, SchemaError> {
    match (
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
                .map(|(effective_arm, projected_arm)| {
                    resolve::merge_baseline(effective_arm, projected_arm.clone())
                })
                .collect::<Result<Vec<_>, _>>()?;
            root.insert("anyOf".into(), Value::Array(arms));
            Ok(Value::Object(root))
        }
        (None, None) => resolve::merge_baseline(effective, projected_document),
        // A mismatched root shape cannot be a layer-preserving projection.
        // Falling back to the document projection is safer than pairing arms
        // incorrectly; this only arises for mixed raw/Simplified root unions,
        // whose `EffectiveSchema::simplified` is already `None`.
        _ => Ok(projected_document),
    }
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

fn project_atom(atom: &mut PropertyAtom, phase: SchemaPhase, property_eager: bool) {
    if let TypeExpr::InlineObject(shape) = &mut atom.ty {
        *shape = project_shape(shape, phase);
    }

    let authored_required = atom
        .constraints
        .iter()
        .chain(atom.array_constraints.iter())
        .any(|constraint| matches!(constraint, Constraint::Required));
    atom.constraints
        .retain(|constraint| !matches!(constraint, Constraint::Required | Constraint::Generated));
    atom.array_constraints
        .retain(|constraint| !matches!(constraint, Constraint::Required | Constraint::Generated));

    let phase_required = match phase {
        SchemaPhase::Launch => property_eager,
        SchemaPhase::Completion => property_eager || authored_required,
    };
    if phase_required {
        if atom.is_array {
            atom.array_constraints.push(Constraint::Required);
        } else {
            atom.constraints.push(Constraint::Required);
        }
    }
}

fn atom_has_property_eager(atom: &PropertyAtom) -> bool {
    let constraints = if atom.is_array {
        &atom.array_constraints
    } else {
        &atom.constraints
    };
    constraints
        .iter()
        .any(|constraint| matches!(constraint, Constraint::Eager))
}
