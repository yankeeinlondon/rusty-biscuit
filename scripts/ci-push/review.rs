//! The trigger review: every context of every branch update, planned from
//! the reviewed revision's own committed tree and checked against the
//! recorded execution constraints, before anything is published or run.
//!
//! What an update runs depends on the workflow it triggers. A push to `main`
//! starts the push run, whose base is the remote's previous tip (all zeros
//! plans full scope). A push to any other branch starts one pull-request run
//! per open pull request from it, against that pull request's base-branch
//! tip on the remote; when the same push also updates that base, the server
//! may apply the two updates in either order, so both states are reviewed.
//! With no open pull request (or a remote that is not GitHub) nothing runs
//! now, and a provisional plan against the remote's `main` is reviewed
//! anyway: the pull request opened next, from the web UI, is a trigger no
//! hook sees.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use gix::ObjectId;

use super::constraints;
use super::git::{GithubRemote, github_repo_of};
use super::{
    BlockReason, ContextSource, Flow, HelperError, PushMode, RefUpdate, ReviewContext,
    TriggerEvent, parse_pull_requests, parse_resolved_plan, short,
};

/// Where a context's committed tree was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeSource {
    /// The clean checkout is the reviewed revision.
    Checkout,
    /// A temporary detached worktree of the reviewed revision.
    Temporary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStatus {
    Passed,
    /// The push was blocked while this context was reviewed.
    Blocked,
}

/// One entry of the review ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextReview {
    pub context: ReviewContext,
    /// The reviewed revision (the update's local object).
    pub trigger: ObjectId,
    pub tree: TreeSource,
    /// Branch names a constraint is matched under: the remote branch, then
    /// the local one when the refspec renames.
    pub branches: Vec<String>,
    pub status: ReviewStatus,
}

/// HEAD's first context: the plan the gates consume and the receipts bind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FirstContext {
    /// Null for a branch-creating push of `main`, which binds no receipt.
    pub base: ObjectId,
    /// The planner's resolved plan, before evidence.
    pub raw_plan: PathBuf,
    /// The plan with verified evidence applied, as reviewed.
    pub review_plan: PathBuf,
}

/// A scope receipt `scope-record` wrote for HEAD's first context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScopeReceipt {
    pub base: ObjectId,
    pub path: PathBuf,
}

/// What the review established for the steps after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PushReview {
    /// HEAD when the review started.
    pub head: ObjectId,
    pub initial_dirty: bool,
    /// A reviewed branch update pushes HEAD's commit.
    pub pushes_head: bool,
    /// A reviewed update starts a CI run now (main, or an open pull request).
    pub starts_run: bool,
    pub first: Option<FirstContext>,
    pub scope_receipt: Option<ScopeReceipt>,
}

/// Reviews every branch update in Git's order. The first failure blocks the
/// whole push.
pub(super) fn review_push(
    flow: &mut Flow<'_, '_>,
    remote_name: &str,
    mode: PushMode,
    updates: &[RefUpdate],
    staging: &Path,
) -> Result<PushReview, BlockReason> {
    flow.info(if mode == PushMode::ScopeOnly {
        "\nCalculating pre-push scope only; no gate will run."
    } else {
        "\nResolving the plan this push would trigger"
    });
    let head = flow.session.head().map_err(|error| BlockReason::Review {
        update: None,
        problem: format!("cannot read the checkout's HEAD: {error}"),
    })?;
    let initial_dirty = flow.session.is_dirty().map_err(|error| BlockReason::Review {
        update: None,
        problem: format!("cannot read the checkout's status: {error}"),
    })?;
    if let Err(error) = flow.session.select_python() {
        flow.interruption()?;
        return Err(BlockReason::Review {
            update: None,
            problem: format!("the planner and evidence helpers need Python: {error}"),
        });
    }
    let mut review = PushReview {
        head: head.commit,
        initial_dirty,
        pushes_head: false,
        starts_run: false,
        first: None,
        scope_receipt: None,
    };
    let remote = Remote {
        name: remote_name.to_owned(),
        url: flow.session.remote_url().map(str::to_owned),
    };
    for (index, update) in updates.iter().enumerate() {
        if !update.is_reviewed() {
            continue;
        }
        if update.local_oid == review.head {
            review.pushes_head = true;
        }
        review_update(flow, &mut review, &remote, updates, index, staging)?;
    }
    flow.interruption()?;
    Ok(review)
}

