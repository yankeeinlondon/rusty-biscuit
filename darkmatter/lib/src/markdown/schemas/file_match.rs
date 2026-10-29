//! `file(match(...))` globs: completion candidates everywhere, and a
//! constraint on each declaring arm of a root union.
//!
//! On a single schema `match` only suggests files: completion and choosers
//! walk it, and validation ignores it. In a root union, conversion emits a
//! declared glob as [`DARKMATTER_MATCH_KEYWORD`] in two places: on an arm
//! property that is directly a `file` (or `file[]`), and on every file inside
//! the arm's own copy of a recursive definition the arm references, at any
//! depth. An existing file outside the glob then rules that arm out, in arm
//! selection and in the final verdict alike. Any other file in an arm (a
//! property-union alternative, a nested inline object, a dictionary value, a
//! tuple slot, a list of a union) only suggests, exactly as it did before
//! recursive types existed.
//!
//! A value is judged only when it names an existing file. A missing file is
//! the existence check's verdict (`file(eager)`), or a lazy output that does
//! not exist yet, and a partial a chooser is about to complete must not rule
//! an arm out before the user picks a file.
//!
//! Each pattern is a [`GlobReference`] pattern (`[!][prefix]glob`) with the
//! file-name view, judged from the value's `cwd`: the document's folder for a
//! frontmatter value, the launch directory for a caller-supplied one.
//!
//! ## Groups
//!
//! Each authored `match(...)` is one group. [`FileMatchGlobs`] judges one
//! group: some positive pattern admits the path and no `!` exclusion of that
//! group rejects it. [`FileMatchGroups`] judges several: a path is accepted
//! only when every group accepts it. Groups are never concatenated into one
//! list, so an exclusion in one group never removes another group's
//! positives, and a combined list would widen the positive choices. Groups
//! whose intersection is empty are valid; proving emptiness would need a file
//! search, and neither construction nor preparation searches.
//!
//! [`FileMatchGlobs`] and [`PreparedFileMatch`] are shared with Claudine's
//! candidate walk.
//!
//! ## Keyword encoding
//!
//! [`DARKMATTER_MATCH_KEYWORD`] is generated-schema metadata, not user
//! configuration. One group is a flat array of patterns
//! (`["*.md", "!_*.md"]`); more than one group is an array of pattern arrays
//! (`[["*.md"], ["guide*"]]`). A mixed array, an empty group, or a non-string
//! pattern is a schema error.

use std::path::{Path, PathBuf};

use biscuit_file::{
    FileResolutionContext, GlobReference, GlobReferenceError, PathIdentity, PreparedGlobReference,
};
use jsonschema::{Keyword, ValidationError, paths::Location};
use serde_json::{Map, Value};

use super::format::resolve_file_reference_in_context;
use super::simplified::{Constraint, DefKey, DefinitionTable, PropertyAtom, PropertyDef, SchemaArm, SimplifiedType, TypeExpr};

/// Keyword carrying the globs a root-union arm enforces on a file value.
pub const DARKMATTER_MATCH_KEYWORD: &str = "x-darkmatter-match";

/// Sibling of [`DARKMATTER_MATCH_KEYWORD`] in an arm's enforcing copy of a
/// recursive definition: the top-level property whose value the copy
/// types, or `null` below a pattern key. A copy sits under `$defs`, so its
/// location cannot name the property a caller may have supplied.
pub const DARKMATTER_MATCH_PROPERTY_KEYWORD: &str = "x-darkmatter-match-property";

/// The patterns of one `file(match(...))` constraint, as a [`GlobReference`]
/// with the file-name view.
///
/// A path is accepted when some positive pattern admits it and no `!`
/// pattern rejects it, each judged relative to the first of its own roots
/// that contains the path. A pattern whose glob after the prefix has no `/`
/// (`*.md`, `^*.md`, `!_*.md`) also matches a file's bare name at any depth.
#[derive(Debug, Clone)]
pub struct FileMatchGlobs {
    globs: GlobReference,
}

impl FileMatchGlobs {
    /// Parse `patterns`.
    ///
    /// ## Errors
    ///
    /// The [`GlobReferenceError`] of the first pattern that is not a valid
    /// glob reference (an invalid glob, `%`, a remote URL, a malformed
    /// prefix), or [`GlobReferenceError::NoPositivePattern`].
    pub fn new(patterns: &[String]) -> Result<Self, GlobReferenceError> {
        Ok(Self {
            globs: GlobReference::new(patterns)?.with_file_name_view(),
        })
    }

    /// Whether `path` is accepted, judged without a walk; it need not exist.
    /// A relative path is read from `ctx`'s `cwd`, which is also where bare
    /// and `./` patterns start.
    #[must_use]
    pub fn matches(&self, path: &Path, ctx: &FileResolutionContext) -> bool {
        self.globs.matches(path, ctx)
    }

    /// The globs resolved against `ctx` once, for a caller that walks their
    /// roots and judges each file it reaches (see
    /// [`GlobReference::prepare`]). `None` for an invalid context.
    #[must_use]
    pub fn prepare(&self, ctx: &FileResolutionContext) -> Option<PreparedGlobReference> {
        self.globs.prepare(ctx)
    }
}

/// Every `match(...)` group of one file property, in authored order; a path
/// is accepted only when every group accepts it (see the module's
/// "Groups").
#[derive(Debug, Clone)]
pub struct FileMatchGroups {
    /// Never empty.
    groups: Vec<FileMatchGlobs>,
}

