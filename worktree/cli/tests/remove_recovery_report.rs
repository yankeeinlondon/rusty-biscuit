//! The reports `wt remove` prints when a removal stops after the checkout
//! moved, refuses before it, or matches a pending removal and another
//! worktree, rendered from library values: the spec's example, a branch
//! failure after the checkout was removed, untrusted text, the listing
//! limits, and the exit code each refusal maps to.

use std::path::{Path, PathBuf};

use biscuit_terminal::components::prose::{LineBreaks, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::query::{FileIdentity, NativeString};
use worktree::remove::lifecycle::{Candidate, Finished, Refusal};
use worktree::remove::outcome::{
    ActivityKind, ActivityLocation, BranchOutcome, BranchState, BranchTarget, EntryCounts, ObservedActivity,
    OperationReport, OriginalState, QuarantineReport, QuarantineState, RegistrationState, RemovalOutcome, ReportNote,
};
use worktree::remove::recovery::OperationId;
use worktree::remove::recovery::lock::LockError;
use worktree::remove::recovery::store::StoreError;
use worktree::worktree::WorktreeEntry;
use worktree_cli::commands::remove::recovery::{ambiguous_markup, refusal_error, stopped_markup};
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
            let error = refusal_error(refusal, "fix-path-spelling", "fix/path-spelling");
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
