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

pub mod frontmatter;
pub mod schema;

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
        {
            return cached.outcome.clone();
        }
        let outcome = match schema::assemble(path, text, config, workspace_roots) {
            Ok(bundle) => SchemaOutcome::Ready(bundle.map(Arc::new)),
            Err(error) => SchemaOutcome::Failed(Arc::new(error)),
        };
        self.schema.insert(uri_key.to_string(), CachedSchema { key, outcome: outcome.clone() });
        outcome
    }
}

/// Content-and-config identity for the schema cache: the document text plus a
/// signature of the schema configuration (extensions + strict mode).
fn schema_cache_key(text: &str, config: &DmlsConfig) -> u64 {
    let config_bytes = serde_json::to_vec(&config.schema).unwrap_or_default();
    xx_hash_bytes(text.as_bytes()) ^ xx_hash_bytes(&config_bytes)
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
