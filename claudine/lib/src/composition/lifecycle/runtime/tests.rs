use super::*;
use crate::composition::lifecycle::actions::RetryBackoff;
use crate::composition::LifecycleErrorInfo;
use std::time::Duration;

fn raised(message: &str) -> LifecycleEventOutcome {
    LifecycleEventOutcome {
        evaluation_error: Some(LifecycleErrorInfo::from_action_failure(
            "evaluation",
            message,
        )),
        ..LifecycleEventOutcome::default()
    }
}

#[test]
fn catch_protocol_origin_matrix_owns_eligibility() {
    let action_error = LifecycleEventOutcome {
        action_error: Some(LifecycleErrorInfo::from_action_failure("stdout", "closed")),
        ..LifecycleEventOutcome::default()
    };
    let cases = [
        (
            "initialize clean",
            LifecycleSignal::Initialize,
            LifecycleCatchState::default(),
            LifecycleEventOutcome::default(),
            None,
        ),
        (
            "initialize evaluation",
            LifecycleSignal::Initialize,
            LifecycleCatchState::default(),
            raised("initialize"),
            Some(LifecycleSignal::Failure),
        ),
        (
            "start action error",
            LifecycleSignal::Start,
            LifecycleCatchState::default(),
            action_error,
            Some(LifecycleSignal::Failure),
        ),
        (
            "initialize explicit error",
            LifecycleSignal::Initialize,
            LifecycleCatchState::default(),
            LifecycleEventOutcome {
                control: Some(StackControl::Error {
                    reason: Some("refused".to_string()),
                }),
                ..LifecycleEventOutcome::default()
            },
            Some(LifecycleSignal::Failure),
        ),
        (
            "failure evaluation",
            LifecycleSignal::Failure,
            LifecycleCatchState {
                terminal_slot: Some(LifecycleSignal::Failure),
                finalize_emitted: false,
            },
            raised("failure"),
            Some(LifecycleSignal::Finalize),
        ),
        (
            "success evaluation",
            LifecycleSignal::Success,
            LifecycleCatchState {
                terminal_slot: Some(LifecycleSignal::Success),
                finalize_emitted: false,
            },
            raised("success"),
            Some(LifecycleSignal::Finalize),
        ),
        (
            "post-finalize loop evaluation",
            LifecycleSignal::Loop,
            LifecycleCatchState {
                terminal_slot: Some(LifecycleSignal::Success),
                finalize_emitted: true,
            },
            raised("loop"),
            None,
        ),
    ];

    for (name, origin, state, outcome, expected) in cases {
        let protocol = LifecycleCatchProtocol::new(origin, state, None, outcome);
        assert_eq!(
            protocol.next_step().map(|step| step.signal),
            expected,
            "catch eligibility changed for {name}"
        );
    }
}

#[test]
fn catch_protocol_origin_matrix_owns_precedence() {
    let mut initialize = LifecycleCatchProtocol::new(
        LifecycleSignal::Initialize,
        LifecycleCatchState::default(),
        None,
        raised("initialize"),
    );
    assert!(initialize.record(LifecycleSignal::Failure, raised("failure")));
    assert!(initialize.record(LifecycleSignal::Finalize, LifecycleEventOutcome::default()));

    let mut start = LifecycleCatchProtocol::new(
        LifecycleSignal::Start,
        LifecycleCatchState::default(),
        None,
        raised("start"),
    );
    assert!(start.record(LifecycleSignal::Failure, LifecycleEventOutcome::default()));
    assert!(start.record(LifecycleSignal::Finalize, raised("finalize")));

    let mut failure = LifecycleCatchProtocol::new(
        LifecycleSignal::Failure,
        LifecycleCatchState {
            terminal_slot: Some(LifecycleSignal::Failure),
            finalize_emitted: false,
        },
        None,
        raised("failure"),
    );
    assert!(failure.record(LifecycleSignal::Finalize, LifecycleEventOutcome::default()));

    let mut success = LifecycleCatchProtocol::new(
        LifecycleSignal::Success,
        LifecycleCatchState {
            terminal_slot: Some(LifecycleSignal::Success),
            finalize_emitted: false,
        },
        None,
        raised("success"),
    );
    assert!(success.record(LifecycleSignal::Finalize, raised("finalize")));

    let loop_gate = LifecycleCatchProtocol::new(
        LifecycleSignal::Loop,
        LifecycleCatchState {
            terminal_slot: Some(LifecycleSignal::Success),
            finalize_emitted: true,
        },
        None,
        raised("loop"),
    );

    let cases = [
        (initialize, LifecycleSignal::Failure),
        (start, LifecycleSignal::Finalize),
        (failure, LifecycleSignal::Failure),
        (success, LifecycleSignal::Finalize),
        (loop_gate, LifecycleSignal::Loop),
    ];
    for (protocol, expected) in cases {
        let result = protocol.finish().expect("protocol complete");
        assert_eq!(result.evaluation_error_signal, Some(expected));
        assert_eq!(result.control, None);
    }
}

