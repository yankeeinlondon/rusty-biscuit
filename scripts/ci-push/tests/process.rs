//! The pure parts of process supervision: cancellation state, exit-code
//! mapping, and interruption classification. Spawning tests live in the
//! `ci-push` integration target, which can locate the fixture executable.

use std::process::ExitStatus;

use rstest::rstest;

use crate::ci_push::process::{
    is_interruption, CancelSignal, Cancellation, ChildCommand, Completed, Termination,
};

#[rstest]
#[case(CancelSignal::Interrupt, 130)]
#[case(CancelSignal::Terminate, 143)]
#[case(CancelSignal::Hangup, 129)]
fn cancellation_exit_codes_follow_the_shell_convention(
    #[case] signal: CancelSignal,
    #[case] expected: u8,
) {
    assert_eq!(signal.exit_code(), expected);
}

#[test]
fn a_token_keeps_the_first_signal_and_leaves_other_tokens_alone() {
    let token = Cancellation::new();
    let other = Cancellation::new();
    assert_eq!(token.requested(), None);

    token.cancel(CancelSignal::Terminate);
    token.cancel(CancelSignal::Interrupt);

    assert_eq!(token.requested(), Some(CancelSignal::Terminate));
    assert_eq!(token.clone().requested(), Some(CancelSignal::Terminate));
    assert_eq!(other.requested(), None);
}

#[test]
fn a_cancelled_token_starts_no_child() {
    let token = Cancellation::new();
    token.cancel(CancelSignal::Hangup);

    // A program that cannot exist: spawning it would be a `NotFound` error.
    let completed = ChildCommand::new("ci-push-no-such-program-anywhere")
        .cancellation(token)
        .run()
        .expect("a cancelled run is not an error");

    assert_eq!(
        completed.termination,
        Termination::Cancelled(CancelSignal::Hangup)
    );
    assert!(completed.interrupted());
    assert!(!completed.success());
}

#[test]
fn a_missing_program_is_a_not_found_error() {
    let error = ChildCommand::new("ci-push-no-such-program-anywhere")
        .cancellation(Cancellation::new())
        .run()
        .expect_err("nothing to start");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert!(error
        .to_string()
        .contains("ci-push-no-such-program-anywhere"));
}

#[cfg(unix)]
fn exited(code: i32) -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(code << 8)
}

#[cfg(windows)]
fn exited(code: i32) -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    ExitStatus::from_raw(code as u32)
}

#[rstest]
#[case(0, false)]
#[case(1, false)]
#[case(127, false)]
#[case(128, true)]
#[case(130, true)]
#[case(143, true)]
#[case(255, true)]
fn exits_of_128_or_more_are_interruptions(#[case] code: i32, #[case] interrupted: bool) {
    assert_eq!(is_interruption(exited(code)), interrupted);
}

#[cfg(unix)]
#[test]
fn death_by_signal_is_an_interruption() {
    use std::os::unix::process::ExitStatusExt;
    assert!(is_interruption(ExitStatus::from_raw(libc::SIGKILL)));
}

#[cfg(windows)]
#[test]
fn an_ntstatus_exit_is_an_interruption() {
    // STATUS_CONTROL_C_EXIT, which reads as a negative `i32`.
    assert!(is_interruption(exited(0xC000_013A_u32 as i32)));
}

#[rstest]
#[case(Termination::Exited(exited(0)), true, false)]
#[case(Termination::Exited(exited(1)), false, false)]
#[case(Termination::Exited(exited(130)), false, true)]
#[case(Termination::TimedOut, false, true)]
#[case(Termination::Cancelled(CancelSignal::Interrupt), false, true)]
fn a_completion_classifies_its_termination(
    #[case] termination: Termination,
    #[case] success: bool,
    #[case] interrupted: bool,
) {
    let completed = Completed {
        termination,
        stdout: Vec::new(),
        stderr: Vec::new(),
        incomplete: false,
    };
    assert_eq!(
        (completed.success(), completed.interrupted()),
        (success, interrupted)
    );
}
