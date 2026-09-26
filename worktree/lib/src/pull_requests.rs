//! Open pull requests for the `wt list` badges, with a freshness window and a
//! stored fallback.
//!
//! The results are persisted per repository beside the comparison cache as
//! `<repo hash>.prs.json` (see [`crate::cache::repo_cache_file`]) with their
//! fetch time. Within [`FRESHNESS_WINDOW`] of the last fetch no request is
//! made. Otherwise one repository-wide request runs under [`LIST_DEADLINE`];
//! when it fails, the stored results are shown with their age. A failure is
//! never stored, so an authentication error cannot become "no open PRs".

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::cache::{atomic_write, repo_cache_file};
use crate::error::WorktreeError;
use crate::git::git_command;

pub const PR_STORE_FORMAT_VERSION: u32 = 1;

/// How long stored results stand in for a request.
pub const FRESHNESS_WINDOW: Duration = Duration::from_secs(60);

/// How long `wt list` waits for the provider.
pub const LIST_DEADLINE: Duration = Duration::from_millis(300);

/// One open PR, as stored and matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenPullRequest {
    pub number: u64,
    pub url: Option<String>,
    /// `owner/repo` of the repository the head branch lives in.
    pub source_repo: Option<String>,
    pub source_branch: String,
    pub target_branch: String,
}

/// A repository-wide open-PR request.
///
/// A trait so tests can script answers and count requests.
pub trait OpenPrSource {
    /// `owner/repo` of origin: the source repository of a same-repository PR.
    fn source_repo(&self) -> Option<String>;
    /// Every open PR, or why there is no answer. Never an empty list for a
    /// failure.
    fn fetch(&self) -> Result<Vec<OpenPullRequest>, String>;
}

/// [`OpenPrSource`] through sniff's blocking provider client.
#[derive(Debug, Clone)]
pub struct SniffOpenPrSource {
    pub remote_url: String,
    pub deadline: Duration,
}

impl SniffOpenPrSource {
    /// The source for the current repository's `origin`, or `None` without one.
    pub fn for_origin() -> Option<Self> {
        let remote_url = git_command(&["remote", "get-url", "origin"]).ok()?;
        Some(Self {
            remote_url,
            deadline: LIST_DEADLINE,
        })
    }
}

impl OpenPrSource for SniffOpenPrSource {
    fn source_repo(&self) -> Option<String> {
        sniff::filesystem::git::repository_link(&self.remote_url).map(|link| link.owner_repo)
    }

    fn fetch(&self) -> Result<Vec<OpenPullRequest>, String> {
        let summaries = sniff::remote::blocking::open_pull_requests(&self.remote_url, self.deadline)
            .map_err(|reason| reason.to_string())?;
        Ok(summaries
            .into_iter()
            .filter_map(|summary| {
                Some(OpenPullRequest {
                    number: summary.number,
                    url: summary.html_url,
                    source_repo: summary.source_repo,
                    source_branch: summary.source_branch?,
                    target_branch: summary.target_branch?,
                })
            })
            .collect())
    }
}

#[derive(Serialize, Deserialize)]
struct StoreFile {
    format_version: u32,
    /// Seconds since the Unix epoch.
    fetched_at: u64,
    source_repo: Option<String>,
    pull_requests: Vec<OpenPullRequest>,
}

/// The open PRs `wt list` shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrListing {
    /// `owner/repo` of origin; a PR from any other repository matches no
    /// local branch.
    pub source_repo: Option<String>,
    pub pull_requests: Vec<OpenPullRequest>,
    /// When the results were fetched (Unix seconds); `None` when nothing is
    /// known.
    pub fetched_at: Option<u64>,
    /// The request failed or missed its deadline, so these are stored
    /// results shown with their age.
    pub stale: bool,
}

impl PrListing {
    /// The open PRs whose head is `branch` in origin's own repository.
    pub fn for_branch<'a>(&'a self, branch: &'a str) -> impl Iterator<Item = &'a OpenPullRequest> + 'a {
        self.pull_requests.iter().filter(move |pr| {
            pr.source_branch == branch
                && matches!(
                    (&pr.source_repo, &self.source_repo),
                    (Some(theirs), Some(ours)) if theirs.eq_ignore_ascii_case(ours)
                )
        })
    }

    /// Whole minutes since the fetch, for the PR age line.
    pub fn age_minutes(&self, now: u64) -> Option<u64> {
        self.fetched_at.map(|fetched| now.saturating_sub(fetched) / 60)
    }
}

/// Where a PR badge goes: the column whose target is the PR's base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrPlacement {
    /// After the `-> parent` cell.
    Parent,
    /// After the `-> {default}` cell.
    Default,
    /// Beside the branch name, with the PR's target named.
    BesideBranch,
}

