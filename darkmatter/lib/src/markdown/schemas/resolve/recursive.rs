//! Expansion of stored named types into references.
//!
//! A definition is stored when it takes part in a reference cycle, or when
//! discovery selected it for sharing (an acyclic structured value referenced
//! from two or more authored sites). Every use of a cyclic definition, and
//! every use of a shared one a reference can represent, is a
//! [`TypeExpr::Ref`] whose atom holds what the use site's property needs:
//! the definition's presence markers and annotations merged with the use
//! site's own postfix. The definition's body is expanded once per pass and
//! stored without them: a cyclic body from a worklist, a shared body at its
//! first use, exactly where inlining would have expanded it.
//!
//! A use of a shared definition stays inline when a reference beside the
//! unchanged body could not reproduce what inlining gives it: a key
//! exclusion that changes how the body's own key bounds count, a constraint
//! a reference cannot carry, or, as a union alternative, anything the union
//! would otherwise flatten into its own alternatives (property markers, a
//! description over a union, a list). A body whose top level carries
//! use-context behavior (a nullable value, an optional `file` alternative,
//! a union alternative's presence marker or suggestion) is never shared.
//! Only definitions a surviving reference reaches are published.
//!
//! What a definition hands its use sites depends only on its top level,
//! never on what sits below a descending edge, but a top level can itself be
//! a reference, an array of one, or a union with referenced arms. So a
//! recursive schema expands twice. The first pass builds use sites from the
//! use site's postfix alone and yields every body; [`UseSummary`] is read off
//! those bodies, following top-level references through other summaries.
//! The second pass builds each use site from its definition's summary with
//! the same merge rules, and the same provenance, an inlined reference uses.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::{
    AtomProvenance, CountedBound, ExpandedDef, ImportEngine, MergeSite, Namespace, NamedReference,
    Tagged, authored_origins, is_array_container_constraint, is_union_marker, merge_constraints,
    reconcile_property_defaults, reinterpret_bounds,
};
use crate::markdown::schemas::{
    SchemaArm, SchemaShape, SimplifiedSchema, SimplifiedType,
    errors::SchemaError,
    simplified::{
        DefKey, DefinitionOuter, DefinitionTable, SchemaConstraintOrigin, SchemaReferenceSite,
        SchemaSourcePath, SchemaSourcePathSegment, SchemaUnionRejection,
        types::{Constraint, PropertyAtom, PropertyDef, TypeExpr},
    },
};

/// A stored definition reached by the current pass.
pub(super) struct ReachedDefinition {
    namespace: Namespace,
    /// The expanded body, with its top-level markers still in place; `None`
    /// until the worklist reaches it.
    body: Option<ExpandedDef>,
}

/// What a stored definition contributes to each of its use sites.
#[derive(Clone)]
pub(super) struct UseSummary {
    /// What the value is at its top level.
    outer: DefinitionOuter,
    /// For a union, the first alternative that is itself an array, which
    /// rules out `[]` at a use site.
    array_arm: Option<usize>,
    /// The presence markers and annotations a use site inherits, with the
    /// declaration that introduced each.
    markers: Vec<Tagged>,
    /// The description a use site inherits.
    description: Option<String>,
    /// The definition's own top-level count bounds. They stay in the body;
    /// a use site's bounds are checked against them so a contradiction is
    /// reported with both declarations.
    bounds: Vec<Tagged>,
    /// Whether the value admits null, which makes every use optional.
    admits_null: bool,
}

/// A constraint that describes a definition's use site rather than its value.
fn is_use_site_marker(constraint: &Constraint) -> bool {
    matches!(
        constraint,
        Constraint::Required
            | Constraint::Generated
            | Constraint::Eager
            | Constraint::Default(_)
            | Constraint::Suggest(_)
            | Constraint::Example(_)
            | Constraint::Category(_)
    )
}

/// What a union's alternative contributes to the union's property: the
/// markers a property-level union hoists from its arms. Everything else on
/// an arm (a file's `eager`, its description, examples) stays with the arm.
fn is_arm_property_marker(constraint: &Constraint) -> bool {
    matches!(
        constraint,
        Constraint::Required | Constraint::Generated | Constraint::Default(_)
    )
}

fn is_count_bound(constraint: &Constraint) -> bool {
    matches!(
        constraint,
        Constraint::MinItems(_)
            | Constraint::MaxItems(_)
            | Constraint::MinKeys(_)
            | Constraint::MaxKeys(_)
            | Constraint::UncountedKeys(_)
    )
}

fn tagged(constraints: &[Constraint], origins: &[SchemaConstraintOrigin]) -> Vec<Tagged> {
    constraints.iter().cloned().zip(origins.iter().cloned()).collect()
}

