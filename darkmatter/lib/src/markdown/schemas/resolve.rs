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

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use biscuit_file::FileReference;
use crate::markdown::compose::{document_resolution_context, find_git_root_from};
use indexmap::IndexMap;
use serde_json::{Map, Value};
use serde_yaml_ng::Value as YamlValue;

use super::{
    SchemaArm, SchemaOrigin, SchemaShape, SimplifiedSchema,
    errors::SchemaError,
    simplified::{
        parse_standalone_schema_document, parse_yaml_schema, to_json_schema,
        types::{
            Constraint, PatternKeyDef, PropertyAtom, PropertyDef, SimplifiedType, TypeExpr,
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
    pub json_schema: Value,
    /// Where the schema came from — an inline `$schema` mapping/sequence
    /// (`Document`) or a referenced schema file (`ReferencedFile`, carrying
    /// the resolved path). Preserved so diagnostics can point
    /// `relatedInformation` at the schema source (R-5 Priority 2).
    pub origin: SchemaOrigin,

    /// Resolved paths of files pulled in by `Name@file` named-type imports
    /// (Feature B), in sorted order. Empty for schemas that use no imports.
    /// These are the dependency edges the base-schema LRU cache and DMLS live
    /// index consume to invalidate a schema when an imported file changes; the
    /// downstream invalidation wiring itself is deferred.
    pub imports: Vec<PathBuf>,

    /// Resolved paths of `example(...)` artifact files (Feature A), in sorted
    /// order. Empty for schemas with no `example()` constraints. Like
    /// [`ResolvedSchema::imports`], these are dependency edges for warm-cache /
    /// DMLS invalidation; the invalidation wiring itself is deferred.
    pub examples: Vec<PathBuf>,

    /// Resolved paths of the schema FILES this `$schema` references directly:
    /// the top-level referenced file (`$schema: ./schema.yaml`) and each
    /// root-union string arm's file. Sorted, deduplicated. Empty for inline
    /// `$schema` mappings. These are dependency edges: a change to the schema
    /// file's own content must invalidate a cached effective schema.
    pub referenced_files: Vec<PathBuf>,
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
/// - [`SchemaError::Unresolved`] when a file reference fails to resolve.
/// - [`SchemaError::Io`] when reading the referenced file fails.
/// - [`SchemaError::AmbiguousReferenced`] when the referenced file is neither
///   a valid SimplifiedSchema nor a JSON Schema.
/// - [`SchemaError::FrontmatterShape`] for unsupported `$schema` shapes.
/// - [`SchemaError::Grammar`] / [`SchemaError::Convert`] propagated from the
///   parser and converter.
pub fn resolve_schema(value: &Value, base_dir: &Path) -> Result<ResolvedSchema, SchemaError> {
    resolve_schema_with_roots(value, base_dir, &[])
}

/// Same as [`resolve_schema`] but threads schema-root resolution context for
/// bare-name references (Phase 3). A bare name (`$schema: claudine.yaml`)
/// resolves against `schema_roots` nearest-first; path-qualified references
/// are untouched.
pub fn resolve_schema_with_roots(
    value: &Value,
    base_dir: &Path,
    schema_roots: &[PathBuf],
) -> Result<ResolvedSchema, SchemaError> {
    let yaml = json_to_yaml(value);
    resolve_yaml_schema_with_roots(&yaml, base_dir, schema_roots)
}

/// Same as [`resolve_schema`] but takes a YAML value directly. Used by the
/// resolver itself (referenced YAML files) and by tests.
pub fn resolve_yaml_schema(
    value: &YamlValue,
    base_dir: &Path,
) -> Result<ResolvedSchema, SchemaError> {
    resolve_yaml_schema_with_roots(value, base_dir, &[])
}

/// Same as [`resolve_yaml_schema`] but threads schema-root resolution context
/// for bare-name references (Phase 3).
pub fn resolve_yaml_schema_with_roots(
    value: &YamlValue,
    base_dir: &Path,
    schema_roots: &[PathBuf],
) -> Result<ResolvedSchema, SchemaError> {
    match value {
        YamlValue::String(reference) => resolve_reference(reference, base_dir, schema_roots),
        YamlValue::Mapping(_) => {
            let schema = parse_yaml_schema(value)?;
            // Inline the definitions of any `Name@file` named-type imports
            // (Feature B) before conversion — `to_json_schema` rejects a
            // surviving import. This is the top-level document, so `@this`
            // resolves against the inline root (`NamespaceKey::Root`).
            let (schema, imports) =
                expand_document_imports(schema, base_dir, NamespaceKey::Root, schema_roots)?;
            let mut json = to_json_schema(&schema)?;
            // Inline `$schema` mappings have no on-disk path, so `this` example
            // references have no target here (the acceptance driver references
            // examples from a schema *file*, resolved in `parse_yaml_referenced_file`).
            let examples = resolve_document_examples(&mut json, base_dir, None, schema_roots)?;
            Ok(ResolvedSchema {
                simplified: Some(schema),
                json_schema: json,
                origin: SchemaOrigin::document(),
                imports,
                examples,
                referenced_files: Vec::new(),
            })
        }
        YamlValue::Sequence(items) => resolve_root_union(items, base_dir, schema_roots),
        other => Err(SchemaError::FrontmatterShape {
            message: format!(
                "$schema must be a mapping, sequence, or string; got {}",
                describe_yaml(other)
            ),
        }),
    }
}

fn resolve_root_union(
    items: &[YamlValue],
    base_dir: &Path,
    schema_roots: &[PathBuf],
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
    for item in items {
        match item {
            YamlValue::Mapping(_) => {
                let arm_schema = parse_yaml_schema(item)?;
                // Named-type imports are inlined per arm before conversion.
                let (arm_schema, arm_imports) =
                    expand_document_imports(arm_schema, base_dir, NamespaceKey::Root, schema_roots)?;
                imports.extend(arm_imports);
                let mut arm_json = to_json_schema(&arm_schema)?;
                examples.extend(resolve_document_examples(
                    &mut arm_json,
                    base_dir,
                    None,
                    schema_roots,
                )?);
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
                let resolved = resolve_reference(reference, base_dir, schema_roots)?;
                imports.extend(resolved.imports.iter().cloned());
                examples.extend(resolved.examples.iter().cloned());
                referenced_files.extend(resolved.referenced_files.iter().cloned());
                // If this arm itself came from a SimplifiedSchema file, preserve
                // it in the simplified projection only when it is a single
                // shape (root unions of unions are not modelled in v1).
                match (&resolved.simplified, all_simplified_arms.as_mut()) {
                    (Some(SimplifiedSchema::Single(shape)), Some(arms)) => {
                        arms.push(SchemaArm::Inline(shape.clone()));
                    }
                    _ => {
                        all_simplified_arms = None;
                    }
                }
                any_of.push(strip_schema_uri(resolved.json_schema));
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
    let mut root = Map::new();
    root.insert(
        "$schema".into(),
        Value::String(super::simplified::DRAFT_2020_12.into()),
    );
    root.insert("anyOf".into(), Value::Array(any_of));

    Ok(ResolvedSchema {
        simplified: all_simplified_arms.map(SimplifiedSchema::Union),
        json_schema: Value::Object(root),
        origin: SchemaOrigin::document(),
        imports: imports.into_iter().collect(),
        examples: examples.into_iter().collect(),
        referenced_files: referenced_files.into_iter().collect(),
    })
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

/// Returns `true` when `reference` is a bare name — no path separator and no
/// special prefix (`./`, `../`, `/`, `@`, `!`, `vault:`, `%`). A bare name
/// is eligible for schema-root resolution when roots are provided.
fn is_bare_name(reference: &str) -> bool {
    if reference.is_empty() {
        return false;
    }
    // Any path separator disqualifies — `docs/x.yaml`, `./x.yaml`, `/x`.
    if reference.contains('/') || reference.contains('\\') {
        return false;
    }
    // Magic-path prefixes — `@`, `!`, `vault:`, `%` — are never bare names.
    !reference.starts_with('@')
        && !reference.starts_with('!')
        && !reference.starts_with('%')
        && !reference.starts_with("vault:")
        && !reference.starts_with("vault::")
}

/// Tries a bare name against each schema root, nearest first. Returns the
/// resolved path of the first root that contains a file matching `name`, or
/// `None` when no root has it.
///
/// Each root is probed with explicit-relative (`./name`) semantics, which pins
/// the probe to exactly that root. An implicit-relative probe would instead
/// search the root's enclosing repository root first (the feature's new
/// repository-first policy), letting a repository-root file shadow a nearer
/// configured schema root and silently load the wrong schema. Explicit-relative
/// resolution never falls back off the given root, so the loop's advertised
/// nearest-first ordering is what actually decides the winner.
fn try_bare_name_in_roots(
    name: &str,
    schema_roots: &[PathBuf],
) -> Result<Option<PathBuf>, SchemaError> {
    // `name` is a bare name here (guaranteed by the `is_bare_name` guard at every
    // call site): no path separator and no sigil. Prefixing `./` therefore yields
    // a well-formed explicit-relative reference pinned to the root it resolves
    // from — `FileReference` stays the sole authority that turns it into a path.
    let pinned = format!("./{name}");
    let file_ref = FileReference::new(&pinned).map_err(|source| SchemaError::Unresolved {
        reference: name.to_string(),
        source,
    })?;
    for root in schema_roots {
        if let Some(path) = file_ref
            .resolve_from(root)
            .map_err(|source| SchemaError::Unresolved {
                reference: name.to_string(),
                source,
            })?
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn resolve_reference(
    reference: &str,
    base_dir: &Path,
    schema_roots: &[PathBuf],
) -> Result<ResolvedSchema, SchemaError> {
    let trimmed = reference.trim();
    if let Some(rest) = trimmed.strip_prefix("http://") {
        let _ = rest;
        return Err(SchemaError::RemoteUnsupported {
            reference: reference.into(),
        });
    }
    if let Some(rest) = trimmed.strip_prefix("https://") {
        let _ = rest;
        return Err(SchemaError::RemoteUnsupported {
            reference: reference.into(),
        });
    }

    // Bare-name resolution against schema roots (Phase 3). When schema roots
    // are provided and the reference has no path component, resolve against
    // the roots nearest-first instead of the document directory.
    if !schema_roots.is_empty() && is_bare_name(trimmed) {
        if let Some(path) = try_bare_name_in_roots(trimmed, schema_roots)? {
            let mut resolved = load_schema_from_path(&path, schema_roots)?;
            resolved.origin = SchemaOrigin::referenced_file(path.clone());
            resolved.referenced_files = vec![canonical_path(&path)];
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
        return Err(SchemaError::Unresolved {
            reference: reference.to_string(),
            source: biscuit_file::FileReferenceError::InvalidSyntax(format!(
                "bare-name reference `{trimmed}` not found in any schema root"
            )),
        });
    }

    let file_ref = FileReference::new(reference).map_err(|source| SchemaError::Unresolved {
        reference: reference.to_string(),
        source,
    })?;
    // `$schema` references resolve through the shared document-backed context:
    // explicit `./`/`../` from the document directory only, implicit path
    // references repository-root first then the document directory — the same
    // order the `file`-typed value references use. No ambient CWD is read.
    let repo_root = find_git_root_from(base_dir);
    let resolution_ctx = document_resolution_context(base_dir, None, &[], repo_root.as_deref());
    let path = file_ref
        .resolve_in_context(&resolution_ctx)
        .map_err(|source| SchemaError::Unresolved {
            reference: reference.to_string(),
            source,
        })?
        .ok_or_else(|| SchemaError::Unresolved {
            reference: reference.to_string(),
            source: biscuit_file::FileReferenceError::InvalidSyntax(format!(
                "no file matched `{reference}`"
            )),
        })?;

    let mut resolved = load_schema_from_path(&path, schema_roots)?;
    // The schema was loaded from a file — record the resolved path so
    // diagnostics can point `relatedInformation` at the referenced source.
    resolved.origin = SchemaOrigin::referenced_file(path.clone());
    // The referenced file's own content is a dependency edge: editing
    // `schema.yaml`'s type must invalidate a cached effective schema.
    resolved.referenced_files = vec![canonical_path(&path)];
    Ok(resolved)
}

fn load_schema_from_path(
    path: &Path,
    schema_roots: &[PathBuf],
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
        _ => parse_yaml_referenced_file(path, &bytes, schema_roots),
    }
}

fn parse_yaml_referenced_file(
    path: &Path,
    bytes: &[u8],
    schema_roots: &[PathBuf],
) -> Result<ResolvedSchema, SchemaError> {
    let text = std::str::from_utf8(bytes).map_err(|_| SchemaError::AmbiguousReferenced {
        path: path.to_path_buf(),
    })?;
    if let Some(document) = parse_standalone_schema_document(text, path)? {
        return resolve_standalone_schema(document.schema, path, schema_roots);
    }

    // Treat the file's contents as a raw JSON Schema serialised in YAML.
    let json: Value =
        serde_yaml_ng::from_str(text).map_err(|_| SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        })?;
    if !json.is_object() {
        return Err(SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        });
    }
    Ok(ResolvedSchema {
        simplified: None,
        json_schema: json,
        origin: SchemaOrigin::document(),
        imports: Vec::new(),
        examples: Vec::new(),
        referenced_files: Vec::new(),
    })
}

fn resolve_standalone_schema(
    schema: SimplifiedSchema,
    path: &Path,
    schema_roots: &[PathBuf],
) -> Result<ResolvedSchema, SchemaError> {
    let file_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let key = NamespaceKey::File(canonical_path(path));
    match schema {
        SimplifiedSchema::Single(_) => {
            let (schema, imports) = expand_document_imports(schema, file_dir, key, schema_roots)?;
            let mut json_schema = to_json_schema(&schema)?;
            let examples = resolve_document_examples(&mut json_schema, file_dir, Some(path), schema_roots)?;
            Ok(ResolvedSchema {
                simplified: Some(schema),
                json_schema,
                origin: SchemaOrigin::document(),
                imports,
                examples,
                referenced_files: Vec::new(),
            })
        }
        SimplifiedSchema::Union(arms) => {
            resolve_standalone_root_union(arms, file_dir, path, schema_roots)
        }
    }
}

fn resolve_standalone_root_union(
    arms: Vec<SchemaArm>,
    base_dir: &Path,
    path: &Path,
    schema_roots: &[PathBuf],
) -> Result<ResolvedSchema, SchemaError> {
    let mut any_of = Vec::with_capacity(arms.len());
    let mut simplified_arms = Vec::with_capacity(arms.len());
    let mut all_simplified = true;
    let mut imports = BTreeSet::new();
    let mut examples = BTreeSet::new();
    let mut referenced_files = BTreeSet::new();

    for arm in arms {
        match arm {
            SchemaArm::Inline(shape) => {
                let schema = SimplifiedSchema::Single(shape);
                let (schema, arm_imports) = expand_document_imports(
                    schema,
                    base_dir,
                    NamespaceKey::File(canonical_path(path)),
                    schema_roots,
                )?;
                imports.extend(arm_imports);
                let SimplifiedSchema::Single(shape) = schema else {
                    unreachable!("single root-union arm remains single")
                };
                let mut arm_json = to_json_schema(&SimplifiedSchema::Single(shape.clone()))?;
                examples.extend(resolve_document_examples(
                    &mut arm_json,
                    base_dir,
                    Some(path),
                    schema_roots,
                )?);
                simplified_arms.push(SchemaArm::Inline(shape));
                any_of.push(strip_schema_uri(arm_json));
            }
            SchemaArm::FileRef(reference) => {
                let resolved = resolve_reference(&reference, base_dir, schema_roots)?;
                imports.extend(resolved.imports.iter().cloned());
                examples.extend(resolved.examples.iter().cloned());
                referenced_files.extend(resolved.referenced_files.iter().cloned());
                if let Some(SimplifiedSchema::Single(shape)) = resolved.simplified {
                    simplified_arms.push(SchemaArm::Inline(shape));
                } else {
                    all_simplified = false;
                }
                any_of.push(strip_schema_uri(resolved.json_schema));
            }
        }
    }

    let mut root = Map::new();
    root.insert(
        "$schema".into(),
        Value::String(super::simplified::DRAFT_2020_12.into()),
    );
    root.insert("anyOf".into(), Value::Array(any_of));
    Ok(ResolvedSchema {
        simplified: all_simplified.then_some(SimplifiedSchema::Union(simplified_arms)),
        json_schema: Value::Object(root),
        origin: SchemaOrigin::document(),
        imports: imports.into_iter().collect(),
        examples: examples.into_iter().collect(),
        referenced_files: referenced_files.into_iter().collect(),
    })
}

fn parse_raw_json_schema(path: &Path, bytes: &[u8]) -> Result<ResolvedSchema, SchemaError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        })?;
    if !value.is_object() {
        return Err(SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        });
    }
    Ok(ResolvedSchema {
        simplified: None,
        json_schema: value,
        origin: SchemaOrigin::document(),
        imports: Vec::new(),
        examples: Vec::new(),
        referenced_files: Vec::new(),
    })
}

// ── Cross-file named-type imports (Feature B) ───────────────────────────────
//
// `Name@fileref` inlines the definition of a named type `Name` from the
// referenced file at the use site (structural substitution). A schema file's
// top-level `$schema:` entries are its named types; `@this` targets the
// current file. Postfix composes outward: `Name[]@file` becomes an array of
// the inlined type, and `Name(constraints)@file` applies constraints to the
// inlined value. Expansion is eager, bounded, and cycle-checked (named types
// form a DAG in v1); `to_json_schema` still rejects any import that survives.

/// Hard cap on named-type import expansion depth. Mirrors the inline-object
/// depth cap so import chains cannot bypass the recursion guard indefinitely.
const MAX_IMPORT_DEPTH: usize = 32;

/// Cycle-detection identity for a named-type namespace. The inline document
/// root has no path, so it uses a dedicated sentinel; every other namespace is
/// keyed by the canonicalized path of the file it was loaded from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NamespaceKey {
    /// The inline document `$schema` (or a root-union arm) — the top-level
    /// schema being resolved, which has no on-disk path of its own.
    Root,
    /// A named-type file, keyed by its canonicalized path.
    File(PathBuf),
}

/// A named-type namespace: the set of top-level `$schema:` entries a file (or
/// the inline document) exposes for `Name@…` imports, plus the context needed
/// to resolve relative imports and `@this` inside it.
struct Namespace {
    /// Cycle-detection identity for this namespace.
    key: NamespaceKey,
    /// Directory that relative imports authored inside this namespace resolve
    /// against (mirrors `$schema` file-ref resolution).
    base_dir: PathBuf,
    /// The unexpanded top-level named types (`$schema:` entries).
    named_types: IndexMap<String, PropertyDef>,
}

/// Drives eager, bounded, cycle-checked expansion of `Name@file` imports over
/// a parsed [`SimplifiedSchema`].
struct ImportEngine {
    /// Stack of `(namespace, type-name)` frames currently being expanded, used
    /// to detect direct and transitive cycles.
    stack: Vec<(NamespaceKey, String)>,
    /// Resolved import file paths, deduplicated — the dependency edges a warm
    /// cache / DMLS index consumes to invalidate on imported-file change.
    dependencies: BTreeSet<PathBuf>,
    /// Schema-root resolution context for bare-name import targets (Phase 3).
    schema_roots: Vec<PathBuf>,
}

/// Expands every `Name@file` import in `schema`. `base_dir` and `key` describe
/// the document being resolved: `base_dir` anchors relative imports and `@this`
/// resolves against `key` and the document's own top-level named types.
/// `schema_roots` provides bare-name resolution context for import targets
/// that have no path component.
///
/// Returns the import-free schema plus the sorted dependency edges. Schemas
/// with no imports are returned untouched (byte-equivalent to legacy output).
fn expand_document_imports(
    schema: SimplifiedSchema,
    base_dir: &Path,
    key: NamespaceKey,
    schema_roots: &[PathBuf],
) -> Result<(SimplifiedSchema, Vec<PathBuf>), SchemaError> {
    if !schema_has_imports(&schema) {
        return Ok((schema, Vec::new()));
    }
    // `@this` resolves against the document's own (unexpanded) named types.
    let named_types = match &schema {
        SimplifiedSchema::Single(shape) => shape.properties.clone(),
        SimplifiedSchema::Union(_) => IndexMap::new(),
    };
    let current = Namespace {
        key,
        base_dir: base_dir.to_path_buf(),
        named_types,
    };
    let mut engine = ImportEngine {
        stack: Vec::new(),
        dependencies: BTreeSet::new(),
        schema_roots: schema_roots.to_vec(),
    };
    let expanded = engine.expand_schema(schema, &current)?;
    Ok((expanded, engine.dependencies.into_iter().collect()))
}

impl ImportEngine {
    fn expand_schema(
        &mut self,
        schema: SimplifiedSchema,
        current: &Namespace,
    ) -> Result<SimplifiedSchema, SchemaError> {
        match schema {
            SimplifiedSchema::Single(shape) => {
                Ok(SimplifiedSchema::Single(self.expand_shape(shape, current)?))
            }
            SimplifiedSchema::Union(arms) => {
                let mut out = Vec::with_capacity(arms.len());
                for arm in arms {
                    out.push(match arm {
                        SchemaArm::Inline(shape) => {
                            SchemaArm::Inline(self.expand_shape(shape, current)?)
                        }
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
    ) -> Result<SchemaShape, SchemaError> {
        let mut properties = IndexMap::with_capacity(shape.properties.len());
        for (name, def) in shape.properties {
            properties.insert(name, self.expand_property_def(def, current)?);
        }
        let mut pattern_keys = Vec::with_capacity(shape.pattern_keys.len());
        for pk in shape.pattern_keys {
            pattern_keys.push(PatternKeyDef {
                key: pk.key,
                def: self.expand_property_def(pk.def, current)?,
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
    ) -> Result<PropertyDef, SchemaError> {
        match def {
            PropertyDef::Single(atom) => self.expand_atom(atom, current),
            PropertyDef::Union(arms) => {
                // A union-typed named type flattens into the enclosing union.
                let mut out = Vec::with_capacity(arms.len());
                for atom in arms {
                    match self.expand_atom(atom, current)? {
                        PropertyDef::Single(a) => out.push(a),
                        PropertyDef::Union(mut arms2) => out.append(&mut arms2),
                    }
                }
                Ok(PropertyDef::Union(out))
            }
        }
    }

    fn expand_atom(
        &mut self,
        atom: PropertyAtom,
        current: &Namespace,
    ) -> Result<PropertyDef, SchemaError> {
        let PropertyAtom {
            ty,
            is_array,
            constraints,
            array_constraints,
            description,
        } = atom;
        match ty {
            TypeExpr::Primitive(p) => Ok(PropertyDef::Single(PropertyAtom {
                ty: TypeExpr::Primitive(p),
                is_array,
                constraints,
                array_constraints,
                description,
            })),
            TypeExpr::InlineObject(shape) => {
                let expanded = self.expand_shape(shape, current)?;
                Ok(PropertyDef::Single(PropertyAtom {
                    ty: TypeExpr::InlineObject(expanded),
                    is_array,
                    constraints,
                    array_constraints,
                    description,
                }))
            }
            TypeExpr::Imported { name, reference } => self.expand_import(
                name,
                reference,
                is_array,
                constraints,
                array_constraints,
                description,
                current,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn expand_import(
        &mut self,
        name: String,
        reference: String,
        is_array: bool,
        constraints: Vec<Constraint>,
        array_constraints: Vec<Constraint>,
        description: Option<String>,
        current: &Namespace,
    ) -> Result<PropertyDef, SchemaError> {
        if self.stack.len() >= MAX_IMPORT_DEPTH {
            return Err(SchemaError::ImportCycle {
                chain: format!(
                    "named-type import expansion exceeded {MAX_IMPORT_DEPTH} levels near \
                     `{name}@{reference}`"
                ),
            });
        }

        let target = self.resolve_namespace(&reference, current)?;
        let def = target.named_types.get(&name).cloned().ok_or_else(|| {
            // A missing named type is a resolution error, not a grammar error:
            // the import parsed fine, the target file simply does not define it.
            SchemaError::Convert {
                property: name.clone(),
                message: format!(
                    "named type `{name}` is not defined in `{reference}` (a schema file's \
                     top-level `$schema:` entries are its named types)"
                ),
            }
        })?;

        let frame = (target.key.clone(), name.clone());
        if self.stack.contains(&frame) {
            return Err(SchemaError::ImportCycle {
                chain: self.describe_cycle(&name),
            });
        }
        self.stack.push(frame);
        let expanded_base = self.expand_property_def(def, &target)?;
        self.stack.pop();

        // Presence/ownership constraints (`required`, `generated`) describe the
        // named type's role as a property in its *defining* file, not the
        // reusable type — so they are stripped on inline. The use site supplies
        // its own presence via import-site `(constraints)`.
        let base = strip_top_level_presence(expanded_base);
        apply_import_postfix(
            base,
            is_array,
            constraints,
            array_constraints,
            description,
            &name,
            &reference,
        )
    }

    /// Resolves the namespace named on the right of `@`. `this` targets the
    /// current namespace; any other value is a `biscuit-file` reference resolved
    /// relative to `current.base_dir` (mirroring `$schema` file-ref resolution).
    /// Bare-name references (no path component) resolve against the schema-root
    /// context nearest-first when roots are configured (Phase 3).
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
                named_types: current.named_types.clone(),
            });
        }
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return Err(SchemaError::RemoteUnsupported {
                reference: reference.to_string(),
            });
        }

        // Bare-name resolution against schema roots (Phase 3).
        let path = if !self.schema_roots.is_empty() && is_bare_name(trimmed) {
            if let Some(p) = try_bare_name_in_roots(trimmed, &self.schema_roots)? {
                p
            } else {
                // Check sibling for pointed error.
                let sibling = current.base_dir.join(trimmed);
                if sibling.is_file() {
                    return Err(SchemaError::BareNameSiblingExists {
                        reference: trimmed.to_string(),
                        suggestion: format!("./{trimmed}"),
                    });
                }
                return Err(SchemaError::Unresolved {
                    reference: reference.to_string(),
                    source: biscuit_file::FileReferenceError::InvalidSyntax(format!(
                        "bare-name reference `{trimmed}` not found in any schema root"
                    )),
                });
            }
        } else {
            let file_ref =
                FileReference::new(reference).map_err(|source| SchemaError::Unresolved {
                    reference: reference.to_string(),
                    source,
                })?;
            file_ref
                .resolve_from(&current.base_dir)
                .map_err(|source| SchemaError::Unresolved {
                    reference: reference.to_string(),
                    source,
                })?
                .ok_or_else(|| SchemaError::Unresolved {
                    reference: reference.to_string(),
                    source: biscuit_file::FileReferenceError::InvalidSyntax(format!(
                        "no file matched `{reference}`"
                    )),
                })?
        };

        let canonical = canonical_path(&path);
        self.dependencies.insert(canonical.clone());
        let named_types = load_named_types(&path)?;
        let base_dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(Namespace {
            key: NamespaceKey::File(canonical),
            base_dir,
            named_types,
        })
    }

    /// Renders the current expansion stack as a `a -> b -> a` chain for the
    /// cycle-error message.
    fn describe_cycle(&self, repeat: &str) -> String {
        let mut parts: Vec<&str> = self.stack.iter().map(|(_, n)| n.as_str()).collect();
        parts.push(repeat);
        parts.join(" -> ")
    }
}

/// Canonicalizes `path` for stable cycle-detection identity, falling back to
/// the given path when canonicalization fails (it should not, since the path
/// was just resolved on disk).
fn canonical_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Loads a standalone schema file's mapping payload as named types.
///
/// ## Errors
///
/// Returns [`SchemaError::AmbiguousReferenced`] when the file is not a
/// SimplifiedSchema mapping (raw JSON Schema or a root union) —
/// imports are only defined among a file's named types, so a raw JSON Schema
/// has none to offer.
fn load_named_types(path: &Path) -> Result<IndexMap<String, PropertyDef>, SchemaError> {
    let bytes = fs::read(path).map_err(|source| SchemaError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|_| SchemaError::AmbiguousReferenced {
        path: path.to_path_buf(),
    })?;
    let Some(document) = parse_standalone_schema_document(text, path)? else {
        return Err(SchemaError::AmbiguousReferenced {
            path: path.to_path_buf(),
        });
    };
    match document.schema {
        SimplifiedSchema::Single(shape) => Ok(shape.properties),
        SimplifiedSchema::Union(_) => Err(SchemaError::SchemaDocument {
            path: path.to_path_buf(),
            message: "root-union schema documents cannot supply named imports".into(),
        }),
    }
}

/// Removes the top-level `required` / `generated` constraints from an inlined
/// named-type definition. Nested constraints (inside inline objects, array
/// items) are untouched — only the outermost presence/ownership semantics are
/// re-decided by the use site.
fn strip_top_level_presence(def: PropertyDef) -> PropertyDef {
    fn strip(mut atom: PropertyAtom) -> PropertyAtom {
        atom.constraints
            .retain(|c| !matches!(c, Constraint::Required | Constraint::Generated));
        atom.array_constraints
            .retain(|c| !matches!(c, Constraint::Required | Constraint::Generated));
        atom
    }
    match def {
        PropertyDef::Single(atom) => PropertyDef::Single(strip(atom)),
        PropertyDef::Union(arms) => PropertyDef::Union(arms.into_iter().map(strip).collect()),
    }
}

/// Applies the import-site postfix (`[]`, `(constraints)`, `-> description`) to
/// an inlined base definition. Verbatim substitution when the site adds no
/// postfix; otherwise the base must be a single atom (v1 does not compose
/// postfix over a union-typed named type).
fn apply_import_postfix(
    base: PropertyDef,
    is_array: bool,
    mut constraints: Vec<Constraint>,
    array_constraints: Vec<Constraint>,
    description: Option<String>,
    name: &str,
    reference: &str,
) -> Result<PropertyDef, SchemaError> {
    if !is_array && constraints.is_empty() && array_constraints.is_empty() && description.is_none()
    {
        return Ok(base);
    }

    let base_atom = match base {
        PropertyDef::Single(atom) => atom,
        PropertyDef::Union(_) => {
            return Err(SchemaError::Convert {
                property: name.to_string(),
                message: format!(
                    "cannot apply `[]`/constraints to the union-typed named type `{name}@{reference}`"
                ),
            });
        }
    };

    if is_array
        && constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::Suggest(_)))
    {
        return Err(SchemaError::Grammar {
            property: name.to_string(),
            message: "`suggest` is not valid at the array level".into(),
            span: constraints
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
        let TypeExpr::Primitive(ty @ (SimplifiedType::String | SimplifiedType::Number)) =
            &base_atom.ty
        else {
            return Err(SchemaError::Grammar {
                property: name.to_string(),
                message: format!(
                    "`suggest` on imported type `{name}@{reference}` requires an exact `string` or `number` target"
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
                    name,
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

    if is_array {
        if base_atom.is_array {
            return Err(SchemaError::Convert {
                property: name.to_string(),
                message: format!(
                    "cannot apply `[]` to the already-array named type `{name}@{reference}`"
                ),
            });
        }
        // `Name[]@file` → array of the inlined base. Import-site `(...)` (which
        // syntactically follows `[]`) applies at the array level.
        let mut arr_constraints = array_constraints;
        arr_constraints.extend(constraints);
        Ok(PropertyDef::Single(PropertyAtom {
            ty: base_atom.ty,
            is_array: true,
            constraints: base_atom.constraints,
            array_constraints: arr_constraints,
            description: description.or(base_atom.description),
        }))
    } else {
        // `Name(constraints)@file` → constraints applied to the inlined value.
        let mut merged = base_atom.constraints;
        merged.extend(constraints);
        Ok(PropertyDef::Single(PropertyAtom {
            ty: base_atom.ty,
            is_array: base_atom.is_array,
            constraints: merged,
            array_constraints: base_atom.array_constraints,
            description: description.or(base_atom.description),
        }))
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
        TypeExpr::Primitive(_) => false,
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
/// example file paths (dependency edges).
///
/// Fast path: a schema with no `x-darkmatter-example` annotation returns an
/// empty `Vec` and leaves `json` byte-equivalent.
fn resolve_document_examples(
    json: &mut Value,
    base_dir: &Path,
    this_file: Option<&Path>,
    schema_roots: &[PathBuf],
) -> Result<Vec<PathBuf>, SchemaError> {
    let mut deps: BTreeSet<PathBuf> = BTreeSet::new();
    resolve_examples_in_json(json, base_dir, this_file, schema_roots, &mut deps)?;
    Ok(deps.into_iter().collect())
}

fn resolve_examples_in_json(
    value: &mut Value,
    base_dir: &Path,
    this_file: Option<&Path>,
    schema_roots: &[PathBuf],
    deps: &mut BTreeSet<PathBuf>,
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
                        deps,
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
                resolve_examples_in_json(child, base_dir, this_file, schema_roots, deps)?;
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                resolve_examples_in_json(item, base_dir, this_file, schema_roots, deps)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Resolves one `example(...)` reference to a validated example object.
///
/// `this` targets the current schema file (`this_file`); every other value is a
/// `biscuit-file` reference resolved relative to `base_dir` — the same
/// resolution `$schema` file references use. The referenced file is validated
/// against the example-artifact envelope with content-hash caching, then — when
/// `target` is `Some` — its `returns` value is validated against the annotated
/// property's target schema as a separate, un-cached gate.
fn resolve_one_example(
    reference: &str,
    base_dir: &Path,
    this_file: Option<&Path>,
    target: Option<&Value>,
    schema_roots: &[PathBuf],
    deps: &mut BTreeSet<PathBuf>,
) -> Result<Value, SchemaError> {
    let trimmed = reference.trim();
    let path = if trimmed == "this" {
        this_file
            .map(Path::to_path_buf)
            .ok_or_else(|| SchemaError::InvalidExample {
                reference: reference.to_string(),
                message: "`this` example reference has no target (the schema is inline, not a file)"
                    .into(),
            })?
    } else {
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return Err(SchemaError::RemoteUnsupported {
                reference: reference.to_string(),
            });
        }
        // Bare-name resolution against schema roots (Phase 3).
        if !schema_roots.is_empty() && is_bare_name(trimmed) {
            if let Some(p) = try_bare_name_in_roots(trimmed, schema_roots)? {
                p
            } else {
                let sibling = base_dir.join(trimmed);
                if sibling.is_file() {
                    return Err(SchemaError::BareNameSiblingExists {
                        reference: trimmed.to_string(),
                        suggestion: format!("./{trimmed}"),
                    });
                }
                return Err(SchemaError::Unresolved {
                    reference: reference.to_string(),
                    source: biscuit_file::FileReferenceError::InvalidSyntax(format!(
                        "bare-name reference `{trimmed}` not found in any schema root"
                    )),
                });
            }
        } else {
            let file_ref =
                FileReference::new(reference).map_err(|source| SchemaError::Unresolved {
                    reference: reference.to_string(),
                    source,
                })?;
            file_ref
                .resolve_from(base_dir)
                .map_err(|source| SchemaError::Unresolved {
                    reference: reference.to_string(),
                    source,
                })?
                .ok_or_else(|| SchemaError::Unresolved {
                    reference: reference.to_string(),
                    source: biscuit_file::FileReferenceError::InvalidSyntax(format!(
                        "no file matched `{reference}`"
                    )),
                })?
        }
    };

    let bytes = fs::read(&path).map_err(|source| SchemaError::Io {
        path: path.clone(),
        source,
    })?;
    let object = super::example::validate_example_bytes(reference, &bytes)?;
    // Layer 3: the example's `returns` must match the property it annotates.
    // Applied here (not inside the byte cache) so the same file validated
    // against different targets never shares a target verdict.
    if let Some(target) = target {
        super::example::validate_returns_against_target(reference, &object, target)?;
    }
    deps.insert(canonical_path(&path));
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
///
/// ## Errors
///
/// Returns [`SchemaError::Baseline`] when the baseline is not a simple
/// object schema (rooted at `"type": "object"` with only `properties` /
/// `required`).
pub fn merge_baseline(baseline: &Value, document: Value) -> Result<Value, SchemaError> {
    let baseline_obj = baseline.as_object().ok_or_else(|| SchemaError::Baseline {
        message: "baseline must be an object schema".into(),
        source: None,
    })?;
    validate_simple_object_schema(baseline_obj)?;

    // Root unions: merge into each arm independently.
    if let Some(arms) = document.get("anyOf").and_then(Value::as_array) {
        let mut new_arms = Vec::with_capacity(arms.len());
        for arm in arms {
            new_arms.push(merge_baseline(baseline, arm.clone())?);
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

    let baseline_props: Map<String, Value> = baseline_obj
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let baseline_required: Vec<String> = baseline_obj
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
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

    for (key, baseline_schema) in &baseline_props {
        if !doc_props.contains_key(key) {
            doc_props.insert(key.clone(), baseline_schema.clone());
            if baseline_required.contains(key) && !doc_required.contains(key) {
                doc_required.push(key.clone());
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

/// Validates that a JSON Schema is a simple object schema: rooted at
/// `"type": "object"` with only `properties` / `required` (plus the cosmetic
/// `$schema` / `additionalProperties: true` / `description` / `title`).
///
/// This is the merge-compatibility contract: both caller baselines and trigger
/// payloads must satisfy it so the property-keyed merge has a sound "later
/// wins" meaning.
pub(crate) fn validate_simple_object_schema(
    schema: &Map<String, Value>,
) -> Result<(), SchemaError> {
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

fn describe_yaml(value: &YamlValue) -> &'static str {
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
    use serde_json::json;

    fn yaml_value(input: &str) -> YamlValue {
        serde_yaml_ng::from_str(input).expect("yaml parse")
    }

    #[test]
    fn resolves_inline_mapping() {
        let v = yaml_value("title: 'string(required)'");
        let resolved = resolve_yaml_schema(&v, Path::new(".")).unwrap();
        assert!(resolved.simplified.is_some());
        assert_eq!(resolved.json_schema["type"], "object");
        let required = resolved.json_schema["required"].as_array().unwrap();
        assert_eq!(required[0], "title");
    }

    #[test]
    fn resolves_inline_root_union() {
        let v = yaml_value("- title: 'string(required)'\n- name: string");
        let resolved = resolve_yaml_schema(&v, Path::new(".")).unwrap();
        assert!(resolved.json_schema["anyOf"].is_array());
    }

    #[test]
    fn remote_https_is_rejected() {
        let v = yaml_value("'https://example.com/schema.json'");
        let err = resolve_yaml_schema(&v, Path::new(".")).unwrap_err();
        assert!(matches!(err, SchemaError::RemoteUnsupported { .. }));
    }

    #[test]
    fn remote_http_is_rejected() {
        let v = yaml_value("'http://example.com/schema.json'");
        let err = resolve_yaml_schema(&v, Path::new(".")).unwrap_err();
        assert!(matches!(err, SchemaError::RemoteUnsupported { .. }));
    }

    #[test]
    fn unsupported_shape_is_rejected() {
        let v = yaml_value("42");
        let err = resolve_yaml_schema(&v, Path::new(".")).unwrap_err();
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
        let resolved = resolve_yaml_schema(&v, dir.path()).unwrap();
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
        let resolved = resolve_yaml_schema(&v, dir.path()).unwrap();
        assert_eq!(resolved.referenced_files, vec![canonical_path(&path)]);
    }

    #[test]
    fn inline_mapping_records_no_referenced_files() {
        let v = yaml_value("title: 'string(required)'");
        let resolved = resolve_yaml_schema(&v, Path::new(".")).unwrap();
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
        let resolved = resolve_yaml_schema(&v, dir.path()).unwrap();
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
        let resolved = resolve_yaml_schema(&v, dir.path()).unwrap();
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
        let resolved = resolve_yaml_schema(&v, dir.path()).unwrap();
        assert!(resolved.simplified.is_none());
    }

    #[test]
    fn non_object_yaml_is_ambiguous() {
        let dir = tempfile::tempdir().unwrap();
        write_schema_file(dir.path(), "bad.yaml", "- one\n- two\n");
        let v = yaml_value("./bad.yaml");
        let err = resolve_yaml_schema(&v, dir.path()).unwrap_err();
        assert!(matches!(err, SchemaError::AmbiguousReferenced { .. }));
    }

    #[test]
    fn missing_file_errors_with_unresolved() {
        let dir = tempfile::tempdir().unwrap();
        let v = yaml_value("./nope.yaml");
        let err = resolve_yaml_schema(&v, dir.path()).unwrap_err();
        assert!(matches!(err, SchemaError::Unresolved { .. }));
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
        use crate::markdown::schemas::validate::build_validator;
        let baseline_yaml = "ctx:\n  today: \"date(generated; required)\"";
        let baseline_value: YamlValue = serde_yaml_ng::from_str(baseline_yaml).unwrap();
        let baseline_schema = parse_yaml_schema(&baseline_value).unwrap();
        let baseline_json = to_json_schema(&baseline_schema).unwrap();

        let doc = json!({"type": "object", "properties": {}});
        let merged = merge_baseline(&baseline_json, doc).unwrap();
        let validator = build_validator(&merged, None, None).unwrap();

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
        resolve_yaml_schema(schema_value, dir)
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
        // `value` is optional, so reach through the null wrapper to the arm
        // that carries the inlined enum.
        let value = &resolved.json_schema["properties"]["value"];
        let arm = &value["anyOf"][1];
        assert!(
            arm["enum"].is_array(),
            "`value` must inline the target's `type` enum: {value}"
        );
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
        let arm = &resolved.json_schema["properties"]["value"]["anyOf"][1];
        assert!(
            arm["enum"].is_array(),
            "`value` must inline the same-file `type` enum"
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
        assert!(
            !matches!(err, SchemaError::Grammar { .. }),
            "an undefined named type must be a resolution error, not a grammar error: {err:?}"
        );
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
        assert!(
            matches!(err, SchemaError::ImportCycle { .. }),
            "an import cycle must be a structural recursion error, not a grammar error: {err:?}"
        );
    }

    #[test]
    fn constrained_import_applies_use_site_constraint() {
        // `Name(constraints)@file` applies the import-site constraint to the
        // inlined value type. Here `type(required)@this` makes `value` required
        // even though the named `type` definition's own `required` is stripped
        // on inline.
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
        let deps = resolve_document_examples(&mut json, dir.path(), Some(&path), &[])
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
        Path::new(env!("CARGO_MANIFEST_DIR"))
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
            resolve_yaml_schema(schema_value, &dir).expect("example.yaml envelope must resolve");

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

    // ── is_bare_name detection ──────────────────────────────────────────

    #[test]
    fn bare_name_plain_filename() {
        assert!(is_bare_name("claudine.yaml"));
        assert!(is_bare_name("schema.yml"));
    }

    #[test]
    fn bare_name_rejects_path_separator() {
        assert!(!is_bare_name("./claudine.yaml"));
        assert!(!is_bare_name("docs/claudine.yaml"));
        assert!(!is_bare_name("../types.yaml"));
        assert!(!is_bare_name("/abs/schema.yaml"));
        assert!(!is_bare_name(r"docs\schema.yaml"));
    }

    #[test]
    fn bare_name_rejects_magic_prefix() {
        assert!(!is_bare_name("@schema.yaml"));
        assert!(!is_bare_name("!schema.yaml"));
        assert!(!is_bare_name("%schema.yaml"));
        assert!(!is_bare_name("vault:schema.yaml"));
        assert!(!is_bare_name("vault::schema.yaml"));
    }

    #[test]
    fn bare_name_rejects_empty() {
        assert!(!is_bare_name(""));
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
            resolve_yaml_schema_with_roots(&v, &root.join("pkg/docs"), &roots).expect("resolves");

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
            resolved.referenced_files[0]
                .to_string_lossy()
                .contains("pkg/schemas"),
            "referenced file must be from the near root"
        );
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
            resolve_yaml_schema_with_roots(&v, &root.join("pkg/docs"), &roots).expect("resolves");
        let required = resolved.json_schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "title"));
    }

    #[test]
    fn bare_name_pins_to_schema_root_not_repository_root() {
        // The precedence flip made implicit resolution repository-first. A
        // schema-root probe must stay pinned to its root: with a collision
        // between the repository root and a nearer configured schema root, the
        // schema root wins (feature review-1 Finding 4). An unpinned
        // (implicit) probe would let `<repo>/schema.yaml` shadow
        // `<repo>/schemas/schema.yaml` and silently validate the wrong schema.
        let tmp = tempfile::tempdir().unwrap();
        // Canonicalize so macOS `/var` -> `/private/var` matches the workdir
        // `gix` reports for repository discovery.
        let root = std::fs::canonicalize(tmp.path()).unwrap();
        // A real git repo so `resolve_from`'s repository discovery has a root
        // to (wrongly) prefer if the probe were not pinned.
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
            resolve_yaml_schema_with_roots(&v, &root.join("docs"), &roots).expect("resolves");

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
            resolve_yaml_schema_with_roots(&v, &root.join("docs"), &roots).expect("resolves");

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
            resolve_yaml_schema_with_roots(&v, &root.join("docs"), &roots).expect("resolves");

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
        let err = resolve_yaml_schema_with_roots(&v, &root.join("docs"), &roots).unwrap_err();

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
    fn bare_name_not_found_no_sibling_is_unresolved() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();

        let roots = vec![root.join("schemas")];
        let v = yaml_value("nonexistent.yaml");
        let err = resolve_yaml_schema_with_roots(&v, &root.join("docs"), &roots).unwrap_err();
        assert!(matches!(err, SchemaError::Unresolved { .. }));
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
        let resolved = resolve_yaml_schema(&v, root).expect("resolves via legacy path");
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

        let resolved = resolve_yaml_schema_with_roots(schema_value, &root.join("docs"), &roots)
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

        let err = resolve_yaml_schema_with_roots(schema_value, &root.join("docs"), &roots)
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

        let resolved = resolve_yaml_schema_with_roots(schema_value, &root.join("docs"), &roots)
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

        let err = resolve_yaml_schema_with_roots(schema_value, &root.join("docs"), &roots)
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
        let r1 = resolve_yaml_schema_with_roots(&v, &root.join("docs"), &roots).unwrap();
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
        let r2 = resolve_yaml_schema_with_roots(schema_value, &root.join("docs"), &roots).unwrap();
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

    /// Suppress unused import warning for PathBuf when tests compile.
    #[test]
    fn _pathbuf_used() {
        let _ = PathBuf::new();
    }
}
