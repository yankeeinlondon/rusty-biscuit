//! The Darkmatter semantic overlay (Layer 2).
//!
//! Where the substrate ([`crate::graph`]) sees plain Markdown, the overlay sees
//! frontmatter *meaning*: a position-aware [`FrontmatterAst`] and the layered
//! effective schema assembled for the document. [`OverlayState`] owns the
//! cross-request state the design's FrontmatterAst policy needs — a per-document
//! last-good tree (so completion/hover survive a mid-keystroke YAML error) and a
//! per-document effective-schema cache (so the jsonschema compile is paid once
//! per edit, not once per request).
//!
//! A [`DocumentOverlay`] is the per-request snapshot handed to providers through
//! [`DocumentContext`](crate::providers::DocumentContext); no provider ever
//! touches the YAML parser or the schema library directly.

pub mod directives;
pub mod expressions;
pub mod frontmatter;
pub mod schema;
pub mod shell;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use biscuit_hash::xx_hash_bytes;
use darkmatter::markdown::schemas::SchemaError;
use lsp_types::Uri;

use crate::config::DmlsConfig;

pub use frontmatter::{FmEntry, FmValueKind, FrontmatterAst, YamlParseError};
pub use schema::SchemaBundle;

/// The outcome of assembling a document's effective schema.
#[derive(Clone)]
pub enum SchemaOutcome {
    /// Assembly succeeded: `Some` when the document has a frontmatter map,
    /// `None` when it has none.
    Ready(Option<Arc<SchemaBundle>>),
    /// Assembly failed — carried so a `dm.schema.prepare` /
    /// `dm.schema.invalid_schema_shape` diagnostic can be ranged.
    Failed(Arc<SchemaError>),
}

/// The per-request overlay snapshot for one open document.
pub struct DocumentOverlay {
    /// The frontmatter tree. `None` when the current buffer failed to parse and
    /// no previous good tree exists.
    pub ast: Option<Arc<FrontmatterAst>>,
    /// Whether [`ast`](Self::ast) is a stale last-good tree (the current buffer
    /// did not parse).
    pub stale: bool,
    /// The current buffer's YAML parse error, if any.
    pub parse_error: Option<YamlParseError>,
    /// The effective schema, or the failure that prevented it.
    pub schema: SchemaOutcome,
}

impl DocumentOverlay {
    /// The ready schema bundle, if assembly succeeded and the document has a
    /// frontmatter map.
    pub fn bundle(&self) -> Option<&SchemaBundle> {
        match &self.schema {
            SchemaOutcome::Ready(Some(bundle)) => Some(bundle),
            _ => None,
        }
    }
}

/// Cross-request overlay caches, behind one lock.
#[derive(Default)]
pub struct OverlayState {
    inner: Mutex<OverlayCache>,
}

#[derive(Default)]
struct OverlayCache {
    /// Last successfully-parsed frontmatter tree per document URI.
    last_good: HashMap<String, Arc<FrontmatterAst>>,
    /// Cached effective schema per document URI, invalidated by content hash.
    schema: HashMap<String, CachedSchema>,
}

struct CachedSchema {
    key: u64,
    /// Each dependency file with the xxHash of its bytes at assembly time. The
    /// set covers the effective schema's dependencies (imports, examples, and the
    /// referenced `$schema` file itself) plus the extension-baseline files (each
    /// matched extension's own path and the types it imports). A cache hit
    /// additionally requires every entry to re-hash to the same value, so editing
    /// a referenced schema file, an imported types file, a referenced example, or
    /// a configured extension baseline invalidates the bundle even when the
    /// document text and config are unchanged. Empty for documents with no such
    /// dependencies, which keeps their lookup a pure text+config check with no
    /// extra I/O.
    deps: Vec<(PathBuf, u64)>,
    outcome: SchemaOutcome,
}

