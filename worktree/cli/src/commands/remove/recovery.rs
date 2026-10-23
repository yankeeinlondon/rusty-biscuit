//! What `wt remove` prints when the removal lifecycle ends anywhere but a
//! clean removal: a refusal that changed nothing ([`refusal_error`]), a
//! removal that stopped after the checkout moved or whose branch step failed
//! ([`stopped_markup`]), a pending removal a retry cannot continue
//! ([`inspection_markup`]), and a name that matches a pending removal and
//! another worktree ([`ambiguous_markup`]).
//!
//! A stop names why watching for changes stopped, where in the removal, and
//! what is known about the delay ([`ObservationLoss`]); a refusal because the
//! watch never started delivering says the same. A watch that was slow to
//! start is a warning, not a refusal ([`slow_startup_markup`]).
//!
//! The stopped and pending reports also say what each process check could
//! not see ([`coverage_note`]), since a removal goes ahead despite limited
//! discovery; a check that saw everything adds nothing.
//!
//! Every builder returns Prose markup, one line per `\n`, for stderr. Paths,
//! names, and errors come from the filesystem, Git, and other processes, so
//! each is shown through [`visible`]. The full detail is in the saved
//! `report.json`; these reports stay compact and always name it.

use std::path::Path;
use std::time::Duration;

use sniff::filesystem::query::NativeString;
use worktree::WorktreeError;
use worktree::remove::lifecycle::{Candidate, ExitClass, Finished, Inspection, Presence, Refusal, Resolution};
use worktree::remove::observation::inventory::{Change, InventoryRefusal};
use worktree::remove::observation::{
    CoverageLost, ObservationBackend, ObservationDiagnostics, ObservationError, ObservationLoss, ObservationStage, Root,
};
use worktree::remove::outcome::{
    ActivityKind, ActivityLocation, BranchState, BranchTarget, DiscoveryPurpose, DiscoveryResult, DiscoveryRun,
    EntryCounts, OperationReport, OriginalState, QuarantineState, RegistrationState, RemovalOutcome, ReportNote,
};
use worktree::remove::usage::CoverageSummary;
use worktree::remove::quarantine::association::MoveErrorClass;
use worktree::remove::recovery::store::STORE_DIR;

use super::typed_command;
use super::usage::{coverage_note, usage_refusal, visible};

/// Observed changes listed before the rest are counted.
pub const ACTIVITY_LIMIT: usize = 3;
/// Errors listed before the rest are counted.
pub const ERROR_LIMIT: usize = 3;

/// Where a stop or a startup refusal points a macOS user whose delivery
/// confirmation stalled.
const FSEVENTSD: &str = "macOS's file-change service, fseventsd, may be delayed. <i>wt</i> does not restart it; \
    see the troubleshooting section of the <i>wt remove</i> documentation.";

/// The warning for a watch that took `startup` to confirm, longer than
/// [`SLOW_STARTUP`](worktree::remove::lifecycle::SLOW_STARTUP); the removal
/// goes on.
pub fn slow_startup_markup(startup: Duration) -> String {
    format!(
        "<yellow>Warning:</yellow> Confirming that the checkout is being watched for changes took {:.1} s. \
        The removal may take longer, and it stops if that confirmation stalls.",
        startup.as_secs_f64()
    )
}

/// `refusal` as the error `wt remove` exits with: 3 or 4 for a refusal, 1 for
/// an operational failure. Nothing was moved or deleted in any of them.
/// `display_name` names the checkout; `name` is what the user typed;
/// `resumed` says the checkout was already moved by an earlier attempt.
pub fn refusal_error(refusal: &Refusal, display_name: &str, name: &str, resumed: bool) -> WorktreeError {
    if let Refusal::Usage(usage) = refusal {
        return usage_refusal(usage, display_name, name);
    }
    if let Refusal::Recovery(inspection) = refusal {
        return WorktreeError::RefusedToLoseWork(inspection_markup(inspection, name));
    }
    let markup = refusal_markup(refusal, display_name, name, resumed);
    match refusal.exit_class() {
        ExitClass::Blocked => WorktreeError::BlockedByEnvironment(markup),
        ExitClass::Failure => WorktreeError::RemovalFailed(markup),
        ExitClass::Refused | ExitClass::Success => WorktreeError::RefusedToLoseWork(markup),
    }
}

