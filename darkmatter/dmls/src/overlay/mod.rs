//! The Darkmatter semantic overlay (Layer 2).
//!
//! Where the substrate ([`crate::graph`]) sees plain Markdown, the overlay sees
//! frontmatter *meaning*: a position-aware [`FrontmatterAst`] and the layered
//! effective schema assembled for the document. [`OverlayState`] owns the
//! cross-request state the design's FrontmatterAst policy needs — a per-document
//! last-good tree (so completion/hover survive a mid-keystroke YAML error), a
//! per-document effective-schema cache (so the jsonschema compile is paid once
//! per edit, not once per request), and the trigger-registry cache
//! ([`registry`]), which serves each document's triggers from memory until a
//! watched change, save, rescan, or configuration change invalidates them.
//!
//! A [`DocumentOverlay`] is the per-request snapshot handed to providers through
//! [`DocumentContext`](crate::providers::DocumentContext); no provider ever
//! touches the YAML parser or the schema library directly.

pub mod directives;
pub mod doc_links;
pub mod expressions;
pub mod frontmatter;
pub mod named_types;
pub mod registry;
pub mod schema;
pub mod shell;
pub mod suggestions;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use biscuit_hash::xx_hash_bytes;

use crate::context::DocumentResolution;
use crate::request_timing::{RequestTimings, Stage};
use crate::work_counters;
use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::file_resolution_context_identity;
use darkmatter::markdown::schemas::resolve::expand_named_types;
use darkmatter::markdown::schemas::{
    SchemaError, SchemaRootKind, SchemaRootState, SchemaRoots, SchemaSourceId, StandaloneSchemaDocument,
    StandaloneSchemaEnvelope,
    SuggestionLintProblem, parse_standalone_schema_document,
};
use lsp_types::Uri;

use crate::config::DmlsConfig;

pub use frontmatter::{
    FmEntry, FmEntryRole, FmPathSegment, FmScalarStyle, FmValueKind, FrontmatterAst,
    YamlParseError, format_dotted,
};
pub use named_types::NamedTypeCatalog;
pub use registry::{RegistrySummary, RootFileClass};
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
    /// A YAML file in a `schemas/` root claims `kind: schema-trigger` but its
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
    /// The schema files the document's own explicit named-type references
    /// import, read when the overlay was assembled.
    pub named_types: Arc<NamedTypeCatalog>,
    /// Why a standalone schema document's own named-type references do not
    /// resolve (an unknown name, contradictory reuse, or an import cycle).
    pub standalone_resolution: Option<Arc<SchemaError>>,
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
    /// The repositories the last workspace listing found below the
    /// workspace folders, whose scopes
    /// [`workspace_probes`](Self::workspace_probes) adds. A separate lock, so recording them never waits for a lookup.
    workspace_repositories: Mutex<Vec<PathBuf>>,
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
    /// Each document's trigger registry, its failure state, and the
    /// diagnostic ownership changes waiting for the router to publish them.
    triggers: registry::TriggerRegistries,
    /// Cached effective schema per document URI, invalidated by content hash,
    /// configuration, trigger registry, or the document's context.
    schema: HashMap<String, CachedSchema>,
    /// Named-type resolution of each open standalone schema document,
    /// invalidated by content hash, the document's context, or a change to
    /// any file it imports.
    standalone_resolution: HashMap<String, CachedStandalone>,
}

/// A document's trigger registry that is not fresh: what
/// [`OverlayState::prepare_registry`] needs to prepare it.
#[derive(Clone)]
pub struct RegistryNeed {
    /// The document the registry belongs to.
    pub path: PathBuf,
    /// The document's context, which the registry is prepared in.
    pub context: FileResolutionContext,
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
    /// referenced `$schema` file itself), the extension-baseline files (each
    /// matched extension's own path and the types it imports), the files the
    /// named-type catalog read, and — for a failed assembly — the schema files
    /// its error names. A cache hit
    /// additionally requires every entry to re-hash to the same value, so editing
    /// a referenced schema file, an imported types file, a referenced example, or
    /// a configured extension baseline invalidates the bundle even when the
    /// document text and config are unchanged. Empty for documents with no such
    /// dependencies, which keeps their lookup a pure text+config check with no
    /// extra I/O.
    deps: Vec<(PathBuf, u64)>,
    outcome: SchemaOutcome,
    named_types: Arc<NamedTypeCatalog>,
    /// Invalid `suggest(...)` candidates written on the inline `$schema`'s
    /// references, judged against the resolved targets.
    reference_suggestions: Arc<Vec<SuggestionLintProblem>>,
}

/// What [`OverlayCache::schema_for`] assembled for a Markdown document.
struct FrontmatterSchema {
    outcome: SchemaOutcome,
    named_types: Arc<NamedTypeCatalog>,
    reference_suggestions: Arc<Vec<SuggestionLintProblem>>,
}

impl FrontmatterSchema {
    fn new(outcome: SchemaOutcome) -> Self {
        Self {
            outcome,
            named_types: Arc::default(),
            reference_suggestions: Arc::default(),
        }
    }
}

/// One standalone schema document's named-type resolution and the files it
/// read, cached like [`CachedSchema`].
struct CachedStandalone {
    key: u64,
    deps: Vec<(PathBuf, u64)>,
    resolved: StandaloneNamedTypes,
}

/// What resolving a standalone schema document's own references produced.
#[derive(Clone, Default)]
struct StandaloneNamedTypes {
    named_types: Arc<NamedTypeCatalog>,
    error: Option<Arc<SchemaError>>,
    /// Invalid `suggest(...)` candidates written on references, judged
    /// against the resolved targets.
    suggestions: Vec<SuggestionLintProblem>,
}