impl OverlayState {
    /// Builds the overlay for one open document.
    ///
    /// ## Returns
    ///
    /// `None` for a document with no frontmatter block (nothing for the overlay
    /// to analyze — the substrate handles the body). `Some` otherwise, carrying
    /// the (possibly stale) tree, the current parse error, and the schema
    /// outcome.
    pub fn for_document(
        &self,
        uri: &Uri,
        text: &str,
        path: &Path,
        config: &DmlsConfig,
        workspace_roots: &[PathBuf],
    ) -> Option<DocumentOverlay> {
        let parse = FrontmatterAst::parse(text)?;
        let uri_key = uri.as_str().to_string();
        let mut cache = self.inner.lock().expect("overlay lock poisoned");

        let (ast, stale, parse_error) = match parse.ast {
            Some(fresh) => {
                let fresh = Arc::new(fresh);
                cache.last_good.insert(uri_key.clone(), Arc::clone(&fresh));
                (Some(fresh), false, None)
            }
            None => {
                let previous = cache.last_good.get(&uri_key).map(Arc::clone);
                (previous.clone(), previous.is_some(), parse.error)
            }
        };

        let schema = cache.schema_for(&uri_key, text, path, config, workspace_roots);
        Some(DocumentOverlay { ast, stale, parse_error, schema })
    }

    /// Drops a closed document's cached state.
    pub fn forget(&self, uri: &Uri) {
        let mut cache = self.inner.lock().expect("overlay lock poisoned");
        cache.last_good.remove(uri.as_str());
        cache.schema.remove(uri.as_str());
    }
}

impl OverlayCache {
    /// Returns the cached schema outcome, reassembling only when the document
    /// content or the schema configuration changed.
    fn schema_for(
        &mut self,
        uri_key: &str,
        text: &str,
        path: &Path,
        config: &DmlsConfig,
        workspace_roots: &[PathBuf],
    ) -> SchemaOutcome {
        let key = schema_cache_key(text, config);
        if let Some(cached) = self.schema.get(uri_key)
            && cached.key == key
            && deps_unchanged(&cached.deps)
        {
            return cached.outcome.clone();
        }
        let outcome = match schema::assemble(path, text, config, workspace_roots) {
            Ok(bundle) => SchemaOutcome::Ready(bundle.map(Arc::new)),
            Err(error) => SchemaOutcome::Failed(Arc::new(error)),
        };
        let deps = collect_dep_hashes(&outcome);
        self.schema
            .insert(uri_key.to_string(), CachedSchema { key, deps, outcome: outcome.clone() });
        outcome
    }
}

/// Content-and-config identity for the schema cache: the document text plus a
/// signature of the schema configuration (extensions + strict mode).
fn schema_cache_key(text: &str, config: &DmlsConfig) -> u64 {
    let config_bytes = serde_json::to_vec(&config.schema).unwrap_or_default();
    xx_hash_bytes(text.as_bytes()) ^ xx_hash_bytes(&config_bytes)
}

/// Reads and hashes each dependency of an assembled bundle, so a later lookup
/// can detect a content change. Covers the effective schema's dependencies
/// (imports, examples, referenced `$schema` file) and the extension-baseline
/// files, which are disjoint sources but may name the same file — dedup by path
/// so each file is hashed once. A dependency that cannot be read is omitted —
/// `assemble` already read every one, so an unreadable one is a transient race,
/// and omitting it forces a fresh assembly on the next lookup rather than
/// trusting a stale bundle.
fn collect_dep_hashes(outcome: &SchemaOutcome) -> Vec<(PathBuf, u64)> {
    let SchemaOutcome::Ready(Some(bundle)) = outcome else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    bundle
        .effective
        .dependencies()
        .iter()
        .chain(bundle.extension_dependencies.iter())
        .filter(|path| seen.insert((*path).clone()))
        .filter_map(|path| {
            std::fs::read(path)
                .ok()
                .map(|bytes| (path.clone(), xx_hash_bytes(&bytes)))
        })
        .collect()
}

