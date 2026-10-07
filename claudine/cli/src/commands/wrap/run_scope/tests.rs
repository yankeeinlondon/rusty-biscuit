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
