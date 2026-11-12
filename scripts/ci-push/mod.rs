//! The pre-push hook's decisions: which ref updates to review, against which
//! comparison bases, whether a constraint refuses the push, which gates run,
//! and what evidence may be published.
//!
//! `.githooks/pre-push` launches the `ci-push` binary; the binary reads the
//! environment and stdin into a [`PushRequest`] and exits with the status of
//! [`run`]'s [`PushOutcome`]. Every boundary `run` crosses (repository
//! discovery, Git, child processes, clock, home directory, environment) is a
//! [`PushServices`] or [`PushSession`] method, so the decisions are tested
//! in-process.
//!
//! One invocation runs in this order, and a later step never starts once an
//! earlier one blocked:
//!
//! 1. validate the mode, the selection, and the complete update list;
//! 2. review every context of every branch update ([`review`]);
//! 3. publish HEAD's scope receipt (advisory);
//! 4. resolve `auto`, then run `just pre-push` for `strict`/`warn`;
//! 5. decide and publish the validation receipt ([`evidence`]).

pub mod constraints;
mod documents;
mod evidence;
pub mod git;
mod host;
mod json;
mod mode;
pub mod process;
pub mod python;
mod review;
mod updates;
pub mod worktree;

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use gix::ObjectId;

pub use documents::{
    ConstraintCell, ConstraintRecord, DocumentError, PlanCell, PullRequest, ResolvedPlan,
    parse_constraint_plan, parse_constraint_record, parse_pull_requests, parse_resolved_plan,
};
pub use evidence::{ScopePublication, Validation, ci_environment, short};
pub use host::HostServices;
pub use mode::{ModeError, ModeSetting, PushMode, SelectionError, parse_selection};
pub use review::{ContextReview, ReviewStatus, TreeSource};
pub use updates::{InputError, RefUpdate, SkipReason, TriggerEvent, parse_updates};

use git::{GitError, HeadIdentity, RemoteTipError};
use process::{CancelSignal, Completed, ProcessError};
use python::PythonSelectionError;
use worktree::{CleanupFailure, CreateError};

/// Everything the hook was invoked with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRequest {
    /// The invoking checkout; repository discovery starts here.
    pub cwd: PathBuf,
    /// Git's first argument; `origin` when Git gave none.
    pub remote_name: String,
    /// Git's second argument; `None` when absent or empty, to be resolved
    /// from the remote's configuration and otherwise kept unknown.
    pub remote_url: Option<String>,
    /// The complete stdin, read before any child starts.
    pub input: Vec<u8>,
    /// `RUSTY_BISCUIT_PRE_PUSH` as set; validated by [`run`] before any work.
    pub mode: Option<OsString>,
    /// `RUSTY_BISCUIT_PRE_PUSH_AREAS` as set.
    pub selection: Option<OsString>,
}

/// The boundaries [`run`] crosses before and around a review session.
/// Production reads the real host; tests inject fakes and assert which
/// boundaries were reached.
pub trait PushServices {
    /// The object format of the repository containing `cwd`, which fixes the
    /// length of every object id Git hands the hook.
    fn object_hash(&mut self, cwd: &Path) -> anyhow::Result<gix::hash::Kind>;

    /// Reports `diagnostic` as it happens, so it interleaves correctly with
    /// the output of the gates.
    fn emit(&mut self, diagnostic: &Diagnostic);

    /// Opens the checkout at `request.cwd` and runs `body` against it. Every
    /// child, temporary tree, and repository handle the session created is
    /// released before this returns, whichever way `body` ended.
    fn session(
        &mut self,
        request: &PushRequest,
        body: &mut dyn FnMut(&mut dyn PushSession) -> anyhow::Result<()>,
    ) -> anyhow::Result<()>;
}

