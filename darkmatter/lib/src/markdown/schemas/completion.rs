//! Schema-aware completion hints for external consumers (e.g. shell-completion
//! generators).
//!
//! This module is a **read-only consumer** of an [`EffectiveSchema`]: it
//! inspects the SimplifiedSchema projection (when available) and yields
//! completion data for a property, without mutating or re-resolving the
//! schema. See the *Shell Completions Integration* section of the spec.
//!
//! [`for_list_item`], [`for_property_value`], and the library's
//! [`alternatives_for_def`] answer a second question for editors: which keys and scalar values to
//! offer inside one incomplete union value, such as an element of `U[]`.
//! [`for_path`] asks it at any depth, following the cursor's [`PathStep`]s
//! through inline objects, array elements, and recursive references.
//!
//! [`path_candidates`] answers a third, for `md`'s shell completion: what may
//! come next in a [`PropertyPath`] or its value, for one command.
//!
//! ## Recursive references
//!
//! A recursive named type, or a shared acyclic one, is a [`TypeExpr::Ref`]
//! into a definition table.
//! Every entry point here opens a reference only where the position it answers
//! is one, through a [`DefinitionLookup`] (the [`EffectiveSchema`] itself, or
//! a table), so descending `tree.children[0].children[0]` opens `item` once
//! per level and never the definitions the path does not reach. A reference
//! whose definition is missing offers nothing.
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

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::EffectiveSchema;
use crate::markdown::compose::expression::call_binding::{NoNamedTypes, shape_from_property_def_with_definitions};
use crate::markdown::compose::setters::{TokenClass, classify_token};
use simplified_schema::coercion::{Additional, Shape};
pub use simplified_schema::completion::*;
use simplified_schema::simplified::query::{Alternative, resolved_alternatives};
use simplified_schema::simplified::{
    Constraint, DefKey, Definition, DefinitionTable, PropertyAtom, PropertyDef, SchemaArm, SchemaShape, SchemaRoot, SimplifiedType,
    TypeExpr,
};
use simplified_schema::{PropertyPath, ResolveStop, SetterValue};

