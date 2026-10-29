//! The lifecycle policy kernel: every decision about which recovery an event
//! admits, how a refused one is routed, which recovery a cleanup refuses,
//! which event a host failure selects, and how the proxy chain is counted.
//!
//! The functions here are pure. [`Run`](super::run::Run),
//! [`ContainerRunner`](super::runtime::ContainerRunner), and a host's own
//! drivers each ask them instead of deciding again. The budget decision of an
//! admitted recovery stays with [`decide_control`](super::control::decide_control)
//! and [`decide_lifecycle_transition`](super::runtime::decide_lifecycle_transition).
//!
//! Production (Claudine's recorded lifecycle traces) is the authority where
//! the subjects differ: a nested sequence routes a refused recovery through
//! `failure` and `finalize`, while a document or a root sequence only runs a
//! `finalize` it already owes.

use super::LifecycleSignal;
use super::control::MAX_PROXY_HOPS;
use super::executor::StackControl;
use super::run::{FailureStage, RecoveryKind};
use super::runtime::{EvaluationMode, LifecycleTransitionAbort};

/// The kind of subject a lifecycle run drives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubjectKind {
    /// A composed document: it launches a provider and may hold a session.
    #[default]
    Document,
    /// The sequence an invocation runs, which owns the process exit.
    RootSequence,
    /// A sequence nested in another container, inline or referenced.
    Sequence,
    /// A group of concurrent legs.
    Group,
}

impl SubjectKind {
    /// Whether the subject runs children rather than a provider, so it has
    /// no session to resume and no `finalize` recovery.
    pub fn is_container(self) -> bool {
        self != Self::Document
    }
}

/// Why an event cannot honor a recovery directive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The event has no attempt to re-enter or document to replace.
    SetupRecoveryUnsupported { event: LifecycleSignal, action: RecoveryKind },
    /// The directive itself cannot be carried out.
    Abort(LifecycleTransitionAbort),
}

/// What an event does with a recovery directive its stack ended in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryVerdict {
    /// Decide it against the budget, the session, and the proxy chain.
    Admit,
    /// It has no effect; the event continues as if it named no directive.
    Ignore,
    /// Refuse it; [`refusal_route`] says which cleanup runs.
    Refuse(Refusal),
}

/// The verdict on `control` in `signal`'s stack of a `subject`; `None` when
/// `control` is not a recovery directive (`retry`, `resume`, `proxy`,
/// `defer`).
///
/// - A container's `finalize` ignores every recovery directive.
/// - Anywhere else `defer` is refused: it has no runtime yet.
/// - The `loop` gate and `cancelled` refuse every recovery: neither has an
///   attempt to re-enter nor a document to replace.
/// - A container never holds a session, so its `resume` is refused.
/// - `initialize` refuses `retry` and `resume` (nothing has launched) and
///   admits `proxy`.
/// - A document's preflight `blocked` refuses every recovery, except that a
///   `retry` or `resume` asking for no attempt is ignored.
/// - A container's `blocked` refuses `retry` and `proxy`.
/// - Every other event admits the directive.
pub fn recovery_verdict(subject: SubjectKind, signal: LifecycleSignal, control: &StackControl) -> Option<RecoveryVerdict> {
    use LifecycleSignal as S;
    let setup = |action| RecoveryVerdict::Refuse(Refusal::SetupRecoveryUnsupported { event: signal, action });
    let abort = |abort| RecoveryVerdict::Refuse(Refusal::Abort(abort));
    let defer = matches!(control, StackControl::Defer { .. });
    let kind = RecoveryKind::of(control);
    if kind.is_none() && !defer {
        return None;
    }
    if subject.is_container() && signal == S::Finalize {
        return Some(RecoveryVerdict::Ignore);
    }
    let Some(kind) = kind else {
        return Some(abort(LifecycleTransitionAbort::DeferredExecutionUnsupported));
    };
    let verdict = match (signal, kind) {
        (S::Loop | S::Cancelled, _) => setup(kind),
        (_, RecoveryKind::Resume) if subject.is_container() => abort(LifecycleTransitionAbort::ResumeWithoutSession),
        (S::Initialize, RecoveryKind::Retry) => setup(kind),
        (S::Initialize, RecoveryKind::Resume) => abort(LifecycleTransitionAbort::ResumeWithoutSession),
        (S::Blocked, RecoveryKind::Retry | RecoveryKind::Resume) if asks_no_attempt(control) && !subject.is_container() => {
            RecoveryVerdict::Ignore
        }
        (S::Blocked, RecoveryKind::Resume) => abort(LifecycleTransitionAbort::ResumeWithoutSession),
        (S::Blocked, _) => setup(kind),
        _ => RecoveryVerdict::Admit,
    };
    Some(verdict)
}

