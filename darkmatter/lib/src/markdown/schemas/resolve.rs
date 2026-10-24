//! `$schema` resolution and baseline merging.
//!
//! This module turns a raw `$schema` frontmatter value into the fully-formed
//! Draft 2020-12 JSON Schema that the validator runs against. Three input
//! shapes are recognised:
//!
//! - **Inline mapping** — interpreted as a [`SimplifiedSchema`] directly.
//! - **Inline sequence** — interpreted as a root-level SimplifiedSchema union.
//!   String items are file references (resolved here); mapping items are
//!   inline arms.
//! - **String scalar** — interpreted as a `biscuit-file` reference to a YAML
//!   or JSON file. YAML SimplifiedSchemas are recognized by the pure
//!   (`$schema` as the sole top-level key) and tagged (`kind: schema` plus
//!   `types`) content envelopes; everything else remains raw JSON Schema.
//!   JSON files are always raw JSON Schema.
//!
//! After resolution, a baseline schema (if configured) is merged with the
//! document schema using property-keyed deep merge — the document side wins
//! on conflict.
//!
//! Remote (`http://` / `https://`) `$schema` values are rejected up front
//! with [`SchemaError::RemoteUnsupported`].
//!
//! Schema documents compiled into the library (the runtime baseline and the
//! Content Policy schema it imports) are prepared by `prepare_embedded_schema`
//! through the same parsing and import expansion. Only the namespace source
//! differs: an embedded document's imports resolve lexically against the
//! embedded table by repository-relative virtual path, never against the disk,
//! the network, or the request's context, and record no dependency edges.
//!
//! A reference in the reserved form `Name@darkmatter:<stem>` names a type of
//! the shipped partial `darkmatter/schemas/partials/<stem>.yaml` from any
//! schema, on disk or embedded, and resolves only against the shipped table.

use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    fs,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use biscuit_file::{FileReference, FileReferenceKind, FileResolutionContext};
use indexmap::IndexMap;
use serde_json::{Map, Value};
use serde_yaml_ng::Value as YamlValue;

use super::{
    SchemaAdvisory, SchemaArm,
    embedded::{self, EmbeddedDocument}, SchemaOrigin, SchemaOriginKind, SchemaShape, SimplifiedSchema,
    errors::SchemaError,
    simplified::{
        AlphanumericOptions, DefKey, DefNamespace, DefinitionTable, SchemaConstraintConflictKind,
        SchemaConstraintOrigin, SchemaDeclaration, SchemaReferenceSite, SchemaSourceId, SchemaSourceMap,
        SchemaSourcePath, SchemaUnionRejection, parse_standalone_schema_document, parse_yaml_schema,
        to_json_schema, to_json_schema_with_definitions,
        types::{
            Constraint, PatternKey, PatternKeyDef, PropertyAtom, PropertyDef, SimplifiedType,
            TypeExpr,
        },
    },
};

/// The product of resolving a document's `$schema`.
///
/// `simplified` is `None` when the effective schema came from a raw JSON
/// Schema file (or from a root union arm referencing one); `json_schema`
/// is always populated.
#[derive(Debug, Clone)]
pub struct ResolvedSchema {
    pub simplified: Option<SimplifiedSchema>,
    /// The cyclic named types `simplified` reaches through
    /// [`TypeExpr::Ref`], lowered once under `json_schema`'s root `$defs`.
    /// Empty when nothing recurses, which is every schema that existed
    /// before recursion was supported.
    pub definitions: Arc<DefinitionTable>,
    pub json_schema: Value,
    /// Where the schema came from — an inline `$schema` mapping/sequence
    /// (`Document`) or a referenced schema file (`ReferencedFile`, carrying
    /// the resolved path). Preserved so diagnostics can point
    /// `relatedInformation` at the schema source (R-5 Priority 2).
    pub origin: SchemaOrigin,

    /// Resolved paths of files pulled in by `Name@file` named-type imports
    /// (Feature B), in sorted order. Empty for schemas that use no imports.
    /// These are dependency edges: a change to an imported file's content
    /// invalidates a cached effective schema or resolved trigger payload.
    pub imports: Vec<PathBuf>,

    /// Resolved paths of `example(...)` artifact files (Feature A), in sorted
    /// order. Empty for schemas with no `example()` constraints. Like
    /// [`ResolvedSchema::imports`], these are dependency edges for cache and
    /// DMLS invalidation.
    pub examples: Vec<PathBuf>,

    /// Resolved paths of the schema FILES this `$schema` references directly:
    /// the top-level referenced file (`$schema: ./schema.yaml`) and each
    /// root-union string arm's file. Sorted, deduplicated. Empty for inline
    /// `$schema` mappings. These are dependency edges: a change to the schema
    /// file's own content must invalidate a cached effective schema.
    pub referenced_files: Vec<PathBuf>,

    /// Every location probed while selecting a schema file, named-type import,
    /// or example file, up to and including the selected one: an implicit
    /// path's authoring-folder candidate before its repository-root fallback,
    /// and each schema root a bare name tried. Sorted, deduplicated, and
    /// lexical (absent locations have no canonical form), so it overlaps the
    /// canonical dependency lists above without sharing their spelling.
    ///
    /// Unlike [`ResolvedSchema::imports`], [`ResolvedSchema::examples`], and
    /// [`ResolvedSchema::referenced_files`], entries may not exist; a nearer
    /// candidate appearing here changes what resolution selects. A recursive
    /// (`%`) reference contributes nothing: its search roots are not probes.
    pub searched: Vec<PathBuf>,

    /// Non-fatal findings discovered while resolving referenced schema files,
    /// sorted and deduplicated by semantic kind and canonical path.
    pub advisories: Vec<SchemaAdvisory>,
}

/// Resolves a frontmatter `$schema` value into a JSON Schema.
///
/// `base_dir` is the directory the document lives in — file references in
/// `$schema` resolve relative to it (as opposed to `file` *property* values,
/// which resolve from the CWD).
///
/// ## Errors
///
/// Returns:
/// - [`SchemaError::RemoteUnsupported`] for `http://` / `https://` values.
/// - [`SchemaError::Unresolved`] when a file reference is malformed or the
///   resolver rejects it.
/// - [`SchemaError::NoMatch`] when a file reference matches no file.
/// - [`SchemaError::Io`] when reading the referenced file fails.
/// - [`SchemaError::AmbiguousReferenced`] when the referenced file is neither
///   a valid SimplifiedSchema nor a JSON Schema.
/// - [`SchemaError::FrontmatterShape`] for unsupported `$schema` shapes.
/// - [`SchemaError::Grammar`] / [`SchemaError::Convert`] propagated from the
///   parser and converter.
pub fn resolve_schema(
    value: &Value,
    base_dir: &Path,
    context: &biscuit_file::FileResolutionContext,
) -> Result<ResolvedSchema, SchemaError> {
    resolve_schema_with_roots(value, base_dir, &[], context)
}

/// Same as [`resolve_schema`] but threads schema-root resolution context for
/// bare-name references (Phase 3). A bare name (`$schema: claudine.yaml`)
/// resolves against `schema_roots` nearest-first; path-qualified references
/// are untouched.
///
/// File references resolve through `request_context` derived to `base_dir`, so
/// the document base changes without recapturing repository, package, home,
/// environment, or configured roots.
pub fn resolve_schema_with_roots(
    value: &Value,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
) -> Result<ResolvedSchema, SchemaError> {
    let yaml = json_to_yaml(value);
    resolve_yaml_schema_with_roots(&yaml, base_dir, schema_roots, request_context)
}

/// Same as [`resolve_schema`] but takes a YAML value directly. Used by the
/// resolver itself (referenced YAML files) and by tests.
pub fn resolve_yaml_schema(
    value: &YamlValue,
    base_dir: &Path,
    context: &biscuit_file::FileResolutionContext,
) -> Result<ResolvedSchema, SchemaError> {
    resolve_yaml_schema_with_roots(value, base_dir, &[], context)
}

/// Same as [`resolve_yaml_schema`] but threads schema-root resolution context
/// for bare-name references (Phase 3).
pub fn resolve_yaml_schema_with_roots(
    value: &YamlValue,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
) -> Result<ResolvedSchema, SchemaError> {
    resolve_yaml_schema_owned_by(value, InlineOwner::Document, base_dir, schema_roots, request_context)
}

/// Who authored an inline `$schema` mapping, which decides the
/// [`DefNamespace`] its named types (and its root-union arms' named types)
/// are stored under.
#[derive(Debug, Clone, Copy)]
pub(crate) enum InlineOwner<'a> {
    /// The document being prepared.
    Document,
    /// The schema-trigger file at this path.
    Trigger(&'a Path),
}

impl InlineOwner<'_> {
    fn root(self) -> DefNamespace {
        match self {
            Self::Document => DefNamespace::InlineRoot,
            Self::Trigger(path) => DefNamespace::TriggerPayload(canonical_path(path)),
        }
    }

    fn union_arm(self, index: usize) -> DefNamespace {
        match self {
            Self::Document => DefNamespace::RootUnionArm(index),
            Self::Trigger(path) => DefNamespace::TriggerPayloadUnionArm(canonical_path(path), index),
        }
    }
}

/// [`resolve_yaml_schema_with_roots`] for an inline value authored by
/// `owner`. A string reference resolves exactly as it does for a document;
/// only the namespaces of inline named types differ.
pub(crate) fn resolve_yaml_schema_owned_by(
    value: &YamlValue,
    owner: InlineOwner<'_>,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
) -> Result<ResolvedSchema, SchemaError> {
    let mut stack = ReferenceStack::default();
    match value {
        YamlValue::String(reference) => {
            resolve_reference_in_context(
                reference,
                base_dir,
                schema_roots,
                request_context,
                &mut stack,
            )
        }
        YamlValue::Mapping(_) => {
            let schema = parse_yaml_schema(value)?;
            // Inline the definitions of any `Name@file` named-type imports
            // (Feature B) before conversion — `to_json_schema` rejects a
            // surviving import. `@this` resolves against this mapping, whose
            // namespace is its owner's.
            let mut searched = BTreeSet::new();
            let mut definitions = DefinitionTable::default();
            let (schema, imports) = expand_document_imports(
                schema,
                base_dir,
                owner.root(),
                &SchemaSourcePath::root(),
                schema_roots,
                request_context,
                &mut searched,
                &mut definitions,
            )?;
            let mut json = to_json_schema_with_definitions(&schema, &definitions)?;
            // Inline `$schema` mappings have no on-disk path, so `this` example
            // references have no target here (the acceptance driver references
            // examples from a schema *file*, resolved in `parse_yaml_referenced_file`).
            let examples = resolve_document_examples(
                &mut json,
                base_dir,
                None,
                schema_roots,
                request_context,
                &mut searched,
            )?;
            Ok(ResolvedSchema {
                simplified: Some(schema),
                definitions: Arc::new(definitions),
                json_schema: json,
                origin: SchemaOrigin::document(),
                imports,
                examples,
                referenced_files: Vec::new(),
                searched: searched.into_iter().collect(),
                advisories: Vec::new(),
            })
        }
        YamlValue::Sequence(items) => {
            resolve_root_union(items, owner, base_dir, schema_roots, request_context, &mut stack)
        }
        other => Err(SchemaError::FrontmatterShape {
            message: format!(
                "$schema must be a mapping, sequence, or string; got {}",
                describe_yaml(other)
            ),
        }),
    }
}

/// The canonical path of the file an explicit named-type reference
/// (`Name@reference`) selects, from a schema authored in `base_dir`.
///
/// This is the resolver's own target selection, exposed so an editor can find
/// the file that defines a referenced type without a second lookup rule.
/// `this` and a reserved `darkmatter:<stem>` reference (a shipped partial)
/// name no file and return `None`.
///
/// ## Errors
///
/// The same [`SchemaError`]s named-type expansion reports for the reference:
/// [`SchemaError::Unresolved`], [`SchemaError::NoMatch`],
/// [`SchemaError::BareNameSiblingExists`], or
/// [`SchemaError::RemoteUnsupported`].
pub fn named_type_reference_path(
    reference: &str,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
) -> Result<Option<PathBuf>, SchemaError> {
    if reference.trim() == "this" || embedded::reserved_stem(reference).is_some() {
        return Ok(None);
    }
    named_type_file_path(reference, base_dir, schema_roots, request_context, &mut BTreeSet::new())
        .map(|path| Some(canonical_path(&path)))
}

/// A schema whose named-type references were expanded, with what expansion
/// read and the cyclic definitions its [`TypeExpr::Ref`]s point into.
#[derive(Debug, Clone)]
pub struct NamedTypeExpansion {
    /// The expanded schema.
    pub schema: SimplifiedSchema,
    /// The files the references imported, sorted.
    pub imports: Vec<PathBuf>,
    /// The cyclic definitions the schema reaches; empty when nothing recurses.
    pub definitions: Arc<DefinitionTable>,
}

/// Expands the named-type references in `schema`, as resolving it would, and
/// returns the expanded schema with the files its references import.
///
/// `defining_file` is the schema file `schema` was authored in, or `None` for
/// an inline `$schema`; it decides what `this` names and the source of every
/// reported location. The schema supplies its own named types (an editor
/// passes its current buffer), so a local reference reads nothing; an
/// explicit reference reads its target through `request_context` from
/// `base_dir`. A root union expands each inline arm with that arm's
/// properties as `this`, as the resolver does; file-reference arms are left
/// as authored.
///
/// ## Errors
///
/// [`SchemaError::UnknownNamedType`], [`SchemaError::ConstraintConflict`],
/// [`SchemaError::UnionReference`], [`SchemaError::NonDescendingCycle`],
/// [`SchemaError::ImportCycle`], and any error reading or resolving an
/// imported file.
pub fn expand_named_types(
    schema: &SimplifiedSchema,
    base_dir: &Path,
    defining_file: Option<&Path>,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
) -> Result<NamedTypeExpansion, SchemaError> {
    let mut definitions = DefinitionTable::default();
    match schema {
        SimplifiedSchema::Single(_) => {
            let key = match defining_file {
                Some(path) => DefNamespace::File(canonical_path(path)),
                None => DefNamespace::InlineRoot,
            };
            let (schema, imports) = expand_document_imports(
                schema.clone(),
                base_dir,
                key,
                &SchemaSourcePath::root(),
                schema_roots,
                request_context,
                &mut BTreeSet::new(),
                &mut definitions,
            )?;
            Ok(NamedTypeExpansion {
                schema,
                imports,
                definitions: Arc::new(definitions),
            })
        }
        SimplifiedSchema::Union(arms) => {
            let mut imports = BTreeSet::new();
            let mut expanded = Vec::with_capacity(arms.len());
            for (index, arm) in arms.iter().enumerate() {
                let SchemaArm::Inline(shape) = arm else {
                    expanded.push(arm.clone());
                    continue;
                };
                let key = match defining_file {
                    Some(path) => DefNamespace::FileRootUnionArm(canonical_path(path), index),
                    None => DefNamespace::RootUnionArm(index),
                };
                let (schema, found) = expand_document_imports(
                    SimplifiedSchema::Single(shape.clone()),
                    base_dir,
                    key,
                    &SchemaSourcePath::root().union_arm(index),
                    schema_roots,
                    request_context,
                    &mut BTreeSet::new(),
                    &mut definitions,
                )?;
                imports.extend(found);
                let SimplifiedSchema::Single(shape) = schema else {
                    unreachable!("single root-union arm remains single")
                };
                expanded.push(SchemaArm::Inline(shape));
            }
            Ok(NamedTypeExpansion {
                schema: SimplifiedSchema::Union(expanded),
                imports: imports.into_iter().collect(),
                definitions: Arc::new(definitions),
            })
        }
    }
}

fn resolve_root_union(
    items: &[YamlValue],
    owner: InlineOwner<'_>,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    stack: &mut ReferenceStack,
) -> Result<ResolvedSchema, SchemaError> {
    if items.is_empty() {
        return Err(SchemaError::FrontmatterShape {
            message: "$schema root union must have at least one arm".into(),
        });
    }
    let mut any_of: Vec<Value> = Vec::with_capacity(items.len());
    let mut all_simplified_arms: Option<Vec<SchemaArm>> = Some(Vec::with_capacity(items.len()));
    let mut imports: BTreeSet<PathBuf> = BTreeSet::new();
    let mut examples: BTreeSet<PathBuf> = BTreeSet::new();
    let mut referenced_files: BTreeSet<PathBuf> = BTreeSet::new();
    let mut searched: BTreeSet<PathBuf> = BTreeSet::new();
    let mut advisories: BTreeSet<SchemaAdvisory> = BTreeSet::new();
    // One table for every inline arm: each arm is its own namespace, and
    // their definitions are emitted once at the union root.
    let mut definitions = DefinitionTable::default();
    let mut root_defs = Map::new();
    // Arms whose definitions join the root after every typed key is known.
    let mut deferred_arms: Vec<usize> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        match item {
            YamlValue::Mapping(_) => {
                let arm_schema = parse_yaml_schema(item)?;
                // Named-type imports are inlined per arm before conversion.
                let (arm_schema, arm_imports) =
                    expand_document_imports(
                        arm_schema,
                        base_dir,
                        owner.union_arm(index),
                        &SchemaSourcePath::root().union_arm(index),
                        schema_roots,
                        request_context,
                        &mut searched,
                        &mut definitions,
                    )?;
                imports.extend(arm_imports);
                let mut arm_json = to_json_schema_with_definitions(&arm_schema, &definitions)?;
                examples.extend(resolve_document_examples(
                    &mut arm_json,
                    base_dir,
                    None,
                    schema_roots,
                    request_context,
                    &mut searched,
                )?);
                hoist_definitions(&mut arm_json, &mut root_defs);
                if let SimplifiedSchema::Single(shape) = &arm_schema {
                    super::simplified::convert::attach_root_union_match(
                        &mut arm_json,
                        shape,
                        &definitions,
                        &mut root_defs,
                    )?;
                }
                if let SimplifiedSchema::Single(shape) = &arm_schema
                    && let Some(arms) = all_simplified_arms.as_mut()
                {
                    arms.push(SchemaArm::Inline(shape.clone()));
                } else {
                    all_simplified_arms = None;
                }
                any_of.push(strip_schema_uri(arm_json));
            }
            YamlValue::String(reference) => {
                let resolved = resolve_reference_in_context(
                    reference,
                    base_dir,
                    schema_roots,
                    request_context,
                    stack,
                )?;
                imports.extend(resolved.imports.iter().cloned());
                examples.extend(resolved.examples.iter().cloned());
                referenced_files.extend(resolved.referenced_files.iter().cloned());
                searched.extend(resolved.searched.iter().cloned());
                advisories.extend(resolved.advisories.iter().cloned());
                // If this arm itself came from a SimplifiedSchema file, preserve
                // it in the simplified projection only when it is a single
                // shape (root unions of unions are not modelled in v1).
                let mut arm_json = resolved.json_schema;
                if join_file_arm_definitions(
                    &resolved.definitions,
                    matches!(resolved.simplified, Some(SimplifiedSchema::Single(_))),
                    &mut arm_json,
                    reference,
                    &mut definitions,
                    &mut root_defs,
                )? {
                    deferred_arms.push(any_of.len());
                }
                if let Some(SimplifiedSchema::Single(shape)) = &resolved.simplified {
                    super::simplified::convert::attach_root_union_match(
                        &mut arm_json,
                        shape,
                        &definitions,
                        &mut root_defs,
                    )?;
                }
                match (&resolved.simplified, all_simplified_arms.as_mut()) {
                    (Some(SimplifiedSchema::Single(shape)), Some(arms)) => {
                        arms.push(SchemaArm::Inline(shape.clone()));
                    }
                    _ => {
                        all_simplified_arms = None;
                    }
                }
                any_of.push(strip_schema_uri(arm_json));
            }
            other => {
                return Err(SchemaError::FrontmatterShape {
                    message: format!(
                        "root union arms must be a mapping or a file reference, got {}",
                        describe_yaml(other)
                    ),
                });
            }
        }
    }
    for index in deferred_arms {
        super::scope::fold_definitions(&mut root_defs, &mut any_of[index]);
    }
    let mut root = Map::new();
    root.insert(
        "$schema".into(),
        Value::String(super::simplified::DRAFT_2020_12.into()),
    );
    root.insert("anyOf".into(), Value::Array(any_of));
    if !root_defs.is_empty() {
        root.insert("$defs".into(), Value::Object(root_defs));
    }

    Ok(ResolvedSchema {
        simplified: all_simplified_arms.map(SimplifiedSchema::Union),
        definitions: Arc::new(definitions),
        json_schema: Value::Object(root),
        origin: SchemaOrigin::document(),
        imports: imports.into_iter().collect(),
        examples: examples.into_iter().collect(),
        referenced_files: referenced_files.into_iter().collect(),
        searched: searched.into_iter().collect(),
        advisories: advisories.into_iter().collect(),
    })
}

/// Moves an inline root-union arm's lowered `$defs` to the union root, where
/// its `#/$defs/…` references resolve once the arm is nested under `anyOf`.
/// Every inline arm of one union lowers against one table, so a key already
/// present names the same definition.
fn hoist_definitions(arm_json: &mut Value, root_defs: &mut Map<String, Value>) {
    let Some(Value::Object(defs)) = arm_json.as_object_mut().and_then(|arm| arm.remove("$defs")) else {
        return;
    };
    for (key, body) in defs {
        root_defs.entry(key).or_insert(body);
    }
}

/// Joins the definitions of a root-union arm that names a schema file to
/// the union root, where its `#/$defs/…` references resolve once the arm is
/// nested under `anyOf`. The file was resolved on its own, so its keys were
/// allocated in a separate table.
///
/// A SimplifiedSchema single-shape arm (`typed`) folds its table into the
/// union's `definitions`, which re-keys only definitions the union does not
/// already hold, and its JSON references are renamed to match: the union's
/// table stays the one its arms lower against. Any other arm (raw JSON
/// Schema, or a file that is itself a root union) has no projection in that
/// table; it returns `true`, and its `$defs` join the root by
/// [`super::scope::fold_definitions`] after every arm is known.
///
/// ## Errors
///
/// [`SchemaError::UnresolvedReference`] or
/// [`SchemaError::UnsupportedMergeReference`] when a reference in the arm
/// would bind to a different target under the union root.
fn join_file_arm_definitions(
    arm_definitions: &DefinitionTable,
    typed: bool,
    arm_json: &mut Value,
    reference: &str,
    definitions: &mut DefinitionTable,
    root_defs: &mut Map<String, Value>,
) -> Result<bool, SchemaError> {
    super::scope::check_joined_references(arm_json, &format!("the root-union arm `{}`", reference.trim()))?;
    if arm_json.get("$defs").is_none() {
        return Ok(false);
    }
    if !typed {
        return Ok(true);
    }
    let renamed = definitions.absorb(arm_definitions);
    let Some(Value::Object(defs)) = arm_json.as_object_mut().and_then(|arm| arm.remove("$defs")) else {
        unreachable!("a lowered schema's `$defs` is an object");
    };
    if !renamed.is_empty() {
        super::scope::rename_references(arm_json, &renamed);
    }
    for (key, mut body) in defs {
        if !renamed.is_empty() {
            super::scope::rename_references(&mut body, &renamed);
        }
        root_defs.entry(renamed.get(&key).cloned().unwrap_or(key)).or_insert(body);
    }
    Ok(false)
}

// ── Bare-name schema-root resolution (Phase 3) ──────────────────────────────
//
// A schema reference with no path component (`$schema: claudine.yaml`,
// `Name@claudine.yaml`, `example(claudine.yaml)`) resolves against the
// schema-root list, nearest first. References with a path separator, `./`
// prefix, or magic-path prefix are untouched and resolve exactly as before.
// `FileReference` remains the authority for turning the selected reference
// into a filesystem path; bare-name selection only chooses *which* root to
// resolve from.

/// Returns `true` when a parsed reference is a non-recursive implicit-relative
/// name with exactly one portable path component.
fn is_bare_name(file_ref: &FileReference) -> bool {
    let class = file_ref.class();
    let portable = file_ref.raw().replace('\\', "/");
    let mut components = Path::new(&portable).components();
    let is_single_component =
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none();
    class.kind == FileReferenceKind::ImplicitRelative
        && !class.recursive
        && is_single_component
}

