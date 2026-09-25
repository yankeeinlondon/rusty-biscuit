//! `wt remove`: report first, then ask, then remove everything that can be
//! removed without losing work.
//!
//! The worktree needs consent when it has dirty or ignored entries; the local
//! branch is deleted when its safety tier says its commits live elsewhere; the
//! branch on origin is deleted only with `--force-remote`. Removing the
//! worktree the caller stands in is handed to the shell wrapper: the first run
//! asks everything and prints `cd:` plus `remove-handoff:<token>`, and the
//! wrapper's `wt remove --handoff <token>` finishes from outside. The rules are
//! item 3 of `2026-09-24-ux-improvements`.

mod policy;
mod report;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use inquire::{Confirm, InquireError, Select};
use worktree::WorktreeError;
use worktree::fork_origin::{ForkOriginStore, fork_origin_path};
use worktree::git::git_command;
use worktree::remove::handoff::{
    self, Approvals, BranchAction, HandoffError, HandoffRecord, HandoffRefusal, HandoffState,
    RemoteApproval, canonical, is_within,
};
use worktree::remove::live_remote::{LIVE_CHECK_DEADLINE, LsRemote};
use worktree::remove::remote::{
    RemoteState, delete_remote_branch, preflight_remote_deletion, remote_destination,
};
use worktree::remove::safety::{
    BranchSafety, NoPrSource, PrSource, SafetyInput, SniffPrSource, assess,
};
use worktree::remove::{Inventory, collect_inventory, remove_local_branch, remove_worktree};
use worktree::worktree::{WorktreeEntry, default_branch, find_worktree, parse_worktree_list};

pub use policy::Flags;
use policy::{Actions, BranchStep, Decision, Question, Refusal, Situation};

fn esc(text: &str) -> String {
    Prose::escape_text(text)
}

fn print(terminal: &Terminal, markup: impl Into<String>) {
    eprintln!("{}", Prose::new(markup.into()).render(terminal));
}

/// Everything known about the worktree before anything is asked.
struct Facts {
    base: PathBuf,
    entry: WorktreeEntry,
    display_name: String,
    /// The branch's full tip SHA (detached: HEAD).
    head: String,
    inventory: Inventory,
    /// `None` for a detached worktree.
    safety: Option<BranchSafety>,
    has_origin: bool,
    /// Gathered with `--force-remote` for an attached branch.
    remote: Option<RemoteState>,
}

impl Facts {
    fn gather(base: &Path, entry: WorktreeEntry, force_remote: bool) -> Result<Self, WorktreeError> {
        let inventory = collect_inventory(base, &entry.path)?;
        let head = match &entry.branch {
            Some(branch) => git_command(&["rev-parse", &format!("refs/heads/{branch}")])?,
            None => entry
                .head_sha
                .clone()
                .ok_or_else(|| WorktreeError::GitParse("worktree has no HEAD".into()))?,
        };
        let has_origin = git_command(&["remote", "get-url", "origin"]).is_ok();
        let heads = LsRemote {
            base,
            deadline: LIVE_CHECK_DEADLINE,
        };
        let (safety, remote) = match &entry.branch {
            None => (None, None),
            Some(branch) => {
                let sniff_prs = SniffPrSource::for_origin(base);
                let source_repo = sniff_prs.as_ref().and_then(SniffPrSource::source_repo);
                let prs: &dyn PrSource = match &sniff_prs {
                    Some(prs) => prs,
                    None => &NoPrSource,
                };
                let default_branch = default_branch().unwrap_or_default();
                let remote_branch = remote_destination(base, branch);
                let input = SafetyInput {
                    base,
                    branch,
                    tip: &head,
                    default_branch: &default_branch,
                    remote_branch: remote_branch.as_deref(),
                    force_remote,
                    source_repo: source_repo.as_deref(),
                };
                let safety = assess(&input, prs, &heads);
                let remote =
                    force_remote.then(|| preflight_remote_deletion(base, branch, &head, &heads));
                (Some(safety), remote)
            }
        };
        let display_name = entry
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| entry.path.display().to_string());
        Ok(Self {
            base: base.to_path_buf(),
            entry,
            display_name,
            head,
            inventory,
            safety,
            has_origin,
            remote,
        })
    }

    fn branch(&self) -> Option<&str> {
        self.entry.branch.as_deref()
    }

    fn situation(&self, interactive: bool) -> Situation {
        Situation {
            needs_consent: self.inventory.needs_consent(),
            branch_safe: self.safety.as_ref().map(|s| s.tier.allows_deletion()),
            interactive,
        }
    }

    fn render_report(&self, terminal: &Terminal) -> String {
        report::render(
            terminal,
            &report::ReportInput {
                display_name: &self.display_name,
                path: &self.entry.path,
                branch: self.branch(),
                inventory: &self.inventory,
                safety: self.safety.as_ref(),
                has_origin: self.has_origin,
                remote: self.remote.as_ref(),
            },
        )
    }

    fn lost_commit_count(&self) -> usize {
        self.safety
            .as_ref()
            .and_then(|s| s.lost_commits.as_ref().ok())
            .map_or(0, Vec::len)
    }
}

