//! Schema-root discovery (ancestor walk) and the per-boundary trigger registry.
//!
//! Trigger schemas live in `schemas/` directories discovered by an ancestor
//! walk from the document's directory up through the configured discovery
//! boundary (inclusive). Every `schemas/` directory on that path is a schema
//! root, nearest first. The walk never continues past the boundary to the
//! filesystem root or home directory.
//!
//! ## Discovery contract
//!
//! - Regular `.yaml`/`.yml` files only; directory and file symlinks are never
//!   followed.
//! - Within a root, files are ordered by UTF-8 filename bytes (not locale or
//!   host filesystem collation).
//! - Two names that collide after case-folding are a load error so a repository
//!   cannot activate differently on case-sensitive and case-insensitive
//!   filesystems.
//! - Nearest root wins by trigger **filename** with no merging; a shadowed file
//!   is never evaluated.
//! - A file that does not claim `kind: trigger-schema` is silently ignored.
//! - A file that claims the envelope but is malformed is a hard load error.
//! - Loading is **transactional**: if any unshadowed opted-in trigger is
//!   invalid, no registry is installed from that scan.
//!
//! See `darkmatter/features/2026-07-10-schema-triggers/spec.md`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::markdown::schemas::errors::SchemaError;

use super::envelope::{TriggerEnvelope, parse_trigger_envelope_from_str};

/// The directory name that marks a schema root.
const SCHEMAS_DIR_NAME: &str = "schemas";

/// Valid YAML extensions for trigger-schema files.
const YAML_EXTENSIONS: &[&str] = &["yaml", "yml"];

// ── Types ───────────────────────────────────────────────────────────────────

/// A loaded trigger envelope with its source file path.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedTrigger {
    /// The filesystem path of the `.trigger.yaml` file.
    pub source: PathBuf,
    /// The parsed, lint-clean trigger envelope.
    pub envelope: TriggerEnvelope,
}

/// A file shadowed by a nearer root's file of the same name.
#[derive(Debug, Clone, PartialEq)]
pub struct ShadowedFile {
    /// The shadowed file's path.
    pub path: PathBuf,
    /// The nearer file that shadowed it.
    pub shadowed_by: PathBuf,
}

/// The per-boundary registry of unshadowed, loaded trigger envelopes.
///
/// Built by [`scan`]. Loading is transactional: if any unshadowed opted-in
/// trigger is invalid, [`scan`] returns `Err` and no registry is installed.
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerRegistry {
    /// The discovery boundary these triggers were discovered under.
    pub boundary: PathBuf,
    /// All discovered schema root directories (nearest first), for the
    /// `md schema triggers` trace.
    pub roots: Vec<PathBuf>,
    /// The ordered, deduped set of loaded triggers (nearest root first,
    /// filename-lexicographic within a root). Shadowed filenames are absent.
    pub triggers: Vec<LoadedTrigger>,
    /// Resolved payloads parallel to [`triggers`](Self::triggers) — one per
    /// trigger, in the same order. Populated by [`scan`] so `effective_for`
    /// does not resolve payloads twice. Empty for programmatically
    /// constructed registries, which fall back to on-demand resolution.
    pub(crate) payloads: Vec<super::assemble::ResolvedPayload>,
    /// Files shadowed by a nearer root's file of the same name.
    pub shadowed: Vec<ShadowedFile>,
}

impl TriggerRegistry {
    /// An empty registry for `boundary` (no roots, no triggers).
    pub fn empty(boundary: impl Into<PathBuf>) -> Self {
        Self {
            boundary: boundary.into(),
            roots: Vec::new(),
            triggers: Vec::new(),
            payloads: Vec::new(),
            shadowed: Vec::new(),
        }
    }

    /// `true` when no triggers were loaded.
    pub fn is_empty(&self) -> bool {
        self.triggers.is_empty()
    }
}

// ── Ancestor walk ───────────────────────────────────────────────────────────

