//! The wait protocol over a scripted store, scripted locks, and a fake clock
//! that advances only when the wait sleeps.

use std::cell::{Cell, RefCell};
use std::io::Write;
use std::path::Path;
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
/// PR store publication ids: the answer stored at launch, and a holder's.
const SEEDED: &str = "5eeded00000000000000000000000000";
const PUBLISHED: &str = "fedcba9876543210fedcba9876543210";

thread_local! {
    /// The fake clock: time since the wait began.
    static CLOCK: Cell<Duration> = const { Cell::new(Duration::ZERO) };
    /// Every launch, in order.
    static LAUNCHES: RefCell<Vec<LaunchArgs>> = const { RefCell::new(Vec::new()) };
    /// When each launched worker exits, by launch order; `None` never.
    static EXITS: RefCell<Vec<Option<Duration>>> = const { RefCell::new(Vec::new()) };
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
    /// Whether the current worker has exited, as the probe sees it.
    worker_alive: Box<dyn Fn() -> bool>,
}

impl Fake {
    fn new(store: impl Fn(Duration) -> StoreState + 'static) -> Self {
        CLOCK.with(|clock| clock.set(Duration::ZERO));
        LAUNCHES.with(|launches| launches.borrow_mut().clear());
        EXITS.with(|exits| exits.borrow_mut().clear());
        Self {
            store: Box::new(store),
            receipt: Box::new(|_, _| None),
            head_lock: Box::new(|_| false),
            pr_lock: Box::new(|_| false),
            pr_answer: Box::new(|_| None),
            ids: RefCell::new(vec![OURS, SECOND]),
            early_probes: Cell::new(0),
            worker_alive: Box::new(|| {
                let launched = LAUNCHES.with(|launches| launches.borrow().len());
                let exit = EXITS.with(|exits| exits.borrow().get(launched.wrapping_sub(1)).copied().flatten());
                launched > 0 && exit.is_none_or(|at| now() < at)
            }),
        }
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

    fn head_lock_held(&self) -> bool {
        if (self.worker_alive)() {
            self.early_probes.set(self.early_probes.get() + 1);
        }
        (self.head_lock)(now())
    }

    fn pr_lock_held(&self) -> bool {
        (self.pr_lock)(now())
    }

    fn pr_publication(&self) -> Option<String> {
        (self.pr_answer)(now()).map(str::to_string)
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
    let waited = wait(request(force), fake, launch, &mut |phase| phases.push(phase));
    (waited.end, phases, now())
}

#[test]
fn our_attempt_is_followed_through_every_phase_to_its_outcome() {
    let fake = Fake::new(|t| {
        let fallback = Phase::CheckingFallback { reason: FallbackReason::NoKey };
        match t {
            t if t < ms(50) => empty(),
            t if t < ms(200) => with(attempt(OURS, Phase::Checking, None)),
            t if t < ms(400) => with(attempt(OURS, fallback, None)),
            t if t < ms(600) => with(attempt(OURS, Phase::Fetching, None)),
            _ => with(attempt(OURS, Phase::Fetching, Some(Outcome::Fetched))),
        }
    });

    let (end, phases, at) = run(&fake, false, launching);

    assert_eq!(
        end,
        WaitEnd::Finished { attempt: attempt(OURS, Phase::Fetching, Some(Outcome::Fetched)), receipt: None }
    );
    assert_eq!(
        phases,
        [Phase::Checking, Phase::CheckingFallback { reason: FallbackReason::NoKey }, Phase::Fetching]
    );
    assert!(at >= ms(600) && at < ms(700), "ends at the outcome: {at:?}");
    assert_eq!(launches(), [LaunchArgs { attempt: OURS.into(), force: false }], "one launch");
    assert_eq!(fake.early_probes.get(), 0, "the lock is never probed while our worker may take it");
}

#[test]
fn a_running_attempt_is_adopted_with_no_second_launch() {
    let fake = Fake::new(|t| {
        let outcome = (t >= ms(900)).then_some(Outcome::InSync);
        with(attempt(OTHER, Phase::Checking, outcome))
    })
    .exits(&[Some(ms(20))])
    .head_lock(|t| t < ms(900));

    let (end, phases, at) = run(&fake, false, launching);

    assert_eq!(
        end,
        WaitEnd::Finished { attempt: attempt(OTHER, Phase::Checking, Some(Outcome::InSync)), receipt: None }
    );
    assert_eq!(phases, [Phase::Checking]);
    assert!(at >= ms(900), "{at:?}");
    assert_eq!(launches().len(), 1);
    assert_eq!(fake.early_probes.get(), 0, "adoption probes only after our worker exited");
}

#[test]
fn a_contender_that_exits_early_is_never_a_finished_check() {
    // Another attempt finished before this run; ours exits without an
    // attempt, and nothing holds the lock.
    for stored in [
        with(attempt(OTHER, Phase::Checking, Some(Outcome::InSync))),
        with(attempt(OTHER, Phase::Checking, None)),
        empty(),
    ] {
        let fake = Fake::new(move |_| stored.clone()).exits(&[Some(ms(30))]);

        let (end, _, at) = run(&fake, false, launching);

        assert_eq!(end, WaitEnd::Unavailable);
        assert!(at < ms(100), "rendered at once: {at:?}");
    }
}

#[test]
fn a_spawn_failure_is_unavailable_at_once() {
    let fake = Fake::new(|_| with(attempt(OTHER, Phase::Checking, None))).head_lock(|_| true);

    let (end, phases, at) = run(&fake, false, failing_launch);

    assert_eq!(end, WaitEnd::Unavailable);
    assert!(phases.is_empty());
    assert_eq!(at, Duration::ZERO, "no wait at all");
}

#[test]
fn a_worker_that_stops_mid_attempt_is_unavailable() {
    let fake = Fake::new(|t| if t < ms(40) { empty() } else { with(attempt(OURS, Phase::Checking, None)) })
        .exits(&[Some(ms(100))]);

    let (end, _, at) = run(&fake, false, launching);

    assert_eq!(end, WaitEnd::Unavailable);
    assert!(at < ms(200), "{at:?}");
}

#[test]
fn the_budget_ends_the_wait_with_the_last_phase_seen() {
    for phase in [Phase::Checking, Phase::Fetching] {
        let fake = Fake::new(move |_| with(attempt(OURS, phase, None)));

        let (end, _, at) = run(&fake, false, launching);

        assert_eq!(end, WaitEnd::TimedOut { last: Some(attempt(OURS, phase, None)) });
        assert!(at >= ORDINARY_BUDGET && at < ORDINARY_BUDGET + ms(50), "{at:?}");
    }
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

        assert_eq!(end, WaitEnd::TimedOut { last: None }, "{:?}", fake.store());
        assert!(phases.is_empty());
    }
}

#[test]
fn a_forced_wait_follows_the_outcome_to_the_receipt() {
    let fake = Fake::new(|t| {
        let outcome = (t >= ms(100)).then_some(Outcome::InSync);
        with(attempt(OURS, Phase::Checking, outcome))
    })
    .receipts(|t, for_attempt| {
        assert_eq!(for_attempt.id, OURS, "the receipt is looked up by our token");
        (t >= ms(5_000)).then(|| receipt(OURS, HeadStatus::Ok, PrStatus::Ok))
    })
    .exits(&[Some(ms(5_000))]);

    let (end, _, at) = run(&fake, true, launching);

    let expected = receipt(OURS, HeadStatus::Ok, PrStatus::Ok);
    assert_eq!(
        end,
        WaitEnd::Finished { attempt: attempt(OURS, Phase::Checking, Some(Outcome::InSync)), receipt: Some(expected) }
    );
    assert!(at >= ms(5_000) && at > ORDINARY_BUDGET, "the 3 s limit does not apply: {at:?}");
    assert_eq!(launches(), [LaunchArgs { attempt: OURS.into(), force: true }]);
}

#[test]
fn a_forced_wait_accepts_the_answer_a_contending_holder_published() {
    let fake = Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
        .receipts(|_, _| Some(receipt(OURS, HeadStatus::Ok, PrStatus::Contended)))
        .pr_lock(|t| t < ms(2_000))
        .pr_answer(|t| Some(if t < ms(2_000) { SEEDED } else { PUBLISHED }))
        .exits(&[Some(ms(10))]);

    let (end, _, at) = run(&fake, true, launching);

    let expected = receipt(OURS, HeadStatus::Ok, PrStatus::Contended);
    assert_eq!(
        end,
        WaitEnd::Finished { attempt: attempt(OURS, Phase::Checking, Some(Outcome::InSync)), receipt: Some(expected) }
    );
    assert!(at >= ms(2_000), "the holder's lock was waited for: {at:?}");
    assert_eq!(launches().len(), 1, "a published answer needs no second request");
}

/// `fetched_at` is whole seconds, so a holder that replaces a young answer
/// within its second leaves it unchanged; only the publication id proves
/// the write, and the run must not ask again.
#[test]
fn a_forced_wait_accepts_a_holders_answer_published_within_the_same_second() {
    let fake = Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
        .receipts(|_, _| Some(receipt(OURS, HeadStatus::Ok, PrStatus::Contended)))
        .pr_lock(|t| t < ms(500))
        .pr_answer(|t| Some(if t < ms(500) { SEEDED } else { PUBLISHED }))
        .exits(&[Some(ms(10))]);

    let (end, _, _) = run(&fake, true, launching);

    let expected = receipt(OURS, HeadStatus::Ok, PrStatus::Contended);
    assert_eq!(
        end,
        WaitEnd::Finished { attempt: attempt(OURS, Phase::Checking, Some(Outcome::InSync)), receipt: Some(expected) }
    );
    assert_eq!(launches().len(), 1, "a new publication id is a published answer");
}

/// A holder that publishes into an empty store is published too.
#[test]
fn a_forced_wait_accepts_a_holders_first_answer() {
    let fake = Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
        .receipts(|_, _| Some(receipt(OURS, HeadStatus::Ok, PrStatus::Contended)))
        .pr_lock(|t| t < ms(500))
        .pr_answer(|t| (t >= ms(500)).then_some(PUBLISHED))
        .exits(&[Some(ms(10))]);

    let (end, _, _) = run(&fake, true, launching);

    assert!(matches!(end, WaitEnd::Finished { receipt: Some(Receipt { prs: PrStatus::Contended, .. }), .. }), "{end:?}");
    assert_eq!(launches().len(), 1);
}

/// The holder's request failed or was skipped, so the store kept the answer
/// (and publication id) it had at launch: the run asks again itself.
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