/// Tries a bare name against each schema root, nearest first. Returns the
/// resolved path of the first root that contains a file matching `name`, or
/// `None` when no root has it. Each root's location is added to `searched` as
/// it is tried.
///
/// Each root is tried with the name spelled as an explicit `./name`, resolved
/// in the request context derived onto that root. An explicit relative
/// reference has exactly one candidate, the root's own entry, so the loop's
/// advertised nearest-first ordering decides the winner. The derivation keeps
/// the request's home and environment and reads no ambient state; it crosses
/// the trust boundary because a schema root (configuration, `~`) may lie
/// outside the request's repository.
fn try_bare_name_in_roots(
    file_ref: &FileReference,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
    searched: &mut BTreeSet<PathBuf>,
) -> Result<Option<PathBuf>, SchemaError> {
    let unresolved = |source| SchemaError::Unresolved {
        reference: file_ref.raw().to_string(),
        source,
    };
    let explicit = FileReference::new(&format!("./{}", file_ref.raw())).map_err(unresolved)?;
    for root in schema_roots {
        let detailed = explicit.resolve_detailed(&request_context.for_trusted_external_cwd(root));
        searched.extend(detailed.probed_paths());
        if let Some(path) = detailed.into_convenience().map_err(unresolved)? {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

// ── `$schema` file-reference delegation guard ───────────────────────────────

/// Hard cap on `$schema` file-reference delegation depth. Mirrors
/// [`MAX_IMPORT_DEPTH`] so neither cross-file mechanism can bypass the other's
/// recursion guard.
const MAX_REFERENCE_DEPTH: usize = 32;

/// Canonical paths for the `$schema` files currently being resolved.
///
/// Whole-file delegation and root-union file arms both re-enter reference
/// resolution. Keeping only open frames permits repeated non-cyclic references
/// while turning direct and transitive loops into structured errors.
#[derive(Default)]
struct ReferenceStack {
    frames: Vec<PathBuf>,
}

impl ReferenceStack {
    fn enter(&mut self, canonical: &Path) -> Result<(), SchemaError> {
        if self.frames.iter().any(|frame| frame == canonical) {
            return Err(SchemaError::ReferenceCycle {
                chain: self.describe(canonical),
            });
        }
        if self.frames.len() >= MAX_REFERENCE_DEPTH {
            return Err(SchemaError::ReferenceDepthExceeded {
                limit: MAX_REFERENCE_DEPTH,
                chain: self.describe(canonical),
            });
        }
        self.frames.push(canonical.to_path_buf());
        Ok(())
    }

    fn leave(&mut self) {
        self.frames.pop();
    }

    fn describe(&self, rejected: &Path) -> String {
        let mut parts: Vec<String> = self
            .frames
            .iter()
            .map(|frame| frame.display().to_string())
            .collect();
        parts.push(rejected.display().to_string());
        parts.join(" -> ")
    }
}

/// Adds this delegation hop without discarding dependency edges collected by
/// the nested schema resolution.
fn accumulate_referenced_file(nested: Vec<PathBuf>, canonical: PathBuf) -> Vec<PathBuf> {
    let mut edges: BTreeSet<PathBuf> = nested.into_iter().collect();
    edges.insert(canonical);
    edges.into_iter().collect()
}

/// Adds the locations this delegation hop probed to those the nested schema
/// resolution recorded.
fn accumulate_searched(resolved: &mut ResolvedSchema, mut searched: BTreeSet<PathBuf>) {
    searched.extend(std::mem::take(&mut resolved.searched));
    resolved.searched = searched.into_iter().collect();
}

/// Attributes diagnostics to the file that actually authored the schema.
fn attribute_origin(resolved: &mut ResolvedSchema, path: &Path) {
    if resolved.origin.kind != SchemaOriginKind::ReferencedFile {
        resolved.origin = SchemaOrigin::referenced_file(path.to_path_buf());
    }
}

fn resolve_reference_in_context(
    reference: &str,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    stack: &mut ReferenceStack,
) -> Result<ResolvedSchema, SchemaError> {
    let trimmed = reference.trim();
    let file_ref = FileReference::new(trimmed).map_err(|source| SchemaError::Unresolved {
        reference: reference.to_string(),
        source,
    })?;
    if file_ref.class().kind == FileReferenceKind::Url {
        return Err(SchemaError::RemoteUnsupported {
            reference: reference.into(),
        });
    }

    let mut searched = BTreeSet::new();
    // Bare-name resolution against schema roots (Phase 3). When schema roots
    // are provided and the reference has no path component, resolve against
    // the roots nearest-first instead of the document directory.
    if !schema_roots.is_empty() && is_bare_name(&file_ref) {
        if let Some(path) =
            try_bare_name_in_roots(&file_ref, schema_roots, request_context, &mut searched)?
        {
            let mut resolved = load_guarded_in_context(
                &path,
                schema_roots,
                request_context,
                stack,
            )?;
            attribute_origin(&mut resolved, &path);
            resolved.referenced_files = accumulate_referenced_file(
                resolved.referenced_files,
                canonical_path(&path),
            );
            accumulate_searched(&mut resolved, searched);
            return Ok(resolved);
        }
        // Bare name not found in any schema root. Check if a document-sibling
        // file of that name exists — if so, suggest `./name` (the author
        // likely intended a relative reference, not a schema-root lookup).
        let sibling = base_dir.join(trimmed);
        if sibling.is_file() {
            return Err(SchemaError::BareNameSiblingExists {
                reference: trimmed.to_string(),
                suggestion: format!("./{trimmed}"),
            });
        }
        // No sibling either — the bare name simply does not resolve.
        return Err(SchemaError::NoMatch {
            reference: trimmed.to_string(),
            bare_name: true,
        });
    }

    // `$schema` references resolve through the shared document-backed context:
    // explicit `./`/`../` from the document directory only, implicit path
    // references from the document directory first, then the repository root — the same
    // order the `file`-typed value references use. No ambient CWD is read.
    let path = resolve_file_reference_in_context(&file_ref, base_dir, request_context, &mut searched)
        .map_err(|source| SchemaError::Unresolved {
            reference: trimmed.to_string(),
            source,
        })?
        .ok_or_else(|| SchemaError::NoMatch {
            reference: trimmed.to_string(),
            bare_name: false,
        })?;

    let mut resolved = load_guarded_in_context(
        &path,
        schema_roots,
        request_context,
        stack,
    )?;
    // The schema was loaded from a file — record the resolved path so
    // diagnostics can point `relatedInformation` at the referenced source.
    attribute_origin(&mut resolved, &path);
    // The referenced file's own content is a dependency edge: editing
    // `schema.yaml`'s type must invalidate a cached effective schema.
    resolved.referenced_files =
        accumulate_referenced_file(resolved.referenced_files, canonical_path(&path));
    accumulate_searched(&mut resolved, searched);
    Ok(resolved)
}

/// Loads one schema file with a cycle/depth frame open. The request context is
/// passed through unchanged, and the frame is closed on success or failure.
fn load_guarded_in_context(
    path: &Path,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    stack: &mut ReferenceStack,
) -> Result<ResolvedSchema, SchemaError> {
    let canonical = canonical_path(path);
    stack.enter(&canonical)?;
    let loaded = load_schema_from_path_in_context(
        path,
        schema_roots,
        request_context,
        stack,
    )
    .map_err(|source| match source {
        SchemaError::Grammar { .. }
        | SchemaError::Convert { .. }
        | SchemaError::UnknownNamedType { .. }
        | SchemaError::NonDescendingCycle { .. }
        | SchemaError::ConstraintConflict { .. }
        | SchemaError::UnionReference { .. }
        | SchemaError::Aggregate { .. } => SchemaError::ReferencedSchema {
            path: canonical.clone(),
            source: Box::new(source),
        },
        other => other,
    });
    stack.leave();
    loaded
}

/// Resolves `file_ref` from `base_dir`, adding every location it probed to
/// `searched`. This and [`try_bare_name_in_roots`] are the only places schema
/// resolution probes a file reference, so `searched` sees every candidate.
fn resolve_file_reference_in_context(
    file_ref: &FileReference,
    base_dir: &Path,
    request_context: &biscuit_file::FileResolutionContext,
    searched: &mut BTreeSet<PathBuf>,
) -> Result<Option<PathBuf>, biscuit_file::FileReferenceError> {
    let detailed = file_ref.resolve_detailed(&request_context.for_cwd(base_dir));
    searched.extend(detailed.probed_paths());
    detailed.into_convenience()
}

fn load_schema_from_path_in_context(
    path: &Path,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    stack: &mut ReferenceStack,
) -> Result<ResolvedSchema, SchemaError> {
    let bytes = fs::read(path).map_err(|source| SchemaError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("json") => parse_raw_json_schema(path, &bytes),
        _ => parse_yaml_referenced_file(path, &bytes, schema_roots, request_context, stack),
    }
}

fn parse_yaml_referenced_file(
    path: &Path,
    bytes: &[u8],
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    stack: &mut ReferenceStack,
) -> Result<ResolvedSchema, SchemaError> {
    let text = std::str::from_utf8(bytes).map_err(|error| SchemaError::SchemaDocument {
        path: path.to_path_buf(),
        message: format!("not UTF-8 text: {error}"),
    })?;
    if let Some(document) = parse_standalone_schema_document(text, path)? {
        return match document.declaration {
            SchemaDeclaration::Schema(schema) => {
                resolve_standalone_schema(schema, path, schema_roots, request_context, stack)
            }
            SchemaDeclaration::Reference(reference) => {
                let file_dir = path.parent().unwrap_or_else(|| Path::new("."));
                resolve_reference_in_context(
                    reference.file_reference().raw(),
                    file_dir,
                    schema_roots,
                    request_context,
                    stack,
                )
            }
        };
    }

    // Treat the file's contents as a raw JSON Schema serialized in YAML.
    // A duplicate key, trailing content, or a second document is a located
    // syntax error in the file, not an unrecognizable schema.
    let yaml: YamlValue = crate::markdown::numeric::load_admitted_yaml(text).map_err(|error| {
        let message = if crate::markdown::numeric::is_yaml_admission_error(&error) {
            error.to_string()
        } else {
            format!("not valid YAML: {error}")
        };
        SchemaError::SchemaDocument {
            path: path.to_path_buf(),
            message,
        }
    })?;
    let advisories = missing_simplified_envelope_advisory(&yaml, path)
        .into_iter()
        .collect();
    let json: Value =
        serde_json::to_value(yaml).map_err(|_| SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        })?;
    if !json.is_object() {
        return Err(SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        });
    }
    Ok(ResolvedSchema {
        simplified: None,
        definitions: Arc::default(),
        json_schema: json,
        origin: SchemaOrigin::document(),
        imports: Vec::new(),
        examples: Vec::new(),
        referenced_files: Vec::new(),
        searched: Vec::new(),
        advisories,
    })
}

/// Returns whether `key` belongs to a Draft 2020-12 vocabulary understood by
/// JSON Schema implementations.
fn is_draft_2020_12_keyword(key: &str) -> bool {
    matches!(
        key,
        // Core vocabulary.
        "$schema"
            | "$id"
            | "$vocabulary"
            | "$anchor"
            | "$dynamicAnchor"
            | "$dynamicRef"
            | "$ref"
            | "$defs"
            | "$comment"
            // Applicator vocabulary.
            | "prefixItems"
            | "items"
            | "contains"
            | "additionalProperties"
            | "properties"
            | "patternProperties"
            | "dependentSchemas"
            | "propertyNames"
            | "if"
            | "then"
            | "else"
            | "allOf"
            | "anyOf"
            | "oneOf"
            | "not"
            // Validation vocabulary.
            | "type"
            | "const"
            | "enum"
            | "multipleOf"
            | "maximum"
            | "exclusiveMaximum"
            | "minimum"
            | "exclusiveMinimum"
            | "maxLength"
            | "minLength"
            | "pattern"
            | "maxItems"
            | "minItems"
            | "uniqueItems"
            | "maxContains"
            | "minContains"
            | "maxProperties"
            | "minProperties"
            | "required"
            | "dependentRequired"
            // Metadata, format, content, and unevaluated vocabularies.
            | "title"
            | "description"
            | "default"
            | "deprecated"
            | "readOnly"
            | "writeOnly"
            | "examples"
            | "format"
            | "contentEncoding"
            | "contentMediaType"
            | "contentSchema"
            | "unevaluatedItems"
            | "unevaluatedProperties"
    )
}

/// Classifies only the narrow shape that would otherwise silently act as an
/// unconstrained raw JSON Schema. The supplied YAML value is already in
/// memory; this performs no resolution or other I/O.
fn missing_simplified_envelope_advisory(
    root: &YamlValue,
    path: &Path,
) -> Option<SchemaAdvisory> {
    let map = root.as_mapping()?;
    if map.is_empty() {
        return None;
    }
    for (key, value) in map {
        let key = key.as_str()?;
        if !value.is_string()
            || key.starts_with('$')
            || key.starts_with("x-")
            || is_draft_2020_12_keyword(key)
        {
            return None;
        }
    }
    parse_yaml_schema(root).ok()?;
    Some(SchemaAdvisory::missing_simplified_envelope(path))
}

fn resolve_standalone_schema(
    schema: SimplifiedSchema,
    path: &Path,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    stack: &mut ReferenceStack,
) -> Result<ResolvedSchema, SchemaError> {
    let file_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let key = DefNamespace::File(canonical_path(path));
    match schema {
        SimplifiedSchema::Single(_) => {
            let mut searched = BTreeSet::new();
            let mut definitions = DefinitionTable::default();
            let (schema, imports) = expand_document_imports(
                schema,
                file_dir,
                key,
                &SchemaSourcePath::root(),
                schema_roots,
                request_context,
                &mut searched,
                &mut definitions,
            )?;
            let mut json_schema = to_json_schema_with_definitions(&schema, &definitions)?;
            let examples = resolve_document_examples(
                &mut json_schema,
                file_dir,
                Some(path),
                schema_roots,
                request_context,
                &mut searched,
            )?;
            Ok(ResolvedSchema {
                simplified: Some(schema),
                definitions: Arc::new(definitions),
                json_schema,
                origin: SchemaOrigin::document(),
                imports,
                examples,
                referenced_files: Vec::new(),
                searched: searched.into_iter().collect(),
                advisories: Vec::new(),
            })
        }
        SimplifiedSchema::Union(arms) => {
            resolve_standalone_root_union(
                arms,
                file_dir,
                path,
                schema_roots,
                request_context,
                stack,
            )
        }
    }
}

/// Prepares the embedded schema document registered at `path`: parses it,
/// expands its imports against `documents` alone, and lowers it.
///
/// Embedded documents are immutable build inputs, so the result records no
/// dependency edges and nothing is read from the disk or the network.
///
/// ## Errors
///
/// The resolver's typed errors. Failures specific to an embedded source are
/// [`SchemaError::SchemaDocument`], naming the importing virtual document: an
/// unregistered `path` or import target, an import that is not a relative
/// virtual path inside the repository root, a whole-file reference or
/// root-union document, or an `example(...)` constraint (which would name a
/// file on disk).
#[cfg(test)]
pub(crate) fn prepare_embedded_schema(
    path: &str,
    documents: &[EmbeddedDocument<'_>],
) -> Result<ResolvedSchema, SchemaError> {
    prepare_embedded_schema_in(path, documents, "").map(|(resolved, _)| resolved)
}

/// [`prepare_embedded_schema`] with every import confined to `{bundle}/`
/// (no confinement when `bundle` is empty), alongside the virtual paths of
/// every document it read (`path` and its transitive imports).
pub(crate) fn prepare_embedded_schema_in(
    path: &str,
    documents: &[EmbeddedDocument<'_>],
    bundle: &str,
) -> Result<(ResolvedSchema, Vec<PathBuf>), SchemaError> {
    let unsupported = |message: &str| SchemaError::SchemaDocument {
        path: PathBuf::from(path),
        message: message.to_string(),
    };
    let document = embedded::find(documents, path)
        .ok_or_else(|| unsupported("not a registered embedded schema document"))?;
    let Some(standalone) = parse_standalone_schema_document(document.text, Path::new(path))? else {
        return Err(SchemaError::AmbiguousReferenced {
            path: PathBuf::from(path),
        });
    };
    let schema = match standalone.declaration {
        SchemaDeclaration::Schema(schema @ SimplifiedSchema::Single(_)) => schema,
        SchemaDeclaration::Schema(SimplifiedSchema::Union(_)) => {
            return Err(unsupported("an embedded schema cannot be a root union"));
        }
        SchemaDeclaration::Reference(_) => {
            return Err(unsupported("an embedded schema cannot be a whole-file reference"));
        }
    };
    lower_embedded_schema(schema, path, documents, bundle)
}

/// Prepares an inline `$schema` mapping authored in the embedded document
/// `importer`: its imports resolve against `documents` inside `{bundle}/`,
/// and `@this` names the mapping's own entries.
///
/// ## Errors
///
/// As [`prepare_embedded_schema_in`]; a mapping that is not a single object
/// schema is a [`SchemaError::SchemaDocument`] naming `importer`.
pub(crate) fn prepare_embedded_inline_schema(
    value: &YamlValue,
    importer: &str,
    documents: &[EmbeddedDocument<'_>],
    bundle: &str,
) -> Result<(ResolvedSchema, Vec<PathBuf>), SchemaError> {
    let schema = parse_yaml_schema(value)?;
    if matches!(schema, SimplifiedSchema::Union(_)) {
        return Err(SchemaError::SchemaDocument {
            path: PathBuf::from(importer),
            message: "an embedded inline schema cannot be a root union".into(),
        });
    }
    lower_embedded_schema(schema, importer, documents, bundle)
}

/// Expands a single-object embedded schema authored at `path` and lowers it,
/// alongside the virtual paths of `path` and every document it imported.
fn lower_embedded_schema(
    schema: SimplifiedSchema,
    path: &str,
    documents: &[EmbeddedDocument<'_>],
    bundle: &str,
) -> Result<(ResolvedSchema, Vec<PathBuf>), SchemaError> {
    let mut definitions = DefinitionTable::default();
    let (schema, mut reads) = expand_imports(
        schema,
        Path::new(""),
        DefNamespace::Embedded(path.to_string()),
        &SchemaSourcePath::root(),
        NamespaceSource::Embedded { documents, bundle },
        // Embedded imports resolve lexically in the table and probe no disk.
        &mut BTreeSet::new(),
        &mut definitions,
    )?;
    let json_schema = to_json_schema_with_definitions(&schema, &definitions)?;
    if contains_example_annotation(&json_schema) {
        return Err(SchemaError::SchemaDocument {
            path: PathBuf::from(path),
            message: "an embedded schema cannot reference `example(...)` artifacts".into(),
        });
    }
    reads.push(PathBuf::from(path));
    let resolved = ResolvedSchema {
        simplified: Some(schema),
        definitions: Arc::new(definitions),
        json_schema,
        origin: SchemaOrigin::document(),
        imports: Vec::new(),
        examples: Vec::new(),
        referenced_files: Vec::new(),
        searched: Vec::new(),
        advisories: Vec::new(),
    };
    Ok((resolved, reads))
}

/// Whether a lowered schema carries an unresolved `example(...)` reference.
fn contains_example_annotation(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            map.contains_key("x-darkmatter-example") || map.values().any(contains_example_annotation)
        }
        Value::Array(items) => items.iter().any(contains_example_annotation),
        _ => false,
    }
}

fn resolve_standalone_root_union(
    arms: Vec<SchemaArm>,
    base_dir: &Path,
    path: &Path,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    stack: &mut ReferenceStack,
) -> Result<ResolvedSchema, SchemaError> {
    let mut any_of = Vec::with_capacity(arms.len());
    let mut simplified_arms = Vec::with_capacity(arms.len());
    let mut all_simplified = true;
    let mut imports = BTreeSet::new();
    let mut examples = BTreeSet::new();
    let mut referenced_files = BTreeSet::new();
    let mut searched = BTreeSet::new();
    let mut advisories = BTreeSet::new();
    let mut definitions = DefinitionTable::default();
    let mut root_defs = Map::new();
    // Arms whose definitions join the root after every typed key is known.
    let mut deferred_arms: Vec<usize> = Vec::new();

    for (index, arm) in arms.into_iter().enumerate() {
        match arm {
            SchemaArm::Inline(shape) => {
                let schema = SimplifiedSchema::Single(shape);
                let (schema, arm_imports) = expand_document_imports(
                    schema,
                    base_dir,
                    DefNamespace::FileRootUnionArm(canonical_path(path), index),
                    &SchemaSourcePath::root().union_arm(index),
                    schema_roots,
                    request_context,
                    &mut searched,
                    &mut definitions,
                )?;
                imports.extend(arm_imports);
                let SimplifiedSchema::Single(shape) = schema else {
                    unreachable!("single root-union arm remains single")
                };
                let mut arm_json =
                    to_json_schema_with_definitions(&SimplifiedSchema::Single(shape.clone()), &definitions)?;
                examples.extend(resolve_document_examples(
                    &mut arm_json,
                    base_dir,
                    Some(path),
                    schema_roots,
                    request_context,
                    &mut searched,
                )?);
                hoist_definitions(&mut arm_json, &mut root_defs);
                super::simplified::convert::attach_root_union_match(
                    &mut arm_json,
                    &shape,
                    &definitions,
                    &mut root_defs,
                )?;
                simplified_arms.push(SchemaArm::Inline(shape));
                any_of.push(strip_schema_uri(arm_json));
            }
            SchemaArm::FileRef(reference) => {
                let resolved = resolve_reference_in_context(
                    &reference,
                    base_dir,
                    schema_roots,
                    request_context,
                    stack,
                )?;
                imports.extend(resolved.imports.iter().cloned());
                examples.extend(resolved.examples.iter().cloned());
                referenced_files.extend(resolved.referenced_files.iter().cloned());
                searched.extend(resolved.searched.iter().cloned());
                advisories.extend(resolved.advisories.iter().cloned());
                let mut arm_json = resolved.json_schema;
                if join_file_arm_definitions(
                    &resolved.definitions,
                    matches!(resolved.simplified, Some(SimplifiedSchema::Single(_))),
                    &mut arm_json,
                    &reference,
                    &mut definitions,
                    &mut root_defs,
                )? {
                    deferred_arms.push(any_of.len());
                }
                if let Some(SimplifiedSchema::Single(shape)) = resolved.simplified {
                    super::simplified::convert::attach_root_union_match(
                        &mut arm_json,
                        &shape,
                        &definitions,
                        &mut root_defs,
                    )?;
                    simplified_arms.push(SchemaArm::Inline(shape));
                } else {
                    all_simplified = false;
                }
                any_of.push(strip_schema_uri(arm_json));
            }
        }
    }

    for index in deferred_arms {
        super::scope::fold_definitions(&mut root_defs, &mut any_of[index]);
    }
    let mut root = Map::new();
    root.insert(
        "$schema".into(),
        Value::String(super::simplified::DRAFT_2020_12.into()),
    );
    root.insert("anyOf".into(), Value::Array(any_of));
    if !root_defs.is_empty() {
        root.insert("$defs".into(), Value::Object(root_defs));
    }
    Ok(ResolvedSchema {
        simplified: all_simplified.then_some(SimplifiedSchema::Union(simplified_arms)),
        definitions: Arc::new(definitions),
        json_schema: Value::Object(root),
        origin: SchemaOrigin::document(),
        imports: imports.into_iter().collect(),
        examples: examples.into_iter().collect(),
        referenced_files: referenced_files.into_iter().collect(),
        searched: searched.into_iter().collect(),
        advisories: advisories.into_iter().collect(),
    })
}

/// ## Errors
///
/// [`SchemaError::SchemaDocument`], naming the line and column, when the
/// bytes are not one strict JSON value: invalid syntax, trailing content, a
/// duplicate key, a number too large for `f64` (`1e400`), or a number whose
/// written value cannot be admitted (an inexact whole number or an
/// underflow, which also names its JSON Pointer).
/// [`SchemaError::AmbiguousReferenced`] when the value is not an object.
fn parse_raw_json_schema(path: &Path, bytes: &[u8]) -> Result<ResolvedSchema, SchemaError> {
    let document_error = |message: String| SchemaError::SchemaDocument {
        path: path.to_path_buf(),
        message,
    };
    let text = std::str::from_utf8(bytes).map_err(|error| document_error(format!("not UTF-8 text: {error}")))?;
    if let Some(message) =
        crate::markdown::coercion::content_error(crate::markdown::coercion::ContentFormat::Json, text)
    {
        return Err(document_error(message));
    }
    let value: Value = serde_json::from_str(text).map_err(|error| document_error(format!("invalid JSON: {error}")))?;
    crate::markdown::numeric::admit_json_text_numbers(text).map_err(|error| document_error(error.to_string()))?;
    if !value.is_object() {
        return Err(SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        });
    }
    Ok(ResolvedSchema {
        simplified: None,
        definitions: Arc::default(),
        json_schema: value,
        origin: SchemaOrigin::document(),
        imports: Vec::new(),
        examples: Vec::new(),
        referenced_files: Vec::new(),
        searched: Vec::new(),
        advisories: Vec::new(),
    })
}

// ── Cross-file named-type imports (Feature B) ───────────────────────────────
//
// `Name@fileref`, or a bare local `Name` (which is `Name@this`), inlines the
// definition of a named type at the use site (structural substitution). A
// schema file's top-level mapping entries (under `$schema:`, or `types:` in a
// `kind: schema` document) are its named types; `@this` targets the schema
// that writes the reference. Every constraint of the definition survives the
// reference, including `required` and `generated`. Postfix composes outward:
// `Name[]@file` becomes an array of the inlined type, and
// `Name(constraints)@file` merges the use-site constraints into the inherited
// ones (`apply_import_postfix`). Expansion is eager, bounded, and
// cycle-checked (named types form a DAG in v1); `to_json_schema` still rejects
// any import that survives.
//
// Each namespace's unexpanded table is parsed once per expansion and shared
// through an `Arc`: the cache, every later import of that file, and every
// `@this` hold the same immutable allocation. Only the selected definition is
// cloned, so each use site expands and modifies its own copy. The sharing
// ends with `expand_document_imports`; nothing persists across calls.

/// Hard cap on named-type import expansion depth. Mirrors the inline-object
/// depth cap so import chains cannot bypass the recursion guard indefinitely.
const MAX_IMPORT_DEPTH: usize = 32;

/// A named-type namespace: the top-level mapping entries (`$schema:`, or
/// `types:` under `kind: schema`) a file (or the inline document) exposes for
/// `Name@…` imports, plus the context needed to resolve relative imports and
/// `@this` inside it.
#[derive(Clone)]
struct Namespace {
    /// The identity of this namespace's table: definitions and cycle
    /// detection key on it.
    key: DefNamespace,
    /// Directory that relative imports authored inside this namespace resolve
    /// against (mirrors `$schema` file-ref resolution).
    base_dir: PathBuf,
    /// The unexpanded top-level named types, shared with the namespace cache
    /// and every other resolution of the same namespace. Never mutated.
    named_types: Arc<IndexMap<String, PropertyDef>>,
}

/// Drives eager, bounded, cycle-checked expansion of `Name@file` imports over
/// a parsed [`SimplifiedSchema`].
struct ImportEngine<'a> {
    /// The acyclic definitions currently being inlined, used to bound the
    /// depth of an import chain. A cyclic definition is never a frame: its
    /// uses are references and its body is expanded on its own.
    stack: Vec<DefKey>,
    /// Resolved import file paths, deduplicated — the dependency edges a warm
    /// cache / DMLS index consumes to invalidate on imported-file change.
    dependencies: BTreeSet<PathBuf>,
    /// Locations probed while selecting import targets and rehoming their
    /// examples (see [`ResolvedSchema::searched`]). Every import site resolves
    /// its reference before `namespace_cache` is consulted, so a cached
    /// namespace still records the probes of each site that names it.
    searched: BTreeSet<PathBuf>,
    /// Where the namespaces that imports name are read from.
    source: NamespaceSource<'a>,
    /// Parsed named-type namespaces keyed by canonical file path.
    namespace_cache: HashMap<PathBuf, CachedNamespace>,
    /// The file each `(reference, base_dir)` selected, so a site repeating a
    /// reference replays its probes into `searched` without resolving it
    /// again. Only successful selections are kept, and the roots and context
    /// they depend on are fixed for the engine's lifetime.
    selected_files: HashMap<(String, PathBuf), SelectedFile>,
    /// Parsed embedded namespaces keyed by virtual path.
    embedded_cache: HashMap<String, Arc<IndexMap<String, PropertyDef>>>,
    /// Parsed shipped partials keyed by virtual path, apart from
    /// `embedded_cache`: a host table may register its own document under a
    /// shipped path.
    shipped_cache: HashMap<String, Arc<IndexMap<String, PropertyDef>>>,
    /// Each named type expanded so far, by `(namespace, name)`, before any
    /// use-site postfix. A definition's expansion depends only on its own
    /// namespace, so a type inlined at many sites (the baseline's lifecycle
    /// stacks) is expanded once. Only successful expansions are kept; a cycle
    /// through a type is found at its first expansion.
    expanded_cache: HashMap<DefKey, ExpandedDef>,
    /// The definitions that take part in a reference cycle, found before
    /// expansion starts. Empty for a schema that does not recurse.
    cyclic: HashSet<DefKey>,
    /// What each cyclic definition contributes to a use site. `None` during
    /// the first of the two passes a recursive schema takes.
    summaries: Option<HashMap<DefKey, recursive::UseSummary>>,
    /// Each cyclic definition reached in this pass, in first-reached order,
    /// with its namespace and, once expanded, its body.
    reached: IndexMap<DefKey, recursive::ReachedDefinition>,
    /// Reached cyclic definitions whose bodies are not expanded yet.
    pending: VecDeque<DefKey>,
}

/// Where an expansion reads the namespaces its imports name. This is all that
/// differs between a disk and an embedded expansion: parsing, expansion,
/// constraint merging, and cycle detection are shared.
enum NamespaceSource<'a> {
    /// Schema files, resolved through the request's captured context; bare
    /// file names try the schema roots first (Phase 3).
    Files {
        schema_roots: Vec<PathBuf>,
        context: Box<biscuit_file::FileResolutionContext>,
    },
    /// The embedded table, by virtual path. Nothing is read from the disk or
    /// the network, and every namespace is [`DefNamespace::Embedded`]. A
    /// non-empty `bundle` confines every import to `{bundle}/`.
    Embedded {
        documents: &'a [EmbeddedDocument<'a>],
        bundle: &'a str,
    },
}

