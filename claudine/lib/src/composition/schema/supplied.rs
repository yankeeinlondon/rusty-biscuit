//! Caller-owned eager file inputs that need completion before lifecycle work.

use biscuit_file::{FileReference, FileResolutionContext};
use darkmatter::markdown::compose::CallerInputRecords;
use darkmatter::markdown::schemas::file_match::{
    FileAlternative, file_alternatives, file_match_admits, root_union_match_groups,
};
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, DefinitionTable, EffectiveSchema, PropertyAtom, PropertyDef,
    SchemaArm, SchemaShape, SimplifiedSchema, SimplifiedType, TypeExpr, ValidationProblem,
    to_json_schema_with_definitions,
    select_literal_discriminant_arm,
};

use super::{
    CompositionError, ResolvedCompositionSource, build_effective_instance,
    is_composition_independent, load_authored_schema, top_level_pointer_segment,
    value_needs_composition,
};

/// An unresolved supplied file, retaining its slot when the caller supplied an array.
#[derive(Debug)]
pub struct UnresolvedSuppliedFile {
    /// The existing typed completion signal and actionable failure reason.
    pub error: CompositionError,
    /// The original array slot; `None` also covers scalar shorthand for `file[]`.
    pub array_index: Option<usize>,
}

/// Inspect only caller-owned eager file inputs, without judging the schema.
///
/// Missing values, lazy output files, and unsupported schema shapes are left
/// for canonical preparation. An unavailable schema is likewise deferred so
/// `initialize` can create or repair it. Literal references use each caller
/// record's frozen origin; the document context owns only schema discovery.
pub fn unresolved_supplied_files(
    source: &ResolvedCompositionSource,
    records: &CallerInputRecords,
    document_context: &FileResolutionContext,
) -> Vec<UnresolvedSuppliedFile> {
    if records.is_empty() {
        return Vec::new();
    }
    let Ok(Some(effective)) =
        load_authored_schema(source, None, document_context, records)
    else {
        return Vec::new();
    };
    let Some(arms) = supplied_file_arms(source, records, document_context, &effective) else {
        return Vec::new();
    };
    let mut pending = Vec::new();
    for (name, record) in records {
        let Some(FileTarget { is_array, alternatives }) =
            arms.eager_file_target(name, effective.definitions())
        else {
            continue;
        };
        let values: Vec<_> = match record.raw() {
            serde_json::Value::String(value) => vec![(None, value.as_str())],
            serde_json::Value::Array(values)
                if is_array && values.iter().all(|v| v.is_string()) =>
            {
                values
                    .iter()
                    .enumerate()
                    .filter_map(|(index, value)| {
                        value.as_str().map(|value| (Some(index), value))
                    })
                    .collect()
            }
            _ => continue,
        };
        for (array_index, provided) in values {
            if provided.trim().is_empty()
                || value_needs_composition(Some(&serde_json::Value::String(provided.to_string())))
            {
                continue;
            }
            if !matches!(
                FileReference::new(provided)
                    .and_then(|reference| reference.resolve_in_context(record.origin())),
                Ok(None)
            ) {
                continue;
            }
            pending.push(UnresolvedSuppliedFile {
                error: unresolved_caller_file(
                    &source.resolved_path,
                    name,
                    provided,
                    FileTarget { is_array, alternatives: alternatives.clone() },
                    record.origin(),
                ),
                array_index,
            });
        }
    }
    pending
}

/// The globs a caller-supplied file reference is completed against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FileTarget {
    pub(super) is_array: bool,
    /// Alternatives of `match(...)` groups (see
    /// [`CompositionError::UnresolvedFileReference`]). Never empty, and no
    /// alternative is empty: a bare `file` has no glob to walk.
    pub(super) alternatives: Vec<Vec<Vec<String>>>,
}

