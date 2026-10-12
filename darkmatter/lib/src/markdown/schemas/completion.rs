//! Schema-aware completion hints for external consumers (e.g. shell-completion
//! generators).
//!
//! This module is a **read-only consumer** of an [`EffectiveSchema`]: it
//! inspects the SimplifiedSchema projection (when available) and yields
//! completion data for a property, without mutating or re-resolving the
//! schema. See the *Shell Completions Integration* section of the spec.
//!
//! [`for_list_item`], [`for_property_value`], and [`alternatives_for_def`]
//! answer a second question for editors: which keys and scalar values to
//! offer inside one incomplete union value, such as an element of `U[]`.
//!
//! Three property categories are surfaced by [`for_property`]:
//!
//! - [`CompletionKind::File`] — filesystem paths filtered by the property's
//!   `match(...)` groups. The caller (shell-completion generator) walks the
//!   filesystem; this module only returns the groups.
//! - [`CompletionKind::Enum`] — the enumerated members of an `enum` property.
//! - [`CompletionKind::Hint`] — `url` / `email` / date-family properties, where
//!   completion is a one-line format hint (no values).
//!
//! All entry points are pure functions over already-resolved schema state —
//! no I/O, no re-parsing.
//!
//! ## Example
//!
//! ```ignore
//! use darkmatter::markdown::schemas::{DarkmatterSchemas, completion};
//!
//! let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()));
//! if let Some(effective) = api.effective_for(&md)? {
//!     for property in completion::completable_properties(&effective) {
//!         let suggestion = completion::for_property(&effective, &property);
//!         // dispatch to shell-completion backend...
//!     }
//! }
//! ```

use std::borrow::Cow;
use std::sync::LazyLock;

use serde_json::Value;

use super::EffectiveSchema;
use super::discriminant::select_literal_discriminant_arm;
use super::simplified::query::{Alternative, flattened_alternatives, yaml_double_quote};
use super::simplified::{
    Constraint, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SimplifiedSchema, SimplifiedType,
    TypeExpr, suggestions_for_def, to_json_schema,
};
use super::validate::ValidatorCache;

static ALTERNATIVE_VALIDATOR_CACHE: LazyLock<ValidatorCache> = LazyLock::new(ValidatorCache::new);

/// Completion data derived from a single property.
#[derive(Debug, Clone, PartialEq)]
pub struct CompletionSuggestion {
    /// Property name (key in frontmatter).
    pub property: String,
    /// `true` when the property is declared with `[]`, i.e. the value is an
    /// array of completable atoms. An array of a union (`U[]`) counts, with
    /// the completable arm describing one element.
    pub is_array: bool,
    /// Optional description from `-> ...` syntax.
    pub description: Option<String>,
    /// `true` when the property value is supplied by the host runtime rather
    /// than authored in static frontmatter.
    pub generated: bool,
    /// The completion category and payload.
    pub kind: CompletionKind,
}

/// The category of completion produced for a property.
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    /// Filesystem-path completion.
    ///
    /// `alternatives` is an OR of AND-ed groups: a path is a candidate when
    /// it satisfies **every** group of **some** alternative, each group judged
    /// as a [`FileMatchGlobs`](super::file_match::FileMatchGlobs) over the raw glob
    /// strings of one `match(...)` (including `!`-prefixed exclusions). A
    /// single property yields one alternative holding each of its groups (a
    /// reused file type adds the use site's groups to the inherited ones); a
    /// root union yields one alternative per contributing arm. Empty when no
    /// contributing atom has a `match` constraint (a bare `file`).
    File { alternatives: Vec<Vec<Vec<String>>> },
    /// Enum-member completion. The members come from the constraint payload
    /// in declaration order.
    Enum { members: Vec<String> },
    /// One-line format hint for value-completion-unfriendly types
    /// (`url`, `email`, `date`, `datetime`, `time`).
    Hint { format: &'static str },
}

