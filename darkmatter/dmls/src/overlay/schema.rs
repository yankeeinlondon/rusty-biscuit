//! Effective-schema assembly for the frontmatter overlay.
//!
//! Reproduces the compose precedence (design "Extension baselines"): the
//! Darkmatter base baseline by default, extension baselines from
//! [`DmlsConfig`] whose activation globs match the document merged over it
//! (Claudine is pure data — a `claudine.yaml` path plus `.claude/**` globs,
//! zero Claudine-specific code), and the document's own `$schema` on top. The
//! library ([`darkmatter::markdown::schemas`]) remains the semantic authority;
//! this module only decides *which* baselines apply and hands the layered
//! result to [`DarkmatterSchemas::effective_for`].

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::compose::find_git_root_from;
use darkmatter::markdown::schemas::resolve::{merge_baseline, resolve_schema};
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, SchemaError, SchemaShape, SimplifiedSchema,
    darkmatter_base_json_schema, triggers::TriggerRegistry,
};
use globset::{Glob, GlobSetBuilder};
use serde_json::Value;

use crate::config::{DmlsConfig, SchemaExtensionConfig};

/// The assembled schema for one document: the effective schema plus the
/// frontmatter JSON it validates (both derived from the same parsed document,
/// so diagnostics never re-parse).
#[derive(Clone)]
pub struct SchemaBundle {
    /// The layered effective schema (base + extensions + document `$schema`).
    pub effective: EffectiveSchema,
    /// The document's frontmatter as JSON, with the `$schema` control key
    /// stripped (it is not document data).
    pub frontmatter_json: Value,
    /// The SimplifiedSchema shapes of the matched extension baselines.
    ///
    /// `effective.simplified` carries only the document's own `$schema`; the
    /// extension shapes are kept separately so completion/hover can offer
    /// extension-declared keys (e.g. Claudine's `provider`/`model`) even when
    /// the document has no `$schema` of its own.
    pub extension_shapes: Vec<SchemaShape>,
    /// Extension-baseline dependency files (each matched extension's own path
    /// plus its imports/examples), sorted and deduplicated. These are NOT on
    /// `effective.dependencies()` because extension baselines are merged into the
    /// baseline JSON schema *before* the effective schema is assembled, so their
    /// edges never reach [`EffectiveSchema::dependencies`]. The overlay cache
    /// content-hashes them alongside the effective dependencies so editing a
    /// configured extension baseline (or a type it imports) invalidates the
    /// cached bundle.
    pub extension_dependencies: Vec<PathBuf>,
}

/// Assembles the effective schema for a document.
///
/// ## Returns
///
/// `Ok(Some(bundle))` for every document with frontmatter (the base baseline
/// always applies); `Ok(None)` only when the document has no frontmatter map.
///
/// ## Errors
///
/// Propagates [`SchemaError`] from extension loading, baseline merging, `$schema`
/// resolution, or validator construction so the caller can range a
/// `dm.schema.prepare` / `dm.schema.invalid_schema_shape` diagnostic.
pub fn assemble(
    doc_path: &Path,
    document_text: &str,
    config: &DmlsConfig,
    workspace_roots: &[PathBuf],
    trigger_registry: Option<TriggerRegistry>,
) -> Result<Option<SchemaBundle>, SchemaError> {
    let combined = combined_baseline(doc_path, config, workspace_roots)?;

    let md: Markdown = document_text.into();
    let md = md.with_source(ComposeSource::File(doc_path.to_path_buf()));
    if md.frontmatter().as_map().is_empty() && md.frontmatter().raw_source().is_none() {
        return Ok(None);
    }

    let mut schemas = DarkmatterSchemas::new().with_baseline_json_schema(combined.baseline)?;
    if let Some(registry) = trigger_registry {
        schemas = schemas.with_trigger_registry(registry);
    }
    if let Some(dir) = doc_path.parent() {
        schemas = schemas.with_file_ref_fallback_dir(dir.to_path_buf());
    }

    match schemas.effective_for(&md)? {
        Some(effective) => Ok(Some(SchemaBundle {
            effective,
            frontmatter_json: frontmatter_json(&md),
            extension_shapes: combined.extension_shapes,
            extension_dependencies: combined.dependencies,
        })),
        None => Ok(None),
    }
}

