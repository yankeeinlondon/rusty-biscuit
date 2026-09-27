//! The last live answer for `origin`'s default branch, for the `wt list`
//! caption's remote observation.
//!
//! `wt list` compares the local default branch with its local tracking ref
//! `origin/<default>`, which is only as new as the last fetch. This store
//! records what `git ls-remote origin refs/heads/<default>` answered, so the
//! caption can say whether that tracking ref still matched the remote and how
//! long ago that was checked. Listing only reads it; [`refresh_remote_head`]
//! runs in the detached `wt internal-refresh` worker.
//!
//! Contracts:
//!
//! - The file is `<repo hash>.remote-head.json` beside the comparison cache
//!   (see [`crate::cache::repo_cache_file`]), bound to the
//!   [`origin_digest`] of `origin`'s exact URL and to the default branch. The
//!   URL itself is never stored or logged.
//! - `checked_at` is read before the request, so a slow request errs old.
//! - `sha: null` is a verified absence: only a complete, successful answer
//!   without the exact ref (see [`crate::live_remote`]). A failure, a deadline,
//!   or malformed output leaves the previous bytes untouched.
//! - A stored SHA that differs from this listing's tracking tip says only that
//!   they differ. It is not chronology: the fetch may be newer than the check,
//!   and the remote may have rewound or been recreated.
//! - The refresh holds a nonblocking lock on the persistent sidecar
//!   `<repo hash>.remote-head.lock` from the freshness recheck through
//!   publication, and never unlinks it. Publication is an atomic rename, so
//!   readers take no lock and never see a partial document.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::cache::{atomic_write, repo_cache_file, try_lock_sidecar};
use crate::error::WorktreeError;
use crate::live_remote::{RemoteHeads, is_object_id};
use crate::pull_requests::{FRESHNESS_WINDOW, RefreshOutcome, origin_digest, origin_url};
use crate::worktree::default_branch_in;

pub const REMOTE_HEAD_FORMAT_VERSION: u32 = 1;

/// How long a background live-head request may take; nothing waits on it.
pub const REMOTE_HEAD_REFRESH_DEADLINE: Duration = Duration::from_secs(10);

#[derive(Serialize, Deserialize)]
struct StoreFile {
    format_version: u32,
    origin_digest: String,
    /// The default branch, without `refs/heads/`.
    branch: String,
    // `deserialize_with` makes the field required: a document without `sha`
    // is corrupt, not a verified absence.
    #[serde(deserialize_with = "Option::deserialize")]
    sha: Option<String>,
    /// Unix seconds, read immediately before the request.
    checked_at: u64,
}

/// A usable stored answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteHead {
    /// The default branch it was asked for.
    pub branch: String,
    /// Its full object ID on `origin`, or `None` when `origin` answered
    /// without it.
    pub sha: Option<String>,
    /// Unix seconds at which the request started.
    pub checked_at: u64,
}

impl RemoteHead {
    /// Whether the answer is at least [`FRESHNESS_WINDOW`] old at `now`. A
    /// future answer is not stale; see [`RemoteHead::is_future_at`].
    pub fn is_stale_at(&self, now: u64) -> bool {
        self.checked_at <= now && now - self.checked_at >= FRESHNESS_WINDOW.as_secs()
    }

    /// Whether the answer is dated after `now`, which makes it unusable.
    pub fn is_future_at(&self, now: u64) -> bool {
        self.checked_at > now
    }
}

/// A stored answer for the current `origin` and default branch, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachedRemoteHead {
    /// Younger than [`FRESHNESS_WINDOW`]: no refresh needed.
    Fresh(RemoteHead),
    /// Still shown with its age, and refreshed in the background.
    Stale(RemoteHead),
    /// No usable answer: none stored, unreadable, another format, an invalid
    /// field, checked in the future, bound to another `origin` or branch, or
    /// no `origin` or default branch at all.
    Miss,
}

/// The store file for the repository whose main worktree is `repo_root`.
pub fn remote_head_store_path(repo_root: &Path) -> Result<PathBuf, WorktreeError> {
    repo_cache_file(repo_root, "remote-head.json")
}

/// The refresh lock sidecar for the store at `store`:
/// `<repo hash>.remote-head.lock`.
pub fn remote_head_lock_path(store: &Path) -> PathBuf {
    store.with_extension("lock")
}

