//! Proxy/redirect reference resolution for the harness.
//!
//! Historically this module carried a private three-branch grammar: absolute
//! passthrough, `@foo` → repository-root join, and everything-else →
//! `source_dir.join`. That grammar diverged from [`FileReference`]: a bare
//! `foo.md` and an explicit `./foo.md` took the identical source-relative path,
//! so an implicit reference never tried the repository root, and `@` meant a
//! repository-root join rather than a magic search.
//!
//! It is now a thin adapter over [`FileReference`] and the shared
//! [`FileResolutionContext`]: implicit references are source-relative then
//! repository-relative, `@` is a magic-root search, `&` pins to the repository
//! root, `^` searches package, package-area, then repository roots, `~` is
//! home-pinned, explicit `./`/`../` stay pinned to the source directory, and resolution
//! probes the filesystem so only an existing regular file is a match.

use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileReferenceError, FileResolutionContext};

use crate::harness::error::{HarnessError, PathResolutionFailure, ResolutionDetail};

/// Context for resolving harness-internal document references.
#[derive(Debug, Clone)]
pub struct HarnessResolutionContext<'a> {
    /// Absolute path to the source document authoring the reference.
    pub source_path: &'a Path,
    /// Repository (worktree) root, when known. Supplied by the caller (already
    /// discovered via `sniff`); implicit references use it after the source.
    pub repo_root: Option<&'a Path>,
    /// Package-area root captured for this request. Repository-scoped (`^`)
    /// references use this anchor before the repository fallback.
    pub package_area: Option<&'a Path>,
}

/// Resolve a raw reference string to an existing file, delegating grammar and
/// candidate ordering to the shared [`FileReference`] resolver.
///
/// ## Resolution rules (all handled by [`FileReference`])
///
/// - **implicit** (`foo.md`, `sub/foo.md`) — source directory first, then the
///   repository root.
/// - **explicit** (`./foo.md`, `../foo.md`) — pinned to the source directory.
/// - **`@foo`** — registered prepend roots, then intrinsic package,
///   package-area, repository, and home roots, then registered append roots.
/// - **`~`**, **`~/foo`** — the user's home directory (`~user` unsupported).
/// - **absolute** — the path itself.
///
/// ## Errors
///
/// Returns [`HarnessError::PathResolutionFailed`] with
/// [`PathResolutionFailure::EmptyReference`] for a blank reference,
/// [`PathResolutionFailure::NoSourceParent`] when the source has no parent
/// directory to anchor against, and [`PathResolutionFailure::TargetMissing`]
/// when no candidate exists. A malformed reference, an absent context anchor,
/// or an I/O probe failure surfaces as
/// [`HarnessError::FileReferenceUnresolvable`] carrying the typed
/// [`FileReferenceError`].
pub fn resolve_harness_path(
    raw: &str,
    ctx: &HarnessResolutionContext<'_>,
) -> Result<PathBuf, HarnessError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(HarnessError::PathResolutionFailed {
            raw: raw.to_string(),
            failure: PathResolutionFailure::EmptyReference,
            source_path: Some(ctx.source_path.to_path_buf()),
            resolved: None,
            resolution: None,
        });
    }

    let file_ref = FileReference::new(trimmed)
        .map_err(|error| unresolvable(trimmed, ctx.source_path, error, None))?;
    let resolution_ctx = build_resolution_context(trimmed, ctx)?;

    let detailed = file_ref.resolve_detailed(&resolution_ctx);
    // Surface the first ordered candidate in the
    // "does not exist" message so a miss names a concrete path.
    let primary = detailed
        .candidates()
        .first()
        .map(|probed| probed.candidate().path().to_path_buf());
    // Retain the whole ordered plan before `into_convenience` discards it, so a
    // no-match projects candidate/root/kind detail rather than just its winner
    // (spec §D8).
    let resolution = ResolutionDetail::from_detailed(&detailed);

    match detailed.into_convenience() {
        Ok(Some(path)) => Ok(path),
        Ok(None) => Err(HarnessError::PathResolutionFailed {
            raw: trimmed.to_string(),
            failure: PathResolutionFailure::TargetMissing,
            source_path: Some(ctx.source_path.to_path_buf()),
            resolved: primary,
            resolution: Some(Box::new(resolution)),
        }),
        Err(error) => Err(unresolvable(
            trimmed,
            ctx.source_path,
            error,
            Some(resolution),
        )),
    }
}

