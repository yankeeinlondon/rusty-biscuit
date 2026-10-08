use super::*;
use std::thread;

#[test]
fn a_thread_with_no_scope_is_never_closed() {
    assert!(!current_closed());
    report_terminal_error("api_remote".into(), "overloaded".into());
}

#[test]
fn closing_a_scope_closes_it_for_the_thread_that_entered_it() {
    let scope = RunScope::default();
    let reader_scope = scope.clone();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (closed_tx, closed_rx) = std::sync::mpsc::channel();
    let reader = thread::spawn(move || {
        let _guard = reader_scope.enter();
        assert!(!current_closed());
        entered_tx.send(()).unwrap();
        closed_rx.recv().unwrap();
        current_closed()
    });

    entered_rx.recv().unwrap();
    scope.close();
    closed_tx.send(()).unwrap();

    assert!(reader.join().unwrap(), "the reader sees its run is over");
    assert!(!current_closed(), "the closing thread never entered the scope");
}

#[test]
fn leaving_the_scope_stops_publishing_into_it() {
    let scope = RunScope::default();
    {
        let _guard = scope.enter();
        report_terminal_error("api_remote".into(), "overloaded".into());
    }
    report_terminal_error("late".into(), "after leaving".into());

    let snapshot = scope.snapshot();
    assert_eq!(
        snapshot.terminal_error,
        Some(("api_remote".to_string(), "overloaded".to_string()))
    );
}

/// Records the events it receives, and what the scope held at each.
struct Recorder {
    scope: RunScope,
    seen: std::sync::mpsc::Sender<(&'static str, bool)>,
}

impl SemanticEventSink for Recorder {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        let published = self.scope.snapshot().summary.is_some();
        self.seen.send((event.kind_str(), published)).unwrap();
    }
}

fn turn_complete() -> SemanticEvent {
    SemanticEvent::TurnComplete {
        provider_status: None,
        token_usage: None,
        cost_usd: None,
        duration_ms: None,
        extra: serde_json::Value::Null,
    }
}

/// Emits a completed turn for every line.
struct CompletingParser(DeferredSink<Recorder>);

impl SemanticStreamParser for CompletingParser {
    fn feed_line(&mut self, _line: &str) {
        self.0.on_semantic_event(turn_complete());
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        self.snapshot(exit_code)
    }

    fn snapshot(&self, exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            assistant_text: "answer".into(),
            exit_code,
            ..Default::default()
        }
    }
}

#[test]
fn a_fed_line_publishes_its_summary_before_the_sink_sees_the_completion() {
    let scope = RunScope::default();
    let _guard = scope.enter();
    let (seen, events) = std::sync::mpsc::channel();
    let mut parser = CompletingParser(DeferredSink::new(Recorder { scope: scope.clone(), seen }));

    feed_line(&mut parser, "result");

    assert_eq!(events.try_recv(), Ok(("turn_complete", true)));
    assert_eq!(scope.snapshot().summary.map(|summary| summary.assistant_text).as_deref(), Some("answer"));
}

#[test]
fn outside_feed_line_the_sink_receives_each_event_at_once() {
    let scope = RunScope::default();
    let _guard = scope.enter();
    let (seen, events) = std::sync::mpsc::channel();
    let mut sink = DeferredSink::new(Recorder { scope: scope.clone(), seen });

    sink.on_semantic_event(turn_complete());

    assert_eq!(events.try_recv(), Ok(("turn_complete", false)));
}

#[test]
fn operation_and_time_are_one_coherent_observation() {
    use observation::{Operation, OperationLane};
    let origin = Instant::now();
    let lane = Arc::new(OperationLane::new(origin));
    let writer = lane.clone();
    let handle = thread::spawn(move || {
        for _ in 0..10_000 {
            writer.record(Operation::PipeWait);
            writer.record(Operation::ProviderParse);
        }
    });
    let mut previous = 0;
    while !handle.is_finished() {
        if let Some(snapshot) = lane.snapshot() {
            assert!(matches!(snapshot.operation, Operation::PipeWait | Operation::ProviderParse));
            assert!(snapshot.at_us >= previous);
            assert!(snapshot.at_us <= origin.elapsed().as_micros() as u64);
            previous = snapshot.at_us;
        }
    }
    handle.join().unwrap();
    assert_eq!(lane.snapshot().unwrap().operation, Operation::ProviderParse);
}

#[test]
fn freezing_retention_rejects_late_publication() {
    let scope = RunScope::default();
    scope.retain_answer("original", false, false);
    scope.retain_raw(b"raw\r\n");
    let frozen = scope.freeze();
    let late = scope.clone();
    thread::spawn(move || {
        let _guard = late.enter();
        late.retain_answer("late", true, true);
        late.retain_raw(b"late");
        late.raw_eof();
        report_terminal_error("late".into(), "ignored".into());
        late.observe_stdout(Operation::Eof);
    }).join().unwrap();
    assert_eq!(scope.freeze(), frozen);
    assert_eq!(frozen.retained.response_text.as_deref(), Some("original"));
    assert_eq!(frozen.retained.response_complete, Some(false));
    assert_eq!(frozen.retained.raw_output_complete, Some(false));
    assert!(scope.snapshot().terminal_error.is_none());
}

