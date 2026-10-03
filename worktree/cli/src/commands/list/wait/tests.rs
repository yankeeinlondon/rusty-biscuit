//! The wait protocol over a scripted store, receipt, and locks, and a fake
//! clock that advances only when the wait sleeps.

use std::cell::{Cell, RefCell};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use worktree::remote_head::{
    Attempt, CheckFailure, FallbackReason, FetchFailure, HeadStatus, Outcome, Phase, PrFailure, PrStatus, Receipt, StoreState,
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
    /// Every receipt the wait discarded, in order, with when.
    static DISCARDED: RefCell<Vec<(String, Duration)>> = const { RefCell::new(Vec::new()) };
}

fn discarded() -> Vec<String> {
    DISCARDED.with(|discarded| discarded.borrow().iter().map(|(id, _)| id.clone()).collect())
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
        DISCARDED.with(|discarded| discarded.borrow_mut().push((attempt_id.to_string(), now())));
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

// Retries keep what the other half already established.

/// Launches the first worker as `EXITS` says; every later launch fails.
fn launching_once(main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
    if launches().is_empty() { launching(main, args) } else { failing_launch(main, args) }
}

/// A forced wait whose first attempt finishes its head check while its PR
/// half is contended by a holder that publishes nothing by 1 s, so it asks
/// for a PR retry; the second attempt's head is `second` from 1 s on.
fn pr_retry(second: Option<Attempt>) -> Fake {
    Fake::new(move |t| match &second {
        Some(second) if t >= ms(1_000) => with(second.clone()),
        _ if t >= ms(1_000) => empty(),
        _ => with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))),
    })
    .receipts(|_, for_attempt| (for_attempt.id == OURS).then(|| receipt(OURS, HeadStatus::Ok, PrStatus::Contended)))
    .pr_lock(|t| t < ms(1_000))
    .pr_answer(|_| Some(SEEDED))
}

#[test]
fn a_pr_retry_that_cannot_launch_keeps_the_finished_head() {
    let fake = pr_retry(None).exits(&[Some(ms(10))]);

    let (end, _, at) = run(&fake, true, launching_once);

    assert_eq!(end, ended(in_sync(OURS), other()), "the holder published nothing; the head check stands");
    assert_eq!(launches(), [launched(OURS), launched(SECOND)], "the retry was tried");
    assert!(at >= ms(1_000) && at < ms(1_100), "{at:?}");
    assert_eq!(discarded(), [OURS], "only the launched attempt's receipt");
}

#[test]
fn a_pr_retry_without_an_attempt_id_keeps_the_finished_head() {
    let fake = pr_retry(None).exits(&[Some(ms(10))]);
    *fake.ids.borrow_mut() = vec![OURS];

    let (end, _, _) = run(&fake, true, launching);

    assert_eq!(end, ended(in_sync(OURS), other()));
    assert_eq!(launches(), [launched(OURS)]);
}

#[test]
fn a_pr_retry_whose_head_does_not_finish_keeps_the_finished_head() {
    // Stops mid-attempt, exits without recording one, or is still checking
    // at the budget: none of these is newer evidence than a finished check.
    let checking = attempt(SECOND, Phase::Checking, None);
    for (second, exit, expected) in [
        (Some(checking.clone()), Some(ms(1_500)), ended(in_sync(OURS), other())),
        (None, Some(ms(1_500)), ended(in_sync(OURS), other())),
        (Some(checking), None, timed_out(in_sync(OURS), PrEnd::Pending)),
    ] {
        let fake = pr_retry(second.clone()).exits(&[Some(ms(10)), exit]);

        let (end, _, _) = run(&fake, true, launching);

        assert_eq!(end, expected, "{second:?}, {exit:?}");
        assert_eq!(launches().len(), 2);
    }
}

#[test]
fn a_pr_retrys_finished_head_supersedes_the_retained_one() {
    let fetched = attempt(SECOND, Phase::Fetching, Some(Outcome::Fetched));
    let fake = pr_retry(Some(fetched)).exits(&[Some(ms(10)), Some(ms(1_500))]);

    let (end, _, _) = run(&fake, true, launching);

    assert_eq!(end, ended(finished(SECOND, Phase::Fetching, Outcome::Fetched), other()));
}

fn rejected() -> PrFailure {
    PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) }
}

/// A forced wait whose first worker finds the head lock held for another
/// branch until 800 ms and reports `first_prs`; the replacement's head is
/// in sync from 1 s on and its receipt is `second_prs`, written at 1.1 s.
fn head_retry(first_prs: PrStatus, second_prs: Option<PrStatus>) -> Fake {
    let trunk = Attempt { branch: "trunk".into(), ..attempt(OTHER, Phase::Checking, None) };
    Fake::new(move |t| {
        if t < ms(1_000) { with(trunk.clone()) } else { with(attempt(SECOND, Phase::Checking, Some(Outcome::InSync))) }
    })
    .receipts(move |t, for_attempt| match for_attempt.id.as_str() {
        OURS => Some(receipt(OURS, HeadStatus::AdoptedElsewhere, first_prs.clone())),
        _ if t >= ms(1_100) => second_prs.clone().map(|prs| receipt(SECOND, HeadStatus::Ok, prs)),
        _ => None,
    })
    .head_lock(|t| t < ms(800))
}

