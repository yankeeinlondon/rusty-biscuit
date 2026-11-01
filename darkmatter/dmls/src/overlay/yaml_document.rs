//! The overlay state of a whole YAML document (`.yaml`, `.yml`).
//!
//! A YAML document is judged by the schema it selects, through the
//! library's one entry point: its claimed `kind` selects that kind's
//! definition, otherwise a root `$schema` is the contract, and a document
//! with neither is not validated
//! ([`DarkmatterSchemas::prepare_yaml_contract`](darkmatter::markdown::schemas::DarkmatterSchemas::prepare_yaml_contract)).
//! Selection, schema resolution, validator compilation, and every other step
//! that reads a file run on the registry worker, beside the document's kind
//! registry ([`super::yaml_contract`]); this module only looks the prepared
//! contract up and judges the current data against it, passively, so
//! building an overlay reads no file.
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
//! document's context, and the prepared contract are unchanged. A contract
//! prepared for another declaration is never used, so a changed or removed
//! `kind` or `$schema` never keeps an earlier selection's results.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_hash::xx_hash_bytes;
use darkmatter::markdown::compose::file_resolution_context_identity;
use darkmatter::markdown::schemas::{LoadedYamlDocument, Route, YamlValidation};
use simplified_schema::{SchemaError, SuggestionLintProblem};
use simplified_schema::resolve::buffers::SourceBuffers;

use super::analysis::{AnalysisIdentity, CrossSnapshotSchema, FrontmatterAnalysis};
use super::references::SchemaReferenceFacts;
use super::registry::InstalledKinds;
use super::yaml_contract::PreparedYaml;
use super::{FileFailures, FrontmatterAst, NamedTypeCatalog, OverlayCache, SchemaBundle, file_failures, suggestions};
use crate::context::DocumentResolution;
use crate::work_counters;

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
    /// The document's contract is not prepared for its current declaration
    /// yet: nothing is judged until the registry worker installs it. The
    /// server waits for that before publishing diagnostics or answering a
    /// schema-dependent request.
    Pending,
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
    /// The files a root-`$schema` contract's named types import.
    pub(super) named_types: Arc<NamedTypeCatalog>,
    /// The `suggest(...)` lint problems of an inline root-`$schema`
    /// contract, in the current text's offsets.
    pub(super) suggestions: Vec<SuggestionLintProblem>,
}

impl OverlayCache {
    /// The YAML document state of `text`, rebuilt only when an input changed.
    ///
    /// `kinds` is the document's installed registry, or `None` outside a
    /// workspace folder, where only the shipped kinds apply. Reads no file:
    /// the document's contract is looked up among the prepared ones
    /// ([`super::yaml_contract`]), and the document is
    /// [`Pending`](YamlDocumentOutcome::Pending) until one is installed for
    /// its declaration.
    pub(super) fn yaml_document(
        &mut self,
        uri_key: &str,
        revision: Option<i32>,
        text: &str,
        path: &Path,
        resolution: &DocumentResolution,
        kinds: Option<InstalledKinds>,
    ) -> YamlState {
        let prepared = resolution
            .context()
            .ok()
            .and_then(|context| self.contracts.key(text, path, context, resolution.generation(), kinds.as_ref()))
            .and_then(|key| self.contracts.lookup(path, key))
            .cloned();
        let context_key = match resolution.context() {
            Ok(context) => file_resolution_context_identity(context),
            Err(_) => 0,
        };
        let mut bytes = Vec::with_capacity(40);
        bytes.extend_from_slice(&xx_hash_bytes(text.as_bytes()).to_le_bytes());
        bytes.extend_from_slice(&context_key.to_le_bytes());
        bytes.extend_from_slice(&resolution.generation().to_le_bytes());
        bytes.extend_from_slice(&revision.unwrap_or(i32::MIN).to_le_bytes());
        bytes.extend_from_slice(&prepared.as_ref().map_or(0, |prepared| prepared.version).to_le_bytes());
        let key = xx_hash_bytes(&bytes);
        if let Some(cached) = self.yaml_documents.get(uri_key)
            && cached.key == key
        {
            return cached.state.clone();
        }
        let last_good = self
            .yaml_documents
            .get(uri_key)
            .map(|cached| cached.state.clone())
            .filter(|state| state.ast.is_some());
        let state = build(text, path, revision, key, resolution, prepared.as_deref(), last_good, &self.shared_schema);
        self.yaml_documents.insert(uri_key.to_owned(), CachedYaml { key, state: state.clone() });
        state
    }
}