/// Selects the trigger-discovery boundary for a document.
///
/// The nearest containing workspace folder is authoritative. When the document
/// is inside a Git repository whose root is at or below that folder, the
/// repository root narrows the boundary. Documents outside all workspace
/// folders intentionally return `None` and never discover triggers.
pub fn trigger_boundary(doc_path: &Path, workspace_roots: &[PathBuf]) -> Option<PathBuf> {
    let workspace = workspace_roots
        .iter()
        .filter(|root| doc_path.starts_with(root))
        .max_by_key(|root| root.components().count())?;
    let start = doc_path.parent().unwrap_or(doc_path);
    match find_git_root_from(start) {
        Some(repo) if repo.starts_with(workspace) => Some(repo),
        _ => Some(workspace.clone()),
    }
}

/// The Darkmatter base baseline with matching extension baselines merged over
/// it, plus the matched extensions' SimplifiedSchema shapes and dependency
/// files.
struct CombinedBaseline {
    baseline: Value,
    extension_shapes: Vec<SchemaShape>,
    dependencies: Vec<PathBuf>,
}

/// The Darkmatter base baseline with every matching extension baseline merged
/// over it (extension wins over base; a later extension wins over an earlier),
/// alongside the matched extensions' SimplifiedSchema shapes.
fn combined_baseline(
    doc_path: &Path,
    config: &DmlsConfig,
    workspace_roots: &[PathBuf],
) -> Result<CombinedBaseline, SchemaError> {
    let mut baseline = darkmatter_base_json_schema();
    let mut shapes = Vec::new();
    let mut dependencies: BTreeSet<PathBuf> = BTreeSet::new();
    for extension in config.schema.extensions.values() {
        if !extension_matches(doc_path, extension, workspace_roots) {
            continue;
        }
        let resolved = load_extension_schema(extension, workspace_roots)?;
        if let Some(SimplifiedSchema::Single(shape)) = &resolved.simplified {
            shapes.push(shape.clone());
        }
        // The extension file's own path (via `referenced_files`, since
        // `load_extension_schema` resolves a `Value::String(path)`) plus the
        // types it imports and examples it references are dependency edges the
        // overlay cache must hash — they never reach `effective.dependencies()`
        // because the extension is merged into the baseline before assembly.
        dependencies.extend(resolved.referenced_files.iter().cloned());
        dependencies.extend(resolved.imports.iter().cloned());
        dependencies.extend(resolved.examples.iter().cloned());
        // `merge_baseline(under, over)` lets `over` win — the extension
        // overrides the base, matching compose's layering.
        baseline = merge_baseline(&baseline, resolved.json_schema)?;
    }
    Ok(CombinedBaseline {
        baseline,
        extension_shapes: shapes,
        dependencies: dependencies.into_iter().collect(),
    })
}

/// Loads one extension's SimplifiedSchema (or JSON Schema) file.
fn load_extension_schema(
    extension: &SchemaExtensionConfig,
    workspace_roots: &[PathBuf],
) -> Result<darkmatter::markdown::schemas::resolve::ResolvedSchema, SchemaError> {
    let path = resolve_extension_path(&extension.path, workspace_roots);
    let base_dir = path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    // Drive resolution through the same file-reference path a `$schema` string
    // uses, so YAML/JSON disambiguation matches document references.
    let value = Value::String(path.to_string_lossy().into_owned());
    resolve_schema(&value, &base_dir)
}

/// Resolves an extension's configured path: relative paths anchor on the first
/// workspace root.
fn resolve_extension_path(path: &Path, workspace_roots: &[PathBuf]) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    workspace_roots
        .first()
        .map(|root| root.join(path))
        .unwrap_or_else(|| path.to_path_buf())
}

