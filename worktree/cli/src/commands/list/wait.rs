//! The foreground half of `wt list`'s update flow (spec §3, plan Rule 6):
//! launch the worker, or adopt the one already running, and follow its
//! attempt through the remote-head store until it has an outcome or the
//! budget runs out.
//!
//! An attempt is identified by its id plus the origin digest and default
//! branch, so an outcome from another id, repository, or branch is never
//! taken as this run's. A contender that exits without recording an attempt
//! is never a finished check: it is adopted only when another attempt is
//! running under the lock, and is otherwise [`WaitEnd::Unavailable`].
//!
//! A forced run's contended PR half is complete only when a refresh
//! published an answer after launch, which the stored answer's publication
//! id and writer show (`fetched_at` is whole seconds and can repeat): a
//! released lock proves only that the holder's request ended, and a failed
//! or skipped request stores nothing. An ordinary listing's miss answer does
//! not count: it can be stored whenever the lock is momentarily free, such as
//! just after a failed holder releases it, and its request may predate this
//! run. Without a refresh's answer the run relaunches once; a second such
//! contention ends as a PR failure.
//!
//! The core ([`wait`]) is pure over [`WaitEnv`], so tests script the store,
//! the locks, and the clock.

use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant};

use biscuit_terminal::components::spinner::{Spinner, SpinnerHandle};
use worktree::pull_requests::{Writer, pr_lock_held, stored_publication, unix_now};
use worktree::remote_head::{
    ATTEMPT_MAX_AGE, Attempt, FallbackReason, HeadStatus, Phase, PrFailure, PrStatus, Receipt, StoreState,
    load_receipt, new_attempt_id, read_store, receipt_path_beside, refresh_lock_held,
};

/// How long ordinary listing waits for the attempt (Decision 1).
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
    pub force: bool,
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
    /// `--refresh`/`--ff`: launch with `--force` and wait for the receipt.
    pub force: bool,
    pub budget: Duration,
}

/// Everything the wait reads, and how its time passes.
pub trait WaitEnv {
    fn store(&self) -> StoreState;
    /// The completion receipt for `attempt`, if one matches it.
    fn receipt(&self, attempt: &Attempt) -> Option<Receipt>;
    /// Deletes attempt `attempt_id`'s receipt, if any; called once the wait
    /// is over for every forced attempt it launched.
    fn discard_receipt(&self, attempt_id: &str);
    /// Takes the lock for an instant: probe only when no worker of ours can
    /// be about to take it.
    fn head_lock_held(&self) -> bool;
    /// As [`WaitEnv::head_lock_held`], for the PR lock.
    fn pr_lock_held(&self) -> bool;
    /// The publication id of the stored PR answer for the current `origin`
    /// (`worktree::pull_requests::stored_publication`) when a refresh wrote
    /// it, and `None` for a listing's answer or no answer. It changes with
    /// every successful write, whereas `fetched_at` can repeat within a
    /// second.
    fn pr_publication(&self) -> Option<String>;
    /// Time since the wait began.
    fn elapsed(&self) -> Duration;
    fn unix_now(&self) -> u64;
    fn sleep(&self, duration: Duration);
    fn new_attempt_id(&self) -> Option<String>;
}

/// How the wait ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitEnd {
    /// The followed attempt has an outcome. Under `force`, `receipt` is this
    /// run's completion receipt, or `None` when the worker exited without
    /// writing one. A PR half left contended after the one relaunch reads
    /// as [`PrStatus::Failed`] with [`PrFailure::Other`].
    Finished { attempt: Attempt, receipt: Option<Receipt> },
    /// The budget ran out; `last` is the followed attempt as last seen.
    TimedOut { last: Option<Attempt> },
    /// The worker could not be launched, or exited with no attempt to follow.
    Unavailable,
}

/// The wait's result, with the launched worker for the caller to ask
/// whether it is still running.
pub struct Waited {
    pub end: WaitEnd,
    pub worker: Option<WorkerHandle>,
}

/// Launches the worker and follows its attempt; `on_phase` sees every phase
/// the followed attempt enters, including the first.
///
/// Under `force`, the receipt of every attempt it launched is discarded
/// before it returns: each is read only by this run.
pub fn wait(
    request: WaitRequest<'_>,
    env: &dyn WaitEnv,
    launch: WorkerLaunch,
    on_phase: &mut dyn FnMut(Phase),
) -> Waited {
    let mut launched = Vec::new();
    let waited = launch_and_follow(request, env, launch, on_phase, &mut launched);
    if request.force {
        for attempt_id in &launched {
            env.discard_receipt(attempt_id);
        }
    }
    waited
}

/// [`wait`], recording each launched attempt id in `launched`.
fn launch_and_follow(
    request: WaitRequest<'_>,
    env: &dyn WaitEnv,
    launch: WorkerLaunch,
    on_phase: &mut dyn FnMut(Phase),
    launched: &mut Vec<String>,
) -> Waited {
    let mut follow = Follow { request, env, on_phase, last: None, pr_retried: false };
    let mut worker = None;
    loop {
        let Some(token) = env.new_attempt_id() else {
            return Waited { end: WaitEnd::Unavailable, worker };
        };
        let launched_at = env.unix_now();
        let pr_before = env.pr_publication();
        let args = LaunchArgs { attempt: token.clone(), force: request.force };
        match launch(request.main, &args) {
            Ok(handle) => {
                launched.push(token.clone());
                worker = Some(handle);
            }
            Err(_) => return Waited { end: WaitEnd::Unavailable, worker },
        }
        let handle = worker.as_mut().expect("just launched");
        if let Some(end) = follow.run(handle, &token, launched_at, pr_before) {
            return Waited { end, worker };
        }
    }
}