impl OverlayState {
    /// Builds the overlay for one open document, first preparing on the
    /// calling thread the trigger registry it needs (see
    /// [`prepare_for_document`](Self::prepare_for_document)).
    ///
    /// The entry point for a caller with no server around it (a test, a
    /// corpus check). The server never calls it: it prepares registries off
    /// its dispatch thread and builds overlays with
    /// [`for_document_timed`](Self::for_document_timed), which only looks
    /// registries up.
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
        resolution: &DocumentResolution,
    ) -> Option<DocumentOverlay> {
        self.prepare_for_document(text, path, workspace_roots, resolution);
        let timings = RequestTimings::untimed();
        self.for_document_timed(uri, text, path, config, workspace_roots, resolution, &timings)
    }

    /// Builds the overlay for one open document from the installed trigger
    /// registries, recording the schema-assembly and trigger-registry stages
    /// into the dispatched message's `timings`.
    ///
    /// Never prepares a registry: a document whose registry is not fresh
    /// ([`registry_demand`](Self::registry_demand)) is served the scope's
    /// installed state (none, for a document never prepared). The server
    /// therefore builds an overlay only once the registry the document needs
    /// is prepared.
    #[allow(clippy::too_many_arguments)]
    pub fn for_document_timed(
        &self,
        uri: &Uri,
        text: &str,
        path: &Path,
        config: &DmlsConfig,
        workspace_roots: &[PathBuf],
        resolution: &DocumentResolution,
        timings: &RequestTimings,
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
            let FrontmatterSchema { outcome: schema, named_types, reference_suggestions } =
                timings.time(Stage::SchemaAssembly, || {
                    cache.schema_for(
                        &uri_key,
                        &schema_text,
                        path,
                        config,
                        workspace_roots,
                        resolution,
                        timings,
                    )
                });
            let mut suggestions = suggestions::inline_lints(text, ast.as_deref());
            // A stale outcome was assembled from the last-good text, whose
            // spans do not describe the current buffer.
            if let SuggestionState::Inline(problems) = &mut suggestions
                && !stale
            {
                problems.extend(reference_suggestions.iter().cloned());
            }
            let schema_authoring = schema::frontmatter_authoring(ast.as_deref(), &schema);
            return Some(DocumentOverlay {
                ast,
                stale,
                parse_error,
                schema,
                suggestions,
                schema_authoring,
                named_types,
                standalone_resolution: None,
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
                        named_types: Arc::default(),
                        standalone_resolution: None,
                    });
                }
                Ok(Some(_)) => {
                    let cache = self.inner.lock().expect("overlay lock poisoned");
                    if resolution.context().is_ok() {
                        // A root I/O failure reaches the documents it governs
                        // through `schema_for`; this envelope's own load error
                        // is read below.
                        let _ = timings.time(Stage::TriggerRegistry, || {
                            cache.trigger_registry(path, workspace_roots)
                        });
                    }
                    if let Some(error) = cache.triggers.load_error_for(path) {
                        return Some(DocumentOverlay {
                            ast: None,
                            stale: false,
                            parse_error: None,
                            schema: SchemaOutcome::Ready(None),
                            suggestions: SuggestionState::TriggerError(error),
                            schema_authoring: SchemaAuthoringState::Inactive,
                            named_types: Arc::default(),
                            standalone_resolution: None,
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
                let mut problems = document.suggestion_lints.clone();
                let model = Arc::new(document);
                let mut cache = self.inner.lock().expect("overlay lock poisoned");
                cache
                    .standalone_last_good
                    .insert(uri.as_str().to_string(), Arc::clone(&model));
                let resolved = timings.time(Stage::SchemaAssembly, || {
                    cache.standalone_named_types(
                        uri.as_str(),
                        text,
                        &model,
                        workspace_roots,
                        resolution,
                    )
                });
                problems.extend(resolved.suggestions.iter().cloned());
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
                    named_types: resolved.named_types,
                    standalone_resolution: resolved.error,
                });
            }
            Ok(None) if claim.is_none() => {}
            result => {
                let envelope = claim?;
                let error = match result {
                    Err(error) => Arc::new(error),
                    Ok(None) => {
                        // The claim is lexical and the parser is authoritative.
                        // When the buffer is well-formed YAML the parser has
                        // simply declined it, so the claim over-reached: an
                        // ordinary document stays inert rather than inheriting
                        // an envelope's diagnostics. Only genuinely malformed
                        // text — the mid-edit case the claim exists to serve —
                        // keeps the last-good model alive.
                        let Err(malformed) =
                            serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text)
                        else {
                            return None;
                        };
                        Arc::new(SchemaError::SchemaDocument {
                            path: path.to_path_buf(),
                            message: malformed.to_string(),
                        })
                    }
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
                    named_types: Arc::default(),
                    standalone_resolution: None,
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
        cache.standalone_resolution.remove(uri.as_str());
    }

    /// Takes trigger-load diagnostic ownership changes since the last refresh.
    pub fn take_trigger_diagnostic_transitions(&self) -> Vec<TriggerDiagnosticTransition> {
        let mut cache = self.inner.lock().expect("overlay lock poisoned");
        cache.triggers.take_transitions()
    }

    /// Marks stale every trigger registry a change at one of `paths` can
    /// affect: a watched-file event or a saved YAML file. Each known file is
    /// re-read and compared with its recorded xxHash first, so an unchanged
    /// file invalidates nothing.
    pub fn invalidate_trigger_inputs(&self, paths: &[PathBuf]) {
        let mut cache = self.inner.lock().expect("overlay lock poisoned");
        cache.triggers.invalidate_paths(paths);
    }

    /// Marks every trigger registry stale: a server rescan, a configuration
    /// change, or a package-manifest change.
    pub fn invalidate_all_trigger_registries(&self) {
        let mut cache = self.inner.lock().expect("overlay lock poisoned");
        cache.triggers.invalidate_all();
    }

    /// The folders the prepared trigger registries read from or searched,
    /// for schema-location watches.
    pub fn trigger_watch_folders(&self) -> std::collections::BTreeSet<crate::workspace::watch::WatchedFolder> {
        let cache = self.inner.lock().expect("overlay lock poisoned");
        cache.triggers.watch_folders()
    }

    /// The probe documents of the workspace scopes (see [`workspace_probes`]),
    /// from the repositories and catalogs current now: each workspace
    /// folder, each repository the last listing found below one
    /// ([`set_workspace_repositories`](Self::set_workspace_repositories)),
    /// and inside each of those the packages and package areas its
    /// repository's scope catalog lists, plus each first-level directory of
    /// a monorepo that is a package area only by the catalog's first-directory
    /// fallback and has a `schemas` folder. Their union is the set of schema
    /// roots the folders' documents apply, so preparing every probe (startup
    /// does, and again every refresh that marked them stale) classifies every
    /// root and loads its payloads.
    ///
    /// Each scope is prepared as a document named [`WORKSPACE_PROBE`] in its
    /// folder ([`prepare_workspace_probe`](Self::prepare_workspace_probe)), as
    /// a document there would be; no such file is read and no Markdown is
    /// parsed. Builds contexts, so it does I/O: the server calls it only off
    /// its dispatch thread.
    pub fn workspace_probes(
        &self,
        workspace_roots: &[PathBuf],
        contexts: &crate::context::RepositoryContexts,
    ) -> Vec<PathBuf> {
        workspace_probes(workspace_roots, &self.workspace_repositories(), contexts)
    }

    /// Prepares one of the [`workspace_probes`](Self::workspace_probes) in
    /// its folder's current context, through
    /// [`prepare_registry_held`](Self::prepare_registry_held) with `hold`. A
    /// registry that is still fresh is not prepared again, and a folder
    /// whose context cannot be built is skipped (and reported
    /// [`Failed`](ProbeOutcome::Failed)), since its documents report the
    /// context failure.
    pub fn prepare_workspace_probe(
        &self,
        probe: &Path,
        workspace_roots: &[PathBuf],
        contexts: &crate::context::RepositoryContexts,
        hold: &dyn Fn(),
    ) -> ProbeOutcome {
        let resolution = contexts.for_document(probe);
        match resolution.context() {
            Ok(context) => self.run_preparation(probe, workspace_roots, context, hold).0,
            Err(_) => ProbeOutcome::Failed,
        }
    }

    /// Records the repositories a workspace listing found
    /// ([`DiscoveryReport::repositories`](crate::workspace::DiscoveryReport::repositories)).
    ///
    /// ## Returns
    ///
    /// Whether the set changed, so the caller queues a workspace preparation.
    pub fn set_workspace_repositories(&self, mut repositories: Vec<PathBuf>) -> bool {
        repositories.sort();
        repositories.dedup();
        let mut current = self.workspace_repositories.lock().expect("repository list lock poisoned");
        let changed = *current != repositories;
        *current = repositories;
        changed
    }

    /// Adds (`present`) or removes the repository at `repository`, for a
    /// watched change inside its `.git` entry.
    ///
    /// ## Returns
    ///
    /// Whether the set changed.
    pub fn note_workspace_repository(&self, repository: &Path, present: bool) -> bool {
        let mut current = self.workspace_repositories.lock().expect("repository list lock poisoned");
        let position = current.iter().position(|known| known == repository);
        match (position, present) {
            (None, true) => {
                current.push(repository.to_path_buf());
                current.sort();
                true
            }
            (Some(index), false) => {
                current.remove(index);
                true
            }
            _ => false,
        }
    }

    fn workspace_repositories(&self) -> Vec<PathBuf> {
        self.workspace_repositories.lock().expect("repository list lock poisoned").clone()
    }

    /// The trigger registry the document at `path` with source `text` needs
    /// prepared, or `None` when it needs none or its registry is fresh. A
    /// lookup only: it reads nothing from disk.
    ///
    /// A document needs a registry when [`for_document_timed`](Self::for_document_timed)
    /// would look one up: it has a frontmatter block, or it is a claimed
    /// trigger envelope in a `schemas` folder, and it lies in a workspace
    /// folder and has a context.
    pub fn registry_demand(
        &self,
        text: &str,
        path: &Path,
        workspace_roots: &[PathBuf],
        resolution: &DocumentResolution,
    ) -> Option<RegistryNeed> {
        let has_frontmatter =
            matches!(darkmatter::markdown::extract_frontmatter_block(text), Ok(Some(_)));
        if !has_frontmatter && !is_trigger_envelope(text, path) {
            return None;
        }
        let context = resolution.context().ok()?;
        schema::trigger_boundary(path, workspace_roots)?;
        let cache = self.inner.lock().expect("overlay lock poisoned");
        (!cache.triggers.is_fresh(path, context))
            .then(|| RegistryNeed { path: path.to_path_buf(), context: context.clone() })
    }

    /// Prepares the trigger registry of the document at `path` in `context`
    /// unless it is fresh. The overlay lock is held only to copy the cached
    /// root contents and to install the result, never across the scan and
    /// payload reads, so a caller that only looks registries up waits for
    /// neither. Does I/O: the server calls it only off its dispatch thread.
    ///
    /// ## Returns
    ///
    /// Whether the registry is fresh afterwards; `false` when a change
    /// arrived during the preparation and touched what it read.
    pub fn prepare_registry(
        &self,
        path: &Path,
        workspace_roots: &[PathBuf],
        context: &FileResolutionContext,
    ) -> bool {
        self.prepare_registry_held(path, workspace_roots, context, &|| {})
    }

    /// [`prepare_registry`](Self::prepare_registry), calling `hold` (a test
    /// seam) when a preparation that ran has read its inputs and before its
    /// result is installed, so a change recorded meanwhile makes it stale.
    pub fn prepare_registry_held(
        &self,
        path: &Path,
        workspace_roots: &[PathBuf],
        context: &FileResolutionContext,
        hold: &dyn Fn(),
    ) -> bool {
        self.run_preparation(path, workspace_roots, context, hold).1
    }

    /// [`prepare_registry_held`](Self::prepare_registry_held), also reporting
    /// what the preparation did.
    ///
    /// ## Returns
    ///
    /// The outcome, and whether the registry is fresh afterwards.
    fn run_preparation(
        &self,
        path: &Path,
        workspace_roots: &[PathBuf],
        context: &FileResolutionContext,
        hold: &dyn Fn(),
    ) -> (ProbeOutcome, bool) {
        let Some(boundary) = schema::trigger_boundary(path, workspace_roots) else {
            return (ProbeOutcome::Fresh, true);
        };
        let preparation = {
            let mut cache = self.inner.lock().expect("overlay lock poisoned");
            cache.triggers.begin(path, &boundary, context)
        };
        let Some(preparation) = preparation else {
            return (ProbeOutcome::Fresh, true);
        };
        let prepared = preparation.run();
        let outcome = if prepared.failed() { ProbeOutcome::Failed } else { ProbeOutcome::Prepared };
        hold();
        let mut cache = self.inner.lock().expect("overlay lock poisoned");
        (outcome, cache.triggers.install(prepared))
    }

    /// [`prepare_registry`](Self::prepare_registry) for the registry
    /// [`registry_demand`](Self::registry_demand) says the document needs.
    pub fn prepare_for_document(
        &self,
        text: &str,
        path: &Path,
        workspace_roots: &[PathBuf],
        resolution: &DocumentResolution,
    ) {
        if let Some(need) = self.registry_demand(text, path, workspace_roots, resolution) {
            self.prepare_registry(&need.path, workspace_roots, &need.context);
        }
    }

    /// The schema roots and files the prepared trigger registries searched.
    pub fn trigger_registry_summary(&self) -> RegistrySummary {
        let cache = self.inner.lock().expect("overlay lock poisoned");
        cache.triggers.summary()
    }
}

