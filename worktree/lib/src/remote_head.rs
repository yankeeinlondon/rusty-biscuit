//! The last live answer for `origin`'s default branch, for the `wt list`
//! caption's remote observation, and the state of the attempt refreshing it.
//!
//! `wt list` compares the local default branch with its local tracking ref
//! `origin/<default>`, which the worker fetches when `origin` differs. This
//! store records what `origin` last answered for `refs/heads/<default>` and
//! how the current attempt is going, so the listing can follow its own
//! attempt to an outcome and, when that attempt has no answer, say when
//! `origin` last answered. Listing only reads it; the detached
//! `wt internal-refresh` worker writes it ([`crate::remote_update`]).
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
//! - Every attempt's completion receipt ([`Receipt`]) is its own file,
//!   `<repo hash>.refresh-receipt.<attempt id>.json`, so overlapping runs
//!   never replace each other's. Its run deletes it once the wait ends; one
//!   left behind (a run that timed out) is deleted by the next worker once
//!   older than [`ATTEMPT_MAX_AGE`] ([`remove_stale_receipts`]).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::cache::{atomic_write, repo_cache_file, try_lock_sidecar};
use crate::error::WorktreeError;
use crate::live_remote::is_object_id;
use crate::pull_requests::{FRESHNESS_WINDOW, origin_digest};

pub const REMOTE_HEAD_FORMAT_VERSION: u32 = 2;

/// The bare-answer format, still read so an upgrade keeps its evidence.
const LEGACY_FORMAT_VERSION: u64 = 1;

pub const RECEIPT_FORMAT_VERSION: u32 = 1;

/// The budget for one check of the live head: the provider API call and any
/// `git ls-remote` fallback together (see [`crate::remote_update`]).
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

/// Whether another process holds the refresh lock of the store at `store`;
/// a lock file that cannot be opened reads as not held.
///
/// The probe takes the lock for an instant, so a worker that tries to take it
/// at that moment finds it contended and exits. Probe only a worker that is
/// already running, never one just launched.
pub fn refresh_lock_held(store: &Path) -> bool {
    matches!(try_lock_sidecar(&remote_head_lock_path(store)), Ok(None))
}

/// Attempt `attempt_id`'s completion receipt for the repository whose main
/// worktree is `repo_root`: `<repo hash>.refresh-receipt.<attempt id>.json`.
///
/// ## Errors
///
/// Fails for an id that is not 32 lowercase hex digits, which keeps the name
/// inside the cache directory.
pub fn refresh_receipt_path(repo_root: &Path, attempt_id: &str) -> Result<PathBuf, WorktreeError> {
    receipt_path_beside(&remote_head_store_path(repo_root)?, attempt_id)
}

/// As [`refresh_receipt_path`], beside the remote-head store at `store`
/// (`<repo hash>.remote-head.json`, or `remote-head.json` in tests).
pub fn receipt_path_beside(store: &Path, attempt_id: &str) -> Result<PathBuf, WorktreeError> {
    if !is_attempt_id(attempt_id) {
        return Err(invalid("a receipt for an invalid attempt id"));
    }
    let name = store.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    let prefix = name.strip_suffix("remote-head.json").unwrap_or_default();
    Ok(store.with_file_name(format!("{prefix}{RECEIPT_INFIX}{attempt_id}.json")))
}

/// Between the repository prefix and the attempt id in a receipt's name.
const RECEIPT_INFIX: &str = "refresh-receipt.";

/// Deletes this repository's receipts, beside `receipt`, last modified more
/// than [`ATTEMPT_MAX_AGE`] before `now`; best effort.
///
/// No run waits on such a receipt: no wait lasts longer than
/// [`ATTEMPT_MAX_AGE`] from its launch, and its worker writes the receipt
/// after that launch. Only names of the form
/// `<repo prefix>refresh-receipt.<anything>.json` are touched.
pub fn remove_stale_receipts(receipt: &Path, now: SystemTime) {
    let (Some(dir), Some(name)) = (receipt.parent(), receipt.file_name().and_then(|name| name.to_str())) else {
        return;
    };
    let Some(at) = name.find(RECEIPT_INFIX) else {
        return;
    };
    let prefix = &name[..at + RECEIPT_INFIX.len()];
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        if !file_name.starts_with(prefix) || !file_name.ends_with(".json") {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|modified| now.duration_since(modified).is_ok_and(|age| age > ATTEMPT_MAX_AGE));
        if stale {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// A fresh attempt id: 128 bits from the OS random source, the same generator
/// as `wt remove`'s hand-off tokens.
pub fn new_attempt_id() -> Result<String, WorktreeError> {
    crate::remove::handoff::new_token()
}

pub(crate) fn is_attempt_id(id: &str) -> bool {
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

/// An `Option` field that must be spelled out: `null` is `None`, while a
/// missing key is an error rather than serde's silent `None`.
pub(crate) fn present<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}

/// A PR request's failure, typed so this run's failure can produce a spec §5
/// line: a mirror of sniff's credentials variants plus `Other`. It lives here
/// for the receipt; `pull_requests::refresh` reports it.
/// Serialized as `{"kind": "credentials-rejected", "key": "GITHUB_TOKEN"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PrFailure {
    CredentialsRequired,
    CredentialsRejected {
        /// The name of the variable used; never its value.
        #[serde(deserialize_with = "present")]
        key: Option<String>,
    },
    CredentialsInsufficient {
        #[serde(deserialize_with = "present")]
        key: Option<String>,
    },
    RateLimited {
        authenticated: bool,
        #[serde(deserialize_with = "present")]
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

/// How the live-head half of a refresh attempt ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeadStatus {
    Ok,
    Failed,
    /// Another worker held the lock; its attempt carries the result.
    AdoptedElsewhere,
}

/// How the PR half of a refresh attempt ended. Serialized as
/// `{"kind": "ok"}`, `{"kind": "failed", "failure": {..}}`, and so on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PrStatus {
    Ok,
    Failed { failure: PrFailure },
    /// The repository is in `~/.wt.json`, so no PR request was made.
    Ignored,
    /// `origin` has no provider to ask (a local path or an unsupported
    /// host), so no PR request was made. Not a failure.
    Unsupported,
    /// Another worker held the PR lock.
    Contended,
}

/// Written by the worker after both halves of every attempt have finished.
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
                PrStatus::Ok | PrStatus::Ignored | PrStatus::Unsupported | PrStatus::Contended => true,
            }
    }
}

