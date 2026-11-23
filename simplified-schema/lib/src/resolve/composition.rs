//! Object composition: the members a composed object receives.
//!
//! An object that writes `...name` spreads or a `$extends` directive is
//! **assembled** before anything inside it is walked or expanded. Assembly
//! applies the object's entries in order (each spread where it was written;
//! every `$extends` operand before the mapping's own members) and keeps, per
//! key, the last declaration. Each kept member remembers the namespace and
//! source path that declared it, so expansion, graph discovery, and the
//! default walk treat it exactly as they treat a member written there.
//!
//! ```mermaid
//! flowchart LR
//!     receiver["composed object"] --> entries["entries in order"]
//!     entries --> operand["operand: follow aliases to a declared object"]
//!     operand --> members["its assembled members (cached)"]
//!     members --> winners["last declaration per literal key and emitted pattern key"]
//!     entries --> winners
//!     winners --> expand["expand each winner in its declaring namespace"]
//! ```
//!
//! A composition hop describes the object at the receiving position, so it
//! never descends into a child value: a loop made only of composition and
//! alias hops is a [`SchemaError::NonDescendingCycle`]. Assembly reads only
//! member declarations, never their child references, so a member that
//! refers back to the receiver (`base: "{ children: branch[] }"`,
//! `branch: "{ ...base }"`) is an ordinary recursive child edge.

use std::sync::Arc;

use indexmap::IndexMap;

use super::{ImportEngine, MAX_IMPORT_DEPTH, Namespace, pattern_key_source_name};
use crate::{
    SchemaShape,
    errors::{CompositionTargetKind, CycleReference, SchemaError},
    simplified::{
        DefKey, SchemaReferenceSite, SchemaSourcePath,
        convert::emitted_pattern_key,
        types::{
            ComposedSiteTable, ComposedSites, CompositionOperand, Extends, ObjectComposition, PatternKey, PropertyDef,
            SimplifiedType, TypeExpr,
        },
    },
};

/// One member of an assembled object: its declaration and where it was
/// declared.
#[derive(Clone)]
pub(super) struct Member {
    pub(super) def: PropertyDef,
    /// The namespace whose names the declaration's references resolve in.
    pub(super) namespace: Namespace,
    /// The declaration's path in that namespace's source.
    pub(super) path: SchemaSourcePath,
    /// Whether the member came from another object, rather than being
    /// written in the receiver.
    pub(super) copied: bool,
}

/// The winning members of a composed object, in output order.
#[derive(Clone, Default)]
pub(super) struct Assembled {
    pub(super) literals: IndexMap<String, Member>,
    pub(super) patterns: Vec<(PatternKey, Member)>,
}

impl Assembled {
    /// Where each copied member was declared, for lowering diagnostics.
    pub(super) fn sites(&self) -> ComposedSites {
        let site = |member: &Member| SchemaReferenceSite {
            source: member.namespace.key.source_id(),
            path: member.path.clone(),
        };
        ComposedSites::new(ComposedSiteTable {
            literals: self
                .literals
                .iter()
                .filter(|(_, member)| member.copied)
                .map(|(name, member)| (name.clone(), site(member)))
                .collect(),
            patterns: self.patterns.iter().map(|(_, member)| member.copied.then(|| site(member))).collect(),
        })
    }
}

/// The written members of `shape`, declared in `namespace` at `path`.
fn own_members(shape: &SchemaShape, namespace: &Namespace, path: &SchemaSourcePath) -> Assembled {
    Assembled {
        literals: shape
            .properties
            .iter()
            .map(|(name, def)| (name.clone(), own(def, namespace, path.property(name.as_str()))))
            .collect(),
        patterns: shape
            .pattern_keys
            .iter()
            .map(|pattern| {
                let path = path.property(pattern_key_source_name(&pattern.key));
                (pattern.key.clone(), own(&pattern.def, namespace, path))
            })
            .collect(),
    }
}

fn own(def: &PropertyDef, namespace: &Namespace, path: SchemaSourcePath) -> Member {
    Member { def: def.clone(), namespace: namespace.clone(), path, copied: false }
}

/// The operand as written, for messages.
fn written(operand: &CompositionOperand) -> String {
    match &operand.reference {
        Some(reference) => format!("{}@{reference}", operand.name),
        None => operand.name.clone(),
    }
}