/// `wt remove <name>`.
pub fn run(name: &str, flags: Flags) -> Result<(), WorktreeError> {
    let terminal = Terminal::default();
    let cwd = std::env::current_dir()?;
    let entry = find_worktree(name)?;
    if entry.is_main {
        return Err(WorktreeError::GitCommand(format!(
            "refusing to remove the main checkout '{name}'. Use plain git for that."
        )));
    }
    let base = find_worktree("base")?.path;
    // From here on nothing (not `wt`, not a git child) holds the worktree as
    // its current directory; on Windows that would block removing it.
    std::env::set_current_dir(&base)?;

    let inside = is_within(&cwd, &entry.path);
    let display = entry
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if inside && !crate::env::shell_wrapper_active() {
        return Err(WorktreeError::BlockedByEnvironment(format!(
            "\n<red><b>Nothing was removed.</b></red> You are inside worktree <blue>{}</blue>, and \
            without the shell wrapper <i>wt</i> cannot move your shell out before removing it.\n{}\n\n\
            Or run <i>wt remove</i> from another directory.",
            esc(&display),
            super::go::wrapper_setup_help()
        )));
    }

    let facts = Facts::gather(&base, entry, flags.force_remote)?;
    eprintln!("{}", facts.render_report(&terminal));

    let interactive = crate::env::is_interactive();
    let decision = policy::decide(facts.situation(interactive), flags, &mut |question| {
        ask(&terminal, &facts, question)
    });
    let decision = match decision {
        Err(WorktreeError::Cancelled) => Decision::Cancelled,
        other => other?,
    };

    match decision {
        Decision::Refuse(refusal) => Err(WorktreeError::RefusedToLoseWork(refusal_markup(
            refusal,
            &facts.display_name,
        ))),
        Decision::Cancelled => {
            print(
                &terminal,
                format!(
                    "\n<dim>Cancelled. Worktree <blue>{}</blue> was not removed.</dim>",
                    esc(&facts.display_name)
                ),
            );
            Ok(())
        }
        Decision::Proceed(actions) if inside => hand_off(&terminal, &facts, &cwd, actions),
        Decision::Proceed(actions) => execute(&terminal, &facts, actions),
    }
}

fn refusal_markup(refusal: Refusal, display: &str) -> String {
    match refusal {
        Refusal::FilesNeedForce => format!(
            "\n<red><b>Nothing was removed.</b></red> Worktree <blue>{}</blue> has uncommitted or \
            ignored files (listed above), and there is no terminal to confirm discarding them.\n  \
            <dim>Add <i>--force-worktree</i> to discard them.</dim>",
            esc(display)
        ),
        Refusal::ForceBranchNeedsWorktree => "\n<red><b>Nothing was removed.</b></red> \
            <i>--force-branch</i> needs the worktree removed first, and it has uncommitted files; \
            add <i>--force-worktree</i> to discard them."
            .to_string(),
    }
}