/// The markup for a refusal other than usage or recovery: what stopped the
/// removal and what to do about it. `resumed` says the checkout was already
/// moved by an earlier attempt, so this one stopped before deleting more.
pub fn refusal_markup(refusal: &Refusal, display_name: &str, name: &str, resumed: bool) -> String {
    let rerun = rerun(name);
    let startup_loss = match refusal {
        Refusal::Observation(ObservationError::NotDelivering(diagnostics)) => {
            diagnostics.loss.as_ref().map(|loss| (diagnostics.backend, &loss.reason))
        }
        _ => None,
    };
    let remedy = match (refusal, startup_loss) {
        (_, Some((backend, reason))) if is_delay(reason) => {
            let remedy = format!("Run {rerun} again; nothing was changed.");
            if backend == ObservationBackend::FsEvents { format!("{remedy} {FSEVENTSD}") } else { remedy }
        }
        (Refusal::Observation(_), _) => "<i>wt</i> removes a checkout only while it can watch it for changes. Remove it \
            from another system, or with <i>git worktree remove</i> once nothing uses it."
            .to_string(),
        (Refusal::Move(error), _) if error.class() == MoveErrorClass::BlockedByEnvironment => {
            "Close any program using the checkout, then run it again; on a filesystem that cannot move a folder \
            without risking replacing another (such as WSL's <i>/mnt/&lt;drive&gt;</i>), remove it from Windows \
            or with <i>git worktree remove</i> instead."
                .to_string()
        }
        (Refusal::Inventory(InventoryRefusal::Changed { .. }), _) => {
            format!("Let it finish, or stop it, then run {rerun} again.")
        }
        (Refusal::Inventory(_) | Refusal::Ineligible(_), _) => {
            format!("Nothing was moved. Fix what is named above, then run {rerun} again.")
        }
        _ => format!("Run {rerun} again."),
    };
    let reason = match (refusal, startup_loss) {
        (_, Some((_, reason))) => format!("the watch did not start delivering changes: {}", loss_reason(reason, None)),
        (Refusal::Inventory(InventoryRefusal::Changed { path, change }), None) => format!(
            "something is still writing into it ({} {} while <i>wt</i> was preparing the removal)",
            visible(&path.display().to_string()),
            change_verb(*change)
        ),
        (_, None) => visible(&refusal.to_string()),
    };
    let stopped = if resumed { "stopped before deleting more of" } else { "stopped before moving" };
    format!(
        "<red><b>Nothing was removed.</b></red> <i>wt</i> {stopped} worktree <blue>{}</blue>: {reason}.\n  \
        <dim>{remedy}</dim>",
        visible(display_name)
    )
}