impl FileMatchGroups {
    /// Parse each group as a [`FileMatchGlobs`].
    ///
    /// ## Errors
    ///
    /// [`GlobReferenceError::NoPositivePattern`] for an empty list of groups,
    /// as for an empty pattern list; otherwise the first error
    /// [`FileMatchGlobs::new`] reports for a group.
    pub fn new<G: AsRef<[String]>>(groups: &[G]) -> Result<Self, GlobReferenceError> {
        if groups.is_empty() {
            return Err(GlobReferenceError::NoPositivePattern);
        }
        let groups = groups
            .iter()
            .map(|group| FileMatchGlobs::new(group.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { groups })
    }

    /// The groups in authored order.
    #[must_use]
    pub fn groups(&self) -> &[FileMatchGlobs] {
        &self.groups
    }

    /// Whether every group accepts `path`, as [`FileMatchGlobs::matches`]
    /// judges it.
    #[must_use]
    pub fn matches(&self, path: &Path, ctx: &FileResolutionContext) -> bool {
        self.groups.iter().all(|group| group.matches(path, ctx))
    }

    /// Every group resolved against `ctx` once. `None` for an invalid
    /// context. Resolves roots only; never lists a directory.
    #[must_use]
    pub fn prepare(&self, ctx: &FileResolutionContext) -> Option<PreparedFileMatch> {
        let groups = self
            .groups
            .iter()
            .map(|group| group.prepare(ctx))
            .collect::<Option<Vec<_>>>()?;
        Some(PreparedFileMatch { groups })
    }
}

/// [`FileMatchGroups`] resolved against one context, for a caller that walks
/// [`roots`](Self::roots) itself and must offer exactly the files every
/// group would list.
pub struct PreparedFileMatch {
    /// One per group, in authored order; never empty.
    groups: Vec<PreparedGlobReference>,
}

impl PreparedFileMatch {
    /// The first group's roots, in precedence order.
    ///
    /// Walking them is complete: a file every group lists is in particular
    /// listed by the first group, so it lies under one of these roots.
    #[must_use]
    pub fn roots(&self) -> Vec<PathBuf> {
        self.groups.first().map(PreparedGlobReference::roots).unwrap_or_default()
    }

    /// Whether every group would list the existing file `path` (see
    /// [`PreparedGlobReference::lists_file`]).
    #[must_use]
    pub fn lists_file(&self, path: &Path) -> bool {
        self.groups.iter().all(|group| group.lists_file(path))
    }

    /// Whether every group could still list a file below `dir` (see
    /// [`PreparedGlobReference::may_list_below`]); `false` lets a walk skip
    /// `dir`.
    #[must_use]
    pub fn may_list_below(&self, dir: &PathIdentity) -> bool {
        self.groups.iter().all(|group| group.may_list_below(dir))
    }
}

/// Why `patterns` cannot be a `match()` constraint: the index of the
/// offending pattern (`None` when the list as a whole is at fault) and the
/// cause. `None` when every pattern is a valid glob reference.
pub(crate) fn definition_error(patterns: &[String]) -> Option<(Option<usize>, GlobReferenceError)> {
    for (index, pattern) in patterns.iter().enumerate() {
        match GlobReference::new([pattern]) {
            Ok(_) | Err(GlobReferenceError::NoPositivePattern) => {}
            Err(error) => return Some((Some(index), error)),
        }
    }
    FileMatchGlobs::new(patterns).err().map(|error| (None, error))
}

/// The `match` groups a simplified root-union arm enforces on a string
/// value of its top-level `property`, one entry per file alternative the
/// value could be judged by; each entry holds that alternative's non-empty
/// groups in authored order.
///
/// The value satisfies the arm's file rule when **some** alternative admits
/// it, and an alternative admits it when **every** one of its groups does:
/// the arm's validator lowers alternatives to `anyOf` and an alternative's
/// groups to one keyword. The same holds for each string item of an array
/// property, whose items are read the same way.
///
/// The alternatives are [`file_alternatives`]'s, through `definitions` (the
/// table the arm was resolved against), so this agrees with the arm's
/// lowering: a property that is directly a `file` is judged, and so is every
/// file reached through a recursive reference; a file alternative written in
/// the arm itself (`file(match(...)) | number`) only suggests.
///
/// `None` when the property is absent, reaches no `match`, or offers some
/// alternative that may accept the string without an enforced glob (`file`
/// without `match`, a suggestion-only file alternative, `string`, `unknown`,
/// or any other string-accepting type, or an unresolved import): then no
/// glob can rule the arm out. A single schema does not use this arm-level
/// constraint.
#[must_use]
pub fn root_union_match_groups<'a>(
    arm: &'a SchemaArm,
    property: &str,
    definitions: &'a DefinitionTable,
) -> Option<Vec<Vec<&'a [String]>>> {
    let SchemaArm::Inline(shape) = arm else {
        return None;
    };
    let alternatives = file_alternatives(shape.properties.get(property)?, definitions)?;
    let groups: Vec<_> = alternatives.iter().map(FileAlternative::match_groups).collect();
    (!groups.is_empty() && groups.iter().all(|groups| !groups.is_empty())).then_some(groups)
}

/// A `file` alternative that a property's string value, or each string item
/// of its list value, is judged by.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FileAlternative<'a> {
    /// The `file` atom; its constraints carry the alternative's timing and
    /// `match` groups.
    pub atom: &'a PropertyAtom,
    /// Whether the property's value is a list of such files: the atom is a
    /// list, or it is reached below a list (`node[]`).
    pub is_array: bool,
}

impl<'a> FileAlternative<'a> {
    /// The alternative's non-empty `match` groups, in authored order.
    #[must_use]
    pub fn match_groups(&self) -> Vec<&'a [String]> {
        self.atom
            .constraints
            .iter()
            .filter_map(|constraint| match constraint {
                Constraint::Match(patterns) if !patterns.is_empty() => Some(patterns.as_slice()),
                _ => None,
            })
            .collect()
    }

    /// Whether the alternative is marked `eager`, on its items or on the list
    /// it is written as (`file(eager)[]`, `file[](eager)`).
    #[must_use]
    pub fn is_eager(&self) -> bool {
        self.atom
            .constraints
            .iter()
            .chain(&self.atom.array_constraints)
            .any(|constraint| matches!(constraint, Constraint::Eager))
    }
}

