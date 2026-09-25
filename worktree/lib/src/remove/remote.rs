//! `--force-remote`: find the branch's copy on origin, report what only it
//! holds, and delete it under a lease.

use std::path::Path;

use crate::git::git_from;

use super::live_remote::{PUSH_DEADLINE, RemoteHeads, run_noninteractive};
use super::safety::Commit;

/// The branch's name on origin: its configured upstream when that is on
/// origin, else its own name. `None` when there is no `origin` remote.
///
/// A PR from another repository never names the destination.
pub fn remote_destination(base: &Path, branch: &str) -> Option<String> {
    git_from(base, base, &["remote", "get-url", "origin"]).ok()?;
    let config = |key: &str| {
        git_from(base, base, &["config", "--get", &format!("branch.{branch}.{key}")])
            .ok()
            .filter(|value| !value.is_empty())
    };
    let upstream = (config("remote").as_deref() == Some("origin"))
        .then(|| config("merge"))
        .flatten()
        .and_then(|merge| merge.strip_prefix("refs/heads/").map(str::to_string));
    Some(upstream.unwrap_or_else(|| branch.to_string()))
}

/// What deleting the branch on origin would do, as observed live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteState {
    /// No `origin` remote.
    NoRemote,
    /// Origin answered and has no such branch.
    Absent { destination: String },
    Present {
        destination: String,
        /// The live head; the deletion's lease is taken against it.
        sha: String,
        remote_only: RemoteOnly,
    },
    /// Origin could not be asked.
    Unavailable { destination: String, reason: String },
}

/// Commits on the remote branch that the local branch lacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteOnly {
    Known(Vec<Commit>),
    /// The live head is not in the local object store, so neither the count
    /// nor the contents can be told.
    Unknown,
}

/// Queries origin's live head for the branch and lists remote-only commits.
pub fn preflight_remote_deletion(
    base: &Path,
    branch: &str,
    tip: &str,
    heads: &dyn RemoteHeads,
) -> RemoteState {
    let Some(destination) = remote_destination(base, branch) else {
        return RemoteState::NoRemote;
    };
    match heads.live_head(&destination) {
        Ok(None) => RemoteState::Absent { destination },
        Err(reason) => RemoteState::Unavailable { destination, reason },
        Ok(Some(sha)) => {
            let remote_only = remote_only_commits(base, &sha, tip);
            RemoteState::Present {
                destination,
                sha,
                remote_only,
            }
        }
    }
}

fn remote_only_commits(base: &Path, remote_sha: &str, tip: &str) -> RemoteOnly {
    let object = format!("{remote_sha}^{{commit}}");
    if git_from(base, base, &["cat-file", "-e", &object]).is_err() {
        return RemoteOnly::Unknown;
    }
    let range = format!("{tip}..{remote_sha}");
    match git_from(base, base, &["log", "--format=%h%x1f%s", &range]) {
        Ok(out) => RemoteOnly::Known(
            out.lines()
                .filter_map(|line| {
                    let (short_sha, subject) = line.split_once('\u{1f}')?;
                    Some(Commit {
                        short_sha: short_sha.to_string(),
                        subject: subject.to_string(),
                    })
                })
                .collect(),
        ),
        Err(_) => RemoteOnly::Unknown,
    }
}