impl ImportEngine<'_> {
    /// Records that `key` was reached, queueing its body for expansion.
    pub(super) fn reach(&mut self, key: &DefKey, namespace: &Namespace) {
        if self.reached.contains_key(key) {
            return;
        }
        self.reached.insert(
            key.clone(),
            ReachedDefinition {
                namespace: namespace.clone(),
                body: None,
            },
        );
        self.pending.push_back(key.clone());
    }

    /// Expands every queued body, including the bodies those reach.
    pub(super) fn expand_reached_definitions(&mut self) -> Result<(), SchemaError> {
        while let Some(key) = self.pending.pop_front() {
            let namespace = self.reached[&key].namespace.clone();
            let def = namespace
                .named_types
                .get(&key.name)
                .cloned()
                .expect("a reached definition exists in its namespace");
            let path = namespace.named_types.root.property(key.name.as_str());
            let body = match def {
                PropertyDef::Single(atom) => self.expand_atom(atom, &namespace, &path)?,
                PropertyDef::Union(arms) => self.expand_union_arms(arms, &namespace, &path)?.into_expanded(),
            };
            let body = self.rehome_definition_examples(body, &namespace.key)?;
            self.reached[&key].body = Some(body);
        }
        Ok(())
    }

    /// Records that the shared definition `key`, whose body is `base`, is
    /// used through a reference in this pass.
    pub(super) fn reach_shared(&mut self, key: &DefKey, namespace: &Namespace, base: &ExpandedDef) {
        if self.reached.contains_key(key) {
            return;
        }
        self.reached.insert(
            key.clone(),
            ReachedDefinition {
                namespace: namespace.clone(),
                body: Some(base.clone()),
            },
        );
    }

    /// Starts the second pass of a schema with stored definitions with what
    /// the first one learned.
    pub(super) fn begin_final_pass(&mut self, summaries: HashMap<DefKey, UseSummary>) {
        self.summaries = Some(summaries);
        self.expanded_cache.clear();
        self.forced_cache.clear();
        self.reached.clear();
        self.pending.clear();
    }

    /// Adds every body a reference in `schema` reaches, directly or through
    /// another body, without its use-site markers, to `definitions` in
    /// first-reached order. A body reached in this pass only by an
    /// expansion whose use was then inlined is left out.
    pub(super) fn publish_definitions(&mut self, schema: &SimplifiedSchema, definitions: &mut DefinitionTable) {
        let mut used = HashSet::new();
        let mut queue = Vec::new();
        schema_refs(schema, &mut |key| {
            if used.insert(key.clone()) {
                queue.push(key.clone());
            }
        });
        while let Some(key) = queue.pop() {
            let Some(body) = self.reached.get(&key).and_then(|reached| reached.body.as_ref()) else {
                continue;
            };
            expanded_refs(body, &mut |key| {
                if used.insert(key.clone()) {
                    queue.push(key.clone());
                }
            });
        }
        for (key, reached) in std::mem::take(&mut self.reached) {
            if !used.contains(&key) {
                continue;
            }
            let body = reached.body.expect("the final pass expanded every reached body");
            let origin = SchemaReferenceSite {
                source: key.namespace.source_id(),
                path: reached.namespace.named_types.root.property(key.name.as_str()),
            };
            definitions.insert(key, strip_use_site_markers(body), origin);
        }
    }

    /// The use of the stored definition `key` written as `reference`.
    pub(super) fn recursive_use(
        &self,
        key: DefKey,
        reference: NamedReference,
        site: &SchemaReferenceSite,
        definition: &SchemaReferenceSite,
    ) -> Result<ExpandedDef, SchemaError> {
        let Some(summaries) = &self.summaries else {
            let base = PropertyAtom {
                ty: TypeExpr::Ref(key),
                is_array: false,
                constraints: Vec::new(),
                array_constraints: Vec::new(),
                description: None,
            };
            return Ok(first_pass_postfix(base, AtomProvenance::default(), reference, site));
        };
        let summary = &summaries[&key];
        let (constraints, value) = summary.markers.iter().cloned().unzip();
        let base = PropertyAtom {
            ty: TypeExpr::Ref(key),
            is_array: false,
            constraints,
            array_constraints: Vec::new(),
            description: summary.description.clone(),
        };
        let provenance = AtomProvenance {
            value,
            array: Vec::new(),
            shape: Vec::new(),
        };
        apply_reference_postfix(base, provenance, summary, reference, site, definition)
    }

    /// Applies a use-site postfix to an expanded definition. A definition
    /// that is itself a use of a stored definition (an inlined alias of one)
    /// takes the reference rules; anything else is inlined.
    pub(super) fn apply_postfix(
        &self,
        base: ExpandedDef,
        reference: NamedReference,
        site: &SchemaReferenceSite,
        definition: &SchemaReferenceSite,
    ) -> Result<ExpandedDef, SchemaError> {
        let no_postfix = !reference.is_array
            && reference.constraints.is_empty()
            && reference.array_constraints.is_empty()
            && reference.description.is_none();
        let ExpandedDef::Single(atom, provenance) = base else {
            return super::apply_import_postfix(base, reference, site, definition);
        };
        let TypeExpr::Ref(key) = &atom.ty else {
            return super::apply_import_postfix(ExpandedDef::Single(atom, provenance), reference, site, definition);
        };
        if no_postfix || atom.is_array {
            // An alias of a list of a cyclic definition is an ordinary array
            // atom; `[]` over it is refused by the inlined rules.
            return super::apply_import_postfix(ExpandedDef::Single(atom, provenance), reference, site, definition);
        }
        match &self.summaries {
            None => Ok(first_pass_postfix(*atom, provenance, reference, site)),
            Some(summaries) => {
                let summary = &summaries[key];
                apply_reference_postfix(*atom, provenance, summary, reference, site, definition)
            }
        }
    }

    /// Reads each reached definition's [`UseSummary`] off its first-pass body.
    pub(super) fn summarize(&self) -> Result<HashMap<DefKey, UseSummary>, SchemaError> {
        let mut summaries = HashMap::new();
        let mut visiting = HashSet::new();
        for key in self.reached.keys() {
            self.summary_of(key, &mut summaries, &mut visiting)?;
        }
        Ok(summaries)
    }

    fn summary_of(
        &self,
        key: &DefKey,
        summaries: &mut HashMap<DefKey, UseSummary>,
        visiting: &mut HashSet<DefKey>,
    ) -> Result<UseSummary, SchemaError> {
        if let Some(summary) = summaries.get(key) {
            return Ok(summary.clone());
        }
        let definition_path = self.reached[key].namespace.named_types.root.property(key.name.as_str());
        let site = SchemaReferenceSite {
            source: key.namespace.source_id(),
            path: definition_path.clone(),
        };
        if !visiting.insert(key.clone()) {
            // Only a top level that is a list of itself, which `[]` over an
            // array already refuses at the use site, comes back here.
            return Err(SchemaError::Convert {
                property: definition_path.to_string(),
                message: format!(
                    "the recursive named type `{}` is a list of itself at its top level",
                    key.name
                ),
            });
        }
        let body = self.reached[key]
            .body
            .as_ref()
            .expect("the first pass expanded every reached body");
        let merge_site = MergeSite { name: &key.name, site: &site };
        let summary = match body {
            ExpandedDef::Union(arms) => {
                self.union_summary(arms, Vec::new(), None, &site, summaries, visiting)?
            }
            ExpandedDef::Single(atom, provenance) => {
                let own: Vec<Tagged> = tagged(&atom.constraints, &provenance.value)
                    .into_iter()
                    .chain(tagged(&atom.array_constraints, &provenance.array))
                    .collect();
                let (markers, bounds): (Vec<Tagged>, Vec<Tagged>) = (
                    own.iter().filter(|(c, _)| is_use_site_marker(c)).cloned().collect(),
                    own.iter().filter(|(c, _)| is_count_bound(c)).cloned().collect(),
                );
                match (&atom.ty, atom.is_array) {
                    (TypeExpr::Ref(target), false) => {
                        let inner = self.summary_of(target, summaries, visiting)?;
                        UseSummary {
                            markers: merge_constraints(inner.markers.clone(), markers, &merge_site)?,
                            description: atom.description.clone().or(inner.description.clone()),
                            bounds: inner.bounds.iter().cloned().chain(bounds).collect(),
                            ..inner
                        }
                    }
                    (TypeExpr::Union(arms), false) => self.union_summary(
                        arms,
                        markers,
                        atom.description.clone(),
                        &site,
                        summaries,
                        visiting,
                    )?,
                    (ty, true) => {
                        // A list's items carry what their own type hands a use
                        // site, as inlining `T[]` keeps it on the items.
                        let markers = match ty {
                            TypeExpr::Ref(target) => {
                                self.check_list_target(target, &site)?;
                                let items = self.summary_of(target, summaries, visiting)?;
                                merge_constraints(items.markers, markers, &merge_site)?
                            }
                            _ => markers,
                        };
                        UseSummary {
                            outer: DefinitionOuter::Array,
                            array_arm: None,
                            markers,
                            description: atom.description.clone(),
                            bounds,
                            admits_null: false,
                        }
                    }
                    (ty, false) => {
                        let (outer, bounds) = match ty {
                            TypeExpr::InlineObject(shape) => (
                                DefinitionOuter::Object,
                                tagged(&shape.constraints, &provenance.shape)
                                    .into_iter()
                                    .filter(|(c, _)| is_count_bound(c))
                                    .chain(bounds)
                                    .collect(),
                            ),
                            TypeExpr::Tuple(_) => (DefinitionOuter::Tuple, bounds),
                            _ => (DefinitionOuter::Other, bounds),
                        };
                        UseSummary {
                            outer,
                            array_arm: None,
                            markers,
                            description: atom.description.clone(),
                            bounds,
                            admits_null: atom.admits_null(),
                        }
                    }
                }
            }
        };
        visiting.remove(key);
        summaries.insert(key.clone(), summary.clone());
        Ok(summary)
    }

    /// What a reached definition is at its top level, and for a union its
    /// first alternative that is a list, read off its first-pass body
    /// through aliases only.
    fn outer_of(&self, key: &DefKey) -> (DefinitionOuter, Option<usize>) {
        let mut key = key;
        for _ in 0..=self.reached.len() {
            let Some(body) = self.reached.get(key).and_then(|reached| reached.body.as_ref()) else {
                break;
            };
            let atom = match body {
                ExpandedDef::Union(arms) => return (DefinitionOuter::Union, self.list_arm(arms)),
                ExpandedDef::Single(atom, _) => atom,
            };
            if atom.is_array {
                return (DefinitionOuter::Array, None);
            }
            match &atom.ty {
                TypeExpr::Ref(target) => key = target,
                TypeExpr::Union(arms) => return (DefinitionOuter::Union, self.list_arm(arms)),
                TypeExpr::InlineObject(_) => return (DefinitionOuter::Object, None),
                TypeExpr::Tuple(_) => return (DefinitionOuter::Tuple, None),
                TypeExpr::Primitive(_) | TypeExpr::Imported { .. } | TypeExpr::Function(_) => break,
            }
        }
        (DefinitionOuter::Other, None)
    }

    fn list_arm(&self, arms: &[PropertyAtom]) -> Option<usize> {
        arms.iter().position(|arm| match (&arm.ty, arm.is_array) {
            (_, true) => true,
            (TypeExpr::Ref(target), false) => self.outer_of(target).0 == DefinitionOuter::Array,
            _ => false,
        })
    }

    /// Refuses a definition's top level `target[]` where a use site would
    /// refuse it, before the summary of `target` is read: `[]` over a list,
    /// or over a union with an alternative that is a list.
    fn check_list_target(&self, target: &DefKey, site: &SchemaReferenceSite) -> Result<(), SchemaError> {
        match self.outer_of(target) {
            (DefinitionOuter::Array, _) => Err(SchemaError::Convert {
                property: site.path.to_string(),
                message: format!("cannot apply `[]` to the already-array named type `{}`", target.name),
            }),
            (DefinitionOuter::Union, Some(arm)) => Err(SchemaError::UnionReference {
                name: target.name.clone(),
                rejection: Box::new(SchemaUnionRejection::ArrayArm { arm }),
                site: site.clone(),
                definition: Box::new(SchemaReferenceSite {
                    source: target.namespace.source_id(),
                    path: self.reached[target].namespace.named_types.root.property(target.name.as_str()),
                }),
            }),
            _ => Ok(()),
        }
    }

    /// The summary of a union whose alternatives are `arms`, written with
    /// the property markers `own` and `description`.
    fn union_summary(
        &self,
        arms: &[PropertyAtom],
        own: Vec<Tagged>,
        description: Option<String>,
        site: &SchemaReferenceSite,
        summaries: &mut HashMap<DefKey, UseSummary>,
        visiting: &mut HashSet<DefKey>,
    ) -> Result<UseSummary, SchemaError> {
        let merge_site = MergeSite {
            name: path_name(&site.path),
            site,
        };
        let mut markers = own;
        let mut array_arm = None;
        let mut admits_null = false;
        for (index, arm) in arms.iter().enumerate() {
            let origin = |constraint: &Constraint| SchemaConstraintOrigin {
                source: site.source.clone(),
                path: site.path.union_arm(index),
                keyword: constraint.keyword().to_string(),
                occurrence: 0,
                span: None,
            };
            let mut arm_markers: Vec<Tagged> = arm
                .constraints
                .iter()
                .chain(&arm.array_constraints)
                .filter(|constraint| is_arm_property_marker(constraint))
                .map(|constraint| (constraint.clone(), origin(constraint)))
                .collect();
            let mut arm_is_array = arm.is_array;
            admits_null |= arm.admits_null();
            if let (TypeExpr::Ref(target), false) = (&arm.ty, arm.is_array) {
                let inner = self.summary_of(target, summaries, visiting)?;
                let inherited = inner.markers.into_iter().filter(|(c, _)| is_arm_property_marker(c)).collect();
                arm_markers = merge_constraints(inherited, arm_markers, &merge_site)?;
                arm_is_array = inner.outer == DefinitionOuter::Array;
                admits_null |= inner.admits_null;
            }
            if arm_is_array && array_arm.is_none() {
                array_arm = Some(index);
            }
            markers = merge_constraints(markers, arm_markers, &merge_site)?;
        }
        Ok(UseSummary {
            outer: DefinitionOuter::Union,
            array_arm,
            markers,
            description,
            bounds: Vec::new(),
            admits_null,
        })
    }
}

