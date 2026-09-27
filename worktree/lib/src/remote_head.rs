//! The last live answer for `origin`'s default branch, for the `wt list`
//! caption's remote observation, and the state of the attempt refreshing it.
//!
//! `wt list` compares the local default branch with its local tracking ref
//! `origin/<default>`, which is only as new as the last fetch. This store
//! records what `origin` last answered for `refs/heads/<default>`, so the
//! caption can say whether that tracking ref still matched the remote and how
//! long ago that was checked. Listing only reads it; the detached
//! `wt internal-refresh` worker writes it.
//!
//! Contracts:
//!
//! - The file is `<repo hash>.remote-head.json` beside the comparison cache
//!   (see [`crate::cache::repo_cache_file`]). Format 2 holds two independent
//!   halves, `answer` and `attempt`, each bound to the [`origin_digest`] of
//!   `origin`'s exact URL and to the default branch. The URL itself is never
//!   stored or logged. A format-1 file (a bare answer) reads as an `answer`
//!   from [`AnswerSource::Git`] with no attempt.
//! - Each half is validated on its own: an invalid half is dropped and the
//!   other is kept. Only an unreadable document or an unknown
//!   `format_version` loses both. A discarded attempt never touches `answer`,
//!   and an attempt's failure never replaces it.
//! - `checked_at` is read before the request, so a slow request errs old.
//! - `sha: null` is a verified absence: only a complete, successful answer
//!   without the exact ref (see [`crate::live_remote`]). A failure, a deadline,
//!   or malformed output leaves the previous answer untouched.
//! - A stored SHA that differs from this listing's tracking tip says only that
//!   they differ. It is not chronology: the fetch may be newer than the check,
//!   and the remote may have rewound or been recreated.
//! - Writers hold a nonblocking lock on the persistent sidecar
//!   `<repo hash>.remote-head.lock` and never unlink it; each writer is a
//!   read-modify-write of one half. Publication is an atomic rename, so
//!   readers take no lock and never see a partial document.
//! - The completion receipt `<repo hash>.refresh-receipt.json` ([`Receipt`])
//!   is bound to one attempt id; last writer wins, and readers key on the id.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::cache::{atomic_write, repo_cache_file, try_lock_sidecar};
use crate::error::WorktreeError;
use crate::live_remote::{RemoteHeads, is_object_id};
use crate::pull_requests::{FRESHNESS_WINDOW, RefreshOutcome, origin_digest, origin_url};
use crate::worktree::default_branch_in;

pub const REMOTE_HEAD_FORMAT_VERSION: u32 = 2;

/// The bare-answer format, still read so an upgrade keeps its evidence.
const LEGACY_FORMAT_VERSION: u64 = 1;

pub const RECEIPT_FORMAT_VERSION: u32 = 1;

/// How long a background live-head request may take; nothing waits on it.
pub const REMOTE_HEAD_REFRESH_DEADLINE: Duration = Duration::from_secs(10);

/// Past this age an attempt is abandoned: the 10 s check, the 60 s fetch,
/// and a 5 s publication allowance.
pub const ATTEMPT_MAX_AGE: Duration = Duration::from_secs(10 + 60 + 5);

/// Which request produced an [`Answer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnswerSource {
    /// The provider's branch-head API.
    Api,
    /// `git ls-remote`.
    Git,
    /// The tracking tip read back after a successful fetch.
    Fetch,
}

/// The last successful answer, whatever the attempt that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    pub origin_digest: String,
    /// The default branch, without `refs/heads/`.
    pub branch: String,
    // `deserialize_with` makes the field required: a document without `sha`
    // is corrupt, not a verified absence.
    #[serde(deserialize_with = "Option::deserialize")]
    pub sha: Option<String>,
    /// Unix seconds, read immediately before the request.
    pub checked_at: u64,
    pub source: AnswerSource,
}

impl Answer {
    fn is_valid(&self) -> bool {
        !self.origin_digest.is_empty() && !self.branch.is_empty() && self.sha.as_deref().is_none_or(is_object_id)
    }

    /// The answer as the caption shows it.
    pub fn head(&self) -> RemoteHead {
        RemoteHead { branch: self.branch.clone(), sha: self.sha.clone(), checked_at: self.checked_at }
    }
}

/// Why a check left the provider API for `git ls-remote`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FallbackReason {
    NoKey,
    RateLimited,
    /// The provider did not show the repository, and no key was set.
    NotVisible,
    /// The provider refused the key that was set.
    Rejected,
    Other,
}

/// Where an unfinished attempt is. Serialized as `{"kind": "checking"}`,
/// `{"kind": "checking-fallback", "reason": "no-key"}`, `{"kind": "fetching"}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Phase {
    Checking,
    CheckingFallback { reason: FallbackReason },
    Fetching,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckFailure {
    Timeout,
    Credentials,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FetchFailure {
    Timeout,
    Other,
}

/// Why an attempt discarded its answer instead of publishing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnavailableReason {
    OriginChanged,
    BranchChanged,
}

/// How an attempt ended. Serialized as `{"kind": "in-sync"}`, or with a
/// `reason` for the last three, e.g. `{"kind": "fetch-failed", "reason": "timeout"}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Outcome {
    InSync,
    Fetched,
    Absent,
    FetchFailed { reason: FetchFailure },
    CheckFailed { reason: CheckFailure },
    Unavailable { reason: UnavailableReason },
}