#[test]
fn retained_data_shapes_and_size_survive_two_round_trips() {
    use retention::INLINE_LIMIT;
    let cases = [None, Some(("", true)), Some(("partial", false)), Some(("answer", true))];
    for answer in cases {
        let scope = RunScope::default();
        if let Some((text, complete)) = answer { scope.retain_answer(text, complete, true); }
        let frozen = scope.freeze();
        assert_eq!(frozen.retained.response_text.as_deref(), answer.map(|a| a.0));
        assert_eq!(frozen.retained.response_complete, answer.map(|a| a.1));
        assert_eq!(frozen.retained.raw_output, None);
        let value = serde_json::to_value(&frozen).unwrap();
        let mut round_trip = value.clone();
        for _ in 0..2 {
            round_trip = serde_json::from_slice(&serde_json::to_vec(&round_trip).unwrap()).unwrap();
        }
        assert_eq!(round_trip, value);
    }
    let scope = RunScope::default();
    scope.retain_answer(&format!("{}é", "\0".repeat(INLINE_LIMIT - 1)), true, true);
    scope.retain_raw(&vec![0; INLINE_LIMIT + 1]);
    scope.raw_eof();
    let frozen = scope.freeze();
    assert_eq!(frozen.retained.response_text.as_ref().unwrap().len(), INLINE_LIMIT - 1);
    assert_eq!(frozen.retained.response_complete, Some(false));
    assert!(frozen.retained.raw_output_truncated);
    assert_eq!(frozen.retained.raw_output_complete, Some(false));
    assert!(serde_json::to_vec(&frozen).unwrap().len() <= 6 * (2 * INLINE_LIMIT) + 64 * 1024);
}

#[test]
fn raw_read_precedes_decoding_and_preserves_delimiters() {
    use std::io::{BufRead, BufReader};
    for bytes in [b"{bad json}\r\n".as_slice(), b"\xff\r\n", b"", b"{}\n\n"] {
        let scope = RunScope::default();
        let reader = retention::RetainingReader::new(std::io::Cursor::new(bytes), scope.clone());
        let _lines: Vec<_> = BufReader::new(reader).lines().collect();
        let frozen = scope.freeze();
        let expected = match std::str::from_utf8(bytes) {
            Ok(text) => serde_json::json!(text),
            Err(_) => serde_json::json!({"encoding": "base64", "data": "/w0K"}),
        };
        assert_eq!(frozen.retained.raw_output, Some(expected));
        assert_eq!(frozen.retained.raw_output_complete, Some(true));
        assert_eq!(frozen.retained.response_text, None);
        assert!(!frozen.verdict_received);
    }
}

#[test]
fn last_message_is_bounded_answer_only_with_honest_completeness() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("last.txt");
    assert!(retention::last_message(&path).is_err());
    for text in ["".to_owned(), "answer".to_owned(), format!("{}é", "x".repeat(retention::INLINE_LIMIT - 1))] {
        std::fs::write(&path, &text).unwrap();
        let (answer, complete) = retention::last_message(&path).unwrap();
        let scope = RunScope::default();
        scope.retain_fallback_answer(&answer, complete);
        let frozen = scope.freeze();
        assert!(answer.len() <= retention::INLINE_LIMIT);
        assert_eq!(frozen.retained.response_complete, Some(text.len() <= retention::INLINE_LIMIT));
        assert!(!frozen.verdict_received);
    }
    std::fs::write(&path, b"\xff").unwrap();
    assert_eq!(retention::last_message(&path).unwrap_err().kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn capture_completeness_requires_eof_flush_and_no_omissions_or_errors() {
    for (eof, flushed, omitted, failed, complete) in [
        (false, false, false, false, false), (true, false, false, false, false),
        (true, true, true, false, false), (true, true, false, true, false),
        (true, true, false, false, true),
    ] {
        let scope = RunScope::default();
        scope.retain_raw(&vec![b'x'; retention::INLINE_LIMIT + 1]);
        scope.capture(CaptureFacts { path: "existing.ndjson".into(), eof, flushed, omitted, failed });
        let frozen = scope.freeze();
        assert_eq!(frozen.retained.raw_output_complete, Some(complete));
        assert!(frozen.retained.raw_output_truncated);
        assert_eq!(frozen.retained.raw_output_path.as_deref(), Some("existing.ndjson"));
    }
}

#[test]
fn every_operation_has_a_stable_serialized_snapshot() {
    use observation::Operation::*;
    let operations = [PipeWait, BytesArrived, RecordProcessing, JsonDecode, SignalObservation,
        ProviderParse, VerdictPublication, AnswerPublication, SummaryConstruction,
        RenderComputation, OutputSubmission, SemanticLogging, HookCallback, LifecycleCallback,
        Eof, JoinStart, JoinEnd, Cutoff, Settlement, StoragePublication, Idle,
        TerminalDelivery, DeliveryComplete, DeliveryFailed];
    let lane = observation::OperationLane::new(Instant::now());
    for operation in operations {
        lane.record(operation);
        let snapshot = lane.snapshot().unwrap();
        assert_eq!(snapshot.operation, operation);
        assert!(serde_json::to_value(snapshot).unwrap()["operation"].is_string());
    }
}

#[test]
fn a_failed_raw_read_leaves_partial_bytes_and_no_answer() {
    use std::io::Read;
    struct FailsAfterData(bool);
    impl Read for FailsAfterData {
        fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
            if self.0 { return Err(std::io::ErrorKind::BrokenPipe.into()); }
            self.0 = true;
            bytes[0] = b'{';
            Ok(1)
        }
    }
    let scope = RunScope::default();
    let mut reader = retention::RetainingReader::new(FailsAfterData(false), scope.clone());
    let mut bytes = [0; 8];
    assert_eq!(reader.read(&mut bytes).unwrap(), 1);
    assert!(reader.read(&mut bytes).is_err());
    let frozen = scope.freeze();
    assert_eq!(frozen.retained.raw_output, Some(serde_json::json!("{")));
    assert_eq!(frozen.retained.raw_output_complete, Some(false));
    assert_eq!(frozen.retained.response_text, None);
}