impl FileTarget {
    /// Add `other`'s alternatives after this target's, skipping any already
    /// present, when both agree on the array shape (ruling D1). Each arm stays
    /// its own alternative: concatenating arms' globs would let one arm's
    /// exclusions and groups leak into another's.
    fn merge(&mut self, other: Self) -> Option<()> {
        if self.is_array != other.is_array {
            return None;
        }
        for alternative in other.alternatives {
            if !self.alternatives.contains(&alternative) {
                self.alternatives.push(alternative);
            }
        }
        Some(())
    }
}

/// The schema arms a caller-supplied file is judged against.
enum SuppliedArms<'a> {
    /// A single schema, or the one root-union arm the caller's inputs apply to.
    Unique(&'a SchemaShape),
    /// The root-union arms still in contention: those that apply, or every arm
    /// when none does.
    Undecided(Vec<&'a SchemaShape>),
}

impl SuppliedArms<'_> {
    /// Where `name` must be an existing file whichever arm finally applies;
    /// `definitions` is the table the arms' references name.
    ///
    /// An undecided union qualifies only when every contending arm declares
    /// `name` as an eager `file(match)` with the same array shape (ruling D1),
    /// because only then does the existence verdict not depend on the arm. Each
    /// arm contributes its groups as one alternative, de-duplicated in arm
    /// order; the chosen literal path settles the arm afterward.
    fn eager_file_target(&self, name: &str, definitions: &DefinitionTable) -> Option<FileTarget> {
        match self {
            Self::Unique(shape) => eager_file_target(shape, name, definitions),
            Self::Undecided(arms) => {
                let mut merged: Option<FileTarget> = None;
                for arm in arms {
                    let target = eager_file_target(arm, name, definitions)?;
                    match &mut merged {
                        None => merged = Some(target),
                        Some(merged) => merged.merge(target)?,
                    }
                }
                merged
            }
        }
    }
}

/// [`file_match_target`] when every file alternative of `name` is eager.
fn eager_file_target(shape: &SchemaShape, name: &str, definitions: &DefinitionTable) -> Option<FileTarget> {
    let definition = shape.properties.get(name)?;
    let alternatives = file_alternatives(definition, definitions)?;
    if !alternatives.iter().all(FileAlternative::is_eager) {
        return None;
    }
    file_match_target(definition, definitions)
}

/// The `match(...)` groups of a property whose string value can only be a
/// file, whether or not it is eager: the property's own `file` atom, or one
/// alternative per `file` alternative its recursive references reach in
/// `definitions` ([`file_alternatives`]).
///
/// `None` when a string could be something else (including an acyclic union
/// with a `file` alternative, which is not read as a file), when no
/// alternative is a file, when some file alternative has no glob to walk, or
/// when the alternatives disagree on whether the value is a list.
fn file_match_target(definition: &PropertyDef, definitions: &DefinitionTable) -> Option<FileTarget> {
    let alternatives = file_alternatives(definition, definitions)?;
    let is_array = alternatives.first()?.is_array;
    let mut target = FileTarget { is_array, alternatives: Vec::new() };
    for alternative in &alternatives {
        let groups = alternative.match_groups();
        if alternative.is_array != is_array || groups.is_empty() {
            return None;
        }
        let groups: Vec<_> = groups.into_iter().map(<[String]>::to_vec).collect();
        if !target.alternatives.contains(&groups) {
            target.alternatives.push(groups);
        }
    }
    Some(target)
}

/// The typed completion signal for a caller value that names no existing file.
///
/// The reason names the directory the value was resolved from, the caller's
/// origin, so the early pass and the late verdict send the user to the same
/// place (R4).
pub(super) fn unresolved_caller_file(
    source_path: &std::path::Path,
    property: &str,
    provided: &str,
    target: FileTarget,
    origin: &FileResolutionContext,
) -> CompositionError {
    CompositionError::UnresolvedFileReference {
        source_path: source_path.to_path_buf(),
        property: property.to_string(),
        provided: provided.to_string(),
        alternatives: target.alternatives,
        is_array: target.is_array,
        reason: caller_no_match_reason(provided, origin),
    }
}

