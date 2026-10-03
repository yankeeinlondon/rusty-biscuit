//! The wait protocol over a scripted store, receipt, and locks, and a fake
//! clock that advances only when the wait sleeps.

use std::cell::{Cell, RefCell};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use worktree::remote_head::{
    Attempt, FallbackReason, FetchFailure, HeadStatus, Outcome, Phase, PrFailure, PrStatus, Receipt, StoreState,
};

use super::*;

const DIGEST: &str = "digest-of-origin";
const BRANCH: &str = "main";
const OURS: &str = "0000000000000000000000000000000a";
const SECOND: &str = "0000000000000000000000000000000b";
const OTHER: &str = "ffffffffffffffffffffffffffffffff";
const STARTED: u64 = 1_790_000_000;
/// PR store publication ids: the answer stored at launch, and a new one.
const SEEDED: &str = "5eeded00000000000000000000000000";
const PUBLISHED: &str = "fedcba9876543210fedcba9876543210";

thread_local! {
    /// The fake clock: time since the wait began.
    static CLOCK: Cell<Duration> = const { Cell::new(Duration::ZERO) };
    /// Every launch, in order.
    static LAUNCHES: RefCell<Vec<LaunchArgs>> = const { RefCell::new(Vec::new()) };
    /// When each launched worker exits, by launch order; `None` never.
    static EXITS: RefCell<Vec<Option<Duration>>> = const { RefCell::new(Vec::new()) };
    /// Every receipt the wait discarded, in order.
    static DISCARDED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn discarded() -> Vec<String> {
    DISCARDED.with(|discarded| discarded.borrow().clone())
}

fn now() -> Duration {
    CLOCK.with(Cell::get)
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// Launches a worker that exits as `EXITS` says.
fn launching(_main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
    let index = LAUNCHES.with(|launches| {
        launches.borrow_mut().push(args.clone());
        launches.borrow().len() - 1
    });
    let exit_at = EXITS.with(|exits| exits.borrow().get(index).copied().flatten());
    Ok(WorkerHandle::new(move || exit_at.is_some_and(|at| now() >= at)))
}

fn failing_launch(_main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
    LAUNCHES.with(|launches| launches.borrow_mut().push(args.clone()));
    Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no wt"))
}

fn launches() -> Vec<LaunchArgs> {
    LAUNCHES.with(|launches| launches.borrow().clone())
}

fn launched(id: &str) -> LaunchArgs {
    LaunchArgs { attempt: id.into() }
}

type Script<T> = Box<dyn Fn(Duration) -> T>;
type ReceiptScript = Box<dyn Fn(Duration, &Attempt) -> Option<Receipt>>;

/// A [`WaitEnv`] whose store, receipt, and locks are functions of the fake
/// clock.
struct Fake {
    store: Script<StoreState>,
    receipt: ReceiptScript,
    head_lock: Script<bool>,
    pr_lock: Script<bool>,
    pr_answer: Script<Option<&'static str>>,
    ids: RefCell<Vec<&'static str>>,
    /// Head-lock probes made while a worker of ours was still running.
    early_probes: Cell<usize>,
    /// PR-lock probes, with the time of each.
    pr_probes: RefCell<Vec<Duration>>,
    /// Whether the current worker has exited, as the probe sees it.
    worker_alive: Box<dyn Fn() -> bool>,
    /// A real PR store (and its `origin`) read the way [`StoreEnv`] reads it,
    /// in place of `pr_lock` and `pr_answer`.
    real_prs: Option<(PathBuf, String)>,
}

impl Fake {
    fn new(store: impl Fn(Duration) -> StoreState + 'static) -> Self {
        CLOCK.with(|clock| clock.set(Duration::ZERO));
        LAUNCHES.with(|launches| launches.borrow_mut().clear());
        EXITS.with(|exits| exits.borrow_mut().clear());
        DISCARDED.with(|discarded| discarded.borrow_mut().clear());
        Self {
            store: Box::new(store),
            receipt: Box::new(|_, _| None),
            head_lock: Box::new(|_| false),
            pr_lock: Box::new(|_| false),
            pr_answer: Box::new(|_| None),
            ids: RefCell::new(vec![OURS, SECOND]),
            early_probes: Cell::new(0),
            pr_probes: RefCell::new(Vec::new()),
            worker_alive: Box::new(|| {
                let launched = LAUNCHES.with(|launches| launches.borrow().len());
                let exit = EXITS.with(|exits| exits.borrow().get(launched.wrapping_sub(1)).copied().flatten());
                launched > 0 && exit.is_none_or(|at| now() < at)
            }),
            real_prs: None,
        }
    }

    fn real_pr_store(mut self, store: &Path, origin: &str) -> Self {
        self.real_prs = Some((store.to_path_buf(), origin.to_string()));
        self
    }

    /// The launched workers exit at these times.
    fn exits(self, at: &[Option<Duration>]) -> Self {
        EXITS.with(|exits| *exits.borrow_mut() = at.to_vec());
        self
    }

    fn receipts(mut self, receipt: impl Fn(Duration, &Attempt) -> Option<Receipt> + 'static) -> Self {
        self.receipt = Box::new(receipt);
        self
    }

    fn head_lock(mut self, held: impl Fn(Duration) -> bool + 'static) -> Self {
        self.head_lock = Box::new(held);
        self
    }

    fn pr_lock(mut self, held: impl Fn(Duration) -> bool + 'static) -> Self {
        self.pr_lock = Box::new(held);
        self
    }

    /// The stored PR answer's publication id.
    fn pr_answer(mut self, publication: impl Fn(Duration) -> Option<&'static str> + 'static) -> Self {
        self.pr_answer = Box::new(publication);
        self
    }
}

impl WaitEnv for Fake {
    fn store(&self) -> StoreState {
        (self.store)(now())
    }

    fn receipt(&self, attempt: &Attempt) -> Option<Receipt> {
        (self.receipt)(now(), attempt)
    }

    fn discard_receipt(&self, attempt_id: &str) {
        DISCARDED.with(|discarded| discarded.borrow_mut().push(attempt_id.to_string()));
    }

    fn head_lock_held(&self) -> bool {
        if (self.worker_alive)() {
            self.early_probes.set(self.early_probes.get() + 1);
        }
        (self.head_lock)(now())
    }

    fn pr_lock_held(&self) -> bool {
        self.pr_probes.borrow_mut().push(now());
        match &self.real_prs {
            Some((store, _)) => pr_lock_held(store),
            None => (self.pr_lock)(now()),
        }
    }

    fn pr_publication(&self) -> Option<String> {
        match &self.real_prs {
            Some((store, origin)) => stored_publication(store, origin, unix_now()),
            None => (self.pr_answer)(now()).map(str::to_string),
        }
    }

    fn elapsed(&self) -> Duration {
        now()
    }

    fn unix_now(&self) -> u64 {
        STARTED + now().as_secs()
    }

    fn sleep(&self, duration: Duration) {
        CLOCK.with(|clock| clock.set(clock.get() + duration));
    }

    fn new_attempt_id(&self) -> Option<String> {
        let mut ids = self.ids.borrow_mut();
        (!ids.is_empty()).then(|| ids.remove(0).to_string())
    }
}

fn attempt(id: &str, phase: Phase, outcome: Option<Outcome>) -> Attempt {
    Attempt { phase, outcome, ..Attempt::begin(id.into(), DIGEST.into(), BRANCH.into(), STARTED) }
}

fn finished(id: &str, phase: Phase, outcome: Outcome) -> HeadEnd {
    HeadEnd::Finished(attempt(id, phase, Some(outcome)))
}

fn in_sync(id: &str) -> HeadEnd {
    finished(id, Phase::Checking, Outcome::InSync)
}

fn with(attempt: Attempt) -> StoreState {
    StoreState { answer: None, attempt: Some(attempt) }
}

fn empty() -> StoreState {
    StoreState::default()
}

fn receipt(id: &str, head: HeadStatus, prs: PrStatus) -> Receipt {
    Receipt {
        attempt_id: id.into(),
        origin_digest: DIGEST.into(),
        branch: BRANCH.into(),
        finished_at: STARTED + 1,
        head,
        prs,
    }
}

/// Our receipt from `at` on, with `prs`.
fn receipt_at(at: Duration, prs: PrStatus) -> impl Fn(Duration, &Attempt) -> Option<Receipt> {
    move |t, for_attempt| (t >= at).then(|| receipt(&for_attempt.id, HeadStatus::Ok, prs.clone()))
}

fn ended(head: HeadEnd, prs: PrEnd) -> WaitEnd {
    WaitEnd { head, prs, timed_out: false }
}

fn timed_out(head: HeadEnd, prs: PrEnd) -> WaitEnd {
    WaitEnd { head, prs, timed_out: true }
}

fn other() -> PrEnd {
    PrEnd::Failed(PrFailure::Other)
}

fn request(force: bool) -> WaitRequest<'static> {
    WaitRequest {
        main: Path::new("/repo"),
        origin_digest: DIGEST,
        branch: BRANCH,
        force,
        budget: if force { FORCED_BUDGET } else { ORDINARY_BUDGET },
    }
}

/// Runs the wait; returns its end, the phases it reported, and when it ended.
fn run(fake: &Fake, force: bool, launch: WorkerLaunch) -> (WaitEnd, Vec<Phase>, Duration) {
    let mut phases = Vec::new();
    let end = wait(request(force), fake, launch, &mut |phase| phases.push(phase));
    (end, phases, now())
}

fn within_the_budget(at: Duration, budget: Duration) -> bool {
    at >= budget && at < budget + ms(50)
}

// Both halves.

#[test]
fn our_attempt_is_followed_through_every_phase_to_both_results() {
    let fake = Fake::new(|t| {
        let fallback = Phase::CheckingFallback { reason: FallbackReason::NoKey };
        match t {
            t if t < ms(50) => empty(),
            t if t < ms(200) => with(attempt(OURS, Phase::Checking, None)),
            t if t < ms(400) => with(attempt(OURS, fallback, None)),
            t if t < ms(600) => with(attempt(OURS, Phase::Fetching, None)),
            _ => with(attempt(OURS, Phase::Fetching, Some(Outcome::Fetched))),
        }
    })
    .receipts(receipt_at(ms(600), PrStatus::Ok))
    .exits(&[Some(ms(600))]);

    let (end, phases, at) = run(&fake, false, launching);

    assert_eq!(end, ended(finished(OURS, Phase::Fetching, Outcome::Fetched), PrEnd::Published));
    assert_eq!(phases, [Phase::Checking, Phase::CheckingFallback { reason: FallbackReason::NoKey }, Phase::Fetching]);
    assert!(at >= ms(600) && at < ms(700), "ends once both are in: {at:?}");
    assert_eq!(launches(), [launched(OURS)], "one launch, with no --force");
    assert_eq!(fake.early_probes.get(), 0, "the head lock is never probed while our worker may take it");
    assert!(fake.pr_probes.borrow().is_empty(), "nor the PR lock, without contention");
}

#[test]
fn an_ordinary_wait_does_not_end_on_a_head_outcome_alone() {
    let fake = Fake::new(|t| {
        let outcome = (t >= ms(100)).then_some(Outcome::InSync);
        with(attempt(OURS, Phase::Fetching, outcome))
    })
    .receipts(receipt_at(ms(1_200), PrStatus::Ok))
    .exits(&[Some(ms(1_200))]);

    let (end, phases, at) = run(&fake, false, launching);

    assert_eq!(end, ended(finished(OURS, Phase::Fetching, Outcome::InSync), PrEnd::Published));
    assert!(at >= ms(1_200) && at < ms(1_300), "waits for the receipt: {at:?}");
    assert_eq!(phases, [Phase::Fetching, Phase::Checking], "the spinner says only `updating` once the head is done");
    assert!(fake.pr_probes.borrow().is_empty());
}

#[test]
fn the_budget_ends_the_wait_with_the_head_finished_and_the_pr_half_running() {
    let fake = Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))));

    let (end, phases, at) = run(&fake, false, launching);

    assert_eq!(end, timed_out(in_sync(OURS), PrEnd::Pending), "the completed head outcome is kept");
    assert!(within_the_budget(at, ORDINARY_BUDGET), "{at:?}");
    assert_eq!(phases, [Phase::Checking, Phase::Checking]);
    assert_eq!(discarded(), [OURS], "an ordinary wait discards the receipt it launched");
}