/// Returns the completion suggestion for `property` if its declared type is
/// in the completable set, otherwise `None`.
///
/// For a property-level union, the first atom whose type is completable wins —
/// the spec's intent of "treat the property as that type for the purpose of
/// completion" without merging competing arms. A resolved union-typed
/// reference (`U`, `U(required)`, `U[]`) is read through its arms the same
/// way.
///
/// For a **root-level union**, `property` is gathered from every inline arm
/// that declares it. When every contributing arm is a `file`, each arm's
/// `match(...)` groups become one alternative, so the union offers the union
/// of its arms' candidates; otherwise the first completable arm wins. See
/// [`suggestion_from_union`].
pub fn for_property(effective: &EffectiveSchema, property: &str) -> Option<CompletionSuggestion> {
    match effective.simplified.as_ref()? {
        SimplifiedSchema::Single(shape) => {
            let def = shape.properties.get(property)?;
            suggestion_from_def(property, def)
        }
        SimplifiedSchema::Union(arms) => suggestion_from_union(property, arms),
    }
}

/// Returns the list of properties in `effective` that have a completable
/// type. Order matches the SimplifiedSchema's declaration order.
///
/// For a root-level union, the union of every inline arm's completable
/// properties is returned, deduplicated in first-seen order. Returns an empty
/// vector when the effective schema was supplied as raw JSON Schema (no
/// SimplifiedSchema projection is available).
pub fn completable_properties(effective: &EffectiveSchema) -> Vec<String> {
    let Some(simplified) = effective.simplified.as_ref() else {
        return Vec::new();
    };
    match simplified {
        SimplifiedSchema::Single(shape) => completable_in_shape(shape),
        SimplifiedSchema::Union(arms) => {
            let mut out: Vec<String> = Vec::new();
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    for name in completable_in_shape(shape) {
                        if !out.contains(&name) {
                            out.push(name);
                        }
                    }
                }
            }
            out
        }
    }
}

fn completable_in_shape(shape: &SchemaShape) -> Vec<String> {
    shape
        .properties
        .iter()
        .filter_map(|(name, def)| first_completable_atom(def).map(|_| name.clone()))
        .collect()
}

/// Builds a completion suggestion for `property` across the inline arms of a
/// root-level union.
///
/// Each arm that declares `property` contributes its first completable atom.
/// When every contributing atom is a `file`, each atom's groups become one
/// alternative (deduplicated, first-seen order) so a union of file-typed arms
/// offers the union of their candidates instead of only the first arm's.
/// Arms are never concatenated into one group: that would let one arm's
/// exclusions and conjunctions leak into another's.
/// Otherwise the first contributing atom wins, matching the single-shape
/// "first completable arm" rule. Unresolved [`SchemaArm::FileRef`] arms are
/// skipped.
fn suggestion_from_union(property: &str, arms: &[SchemaArm]) -> Option<CompletionSuggestion> {
    let alternatives: Vec<Alternative<'_>> = arms
        .iter()
        .filter_map(|arm| match arm {
            SchemaArm::Inline(shape) => shape.properties.get(property),
            SchemaArm::FileRef(_) => None,
        })
        .filter_map(first_completable_atom)
        .collect();

    let first = *alternatives.first()?;
    let all_file = alternatives
        .iter()
        .all(|alternative| matches!(&alternative.atom.ty, TypeExpr::Primitive(SimplifiedType::File)));
    if all_file {
        let mut file_alternatives: Vec<Vec<Vec<String>>> = Vec::new();
        for alternative in &alternatives {
            let groups = match_groups(alternative.atom);
            if !groups.is_empty() && !file_alternatives.contains(&groups) {
                file_alternatives.push(groups);
            }
        }
        return Some(CompletionSuggestion {
            property: property.to_string(),
            is_array: alternatives.iter().any(|alternative| alternative.is_array),
            description: first.atom.description.clone(),
            generated: alternatives
                .iter()
                .all(|alternative| atom_is_generated(alternative.atom)),
            kind: CompletionKind::File {
                alternatives: file_alternatives,
            },
        });
    }

    Some(CompletionSuggestion {
        property: property.to_string(),
        is_array: first.is_array,
        description: first.atom.description.clone(),
        generated: atom_is_generated(first.atom),
        kind: kind_for_atom(first.atom)?,
    })
}

