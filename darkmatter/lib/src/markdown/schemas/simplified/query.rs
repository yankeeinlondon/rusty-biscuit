//! Library-facing completion query over resolved SimplifiedSchema suggestions.
//!
//! The public entry point [`suggestions_for_path`] returns lint-valid
//! `suggest(...)` candidates for a property path, preserving declaration order
//! and retaining scalar/array-item type information. This is the single source
//! DMLS completion reads from — it does not consult raw generated
//! `x-darkmatter-suggest` annotations.
//!
//! ## Union selection
//!
//! - **Property-level union**: the sole eligible suggestion-bearing atom is
//!   used (cardinality guarantees at most one). All atoms remain linted. A
//!   resolved union-typed reference (`U`, `U(required)`, `U[]`) is read
//!   through its alternatives the same way; for `U[]` the alternatives are
//!   the item's, and the query reports `is_array`.
//! - **Root-level union**: the first declaration-order arm with an eligible
//!   suggestion-bearing definition for the property is used. All arms remain
//!   linted.
//!
//! The selected atom only stores the list: its candidates are kept when any
//! alternative of the property at their insertion site accepts them, as
//! [`super::lint_suggestions`] judges.
//!
//! ## Side-effect freedom
//!
//! The query performs no filesystem discovery, shell execution, composition
//! directive evaluation, or network access. It reads the SimplifiedSchema AST
//! and the in-memory lint results only.

use super::{
    Constraint, DefKey, Definition, DefinitionTable, PropertyAtom, PropertyDef, SchemaArm, SchemaShape,
    SimplifiedSchema, SuggestionCandidate, SuggestionLintProblem, TypeExpr, lint_suggestions,
};

/// One lint-valid suggestion candidate available for value completion.
///
/// Produced by [`suggestions_for_path`]. Each item carries YAML-safe
/// insertion text ready for a text edit: double-quoted for strings (with
/// required escaping), canonical decimal for numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct SuggestionItem {
    /// Decoded argument text (after quote/escape processing).
    pub decoded: String,
    /// Target-directed interpreted JSON value.
    pub value: serde_json::Value,
    /// Canonical decimal text for numeric candidates. `None` for strings.
    pub canonical_decimal: Option<String>,
    /// YAML-safe insertion text: double-quoted for strings, canonical decimal
    /// for numbers.
    pub insert_text: String,
    /// Display label (decoded text for strings, canonical decimal for numbers).
    pub label: String,
    /// Whether this candidate targets a `string` property.
    pub is_string: bool,
    /// Whether this candidate targets a `number` property.
    pub is_number: bool,
}

/// The result of querying suggestions for one property path.
///
/// Returned by [`suggestions_for_path`]. Candidates are in declaration order
/// with invalid candidates (those identified by [`super::lint_suggestions`])
/// omitted; valid siblings are retained.
#[derive(Debug, Clone, PartialEq)]
pub struct SuggestionQuery {
    /// Whether the eligible type is an array form (`string[]` / `number[]`),
    /// or an alternative of an array of a union (`U[]`).
    ///
    /// When `true`, completion should insert one candidate element at the
    /// cursor — not a collection of suggestions or an entire array.
    pub is_array: bool,
    /// Lint-valid candidates in declaration order.
    pub items: Vec<SuggestionItem>,
}

/// Queries lint-valid suggestion candidates for `property_path` in `schema`.
///
/// `property_path` is a slice of segments (e.g. `["settings", "mode"]`) that
/// descends through inline objects. A single segment queries a top-level
/// property.
///
/// Returns `None` when the property has no `suggest(...)` constraint, when the
/// schema is a raw JSON Schema (`SimplifiedSchema` unavailable), or when the
/// path does not resolve.
///
/// Invalid candidates (those with a lint problem) are omitted; valid siblings
/// are retained in declaration order.
///
/// ## Union selection
///
/// For a property-level union, the sole eligible suggestion-bearing atom is
/// used (cardinality guarantees at most one). For a root-level union, the
/// first declaration-order arm with an eligible suggestion-bearing definition
/// for the property is used. All arms and atoms remain linted regardless of
/// which is selected for completion.
///
/// ## Side-effect freedom
///
/// No filesystem discovery, shell execution, composition directive evaluation,
/// or network access occurs.
pub fn suggestions_for_path(
    schema: &SimplifiedSchema,
    property_path: &[&str],
) -> Option<SuggestionQuery> {
    let (leaf, ancestors) = property_path.split_last()?;
    let (alternative, root_arm) = suggest_atom_and_provenance(schema, ancestors, leaf)?;
    let lints = lint_suggestions(schema).unwrap_or_default();
    let items = alternative_items(&alternative, &lints, property_path, &root_arm);
    let is_array = alternative.is_array;
    Some(SuggestionQuery { is_array, items })
}

