//! `wt remove`: show what removing the worktree would lose, ask once, then
//! delete it.
//!
//! [`facts::gather`] finds everything risky in one pass (a lock, programs
//! using the checkout, uncommitted files, links, mounts, submodule work) and
//! the report shows each as a warning ([`report`]). An interactive caller is
//! asked one question about the checkout unless `--force` answered it; a
//! caller nobody can ask is refused (exit 3) when there is anything to warn
//! about, and the checkout is removed when there is not ([`policy`]). After
//! "yes" the library's removal lifecycle ([`lifecycle::remove`]) deletes
//! the checkout; nothing about its content stops it. The branch's safety
//! tiers and questions are unchanged, and `--force` never implies
//! `--force-branch` or `--force-remote`.
//!
//! A removal an earlier run left unfinished is found by the name the user
//! typed ([`lifecycle::resolve`]) and finished with its saved approval,
//! asking nothing ([`lifecycle::finish_pending`]). Removing the worktree the
//! caller stands in is handed to the shell wrapper: the first run asks, saves
//! the approval, and prints `cd:` plus `remove-handoff:<token>`, and the
//! wrapper's `wt remove --handoff <token>` finishes that operation from
//! outside ([`run_handoff`]). The rules are
//! `2026-10-22-confirmed-remove-always-deletes`.