/// One open checkout and everything a reviewed push does to or from it.
///
/// Errors are returned unrendered: the caller decides whether a failure
/// blocks the push or is advisory.
pub trait PushSession {
    /// The checkout's root: the reviewed tree when HEAD is clean, the
    /// directory the gates run in, and the base of a relative evidence root.
    fn root(&self) -> &Path;
    /// Git's remote URL, resolved from configuration when Git gave none;
    /// `None` is an unknown identity.
    fn remote_url(&self) -> Option<&str>;
    fn head(&mut self) -> Result<HeadIdentity, GitError>;
    fn is_dirty(&mut self) -> Result<bool, GitError>;
    /// Proves one Python interpreter for every later [`PushSession::python`]
    /// call and for the gates.
    fn select_python(&mut self) -> Result<(), PythonSelectionError>;
    /// The tip of `branch` on the remote, present locally (cached per branch).
    fn remote_tip(&mut self, branch: &str) -> Result<ObjectId, RemoteTipError>;
    /// Whether `commit` is present locally after at most one fetch of `branch`.
    fn ensure_commit(&mut self, commit: ObjectId, branch: &str) -> Result<bool, RemoteTipError>;
    /// The planner's `--arguments-file` for the committed `base..head` diff.
    fn arguments_file(&mut self, base: ObjectId, head: ObjectId) -> Result<Vec<u8>, GitError>;
    /// `gh pr list` of the open pull requests from `branch` on `repository`
    /// (`OWNER/REPO`); a `NotFound` error means `gh` is not installed.
    fn list_pull_requests(&mut self, repository: &str, branch: &str) -> Result<Completed, ProcessError>;
    /// Materializes `commit`'s tree in a temporary detached worktree and
    /// returns its root. At most one exists at a time.
    fn materialize(&mut self, commit: ObjectId) -> Result<PathBuf, CreateError>;
    /// Removes the tree [`PushSession::materialize`] made, if any.
    fn release_tree(&mut self) -> Result<(), CleanupFailure>;
    /// Runs the selected interpreter with `args` in `cwd`, output captured.
    fn python(&mut self, cwd: &Path, args: &[OsString]) -> Result<Completed, ProcessError>;
    /// The resolved plan rendered for the terminal; display only.
    fn render_plan(&mut self, plan: &[u8]) -> anyhow::Result<String>;
    /// Attaches `receipt` to `head` on the scope notes ref and pushes it.
    fn publish_scope_note(&mut self, receipt: &[u8], head: ObjectId) -> Result<(), String>;
    /// Runs `just pre-push <selection...>` in the checkout with `variables`
    /// set, the selected interpreter handed over through `staging`, and its
    /// output reaching the developer directly.
    fn run_gates(
        &mut self,
        selection: &[String],
        variables: &[(OsString, OsString)],
        staging: &Path,
    ) -> Result<Completed, ProcessError>;
    /// This host's CI environment name; `None` when CI has none for its OS.
    fn environment(&mut self) -> Option<String>;
    /// The home directory (`USERPROFILE` on native Windows, else `HOME`).
    fn home(&self) -> Option<PathBuf>;
    fn variable(&self, name: &str) -> Option<OsString>;
    /// Today's local calendar date, which decides constraint expiry.
    fn today(&self) -> NaiveDate;
    /// The catchable signal this invocation received, if any.
    fn cancelled(&self) -> Option<CancelSignal>;
    fn emit(&mut self, diagnostic: &Diagnostic);
}

/// A Python interpreter proven to run, and the arguments that select it
/// (`py -3` is `py` with prefix `["-3"]`). Every direct Python child uses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonCommand {
    pub executable: PathBuf,
    pub prefix: Vec<OsString>,
}

/// Where one review context's comparison base comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextSource {
    /// A push to `main`, compared against the remote's previous tip.
    MainPush,
    /// An open pull request from the pushed branch. `incoming` marks the
    /// state where the same push's update of the base branch lands first.
    PullRequest {
        number: u64,
        base_branch: String,
        incoming: bool,
    },
    /// No open pull request (or a remote that is not GitHub): the plan a pull
    /// request into the remote's `main` would get. `incoming` marks the state
    /// where this push's own update of `main` lands first.
    Provisional { incoming: bool },
}

/// One comparison the review plans and checks for one reviewed update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewContext {
    /// Index of the update in the parsed list.
    pub update: usize,
    /// The comparison base; null means full scope.
    pub base: ObjectId,
    pub event: TriggerEvent,
    pub source: ContextSource,
}

/// Why a run publishes no validation receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WithholdReason {
    /// The checkout had uncommitted changes when the hook started.
    InitialDirty,
    /// The gates left the checkout dirty.
    FinalDirty,
    /// HEAD moved between the review and the end of the gates.
    HeadChanged,
    SelectionOverride,
    /// The push does not carry the checked-out HEAD.
    OutgoingHeadMissing,
    /// HEAD's first context has no nonzero comparison base.
    NoReviewedComparison,
    NoLocalPlan,
    EmptyManifest,
    /// A process outside the gates' tree still held their output.
    Incomplete,
    Interrupted,
    UnsupportedOs,
    ReportRetentionFailed,
}