#[test]
fn a_head_retry_keeps_the_first_receipts_pr_diagnosis() {
    let failed = PrStatus::Failed { failure: rejected() };

    // The replacement exits without a receipt.
    let fake = head_retry(failed.clone(), None).exits(&[Some(ms(20)), Some(ms(1_100))]);
    let (end, _, _) = run(&fake, true, launching);
    assert_eq!(end, ended(in_sync(SECOND), PrEnd::Failed(rejected())), "never a generic failure");
    assert_eq!(launches().len(), 2);

    // The replacement cannot be launched, or has no attempt id.
    let fake = head_retry(failed.clone(), None).exits(&[Some(ms(20))]);
    let (end, _, _) = run(&fake, true, launching_once);
    assert_eq!(end, ended(HeadEnd::Unavailable, PrEnd::Failed(rejected())), "launch error");

    let fake = head_retry(failed.clone(), None).exits(&[Some(ms(20))]);
    *fake.ids.borrow_mut() = vec![OURS];
    let (end, _, _) = run(&fake, true, launching);
    assert_eq!(end, ended(HeadEnd::Unavailable, PrEnd::Failed(rejected())), "no attempt id");

    // The replacement is still running at the budget.
    let fake = head_retry(failed, None).exits(&[Some(ms(20)), None]);
    let (end, _, _) = run(&fake, true, launching);
    assert_eq!(end, timed_out(in_sync(SECOND), PrEnd::Failed(rejected())), "not pending");
}

#[test]
fn a_head_retrys_own_pr_result_supersedes_the_retained_one() {
    let limited = PrFailure::RateLimited { authenticated: true, key: Some("GITHUB_TOKEN".into()) };
    for (second, expected) in [
        (PrStatus::Ok, PrEnd::Published),
        (PrStatus::Failed { failure: limited.clone() }, PrEnd::Failed(limited.clone())),
        (PrStatus::Ignored, PrEnd::Ignored),
    ] {
        let fake = head_retry(PrStatus::Failed { failure: rejected() }, Some(second.clone()))
            .exits(&[Some(ms(20)), Some(ms(1_100))]);

        let (end, _, _) = run(&fake, true, launching);

        assert_eq!(end, ended(in_sync(SECOND), expected), "{second:?}");
    }
}

#[test]
fn a_head_retry_never_undoes_the_first_receipts_success() {
    let fake = head_retry(PrStatus::Ok, Some(PrStatus::Failed { failure: rejected() }))
        .exits(&[Some(ms(20)), Some(ms(1_100))]);

    let (end, _, _) = run(&fake, true, launching);

    assert_eq!(end, ended(in_sync(SECOND), PrEnd::Published), "a publication outranks a later failed receipt");
}

#[test]
fn a_head_retry_after_pr_contention_is_the_one_pr_retry() {
    let fake = head_retry(PrStatus::Contended, Some(PrStatus::Contended))
        .pr_answer(|_| Some(SEEDED))
        .exits(&[Some(ms(20)), Some(ms(1_100))]);

    let (end, _, _) = run(&fake, true, launching);

    assert_eq!(end, ended(in_sync(SECOND), other()), "a second contention is a generic failure");
    assert_eq!(launches().len(), 2, "no third launch");
}

#[test]
fn a_first_launch_without_an_attempt_id_is_unavailable_and_generic() {
    let fake = Fake::new(|_| empty());
    fake.ids.borrow_mut().clear();

    let (end, _, at) = run(&fake, false, launching);

    assert_eq!(end, ended(HeadEnd::Unavailable, other()), "nothing invented");
    assert!(launches().is_empty());
    assert_eq!(at, Duration::ZERO);
}

// Every relaunch is gated on the shared budget.

const JUST_BEFORE: Duration = Duration::from_millis(74_900);
const PAST: Duration = Duration::from_millis(76_000);

/// A forced wait whose head finished at once and whose PR half was
/// contended by a holder that releases its lock at `release` without
/// publishing; a replacement never records anything.
fn contended_until(release: Duration) -> Fake {
    Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
        .receipts(|_, for_attempt| (for_attempt.id == OURS).then(|| receipt(OURS, HeadStatus::Ok, PrStatus::Contended)))
        .pr_lock(move |t| t < release)
        .pr_answer(|_| Some(SEEDED))
        .exits(&[Some(ms(10)), None])
}

