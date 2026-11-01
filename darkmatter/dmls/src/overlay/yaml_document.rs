//! The overlay state of a whole YAML document (`.yaml`, `.yml`).
//!
//! A YAML document is judged by the schema it selects, through the
//! library's one entry point: its claimed `kind` selects that kind's
//! definition, otherwise a root `$schema` is the contract, and a document
//! with neither is not validated
//! ([`DarkmatterSchemas::validate_loaded_yaml_document`]). The kinds come
//! from the document's [`KindRegistry`], prepared on the registry worker
//! beside its trigger registry ([`super::registry`]); this module only looks
//! that registry up, so building an overlay never scans a schema root.
//!
//! The selected schema's report, the whole document's tree
//! ([`FrontmatterAst::parse_yaml_document`]), and a [`SchemaBundle`] holding
//! the selected schema alone (no Markdown baseline, no trigger layer) are
//! cached per document. Diagnostics project the report
//! ([`crate::diagnostics::yaml_document`]); completion and hover read the
//! bundle and tree through the shared [`FrontmatterAnalysis`], whose root
//! shape is the selected schema's alone.
//!
//! The cached state is reused while the text, the buffer version, the
//! document's context, the registry version, and every file the selected
//! schema read (and every location a `$schema` lookup probed) are unchanged.
//! Any change rebuilds it whole, so a changed or removed `kind` or `$schema`
//! never keeps an earlier selection's results.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use biscuit_hash::xx_hash_bytes;
use darkmatter::markdown::compose::file_resolution_context_identity;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, KindRegistry, LoadedYamlDocument, Route, YamlOutcome, YamlValidation,
};
use simplified_schema::SchemaError;

use super::analysis::{AnalysisIdentity, CrossSnapshotSchema, FrontmatterAnalysis};
use super::references::SchemaReferenceFacts;
use super::registry::InstalledKinds;
use super::{
    FileFailures, FrontmatterAst, NamedTypeCatalog, OverlayCache, SchemaBundle, deps_unchanged, error_dependencies,
    file_failures, hash_files, probes_unchanged,
};
use crate::context::DocumentResolution;

/// What judging one YAML document produced.
pub enum YamlDocumentOutcome {
    /// The text is not one admissible YAML document: the loader's located
    /// [`SchemaError::SchemaDocument`]. Nothing was validated.
    LoadFailed(Arc<SchemaError>),
    /// The document has no file-resolution context, so no schema was
    /// selected (its context-failure diagnostic says why).
    Unvalidated,
    /// No schema was selected: the root is not a mapping, or it has neither
    /// a claimed `kind` nor a `$schema`.
    NoSchema,
    /// Selecting or preparing the schema failed: an ambiguous, reserved, or
    /// unavailable `kind`, or a root `$schema` that does not resolve.
    Failed(Arc<SchemaError>),
    /// The document was judged against its selected schema.
    Validated(Arc<YamlValidation>),
}

/// The YAML document state a [`DocumentOverlay`](super::DocumentOverlay)
/// carries.
pub struct YamlDocumentOverlay {
    /// What judging the current buffer produced.
    pub outcome: YamlDocumentOutcome,
}

impl YamlDocumentOverlay {
    /// The validated document, when a schema was selected and prepared.
    pub fn validation(&self) -> Option<&YamlValidation> {
        match &self.outcome {
            YamlDocumentOutcome::Validated(validation) => Some(validation),
            _ => None,
        }
    }

    /// Whether the document's `kind` selected its schema, so its root
    /// `$schema` (if any) is data rather than a reference.
    pub fn kind_route(&self) -> bool {
        matches!(self.validation().map(|validation| &validation.route), Some(Route::Kind(_)))
            || matches!(&self.outcome, YamlDocumentOutcome::Failed(error) if matches!(**error, SchemaError::KindSelection { .. }))
    }
}

/// One YAML document's cached state and what it was built from.
pub(super) struct CachedYaml {
    key: u64,
    /// Every file the selected schema (or its failure) read, with its
    /// xxHash; a hit requires each to be unchanged.
    deps: Vec<(PathBuf, u64)>,
    /// Every location a root `$schema` lookup probed, with whether a file
    /// existed there.
    probes: Vec<(PathBuf, bool)>,
    state: YamlState,
}