impl fmt::Display for WithholdReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            WithholdReason::InitialDirty => {
                "the checkout had uncommitted changes when the push started, so the run is not this tree's"
            }
            WithholdReason::FinalDirty => {
                "the checkout has uncommitted changes after the gates, so the run is not this tree's"
            }
            WithholdReason::HeadChanged => "HEAD moved while the push was reviewed or validated",
            WithholdReason::SelectionOverride => {
                "RUSTY_BISCUIT_PRE_PUSH_AREAS narrowed the gates, which proves only those packages"
            }
            WithholdReason::OutgoingHeadMissing => {
                "this push does not carry HEAD, and the gates ran on HEAD's tree"
            }
            WithholdReason::NoReviewedComparison => {
                "the gates replanned from the working tree because no reviewed comparison plan exists for HEAD"
            }
            WithholdReason::NoLocalPlan => {
                "the local run wrote no plan, so what it ran cannot be tied to the outgoing tree"
            }
            WithholdReason::EmptyManifest => {
                "the run staged no L1/L2 report: every recordable cell was already covered by prior evidence, or none was planned for this host"
            }
            WithholdReason::Incomplete => {
                "a process outside the gates still held their output when they ended, so the run is not known to be complete"
            }
            WithholdReason::Interrupted => {
                "validation was interrupted, so staged reports do not prove a complete run"
            }
            WithholdReason::UnsupportedOs => "this host's operating system has no CI environment",
            WithholdReason::ReportRetentionFailed => "the staged reports could not be retained",
        })
    }
}

/// Why the push is blocked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockReason {
    InvalidMode(ModeError),
    InvalidSelection(SelectionError),
    MalformedInput(InputError),
    /// An update or context could not be reviewed: its base, tree, plan, or
    /// evidence could not be established. Exit status 2.
    Review { update: Option<String>, problem: String },
    /// A recorded execution constraint refuses a reviewed plan, or the store
    /// or plan it is decided from cannot be read. Exit status 1.
    Constraint { update: String, refusals: Vec<String> },
    /// The hook or a gate was interrupted; nothing is published.
    Interrupted { exit_code: u8 },
    /// `strict` and a gate failed with an ordinary status.
    GateFailed { exit_code: u8 },
}

impl BlockReason {
    /// What to do about a constraint refusal.
    pub const CONSTRAINT_ADVICE: &'static str = "Review the resolved plan with 'just ci-local --plan'. Reuse qualifying evidence for that environment, or remove the constraint file once the restriction no longer applies — never work around it by pushing with --no-verify.";

    fn exit_code(&self) -> u8 {
        match self {
            BlockReason::Review { .. } => 2,
            BlockReason::Interrupted { exit_code } | BlockReason::GateFailed { exit_code } => {
                *exit_code
            }
            BlockReason::InvalidMode(_)
            | BlockReason::InvalidSelection(_)
            | BlockReason::MalformedInput(_)
            | BlockReason::Constraint { .. } => 1,
        }
    }
}

impl fmt::Display for BlockReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockReason::InvalidMode(error) => error.fmt(formatter),
            BlockReason::InvalidSelection(error) => error.fmt(formatter),
            BlockReason::MalformedInput(error) => {
                write!(formatter, "{error}; nothing was reviewed, published, or run")
            }
            BlockReason::Review { update, problem } => {
                match update {
                    Some(update) => write!(formatter, "trigger review failed for {update}: {problem}")?,
                    None => write!(formatter, "trigger review failed: {problem}")?,
                }
                formatter.write_str(
                    "\nThe plan this push would trigger cannot be reviewed against the recorded execution constraints, so the push is blocked.",
                )
            }
            BlockReason::Constraint { update, refusals } => {
                write!(formatter, "a recorded execution constraint refuses {update}:")?;
                for refusal in refusals {
                    write!(formatter, "\n  - {refusal}")?;
                }
                write!(formatter, "\n{}", Self::CONSTRAINT_ADVICE)
            }
            BlockReason::Interrupted { exit_code } => write!(
                formatter,
                "pre-push interrupted (exit {exit_code}); no validation receipt is published"
            ),
            BlockReason::GateFailed { exit_code } => write!(
                formatter,
                "pre-push validation failed (exit {exit_code}) in strict mode. To bypass this hook, use 'git push --no-verify'."
            ),
        }
    }
}

