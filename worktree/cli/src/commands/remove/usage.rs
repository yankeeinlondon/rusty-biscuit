//! What `wt remove` says about other processes using the checkout: the
//! report section that lists them before the question ([`holders_markup`]),
//! which never stops the removal, and the refusal when the usage check
//! could not verify the checkout at all (exit 3, nothing moved).
//!
//! Names and paths come from other processes and the filesystem, so every
//! one is shown with control characters escaped and Prose markup escaped.

use sniff::filesystem::query::{CoverageStatus, EvidenceKind, FileIdentity, Mechanism, NativeString, Outcome, PathUsageError};
use worktree::WorktreeError;
use worktree::remove::usage::{CoverageSummary, EvidenceLocation, Holder, UsageClearance, UsageRefusal};

use super::typed_command;

/// Processes listed before the rest are counted.
pub const HOLDER_LIMIT: usize = 10;
/// Evidence lines listed per process before the rest are counted.
pub const EVIDENCE_LIMIT: usize = 5;

/// `refusal` as the error `wt remove` exits with (exit 3).
pub fn usage_refusal(refusal: &UsageRefusal, display_name: &str, name: &str) -> WorktreeError {
    WorktreeError::RefusedToLoseWork(refusal_markup(refusal, display_name, name))
}

/// The Prose markup for `refusal`, one line per `\n`. `display_name` is the
/// checkout's directory name; `name` is what the user typed, for the rerun
/// command.
pub fn refusal_markup(refusal: &UsageRefusal, display_name: &str, name: &str) -> String {
    let rerun = typed_command("wt remove", name).unwrap_or_else(|| "<i>wt remove</i> for this worktree".to_string());
    let UsageRefusal::Unverified(error) = refusal;
    format!(
        "<red><b>Nothing was removed.</b></red> <i>wt</i> could not confirm the checkout of \
        <blue>{}</blue> right before removing it: {}.\n  <dim>Run {rerun} again.</dim>",
        visible(display_name),
        unverified_reason(error)
    )
}

/// The report section naming the other processes using the checkout, for
/// the reader to weigh before answering; `None` when none was observed. It
/// says that they do not stop the removal.
pub fn holders_markup(clearance: &UsageClearance) -> Option<String> {
    let holders = &clearance.holders;
    if holders.is_empty() {
        return None;
    }
    let mut lines = vec![format!(
        "<yellow><b>In use:</b></yellow> {} {} using this worktree now.",
        holders.len(),
        if holders.len() == 1 { "process is" } else { "processes are" }
    )];
    for holder in holders.iter().take(HOLDER_LIMIT) {
        lines.push(String::new());
        lines.extend(holder_lines(holder));
    }
    if holders.len() > HOLDER_LIMIT {
        lines.push(format!("<dim>…and {} more processes</dim>", holders.len() - HOLDER_LIMIT));
    }
    lines.push(String::new());
    lines.push(
        "<dim>They don't stop the removal; a tool watching for changes is often harmless. A program working \
        in the worktree may lose its files. On Windows, a file one of them holds open can stop deletion \
        partway; running the same command again continues it.</dim>"
            .to_string(),
    );
    if let Some(note) = coverage_note(&clearance.coverage) {
        lines.push(format!("<dim>{note}</dim>"));
    }
    Some(lines.join("\n"))
}

fn holder_lines(holder: &Holder) -> Vec<String> {
    let mut heading = format!("<b>PID {}</b>", holder.pid);
    match (&holder.name, &holder.executable) {
        (Some(name), Some(executable)) if name.as_os_str() != executable.as_os_str() => {
            heading.push_str(&format!("  {} <dim>{}</dim>", native(name), native(executable)));
        }
        (Some(name), _) | (None, Some(name)) => heading.push_str(&format!("  {}", native(name))),
        (None, None) => heading.push_str("  <dim>name unavailable</dim>"),
    }
    if holder.identity_uncertain {
        heading.push_str(" <dim>(could not be tied to one process lifetime)</dim>");
    }
    let mut lines = vec![heading];
    for evidence in holder.evidence.iter().take(EVIDENCE_LIMIT) {
        let label = kind_label(evidence.kind);
        match &evidence.location {
            EvidenceLocation::Path(path) => lines.push(format!("  {label}: {}", native(path))),
            EvidenceLocation::Aliases(paths) => {
                lines.push(format!("  {label}, one object with {} names in the checkout:", paths.len()));
                lines.extend(paths.iter().map(|path| format!("    {}", native(path))));
            }
            EvidenceLocation::Unnamed { identity, observed } => {
                let mut line = format!("  {label}: an object in the checkout that no path names now");
                if let Some(observed) = observed {
                    line.push_str(&format!(", opened as {}", native(observed)));
                }
                if let Some(identity) = identity {
                    line.push_str(&format!(" <dim>({})</dim>", identity_label(identity)));
                }
                lines.push(line);
            }
        }
    }
    if holder.evidence.len() > EVIDENCE_LIMIT {
        lines.push(format!("  <dim>…and {} more</dim>", holder.evidence.len() - EVIDENCE_LIMIT));
    }
    lines
}