/// Queries lint-valid suggestion candidates from an already-resolved property
/// definition.
///
/// This is the context-aware companion to [`suggestions_for_path`]. A caller
/// that has resolved a property to its selected discriminated-union arm (or a
/// merged-union fallback view) passes that `PropertyDef` here, so the candidates
/// follow the caller's selected arm rather than the context-free first-arm
/// choice [`suggestions_for_path`] makes during its own ancestor traversal.
///
/// `leaf` is the property's leaf key name, used only for target-schema error
/// context while linting candidates.
///
/// Returns `None` when no atom of `def` carries a `suggest(...)` constraint.
/// Candidates from every suggest-bearing atom are combined in atom-declaration
/// order — so a merged union carrying suggestions on more than one arm offers
/// all of them. A candidate is judged against the whole of `def`, so one stored
/// on a union arm is kept when any alternative at its insertion site accepts it
/// (see [`super::lint_suggestions`]). `is_array` is set when any suggest-bearing
/// atom is an array form or an item alternative of an array of a union (`U[]`).
///
/// ## Side-effect freedom
///
/// No filesystem discovery, shell execution, composition directive evaluation,
/// or network access occurs.
pub fn suggestions_for_def(leaf: &str, def: &PropertyDef) -> Option<SuggestionQuery> {
    let by_alternative = suggestions_by_alternative(leaf, def);
    if by_alternative.is_empty() {
        return None;
    }
    let is_array = by_alternative.iter().any(|(alternative, _)| alternative.is_array);
    let items = by_alternative
        .into_iter()
        .flat_map(|(_, items)| items)
        .collect();
    Some(SuggestionQuery { is_array, items })
}

/// Lint-valid candidates of each suggest-bearing alternative of `def`, in
/// declaration order, judged against the whole definition.
///
/// The alternatives borrow from `def`, so a caller walking
/// [`flattened_alternatives`] of the same definition can match them by atom
/// identity.
pub(crate) fn suggestions_by_alternative<'a>(
    leaf: &str,
    def: &'a PropertyDef,
) -> Vec<(Alternative<'a>, Vec<SuggestionItem>)> {
    let alternatives: Vec<Alternative<'a>> = flattened_alternatives(def)
        .into_iter()
        .filter(|alternative| has_suggest(alternative.atom))
        .collect();
    if alternatives.is_empty() {
        return Vec::new();
    }
    // Lint the definition as a one-property schema so every alternative is in
    // view; isolating the storing atom would judge a shared union list against
    // that one arm.
    let mut shape = SchemaShape::default();
    shape.properties.insert(leaf.to_string(), def.clone());
    let lints = lint_suggestions(&SimplifiedSchema::Single(shape)).unwrap_or_default();
    alternatives
        .into_iter()
        .map(|alternative| {
            let items = alternative_items(&alternative, &lints, &[leaf], &None);
            (alternative, items)
        })
        .collect()
}

/// The lint-valid candidates stored on one alternative.
fn alternative_items(
    alternative: &Alternative<'_>,
    lints: &[SuggestionLintProblem],
    property_path: &[&str],
    root_arm: &Option<usize>,
) -> Vec<SuggestionItem> {
    let atom = alternative.atom;
    let Some(candidates) = atom.constraints.iter().find_map(|c| match c {
        Constraint::Suggest(candidates) => Some(candidates.as_slice()),
        _ => None,
    }) else {
        return Vec::new();
    };
    let is_string = matches!(atom.ty, TypeExpr::Primitive(super::SimplifiedType::String));
    let is_number = matches!(atom.ty, TypeExpr::Primitive(super::SimplifiedType::Number));
    candidates
        .iter()
        .filter(|candidate| {
            !is_lint_problem(
                candidates,
                lints,
                property_path,
                root_arm,
                &alternative.arm,
                candidate,
            )
        })
        .map(|candidate| build_item(candidate, is_string, is_number))
        .collect()
}

/// Resolves the alternative carrying `suggest(...)` for a path together with
/// the lint-problem root-arm provenance (the property-arm provenance is the
/// alternative's [`Alternative::arm`]), applying the union-selection rules.
///
/// Provenance is derived from the same traversal that selects the atom so lint
/// filtering always targets the arm/atom the suggestion came from. A root-union
/// arm whose ancestors do not resolve is skipped (continuing to later arms)
/// rather than aborting the whole query.
fn suggest_atom_and_provenance<'a>(
    schema: &'a SimplifiedSchema,
    ancestors: &[&str],
    leaf: &str,
) -> Option<(Alternative<'a>, Option<usize>)> {
    match schema {
        SimplifiedSchema::Single(shape) => {
            let shape = descend(shape, ancestors)?;
            let def = shape.properties.get(leaf)?;
            Some((suggest_atom_in_def(def)?, None))
        }
        SimplifiedSchema::Union(arms) => {
            for (root_arm, arm) in arms.iter().enumerate() {
                if let SchemaArm::Inline(shape) = arm {
                    let Some(shape) = descend(shape, ancestors) else {
                        continue;
                    };
                    let Some(def) = shape.properties.get(leaf) else {
                        continue;
                    };
                    if let Some(alternative) = suggest_atom_in_def(def) {
                        return Some((alternative, Some(root_arm)));
                    }
                }
            }
            None
        }
    }
}

