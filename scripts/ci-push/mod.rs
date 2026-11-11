//! The pre-push hook's decisions: which ref updates to review, against which
//! comparison bases, whether a constraint refuses the push, which gates run,
//! and what evidence may be published.
//!
//! `.githooks/pre-push` launches the `ci-push` binary; the binary reads the
//! environment and stdin into a [`PushRequest`] and renders the
//! [`PushOutcome`] of [`run`]. Every boundary `run` crosses (repository
//! discovery, processes, clock, filesystem) is a [`PushServices`] method, so
//! the decisions are tested in-process.

mod documents;
mod host;
mod json;
mod mode;
mod updates;

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

pub use documents::{
    ConstraintCell, ConstraintRecord, DocumentError, PlanCell, PullRequest, ResolvedPlan,
    parse_constraint_plan, parse_constraint_record, parse_pull_requests, parse_resolved_plan,
};
pub use host::HostServices;
pub use mode::{ModeError, ModeSetting, PushMode, SelectionError, parse_selection};
pub use updates::{InputError, RefUpdate, SkipReason, TriggerEvent, parse_updates};

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

/// The boundaries [`run`] crosses. Production reads the real host; tests
/// inject fakes and assert which boundaries were reached.
pub trait PushServices {
    /// The object format of the repository containing `cwd`, which fixes the
    /// length of every object id Git hands the hook.
    fn object_hash(&mut self, cwd: &Path) -> anyhow::Result<gix::hash::Kind>;
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
    /// request into the remote's `main` would get.
    Provisional,
}

/// One comparison the review plans and checks for one reviewed update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewContext {
    /// Index of the update in the parsed list.
    pub update: usize,
    /// The comparison base; null means full scope.
    pub base: gix::ObjectId,
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
    /// HEAD moved while the gates ran.
    HeadChanged,
    SelectionOverride,
    /// The push does not carry the checked-out HEAD.
    OutgoingHeadMissing,
    /// HEAD's first context has no nonzero comparison base.
    NoReviewedComparison,
    NoLocalPlan,
    EmptyManifest,
    Incomplete,
    Interrupted,
    UnsupportedOs,
    ReportRetentionFailed,
}

/// Why the push is blocked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockReason {
    InvalidMode(ModeError),
    InvalidSelection(SelectionError),
    MalformedInput(InputError),
    /// Branch review is not built yet; nothing reviewable may pass unreviewed.
    ReviewUnavailable,
}

impl fmt::Display for BlockReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockReason::InvalidMode(error) => error.fmt(formatter),
            BlockReason::InvalidSelection(error) => error.fmt(formatter),
            BlockReason::MalformedInput(error) => {
                write!(formatter, "{error}; nothing was reviewed, published, or run")
            }
            BlockReason::ReviewUnavailable => formatter
                .write_str("ci-push cannot review branch updates yet; the shell hook still owns the review"),
        }
    }
}

/// Something the hook reports. The order is the order it happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
    /// `off` was given and now means `scope-only`.
    ModeDeprecated,
    UpdateSkipped { update: RefUpdate, reason: SkipReason },
    /// No update is a branch update, so nothing triggers a run.
    NothingToReview,
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
            Diagnostic::Blocked(reason) => write!(formatter, "Push blocked: {reason}"),
        }
    }
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
    pub diagnostics: Vec<Diagnostic>,
}

impl PushOutcome {
    fn blocked(mut self, reason: BlockReason) -> Self {
        self.exit_code = 1;
        self.diagnostics.push(Diagnostic::Blocked(reason));
        self
    }
}

/// Decides one push.
///
/// The mode and selection are validated before discovery, and the complete
/// update list before any review. An `Err` is a failure of a boundary
/// (discovery), which also blocks the push.
pub fn run(request: PushRequest, services: &mut dyn PushServices) -> anyhow::Result<PushOutcome> {
    let mut outcome = PushOutcome {
        exit_code: 0,
        mode: None,
        updates: Vec::new(),
        diagnostics: Vec::new(),
    };
    let setting = match PushMode::parse(request.mode.as_deref()) {
        Ok(setting) => setting,
        Err(error) => return Ok(outcome.blocked(BlockReason::InvalidMode(error))),
    };
    outcome.mode = Some(setting.mode);
    if setting.deprecated_off {
        outcome.diagnostics.push(Diagnostic::ModeDeprecated);
    }
    if let Err(error) = parse_selection(request.selection.as_deref()) {
        return Ok(outcome.blocked(BlockReason::InvalidSelection(error)));
    }

    let hash = services.object_hash(&request.cwd)?;
    outcome.updates = match parse_updates(&request.input, hash) {
        Ok(updates) => updates,
        Err(error) => return Ok(outcome.blocked(BlockReason::MalformedInput(error))),
    };
    for update in &outcome.updates {
        if let Some(reason) = update.skip_reason() {
            outcome.diagnostics.push(Diagnostic::UpdateSkipped {
                update: update.clone(),
                reason,
            });
        }
    }
    if !outcome.updates.iter().any(RefUpdate::is_reviewed) {
        outcome.diagnostics.push(Diagnostic::NothingToReview);
        return Ok(outcome);
    }
    Ok(outcome.blocked(BlockReason::ReviewUnavailable))
}

#[cfg(test)]
mod tests;
