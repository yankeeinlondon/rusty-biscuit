//! Preflight: the checks a run passes before its lifecycle shells may unlock.
//!
//! A host registers its checks once, in run order, on a [`PreflightPlan`].
//! Each attempt walks them through a [`PreflightRun`]: a check runs the host's
//! own body and keeps the host's own error type, so a host's diagnostics do
//! not change by passing through the plan. A check's [`CheckStage`] says
//! whether its failure belongs to the run: an [`CheckStage::InRun`] failure is
//! a [`HostFailure`] at [`FailureStage::Preflight`], while a
//! [`CheckStage::BeforeRun`] failure ends the invocation before any event
//! fires.
//!
//! The approval-free half of the lifecycle shell audit lives here too:
//! [`shell_walk`] fixes each lifecycle shell site's bytes or typed template,
//! and [`candidates`] collects the commands and sites approval sees and
//! settles each site's mode once approval answers. The prompting approval
//! and the shell policy stay with the host.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::lifecycle::preflight::{CheckSpec, CheckStage, PreflightPlan};
//! use darkmatter::lifecycle::run::{ApprovalEpoch, EntryReason, Preparation};
//!
//! let plan = PreflightPlan::new()
//!     .register(CheckSpec { name: "agent" })
//!     .and_then(|plan| plan.register(CheckSpec { name: "schema" }))
//!     .unwrap();
//! let mut run = plan.begin(EntryReason::Initial);
//! let agent = run.check("agent", CheckStage::BeforeRun, || Ok::<_, String>("claude")).unwrap();
//! assert_eq!(agent, "claude");
//! run.skip("schema");
//! assert_eq!(run.ready(ApprovalEpoch(1)), Preparation::Ready { epoch: ApprovalEpoch(1) });
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Map, Value};
use thiserror::Error;

use crate::markdown::MarkdownError;
use crate::markdown::compose::ContextBuildError;
use crate::markdown::compose::context::merge::CtxMergeError;
use crate::markdown::compose::expression::{BindingError, BindingView, ExpressionError, RuntimeBinding};

use super::context::LifecycleErrorInfo;
use super::run::{ApprovalEpoch, EntryReason, FailureStage, HostFailure, Preparation};

pub mod candidates;
pub mod shell_walk;

pub use candidates::{
    LifecycleShellAudit, collect_lifecycle_shell_audit, lifecycle_document_roots, lifecycle_shell_candidates,
    settle_lifecycle_shell_modes,
};
pub use shell_walk::{LifecycleShellSites, ShellWalkOptions, approval_failure};

/// Where a failed check leaves the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckStage {
    /// Before the run starts: a failure fires no event.
    BeforeRun,
    /// Inside the run: a failure is a [`FailureStage::Preflight`] failure.
    InRun,
}

/// One registered check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckSpec {
    pub name: &'static str,
}

/// A registration the plan refused.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PreflightRegistrationError {
    #[error("preflight check `{0}` is already registered")]
    DuplicateCheck(&'static str),
}

/// A check a run reached out of its registered order.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PreflightOrderError {
    #[error("preflight check `{0}` is not registered")]
    Unregistered(&'static str),
    #[error("preflight check `{check}` runs after `{after}`, which is registered after it")]
    OutOfOrder { check: &'static str, after: &'static str },
}

/// A host's checks, in run order. Built once and shared by every attempt.
#[derive(Clone, Debug, Default)]
pub struct PreflightPlan {
    checks: Vec<CheckSpec>,
}

impl PreflightPlan {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends `spec`, which runs after every check already registered.
    ///
    /// ## Errors
    ///
    /// [`PreflightRegistrationError::DuplicateCheck`] when the name is taken.
    pub fn register(mut self, spec: CheckSpec) -> Result<Self, PreflightRegistrationError> {
        if self.position(spec.name).is_some() {
            return Err(PreflightRegistrationError::DuplicateCheck(spec.name));
        }
        self.checks.push(spec);
        Ok(self)
    }

    /// The registered names, in run order.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.checks.iter().map(|spec| spec.name)
    }

    /// Starts one attempt's walk through the plan.
    pub fn begin(&self, entry: EntryReason) -> PreflightRun<'_> {
        PreflightRun { plan: self, entry, reached: None }
    }

    fn position(&self, name: &str) -> Option<usize> {
        self.checks.iter().position(|spec| spec.name == name)
    }
}

/// One attempt's walk through a [`PreflightPlan`].
///
/// A check may be passed over (a path that has nothing to check need not
/// call it), but never run before one registered ahead of it that already
/// ran.
#[derive(Debug)]
pub struct PreflightRun<'a> {
    plan: &'a PreflightPlan,
    entry: EntryReason,
    /// The index of the last check run or skipped.
    reached: Option<usize>,
}

