//! `wt remove`: report first, then ask, then remove everything that can be
//! removed without losing work.
//!
//! The worktree needs consent when it has dirty or ignored entries; the local
//! branch is deleted when its safety tier says its commits live elsewhere; the
//! branch on origin is deleted only with `--force-remote`. Removing the
//! worktree the caller stands in is handed to the shell wrapper: the first run
//! asks everything and prints `cd:` plus `remove-handoff:<token>`, and the
//! wrapper's `wt remove --handoff <token>` finishes from outside.
//!
//! A worktree Git can no longer read is prepared first ([`prepare`]): a
//! missing directory has only its record removed, after its surviving index
//! is checked; a directory whose `.git` file is gone has its link repaired and
//! verified, then takes the ordinary path; anything else refuses. The rules
//! are item 3 of `2026-09-24-ux-improvements`.

mod policy;
mod report;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use inquire::{Confirm, InquireError, Select};
use worktree::WorktreeError;
use worktree::availability::{self, Availability, OtherCondition};
use worktree::fork_origin::{ForkOriginStore, fork_origin_path};
use worktree::git::git_command;
use worktree::remove::handoff::{
    self, Approvals, BranchAction, HandoffError, HandoffRecord, HandoffRefusal, HandoffState,
    RemoteApproval, canonical, checkout_git_dir, is_within,
};
use worktree::live_remote::{LIVE_CHECK_DEADLINE, LsRemote};
use worktree::remove::remote::{
    Reinterpretation, RemoteState, delete_remote_branch, preflight_remote_deletion,
    remote_destination,
};
use worktree::remove::safety::{
    BranchSafety, NoPrSource, PrSource, Reconfirmation, SafetyInput, SniffPrSource, Tier, assess,
    reconfirm,
};
use worktree::remove::missing::{MissingCheckout, MissingRefusal, remove_missing_record};
use worktree::remove::repair::{self, RepairRefusal};
use worktree::remove::{
    CheckoutState, Inventory, PrepareRefusal, collect_inventory, prepare, remove_local_branch,
    remove_worktree,
};
use worktree::remove::included::classify_included;
use worktree::worktree::{WorktreeEntry, default_branch, find_worktree, parse_worktree_list};

use super::list_table::{shell_word, unavailable_reason};

pub use policy::Flags;
use policy::{Actions, BranchStep, Decision, Question, Refusal, Situation};

fn esc(text: &str) -> String {
    Prose::escape_text(text)
}

fn print(terminal: &Terminal, markup: impl Into<String>) {
    eprintln!("{}", Prose::new(markup.into()).render(terminal));
}

/// The name a worktree is reported by: its directory's basename.
fn display_name(entry: &WorktreeEntry) -> String {
    entry
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| entry.path.display().to_string())
}

/// `error` with `context` (the target and the operation) before its message.
/// The variant, and so the exit code, is kept; variants that carry their own
/// context or Prose markup are returned unchanged.
fn in_context(error: WorktreeError, context: &str) -> WorktreeError {
    match error {
        WorktreeError::GitCommand(message) => WorktreeError::GitCommand(format!("{context}: {message}")),
        WorktreeError::GitParse(message) => WorktreeError::GitParse(format!("{context}: {message}")),
        WorktreeError::IncludeSetDiscovery(message) => {
            WorktreeError::IncludeSetDiscovery(format!("{context}: {message}"))
        }
        WorktreeError::Io(error) => WorktreeError::Io(std::io::Error::new(error.kind(), format!("{context}: {error}"))),
        other => other,
    }
}

/// Prose markup for `command` followed by `argument`, a command the user can
/// type, or `None` when `argument` can't be spelled the same way in every
/// shell.
fn typed_command(command: &str, argument: &str) -> Option<String> {
    Some(format!("<i>{} {}</i>", esc(command), esc(&shell_word(argument)?)))
}