#[test]
fn catch_protocol_owns_blocked_failure_finalize_order_and_error_threading() {
    let blocked = raised("blocked");
    let mut protocol = LifecycleCatchProtocol::new(
        LifecycleSignal::Blocked,
        LifecycleCatchState {
            terminal_slot: Some(LifecycleSignal::Blocked),
            finalize_emitted: false,
        },
        Some(LifecycleErrorInfo::from_action_failure("preflight", "original")),
        blocked,
    );
    let failure = protocol.next_step().expect("failure requested");
    assert_eq!(failure.signal, LifecycleSignal::Failure);
    assert_eq!(
        failure.execution,
        LifecycleCatchExecution::RedesignateBlockedAsFailure
    );
    assert_eq!(failure.error.as_ref().map(|error| error.msg.as_str()), Some("blocked"));

    assert!(protocol.record(LifecycleSignal::Failure, raised("failure")));
    let finalize = protocol.next_step().expect("finalize requested");
    assert_eq!(finalize.signal, LifecycleSignal::Finalize);
    assert_eq!(finalize.execution, LifecycleCatchExecution::Record);
    assert_eq!(
        finalize.error.as_ref().map(|error| error.msg.as_str()),
        Some("failure")
    );

    assert!(protocol.record(LifecycleSignal::Finalize, raised("finalize")));
    let result = protocol.finish().expect("protocol complete");
    assert_eq!(result.evaluation_error_signal, Some(LifecycleSignal::Finalize));
    assert_eq!(
        result.evaluation_error.as_ref().map(|error| error.msg.as_str()),
        Some("finalize")
    );
    assert_eq!(
        result.setup_error.as_ref().map(|error| error.msg.as_str()),
        Some("blocked")
    );
    assert_eq!(result.control, None);
}

#[test]
fn catch_protocol_returns_the_setup_error_that_selected_failure() {
    let mut protocol = LifecycleCatchProtocol::new(
        LifecycleSignal::Initialize,
        LifecycleCatchState::default(),
        None,
        LifecycleEventOutcome {
            control: Some(StackControl::Error {
                reason: Some("refused".to_string()),
            }),
            ..LifecycleEventOutcome::default()
        },
    );
    assert!(protocol.record(
        LifecycleSignal::Failure,
        LifecycleEventOutcome::default(),
    ));
    assert!(protocol.record(
        LifecycleSignal::Finalize,
        LifecycleEventOutcome::default(),
    ));
    let result = protocol.finish().expect("protocol complete");
    assert_eq!(
        result.setup_error.as_ref().map(|error| error.msg.as_str()),
        Some("refused")
    );
    assert!(matches!(result.control, Some(StackControl::Error { .. })));
}

