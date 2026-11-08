//! `{{ … }}` slots in a parsed schema: finding them, and resolving them
//! through a host.
//!
//! A schema whose `enum(...)`/`suggest(...)` lists or value positions hold a
//! slot is a **template**. Finding slots is passive: it reads the typed
//! declaration and never evaluates an expression or reads a file. Lowering a
//! template fails with [`SchemaError::UnresolvedSlots`], which lists the
//! slots [`collect_slots`] finds and, from a resolution entry point, carries
//! a [`PendingSchema`].
//!
//! Resolution is explicit: [`PendingSchema::resolve`] (or
//! [`crate::SimplifiedSchema::resolve_slots`]) asks a [`SlotHost`] for each
//! slot's value, then runs the same resolution again inside
//! [`with_values`], where every lowering site [`substitute`]s the values
//! right after import expansion. The result is an ordinary concrete schema;
//! the template is never changed.
//!
//! See `simplified-schema/docs/topics/language/slots.md`.

mod host;
pub(crate) mod pending;
mod substitute;

pub use host::{SlotAdmissionError, SlotFailure, SlotHost, SlotRequest, SlotResolutionError, SlotRunRequest};
pub use pending::{PendingSchema, Template};
pub(crate) use pending::{Attempt, Replay, resolve_with};
pub use substitute::{
    MAX_SLOT_PATTERN_BYTES, SlotKey, SlotValues, active_values, check_slot_pattern, substitute, with_values,
};
pub(crate) use substitute::{apply_active, apply_active_to_arm, omitting};

use std::path::Path;

use crate::errors::{SchemaError, UnresolvedSlots};
use crate::simplified::{
    Constraint, DefKey, DefinitionTable, FunctionParameters, PropertyAtom, PropertyDef, SchemaArm, SchemaRoot, SchemaShape,
    Slot, TypeExpr,
};

/// Every slot `root` and `definitions` hold, in declaration order (the
/// root first, then each stored definition in allocation order), with the
/// slot of a definition reached from several places listed once.
pub fn collect_slots(root: &SchemaRoot, definitions: &DefinitionTable) -> Vec<Slot> {
    let mut slots = Vec::new();
    root_slots(root, &mut slots);
    for (_, definition) in definitions.iter() {
        def_slots(&definition.body, &mut slots);
    }
    slots
}

/// Whether `root` or `definitions` hold any slot.
pub fn has_dynamic(root: &SchemaRoot, definitions: &DefinitionTable) -> bool {
    !collect_slots(root, definitions).is_empty()
}

/// The slots of one property definition, in authored order.
pub fn definition_slots(def: &PropertyDef) -> Vec<Slot> {
    let mut slots = Vec::new();
    def_slots(def, &mut slots);
    slots
}

/// The slots of `def` and of every stored definition it reaches through a
/// [`TypeExpr::Ref`] in `definitions`, each once, in the order they are met.
pub fn reachable_slots(def: &PropertyDef, definitions: &DefinitionTable) -> Vec<Slot> {
    let mut slots = definition_slots(def);
    let mut seen = Vec::new();
    let mut pending = Vec::new();
    def_refs(def, &mut pending);
    while let Some(key) = pending.pop() {
        if seen.contains(&key) {
            continue;
        }
        if let Some(definition) = definitions.get(&key) {
            for slot in definition_slots(&definition.body) {
                push_unique(&mut slots, &slot);
            }
            def_refs(&definition.body, &mut pending);
        }
        seen.push(key);
    }
    slots
}

/// Removes the top-level properties `omitted` names from a single-object
/// `root`, then every stored definition the rest of it no longer reaches.
/// A root union is left as it is. Returns whether anything was removed.
pub(crate) fn omit_properties(root: &mut SchemaRoot, definitions: &mut DefinitionTable, omitted: &[String]) -> bool {
    let SchemaRoot::Single(shape) = root else {
        return false;
    };
    let before = shape.properties.len();
    shape.properties.retain(|name, _| !omitted.contains(name));
    if shape.properties.len() == before {
        return false;
    }
    let mut pending = Vec::new();
    for def in shape.properties.values().chain(shape.pattern_keys.iter().map(|pattern| &pattern.def)) {
        def_refs(def, &mut pending);
    }
    constraint_refs(&shape.constraints, &mut pending);
    let mut reached = std::collections::HashSet::new();
    while let Some(key) = pending.pop() {
        if let Some(definition) = definitions.get(&key)
            && reached.insert(key)
        {
            def_refs(&definition.body, &mut pending);
        }
    }
    definitions.retain(|key| reached.contains(key));
    true
}

