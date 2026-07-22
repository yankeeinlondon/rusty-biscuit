//! Shared path-projection helpers for repo/base/home-relative rendering.
//!
//! The expression functions (`relative`, `dirname`, `basename`, …) and the
//! eager-`file` schema rewrite pass (see `markdown/schemas/rewrite.rs`) must
//! produce the *same* repo-relative projection of an absolute path. Housing the
//! projection here guarantees that consistency by construction rather than by
//! parallel re-implementation (spec Decision #2).
//!
//! Two surfaces are exposed:
//!
//! - [`make_relative`] — the raw projection. Returns the OS-native string,
//!   which may carry `\` separators on Windows.
//! - [`make_portable_relative`] — the same projection with separators
//!   normalized to `/`, so committed Markdown is identical across OSes (spec
//!   Risks: "Cross-platform path text drift").
//!
//! Resolution ladder (first match wins):
//! 1. git-root-relative when `base_dir` is inside a repo;
//! 2. `base_dir`-relative;
//! 3. `~/`-aliased home path;
//! 4. the absolute path verbatim.

use std::path::Path;

/// Projects an absolute path to a repo/base/home-relative string.
///
/// This is the raw projection; the returned string uses the OS-native path
/// separator. Callers that persist the result into committed Markdown (the
/// eager-`file` rewrite pass, and the path-component expression functions whose
/// output is composed into document text) should prefer
/// [`make_portable_relative`] so the stored value is portable across OSes.
///
/// `relative(file)` keeps using this raw form to preserve its existing contract.
#[cfg(test)]
pub(crate) fn make_relative(abs: &Path, base_dir: &Path) -> String {
    make_relative_in_context(abs, base_dir, None)
}

pub(crate) fn make_relative_in_context(
    abs: &Path,
    base_dir: &Path,
    request_context: Option<&biscuit_file::FileResolutionContext>,
) -> String {
    let repository_root = match request_context {
        Some(context) => context.repository_root().map(Path::to_path_buf),
        None => crate::markdown::compose::find_git_root_from(base_dir),
    };
    if let Some(repo) = repository_root
        && let Ok(stripped) = abs.strip_prefix(&repo)
    {
        return stripped.to_string_lossy().to_string();
    }
    if let Ok(stripped) = abs.strip_prefix(base_dir) {
        return stripped.to_string_lossy().to_string();
    }
    let home_dir = match request_context {
        Some(context) => context.home_dir().map(Path::to_path_buf),
        None => dirs::home_dir(),
    };
    if let Some(home) = home_dir
        && let Ok(stripped) = abs.strip_prefix(&home)
    {
        return format!("~/{}", stripped.to_string_lossy());
    }
    abs.to_string_lossy().to_string()
}

/// Projects an absolute path to a repo/base/home-relative string with `/`
/// separators, suitable for persisted Markdown.
///
/// Identical to [`make_relative`] except the result is normalized so backslashes
/// (Windows' native separator) become forward slashes. This is the function the
/// eager-`file` rewrite pass calls so a committed value stays byte-identical
/// regardless of the OS it was authored on.
#[cfg(test)]
pub(crate) fn make_portable_relative(abs: &Path, base_dir: &Path) -> String {
    make_relative(abs, base_dir).replace('\\', "/")
}

pub(crate) fn make_portable_relative_in_context(
    abs: &Path,
    base_dir: &Path,
    request_context: Option<&biscuit_file::FileResolutionContext>,
) -> String {
    make_relative_in_context(abs, base_dir, request_context).replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Creates a temp directory that looks like a git repository root.
    fn git_root_fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        dir
    }

    #[test]
    fn git_root_relative_when_inside_repo() {
        let repo = git_root_fixture();
        std::fs::create_dir_all(repo.path().join("sub")).unwrap();
        let abs = repo.path().join("sub/note.md");
        assert_eq!(make_relative(&abs, repo.path()), "sub/note.md");
        assert_eq!(make_portable_relative(&abs, repo.path()), "sub/note.md");
    }

    #[test]
    fn base_dir_relative_outside_repo() {
        // A plain temp dir: no `.git` ancestor on the host, so the projection
        // falls through to the base_dir-relative arm.
        let base = TempDir::new().unwrap();
        std::fs::create_dir_all(base.path().join("deep")).unwrap();
        let abs = base.path().join("deep/file.md");
        assert_eq!(make_relative(&abs, base.path()), "deep/file.md");
        assert_eq!(make_portable_relative(&abs, base.path()), "deep/file.md");
    }

    #[test]
    fn home_aliased_when_under_home() {
        // base_dir is a temp dir (outside any repo, outside $HOME), so the
        // projection reaches the home-alias arm for a path under $HOME.
        let base = TempDir::new().unwrap();
        let home = dirs::home_dir().expect("home dir available");
        let abs = home.join("notes/file.md");
        assert_eq!(make_relative(&abs, base.path()), "~/notes/file.md");
        assert_eq!(
            make_portable_relative(&abs, base.path()),
            "~/notes/file.md"
        );
    }

    #[test]
    fn absolute_fallback_when_no_prefix_matches() {
        let base = TempDir::new().unwrap();
        let other = TempDir::new().unwrap();
        let abs = other.path().join("file.md");
        let rendered = make_relative(&abs, base.path());
        // No repo, not under base_dir, not under $HOME → absolute verbatim.
        assert_eq!(rendered, abs.to_string_lossy());
    }

    #[test]
    fn portable_relative_normalizes_separators_to_forward_slash() {
        let repo = git_root_fixture();
        // On Unix the backslash is a legal filename character, so the component
        // `sub\dir` survives into the raw projection. On Windows the same bytes
        // parse as the `sub\dir` separator pair. Either way the portable
        // wrapper must emit a single forward-slash-joined string — proving the
        // stored Markdown is the same on every OS.
        let abs = repo.path().join("sub\\dir/file.md");
        let raw = make_relative(&abs, repo.path());
        let portable = make_portable_relative(&abs, repo.path());
        assert_ne!(
            raw, portable,
            "raw projection must still carry the OS separator"
        );
        assert!(
            !portable.contains('\\'),
            "portable projection must use forward slashes only: {portable:?}"
        );
        assert_eq!(portable, "sub/dir/file.md");
    }
}