/// Places a PR targeting `target` for a branch whose fork parent is `parent`.
pub fn placement(target: &str, parent: Option<&str>, default_branch: &str) -> PrPlacement {
    if parent.is_some_and(|parent| parent != default_branch && parent == target) {
        PrPlacement::Parent
    } else if target == default_branch {
        PrPlacement::Default
    } else {
        PrPlacement::BesideBranch
    }
}

/// The store file for the repository whose main worktree is `repo_root`.
pub fn pr_store_path(repo_root: &Path) -> Result<PathBuf, WorktreeError> {
    repo_cache_file(repo_root, "prs.json")
}

/// Seconds since the Unix epoch.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

/// The open PRs for the repository whose store is at `store`.
///
/// `connect` builds the source only when a request is due, so a fresh store
/// costs no git call either. It returns `None` without an origin remote,
/// which shows no badges.
pub fn open_pull_requests(
    store: &Path,
    now: u64,
    connect: impl FnOnce() -> Option<Box<dyn OpenPrSource>>,
) -> PrListing {
    let stored = load(store);
    if let Some(file) = &stored
        && now.saturating_sub(file.fetched_at) < FRESHNESS_WINDOW.as_secs()
        && file.fetched_at <= now
    {
        return listing(file, false);
    }

    let Some(source) = connect() else {
        return PrListing::default();
    };
    match source.fetch() {
        Ok(pull_requests) => {
            let file = StoreFile {
                format_version: PR_STORE_FORMAT_VERSION,
                fetched_at: now,
                source_repo: source.source_repo(),
                pull_requests,
            };
            let _ = save(store, &file);
            listing(&file, false)
        }
        Err(_) => stored.map(|file| listing(&file, true)).unwrap_or_default(),
    }
}

fn listing(file: &StoreFile, stale: bool) -> PrListing {
    PrListing {
        source_repo: file.source_repo.clone(),
        pull_requests: file.pull_requests.clone(),
        fetched_at: Some(file.fetched_at),
        stale,
    }
}

fn load(path: &Path) -> Option<StoreFile> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice::<StoreFile>(&bytes)
        .ok()
        .filter(|file| file.format_version == PR_STORE_FORMAT_VERSION)
}

