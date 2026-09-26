//! Open pull requests for the `wt list` badges, with a freshness window and
//! a stored answer bound to `origin`.
//!
//! The results are persisted per repository beside the comparison cache as
//! `<repo hash>.prs.json` (see [`crate::cache::repo_cache_file`]) with their
//! fetch time and a digest of the exact `origin` URL they were fetched for.
//! [`select_cached`] serves a stored answer only while `origin` still matches;
//! one older than [`FRESHNESS_WINDOW`] is served as stale, for [`refresh`] to
//! replace in another process. With no usable answer, [`fetch_and_publish`]
//! makes the request under [`LIST_DEADLINE`]. A failure is never stored, so an
//! authentication error cannot become "no open PRs".

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::cache::{atomic_write, repo_cache_file};
use crate::error::WorktreeError;
use crate::git::git_from;

pub const PR_STORE_FORMAT_VERSION: u32 = 2;

/// How long stored results stand in without a refresh.
pub const FRESHNESS_WINDOW: Duration = Duration::from_secs(60);

/// How long `wt list` waits for the provider when it has no stored answer.
pub const LIST_DEADLINE: Duration = Duration::from_millis(300);

/// How long a background refresh waits for the provider; nothing waits on it.
pub const REFRESH_DEADLINE: Duration = Duration::from_secs(10);

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

/// `origin`'s URL as the repository at `repo_root` reports it, or `None`
/// without one.
///
/// A stored answer is bound to this exact value; pass the main checkout so
/// every caller reads the same configuration.
pub fn origin_url(repo_root: &Path) -> Option<String> {
    git_from(repo_root, repo_root, &["remote", "get-url", "origin"])
        .ok()
        .filter(|url| !url.is_empty())
}

/// The digest a store records for `origin`. The URL itself is never stored,
/// since it can carry credentials.
pub fn origin_digest(origin: &str) -> String {
    biscuit_hash::blake3_hash(origin)
}

#[derive(Serialize, Deserialize)]
struct StoreFile {
    format_version: u32,
    /// [`origin_digest`] of the URL the request was made for.
    origin_digest: String,
    /// Seconds since the Unix epoch at which the request started.
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

    /// Whether these results are at least [`FRESHNESS_WINDOW`] old at `now`.
    /// Unknown and future fetch times are not stale.
    pub fn is_stale_at(&self, now: u64) -> bool {
        self.fetched_at
            .is_some_and(|fetched| fetched <= now && now - fetched >= FRESHNESS_WINDOW.as_secs())
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

/// A stored answer for the current `origin`, if there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachedPrs {
    /// Younger than [`FRESHNESS_WINDOW`]: show it and make no request.
    Fresh(PrListing),
    /// Show it at once, and [`refresh`] it in the background.
    Stale(PrListing),
    /// No usable answer: none stored, unreadable, another format, fetched in
    /// the future, bound to another `origin`, or no `origin` at all.
    Miss,
}

/// What the stored answer at `store` offers when `origin` is the current
/// [`origin_url`]. A valid empty answer is an answer, not a miss.
pub fn select_cached(store: &Path, origin: Option<&str>, now: u64) -> CachedPrs {
    let Some(origin) = origin else {
        return CachedPrs::Miss;
    };
    let Some(file) = load(store) else {
        return CachedPrs::Miss;
    };
    if file.fetched_at > now || file.origin_digest != origin_digest(origin) {
        return CachedPrs::Miss;
    }
    let listing = listing(&file);
    if listing.is_stale_at(now) {
        CachedPrs::Stale(listing)
    } else {
        CachedPrs::Fresh(listing)
    }
}

/// Makes the request for `origin` and stores a successful answer, stamped
/// `now`, while `repo_root`'s `origin` still matches.
///
/// `None` when the request failed or `origin` changed during it; the store is
/// then untouched. An answer for a previous `origin` is never returned, so
/// this run cannot show badges from another repository.
pub fn fetch_and_publish(
    store: &Path,
    repo_root: &Path,
    origin: &str,
    now: u64,
    source: &dyn OpenPrSource,
) -> Option<PrListing> {
    let file = fetch(origin, now, source).ok()?;
    if origin_url(repo_root).as_deref() != Some(origin) {
        return None;
    }
    let _ = save(store, &file);
    Some(listing(&file))
}

/// Why a [`refresh`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshOutcome {
    /// A new answer was stored.
    Refreshed,
    /// Another refresh stored a fresh answer first; no request was made.
    AlreadyFresh,
    /// Another refresh holds the lock; no request was made.
    Contended,
    /// The lock file could not be opened or locked; no request was made.
    LockFailed,
    /// `origin` is missing; no request was made.
    NoOrigin,
    /// `origin` changed during the request, so the answer was discarded.
    OriginChanged,
    /// The request failed; the store is untouched.
    Failed,
    /// The answer could not be written; the store is untouched.
    PublishFailed,
}