/// Deletes `refs/heads/<destination>` on origin only if it still points at
/// `observed_sha`, so a push made after the report fails the deletion instead
/// of losing the new commits.
///
/// ## Errors
///
/// Git's reason: a failed lease, a protected branch, or an unreachable origin.
pub fn delete_remote_branch(base: &Path, destination: &str, observed_sha: &str) -> Result<(), String> {
    let refname = format!("refs/heads/{destination}");
    let lease = format!("--force-with-lease={refname}:{observed_sha}");
    let refspec = format!(":{refname}");
    run_noninteractive(
        base,
        &["push", "--porcelain", &lease, "origin", &refspec],
        PUSH_DEADLINE,
    )
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remove::live_remote::{LIVE_CHECK_DEADLINE, LsRemote};
    use crate::remove::test_support::TestRepo;

    fn heads(repo: &TestRepo) -> LsRemote<'static> {
        LsRemote {
            base: Box::leak(Box::new(repo.path())),
            deadline: LIVE_CHECK_DEADLINE,
        }
    }

    #[test]
    fn destination_is_the_origin_upstream_else_the_branch_name() {
        let repo = TestRepo::new();
        assert_eq!(remote_destination(&repo.path(), "feat/x"), None, "no origin");

        let repo = TestRepo::with_origin();
        repo.git(&["branch", "feat/x"]);
        assert_eq!(remote_destination(&repo.path(), "feat/x").as_deref(), Some("feat/x"));

        repo.git(&["push", "-q", "origin", "feat/x:refs/heads/renamed"]);
        repo.git(&["branch", "-u", "origin/renamed", "feat/x"]);
        assert_eq!(remote_destination(&repo.path(), "feat/x").as_deref(), Some("renamed"));

        // An upstream on another remote is ignored.
        repo.git(&["remote", "add", "fork", repo.origin_path().to_str().unwrap()]);
        repo.git(&["fetch", "-q", "fork"]);
        repo.git(&["branch", "-u", "fork/renamed", "feat/x"]);
        assert_eq!(remote_destination(&repo.path(), "feat/x").as_deref(), Some("feat/x"));
    }

    #[test]
    fn preflight_reports_absent_present_and_remote_only_commits() {
        let repo = TestRepo::with_origin();
        let tip = repo.sha("main");
        repo.git(&["branch", "feat/x"]);
        assert_eq!(
            preflight_remote_deletion(&repo.path(), "feat/x", &tip, &heads(&repo)),
            RemoteState::Absent { destination: "feat/x".into() }
        );

        repo.git(&["push", "-q", "origin", "feat/x"]);
        let state = preflight_remote_deletion(&repo.path(), "feat/x", &tip, &heads(&repo));
        assert_eq!(
            state,
            RemoteState::Present {
                destination: "feat/x".into(),
                sha: tip.clone(),
                remote_only: RemoteOnly::Known(Vec::new()),
            }
        );

        // Someone pushed a commit we never fetched: its contents are unknown.
        let theirs = repo.push_commit_to_origin("feat/x", "theirs.txt");
        let RemoteState::Present { sha, remote_only, .. } =
            preflight_remote_deletion(&repo.path(), "feat/x", &tip, &heads(&repo))
        else {
            panic!("expected present");
        };
        assert_eq!(sha, theirs);
        assert_eq!(remote_only, RemoteOnly::Unknown);

        // After a fetch the same commit is listed.
        repo.git(&["fetch", "-q", "origin"]);
        let RemoteState::Present { remote_only, .. } =
            preflight_remote_deletion(&repo.path(), "feat/x", &tip, &heads(&repo))
        else {
            panic!("expected present");
        };
        let RemoteOnly::Known(commits) = remote_only else {
            panic!("expected known commits");
        };
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].subject, "add theirs.txt");
    }

    #[test]
    fn deletion_succeeds_under_the_observed_lease() {
        let repo = TestRepo::with_origin();
        repo.git(&["branch", "feat/x"]);
        repo.git(&["push", "-q", "origin", "feat/x"]);
        let observed = repo.sha("feat/x");

        delete_remote_branch(&repo.path(), "feat/x", &observed).unwrap();
        assert_eq!(heads(&repo).live_head("feat/x"), Ok(None));
    }

    #[test]
    fn a_push_between_preflight_and_deletion_fails_the_lease_and_keeps_the_new_head() {
        let repo = TestRepo::with_origin();
        repo.git(&["branch", "feat/x"]);
        repo.git(&["push", "-q", "origin", "feat/x"]);
        let RemoteState::Present { sha: observed, .. } =
            preflight_remote_deletion(&repo.path(), "feat/x", &repo.sha("feat/x"), &heads(&repo))
        else {
            panic!("expected present");
        };

        let theirs = repo.push_commit_to_origin("feat/x", "late.txt");
        let error = delete_remote_branch(&repo.path(), "feat/x", &observed).unwrap_err();
        assert!(!error.is_empty());
        assert_eq!(heads(&repo).live_head("feat/x"), Ok(Some(theirs)));
    }

    #[test]
    fn an_unreachable_origin_is_unavailable() {
        let repo = TestRepo::new();
        repo.git(&["remote", "add", "origin", "/nonexistent/origin.git"]);
        let state = preflight_remote_deletion(&repo.path(), "main", &repo.sha("main"), &heads(&repo));
        assert!(matches!(state, RemoteState::Unavailable { .. }), "{state:?}");
        assert!(delete_remote_branch(&repo.path(), "main", &repo.sha("main")).is_err());
    }
}