/// The `file` alternatives a string value of the property `definition`, or
/// each string item of a list value, is judged by, in authored order.
///
/// Two kinds of property are read, and they are the two a root-union arm
/// enforces `match(...)` on:
///
/// - a property that is directly a `file` (or `file[]`): that one atom;
/// - a property whose alternatives reach [`TypeExpr::Ref`] uses of recursive
///   definitions, through property-level unions and union-typed atoms:
///   inside a definition the walk follows what a validator follows (unions
///   and further references through `definitions`, the table `definition`
///   was resolved against), each definition entered once per list depth, so
///   recursion terminates.
///
/// Alternatives that never hold a string or a string item (`number`,
/// `boolean`, `null`, `object`, inline objects, tuples, functions, and a list
/// nested in a list) are skipped, so the result may be empty.
///
/// `None` when some alternative may accept a string other than as a file
/// (`string`, `unknown`, or any other string-accepting type, or an unresolved
/// import or reference), or when the property itself offers a `file`
/// alternative beside others (`file | number`, an acyclic name bound to such
/// a union, `(file | number)[]`). Those acyclic shapes were never read as a
/// file before recursive types existed, and their `match(...)` only
/// suggests, so they keep that behavior.
#[must_use]
pub fn file_alternatives<'a>(
    definition: &'a PropertyDef,
    definitions: &'a DefinitionTable,
) -> Option<Vec<FileAlternative<'a>>> {
    if let PropertyDef::Single(atom) = definition
        && matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File))
    {
        return Some(vec![FileAlternative { atom, is_array: atom.is_array }]);
    }
    let mut alternatives = Vec::new();
    let mut entered = std::collections::HashSet::new();
    collect_reference_alternatives(alternatives_of(definition), false, definitions, &mut entered, &mut alternatives)?;
    Some(alternatives)
}

/// [`collect_file_alternatives`] for the alternatives written on the property
/// itself: only a reference is entered, and a `file` here is a
/// suggestion-only alternative, so the property is not read as a file. A
/// reference to a shared acyclic definition stands for its inlined value, so
/// its body's alternatives are read as if written here.
fn collect_reference_alternatives<'a>(
    atoms: &'a [PropertyAtom],
    in_array: bool,
    definitions: &'a DefinitionTable,
    entered: &mut std::collections::HashSet<(&'a DefKey, bool)>,
    alternatives: &mut Vec<FileAlternative<'a>>,
) -> Option<()> {
    for atom in atoms {
        if in_array && atom.is_array {
            continue;
        }
        let is_array = in_array || atom.is_array;
        match &atom.ty {
            TypeExpr::Ref(key) if !definitions.is_cyclic(key) => {
                let body = &definitions.get(key)?.body;
                collect_reference_alternatives(alternatives_of(body), is_array, definitions, entered, alternatives)?;
            }
            TypeExpr::Ref(_) => collect_file_alternatives(
                std::slice::from_ref(atom),
                in_array,
                definitions,
                entered,
                alternatives,
            )?,
            TypeExpr::Union(arms) => {
                collect_reference_alternatives(arms, is_array, definitions, entered, alternatives)?;
            }
            TypeExpr::Primitive(
                SimplifiedType::Number | SimplifiedType::Boolean | SimplifiedType::Null | SimplifiedType::Object,
            )
            | TypeExpr::InlineObject(_)
            | TypeExpr::Tuple(_)
            | TypeExpr::Function(_) => {}
            TypeExpr::Primitive(_) | TypeExpr::Imported { .. } => return None,
        }
    }
    Some(())
}

/// The atoms a definition offers as alternatives.
fn alternatives_of(definition: &PropertyDef) -> &[PropertyAtom] {
    match definition {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(atoms) => atoms,
    }
}

/// Adds the file alternatives among `atoms`, reached below a list when
/// `in_array`, to `alternatives`; `None` when some alternative may accept a
/// string other than as a file.
fn collect_file_alternatives<'a>(
    atoms: &'a [PropertyAtom],
    in_array: bool,
    definitions: &'a DefinitionTable,
    entered: &mut std::collections::HashSet<(&'a DefKey, bool)>,
    alternatives: &mut Vec<FileAlternative<'a>>,
) -> Option<()> {
    for atom in atoms {
        if in_array && atom.is_array {
            continue;
        }
        let is_array = in_array || atom.is_array;
        match &atom.ty {
            TypeExpr::Primitive(SimplifiedType::File) => alternatives.push(FileAlternative { atom, is_array }),
            TypeExpr::Ref(key) => {
                // A definition entered once at this depth has given all its
                // alternatives, and a cycle ends here.
                if entered.insert((key, is_array)) {
                    let body = &definitions.get(key)?.body;
                    collect_file_alternatives(alternatives_of(body), is_array, definitions, entered, alternatives)?;
                }
            }
            TypeExpr::Union(arms) => collect_file_alternatives(arms, is_array, definitions, entered, alternatives)?,
            TypeExpr::Primitive(
                SimplifiedType::Number | SimplifiedType::Boolean | SimplifiedType::Null | SimplifiedType::Object,
            )
            | TypeExpr::InlineObject(_)
            | TypeExpr::Tuple(_)
            | TypeExpr::Function(_) => {}
            TypeExpr::Primitive(_) | TypeExpr::Imported { .. } => return None,
        }
    }
    Some(())
}

/// Every non-empty `match` group of a `file` property, in authored order;
/// `None` when it has none.
pub fn file_match_groups(definition: &PropertyDef) -> Option<Vec<&[String]>> {
    let PropertyDef::Single(atom) = definition else {
        return None;
    };
    if !matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File)) {
        return None;
    }
    let groups: Vec<&[String]> = atom
        .constraints
        .iter()
        .filter_map(|constraint| match constraint {
            Constraint::Match(patterns) if !patterns.is_empty() => Some(patterns.as_slice()),
            _ => None,
        })
        .collect();
    (!groups.is_empty()).then_some(groups)
}

