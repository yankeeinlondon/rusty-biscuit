use super::*;
use std::sync::mpsc;

/// Short bounds so no test waits on the production 5 s / 120 s values.
const BUDGET: ReaderBudget = ReaderBudget {
    pipe_cap: Duration::from_millis(20),
    pipe_settle: Duration::from_millis(100),
    drain_limit: Duration::from_secs(10),
};

/// Stands in for a provider parser: its summary echoes the lines it was fed.
struct EchoParser {
    lines: Vec<String>,
}

impl SemanticStreamParser for EchoParser {
    fn feed_line(&mut self, line: &str) {
        self.lines.push(line.to_string());
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            assistant_text: self.lines.join("\n"),
            exit_code,
            ..Default::default()
        }
    }
}

fn empty_slot() -> ParserSlot {
    Arc::new(Mutex::new(None))
}

/// Feed `lines` through `progress` the way the stdout reader does, then hand
/// the parser back through `slot`.
fn feed_and_hand_back(progress: &ReaderProgress, slot: &ParserSlot, lines: Vec<String>) {
    let mut parser: Box<dyn SemanticStreamParser> = Box::new(EchoParser { lines: Vec::new() });
    for line in progress.track(lines.into_iter()) {
        parser.feed_line(&line);
    }
    *slot.lock().unwrap() = Some(parser);
}

fn wait_until(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "the reader never reached the expected state"
        );
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn a_reader_blocked_on_its_pipe_past_the_cap_times_out_rather_than_panicking() {
    let progress = ReaderProgress::default();
    let (held_open, pipe) = mpsc::channel::<String>();
    let reader_progress = progress.clone();
    let handle = thread::spawn(move || for _line in reader_progress.track(pipe.into_iter()) {});

    let outcome = join_reader(handle, &progress, BUDGET, Instant::now());

    assert!(
        matches!(outcome, JoinOutcome::TimedOut(ReaderStall::PipeOpen)),
        "{outcome:?}"
    );
    let (parser, warning) = settle_parser(outcome, &empty_slot(), 0, BUDGET, &ResultSnapshot::default());
    let summary = parser.finish(0);
    assert_eq!(summary.error_kind.as_deref(), Some("stream_reader_timeout"));
    let message = summary.error_message.expect("a timeout explains itself");
    assert!(!message.contains("panicked"), "{message}");
    assert!(
        message.contains("held by a process the agent started"),
        "{message}"
    );
    assert_eq!(warning.as_deref(), Some(message.as_str()));
    drop(held_open);
}

#[test]
fn a_panicking_reader_reports_its_payload() {
    let progress = ReaderProgress::default();
    let reader_progress = progress.clone();
    // The panic is raised while handling a line, so the drain limit applies
    // and the panic hook's slow report cannot be mistaken for a held pipe.
    let handle = thread::spawn(move || {
        if let Some(line) = reader_progress.track(["816"].into_iter()).next() {
            panic!("renderer exploded on line {line}");
        }
    });

    let outcome = join_reader(handle, &progress, BUDGET, Instant::now());

    let (parser, warning) = settle_parser(outcome, &empty_slot(), 0, BUDGET, &ResultSnapshot::default());
    let summary = parser.finish(0);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("parse_failure"));
    assert_eq!(
        summary.error_message.as_deref(),
        Some("Stream parser thread panicked: renderer exploded on line 816")
    );
    assert_eq!(warning, None);
}

#[test]
fn panic_message_reads_static_and_owned_payloads() {
    assert_eq!(panic_message(&"static"), "static");
    assert_eq!(panic_message(&String::from("owned")), "owned");
    assert_eq!(panic_message(&42_u8), "the panic payload was not a string");
}

#[test]
fn a_reader_slow_after_eof_within_the_drain_limit_yields_the_real_summary() {
    let progress = ReaderProgress::default();
    let slot = empty_slot();
    let (reader_progress, reader_slot) = (progress.clone(), Arc::clone(&slot));
    let handle = thread::spawn(move || {
        feed_and_hand_back(
            &reader_progress,
            &reader_slot,
            vec!["final".into(), "answer".into()],
        );
        // The final render, slowed well past the pipe cap.
        thread::sleep(BUDGET.pipe_cap * 5);
    });
    wait_until(|| progress.reached_eof());

    let outcome = join_reader(handle, &progress, BUDGET, Instant::now());

    assert!(matches!(outcome, JoinOutcome::Joined(())), "{outcome:?}");
    let (parser, warning) = settle_parser(outcome, &slot, 0, BUDGET, &ResultSnapshot::default());
    let summary = parser.finish(0);
    assert_eq!(summary.exit_code, 0);
    assert!(!summary.is_error);
    assert_eq!(summary.assistant_text, "final\nanswer");
    assert_eq!(warning, None);
}

