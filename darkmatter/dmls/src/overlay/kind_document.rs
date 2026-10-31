//! The overlay state of a YAML document validated by its top-level `kind`.
//!
//! A buffer whose YAML mapping names a shipped kind (`kind: schema`,
//! `kind: schema-trigger`) is validated against that kind's definition through
//! [`DarkmatterSchemas::validate_kind_document`], whatever the Rust envelope
//! parsers make of it. The overlay then serves it like frontmatter: the tree
//! is the whole document ([`FrontmatterAst::parse_yaml_document`], whose
//! `$schema` is data rather than a reference), the bundle holds the kind's
//! effective schema, and the shared analysis projects the definition's shape
//! alone, so completion and hover offer the definition's keys and
//! descriptions. Diagnostics come from the report
//! ([`crate::diagnostics::kind_document`]).

use std::path::Path;
use std::sync::Arc;

use biscuit_hash::xx_hash_bytes;
use darkmatter::markdown::compose::file_resolution_context_identity;
use darkmatter::markdown::schemas::{DarkmatterSchemas, ValidationReport};

use super::analysis::{AnalysisIdentity, FrontmatterAnalysis};
use super::{FrontmatterAst, NamedTypeCatalog, OverlayCache, SchemaBundle};
use crate::context::DocumentResolution;

/// A claimed kind document's validation and the tree and bundle the
/// frontmatter providers read for it.
pub struct KindDocumentOverlay {
    /// The `kind` that claimed the document.
    pub kind: String,
    /// The raw validation report against the kind's definition, with problems
    /// located in the buffer text.
    pub report: ValidationReport,
    /// The whole document's tree.
    pub ast: Arc<FrontmatterAst>,
    /// The kind's effective schema, with the document as data.
    pub bundle: Arc<SchemaBundle>,
}

/// One document's kind state, reused while its text and context are unchanged
/// so the analysis built over it survives between requests.
pub(super) struct CachedKind {
    key: u64,
    state: Option<KindState>,
}

/// A claimed kind document's overlay and the analysis built over it.
#[derive(Clone)]
pub(super) struct KindState {
    pub(super) overlay: Arc<KindDocumentOverlay>,
    pub(super) analysis: Arc<FrontmatterAnalysis>,
}

impl OverlayCache {
    /// The kind state of `text`, or `None` when the buffer is not a YAML
    /// mapping claimed by a shipped kind, does not load, or has no context.
    pub(super) fn kind_document(
        &mut self,
        uri_key: &str,
        revision: Option<i32>,
        text: &str,
        path: &Path,
        resolution: &DocumentResolution,
    ) -> Option<KindState> {
        // The validator needs a context to be built in; a document without
        // one is told why by its context-failure diagnostic.
        let Ok(context) = resolution.context() else {
            self.kinds.remove(uri_key);
            return None;
        };
        let key = xx_hash_bytes(text.as_bytes())
            ^ file_resolution_context_identity(context)
            ^ xx_hash_bytes(resolution.generation().to_string().as_bytes())
            ^ xx_hash_bytes(&revision.unwrap_or(i32::MIN).to_le_bytes());
        if let Some(cached) = self.kinds.get(uri_key)
            && cached.key == key
        {
            return cached.state.clone();
        }
        let state = build(text, path, revision, key, context.clone(), &self.shared_schema);
        self.kinds.insert(uri_key.to_owned(), CachedKind { key, state: state.clone() });
        state
    }
}

fn build(
    text: &str,
    path: &Path,
    revision: Option<i32>,
    key: u64,
    context: biscuit_file::FileResolutionContext,
    shared: &Arc<super::analysis::CrossSnapshotSchema>,
) -> Option<KindState> {
    let schemas = DarkmatterSchemas::new(context);
    // A buffer that does not load is reported by its YAML parse diagnostic.
    let validation = schemas.validate_kind_document(text, path).ok()??;
    let effective = match schemas.effective_for_kind(&validation.kind) {
        Ok(Some(effective)) => effective,
        Ok(None) => return None,
        Err(error) => {
            // A shipped definition always compiles (the library's tests
            // prove it), so this is a defect, not an author error.
            tracing::error!(kind = %validation.kind, %error, "kind definition did not compile");
            return None;
        }
    };
    let ast = Arc::new(FrontmatterAst::parse_yaml_document(text).ast?);
    let bundle = Arc::new(SchemaBundle {
        advisories: effective.advisories().to_vec(),
        effective,
        frontmatter_json: validation.document,
        extension_shapes: Vec::new(),
        extension_dependencies: Vec::new(),
        extension_definitions: Vec::new(),
    });
    let text_hash = xx_hash_bytes(text.as_bytes());
    let identity = AnalysisIdentity {
        revision,
        text_hash,
        stale: false,
        tree_text_hash: text_hash,
        schema_key: Some(key),
        bundle: Some(Arc::clone(&bundle)),
        named_types: Arc::new(NamedTypeCatalog::default()),
        kind_document: true,
    };
    let analysis = Arc::new(FrontmatterAnalysis::new(identity, Some(Arc::clone(&ast)), Arc::clone(shared)));
    Some(KindState {
        overlay: Arc::new(KindDocumentOverlay {
            kind: validation.kind,
            report: validation.report,
            ast,
            bundle,
        }),
        analysis,
    })
}
