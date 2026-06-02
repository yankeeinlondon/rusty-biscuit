//! Resolution context for filesystem-aware expression functions.
//!
//! Read-only: these helpers resolve and read paths; they never mutate.

use biscuit_file::PathPosition;
use std::path::PathBuf;

/// The document-relative resolution environment passed to filesystem
/// expression functions (`absolute`, `relative`, `frontmatter`, …).
#[derive(Clone, Debug, Default)]
pub struct ResolutionContext {
    /// Directory the current document lives in; relative/`@` refs resolve here.
    pub base_dir: PathBuf,
    /// Magic (`@`) search paths, mirroring the compose link-resolution config.
    pub magic_paths: Vec<(PathBuf, PathPosition)>,
}

impl ResolutionContext {
    /// Creates a context rooted at `base_dir` with no magic search paths.
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            magic_paths: Vec::new(),
        }
    }
}

/// Normalizes a filepath argument: strips a leading `file://` scheme and
/// collapses doubled `/` separators (per the spec's normalization note).
pub fn normalize_path_arg(raw: &str) -> String {
    let stripped = raw.strip_prefix("file://").unwrap_or(raw);
    // Collapse repeated slashes; a leading "./" or "../" is preserved because
    // only consecutive separators are merged, not the dots that precede them.
    let mut out = String::with_capacity(stripped.len());
    let mut prev_slash = false;
    for ch in stripped.chars() {
        if ch == '/' {
            if !prev_slash {
                out.push(ch);
            }
            prev_slash = true;
        } else {
            out.push(ch);
            prev_slash = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn normalizes_file_scheme_and_double_slashes() {
        assert_eq!(normalize_path_arg("file://foo/bar"), "foo/bar");
        assert_eq!(normalize_path_arg("foo//bar"), "foo/bar");
        assert_eq!(normalize_path_arg("./a//b"), "./a/b");
    }

    #[test]
    fn resolution_context_default_is_cwd_no_magic() {
        let ctx = ResolutionContext::new(PathBuf::from("/tmp/docdir"));
        assert_eq!(ctx.base_dir, PathBuf::from("/tmp/docdir"));
        assert!(ctx.magic_paths.is_empty());
    }
}