/// Asks one question after exactly one blank line.
fn ask(terminal: &Terminal, facts: &Facts, question: Question) -> Result<bool, WorktreeError> {
    match question {
        Question::DiscardFiles => {
            eprintln!();
            let label = Prose::new(format!(
                "Discard the files listed above and remove worktree <blue>{}</blue>?",
                esc(&facts.display_name)
            ))
            .render(terminal);
            Confirm::new(&label)
                .with_default(false)
                .prompt()
                .map_err(map_inquire_err)
        }
        Question::DeleteUnsafeBranch => {
            let commits = facts
                .safety
                .as_ref()
                .and_then(|s| s.lost_commits.as_ref().ok())
                .cloned()
                .unwrap_or_default();
            eprintln!();
            print(terminal, report::lost_commits_markup(&commits));
            eprintln!();
            let branch = facts.branch().unwrap_or_default();
            let lose = format!(
                "Delete it and lose {} commit{}",
                commits.len(),
                if commits.len() == 1 { "" } else { "s" }
            );
            let label = Prose::new(format!(
                "Branch <blue>{}</blue> is not safe to delete:",
                esc(branch)
            ))
            .render(terminal);
            let choice = Select::new(&label, vec!["Keep the branch".to_string(), lose])
                .with_starting_cursor(0)
                .raw_prompt()
                .map_err(map_inquire_err)?;
            Ok(choice.index == 1)
        }
    }
}

fn map_inquire_err(e: InquireError) -> WorktreeError {
    match e {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            WorktreeError::Cancelled
        }
        e => WorktreeError::Io(std::io::Error::other(e.to_string())),
    }
}

/// Removes the worktree, then the branch, then (with `--force-remote`) the
/// branch on origin.
fn execute(terminal: &Terminal, facts: &Facts, actions: Actions) -> Result<(), WorktreeError> {
    remove_worktree(&facts.base, &facts.entry.path, actions.discard_files)?;
    let mut removed = vec![format!("worktree {}", facts.display_name)];
    print(
        terminal,
        format!(
            "\n<green>Removed worktree</green> <b>{}</b> <dim>at {}</dim>",
            esc(&facts.display_name),
            esc(&facts.entry.path.display().to_string())
        ),
    );

    if let (Some(branch), Some(step)) = (facts.branch(), actions.branch) {
        match step {
            BranchStep::Delete { .. } => {
                remove_local_branch(&facts.base, branch)?;
                removed.push(format!("branch {branch}"));
                let unsafe_deleted = !facts
                    .safety
                    .as_ref()
                    .is_some_and(|s| s.tier.allows_deletion());
                let lost = if unsafe_deleted && facts.lost_commit_count() > 0 {
                    format!(
                        " <dim>({} lost)</dim>",
                        report::commit_count(facts.lost_commit_count())
                    )
                } else {
                    String::new()
                };
                print(
                    terminal,
                    format!("<green>Deleted branch</green> <b>{}</b>{lost}", esc(branch)),
                );
            }
            BranchStep::KeepWithWarning => print(
                terminal,
                format!(
                    "<yellow><b>Warning:</b></yellow> kept branch <b>{b}</b>: its commits exist \
                    nowhere else.\n  <dim>Delete it with <i>git branch -D {b}</i> if you do not \
                    need them.</dim>",
                    b = esc(branch)
                ),
            ),
            BranchStep::Keep => print(
                terminal,
                format!("<dim>Kept branch <b>{}</b>.</dim>", esc(branch)),
            ),
        }
    }

    if actions.delete_remote {
        delete_on_origin(terminal, facts, &removed)?;
    }
    Ok(())
}

