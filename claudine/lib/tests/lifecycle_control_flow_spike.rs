//! Differential evidence against production APIs; the candidate model imports no Claudine types.
#[path = "../../../darkmatter/features/2026-09-22-lifecycle-events/spike/model.rs"]
mod model;

use claudine::composition::lifecycle::actions::RetryBackoff;
use claudine::composition::lifecycle::control::{ControlDispatch, decide_control};
use claudine::composition::lifecycle::executor::{LifecycleEventOutcome, StackControl};
use claudine::composition::lifecycle::runtime::{LifecycleCatchExecution, LifecycleCatchProtocol, LifecycleCatchState};
use claudine::composition::{LifecycleErrorInfo, LifecycleSignal};
use model::{Catch, Control, Outcome, Recovery, Role};

fn signal(role: Role) -> LifecycleSignal {
    match role {
        Role::Initialize => LifecycleSignal::Initialize, Role::Composed => LifecycleSignal::Start,
        Role::Blocked => LifecycleSignal::Blocked, Role::Success => LifecycleSignal::Success,
        Role::Failure => LifecycleSignal::Failure, Role::Finalize => LifecycleSignal::Finalize,
        Role::Loop => LifecycleSignal::Loop,
        _ => panic!("custom event has no production counterpart"),
    }
}
fn info(code: u8) -> LifecycleErrorInfo {
    LifecycleErrorInfo::from_action_failure("error", code.to_string())
}
fn control(value: Control) -> StackControl {
    match value {
        Control::Retry => StackControl::Retry { max_attempts: 1, backoff: RetryBackoff::Fixed, delay: "0s".into() },
        Control::Resume => StackControl::Resume { message: "continue".into(), max_attempts: 1 },
        Control::Error => StackControl::Error { reason: Some("99".into()) },
        Control::Stop => StackControl::Stop,
        _ => panic!("outside differential control subset"),
    }
}
fn outcome(value: Outcome) -> LifecycleEventOutcome {
    LifecycleEventOutcome { evaluation_error: value.evaluation.map(info), action_error: value.action.map(info),
        control: value.control.map(control) }
}