/// Descends through inline-object ancestors to reach the leaf's shape.
fn descend<'a>(root: &'a SchemaShape, ancestors: &[&str]) -> Option<&'a SchemaShape> {
    let mut shape = root;
    for segment in ancestors {
        let def = shape.properties.get(*segment)?;
        shape = inline_object_shape(def)?;
    }
    Some(shape)
}

/// The inline-object [`SchemaShape`] of the first inline-object arm.
fn inline_object_shape(def: &PropertyDef) -> Option<&SchemaShape> {
    atoms_of(def).iter().find_map(|atom| match &atom.ty {
        TypeExpr::InlineObject(inner) => Some(inner),
        TypeExpr::Primitive(_)
        | TypeExpr::Imported { .. }
        | TypeExpr::Union(_)
        | TypeExpr::Tuple(_)
        | TypeExpr::Function(_)
        | TypeExpr::Ref(_) => None,
    })
}

/// The first alternative of `def` carrying `suggest(...)`.
fn suggest_atom_in_def(def: &PropertyDef) -> Option<Alternative<'_>> {
    flattened_alternatives(def)
        .into_iter()
        .find(|alternative| has_suggest(alternative.atom))
}

fn has_suggest(atom: &PropertyAtom) -> bool {
    atom.constraints
        .iter()
        .any(|constraint| matches!(constraint, Constraint::Suggest(_)))
}

/// One leaf alternative of a property definition, as produced by
/// [`flattened_alternatives`].
#[derive(Debug, Clone, Copy)]
pub struct Alternative<'a> {
    /// The non-union atom. When [`Alternative::is_array`] is set, its
    /// `constraints` bind to one array element.
    pub atom: &'a PropertyAtom,
    /// The atom's index in its innermost enclosing alternative list, or `None`
    /// for a definition that is not a union. This is the `property_arm`
    /// [`super::lint_suggestions`] reports for the atom.
    pub arm: Option<usize>,
    /// Whether the property holds an array whose element is this atom: the
    /// atom's own `[]`, or the `[]` of an enclosing union-typed reference
    /// (`U[]`).
    pub is_array: bool,
}

/// Every leaf alternative of `def`, in declaration order: the atoms of a
/// property-level union, and the arms of a resolved union-typed reference
/// (`U`, `U(required)`, `U[]`) wherever it appears, looking through nested
/// union arms. A union atom never appears in the result itself.
pub fn flattened_alternatives(def: &PropertyDef) -> Vec<Alternative<'_>> {
    let mut out = Vec::new();
    match def {
        PropertyDef::Single(atom) => push_alternatives(atom, None, false, &mut out),
        PropertyDef::Union(atoms) => {
            for (index, atom) in atoms.iter().enumerate() {
                push_alternatives(atom, Some(index), false, &mut out);
            }
        }
    }
    out
}

fn push_alternatives<'a>(
    atom: &'a PropertyAtom,
    arm: Option<usize>,
    outer_array: bool,
    out: &mut Vec<Alternative<'a>>,
) {
    let is_array = outer_array || atom.is_array;
    match atom.union_arms() {
        Some(arms) => {
            for (index, inner) in arms.iter().enumerate() {
                push_alternatives(inner, Some(index), is_array, out);
            }
        }
        None => out.push(Alternative { atom, arm, is_array }),
    }
}

/// Where the body of a [`TypeExpr::Ref`] is looked up.
///
/// Reference-aware queries ask for one definition at a time, only when the
/// position they are answering is a reference, so an implementation sees
/// exactly the definitions a query needed. [`DefinitionTable`] and
/// [`EffectiveSchema`](crate::markdown::schemas::EffectiveSchema) implement it;
/// a caller may wrap either to observe or restrict lookups.
pub trait DefinitionLookup {
    /// The definition `key` names; `None` when it is not in scope, which a
    /// query treats as offering nothing, never as an unconstrained value.
    fn lookup_definition(&self, key: &DefKey) -> Option<&Definition>;
}

impl DefinitionLookup for DefinitionTable {
    fn lookup_definition(&self, key: &DefKey) -> Option<&Definition> {
        self.get(key)
    }
}

/// [`flattened_alternatives`] with each reference replaced by the leaf
/// alternatives of its definition's body, looked up in `definitions`.
///
/// Only references at the top of `def` resolve: a reference used directly, as
/// a union arm, or as the element of `def`'s array, followed through alias and
/// union bodies. A reference nested inside an inline object's properties is
/// left for a later query that descends into that property, so the result is
/// one level of the recursive type and never its closure.
///
/// A body alternative keeps the body's [`Alternative::arm`]. A reference that
/// stays in the result is one the query could not open: its definition is
/// missing, it closes an alias loop, or it is an array element whose body is
/// itself an array (`list[]` of `list: item[]`), which has no leaf alternative
/// at the element site. Such a reference offers nothing.
pub fn resolved_alternatives<'a, L>(def: &'a PropertyDef, definitions: &'a L) -> Vec<Alternative<'a>>
where
    L: DefinitionLookup + ?Sized,
{
    resolve_with_bodies(def, definitions).0
}