impl<'a> ImportEngine<'a> {
    fn new(source: NamespaceSource<'a>) -> Self {
        Self {
            stack: Vec::new(),
            dependencies: BTreeSet::new(),
            searched: BTreeSet::new(),
            source,
            namespace_cache: HashMap::new(),
            selected_files: HashMap::new(),
            embedded_cache: HashMap::new(),
            shipped_cache: HashMap::new(),
            expanded_cache: HashMap::new(),
            cyclic: HashSet::new(),
            summaries: None,
            reached: IndexMap::new(),
            pending: VecDeque::new(),
        }
    }
}

/// The schema file one import reference selected.
struct SelectedFile {
    path: PathBuf,
    canonical: PathBuf,
    /// The locations the selection probed.
    probed: Vec<PathBuf>,
}

struct CachedNamespace {
    base_dir: PathBuf,
    named_types: Arc<IndexMap<String, PropertyDef>>,
}

/// One named-type reference as authored: the type, the namespace it names,
/// and the postfix the use site adds.
struct NamedReference {
    name: String,
    reference: String,
    is_array: bool,
    constraints: Vec<Constraint>,
    array_constraints: Vec<Constraint>,
    description: Option<String>,
}

/// A definition after expansion.
///
/// A single atom carries the origin of each top-level constraint, which a
/// use-site merge needs to report where contradictory values were declared.
/// A union carries none: a postfix over a union keeps the arms verbatim, and
/// an arm default it lifts takes its arm's position in the definition.
#[derive(Clone)]
enum ExpandedDef {
    Single(Box<PropertyAtom>, AtomProvenance),
    Union(Vec<PropertyAtom>),
}

impl ExpandedDef {
    fn into_def(self) -> PropertyDef {
        match self {
            Self::Single(atom, _) => PropertyDef::Single(*atom),
            Self::Union(arms) => PropertyDef::Union(arms),
        }
    }
}

/// A union's flattened arms and the property markers hoisted off the
/// postfixed union arms flattened into it.
struct FlatUnion {
    arms: Vec<PropertyAtom>,
    markers: Vec<Tagged>,
}

impl FlatUnion {
    /// The union as a definition. Without hoisted markers it stays a bare arm
    /// list, so a schema without postfixed union arms resolves as before;
    /// with them it is the non-array union atom a postfixed union alias
    /// resolves to, holding the markers.
    fn into_expanded(self) -> ExpandedDef {
        if self.markers.is_empty() {
            return ExpandedDef::Union(self.arms);
        }
        let (constraints, value) = self.markers.into_iter().unzip();
        ExpandedDef::Single(
            Box::new(PropertyAtom {
                ty: TypeExpr::Union(self.arms),
                is_array: false,
                constraints,
                array_constraints: Vec::new(),
                description: None,
            }),
            AtomProvenance {
                value,
                array: Vec::new(),
                shape: Vec::new(),
            },
        )
    }
}

/// Adds a property marker unless `markers` already holds it; markers are
/// flags, so the first origin is kept.
fn add_marker(markers: &mut Vec<Tagged>, marker: Tagged) {
    if !markers.iter().any(|(existing, _)| existing == &marker.0) {
        markers.push(marker);
    }
}

/// Origins of an atom's top-level constraints, index-aligned with its
/// `constraints`, its `array_constraints`, and (for an inline object) its
/// shape's `$constraints`.
///
/// An inherited constraint keeps the origin of the declaration that
/// introduced it, however many references it passed through.
#[derive(Debug, Clone, Default)]
struct AtomProvenance {
    value: Vec<SchemaConstraintOrigin>,
    array: Vec<SchemaConstraintOrigin>,
    shape: Vec<SchemaConstraintOrigin>,
}

impl AtomProvenance {
    /// Origins for an atom authored at `path` in `source`.
    fn authored(atom: &PropertyAtom, source: &SchemaSourceId, path: &SchemaSourcePath) -> Self {
        let (value, array) =
            authored_origins(&atom.constraints, &atom.array_constraints, source, path);
        let shape = match &atom.ty {
            TypeExpr::InlineObject(shape) => shape
                .constraints
                .iter()
                .map(|constraint| {
                    let key = dollar_constraints_key(constraint);
                    SchemaConstraintOrigin {
                        source: source.clone(),
                        path: path.property("$constraints").property(key),
                        keyword: key.to_string(),
                        occurrence: 0,
                        span: None,
                    }
                })
                .collect(),
            _ => Vec::new(),
        };
        Self {
            value,
            array,
            shape,
        }
    }
}

/// Origins for the constraint groups of one authored atom. `occurrence`
/// counts same-keyword constraints in source order: the value-level group
/// precedes the array-level group, as in `T(c)[](c)`.
fn authored_origins(
    value: &[Constraint],
    array: &[Constraint],
    source: &SchemaSourceId,
    path: &SchemaSourcePath,
) -> (Vec<SchemaConstraintOrigin>, Vec<SchemaConstraintOrigin>) {
    let mut occurrences: HashMap<&'static str, usize> = HashMap::new();
    let mut origin = |constraint: &Constraint| {
        let keyword = constraint.keyword();
        let occurrence = occurrences.entry(keyword).or_default();
        let origin = SchemaConstraintOrigin {
            source: source.clone(),
            path: path.clone(),
            keyword: keyword.to_string(),
            occurrence: *occurrence,
            span: None,
        };
        *occurrence += 1;
        origin
    };
    let value = value.iter().map(&mut origin).collect();
    let array = array.iter().map(&mut origin).collect();
    (value, array)
}

/// The `$constraints` key that authors `constraint` (inverse of
/// `simplified::parse_dollar_constraints`).
fn dollar_constraints_key(constraint: &Constraint) -> &'static str {
    match constraint {
        Constraint::MinKeys(_) => "min-keys",
        Constraint::MaxKeys(_) => "max-keys",
        Constraint::MinItems(_) => "min-items",
        Constraint::MaxItems(_) => "max-items",
        other => other.keyword(),
    }
}

/// The mapping key a pattern property is authored under, which is its
/// segment in a [`SchemaSourcePath`].
fn pattern_key_source_name(key: &PatternKey) -> String {
    match key {
        PatternKey::CatchAll => "<string>".to_string(),
        PatternKey::Starting(prefix) => format!("<starting::{prefix}>"),
        PatternKey::Ending(suffix) => format!("<ending::{suffix}>"),
        PatternKey::Pattern(pattern) => format!("<pattern::{pattern}>"),
    }
}

/// Expands every `Name@file` import in `schema`. `base_dir` and `key` describe
/// the document being resolved: `base_dir` anchors relative imports and `@this`
/// resolves against `key` and the document's own top-level named types.
/// `root` is the declaration's path in its source (a root-union arm of an
/// inline document is `[N]`), so reported paths match the source map.
/// `schema_roots` provides bare-name resolution context for import targets
/// that have no path component.
///
/// Returns the import-free schema plus the sorted dependency edges, and adds
/// every location probed for an import target or a rehomed example to
/// `searched`. Schemas with no imports are returned untouched
/// (byte-equivalent to legacy output).
///
/// The cyclic definitions the schema reaches are added to `definitions`,
/// which allocates their `$defs` keys; a root union passes one table to
/// every inline arm so their keys never collide.
#[allow(clippy::too_many_arguments)]
fn expand_document_imports(
    schema: SimplifiedSchema,
    base_dir: &Path,
    key: DefNamespace,
    root: &SchemaSourcePath,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    searched: &mut BTreeSet<PathBuf>,
    definitions: &mut DefinitionTable,
) -> Result<(SimplifiedSchema, Vec<PathBuf>), SchemaError> {
    let source = NamespaceSource::Files {
        schema_roots: schema_roots.to_vec(),
        context: Box::new(request_context.clone()),
    };
    expand_imports(schema, base_dir, key, root, source, searched, definitions)
}

/// [`expand_document_imports`] over an explicit namespace source.
///
/// The named-type graph is discovered first. Without a cycle, expansion
/// inlines every reference exactly as it always has. With one, every use of a
/// cyclic definition becomes a [`TypeExpr::Ref`] and expansion runs twice:
/// the first pass learns what each cyclic definition contributes to a use
/// site (its markers and outer shape, see `recursive::UseSummary`), and the
/// second builds every use site from that summary with full provenance.
fn expand_imports(
    schema: SimplifiedSchema,
    base_dir: &Path,
    key: DefNamespace,
    root: &SchemaSourcePath,
    source: NamespaceSource<'_>,
    searched: &mut BTreeSet<PathBuf>,
    definitions: &mut DefinitionTable,
) -> Result<(SimplifiedSchema, Vec<PathBuf>), SchemaError> {
    if !schema_has_imports(&schema) {
        return Ok((schema, Vec::new()));
    }
    // `@this` resolves against the document's own (unexpanded) named types.
    // `schema` is consumed by expansion below, so this is the one copy.
    let named_types = Arc::new(match &schema {
        SimplifiedSchema::Single(shape) => shape.properties.clone(),
        SimplifiedSchema::Union(_) => IndexMap::new(),
    });
    let current = Namespace {
        key,
        base_dir: base_dir.to_path_buf(),
        named_types,
    };
    let mut engine = ImportEngine::new(source);
    engine.cyclic = engine.discover(&schema, &current, root)?;
    let expanded = if engine.cyclic.is_empty() {
        engine.expand_schema(schema, &current, root)?
    } else {
        engine.expand_schema(schema.clone(), &current, root)?;
        engine.expand_reached_definitions()?;
        let summaries = engine.summarize()?;
        engine.begin_final_pass(summaries);
        let expanded = engine.expand_schema(schema, &current, root)?;
        engine.expand_reached_definitions()?;
        engine.publish_definitions(definitions);
        expanded
    };
    searched.extend(engine.searched);
    Ok((expanded, engine.dependencies.into_iter().collect()))
}

impl ImportEngine<'_> {
    fn expand_schema(
        &mut self,
        schema: SimplifiedSchema,
        current: &Namespace,
        root: &SchemaSourcePath,
    ) -> Result<SimplifiedSchema, SchemaError> {
        match schema {
            SimplifiedSchema::Single(shape) => Ok(SimplifiedSchema::Single(
                self.expand_shape(shape, current, root)?,
            )),
            SimplifiedSchema::Union(arms) => {
                let mut out = Vec::with_capacity(arms.len());
                for (index, arm) in arms.into_iter().enumerate() {
                    out.push(match arm {
                        SchemaArm::Inline(shape) => SchemaArm::Inline(self.expand_shape(
                            shape,
                            current,
                            &root.union_arm(index),
                        )?),
                        // File-ref arms are inlined by the root-union resolver,
                        // not here; leave them for `to_json_schema` to reject if
                        // one somehow survives.
                        SchemaArm::FileRef(s) => SchemaArm::FileRef(s),
                    });
                }
                Ok(SimplifiedSchema::Union(out))
            }
        }
    }

    fn expand_shape(
        &mut self,
        shape: SchemaShape,
        current: &Namespace,
        path: &SchemaSourcePath,
    ) -> Result<SchemaShape, SchemaError> {
        let mut properties = IndexMap::with_capacity(shape.properties.len());
        for (name, def) in shape.properties {
            let property_path = path.property(name.as_str());
            let def = self.expand_property_def(def, current, &property_path)?;
            properties.insert(name, def);
        }
        let mut pattern_keys = Vec::with_capacity(shape.pattern_keys.len());
        for pk in shape.pattern_keys {
            let property_path = path.property(pattern_key_source_name(&pk.key));
            pattern_keys.push(PatternKeyDef {
                def: self.expand_property_def(pk.def, current, &property_path)?,
                key: pk.key,
            });
        }
        Ok(SchemaShape {
            properties,
            pattern_keys,
            constraints: shape.constraints,
        })
    }

    fn expand_property_def(
        &mut self,
        def: PropertyDef,
        current: &Namespace,
        path: &SchemaSourcePath,
    ) -> Result<PropertyDef, SchemaError> {
        match def {
            PropertyDef::Single(atom) => Ok(self.expand_atom(atom, current, path)?.into_def()),
            PropertyDef::Union(arms) => Ok(self
                .expand_union_arms(arms, current, path)?
                .into_expanded()
                .into_def()),
        }
    }

    /// Expands the arms of a property-level union. A union-typed named type
    /// flattens into the enclosing union, which can bring in a second
    /// `suggest(...)` the authored union never showed, so the at-most-one
    /// rule is checked again over the flattened arms.
    ///
    /// A postfixed union arm (`[U(required), T]`, `[U -> text, T]`) flattens
    /// too, so every consumer of the lowered `anyOf` sees one list of
    /// alternatives. Its markers are property-level and are returned in
    /// [`FlatUnion::markers`] rather than copied onto its arms, where an
    /// `eager` would turn a `file` arm into `file(eager)`. Its description
    /// describes each of its arms that has none of its own.
    fn expand_union_arms(
        &mut self,
        arms: Vec<PropertyAtom>,
        current: &Namespace,
        path: &SchemaSourcePath,
    ) -> Result<FlatUnion, SchemaError> {
        let mut out = Vec::with_capacity(arms.len());
        let mut markers = Vec::new();
        for (index, atom) in arms.into_iter().enumerate() {
            match self.expand_atom(atom, current, &path.union_arm(index))? {
                ExpandedDef::Single(atom, provenance) => match *atom {
                    PropertyAtom {
                        ty: TypeExpr::Union(nested),
                        is_array: false,
                        constraints,
                        array_constraints,
                        description,
                    } => {
                        for marker in constraints
                            .into_iter()
                            .zip(provenance.value)
                            .chain(array_constraints.into_iter().zip(provenance.array))
                        {
                            add_marker(&mut markers, marker);
                        }
                        out.extend(nested.into_iter().map(|mut arm| {
                            if arm.description.is_none() {
                                arm.description.clone_from(&description);
                            }
                            arm
                        }));
                    }
                    atom => out.push(atom),
                },
                ExpandedDef::Union(mut nested) => out.append(&mut nested),
            }
        }
        let suggestions = out
            .iter()
            .flat_map(PropertyAtom::property_constraints)
            .filter(|constraint| matches!(constraint, Constraint::Suggest(_)))
            .count();
        if suggestions > 1 {
            // An inherited candidate's span belongs to another source, so no
            // span is reported; the property path names the location.
            return Err(SchemaError::Grammar {
                property: path.to_string(),
                message: "a property definition may contain at most one `suggest` constraint"
                    .into(),
                span: 0..0,
            });
        }
        Ok(FlatUnion { arms: out, markers })
    }

    fn expand_atom(
        &mut self,
        atom: PropertyAtom,
        current: &Namespace,
        path: &SchemaSourcePath,
    ) -> Result<ExpandedDef, SchemaError> {
        let PropertyAtom {
            ty,
            is_array,
            constraints,
            array_constraints,
            description,
        } = atom;
        let mut hoisted = Vec::new();
        let ty = match ty {
            TypeExpr::Imported { name, reference } => {
                return self.expand_import(
                    NamedReference {
                        name,
                        reference,
                        is_array,
                        constraints,
                        array_constraints,
                        description,
                    },
                    current,
                    path,
                );
            }
            TypeExpr::Primitive(p) => TypeExpr::Primitive(p),
            TypeExpr::InlineObject(shape) => {
                TypeExpr::InlineObject(self.expand_shape(shape, current, path)?)
            }
            TypeExpr::Union(arms) => {
                let flat = self.expand_union_arms(arms, current, path)?;
                hoisted = flat.markers;
                TypeExpr::Union(flat.arms)
            }
            TypeExpr::Tuple(tuple) => TypeExpr::Tuple(tuple.try_map_defs(|index, def| {
                self.expand_property_def(def, current, &path.union_arm(index))
            })?),
            TypeExpr::Function(function) => TypeExpr::Function(function),
            TypeExpr::Ref(key) => TypeExpr::Ref(key),
        };
        let mut atom = PropertyAtom {
            ty,
            is_array,
            constraints,
            array_constraints,
            description,
        };
        let mut provenance = AtomProvenance::authored(&atom, &current.key.source_id(), path);
        // Markers hoisted off a flattened union arm belong to the property:
        // the container's list for an array of the union.
        let (list, origins) = if atom.is_array {
            (&mut atom.array_constraints, &mut provenance.array)
        } else {
            (&mut atom.constraints, &mut provenance.value)
        };
        let mut existing: Vec<Tagged> = std::mem::take(list).into_iter().zip(std::mem::take(origins)).collect();
        for marker in hoisted {
            add_marker(&mut existing, marker);
        }
        (*list, *origins) = existing.into_iter().unzip();
        Ok(ExpandedDef::Single(Box::new(atom), provenance))
    }

    /// Expands the reference written at `path` in `current`.
    ///
    /// The definition is expanded in its own namespace, so a bare name inside
    /// it resolves in the file that defines it, and its inherited constraints
    /// keep the origin of the declaration that introduced them.
    fn expand_import(
        &mut self,
        reference: NamedReference,
        current: &Namespace,
        path: &SchemaSourcePath,
    ) -> Result<ExpandedDef, SchemaError> {
        if self.stack.len() >= MAX_IMPORT_DEPTH {
            return Err(SchemaError::ImportCycle {
                chain: format!(
                    "named-type import expansion exceeded {MAX_IMPORT_DEPTH} levels near \
                     `{}@{}`",
                    reference.name, reference.reference
                ),
            });
        }

        let target = self.resolve_namespace(&reference.reference, current)?;
        let site = SchemaReferenceSite {
            source: current.key.source_id(),
            path: path.clone(),
        };
        let Some(def) = target.named_types.get(&reference.name) else {
            // A missing named type is a resolution error, not a grammar error:
            // the reference parsed fine, its namespace simply does not define it.
            return Err(SchemaError::UnknownNamedType {
                name: reference.name,
                namespace: target.key.source_id(),
                site,
            });
        };

        let frame = DefKey::new(target.key.clone(), reference.name.clone());
        let definition_path = SchemaSourcePath::root().property(reference.name.as_str());
        let definition = SchemaReferenceSite {
            source: target.key.source_id(),
            path: definition_path.clone(),
        };
        if self.cyclic.contains(&frame) {
            self.reach(&frame, &target);
            return self.recursive_use(frame, reference, &site, &definition);
        }
        let base = if let Some(expanded) = self.expanded_cache.get(&frame) {
            expanded.clone()
        } else {
            // Admission rejected every cycle that does not pass through a
            // cyclic definition, so this guards an invariant.
            if self.stack.contains(&frame) {
                return Err(SchemaError::ImportCycle {
                    chain: self.describe_cycle(&reference.name),
                });
            }
            self.stack.push(frame.clone());
            // Cloned only here: a cached expansion never reads the body.
            let base = match def.clone() {
                PropertyDef::Single(atom) => self.expand_atom(atom, &target, &definition_path)?,
                PropertyDef::Union(arms) => {
                    self.expand_union_arms(arms, &target, &definition_path)?.into_expanded()
                }
            };
            self.stack.pop();

            let base = self.rehome_definition_examples(base, &target.key)?;
            self.expanded_cache.insert(frame, base.clone());
            base
        };
        self.apply_postfix(base, reference, &site, &definition)
    }

    /// Pins the examples of a definition expanded from a schema file (see
    /// [`Self::rehome_examples`]); other namespaces have none to pin.
    fn rehome_definition_examples(
        &mut self,
        base: ExpandedDef,
        namespace: &DefNamespace,
    ) -> Result<ExpandedDef, SchemaError> {
        match namespace {
            DefNamespace::File(file) | DefNamespace::FileRootUnionArm(file, _) => {
                self.rehome_examples(base, &file.clone())
            }
            // An embedded definition's examples are rejected once the
            // embedded schema is lowered (`prepare_embedded_schema`).
            DefNamespace::InlineRoot
            | DefNamespace::RootUnionArm(_)
            | DefNamespace::TriggerPayload(_)
            | DefNamespace::TriggerPayloadUnionArm(..)
            | DefNamespace::Embedded(_)
            | DefNamespace::Shipped(_) => Ok(base),
        }
    }

    /// Pins the `example(...)` references of a definition inlined from
    /// `file` to the paths they name from that file.
    ///
    /// Examples are resolved after conversion against the top-level schema's
    /// directory (`resolve_document_examples`), which is the wrong anchor for
    /// a definition authored elsewhere. Each reference is resolved here,
    /// through the same ladder, from `file`'s directory (`this` is `file`),
    /// and replaced by the resolved absolute path, which resolves to itself
    /// from any directory. A missing example fails here with the error the
    /// later pass would have raised.
    fn rehome_examples(&mut self, base: ExpandedDef, file: &Path) -> Result<ExpandedDef, SchemaError> {
        let NamespaceSource::Files { schema_roots, context } = &self.source else {
            unreachable!("a file namespace is only loaded from the file source")
        };
        let searched = &mut self.searched;
        let base_dir = file.parent().unwrap_or_else(|| Path::new("."));
        let mut rehome = |atom: &mut PropertyAtom| {
            rehome_atom_examples(atom, base_dir, file, schema_roots, context, searched)
        };
        Ok(match base {
            ExpandedDef::Single(mut atom, provenance) => {
                rehome(&mut atom)?;
                ExpandedDef::Single(atom, provenance)
            }
            ExpandedDef::Union(mut arms) => {
                for atom in &mut arms {
                    rehome(atom)?;
                }
                ExpandedDef::Union(arms)
            }
        })
    }

    /// Resolves the namespace named on the right of `@`. `this` targets the
    /// current namespace, and `darkmatter:<stem>` a shipped partial
    /// ([`embedded::SHIPPED_NAMESPACE`]); a reference written inside a shipped
    /// partial resolves lexically among the shipped documents. Any other value
    /// is a `biscuit-file` reference resolved relative to `current.base_dir`
    /// (mirroring `$schema` file-ref resolution). Bare-name references (no
    /// path component) resolve against the schema-root context nearest-first
    /// when roots are configured (Phase 3).
    fn resolve_namespace(
        &mut self,
        reference: &str,
        current: &Namespace,
    ) -> Result<Namespace, SchemaError> {
        let trimmed = reference.trim();
        if trimmed == "this" {
            return Ok(Namespace {
                key: current.key.clone(),
                base_dir: current.base_dir.clone(),
                named_types: Arc::clone(&current.named_types),
            });
        }
        if let Some(stem) = embedded::reserved_stem(trimmed) {
            let document = embedded::shipped_partial(stem).ok_or_else(|| SchemaError::NoMatch {
                reference: trimmed.to_string(),
                bare_name: false,
            })?;
            return self.shipped_namespace(document.path);
        }
        if let DefNamespace::Shipped(importer) = &current.key {
            let target = embedded::resolve_path(importer, reference).map_err(|reason| SchemaError::SchemaDocument {
                path: PathBuf::from(importer),
                message: format!("embedded import `{trimmed}`: {reason}"),
            })?;
            let document = embedded::find(embedded::SHIPPED, &target).ok_or_else(|| SchemaError::SchemaDocument {
                path: PathBuf::from(importer),
                message: format!("embedded import `{trimmed}`: `{target}` is not a shipped schema document"),
            })?;
            return self.shipped_namespace(document.path);
        }
        let (path, canonical) = match &self.source {
            NamespaceSource::Files { schema_roots, context } => {
                let site = (reference.to_string(), current.base_dir.clone());
                if let Some(selected) = self.selected_files.get(&site) {
                    self.searched.extend(selected.probed.iter().cloned());
                    (selected.path.clone(), selected.canonical.clone())
                } else {
                    let mut probed = BTreeSet::new();
                    let selected = named_type_file_path(reference, &current.base_dir, schema_roots, context, &mut probed);
                    self.searched.extend(probed.iter().cloned());
                    let path = selected?;
                    let canonical = canonical_path(&path);
                    self.selected_files.insert(
                        site,
                        SelectedFile {
                            path: path.clone(),
                            canonical: canonical.clone(),
                            probed: probed.into_iter().collect(),
                        },
                    );
                    (path, canonical)
                }
            }
            NamespaceSource::Embedded { documents, bundle } => {
                let (documents, bundle) = (*documents, *bundle);
                let DefNamespace::Embedded(importer) = &current.key else {
                    unreachable!("an embedded expansion holds only embedded namespaces")
                };
                return self.resolve_embedded_namespace(documents, bundle, importer, reference);
            }
        };

        self.dependencies.insert(canonical.clone());
        // Reuse an already-loaded namespace for this file: multiple import sites
        // targeting the same file (`A@types.yaml`, `B@types.yaml`, …) otherwise
        // re-read and re-parse it once each.
        if let Some(cached) = self.namespace_cache.get(&canonical) {
            return Ok(Namespace {
                key: DefNamespace::File(canonical),
                base_dir: cached.base_dir.clone(),
                named_types: Arc::clone(&cached.named_types),
            });
        }
        let named_types = load_named_types(&path)?;
        let base_dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        self.namespace_cache.insert(
            canonical.clone(),
            CachedNamespace {
                base_dir: base_dir.clone(),
                named_types: Arc::clone(&named_types),
            },
        );
        Ok(Namespace {
            key: DefNamespace::File(canonical),
            base_dir,
            named_types,
        })
    }

    /// Resolves `reference`, written in the embedded document `importer`,
    /// against the embedded table only. The target is recorded as a virtual
    /// path, the embedded expansion's read set; it is never a file
    /// dependency edge, since embedded documents are immutable build inputs.
    fn resolve_embedded_namespace(
        &mut self,
        documents: &[EmbeddedDocument<'_>],
        bundle: &str,
        importer: &str,
        reference: &str,
    ) -> Result<Namespace, SchemaError> {
        let invalid = |reason: String| SchemaError::SchemaDocument {
            path: PathBuf::from(importer),
            message: format!("embedded import `{}`: {reason}", reference.trim()),
        };
        let target = if bundle.is_empty() {
            embedded::resolve_path(importer, reference)
        } else {
            embedded::resolve_bundle_path(bundle, importer, reference)
        }
        .map_err(invalid)?;
        self.dependencies.insert(PathBuf::from(&target));
        let named_types = match self.embedded_cache.get(&target) {
            Some(cached) => Arc::clone(cached),
            None => {
                let document = embedded::find(documents, &target).ok_or_else(|| {
                    invalid(format!("`{target}` is not an embedded schema document"))
                })?;
                let named_types =
                    Arc::new(named_types_from_text(document.text, Path::new(document.path))?);
                self.embedded_cache.insert(target.clone(), Arc::clone(&named_types));
                named_types
            }
        };
        Ok(Namespace {
            key: DefNamespace::Embedded(target),
            base_dir: PathBuf::new(),
            named_types,
        })
    }

    /// The shipped partial registered at `path` as a namespace. An embedded
    /// expansion records it in its read set; a disk expansion records no
    /// dependency edge, since a shipped document is an immutable build input
    /// and names no host file.
    fn shipped_namespace(&mut self, path: &str) -> Result<Namespace, SchemaError> {
        if matches!(self.source, NamespaceSource::Embedded { .. }) {
            self.dependencies.insert(PathBuf::from(path));
        }
        let named_types = match self.shipped_cache.get(path) {
            Some(cached) => Arc::clone(cached),
            None => {
                let named_types = shipped_named_types(path)?;
                self.shipped_cache.insert(path.to_string(), Arc::clone(&named_types));
                named_types
            }
        };
        Ok(Namespace {
            key: DefNamespace::Shipped(path.to_string()),
            base_dir: PathBuf::new(),
            named_types,
        })
    }

    /// Renders the current expansion stack as a `a -> b -> a` chain for the
    /// cycle-error message.
    fn describe_cycle(&self, repeat: &str) -> String {
        let mut parts: Vec<&str> = self.stack.iter().map(|frame| frame.name.as_str()).collect();
        parts.push(repeat);
        parts.join(" -> ")
    }
}