    let expected = receipt(SECOND, HeadStatus::Ok, PrStatus::Ok);
    assert_eq!(
        end,
        WaitEnd::Finished { attempt: attempt(SECOND, Phase::Checking, Some(Outcome::InSync)), receipt: Some(expected) }
    );
    assert!(at >= ms(2_000), "{at:?}");
    assert_eq!(
        launches(),
        [LaunchArgs { attempt: OURS.into(), force: true }, LaunchArgs { attempt: SECOND.into(), force: true }]
    );
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

    let failed = PrStatus::Failed { failure: PrFailure::Other };
    assert_eq!(
        end,
        WaitEnd::Finished {
            attempt: attempt(SECOND, Phase::Checking, Some(Outcome::InSync)),
            receipt: Some(receipt(SECOND, HeadStatus::Ok, failed)),
        }
    );
    assert!(at >= ms(3_000), "the second holder was waited for too: {at:?}");
    assert_eq!(launches().len(), 2, "one relaunch, no more");
}

#[test]
fn a_forced_worker_that_exits_without_a_receipt_ends_the_wait() {
    let fake = Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync)))).exits(&[Some(ms(200))]);

    let (end, _, at) = run(&fake, true, launching);

    assert_eq!(
        end,
        WaitEnd::Finished { attempt: attempt(OURS, Phase::Checking, Some(Outcome::InSync)), receipt: None }
    );
    assert!(at < ms(300), "a failed publication is bounded: {at:?}");
}