#[test]
fn the_budget_ends_the_wait_with_the_last_phase_seen() {
    for phase in [Phase::Checking, Phase::Fetching] {
        let fake = Fake::new(move |_| with(attempt(OURS, phase, None)));

        let (end, _, at) = run(&fake, false, launching);

        assert_eq!(end, timed_out(HeadEnd::Running { last: Some(attempt(OURS, phase, None)) }, PrEnd::Pending));
        assert!(within_the_budget(at, ORDINARY_BUDGET), "{at:?}");
    }
}

#[test]
fn a_publication_while_the_head_is_held_is_kept_at_the_timeout() {
    let fake = Fake::new(|_| with(attempt(OURS, Phase::Checking, None)))
        .pr_answer(|t| Some(if t < ms(500) { SEEDED } else { PUBLISHED }));

    let (end, phases, at) = run(&fake, false, launching);

    let head = HeadEnd::Running { last: Some(attempt(OURS, Phase::Checking, None)) };
    assert_eq!(end, timed_out(head, PrEnd::Published), "no combined receipt needed");
    assert!(within_the_budget(at, ORDINARY_BUDGET), "{at:?}");
    assert_eq!(phases, [Phase::Checking], "the head is still being checked");
}

/// The store is the authority on whether a publication happened: a receipt
/// that later says the attempt failed, or a store that later reads as a
/// miss, does not undo it.
#[test]
fn an_observed_publication_outranks_a_later_failed_receipt() {
    // Our worker's head half was contended, so the head is another run's
    // attempt, still running when our receipt arrives.
    let fake = Fake::new(|t| {
        let outcome = (t >= ms(800)).then_some(Outcome::InSync);
        with(attempt(OTHER, Phase::Checking, outcome))
    })
    .head_lock(|t| t < ms(800))
    .pr_answer(|t| match t {
        t if t < ms(200) => Some(SEEDED),
        t if t < ms(400) => Some(PUBLISHED),
        _ => None,
    })
    .receipts(|t, _| {
        let failed = PrStatus::Failed { failure: PrFailure::CredentialsRejected { key: None } };
        (t >= ms(500)).then(|| receipt(OURS, HeadStatus::AdoptedElsewhere, failed))
    })
    .exits(&[Some(ms(500))]);

    let (end, _, at) = run(&fake, false, launching);

    assert_eq!(end, ended(in_sync(OTHER), PrEnd::Published));
    assert!(at >= ms(800) && at < ms(900), "{at:?}");
}

