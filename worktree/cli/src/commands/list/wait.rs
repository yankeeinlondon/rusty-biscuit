//! The foreground half of `wt list`'s update flow: launch the worker, or
//! adopt the head attempt already running, and follow both of its halves
//! until each has a result or the budget runs out.
//!
//! The head half is followed through the remote-head store. An attempt is
//! identified by its id plus the origin digest and default branch, so an
//! outcome from another id, repository, or branch is never taken as this
//! run's. A contender that exits without recording an attempt is never a
//! finished check: it is adopted only when another attempt is running under
//! the lock, or when this run's receipt proves the head lock was held.
//!
//! The PR half is followed through this run's own completion receipt, which
//! the worker writes on every attempt once both halves are done, and through
//! the PR store's publication id: an id that differs from the one stored at
//! launch proves a successful publication (`fetched_at` is whole seconds and
//! can repeat), and outranks whatever the receipt later says. A contended PR
//! half is complete only when such an id appears once the holder's lock is
//! released, since a failed holder stores nothing. Without one an ordinary
//! wait reports a PR failure, and a forced wait relaunches once, if the
//! budget has not run out; a second such contention is a PR failure too.
//!
//! The two results are kept apart, so a timeout caused by one half never
//! hides the other's.
//!
//! What a successful request was sent with is taken from exactly the
//! publications the wait accepted: the head attempt it followed (its own or
//! an adopted one, whatever environment that worker inherited) and the first
//! new PR publication it saw, read in the same atomic write as the id, so no
//! receipt is needed for it. A PR success known only from a receipt has
//! unknown credentials. A relaunch (for either half) also runs the other half
//! again, so each half's result from an earlier launch is retained: a
//! finished head check, and a PR result the earlier receipt reported. The
//! replacement's own result supersedes it; a replacement that cannot start,
//! stops, times out, or exits without a receipt leaves it standing. The core ([`wait`]) is pure over [`WaitEnv`], so tests
//! script the stores, the receipt, the locks, and the clock.

use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant};

use biscuit_terminal::components::spinner::{Spinner, SpinnerHandle};
use worktree::pull_requests::{StoredPublication, pr_lock_held, stored_publication, unix_now};
use worktree::remote_head::{
    ATTEMPT_MAX_AGE, Attempt, CredentialEvidence, FallbackReason, HeadStatus, Phase, PrFailure, PrStatus, Receipt, StoreState,
    load_receipt, new_attempt_id, read_store, receipt_path_beside, refresh_lock_held,
};

/// How long ordinary listing waits for both halves.
pub const ORDINARY_BUDGET: Duration = Duration::from_secs(3);
/// How long `--refresh` and `--ff` wait: the worker's 10 s check, 60 s
/// fetch, and publication allowance.
pub const FORCED_BUDGET: Duration = ATTEMPT_MAX_AGE;
const POLL_INTERVAL: Duration = Duration::from_millis(25);
/// The spinner stays hidden this long, so a quick answer never flashes it.
const SPINNER_DELAY: Duration = Duration::from_millis(150);

/// What the launched worker is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchArgs {
    pub attempt: String,
}

/// A launched worker, polled for exit and never joined or killed.
pub struct WorkerHandle {
    exited: Box<dyn FnMut() -> bool + Send>,
}

impl WorkerHandle {
    /// `exited` answers whether the worker has exited; once `true`, it must
    /// stay `true`.
    pub fn new(exited: impl FnMut() -> bool + Send + 'static) -> Self {
        Self { exited: Box::new(exited) }
    }

    /// A spawned process. An error from `try_wait` reads as exited, since
    /// nothing more can be learned from it.
    pub fn from_child(mut child: Child) -> Self {
        Self::new(move || !matches!(child.try_wait(), Ok(None)))
    }

    pub fn has_exited(&mut self) -> bool {
        (self.exited)()
    }
}

