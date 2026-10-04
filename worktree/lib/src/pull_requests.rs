//! Open pull requests for the `wt list` badges, as a stored answer bound to
//! `origin`.
//!
//! The results are persisted per repository beside the comparison cache as
//! `<repo hash>.prs.json` (see [`crate::cache::repo_cache_file`]) with their
//! fetch time and a digest of the exact `origin` URL they were fetched for.
//! [`select_cached`] serves a stored answer only while `origin` still matches;
//! one older than [`FRESHNESS_WINDOW`] is marked stale for the age line.
//!
//! [`refresh`], the background worker's PR half, is the only writer. Whenever
//! it wins the refresh lock ([`pr_lock_path`]) it makes the request, however
//! young the stored answer, and it holds the lock from before the request
//! through publication, so overlapping refreshes make one request at a time
//! and an older answer can never replace a newer one. A failure is never
//! stored, so an authentication error cannot become "no open PRs".
//!
//! Every successful write stamps a new random publication id beside
//! `fetched_at`. `fetched_at` is whole seconds at the request's start, so two
//! answers can share it; [`stored_publication`] is what tells a waiting run
//! that another process published. The same atomic write records which
//! credentials the request was sent with ([`CredentialEvidence`]), so a run
//! that sees the publication learns them without waiting for any receipt.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::cache::{atomic_write, repo_cache_file, try_lock_sidecar};
use crate::error::WorktreeError;
use crate::git::git_from;
use crate::remote_head::{CredentialEvidence, PrFailure, is_attempt_id, present};
use crate::strict_json;
use sniff::remote::blocking::PrUnavailable;

/// Format 3 added `publication`, format 4 `writer`, format 5 dropped
/// `writer` again, and format 6 added `credentials`. Format 5 is still read,
/// with [`CredentialEvidence::Unknown`]; any other format is a
/// [`CachedPrs::Miss`].
pub const PR_STORE_FORMAT_VERSION: u32 = 6;

/// The format before `credentials`.
const UNCREDENTIALED_FORMAT_VERSION: u32 = 5;

/// The age from which a stored answer is shown with its age.
pub const FRESHNESS_WINDOW: Duration = Duration::from_secs(60);

/// How long a [`refresh`] waits for the provider.
pub const REFRESH_DEADLINE: Duration = Duration::from_secs(10);

/// One open PR, as stored and matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenPullRequest {
    pub number: u64,
    #[serde(deserialize_with = "present")]
    pub url: Option<String>,
    /// `owner/repo` of the repository the head branch lives in.
    #[serde(deserialize_with = "present")]
    pub source_repo: Option<String>,
    pub source_branch: String,
    pub target_branch: String,
}

/// Why an [`OpenPrSource`] has no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrRequestError {
    /// `origin` has no provider that answers open-PR requests (a local path
    /// or an unsupported host). Not a failure: there is nothing to ask.
    Unsupported,
    Failed(PrFailure),
}

impl From<PrFailure> for PrRequestError {
    fn from(failure: PrFailure) -> Self {
        Self::Failed(failure)
    }
}

impl PrRequestError {
    /// Sniff's reason, with [`PrUnavailable::Unsupported`] kept apart so it
    /// never becomes a [`PrFailure`].
    pub fn from_unavailable(reason: &PrUnavailable) -> Self {
        match reason {
            PrUnavailable::Unsupported { .. } => Self::Unsupported,
            reason => Self::Failed(PrFailure::from_unavailable(reason)),
        }
    }
}

/// A repository-wide open-PR request.
///
/// A trait so tests can script answers and count requests.
pub trait OpenPrSource {
    /// `owner/repo` of origin: the source repository of a same-repository PR.
    fn source_repo(&self) -> Option<String>;
    /// Every open PR, or why there is no answer. Never an empty list for a
    /// failure.
    fn fetch(&self) -> Result<FetchedPrs, PrRequestError>;
}

/// A successful open-PR request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedPrs {
    pub pull_requests: Vec<OpenPullRequest>,
    /// What the request's pages were sent with, as the sending client
    /// selected it.
    pub credentials: CredentialEvidence,
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

    fn fetch(&self) -> Result<FetchedPrs, PrRequestError> {
        let answer = sniff::remote::blocking::open_pull_requests(&self.remote_url, self.deadline)
            .map_err(|reason| PrRequestError::from_unavailable(&reason))?;
        let pull_requests = answer
            .pull_requests
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
            .collect();
        Ok(FetchedPrs { pull_requests, credentials: CredentialEvidence::from_sniff(&answer.credentials) })
    }
}