/// Replaces the stored answer for the repository at `repo_root` unless
/// another process is refreshing it or already has.
///
/// Holds a nonblocking exclusive lock on the persistent sidecar
/// [`pr_lock_path`] from the freshness recheck through publication, so
/// concurrent refreshes make at most one request, and a crashed holder's lock
/// is released by the OS. The sidecar is never deleted: unlinking it would let
/// a new process lock a different file while the old lock is held. `clock` is
/// read before the request, so the stored age errs old.
pub fn refresh(
    store: &Path,
    repo_root: &Path,
    clock: impl Fn() -> u64,
    connect: impl FnOnce(&str) -> Box<dyn OpenPrSource>,
) -> RefreshOutcome {
    let Some(_lock) = (match try_lock(&pr_lock_path(store)) {
        Ok(lock) => lock,
        Err(_) => return RefreshOutcome::LockFailed,
    }) else {
        return RefreshOutcome::Contended;
    };

    let Some(origin) = origin_url(repo_root) else {
        return RefreshOutcome::NoOrigin;
    };
    let started = clock();
    if matches!(select_cached(store, Some(&origin), started), CachedPrs::Fresh(_)) {
        return RefreshOutcome::AlreadyFresh;
    }
    let Ok(file) = fetch(&origin, started, connect(&origin).as_ref()) else {
        return RefreshOutcome::Failed;
    };
    if origin_url(repo_root).as_deref() != Some(origin.as_str()) {
        return RefreshOutcome::OriginChanged;
    }
    match save(store, &file) {
        Ok(()) => RefreshOutcome::Refreshed,
        Err(_) => RefreshOutcome::PublishFailed,
    }
}

/// The store file for the repository whose main worktree is `repo_root`.
pub fn pr_store_path(repo_root: &Path) -> Result<PathBuf, WorktreeError> {
    repo_cache_file(repo_root, "prs.json")
}

/// The refresh lock sidecar for the store at `store`: `<repo hash>.prs.lock`.
pub fn pr_lock_path(store: &Path) -> PathBuf {
    store.with_extension("lock")
}

/// Seconds since the Unix epoch.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

/// The locked sidecar, `None` when another process holds it. Dropping the
/// file releases the lock.
fn try_lock(path: &Path) -> std::io::Result<Option<fs::File>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    if fs4::fs_std::FileExt::try_lock_exclusive(&file)? {
        Ok(Some(file))
    } else {
        Ok(None)
    }
}

fn fetch(origin: &str, fetched_at: u64, source: &dyn OpenPrSource) -> Result<StoreFile, String> {
    Ok(StoreFile {
        format_version: PR_STORE_FORMAT_VERSION,
        origin_digest: origin_digest(origin),
        fetched_at,
        pull_requests: source.fetch()?,
        source_repo: source.source_repo(),
    })
}