/// Whether a caller's file value satisfies every one of an arm's `groups`,
/// resolved and judged from the caller's own `origin`.
///
/// The same judgment the [`DARKMATTER_MATCH_KEYWORD`] validator makes: a value
/// that names no existing file, or still holds template or shell syntax, is
/// not judged. A group that is not a valid list of glob references admits
/// every value, as it offers none; the schema definition check reports it.
#[must_use]
pub fn file_match_admits<G: AsRef<[String]>>(
    value: &str,
    groups: &[G],
    origin: &FileResolutionContext,
) -> bool {
    let valid: Vec<FileMatchGlobs> = groups
        .iter()
        .filter_map(|group| FileMatchGlobs::new(group.as_ref()).ok())
        .collect();
    valid.is_empty() || admits(&valid, value, None, None, origin)
}

/// Resolution matches the `darkmatter-file` format's: from `base_dir`, else
/// the context's `cwd`. That directory is also the value's `cwd` for the
/// globs. Every group must accept the resolved path.
fn admits(
    groups: &[FileMatchGlobs],
    value: &str,
    base_dir: Option<&Path>,
    fallback: Option<&Path>,
    context: &FileResolutionContext,
) -> bool {
    if crate::markdown::literal_token::holds_pending_syntax(value) {
        return true;
    }
    let Ok(path) = resolve_file_reference_in_context(value, base_dir, fallback, context) else {
        return true;
    };
    let value_context = match base_dir {
        Some(base_dir) => {
            crate::markdown::compose::expression::resolve_ctx::document_file_context(base_dir, context)
        }
        None => context.clone(),
    };
    groups.iter().all(|group| group.matches(&path, &value_context))
}

type KeywordResult<'a> = Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>>;

/// Factory for [`DARKMATTER_MATCH_KEYWORD`], judging values in the
/// validator's context (see [`admits`]); a property a caller supplied is
/// judged in the context it was authored in. A structural validator judges
/// only an absolute path naming an existing file, by its full path
/// ([`GlobReference::matches_without_context`]), and admits every value whose
/// meaning needs a context. Root-union coercion relies on that to pick an
/// arm by glob.
pub(crate) fn match_keyword_factory(
    file_values: super::validate::FileValues,
) -> impl for<'a> Fn(&'a Map<String, Value>, &'a Value, Location) -> KeywordResult<'a>
+ Send
+ Sync
+ 'static {
    move |parent, schema, location| {
        let groups = decode_match_groups(schema)?;
        let property = match parent.get(DARKMATTER_MATCH_PROPERTY_KEYWORD) {
            Some(named) => named.as_str().map(str::to_string),
            None => top_level_property(&location),
        };
        let globs = FileMatchGroups::new(&groups).map_err(|error| {
            ValidationError::schema(format!("{DARKMATTER_MATCH_KEYWORD} holds an invalid glob: {error}"))
        })?;
        Ok(Box::new(MatchKeyword {
            globs,
            groups,
            judged_in: JudgedIn::for_keyword(&file_values, property.as_deref()),
        }))
    }
}

/// The groups of a [`DARKMATTER_MATCH_KEYWORD`] value (see the module's
/// "Keyword encoding"). A flat array, even an empty one, is one group, so
/// its pattern-level errors read as they did before groups existed.
fn decode_match_groups(schema: &Value) -> Result<Vec<Vec<String>>, ValidationError<'static>> {
    let not_strings =
        || ValidationError::schema(format!("{DARKMATTER_MATCH_KEYWORD} must be an array of strings"));
    let items = schema.as_array().ok_or_else(not_strings)?;
    if !items.iter().any(Value::is_array) {
        let patterns = items
            .iter()
            .map(|item| item.as_str().map(String::from))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(not_strings)?;
        return Ok(vec![patterns]);
    }
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let group = item.as_array().ok_or_else(|| {
                ValidationError::schema(format!(
                    "{DARKMATTER_MATCH_KEYWORD} must not mix patterns and pattern groups"
                ))
            })?;
            let patterns = group
                .iter()
                .map(|pattern| pattern.as_str().map(String::from))
                .collect::<Option<Vec<_>>>()
                .filter(|patterns| !patterns.is_empty());
            patterns.ok_or_else(|| {
                ValidationError::schema(format!(
                    "{DARKMATTER_MATCH_KEYWORD} group {index} must be a non-empty array of strings"
                ))
            })
        })
        .collect()
}

/// Where a [`MatchKeyword`] judges its values.
enum JudgedIn {
    /// No context: only an existing absolute path is judged.
    Syntax,
    /// Resolved like the `darkmatter-file` format, from `base_dir` (else the
    /// context's `cwd`), and judged from that directory.
    Resolved {
        base_dir: Option<PathBuf>,
        fallback: Option<PathBuf>,
        context: Box<FileResolutionContext>,
    },
}

impl JudgedIn {
    /// A caller-supplied property is judged in the context it was authored
    /// in; its value is already projected from there.
    /// `property` is the top-level property the keyword judges, if any; a
    /// whole-instance origin ([`super::validate::CallerOrigins::whole_instance`])
    /// applies whatever it names.
    fn for_keyword(file_values: &super::validate::FileValues, property: Option<&str>) -> Self {
        match file_values {
            super::validate::FileValues::Syntax => Self::Syntax,
            super::validate::FileValues::Resolved { base_dir, fallback, context, callers } => {
                match callers.origin_at(property) {
                    super::validate::ValueOrigin::Caller(origin) => Self::Resolved {
                        base_dir: None,
                        fallback: None,
                        context: Box::new(origin.clone()),
                    },
                    super::validate::ValueOrigin::Document => Self::Resolved {
                        base_dir: base_dir.clone(),
                        fallback: fallback.clone(),
                        context: context.clone(),
                    },
                }
            }
        }
    }
}

/// The top-level property a keyword's schema location belongs to: the
/// segment after the location's first `properties`. Inside a root-union arm
/// that is the arm's property; an enforcing copy under `$defs` names its
/// property with [`DARKMATTER_MATCH_PROPERTY_KEYWORD`] instead.
fn top_level_property(location: &Location) -> Option<String> {
    let pointer = location.as_str();
    let mut segments = pointer.split('/').skip(1);
    segments.by_ref().find(|segment| *segment == "properties")?;
    segments
        .next()
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
}

struct MatchKeyword {
    globs: FileMatchGroups,
    groups: Vec<Vec<String>>,
    judged_in: JudgedIn,
}