/// The remote the push writes to.
struct Remote {
    name: String,
    url: Option<String>,
}

impl Remote {
    /// The URL as messages print it and constraints match it; empty is an
    /// unknown identity, which matches every repository-scoped record.
    fn url(&self) -> &str {
        self.url.as_deref().unwrap_or_default()
    }
}

/// One update: its contexts, its committed tree, and each context's review.
fn review_update(
    flow: &mut Flow<'_, '_>,
    review: &mut PushReview,
    remote: &Remote,
    updates: &[RefUpdate],
    index: usize,
    staging: &Path,
) -> Result<(), BlockReason> {
    let update = &updates[index];
    let label = update.label();
    let trigger = update.local_oid;
    let remote_branch = update.remote_branch().unwrap_or_default().to_owned();
    flow.info(format!("\nReviewing {label} ({})", short(trigger)));
    // CI validates the remote branch; a renamed refspec keeps the local name
    // in force too, so a constraint recorded under either name still binds.
    let mut branches = vec![remote_branch.clone()];
    match update.local_branch() {
        Some(local) if local != remote_branch => {
            branches.push(local.to_owned());
            flow.info(format!(
                "Branch identity: {remote_branch} (pushed from {local}; a constraint recorded under either name applies)"
            ));
        }
        _ => flow.info(format!("Branch identity: {remote_branch}")),
    }
    flow.info(format!("Remote: {} ({})", remote.name, remote.url()));

    let blocked = |problem: String| BlockReason::Review {
        update: Some(label.clone()),
        problem,
    };
    let contexts = resolve_contexts(flow, review, remote, updates, index).map_err(|error| match error {
        ContextError::Blocked(reason) => reason,
        ContextError::Problem(problem) => blocked(problem),
    })?;

    // The checkout serves as the committed tree only when it IS that tree.
    let why_temporary = if trigger != review.head {
        Some(format!("{} is not HEAD", short(trigger)))
    } else if review.initial_dirty {
        Some("the checkout has uncommitted changes".to_owned())
    } else {
        None
    };
    let (root, tree) = match why_temporary {
        None => (flow.session.root().to_path_buf(), TreeSource::Checkout),
        Some(why) => {
            let root = flow.session.materialize(trigger).map_err(|error| {
                if let Some(cleanup) = &error.cleanup {
                    flow.say(super::Diagnostic::Advisory(cleanup.to_string()));
                }
                blocked(format!(
                    "could not materialize the committed tree of {} in a temporary worktree: {}",
                    short(trigger),
                    error.message
                ))
            })?;
            flow.info(format!(
                "Reviewing the committed tree of {} in a temporary worktree ({why}).",
                short(trigger)
            ));
            (root, TreeSource::Temporary)
        }
    };

    let mut result = Ok(());
    for (position, (context, kind)) in contexts.iter().enumerate() {
        let target = Target {
            label: &label,
            trigger,
            root: &root,
            tree,
            branches: &branches,
            kind,
            first_of_update: position == 0,
            contexts: contexts.len(),
        };
        result = review_context(flow, review, remote, context, &target, staging);
        if result.is_err() {
            break;
        }
    }
    if tree == TreeSource::Temporary {
        if let Err(failure) = flow.session.release_tree() {
            flow.say(super::Diagnostic::Advisory(failure.to_string()));
        }
    }
    result
}

/// The update-level facts one context's review needs.
struct Target<'a> {
    label: &'a str,
    trigger: ObjectId,
    root: &'a Path,
    tree: TreeSource,
    branches: &'a [String],
    /// Why this base, for the narration.
    kind: &'a str,
    first_of_update: bool,
    /// How many contexts the update has.
    contexts: usize,
}

enum ContextError {
    Blocked(BlockReason),
    Problem(String),
}

impl From<BlockReason> for ContextError {
    fn from(reason: BlockReason) -> Self {
        ContextError::Blocked(reason)
    }
}

