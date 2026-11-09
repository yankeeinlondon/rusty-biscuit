//! Union-aware completion over a resolved SimplifiedSchema: which keys and
//! scalar values to offer inside one incomplete value, such as an element
//! of `U[]` ([`alternatives_for_def_in`]), at any depth of a cursor path
//! ([`for_path_in`], [`PathStep`]) through inline objects, array elements,
//! and recursive references.
//!
//! ## Recursive references
//!
//! A recursive named type, or a shared acyclic one, is a [`TypeExpr::Ref`]
//! into a definition table.
//! Every entry point here opens a reference only where the position it answers
//! is one, through a [`DefinitionLookup`], so descending
//! `tree.children[0].children[0]` opens `item` once per level and never the
//! definitions the path does not reach. A reference whose definition is
//! missing offers nothing.
//!
//! All entry points are pure functions over already-resolved schema state —
//! no I/O, no re-parsing.

use std::borrow::Cow;
use std::sync::{Arc, LazyLock};

use serde_json::Value;

use super::discriminant::select_literal_discriminant_arm;
pub use super::simplified::query::DefinitionLookup;
use super::simplified::query::{
    SuggestionItem, reachable_definitions, resolve_with_bodies, resolved_alternatives,
    suggestions_by_resolved_alternative, yaml_double_quote,
};
use super::simplified::{
    Constraint, DefinitionTable, DynamicElement, DynamicKind, PropertyAtom, PropertyDef, SchemaArm, SchemaShape,
    SchemaRoot, TypeExpr, to_json_schema_with_definitions,
};
use super::validate::ValidatorCache;

static ALTERNATIVE_VALIDATOR_CACHE: LazyLock<ValidatorCache> = LazyLock::new(ValidatorCache::new);

/// Empties this module's process-wide caches (see
/// [`super::reset_process_caches_for_bench`]).
pub(crate) fn reset_process_caches() {
    ALTERNATIVE_VALIDATOR_CACHE.clear();
}

/// Where an incomplete value sits relative to its property, for
/// [`alternatives_for_def`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlternativeSite {
    /// One element of an array property (`U[]`, `T[]`, or a union of array
    /// arms): the alternatives are the element's.
    Item,
    /// The property's own value: the alternatives are the non-array atoms of
    /// the property (a property-level union, or a `U` / `U(required)`
    /// reference).
    Value,
}

/// The keys and values an editor may offer for one incomplete union value,
/// combined across every applicable alternative, or narrowed to one when an
/// authored literal tag selects it.
///
/// Advisory only: nothing here validates, coerces, or selects an arm by
/// closeness, and keys the value already carries are not removed — the caller
/// filters by what is authored and by the typed prefix.
#[derive(Debug, Clone, PartialEq)]
pub struct AlternativeCompletion {
    /// `true` for an [`AlternativeSite::Item`] query: completion inserts one
    /// element, never a whole array.
    pub is_array: bool,
    /// The alternative an unambiguous literal tag on an object value selected
    /// (an index into the site's alternatives, in declaration order with
    /// nested unions flattened). `None` when the choices are combined.
    pub selected_arm: Option<usize>,
    /// Keys of the applicable inline-object alternatives, deduplicated in
    /// alternative-then-declaration order.
    pub keys: Vec<AlternativeKey>,
    /// Values of the applicable scalar alternatives — `enum` members,
    /// `literal(...)` values, and lint-valid `suggest(...)` candidates —
    /// deduplicated by insertion text in alternative order. In a template a
    /// list's literal elements are offered and its slots are not.
    pub values: Vec<AlternativeValue>,
    /// Whether an applicable scalar alternative holds an unresolved
    /// `{{ … }}` slot among the values it offers, so `values` is not every
    /// value there is.
    pub pending: bool,
}

/// One key offered by an inline-object alternative.
#[derive(Debug, Clone, PartialEq)]
pub struct AlternativeKey {
    /// Mapping key.
    pub name: String,
    /// `true` when every alternative offering the key requires it.
    pub required: bool,
    /// The description of the key's first declaration.
    pub description: Option<String>,
    /// The alternatives that declare the key.
    pub arms: Vec<usize>,
}

