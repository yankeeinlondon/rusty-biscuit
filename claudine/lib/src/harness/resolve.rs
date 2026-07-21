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

use crate::harness::error::{HarnessError, PathResolutionFailure, ResolutionDetail};

/// Context for resolving harness-internal document references.
#[derive(Debug, Clone)]
pub struct HarnessResolutionContext<'a> {
    /// Absolute path to the source document authoring the reference.
    pub source_path: &'a Path,
    /// Repository (worktree) root, when known. Supplied by the caller (already
    /// discovered via `sniff`); implicit references anchor on it first.
    pub repo_root: Option<&'a Path>,
    /// Package-area root captured for this request. Package (`!`) references
    /// use this anchor before the repository fallback.
    pub package_area: Option<&'a Path>,
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
    // The first candidate is the repository-first primary; surface it in the
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
            package_area: None,
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
            package_area: None,
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
            package_area: None,
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
            package_area: None,
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
            package_area: None,
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
            package_area: None,
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
            package_area: None,
        };
        let result = resolve_harness_path("sibling.md", &ctx).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn package_reference_prefers_captured_package_area_over_repository_root() {
        let repo = tempfile::tempdir().unwrap();
        let package_area = repo.path().join("claudine");
        let source = package_area.join("prompts/run.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "x").unwrap();
        std::fs::write(repo.path().join("shared.md"), "repository decoy").unwrap();
        let package_target = package_area.join("shared.md");
        std::fs::write(&package_target, "package").unwrap();

        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: Some(repo.path()),
            package_area: Some(&package_area),
        };

        assert_eq!(
            resolve_harness_path("!shared.md", &ctx).unwrap(),
            package_target,
        );
    }

    /// A blank reference is rejected before resolution.
    #[test]
    fn empty_reference_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("run.md");
        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: None,
            package_area: None,
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

    /// D8: an implicit no-match retains the whole ordered candidate plan. The
    /// bare reference misses at both anchors, so the typed diagnostic projects
    /// `kind`, `repository_root`, and the two probed candidates in
    /// repository-then-source order — not just the winner the convenience
    /// projection would keep.
    #[test]
    fn implicit_no_match_projects_ordered_candidate_detail() {
        use crate::diagnostics::Diagnostic;

        let repo = tempfile::tempdir().unwrap();
        let source = repo.path().join("prompts/run.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "x").unwrap();

        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: Some(repo.path()),
            package_area: None,
        };
        // Neither `<repo>/absent.md` nor `<repo>/prompts/absent.md` exists.
        let err = resolve_harness_path("absent.md", &ctx).unwrap_err();
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

        let detail = err.detail();
        assert_eq!(detail["kind"], serde_json::json!("implicit_relative"));
        assert_eq!(
            detail["effective_kind"],
            serde_json::json!("implicit_relative")
        );
        assert_eq!(detail["failure"], serde_json::json!("no_match"));
        assert_eq!(
            detail["repository_root"],
            serde_json::json!(repo.path().to_string_lossy())
        );

        let candidates = detail["candidates"]
            .as_array()
            .expect("candidates must be an array");
        assert_eq!(candidates.len(), 2, "both anchors were probed: {detail}");

        let repo_candidate = repo.path().join("absent.md");
        assert_eq!(
            candidates[0]["path"],
            serde_json::json!(repo_candidate.to_string_lossy())
        );
        assert_eq!(candidates[0]["provenance"], serde_json::json!("repository"));
        assert_eq!(candidates[0]["disposition"], serde_json::json!("missing"));

        let source_candidate = repo.path().join("prompts/absent.md");
        assert_eq!(
            candidates[1]["path"],
            serde_json::json!(source_candidate.to_string_lossy())
        );
        assert_eq!(candidates[1]["provenance"], serde_json::json!("source"));
        assert_eq!(candidates[1]["disposition"], serde_json::json!("missing"));
    }

    /// D8: a non-`NotFound` probe failure retains the same detailed projection
    /// as no-match, including every earlier probe and the terminal I/O probe.
    #[cfg(unix)]
    #[test]
    fn io_failure_projects_full_ordered_candidate_detail() {
        use crate::diagnostics::Diagnostic;

        let repo = tempfile::tempdir().unwrap();
        let source = repo.path().join("prompts/run.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "x").unwrap();

        // The repository candidate is absent. The source candidate descends
        // through a regular file, producing ENOTDIR rather than NotFound.
        std::fs::write(repo.path().join("prompts/blocker"), "x").unwrap();

        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: Some(repo.path()),
            package_area: None,
        };
        let err = resolve_harness_path("blocker/target.md", &ctx).unwrap_err();
        assert!(
            matches!(
                err,
                HarnessError::FileReferenceUnresolvable {
                    resolution: Some(_),
                    ..
                }
            ),
            "unexpected variant: {err:?}"
        );

        let detail = err.detail();
        assert_eq!(detail["kind"], serde_json::json!("implicit_relative"));
        assert_eq!(
            detail["effective_kind"],
            serde_json::json!("implicit_relative")
        );
        assert_eq!(detail["failure"], serde_json::json!("permission_io"));
        assert_eq!(
            detail["repository_root"],
            serde_json::json!(repo.path().to_string_lossy())
        );

        let candidates = detail["candidates"]
            .as_array()
            .expect("candidates must be an array");
        assert_eq!(candidates.len(), 2, "both probes must survive: {detail}");

        let repository_candidate = repo.path().join("blocker/target.md");
        assert_eq!(
            candidates[0]["path"],
            serde_json::json!(repository_candidate.to_string_lossy())
        );
        assert_eq!(candidates[0]["provenance"], serde_json::json!("repository"));
        assert_eq!(candidates[0]["disposition"], serde_json::json!("missing"));

        let source_candidate = repo.path().join("prompts/blocker/target.md");
        assert_eq!(
            candidates[1]["path"],
            serde_json::json!(source_candidate.to_string_lossy())
        );
        assert_eq!(candidates[1]["provenance"], serde_json::json!("source"));
        assert_eq!(candidates[1]["disposition"], serde_json::json!("io"));
    }

    /// The retained plan is also reachable as typed candidates for the renderer,
    /// in the same repository-then-source order.
    #[test]
    fn implicit_no_match_exposes_typed_candidates_for_rendering() {
        use biscuit_file::RootProvenance;

        let repo = tempfile::tempdir().unwrap();
        let source = repo.path().join("prompts/run.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "x").unwrap();

        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: Some(repo.path()),
            package_area: None,
        };
        let err = resolve_harness_path("absent.md", &ctx).unwrap_err();

        let candidates = err.resolution_candidates();
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[0].candidate().provenance(),
            RootProvenance::Repository
        );
        assert_eq!(candidates[1].candidate().provenance(), RootProvenance::Source);
    }

    /// A failure drawn before resolution carries no plan: `resolution` stays
    /// `None`, so the structured `candidates`/`kind`/`repository_root` keys
    /// project `null` rather than an invented shape.
    #[test]
    fn pre_resolution_failure_carries_no_candidate_plan() {
        use crate::diagnostics::Diagnostic;

        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("run.md");
        let ctx = HarnessResolutionContext {
            source_path: &source,
            repo_root: None,
            package_area: None,
        };
        let err = resolve_harness_path("   ", &ctx).unwrap_err();
        assert!(err.resolution_candidates().is_empty());
        let detail = err.detail();
        for key in ["kind", "repository_root", "candidates"] {
            assert_eq!(detail[key], serde_json::Value::Null, "`{key}` must be null");
        }
    }

    #[test]
    #[serial_test::serial]
    fn snapshot_resolver_ignores_later_cwd_and_environment_changes() {
        let repo = tempfile::tempdir().unwrap();
        let docs = repo.path().join("docs");
        let nested = docs.join("nested");
        let home = repo.path().join("home");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(nested.join("child.md"), "child").unwrap();
        std::fs::write(home.join("home.md"), "home").unwrap();
        std::fs::write(repo.path().join("env.md"), "env").unwrap();

        let mut env = std::collections::HashMap::new();
        env.insert("SNAPSHOT_ROOT".to_string(), repo.path().display().to_string());
        let snapshot = FileResolutionContext::new(&docs)
            .with_repository_root(repo.path())
            .with_home_dir(&home)
            .with_env(env);
        let unrelated = tempfile::tempdir().unwrap();
        let prior_cwd = std::env::current_dir().unwrap();
        let prior_root = std::env::var_os("SNAPSHOT_ROOT");
        // SAFETY: this test is serialized while mutating process-global state.
        unsafe { std::env::set_var("SNAPSHOT_ROOT", unrelated.path()) };
        std::env::set_current_dir(unrelated.path()).unwrap();

        let source = nested.join("target.md");
        let child = resolve_harness_path_in_context("./child.md", &source, &snapshot).unwrap();
        let home_file = resolve_harness_path_in_context("~/home.md", &source, &snapshot).unwrap();
        let env_file = resolve_harness_path_in_context(
            "{{SNAPSHOT_ROOT}}/env.md",
            &source,
            &snapshot,
        )
        .unwrap();

        std::env::set_current_dir(prior_cwd).unwrap();
        match prior_root {
            Some(value) => unsafe { std::env::set_var("SNAPSHOT_ROOT", value) },
            None => unsafe { std::env::remove_var("SNAPSHOT_ROOT") },
        }
        assert_eq!(child, nested.join("child.md"));
        assert_eq!(home_file, home.join("home.md"));
        assert_eq!(env_file, repo.path().join("env.md"));
    }
}
