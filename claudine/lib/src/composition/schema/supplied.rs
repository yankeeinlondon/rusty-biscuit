//! Caller-owned eager file inputs that need completion before lifecycle work.

use biscuit_file::{FileReference, FileResolutionContext};
use darkmatter::markdown::compose::CallerInputRecords;
use darkmatter::markdown::schemas::file_match::{file_match_admits, root_union_match_patterns};
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, EffectiveSchema, PropertyAtom, PropertyDef, SchemaArm,
    SchemaShape, SimplifiedSchema, SimplifiedType, TypeExpr, ValidationProblem,
    select_literal_discriminant_arm,
};

use super::{
    CompositionError, ResolvedCompositionSource, build_effective_instance,
    is_composition_independent, load_effective_schema, top_level_pointer_segment,
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
        load_effective_schema(source, None, document_context, records)
    else {
        return Vec::new();
    };
    let Some(arms) = supplied_file_arms(source, records, document_context, &effective) else {
        return Vec::new();
    };
    let mut pending = Vec::new();
    for (name, record) in records {
        let Some(FileTarget { is_array, patterns }) = arms.eager_file_target(name) else {
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
                    FileTarget { is_array, patterns: patterns.clone() },
                    record.origin(),
                ),
                array_index,
            });
        }
    }
    pending
}

/// The glob a caller-supplied file reference is completed against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FileTarget {
    pub(super) is_array: bool,
    /// Never empty: a bare `file` has no glob to walk.
    pub(super) patterns: Vec<String>,
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
    /// Where `name` must be an existing file whichever arm finally applies.
    ///
    /// An undecided union qualifies only when every contending arm declares
    /// `name` as an eager `file(match)` with the same array shape (ruling D1),
    /// because only then does the existence verdict not depend on the arm. Its
    /// patterns are the union of the arms', de-duplicated in arm order and then
    /// pattern order; the chosen literal path settles the arm afterward.
    fn eager_file_target(&self, name: &str) -> Option<FileTarget> {
        match self {
            Self::Unique(shape) => eager_file_target(shape, name),
            Self::Undecided(arms) => {
                let mut merged: Option<FileTarget> = None;
                for arm in arms {
                    let target = eager_file_target(arm, name)?;
                    match &mut merged {
                        None => merged = Some(target),
                        Some(merged) if merged.is_array == target.is_array => {
                            for pattern in target.patterns {
                                if !merged.patterns.contains(&pattern) {
                                    merged.patterns.push(pattern);
                                }
                            }
                        }
                        Some(_) => return None,
                    }
                }
                merged
            }
        }
    }
}

fn eager_file_target(shape: &SchemaShape, name: &str) -> Option<FileTarget> {
    let Some(PropertyDef::Single(atom)) = shape.properties.get(name) else {
        return None;
    };
    let eager = atom
        .constraints
        .iter()
        .chain(&atom.array_constraints)
        .any(|constraint| matches!(constraint, Constraint::Eager));
    if !eager {
        return None;
    }
    file_match_target(atom)
}

/// The `match(...)` glob of a `file`/`file[]` atom, whether or not it is eager.
pub(super) fn file_match_target(atom: &PropertyAtom) -> Option<FileTarget> {
    if !matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File)) {
        return None;
    }
    atom.constraints.iter().find_map(|constraint| match constraint {
        Constraint::Match(patterns) if !patterns.is_empty() => Some(FileTarget {
            is_array: atom.is_array,
            patterns: patterns.clone(),
        }),
        _ => None,
    })
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
        patterns: target.patterns,
        is_array: target.is_array,
        reason: caller_no_match_reason(provided, origin),
    }
}

/// The glob a validator's file-reference verdict on `name` is completed against.
///
/// A single schema reads the property's own `file(match)` declaration. A root
/// union whose literal discriminant in `instance` settles one arm reads only
/// that arm, because a file from another arm's glob would now be rejected by
/// the settled arm's own. Otherwise it reads every arm that declares `name` as
/// a `file(match)`: one such arm is that arm's glob, and several merge by the
/// D1 rule (arm order, then pattern order, de-duplicated) provided they agree
/// on the array shape.
pub(super) fn file_reference_target(
    effective: &EffectiveSchema,
    name: &str,
    instance: &serde_json::Value,
) -> Option<FileTarget> {
    let file_atom = |shape: &SchemaShape| match shape.properties.get(name) {
        Some(PropertyDef::Single(atom)) => file_match_target(atom),
        _ => None,
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
                    Some(merged) if merged.is_array == target.is_array => {
                        for pattern in target.patterns {
                            if !merged.patterns.contains(&pattern) {
                                merged.patterns.push(pattern);
                            }
                        }
                    }
                    Some(_) => return None,
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
    format!(
        "{NO_MATCH} `{provided}` while resolving from `{}`",
        origin.cwd().display(),
    )
}

/// Darkmatter's `NoMatch` text: a reference that failed existence resolution,
/// as opposed to a parse or resolution error that a glob walk cannot rescue.
pub(super) const NO_MATCH: &str = "no existing file matched reference";

/// Whether every existing caller file this arm declares falls inside its
/// `match` glob.
///
/// Each arm is validated alone below, where Darkmatter cannot tell which of
/// its globs discriminate, so the arm-level rule is applied here through the
/// same judgment Darkmatter's validator makes: a value naming no existing file
/// (a partial the chooser will complete) never rules an arm out.
fn admits_arm_files(arm: &SchemaArm, records: &CallerInputRecords) -> bool {
    records.iter().all(|(name, record)| {
        let Some(patterns) = root_union_match_patterns(arm, name) else {
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
            .all(|value| file_match_admits(value, patterns, record.origin()))
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
            if !records.contains_key(name) {
                continue;
            }
            let PropertyDef::Single(atom) = definition else {
                continue;
            };
            if !matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File)) {
                continue;
            }
            atom.constraints.retain(|constraint| !matches!(constraint, Constraint::Eager));
            atom.array_constraints.retain(|constraint| !matches!(constraint, Constraint::Eager));
            if atom.is_array && candidate.get(name).is_some_and(serde_json::Value::is_string) {
                candidate[name] = serde_json::Value::Array(vec![candidate[name].take()]);
            }
        }
        let schemas = DarkmatterSchemas::new(document_context.clone())
            .with_baseline(SimplifiedSchema::Single(relaxed))
            .ok()?;
        let projected = schemas.effective_for(&schema_source).ok()??;
        if !admits_arm_files(arm, records) {
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

#[cfg(test)]
mod tests;