mod interrupt;
pub mod outcome;
mod policy;
pub mod report;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use biscuit_terminal::components::prose::{InlineProse, LineBreaks, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use inquire::{Confirm, InquireError, Select};
use worktree::{WorktreeCandidate, WorktreeError};
use worktree::fork_origin::{ForkOriginStore, fork_origin_path};
use worktree::live_remote::{LIVE_CHECK_DEADLINE, LsRemote};
use worktree::remove::clock::HostClock;
use worktree::remove::lifecycle::{
    self, ApprovedBranch, Candidate, Env, ExitClass, Notify, Removal, Request, Resolved, ResolveError, Revalidate,
};
use worktree::remove::delete::Wait;
use worktree::remove::facts::{self, Caller, Host, Sources, Target};
use worktree::remove::faults::NoFaults;
use worktree::remove::handoff::{self, HandoffError, OperationHandoff, canonical, is_within};
use worktree::remove::mounts::NativeMounts;
use worktree::remove::recovery::compact::{BranchAction, StepApproval};
use worktree::remove::remote::{Reinterpretation, RemoteState, origin_url, preflight_remote_deletion, remote_destination};
use worktree::remove::repair;
use worktree::remove::safety::{
    BranchSafety, NoPrSource, PrSource, Reconfirmation, SafetyInput, SniffPrSource, Tier, assess, reconfirm,
};
use worktree::remove::usage::SniffUsage;
use worktree::remove::branch_tip;
use worktree::worktree::{WorktreeEntry, default_branch, parse_worktree_list, resolve_worktree};

pub use policy::Flags;
use policy::{Actions, BranchStep, Decision, Question, Situation};
use report::visible;

/// Escape text for Prose markup, leaving the code spans it marks with
/// backticks literal.
fn esc(text: &str) -> String {
    Prose::escape_text_outside_code_spans(text)
}

/// Prints one message to stderr; each `\n` in `markup` starts a new line.
fn print(terminal: &Terminal, markup: impl Into<String>) {
    let prose = Prose::new(markup.into()).with_line_breaks(LineBreaks::Hard);
    eprintln!("{}", prose.render(terminal));
}

/// The name a worktree is reported by: its directory's basename.
fn display_name(entry: &WorktreeEntry) -> String {
    outcome::name_of(&entry.path)
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

/// What is known about the worktree's branch before anything is asked.
struct BranchFacts {
    /// `None` for a detached worktree.
    name: Option<String>,
    /// The branch's full tip SHA; what its deletion is bound to.
    tip: Option<String>,
    safety: Option<BranchSafety>,
    has_origin: bool,
    /// The live preflight, with `--force-remote`.
    remote: Option<RemoteState>,
}

impl BranchFacts {
    /// The branch's tier (PR lookup and live checks) and, with
    /// `--force-remote`, the remote preflight.
    fn assess(base: &Path, entry: &WorktreeEntry, force_remote: bool) -> Result<Self, WorktreeError> {
        let mut facts = Self { name: entry.branch.clone(), tip: None, safety: None, has_origin: origin_url(base).is_some(), remote: None };
        let Some(branch) = entry.branch.as_deref() else {
            return Ok(facts);
        };
        let tip = branch_tip(base, branch).map_err(|error| {
            in_context(error, &format!("could not read branch {branch} of worktree {}", display_name(entry)))
        })?;
        let heads = live_heads(base);
        let sniff_prs = SniffPrSource::for_origin(base);
        let prs: &dyn PrSource = match &sniff_prs {
            Some(prs) => prs,
            None => &NoPrSource,
        };
        let context = SafetyContext::read(base, branch, sniff_prs.as_ref());
        facts.safety = Some(assess(&context.input(base, branch, &tip, force_remote), prs, &heads));
        facts.remote = force_remote.then(|| preflight_remote_deletion(base, branch, &tip, &heads));
        facts.tip = Some(tip);
        Ok(facts)
    }

    fn lost_commit_count(&self) -> usize {
        self.safety.as_ref().and_then(|s| s.lost_commits.as_ref().ok()).map_or(0, Vec::len)
    }

    /// The approved branch steps, bound to what the report showed: the local
    /// branch at its tip, and the branch on origin only as the preflight saw
    /// it.
    fn approved(&self, actions: &Actions) -> Vec<ApprovedBranch> {
        let mut steps = Vec::new();
        if let (Some(name), Some(tip), Some(BranchStep::Delete { approved })) = (&self.name, &self.tip, actions.branch) {
            steps.push(ApprovedBranch {
                action: BranchAction::Local { name: name.clone(), expected_tip: tip.clone() },
                approval: if approved { StepApproval::Explicit } else { StepApproval::Conditional },
            });
        }
        if let (Some(RemoteState::Present { destination, endpoint, sha, .. }), true) = (&self.remote, actions.delete_remote) {
            steps.push(ApprovedBranch {
                action: BranchAction::Remote {
                    remote: "origin".into(),
                    destination: destination.clone(),
                    endpoint: endpoint.clone(),
                    expected_head: sha.clone(),
                },
                approval: StepApproval::Explicit,
            });
        }
        steps
    }

    /// What the report adds about the branch steps that are not lifecycle
    /// steps: a branch kept by the policy, and a remote deletion the
    /// preflight could not bind.
    fn unbound_report(&self, actions: &Actions) -> outcome::Report {
        let mut report = outcome::Report::default();
        if let (Some(branch), Some(step)) = (&self.name, actions.branch) {
            match step {
                BranchStep::Delete { .. } => {}
                BranchStep::KeepWithWarning => report.lines.push(format!(
                    "<yellow><b>Warning:</b></yellow> kept branch <b>{b}</b>: its commits exist \
                    nowhere else.\n  <dim>Delete it with <i>git branch -D {b}</i> if you do not \
                    need them.</dim>",
                    b = esc(branch)
                )),
                BranchStep::Keep => report.lines.push(format!("<dim>Kept branch <b>{}</b>.</dim>", esc(branch))),
            }
        }
        if actions.delete_remote {
            match &self.remote {
                Some(RemoteState::Present { .. }) => {}
                Some(RemoteState::Unavailable { destination, reason, .. }) => report.failures.push(format!(
                    "<b>origin/{d}</b> was not deleted: {}.\n  <dim>Finish with <i>git push origin --delete {d}</i>.</dim>",
                    visible(&outcome::one_line(reason)),
                    d = visible(destination)
                )),
                // `run` refuses these before removing anything.
                Some(RemoteState::MultiplePushUrls { destination, .. } | RemoteState::ReinterpretedEndpoint { destination, .. }) => {
                    report.failures.push(format!("<b>origin/{}</b> was not deleted.", visible(destination)));
                }
                Some(RemoteState::Absent { .. } | RemoteState::NoRemote) | None => {
                    report.lines.push("<dim>Nothing to delete on origin.</dim>".into());
                }
            }
        }
        report
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
    // One listing finds the base checkout and resolves the name.
    let entries = worktree_entries()?;
    let base = resolve_worktree(&entries, "base")?.path;
    let entry = match lifecycle::resolve_in(&base, entries, name) {
        Ok(Resolved::Live(entry)) => entry,
        // An earlier run's removal is finished with its own approval (R19).
        Ok(Resolved::Pending(id)) => {
            std::env::set_current_dir(&base)?;
            return with_env(&terminal, &base, |env| {
                finish(&terminal, lifecycle::finish_pending(&base, &id, env), None, outcome::Report::default())
            });
        }
        Ok(Resolved::Ambiguous(candidates)) => return Err(ambiguous(name, &candidates)),
        Err(ResolveError::NotFound(_)) => return Err(WorktreeError::WorktreeNotFound(name.to_string())),
        Err(ResolveError::Git(error)) => return Err(error),
        Err(ResolveError::Store(error)) => {
            return Err(WorktreeError::RemovalFailed(format!(
                "<red><b>Nothing was removed.</b></red> The record of an earlier removal could not be read: {}.",
                visible(&error.to_string())
            )));
        }
    };
    if entry.is_main {
        return Err(WorktreeError::GitCommand(format!(
            "refusing to remove the main checkout '{name}'. Use plain git for that."
        )));
    }
    // From here on nothing (not `wt`, not a git child) holds the worktree as
    // its current directory; on Windows that would block removing it.
    std::env::set_current_dir(&base)?;

    let (parent, shell) = parent_process();
    let caller = Caller { cwd: cwd.clone(), shell_wrapper: crate::env::shell_wrapper_active(), shell, parent };
    let gathered = facts::gather(&Sources {
        base: &base,
        entry: &entry,
        usage: &SniffUsage,
        mounts: &NativeMounts,
        git: &repair::Git,
        caller: &caller,
        host: &Host::detect(),
    });
    if let Some(blocker) = &gathered.blocker {
        return Err(WorktreeError::BlockedByEnvironment(visible(&blocker.to_string())));
    }
    let display = display_name(&entry);
    let branch = BranchFacts::assess(&base, &entry, flags.force_remote)?;

    let interactive = crate::env::is_interactive();
    let asking = interactive && !flags.force;
    eprintln!(
        "{}",
        report::render(
            &terminal,
            &report::ReportInput {
                display_name: &display,
                path: &entry.path,
                asking,
                facts: &gathered,
                branch: branch.name.as_deref(),
                safety: branch.safety.as_ref(),
                has_origin: branch.has_origin,
                remote: branch.remote.as_ref(),
            },
        )
    );
    if let Some(refusal) = unprovable_remote(branch.remote.as_ref()) {
        return Err(WorktreeError::RefusedToLoseWork(refusal));
    }

    let situation = Situation {
        warned: gathered.needs_force(),
        branch_safe: branch.safety.as_ref().map(|s| s.tier.allows_deletion()),
        interactive,
    };
    let decision = policy::decide(situation, flags, &mut |question| ask(&terminal, &branch, question));
    let decision = match decision {
        Err(WorktreeError::Cancelled) => Decision::Cancelled,
        other => other?,
    };

    let actions = match decision {
        Decision::NeedsForce => {
            return Err(WorktreeError::RefusedToLoseWork(format!(
                "<red><b>Nothing was removed.</b></red> There is no terminal to ask whether to delete worktree \
                <blue>{}</blue> despite the warnings listed above.\n  <dim>Add <i>--force</i> to delete it anyway.</dim>",
                esc(&display)
            )));
        }
        Decision::Cancelled => {
            eprintln!();
            print(&terminal, format!("<dim>Cancelled. Worktree <blue>{}</blue> was not removed.</dim>", esc(&display)));
            return Ok(());
        }
        Decision::Proceed(actions) => actions,
    };
    let approved = branch.approved(&actions);
    let request = Request { base: &base, entry: &entry, requested_name: name, facts: &gathered, branches: &approved };
    let lost = matches!(actions.branch, Some(BranchStep::Delete { approved: true })).then(|| branch.lost_commit_count());
    let moving = caller.shell_wrapper && is_within(&cwd, &entry.path);
    // Only a checkout directory is handed off. The shell can also stand in
    // the real checkout behind a link to it; that is deleted now and the
    // wrapper moves the shell out afterward.
    if moving && matches!(gathered.target, Target::Checkout { .. }) {
        return hand_off(&terminal, &request, &branch, &actions, &cwd);
    }
    with_env(&terminal, &base, |env| finish(&terminal, lifecycle::remove(&request, env), lost, branch.unbound_report(&actions)))?;
    if moving {
        let (landing, _) = landing(&base, &entry, &cwd)?;
        println!("cd:{}", landing.display());
    }
    Ok(())
}

/// `wt`'s parent process (normally the shell that ran it): its PID, and on
/// Windows its name for the shell-inside refusal.
#[cfg(unix)]
fn parent_process() -> (Option<u32>, Option<String>) {
    (Some(std::os::unix::process::parent_id()), None)
}

/// `wt`'s parent process (normally the shell that ran it): its PID, and on
/// Windows its name for the shell-inside refusal.
#[cfg(windows)]
fn parent_process() -> (Option<u32>, Option<String>) {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

    let own = Pid::from_u32(std::process::id());
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[own]), true, ProcessRefreshKind::nothing());
    let Some(parent) = system.process(own).and_then(sysinfo::Process::parent) else {
        return (None, None);
    };
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[parent]), true, ProcessRefreshKind::nothing());
    let name = system.process(parent).map(|process| process.name().to_string_lossy().into_owned());
    (Some(parent.as_u32()), name)
}