#[test]
fn catch_protocol_finalizes_clean_blocked_without_failure() {
    let control = StackControl::Stop;
    let mut protocol = LifecycleCatchProtocol::new(
        LifecycleSignal::Blocked,
        LifecycleCatchState {
            terminal_slot: Some(LifecycleSignal::Blocked),
            finalize_emitted: false,
        },
        Some(LifecycleErrorInfo::from_action_failure("preflight", "original")),
        LifecycleEventOutcome {
            control: Some(control.clone()),
            ..LifecycleEventOutcome::default()
        },
    );
    assert_eq!(
        protocol.next_step().map(|step| step.signal),
        Some(LifecycleSignal::Finalize)
    );
    assert!(protocol.record(LifecycleSignal::Finalize, LifecycleEventOutcome::default()));
    let result = protocol.finish().expect("protocol complete");
    assert_eq!(result.evaluation_error_signal, None);
    assert_eq!(result.setup_error, None);
    assert_eq!(result.control, Some(control));
}

fn transition(
    event: LifecycleSignal,
    outcome: &LifecycleEventOutcome,
    has_prior_error: bool,
    finalize_emitted: bool,
) -> LifecycleTransitionDecision {
    decide_lifecycle_transition(&LifecycleTransitionInput {
        event,
        terminal_slot: matches!(
            event,
            LifecycleSignal::Success | LifecycleSignal::Blocked | LifecycleSignal::Failure
        )
        .then_some(event),
        provider_launched: false,
        has_prior_error,
        outcome,
        has_session: false,
        attempt: 1,
        control_budget: 0,
        proxy_hops_used: 0,
        proxy_target_seen: false,
        finalize_emitted,
    })
}

#[test]
fn transition_error_matrix_covers_every_lifecycle_event() {
    let clean = LifecycleEventOutcome::default();
    let action = LifecycleEventOutcome {
        action_error: Some(LifecycleErrorInfo::from_action_failure("shell", "failed")),
        ..LifecycleEventOutcome::default()
    };
    let evaluation = raised("evaluation");
    let explicit = LifecycleEventOutcome {
        control: Some(StackControl::Error {
            reason: Some("failed".to_string()),
        }),
        ..LifecycleEventOutcome::default()
    };

    struct Case {
        event: LifecycleSignal,
        outcome: &'static str,
        prior: bool,
        finalized: bool,
        expected: LifecycleTransitionDecision,
    }

    let cases = [
        Case {
            event: LifecycleSignal::Initialize,
            outcome: "clean",
            prior: false,
            finalized: false,
            expected: LifecycleTransitionDecision::Continue,
        },
        Case {
            event: LifecycleSignal::Start,
            outcome: "action",
            prior: false,
            finalized: false,
            expected: LifecycleTransitionDecision::CatchFailure {
                error: LifecycleTransitionError::Action,
            },
        },
        Case {
            event: LifecycleSignal::Success,
            outcome: "clean",
            prior: false,
            finalized: false,
            expected: LifecycleTransitionDecision::Finalize { error: None },
        },
        Case {
            event: LifecycleSignal::Blocked,
            outcome: "clean",
            prior: true,
            finalized: true,
            expected: LifecycleTransitionDecision::TerminalFailure {
                error: LifecycleTransitionError::Prior,
            },
        },
        Case {
            event: LifecycleSignal::Failure,
            outcome: "clean",
            prior: true,
            finalized: false,
            expected: LifecycleTransitionDecision::Finalize {
                error: Some(LifecycleTransitionError::Prior),
            },
        },
        Case {
            event: LifecycleSignal::Finalize,
            outcome: "clean",
            prior: false,
            finalized: true,
            expected: LifecycleTransitionDecision::TerminalSuccess,
        },
        Case {
            event: LifecycleSignal::Loop,
            outcome: "clean",
            prior: false,
            finalized: true,
            expected: LifecycleTransitionDecision::Continue,
        },
        Case {
            event: LifecycleSignal::Initialize,
            outcome: "evaluation",
            prior: true,
            finalized: false,
            expected: LifecycleTransitionDecision::CatchFailure {
                error: LifecycleTransitionError::Evaluation,
            },
        },
        Case {
            event: LifecycleSignal::Initialize,
            outcome: "explicit",
            prior: false,
            finalized: false,
            expected: LifecycleTransitionDecision::CatchFailure {
                error: LifecycleTransitionError::ExplicitControl,
            },
        },
        Case {
            event: LifecycleSignal::Success,
            outcome: "evaluation",
            prior: false,
            finalized: false,
            expected: LifecycleTransitionDecision::Finalize {
                error: Some(LifecycleTransitionError::Evaluation),
            },
        },
        Case {
            event: LifecycleSignal::Finalize,
            outcome: "evaluation",
            prior: true,
            finalized: true,
            expected: LifecycleTransitionDecision::Abort(
                LifecycleTransitionAbort::EvaluationAfterFinalize,
            ),
        },
    ];

    for case in cases {
        let outcome = match case.outcome {
            "clean" => &clean,
            "action" => &action,
            "evaluation" => &evaluation,
            "explicit" => &explicit,
            _ => unreachable!(),
        };
        assert_eq!(
            transition(case.event, outcome, case.prior, case.finalized),
            case.expected,
            "transition changed for {:?}/{}",
            case.event,
            case.outcome,
        );
    }
}

