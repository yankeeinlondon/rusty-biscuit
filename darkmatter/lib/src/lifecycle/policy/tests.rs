//! Each verdict, route, and selection against the production trace that
//! decides it (`claudine/cli/tests/fixtures/lifecycle_traces/<name>.trace`),
//! or, for a row no trace covers, the Claudine driver that decided it before
//! the kernel did.

use indexmap::IndexMap;

use super::*;
use crate::lifecycle::actions::{LifecyclePath, RetryBackoff};
use crate::lifecycle::executor::ActionLocation;
use LifecycleSignal::{Blocked, Cancelled, Failure, Finalize, Initialize, Loop, Start, Success};

const SUBJECTS: [SubjectKind; 4] =
    [SubjectKind::Document, SubjectKind::RootSequence, SubjectKind::Sequence, SubjectKind::Group];
const CONTAINERS: [SubjectKind; 3] = [SubjectKind::RootSequence, SubjectKind::Sequence, SubjectKind::Group];

fn retry(max_attempts: u32) -> StackControl {
    StackControl::Retry { max_attempts, backoff: RetryBackoff::Fixed, delay: "0s".into(), overlay: IndexMap::new() }
}

fn resume(max_attempts: u32) -> StackControl {
    StackControl::Resume { message: "continue".into(), max_attempts }
}

fn proxy(signal: LifecycleSignal) -> StackControl {
    StackControl::Proxy {
        target: "next.md".into(),
        overlay: IndexMap::new(),
        location: ActionLocation::new(signal, LifecyclePath::root(signal.property_name())),
    }
}

fn defer() -> StackControl {
    StackControl::Defer { delay: "1h".into(), reason: None }
}

fn setup(event: LifecycleSignal, action: RecoveryKind) -> Option<RecoveryVerdict> {
    Some(RecoveryVerdict::Refuse(Refusal::SetupRecoveryUnsupported { event, action }))
}

fn aborted(abort: LifecycleTransitionAbort) -> Option<RecoveryVerdict> {
    Some(RecoveryVerdict::Refuse(Refusal::Abort(abort)))
}

const ADMIT: Option<RecoveryVerdict> = Some(RecoveryVerdict::Admit);
const IGNORE: Option<RecoveryVerdict> = Some(RecoveryVerdict::Ignore);

#[test]
fn only_recovery_directives_get_a_verdict() {
    let others = [
        StackControl::Stop,
        StackControl::Skip,
        StackControl::Error { reason: None },
        StackControl::Break { reason: None, code: 0 },
    ];
    for subject in SUBJECTS {
        for control in &others {
            assert_eq!(recovery_verdict(subject, Start, control), None, "{subject:?} {control:?}");
        }
    }
}

/// Traces `retry-in-initialize`, `retry-in-initialize-root-sequence`, and
/// `retry-in-initialize-group-leg`.
#[test]
fn every_subject_refuses_a_retry_in_initialize() {
    for subject in SUBJECTS {
        assert_eq!(recovery_verdict(subject, Initialize, &retry(1)), setup(Initialize, RecoveryKind::Retry), "{subject:?}");
    }
}

/// Trace `resume-in-initialize`; a container has no session either.
#[test]
fn every_subject_refuses_a_resume_in_initialize_for_want_of_a_session() {
    for subject in SUBJECTS {
        assert_eq!(
            recovery_verdict(subject, Initialize, &resume(1)),
            aborted(LifecycleTransitionAbort::ResumeWithoutSession),
            "{subject:?}"
        );
    }
}

/// Trace `proxy-in-initialize`.
#[test]
fn every_subject_admits_a_hand_off_in_initialize() {
    for subject in SUBJECTS {
        assert_eq!(recovery_verdict(subject, Initialize, &proxy(Initialize)), ADMIT, "{subject:?}");
    }
}

/// Traces `retry-in-preflight-blocked`, `resume-in-preflight-blocked`, and
/// `defer-in-preflight-blocked`; the hand-off row is the old
/// `preflight_blocked_control_error`'s.
#[test]
fn a_documents_preflight_blocked_refuses_every_recovery() {
    let document = SubjectKind::Document;
    assert_eq!(recovery_verdict(document, Blocked, &retry(1)), setup(Blocked, RecoveryKind::Retry));
    assert_eq!(recovery_verdict(document, Blocked, &resume(1)), aborted(LifecycleTransitionAbort::ResumeWithoutSession));
    assert_eq!(recovery_verdict(document, Blocked, &proxy(Blocked)), setup(Blocked, RecoveryKind::Proxy));
    assert_eq!(recovery_verdict(document, Blocked, &defer()), aborted(LifecycleTransitionAbort::DeferredExecutionUnsupported));
}