/// Something the hook reports. The order is the order it happened; wording
/// is for people, and only the variants are a contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
    /// `off` was given and now means `scope-only`.
    ModeDeprecated,
    UpdateSkipped { update: RefUpdate, reason: SkipReason },
    /// No update is a branch update, so nothing triggers a run.
    NothingToReview,
    /// Narration of the review and the gates.
    Info(String),
    /// Output a helper wrote to stderr, relayed after it finished.
    HelperOutput(String),
    /// A failure that leaves the push's outcome unchanged: scope or
    /// validation publication, or cleanup of a temporary tree.
    Advisory(String),
    /// Auto mode found no update that starts a CI run, so no gate runs.
    NoGateRuns,
    /// No validation receipt is published, and why.
    Withheld(WithholdReason),
    /// `warn` let an ordinary gate failure through.
    WarnedFailure { exit_code: u8 },
    GatesPassed,
    Blocked(BlockReason),
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Diagnostic::ModeDeprecated => {
                formatter.write_str("RUSTY_BISCUIT_PRE_PUSH=off is deprecated; it now means scope-only.")
            }
            Diagnostic::UpdateSkipped { update, reason } => {
                write!(formatter, "Not reviewing {}: {reason}.", update.label())
            }
            Diagnostic::NothingToReview => formatter.write_str(
                "This push updates no branch, so it triggers no run and there is no plan to review.",
            ),
            Diagnostic::Info(text) | Diagnostic::HelperOutput(text) | Diagnostic::Advisory(text) => {
                formatter.write_str(text)
            }
            Diagnostic::NoGateRuns => formatter.write_str(
                "No gate runs: this push starts no CI run (it updates neither main nor a branch with an open pull request).\nTo test before opening a pull request, use 'just pr' or set RUSTY_BISCUIT_PRE_PUSH=strict.",
            ),
            Diagnostic::Withheld(reason) => write!(formatter, "No validation receipt: {reason}."),
            Diagnostic::WarnedFailure { exit_code } => write!(
                formatter,
                "Pre-push validation failed (exit {exit_code}).\nContinuing push in warn mode. To bypass this hook entirely, use 'git push --no-verify'."
            ),
            Diagnostic::GatesPassed => formatter.write_str("Pre-push validation passed."),
            Diagnostic::Blocked(reason) => write!(formatter, "Push blocked: {reason}"),
        }
    }
}

/// The gates' invocation, as run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateRun {
    /// The mode the gates ran in (`auto` already resolved).
    pub mode: PushMode,
    pub selection: Vec<String>,
    /// The reviewed plan fed through `BISCUIT_CI_PLAN_IN`; `None` replanned.
    pub plan_in: Option<PathBuf>,
    /// The gates' exit code; `None` when they did not exit on their own.
    pub exit_code: Option<i32>,
    /// Cancelled, killed, or exited with 128 or more.
    pub interrupted: bool,
    /// A process outside the gates' tree still held their output.
    pub incomplete: bool,
}

/// The result of one hook invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushOutcome {
    /// The hook's exit status; nonzero blocks the push.
    pub exit_code: u8,
    /// The validated mode, once it was read.
    pub mode: Option<PushMode>,
    /// The parsed update list, once the input was read.
    pub updates: Vec<RefUpdate>,
    /// Every context reviewed or attempted, in review order.
    pub reviews: Vec<ContextReview>,
    pub scope: ScopePublication,
    pub gates: Option<GateRun>,
    /// `None` when no gate ran.
    pub validation: Option<Validation>,
    pub diagnostics: Vec<Diagnostic>,
}

impl PushOutcome {
    fn new() -> Self {
        PushOutcome {
            exit_code: 0,
            mode: None,
            updates: Vec::new(),
            reviews: Vec::new(),
            scope: ScopePublication::NotRecorded,
            gates: None,
            validation: None,
            diagnostics: Vec::new(),
        }
    }
}

/// Where diagnostics go: the outcome, and the reporter as they happen.
struct Report<'a> {
    outcome: &'a mut PushOutcome,
}