#[test]
fn the_receipt_reports_each_pr_status() {
    let rejected = PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) };
    for (status, expected) in [
        (PrStatus::Ok, PrEnd::Published),
        (PrStatus::Failed { failure: rejected.clone() }, PrEnd::Failed(rejected.clone())),
        (PrStatus::Ignored, PrEnd::Ignored),
        (PrStatus::Unsupported, PrEnd::Unsupported),
    ] {
        let fake = Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
            .receipts(receipt_at(ms(300), status.clone()))
            .exits(&[Some(ms(300))]);

        let (end, _, at) = run(&fake, false, launching);

        assert_eq!(end, ended(in_sync(OURS), expected), "{status:?}");
        assert!(at < ms(400), "{at:?}");
    }
}

#[test]
fn a_worker_that_exits_without_a_receipt_is_a_generic_pr_failure() {
    for force in [false, true] {
        let fake =
            Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync)))).exits(&[Some(ms(200))]);

        let (end, _, at) = run(&fake, force, launching);

        assert_eq!(end, ended(in_sync(OURS), other()), "no invented reason (force: {force})");
        assert!(at < ms(300), "a missing receipt is bounded: {at:?}");
    }
}

#[test]
fn a_worker_that_exits_without_a_receipt_keeps_a_new_publication() {
    let fake = Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
        .pr_answer(|t| (t >= ms(150)).then_some(PUBLISHED))
        .exits(&[Some(ms(200))]);

    let (end, _, _) = run(&fake, false, launching);

    assert_eq!(end, ended(in_sync(OURS), PrEnd::Published), "a first answer into an empty store counts");
}

#[test]
fn an_outcome_for_another_attempt_branch_or_origin_is_never_taken() {
    let other_id = attempt(OTHER, Phase::Checking, Some(Outcome::InSync));
    let other_branch = Attempt { branch: "trunk".into(), ..attempt(OURS, Phase::Checking, Some(Outcome::InSync)) };
    let other_origin =
        Attempt { origin_digest: "another".into(), ..attempt(OURS, Phase::Checking, Some(Outcome::InSync)) };
    let expired = Attempt { started_at: 0, ..attempt(OURS, Phase::Checking, Some(Outcome::InSync)) };
    for stored in [other_id, other_branch, other_origin, expired] {
        let fake = Fake::new(move |_| with(stored.clone()));

        let (end, phases, _) = run(&fake, false, launching);

        assert_eq!(end, timed_out(HeadEnd::Running { last: None }, PrEnd::Pending), "{:?}", fake.store());
        assert!(phases.is_empty());
    }
}

// Launch failures and adoption.

#[test]
fn a_contender_that_exits_early_is_never_a_finished_check() {
    // Another attempt finished before this run; ours exits without an
    // attempt or a receipt, and nothing holds the lock.
    for stored in [
        with(attempt(OTHER, Phase::Checking, Some(Outcome::InSync))),
        with(attempt(OTHER, Phase::Checking, None)),
        empty(),
    ] {
        let fake = Fake::new(move |_| stored.clone()).exits(&[Some(ms(30))]);

        let (end, _, at) = run(&fake, false, launching);

        assert_eq!(end, ended(HeadEnd::Unavailable, other()));
        assert!(at < ms(100), "rendered at once: {at:?}");
    }
}

#[test]
fn a_spawn_failure_is_unavailable_at_once() {
    let fake = Fake::new(|_| with(attempt(OTHER, Phase::Checking, None))).head_lock(|_| true);

    let (end, phases, at) = run(&fake, false, failing_launch);

    assert_eq!(end, ended(HeadEnd::Unavailable, other()));
    assert!(phases.is_empty());
    assert_eq!(at, Duration::ZERO, "no wait at all");
    assert!(discarded().is_empty(), "nothing was launched");
}

#[test]
fn a_worker_that_stops_mid_attempt_is_unavailable() {
    let fake = Fake::new(|t| if t < ms(40) { empty() } else { with(attempt(OURS, Phase::Checking, None)) })
        .exits(&[Some(ms(100))]);

    let (end, _, at) = run(&fake, false, launching);

    assert_eq!(end, ended(HeadEnd::Unavailable, other()));
    assert!(at < ms(200), "{at:?}");
}