/// A provider API condition from spec §5, mirrored from sniff's
/// `PrUnavailable` so the store does not depend on sniff's enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ApiCondition {
    CredentialsRequired,
    CredentialsRejected,
    CredentialsInsufficient,
    RateLimited { authenticated: bool },
    NotFoundOrNotPermitted,
}

/// What this attempt's provider API request observed, for the foreground's
/// §5 credentials line and §8 fallback notice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiNote {
    pub condition: ApiCondition,
    /// The name of the variable the request used; never its value.
    #[serde(deserialize_with = "Option::deserialize")]
    pub key: Option<String>,
    /// Whether the `git ls-remote` fallback then answered.
    pub fallback_answered: bool,
}

impl ApiNote {
    fn is_valid(&self) -> bool {
        self.key.as_deref().is_none_or(is_variable_name)
    }
}

/// One worker's refresh, from its first write to its outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    /// 32 lowercase hex characters; see [`new_attempt_id`].
    pub id: String,
    pub origin_digest: String,
    pub branch: String,
    /// Unix seconds at which the worker took the lock.
    pub started_at: u64,
    pub phase: Phase,
    /// `None` while the attempt is running.
    #[serde(deserialize_with = "Option::deserialize")]
    pub outcome: Option<Outcome>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub api: Option<ApiNote>,
}

impl Attempt {
    /// A new attempt in phase [`Phase::Checking`], with no outcome yet.
    pub fn begin(id: String, origin_digest: String, branch: String, started_at: u64) -> Self {
        Self { id, origin_digest, branch, started_at, phase: Phase::Checking, outcome: None, api: None }
    }

    fn is_valid(&self) -> bool {
        is_attempt_id(&self.id)
            && !self.origin_digest.is_empty()
            && !self.branch.is_empty()
            && self.api.as_ref().is_none_or(ApiNote::is_valid)
    }

    /// Whether the attempt still speaks for `origin_digest` and `branch` at
    /// `now`: not dated in the future, and at most [`ATTEMPT_MAX_AGE`] old.
    pub fn is_current_for(&self, origin_digest: &str, branch: &str, now: u64) -> bool {
        self.origin_digest == origin_digest
            && self.branch == branch
            && self.started_at <= now
            && now - self.started_at <= ATTEMPT_MAX_AGE.as_secs()
    }
}

/// The two halves of the store, each independently valid or absent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoreState {
    pub answer: Option<Answer>,
    /// Only intrinsically valid: see [`select_attempt`] for one that still
    /// applies to the current `origin` and default branch.
    pub attempt: Option<Attempt>,
}

#[derive(Serialize)]
struct StoreFile<'a> {
    format_version: u32,
    answer: Option<&'a Answer>,
    attempt: Option<&'a Attempt>,
}

#[derive(Deserialize)]
struct LegacyStoreFile {
    origin_digest: String,
    branch: String,
    #[serde(deserialize_with = "Option::deserialize")]
    sha: Option<String>,
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

/// The completion receipt for the repository whose main worktree is
/// `repo_root`: `<repo hash>.refresh-receipt.json`.
pub fn refresh_receipt_path(repo_root: &Path) -> Result<PathBuf, WorktreeError> {
    repo_cache_file(repo_root, "refresh-receipt.json")
}

/// A fresh attempt id: 128 bits from the OS random source, the same generator
/// as `wt remove`'s hand-off tokens.
pub fn new_attempt_id() -> Result<String, WorktreeError> {
    crate::remove::handoff::new_token()
}

fn is_attempt_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A name is all a note may hold, so anything else is rejected rather than
/// risk storing a secret.
fn is_variable_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|b| b == b'_' || b.is_ascii_alphabetic())
        && bytes.all(|b| b == b'_' || b.is_ascii_alphanumeric())
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
    let Some(answer) = read_store(store).answer else {
        return CachedRemoteHead::Miss;
    };
    if answer.origin_digest != origin_digest(origin) || answer.branch != default_branch {
        return CachedRemoteHead::Miss;
    }
    let head = answer.head();
    if head.is_future_at(now) {
        CachedRemoteHead::Miss
    } else if head.is_stale_at(now) {
        CachedRemoteHead::Stale(head)
    } else {
        CachedRemoteHead::Fresh(head)
    }
}

/// The stored attempt, if it is valid and still current for `origin` (the
/// current [`origin_url`]) and `default_branch` at `now`
/// ([`Attempt::is_current_for`]).
pub fn select_attempt(
    store: &Path,
    origin: Option<&str>,
    default_branch: Option<&str>,
    now: u64,
) -> Option<Attempt> {
    let (origin, default_branch) = (origin?, default_branch?);
    let digest = origin_digest(origin);
    read_store(store).attempt.filter(|attempt| attempt.is_current_for(&digest, default_branch, now))
}

/// Both halves of the store at `store`, validated independently; a missing or
/// unreadable file, or another format, is empty.
pub fn read_store(store: &Path) -> StoreState {
    let Some(document) = fs::read(store).ok().and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
    else {
        return StoreState::default();
    };
    match document.get("format_version").and_then(serde_json::Value::as_u64) {
        Some(LEGACY_FORMAT_VERSION) => StoreState { answer: legacy_answer(document), attempt: None },
        Some(version) if version == u64::from(REMOTE_HEAD_FORMAT_VERSION) => StoreState {
            answer: half::<Answer>(&document, "answer").filter(Answer::is_valid),
            attempt: half::<Attempt>(&document, "attempt").filter(Attempt::is_valid),
        },
        _ => StoreState::default(),
    }
}