#[test]
fn a_pr_retry_is_launched_only_within_the_budget() {
    for (release, expected, launch_count) in [
        // The relaunch runs; its PR half is still asking at the budget.
        (JUST_BEFORE, timed_out(in_sync(OURS), PrEnd::Pending), 2),
        // The holder failed, and no time is left to ask again.
        (FORCED_BUDGET, timed_out(in_sync(OURS), other()), 1),
        (PAST, timed_out(in_sync(OURS), PrEnd::Pending), 1),
    ] {
        let fake = contended_until(release);

        let (end, _, at) = run(&fake, true, launching);

        assert_eq!(end, expected, "released at {release:?}");
        assert_eq!(launches().len(), launch_count, "released at {release:?}");
        assert!(within_the_budget(at, FORCED_BUDGET), "{at:?}");
    }
}

/// A forced wait whose worker found the head lock held for another branch
/// until `release`, with its own PR half rejected; the replacement's head
/// is in sync 200 ms after the release and reports a publication.
fn held_for_another_branch_until(release: Duration) -> Fake {
    let trunk = Attempt { branch: "trunk".into(), ..attempt(OTHER, Phase::Checking, None) };
    let replaced = release + ms(200);
    Fake::new(move |t| {
        if t < replaced { with(trunk.clone()) } else { with(attempt(SECOND, Phase::Checking, Some(Outcome::InSync))) }
    })
    .receipts(move |t, for_attempt| match for_attempt.id.as_str() {
        OURS => Some(receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::Failed { failure: rejected() })),
        _ => (t >= replaced).then(|| receipt(SECOND, HeadStatus::Ok, PrStatus::Ok)),
    })
    .head_lock(move |t| t < release)
    .exits(&[Some(ms(20)), Some(replaced)])
}

#[test]
fn a_head_retry_is_launched_only_within_the_budget() {
    let unchecked = || HeadEnd::Running { last: None };
    for (release, expected, launch_count) in [
        (ms(100), ended(in_sync(SECOND), PrEnd::Published), 2),
        // The replacement cannot finish by the budget; the first receipt's
        // PR diagnosis stands.
        (JUST_BEFORE, timed_out(unchecked(), PrEnd::Failed(rejected())), 2),
        (FORCED_BUDGET, timed_out(unchecked(), PrEnd::Failed(rejected())), 1),
        (PAST, timed_out(unchecked(), PrEnd::Failed(rejected())), 1),
    ] {
        let fake = held_for_another_branch_until(release);

        let (end, _, at) = run(&fake, true, launching);

        assert_eq!(end, expected, "released at {release:?}");
        assert_eq!(launches().len(), launch_count, "released at {release:?}");
        if end.timed_out {
            assert!(within_the_budget(at, FORCED_BUDGET), "{at:?}");
        }
    }
}

#[test]
fn a_holder_for_another_branch_past_the_budget_is_a_timeout_that_keeps_a_publication() {
    let trunk = Attempt { branch: "trunk".into(), ..attempt(OTHER, Phase::Checking, None) };
    let fake = Fake::new(move |_| with(trunk.clone()))
        .receipts(|_, _| Some(receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::Ok)))
        .head_lock(|_| true)
        .exits(&[Some(ms(20))]);

    let (end, _, at) = run(&fake, true, launching);

    assert_eq!(end, timed_out(HeadEnd::Running { last: None }, PrEnd::Published), "the refresh hint shows");
    assert!(within_the_budget(at, FORCED_BUDGET), "{at:?}");
    assert_eq!(launches().len(), 1);
    assert_eq!(fake.early_probes.get(), 0, "the head lock is probed only after our worker exited");
}

#[test]
fn an_ordinary_contended_pr_half_released_at_the_budget_never_relaunches() {
    for (release, expected) in [
        (ms(2_900), ended(in_sync(OURS), other())),
        // The poll at the budget still sees the release: the holder failed.
        (ORDINARY_BUDGET, ended(in_sync(OURS), other())),
        (ms(3_500), timed_out(in_sync(OURS), PrEnd::Pending)),
    ] {
        let fake = contended_until(release);

        let (end, _, at) = run(&fake, false, launching);

        assert_eq!(end, expected, "released at {release:?}");
        assert_eq!(launches().len(), 1);
        assert!(at < ORDINARY_BUDGET + ms(50), "{at:?}");
    }
}

// Once only the PR half is left, the spinner says `updating`, whichever
// route ran the head half that just finished.

/// The head phases with their own spinner text, each with the outcome a
/// head ending in that phase records.
fn distinct_phases() -> [(Phase, Outcome); 3] {
    let failed = Outcome::CheckFailed { reason: CheckFailure::Other };
    [
        (Phase::Fetching, Outcome::Fetched),
        (Phase::CheckingFallback { reason: FallbackReason::NoKey }, failed),
        (Phase::CheckingFallback { reason: FallbackReason::RateLimited }, failed),
    ]
}

