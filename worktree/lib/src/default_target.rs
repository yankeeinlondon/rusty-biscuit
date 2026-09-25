//! The default-branch target: the one tip that ahead/behind and merge
//! comparisons are measured against.
//!
//! Of the local default branch and `origin/<default>`, the one that contains
//! the other wins. When they have diverged, `origin/<default>` wins, because it
//! is the copy everyone shares. Without a remote-tracking ref the local tip is
//! used. `wt` never fetches, so `origin/<default>` is as of the last fetch.

use std::path::Path;

use crate::git::git_from;

/// The selected default-branch tip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultTarget {
    /// The ref as the user reads it: `main` or `origin/main`.
    pub reference: String,
    pub sha: String,
    /// The local and remote-tracking tips each have commits the other lacks.
    pub diverged: bool,
}

/// Selects the default-branch target for the repository at `base`.
///
/// Returns `None` when neither `refs/heads/<default>` nor
/// `refs/remotes/origin/<default>` exists.
pub fn select_default_target(base: &Path, default_branch: &str) -> Option<DefaultTarget> {
    let resolve = |full_ref: &str| {
        git_from(base, base, &["rev-parse", "--verify", "--quiet", full_ref])
            .ok()
            .filter(|sha| !sha.is_empty())
    };
    let local = resolve(&format!("refs/heads/{default_branch}^{{commit}}"));
    let remote = resolve(&format!("refs/remotes/origin/{default_branch}^{{commit}}"));
    let remote_name = format!("origin/{default_branch}");

    let target = |reference: &str, sha: String, diverged: bool| DefaultTarget {
        reference: reference.to_string(),
        sha,
        diverged,
    };
    match (local, remote) {
        (None, None) => None,
        (Some(local), None) => Some(target(default_branch, local, false)),
        (None, Some(remote)) => Some(target(&remote_name, remote, false)),
        (Some(local), Some(remote)) => {
            if is_ancestor(base, &remote, &local) {
                Some(target(default_branch, local, false))
            } else if is_ancestor(base, &local, &remote) {
                Some(target(&remote_name, remote, false))
            } else {
                Some(target(&remote_name, remote, true))
            }
        }
    }
}

/// Whether `ancestor` is part of `descendant`'s history (a commit is its own
/// ancestor). A git failure counts as "no".
pub(crate) fn is_ancestor(base: &Path, ancestor: &str, descendant: &str) -> bool {
    git_from(base, base, &["merge-base", "--is-ancestor", ancestor, descendant]).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remove::test_support::TestRepo;

    #[test]
    fn local_only_selects_the_local_tip() {
        let repo = TestRepo::new();
        let target = select_default_target(&repo.path(), "main").unwrap();
        assert_eq!(target.reference, "main");
        assert_eq!(target.sha, repo.sha("main"));
        assert!(!target.diverged);
    }

    #[test]
    fn missing_default_branch_selects_nothing() {
        let repo = TestRepo::new();
        assert_eq!(select_default_target(&repo.path(), "trunk"), None);
    }

    #[test]
    fn the_descendant_wins_in_both_directions() {
        let repo = TestRepo::with_origin();
        // Equal tips: local wins.
        assert_eq!(select_default_target(&repo.path(), "main").unwrap().reference, "main");

        // origin/main ahead (PR-driven): origin wins.
        repo.push_commit_to_origin("main", "upstream.txt");
        repo.git(&["fetch", "origin"]);
        let target = select_default_target(&repo.path(), "main").unwrap();
        assert_eq!(target.reference, "origin/main");
        assert_eq!(target.sha, repo.sha("origin/main"));
        assert!(!target.diverged);

        // Local ahead (Gitflow-style): local wins.
        repo.git(&["merge", "--ff-only", "origin/main"]);
        repo.commit("local.txt");
        let target = select_default_target(&repo.path(), "main").unwrap();
        assert_eq!(target.reference, "main");
        assert!(!target.diverged);
    }

    #[test]
    fn diverged_tips_select_origin_and_say_so() {
        let repo = TestRepo::with_origin();
        repo.commit("local.txt");
        repo.push_commit_to_origin("main", "upstream.txt");
        repo.git(&["fetch", "origin"]);

        let target = select_default_target(&repo.path(), "main").unwrap();
        assert_eq!(target.reference, "origin/main");
        assert!(target.diverged);
    }
}
