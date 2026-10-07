use super::*;

#[test]
fn fresh_coordinator_treats_stdout_as_newline_terminated() {
    let coord = StreamOutput::new();
    let inner = coord.inner.lock().unwrap();
    assert!(inner.last_stdout_newline);
}

#[test]
fn stdout_writer_tracks_trailing_newline() {
    let coord = StreamOutput::new();
    let mut writer = coord.stdout_writer();

    writer.write_all(b"hello").unwrap();
    assert!(!coord.inner.lock().unwrap().last_stdout_newline);

    writer.write_all(b"\n").unwrap();
    assert!(coord.inner.lock().unwrap().last_stdout_newline);
}

#[test]
fn stdout_writer_tracks_no_newline_in_last_write() {
    let coord = StreamOutput::new();
    let mut writer = coord.stdout_writer();

    writer.write_all(b"first\n").unwrap();
    assert!(coord.inner.lock().unwrap().last_stdout_newline);

    writer.write_all(b"second partial").unwrap();
    assert!(!coord.inner.lock().unwrap().last_stdout_newline);
}

#[test]
fn stdout_writer_flags_newline_when_buffer_contains_embedded_newlines() {
    // Only the final byte matters for cursor state.
    let coord = StreamOutput::new();
    let mut writer = coord.stdout_writer();

    writer.write_all(b"alpha\nbeta").unwrap();
    assert!(!coord.inner.lock().unwrap().last_stdout_newline);
}

#[test]
fn emit_stderr_line_after_newline_terminated_stdout_leaves_flag_unchanged() {
    // When stdout is already on a fresh row, emit_stderr_line must not
    // push a spurious blank line to stdout. We cannot observe stdout
    // bytes directly in a unit test but we can assert the bookkeeping
    // remains consistent.
    let coord = StreamOutput::new();
    let mut writer = coord.stdout_writer();
    writer.write_all(b"done\n").unwrap();
    coord.emit_stderr_line("tool: bash");
    assert!(coord.inner.lock().unwrap().last_stdout_newline);
}