fn asks_no_attempt(control: &StackControl) -> bool {
    matches!(control, StackControl::Retry { max_attempts: 0, .. } | StackControl::Resume { max_attempts: 0, .. })
}

/// Which cleanup a refused recovery runs before the run reports the refusal.
///
/// Each event runs at most once, as the run's accounting admits it, and in
/// [`EvaluationMode::RefusalCleanup`](super::runtime::EvaluationMode::RefusalCleanup).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusalRoute {
    /// `finalize`, only when a terminal event already ran and `finalize` has
    /// not, reading the error selected before the refusal.
    FinalizeIfOwed,
    /// As [`Self::FinalizeIfOwed`], reading the refusal.
    FinalizeIfOwedReadingRefusal,
    /// `failure` (re-designating a `success` or `blocked` that holds the
    /// terminal slot), unless `failure` is the refusing event, then
    /// `finalize`; both read the refusal.
    FailureThenFinalize,
}

/// How a `subject` routes `refusal`.
///
/// A document or a root sequence ends at once, finalizing only what it owes
/// (traces `retry-in-initialize`, `resume-in-preflight-blocked`,
/// `proxy-hop-limit-after-launch`, `retry-in-initialize-root-sequence`). A
/// nested sequence fails through its catch events
/// (`retry-in-initialize-group-leg`). A group fails through them for a setup
/// refusal and only finalizes for an aborted directive.
pub fn refusal_route(subject: SubjectKind, refusal: &Refusal) -> RefusalRoute {
    match (subject, refusal) {
        (SubjectKind::Document | SubjectKind::RootSequence, _) => RefusalRoute::FinalizeIfOwed,
        (SubjectKind::Sequence, _) | (SubjectKind::Group, Refusal::SetupRecoveryUnsupported { .. }) => {
            RefusalRoute::FailureThenFinalize
        }
        (SubjectKind::Group, Refusal::Abort(_)) => RefusalRoute::FinalizeIfOwedReadingRefusal,
    }
}

/// The terminal event a host failure selects: `blocked` for a failure while
/// preparing (preflight, setup, a refused hand-off or recovery) before the
/// operation started, and `failure` otherwise.
pub fn failure_event(stage: FailureStage, launched: bool) -> LifecycleSignal {
    let preparing = matches!(
        stage,
        FailureStage::Preflight | FailureStage::Setup | FailureStage::HandoffRefused | FailureStage::RecoveryRefused
    );
    if preparing && !launched { LifecycleSignal::Blocked } else { LifecycleSignal::Failure }
}

/// Whether a proxy chain of `documents_in_chain` documents may hand off once
/// more.
///
/// The chain counts every document it has run, the origin included, so of
/// [`MAX_PROXY_HOPS`] (16) hand-offs the 16th is refused (traces
/// `proxy-hop-limit`, `proxy-hop-limit-after-launch`).
pub fn proxy_budget_exhausted(documents_in_chain: usize) -> bool {
    documents_in_chain >= MAX_PROXY_HOPS
}

/// The recovery verb `control` requests when `mode` is a cleanup that refuses
/// every recovery (`None` under [`EvaluationMode::NormalRecovery`]).
pub fn refused_recovery(mode: EvaluationMode, control: &StackControl) -> Option<&'static str> {
    if mode == EvaluationMode::NormalRecovery {
        return None;
    }
    match control {
        StackControl::Retry { .. } => Some("retry"),
        StackControl::Resume { .. } => Some("resume"),
        StackControl::Proxy { .. } => Some("proxy"),
        StackControl::Defer { .. } => Some("defer"),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