fn listing(file: &StoreFile) -> PrListing {
    PrListing {
        source_repo: file.source_repo.clone(),
        pull_requests: file.pull_requests.clone(),
        fetched_at: Some(file.fetched_at),
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
    use std::cell::{Cell, RefCell};
    use std::process::Command;
    use std::rc::Rc;

    use super::*;

    const NOW: u64 = 1_790_000_000;
    // Hosts no user `url.*.insteadOf` rule plausibly rewrites, since the tests
    // compare what `git remote get-url` returns.
    const ORIGIN: &str = "https://prs.example.invalid/o/r.git";

    fn pr(number: u64, repo: Option<&str>, branch: &str, target: &str) -> OpenPullRequest {
        OpenPullRequest {
            number,
            url: Some(format!("https://github.com/o/r/pull/{number}")),
            source_repo: repo.map(str::to_string),
            source_branch: branch.to_string(),
            target_branch: target.to_string(),
        }
    }

    fn git(repo: &Path, args: &[&str]) {
        let status = Command::new("git").arg("-C").arg(repo).args(args).status().expect("git should run");
        assert!(status.success(), "git {args:?} failed");
    }

    /// A repository whose `origin` is `url` (none for `None`), and a store
    /// path beside it.
    fn repo(url: Option<&str>) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        if let Some(url) = url {
            git(&root, &["remote", "add", "origin", url]);
        }
        let store = dir.path().join("cache").join("abc.prs.json");
        (dir, root, store)
    }

    /// A scripted source that counts its requests and can act mid-request.
    struct Stub {
        answer: Result<Vec<OpenPullRequest>, String>,
        calls: Rc<Cell<usize>>,
        during: Option<Box<dyn Fn()>>,
    }

    impl OpenPrSource for Stub {
        fn source_repo(&self) -> Option<String> {
            Some("o/r".to_string())
        }
        fn fetch(&self) -> Result<Vec<OpenPullRequest>, String> {
            self.calls.set(self.calls.get() + 1);
            if let Some(during) = &self.during {
                during();
            }
            self.answer.clone()
        }
    }

    fn stub(answer: Result<Vec<OpenPullRequest>, String>) -> (Rc<Cell<usize>>, Stub) {
        let calls = Rc::new(Cell::new(0));
        (Rc::clone(&calls), Stub { answer, calls, during: None })
    }

    /// The origins a `connect` was given.
    type Origins = Rc<RefCell<Vec<String>>>;

    /// A `connect` for [`refresh`] that records the origin it was given.
    fn connector(source: Stub) -> (Origins, impl FnOnce(&str) -> Box<dyn OpenPrSource>) {
        let origins = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&origins);
        (origins, move |origin: &str| {
            seen.borrow_mut().push(origin.to_string());
            Box::new(source) as Box<dyn OpenPrSource>
        })
    }

    /// Stores `prs` for `origin`, fetched at `at`.
    fn seed(store: &Path, root: &Path, origin: &str, at: u64, prs: Vec<OpenPullRequest>) {
        let (_, source) = stub(Ok(prs));
        fetch_and_publish(store, root, origin, at, &source).expect("seeding fetch succeeds");
    }

    fn numbers(listing: &PrListing) -> Vec<u64> {
        listing.pull_requests.iter().map(|pr| pr.number).collect()
    }

    #[test]
    fn a_stored_answer_is_fresh_below_60_seconds_and_stale_from_60() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, ORIGIN, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);

        let CachedPrs::Fresh(fresh) = select_cached(&store, Some(ORIGIN), NOW + 59) else {
            panic!("59 s old must be fresh");
        };
        assert_eq!(fresh.fetched_at, Some(NOW));
        assert_eq!(numbers(&fresh), [99]);
        assert!(!fresh.is_stale_at(NOW + 59));

        let CachedPrs::Stale(stale) = select_cached(&store, Some(ORIGIN), NOW + 60) else {
            panic!("exactly 60 s old must be stale");
        };
        assert_eq!(stale, fresh, "a stale answer still shows its badges");
        assert!(stale.is_stale_at(NOW + 60));
        assert_eq!(stale.age_minutes(NOW + 12 * 60 + 5), Some(12));
    }

    #[test]
    fn a_fresh_selection_that_crosses_the_window_before_rendering_reads_stale() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, ORIGIN, NOW, Vec::new());
        let CachedPrs::Fresh(listing) = select_cached(&store, Some(ORIGIN), NOW + 59) else {
            panic!("fresh at selection");
        };
        assert!(listing.is_stale_at(NOW + 61), "age is judged at render time, not selection time");
    }

    #[test]
    fn an_answer_for_another_or_no_origin_is_a_miss_even_when_fresh() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, ORIGIN, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);

        assert_eq!(select_cached(&store, Some("https://prs.example.invalid/o/other.git"), NOW + 1), CachedPrs::Miss);
        assert_eq!(select_cached(&store, Some("git@prs.example.invalid:o/r.git"), NOW + 1), CachedPrs::Miss);
        assert_eq!(select_cached(&store, None, NOW + 1), CachedPrs::Miss);
        assert!(matches!(select_cached(&store, Some(ORIGIN), NOW + 1), CachedPrs::Fresh(_)));
    }

    #[test]
    fn a_stored_empty_answer_is_an_answer() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, ORIGIN, NOW, Vec::new());
        let expected = PrListing { source_repo: Some("o/r".into()), pull_requests: Vec::new(), fetched_at: Some(NOW) };
        assert_eq!(select_cached(&store, Some(ORIGIN), NOW + 1), CachedPrs::Fresh(expected.clone()));
        assert_eq!(select_cached(&store, Some(ORIGIN), NOW + 600), CachedPrs::Stale(expected));
    }

    #[test]
    fn corrupt_old_format_other_version_and_future_stores_are_misses() {
        let (_dir, _root, store) = repo(Some(ORIGIN));
        fs::create_dir_all(store.parent().unwrap()).unwrap();
        let file = |version: u32, fetched_at: u64| {
            serde_json::to_vec(&StoreFile {
                format_version: version,
                origin_digest: origin_digest(ORIGIN),
                fetched_at,
                source_repo: Some("o/r".into()),
                pull_requests: vec![pr(99, Some("o/r"), "fix/x", "main")],
            })
            .unwrap()
        };
        // Format 1 exactly as it was written: no origin binding.
        let version_one = serde_json::json!({
            "format_version": 1,
            "fetched_at": NOW,
            "source_repo": "o/r",
            "pull_requests": [{
                "number": 99,
                "url": "https://github.com/o/r/pull/99",
                "source_repo": "o/r",
                "source_branch": "fix/x",
                "target_branch": "main",
            }],
        });
        for contents in [
            b"{not json".to_vec(),
            serde_json::to_vec(&version_one).unwrap(),
            file(PR_STORE_FORMAT_VERSION + 1, NOW),
            // A clock set back must not freeze the store, fresh or stale.
            file(PR_STORE_FORMAT_VERSION, NOW + 1),
            file(PR_STORE_FORMAT_VERSION, NOW + 3600),
        ] {
            fs::write(&store, contents).unwrap();
            assert_eq!(select_cached(&store, Some(ORIGIN), NOW), CachedPrs::Miss);
        }
        assert_eq!(select_cached(&store.with_file_name("absent.prs.json"), Some(ORIGIN), NOW), CachedPrs::Miss);
        fs::write(&store, file(PR_STORE_FORMAT_VERSION, NOW)).unwrap();
        assert!(matches!(select_cached(&store, Some(ORIGIN), NOW), CachedPrs::Fresh(_)), "control");
    }

    #[test]
    fn the_store_records_only_a_digest_of_the_origin() {
        let secret = "https://user:hunter2@prs.example.invalid/o/r.git";
        let (_dir, root, store) = repo(Some(secret));
        assert_eq!(origin_url(&root).as_deref(), Some(secret));
        seed(&store, &root, secret, NOW, Vec::new());
        let bytes = String::from_utf8(fs::read(&store).unwrap()).unwrap();
        assert!(!bytes.contains("hunter2") && !bytes.contains("example.invalid"), "{bytes}");
        assert!(bytes.contains(&origin_digest(secret)));
    }

    #[test]
    fn origin_url_is_none_without_an_origin() {
        let (_dir, root, _store) = repo(None);
        assert_eq!(origin_url(&root), None);
    }

    #[test]
    fn a_failed_foreground_request_leaves_the_store_untouched() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        let (calls, source) = stub(Err("timeout".into()));
        assert_eq!(fetch_and_publish(&store, &root, ORIGIN, NOW, &source), None);
        assert_eq!(calls.get(), 1);
        assert!(!store.exists(), "an unavailable answer must never be cached as an empty list");

        seed(&store, &root, ORIGIN, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);
        let before = fs::read(&store).unwrap();
        let (_, source) = stub(Err("provider denied the query: 401".into()));
        assert_eq!(fetch_and_publish(&store, &root, ORIGIN, NOW + 600, &source), None);
        assert_eq!(fs::read(&store).unwrap(), before);
    }

    #[test]
    fn a_foreground_answer_is_discarded_when_origin_changed_during_the_request() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        let (calls, mut source) = stub(Ok(vec![pr(7, Some("o/r"), "fix/x", "main")]));
        let moved = root.clone();
        source.during = Some(Box::new(move || git(&moved, &["remote", "set-url", "origin", "https://prs.example.invalid/o/new.git"])));

        assert_eq!(
            fetch_and_publish(&store, &root, ORIGIN, NOW, &source),
            None,
            "this run must not show badges from the previous origin"
        );
        assert_eq!(calls.get(), 1);
        assert!(!store.exists(), "an answer for the old origin must not be bound to the new one");
    }

    #[test]
    fn a_refresh_replaces_a_stale_answer_stamped_at_its_start() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, ORIGIN, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);

        let (calls, source) = stub(Ok(vec![pr(104, Some("o/r"), "feat/y", "feat/theme")]));
        let (origins, connect) = connector(source);
        let ticks = Cell::new(NOW + 600);
        let clock = || {
            let now = ticks.get();
            ticks.set(now + 5);
            now
        };
        assert_eq!(refresh(&store, &root, clock, connect), RefreshOutcome::Refreshed);
        assert_eq!(calls.get(), 1);
        assert_eq!(*origins.borrow(), [ORIGIN], "the request is made for the exact origin value");

        // Read, write, read: the next list sees the refreshed answer as fresh.
        let CachedPrs::Fresh(listing) = select_cached(&store, Some(ORIGIN), NOW + 601) else {
            panic!("the refreshed answer must be fresh");
        };
        assert_eq!(numbers(&listing), [104]);
        assert_eq!(listing.fetched_at, Some(NOW + 600));

        let (calls, source) = stub(Ok(vec![pr(105, Some("o/r"), "feat/z", "main")]));
        let (_, connect) = connector(source);
        assert_eq!(refresh(&store, &root, || NOW + 700, connect), RefreshOutcome::Refreshed);
        assert_eq!(calls.get(), 1);
        let CachedPrs::Fresh(listing) = select_cached(&store, Some(ORIGIN), NOW + 701) else {
            panic!("fresh after the second refresh");
        };
        assert_eq!(numbers(&listing), [105]);
    }

    #[test]
    fn a_refresh_fills_a_miss_including_an_answer_for_another_origin() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        let (_, source) = stub(Ok(vec![pr(1, Some("o/r"), "a", "main")]));
        let (_, connect) = connector(source);
        assert_eq!(refresh(&store, &root, || NOW, connect), RefreshOutcome::Refreshed);

        git(&root, &["remote", "set-url", "origin", "https://prs.example.invalid/o/new.git"]);
        let (calls, source) = stub(Ok(vec![pr(2, Some("o/new"), "b", "main")]));
        let (_, connect) = connector(source);
        assert_eq!(refresh(&store, &root, || NOW + 1, connect), RefreshOutcome::Refreshed);
        assert_eq!(calls.get(), 1, "a fresh answer for the old origin does not count");
        let CachedPrs::Fresh(listing) = select_cached(&store, Some("https://prs.example.invalid/o/new.git"), NOW + 2) else {
            panic!("bound to the new origin");
        };
        assert_eq!(numbers(&listing), [2]);
    }

    #[test]
    fn a_refresh_skips_the_request_when_the_answer_is_already_fresh() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, ORIGIN, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);
        let before = fs::read(&store).unwrap();
        let (calls, source) = stub(Ok(Vec::new()));
        let (_, connect) = connector(source);
        assert_eq!(refresh(&store, &root, || NOW + 59, connect), RefreshOutcome::AlreadyFresh);
        assert_eq!(calls.get(), 0);
        assert_eq!(fs::read(&store).unwrap(), before);
    }

    #[test]
    fn a_refresh_makes_no_request_while_another_holds_the_lock() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, ORIGIN, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);
        let before = fs::read(&store).unwrap();

        let holder = try_lock(&pr_lock_path(&store)).unwrap().expect("the first lock is free");
        let (calls, source) = stub(Ok(Vec::new()));
        let (_, connect) = connector(source);
        assert_eq!(refresh(&store, &root, || NOW + 600, connect), RefreshOutcome::Contended);
        assert_eq!(calls.get(), 0);
        assert_eq!(fs::read(&store).unwrap(), before);

        drop(holder);
        assert!(pr_lock_path(&store).exists(), "the sidecar persists across holders");
        let (calls, source) = stub(Ok(Vec::new()));
        let (_, connect) = connector(source);
        assert_eq!(refresh(&store, &root, || NOW + 600, connect), RefreshOutcome::Refreshed);
        assert_eq!(calls.get(), 1, "a released lock lets the next refresh run");
        assert!(pr_lock_path(&store).exists(), "a refresh never deletes the sidecar");
    }

    #[test]
    fn a_refresh_that_cannot_open_its_lock_makes_no_request() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, ORIGIN, NOW, Vec::new());
        let before = fs::read(&store).unwrap();
        fs::create_dir_all(pr_lock_path(&store)).unwrap();
        let (calls, source) = stub(Ok(Vec::new()));
        let (_, connect) = connector(source);
        assert_eq!(refresh(&store, &root, || NOW + 600, connect), RefreshOutcome::LockFailed);
        assert_eq!(calls.get(), 0);
        assert_eq!(fs::read(&store).unwrap(), before);
    }

    #[test]
    fn a_failed_refresh_leaves_the_stored_answer_untouched() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, ORIGIN, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);
        let before = fs::read(&store).unwrap();
        for reason in ["offline", "provider denied the query: 401"] {
            let (calls, source) = stub(Err(reason.into()));
            let (_, connect) = connector(source);
            assert_eq!(refresh(&store, &root, || NOW + 600, connect), RefreshOutcome::Failed);
            assert_eq!(calls.get(), 1);
            assert_eq!(fs::read(&store).unwrap(), before, "{reason} must not replace the answer");
        }
        let CachedPrs::Stale(listing) = select_cached(&store, Some(ORIGIN), NOW + 600) else {
            panic!("still the stale answer");
        };
        assert_eq!(numbers(&listing), [99]);
    }

    #[test]
    fn a_refresh_discards_its_answer_when_origin_changed_during_the_request() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, ORIGIN, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);
        let before = fs::read(&store).unwrap();
        let (_, mut source) = stub(Ok(vec![pr(7, Some("o/r"), "fix/x", "main")]));
        let moved = root.clone();
        source.during = Some(Box::new(move || git(&moved, &["remote", "set-url", "origin", "https://prs.example.invalid/o/new.git"])));
        let (_, connect) = connector(source);
        assert_eq!(refresh(&store, &root, || NOW + 600, connect), RefreshOutcome::OriginChanged);
        assert_eq!(fs::read(&store).unwrap(), before);
    }

    #[test]
    fn a_refresh_without_an_origin_makes_no_request() {
        let (_dir, root, store) = repo(None);
        let (calls, source) = stub(Ok(Vec::new()));
        let (_, connect) = connector(source);
        assert_eq!(refresh(&store, &root, || NOW, connect), RefreshOutcome::NoOrigin);
        assert_eq!(calls.get(), 0);
        assert!(!store.exists());
    }

    #[test]
    fn staleness_needs_a_known_past_fetch_time() {
        let at = |fetched_at| PrListing { fetched_at, ..PrListing::default() };
        assert!(!at(None).is_stale_at(NOW));
        assert!(!at(Some(NOW + 1)).is_stale_at(NOW));
        assert!(!at(Some(NOW - 59)).is_stale_at(NOW));
        assert!(at(Some(NOW - 60)).is_stale_at(NOW));
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
        assert_eq!(pr_lock_path(&store).file_name().unwrap().to_string_lossy(), format!("{stem}.prs.lock"));
    }
}