/// The report for a removal that changed something and did not finish: both
/// locations, what was deleted and what is left, observed activity, Git's
/// registration, each branch, that only approved entries were deleted,
/// limited process checks (one line per distinct gap), and what to do: a
/// step for what stopped it when one helps ([`first_step`]), then the
/// retry. `branch` is the worktree's
/// branch, reported kept when no step touched it; `base` is the base
/// checkout a retry runs from.
pub fn stopped_markup(finished: &Finished, display_name: &str, name: &str, branch: Option<&str>, base: &Path) -> String {
    let report = &finished.report;
    let mut lines = Vec::new();
    if report.outcome == RemovalOutcome::Removed {
        // The checkout is verified gone; only a branch step is unfinished.
        lines.push(format!(
            "<yellow><b>Removed worktree</b></yellow> <b>{}</b> <dim>at {}</dim>, but a branch step did not finish.",
            visible(display_name),
            native(&report.original)
        ));
        lines.push(String::new());
    } else {
        lines.push(format!("<red><b>Removal stopped:</b></red> {}.", reason(report)));
        lines.push(String::new());
        lines.extend(location_lines(report));
        lines.push(format!("Git registration: {}", registration(report.registration)));
        if let Some(observation) = &report.observation {
            lines.extend(loss_lines(observation, report));
        }
    }
    lines.extend(branch_lines(report, branch));
    for error in report.errors.iter().take(ERROR_LIMIT) {
        lines.push(format!("<dim>Error: {}</dim>", first_line(error)));
    }
    if report.errors.len() > ERROR_LIMIT {
        lines.push(format!("<dim>…and {} more errors, listed in the saved report</dim>", report.errors.len() - ERROR_LIMIT));
    }
    if report.outcome != RemovalOutcome::Removed && report.journal.deleted.total() > 0 {
        lines.push("Only entries you approved before the move were deleted.".to_string());
        if report.notes.contains(&ReportNote::ConcurrentWriteLimit) {
            lines.push(
                "  <dim>A file written in the instant before <i>wt</i> deleted its path would have been deleted \
                too; the saved report lists every deleted path.</dim>"
                    .to_string(),
            );
        }
    }
    lines.extend(discovery_lines(report));

    lines.push(String::new());
    let retry = format!("Retry: {}", rerun(name));
    // Only a checkout that is still partly there is listed and asked about
    // again; a removed one's retry asks only about the branches.
    let explained = report.outcome != RemovalOutcome::Removed && report.original_state != OriginalState::Kept;
    let explanation = format!(
        "Run it from anywhere outside the moved directory, such as the base checkout at {}. It lists what is \
        left and asks again about anything new or changed before deleting more.",
        visible(&base.display().to_string())
    );
    match first_step(report) {
        Some(step) => {
            lines.push("What to do:".to_string());
            lines.push(format!("  1. {step}"));
            lines.push(format!("  2. {retry}"));
            if explained {
                lines.push(format!("     <dim>{explanation}</dim>"));
            }
        }
        None => {
            lines.push(retry);
            if explained {
                lines.push(format!("  <dim>{explanation}</dim>"));
            }
        }
    }
    lines.push(format!("Full report: {}", visible(&finished.report_path.display().to_string())));
    for warning in &finished.warnings {
        lines.push(format!("<yellow>Warning:</yellow> {}", visible(warning)));
    }
    lines.join("\n")
}

/// The warning for a removal that finished although things changed after
/// the user agreed, or watching stopped: what happened (at most
/// [`ACTIVITY_LIMIT`] changes, then a count) and the saved report. `None`
/// when nothing did.
pub fn agreed_changes_markup(finished: &Finished) -> Option<String> {
    let report = &finished.report;
    let total = report.activity.len() as u64 + report.truncation.activity_omitted;
    if total == 0 {
        return None;
    }
    let mut lines = vec![format!(
        "<yellow>Warning:</yellow> {total} {} after you agreed; <i>wt</i> removed the worktree anyway.",
        if total == 1 { "change happened" } else { "changes happened" }
    )];
    for observed in report.activity.iter().take(ACTIVITY_LIMIT) {
        lines.push(match (observed.kind, report.observation.as_ref().and_then(|observation| observation.loss.as_ref())) {
            (ActivityKind::CoverageLost, Some(loss)) => {
                format!("  watching for changes stopped: {}", loss_reason(&loss.reason, Some(report)))
            }
            (ActivityKind::CoverageLost, None) => "  watching for changes stopped".to_string(),
            (kind, _) if observed.location == ActivityLocation::Original => {
                format!("  {} {} <dim>(outside the worktree; not touched)</dim>", native(&observed.path), activity_verb(kind))
            }
            (kind, _) => format!("  {} {}", native(&observed.path), activity_verb(kind)),
        });
    }
    if total > ACTIVITY_LIMIT as u64 {
        lines.push(format!("  <dim>…and {} more, listed in the saved report</dim>", total - ACTIVITY_LIMIT as u64));
    }
    lines.push(format!("Full report: {}", visible(&finished.report_path.display().to_string())));
    Some(lines.join("\n"))
}