/// The globs a validator's file-reference verdict on `name` is completed against.
///
/// A single schema reads the property's own `file(match)` declaration. A root
/// union whose literal discriminant in `instance` settles one arm reads only
/// that arm, because a file from another arm's glob would now be rejected by
/// the settled arm's own. Otherwise it reads every arm that declares `name` as
/// a `file(match)`: one such arm is that arm's groups, and several merge by
/// the D1 rule (one alternative per arm, in arm order, de-duplicated)
/// provided they agree on the array shape.
pub(super) fn file_reference_target(
    effective: &EffectiveSchema,
    name: &str,
    instance: &serde_json::Value,
) -> Option<FileTarget> {
    let definitions = effective.definitions();
    let file_atom = |shape: &SchemaShape| {
        shape
            .properties
            .get(name)
            .and_then(|definition| file_match_target(definition, definitions))
    };
    match effective.simplified.as_ref()? {
        SimplifiedSchema::Single(shape) => file_atom(shape),
        SimplifiedSchema::Union(arms) => {
            let json_arms = effective
                .json_schema
                .get("anyOf")
                .and_then(serde_json::Value::as_array)
                .filter(|json_arms| json_arms.len() == arms.len());
            if let Some(settled) = json_arms
                .and_then(|json_arms| select_literal_discriminant_arm(json_arms, instance))
            {
                return match &arms[settled] {
                    SchemaArm::Inline(shape) => file_atom(shape),
                    SchemaArm::FileRef(_) => None,
                };
            }
            let mut merged: Option<FileTarget> = None;
            for target in arms.iter().filter_map(|arm| match arm {
                SchemaArm::Inline(shape) => file_atom(shape),
                SchemaArm::FileRef(_) => None,
            }) {
                match &mut merged {
                    None => merged = Some(target),
                    Some(merged) => merged.merge(target)?,
                }
            }
            merged
        }
    }
}

/// Re-read each "no existing file matched" verdict on a caller-owned value
/// from the caller's own origin (R4).
///
/// A late verdict can resolve a caller value in document context, so its
/// message names the document's directory. For a `NoMatch` problem on a
/// property with a caller record, the raw value the message names is resolved
/// again against
/// [`CallerInputRecord::origin`](darkmatter::markdown::compose::CallerInputRecord::origin).
/// When it is still unresolved, the message is rewritten to the one the early
/// pass uses, naming the caller's base directory. A value that does resolve
/// from the caller's origin leaves its problem untouched: that verdict was
/// about something else. This needs no schema metadata, so it holds for every
/// shape, including a bare `file` and a union whose arms disagree.
pub(super) fn rebase_caller_file_problems(
    problems: &mut [ValidationProblem],
    records: &CallerInputRecords,
) {
    for problem in problems {
        if !problem.message.contains(NO_MATCH) {
            continue;
        }
        let Some(record) =
            top_level_pointer_segment(&problem.path).and_then(|name| records.get(&name))
        else {
            continue;
        };
        let values: Vec<&str> = match record.raw() {
            serde_json::Value::String(value) => vec![value.as_str()],
            serde_json::Value::Array(values) => {
                values.iter().filter_map(serde_json::Value::as_str).collect()
            }
            _ => continue,
        };
        let Some(provided) = values.into_iter().find(|provided| {
            problem.message.contains(&format!("{NO_MATCH} `{provided}`"))
                && matches!(
                    FileReference::new(provided)
                        .and_then(|reference| reference.resolve_in_context(record.origin())),
                    Ok(None)
                )
        }) else {
            continue;
        };
        problem.message = caller_no_match_reason(provided, record.origin());
    }
}

