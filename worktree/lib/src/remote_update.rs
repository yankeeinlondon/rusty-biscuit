//! The worker's update of `origin/<default>` for `wt list`: check the remote,
//! publish what it answered, and fetch the one tracking ref when it differs.
//!
//! Contracts (spec `2026-09-27-list-freshness-ux` §3):
//!
//! - One attempt holds the nonblocking remote-head lock from its first write
//!   to its outcome. A contender writes nothing and makes no request.
//! - The attempt record is written before any request, and its phase follows
//!   the work: `checking`, `checking-fallback` when the provider API gave way
//!   to `git ls-remote`, then `fetching`.
//! - The provider API call and any `ls-remote` fallback share one
//!   [`REMOTE_HEAD_REFRESH_DEADLINE`] budget. Only a complete `ls-remote`
//!   answer without the ref is an absence; a provider 404 never is.
//! - A successful check is published before the fetch starts, so a failed
//!   fetch keeps the newly checked answer, and a failed check keeps the
//!   previous one.
//! - `origin` and the default branch are read again after each network step;
//!   a change ends the attempt as unavailable and publishes nothing more.
//! - Every store write is best effort: a failed write ends the attempt, and
//!   the foreground's bounded wait covers it.

use std::path::Path;
use std::time::Duration;

use sniff::remote::blocking::PrUnavailable;

use crate::cache::try_lock_sidecar;
use crate::git::git_from;
use crate::live_remote::{GitFailure, LsRemote, fetch_tracking_ref, is_valid_branch_name};
use crate::pull_requests::{origin_digest, origin_url};
use crate::remote_head::{
    Answer, AnswerSource, ApiCondition, ApiNote, Attempt, CheckFailure, FallbackReason, FetchFailure,
    HeadStatus, Outcome, Phase, REMOTE_HEAD_REFRESH_DEADLINE, UnavailableReason, begin_attempt,
    finish_attempt, publish_answer, remote_head_lock_path, set_phase,
};
use crate::worktree::default_branch_in;

/// How long the fetch of the tracking ref may take.
pub const FETCH_DEADLINE: Duration = Duration::from_secs(60);

/// A provider's branch-head API.
pub trait BranchHeadSource {
    /// The object ID of `refs/heads/<branch>` in the repository at `origin`.
    fn branch_head(&self, origin: &str, branch: &str, deadline: Duration) -> Result<String, PrUnavailable>;

    /// The name of the token variable a request for `origin` sends, or
    /// `None` for an anonymous request. It tells a 404 with a key from one
    /// without, which the error itself cannot.
    fn key_in_use(&self, origin: &str) -> Option<String>;
}

/// [`BranchHeadSource`] through sniff's blocking provider client.
#[derive(Debug, Clone, Copy, Default)]
pub struct SniffBranchHeads;

impl BranchHeadSource for SniffBranchHeads {
    fn branch_head(&self, origin: &str, branch: &str, deadline: Duration) -> Result<String, PrUnavailable> {
        sniff::remote::blocking::branch_head(origin, branch, deadline).map(|head| head.sha)
    }

    /// The first of the provider's variables that is set and not empty.
    /// sniff's host-bound `SNIFF_*_TOKEN` override is not seen here, so a 404
    /// sent with only that token reads as sent without one.
    fn key_in_use(&self, origin: &str) -> Option<String> {
        sniff::remote::blocking::credential_env(origin)?
            .variables
            .into_iter()
            .find(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()))
    }
}

/// Git's side of an attempt, addressed to `origin` by name.
pub trait GitRemote {
    /// `git ls-remote origin refs/heads/<branch>`: `Ok(None)` only for a
    /// complete answer without the ref.
    fn live_head(&self, branch: &str, deadline: Duration) -> Result<Option<String>, GitFailure>;

    /// Fetches exactly `refs/remotes/origin/<branch>`.
    fn fetch(&self, branch: &str, deadline: Duration) -> Result<(), GitFailure>;
}

/// [`GitRemote`] through [`crate::live_remote`], run in `main`.
#[derive(Debug, Clone, Copy)]
pub struct GitTransport<'a> {
    pub main: &'a Path,
}