/// Starts `wt internal-refresh` for a main checkout.
pub type WorkerLaunch = fn(&Path, &LaunchArgs) -> std::io::Result<WorkerHandle>;

/// Which attempt the wait is for.
#[derive(Debug, Clone, Copy)]
pub struct WaitRequest<'a> {
    pub main: &'a Path,
    pub origin_digest: &'a str,
    pub branch: &'a str,
    /// `--refresh`/`--ff`: retry a PR half whose contending holder published
    /// nothing, and a head attempt held for another origin or branch.
    pub force: bool,
    pub budget: Duration,
}

/// Everything the wait reads, and how its time passes.
pub trait WaitEnv {
    fn store(&self) -> StoreState;
    /// The completion receipt for `attempt`, if one matches it.
    fn receipt(&self, attempt: &Attempt) -> Option<Receipt>;
    /// Deletes attempt `attempt_id`'s receipt, if it exists yet; called once,
    /// as the wait returns, for every attempt it launched.
    fn discard_receipt(&self, attempt_id: &str);
    /// Takes the lock for an instant: probe only when no worker of ours can
    /// be about to take it.
    fn head_lock_held(&self) -> bool;
    /// As [`WaitEnv::head_lock_held`], for the PR lock.
    fn pr_lock_held(&self) -> bool;
    /// The usable stored PR answer's publication for the current `origin`
    /// (`worktree::pull_requests::stored_publication`). Its id changes with
    /// every successful write, whereas `fetched_at` can repeat within a
    /// second.
    fn pr_publication(&self) -> Option<StoredPublication>;
    /// Time since the wait began.
    fn elapsed(&self) -> Duration;
    fn unix_now(&self) -> u64;
    fn sleep(&self, duration: Duration);
    fn new_attempt_id(&self) -> Option<String>;
}

/// How the wait ended: each half's result, kept apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitEnd {
    pub head: HeadEnd,
    pub prs: PrEnd,
    /// What the new PR publication this wait accepted was sent with;
    /// [`CredentialEvidence::Unknown`] when it accepted none (a success
    /// known only from a receipt included). Never an older answer's.
    pub pr_credentials: CredentialEvidence,
    /// The budget ran out before both halves had a result.
    pub timed_out: bool,
}

/// What the wait learned of the followed head attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadEnd {
    /// The followed attempt has an outcome.
    Finished(Attempt),
    /// The budget ran out first; `last` is the followed attempt as last seen.
    Running { last: Option<Attempt> },
    /// The worker could not be launched, stopped mid-attempt, or exited with
    /// no attempt to follow, and no earlier launch of this wait finished the
    /// check (a retry keeps that one as [`HeadEnd::Finished`]).
    Unavailable,
}

/// What the wait learned of this run's PR half. A contended half never ends
/// as such: it resolves to [`PrEnd::Published`], [`PrEnd::Failed`], or
/// [`PrEnd::Pending`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrEnd {
    /// A new usable publication appeared during the wait (this run's, or a
    /// contending holder's), or the receipt reported success.
    Published,
    /// The repository is in `~/.wt.json`.
    Ignored,
    /// `origin` has no provider to ask.
    Unsupported,
    /// The request failed, the worker exited or could not start without a
    /// receipt or a new publication, or a contending holder published
    /// nothing. Only a receipt carries a specific reason; one from an
    /// earlier launch stands until a replacement's receipt reports again.
    Failed(PrFailure),
    /// The budget ran out with no result.
    Pending,
}

