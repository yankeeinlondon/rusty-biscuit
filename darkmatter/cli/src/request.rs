//! The request every `md` route resolves file references against.
//!
//! `main` reads the process once ([`RequestSnapshot::from_process`]) and hands
//! the resulting [`MdRequest`] to every route. A route never reads the current
//! directory, home directory, or environment itself: document arguments,
//! transclusion targets, and schema file values all resolve against contexts
//! built from this one snapshot. See `darkmatter/docs/topics/compose-requests.md`.

use std::cell::OnceCell;
use std::path::Path;

use biscuit_file::{FileReference, FileResolutionContext};
use color_eyre::eyre::{Context, Result};
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};

/// One `md` invocation's request snapshot and its lazily built launch context.
///
/// The launch context (repository discovery from the launch directory) is
/// built on first use, so a route that resolves nothing never pays for
/// discovery and never fails on a repository it does not need.
#[derive(Debug)]
pub struct MdRequest {
    snapshot: RequestSnapshot,
    launch: OnceCell<FileResolutionContext>,
}

impl MdRequest {
    /// Wraps the invocation's snapshot.
    pub fn new(snapshot: RequestSnapshot) -> Self {
        Self { snapshot, launch: OnceCell::new() }
    }

    /// The snapshot `main` captured.
    pub fn snapshot(&self) -> &RequestSnapshot {
        &self.snapshot
    }

    /// The directory `md` was launched from.
    pub fn launch_dir(&self) -> &Path {
        self.snapshot.request_dir()
    }

    /// The context built at the launch directory.
    ///
    /// ## Errors
    ///
    /// Returns the builder's error, for example when repository discovery at
    /// the launch directory fails.
    pub fn launch_context(&self) -> Result<&FileResolutionContext> {
        if let Some(context) = self.launch.get() {
            return Ok(context);
        }
        let context = build_resolution_context(&self.snapshot).wrap_err_with(|| {
            format!(
                "Failed to prepare file resolution at the launch directory {}",
                self.launch_dir().display()
            )
        })?;
        Ok(self.launch.get_or_init(|| context))
    }

    /// The context a document at `resolved`, opened as `opening`, resolves its
    /// own references against.
    ///
    /// A document inside the launch tree derives from the launch context
    /// (`for_source_reference` / `for_source`), so it keeps the launch
    /// repository's package catalog and `@` scope. A document the launch tree
    /// does not contain (another repository, or outside any repository) gets a
    /// context built at its own directory and derived as a trusted external
    /// source, keeping this request's home and environment.
    ///
    /// ## Errors
    ///
    /// Returns the builder's error for the launch directory or, for a document
    /// outside the launch tree, for the document's directory.
    pub fn document_context(
        &self,
        opening: Option<&FileReference>,
        resolved: &Path,
    ) -> Result<FileResolutionContext> {
        let launch = self.launch_context()?;
        let derived = match opening {
            Some(reference) => launch.for_source_reference(reference, resolved),
            None => launch.for_source(resolved),
        };
        if derived.validate().is_ok() {
            return Ok(derived);
        }
        let source_dir = resolved.parent().unwrap_or(self.launch_dir());
        let external = build_resolution_context(&self.snapshot.at_request_dir(source_dir))
            .wrap_err_with(|| {
                format!("Failed to prepare file resolution for {}", resolved.display())
            })?;
        Ok(match opening {
            Some(reference) => external.for_trusted_external_source_reference(reference, resolved),
            None => external.for_trusted_external_source(resolved),
        })
    }
}
