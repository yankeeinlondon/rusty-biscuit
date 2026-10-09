//! Captured-attempt delivery through the output worker, and the output
//! status each attempt branch reports.

use super::*;
use crate::commands::wrap::output_worker::tests::{Gate, wedge_worker};
use crate::commands::wrap::output_worker::Stream;
use crate::commands::wrap::stream_io::StreamOutput;
use std::time::{Duration, Instant};

/// Opens the gate when the test ends, so an abandoned worker is released even
/// when an assertion fails.
struct ReleaseOnDrop(Gate);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.release();
    }
}

fn written(gate: &Gate, stream: Stream) -> String {
    gate.written()
        .into_iter()
        .filter(|(written_to, _)| *written_to == stream)
        .map(|(_, bytes)| String::from_utf8(bytes).unwrap())
        .collect()
}

/// The response goes to stdout and the unreported stderr to stderr, each
/// newline-terminated, and the drain waits until both reached the terminal.
#[test]
fn a_captured_response_and_its_stderr_reach_their_own_streams() {
    let (gate, _entered) = Gate::new(true);
    let output = StreamOutput::with_sink(Box::new(gate.clone()));

    let loss = deliver_capture(
        &output,
        &crate::log::terminal(),
        Some("the answer"),
        Some("provider warning"),
        Instant::now() + Duration::from_secs(10),
    )
    .unwrap();

    assert_eq!(loss, None);
    let stdout = written(&gate, Stream::Stdout);
    assert!(stdout.contains("the answer"), "{stdout:?}");
    assert!(stdout.ends_with('\n'), "{stdout:?}");
    assert_eq!(written(&gate, Stream::Stderr), "provider warning\n");
    let last = gate.written().last().map(|(stream, _)| *stream);
    assert_eq!(last, Some(Stream::Stderr), "stderr follows the response");
}

/// A terminal that never accepts the response holds the attempt only until
/// the deadline, and the next attempt's delivery is refused at once instead of
/// queueing behind the stalled write.
#[test]
fn a_terminal_that_never_accepts_the_captured_response_cannot_hold_the_attempt() {
    let (gate, entered) = Gate::new(false);
    let _release = ReleaseOnDrop(gate.clone());
    let output = StreamOutput::with_sink(Box::new(gate.clone()));
    wedge_worker(&output, &entered);

    let started = Instant::now();
    let loss = deliver_capture(
        &output,
        &crate::log::terminal(),
        Some("the answer"),
        Some("provider warning"),
        Instant::now() + Duration::from_millis(200),
    )
    .unwrap();

    assert!(loss.is_some_and(|loss| loss.stalled), "{loss:?}");
    assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
    assert!(output.loss().stalled);

    let rejected = output.loss().rejected_frames;
    let next = Instant::now();
    let loss = deliver_capture(
        &output,
        &crate::log::terminal(),
        Some("next attempt"),
        None,
        Instant::now() + Duration::from_secs(60),
    )
    .unwrap();
    assert!(loss.is_some_and(|loss| loss.rejected_frames > 0), "{loss:?}");
    assert!(next.elapsed() < Duration::from_secs(5), "{:?}", next.elapsed());
    assert!(output.loss().rejected_frames > rejected);
    assert!(!written(&gate, Stream::Stdout).contains("next attempt"));
}

/// Once the terminal is known stalled, the capture/passthrough budget warnings
/// and the user-interrupt status skip the terminal instead of waiting on the
/// stderr lock the abandoned output-worker write holds.
#[test]
fn attempt_warnings_and_interrupt_status_after_a_stall_do_not_wait_on_the_terminal() {
    crate::terminal_gate::mark_stalled();

    let returned = crate::terminal_gate::tests::returns_while_std_streams_are_locked(|| {
        let term = crate::log::terminal();
        warn_unenforced_budget("step_timeout", &term);
        warn_unenforced_budget("stall_timeout", &term);
        crate::log::message(&crate::output::format_user_interrupt_status());
    });

    assert!(returned, "a harness status write waited on a stalled terminal");
    assert_eq!(crate::terminal_gate::skipped_writes(), 3);
}

