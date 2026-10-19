//! The reports `wt remove` prints when a removal stops after the checkout
//! moved, refuses before it, or matches a pending removal and another
//! worktree, rendered from library values: the spec's example, a branch
//! failure after the checkout was removed, untrusted text, the listing
//! limits, the limits of each process check, why watching stopped, a slow
//! watch startup, and the exit code each refusal maps to.

use std::path::{Path, PathBuf};
use std::time::Duration;

use biscuit_terminal::components::prose::{LineBreaks, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::terminal::Terminal;
use serde_json::json;
use sniff::filesystem::query::{FileIdentity, NativeString, PathUsageReport};
use worktree::remove::lifecycle::{Candidate, ExitClass, Finished, Inspection, Presence, Refusal, Resolution};
use worktree::remove::observation::{
    CoverageLost, FsEventsLossFlag, ObservationBackend, ObservationDiagnostics, ObservationError, ObservationLoss,
    ObservationStage, Root,
};
use worktree::remove::outcome::{
    ActivityKind, ActivityLocation, BranchOutcome, BranchState, BranchTarget, DiscoveryPurpose, DiscoveryResult,
    DiscoveryRun, EarlierAttempt, EntryCounts, ObservedActivity, OperationReport, OriginalState, QuarantineReport,
    QuarantineState, RegistrationState, RemovalOutcome, ReportNote,
};
use worktree::remove::recovery::OperationId;
use worktree::remove::recovery::lock::LockError;
use worktree::remove::recovery::record::{
    BranchBinding, IntendedAction, Phase, RECORD_VERSION, RecoveryRecord, RepositoryBinding,
};
use worktree::remove::recovery::store::StoreError;
use worktree::remove::usage::CoverageSummary;
use worktree::worktree::WorktreeEntry;
use worktree_cli::commands::remove::recovery::{
    ambiguous_markup, inspection_markup, refusal_error, slow_startup_markup, stopped_markup,
};
use worktree_cli::commands::remove::usage::coverage_note;
use worktree_cli::exit::exit_code;

const ORIGINAL: &str = "/work/fix-path-spelling";
const QUARANTINE: &str = "/work/.wt-remove-fix-path-spelling-a91c";
const ID: &str = "a91c0000000000000000000000000000";
/// 2026-10-12 14:03:07 UTC.
const AT_MS: u64 = 1_791_295_387_000;

fn render(markup: &str) -> String {
    let terminal = Terminal::builder().is_tty(false).osc_link_support(false).width(100).color_depth(ColorDepth::None).build();
    Prose::new(markup).with_line_breaks(LineBreaks::Hard).render(&terminal)
}

fn native(path: &str) -> NativeString {
    NativeString::from(Path::new(path))
}

fn files(files: u64) -> EntryCounts {
    EntryCounts { files, directories: 0, links: 0 }
}

/// A report for a removal that moved [`ORIGINAL`] to [`QUARANTINE`] and
/// stopped with `outcome`, `retained` entries left there.
fn moved(outcome: RemovalOutcome, retained: u64) -> OperationReport {
    let mut report = OperationReport::new(OperationId::parse(ID), outcome, native(ORIGINAL));
    report.original_state = OriginalState::Moved;
    report.quarantine = Some(QuarantineReport {
        path: native(QUARANTINE),
        identity: FileIdentity::Unix { device: 7, inode: 3 },
        state: QuarantineState::Present,
        retained_entries: retained,
        retained: Vec::new(),
    });
    report.registration = RegistrationState::AtQuarantine;
    report.notes.push(ReportNote::ConcurrentWriteLimit);
    report
}

fn finished(report: OperationReport) -> Finished {
    Finished { report, report_path: PathBuf::from(format!("/repo/.git/wt-removal/{ID}/report.json")), warnings: Vec::new() }
}

fn stopped(finished: &Finished, branch: Option<&str>) -> String {
    render(&stopped_markup(finished, "fix-path-spelling", "fix/path-spelling", branch, Path::new("/repo")))
}

fn not_attempted(report: &mut OperationReport) {
    report.branches = vec![
        BranchOutcome { target: BranchTarget::Local { name: "fix/path-spelling".into() }, state: BranchState::NotAttempted },
        BranchOutcome {
            target: BranchTarget::Remote { remote: "origin".into(), name: "fix/path-spelling".into() },
            state: BranchState::NotAttempted,
        },
    ];
}

#[test]
fn the_specs_example() {
    let mut report = moved(RemovalOutcome::ActivityDetected, 6);
    report.original_state = OriginalState::Recreated;
    report.journal.deleted = files(124);
    report.push_activity(ObservedActivity {
        location: ActivityLocation::Original,
        path: native("/work/fix-path-spelling/.gitnexus/index.db"),
        kind: ActivityKind::Created,
        received_at_ms: AT_MS,
    });
    not_attempted(&mut report);

    insta::assert_snapshot!(stopped(&finished(report), Some("fix/path-spelling")));
}

/// The checkout is verified gone and the remote deletion failed after the
/// local one: "already deleted" and "not deleted" are told apart, Git's
/// multi-line error shows its first line, and no stop guidance is given.
#[test]
fn a_branch_failure_after_the_checkout_was_removed() {
    let mut report = moved(RemovalOutcome::Removed, 0);
    report.quarantine.as_mut().unwrap().state = QuarantineState::Removed;
    report.registration = RegistrationState::Removed;
    report.journal.deleted = EntryCounts { files: 40, directories: 6, links: 1 };
    report.branches = vec![
        BranchOutcome { target: BranchTarget::Local { name: "fix/path-spelling".into() }, state: BranchState::Deleted },
        BranchOutcome {
            target: BranchTarget::Remote { remote: "origin".into(), name: "fix/path-spelling".into() },
            state: BranchState::Failed {
                error: "remote: error: cannot lock ref 'refs/heads/fix/path-spelling'\nerror: failed to push some refs".into(),
            },
        },
    ];
    report.errors.push("a branch action failed".into());

    insta::assert_snapshot!(stopped(&finished(report), Some("fix/path-spelling")));
}

/// Names, paths, and errors from the filesystem and other processes are
/// shown literally: markup is escaped and control characters are spelled
/// out, so none can style the report or forge a line.
#[test]
fn untrusted_text_is_shown_literally() {
    let mut report = OperationReport::new(OperationId::parse(ID), RemovalOutcome::CleanupFailed, native("/work/<red>x</red>"));
    report.original_state = OriginalState::Moved;
    report.quarantine = Some(QuarantineReport {
        path: native("/work/.wt-remove-x\u{1b}[2J"),
        identity: FileIdentity::Unix { device: 7, inode: 3 },
        state: QuarantineState::Unknown,
        retained_entries: 0,
        retained: Vec::new(),
    });
    report.registration = RegistrationState::Unknown;
    report.errors.push("could not delete <b>bold</b>\ncd:/tmp".into());
    let mut finished = finished(report);
    finished.warnings.push("the report could not be saved: <dim>disk full</dim>".into());

    insta::assert_snapshot!(render(&stopped_markup(&finished, "x<i>", "x\ny", Some("b<u>"), Path::new("/repo"))));
}

/// Lost coverage after uncertain deletions: only the first observed changes
/// and errors are listed, the rest counted, and entries without a recorded
/// result are reported as possibly deleted.
#[test]
fn long_lists_are_counted_and_uncertain_entries_named() {
    let mut report = moved(RemovalOutcome::StateUnverified, 1);
    report.journal.deleted = EntryCounts { files: 1, directories: 1, links: 0 };
    report.journal.uncertain = files(2);
    for index in 0..5 {
        report.push_activity(ObservedActivity {
            location: ActivityLocation::Quarantine,
            path: native(&format!("{QUARANTINE}/target/{index}.o")),
            kind: ActivityKind::Modified,
            received_at_ms: AT_MS + index * 1000,
        });
    }
    report.push_activity(ObservedActivity {
        location: ActivityLocation::Quarantine,
        path: native(QUARANTINE),
        kind: ActivityKind::CoverageLost,
        received_at_ms: AT_MS + 9000,
    });
    report.truncation.activity_omitted = 10;
    report.errors = (1..=5).map(|index| format!("error {index}")).collect();

    insta::assert_snapshot!(stopped(&finished(report), None));
}

/// Each refusal names what stopped the removal before anything moved, and
/// maps to its exit code: a changed approval 3, another `wt` holding the
/// lock 4, an operational failure 1.
#[test]
fn refusals_say_nothing_was_removed_and_keep_their_exit_codes() {
    let changed = Refusal::Changed("the branch moved after it was checked".into());
    let busy = Refusal::Store(StoreError::Lock(LockError::Busy));
    let failed = Refusal::Git(worktree::WorktreeError::GitCommand("git exploded".into()));

    let rendered: Vec<String> = [(&changed, 3), (&busy, 4), (&failed, 1)]
        .into_iter()
        .map(|(refusal, code)| {
            let error = refusal_error(refusal, "fix-path-spelling", "fix/path-spelling", false);
            assert_eq!(exit_code(&error), code, "{refusal:?}");
            render(&error.to_string())
        })
        .collect();

    insta::assert_snapshot!(rendered.join("\n---\n"));
}

#[test]
fn a_name_matching_a_pending_removal_and_a_live_worktree() {
    let candidates = [
        Candidate::Live(WorktreeEntry {
            path: PathBuf::from(ORIGINAL),
            branch: Some("fix/path-spelling".into()),
            head_sha: None,
            is_main: false,
            is_current: false,
            prunable: None,
        }),
        Candidate::Pending { id: OperationId::parse(ID).unwrap(), original: PathBuf::from(ORIGINAL), quarantine: PathBuf::from(QUARANTINE) },
    ];

    insta::assert_snapshot!(render(&ambiguous_markup("fix-path-spelling", &candidates)));
}

/// A Sniff report about `path` that found no usage, with `coverage` as
/// `(mechanism, status)` pairs.
fn discovery(path: &str, outcome: &str, coverage: &[(&str, &str)], budget_exhausted: bool) -> PathUsageReport {
    serde_json::from_value(json!({
        "schema_version": 1,
        "target": {
            "requested": {"display": path},
            "resolved": {"display": path},
            "kind": "directory",
            "identity": {"kind": "unix", "device": "7", "inode": "3"},
            "recursive": true
        },
        "outcome": outcome,
        "started_at": "2026-10-12T00:00:00Z",
        "finished_at": "2026-10-12T00:00:01Z",
        "budget_ms": 2000,
        "elapsed_us": 1000,
        "budget_exhausted": budget_exhausted,
        "processes": [],
        "coverage": coverage.iter().map(|(mechanism, status)| json!({
            "mechanism": mechanism, "scope": "target_tree", "status": status, "attempted": null,
            "succeeded": null, "reason": null, "truncation": null, "limitations": []
        })).collect::<Vec<_>>(),
        "limitations": []
    }))
    .expect("valid report")
}

/// The three limited shapes a removal goes ahead with: a usable query that
/// ran out of its budget, one that could not run, and one this system does
/// not support.
fn limited(path: &str) -> [(&'static str, PathUsageReport); 3] {
    [
        ("partial", discovery(path, "usable", &[("open_handles", "partial"), ("loaded_modules", "not_attempted")], true)),
        ("unavailable", discovery(path, "unavailable", &[("process_enumeration", "failed")], false)),
        ("unsupported", discovery(path, "unsupported", &[("open_handles", "unsupported")], false)),
    ]
}

fn complete(path: &str) -> PathUsageReport {
    discovery(path, "usable", &[("process_enumeration", "complete"), ("open_handles", "complete")], false)
}

fn run(purpose: DiscoveryPurpose, path: &str, report: PathUsageReport) -> DiscoveryRun {
    DiscoveryRun { at_ms: AT_MS, purpose, path: native(path), result: DiscoveryResult::Report(Box::new(report)) }
}

fn note(report: &PathUsageReport) -> String {
    coverage_note(&CoverageSummary::of(report)).expect("a limited report has a note")
}

fn stopped_markup_of(report: OperationReport) -> String {
    stopped_markup(&finished(report), "fix-path-spelling", "fix/path-spelling", None, Path::new("/repo"))
}

/// A removal goes ahead despite limited discovery, so a stop says what the
/// check before the move could not see, and what each check after the stop
/// could not see, beside the recovery guidance.
#[test]
fn a_stop_names_the_limits_of_each_process_check() {
    for ((shape, before), (_, after)) in limited(ORIGINAL).into_iter().zip(limited(QUARANTINE)) {
        let mut report = moved(RemovalOutcome::ActivityDetected, 6);
        report.discovery.push(run(DiscoveryPurpose::Preflight, ORIGINAL, before.clone()));
        report.discovery.push(run(DiscoveryPurpose::Diagnostic, QUARANTINE, after.clone()));

        let markup = stopped_markup_of(report);

        let before_line = format!("<dim>Before the move: {}</dim>", note(&before));
        let after_line = format!("<dim>After the stop, at {QUARANTINE}: {}</dim>", note(&after));
        assert!(markup.contains(&before_line), "{shape}: {markup}");
        assert!(markup.contains(&after_line), "{shape}: {markup}");
        let guidance = markup.find("Stop any tasks using either path").expect("guidance");
        assert!(markup.find(&after_line).unwrap() < guidance, "{shape}: coverage precedes the guidance\n{markup}");
    }
}

/// Full coverage before the move and after the stop adds no line.
#[test]
fn a_stop_after_complete_checks_adds_no_coverage_line() {
    let mut report = moved(RemovalOutcome::ActivityDetected, 6);
    report.discovery.push(run(DiscoveryPurpose::Preflight, ORIGINAL, complete(ORIGINAL)));
    report.discovery.push(run(DiscoveryPurpose::Diagnostic, QUARANTINE, complete(QUARANTINE)));

    let markup = stopped_markup_of(report);

    assert!(!markup.contains("Discovery"), "{markup}");
}

/// A diagnostic query that failed says so, escaped; a retry's check before
/// deleting from the moved directory is told apart from the move's; a retry
/// that made no check of its own reports the earlier attempt's, as such.
#[test]
fn a_stop_tells_its_checks_apart() {
    let [(_, partial), ..] = limited(QUARANTINE);
    let mut report = moved(RemovalOutcome::StateUnverified, 1);
    report.discovery.push(run(DiscoveryPurpose::Preflight, QUARANTINE, partial.clone()));
    report.discovery.push(DiscoveryRun {
        at_ms: AT_MS,
        purpose: DiscoveryPurpose::Diagnostic,
        path: native(ORIGINAL),
        result: DiscoveryResult::Failed { kind: "root_identity".into(), message: "denied <b>x</b>\nmore".into() },
    });

    let markup = stopped_markup_of(report);

    assert!(markup.contains(&format!("<dim>Before deleting from the moved directory: {}</dim>", note(&partial))), "{markup}");
    assert!(
        markup.contains(&format!("<dim>After the stop, at {ORIGINAL}: Discovery could not run (denied \\<b\\>x\\</b\\> …).</dim>")),
        "{markup}"
    );

    let [_, (_, unavailable), _] = limited(ORIGINAL);
    let mut retry = moved(RemovalOutcome::CleanupFailed, 0);
    retry.earlier_attempts.push(EarlierAttempt {
        outcome: RemovalOutcome::CleanupFailed,
        original_state: OriginalState::Moved,
        quarantine: None,
        registration: RegistrationState::AtQuarantine,
        timeline: Vec::new(),
        discovery: vec![run(DiscoveryPurpose::Preflight, ORIGINAL, unavailable.clone())],
        activity: Vec::new(),
        activity_omitted: 0,
        branches: Vec::new(),
        errors: Vec::new(),
        observation: None,
    });

    let markup = stopped_markup_of(retry);

    assert!(markup.contains(&format!("<dim>Before the move (earlier attempt): {}</dim>", note(&unavailable))), "{markup}");
}

/// A spec-shaped stop with partial discovery before the move, as rendered.
#[test]
fn a_stop_after_partial_discovery() {
    let mut report = moved(RemovalOutcome::ActivityDetected, 6);
    report.original_state = OriginalState::Recreated;
    report.journal.deleted = files(124);
    report.push_activity(ObservedActivity {
        location: ActivityLocation::Original,
        path: native("/work/fix-path-spelling/.gitnexus/index.db"),
        kind: ActivityKind::Created,
        received_at_ms: AT_MS,
    });
    report.discovery.push(run(
        DiscoveryPurpose::Preflight,
        ORIGINAL,
        discovery(ORIGINAL, "usable", &[("open_handles", "partial"), ("fsevents", "unsupported")], false),
    ));
    not_attempted(&mut report);

    insta::assert_snapshot!(stopped(&finished(report), Some("fix/path-spelling")));
}

fn inspection(discovery: PathUsageReport) -> Inspection {
    let unix = |inode| FileIdentity::Unix { device: 7, inode };
    let id = OperationId::parse(ID).expect("valid id");
    Inspection {
        id: id.clone(),
        record: RecoveryRecord {
            record_version: RECORD_VERSION,
            operation_id: id,
            created_at_ms: AT_MS,
            requested_name: "fix/path-spelling".into(),
            aliases: Vec::new(),
            original: native(ORIGINAL),
            quarantine: native(QUARANTINE),
            checkout_identity: unix(3),
            repository: RepositoryBinding {
                common_dir: native("/repo/.git"),
                common_dir_identity: unix(4),
                admin_dir: native("/repo/.git/worktrees/fix-path-spelling"),
                admin_dir_identity: unix(5),
            },
            branch: BranchBinding::Named("fix/path-spelling".into()),
            approved_tip: "a".repeat(40),
            consent_fingerprint: "b".repeat(64),
            discovery,
            intended_actions: vec![IntendedAction::RemoveCheckout],
            completed_phase: Phase::Deleting,
        },
        original: Presence::Other,
        quarantine: Presence::Checkout,
        registration: RegistrationState::AtQuarantine,
        journal: None,
        torn_tail: false,
        retained: None,
        resolution: Resolution::OriginalRecreated,
    }
}

/// A pending removal that cannot continue describes the saved check's
/// limits as evidence from before the move; a complete one adds nothing.
#[test]
fn a_pending_removal_names_the_limits_of_its_saved_check() {
    for (shape, saved) in limited(ORIGINAL) {
        let markup = inspection_markup(&inspection(saved.clone()), "fix/path-spelling");

        let line = format!("<dim>Saved from before the move: {}</dim>", note(&saved));
        assert!(markup.contains(&line), "{shape}: {markup}");
        assert!(markup.find(&line).unwrap() < markup.find("Nothing was deleted or repaired").unwrap(), "{shape}: {markup}");
    }

    let markup = inspection_markup(&inspection(complete(ORIGINAL)), "fix/path-spelling");

    assert!(!markup.contains("Discovery"), "{markup}");
}

const FSEVENTSD: &str = "fseventsd";
const NOTHING_LOST: &str = "Nothing is known to have been lost";

fn diagnostics(backend: ObservationBackend, stage: ObservationStage, reason: CoverageLost) -> ObservationDiagnostics {
    ObservationDiagnostics {
        backend,
        startup_ms: Some(240),
        last_round_trip_ms: Some(90),
        longest_round_trip_ms: Some(2400),
        loss: Some(ObservationLoss { stage, reason, entries_since_drain: None }),
    }
}

/// [`moved`] with nothing deleted yet, stopped by `observation`'s loss.
fn lost(observation: ObservationDiagnostics) -> Finished {
    let mut report = moved(RemovalOutcome::StateUnverified, 9);
    report.observation = Some(observation);
    finished(report)
}

/// macOS dropped events mid-deletion: every flag and the raw mask are named
/// with the entries deleted since the previous check, and since this is a
/// real drop, not a delay, no fseventsd advice is given. The locations,
/// retry, and report lines stay.
#[test]
fn a_stop_names_the_fsevents_flags_that_dropped_changes() {
    let mut observation = diagnostics(
        ObservationBackend::FsEvents,
        ObservationStage::Deletion,
        CoverageLost::FsEventsDropped {
            raw: 0x0000_000c,
            flags: vec![FsEventsLossFlag::KernelDropped, FsEventsLossFlag::UserDropped],
        },
    );
    observation.loss.as_mut().unwrap().entries_since_drain = Some(512);
    let finished = lost(observation);

    let rendered = stopped(&finished, Some("fix/path-spelling"));

    assert!(rendered.contains("Watching stopped during deletion, 512 entries after the previous check: macOS could not report every change (KernelDropped, UserDropped; flags 0xc)."), "{rendered}");
    assert!(rendered.contains(&format!("Moved to: {QUARANTINE}")), "{rendered}");
    assert!(rendered.contains("Retry: "), "{rendered}");
    assert!(rendered.contains("Full report: /repo/.git/wt-removal/"), "{rendered}");
    assert!(!rendered.contains(FSEVENTSD) && !rendered.contains(NOTHING_LOST), "{rendered}");
    assert_eq!(finished.report.journal.deleted.total(), 0);
    assert_eq!(finished.exit_class(), ExitClass::Failure, "a stop exits 1 even with nothing deleted");
    insta::assert_snapshot!(rendered);
}

/// A macOS stall after the move is a delay, not a known loss: the report
/// says so, points at fseventsd, and gives the longest confirmed check.
#[test]
fn a_macos_stall_after_the_move_points_at_fseventsd() {
    let finished = lost(diagnostics(
        ObservationBackend::FsEvents,
        ObservationStage::PostMove,
        CoverageLost::DeliveryStalled { waited: Duration::from_millis(30_400), watch: None },
    ));

    let rendered = stopped(&finished, None);

    assert!(rendered.contains("Watching stopped after the move: delivery of earlier changes could not be confirmed: nothing arrived for 30 s."), "{rendered}");
    assert!(rendered.contains(NOTHING_LOST), "{rendered}");
    assert!(rendered.contains("macOS's file-change service, fseventsd, may be delayed."), "{rendered}");
    assert!(rendered.contains("Confirmed checks took up to 2.4 s before this."), "{rendered}");
    insta::assert_snapshot!(rendered);
}

/// A Windows ceiling on the parent's watch names the folder that holds the
/// checkout, with no macOS remedy.
#[test]
fn a_windows_ceiling_names_the_watched_folder() {
    let finished = lost(diagnostics(
        ObservationBackend::ReadDirectoryChanges,
        ObservationStage::PreMove,
        CoverageLost::DrainCeiling { waited: Duration::from_millis(4500), watch: Some(Root::Parent) },
    ));

    let rendered = stopped(&finished, None);

    assert!(rendered.contains("Watching stopped before the move: delivery of earlier changes on the folder holding the checkout at /work could not be confirmed within 4.5 s."), "{rendered}");
    assert!(rendered.contains(NOTHING_LOST), "{rendered}");
    assert!(!rendered.contains(FSEVENTSD), "{rendered}");
    insta::assert_snapshot!(rendered);
}

/// A report from before diagnostics existed, or a stop with no loss, adds
/// no line.
#[test]
fn a_stop_without_a_loss_adds_no_watching_line() {
    let without = finished(moved(RemovalOutcome::StateUnverified, 9));
    let mut no_loss = diagnostics(ObservationBackend::FsEvents, ObservationStage::Deletion, CoverageLost::Overflow);
    no_loss.loss = None;

    for finished in [without, lost(no_loss)] {
        let rendered = stopped(&finished, None);

        assert!(!rendered.contains("Watching stopped"), "{rendered}");
    }
}

fn not_delivering(backend: ObservationBackend, reason: CoverageLost) -> Refusal {
    Refusal::Observation(ObservationError::NotDelivering(Box::new(diagnostics(backend, ObservationStage::Startup, reason))))
}

/// A watch that never confirmed delivery refuses as an environment limit
/// (exit 4). A stall says nothing changed and, on macOS, points at
/// fseventsd; a real drop keeps the advice to remove the checkout another
/// way.
#[test]
fn a_watch_that_never_delivered_refuses_with_its_reason() {
    let stalled = not_delivering(
        ObservationBackend::FsEvents,
        CoverageLost::DeliveryStalled { waited: Duration::from_millis(5000), watch: Some(Root::Tree) },
    );
    let overflowed = not_delivering(ObservationBackend::ReadDirectoryChanges, CoverageLost::Overflow);

    let rendered: Vec<String> = [&stalled, &overflowed]
        .into_iter()
        .map(|refusal| {
            let error = refusal_error(refusal, "fix-path-spelling", "fix/path-spelling", false);
            assert_eq!(exit_code(&error), 4, "{refusal:?}");
            render(&error.to_string())
        })
        .collect();

    let [stalled, overflowed] = rendered.as_slice() else { unreachable!() };
    assert!(stalled.contains("the watch did not start delivering changes: delivery of earlier changes on the checkout could not be confirmed: nothing arrived for 5.0 s."), "{stalled}");
    assert!(stalled.contains("again; nothing was changed."), "{stalled}");
    assert!(stalled.contains(FSEVENTSD), "{stalled}");
    assert!(overflowed.contains("the watch did not start delivering changes: the system dropped change notifications (its queue overflowed)."), "{overflowed}");
    assert!(overflowed.contains("removes a checkout only while it can watch it for changes"), "{overflowed}");
    assert!(!overflowed.contains(FSEVENTSD), "{overflowed}");
    insta::assert_snapshot!(rendered.join("\n---\n"));
}

/// A resumed attempt's checkout was already moved, so its refusal says it
/// stopped before deleting more, not before moving.
#[test]
fn a_resumed_refusal_says_it_stopped_before_deleting_more() {
    let refusal = Refusal::Changed("the branch moved after it was checked".into());

    let rendered = render(&refusal_error(&refusal, "fix-path-spelling", "fix/path-spelling", true).to_string());

    assert!(rendered.contains("Nothing was removed."), "{rendered}");
    assert!(rendered.contains("stopped before deleting more of worktree fix-path-spelling"), "{rendered}");
    assert!(!rendered.contains("before moving"), "{rendered}");
}

/// The slow-startup warning gives the startup to one decimal.
#[test]
fn a_slow_startup_warns_with_its_duration() {
    let rendered: Vec<String> =
        [1600, 12_300].into_iter().map(|millis| render(&slow_startup_markup(Duration::from_millis(millis)))).collect();

    assert!(rendered[0].contains("took 1.6 s."), "{}", rendered[0]);
    assert!(rendered[1].contains("took 12.3 s."), "{}", rendered[1]);
    insta::assert_snapshot!(rendered.join("\n---\n"));
}