impl Report<'_> {
    fn push(&mut self, sink: &mut dyn FnMut(&Diagnostic), diagnostic: Diagnostic) {
        sink(&diagnostic);
        self.outcome.diagnostics.push(diagnostic);
    }

    fn block(&mut self, sink: &mut dyn FnMut(&Diagnostic), reason: BlockReason) {
        self.outcome.exit_code = reason.exit_code();
        self.push(sink, Diagnostic::Blocked(reason));
    }
}

/// Decides one push.
///
/// The mode and selection are validated before discovery, and the complete
/// update list before any review. Every context of every branch update is
/// reviewed before anything is published or any gate runs. An `Err` is a
/// failure of a boundary outside a review (discovery, the session itself),
/// which also blocks the push.
pub fn run(request: PushRequest, services: &mut dyn PushServices) -> anyhow::Result<PushOutcome> {
    let mut outcome = PushOutcome::new();
    let mut report = Report { outcome: &mut outcome };
    let setting = match PushMode::parse(request.mode.as_deref()) {
        Ok(setting) => setting,
        Err(error) => {
            report.block(&mut |d| services.emit(d), BlockReason::InvalidMode(error));
            return Ok(outcome);
        }
    };
    report.outcome.mode = Some(setting.mode);
    if setting.deprecated_off {
        report.push(&mut |d| services.emit(d), Diagnostic::ModeDeprecated);
    }
    let selection = match parse_selection(request.selection.as_deref()) {
        Ok(selection) => selection.unwrap_or_default(),
        Err(error) => {
            report.block(&mut |d| services.emit(d), BlockReason::InvalidSelection(error));
            return Ok(outcome);
        }
    };

    let hash = services.object_hash(&request.cwd)?;
    report.outcome.updates = match parse_updates(&request.input, hash) {
        Ok(updates) => updates,
        Err(error) => {
            report.block(&mut |d| services.emit(d), BlockReason::MalformedInput(error));
            return Ok(outcome);
        }
    };
    let updates = report.outcome.updates.clone();
    for update in &updates {
        if let Some(reason) = update.skip_reason() {
            report.push(
                &mut |d| services.emit(d),
                Diagnostic::UpdateSkipped {
                    update: update.clone(),
                    reason,
                },
            );
        }
    }
    if !updates.iter().any(RefUpdate::is_reviewed) {
        report.push(&mut |d| services.emit(d), Diagnostic::NothingToReview);
        return Ok(outcome);
    }

    services.session(&request, &mut |session| {
        reviewed_push(&request, setting.mode, &selection, &updates, session, &mut outcome)
    })?;
    Ok(outcome)
}

/// Steps 2 to 5 of a push with at least one branch update.
fn reviewed_push(
    request: &PushRequest,
    mode: PushMode,
    selection: &[String],
    updates: &[RefUpdate],
    session: &mut dyn PushSession,
    outcome: &mut PushOutcome,
) -> anyhow::Result<()> {
    let staging = tempfile::Builder::new()
        .prefix("ci-push-")
        .tempdir()
        .map_err(|error| anyhow::anyhow!("cannot create a staging directory: {error}"))?;
    let mut flow = Flow {
        session,
        report: Report { outcome },
    };

    let review = match review::review_push(
        &mut flow,
        &request.remote_name,
        mode,
        updates,
        staging.path(),
    ) {
        Ok(review) => review,
        Err(reason) => {
            flow.block(reason);
            return Ok(());
        }
    };

    evidence::publish_scope(&mut flow, &request.remote_name, &review);
    if let Err(reason) = flow.interruption() {
        flow.block(reason);
        return Ok(());
    }
    let mode = match mode {
        PushMode::Auto if review.starts_run => PushMode::Strict,
        PushMode::Auto => {
            flow.say(Diagnostic::NoGateRuns);
            return Ok(());
        }
        mode => mode,
    };
    if mode == PushMode::ScopeOnly {
        return Ok(());
    }
    let gates = run_gates(&mut flow, mode, selection, &review, staging.path())?;
    let retained = evidence::publish_validation(
        &mut flow,
        &request.remote_name,
        &review,
        &gates,
        staging.path(),
    );
    if !retained {
        let kept = staging.keep();
        flow.say(Diagnostic::Advisory(format!(
            "The run's staged reports remain at {}.",
            kept.join(evidence::REPORTS_DIRECTORY).display()
        )));
    }
    finish(&mut flow, &gates);
    Ok(())
}