impl ImportEngine<'_> {
    /// The shared definition `base`'s top level is a reference to, when it
    /// is an alias of one.
    pub(super) fn shared_alias_target<'b>(&self, base: &'b ExpandedDef) -> Option<&'b DefKey> {
        let ExpandedDef::Single(atom, _) = base else {
            return None;
        };
        match (&atom.ty, atom.is_array) {
            (TypeExpr::Ref(key), false) if self.shared.contains(key) => Some(key),
            _ => None,
        }
    }

    /// Whether the use `reference` of the shared definition `key`, whose
    /// expansion is `base`, can be a reference that behaves as the inlined
    /// copy does. The first pass makes every representable-looking use a
    /// reference so every body is summarized; the second judges the postfix
    /// against the summary.
    pub(super) fn can_reference(
        &self,
        key: &DefKey,
        base: &ExpandedDef,
        reference: &NamedReference,
        as_arm: bool,
    ) -> bool {
        if !self.is_shareable(base) {
            return false;
        }
        let summary = match &self.summaries {
            None => None,
            Some(summaries) => match summaries.get(key) {
                Some(summary) => Some(summary),
                None => return false,
            },
        };
        if as_arm {
            return self.arm_keeps_reference(base, reference);
        }
        summary.is_none_or(|summary| {
            use_is_representable(
                summary,
                &[],
                &reference.constraints,
                &reference.array_constraints,
                reference.is_array,
            )
        })
    }

    /// Whether an inlined use of `base` may keep the reference `base`'s top
    /// level holds to a shared definition (an alias of one). When it may
    /// not, the alias is expanded again with the chain inlined.
    pub(super) fn alias_use_keeps_reference(
        &self,
        base: &ExpandedDef,
        reference: &NamedReference,
        as_arm: bool,
    ) -> bool {
        let Some(target) = self.shared_alias_target(base) else {
            return true;
        };
        if as_arm {
            return self.arm_keeps_reference(base, reference);
        }
        let no_postfix = !reference.is_array
            && reference.constraints.is_empty()
            && reference.array_constraints.is_empty()
            && reference.description.is_none();
        let (Some(summaries), false) = (&self.summaries, no_postfix) else {
            return true;
        };
        let ExpandedDef::Single(atom, _) = base else {
            return true;
        };
        let inherited: Vec<Constraint> = atom.constraints.iter().chain(&atom.array_constraints).cloned().collect();
        summaries.get(target).is_none_or(|summary| {
            use_is_representable(
                summary,
                &inherited,
                &reference.constraints,
                &reference.array_constraints,
                reference.is_array,
            )
        })
    }

    /// Whether a union alternative may be a reference to `base` rather than
    /// the arm, or arms, inlining would place in the union.
    fn arm_keeps_reference(&self, base: &ExpandedDef, reference: &NamedReference) -> bool {
        reference.constraints.is_empty()
            && reference.array_constraints.is_empty()
            && !self.arm_differs_from_reference(base, 0)
    }

    /// Whether inlining `base` as a union alternative gives the union
    /// something a reference arm would not: a union target's alternatives,
    /// which flatten into the enclosing union so every consumer sees one
    /// list of alternatives, property markers or annotations it hoists, or a
    /// list, which `[]` over the union refuses.
    fn arm_differs_from_reference(&self, base: &ExpandedDef, depth: usize) -> bool {
        let atom = match base {
            ExpandedDef::Union(_) => return true,
            ExpandedDef::Single(atom, _) => atom,
        };
        if atom.is_array
            || atom
                .constraints
                .iter()
                .chain(&atom.array_constraints)
                .any(is_use_site_marker)
        {
            return true;
        }
        match &atom.ty {
            TypeExpr::Union(_) => true,
            TypeExpr::Ref(target) => {
                if depth > self.reached.len() {
                    return true;
                }
                match self.reached.get(target).and_then(|reached| reached.body.as_ref()) {
                    Some(body) => self.arm_differs_from_reference(body, depth + 1),
                    None => true,
                }
            }
            _ => false,
        }
    }

    /// Whether `base`, a shared definition's expansion, behaves the same
    /// stored once as at each use. Its top level must not admit null (which
    /// makes each use optional) or carry an annotation the converter places
    /// at the use (`operands`, `deferred`); a union's alternatives must
    /// carry no property marker or suggestion, which a property hoists or
    /// counts, no optional `file` alternative, whose empty-string tolerance
    /// depends on the use, and no reference into a cycle.
    ///
    /// A `file(match(...))` inside the value does not matter here: a
    /// root-union arm lowers a reference to a shared acyclic definition as
    /// the inlined value, suggesting its globs (see `MatchCopies` in
    /// `simplified/convert.rs`).
    fn is_shareable(&self, base: &ExpandedDef) -> bool {
        let (top, arms): (Vec<&Constraint>, Option<&[PropertyAtom]>) = match base {
            ExpandedDef::Union(arms) => (Vec::new(), Some(arms)),
            ExpandedDef::Single(atom, _) => {
                if atom.admits_null() {
                    return false;
                }
                let arms = match (&atom.ty, atom.is_array) {
                    (TypeExpr::Union(arms), false) => Some(arms.as_slice()),
                    _ => None,
                };
                (atom.constraints.iter().chain(&atom.array_constraints).collect(), arms)
            }
        };
        if top
            .iter()
            .any(|constraint| matches!(constraint, Constraint::Operands(_) | Constraint::Deferred))
        {
            return false;
        }
        let Some(arms) = arms else {
            return true;
        };
        if top.iter().any(|constraint| is_arm_property_marker(constraint)) {
            return false;
        }
        arms.iter().all(|arm| {
            !arm.admits_null()
                && !arm
                    .constraints
                    .iter()
                    .chain(&arm.array_constraints)
                    .any(|constraint| is_arm_property_marker(constraint) || matches!(constraint, Constraint::Suggest(_)))
                && !(matches!(arm.ty, TypeExpr::Primitive(SimplifiedType::File)) && !arm.is_array)
                && !matches!(&arm.ty, TypeExpr::Ref(target) if self.cyclic.contains(target))
        })
    }
}