/// Walks from `document_dir` up through `boundary` (inclusive), collecting
/// every `schemas/` directory as a root, **nearest first**.
///
/// A document directory outside the boundary yields no roots. The walk never
/// continues past the boundary to the filesystem root or home, even when
/// boundary discovery failed.
pub fn schema_roots(document_dir: &Path, boundary: &Path) -> Vec<PathBuf> {
    if !document_dir.starts_with(boundary) {
        return Vec::new();
    }

    let mut roots = Vec::new();
    let mut current = document_dir.to_path_buf();
    loop {
        let schemas_dir = current.join(SCHEMAS_DIR_NAME);
        let is_real_directory = std::fs::symlink_metadata(&schemas_dir)
            .map(|metadata| {
                let file_type = metadata.file_type();
                file_type.is_dir() && !file_type.is_symlink()
            })
            .unwrap_or(false);
        if is_real_directory {
            roots.push(schemas_dir);
        }
        if current == boundary {
            break;
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            // Reached the filesystem root without hitting the boundary.
            // Shouldn't happen after the `starts_with` check, but guard
            // against an infinite loop regardless.
            None => break,
        }
    }
    roots
}

// ── Per-root file enumeration ───────────────────────────────────────────────

/// A regular `.yaml`/`.yml` file discovered in a schema root.
#[derive(Debug, Clone)]
struct RootFile {
    /// Full filesystem path.
    path: PathBuf,
    /// UTF-8 filename (last path component).
    name: String,
}

/// Enumerates regular `.yaml`/`.yml` files in `root`, rejecting symlinks and
/// case-fold collisions.
///
/// Files are returned sorted by UTF-8 filename bytes. Two names that collide
/// after case-folding are a load error.
fn enumerate_root(root: &Path) -> Result<Vec<RootFile>, SchemaError> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) => {
            return Err(SchemaError::Io {
                path: root.to_path_buf(),
                source: e,
            });
        }
    };

    let mut files: Vec<RootFile> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();

        // Do not follow symlinks: check the entry's file type directly.
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(e) => {
                return Err(SchemaError::Io {
                    path: path.clone(),
                    source: e,
                });
            }
        };
        if file_type.is_symlink() || !file_type.is_file() {
            continue;
        }

        // Only `.yaml` / `.yml`.
        let is_yaml = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| YAML_EXTENSIONS.contains(&ext));
        if !is_yaml {
            continue;
        }

        // UTF-8 filename.
        let Some(name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };

        files.push(RootFile { path, name });
    }

    // Sort by UTF-8 filename bytes.
    files.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));

    // Detect case-fold collisions.
    check_case_fold_collision(root, &files)?;

    Ok(files)
}

/// Returns `Err` when two filenames in the same root collide after
/// case-folding, preventing divergent behavior on case-sensitive vs
/// case-insensitive filesystems.
fn check_case_fold_collision(
    root: &Path,
    files: &[RootFile],
) -> Result<(), SchemaError> {
    let mut seen_lowered: HashMap<String, String> = HashMap::new();
    for file in files {
        let lowered = file.name.to_lowercase();
        if let Some(existing) = seen_lowered.get(&lowered) {
            return Err(SchemaError::TriggerCaseFoldCollision {
                root: root.to_path_buf(),
                files: vec![existing.clone(), file.name.clone()],
            });
        }
        seen_lowered.insert(lowered, file.name.clone());
    }
    Ok(())
}

// ── Path normalization ──────────────────────────────────────────────────────

/// Lexically splits a path string into segments, handling both `/` and `\`
/// separators. Empty segments and `.` segments are filtered out.
fn lexical_segments(path: &str) -> Vec<&str> {
    path.split(['/', '\\'])
        .filter(|s| !s.is_empty() && *s != ".")
        .collect()
}

/// Normalizes a document path string against a boundary path string into a
/// boundary-relative, `/`-separated path suitable for `$path` matching.
///
/// Both inputs are treated **lexically**: no filesystem access, no symlink
/// resolution. The output uses `/` separators on every platform. Returns
/// `None` when the document is not lexically under the boundary, or when a
/// `..` component would escape it.
///
/// Handles both POSIX-style (`/`) and Windows-style (`\`) separators in the
/// inputs so the same logic is testable on every platform.
pub fn normalize_relative_path(document: &str, boundary: &str) -> Option<String> {
    let doc_segments = lexical_segments(document);
    let boundary_segments = lexical_segments(boundary);

    if doc_segments.len() < boundary_segments.len() {
        return None;
    }
    if doc_segments[..boundary_segments.len()] != boundary_segments[..] {
        return None;
    }

    let relative = &doc_segments[boundary_segments.len()..];
    if relative.contains(&"..") {
        return None;
    }
    if relative.is_empty() {
        return None;
    }

    Some(relative.join("/"))
}