/// Everything known about the worktree before anything is asked.
struct Facts {
    base: PathBuf,
    entry: WorktreeEntry,
    display_name: String,
    /// The branch's full tip SHA (detached: HEAD).
    head: String,
    /// What [`prepare`] found and did before any check.
    checkout: CheckoutState,
    /// The checkout's files. Empty for [`CheckoutState::Missing`], which had
    /// no directory to inspect; that never means "checked clean".
    inventory: Inventory,
    /// `None` for a detached worktree, and in a handoff's second run, which
    /// never assesses the tiers (see [`run_handoff`]).
    safety: Option<BranchSafety>,
    has_origin: bool,
    /// The live preflight for an attached branch: with `--force-remote` in
    /// the first run, and when remote deletion was approved in the second.
    remote: Option<RemoteState>,
    /// [`fingerprint`] of the files as reported, taken in the first run
    /// before the report when consent may cover them or a handoff will bind
    /// them. Discarding files is approved for this state only.
    reported: Option<String>,
}

impl Facts {
    /// The local facts both runs read: the target's files and include rules
    /// (for a missing directory, what [`prepare`] read from its record
    /// instead), and its tip. No network.
    fn local(base: &Path, entry: WorktreeEntry, checkout: CheckoutState) -> Result<Self, WorktreeError> {
        let display_name = display_name(&entry);
        let (inventory, head) = match &checkout {
            CheckoutState::Missing(missing) => (Inventory::default(), missing.head.clone()),
            CheckoutState::Healthy | CheckoutState::Repaired(_) => {
                let kept = if matches!(checkout, CheckoutState::Repaired(_)) {
                    " (the .git link restored for this check was left in place)"
                } else {
                    ""
                };
                let operation =
                    format!("could not check the files of worktree {display_name} at {}{kept}", entry.path.display());
                let context = |error| in_context(error, &operation);
                let mut inventory = collect_inventory(base, &entry.path).map_err(context)?;
                inventory.included =
                    classify_included(base, &entry.path, entry.branch.as_deref()).map_err(context)?;
                inventory.expand_mixed_ignored(base, &entry.path).map_err(context)?;
                let head = match &entry.branch {
                    Some(branch) => git_command(&["rev-parse", &format!("refs/heads/{branch}")]).map_err(context)?,
                    None => entry
                        .head_sha
                        .clone()
                        .ok_or_else(|| context(WorktreeError::GitParse("worktree has no HEAD".into())))?,
                };
                (inventory, head)
            }
        };
        Ok(Self {
            base: base.to_path_buf(),
            entry,
            display_name,
            head,
            checkout,
            inventory,
            safety: None,
            has_origin: false,
            remote: None,
            reported: None,
        })
    }

    /// The first run's report facts: the branch's tier (PR lookup and live
    /// checks) and, with `--force-remote`, the remote preflight.
    fn assess(&mut self, force_remote: bool) {
        self.has_origin = git_command(&["remote", "get-url", "origin"]).is_ok();
        let Some(branch) = self.entry.branch.clone() else {
            return;
        };
        let base = self.base.as_path();
        let heads = live_heads(base);
        let sniff_prs = SniffPrSource::for_origin(base);
        let prs: &dyn PrSource = match &sniff_prs {
            Some(prs) => prs,
            None => &NoPrSource,
        };
        let context = SafetyContext::read(base, &branch, sniff_prs.as_ref());
        self.safety = Some(assess(&context.input(base, &branch, &self.head, force_remote), prs, &heads));
        self.remote = force_remote.then(|| preflight_remote_deletion(base, &branch, &self.head, &heads));
    }

    fn branch(&self) -> Option<&str> {
        self.entry.branch.as_deref()
    }

    fn missing(&self) -> Option<&MissingCheckout> {
        match &self.checkout {
            CheckoutState::Missing(missing) => Some(missing),
            CheckoutState::Healthy | CheckoutState::Repaired(_) => None,
        }
    }

    /// What a refusal or cancellation adds after a repair: the restored link
    /// is never rolled back.
    fn kept_link_note(&self) -> String {
        match self.checkout {
            CheckoutState::Repaired(_) => format!(
                "\n  <dim>The .git link restored for <blue>{}</blue> was left in place.</dim>",
                esc(&self.display_name)
            ),
            CheckoutState::Healthy | CheckoutState::Missing(_) => String::new(),
        }
    }

