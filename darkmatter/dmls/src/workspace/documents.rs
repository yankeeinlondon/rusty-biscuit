//! Open-document store: full-document sync with the client buffer as the
//! authority over disk.
//!
//! Every snapshot is stamped `(uri, version)` via its [`SourceMap`], and
//! stale `didChange` notifications (version not newer than the stored one)
//! are rejected so a late-arriving change can never roll a document back.

use std::collections::HashMap;
use std::sync::Arc;

use lsp_types::Uri;
use thiserror::Error;

use crate::source_map::{PositionEncoding, SourceMap};

/// Errors from document lifecycle notifications.
#[derive(Debug, Error, PartialEq)]
pub enum DocumentError {
    /// A change/close/save referenced a document that is not open.
    #[error("document is not open: {uri}")]
    NotOpen {
        /// URI the notification referenced.
        uri: String,
    },
    /// A `didChange` carried a version not newer than the stored one.
    #[error("stale change for {uri}: received version {received}, have {current}")]
    StaleChange {
        /// URI the notification referenced.
        uri: String,
        /// Version currently stored.
        current: i32,
        /// Version the stale notification carried.
        received: i32,
    },
    /// A `didChange` carried a ranged (incremental) edit; DMLS advertises
    /// full sync only.
    #[error("incremental change for {uri} but only full sync is advertised")]
    IncrementalChange {
        /// URI the notification referenced.
        uri: String,
    },
}

/// One open document snapshot (client buffer authoritative).
#[derive(Debug)]
pub struct OpenDocument {
    text: Arc<str>,
    version: i32,
    source_map: Arc<SourceMap>,
}

impl OpenDocument {
    /// The current buffer text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The client's version number for this snapshot.
    pub fn version(&self) -> i32 {
        self.version
    }

    /// The source map for exactly this `(uri, version)` snapshot.
    pub fn source_map(&self) -> &Arc<SourceMap> {
        &self.source_map
    }
}

/// All currently open documents.
#[derive(Debug)]
pub struct DocumentStore {
    encoding: PositionEncoding,
    documents: HashMap<Uri, OpenDocument>,
}

impl DocumentStore {
    /// Creates an empty store using the negotiated position encoding.
    pub fn new(encoding: PositionEncoding) -> Self {
        Self {
            encoding,
            documents: HashMap::new(),
        }
    }

    /// Handles `textDocument/didOpen`. Re-opening an already-open URI
    /// replaces the snapshot (clients may re-open after a crash).
    pub fn open(&mut self, uri: Uri, version: i32, text: String) {
        let snapshot = self.snapshot(uri.clone(), version, text.into());
        self.documents.insert(uri, snapshot);
    }

    /// Handles `textDocument/didChange` full-text replacement.
    ///
    /// ## Errors
    ///
    /// [`DocumentError::NotOpen`] for unknown URIs and
    /// [`DocumentError::StaleChange`] when `version` is not newer than the
    /// stored snapshot's.
    pub fn change(&mut self, uri: &Uri, version: i32, text: String) -> Result<(), DocumentError> {
        let current = self
            .documents
            .get(uri)
            .ok_or_else(|| DocumentError::NotOpen {
                uri: uri.as_str().to_string(),
            })?
            .version;
        if version <= current {
            return Err(DocumentError::StaleChange {
                uri: uri.as_str().to_string(),
                current,
                received: version,
            });
        }
        let snapshot = self.snapshot(uri.clone(), version, text.into());
        self.documents.insert(uri.clone(), snapshot);
        Ok(())
    }

    /// Handles `textDocument/didClose`.
    ///
    /// ## Errors
    ///
    /// [`DocumentError::NotOpen`] for unknown URIs.
    pub fn close(&mut self, uri: &Uri) -> Result<(), DocumentError> {
        self.documents
            .remove(uri)
            .map(|_| ())
            .ok_or_else(|| DocumentError::NotOpen {
                uri: uri.as_str().to_string(),
            })
    }

    /// Returns the open snapshot for `uri`, if any.
    pub fn get(&self, uri: &Uri) -> Option<&OpenDocument> {
        self.documents.get(uri)
    }

    /// Whether `uri` is currently open.
    pub fn is_open(&self, uri: &Uri) -> bool {
        self.documents.contains_key(uri)
    }

    /// The URIs of all currently open documents.
    pub fn open_uris(&self) -> Vec<Uri> {
        self.documents.keys().cloned().collect()
    }

    /// Number of open documents.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Whether no documents are open.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    fn snapshot(&self, uri: Uri, version: i32, text: Arc<str>) -> OpenDocument {
        let source_map = Arc::new(SourceMap::new(
            uri,
            version,
            self.encoding,
            Arc::clone(&text),
        ));
        OpenDocument {
            text,
            version,
            source_map,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(s: &str) -> Uri {
        s.parse().unwrap()
    }

    fn store() -> DocumentStore {
        DocumentStore::new(PositionEncoding::Utf16)
    }

    #[test]
    fn test_open_then_get() {
        let mut store = store();
        let doc = uri("file:///a.md");
        store.open(doc.clone(), 1, "# Hi".to_string());
        let open = store.get(&doc).unwrap();
        assert_eq!(open.text(), "# Hi");
        assert_eq!(open.version(), 1);
        assert_eq!(open.source_map().version(), 1);
        assert_eq!(open.source_map().uri(), &doc);
    }

    #[test]
    fn test_change_replaces_full_text() {
        let mut store = store();
        let doc = uri("file:///a.md");
        store.open(doc.clone(), 1, "one".to_string());
        store.change(&doc, 2, "two".to_string()).unwrap();
        let open = store.get(&doc).unwrap();
        assert_eq!(open.text(), "two");
        assert_eq!(open.version(), 2);
        assert_eq!(open.source_map().version(), 2);
    }

    #[test]
    fn test_stale_change_rejected() {
        let mut store = store();
        let doc = uri("file:///a.md");
        store.open(doc.clone(), 5, "five".to_string());
        let err = store.change(&doc, 5, "again".to_string()).unwrap_err();
        assert_eq!(
            err,
            DocumentError::StaleChange {
                uri: "file:///a.md".to_string(),
                current: 5,
                received: 5,
            }
        );
        let err = store.change(&doc, 3, "old".to_string()).unwrap_err();
        assert!(matches!(err, DocumentError::StaleChange { .. }));
        assert_eq!(store.get(&doc).unwrap().text(), "five");
    }

    #[test]
    fn test_change_unknown_uri_rejected() {
        let mut store = store();
        let doc = uri("file:///nope.md");
        let err = store.change(&doc, 1, "x".to_string()).unwrap_err();
        assert!(matches!(err, DocumentError::NotOpen { .. }));
    }

    #[test]
    fn test_close_removes_document() {
        let mut store = store();
        let doc = uri("file:///a.md");
        store.open(doc.clone(), 1, "x".to_string());
        store.close(&doc).unwrap();
        assert!(!store.is_open(&doc));
        assert!(matches!(
            store.close(&doc),
            Err(DocumentError::NotOpen { .. })
        ));
    }

    #[test]
    fn test_reopen_replaces_snapshot() {
        let mut store = store();
        let doc = uri("file:///a.md");
        store.open(doc.clone(), 7, "old".to_string());
        store.open(doc.clone(), 1, "new".to_string());
        let open = store.get(&doc).unwrap();
        assert_eq!(open.text(), "new");
        assert_eq!(open.version(), 1);
    }
}
