//! What `wt remove` prints after the confirmed lifecycle
//! ([`worktree::remove::confirmed`]) ran: the one final line of a removal
//! that finished, the plain sentences of one that did not (a retained mount,
//! a permanent failure, a kept branch), and the refusal of one that changed
//! nothing.
//!
//! Every builder returns Prose markup for stderr. Paths, names, and errors
//! come from the filesystem, Git, and other processes, so each is shown
//! through [`visible`]. A normal removal names no hidden directory, retry
//! command, or report; a failure that needs the user names the path it is
//! about, hidden or not.

use std::path::Path;

use worktree::WorktreeError;
use worktree::remove::confirmed::{BranchOutcome, BranchResult, Candidate, CheckoutResult, ExitClass, Finished, Refusal};
use worktree::remove::recovery::compact::UnboundTarget;

use super::report::visible;

/// `text` from Git or the operating system on one line: its lines trimmed,
/// blank and repeated ones dropped, joined with "; ".
pub fn one_line(text: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if !parts.contains(&line) {
            parts.push(line);
        }
    }
    parts.join("; ")
}

/// The error that carries `markup` with the exit code of `class`.
pub fn error_for(class: ExitClass, markup: String) -> WorktreeError {
    match class {
        ExitClass::Refused => WorktreeError::RefusedToLoseWork(markup),
        ExitClass::Blocked => WorktreeError::BlockedByEnvironment(markup),
        ExitClass::Success | ExitClass::Failure => WorktreeError::RemovalFailed(markup),
    }
}

/// A removal that changed nothing, with its exit code (N5).
pub fn refusal_error(refusal: &Refusal) -> WorktreeError {
    error_for(refusal.exit_class(), refusal_markup(refusal))
}

pub fn refusal_markup(refusal: &Refusal) -> String {
    match refusal {
        Refusal::NotSaved(error) => format!(
            "<red><b>Nothing was deleted.</b></red> The approval could not be saved, so the removal did not start: {}.",
            visible(&one_line(&error.to_string()))
        ),
        Refusal::Cancelled => "<red><b>Nothing was deleted.</b></red> The removal was cancelled before it started.".into(),
        other => format!("<red><b>Nothing was removed:</b></red> {}.", visible(&one_line(&other.to_string()))),
    }
}

/// What the run left for the user, beyond what the final line says.
#[derive(Debug, Default)]
pub struct Report {
    /// Printed in order on success and failure alike.
    pub lines: Vec<String>,
    /// One plain sentence each; any makes the run exit 1.
    pub failures: Vec<String>,
}

/// The final report of a removal that got past saving its approval.
/// `lost` is the number of commits the first run's report said deleting an
/// unsafe branch would lose, when it was approved anyway.
pub fn finished_report(finished: &Finished, display_name: &str, lost: Option<usize>) -> Report {
    let mut report = Report::default();
    for warning in &finished.warnings {
        report.lines.push(format!("<yellow>Warning:</yellow> {}", visible(&one_line(warning))));
    }
    let name = visible(display_name);
    let mut deleted_local = None;
    for branch in &finished.branches {
        match (&branch.target, &branch.outcome) {
            (UnboundTarget::Local { name }, BranchOutcome::Deleted) => deleted_local = Some(name.clone()),
            (UnboundTarget::Remote { name, .. }, BranchOutcome::Deleted) => {
                report.lines.push(format!("<green>Deleted</green> <b>origin/{}</b>", visible(name)));
            }
            (target, BranchOutcome::AlreadyAbsent) => {
                report.lines.push(format!("<dim>{} was already gone.</dim>", branch_label(target)));
            }
            (_, BranchOutcome::Kept(_) | BranchOutcome::NotCarriedOver) => report.failures.push(kept_markup(branch)),
        }
    }
    match &finished.checkout {
        CheckoutResult::Removed => {
            let mut line = format!("<green>Removed</green> <b>{name}</b> ({})", entries(finished.entries));
            if let Some(branch) = &deleted_local {
                line.push_str(&format!(" and deleted branch <b>{}</b>", visible(branch)));
                if let Some(lost) = lost.filter(|lost| *lost > 0) {
                    line.push_str(&format!(" <dim>({} lost)</dim>", super::report::commit_count(lost)));
                }
            }
            line.push_str(&late_changes(&finished.late_changes));
            line.push('.');
            report.lines.insert(0, line);
        }
        CheckoutResult::AlreadyRemoved => {
            let mut line = format!("<green>Removed the record of worktree</green> <b>{name}</b> <dim>(its directory was already gone)</dim>");
            if let Some(branch) = &deleted_local {
                line.push_str(&format!(" and deleted branch <b>{}</b>", visible(branch)));
            }
            line.push('.');
            report.lines.insert(0, line);
        }
        pending => report.failures.insert(0, sentence(&pending.to_string())),
    }
    if let Some(original) = &finished.original_kept {
        report.lines.push(format!(
            "New content found at {} after the worktree was moved away was kept.",
            visible(&original.display().to_string())
        ));
    }
    report
}