impl OverlayCache {
    /// Returns the cached schema outcome, reassembling only when the document
    /// content or the schema configuration changed.
    #[allow(clippy::too_many_arguments)]
    fn schema_for(
        &mut self,
        uri_key: &str,
        text: &str,
        path: &Path,
        config: &DmlsConfig,
        workspace_roots: &[PathBuf],
        resolution: &DocumentResolution,
        timings: &RequestTimings,
    ) -> FrontmatterSchema {
        // A document whose context failed resolves no `$schema` reference or
        // trigger payload; its context-failure diagnostic says why.
        let Ok(context) = resolution.context() else {
            self.schema.remove(uri_key);
            return FrontmatterSchema::new(SchemaOutcome::Ready(None));
        };
        let registry = match timings.time(Stage::TriggerRegistry, || {
            self.trigger_registry(path, workspace_roots)
        }) {
            Ok(registry) => registry,
            Err(error) => {
                self.schema.remove(uri_key);
                return FrontmatterSchema::new(SchemaOutcome::Failed(error));
            }
        };
        let registry_key = registry.as_ref().map_or(0, |installed| installed.version);
        let registry = registry.map(|installed| installed.registry);
        // File values, `$schema` references, schema roots, and `$path`
        // verdicts all depend on the context, so the key carries its full
        // identity (repository, `cwd`, tree root and origin, home,
        // environment, `@` roots). The generation adds a rebuilt context
        // whose package catalog changed.
        let context_key = file_resolution_context_identity(context)
            ^ xx_hash_bytes(resolution.generation().to_string().as_bytes());
        let key = schema_cache_key(text, config) ^ registry_key ^ context_key;
        if let Some(cached) = self.schema.get(uri_key)
            && cached.key == key
            && deps_unchanged(&cached.deps)
        {
            return FrontmatterSchema {
                outcome: cached.outcome.clone(),
                named_types: Arc::clone(&cached.named_types),
                reference_suggestions: Arc::clone(&cached.reference_suggestions),
            };
        }
        let schema_roots: Vec<PathBuf> = registry
            .as_ref()
            .map(|registry| registry.roots.search_paths().to_vec())
            .unwrap_or_default();
        let registry_files: Vec<PathBuf> = registry
            .as_ref()
            .map(|registry| registry.dependencies().map(Path::to_path_buf).collect())
            .unwrap_or_default();
        let outcome = match schema::assemble(
            path,
            text,
            config,
            workspace_roots,
            registry,
            context,
        ) {
            Ok(bundle) => SchemaOutcome::Ready(bundle.map(Arc::new)),
            Err(error) => SchemaOutcome::Failed(Arc::new(error)),
        };
        let named_types = Arc::new(frontmatter_named_types(text, path, &schema_roots, context));
        let reference_suggestions = Arc::new(
            FrontmatterAst::parse(text)
                .and_then(|parse| parse.ast)
                .and_then(|ast| suggestions::inline_schema(text, Some(&ast)))
                .map_or_else(Vec::new, |schema| {
                    let base_dir = path.parent().unwrap_or(path);
                    suggestions::reference_lints(&schema, base_dir, None, &schema_roots, context)
                }),
        );
        // The registry tracks its envelopes and payload files, and a change to
        // one installs a new registry version, so this cache does not re-read
        // them.
        let registry_files: std::collections::HashSet<&Path> =
            registry_files.iter().map(PathBuf::as_path).collect();
        let deps = collect_dep_hashes(&outcome, &named_types, &registry_files);
        let schema = FrontmatterSchema {
            outcome,
            named_types,
            reference_suggestions,
        };
        self.schema.insert(
            uri_key.to_string(),
            CachedSchema {
                key,
                deps,
                outcome: schema.outcome.clone(),
                named_types: Arc::clone(&schema.named_types),
                reference_suggestions: Arc::clone(&schema.reference_suggestions),
            },
        );
        schema
    }