impl MatchKeyword {
    fn check(&self, value: &str) -> bool {
        match &self.judged_in {
            JudgedIn::Syntax => super::format::context_free_path(value)
                .filter(|path| path.exists())
                .is_none_or(|path| {
                    self.globs.groups.iter().all(|group| group.globs.matches_without_context(&path))
                }),
            JudgedIn::Resolved { base_dir, fallback, context } => {
                admits(&self.globs.groups, value, base_dir.as_deref(), fallback.as_deref(), context)
            }
        }
    }

    /// Every group as `` `match(…)` ``, joined as an English list.
    fn describe(&self) -> String {
        let described: Vec<String> =
            self.groups.iter().map(|group| format!("`match({})`", group.join(", "))).collect();
        match described.as_slice() {
            [only] => only.clone(),
            [first, second] => format!("{first} and {second}"),
            [init @ .., last] => format!("{}, and {last}", init.join(", ")),
            [] => String::new(),
        }
    }
}

impl<'i> Keyword<'i> for MatchKeyword {
    fn validate(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        match instance {
            Value::String(value) if !self.check(value) => Err(ValidationError::custom(format!(
                "`{value}` is outside this schema arm's {}",
                self.describe()
            ))),
            _ => Ok(()),
        }
    }

    fn is_valid(&self, instance: &'i Value) -> bool {
        match instance {
            Value::String(value) => self.check(value),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::simplified::parse_yaml_schema;

    fn union(yaml: &str) -> Vec<SchemaArm> {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
        match parse_yaml_schema(&value).unwrap() {
            crate::markdown::schemas::SimplifiedSchema::Union(arms) => arms,
            other => panic!("expected a union, got {other:?}"),
        }
    }

    #[test]
    fn every_declared_root_union_glob_constrains_its_arm() {
        let arms = union(
            "- {kind: 'literal(feature)', spec: 'file(eager;match(**/features/**/spec.md))', plan: 'file(match(**/*plan*.md))', solo: 'file(match(*.md))'}\n\
             - {kind: 'literal(fix)', spec: 'file(eager;match(**/fixes/**/spec.md))', plan: 'file(match(**/*plan*.md))'}\n",
        );
        let none = DefinitionTable::default();
        assert_eq!(
            root_union_match_groups(&arms[0], "spec", &none),
            Some(vec![vec![&["**/features/**/spec.md".to_string()][..]]])
        );
        assert_eq!(
            root_union_match_groups(&arms[1], "spec", &none),
            Some(vec![vec![&["**/fixes/**/spec.md".to_string()][..]]])
        );
        assert_eq!(
            root_union_match_groups(&arms[0], "plan", &none),
            Some(vec![vec![&["**/*plan*.md".to_string()][..]]])
        );
        assert_eq!(root_union_match_groups(&arms[0], "solo", &none), Some(vec![vec![&["*.md".to_string()][..]]]));
        assert_eq!(root_union_match_groups(&arms[1], "solo", &none), None);
        assert_eq!(root_union_match_groups(&arms[0], "kind", &none), None);
    }

    #[test]
    fn conversion_emits_every_root_union_glob() {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(
            "- {spec: 'file(eager;match(**/features/**/spec.md))', plan: 'file(match(*.md))[]'}\n\
             - {spec: 'file(eager;match(**/fixes/**/spec.md))', plan: 'file(match(*.md))[]'}\n",
        )
        .unwrap();
        let json =
            crate::markdown::schemas::to_json_schema(&parse_yaml_schema(&value).unwrap()).unwrap();
        let spec = &json["anyOf"][1]["properties"]["spec"];
        let file_arm = spec["anyOf"]
            .as_array()
            .and_then(|arms| arms.iter().find(|arm| arm.get("format").is_some()))
            .unwrap_or(spec);
        assert_eq!(file_arm[DARKMATTER_MATCH_KEYWORD], serde_json::json!(["**/fixes/**/spec.md"]));
        for arm in &json["anyOf"].as_array().unwrap()[..2] {
            let plan = &arm["properties"]["plan"];
            assert!(
                plan.to_string().contains("\"x-darkmatter-match\":[\"*.md\"]"),
                "the identical array glob must constrain each arm: {plan}"
            );
        }
        let single: serde_yaml_ng::Value =
            serde_yaml_ng::from_str("{spec: 'file(eager;match(**/fixes/**/spec.md))'}").unwrap();
        let single =
            crate::markdown::schemas::to_json_schema(&parse_yaml_schema(&single).unwrap()).unwrap();
        assert!(!single.to_string().contains(DARKMATTER_MATCH_KEYWORD));
    }

    #[test]
    fn a_glob_against_an_unglobbed_declaration_constrains_its_arm() {
        let arms = union(
            "- {spec: 'file(match(**/fixes/**/spec.md))'}\n\
             - {spec: 'string'}\n",
        );
        let none = DefinitionTable::default();
        assert!(root_union_match_groups(&arms[0], "spec", &none).is_some());
        assert_eq!(root_union_match_groups(&arms[1], "spec", &none), None);
    }

    #[test]
    fn globs_judge_a_path_relative_to_the_value_cwd() {
        let globs = FileMatchGlobs::new(&["fixes/**/spec.md".to_string()]).unwrap();
        let launch = tempfile::tempdir().unwrap();
        let context = FileResolutionContext::new(launch.path());
        assert!(globs.matches(Path::new("fixes/x/spec.md"), &context));
        assert!(globs.matches(&launch.path().join("fixes").join("x").join("spec.md"), &context));
        assert!(!globs.matches(Path::new("features/x/spec.md"), &context));
    }

    #[test]
    fn a_missing_file_is_never_judged() {
        let dir = tempfile::tempdir().unwrap();
        let origin = FileResolutionContext::new(dir.path());
        let patterns = ["**/fixes/**/spec.md".to_string()];
        assert!(file_match_admits("partial", &[&patterns[..]], &origin));
        std::fs::create_dir_all(dir.path().join("features/x")).unwrap();
        std::fs::write(dir.path().join("features/x/spec.md"), "").unwrap();
        assert!(!file_match_admits("features/x/spec.md", &[&patterns[..]], &origin));
        std::fs::create_dir_all(dir.path().join("fixes/x")).unwrap();
        std::fs::write(dir.path().join("fixes/x/spec.md"), "").unwrap();
        assert!(file_match_admits("fixes/x/spec.md", &[&patterns[..]], &origin));
    }

    fn strings(patterns: &[&str]) -> Vec<String> {
        patterns.iter().map(|pattern| (*pattern).to_string()).collect()
    }

    /// A canonical tempdir holding `files` (empty contents) and its context.
    fn tree(files: &[&str]) -> (tempfile::TempDir, PathBuf, FileResolutionContext) {
        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        for file in files {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "").unwrap();
        }
        let context = FileResolutionContext::new(&root);
        (dir, root, context)
    }

    /// The walk Claudine's candidate listing makes: the roots, pruned by
    /// `may_list_below`, keeping what `lists_file` accepts. Returns names
    /// relative to `root`, with `/`, sorted.
    fn prepared_walk(prepared: &PreparedFileMatch, root: &Path) -> Vec<String> {
        fn visit(prepared: &PreparedFileMatch, dir: &Path, out: &mut Vec<PathBuf>) {
            if !prepared.may_list_below(&PathIdentity::new(dir)) {
                return;
            }
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                let kind = entry.file_type().unwrap();
                if kind.is_dir() {
                    visit(prepared, &path, out);
                } else if kind.is_file() && prepared.lists_file(&path) && !out.contains(&path) {
                    out.push(path);
                }
            }
        }
        let mut found = Vec::new();
        for walk_root in prepared.roots() {
            visit(prepared, &walk_root, &mut found);
        }
        let mut names: Vec<String> = found
            .iter()
            .map(|path| biscuit_file::to_portable_string(path.strip_prefix(root).unwrap()))
            .collect();
        names.sort();
        names
    }

