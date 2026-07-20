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
pub mod doc_links;
pub mod expressions;
pub mod frontmatter;
pub mod schema;
pub mod shell;
pub mod suggestions;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use biscuit_hash::xx_hash_bytes;
use darkmatter::markdown::schemas::{
    SchemaError, StandaloneSchemaDocument, StandaloneSchemaEnvelope,
    SuggestionLintProblem, parse_standalone_schema_document,
};
use darkmatter::markdown::schemas::triggers::{TriggerRegistry, scan as scan_triggers};
use lsp_types::Uri;

use crate::config::DmlsConfig;

pub use frontmatter::{FmEntry, FmValueKind, FrontmatterAst, YamlParseError};
pub use schema::{
    FrontmatterSchemaValue, MetaSchemaKind, SchemaAuthoringState, SchemaBundle,
};

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

/// Suggestion-lint state for the current document.
///
/// Carries invalid `suggest(...)` candidate problems with document-relative byte
/// spans so the diagnostics module can publish `dm.schema.invalid_suggestion`
/// warnings on the exact authoring argument range. For a standalone
/// SimplifiedSchema YAML document, also carries a malformed-envelope error when
/// a recognized envelope could not be parsed.
#[derive(Clone, Debug)]
pub enum SuggestionState {
    /// The document is neither an inline-schema Markdown document nor a
    /// recognized standalone SimplifiedSchema envelope. No suggestion
    /// diagnostics apply.
    Inactive,
    /// An inline `$schema` mapping was found in Markdown frontmatter. The
    /// problems carry document-relative spans projected through YAML quoting.
    Inline(Vec<SuggestionLintProblem>),
    /// The open buffer was classified as a standalone SimplifiedSchema YAML
    /// document (pure or tagged envelope). Suggestion warnings are owned by
    /// this document, never duplicated onto consuming Markdown documents.
    Standalone {
        /// The content envelope that claimed the document.
        envelope: StandaloneSchemaEnvelope,
        /// Invalid suggestion candidates in declaration order.
        problems: Vec<SuggestionLintProblem>,
        /// A malformed recognized envelope (missing/malformed `types`,
        /// unsupported tagged-envelope keys, invalid payload).
        error: Option<Arc<SchemaError>>,
    },
    /// A YAML file in a `schemas/` root claims `kind: trigger-schema` but its
    /// envelope is malformed. The registry remains last-good while this error
    /// is published on the authoring file.
    TriggerError(Arc<SchemaError>),
}

/// The per-request overlay snapshot for one open document.
pub struct DocumentOverlay {
    /// The frontmatter tree. `None` when the current buffer failed to parse and
    /// no previous good tree exists.
    pub ast: Option<Arc<FrontmatterAst>>,
    /// Whether [`ast`](Self::ast) is a stale last-good frontmatter tree (the
    /// current buffer did not parse).
    pub stale: bool,
    /// The current buffer's YAML parse error, if any.
    pub parse_error: Option<YamlParseError>,
    /// The effective schema, or the failure that prevented it.
    pub schema: SchemaOutcome,
    /// Suggestion-lint state for `suggest(...)` candidate diagnostics.
    pub suggestions: SuggestionState,
    /// Parsed semantic schema-authoring regions and standalone state.
    pub schema_authoring: SchemaAuthoringState,
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