/// Every run the update triggers, each with the narration of its base, in
/// order: main's push run; or each open pull request in `gh`'s order, the
/// base's current tip before a distinct incoming revision; or the
/// provisional plan against the remote's `main`.
fn resolve_contexts(
    flow: &mut Flow<'_, '_>,
    review: &mut PushReview,
    remote: &Remote,
    updates: &[RefUpdate],
    index: usize,
) -> Result<Vec<(ReviewContext, String)>, ContextError> {
    let update = &updates[index];
    let event = update.event();
    let context = |base: ObjectId, source: ContextSource| ReviewContext {
        update: index,
        base,
        event,
        source,
    };
    if event == TriggerEvent::Push {
        review.starts_run = true;
        if update.remote_oid.is_null() {
            return Ok(vec![(
                context(update.remote_oid, ContextSource::MainPush),
                "the push event's before is all zeros for a branch-creating push of main, so CI plans full scope".to_owned(),
            )]);
        }
        let present = flow.session.ensure_commit(update.remote_oid, "main");
        flow.interruption()?;
        return match present {
            Ok(true) => Ok(vec![(
                context(update.remote_oid, ContextSource::MainPush),
                "the remote's main, the push event's before".to_owned(),
            )]),
            Ok(false) => Err(ContextError::Problem(format!(
                "the remote's main is at {}, which is not available locally even after fetching {}; fetch and push again",
                update.remote_oid, remote.name
            ))),
            Err(error) => Err(ContextError::Problem(error.to_string())),
        };
    }

    let branch = update.remote_branch().unwrap_or_default();
    let why_provisional = match github_repo_of(remote.url()) {
        GithubRemote::Unrecognized => {
            return Err(ContextError::Problem(format!(
                "{} names github.com in a form this hook does not read, so the open pull requests that decide what this update runs cannot be listed; use the git@github.com:OWNER/REPO or https://github.com/OWNER/REPO form, or push with --no-verify to skip this review entirely",
                remote.url()
            )));
        }
        GithubRemote::NotGithub => format!(
            "no open pull request, since {} is not a GitHub remote",
            remote.url()
        ),
        GithubRemote::Repository(repository) => {
            let pull_requests = list_pull_requests(flow, remote, &repository, branch)?;
            let mut contexts = Vec::new();
            for pull_request in pull_requests {
                let base_branch = &pull_request.base_branch;
                let states = target_states(flow, updates, base_branch).map_err(|problem| {
                    ContextError::Problem(format!(
                        "the base of pull request #{} into {base_branch} cannot be established: {problem}; push with --no-verify to skip this review entirely",
                        pull_request.number
                    ))
                })?;
                if states.len() > 1 {
                    flow.info(format!(
                        "{base_branch} is updated by this push too, so the run of pull request #{} may see either its current tip or the incoming revision as its base; both are reviewed.",
                        pull_request.number
                    ));
                }
                for (base, incoming) in states {
                    let kind = if incoming {
                        format!(
                            "pull request #{} into {base_branch}, the revision this push also writes to {base_branch}",
                            pull_request.number
                        )
                    } else {
                        format!(
                            "pull request #{} into {base_branch}, the tip of {base_branch} on {}",
                            pull_request.number, remote.name
                        )
                    };
                    let source = ContextSource::PullRequest {
                        number: pull_request.number,
                        base_branch: base_branch.clone(),
                        incoming,
                    };
                    contexts.push((context(base, source), kind));
                }
            }
            if !contexts.is_empty() {
                review.starts_run = true;
                return Ok(contexts);
            }
            format!("no open pull request from {branch} on {repository}")
        }
    };

    let states = target_states(flow, updates, "main").map_err(|problem| {
        ContextError::Problem(format!(
            "this update triggers no run now, and the provisional plan against the remote's main cannot be resolved: {problem}"
        ))
    })?;
    Ok(states
        .into_iter()
        .map(|(base, incoming)| {
            let kind = if incoming {
                format!("provisional: {why_provisional}; main is updated by this push too, and a pull request opened against it once that lands will run this plan")
            } else {
                format!("provisional: {why_provisional}; a pull request opened against main before main advances will run this plan")
            };
            (context(base, ContextSource::Provisional { incoming }), kind)
        })
        .collect())
}