/// Provider status and reasoning reach stderr through `emit_stderr_line`.
/// A sequence task's handle must decorate them, or a task's body carries the
/// bar while its own status lines float unattributed beside it.
#[test]
fn decorated_handle_attributes_status_lines() {
    let buf: TestRecorder = Arc::new(Mutex::new(Vec::new()));
    let coord = StreamOutput::test_recorder(buf.clone()).decorated("│ ".to_string());

    coord.emit_stderr_line("reasoning: weighing options");
    coord.emit_stderr_frames(&["tool: bash".to_string()]);

    let recorded = buf.lock().unwrap();
    assert_eq!(
        recorded
            .iter()
            .map(|(is_stdout, line)| (*is_stdout, line.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (false, "│ reasoning: weighing options"),
            (false, "│ tool: bash"),
        ],
    );
}

/// Rendered status is often a multi-line block; an unprefixed continuation
/// line would read as belonging to whichever task wrote last.
#[test]
fn decorated_handle_attributes_every_line_of_a_block() {
    let buf: TestRecorder = Arc::new(Mutex::new(Vec::new()));
    let coord = StreamOutput::test_recorder(buf.clone()).decorated("│ ".to_string());

    coord.emit_stderr_line("first\nsecond\nthird");

    let recorded = buf.lock().unwrap();
    assert_eq!(recorded[0].1, "│ first\n│ second\n│ third");
}

/// A non-sequence run must be byte-identical to before the seam existed.
#[test]
fn undecorated_handle_leaves_status_untouched() {
    let buf: TestRecorder = Arc::new(Mutex::new(Vec::new()));
    let coord = StreamOutput::test_recorder(buf.clone());

    coord.emit_stderr_line("tool: bash");

    assert_eq!(buf.lock().unwrap()[0].1, "tool: bash");
}

/// The decoration is per-handle, but the cursor is one real thing: a
/// derived handle must serialize against the same state, or two tasks
/// disagree about whether stdout is mid-line.
#[test]
fn decorated_handle_shares_cursor_state_with_its_parent() {
    let coord = StreamOutput::new();
    let decorated = coord.decorated("│ ".to_string());

    let mut writer = coord.stdout_writer();
    writer.write_all(b"partial").unwrap();

    assert!(!decorated.inner.lock().unwrap().last_stdout_newline);
    assert!(Arc::ptr_eq(&coord.inner, &decorated.inner));
}

#[test]
fn emit_stderr_line_marks_stdout_as_newline_terminated_after_injection() {
    // After stdout was left mid-line, emit_stderr_line writes `\n` to
    // stdout; the tracked flag must flip so the next stderr emission
    // does not inject another `\n`.
    let coord = StreamOutput::new();
    let mut writer = coord.stdout_writer();
    writer.write_all(b"partial text").unwrap();
    assert!(!coord.inner.lock().unwrap().last_stdout_newline);
    coord.emit_stderr_line("tool: bash");
    assert!(coord.inner.lock().unwrap().last_stdout_newline);
}

use super::super::output_worker::tests::Gate;
use std::time::Duration;

fn blocked_coordinator() -> (Arc<StreamOutput>, Gate, std::sync::mpsc::Receiver<()>) {
    let (gate, entered) = Gate::new(false);
    (StreamOutput::with_sink(Box::new(gate.clone())), gate, entered)
}

/// Frames reach the sink in the order the cursor bookkeeping assumed: a
/// status line after a partial stdout line is preceded by the newline that
/// puts it on a fresh row.
#[test]
fn queued_frames_keep_cursor_order_for_an_unblocked_sink() {
    let (gate, _entered) = Gate::new(true);
    let coord = StreamOutput::with_sink(Box::new(gate.clone()));
    let mut writer = coord.stdout_writer();

    writer.write_all(b"partial").unwrap();
    coord.emit_stderr_line("tool: bash");
    coord.emit_stdout_line("done");
    assert_eq!(
        coord.drain(Instant::now() + Duration::from_secs(10)),
        Drained::Complete
    );

    let written: Vec<(Stream, String)> = gate
        .written()
        .into_iter()
        .map(|(stream, bytes)| (stream, String::from_utf8(bytes).unwrap()))
        .collect();
    assert_eq!(
        written,
        vec![
            (Stream::Stdout, "partial".to_string()),
            (Stream::Stdout, "\n".to_string()),
            (Stream::Stderr, "tool: bash\n".to_string()),
            (Stream::Stdout, "done\n".to_string()),
        ]
    );
}

/// A permanently blocked terminal must not stall a writer, a status line,
/// a frame group, the trailer wait, or the exit drain.
#[test]
fn a_blocked_terminal_stalls_no_caller() {
    let (coord, gate, entered) = blocked_coordinator();
    let mut writer = coord.stdout_writer();
    writer.write_all(b"first").unwrap();
    entered.recv_timeout(Duration::from_secs(10)).unwrap();

    let started = Instant::now();
    assert_eq!(writer.write(b"more").unwrap(), 4);
    coord.emit_stderr_line("status");
    coord.emit_stderr_frames(&["a".to_string(), "b".to_string()]);
    coord.emit_stdout_frames(&["c".to_string()]);
    coord.emit_stdout_line("d");
    coord.emit_stderr_undecorated("agent stderr");
    assert!(started.elapsed() < Duration::from_secs(5), "an emission blocked");

    coord.set_drain_deadline(Instant::now() + Duration::from_millis(50));
    assert_eq!(coord.drain_final(Duration::from_secs(60)), Drained::Disabled);
    assert!(coord.loss().stalled);

    // The sink is now silent: later emissions return at once and are counted.
    let before = coord.loss().rejected_frames;
    coord.emit_stderr_line("after the cutoff");
    assert_eq!(coord.loss().rejected_frames, before + 1);
    gate.release();
}

/// The final drain consumes the deadline a run set, so a stale clock from
/// one iteration cannot cut off the next iteration's output.
#[test]
fn the_drain_deadline_is_consumed_by_the_final_drain() {
    let (gate, _entered) = Gate::new(true);
    let coord = StreamOutput::with_sink(Box::new(gate.clone()));
    coord.set_drain_deadline(Instant::now() - Duration::from_secs(1));
    assert_eq!(coord.drain_final(Duration::from_secs(10)), Drained::Complete);

    coord.emit_stderr_line("next iteration");
    assert_eq!(coord.drain_final(Duration::from_secs(10)), Drained::Complete);
    assert_eq!(gate.written().len(), 1);
}

/// A reader whose run was closed at cutoff cannot write into the next
/// iteration, even though the terminal is healthy.
#[test]
fn a_closed_run_cannot_write_into_the_next_iteration() {
    let (gate, _entered) = Gate::new(true);
    let coord = StreamOutput::with_sink(Box::new(gate.clone()));
    let scope = run_scope::RunScope::default();

    let reader_scope = scope.clone();
    let reader_coord = coord.clone();
    let (closed_tx, closed_rx) = std::sync::mpsc::channel::<()>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let (emitted_tx, emitted_rx) = std::sync::mpsc::channel::<()>();
    let reader = std::thread::spawn(move || {
        let _guard = reader_scope.enter();
        reader_coord.emit_stderr_line("before cutoff");
        emitted_tx.send(()).unwrap();
        closed_rx.recv().unwrap();
        // Late output of the abandoned reader.
        reader_coord.emit_stderr_line("late");
        reader_coord.stdout_writer().write_all(b"late text").unwrap();
        done_tx.send(()).unwrap();
    });

    emitted_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(
        coord.drain(Instant::now() + Duration::from_secs(10)),
        Drained::Complete
    );
    scope.close();
    closed_tx.send(()).unwrap();
    done_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    reader.join().unwrap();

    // The next iteration writes normally.
    coord.emit_stderr_line("next iteration");
    assert_eq!(
        coord.drain(Instant::now() + Duration::from_secs(10)),
        Drained::Complete
    );

    let lines: Vec<String> = gate
        .written()
        .into_iter()
        .map(|(_, bytes)| String::from_utf8(bytes).unwrap())
        .collect();
    assert_eq!(lines, vec!["before cutoff\n", "next iteration\n"]);
    assert_eq!(coord.loss().rejected_frames, 2);
}

/// A reader blocked behind a slow terminal on a middle line still reaches
/// the final result line, so parsing produces the complete result.
#[test]
fn a_reader_behind_a_blocked_terminal_still_parses_the_unread_result() {
    use super::super::exec::reader_join::{
        JoinOutcome, ParserSlot, ReaderBudget, ReaderProgress, join_reader, settle_parser,
    };
    use claudine::stream::parser::SemanticStreamParser;
    use claudine::stream::summary::StreamExecutionSummary;

    struct RenderingParser {
        out: StdoutWriter,
        text: String,
    }
    impl SemanticStreamParser for RenderingParser {
        fn feed_line(&mut self, line: &str) {
            // Rendering a line writes to the terminal from inside the parser.
            self.out.write_all(line.as_bytes()).unwrap();
            self.text.push_str(line);
        }
        fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
            self.snapshot(exit_code)
        }
        fn snapshot(&self, exit_code: i32) -> StreamExecutionSummary {
            StreamExecutionSummary {
                assistant_text: self.text.clone(),
                exit_code,
                ..Default::default()
            }
        }
    }

    let (coord, gate, entered) = blocked_coordinator();
    let slot: ParserSlot = Arc::new(Mutex::new(None));
    let progress = ReaderProgress::default();
    let reader_slot = Arc::clone(&slot);
    let reader_progress = progress.clone();
    let reader_coord = coord.clone();
    let reader = std::thread::spawn(move || {
        let mut parser: Box<dyn SemanticStreamParser> = Box::new(RenderingParser {
            out: reader_coord.stdout_writer(),
            text: String::new(),
        });
        for line in reader_progress.track(["first ", "middle ", "result"].into_iter()) {
            parser.feed_line(line);
        }
        *reader_slot.lock().unwrap() = Some(parser);
    });

    let budget = ReaderBudget {
        pipe_cap: Duration::from_secs(5),
        pipe_settle: Duration::from_millis(100),
        drain_limit: Duration::from_secs(10),
    };
    let outcome = join_reader(reader, &progress, budget, Instant::now());
    assert!(matches!(outcome, JoinOutcome::Joined(())), "{outcome:?}");
    entered.recv_timeout(Duration::from_secs(10)).unwrap();

    let (parser, warning) = settle_parser(
        outcome,
        &slot,
        0,
        budget,
        &run_scope::ResultSnapshot::default(),
    );
    let summary = parser.finish(0);
    assert_eq!(summary.assistant_text, "first middle result");
    assert_eq!(warning, None);

    // The wrapper still finishes: the blocked terminal costs only its output.
    assert_eq!(
        coord.drain(Instant::now() + Duration::from_millis(50)),
        Drained::Disabled
    );
    gate.release();
}

/// Across several iterations on a stalled terminal, no iteration starts a
/// writer of its own.
#[test]
fn iterations_on_a_stalled_terminal_share_one_writer() {
    let (coord, gate, entered) = blocked_coordinator();
    coord.emit_stderr_line("iteration 0");
    entered.recv_timeout(Duration::from_secs(10)).unwrap();
    for iteration in 1..5 {
        coord.emit_stderr_line(&format!("iteration {iteration}"));
        coord.set_drain_deadline(Instant::now() + Duration::from_millis(20));
        coord.drain_final(Duration::from_secs(60));
    }
    assert_eq!(coord.worker.threads_started(), 1);
    gate.release();
}