/// [`LifecycleCatchProtocol`] and [`decide_lifecycle_transition`] must never
/// disagree about which origins enter the failure catch: both read the single
/// `setup_catch_error` predicate. A second eligibility rule regrown in either
/// place surfaces here as a mismatch rather than as adapters that compensate
/// with their own branches.
#[test]
fn catch_protocol_and_transition_agree_on_setup_catch_eligibility() {
    let outcomes = [
        ("clean", LifecycleEventOutcome::default()),
        ("evaluation", raised("evaluation")),
        (
            "action",
            LifecycleEventOutcome {
                action_error: Some(LifecycleErrorInfo::from_action_failure("shell", "failed")),
                ..LifecycleEventOutcome::default()
            },
        ),
        (
            "explicit",
            LifecycleEventOutcome {
                control: Some(StackControl::Error {
                    reason: Some("refused".to_string()),
                }),
                ..LifecycleEventOutcome::default()
            },
        ),
    ];

    for origin in LifecycleSignal::ALL {
        for (name, outcome) in &outcomes {
            let decided = matches!(
                transition(origin, outcome, false, false),
                LifecycleTransitionDecision::CatchFailure { .. }
            );
            let mut protocol = LifecycleCatchProtocol::new(
                origin,
                LifecycleCatchState::default(),
                None,
                outcome.clone(),
            );
            let requested =
                protocol.next_step().map(|step| step.signal) == Some(LifecycleSignal::Failure);
            assert_eq!(
                decided, requested,
                "{origin:?}/{name}: decide_lifecycle_transition and LifecycleCatchProtocol \
                 disagree on setup-catch eligibility"
            );
            if !requested {
                continue;
            }
            // Every catch the protocol selects must also report the setup error
            // that selected it: that is the value the composition, loop-engine,
            // and harness adapters branch on instead of re-deriving eligibility.
            assert!(protocol.record(LifecycleSignal::Failure, LifecycleEventOutcome::default()));
            assert!(protocol.record(LifecycleSignal::Finalize, LifecycleEventOutcome::default()));
            let result = protocol.finish().expect("protocol complete");
            assert!(
                result.setup_error.is_some(),
                "{origin:?}/{name}: catch selected without a reported setup error"
            );
        }
    }
}