/// Whether `doc_path` matches an extension's activation globs. An extension
/// with no globs never auto-activates.
fn extension_matches(
    doc_path: &Path,
    extension: &SchemaExtensionConfig,
    workspace_roots: &[PathBuf],
) -> bool {
    if extension.globs.is_empty() {
        return false;
    }
    let relative = relative_to_root(doc_path, workspace_roots);
    let mut builder = GlobSetBuilder::new();
    for pattern in &extension.globs {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    match builder.build() {
        Ok(set) => set.is_match(&relative),
        Err(_) => false,
    }
}

/// The document path relative to its first ancestor workspace root (else the
/// path itself), for glob matching.
fn relative_to_root(doc_path: &Path, workspace_roots: &[PathBuf]) -> PathBuf {
    for root in workspace_roots {
        if let Ok(relative) = doc_path.strip_prefix(root) {
            return relative.to_path_buf();
        }
    }
    doc_path.to_path_buf()
}

/// The document frontmatter as JSON, with the `$schema` control key removed.
fn frontmatter_json(md: &Markdown) -> Value {
    let map = md.frontmatter().as_map();
    let mut object = serde_json::Map::with_capacity(map.len());
    for (key, value) in map {
        if key == "$schema" {
            continue;
        }
        object.insert(key.clone(), value.clone());
    }
    Value::Object(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_extension(globs: &[&str], path: &str) -> DmlsConfig {
        let mut config = DmlsConfig::default();
        config.schema.extensions.insert(
            "claudine".to_string(),
            SchemaExtensionConfig {
                path: PathBuf::from(path),
                globs: globs.iter().map(|g| g.to_string()).collect(),
            },
        );
        config
    }

    #[test]
    fn test_base_baseline_applies_without_schema() {
        let bundle = assemble(
            Path::new("/w/doc.md"),
            "---\ntitle: Hello\n---\n\nbody\n",
            &DmlsConfig::default(),
            &[PathBuf::from("/w")],
            None,
        )
        .expect("assembles")
        .expect("has frontmatter");
        // A plain title validates cleanly against the base baseline (which
        // allows additional properties).
        let report = bundle.effective.validate(&bundle.frontmatter_json);
        assert!(report.valid, "unexpected problems: {:?}", report.problems);
    }

    #[test]
    fn test_no_frontmatter_yields_none() {
        let bundle = assemble(
            Path::new("/w/doc.md"),
            "# Just a body\n",
            &DmlsConfig::default(),
            &[PathBuf::from("/w")],
            None,
        )
        .expect("assembles");
        assert!(bundle.is_none());
    }

    #[test]
    fn test_extension_glob_gates_activation() {
        // The extension path does not exist, so activation must be attempted
        // only when the glob matches (a non-matching doc never loads it).
        let config = config_with_extension(&[".claude/**"], "missing-schema.yaml");
        let outside = assemble(
            Path::new("/w/notes/x.md"),
            "---\ntitle: A\n---\n\nbody\n",
            &config,
            &[PathBuf::from("/w")],
            None,
        );
        // Outside `.claude/**`: extension never loaded, assembly succeeds.
        assert!(outside.is_ok());
    }

    #[test]
    fn test_relative_to_root() {
        assert_eq!(
            relative_to_root(Path::new("/w/.claude/p.md"), &[PathBuf::from("/w")]),
            PathBuf::from(".claude/p.md")
        );
        assert_eq!(
            relative_to_root(Path::new("/other/p.md"), &[PathBuf::from("/w")]),
            PathBuf::from("/other/p.md")
        );
    }

    #[test]
    fn test_extension_matches_respects_globs() {
        let extension = SchemaExtensionConfig {
            path: PathBuf::from("claudine.yaml"),
            globs: vec![".claude/**".to_string()],
        };
        let roots = [PathBuf::from("/w")];
        assert!(extension_matches(Path::new("/w/.claude/p.md"), &extension, &roots));
        assert!(!extension_matches(Path::new("/w/notes/p.md"), &extension, &roots));
        let no_globs = SchemaExtensionConfig {
            path: PathBuf::from("x.yaml"),
            globs: Vec::new(),
        };
        assert!(!extension_matches(Path::new("/w/.claude/p.md"), &no_globs, &roots));
    }
}