impl PreflightRun<'_> {
    pub fn entry(&self) -> EntryReason {
        self.entry
    }

    /// Records that `name` is reached next.
    ///
    /// ## Errors
    ///
    /// [`PreflightOrderError`] when `name` is unregistered, or registered at
    /// or before the last check this run reached.
    pub fn advance(&mut self, name: &'static str) -> Result<(), PreflightOrderError> {
        let index = self.plan.position(name).ok_or(PreflightOrderError::Unregistered(name))?;
        if let Some(reached) = self.reached.filter(|&reached| reached >= index) {
            return Err(PreflightOrderError::OutOfOrder { check: name, after: self.plan.checks[reached].name });
        }
        self.reached = Some(index);
        Ok(())
    }

    /// Runs the check `name`'s body.
    ///
    /// An out-of-order call is a host programming error: it fails a debug
    /// assertion, and a release build still runs the body so the host's
    /// behavior never depends on it.
    ///
    /// ## Errors
    ///
    /// The body's error, with the check, its stage, and this attempt's entry.
    pub fn check<T, E>(
        &mut self,
        name: &'static str,
        stage: CheckStage,
        body: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, PreflightFailure<E>> {
        let order = self.advance(name);
        debug_assert!(order.is_ok(), "{}", order.as_ref().err().map(ToString::to_string).unwrap_or_default());
        body().map_err(|error| PreflightFailure { check: name, stage, entry: self.entry, error })
    }

    /// Passes over `name` on a path that has nothing for it to check.
    pub fn skip(&mut self, name: &'static str) {
        let order = self.advance(name);
        debug_assert!(order.is_ok(), "{}", order.as_ref().err().map(ToString::to_string).unwrap_or_default());
    }

    /// Every check passed: the preparation reply for the run.
    pub fn ready(self, epoch: ApprovalEpoch) -> Preparation {
        Preparation::Ready { epoch }
    }
}

/// A check whose body failed, carrying the host's own error.
#[derive(Debug)]
pub struct PreflightFailure<E> {
    pub check: &'static str,
    pub stage: CheckStage,
    pub entry: EntryReason,
    pub error: E,
}

impl<E> PreflightFailure<E> {
    /// The run's view of this failure: a [`FailureStage::Preflight`] failure
    /// for an [`CheckStage::InRun`] check, `None` for a check that fails
    /// before the run starts.
    pub fn host_failure(&self, info: LifecycleErrorInfo) -> Option<HostFailure> {
        (self.stage == CheckStage::InRun).then(|| HostFailure {
            stage: FailureStage::Preflight,
            entry: self.entry,
            error: info,
        })
    }
}

/// The binding catalog the lifecycle shell walk reads, which the host owns.
///
/// The walk fixes shell bytes at preflight, before any event fires, so it
/// reads only early values. Which roots are late, and what an enclosing loop
/// or group supplies, is the host's catalog.
pub trait ShellAuditBindings {
    /// Passive findings for `raw` in the lifecycle shell approval scope, in
    /// authored order: a late or unknown read is one. A text that does not
    /// parse has none; resolving it reports the parse failure.
    fn approval_findings(&self, raw: &str) -> Vec<ExpressionError>;

    /// The view and globals that resolve a shell's bytes at preflight.
    fn approval_bindings(&self) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>);

    /// The view and globals a sequence task's setup reads, with the lexical
    /// group variables and sequence root.
    fn task_setup_bindings(
        &self,
        group: Option<&Map<String, Value>>,
        seq: Option<&Value>,
    ) -> (Arc<BindingView>, HashMap<String, RuntimeBinding<'static>>);

    /// Whether the document is a loop prepared before its first iteration, so
    /// `loop` has no value yet.
    fn loop_pending(&self) -> bool;
}

/// The root of a binding read refused as unavailable in its scope.
pub fn unavailable_root(error: &ExpressionError) -> Option<&str> {
    match error {
        ExpressionError::Binding(binding) => match binding.as_ref() {
            BindingError::Unavailable(read) => Some(&read.root),
            _ => None,
        },
        _ => None,
    }
}

/// A preflight failure of the lifecycle shell walk.
///
/// A host that keeps its own error catalog maps each variant onto its twin;
/// the `Display` text is part of that contract.
#[derive(Debug, Error)]
pub enum PreflightError {
    /// A lifecycle shell command or `set` value could not be fixed at
    /// preflight: a late or missing read, a parse failure, or a command that
    /// must run as its template and is not a valid one.
    #[error(
        "lifecycle shell command at `{property}` failed pre-flight resolution: {message} \
         (raw: `{raw}`) ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleShellResolution {
        source_path: PathBuf,
        /// Authored lifecycle path, e.g. `"start[0].shell.command"`.
        property: String,
        /// The command exactly as authored.
        raw: String,
        message: String,
        #[source]
        source: Option<Box<MarkdownError>>,
    },
    /// Building the early-binding state the walk reads failed.
    #[error(
        "pre-flight shell approval failed: lifecycle shell pre-flight: \
         building early-binding state failed: {source}"
    )]
    PreFlightStateBuildFailed {
        #[source]
        source: CtxMergeError,
    },
    /// The document's expression resolution context could not be built.
    #[error(transparent)]
    ResolutionContext(#[from] ContextBuildError),
}

#[cfg(test)]
mod tests;