/// `text` as a sentence: capitalized, with a full stop.
fn sentence(text: &str) -> String {
    let text = visible(text);
    let mut chars = text.chars();
    let mut out: String = chars.next().map(|first| first.to_uppercase().collect()).unwrap_or_default();
    out.push_str(chars.as_str());
    if !out.ends_with('.') {
        out.push('.');
    }
    out
}

fn branch_label(target: &UnboundTarget) -> String {
    match target {
        UnboundTarget::Local { name } => format!("Branch <b>{}</b>", visible(name)),
        UnboundTarget::Remote { remote, name } => format!("<b>{}/{}</b>", visible(remote), visible(name)),
    }
}

/// Why a branch step did not run, and how the user can finish it.
fn kept_markup(branch: &BranchResult) -> String {
    let label = branch_label(&branch.target);
    let finish = match &branch.target {
        UnboundTarget::Local { name } => format!("git branch -D {}", visible(name)),
        UnboundTarget::Remote { remote, name } => format!("git push {} --delete {}", visible(remote), visible(name)),
    };
    match &branch.outcome {
        BranchOutcome::NotCarriedOver => format!(
            "{label} was kept: the earlier removal that approved deleting it did not record enough to delete \
            exactly what was approved.\n  <dim>Delete it with <i>{finish}</i> if you no longer need it.</dim>"
        ),
        BranchOutcome::Kept(reason) => format!(
            "{label} was kept: {}.\n  <dim>Delete it with <i>{finish}</i> if you no longer need it.</dim>",
            visible(one_line(reason).trim_end_matches('.'))
        ),
        BranchOutcome::Deleted | BranchOutcome::AlreadyAbsent => String::new(),
    }
}

/// "441,203 entries", or that the count is unknown.
fn entries(count: Option<u64>) -> String {
    match count {
        None => "unknown number of entries".into(),
        Some(1) => "1 entry".into(),
        Some(count) => format!("{} entries", grouped(count)),
    }
}

fn grouped(count: u64) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// R14: the uncommitted files the question did not show. Never claims to be
/// all that changed.
fn late_changes(files: &[std::path::PathBuf]) -> String {
    if files.is_empty() {
        return String::new();
    }
    const SHOWN: usize = 5;
    let mut names: Vec<String> = files.iter().take(SHOWN).map(|file| visible(&file.display().to_string())).collect();
    if files.len() > SHOWN {
        names.push(format!("and {} more", files.len() - SHOWN));
    }
    let noun = if files.len() == 1 { "file" } else { "files" };
    format!("; also deleted {} {noun} changed after you answered: {}", files.len(), names.join(", "))
}

/// A name that matches a pending removal and another worktree (or several
/// pending removals): nothing is chosen.
pub fn ambiguous_markup(name: &str, candidates: &[Candidate]) -> String {
    let mut out = format!(
        "<red><b>Nothing was removed.</b></red> <b>{}</b> matches more than one worktree or unfinished removal:",
        visible(name)
    );
    for candidate in candidates {
        match candidate {
            Candidate::Live(entry) => {
                out.push_str(&format!("\n  worktree at {}", visible(&entry.path.display().to_string())));
            }
            Candidate::Pending { original, .. } => out.push_str(&format!(
                "\n  an unfinished removal of the worktree that was at {}",
                visible(&original.display().to_string())
            )),
        }
    }
    out.push_str("\n  <dim>Name it by its directory or branch so it matches one.</dim>");
    out
}

/// The name a removal is reported by: its directory's basename.
pub fn name_of(path: &Path) -> String {
    path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_counts_are_grouped_or_unknown() {
        assert_eq!(entries(Some(441_203)), "441,203 entries");
        assert_eq!(entries(Some(1_000)), "1,000 entries");
        assert_eq!(entries(Some(999)), "999 entries");
        assert_eq!(entries(Some(1)), "1 entry");
        assert_eq!(entries(None), "unknown number of entries");
    }

    #[test]
    fn late_changes_name_at_most_five_files() {
        assert_eq!(late_changes(&[]), "");
        assert_eq!(late_changes(&["src/new.rs".into()]), "; also deleted 1 file changed after you answered: src/new.rs");
        let many: Vec<std::path::PathBuf> = (0..7).map(|i| format!("f{i}").into()).collect();
        assert!(late_changes(&many).ends_with("f4, and 2 more"), "{}", late_changes(&many));
    }

    #[test]
    fn gits_multi_line_errors_read_as_one_line() {
        let text = "To ../origin.git\n ! [rejected]  feat/x (stale info)   \nerror: failed to push\nerror: failed to push\n";
        assert_eq!(one_line(text), "To ../origin.git; ! [rejected]  feat/x (stale info); error: failed to push");
    }

    #[test]
    fn a_pending_sentence_reads_as_one_sentence() {
        assert_eq!(sentence("x could not be deleted: the disk is read-only"), "X could not be deleted: the disk is read-only.");
    }
}