    fn situation(&self, interactive: bool) -> Situation {
        Situation {
            needs_consent: match self.missing() {
                Some(missing) => missing.needs_consent(),
                None => self.inventory.needs_consent(),
            },
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
                missing: self.missing(),
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

/// The live-check transport both runs use.
fn live_heads(base: &Path) -> LsRemote<'_> {
    LsRemote {
        base,
        deadline: LIVE_CHECK_DEADLINE,
    }
}

/// The locally read parts of a [`SafetyInput`].
struct SafetyContext {
    default_branch: String,
    remote_branch: Option<String>,
    source_repo: Option<String>,
}

impl SafetyContext {
    fn read(base: &Path, branch: &str, prs: Option<&SniffPrSource>) -> Self {
        Self {
            default_branch: default_branch().unwrap_or_default(),
            remote_branch: remote_destination(base, branch),
            source_repo: prs.and_then(SniffPrSource::source_repo),
        }
    }

    fn input<'a>(&'a self, base: &'a Path, branch: &'a str, tip: &'a str, force_remote: bool) -> SafetyInput<'a> {
        SafetyInput {
            base,
            branch,
            tip,
            default_branch: &self.default_branch,
            remote_branch: self.remote_branch.as_deref(),
            force_remote,
            source_repo: self.source_repo.as_deref(),
        }
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
    let display = display_name(&entry);
    if inside && !crate::env::shell_wrapper_active() {
        return Err(WorktreeError::BlockedByEnvironment(format!(
            "\n<red><b>Nothing was removed.</b></red> You are inside worktree <blue>{}</blue>, and \
            without the shell wrapper <i>wt</i> cannot move your shell out before removing it.\n{}\n\n\
            Or run <i>wt remove</i> from another directory.",
            esc(&display),
            super::go::wrapper_setup_help()
        )));
    }

    // After the guards above, so nothing is repaired for a removal that
    // could not go ahead anyway.
    let checkout = prepare(&base, &entry, &repair::Git)
        .map_err(|refusal| WorktreeError::RefusedToLoseWork(prepare_refusal_markup(&refusal, name, &entry)))?;
    if let CheckoutState::Repaired(_) = checkout {
        print(
            &terminal,
            format!(
                "\n<green>Restored the link</green> for <blue>{}</blue> so its files could be checked.\n  \
                <dim>git worktree repair may also have restored other worktrees' links.</dim>",
                esc(&display)
            ),
        );
    }

    let mut facts = Facts::local(&base, entry, checkout)?;
    if facts.missing().is_none() && (inside || facts.inventory.needs_consent()) {
        facts.reported = Some(fingerprint(&facts)?);
    }
    facts.assess(flags.force_remote);
    eprintln!("{}", facts.render_report(&terminal));
    if let Some(refusal) = unprovable_remote(facts.remote.as_ref()) {
        return Err(WorktreeError::RefusedToLoseWork(refusal + &facts.kept_link_note()));
    }

    let interactive = crate::env::is_interactive();
    let decision = policy::decide(facts.situation(interactive), flags, &mut |question| {
        ask(&terminal, &facts, question)
    });
    let decision = match decision {
        Err(WorktreeError::Cancelled) => Decision::Cancelled,
        other => other?,
    };

    match decision {
        Decision::Refuse(refusal) => Err(WorktreeError::RefusedToLoseWork(
            refusal_markup(refusal, &facts) + &facts.kept_link_note(),
        )),
        Decision::Cancelled => {
            print(
                &terminal,
                format!(
                    "\n<dim>Cancelled. Worktree <blue>{}</blue> was not removed.</dim>{}",
                    esc(&facts.display_name),
                    facts.kept_link_note()
                ),
            );
            Ok(())
        }
        // A missing directory has nowhere to stand in, so needs no move.
        Decision::Proceed(actions) if inside && facts.missing().is_none() => {
            hand_off(&terminal, &facts, &cwd, actions)
        }
        Decision::Proceed(actions) => {
            if actions.discard_files && facts.missing().is_none() {
                files_unchanged(&facts)?;
            }
            execute(&terminal, &facts, actions)
        }
    }
}

/// Refuses (exit 3) unless the files still match the report: the same
/// listed entries and the same [`Facts::reported`] fingerprint.
///
/// `git worktree remove --force` discards whatever the checkout holds when it
/// runs, so consent given to the report (by a question or
/// `--force-worktree`) would otherwise also cover files changed or staged
/// while the question was open.
fn files_unchanged(facts: &Facts) -> Result<(), WorktreeError> {
    let fresh = Facts::local(&facts.base, facts.entry.clone(), facts.checkout.clone())?;
    if fresh.inventory.dirty == facts.inventory.dirty && facts.reported.as_deref() == Some(fingerprint(&fresh)?.as_str()) {
        return Ok(());
    }
    Err(WorktreeError::RefusedToLoseWork(
        start_again("The worktree's files or staged changes changed after they were checked.") + &facts.kept_link_note(),
    ))
}

/// `--force-remote` cannot show that it deletes from exactly the repository
/// reported.
fn unprovable_remote(state: Option<&RemoteState>) -> Option<String> {
    match state? {
        RemoteState::MultiplePushUrls { .. } => Some(
            "\n<red><b>Nothing was removed.</b></red> Origin pushes to more than one repository, \
            so <i>--force-remote</i> cannot delete the branch from exactly the one reported.\n  \
            <dim>Leave out <i>--force-remote</i>, or delete the branch in each repository with \
            <i>git push</i>.</dim>"
                .to_string(),
        ),
        RemoteState::ReinterpretedEndpoint {
            endpoint,
            by: Reinterpretation::Rewrite(rule),
            ..
        } => Some(format!(
            "\n<red><b>Nothing was removed.</b></red> Git would rewrite origin's push URL <b>{}</b> \
            again by <i>{}</i>, so <i>--force-remote</i> cannot delete the branch from exactly the \
            repository reported.\n  \
            <dim>Leave out <i>--force-remote</i>, or change that rule.</dim>",
            esc(endpoint),
            esc(rule)
        )),
        RemoteState::ReinterpretedEndpoint {
            endpoint,
            by: Reinterpretation::RemoteName { source },
            ..
        } => Some(format!(
            "\n<red><b>Nothing was removed.</b></red> Git would read origin's push URL <b>{}</b> \
            as the name of the remote defined by <i>{}</i>, so <i>--force-remote</i> cannot delete \
            the branch from exactly the repository reported.\n  \
            <dim>Leave out <i>--force-remote</i>, or rename that remote.</dim>",
            esc(endpoint),
            esc(source)
        )),
        _ => None,
    }
}

fn refusal_markup(refusal: Refusal, facts: &Facts) -> String {
    match refusal {
        Refusal::FilesNeedForce if facts.missing().is_some() => format!(
            "\n<red><b>Nothing was removed.</b></red> The record of worktree <blue>{}</blue> holds \
            staged changes (listed above), and there is no terminal to confirm discarding them.\n  \
            <dim>Add <i>--force-worktree</i> to discard them.</dim>",
            esc(&facts.display_name)
        ),
        Refusal::FilesNeedForce => format!(
            "\n<red><b>Nothing was removed.</b></red> Worktree <blue>{}</blue> has uncommitted or \
            protected included files (listed above), and there is no terminal to confirm discarding them.\n  \
            <dim>Add <i>--force-worktree</i> to discard them.</dim>",
            esc(&facts.display_name)
        ),
        Refusal::ForceBranchNeedsWorktree => "\n<red><b>Nothing was removed.</b></red> \
            <i>--force-branch</i> needs the worktree removed first, and it has uncommitted files; \
            add <i>--force-worktree</i> to discard them."
            .to_string(),
    }
}

/// Why `wt remove name` stopped before checking anything: the worktree is
/// unavailable (Git can't read it, or its path is a link) and [`prepare`]
/// could not make it checkable. Every case is a
/// refusal (exit 3) that removed nothing; only a repair attempt may have
/// changed Git metadata, and then the message says so.
fn prepare_refusal_markup(refusal: &PrepareRefusal, name: &str, entry: &WorktreeEntry) -> String {
    let label = esc(&display_name(entry));
    let path = entry.path.to_string_lossy();
    let nothing = "<b>No working files, branches, or worktree records were removed.</b>";
    let list = "<i>git worktree list --porcelain</i>";
    let retry = typed_command("wt remove", name).unwrap_or_else(|| "<i>wt remove</i>".to_string());
    match refusal {
        PrepareRefusal::Repair(RepairRefusal::Unverified { failures, attempt }) => {
            let repair = typed_command("git worktree repair", &path)
                .unwrap_or_else(|| format!("<i>git worktree repair</i> naming {}", esc(&path)));
            let mut markup = format!(
                "\n<red><b>Can't remove {label}:</b></red> its .git file was missing and Git couldn't \
                restore a verified link.\n{nothing} The repair attempt may have changed Git metadata \
                for this or other worktrees.\nFrom the base checkout, inspect the repair result with \
                {list} and try {repair}, then retry {retry}."
            );
            // Several checks fail with the same message when Git can't
            // resolve the checkout at all; say it once.
            let mut reasons: Vec<String> = Vec::new();
            for failure in failures.iter().map(ToString::to_string) {
                if !reasons.contains(&failure) {
                    reasons.push(failure);
                }
            }
            for reason in reasons {
                markup.push_str(&format!("\n  <dim>Not verified: {}</dim>", esc(&reason)));
            }
            for line in attempt.diagnostics() {
                markup.push_str(&format!("\n  <dim>Repair output: {}</dim>", esc(&line)));
            }
            markup
        }
        PrepareRefusal::Repair(refusal) => format!(
            "\n<red><b>Can't remove {label}:</b></red> Git can't read this worktree, and no repair was \
            attempted: {}.\n{nothing}\n  <dim>Inspect it from the base checkout with {list}, then retry \
            {retry}.</dim>",
            esc(&refusal.to_string())
        ),
        PrepareRefusal::Missing(refusal) => format!(
            "\n<red><b>Can't remove {label}:</b></red> its directory is gone, and its remaining Git record \
            couldn't be checked: {}.\n{nothing}\n  <dim>Inspect it from the base checkout with {list}, \
            then retry {retry}.</dim>",
            esc(&refusal.to_string())
        ),
        PrepareRefusal::Unavailable(condition) => format!(
            "\n<red><b>Can't remove {label}:</b></red> {}.\n{nothing} \
            <i>wt remove</i> deletes a directory only after checking its files, and no <i>--force</i> \
            flag changes that.\n  <dim>Restore or move what is at {}, then retry {retry}.</dim>",
            unavailable_reason(entry, condition),
            esc(&path)
        ),
    }
}

/// Asks one question after exactly one blank line.
fn ask(terminal: &Terminal, facts: &Facts, question: Question) -> Result<bool, WorktreeError> {
    match question {
        Question::DiscardFiles => {
            eprintln!();
            let included = facts.inventory.included.needs_consent.iter()
                .map(|(path, _)| report::visible_include_path(path)).collect::<Vec<_>>();
            let question = if facts.missing().is_some() {
                format!("Discard the staged changes listed above and remove the record of worktree <blue>{}</blue>?",
                    esc(&facts.display_name))
            } else {
                let names = if included.is_empty() { "the files listed above".to_string() }
                    else { format!("the files listed above, including {}", included.join(", ")) };
                format!("Discard {names} and remove worktree <blue>{}</blue>?", esc(&facts.display_name))
            };
            let label = Prose::new(question).render(terminal);
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

/// Removes the worktree (for a missing directory, only its record), then the
/// branch, then (with `--force-remote`) the branch on origin. Nothing after a
/// failed step runs.
fn execute(terminal: &Terminal, facts: &Facts, actions: Actions) -> Result<(), WorktreeError> {
    let target = format!("worktree {} at {}", facts.display_name, facts.entry.path.display());
    let copy_record_warning = if let Some(missing) = facts.missing() {
        remove_missing_record(&facts.base, &facts.entry, missing).map_err(|refusal| missing_record_error(refusal, facts))?
    } else {
        remove_worktree(&facts.base, &facts.entry.path, actions.discard_files).map_err(|error| match error {
            WorktreeError::NotARealDirectory(_) => WorktreeError::RefusedToLoseWork(format!(
                "\n<red><b>Nothing was removed.</b></red> {} is no longer the directory of worktree <blue>{}</blue> \
                that was checked: a link or something else replaced it.\n  <dim>Put the checkout back at that \
                path, then run <i>wt remove</i> again.</dim>",
                esc(&facts.entry.path.display().to_string()),
                esc(&facts.display_name)
            )),
            other => in_context(other, &format!("could not remove {target}")),
        })?;
        worktree::copy_record::delete_for(&facts.base, &facts.entry.path)
    };
    if let Some(warning) = copy_record_warning {
        print(terminal, format!("<yellow>Warning:</yellow> could not delete copy record: {}", esc(&warning)));
    }
    let mut removed = vec![format!("worktree {}", facts.display_name)];
    let path = esc(&facts.entry.path.display().to_string());
    print(
        terminal,
        if facts.missing().is_some() {
            format!(
                "\n<green>Removed the record of worktree</green> <b>{}</b> <dim>(its directory at {path} was \
                already gone)</dim>",
                esc(&facts.display_name)
            )
        } else {
            format!("\n<green>Removed worktree</green> <b>{}</b> <dim>at {path}</dim>", esc(&facts.display_name))
        },
    );

    if let (Some(branch), Some(step)) = (facts.branch(), actions.branch) {
        match step {
            BranchStep::Delete { .. } => {
                remove_local_branch(&facts.base, branch, &facts.head).map_err(|error| {
                    in_context(error, &format!("removed {}, but could not delete branch {branch}", removed.join(" and ")))
                })?;
                removed.push(format!("branch {branch}"));
                // Without assessed safety (a handoff's second run) there is
                // no count to claim; the first run's report named the commits.
                let lost = match &facts.safety {
                    Some(safety) if !safety.tier.allows_deletion() && facts.lost_commit_count() > 0 => {
                        format!(
                            " <dim>({} lost)</dim>",
                            report::commit_count(facts.lost_commit_count())
                        )
                    }
                    _ => String::new(),
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

/// The error for a missing directory's record that was not removed. No branch
/// step has run.
fn missing_record_error(refusal: MissingRefusal, facts: &Facts) -> WorktreeError {
    let name = esc(&facts.display_name);
    match refusal {
        MissingRefusal::RecordRemoval(error) => in_context(
            error,
            &format!(
                "could not remove the record of worktree {} at {}; no branch was deleted",
                facts.display_name,
                facts.entry.path.display()
            ),
        ),
        MissingRefusal::Reappeared(_) => WorktreeError::RefusedToLoseWork(start_again(&format!(
            "Something now exists at <b>{}</b>, where the directory of worktree <blue>{name}</blue> was \
            gone, and its contents have not been checked.",
            esc(&facts.entry.path.display().to_string())
        ))),
        other => WorktreeError::RefusedToLoseWork(start_again(&format!(
            "The record of worktree <blue>{name}</blue> was not removed: {}.",
            esc(&other.to_string())
        ))),
    }
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
            destination,
            endpoint,
            sha,
            ..
        }) => {
            delete_remote_branch(&facts.base, endpoint, destination, sha)
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
            ..
        }) => Err(failure(destination, reason)),
        // `run` refuses this before removing anything.
        Some(RemoteState::MultiplePushUrls { destination, .. }) => Err(failure(
            destination,
            "origin pushes to more than one repository",
        )),
        Some(RemoteState::ReinterpretedEndpoint {
            destination,
            endpoint,
            by,
        }) => Err(failure(
            destination,
            &format!("origin's push URL {endpoint}: {by}"),
        )),
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
    let git_dir = checkout_git_dir(&facts.base, &facts.entry.path).map_err(|error| {
        in_context(error, &format!("could not read the .git link of worktree {}", facts.display_name))
    })?;
    let state = HandoffState {
        repo: canonical(&facts.base),
        target: canonical(&facts.entry.path),
        head: facts.head.clone(),
        branch: facts.entry.branch.clone(),
        git_dir,
        fingerprint: facts.reported.clone().ok_or_else(|| {
            WorktreeError::GitParse("the reported files were not fingerprinted before the handoff".into())
        })?,
        rules: facts.inventory.included.rules.clone(),
        baseline: facts.inventory.included.baseline.clone(),
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
            destination,
            endpoint,
            sha,
            ..
        }) => RemoteApproval {
            destination: Some(destination.clone()),
            endpoint: Some(endpoint.clone()),
            observed_sha: Some(sha.clone()),
        },
        Some(
            RemoteState::Absent {
                destination,
                endpoint,
            }
            | RemoteState::Unavailable {
                destination,
                endpoint,
                ..
            },
        ) => RemoteApproval {
            destination: Some(destination.clone()),
            endpoint: Some(endpoint.clone()),
            observed_sha: None,
        },
        Some(RemoteState::MultiplePushUrls { destination, .. }) => RemoteApproval {
            destination: Some(destination.clone()),
            endpoint: None,
            observed_sha: None,
        },
        Some(RemoteState::ReinterpretedEndpoint {
            destination,
            endpoint,
            ..
        }) => RemoteApproval {
            destination: Some(destination.clone()),
            endpoint: Some(endpoint.clone()),
            observed_sha: None,
        },
        Some(RemoteState::NoRemote) | None => RemoteApproval {
            destination: None,
            endpoint: None,
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

/// [`Inventory::fingerprint`] of the target, refusing (exit 4) when part of
/// what removal would delete cannot be read: an approval that does not cover
/// it could not prove it unchanged.
fn fingerprint(facts: &Facts) -> Result<String, WorktreeError> {
    facts
        .inventory
        .fingerprint(&facts.base, &facts.entry.path)
        .map_err(|error| match error {
            WorktreeError::Io(error) => WorktreeError::BlockedByEnvironment(start_again(&format!(
                "Could not read everything the removal would delete: {}. \
                Make it readable, or run <i>wt remove</i> from outside the worktree.",
                esc(&error.to_string())
            ))),
            other => other,
        })
}

fn start_again(reason: &str) -> String {
    format!(
        "\n<red><b>Nothing was removed.</b></red> {reason}\n  \
        <dim>Run <i>wt remove</i> again to start over.</dim>"
    )
}

/// [`reconfirm`] for the second run: local proof first, then a PR or a live
/// `origin/*` head. With remote deletion approved, the destination and its
/// open PR are about to go, so neither counts.
fn reconfirm_branch(base: &Path, branch: &str, tip: &str, force_remote: bool) -> Reconfirmation {
    let sniff_prs = SniffPrSource::for_origin(base);
    let prs: &dyn PrSource = match &sniff_prs {
        Some(prs) => prs,
        None => &NoPrSource,
    };
    let context = SafetyContext::read(base, branch, sniff_prs.as_ref());
    reconfirm(&context.input(base, branch, tip, force_remote), prs, &live_heads(base))
}

/// The refusal for a branch whose safety no longer holds, with the evidence
/// that was tried. The first run's report is not repeated: its facts were not
/// gathered again.
fn no_longer_safe(reconfirmation: &Reconfirmation) -> String {
    let mut reason = "The branch is no longer safe to delete.".to_string();
    if let Tier::Unknown(why) = &reconfirmation.tier {
        reason.push_str(&format!(" Where its commits live could not be checked ({}).", esc(why)));
    }
    for note in &reconfirmation.notes {
        reason.push_str(&format!("\n  <dim>{}</dim>", esc(note)));
    }
    start_again(&reason)
}

/// Why the second run must not delete on origin, or `None` when the fresh
/// preflight matches the approval.
///
/// The destination and endpoint are recomputed from configuration, and two
/// branches or repositories can share a head, so the SHA alone cannot tell
/// that `execute` would delete a branch the caller never saw. The live head
/// must be a verified answer equal to the approved one: an approved head
/// that is now absent, a head that appeared, and an unreachable origin all
/// refuse. The lease stays the approved SHA, which the match makes equal to
/// the fresh one.
fn remote_changed(approval: &RemoteApproval, state: Option<&RemoteState>) -> Option<String> {
    let current = remote_approval(state);
    if current.destination != approval.destination {
        return Some(start_again("The branch on origin to delete changed since you confirmed."));
    }
    if current.endpoint != approval.endpoint {
        return Some(start_again("The repository origin pushes to changed since you confirmed."));
    }
    // A rewrite rule or a remote named like the endpoint, added since the
    // first run, leaves the endpoint's spelling unchanged but would redirect
    // the deletion.
    if let Some(refusal) = unprovable_remote(state) {
        return Some(refusal);
    }
    match (state, approval.observed_sha.as_deref()) {
        (Some(RemoteState::Present { sha, .. }), Some(approved)) if sha == approved => None,
        (Some(RemoteState::Absent { .. }), None) => None,
        // No origin (or a detached worktree) in both runs: nothing to delete.
        (Some(RemoteState::NoRemote) | None, None) => None,
        (Some(RemoteState::Unavailable { reason, .. }), _) => Some(start_again(&format!(
            "Origin could not be reached, so the branch on origin cannot be shown unchanged \
            since you confirmed.\n  <dim>{}</dim>",
            esc(reason.trim())
        ))),
        _ => Some(start_again("The branch on origin changed since you confirmed.")),
    }
}

/// `wt remove --handoff <token>`, run by the wrapper after its `cd`.
///
/// Every local fact the approval covered is read again and compared before
/// anything is removed. The network is asked only for what the approved
/// actions still depend on: a branch deleted because it was safe is proved
/// again by [`reconfirm_branch`], and an approved remote deletion needs the
/// live head. Keeping the branch or deleting it by explicit approval needs
/// neither.
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

    // The second run never repairs: a link that broke since the first run
    // refuses, and one redirected elsewhere fails the `git_dir` comparison.
    // A checkout replaced by a link passes every comparison below, because
    // both sides resolve through the new link, so its kind is checked here.
    match availability::classify(&entry) {
        Availability::Healthy => {}
        Availability::Other(OtherCondition::Link) => {
            return Err(WorktreeError::RefusedToLoseWork(start_again(
                "The worktree's path is now a link, which may have replaced the checkout you confirmed.",
            )));
        }
        _ => {
            return Err(WorktreeError::RefusedToLoseWork(start_again(
                "Git can no longer read the worktree: its .git link broke since you confirmed. \
                Nothing was repaired.",
            )));
        }
    }
    let git_dir = checkout_git_dir(&base, &entry.path).map_err(|error| {
        WorktreeError::RefusedToLoseWork(start_again(&format!(
            "The worktree's .git link can no longer be read: {}.",
            esc(&error.to_string())
        )))
    })?;

    let remote_approval = record.approvals.remote.clone();
    let mut facts = Facts::local(&base, entry, CheckoutState::Healthy)?;
    let fresh = HandoffState {
        repo: canonical(&base),
        target: canonical(&facts.entry.path),
        head: facts.head.clone(),
        branch: facts.entry.branch.clone(),
        git_dir,
        fingerprint: fingerprint(&facts)?,
        rules: facts.inventory.included.rules.clone(),
        baseline: facts.inventory.included.baseline.clone(),
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

    // Only a branch approved because it was safe needs its safety proved
    // again; a kept or explicitly approved branch needs no PR or live answer.
    if record.approvals.branch == Some(BranchAction::DeleteIfSafe)
        && let Some(branch) = facts.branch()
    {
        let reconfirmation = reconfirm_branch(&base, branch, &facts.head, remote_approval.is_some());
        if !reconfirmation.tier.allows_deletion() {
            return Err(WorktreeError::RefusedToLoseWork(no_longer_safe(&reconfirmation)));
        }
    }
    if let Some(approval) = &remote_approval {
        if let Some(branch) = facts.branch() {
            facts.remote = Some(preflight_remote_deletion(&base, branch, &facts.head, &live_heads(&base)));
        }
        if let Some(refusal) = remote_changed(approval, facts.remote.as_ref()) {
            return Err(WorktreeError::RefusedToLoseWork(refusal));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exit::exit_code;

    #[test]
    fn context_never_changes_an_exit_code() {
        let errors = [
            WorktreeError::GitCommand("fatal: not a git repository".into()),
            WorktreeError::GitParse("worktree has no HEAD".into()),
            WorktreeError::IncludeSetDiscovery("git failed".into()),
            WorktreeError::Io(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
            WorktreeError::RefusedToLoseWork("markup".into()),
            WorktreeError::BlockedByEnvironment("markup".into()),
            WorktreeError::DirectoryInUse("/wt/feat-x".into()),
            WorktreeError::Cancelled,
        ];
        for error in errors {
            let before = (exit_code(&error), error.to_string());
            let wrapped = in_context(error, "could not check the files of worktree feat-x at /wt/feat-x");
            assert_eq!(exit_code(&wrapped), before.0, "{wrapped:?}");
            let message = wrapped.to_string();
            match &wrapped {
                WorktreeError::GitCommand(_) | WorktreeError::GitParse(_) | WorktreeError::IncludeSetDiscovery(_) => {
                    assert!(message.contains("worktree feat-x at /wt/feat-x: "), "{message}");
                }
                WorktreeError::Io(io) => {
                    assert!(message.contains("worktree feat-x"), "{message}");
                    assert_eq!(io.kind(), std::io::ErrorKind::PermissionDenied);
                }
                // Markup and self-describing errors are left alone.
                _ => assert_eq!(message, before.1),
            }
        }
    }
}