    /// Resolves a standalone schema document's own named-type references
    /// against its current buffer, cached until the text, the context, or an
    /// imported file changes.
    ///
    /// Local references read nothing; explicit references read their targets
    /// through the document's context, as resolving the saved file would.
    fn standalone_named_types(
        &mut self,
        uri_key: &str,
        text: &str,
        model: &StandaloneSchemaDocument,
        workspace_roots: &[PathBuf],
        resolution: &DocumentResolution,
    ) -> StandaloneNamedTypes {
        let Ok(context) = resolution.context() else {
            self.standalone_resolution.remove(uri_key);
            return StandaloneNamedTypes::default();
        };
        let key = xx_hash_bytes(text.as_bytes())
            ^ file_resolution_context_identity(context)
            ^ xx_hash_bytes(resolution.generation().to_string().as_bytes());
        if let Some(cached) = self.standalone_resolution.get(uri_key)
            && cached.key == key
            && deps_unchanged(&cached.deps)
        {
            return cached.resolved.clone();
        }
        let path = model.path.as_path();
        // A schema file's bare-name targets use the same roots a consuming
        // document in its workspace folder would use.
        let schema_roots = if schema::trigger_boundary(path, workspace_roots).is_some() {
            SchemaRoots::for_document(&context.for_source(path))
                .map(|roots| roots.search_paths().to_vec())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let base_dir = path.parent().unwrap_or(path);
        let named_types = Arc::new(NamedTypeCatalog::build(
            &model.declaration,
            base_dir,
            &schema_roots,
            context,
        ));
        let expanded = match model.schema() {
            Some(schema) => {
                expand_named_types(schema, base_dir, Some(path), &schema_roots, context)
                    .map(|(_, imports)| imports)
            }
            None => Ok(Vec::new()),
        };
        let (imports, error) = match expanded {
            Ok(imports) => (imports, None),
            Err(error) => (error_dependencies(&error), Some(Arc::new(error))),
        };
        let suggestions = model.schema().map_or_else(Vec::new, |schema| {
            suggestions::reference_lints(schema, base_dir, Some(path), &schema_roots, context)
        });
        let deps = hash_files(imports.iter().chain(named_types.dependencies()));
        let resolved = StandaloneNamedTypes { named_types, error, suggestions };
        self.standalone_resolution.insert(
            uri_key.to_string(),
            CachedStandalone { key, deps, resolved: resolved.clone() },
        );
        resolved
    }

    /// The document's installed trigger registry, looked up in the registry
    /// cache (never prepared). A trigger-file load error keeps the last-good
    /// registry (the file is likely mid-edit, and its own diagnostic reports
    /// the error); a schema-root I/O error is returned instead, since any
    /// registry would silently lack that root's triggers and bare names.
    fn trigger_registry(
        &self,
        path: &Path,
        workspace_roots: &[PathBuf],
    ) -> Result<Option<registry::InstalledRegistry>, Arc<SchemaError>> {
        if schema::trigger_boundary(path, workspace_roots).is_none() {
            return Ok(None);
        }
        self.triggers.lookup(path)
    }
}

/// Whether `text` at `path` is a claimed trigger envelope in a `schemas`
/// folder, the documents whose overlay reads the trigger-load error.
fn is_trigger_envelope(text: &str, path: &Path) -> bool {
    path.parent().and_then(Path::file_name).is_some_and(|name| name == "schemas")
        && matches!(path.extension().and_then(|ext| ext.to_str()), Some("yaml" | "yml"))
        && matches!(darkmatter::markdown::schemas::parse_trigger_envelope_from_str(text), Ok(Some(_)))
}

/// The document name a workspace scope's registry is prepared under (see
/// [`OverlayState::workspace_probes`]). Only its folder matters:
/// the registry is the one any document directly in the folder gets.
pub const WORKSPACE_PROBE: &str = ".dmls-workspace";

/// The probe documents of every workspace scope, deduplicated in this
/// order: one in each workspace folder and each listed repository inside
/// one; then, for each of those scope folders, one in each package-area and
/// package root its repository catalog lists inside it, and one in each
/// [`fallback_areas`] directory.
///
/// A scope that contains the folder (a package or repository above a
/// workspace subfolder) is the folder's own context and needs no probe of
/// its own; a catalog root outside every folder has no workspace documents.
/// What [`OverlayState::prepare_workspace_probe`] did with one probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// Nothing to prepare: the registry was fresh, or the probe is under no
    /// trigger boundary. Nothing was read.
    Fresh,
    /// A preparation ran and its registry loaded.
    Prepared,
    /// A preparation ran and failed (a trigger or schema root did not
    /// load), or the folder's context could not be built.
    Failed,
}

