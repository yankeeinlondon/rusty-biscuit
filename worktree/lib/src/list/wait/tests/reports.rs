//! Worker reports in the wait: each owned launch's report is taken from the
//! receipt reads the wait makes anyway, before that receipt is deleted, and is
//! never a reason to wait longer or to change which outcome wins. Every case
//! runs the same scripted wait untimed and timed and compares the two.

use super::*;
use crate::list::{RemoteAnswers, worker_reports};
use crate::timing::{Stage, WorkerReportStatus, worker_fixture};

fn complete() -> LaunchReport {
    LaunchReport::from_worker(worker_fixture(&[Stage::PrRefresh, Stage::HeadRefresh]))
}

/// What a worker whose PR half panicked writes: the head half alone.
fn partial() -> LaunchReport {
    LaunchReport::from_worker(worker_fixture(&[Stage::HeadRefresh]))
}

fn timed(receipt: Receipt, durations: LaunchReport) -> Receipt {
    Receipt { durations, ..receipt }
}

fn entry(launch_index: u32, attempt_id: &str, report: LaunchReport) -> WorkerReport {
    WorkerReport { launch_index, attempt_id: attempt_id.into(), report }
}

/// `script`, refusing any read after the wait has discarded a receipt: a
/// report is captured before deletion, never by a read after it.
fn read_before_discard(
    script: impl Fn(Duration, &Attempt) -> Option<Receipt> + 'static,
) -> impl Fn(Duration, &Attempt) -> Option<Receipt> {
    move |t, attempt| {
        assert!(discarded().is_empty(), "a receipt was read after the wait discarded one");
        script(t, attempt)
    }
}

/// Runs the wait `fake` makes untimed, then timed, and returns the timed
/// end after checking that timing changed nothing else: the same outcomes,
/// the same end time, the same receipts discarded, and a timings request
/// only on the timed run's launches.
fn untimed_then_timed(fake: impl Fn() -> Fake, force: bool, launch: WorkerLaunch) -> WaitEnd {
    let run = |timings: bool| {
        let fake = fake();
        let end = wait(WaitRequest { timings, ..request(force) }, &fake, launch, &mut |_| {});
        (end, now(), discarded(), launches())
    };
    let (untimed, untimed_at, untimed_discarded, untimed_launches) = run(false);
    let (timed, timed_at, timed_discarded, timed_launches) = run(true);

    assert!(untimed.worker_reports.is_empty(), "an untimed wait keeps no reports: {untimed:?}");
    assert!(untimed_launches.iter().all(|args| !args.timings), "nor asks a worker for timings");
    assert!(timed_launches.iter().all(|args| args.timings), "{timed_launches:?}");
    assert_eq!(WaitEnd { worker_reports: Vec::new(), ..timed.clone() }, untimed, "the same outcomes win");
    assert_eq!(timed_at, untimed_at, "timing never extends the wait");
    assert_eq!(timed_discarded, untimed_discarded, "every launched receipt is still discarded");
    timed
}

/// The library's summary of a wait that kept its `origin`.
fn summary(end: &WaitEnd) -> (Vec<WorkerReport>, WorkerReportStatus) {
    let remote = RemoteAnswers { origin: Some("origin".into()), waited: Some(end.clone()), ..RemoteAnswers::default() };
    worker_reports(&remote).expect("a followed worker has a summary")
}

#[test]
fn a_report_is_taken_from_the_receipt_before_the_receipt_is_deleted() {
    let fake = || {
        Fake::new(|t| with(attempt(OURS, Phase::Checking, (t >= ms(300)).then_some(Outcome::InSync))))
            .receipts(read_before_discard(|t, for_attempt| {
                (t >= ms(300)).then(|| timed(receipt(&for_attempt.id, HeadStatus::Ok, PrStatus::Ok), complete()))
            }))
            .exits(&[Some(ms(300))])
    };

    let end = untimed_then_timed(fake, false, launching);

    assert_eq!((&end.head, &end.prs), (&in_sync(OURS), &PrEnd::Published));
    assert_eq!(end.worker_reports, [entry(0, OURS, complete())]);
    assert_eq!(discarded(), [OURS], "and the receipt is deleted afterward");
    assert_eq!(summary(&end), (end.worker_reports.clone(), WorkerReportStatus::Complete));
}

#[test]
fn an_early_publication_ends_the_wait_with_the_report_missing() {
    let fake = || {
        Fake::new(|t| with(attempt(OURS, Phase::Checking, (t >= ms(100)).then_some(Outcome::InSync))))
            .pr_answer(|t| Some(if t < ms(100) { SEEDED } else { PUBLISHED }))
            .receipts(read_before_discard(|t, for_attempt| {
                (t >= ms(1_000)).then(|| timed(receipt(&for_attempt.id, HeadStatus::Ok, PrStatus::Ok), complete()))
            }))
            .exits(&[Some(ms(1_000))])
    };

    let end = untimed_then_timed(fake, false, launching);

    assert_eq!((&end.prs, end.timed_out), (&PrEnd::Published, false));
    assert!(now() < ms(1_000), "the wait never waits for the report: {:?}", now());
    assert_eq!(end.worker_reports, [entry(0, OURS, LaunchReport::Missing)]);
    assert_eq!(summary(&end).1, WorkerReportStatus::Missing);
}

#[test]
fn a_timeout_leaves_the_report_missing() {
    let fake = || Fake::new(|_| with(attempt(OURS, Phase::Checking, None)));

    let end = untimed_then_timed(fake, false, launching);

    assert!(end.timed_out);
    assert!(within_the_budget(now(), ORDINARY_BUDGET), "{:?}", now());
    assert_eq!(end.worker_reports, [entry(0, OURS, LaunchReport::Missing)]);
}

