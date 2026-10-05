//! Warnings for glob references listed during composition.
//!
//! `find_files()` and `::file-links <glob>` list a
//! [`biscuit_file::GlobReference`] and report each file symlink the listing
//! skipped (its target leaves the file tree) as one
//! [`ComposeWarning::GLOB_SKIPPED_SYMLINK_CODE`] warning.

use std::sync::{Arc, Mutex, MutexGuard};

use biscuit_file::GlobListing;

use super::ComposeWarning;

/// The request's sink for warnings raised by `find_files()`, which runs
/// inside expression evaluation and has no report of its own.
///
/// Every nested and transcluded child clones the request's options, so one
/// handle, drained by the root pipeline, collects the warnings raised
/// anywhere under the root.
#[derive(Clone, Debug, Default)]
pub(crate) struct GlobWarningSink {
    warnings: Arc<Mutex<Vec<ComposeWarning>>>,
}

impl GlobWarningSink {
    /// Records one warning per entry `listing` skipped, under `stage`.
    pub(crate) fn record(&self, stage: &str, listing: &GlobListing) {
        lock(&self.warnings).extend(
            listing
                .skipped
                .iter()
                .map(|entry| ComposeWarning::skipped_symlink(stage, entry)),
        );
    }

    /// Drains the warnings recorded so far in the request.
    pub(crate) fn take_warnings(&self) -> Vec<ComposeWarning> {
        std::mem::take(&mut *lock(&self.warnings))
    }
}

fn lock<T: ?Sized>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