/// One scalar value offered by a scalar alternative.
#[derive(Debug, Clone, PartialEq)]
pub struct AlternativeValue {
    /// Display label.
    pub label: String,
    /// YAML-safe insertion text: double-quoted for strings.
    pub insert_text: String,
    /// The first alternative offering the value.
    pub arm: usize,
}

/// One step of a cursor path from the frontmatter root, as [`for_path_in`] and
/// [`def_at_path`] read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStep<'a> {
    /// A mapping key.
    Key(&'a str),
    /// An element of an array. The position only locates the authored element
    /// that narrows a discriminated union; an element not authored yet (an
    /// index past the end) has the same type as any other.
    Index(usize),
}

/// The completion alternatives at `path` in `schema`, resolving recursive
/// references through `definitions`.
///
/// ## Notes
///
/// Lazy: a definition is looked up only where the path stands on a reference
/// (or the completed value is one), once per reference at that position, so
/// `item[]` reached through `n` levels of `children` costs `n + 1` lookups and
/// a definition the path does not reach is never opened. A lookup that
/// returns `None` ends the descent with no completion there.
pub fn for_path_in<L>(
    schema: &SchemaRoot,
    definitions: &L,
    frontmatter: &Value,
    path: &[PathStep<'_>],
) -> Option<AlternativeCompletion>
where
    L: DefinitionLookup + ?Sized,
{
    let (def, site) = def_at_path(schema, definitions, frontmatter, path)?;
    let value = instance_at(frontmatter, path).unwrap_or(&Value::Null);
    alternatives_for_def_in(&def, site, value, definitions)
}

/// The definition governing the value at `path` and where that value sits in
/// it: [`AlternativeSite::Value`] for a path ending on a key, the definition's
/// property; [`AlternativeSite::Item`] for a path ending on an index, the
/// definition of the array that holds the element.
///
/// The returned definition may itself be a reference (an element of `item[]`
/// is the `Ref` of `item`); pass it with `definitions` to
/// [`alternatives_for_def_in`] or [`resolved_alternatives`] to open it. Narrowing,
/// laziness, and `None` are as for [`for_path_in`]. A key
/// that several unnarrowed object arms declare resolves to the union of their
/// atoms.
///
/// [`resolved_alternatives`]: super::simplified::query::resolved_alternatives
pub fn def_at_path<'a, L>(
    schema: &'a SchemaRoot,
    definitions: &'a L,
    frontmatter: &Value,
    path: &[PathStep<'_>],
) -> Option<(Cow<'a, PropertyDef>, AlternativeSite)>
where
    L: DefinitionLookup + ?Sized,
{
    let (PathStep::Key(first), rest) = path.split_first()? else {
        return None;
    };
    let mut def = root_property_def(schema, first)?;
    let mut site = AlternativeSite::Value;
    for (offset, step) in rest.iter().enumerate() {
        match step {
            PathStep::Index(_) if site == AlternativeSite::Value => site = AlternativeSite::Item,
            // An element of an element has no leaf alternative to descend.
            PathStep::Index(_) => return None,
            PathStep::Key(key) => {
                let parent = instance_at(frontmatter, &path[..=offset]).unwrap_or(&Value::Null);
                def = match def {
                    Cow::Borrowed(def) => property_in(def, site, key, parent, definitions)?,
                    Cow::Owned(def) => Cow::Owned(property_in(&def, site, key, parent, definitions)?.into_owned()),
                };
                site = AlternativeSite::Value;
            }
        }
    }
    Some((def, site))
}

/// The definition of `key` in the object alternatives of `def` at `site`,
/// narrowed by the authored `parent` object's literal tag.
fn property_in<'a, L>(
    def: &'a PropertyDef,
    site: AlternativeSite,
    key: &str,
    parent: &Value,
    definitions: &'a L,
) -> Option<Cow<'a, PropertyDef>>
where
    L: DefinitionLookup + ?Sized,
{
    let is_array = site == AlternativeSite::Item;
    let shapes: Vec<&SchemaShape> = resolved_alternatives(def, definitions)
        .into_iter()
        .filter(|alternative| alternative.is_array == is_array)
        .filter_map(|alternative| match &alternative.atom.ty {
            TypeExpr::InlineObject(shape) => Some(shape),
            _ => None,
        })
        .collect();
    let selected = parent.is_object().then(|| {
        let projections: Vec<Value> = shapes.iter().map(|shape| shape_discriminant_json(shape)).collect();
        select_literal_discriminant_arm(&projections, parent)
    });
    let defs: Vec<&PropertyDef> = match selected.flatten() {
        Some(index) => shapes[index].properties.get(key).into_iter().collect(),
        None => shapes.iter().filter_map(|shape| shape.properties.get(key)).collect(),
    };
    match defs.as_slice() {
        [] => None,
        [only] => Some(Cow::Borrowed(*only)),
        _ => union_of(defs).map(Cow::Owned),
    }
}

/// The authored value at `path` in `frontmatter`, if it is there.
fn instance_at<'v>(frontmatter: &'v Value, path: &[PathStep<'_>]) -> Option<&'v Value> {
    path.iter().try_fold(frontmatter, |value, step| match step {
        PathStep::Key(key) => value.get(*key),
        PathStep::Index(index) => value.get(*index),
    })
}

/// Whether `value` is a valid instance of one leaf alternative (an
/// [`Alternative::atom`](crate::simplified::Alternative::atom)), lowered on its own as a present value: the
/// optional-property null wrapper is never added, so `null` matches only an
/// alternative that itself accepts `null`, as inside an item union.
///
/// Editors use it to name the alternative a value matches only when exactly
/// one accepts it. It is structural and passive: a `file` alternative is
/// judged by syntax, never by probing the filesystem, so it must not authorize
/// a transformation.
///
/// An alternative that is or contains a recursive reference has no
/// definition in scope here and accepts nothing; use [`alternative_accepts_in`].
pub fn alternative_accepts(atom: &PropertyAtom, value: &Value) -> bool {
    alternative_accepts_in(atom, value, &DefinitionTable::default())
}

/// [`alternative_accepts`] with recursive references resolved through
/// `definitions`: the alternative is lowered with every definition it reaches,
/// so a value is judged against the recursive type at any depth. `false` when
/// a reached definition is missing.
pub fn alternative_accepts_in<L>(atom: &PropertyAtom, value: &Value, definitions: &L) -> bool
where
    L: DefinitionLookup + ?Sized,
{
    AlternativeMatcher::prepare(atom, definitions).accepts(value)
}

/// [`alternative_accepts_in`] for one alternative, lowered and compiled once
/// so it can judge many values. Cloning shares the compiled validator.
#[derive(Clone)]
pub struct AlternativeMatcher {
    /// `None` when the alternative reaches a missing definition or does not
    /// lower to a valid schema: it accepts nothing.
    validator: Option<Arc<jsonschema::Validator>>,
}

impl AlternativeMatcher {
    /// Lowers `atom` as a present value, with every definition it reaches
    /// through `definitions`, and fetches or compiles its structural
    /// validator.
    pub fn prepare<L>(atom: &PropertyAtom, definitions: &L) -> Self
    where
        L: DefinitionLookup + ?Sized,
    {
        crate::note_work!(AlternativeMatcherPrepare);
        let validator = reachable_definitions(atom, definitions).and_then(|table| {
            let mut present = atom.clone();
            present.constraints.push(Constraint::Required);
            let mut shape = SchemaShape::default();
            shape
                .properties
                .insert("value".to_string(), PropertyDef::Single(present));
            let schema = to_json_schema_with_definitions(&SchemaRoot::Single(shape), &table).ok()?;
            ALTERNATIVE_VALIDATOR_CACHE.structural_validator_for(&schema).ok()
        });
        Self { validator }
    }

    /// Whether `value` is a valid instance of the prepared alternative.
    pub fn accepts(&self, value: &Value) -> bool {
        self.validator
            .as_ref()
            .is_some_and(|validator| validator.is_valid(&serde_json::json!({ "value": value })))
    }
}

/// A content fingerprint of `atom` together with every definition it reaches
/// through `definitions`: two alternatives with equal fingerprints prepare
/// the same [`AlternativeMatcher`], so a caller can key prepared matchers on
/// it across schema assemblies. Taking one builds no schema and allocates
/// only the traversal's reference keys. `None` when a reached definition is
/// missing (the alternative accepts nothing). Stable within one process only.
pub fn alternative_fingerprint<L>(atom: &PropertyAtom, definitions: &L) -> Option<u64>
where
    L: DefinitionLookup + ?Sized,
{
    super::simplified::query::reachable_fingerprint(atom, definitions)
}

/// [`alternatives_for_def_in`] with no definition in scope: a recursive
/// reference offers nothing.
pub fn alternatives_for_def(
    def: &PropertyDef,
    site: AlternativeSite,
    value: &Value,
) -> Option<AlternativeCompletion> {
    alternatives_for_def_in(def, site, value, &DefinitionTable::default())
}

/// Completion for one incomplete value of `def` at `site`, given the value as
/// authored so far, with recursive references opened through `definitions`.
///
/// The alternatives are `def`'s, with a reference replaced by its
/// definition's (one level: a reference nested in a property of the result is
/// not opened). They follow the authored value's JSON kind, which no
/// alternative of another kind can ever accept: `null` keeps every
/// alternative; an object keeps the inline-object alternatives and narrows to
/// one when [`select_literal_discriminant_arm`] picks it; a string, number, or
/// boolean keeps the scalar alternatives; an array keeps none. A missing,
/// unknown, duplicate, or conflicting tag keeps the combined choices.
///
/// Returns `None` when `def` has no alternative at `site`.
///
/// ## Notes
///
/// Passive: reads the resolved definition only, with no I/O or expression
/// evaluation.
pub fn alternatives_for_def_in<L>(
    def: &PropertyDef,
    site: AlternativeSite,
    value: &Value,
    definitions: &L,
) -> Option<AlternativeCompletion>
where
    L: DefinitionLookup + ?Sized,
{
    let is_array = site == AlternativeSite::Item;
    let (resolved, bodies) = resolve_with_bodies(def, definitions);
    let alternatives: Vec<&PropertyAtom> = resolved
        .into_iter()
        .filter(|alternative| alternative.is_array == is_array)
        .map(|alternative| alternative.atom)
        .collect();
    if alternatives.is_empty() {
        return None;
    }

    let selected_arm = value.is_object().then(|| {
        let projections: Vec<Value> = alternatives.iter().map(|atom| discriminant_json(atom)).collect();
        select_literal_discriminant_arm(&projections, value)
    });
    let selected_arm = selected_arm.flatten();

    // Candidates are judged against the whole definition, so a list a union
    // stores on one arm keeps the entries its sibling arms accept.
    let suggestions = suggestions_by_resolved_alternative("item", def, &bodies);
    let mut completion = AlternativeCompletion {
        is_array,
        selected_arm,
        keys: Vec::new(),
        values: Vec::new(),
        pending: false,
    };
    for (arm, atom) in alternatives.iter().enumerate() {
        if selected_arm.is_some_and(|selected| selected != arm) || !applies_to(atom, value) {
            continue;
        }
        match &atom.ty {
            TypeExpr::InlineObject(shape) => add_keys(&mut completion.keys, shape, arm),
            TypeExpr::Primitive(_) => {
                let suggested = suggestions
                    .iter()
                    .find(|(alternative, _)| std::ptr::eq(alternative.atom, *atom))
                    .map_or(&[][..], |(_, items)| items.as_slice());
                add_values(&mut completion.values, atom, arm, suggested);
                completion.pending |= atom.has_pending_values();
            }
            // A template type's shape is pending: it offers nothing yet and
            // says so, rather than looking like a type with no values.
            TypeExpr::Slot(_) => completion.pending = true,
            // An unresolved import has no shape to offer; flattening never
            // yields a union; a reference left here could not be opened.
            TypeExpr::Imported { .. }
            | TypeExpr::Union(_)
            | TypeExpr::Tuple(_)
            | TypeExpr::Function(_)
            | TypeExpr::Ref(_) => {}
        }
    }
    Some(completion)
}

/// `property`'s top-level definition; across a root union, every declaring
/// inline arm's atoms as one property-level union.
pub fn root_property_def<'a>(schema: &'a SchemaRoot, property: &str) -> Option<Cow<'a, PropertyDef>> {
    match schema {
        SchemaRoot::Single(shape) => shape.properties.get(property).map(Cow::Borrowed),
        SchemaRoot::Union(arms) => union_of(
            arms.iter()
                .filter_map(|arm| match arm {
                    SchemaArm::Inline(shape) => shape.properties.get(property),
                    SchemaArm::FileRef(_) => None,
                })
                .collect(),
        )
        .map(Cow::Owned),
    }
}