#[test]
fn freezing_discards_deferred_callbacks_from_the_abandoned_run() {
    let scope = RunScope::default();
    let (events_tx, events_rx) = std::sync::mpsc::channel();
    let _guard = scope.enter();
    let mut sink = DeferredSink::new(Recorder { scope: scope.clone(), seen: events_tx });
    DEFERRED.with(|queue| *queue.borrow_mut() = Some(Vec::new()));
    sink.on_semantic_event(turn_complete());
    let frozen = scope.freeze();
    let next = RunScope::default();
    let _next_guard = next.enter();
    for work in DEFERRED.with(|queue| queue.borrow_mut().take().unwrap()) { work(); }
    assert!(events_rx.try_recv().is_err());
    assert_eq!(scope.freeze(), frozen);
    assert!(!current_closed());
    assert_eq!(next.freeze().retained.response_text, None);
}

#[test]
fn clipped_multibyte_answer_stays_a_prefix_after_later_deltas() {
    let scope = RunScope::default();
    scope.retain_answer(&"x".repeat(retention::INLINE_LIMIT - 1), false, false);
    scope.retain_answer("é", false, false);
    scope.retain_answer("z", true, false);
    let frozen = scope.freeze();
    assert_eq!(frozen.retained.response_text.unwrap(), "x".repeat(retention::INLINE_LIMIT - 1));
    assert_eq!(frozen.retained.response_complete, Some(false));
}

#[test]
fn stderr_callbacks_cannot_replace_the_stdout_operation() {
    let scope = RunScope::default();
    let _stdout = scope.enter();
    observe(Operation::ProviderParse);
    {
        let _stderr = scope.enter_stderr();
        observe(Operation::HookCallback);
    }
    observe(Operation::JsonDecode);
    let frozen = scope.freeze();
    assert_eq!(frozen.stdout.unwrap().operation, Operation::JsonDecode);
    assert_eq!(frozen.stderr.unwrap().operation, Operation::HookCallback);
}

#[test]
fn capture_path_limit_keeps_serialized_detail_within_its_bound() {
    for length in [8 * 1024, 8 * 1024 + 1] {
        let scope = RunScope::default();
        scope.retain_answer(&"\0".repeat(retention::INLINE_LIMIT), true, true);
        scope.retain_raw(&vec![0; retention::INLINE_LIMIT]);
        scope.capture(CaptureFacts { path: "\u{1}".repeat(length), ..Default::default() });
        let snapshot = scope.freeze();
        assert_eq!(snapshot.retained.raw_output_path.is_some(), length <= 8 * 1024);
        assert!(serde_json::to_vec(&snapshot).unwrap().len() <= 6 * (2 * retention::INLINE_LIMIT) + 64 * 1024);
    }
}

#[test]
fn a_raw_prefix_split_inside_utf8_remains_lossless() {
    use base64::Engine;
    let scope = RunScope::default();
    scope.retain_raw(&vec![b'x'; retention::INLINE_LIMIT - 1]);
    scope.retain_raw("é".as_bytes());
    scope.raw_eof();
    let frozen = scope.freeze();
    let raw = frozen.retained.raw_output.unwrap();
    assert_eq!(raw["encoding"], "base64");
    let bytes = base64::engine::general_purpose::STANDARD.decode(raw["data"].as_str().unwrap()).unwrap();
    assert_eq!(bytes.len(), retention::INLINE_LIMIT);
    assert_eq!(bytes.last(), Some(&0xc3));
    assert!(bytes[..bytes.len() - 1].iter().all(|byte| *byte == b'x'));
    assert!(frozen.retained.raw_output_truncated);
    assert_eq!(frozen.retained.raw_output_complete, Some(false));
}