/// `id`'s head in `phase` from `from`, finished with `outcome` from
/// `finish` on.
fn head_in(id: &'static str, phase: Phase, outcome: Outcome, finish: Duration) -> impl Fn(Duration) -> Attempt {
    move |t| attempt(id, phase, (t >= finish).then_some(outcome))
}

const ROUTES: [&str; 4] = ["PR retry", "ordinary launch", "adopted head", "head retry"];

/// One of [`ROUTES`], followed into a head in `phase` that finishes with
/// `outcome` while its PR half is still pending; returns whether the wait is
/// forced, and the scripted environment. Built one at a time, since
/// `Fake::new` and `Fake::exits` reset shared state.
fn route(name: &str, phase: Phase, outcome: Outcome) -> (bool, Fake) {
    match name {
        // The first head finishes at once; its PR holder releases at 1 s
        // having published nothing, so the replacement runs both halves
        // again. Its head finishes at 1.1 s and its receipt arrives at 2 s.
        "PR retry" => {
            let replacement = head_in(SECOND, phase, outcome, ms(1_100));
            let fake = Fake::new(move |t| {
                if t < ms(1_000) { with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))) } else { with(replacement(t)) }
            })
            .receipts(|t, for_attempt| match for_attempt.id.as_str() {
                OURS => Some(receipt(OURS, HeadStatus::Ok, PrStatus::Contended)),
                _ => (t >= ms(2_000)).then(|| receipt(SECOND, HeadStatus::Ok, PrStatus::Ok)),
            })
            .pr_lock(|t| t < ms(1_000))
            .pr_answer(|_| Some(SEEDED))
            .exits(&[Some(ms(10)), Some(ms(2_000))]);
            (true, fake)
        }
        "ordinary launch" => {
            let ours = head_in(OURS, phase, outcome, ms(100));
            let fake =
                Fake::new(move |t| with(ours(t))).receipts(receipt_at(ms(2_000), PrStatus::Ok)).exits(&[Some(ms(2_000))]);
            (false, fake)
        }
        // Our worker found a matching attempt running; the PR holder keeps
        // its lock until 2 s, when its publication appears.
        "adopted head" => {
            let theirs = head_in(OTHER, phase, outcome, ms(900));
            let fake = Fake::new(move |t| with(theirs(t)))
                .receipts(|t, _| (t >= ms(20)).then(|| receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::Contended)))
                .head_lock(|t| t < ms(900))
                .pr_lock(|t| t < ms(2_000))
                .pr_answer(|t| Some(if t < ms(2_000) { SEEDED } else { PUBLISHED }))
                .exits(&[Some(ms(20))]);
            (false, fake)
        }
        // The head lock is held for another branch until 800 ms and the
        // first PR request failed, so the replacement runs both halves; its
        // head appears at 1 s and finishes at 1.1 s.
        "head retry" => {
            let trunk = Attempt { branch: "trunk".into(), ..attempt(OTHER, Phase::Checking, None) };
            let replacement = head_in(SECOND, phase, outcome, ms(1_100));
            let fake = Fake::new(move |t| if t < ms(1_000) { with(trunk.clone()) } else { with(replacement(t)) })
                .receipts(|t, for_attempt| match for_attempt.id.as_str() {
                    OURS => Some(receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::Failed { failure: rejected() })),
                    _ => (t >= ms(2_000)).then(|| receipt(SECOND, HeadStatus::Ok, PrStatus::Ok)),
                })
                .head_lock(|t| t < ms(800))
                .exits(&[Some(ms(20)), Some(ms(2_000))]);
            (true, fake)
        }
        _ => unreachable!("no route {name}"),
    }
}

