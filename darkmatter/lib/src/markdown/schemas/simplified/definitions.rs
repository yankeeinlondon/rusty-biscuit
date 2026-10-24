//! The finite definition table behind recursive named types.
//!
//! A named type that takes part in a reference cycle is stored once here and
//! every use of it is a [`TypeExpr::Ref`]. Acyclic named types never enter the
//! table: they inline at each use site exactly as before, so a schema without
//! recursion has an empty table and lowers with no `$defs`.
//!
//! A [`Definition`] body is the reusable value shape. Property presence and
//! annotations (`required`, `generated`, `eager`, `default`, `suggest`,
//! `example`, `category`, and the description) belong to each use site, which
//! the resolver builds from the definition's markers and the use site's own
//! postfix. Bodies refer to each other by [`DefKey`], never by `Arc`, so a table
//! holds no ownership cycle.

use std::{collections::BTreeMap, path::PathBuf};

use indexmap::IndexMap;

use super::{
    source::{SchemaReferenceSite, SchemaSourceId},
    types::{PropertyDef, TypeExpr},
};

/// The identity of one named definition: the namespace that declares it and
/// its name there.
///
/// Callers compare and hash keys; they never parse one, and a key never
/// appears in generated JSON (see [`Definition::json_key`]).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefKey {
    /// The namespace whose named types declare the definition.
    pub namespace: DefNamespace,
    /// The definition's name in that namespace.
    pub name: String,
}

impl DefKey {
    /// The key of `name` in `namespace`.
    pub fn new(namespace: DefNamespace, name: impl Into<String>) -> Self {
        Self {
            namespace,
            name: name.into(),
        }
    }
}

/// A table of named types that `Name@…` references resolve in.
///
/// Every independently authored source of named types has its own identity:
/// the document's inline `$schema` (and each of its root-union arms), each
/// disk trigger's inline payload, each schema file (and each of its inline
/// root-union arms), and each embedded or shipped document. An effective
/// schema looks references up across the tables of several of these sources
/// at once, so two sources that each declare `item` must never produce equal
/// keys: a shared identity would return one source's body for the other's
/// reference.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DefNamespace {
    /// The inline `$schema` mapping of the document being prepared.
    InlineRoot,
    /// Inline arm `index` of a document's root union.
    RootUnionArm(usize),
    /// The inline `$schema` payload of a schema-trigger file, by the
    /// canonical path of that trigger file.
    TriggerPayload(PathBuf),
    /// Inline arm `index` of a trigger payload's root union, by the canonical
    /// path of the trigger file. Such a payload is rejected as not mergeable,
    /// but its arms are resolved before that check runs.
    TriggerPayloadUnionArm(PathBuf, usize),
    /// A schema file, by the canonical path the resolver loaded it from.
    File(PathBuf),
    /// Inline arm `index` of a schema file whose declaration is a root union.
    FileRootUnionArm(PathBuf, usize),
    /// An embedded document by virtual path, including an inline schema
    /// authored inside one (its importer).
    Embedded(String),
    /// A shipped partial reached through `@darkmatter:<stem>`, by virtual path.
    Shipped(String),
}

impl DefNamespace {
    /// The public identity of the source this namespace was authored in.
    pub fn source_id(&self) -> SchemaSourceId {
        match self {
            // A trigger payload is resolved from its YAML value, like a
            // document's inline `$schema`, so its origins carry the same
            // source and no span.
            Self::InlineRoot
            | Self::RootUnionArm(_)
            | Self::TriggerPayload(_)
            | Self::TriggerPayloadUnionArm(..) => SchemaSourceId::InlineDocument,
            Self::File(path) | Self::FileRootUnionArm(path, _) => SchemaSourceId::File(path.clone()),
            Self::Embedded(path) | Self::Shipped(path) => SchemaSourceId::Embedded(path.clone()),
        }
    }
}

/// One cyclic named type.
#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    /// The reusable value shape, expanded. Nested uses of cyclic definitions
    /// are [`TypeExpr::Ref`]; presence markers and annotations were moved to
    /// the use sites.
    pub body: PropertyDef,
    /// Where the definition was declared: its source and `[name]` path.
    pub origin: SchemaReferenceSite,
    /// The key the body is emitted under in the lowered schema's `$defs`.
    /// Opaque: allocated in traversal order, never derived from a path.
    pub json_key: String,
}

/// What a definition's value is at its top level, which decides the
/// constraints a use site may add beside a reference to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionOuter {
    /// An inline object (`{ … }`).
    Object,
    /// An array (`T[]`).
    Array,
    /// A positional tuple.
    Tuple,
    /// A union of alternatives.
    Union,
    /// Anything else.
    Other,
}

/// The finite, immutable set of cyclic definitions one resolution reached.
///
/// Entries keep allocation order, which is the order the resolver first
/// reached each definition, and that order is the `$defs` emission order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DefinitionTable {
    entries: IndexMap<DefKey, Definition>,
    by_json_key: BTreeMap<String, DefKey>,
}

impl DefinitionTable {
    /// Whether the table holds no definition (the acyclic case).
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The number of definitions.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The definition stored under `key`.
    pub fn get(&self, key: &DefKey) -> Option<&Definition> {
        self.entries.get(key)
    }

    /// The identity behind a generated `$defs` key.
    pub fn key_for_json(&self, json_key: &str) -> Option<&DefKey> {
        self.by_json_key.get(json_key)
    }

    /// Every definition, in allocation order.
    pub fn iter(&self) -> impl Iterator<Item = (&DefKey, &Definition)> {
        self.entries.iter()
    }