/// The attempt outcome a provider-clean run reports, with `output_status`
/// attached as the attempt branches attach it.
fn outcome_with(exit_code: i32, output_status: claudine::harness::OutputStatus) -> claudine::harness::AttemptOutcome {
    let summary = claudine::stream::summary::StreamExecutionSummary {
        exit_code,
        ..Default::default()
    };
    claudine::harness::AttemptOutcome {
        output_status,
        ..claudine::harness::build_attempt_outcome(
            1,
            &summary,
            claudine::harness::ProcessTermination::Completed,
        )
    }
}

/// A structured composition attempt whose reader timed out after the result,
/// and whose terminal then stalled on the trailer, stores both on its one
/// session-end record and reports both on the attempt, which still succeeds.
#[test]
fn a_structured_attempt_reports_its_reader_warning_and_final_delivery_stall() {
    use crate::commands::wrap::policy::session_end_tests::{
        RecordHome, ReleaseOnDrop, publish, reader_timeout_warning, section_stream_over,
        successful_summary,
    };
    let home = RecordHome::new();
    let (gate, entered) = Gate::new(false);
    let _release = ReleaseOnDrop(gate.clone());
    let (output, section_stream) = section_stream_over(&gate);
    wedge_worker(&output, &entered);
    output.set_drain_deadline(Instant::now() + Duration::from_millis(50));
    let warnings = vec![reader_timeout_warning()];

    publish(&successful_summary(), &section_stream, &warnings);
    let status = structured_output_status(&warnings, section_stream.output_loss());

    let record = home.only_session_end();
    let incomplete = &record.extra["output_incomplete"];
    assert_eq!(incomplete["stalled"], true, "{incomplete}");
    assert_eq!(incomplete["reader_warnings"], serde_json::json!(warnings), "{incomplete}");
    assert!(status.delivery_incomplete && !status.partial_capture, "{status:?}");
    assert_eq!(status.warnings, [warnings[0].as_str(), DELIVERY_LOSS_WARNING]);
    let outcome = outcome_with(0, status);
    assert!(claudine::harness::classify_failure(&outcome).is_none());
    assert_eq!(outcome.exit_code, 0);
}

/// A capture pipe a descendant held open yields a partial attempt carrying
/// the reader's warning, which is not a provider failure.
#[test]
fn a_held_open_capture_is_a_partial_attempt_with_its_warning() {
    use crate::commands::wrap::exec::reader_join::{
        JoinOutcome, ReaderBudget, ReaderStall, ReaderStream, reader_failure,
    };
    let warning = reader_failure(
        ReaderStream::Stdout,
        &JoinOutcome::<()>::TimedOut(ReaderStall::PipeOpen),
        ReaderBudget::default(),
    )
    .unwrap()
    .message;

    let status = captured_output_status(true, std::slice::from_ref(&warning), None);

    assert!(status.partial_capture && !status.delivery_incomplete, "{status:?}");
    assert_eq!(status.warnings, [warning]);
    let outcome = outcome_with(0, status);
    assert!(claudine::harness::classify_failure(&outcome).is_none());
    assert!(!outcome.output_status.is_complete());
}

/// A captured response the terminal never accepted is a delivery loss on the
/// attempt, and a capture with nothing to show reports none.
#[test]
fn a_captured_response_the_terminal_refused_is_a_delivery_loss_on_the_attempt() {
    let (gate, entered) = Gate::new(false);
    let _release = ReleaseOnDrop(gate.clone());
    let output = StreamOutput::with_sink(Box::new(gate.clone()));
    wedge_worker(&output, &entered);
    let loss = deliver_capture(
        &output,
        &crate::log::terminal(),
        Some("the answer"),
        None,
        Instant::now() + Duration::from_millis(50),
    )
    .unwrap();

    let status = captured_output_status(false, &[], loss);

    assert!(status.delivery_incomplete && !status.partial_capture, "{status:?}");
    assert_eq!(status.warnings, [DELIVERY_LOSS_WARNING]);
    assert!(captured_output_status(false, &[], None).is_complete());
}

/// An inherited attempt whose forwarder could not write keeps the provider's
/// own exit code and failure classification, with the forwarding problem
/// reported beside it rather than as its cause.
#[test]
fn an_inherited_attempt_keeps_the_native_exit_and_a_distinct_forwarding_diagnostic() {
    let warning =
        "Claudine could not forward the agent's stdout to the terminal: terminal went away".to_string();

    let failed = outcome_with(3, forwarded_output_status(std::slice::from_ref(&warning)));
    let clean = outcome_with(0, forwarded_output_status(std::slice::from_ref(&warning)));

    assert_eq!(failed.exit_code, 3);
    assert_eq!(
        claudine::harness::classify_failure(&failed),
        Some(claudine::harness::FailureEvent::AgentFailure)
    );
    assert!(claudine::harness::classify_failure(&clean).is_none());
    for outcome in [&failed, &clean] {
        assert!(outcome.output_status.delivery_incomplete, "{:?}", outcome.output_status);
        assert!(!outcome.output_status.partial_capture);
        assert_eq!(outcome.output_status.warnings, [warning.as_str()]);
        assert_eq!(outcome.error_kind, None, "forwarding is not the provider's cause");
    }
}