#[test]
fn a_finished_heads_phase_gives_way_to_updating_while_prs_are_pending_on_every_route() {
    // Every case runs, so a failure names each route and phase it affects.
    let mut wrong = Vec::new();
    for (phase, outcome) in distinct_phases() {
        for name in ROUTES {
            let (force, fake) = route(name, phase, outcome);

            let (end, phases, at) = run(&fake, force, launching);

            let finished = matches!(&end.head, HeadEnd::Finished(Attempt { outcome: Some(o), .. }) if *o == outcome);
            let followed = end.prs == PrEnd::Published && finished && at >= ms(2_000) && phases.contains(&phase);
            if !followed || phases.last() != Some(&Phase::Checking) {
                wrong.push(format!("{name}, {phase:?}: {end:?} at {at:?}, spinner {phases:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "the spinner must end on `updating` after the head's phase:\n{}", wrong.join("\n"));
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
    pub(super) type Spoil = std::rc::Rc<dyn Fn(&mut Receipt, &Path)>;

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
        let spoil = SPOIL.with(|spoil| spoil.borrow().clone());
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
    type Spoiler = fn(&mut Receipt, &Path);
    let spoilers: [(&str, Spoiler); 5] = [
        ("another origin", |receipt, _| receipt.origin_digest = "another".into()),
        ("another branch", |receipt, _| receipt.branch = "trunk".into()),
        ("another attempt id", |receipt, _| receipt.attempt_id = OTHER.into()),
        ("finished before the attempt began", |receipt, _| receipt.finished_at = 1),
        ("malformed", |_, path| std::fs::write(path, "{\"format_version\": 1, \"attempt_id\":").unwrap()),
    ];
    for (label, spoil) in spoilers {
        let (end, _) = receipt_files::run(Some(std::rc::Rc::new(spoil)));

        assert_eq!(end.prs, other(), "{label}: a generic failure, never the receipt's reason");
        assert!(matches!(end.head, HeadEnd::Finished(_)), "{label}: the head outcome is kept: {end:?}");
        assert!(!end.timed_out, "{label}: bounded by the worker's exit");
    }
}

/// The receipt Input Robustness Matrix, carried through the wait with real
/// files. The cells copy
/// `worktree::remote_head::tests::the_receipt_reader_walks_the_input_robustness_matrix`,
/// since a library's unit tests cannot be shared.
mod receipt_matrix {
    use serde_json::{Map, Value, json};

    use super::*;

    /// One edit to a receipt the real writer wrote.
    pub(super) enum JsonEdit {
        /// Repeat the key at a JSON pointer, with its written value, in the
        /// same object.
        Dup(String),
        /// Replace (or, with `None`, remove) the value at a JSON pointer.
        Set(String, Option<Value>),
        Append(&'static str),
    }

    /// The object holding `pointer`'s last segment, and that segment.
    fn parent<'a>(document: &'a mut Value, pointer: &'a str) -> (&'a mut Map<String, Value>, &'a str) {
        let (parent, key) = pointer.rsplit_once('/').expect("a pointer");
        let target = if parent.is_empty() { document } else { document.pointer_mut(parent).expect("the parent exists") };
        (target.as_object_mut().expect("an object"), key)
    }

    impl JsonEdit {
        pub(super) fn apply(&self, written: &Value) -> Vec<u8> {
            let mut document = written.clone();
            match self {
                Self::Set(pointer, value) => {
                    let (map, key) = parent(&mut document, pointer);
                    match value {
                        Some(value) => {
                            map.insert(key.to_string(), value.clone());
                        }
                        None => assert!(map.remove(key).is_some(), "{pointer} exists"),
                    }
                    serde_json::to_vec(&document).expect("serializes")
                }
                Self::Dup(pointer) => {
                    let (map, key) = parent(&mut document, pointer);
                    let value = map.get(key).unwrap_or_else(|| panic!("{pointer} exists")).clone();
                    map.insert("__repeat__".into(), value);
                    let key = key.to_string();
                    let text = serde_json::to_string(&document).expect("serializes");
                    text.replacen("\"__repeat__\"", &format!("\"{key}\""), 1).into_bytes()
                }
                Self::Append(tail) => {
                    let mut bytes = serde_json::to_vec(&document).expect("serializes");
                    bytes.extend_from_slice(tail.as_bytes());
                    bytes
                }
            }
        }
    }

    /// The failures that carry fields, as the matrix's positive controls.
    pub(super) fn field_bearing_failures() -> [PrFailure; 3] {
        [
            PrFailure::CredentialsRejected { key: Some("GITHUB_TOKEN".into()) },
            PrFailure::CredentialsInsufficient { key: Some("GH_TOKEN".into()) },
            PrFailure::RateLimited { authenticated: true, key: Some("GITHUB_TOKEN".into()) },
        ]
    }

    /// Each edit to a receipt whose PR half failed with `failure`, and the
    /// failure the receipt then reports, `None` for a missing receipt.
    pub(super) fn cells(failure: &PrFailure) -> Vec<(JsonEdit, Option<PrFailure>)> {
        use JsonEdit::*;
        let set = |pointer: &str, value: Option<Value>| Set(pointer.to_string(), value);
        let mut cells = Vec::new();

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
            ("/attempt_id", json!(OTHER)),
            ("/attempt_id", json!("0123456789ABCDEF0123456789ABCDEF")),
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

        for field in ["/prs/kind", "/prs/failure", "/prs/failure/kind"] {
            for shape in [None, Some(Value::Null), Some(json!([])), Some(json!({})), Some(json!("")), Some(json!(1))] {
                cells.push((set(field, shape), None));
            }
            cells.push((Dup(field.to_string()), None));
        }
        cells.push((set("/prs/kind", Some(json!("skipped-fresh"))), None));
        cells.push((set("/prs/failure", Some(json!("other"))), None));
        cells.push((set("/prs/failure/kind", Some(json!("credentials-lost"))), None));

        let unnamed = match failure.clone() {
            PrFailure::CredentialsRejected { .. } => PrFailure::CredentialsRejected { key: None },
            PrFailure::CredentialsInsufficient { .. } => PrFailure::CredentialsInsufficient { key: None },
            PrFailure::RateLimited { authenticated, .. } => PrFailure::RateLimited { authenticated, key: None },
            other => panic!("{other:?} carries no key"),
        };
        cells.push((set("/prs/failure/key", Some(Value::Null)), Some(unnamed)));
        for shape in [None, Some(json!([])), Some(json!({})), Some(json!("")), Some(json!(1)), Some(json!("a token")), Some(json!("1TOKEN"))] {
            cells.push((set("/prs/failure/key", shape), None));
        }
        cells.push((Dup("/prs/failure/key".into()), None));

        if let PrFailure::RateLimited { key, .. } = failure {
            let unauthenticated = PrFailure::RateLimited { authenticated: false, key: key.clone() };
            cells.push((set("/prs/failure/authenticated", Some(json!(false))), Some(unauthenticated)));
            for shape in [None, Some(Value::Null), Some(json!([])), Some(json!({})), Some(json!("")), Some(json!("true")), Some(json!(1))] {
                cells.push((set("/prs/failure/authenticated", shape), None));
            }
            cells.push((Dup("/prs/failure/authenticated".into()), None));
        }

        cells.push((set("/extra", Some(json!(1))), Some(failure.clone())));
        cells.push((set("/prs/failure/extra", Some(json!(1))), Some(failure.clone())));
        cells.push((Append("garbage"), None));
        cells.push((Append("{}"), None));
        cells
    }
}

/// Every cell of the receipt matrix, for each failure that carries fields,
/// through the public wait over real files: a rejected receipt keeps the
/// finished head, is a generic failure that never names credentials, and ends
/// with the worker, inside the budget; a valid edit keeps the receipt's own
/// failure.
#[test]
fn every_malformed_receipt_through_the_wait_keeps_the_head_and_is_a_generic_failure() {
    use std::rc::Rc;

    use worktree::remote_head::write_receipt;

    let mut wrong = Vec::new();
    let mut walked = 0;
    for failure in receipt_matrix::field_bearing_failures() {
        let control = Some(failure.clone());
        let cells = std::iter::once((None, control)).chain(receipt_matrix::cells(&failure).into_iter().map(|(edit, want)| (Some(edit), want)));
        for (edit, want) in cells {
            let edit = Rc::new(edit);
            let label = Rc::new(RefCell::new(String::from("control")));
            let (failure, written_label) = (failure.clone(), Rc::clone(&label));
            let spoil: receipt_files::Spoil = Rc::new(move |receipt, path| {
                receipt.prs = PrStatus::Failed { failure: failure.clone() };
                write_receipt(path, receipt).expect("receipt");
                if let Some(edit) = edit.as_ref() {
                    let written: serde_json::Value = serde_json::from_slice(&std::fs::read(path).expect("read")).expect("json");
                    let bytes = edit.apply(&written);
                    *written_label.borrow_mut() = String::from_utf8_lossy(&bytes).into_owned();
                    std::fs::write(path, bytes).expect("write");
                }
            });
            let began = std::time::Instant::now();

            let (end, left) = receipt_files::run(Some(spoil));

            let took = began.elapsed();
            let expected = want.map_or_else(other, PrEnd::Failed);
            let head_kept = matches!(&end.head, HeadEnd::Finished(Attempt { outcome: Some(Outcome::InSync), .. }));
            if end.prs != expected || !head_kept || end.timed_out || took >= ORDINARY_BUDGET || !left.is_empty() {
                wrong.push(format!("{}: {end:?} in {took:?}, left {left:?}; expected {expected:?}", label.borrow()));
            }
            walked += 1;
        }
    }
    assert!(wrong.is_empty(), "{} of {walked} receipts were not carried through the wait:\n{}", wrong.len(), wrong.join("\n"));
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

// Cleanup is one deletion attempt per launched receipt, made as the wait
// returns. A receipt the wait observed exists then and is deleted. The wait
// can also end before its worker writes the receipt: a new publication, or a
// result retained from a replaced launch, plus a finished head is enough. A
// receipt written after that is left for the next worker's age sweep
// (`a_receipt_written_after_an_early_success_is_left_for_the_stale_sweep`).

/// Each receipt the wait discarded, split into those that existed when it
/// was discarded (observed) and those its worker had not yet written (late).
/// Every discard is made at `at`, the moment the wait returned.
fn cleanup_at(fake: &Fake, at: Duration) -> (Vec<String>, Vec<String>) {
    let (mut observed, mut late) = (Vec::new(), Vec::new());
    for (id, when) in DISCARDED.with(|discarded| discarded.borrow().clone()) {
        assert_eq!(when, at, "{id} is discarded as the wait returns, not later");
        if receipt_written(fake, &id, when) { observed.push(id) } else { late.push(id) }
    }
    (observed, late)
}

fn receipt_written(fake: &Fake, id: &str, at: Duration) -> bool {
    (fake.receipt)(at, &attempt(id, Phase::Checking, None)).is_some()
}

/// One way the wait succeeds before the last launched worker writes its
/// receipt: whether it is forced, the scripted environment, the expected end,
/// and when it ends. The last launch's receipt is written at `ends + 100 ms`.
/// Built one at a time, since `Fake::new` resets shared state.
fn early_success(name: &str) -> (bool, Fake, WaitEnd, Duration) {
    let publishes_at = |at: Duration| move |t| Some(if t < at { SEEDED } else { PUBLISHED });
    match name {
        // The head finishes and the PR answer is published at 100 ms; the
        // receipt follows at 200 ms.
        "ordinary" | "forced" => {
            let fake = Fake::new(|t| with(attempt(OURS, Phase::Checking, (t >= ms(100)).then_some(Outcome::InSync))))
                .pr_answer(publishes_at(ms(100)))
                .receipts(receipt_at(ms(200), PrStatus::Ok))
                .exits(&[Some(ms(200))]);
            (name == "forced", fake, ended(in_sync(OURS), PrEnd::Published), ms(100))
        }
        // The first receipt reports contention; the holder releases at 1 s
        // having published nothing, so the replacement asks again. It
        // publishes at 1.1 s and writes its receipt at 1.2 s.
        "forced PR retry" => {
            let fake = Fake::new(|_| latest_launch_in_sync())
                .receipts(|t, for_attempt| match for_attempt.id.as_str() {
                    OURS => Some(receipt(OURS, HeadStatus::Ok, PrStatus::Contended)),
                    _ => (t >= ms(1_200)).then(|| receipt(SECOND, HeadStatus::Ok, PrStatus::Ok)),
                })
                .pr_lock(|t| t < ms(1_000))
                .pr_answer(publishes_at(ms(1_100)))
                .exits(&[Some(ms(10)), Some(ms(1_200))]);
            (true, fake, ended(in_sync(SECOND), PrEnd::Published), ms(1_100))
        }
        // The first receipt reports a held head lock and PR success; the
        // replacement's head finishes at 1 s, so the retained success ends
        // the wait with no new publication. Its receipt follows at 1.1 s.
        "forced head retry" => {
            let fake = head_retry(PrStatus::Ok, Some(PrStatus::Ok)).exits(&[Some(ms(20)), Some(ms(1_100))]);
            (true, fake, ended(in_sync(SECOND), PrEnd::Published), ms(1_000))
        }
        _ => unreachable!("no route {name}"),
    }
}

#[test]
fn an_early_success_deletes_only_the_receipts_it_saw_and_leaves_a_later_one() {
    for name in ["ordinary", "forced", "forced PR retry", "forced head retry"] {
        let (force, fake, expected, ends) = early_success(name);

        let (end, _, at) = run(&fake, force, launching);

        assert_eq!(end, expected, "{name}");
        assert_eq!(at, ends, "{name}: success needs no receipt from the last launch");
        let launched_ids: Vec<String> = launches().into_iter().map(|args| args.attempt).collect();
        assert_eq!(discarded(), launched_ids, "{name}: each launched receipt once, nothing else");
        let (observed, late) = cleanup_at(&fake, at);
        let last = launched_ids.last().expect("a launch");
        assert_eq!(late, std::slice::from_ref(last), "{name}: the last launch's receipt did not exist yet");
        assert_eq!(observed, launched_ids[..launched_ids.len() - 1], "{name}: a replaced launch's receipt was read");
        assert!(receipt_written(&fake, last, at + ms(100)), "{name}: its worker writes it after the wait");
    }
}

/// The routes whose result needs the receipt: the wait ends on it, so the
/// receipt exists when it is deleted. An adopted head's run is never ours to
/// clean up.
#[test]
fn a_wait_that_ends_on_its_receipt_deletes_it() {
    let failed = PrStatus::Failed { failure: rejected() };
    for (status, expected) in [(failed, PrEnd::Failed(rejected())), (PrStatus::Unsupported, PrEnd::Unsupported)] {
        for force in [false, true] {
            let fake = Fake::new(|t| with(attempt(OURS, Phase::Checking, (t >= ms(100)).then_some(Outcome::InSync))))
                .receipts(receipt_at(ms(200), status.clone()))
                .exits(&[Some(ms(200))]);

            let (end, _, at) = run(&fake, force, launching);

            assert_eq!(end, ended(in_sync(OURS), expected.clone()), "{status:?}, force: {force}");
            assert_eq!(at, ms(200), "the wait ends on the receipt");
            assert_eq!(cleanup_at(&fake, at), (vec![OURS.to_string()], vec![]), "{status:?}: observed, so deleted");
        }
    }

    let fake = Fake::new(|t| with(attempt(OTHER, Phase::Checking, (t >= ms(900)).then_some(Outcome::InSync))))
        .receipts(|t, for_attempt| {
            (for_attempt.id == OURS && t >= ms(20)).then(|| receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::Ok))
        })
        .exits(&[Some(ms(20))])
        .head_lock(|t| t < ms(900));

    let (end, _, at) = run(&fake, false, launching);

    assert_eq!(end, ended(in_sync(OTHER), PrEnd::Published));
    assert_eq!(cleanup_at(&fake, at), (vec![OURS.to_string()], vec![]), "our own observed receipt, never the adopted run's");
}

/// The late receipt with real files: the wait ends on a new publication and
/// a finished head before its worker writes the receipt; the worker then
/// sweeps and writes as `refresh_worker::run_and_record` does, and the file
/// stays until a later worker's sweep finds it older than `ATTEMPT_MAX_AGE`.
mod late_receipt {
    use worktree::pull_requests::origin_digest;
    use worktree::remote_head::{begin_attempt, finish_attempt, receipt_path_beside};

    use super::*;

    thread_local! {
        static STORES: RefCell<Option<(PathBuf, PathBuf)>> = const { RefCell::new(None) };
    }

    /// Finishes the head check and publishes a PR answer, but is still
    /// running: its receipt comes after both halves join.
    fn worker(_main: &Path, args: &LaunchArgs) -> std::io::Result<WorkerHandle> {
        let (head, prs) = STORES.with(|stores| stores.borrow().clone()).expect("late_receipt::run");
        let attempt = Attempt::begin(args.attempt.clone(), origin_digest(stored_prs::ORIGIN), BRANCH.into(), unix_now());
        begin_attempt(&head, &attempt).expect("attempt");
        finish_attempt(&head, &args.attempt, Outcome::InSync, None).expect("outcome");
        let store = stored_prs::store_json(PUBLISHED, unix_now(), &origin_digest(stored_prs::ORIGIN), "[]");
        std::fs::write(&prs, store).expect("PR store");
        assert!(!receipt_path_beside(&head, &args.attempt).expect("path").exists());
        Ok(WorkerHandle::new(|| false))
    }

    /// Runs one ordinary wait over real stores in `dir`; returns its end and
    /// the remote-head store.
    pub(super) fn run(dir: &Path) -> (WaitEnd, PathBuf) {
        let head = dir.join("abc.remote-head.json");
        let prs = dir.join("abc.prs.json");
        STORES.with(|stores| *stores.borrow_mut() = Some((head.clone(), prs.clone())));
        let digest = origin_digest(stored_prs::ORIGIN);
        let request = WaitRequest { main: Path::new("/repo"), origin_digest: &digest, branch: BRANCH, force: false, budget: ORDINARY_BUDGET };
        let env = StoreEnv::new(head.clone(), prs, stored_prs::ORIGIN.into());
        (wait(request, &env, worker, &mut |_| {}), head)
    }
}

#[test]
fn a_receipt_written_after_an_early_success_is_left_for_the_stale_sweep() {
    use std::time::SystemTime;

    use worktree::pull_requests::origin_digest;
    use worktree::remote_head::{ATTEMPT_MAX_AGE, remove_stale_receipts, write_receipt};

    let dir = tempfile::tempdir().expect("temp dir");
    let (end, head) = late_receipt::run(dir.path());

    let HeadEnd::Finished(followed) = &end.head else {
        panic!("the head finished: {end:?}");
    };
    assert_eq!((&end.prs, end.timed_out), (&PrEnd::Published, false), "{end:?}");
    let path = receipt_path_beside(&head, &followed.id).expect("path");
    assert!(!path.exists(), "the wait ended before its worker wrote the receipt");

    // The worker's halves join after the wait: it sweeps, then writes.
    remove_stale_receipts(&path, SystemTime::now());
    let late = Receipt {
        attempt_id: followed.id.clone(),
        origin_digest: origin_digest(stored_prs::ORIGIN),
        branch: BRANCH.into(),
        finished_at: unix_now(),
        head: HeadStatus::Ok,
        prs: PrStatus::Ok,
    };
    write_receipt(&path, &late).expect("receipt");
    assert_eq!(load_receipt(&path, followed), Some(late), "a complete receipt, left behind");

    // A later worker's sweep, before writing its own receipt.
    let next = receipt_path_beside(&head, SECOND).expect("path");
    remove_stale_receipts(&next, SystemTime::now());
    assert!(path.exists(), "kept while younger than ATTEMPT_MAX_AGE");
    remove_stale_receipts(&next, SystemTime::now() + ATTEMPT_MAX_AGE + Duration::from_secs(1));
    assert!(!path.exists(), "swept once older than ATTEMPT_MAX_AGE");
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