/// Whether a use with the postfix `value`/`array` (`is_array` for `[]`),
/// over a reference that already holds `inherited`, lowers beside the
/// unchanged body of a definition summarized by `summary` exactly as the
/// inlined copy does.
///
/// Beside a reference, the use's key bounds count keys in their own scope;
/// inlined, they merge with the body's into one. The two agree only when
/// both scopes leave out the same keys, or when one side has no bound for
/// the other's exclusion to change: `uncounted-keys(note)` at a use of a
/// body bounded by `max-keys(1)` must inline.
fn use_is_representable(
    summary: &UseSummary,
    inherited: &[Constraint],
    value: &[Constraint],
    array: &[Constraint],
    is_array: bool,
) -> bool {
    if inherited
        .iter()
        .chain(value)
        .chain(array)
        .any(|constraint| matches!(constraint, Constraint::Suggest(_) | Constraint::Options(_)))
    {
        return false;
    }
    if is_array {
        let union_conflict = summary.outer == DefinitionOuter::Union
            && (summary.array_arm.is_some()
                || summary
                    .markers
                    .iter()
                    .map(|(constraint, _)| constraint)
                    .chain(inherited)
                    .any(|constraint| matches!(constraint, Constraint::Default(_))));
        if !value.is_empty()
            || summary.outer == DefinitionOuter::Array
            || union_conflict
            || !array
                .iter()
                .all(|constraint| is_use_site_marker(constraint) || is_array_container_constraint(constraint))
        {
            return false;
        }
    } else {
        let allowed = |constraint: &Constraint| match summary.outer {
            DefinitionOuter::Union => is_union_marker(constraint),
            DefinitionOuter::Object => {
                is_use_site_marker(constraint)
                    || matches!(
                        constraint,
                        Constraint::MinKeys(_) | Constraint::MaxKeys(_) | Constraint::UncountedKeys(_)
                    )
            }
            DefinitionOuter::Array => {
                is_use_site_marker(constraint)
                    || matches!(
                        constraint,
                        Constraint::MinItems(_)
                            | Constraint::MaxItems(_)
                            | Constraint::Unique
                            | Constraint::Min(_)
                            | Constraint::Max(_)
                    )
            }
            _ => is_use_site_marker(constraint),
        };
        if !array.is_empty() || !value.iter().all(allowed) {
            return false;
        }
    }
    if summary.outer != DefinitionOuter::Object {
        return true;
    }
    let sibling: Vec<&Constraint> = if is_array {
        inherited.iter().collect()
    } else {
        inherited.iter().chain(value).collect()
    };
    let (use_excluded, use_bounded) = key_scope(sibling.into_iter());
    let (body_excluded, body_bounded) = key_scope(summary.bounds.iter().map(|(constraint, _)| constraint));
    match use_excluded {
        None => !use_bounded || body_excluded.is_none(),
        Some(keys) => body_excluded.as_ref() == Some(&keys) || (body_excluded.is_none() && !body_bounded),
    }
}