/// Runs `body` with the lifecycle's surroundings on this host; Ctrl-C stops
/// the lifecycle at its next safe point while `body` runs.
fn with_env<T>(terminal: &Terminal, base: &Path, body: impl FnOnce(&Env<'_>) -> T) -> T {
    // Opened only now: an interrupt while asking keeps its default effect.
    let window = interrupt::Window::open();
    let cancelled = || window.interrupted();
    let revalidate = Reconfirm { base };
    let notify = Stderr { terminal };
    let env = Env {
        usage: &SniffUsage,
        mounts: &NativeMounts,
        clock: &HostClock,
        faults: &NoFaults,
        cancelled: &cancelled,
        revalidate: &revalidate,
        notify: &notify,
    };
    body(&env)
}

/// Prints how the removal ended; `Err` carries the exit code.
fn finish(terminal: &Terminal, removal: Removal, lost: Option<usize>, extra: outcome::Report) -> Result<(), WorktreeError> {
    let finished = match removal {
        Removal::Refused(refusal) => return Err(outcome::refusal_error(&refusal)),
        Removal::Finished(finished) => finished,
    };
    let mut report = outcome::finished_report(&finished, &outcome::name_of(&finished.path), lost);
    report.lines.extend(extra.lines);
    report.failures.extend(extra.failures);
    eprintln!();
    for line in &report.lines {
        print(terminal, line.as_str());
    }
    if report.failures.is_empty() && finished.exit_class() == ExitClass::Success {
        Ok(())
    } else {
        Err(outcome::error_for(ExitClass::Failure, report.failures.join("\n")))
    }
}

/// [`Revalidate`] with the same proof the first run used: local first, then
/// a PR or a live `origin/*` head.
struct Reconfirm<'a> {
    base: &'a Path,
}