#[test]
fn transition_control_matrix_covers_preflight_and_harness_controls() {
    let cases = [
        (
            "retry before launch",
            StackControl::Retry {
                max_attempts: 1,
                backoff: RetryBackoff::Fixed,
                delay: "0s".to_string(),
            },
            false,
            false,
            1,
            0,
            0,
            false,
            LifecycleTransitionDecision::Reenter(ControlDispatch::Retry {
                delay: Duration::ZERO,
                reenter_preflight: true,
            }),
        ),
        (
            "retry exhausted",
            StackControl::Retry {
                max_attempts: 1,
                backoff: RetryBackoff::Fixed,
                delay: "0s".to_string(),
            },
            true,
            false,
            2,
            2,
            0,
            false,
            LifecycleTransitionDecision::Finalize { error: None },
        ),
        (
            "resume with session",
            StackControl::Resume {
                message: "continue".to_string(),
                max_attempts: 1,
            },
            true,
            true,
            1,
            0,
            0,
            false,
            LifecycleTransitionDecision::Reenter(ControlDispatch::Resume {
                message: "continue".to_string(),
            }),
        ),
        (
            "resume without session",
            StackControl::Resume {
                message: "continue".to_string(),
                max_attempts: 1,
            },
            true,
            false,
            1,
            0,
            0,
            false,
            LifecycleTransitionDecision::Abort(
                LifecycleTransitionAbort::ResumeWithoutSession,
            ),
        ),
        (
            "proxy handoff",
            StackControl::Proxy {
                target: "next.md".to_string(),
                overlay: indexmap::IndexMap::new(),
                location: crate::composition::ActionLocation::new(LifecycleSignal::Failure, 0, 0),
            },
            true,
            false,
            1,
            0,
            1,
            false,
            LifecycleTransitionDecision::ProxyHandoff {
                target: "next.md".to_string(),
            },
        ),
        (
            "proxy budget exhausted",
            StackControl::Proxy {
                target: "next.md".to_string(),
                overlay: indexmap::IndexMap::new(),
                location: crate::composition::ActionLocation::new(LifecycleSignal::Failure, 0, 0),
            },
            true,
            false,
            1,
            0,
            MAX_PROXY_HOPS,
            false,
            LifecycleTransitionDecision::Abort(
                LifecycleTransitionAbort::ProxyBudgetExhausted,
            ),
        ),
        (
            "stop falls through",
            StackControl::Stop,
            true,
            false,
            1,
            0,
            0,
            false,
            LifecycleTransitionDecision::Finalize { error: None },
        ),
        (
            "skip falls through",
            StackControl::Skip,
            false,
            false,
            1,
            0,
            0,
            false,
            LifecycleTransitionDecision::Finalize { error: None },
        ),
        (
            "explicit error fails",
            StackControl::Error {
                reason: Some("failed".to_string()),
            },
            true,
            false,
            1,
            0,
            0,
            false,
            LifecycleTransitionDecision::TerminalFailure {
                error: LifecycleTransitionError::ExplicitControl,
            },
        ),
        (
            "defer unsupported",
            StackControl::Defer {
                delay: "1m".to_string(),
                reason: None,
            },
            false,
            false,
            1,
            0,
            0,
            false,
            LifecycleTransitionDecision::Abort(
                LifecycleTransitionAbort::DeferredExecutionUnsupported,
            ),
        ),
    ];

    for (
        name,
        control,
        launched,
        session,
        attempt,
        budget,
        proxy_hops,
        target_seen,
        expected,
    ) in cases
    {
        let outcome = LifecycleEventOutcome {
            control: Some(control),
            ..LifecycleEventOutcome::default()
        };
        let actual = decide_lifecycle_transition(&LifecycleTransitionInput {
            event: LifecycleSignal::Failure,
            terminal_slot: Some(LifecycleSignal::Failure),
            provider_launched: launched,
            has_prior_error: false,
            outcome: &outcome,
            has_session: session,
            attempt,
            control_budget: budget,
            proxy_hops_used: proxy_hops,
            proxy_target_seen: target_seen,
            finalize_emitted: false,
        });
        assert_eq!(actual, expected, "control transition changed for {name}");
    }
}