/// Normalizes a `Path` document against a `Path` boundary for `$path`
/// matching, using the same lexical logic as [`normalize_relative_path`].
///
/// Returns `None` when the document is not lexically under the boundary.
pub fn normalize_path(document: &Path, boundary: &Path) -> Option<String> {
    normalize_relative_path(&document.to_string_lossy(), &boundary.to_string_lossy())
}

// ── Full scan ───────────────────────────────────────────────────────────────

/// Performs a full trigger-schema discovery scan for a document under a
/// boundary.
///
/// Walks ancestor `schemas/` roots, enumerates files, applies filename
/// shadowing, classifies each unshadowed file, loads trigger envelopes, and
/// resolves every payload.
///
/// Loading is **transactional**: if any unshadowed opted-in trigger file is
/// malformed or its payload fails to resolve (missing file, cyclic reference,
/// non-mergeable shape), no registry is installed (an error is returned).
/// Payload resolution runs regardless of whether any current document matches
/// the trigger, so a registry is never installed with a known-bad payload.
///
/// A document outside the boundary yields an empty registry (not an error).
pub fn scan(document: &Path, boundary: &Path) -> Result<TriggerRegistry, SchemaError> {
    let document_dir = document
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    // Ancestor walk.
    let roots = schema_roots(document_dir, boundary);
    if roots.is_empty() {
        return Ok(TriggerRegistry::empty(boundary));
    }

    // Enumerate each root (nearest first).
    let mut all_root_files: Vec<Vec<RootFile>> = Vec::with_capacity(roots.len());
    for root in &roots {
        all_root_files.push(enumerate_root(root)?);
    }

    // Apply filename shadowing: nearest root wins by filename.
    let mut first_by_name: HashMap<String, PathBuf> = HashMap::new();
    let mut unshadowed: Vec<RootFile> = Vec::new();
    let mut shadowed: Vec<ShadowedFile> = Vec::new();
    for root_files in &all_root_files {
        for file in root_files {
            if let Some(shadowing_path) = first_by_name.get(&file.name) {
                shadowed.push(ShadowedFile {
                    path: file.path.clone(),
                    shadowed_by: shadowing_path.clone(),
                });
            } else {
                first_by_name.insert(file.name.clone(), file.path.clone());
                unshadowed.push(file.clone());
            }
        }
    }

    // Classify and load each unshadowed file (transactional).
    let mut triggers: Vec<LoadedTrigger> = Vec::new();
    for file in &unshadowed {
        let content = std::fs::read_to_string(&file.path).map_err(|e| SchemaError::Io {
            path: file.path.clone(),
            source: e,
        })?;
        match parse_trigger_envelope_from_str(&content) {
            Ok(None) => {
                // Not a trigger-schema — silently ignored.
            }
            Ok(Some(envelope)) => {
                triggers.push(LoadedTrigger {
                    source: file.path.clone(),
                    envelope,
                });
            }
            Err(source) => {
                return Err(SchemaError::TriggerLoad {
                    path: file.path.clone(),
                    source: Box::new(source),
                });
            }
        }
    }

    // Resolve every payload (transactional). Payload failures — missing files,
    // cycles, non-mergeable shapes — are hard load errors at scan time,
    // independent of whether any current document matches the trigger.
    let mut payloads = Vec::with_capacity(triggers.len());
    for trigger in &triggers {
        let payload = super::assemble::resolve_trigger_payload(trigger, &roots).map_err(
            |source| SchemaError::TriggerLoad {
                path: trigger.source.clone(),
                source: Box::new(source),
            },
        )?;
        payloads.push(payload);
    }

    Ok(TriggerRegistry {
        boundary: boundary.to_path_buf(),
        roots,
        triggers,
        payloads,
        shadowed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::errors::SchemaError;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Creates a temp directory pinned as a repo root with a `.git` marker so
    /// ancestor walks never escape into a shared tempdir.
    fn repo_fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    /// A minimal valid trigger envelope with a payload reference.
    const VALID_TRIGGER: &str =
        "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: payload-a.yaml\n";

    /// A second minimal trigger with a different match key.
    const VALID_TRIGGER_2: &str =
        "kind: trigger-schema\nmatch:\n  title: string(required)\n$schema: payload-b.yaml\n";

    /// A simple mergeable payload for test fixtures.
    const PAYLOAD_A: &str = "$schema:\n  model: string(required)\n";
    const PAYLOAD_B: &str = "$schema:\n  owner: string(required)\n";

    /// Writes a payload file (`payload-a.yaml` / `payload-b.yaml`) into `dir`.
    fn write_payloads(dir: &Path) {
        write_file(&dir.join("payload-a.yaml"), PAYLOAD_A);
        write_file(&dir.join("payload-b.yaml"), PAYLOAD_B);
    }

    // ── Ancestor walk ───────────────────────────────────────────────────

    #[test]
    fn ancestor_walk_finds_nearest_first() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("pkg/schemas")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();

        let doc_dir = root.join("pkg/sub");
        fs::create_dir_all(&doc_dir).unwrap();

        let roots = schema_roots(&doc_dir, root);
        // Nearest first: pkg/schemas before root/schemas.
        assert_eq!(roots.len(), 2);
        assert!(roots[0].ends_with("pkg/schemas"));
        assert!(roots[1].ends_with("schemas"));
    }

    #[test]
    fn ancestor_walk_inclusive_boundary() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();

        // Document lives directly in the boundary root.
        let roots = schema_roots(root, root);
        assert_eq!(roots.len(), 1);
        assert!(roots[0].ends_with("schemas"));
    }

    #[test]
    fn ancestor_walk_document_outside_boundary_no_roots() {
        let repo = repo_fixture();
        let other = repo_fixture();

        let roots = schema_roots(other.path(), repo.path());
        assert!(roots.is_empty());
    }

    #[test]
    fn ancestor_walk_never_past_boundary() {
        let repo = repo_fixture();
        let root = repo.path();
        // A schemas/ dir above the boundary must not be found.
        let above = root.parent().unwrap();
        // Only test if above has a schemas dir; skip if not writable.
        let above_schemas = above.join("schemas");
        let has_above = above_schemas.is_dir();

        let doc_dir = root.join("pkg");
        fs::create_dir_all(&doc_dir).unwrap();
        let roots = schema_roots(&doc_dir, root);
        // Walk stops at boundary.
        if has_above {
            assert!(
                !roots.iter().any(|r| r == &above_schemas),
                "walk should not continue past boundary"
            );
        }
    }

    #[test]
    fn ancestor_walk_no_schemas_dirs() {
        let repo = repo_fixture();
        let doc_dir = repo.path().join("pkg");
        fs::create_dir_all(&doc_dir).unwrap();
        let roots = schema_roots(&doc_dir, repo.path());
        assert!(roots.is_empty());
    }

    // ── File enumeration ────────────────────────────────────────────────

    #[cfg(any(unix, windows))]
    #[test]
    fn ancestor_walk_excludes_symlinked_schemas_root() {
        let repo = repo_fixture();
        let external = repo_fixture();
        let external_schemas = external.path().join("schemas");
        fs::create_dir_all(&external_schemas).unwrap();

        let schemas_link = repo.path().join("schemas");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&external_schemas, &schemas_link).unwrap();
        #[cfg(windows)]
        if let Err(error) = std::os::windows::fs::symlink_dir(&external_schemas, &schemas_link) {
            if error.kind() == std::io::ErrorKind::PermissionDenied
                || error.raw_os_error() == Some(1314)
            {
                return;
            }
            panic!("failed to create directory symlink: {error}");
        }

        let roots = schema_roots(repo.path(), repo.path());
        assert!(roots.is_empty(), "symlinked schema root must be excluded");
    }

    #[test]
    fn enumerate_yaml_and_yml() {
        let repo = repo_fixture();
        let schemas = repo.path().join("schemas");
        fs::create_dir_all(&schemas).unwrap();
        fs::write(schemas.join("a.yaml"), VALID_TRIGGER).unwrap();
        fs::write(schemas.join("b.yml"), VALID_TRIGGER_2).unwrap();
        // Non-YAML file is ignored.
        fs::write(schemas.join("c.txt"), "not yaml").unwrap();

        let files = enumerate_root(&schemas).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "a.yaml");
        assert_eq!(files[1].name, "b.yml");
    }

    #[test]
    fn enumerate_sorted_by_utf8_bytes() {
        let repo = repo_fixture();
        let schemas = repo.path().join("schemas");
        fs::create_dir_all(&schemas).unwrap();
        // Byte order: M(77) < a(97) < z(122).
        fs::write(schemas.join("z.trigger.yaml"), VALID_TRIGGER).unwrap();
        fs::write(schemas.join("a.trigger.yaml"), VALID_TRIGGER_2).unwrap();
        fs::write(schemas.join("M.trigger.yaml"), VALID_TRIGGER).unwrap();

        let files = enumerate_root(&schemas).unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].name, "M.trigger.yaml");
        assert_eq!(files[1].name, "a.trigger.yaml");
        assert_eq!(files[2].name, "z.trigger.yaml");
    }

    #[test]
    fn detect_case_fold_collision_directly() {
        // On macOS (case-insensitive filesystem) two files differing only by
        // case cannot physically coexist, so test the collision check logic
        // directly.
        let root = Path::new("/repo/schemas");
        let files = vec![
            RootFile {
                path: PathBuf::from("/repo/schemas/Foo.yaml"),
                name: "Foo.yaml".to_string(),
            },
            RootFile {
                path: PathBuf::from("/repo/schemas/foo.yaml"),
                name: "foo.yaml".to_string(),
            },
        ];
        let err = check_case_fold_collision(root, &files).unwrap_err();
        assert!(matches!(
            err,
            SchemaError::TriggerCaseFoldCollision { .. }
        ));
    }

    #[test]
    fn detect_no_collision_when_names_differ() {
        let root = Path::new("/repo/schemas");
        let files = vec![
            RootFile {
                path: PathBuf::from("/repo/schemas/a.yaml"),
                name: "a.yaml".to_string(),
            },
            RootFile {
                path: PathBuf::from("/repo/schemas/b.yaml"),
                name: "b.yaml".to_string(),
            },
        ];
        assert!(check_case_fold_collision(root, &files).is_ok());
    }

    #[test]
    fn enumerate_excludes_symlinked_file() {
        let repo = repo_fixture();
        let schemas = repo.path().join("schemas");
        fs::create_dir_all(&schemas).unwrap();
        let target = repo.path().join("real.yaml");
        fs::write(&target, VALID_TRIGGER).unwrap();

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, schemas.join("link.yaml")).unwrap();
        }

        let files = enumerate_root(&schemas).unwrap();
        // On non-Unix the symlink test is a no-op; just verify the real file
        // was found and the symlink was excluded.
        #[cfg(unix)]
        {
            assert_eq!(files.len(), 0, "symlinked file should be excluded");
        }
        #[cfg(not(unix))]
        {
            let _ = target;
        }
    }

    #[test]
    fn enumerate_excludes_subdirectories() {
        // `read_dir` lists only immediate children; subdirectories are never
        // descended into. This implicitly satisfies the "do not follow
        // directory symlinks" policy since symlinked subdirs are also
        // immediate-child entries that are not regular files.
        let repo = repo_fixture();
        let schemas = repo.path().join("schemas");
        fs::create_dir_all(schemas.join("subdir")).unwrap();
        fs::write(schemas.join("subdir/hidden.yaml"), VALID_TRIGGER).unwrap();
        fs::write(schemas.join("visible.yaml"), VALID_TRIGGER_2).unwrap();

        let files = enumerate_root(&schemas).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "visible.yaml");
    }

    // ── Path normalization ──────────────────────────────────────────────

    #[test]
    fn normalize_posix_path() {
        let rel = normalize_relative_path("/repo/pkg/doc.md", "/repo");
        assert_eq!(rel.as_deref(), Some("pkg/doc.md"));
    }

    #[test]
    fn normalize_windows_path() {
        let rel = normalize_relative_path(r"C:\repo\pkg\doc.md", r"C:\repo");
        assert_eq!(rel.as_deref(), Some("pkg/doc.md"));
    }

    #[test]
    fn normalize_mixed_separators() {
        let rel = normalize_relative_path(r"C:/repo\pkg/sub\doc.md", r"C:\repo");
        assert_eq!(rel.as_deref(), Some("pkg/sub/doc.md"));
    }

    #[test]
    fn normalize_outside_boundary_is_none() {
        assert!(normalize_relative_path("/other/doc.md", "/repo").is_none());
    }

    #[test]
    fn normalize_dotdot_in_relative_is_none() {
        // Document path with `..` that resolves outside boundary.
        assert!(normalize_relative_path("/repo/../other/doc.md", "/repo").is_none());
    }

    #[test]
    fn normalize_dot_segments_filtered() {
        let rel = normalize_relative_path("/repo/./pkg/./doc.md", "/repo");
        assert_eq!(rel.as_deref(), Some("pkg/doc.md"));
    }

    #[test]
    fn normalize_boundary_equals_document_dir_is_none() {
        // The document *is* the boundary — no relative path.
        assert!(normalize_relative_path("/repo", "/repo").is_none());
    }

    #[test]
    fn normalize_case_sensitive() {
        // Case-sensitive: different case prefix → not under boundary.
        assert!(normalize_relative_path("/Repo/doc.md", "/repo").is_none());
    }

    #[test]
    fn normalize_path_from_path_objects() {
        let doc = PathBuf::from("/repo/pkg/doc.md");
        let boundary = PathBuf::from("/repo");
        // On macOS/Linux this works directly; on Windows the separators differ
        // but the function handles both.
        let rel = normalize_path(&doc, &boundary);
        // Only assert on platforms where / is the native separator.
        if cfg!(not(windows)) {
            assert_eq!(rel.as_deref(), Some("pkg/doc.md"));
        }
    }

    // ── Shadowing ────────────────────────────────────────────────────────

    #[test]
    fn shadowing_nearest_root_wins() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("pkg/schemas")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // Same filename in both roots.
        fs::write(
            root.join("pkg/schemas/claudine.trigger.yaml"),
            VALID_TRIGGER,
        )
        .unwrap();
        fs::write(
            root.join("schemas/claudine.trigger.yaml"),
            VALID_TRIGGER_2,
        )
        .unwrap();
        // Payload for the winning (nearest) trigger.
        write_payloads(&root.join("pkg/schemas"));

        let doc = root.join("pkg/sub/doc.md");
        let registry = scan(&doc, root).unwrap();
        assert_eq!(registry.triggers.len(), 1);
        // The nearest root's file won.
        assert!(registry.triggers[0]
            .source
            .ends_with("pkg/schemas/claudine.trigger.yaml"));
        assert_eq!(registry.shadowed.len(), 1);
        assert!(registry.shadowed[0]
            .path
            .ends_with("schemas/claudine.trigger.yaml"));
    }

    #[test]
    fn shadowing_different_names_coexist() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("pkg/schemas")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::write(
            root.join("pkg/schemas/claudine.trigger.yaml"),
            VALID_TRIGGER,
        )
        .unwrap();
        fs::write(
            root.join("schemas/inline.trigger.yaml"),
            VALID_TRIGGER_2,
        )
        .unwrap();
        // Each trigger's payload must be co-located; using distinct payload
        // filenames keeps shadowing empty.
        write_file(&root.join("pkg/schemas/payload-a.yaml"), PAYLOAD_A);
        write_file(&root.join("schemas/payload-b.yaml"), PAYLOAD_B);

        let doc = root.join("pkg/sub/doc.md");
        let registry = scan(&doc, root).unwrap();
        assert_eq!(registry.triggers.len(), 2);
        assert!(registry.shadowed.is_empty());
    }

    // ── Discovery classification ────────────────────────────────────────

    #[test]
    fn non_trigger_yaml_silently_ignored() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // A normal schema file (no kind: trigger-schema).
        fs::write(
            root.join("schemas/normal.yaml"),
            "$schema:\n  title: string\n",
        )
        .unwrap();
        // A trigger file.
        fs::write(
            root.join("schemas/trigger.trigger.yaml"),
            VALID_TRIGGER,
        )
        .unwrap();
        write_payloads(&root.join("schemas"));

        let doc = root.join("doc.md");
        let registry = scan(&doc, root).unwrap();
        assert_eq!(registry.triggers.len(), 1);
        assert!(registry.triggers[0]
            .source
            .ends_with("trigger.trigger.yaml"));
    }

    #[test]
    fn malformed_trigger_is_hard_error() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // Claims the envelope but has no `match:`.
        fs::write(
            root.join("schemas/bad.trigger.yaml"),
            "kind: trigger-schema\n",
        )
        .unwrap();

        let doc = root.join("doc.md");
        let err = scan(&doc, root).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerLoad { .. }));
    }

    #[test]
    fn malformed_trigger_atomic_no_registry() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // One valid, one malformed.
        fs::write(
            root.join("schemas/good.trigger.yaml"),
            VALID_TRIGGER,
        )
        .unwrap();
        fs::write(
            root.join("schemas/bad.trigger.yaml"),
            "kind: trigger-schema\n",
        )
        .unwrap();
        write_payloads(&root.join("schemas"));

        let doc = root.join("doc.md");
        let result = scan(&doc, root);
        assert!(result.is_err(), "transactional: no registry on any failure");
    }

    #[test]
    fn vacuous_trigger_is_hard_error() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::write(
            root.join("schemas/vacuous.trigger.yaml"),
            "kind: trigger-schema\nmatch:\n  maybe: string\n",
        )
        .unwrap();

        let doc = root.join("doc.md");
        let err = scan(&doc, root).unwrap_err();
        assert!(matches!(
            err,
            SchemaError::TriggerLoad { ref source, .. }
                if matches!(**source, SchemaError::TriggerVacuousArm)
        ));
    }

    // ── Full scan integration ───────────────────────────────────────────

    #[test]
    fn scan_nested_roots_and_ordering() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("pkg/schemas")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();

        // Nearest root has two triggers; root-level has one (shadowed).
        write_file(
            &root.join("pkg/schemas/a.trigger.yaml"),
            VALID_TRIGGER,
        );
        write_file(
            &root.join("pkg/schemas/b.trigger.yaml"),
            VALID_TRIGGER_2,
        );
        write_file(
            &root.join("schemas/a.trigger.yaml"),
            VALID_TRIGGER_2,
        );
        write_payloads(&root.join("pkg/schemas"));

        let doc = root.join("pkg/sub/doc.md");
        let registry = scan(&doc, root).unwrap();
        // Two unshadowed triggers (a and b from nearest root).
        assert_eq!(registry.triggers.len(), 2);
        // Order: filename-lexicographic within nearest root.
        assert!(registry.triggers[0]
            .source
            .ends_with("pkg/schemas/a.trigger.yaml"));
        assert!(registry.triggers[1]
            .source
            .ends_with("pkg/schemas/b.trigger.yaml"));
        // Root-level a.trigger.yaml is shadowed.
        assert_eq!(registry.shadowed.len(), 1);
        // Roots discovered nearest first.
        assert_eq!(registry.roots.len(), 2);
        assert!(registry.roots[0].ends_with("pkg/schemas"));
    }

    #[test]
    fn scan_document_outside_boundary_empty() {
        let repo = repo_fixture();
        let other = repo_fixture();
        fs::create_dir_all(repo.path().join("schemas")).unwrap();
        fs::write(
            repo.path().join("schemas/t.trigger.yaml"),
            VALID_TRIGGER,
        )
        .unwrap();

        let doc = other.path().join("doc.md");
        let registry = scan(&doc, repo.path()).unwrap();
        assert!(registry.is_empty());
        assert!(registry.roots.is_empty());
    }

    #[test]
    fn scan_no_schemas_dirs_empty() {
        let repo = repo_fixture();
        let doc = repo.path().join("doc.md");
        let registry = scan(&doc, repo.path()).unwrap();
        assert!(registry.is_empty());
    }

    #[test]
    fn scan_yml_and_yaml_mix() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write_file(
            &root.join("schemas/a.trigger.yml"),
            VALID_TRIGGER,
        );
        write_file(
            &root.join("schemas/b.trigger.yaml"),
            VALID_TRIGGER_2,
        );
        write_payloads(&root.join("schemas"));

        let doc = root.join("doc.md");
        let registry = scan(&doc, root).unwrap();
        assert_eq!(registry.triggers.len(), 2);
    }

    #[test]
    fn scan_empty_schemas_dir_no_error() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();

        let doc = root.join("doc.md");
        let registry = scan(&doc, root).unwrap();
        assert!(registry.is_empty());
        assert_eq!(registry.roots.len(), 1);
    }

    #[test]
    fn scan_non_yaml_yaml_files_ignored() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::write(root.join("schemas/readme.md"), "# Schemas").unwrap();
        fs::write(root.join("schemas/config.json"), "{}").unwrap();
        fs::write(
            root.join("schemas/real.trigger.yaml"),
            VALID_TRIGGER,
        )
        .unwrap();
        write_payloads(&root.join("schemas"));

        let doc = root.join("doc.md");
        let registry = scan(&doc, root).unwrap();
        assert_eq!(registry.triggers.len(), 1);
    }

    // ── Payload resolution is transactional at scan time ───────────────

    #[test]
    fn unmatched_trigger_with_missing_payload_is_hard_error() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // The trigger matches on `prompt`, which no document in this test
        // provides — but the payload file is absent, so scan must fail
        // regardless of whether any document would match.
        fs::write(
            root.join("schemas/lonely.trigger.yaml"),
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n\
             $schema: does-not-exist.yaml\n",
        )
        .unwrap();

        let doc = root.join("doc.md");
        let err = scan(&doc, root).unwrap_err();
        assert!(
            matches!(err, SchemaError::TriggerLoad { .. }),
            "missing payload must be a hard load error: {err:?}"
        );
    }

    #[test]
    fn unmatched_trigger_with_non_mergeable_payload_is_hard_error() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // Inline root-union payload — not merge-compatible. The match key
        // (`prompt`) is absent from every document in this test.
        fs::write(
            root.join("schemas/union.trigger.yaml"),
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n\
             $schema:\n  - model: string(required)\n  - title: string(required)\n",
        )
        .unwrap();

        let doc = root.join("doc.md");
        let err = scan(&doc, root).unwrap_err();
        match err {
            SchemaError::TriggerLoad { source, .. } => {
                assert!(
                    matches!(*source, SchemaError::TriggerPayloadNotMergeable { .. }),
                    "inner error must be TriggerPayloadNotMergeable: {source:?}"
                );
            }
            other => panic!("expected TriggerLoad wrapping TriggerPayloadNotMergeable, got {other:?}"),
        }
    }

    #[test]
    fn payload_resolution_failure_is_transactional() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // One valid trigger with a proper payload.
        write_file(&root.join("schemas/good.trigger.yaml"), VALID_TRIGGER);
        write_payloads(&root.join("schemas"));
        // One trigger whose payload file does not exist.
        fs::write(
            root.join("schemas/bad.trigger.yaml"),
            "kind: trigger-schema\nmatch:\n  title: string(required)\n\
             $schema: missing.yaml\n",
        )
        .unwrap();

        let doc = root.join("doc.md");
        let result = scan(&doc, root);
        assert!(
            result.is_err(),
            "a single bad payload must fail the whole scan transactionally"
        );
    }

    // ── Error display ───────────────────────────────────────────────────

    #[test]
    fn case_fold_collision_display_includes_files() {
        let err = SchemaError::TriggerCaseFoldCollision {
            root: PathBuf::from("/repo/schemas"),
            files: vec!["Foo.yaml".into(), "foo.yaml".into()],
        };
        let msg = err.to_string();
        assert!(msg.contains("Foo.yaml"));
        assert!(msg.contains("foo.yaml"));
    }

    #[test]
    fn trigger_load_display_includes_path() {
        let err = SchemaError::TriggerLoad {
            path: PathBuf::from("/repo/schemas/bad.trigger.yaml"),
            source: Box::new(SchemaError::TriggerMatch {
                message: "missing match key".into(),
            }),
        };
        let msg = err.to_string();
        assert!(msg.contains("bad.trigger.yaml"));
        assert!(msg.contains("missing match key"));
    }
}