fn delete_on_origin(terminal: &Terminal, facts: &Facts, removed: &[String]) -> Result<(), WorktreeError> {
    let failure = |destination: &str, reason: &str| {
        WorktreeError::GitCommand(format!(
            "removed {}, but origin/{destination} was not deleted: {reason}\n\
            Finish with: git push origin --delete {destination}",
            removed.join(" and ")
        ))
    };
    match &facts.remote {
        Some(RemoteState::Present {
            destination, sha, ..
        }) => {
            delete_remote_branch(&facts.base, destination, sha)
                .map_err(|reason| failure(destination, &reason))?;
            print(
                terminal,
                format!("<green>Deleted</green> <b>origin/{}</b>", esc(destination)),
            );
            Ok(())
        }
        Some(RemoteState::Unavailable {
            destination,
            reason,
        }) => Err(failure(destination, reason)),
        Some(RemoteState::Absent { .. }) | Some(RemoteState::NoRemote) | None => {
            print(terminal, "<dim>Nothing to delete on origin.</dim>");
            Ok(())
        }
    }
}

/// The directory the caller lands in: the fork parent's worktree when the
/// branch has a fork-origin record and that parent has a worktree; otherwise
/// the base repo. Returns the directory and how to describe it.
fn landing_root(facts: &Facts, entries: &[WorktreeEntry]) -> (PathBuf, String) {
    let parent = facts.branch().and_then(|branch| {
        let store = ForkOriginStore::load_from(&fork_origin_path(&facts.base).ok()?);
        store.get(branch).map(|origin| origin.base_branch.clone())
    });
    if let Some(parent) = parent
        && let Some(worktree) = entries.iter().find(|e| {
            e.branch.as_deref() == Some(parent.as_str()) && canonical(&e.path) != canonical(&facts.entry.path)
        })
    {
        let description = if worktree.is_main {
            format!("the base repo (on <b>{}</b>, the branch this one was forked from)", esc(&parent))
        } else {
            format!(
                "the <blue>{}</blue> worktree (the branch this one was forked from)",
                esc(&parent)
            )
        };
        return (worktree.path.clone(), description);
    }
    (facts.base.clone(), "the base repo".to_string())
}

fn worktree_entries() -> Result<Vec<WorktreeEntry>, WorktreeError> {
    Ok(parse_worktree_list(&git_command(&["worktree", "list", "--porcelain"])?))
}

/// First run of a move-first removal: record the state and approvals, then
/// tell the wrapper where to move and which token finishes the job.
fn hand_off(terminal: &Terminal, facts: &Facts, cwd: &Path, actions: Actions) -> Result<(), WorktreeError> {
    let entries = worktree_entries()?;
    let (root, description) = landing_root(facts, &entries);
    // Keep the caller's subdirectory when it exists there too (`wt go`'s rule).
    let relative = canonical(cwd)
        .strip_prefix(canonical(&facts.entry.path))
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let landing = if root.join(&relative).is_dir() {
        root.join(&relative)
    } else {
        root
    };

    let approvals = Approvals {
        discard_files: actions.discard_files,
        branch: actions.branch.map(|step| match step {
            BranchStep::Delete { approved: true } => BranchAction::Delete,
            BranchStep::Delete { approved: false } => BranchAction::DeleteIfSafe,
            BranchStep::KeepWithWarning | BranchStep::Keep => BranchAction::Keep,
        }),
        remote: actions.delete_remote.then(|| remote_approval(facts.remote.as_ref())),
    };
    let state = HandoffState {
        repo: canonical(&facts.base),
        target: canonical(&facts.entry.path),
        head: facts.head.clone(),
        branch: facts.entry.branch.clone(),
        fingerprint: facts.inventory.fingerprint(&facts.entry.path),
        landing: canonical(&landing),
    };
    let token = handoff::new_token()?;
    let path = handoff::handoff_path(&facts.base, &token).map_err(|_| {
        WorktreeError::Io(std::io::Error::other("user cache directory unavailable"))
    })?;
    handoff::write_record(&path, &HandoffRecord::new(state, approvals, now()))?;

    print(
        terminal,
        format!(
            "\nMoving you to {description} <dim>at {}</dim> to finish removing the worktree.",
            esc(&landing.display().to_string())
        ),
    );
    println!("cd:{}", landing.display());
    println!("remove-handoff:{token}");
    Ok(())
}