/// Publishes `receipt` at `path` ([`refresh_receipt_path`] for its attempt)
/// atomically.
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
    use super::*;
    use crate::pull_requests::origin_url;
    use crate::remove::test_support::TestRepo;

    const NOW: u64 = 1_790_000_000;
    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    const OTHER_SHA: &str = "89abcdef0123456789abcdef0123456789abcdef";

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
        let answer = Answer {
            origin_digest: origin_digest(&origin(repo)),
            branch: "main".into(),
            sha: sha.map(str::to_string),
            checked_at: at,
            source: AnswerSource::Git,
        };
        publish_answer(&store(repo), &answer).unwrap();
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
    fn the_lock_probe_sees_a_holder_and_releases_its_own_hold() {
        let (_dir, store) = temp_store();
        assert!(!refresh_lock_held(&store), "no holder");
        assert!(!refresh_lock_held(&store), "the probe released its own hold");

        let holder = try_lock_sidecar(&remote_head_lock_path(&store)).unwrap().expect("free");
        assert!(refresh_lock_held(&store));
        drop(holder);
        assert!(!refresh_lock_held(&store));
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
        let receipt = refresh_receipt_path(dir.path(), ID).unwrap();
        assert_eq!(receipt.file_name().unwrap().to_string_lossy(), format!("{stem}.refresh-receipt.{ID}.json"));
        assert_eq!(cache.parent(), receipt.parent());
        assert_eq!(receipt_path_beside(&store, ID).unwrap(), receipt);
        assert!(refresh_receipt_path(dir.path(), "../../escape").is_err());
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
        let prs = [PrStatus::Ok, PrStatus::Ignored, PrStatus::Unsupported, PrStatus::Contended]
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

    /// Two overlapping forced attempts each keep their own receipt.
    #[test]
    fn receipts_of_different_attempts_never_replace_each_other() {
        let (_dir, store) = temp_store();
        let ours = receipt_path_beside(&store, ID).unwrap();
        let theirs = receipt_path_beside(&store, OTHER_ID).unwrap();
        assert_ne!(ours, theirs);
        write_receipt(&ours, &receipt()).unwrap();
        let other = Receipt { attempt_id: OTHER_ID.into(), prs: PrStatus::Ok, ..receipt() };
        write_receipt(&theirs, &other).unwrap();

        assert_eq!(load_receipt(&ours, &attempt(NOW)), Some(receipt()));
        assert_eq!(load_receipt(&theirs, &Attempt { id: OTHER_ID.into(), ..attempt(NOW) }), Some(other));
    }

    #[test]
    fn only_this_repositorys_receipts_older_than_an_attempt_are_swept() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("abc.remote-head.json");
        let now = SystemTime::now();
        let aged = |path: &Path, age: Duration| {
            fs::write(path, b"{}").unwrap();
            let file = fs::OpenOptions::new().write(true).open(path).unwrap();
            file.set_modified(now - age).unwrap();
        };
        let stale = receipt_path_beside(&store, ID).unwrap();
        let fresh = receipt_path_beside(&store, OTHER_ID).unwrap();
        let legacy = dir.path().join("abc.refresh-receipt.json");
        let other_repo = dir.path().join(format!("xyz.refresh-receipt.{ID}.json"));
        let unrelated = dir.path().join("abc.prs.json");
        let over = ATTEMPT_MAX_AGE + Duration::from_secs(1);
        aged(&stale, over);
        aged(&fresh, ATTEMPT_MAX_AGE - Duration::from_secs(1));
        aged(&legacy, over);
        aged(&other_repo, over);
        aged(&unrelated, over);

        remove_stale_receipts(&fresh, now);

        assert!(!stale.exists(), "older than any wait");
        assert!(!legacy.exists(), "the old single receipt goes too");
        assert!(fresh.exists(), "a run may still be waiting for it");
        assert!(other_repo.exists() && unrelated.exists(), "never another repository's or another store");
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

    /// The Input Robustness Matrix for the receipt: every load-bearing field
    /// in every shape, one edit per row, from a file written by
    /// [`write_receipt`]. Every row is a missing receipt, never a default
    /// outcome.
    #[test]
    fn the_receipt_reader_walks_the_input_robustness_matrix() {
        use serde_json::{Value, json};
        let (dir, _) = temp_store();
        let path = dir.path().join("abc.refresh-receipt.json");
        write_receipt(&path, &receipt()).unwrap();
        let written = raw_json(&path);
        let read = |bytes: &[u8]| {
            fs::write(&path, bytes).unwrap();
            load_receipt(&path, &attempt(NOW))
        };
        assert_eq!(read(&serde_json::to_vec(&written).unwrap()), Some(receipt()), "control");

        let set = |pointer: &str, value: Option<Value>| {
            let mut document = written.clone();
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            let target = if parent.is_empty() { &mut document } else { document.pointer_mut(parent).unwrap() };
            let map = target.as_object_mut().unwrap();
            match value {
                Some(value) => {
                    map.insert(key.to_string(), value);
                }
                None => assert!(map.remove(key).is_some(), "{pointer} exists"),
            }
            serde_json::to_vec(&document).unwrap()
        };
        let compact = serde_json::to_string(&written).unwrap();
        let prefixed = |member: &str| compact.replacen('{', &format!("{{{member},"), 1).into_bytes();

        let mut misses = Vec::new();
        for field in ["/format_version", "/attempt_id", "/origin_digest", "/branch", "/finished_at", "/head", "/prs"] {
            misses.push(set(field, None));
            misses.push(set(field, Some(Value::Null)));
            misses.push(set(field, Some(json!([]))));
            misses.push(set(field, Some(json!({}))));
        }
        for (field, wrong) in [
            ("/format_version", json!("1")),
            ("/format_version", json!(2)),
            ("/attempt_id", json!(1)),
            ("/attempt_id", json!("")),
            ("/attempt_id", json!(OTHER_ID)),
            ("/origin_digest", json!(1)),
            ("/origin_digest", json!("")),
            ("/origin_digest", json!("another-digest")),
            ("/branch", json!(1)),
            ("/branch", json!("")),
            ("/branch", json!("trunk")),
            ("/finished_at", json!("later")),
            ("/finished_at", json!(-1)),
            ("/finished_at", json!(NOW - 1)),
            ("/head", json!(1)),
            ("/head", json!("")),
            ("/head", json!("moved")),
            ("/prs", json!("failed")),
            ("/prs", json!({ "kind": "skipped-fresh" })),
            ("/prs", json!({ "kind": "" })),
            ("/prs", json!({ "kind": 1 })),
            ("/prs/failure", Value::Null),
            ("/prs/failure", json!("other")),
            ("/prs/failure/kind", json!("credentials-lost")),
            ("/prs/failure/key", json!(1)),
            ("/prs/failure/key", json!("a token")),
            ("/prs/failure/key", json!("")),
        ] {
            misses.push(set(field, Some(wrong)));
        }
        for field in ["/prs/kind", "/prs/failure", "/prs/failure/kind", "/prs/failure/key"] {
            misses.push(set(field, None));
        }
        for member in [
            "\"format_version\":1".to_string(),
            format!("\"attempt_id\":\"{ID}\""),
            format!("\"origin_digest\":\"{DIGEST}\""),
            "\"branch\":\"main\"".to_string(),
            format!("\"finished_at\":{}", NOW + 30),
            "\"head\":\"ok\"".to_string(),
            "\"prs\":{\"kind\":\"ok\"}".to_string(),
        ] {
            misses.push(prefixed(&member));
        }
        misses.push(compact.replacen("\"kind\":\"failed\"", "\"kind\":\"failed\",\"kind\":\"ok\"", 1).into_bytes());
        misses.push(compact.replacen("\"key\":", "\"key\":\"GH_TOKEN\",\"key\":", 1).into_bytes());
        misses.push(format!("{compact}garbage").into_bytes());
        misses.push(format!("{compact}{{}}").into_bytes());
        for bytes in misses {
            assert_eq!(read(&bytes), None, "{}", String::from_utf8_lossy(&bytes));
        }

        // A null key is a failure that names no variable, not a miss.
        let unnamed = read(&set("/prs/failure/key", Some(Value::Null)));
        assert_eq!(
            unnamed.map(|receipt| receipt.prs),
            Some(PrStatus::Failed { failure: PrFailure::CredentialsRejected { key: None } })
        );
        // `ok` with no store publication is still the receipt's answer.
        assert_eq!(read(&set("/prs", Some(json!({ "kind": "ok" })))).map(|receipt| receipt.prs), Some(PrStatus::Ok));
    }
}