/// Why watching stopped and where, and for a delay what it means, the
/// macOS remedy, and how long confirmed checks took before it.
fn loss_lines(observation: &ObservationDiagnostics, report: &OperationReport) -> Vec<String> {
    let Some(ObservationLoss { stage, reason, entries_since_drain }) = &observation.loss else {
        return Vec::new();
    };
    let stage = match stage {
        ObservationStage::Startup => "while the watch was starting".to_string(),
        ObservationStage::PreMove => "before the move".to_string(),
        ObservationStage::PostMove => "after the move".to_string(),
        ObservationStage::Deletion => match entries_since_drain {
            Some(1) => "during deletion, 1 entry after the previous check".to_string(),
            Some(count) => format!("during deletion, {count} entries after the previous check"),
            None => "during deletion".to_string(),
        },
        ObservationStage::Unregistration => "while removing Git's record of the worktree".to_string(),
    };
    let mut lines = vec![format!("Watching stopped {stage}: {}.", loss_reason(reason, Some(report)))];
    if is_delay(reason) {
        lines.push(
            "  <dim>Nothing is known to have been lost; <i>wt</i> stops because it cannot confirm that every \
            change arrived.</dim>"
                .to_string(),
        );
        if observation.backend == ObservationBackend::FsEvents {
            lines.push(format!("  <dim>{FSEVENTSD}</dim>"));
        }
        if let Some(longest) = observation.longest_round_trip_ms {
            lines.push(format!("  <dim>Confirmed checks took up to {} s before this.</dim>", seconds(Duration::from_millis(longest))));
        }
    }
    lines
}

/// A stall or an expired ceiling: delivery could not be confirmed, which
/// does not show that anything was dropped.
fn is_delay(reason: &CoverageLost) -> bool {
    matches!(reason, CoverageLost::DeliveryStalled { .. } | CoverageLost::DrainCeiling { .. })
}

/// Why coverage was lost, in the user's terms. With `report`, a watched
/// place is named by its path; a startup refusal has none yet.
fn loss_reason(reason: &CoverageLost, report: Option<&OperationReport>) -> String {
    let on = |watch: &Option<Root>| watch.map(|root| format!(" on {}", place(root, report))).unwrap_or_default();
    match reason {
        CoverageLost::Overflow => "the system dropped change notifications (its queue overflowed)".to_string(),
        CoverageLost::FsEventsDropped { raw, flags } => {
            let names: Vec<String> = flags.iter().map(ToString::to_string).collect();
            format!("macOS could not report every change ({}; flags {raw:#x})", names.join(", "))
        }
        CoverageLost::QueueFull { capacity } => format!("more than {capacity} changes arrived between two checks"),
        CoverageLost::WatchLost { root } => format!("the watch on {} stopped", place(*root, report)),
        CoverageLost::ReadFailed { message } => format!("reading changes failed ({})", first_line(message)),
        CoverageLost::DeliveryStalled { waited, watch } => {
            format!("delivery of earlier changes{} could not be confirmed: nothing arrived for {} s", on(watch), seconds(*waited))
        }
        CoverageLost::DrainCeiling { waited, watch } => {
            format!("delivery of earlier changes{} could not be confirmed within {} s", on(watch), seconds(*waited))
        }
    }
}

/// The watched place `root`: the checkout where it is now, the folder that
/// holds it, or Git's record.
fn place(root: Root, report: Option<&OperationReport>) -> String {
    match (root, report) {
        (Root::Tree, Some(report)) => {
            let path = report.quarantine.as_ref().map_or(&report.original, |quarantine| &quarantine.path);
            format!("the checkout at {}", native(path))
        }
        (Root::Tree, None) => "the checkout".to_string(),
        (Root::Parent, report) => match report.and_then(|report| report.original.as_path().parent().map(Path::to_path_buf)) {
            Some(parent) => format!("the folder holding the checkout at {}", visible(&parent.display().to_string())),
            None => "the folder holding the checkout".to_string(),
        },
        (Root::Admin, _) => "Git's record of the worktree".to_string(),
    }
}

