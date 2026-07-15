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
fn blocked_routing_uses_finalize_failure_origin_precedence() {
    let blocked = raised("blocked");
    let failure = raised("failure");
    let finalize = raised("finalize");
    assert_eq!(
        route_blocked_finalize(&blocked, Some(&failure), Some(&finalize))
            .evaluation_error_signal,
        Some(LifecycleSignal::Finalize)
    );
    assert_eq!(
        route_blocked_finalize(&blocked, Some(&failure), None).evaluation_error_signal,
        Some(LifecycleSignal::Failure)
    );
    assert_eq!(
        route_blocked_finalize(&blocked, None, None).evaluation_error_signal,
        Some(LifecycleSignal::Blocked)
    );
}

#[test]
fn phase_1_terminal_routing_matrix_pins_precedence_and_control() {
    let control = StackControl::Stop;
    let clean = LifecycleEventOutcome {
        control: Some(control.clone()),
        ..LifecycleEventOutcome::default()
    };
    let action_error = LifecycleEventOutcome {
        control: Some(control.clone()),
        action_error: Some(LifecycleErrorInfo::from_action_failure("stdout", "closed")),
        ..LifecycleEventOutcome::default()
    };
    let failure_raise = raised("failure");
    let finalize_raise = raised("finalize");

    let cases = [
        (
            "blocked clean",
            route_blocked_finalize(&clean, None, None),
            None,
            Some(control.clone()),
        ),
        (
            "blocked action error keeps originating control",
            route_blocked_finalize(&action_error, None, None),
            None,
            Some(control.clone()),
        ),
        (
            "failure raise supersedes blocked",
            route_blocked_finalize(&clean, Some(&failure_raise), None),
            Some(LifecycleSignal::Failure),
            None,
        ),
        (
            "finalize raise supersedes failure raise",
            route_blocked_finalize(&clean, Some(&failure_raise), Some(&finalize_raise)),
            Some(LifecycleSignal::Finalize),
            None,
        ),
        (
            "failure origin clean",
            route_failure_finalize(&clean, None),
            None,
            Some(control.clone()),
        ),
        (
            "failure finalize raise",
            route_failure_finalize(&clean, Some(&finalize_raise)),
            Some(LifecycleSignal::Finalize),
            None,
        ),
        (
            "loop origin clean",
            route_loop_gate(&clean, None),
            None,
            Some(control.clone()),
        ),
        (
            "loop finalize raise",
            route_loop_gate(&clean, Some(&finalize_raise)),
            Some(LifecycleSignal::Finalize),
            None,
        ),
    ];

    for (name, actual, expected_signal, expected_control) in cases {
        assert_eq!(
            actual.evaluation_error_signal, expected_signal,
            "evaluation-error precedence changed for {name}"
        );
        assert_eq!(
            actual.control, expected_control,
            "control propagation changed for {name}"
        );
    }
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