/// The parsed named types of each shipped partial, shared by every
/// expansion in the process: a shipped document is compiled in, so its parse
/// never changes. Failures are not kept.
static SHIPPED_NAMED_TYPES: std::sync::LazyLock<std::sync::Mutex<HashMap<&'static str, Arc<NamedTypes>>>> =
    std::sync::LazyLock::new(Default::default);

/// A schema document's top-level named types, unexpanded.
type NamedTypes = IndexMap<String, PropertyDef>;

/// Empties the process-wide named-type parse memos (see
/// [`crate::markdown::schemas::reset_process_caches_for_bench`]).
pub(crate) fn reset_process_caches() {
    SHIPPED_NAMED_TYPES.lock().expect("shipped partial memo poisoned").clear();
    FILE_NAMED_TYPES.lock().expect("named-type file memo poisoned").clear();
}

/// The named types of the shipped partial registered at `path`.
fn shipped_named_types(path: &str) -> Result<Arc<NamedTypes>, SchemaError> {
    let document = embedded::find(embedded::SHIPPED, path).expect("the caller found the shipped document");
    if let Some(cached) = SHIPPED_NAMED_TYPES.lock().expect("shipped partial memo poisoned").get(document.path) {
        return Ok(Arc::clone(cached));
    }
    let named_types = Arc::new(named_types_from_text(document.text, Path::new(document.path))?);
    SHIPPED_NAMED_TYPES
        .lock()
        .expect("shipped partial memo poisoned")
        .insert(document.path, Arc::clone(&named_types));
    Ok(named_types)
}

/// The file an explicit named-type reference (`Name@reference`) selects,
/// resolved exactly as the resolver selects it: schema roots nearest-first for
/// a bare file name when roots are configured, else through `request_context`
/// from `base_dir` (the directory of the schema that wrote the reference).
fn named_type_file_path(
    reference: &str,
    base_dir: &Path,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    searched: &mut BTreeSet<PathBuf>,
) -> Result<PathBuf, SchemaError> {
    let trimmed = reference.trim();
    let file_ref =
        FileReference::new(trimmed).map_err(|source| SchemaError::Unresolved {
            reference: reference.to_string(),
            source,
        })?;
    if file_ref.class().kind == FileReferenceKind::Url {
        return Err(SchemaError::RemoteUnsupported {
            reference: reference.to_string(),
        });
    }

    // Bare-name resolution against schema roots (Phase 3).
    let path = if !schema_roots.is_empty() && is_bare_name(&file_ref) {
        if let Some(p) = try_bare_name_in_roots(
            &file_ref,
            schema_roots,
            request_context,
            searched,
        )? {
            p
        } else {
            // Check sibling for pointed error.
            let sibling = base_dir.join(trimmed);
            if sibling.is_file() {
                return Err(SchemaError::BareNameSiblingExists {
                    reference: trimmed.to_string(),
                    suggestion: format!("./{trimmed}"),
                });
            }
            return Err(SchemaError::NoMatch {
                reference: reference.to_string(),
                bare_name: true,
            });
        }
    } else {
        resolve_file_reference_in_context(
            &file_ref,
            base_dir,
            request_context,
            searched,
        )
            .map_err(|source| SchemaError::Unresolved {
                reference: trimmed.to_string(),
                source,
            })?
            .ok_or_else(|| SchemaError::NoMatch {
                reference: trimmed.to_string(),
                bare_name: false,
            })?
    };
    Ok(path)
}

/// Rewrites every `example(...)` reference in `atom`, including those of
/// nested inline-object properties and union arms, to the path it resolves to from
/// `base_dir` (see [`ImportEngine::rehome_examples`]). A path that is not
/// valid UTF-8 cannot be spelled as a reference and is left as authored.
fn rehome_atom_examples(
    atom: &mut PropertyAtom,
    base_dir: &Path,
    this_file: &Path,
    schema_roots: &[PathBuf],
    request_context: &FileResolutionContext,
    searched: &mut BTreeSet<PathBuf>,
) -> Result<(), SchemaError> {
    for constraint in atom
        .constraints
        .iter_mut()
        .chain(atom.array_constraints.iter_mut())
    {
        if let Constraint::Example(references) = constraint {
            for reference in references.iter_mut() {
                let path = resolve_example_path(
                    reference,
                    base_dir,
                    Some(this_file),
                    schema_roots,
                    request_context,
                    searched,
                )?;
                if let Some(resolved) = path.to_str() {
                    *reference = resolved.to_string();
                }
            }
        }
    }
    if let TypeExpr::Union(arms) = &mut atom.ty {
        // An arm inlined from another file was already rehomed there; its
        // absolute paths resolve to themselves.
        for arm in arms {
            rehome_atom_examples(arm, base_dir, this_file, schema_roots, request_context, searched)?;
        }
    }
    if let TypeExpr::InlineObject(shape) = &mut atom.ty {
        let defs = shape
            .properties
            .values_mut()
            .chain(shape.pattern_keys.iter_mut().map(|pattern| &mut pattern.def));
        for def in defs {
            match def {
                PropertyDef::Single(inner) => {
                    rehome_atom_examples(
                        inner,
                        base_dir,
                        this_file,
                        schema_roots,
                        request_context,
                        searched,
                    )?;
                }
                PropertyDef::Union(arms) => {
                    for inner in arms {
                        rehome_atom_examples(
                            inner,
                            base_dir,
                            this_file,
                            schema_roots,
                            request_context,
                            searched,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Canonicalizes `path` for stable cycle-detection identity, falling back to
/// the given path when canonicalization fails (it should not, since the path
/// was just resolved on disk).
fn canonical_path(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Loads a standalone schema file's mapping payload as named types.
///
/// ## Errors
///
/// Returns [`SchemaError::AmbiguousReferenced`] when the file is not a
/// SimplifiedSchema mapping (raw JSON Schema or a root union) —
/// imports are only defined among a file's named types, so a raw JSON Schema
/// has none to offer.
fn load_named_types(path: &Path) -> Result<Arc<NamedTypes>, SchemaError> {
    let bytes = fs::read(path).map_err(|source| SchemaError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|_| SchemaError::AmbiguousReferenced {
        path: path.to_path_buf(),
    })?;
    let key = (path.to_path_buf(), biscuit_hash::xx_hash_bytes(&bytes));
    if let Some(cached) = FILE_NAMED_TYPES.lock().expect("named-type file memo poisoned").get(&key) {
        return Ok(Arc::clone(cached));
    }
    let named_types = Arc::new(named_types_from_text(text, path)?);
    let mut memo = FILE_NAMED_TYPES.lock().expect("named-type file memo poisoned");
    if memo.len() >= FILE_NAMED_TYPES_CAPACITY {
        memo.clear();
    }
    memo.insert(key, Arc::clone(&named_types));
    Ok(named_types)
}

/// How many parsed schema files [`FILE_NAMED_TYPES`] holds before it starts
/// over.
const FILE_NAMED_TYPES_CAPACITY: usize = 256;

/// The parsed named types of each schema file an import read, keyed by the
/// path it was read from and a hash of the bytes read, so every payload and
/// document importing one file revision parses it once. The file is read on
/// every use: an edit or deletion is seen at once, and a failed read or parse
/// is never served from here.
static FILE_NAMED_TYPES: std::sync::LazyLock<std::sync::Mutex<HashMap<FileRevision, Arc<NamedTypes>>>> =
    std::sync::LazyLock::new(Default::default);

/// A file path and the hash of the bytes read from it.
type FileRevision = (PathBuf, u64);

/// The named types a schema document's text declares; `path` identifies the
/// document in errors.
fn named_types_from_text(text: &str, path: &Path) -> Result<IndexMap<String, PropertyDef>, SchemaError> {
    // The classifier declines unparseable YAML; an import target must be
    // a schema, so name the syntax error instead of "ambiguous".
    if let Err(error) = serde_yaml_ng::from_str::<YamlValue>(text) {
        return Err(SchemaError::SchemaDocument {
            path: path.to_path_buf(),
            message: format!("not valid YAML: {error}"),
        });
    }
    let Some(document) = parse_standalone_schema_document(text, path)? else {
        return Err(SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        });
    };
    match document.declaration {
        SchemaDeclaration::Schema(SimplifiedSchema::Single(shape)) => Ok(shape.properties),
        SchemaDeclaration::Schema(SimplifiedSchema::Union(_)) => Err(SchemaError::SchemaDocument {
            path: path.to_path_buf(),
            message: "root-union schema documents cannot supply named imports".into(),
        }),
        // A whole-file reference declares no named types of its own; imports
        // are defined only among a file's own top-level entries.
        SchemaDeclaration::Reference(_) => Err(SchemaError::SchemaDocument {
            path: path.to_path_buf(),
            message: "whole-file reference schema documents cannot supply named imports".into(),
        }),
    }
}

// ── Use-site constraint merging ─────────────────────────────────────────────

/// A constraint paired with the declaration that authored it.
type Tagged = (Constraint, SchemaConstraintOrigin);

/// The reference a merge belongs to, for contradiction errors.
struct MergeSite<'a> {
    name: &'a str,
    site: &'a SchemaReferenceSite,
}

impl MergeSite<'_> {
    fn conflict(
        &self,
        kind: SchemaConstraintConflictKind,
        mut constraints: Vec<SchemaConstraintOrigin>,
    ) -> SchemaError {
        locate_origin_spans(&mut constraints);
        SchemaError::ConstraintConflict {
            name: self.name.to_string(),
            kind,
            site: self.site.clone(),
            constraints,
        }
    }
}

/// Fills the span of each origin authored in a schema file by re-reading and
/// re-projecting that file. Runs only on the error path. A file that can no
/// longer be read or projected leaves its spans `None`; the error being built
/// still names the file, path, and keyword.
fn locate_origin_spans(origins: &mut [SchemaConstraintOrigin]) {
    let mut documents: HashMap<PathBuf, Option<(String, SchemaSourceMap)>> = HashMap::new();
    for origin in origins.iter_mut() {
        let SchemaSourceId::File(path) = &origin.source else {
            continue;
        };
        if origin.span.is_some() {
            continue;
        }
        let document = documents.entry(path.clone()).or_insert_with(|| {
            let text = fs::read_to_string(path).ok()?;
            let source_map = parse_standalone_schema_document(&text, path).ok()??.source_map;
            Some((text, source_map))
        });
        if let Some((text, source_map)) = document {
            origin.span =
                source_map.constraint_span(text, &origin.path, &origin.keyword, origin.occurrence);
        }
    }
}

/// Applies the use-site postfix (`[]`, `(constraints)`, `-> description`) of
/// `reference` to its expanded definition.
///
/// The definition is inherited verbatim when the use site adds no postfix.
/// A postfix over a [`UnionTarget`] (a union definition or a postfixed union
/// alias) is applied by [`apply_union_postfix`]. Over a single atom, use-site constraints are
/// interpreted against the resolved target and merged with the inherited
/// ones (Decided 4):
///
/// - A use-site `min`/`max` counts characters on a `string` target and items
///   on a target that is already an array, whose postfix addresses the
///   container; `suggest` candidates there still describe items.
/// - Repeated bounds of one kind keep the stricter value; a lower bound above
///   the upper bound is a [`SchemaError::ConstraintConflict`]. For an inline
///   object, `$constraints` key bounds and postfix key bounds are one scope.
/// - Flags are idempotent; every distinct `pattern`, every `match` group, and
///   every example is kept, inherited first.
/// - Repeated `scheme` lists intersect and repeated defaults must be equal.
/// - Item constraints and the constraints of a container added by `Name[]`
///   never merge with each other.
fn apply_import_postfix(
    base: ExpandedDef,
    reference: NamedReference,
    site: &SchemaReferenceSite,
    definition: &SchemaReferenceSite,
) -> Result<ExpandedDef, SchemaError> {
    if !reference.is_array
        && reference.constraints.is_empty()
        && reference.array_constraints.is_empty()
        && reference.description.is_none()
    {
        return Ok(base);
    }

    let (base_atom, base_provenance) = match UnionTarget::of(base) {
        Ok(target) => return apply_union_postfix(target, reference, site, definition),
        Err((atom, provenance)) => (*atom, provenance),
    };
    let NamedReference {
        name,
        reference,
        is_array,
        mut constraints,
        array_constraints,
        description,
    } = reference;

    // `alphanumeric` options belong to the target's identity, not to its
    // constraints: a use site may repeat them exactly but never change them.
    if constraints.iter().any(|constraint| matches!(constraint, Constraint::Options(_))) {
        let (use_value_origins, _) =
            authored_origins(&constraints, &array_constraints, &site.source, &site.path);
        let (options, rest): (Vec<_>, Vec<_>) = constraints
            .into_iter()
            .zip(use_value_origins)
            .partition(|(constraint, _)| matches!(constraint, Constraint::Options(_)));
        constraints = rest.into_iter().map(|(constraint, _)| constraint).collect();
        check_use_site_options(
            &base_atom,
            &base_provenance,
            options,
            &MergeSite { name: &name, site },
            definition,
            &reference,
        )?;
    }

    // Only a union target gives a list before `[]` a meaning (its property
    // markers); item constraints on a single named type are not supported.
    if is_array && let Some(first) = constraints.first() {
        return Err(SchemaError::Convert {
            property: site.path.to_string(),
            message: format!(
                "constraint `{}` before `[]` is not valid on the named type `{name}@{reference}`; \
                 only a named union accepts `required`, `generated`, or `eager` there, and \
                 constraints after `[]` apply to the list",
                first.keyword()
            ),
        });
    }

    if is_array
        && array_constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::Suggest(_)))
    {
        return Err(SchemaError::Grammar {
            property: name.to_string(),
            message: "`suggest` is not valid at the array level".into(),
            span: array_constraints
                .iter()
                .find_map(|constraint| match constraint {
                    Constraint::Suggest(candidates) => candidates.first().map(|candidate| candidate.span.clone()),
                    _ => None,
                })
                .unwrap_or(0..0),
        });
    }

    if constraints
        .iter()
        .any(|constraint| matches!(constraint, Constraint::Suggest(_)))
    {
        let TypeExpr::Primitive(
            ty @ (SimplifiedType::String | SimplifiedType::Number | SimplifiedType::Alphanumeric),
        ) = &base_atom.ty
        else {
            return Err(SchemaError::Grammar {
                property: name.to_string(),
                message: format!(
                    "`suggest` on imported type `{name}@{reference}` requires an exact `string`, `number`, or `alphanumeric` target"
                ),
                span: constraints
                    .iter()
                    .find_map(|constraint| match constraint {
                        Constraint::Suggest(candidates) => {
                            candidates.first().map(|candidate| candidate.span.clone())
                        }
                        _ => None,
                    })
                    .unwrap_or(0..0),
            });
        };
        for constraint in &mut constraints {
            if let Constraint::Suggest(candidates) = constraint {
                super::simplified::grammar::reinterpret_suggestion_candidates(
                    candidates,
                    *ty,
                    &name,
                )?;
            }
        }
    }

    let suggestion_count = base_atom
        .constraints
        .iter()
        .chain(constraints.iter())
        .filter(|constraint| matches!(constraint, Constraint::Suggest(_)))
        .count();
    if suggestion_count > 1 {
        let span = constraints
            .iter()
            .find_map(|constraint| match constraint {
                Constraint::Suggest(candidates) => candidates.first().map(|candidate| candidate.span.clone()),
                _ => None,
            })
            .unwrap_or(0..0);
        return Err(SchemaError::Grammar {
            property: name.to_string(),
            message: "a property definition may contain at most one `suggest` constraint".into(),
            span,
        });
    }

    let property = site.path.to_string();
    let merge_site = MergeSite { name: &name, site };
    let (use_value_origins, use_array_origins) =
        authored_origins(&constraints, &array_constraints, &site.source, &site.path);
    let use_site: Vec<Tagged> = constraints
        .into_iter()
        .zip(use_value_origins)
        .chain(array_constraints.into_iter().zip(use_array_origins))
        .collect();

    let PropertyAtom {
        mut ty,
        is_array: base_is_array,
        constraints: base_value,
        array_constraints: base_array,
        description: base_description,
    } = base_atom;
    let AtomProvenance {
        value: base_value_origins,
        array: base_array_origins,
        shape: mut shape_origins,
    } = base_provenance;
    let inherited_value: Vec<Tagged> = base_value.into_iter().zip(base_value_origins).collect();
    let inherited_array: Vec<Tagged> = base_array.into_iter().zip(base_array_origins).collect();

    let (value, array) = if is_array {
        if base_is_array {
            return Err(SchemaError::Convert {
                property: name.to_string(),
                message: format!(
                    "cannot apply `[]` to the already-array named type `{name}@{reference}`"
                ),
            });
        }
        // `Name[](c)`: the constraint list follows `[]`, was parsed at the array
        // level, and applies to the new container (a list before `[]` was
        // rejected above). The inherited constraints stay on the items.
        (inherited_value, merge_constraints(Vec::new(), use_site, &merge_site)?)
    } else if base_is_array {
        let (items, container): (Vec<Tagged>, Vec<Tagged>) = use_site
            .into_iter()
            .partition(|(constraint, _)| matches!(constraint, Constraint::Suggest(_)));
        let container = reinterpret_bounds(container, CountedBound::Items, &property)?;
        (
            merge_constraints(inherited_value, items, &merge_site)?,
            merge_constraints(inherited_array, container, &merge_site)?,
        )
    } else {
        match &mut ty {
            TypeExpr::InlineObject(shape) => {
                // `$constraints` and postfix key bounds are one scope. The
                // winners go back to the shape so the converter, which applies
                // the shape's then the atom's key bounds, sees exactly one of
                // each.
                let shape_tagged: Vec<Tagged> = std::mem::take(&mut shape.constraints)
                    .into_iter()
                    .zip(std::mem::take(&mut shape_origins))
                    .collect();
                let (shape_keys, shape_other): (Vec<Tagged>, Vec<Tagged>) =
                    shape_tagged.into_iter().partition(|(c, _)| is_key_bound(c));
                let inherited = shape_keys.into_iter().chain(inherited_value).collect();
                let merged = merge_constraints(inherited, use_site, &merge_site)?;
                let (keys, value): (Vec<Tagged>, Vec<Tagged>) =
                    merged.into_iter().partition(|(c, _)| is_key_bound(c));
                (shape.constraints, shape_origins) = shape_other.into_iter().chain(keys).unzip();
                (value, inherited_array)
            }
            TypeExpr::Primitive(SimplifiedType::String | SimplifiedType::Alphanumeric) => {
                let use_site = reinterpret_bounds(use_site, CountedBound::Length, &property)?;
                (merge_constraints(inherited_value, use_site, &merge_site)?, inherited_array)
            }
            _ => (merge_constraints(inherited_value, use_site, &merge_site)?, inherited_array),
        }
    };
    let (value, array) = reconcile_property_defaults(value, array, &merge_site)?;

    let (constraints, value_origins) = value.into_iter().unzip();
    let (array_constraints, array_origins) = array.into_iter().unzip();
    Ok(ExpandedDef::Single(
        Box::new(PropertyAtom {
            ty,
            is_array: is_array || base_is_array,
            constraints,
            array_constraints,
            description: description.or(base_description),
        }),
        AtomProvenance {
            value: value_origins,
            array: array_origins,
            shape: shape_origins,
        },
    ))
}

/// A resolved definition whose value is one of a union's alternatives.
///
/// The target is a union however expansion represented it: the arms of a
/// union definition, or the non-array union atom an earlier postfixed alias
/// (`R: U(required)`, `D: U -> text`) produced. The alias's markers keep
/// their origins and, with its description, are inherited by every later
/// reference.
struct UnionTarget {
    arms: Vec<PropertyAtom>,
    value: Vec<Tagged>,
    array: Vec<Tagged>,
    description: Option<String>,
}

impl UnionTarget {
    /// Splits a union target from `base`, or returns the single atom (an
    /// array of a union included) for the ordinary single-type rules.
    fn of(base: ExpandedDef) -> Result<Self, (Box<PropertyAtom>, AtomProvenance)> {
        match base {
            ExpandedDef::Union(arms) => Ok(Self {
                arms,
                value: Vec::new(),
                array: Vec::new(),
                description: None,
            }),
            ExpandedDef::Single(atom, provenance) => match *atom {
                PropertyAtom {
                    ty: TypeExpr::Union(arms),
                    is_array: false,
                    constraints,
                    array_constraints,
                    description,
                } => Ok(Self {
                    arms,
                    value: constraints.into_iter().zip(provenance.value).collect(),
                    array: array_constraints.into_iter().zip(provenance.array).collect(),
                    description,
                }),
                atom => Err((Box::new(atom), provenance)),
            },
        }
    }
}

/// Applies a use-site postfix to a union target, producing one atom whose
/// type is the ordered, flattened [`TypeExpr::Union`].
///
/// - Without `[]` only `required`, `generated`, and `eager` apply; they and
///   the description belong to the property as a whole.
/// - Markers and a description inherited from a postfixed alias are kept;
///   with `[]` its markers move to the container, which is the property.
/// - With `[]` the list after `[]` was parsed at the array level and belongs
///   to the container; only constraints valid on an array container apply.
///   A list before `[]` may carry only the markers, which are property-level
///   and join the container exactly as they do after `[]`.
/// - `[]` is rejected when a flattened arm is already an array.
/// - With `[]`, an arm's default lifts to the container only when it is an
///   array, and merges with the other container defaults under the
///   equal-value rule; any other inherited default is rejected rather than
///   discarded or wrapped.
///
/// Arms otherwise keep their own constraints and descriptions, so an item
/// arm lowers exactly as the same arm does in a property-level union.
/// Presence and generation are read through the arms
/// ([`PropertyAtom::is_required`]), not copied onto the container.
fn apply_union_postfix(
    target: UnionTarget,
    reference: NamedReference,
    site: &SchemaReferenceSite,
    definition: &SchemaReferenceSite,
) -> Result<ExpandedDef, SchemaError> {
    let UnionTarget {
        mut arms,
        value: inherited_value,
        array: inherited_array,
        description: inherited_description,
    } = target;
    let NamedReference {
        name,
        is_array,
        constraints,
        array_constraints,
        description,
        ..
    } = reference;
    let description = description.or(inherited_description);
    let merge_site = MergeSite { name: &name, site };
    let reject = |rejection: SchemaUnionRejection| SchemaError::UnionReference {
        name: name.clone(),
        rejection: Box::new(rejection),
        site: site.clone(),
        definition: Box::new(definition.clone()),
    };

    if !is_array {
        if let Some(rejected) = constraints
            .iter()
            .chain(&array_constraints)
            .find(|constraint| !is_union_marker(constraint))
        {
            return Err(reject(SchemaUnionRejection::Constraint {
                keyword: rejected.keyword().to_string(),
                array_level: false,
            }));
        }
        let (use_value_origins, use_array_origins) =
            authored_origins(&constraints, &array_constraints, &site.source, &site.path);
        let value = merge_constraints(
            inherited_value,
            constraints.into_iter().zip(use_value_origins).collect(),
            &merge_site,
        )?;
        let array = merge_constraints(
            inherited_array,
            array_constraints.into_iter().zip(use_array_origins).collect(),
            &merge_site,
        )?;
        let (constraints, value_origins) = value.into_iter().unzip();
        let (array_constraints, array_origins) = array.into_iter().unzip();
        return Ok(ExpandedDef::Single(
            Box::new(PropertyAtom {
                ty: TypeExpr::Union(arms),
                is_array: false,
                constraints,
                array_constraints,
                description,
            }),
            AtomProvenance {
                value: value_origins,
                array: array_origins,
                shape: Vec::new(),
            },
        ));
    }

    if let Some(rejected) = constraints.iter().find(|constraint| !is_union_marker(constraint)) {
        return Err(reject(SchemaUnionRejection::Constraint {
            keyword: rejected.keyword().to_string(),
            array_level: false,
        }));
    }
    if let Some(rejected) = array_constraints
        .iter()
        .find(|constraint| !is_union_marker(constraint) && !is_array_container_constraint(constraint))
    {
        return Err(reject(SchemaUnionRejection::Constraint {
            keyword: rejected.keyword().to_string(),
            array_level: true,
        }));
    }
    let use_site: Vec<Constraint> = constraints.into_iter().chain(array_constraints).collect();
    // Postfixed union arms were flattened, so `is_array` is the arm's outer
    // shape; an array inside an object arm does not count.
    if let Some(arm) = arms.iter().position(|arm| arm.is_array) {
        return Err(reject(SchemaUnionRejection::ArrayArm { arm }));
    }

    // An arm's default is a property-level value. On an array of the union it
    // can only be the array's default, so it lifts when it is one. An alias's
    // inherited markers are property-level too, and join the container's.
    let mut lifted: Vec<Tagged> = inherited_value.into_iter().chain(inherited_array).collect();
    for (index, arm) in arms.iter_mut().enumerate() {
        for default in take_arm_defaults(arm) {
            if !default.is_array() {
                return Err(reject(SchemaUnionRejection::ScalarDefault {
                    arm: index,
                    value: default,
                }));
            }
            lifted.push((
                Constraint::Default(default),
                SchemaConstraintOrigin {
                    source: definition.source.clone(),
                    path: definition.path.union_arm(index),
                    keyword: "default".to_string(),
                    occurrence: 0,
                    span: None,
                },
            ));
        }
    }

    let (_, use_origins) = authored_origins(&[], &use_site, &site.source, &site.path);
    let merged = merge_constraints(lifted, use_site.into_iter().zip(use_origins).collect(), &merge_site)?;
    let (array_constraints, array_origins) = merged.into_iter().unzip();
    Ok(ExpandedDef::Single(
        Box::new(PropertyAtom {
            ty: TypeExpr::Union(arms),
            is_array: true,
            constraints: Vec::new(),
            array_constraints,
            description,
        }),
        AtomProvenance {
            value: Vec::new(),
            array: array_origins,
            shape: Vec::new(),
        },
    ))
}

/// A property marker a union reference accepts without `[]`.
fn is_union_marker(constraint: &Constraint) -> bool {
    matches!(
        constraint,
        Constraint::Required | Constraint::Generated | Constraint::Eager
    )
}

/// A constraint the converter accepts on an array container besides the
/// property markers (mirrors `convert::apply_array_constraints`).
fn is_array_container_constraint(constraint: &Constraint) -> bool {
    matches!(
        constraint,
        Constraint::MinItems(_)
            | Constraint::MaxItems(_)
            | Constraint::Unique
            | Constraint::Default(_)
            | Constraint::Example(_)
    )
}

/// Removes every default `arm` supplies to its containing property and
/// returns them.
fn take_arm_defaults(arm: &mut PropertyAtom) -> Vec<Value> {
    let mut defaults = Vec::new();
    for list in [&mut arm.constraints, &mut arm.array_constraints] {
        list.retain(|constraint| match constraint {
            Constraint::Default(value) => {
                defaults.push(value.clone());
                false
            }
            _ => true,
        });
    }
    defaults
}

/// What a use-site `min`/`max` counts once the target is known.
#[derive(Clone, Copy)]
enum CountedBound {
    /// Characters of a `string` value.
    Length,
    /// Items of an array container.
    Items,
}

/// Reinterprets use-site `min`/`max`, which a reference parses as numeric
/// bounds before its target is known, as counts of `counted`.
fn reinterpret_bounds(
    constraints: Vec<Tagged>,
    counted: CountedBound,
    property: &str,
) -> Result<Vec<Tagged>, SchemaError> {
    constraints
        .into_iter()
        .map(|(constraint, origin)| {
            let constraint = match (constraint, counted) {
                (Constraint::Min(n), CountedBound::Length) => {
                    Constraint::MinLen(bound_count(n, "min", property)?)
                }
                (Constraint::Max(n), CountedBound::Length) => {
                    Constraint::MaxLen(bound_count(n, "max", property)?)
                }
                (Constraint::Min(n), CountedBound::Items) => {
                    Constraint::MinItems(bound_count(n, "min", property)?)
                }
                (Constraint::Max(n), CountedBound::Items) => {
                    Constraint::MaxItems(bound_count(n, "max", property)?)
                }
                (other, _) => other,
            };
            Ok((constraint, origin))
        })
        .collect()
}

/// A count bound must be a non-negative integer (the wording mirrors the
/// grammar's check for a directly typed `string(min(n))`).
fn bound_count(n: f64, keyword: &str, property: &str) -> Result<usize, SchemaError> {
    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 {
        return Err(SchemaError::Convert {
            property: property.to_string(),
            message: format!("`{keyword}` requires a non-negative integer, got `{n}`"),
        });
    }
    Ok(n as usize)
}

fn is_key_bound(constraint: &Constraint) -> bool {
    matches!(constraint, Constraint::MinKeys(_) | Constraint::MaxKeys(_))
}

/// The kind a bound limits; bounds merge only within one kind.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BoundKind {
    Number,
    Length,
    Items,
    Keys,
}

#[derive(Clone, Copy)]
struct Bound {
    kind: BoundKind,
    lower: bool,
    value: f64,
}

impl Bound {
    fn of(constraint: &Constraint) -> Option<Self> {
        let (kind, lower, value) = match constraint {
            Constraint::Min(n) => (BoundKind::Number, true, *n),
            Constraint::Max(n) => (BoundKind::Number, false, *n),
            Constraint::MinLen(n) => (BoundKind::Length, true, *n as f64),
            Constraint::MaxLen(n) => (BoundKind::Length, false, *n as f64),
            Constraint::MinItems(n) => (BoundKind::Items, true, *n as f64),
            Constraint::MaxItems(n) => (BoundKind::Items, false, *n as f64),
            Constraint::MinKeys(n) => (BoundKind::Keys, true, *n as f64),
            Constraint::MaxKeys(n) => (BoundKind::Keys, false, *n as f64),
            _ => return None,
        };
        Some(Self { kind, lower, value })
    }

    fn stricter_than(&self, other: &Self) -> bool {
        if self.lower {
            self.value > other.value
        } else {
            self.value < other.value
        }
    }
}

fn is_flag(constraint: &Constraint) -> bool {
    matches!(
        constraint,
        Constraint::NotEmpty
            | Constraint::Integer
            | Constraint::Unique
            | Constraint::Eager
            | Constraint::Required
            | Constraint::Generated
    )
}

/// Checks the `alphanumeric` options a use site wrote against the named
/// type's. Equal canonical sets are a repetition; any difference is a
/// [`SchemaConstraintConflictKind::Options`] conflict, and options on a
/// target that is not `alphanumeric` are not valid.
fn check_use_site_options(
    base: &PropertyAtom,
    base_provenance: &AtomProvenance,
    options: Vec<Tagged>,
    merge_site: &MergeSite<'_>,
    definition: &SchemaReferenceSite,
    reference: &str,
) -> Result<(), SchemaError> {
    let name = merge_site.name;
    let written = options
        .iter()
        .filter_map(|(constraint, _)| match constraint {
            Constraint::Options(options) => Some(options.iter().copied()),
            _ => None,
        })
        .flatten()
        .collect::<AlphanumericOptions>();
    let Some(target) = base.alphanumeric_options() else {
        return Err(SchemaError::Convert {
            property: merge_site.site.path.to_string(),
            message: format!(
                "`alphanumeric` options ({}) are not valid on the named type \
                 `{name}@{reference}`, which is not `alphanumeric`",
                written.keywords().collect::<Vec<_>>().join(", ")
            ),
        });
    };
    if written.canonical() == target {
        return Ok(());
    }
    let use_origin = options
        .into_iter()
        .map(|(_, origin)| origin)
        .next()
        .expect("caller passes at least one option list");
    let base_origin = base
        .constraints
        .iter()
        .zip(&base_provenance.value)
        .find_map(|(constraint, origin)| {
            matches!(constraint, Constraint::Options(_)).then(|| origin.clone())
        })
        // The base type writes no options; point at its declaration.
        .unwrap_or_else(|| SchemaConstraintOrigin {
            source: definition.source.clone(),
            path: definition.path.clone(),
            keyword: SimplifiedType::Alphanumeric.as_keyword().to_string(),
            occurrence: 0,
            span: None,
        });
    Err(merge_site.conflict(SchemaConstraintConflictKind::Options, vec![base_origin, use_origin]))
}

/// Merges `inherited` and then `use_site` constraints of one level (value or
/// array container) under the rules on [`apply_import_postfix`]. The result
/// keeps first-seen order; a stricter bound replaces the looser one in place
/// and takes its origin.
fn merge_constraints(
    inherited: Vec<Tagged>,
    use_site: Vec<Tagged>,
    merge_site: &MergeSite<'_>,
) -> Result<Vec<Tagged>, SchemaError> {
    // The third field is the position in the inherited-then-use-site stream,
    // which orders a conflict's origins inherited first.
    let mut merged: Vec<(Constraint, SchemaConstraintOrigin, usize)> = Vec::new();
    for (sequence, (constraint, origin)) in inherited.into_iter().chain(use_site).enumerate() {
        if let Some(bound) = Bound::of(&constraint) {
            let same_slot = merged.iter().enumerate().find_map(|(index, (existing, ..))| {
                Bound::of(existing)
                    .filter(|other| other.kind == bound.kind && other.lower == bound.lower)
                    .map(|other| (index, other))
            });
            match same_slot {
                Some((index, existing)) if bound.stricter_than(&existing) => {
                    merged[index] = (constraint, origin, sequence);
                }
                Some(_) => {}
                None => merged.push((constraint, origin, sequence)),
            }
            continue;
        }
        match &constraint {
            Constraint::Scheme(schemes) => {
                if let Some((Constraint::Scheme(existing), existing_origin, _)) = merged
                    .iter_mut()
                    .find(|(c, ..)| matches!(c, Constraint::Scheme(_)))
                {
                    existing.retain(|scheme| schemes.contains(scheme));
                    if existing.is_empty() {
                        return Err(merge_site.conflict(
                            SchemaConstraintConflictKind::Scheme,
                            vec![existing_origin.clone(), origin],
                        ));
                    }
                    continue;
                }
            }
            Constraint::Default(value) => {
                if let Some((Constraint::Default(existing), existing_origin, _)) = merged
                    .iter()
                    .find(|(c, ..)| matches!(c, Constraint::Default(_)))
                {
                    if defaults_equal(existing, value) {
                        continue;
                    }
                    return Err(merge_site.conflict(
                        SchemaConstraintConflictKind::Default,
                        vec![existing_origin.clone(), origin],
                    ));
                }
            }
            flag_or_pattern
                if (is_flag(flag_or_pattern) || matches!(flag_or_pattern, Constraint::Pattern(_)))
                    && merged.iter().any(|(existing, ..)| existing == flag_or_pattern) =>
            {
                continue;
            }
            _ => {}
        }
        merged.push((constraint, origin, sequence));
    }

    for kind in [BoundKind::Number, BoundKind::Length, BoundKind::Items, BoundKind::Keys] {
        let find = |lower: bool| {
            merged.iter().find_map(|(constraint, origin, sequence)| {
                Bound::of(constraint)
                    .filter(|bound| bound.kind == kind && bound.lower == lower)
                    .map(|bound| (bound.value, origin, *sequence))
            })
        };
        if let (Some(lower), Some(upper)) = (find(true), find(false))
            && lower.0 > upper.0
        {
            let mut ordered = [lower, upper];
            ordered.sort_by_key(|(_, _, sequence)| *sequence);
            return Err(merge_site.conflict(
                SchemaConstraintConflictKind::Bounds,
                ordered.iter().map(|(_, origin, _)| (*origin).clone()).collect(),
            ));
        }
    }
    Ok(merged
        .into_iter()
        .map(|(constraint, origin, _)| (constraint, origin))
        .collect())
}

/// A default is a property-level value wherever it is written, so one on the
/// items and one on the container must agree. An equal duplicate on the
/// container is dropped.
fn reconcile_property_defaults(
    value: Vec<Tagged>,
    mut array: Vec<Tagged>,
    merge_site: &MergeSite<'_>,
) -> Result<(Vec<Tagged>, Vec<Tagged>), SchemaError> {
    let item_default = value.iter().find_map(|(constraint, origin)| match constraint {
        Constraint::Default(default) => Some((default, origin)),
        _ => None,
    });
    if let Some((item_default, item_origin)) = item_default {
        if let Some((_, container_origin)) = array.iter().find(|(constraint, _)| {
            matches!(constraint, Constraint::Default(default) if !defaults_equal(default, item_default))
        }) {
            return Err(merge_site.conflict(
                SchemaConstraintConflictKind::Default,
                vec![item_origin.clone(), container_origin.clone()],
            ));
        }
        array.retain(|(constraint, _)| !matches!(constraint, Constraint::Default(_)));
    }
    Ok((value, array))
}

/// Whether two defaults lower to the same JSON value. Integral floats equal
/// integers (`3.0` is `3`); a string never equals a number. Mirrors the
/// converter's `normalize_json_number` so a merge accepts exactly the
/// defaults that lower identically.
fn defaults_equal(left: &Value, right: &Value) -> bool {
    normalized_default(left) == normalized_default(right)
}

fn normalized_default(value: &Value) -> Value {
    match value {
        Value::Number(n) if n.is_i64() || n.is_u64() => value.clone(),
        Value::Number(n) => n
            .as_f64()
            .filter(|f| {
                f.is_finite() && f.fract() == 0.0 && (i64::MIN as f64..=i64::MAX as f64).contains(f)
            })
            .map_or_else(|| value.clone(), |f| Value::from(f as i64)),
        Value::Array(items) => Value::Array(items.iter().map(normalized_default).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), normalized_default(item)))
                .collect(),
        ),
        other => other.clone(),
    }
}