fn list_pull_requests(
    flow: &mut Flow<'_, '_>,
    remote: &Remote,
    repository: &str,
    branch: &str,
) -> Result<Vec<super::PullRequest>, ContextError> {
    let listed = flow.session.list_pull_requests(repository, branch);
    flow.interruption()?;
    let completed = match listed {
        Ok(completed) => completed,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(ContextError::Problem(format!(
                "{} is a GitHub remote, so the open pull requests from {branch} decide what this update runs, but `gh` is not installed; install it, or push with --no-verify to skip this review entirely",
                remote.url()
            )));
        }
        Err(error) => {
            return Err(ContextError::Problem(format!(
                "could not list the open pull requests from {branch} on {repository}: {error}; push with --no-verify to skip this review entirely"
            )));
        }
    };
    if !completed.success() {
        let stderr = String::from_utf8_lossy(&completed.stderr).replace(['\r', '\n'], " ");
        return Err(ContextError::Problem(format!(
            "could not list the open pull requests from {branch} on {repository}: `gh pr list` failed with: {}; fix that (usually `gh auth login`), or push with --no-verify to skip this review entirely",
            stderr.trim()
        )));
    }
    parse_pull_requests(&completed.stdout, "`gh pr list` output").map_err(|error| {
        ContextError::Problem(format!(
            "`gh pr list` returned something ci-push could not read for {branch} on {repository}: {error}"
        ))
    })
}

/// The base states a run into `branch` can see, the remote's current tip
/// first, then (`true`) this push's own distinct revision of `branch`. A
/// push deleting `branch` leaves no state this hook can vouch for: the run
/// may fire against the old tip before the deletion lands, and has no base
/// afterwards.
fn target_states(
    flow: &mut Flow<'_, '_>,
    updates: &[RefUpdate],
    branch: &str,
) -> Result<Vec<(ObjectId, bool)>, String> {
    let destination = format!("refs/heads/{branch}");
    let incoming = updates.iter().find(|update| update.remote_ref == destination);
    if incoming.is_some_and(|update| update.local_oid.is_null()) {
        return Err(format!(
            "this push also deletes {branch} ({destination}), so a run into it may fire against the old tip before the deletion lands and has no base at all afterwards"
        ));
    }
    let tip = flow.session.remote_tip(branch);
    if flow.session.cancelled().is_some() {
        return Err("the push was interrupted".to_owned());
    }
    let mut states = Vec::new();
    match tip {
        Ok(tip) => states.push((tip, false)),
        // A branch this same push creates has no current tip to review.
        Err(_) if incoming.is_some_and(|update| update.remote_oid.is_null()) => {}
        Err(error) => return Err(error.to_string()),
    }
    if let Some(incoming) = incoming {
        if states.first().is_none_or(|(tip, _)| *tip != incoming.local_oid) {
            states.push((incoming.local_oid, true));
        }
    }
    Ok(states)
}

