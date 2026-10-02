//! The request every `md` route resolves file references against.
//!
//! `main` reads the process once ([`RequestSnapshot::from_process`]), adds the
//! `--magic-root` directories ([`with_magic_roots`]), and hands the resulting
//! [`MdRequest`] to every route. A route never reads the current
//! directory, home directory, or environment itself: document arguments,
//! transclusion targets, and schema file values all resolve against contexts
//! built from this one snapshot. See `darkmatter/docs/topics/compose-requests.md`.

use std::cell::OnceCell;
use std::fmt;
use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileResolutionContext, PathPosition};
use color_eyre::eyre::{Context, Result};
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};

/// A `--magic-root` directory that is not a directory on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagicRootError {
    /// The directory as the caller spelled it.
    pub argument: PathBuf,
    /// The directory the argument names from the launch directory.
    pub resolved: PathBuf,
}

impl fmt::Display for MagicRootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "--magic-root {}: {} is not a directory",
            self.argument.display(),
            self.resolved.display()
        )
    }
}

impl std::error::Error for MagicRootError {}

/// `snapshot` with each `--magic-root` directory registered as an extra `@`
/// root, in the order given, ahead of its tier's default roots.
///
/// A relative directory is relative to the snapshot's (launch) directory. The
/// tier is inferred from where the directory lies: inside the launch
/// repository it is searched with the local roots, elsewhere with the user
/// roots before the home directory.
///
/// ## Errors
///
/// [`MagicRootError`] for the first directory that does not exist or is not a
/// directory; a root that cannot be searched is never silently skipped.
pub fn with_magic_roots(
    snapshot: RequestSnapshot,
    roots: &[PathBuf],
) -> Result<RequestSnapshot, MagicRootError> {
    let mut snapshot = snapshot;
    for argument in roots {
        let resolved = snapshot.request_dir().join(argument);
        if !resolved.is_dir() {
            return Err(MagicRootError { argument: argument.clone(), resolved });
        }
        snapshot = snapshot.with_magic_root(resolved, PathPosition::Start);
    }
    Ok(snapshot)
}

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
    /// A document inside the launch repository derives from the launch context
    /// (`for_source_reference` / `for_source`), so it keeps the launch
    /// repository's package catalog and `@` scope. Any other document gets a
    /// context built at its own directory, keeping this request's home and
    /// environment: when that build finds a repository (the launch directory
    /// is in another repository, or in none) the document derives from it as a
    /// trusted external source, so its links, `&` references, and trigger
    /// schemas see its own repository, while its `@` references keep the
    /// launch `@` scope (the launch tree and `--magic-root` directories). A
    /// document in no repository, outside the launch repository, also derives
    /// as a trusted external source; one in no repository launched from no
    /// repository keeps the launch derivation.
    ///
    /// ## Errors
    ///
    /// Returns the builder's error for the launch directory or, for a document
    /// outside the launch repository, for the document's directory.
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
        let derived_is_valid = derived.validate().is_ok();
        if derived_is_valid && launch.repository_root().is_some() {
            return Ok(derived);
        }
        let source_dir = resolved.parent().unwrap_or(self.launch_dir());
        let external = build_resolution_context(&self.snapshot.at_request_dir(source_dir))
            .wrap_err_with(|| {
                format!("Failed to prepare file resolution for {}", resolved.display())
            })?;
        // A launch directory outside every repository derives any document
        // without complaint, but its fallback tree knows no repository; only
        // the document's own discovery can tell whether one contains it.
        if derived_is_valid && external.repository_root().is_none() {
            return Ok(derived);
        }
        let source = match opening {
            Some(reference) => external.for_trusted_external_source_reference(reference, resolved),
            None => external.for_trusted_external_source(resolved),
        };
        // The rebuild re-anchors `./`, bare, `&`, and `^` on the document's
        // repository; `@` keeps searching the launch tree, as it does for a
        // source derived from the launch context.
        Ok(source.with_launch_magic_scope(launch.launch_magic_scope().clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_roots_resolve_from_the_launch_directory_in_order() {
        let launch = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(launch.path().join("first")).unwrap();
        let second = tempfile::tempdir().unwrap();

        let snapshot = with_magic_roots(
            RequestSnapshot::new(launch.path()),
            &[PathBuf::from("first"), second.path().to_path_buf()],
        )
        .unwrap();

        let roots: Vec<_> = snapshot.magic_roots().iter().map(|(path, position, _)| (path.clone(), *position)).collect();
        assert_eq!(
            roots,
            [
                (launch.path().join("first"), PathPosition::Start),
                (second.path().to_path_buf(), PathPosition::Start),
            ]
        );
    }

    #[test]
    fn a_missing_magic_root_is_an_error() {
        let launch = tempfile::tempdir().unwrap();
        std::fs::write(launch.path().join("file.md"), "").unwrap();

        for argument in ["missing", "file.md"] {
            let error = with_magic_roots(RequestSnapshot::new(launch.path()), &[PathBuf::from(argument)])
                .unwrap_err();
            assert_eq!(
                error,
                MagicRootError { argument: PathBuf::from(argument), resolved: launch.path().join(argument) }
            );
        }
    }
}
