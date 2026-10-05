//! [`Listing::timings`]: which stages each path records, that the document
//! reconciles, that the `git` counts cover the work, and that measuring
//! changes nothing the listing does. No assertion depends on host speed.

use crate::timing::{ChildrenKind, Scope, Span, Stage, Timings, WorkerReportStatus};

use super::*;

fn stages(spans: &[Span]) -> Vec<Stage> {
    spans.iter().map(Span::stage).collect()
}

fn timings(run: &Run) -> &Timings {
    run.listing.timings.as_ref().expect("timed")
}

/// The children of the span at `path`.
fn children(run: &Run, path: &[Stage]) -> Vec<Stage> {
    let span = timings(run).span(path).unwrap_or_else(|| panic!("no {path:?}: {:#?}", timings(run)));
    stages(span.children())
}

fn timed(options: ListOptions) -> ListOptions {
    ListOptions { timings: true, ..options }
}

/// The document is a library one and survives its own strict decoder, which
/// rejects any remainder that does not reconcile exactly.
fn assert_reconciles(run: &Run, label: &str) {
    let timings = timings(run);
    assert_eq!(timings.scope(), Scope::Library, "{label}");
    let json = timings.to_json();
    let decoded = Timings::from_json(&json).unwrap_or_else(|e| panic!("{label}: {e}: {timings:#?}"));
    assert_eq!(decoded.to_json(), json, "{label}: whole microseconds round-trip");
}

use Stage::*;

#[test]
#[serial_test::serial]
fn without_an_origin_the_local_reads_hold_the_gathers_and_the_graph_steps() {
    let repo = Repo::new();
    run_git(&repo.main, &["remote", "remove", "origin"]);

    let run = run(&repo.main, Script::finishing(), timed(options(true, false, BUDGET)));

    assert_eq!(stages(timings(&run).spans()), [ReadWorktrees, OriginLookup, PrepareLocal, LocalReads, Commit]);
    assert_eq!(timings(&run).span(&[LocalReads]).map(Span::children_kind), Some(ChildrenKind::Concurrent));
    assert_eq!(children(&run, &[LocalReads]), [LocalGather, GraphHistory]);
    assert_eq!(children(&run, &[LocalReads, LocalGather]), [WorktreeStatus, BranchComparisons]);
    assert_eq!(children(&run, &[LocalReads, GraphHistory]), [ShallowCheck, DefaultTips, LaneAssembly], "a base view");
    assert_reconciles(&run, "no origin");
}

#[test]
#[serial_test::serial]
fn a_followed_wait_makes_the_remote_region_with_the_launch_and_the_follow() {
    let repo = Repo::new();

    let run = run(&repo.main, Script::finishing(), timed(options(false, false, BUDGET)));

    assert_eq!(
        stages(timings(&run).spans()),
        [ReadWorktrees, OriginLookup, PrepareLocal, RemoteAndLocal, RefReread, Commit]
    );
    assert_eq!(children(&run, &[RemoteAndLocal]), [RefreshWorker, PrCacheRead, LocalGather]);
    assert_eq!(children(&run, &[RemoteAndLocal, RefreshWorker]), [WorkerLaunch, WorkerWait]);
    let worker = timings(&run).span(&[RemoteAndLocal, RefreshWorker]).expect("refresh worker");
    assert_eq!(worker.git_calls(), None, "the worker's own work runs in another process: unknown, not zero");
    // The launched worker was asked for timings, and its report travels
    // beside the spans, outside every reconciliation.
    let reports = timings(&run).worker_reports();
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert_eq!((reports[0].launch_index, reports[0].status()), (0, WorkerReportStatus::Complete));
    let halves = reports[0].timings().and_then(|worker| worker.span(&[WorkerHalves])).expect("the halves group");
    assert_eq!(stages(halves.children()), [PrRefresh, HeadRefresh]);
    assert_eq!(timings(&run).worker_report_status(), Some(WorkerReportStatus::Complete));
    assert_reconciles(&run, "remote");
}

#[test]
#[serial_test::serial]
fn without_a_worker_the_stored_answers_are_read_inside_the_local_reads() {
    let repo = Repo::new();

    let run = run(&repo.main, Script::finishing(), timed(ListOptions { worker: None, ..options(true, false, BUDGET) }));

    assert_eq!(stages(timings(&run).spans()), [ReadWorktrees, OriginLookup, PrepareLocal, LocalReads, Commit]);
    assert_eq!(children(&run, &[LocalReads]), [PrCacheRead, LocalGather, GraphHistory]);
    assert_eq!(timings(&run).worker_report_status(), None, "no worker followed, no worker section");
    assert_reconciles(&run, "no worker");
}