/// Collects the definition keys `def` names directly.
fn def_refs(def: &PropertyDef, out: &mut Vec<DefKey>) {
    let atoms: &[PropertyAtom] = match def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(atoms) => atoms,
    };
    for atom in atoms {
        atom_refs(atom, out);
    }
}

fn atom_refs(atom: &PropertyAtom, out: &mut Vec<DefKey>) {
    constraint_refs(&atom.constraints, out);
    constraint_refs(&atom.array_constraints, out);
    match &atom.ty {
        TypeExpr::Ref(key) => out.push(key.clone()),
        TypeExpr::InlineObject(shape) => {
            shape.properties.values().chain(shape.pattern_keys.iter().map(|pattern| &pattern.def)).for_each(|def| def_refs(def, out));
            constraint_refs(&shape.constraints, out);
        }
        TypeExpr::Union(atoms) => atoms.iter().for_each(|atom| atom_refs(atom, out)),
        TypeExpr::Tuple(tuple) => {
            tuple.elements.iter().for_each(|element| def_refs(&element.def, out));
            if let Some(spread) = &tuple.spread {
                atom_refs(spread, out);
            }
        }
        TypeExpr::Function(function) => {
            function.parameters.as_slice().iter().for_each(|parameter| def_refs(&parameter.def, out));
            def_refs(&function.returns, out);
        }
        TypeExpr::Primitive(_) | TypeExpr::Imported { .. } => {}
    }
}

/// The definition keys the declarations among `constraints` name.
fn constraint_refs(constraints: &[Constraint], out: &mut Vec<DefKey>) {
    for constraint in constraints {
        if let Constraint::Property { declaration, .. } = constraint {
            def_refs(declaration, out);
        }
    }
}

/// `Err(UnresolvedSlots)` naming every slot when the schema is a template;
/// the guard every lowering entry point runs first, so no lowering ever
/// meets a dynamic constraint or builds a partial schema.
pub(crate) fn reject_unresolved(root: &SchemaRoot, definitions: &DefinitionTable) -> Result<(), SchemaError> {
    let slots = collect_slots(root, definitions);
    if slots.is_empty() {
        return Ok(());
    }
    Err(SchemaError::UnresolvedSlots(Box::new(UnresolvedSlots::new(slots))))
}

fn push_unique(slots: &mut Vec<Slot>, slot: &Slot) {
    if !slots.contains(slot) {
        slots.push(slot.clone());
    }
}

fn root_slots(root: &SchemaRoot, slots: &mut Vec<Slot>) {
    match root {
        SchemaRoot::Single(shape) => shape_slots(shape, slots),
        SchemaRoot::Union(arms) => {
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    shape_slots(shape, slots);
                }
            }
        }
    }
}

fn shape_slots(shape: &SchemaShape, slots: &mut Vec<Slot>) {
    for def in shape.properties.values() {
        def_slots(def, slots);
    }
    for pattern in &shape.pattern_keys {
        def_slots(&pattern.def, slots);
    }
    constraint_slots(&shape.constraints, slots);
}

fn def_slots(def: &PropertyDef, slots: &mut Vec<Slot>) {
    match def {
        PropertyDef::Single(atom) => atom_slots(atom, slots),
        PropertyDef::Union(atoms) => atoms.iter().for_each(|atom| atom_slots(atom, slots)),
    }
}

fn atom_slots(atom: &PropertyAtom, slots: &mut Vec<Slot>) {
    constraint_slots(&atom.constraints, slots);
    match &atom.ty {
        TypeExpr::Primitive(_) | TypeExpr::Imported { .. } | TypeExpr::Ref(_) => {}
        TypeExpr::InlineObject(shape) => shape_slots(shape, slots),
        TypeExpr::Union(atoms) => atoms.iter().for_each(|atom| atom_slots(atom, slots)),
        TypeExpr::Tuple(tuple) => {
            for element in &tuple.elements {
                def_slots(&element.def, slots);
            }
            if let Some(spread) = &tuple.spread {
                atom_slots(spread, slots);
            }
        }
        TypeExpr::Function(function) => {
            if let FunctionParameters::List(parameters) = &function.parameters {
                for parameter in parameters {
                    def_slots(&parameter.def, slots);
                }
            }
            def_slots(&function.returns, slots);
        }
    }
    constraint_slots(&atom.array_constraints, slots);
}