fn save(path: &Path, file: &StoreFile) -> Result<(), WorktreeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write(path, &serde_json::to_vec_pretty(file)?)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    const NOW: u64 = 1_790_000_000;

    fn pr(number: u64, repo: Option<&str>, branch: &str, target: &str) -> OpenPullRequest {
        OpenPullRequest {
            number,
            url: Some(format!("https://github.com/o/r/pull/{number}")),
            source_repo: repo.map(str::to_string),
            source_branch: branch.to_string(),
            target_branch: target.to_string(),
        }
    }

    /// A scripted source that counts its requests.
    struct Stub {
        answer: Result<Vec<OpenPullRequest>, String>,
        calls: Rc<Cell<usize>>,
    }

    impl OpenPrSource for Stub {
        fn source_repo(&self) -> Option<String> {
            Some("o/r".to_string())
        }
        fn fetch(&self) -> Result<Vec<OpenPullRequest>, String> {
            self.calls.set(self.calls.get() + 1);
            self.answer.clone()
        }
    }

    fn stub(answer: Result<Vec<OpenPullRequest>, String>) -> (Rc<Cell<usize>>, impl FnOnce() -> Option<Box<dyn OpenPrSource>>) {
        let calls = Rc::new(Cell::new(0));
        let counter = Rc::clone(&calls);
        (calls, move || Some(Box::new(Stub { answer, calls: counter }) as Box<dyn OpenPrSource>))
    }

    #[test]
    fn a_fetch_is_stored_and_a_fresh_store_skips_the_network() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("nested").join("repo.prs.json");

        let (calls, connect) = stub(Ok(vec![pr(99, Some("o/r"), "fix/x", "main")]));
        let first = open_pull_requests(&store, NOW, connect);
        assert_eq!(calls.get(), 1);
        assert!(!first.stale);
        assert_eq!(first.fetched_at, Some(NOW));
        assert_eq!(first.pull_requests.len(), 1);

        let (calls, connect) = stub(Err("must not be asked".into()));
        let second = open_pull_requests(&store, NOW + 59, connect);
        assert_eq!(calls.get(), 0, "a store younger than 60 s must skip the request");
        assert_eq!(second, first);

        // Read, write, read: the second fetch replaces the first.
        let (calls, connect) = stub(Ok(vec![pr(104, Some("o/r"), "feat/y", "feat/theme")]));
        let third = open_pull_requests(&store, NOW + 60, connect);
        assert_eq!(calls.get(), 1, "a 60 s old store must be refreshed");
        assert_eq!(third.pull_requests[0].number, 104);
        let (_, connect) = stub(Err("offline".into()));
        assert_eq!(open_pull_requests(&store, NOW + 61, connect), third);
    }

    #[test]
    fn a_failed_request_shows_the_stored_results_with_their_age() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("repo.prs.json");
        let (_, connect) = stub(Ok(vec![pr(99, Some("o/r"), "fix/x", "main")]));
        open_pull_requests(&store, NOW, connect);

        let (calls, connect) = stub(Err("provider denied the query: 401".into()));
        let listing = open_pull_requests(&store, NOW + 12 * 60 + 5, connect);
        assert_eq!(calls.get(), 1);
        assert!(listing.stale);
        assert_eq!(listing.pull_requests[0].number, 99);
        assert_eq!(listing.age_minutes(NOW + 12 * 60 + 5), Some(12));

        // The failure was not stored: the fetch time is still the first one.
        let (_, connect) = stub(Err("still offline".into()));
        assert_eq!(open_pull_requests(&store, NOW + 3600, connect).fetched_at, Some(NOW));
    }

    #[test]
    fn a_failure_with_nothing_stored_shows_no_badges_and_stores_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("repo.prs.json");
        let (_, connect) = stub(Err("timeout".into()));
        assert_eq!(open_pull_requests(&store, NOW, connect), PrListing::default());
        assert!(!store.exists(), "an unavailable answer must never be cached as an empty list");
    }

    #[test]
    fn no_origin_shows_no_badges() {
        let dir = tempfile::tempdir().unwrap();
        let listing = open_pull_requests(&dir.path().join("repo.prs.json"), NOW, || None);
        assert_eq!(listing, PrListing::default());
    }

    #[test]
    fn corrupt_other_version_and_future_stores_are_refetched() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("repo.prs.json");
        for contents in [
            b"{not json".to_vec(),
            serde_json::to_vec(&StoreFile {
                format_version: PR_STORE_FORMAT_VERSION + 1,
                fetched_at: NOW,
                source_repo: None,
                pull_requests: Vec::new(),
            })
            .unwrap(),
            // A clock set back must not freeze the store.
            serde_json::to_vec(&StoreFile {
                format_version: PR_STORE_FORMAT_VERSION,
                fetched_at: NOW + 3600,
                source_repo: None,
                pull_requests: Vec::new(),
            })
            .unwrap(),
        ] {
            fs::write(&store, contents).unwrap();
            let (calls, connect) = stub(Ok(Vec::new()));
            open_pull_requests(&store, NOW, connect);
            assert_eq!(calls.get(), 1);
        }
    }

    #[test]
    fn prs_match_by_source_repository_and_branch() {
        let listing = PrListing {
            source_repo: Some("Owner/Repo".into()),
            pull_requests: vec![
                pr(1, Some("owner/repo"), "fix/x", "main"),
                pr(2, Some("someone/fork"), "fix/x", "main"),
                pr(3, None, "fix/x", "main"),
                pr(4, Some("owner/repo"), "fix/y", "main"),
            ],
            fetched_at: Some(NOW),
            stale: false,
        };
        let numbers: Vec<u64> = listing.for_branch("fix/x").map(|pr| pr.number).collect();
        assert_eq!(numbers, [1], "a fork's same-named branch must not get the badge");

        let unknown_origin = PrListing { source_repo: None, ..listing };
        assert_eq!(unknown_origin.for_branch("fix/x").count(), 0);
    }

    #[test]
    fn badges_follow_the_prs_target() {
        use PrPlacement::*;
        assert_eq!(placement("feat/theme", Some("feat/theme"), "main"), Parent);
        assert_eq!(placement("main", Some("feat/theme"), "main"), Default);
        assert_eq!(placement("main", Some("main"), "main"), Default);
        assert_eq!(placement("main", None, "main"), Default);
        assert_eq!(placement("release/2", Some("feat/theme"), "main"), BesideBranch);
        assert_eq!(placement("release/2", None, "main"), BesideBranch);
    }

    #[test]
    fn store_file_sits_beside_the_comparison_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = crate::cache::cache_path(dir.path()).unwrap();
        let store = pr_store_path(dir.path()).unwrap();
        assert_eq!(cache.parent(), store.parent());
        let stem = cache.file_stem().unwrap().to_string_lossy().into_owned();
        assert_eq!(store.file_name().unwrap().to_string_lossy(), format!("{stem}.prs.json"));
    }
}