/// No trace: the old `preflight_blocked_control_error` decided the budget
/// first, so a directive asking for no attempt kept the blocking error. A
/// container's `blocked` refuses it all the same.
#[test]
fn a_documents_preflight_blocked_ignores_a_directive_asking_no_attempt() {
    assert_eq!(recovery_verdict(SubjectKind::Document, Blocked, &retry(0)), IGNORE);
    assert_eq!(recovery_verdict(SubjectKind::Document, Blocked, &resume(0)), IGNORE);
    for subject in CONTAINERS {
        assert_eq!(recovery_verdict(subject, Blocked, &retry(0)), setup(Blocked, RecoveryKind::Retry), "{subject:?}");
    }
}

/// Traces `retry-in-start`, `retry-admitted`, `retry-exhausted`,
/// `resume-admitted`, `resume-no-session` (admitted, then decided without a
/// session), `retry-in-finalize`, `proxy-adopted`, and `proxy-hop-limit`.
#[test]
fn a_documents_start_and_terminal_events_admit_every_recovery() {
    for signal in [Start, Success, Failure, Finalize] {
        for control in [retry(1), resume(1), proxy(signal)] {
            assert_eq!(recovery_verdict(SubjectKind::Document, signal, &control), ADMIT, "{signal:?} {control:?}");
        }
        assert_eq!(
            recovery_verdict(SubjectKind::Document, signal, &defer()),
            aborted(LifecycleTransitionAbort::DeferredExecutionUnsupported),
            "{signal:?}"
        );
    }
}

/// No trace: the old `ContainerRunner::record_outcome`.
#[test]
fn a_container_never_resumes_and_its_finalize_ignores_recovery() {
    for subject in CONTAINERS {
        for signal in [Start, Success, Failure] {
            assert_eq!(recovery_verdict(subject, signal, &retry(1)), ADMIT, "{subject:?} {signal:?}");
            assert_eq!(recovery_verdict(subject, signal, &proxy(signal)), ADMIT, "{subject:?} {signal:?}");
            assert_eq!(
                recovery_verdict(subject, signal, &resume(1)),
                aborted(LifecycleTransitionAbort::ResumeWithoutSession),
                "{subject:?} {signal:?}"
            );
        }
        assert_eq!(recovery_verdict(subject, Blocked, &proxy(Blocked)), setup(Blocked, RecoveryKind::Proxy));
        assert_eq!(recovery_verdict(subject, Blocked, &resume(1)), aborted(LifecycleTransitionAbort::ResumeWithoutSession));
        for control in [retry(1), resume(1), proxy(Finalize), defer()] {
            assert_eq!(recovery_verdict(subject, Finalize, &control), IGNORE, "{subject:?} {control:?}");
        }
    }
}

/// No trace: the old `run_loop_gate`, which every looping subject shares, and
/// `Run`'s rule for `cancelled`.
#[test]
fn the_loop_gate_and_cancelled_refuse_every_recovery() {
    for subject in SUBJECTS {
        for signal in [Loop, Cancelled] {
            assert_eq!(recovery_verdict(subject, signal, &retry(1)), setup(signal, RecoveryKind::Retry));
            assert_eq!(recovery_verdict(subject, signal, &resume(1)), setup(signal, RecoveryKind::Resume));
            assert_eq!(recovery_verdict(subject, signal, &proxy(signal)), setup(signal, RecoveryKind::Proxy));
            assert_eq!(recovery_verdict(subject, signal, &defer()), aborted(LifecycleTransitionAbort::DeferredExecutionUnsupported));
        }
    }
}

/// Traces `retry-in-initialize`, `resume-in-initialize`,
/// `resume-in-preflight-blocked`, `defer-in-preflight-blocked`,
/// `proxy-hop-limit-after-launch` (a document) and
/// `retry-in-initialize-root-sequence` (a root sequence): each finalizes only
/// what it owes, reading the error from before the refusal.
#[test]
fn a_document_or_root_sequence_finalizes_only_what_it_owes() {
    let refusals = [
        Refusal::SetupRecoveryUnsupported { event: Initialize, action: RecoveryKind::Retry },
        Refusal::Abort(LifecycleTransitionAbort::ResumeWithoutSession),
        Refusal::Abort(LifecycleTransitionAbort::ProxyBudgetExhausted),
    ];
    for subject in [SubjectKind::Document, SubjectKind::RootSequence] {
        for refusal in &refusals {
            assert_eq!(refusal_route(subject, refusal), RefusalRoute::FinalizeIfOwed, "{subject:?} {refusal:?}");
        }
    }
}