/// The atoms of `defs` as one definition, without duplicates: `None` for no
/// atom, a single atom alone, otherwise a property-level union.
fn union_of(defs: Vec<&PropertyDef>) -> Option<PropertyDef> {
    let mut atoms: Vec<PropertyAtom> = Vec::new();
    for def in defs {
        let declared = match def {
            PropertyDef::Single(atom) => std::slice::from_ref(atom),
            PropertyDef::Union(atoms) => atoms.as_slice(),
        };
        for atom in declared {
            if !atoms.contains(atom) {
                atoms.push(atom.clone());
            }
        }
    }
    match atoms.len() {
        0 => None,
        1 => atoms.pop().map(PropertyDef::Single),
        _ => Some(PropertyDef::Union(atoms)),
    }
}

/// Whether an alternative can accept a value of the authored JSON kind.
fn applies_to(atom: &PropertyAtom, value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Object(_) => matches!(atom.ty, TypeExpr::InlineObject(_)),
        Value::String(_) | Value::Number(_) | Value::Bool(_) => matches!(atom.ty, TypeExpr::Primitive(_)),
        Value::Array(_) => false,
    }
}

/// The minimal JSON [`select_literal_discriminant_arm`] reads: an inline
/// object's `literal(...)` keys as consts. A scalar alternative projects to
/// `{}` so indexes stay aligned. A recursive reference was already opened to
/// its definition's object, so the unscoped selector needs no `$defs`.
fn discriminant_json(atom: &PropertyAtom) -> Value {
    match &atom.ty {
        TypeExpr::InlineObject(shape) => shape_discriminant_json(shape),
        _ => serde_json::json!({}),
    }
}

