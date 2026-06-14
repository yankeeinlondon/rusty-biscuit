//! gitignore matching for filesystem tree construction.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use ignore::Match;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

/// Matches paths against the `.gitignore` rules in effect for a directory tree,
/// using Git's hierarchical semantics.
///
/// Each `.gitignore` file is scoped to the directory that contains it: its
/// patterns are matched relative to that directory, a deeper file overrides a
/// shallower one, and a rule cannot re-include a path whose parent directory is
/// itself ignored. Per-directory matchers are loaded lazily and cached as paths
/// are evaluated, so `.gitignore` files both above the rendered component root
/// (repository-level rules) and below it (nested rules) participate.
pub struct GitignoreMatcher {
    /// Nearest enclosing repository root (closest ancestor with a `.git`
    /// entry), or the tree root when none is found. Evaluation never ascends
    /// past this directory.
    repo_root: PathBuf,
    /// Per-directory `.gitignore` matchers, loaded on demand. A `None` value
    /// records a directory with no readable `.gitignore` so it is not probed
    /// from disk twice.
    cache: RefCell<HashMap<PathBuf, Option<Gitignore>>>,
}

impl GitignoreMatcher {
    /// Builds a matcher for the tree rooted at `root`.
    pub fn for_root(root: &Path) -> Self {
        Self {
            repo_root: repo_root_for(root),
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// Whether `abs_path` is ignored. `is_dir` selects directory-only rules.
    ///
    /// `abs_path` should be an absolute path within the matcher's repository
    /// root; paths outside it are never ignored.
    pub fn is_ignored(&self, abs_path: &Path, is_dir: bool) -> bool {
        let Ok(rel) = abs_path.strip_prefix(&self.repo_root) else {
            return false;
        };
        let components: Vec<&OsStr> = rel
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s),
                _ => None,
            })
            .collect();
        if components.is_empty() {
            return false;
        }

        // Descend one path component at a time from the repository root. Each
        // candidate is evaluated against every `.gitignore` from the repository
        // root down to the candidate's parent, deeper files overriding
        // shallower ones. Git never descends into an ignored directory, so once
        // an intermediate directory is ignored the leaf is ignored too,
        // regardless of any deeper re-include rule.
        let mut ignored = false;
        let mut current = self.repo_root.clone();
        let last = components.len() - 1;
        for (idx, comp) in components.iter().enumerate() {
            let candidate = current.join(comp);
            let candidate_is_dir = if idx == last { is_dir } else { true };

            if let Some(decision) =
                self.decision_for_candidate(&current, &candidate, candidate_is_dir)
            {
                ignored = decision;
            }

            if ignored && candidate_is_dir && idx != last {
                return true;
            }
            current = candidate;
        }
        ignored
    }

    /// The ignore/whitelist decision for `candidate`, considering every
    /// `.gitignore` from the repository root down to `parent` (inclusive), with
    /// deeper files overriding shallower ones. `None` when no rule applies.
    fn decision_for_candidate(
        &self,
        parent: &Path,
        candidate: &Path,
        is_dir: bool,
    ) -> Option<bool> {
        let mut decision = None;
        for dir in dirs_from_to(&self.repo_root, parent) {
            if let Some(verdict) = self.dir_decision(&dir, candidate, is_dir) {
                decision = Some(verdict);
            }
        }
        decision
    }

    /// Evaluates `candidate` against the `.gitignore` located in `dir`, whose
    /// patterns are scoped to `dir`. Loads and caches the matcher on first use.
    fn dir_decision(&self, dir: &Path, candidate: &Path, is_dir: bool) -> Option<bool> {
        let mut cache = self.cache.borrow_mut();
        let entry = cache
            .entry(dir.to_path_buf())
            .or_insert_with(|| build_dir_gitignore(dir));
        match entry.as_ref() {
            None => None,
            Some(gi) => match gi.matched(candidate, is_dir) {
                Match::None => None,
                Match::Ignore(_) => Some(true),
                Match::Whitelist(_) => Some(false),
            },
        }
    }
}

/// Builds a [`Gitignore`] for the `.gitignore` in `dir`, scoped to `dir` so its
/// patterns match relative to that directory. Returns `None` when no readable
/// `.gitignore` is present.
fn build_dir_gitignore(dir: &Path) -> Option<Gitignore> {
    let gitignore_path = dir.join(".gitignore");
    if !gitignore_path.is_file() {
        return None;
    }
    let mut builder = GitignoreBuilder::new(dir);
    let _ = builder.add(&gitignore_path);
    builder.build().ok()
}

/// Nearest ancestor of `root` (inclusive) containing a `.git` entry, or `root`
/// when none is found.
fn repo_root_for(root: &Path) -> PathBuf {
    let mut current = Some(root);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return dir.to_path_buf();
        }
        current = dir.parent();
    }
    root.to_path_buf()
}