struct Follow<'r, 'e> {
    request: WaitRequest<'r>,
    env: &'e dyn WaitEnv,
    on_phase: &'e mut dyn FnMut(Phase),
    last: Option<Attempt>,
    /// A contended PR half has already cost one relaunch.
    pr_retried: bool,
}

impl Follow<'_, '_> {
    /// Follows `token`'s attempt, or the one it adopts, for one launch.
    /// `None` asks for a new launch: a forced run found the lock held for
    /// another origin or branch, and that holder has finished; or its PR
    /// half was contended and the holder published nothing.
    ///
    /// `pr_before` is [`WaitEnv::pr_publication`] at launch; a contending
    /// holder published only if a refresh's id is stored and differs once
    /// its lock opens.
    fn run(
        &mut self,
        worker: &mut WorkerHandle,
        token: &str,
        launched_at: u64,
        pr_before: Option<String>,
    ) -> Option<WaitEnd> {
        let request = self.request;
        let ours = Attempt::begin(token.to_string(), request.origin_digest.to_string(), request.branch.to_string(), launched_at);
        let mut followed = token.to_string();
        loop {
            // Exit is read before the store, so an outcome written just
            // before the worker exited is always seen.
            let exited = worker.has_exited();
            let current = self.current();
            if let Some(attempt) = current.as_ref().filter(|attempt| attempt.id == followed) {
                self.observe(attempt.clone());
            }
            let seen = self.last.as_ref().filter(|attempt| attempt.id == followed);
            let finished = seen.filter(|attempt| attempt.outcome.is_some()).cloned();

            if let Some(attempt) = finished {
                if !request.force {
                    return Some(WaitEnd::Finished { attempt, receipt: None });
                }
                match self.env.receipt(&ours) {
                    Some(receipt) if receipt.prs == PrStatus::Contended => {
                        if !self.env.pr_lock_held() {
                            let after = self.env.pr_publication();
                            let published = after.is_some() && after != pr_before;
                            if published {
                                return Some(WaitEnd::Finished { attempt, receipt: Some(receipt) });
                            }
                            if !self.pr_retried {
                                self.pr_retried = true;
                                return None;
                            }
                            let prs = PrStatus::Failed { failure: PrFailure::Other };
                            return Some(WaitEnd::Finished { attempt, receipt: Some(Receipt { prs, ..receipt }) });
                        }
                    }
                    Some(receipt) => {
                        return Some(WaitEnd::Finished { attempt, receipt: Some(receipt) });
                    }
                    None if exited => return Some(WaitEnd::Finished { attempt, receipt: None }),
                    None => {}
                }
            } else if exited && followed == token {
                if seen.is_some() {
                    // Our worker stopped mid-attempt: a failed store write.
                    return Some(WaitEnd::Unavailable);
                }
                // Our worker never recorded an attempt: another held the lock.
                match self.adoption(current, &ours) {
                    Adoption::Follow(attempt) => {
                        followed = attempt.id.clone();
                        self.observe(attempt);
                        continue;
                    }
                    Adoption::Relaunch => return None,
                    Adoption::None => return Some(WaitEnd::Unavailable),
                }
            }

            if self.env.elapsed() >= request.budget {
                let last = self.last.clone().filter(|attempt| attempt.id == followed);
                return Some(WaitEnd::TimedOut { last });
            }
            self.env.sleep(POLL_INTERVAL);
        }
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
            (self.on_phase)(attempt.phase);
        }
        self.last = Some(attempt);
    }

    /// What to do once our worker has exited without an attempt of its own.
    ///
    /// Its lock is released, so probing cannot take it from our worker.
    fn adoption(&self, current: Option<Attempt>, ours: &Attempt) -> Adoption {
        let receipt = self.request.force.then(|| self.env.receipt(ours)).flatten();
        let adopted_elsewhere = receipt.as_ref().is_some_and(|receipt| receipt.head == HeadStatus::AdoptedElsewhere);
        match current {
            // Another worker is checking this origin and branch now.
            Some(attempt) if attempt.outcome.is_none() && self.env.head_lock_held() => Adoption::Follow(attempt),
            // A forced worker's receipt proves the lock was held when it
            // tried, so that attempt ran alongside this run.
            Some(attempt) if adopted_elsewhere => Adoption::Follow(attempt),
            None if adopted_elsewhere => {
                // Held for another origin or branch: wait for that holder,
                // then try again within the same budget.
                while self.env.head_lock_held() {
                    if self.env.elapsed() >= self.request.budget {
                        return Adoption::None;
                    }
                    self.env.sleep(POLL_INTERVAL);
                }
                Adoption::Relaunch
            }
            _ => Adoption::None,
        }
    }
}

enum Adoption {
    Follow(Attempt),
    Relaunch,
    None,
}

/// The spinner's text for `phase` (spec §3 step 3).
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

    fn pr_publication(&self) -> Option<String> {
        refresh_publication(&self.pr_store, &self.origin)
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

/// [`WaitEnv::pr_publication`] over the PR store at `pr_store`.
fn refresh_publication(pr_store: &Path, origin: &str) -> Option<String> {
    stored_publication(pr_store, origin, unix_now())
        .filter(|publication| publication.writer == Writer::Refresh)
        .map(|publication| publication.id)
}

#[cfg(test)]
mod tests;