/// The overlay parts of one YAML document.
#[derive(Clone)]
pub(super) struct YamlState {
    pub(super) overlay: Arc<YamlDocumentOverlay>,
    /// The whole document's tree; on a load failure, the last tree that
    /// loaded (then [`Self::stale`]).
    pub(super) ast: Option<Arc<FrontmatterAst>>,
    pub(super) stale: bool,
    /// The selected schema alone, with the document as data.
    pub(super) bundle: Option<Arc<SchemaBundle>>,
    pub(super) analysis: Arc<FrontmatterAnalysis>,
    pub(super) schema_references: Arc<SchemaReferenceFacts>,
    pub(super) file_failures: FileFailures,
}

impl OverlayCache {
    /// The YAML document state of `text`, rebuilt only when an input changed.
    ///
    /// `kinds` is the document's installed registry, or `None` outside a
    /// workspace folder, where only the shipped kinds apply.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn yaml_document(
        &mut self,
        uri_key: &str,
        revision: Option<i32>,
        text: &str,
        path: &Path,
        resolution: &DocumentResolution,
        kinds: Option<InstalledKinds>,
    ) -> YamlState {
        let context_key = match resolution.context() {
            Ok(context) => {
                file_resolution_context_identity(context) ^ xx_hash_bytes(resolution.generation().to_string().as_bytes())
            }
            Err(_) => 0,
        };
        let key = xx_hash_bytes(text.as_bytes())
            ^ context_key
            ^ xx_hash_bytes(&revision.unwrap_or(i32::MIN).to_le_bytes())
            ^ kinds.as_ref().map_or(0, |kinds| xx_hash_bytes(&kinds.version.to_le_bytes()));
        if let Some(cached) = self.yaml_documents.get(uri_key)
            && cached.key == key
            && deps_unchanged(&cached.deps)
            && probes_unchanged(&cached.probes)
        {
            return cached.state.clone();
        }
        let last_good = self
            .yaml_documents
            .get(uri_key)
            .map(|cached| cached.state.clone())
            .filter(|state| state.ast.is_some());
        let shared = Arc::clone(&self.shared_schema);
        let Built { state, deps, probes } = build(text, path, revision, key, resolution, kinds, last_good, &shared);
        self.yaml_documents.insert(uri_key.to_owned(), CachedYaml { key, deps, probes, state: state.clone() });
        state
    }
}

/// A freshly built [`YamlState`] and the inputs a later hit re-checks.
struct Built {
    state: YamlState,
    deps: Vec<(PathBuf, u64)>,
    probes: Vec<(PathBuf, bool)>,
}

impl Built {
    /// A state that read no file.
    fn alone(state: YamlState) -> Self {
        Self { state, deps: Vec::new(), probes: Vec::new() }
    }
}

#[allow(clippy::too_many_arguments)]
fn build(
    text: &str,
    path: &Path,
    revision: Option<i32>,
    key: u64,
    resolution: &DocumentResolution,
    kinds: Option<InstalledKinds>,
    last_good: Option<YamlState>,
    shared: &Arc<CrossSnapshotSchema>,
) -> Built {
    let loaded = match LoadedYamlDocument::load(text, path) {
        Ok(loaded) => loaded,
        Err(error) => {
            let error = YamlDocumentOutcome::LoadFailed(Arc::new(error));
            // The last tree that loaded keeps completion and hover alive
            // while the buffer is mid-edit; nothing is validated.
            return Built::alone(match last_good {
                Some(previous) => YamlState { overlay: Arc::new(YamlDocumentOverlay { outcome: error }), stale: true, ..previous },
                None => empty_state(error, revision, text, key, shared),
            });
        }
    };
    let Ok(context) = resolution.context() else {
        return Built::alone(parsed_state(YamlDocumentOutcome::Unvalidated, None, revision, text, key, shared));
    };
    let context: FileResolutionContext = context.for_source(path);
    let registry = kinds.map_or_else(KindRegistry::shipped, |kinds| kinds.registry);
    let schema_roots = registry.roots().search_paths().to_vec();
    let schemas = DarkmatterSchemas::new(context.clone()).with_kind_registry(registry);
    let outcome = match schemas.validate_loaded_yaml_document(&loaded) {
        Ok(YamlOutcome::NoSchema) => YamlDocumentOutcome::NoSchema,
        Ok(YamlOutcome::Validated(validation)) => YamlDocumentOutcome::Validated(Arc::new(validation)),
        Err(error) => YamlDocumentOutcome::Failed(Arc::new(error)),
    };
    let bundle = match &outcome {
        YamlDocumentOutcome::Validated(validation) => Some(Arc::new(bundle(validation))),
        _ => None,
    };
    let mut deps: Vec<PathBuf> = match (&outcome, &bundle) {
        // A definition's own files are the registry's to track.
        (YamlDocumentOutcome::Validated(validation), Some(bundle)) if validation.route == Route::RootSchema => {
            bundle.effective.dependencies().to_vec()
        }
        (YamlDocumentOutcome::Failed(error), _) => error_dependencies(error),
        _ => Vec::new(),
    };
    let failures = match &outcome {
        YamlDocumentOutcome::Failed(error) => file_failures(error),
        _ => FileFailures::default(),
    };
    let mut state = parsed_state(outcome, bundle, revision, text, key, shared);
    state.file_failures = failures;
    let assembled = state.bundle.is_some();
    if let Some(ast) = state.ast.as_deref()
        && ast.schema_entry().is_some()
    {
        let facts = SchemaReferenceFacts::select_in(ast, text, path, Ok(&schema_roots), &context, assembled);
        deps.extend(facts.selected().cloned());
        state.schema_references = Arc::new(facts);
    }
    let probes = state.schema_references.probes();
    Built { state, deps: hash_files(deps.iter()), probes }
}