/// Directories from `repo_root` down to `leaf` (inclusive), ancestors first.
fn dirs_from_to(repo_root: &Path, leaf: &Path) -> Vec<PathBuf> {
    let mut chain = Vec::new();
    let mut current = Some(leaf);
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

    /// Creates a temp directory containing a `.git` marker so it registers as a
    /// repository root, plus the listed `(relative_path, contents)` files. Any
    /// parent directories are created as needed.
    fn repo_with(files: &[(&str, &str)]) -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::create_dir_all(temp.path().join(".git")).unwrap();
        for (rel, contents) in files {
            let path = temp.path().join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, contents).unwrap();
        }
        temp
    }

    #[test]
    fn matcher_flags_ignored_path() {
        let temp = tempfile::tempdir().expect("create temp dir");
        std::fs::write(temp.path().join(".gitignore"), "secret.txt\n").unwrap();

        let matcher = GitignoreMatcher::for_root(temp.path());

        assert!(matcher.is_ignored(&temp.path().join("secret.txt"), false));
        assert!(!matcher.is_ignored(&temp.path().join("public.txt"), false));
    }

    #[test]
    fn nested_anchored_rule_is_scoped_to_its_directory() {
        // `/draft.md` in `docs/.gitignore` must anchor to `docs`, not the repo
        // root, and must not leak into `docs/sub`.
        let repo = repo_with(&[("docs/.gitignore", "/draft.md\n")]);
        let matcher = GitignoreMatcher::for_root(&repo.path().join("docs"));

        assert!(
            matcher.is_ignored(&repo.path().join("docs/draft.md"), false),
            "anchored rule must match in its own directory",
        );
        assert!(
            !matcher.is_ignored(&repo.path().join("draft.md"), false),
            "anchored rule must not match at the repository root",
        );
        assert!(
            !matcher.is_ignored(&repo.path().join("docs/sub/draft.md"), false),
            "anchored rule must not match in a subdirectory",
        );
    }

    #[test]
    fn nested_unanchored_rule_does_not_affect_siblings() {
        // `*.tmp` in `docs/.gitignore` applies under `docs` (including nested
        // dirs) but never to the sibling `src` subtree.
        let repo = repo_with(&[("docs/.gitignore", "*.tmp\n")]);
        let matcher = GitignoreMatcher::for_root(repo.path());

        assert!(matcher.is_ignored(&repo.path().join("docs/scratch.tmp"), false));
        assert!(matcher.is_ignored(&repo.path().join("docs/sub/scratch.tmp"), false));
        assert!(
            !matcher.is_ignored(&repo.path().join("src/scratch.tmp"), false),
            "a rule in docs/ must not affect the sibling src/ subtree",
        );
    }

    #[test]
    fn deeper_negation_overrides_shallower_ignore() {
        // The repo ignores every `*.log`; `docs/.gitignore` re-includes one.
        let repo = repo_with(&[
            (".gitignore", "*.log\n"),
            ("docs/.gitignore", "!keep.log\n"),
        ]);
        let matcher = GitignoreMatcher::for_root(repo.path());

        assert!(
            !matcher.is_ignored(&repo.path().join("docs/keep.log"), false),
            "deeper negation must re-include the file",
        );
        assert!(
            matcher.is_ignored(&repo.path().join("docs/other.log"), false),
            "non-negated files stay ignored under the deeper rule",
        );
        assert!(
            matcher.is_ignored(&repo.path().join("root.log"), false),
            "the shallow rule still ignores files outside docs/",
        );
    }

    #[test]
    fn repository_rule_applies_to_a_nested_component_root() {
        // The component root is `docs`, below the repository root. A repo-level
        // rule must still dim matching entries inside `docs`.
        let repo = repo_with(&[(".gitignore", "secret.md\n")]);
        let matcher = GitignoreMatcher::for_root(&repo.path().join("docs"));

        assert!(matcher.is_ignored(&repo.path().join("docs/secret.md"), false));
        assert!(!matcher.is_ignored(&repo.path().join("docs/public.md"), false));
    }

    #[test]
    fn gitignore_below_the_component_root_is_evaluated() {
        // A `.gitignore` deeper than the matcher root must still apply — the
        // bug this matcher was rebuilt to fix.
        let repo = repo_with(&[("docs/private/.gitignore", "hush.md\n")]);
        let matcher = GitignoreMatcher::for_root(&repo.path().join("docs"));

        assert!(
            matcher.is_ignored(&repo.path().join("docs/private/hush.md"), false),
            "a .gitignore below the component root must dim its entries",
        );
        assert!(!matcher.is_ignored(&repo.path().join("docs/private/loud.md"), false));
    }

    #[test]
    fn ignored_directory_ignores_its_whole_subtree() {
        // Git never descends into an ignored directory, so a leaf with no rule
        // of its own is ignored when an ancestor directory is ignored.
        let repo = repo_with(&[(".gitignore", "build/\n")]);
        let matcher = GitignoreMatcher::for_root(repo.path());

        assert!(matcher.is_ignored(&repo.path().join("build"), true));
        assert!(matcher.is_ignored(&repo.path().join("build/out/app.o"), false));
        assert!(!matcher.is_ignored(&repo.path().join("src/out/app.o"), false));
    }
}
