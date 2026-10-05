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
//!   (see [`crate::cache::repo_cache_file`]). Format 3 holds two independent
//!   halves, `answer` and `attempt`, each bound to the [`origin_digest`] of
//!   `origin`'s exact URL and to the default branch. The URL itself is never
//!   stored or logged. A format-1 file (a bare answer) reads as an `answer`
//!   from [`AnswerSource::Git`] with no attempt. A format-2 file keeps its
//!   `answer` and loses its `attempt`, which cannot say which credentials its
//!   API check was sent with ([`Attempt::credentials`]).
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
//!   never replace each other's. Its run tries once to delete it as the wait
//!   ends; one written after that (the run timed out, or succeeded before
//!   its worker wrote the receipt) is deleted by the next worker once
//!   older than [`ATTEMPT_MAX_AGE`] ([`remove_stale_receipts`]).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cache::{atomic_write, repo_cache_file, try_lock_sidecar};
use crate::error::WorktreeError;
use crate::live_remote::is_object_id;
use crate::pull_requests::{FRESHNESS_WINDOW, origin_digest};
use crate::strict_json;
use crate::timing::{LaunchReport, WorkerTimings};

/// Format 3 added [`Attempt::credentials`].
pub const REMOTE_HEAD_FORMAT_VERSION: u32 = 3;

/// The bare-answer format, still read so an upgrade keeps its evidence.
const LEGACY_FORMAT_VERSION: u64 = 1;

/// The format before attempt credentials: its answer is kept, its attempt
/// dropped.
const ATTEMPTLESS_FORMAT_VERSION: u64 = 2;

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

/// Which credentials a successful provider API request was sent with, as the
/// worker recorded it from sniff's own selection for that request.
///
/// Serialized as `{"state": "anonymous"}`,
/// `{"state": "keyed", "variables": ["GITHUB_TOKEN"]}`, or
/// `{"state": "unknown"}`. Only names are stored, never token values. Only
/// [`CredentialEvidence::Anonymous`] may ever be shown as a keyless answer:
/// `unknown` is what a record that predates the evidence, or an attempt
/// whose API check did not succeed, carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum CredentialEvidence {
    /// Every request was sent without a token.
    Anonymous,
    /// At least one request carried a token from these variables (never
    /// empty).
    Keyed { variables: Vec<String> },
    /// Nothing is known about what was sent.
    Unknown,
}

impl CredentialEvidence {
    /// What sniff reported for a successful lookup.
    pub fn from_sniff(credentials: &sniff::remote::blocking::RequestCredentials) -> Self {
        use sniff::remote::blocking::RequestCredentials;
        match credentials {
            RequestCredentials::Anonymous => Self::Anonymous,
            RequestCredentials::Keyed { variables } => Self::Keyed { variables: variables.clone() },
            // `Unknown`, and any state a later sniff adds: nothing to claim.
            _ => Self::Unknown,
        }
    }

    /// A `keyed` state names at least one variable, and every name is an
    /// upper-case variable name, as every variable sniff reads is, so nothing
    /// shaped like a token (`ghp_…` is mixed case) is ever stored.
    pub(crate) fn is_valid(&self) -> bool {
        let credential_variable = |name: &str| is_variable_name(name) && !name.bytes().any(|b| b.is_ascii_lowercase());
        match self {
            Self::Keyed { variables } => !variables.is_empty() && variables.iter().all(|name| credential_variable(name)),
            Self::Anonymous | Self::Unknown => true,
        }
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
    /// The credentials of the check's successful API request, recorded as
    /// it succeeds and kept through the fetch, its failure, or its timeout.
    /// [`CredentialEvidence::Unknown`] until then, and for a check the API
    /// did not answer. Required: an attempt without it is dropped, never
    /// read as anonymous.
    #[serde(deserialize_with = "strict_json::nested")]
    pub credentials: CredentialEvidence,
}

impl Attempt {
    /// A new attempt in phase [`Phase::Checking`], with no outcome yet.
    pub fn begin(id: String, origin_digest: String, branch: String, started_at: u64) -> Self {
        Self {
            id,
            origin_digest,
            branch,
            started_at,
            phase: Phase::Checking,
            outcome: None,
            api: None,
            credentials: CredentialEvidence::Unknown,
        }
    }