/// One context: plan, verify, apply, render, and check constraints; for
/// HEAD's first context, also capture its plans and record its scope.
fn review_context(
    flow: &mut Flow<'_, '_>,
    review: &mut PushReview,
    remote: &Remote,
    context: &ReviewContext,
    target: &Target<'_>,
    staging: &Path,
) -> Result<(), BlockReason> {
    let entry = flow.report.outcome.reviews.len();
    flow.report.outcome.reviews.push(ContextReview {
        context: context.clone(),
        trigger: target.trigger,
        tree: target.tree,
        branches: target.branches.to_vec(),
        status: ReviewStatus::Blocked,
    });
    let base = context.base;
    let base_text = if base.is_null() { base.to_string() } else { short(base) };
    flow.info(format!(
        "Comparison base for {}: {base_text} ({})",
        short(target.trigger),
        target.kind
    ));
    let directory = staging.join(format!("context-{}", entry + 1));
    let blocked = |problem: String| BlockReason::Review {
        update: Some(target.label.to_owned()),
        problem,
    };
    std::fs::create_dir_all(&directory)
        .map_err(|error| blocked(format!("cannot create {}: {error}", directory.display())))?;
    let files = ContextFiles::new(&directory);

    // Execution constraints bind the trigger, not the gates, so they are
    // decided from the plan CI would resolve for this revision, never from
    // a selection override. The store is kept out of the planner so that the
    // check below is the single decision point: a cell the planner had
    // already resolved to `prohibited` would no longer show as executing.
    let scripts = target.root.join("scripts").join("ci");
    let mut planner: Vec<OsString> = vec![
        scripts.join("affected_scope.py").into(),
        "--event".into(),
        context.event.as_str().into(),
        "--plan-out".into(),
        files.raw_plan.clone().into(),
        "--base".into(),
        base.to_string().into(),
        "--head".into(),
        target.trigger.to_string().into(),
    ];
    if base.is_null() {
        planner.push("--all".into());
    } else {
        let arguments = flow.session.arguments_file(base, target.trigger).map_err(|error| {
            blocked(format!(
                "cannot compare {base} with {} to plan its committed scope: {error}",
                target.trigger
            ))
        })?;
        std::fs::write(&files.arguments, arguments)
            .map_err(|error| blocked(format!("cannot write {}: {error}", files.arguments.display())))?;
        planner.extend(["--arguments-file".into(), files.arguments.clone().into()]);
    }
    let scope = flow.helper(target.root, &planner).map_err(|error| {
        error.block(|detail| {
            blocked(format!(
                "the planner could not resolve the committed scope of {} against {base_text}: {detail}",
                short(target.trigger)
            ))
        })
    })?;
    write(&files.scope, &scope.stdout).map_err(blocked)?;

    let checkout = flow.session.root().to_path_buf();
    let verify: Vec<OsString> = vec![
        scripts.join("local_evidence.py").into(),
        "verify".into(),
        "--cells".into(),
        "--plan".into(),
        files.raw_plan.clone().into(),
        "--base".into(),
        base.to_string().into(),
        "--head".into(),
        target.trigger.to_string().into(),
        "--rejections".into(),
        files.rejected.clone().into(),
    ];
    let accepted = flow.helper(&checkout, &verify).map_err(|error| {
        error.block(|detail| {
            blocked(format!(
                "published evidence could not be verified against the resolved plan of {}: {detail}",
                short(target.trigger)
            ))
        })
    })?;
    write(&files.accepted, &accepted.stdout).map_err(blocked)?;

    let apply: Vec<OsString> = vec![
        scripts.join("affected_scope.py").into(),
        "--apply-to".into(),
        files.raw_plan.clone().into(),
        "--accepted-cells".into(),
        files.accepted.clone().into(),
        "--evidence-rejections".into(),
        files.rejected.clone().into(),
        "--resolved-plan".into(),
        "--plan-out".into(),
        files.review_plan.clone().into(),
    ];
    flow.helper(&checkout, &apply).map_err(|error| {
        error.block(|detail| {
            blocked(format!(
                "the verified evidence could not be applied to the resolved plan of {}: {detail}",
                short(target.trigger)
            ))
        })
    })?;

    let refuse = |refusal: String| BlockReason::Constraint {
        update: target.label.to_owned(),
        refusals: vec![refusal],
    };
    let plan = std::fs::read(&files.review_plan).map_err(|error| {
        refuse(format!(
            "the reviewed plan {} cannot be read: {error}",
            files.review_plan.display()
        ))
    })?;
    match flow.session.render_plan(&plan) {
        Ok(rendered) => flow.info(rendered),
        Err(error) => flow.info(format!("(the resolved plan could not be rendered: {error})")),
    }
    let document = files.review_plan.display().to_string();
    parse_resolved_plan(&plan, &document).map_err(|error| refuse(error.to_string()))?;
    check_constraints(flow, remote, target, &plan, &document)?;

    if target.trigger == review.head && target.first_of_update && review.first.is_none() {
        review.first = Some(FirstContext {
            base,
            raw_plan: files.raw_plan.clone(),
            review_plan: files.review_plan.clone(),
        });
        record_scope(flow, review, target, &files)?;
    }
    flow.report.outcome.reviews[entry].status = ReviewStatus::Passed;
    Ok(())
}

/// The staging files of one context.
struct ContextFiles {
    arguments: PathBuf,
    scope: PathBuf,
    raw_plan: PathBuf,
    accepted: PathBuf,
    rejected: PathBuf,
    review_plan: PathBuf,
    scope_receipt: PathBuf,
}