/// Whole seconds from 10 s up, else one decimal.
fn seconds(duration: Duration) -> String {
    let seconds = duration.as_secs_f64();
    if seconds >= 10.0 { format!("{seconds:.0}") } else { format!("{seconds:.1}") }
}

/// Both locations, what was deleted, what is left, and what was observed.
fn location_lines(report: &OperationReport) -> Vec<String> {
    let mut lines = vec![format!("Original: {}{}", native(&report.original), original_note(report.original_state))];
    if let Some(quarantine) = &report.quarantine {
        let state = match quarantine.state {
            QuarantineState::Present => "",
            QuarantineState::Removed => " <dim>(already removed)</dim>",
            QuarantineState::Unknown => " <dim>(could not be inspected)</dim>",
        };
        lines.push(format!("Moved to: {}{state}", native(&quarantine.path)));
    }
    lines.push(format!("Deleted before stopping: {}", deleted_phrase(&report.journal.deleted)));
    if report.journal.uncertain.total() > 0 {
        lines.push(format!(
            "Possibly deleted: {} <dim>(no result was recorded for them)</dim>",
            counts(&report.journal.uncertain)
        ));
    }
    if let Some(quarantine) = &report.quarantine
        && quarantine.state == QuarantineState::Present
    {
        lines.push(format!("Remaining: {} in the moved directory", entries(quarantine.retained_entries)));
    }
    for observed in report.activity.iter().take(ACTIVITY_LIMIT) {
        lines.push(format!(
            "Observed: {} {} <dim>(reported at {})</dim>",
            native(&observed.path),
            activity_verb(observed.kind),
            clock(observed.received_at_ms)
        ));
    }
    let omitted = report.activity.len().saturating_sub(ACTIVITY_LIMIT) as u64 + report.truncation.activity_omitted;
    if omitted > 0 {
        lines.push(format!("<dim>…and {omitted} more changes, listed in the saved report</dim>"));
    }
    lines
}

/// One line per process check whose coverage was limited: the last check
/// before a mutation (from an earlier attempt when this one made none), and
/// each diagnostic check after this stop. A check that saw everything adds
/// nothing.
fn discovery_lines(report: &OperationReport) -> Vec<String> {
    let current = report.discovery.iter().any(|run| run.purpose == DiscoveryPurpose::Preflight);
    let earlier = report
        .earlier_attempts
        .iter()
        .rev()
        .flat_map(|attempt| attempt.discovery.iter().rev())
        .find(|run| run.purpose == DiscoveryPurpose::Preflight)
        .filter(|_| !current);
    earlier
        .map(|run| (run, " (earlier attempt)"))
        .into_iter()
        .chain(report.discovery.iter().map(|run| (run, "")))
        .filter_map(|(run, attempt)| {
            let context = match run.purpose {
                DiscoveryPurpose::Preflight if run.path == report.original => format!("Before the move{attempt}"),
                DiscoveryPurpose::Preflight => format!("Before deleting from the moved directory{attempt}"),
                DiscoveryPurpose::Diagnostic => format!("After the stop, at {}", native(&run.path)),
                // The recheck after it is the decisive preflight.
                DiscoveryPurpose::WindowsSearchHold => return None,
            };
            run_note(run).map(|note| (context, note))
        })
        .fold(Vec::<(Vec<String>, String)>::new(), |mut merged, (context, note)| {
            // The same gap seen before the move and after the stop is said once.
            match merged.iter_mut().find(|(_, seen)| *seen == note) {
                Some((contexts, _)) => contexts.push(lowercase_first(&context)),
                None => merged.push((vec![context], note)),
            }
            merged
        })
        .into_iter()
        .map(|(contexts, note)| format!("<dim>{}: {note}</dim>", join_and(&contexts)))
        .collect()
}

/// `context` with its first letter lowercased, to follow another in one
/// sentence.
fn lowercase_first(context: &str) -> String {
    let mut chars = context.chars();
    chars.next().map(|first| first.to_lowercase().chain(chars).collect()).unwrap_or_default()
}