#[test]
fn a_running_attempt_is_adopted_and_our_receipt_still_read() {
    let fake = Fake::new(|t| {
        let outcome = (t >= ms(900)).then_some(Outcome::InSync);
        with(attempt(OTHER, Phase::Checking, outcome))
    })
    .receipts(|t, for_attempt| {
        assert_eq!(for_attempt.id, OURS, "the receipt is looked up by our launched id");
        (t >= ms(20)).then(|| receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::Ok))
    })
    .exits(&[Some(ms(20))])
    .head_lock(|t| t < ms(900));

    let (end, phases, at) = run(&fake, false, launching);

    assert_eq!(end, ended(in_sync(OTHER), PrEnd::Published));
    assert_eq!(phases, [Phase::Checking]);
    assert!(at >= ms(900), "{at:?}");
    assert_eq!(launches().len(), 1);
    assert_eq!(fake.early_probes.get(), 0, "adoption probes only after our worker exited");
    assert_eq!(discarded(), [OURS], "only our own receipt, never the adopted run's");
}

#[test]
fn a_holder_that_finished_before_adoption_is_followed_only_on_our_receipts_word() {
    let finished_holder = attempt(OTHER, Phase::Fetching, Some(Outcome::Fetched));
    let rejected = PrFailure::RateLimited { authenticated: false, key: None };
    let stored = finished_holder.clone();
    let fake = Fake::new(move |_| with(stored.clone()))
        .receipts(move |t, _| {
            (t >= ms(20)).then(|| receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::Failed { failure: rejected.clone() }))
        })
        .exits(&[Some(ms(20))]);

    let (end, _, _) = run(&fake, false, launching);

    let rejected = PrFailure::RateLimited { authenticated: false, key: None };
    assert_eq!(end, ended(HeadEnd::Finished(finished_holder), PrEnd::Failed(rejected)));
}

#[test]
fn an_ordinary_wait_never_relaunches_for_another_branchs_holder() {
    let trunk = Attempt { branch: "trunk".into(), ..attempt(OTHER, Phase::Checking, None) };
    let fake = Fake::new(move |_| with(trunk.clone()))
        .receipts(|t, _| (t >= ms(20)).then(|| receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::Ok)))
        .head_lock(|_| true)
        .exits(&[Some(ms(20))]);

    let (end, _, _) = run(&fake, false, launching);

    assert_eq!(end, ended(HeadEnd::Unavailable, PrEnd::Published));
    assert_eq!(launches().len(), 1);
}

#[test]
fn a_forced_wait_relaunches_after_a_holder_for_another_branch_finishes() {
    let trunk = Attempt { branch: "trunk".into(), ..attempt(OTHER, Phase::Checking, None) };
    let fake = Fake::new(move |t| {
        if t < ms(1_000) {
            with(trunk.clone())
        } else {
            with(attempt(SECOND, Phase::Checking, Some(Outcome::InSync)))
        }
    })
    .receipts(|_, for_attempt| {
        Some(match for_attempt.id.as_str() {
            OURS => receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::Ok),
            _ => receipt(SECOND, HeadStatus::Ok, PrStatus::Ok),
        })
    })
    .head_lock(|t| t < ms(800))
    .exits(&[Some(ms(20)), Some(ms(1_100))]);

    let (end, _, at) = run(&fake, true, launching);

    assert_eq!(launches(), [launched(OURS), launched(SECOND)]);
    assert_eq!(end, ended(in_sync(SECOND), PrEnd::Published));
    assert!(at >= ms(1_000), "{at:?}");
    assert_eq!(discarded(), [OURS, SECOND], "a retry's receipt is discarded too");
}

#[test]
fn a_forced_wait_adopts_a_matching_attempt_its_worker_reported() {
    let fake = Fake::new(|t| {
        let outcome = (t >= ms(700)).then_some(Outcome::Fetched);
        with(attempt(OTHER, Phase::Fetching, outcome))
    })
    .receipts(|_, _| Some(receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::Ok)))
    .head_lock(|t| t < ms(700))
    .exits(&[Some(ms(20))]);

    let (end, _, _) = run(&fake, true, launching);

    assert_eq!(end, ended(finished(OTHER, Phase::Fetching, Outcome::Fetched), PrEnd::Published));
    assert_eq!(launches().len(), 1);
}

#[test]
fn a_forced_wait_follows_the_outcome_to_the_receipt_past_the_ordinary_budget() {
    let fake = Fake::new(|t| {
        let outcome = (t >= ms(100)).then_some(Outcome::InSync);
        with(attempt(OURS, Phase::Checking, outcome))
    })
    .receipts(receipt_at(ms(5_000), PrStatus::Ok))
    .exits(&[Some(ms(5_000))]);

    let (end, _, at) = run(&fake, true, launching);

    assert_eq!(end, ended(in_sync(OURS), PrEnd::Published));
    assert!(at >= ms(5_000) && at > ORDINARY_BUDGET, "the 3 s limit does not apply: {at:?}");
    assert_eq!(launches(), [launched(OURS)]);
    assert_eq!(discarded(), [OURS]);
}

#[test]
fn an_attempt_that_fails_its_fetch_still_waits_for_the_receipt() {
    for force in [false, true] {
        let failed = Outcome::FetchFailed { reason: FetchFailure::Timeout };
        let fake = Fake::new(move |_| with(attempt(OURS, Phase::Fetching, Some(failed))))
            .receipts(|t, _| (t >= ms(300)).then(|| receipt(OURS, HeadStatus::Failed, PrStatus::Ok)))
            .exits(&[Some(ms(300))]);

        let (end, _, at) = run(&fake, force, launching);

        assert_eq!(end, ended(finished(OURS, Phase::Fetching, failed), PrEnd::Published), "force: {force}");
        assert!(at >= ms(300), "{at:?}");
    }
}

// PR contention.

#[test]
fn an_ordinary_contended_pr_half_waits_for_the_lock_and_fails_without_a_new_answer() {
    let fake = Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
        .receipts(receipt_at(ms(300), PrStatus::Contended))
        .pr_lock(|t| t < ms(1_000))
        .pr_answer(|_| Some(SEEDED))
        .exits(&[Some(ms(300))]);

    let (end, _, at) = run(&fake, false, launching);

    assert_eq!(end, ended(in_sync(OURS), other()), "a released lock alone proves nothing");
    assert!(at >= ms(1_000) && at < ms(1_100), "the holder's lock was waited for: {at:?}");
    assert_eq!(launches().len(), 1, "an ordinary wait never relaunches");
    assert!(
        fake.pr_probes.borrow().iter().all(|&probe| probe >= ms(300)),
        "the PR lock is probed only once our receipt reports contention: {:?}",
        fake.pr_probes.borrow()
    );
}