/// The keys a scope's `uncounted-keys` leave out, and whether it bounds the
/// key count.
fn key_scope<'c>(constraints: impl Iterator<Item = &'c Constraint>) -> (Option<BTreeSet<&'c str>>, bool) {
    let mut excluded: Option<BTreeSet<&str>> = None;
    let mut bounded = false;
    for constraint in constraints {
        match constraint {
            Constraint::UncountedKeys(keys) => excluded.get_or_insert_default().extend(keys.iter().map(String::as_str)),
            Constraint::MinKeys(_) | Constraint::MaxKeys(_) => bounded = true,
            _ => {}
        }
    }
    (excluded, bounded)
}

/// Calls `found` with every definition a reference in `schema` names.
fn schema_refs(schema: &SimplifiedSchema, found: &mut impl FnMut(&DefKey)) {
    match schema {
        SimplifiedSchema::Single(shape) => shape_refs(shape, found),
        SimplifiedSchema::Union(arms) => {
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    shape_refs(shape, found);
                }
            }
        }
    }
}

fn expanded_refs(body: &ExpandedDef, found: &mut impl FnMut(&DefKey)) {
    match body {
        ExpandedDef::Single(atom, _) => atom_refs(atom, found),
        ExpandedDef::Union(arms) => arms.iter().for_each(|arm| atom_refs(arm, found)),
    }
}

