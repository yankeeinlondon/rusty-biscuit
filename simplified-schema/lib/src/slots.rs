//! `{{ … }}` slots in a parsed schema: finding them without evaluating them.
//!
//! A schema whose `enum(...)`/`suggest(...)` lists or value positions hold a
//! slot is a **template**. Everything here is passive: it reads the typed
//! declaration and never evaluates an expression or reads a file. Lowering a
//! template fails with [`SchemaError::UnresolvedSlots`], which lists the
//! slots [`collect_slots`] finds.
//!
//! See `simplified-schema/docs/topics/language/slots.md`.

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
    match &atom.ty {
        TypeExpr::Ref(key) => out.push(key.clone()),
        TypeExpr::InlineObject(shape) => {
            shape.properties.values().chain(shape.pattern_keys.iter().map(|pattern| &pattern.def)).for_each(|def| def_refs(def, out));
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