fn legacy_answer(document: serde_json::Value) -> Option<Answer> {
    let file = serde_json::from_value::<LegacyStoreFile>(document).ok()?;
    let answer = Answer {
        origin_digest: file.origin_digest,
        branch: file.branch,
        sha: file.sha,
        checked_at: file.checked_at,
        source: AnswerSource::Git,
    };
    answer.is_valid().then_some(answer)
}

fn half<T: serde::de::DeserializeOwned>(document: &serde_json::Value, key: &str) -> Option<T> {
    document.get(key).and_then(|value| T::deserialize(value).ok())
}

/// Records `attempt` as the current one, replacing any other, and keeps the
/// answer. Call with the lock held.
pub fn begin_attempt(store: &Path, attempt: &Attempt) -> Result<(), WorktreeError> {
    if !attempt.is_valid() {
        return Err(invalid("an invalid attempt"));
    }
    let mut state = read_store(store);
    state.attempt = Some(attempt.clone());
    save(store, &state)
}

/// Moves attempt `id` to `phase`. `api`, when given, replaces the attempt's
/// note; `None` keeps it. Call with the lock held.
///
/// ## Errors
///
/// Fails without writing when the stored attempt is not `id`.
pub fn set_phase(store: &Path, id: &str, phase: Phase, api: Option<ApiNote>) -> Result<(), WorktreeError> {
    update_attempt(store, id, api, |attempt| attempt.phase = phase)
}

/// Records attempt `id`'s outcome, keeping its phase as last reached. `api`
/// is as for [`set_phase`]. Call with the lock held.
///
/// ## Errors
///
/// Fails without writing when the stored attempt is not `id`.
pub fn finish_attempt(store: &Path, id: &str, outcome: Outcome, api: Option<ApiNote>) -> Result<(), WorktreeError> {
    update_attempt(store, id, api, |attempt| attempt.outcome = Some(outcome))
}

/// Replaces the answer and keeps the attempt. Call with the lock held.
pub fn publish_answer(store: &Path, answer: &Answer) -> Result<(), WorktreeError> {
    if !answer.is_valid() {
        return Err(invalid("an invalid answer"));
    }
    let mut state = read_store(store);
    state.answer = Some(answer.clone());
    save(store, &state)
}

fn update_attempt(
    store: &Path,
    id: &str,
    api: Option<ApiNote>,
    change: impl FnOnce(&mut Attempt),
) -> Result<(), WorktreeError> {
    if api.as_ref().is_some_and(|note| !note.is_valid()) {
        return Err(invalid("an invalid API note"));
    }
    let mut state = read_store(store);
    let Some(attempt) = state.attempt.as_mut().filter(|attempt| attempt.id == id) else {
        return Err(WorktreeError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "the stored attempt is not this worker's",
        )));
    };
    change(attempt);
    if api.is_some() {
        attempt.api = api;
    }
    save(store, &state)
}

fn invalid(what: &str) -> WorktreeError {
    WorktreeError::Io(io::Error::new(io::ErrorKind::InvalidInput, format!("refusing to store {what}")))
}

/// Asks `origin` for its default branch through `heads` and publishes the
/// answer (from [`AnswerSource::Git`]), unless another process is refreshing
/// it or already has.
///
/// `repo_root` is the main checkout; `origin` and the default branch are read
/// from it before the request and again after it, and any change discards the
/// answer. `origin` is addressed by name, since the tracking ref comes from
/// its fetch URL. The stored attempt, if any, is left as it was.
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
    let answer = Answer { origin_digest: origin_digest(&origin), branch, sha, checked_at, source: AnswerSource::Git };
    match publish_answer(store, &answer) {
        Ok(()) => RefreshOutcome::Refreshed,
        Err(_) => RefreshOutcome::PublishFailed,
    }
}

fn save(path: &Path, state: &StoreState) -> Result<(), WorktreeError> {
    let file = StoreFile {
        format_version: REMOTE_HEAD_FORMAT_VERSION,
        answer: state.answer.as_ref(),
        attempt: state.attempt.as_ref(),
    };
    write_json(path, &file)
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), WorktreeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write(path, &serde_json::to_vec_pretty(value)?)
}

/// A PR request's failure, typed so this run's failure can produce a spec §5
/// line: a mirror of sniff's credentials variants plus `Other`. It lives here
/// for the receipt; `pull_requests` reuses it for the foreground request.
/// Serialized as `{"kind": "credentials-rejected", "key": "GITHUB_TOKEN"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PrFailure {
    CredentialsRequired,
    CredentialsRejected {
        /// The name of the variable used; never its value.
        key: Option<String>,
    },
    CredentialsInsufficient {
        key: Option<String>,
    },
    RateLimited {
        authenticated: bool,
        key: Option<String>,
    },
    NotFoundOrNotPermitted,
    Other,
}

