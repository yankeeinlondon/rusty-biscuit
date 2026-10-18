//! Workspace listing and the progress seam background work reports through.
//!
//! Server startup only lists files ([`list_files`]); Markdown is parsed when
//! a feature needs it, and the index store's revalidation reads every listed
//! file on the first workspace-wide request
//! ([`crate::index_store::WorkspaceFacts::revalidate`]). Progress is reported
//! through the [`ProgressReporter`] seam so the protocol loop can drive
//! `window/workDoneProgress` while tests use a silent collector.

use std::path::PathBuf;

use crate::config::WorkspaceConfig;

use super::discover::{DiscoveryReport, discover_workspace};

/// A sink for indexing progress. Implementations drive
/// `window/workDoneProgress` (protocol loop) or collect for tests.
pub trait ProgressReporter: Send + Sync {
    /// Emitted once before discovery, so the client sees activity within the
    /// first-notification budget.
    fn begin(&self, title: &str);
    /// Emitted as files finish (`done` of `total`).
    fn report(&self, done: usize, total: usize);
    /// Emitted once when indexing completes.
    fn end(&self);
}

/// A progress reporter that does nothing (bench mode, tests without a client).
#[derive(Debug, Default)]
pub struct SilentProgress;

impl ProgressReporter for SilentProgress {
    fn begin(&self, _title: &str) {}
    fn report(&self, _done: usize, _total: usize) {}
    fn end(&self) {}
}

/// Lists a workspace's files without reading them, logging each duplicate
/// physical file at `warn` and an incomplete walk at `warn`.
pub fn list_files(roots: &[PathBuf], config: &WorkspaceConfig) -> DiscoveryReport {
    let report = discover_workspace(roots, config);
    for duplicate in &report.duplicates {
        tracing::warn!("skipping duplicate physical file {}", duplicate.display());
    }
    if !report.is_complete() {
        tracing::warn!(
            errors = report.errors.len(),
            first_error = report.errors.first().map(String::as_str).unwrap_or_default(),
            "workspace discovery incomplete"
        );
    }
    report
}