// ── Import-tree traversal helpers (fast-path detection) ─────────────────────

fn schema_has_imports(schema: &SimplifiedSchema) -> bool {
    match schema {
        SimplifiedSchema::Single(shape) => shape_has_imports(shape),
        SimplifiedSchema::Union(arms) => arms.iter().any(|arm| match arm {
            SchemaArm::Inline(shape) => shape_has_imports(shape),
            SchemaArm::FileRef(_) => false,
        }),
    }
}

fn shape_has_imports(shape: &SchemaShape) -> bool {
    shape.properties.values().any(def_has_imports)
        || shape.pattern_keys.iter().any(|pk| def_has_imports(&pk.def))
}

fn def_has_imports(def: &PropertyDef) -> bool {
    match def {
        PropertyDef::Single(atom) => atom_has_imports(atom),
        PropertyDef::Union(arms) => arms.iter().any(atom_has_imports),
    }
}

fn atom_has_imports(atom: &PropertyAtom) -> bool {
    match &atom.ty {
        TypeExpr::Imported { .. } => true,
        TypeExpr::InlineObject(shape) => shape_has_imports(shape),
        TypeExpr::Union(arms) => arms.iter().any(atom_has_imports),
        TypeExpr::Tuple(tuple) => tuple.any_atom(atom_has_imports),
        TypeExpr::Primitive(_) | TypeExpr::Function(_) | TypeExpr::Ref(_) => false,
    }
}

// ── `example(...)` artifact resolution (Feature A) ──────────────────────────
//
// The converter emits `example(...)` constraints as an `x-darkmatter-example`
// array of raw reference strings. This pass walks the compiled JSON Schema,
// resolves each reference (relative to the schema's directory; `this` targets
// the schema file), validates the referenced artifact at load time (fail-loud,
// O-A2), and replaces the reference-string array with the resolved example
// objects so downstream consumers read them without re-reading disk. Each
// resolved file is recorded as a dependency edge.