    /// Whether this overlay is a standalone schema authoring document.
    pub fn is_standalone_schema(&self) -> bool {
        matches!(self.schema_authoring, SchemaAuthoringState::Standalone { .. })
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
    /// Source text paired with `last_good`; schema matching must use this text
    /// while the current YAML is malformed so activation does not flap.
    last_good_text: HashMap<String, String>,
    /// Last successfully parsed standalone schema model per document URI.
    standalone_last_good: HashMap<String, Arc<StandaloneSchemaDocument>>,
    /// Last transactionally loaded registry for each document walk within a
    /// boundary. A failed rescan leaves the previous value installed.
    trigger_registries: HashMap<(PathBuf, PathBuf), TriggerRegistry>,
    /// Current failed transactional scan per discovery scope. This state is
    /// independent of `trigger_registries` so failure ownership can change
    /// without discarding the last-good registry.
    trigger_load_errors: HashMap<(PathBuf, PathBuf), TriggerLoadError>,
    /// Diagnostic ownership changes waiting for the router to publish them.
    trigger_diagnostic_transitions: Vec<TriggerDiagnosticTransition>,
    /// Cached effective schema per document URI, invalidated by content hash.
    schema: HashMap<String, CachedSchema>,
}

#[derive(Clone)]
struct TriggerLoadError {
    path: PathBuf,
    error: Arc<SchemaError>,
}

/// A trigger-load diagnostic that must be published or cleared.
pub struct TriggerDiagnosticTransition {
    /// Trigger envelope owning the diagnostic.
    pub path: PathBuf,
    /// The current load error, or `None` when a prior diagnostic must clear.
    pub error: Option<Arc<SchemaError>>,
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
    /// `None` for a document with no frontmatter block and no standalone
    /// SimplifiedSchema envelope. `Some` otherwise, carrying the (possibly
    /// stale) tree, the current parse error, effective schema outcome,
    /// suggestion-lint state, and semantic schema-authoring state.
    pub fn for_document(
        &self,
        uri: &Uri,
        text: &str,
        path: &Path,
        config: &DmlsConfig,
        workspace_roots: &[PathBuf],
    ) -> Option<DocumentOverlay> {
        // Markdown frontmatter path.
        if let Some(parse) = FrontmatterAst::parse(text) {
            let uri_key = uri.as_str().to_string();
            let mut cache = self.inner.lock().expect("overlay lock poisoned");

            let (ast, stale, parse_error) = match parse.ast {
                Some(fresh) => {
                    let fresh = Arc::new(fresh);
                    cache.last_good.insert(uri_key.clone(), Arc::clone(&fresh));
                    cache.last_good_text.insert(uri_key.clone(), text.to_string());
                    (Some(fresh), false, None)
                }
                None => {
                    let previous = cache.last_good.get(&uri_key).map(Arc::clone);
                    (previous.clone(), previous.is_some(), parse.error)
                }
            };

            let schema_text = if stale {
                cache.last_good_text.get(&uri_key).map(String::as_str).unwrap_or(text)
            } else {
                text
            };
            let schema_text = schema_text.to_string();
            let schema = cache.schema_for(
                &uri_key,
                &schema_text,
                path,
                config,
                workspace_roots,
            );
            let suggestions = suggestions::inline_lints(text, ast.as_deref());
            let schema_authoring = schema::frontmatter_authoring(ast.as_deref(), &schema);
            return Some(DocumentOverlay {
                ast,
                stale,
                parse_error,
                schema,
                suggestions,
                schema_authoring,
            });
        }

        // Trigger-schema authoring path. Only claimed, malformed envelopes are
        // diagnostics; ordinary schema-root YAML remains silent.
        if path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "schemas")
            && matches!(path.extension().and_then(|ext| ext.to_str()), Some("yaml" | "yml"))
        {
            match darkmatter::markdown::schemas::parse_trigger_envelope_from_str(text) {
                Err(error) => {
                    return Some(DocumentOverlay {
                        ast: None,
                        stale: false,
                        parse_error: None,
                        schema: SchemaOutcome::Ready(None),
                        suggestions: SuggestionState::TriggerError(Arc::new(error)),
                        schema_authoring: SchemaAuthoringState::Inactive,
                    });
                }
                Ok(Some(_)) => {
                    let mut cache = self.inner.lock().expect("overlay lock poisoned");
                    cache.trigger_registry(path, workspace_roots);
                    if let Some(error) = cache
                        .trigger_load_errors
                        .values()
                        .find(|failure| failure.path == path)
                        .map(|failure| Arc::clone(&failure.error))
                    {
                        return Some(DocumentOverlay {
                            ast: None,
                            stale: false,
                            parse_error: None,
                            schema: SchemaOutcome::Ready(None),
                            suggestions: SuggestionState::TriggerError(error),
                            schema_authoring: SchemaAuthoringState::Inactive,
                        });
                    }
                }
                Ok(None) => {}
            }
        }