/// The session plus the outcome being reported, threaded through every step.
struct Flow<'s, 'o> {
    session: &'s mut dyn PushSession,
    report: Report<'o>,
}

impl Flow<'_, '_> {
    fn say(&mut self, diagnostic: Diagnostic) {
        let session = &mut *self.session;
        self.report.push(&mut |d| session.emit(d), diagnostic);
    }

    fn info(&mut self, text: impl Into<String>) {
        self.say(Diagnostic::Info(text.into()));
    }

    fn block(&mut self, reason: BlockReason) {
        let session = &mut *self.session;
        self.report.block(&mut |d| session.emit(d), reason);
    }

    /// An error once a catchable signal arrived: nothing after it may run.
    fn interruption(&self) -> Result<(), BlockReason> {
        match self.session.cancelled() {
            Some(signal) => Err(BlockReason::Interrupted {
                exit_code: signal.exit_code(),
            }),
            None => Ok(()),
        }
    }

    /// Runs one Python helper. Its stderr is relayed on success and becomes
    /// the failure's detail otherwise, prefixed by the exact command.
    fn helper(&mut self, cwd: &Path, args: &[OsString]) -> Result<Completed, HelperError> {
        let mut command = String::from("python");
        for arg in args {
            command.push(' ');
            command.push_str(&arg.to_string_lossy());
        }
        let result = self.session.python(cwd, args);
        if let Some(signal) = self.session.cancelled() {
            return Err(HelperError::Interrupted(signal.exit_code()));
        }
        let completed = result.map_err(|error| {
            HelperError::failed(format!("`{command}` could not be started: {error}"))
        })?;
        let stderr = String::from_utf8_lossy(&completed.stderr).trim_end().to_owned();
        let status = match completed.termination {
            process::Termination::Exited(status) if status.success() => {
                if !stderr.is_empty() {
                    self.say(Diagnostic::HelperOutput(stderr));
                }
                return Ok(completed);
            }
            process::Termination::Exited(status) => match status.code() {
                Some(code) => format!("exited with status {code}"),
                None => "was killed by a signal".to_owned(),
            },
            process::Termination::TimedOut => "timed out".to_owned(),
            process::Termination::Cancelled(signal) => {
                return Err(HelperError::Interrupted(signal.exit_code()));
            }
        };
        let code = match completed.termination {
            process::Termination::Exited(status) => status.code(),
            _ => None,
        };
        let detail = if stderr.is_empty() {
            format!("`{command}` {status}")
        } else {
            format!("`{command}` {status}:\n{stderr}")
        };
        Err(HelperError::Failed { code, detail })
    }
}

/// Why a Python helper produced no usable result.
enum HelperError {
    /// A catchable signal ended it (or arrived while it ran).
    Interrupted(u8),
    Failed { code: Option<i32>, detail: String },
}

impl HelperError {
    fn failed(detail: String) -> Self {
        HelperError::Failed { code: None, detail }
    }

    /// The block this failure causes in a review step: an interruption, or
    /// the step's own reason built from the failure's detail.
    fn block(self, failed: impl FnOnce(String) -> BlockReason) -> BlockReason {
        match self {
            HelperError::Interrupted(exit_code) => BlockReason::Interrupted { exit_code },
            HelperError::Failed { detail, .. } => failed(detail),
        }
    }
}