fn workspace_probes(
    workspace_roots: &[PathBuf],
    repositories: &[PathBuf],
    contexts: &crate::context::RepositoryContexts,
) -> Vec<PathBuf> {
    let mut probes: Vec<PathBuf> = Vec::new();
    let mut push = |probe: PathBuf| {
        if !probes.contains(&probe) {
            probes.push(probe);
        }
    };
    let nested = repositories
        .iter()
        .filter(|repository| workspace_roots.iter().any(|root| repository.starts_with(root)));
    for root in workspace_roots.iter().chain(nested) {
        let probe = root.join(WORKSPACE_PROBE);
        let resolution = contexts.for_document(&probe);
        push(probe);
        let Some(catalog) = resolution
            .context()
            .ok()
            .and_then(FileResolutionContext::repository_scope_catalog)
        else {
            continue;
        };
        for scope in catalog.package_area_roots().iter().chain(catalog.package_roots()) {
            if scope.starts_with(root) && scope != root {
                push(scope.join(WORKSPACE_PROBE));
            }
        }
        for area in fallback_areas(catalog, root, contexts) {
            push(area.join(WORKSPACE_PROBE));
        }
    }
    probes
}

/// The first-level directories of `catalog`'s repository, inside `root`,
/// that the catalog makes a package area only by its first-directory
/// fallback (a monorepo directory no listed package or area claims) and
/// whose package-area schema root [`SchemaRoots::for_document`] searches.
///
/// Costs one listing of the repository root and, per child directory, one
/// `schemas` metadata read; a child without one is never given a context or
/// a scope, so a directory with no schema root adds no work to preparation.
/// The context, not this function, decides the area: a child whose probe
/// context names another area (or none, outside a monorepo) is skipped.
fn fallback_areas(
    catalog: &biscuit_file::RepositoryScopeCatalog,
    root: &Path,
    contexts: &crate::context::RepositoryContexts,
) -> Vec<PathBuf> {
    let repository = catalog.repository_root();
    if !repository.starts_with(root) {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(repository) else {
        return Vec::new();
    };
    let mut children: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| entry.file_name() != ".git")
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .filter(|child| !catalog.package_area_roots().contains(child) && !catalog.package_roots().contains(child))
        .filter(|child| std::fs::symlink_metadata(child.join("schemas")).is_ok_and(|meta| meta.is_dir()))
        .collect();
    children.sort();
    children.retain(|child| {
        let resolution = contexts.for_document(&child.join(WORKSPACE_PROBE));
        let Ok(context) = resolution.context() else {
            return false;
        };
        if context.package_area() != Some(child.as_path()) {
            return false;
        }
        // An unreadable root keeps the probe, so preparation reports it.
        SchemaRoots::for_document(context).map_or(true, |roots| {
            roots.entries().iter().any(|entry| {
                entry.kind == SchemaRootKind::PackageArea && matches!(entry.state, SchemaRootState::Searched(_))
            })
        })
    });
    children
}

/// Content-and-config identity for the schema cache: the document text plus a
/// signature of the schema configuration (extensions + strict mode).
fn schema_cache_key(text: &str, config: &DmlsConfig) -> u64 {
    let config_bytes = serde_json::to_vec(&config.schema).unwrap_or_default();
    xx_hash_bytes(text.as_bytes()) ^ xx_hash_bytes(&config_bytes)
}

/// Reads and hashes each dependency of an assembled bundle, so a later lookup
/// can detect a content change. Covers the effective schema's dependencies
/// (imports, examples, referenced `$schema` file), the extension-baseline
/// files, and the files the named-type catalog read, which may name the same
/// file — dedup by path so each file is hashed once. The trigger envelopes and
/// payload files in `registry_files` are left out: the registry cache tracks
/// them. A failed assembly has no
/// bundle, so its edges come from the files its error names: without them a
/// fix made in an imported file would never clear the cached failure. A
/// dependency that cannot be read is omitted — `assemble` already read every
/// one, so an unreadable one is a transient race, and omitting it forces a
/// fresh assembly on the next lookup rather than trusting a stale bundle.
fn collect_dep_hashes(
    outcome: &SchemaOutcome,
    named_types: &NamedTypeCatalog,
    registry_files: &std::collections::HashSet<&Path>,
) -> Vec<(PathBuf, u64)> {
    let mut paths: Vec<PathBuf> = match outcome {
        SchemaOutcome::Ready(Some(bundle)) => bundle
            .effective
            .dependencies()
            .iter()
            .chain(bundle.extension_dependencies.iter())
            .cloned()
            .collect(),
        SchemaOutcome::Ready(None) => Vec::new(),
        SchemaOutcome::Failed(error) => error_dependencies(error),
    };
    paths.extend(named_types.dependencies().cloned());
    paths.retain(|path| !registry_files.contains(path.as_path()));
    hash_files(paths.iter())
}

/// Hashes each distinct readable file once.
fn hash_files<'a>(paths: impl Iterator<Item = &'a PathBuf>) -> Vec<(PathBuf, u64)> {
    let mut seen = std::collections::HashSet::new();
    paths
        .filter(|path| seen.insert((*path).clone()))
        .filter_map(|path| {
            work_counters::read(path)
                .ok()
                .map(|bytes| (path.clone(), work_counters::hash_bytes(&bytes)))
        })
        .collect()
}