    #[test]
    fn one_group_judges_exactly_as_its_globs() {
        let files = ["a.md", "_draft.md", "notes.txt", "docs/b.md", "docs/_c.md", "src/d.rs"];
        let (_dir, root, context) = tree(&files);
        let group = strings(&["*.md", "!_*.md", "src/**/*.rs"]);
        let globs = FileMatchGlobs::new(&group).unwrap();
        let groups = FileMatchGroups::new(&[&group]).unwrap();
        assert_eq!(groups.groups().len(), 1);

        let prepared_globs = globs.prepare(&context).unwrap();
        let prepared_groups = groups.prepare(&context).unwrap();
        assert_eq!(prepared_groups.roots(), prepared_globs.roots());
        for file in files.iter().chain(&["missing.md", "docs"]) {
            let path = root.join(file);
            assert_eq!(groups.matches(&path, &context), globs.matches(&path, &context), "matches {file}");
            assert_eq!(
                groups.matches(Path::new(file), &context),
                globs.matches(Path::new(file), &context),
                "relative matches {file}"
            );
            assert_eq!(prepared_groups.lists_file(&path), prepared_globs.lists_file(&path), "lists {file}");
            let identity = PathIdentity::new(&path);
            assert_eq!(
                prepared_groups.may_list_below(&identity),
                prepared_globs.may_list_below(&identity),
                "below {file}"
            );
            let single_admits = file_match_admits(file, &[&group], &context);
            assert_eq!(
                single_admits,
                FileMatchGlobs::new(&group).is_ok_and(|globs| admits(&[globs], file, None, None, &context)),
                "admits {file}"
            );
        }
        assert_eq!(prepared_walk(&prepared_groups, &root), ["a.md", "docs/b.md", "src/d.rs"]);
    }

    #[test]
    fn one_group_takes_several_positives_and_exclusions() {
        let (_dir, root, context) = tree(&["a.md", "b.txt", "_a.md", "keep.log", "skip.log", "c.rs"]);
        let groups = FileMatchGroups::new(&[strings(&["*.md", "*.txt", "*.log", "!_*", "!skip.*"])]).unwrap();
        let prepared = groups.prepare(&context).unwrap();
        assert_eq!(prepared_walk(&prepared, &root), ["a.md", "b.txt", "keep.log"]);
        for (value, admitted) in
            [("a.md", true), ("b.txt", true), ("keep.log", true), ("_a.md", false), ("skip.log", false), ("c.rs", false)]
        {
            assert_eq!(groups.matches(&root.join(value), &context), admitted, "matches {value}");
            assert_eq!(
                file_match_admits(value, &[strings(&["*.md", "*.txt", "*.log", "!_*", "!skip.*"])], &context),
                admitted,
                "admits {value}"
            );
        }
    }

    #[test]
    fn groups_intersect_instead_of_widening() {
        let (_dir, root, context) = tree(&["guide-intro.md", "notes.md", "guide.txt", "docs/guide-two.md"]);
        let authored = [strings(&["*.md"]), strings(&["guide*"])];
        let groups = FileMatchGroups::new(&authored).unwrap();

        assert!(groups.matches(&root.join("guide-intro.md"), &context));
        assert!(!groups.matches(&root.join("notes.md"), &context), "only the first group admits it");
        assert!(!groups.matches(&root.join("guide.txt"), &context), "only the second group admits it");

        assert!(file_match_admits("guide-intro.md", &authored, &context));
        assert!(!file_match_admits("notes.md", &authored, &context));
        assert!(!file_match_admits("guide.txt", &authored, &context));
        assert!(file_match_admits("guide-missing.txt", &authored, &context), "a missing file is never judged");

        let prepared = groups.prepare(&context).unwrap();
        assert_eq!(prepared_walk(&prepared, &root), ["docs/guide-two.md", "guide-intro.md"]);
    }