/// Entries of a composed object in the order they apply.
enum Entry<'s> {
    Literal(&'s String, &'s PropertyDef),
    Pattern(&'s crate::simplified::types::PatternKeyDef),
    Operand(&'s CompositionOperand, SchemaSourcePath),
}

fn entries<'s>(shape: &'s SchemaShape, composition: &'s ObjectComposition, path: &SchemaSourcePath) -> Vec<Entry<'s>> {
    let mut literals = shape.properties.iter().map(|(name, def)| Entry::Literal(name, def)).peekable();
    let mut patterns = shape.pattern_keys.iter().map(Entry::Pattern).peekable();
    let mut out = Vec::with_capacity(shape.properties.len() + shape.pattern_keys.len() + 1);
    match composition {
        ObjectComposition::Inline(spreads) => {
            let (mut literal, mut pattern) = (0, 0);
            for (index, spread) in spreads.iter().enumerate() {
                while literal < spread.literals_before
                    && let Some(entry) = literals.next()
                {
                    out.push(entry);
                    literal += 1;
                }
                while pattern < spread.patterns_before
                    && let Some(entry) = patterns.next()
                {
                    out.push(entry);
                    pattern += 1;
                }
                out.push(Entry::Operand(&spread.operand, path.spread(index)));
            }
        }
        ObjectComposition::Extends(Extends::Scalar(operand)) => {
            out.push(Entry::Operand(operand, path.property("$extends")));
        }
        ObjectComposition::Extends(Extends::Sequence(operands)) => {
            let directive = path.property("$extends");
            out.extend(operands.iter().enumerate().map(|(index, operand)| Entry::Operand(operand, directive.union_arm(index))));
        }
    }
    out.extend(literals);
    out.extend(patterns);
    out
}

impl ImportEngine<'_> {
    /// The winning members of `shape`, declared in `namespace` at `path`,
    /// or `None` when it composes nothing.
    ///
    /// Every operand is resolved, even one whose whole contribution a later
    /// entry replaces, so a missing or ineligible target always fails.
    pub(super) fn assemble(
        &mut self,
        shape: &SchemaShape,
        namespace: &Namespace,
        path: &SchemaSourcePath,
    ) -> Result<Option<Assembled>, SchemaError> {
        let Some(composition) = shape.composition.as_deref() else {
            return Ok(None);
        };
        let mut literals: IndexMap<String, Member> = IndexMap::new();
        let mut patterns: Vec<(PatternKey, Member)> = Vec::new();
        for entry in entries(shape, composition, path) {
            match entry {
                // `insert` keeps a replaced key at its first position.
                Entry::Literal(name, def) => {
                    literals.insert(name.clone(), own(def, namespace, path.property(name.as_str())));
                }
                Entry::Pattern(pattern) => {
                    let path = path.property(pattern_key_source_name(&pattern.key));
                    patterns.push((pattern.key.clone(), own(&pattern.def, namespace, path)));
                }
                Entry::Operand(operand, site_path) => {
                    let target = self.composition_target(operand, namespace, &site_path)?;
                    for (name, member) in &target.literals {
                        literals.insert(name.clone(), Member { copied: true, ..member.clone() });
                    }
                    patterns.extend(target.patterns.iter().map(|(key, member)| (key.clone(), Member { copied: true, ..member.clone() })));
                }
            }
        }
        // A pattern is identified by the key it lowers to against the final
        // literal keys, as a handwritten object's would be; the catch-all is
        // one identity of its own. The last entry wins at the first position.
        let names: Vec<&str> = literals.keys().map(String::as_str).collect();
        let mut winners: IndexMap<Option<String>, (PatternKey, Member)> = IndexMap::new();
        for (key, member) in patterns {
            winners.insert(emitted_pattern_key(&key, &names), (key, member));
        }
        Ok(Some(Assembled { literals, patterns: winners.into_values().collect() }))
    }

    /// The assembled members of the object `operand` (written at `site_path`
    /// in `namespace`) names, following aliases to it.
    fn composition_target(
        &mut self,
        operand: &CompositionOperand,
        namespace: &Namespace,
        site_path: &SchemaSourcePath,
    ) -> Result<Arc<Assembled>, SchemaError> {
        let active = self.composition_stack.len();
        let result = self.follow_target(operand, namespace, site_path);
        self.composition_stack.truncate(active);
        result
    }

    fn follow_target(
        &mut self,
        operand: &CompositionOperand,
        namespace: &Namespace,
        site_path: &SchemaSourcePath,
    ) -> Result<Arc<Assembled>, SchemaError> {
        let operand_site = SchemaReferenceSite { source: namespace.key.source_id(), path: site_path.clone() };
        let lookup_failed = |source: SchemaError| SchemaError::CompositionReference {
            operand: written(operand),
            site: operand_site.clone(),
            source: Box::new(source),
        };
        // Each hop: the name and namespace reference to look up, the
        // namespace it is written in, and where.
        let mut name = operand.name.clone();
        let mut reference = operand.resolved_reference().to_string();
        let mut from = namespace.clone();
        let mut site = operand_site.clone();
        loop {
            // Composition and alias hops are charged against the expansion
            // that is assembling them.
            let depth = self.stack.len() + self.composition_stack.len();
            if depth >= MAX_IMPORT_DEPTH {
                return Err(lookup_failed(SchemaError::ImportCycle {
                    chain: format!(
                        "named-type import expansion exceeded {MAX_IMPORT_DEPTH} levels near `{name}@{reference}`"
                    ),
                }));
            }
            let target = self.resolve_namespace(&reference, &from, &site.path).map_err(lookup_failed)?;
            let Some(def) = target.named_types.get(&name).cloned() else {
                return Err(SchemaError::UnknownNamedType { name, namespace: target.key.source_id(), site });
            };
            let key = DefKey::new(target.key.clone(), name.clone());
            let edge = CycleReference {
                site: site.clone(),
                target: name.clone(),
                target_source: target.key.source_id(),
                span: None,
            };
            if let Some(start) = self.composition_stack.iter().position(|(active, _)| active == &key) {
                let mut cycle: Vec<CycleReference> =
                    self.composition_stack[start + 1..].iter().map(|(_, edge)| edge.clone()).collect();
                cycle.push(edge);
                let mut cycles = vec![cycle];
                super::graph::locate_reference_spans(&mut cycles);
                return Err(SchemaError::NonDescendingCycle { cycles });
            }
            if let Some(cached) = self.assembled_cache.get(&(key.clone(), depth)) {
                return Ok(Arc::clone(cached));
            }
            self.composition_stack.push((key.clone(), edge));
            let definition_path = target.named_types.root.property(name.as_str());
            let invalid = |reason: CompositionTargetKind| SchemaError::InvalidCompositionTarget {
                name: written(operand),
                site: operand_site.clone(),
                definition: Box::new(SchemaReferenceSite {
                    source: target.key.source_id(),
                    path: definition_path.clone(),
                }),
                reason,
            };
            let atom = match &def {
                PropertyDef::Union(_) => return Err(invalid(CompositionTargetKind::Union)),
                PropertyDef::Single(atom) => atom,
            };
            if atom.is_array {
                return Err(invalid(CompositionTargetKind::Array));
            }
            // The alias's and the object's own qualifiers stay with them;
            // only the members are copied.
            let shape = match &atom.ty {
                TypeExpr::InlineObject(shape) => shape,
                TypeExpr::Imported { name: next, reference: next_reference } => {
                    name = next.clone();
                    reference = next_reference.clone();
                    site = SchemaReferenceSite { source: target.key.source_id(), path: definition_path };
                    from = target;
                    continue;
                }
                TypeExpr::Union(_) => return Err(invalid(CompositionTargetKind::Union)),
                TypeExpr::Tuple(_) => return Err(invalid(CompositionTargetKind::Array)),
                TypeExpr::Function(_) => return Err(invalid(CompositionTargetKind::Callable)),
                TypeExpr::Slot(_) => return Err(invalid(CompositionTargetKind::Template)),
                TypeExpr::Primitive(SimplifiedType::Object) | TypeExpr::Ref(_) => {
                    return Err(invalid(CompositionTargetKind::OpaqueObject));
                }
                TypeExpr::Primitive(_) => return Err(invalid(CompositionTargetKind::Primitive)),
            };
            let assembled = match self.assemble(shape, &target, &definition_path)? {
                Some(assembled) => assembled,
                None => own_members(shape, &target, &definition_path),
            };
            let assembled = Arc::new(assembled);
            self.assembled_cache.insert((key, depth), Arc::clone(&assembled));
            return Ok(assembled);
        }
    }

    /// `shape`'s members for walking it: its assembled winners when it
    /// composes, else what it declares. An assembly failure is reported by
    /// expansion, so a walk that cannot fail falls back to the written
    /// members.
    pub(super) fn walk_members(&mut self, shape: &SchemaShape, namespace: &Namespace, path: &SchemaSourcePath) -> Assembled {
        match self.assemble(shape, namespace, path) {
            Ok(Some(assembled)) => assembled,
            Ok(None) | Err(_) => own_members(shape, namespace, path),
        }
    }
}