/// Resolves a harness reference from an immutable request snapshot.
///
/// Only the authoring document changes; every other resolution input is
/// retained from `request_context` without ambient reads or discovery.
pub fn resolve_harness_path_in_context(
    raw: &str,
    source_path: &Path,
    request_context: &FileResolutionContext,
) -> Result<PathBuf, HarnessError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(HarnessError::PathResolutionFailed {
            raw: raw.to_string(),
            failure: PathResolutionFailure::EmptyReference,
            source_path: Some(source_path.to_path_buf()),
            resolved: None,
            resolution: None,
        });
    }
    if source_path.parent().is_none() {
        return Err(HarnessError::PathResolutionFailed {
            raw: trimmed.to_string(),
            failure: PathResolutionFailure::NoSourceParent,
            source_path: Some(source_path.to_path_buf()),
            resolved: None,
            resolution: None,
        });
    }
    let file_ref = FileReference::new(trimmed)
        .map_err(|error| unresolvable(trimmed, source_path, error, None))?;
    let detailed = file_ref.resolve_detailed(&request_context.for_source(source_path));
    let primary = detailed
        .candidates()
        .first()
        .map(|probed| probed.candidate().path().to_path_buf());
    let resolution = ResolutionDetail::from_detailed(&detailed);
    match detailed.into_convenience() {
        Ok(Some(path)) => Ok(path),
        Ok(None) => Err(HarnessError::PathResolutionFailed {
            raw: trimmed.to_string(),
            failure: PathResolutionFailure::TargetMissing,
            source_path: Some(source_path.to_path_buf()),
            resolved: primary,
            resolution: Some(Box::new(resolution)),
        }),
        Err(error) => Err(unresolvable(
            trimmed,
            source_path,
            error,
            Some(resolution),
        )),
    }
}

/// Build the explicit resolution context for a document-backed reference.
///
/// `base_dir` is the source document's directory. A caller-supplied repository
/// root is retained only when it lexically contains that directory; implicit
/// references still probe the source directory before that repository scope.
fn build_resolution_context(
    trimmed: &str,
    ctx: &HarnessResolutionContext<'_>,
) -> Result<FileResolutionContext, HarnessError> {
    let base_dir = ctx
        .source_path
        .parent()
        .ok_or_else(|| HarnessError::PathResolutionFailed {
            raw: trimmed.to_string(),
            failure: PathResolutionFailure::NoSourceParent,
            source_path: Some(ctx.source_path.to_path_buf()),
            resolved: None,
            resolution: None,
        })?;

    let mut resolution_ctx =
        FileResolutionContext::new(base_dir).with_source_path(ctx.source_path);
    if let Some(root) = ctx.repo_root.filter(|root| base_dir.starts_with(root)) {
        resolution_ctx = resolution_ctx.with_repository_root(root);
    }
    if let Some(package_area) = ctx.package_area {
        resolution_ctx = resolution_ctx.with_package_area(package_area);
    }
    Ok(resolution_ctx)
}

/// Wrap a typed [`FileReferenceError`] in the harness diagnostic.
fn unresolvable(
    reference: &str,
    source_path: &Path,
    error: FileReferenceError,
    resolution: Option<ResolutionDetail>,
) -> HarnessError {
    HarnessError::FileReferenceUnresolvable {
        reference: reference.to_string(),
        source_path: Some(source_path.to_path_buf()),
        resolution: resolution.map(Box::new),
        source: Box::new(error),
    }
}

#[cfg(test)]
mod tests;
