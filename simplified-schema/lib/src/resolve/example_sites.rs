//! Which declaration wrote each `example(...)` reference of one resolution.
//!
//! Examples are read twice: an imported declaration's are rehomed (resolved
//! to an absolute path) while expansion inlines it, and every example is
//! loaded and judged only after the expanded schema is lowered, from the
//! lowered JSON, which keeps the reference's spelling but neither the file
//! nor the declaration that wrote it. This table keeps them, recorded where
//! each reference is first read: a document's own declarations as written,
//! and an imported declaration's when it is rehomed, under the path it was
//! rehomed to. A spelling that two declarations wrote has no single writer
//! and is recorded as unknown, never as the first of them.

use std::collections::HashMap;

use crate::simplified::types::{Constraint, PropertyAtom, PropertyDef, SchemaShape, TypeExpr};
use crate::simplified::{SchemaReferenceSite, SchemaSourceId, SchemaSourcePath};

use super::pattern_key_source_name;

/// The declaration that wrote each example reference spelling, by spelling
/// (`None` when several did).
#[derive(Debug, Clone, Default)]
pub(crate) struct ExampleSites(HashMap<String, Option<SchemaReferenceSite>>);

impl ExampleSites {
    /// The declaration that wrote `reference`, when exactly one did.
    pub(crate) fn of(&self, reference: &str) -> Option<&SchemaReferenceSite> {
        self.0.get(reference).and_then(Option::as_ref)
    }

    /// `site` read `reference` as written: a second declaration writing the
    /// same spelling makes its writer unknown.
    pub(crate) fn read(&mut self, reference: &str, site: &SchemaReferenceSite) {
        match self.0.get_mut(reference) {
            None => {
                self.0.insert(reference.to_string(), Some(site.clone()));
            }
            Some(known) if known.as_ref() != Some(site) => *known = None,
            Some(_) => {}
        }
    }

    /// `site` holds `reference` already normalized, as a declaration that
    /// inlined another file's does: the writer recorded when that file's
    /// declaration was rehomed stays, and is recorded only when none was.
    pub(crate) fn carried(&mut self, reference: &str, site: &SchemaReferenceSite) {
        self.0.entry(reference.to_string()).or_insert_with(|| Some(site.clone()));
    }

    /// Adds `other`'s spellings, each as a read of its own.
    pub(crate) fn extend(&mut self, other: ExampleSites) {
        for (reference, site) in other.0 {
            match site {
                Some(site) => self.read(&reference, &site),
                None => {
                    self.0.insert(reference, None);
                }
            }
        }
    }

    /// Records every example reference `shape` writes, as written, at its
    /// path under `root` in `source`: its properties' and pattern keys'
    /// definitions, union arms, and nested inline objects and tuples.
    pub(crate) fn read_shape(&mut self, shape: &SchemaShape, source: &SchemaSourceId, root: &SchemaSourcePath) {
        for (name, def) in &shape.properties {
            self.read_def(def, source, &root.property(name.as_str()));
        }
        for pattern in &shape.pattern_keys {
            self.read_def(&pattern.def, source, &root.property(pattern_key_source_name(&pattern.key)));
        }
    }

    fn read_def(&mut self, def: &PropertyDef, source: &SchemaSourceId, path: &SchemaSourcePath) {
        match def {
            PropertyDef::Single(atom) => self.read_atom(atom, source, path),
            PropertyDef::Union(arms) => {
                for (index, arm) in arms.iter().enumerate() {
                    self.read_atom(arm, source, &path.union_arm(index));
                }
            }
        }
    }

    /// The paths below an atom follow the resolver's (see
    /// `ExampleRehome::rehome`): an arm or tuple element is an arm index.
    fn read_atom(&mut self, atom: &PropertyAtom, source: &SchemaSourceId, path: &SchemaSourcePath) {
        let site = SchemaReferenceSite { source: source.clone(), path: path.clone() };
        for constraint in atom.constraints.iter().chain(&atom.array_constraints) {
            if let Constraint::Example(references) = constraint {
                for reference in references {
                    self.read(reference, &site);
                }
            }
        }
        match &atom.ty {
            TypeExpr::InlineObject(shape) => self.read_shape(shape, source, path),
            TypeExpr::Union(arms) => {
                for (index, arm) in arms.iter().enumerate() {
                    self.read_atom(arm, source, &path.union_arm(index));
                }
            }
            TypeExpr::Tuple(tuple) => {
                for (index, element) in tuple.elements.iter().enumerate() {
                    self.read_def(&element.def, source, &path.union_arm(index));
                }
                if let Some(spread) = &tuple.spread {
                    self.read_atom(spread, source, &path.union_arm(tuple.elements.len()));
                }
            }
            TypeExpr::Primitive(_) | TypeExpr::Imported { .. } | TypeExpr::Ref(_) | TypeExpr::Function(_) | TypeExpr::Slot(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(path: &str) -> SchemaReferenceSite {
        SchemaReferenceSite { source: SchemaSourceId::InlineDocument, path: SchemaSourcePath::root().property(path) }
    }

    #[test]
    fn a_spelling_two_declarations_read_has_no_writer() {
        let mut sites = ExampleSites::default();
        sites.read("./a.yaml", &site("a"));
        sites.read("./a.yaml", &site("a"));
        assert_eq!(sites.of("./a.yaml"), Some(&site("a")), "the same declaration twice");
        sites.read("./a.yaml", &site("b"));
        assert_eq!(sites.of("./a.yaml"), None);
        sites.carried("./a.yaml", &site("c"));
        assert_eq!(sites.of("./a.yaml"), None, "a carried spelling never decides");
    }

    #[test]
    fn a_carried_spelling_keeps_the_writer_recorded_when_it_was_rehomed() {
        let mut sites = ExampleSites::default();
        sites.read("/lib/a.yaml", &site("inner"));
        sites.carried("/lib/a.yaml", &site("outer"));
        assert_eq!(sites.of("/lib/a.yaml"), Some(&site("inner")));
        sites.carried("/lib/b.yaml", &site("outer"));
        assert_eq!(sites.of("/lib/b.yaml"), Some(&site("outer")), "none was recorded");
    }
}