#[test]
fn an_ordinary_contended_pr_half_takes_the_holders_answer() {
    // Same-second and first answers carry only a new id.
    for (before, after) in [(Some(SEEDED), PUBLISHED), (None, PUBLISHED)] {
        let fake = Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
            .receipts(receipt_at(ms(10), PrStatus::Contended))
            .pr_lock(|t| t < ms(500))
            .pr_answer(move |t| if t < ms(500) { before } else { Some(after) })
            .exits(&[Some(ms(10))]);

        let (end, _, _) = run(&fake, false, launching);

        assert_eq!(end, ended(in_sync(OURS), PrEnd::Published), "{before:?}");
        assert_eq!(launches().len(), 1);
    }
}

#[test]
fn contention_never_extends_the_ordinary_budget() {
    let fake = Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
        .receipts(receipt_at(ms(300), PrStatus::Contended))
        .pr_lock(|_| true)
        .pr_answer(|_| Some(SEEDED))
        .exits(&[Some(ms(300))]);

    let (end, _, at) = run(&fake, false, launching);

    assert_eq!(end, timed_out(in_sync(OURS), PrEnd::Pending), "the holder is still asking");
    assert!(within_the_budget(at, ORDINARY_BUDGET), "{at:?}");
}

#[test]
fn a_forced_wait_accepts_the_answer_a_contending_holder_published() {
    let fake = Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
        .receipts(|_, _| Some(receipt(OURS, HeadStatus::Ok, PrStatus::Contended)))
        .pr_lock(|t| t < ms(2_000))
        .pr_answer(|t| Some(if t < ms(2_000) { SEEDED } else { PUBLISHED }))
        .exits(&[Some(ms(10))]);

    let (end, _, at) = run(&fake, true, launching);

    assert_eq!(end, ended(in_sync(OURS), PrEnd::Published));
    assert!(at >= ms(2_000), "the holder's lock was waited for: {at:?}");
    assert_eq!(launches().len(), 1, "a published answer needs no second request");
}

/// The holder's request failed, so the store kept the answer (and
/// publication id) it had at launch: the run asks again itself.
#[test]
fn a_forced_wait_relaunches_once_when_a_contending_holder_published_nothing() {
    let fake = Fake::new(|t| {
        let id = if t < ms(2_000) { OURS } else { SECOND };
        with(attempt(id, Phase::Checking, Some(Outcome::InSync)))
    })
    .receipts(|_, for_attempt| {
        Some(match for_attempt.id.as_str() {
            OURS => receipt(OURS, HeadStatus::Ok, PrStatus::Contended),
            _ => receipt(SECOND, HeadStatus::Ok, PrStatus::Ok),
        })
    })
    .pr_lock(|t| t < ms(2_000))
    .pr_answer(|_| Some(SEEDED))
    .exits(&[Some(ms(10)), Some(ms(2_100))]);

    let (end, _, at) = run(&fake, true, launching);

    assert_eq!(end, ended(in_sync(SECOND), PrEnd::Published));
    assert!(at >= ms(2_000), "{at:?}");
    assert_eq!(launches(), [launched(OURS), launched(SECOND)]);
    assert_eq!(discarded(), [OURS, SECOND], "each launched attempt's receipt is discarded once the wait ends");
}

#[test]
fn a_forced_wait_reports_a_pr_failure_when_the_relaunch_is_contended_too() {
    let fake = Fake::new(|t| {
        let id = if t < ms(1_000) { OURS } else { SECOND };
        with(attempt(id, Phase::Checking, Some(Outcome::InSync)))
    })
    .receipts(|t, for_attempt| {
        let written = for_attempt.id == OURS || t >= ms(1_100);
        written.then(|| receipt(&for_attempt.id, HeadStatus::Ok, PrStatus::Contended))
    })
    .pr_lock(|t| t < ms(1_000) || (ms(1_050)..ms(3_000)).contains(&t))
    .exits(&[Some(ms(10)), Some(ms(1_100))]);

    let (end, _, at) = run(&fake, true, launching);

    assert_eq!(end, ended(in_sync(SECOND), other()));
    assert!(at >= ms(3_000), "the second holder was waited for too: {at:?}");
    assert_eq!(launches().len(), 2, "one relaunch, no more");
    assert!(
        fake.pr_probes.borrow().iter().all(|&probe| probe <= ms(1_000) || probe >= ms(1_100)),
        "no probe while the relaunched worker had not reported: {:?}",
        fake.pr_probes.borrow()
    );
}

#[test]
fn a_forced_relaunch_shares_the_original_budget() {
    let fake = Fake::new(|t| {
        let id = if t < ms(1_000) { OURS } else { SECOND };
        with(attempt(id, Phase::Checking, Some(Outcome::InSync)))
    })
    .receipts(|_, for_attempt| (for_attempt.id == OURS).then(|| receipt(OURS, HeadStatus::Ok, PrStatus::Contended)))
    .pr_lock(|t| t < ms(1_000))
    .pr_answer(|_| Some(SEEDED))
    .exits(&[Some(ms(10)), None]);

    let (end, _, at) = run(&fake, true, launching);

    assert_eq!(end, timed_out(in_sync(SECOND), PrEnd::Pending));
    assert!(within_the_budget(at, FORCED_BUDGET), "the relaunch did not restart the clock: {at:?}");
    assert_eq!(launches().len(), 2);
}

/// Real PR stores for the contended-holder rule: a repository whose
/// `origin` the store binds to, a store seeded before launch, and launch
/// stubs that act as a holder finishing before our worker's receipt.
mod stored_prs {
    use std::process::Command;

    use worktree::pull_requests::{OpenPrSource, OpenPullRequest, PrRequestError, RefreshOutcome, origin_digest, refresh};

    use super::*;

    pub(super) const ORIGIN: &str = "https://prs.example.invalid/owner/repo.git";