    #[test]
    fn an_exclusion_stays_inside_its_own_group() {
        let (_dir, root, context) = tree(&["guide-a.md", "guide-b.md", "notes.md"]);
        // The second group's `!guide-b*` must not reach the first group's
        // positives, and the first group's `!notes*` excludes from it alone.
        let authored = [strings(&["*.md", "!notes*"]), strings(&["guide*", "!guide-b*"])];
        let groups = FileMatchGroups::new(&authored).unwrap();
        assert!(groups.matches(&root.join("guide-a.md"), &context));
        assert!(!groups.matches(&root.join("guide-b.md"), &context));
        assert!(!groups.matches(&root.join("notes.md"), &context));

        // The same exclusion in a separate group leaves the first group's
        // positive `*.md` intact for every name the exclusion does not name.
        let separate = [strings(&["*.md"]), strings(&["*", "!notes*"])];
        assert!(file_match_admits("guide-b.md", &separate, &context));
        assert!(!file_match_admits("notes.md", &separate, &context));
        let prepared = FileMatchGroups::new(&separate).unwrap().prepare(&context).unwrap();
        assert_eq!(prepared_walk(&prepared, &root), ["guide-a.md", "guide-b.md"]);
    }

    #[test]
    fn an_empty_intersection_constructs_and_prepares_without_a_walk() {
        let authored = [strings(&["*.md"]), strings(&["*.txt"])];
        // Construction takes no context, so it has no directory to read.
        let groups = FileMatchGroups::new(&authored).unwrap();

        // Preparation against a directory that does not exist yet: had it
        // listed any directory to decide anything, there would have been
        // nothing to list, yet it yields a usable judgment, which then reads
        // files created only afterwards. The prepared value therefore holds
        // resolved roots and patterns, not a snapshot of a walk.
        let parent = tempfile::tempdir().unwrap();
        let later = biscuit_file::canonicalize_simplified(parent.path()).unwrap().join("later");
        let context = FileResolutionContext::new(&later);
        let prepared = groups.prepare(&context).expect("a lexically valid context");
        let one_group = FileMatchGroups::new(&authored[..1]).unwrap().prepare(&context).unwrap();
        assert_eq!(prepared.roots(), one_group.roots());

        std::fs::create_dir_all(&later).unwrap();
        for file in ["a.md", "b.txt", "c.md.txt"] {
            std::fs::write(later.join(file), "").unwrap();
        }
        assert!(one_group.lists_file(&later.join("a.md")), "a file created after preparation is judged");
        assert_eq!(prepared_walk(&prepared, &later), Vec::<String>::new());
        for file in ["a.md", "b.txt", "c.md.txt"] {
            assert!(!groups.matches(&later.join(file), &context), "{file}");
            assert!(!file_match_admits(file, &authored, &context), "{file}");
        }
    }

    #[test]
    fn an_empty_or_invalid_group_list_is_an_error() {
        let no_groups: [Vec<String>; 0] = [];
        assert!(matches!(FileMatchGroups::new(&no_groups), Err(GlobReferenceError::NoPositivePattern)));
        assert!(FileMatchGroups::new(&[strings(&["*.md"]), strings(&["!x"])]).is_err());
        assert!(FileMatchGroups::new(&[strings(&["*.md"]), strings(&["[x"])]).is_err());
    }

    #[test]
    fn root_union_match_groups_keeps_every_group_in_authored_order() {
        let arms = union("- {spec: 'file(match(*.md))'}\n- {spec: string}\n");
        let SchemaArm::Inline(shape) = &arms[0] else { panic!("an inline arm") };
        let mut shape = shape.clone();
        let Some(PropertyDef::Single(atom)) = shape.properties.get_mut("spec") else {
            panic!("a single file atom")
        };
        atom.constraints.push(Constraint::Match(strings(&["guide*", "!guide-old*"])));
        atom.constraints.push(Constraint::Match(Vec::new()));
        let arm = SchemaArm::Inline(shape);
        let none = DefinitionTable::default();
        let alternatives = root_union_match_groups(&arm, "spec", &none).unwrap();
        assert_eq!(alternatives, [vec![&strings(&["*.md"])[..], &strings(&["guide*", "!guide-old*"])[..]]]);
    }

    /// The first arm of `$schema` resolved in a temporary folder, with the
    /// definition table its references name.
    fn resolved_first_arm(yaml: &str) -> (SchemaArm, std::sync::Arc<DefinitionTable>) {
        let dir = tempfile::tempdir().unwrap();
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
        let resolved = crate::markdown::schemas::resolve::resolve_yaml_schema(
            &value,
            dir.path(),
            &FileResolutionContext::new(dir.path()),
        )
        .unwrap();
        let Some(crate::markdown::schemas::SimplifiedSchema::Union(mut arms)) = resolved.simplified else {
            panic!("expected a root union");
        };
        (arms.remove(0), resolved.definitions)
    }

    #[test]
    fn root_union_match_groups_follow_a_recursive_reference() {
        let (arm, definitions) = resolved_first_arm(
            "- node: \"file(match('./notes/*.md')) | { children: node[] }\"\n  spec: node(required)\n\
             - spec: number\n",
        );
        assert!(!definitions.is_empty(), "node recurses, so it stays a reference");
        assert_eq!(root_union_match_groups(&arm, "spec", &definitions), Some(vec![vec![&strings(&["./notes/*.md"])[..]]]));
        assert_eq!(
            root_union_match_groups(&arm, "spec", &DefinitionTable::default()),
            None,
            "a reference the table does not hold offers no glob"
        );
    }