/// What the store at `store` offers for `origin` (the current
/// [`origin_url`]) and `default_branch`.
pub fn select_cached_head(
    store: &Path,
    origin: Option<&str>,
    default_branch: Option<&str>,
    now: u64,
) -> CachedRemoteHead {
    let (Some(origin), Some(default_branch)) = (origin, default_branch) else {
        return CachedRemoteHead::Miss;
    };
    let Some(file) = load(store) else {
        return CachedRemoteHead::Miss;
    };
    if file.origin_digest != origin_digest(origin) || file.branch != default_branch {
        return CachedRemoteHead::Miss;
    }
    let head = RemoteHead {
        branch: file.branch,
        sha: file.sha,
        checked_at: file.checked_at,
    };
    if head.is_future_at(now) {
        CachedRemoteHead::Miss
    } else if head.is_stale_at(now) {
        CachedRemoteHead::Stale(head)
    } else {
        CachedRemoteHead::Fresh(head)
    }
}

/// Asks `origin` for its default branch through `heads` and stores the
/// answer, unless another process is refreshing it or already has.
///
/// `repo_root` is the main checkout; `origin` and the default branch are read
/// from it before the request and again after it, and any change discards the
/// answer. `origin` is addressed by name, since the tracking ref comes from
/// its fetch URL.
pub fn refresh_remote_head(
    store: &Path,
    repo_root: &Path,
    clock: impl Fn() -> u64,
    heads: &dyn RemoteHeads,
) -> RefreshOutcome {
    let Some(_lock) = (match try_lock_sidecar(&remote_head_lock_path(store)) {
        Ok(lock) => lock,
        Err(_) => return RefreshOutcome::LockFailed,
    }) else {
        return RefreshOutcome::Contended;
    };

    let Some(origin) = origin_url(repo_root) else {
        return RefreshOutcome::NoOrigin;
    };
    let Ok(branch) = default_branch_in(repo_root) else {
        return RefreshOutcome::NoDefaultBranch;
    };
    let checked_at = clock();
    if matches!(
        select_cached_head(store, Some(&origin), Some(&branch), checked_at),
        CachedRemoteHead::Fresh(_)
    ) {
        return RefreshOutcome::AlreadyFresh;
    }
    let Ok(sha) = heads.live_head("origin", &branch) else {
        return RefreshOutcome::Failed;
    };
    if origin_url(repo_root).as_deref() != Some(origin.as_str()) {
        return RefreshOutcome::OriginChanged;
    }
    if default_branch_in(repo_root).ok().as_deref() != Some(branch.as_str()) {
        return RefreshOutcome::DefaultBranchChanged;
    }
    let file = StoreFile {
        format_version: REMOTE_HEAD_FORMAT_VERSION,
        origin_digest: origin_digest(&origin),
        branch,
        sha,
        checked_at,
    };
    match save(store, &file) {
        Ok(()) => RefreshOutcome::Refreshed,
        Err(_) => RefreshOutcome::PublishFailed,
    }
}

fn load(path: &Path) -> Option<StoreFile> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice::<StoreFile>(&bytes).ok().filter(|file| {
        file.format_version == REMOTE_HEAD_FORMAT_VERSION
            && !file.branch.is_empty()
            && file.sha.as_deref().is_none_or(is_object_id)
    })
}

