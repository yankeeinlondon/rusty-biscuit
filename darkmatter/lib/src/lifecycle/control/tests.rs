//! Control admission: the retry/resume budget is fixed on first firing, and
//! delays degrade to no wait.

use super::*;

fn retry(max_attempts: u32) -> StackControl {
    StackControl::Retry {
        max_attempts,
        backoff: RetryBackoff::Exponential,
        delay: "2s".to_string(),
        overlay: Default::default(),
    }
}

#[test]
fn a_retry_is_admitted_until_its_first_firing_budget_is_spent() {
    let control = retry(2);
    let budget = control_budget_for(1, 2);

    assert_eq!(
        decide_control(&control, 1, 0, false, true),
        ControlDispatch::Retry { delay: Duration::from_secs(2), reenter_preflight: false }
    );
    assert_eq!(
        decide_control(&control, 2, budget, false, true),
        ControlDispatch::Retry { delay: Duration::from_secs(4), reenter_preflight: false }
    );
    assert_eq!(decide_control(&control, 3, budget, false, true), ControlDispatch::Exhausted);
}

#[test]
fn a_resume_needs_both_budget_and_a_session() {
    let control = StackControl::Resume { message: "go on".to_string(), max_attempts: 1 };

    assert_eq!(decide_control(&control, 1, 0, false, true), ControlDispatch::ResumeWithoutSession);
    assert_eq!(
        decide_control(&control, 1, 0, true, true),
        ControlDispatch::Resume { message: "go on".to_string() }
    );
    assert_eq!(decide_control(&control, 2, 2, true, true), ControlDispatch::Exhausted);
}

#[test]
fn a_delay_that_is_empty_zero_or_malformed_is_no_wait() {
    assert_eq!(parse_delay("1.5 min"), Duration::from_secs(90));
    for raw in ["", "0s", "soon", "5ms"] {
        assert_eq!(parse_delay(raw), Duration::ZERO, "{raw:?}");
    }
}
