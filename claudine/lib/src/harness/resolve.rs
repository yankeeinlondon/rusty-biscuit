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
//! [`FileResolutionContext`]: implicit references are repository-first then
//! source-relative (D4), `@` is a magic-root search (G2), `~` is home-pinned,
//! explicit `./`/`../` stay pinned to the source directory, and resolution
//! probes the filesystem so only an existing regular file is a match.

use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileReferenceError, FileResolutionContext};

use crate::harness::error::{HarnessError, PathResolutionFailure};

/// Context for resolving harness-internal document references.
#[derive(Debug, Clone)]
pub struct HarnessResolutionContext<'a> {
    /// Absolute path to the source document authoring the reference.
    pub source_path: &'a Path,
    /// Repository (worktree) root, when known. Supplied by the caller (already
    /// discovered via `sniff`); implicit references anchor on it first.
    pub repo_root: Option<&'a Path>,
}

/// Resolve a raw reference string to an existing file, delegating grammar and
/// candidate ordering to the shared [`FileReference`] resolver.
///
/// ## Resolution rules (all handled by [`FileReference`])
///
/// - **implicit** (`foo.md`, `sub/foo.md`) — repository root first, then the
///   source document's directory.
/// - **explicit** (`./foo.md`, `../foo.md`) — pinned to the source directory.
/// - **`@foo`** — magic-root search (repository root, configured roots, home).
/// - **`~foo`** / **`~/foo`** — the user's home directory.
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
        });
    }

    let file_ref = FileReference::new(trimmed)
        .map_err(|error| unresolvable(trimmed, ctx.source_path, error))?;
    let resolution_ctx = build_resolution_context(trimmed, ctx)?;

    let detailed = file_ref.resolve_detailed(&resolution_ctx);
    // The first candidate is the repository-first primary; surface it in the
    // "does not exist" message so a miss names a concrete path.
    let primary = detailed
        .candidates()
        .first()
        .map(|probed| probed.candidate().path().to_path_buf());

    match detailed.into_convenience() {
        Ok(Some(path)) => Ok(path),
        Ok(None) => Err(HarnessError::PathResolutionFailed {
            raw: trimmed.to_string(),
            failure: PathResolutionFailure::TargetMissing,
            source_path: Some(ctx.source_path.to_path_buf()),
            resolved: primary,
        }),
        Err(error) => Err(unresolvable(trimmed, ctx.source_path, error)),
    }
}

/// Build the explicit resolution context for a document-backed reference.
///
/// `base_dir` is the source document's directory. The caller-supplied
/// repository root anchors implicit references first, but only when it lexically
/// contains that directory — otherwise the shared containment check would reject
/// it, so resolution falls back to source-relative candidates.
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
        })?;

    let mut resolution_ctx =
        FileResolutionContext::new(base_dir).with_source_path(ctx.source_path);
    if let Some(root) = ctx.repo_root.filter(|root| base_dir.starts_with(root)) {
        resolution_ctx = resolution_ctx.with_repository_root(root);
    }
    Ok(resolution_ctx)
}