fn run_gates(
    flow: &mut Flow<'_, '_>,
    mode: PushMode,
    selection: &[String],
    review: &review::PushReview,
    staging: &Path,
) -> anyhow::Result<GateRun> {
    let reports = staging.join(evidence::REPORTS_DIRECTORY);
    std::fs::create_dir_all(&reports)
        .map_err(|error| anyhow::anyhow!("cannot create {}: {error}", reports.display()))?;
    let mut variables: Vec<(OsString, OsString)> = vec![
        ("BISCUIT_CI_SCOPE_OUT".into(), staging.join(evidence::GATE_SCOPE_FILE).into()),
        ("BISCUIT_CI_PLAN_OUT".into(), staging.join(evidence::GATE_PLAN_FILE).into()),
        ("BISCUIT_CI_REPORTS_OUT".into(), reports.into()),
    ];

    // The reviewed plan is fed to the gates only when the run it describes is
    // the run this checkout can perform: HEAD's committed tree IS the working
    // tree, both at the push's start and now (HEAD unmoved, still clean),
    // and the selection is the plan's. Otherwise `ci-local` replans from the
    // working tree for feedback, and the validation receipt is withheld. An
    // unreadable HEAD or status counts as changed.
    let head_now = flow.session.head().map(|head| head.commit).ok();
    let dirty_now = flow.session.is_dirty().unwrap_or(true);
    let comparison = review.first.as_ref().filter(|first| !first.base.is_null());
    let plan_in = if !selection.is_empty() {
        flow.info(format!(
            "Running pre-push validation for: {}\n(RUSTY_BISCUIT_PRE_PUSH_AREAS narrows the local gates; nothing is published)",
            selection.join(" ")
        ));
        None
    } else if review.initial_dirty {
        flow.info("Running pre-push validation for source-changed packages, replanned from the working tree\n(the checkout has uncommitted changes, so the reviewed plan is not this tree's; nothing is published)");
        None
    } else if head_now != Some(review.head) {
        flow.info("Running pre-push validation for source-changed packages, replanned from the working tree\n(HEAD moved after the review, so the reviewed plan is not this tree's; nothing is published)");
        None
    } else if dirty_now {
        flow.info("Running pre-push validation for source-changed packages, replanned from the working tree\n(the checkout changed after the review, or its status cannot be read, so the reviewed plan is not this tree's; nothing is published)");
        None
    } else if let Some(first) = comparison {
        flow.info(format!(
            "Running pre-push validation from the reviewed plan of {} (base {})\n(cells that qualifying prior evidence already covers are skipped; a cell whose newest evidence is a failure is rerun)",
            short(review.head),
            short(first.base)
        ));
        Some(first.review_plan.clone())
    } else {
        flow.info("Running pre-push validation for source-changed packages, replanned from the working tree\n(no reviewed comparison plan exists for HEAD; nothing is published)");
        None
    };
    if let Some(plan) = &plan_in {
        variables.push((process::PLAN_INPUT_VARIABLE.into(), plan.clone().into()));
    }

    let completed = flow.session.run_gates(selection, &variables, staging);
    // The gates mark a run they abandoned after staging reports; like a
    // signal, that makes the run an interruption in every mode.
    let cancelled = flow.session.cancelled().is_some()
        || staging
            .join(evidence::REPORTS_DIRECTORY)
            .join(evidence::INTERRUPTED_MARKER)
            .exists();
    let run = match completed {
        Ok(completed) => GateRun {
            mode,
            selection: selection.to_vec(),
            plan_in,
            exit_code: match completed.termination {
                process::Termination::Exited(status) => status.code(),
                process::Termination::TimedOut | process::Termination::Cancelled(_) => None,
            },
            interrupted: completed.interrupted() || cancelled,
            incomplete: completed.incomplete,
        },
        Err(error) => {
            // A gate that cannot start fails as a shell's missing command does.
            flow.say(Diagnostic::Advisory(format!("The gates could not be started: {error}")));
            GateRun {
                mode,
                selection: selection.to_vec(),
                plan_in,
                exit_code: Some(127),
                interrupted: cancelled,
                incomplete: false,
            }
        }
    };
    flow.report.outcome.gates = Some(run.clone());
    Ok(run)
}

/// The exit status and final words once the gates ended and evidence was
/// decided. An interruption blocks in every mode; `warn` lets only an
/// ordinary failure through.
fn finish(flow: &mut Flow<'_, '_>, run: &GateRun) {
    if run.interrupted || flow.session.cancelled().is_some() {
        let exit_code = flow
            .session
            .cancelled()
            .map(CancelSignal::exit_code)
            .or_else(|| {
                run.exit_code
                    .filter(|code| (128..=255).contains(code))
                    .map(|code| code as u8)
            })
            .unwrap_or(130);
        flow.block(BlockReason::Interrupted { exit_code });
        return;
    }
    match run.exit_code {
        Some(0) => flow.say(Diagnostic::GatesPassed),
        code => {
            let exit_code = code.map_or(1, |code| code.clamp(1, 127) as u8);
            if run.mode == PushMode::Warn {
                flow.say(Diagnostic::WarnedFailure { exit_code });
            } else {
                flow.block(BlockReason::GateFailed { exit_code });
            }
        }
    }
}

#[cfg(test)]
mod tests;