/// Trace `retry-in-initialize-group-leg`: the leg is a nested sequence.
#[test]
fn a_nested_sequence_fails_through_failure_then_finalize() {
    for refusal in [
        Refusal::SetupRecoveryUnsupported { event: Initialize, action: RecoveryKind::Retry },
        Refusal::Abort(LifecycleTransitionAbort::ResumeWithoutSession),
    ] {
        assert_eq!(refusal_route(SubjectKind::Sequence, &refusal), RefusalRoute::FailureThenFinalize);
    }
}

/// No trace: the old group driver failed through `failure` for a setup
/// refusal and only finalized, reading the refusal, for an aborted one.
#[test]
fn a_group_fails_a_setup_refusal_and_finalizes_an_aborted_one() {
    let setup = Refusal::SetupRecoveryUnsupported { event: Blocked, action: RecoveryKind::Proxy };
    assert_eq!(refusal_route(SubjectKind::Group, &setup), RefusalRoute::FailureThenFinalize);
    let aborted = Refusal::Abort(LifecycleTransitionAbort::DeferredExecutionUnsupported);
    assert_eq!(refusal_route(SubjectKind::Group, &aborted), RefusalRoute::FinalizeIfOwedReadingRefusal);
}

/// Traces `blocked-preflight`, `blocked-refused-handoff`,
/// `shell-unlocked-blocked-after-start` (before launch: `blocked`) and
/// `resume-incompatible` (`failure`).
#[test]
fn a_preparation_failure_before_launch_is_blocked_and_any_other_is_failure() {
    let preparing = [FailureStage::Preflight, FailureStage::Setup, FailureStage::HandoffRefused, FailureStage::RecoveryRefused];
    for stage in preparing {
        assert_eq!(failure_event(stage, false), Blocked, "{stage:?}");
        assert_eq!(failure_event(stage, true), Failure, "{stage:?}");
    }
    for stage in [FailureStage::LaunchConstruction, FailureStage::ResumeIncompatible] {
        assert_eq!(failure_event(stage, false), Failure, "{stage:?}");
    }
}

/// Traces `proxy-hop-limit` and `proxy-hop-limit-after-launch`: the origin and
/// 15 adopted hops make 16 documents, and the 16th hand-off is refused.
#[test]
fn the_proxy_chain_counts_its_origin_so_the_sixteenth_hand_off_is_refused() {
    assert!(!proxy_budget_exhausted(1));
    assert!(!proxy_budget_exhausted(MAX_PROXY_HOPS - 1));
    assert!(proxy_budget_exhausted(MAX_PROXY_HOPS));
}

/// Section 8's two recovery modes: under normal recovery a directive is the
/// event's to request, and in every catch-only cleanup mode (fatal
/// evaluation, refusal, cancellation) each recovery verb is refused and named,
/// while a non-recovery control is never a refusal (the run guard's cleanup
/// pass and `error_routing.rs::run_cancelled_prompt_cleanup` before the
/// kernel).
#[test]
fn only_a_cleanup_mode_refuses_a_recovery_and_names_its_verb() {
    let recoveries = [(retry(2), "retry"), (resume(2), "resume"), (proxy(Finalize), "proxy"), (defer(), "defer")];
    let others = [
        StackControl::Stop,
        StackControl::Skip,
        StackControl::Error { reason: None },
        StackControl::Break { reason: None, code: 0 },
    ];
    for (control, _) in &recoveries {
        assert_eq!(refused_recovery(EvaluationMode::NormalRecovery, control), None, "{control:?}");
    }
    for mode in [EvaluationMode::FatalEvaluationCleanup, EvaluationMode::RefusalCleanup, EvaluationMode::CancellationCleanup] {
        for (control, verb) in &recoveries {
            assert_eq!(refused_recovery(mode, control), Some(*verb), "{mode:?} {control:?}");
        }
        for control in &others {
            assert_eq!(refused_recovery(mode, control), None, "{mode:?} {control:?}");
        }
    }
}