impl ContextFiles {
    fn new(directory: &Path) -> Self {
        ContextFiles {
            arguments: directory.join("arguments"),
            scope: directory.join("scope.json"),
            raw_plan: directory.join("plan.json"),
            accepted: directory.join("accepted.json"),
            rejected: directory.join("rejected.json"),
            review_plan: directory.join("review.json"),
            scope_receipt: directory.join("scope-receipt.json"),
        }
    }
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// Refuses the context when a recorded constraint is unsatisfied by its
/// evidence-applied plan, matched under the pushed URL and every branch name
/// of the update. A store or plan that cannot be read refuses it too.
fn check_constraints(
    flow: &mut Flow<'_, '_>,
    remote: &Remote,
    target: &Target<'_>,
    plan: &[u8],
    document: &str,
) -> Result<(), BlockReason> {
    let refuse = |refusals: Vec<String>| BlockReason::Constraint {
        update: target.label.to_owned(),
        refusals,
    };
    let variable = flow.session.variable(constraints::ENVIRONMENT_VARIABLE);
    let home = flow.session.home();
    let explicit = variable.as_deref().filter(|value| !value.is_empty());
    let directory = match (&home, explicit) {
        (_, Some(_)) | (Some(_), None) => constraints::resolve_directory(
            home.as_deref().unwrap_or(Path::new("")),
            remote.url(),
            None,
            explicit,
        ),
        (None, None) => {
            return Err(refuse(vec![format!(
                "no home directory is known, so the constraint store cannot be located; set {}",
                constraints::ENVIRONMENT_VARIABLE
            )]));
        }
    };
    let records = constraints::load(&directory)
        .map_err(|error| refuse(vec![format!("ci-constraints: {error}")]))?;
    let cells = super::parse_constraint_plan(plan, document)
        .map_err(|error| refuse(vec![format!("ci-constraints: {error}")]))?;
    let selection = constraints::select(records, remote.url(), target.branches, flow.session.today());
    for expired in &selection.expired {
        flow.info(format!(
            "ci-constraints: ignoring expired constraint — {}",
            expired.describe()
        ));
    }
    let mut binding = selection.active;
    binding.extend(selection.malformed);
    let refusals: Vec<String> = constraints::unsatisfied(&binding, Some(&cells))
        .into_iter()
        .map(constraints::Constraint::describe)
        .collect();
    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refuse(refusals))
    }
}

/// Records HEAD's scope receipt for its first context. Recording is
/// advisory: CI calculates scope itself on any miss. An all-zero base binds
/// no receipt.
fn record_scope(
    flow: &mut Flow<'_, '_>,
    review: &mut PushReview,
    target: &Target<'_>,
    files: &ContextFiles,
) -> Result<(), BlockReason> {
    let head = short(review.head);
    let Some(first) = &review.first else {
        return Ok(());
    };
    let base = first.base;
    if base.is_null() {
        flow.info(format!(
            "No scope receipt for {head}: a branch-creating push of main carries no comparison base, so CI calculates full scope itself."
        ));
        return Ok(());
    }
    let checkout = flow.session.root().to_path_buf();
    let record: Vec<OsString> = vec![
        target.root.join("scripts").join("ci").join("local_evidence.py").into(),
        "scope-record".into(),
        "--plan".into(),
        files.raw_plan.clone().into(),
        "--scope".into(),
        files.scope.clone().into(),
        "--base".into(),
        base.to_string().into(),
        "--head".into(),
        review.head.to_string().into(),
    ];
    let recorded = match flow.helper(&checkout, &record) {
        Ok(recorded) => recorded,
        Err(HelperError::Interrupted(exit_code)) => {
            return Err(BlockReason::Interrupted { exit_code });
        }
        Err(HelperError::Failed { detail, .. }) => {
            flow.say(super::Diagnostic::Advisory(format!(
                "Scope evidence RECORDING failed for {head}; nothing is published and CI will calculate scope itself. {detail}"
            )));
            return Ok(());
        }
    };
    if let Err(problem) = write(&files.scope_receipt, &recorded.stdout) {
        flow.say(super::Diagnostic::Advisory(format!(
            "Scope evidence RECORDING failed for {head}; nothing is published and CI will calculate scope itself. {problem}"
        )));
        return Ok(());
    }
    review.scope_receipt = Some(ScopeReceipt {
        base,
        path: files.scope_receipt.clone(),
    });
    if target.contexts > 1 {
        flow.info(format!(
            "The scope receipt for {head} binds its first comparison base only ({}); one note holds one receipt, and CI calculates scope itself for every other context.",
            target.kind
        ));
    }
    Ok(())
}