fn remote_approval(state: Option<&RemoteState>) -> RemoteApproval {
    match state {
        Some(RemoteState::Present {
            destination, sha, ..
        }) => RemoteApproval {
            destination: Some(destination.clone()),
            observed_sha: Some(sha.clone()),
        },
        Some(RemoteState::Absent { destination } | RemoteState::Unavailable { destination, .. }) => {
            RemoteApproval {
                destination: Some(destination.clone()),
                observed_sha: None,
            }
        }
        Some(RemoteState::NoRemote) | None => RemoteApproval {
            destination: None,
            observed_sha: None,
        },
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

fn start_again(reason: &str) -> String {
    format!(
        "\n<red><b>Nothing was removed.</b></red> {reason}\n  \
        <dim>Run <i>wt remove</i> again to start over.</dim>"
    )
}

/// `wt remove --handoff <token>`, run by the wrapper after its `cd`.
pub fn run_handoff(token: &str) -> Result<(), WorktreeError> {
    let terminal = Terminal::default();
    let cwd = std::env::current_dir()?;
    let base = find_worktree("base")?.path;

    let record = handoff::handoff_path(&base, token)
        .and_then(|path| handoff::consume(&path, now()))
        .map_err(|error| {
            WorktreeError::BlockedByEnvironment(start_again(match error {
                HandoffError::Missing => "There is no pending removal for this handoff token.",
                HandoffError::Expired => "The removal handoff expired before the shell moved.",
            }))
        })?;

    let entry = worktree_entries()?
        .into_iter()
        .find(|e| canonical(&e.path) == canonical(&record.state.target))
        .ok_or_else(|| {
            WorktreeError::RefusedToLoseWork(start_again(
                "The worktree is no longer registered where it was.",
            ))
        })?;
    std::env::set_current_dir(&base)?;

    let remote_approval = record.approvals.remote.clone();
    let facts = Facts::gather(&base, entry, remote_approval.is_some())?;
    let fresh = HandoffState {
        repo: canonical(&base),
        target: canonical(&facts.entry.path),
        head: facts.head.clone(),
        branch: facts.entry.branch.clone(),
        fingerprint: facts.inventory.fingerprint(&facts.entry.path),
        landing: canonical(&cwd),
    };
    match handoff::verify(&record, &fresh, &cwd) {
        Ok(()) => {}
        Err(HandoffRefusal::InsideTarget) => {
            return Err(WorktreeError::BlockedByEnvironment(start_again(
                "Your shell is still inside the worktree.",
            )));
        }
        Err(HandoffRefusal::Changed(fields)) => {
            return Err(WorktreeError::RefusedToLoseWork(start_again(&format!(
                "These changed since you confirmed: {}.",
                fields.join(", ")
            ))));
        }
    }

    // Re-check the tiers without prompts: a branch approved only because it
    // was safe must still be safe.
    if record.approvals.branch == Some(BranchAction::DeleteIfSafe)
        && !facts.safety.as_ref().is_some_and(|s| s.tier.allows_deletion())
    {
        eprintln!("{}", facts.render_report(&terminal));
        return Err(WorktreeError::RefusedToLoseWork(start_again(
            "The branch is no longer safe to delete.",
        )));
    }
    if let Some(approval) = &remote_approval {
        let now_sha = match &facts.remote {
            Some(RemoteState::Present { sha, .. }) => Some(sha.clone()),
            _ => None,
        };
        if now_sha.is_some() && now_sha != approval.observed_sha {
            eprintln!("{}", facts.render_report(&terminal));
            return Err(WorktreeError::RefusedToLoseWork(start_again(
                "The branch on origin changed since you confirmed.",
            )));
        }
    }

    let actions = Actions {
        discard_files: record.approvals.discard_files,
        branch: record.approvals.branch.map(|action| match action {
            BranchAction::Delete => BranchStep::Delete { approved: true },
            BranchAction::DeleteIfSafe => BranchStep::Delete { approved: false },
            BranchAction::Keep => BranchStep::Keep,
        }),
        delete_remote: remote_approval.is_some(),
    };
    execute(&terminal, &facts, actions)?;
    print(
        &terminal,
        format!("<dim>You are now in {}.</dim>", esc(&cwd.display().to_string())),
    );
    Ok(())
}