/// Resolves every `x-darkmatter-example` reference-string array in `json` into
/// validated example objects. Returns the sorted, deduplicated set of resolved
/// example file paths (dependency edges), and adds every location probed for
/// an example file to `searched`.
///
/// Fast path: a schema with no `x-darkmatter-example` annotation returns an
/// empty `Vec` and leaves `json` byte-equivalent.
fn resolve_document_examples(
    json: &mut Value,
    base_dir: &Path,
    this_file: Option<&Path>,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    searched: &mut BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>, SchemaError> {
    let mut deps: BTreeSet<PathBuf> = BTreeSet::new();
    let mut reads: HashMap<PathBuf, Vec<u8>> = HashMap::new();
    // A target is compiled alone, so it takes the root's definitions (read
    // before any annotation inside them is resolved in place).
    let definitions = json.get("$defs").cloned();
    resolve_examples_in_json(
        json,
        definitions.as_ref(),
        base_dir,
        this_file,
        schema_roots,
        request_context,
        &mut deps,
        &mut reads,
        searched,
    )?;
    Ok(deps.into_iter().collect())
}

#[allow(clippy::too_many_arguments)]
fn resolve_examples_in_json(
    value: &mut Value,
    definitions: Option<&Value>,
    base_dir: &Path,
    this_file: Option<&Path>,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    deps: &mut BTreeSet<PathBuf>,
    reads: &mut HashMap<PathBuf, Vec<u8>>,
    searched: &mut BTreeSet<PathBuf>,
) -> Result<(), SchemaError> {
    match value {
        Value::Object(map) => {
            // Resolve the annotation in place when it is still an array of
            // reference strings (a resolved array holds objects, not strings).
            if let Some(Value::Array(items)) = map.get("x-darkmatter-example")
                && items.iter().all(Value::is_string)
            {
                // The annotated target is this property's own compiled schema
                // with the example annotation stripped — its sibling keys carry
                // the property's value type, which each example's `returns` must
                // match. Cloned before the annotation is overwritten below.
                let mut target = map.clone();
                target.remove("x-darkmatter-example");
                if let Some(definitions) = definitions {
                    target.insert("$defs".into(), definitions.clone());
                }
                let target = Value::Object(target);
                let mut resolved = Vec::with_capacity(items.len());
                for item in items {
                    let reference = item.as_str().expect("all-string checked above");
                    resolved.push(resolve_one_example(
                        reference,
                        base_dir,
                        this_file,
                        Some(&target),
                        schema_roots,
                        request_context,
                        deps,
                        reads,
                        searched,
                    )?);
                }
                map.insert("x-darkmatter-example".into(), Value::Array(resolved));
            }
            // Recurse into the rest of the schema. The just-resolved annotation
            // is skipped so resolved example *data* (which may itself carry a
            // key literally named `x-darkmatter-example`) is never re-resolved.
            for (key, child) in map.iter_mut() {
                if key == "x-darkmatter-example" {
                    continue;
                }
                resolve_examples_in_json(
                    child,
                    definitions,
                    base_dir,
                    this_file,
                    schema_roots,
                    request_context,
                    deps,
                    reads,
                    searched,
                )?;
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                resolve_examples_in_json(
                    item,
                    definitions,
                    base_dir,
                    this_file,
                    schema_roots,
                    request_context,
                    deps,
                    reads,
                    searched,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// The file an `example(...)` reference names: `this` is `this_file`; any
/// other value is a `biscuit-file` reference resolved from `base_dir`, with a
/// bare name tried against the schema roots first.
fn resolve_example_path(
    reference: &str,
    base_dir: &Path,
    this_file: Option<&Path>,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    searched: &mut BTreeSet<PathBuf>,
) -> Result<PathBuf, SchemaError> {
    let trimmed = reference.trim();
    Ok(if trimmed == "this" {
        this_file
            .map(Path::to_path_buf)
            .ok_or_else(|| SchemaError::InvalidExample {
                reference: reference.to_string(),
                message: "`this` example reference has no target (the schema is inline, not a file)"
                    .into(),
            })?
    } else {
        let file_ref =
            FileReference::new(trimmed).map_err(|source| SchemaError::Unresolved {
                reference: reference.to_string(),
                source,
            })?;
        if file_ref.class().kind == FileReferenceKind::Url {
            return Err(SchemaError::RemoteUnsupported {
                reference: reference.to_string(),
            });
        }
        // Bare-name resolution against schema roots (Phase 3).
        if !schema_roots.is_empty() && is_bare_name(&file_ref) {
            if let Some(p) =
                try_bare_name_in_roots(&file_ref, schema_roots, request_context, searched)?
            {
                p
            } else {
                let sibling = base_dir.join(trimmed);
                if sibling.is_file() {
                    return Err(SchemaError::BareNameSiblingExists {
                        reference: trimmed.to_string(),
                        suggestion: format!("./{trimmed}"),
                    });
                }
                return Err(SchemaError::NoMatch {
                    reference: reference.to_string(),
                    bare_name: true,
                });
            }
        } else {
            resolve_file_reference_in_context(&file_ref, base_dir, request_context, searched)
                .map_err(|source| SchemaError::Unresolved {
                    reference: trimmed.to_string(),
                    source,
                })?
                .ok_or_else(|| SchemaError::NoMatch {
                    reference: trimmed.to_string(),
                    bare_name: false,
                })?
        }
    })
}

/// Resolves one `example(...)` reference to a validated example object.
///
/// `this` targets the current schema file (`this_file`); every other value is a
/// `biscuit-file` reference resolved relative to `base_dir` — the same
/// resolution `$schema` file references use. The referenced file is validated
/// against the example-artifact envelope with content-hash caching, then — when
/// `target` is `Some` — its `returns` value is validated against the annotated
/// property's target schema as a separate, un-cached gate.
#[allow(clippy::too_many_arguments)]
fn resolve_one_example(
    reference: &str,
    base_dir: &Path,
    this_file: Option<&Path>,
    target: Option<&Value>,
    schema_roots: &[PathBuf],
    request_context: &biscuit_file::FileResolutionContext,
    deps: &mut BTreeSet<PathBuf>,
    reads: &mut HashMap<PathBuf, Vec<u8>>,
    searched: &mut BTreeSet<PathBuf>,
) -> Result<Value, SchemaError> {
    let path =
        resolve_example_path(reference, base_dir, this_file, schema_roots, request_context, searched)?;
    let canonical = canonical_path(&path);
    // Reuse the bytes when another `example(...)` site already read this file
    // during the same resolution run.
    let bytes = match reads.get(&canonical) {
        Some(cached) => cached.clone(),
        None => {
            let read = fs::read(&path).map_err(|source| SchemaError::Io {
                path: path.clone(),
                source,
            })?;
            reads.insert(canonical.clone(), read.clone());
            read
        }
    };
    let object = super::example::validate_example_bytes(reference, &bytes)?;
    // Layer 3: the example's `returns` must match the property it annotates.
    // Applied here (not inside the byte cache) so the same file validated
    // against different targets never shares a target verdict.
    if let Some(target) = target {
        super::example::validate_returns_against_target(reference, &object, target)?;
    }
    deps.insert(canonical);
    Ok(object)
}

/// Merges a baseline JSON Schema into a document JSON Schema using the
/// property-keyed deep merge documented in the spec.
///
/// - When both schemas declare the same property, the **document** side wins
///   entirely (its type + constraints replace the baseline's).
/// - `required` arrays are unioned, then de-duplicated, but only document
///   properties retain their baseline-required status if they appear in the
///   baseline's `required` array (since the document's own `required` may
///   restate them).
/// - Properties present only in one side are preserved verbatim.
/// - The baseline's root `$defs` move to the document's root, where its
///   `#/$defs/…` references resolve once its properties are copied in (into
///   every arm of a root union). The document's keys and references never
///   change; a baseline key that collides with a different document body is
///   re-keyed (`item-2`) along with the baseline's references to it.
///
/// ## Errors
///
/// Returns [`SchemaError::Baseline`] when the baseline is not a simple
/// object schema (rooted at `"type": "object"` with only `properties` /
/// `required` / `$defs`), [`SchemaError::UnresolvedReference`] or
/// [`SchemaError::UnsupportedMergeReference`] when its references would not
/// keep their meaning under the document's root, and
/// [`SchemaError::UnsupportedMergeReference`] when its definitions would move
/// into a root-union arm that declares its own `$id`.
pub fn merge_baseline(baseline: &Value, document: Value) -> Result<Value, SchemaError> {
    validate_baseline_schema(baseline)?;
    let mut document = document;
    let baseline = move_baseline_definitions(baseline, &mut document)?;
    merge_layer(&baseline, document)
}

/// The baseline with its root `$defs` moved into `document`'s root and its
/// references renamed to match; borrowed unchanged when it has none.
fn move_baseline_definitions<'b>(
    baseline: &'b Value,
    document: &mut Value,
) -> Result<std::borrow::Cow<'b, Value>, SchemaError> {
    if baseline.get("$defs").is_none() {
        return Ok(std::borrow::Cow::Borrowed(baseline));
    }
    let Some(document_obj) = document.as_object_mut() else {
        // `merge_layer` reports the non-object document.
        return Ok(std::borrow::Cow::Borrowed(baseline));
    };
    // `#/$defs` inside an arm with its own `$id` resolves in that arm, not in
    // the root the definitions move to.
    if let Some(arms) = document_obj.get("anyOf").and_then(Value::as_array)
        && let Some(index) = arms.iter().position(|arm| arm.get("$id").is_some())
    {
        return Err(SchemaError::UnsupportedMergeReference {
            schema: "the baseline".into(),
            location: format!("/anyOf/{index}"),
            reason: "the document arm's `$id` would change the scope the baseline's definitions resolve in"
                .into(),
        });
    }
    let mut baseline = baseline.clone();
    let mut document_defs = match document_obj.remove("$defs") {
        Some(Value::Object(defs)) => defs,
        Some(other) => {
            document_obj.insert("$defs".into(), other);
            return Err(SchemaError::Baseline {
                message: "the document's root `$defs` is not an object, so the baseline's definitions \
                          cannot join it"
                    .into(),
                source: None,
            });
        }
        None => Map::new(),
    };
    super::scope::fold_definitions(&mut document_defs, &mut baseline);
    document_obj.insert("$defs".into(), Value::Object(document_defs));
    Ok(std::borrow::Cow::Owned(baseline))
}

/// The property-keyed merge of an already admitted `baseline` (no root
/// `$defs` of its own) into `document`, arm by arm for a root union.
pub(super) fn merge_layer(baseline: &Value, document: Value) -> Result<Value, SchemaError> {
    let baseline_obj = baseline
        .as_object()
        .expect("validated baseline must be an object");

    // Root unions: merge into each arm independently.
    if let Some(arms) = document.get("anyOf").and_then(Value::as_array) {
        let mut new_arms = Vec::with_capacity(arms.len());
        for arm in arms {
            new_arms.push(merge_layer(baseline, arm.clone())?);
        }
        let mut merged = document.as_object().cloned().unwrap_or_else(Map::new);
        merged.insert("anyOf".into(), Value::Array(new_arms));
        return Ok(Value::Object(merged));
    }

    let mut document_obj = match document {
        Value::Object(m) => m,
        _ => {
            return Err(SchemaError::Baseline {
                message: "document schema must be an object".into(),
                source: None,
            });
        }
    };

    let baseline_props = baseline_obj.get("properties").and_then(Value::as_object);
    let baseline_required: std::collections::HashSet<&str> = baseline_obj
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .collect()
        })
        .unwrap_or_default();

    let mut doc_props = document_obj
        .remove("properties")
        .and_then(|v| match v {
            Value::Object(m) => Some(m),
            _ => None,
        })
        .unwrap_or_default();
    let mut doc_required: Vec<String> = document_obj
        .remove("required")
        .and_then(|v| match v {
            Value::Array(a) => Some(a),
            _ => None,
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();

    if let Some(baseline_props) = baseline_props {
        for (key, baseline_schema) in baseline_props {
            if !doc_props.contains_key(key) {
                doc_props.insert(key.clone(), baseline_schema.clone());
                if baseline_required.contains(key.as_str()) && !doc_required.contains(key) {
                    doc_required.push(key.clone());
                }
            }
        }
    }

    document_obj.insert("properties".into(), Value::Object(doc_props));
    if !doc_required.is_empty() {
        // Stable order: keep the existing document order, then append any
        // baseline-only required props.
        let mut seen = std::collections::HashSet::new();
        let mut ordered: Vec<Value> = Vec::with_capacity(doc_required.len());
        for name in doc_required {
            if seen.insert(name.clone()) {
                ordered.push(Value::String(name));
            }
        }
        document_obj.insert("required".into(), Value::Array(ordered));
    } else {
        document_obj.remove("required");
    }

    // Ensure required fields like "$schema" and "type" survive — copy from
    // baseline where the document didn't supply them.
    if !document_obj.contains_key("type") && baseline_obj.get("type").is_some() {
        document_obj.insert("type".into(), baseline_obj["type"].clone());
    }
    if !document_obj.contains_key("$schema") && baseline_obj.get("$schema").is_some() {
        document_obj.insert("$schema".into(), baseline_obj["$schema"].clone());
    }
    if !document_obj.contains_key("additionalProperties")
        && baseline_obj.get("additionalProperties").is_some()
    {
        document_obj.insert(
            "additionalProperties".into(),
            baseline_obj["additionalProperties"].clone(),
        );
    }

    Ok(Value::Object(document_obj))
}

pub(super) fn validate_baseline_schema(baseline: &Value) -> Result<(), SchemaError> {
    validate_simple_object_schema(baseline, "the baseline")
}

/// Validates that a JSON Schema is a simple object schema: rooted at
/// `"type": "object"` with only `properties` / `required` (plus the cosmetic
/// `$schema` / `additionalProperties: true` / `description` / `title`, and a
/// root `$defs` table its properties refer to).
///
/// This is the merge-compatibility contract: both caller baselines and trigger
/// payloads must satisfy it so the property-keyed merge has a sound "later
/// wins" meaning. `label` names the schema in errors.
///
/// A root `$defs` moves to the root the schema merges under, so a schema that
/// carries one must be a valid Draft 2020-12 schema and its references must
/// stay in its own table ([`super::scope::check_joined_references`]). Every
/// schema's `#/$defs/…` references must resolve in its own table; the other
/// reference forms of a schema without `$defs` are not inspected, as before.
pub(crate) fn validate_simple_object_schema(schema: &Value, label: &str) -> Result<(), SchemaError> {
    let object = schema.as_object().ok_or_else(|| SchemaError::Baseline {
        message: format!("{label} must be an object schema"),
        source: None,
    })?;
    validate_simple_object_root(object)?;
    if object.contains_key("$defs")
        && let Err(error) = jsonschema::draft202012::meta::validate(schema)
    {
        return Err(SchemaError::Baseline {
            message: format!(
                "{label} declares `$defs` but is not a valid Draft 2020-12 schema at `{}`: {error}",
                error.instance_path()
            ),
            source: None,
        });
    }
    super::scope::check_joined_references(schema, label)
}

/// The root-key half of [`validate_simple_object_schema`].
pub(super) fn validate_simple_object_root(schema: &Map<String, Value>) -> Result<(), SchemaError> {
    // `additionalProperties: true` is permitted (SimplifiedSchema-generated
    // baselines emit it); `false` and schema-shaped values are not.
    const ALLOWED_KEYS: &[&str] = &[
        "$schema",
        "type",
        "properties",
        "required",
        "additionalProperties",
        "description",
        "title",
        "$defs",
    ];

    if let Some(ty) = schema.get("type") {
        if ty != "object" {
            return Err(SchemaError::Baseline {
                message: format!("baseline root must be type=object, got {ty}"),
                source: None,
            });
        }
    } else {
        return Err(SchemaError::Baseline {
            message: "baseline root must declare type=object".into(),
            source: None,
        });
    }

    for key in schema.keys() {
        if !ALLOWED_KEYS.contains(&key.as_str()) {
            return Err(SchemaError::Baseline {
                message: format!(
                    "baseline uses unsupported JSON Schema construct `{key}`; only \
                     simple object schemas (properties + required) are allowed"
                ),
                source: None,
            });
        }
    }

    if let Some(ap) = schema.get("additionalProperties") {
        match ap {
            Value::Bool(true) => {}
            _ => {
                return Err(SchemaError::Baseline {
                    message: "baseline uses unsupported JSON Schema construct \
                              `additionalProperties: false`; only simple object \
                              schemas (properties + required) are allowed"
                        .into(),
                    source: None,
                });
            }
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────

pub(crate) fn describe_yaml(value: &YamlValue) -> &'static str {
    match value {
        YamlValue::Null => "null",
        YamlValue::Bool(_) => "boolean",
        YamlValue::Number(_) => "number",
        YamlValue::String(_) => "string",
        YamlValue::Sequence(_) => "sequence",
        YamlValue::Mapping(_) => "mapping",
        YamlValue::Tagged(_) => "tagged-value",
    }
}

/// Converts a `serde_json::Value` (the type frontmatter uses internally)
/// into a `serde_yaml_ng::Value` so the YAML-shape parser can consume it
/// without a second deserialisation round-trip.
fn json_to_yaml(value: &Value) -> YamlValue {
    match value {
        Value::Null => YamlValue::Null,
        Value::Bool(b) => YamlValue::Bool(*b),
        Value::Number(n) => {
            // serde_yaml_ng accepts the same numeric forms as serde_json.
            if let Some(i) = n.as_i64() {
                YamlValue::Number(serde_yaml_ng::Number::from(i))
            } else if let Some(u) = n.as_u64() {
                YamlValue::Number(serde_yaml_ng::Number::from(u))
            } else if let Some(f) = n.as_f64() {
                YamlValue::Number(serde_yaml_ng::Number::from(f))
            } else {
                YamlValue::Null
            }
        }
        Value::String(s) => YamlValue::String(s.clone()),
        Value::Array(items) => YamlValue::Sequence(items.iter().map(json_to_yaml).collect()),
        Value::Object(map) => {
            let mut out = serde_yaml_ng::Mapping::new();
            for (k, v) in map {
                out.insert(YamlValue::String(k.clone()), json_to_yaml(v));
            }
            YamlValue::Mapping(out)
        }
    }
}

fn strip_schema_uri(value: Value) -> Value {
    if let Value::Object(mut map) = value {
        map.remove("$schema");
        Value::Object(map)
    } else {
        value
    }
}

/// Resolves a SimplifiedSchema (typed) under the same disambiguation rules
/// used for `$schema` references. Useful when callers already have a
/// SimplifiedSchema in hand (e.g. the library-level `with_baseline`) and
/// only need to convert + merge.
pub fn shape_to_schema(shape: &SchemaShape) -> Result<Value, SchemaError> {
    to_json_schema(&SimplifiedSchema::Single(shape.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::SchemaAdvisoryKind;
    use serde_json::json;

    fn yaml_value(input: &str) -> YamlValue {
        serde_yaml_ng::from_str(input).expect("yaml parse")
    }

    #[test]
    fn resolves_inline_mapping() {
        let v = yaml_value("title: 'string(required)'");
        let resolved = resolve_yaml_schema(&v, Path::new("."), &biscuit_file::FileResolutionContext::new(Path::new("."))).unwrap();
        assert!(resolved.simplified.is_some());
        assert_eq!(resolved.json_schema["type"], "object");
        let required = resolved.json_schema["required"].as_array().unwrap();
        assert_eq!(required[0], "title");
    }

    #[test]
    fn resolves_inline_root_union() {
        let v = yaml_value("- title: 'string(required)'\n- name: string");
        let resolved = resolve_yaml_schema(&v, Path::new("."), &biscuit_file::FileResolutionContext::new(Path::new("."))).unwrap();
        assert!(resolved.json_schema["anyOf"].is_array());
    }

    #[test]
    fn remote_https_is_rejected() {
        let v = yaml_value("'https://example.com/schema.json'");
        let err = resolve_yaml_schema(&v, Path::new("."), &biscuit_file::FileResolutionContext::new(Path::new("."))).unwrap_err();
        assert!(matches!(err, SchemaError::RemoteUnsupported { .. }));
    }

    #[test]
    fn remote_http_is_rejected() {
        let v = yaml_value("'http://example.com/schema.json'");
        let err = resolve_yaml_schema(&v, Path::new("."), &biscuit_file::FileResolutionContext::new(Path::new("."))).unwrap_err();
        assert!(matches!(err, SchemaError::RemoteUnsupported { .. }));
    }

    #[test]
    fn unsupported_shape_is_rejected() {
        let v = yaml_value("42");
        let err = resolve_yaml_schema(&v, Path::new("."), &biscuit_file::FileResolutionContext::new(Path::new("."))).unwrap_err();
        assert!(matches!(err, SchemaError::FrontmatterShape { .. }));
    }

    fn write_schema_file(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn resolves_simplified_yaml_file_reference() {
        let dir = tempfile::tempdir().unwrap();
        write_schema_file(
            dir.path(),
            "schema.yaml",
            "$schema:\n  title: 'string(required)'\n",
        );
        let v = yaml_value("./schema.yaml");
        let resolved = resolve_yaml_schema(&v, dir.path(), &biscuit_file::FileResolutionContext::new(dir.path())).unwrap();
        assert!(resolved.simplified.is_some());
        let required = resolved.json_schema["required"].as_array().unwrap();
        assert_eq!(required[0], "title");
    }

    #[test]
    fn referenced_file_is_recorded_as_dependency_edge() {
        // Editing `schema.yaml`'s own type must be able to invalidate a cached
        // effective schema, so the referenced file itself is a dependency edge.
        let dir = tempfile::tempdir().unwrap();
        let path = write_schema_file(
            dir.path(),
            "schema.yaml",
            "$schema:\n  title: 'string(required)'\n",
        );
        let v = yaml_value("./schema.yaml");
        let resolved = resolve_yaml_schema(&v, dir.path(), &biscuit_file::FileResolutionContext::new(dir.path())).unwrap();
        assert_eq!(resolved.referenced_files, vec![canonical_path(&path)]);
    }

    #[test]
    fn inline_mapping_records_no_referenced_files() {
        let v = yaml_value("title: 'string(required)'");
        let resolved = resolve_yaml_schema(&v, Path::new("."), &biscuit_file::FileResolutionContext::new(Path::new("."))).unwrap();
        assert!(resolved.referenced_files.is_empty());
    }

    #[test]
    fn root_union_string_arm_records_referenced_file() {
        // A root-union string arm's file is a dependency edge like a top-level
        // referenced file; an inline arm contributes none.
        let dir = tempfile::tempdir().unwrap();
        let arm = write_schema_file(
            dir.path(),
            "arm.yaml",
            "$schema:\n  title: 'string(required)'\n",
        );
        let v = yaml_value("- ./arm.yaml\n- name: string\n");
        let resolved = resolve_yaml_schema(&v, dir.path(), &biscuit_file::FileResolutionContext::new(dir.path())).unwrap();
        assert_eq!(resolved.referenced_files, vec![canonical_path(&arm)]);
    }

    #[test]
    fn resolves_raw_json_schema_file_reference() {
        let dir = tempfile::tempdir().unwrap();
        write_schema_file(
            dir.path(),
            "schema.json",
            r#"{"type":"object","properties":{"x":{"type":"number"}}}"#,
        );
        let v = yaml_value("./schema.json");
        let resolved = resolve_yaml_schema(&v, dir.path(), &biscuit_file::FileResolutionContext::new(dir.path())).unwrap();
        assert!(resolved.simplified.is_none());
        assert_eq!(resolved.json_schema["properties"]["x"]["type"], "number");
    }

    #[test]
    fn yaml_without_schema_key_is_treated_as_json_schema() {
        let dir = tempfile::tempdir().unwrap();
        write_schema_file(
            dir.path(),
            "no-schema.yaml",
            "title: hello\nproperties:\n  x:\n    type: number\n",
        );
        let v = yaml_value("./no-schema.yaml");
        let resolved = resolve_yaml_schema(&v, dir.path(), &biscuit_file::FileResolutionContext::new(dir.path())).unwrap();
        assert!(resolved.simplified.is_none());
    }

    #[test]
    fn bare_simplified_map_remains_raw_json_schema_and_reports_advisory() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_schema_file(
            dir.path(),
            "schema.yaml",
            "source_marker: string(required)\nspec: 'file(eager; required)'\ncaller_spec: 'file(eager; required)'\n",
        );
        let resolved = resolve_yaml_schema(&yaml_value("./schema.yaml"), dir.path(), &biscuit_file::FileResolutionContext::new(dir.path())).unwrap();

        assert!(resolved.simplified.is_none());
        assert_eq!(resolved.json_schema["source_marker"], "string(required)");
        assert_eq!(resolved.json_schema["spec"], "file(eager; required)");
        let validator = jsonschema::validator_for(&resolved.json_schema).unwrap();
        assert!(validator.is_valid(&json!({ "anything": true })));
        assert_eq!(resolved.advisories.len(), 1);
        let advisory = &resolved.advisories[0];
        assert_eq!(
            advisory.kind(),
            SchemaAdvisoryKind::MissingSimplifiedEnvelope
        );
        assert_eq!(advisory.source(), "darkmatter.schema");
        assert_eq!(advisory.code(), "dm.schema.missing_simplified_envelope");
        assert_eq!(advisory.path(), path);
        let message = advisory.message();
        assert!(message.contains(&path.display().to_string()));
        assert!(message.contains("`$schema:`"));
        assert!(message.contains("`kind: schema` + `types:`"));
    }

    #[test]
    fn bare_map_advisory_excludes_json_schema_envelopes_and_invalid_maps() {
        let cases = [
            (
                "object-properties.yaml",
                "type: object\nproperties:\n  name:\n    type: string\n",
            ),
            ("format.yaml", "format: string\n"),
            ("title.yaml", "title: string(required)\n"),
            ("comment.yaml", "$comment: string\n"),
            ("custom-dollar.yaml", "$custom: string\n"),
            ("custom-extension.yaml", "x-custom: string\n"),
            ("pure.yaml", "$schema:\n  name: string(required)\n"),
            (
                "kinded.yaml",
                "kind: schema\ntypes:\n  name: string(required)\n",
            ),
            ("mixed.yaml", "name: string(required)\ncount: 1\n"),
            ("invalid.yaml", "name: string(min(nope))\n"),
        ];

        let dir = tempfile::tempdir().unwrap();
        for (name, body) in cases {
            write_schema_file(dir.path(), name, body);
            let resolved =
                resolve_yaml_schema(&yaml_value(&format!("./{name}")), dir.path(), &biscuit_file::FileResolutionContext::new(dir.path())).unwrap();
            assert!(
                resolved.advisories.is_empty(),
                "unexpected advisory for {name}"
            );
        }

        write_schema_file(
            dir.path(),
            "target.yaml",
            "$schema:\n  name: string(required)\n",
        );
        write_schema_file(dir.path(), "scalar.yaml", "$schema: ./target.yaml\n");
        let resolved =
            resolve_yaml_schema(&yaml_value("./scalar.yaml"), dir.path(), &biscuit_file::FileResolutionContext::new(dir.path())).unwrap();
        assert!(resolved.advisories.is_empty());
    }

    #[test]
    fn root_union_sorts_and_deduplicates_schema_advisories() {
        let dir = tempfile::tempdir().unwrap();
        let z_path = write_schema_file(dir.path(), "z.yaml", "zeta: string(required)\n");
        let a_path = write_schema_file(dir.path(), "a.yaml", "alpha: number(default(1))\n");
        let resolved = resolve_yaml_schema(
            &yaml_value("- ./z.yaml\n- ./a.yaml\n- ./z.yaml\n"),
            dir.path(),
            &biscuit_file::FileResolutionContext::new(dir.path()),
        )
        .unwrap();

        assert_eq!(
            resolved
                .advisories
                .iter()
                .map(SchemaAdvisory::path)
                .collect::<Vec<_>>(),
            vec![a_path.clone(), z_path.clone()]
        );

        write_schema_file(
            dir.path(),
            "root.yaml",
            "$schema:\n  - ./z.yaml\n  - ./a.yaml\n  - ./z.yaml\n",
        );
        let nested = resolve_yaml_schema(&yaml_value("./root.yaml"), dir.path(), &biscuit_file::FileResolutionContext::new(dir.path())).unwrap();
        assert_eq!(
            nested
                .advisories
                .iter()
                .map(SchemaAdvisory::path)
                .collect::<Vec<_>>(),
            vec![a_path, z_path]
        );
    }

    #[test]
    fn non_object_yaml_is_ambiguous() {
        let dir = tempfile::tempdir().unwrap();
        write_schema_file(dir.path(), "bad.yaml", "- one\n- two\n");
        let v = yaml_value("./bad.yaml");
        let err = resolve_yaml_schema(&v, dir.path(), &biscuit_file::FileResolutionContext::new(dir.path())).unwrap_err();
        assert!(matches!(err, SchemaError::AmbiguousReferenced { .. }));
    }

    #[test]
    fn missing_file_errors_with_no_match() {
        let dir = tempfile::tempdir().unwrap();
        let v = yaml_value("./nope.yaml");
        let err = resolve_yaml_schema(&v, dir.path(), &biscuit_file::FileResolutionContext::new(dir.path())).unwrap_err();
        assert!(matches!(err, SchemaError::NoMatch { bare_name: false, .. }), "{err:?}");
        assert!(!err.to_string().contains("::file-links"), "a plain miss has no glob hint: {err}");
    }

    /// A `$schema` reference that looks like a glob names one file; its miss
    /// is a `NoMatch` carrying the literal-glob hint. A malformed reference is
    /// `Unresolved` with its syntax error and carries no hint, whatever its
    /// text.
    #[test]
    fn literal_glob_miss_hints_and_invalid_syntax_does_not() {
        let dir = tempfile::tempdir().unwrap();
        let context = biscuit_file::FileResolutionContext::new(dir.path());

        let glob = resolve_yaml_schema(&yaml_value("./schemas/*.yaml"), dir.path(), &context).unwrap_err();
        assert!(matches!(glob, SchemaError::NoMatch { .. }), "{glob:?}");
        assert!(glob.to_string().contains("::file-links"), "{glob}");

        let malformed = resolve_yaml_schema(&yaml_value("~user/*.yaml"), dir.path(), &context).unwrap_err();
        assert!(matches!(malformed, SchemaError::Unresolved { .. }), "{malformed:?}");
        assert!(!malformed.to_string().contains("::file-links"), "{malformed}");
    }

    #[test]
    fn baseline_merge_adds_baseline_only_props() {
        let baseline = json!({
            "type": "object",
            "properties": {
                "owner": { "type": "string" }
            },
            "required": ["owner"]
        });
        let doc = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" }
            },
            "required": ["title"]
        });
        let merged = merge_baseline(&baseline, doc).unwrap();
        let props = merged["properties"].as_object().unwrap();
        assert!(props.contains_key("owner"));
        assert!(props.contains_key("title"));
        let required = merged["required"].as_array().unwrap();
        let names: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(names.contains(&"owner"));
        assert!(names.contains(&"title"));
    }

    #[test]
    fn baseline_merge_document_wins_on_conflict() {
        let baseline = json!({
            "type": "object",
            "properties": {
                "title": { "type": "number" }
            }
        });
        let doc = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" }
            }
        });
        let merged = merge_baseline(&baseline, doc).unwrap();
        assert_eq!(merged["properties"]["title"]["type"], "string");
    }

    #[test]
    fn baseline_must_be_simple_object_schema() {
        let baseline = json!({
            "type": "object",
            "patternProperties": { "^x": {} }
        });
        let doc = json!({"type":"object","properties":{}});
        let err = merge_baseline(&baseline, doc).unwrap_err();
        assert!(matches!(err, SchemaError::Baseline { .. }));
    }

    #[test]
    fn baseline_rejects_additional_properties_false() {
        let baseline = json!({
            "type": "object",
            "properties": { "owner": { "type": "string" } },
            "additionalProperties": false,
        });
        let doc = json!({"type":"object","properties":{}});
        let err = merge_baseline(&baseline, doc).unwrap_err();
        match err {
            SchemaError::Baseline { message, .. } => {
                assert!(
                    message.contains("additionalProperties"),
                    "expected message to mention additionalProperties, got: {message}"
                );
            }
            other => panic!("expected SchemaError::Baseline, got {other:?}"),
        }
    }

    #[test]
    fn baseline_rejects_additional_properties_schema() {
        let baseline = json!({
            "type": "object",
            "properties": { "owner": { "type": "string" } },
            "additionalProperties": { "type": "string" },
        });
        let doc = json!({"type":"object","properties":{}});
        let err = merge_baseline(&baseline, doc).unwrap_err();
        assert!(matches!(err, SchemaError::Baseline { .. }));
    }

    #[test]
    fn baseline_permits_additional_properties_true() {
        // SimplifiedSchema-generated baselines emit `additionalProperties:
        // true`; permitting that keeps the generated path working.
        let baseline = json!({
            "type": "object",
            "properties": { "owner": { "type": "string" } },
            "additionalProperties": true,
        });
        let doc = json!({"type":"object","properties":{}});
        merge_baseline(&baseline, doc).unwrap();
    }

    #[test]
    fn baseline_merge_into_root_union_applies_per_arm() {
        let baseline = json!({
            "type": "object",
            "properties": {
                "owner": { "type": "string" }
            }
        });
        let doc = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "anyOf": [
                {"type":"object","properties":{"title":{"type":"string"}}},
                {"type":"object","properties":{"name":{"type":"string"}}}
            ]
        });
        let merged = merge_baseline(&baseline, doc).unwrap();
        let arms = merged["anyOf"].as_array().unwrap();
        for arm in arms {
            assert!(arm["properties"].get("owner").is_some());
        }
    }

    // ── `generated` baseline merge audit (Phase 2) ──────────────────────────

    /// A baseline `generated; required` property must NOT get force-added to
    /// the merged `required` list. The convert layer suppresses the
    /// `required` entry for generated properties, so by the time the baseline
    /// reaches `merge_baseline` its `required` array already excludes the
    /// generated property — the merge has nothing to copy over. This test
    /// proves that contract end-to-end (SimplifiedSchema → JSON Schema →
    /// merge_baseline) and guards against a future regression that re-introduces
    /// generated properties into the baseline `required` list.
    #[test]
    fn baseline_generated_required_property_is_not_force_added_to_required() {
        // Build the baseline via the real convert path so the audit reflects
        // the actual baseline shape a library caller hands to merge_baseline.
        let baseline_yaml = "ctx_today: 'string(generated; required)'";
        let baseline_value: YamlValue = serde_yaml_ng::from_str(baseline_yaml).unwrap();
        let baseline_schema = parse_yaml_schema(&baseline_value).unwrap();
        let baseline_json = to_json_schema(&baseline_schema).unwrap();

        // Sanity: the generated property is absent from the baseline's own
        // `required` array (this is the convert-level suppression).
        assert!(
            baseline_json.get("required").is_none(),
            "baseline must not carry a static `required` entry for a generated property: {baseline_json:?}"
        );

        // A document schema that does not declare `ctx_today`.
        let doc = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" }
            }
        });
        let merged = merge_baseline(&baseline_json, doc).unwrap();

        // The generated property is copied across (it is baseline-only), but
        // the merged `required` list must NOT contain it.
        assert!(
            merged["properties"].get("ctx_today").is_some(),
            "generated baseline property should be merged into properties: {merged:?}"
        );
        let required = merged.get("required").and_then(|r| r.as_array());
        match required {
            None => { /* no required array at all — also fine */ }
            Some(arr) => {
                let names: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
                assert!(
                    !names.contains(&"ctx_today"),
                    "generated baseline property must not be force-added to required: {merged:?}"
                );
            }
        }
    }

    /// End-to-end: a baseline that models `ctx` as a nested object with a
    /// `generated; required` inner `today` property merges cleanly, and an
    /// authored document omitting `ctx` entirely validates. This mirrors the
    /// motivating `darkmatter.yaml` baseline shape.
    #[test]
    fn baseline_nested_ctx_generated_validates_when_ctx_absent() {
        use crate::markdown::schemas::validate::build_structural_validator;
        let baseline_yaml = "ctx:\n  today: \"date(generated; required)\"";
        let baseline_value: YamlValue = serde_yaml_ng::from_str(baseline_yaml).unwrap();
        let baseline_schema = parse_yaml_schema(&baseline_value).unwrap();
        let baseline_json = to_json_schema(&baseline_schema).unwrap();

        let doc = json!({"type": "object", "properties": {}});
        let merged = merge_baseline(&baseline_json, doc).unwrap();
        let validator = build_structural_validator(&merged).unwrap();

        // Authored document omits `ctx` entirely — validates.
        assert!(
            validator.is_valid(&json!({ "title": "Hello" })),
            "authored doc omitting generated ctx must validate: merged={merged:?}"
        );
        // Host supplies a wrongly-typed `ctx.today` — fails type validation.
        assert!(
            !validator.is_valid(&json!({ "ctx": { "today": 42 } })),
            "wrongly-typed ctx.today must fail: merged={merged:?}"
        );
        // Host supplies a correctly-typed `ctx.today` — validates.
        assert!(
            validator.is_valid(&json!({ "ctx": { "today": "2026-07-04" } })),
            "correctly-typed ctx.today must validate: merged={merged:?}"
        );
    }
}

/// Tests for the schema-plus composition primitives
/// (`darkmatter/features/2026-07-08-schema-plus/`): cross-file named-type
/// imports (Feature B), `example(...)` resolution (Feature A), and end-to-end
/// fixture validation of the acceptance corpus through the public resolve path.
#[cfg(test)]
mod schema_plus_phase1 {
    use super::*;
    use std::path::PathBuf;

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    /// Resolves the `$schema` block of a written schema file from its own dir.
    fn resolve_file(dir: &Path, name: &str) -> Result<ResolvedSchema, SchemaError> {
        let raw = std::fs::read_to_string(dir.join(name)).unwrap();
        let fm: YamlValue = serde_yaml_ng::from_str(&raw).unwrap();
        let schema_value = fm
            .get("$schema")
            .expect("schema file must have a `$schema` key");
        resolve_yaml_schema(schema_value, dir, &biscuit_file::FileResolutionContext::new(dir))
    }

    // ── Feature B — `@` cross-file named-type import ─────────────────────