    fn is_valid(&self) -> bool {
        is_attempt_id(&self.id)
            && !self.origin_digest.is_empty()
            && !self.branch.is_empty()
            && self.api.as_ref().is_none_or(ApiNote::is_valid)
            && self.credentials.is_valid()
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
/// unreadable file, another format, or a repeated top-level key is empty, and
/// a half that repeats a key is invalid ([`crate::strict_json`]).
pub fn read_store(store: &Path) -> StoreState {
    let Some(document) = fs::read(store).ok().and_then(|bytes| strict_json::members(&bytes)) else {
        return StoreState::default();
    };
    match document.get::<u64>("format_version") {
        Some(LEGACY_FORMAT_VERSION) => StoreState { answer: document.into_value().and_then(legacy_answer), attempt: None },
        Some(ATTEMPTLESS_FORMAT_VERSION) => {
            StoreState { answer: document.get::<Answer>("answer").filter(Answer::is_valid), attempt: None }
        }
        Some(version) if version == u64::from(REMOTE_HEAD_FORMAT_VERSION) => StoreState {
            answer: document.get::<Answer>("answer").filter(Answer::is_valid),
            attempt: document.get::<Attempt>("attempt").filter(Attempt::is_valid),
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

/// Records the credentials attempt `id`'s successful API check was sent
/// with. Every later write of the attempt keeps them. Call with the lock
/// held.
///
/// ## Errors
///
/// Fails without writing when the stored attempt is not `id`, or
/// `credentials` holds anything but variable names.
pub fn set_credentials(store: &Path, id: &str, credentials: CredentialEvidence) -> Result<(), WorktreeError> {
    if !credentials.is_valid() {
        return Err(invalid("invalid request credentials"));
    }
    update_attempt(store, id, None, |attempt| attempt.credentials = credentials)
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
    Failed {
        #[serde(deserialize_with = "strict_json::nested")]
        failure: PrFailure,
    },
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
    /// The worker's own timings, only for an attempt launched with timing
    /// on. Read after the outcome is accepted and never part of it: absent
    /// is [`LaunchReport::Missing`], and anything unreadable is
    /// [`LaunchReport::Invalid`] beside an intact outcome.
    #[serde(skip)]
    pub durations: LaunchReport,
}

#[derive(Serialize)]
struct ReceiptFileOut<'a> {
    format_version: u32,
    #[serde(flatten)]
    receipt: &'a Receipt,
    // The optional `durations` member: an additive field of format 1, which
    // older readers ignore.
    #[serde(skip_serializing_if = "Option::is_none")]
    durations: Option<Value>,
}

// Spelled out rather than `#[serde(flatten)]`: flattening buffers the input,
// and serde's buffer reads an integer `kind` as a variant index, so
// `"prs": {"kind": 1}` was a failure never recorded.
#[derive(Deserialize)]
struct ReceiptFileIn {
    format_version: u32,
    attempt_id: String,
    origin_digest: String,
    branch: String,
    finished_at: u64,
    head: HeadStatus,
    prs: PrStatus,
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
    let durations = receipt.durations.timings().map(WorkerTimings::to_value);
    write_json(path, &ReceiptFileOut { format_version: RECEIPT_FORMAT_VERSION, receipt, durations })
}

/// The receipt at `path` for `attempt`: `None` when missing, unreadable,
/// invalid, for another attempt id, origin, or branch, or finished before
/// the attempt started.
///
/// The `durations` member is decoded only once the outcome is accepted, and
/// its state never decides whether the receipt is ([`Receipt::durations`]).
pub fn load_receipt(path: &Path, attempt: &Attempt) -> Option<Receipt> {
    let bytes = fs::read(path).ok()?;
    let file = serde_json::from_slice::<ReceiptFileIn>(&bytes).ok()?;
    let receipt = Receipt {
        attempt_id: file.attempt_id,
        origin_digest: file.origin_digest,
        branch: file.branch,
        finished_at: file.finished_at,
        head: file.head,
        prs: file.prs,
        durations: LaunchReport::Missing,
    };
    let accepted = file.format_version == RECEIPT_FORMAT_VERSION
        && receipt.is_valid()
        && receipt.attempt_id == attempt.id
        && receipt.origin_digest == attempt.origin_digest
        && receipt.branch == attempt.branch
        && receipt.finished_at >= attempt.started_at;
    accepted.then(|| Receipt { durations: receipt_durations(&bytes), ..receipt })
}

/// The report in an accepted receipt's `durations` member.
fn receipt_durations(bytes: &[u8]) -> LaunchReport {
    // The outcome reader ignores unknown members, so a repeated
    // `durations` key reaches this point; it is never read as last-wins.
    let Some(members) = strict_json::members(bytes) else {
        return LaunchReport::Invalid;
    };
    if !members.contains("durations") {
        return LaunchReport::Missing;
    }
    match members.get::<Value>("durations").map(WorkerTimings::from_value) {
        Some(Ok(timings)) => LaunchReport::from_worker(timings),
        Some(Err(_)) | None => LaunchReport::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pull_requests::origin_url;
    use crate::remove::test_support::TestRepo;
    use crate::timing::{Stage, worker_fixture};

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
        assert_eq!(raw_json(&store(&repo))["format_version"], 3);
        assert_eq!(read_store(&store(&repo)), StoreState { answer: Some(expected), attempt: Some(started) });
        assert_eq!(select(&repo, NOW + 1), CachedRemoteHead::Fresh(head(Some(SHA), NOW)));
    }

    #[test]
    fn a_format_2_file_keeps_its_answer_and_drops_its_attempt() {
        let (_dir, store) = seeded_store();
        let mut format_2 = raw_json(&store);
        format_2["format_version"] = serde_json::json!(2);
        format_2["attempt"].as_object_mut().unwrap().remove("credentials");
        write_raw(&store, &format_2);
        let expected = read_store(&store);
        assert_eq!(expected.attempt, None, "a format-2 attempt cannot say what its check was sent with");
        assert_eq!(expected.answer, Some(answer(Some(SHA), NOW, AnswerSource::Git)), "its answer is kept");

        // With the field a format-3 writer adds, it is still format 2.
        format_2["attempt"]["credentials"] = serde_json::json!({ "state": "anonymous" });
        write_raw(&store, &format_2);
        assert_eq!(read_store(&store), expected);
    }

    #[test]
    fn credentials_survive_every_later_write_of_the_attempt_and_never_hold_a_value() {
        let (_dir, store) = seeded_store();
        let keyed = CredentialEvidence::Keyed { variables: vec!["SNIFF_GITHUB_GIT_2E_EXAMPLE_TOKEN".into()] };
        set_credentials(&store, ID, keyed.clone()).unwrap();
        set_phase(&store, ID, Phase::Fetching, None).unwrap();
        publish_answer(&store, &answer(Some(SHA), NOW + 1, AnswerSource::Fetch)).unwrap();
        finish_attempt(&store, ID, Outcome::FetchFailed { reason: FetchFailure::Timeout }, None).unwrap();
        assert_eq!(read_store(&store).attempt.map(|attempt| attempt.credentials), Some(keyed.clone()));

        let before = fs::read(&store).unwrap();
        for variables in [vec!["ghp_sentinelValueThatMustNeverBeStored".to_string()], vec!["A B".into()], Vec::new()] {
            assert!(set_credentials(&store, ID, CredentialEvidence::Keyed { variables }).is_err());
        }
        assert!(set_credentials(&store, OTHER_ID, CredentialEvidence::Anonymous).is_err(), "another attempt's");
        assert_eq!(fs::read(&store).unwrap(), before, "refused without writing");
    }

    #[test]
    fn the_store_round_trips_with_its_documented_spellings() {
        let (_dir, store) = seeded_store();
        set_credentials(&store, ID, CredentialEvidence::Keyed { variables: vec!["GH_TOKEN".into()] }).unwrap();
        set_phase(&store, ID, Phase::CheckingFallback { reason: FallbackReason::RateLimited }, Some(note())).unwrap();
        finish_attempt(&store, ID, Outcome::FetchFailed { reason: FetchFailure::Timeout }, None).unwrap();

        assert_eq!(
            raw_json(&store),
            serde_json::json!({
                "format_version": 3,
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
                    "credentials": { "state": "keyed", "variables": ["GH_TOKEN"] },
                },
            })
        );
        set_credentials(&store, ID, CredentialEvidence::Anonymous).unwrap();
        assert_eq!(raw_json(&store)["attempt"]["credentials"], serde_json::json!({ "state": "anonymous" }));
        set_credentials(&store, ID, CredentialEvidence::Unknown).unwrap();
        assert_eq!(raw_json(&store)["attempt"]["credentials"], serde_json::json!({ "state": "unknown" }));

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
        for version in [serde_json::json!(4), serde_json::json!("3")] {
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

    /// What a store edit leaves of the attempt.
    enum AttemptExpect {
        Kept,
        Dropped,
        Changed(fn(&mut Attempt)),
    }

    /// The Input Robustness Matrix for the store: every load-bearing field of
    /// both halves in every shape, one edit per cell, from a file the real
    /// writers wrote, read through [`select_cached_head`] and
    /// [`select_attempt`]. An invalid half is dropped and the other kept.
    #[test]
    fn the_store_reader_walks_the_input_robustness_matrix() {
        use serde_json::{Value, json};
        use AttemptExpect::*;
        use JsonEdit::*;
        let origin = "https://heads.example.invalid/o/r.git";
        let digest = origin_digest(origin);
        let (_dir, store) = temp_store();
        let answered = Answer { origin_digest: digest.clone(), ..answer(Some(SHA), NOW, AnswerSource::Git) };
        publish_answer(&store, &answered).unwrap();
        begin_attempt(&store, &Attempt { origin_digest: digest.clone(), ..attempt(NOW) }).unwrap();
        set_credentials(&store, ID, CredentialEvidence::Anonymous).unwrap();
        set_phase(&store, ID, Phase::CheckingFallback { reason: FallbackReason::RateLimited }, Some(note())).unwrap();
        finish_attempt(&store, ID, Outcome::FetchFailed { reason: FetchFailure::Timeout }, None).unwrap();
        let written = raw_json(&store);
        let control = read_store(&store).attempt.expect("the written attempt");
        assert_eq!(control.credentials, CredentialEvidence::Anonymous, "control: the writers kept the credentials");
        let read = |bytes: &[u8]| {
            fs::write(&store, bytes).unwrap();
            (
                select_cached_head(&store, Some(origin), Some("main"), NOW + 1),
                select_attempt(&store, Some(origin), Some("main"), NOW + 1),
            )
        };
        let fresh = CachedRemoteHead::Fresh(head(Some(SHA), NOW));
        assert_eq!(read(&serde_json::to_vec(&written).unwrap()), (fresh.clone(), Some(control.clone())), "control");

        let set = |pointer: &str, value: Option<Value>| Set(pointer.to_string(), value);
        let mut cells: Vec<(JsonEdit, bool, AttemptExpect)> = Vec::new();
        // Every field: absent, `[]`, `{}`, a wrong whole type, and a repeated
        // key spoil the half that holds it.
        let fields: [(&str, Value); 23] = [
            ("/answer/origin_digest", json!(1)),
            ("/answer/branch", json!(1)),
            ("/answer/sha", json!(1)),
            ("/answer/checked_at", json!("1790000000")),
            ("/answer/source", json!(1)),
            ("/attempt/id", json!(1)),
            ("/attempt/origin_digest", json!(1)),
            ("/attempt/branch", json!(1)),
            ("/attempt/started_at", json!("1790000000")),
            ("/attempt/phase", json!("checking")),
            ("/attempt/phase/kind", json!(1)),
            ("/attempt/phase/reason", json!(1)),
            ("/attempt/outcome", json!("fetch-failed")),
            ("/attempt/outcome/kind", json!(3)),
            ("/attempt/outcome/reason", json!(0)),
            ("/attempt/api", json!("rate-limited")),
            ("/attempt/api/condition", json!("rate-limited")),
            ("/attempt/api/condition/kind", json!(3)),
            ("/attempt/api/condition/authenticated", json!("false")),
            ("/attempt/api/fallback_answered", json!("true")),
            ("/attempt/credentials", json!("anonymous")),
            ("/attempt/credentials/state", json!(0)),
            ("/attempt/credentials/state", json!("keyless")),
        ];
        for (pointer, wrong) in fields {
            let in_answer = pointer.starts_with("/answer");
            let spoil = |cells: &mut Vec<(JsonEdit, bool, AttemptExpect)>, edit| {
                cells.push((edit, !in_answer, if in_answer { Kept } else { Dropped }));
            };
            for shape in [None, Some(json!([])), Some(json!({})), Some(wrong)] {
                spoil(&mut cells, set(pointer, shape));
            }
            spoil(&mut cells, Dup(pointer.to_string()));
        }
        // Explicit null: a verified absence, a running attempt, no note, and a
        // note naming no variable are valid; null anywhere else spoils its
        // half.
        for pointer in [
            "/answer/origin_digest",
            "/answer/branch",
            "/answer/checked_at",
            "/answer/source",
            "/attempt/id",
            "/attempt/origin_digest",
            "/attempt/branch",
            "/attempt/started_at",
            "/attempt/phase",
            "/attempt/phase/kind",
            "/attempt/phase/reason",
            "/attempt/outcome/kind",
            "/attempt/outcome/reason",
            "/attempt/api/condition",
            "/attempt/api/condition/kind",
            "/attempt/api/condition/authenticated",
            "/attempt/api/fallback_answered",
            "/attempt/credentials",
            "/attempt/credentials/state",
        ] {
            let in_answer = pointer.starts_with("/answer");
            cells.push((set(pointer, Some(Value::Null)), !in_answer, if in_answer { Kept } else { Dropped }));
        }
        cells.push((set("/attempt/outcome", Some(Value::Null)), true, Changed(|attempt| attempt.outcome = None)));
        cells.push((set("/attempt/api", Some(Value::Null)), true, Changed(|attempt| attempt.api = None)));
        cells.push((set("/attempt/api/key", Some(Value::Null)), true, Changed(|attempt| attempt.api.as_mut().unwrap().key = None)));
        // `key`: anything but a variable name or null spoils the attempt.
        for shape in [None, Some(json!([])), Some(json!({})), Some(json!(1)), Some(json!("")), Some(json!("a token"))] {
            cells.push((set("/attempt/api/key", shape), true, Dropped));
        }
        cells.push((Dup("/attempt/api/key".into()), true, Dropped));
        // `credentials`: a bad shape drops the attempt and keeps the answer;
        // it is never read as anonymous.
        for shape in [
            json!({ "state": "keyed" }),
            json!({ "state": "keyed", "variables": null }),
            json!({ "state": "keyed", "variables": "GH_TOKEN" }),
            json!({ "state": "keyed", "variables": {} }),
            json!({ "state": "keyed", "variables": [] }),
            json!({ "state": "keyed", "variables": ["GH_TOKEN", 1] }),
            json!({ "state": "keyed", "variables": [1] }),
            json!({ "state": "keyed", "variables": [1, null] }),
            json!({ "state": "keyed", "variables": ["GH TOKEN"] }),
            json!({ "state": "keyed", "variables": [""] }),
            json!({ "state": "keyed", "variables": ["ghp_looksLikeAToken123"] }),
        ] {
            cells.push((set("/attempt/credentials", Some(shape)), true, Dropped));
        }
        cells.push((
            set("/attempt/credentials", Some(json!({ "state": "keyed", "variables": ["GH_TOKEN", "GITHUB_TOKEN"] }))),
            true,
            Changed(|attempt| {
                attempt.credentials = CredentialEvidence::Keyed { variables: vec!["GH_TOKEN".into(), "GITHUB_TOKEN".into()] };
            }),
        ));
        cells.push((
            set("/attempt/credentials", Some(json!({ "state": "unknown" }))),
            true,
            Changed(|attempt| attempt.credentials = CredentialEvidence::Unknown),
        ));
        cells.push((set("/attempt/credentials/extra", Some(json!(1))), true, Kept));
        // Format 2 keeps its answer and loses its attempt, credentials or not.
        cells.push((set("/format_version", Some(json!(2))), true, Dropped));
        // Empty strings: an empty binding or id is invalid.
        for pointer in ["/answer/origin_digest", "/answer/branch", "/answer/sha", "/answer/source"] {
            cells.push((set(pointer, Some(json!(""))), false, Kept));
        }
        for pointer in ["/attempt/id", "/attempt/origin_digest", "/attempt/branch", "/attempt/phase/kind", "/attempt/outcome/kind"] {
            cells.push((set(pointer, Some(json!(""))), true, Dropped));
        }
        // Stale, future, and misbound halves are not served.
        cells.push((set("/answer/checked_at", Some(json!(NOW + 2))), false, Kept));
        cells.push((set("/answer/origin_digest", Some(json!(origin_digest("https://heads.example.invalid/o/other.git")))), false, Kept));
        cells.push((set("/answer/branch", Some(json!("trunk"))), false, Kept));
        cells.push((set("/attempt/started_at", Some(json!(NOW + 2))), true, Dropped));
        cells.push((set("/attempt/started_at", Some(json!(NOW - ATTEMPT_MAX_AGE.as_secs()))), true, Dropped));
        cells.push((set("/attempt/origin_digest", Some(json!(origin_digest("https://heads.example.invalid/o/other.git")))), true, Dropped));
        cells.push((set("/attempt/branch", Some(json!("trunk"))), true, Dropped));
        // The envelope: either half absent or null is only that half.
        cells.push((set("/answer", None), false, Kept));
        cells.push((set("/answer", Some(Value::Null)), false, Kept));
        cells.push((set("/attempt", None), true, Dropped));
        cells.push((set("/attempt", Some(Value::Null)), true, Dropped));
        cells.push((set("/answer", Some(json!([]))), false, Kept));
        cells.push((set("/attempt", Some(json!(1))), true, Dropped));
        for shape in [None, Some(Value::Null), Some(json!("3")), Some(json!([])), Some(json!({})), Some(json!(4))] {
            cells.push((set("/format_version", shape), false, Dropped));
        }
        for pointer in ["/format_version", "/answer", "/attempt"] {
            cells.push((Dup(pointer.into()), false, Dropped));
        }
        // Unknown fields are ignored; trailing content loses both halves.
        cells.push((set("/extra", Some(json!(1))), true, Kept));
        cells.push((set("/answer/extra", Some(json!(1))), true, Kept));
        cells.push((set("/attempt/phase/extra", Some(json!(1))), true, Kept));
        cells.push((Append("garbage"), false, Dropped));
        cells.push((Append("{}"), false, Dropped));

        let mut wrong = Vec::new();
        for (edit, answer_kept, attempt) in &cells {
            let bytes = edit.apply(&written);
            let want_attempt = match attempt {
                Kept => Some(control.clone()),
                Dropped => None,
                Changed(change) => {
                    let mut changed = control.clone();
                    change(&mut changed);
                    Some(changed)
                }
            };
            let want = (if *answer_kept { fresh.clone() } else { CachedRemoteHead::Miss }, want_attempt);
            let got = read(&bytes);
            if got != want {
                wrong.push(format!("{}: {got:?}, expected {want:?}", String::from_utf8_lossy(&bytes)));
            }
        }
        assert!(wrong.is_empty(), "{} of {} cells read wrong:\n{}", wrong.len(), cells.len(), wrong.join("\n"));

        // A null `sha` is a verified absence, not a miss.
        let absent = set("/answer/sha", Some(Value::Null)).apply(&written);
        assert_eq!(read(&absent), (CachedRemoteHead::Fresh(head(None, NOW)), Some(control)));
    }

    fn receipt() -> Receipt {
        Receipt {
            attempt_id: ID.into(),
            origin_digest: DIGEST.into(),
            branch: "main".into(),
            finished_at: NOW + 30,
            head: HeadStatus::Ok,
            prs: PrStatus::Failed { failure: PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) } },
            durations: LaunchReport::Missing,
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

    /// Two overlapping attempts each keep their own receipt.
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
    /// One edit to a document the real writer wrote.
    enum JsonEdit {
        /// Repeat the key at a JSON pointer, with its written value, in the
        /// same object.
        Dup(String),
        /// Replace (or, with `None`, remove) the value at a JSON pointer.
        Set(String, Option<serde_json::Value>),
        Append(&'static str),
    }

    impl JsonEdit {
        fn apply(&self, written: &serde_json::Value) -> Vec<u8> {
            let mut document = written.clone();
            match self {
                Self::Set(pointer, value) => {
                    let (parent, key) = pointer.rsplit_once('/').unwrap();
                    let target = if parent.is_empty() { &mut document } else { document.pointer_mut(parent).unwrap() };
                    if let Some(array) = target.as_array_mut() {
                        let index: usize = key.parse().unwrap_or_else(|_| panic!("{pointer} indexes an array"));
                        array[index] = value.clone().unwrap_or_else(|| panic!("{pointer}: replace an element, never remove it"));
                        return serde_json::to_vec(&document).unwrap();
                    }
                    let map = target.as_object_mut().unwrap_or_else(|| panic!("{pointer} is not in an object"));
                    match value {
                        Some(value) => {
                            map.insert(key.to_string(), value.clone());
                        }
                        None => assert!(map.remove(key).is_some(), "{pointer} exists"),
                    }
                    serde_json::to_vec(&document).unwrap()
                }
                Self::Dup(pointer) => {
                    let (parent, key) = pointer.rsplit_once('/').unwrap();
                    let target = if parent.is_empty() { &mut document } else { document.pointer_mut(parent).unwrap() };
                    let map = target.as_object_mut().unwrap_or_else(|| panic!("{pointer} is not in an object"));
                    let value = map.get(key).unwrap_or_else(|| panic!("{pointer} exists")).clone();
                    map.insert("__repeat__".into(), value);
                    serde_json::to_string(&document).unwrap().replacen("\"__repeat__\"", &format!("\"{key}\""), 1).into_bytes()
                }
                Self::Append(tail) => {
                    let mut bytes = serde_json::to_vec(&document).unwrap();
                    bytes.extend_from_slice(tail.as_bytes());
                    bytes
                }
            }
        }
    }

    /// The failures that carry fields, as the matrix's positive controls.
    fn field_bearing_failures() -> [PrFailure; 3] {
        [
            PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) },
            PrFailure::CredentialsInsufficient { key: Some("GH_TOKEN".into()) },
            PrFailure::RateLimited { authenticated: true, key: Some("GITHUB_TOKEN".into()) },
        ]
    }

    /// The receipt matrix's cells for a receipt whose PR half failed with
    /// `failure`: each edit, and the PR half the receipt then reports, `None`
    /// for a missing receipt. The same table is walked through `wt list`'s
    /// wait (`list::wait::tests::receipt_matrix`).
    fn receipt_cells(failure: &PrFailure) -> Vec<(JsonEdit, Option<PrStatus>)> {
        use serde_json::{Value, json};
        use JsonEdit::*;
        let set = |pointer: &str, value: Option<Value>| Set(pointer.to_string(), value);
        let failed = |failure: PrFailure| Some(PrStatus::Failed { failure });
        let mut cells = Vec::new();

        // Envelope: absent, null, `[]`, `{}`, empty, a repeated key, a wrong
        // type, and a wrong binding are each a missing receipt.
        for field in ["/format_version", "/attempt_id", "/origin_digest", "/branch", "/finished_at", "/head", "/prs"] {
            for shape in [None, Some(Value::Null), Some(json!([])), Some(json!({})), Some(json!(""))] {
                cells.push((set(field, shape), None));
            }
            cells.push((Dup(field.to_string()), None));
        }
        for (field, wrong) in [
            ("/format_version", json!("1")),
            ("/format_version", json!(2)),
            ("/attempt_id", json!(1)),
            ("/attempt_id", json!(OTHER_ID)),
            ("/attempt_id", json!(ID.to_uppercase())),
            ("/origin_digest", json!(1)),
            ("/origin_digest", json!("another-digest")),
            ("/branch", json!(1)),
            ("/branch", json!("trunk")),
            ("/finished_at", json!("later")),
            ("/finished_at", json!(-1)),
            ("/finished_at", json!(1)),
            ("/head", json!(1)),
            ("/head", json!("moved")),
            ("/prs", json!("failed")),
            ("/prs", json!({ "kind": "skipped-fresh" })),
        ] {
            cells.push((set(field, Some(wrong)), None));
        }

        // `prs.kind`, `prs.failure`, and `prs.failure.kind`: every shape is a
        // missing receipt, never a default or another failure.
        for field in ["/prs/kind", "/prs/failure", "/prs/failure/kind"] {
            for shape in [None, Some(Value::Null), Some(json!([])), Some(json!({})), Some(json!("")), Some(json!(1))] {
                cells.push((set(field, shape), None));
            }
            cells.push((Dup(field.to_string()), None));
        }
        cells.push((set("/prs/kind", Some(json!("skipped-fresh"))), None));
        cells.push((set("/prs/failure", Some(json!("other"))), None));
        cells.push((set("/prs/failure/kind", Some(json!("credentials-lost"))), None));

        // `key`: null is a failure that names no variable; anything but a
        // variable name is a missing receipt.
        let unnamed = match failure.clone() {
            PrFailure::CredentialsRejected { .. } => PrFailure::CredentialsRejected { key: None },
            PrFailure::CredentialsInsufficient { .. } => PrFailure::CredentialsInsufficient { key: None },
            PrFailure::RateLimited { authenticated, .. } => PrFailure::RateLimited { authenticated, key: None },
            other => panic!("{other:?} carries no key"),
        };
        cells.push((set("/prs/failure/key", Some(Value::Null)), failed(unnamed)));
        for shape in [None, Some(json!([])), Some(json!({})), Some(json!("")), Some(json!(1)), Some(json!("a token")), Some(json!("1TOKEN"))] {
            cells.push((set("/prs/failure/key", shape), None));
        }
        cells.push((Dup("/prs/failure/key".into()), None));

        // The rate limit's `authenticated`: only a boolean.
        if let PrFailure::RateLimited { key, .. } = failure {
            cells.push((set("/prs/failure/authenticated", Some(json!(false))), failed(PrFailure::RateLimited { authenticated: false, key: key.clone() })));
            for shape in [None, Some(Value::Null), Some(json!([])), Some(json!({})), Some(json!("")), Some(json!("true")), Some(json!(1))] {
                cells.push((set("/prs/failure/authenticated", shape), None));
            }
            cells.push((Dup("/prs/failure/authenticated".into()), None));
        }

        // Unknown fields are ignored; trailing content is a missing receipt.
        cells.push((set("/extra", Some(json!(1))), failed(failure.clone())));
        cells.push((set("/prs/failure/extra", Some(json!(1))), failed(failure.clone())));
        cells.push((Append("garbage"), None));
        cells.push((Append("{}"), None));
        cells
    }

    /// The Input Robustness Matrix for the receipt: every load-bearing field
    /// in every shape, for every failure that carries fields, one edit per
    /// cell from a file written by [`write_receipt`]. A rejected cell is a
    /// missing receipt, never a default outcome.
    #[test]
    fn the_receipt_reader_walks_the_input_robustness_matrix() {
        let (dir, _) = temp_store();
        let path = dir.path().join("abc.refresh-receipt.json");
        let mut wrong = Vec::new();
        let mut walked = 0;
        for failure in field_bearing_failures() {
            let control = Receipt { prs: PrStatus::Failed { failure: failure.clone() }, ..receipt() };
            write_receipt(&path, &control).unwrap();
            let written = raw_json(&path);
            let read = |bytes: &[u8]| {
                fs::write(&path, bytes).unwrap();
                load_receipt(&path, &attempt(NOW))
            };
            assert_eq!(read(&serde_json::to_vec(&written).unwrap()), Some(control.clone()), "control: {failure:?}");

            for (edit, prs) in receipt_cells(&failure) {
                let bytes = edit.apply(&written);
                let want = prs.map(|prs| Receipt { prs, ..control.clone() });
                let got = read(&bytes);
                if got != want {
                    wrong.push(format!("{}: {got:?}, expected {want:?}", String::from_utf8_lossy(&bytes)));
                }
                walked += 1;
            }
        }
        assert!(wrong.is_empty(), "{} of {walked} cells read wrong:\n{}", wrong.len(), wrong.join("\n"));

        // `ok` with no store publication is still the receipt's answer.
        write_receipt(&path, &Receipt { prs: PrStatus::Ok, ..receipt() }).unwrap();
        assert_eq!(load_receipt(&path, &attempt(NOW)).map(|receipt| receipt.prs), Some(PrStatus::Ok));
    }

    fn timed(halves: &[Stage]) -> Receipt {
        Receipt { durations: LaunchReport::from_worker(worker_fixture(halves)), ..receipt() }
    }

    #[test]
    fn a_receipt_round_trips_with_and_without_the_workers_durations() {
        let (dir, _) = temp_store();
        let path = dir.path().join("abc.refresh-receipt.json");

        let complete = timed(&[Stage::PrRefresh, Stage::HeadRefresh]);
        assert!(matches!(complete.durations, LaunchReport::Complete(_)), "{complete:?}");
        write_receipt(&path, &complete).unwrap();
        assert_eq!(raw_json(&path)["durations"]["format_version"], 1, "its own version, beside the receipt's");
        assert_eq!(raw_json(&path)["format_version"], RECEIPT_FORMAT_VERSION, "the receipt stays at version 1");
        assert_eq!(load_receipt(&path, &attempt(NOW)), Some(complete.clone()));
        // Read, write, and read again: nothing drifts.
        let reread = load_receipt(&path, &attempt(NOW)).unwrap();
        write_receipt(&path, &reread).unwrap();
        assert_eq!(load_receipt(&path, &attempt(NOW)), Some(complete));

        // A half that panicked left no span: partial, still usable.
        let partial = timed(&[Stage::HeadRefresh]);
        assert!(matches!(partial.durations, LaunchReport::Partial(_)), "{partial:?}");
        write_receipt(&path, &partial).unwrap();
        assert_eq!(load_receipt(&path, &attempt(NOW)), Some(partial));

        // An untimed attempt, and a report with nothing usable, write no member.
        for durations in [LaunchReport::Missing, LaunchReport::Invalid] {
            write_receipt(&path, &Receipt { durations, ..receipt() }).unwrap();
            assert!(raw_json(&path).get("durations").is_none());
            assert_eq!(load_receipt(&path, &attempt(NOW)), Some(receipt()), "absent is missing");
        }
    }

    #[test]
    fn durations_never_rescue_a_receipt_for_another_attempt_origin_or_branch() {
        let (dir, _) = temp_store();
        let path = dir.path().join("abc.refresh-receipt.json");
        write_receipt(&path, &timed(&[Stage::PrRefresh, Stage::HeadRefresh])).unwrap();
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
    }

    /// The Input Robustness Matrix for the receipt's optional `durations`:
    /// every load-bearing field of the worker document in every shape, one
    /// edit per cell from a receipt written with a full report. The outcome
    /// is the control's in every cell; only the report's state changes, and
    /// only trailing content (the receipt's own matrix) loses the receipt.
    #[test]
    fn the_receipt_durations_walk_the_input_robustness_matrix() {
        use serde_json::{Value, json};
        use JsonEdit::*;

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Report {
            Complete,
            Partial,
            Missing,
            Invalid,
        }
        let state = |report: &LaunchReport| match report {
            LaunchReport::Complete(_) => Report::Complete,
            LaunchReport::Partial(_) => Report::Partial,
            LaunchReport::Missing => Report::Missing,
            LaunchReport::Invalid => Report::Invalid,
        };

        let (dir, _) = temp_store();
        let path = dir.path().join("abc.refresh-receipt.json");
        let control = timed(&[Stage::PrRefresh, Stage::HeadRefresh]);
        write_receipt(&path, &control).unwrap();
        let written = raw_json(&path);
        let read = |bytes: &[u8]| {
            fs::write(&path, bytes).unwrap();
            load_receipt(&path, &attempt(NOW))
        };
        assert_eq!(read(&serde_json::to_vec(&written).unwrap()), Some(control.clone()), "control");

        let set = |pointer: &str, value: Option<Value>| Set(pointer.to_string(), value);
        // The halves group, and the head check inside the head half.
        const HALVES: &str = "/durations/spans/1";
        const CHECK: &str = "/durations/spans/1/children/1/children/0";
        assert_eq!(written.pointer(&format!("{HALVES}/stage")), Some(&json!("worker_halves")));
        assert_eq!(written.pointer(&format!("{CHECK}/stage")), Some(&json!("head_check")));

        let mut cells: Vec<(JsonEdit, Report)> = Vec::new();
        // The member: absent is missing; every other shape is invalid.
        cells.push((set("/durations", None), Report::Missing));
        for shape in [Value::Null, json!(1), json!(""), json!([]), json!({}), json!(true)] {
            cells.push((set("/durations", Some(shape)), Report::Invalid));
        }
        cells.push((Dup("/durations".into()), Report::Invalid));

        // Required fields: absent, null, wrong types, a negative or
        // fractional number, and a repeated key are each invalid.
        let required = [
            "/durations/format_version".to_string(),
            "/durations/total_us".into(),
            "/durations/spans".into(),
            "/durations/unattributed_us".into(),
            "/durations/over_attributed_us".into(),
            format!("{HALVES}/stage"),
            format!("{HALVES}/elapsed_us"),
            format!("{HALVES}/children_kind"),
            format!("{HALVES}/children"),
            format!("{HALVES}/unattributed_us"),
            format!("{HALVES}/over_attributed_us"),
            format!("{CHECK}/stage"),
            format!("{CHECK}/elapsed_us"),
            format!("{CHECK}/children_kind"),
            format!("{CHECK}/children"),
        ];
        for field in &required {
            let shapes = [None, Some(Value::Null), Some(json!("x")), Some(json!(-1)), Some(json!(1.5)), Some(json!({})), Some(json!(true))];
            for shape in shapes {
                cells.push((set(field, shape), Report::Invalid));
            }
            if !field.ends_with("/children") {
                cells.push((set(field, Some(json!([]))), Report::Invalid));
            }
            cells.push((Dup(field.clone()), Report::Invalid));
        }

        // Values of the right type that the decoder still refuses.
        cells.push((set("/durations/format_version", Some(json!(2))), Report::Invalid));
        cells.push((set("/durations/format_version", Some(json!("1"))), Report::Invalid));
        cells.push((set("/durations/total_us", Some(json!(399))), Report::Invalid));
        cells.push((set(&format!("{CHECK}/stage"), Some(json!(""))), Report::Invalid));
        cells.push((set(&format!("{CHECK}/stage"), Some(json!("api_request"))), Report::Invalid));
        cells.push((set(&format!("{HALVES}/children_kind"), Some(json!("parallel"))), Report::Invalid));
        cells.push((set(&format!("{HALVES}/unattributed_us"), Some(json!(1))), Report::Invalid));
        cells.push((set(&format!("{HALVES}/stage"), Some(json!("worker_setup"))), Report::Invalid));
        // Decodes, but without the halves group no worker wrote it.
        cells.push((set(&format!("{HALVES}/stage"), Some(json!("head_fetch"))), Report::Invalid));

        // `children`: one bad element or every element bad is invalid;
        // empty, or one half alone, is a partial report.
        cells.push((set(&format!("{HALVES}/children/0"), Some(json!(1))), Report::Invalid));
        cells.push((set("/durations/spans", Some(json!([1, 2]))), Report::Invalid));
        cells.push((set(&format!("{HALVES}/children"), Some(json!([]))), Report::Partial));
        let head_half = written.pointer(&format!("{HALVES}/children/1")).unwrap().clone();
        cells.push((set(&format!("{HALVES}/children"), Some(json!([head_half]))), Report::Partial));
        cells.push((set(&format!("{CHECK}/children"), Some(json!([]))), Report::Complete));

        // `git_calls`: absent is unknown and a count is known; null or any
        // other type is invalid.
        cells.push((set(&format!("{HALVES}/git_calls"), Some(json!(3))), Report::Complete));
        cells.push((set(&format!("{HALVES}/git_calls"), Some(json!(0))), Report::Complete));
        for shape in [Value::Null, json!("3"), json!(-1), json!(1.5), json!([]), json!({})] {
            cells.push((set(&format!("{HALVES}/git_calls"), Some(shape)), Report::Invalid));
        }

        // Unknown fields are ignored at every level.
        cells.push((set("/durations/extra", Some(json!(1))), Report::Complete));
        cells.push((set(&format!("{HALVES}/extra"), Some(json!(1))), Report::Complete));

        let outcome = Receipt { durations: LaunchReport::Missing, ..control.clone() };
        let mut wrong = Vec::new();
        let mut check = |bytes: Vec<u8>, want: Report| {
            let got = read(&bytes);
            let got = got.map(|receipt| (state(&receipt.durations), Receipt { durations: LaunchReport::Missing, ..receipt }));
            if got != Some((want, outcome.clone())) {
                wrong.push(format!("{}: {got:?}, expected {want:?} beside the intact outcome", String::from_utf8_lossy(&bytes)));
            }
        };
        let walked = cells.len() + 1;
        for (edit, want) in cells {
            check(edit.apply(&written), want);
        }
        // A repeated `git_calls` key, once the field is present.
        let mut counted = written.clone();
        counted.pointer_mut(HALVES).unwrap().as_object_mut().unwrap().insert("git_calls".into(), json!(3));
        check(Dup(format!("{HALVES}/git_calls")).apply(&counted), Report::Invalid);
        assert!(wrong.is_empty(), "{} of {walked} cells read wrong:\n{}", wrong.len(), wrong.join("\n"));

        // Trailing content is the receipt's own matrix: no receipt at all.
        assert_eq!(read(&Append("garbage").apply(&written)), None);
    }
}
