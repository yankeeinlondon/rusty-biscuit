//! `--force-remote`: find the branch's copy on origin, report what only it
//! holds, and delete it under a lease.
//!
//! `origin` is a nickname whose URLs can change between the report and the
//! deletion, and `ls-remote origin` reads the fetch URL while `push origin`
//! follows the push URL. So the live check and the deletion both address
//! origin's resolved push endpoint ([`push_endpoints`]), which the handoff
//! record also stores (Decision 21, amended by review 3).

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

/// Where `git push origin` writes: origin's push URLs as git resolves them
/// (`pushurl` over `url`, after `insteadOf`/`pushInsteadOf` rewriting),
/// spelled as git prints them. Relative paths resolve against `base`, which
/// is where every network call runs.
///
/// ## Errors
///
/// Git's reason, for example when there is no `origin` remote.
pub fn push_endpoints(base: &Path) -> Result<Vec<String>, String> {
    let out = git_from(base, base, &["remote", "get-url", "--push", "--all", "origin"])
        .map_err(|e| e.to_string())?;
    Ok(out.lines().filter(|line| !line.is_empty()).map(str::to_string).collect())
}

/// What deleting the branch on origin would do, as observed live.
///
/// `endpoint` is the push URL both the observation and the deletion use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteState {
    /// No `origin` remote.
    NoRemote,
    /// Origin pushes to more than one repository. `--force-remote` refuses
    /// this before removing anything rather than check each one.
    MultiplePushUrls {
        destination: String,
        endpoints: Vec<String>,
    },
    /// The endpoint answered and has no such branch.
    Absent { destination: String, endpoint: String },
    Present {
        destination: String,
        endpoint: String,
        /// The live head; the deletion's lease is taken against it.
        sha: String,
        remote_only: RemoteOnly,
    },
    /// The endpoint could not be asked.
    Unavailable {
        destination: String,
        endpoint: String,
        reason: String,
    },
}

/// Commits on the remote branch that the local branch lacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteOnly {
    Known(Vec<Commit>),
    /// The live head is not in the local object store, so neither the count
    /// nor the contents can be told.
    Unknown,
}