    #[test]
    fn expands_named_type_import() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "types.yaml",
            "$schema:\n  type: 'enum(string, number; required)'\n",
        );
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  value: type@./types.yaml\n",
        );
        let resolved = resolve_file(dir.path(), "main.yaml").expect("import must resolve");
        // `value` inherits the definition's `required`, so it is not
        // null-wrapped: the inlined enum sits at the property root.
        let value = &resolved.json_schema["properties"]["value"];
        assert!(
            value["enum"].is_array(),
            "`value` must inline the target's `type` enum: {value}"
        );
        assert_eq!(resolved.json_schema["required"], serde_json::json!(["value"]));
    }

    #[test]
    fn resolves_this_self_import() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "self.yaml",
            "$schema:\n  type: 'enum(a, b; required)'\n  value: type@this\n",
        );
        let resolved = resolve_file(dir.path(), "self.yaml").expect("`@this` must resolve");
        let value = &resolved.json_schema["properties"]["value"];
        assert!(
            value["enum"].is_array(),
            "`value` must inline the same-file `type` enum and its `required`: {value}"
        );
        assert_eq!(
            resolved.json_schema["required"],
            serde_json::json!(["type", "value"])
        );
    }

    #[test]
    fn expands_array_named_type_import() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "types.yaml",
            "$schema:\n  parameter:\n    \"<string>\": any\n    $constraints:\n      min-keys: 1\n      max-keys: 1\n",
        );
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  parameters: parameter[]@./types.yaml\n",
        );
        let resolved = resolve_file(dir.path(), "main.yaml").expect("array import must resolve");
        let arm = &resolved.json_schema["properties"]["parameters"]["anyOf"][1];
        assert_eq!(
            arm["type"], "array",
            "`parameter[]@file` must inline an array: {arm}"
        );
    }

    #[test]
    fn import_from_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  value: type@./nope.yaml\n",
        );
        let err = resolve_file(dir.path(), "main.yaml").expect_err("missing import file must error");
        assert!(
            !matches!(err, SchemaError::Grammar { .. }),
            "a missing import target must be a resolution error, not a grammar error: {err:?}"
        );
    }

    #[test]
    fn import_of_missing_type_name_errors() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "types.yaml", "$schema:\n  type: 'enum(a, b)'\n");
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  value: missing@./types.yaml\n",
        );
        let err =
            resolve_file(dir.path(), "main.yaml").expect_err("undefined named type must error");
        let SchemaError::UnknownNamedType { name, namespace, site } = err else {
            panic!("an undefined named type must be UnknownNamedType: {err:?}");
        };
        assert_eq!(name, "missing");
        assert_eq!(
            namespace,
            SchemaSourceId::File(canonical_path(&dir.path().join("types.yaml")))
        );
        assert_eq!(site.source, SchemaSourceId::InlineDocument);
        assert_eq!(site.path, SchemaSourcePath::root().property("value"));
    }

    #[test]
    fn import_cycle_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "a.yaml",
            "$schema:\n  type: type@./b.yaml\n",
        );
        write(
            dir.path(),
            "b.yaml",
            "$schema:\n  type: type@./a.yaml\n",
        );
        let err = resolve_file(dir.path(), "a.yaml").expect_err("import cycle must error");
        // An alias cycle never descends into a child value, so it is the
        // typed cycle error rather than a grammar error.
        let SchemaError::NonDescendingCycle { cycles } = &err else {
            panic!("an alias cycle must be a NonDescendingCycle: {err:?}");
        };
        assert_eq!(cycles.len(), 1, "{err:?}");
        assert_eq!(cycles[0].len(), 2, "both references form the cycle: {err:?}");
    }

    #[test]
    fn constrained_import_applies_use_site_constraint() {
        // `Name(constraints)@file` applies the import-site constraint to the
        // inlined value type. Here the named `type` has no presence of its
        // own, so `type(required)@this` alone makes `value` required.
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "self.yaml",
            "$schema:\n  type: 'enum(a, b)'\n  value: type(required)@this\n",
        );
        let resolved = resolve_file(dir.path(), "self.yaml").expect("constrained import resolves");
        // `value` is required, so it is not null-wrapped: the enum lives at the
        // property root, not under an `anyOf` null wrapper.
        let value = &resolved.json_schema["properties"]["value"];
        assert!(
            value["enum"].is_array(),
            "constrained import must inline the enum at the property root: {value}"
        );
        let required = resolved.json_schema["required"]
            .as_array()
            .expect("required array");
        assert!(
            required.iter().any(|v| v == "value"),
            "`type(required)@this` must mark `value` required: {required:?}"
        );
    }

    #[test]
    fn import_from_non_simplified_file_errors() {
        // A raw JSON Schema file exposes no named types, so importing from it is
        // an ambiguous-reference error, not a grammar error.
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "raw.json",
            r#"{"type":"object","properties":{"x":{"type":"number"}}}"#,
        );
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  value: type@./raw.json\n",
        );
        let err = resolve_file(dir.path(), "main.yaml")
            .expect_err("import from a raw JSON Schema must error");
        assert!(
            matches!(err, SchemaError::AmbiguousReferenced { .. }),
            "importing from a non-SimplifiedSchema file must be an ambiguous-reference error: {err:?}"
        );
    }

    #[test]
    fn unresolved_import_reaching_conversion_is_a_convert_error() {
        // The resolver is the only inliner: a `SimplifiedSchema` still carrying
        // a `TypeExpr::Imported` at conversion time is a hard `Convert` error
        // (parity with an unresolved root-union `FileRef`).
        let atom = crate::markdown::schemas::simplified::grammar::parse_type_expr(
            "value",
            "type@./types.yaml",
        )
        .expect("import expression parses");
        let mut shape = SchemaShape::new();
        shape
            .properties
            .insert("value".into(), PropertyDef::Single(atom));
        let err = to_json_schema(&SimplifiedSchema::Single(shape))
            .expect_err("an unresolved import must not convert");
        assert!(
            matches!(err, SchemaError::Convert { .. }),
            "an unresolved import reaching conversion must be a Convert error: {err:?}"
        );
    }

    #[test]
    fn import_records_dependency_edge() {
        // The resolved product records the imported file as a dependency edge so
        // a warm cache / DMLS index can invalidate on imported-file change.
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "types.yaml",
            "$schema:\n  type: 'enum(a, b)'\n",
        );
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  value: type@./types.yaml\n",
        );
        let resolved = resolve_file(dir.path(), "main.yaml").expect("import resolves");
        let expected = canonical_path(&dir.path().join("types.yaml"));
        assert!(
            resolved.imports.contains(&expected),
            "resolved schema must record the imported file as a dependency edge: {:?}",
            resolved.imports
        );
    }

    // ── Feature A — `example(...)` resolution ────────────────────────────

    /// A minimal valid example artifact (envelope-conformant).
    const VALID_EXAMPLE: &str =
        "kind: example\ninvocation: \"{{ ctx.today }}\"\nreturns: \"2024-12-25\"\ndescription: d\n";

    #[test]
    fn example_resolves_and_emits_resolved_object() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "today-example.yaml", VALID_EXAMPLE);
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  today: \"date(example(./today-example.yaml))\"\n",
        );
        let resolved = resolve_file(dir.path(), "main.yaml").expect("example must resolve");
        // `today` is optional, so `x-darkmatter-example` rides on the outer
        // (nullable) property wrapper, not the typed date arm.
        let examples = &resolved.json_schema["properties"]["today"]["x-darkmatter-example"];
        assert!(examples.is_array(), "expected resolved example array: {examples}");
        assert_eq!(
            examples[0]["kind"], "example",
            "reference string must be replaced by the resolved example object: {examples}"
        );
        // And the resolved file is recorded as a dependency edge.
        let expected = canonical_path(&dir.path().join("today-example.yaml"));
        assert!(
            resolved.examples.contains(&expected),
            "resolved example file must be a dependency edge: {:?}",
            resolved.examples
        );
    }

    #[test]
    fn multiple_examples_resolve_to_multiple_objects() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.yaml", VALID_EXAMPLE);
        write(dir.path(), "b.yaml", VALID_EXAMPLE);
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  today: \"date(example(./a.yaml, ./b.yaml))\"\n",
        );
        let resolved = resolve_file(dir.path(), "main.yaml").expect("examples must resolve");
        let examples = resolved.json_schema["properties"]["today"]["x-darkmatter-example"]
            .as_array()
            .expect("resolved example array");
        assert_eq!(examples.len(), 2, "both examples must resolve: {examples:?}");
    }

    #[test]
    fn example_resolves_relative_subdirectory_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("ctx")).unwrap();
        write(dir.path(), "ctx/today-example.yaml", VALID_EXAMPLE);
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  today: \"date(example(./ctx/today-example.yaml))\"\n",
        );
        let resolved = resolve_file(dir.path(), "main.yaml").expect("subdir example resolves");
        assert!(
            resolved.json_schema["properties"]["today"]["x-darkmatter-example"][0]["kind"]
                == "example"
        );
    }

    #[test]
    fn missing_example_file_is_a_schema_load_error() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  today: \"date(example(./nope.yaml))\"\n",
        );
        let err = resolve_file(dir.path(), "main.yaml").expect_err("missing example must error");
        assert!(
            !matches!(err, SchemaError::Grammar { .. }),
            "a missing example file must be a schema-load error, not a grammar error: {err:?}"
        );
    }

    #[test]
    fn malformed_example_envelope_is_a_schema_load_error() {
        let dir = tempfile::tempdir().unwrap();
        // `kind` is required and must be the `example` enum; `sample` fails.
        write(
            dir.path(),
            "bad-example.yaml",
            "kind: sample\ninvocation: \"x\"\n",
        );
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  today: \"date(example(./bad-example.yaml))\"\n",
        );
        let err =
            resolve_file(dir.path(), "main.yaml").expect_err("malformed example must error");
        assert!(
            matches!(err, SchemaError::InvalidExample { .. }),
            "a malformed example artifact must be an InvalidExample error: {err:?}"
        );
    }

    #[test]
    fn invalid_inherited_parameters_is_a_schema_load_error() {
        let dir = tempfile::tempdir().unwrap();
        // A two-key parameter map violates the exact one-key arity (O-A4).
        write(
            dir.path(),
            "bad-params.yaml",
            "kind: example\nparameters:\n  - a: 1\n    b: 2\ninvocation: \"f(a, b)\"\nreturns: x\ndescription: d\n",
        );
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  today: \"date(example(./bad-params.yaml))\"\n",
        );
        let err = resolve_file(dir.path(), "main.yaml")
            .expect_err("invalid parameters must error");
        match err {
            SchemaError::InvalidExample { message, .. } => {
                assert!(
                    message.contains("parameters"),
                    "error must name the parameters failure: {message}"
                );
            }
            other => panic!("expected InvalidExample, got {other:?}"),
        }
    }

    #[test]
    fn example_returns_matching_target_type_resolves() {
        // Positive guard: a required, concretely-typed property whose example
        // `returns` matches the annotated target type resolves cleanly.
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "count-example.yaml",
            "kind: example\ninvocation: \"count()\"\nreturns: 3\ndescription: d\n",
        );
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  count: \"number(required; example(./count-example.yaml))\"\n",
        );
        let resolved =
            resolve_file(dir.path(), "main.yaml").expect("matching-type example must resolve");
        let examples = &resolved.json_schema["properties"]["count"]["x-darkmatter-example"];
        assert_eq!(
            examples[0]["kind"], "example",
            "example must resolve when its `returns` matches the number target: {examples}"
        );
    }

    #[test]
    fn example_returns_wrong_target_type_is_a_schema_load_error() {
        // The core gap this fix closes: a well-formed envelope whose `returns`
        // is the wrong type for the annotated property (string `returns` on a
        // `number` property) must fail at load with `InvalidExample`, not slip
        // through because only the envelope was checked.
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "bad-returns.yaml",
            "kind: example\ninvocation: \"count()\"\nreturns: \"not-a-number\"\ndescription: d\n",
        );
        write(
            dir.path(),
            "main.yaml",
            "$schema:\n  count: \"number(required; example(./bad-returns.yaml))\"\n",
        );
        let err = resolve_file(dir.path(), "main.yaml")
            .expect_err("wrong-typed example `returns` must error");
        match err {
            SchemaError::InvalidExample { message, .. } => assert!(
                message.contains("annotated target type"),
                "error must name the target-type mismatch: {message}"
            ),
            other => panic!("expected InvalidExample, got {other:?}"),
        }
    }

    #[test]
    fn example_this_targets_the_current_file() {
        // `example(this)` targets the schema file itself. Here that file is a
        // valid example artifact, so `this` resolution loads and validates it.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("self-example.yaml");
        std::fs::write(&path, VALID_EXAMPLE).unwrap();

        let mut json = serde_json::json!({
            "type": "object",
            "properties": {
                "demo": { "type": "string", "x-darkmatter-example": ["this"] }
            }
        });
        let deps = resolve_document_examples(
            &mut json,
            dir.path(),
            Some(&path),
            &[],
            &FileResolutionContext::new(dir.path()),
            &mut BTreeSet::new(),
        )
            .expect("`this` example must resolve");
        assert_eq!(
            json["properties"]["demo"]["x-darkmatter-example"][0]["kind"],
            "example"
        );
        assert!(deps.contains(&canonical_path(&path)));
    }

    #[test]
    fn schema_without_examples_leaves_json_unchanged() {
        // Fast path: a schema with no `example(...)` records no example edges
        // and its compiled JSON is untouched.
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "main.yaml", "$schema:\n  today: date\n");
        let resolved = resolve_file(dir.path(), "main.yaml").expect("resolves");
        assert!(resolved.examples.is_empty());
        assert!(
            resolved.json_schema["properties"]["today"]
                .get("x-darkmatter-example")
                .is_none()
        );
    }

    // ── Fixture validation (acceptance corpus) ───────────────────────────

    fn feature_dir() -> PathBuf {
        biscuit_test_harness::manifest_dir!()
            .join("..")
            .join("features")
            .join("_completed")
            .join("2026-07-08-schema-plus")
    }

    fn fixture_json(name: &str) -> serde_json::Value {
        let raw = std::fs::read_to_string(feature_dir().join(name)).expect("read fixture");
        let y: YamlValue = serde_yaml_ng::from_str(&raw).expect("parse fixture yaml");
        serde_json::to_value(&y).expect("fixture yaml -> json")
    }

    /// Validates a corpus fixture through the same example-artifact path a
    /// schema load uses (envelope + `parameters` shape, with native-value
    /// coercion for the `{ frontmatter: yaml }` arm).
    fn assert_fixture_valid(name: &str) {
        assert!(
            feature_dir().join(name).exists(),
            "fixture `{name}` must exist (path check to isolate harness issues from feature gaps)"
        );
        let doc = fixture_json(name);
        super::super::example::validate_example_object(name, &doc)
            .unwrap_or_else(|err| panic!("example envelope must validate fixture `{name}`: {err}"));
    }

    /// Drift guard: the on-disk `example.yaml` envelope fixture compiles to the
    /// same JSON Schema as the built-in [`super::super::example::EXAMPLE_ENVELOPE_YAML`],
    /// so the acceptance corpus and the runtime validator stay in lockstep.
    #[test]
    fn example_envelope_matches_fixture() {
        let dir = feature_dir();
        let raw = std::fs::read_to_string(dir.join("example.yaml")).expect("read example.yaml");
        let fm: YamlValue = serde_yaml_ng::from_str(&raw).expect("parse example.yaml");
        let schema_value = fm.get("$schema").expect("example.yaml has a `$schema` key");
        let fixture_json =
            resolve_yaml_schema(schema_value, &dir, &biscuit_file::FileResolutionContext::new(&dir)).expect("example.yaml envelope must resolve");

        let builtin: YamlValue = serde_yaml_ng::from_str(
            super::super::example::EXAMPLE_ENVELOPE_YAML,
        )
        .expect("built-in envelope parses");
        let builtin_json =
            to_json_schema(&parse_yaml_schema(&builtin).expect("built-in envelope parses"))
                .expect("built-in envelope converts");

        assert_eq!(
            fixture_json.json_schema, builtin_json,
            "on-disk example.yaml must match the built-in example envelope"
        );
    }

    #[test]
    fn example_validates_today_fixture() {
        assert_fixture_valid("today-example.yaml");
    }

    #[test]
    fn example_validates_as_unordered_list_fixture() {
        assert_fixture_valid("as_unordered_list-example.yaml");
    }

    #[test]
    fn example_validates_as_unordered_list_2_fixture() {
        assert_fixture_valid("as_unordered_list-example-2.yaml");
    }

    #[test]
    fn example_validates_as_unordered_list_fm_fixture() {
        assert_fixture_valid("as_unordered_list-example-fm.yaml");
    }
}

/// Phase 3 — bare-name schema-root resolution tests.
///
/// Bare names (`$schema: claudine.yaml`, `Name@claudine.yaml`,
/// `example(claudine.yaml)`) resolve against the schema-root list, nearest
/// first. Path-qualified references are untouched. A bare name not found in any
/// root but with a document-sibling file produces a `./name` suggestion.
#[cfg(test)]
mod bare_name_phase3 {
    use super::*;
    use std::path::PathBuf;

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    fn yaml_value(input: &str) -> YamlValue {
        serde_yaml_ng::from_str(input).expect("yaml parse")
    }

    fn parses_as_bare_name(raw: &str) -> bool {
        FileReference::new(raw).is_ok_and(|file_ref| is_bare_name(&file_ref))
    }

    // ── Parsed bare-name classification ─────────────────────────────────

    #[test]
    fn bare_name_plain_filename() {
        assert!(parses_as_bare_name("claudine.yaml"));
        assert!(parses_as_bare_name("schema.yml"));
    }

    #[test]
    fn bare_name_rejects_path_separator() {
        assert!(!parses_as_bare_name("./claudine.yaml"));
        assert!(!parses_as_bare_name("docs/claudine.yaml"));
        assert!(!parses_as_bare_name("../types.yaml"));
        assert!(!parses_as_bare_name("/abs/schema.yaml"));
        assert!(!parses_as_bare_name(r"docs\schema.yaml"));
    }

    #[test]
    fn bare_name_rejects_special_and_recursive_classes() {
        for raw in [
            "@schema.yaml",
            "^schema.yaml",
            "%schema.yaml",
            "vault:schema.yaml",
            "vault::schema.yaml",
            "~/schema.yaml",
            "https://example.com/schema.yaml",
        ] {
            let file_ref = FileReference::new(raw).unwrap();
            assert!(!is_bare_name(&file_ref), "{raw} must not be a bare name");
        }
        assert!(matches!(
            FileReference::new("!schema.yaml"),
            Err(biscuit_file::FileReferenceError::InvalidSyntax(_))
        ));
    }

    #[test]
    fn invalid_reference_fails_before_bare_name_classification() {
        assert!(matches!(
            FileReference::new(""),
            Err(biscuit_file::FileReferenceError::InvalidSyntax(_))
        ));
        assert!(matches!(
            FileReference::new("%"),
            Err(biscuit_file::FileReferenceError::InvalidSyntax(_))
        ));
    }

    // ── $schema bare-name resolution ────────────────────────────────────