#[test]
fn a_forced_wait_adopts_a_matching_attempt_its_worker_reported() {
    let fake = Fake::new(|t| {
        let outcome = (t >= ms(700)).then_some(Outcome::Fetched);
        with(attempt(OTHER, Phase::Fetching, outcome))
    })
    .receipts(|_, _| Some(receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::SkippedFresh)))
    .head_lock(|t| t < ms(700))
    .exits(&[Some(ms(20))]);

    let (end, _, _) = run(&fake, true, launching);

    let expected = receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::SkippedFresh);
    assert_eq!(
        end,
        WaitEnd::Finished { attempt: attempt(OTHER, Phase::Fetching, Some(Outcome::Fetched)), receipt: Some(expected) }
    );
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

    assert_eq!(
        launches(),
        [LaunchArgs { attempt: OURS.into(), force: true }, LaunchArgs { attempt: SECOND.into(), force: true }]
    );
    match end {
        WaitEnd::Finished { attempt, receipt: Some(receipt) } => {
            assert_eq!(attempt.id, SECOND);
            assert_eq!(receipt.attempt_id, SECOND);
        }
        other => panic!("{other:?}"),
    }
    assert!(at >= ms(1_000), "{at:?}");
}

#[test]
fn an_ordinary_wait_never_relaunches_for_another_branchs_holder() {
    let trunk = Attempt { branch: "trunk".into(), ..attempt(OTHER, Phase::Checking, None) };
    let fake = Fake::new(move |_| with(trunk.clone())).head_lock(|_| true).exits(&[Some(ms(20))]);

    let (end, _, _) = run(&fake, false, launching);

    assert_eq!(end, WaitEnd::Unavailable);
    assert_eq!(launches().len(), 1);
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

#[test]
fn a_forced_attempt_that_fails_its_fetch_still_waits_for_the_receipt() {
    let failed = Outcome::FetchFailed { reason: FetchFailure::Timeout };
    let fake = Fake::new(move |_| with(attempt(OURS, Phase::Fetching, Some(failed))))
        .receipts(|t, _| (t >= ms(300)).then(|| receipt(OURS, HeadStatus::Failed, PrStatus::Ok)))
        .exits(&[Some(ms(300))]);

    let (end, _, _) = run(&fake, true, launching);

    assert!(matches!(end, WaitEnd::Finished { receipt: Some(Receipt { head: HeadStatus::Failed, .. }), .. }), "{end:?}");
}