/// Queries the push endpoint's live head for the branch and lists
/// remote-only commits.
pub fn preflight_remote_deletion(
    base: &Path,
    branch: &str,
    tip: &str,
    heads: &dyn RemoteHeads,
) -> RemoteState {
    let Some(destination) = remote_destination(base, branch) else {
        return RemoteState::NoRemote;
    };
    let endpoint = match push_endpoints(base) {
        Ok(mut endpoints) if endpoints.len() == 1 => endpoints.remove(0),
        Ok(endpoints) if endpoints.len() > 1 => {
            return RemoteState::MultiplePushUrls {
                destination,
                endpoints,
            };
        }
        other => {
            return RemoteState::Unavailable {
                destination,
                endpoint: String::new(),
                reason: other.err().unwrap_or_else(|| "origin has no push URL".to_string()),
            };
        }
    };
    match heads.live_head(&endpoint, &destination) {
        Ok(None) => RemoteState::Absent { destination, endpoint },
        Err(reason) => RemoteState::Unavailable {
            destination,
            endpoint,
            reason,
        },
        Ok(Some(sha)) => {
            let remote_only = remote_only_commits(base, &sha, tip);
            RemoteState::Present {
                destination,
                endpoint,
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

/// Deletes `refs/heads/<destination>` at `endpoint` only if it still points
/// at `observed_sha`, so a push made after the report fails the deletion
/// instead of losing the new commits.
///
/// ## Errors
///
/// Git's reason: a failed lease, a protected branch, or an unreachable
/// endpoint.
pub fn delete_remote_branch(
    base: &Path,
    endpoint: &str,
    destination: &str,
    observed_sha: &str,
) -> Result<(), String> {
    let refname = format!("refs/heads/{destination}");
    let lease = format!("--force-with-lease={refname}:{observed_sha}");
    let refspec = format!(":{refname}");
    run_noninteractive(
        base,
        &["push", "--porcelain", &lease, endpoint, &refspec],
        PUSH_DEADLINE,
    )?;
    // A push to a URL, unlike one to `origin`, leaves the remote-tracking ref
    // behind; drop it as `git push origin --delete` would.
    let _ = git_from(
        base,
        base,
        &["update-ref", "-d", &format!("refs/remotes/origin/{destination}")],
    );
    Ok(())
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

    fn origin_url(repo: &TestRepo) -> String {
        repo.origin_path().to_str().unwrap().to_string()
    }

    #[test]
    fn push_endpoints_follow_pushurl_and_list_every_push_url() {
        let repo = TestRepo::new();
        assert!(push_endpoints(&repo.path()).is_err(), "no origin");

        let repo = TestRepo::with_origin();
        assert_eq!(push_endpoints(&repo.path()), Ok(vec![origin_url(&repo)]));

        repo.git(&["remote", "set-url", "--push", "origin", "/elsewhere/push.git"]);
        assert_eq!(push_endpoints(&repo.path()), Ok(vec!["/elsewhere/push.git".to_string()]));

        repo.git(&["remote", "set-url", "--add", "--push", "origin", "/elsewhere/second.git"]);
        assert_eq!(
            push_endpoints(&repo.path()),
            Ok(vec!["/elsewhere/push.git".to_string(), "/elsewhere/second.git".to_string()])
        );

        // pushInsteadOf rewrites the URL git would actually push to.
        repo.git(&["config", "--unset-all", "remote.origin.pushurl"]);
        repo.git(&["config", "remote.origin.url", "short:repo.git"]);
        repo.git(&["config", "url./rewritten/.pushInsteadOf", "short:"]);
        assert_eq!(push_endpoints(&repo.path()), Ok(vec!["/rewritten/repo.git".to_string()]));
    }

    #[test]
    fn several_push_urls_are_reported_without_asking_any() {
        let repo = TestRepo::with_origin();
        repo.git(&["remote", "set-url", "--add", "--push", "origin", "/elsewhere/second.git"]);
        repo.git(&["remote", "set-url", "--add", "--push", "origin", &origin_url(&repo)]);
        let state = preflight_remote_deletion(&repo.path(), "main", &repo.sha("main"), &heads(&repo));
        let RemoteState::MultiplePushUrls { destination, endpoints } = state else {
            panic!("expected several push URLs, got {state:?}");
        };
        assert_eq!(destination, "main");
        assert_eq!(endpoints.len(), 2);
    }

    #[test]
    fn preflight_reports_absent_present_and_remote_only_commits() {
        let repo = TestRepo::with_origin();
        let endpoint = origin_url(&repo);
        let tip = repo.sha("main");
        repo.git(&["branch", "feat/x"]);
        assert_eq!(
            preflight_remote_deletion(&repo.path(), "feat/x", &tip, &heads(&repo)),
            RemoteState::Absent {
                destination: "feat/x".into(),
                endpoint: endpoint.clone(),
            }
        );

        repo.git(&["push", "-q", "origin", "feat/x"]);
        let state = preflight_remote_deletion(&repo.path(), "feat/x", &tip, &heads(&repo));
        assert_eq!(
            state,
            RemoteState::Present {
                destination: "feat/x".into(),
                endpoint,
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
        repo.git(&["fetch", "-q", "origin"]);
        let observed = repo.sha("feat/x");

        delete_remote_branch(&repo.path(), &origin_url(&repo), "feat/x", &observed).unwrap();
        assert_eq!(heads(&repo).live_head("origin", "feat/x"), Ok(None));
        assert!(
            repo.try_git(&["rev-parse", "--verify", "--quiet", "refs/remotes/origin/feat/x"]).is_err(),
            "the remote-tracking ref goes too, as with `git push origin --delete`"
        );
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
        let error =
            delete_remote_branch(&repo.path(), &origin_url(&repo), "feat/x", &observed).unwrap_err();
        assert!(!error.is_empty());
        assert_eq!(heads(&repo).live_head("origin", "feat/x"), Ok(Some(theirs)));
    }

    #[test]
    fn an_unreachable_origin_is_unavailable() {
        let repo = TestRepo::new();
        repo.git(&["remote", "add", "origin", "/nonexistent/origin.git"]);
        let state = preflight_remote_deletion(&repo.path(), "main", &repo.sha("main"), &heads(&repo));
        assert!(matches!(state, RemoteState::Unavailable { .. }), "{state:?}");
        assert!(
            delete_remote_branch(&repo.path(), "/nonexistent/origin.git", "main", &repo.sha("main"))
                .is_err()
        );
    }
}