/// [`resolved_alternatives`] together with every definition body it opened,
/// in first-opened order. A body is the scope its own `suggest(...)`
/// candidates are linted in.
pub(crate) fn resolve_with_bodies<'a, L>(
    def: &'a PropertyDef,
    definitions: &'a L,
) -> (Vec<Alternative<'a>>, Vec<&'a PropertyDef>)
where
    L: DefinitionLookup + ?Sized,
{
    let mut resolver = Resolver {
        definitions,
        open: Vec::new(),
        bodies: Vec::new(),
    };
    let mut out = Vec::new();
    resolver.push_def(def, false, &mut out);
    (out, resolver.bodies)
}

struct Resolver<'a, L: ?Sized> {
    definitions: &'a L,
    /// The references being opened on the current alias chain.
    open: Vec<&'a DefKey>,
    bodies: Vec<&'a PropertyDef>,
}

impl<'a, L: DefinitionLookup + ?Sized> Resolver<'a, L> {
    fn push_def(&mut self, def: &'a PropertyDef, outer_array: bool, out: &mut Vec<Alternative<'a>>) {
        match def {
            PropertyDef::Single(atom) => self.push_atom(atom, None, outer_array, out),
            PropertyDef::Union(atoms) => {
                for (index, atom) in atoms.iter().enumerate() {
                    self.push_atom(atom, Some(index), outer_array, out);
                }
            }
        }
    }

    fn push_atom(
        &mut self,
        atom: &'a PropertyAtom,
        arm: Option<usize>,
        outer_array: bool,
        out: &mut Vec<Alternative<'a>>,
    ) {
        let is_array = outer_array || atom.is_array;
        let key = match &atom.ty {
            TypeExpr::Union(arms) => {
                for (index, inner) in arms.iter().enumerate() {
                    self.push_atom(inner, Some(index), is_array, out);
                }
                return;
            }
            TypeExpr::Ref(key) => key,
            _ => {
                out.push(Alternative { atom, arm, is_array });
                return;
            }
        };
        // Resolution rejects an alias cycle; an adversarial lookup must still
        // not loop.
        let definition = match self.definitions.lookup_definition(key) {
            Some(definition) if !self.open.contains(&key) => definition,
            _ => {
                out.push(Alternative { atom, arm, is_array });
                return;
            }
        };
        if !self.bodies.iter().any(|body| std::ptr::eq(*body, &definition.body)) {
            self.bodies.push(&definition.body);
        }
        self.open.push(key);
        let mut body = Vec::new();
        self.push_def(&definition.body, false, &mut body);
        self.open.pop();
        let mut kept_reference = false;
        for alternative in body {
            if is_array && alternative.is_array {
                if !kept_reference {
                    out.push(Alternative { atom, arm, is_array });
                    kept_reference = true;
                }
                continue;
            }
            out.push(Alternative {
                is_array: is_array || alternative.is_array,
                ..alternative
            });
        }
    }
}

/// Lint-valid candidates of each suggest-bearing alternative of `def` and of
/// each definition body in `bodies` (as [`resolve_with_bodies`] returns them),
/// each judged against the definition that stores it.
pub(crate) fn suggestions_by_resolved_alternative<'a>(
    leaf: &str,
    def: &'a PropertyDef,
    bodies: &[&'a PropertyDef],
) -> Vec<(Alternative<'a>, Vec<SuggestionItem>)> {
    std::iter::once(def)
        .chain(bodies.iter().copied())
        .flat_map(|source| suggestions_by_alternative(leaf, source))
        .collect()
}

/// Every definition `atom` reaches through `definitions`, transitively, as a
/// table a lowering of `atom` can resolve its references in. `None` when a
/// reached reference has no definition.
///
/// This is the whole closure, as validating a value needs; completion's path
/// descent never calls it.
pub(crate) fn reachable_definitions<L>(atom: &PropertyAtom, definitions: &L) -> Option<DefinitionTable>
where
    L: DefinitionLookup + ?Sized,
{
    let mut pending: Vec<DefKey> = Vec::new();
    collect_atom_refs(atom, &mut pending);
    let mut table = DefinitionTable::default();
    while let Some(key) = pending.pop() {
        if table.get(&key).is_some() {
            continue;
        }
        let definition = definitions.lookup_definition(&key)?;
        table.insert(key, definition.body.clone(), definition.origin.clone());
        collect_def_refs(&definition.body, &mut pending);
    }
    Some(table)
}

/// A content fingerprint of `atom` and of every definition it reaches through
/// `definitions`, taken in the order [`reachable_definitions`] collects them,
/// which is also the order a lowering allocates their `$defs` keys in. Equal
/// fingerprints therefore lower to the same schema. `None` when a reached
/// reference has no definition, exactly when [`reachable_definitions`] is.
///
/// The derived `Debug` text of each value is streamed into the hasher, never
/// built, so fingerprinting a large recursive alternative allocates only its
/// reference keys. The hash is stable within one process only.
pub(crate) fn reachable_fingerprint<L>(atom: &PropertyAtom, definitions: &L) -> Option<u64>
where
    L: DefinitionLookup + ?Sized,
{
    use std::fmt::Write as _;
    use std::hash::Hasher as _;

    let mut hasher = biscuit_hash::XxHasher::new();
    write!(hasher, "{atom:?}").ok()?;
    let mut pending: Vec<DefKey> = Vec::new();
    collect_atom_refs(atom, &mut pending);
    let mut seen = std::collections::HashSet::new();
    while let Some(key) = pending.pop() {
        if seen.contains(&key) {
            continue;
        }
        let definition = definitions.lookup_definition(&key)?;
        write!(hasher, "\u{0}{key:?}\u{0}{:?}\u{0}{:?}", definition.body, definition.origin).ok()?;
        collect_def_refs(&definition.body, &mut pending);
        seen.insert(key);
    }
    Some(hasher.finish())
}