/// The unresolved-reference text, naming the caller's base directory.
fn caller_no_match_reason(provided: &str, origin: &FileResolutionContext) -> String {
    darkmatter::markdown::errors::with_glob_hint(
        format!("{NO_MATCH} `{provided}` while resolving from `{}`", origin.cwd().display()),
        biscuit_file::ResolutionFailure::NoMatch,
        provided,
    )
}

/// Darkmatter's `NoMatch` text: a reference that failed existence resolution,
/// as opposed to a parse or resolution error that a glob walk cannot rescue.
pub(super) const NO_MATCH: &str = "no existing file matched reference";

/// Whether every existing caller file this arm declares satisfies the arm's
/// `match` rule: some file alternative the value could be judged by admits
/// it. The rule is the one the arm's lowering enforces (see
/// [`root_union_match_groups`]): a property that is directly a `file`, or a
/// file reached through a recursive reference in `definitions`; a file
/// alternative written in the arm itself only suggests and rules nothing out.
///
/// Each arm is validated alone below, where Darkmatter cannot tell which of
/// its globs discriminate, so the arm-level rule is applied here through the
/// same judgment Darkmatter's validator makes: a value naming no existing file
/// (a partial the chooser will complete) never rules an arm out.
fn admits_arm_files(arm: &SchemaArm, records: &CallerInputRecords, definitions: &DefinitionTable) -> bool {
    records.iter().all(|(name, record)| {
        let Some(alternatives) = root_union_match_groups(arm, name, definitions) else {
            return true;
        };
        let values: Vec<&str> = match record.raw() {
            serde_json::Value::String(value) => vec![value.as_str()],
            serde_json::Value::Array(values) => {
                values.iter().filter_map(serde_json::Value::as_str).collect()
            }
            _ => Vec::new(),
        };
        values
            .into_iter()
            .all(|value| {
                alternatives
                    .iter()
                    .any(|groups| file_match_admits(value, groups, record.origin()))
            })
    })
}

