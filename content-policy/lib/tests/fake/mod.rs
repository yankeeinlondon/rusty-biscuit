//! A scripted `FileProvider` for the `FileChanged` tests.
//!
//! Kept out of `common`, whose `include_bytes!` reads of the migrated
//! repository documents would schedule every binary that declares it.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use content_policy::{FileObservation, FileProvider, FileRequest};

/// Answers each path with a scripted observation (`Missing` when unscripted)
/// and records every request.
#[derive(Debug, Default)]
pub struct FakeFiles {
    files: Mutex<HashMap<String, FileObservation>>,
    requests: Mutex<Vec<(String, PathBuf)>>,
}

impl FakeFiles {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn with(self: &Arc<Self>, path: &str, observation: FileObservation) -> Arc<Self> {
        self.files.lock().unwrap().insert(path.to_string(), observation);
        Arc::clone(self)
    }

    pub fn present(self: &Arc<Self>, path: &str, bytes: &[u8]) -> Arc<Self> {
        self.with(path, FileObservation::Present(bytes.to_vec()))
    }

    /// Every request so far, as `(path, base_dir)`.
    pub fn requests(&self) -> Vec<(String, PathBuf)> {
        self.requests.lock().unwrap().clone()
    }

    pub fn requests_for(&self, path: &str) -> usize {
        self.requests().iter().filter(|(requested, _)| requested == path).count()
    }
}

impl FileProvider for FakeFiles {
    fn observe(&self, request: &FileRequest<'_>) -> FileObservation {
        self.requests
            .lock()
            .unwrap()
            .push((request.path.to_string(), request.base_dir.to_path_buf()));
        self.files
            .lock()
            .unwrap()
            .get(request.path)
            .cloned()
            .unwrap_or(FileObservation::Missing)
    }
}