fn collect_def_refs(def: &PropertyDef, out: &mut Vec<DefKey>) {
    match def {
        PropertyDef::Single(atom) => collect_atom_refs(atom, out),
        PropertyDef::Union(atoms) => atoms.iter().for_each(|atom| collect_atom_refs(atom, out)),
    }
}

fn collect_atom_refs(atom: &PropertyAtom, out: &mut Vec<DefKey>) {
    match &atom.ty {
        TypeExpr::Ref(key) => out.push(key.clone()),
        TypeExpr::InlineObject(shape) => {
            for def in shape.properties.values() {
                collect_def_refs(def, out);
            }
            for pattern in &shape.pattern_keys {
                collect_def_refs(&pattern.def, out);
            }
        }
        TypeExpr::Union(arms) => arms.iter().for_each(|arm| collect_atom_refs(arm, out)),
        TypeExpr::Tuple(tuple) => {
            for element in &tuple.elements {
                collect_def_refs(&element.def, out);
            }
            if let Some(spread) = &tuple.spread {
                collect_atom_refs(spread, out);
            }
        }
        TypeExpr::Primitive(_) | TypeExpr::Imported { .. } | TypeExpr::Function(_) => {}
    }
}

/// The atoms whose shapes a property path may descend into: a union's
/// alternatives, including the item alternatives of an array of a union.
fn atoms_of(def: &PropertyDef) -> &[PropertyAtom] {
    match def {
        PropertyDef::Single(atom) => atom.union_arms().unwrap_or(std::slice::from_ref(atom)),
        PropertyDef::Union(atoms) => atoms,
    }
}

/// Whether a candidate is identified as invalid by the lint problems.
fn is_lint_problem(
    candidates: &[SuggestionCandidate],
    lints: &[SuggestionLintProblem],
    property_path: &[&str],
    root_arm: &Option<usize>,
    property_arm: &Option<usize>,
    candidate: &SuggestionCandidate,
) -> bool {
    let index = candidates
        .iter()
        .position(|c| c.decoded == candidate.decoded && c.interpreted == candidate.interpreted);
    let Some(index) = index else {
        return false;
    };
    lints.iter().any(|problem| {
        problem
            .property_path
            .iter()
            .map(String::as_str)
            .eq(property_path.iter().copied())
            && problem.root_arm == *root_arm
            && problem.property_arm == *property_arm
            && problem.decoded == candidate.decoded
            && problem.interpreted == candidate.interpreted
            && candidate_index_matches(problem, candidates, index)
    })
}

fn candidate_index_matches(
    problem: &SuggestionLintProblem,
    candidates: &[SuggestionCandidate],
    target_index: usize,
) -> bool {
    candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.decoded == problem.decoded && c.interpreted == problem.interpreted && problem.span == c.span
        })
        .any(|(i, _)| i == target_index)
}

/// Builds a [`SuggestionItem`] from a candidate.
fn build_item(candidate: &SuggestionCandidate, is_string: bool, is_number: bool) -> SuggestionItem {
    let (label, insert_text) = if is_number {
        let text = candidate
            .canonical_decimal
            .as_deref()
            .unwrap_or(candidate.decoded.as_str());
        (text.to_string(), text.to_string())
    } else {
        (
            candidate.decoded.clone(),
            yaml_double_quote(&candidate.decoded),
        )
    };
    SuggestionItem {
        decoded: candidate.decoded.clone(),
        value: candidate.interpreted.clone(),
        canonical_decimal: candidate.canonical_decimal.clone(),
        insert_text,
        label,
        is_string,
        is_number,
    }
}