impl GitRemote for GitTransport<'_> {
    fn live_head(&self, branch: &str, deadline: Duration) -> Result<Option<String>, GitFailure> {
        LsRemote { base: self.main, deadline }.head("origin", branch).map_err(|error| error.failure)
    }

    fn fetch(&self, branch: &str, deadline: Duration) -> Result<(), GitFailure> {
        fetch_tracking_ref(self.main, branch, deadline).map_err(|error| error.failure)
    }
}

/// What an attempt reaches outside its store.
pub struct Seams<'a> {
    pub api: &'a dyn BranchHeadSource,
    pub git: &'a dyn GitRemote,
    /// Unix seconds, for the stored timestamps.
    pub now: &'a dyn Fn() -> u64,
    /// Any monotonic clock; only differences are used, for the check budget.
    pub monotonic: &'a dyn Fn() -> Duration,
}

/// One attempt to run.
#[derive(Debug, Clone, Copy)]
pub struct AttemptRequest<'a> {
    /// The remote-head store ([`crate::remote_head::remote_head_store_path`]).
    pub store: &'a Path,
    /// The main checkout.
    pub main: &'a Path,
    /// The attempt id ([`crate::remote_head::new_attempt_id`]).
    pub id: &'a str,
    /// The repository is in `~/.wt.json`: check with `ls-remote` alone.
    pub ignore_api: bool,
}

/// How [`run_attempt`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptEnd {
    /// The attempt recorded this outcome.
    Finished(Outcome),
    /// Another worker holds the lock; nothing was written or asked.
    Contended,
    /// Nothing was recorded or asked: no `origin`, no default branch, or the
    /// lock could not be taken.
    NotStarted,
    /// A store write failed, so the attempt stopped where it was. Its record,
    /// if any, stays unfinished until it expires.
    WriteFailed,
}

impl AttemptEnd {
    /// The live-head half's status in a completion receipt.
    pub fn head_status(self) -> HeadStatus {
        match self {
            Self::Finished(Outcome::InSync | Outcome::Fetched | Outcome::Absent) => HeadStatus::Ok,
            Self::Contended => HeadStatus::AdoptedElsewhere,
            Self::Finished(_) | Self::NotStarted | Self::WriteFailed => HeadStatus::Failed,
        }
    }
}

/// A failed store write; it ends the attempt.
struct WriteFailed;

/// A successful check.
struct Checked {
    sha: Option<String>,
    source: AnswerSource,
    /// Unix seconds, read before the first request.
    checked_at: u64,
}

/// A check's result and what its provider request observed.
struct CheckReport {
    result: Result<Checked, CheckFailure>,
    api: Option<ApiNote>,
}

/// The attempt's identity, for its store writes.
struct Recorder<'a> {
    store: &'a Path,
    id: &'a str,
}

impl Recorder<'_> {
    fn phase(&self, phase: Phase, api: Option<ApiNote>) -> Result<(), WriteFailed> {
        set_phase(self.store, self.id, phase, api).map_err(|_| WriteFailed)
    }

    fn publish(&self, answer: &Answer) -> Result<(), WriteFailed> {
        publish_answer(self.store, answer).map_err(|_| WriteFailed)
    }

    fn finish(&self, outcome: Outcome, api: Option<ApiNote>) -> AttemptEnd {
        match finish_attempt(self.store, self.id, outcome, api) {
            Ok(()) => AttemptEnd::Finished(outcome),
            Err(_) => AttemptEnd::WriteFailed,
        }
    }
}