    thread_local! {
        /// The store and repository the launch stubs write through, and what
        /// the store holds after the holder.
        static REAL: RefCell<Option<(PathBuf, PathBuf)>> = const { RefCell::new(None) };
        static AFTER: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    pub(super) struct Repo {
        _dir: tempfile::TempDir,
        pub(super) store: PathBuf,
    }

    /// A repository whose store holds `SEEDED`, stored at `fetched_at`.
    pub(super) fn repo(fetched_at: u64) -> Repo {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("repo");
        std::fs::create_dir_all(&root).expect("repo dir");
        for args in [&["init", "-q"][..], &["remote", "add", "origin", ORIGIN]] {
            let status = Command::new("git").arg("-C").arg(&root).args(args).status().expect("git runs");
            assert!(status.success(), "git {args:?}");
        }
        let store = dir.path().join("cache").join("abc.prs.json");
        std::fs::create_dir_all(store.parent().unwrap()).unwrap();
        std::fs::write(&store, store_json(SEEDED, fetched_at, &origin_digest(ORIGIN), "[]")).unwrap();
        REAL.with(|real| *real.borrow_mut() = Some((store.clone(), root)));
        Repo { _dir: dir, store }
    }

    /// A format-5 store as the worker writes it.
    pub(super) fn store_json(publication: &str, fetched_at: u64, digest: &str, pull_requests: &str) -> String {
        format!(
            r#"{{"format_version":5,"origin_digest":"{digest}","fetched_at":{fetched_at},"publication":"{publication}","source_repo":"owner/repo","pull_requests":{pull_requests}}}"#
        )
    }

    /// The holder leaves the store holding `contents`.
    pub(super) fn after_the_holder(contents: String) {
        AFTER.with(|after| *after.borrow_mut() = Some(contents));
    }

    pub(super) fn holder_writes(main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        let (store, _) = REAL.with(|real| real.borrow().clone()).expect("stored_prs::repo()");
        if let Some(contents) = AFTER.with(|after| after.borrow().clone()) {
            std::fs::write(&store, contents).unwrap();
        }
        launching(main, args)
    }

    struct Source(Result<u64, PrFailure>);

    impl OpenPrSource for Source {
        fn source_repo(&self) -> Option<String> {
            Some("owner/repo".into())
        }
        fn fetch(&self) -> Result<Vec<OpenPullRequest>, PrRequestError> {
            self.0.clone().map_err(PrRequestError::from).map(|number| {
                vec![OpenPullRequest {
                    number,
                    url: None,
                    source_repo: Some("owner/repo".into()),
                    source_branch: "feat".into(),
                    target_branch: "main".into(),
                }]
            })
        }
    }

    /// The real `refresh` as a holder, answering `answer`, then our launch.
    fn holder_refreshes(main: &Path, args: &LaunchArgs, answer: Result<u64, PrFailure>) -> std::io::Result<WorkerHandle> {
        let (store, root) = REAL.with(|real| real.borrow().clone()).expect("stored_prs::repo()");
        let outcome = refresh(&store, &root, unix_now, |_| Box::new(Source(answer)) as Box<dyn OpenPrSource>);
        assert!(matches!(outcome, RefreshOutcome::Refreshed | RefreshOutcome::Failed(_)), "{outcome:?}");
        launching(main, args)
    }

    pub(super) fn holder_fails(main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        holder_refreshes(main, args, Err(PrFailure::Other))
    }

    pub(super) fn holder_answers(main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        holder_refreshes(main, args, Ok(2))
    }
}

fn contended_after(fake: Fake) -> Fake {
    fake.receipts(|_, for_attempt| Some(receipt(&for_attempt.id, HeadStatus::Ok, PrStatus::Contended)))
}

#[test]
fn a_contending_holders_real_refresh_is_followed_by_its_publication_id() {
    for force in [false, true] {
        let repo = stored_prs::repo(unix_now());
        let fake = contended_after(Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync)))))
            .real_pr_store(&repo.store, stored_prs::ORIGIN)
            .exits(&[Some(ms(10))]);

        let (end, _, _) = run(&fake, force, stored_prs::holder_answers);

        assert_eq!(end, ended(in_sync(OURS), PrEnd::Published), "force: {force}");
        assert_eq!(launches().len(), 1, "the holder's answer needs no second request");
    }
}

#[test]
fn a_contending_holders_failed_real_refresh_is_never_a_publication() {
    let repo = stored_prs::repo(unix_now());
    let fake = contended_after(Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync)))))
        .real_pr_store(&repo.store, stored_prs::ORIGIN)
        .exits(&[Some(ms(10))]);

    let (end, _, _) = run(&fake, false, stored_prs::holder_fails);

    assert_eq!(end, ended(in_sync(OURS), other()));

    let repo = stored_prs::repo(unix_now());
    let fake = contended_after(Fake::new(|_| latest_launch_in_sync()))
        .real_pr_store(&repo.store, stored_prs::ORIGIN)
        .exits(&[Some(ms(10)), Some(ms(10))]);

    let (end, _, _) = run(&fake, true, stored_prs::holder_fails);

    assert_eq!(end, ended(in_sync(SECOND), other()), "forced: one relaunch, then a failure");
    assert_eq!(launches().len(), 2);
}

/// What a holder leaves in the store, and whether that is a publication
/// this run may take: a new id counts even within the same second or for an
/// empty answer; an unchanged id, an unreadable store, a future date, and
/// another origin never do.
#[test]
fn only_a_new_usable_publication_id_counts_for_a_contended_half() {
    use worktree::pull_requests::origin_digest;

    // In the past, so a later stamp is not in the future.
    let seeded_at = unix_now() - 10;
    let ours = origin_digest(stored_prs::ORIGIN);
    let one_pr = r#"[{"number":2,"url":null,"source_repo":"owner/repo","source_branch":"feat","target_branch":"main"}]"#;
    let cells: Vec<(&str, String, bool)> = vec![
        ("control: a new id", stored_prs::store_json(PUBLISHED, seeded_at + 1, &ours, one_pr), true),
        ("a new id in the same second", stored_prs::store_json(PUBLISHED, seeded_at, &ours, one_pr), true),
        ("a new id for an empty answer", stored_prs::store_json(PUBLISHED, seeded_at, &ours, "[]"), true),
        ("the same id rewritten", stored_prs::store_json(SEEDED, seeded_at + 1, &ours, one_pr), false),
        ("corrupt", "{not json".to_string(), false),
        ("future-dated", stored_prs::store_json(PUBLISHED, seeded_at + 3_600, &ours, one_pr), false),
        ("another origin", stored_prs::store_json(PUBLISHED, seeded_at, &origin_digest("https://elsewhere.invalid/x.git"), one_pr), false),
        ("an older format", stored_prs::store_json(PUBLISHED, seeded_at, &ours, one_pr).replace(r#""format_version":5"#, r#""format_version":4"#), false),
    ];
    for (label, contents, counts) in cells {
        let repo = stored_prs::repo(seeded_at);
        stored_prs::after_the_holder(contents);
        let fake = contended_after(Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync)))))
            .real_pr_store(&repo.store, stored_prs::ORIGIN)
            .exits(&[Some(ms(10))]);

        let (end, _, _) = run(&fake, false, stored_prs::holder_writes);

        let expected = if counts { PrEnd::Published } else { other() };
        assert_eq!(end, ended(in_sync(OURS), expected), "{label}");
    }
}