impl PrFailure {
    /// The confirmed credentials conditions sniff establishes, by kind; every
    /// other reason (a timeout, a network failure) is [`PrFailure::Other`],
    /// since it names nothing a key would fix. Reached only through
    /// [`PrRequestError::from_unavailable`], which takes
    /// [`PrUnavailable::Unsupported`] out first.
    fn from_unavailable(reason: &PrUnavailable) -> Self {
        match reason {
            PrUnavailable::CredentialsRequired { .. } => Self::CredentialsRequired,
            PrUnavailable::CredentialsRejected { key } => Self::CredentialsRejected { key: Some(key.clone()) },
            PrUnavailable::CredentialsInsufficient { key } => Self::CredentialsInsufficient { key: Some(key.clone()) },
            PrUnavailable::RateLimited { authenticated, key } => {
                Self::RateLimited { authenticated: *authenticated, key: key.clone() }
            }
            PrUnavailable::NotFoundOrNotPermitted { .. } => Self::NotFoundOrNotPermitted,
            _ => Self::Other,
        }
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

/// Every field must be present: a missing or wrongly typed one, a duplicate
/// key, or trailing content makes the whole file a miss, never an empty or
/// partial answer. Unknown fields are ignored, as in every earlier format.
#[derive(Serialize, Deserialize)]
struct StoreFile {
    format_version: u32,
    /// [`origin_digest`] of the URL the request was made for.
    origin_digest: String,
    /// Seconds since the Unix epoch at which the request started.
    fetched_at: u64,
    /// A random id ([`crate::remote_head::new_attempt_id`]) new with every
    /// write; see [`stored_publication`].
    publication: String,
    #[serde(deserialize_with = "present")]
    source_repo: Option<String>,
    pull_requests: Vec<OpenPullRequest>,
    /// What the request that produced this answer was sent with. Malformed,
    /// absent, or `null` makes the whole publication a miss.
    #[serde(deserialize_with = "strict_json::nested")]
    credentials: CredentialEvidence,
}

/// Format 5, read with the same strictness; its credentials are unknown.
#[derive(Deserialize)]
struct UncredentialedStoreFile {
    format_version: u32,
    origin_digest: String,
    fetched_at: u64,
    publication: String,
    #[serde(deserialize_with = "present")]
    source_repo: Option<String>,
    pull_requests: Vec<OpenPullRequest>,
}

impl From<UncredentialedStoreFile> for StoreFile {
    fn from(file: UncredentialedStoreFile) -> Self {
        Self {
            format_version: file.format_version,
            origin_digest: file.origin_digest,
            fetched_at: file.fetched_at,
            publication: file.publication,
            source_repo: file.source_repo,
            pull_requests: file.pull_requests,
            credentials: CredentialEvidence::Unknown,
        }
    }
}

/// Only `format_version`, to pick the reader.
#[derive(Deserialize)]
struct FormatOnly {
    format_version: u32,
}

/// One usable publication: its id, and what its request was sent with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPublication {
    /// New with every successful write.
    pub id: String,
    /// [`CredentialEvidence::Unknown`] for a format-5 answer.
    pub credentials: CredentialEvidence,
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
    /// Younger than [`FRESHNESS_WINDOW`].
    Fresh(PrListing),
    /// [`FRESHNESS_WINDOW`] old or more; still shown, with its age.
    Stale(PrListing),
    /// No usable answer: none stored, unreadable, another format, fetched in
    /// the future, bound to another `origin`, or no `origin` at all.
    Miss,
}

/// What the stored answer at `store` offers when `origin` is the current
/// [`origin_url`]. A valid empty answer is an answer, not a miss.
pub fn select_cached(store: &Path, origin: Option<&str>, now: u64) -> CachedPrs {
    let Some(file) = usable(store, origin, now) else {
        return CachedPrs::Miss;
    };
    let listing = listing(&file);
    if listing.is_stale_at(now) {
        CachedPrs::Stale(listing)
    } else {
        CachedPrs::Fresh(listing)
    }
}

/// The publication of the answer [`select_cached`] would serve, or `None`
/// on a miss.
///
/// The id is new with every successful write, so a changed id proves a
/// publication even when `fetched_at` did not move. Its credentials were
/// written in the same atomic write as the answer and the id.
pub fn stored_publication(store: &Path, origin: &str, now: u64) -> Option<StoredPublication> {
    usable(store, Some(origin), now).map(|file| StoredPublication { id: file.publication, credentials: file.credentials })
}

/// Why a [`refresh`] ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshOutcome {
    /// A new answer was stored.
    Refreshed,
    /// Another refresh holds the lock; no request was made.
    Contended,
    /// The lock file could not be opened or locked; no request was made.
    LockFailed,
    /// `origin` is missing; no request was made.
    NoOrigin,
    /// `origin` has no provider to ask ([`PrRequestError::Unsupported`]);
    /// the store is untouched.
    Unsupported,
    /// `origin` changed during the request, so the answer was discarded.
    OriginChanged,
    /// The request failed; the store is untouched.
    Failed(PrFailure),
    /// The answer could not be written, or no publication id could be made;
    /// the store is untouched.
    PublishFailed,
}

/// Asks for the open PRs of the repository at `repo_root` and stores the
/// answer, unless another process is refreshing it.
///
/// Holds a nonblocking exclusive lock on the persistent sidecar
/// [`pr_lock_path`] from before the request through publication, so
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
    let Some(_lock) = (match try_lock_sidecar(&pr_lock_path(store)) {
        Ok(lock) => lock,
        Err(_) => return RefreshOutcome::LockFailed,
    }) else {
        return RefreshOutcome::Contended;
    };

    let Some(origin) = origin_url(repo_root) else {
        return RefreshOutcome::NoOrigin;
    };
    let started = clock();
    let answer = match fetch(connect(&origin).as_ref()) {
        Ok(answer) => answer,
        Err(PrRequestError::Unsupported) => return RefreshOutcome::Unsupported,
        Err(PrRequestError::Failed(failure)) => return RefreshOutcome::Failed(failure),
    };
    if origin_url(repo_root).as_deref() != Some(origin.as_str()) {
        return RefreshOutcome::OriginChanged;
    }
    match publish(store, &origin, started, &answer) {
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

/// Whether another process holds the refresh lock of the PR store at
/// `store`; a lock file that cannot be opened reads as not held.
///
/// Like [`crate::remote_head::refresh_lock_held`], the probe takes the lock
/// for an instant: probe only a worker that is already running.
pub fn pr_lock_held(store: &Path) -> bool {
    matches!(try_lock_sidecar(&pr_lock_path(store)), Ok(None))
}

/// Seconds since the Unix epoch.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

/// A successful request's answer, before it is stored.
struct Answer {
    source_repo: Option<String>,
    fetched: FetchedPrs,
}

fn fetch(source: &dyn OpenPrSource) -> Result<Answer, PrRequestError> {
    Ok(Answer { fetched: source.fetch()?, source_repo: source.source_repo() })
}

/// Stores `answer` for `origin`, stamped `fetched_at`, under a new
/// publication id. Call with the lock held.
fn publish(path: &Path, origin: &str, fetched_at: u64, answer: &Answer) -> Result<(), WorktreeError> {
    let file = StoreFile {
        format_version: PR_STORE_FORMAT_VERSION,
        origin_digest: origin_digest(origin),
        fetched_at,
        publication: crate::remote_head::new_attempt_id()?,
        source_repo: answer.source_repo.clone(),
        pull_requests: answer.fetched.pull_requests.clone(),
        credentials: answer.fetched.credentials.clone(),
    };
    if !file.credentials.is_valid() {
        return Err(WorktreeError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "refusing to store invalid request credentials",
        )));
    }
    save(path, &file)
}