/// Checks `origin`'s default branch, publishes the answer, and fetches
/// `refs/remotes/origin/<default>` when it differs from the local tip.
pub fn run_attempt(request: AttemptRequest<'_>, seams: &Seams<'_>) -> AttemptEnd {
    let Ok(lock) = try_lock_sidecar(&remote_head_lock_path(request.store)) else {
        return AttemptEnd::NotStarted;
    };
    let Some(_lock) = lock else {
        return AttemptEnd::Contended;
    };
    let main = request.main;
    let Some(origin) = origin_url(main) else {
        return AttemptEnd::NotStarted;
    };
    let Ok(branch) = default_branch_in(main) else {
        return AttemptEnd::NotStarted;
    };
    let digest = origin_digest(&origin);
    let recorder = Recorder { store: request.store, id: request.id };
    let attempt = Attempt::begin(request.id.to_string(), digest.clone(), branch.clone(), (seams.now)());
    if begin_attempt(request.store, &attempt).is_err() {
        return AttemptEnd::WriteFailed;
    }
    // Both the refspec and the ls-remote pattern are built from the name.
    if !is_valid_branch_name(main, &branch) {
        return recorder.finish(Outcome::CheckFailed { reason: CheckFailure::Other }, None);
    }

    let report = match check(&origin, &branch, request.ignore_api, seams, Some(&recorder)) {
        Ok(report) => report,
        Err(WriteFailed) => return AttemptEnd::WriteFailed,
    };
    if let Some(reason) = changed(main, &origin, &branch) {
        return recorder.finish(Outcome::Unavailable { reason }, report.api);
    }
    let checked = match report.result {
        Ok(checked) => checked,
        Err(reason) => return recorder.finish(Outcome::CheckFailed { reason }, report.api),
    };
    let answer = Answer {
        origin_digest: digest.clone(),
        branch: branch.clone(),
        sha: checked.sha.clone(),
        checked_at: checked.checked_at,
        source: checked.source,
    };
    if recorder.publish(&answer).is_err() {
        return AttemptEnd::WriteFailed;
    }
    let Some(remote_sha) = checked.sha else {
        // A verified absence leaves the tracking ref as it is.
        return recorder.finish(Outcome::Absent, report.api);
    };
    let before = tracking_tip(main, &branch);
    if before.as_deref() == Some(remote_sha.as_str()) {
        return recorder.finish(Outcome::InSync, report.api);
    }

    if recorder.phase(Phase::Fetching, None).is_err() {
        return AttemptEnd::WriteFailed;
    }
    let fetch_started = (seams.now)();
    let fetched = seams.git.fetch(&branch, FETCH_DEADLINE);
    if let Some(reason) = changed(main, &origin, &branch) {
        return recorder.finish(Outcome::Unavailable { reason }, report.api);
    }
    let after = tracking_tip(main, &branch);
    let outcome = match fetched {
        // The fetched tip, not the checked SHA: the remote may have moved
        // between the check and the fetch.
        Ok(()) => match after {
            Some(tip) => {
                let answer = Answer { sha: Some(tip), checked_at: fetch_started, source: AnswerSource::Fetch, ..answer };
                if recorder.publish(&answer).is_err() {
                    return AttemptEnd::WriteFailed;
                }
                Outcome::Fetched
            }
            None => Outcome::FetchFailed { reason: FetchFailure::Other },
        },
        Err(failure) => {
            let failed = Outcome::FetchFailed {
                reason: match failure {
                    GitFailure::Timeout => FetchFailure::Timeout,
                    GitFailure::Credentials | GitFailure::Other => FetchFailure::Other,
                },
            };
            match after.filter(|tip| before.as_ref() != Some(tip)) {
                // Someone else moved the ref, perhaps a concurrent fetch. It
                // counts only if it reached what the remote has *now*.
                Some(tip) => match confirm_tip(main, &origin, &branch, &tip, request.ignore_api, seams) {
                    Some(recheck) => {
                        let answer = Answer {
                            sha: recheck.sha,
                            checked_at: recheck.checked_at,
                            source: recheck.source,
                            ..answer
                        };
                        if recorder.publish(&answer).is_err() {
                            return AttemptEnd::WriteFailed;
                        }
                        Outcome::Fetched
                    }
                    None => failed,
                },
                None => failed,
            }
        }
    };
    recorder.finish(outcome, report.api)
}

/// A second check, with its own budget and no phase writes, that confirms
/// the remote still has `tip`, while `origin` and the branch are unchanged.
fn confirm_tip(
    main: &Path,
    origin: &str,
    branch: &str,
    tip: &str,
    ignore_api: bool,
    seams: &Seams<'_>,
) -> Option<Checked> {
    let checked = check(origin, branch, ignore_api, seams, None).ok()?.result.ok()?;
    (checked.sha.as_deref() == Some(tip) && changed(main, origin, branch).is_none()).then_some(checked)
}