/// The attempt of the latest launch, finished in sync.
fn latest_launch_in_sync() -> StoreState {
    let id = launches().last().map_or_else(|| OURS.to_string(), |args| args.attempt.clone());
    with(attempt(&id, Phase::Checking, Some(Outcome::InSync)))
}

/// Receipts over the real store files: a receipt for another id, origin, or
/// branch, an older one, or a malformed file is missing, and a missing
/// receipt is a bounded generic failure.
mod receipt_files {
    use worktree::pull_requests::origin_digest;
    use worktree::remote_head::{begin_attempt, finish_attempt, receipt_path_beside, write_receipt};

    use super::*;

    pub(super) const ORIGIN: &str = "https://github.com/owner/repo.git";

    /// Spoils the receipt our worker writes, or writes its file itself.
    pub(super) type Spoil = fn(&mut Receipt, &Path);

    thread_local! {
        static HEAD: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
        /// How the stub worker spoils its receipt.
        static SPOIL: RefCell<Option<Spoil>> = const { RefCell::new(None) };
    }

    fn head() -> PathBuf {
        HEAD.with(|head| head.borrow().clone()).expect("receipt_files::run")
    }

    /// Our worker: checks in sync and writes its receipt, as `SPOIL` says.
    fn worker(_main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        let head = head();
        let attempt = Attempt::begin(args.attempt.clone(), origin_digest(ORIGIN), BRANCH.into(), unix_now());
        begin_attempt(&head, &attempt).expect("attempt");
        finish_attempt(&head, &args.attempt, Outcome::InSync, None).expect("outcome");
        let path = receipt_path_beside(&head, &args.attempt).expect("path");
        let mut receipt = Receipt {
            attempt_id: args.attempt.clone(),
            origin_digest: origin_digest(ORIGIN),
            branch: BRANCH.into(),
            finished_at: unix_now(),
            head: HeadStatus::Ok,
            prs: PrStatus::Failed { failure: PrFailure::CredentialsRejected { key: None } },
        };
        let spoil = SPOIL.with(|spoil| *spoil.borrow());
        if let Some(spoil) = spoil {
            spoil(&mut receipt, &path);
        }
        if !path.exists() {
            write_receipt(&path, &receipt).expect("receipt");
        }
        Ok(WorkerHandle::new(|| true))
    }

    /// Runs one ordinary wait over real stores; returns its end and the
    /// receipts left behind.
    pub(super) fn run(spoil: Option<Spoil>) -> (WaitEnd, Vec<String>) {
        let dir = tempfile::tempdir().expect("temp dir");
        let head = dir.path().join("abc.remote-head.json");
        HEAD.with(|stored| *stored.borrow_mut() = Some(head.clone()));
        SPOIL.with(|stored| *stored.borrow_mut() = spoil);
        let digest = origin_digest(ORIGIN);
        let request = WaitRequest { main: Path::new("/repo"), origin_digest: &digest, branch: BRANCH, force: false, budget: ORDINARY_BUDGET };
        let env = StoreEnv::new(head, dir.path().join("abc.prs.json"), ORIGIN.into());
        let end = wait(request, &env, worker, &mut |_| {});
        let left = std::fs::read_dir(dir.path())
            .expect("dir")
            .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains("receipt"))
            .collect();
        (end, left)
    }
}

#[test]
fn a_real_receipt_is_read_and_discarded_by_an_ordinary_wait() {
    let (end, left) = receipt_files::run(None);

    let PrEnd::Failed(failure) = &end.prs else {
        panic!("the receipt's failure: {end:?}");
    };
    assert_eq!(failure, &PrFailure::CredentialsRejected { key: None });
    assert!(matches!(end.head, HeadEnd::Finished(_)), "{end:?}");
    assert!(left.is_empty(), "the wait deletes the receipt it launched: {left:?}");
}

#[test]
fn a_receipt_for_another_attempt_or_a_malformed_one_is_missing() {
    let spoilers: [(&str, receipt_files::Spoil); 5] = [
        ("another origin", |receipt, _| receipt.origin_digest = "another".into()),
        ("another branch", |receipt, _| receipt.branch = "trunk".into()),
        ("another attempt id", |receipt, _| receipt.attempt_id = OTHER.into()),
        ("finished before the attempt began", |receipt, _| receipt.finished_at = 1),
        ("malformed", |_, path| std::fs::write(path, "{\"format_version\": 1, \"attempt_id\":").unwrap()),
    ];
    for (label, spoil) in spoilers {
        let (end, _) = receipt_files::run(Some(spoil));

        assert_eq!(end.prs, other(), "{label}: a generic failure, never the receipt's reason");
        assert!(matches!(end.head, HeadEnd::Finished(_)), "{label}: the head outcome is kept: {end:?}");
        assert!(!end.timed_out, "{label}: bounded by the worker's exit");
    }
}

/// Two overlapping forced runs over real stores: run A's worker holds the
/// head lock and records a PR credentials failure; run B, launched while A's
/// receipt waits to be read, is contended, follows A's attempt, and writes
/// its own receipt after A's. With one receipt file per repository B's write
/// replaced A's, and A lost its PR failure line.
mod overlapping_runs {
    use worktree::pull_requests::origin_digest;
    use worktree::remote_head::{begin_attempt, finish_attempt, receipt_path_beside, write_receipt};

    use super::*;

    pub(super) const ORIGIN: &str = "https://github.com/owner/repo.git";

    thread_local! {
        /// The remote-head store and PR store, and run B's end.
        static STORES: RefCell<Option<(PathBuf, PathBuf)>> = const { RefCell::new(None) };
        static B_END: RefCell<Option<WaitEnd>> = const { RefCell::new(None) };
    }