impl Revalidate for Reconfirm<'_> {
    fn still_safe(&self, name: &str, tip: &str, remote_deleted: bool) -> Result<(), String> {
        let reconfirmation = reconfirm_branch(self.base, name, tip, remote_deleted);
        if reconfirmation.tier.allows_deletion() { Ok(()) } else { Err(no_longer_safe(&reconfirmation)) }
    }
}

/// [`Notify`] on stderr.
struct Stderr<'t> {
    terminal: &'t Terminal,
}

impl Notify for Stderr<'_> {
    fn removal_running(&self) {
        print(
            self.terminal,
            "<dim>Another <i>wt remove</i> of this worktree is running; waiting for it to finish (Ctrl-C to stop waiting).</dim>",
        );
    }

    fn waiting(&self, wait: &Wait) {
        print(self.terminal, format!("<yellow>Waiting:</yellow> {}", visible(&wait.to_string())));
    }
}

/// A name that matches more than one target: never resolved by picking one.
fn ambiguous(name: &str, candidates: &[Candidate]) -> WorktreeError {
    let live: Option<Vec<WorktreeCandidate>> = candidates
        .iter()
        .map(|candidate| match candidate {
            Candidate::Live(entry) => Some(WorktreeCandidate {
                branch: entry.branch.clone(),
                basename: display_name(entry),
                path: entry.path.clone(),
            }),
            Candidate::Pending { .. } => None,
        })
        .collect();
    match live {
        // Only live worktrees: the ambiguity `wt` has always reported.
        Some(candidates) => WorktreeError::AmbiguousWorktree { name: name.to_string(), candidates },
        None => WorktreeError::RefusedToLoseWork(outcome::ambiguous_markup(name, candidates)),
    }
}