/// The schema files a resolution error names: the referenced schema that
/// failed, the namespace searched for an unknown name, the file defining a
/// union a reference misuses, and every file that authored a reference or a
/// conflicting constraint.
fn error_dependencies(error: &SchemaError) -> Vec<PathBuf> {
    fn file(source: &SchemaSourceId, out: &mut Vec<PathBuf>) {
        if let SchemaSourceId::File(path) = source {
            out.push(path.clone());
        }
    }
    fn walk(error: &SchemaError, out: &mut Vec<PathBuf>) {
        match error {
            SchemaError::Aggregate { errors } => errors.iter().for_each(|error| walk(error, out)),
            SchemaError::ReferencedSchema { path, source } => {
                out.push(path.clone());
                walk(source, out);
            }
            SchemaError::UnknownNamedType { namespace, site, .. } => {
                file(namespace, out);
                file(&site.source, out);
            }
            SchemaError::ConstraintConflict { site, constraints, .. } => {
                file(&site.source, out);
                constraints.iter().for_each(|origin| file(&origin.source, out));
            }
            SchemaError::UnionReference { site, definition, .. } => {
                file(&site.source, out);
                file(&definition.source, out);
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(error, &mut out);
    out
}

/// The catalog of files a Markdown document's inline `$schema` imports by
/// explicit named-type reference. A `$schema` that is a file reference
/// authors no references of its own.
fn frontmatter_named_types(
    text: &str,
    path: &Path,
    schema_roots: &[PathBuf],
    context: &FileResolutionContext,
) -> NamedTypeCatalog {
    let markdown: darkmatter::markdown::Markdown = text.into();
    let Some(value) = markdown
        .frontmatter()
        .as_map()
        .get("$schema")
        .and_then(|value| serde_yaml_ng::to_value(value).ok())
    else {
        return NamedTypeCatalog::default();
    };
    let base_dir = path.parent().unwrap_or(path);
    NamedTypeCatalog::build_tolerant(&value, base_dir, schema_roots, context)
}

/// Whether every recorded dependency file still hashes to its recorded value. An
/// empty set (the no-dependency fast path) is trivially unchanged and reads
/// nothing; a missing or unreadable dependency counts as changed, forcing
/// reassembly.
fn deps_unchanged(deps: &[(PathBuf, u64)]) -> bool {
    deps.iter().all(|(path, hash)| {
        work_counters::read(path)
            .map(|bytes| work_counters::hash_bytes(&bytes) == *hash)
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
                    &crate::context::test_support::resolution_for(Path::new("/w/doc.md")),
                )
                .is_none()
        );
    }

    #[test]
    fn local_references_record_no_external_dependency() {
        let state = OverlayState::default();
        let root = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(root.path()).unwrap();
        let cases = [
            ("doc.md", "---\n$schema:\n  title: base\n  other: 'base@this'\n  base: string\n---\n"),
            ("types.yaml", "kind: schema\ntypes:\n  title: base\n  base: string\n"),
        ];
        for (name, text) in cases {
            let path = root.join(name);
            let uri_text = url::Url::from_file_path(&path).unwrap().to_string();
            let uri_text = uri_text.as_str();
            let path = path.as_path();
            let overlay = state
                .for_document(
                    &uri(uri_text),
                    text,
                    path,
                    &DmlsConfig::default(),
                    std::slice::from_ref(&root),
                    &crate::context::test_support::resolution_for(path),
                )
                .unwrap();
            let path = path.display();
            assert!(overlay.named_types.externals().is_empty(), "{path}");
            assert!(overlay.standalone_resolution.is_none(), "{path}");
            if name == "doc.md" {
                let bundle = overlay.bundle().expect("local references resolve");
                assert!(bundle.effective.dependencies().is_empty(), "{path}");
            }
            let cache = state.inner.lock().unwrap();
            let deps = cache
                .schema
                .get(uri_text)
                .map(|cached| cached.deps.len())
                .or_else(|| cache.standalone_resolution.get(uri_text).map(|cached| cached.deps.len()));
            assert_eq!(deps, Some(0), "{path}: a local reference reads no file");
        }
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
                &crate::context::test_support::resolution_for(Path::new("/w/doc.md")),
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
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&doc_path))
            .unwrap();
        let bundle1 = ready_bundle(&first);
        let report1 = bundle1.effective.validate(&bundle1.frontmatter_json);
        assert!(report1.valid, "`a` is a valid enum(a, b): {:?}", report1.problems);

        // Change the imported file only.
        std::fs::write(root.join("types.yaml"), "$schema:\n  type: 'enum(x, y)'\n").unwrap();

        let second = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&doc_path))
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

    fn property_names(overlay: &DocumentOverlay) -> Vec<String> {
        let bundle = ready_bundle(overlay);
        let mut names: Vec<String> = bundle.effective.json_schema["properties"]
            .as_object()
            .map(|properties| properties.keys().cloned().collect())
            .unwrap_or_default();
        names.retain(|name| name.starts_with("from_"));
        names.sort();
        names
    }

    /// Criterion 22: the same document text under two contexts whose `^`
    /// roots differ is assembled for each, not served from one cache entry.
    /// Both resolutions share a generation, so only the context itself can
    /// tell them apart. The document lies outside every workspace folder, so
    /// no trigger registry enters the key either.
    #[test]
    fn schema_cache_keys_on_the_package_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let package = root.join("pkg");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(root.join("shape.yaml"), "$schema:\n  from_repo: string\n").unwrap();
        std::fs::write(package.join("shape.yaml"), "$schema:\n  from_pkg: string\n").unwrap();
        let doc_path = package.join("doc.md");
        let text = "---\n$schema: ^shape.yaml\n---\n\nbody\n";
        let base = FileResolutionContext::from_snapshot(&root, None, HashMap::new())
            .with_repository_root(&root);
        let in_package = DocumentResolution::from_context(
            base.clone().with_package_root(&package).for_source(&doc_path),
        );
        let in_repository = DocumentResolution::from_context(base.for_source(&doc_path));
        let state = OverlayState::default();
        let doc = uri("file:///w/pkg/doc.md");

        let first = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &[], &in_package)
            .unwrap();
        assert_eq!(property_names(&first), ["from_pkg"]);
        let second = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &[], &in_repository)
            .unwrap();
        assert_eq!(property_names(&second), ["from_repo"], "a stale cache hit served `^` from the package");
        let third = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &[], &in_package)
            .unwrap();
        assert_eq!(property_names(&third), ["from_pkg"]);
    }

    /// Criterion 22: the same document text under two snapshots whose
    /// `SCHEMAS_DIR` differs resolves its bare-name `$schema` in each.
    #[test]
    fn schema_cache_keys_on_the_snapshot_environment() {
        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let repo = root.join("repo");
        let (defs_a, defs_b) = (root.join("defs-a"), root.join("defs-b"));
        for defs in [&repo, &defs_a, &defs_b] {
            std::fs::create_dir_all(defs).unwrap();
        }
        std::fs::write(defs_a.join("user.yaml"), "$schema:\n  from_a: string\n").unwrap();
        std::fs::write(defs_b.join("user.yaml"), "$schema:\n  from_b: string\n").unwrap();
        let doc_path = repo.join("doc.md");
        let text = "---\n$schema: user.yaml\n---\n\nbody\n";
        let resolution = |defs: &Path| {
            let env = HashMap::from([(
                "SCHEMAS_DIR".to_string(),
                defs.to_string_lossy().into_owned(),
            )]);
            DocumentResolution::from_context(
                FileResolutionContext::from_snapshot(&repo, None, env)
                    .with_repository_root(&repo)
                    .for_source(&doc_path),
            )
        };
        let state = OverlayState::default();
        let doc = uri("file:///w/repo/doc.md");
        let roots = [repo.clone()];

        let first = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &roots, &resolution(&defs_a))
            .unwrap();
        assert_eq!(property_names(&first), ["from_a"]);
        let second = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &roots, &resolution(&defs_b))
            .unwrap();
        assert_eq!(property_names(&second), ["from_b"], "a stale cache hit served the first SCHEMAS_DIR");
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
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&doc_path))
            .unwrap();
        let bundle1 = ready_bundle(&first);
        let report1 = bundle1.effective.validate(&bundle1.frontmatter_json);
        assert!(report1.valid, "`a` is a valid enum(a, b): {:?}", report1.problems);

        // Change the referenced schema file's own type only.
        std::fs::write(root.join("schema.yaml"), "$schema:\n  value: 'enum(x, y)'\n").unwrap();

        let second = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&doc_path))
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
    fn schema_cache_invalidates_on_terminal_file_in_a_reference_chain() {
        // `doc.md` -> `a.yaml` -> `b.yaml`: `a.yaml` is a pure redirect, so the
        // declaration the document actually validates against lives two hops
        // away. Every hop must reach `effective.dependencies()`, otherwise
        // editing the terminal file leaves the document validating against a
        // stale cached schema.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("a.yaml"), "$schema: ./b.yaml\n").unwrap();
        std::fs::write(root.join("b.yaml"), "$schema:\n  value: 'enum(a, b)'\n").unwrap();
        let doc_path = root.join("doc.md");
        let text = "---\n$schema: ./a.yaml\nvalue: a\n---\n\nbody\n";
        let state = OverlayState::default();
        let doc = uri("file:///w/doc.md");
        let roots = [root.to_path_buf()];

        let first = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&doc_path))
            .unwrap();
        let bundle1 = ready_bundle(&first);
        let report1 = bundle1.effective.validate(&bundle1.frontmatter_json);
        assert!(report1.valid, "`a` is a valid enum(a, b): {:?}", report1.problems);
        assert!(
            bundle1
                .effective
                .dependencies()
                .iter()
                .any(|p| p.ends_with("b.yaml")),
            "the terminal hop must be a dependency edge: {:?}",
            bundle1.effective.dependencies(),
        );

        // Edit the TERMINAL file only — `a.yaml` and the document are untouched.
        std::fs::write(root.join("b.yaml"), "$schema:\n  value: 'enum(x, y)'\n").unwrap();

        let second = state
            .for_document(&doc, text, &doc_path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&doc_path))
            .unwrap();
        let bundle2 = ready_bundle(&second);
        assert!(
            !Arc::ptr_eq(&bundle1, &bundle2),
            "a terminal-hop change must reassemble the bundle"
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

        let first = state.for_document(&doc, text, &doc_path, &config, &roots, &crate::context::test_support::resolution_for(&doc_path)).unwrap();
        let bundle1 = ready_bundle(&first);
        let report1 = bundle1.effective.validate(&bundle1.frontmatter_json);
        assert!(report1.valid, "`a` is a valid enum(a, b): {:?}", report1.problems);

        // Change the extension baseline file's declared type only.
        std::fs::write(root.join("ext.yaml"), "$schema:\n  provider: 'enum(x, y)'\n").unwrap();

        let second = state.for_document(&doc, text, &doc_path, &config, &roots, &crate::context::test_support::resolution_for(&doc_path)).unwrap();
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
            .for_document(&doc, text, path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(path))
            .unwrap();
        let second = state
            .for_document(&doc, text, path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(path))
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
            .for_document(&doc, "---\ntitle: Good\n---\n\nbody\n", path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(path))
            .unwrap();
        // Then a hard YAML error keeps the previous tree, flagged stale, and
        // surfaces the parse error.
        let overlay = state
            .for_document(&doc, "---\nkey:\n\tbad: 1\n---\n\nbody\n", path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(path))
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
                    &crate::context::test_support::resolution_for(path),
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
                &crate::context::test_support::resolution_for(Path::new("/w/doc.md")),
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
                    &crate::context::test_support::resolution_for(path),
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
                        &crate::context::test_support::resolution_for(path),
                    )
                    .is_none(),
                "path-only activation is forbidden for {path:?}"
            );
        }
    }

    /// Escape-bearing scalars once let the lexical claim over-reach on
    /// well-formed ordinary YAML, and the resulting disagreement with the
    /// authoritative parser reached an `expect_err` on a successful parse.
    /// Such a disagreement must deactivate, never crash.
    #[test]
    fn escaped_quotes_stay_inert_without_panicking() {
        let state = OverlayState::default();
        let roots = [PathBuf::from("/w")];
        for (uri_text, path, text) in [
            (
                "file:///w/raw.schema.json",
                Path::new("/w/raw.schema.json"),
                r#"{"$schema":"https://example.com/quo\"ted","type":"object"}"#,
            ),
            (
                "file:///w/quoted.yaml",
                Path::new("/w/quoted.yaml"),
                "description: \"some text\nkind: schema\n\"\n",
            ),
            (
                "file:///w/single.yaml",
                Path::new("/w/single.yaml"),
                "description: 'multi\nkind: schema\n'\n",
            ),
        ] {
            assert!(
                state
                    .for_document(&uri(uri_text), text, path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(path))
                    .is_none(),
                "escape-bearing ordinary YAML must stay inert: {path:?}"
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
            .for_document(&doc, good, path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(path))
            .expect("valid schema overlay");
        let SchemaAuthoringState::Standalone { model: Some(first_model), .. } =
            &first.schema_authoring
        else {
            panic!("valid standalone model");
        };

        for malformed in ["$schema:\n\tbad: string\n", "$schema: 42\n"] {
            let current = state
                .for_document(&doc, malformed, path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(path))
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
                &crate::context::test_support::resolution_for(Path::new("/w/fresh.yaml")),
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
                &crate::context::test_support::resolution_for(path),
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
                &crate::context::test_support::resolution_for(path),
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
        let root = biscuit_test_harness::manifest_dir!().join("../docs/schemas");
        let expected = std::collections::BTreeMap::from([
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
                &crate::context::test_support::resolution_for(&path),
            ) {
                Some(DocumentOverlay {
                    schema_authoring:
                        SchemaAuthoringState::Standalone { model: Some(_), error: None, .. },
                    ..
                }) => "ready",
                Some(DocumentOverlay {
                    schema_authoring:
                        SchemaAuthoringState::Standalone { error: Some(_), .. },
                    ..
                }) => "error",
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
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: prompt.yaml\n",
        )
        .unwrap();
    }

    fn copy_dialect_fixture(root: &Path) {
        let source = biscuit_test_harness::manifest_dir!()
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
            .for_document(&doc, text, &path, &DmlsConfig::default(), &[root.to_path_buf()], &crate::context::test_support::resolution_for(&path))
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
                &crate::context::test_support::resolution_for(&outside),
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
            .for_document(&doc, good, &path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&path))
            .unwrap();
        assert!(!ready_bundle(&first).effective.validate(&ready_bundle(&first).frontmatter_json).valid);

        let broken = "---\nprompt:\n\tbad: yaml\n---\n";
        let stale = state
            .for_document(&doc, broken, &path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&path))
            .unwrap();
        assert!(stale.stale);
        assert!(
            !ready_bundle(&stale).effective.validate(&ready_bundle(&stale).frontmatter_json).valid,
            "a malformed edit must retain the prior activation"
        );

        std::fs::write(
            root.join("schemas/prompt.trigger.yaml"),
            "kind: schema-trigger\nmatch: {}\n",
        )
        .unwrap();
        state.invalidate_trigger_inputs(&[root.join("schemas/prompt.trigger.yaml")]);
        let retained = state
            .for_document(&doc, good, &path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&path))
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
    fn repeated_overlay_builds_skip_the_registry_but_keep_schema_freshness() {
        use crate::work_counters::work_counts;

        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        write_trigger_fixture(&root);
        let local = root.join("local.yaml");
        std::fs::write(&local, "$schema:\n  owner: string(required)\n").unwrap();
        let path = root.join("doc.md");
        let text = "---\nprompt: hello\n$schema: ./local.yaml\n---\n\nbody\n";
        let state = OverlayState::default();
        let doc: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
        let roots = [root.clone()];
        let resolution = crate::context::test_support::resolution_in_repository(&root, &path);
        let missing = |overlay: &DocumentOverlay| -> Vec<String> {
            let bundle = ready_bundle(overlay);
            bundle
                .effective
                .validate(&bundle.frontmatter_json)
                .problems
                .into_iter()
                .map(|problem| problem.message)
                .collect()
        };
        let build = || {
            state
                .for_document(&doc, text, &path, &DmlsConfig::default(), &roots, &resolution)
                .unwrap()
        };
        let first = missing(&build());
        assert!(first.iter().any(|message| message.contains("model")), "trigger payload: {first:?}");
        assert!(first.iter().any(|message| message.contains("owner")), "document schema: {first:?}");

        let before = work_counts();
        for _ in 0..3 {
            build();
        }
        let work = work_counts().since(before);
        // Only the document's own `$schema` file is re-hashed per build; the
        // trigger payload and the schema roots are not touched.
        assert_eq!((work.registry_scans, work.enumerations, work.reads), (0, 0, 3), "{work:?}");

        // The document's `$schema` file keeps its read-on-lookup freshness: an
        // edit with no watched event still re-assembles.
        std::fs::write(&local, "$schema:\n  reviewer: string(required)\n").unwrap();
        let edited = missing(&build());
        assert!(edited.iter().any(|message| message.contains("reviewer")), "{edited:?}");
        assert!(edited.iter().any(|message| message.contains("model")), "{edited:?}");
    }

    #[test]
    fn the_server_overlay_build_only_looks_registries_up() {
        use crate::work_counters::work_counts;

        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        write_trigger_fixture(&root);
        let path = root.join("doc.md");
        let text = "---\nprompt: hello\n---\n\nbody\n";
        let state = OverlayState::default();
        let doc: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
        let roots = [root.clone()];
        let resolution = crate::context::test_support::resolution_in_repository(&root, &path);
        let build = || {
            state
                .for_document_timed(&doc, text, &path, &DmlsConfig::default(), &roots, &resolution, &RequestTimings::untimed())
                .unwrap()
        };
        let triggered = |overlay: &DocumentOverlay| {
            let bundle = ready_bundle(overlay);
            !bundle.effective.validate(&bundle.frontmatter_json).valid
        };

        let before = work_counts();
        assert!(!triggered(&build()), "no registry is installed yet, and the build prepares none");
        assert_eq!(work_counts().since(before).registry_scans, 0);

        let need = state.registry_demand(text, &path, &roots, &resolution).expect("not prepared");
        assert_eq!(need.path, path);
        assert!(state.prepare_registry(&need.path, &roots, &need.context));
        assert!(state.registry_demand(text, &path, &roots, &resolution).is_none(), "fresh");
        assert!(triggered(&build()), "the installed registry's trigger applies");
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
            .for_document(&doc, good, &path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&path))
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
        state.invalidate_trigger_inputs(&[root.join("schemas/prompt.yaml")]);

        let retained = state
            .for_document(&doc, good, &path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&path))
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
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: a.yaml\n",
        )
        .unwrap();
        std::fs::write(&a_payload, "$schema:\n  model: string(required)\n").unwrap();
        std::fs::write(
            &b_envelope,
            "kind: schema-trigger\nmatch:\n  task: string(required)\n$schema: b.yaml\n",
        )
        .unwrap();
        std::fs::write(&b_payload, "$schema:\n  provider: string(required)\n").unwrap();

        let path = root.join("doc.md");
        let text = "---\nprompt: hello\n---\n\nbody\n";
        let state = OverlayState::default();
        let doc: Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
        let roots = [root.to_path_buf()];

        let initial = state
            .for_document(&doc, text, &path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&path))
            .unwrap();
        let initial_report =
            ready_bundle(&initial).effective.validate(&ready_bundle(&initial).frontmatter_json);
        assert!(
            initial_report.problems.iter().any(|problem| problem.message.contains("model")),
            "the matching trigger must contribute to the last-good schema: {:?}",
            initial_report.problems
        );

        std::fs::remove_file(&a_payload).unwrap();
        state.invalidate_trigger_inputs(std::slice::from_ref(&a_payload));
        let failed_a = state
            .for_document(&doc, text, &path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&path))
            .unwrap();
        let transitions = state.take_trigger_diagnostic_transitions();
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].path, a_envelope);
        assert!(transitions[0].error.is_some());

        std::fs::write(&a_payload, "$schema:\n  model: string(required)\n").unwrap();
        std::fs::remove_file(&b_payload).unwrap();
        state.invalidate_trigger_inputs(&[a_payload.clone(), b_payload.clone()]);
        let failed_b = state
            .for_document(&doc, text, &path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_for(&path))
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
                .for_document(&doc, &text, &path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_in_repository(root, &path))
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
            .for_document(&doc, &text, &path, &DmlsConfig::default(), &roots, &crate::context::test_support::resolution_in_repository(root, &path))
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
                &crate::context::test_support::resolution_in_repository(root, &path),
            )
            .unwrap();
        let SchemaOutcome::Failed(error) = overlay.schema else {
            panic!("sibling-only bare reference must fail schema preparation");
        };
        assert!(error.to_string().contains("./local.yaml"));
    }
}