#[test]
fn differential_catch_traces_preserve_requests_errors_redesignation_and_control() {
    let roles = [Role::Initialize, Role::Composed, Role::Blocked, Role::Success, Role::Failure, Role::Finalize, Role::Loop];
    let outcomes = [Outcome::default(), Outcome::raised(11), Outcome { action: Some(12), ..Outcome::default() },
        Outcome::control(Control::Error), Outcome::control(Control::Retry),
        Outcome { evaluation: Some(13), control: Some(Control::Retry), ..Outcome::default() }];
    let mut cases = 0;
    for role in roles {
        for slot in [None, Some(Role::Blocked), Some(Role::Success), Some(Role::Failure)] {
            for finalized in [false, true] {
                for prior in [None, Some(10)] {
                    for origin in outcomes {
                        for failure in outcomes {
                            for cleanup in outcomes {
                                let mut candidate = Catch::new(role, slot, finalized, prior, origin);
                                let mut production = LifecycleCatchProtocol::new(signal(role), LifecycleCatchState {
                                    terminal_slot: slot.map(signal), finalize_emitted: finalized,
                                }, prior.map(info), outcome(origin));
                                let context = (role, slot, finalized, prior, origin, failure, cleanup);
                                let mut steps = 0;
                                loop {
                                    let actual = production.next_step().cloned();
                                    assert_eq!(candidate.pending.is_some(), actual.is_some(), "{context:?}");
                                    let Some(expected) = candidate.pending else { break };
                                    let actual = actual.unwrap();
                                    assert_eq!(actual.signal, signal(expected.role), "{context:?}");
                                    assert_eq!(actual.error, expected.error.map(info), "{context:?}");
                                    assert_eq!(actual.execution == LifecycleCatchExecution::RedesignateBlockedAsFailure,
                                        expected.redesignate, "{context:?}");
                                    let response = if expected.role == Role::Failure { failure } else { cleanup };
                                    assert!(production.record(actual.signal, outcome(response)));
                                    assert!(candidate.record(expected.role, response));
                                    steps += 1;
                                    assert!(steps <= 2, "unbounded catch {context:?}");
                                }
                                let actual = production.finish().unwrap();
                                assert_eq!(actual.setup_error, candidate.setup_error.map(info), "{context:?}");
                                assert_eq!(actual.evaluation_error_signal, candidate.evaluation.map(|(r, _)| signal(r)), "{context:?}");
                                assert_eq!(actual.evaluation_error, candidate.evaluation.map(|(_, e)| info(e)), "{context:?}");
                                assert_eq!(actual.control, candidate.control().map(control), "{context:?}");
                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 24_192);
}

#[test]
fn differential_retry_resume_admission_preserves_launch_session_and_budget_rules() {
    let mut cases = 0;
    for action in [Control::Retry, Control::Resume] {
        for attempt in [1, 2, 3, u32::MAX] {
            for budget in [0, 1, 2, 3, u32::MAX] {
                for session in [false, true] {
                    for launched in [false, true] {
                        let ceiling = if budget == 0 { attempt.saturating_add(1) } else { budget };
                        let expected = model::recovery(action, attempt, ceiling, session, launched);
                        let actual = match decide_control(&control(action), attempt, budget, session, launched) {
                            ControlDispatch::Retry { reenter_preflight, delay } => {
                                assert!(delay.is_zero());
                                Recovery::Retry { preflight: reenter_preflight }
                            }
                            ControlDispatch::Resume { message } => { assert_eq!(message, "continue"); Recovery::Resume }
                            ControlDispatch::ResumeWithoutSession => Recovery::NoSession,
                            ControlDispatch::Exhausted => Recovery::Exhausted,
                            other => panic!("unexpected {other:?}"),
                        };
                        assert_eq!(actual, expected, "{action:?} attempt={attempt} budget={budget} session={session} launched={launched}");
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 160);
}

#[test]
fn catch_cleanup_controls_are_not_normal_finalize_recovery() {
    let mut production = LifecycleCatchProtocol::new(LifecycleSignal::Start, LifecycleCatchState::default(),
        None, outcome(Outcome::raised(11)));
    assert!(production.record(LifecycleSignal::Failure, outcome(Outcome::control(Control::Retry))));
    assert!(production.record(LifecycleSignal::Finalize, outcome(Outcome::control(Control::Resume))));
    let result = production.finish().unwrap();
    assert_eq!(result.control, None);
    assert_eq!(result.evaluation_error_signal, Some(LifecycleSignal::Start));
}

#[test]
fn production_guard_records_downgrade_and_distinguishes_retry_from_proxy_reset() {
    use claudine::composition::{DefaultLifecycleEmitter, LifecycleConfig, LifecycleRunGuard, LifecycleRuntimeContext};
    use claudine::events::GlobalSettings;
    use claudine::messaging::RuntimeMessagingSettings;
    use biscuit_terminal::terminal::Terminal;

    // Only record methods run; empty configuration also makes panic/drop silent.
    let config = LifecycleConfig::default();
    let settings = GlobalSettings::default();
    let messaging = RuntimeMessagingSettings::default();
    let term = Terminal::default();
    let context = LifecycleRuntimeContext { settings: &settings, messaging: &messaging, term: &term,
        source_path: std::path::Path::new("in-memory.md"), repo_root: None, launch_area: None, context: None };
    let mut guard = LifecycleRunGuard::new(&config, &context, &DefaultLifecycleEmitter);
    assert!(!guard.record_event_emission(LifecycleSignal::Finalize));
    assert!(guard.record_event_emission(LifecycleSignal::Initialize));
    assert!(guard.record_event_emission(LifecycleSignal::Start));
    guard.mark_provider_launched();
    assert!(guard.record_event_emission(LifecycleSignal::Success));
    assert!(guard.redesignate_terminal_to_failure());
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    assert!(!guard.record_event_emission(LifecycleSignal::Failure));
    assert!(guard.record_event_emission(LifecycleSignal::Finalize));
    assert!(!guard.record_event_emission(LifecycleSignal::Finalize));
    guard.reset_for_next_iteration();
    assert!(guard.initialize_emitted());
    assert!(!guard.start_emitted());
    assert!(!guard.provider_launched());
    assert!(!guard.finalize_emitted());
    assert_eq!(guard.terminal_signal(), None);
    assert!(!guard.record_event_emission(LifecycleSignal::Initialize));
    assert!(guard.record_event_emission(LifecycleSignal::Start));
    guard.reset_for_proxy();
    assert!(!guard.initialize_emitted());
    assert!(!guard.start_emitted());
    assert!(guard.record_event_emission(LifecycleSignal::Initialize));
    guard.defuse();
}