/// A captured response, and separately captured stderr, that the terminal
/// refuses with a write error make the attempt's delivery incomplete; the
/// captured text and the provider's exit stay as they were. The healthy
/// control is complete.
#[test]
fn a_captured_response_or_stderr_refused_with_a_write_error_is_a_delivery_loss() {
    for failing in [Some(Stream::Stdout), Some(Stream::Stderr), None] {
        let gate = match failing {
            Some(stream) => Gate::failing(stream),
            None => Gate::new(true).0,
        };
        let output = StreamOutput::with_sink(Box::new(gate.clone()));

        let loss = deliver_capture(
            &output,
            &crate::log::terminal(),
            Some("the answer"),
            Some("provider warning"),
            Instant::now() + Duration::from_secs(10),
        )
        .unwrap();
        let status = captured_output_status(false, &[], loss);
        let outcome = claudine::harness::AttemptOutcome {
            final_response: "the answer".into(),
            ..outcome_with(0, status)
        };

        assert_eq!(outcome.exit_code, 0, "{failing:?}");
        assert!(claudine::harness::classify_failure(&outcome).is_none(), "{failing:?}");
        assert_eq!(outcome.final_response, "the answer");
        match failing {
            Some(stream) => {
                assert!(loss.is_some_and(|loss| loss.failed_writes > 0), "{stream:?}: {loss:?}");
                assert!(outcome.output_status.delivery_incomplete, "{stream:?}");
                assert!(!outcome.output_status.partial_capture, "{stream:?}");
                assert_eq!(outcome.output_status.warnings, [DELIVERY_LOSS_WARNING], "{stream:?}");
            }
            None => {
                assert_eq!(loss, None);
                assert!(outcome.output_status.is_complete(), "{:?}", outcome.output_status);
            }
        }
    }
}

/// A structured attempt whose answer (stdout) or trailer (stderr) the terminal
/// refused with a write error reports incomplete delivery, and its one
/// session-end record carries `output_incomplete` with the failed write. The
/// healthy control stores no `output_incomplete`. The provider's exit and
/// verdict are unchanged in every case.
#[test]
fn a_structured_attempt_reports_a_write_error_on_either_stream() {
    use crate::commands::wrap::policy::session_end_tests::{
        RecordHome, publish, section_stream_over, successful_summary,
    };
    use std::io::Write as _;
    for failing in [Some(Stream::Stdout), Some(Stream::Stderr), None] {
        let home = RecordHome::new();
        let gate = match failing {
            Some(stream) => Gate::failing(stream),
            None => Gate::new(true).0,
        };
        let (_output, section_stream) = section_stream_over(&gate);
        section_stream.stdout_writer().write_all(b"the answer\n").unwrap();

        publish(&successful_summary(), &section_stream, &[]);
        let status = structured_output_status(&[], section_stream.output_loss());

        let record = home.only_session_end();
        assert_eq!(record.extra["exit_code"], 0, "{failing:?}");
        let outcome = outcome_with(0, status);
        assert!(claudine::harness::classify_failure(&outcome).is_none(), "{failing:?}");
        match failing {
            Some(stream) => {
                let incomplete = &record.extra["output_incomplete"];
                assert!(incomplete["failed_writes"].as_u64() >= Some(1), "{stream:?}: {incomplete}");
                assert_eq!(incomplete["stalled"], false, "{stream:?}: {incomplete}");
                assert!(outcome.output_status.delivery_incomplete, "{stream:?}");
                assert_eq!(outcome.output_status.warnings, [DELIVERY_LOSS_WARNING], "{stream:?}");
            }
            None => {
                assert!(!record.extra.contains_key("output_incomplete"), "{:?}", record.extra);
                assert!(outcome.output_status.is_complete(), "{:?}", outcome.output_status);
            }
        }
    }
}