/// The stored answer for `origin`, unless it is unreadable, another format,
/// without a valid publication id, fetched after `now`, or bound to another
/// `origin`.
fn usable(store: &Path, origin: Option<&str>, now: u64) -> Option<StoreFile> {
    let origin = origin?;
    load(store).filter(|file| file.fetched_at <= now && file.origin_digest == origin_digest(origin))
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
    let file = match serde_json::from_slice::<FormatOnly>(&bytes).ok()?.format_version {
        PR_STORE_FORMAT_VERSION => serde_json::from_slice::<StoreFile>(&bytes).ok()?,
        UNCREDENTIALED_FORMAT_VERSION => serde_json::from_slice::<UncredentialedStoreFile>(&bytes).ok()?.into(),
        _ => return None,
    };
    (is_attempt_id(&file.publication) && file.credentials.is_valid()).then_some(file)
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
        answer: Result<Vec<OpenPullRequest>, PrRequestError>,
        /// What a successful answer reports it was sent with.
        credentials: CredentialEvidence,
        calls: Rc<Cell<usize>>,
        during: Option<Box<dyn Fn()>>,
    }

    impl OpenPrSource for Stub {
        fn source_repo(&self) -> Option<String> {
            Some("o/r".to_string())
        }
        fn fetch(&self) -> Result<FetchedPrs, PrRequestError> {
            self.calls.set(self.calls.get() + 1);
            if let Some(during) = &self.during {
                during();
            }
            let pull_requests = self.answer.clone()?;
            Ok(FetchedPrs { pull_requests, credentials: self.credentials.clone() })
        }
    }

    fn stub(answer: Result<Vec<OpenPullRequest>, PrRequestError>) -> (Rc<Cell<usize>>, Stub) {
        let calls = Rc::new(Cell::new(0));
        (Rc::clone(&calls), Stub { answer, credentials: CredentialEvidence::Anonymous, calls, during: None })
    }

    fn failing(failure: PrFailure) -> (Rc<Cell<usize>>, Stub) {
        stub(Err(failure.into()))
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

    /// Runs [`refresh`] at `at` against `source`.
    fn refresh_with(store: &Path, root: &Path, at: u64, source: Stub) -> RefreshOutcome {
        let (_, connect) = connector(source);
        refresh(store, root, || at, connect)
    }

    /// Stores `prs` for the repository's origin, fetched at `at`, through the
    /// real writer.
    fn seed(store: &Path, root: &Path, at: u64, prs: Vec<OpenPullRequest>) {
        let (_, source) = stub(Ok(prs));
        assert_eq!(refresh_with(store, root, at, source), RefreshOutcome::Refreshed, "seeding");
    }

    fn numbers(listing: &PrListing) -> Vec<u64> {
        listing.pull_requests.iter().map(|pr| pr.number).collect()
    }

    #[test]
    fn a_stored_answer_is_fresh_below_60_seconds_and_stale_from_60() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);

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
        seed(&store, &root, NOW, Vec::new());
        let CachedPrs::Fresh(listing) = select_cached(&store, Some(ORIGIN), NOW + 59) else {
            panic!("fresh at selection");
        };
        assert!(listing.is_stale_at(NOW + 61), "age is judged at render time, not selection time");
    }

    #[test]
    fn an_answer_for_another_or_no_origin_is_a_miss_even_when_fresh() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);

        assert_eq!(select_cached(&store, Some("https://prs.example.invalid/o/other.git"), NOW + 1), CachedPrs::Miss);
        assert_eq!(select_cached(&store, Some("git@prs.example.invalid:o/r.git"), NOW + 1), CachedPrs::Miss);
        assert_eq!(select_cached(&store, None, NOW + 1), CachedPrs::Miss);
        assert!(matches!(select_cached(&store, Some(ORIGIN), NOW + 1), CachedPrs::Fresh(_)));
    }

    #[test]
    fn a_stored_empty_answer_is_an_answer() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, Vec::new());
        let expected = PrListing { source_repo: Some("o/r".into()), pull_requests: Vec::new(), fetched_at: Some(NOW) };
        assert_eq!(select_cached(&store, Some(ORIGIN), NOW + 1), CachedPrs::Fresh(expected.clone()));
        assert_eq!(select_cached(&store, Some(ORIGIN), NOW + 600), CachedPrs::Stale(expected));
        assert!(stored_publication(&store, ORIGIN, NOW + 1).is_some(), "an empty answer has a publication too");
    }

    #[test]
    fn a_format_5_answer_is_served_with_unknown_credentials_and_a_future_format_is_a_miss() {
        let (_dir, _root, store) = repo(Some(ORIGIN));
        fs::create_dir_all(store.parent().unwrap()).unwrap();
        let id = "0123456789abcdef0123456789abcdef";
        let element = serde_json::json!({
            "number": 99, "url": null, "source_repo": "o/r", "source_branch": "fix/x", "target_branch": "main",
        });
        let format_5 = serde_json::json!({
            "format_version": 5, "origin_digest": origin_digest(ORIGIN), "fetched_at": NOW, "publication": id,
            "source_repo": "o/r", "pull_requests": [element],
        });
        let expected = PrListing {
            source_repo: Some("o/r".into()),
            pull_requests: vec![OpenPullRequest { url: None, ..pr(99, Some("o/r"), "fix/x", "main") }],
            fetched_at: Some(NOW),
        };

        fs::write(&store, serde_json::to_vec(&format_5).unwrap()).unwrap();
        assert_eq!(select_cached(&store, Some(ORIGIN), NOW), CachedPrs::Fresh(expected));
        assert_eq!(
            stored_publication(&store, ORIGIN, NOW),
            Some(StoredPublication { id: id.into(), credentials: CredentialEvidence::Unknown }),
            "a format-5 answer can never say it was anonymous"
        );
        // A format-5 reader ignores unknown fields, so a stray format-6 field
        // is ignored too; it never makes the answer anonymous.
        let mut format_5_claiming = format_5.clone();
        format_5_claiming["credentials"] = serde_json::json!({ "state": "anonymous" });
        fs::write(&store, serde_json::to_vec(&format_5_claiming).unwrap()).unwrap();
        assert_eq!(
            stored_publication(&store, ORIGIN, NOW).map(|publication| publication.credentials),
            Some(CredentialEvidence::Unknown),
            "format 5 with a credentials field"
        );

        let mut format_5_ill = format_5.clone();
        format_5_ill["publication"] = serde_json::json!("not-an-id");
        let mut format_7 = format_5.clone();
        format_7["format_version"] = serde_json::json!(7);
        format_7["credentials"] = serde_json::json!({ "state": "anonymous" });
        for (label, document) in [("an invalid format-5 field", format_5_ill), ("format 7", format_7)] {
            fs::write(&store, serde_json::to_vec(&document).unwrap()).unwrap();
            assert_eq!(select_cached(&store, Some(ORIGIN), NOW), CachedPrs::Miss, "{label}");
            assert_eq!(stored_publication(&store, ORIGIN, NOW), None, "{label}");
        }
    }

    #[test]
    fn a_publication_records_what_its_request_was_sent_with_and_only_names() {
        const SECRET: &str = "ghp_sentinelValueThatMustNeverBeStored";
        let (_dir, root, store) = repo(Some(ORIGIN));
        for credentials in [
            CredentialEvidence::Anonymous,
            CredentialEvidence::Keyed { variables: vec!["SNIFF_GITHUB_GIT_2E_EXAMPLE_TOKEN".into()] },
            CredentialEvidence::Unknown,
        ] {
            let (_, mut source) = stub(Ok(Vec::new()));
            source.credentials = credentials.clone();
            assert_eq!(refresh_with(&store, &root, NOW, source), RefreshOutcome::Refreshed);
            assert_eq!(
                stored_publication(&store, ORIGIN, NOW).map(|publication| publication.credentials),
                Some(credentials.clone()),
                "an empty answer carries its credentials too"
            );
        }

        // Anything but a variable name is refused, and the stored answer kept.
        let before = fs::read(&store).unwrap();
        for variables in [vec![SECRET.to_string()], vec!["GH_TOKEN".into(), "has space".into()], Vec::new()] {
            let (_, mut source) = stub(Ok(Vec::new()));
            source.credentials = CredentialEvidence::Keyed { variables };
            assert_eq!(refresh_with(&store, &root, NOW, source), RefreshOutcome::PublishFailed);
        }
        let after = fs::read(&store).unwrap();
        assert_eq!(after, before);
        assert!(!String::from_utf8_lossy(&after).contains(SECRET));
    }

    #[test]
    fn earlier_formats_exactly_as_they_were_written_are_misses() {
        let (_dir, _root, store) = repo(Some(ORIGIN));
        fs::create_dir_all(store.parent().unwrap()).unwrap();
        let element = serde_json::json!({
            "number": 99,
            "url": "https://github.com/o/r/pull/99",
            "source_repo": "o/r",
            "source_branch": "fix/x",
            "target_branch": "main",
        });
        let id = "0123456789abcdef0123456789abcdef";
        let digest = origin_digest(ORIGIN);
        for (format, document) in [
            // No origin binding.
            (1, serde_json::json!({ "format_version": 1, "fetched_at": NOW, "source_repo": "o/r", "pull_requests": [element] })),
            // No publication id.
            (2, serde_json::json!({ "format_version": 2, "origin_digest": digest, "fetched_at": NOW, "source_repo": "o/r", "pull_requests": [element] })),
            // No writer.
            (3, serde_json::json!({ "format_version": 3, "origin_digest": digest, "fetched_at": NOW, "publication": id, "source_repo": "o/r", "pull_requests": [element] })),
            // With the writer format 5 dropped: a miss even though every
            // format-5 field is present and valid.
            (4, serde_json::json!({ "format_version": 4, "origin_digest": digest, "fetched_at": NOW, "publication": id, "writer": "refresh", "source_repo": "o/r", "pull_requests": [element] })),
        ] {
            fs::write(&store, serde_json::to_vec(&document).unwrap()).unwrap();
            assert_eq!(select_cached(&store, Some(ORIGIN), NOW), CachedPrs::Miss, "format {format}");
            assert_eq!(stored_publication(&store, ORIGIN, NOW), None, "format {format}");
        }
        assert_eq!(select_cached(&store.with_file_name("absent.prs.json"), Some(ORIGIN), NOW), CachedPrs::Miss);
    }

    /// One edit to a store the real writer wrote.
    enum Edit {
        /// Replace (or, with `None`, remove) the value at a JSON pointer.
        Set(&'static str, Option<serde_json::Value>),
        /// Rewrite the compact text: `(find, replace)`, first match only.
        Text(&'static str, &'static str),
        Append(&'static str),
    }

    /// What reading an edited store gives through [`select_cached`] and
    /// [`stored_publication`].
    enum Expect {
        Miss,
        /// A fresh answer under the written publication id: the written
        /// listing with this change.
        Answer(fn(&mut PrListing)),
        /// The same, but at least [`FRESHNESS_WINDOW`] old.
        Stale(fn(&mut PrListing)),
        /// The written answer, fresh, with these credentials instead.
        Evidence(fn() -> CredentialEvidence),
    }

    /// The Input Robustness Matrix for the store: every load-bearing field of
    /// the envelope and of a PR element in every shape, one edit per cell,
    /// from a file written by [`refresh`], read through the public results.
    #[test]
    fn the_store_reader_walks_the_input_robustness_matrix() {
        use serde_json::{Value, json};
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);
        let written: Value = serde_json::from_slice(&fs::read(&store).unwrap()).unwrap();
        let id = written["publication"].as_str().unwrap().to_string();
        let element = written["pull_requests"][0].clone();
        let publication = |credentials| Some(StoredPublication { id: id.clone(), credentials });
        let expected = PrListing {
            source_repo: Some("o/r".into()),
            pull_requests: vec![pr(99, Some("o/r"), "fix/x", "main")],
            fetched_at: Some(NOW),
        };

        let apply = |edit: &Edit| -> Vec<u8> {
            let mut document = written.clone();
            match edit {
                Edit::Set(pointer, value) => {
                    let (parent, key) = pointer.rsplit_once('/').unwrap();
                    let target = if parent.is_empty() { &mut document } else { document.pointer_mut(parent).unwrap() };
                    match (target, value) {
                        (Value::Object(map), Some(value)) => {
                            map.insert(key.to_string(), value.clone());
                        }
                        (Value::Object(map), None) => {
                            assert!(map.remove(key).is_some(), "{pointer} exists");
                        }
                        (Value::Array(items), Some(value)) => items[key.parse::<usize>().unwrap()] = value.clone(),
                        (Value::Array(items), None) => {
                            items.remove(key.parse::<usize>().unwrap());
                        }
                        _ => panic!("{pointer} is not in a container"),
                    }
                    serde_json::to_vec(&document).unwrap()
                }
                Edit::Text(find, replace) => {
                    let text = serde_json::to_string(&document).unwrap();
                    assert!(text.contains(find), "{find} in {text}");
                    text.replacen(find, replace, 1).into_bytes()
                }
                Edit::Append(tail) => {
                    let mut bytes = serde_json::to_vec(&document).unwrap();
                    bytes.extend_from_slice(tail.as_bytes());
                    bytes
                }
            }
        };
        let read = |bytes: &[u8]| {
            fs::write(&store, bytes).unwrap();
            (select_cached(&store, Some(ORIGIN), NOW), stored_publication(&store, ORIGIN, NOW))
        };

        // Control: the unedited file, re-serialized the way every edit is.
        assert_eq!(
            read(&serde_json::to_vec(&written).unwrap()),
            (CachedPrs::Fresh(expected.clone()), publication(CredentialEvidence::Anonymous))
        );
        // The same file is a miss for another origin, or none.
        assert_eq!(select_cached(&store, Some("https://prs.example.invalid/o/other.git"), NOW), CachedPrs::Miss);
        assert_eq!(stored_publication(&store, "https://prs.example.invalid/o/other.git", NOW), None);
        assert_eq!(select_cached(&store, None, NOW), CachedPrs::Miss);

        use Edit::*;
        use Expect::*;
        let null = || Some(Value::Null);
        // Duplicates are inserted at the start of their object, so the text
        // they find is unambiguous whatever order the keys were written in;
        // each repeats the written value, so only the repetition can reject it.
        let cells: Vec<(Edit, Expect)> = vec![
            // format_version
            (Set("/format_version", None), Miss),
            (Set("/format_version", null()), Miss),
            (Set("/format_version", Some(json!("5"))), Miss),
            (Set("/format_version", Some(json!(true))), Miss),
            (Set("/format_version", Some(json!([]))), Miss),
            (Set("/format_version", Some(json!({}))), Miss),
            (Set("/format_version", Some(json!(""))), Miss),
            (Set("/format_version", Some(json!(4))), Miss),
            (Set("/format_version", Some(json!(7))), Miss),
            (Text("{", "{\"format_version\":6,"), Miss),
            // credentials: anything but a valid tagged state is a miss, never
            // read as anonymous; only `anonymous` itself can become a notice
            (Set("/credentials", None), Miss),
            (Set("/credentials", null()), Miss),
            (Set("/credentials", Some(json!("anonymous"))), Miss),
            (Set("/credentials", Some(json!(1))), Miss),
            (Set("/credentials", Some(json!([]))), Miss),
            (Set("/credentials", Some(json!({}))), Miss),
            (Set("/credentials", Some(json!({ "state": null }))), Miss),
            (Set("/credentials", Some(json!({ "state": 0 }))), Miss),
            (Set("/credentials", Some(json!({ "state": "keyless" }))), Miss),
            (Set("/credentials", Some(json!({ "state": "keyed" }))), Miss),
            (Set("/credentials", Some(json!({ "state": "keyed", "variables": null }))), Miss),
            (Set("/credentials", Some(json!({ "state": "keyed", "variables": "GH_TOKEN" }))), Miss),
            (Set("/credentials", Some(json!({ "state": "keyed", "variables": {} }))), Miss),
            (Set("/credentials", Some(json!({ "state": "keyed", "variables": [] }))), Miss),
            (Set("/credentials", Some(json!({ "state": "keyed", "variables": ["GH_TOKEN", 1] }))), Miss),
            (Set("/credentials", Some(json!({ "state": "keyed", "variables": [1] }))), Miss),
            (Set("/credentials", Some(json!({ "state": "keyed", "variables": [1, null] }))), Miss),
            (Set("/credentials", Some(json!({ "state": "keyed", "variables": ["GH TOKEN"] }))), Miss),
            (Set("/credentials", Some(json!({ "state": "keyed", "variables": [""] }))), Miss),
            (Set("/credentials", Some(json!({ "state": "keyed", "variables": ["ghp_looksLikeAToken123"] }))), Miss),
            (Text("\"state\":\"anonymous\"", "\"state\":\"anonymous\",\"state\":\"anonymous\""), Miss),
            (Text("{", "{\"credentials\":{\"state\":\"anonymous\"},"), Miss),
            (
                Set("/credentials", Some(json!({ "state": "keyed", "variables": ["GH_TOKEN"] }))),
                Evidence(|| CredentialEvidence::Keyed { variables: vec!["GH_TOKEN".into()] }),
            ),
            (Set("/credentials", Some(json!({ "state": "unknown" }))), Evidence(|| CredentialEvidence::Unknown)),
            (Set("/credentials/extra", Some(json!(true))), Answer(|_| {})),
            // publication
            (Set("/publication", None), Miss),
            (Set("/publication", null()), Miss),
            (Set("/publication", Some(json!(123))), Miss),
            (Set("/publication", Some(json!([]))), Miss),
            (Set("/publication", Some(json!({}))), Miss),
            (Set("/publication", Some(json!(""))), Miss),
            (Set("/publication", Some(json!("not-an-id"))), Miss),
            (Set("/publication", Some(json!("0123456789ABCDEF0123456789ABCDEF"))), Miss),
            (Text("{", "{\"publication\":\"0123456789abcdef0123456789abcdef\","), Miss),
            // origin_digest
            (Set("/origin_digest", None), Miss),
            (Set("/origin_digest", null()), Miss),
            (Set("/origin_digest", Some(json!(123))), Miss),
            (Set("/origin_digest", Some(json!([]))), Miss),
            (Set("/origin_digest", Some(json!({}))), Miss),
            (Set("/origin_digest", Some(json!(""))), Miss),
            (Set("/origin_digest", Some(json!(origin_digest("https://prs.example.invalid/o/other.git")))), Miss),
            (Text("{", "{\"origin_digest\":\"x\","), Miss),
            // fetched_at
            (Set("/fetched_at", None), Miss),
            (Set("/fetched_at", null()), Miss),
            (Set("/fetched_at", Some(json!("1790000000"))), Miss),
            (Set("/fetched_at", Some(json!([]))), Miss),
            (Set("/fetched_at", Some(json!({}))), Miss),
            (Set("/fetched_at", Some(json!(""))), Miss),
            (Set("/fetched_at", Some(json!(-1))), Miss),
            (Set("/fetched_at", Some(json!(NOW + 1))), Miss),
            (Set("/fetched_at", Some(json!(NOW - 600))), Stale(|listing| listing.fetched_at = Some(NOW - 600))),
            (Text("{", "{\"fetched_at\":1790000000,"), Miss),
            // source_repo: null is a valid "unknown", "" a repository no PR
            // can match
            (Set("/source_repo", None), Miss),
            (Set("/source_repo", null()), Answer(|listing| listing.source_repo = None)),
            (Set("/source_repo", Some(json!(123))), Miss),
            (Set("/source_repo", Some(json!([]))), Miss),
            (Set("/source_repo", Some(json!({}))), Miss),
            (Set("/source_repo", Some(json!(""))), Answer(|listing| listing.source_repo = Some(String::new()))),
            (Text("{", "{\"source_repo\":\"o/r\","), Miss),
            // pull_requests, the answer list
            (Set("/pull_requests", None), Miss),
            (Set("/pull_requests", null()), Miss),
            (Set("/pull_requests", Some(json!(123))), Miss),
            (Set("/pull_requests", Some(json!({}))), Miss),
            (Set("/pull_requests", Some(json!(""))), Miss),
            (Set("/pull_requests", Some(json!([]))), Answer(|listing| listing.pull_requests.clear())),
            (Text("{", "{\"pull_requests\":[],"), Miss),
            // ... an invalid element among valid ones is never filtered out,
            // and invalid elements never become an empty answer
            (Set("/pull_requests/0", Some(json!(123))), Miss),
            (Set("/pull_requests/0", null()), Miss),
            (Set("/pull_requests/0", Some(json!([]))), Miss),
            (Set("/pull_requests/0", Some(json!({}))), Miss),
            (Set("/pull_requests", Some(json!([element.clone(), 123]))), Miss),
            (Set("/pull_requests", Some(json!([123, element.clone()]))), Miss),
            (Set("/pull_requests", Some(json!([element.clone(), { "number": 98 }]))), Miss),
            (Set("/pull_requests", Some(json!([123, null]))), Miss),
            (
                Set("/pull_requests", Some(json!([element.clone(), element.clone()]))),
                Answer(|listing| listing.pull_requests.push(listing.pull_requests[0].clone())),
            ),
            // number
            (Set("/pull_requests/0/number", None), Miss),
            (Set("/pull_requests/0/number", null()), Miss),
            (Set("/pull_requests/0/number", Some(json!("99"))), Miss),
            (Set("/pull_requests/0/number", Some(json!([]))), Miss),
            (Set("/pull_requests/0/number", Some(json!({}))), Miss),
            (Set("/pull_requests/0/number", Some(json!(""))), Miss),
            (Set("/pull_requests/0/number", Some(json!(-1))), Miss),
            (Set("/pull_requests/0/number", Some(json!(1.5))), Miss),
            (Text("[{", "[{\"number\":99,"), Miss),
            // url: null is a valid "no link", "" a link with no text
            (Set("/pull_requests/0/url", None), Miss),
            (Set("/pull_requests/0/url", null()), Answer(|listing| listing.pull_requests[0].url = None)),
            (Set("/pull_requests/0/url", Some(json!(1))), Miss),
            (Set("/pull_requests/0/url", Some(json!([]))), Miss),
            (Set("/pull_requests/0/url", Some(json!({}))), Miss),
            (Set("/pull_requests/0/url", Some(json!(""))), Answer(|listing| listing.pull_requests[0].url = Some(String::new()))),
            (Text("[{", "[{\"url\":\"https://github.com/o/r/pull/99\","), Miss),
            // source_repo of a PR: null is a valid "unknown"
            (Set("/pull_requests/0/source_repo", None), Miss),
            (Set("/pull_requests/0/source_repo", null()), Answer(|listing| listing.pull_requests[0].source_repo = None)),
            (Set("/pull_requests/0/source_repo", Some(json!(1))), Miss),
            (Set("/pull_requests/0/source_repo", Some(json!([]))), Miss),
            (Set("/pull_requests/0/source_repo", Some(json!({}))), Miss),
            (
                Set("/pull_requests/0/source_repo", Some(json!(""))),
                Answer(|listing| listing.pull_requests[0].source_repo = Some(String::new())),
            ),
            (Text("[{", "[{\"source_repo\":\"o/r\","), Miss),
            // source_branch: "" is a string, so an answer no branch matches
            (Set("/pull_requests/0/source_branch", None), Miss),
            (Set("/pull_requests/0/source_branch", null()), Miss),
            (Set("/pull_requests/0/source_branch", Some(json!(1))), Miss),
            (Set("/pull_requests/0/source_branch", Some(json!([]))), Miss),
            (Set("/pull_requests/0/source_branch", Some(json!({}))), Miss),
            (
                Set("/pull_requests/0/source_branch", Some(json!(""))),
                Answer(|listing| listing.pull_requests[0].source_branch = String::new()),
            ),
            (Text("[{", "[{\"source_branch\":\"fix/x\","), Miss),
            // target_branch: likewise
            (Set("/pull_requests/0/target_branch", None), Miss),
            (Set("/pull_requests/0/target_branch", null()), Miss),
            (Set("/pull_requests/0/target_branch", Some(json!(1))), Miss),
            (Set("/pull_requests/0/target_branch", Some(json!(["main"]))), Miss),
            (Set("/pull_requests/0/target_branch", Some(json!({}))), Miss),
            (
                Set("/pull_requests/0/target_branch", Some(json!(""))),
                Answer(|listing| listing.pull_requests[0].target_branch = String::new()),
            ),
            (Text("[{", "[{\"target_branch\":\"main\","), Miss),
            // Unknown fields are ignored, as in every earlier format, including
            // a leftover format-4 `writer` (a format-4 file itself is a miss).
            (Set("/writer", Some(json!("listing"))), Answer(|_| {})),
            (Set("/pull_requests/0/draft", Some(json!(true))), Answer(|_| {})),
            // trailing or invalid content
            (Append("garbage"), Miss),
            (Append("{}"), Miss),
            (Append(","), Miss),
            (Text("{", "{{"), Miss),
        ];

        // Every cell runs, so a failure names each one it affects.
        let mut wrong = Vec::new();
        for (edit, expect) in &cells {
            let bytes = apply(edit);
            let want = match expect {
                Miss => (CachedPrs::Miss, None),
                Answer(change) | Stale(change) => {
                    let mut listing = expected.clone();
                    change(&mut listing);
                    let cached = if matches!(expect, Stale(_)) { CachedPrs::Stale(listing) } else { CachedPrs::Fresh(listing) };
                    (cached, publication(CredentialEvidence::Anonymous))
                }
                Evidence(credentials) => (CachedPrs::Fresh(expected.clone()), publication(credentials())),
            };
            let got = read(&bytes);
            if got != want {
                wrong.push(format!("{}: {got:?}, expected {want:?}", String::from_utf8_lossy(&bytes)));
            }
        }
        assert!(wrong.is_empty(), "{} of {} cells read wrong:\n{}", wrong.len(), cells.len(), wrong.join("\n"));
    }

    #[test]
    fn every_publication_stores_a_new_id_even_within_one_second() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        assert_eq!(stored_publication(&store, ORIGIN, NOW), None, "nothing stored");
        seed(&store, &root, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);
        let seeded = stored_publication(&store, ORIGIN, NOW).expect("the seeded answer has an id").id;
        assert!(is_attempt_id(&seeded), "{seeded}");

        let (_, source) = stub(Ok(vec![pr(8, Some("o/r"), "feat/z", "main")]));
        assert_eq!(refresh_with(&store, &root, NOW, source), RefreshOutcome::Refreshed);
        let refreshed = stored_publication(&store, ORIGIN, NOW).expect("the new answer has an id").id;
        assert_ne!(refreshed, seeded, "same second, new answer, new id");
        let CachedPrs::Fresh(listing) = select_cached(&store, Some(ORIGIN), NOW) else {
            panic!("the refreshed answer is served");
        };
        assert_eq!((listing.fetched_at, numbers(&listing)), (Some(NOW), vec![8]), "fetched_at did not move");

        let (_, source) = failing(PrFailure::Other);
        assert_eq!(refresh_with(&store, &root, NOW, source), RefreshOutcome::Failed(PrFailure::Other));
        assert_eq!(stored_publication(&store, ORIGIN, NOW).map(|p| p.id), Some(refreshed), "a failure publishes nothing");
    }

    #[test]
    fn a_publication_id_is_read_only_where_the_answer_would_be_served() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, Vec::new());
        assert!(stored_publication(&store, ORIGIN, NOW + 600).is_some(), "a stale answer still has its id");
        assert_eq!(stored_publication(&store, "https://prs.example.invalid/o/other.git", NOW), None);
        assert_eq!(stored_publication(&store, ORIGIN, NOW - 1), None, "fetched in the future");
    }

    #[test]
    fn the_store_records_only_a_digest_of_the_origin() {
        let secret = "https://user:hunter2@prs.example.invalid/o/r.git";
        let (_dir, root, store) = repo(Some(secret));
        assert_eq!(origin_url(&root).as_deref(), Some(secret));
        seed(&store, &root, NOW, Vec::new());
        let bytes = String::from_utf8(fs::read(&store).unwrap()).unwrap();
        assert!(!bytes.contains("hunter2") && !bytes.contains("example.invalid"), "{bytes}");
        assert!(bytes.contains(&origin_digest(secret)));
        assert!(!bytes.contains("writer"), "format 5 has no writer: {bytes}");
    }

    #[test]
    fn origin_url_is_none_without_an_origin() {
        let (_dir, root, _store) = repo(None);
        assert_eq!(origin_url(&root), None);
    }

    #[test]
    fn every_sniff_reason_maps_to_its_condition_with_unsupported_kept_apart() {
        let key = || "GH_TOKEN".to_string();
        let failed = PrRequestError::Failed;
        let cases = [
            (PrUnavailable::CredentialsRequired { key: None }, failed(PrFailure::CredentialsRequired)),
            (
                PrUnavailable::CredentialsRejected { key: key() },
                failed(PrFailure::CredentialsRejected { key: Some(key()) }),
            ),
            (
                PrUnavailable::CredentialsInsufficient { key: key() },
                failed(PrFailure::CredentialsInsufficient { key: Some(key()) }),
            ),
            (
                PrUnavailable::RateLimited { authenticated: false, key: None },
                failed(PrFailure::RateLimited { authenticated: false, key: None }),
            ),
            (
                PrUnavailable::RateLimited { authenticated: true, key: Some(key()) },
                failed(PrFailure::RateLimited { authenticated: true, key: Some(key()) }),
            ),
            (
                PrUnavailable::NotFoundOrNotPermitted { message: "404".into(), key: None },
                failed(PrFailure::NotFoundOrNotPermitted),
            ),
            (PrUnavailable::Timeout { deadline: Duration::from_millis(300) }, failed(PrFailure::Other)),
            (PrUnavailable::Network { message: "refused".into() }, failed(PrFailure::Other)),
            (PrUnavailable::Unsupported { message: "local path".into() }, PrRequestError::Unsupported),
            (PrUnavailable::Other { message: "bad json".into() }, failed(PrFailure::Other)),
        ];
        for (reason, expected) in cases {
            assert_eq!(PrRequestError::from_unavailable(&reason), expected, "{reason:?}");
        }
    }

    /// A local-path origin is what sniff reports as unsupported: the real
    /// source makes no request and the refresh stores nothing.
    #[test]
    fn a_refresh_for_a_local_path_origin_is_unsupported_and_stores_nothing() {
        let (dir, root, store) = repo(None);
        let local = dir.path().join("origin.git");
        git(&root, &["remote", "add", "origin", local.to_str().unwrap()]);
        let outcome = refresh(&store, &root, || NOW, |origin: &str| {
            Box::new(SniffOpenPrSource { remote_url: origin.to_string(), deadline: REFRESH_DEADLINE }) as Box<dyn OpenPrSource>
        });
        assert_eq!(outcome, RefreshOutcome::Unsupported);
        assert!(!store.exists(), "an unsupported origin stores nothing");
        assert_eq!(select_cached(&store, origin_url(&root).as_deref(), NOW), CachedPrs::Miss);
    }

    #[test]
    fn an_unsupported_answer_leaves_a_stored_answer_untouched() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);
        let before = fs::read(&store).unwrap();
        let (calls, source) = stub(Err(PrRequestError::Unsupported));
        assert_eq!(refresh_with(&store, &root, NOW + 600, source), RefreshOutcome::Unsupported);
        assert_eq!(calls.get(), 1);
        assert_eq!(fs::read(&store).unwrap(), before);
    }

    #[test]
    fn a_refresh_replaces_a_stale_answer_stamped_at_its_start() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);

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
    }

    /// Every call that wins the lock asks, however young the stored answer:
    /// a fresh answer never stops the next request.
    #[test]
    fn a_refresh_asks_even_while_a_fresh_answer_is_stored() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);
        let first = stored_publication(&store, ORIGIN, NOW).unwrap();
        assert!(matches!(select_cached(&store, Some(ORIGIN), NOW + 1), CachedPrs::Fresh(_)), "precondition");

        for (at, number) in [(NOW + 1, 7), (NOW + 1, 8), (NOW + 10, 9)] {
            let (calls, source) = stub(Ok(vec![pr(number, Some("o/r"), "feat/y", "main")]));
            assert_eq!(refresh_with(&store, &root, at, source), RefreshOutcome::Refreshed, "at {at}");
            assert_eq!(calls.get(), 1, "one request at {at}");
            let CachedPrs::Fresh(listing) = select_cached(&store, Some(ORIGIN), at) else {
                panic!("the new answer is served");
            };
            assert_eq!((numbers(&listing), listing.fetched_at), (vec![number], Some(at)));
        }
        assert_ne!(stored_publication(&store, ORIGIN, NOW + 10).unwrap(), first);

        // A successful empty answer replaces the badges too.
        let (_, source) = stub(Ok(Vec::new()));
        assert_eq!(refresh_with(&store, &root, NOW + 11, source), RefreshOutcome::Refreshed);
        let CachedPrs::Fresh(listing) = select_cached(&store, Some(ORIGIN), NOW + 11) else {
            panic!("an empty answer is served");
        };
        assert!(listing.pull_requests.is_empty());
    }

    #[test]
    fn a_refresh_fills_a_miss_including_an_answer_for_another_origin() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, vec![pr(1, Some("o/r"), "a", "main")]);

        git(&root, &["remote", "set-url", "origin", "https://prs.example.invalid/o/new.git"]);
        let (calls, source) = stub(Ok(vec![pr(2, Some("o/new"), "b", "main")]));
        assert_eq!(refresh_with(&store, &root, NOW + 1, source), RefreshOutcome::Refreshed);
        assert_eq!(calls.get(), 1);
        let CachedPrs::Fresh(listing) = select_cached(&store, Some("https://prs.example.invalid/o/new.git"), NOW + 2) else {
            panic!("bound to the new origin");
        };
        assert_eq!(numbers(&listing), [2]);
        assert_eq!(select_cached(&store, Some(ORIGIN), NOW + 2), CachedPrs::Miss, "the old origin's answer is gone");
    }

    #[test]
    fn a_refresh_makes_no_request_while_another_holds_the_lock() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);
        let before = fs::read(&store).unwrap();

        let holder = try_lock_sidecar(&pr_lock_path(&store)).unwrap().expect("the first lock is free");
        assert!(pr_lock_held(&store));
        let (calls, source) = stub(Ok(Vec::new()));
        assert_eq!(refresh_with(&store, &root, NOW + 600, source), RefreshOutcome::Contended);
        assert_eq!(calls.get(), 0);
        assert_eq!(fs::read(&store).unwrap(), before);

        drop(holder);
        assert!(!pr_lock_held(&store));
        assert!(pr_lock_path(&store).exists(), "the sidecar persists across holders");
        let (calls, source) = stub(Ok(Vec::new()));
        assert_eq!(refresh_with(&store, &root, NOW + 600, source), RefreshOutcome::Refreshed);
        assert_eq!(calls.get(), 1, "a released lock lets the next refresh run");
        assert!(pr_lock_path(&store).exists(), "a refresh never deletes the sidecar");
    }

    /// The lock is held through the request, so a refresh started while
    /// another's request is in flight makes no request of its own.
    #[test]
    fn a_refresh_during_another_refreshs_request_is_contended() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        let (calls, mut source) = stub(Ok(vec![pr(1, Some("o/r"), "a", "main")]));
        let (during_store, during_root) = (store.clone(), root.clone());
        let inner_calls = Rc::new(Cell::new(usize::MAX));
        let seen = Rc::clone(&inner_calls);
        source.during = Some(Box::new(move || {
            let (calls, inner) = stub(Ok(vec![pr(2, Some("o/r"), "b", "main")]));
            assert_eq!(refresh_with(&during_store, &during_root, NOW, inner), RefreshOutcome::Contended);
            seen.set(calls.get());
        }));

        assert_eq!(refresh_with(&store, &root, NOW, source), RefreshOutcome::Refreshed);
        assert_eq!((calls.get(), inner_calls.get()), (1, 0));
        let CachedPrs::Fresh(listing) = select_cached(&store, Some(ORIGIN), NOW) else {
            panic!("the holder's answer is stored");
        };
        assert_eq!(numbers(&listing), [1]);
    }

    #[test]
    fn a_refresh_that_cannot_open_its_lock_makes_no_request() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, Vec::new());
        let before = fs::read(&store).unwrap();
        // The seed made the sidecar; a directory in its place cannot be locked.
        fs::remove_file(pr_lock_path(&store)).unwrap();
        fs::create_dir_all(pr_lock_path(&store)).unwrap();
        let (calls, source) = stub(Ok(Vec::new()));
        assert_eq!(refresh_with(&store, &root, NOW + 600, source), RefreshOutcome::LockFailed);
        assert_eq!(calls.get(), 0);
        assert_eq!(fs::read(&store).unwrap(), before);
    }

    #[test]
    fn a_failed_refresh_leaves_the_stored_answer_untouched() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);
        let before = fs::read(&store).unwrap();
        for reason in [PrFailure::Other, PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) }] {
            let (calls, source) = failing(reason.clone());
            assert_eq!(refresh_with(&store, &root, NOW + 600, source), RefreshOutcome::Failed(reason.clone()));
            assert_eq!(calls.get(), 1);
            assert_eq!(fs::read(&store).unwrap(), before, "{reason:?} must not replace the answer");
        }
        let CachedPrs::Stale(listing) = select_cached(&store, Some(ORIGIN), NOW + 600) else {
            panic!("still the stale answer");
        };
        assert_eq!(numbers(&listing), [99]);
    }

    #[test]
    fn each_credentials_failure_is_reported_and_never_stored() {
        let key = || Some("GITHUB_TOKEN".to_string());
        for failure in [
            PrFailure::CredentialsRequired,
            PrFailure::CredentialsRejected { key: key() },
            PrFailure::CredentialsInsufficient { key: key() },
            PrFailure::RateLimited { authenticated: false, key: None },
            PrFailure::RateLimited { authenticated: true, key: key() },
            PrFailure::NotFoundOrNotPermitted,
        ] {
            let (_dir, root, store) = repo(Some(ORIGIN));
            let (calls, source) = failing(failure.clone());
            assert_eq!(refresh_with(&store, &root, NOW, source), RefreshOutcome::Failed(failure.clone()));
            assert_eq!(calls.get(), 1);
            assert!(!store.exists(), "{failure:?} must never be stored");
            assert_eq!(select_cached(&store, Some(ORIGIN), NOW), CachedPrs::Miss, "{failure:?}");
        }
    }

    #[test]
    fn a_refresh_discards_its_answer_when_origin_changed_during_the_request() {
        let (_dir, root, store) = repo(Some(ORIGIN));
        seed(&store, &root, NOW, vec![pr(99, Some("o/r"), "fix/x", "main")]);
        let before = fs::read(&store).unwrap();
        let (_, mut source) = stub(Ok(vec![pr(7, Some("o/r"), "fix/x", "main")]));
        let moved = root.clone();
        source.during = Some(Box::new(move || git(&moved, &["remote", "set-url", "origin", "https://prs.example.invalid/o/new.git"])));
        assert_eq!(refresh_with(&store, &root, NOW + 600, source), RefreshOutcome::OriginChanged);
        assert_eq!(fs::read(&store).unwrap(), before);
        assert_eq!(select_cached(&store, Some("https://prs.example.invalid/o/new.git"), NOW + 600), CachedPrs::Miss);
    }

    #[test]
    fn a_refresh_without_an_origin_makes_no_request() {
        let (_dir, root, store) = repo(None);
        let (calls, source) = stub(Ok(Vec::new()));
        assert_eq!(refresh_with(&store, &root, NOW, source), RefreshOutcome::NoOrigin);
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