impl PrFailure {
    fn is_valid(&self) -> bool {
        match self {
            Self::CredentialsRejected { key } | Self::CredentialsInsufficient { key } | Self::RateLimited { key, .. } => {
                key.as_deref().is_none_or(is_variable_name)
            }
            Self::CredentialsRequired | Self::NotFoundOrNotPermitted | Self::Other => true,
        }
    }
}

/// How the live-head half of a forced refresh ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeadStatus {
    Ok,
    Failed,
    /// Another worker held the lock; its attempt carries the result.
    AdoptedElsewhere,
}

/// How the PR half of a forced refresh ended. Serialized as
/// `{"kind": "ok"}`, `{"kind": "failed", "failure": {..}}`, and so on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PrStatus {
    Ok,
    Failed { failure: PrFailure },
    SkippedFresh,
    /// Another worker held the PR lock.
    Contended,
}

/// Written by the worker after both halves of an `--refresh`/`--ff` attempt
/// have finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub attempt_id: String,
    pub origin_digest: String,
    pub branch: String,
    /// Unix seconds.
    pub finished_at: u64,
    pub head: HeadStatus,
    pub prs: PrStatus,
}

#[derive(Serialize)]
struct ReceiptFileOut<'a> {
    format_version: u32,
    #[serde(flatten)]
    receipt: &'a Receipt,
}

#[derive(Deserialize)]
struct ReceiptFileIn {
    format_version: u32,
    #[serde(flatten)]
    receipt: Receipt,
}

impl Receipt {
    fn is_valid(&self) -> bool {
        is_attempt_id(&self.attempt_id)
            && !self.origin_digest.is_empty()
            && !self.branch.is_empty()
            && match &self.prs {
                PrStatus::Failed { failure } => failure.is_valid(),
                PrStatus::Ok | PrStatus::SkippedFresh | PrStatus::Contended => true,
            }
    }
}

/// Publishes `receipt` at `path` atomically, replacing any other.
pub fn write_receipt(path: &Path, receipt: &Receipt) -> Result<(), WorktreeError> {
    if !receipt.is_valid() {
        return Err(invalid("an invalid receipt"));
    }
    write_json(path, &ReceiptFileOut { format_version: RECEIPT_FORMAT_VERSION, receipt })
}