#[allow(clippy::too_many_arguments)]
fn build(
    text: &str,
    path: &Path,
    revision: Option<i32>,
    key: u64,
    resolution: &DocumentResolution,
    prepared: Option<&PreparedYaml>,
    last_good: Option<YamlState>,
    shared: &Arc<CrossSnapshotSchema>,
) -> YamlState {
    let loaded = match LoadedYamlDocument::load(text, path) {
        Ok(loaded) => loaded,
        Err(error) => {
            let error = YamlDocumentOutcome::LoadFailed(Arc::new(error));
            // The last tree that loaded keeps completion and hover alive
            // while the buffer is mid-edit; nothing is validated.
            return match last_good {
                Some(previous) => YamlState {
                    overlay: Arc::new(YamlDocumentOverlay { outcome: error }),
                    stale: true,
                    // Located in the last text that loaded.
                    suggestions: Vec::new(),
                    ..previous
                },
                None => empty_state(error, revision, text, key, shared),
            };
        }
    };
    if resolution.context().is_err() {
        return parsed_state(YamlDocumentOutcome::Unvalidated, None, revision, text, key, shared);
    }
    let Some(prepared) = prepared else {
        return parsed_state(YamlDocumentOutcome::Pending, None, revision, text, key, shared);
    };
    let outcome = match (&prepared.contract, &prepared.validation) {
        (Err(error), _) => YamlDocumentOutcome::Failed(Arc::clone(error)),
        (Ok(None), _) => YamlDocumentOutcome::NoSchema,
        // Judged on the worker: its kind checks read files.
        (Ok(Some(_)), Some(validation)) => YamlDocumentOutcome::Validated(Arc::clone(validation)),
        (Ok(Some(contract)), None) => YamlDocumentOutcome::Validated(Arc::new(contract.validate(&loaded))),
    };
    let bundle = match &outcome {
        YamlDocumentOutcome::Validated(validation) => Some(Arc::new(bundle(validation))),
        _ => None,
    };
    let failures = match &outcome {
        YamlDocumentOutcome::Failed(error) => file_failures(error),
        _ => FileFailures::default(),
    };
    let mut state = parsed_state(outcome, bundle, revision, text, key, shared);
    state.file_failures = failures;
    state.schema_references = Arc::clone(&prepared.schema_references);
    state.named_types = Arc::clone(&prepared.named_types);
    state
}

/// The xxHash of `path`'s text: its open buffer's, else the file's.
pub(super) fn source_hash(path: &Path, sources: &SourceBuffers) -> Option<u64> {
    match sources.get(path) {
        Some(text) => Some(work_counters::hash_bytes(text.as_bytes())),
        None => work_counters::read(path).ok().map(|bytes| work_counters::hash_bytes(&bytes)),
    }
}

/// Whether a file exists at `path`, on disk or as an open buffer.
pub(super) fn present(path: &Path, sources: &SourceBuffers) -> bool {
    path.is_file() || sources.get(path).is_some()
}

/// Hashes each distinct readable source once.
pub(super) fn hash_files<'a>(paths: impl Iterator<Item = &'a PathBuf>, sources: &SourceBuffers) -> Vec<(PathBuf, u64)> {
    let mut seen = std::collections::HashSet::new();
    paths
        .filter(|path| seen.insert((*path).clone()))
        .filter_map(|path| source_hash(path, sources).map(|hash| (path.clone(), hash)))
        .collect()
}

/// Whether every recorded source still hashes to its recorded value; a
/// missing source counts as changed.
pub(super) fn deps_unchanged(deps: &[(PathBuf, u64)], sources: &SourceBuffers) -> bool {
    deps.iter().all(|(path, hash)| source_hash(path, sources) == Some(*hash))
}

/// Whether a source is present at every probed location exactly where it was.
pub(super) fn probes_unchanged(probes: &[(PathBuf, bool)], sources: &SourceBuffers) -> bool {
    probes.iter().all(|(path, existed)| present(path, sources) == *existed)
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
    let suggestions = ast.as_deref().map(|ast| suggestions::yaml_document_lints(text, ast)).unwrap_or_default();
    YamlState {
        overlay: Arc::new(overlay),
        ast,
        stale: false,
        bundle,
        analysis,
        schema_references: Arc::default(),
        file_failures: FileFailures::default(),
        named_types: Arc::default(),
        suggestions,
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
        named_types: Arc::default(),
        suggestions: Vec::new(),
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