fn kind_label(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::WatchRegistration => "Watching",
        EvidenceKind::OpenHandle => "Open handle",
        EvidenceKind::WorkingDirectory => "Working directory",
        EvidenceKind::LoadedModule => "Loaded module",
    }
}

fn identity_label(identity: &FileIdentity) -> String {
    match identity {
        FileIdentity::Unix { device, inode } => format!("device {device}, inode {inode}"),
        FileIdentity::Windows { volume_serial, file_id } => format!("volume {volume_serial:#x}, file ID {file_id:#x}"),
    }
}

fn unverified_reason(error: &PathUsageError) -> String {
    match error {
        PathUsageError::MissingTarget { .. } => "it disappeared after it was checked".to_string(),
        PathUsageError::UnsupportedTargetKind { file_type, .. } => {
            format!("its path is now a {} rather than a directory", visible(file_type))
        }
        PathUsageError::RootValidationTimeout { budget, .. } => {
            format!("the check ran out of its {} ms before it could identify the checkout", budget.as_millis())
        }
        PathUsageError::RootIdentity { source, .. } => {
            format!("its identity could not be read ({})", visible(&source.to_string()))
        }
        PathUsageError::InvalidOption { message } => format!("the usage check was misconfigured ({})", visible(message)),
    }
}

/// One sentence on what the usage check could not see, or `None` when it
/// saw everything it looks for.
pub fn coverage_note(coverage: &CoverageSummary) -> Option<String> {
    if coverage.is_complete() {
        return None;
    }
    let mut clauses = Vec::new();
    for (status, phrase) in [
        (CoverageStatus::Unsupported, "cannot be listed on this system"),
        (CoverageStatus::Failed, "could not be read"),
        (CoverageStatus::Partial, "were only partly read"),
        (CoverageStatus::NotAttempted, "were not checked"),
    ] {
        if status == CoverageStatus::Unsupported && coverage.outcome == Outcome::Unsupported {
            continue; // the lead already says so
        }
        let mechanisms: Vec<&str> =
            coverage.gaps.iter().filter(|gap| gap.status == status).map(|gap| mechanism_label(gap.mechanism)).collect();
        if !mechanisms.is_empty() {
            clauses.push(format!("{} {phrase}", join(&mechanisms)));
        }
    }
    if coverage.budget_exhausted {
        clauses.push("the check ran out of time".to_string());
    }
    if coverage.evidence_omitted > 0 {
        clauses.push(format!("{} more usage records were found but not kept", coverage.evidence_omitted));
    }
    let lead = match coverage.outcome {
        Outcome::Usable => "Discovery is partial",
        Outcome::Unavailable => "Discovery could not run",
        Outcome::Unsupported => "Discovery is not supported on this system",
    };
    Some(if clauses.is_empty() { format!("{lead}.") } else { format!("{lead}; {}.", clauses.join("; ")) })
}

fn mechanism_label(mechanism: Mechanism) -> &'static str {
    match mechanism {
        Mechanism::ProcessEnumeration => "the process list",
        Mechanism::TreeIdentity => "the checkout's files",
        Mechanism::OpenHandles => "open handles",
        Mechanism::WorkingDirectories => "working directories",
        Mechanism::Inotify => "inotify watches",
        Mechanism::Fanotify => "fanotify marks",
        Mechanism::Fsevents => "FSEvents subscriptions",
        Mechanism::LoadedModules => "loaded modules",
        Mechanism::DirectoryChangeSubscriptions => "directory-change subscriptions",
        Mechanism::Polling => "polling watchers",
    }
}

/// `a`, `a and b`, `a, b and c`.
fn join(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [only] => (*only).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn native(value: &NativeString) -> String {
    visible(&value.display())
}

/// `text` safe to show: control characters escaped (`\n`, `\u{1b}`), then
/// Prose markup escaped.
pub fn visible(text: &str) -> String {
    let shown: String = text
        .chars()
        .map(|c| if c.is_control() { c.escape_default().to_string() } else { c.to_string() })
        .collect();
    biscuit_terminal::components::prose::Prose::escape_text(&shown)
}