/// Resolves a reference in the document's table or a trigger payload's, as
/// [`EffectiveSchema::definition`] does.
impl DefinitionLookup for EffectiveSchema {
    fn lookup_definition(&self, key: &DefKey) -> Option<&Definition> {
        self.definition(key)
    }
}

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
/// way, and so is a recursive reference whose definition is a union.
///
/// For a **root-level union**, `property` is gathered from every inline arm
/// that declares it. When every contributing arm is a `file`, each arm's
/// `match(...)` groups become one alternative, so the union offers the union
/// of its arms' candidates; otherwise the first completable arm wins. See
/// [`suggestion_from_union`].
pub fn for_property(effective: &EffectiveSchema, property: &str) -> Option<CompletionSuggestion> {
    match effective.simplified.as_ref()? {
        SchemaRoot::Single(shape) => {
            let def = shape.properties.get(property)?;
            suggestion_from_def(property, def, effective)
        }
        SchemaRoot::Union(arms) => suggestion_from_union(property, arms, effective),
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
        SchemaRoot::Single(shape) => completable_in_shape(shape, effective),
        SchemaRoot::Union(arms) => {
            let mut out: Vec<String> = Vec::new();
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    for name in completable_in_shape(shape, effective) {
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

fn completable_in_shape(shape: &SchemaShape, definitions: &EffectiveSchema) -> Vec<String> {
    shape
        .properties
        .iter()
        .filter_map(|(name, def)| first_completable_atom(def, definitions).map(|_| name.clone()))
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
fn suggestion_from_union<'a>(
    property: &str,
    arms: &'a [SchemaArm],
    definitions: &'a EffectiveSchema,
) -> Option<CompletionSuggestion> {
    let alternatives: Vec<Alternative<'a>> = arms
        .iter()
        .filter_map(|arm| match arm {
            SchemaArm::Inline(shape) => shape.properties.get(property),
            SchemaArm::FileRef(_) => None,
        })
        .filter_map(|def| first_completable_atom(def, definitions))
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

fn suggestion_from_def(
    property: &str,
    def: &PropertyDef,
    definitions: &EffectiveSchema,
) -> Option<CompletionSuggestion> {
    let alternative = first_completable_atom(def, definitions)?;
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
/// property-level union `U` expands to, and through a recursive reference to
/// its definition's alternatives.
fn first_completable_atom<'a>(def: &'a PropertyDef, definitions: &'a EffectiveSchema) -> Option<Alternative<'a>> {
    resolved_alternatives(def, definitions)
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
        // arms through `resolved_alternatives`, which also opens a reference,
        // so a `Ref` here is one whose definition is missing.
        TypeExpr::InlineObject(_)
        | TypeExpr::Imported { .. }
        | TypeExpr::Union(_)
        | TypeExpr::Tuple(_)
        | TypeExpr::Function(_)
        | TypeExpr::Ref(_) => false,
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
        TypeExpr::InlineObject(_)
        | TypeExpr::Imported { .. }
        | TypeExpr::Union(_)
        | TypeExpr::Tuple(_)
        | TypeExpr::Function(_)
        | TypeExpr::Ref(_) => return None,
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

/// Completion for one element of the array property `property`, given the
/// element as authored so far (`Value::Null` for a missing or empty item).
///
/// Returns `None` when the property is unknown, is not an array, or the
/// effective schema has no SimplifiedSchema projection. A root-level union
/// contributes the property's alternatives from every inline arm that
/// declares it. Each element is judged on its own, so neighboring items can
/// narrow to different alternatives. A recursive reference resolves through
/// `effective`. See [`alternatives_for_def_in`].
pub fn for_list_item(
    effective: &EffectiveSchema,
    property: &str,
    item: &Value,
) -> Option<AlternativeCompletion> {
    let schema = effective.simplified.as_ref()?;
    let def = root_property_def(schema, property)?;
    alternatives_for_def_in(&def, AlternativeSite::Item, item, effective)
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
    let schema = effective.simplified.as_ref()?;
    let def = root_property_def(schema, property)?;
    alternatives_for_def_in(&def, AlternativeSite::Value, value, effective)
}

/// The keys and values to offer for the value at `path`, given the whole
/// authored frontmatter mapping (`Value::Null` when none is available).
///
/// `path` starts at a top-level property and may cross inline objects, array
/// elements, and recursive references to any depth: `[Key("tree"),
/// Key("children"), Index(0), Key("children"), Index(0)]` offers the keys of
/// the second-level `item` even when that element is still empty or missing
/// required keys. Ending on an [`PathStep::Index`] completes one element
/// ([`AlternativeSite::Item`]); ending on a key completes that property's
/// value. Each ancestor that is a discriminated union narrows to the arm its
/// authored literal tag selects, as [`alternatives_for_def_in`] narrows the
/// final value; without a tag every object arm contributes.
///
/// Returns `None` for an empty path, a path that leaves the schema, or a
/// SimplifiedSchema-less effective schema. See [`for_path_in`] for the
/// reference-resolution contract.
pub fn for_path(effective: &EffectiveSchema, frontmatter: &Value, path: &[PathStep<'_>]) -> Option<AlternativeCompletion> {
    for_path_in(effective.simplified.as_ref()?, effective, frontmatter, path)
}

/// The type declared for the value at `path`, for reading a value written on a
/// command line through `SetterValue::bind`.
///
/// The declaration comes from the layers `effective` was assembled with
/// ([`EffectiveSchema::declared_property_shape`]), and a union narrows by
/// `frontmatter`'s literal tags. A key those layers do not declare is undeclared,
/// even when the compiled baseline would declare it. A pattern key declares every
/// key it matches.
///
/// `None` when the path leaves the schema, or the declaration has no conversion
/// shape: the caller then treats the value as undeclared. So is an `unknown` the
/// schema does not actually keep `written` at, which is how a raw JSON Schema's
/// `$ref` or `allOf` reads ([`keeps_text_at`]). Only unquoted text is in question:
/// `written` is `None` when there is no value yet, as in completion, and every
/// other form reads the same under `unknown` as undeclared.
pub fn declared_shape(
    effective: &EffectiveSchema,
    frontmatter: &Value,
    path: &[PathStep<'_>],
    written: Option<&SetterValue>,
) -> Option<Shape> {
    let root = SchemaRoot::Single(effective.declared_property_shape(frontmatter));
    // A definition stops at a pattern key and at an element of an element; the
    // shape it lowers to knows both.
    let (reached, (def, site)) =
        (1..=path.len()).rev().find_map(|depth| Some((depth, def_at_path(&root, effective, frontmatter, &path[..depth])?)))?;
    let lowered = shape_from_property_def_with_definitions(&def, &NoNamedTypes, effective.layered_definitions()).ok()?;
    let at_definition = match (site, path[reached - 1]) {
        (AlternativeSite::Item, PathStep::Index(index)) => child_shape(&lowered, PathStep::Index(index))?,
        _ => lowered,
    };
    let shape = path[reached..].iter().try_fold(at_definition, |shape, step| child_shape(&shape, *step))?;
    match written {
        Some(SetterValue::Text { raw }) if declares_nothing(&shape) => keeps_text_at(effective, frontmatter, path, raw).then_some(shape),
        _ => Some(shape),
    }
}

/// The shape of the child of a value shaped `shape`: [`def_at_path`] stops at the
/// array's own definition for a path ending on an index. A key several patterns
/// match must satisfy them all, which no one shape says, so it has none.
fn child_shape(shape: &Shape, step: PathStep<'_>) -> Option<Shape> {
    match (shape, step) {
        (Shape::Array(array), PathStep::Index(_)) => Some(array.items.as_ref().clone()),
        (Shape::Tuple(tuple), PathStep::Index(index)) => tuple
            .prefix
            .get(index)
            .map(|slot| slot.shape.clone())
            .or_else(|| tuple.rest.as_deref().cloned()),
        (Shape::Object(object), PathStep::Key(key)) => {
            if let Some(property) = object.properties.iter().find(|property| property.name == key) {
                return Some(property.shape.clone());
            }
            let mut matching = object.patterns.iter().filter(|pattern| pattern.matcher.matches(key));
            match (matching.next(), matching.next(), &object.additional) {
                (Some(only), None, _) => Some(only.shape.clone()),
                (None, _, Additional::Shape(additional)) => Some(additional.as_ref().clone()),
                _ => None,
            }
        }
        (Shape::Union(arms), _) => {
            let mut children: Vec<Shape> = arms.iter().filter_map(|arm| child_shape(arm, step)).collect();
            match children.len() {
                0 => None,
                1 => children.pop(),
                _ => Some(Shape::Union(children)),
            }
        }
        (Shape::Assert(inner, _) | Shape::Deferrable(inner), _) => child_shape(inner, step),
        (Shape::Ref(reference), _) => {
            let target = reference.target()?;
            child_shape(&target, step)
        }
        _ => None,
    }
}

fn declares_nothing(shape: &Shape) -> bool {
    match shape {
        Shape::Any => true,
        Shape::Deferrable(inner) => declares_nothing(inner),
        _ => false,
    }
}

/// Whether the schema accepts `text` at `path`, as written, leaving aside problems
/// elsewhere in the document.
///
/// A declared `unknown` does. A raw JSON Schema layer also projects what it cannot
/// name to `unknown`, so reading a value as text there could turn a number the
/// schema requires into a string its conversion cannot recover from `allOf`. The
/// question is about this text because a string's fit can depend on its content:
/// a `pattern`, a `format`, or an `enum` behind a `$ref`.
fn keeps_text_at(effective: &EffectiveSchema, frontmatter: &Value, path: &[PathStep<'_>], text: &str) -> bool {
    let mut probe = frontmatter.clone();
    let slot = path.iter().try_fold(&mut probe, |value, step| match step {
        PathStep::Key(key) => value.get_mut(*key),
        PathStep::Index(index) => value.get_mut(*index),
    });
    let Some(slot) = slot else {
        return true;
    };
    *slot = Value::String(text.to_string());
    let target: Vec<String> = path
        .iter()
        .map(|step| match step {
            PathStep::Key(key) => (*key).to_string(),
            PathStep::Index(index) => index.to_string(),
        })
        .collect();
    !effective.validate_raw(&probe).problems.iter().any(|problem| {
        // A required or unknown-key problem names a child, never this value, and
        // one at the root is about the whole document.
        let about_a_value = problem.property.is_none() && problem.offending_properties.is_empty();
        let at = problem.instance_path.segments();
        about_a_value && !at.is_empty() && at.iter().zip(&target).all(|(left, right)| left == right)
    })
}

/// The command a path is completed for. Each one decides which keys are offered
/// and how a candidate is spelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathCommand {
    /// `md compose`, whose paths are `key=value` setters.
    Compose,
    /// `md set`, which takes the path and the value as separate arguments.
    Set,
    /// `md get`.
    Get,
    /// `md rm`.
    Rm,
}

/// The word being completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPosition<'a> {
    /// A word naming a path. Under [`PathCommand::Compose`] it may already hold
    /// a setter's `=`, which asks for that key's values.
    Path(&'a str),
    /// A value given as its own argument, as `md set` takes one.
    Value {
        /// The path the value is for.
        path: &'a PropertyPath,
        /// The value typed so far.
        typed: &'a str,
    },
}

/// The schema a completion question reads. Only one is ever read: a document may
/// replace a baseline declaration, so they are never combined.
#[derive(Clone, Copy)]
pub enum PathSchema<'a> {
    /// The schema the request selected.
    Effective(&'a EffectiveSchema),
    /// The compiled baseline, when the request's own selection could not be worked
    /// out.
    CompiledBaseline,
    /// The request selected no schema at all, as with a disabled baseline and a
    /// document without `$schema`: only present keys remain.
    Nothing,
}

/// One completion question for [`path_candidates`].
#[derive(Clone, Copy)]
pub struct PathQuery<'a> {
    /// What declares the keys and values.
    pub schema: PathSchema<'a>,
    /// The frontmatter with the line's earlier setters already applied. Anything
    /// but an object reads as an empty document.
    pub frontmatter: &'a Value,
    /// The receiving command.
    pub command: PathCommand,
    /// The word being completed.
    pub position: PathPosition<'a>,
    /// Paths given earlier on the line, which are not offered again.
    pub earlier_paths: &'a [PropertyPath],
}

/// What accepting a [`PathCandidate`] does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathCandidateKind {
    /// `style.page.`: continue into the children, with no space after it.
    Descend,
    /// `title=`: the value comes next, with no space after it.
    Assign,
    /// `title`: names a whole node, so a space follows.
    Select,
    /// A finished value, so a space follows.
    Value,
}

/// One completion, independent of any shell or editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathCandidate {
    /// The complete text of the word under the cursor, property-path escaped and
    /// spelled so `md` decodes the intended value, but with no shell quoting.
    /// For a setter's value this includes `key=`; a shell that replaces only
    /// the text after `=` must strip it.
    pub insert: String,
    /// The schema's description of the key, when it has one.
    pub help: Option<String>,
    /// What accepting it does next.
    pub kind: PathCandidateKind,
}

/// The candidates for one word, in the order they should be shown.
///
/// Reads only `query`: it opens no file, evaluates nothing, and reports nothing,
/// so a word it cannot make sense of simply has no candidates. A candidate that
/// would fail at once when run, such as a child of a scalar, is left out.
///
/// See [`path_value_takes_files`] for values that come from the filesystem,
/// which this function does not walk.
#[must_use]
pub fn path_candidates(query: &PathQuery<'_>) -> Vec<PathCandidate> {
    let no_definitions = DefinitionTable::default();
    let no_frontmatter = Value::Object(serde_json::Map::new());
    let frontmatter = readable_frontmatter(query, &no_frontmatter);
    match word_target(query) {
        WordTarget::Path(typed) => key_candidates(query, &SchemaView::new(query.schema, frontmatter, &no_definitions), typed),
        WordTarget::Value(target) => value_candidates(query, frontmatter, &target, &no_definitions),
        WordTarget::Nothing => Vec::new(),
    }
}

/// Whether the value being completed may be a file path, so a caller should add
/// ordinary files to [`path_candidates`]. `false` for a word naming a path.
#[must_use]
pub fn path_value_takes_files(query: &PathQuery<'_>) -> bool {
    let WordTarget::Value(target) = word_target(query) else {
        return false;
    };
    let no_definitions = DefinitionTable::default();
    let no_frontmatter = Value::Object(serde_json::Map::new());
    let frontmatter = readable_frontmatter(query, &no_frontmatter);
    let Some((steps, _)) = walk_path(frontmatter, &target.path) else {
        return false;
    };
    let cleared = without_value(frontmatter, &steps);
    let view = SchemaView::new(query.schema, &cleared, &no_definitions);
    let Some((def, site)) = def_at_path(&view.root, view.definitions, view.frontmatter, &steps) else {
        return false;
    };
    let is_array = site == AlternativeSite::Item;
    resolved_alternatives(&def, view.definitions)
        .into_iter()
        .any(|alternative| alternative.is_array == is_array && matches!(alternative.atom.ty, TypeExpr::Primitive(SimplifiedType::File)))
}

fn readable_frontmatter<'a>(query: &PathQuery<'a>, no_frontmatter: &'a Value) -> &'a Value {
    if query.frontmatter.is_object() { query.frontmatter } else { no_frontmatter }
}

enum WordTarget<'a> {
    Path(&'a str),
    Value(ValueTarget<'a>),
    Nothing,
}

struct ValueTarget<'a> {
    path: PropertyPath,
    typed: &'a str,
    /// The `key=` a candidate repeats, so it is the whole word.
    lead: &'a str,
}

fn word_target<'a>(query: &PathQuery<'a>) -> WordTarget<'a> {
    match query.position {
        PathPosition::Value { path, typed } => WordTarget::Value(ValueTarget {
            path: path.clone(),
            typed,
            lead: "",
        }),
        PathPosition::Path(typed) if query.command != PathCommand::Compose || !typed.contains('=') => WordTarget::Path(typed),
        PathPosition::Path(typed) => match classify_token(typed) {
            TokenClass::Setter(setter) => {
                let (lead, value) = typed.split_at(typed.len() - setter.raw.len());
                WordTarget::Value(ValueTarget {
                    path: setter.path,
                    typed: value,
                    lead,
                })
            }
            TokenClass::Path | TokenClass::EmptyKey => WordTarget::Nothing,
        },
    }
}

/// The schema one question reads, over the same layers [`declared_shape`] reads.
///
/// [`for_path`] is not enough here: it reads only the document's own layer, so it
/// would never reach a baseline or trigger property such as `style`.
struct SchemaView<'a> {
    root: SchemaRoot,
    definitions: &'a dyn DefinitionLookup,
    frontmatter: &'a Value,
}

impl<'a> SchemaView<'a> {
    fn new(schema: PathSchema<'a>, frontmatter: &'a Value, no_definitions: &'a DefinitionTable) -> Self {
        match schema {
            PathSchema::Effective(effective) => Self {
                root: SchemaRoot::Single(effective.declared_property_shape(frontmatter)),
                definitions: effective,
                frontmatter,
            },
            // The baseline is definition-free, so an empty table reads all of it.
            PathSchema::CompiledBaseline => Self {
                root: SchemaRoot::Single(super::frontmatter_shape::effective_property_shape(&[], None, frontmatter)),
                definitions: no_definitions,
                frontmatter,
            },
            PathSchema::Nothing => Self {
                root: SchemaRoot::Single(SchemaShape::default()),
                definitions: no_definitions,
                frontmatter,
            },
        }
    }

    /// The keys the schema declares directly under `steps`, the root when empty.
    fn declared_keys(&self, steps: &[PathStep<'_>]) -> Vec<AlternativeKey> {
        if steps.is_empty() {
            let SchemaRoot::Single(shape) = &self.root else {
                return Vec::new();
            };
            return shape
                .properties
                .iter()
                .map(|(name, def)| AlternativeKey {
                    name: name.clone(),
                    required: def.is_required(),
                    description: declared_description(def),
                    arms: Vec::new(),
                })
                .collect();
        }
        for_path_in(&self.root, self.definitions, self.frontmatter, steps).map_or_else(Vec::new, |completion| completion.keys)
    }

    /// Whether the value at `steps` may be something other than an inline
    /// object. An undeclared value may be anything.
    fn admits_non_object(&self, steps: &[PathStep<'_>]) -> bool {
        let Some((def, site)) = def_at_path(&self.root, self.definitions, self.frontmatter, steps) else {
            return true;
        };
        let is_array = site == AlternativeSite::Item;
        resolved_alternatives(&def, self.definitions)
            .into_iter()
            .any(|alternative| alternative.is_array != is_array || !matches!(alternative.atom.ty, TypeExpr::InlineObject(_)))
    }
}

fn declared_description(def: &PropertyDef) -> Option<String> {
    match def {
        PropertyDef::Single(atom) => atom.description.clone(),
        PropertyDef::Union(atoms) => atoms.iter().find_map(|atom| atom.description.clone()),
    }
}

/// Splits a typed path at its last unescaped dot. A trailing dot leaves an empty
/// partial segment, which asks for every child.
fn split_typed(typed: &str) -> (Option<&str>, &str) {
    let mut last_dot = None;
    let mut characters = typed.char_indices().peekable();
    while let Some((position, character)) = characters.next() {
        match character {
            '\\' if matches!(characters.peek(), Some((_, '.' | '\\'))) => {
                characters.next();
            }
            '.' => last_dot = Some(position),
            _ => {}
        }
    }
    match last_dot {
        Some(position) => (Some(&typed[..position]), &typed[position + 1..]),
        None => (None, typed),
    }
}

/// How `path` walks `frontmatter`: its typed steps, including those past the end
/// of what is present, and the value when it is there. `None` when running the
/// path could only fail: a scalar or `null` parent, an index out of range, or a
/// number under a missing parent.
fn walk_path<'a>(frontmatter: &'a Value, path: &'a PropertyPath) -> Option<(Vec<PathStep<'a>>, Option<&'a Value>)> {
    let resolved = path.resolve(frontmatter);
    match resolved.stop() {
        ResolveStop::Found(value) => Some((resolved.steps().to_vec(), Some(value))),
        ResolveStop::MissingKey => {
            let missing = resolved.unresolved();
            let number_under_missing = missing.iter().skip(1).any(|segment| !segment.is_empty() && segment.bytes().all(|byte| byte.is_ascii_digit()));
            (!number_under_missing).then(|| {
                let mut steps = resolved.steps().to_vec();
                steps.extend(missing.iter().map(|segment| PathStep::Key(segment.as_str())));
                (steps, None)
            })
        }
        ResolveStop::OutOfRange { .. } | ResolveStop::NotContainer { .. } => None,
    }
}

fn key_candidates(query: &PathQuery<'_>, view: &SchemaView<'_>, typed: &str) -> Vec<PathCandidate> {
    let (parent_text, partial) = split_typed(typed);
    let Ok(decoded_partial) = PropertyPath::parse(partial) else {
        return Vec::new();
    };
    let decoded_partial = decoded_partial.segments()[0].as_str();
    let parent = match parent_text.map(PropertyPath::parse) {
        None => None,
        Some(Ok(parent)) if parent.segments().iter().all(|segment| !segment.is_empty()) => Some(parent),
        Some(_) => return Vec::new(),
    };
    let (parent_steps, parent_value) = match &parent {
        None => (Vec::new(), Some(view.frontmatter)),
        Some(parent) => match walk_path(view.frontmatter, parent) {
            Some(walked) => walked,
            None => return Vec::new(),
        },
    };
    if parent_value.is_some_and(|value| !value.is_object() && !value.is_array()) {
        return Vec::new();
    }

    let parent_is_array = parent_value.is_some_and(Value::is_array);
    let children = if parent_is_array {
        let length = parent_value.and_then(Value::as_array).map_or(0, Vec::len);
        (0..length).map(|index| (index.to_string(), None)).collect()
    } else {
        let present = parent_value
            .and_then(Value::as_object)
            .map_or_else(Vec::new, |object| object.keys().cloned().collect());
        ordered_children(query.command, view.declared_keys(&parent_steps), present)
    };

    let lead = parent_text.map_or_else(String::new, |text| format!("{text}."));
    let parent_segments = parent.as_ref().map_or(&[][..], PropertyPath::segments);
    let mut candidates = Vec::new();
    for (position, (name, help)) in children.iter().enumerate() {
        let Ok(spelled) = PropertyPath::from_segments([name.as_str()]) else {
            continue;
        };
        let spelled = spelled.to_string();
        let segments = parent_segments.iter().map(String::as_str).chain([name.as_str()]);
        let Ok(path) = PropertyPath::from_segments(segments) else {
            continue;
        };
        let matches_partial = spelled.starts_with(partial) || name.starts_with(decoded_partial);
        if name.is_empty() || !matches_partial || query.earlier_paths.contains(&path) || !command_can_spell(query.command, &path) {
            continue;
        }
        let step = if parent_is_array { PathStep::Index(position) } else { PathStep::Key(name) };
        let mut steps = parent_steps.clone();
        steps.push(step);
        let present = parent_value.and_then(|value| match step {
            PathStep::Key(key) => value.get(key),
            PathStep::Index(index) => value.get(index),
        });
        for (suffix, kind) in node_spellings(query.command, view, &steps, present) {
            candidates.push(PathCandidate {
                insert: format!("{lead}{spelled}{suffix}"),
                help: help.clone(),
                kind,
            });
        }
    }
    candidates
}

/// Child keys in the command's order, each once, keeping the schema's description
/// on a key that is also present.
fn ordered_children(command: PathCommand, declared: Vec<AlternativeKey>, present: Vec<String>) -> Vec<(String, Option<String>)> {
    let descriptions: HashMap<String, Option<String>> = declared.iter().map(|key| (key.name.clone(), key.description.clone())).collect();
    let (required, optional): (Vec<_>, Vec<_>) = declared.into_iter().partition(|key| key.required);
    let declared_names = required.into_iter().chain(optional).map(|key| key.name);
    let names: Vec<String> = match command {
        PathCommand::Compose | PathCommand::Set => declared_names.chain(present).collect(),
        PathCommand::Get => present.into_iter().chain(declared_names).collect(),
        PathCommand::Rm => present,
    };
    let mut seen = HashSet::new();
    names
        .into_iter()
        .filter(|name| seen.insert(name.clone()))
        .map(|name| {
            let help = descriptions.get(&name).cloned().flatten();
            (name, help)
        })
        .collect()
}

fn command_can_spell(command: PathCommand, path: &PropertyPath) -> bool {
    command != PathCommand::Compose || matches!(classify_token(&format!("{path}=")), TokenClass::Setter(setter) if setter.path == *path)
}

/// How one node may be written for `command`, descent first.
fn node_spellings(
    command: PathCommand,
    view: &SchemaView<'_>,
    steps: &[PathStep<'_>],
    present: Option<&Value>,
) -> Vec<(&'static str, PathCandidateKind)> {
    // `rm` can only remove what is there, so declared keys open nothing for it.
    let declares_keys = || command != PathCommand::Rm && !view.declared_keys(steps).is_empty();
    let descends = match present {
        Some(Value::Object(object)) => !object.is_empty() || declares_keys(),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(_) => false,
        None => declares_keys(),
    };
    let mut spellings = Vec::new();
    if descends {
        spellings.push((".", PathCandidateKind::Descend));
    }
    if command == PathCommand::Compose {
        spellings.push(("=", PathCandidateKind::Assign));
    } else if present.is_some() || !descends || view.admits_non_object(steps) {
        spellings.push(("", PathCandidateKind::Select));
    }
    spellings
}

fn value_candidates(
    query: &PathQuery<'_>,
    frontmatter: &Value,
    target: &ValueTarget<'_>,
    no_definitions: &DefinitionTable,
) -> Vec<PathCandidate> {
    let Some((steps, _)) = walk_path(frontmatter, &target.path) else {
        return Vec::new();
    };
    // The value being replaced must not narrow its own choices: a tag that is
    // set already still offers the others.
    let cleared = without_value(frontmatter, &steps);
    let view = SchemaView::new(query.schema, &cleared, no_definitions);
    let Some((def, site)) = def_at_path(&view.root, view.definitions, view.frontmatter, &steps) else {
        return Vec::new();
    };
    let declared = match query.schema {
        PathSchema::Effective(effective) => declared_shape(effective, &cleared, &steps, None),
        PathSchema::CompiledBaseline | PathSchema::Nothing => None,
    };
    let is_array = site == AlternativeSite::Item;
    let offered = alternatives_for_def_in(&def, site, &Value::Null, view.definitions).map_or_else(Vec::new, |completion| completion.values);

    let mut values: Vec<(String, Value)> = Vec::new();
    let alternatives = resolved_alternatives(&def, view.definitions);
    let alternatives = alternatives.iter().filter(|alternative| alternative.is_array == is_array);
    for (arm, alternative) in alternatives.enumerate() {
        values.extend(offered.iter().filter(|value| value.arm == arm).map(|value| (value.label.clone(), intended_value(value))));
        if matches!(alternative.atom.ty, TypeExpr::Primitive(SimplifiedType::Boolean)) {
            values.extend([("true".to_string(), Value::Bool(true)), ("false".to_string(), Value::Bool(false))]);
        }
    }

    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter_map(|(label, intended)| {
            let spelled = spell_value(&label, &intended, declared.as_ref());
            let matches_typed = spelled.starts_with(target.typed) || label.starts_with(target.typed);
            (matches_typed && seen.insert(spelled.clone())).then(|| PathCandidate {
                insert: format!("{}{spelled}", target.lead),
                help: None,
                kind: PathCandidateKind::Value,
            })
        })
        .collect()
}

/// A copy of `frontmatter` without the value at `steps`. An array element becomes
/// `null`, so its neighbors keep their positions.
fn without_value(frontmatter: &Value, steps: &[PathStep<'_>]) -> Value {
    let mut copy = frontmatter.clone();
    let Some((last, parents)) = steps.split_last() else {
        return copy;
    };
    let parent = parents.iter().try_fold(&mut copy, |value, step| match step {
        PathStep::Key(key) => value.get_mut(*key),
        PathStep::Index(index) => value.get_mut(*index),
    });
    match (parent, last) {
        (Some(Value::Object(object)), PathStep::Key(key)) => {
            object.remove(*key);
        }
        (Some(Value::Array(items)), PathStep::Index(index)) => {
            if let Some(item) = items.get_mut(*index) {
                *item = Value::Null;
            }
        }
        _ => {}
    }
    copy
}

/// The scalar an [`AlternativeValue`] stands for: its insertion text quotes a
/// string and nothing else, and its label is the decoded text.
fn intended_value(value: &AlternativeValue) -> Value {
    if value.insert_text.starts_with('"') {
        return Value::String(value.label.clone());
    }
    serde_json::from_str(&value.label).unwrap_or_else(|_| Value::String(value.label.clone()))
}

/// The text `md` reads back as `intended`: the label when the value parser and
/// `declared` agree on it, otherwise a quoted string. A member named `null`, `5`,
/// or `{a}` would otherwise arrive as something else.
fn spell_value(label: &str, intended: &Value, declared: Option<&Shape>) -> String {
    let reads_back = SetterValue::parse(label).is_ok_and(|value| value.bind(declared) == *intended);
    match intended {
        Value::String(text) if !reads_back => Value::String(text.clone()).to_string(),
        _ => label.to_string(),
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