    /// What `key`'s value is at its top level, looking through a body that
    /// is itself a reference to another definition.
    ///
    /// Alias chains between definitions are acyclic (a cycle that never
    /// descends into a child value is rejected), but the walk stops after as
    /// many steps as the table has entries in any case.
    pub fn outer(&self, key: &DefKey) -> Option<DefinitionOuter> {
        let mut current = self.entries.get(key)?;
        for _ in 0..=self.entries.len() {
            let atom = match &current.body {
                PropertyDef::Union(_) => return Some(DefinitionOuter::Union),
                PropertyDef::Single(atom) => atom,
            };
            if atom.is_array {
                return Some(DefinitionOuter::Array);
            }
            match &atom.ty {
                TypeExpr::InlineObject(_) => return Some(DefinitionOuter::Object),
                TypeExpr::Tuple(_) => return Some(DefinitionOuter::Tuple),
                TypeExpr::Union(_) => return Some(DefinitionOuter::Union),
                TypeExpr::Ref(target) => current = self.entries.get(target)?,
                TypeExpr::Primitive(_) | TypeExpr::Imported { .. } | TypeExpr::Function(_) => {
                    return Some(DefinitionOuter::Other);
                }
            }
        }
        None
    }

    /// Adds `body` under `key`, allocating its `$defs` key, unless `key` is
    /// already present. Returns the definition's `$defs` key either way.
    ///
    /// The first definition named `item` gets `item`; a later, different key
    /// with the same name gets `item-2`, `item-3`, and so on.
    pub(crate) fn insert(&mut self, key: DefKey, body: PropertyDef, origin: SchemaReferenceSite) -> String {
        if let Some(existing) = self.entries.get(&key) {
            return existing.json_key.clone();
        }
        let json_key = self.free_json_key(&key.name);
        self.by_json_key.insert(json_key.clone(), key.clone());
        self.entries.insert(
            key,
            Definition {
                body,
                origin,
                json_key: json_key.clone(),
            },
        );
        json_key
    }

    /// This table with every body replaced by `project(body)`, under the
    /// same identities, `$defs` keys, and allocation order, so a schema
    /// lowered against either table emits the same references.
    pub(crate) fn map_bodies(&self, mut project: impl FnMut(&PropertyDef) -> PropertyDef) -> DefinitionTable {
        DefinitionTable {
            entries: self
                .entries
                .iter()
                .map(|(key, definition)| {
                    (key.clone(), Definition { body: project(&definition.body), ..definition.clone() })
                })
                .collect(),
            by_json_key: self.by_json_key.clone(),
        }
    }

    /// Adds every definition of `other` that this table lacks, in `other`'s
    /// allocation order, and returns the `$defs` keys that changed: `other`'s
    /// key mapped to the key it has here. A definition both tables hold keeps
    /// this table's key; a new one keeps its key when it is free and takes
    /// the next `name-2`, `name-3`, … otherwise.
    ///
    /// Bodies refer to each other by [`DefKey`], so nothing inside them
    /// changes; a schema lowered against `other` has its `#/$defs/…`
    /// references renamed with the returned map.
    pub(crate) fn absorb(&mut self, other: &DefinitionTable) -> BTreeMap<String, String> {
        let mut renamed = BTreeMap::new();
        for (key, definition) in &other.entries {
            let json_key = match self.entries.get(key) {
                Some(existing) => existing.json_key.clone(),
                None => {
                    let json_key = if self.by_json_key.contains_key(&definition.json_key) {
                        self.free_json_key(&key.name)
                    } else {
                        definition.json_key.clone()
                    };
                    self.by_json_key.insert(json_key.clone(), key.clone());
                    self.entries.insert(key.clone(), Definition { json_key: json_key.clone(), ..definition.clone() });
                    json_key
                }
            };
            if json_key != definition.json_key {
                renamed.insert(definition.json_key.clone(), json_key);
            }
        }
        renamed
    }

    fn free_json_key(&self, name: &str) -> String {
        if !self.by_json_key.contains_key(name) {
            return name.to_string();
        }
        (2..)
            .map(|suffix| format!("{name}-{suffix}"))
            .find(|candidate| !self.by_json_key.contains_key(candidate))
            .expect("an unbounded suffix range has a free key")
    }
}

/// The local JSON Pointer reference to the `$defs` entry `json_key`, with the
/// key escaped as an RFC 6901 token (`~` → `~0`, `/` → `~1`).
pub fn definition_pointer(json_key: &str) -> String {
    format!("#/$defs/{}", json_key.replace('~', "~0").replace('/', "~1"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::simplified::{PropertyAtom, SchemaSourcePath, SimplifiedType};

    fn origin(name: &str) -> SchemaReferenceSite {
        SchemaReferenceSite {
            source: SchemaSourceId::InlineDocument,
            path: SchemaSourcePath::root().property(name),
        }
    }

    fn body() -> PropertyDef {
        PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Object))
    }

    #[test]
    fn same_named_definitions_from_different_namespaces_get_distinct_keys() {
        let mut table = DefinitionTable::default();
        let inline = DefKey::new(DefNamespace::InlineRoot, "item");
        let file = DefKey::new(DefNamespace::File(PathBuf::from("/types.yaml")), "item");
        assert_eq!(table.insert(inline.clone(), body(), origin("item")), "item");
        assert_eq!(table.insert(file.clone(), body(), origin("item")), "item-2");
        assert_eq!(table.insert(inline.clone(), body(), origin("item")), "item", "a key is allocated once");
        assert_eq!(table.key_for_json("item-2"), Some(&file));
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn pointer_tokens_are_escaped() {
        assert_eq!(definition_pointer("a/b~c"), "#/$defs/a~1b~0c");
        assert_eq!(definition_pointer("item"), "#/$defs/item");
    }
}