    /// A file alternative written in the arm itself only suggests, as before
    /// recursive types existed, so it never rules the arm out; a reference
    /// beside a non-string alternative is still judged through its copy.
    #[test]
    fn root_union_match_groups_leave_a_file_alternative_of_a_property_union_unjudged() {
        for suggestion_only in [
            "file(match('./notes/*.md')) | number",
            "file(match('./notes/*.md')) | file(match('./guides/*.md'))",
            "file(match('./notes/*.md')) | string",
            "file(match('./notes/*.md')) | file",
            "(file(match('./notes/*.md')) | number)[]",
        ] {
            let (arm, definitions) = resolved_first_arm(&format!("- spec: \"{suggestion_only}\"\n- spec: number\n"));
            assert_eq!(root_union_match_groups(&arm, "spec", &definitions), None, "`{suggestion_only}`");
        }
        let (arm, definitions) = resolved_first_arm(
            "- choice: \"file(match('./notes/*.md')) | number\"\n  spec: choice\n- spec: number\n",
        );
        assert_eq!(root_union_match_groups(&arm, "spec", &definitions), None, "an acyclic name bound to the union");

        let node = "- node: \"file(match('./notes/*.md')) | { children: node[] }\"\n";
        let (arm, definitions) = resolved_first_arm(&format!("{node}  spec: node | number\n- spec: number\n"));
        assert_eq!(root_union_match_groups(&arm, "spec", &definitions), Some(vec![vec![&strings(&["./notes/*.md"])[..]]]));
        let (arm, definitions) =
            resolved_first_arm(&format!("{node}  spec: \"node | file(match('./guides/*.md'))\"\n- spec: number\n"));
        assert_eq!(
            root_union_match_groups(&arm, "spec", &definitions),
            None,
            "the arm's own file alternative accepts a string with no enforced glob"
        );
    }

    #[test]
    fn root_union_match_groups_read_an_acyclic_named_file_type() {
        let (arm, definitions) =
            resolved_first_arm("- note: \"file(match('./notes/*.md'))\"\n  spec: note(required)\n- spec: number\n");
        assert_eq!(root_union_match_groups(&arm, "spec", &definitions), Some(vec![vec![&strings(&["./notes/*.md"])[..]]]));
    }

    #[test]
    fn root_union_match_groups_read_a_direct_file_property() {
        let (arm, definitions) = resolved_first_arm("- spec: \"file(match('./notes/*.md'))\"\n- spec: number\n");
        assert_eq!(root_union_match_groups(&arm, "spec", &definitions), Some(vec![vec![&strings(&["./notes/*.md"])[..]]]));
    }

    fn keyword_schema(keyword: Value) -> Value {
        serde_json::json!({ "type": "object", "properties": { "spec": { "type": "string", DARKMATTER_MATCH_KEYWORD: keyword } } })
    }

    fn build(keyword: Value, context: &FileResolutionContext) -> Result<jsonschema::Validator, String> {
        crate::markdown::schemas::validate::build_validator_in_context(&keyword_schema(keyword), None, None, context)
            .map_err(|error| error.to_string())
    }

    fn rejection(validator: &jsonschema::Validator, value: &str) -> Option<String> {
        let instance = serde_json::json!({ "spec": value });
        let error = validator.iter_errors(&instance).next()?;
        assert!(
            error.schema_path().as_str().ends_with(DARKMATTER_MATCH_KEYWORD),
            "the error must come from the keyword itself: {}",
            error.schema_path().as_str()
        );
        Some(error.to_string())
    }

    #[test]
    fn a_flat_keyword_is_one_group_with_its_message_unchanged() {
        let (_dir, _root, context) = tree(&["a.md", "_a.md", "b.txt"]);
        let validator = build(serde_json::json!(["*.md", "!_*"]), &context).unwrap();
        assert_eq!(rejection(&validator, "a.md"), None);
        assert_eq!(rejection(&validator, "missing.txt"), None);
        assert_eq!(
            rejection(&validator, "b.txt").as_deref(),
            Some("`b.txt` is outside this schema arm's `match(*.md, !_*)`")
        );
        assert!(rejection(&validator, "_a.md").is_some());
    }

    #[test]
    fn a_nested_keyword_requires_every_group() {
        let (dir, root, context) = tree(&["guide-intro.md", "notes.md", "guide.txt"]);
        let keyword = serde_json::json!([["*.md"], ["guide*"]]);
        let validator = build(keyword.clone(), &context).unwrap();
        assert_eq!(rejection(&validator, "guide-intro.md"), None);
        for value in ["notes.md", "guide.txt"] {
            assert_eq!(
                rejection(&validator, value),
                Some(format!("`{value}` is outside this schema arm's `match(*.md)` and `match(guide*)`"))
            );
        }
        let three = build(serde_json::json!([["*.md"], ["guide*"], ["*intro*", "!x"]]), &context).unwrap();
        assert_eq!(
            rejection(&three, "notes.md").as_deref(),
            Some("`notes.md` is outside this schema arm's `match(*.md)`, `match(guide*)`, and `match(*intro*, !x)`")
        );

        // A structural validator judges an existing absolute path by every group.
        let structural =
            crate::markdown::schemas::validate::build_structural_validator(&keyword_schema(keyword)).unwrap();
        let absolute = |name: &str| biscuit_file::to_portable_string(&root.join(name));
        assert!(structural.is_valid(&serde_json::json!({ "spec": absolute("guide-intro.md") })));
        assert!(!structural.is_valid(&serde_json::json!({ "spec": absolute("notes.md") })));
        assert!(!structural.is_valid(&serde_json::json!({ "spec": absolute("guide.txt") })));
        drop(dir);
    }

    #[test]
    fn a_malformed_keyword_is_a_schema_error() {
        let (_dir, _root, context) = tree(&[]);
        for (keyword, fragment) in [
            (serde_json::json!(["*.md", ["guide*"]]), "must not mix patterns and pattern groups"),
            (serde_json::json!([["guide*"], "*.md"]), "must not mix patterns and pattern groups"),
            (serde_json::json!([["*.md"], []]), "group 1 must be a non-empty array of strings"),
            (serde_json::json!([["*.md"], ["x", 1]]), "group 1 must be a non-empty array of strings"),
            (serde_json::json!([["*.md"], ["!x"]]), "invalid glob"),
            (serde_json::json!([]), "invalid glob"),
            (serde_json::json!(["*.md", 1]), "must be an array of strings"),
            (serde_json::json!("*.md"), "must be an array of strings"),
        ] {
            let error = build(keyword.clone(), &context).err().unwrap_or_else(|| panic!("{keyword} must not build"));
            assert!(error.contains(fragment), "{keyword}: `{fragment}` not in {error:?}");
        }
    }
}