#[test]
#[serial_test::serial]
fn a_regather_records_its_input_and_its_reads_but_never_dirtiness_again() {
    let repo = Repo::new();
    let _seam = overlap::Installed::with_mode(Mode::Observe);
    let upstream = repo.upstream_commit("main");
    let script = Script::held(Hold::BothFinished).moving(&[&["update-ref", "refs/remotes/origin/main", &upstream]]);

    let run = run(&repo.feature, script, timed(options(true, true, BUDGET)));

    assert!(run.regathered());
    assert_eq!(
        stages(timings(&run).spans()),
        [ReadWorktrees, OriginLookup, PrepareLocal, RemoteAndLocal, RefReread, Regather, Commit]
    );
    assert_eq!(children(&run, &[Regather]), [PrepareLocal, LocalReads]);
    assert_eq!(children(&run, &[Regather, LocalReads]), [BranchComparisons, GraphHistory]);
    // Both the graph and verbose details are needed: one history, with the
    // verbose reads inside it.
    let focused = [ShallowCheck, DefaultTips, FocusedMergeBase, VerboseDetails, LaneAssembly];
    assert_eq!(children(&run, &[RemoteAndLocal, GraphHistory]), focused);
    assert_eq!(children(&run, &[Regather, LocalReads, GraphHistory]), focused);
    assert!(timings(&run).span(&[RemoteAndLocal, VerboseHistory]).is_none());
    assert_reconciles(&run, "regather");
}

#[test]
#[serial_test::serial]
fn a_fast_forward_that_moves_a_checkout_records_its_refresh() {
    let repo = Repo::new();
    run_git(&repo.main, &["update-ref", "refs/remotes/origin/main", &repo.upstream_commit("main")]);
    let fast_forward = timed(ListOptions { fast_forward: true, ..options(false, false, BUDGET) });

    let run = run(&repo.main, Script::finishing(), fast_forward);

    assert!(matches!(run.listing.ff, Some(FfResult::Moved { checkout: Some(_), .. })), "{:?}", run.listing.ff);
    assert_eq!(
        stages(timings(&run).spans()),
        [ReadWorktrees, OriginLookup, PrepareLocal, RemoteAndLocal, FastForward, RefReread, Regather, Commit, CheckoutRefresh]
    );
    assert_eq!(children(&run, &[Regather, LocalReads]), [BranchComparisons], "no history without a graph or -v");
    assert_reconciles(&run, "--ff");
}

#[test]
#[serial_test::serial]
fn verbose_details_alone_are_a_verbose_history_with_no_shallow_check() {
    let repo = Repo::new();

    let run = run(&repo.feature, Script::finishing(), timed(options(false, true, BUDGET)));

    assert_eq!(children(&run, &[RemoteAndLocal]), [RefreshWorker, PrCacheRead, LocalGather, VerboseHistory]);
    assert_eq!(children(&run, &[RemoteAndLocal, VerboseHistory]), [DefaultTips, FocusedMergeBase, VerboseDetails]);
    assert!(timings(&run).span(&[RemoteAndLocal, GraphHistory]).is_none());
    assert_reconciles(&run, "-v");
}

/// Every library `git` process is counted once, in exactly one top-level
/// span, and each history step's count adds up to its parent's.
#[test]
#[serial_test::serial]
fn the_git_counts_cover_every_call_once() {
    let repo = Repo::new();
    let _seam = overlap::Installed::with_mode(Mode::Observe);
    let upstream = repo.upstream_commit("main");
    let script = Script::held(Hold::BothFinished).moving(&[&["update-ref", "refs/remotes/origin/main", &upstream]]);

    let run = run(&repo.feature, script, timed(options(true, true, BUDGET)));

    let top: Vec<u64> = timings(&run).spans().iter().map(|span| span.git_calls().expect("counted")).collect();
    assert_eq!(top.iter().sum::<u64>(), run.calls.len() as u64, "{top:?} vs {:?}", run.calls);
    for path in [&[RemoteAndLocal, GraphHistory][..], &[Regather, LocalReads, GraphHistory]] {
        let history = timings(&run).span(path).expect("history");
        let steps: u64 = history.children().iter().map(|step| step.git_calls().expect("counted")).sum();
        assert!(steps > 0, "{path:?}");
        assert_eq!(history.git_calls(), Some(steps), "{path:?}: {history:#?}");
    }
    let status = timings(&run).span(&[RemoteAndLocal, LocalGather, WorktreeStatus]).expect("status");
    assert_eq!(status.git_calls(), Some(run.status_walks() as u64), "one status walk per worktree");
}

/// Measuring changes nothing: the same listing, timed and not, makes the same
/// `git` calls and shows the same facts; untimed, it has no document.
#[test]
#[serial_test::serial]
fn timings_on_and_off_do_the_same_work_and_show_the_same_facts() {
    let repo = Repo::new();
    let plain = options(true, true, BUDGET);
    let listings = [plain, timed(plain)].map(|options| {
        repo.clear_cache();
        let mut run = run(&repo.feature, Script::finishing(), options);
        run.calls.sort();
        run
    });
    let [off, on] = &listings;

    assert!(off.listing.timings.is_none() && on.listing.timings.is_some());
    assert_eq!(off.calls, on.calls);
    let facts = |run: &Run| {
        let list = &run.listing.list;
        let dirty: Vec<DirtyStatus> = list.statuses.iter().map(|status| status.dirty).collect();
        (
            list.caption.clone(),
            list.target.clone(),
            list.tree.clone(),
            list.comparisons.clone(),
            dirty,
            run.listing.graph.clone(),
            details(&run.listing.verbose),
            run.listing.regathered,
        )
    };
    assert_eq!(facts(off), facts(on));
}