/// Launches the worker and follows its attempt; `on_phase` sees every phase
/// the followed head attempt enters, including the first, and
/// [`Phase::Checking`] once the head half is done while the PR half is not;
/// again after a relaunched or adopted head showed a phase and finished.
///
/// The receipt of every attempt it launched is deleted, once, as it returns
/// (each is read only by this run), so a receipt written later survives until
/// a worker's age sweep (`worktree::remote_head::remove_stale_receipts`). That
/// happens after a timeout, and after an early success: a new publication, or
/// a result retained from a replaced launch, plus a finished head can end the
/// wait before the last worker writes its receipt. An adopted attempt's
/// receipt belongs to its own run.
pub fn wait(request: WaitRequest<'_>, env: &dyn WaitEnv, launch: WorkerLaunch, on_phase: &mut dyn FnMut(Phase)) -> WaitEnd {
    let mut launched = Vec::new();
    let end = launch_and_follow(request, env, launch, on_phase, &mut launched);
    for attempt_id in &launched {
        env.discard_receipt(attempt_id);
    }
    end
}

/// [`wait`], recording each launched attempt id in `launched`.
fn launch_and_follow(
    request: WaitRequest<'_>,
    env: &dyn WaitEnv,
    launch: WorkerLaunch,
    on_phase: &mut dyn FnMut(Phase),
    launched: &mut Vec<String>,
) -> WaitEnd {
    // Read once, before the first launch: any later id is a publication made
    // during this wait, whichever launch caused it.
    let pr_before = env.pr_publication().map(|publication| publication.id);
    let mut follow = Follow {
        request,
        env,
        on_phase,
        last: None,
        pr_before,
        pr_published: false,
        pr_credentials: CredentialEvidence::Unknown,
        pr_retried: false,
        pr_only: false,
        head_retained: None,
        pr_retained: None,
    };
    loop {
        let Some(token) = env.new_attempt_id() else {
            return follow.unavailable();
        };
        let launched_at = env.unix_now();
        let mut worker = match launch(request.main, &LaunchArgs { attempt: token.clone() }) {
            Ok(handle) => {
                launched.push(token.clone());
                handle
            }
            Err(_) => return follow.unavailable(),
        };
        if let Some(end) = follow.run(&mut worker, &token, launched_at) {
            return end;
        }
    }
}

struct Follow<'r, 'e> {
    request: WaitRequest<'r>,
    env: &'e dyn WaitEnv,
    on_phase: &'e mut dyn FnMut(Phase),
    last: Option<Attempt>,
    /// [`WaitEnv::pr_publication`] before the first launch.
    pr_before: Option<String>,
    /// A new publication was seen; nothing the receipt says undoes that.
    pr_published: bool,
    /// The credentials of the first new publication seen, read with its id;
    /// a later publication never replaces them.
    pr_credentials: CredentialEvidence,
    /// A contended PR half has already cost one relaunch.
    pr_retried: bool,
    /// The spinner was told that only the PR half is left, and no head phase
    /// has been shown since: a replacement or adopted head's phase clears it,
    /// so that head finishing tells the spinner again.
    pr_only: bool,
    /// The finished head attempt of an earlier launch, kept while a PR
    /// retry runs the head half again.
    head_retained: Option<Attempt>,
    /// The PR result an earlier launch's receipt reported, kept while a head
    /// retry runs the PR half again.
    pr_retained: Option<PrEnd>,
}

/// The PR half's state at one poll.
enum PrState {
    Resolved(PrEnd),
    Waiting,
    /// Contended, and the holder published nothing: relaunch once the head
    /// half allows it.
    Retry,
}