        // Standalone SimplifiedSchema YAML envelope path. A lexical content
        // claim keeps the last-good semantic model alive through malformed
        // edits; the current parse result always owns diagnostics.
        let claim = schema::standalone_envelope_claim(text);
        match parse_standalone_schema_document(text, path) {
            Ok(Some(document)) => {
                let envelope = document.envelope;
                let problems = document.suggestion_lints.clone();
                let model = Arc::new(document);
                let mut cache = self.inner.lock().expect("overlay lock poisoned");
                cache
                    .standalone_last_good
                    .insert(uri.as_str().to_string(), Arc::clone(&model));
                return Some(DocumentOverlay {
                    ast: None,
                    stale: false,
                    parse_error: None,
                    schema: SchemaOutcome::Ready(None),
                    suggestions: SuggestionState::Standalone {
                        envelope,
                        problems,
                        error: None,
                    },
                    schema_authoring: SchemaAuthoringState::Standalone {
                        envelope,
                        model: Some(model),
                        stale: false,
                        error: None,
                    },
                });
            }
            Ok(None) if claim.is_none() => {}
            result => {
                let envelope = claim?;
                let error = match result {
                    Err(error) => Arc::new(error),
                    Ok(None) => Arc::new(SchemaError::SchemaDocument {
                        path: path.to_path_buf(),
                        message: serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text)
                            .expect_err("a lexical claim returning None must be malformed YAML")
                            .to_string(),
                    }),
                    Ok(Some(_)) => unreachable!("handled above"),
                };
                let cache = self.inner.lock().expect("overlay lock poisoned");
                let model = cache.standalone_last_good.get(uri.as_str()).map(Arc::clone);
                let stale = model.is_some();
                return Some(DocumentOverlay {
                    ast: None,
                    stale: false,
                    parse_error: None,
                    schema: SchemaOutcome::Ready(None),
                    suggestions: SuggestionState::Standalone {
                        envelope,
                        problems: Vec::new(),
                        error: Some(Arc::clone(&error)),
                    },
                    schema_authoring: SchemaAuthoringState::Standalone {
                        envelope,
                        model,
                        stale,
                        error: Some(error),
                    },
                });
            }
        }