#[test]
fn an_adopted_head_keeps_our_own_pr_report_and_is_adopted_without_one() {
    let adopting = |durations: LaunchReport| {
        move || {
            let durations = durations.clone();
            Fake::new(|t| with(attempt(OTHER, Phase::Checking, (t >= ms(900)).then_some(Outcome::InSync))))
                .receipts(read_before_discard(move |t, _| {
                    (t >= ms(20)).then(|| timed(receipt(OURS, HeadStatus::AdoptedElsewhere, PrStatus::Ok), durations.clone()))
                }))
                .exits(&[Some(ms(20))])
                .head_lock(|t| t < ms(900))
        }
    };

    // Our worker's PR half ran and measured itself: an owned report.
    let end = untimed_then_timed(adopting(complete()), false, launching);
    assert_eq!(end.head, in_sync(OTHER), "the adopted head is followed");
    assert_eq!(end.worker_reports, [entry(0, OURS, complete())], "never the adopted attempt's");
    assert_eq!(summary(&end).1, WorkerReportStatus::Complete, "adoption does not hide an owned report");

    let end = untimed_then_timed(adopting(LaunchReport::Missing), false, launching);
    assert_eq!(end.worker_reports, [entry(0, OURS, LaunchReport::Missing)]);
    assert_eq!(summary(&end).1, WorkerReportStatus::Adopted);
}

#[test]
fn a_forced_retry_is_a_second_entry_never_summed_into_the_first() {
    let fake = || {
        Fake::new(|t| {
            let id = if t < ms(2_000) { OURS } else { SECOND };
            with(attempt(id, Phase::Checking, Some(Outcome::InSync)))
        })
        .receipts(read_before_discard(|_, for_attempt| {
            Some(match for_attempt.id.as_str() {
                OURS => timed(receipt(OURS, HeadStatus::Ok, PrStatus::Contended), complete()),
                _ => timed(receipt(SECOND, HeadStatus::Ok, PrStatus::Ok), partial()),
            })
        }))
        .pr_lock(|t| t < ms(2_000))
        .pr_answer(|_| Some(SEEDED))
        .exits(&[Some(ms(10)), Some(ms(2_100))])
    };

    let end = untimed_then_timed(fake, true, launching);

    assert_eq!((&end.head, &end.prs), (&in_sync(SECOND), &PrEnd::Published));
    assert_eq!(end.worker_reports, [entry(0, OURS, complete()), entry(1, SECOND, partial())]);
    assert_eq!(summary(&end).1, WorkerReportStatus::Partial);
}

#[test]
fn a_half_that_panicked_is_a_partial_report_beside_the_unchanged_outcome() {
    // The worker records a panicked PR half as a generic failure.
    let fake = || {
        Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
            .receipts(read_before_discard(|t, for_attempt| {
                let prs = PrStatus::Failed { failure: PrFailure::Other };
                (t >= ms(50)).then(|| timed(receipt(&for_attempt.id, HeadStatus::Ok, prs), partial()))
            }))
            .exits(&[Some(ms(50))])
    };

    let end = untimed_then_timed(fake, false, launching);

    assert_eq!(end.prs, other());
    assert_eq!(end.worker_reports, [entry(0, OURS, partial())]);
    assert_eq!(summary(&end).1, WorkerReportStatus::Partial);
}

#[test]
fn an_unreadable_report_is_invalid_and_the_outcome_stands() {
    let fake = || {
        Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
            .receipts(read_before_discard(|t, for_attempt| {
                (t >= ms(50)).then(|| timed(receipt(&for_attempt.id, HeadStatus::Ok, PrStatus::Unsupported), LaunchReport::Invalid))
            }))
            .exits(&[Some(ms(50))])
    };

    let end = untimed_then_timed(fake, false, launching);

    assert_eq!(end.prs, PrEnd::Unsupported);
    assert_eq!(end.worker_reports, [entry(0, OURS, LaunchReport::Invalid)]);
    assert_eq!(summary(&end).1, WorkerReportStatus::Invalid);
}

#[test]
fn a_worker_that_cannot_start_has_no_entry() {
    let end = untimed_then_timed(|| Fake::new(|_| empty()), false, failing_launch);

    assert_eq!(end.head, HeadEnd::Unavailable);
    assert!(end.worker_reports.is_empty());
    assert_eq!(summary(&end), (Vec::new(), WorkerReportStatus::Missing));
}

#[test]
fn a_changed_origin_suppresses_every_report() {
    let fake = || {
        Fake::new(|_| with(attempt(OURS, Phase::Checking, Some(Outcome::InSync))))
            .receipts(|t, for_attempt| (t >= ms(50)).then(|| timed(receipt(&for_attempt.id, HeadStatus::Ok, PrStatus::Ok), complete())))
            .exits(&[Some(ms(50))])
    };
    let end = untimed_then_timed(fake, false, launching);
    assert_eq!(end.worker_reports, [entry(0, OURS, complete())], "control: the wait kept a report");

    let remote = RemoteAnswers { origin: Some("origin".into()), origin_changed: true, waited: Some(end), ..RemoteAnswers::default() };
    assert_eq!(worker_reports(&remote), Some((Vec::new(), WorkerReportStatus::OriginChanged)));
    assert_eq!(worker_reports(&RemoteAnswers::default()), None, "no wait, no worker section");
}