fn shape_refs(shape: &SchemaShape, found: &mut impl FnMut(&DefKey)) {
    for def in shape.properties.values().chain(shape.pattern_keys.iter().map(|pattern| &pattern.def)) {
        def_refs(def, found);
    }
}

fn def_refs(def: &PropertyDef, found: &mut impl FnMut(&DefKey)) {
    match def {
        PropertyDef::Single(atom) => atom_refs(atom, found),
        PropertyDef::Union(arms) => arms.iter().for_each(|arm| atom_refs(arm, found)),
    }
}

fn atom_refs(atom: &PropertyAtom, found: &mut impl FnMut(&DefKey)) {
    match &atom.ty {
        TypeExpr::Ref(key) => found(key),
        TypeExpr::InlineObject(shape) => shape_refs(shape, found),
        TypeExpr::Union(arms) => arms.iter().for_each(|arm| atom_refs(arm, found)),
        TypeExpr::Tuple(tuple) => {
            for element in &tuple.elements {
                def_refs(&element.def, found);
            }
            if let Some(spread) = &tuple.spread {
                atom_refs(spread, found);
            }
        }
        TypeExpr::Primitive(_) | TypeExpr::Imported { .. } | TypeExpr::Function(_) => {}
    }
}

/// The name a definition path's single segment holds.
fn path_name(path: &SchemaSourcePath) -> &str {
    match path.segments().first() {
        Some(SchemaSourcePathSegment::Property(name)) => name,
        _ => "",
    }
}