impl Follow<'_, '_> {
    /// Follows `token`'s attempt, or the head attempt it adopts, for one
    /// launch. `None` asks for a new launch: a forced run found the head lock
    /// held for another origin or branch, and that holder has finished; or
    /// its PR half was contended and the holder published nothing. Either is
    /// asked only while [`Follow::may_relaunch`] allows; past it, the wait
    /// ends here as a timeout.
    fn run(&mut self, worker: &mut WorkerHandle, token: &str, launched_at: u64) -> Option<WaitEnd> {
        let request = self.request;
        let ours = Attempt::begin(token.to_string(), request.origin_digest.to_string(), request.branch.to_string(), launched_at);
        let mut followed = token.to_string();
        loop {
            // Exit is read before the stores and the receipt, so anything
            // written just before the worker exited is always seen.
            let exited = worker.has_exited();
            let current = self.current();
            if let Some(attempt) = current.as_ref().filter(|attempt| attempt.id == followed) {
                self.observe(attempt.clone());
            }
            // Always our own launched id, even while following another's.
            let receipt = self.env.receipt(&ours);

            let seen = self.last.as_ref().filter(|attempt| attempt.id == followed);
            let mut head = seen.filter(|attempt| attempt.outcome.is_some()).cloned().map(HeadEnd::Finished);
            if head.is_none() && exited && followed == token {
                if seen.is_some() {
                    // Our worker stopped mid-attempt: a failed store write.
                    head = Some(self.head_or_retained(HeadEnd::Unavailable));
                } else {
                    // Our worker never recorded an attempt: another held the lock.
                    match self.adoption(current, receipt.as_ref()) {
                        Adoption::Follow(attempt) => {
                            followed = attempt.id.clone();
                            self.observe(attempt);
                            continue;
                        }
                        Adoption::Relaunch => {
                            self.retain_pr(receipt.as_ref());
                            return None;
                        }
                        // Left unresolved, so the budget check below ends it
                        // as a timeout.
                        Adoption::Wait => {}
                        Adoption::None => head = Some(self.head_or_retained(HeadEnd::Unavailable)),
                    }
                }
            }

            let mut retry = false;
            let prs = match self.pr_state(receipt.as_ref(), exited) {
                PrState::Resolved(prs) => Some(prs),
                PrState::Waiting => None,
                PrState::Retry if head.is_some() && self.may_relaunch() => {
                    self.pr_retried = true;
                    if let Some(HeadEnd::Finished(attempt)) = head {
                        self.head_retained = Some(attempt);
                    }
                    return None;
                }
                // Wait for the head half, or end at the budget: the holder
                // published nothing either way.
                PrState::Retry => {
                    retry = true;
                    None
                }
            };

            match (head, prs) {
                (Some(head), Some(prs)) => {
                    return Some(WaitEnd { head, prs, pr_credentials: self.pr_credentials.clone(), timed_out: false });
                }
                (head, prs) => {
                    if head.is_some() && !self.pr_only {
                        // The head's last phase no longer describes the wait.
                        self.pr_only = true;
                        (self.on_phase)(Phase::Checking);
                    }
                    if self.env.elapsed() >= request.budget {
                        let head = head.unwrap_or_else(|| {
                            self.head_or_retained(HeadEnd::Running {
                                last: self.last.clone().filter(|attempt| attempt.id == followed),
                            })
                        });
                        // A holder that published nothing has finished: not pending.
                        let unresolved =
                            if retry { self.pr_unknown() } else { self.pr_retained.clone().unwrap_or(PrEnd::Pending) };
                        return Some(WaitEnd {
                            head,
                            prs: prs.unwrap_or(unresolved),
                            pr_credentials: self.pr_credentials.clone(),
                            timed_out: true,
                        });
                    }
                }
            }
            self.env.sleep(POLL_INTERVAL);
        }
    }

    /// The end when no worker can be launched: what earlier launches of this
    /// wait established, if any, and otherwise no head attempt and a generic
    /// PR failure.
    fn unavailable(&mut self) -> WaitEnd {
        let prs = if self.published() { PrEnd::Published } else { self.pr_unknown() };
        WaitEnd {
            head: self.head_or_retained(HeadEnd::Unavailable),
            prs,
            pr_credentials: self.pr_credentials.clone(),
            timed_out: false,
        }
    }

    /// The gate for every replacement launch. The budget is shared, so a
    /// launch at or past it starts work this wait cannot include: the wait
    /// ends as a timeout instead, with the results it already has.
    fn may_relaunch(&self) -> bool {
        self.env.elapsed() < self.request.budget
    }