/// Whether every recorded dependency file still hashes to its recorded value. An
/// empty set (the no-dependency fast path) is trivially unchanged and reads
/// nothing; a missing or unreadable dependency counts as changed, forcing
/// reassembly.
fn deps_unchanged(deps: &[(PathBuf, u64)]) -> bool {
    deps.iter().all(|(path, hash)| {
        std::fs::read(path)
            .map(|bytes| xx_hash_bytes(&bytes) == *hash)
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(s: &str) -> Uri {
        s.parse().unwrap()
    }

    #[test]
    fn test_no_frontmatter_returns_none() {
        let state = OverlayState::default();
        assert!(
            state
                .for_document(
                    &uri("file:///w/doc.md"),
                    "# body only\n",
                    Path::new("/w/doc.md"),
                    &DmlsConfig::default(),
                    &[PathBuf::from("/w")],
                )
                .is_none()
        );
    }

    #[test]
    fn test_fresh_parse_is_not_stale() {
        let state = OverlayState::default();
        let overlay = state
            .for_document(
                &uri("file:///w/doc.md"),
                "---\ntitle: X\n---\n\nbody\n",
                Path::new("/w/doc.md"),
                &DmlsConfig::default(),
                &[PathBuf::from("/w")],
            )
            .unwrap();
        assert!(!overlay.stale);
        assert!(overlay.ast.is_some());
        assert!(overlay.parse_error.is_none());
    }

    fn ready_bundle(overlay: &DocumentOverlay) -> Arc<SchemaBundle> {
        match &overlay.schema {
            SchemaOutcome::Ready(Some(bundle)) => Arc::clone(bundle),
            _ => panic!("expected a ready schema bundle"),
        }
    }

    #[test]
    fn schema_cache_invalidates_on_dependency_change() {
        // Editing an imported types file — with the document text and config
        // otherwise unchanged — must invalidate the cached bundle, not leave the
        // open document validating against the stale schema.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("types.yaml"), "$schema:\n  type: 'enum(a, b)'\n").unwrap();
        let doc_path = root.join("doc.md");
        let text = "---\n$schema:\n  value: type@./types.yaml\nvalue: a\n---\n\nbody\n";
        let state = OverlayState::default();
        let doc = uri("file:///w/doc.md");
        let roots = [root.to_path_buf()];

        let first = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &roots)
            .unwrap();
        let bundle1 = ready_bundle(&first);
        let report1 = bundle1.effective.validate(&bundle1.frontmatter_json);
        assert!(report1.valid, "`a` is a valid enum(a, b): {:?}", report1.problems);

        // Change the imported file only.
        std::fs::write(root.join("types.yaml"), "$schema:\n  type: 'enum(x, y)'\n").unwrap();

        let second = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &roots)
            .unwrap();
        let bundle2 = ready_bundle(&second);
        assert!(
            !Arc::ptr_eq(&bundle1, &bundle2),
            "a dependency-content change must reassemble the bundle"
        );
        let report2 = bundle2.effective.validate(&bundle2.frontmatter_json);
        assert!(
            !report2.valid,
            "`a` is no longer a valid enum(x, y): {:?}",
            report2.problems
        );
    }

    #[test]
    fn schema_cache_invalidates_on_referenced_schema_file_change() {
        // A `$schema: ./schema.yaml` document depends on the referenced schema
        // file's own content. Editing that file — with the document text and
        // config unchanged — must reassemble the bundle, not leave the document
        // validating against the stale referenced schema.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("schema.yaml"), "$schema:\n  value: 'enum(a, b)'\n").unwrap();
        let doc_path = root.join("doc.md");
        let text = "---\n$schema: ./schema.yaml\nvalue: a\n---\n\nbody\n";
        let state = OverlayState::default();
        let doc = uri("file:///w/doc.md");
        let roots = [root.to_path_buf()];

        let first = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &roots)
            .unwrap();
        let bundle1 = ready_bundle(&first);
        let report1 = bundle1.effective.validate(&bundle1.frontmatter_json);
        assert!(report1.valid, "`a` is a valid enum(a, b): {:?}", report1.problems);

        // Change the referenced schema file's own type only.
        std::fs::write(root.join("schema.yaml"), "$schema:\n  value: 'enum(x, y)'\n").unwrap();

        let second = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &roots)
            .unwrap();
        let bundle2 = ready_bundle(&second);
        assert!(
            !Arc::ptr_eq(&bundle1, &bundle2),
            "a referenced-schema-file change must reassemble the bundle"
        );
        let report2 = bundle2.effective.validate(&bundle2.frontmatter_json);
        assert!(
            !report2.valid,
            "`a` is no longer a valid enum(x, y): {:?}",
            report2.problems
        );
    }

    #[test]
    fn schema_cache_invalidates_on_extension_baseline_change() {
        // An extension baseline is merged into the baseline JSON schema before
        // the effective schema is assembled, so its dependency edges never reach
        // `effective.dependencies()`. Editing a configured extension baseline —
        // document text and config unchanged — must still reassemble the bundle
        // via `SchemaBundle::extension_dependencies`.
        use crate::config::SchemaExtensionConfig;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("ext.yaml"), "$schema:\n  provider: 'enum(a, b)'\n").unwrap();
        let doc_path = root.join("doc.md");
        // No document `$schema`: `provider` is contributed solely by the
        // extension baseline, so validation exercises the extension edge.
        let text = "---\nprovider: a\n---\n\nbody\n";

        let mut config = DmlsConfig::default();
        config.schema.extensions.insert(
            "ext".to_string(),
            SchemaExtensionConfig {
                path: PathBuf::from("ext.yaml"),
                globs: vec!["*.md".to_string()],
            },
        );

        let state = OverlayState::default();
        let doc = uri("file:///w/doc.md");
        let roots = [root.to_path_buf()];

        let first = state.for_document(&doc, text, &doc_path, &config, &roots).unwrap();
        let bundle1 = ready_bundle(&first);
        let report1 = bundle1.effective.validate(&bundle1.frontmatter_json);
        assert!(report1.valid, "`a` is a valid enum(a, b): {:?}", report1.problems);

        // Change the extension baseline file's declared type only.
        std::fs::write(root.join("ext.yaml"), "$schema:\n  provider: 'enum(x, y)'\n").unwrap();

        let second = state.for_document(&doc, text, &doc_path, &config, &roots).unwrap();
        let bundle2 = ready_bundle(&second);
        assert!(
            !Arc::ptr_eq(&bundle1, &bundle2),
            "an extension-baseline change must reassemble the bundle"
        );
        let report2 = bundle2.effective.validate(&bundle2.frontmatter_json);
        assert!(
            !report2.valid,
            "`a` is no longer a valid enum(x, y): {:?}",
            report2.problems
        );
    }

    #[test]
    fn schema_cache_hit_without_dependencies() {
        // The fast path: a document with no imports/examples records no dep edges,
        // so an unchanged re-request returns the very same cached bundle (Arc).
        let state = OverlayState::default();
        let doc = uri("file:///w/plain.md");
        let path = Path::new("/w/plain.md");
        let roots = [PathBuf::from("/w")];
        let text = "---\ntitle: Hi\n---\n\nbody\n";
        let first = state
            .for_document(&doc, text, path, &DmlsConfig::default(), &roots)
            .unwrap();
        let second = state
            .for_document(&doc, text, path, &DmlsConfig::default(), &roots)
            .unwrap();
        assert!(
            Arc::ptr_eq(&ready_bundle(&first), &ready_bundle(&second)),
            "a no-dependency document must hit the cache unchanged"
        );
    }

    #[test]
    fn test_last_good_survives_malformed_edit() {
        let state = OverlayState::default();
        let doc = uri("file:///w/doc.md");
        let path = Path::new("/w/doc.md");
        let roots = [PathBuf::from("/w")];
        // First, a good parse seeds the last-good tree.
        state
            .for_document(&doc, "---\ntitle: Good\n---\n\nbody\n", path, &DmlsConfig::default(), &roots)
            .unwrap();
        // Then a hard YAML error keeps the previous tree, flagged stale, and
        // surfaces the parse error.
        let overlay = state
            .for_document(&doc, "---\nkey:\n\tbad: 1\n---\n\nbody\n", path, &DmlsConfig::default(), &roots)
            .unwrap();
        assert!(overlay.stale);
        assert!(overlay.parse_error.is_some());
        let ast = overlay.ast.expect("last-good tree retained");
        assert_eq!(ast.entry_by_dotted("title").unwrap().key, "title");
    }
}