fn shape_discriminant_json(shape: &SchemaShape) -> Value {
    let mut properties = serde_json::Map::new();
    for (key, def) in &shape.properties {
        if let PropertyDef::Single(child) = def
            && let Some(value) = child.literal_value()
        {
            properties.insert(key.clone(), serde_json::json!({ "const": value }));
        }
    }
    serde_json::json!({ "properties": properties })
}

fn add_keys(keys: &mut Vec<AlternativeKey>, shape: &SchemaShape, arm: usize) {
    for (name, def) in &shape.properties {
        let required = def.is_required();
        match keys.iter_mut().find(|key| key.name == *name) {
            Some(key) => {
                key.required &= required;
                key.arms.push(arm);
            }
            None => keys.push(AlternativeKey {
                name: name.clone(),
                required,
                description: def_description(def),
                arms: vec![arm],
            }),
        }
    }
}

fn def_description(def: &PropertyDef) -> Option<String> {
    match def {
        PropertyDef::Single(atom) => atom.description.clone(),
        PropertyDef::Union(atoms) => atoms.iter().find_map(|atom| atom.description.clone()),
    }
}

fn add_values(
    values: &mut Vec<AlternativeValue>,
    atom: &PropertyAtom,
    arm: usize,
    suggested: &[SuggestionItem],
) {
    let mut push = |label: String, insert_text: String| {
        if !values.iter().any(|value| value.insert_text == insert_text) {
            values.push(AlternativeValue {
                label,
                insert_text,
                arm,
            });
        }
    };
    if let Some(literal) = atom.literal_value() {
        match literal {
            Value::String(text) => push(text.clone(), yaml_double_quote(text)),
            other => push(other.to_string(), other.to_string()),
        }
    }
    for constraint in &atom.constraints {
        match constraint {
            Constraint::Members(members) => {
                for member in members {
                    push(member.clone(), yaml_double_quote(member));
                }
            }
            Constraint::Dynamic(list) if list.kind == DynamicKind::Members => {
                for element in &list.elements {
                    if let DynamicElement::Member(member) = element {
                        push(member.clone(), yaml_double_quote(member));
                    }
                }
            }
            _ => {}
        }
    }
    for item in suggested {
        push(item.label.clone(), item.insert_text.clone());
    }
}