    pub(super) fn credentials_rejected() -> PrFailure {
        PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) }
    }

    fn stores() -> (PathBuf, PathBuf) {
        STORES.with(|stores| stores.borrow().clone()).expect("overlapping_runs::run")
    }

    fn receipt_for(id: &str, head: HeadStatus, prs: PrStatus) -> Receipt {
        Receipt {
            attempt_id: id.into(),
            origin_digest: origin_digest(ORIGIN),
            branch: BRANCH.into(),
            finished_at: unix_now(),
            head,
            prs,
        }
    }

    fn forced(digest: &str) -> WaitRequest<'_> {
        WaitRequest { main: Path::new("/repo"), origin_digest: digest, branch: BRANCH, force: true, budget: FORCED_BUDGET }
    }

    /// Run A's worker: checks in sync, fails its PR half, writes its receipt;
    /// then run B happens before run A reads anything.
    fn run_a_worker(_main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        let (head, prs) = stores();
        let attempt = Attempt::begin(args.attempt.clone(), origin_digest(ORIGIN), BRANCH.into(), unix_now());
        begin_attempt(&head, &attempt).expect("attempt");
        finish_attempt(&head, &args.attempt, Outcome::InSync, None).expect("outcome");
        let failed = PrStatus::Failed { failure: credentials_rejected() };
        write_receipt(&receipt_path_beside(&head, &args.attempt).expect("path"), &receipt_for(&args.attempt, HeadStatus::Ok, failed))
            .expect("receipt A");

        let digest = origin_digest(ORIGIN);
        let env = StoreEnv::new(head, prs, ORIGIN.into());
        let b = wait(forced(&digest), &env, run_b_worker, &mut |_| {});
        B_END.with(|end| *end.borrow_mut() = Some(b));
        Ok(WorkerHandle::new(|| true))
    }

    /// Run B's worker: the head lock was held (by A), its PR half ran after
    /// A's released the PR lock, and it writes its receipt after A's.
    fn run_b_worker(_main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        let (head, _) = stores();
        let receipt = receipt_for(&args.attempt, HeadStatus::AdoptedElsewhere, PrStatus::Ok);
        write_receipt(&receipt_path_beside(&head, &args.attempt).expect("path"), &receipt).expect("receipt B");
        Ok(WorkerHandle::new(|| true))
    }

    /// Runs A (and B inside it); returns both ends and the files left.
    pub(super) fn run() -> (WaitEnd, WaitEnd, Vec<String>) {
        let dir = tempfile::tempdir().expect("temp dir");
        let head = dir.path().join("abc.remote-head.json");
        let prs = dir.path().join("abc.prs.json");
        STORES.with(|stores| *stores.borrow_mut() = Some((head.clone(), prs.clone())));

        let digest = origin_digest(ORIGIN);
        let a = wait(forced(&digest), &StoreEnv::new(head, prs, ORIGIN.into()), run_a_worker, &mut |_| {});

        let b = B_END.with(|end| end.borrow_mut().take()).expect("run B ran");
        let left = std::fs::read_dir(dir.path())
            .expect("dir")
            .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains("receipt"))
            .collect();
        (a, b, left)
    }
}

#[test]
fn overlapping_forced_runs_each_read_their_own_receipt() {
    let (a, b, left) = overlapping_runs::run();

    let HeadEnd::Finished(followed_by_a) = &a.head else {
        panic!("run A followed no attempt: {a:?}");
    };
    assert_eq!(a.prs, PrEnd::Failed(overlapping_runs::credentials_rejected()));
    assert!(
        super::super::credential_line(overlapping_runs::ORIGIN, None, Some(&overlapping_runs::credentials_rejected()))
            .is_some(),
        "run A still gets its PR failure line"
    );

    let HeadEnd::Finished(followed_by_b) = &b.head else {
        panic!("run B followed no attempt: {b:?}");
    };
    assert_eq!(b.prs, PrEnd::Published, "B's own receipt, not A's failure");
    assert_eq!(followed_by_b.id, followed_by_a.id, "B followed A's attempt");

    assert!(left.is_empty(), "each run deletes its own receipt: {left:?}");
}

#[test]
fn the_spinner_text_follows_the_phase() {
    assert_eq!(phase_text(Phase::Checking), "updating");
    assert_eq!(
        phase_text(Phase::CheckingFallback { reason: FallbackReason::NoKey }),
        "no API key, using fallback method"
    );
    assert_eq!(
        phase_text(Phase::CheckingFallback { reason: FallbackReason::NotVisible }),
        "no API key, using fallback method"
    );
    assert_eq!(
        phase_text(Phase::CheckingFallback { reason: FallbackReason::RateLimited }),
        "rate limited, using fallback method"
    );
    assert_eq!(phase_text(Phase::CheckingFallback { reason: FallbackReason::Rejected }), "updating");
    assert_eq!(phase_text(Phase::CheckingFallback { reason: FallbackReason::Other }), "updating");
    assert_eq!(phase_text(Phase::Fetching), "pulling remote updates");
}

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn the_spinner_writes_nothing_when_its_output_is_not_a_terminal() {
    let captured = Captured::default();
    let progress = Progress::on(captured.clone(), false);
    for phase in [Phase::Checking, Phase::Fetching] {
        progress.show(phase);
    }
    std::thread::sleep(SPINNER_DELAY + ms(100));
    progress.finish();

    assert!(captured.0.lock().unwrap().is_empty());
}

#[test]
fn the_spinner_draws_the_phase_text_and_clears_its_line_on_a_terminal() {
    let captured = Captured::default();
    let progress = Progress::on(captured.clone(), true);
    progress.show(Phase::Fetching);
    std::thread::sleep(SPINNER_DELAY + ms(200));
    progress.finish();

    let written = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(written.contains("pulling remote updates"), "{written:?}");
    assert!(written.ends_with(biscuit_terminal::components::spinner::CLEAR_LINE), "{written:?}");
}

#[test]
fn a_launched_process_reports_its_exit() {
    let child = std::process::Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("git");
    let mut handle = WorkerHandle::from_child(child);
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !handle.has_exited() {
        assert!(std::time::Instant::now() < deadline, "git --version never exited");
        std::thread::sleep(ms(10));
    }
    assert!(handle.has_exited(), "exit stays reported");
}