fn constraint_slots(constraints: &[Constraint], slots: &mut Vec<Slot>) {
    for constraint in constraints {
        if let Constraint::Property { declaration, .. } = constraint {
            def_slots(declaration, slots);
        }
        for slot in constraint.slots() {
            push_unique(slots, slot);
        }
    }
}

/// Sets `origin` on every slot of `atom` that has none: the slots of a
/// definition lifted out of the file `origin` evaluate in that file's
/// context wherever the definition is used.
pub(crate) fn stamp_atom_origin(atom: &mut PropertyAtom, origin: &Path) {
    visit_atom_slots_mut(atom, &mut |slot| {
        if slot.origin.is_none() {
            slot.origin = Some(origin.to_path_buf());
        }
    });
}

/// [`stamp_atom_origin`] for a whole declaration.
pub(crate) fn stamp_root_origin(root: &mut SchemaRoot, origin: &Path) {
    let mut stamp = |slot: &mut Slot| {
        if slot.origin.is_none() {
            slot.origin = Some(origin.to_path_buf());
        }
    };
    match root {
        SchemaRoot::Single(shape) => visit_shape_slots_mut(shape, &mut stamp),
        SchemaRoot::Union(arms) => {
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    visit_shape_slots_mut(shape, &mut stamp);
                }
            }
        }
    }
}

fn visit_shape_slots_mut(shape: &mut SchemaShape, visit: &mut dyn FnMut(&mut Slot)) {
    for def in shape.properties.values_mut() {
        visit_def_slots_mut(def, visit);
    }
    for pattern in &mut shape.pattern_keys {
        visit_def_slots_mut(&mut pattern.def, visit);
    }
    visit_constraint_slots_mut(&mut shape.constraints, visit);
}

fn visit_def_slots_mut(def: &mut PropertyDef, visit: &mut dyn FnMut(&mut Slot)) {
    match def {
        PropertyDef::Single(atom) => visit_atom_slots_mut(atom, visit),
        PropertyDef::Union(atoms) => atoms.iter_mut().for_each(|atom| visit_atom_slots_mut(atom, visit)),
    }
}

fn visit_atom_slots_mut(atom: &mut PropertyAtom, visit: &mut dyn FnMut(&mut Slot)) {
    visit_constraint_slots_mut(&mut atom.constraints, visit);
    match &mut atom.ty {
        TypeExpr::Primitive(_) | TypeExpr::Imported { .. } | TypeExpr::Ref(_) => {}
        TypeExpr::InlineObject(shape) => visit_shape_slots_mut(shape, visit),
        TypeExpr::Union(atoms) => atoms.iter_mut().for_each(|atom| visit_atom_slots_mut(atom, visit)),
        TypeExpr::Tuple(tuple) => {
            for element in &mut tuple.elements {
                visit_def_slots_mut(&mut element.def, visit);
            }
            if let Some(spread) = &mut tuple.spread {
                visit_atom_slots_mut(spread, visit);
            }
        }
        TypeExpr::Function(function) => {
            if let FunctionParameters::List(parameters) = &mut function.parameters {
                for parameter in parameters {
                    visit_def_slots_mut(&mut parameter.def, visit);
                }
            }
            visit_def_slots_mut(&mut function.returns, visit);
        }
    }
    visit_constraint_slots_mut(&mut atom.array_constraints, visit);
}

fn visit_constraint_slots_mut(constraints: &mut [Constraint], visit: &mut dyn FnMut(&mut Slot)) {
    for constraint in constraints {
        match constraint {
            Constraint::Property { declaration, .. } => visit_def_slots_mut(declaration, visit),
            Constraint::Dynamic(list) => {
                for element in &mut list.elements {
                    if let crate::simplified::DynamicElement::Slot(slot) = element {
                        visit(slot);
                    }
                }
            }
            Constraint::DynamicValue { slot, .. } => visit(slot),
            _ => {}
        }
    }
}
