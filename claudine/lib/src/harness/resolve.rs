//! Source-relative path resolution for harness validation subjects
//! and redirect targets.

use std::path::{Path, PathBuf};

use crate::harness::error::{HarnessError, PathResolutionFailure};

/// Context for resolving harness-internal paths.
#[derive(Debug, Clone)]
pub struct HarnessResolutionContext<'a> {
    /// Absolute path to the source document.
    pub source_path: &'a Path,
    /// Repository root, if known.
    pub repo_root: Option<&'a Path>,
}

/// Resolve a raw path string according to harness conventions.
///
/// ## Resolution rules
///
/// 1. **Absolute path** — returned as-is.
/// 2. **`@foo/bar`** — stripped of `@` and joined with `repo_root`. Errors if
///    `repo_root` is `None`.
/// 3. **Other relative** — joined with `source_path.parent()`.
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

    let path = Path::new(trimmed);

    // 1. Absolute path
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    // 2. @-prefixed: repo-root-relative
    if let Some(rest) = trimmed.strip_prefix('@') {
        let repo_root = ctx
            .repo_root
            .ok_or_else(|| HarnessError::RepoRootRequired {
                path: raw.to_string(),
            })?;
        return Ok(repo_root.join(rest));
    }

    // 3. Relative to source document's directory
    let source_dir =
        ctx.source_path
            .parent()
            .ok_or_else(|| HarnessError::PathResolutionFailed {
                raw: raw.to_string(),
                failure: PathResolutionFailure::NoSourceParent,
                source_path: Some(ctx.source_path.to_path_buf()),
                resolved: None,
            })?;

    Ok(source_dir.join(trimmed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_path_returned_as_is() {
        let ctx = HarnessResolutionContext {
            source_path: Path::new("/repo/prompts/run.md"),
            repo_root: Some(Path::new("/repo")),
        };
        let result = resolve_harness_path("/abs/path.md", &ctx).unwrap();
        assert_eq!(result, PathBuf::from("/abs/path.md"));
    }

    #[test]
    fn at_prefix_resolves_to_repo_root() {
        let ctx = HarnessResolutionContext {
            source_path: Path::new("/repo/prompts/run.md"),
            repo_root: Some(Path::new("/repo")),
        };
        let result = resolve_harness_path("@docs/plan.md", &ctx).unwrap();
        assert_eq!(result, PathBuf::from("/repo/docs/plan.md"));
    }

    #[test]
    fn at_prefix_errors_without_repo_root() {
        let ctx = HarnessResolutionContext {
            source_path: Path::new("/repo/prompts/run.md"),
            repo_root: None,
        };
        let err = resolve_harness_path("@docs/plan.md", &ctx).unwrap_err();
        assert!(matches!(err, HarnessError::RepoRootRequired { .. }));
    }

    #[test]
    fn dot_slash_relative_to_source() {
        let ctx = HarnessResolutionContext {
            source_path: Path::new("/repo/prompts/run.md"),
            repo_root: Some(Path::new("/repo")),
        };
        let result = resolve_harness_path("./local.md", &ctx).unwrap();
        assert_eq!(result, PathBuf::from("/repo/prompts/local.md"));
    }

    #[test]
    fn bare_relative_to_source() {
        let ctx = HarnessResolutionContext {
            source_path: Path::new("/repo/prompts/run.md"),
            repo_root: Some(Path::new("/repo")),
        };
        let result = resolve_harness_path("sub/file.md", &ctx).unwrap();
        assert_eq!(result, PathBuf::from("/repo/prompts/sub/file.md"));
    }
}