        None
    }

    /// Drops a closed document's cached state.
    pub fn forget(&self, uri: &Uri) {
        let mut cache = self.inner.lock().expect("overlay lock poisoned");
        cache.last_good.remove(uri.as_str());
        cache.last_good_text.remove(uri.as_str());
        cache.standalone_last_good.remove(uri.as_str());
        cache.schema.remove(uri.as_str());
    }

    /// Takes trigger-load diagnostic ownership changes since the last refresh.
    pub fn take_trigger_diagnostic_transitions(&self) -> Vec<TriggerDiagnosticTransition> {
        let mut cache = self.inner.lock().expect("overlay lock poisoned");
        std::mem::take(&mut cache.trigger_diagnostic_transitions)
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
        let registry = self.trigger_registry(path, workspace_roots);
        let registry_key = registry
            .as_ref()
            .map(|registry| xx_hash_bytes(format!("{registry:?}").as_bytes()))
            .unwrap_or_default();
        let key = schema_cache_key(text, config) ^ registry_key;
        if let Some(cached) = self.schema.get(uri_key)
            && cached.key == key
            && deps_unchanged(&cached.deps)
        {
            return cached.outcome.clone();
        }
        let outcome = match schema::assemble(path, text, config, workspace_roots, registry) {
            Ok(bundle) => SchemaOutcome::Ready(bundle.map(Arc::new)),
            Err(error) => SchemaOutcome::Failed(Arc::new(error)),
        };
        let deps = collect_dep_hashes(&outcome);
        self.schema
            .insert(uri_key.to_string(), CachedSchema { key, deps, outcome: outcome.clone() });
        outcome
    }

    fn trigger_registry(
        &mut self,
        path: &Path,
        workspace_roots: &[PathBuf],
    ) -> Option<TriggerRegistry> {
        let boundary = schema::trigger_boundary(path, workspace_roots)?;
        let document_dir = path.parent().unwrap_or(path).to_path_buf();
        let key = (boundary.clone(), document_dir);
        match scan_triggers(path, &boundary) {
            Ok(registry) => {
                self.trigger_registries.insert(key.clone(), registry.clone());
                if let Some(previous) = self.trigger_load_errors.remove(&key) {
                    self.trigger_diagnostic_transitions.push(TriggerDiagnosticTransition {
                        path: previous.path,
                        error: None,
                    });
                }
                Some(registry)
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "retaining last-good trigger registry");
                if let SchemaError::TriggerLoad { path, .. } = &error {
                    let failure = TriggerLoadError {
                        path: path.clone(),
                        error: Arc::new(error),
                    };
                    let previous = self.trigger_load_errors.insert(key.clone(), failure.clone());
                    if let Some(old) = previous.as_ref().filter(|old| old.path != failure.path) {
                        self.trigger_diagnostic_transitions.push(TriggerDiagnosticTransition {
                            path: old.path.clone(),
                            error: None,
                        });
                    }
                    if previous.as_ref().is_none_or(|old| {
                        old.path != failure.path || old.error.to_string() != failure.error.to_string()
                    }) {
                        self.trigger_diagnostic_transitions.push(TriggerDiagnosticTransition {
                            path: failure.path,
                            error: Some(failure.error),
                        });
                    }
                }
                self.trigger_registries.get(&key).cloned()
            }
        }
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

    use darkmatter::markdown::schemas::{SchemaSourcePath, SchemaSpanKind, SimplifiedSchema};

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

    #[test]
    fn standalone_pure_and_tagged_models_carry_schema_and_authored_spans() {
        for (uri_text, path, text, property) in [
            (
                "file:///w/notes.txt",
                Path::new("/w/notes.txt"),
                "$schema:\r\n  café: \"string(required)\"\r\n",
                "café",
            ),
            (
                "file:///w/types.data",
                Path::new("/w/types.data"),
                "---\nkind: schema\ntypes:\n  title: 'number(min(1))'\n...\n",
                "title",
            ),
        ] {
            let state = OverlayState::default();
            let overlay = state
                .for_document(
                    &uri(uri_text),
                    text,
                    path,
                    &DmlsConfig::default(),
                    &[PathBuf::from("/w")],
                )
                .expect("content claims a standalone schema");
            let SchemaAuthoringState::Standalone { model: Some(model), stale, error, .. } =
                &overlay.schema_authoring
            else {
                panic!("expected a parsed standalone schema model");
            };
            assert!(!stale);
            assert!(error.is_none());
            let property_path = SchemaSourcePath::root().property(property);
            let key_span = model
                .source_map
                .spans(&property_path, SchemaSpanKind::MappingKey)
                .first()
                .expect("mapping-key span");
            assert_eq!(&text[key_span.clone()], property);
            let type_span = model
                .source_map
                .spans(&property_path, SchemaSpanKind::TypeKeyword)
                .first()
                .expect("type-keyword span");
            assert!(matches!(&text[type_span.clone()], "string" | "number"));
            assert!(matches!(model.schema(), Some(SimplifiedSchema::Single(_))));
        }
    }

    #[test]
    fn frontmatter_meta_schema_activation_uses_effective_property_definitions() {
        let text = concat!(
            "---\n$schema:\n",
            "  definition_value: [string, type-definition]\n",
            "  schema_value: schema\n",
            "  schema_ref: schema\n",
            "  plain: string\n",
            "  missing_value: type-definition\n",
            "definition_value: \"number(min(1))\"\n",
            "schema_value:\n  title: string(required)\n",
            "schema_ref: './missing.yaml'\n",
            "plain: type-definition\n",
            "---\nBody\n",
        );
        let state = OverlayState::default();
        let overlay = state
            .for_document(
                &uri("file:///w/doc.md"),
                text,
                Path::new("/w/doc.md"),
                &DmlsConfig::default(),
                &[PathBuf::from("/w")],
            )
            .expect("frontmatter overlay");
        let SchemaAuthoringState::Frontmatter(values) = &overlay.schema_authoring else {
            panic!("expected frontmatter semantic activation");
        };
        let kinds = |pointer: &str| {
            values
                .iter()
                .find(|value| value.pointer == pointer)
                .map(|value| value.kinds.as_slice())
        };
        assert_eq!(kinds("/$schema"), Some(&[MetaSchemaKind::Schema][..]));
        assert_eq!(
            kinds("/definition_value"),
            Some(&[MetaSchemaKind::TypeDefinition][..])
        );
        assert_eq!(kinds("/schema_value"), Some(&[MetaSchemaKind::Schema][..]));
        assert_eq!(kinds("/schema_ref"), Some(&[MetaSchemaKind::Schema][..]));
        assert!(kinds("/plain").is_none(), "key names and value text never activate");
        assert!(
            kinds("/missing_value").is_none(),
            "an effective definition without an authored value has no active region"
        );
        for value in values {
            assert!(!value.value_span.is_empty());
            assert!(value.value_span.end <= text.len());
        }
    }

    #[test]
    fn standalone_activation_is_content_based_not_path_based() {
        let state = OverlayState::default();
        let roots = [PathBuf::from("/w")];
        for (uri_text, path, text) in [
            ("file:///w/note.txt", Path::new("/w/note.txt"), "$schema:\n  title: string\n"),
            (
                "file:///w/anything.data",
                Path::new("/w/anything.data"),
                "kind: schema\ntypes:\n  title: string\n",
            ),
        ] {
            let overlay = state
                .for_document(
                    &uri(uri_text),
                    text,
                    path,
                    &DmlsConfig::default(),
                    &roots,
                )
                .expect("content activates regardless of path");
            assert!(overlay.is_standalone_schema());
        }

        for (uri_text, path, text) in [
            (
                "file:///w/schemas/config.yaml",
                Path::new("/w/schemas/config.yaml"),
                "title: ordinary yaml\n",
            ),
            (
                "file:///w/schema.schema.yaml",
                Path::new("/w/schema.schema.yaml"),
                "name: filename alone is inert\n",
            ),
            (
                "file:///w/raw.schema.json",
                Path::new("/w/raw.schema.json"),
                "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"type\":\"object\"}",
            ),
        ] {
            assert!(
                state
                    .for_document(
                        &uri(uri_text),
                        text,
                        path,
                        &DmlsConfig::default(),
                        &roots,
                    )
                    .is_none(),
                "path-only activation is forbidden for {path:?}"
            );
        }
    }

    #[test]
    fn standalone_last_good_survives_malformed_yaml_and_shape_edits() {
        let state = OverlayState::default();
        let doc = uri("file:///w/schema.yaml");
        let path = Path::new("/w/schema.yaml");
        let roots = [PathBuf::from("/w")];
        let good = "$schema:\n  title: string(required)\n";
        let first = state
            .for_document(&doc, good, path, &DmlsConfig::default(), &roots)
            .expect("valid schema overlay");
        let SchemaAuthoringState::Standalone { model: Some(first_model), .. } =
            &first.schema_authoring
        else {
            panic!("valid standalone model");
        };

        for malformed in ["$schema:\n\tbad: string\n", "$schema: 42\n"] {
            let current = state
                .for_document(&doc, malformed, path, &DmlsConfig::default(), &roots)
                .expect("lexical claim remains active");
            let SchemaAuthoringState::Standalone { model, stale, error, .. } =
                &current.schema_authoring
            else {
                panic!("malformed claimed document stays standalone");
            };
            let retained = model.as_ref().expect("last-good model retained");
            assert!(Arc::ptr_eq(first_model, retained));
            assert!(*stale);
            assert!(error.is_some(), "current text owns a parser error");
            assert!(matches!(
                current.suggestions,
                SuggestionState::Standalone { error: Some(_), .. }
            ));
        }

        let fresh = OverlayState::default();
        let malformed = fresh
            .for_document(
                &uri("file:///w/fresh.yaml"),
                "$schema:\n\tbad: string\n",
                Path::new("/w/fresh.yaml"),
                &DmlsConfig::default(),
                &roots,
            )
            .expect("lexical claim activates diagnostics");
        let SchemaAuthoringState::Standalone { model, stale, error, .. } =
            malformed.schema_authoring
        else {
            panic!("fresh malformed claim is standalone");
        };
        assert!(model.is_none(), "no semantic model may be invented");
        assert!(!stale);
        assert!(error.is_some());
    }

    #[test]
    fn forget_clears_standalone_last_good_state() {
        let state = OverlayState::default();
        let doc = uri("file:///w/schema.yaml");
        let path = Path::new("/w/schema.yaml");
        let roots = [PathBuf::from("/w")];
        state
            .for_document(
                &doc,
                "$schema:\n  title: string\n",
                path,
                &DmlsConfig::default(),
                &roots,
            )
            .expect("seed standalone model");
        state.forget(&doc);
        let overlay = state
            .for_document(
                &doc,
                "$schema: 42\n",
                path,
                &DmlsConfig::default(),
                &roots,
            )
            .expect("claimed malformed overlay");
        let SchemaAuthoringState::Standalone { model, stale, error, .. } =
            overlay.schema_authoring
        else {
            panic!("standalone state");
        };
        assert!(model.is_none());
        assert!(!stale);
        assert!(error.is_some());
    }

    #[test]
    fn shipped_schema_corpus_uses_content_classification() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/schemas");
        let expected = std::collections::BTreeMap::from([
            ("claudine-types.yaml".to_string(), "ready"),
            ("claudine.yaml".to_string(), "ready"),
            ("darkmatter.yaml".to_string(), "ready"),
            ("env.yaml".to_string(), "ready"),
            ("err.yaml".to_string(), "error"),
            ("expression-functions.yaml".to_string(), "inactive"),
            ("schema-definition.yaml".to_string(), "inactive"),
        ]);
        let state = OverlayState::default();
        let roots = [root.clone()];
        let mut observed = std::collections::BTreeMap::new();
        for entry in std::fs::read_dir(&root).expect("schema corpus") {
            let entry = entry.expect("schema artifact");
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
                continue;
            }
            let name = path.file_name().unwrap().to_str().unwrap().to_string();
            let text = std::fs::read_to_string(&path).expect("read artifact");
            let doc: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
            let classification = match state.for_document(
                &doc,
                &text,
                &path,
                &DmlsConfig::default(),
                &roots,
            ) {
                Some(DocumentOverlay {
                    schema_authoring:
                        SchemaAuthoringState::Standalone { model: Some(_), error: None, .. },
                    ..
                }) => "ready",
                Some(DocumentOverlay {
                    schema_authoring:
                        SchemaAuthoringState::Standalone { error: Some(error), .. },
                    ..
                }) => {
                    if name == "darkmatter.yaml" {
                        panic!("shipped Darkmatter schema failed passive parsing: {error}");
                    }
                    "error"
                }
                None => "inactive",
                Some(_) => "other",
            };
            observed.insert(name, classification);
        }
        assert_eq!(observed, expected);
    }

    fn write_trigger_fixture(root: &Path) {
        std::fs::create_dir_all(root.join("schemas")).unwrap();
        std::fs::write(
            root.join("schemas/prompt.yaml"),
            "$schema:\n  model: string(required)\n",
        )
        .unwrap();
        std::fs::write(
            root.join("schemas/prompt.trigger.yaml"),
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: prompt.yaml\n",
        )
        .unwrap();
    }

    fn copy_dialect_fixture(root: &Path) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/schema-triggers");
        for directory in ["schemas", "docs"] {
            std::fs::create_dir_all(root.join(directory)).unwrap();
            for entry in std::fs::read_dir(source.join(directory)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), root.join(directory).join(entry.file_name())).unwrap();
            }
        }
    }

    #[test]
    fn trigger_activation_uses_workspace_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_trigger_fixture(root);
        let path = root.join("doc.md");
        let text = "---\nprompt: hello\n---\n\nbody\n";
        let state = OverlayState::default();
        let doc: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();

        let overlay = state
            .for_document(&doc, text, &path, &DmlsConfig::default(), &[root.to_path_buf()])
            .unwrap();
        let bundle = ready_bundle(&overlay);
        let report = bundle.effective.validate(&bundle.frontmatter_json);
        assert!(
            report.problems.iter().any(|problem| problem.message.contains("model")),
            "matching trigger must contribute its required property: {:?}",
            report.problems
        );

        let outside = root.parent().unwrap().join("outside.md");
        let outside_uri: Uri =
            url::Url::from_file_path(&outside).unwrap().as_str().parse().unwrap();
        let overlay = state
            .for_document(
                &outside_uri,
                text,
                &outside,
                &DmlsConfig::default(),
                &[root.to_path_buf()],
            )
            .unwrap();
        let report = ready_bundle(&overlay).effective.validate(
            &ready_bundle(&overlay).frontmatter_json,
        );
        assert!(report.valid, "outside-workspace documents must not discover triggers");
    }

    #[test]
    fn trigger_activation_has_yaml_hysteresis_and_transactional_registry() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_trigger_fixture(root);
        let path = root.join("doc.md");
        let good = "---\nprompt: hello\n---\n\nbody\n";
        let state = OverlayState::default();
        let doc: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
        let roots = [root.to_path_buf()];

        let first = state
            .for_document(&doc, good, &path, &DmlsConfig::default(), &roots)
            .unwrap();
        assert!(!ready_bundle(&first).effective.validate(&ready_bundle(&first).frontmatter_json).valid);

        let broken = "---\nprompt:\n\tbad: yaml\n---\n";
        let stale = state
            .for_document(&doc, broken, &path, &DmlsConfig::default(), &roots)
            .unwrap();
        assert!(stale.stale);
        assert!(
            !ready_bundle(&stale).effective.validate(&ready_bundle(&stale).frontmatter_json).valid,
            "a malformed edit must retain the prior activation"
        );

        std::fs::write(
            root.join("schemas/prompt.trigger.yaml"),
            "kind: trigger-schema\nmatch: {}\n",
        )
        .unwrap();
        let retained = state
            .for_document(&doc, good, &path, &DmlsConfig::default(), &roots)
            .unwrap();
        assert!(
            !ready_bundle(&retained)
                .effective
                .validate(&ready_bundle(&retained).frontmatter_json)
                .valid,
            "a malformed envelope edit must retain the last-good registry"
        );
    }

    #[test]
    fn trigger_payload_edit_retains_last_good_registry() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_trigger_fixture(root);
        let path = root.join("doc.md");
        let good = "---\nprompt: hello\n---\n\nbody\n";
        let state = OverlayState::default();
        let doc: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
        let roots = [root.to_path_buf()];

        // Seed: the trigger activates and contributes its `model` required
        // property, so the document is invalid (missing `model`).
        let first = state
            .for_document(&doc, good, &path, &DmlsConfig::default(), &roots)
            .unwrap();
        let first_report =
            ready_bundle(&first).effective.validate(&ready_bundle(&first).frontmatter_json);
        assert!(
            first_report.problems.iter().any(|p| p.message.contains("model")),
            "initial activation must contribute the `model` required property: {:?}",
            first_report.problems
        );

        // Edit the PAYLOAD file to an invalid shape (root union). The envelope
        // is untouched; only the payload becomes non-mergeable. `scan` must now
        // fail, so the last-good registry is retained — the document must NOT
        // flap to `SchemaOutcome::Failed` and must keep the prior activation.
        std::fs::write(
            root.join("schemas/prompt.yaml"),
            "$schema:\n  - model: string(required)\n  - title: string(required)\n",
        )
        .unwrap();

        let retained = state
            .for_document(&doc, good, &path, &DmlsConfig::default(), &roots)
            .unwrap();
        let retained_report = ready_bundle(&retained)
            .effective
            .validate(&ready_bundle(&retained).frontmatter_json);
        assert!(
            retained_report.problems.iter().any(|p| p.message.contains("model")),
            "a payload edit must retain the last-good registry and activation: {:?}",
            retained_report.problems
        );
    }

    #[test]
    fn consecutive_failed_scans_transfer_diagnostic_ownership() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let schemas = root.join("schemas");
        std::fs::create_dir(&schemas).unwrap();
        let a_envelope = schemas.join("a.trigger.yaml");
        let a_payload = schemas.join("a.yaml");
        let b_envelope = schemas.join("b.trigger.yaml");
        let b_payload = schemas.join("b.yaml");
        std::fs::write(
            &a_envelope,
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: a.yaml\n",
        )
        .unwrap();
        std::fs::write(&a_payload, "$schema:\n  model: string(required)\n").unwrap();
        std::fs::write(
            &b_envelope,
            "kind: trigger-schema\nmatch:\n  task: string(required)\n$schema: b.yaml\n",
        )
        .unwrap();
        std::fs::write(&b_payload, "$schema:\n  provider: string(required)\n").unwrap();

        let path = root.join("doc.md");
        let text = "---\nprompt: hello\n---\n\nbody\n";
        let state = OverlayState::default();
        let doc: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
        let roots = [root.to_path_buf()];

        let initial = state
            .for_document(&doc, text, &path, &DmlsConfig::default(), &roots)
            .unwrap();
        let initial_report =
            ready_bundle(&initial).effective.validate(&ready_bundle(&initial).frontmatter_json);
        assert!(
            initial_report.problems.iter().any(|problem| problem.message.contains("model")),
            "the matching trigger must contribute to the last-good schema: {:?}",
            initial_report.problems
        );

        std::fs::remove_file(&a_payload).unwrap();
        let failed_a = state
            .for_document(&doc, text, &path, &DmlsConfig::default(), &roots)
            .unwrap();
        let transitions = state.take_trigger_diagnostic_transitions();
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].path, a_envelope);
        assert!(transitions[0].error.is_some());

        std::fs::write(&a_payload, "$schema:\n  model: string(required)\n").unwrap();
        std::fs::remove_file(&b_payload).unwrap();
        let failed_b = state
            .for_document(&doc, text, &path, &DmlsConfig::default(), &roots)
            .unwrap();
        let transitions = state.take_trigger_diagnostic_transitions();
        assert_eq!(transitions.len(), 2);
        assert_eq!(transitions[0].path, a_envelope);
        assert!(transitions[0].error.is_none(), "A's obsolete diagnostic must clear");
        assert_eq!(transitions[1].path, b_envelope);
        assert!(transitions[1].error.is_some(), "B must own the current diagnostic");

        for overlay in [&failed_a, &failed_b] {
            let bundle = ready_bundle(overlay);
            let report = bundle.effective.validate(&bundle.frontmatter_json);
            assert!(
                report.problems.iter().any(|problem| problem.message.contains("model")),
                "a failed replacement scan must retain the last-good consumer schema: {:?}",
                report.problems
            );
        }
    }

    #[test]
    fn dialect_family_matches_cli_parity_corpus() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        copy_dialect_fixture(root);
        let state = OverlayState::default();
        let roots = [root.to_path_buf()];

        for (document, required) in [
            ("claudine.md", "provider"),
            ("inline-compose.md", "output"),
            ("sequence.md", "sequence_name"),
        ] {
            let path = root.join("docs").join(document);
            let text = std::fs::read_to_string(&path).unwrap();
            let doc: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
            let overlay = state
                .for_document(&doc, &text, &path, &DmlsConfig::default(), &roots)
                .unwrap();
            let bundle = ready_bundle(&overlay);
            let report = bundle.effective.validate(&bundle.frontmatter_json);
            assert!(
                report.problems.iter().any(|problem| problem.message.contains(required)),
                "{document} must activate only its `{required}` payload: {:?}",
                report.problems
            );
        }

        let path = root.join("docs/plain.md");
        let text = std::fs::read_to_string(&path).unwrap();
        let doc: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
        let overlay = state
            .for_document(&doc, &text, &path, &DmlsConfig::default(), &roots)
            .unwrap();
        let bundle = ready_bundle(&overlay);
        assert!(bundle.effective.validate(&bundle.frontmatter_json).valid);
    }

    #[test]
    fn sibling_only_bare_reference_reaches_dmls_schema_diagnostic_state() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::create_dir_all(root.join("schemas")).unwrap();
        std::fs::write(
            root.join("docs/local.yaml"),
            "$schema:\n  title: string(required)\n",
        )
        .unwrap();
        let path = root.join("docs/doc.md");
        let text = "---\n$schema: local.yaml\ntitle: Test\n---\nBody\n";
        let doc: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
        let overlay = OverlayState::default()
            .for_document(
                &doc,
                text,
                &path,
                &DmlsConfig::default(),
                &[root.to_path_buf()],
            )
            .unwrap();
        let SchemaOutcome::Failed(error) = overlay.schema else {
            panic!("sibling-only bare reference must fail schema preparation");
        };
        assert!(error.to_string().contains("./local.yaml"));
    }
}