    /// `end`, unless an earlier launch finished the head check: a head that
    /// did not finish this time is no newer evidence than one that did.
    fn head_or_retained(&self, end: HeadEnd) -> HeadEnd {
        self.head_retained.clone().map_or(end, HeadEnd::Finished)
    }

    /// The PR result when this launch gave no usable one: a publication
    /// seen, else an earlier receipt's report, else a generic failure.
    fn pr_unknown(&self) -> PrEnd {
        if self.pr_published {
            return PrEnd::Published;
        }
        self.pr_retained.clone().unwrap_or(PrEnd::Failed(PrFailure::Other))
    }

    /// Keeps what `receipt` (of the launch being replaced) reported of the
    /// PR half. A success counts as a publication, which no later receipt
    /// undoes; a contention makes the relaunch this wait's one PR retry too.
    fn retain_pr(&mut self, receipt: Option<&Receipt>) {
        let Some(status) = receipt.map(|receipt| &receipt.prs) else {
            return;
        };
        match reported(status) {
            Some(PrEnd::Published) => self.pr_published = true,
            Some(prs) => self.pr_retained = Some(prs),
            None => self.pr_retried = true,
        }
    }

    /// A usable publication id other than the one stored before launch.
    /// The first one seen is the accepted publication: its credentials are
    /// kept, from the same read as its id.
    fn published(&mut self) -> bool {
        if !self.pr_published
            && let Some(publication) = self.env.pr_publication()
            && Some(&publication.id) != self.pr_before.as_ref()
        {
            self.pr_published = true;
            self.pr_credentials = publication.credentials;
        }
        self.pr_published
    }

    /// The PR half as of this poll. The PR lock is probed only once our
    /// receipt reports contention, so the probe can never take the lock
    /// ahead of our own worker's PR half.
    fn pr_state(&mut self, receipt: Option<&Receipt>, exited: bool) -> PrState {
        if self.published() {
            return PrState::Resolved(PrEnd::Published);
        }
        let resolved = match receipt.map(|receipt| &receipt.prs) {
            Some(PrStatus::Contended) => {
                if self.env.pr_lock_held() {
                    return PrState::Waiting;
                }
                // Read after the probe: the holder publishes before releasing.
                if self.published() {
                    PrEnd::Published
                } else if self.request.force && !self.pr_retried {
                    return PrState::Retry;
                } else {
                    self.pr_unknown()
                }
            }
            Some(status) => reported(status).unwrap_or_else(|| self.pr_unknown()),
            // Exited without a usable receipt: no new reason can be told.
            None if exited => self.pr_unknown(),
            None => return PrState::Waiting,
        };
        PrState::Resolved(resolved)
    }

    fn current(&self) -> Option<Attempt> {
        let request = self.request;
        self.env
            .store()
            .attempt
            .filter(|attempt| attempt.is_current_for(request.origin_digest, request.branch, self.env.unix_now()))
    }

    fn observe(&mut self, attempt: Attempt) {
        let changed = self.last.as_ref().is_none_or(|last| last.id != attempt.id || last.phase != attempt.phase);
        if changed {
            self.pr_only = false;
            (self.on_phase)(attempt.phase);
        }
        self.last = Some(attempt);
    }

    /// What to do once our worker has exited without an attempt of its own.
    ///
    /// Its lock is released, so probing cannot take it from our worker.
    fn adoption(&self, current: Option<Attempt>, receipt: Option<&Receipt>) -> Adoption {
        let adopted_elsewhere = receipt.is_some_and(|receipt| receipt.head == HeadStatus::AdoptedElsewhere);
        match current {
            // Another worker is checking this origin and branch now.
            Some(attempt) if attempt.outcome.is_none() && self.env.head_lock_held() => Adoption::Follow(attempt),
            // Our receipt proves the lock was held when our worker tried, so
            // that attempt ran alongside this run.
            Some(attempt) if adopted_elsewhere => Adoption::Follow(attempt),
            // Held for another origin or branch: wait for that holder, then
            // try again if the budget still allows a launch.
            None if adopted_elsewhere && self.request.force => {
                if !self.env.head_lock_held() && self.may_relaunch() { Adoption::Relaunch } else { Adoption::Wait }
            }
            _ => Adoption::None,
        }
    }
}