/// Decide which arms a caller's file inputs are judged against.
///
/// A root union needs a unique applicable arm before its file metadata has
/// meaning. An existing caller file outside the arm's glob rules it out
/// ([`admits_arm_files`]). Two relaxations apply when judging each
/// arm. Existence is relaxed
/// only for caller-owned files, because initialization still owns that verdict.
/// A problem on a value that still needs composition (`{{…}}`/`$(…)`) is
/// ignored, by the same rule the pre-validator uses, because composition may
/// yet make it valid. Nothing else is relaxed: ordinary string alternatives,
/// literal invalid siblings, and conflicting discriminants never select an
/// arm. When no unique arm applies the arms stay [`SuppliedArms::Undecided`],
/// and only the D1 rule in [`SuppliedArms::eager_file_target`] can still ask
/// for a file. Returns `None` for a union with a non-inline arm or a schema
/// that cannot be projected.
fn supplied_file_arms<'a>(
    source: &ResolvedCompositionSource,
    records: &CallerInputRecords,
    document_context: &FileResolutionContext,
    effective: &'a EffectiveSchema,
) -> Option<SuppliedArms<'a>> {
    let arms = match effective.simplified.as_ref()? {
        SimplifiedSchema::Single(shape) => return Some(SuppliedArms::Unique(shape)),
        SimplifiedSchema::Union(arms) => arms,
    };
    let overrides = serde_json::Value::Object(
        records
            .iter()
            .map(|(name, record)| (name.clone(), record.raw().clone()))
            .collect(),
    );
    let instance = build_effective_instance(source, Some(&overrides));
    let mut schema_source = source.markdown.clone();
    schema_source.frontmatter_mut().as_map_mut().shift_remove("$schema");
    let mut shapes = Vec::with_capacity(arms.len());
    let mut applicable = Vec::new();
    for arm in arms {
        let SchemaArm::Inline(shape) = arm else {
            return None;
        };
        shapes.push(shape);
        let mut relaxed = shape.clone();
        let mut candidate = instance.clone();
        for (name, definition) in &mut relaxed.properties {
            let Some(record) = records.get(name) else {
                continue;
            };
            if let PropertyDef::Single(atom) = definition
                && matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File))
            {
                without_eager(atom);
                if atom.is_array && candidate.get(name).is_some_and(serde_json::Value::is_string) {
                    candidate[name] = serde_json::Value::Array(vec![candidate[name].take()]);
                }
                continue;
            }
            if let Some(lazy) = lazy_file_alternatives(definition, record.raw(), effective.definitions()) {
                if lazy.iter().all(|atom| atom.is_array)
                    && candidate.get(name).is_some_and(serde_json::Value::is_string)
                {
                    candidate[name] = serde_json::Value::Array(vec![candidate[name].take()]);
                }
                *definition = match <[PropertyAtom; 1]>::try_from(lazy) {
                    Ok([atom]) => PropertyDef::Single(atom),
                    Err(lazy) => PropertyDef::Union(lazy),
                };
            }
        }
        // The arm's references name the document's recursive definitions, so
        // it lowers against that table; without it an arm reaching a
        // recursive type would not convert and no arm would be selected.
        let relaxed = to_json_schema_with_definitions(
            &SimplifiedSchema::Single(relaxed),
            effective.definitions(),
        )
        .ok()?;
        let schemas = DarkmatterSchemas::new(document_context.clone())
            .with_baseline_json_schema(relaxed)
            .ok()?;
        let projected = schemas.effective_for(&schema_source).ok()??;
        if !admits_arm_files(arm, records, effective.definitions()) {
            continue;
        }
        let report = projected.validate(&candidate);
        if report
            .problems
            .iter()
            .all(|problem| {
                !is_composition_independent(problem, &candidate, &std::collections::BTreeSet::new())
            })
        {
            applicable.push(shape);
        }
    }
    Some(match applicable.len() {
        1 => SuppliedArms::Unique(applicable[0]),
        0 => SuppliedArms::Undecided(shapes),
        _ => SuppliedArms::Undecided(applicable),
    })
}

fn without_eager(atom: &mut PropertyAtom) {
    atom.constraints.retain(|constraint| !matches!(constraint, Constraint::Eager));
    atom.array_constraints.retain(|constraint| !matches!(constraint, Constraint::Eager));
}

/// The lazy `file` alternatives a caller's string value (or list of strings)
/// of a property written through a recursive reference is judged by, for
/// relaxing existence on that value alone.
///
/// A string can only match a file alternative ([`file_alternatives`]), so
/// those alternatives, each with its list shape and without `eager`, judge
/// the value as the full definition would, short of existence. The shared
/// definitions stay untouched, so a document-authored use keeps its `eager`.
/// A list written on the property itself keeps its item-count bounds. `None`
/// for any other value, or when a string could be something other than a
/// file.
fn lazy_file_alternatives(
    definition: &PropertyDef,
    value: &serde_json::Value,
    definitions: &DefinitionTable,
) -> Option<Vec<PropertyAtom>> {
    let strings = match value {
        serde_json::Value::String(_) => true,
        serde_json::Value::Array(items) => items.iter().all(serde_json::Value::is_string),
        _ => false,
    };
    if !strings {
        return None;
    }
    let outer_bounds = match definition {
        PropertyDef::Single(atom) if atom.is_array => atom.array_constraints.as_slice(),
        _ => &[],
    };
    let alternatives = file_alternatives(definition, definitions)?;
    if alternatives.is_empty() {
        return None;
    }
    Some(
        alternatives
            .iter()
            .map(|alternative| {
                let mut atom = alternative.atom.clone();
                if alternative.is_array && !atom.is_array {
                    atom.is_array = true;
                    atom.array_constraints = outer_bounds.to_vec();
                }
                without_eager(&mut atom);
                atom
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests;