/// `a`, `a and b`, `a, b and c`.
fn join_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// [`coverage_note`] for `run`, or why the query itself failed.
fn run_note(run: &DiscoveryRun) -> Option<String> {
    match &run.result {
        DiscoveryResult::Report(report) => coverage_note(&CoverageSummary::of(report)),
        DiscoveryResult::Failed { message, .. } => Some(format!("Discovery could not run ({}).", first_line(message))),
    }
}

/// The first non-empty line of `error`, shown safely; Git's errors run to
/// several lines, and the saved report keeps them whole.
fn first_line(error: &str) -> String {
    let mut lines = error.lines().map(str::trim).filter(|line| !line.is_empty());
    let first = lines.next().unwrap_or(error);
    if lines.next().is_some() { format!("{} …", visible(first)) } else { visible(first) }
}

/// The headline reason, from the outcome and the first observed change.
fn reason(report: &OperationReport) -> String {
    match report.outcome {
        RemovalOutcome::Removed => "a branch step did not finish".to_string(),
        RemovalOutcome::ActivityDetected => match report.activity.first().map(|observed| (observed.location, observed.kind)) {
            Some((ActivityLocation::Original, ActivityKind::Created)) => "new files appeared at the original path".to_string(),
            Some((ActivityLocation::Original, _)) => "something changed at the original path".to_string(),
            Some((ActivityLocation::Quarantine, _)) => "files changed in the moved checkout".to_string(),
            Some((ActivityLocation::Admin, _)) => match git_change(report) {
                Some(GitChange::Index) => "a Git command ran in this worktree while it was being removed".to_string(),
                Some(GitChange::Head) => {
                    "a Git command changed this worktree's HEAD while it was being removed".to_string()
                }
                None => "Git's record of the worktree changed".to_string(),
            },
            None => "unexpected activity was observed".to_string(),
        },
        RemovalOutcome::StateUnverified => {
            if report.activity.iter().any(|observed| observed.kind == ActivityKind::CoverageLost) {
                "watching for changes stopped working, so what happened next can't be verified".to_string()
            } else {
                "the result could not be verified".to_string()
            }
        }
        RemovalOutcome::CleanupFailed => match report.errors.first() {
            Some(error) => format!("a step failed ({})", first_line(error)),
            None => "a step failed".to_string(),
        },
        RemovalOutcome::UsageRefused => "another process is using the checkout".to_string(),
    }
}

/// A change in Git's record of the worktree that only a Git command makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitChange {
    /// `index` or `index.lock`: `git status` refreshing it is enough.
    Index,
    /// `HEAD` or `HEAD.lock`: a checkout, commit, or reset.
    Head,
}

/// The [`GitChange`] behind the first observed change, when it is one.
fn git_change(report: &OperationReport) -> Option<GitChange> {
    let first = report.activity.first().filter(|observed| observed.location == ActivityLocation::Admin)?;
    match first.path.as_path().file_name()?.to_str()? {
        "index" | "index.lock" => Some(GitChange::Index),
        "HEAD" | "HEAD.lock" => Some(GitChange::Head),
        _ => None,
    }
}

/// What to do before retrying a removal stopped by observed activity, or
/// `None` when the retry alone is the remedy.
fn first_step(report: &OperationReport) -> Option<String> {
    if report.outcome != RemovalOutcome::ActivityDetected {
        return None;
    }
    if report.original_state == OriginalState::Recreated {
        return Some(format!(
            "Move or save what is now at {}; <i>wt</i> never deletes it, and the retry refuses until it is gone.",
            native(&report.original)
        ));
    }
    if git_change(report).is_some() {
        return Some(
            "Let that Git command finish. A shell prompt that shows Git status, an editor's Git integration, or a \
            <i>git</i> command in another terminal can start one; don't run Git in the moved directory until the \
            removal finishes."
                .to_string(),
        );
    }
    let observed = report.activity.iter().find(|observed| observed.kind != ActivityKind::CoverageLost)?;
    Some(format!("Stop whatever is writing to {}.", native(&observed.path)))
}