/// The selected schema alone as the frontmatter providers read it. On the
/// root-`$schema` route the document's `$schema` names the schema and is not
/// data, as in Markdown frontmatter; on the kind route it is data.
fn bundle(validation: &YamlValidation) -> SchemaBundle {
    let effective = validation.effective();
    let mut document = validation.document_json().clone();
    if validation.route == Route::RootSchema
        && let serde_json::Value::Object(map) = &mut document
    {
        map.shift_remove("$schema");
    }
    SchemaBundle {
        advisories: effective.advisories().to_vec(),
        effective: (**effective).clone(),
        frontmatter_json: document,
        extension_shapes: Vec::new(),
        extension_dependencies: Vec::new(),
        extension_definitions: Vec::new(),
    }
}

/// The state of a document that loaded: its tree, parsed with `$schema` as
/// data when its `kind` selected (or failed to select) the schema.
fn parsed_state(
    outcome: YamlDocumentOutcome,
    bundle: Option<Arc<SchemaBundle>>,
    revision: Option<i32>,
    text: &str,
    key: u64,
    shared: &Arc<CrossSnapshotSchema>,
) -> YamlState {
    let overlay = YamlDocumentOverlay { outcome };
    let ast = FrontmatterAst::parse_yaml_document(text, overlay.kind_route()).ast.map(Arc::new);
    let analysis = analysis(revision, text, key, bundle.as_ref(), ast.as_ref(), shared);
    YamlState {
        overlay: Arc::new(overlay),
        ast,
        stale: false,
        bundle,
        analysis,
        schema_references: Arc::default(),
        file_failures: FileFailures::default(),
    }
}

/// The state of a document that never loaded: no tree to serve.
fn empty_state(
    outcome: YamlDocumentOutcome,
    revision: Option<i32>,
    text: &str,
    key: u64,
    shared: &Arc<CrossSnapshotSchema>,
) -> YamlState {
    YamlState {
        overlay: Arc::new(YamlDocumentOverlay { outcome }),
        ast: None,
        stale: false,
        bundle: None,
        analysis: analysis(revision, text, key, None, None, shared),
        schema_references: Arc::default(),
        file_failures: FileFailures::default(),
    }
}

fn analysis(
    revision: Option<i32>,
    text: &str,
    key: u64,
    bundle: Option<&Arc<SchemaBundle>>,
    ast: Option<&Arc<FrontmatterAst>>,
    shared: &Arc<CrossSnapshotSchema>,
) -> Arc<FrontmatterAnalysis> {
    let text_hash = xx_hash_bytes(text.as_bytes());
    let identity = AnalysisIdentity {
        revision,
        text_hash,
        stale: false,
        tree_text_hash: text_hash,
        schema_key: Some(key),
        bundle: bundle.cloned(),
        named_types: Arc::new(NamedTypeCatalog::default()),
        yaml_document: true,
    };
    Arc::new(FrontmatterAnalysis::new(identity, ast.cloned(), Arc::clone(shared)))
}