    #[test]
    fn bare_name_resolves_nearest_root_first() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Two schema roots: pkg/schemas (near) and schemas (far).
        fs::create_dir_all(root.join("pkg/schemas")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::create_dir_all(root.join("pkg/docs")).unwrap();

        // Same filename in both roots — near wins.
        write(
            &root.join("pkg/schemas"),
            "claudine.yaml",
            "$schema:\n  prompt: 'string(required)'\n",
        );
        write(
            &root.join("schemas"),
            "claudine.yaml",
            "$schema:\n  title: 'string(required)'\n",
        );

        let roots = vec![
            root.join("pkg/schemas"),
            root.join("schemas"),
        ];
        let v = yaml_value("claudine.yaml");
        let resolved =
            resolve_yaml_schema_with_roots(&v, &root.join("pkg/docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("pkg/docs"))).expect("resolves");

        // Near root wins → `prompt` is required.
        let required = resolved.json_schema["required"].as_array().unwrap();
        assert!(
            required.iter().any(|v| v == "prompt"),
            "nearest root must win: {required:?}"
        );
        assert!(
            resolved.referenced_files.len() == 1,
            "one referenced file recorded"
        );
        assert!(
            resolved.referenced_files[0].ends_with(
                Path::new("pkg")
                    .join("schemas")
                    .join("claudine.yaml")
            ),
            "referenced file must be from the near root"
        );
    }

    #[test]
    #[serial_test::serial(schema_root_snapshot)]
    fn bare_name_root_probe_uses_captured_environment() {
        const NAME_VAR: &str = "DARKMATTER_SCHEMA_ROOT_NAME";
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let schemas = root.join("schemas");
        let docs = root.join("docs");
        fs::create_dir_all(&schemas).unwrap();
        fs::create_dir_all(&docs).unwrap();
        write(
            &schemas,
            "captured.yaml",
            "$schema:\n  captured: 'string(required)'\n",
        );

        let request_context = FileResolutionContext::from_snapshot(
            &docs,
            Some(root.join("captured-home")),
            std::collections::HashMap::from([(
                NAME_VAR.to_string(),
                "captured.yaml".to_string(),
            )]),
        );
        let prior_name = std::env::var_os(NAME_VAR);
        let prior_home = std::env::var_os("HOME");
        // SAFETY: this test is serialized while process-global state is changed.
        unsafe {
            std::env::set_var(NAME_VAR, "ambient.yaml");
            std::env::set_var("HOME", root.join("ambient-home"));
        }

        let value = yaml_value("'{{DARKMATTER_SCHEMA_ROOT_NAME}}'");
        let result = resolve_yaml_schema_with_roots(
            &value,
            &docs,
            std::slice::from_ref(&schemas),
            &request_context,
        );

        match prior_name {
            Some(value) => unsafe { std::env::set_var(NAME_VAR, value) },
            None => unsafe { std::env::remove_var(NAME_VAR) },
        }
        match prior_home {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }

        let resolved = result.expect("captured schema name resolves");
        let required = resolved.json_schema["required"].as_array().unwrap();
        assert!(required.iter().any(|value| value == "captured"));
    }

    #[test]
    fn bare_name_resolves_from_far_root_when_near_lacks_it() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("pkg/schemas")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::create_dir_all(root.join("pkg/docs")).unwrap();

        // Only far root has the file.
        write(
            &root.join("schemas"),
            "claudine.yaml",
            "$schema:\n  title: 'string(required)'\n",
        );

        let roots = vec![
            root.join("pkg/schemas"),
            root.join("schemas"),
        ];
        let v = yaml_value("claudine.yaml");
        let resolved =
            resolve_yaml_schema_with_roots(&v, &root.join("pkg/docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("pkg/docs"))).expect("resolves");
        let required = resolved.json_schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "title"));
    }

    #[test]
    fn bare_name_pins_to_schema_root_not_repository_root() {
        // The precedence flip made implicit resolution document-first. A
        // schema-root probe must stay pinned to its root: with a collision
        // between the repository root and a nearer configured schema root, the
        // schema root wins (feature review-1 Finding 4). An unpinned
        // (implicit) probe would let `<repo>/schema.yaml` shadow
        // `<repo>/schemas/schema.yaml` and silently validate the wrong schema.
        let tmp = tempfile::tempdir().unwrap();
        // Canonicalize so macOS `/var` -> `/private/var` matches the workdir
        // `gix` reports for repository discovery.
        let root = std::fs::canonicalize(tmp.path()).unwrap();
        // A real git repo makes the repository-root collision representative of
        // the request context used by composition hosts.
        gix::init(&root).unwrap();

        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();

        // Collision: same bare name at the repository root and the configured
        // schema root, each with a distinguishing required field.
        write(
            &root,
            "schema.yaml",
            "$schema:\n  repo_wins: 'string(required)'\n",
        );
        write(
            &root.join("schemas"),
            "schema.yaml",
            "$schema:\n  schema_root_wins: 'string(required)'\n",
        );

        let roots = vec![root.join("schemas")];
        let v = yaml_value("schema.yaml");
        let resolved =
            resolve_yaml_schema_with_roots(&v, &root.join("docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("docs"))).expect("resolves");

        let required = resolved.json_schema["required"].as_array().unwrap();
        assert!(
            required.iter().any(|v| v == "schema_root_wins"),
            "configured schema root must win over the repository root: {required:?}"
        );
        assert!(
            !required.iter().any(|v| v == "repo_wins"),
            "repository-root file must not be selected: {required:?}"
        );
        assert_eq!(resolved.referenced_files.len(), 1);
        assert!(
            resolved.referenced_files[0]
                .to_string_lossy()
                .contains("schemas"),
            "referenced file must come from the configured schema root: {:?}",
            resolved.referenced_files[0]
        );
    }

    #[test]
    fn bare_name_ordering_survives_repository_root_collision() {
        // Nearest-first ordering across multiple configured roots must hold even
        // when the repository root also carries the colliding name. The near
        // root wins; neither the far root nor the repository root shadows it.
        let tmp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(tmp.path()).unwrap();
        gix::init(&root).unwrap();

        fs::create_dir_all(root.join("near")).unwrap();
        fs::create_dir_all(root.join("far")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();

        write(
            &root,
            "schema.yaml",
            "$schema:\n  repo_wins: 'string(required)'\n",
        );
        write(
            &root.join("near"),
            "schema.yaml",
            "$schema:\n  near_wins: 'string(required)'\n",
        );
        write(
            &root.join("far"),
            "schema.yaml",
            "$schema:\n  far_wins: 'string(required)'\n",
        );

        let roots = vec![root.join("near"), root.join("far")];
        let v = yaml_value("schema.yaml");
        let resolved =
            resolve_yaml_schema_with_roots(&v, &root.join("docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("docs"))).expect("resolves");

        let required = resolved.json_schema["required"].as_array().unwrap();
        assert!(
            required.iter().any(|v| v == "near_wins"),
            "nearest configured root must win: {required:?}"
        );
        assert!(
            !required.iter().any(|v| v == "far_wins" || v == "repo_wins"),
            "neither the far root nor the repository root may shadow the near root: {required:?}"
        );
    }

    #[test]
    fn path_qualified_ref_unchanged_with_roots() {
        // A `./`-prefixed reference must NOT use schema roots.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();

        // Schema file in docs (sibling), not in schemas root.
        write(
            &root.join("docs"),
            "local.yaml",
            "$schema:\n  title: 'string(required)'\n",
        );
        // Different content in schemas root.
        write(
            &root.join("schemas"),
            "local.yaml",
            "$schema:\n  body: 'string(required)'\n",
        );

        let roots = vec![root.join("schemas")];
        let v = yaml_value("./local.yaml");
        let resolved =
            resolve_yaml_schema_with_roots(&v, &root.join("docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("docs"))).expect("resolves");

        // `./local.yaml` resolves to the sibling, not the root.
        let required = resolved.json_schema["required"].as_array().unwrap();
        assert!(
            required.iter().any(|v| v == "title"),
            "path-qualified ref must use document dir, not schema roots"
        );
    }

    #[test]
    fn sibling_only_bare_name_produces_suggestion() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();

        // Sibling file exists but schema root does NOT have it.
        write(
            &root.join("docs"),
            "local.yaml",
            "$schema:\n  title: 'string(required)'\n",
        );

        let roots = vec![root.join("schemas")];
        let v = yaml_value("local.yaml");
        let err = resolve_yaml_schema_with_roots(&v, &root.join("docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("docs"))).unwrap_err();

        match err {
            SchemaError::BareNameSiblingExists {
                reference,
                suggestion,
            } => {
                assert_eq!(reference, "local.yaml");
                assert_eq!(suggestion, "./local.yaml");
            }
            other => panic!("expected BareNameSiblingExists, got {other:?}"),
        }
    }

    #[test]
    fn bare_name_not_found_no_sibling_is_no_match() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();

        let roots = vec![root.join("schemas")];
        let v = yaml_value("nonexistent.yaml");
        let err = resolve_yaml_schema_with_roots(&v, &root.join("docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("docs"))).unwrap_err();
        assert!(matches!(err, SchemaError::NoMatch { bare_name: true, .. }), "{err:?}");
    }

    #[test]
    fn no_roots_preserves_legacy_behavior() {
        // Without schema_roots, a bare name resolves relative to base_dir
        // (backward compatibility).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        write(
            root,
            "sibling.yaml",
            "$schema:\n  title: 'string(required)'\n",
        );

        let v = yaml_value("sibling.yaml");
        let resolved = resolve_yaml_schema(&v, root, &biscuit_file::FileResolutionContext::new(root)).expect("resolves via legacy path");
        let required = resolved.json_schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "title"));
    }

    // ── Name@file import bare-name resolution ──────────────────────────

    #[test]
    fn bare_name_import_resolves_via_schema_roots() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();

        // types.yaml lives in the schema root.
        write(
            &root.join("schemas"),
            "types.yaml",
            "$schema:\n  base: 'enum(a, b)'\n",
        );
        // main schema references it with a bare name.
        write(
            &root.join("docs"),
            "main.yaml",
            "$schema:\n  value: base@types.yaml\n",
        );

        let roots = vec![root.join("schemas")];
        let main_raw = std::fs::read_to_string(root.join("docs/main.yaml")).unwrap();
        let main_yaml: YamlValue = serde_yaml_ng::from_str(&main_raw).unwrap();
        let schema_value = main_yaml.get("$schema").unwrap();

        let resolved = resolve_yaml_schema_with_roots(schema_value, &root.join("docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("docs")))
            .expect("import resolves via roots");

        // `value` should have the inlined enum from types.yaml.
        let value = &resolved.json_schema["properties"]["value"];
        let arm = &value["anyOf"][1];
        assert!(
            arm["enum"].is_array(),
            "bare-name import must resolve via schema roots: {value}"
        );
    }

    #[test]
    fn bare_name_import_sibling_suggestion() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();

        // types.yaml is a sibling, not in any schema root.
        write(
            &root.join("docs"),
            "types.yaml",
            "$schema:\n  base: 'enum(a, b)'\n",
        );
        write(
            &root.join("docs"),
            "main.yaml",
            "$schema:\n  value: base@types.yaml\n",
        );

        let roots = vec![root.join("schemas")];
        let main_raw = std::fs::read_to_string(root.join("docs/main.yaml")).unwrap();
        let main_yaml: YamlValue = serde_yaml_ng::from_str(&main_raw).unwrap();
        let schema_value = main_yaml.get("$schema").unwrap();

        let err = resolve_yaml_schema_with_roots(schema_value, &root.join("docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("docs")))
            .unwrap_err();
        assert!(
            matches!(err, SchemaError::BareNameSiblingExists { .. }),
            "sibling import must produce BareNameSiblingExists: {err:?}"
        );
    }

    // ── example(...) bare-name resolution ──────────────────────────────

    #[test]
    fn bare_name_example_resolves_via_schema_roots() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();

        const VALID_EXAMPLE: &str =
            "kind: example\ninvocation: \"x\"\nreturns: \"val\"\ndescription: d\n";

        // Example lives in the schema root.
        write(&root.join("schemas"), "demo-example.yaml", VALID_EXAMPLE);
        // Main schema references it with a bare name.
        write(
            &root.join("docs"),
            "main.yaml",
            "$schema:\n  demo: \"string(example(demo-example.yaml))\"\n",
        );

        let roots = vec![root.join("schemas")];
        let main_raw = std::fs::read_to_string(root.join("docs/main.yaml")).unwrap();
        let main_yaml: YamlValue = serde_yaml_ng::from_str(&main_raw).unwrap();
        let schema_value = main_yaml.get("$schema").unwrap();

        let resolved = resolve_yaml_schema_with_roots(schema_value, &root.join("docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("docs")))
            .expect("example resolves via roots");

        let examples = &resolved.json_schema["properties"]["demo"]["x-darkmatter-example"];
        assert!(
            examples.is_array(),
            "bare-name example must resolve via schema roots: {examples}"
        );
        assert_eq!(examples[0]["kind"], "example");
    }

    #[test]
    fn bare_name_example_sibling_suggestion() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();

        const VALID_EXAMPLE: &str =
            "kind: example\ninvocation: \"x\"\nreturns: \"val\"\ndescription: d\n";

        // Example is a sibling, not in schema root.
        write(&root.join("docs"), "demo-example.yaml", VALID_EXAMPLE);
        write(
            &root.join("docs"),
            "main.yaml",
            "$schema:\n  demo: \"string(example(demo-example.yaml))\"\n",
        );

        let roots = vec![root.join("schemas")];
        let main_raw = std::fs::read_to_string(root.join("docs/main.yaml")).unwrap();
        let main_yaml: YamlValue = serde_yaml_ng::from_str(&main_raw).unwrap();
        let schema_value = main_yaml.get("$schema").unwrap();

        let err = resolve_yaml_schema_with_roots(schema_value, &root.join("docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("docs")))
            .unwrap_err();
        assert!(
            matches!(err, SchemaError::BareNameSiblingExists { .. }),
            "sibling example must produce BareNameSiblingExists: {err:?}"
        );
    }

    // ── Error display ──────────────────────────────────────────────────

    #[test]
    fn bare_name_sibling_display_includes_suggestion() {
        let err = SchemaError::BareNameSiblingExists {
            reference: "types.yaml".into(),
            suggestion: "./types.yaml".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("types.yaml"));
        assert!(msg.contains("./types.yaml"));
    }

    /// Proves the three reference kinds (`$schema`, `Name@file`, `example(...)`)
    /// share a single resolution ladder — all three go through the same
    /// bare-name path when schema roots are configured.
    #[test]
    fn shared_ladder_proves_no_duplicate_resolver() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();

        // A single file in the schema root serves all three reference kinds.
        write(
            &root.join("schemas"),
            "shared.yaml",
            "$schema:\n  base: 'enum(a, b)'\n",
        );

        let roots = vec![root.join("schemas")];

        // 1. `$schema` bare-name reference.
        let v = yaml_value("shared.yaml");
        let r1 = resolve_yaml_schema_with_roots(&v, &root.join("docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("docs"))).unwrap();
        assert!(r1.referenced_files.len() == 1);

        // 2. `Name@file` bare-name import target.
        write(
            &root.join("docs"),
            "importer.yaml",
            "$schema:\n  value: base@shared.yaml\n",
        );
        let importer_raw = std::fs::read_to_string(root.join("docs/importer.yaml")).unwrap();
        let importer_yaml: YamlValue = serde_yaml_ng::from_str(&importer_raw).unwrap();
        let schema_value = importer_yaml.get("$schema").unwrap();
        let r2 = resolve_yaml_schema_with_roots(schema_value, &root.join("docs"), &roots, &biscuit_file::FileResolutionContext::new(root.join("docs"))).unwrap();
        let expected = canonical_path(&root.join("schemas/shared.yaml"));
        assert!(
            r2.imports.contains(&expected),
            "import must resolve through the shared ladder: {:?}",
            r2.imports
        );

        // 3. The shared file resolved from a `$schema` reference has the same
        // canonical path as the import target.
        let r1_canonical = &r1.referenced_files[0];
        assert_eq!(
            r1_canonical, &expected,
            "$schema and Name@file must resolve to the same file through the shared ladder"
        );
    }

    /// Multiple `Name@file` imports targeting the same file all inline
    /// correctly and record the target as a single dependency edge — the
    /// per-run namespace cache (F27) must not change resolution semantics.
    #[test]
    fn multiple_imports_from_same_file_resolve_and_dedup() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("docs")).unwrap();

        write(
            &root.join("docs"),
            "types.yaml",
            "$schema:\n  A: 'string(required)'\n  B: 'number'\n  C: 'enum(x, y)'\n",
        );
        write(
            &root.join("docs"),
            "importer.yaml",
            "$schema:\n  first: A@types.yaml\n  second: B@types.yaml\n  third: C@types.yaml\n",
        );

        let raw = std::fs::read_to_string(root.join("docs/importer.yaml")).unwrap();
        let importer: YamlValue = serde_yaml_ng::from_str(&raw).unwrap();
        let schema_value = importer.get("$schema").unwrap();
        let resolved =
            resolve_yaml_schema_with_roots(schema_value, &root.join("docs"), &[], &biscuit_file::FileResolutionContext::new(root.join("docs"))).unwrap();

        // All three named types inlined to their declared shapes. `A` carries
        // `required`, which `first` inherits, so `first` is not null-wrapped;
        // the optional `second` and `third` lower to a nullable `anyOf`.
        let props = resolved.json_schema["properties"].as_object().unwrap();
        assert_eq!(props["first"], serde_json::json!({"type": "string"}));
        assert_eq!(resolved.json_schema["required"], serde_json::json!(["first"]));
        let arm_type = |key: &str| -> String {
            props[key]["anyOf"]
                .as_array()
                .unwrap()
                .iter()
                .find_map(|arm| arm.get("type").filter(|t| *t != "null"))
                .map(|t| t.as_str().unwrap().to_string())
                .unwrap_or_default()
        };
        assert_eq!(arm_type("second"), "number");
        let third_enum = props["third"]["anyOf"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|arm| arm.get("enum"))
            .unwrap();
        assert_eq!(third_enum.as_array().unwrap().len(), 2);

        // The shared target file is recorded exactly once as a dependency edge.
        let expected = canonical_path(&root.join("docs/types.yaml"));
        assert_eq!(
            resolved.imports.iter().filter(|p| *p == &expected).count(),
            1,
            "the shared import target must appear once, not once per import site"
        );
    }

    /// Suppress unused import warning for PathBuf when tests compile.
    #[test]
    fn _pathbuf_used() {
        let _ = PathBuf::new();
    }
}

/// Shared named-type tables: allocation identity through the real resolver,
/// and use-site independence through public resolution, for both standalone
/// mapping envelopes as the import target.
#[cfg(test)]
mod shared_type_tables {
    use super::*;
    use serde_json::json;

    /// Writes `types.yaml` holding `body` (a two-space-indented mapping) as
    /// either a pure `$schema:` or a tagged `kind: schema` / `types:` envelope.
    fn write_types(dir: &Path, tagged: bool, body: &str) -> PathBuf {
        let header = if tagged { "kind: schema\ntypes:\n" } else { "$schema:\n" };
        let path = dir.join("types.yaml");
        std::fs::write(&path, format!("{header}{body}")).unwrap();
        path
    }

    fn engine(dir: &Path) -> ImportEngine<'static> {
        ImportEngine::new(NamespaceSource::Files {
            schema_roots: Vec::new(),
            context: Box::new(FileResolutionContext::new(dir)),
        })
    }

    fn root_namespace(dir: &Path, named_types: IndexMap<String, PropertyDef>) -> Namespace {
        Namespace {
            key: DefNamespace::InlineRoot,
            base_dir: dir.to_path_buf(),
            named_types: Arc::new(named_types),
        }
    }

    fn resolve_root(dir: &Path, schema: &str) -> ResolvedSchema {
        let value: YamlValue = serde_yaml_ng::from_str(schema).unwrap();
        resolve_yaml_schema(&value, dir, &FileResolutionContext::new(dir))
            .expect("schema must resolve")
    }

    // ── Allocation identity ──────────────────────────────────────────────

    #[test]
    fn first_load_cache_and_later_imports_share_one_table() {
        for tagged in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let types = write_types(dir.path(), tagged, "  A: 'string(required)'\n  B: number\n");
            let mut engine = engine(dir.path());
            let root = root_namespace(dir.path(), IndexMap::new());

            let first = engine.resolve_namespace("./types.yaml", &root).unwrap();
            let canonical = canonical_path(&types);
            assert_eq!(first.key, DefNamespace::File(canonical.clone()));
            let cached = &engine.namespace_cache[&canonical];
            assert!(
                Arc::ptr_eq(&first.named_types, &cached.named_types),
                "tagged={tagged}: first load and cache entry must share one allocation"
            );

            // Two later imports, selecting distinct names and spelling the
            // reference differently, reach the same canonical cache entry.
            let for_a = engine.resolve_namespace("./types.yaml", &root).unwrap();
            let for_b = engine.resolve_namespace("types.yaml", &root).unwrap();
            assert!(for_a.named_types.contains_key("A"), "tagged={tagged}");
            assert!(for_b.named_types.contains_key("B"), "tagged={tagged}");
            for later in [&for_a, &for_b] {
                assert!(
                    Arc::ptr_eq(&later.named_types, &first.named_types),
                    "tagged={tagged}: later imports must reuse the first-loaded table"
                );
            }

            // Full expansion of both imports keeps that one cached table.
            let importer: YamlValue =
                serde_yaml_ng::from_str("x: A@./types.yaml\ny: B@types.yaml\n").unwrap();
            let schema = parse_yaml_schema(&importer).unwrap();
            engine.expand_schema(schema, &root, &SchemaSourcePath::root()).unwrap();
            assert!(
                Arc::ptr_eq(&engine.namespace_cache[&canonical].named_types, &first.named_types),
                "tagged={tagged}: expansion must not replace the cached table"
            );
            assert_eq!(engine.dependencies.iter().collect::<Vec<_>>(), vec![&canonical]);
        }
    }

    #[test]
    fn this_from_inline_root_shares_the_root_table() {
        let dir = tempfile::tempdir().unwrap();
        let schema: YamlValue = serde_yaml_ng::from_str("name: string\n").unwrap();
        let SimplifiedSchema::Single(shape) = parse_yaml_schema(&schema).unwrap() else {
            panic!("mapping schema must parse as a single shape");
        };
        let root = root_namespace(dir.path(), shape.properties);
        let mut engine = engine(dir.path());

        for _ in 0..2 {
            let this = engine.resolve_namespace("this", &root).unwrap();
            assert_eq!(this.key, DefNamespace::InlineRoot);
            assert!(
                Arc::ptr_eq(&this.named_types, &root.named_types),
                "`@this` must share the root's table"
            );
        }
        assert!(engine.namespace_cache.is_empty(), "`@this` loads no file");
        assert!(engine.dependencies.is_empty(), "`@this` records no dependency");
    }

    #[test]
    fn this_from_loaded_file_shares_that_files_table() {
        for tagged in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let types = write_types(dir.path(), tagged, "  name: string\n");
            let mut engine = engine(dir.path());
            let root = root_namespace(dir.path(), IndexMap::new());
            let file = engine.resolve_namespace("./types.yaml", &root).unwrap();

            for _ in 0..2 {
                let this = engine.resolve_namespace("this", &file).unwrap();
                assert_eq!(this.key, DefNamespace::File(canonical_path(&types)));
                assert!(
                    Arc::ptr_eq(&this.named_types, &file.named_types),
                    "tagged={tagged}: `@this` in a file must share that file's table"
                );
                assert!(
                    !Arc::ptr_eq(&this.named_types, &root.named_types),
                    "tagged={tagged}: a file's table must be distinct from the root's"
                );
            }
        }
    }

    // ── Use-site independence ────────────────────────────────────────────

    #[test]
    fn cross_file_type_imported_twice_keeps_each_use_independent() {
        for tagged in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let types = write_types(dir.path(), tagged, "  name: 'string(required)'\n");
            let resolved = resolve_root(
                dir.path(),
                "primary: 'name(required; not-empty)@./types.yaml -> primary name'\n\
                 backup: name@./types.yaml\n",
            );
            assert_eq!(
                resolved.json_schema["properties"],
                json!({
                    "primary": {"type": "string", "pattern": "\\S", "description": "primary name"},
                    "backup": {"type": "string"},
                }),
                "tagged={tagged}: only `primary` carries its use-site modifiers"
            );
            // Both uses inherit the definition's `required`; the second use
            // is not affected by the first use's postfix.
            assert_eq!(
                resolved.json_schema["required"],
                json!(["primary", "backup"]),
                "tagged={tagged}"
            );
            assert_eq!(resolved.imports, vec![canonical_path(&types)], "tagged={tagged}");
        }
    }

    #[test]
    fn root_this_type_imported_twice_keeps_each_use_independent() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = resolve_root(
            dir.path(),
            "name: 'number(required)'\n\
             primary: 'name(required; min(1))@this -> primary count'\n\
             backup: name@this\n",
        );
        assert_eq!(
            resolved.json_schema["properties"],
            json!({
                "name": {"type": "number"},
                "primary": {"type": "number", "minimum": 1, "description": "primary count"},
                "backup": {"type": "number"},
            })
        );
        assert_eq!(resolved.json_schema["required"], json!(["name", "primary", "backup"]));
        assert!(resolved.imports.is_empty());
    }

    #[test]
    fn this_inside_imported_file_selects_that_files_type() {
        for tagged in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let types = write_types(
                dir.path(),
                tagged,
                "  name: 'string(required)'\n  wrapper:\n    \
                 first: 'name(required; not-empty)@this -> first'\n    second: name@this\n",
            );
            // The root defines its own, different `name`, so the output shows
            // which namespace each `@this` selected.
            let resolved = resolve_root(
                dir.path(),
                "name: number\nlocal: 'name(required)@this'\nw: wrapper@./types.yaml\n",
            );
            let properties = &resolved.json_schema["properties"];
            assert_eq!(properties["local"], json!({"type": "number"}), "tagged={tagged}");
            assert_eq!(
                properties["w"],
                json!({"anyOf": [{"type": "null"}, {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "first": {"type": "string", "pattern": "\\S", "description": "first"},
                        "second": {"type": "string"},
                    },
                    "required": ["first", "second"],
                }]}),
                "tagged={tagged}: `@this` inside types.yaml must select its string `name`"
            );
            assert_eq!(resolved.json_schema["required"], json!(["local"]), "tagged={tagged}");
            assert_eq!(resolved.imports, vec![canonical_path(&types)], "tagged={tagged}");
        }
    }

    #[test]
    fn expand_named_types_reads_this_from_the_supplied_schema_not_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let context = biscuit_file::FileResolutionContext::new(dir.path());
        std::fs::write(dir.path().join("types.yaml"), "kind: schema\ntypes:\n  shared: number\n").unwrap();
        // The defining file on disk is stale: it defines no `base`.
        let schema_file = dir.path().join("schema.yaml");
        std::fs::write(&schema_file, "kind: schema\ntypes:\n  other: string\n").unwrap();
        let buffer = parse_yaml_schema(
            &serde_yaml_ng::from_str("title: base\ncount: 'shared@./types.yaml'\nbase: string(required)\n")
                .unwrap(),
        )
        .unwrap();

        let NamedTypeExpansion { schema: expanded, imports, .. } =
            expand_named_types(&buffer, dir.path(), Some(&schema_file), &[], &context).unwrap();
        let SimplifiedSchema::Single(shape) = expanded else { panic!("single schema") };
        assert!(shape.properties["title"].is_required(), "`base` came from the buffer");
        assert_eq!(imports, vec![canonical_path(&dir.path().join("types.yaml"))]);

        let missing = parse_yaml_schema(&serde_yaml_ng::from_str("title: nope\n").unwrap()).unwrap();
        let error = expand_named_types(&missing, dir.path(), Some(&schema_file), &[], &context)
            .unwrap_err();
        let SchemaError::UnknownNamedType { name, namespace, site } = error else {
            panic!("unknown named type: {error:?}")
        };
        assert_eq!(name, "nope");
        assert_eq!(namespace, SchemaSourceId::File(canonical_path(&schema_file)));
        assert_eq!(site.path, SchemaSourcePath::root().property("title"));

        let inline = expand_named_types(&missing, dir.path(), None, &[], &context).unwrap_err();
        assert!(matches!(
            inline,
            SchemaError::UnknownNamedType { namespace: SchemaSourceId::InlineDocument, .. }
        ));
    }

    #[test]
    fn named_type_reference_path_selects_the_resolver_target() {
        let dir = tempfile::tempdir().unwrap();
        let context = biscuit_file::FileResolutionContext::new(dir.path());
        let types = dir.path().join("types.yaml");
        std::fs::write(&types, "kind: schema\ntypes:\n  shared: number\n").unwrap();
        assert_eq!(named_type_reference_path("this", dir.path(), &[], &context).unwrap(), None);
        assert_eq!(
            named_type_reference_path("./types.yaml", dir.path(), &[], &context).unwrap(),
            Some(canonical_path(&types))
        );
        assert!(matches!(
            named_type_reference_path("./missing.yaml", dir.path(), &[], &context),
            Err(SchemaError::NoMatch { .. })
        ));
    }
}

#[cfg(test)]
mod reserved_namespace {
    //! `Name@darkmatter:<stem>` names a type of a shipped partial whoever
    //! writes it: a disk schema, a Darkmatter embedded document, or a host
    //! bundle document. References inside the partial resolve among the
    //! shipped documents, also during a disk expansion.

    use super::*;

    fn yaml(input: &str) -> YamlValue {
        serde_yaml_ng::from_str(input).expect("yaml parse")
    }

    /// The property names of the object `json` lowers to, or of its
    /// non-null `anyOf` arm.
    fn keys(json: &Value) -> BTreeSet<String> {
        let object = json["anyOf"]
            .as_array()
            .and_then(|arms| arms.iter().find(|arm| arm["type"] == "object"))
            .unwrap_or(json);
        object["properties"].as_object().expect("an object").keys().cloned().collect()
    }

    #[test]
    fn a_disk_schema_reaches_a_shipped_partial_and_records_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let context = FileResolutionContext::new(dir.path());
        let resolved = resolve_yaml_schema(&yaml("talk: communicate@darkmatter:lifecycle\n"), dir.path(), &context)
            .expect("the reserved reference resolves");
        assert!(keys(&resolved.json_schema["properties"]["talk"]).contains("say"));
        assert!(resolved.referenced_files.is_empty(), "{:?}", resolved.referenced_files);
        assert!(resolved.imports.is_empty(), "a shipped partial is no file dependency: {:?}", resolved.imports);
    }

    #[test]
    fn a_disk_library_file_reaches_a_shipped_partial_from_any_folder() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("types.yaml"), "kind: schema\ntypes:\n  tally: count@darkmatter:lifecycle\n").unwrap();
        let context = FileResolutionContext::new(dir.path());
        let resolved = resolve_yaml_schema(&yaml("n: tally@./a/b/types.yaml\n"), dir.path(), &context)
            .expect("the library's reserved reference resolves");
        let arms = resolved.json_schema["properties"]["n"]["anyOf"].as_array().expect("count is a union");
        assert!(arms.iter().any(|arm| arm["type"] == "integer"), "{arms:?}");
        assert!(arms.iter().any(|arm| arm["pattern"].is_string()), "count's `interpolation` arm resolved locally");
        assert_eq!(resolved.imports, vec![canonical_path(&nested.join("types.yaml"))]);
    }

    #[test]
    fn relative_references_inside_a_reached_partial_stay_in_the_shipped_table() {
        // md-lifecycle.yaml names its verbs `info-short@./lifecycle.yaml`;
        // the importer's folder holds a decoy that must not be read.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("lifecycle.yaml"), "kind: schema\ntypes:\n  info-short: number\n").unwrap();
        let context = FileResolutionContext::new(dir.path());
        let resolved =
            resolve_yaml_schema(&yaml("talk: md-communicate@darkmatter:md-lifecycle\n"), dir.path(), &context)
                .expect("the partial's own imports resolve");
        let talk = &resolved.json_schema["properties"]["talk"];
        let object = talk["anyOf"].as_array().and_then(|arms| arms.iter().find(|arm| arm["type"] == "object")).unwrap_or(talk);
        let info = object["properties"]["info"].to_string();
        assert!(info.contains("\"string\"") && !info.contains("\"number\""), "{info}");
        assert!(resolved.imports.is_empty(), "{:?}", resolved.imports);
    }

    #[test]
    fn a_darkmatter_embedded_document_reaches_a_shipped_partial() {
        let (resolved, reads) = prepare_embedded_inline_schema(
            &yaml("talk: communicate@darkmatter:lifecycle\n"),
            embedded::SHIPPED[0].path,
            embedded::SHIPPED,
            "",
        )
        .expect("resolves in the shipped table");
        assert!(keys(&resolved.json_schema["properties"]["talk"]).contains("info"));
        assert!(reads.contains(&PathBuf::from("darkmatter/schemas/partials/lifecycle.yaml")), "{reads:?}");
    }

    #[test]
    fn a_host_bundle_document_reaches_a_shipped_partial_outside_its_bundle() {
        let documents = [EmbeddedDocument {
            path: "host/actions.yaml",
            text: "kind: schema\ntypes:\n  talk: communicate@darkmatter:lifecycle\n",
        }];
        let (resolved, _) = prepare_embedded_inline_schema(
            &yaml("talk: talk@./actions.yaml\n"),
            "host/trigger.yaml",
            &documents,
            "host",
        )
        .expect("the bundle's reserved reference resolves");
        assert!(keys(&resolved.json_schema["properties"]["talk"]).contains("say"));
        // A plain relative climb out of the bundle is still refused.
        let escape = prepare_embedded_inline_schema(
            &yaml("talk: communicate@../darkmatter/schemas/partials/lifecycle.yaml\n"),
            "host/trigger.yaml",
            &documents,
            "host",
        );
        assert!(escape.is_err(), "{escape:?}");
    }

    #[test]
    fn an_unknown_stem_is_an_error_naming_the_reference() {
        let dir = tempfile::tempdir().unwrap();
        let context = FileResolutionContext::new(dir.path());
        for reference in ["darkmatter:bogus", "darkmatter:", "darkmatter:../fm", "darkmatter:fm.yaml"] {
            let error = resolve_yaml_schema(&yaml(&format!("x: \"t@{reference}\"\n")), dir.path(), &context)
                .expect_err("an unknown stem fails");
            let SchemaError::NoMatch { reference: named, .. } = &error else {
                panic!("`{reference}`: {error:?}")
            };
            assert_eq!(named, reference);
            assert!(error.to_string().contains(reference), "{error}");
        }
        let embedded = prepare_embedded_inline_schema(&yaml("x: t@darkmatter:bogus\n"), "host/a.yaml", &[], "host");
        assert!(matches!(embedded, Err(SchemaError::NoMatch { .. })), "{embedded:?}");
        let missing_type = resolve_yaml_schema(&yaml("x: nope@darkmatter:lifecycle\n"), dir.path(), &context);
        assert!(
            matches!(&missing_type, Err(SchemaError::UnknownNamedType { name, .. }) if name == "nope"),
            "{missing_type:?}"
        );
    }

    #[test]
    fn a_reserved_reference_names_no_host_file() {
        let dir = tempfile::tempdir().unwrap();
        let context = FileResolutionContext::new(dir.path());
        assert_eq!(named_type_reference_path("darkmatter:lifecycle", dir.path(), &[], &context).unwrap(), None);
        assert_eq!(named_type_reference_path(" darkmatter:bogus ", dir.path(), &[], &context).unwrap(), None);
    }
}

#[cfg(test)]
mod searched_tests;

mod graph;
mod recursive;

#[cfg(test)]
mod embedded_recursion {
    //! Recursive named types inside embedded documents resolve against the
    //! embedded table alone, with no filesystem read.

    use super::*;

    const ITEM: &str = "kind: schema\ntypes:\n  item: \"{ children: item[] }\"\n";
    const PAIR: &str = "kind: schema\ntypes:\n  a: \"{ b: b@./pair.yaml }\"\n  b: \"{ a: a[]@./pair.yaml }\"\n";

    #[test]
    fn self_and_mutual_recursion_resolve_from_embedded_documents() {
        let documents = [
            EmbeddedDocument { path: "bundle/item.yaml", text: ITEM },
            EmbeddedDocument { path: "bundle/pair.yaml", text: PAIR },
            EmbeddedDocument {
                path: "bundle/root.yaml",
                text: "kind: schema\ntypes:\n  tree: item@./item.yaml\n  pair: a@./pair.yaml\n",
            },
        ];
        let (resolved, reads) = prepare_embedded_schema_in("bundle/root.yaml", &documents, "bundle")
            .expect("embedded recursion resolves");
        assert!(resolved.imports.is_empty(), "embedded documents record no file edge");
        assert_eq!(reads.len(), 3, "{reads:?}");
        let namespaces: Vec<&DefNamespace> = resolved.definitions.iter().map(|(key, _)| &key.namespace).collect();
        assert_eq!(
            namespaces,
            [
                &DefNamespace::Embedded("bundle/item.yaml".into()),
                &DefNamespace::Embedded("bundle/pair.yaml".into()),
                &DefNamespace::Embedded("bundle/pair.yaml".into()),
            ]
        );
        let defs = resolved.json_schema["$defs"].as_object().expect("$defs");
        assert_eq!(defs.keys().collect::<Vec<_>>(), ["item", "a", "b"]);
        assert_eq!(resolved.json_schema["properties"]["tree"]["anyOf"][1], serde_json::json!({"$ref": "#/$defs/item"}));
    }
}