enum Adoption {
    Follow(Attempt),
    Relaunch,
    /// A forced run's holder for another origin or branch still holds the
    /// lock, or released it too late to relaunch: poll again, or time out.
    Wait,
    None,
}

/// The PR result a receipt reports by itself; `None` for a contention,
/// which only the publication id and the PR lock can resolve.
fn reported(status: &PrStatus) -> Option<PrEnd> {
    match status {
        PrStatus::Ok => Some(PrEnd::Published),
        PrStatus::Failed { failure } => Some(PrEnd::Failed(failure.clone())),
        PrStatus::Ignored => Some(PrEnd::Ignored),
        PrStatus::Unsupported => Some(PrEnd::Unsupported),
        PrStatus::Contended => None,
    }
}

/// The spinner's text for `phase`.
pub fn phase_text(phase: Phase) -> &'static str {
    match phase {
        Phase::CheckingFallback { reason: FallbackReason::NoKey | FallbackReason::NotVisible } => {
            "no API key, using fallback method"
        }
        Phase::CheckingFallback { reason: FallbackReason::RateLimited } => "rate limited, using fallback method",
        Phase::Fetching => "pulling remote updates",
        Phase::Checking | Phase::CheckingFallback { .. } => "updating",
    }
}

/// The spinner shown while waiting; it draws only when its output is a
/// terminal, after [`SPINNER_DELAY`].
pub struct Progress {
    spinner: SpinnerHandle,
}

impl Progress {
    pub fn on_stderr() -> Self {
        Self { spinner: Spinner::new(phase_text(Phase::Checking)).with_delay(SPINNER_DELAY).start_on_stderr() }
    }

    #[cfg(test)]
    pub fn on(writer: impl std::io::Write + Send + 'static, is_terminal: bool) -> Self {
        Self {
            spinner: Spinner::new(phase_text(Phase::Checking)).with_delay(SPINNER_DELAY).start_on(writer, is_terminal),
        }
    }

    pub fn show(&self, phase: Phase) {
        self.spinner.set_text(phase_text(phase));
    }

    /// Clears the spinner's line, if it drew one.
    pub fn finish(self) {
        self.spinner.finish();
    }
}

/// The production [`WaitEnv`]: the real store, receipt, locks, and clock.
pub struct StoreEnv {
    store: PathBuf,
    pr_store: PathBuf,
    origin: String,
    started: Instant,
}

impl StoreEnv {
    /// Receipts are read beside the remote-head store `store`.
    pub fn new(store: PathBuf, pr_store: PathBuf, origin: String) -> Self {
        Self { store, pr_store, origin, started: Instant::now() }
    }
}

impl WaitEnv for StoreEnv {
    fn store(&self) -> StoreState {
        read_store(&self.store)
    }

    fn receipt(&self, attempt: &Attempt) -> Option<Receipt> {
        load_receipt(&receipt_path_beside(&self.store, &attempt.id).ok()?, attempt)
    }

    fn discard_receipt(&self, attempt_id: &str) {
        if let Ok(path) = receipt_path_beside(&self.store, attempt_id) {
            let _ = std::fs::remove_file(path);
        }
    }

    fn head_lock_held(&self) -> bool {
        refresh_lock_held(&self.store)
    }

    fn pr_lock_held(&self) -> bool {
        pr_lock_held(&self.pr_store)
    }

    fn pr_publication(&self) -> Option<StoredPublication> {
        stored_publication(&self.pr_store, &self.origin, unix_now())
    }

    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    fn unix_now(&self) -> u64 {
        unix_now()
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }

    fn new_attempt_id(&self) -> Option<String> {
        new_attempt_id().ok()
    }
}

#[cfg(test)]
mod tests;