/// Produces a YAML-safe double-quoted scalar from `text`.
///
/// Escapes backslash, double quote, and common control characters using YAML
/// 1.2 double-quoted escape sequences (compatible with JSON escaping).
pub fn yaml_double_quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            '\x07' => out.push_str("\\a"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            '\x0b' => out.push_str("\\v"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Prefix-matches a candidate's decoded text against `prefix`, case-sensitive.
///
/// For numeric candidates the decoded text is the authored spelling (e.g.
/// `003.5`), while the canonical label/insert text is normalized (`3.5`).
/// Prefix filtering must use the decoded text so an author's typed prefix
/// matches their authored spelling.
pub fn matches_prefix(item: &SuggestionItem, prefix: &str) -> bool {
    item.decoded.starts_with(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(yaml_body: &str) -> SimplifiedSchema {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml_body).unwrap();
        let schema_value = value.get("$schema").unwrap();
        super::super::parse_yaml_schema(schema_value).unwrap()
    }

    /// The single-shape property definition for `name`.
    fn def<'a>(schema: &'a SimplifiedSchema, name: &str) -> &'a PropertyDef {
        match schema {
            SimplifiedSchema::Single(shape) => shape.properties.get(name).unwrap(),
            SimplifiedSchema::Union(_) => panic!("expected a single-shape schema"),
        }
    }

    /// The first atom of a definition.
    fn atom(def: &PropertyDef) -> &PropertyAtom {
        match def {
            PropertyDef::Single(atom) => atom,
            PropertyDef::Union(atoms) => &atoms[0],
        }
    }

    #[test]
    fn suggestions_for_def_resolves_single_atom() {
        let s = schema("$schema:\n  color: string(suggest(red, green, blue))\n");
        let q = suggestions_for_def("color", def(&s, "color")).unwrap();
        assert!(!q.is_array);
        let labels: Vec<&str> = q.items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, vec!["red", "green", "blue"]);
    }

    #[test]
    fn suggestions_for_def_returns_none_without_suggest() {
        let s = schema("$schema:\n  title: string\n");
        assert!(suggestions_for_def("title", def(&s, "title")).is_none());
    }

    #[test]
    fn suggestions_for_def_omits_invalid_candidates() {
        let s = schema("$schema:\n  count: number(min(0); suggest(1, many, 2))\n");
        let q = suggestions_for_def("count", def(&s, "count")).unwrap();
        let labels: Vec<&str> = q.items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, vec!["1", "2"]);
    }

    #[test]
    fn suggestions_for_def_array_form_sets_is_array() {
        let s = schema("$schema:\n  tags: string(suggest(alpha, beta))[]\n");
        let q = suggestions_for_def("tags", def(&s, "tags")).unwrap();
        assert!(q.is_array);
        assert_eq!(q.items.len(), 2);
    }

    #[test]
    fn suggestions_for_def_combines_every_suggest_atom() {
        // A merged-union view (as DMLS builds for an unresolved discriminated
        // union) carrying `suggest(...)` on more than one atom offers every
        // arm's candidates in atom-declaration order.
        let a = schema("$schema:\n  a: string(suggest(red, green))\n");
        let b = schema("$schema:\n  b: string(suggest(cyan, magenta))\n");
        let merged = PropertyDef::Union(vec![atom(def(&a, "a")).clone(), atom(def(&b, "b")).clone()]);
        let q = suggestions_for_def("palette", &merged).unwrap();
        let labels: Vec<&str> = q.items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, vec!["red", "green", "cyan", "magenta"]);
    }

    #[test]
    fn string_suggestions_preserve_order() {
        let s = schema("$schema:\n  color: string(suggest(red, green, blue))\n");
        let q = suggestions_for_path(&s, &["color"]).unwrap();
        assert!(!q.is_array);
        assert_eq!(q.items.len(), 3);
        assert_eq!(q.items[0].label, "red");
        assert_eq!(q.items[0].insert_text, "\"red\"");
        assert_eq!(q.items[1].label, "green");
        assert_eq!(q.items[1].insert_text, "\"green\"");
        assert_eq!(q.items[2].label, "blue");
    }

    #[test]
    fn number_suggestions_use_canonical_decimal() {
        let s = schema("$schema:\n  ratio: number(suggest(0.25, 0.5, 1))\n");
        let q = suggestions_for_path(&s, &["ratio"]).unwrap();
        assert!(!q.is_array);
        assert_eq!(q.items.len(), 3);
        assert_eq!(q.items[0].label, "0.25");
        assert_eq!(q.items[0].insert_text, "0.25");
        assert_eq!(q.items[1].label, "0.5");
        assert_eq!(q.items[1].insert_text, "0.5");
        assert_eq!(q.items[2].label, "1");
        assert_eq!(q.items[2].insert_text, "1");
    }

    #[test]
    fn array_form_sets_is_array() {
        let s = schema("$schema:\n  tags: string(suggest(alpha, beta))[]\n");
        let q = suggestions_for_path(&s, &["tags"]).unwrap();
        assert!(q.is_array);
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "alpha");
        assert_eq!(q.items[0].insert_text, "\"alpha\"");
    }

    #[test]
    fn invalid_candidates_are_omitted() {
        let s = schema("$schema:\n  count: number(min(0); suggest(1, many, 2))\n");
        let q = suggestions_for_path(&s, &["count"]).unwrap();
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "1");
        assert_eq!(q.items[1].label, "2");
    }

    #[test]
    fn sibling_lint_does_not_hide_identical_valid_candidate() {
        let s = schema(
            "$schema:\n  bad: number(max(0); suggest(1))\n  good: number(max(1); suggest(1))\n",
        );

        assert!(suggestions_for_path(&s, &["bad"]).unwrap().items.is_empty());
        assert_eq!(suggestions_for_path(&s, &["good"]).unwrap().items[0].label, "1");
    }

    #[test]
    fn nested_lint_does_not_hide_identical_valid_candidate() {
        let s = schema(
            "$schema:\n  bad: \"{ value: number(max(0); suggest(1)) }\"\n  good: \"{ value: number(max(1); suggest(1)) }\"\n",
        );

        assert!(suggestions_for_path(&s, &["bad", "value"]).unwrap().items.is_empty());
        assert_eq!(
            suggestions_for_path(&s, &["good", "value"]).unwrap().items[0].label,
            "1"
        );
    }

    #[test]
    fn root_union_property_lint_does_not_hide_identical_valid_candidate() {
        let s = schema(
            "$schema:\n  - bad: number(max(0); suggest(1))\n  - good: number(max(1); suggest(1))\n",
        );

        assert!(suggestions_for_path(&s, &["bad"]).unwrap().items.is_empty());
        assert_eq!(suggestions_for_path(&s, &["good"]).unwrap().items[0].label, "1");
    }

    #[test]
    fn dotted_property_name_does_not_collide_with_nested_path() {
        let s = schema(
            "$schema:\n  a: \"{ b: number(max(0); suggest(1)) }\"\n  a.b: number(max(1); suggest(1))\n",
        );

        assert!(suggestions_for_path(&s, &["a", "b"]).unwrap().items.is_empty());
        assert_eq!(suggestions_for_path(&s, &["a.b"]).unwrap().items[0].label, "1");
    }

    #[test]
    fn no_suggest_returns_none() {
        let s = schema("$schema:\n  title: string\n");
        assert!(suggestions_for_path(&s, &["title"]).is_none());
    }

    #[test]
    fn unknown_property_returns_none() {
        let s = schema("$schema:\n  title: string\n");
        assert!(suggestions_for_path(&s, &["nope"]).is_none());
    }

    #[test]
    fn nested_inline_object_descends() {
        let s = schema("$schema:\n  settings: \"{ mode: string(suggest(fast, slow)) }\"\n");
        let q = suggestions_for_path(&s, &["settings", "mode"]).unwrap();
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "fast");
        assert_eq!(q.items[1].label, "slow");
    }

    #[test]
    fn property_union_uses_sole_suggestion_arm() {
        let s = schema("$schema:\n  choice:\n    - boolean\n    - string(suggest(first, second))\n");
        let q = suggestions_for_path(&s, &["choice"]).unwrap();
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "first");
        assert_eq!(q.items[1].label, "second");
    }

    #[test]
    fn root_union_uses_first_suggestion_arm() {
        let s = schema("$schema:\n  - root: string(suggest(arm-one))\n  - root: string(suggest(arm-two))\n");
        let q = suggestions_for_path(&s, &["root"]).unwrap();
        assert_eq!(q.items.len(), 1);
        assert_eq!(q.items[0].label, "arm-one");
    }

    #[test]
    fn root_union_filters_invalid_candidate_from_later_arm() {
        let s = schema("$schema:\n  - value: string\n  - value: number(suggest(1, many, 2))\n");
        let q = suggestions_for_path(&s, &["value"]).unwrap();
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "1");
        assert_eq!(q.items[1].label, "2");
    }

    #[test]
    fn root_union_finds_nested_property_in_later_arm() {
        let s = schema(
            "$schema:\n  - settings: string\n  - settings: \"{ mode: string(suggest(fast, slow)) }\"\n",
        );
        let q = suggestions_for_path(&s, &["settings", "mode"]).unwrap();
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "fast");
        assert_eq!(q.items[1].label, "slow");
    }

    #[test]
    fn root_union_continues_when_earlier_arm_lacks_ancestor() {
        let s = schema(
            "$schema:\n  - other: string\n  - settings: \"{ mode: string(suggest(fast, slow)) }\"\n",
        );
        let q = suggestions_for_path(&s, &["settings", "mode"]).unwrap();
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "fast");
        assert_eq!(q.items[1].label, "slow");
    }

    #[test]
    fn prefix_filter_matches_label() {
        let s = schema("$schema:\n  color: string(suggest(red, green, blue))\n");
        let q = suggestions_for_path(&s, &["color"]).unwrap();
        let filtered: Vec<&SuggestionItem> =
            q.items.iter().filter(|item| matches_prefix(item, "gr")).collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].label, "green");
    }

    #[test]
    fn prefix_filter_uses_decoded_not_canonical() {
        let s = schema("$schema:\n  v: number(suggest(003.5))\n");
        let q = suggestions_for_path(&s, &["v"]).unwrap();
        assert_eq!(q.items.len(), 1);
        assert_eq!(q.items[0].decoded, "003.5");
        assert_eq!(q.items[0].label, "3.5");
        assert_eq!(q.items[0].insert_text, "3.5");
        assert!(matches_prefix(&q.items[0], "00"));
        assert!(!matches_prefix(&q.items[0], "3"));
    }

    #[test]
    fn prefix_filter_trailing_fractional_zero() {
        let s = schema("$schema:\n  v: number(suggest(3.0, 3.50))\n");
        let q = suggestions_for_path(&s, &["v"]).unwrap();
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].decoded, "3.0");
        assert_eq!(q.items[0].label, "3");
        assert_eq!(q.items[1].decoded, "3.50");
        assert_eq!(q.items[1].label, "3.5");
        assert!(matches_prefix(&q.items[0], "3."));
        assert!(matches_prefix(&q.items[1], "3."));
        assert!(!matches_prefix(&q.items[0], "3.5"));
        assert!(matches_prefix(&q.items[1], "3.5"));
    }

    #[test]
    fn empty_prefix_matches_all() {
        let s = schema("$schema:\n  color: string(suggest(red, green, blue))\n");
        let q = suggestions_for_path(&s, &["color"]).unwrap();
        let count = q.items.iter().filter(|item| matches_prefix(item, "")).count();
        assert_eq!(count, 3);
    }

    #[test]
    fn yaml_double_quote_escapes_special_chars() {
        assert_eq!(yaml_double_quote("simple"), "\"simple\"");
        assert_eq!(yaml_double_quote("with \"quotes\""), "\"with \\\"quotes\\\"\"");
        assert_eq!(yaml_double_quote("back\\slash"), "\"back\\\\slash\"");
        assert_eq!(yaml_double_quote("line\nbreak"), "\"line\\nbreak\"");
    }

    #[test]
    fn negative_number_uses_canonical_decimal() {
        let s = schema("$schema:\n  score: number(min(-100); max(100); suggest(-1, 50, 101))\n");
        let q = suggestions_for_path(&s, &["score"]).unwrap();
        assert_eq!(q.items[0].label, "-1");
        assert_eq!(q.items[0].insert_text, "-1");
    }

    #[test]
    fn number_integer_constraint_suggestions() {
        let s = schema("$schema:\n  port: number(integer; suggest(80, 443))\n");
        let q = suggestions_for_path(&s, &["port"]).unwrap();
        assert_eq!(q.items.len(), 2);
        assert_eq!(q.items[0].label, "80");
        assert_eq!(q.items[1].label, "443");
    }

    #[test]
    fn string_with_spaces_is_quoted() {
        let s = schema("$schema:\n  v: string(suggest(\"blue gray\", plain))\n");
        let q = suggestions_for_path(&s, &["v"]).unwrap();
        assert_eq!(q.items[0].label, "blue gray");
        assert_eq!(q.items[0].insert_text, "\"blue gray\"");
    }

    /// An unresolved bare reference is the same AST as `Name@this`, so a
    /// suggestion query answers identically for both spellings.
    #[test]
    fn bare_reference_suggestions_match_explicit_local_reference() {
        let s = schema(concat!(
            "$schema:\n",
            "  bare: code(suggest(a, 2))\n",
            "  explicit: code(suggest(a, 2))@this\n",
            "  code: string\n",
        ));
        let bare = suggestions_for_def("v", def(&s, "bare")).expect("bare suggestions");
        let explicit =
            suggestions_for_def("v", def(&s, "explicit")).expect("explicit suggestions");
        assert_eq!(bare, explicit);
        let labels: Vec<&str> = bare.items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, ["a", "2"]);
    }

    fn reference(name: &str, is_array: bool) -> PropertyAtom {
        PropertyAtom {
            ty: TypeExpr::Ref(DefKey::new(super::super::DefNamespace::InlineRoot, name)),
            is_array,
            ..PropertyAtom::bare(super::super::SimplifiedType::String)
        }
    }

    fn table(entries: &[(&str, PropertyAtom)]) -> DefinitionTable {
        let mut table = DefinitionTable::default();
        for (name, body) in entries {
            let origin = super::super::SchemaReferenceSite {
                source: super::super::SchemaSourceId::InlineDocument,
                path: super::super::SchemaSourcePath::root().property(*name),
            };
            let key = DefKey::new(super::super::DefNamespace::InlineRoot, *name);
            table.insert(key, PropertyDef::Single(body.clone()), origin);
        }
        table
    }

    /// Resolution rejects alias loops, but a lookup that holds one must end
    /// with the reference unopened rather than recursing forever.
    #[test]
    fn resolved_alternatives_stops_at_an_alias_loop() {
        let definitions = table(&[("a", reference("b", false)), ("b", reference("a", false))]);
        let use_site = PropertyDef::Single(reference("a", false));
        let alternatives = resolved_alternatives(&use_site, &definitions);
        assert_eq!(alternatives.len(), 1);
        assert!(matches!(&alternatives[0].atom.ty, TypeExpr::Ref(key) if key.name == "a"));
    }

    /// An element of `list[]` where `list` is itself an array has no leaf
    /// alternative at the element site, so the reference stays unopened.
    #[test]
    fn resolved_alternatives_keeps_an_array_of_an_array_definition_unopened() {
        let object = PropertyAtom::bare_inline_object(SchemaShape::default());
        let definitions = table(&[("list", PropertyAtom { is_array: true, ..object })]);
        let as_value = PropertyDef::Single(reference("list", false));
        let value = resolved_alternatives(&as_value, &definitions);
        assert!(value.len() == 1 && value[0].is_array && matches!(value[0].atom.ty, TypeExpr::InlineObject(_)));

        let as_items = PropertyDef::Single(reference("list", true));
        let items = resolved_alternatives(&as_items, &definitions);
        assert!(items.len() == 1 && items[0].is_array && matches!(items[0].atom.ty, TypeExpr::Ref(_)));
    }
}