fn original_note(state: OriginalState) -> &'static str {
    match state {
        OriginalState::Kept => " <dim>(not moved)</dim>",
        OriginalState::Moved => "",
        OriginalState::Recreated => " <yellow>(something new is here; it was not touched)</yellow>",
        OriginalState::Unknown => " <dim>(could not be inspected)</dim>",
    }
}

fn registration(state: RegistrationState) -> &'static str {
    match state {
        RegistrationState::AtOriginal => "still points to the original path",
        RegistrationState::AtQuarantine => "now points to the moved directory",
        RegistrationState::Removed => "removed",
        RegistrationState::Unknown => "could not be read",
    }
}

/// One line per branch. "Kept" and "already deleted" are told apart, so a
/// later failure never hides an earlier deletion; `branch`, when no step
/// touched it, is kept.
fn branch_lines(report: &OperationReport, branch: Option<&str>) -> Vec<String> {
    let mut lines: Vec<String> = report
        .branches
        .iter()
        .map(|outcome| {
            let label = match &outcome.target {
                BranchTarget::Local { name } => format!("Branch <b>{}</b>", visible(name)),
                BranchTarget::Remote { remote, name } => format!("Branch <b>{}/{}</b>", visible(remote), visible(name)),
            };
            let state = match &outcome.state {
                BranchState::Kept => "kept".to_string(),
                BranchState::NotAttempted => "kept <dim>(not attempted)</dim>".to_string(),
                BranchState::Deleted => "already deleted".to_string(),
                BranchState::Failed { error } => format!("<red>not deleted</red> <dim>({})</dim>", first_line(error)),
            };
            format!("{label}: {state}")
        })
        .collect();
    let touched = report.branches.iter().any(|outcome| matches!(&outcome.target, BranchTarget::Local { .. }));
    if let Some(branch) = branch
        && !touched
    {
        lines.insert(0, format!("Branch <b>{}</b>: kept", visible(branch)));
    }
    lines
}

fn change_verb(change: Change) -> &'static str {
    match change {
        Change::Appeared => "was created",
        Change::Vanished => "disappeared",
        Change::Replaced => "was replaced",
        Change::Modified => "was modified",
    }
}

fn activity_verb(kind: ActivityKind) -> &'static str {
    match kind {
        ActivityKind::Created => "was created",
        ActivityKind::Modified => "was modified",
        ActivityKind::Replaced => "was replaced",
        ActivityKind::Vanished => "disappeared",
        ActivityKind::CoverageLost => "stopped being watched",
    }
}

/// `124 files and 3 folders (listed in the saved report)`, or `nothing`.
fn deleted_phrase(deleted: &EntryCounts) -> String {
    if deleted.total() == 0 {
        return "nothing".to_string();
    }
    format!("{} <dim>(listed in the saved report)</dim>", counts(deleted))
}