/// A first-pass use site: the use site's postfix laid over `base`, with no
/// rule that needs the definition's summary applied yet.
fn first_pass_postfix(
    mut base: PropertyAtom,
    mut provenance: AtomProvenance,
    reference: NamedReference,
    site: &SchemaReferenceSite,
) -> ExpandedDef {
    let (value, array) = authored_origins(
        &reference.constraints,
        &reference.array_constraints,
        &site.source,
        &site.path,
    );
    base.is_array = reference.is_array;
    base.constraints.extend(reference.constraints);
    provenance.value.extend(value);
    base.array_constraints.extend(reference.array_constraints);
    provenance.array.extend(array);
    base.description = reference.description.or(base.description);
    ExpandedDef::Single(Box::new(base), provenance)
}

/// Applies a use-site postfix to a reference whose inherited markers are
/// `base`'s constraints, under the rules an inlined reference to the same
/// definition follows ([`super::apply_import_postfix`] and
/// [`super::apply_union_postfix`]):
///
/// - over a union, only the property markers apply without `[]`; with `[]`
///   the list's container constraints apply too, an alternative that is an
///   array refuses it, and an inherited default must be an array;
/// - over anything else, `[]` is refused when the definition is already an
///   array; a use site's `min`/`max` count a definition list's items; and
///   bounds that contradict the definition's own are a conflict naming both;
/// - a definition that admits null is never required.
fn apply_reference_postfix(
    base: PropertyAtom,
    provenance: AtomProvenance,
    summary: &UseSummary,
    reference: NamedReference,
    site: &SchemaReferenceSite,
    definition: &SchemaReferenceSite,
) -> Result<ExpandedDef, SchemaError> {
    let NamedReference {
        name,
        reference: authored,
        is_array,
        constraints,
        array_constraints,
        description,
    } = reference;
    let merge_site = MergeSite { name: &name, site };
    let (use_value_origins, use_array_origins) =
        authored_origins(&constraints, &array_constraints, &site.source, &site.path);
    let inherited_value = tagged(&base.constraints, &provenance.value);
    let inherited_array = tagged(&base.array_constraints, &provenance.array);
    let description = description.or(base.description);

    let (value, array) = if summary.outer == DefinitionOuter::Union {
        let reject = |rejection: SchemaUnionRejection| SchemaError::UnionReference {
            name: name.clone(),
            rejection: Box::new(rejection),
            site: site.clone(),
            definition: Box::new(definition.clone()),
        };
        let marker_only = |list: &[Constraint], array_level: bool| {
            list.iter()
                .find(|constraint| {
                    !is_union_marker(constraint) && !(array_level && is_array && is_array_container_constraint(constraint))
                })
                .map(|rejected| {
                    reject(SchemaUnionRejection::Constraint {
                        keyword: rejected.keyword().to_string(),
                        array_level,
                    })
                })
        };
        if let Some(error) = marker_only(&constraints, false).or_else(|| marker_only(&array_constraints, is_array)) {
            return Err(error);
        }
        let use_value: Vec<Tagged> = constraints.into_iter().zip(use_value_origins).collect();
        let use_array: Vec<Tagged> = array_constraints.into_iter().zip(use_array_origins).collect();
        if is_array {
            if let Some(arm) = summary.array_arm {
                return Err(reject(SchemaUnionRejection::ArrayArm { arm }));
            }
            let lifted: Vec<Tagged> = inherited_value.into_iter().chain(inherited_array).collect();
            if let Some((Constraint::Default(value), origin)) = lifted
                .iter()
                .find(|(constraint, _)| matches!(constraint, Constraint::Default(value) if !value.is_array()))
            {
                return Err(reject(SchemaUnionRejection::ScalarDefault {
                    arm: arm_index(&origin.path),
                    value: value.clone(),
                }));
            }
            let use_site = use_value.into_iter().chain(use_array).collect();
            (Vec::new(), merge_constraints(lifted, use_site, &merge_site)?)
        } else {
            (
                merge_constraints(inherited_value, use_value, &merge_site)?,
                merge_constraints(inherited_array, use_array, &merge_site)?,
            )
        }
    } else {
        if constraints.iter().any(|constraint| matches!(constraint, Constraint::Options(_))) {
            return Err(SchemaError::Convert {
                property: site.path.to_string(),
                message: format!(
                    "`alphanumeric` options are not valid on the named type `{name}@{authored}`, \
                     which is not `alphanumeric`"
                ),
            });
        }
        if is_array && let Some(first) = constraints.first() {
            return Err(SchemaError::Convert {
                property: site.path.to_string(),
                message: format!(
                    "constraint `{}` before `[]` is not valid on the named type `{name}@{authored}`; \
                     only a named union accepts `required`, `generated`, or `eager` there, and \
                     constraints after `[]` apply to the list",
                    first.keyword()
                ),
            });
        }
        if let Some(span) = constraints
            .iter()
            .chain(&array_constraints)
            .find_map(|constraint| match constraint {
                Constraint::Suggest(candidates) => Some(candidates.first().map(|c| c.span.clone()).unwrap_or(0..0)),
                _ => None,
            })
        {
            return Err(SchemaError::Grammar {
                property: name.to_string(),
                message: format!(
                    "`suggest` on imported type `{name}@{authored}` requires an exact `string`, `number`, or \
                     `alphanumeric` target"
                ),
                span,
                related: Vec::new(),
            });
        }
        let use_site: Vec<Tagged> = constraints
            .into_iter()
            .zip(use_value_origins)
            .chain(array_constraints.into_iter().zip(use_array_origins))
            .collect();
        if is_array {
            if summary.outer == DefinitionOuter::Array {
                return Err(SchemaError::Convert {
                    property: name.to_string(),
                    message: format!("cannot apply `[]` to the already-array named type `{name}@{authored}`"),
                });
            }
            (inherited_value, merge_constraints(Vec::new(), use_site, &merge_site)?)
        } else {
            let property = site.path.to_string();
            let use_site = if summary.outer == DefinitionOuter::Array {
                reinterpret_bounds(use_site, CountedBound::Items, &property)?
            } else {
                use_site
            };
            // The definition's bounds stay in its body; merging against them
            // only reports a contradiction with both declarations.
            let against = summary.bounds.iter().cloned().chain(inherited_value.iter().cloned()).collect();
            merge_constraints(against, use_site.clone(), &merge_site)?;
            (merge_constraints(inherited_value, use_site, &merge_site)?, inherited_array)
        }
    };
    let (mut value, mut array) = reconcile_property_defaults(value, array, &merge_site)?;
    if summary.admits_null {
        value.retain(|(constraint, _)| !matches!(constraint, Constraint::Required));
        array.retain(|(constraint, _)| !matches!(constraint, Constraint::Required));
    }
    let (constraints, value_origins) = value.into_iter().unzip();
    let (array_constraints, array_origins) = array.into_iter().unzip();
    Ok(ExpandedDef::Single(
        Box::new(PropertyAtom {
            ty: base.ty,
            is_array: is_array || base.is_array,
            constraints,
            array_constraints,
            description,
        }),
        AtomProvenance {
            value: value_origins,
            array: array_origins,
            shape: Vec::new(),
        },
    ))
}

/// The union arm a definition-relative origin path ends in.
fn arm_index(path: &SchemaSourcePath) -> usize {
    match path.segments().last() {
        Some(SchemaSourcePathSegment::UnionArm(index)) => *index,
        _ => 0,
    }
}

/// A definition body without what its use sites took from it.
fn strip_use_site_markers(body: ExpandedDef) -> PropertyDef {
    let strip_arm = |arm: &mut PropertyAtom| {
        arm.constraints.retain(|constraint| !is_arm_property_marker(constraint));
        arm.array_constraints.retain(|constraint| !is_arm_property_marker(constraint));
    };
    match body {
        ExpandedDef::Union(mut arms) => {
            arms.iter_mut().for_each(strip_arm);
            PropertyDef::Union(arms)
        }
        ExpandedDef::Single(atom, _) => {
            let mut atom = *atom;
            atom.constraints.retain(|constraint| !is_use_site_marker(constraint));
            atom.array_constraints.retain(|constraint| !is_use_site_marker(constraint));
            atom.description = None;
            if !atom.is_array && let TypeExpr::Union(arms) = &mut atom.ty {
                arms.iter_mut().for_each(strip_arm);
            }
            PropertyDef::Single(atom)
        }
    }
}