/// The 2026-10-06 failure: the reader was stuck writing one line's output
/// before it had read to EOF.
#[test]
fn a_reader_slow_on_a_line_before_eof_gets_the_drain_limit() {
    let progress = ReaderProgress::default();
    let (release, slow_line) = mpsc::channel::<()>();
    let reader_progress = progress.clone();
    let handle = thread::spawn(move || {
        for _line in reader_progress.track(["large final message"].into_iter()) {
            let _ = slow_line.recv();
        }
    });
    wait_until(|| progress.waiting_for().is_none());
    let since = Instant::now();
    thread::sleep(BUDGET.pipe_cap * 3);
    release.send(()).unwrap();

    let outcome = join_reader(handle, &progress, BUDGET, since);

    assert!(matches!(outcome, JoinOutcome::Joined(())), "{outcome:?}");
}

#[test]
fn a_reader_back_on_a_held_pipe_after_a_slow_line_times_out_at_the_pipe_cap() {
    let progress = ReaderProgress::default();
    let (held_open, pipe) = mpsc::channel::<String>();
    held_open.send("slow line".into()).unwrap();
    let reader_progress = progress.clone();
    let handle = thread::spawn(move || {
        for _line in reader_progress.track(pipe.into_iter()) {
            thread::sleep(BUDGET.pipe_cap * 3);
        }
    });
    wait_until(|| progress.waiting_for().is_none());
    let since = Instant::now();

    let outcome = join_reader(handle, &progress, BUDGET, since);

    assert!(
        matches!(outcome, JoinOutcome::TimedOut(ReaderStall::PipeOpen)),
        "{outcome:?}"
    );
    assert!(since.elapsed() < BUDGET.drain_limit);
    drop(held_open);
}

#[test]
fn a_reader_past_the_drain_limit_after_feeding_every_line_keeps_the_real_summary() {
    let budget = ReaderBudget {
        pipe_cap: Duration::from_millis(10),
        pipe_settle: Duration::from_millis(5),
        drain_limit: Duration::from_millis(50),
    };
    let progress = ReaderProgress::default();
    let slot = empty_slot();
    let (release, stuck_render) = mpsc::channel::<()>();
    let (reader_progress, reader_slot) = (progress.clone(), Arc::clone(&slot));
    let handle = thread::spawn(move || {
        feed_and_hand_back(&reader_progress, &reader_slot, vec!["done".into()]);
        let _ = stuck_render.recv();
    });
    wait_until(|| slot.lock().unwrap().is_some());

    let outcome = join_reader(handle, &progress, budget, Instant::now());

    assert!(
        matches!(outcome, JoinOutcome::TimedOut(ReaderStall::Processing)),
        "{outcome:?}"
    );
    let (parser, warning) = settle_parser(outcome, &slot, 0, budget, &ResultSnapshot::default());
    let summary = parser.finish(0);
    assert_eq!(summary.exit_code, 0);
    assert!(!summary.is_error);
    assert_eq!(summary.assistant_text, "done");
    let warning = warning.expect("the stall is reported");
    assert!(warning.contains("result is kept"), "{warning}");
    release.send(()).unwrap();
}

