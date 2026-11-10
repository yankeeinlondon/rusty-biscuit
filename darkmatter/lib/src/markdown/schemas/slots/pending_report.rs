//! Where an unresolved slot sits in its template, for the report a surface
//! that cannot resolve it gives: the property whose declaration holds it,
//! and whether the slot supplies that property's whole type (a template
//! type, whose type is pending) or one constraint of an authored type (a
//! template constraint).
//!
//! Found passively in the expanded template the resolution handle kept;
//! nothing is read or evaluated.

use std::collections::HashSet;

use simplified_schema::UnresolvedSlots;
use simplified_schema::simplified::{
    DefKey, DefinitionTable, PropertyAtom, PropertyDef, SchemaArm, SchemaRoot, SchemaShape, Slot, TypeExpr,
};

/// The kind of slot a property holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotRole {
    /// The slot is the property's type, or one of its union arms, tuple
    /// elements, or item types.
    TemplateType,
    /// The slot fills a constraint of an authored type.
    TemplateConstraint,
}

/// The property whose declaration holds `slot`, by its dotted path from the
/// schema's root (`status`, `owner.name`), and the slot's role there. A slot
/// a named type holds is reported at the first property that uses the type.
/// `None` when the template was not kept or no property reaches the slot.
pub(crate) fn slot_property(unresolved: &UnresolvedSlots, slot: &Slot) -> Option<(String, SlotRole)> {
    let (root, definitions) = unresolved.pending.as_ref()?.template()?;
    let shapes: Vec<&SchemaShape> = match root {
        SchemaRoot::Single(shape) => vec![shape],
        SchemaRoot::Union(arms) => arms
            .iter()
            .filter_map(|arm| match arm {
                SchemaArm::Inline(shape) => Some(shape),
                _ => None,
            })
            .collect(),
    };
    let mut finder = Finder { slot, definitions, visited: HashSet::new() };
    shapes.into_iter().find_map(|shape| finder.shape(shape, ""))
}

/// The text a pending report adds for `slot`: "property `status` has a
/// template type, so its type is pending" or "property `os` has a template
/// constraint". `None` when [`slot_property`] finds no property.
pub(crate) fn pending_context(unresolved: &UnresolvedSlots, slot: &Slot) -> Option<String> {
    let (property, role) = slot_property(unresolved, slot)?;
    Some(match role {
        SlotRole::TemplateType => format!("property `{property}` has a template type, so its type is pending"),
        SlotRole::TemplateConstraint => format!("property `{property}` has a template constraint"),
    })
}

struct Finder<'a> {
    slot: &'a Slot,
    definitions: &'a DefinitionTable,
    visited: HashSet<DefKey>,
}

impl Finder<'_> {
    fn shape(&mut self, shape: &SchemaShape, prefix: &str) -> Option<(String, SlotRole)> {
        let path = |name: &str| if prefix.is_empty() { name.to_string() } else { format!("{prefix}.{name}") };
        for (name, def) in &shape.properties {
            if let Some(found) = self.property(def, &path(name)) {
                return Some(found);
            }
        }
        // A pattern key has no name a reader would recognize in a path.
        shape.pattern_keys.iter().find_map(|pattern| self.property(&pattern.def, &path("<key>")))
    }

    /// The role of the slot in `def`, declared at `path`; a nested inline
    /// object's property reports its own deeper path.
    fn property(&mut self, def: &PropertyDef, path: &str) -> Option<(String, SlotRole)> {
        if !simplified_schema::slots::reachable_slots(def, self.definitions).contains(self.slot) {
            return None;
        }
        let atoms: &[PropertyAtom] = match def {
            PropertyDef::Single(atom) => std::slice::from_ref(atom),
            PropertyDef::Union(atoms) => atoms,
        };
        for atom in atoms {
            if let Some(found) = self.atom(atom, path) {
                return Some(found);
            }
        }
        // Reachable but not as a type: a constraint of this property's own
        // type, or of a type it uses.
        Some((path.to_string(), SlotRole::TemplateConstraint))
    }

    /// `Some` only when the slot is a template type somewhere under `atom`
    /// or sits inside a nested object property, which names a deeper path.
    fn atom(&mut self, atom: &PropertyAtom, path: &str) -> Option<(String, SlotRole)> {
        match &atom.ty {
            TypeExpr::Slot(slot) if slot == self.slot => Some((path.to_string(), SlotRole::TemplateType)),
            TypeExpr::InlineObject(shape) => self.shape(shape, path),
            TypeExpr::Union(atoms) => atoms.iter().find_map(|atom| self.atom(atom, path)),
            TypeExpr::Tuple(tuple) => tuple
                .elements
                .iter()
                .flat_map(|element| match &element.def {
                    PropertyDef::Single(atom) => std::slice::from_ref(atom),
                    PropertyDef::Union(atoms) => atoms.as_slice(),
                })
                .find_map(|atom| self.atom(atom, path)),
            TypeExpr::Ref(key) => {
                if !self.visited.insert(key.clone()) {
                    return None;
                }
                let body = &self.definitions.get(key)?.body;
                let atoms: &[PropertyAtom] = match body {
                    PropertyDef::Single(atom) => std::slice::from_ref(atom),
                    PropertyDef::Union(atoms) => atoms,
                };
                atoms.iter().find_map(|atom| self.atom(atom, path))
            }
            _ => None,
        }
    }
}