/// The receipt at `path` for `attempt`: `None` when missing, unreadable,
/// invalid, for another attempt id, origin, or branch, or finished before
/// the attempt started.
pub fn load_receipt(path: &Path, attempt: &Attempt) -> Option<Receipt> {
    let bytes = fs::read(path).ok()?;
    let file = serde_json::from_slice::<ReceiptFileIn>(&bytes).ok()?;
    let receipt = file.receipt;
    (file.format_version == RECEIPT_FORMAT_VERSION
        && receipt.is_valid()
        && receipt.attempt_id == attempt.id
        && receipt.origin_digest == attempt.origin_digest
        && receipt.branch == attempt.branch
        && receipt.finished_at >= attempt.started_at)
        .then_some(receipt)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::mpsc;
    use std::time::Instant;

    use super::*;
    use crate::live_remote::LsRemote;
    use crate::live_remote::tests::{FAST, Loopback, http_origin};
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

    /// A format-1 document: a bare answer, read as one from Git.
    fn document(repo: &TestRepo) -> serde_json::Value {
        serde_json::json!({
            "format_version": 1,
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
    fn an_unauthorized_origin_fails_fast_and_leaves_the_store_alone() {
        let server = Loopback::unauthorized();
        let repo = http_origin(&server);
        seed(&repo, Some(SHA), NOW);
        let before = fs::read(store(&repo)).unwrap();
        let path = repo.path();
        let heads = LsRemote { base: &path, deadline: REMOTE_HEAD_REFRESH_DEADLINE };

        let started = Instant::now();
        let outcome = refresh_remote_head(&store(&repo), &path, || NOW + 600, &heads);
        let elapsed = started.elapsed();
        assert_eq!(outcome, RefreshOutcome::Failed);
        assert!(elapsed < FAST, "took {elapsed:?}");
        assert!(server.accepted() >= 1, "git never reached the server");
        assert_eq!(fs::read(store(&repo)).unwrap(), before);

        // With no previous answer, nothing is stored, least of all absence.
        let empty = http_origin(&server);
        let path = empty.path();
        let heads = LsRemote { base: &path, deadline: REMOTE_HEAD_REFRESH_DEADLINE };
        assert_eq!(refresh_remote_head(&store(&empty), &path, || NOW, &heads), RefreshOutcome::Failed);
        assert!(!store(&empty).exists());
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
        let receipt = refresh_receipt_path(dir.path()).unwrap();
        assert_eq!(receipt.file_name().unwrap().to_string_lossy(), format!("{stem}.refresh-receipt.json"));
        assert_eq!(cache.parent(), receipt.parent());
    }

    // Format 2: the answer and attempt halves, and the completion receipt.

    const ID: &str = "0123456789abcdef0123456789abcdef";
    const OTHER_ID: &str = "fedcba9876543210fedcba9876543210";
    const DIGEST: &str = "digest-of-origin";

    fn temp_store() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("abc.remote-head.json");
        (dir, store)
    }

    fn answer(sha: Option<&str>, checked_at: u64, source: AnswerSource) -> Answer {
        Answer {
            origin_digest: DIGEST.into(),
            branch: "main".into(),
            sha: sha.map(str::to_string),
            checked_at,
            source,
        }
    }

    fn attempt(started_at: u64) -> Attempt {
        Attempt::begin(ID.into(), DIGEST.into(), "main".into(), started_at)
    }

    fn note() -> ApiNote {
        ApiNote {
            condition: ApiCondition::RateLimited { authenticated: false },
            key: Some("GITHUB_TOKEN".into()),
            fallback_answered: true,
        }
    }

    fn write_raw(store: &Path, document: &serde_json::Value) {
        fs::write(store, serde_json::to_vec(document).unwrap()).unwrap();
    }

    fn raw_json(path: &Path) -> serde_json::Value {
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }

    /// A store holding a Git answer checked at `NOW` and a running attempt
    /// started at `NOW`.
    fn seeded_store() -> (tempfile::TempDir, PathBuf) {
        let (dir, store) = temp_store();
        publish_answer(&store, &answer(Some(SHA), NOW, AnswerSource::Git)).unwrap();
        begin_attempt(&store, &attempt(NOW)).unwrap();
        (dir, store)
    }

    #[test]
    fn a_format_1_file_reads_as_a_git_answer_and_a_write_keeps_it() {
        let repo = TestRepo::with_origin();
        raw(&repo, document(&repo));
        let expected = Answer {
            origin_digest: origin_digest(&origin(&repo)),
            branch: "main".into(),
            sha: Some(SHA.into()),
            checked_at: NOW,
            source: AnswerSource::Git,
        };
        assert_eq!(
            read_store(&store(&repo)),
            StoreState { answer: Some(expected.clone()), attempt: None }
        );
        assert_eq!(select(&repo, NOW + 1), CachedRemoteHead::Fresh(head(Some(SHA), NOW)));

        let started = Attempt::begin(ID.into(), expected.origin_digest.clone(), "main".into(), NOW + 5);
        begin_attempt(&store(&repo), &started).unwrap();
        assert_eq!(raw_json(&store(&repo))["format_version"], 2);
        assert_eq!(read_store(&store(&repo)), StoreState { answer: Some(expected), attempt: Some(started) });
        assert_eq!(select(&repo, NOW + 1), CachedRemoteHead::Fresh(head(Some(SHA), NOW)));
    }

    #[test]
    fn the_store_round_trips_with_its_documented_spellings() {
        let (_dir, store) = seeded_store();
        set_phase(&store, ID, Phase::CheckingFallback { reason: FallbackReason::RateLimited }, Some(note())).unwrap();
        finish_attempt(&store, ID, Outcome::FetchFailed { reason: FetchFailure::Timeout }, None).unwrap();

        assert_eq!(
            raw_json(&store),
            serde_json::json!({
                "format_version": 2,
                "answer": {
                    "origin_digest": DIGEST, "branch": "main", "sha": SHA,
                    "checked_at": NOW, "source": "git",
                },
                "attempt": {
                    "id": ID, "origin_digest": DIGEST, "branch": "main", "started_at": NOW,
                    "phase": { "kind": "checking-fallback", "reason": "rate-limited" },
                    "outcome": { "kind": "fetch-failed", "reason": "timeout" },
                    "api": {
                        "condition": { "kind": "rate-limited", "authenticated": false },
                        "key": "GITHUB_TOKEN",
                        "fallback_answered": true,
                    },
                },
            })
        );

        // Read, write, read: stable in value and in bytes.
        let first = read_store(&store);
        let bytes = fs::read(&store).unwrap();
        begin_attempt(&store, first.attempt.as_ref().unwrap()).unwrap();
        assert_eq!(read_store(&store), first);
        assert_eq!(fs::read(&store).unwrap(), bytes);
    }

    #[test]
    fn every_phase_and_outcome_survives_a_round_trip() {
        let (_dir, store) = seeded_store();
        let phases = [
            Phase::Checking,
            Phase::Fetching,
            Phase::CheckingFallback { reason: FallbackReason::NoKey },
            Phase::CheckingFallback { reason: FallbackReason::RateLimited },
            Phase::CheckingFallback { reason: FallbackReason::NotVisible },
            Phase::CheckingFallback { reason: FallbackReason::Rejected },
            Phase::CheckingFallback { reason: FallbackReason::Other },
        ];
        for phase in phases {
            set_phase(&store, ID, phase, None).unwrap();
            assert_eq!(read_store(&store).attempt.unwrap().phase, phase);
        }
        let outcomes = [
            Outcome::InSync,
            Outcome::Fetched,
            Outcome::Absent,
            Outcome::FetchFailed { reason: FetchFailure::Timeout },
            Outcome::FetchFailed { reason: FetchFailure::Other },
            Outcome::CheckFailed { reason: CheckFailure::Timeout },
            Outcome::CheckFailed { reason: CheckFailure::Credentials },
            Outcome::CheckFailed { reason: CheckFailure::Other },
            Outcome::Unavailable { reason: UnavailableReason::OriginChanged },
            Outcome::Unavailable { reason: UnavailableReason::BranchChanged },
        ];
        for outcome in outcomes {
            finish_attempt(&store, ID, outcome, None).unwrap();
            assert_eq!(read_store(&store).attempt.unwrap().outcome, Some(outcome));
        }
        let conditions = [
            ApiCondition::CredentialsRequired,
            ApiCondition::CredentialsRejected,
            ApiCondition::CredentialsInsufficient,
            ApiCondition::RateLimited { authenticated: true },
            ApiCondition::NotFoundOrNotPermitted,
        ];
        for condition in conditions {
            let note = ApiNote { condition, key: None, fallback_answered: false };
            set_phase(&store, ID, Phase::Checking, Some(note.clone())).unwrap();
            assert_eq!(read_store(&store).attempt.unwrap().api, Some(note));
        }
        for source in [AnswerSource::Api, AnswerSource::Git, AnswerSource::Fetch] {
            let published = answer(None, NOW, source);
            publish_answer(&store, &published).unwrap();
            assert_eq!(read_store(&store).answer, Some(published));
        }
    }

    #[test]
    fn an_attempt_is_current_through_attempt_max_age_and_not_one_second_beyond() {
        let (_dir, store) = seeded_store();
        let origin = "https://heads.example.invalid/o/r.git";
        // `select_attempt` digests the URL; bind the seeded attempt to it.
        let bound = Attempt { origin_digest: origin_digest(origin), ..attempt(NOW) };
        begin_attempt(&store, &bound).unwrap();
        let before = fs::read(&store).unwrap();
        let max = ATTEMPT_MAX_AGE.as_secs();
        assert_eq!(max, 75);

        for (now, current) in [(NOW, true), (NOW + max, true), (NOW + max + 1, false), (NOW - 1, false)] {
            assert_eq!(select_attempt(&store, Some(origin), Some("main"), now).is_some(), current, "at {now}");
        }
        assert_eq!(select_attempt(&store, Some("https://heads.example.invalid/o/other.git"), Some("main"), NOW), None);
        assert_eq!(select_attempt(&store, Some(origin), Some("trunk"), NOW), None);
        assert_eq!(select_attempt(&store, None, Some("main"), NOW), None);
        assert_eq!(select_attempt(&store, Some(origin), None, NOW), None);

        // Discarding is a reading: the answer and the bytes stay.
        assert_eq!(fs::read(&store).unwrap(), before);
        assert_eq!(
            read_store(&store),
            StoreState { answer: Some(answer(Some(SHA), NOW, AnswerSource::Git)), attempt: Some(bound) }
        );
    }

    #[test]
    fn an_invalid_attempt_field_drops_the_attempt_and_keeps_the_answer() {
        let (_dir, store) = seeded_store();
        let valid = raw_json(&store);
        let expected_answer = Some(answer(Some(SHA), NOW, AnswerSource::Git));
        let with = |key: &str, value: serde_json::Value| {
            let mut document = valid.clone();
            document["attempt"][key] = value;
            document
        };
        let without = |key: &str| {
            let mut document = valid.clone();
            document["attempt"].as_object_mut().unwrap().remove(key);
            document
        };
        for document in [
            with("id", ID[..31].into()),
            with("id", format!("{ID}0").into()),
            with("id", ID.to_uppercase().into()),
            with("id", "../../etc/passwd-0123456789abcdef".into()),
            with("origin_digest", "".into()),
            with("branch", "".into()),
            with("started_at", "now".into()),
            with("started_at", (-1).into()),
            with("phase", "checking".into()),
            with("phase", serde_json::json!({ "kind": "waiting" })),
            with("phase", serde_json::json!({ "kind": "checking-fallback" })),
            with("phase", serde_json::json!({ "kind": "checking-fallback", "reason": "bored" })),
            with("outcome", serde_json::json!({ "kind": "fetch-failed", "reason": "credentials" })),
            with("outcome", serde_json::json!({ "kind": "unavailable" })),
            with("outcome", serde_json::json!({ "kind": "done" })),
            with("api", serde_json::json!({ "condition": { "kind": "rate-limited" }, "key": null, "fallback_answered": false })),
            with("api", serde_json::json!({ "condition": { "kind": "credentials-required" }, "key": "ghp_x y", "fallback_answered": false })),
            with("api", serde_json::json!({ "condition": { "kind": "credentials-required" }, "key": "1TOKEN", "fallback_answered": false })),
            with("api", serde_json::json!({ "condition": { "kind": "credentials-required" }, "fallback_answered": false })),
            without("id"),
            without("phase"),
            without("outcome"),
            without("api"),
            without("started_at"),
        ] {
            write_raw(&store, &document);
            assert_eq!(
                read_store(&store),
                StoreState { answer: expected_answer.clone(), attempt: None },
                "{document}"
            );
        }
        // A key that names a variable, and no note at all, are both valid.
        write_raw(&store, &with("api", serde_json::Value::Null));
        assert!(read_store(&store).attempt.is_some());
    }

    #[test]
    fn an_invalid_answer_drops_the_answer_and_keeps_the_attempt() {
        let (_dir, store) = seeded_store();
        let valid = raw_json(&store);
        let with = |key: &str, value: serde_json::Value| {
            let mut document = valid.clone();
            document["answer"][key] = value;
            document
        };
        let mut without_sha = valid.clone();
        without_sha["answer"].as_object_mut().unwrap().remove("sha");
        for document in [
            with("sha", SHA[..39].into()),
            with("sha", SHA.to_uppercase().into()),
            with("branch", "".into()),
            with("origin_digest", "".into()),
            with("source", "cache".into()),
            with("checked_at", "yesterday".into()),
            without_sha,
        ] {
            write_raw(&store, &document);
            assert_eq!(read_store(&store), StoreState { answer: None, attempt: Some(attempt(NOW)) }, "{document}");
        }

        // Only an unreadable document or an unknown format loses both halves.
        for version in [serde_json::json!(3), serde_json::json!("2")] {
            let mut document = valid.clone();
            document["format_version"] = version;
            write_raw(&store, &document);
            assert_eq!(read_store(&store), StoreState::default(), "{document}");
        }
        let mut no_version = valid.clone();
        no_version.as_object_mut().unwrap().remove("format_version");
        write_raw(&store, &no_version);
        assert_eq!(read_store(&store), StoreState::default());
        fs::write(&store, b"{not json").unwrap();
        assert_eq!(read_store(&store), StoreState::default());
    }

    #[test]
    fn writers_keep_the_answer_while_the_attempt_moves() {
        let (_dir, store) = seeded_store();
        let kept = Some(answer(Some(SHA), NOW, AnswerSource::Git));

        set_phase(&store, ID, Phase::CheckingFallback { reason: FallbackReason::NoKey }, Some(note())).unwrap();
        assert_eq!(read_store(&store).answer, kept);
        set_phase(&store, ID, Phase::Fetching, None).unwrap();
        let state = read_store(&store);
        assert_eq!(state.answer, kept);
        let running = state.attempt.unwrap();
        assert_eq!((running.phase, running.outcome, running.api), (Phase::Fetching, None, Some(note())));

        finish_attempt(&store, ID, Outcome::Fetched, None).unwrap();
        let state = read_store(&store);
        assert_eq!(state.answer, kept);
        let finished = state.attempt.unwrap();
        assert_eq!(
            (finished.phase, finished.outcome, finished.api),
            (Phase::Fetching, Some(Outcome::Fetched), Some(note())),
            "the phase stays as last reached and the note is kept"
        );

        // A new attempt replaces the old one and still keeps the answer.
        let next = Attempt::begin(OTHER_ID.into(), DIGEST.into(), "main".into(), NOW + 100);
        begin_attempt(&store, &next).unwrap();
        assert_eq!(read_store(&store), StoreState { answer: kept, attempt: Some(next) });
    }

    #[test]
    fn publishing_an_answer_keeps_the_attempt() {
        let (_dir, store) = seeded_store();
        set_phase(&store, ID, Phase::Fetching, None).unwrap();
        let running = read_store(&store).attempt;
        let fetched = answer(Some(OTHER_SHA), NOW + 3, AnswerSource::Fetch);
        publish_answer(&store, &fetched).unwrap();
        assert_eq!(read_store(&store), StoreState { answer: Some(fetched), attempt: running });

        // Into an empty store, too.
        let (_empty_dir, empty) = temp_store();
        let checked = answer(None, NOW, AnswerSource::Api);
        publish_answer(&empty, &checked).unwrap();
        assert_eq!(read_store(&empty), StoreState { answer: Some(checked), attempt: None });
    }

    #[test]
    fn a_failed_or_stale_attempt_never_replaces_the_answer() {
        let (_dir, store) = seeded_store();
        let kept = Some(answer(Some(SHA), NOW, AnswerSource::Git));
        for outcome in [
            Outcome::CheckFailed { reason: CheckFailure::Credentials },
            Outcome::FetchFailed { reason: FetchFailure::Other },
            Outcome::Unavailable { reason: UnavailableReason::OriginChanged },
        ] {
            finish_attempt(&store, ID, outcome, Some(note())).unwrap();
            assert_eq!(read_store(&store).answer, kept, "{outcome:?}");
        }

        // An abandoned attempt is discarded on read; the answer is still shown.
        let origin = "https://heads.example.invalid/o/r.git";
        let answered = Answer { origin_digest: origin_digest(origin), ..answer(Some(SHA), NOW, AnswerSource::Git) };
        publish_answer(&store, &answered).unwrap();
        begin_attempt(&store, &Attempt { origin_digest: origin_digest(origin), ..attempt(NOW) }).unwrap();
        let late = NOW + ATTEMPT_MAX_AGE.as_secs() + 1;
        assert_eq!(select_attempt(&store, Some(origin), Some("main"), late), None);
        assert_eq!(
            select_cached_head(&store, Some(origin), Some("main"), late),
            CachedRemoteHead::Stale(head(Some(SHA), NOW))
        );
    }

    #[test]
    fn writers_refuse_another_attempt_and_invalid_values_without_writing() {
        let (_dir, store) = seeded_store();
        let before = fs::read(&store).unwrap();
        assert!(set_phase(&store, OTHER_ID, Phase::Fetching, None).is_err());
        assert!(finish_attempt(&store, OTHER_ID, Outcome::InSync, None).is_err());
        let bad_note = ApiNote { key: Some("ghp_secret value".into()), ..note() };
        assert!(set_phase(&store, ID, Phase::Fetching, Some(bad_note)).is_err());
        assert!(begin_attempt(&store, &Attempt { id: "not-an-id".into(), ..attempt(NOW) }).is_err());
        assert!(publish_answer(&store, &answer(Some("abc"), NOW, AnswerSource::Git)).is_err());
        assert_eq!(fs::read(&store).unwrap(), before);

        let (_empty_dir, empty) = temp_store();
        assert!(set_phase(&empty, ID, Phase::Fetching, None).is_err());
        assert!(!empty.exists());
    }

    #[test]
    fn a_refresh_keeps_the_stored_attempt() {
        let repo = TestRepo::with_origin();
        let running = Attempt::begin(ID.into(), origin_digest(&origin(&repo)), "main".into(), NOW);
        begin_attempt(&store(&repo), &running).unwrap();
        seed(&repo, Some(SHA), NOW);
        let state = read_store(&store(&repo));
        assert_eq!(state.attempt, Some(running));
        assert_eq!(state.answer.map(|answer| answer.source), Some(AnswerSource::Git));
    }

    #[test]
    fn attempt_ids_are_32_random_hex_characters() {
        let first = new_attempt_id().unwrap();
        let second = new_attempt_id().unwrap();
        assert!(is_attempt_id(&first) && is_attempt_id(&second), "{first} {second}");
        assert_ne!(first, second);
    }

    fn receipt() -> Receipt {
        Receipt {
            attempt_id: ID.into(),
            origin_digest: DIGEST.into(),
            branch: "main".into(),
            finished_at: NOW + 30,
            head: HeadStatus::Ok,
            prs: PrStatus::Failed { failure: PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) } },
        }
    }

    #[test]
    fn a_receipt_round_trips_with_its_documented_spellings() {
        let (dir, _) = temp_store();
        let path = dir.path().join("abc.refresh-receipt.json");
        write_receipt(&path, &receipt()).unwrap();
        assert_eq!(
            raw_json(&path),
            serde_json::json!({
                "format_version": 1,
                "attempt_id": ID, "origin_digest": DIGEST, "branch": "main", "finished_at": NOW + 30,
                "head": "ok",
                "prs": { "kind": "failed", "failure": { "kind": "credentials-rejected", "key": "GITHUB_TOKEN" } },
            })
        );
        assert_eq!(load_receipt(&path, &attempt(NOW)), Some(receipt()));

        let heads = [HeadStatus::Ok, HeadStatus::Failed, HeadStatus::AdoptedElsewhere];
        let failures = [
            PrFailure::CredentialsRequired,
            PrFailure::CredentialsRejected { key: None },
            PrFailure::CredentialsInsufficient { key: Some("GH_TOKEN".into()) },
            PrFailure::RateLimited { authenticated: true, key: Some("GITHUB_TOKEN".into()) },
            PrFailure::NotFoundOrNotPermitted,
            PrFailure::Other,
        ];
        let prs = [PrStatus::Ok, PrStatus::SkippedFresh, PrStatus::Contended]
            .into_iter()
            .chain(failures.into_iter().map(|failure| PrStatus::Failed { failure }));
        for (head, prs) in heads.into_iter().cycle().zip(prs) {
            let written = Receipt { head, prs, ..receipt() };
            write_receipt(&path, &written).unwrap();
            assert_eq!(load_receipt(&path, &attempt(NOW)), Some(written));
        }
    }

    #[test]
    fn a_receipt_for_another_attempt_or_older_than_the_attempt_is_ignored() {
        let (dir, _) = temp_store();
        let path = dir.path().join("abc.refresh-receipt.json");
        write_receipt(&path, &receipt()).unwrap();
        let ours = attempt(NOW);
        assert!(load_receipt(&path, &ours).is_some(), "control");

        for other in [
            Attempt { id: OTHER_ID.into(), ..ours.clone() },
            Attempt { origin_digest: "another-digest".into(), ..ours.clone() },
            Attempt { branch: "trunk".into(), ..ours.clone() },
            Attempt { started_at: NOW + 31, ..ours.clone() },
        ] {
            assert_eq!(load_receipt(&path, &other), None, "{other:?}");
        }
        // Finishing in the second it started is not older.
        assert!(load_receipt(&path, &Attempt { started_at: NOW + 30, ..ours }).is_some());
    }

    #[test]
    fn a_malformed_or_missing_receipt_is_ignored() {
        let (dir, _) = temp_store();
        let path = dir.path().join("abc.refresh-receipt.json");
        assert_eq!(load_receipt(&path, &attempt(NOW)), None);

        write_receipt(&path, &receipt()).unwrap();
        let valid = raw_json(&path);
        let with = |key: &str, value: serde_json::Value| {
            let mut document = valid.clone();
            document[key] = value;
            document
        };
        let without = |key: &str| {
            let mut document = valid.clone();
            document.as_object_mut().unwrap().remove(key);
            document
        };
        for document in [
            with("format_version", 2.into()),
            with("attempt_id", ID.to_uppercase().into()),
            with("branch", "".into()),
            with("finished_at", "later".into()),
            with("head", "moved".into()),
            with("prs", "ok".into()),
            with("prs", serde_json::json!({ "kind": "failed" })),
            with("prs", serde_json::json!({ "kind": "failed", "failure": { "kind": "credentials-rejected", "key": "a token" } })),
            without("format_version"),
            without("head"),
            without("prs"),
        ] {
            write_raw(&path, &document);
            assert_eq!(load_receipt(&path, &attempt(NOW)), None, "{document}");
        }
        fs::write(&path, b"{not json").unwrap();
        assert_eq!(load_receipt(&path, &attempt(NOW)), None);

        let refused = Receipt { attempt_id: "short".into(), ..receipt() };
        assert!(write_receipt(&path, &refused).is_err());
    }
}