/// `3 files, 1 folder and 2 links`, leaving out zero counts.
pub fn counts(counts: &EntryCounts) -> String {
    let parts: Vec<String> = [(counts.files, "file", "files"), (counts.directories, "folder", "folders"), (counts.links, "link", "links")]
        .into_iter()
        .filter(|(count, _, _)| *count > 0)
        .map(|(count, one, many)| format!("{count} {}", if count == 1 { one } else { many }))
        .collect();
    match parts.as_slice() {
        [] => "0 entries".to_string(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn entries(count: u64) -> String {
    format!("{count} {}", if count == 1 { "entry" } else { "entries" })
}

/// `HH:MM:SS UTC` for milliseconds since the Unix epoch.
fn clock(at_ms: u64) -> String {
    let seconds = (at_ms / 1000) % 86_400;
    format!("{:02}:{:02}:{:02} UTC", seconds / 3600, seconds / 60 % 60, seconds % 60)
}

/// Why a pending removal cannot be continued, what is where, the limits of
/// the saved process check, and the manual way out. Nothing was changed by the run that prints it.
pub fn inspection_markup(inspection: &Inspection, name: &str) -> String {
    let record = &inspection.record;
    let mut lines = vec![
        format!(
            "<red><b>Can't continue the removal of {}:</b></red> {}.",
            visible(&record.original.display()),
            visible(&inspection.resolution.to_string())
        ),
        String::new(),
        format!("Original: {}{}", native(&record.original), presence_note(&inspection.original)),
        format!("Moved to: {}{}", native(&record.quarantine), presence_note(&inspection.quarantine)),
        format!("Git registration: {}", registration(inspection.registration)),
    ];
    if let Some(journal) = &inspection.journal {
        lines.push(format!("Deleted so far: {}", if journal.deleted.total() == 0 { "nothing".to_string() } else { counts(&journal.deleted) }));
        if journal.uncertain.total() > 0 {
            lines.push(format!("Possibly deleted: {} <dim>(no result was recorded for them)</dim>", counts(&journal.uncertain)));
        }
    }
    if let Some(retained) = &inspection.retained {
        lines.push(format!("Remaining: {} in the moved directory", entries(retained.len() as u64)));
    }
    if let Some(note) = coverage_note(&CoverageSummary::of(&record.discovery)) {
        lines.push(format!("<dim>Saved from before the move: {note}</dim>"));
    }
    lines.push(String::new());
    lines.push("<b>Nothing was deleted or repaired by this run.</b>".to_string());
    let rerun = rerun(name);
    lines.push(match &inspection.resolution {
        Resolution::OriginalRecreated => format!(
            "Move or save what is now at the original path, then retry: {rerun}"
        ),
        Resolution::PartialCheckout => format!(
            "Git can no longer read what is left in the moved directory. Save what you need from it and delete it \
            yourself; {rerun} then removes Git's record of it."
        ),
        _ => "Inspect both locations; nothing is cleaned up automatically.".to_string(),
    });
    let dir = record.repository.common_dir.as_path().join(STORE_DIR).join(inspection.id.as_str());
    lines.push(format!("Recovery record: {}", visible(&dir.display().to_string())));
    lines.join("\n")
}

fn presence_note(presence: &Presence) -> String {
    match presence {
        Presence::Absent => " <dim>(gone)</dim>".to_string(),
        Presence::Checkout => " <dim>(the checkout)</dim>".to_string(),
        Presence::Other => " <yellow>(something else is here)</yellow>".to_string(),
        Presence::Uninspectable(error) => format!(" <dim>(could not be inspected: {})</dim>", visible(error)),
    }
}

/// The refusal for a name that matches a pending removal and another
/// worktree (or two pending removals). Nothing is picked.
pub fn ambiguous_markup(name: &str, candidates: &[Candidate]) -> String {
    let mut lines = vec![
        format!(
            "<red><b>Nothing was removed.</b></red> <b>{}</b> matches a pending removal and another worktree:",
            visible(name)
        ),
        String::new(),
    ];
    for candidate in candidates {
        lines.push(match candidate {
            Candidate::Live(entry) => format!(
                "  worktree at {} <dim>(branch {})</dim>",
                visible(&entry.path.display().to_string()),
                visible(entry.branch.as_deref().unwrap_or("detached"))
            ),
            Candidate::Pending { id, original, quarantine } => format!(
                "  pending removal of {} <dim>(moved to {}, operation {})</dim>",
                visible(&original.display().to_string()),
                visible(&quarantine.display().to_string()),
                visible(id.as_str())
            ),
        });
    }
    lines.push(String::new());
    lines.push(
        "<dim>Neither is removed while the name means both. Rename or remove the other worktree with \
        <i>git worktree</i>, then retry.</dim>"
            .to_string(),
    );
    lines.join("\n")
}

fn rerun(name: &str) -> String {
    typed_command("wt remove", name).unwrap_or_else(|| "<i>wt remove</i> with this worktree's name".to_string())
}

fn native(value: &NativeString) -> String {
    visible(&value.display())
}