fn suggestion_from_def(property: &str, def: &PropertyDef) -> Option<CompletionSuggestion> {
    let alternative = first_completable_atom(def)?;
    let atom = alternative.atom;
    let kind = kind_for_atom(atom)?;
    Some(CompletionSuggestion {
        property: property.to_string(),
        is_array: alternative.is_array,
        description: atom.description.clone(),
        generated: atom_is_generated(atom),
        kind,
    })
}

/// The first completable alternative of `def`, looking through a resolved
/// union-typed reference so `U(required)` and `U[]` complete like the
/// property-level union `U` expands to.
fn first_completable_atom(def: &PropertyDef) -> Option<Alternative<'_>> {
    flattened_alternatives(def)
        .into_iter()
        .find(|alternative| is_completable(&alternative.atom.ty))
}

fn atom_is_generated(atom: &PropertyAtom) -> bool {
    atom.constraints
        .iter()
        .chain(atom.array_constraints.iter())
        .any(|constraint| matches!(constraint, Constraint::Generated))
}

fn is_completable(ty: &TypeExpr) -> bool {
    match ty {
        TypeExpr::Primitive(p) => matches!(
            *p,
            SimplifiedType::File
                | SimplifiedType::Enum
                | SimplifiedType::Url
                | SimplifiedType::Email
                | SimplifiedType::Date
                | SimplifiedType::DateTime
                | SimplifiedType::Time
        ),
        // Inline objects don't have a single completable value at the
        // property level — they describe a typed shape. Imported types are
        // resolved before completion runs; an unresolved one is not
        // completable here. A union never reaches here: callers read its
        // arms through `flattened_alternatives`.
        TypeExpr::InlineObject(_) | TypeExpr::Imported { .. } | TypeExpr::Union(_) => false,
    }
}

/// Every non-empty `match(...)` group on `atom`, in authored order.
fn match_groups(atom: &PropertyAtom) -> Vec<Vec<String>> {
    atom.constraints
        .iter()
        .filter_map(|constraint| match constraint {
            Constraint::Match(patterns) if !patterns.is_empty() => Some(patterns.clone()),
            _ => None,
        })
        .collect()
}