/// `--force-remote` cannot show that it deletes from exactly the repository
/// reported.
fn unprovable_remote(state: Option<&RemoteState>) -> Option<String> {
    match state? {
        RemoteState::MultiplePushUrls { .. } => Some(
            "<red><b>Nothing was removed.</b></red> Origin pushes to more than one repository, \
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
            "<red><b>Nothing was removed.</b></red> Git would rewrite origin's push URL <b>{}</b> \
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
            "<red><b>Nothing was removed.</b></red> Git would read origin's push URL <b>{}</b> \
            as the name of the remote defined by <i>{}</i>, so <i>--force-remote</i> cannot delete \
            the branch from exactly the repository reported.\n  \
            <dim>Leave out <i>--force-remote</i>, or rename that remote.</dim>",
            esc(endpoint),
            esc(source)
        )),
        _ => None,
    }
}

/// Asks one question after exactly one blank line.
fn ask(terminal: &Terminal, branch: &BranchFacts, question: Question) -> Result<bool, WorktreeError> {
    match question {
        Question::DeleteCheckout => {
            eprintln!();
            let label = InlineProse::new("Delete it?").render(terminal);
            Confirm::new(&label).with_default(false).prompt().map_err(map_inquire_err)
        }
        Question::DeleteUnsafeBranch => {
            let commits = branch.safety.as_ref().and_then(|s| s.lost_commits.as_ref().ok()).cloned().unwrap_or_default();
            eprintln!();
            print(terminal, report::lost_commits_markup(&commits));
            eprintln!();
            let lose = format!("Delete it and lose {} commit{}", commits.len(), if commits.len() == 1 { "" } else { "s" });
            let label = InlineProse::new(format!(
                "Branch <blue>{}</blue> is not safe to delete:",
                esc(branch.name.as_deref().unwrap_or_default())
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
        InquireError::OperationCanceled | InquireError::OperationInterrupted => WorktreeError::Cancelled,
        e => WorktreeError::Io(std::io::Error::other(e.to_string())),
    }
}

/// The directory the caller lands in: the fork parent's worktree when the
/// branch has a fork-origin record and that parent has a worktree; otherwise
/// the base repo. Returns the directory and how to describe it.
fn landing_root(base: &Path, entry: &WorktreeEntry, entries: &[WorktreeEntry]) -> (PathBuf, String) {
    let parent = entry.branch.as_deref().and_then(|branch| {
        let store = ForkOriginStore::load_from(&fork_origin_path(base).ok()?);
        store.get(branch).map(|origin| origin.base_branch.clone())
    });
    if let Some(parent) = parent
        && let Some(worktree) = entries
            .iter()
            .find(|e| e.branch.as_deref() == Some(parent.as_str()) && canonical(&e.path) != canonical(&entry.path))
    {
        let description = if worktree.is_main {
            format!("the base repo (on <b>{}</b>, the branch this one was forked from)", esc(&parent))
        } else {
            format!("the <blue>{}</blue> worktree (the branch this one was forked from)", esc(&parent))
        };
        return (worktree.path.clone(), description);
    }
    (base.to_path_buf(), "the base repo".to_string())
}

/// Where the wrapper moves a caller who stands in `entry`, and how to
/// describe it: [`landing_root`], keeping the caller's subdirectory when it
/// exists there too (`wt go`'s rule).
fn landing(base: &Path, entry: &WorktreeEntry, cwd: &Path) -> Result<(PathBuf, String), WorktreeError> {
    let entries = worktree_entries()?;
    let (root, description) = landing_root(base, entry, &entries);
    let relative = canonical(cwd).strip_prefix(canonical(&entry.path)).map(Path::to_path_buf).unwrap_or_default();
    let landing = if root.join(&relative).is_dir() { root.join(&relative) } else { root };
    Ok((landing, description))
}

fn worktree_entries() -> Result<Vec<WorktreeEntry>, WorktreeError> {
    Ok(parse_worktree_list(&worktree::git::worktree_list()?))
}

/// First run of a removal the caller stands in (R18): save the approval as
/// a pending operation, then tell the wrapper where to move and which token
/// finishes it. Nothing is unlocked, moved, or deleted here.
fn hand_off(
    terminal: &Terminal,
    request: &Request<'_>,
    branch: &BranchFacts,
    actions: &Actions,
    cwd: &Path,
) -> Result<(), WorktreeError> {
    let id = with_env(terminal, request.base, |env| lifecycle::save_approval(request, env))
        .map_err(|refusal| outcome::refusal_error(&refusal))?;
    // What the second run cannot say: a branch the policy kept, and a
    // remote deletion the preflight could not bind.
    let unbound = branch.unbound_report(actions);
    for line in unbound.lines.iter().chain(&unbound.failures) {
        print(terminal, line.as_str());
    }

    let (landing, description) = landing(request.base, request.entry, cwd)?;

    let token = handoff::new_token()?;
    let path = handoff::handoff_path(request.base, &token)
        .map_err(|_| WorktreeError::Io(std::io::Error::other("user cache directory unavailable")))?;
    let shown = lifecycle::late::shown(request.facts.inventory.as_ref());
    let record = OperationHandoff::new(id, canonical(request.base), canonical(&request.entry.path), now(), shown);
    handoff::write_operation(&path, &record)?;

    eprintln!();
    print(
        terminal,
        format!(
            "Moving you to {description} <dim>at {}</dim> to finish removing the worktree.",
            esc(&landing.display().to_string())
        ),
    );
    println!("cd:{}", landing.display());
    println!("remove-handoff:{token}");
    Ok(())
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_secs()).unwrap_or_default()
}

/// [`reconfirm`] for a later run: local proof first, then a PR or a live
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

/// Why a conditionally approved branch is kept, in plain words, with the
/// evidence that was tried.
fn no_longer_safe(reconfirmation: &Reconfirmation) -> String {
    let mut reason = "it is no longer safe to delete".to_string();
    if let Tier::Unknown(why) = &reconfirmation.tier {
        reason.push_str(&format!(" (where its commits live could not be checked: {why})"));
    }
    for note in &reconfirmation.notes {
        reason.push_str(&format!("; {note}"));
    }
    reason
}

/// `wt remove --handoff <token>`, run by the wrapper after its `cd` (R18).
///
/// The first run saved the approval as a pending operation; this run checks
/// that the caller left the worktree and that the repository is the same,
/// then finishes the operation with every branch protection rechecked. It
/// compares nothing about the checkout's content and asks nothing; the final
/// line names uncommitted files the first run's question did not show (R14).
pub fn run_handoff(token: &str) -> Result<(), WorktreeError> {
    let terminal = Terminal::default();
    let cwd = std::env::current_dir()?;
    let entries = worktree_entries()?;
    let base = resolve_worktree(&entries, "base")?.path;

    let record = handoff::handoff_path(&base, token)
        .and_then(|path| handoff::consume_operation(&path, now()))
        .map_err(|error| {
            WorktreeError::RefusedToLoseWork(match error {
                HandoffError::Missing => {
                    "<red><b>Nothing was removed.</b></red> There is no pending removal for this handoff token.".to_string()
                }
                HandoffError::Expired => "<red><b>Nothing was removed.</b></red> The removal handoff expired before the \
                    shell moved.\n  <dim>Run <i>wt remove</i> with the worktree's name to finish it.</dim>"
                    .to_string(),
            })
        })?;
    std::env::set_current_dir(&base)?;
    with_env(&terminal, &base, |env| {
        finish(&terminal, lifecycle::finish_handoff(&base, &record, &cwd, env), None, outcome::Report::default())
    })?;
    print(&terminal, format!("<dim>You are now in {}.</dim>", esc(&cwd.display().to_string())));
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
            WorktreeError::RemovalFailed("markup".into()),
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