/// Asks the provider API, then `ls-remote` when the API cannot answer, all
/// within one [`REMOTE_HEAD_REFRESH_DEADLINE`]. `recorder`, when given,
/// records the switch to the fallback.
fn check(
    origin: &str,
    branch: &str,
    ignore_api: bool,
    seams: &Seams<'_>,
    recorder: Option<&Recorder<'_>>,
) -> Result<CheckReport, WriteFailed> {
    let checked_at = (seams.now)();
    let started = (seams.monotonic)();
    let git_check = |api: Option<ApiNote>| {
        let remaining = REMOTE_HEAD_REFRESH_DEADLINE.saturating_sub((seams.monotonic)().saturating_sub(started));
        if remaining.is_zero() {
            return CheckReport { result: Err(CheckFailure::Timeout), api };
        }
        match seams.git.live_head(branch, remaining) {
            Ok(sha) => CheckReport {
                result: Ok(Checked { sha, source: AnswerSource::Git, checked_at }),
                api: api.map(|note| ApiNote { fallback_answered: true, ..note }),
            },
            Err(failure) => CheckReport {
                result: Err(match failure {
                    GitFailure::Timeout => CheckFailure::Timeout,
                    GitFailure::Credentials => CheckFailure::Credentials,
                    GitFailure::Other => CheckFailure::Other,
                }),
                api,
            },
        }
    };
    if ignore_api {
        return Ok(git_check(None));
    }
    let error = match seams.api.branch_head(origin, branch, REMOTE_HEAD_REFRESH_DEADLINE) {
        Ok(sha) => {
            return Ok(CheckReport {
                result: Ok(Checked { sha: Some(sha), source: AnswerSource::Api, checked_at }),
                api: None,
            });
        }
        // Not a provider sniff can ask: Git is the check itself, not a
        // fallback, so the phase stays `checking`.
        Err(PrUnavailable::Unsupported { .. }) => return Ok(git_check(None)),
        Err(error) => error,
    };
    let (reason, note) = fallback(&error, || seams.api.key_in_use(origin));
    if let Some(recorder) = recorder {
        recorder.phase(Phase::CheckingFallback { reason }, note.clone())?;
    }
    Ok(git_check(note))
}

/// Why the API gave way to `ls-remote`, and the §5 condition it observed.
fn fallback(error: &PrUnavailable, key_in_use: impl FnOnce() -> Option<String>) -> (FallbackReason, Option<ApiNote>) {
    let note = |condition, key| Some(ApiNote { condition, key, fallback_answered: false });
    match error {
        PrUnavailable::CredentialsRequired { .. } => {
            (FallbackReason::NoKey, note(ApiCondition::CredentialsRequired, None))
        }
        PrUnavailable::CredentialsRejected { key } => {
            (FallbackReason::Rejected, note(ApiCondition::CredentialsRejected, Some(key.clone())))
        }
        PrUnavailable::CredentialsInsufficient { key } => {
            (FallbackReason::Rejected, note(ApiCondition::CredentialsInsufficient, Some(key.clone())))
        }
        PrUnavailable::RateLimited { authenticated, key } => (
            FallbackReason::RateLimited,
            note(ApiCondition::RateLimited { authenticated: *authenticated }, key.clone()),
        ),
        PrUnavailable::NotFoundOrNotPermitted { .. } => {
            let key = key_in_use();
            // Without a key the repository may simply be private.
            let reason = if key.is_none() { FallbackReason::NotVisible } else { FallbackReason::Other };
            (reason, note(ApiCondition::NotFoundOrNotPermitted, key))
        }
        _ => (FallbackReason::Other, None),
    }
}

/// Why the attempt no longer speaks for `origin` and `branch`, if it does not.
fn changed(main: &Path, origin: &str, branch: &str) -> Option<UnavailableReason> {
    if origin_url(main).as_deref() != Some(origin) {
        Some(UnavailableReason::OriginChanged)
    } else if default_branch_in(main).ok().as_deref() != Some(branch) {
        Some(UnavailableReason::BranchChanged)
    } else {
        None
    }
}

/// The object ID `refs/remotes/origin/<branch>` points at, if it exists.
fn tracking_tip(main: &Path, branch: &str) -> Option<String> {
    let refname = format!("refs/remotes/origin/{branch}");
    git_from(main, main, &["rev-parse", "--verify", "--quiet", &refname]).ok()
}

#[cfg(test)]
mod tests;
