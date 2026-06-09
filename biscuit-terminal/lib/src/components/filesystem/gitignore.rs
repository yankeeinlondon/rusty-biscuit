//! gitignore matching for filesystem tree construction.

use std::path::Path;

use ignore::gitignore::{Gitignore, GitignoreBuilder};

/// Matches paths against the `.gitignore` rules in effect for a directory tree.
///
/// Loads the `.gitignore` files from the nearest enclosing git repository (the
/// closest ancestor containing a `.git` entry, or `root` itself when none is
/// found) down through `root`, so repository-level ignore rules apply to a
/// nested render root such as a `docs/` subtree.
pub struct GitignoreMatcher {
    inner: Option<Gitignore>,
}

impl GitignoreMatcher {
    /// Builds a matcher for the tree rooted at `root`.
    pub fn for_root(root: &Path) -> Self {
        let repo_root = repo_root_for(root);

        let mut builder = GitignoreBuilder::new(&repo_root);

        // Ancestors first so deeper `.gitignore` rules can override shallower
        // ones, matching git's precedence.
        for dir in dirs_from_to(&repo_root, root) {
            let _ = builder.add(dir.join(".gitignore"));
        }

        Self {
            inner: builder.build().ok(),
        }
    }

    /// Whether `abs_path` is ignored. `is_dir` selects directory-only rules.
    ///
    /// `abs_path` should be an absolute path within the matcher's root.
    pub fn is_ignored(&self, abs_path: &Path, is_dir: bool) -> bool {
        match &self.inner {
            None => false,
            Some(matcher) => matcher
                .matched_path_or_any_parents(abs_path, is_dir)
                .is_ignore(),
        }
    }
}

/// Nearest ancestor of `root` (inclusive) containing a `.git` entry, or `root`
/// when none is found.
fn repo_root_for(root: &Path) -> std::path::PathBuf {
    let mut current = Some(root);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return dir.to_path_buf();
        }
        current = dir.parent();
    }
    root.to_path_buf()
}

/// Directories from `repo_root` down to `root` (inclusive), ancestors first.
fn dirs_from_to(repo_root: &Path, root: &Path) -> Vec<std::path::PathBuf> {
    let mut chain = Vec::new();
    let mut current = Some(root);
    while let Some(dir) = current {
        chain.push(dir.to_path_buf());
        if dir == repo_root {
            break;
        }
        current = dir.parent();
    }
    chain.reverse();
    chain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matcher_flags_ignored_path() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join(".gitignore"), "secret.txt\n").unwrap();

        let matcher = GitignoreMatcher::for_root(temp.path());

        assert!(matcher.is_ignored(&temp.path().join("secret.txt"), false));
        assert!(!matcher.is_ignored(&temp.path().join("public.txt"), false));
    }
}