#[test]
fn reader_failure_names_each_outcome_with_the_same_wording_for_every_stream() {
    let joined: JoinOutcome<()> = JoinOutcome::Joined(());
    assert_eq!(reader_failure(ReaderStream::Stdout, &joined, BUDGET), None);

    let panicked: JoinOutcome<()> = JoinOutcome::Panicked("boom".into());
    let failure = reader_failure(ReaderStream::Stderr, &panicked, BUDGET).unwrap();
    assert_eq!(failure.error_kind, "parse_failure");
    assert_eq!(failure.message, "Stream stderr reader thread panicked: boom");

    let non_string: JoinOutcome<()> = JoinOutcome::Panicked(panic_message(&42_u8));
    let failure = reader_failure(ReaderStream::Output, &non_string, BUDGET).unwrap();
    assert_eq!(
        failure.message,
        "Stream parser thread panicked: the panic payload was not a string"
    );

    for (stall, expected) in [
        (ReaderStall::PipeOpen, "pipe is still open"),
        (ReaderStall::Processing, "terminal was not accepting output"),
    ] {
        let timed_out: JoinOutcome<()> = JoinOutcome::TimedOut(stall);
        let failure = reader_failure(ReaderStream::Stdout, &timed_out, BUDGET).unwrap();
        assert_eq!(failure.error_kind, "stream_reader_timeout");
        assert!(failure.message.contains("agent's stdout"), "{}", failure.message);
        assert!(failure.message.contains(expected), "{}", failure.message);
    }
}

#[test]
fn a_reader_that_finished_is_joined_even_when_the_clock_ran_out_long_ago() {
    let progress = ReaderProgress::default();
    let reader_progress = progress.clone();
    let handle = thread::spawn(move || for _line in reader_progress.track(["x"].into_iter()) {});
    wait_until(|| progress.reached_eof());
    while !handle.is_finished() {
        thread::sleep(Duration::from_millis(1));
    }
    let long_ago = Instant::now()
        .checked_sub(Duration::from_secs(3600))
        .unwrap_or_else(Instant::now);

    let outcome = join_reader(handle, &progress, BUDGET, long_ago);

    assert!(matches!(outcome, JoinOutcome::Joined(())), "{outcome:?}");
}

#[test]
fn the_warning_line_carries_the_message_through_the_status_component() {
    let term = Terminal::default();
    let line = reader_warning_line("output may be incomplete", &term);
    assert!(line.contains("output may be incomplete"), "{line}");
}

/// A reader that timed out while it still held its parser, with the result
/// line already seen by the provider's sink.
fn stalled_with_empty_slot() -> JoinOutcome<()> {
    JoinOutcome::TimedOut(ReaderStall::Processing)
}

#[test]
fn a_stalled_reader_with_a_published_turn_and_exit_zero_keeps_the_result() {
    let snapshot = ResultSnapshot {
        turn_complete: Some(crate::commands::wrap::run_scope::TurnResult {
            provider_status: Some("success".into()),
            duration_ms: Some(1200),
        }),
        terminal_error: None,
    };

    let (parser, warning) =
        settle_parser(stalled_with_empty_slot(), &empty_slot(), 0, BUDGET, &snapshot);

    let summary = parser.finish(0);
    assert!(!summary.is_error);
    assert_eq!(summary.error_kind, None);
    assert_eq!(summary.exit_code, 0);
    assert_eq!(summary.provider_status.as_deref(), Some("success"));
    assert_eq!(summary.duration_ms, Some(1200));
    let warning = warning.expect("the lost output is announced");
    assert!(warning.contains("its output may be incomplete"), "{warning}");
}

#[test]
fn a_stalled_reader_keeps_the_provider_error_it_had_published() {
    let snapshot = ResultSnapshot {
        turn_complete: None,
        terminal_error: Some(("api_remote".into(), "overloaded".into())),
    };

    let (parser, warning) =
        settle_parser(stalled_with_empty_slot(), &empty_slot(), 1, BUDGET, &snapshot);

    let summary = parser.finish(1);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("api_remote"));
    assert_eq!(summary.error_message.as_deref(), Some("overloaded"));
    assert!(warning.is_some());
}

#[test]
fn a_stalled_reader_with_no_published_result_is_an_incomplete_stream() {
    let (parser, warning) = settle_parser(
        stalled_with_empty_slot(),
        &empty_slot(),
        0,
        BUDGET,
        &ResultSnapshot::default(),
    );

    let summary = parser.finish(0);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("stream_reader_timeout"));
    assert!(warning.is_some());
}

#[test]
fn a_published_turn_does_not_turn_a_nonzero_exit_into_success() {
    let snapshot = ResultSnapshot {
        turn_complete: Some(Default::default()),
        terminal_error: None,
    };

    let (parser, _warning) =
        settle_parser(stalled_with_empty_slot(), &empty_slot(), 2, BUDGET, &snapshot);

    let summary = parser.finish(2);
    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("stream_reader_timeout"));
    assert_eq!(summary.exit_code, 2);
}