/// Wrap a typed [`FileReferenceError`] in the harness diagnostic.
fn unresolvable(reference: &str, source_path: &Path, error: FileReferenceError) -> HarnessError {
    HarnessError::FileReferenceUnresolvable {
        reference: reference.to_string(),
        source_path: Some(source_path.to_path_buf()),
        source: Box::new(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Absolute paths still classify absolute, but resolution now probes the
    /// filesystem — an existing absolute target resolves to itself.
    #[test]
    fn absolute_existing_path_resolves_to_itself() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("run.md");
        std::fs::write(&source, "x").unwrap();
        let target = dir.path().join("abs.md");
        std::fs::write(&target, "x").unwrap();

        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: Some(dir.path()),
        };
        let result = resolve_harness_path(target.to_str().unwrap(), &ctx).unwrap();
        assert_eq!(result, target);
    }

    /// An absolute reference to a missing file is a typed `TargetMissing`, not
    /// a bare path passthrough (the private grammar returned the path unchecked).
    #[test]
    fn absolute_missing_path_is_target_missing() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("run.md");
        std::fs::write(&source, "x").unwrap();

        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: None,
        };
        let missing = dir.path().join("nope.md");
        let err = resolve_harness_path(missing.to_str().unwrap(), &ctx).unwrap_err();
        assert!(
            matches!(
                err,
                HarnessError::PathResolutionFailed {
                    failure: PathResolutionFailure::TargetMissing,
                    ..
                }
            ),
            "unexpected variant: {err:?}"
        );
    }

    /// G2: `@foo` is a magic-root search, so the repository root is a search
    /// root — `@prompts/x.md` resolves under `<repo>/prompts/`.
    #[test]
    fn at_prefix_searches_repository_root() {
        let repo = tempfile::tempdir().unwrap();
        let source = repo.path().join("sub/run.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "x").unwrap();
        let target = repo.path().join("prompts/plan.md");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "x").unwrap();

        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: Some(repo.path()),
        };
        let result = resolve_harness_path("@prompts/plan.md", &ctx).unwrap();
        assert_eq!(result, target);
    }

    /// Without a repository root a magic reference falls back to home-only
    /// search; an unfound target is `TargetMissing`, never `RepoRootRequired`
    /// (the private grammar errored here — G2 removed that contract).
    #[test]
    fn at_prefix_without_repo_root_is_target_missing() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("run.md");
        std::fs::write(&source, "x").unwrap();

        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: None,
        };
        let err = resolve_harness_path("@docs/definitely-absent-xyz.md", &ctx).unwrap_err();
        assert!(
            matches!(
                err,
                HarnessError::PathResolutionFailed {
                    failure: PathResolutionFailure::TargetMissing,
                    ..
                }
            ),
            "unexpected variant: {err:?}"
        );
    }

    /// Explicit `./foo` stays pinned to the source directory.
    #[test]
    fn dot_slash_is_source_relative() {
        let repo = tempfile::tempdir().unwrap();
        let source = repo.path().join("prompts/run.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "x").unwrap();
        let local = repo.path().join("prompts/local.md");
        std::fs::write(&local, "x").unwrap();

        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: Some(repo.path()),
        };
        let result = resolve_harness_path("./local.md", &ctx).unwrap();
        assert_eq!(result, local);
    }

    /// A bare implicit reference is repository-first: with the same basename
    /// present at both the repository root and the source directory, the
    /// repository root wins (D4).
    #[test]
    fn implicit_reference_prefers_repository_root() {
        let repo = tempfile::tempdir().unwrap();
        let source = repo.path().join("prompts/run.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "x").unwrap();
        let repo_copy = repo.path().join("shared.md");
        std::fs::write(&repo_copy, "x").unwrap();
        let source_copy = repo.path().join("prompts/shared.md");
        std::fs::write(&source_copy, "x").unwrap();

        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: Some(repo.path()),
        };
        let result = resolve_harness_path("shared.md", &ctx).unwrap();
        assert_eq!(
            result, repo_copy,
            "implicit reference must resolve repository-first"
        );
    }

    /// With no repository root, a bare implicit reference is source-relative.
    #[test]
    fn implicit_reference_without_repo_root_is_source_relative() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("run.md");
        std::fs::write(&source, "x").unwrap();
        let target = dir.path().join("sibling.md");
        std::fs::write(&target, "x").unwrap();

        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: None,
        };
        let result = resolve_harness_path("sibling.md", &ctx).unwrap();
        assert_eq!(result, target);
    }

    /// A blank reference is rejected before resolution.
    #[test]
    fn empty_reference_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("run.md");
        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: None,
        };
        let err = resolve_harness_path("   ", &ctx).unwrap_err();
        assert!(matches!(
            err,
            HarnessError::PathResolutionFailed {
                failure: PathResolutionFailure::EmptyReference,
                ..
            }
        ));
    }
}