fn kind_for_atom(atom: &PropertyAtom) -> Option<CompletionKind> {
    let ty = match &atom.ty {
        TypeExpr::Primitive(ty) => *ty,
        TypeExpr::InlineObject(_) | TypeExpr::Imported { .. } | TypeExpr::Union(_) => return None,
    };
    match ty {
        SimplifiedType::File => {
            let groups = match_groups(atom);
            Some(CompletionKind::File {
                alternatives: if groups.is_empty() { Vec::new() } else { vec![groups] },
            })
        }
        SimplifiedType::Enum => Some(CompletionKind::Enum {
            members: atom
                .constraints
                .iter()
                .find_map(|c| match c {
                    Constraint::Members(members) => Some(members.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
        }),
        SimplifiedType::Url => Some(CompletionKind::Hint {
            format: "URL (e.g. https://example.com/...)",
        }),
        SimplifiedType::Email => Some(CompletionKind::Hint {
            format: "email address (e.g. name@example.com)",
        }),
        SimplifiedType::Date => Some(CompletionKind::Hint {
            format: "ISO-8601 date (YYYY-MM-DD)",
        }),
        SimplifiedType::DateTime => Some(CompletionKind::Hint {
            format: "ISO-8601 datetime (e.g. 2026-05-11T12:00:00Z)",
        }),
        SimplifiedType::Time => Some(CompletionKind::Hint {
            format: "time-of-day (HH:MM[:SS][±HH:MM])",
        }),
        _ => None,
    }
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
    /// deduplicated by insertion text in alternative order.
    pub values: Vec<AlternativeValue>,
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

/// Completion for one element of the array property `property`, given the
/// element as authored so far (`Value::Null` for a missing or empty item).
///
/// Returns `None` when the property is unknown, is not an array, or the
/// effective schema has no SimplifiedSchema projection. A root-level union
/// contributes the property's alternatives from every inline arm that
/// declares it. Each element is judged on its own, so neighboring items can
/// narrow to different alternatives. See [`alternatives_for_def`].
pub fn for_list_item(
    effective: &EffectiveSchema,
    property: &str,
    item: &Value,
) -> Option<AlternativeCompletion> {
    let def = property_def(effective, property)?;
    alternatives_for_def(&def, AlternativeSite::Item, item)
}

/// Completion for the value of `property` itself, given the value as
/// authored so far. A union used as a property completes exactly as the same
/// union used as an array element does through [`for_list_item`], apart from
/// `is_array`.
pub fn for_property_value(
    effective: &EffectiveSchema,
    property: &str,
    value: &Value,
) -> Option<AlternativeCompletion> {
    let def = property_def(effective, property)?;
    alternatives_for_def(&def, AlternativeSite::Value, value)
}

/// Whether `value` is a valid instance of one leaf alternative (an
/// [`Alternative::atom`]), lowered on its own as a present value: the
/// optional-property null wrapper is never added, so `null` matches only an
/// alternative that itself accepts `null`, as inside an item union.
///
/// Editors use it to name the alternative a value matches only when exactly
/// one accepts it. It is structural and passive: a `file` alternative is
/// judged by syntax, never by probing the filesystem, so it must not authorize
/// a transformation.
pub fn alternative_accepts(atom: &PropertyAtom, value: &Value) -> bool {
    let mut present = atom.clone();
    present.constraints.push(Constraint::Required);
    let mut shape = SchemaShape::default();
    shape
        .properties
        .insert("value".to_string(), PropertyDef::Single(present));
    let Ok(schema) = to_json_schema(&SimplifiedSchema::Single(shape)) else {
        return false;
    };
    ALTERNATIVE_VALIDATOR_CACHE
        .structural_validator_for(&schema)
        .is_ok_and(|validator| validator.is_valid(&serde_json::json!({ "value": value })))
}

/// Completion for one incomplete value of `def` at `site`, given the value as
/// authored so far.
///
/// The applicable alternatives follow the authored value's JSON kind, which
/// no alternative of another kind can ever accept: `null` keeps every
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
pub fn alternatives_for_def(
    def: &PropertyDef,
    site: AlternativeSite,
    value: &Value,
) -> Option<AlternativeCompletion> {
    let is_array = site == AlternativeSite::Item;
    let alternatives: Vec<&PropertyAtom> = flattened_alternatives(def)
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

    let mut completion = AlternativeCompletion {
        is_array,
        selected_arm,
        keys: Vec::new(),
        values: Vec::new(),
    };
    for (arm, atom) in alternatives.iter().enumerate() {
        if selected_arm.is_some_and(|selected| selected != arm) || !applies_to(atom, value) {
            continue;
        }
        match &atom.ty {
            TypeExpr::InlineObject(shape) => add_keys(&mut completion.keys, shape, arm),
            TypeExpr::Primitive(_) => add_values(&mut completion.values, atom, arm),
            // An unresolved import has no shape to offer; flattening never
            // yields a union.
            TypeExpr::Imported { .. } | TypeExpr::Union(_) => {}
        }
    }
    Some(completion)
}

/// `property`'s definition; across a root union, every declaring inline arm's
/// atoms as one property-level union.
fn property_def<'a>(effective: &'a EffectiveSchema, property: &str) -> Option<Cow<'a, PropertyDef>> {
    match effective.simplified.as_ref()? {
        SimplifiedSchema::Single(shape) => shape.properties.get(property).map(Cow::Borrowed),
        SimplifiedSchema::Union(arms) => {
            let mut atoms: Vec<PropertyAtom> = Vec::new();
            for arm in arms {
                let SchemaArm::Inline(shape) = arm else {
                    continue;
                };
                let Some(def) = shape.properties.get(property) else {
                    continue;
                };
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
                1 => atoms.pop().map(|atom| Cow::Owned(PropertyDef::Single(atom))),
                _ => Some(Cow::Owned(PropertyDef::Union(atoms))),
            }
        }
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
/// `{}` so indexes stay aligned.
fn discriminant_json(atom: &PropertyAtom) -> Value {
    let TypeExpr::InlineObject(shape) = &atom.ty else {
        return serde_json::json!({});
    };
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
        let required = match def {
            PropertyDef::Single(atom) => atom.is_required(),
            PropertyDef::Union(atoms) => atoms.iter().any(PropertyAtom::is_required),
        };
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

fn add_values(values: &mut Vec<AlternativeValue>, atom: &PropertyAtom, arm: usize) {
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
        if let Constraint::Members(members) = constraint {
            for member in members {
                push(member.clone(), yaml_double_quote(member));
            }
        }
    }
    // `suggestions_for_def` lints the arm's candidates and drops invalid ones;
    // the leaf name only labels lint context.
    if let Some(query) = suggestions_for_def("item", &PropertyDef::Single(atom.clone())) {
        for item in query.items {
            push(item.label, item.insert_text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;
    use crate::markdown::schemas::DarkmatterSchemas;

    fn effective(yaml_body: &str) -> EffectiveSchema {
        let md_text = format!("---\n{yaml_body}---\nbody\n");
        let md: Markdown = md_text.as_str().into();
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()));
        api.effective_for(&md)
            .expect("effective_for")
            .expect("effective schema present")
    }

    #[test]
    fn file_property_with_match_patterns() {
        let eff = effective("$schema:\n  cover: \"file(match('*.png', '*.jpg'))\"\n");
        let suggestion = for_property(&eff, "cover").expect("cover should be completable");
        assert_eq!(suggestion.property, "cover");
        assert!(!suggestion.is_array);
        assert!(!suggestion.generated);
        match suggestion.kind {
            CompletionKind::File { alternatives } => {
                assert_eq!(alternatives, vec![vec![vec!["*.png", "*.jpg"]]]);
            }
            other => panic!("expected File completion, got {other:?}"),
        }
    }

    #[test]
    fn file_property_without_match_yields_empty_patterns() {
        let eff = effective("$schema:\n  cover: file\n");
        let suggestion = for_property(&eff, "cover").expect("cover should be completable");
        match suggestion.kind {
            CompletionKind::File { alternatives } => assert!(alternatives.is_empty()),
            other => panic!("expected File completion, got {other:?}"),
        }
    }

    #[test]
    fn eager_file_match_patterns_still_reach_completion() {
        // Regression: `match(...)` is no longer lowered into the compiled JSON
        // Schema (the `x-darkmatter-match` keyword was removed), but completion
        // reads patterns from the SimplifiedSchema atom, so they must still
        // surface — including on an eager `file(eager; match(...))` field.
        let eff = effective("$schema:\n  review: \"file(eager; match('**/*review*.md'))\"\n");
        let suggestion = for_property(&eff, "review").expect("review should be completable");
        match suggestion.kind {
            CompletionKind::File { alternatives } => {
                assert_eq!(alternatives, vec![vec![vec!["**/*review*.md"]]]);
            }
            other => panic!("expected File completion, got {other:?}"),
        }
    }

    #[test]
    fn enum_property_returns_members() {
        let eff = effective("$schema:\n  status: enum(draft, published, archived)\n");
        let suggestion = for_property(&eff, "status").expect("status should be completable");
        match suggestion.kind {
            CompletionKind::Enum { members } => {
                assert_eq!(members, vec!["draft", "published", "archived"]);
            }
            other => panic!("expected Enum completion, got {other:?}"),
        }
    }

    #[test]
    fn generated_file_property_marks_completion_metadata() {
        let eff = effective("$schema:\n  cover: \"file(generated; match('*.png'))\"\n");
        let suggestion = for_property(&eff, "cover").expect("cover should be completable");
        assert!(suggestion.generated);
        match suggestion.kind {
            CompletionKind::File { alternatives } => assert_eq!(alternatives, vec![vec![vec!["*.png"]]]),
            other => panic!("expected File completion, got {other:?}"),
        }
    }

    #[test]
    fn generated_enum_property_marks_completion_metadata() {
        let eff = effective("$schema:\n  status: enum(draft, published; generated)\n");
        let suggestion = for_property(&eff, "status").expect("status should be completable");
        assert!(suggestion.generated);
        match suggestion.kind {
            CompletionKind::Enum { members } => assert_eq!(members, vec!["draft", "published"]),
            other => panic!("expected Enum completion, got {other:?}"),
        }
    }

    #[test]
    fn generated_hint_property_marks_completion_metadata() {
        let eff = effective("$schema:\n  published: date(generated)\n");
        let suggestion = for_property(&eff, "published").expect("published should be completable");
        assert!(suggestion.generated);
        match suggestion.kind {
            CompletionKind::Hint { format } => assert!(format.contains("date")),
            other => panic!("expected Hint, got {other:?}"),
        }
    }

    #[test]
    fn url_property_returns_hint() {
        let eff = effective("$schema:\n  homepage: url\n");
        let suggestion = for_property(&eff, "homepage").expect("homepage should be completable");
        match suggestion.kind {
            CompletionKind::Hint { format } => assert!(format.contains("URL")),
            other => panic!("expected Hint, got {other:?}"),
        }
    }

    #[test]
    fn non_completable_type_returns_none() {
        let eff = effective("$schema:\n  title: string\n");
        assert!(for_property(&eff, "title").is_none());
    }

    #[test]
    fn unknown_property_returns_none() {
        let eff = effective("$schema:\n  title: string\n");
        assert!(for_property(&eff, "nope").is_none());
    }

    #[test]
    fn array_file_property_is_marked_as_array() {
        let eff = effective("$schema:\n  attachments: \"file[]\"\n");
        let suggestion =
            for_property(&eff, "attachments").expect("attachments should be completable");
        assert!(suggestion.is_array);
        matches!(suggestion.kind, CompletionKind::File { .. });
    }

    #[test]
    fn completable_properties_filters_to_completable_types() {
        let eff = effective(
            "$schema:\n  title: string\n  cover: file\n  status: enum(a, b)\n  homepage: url\n",
        );
        let mut names = completable_properties(&eff);
        names.sort();
        assert_eq!(names, vec!["cover", "homepage", "status"]);
    }

    #[test]
    fn description_is_preserved() {
        let eff = effective("$schema:\n  cover: \"file -> The cover image for this post\"\n");
        let suggestion = for_property(&eff, "cover").expect("cover should be completable");
        assert_eq!(
            suggestion.description.as_deref(),
            Some("The cover image for this post")
        );
    }

    #[test]
    fn property_union_picks_first_completable_arm() {
        let eff = effective("$schema:\n  href:\n    - url\n    - file\n");
        let suggestion = for_property(&eff, "href").expect("href should be completable");
        match suggestion.kind {
            CompletionKind::Hint { format } => assert!(format.contains("URL")),
            other => panic!("expected URL hint from first arm, got {other:?}"),
        }
    }

    #[test]
    fn property_union_uses_selected_arm_generated_metadata() {
        let eff = effective("$schema:\n  href:\n    - url\n    - file(generated)\n");
        let suggestion = for_property(&eff, "href").expect("href should be completable");
        assert!(!suggestion.generated);
        match suggestion.kind {
            CompletionKind::Hint { format } => assert!(format.contains("URL")),
            other => panic!("expected URL hint from first arm, got {other:?}"),
        }

        let eff = effective("$schema:\n  href:\n    - url(generated)\n    - file\n");
        let suggestion = for_property(&eff, "href").expect("href should be completable");
        assert!(suggestion.generated);
        match suggestion.kind {
            CompletionKind::Hint { format } => assert!(format.contains("URL")),
            other => panic!("expected URL hint from first arm, got {other:?}"),
        }
    }

    #[test]
    fn root_union_offers_completable_properties_from_every_arm() {
        let eff = effective(concat!(
            "$schema:\n",
            "  - spec: \"file(match('**/spec*.md'))\"\n",
            "  - design: \"file(match('**/design*.md'))\"\n",
        ));
        let names = completable_properties(&eff);
        assert_eq!(names, vec!["spec", "design"]);
    }

    #[test]
    fn root_union_for_property_returns_per_arm_file_patterns() {
        let eff = effective(concat!(
            "$schema:\n",
            "  - spec: \"file(match('**/spec*.md'))\"\n",
            "  - design: \"file(match('**/design*.md'))\"\n",
        ));
        match for_property(&eff, "spec").expect("spec completable").kind {
            CompletionKind::File { alternatives } => assert_eq!(alternatives, vec![vec![vec!["**/spec*.md"]]]),
            other => panic!("expected File completion for spec, got {other:?}"),
        }
        match for_property(&eff, "design").expect("design completable").kind {
            CompletionKind::File { alternatives } => assert_eq!(alternatives, vec![vec![vec!["**/design*.md"]]]),
            other => panic!("expected File completion for design, got {other:?}"),
        }
    }

    #[test]
    fn root_union_offers_each_file_arm_as_its_own_alternative() {
        // The same property in two file-typed arms must offer either arm's
        // candidates instead of surfacing only the first arm's; each arm stays
        // its own alternative so one arm's globs never widen or narrow another.
        let eff = effective(concat!(
            "$schema:\n",
            "  - doc: \"file(match('**/spec*.md'))\"\n",
            "  - doc: \"file(match('**/design*.md', '**/spec*.md'))\"\n",
            "  - doc: \"file(match('**/spec*.md'))\"\n",
        ));
        match for_property(&eff, "doc").expect("doc completable").kind {
            CompletionKind::File { alternatives } => {
                assert_eq!(
                    alternatives,
                    vec![vec![vec!["**/spec*.md"]], vec![vec!["**/design*.md", "**/spec*.md"]]]
                );
            }
            other => panic!("expected combined File completion, got {other:?}"),
        }
    }

    #[test]
    fn root_union_file_property_is_generated_only_when_every_arm_is_generated() {
        let eff = effective(concat!(
            "$schema:\n",
            "  - doc: \"file(generated; match('**/spec*.md'))\"\n",
            "  - doc: \"file(generated; match('**/design*.md'))\"\n",
        ));
        let suggestion = for_property(&eff, "doc").expect("doc completable");
        assert!(suggestion.generated);
        match suggestion.kind {
            CompletionKind::File { alternatives } => {
                assert_eq!(alternatives, vec![vec![vec!["**/spec*.md"]], vec![vec!["**/design*.md"]]]);
            }
            other => panic!("expected combined File completion, got {other:?}"),
        }

        let eff = effective(concat!(
            "$schema:\n",
            "  - doc: \"file(generated; match('**/spec*.md'))\"\n",
            "  - doc: \"file(match('**/design*.md'))\"\n",
        ));
        let suggestion = for_property(&eff, "doc").expect("doc completable");
        assert!(!suggestion.generated);
        match suggestion.kind {
            CompletionKind::File { alternatives } => {
                assert_eq!(alternatives, vec![vec![vec!["**/spec*.md"]], vec![vec!["**/design*.md"]]]);
            }
            other => panic!("expected combined File completion, got {other:?}"),
        }
    }

    #[test]
    fn root_union_non_file_property_uses_selected_arm_generated_metadata() {
        let eff = effective(concat!(
            "$schema:\n",
            "  - status: enum(draft, published; generated)\n",
            "  - status: enum(archived)\n",
        ));
        let suggestion = for_property(&eff, "status").expect("status completable");
        assert!(suggestion.generated);
        match suggestion.kind {
            CompletionKind::Enum { members } => assert_eq!(members, vec!["draft", "published"]),
            other => panic!("expected Enum completion, got {other:?}"),
        }
    }

    #[test]
    fn repeated_match_groups_stay_separate_groups_of_one_alternative() {
        let eff = effective("$schema:\n  doc: \"file(match('*.md'); match('guide*', '!guide-old*'))\"\n");
        match for_property(&eff, "doc").expect("doc completable").kind {
            CompletionKind::File { alternatives } => {
                assert_eq!(alternatives, vec![vec![vec!["*.md"], vec!["guide*", "!guide-old*"]]]);
            }
            other => panic!("expected grouped File completion, got {other:?}"),
        }
    }

    #[test]
    fn root_union_arm_keeps_its_groups_together() {
        let eff = effective(concat!(
            "$schema:\n",
            "  - doc: \"file(match('*.md'); match('guide*'))\"\n",
            "  - doc: \"file(match('*.txt'))\"\n",
        ));
        match for_property(&eff, "doc").expect("doc completable").kind {
            CompletionKind::File { alternatives } => {
                assert_eq!(alternatives, vec![vec![vec!["*.md"], vec!["guide*"]], vec![vec!["*.txt"]]]);
            }
            other => panic!("expected grouped File completion, got {other:?}"),
        }
    }
}