fn save(path: &Path, file: &StoreFile) -> Result<(), WorktreeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write(path, &serde_json::to_vec_pretty(file)?)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::mpsc;

    use super::*;
    use crate::live_remote::LsRemote;
    use crate::remove::test_support::TestRepo;

    const NOW: u64 = 1_790_000_000;
    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    const OTHER_SHA: &str = "89abcdef0123456789abcdef0123456789abcdef";
    const BOUND: Duration = Duration::from_secs(10);

    /// Pauses a request until the test releases it.
    struct Gate {
        ready: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
    }

    /// A scripted [`RemoteHeads`] that records each request.
    struct Stub {
        answer: Result<Option<String>, String>,
        calls: Mutex<Vec<String>>,
        gate: Option<Gate>,
    }

    impl Stub {
        fn new(answer: Result<Option<&str>, &str>) -> Self {
            Self {
                answer: answer.map(|sha| sha.map(str::to_string)).map_err(str::to_string),
                calls: Mutex::new(Vec::new()),
                gate: None,
            }
        }

        /// A stub whose request signals the first receiver when it starts and
        /// waits for the second sender before answering.
        fn gated(answer: Result<Option<&str>, &str>) -> (Self, mpsc::Receiver<()>, mpsc::Sender<()>) {
            let (ready, started) = mpsc::channel();
            let (release, released) = mpsc::channel();
            let mut stub = Self::new(answer);
            stub.gate = Some(Gate { ready, release: Mutex::new(released) });
            (stub, started, release)
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl RemoteHeads for Stub {
        fn live_head(&self, remote: &str, branch: &str) -> Result<Option<String>, String> {
            self.calls.lock().unwrap().push(format!("{remote} {branch}"));
            if let Some(gate) = &self.gate {
                gate.ready.send(()).unwrap();
                gate.release
                    .lock()
                    .unwrap()
                    .recv_timeout(BOUND)
                    .expect("the test releases the request");
            }
            self.answer.clone()
        }
    }

    fn store(repo: &TestRepo) -> PathBuf {
        repo.cache_path().join("abc.remote-head.json")
    }

    fn origin(repo: &TestRepo) -> String {
        origin_url(&repo.path()).expect("the test repository has an origin")
    }

    fn select(repo: &TestRepo, now: u64) -> CachedRemoteHead {
        select_cached_head(&store(repo), Some(&origin(repo)), Some("main"), now)
    }

    fn head(sha: Option<&str>, checked_at: u64) -> RemoteHead {
        RemoteHead { branch: "main".into(), sha: sha.map(str::to_string), checked_at }
    }

    /// Stores `sha` for the repository's origin and `main`, checked at `at`.
    fn seed(repo: &TestRepo, sha: Option<&str>, at: u64) {
        assert_eq!(
            refresh_remote_head(&store(repo), &repo.path(), || at, &Stub::new(Ok(sha))),
            RefreshOutcome::Refreshed
        );
    }

    fn raw(repo: &TestRepo, document: serde_json::Value) {
        fs::create_dir_all(repo.cache_path()).unwrap();
        fs::write(store(repo), serde_json::to_vec(&document).unwrap()).unwrap();
    }

    fn document(repo: &TestRepo) -> serde_json::Value {
        serde_json::json!({
            "format_version": REMOTE_HEAD_FORMAT_VERSION,
            "origin_digest": origin_digest(&origin(repo)),
            "branch": "main",
            "sha": SHA,
            "checked_at": NOW,
        })
    }

    #[test]
    fn an_answer_is_fresh_below_60_seconds_and_stale_from_60() {
        let repo = TestRepo::with_origin();
        seed(&repo, Some(SHA), NOW);

        assert_eq!(select(&repo, NOW), CachedRemoteHead::Fresh(head(Some(SHA), NOW)));
        assert_eq!(select(&repo, NOW + 59), CachedRemoteHead::Fresh(head(Some(SHA), NOW)));
        assert_eq!(select(&repo, NOW + 60), CachedRemoteHead::Stale(head(Some(SHA), NOW)));
        assert_eq!(select(&repo, NOW + 86_400), CachedRemoteHead::Stale(head(Some(SHA), NOW)));

        // Age is judged again at render time.
        let fresh = head(Some(SHA), NOW);
        assert!(!fresh.is_stale_at(NOW + 59) && fresh.is_stale_at(NOW + 60));
        assert!(fresh.is_future_at(NOW - 1) && !fresh.is_stale_at(NOW - 1));
        assert!(!fresh.is_future_at(NOW));
    }

    #[test]
    fn a_verified_absence_is_an_answer_not_a_miss() {
        let repo = TestRepo::with_origin();
        seed(&repo, None, NOW);
        assert_eq!(select(&repo, NOW + 1), CachedRemoteHead::Fresh(head(None, NOW)));
        assert_eq!(select(&repo, NOW + 60), CachedRemoteHead::Stale(head(None, NOW)));
    }

    #[test]
    fn another_origin_another_branch_no_origin_and_a_future_answer_are_misses() {
        let repo = TestRepo::with_origin();
        seed(&repo, Some(SHA), NOW);
        let store = store(&repo);
        let origin = origin(&repo);

        for (origin, branch) in [
            (Some("/elsewhere/origin.git"), Some("main")),
            (Some(origin.as_str()), Some("trunk")),
            (None, Some("main")),
            (Some(origin.as_str()), None),
        ] {
            assert_eq!(select_cached_head(&store, origin, branch, NOW + 1), CachedRemoteHead::Miss);
        }
        // A clock set back must not freeze an answer, fresh or stale.
        assert_eq!(select(&repo, NOW - 1), CachedRemoteHead::Miss);
        assert_eq!(select(&repo, NOW - 3600), CachedRemoteHead::Miss);
        assert!(matches!(select(&repo, NOW), CachedRemoteHead::Fresh(_)), "control");
    }

    #[test]
    fn corrupt_other_format_invalid_and_incomplete_documents_are_misses() {
        let repo = TestRepo::with_origin();
        let with = |key: &str, value: serde_json::Value| {
            let mut document = document(&repo);
            document[key] = value;
            document
        };
        let without = |key: &str| {
            let mut document = document(&repo);
            document.as_object_mut().unwrap().remove(key);
            document
        };
        for document in [
            with("format_version", 0.into()),
            with("format_version", 2.into()),
            with("sha", SHA[..39].into()),
            with("sha", format!("{SHA}0").into()),
            with("sha", SHA.to_uppercase().into()),
            with("sha", SHA[..12].into()),
            with("sha", "g".repeat(40).into()),
            with("sha", 7.into()),
            with("branch", "".into()),
            with("checked_at", "yesterday".into()),
            without("sha"),
            without("branch"),
            without("checked_at"),
            without("origin_digest"),
            without("format_version"),
        ] {
            raw(&repo, document.clone());
            assert_eq!(select(&repo, NOW + 1), CachedRemoteHead::Miss, "{document}");
        }
        fs::write(store(&repo), b"{not json").unwrap();
        assert_eq!(select(&repo, NOW + 1), CachedRemoteHead::Miss);
        fs::remove_file(store(&repo)).unwrap();
        assert_eq!(select(&repo, NOW + 1), CachedRemoteHead::Miss);

        // SHA-256 object IDs and a verified absence are valid.
        let sha256 = "e".repeat(64);
        raw(&repo, with("sha", sha256.clone().into()));
        assert_eq!(select(&repo, NOW + 1), CachedRemoteHead::Fresh(head(Some(&sha256), NOW)));
        raw(&repo, with("sha", serde_json::Value::Null));
        assert_eq!(select(&repo, NOW + 1), CachedRemoteHead::Fresh(head(None, NOW)));
    }

    #[test]
    fn a_refresh_asks_origin_by_name_for_the_default_branch_and_stamps_its_start() {
        let repo = TestRepo::with_origin();
        let stub = Stub::new(Ok(Some(SHA)));
        let ticks = std::cell::Cell::new(NOW);
        let clock = || {
            let now = ticks.get();
            ticks.set(now + 5);
            now
        };
        assert_eq!(refresh_remote_head(&store(&repo), &repo.path(), clock, &stub), RefreshOutcome::Refreshed);
        assert_eq!(stub.calls(), ["origin main"]);
        assert_eq!(select(&repo, NOW + 59), CachedRemoteHead::Fresh(head(Some(SHA), NOW)));

        // Read, write, read: a stale answer is replaced and read back fresh.
        let stub = Stub::new(Ok(Some(OTHER_SHA)));
        assert_eq!(
            refresh_remote_head(&store(&repo), &repo.path(), || NOW + 600, &stub),
            RefreshOutcome::Refreshed
        );
        assert_eq!(select(&repo, NOW + 601), CachedRemoteHead::Fresh(head(Some(OTHER_SHA), NOW + 600)));
    }

    #[test]
    fn a_refresh_follows_a_non_main_default_branch() {
        let repo = TestRepo::with_origin();
        repo.git(&["branch", "trunk"]);
        repo.git(&["update-ref", "refs/remotes/origin/trunk", "trunk"]);
        repo.git(&["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/trunk"]);
        let stub = Stub::new(Ok(Some(SHA)));
        assert_eq!(refresh_remote_head(&store(&repo), &repo.path(), || NOW, &stub), RefreshOutcome::Refreshed);
        assert_eq!(stub.calls(), ["origin trunk"]);
        let origin = origin(&repo);
        assert!(matches!(
            select_cached_head(&store(&repo), Some(&origin), Some("trunk"), NOW),
            CachedRemoteHead::Fresh(_)
        ));
        assert_eq!(select(&repo, NOW), CachedRemoteHead::Miss, "bound to trunk, not main");
    }

    #[test]
    fn the_store_records_only_a_digest_of_the_origin() {
        let repo = TestRepo::with_origin();
        let secret = "https://user:hunter2@heads.example.invalid/o/r.git";
        repo.git(&["remote", "set-url", "origin", secret]);
        seed(&repo, Some(SHA), NOW);
        let bytes = fs::read_to_string(store(&repo)).unwrap();
        assert!(!bytes.contains("hunter2") && !bytes.contains("example.invalid"), "{bytes}");
        assert!(bytes.contains(&origin_digest(secret)));
    }

    #[test]
    fn without_an_origin_or_a_default_branch_no_request_is_made() {
        let repo = TestRepo::new();
        let stub = Stub::new(Ok(Some(SHA)));
        assert_eq!(refresh_remote_head(&store(&repo), &repo.path(), || NOW, &stub), RefreshOutcome::NoOrigin);

        let repo = TestRepo::with_origin();
        repo.git(&["branch", "-m", "main", "trunk"]);
        assert_eq!(
            refresh_remote_head(&store(&repo), &repo.path(), || NOW, &stub),
            RefreshOutcome::NoDefaultBranch
        );
        assert!(stub.calls().is_empty());
        assert!(!store(&repo).exists());
    }

    #[test]
    fn a_fresh_answer_skips_the_request() {
        let repo = TestRepo::with_origin();
        seed(&repo, Some(SHA), NOW);
        let before = fs::read(store(&repo)).unwrap();
        let stub = Stub::new(Ok(Some(OTHER_SHA)));
        assert_eq!(
            refresh_remote_head(&store(&repo), &repo.path(), || NOW + 59, &stub),
            RefreshOutcome::AlreadyFresh
        );
        assert!(stub.calls().is_empty());
        assert_eq!(fs::read(store(&repo)).unwrap(), before);
    }

    #[test]
    fn failures_leave_the_previous_bytes_untouched() {
        let repo = TestRepo::with_origin();
        seed(&repo, Some(SHA), NOW);
        let before = fs::read(store(&repo)).unwrap();
        for reason in ["origin did not answer within 10 s", "unexpected ls-remote output: \"garbage\""] {
            let stub = Stub::new(Err(reason));
            assert_eq!(
                refresh_remote_head(&store(&repo), &repo.path(), || NOW + 600, &stub),
                RefreshOutcome::Failed
            );
            assert_eq!(stub.calls().len(), 1);
            assert_eq!(fs::read(store(&repo)).unwrap(), before, "{reason} must not replace the answer");
        }
        assert_eq!(select(&repo, NOW + 600), CachedRemoteHead::Stale(head(Some(SHA), NOW)));

        // A failure with no previous answer stores nothing, least of all absence.
        let empty = TestRepo::with_origin();
        let stub = Stub::new(Err("could not read git's output"));
        assert_eq!(refresh_remote_head(&store(&empty), &empty.path(), || NOW, &stub), RefreshOutcome::Failed);
        assert!(!store(&empty).exists());
    }

    #[test]
    fn a_publication_or_lock_failure_is_reported_and_changes_nothing() {
        let repo = TestRepo::with_origin();
        fs::create_dir_all(store(&repo)).unwrap();
        let stub = Stub::new(Ok(Some(SHA)));
        assert_eq!(
            refresh_remote_head(&store(&repo), &repo.path(), || NOW, &stub),
            RefreshOutcome::PublishFailed
        );
        assert!(store(&repo).is_dir() && fs::read_dir(store(&repo)).unwrap().next().is_none());
        assert_eq!(select(&repo, NOW), CachedRemoteHead::Miss);

        let repo = TestRepo::with_origin();
        seed(&repo, Some(SHA), NOW);
        let before = fs::read(store(&repo)).unwrap();
        fs::remove_file(remote_head_lock_path(&store(&repo))).unwrap();
        fs::create_dir(remote_head_lock_path(&store(&repo))).unwrap();
        let stub = Stub::new(Ok(Some(OTHER_SHA)));
        assert_eq!(
            refresh_remote_head(&store(&repo), &repo.path(), || NOW + 600, &stub),
            RefreshOutcome::LockFailed
        );
        assert!(stub.calls().is_empty());
        assert_eq!(fs::read(store(&repo)).unwrap(), before);
    }

    /// Runs a refresh whose request is held until `during` has run.
    fn refresh_while_blocked(repo: &TestRepo, during: impl FnOnce()) -> RefreshOutcome {
        let (stub, started, release) = Stub::gated(Ok(Some(OTHER_SHA)));
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| refresh_remote_head(&store(repo), &repo.path(), || NOW + 600, &stub));
            started.recv_timeout(BOUND).expect("the request starts");
            during();
            release.send(()).unwrap();
            worker.join().unwrap()
        })
    }

    #[test]
    fn an_origin_change_during_the_request_discards_the_answer() {
        let repo = TestRepo::with_origin();
        seed(&repo, Some(SHA), NOW);
        let before = fs::read(store(&repo)).unwrap();
        let outcome = refresh_while_blocked(&repo, || {
            repo.git(&["remote", "set-url", "origin", "https://heads.example.invalid/o/new.git"]);
        });
        assert_eq!(outcome, RefreshOutcome::OriginChanged);
        assert_eq!(fs::read(store(&repo)).unwrap(), before);
    }

    #[test]
    fn a_default_branch_change_during_the_request_discards_the_answer() {
        let repo = TestRepo::with_origin();
        repo.git(&["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
        seed(&repo, Some(SHA), NOW);
        let before = fs::read(store(&repo)).unwrap();
        let outcome = refresh_while_blocked(&repo, || {
            repo.git(&["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/trunk"]);
        });
        assert_eq!(outcome, RefreshOutcome::DefaultBranchChanged);
        assert_eq!(fs::read(store(&repo)).unwrap(), before);
    }

    #[test]
    fn a_competing_refresh_is_contended_and_a_published_answer_stops_the_next() {
        let repo = TestRepo::with_origin();
        seed(&repo, Some(SHA), NOW);
        let second = Stub::new(Ok(Some(SHA)));
        let first = refresh_while_blocked(&repo, || {
            assert_eq!(
                refresh_remote_head(&store(&repo), &repo.path(), || NOW + 600, &second),
                RefreshOutcome::Contended
            );
        });
        assert_eq!(first, RefreshOutcome::Refreshed);
        assert!(second.calls().is_empty(), "contention makes no request");
        assert!(remote_head_lock_path(&store(&repo)).exists(), "the sidecar is never unlinked");

        let third = Stub::new(Ok(Some(SHA)));
        assert_eq!(
            refresh_remote_head(&store(&repo), &repo.path(), || NOW + 601, &third),
            RefreshOutcome::AlreadyFresh
        );
        assert!(third.calls().is_empty());
        assert_eq!(select(&repo, NOW + 601), CachedRemoteHead::Fresh(head(Some(OTHER_SHA), NOW + 600)));
    }

    #[test]
    fn git_ls_remote_stores_present_pushed_and_absent_heads() {
        let repo = TestRepo::with_origin();
        let path = repo.path();
        let heads = LsRemote { base: &path, deadline: REMOTE_HEAD_REFRESH_DEADLINE };
        let refresh = |at| refresh_remote_head(&store(&repo), &path, || at, &heads);

        assert_eq!(refresh(NOW), RefreshOutcome::Refreshed);
        let tracking = repo.sha("origin/main");
        assert_eq!(select(&repo, NOW), CachedRemoteHead::Fresh(head(Some(&tracking), NOW)));

        // Another clone's push is seen live, with no fetch here.
        let pushed = repo.push_commit_to_origin("main", "upstream.txt");
        assert_eq!(refresh(NOW + 60), RefreshOutcome::Refreshed);
        assert_eq!(select(&repo, NOW + 60), CachedRemoteHead::Fresh(head(Some(&pushed), NOW + 60)));
        assert_eq!(repo.sha("origin/main"), tracking, "the tracking ref is untouched");

        repo.git_in(&repo.origin_path(), &["update-ref", "-d", "refs/heads/main"]);
        assert_eq!(refresh(NOW + 120), RefreshOutcome::Refreshed);
        assert_eq!(select(&repo, NOW + 120), CachedRemoteHead::Fresh(head(None, NOW + 120)));
    }

    #[test]
    fn store_and_lock_sit_beside_the_comparison_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = crate::cache::cache_path(dir.path()).unwrap();
        let store = remote_head_store_path(dir.path()).unwrap();
        assert_eq!(cache.parent(), store.parent());
        let stem = cache.file_stem().unwrap().to_string_lossy().into_owned();
        assert_eq!(store.file_name().unwrap().to_string_lossy(), format!("{stem}.remote-head.json"));
        assert_eq!(
            remote_head_lock_path(&store).file_name().unwrap().to_string_lossy(),
            format!("{stem}.remote-head.lock")
        );
    }
}
