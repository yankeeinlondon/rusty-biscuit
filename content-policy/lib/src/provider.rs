//! The file provider contract: how evaluation and renewal observe the files
//! `FileChanged` rules watch.
//!
//! A provider returns what it saw, never a verdict; the library computes
//! fingerprints and decides what each observation means, so a fake provider
//! and the bundled adapter share one hashing path.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

/// One watched file to observe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileRequest<'a> {
    /// The path as authored in the rule, such as `../src/config.rs` or
    /// `&Cargo.toml`. It has already passed the lexical path rules.
    pub path: &'a str,
    /// The directory a relative path resolves from: the document's directory,
    /// or the directory a caller chose for an evidence record with no document.
    pub base_dir: &'a Path,
}

/// What a provider saw at a watched path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileObservation {
    /// The file's bytes.
    Present(Vec<u8>),
    /// Nothing exists at the path; a broken symlink included. The rule
    /// triggers: the content the document was based on is gone.
    Missing,
    /// The path names a directory or another non-file.
    NotAFile,
    /// The file exists but could not be read; the reason names the failure.
    Unreadable(String),
    /// The path resolves outside the boundary. Evaluation turns this into a
    /// validation diagnostic, so the document gets no verdict.
    OutsideBoundary(String),
}

/// Observes watched files for `FileChanged` rules.
///
/// Implementations are synchronous. The bundled `FileAdapter` (feature
/// `file-adapter`) reads the local file system; tests and callers with their
/// own storage supply another.
pub trait FileProvider: Send + Sync {
    /// Observes one watched file.
    fn observe(&self, request: &FileRequest<'_>) -> FileObservation;
}

/// A provider plus the base directory its requests resolve from.
#[derive(Clone)]
pub(crate) struct FileSource {
    pub provider: Arc<dyn FileProvider>,
    pub base_dir: PathBuf,
}

impl fmt::Debug for FileSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileSource")
            .field("base_dir", &self.base_dir)
            .finish_non_exhaustive()
    }
}

/// One run's observations: identical requests share one observation.
pub(crate) struct Observer<'a> {
    source: Option<&'a FileSource>,
    seen: RefCell<HashMap<String, Rc<FileObservation>>>,
}

impl<'a> Observer<'a> {
    pub fn new(source: Option<&'a FileSource>) -> Self {
        Self {
            source,
            seen: RefCell::new(HashMap::new()),
        }
    }

    /// The observation of `path`, or `None` without a provider.
    pub fn observe(&self, path: &str) -> Option<Rc<FileObservation>> {
        let source = self.source?;
        if let Some(seen) = self.seen.borrow().get(path) {
            return Some(Rc::clone(seen));
        }
        let observation = Rc::new(source.provider.observe(&FileRequest {
            path,
            base_dir: &source.base_dir,
        }));
        self.seen
            .borrow_mut()
            .insert(path.to_string(), Rc::clone(&observation));
        Some(observation)
    }
}
