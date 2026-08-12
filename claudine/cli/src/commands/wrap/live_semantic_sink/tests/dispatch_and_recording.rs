use super::*;
use biscuit_terminal::discovery::detection::ColorDepth;
use serde_json::json;
use std::sync::Mutex as StdMutex;

fn make_sink(
    captured_lines: Arc<StdMutex<Vec<String>>>,
    captured_dispatch: Arc<StdMutex<Vec<(AgenticEvent, String)>>>,
) -> LiveSemanticSink {
    make_sink_for_provider(
        Provider::Claude,
        Verbosity::Normal,
        captured_lines,
        captured_dispatch,
    )
}

fn make_sink_for_provider(
    provider: Provider,
    verbosity: Verbosity,
    captured_lines: Arc<StdMutex<Vec<String>>>,
    captured_dispatch: Arc<StdMutex<Vec<(AgenticEvent, String)>>>,
) -> LiveSemanticSink {
    let dispatch = {
        let captured = captured_dispatch.clone();
        Box::new(move |event: AgenticEvent, meta: DispatchEventMeta| {
            captured
                .lock()
                .unwrap()
                .push((event, meta.tool_name.unwrap_or_default()));
        })
    };
    let emit = {
        let lines = captured_lines.clone();
        Box::new(move |line: &str| {
            lines.lock().unwrap().push(line.to_string());
        })
    };
    let mut sink = LiveSemanticSink::new(
        provider,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        verbosity,
        Arc::new(Mutex::new(StructuredSummaryDetails::default())),
        dispatch,
        emit,
    );
    sink.terminal = Terminal::builder()
        .is_tty(true)
        .color_depth(ColorDepth::TrueColor)
        .osc_link_support(true)
        .build();
    sink
}

/// Review-1 Finding 1 — live dispatched/logged records must carry
/// `agent_pid` once the wrapper has stamped the spawned child PID.
///
/// The same `DispatchEventMeta` is handed to both the JSONL logger and
/// the hook dispatcher (step 7 of `on_semantic_event`), so asserting the
/// logged copy proves the dispatched hook/action context too. Before
/// `set_agent_pid`, records stay `None`; after, they carry the PID.
#[test]
fn live_records_carry_agent_pid_after_set() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let logged: Arc<StdMutex<Vec<DispatchEventMeta>>> = Arc::new(StdMutex::new(Vec::new()));

    let logger = {
        let captured = logged.clone();
        Box::new(move |_event: &SemanticEvent, meta: &DispatchEventMeta| {
            captured.lock().unwrap().push(meta.clone());
        })
    };

    let mut sink = make_sink(lines, dispatched).with_event_logger(logger);

    // Before spawn the wrapper has no child PID to report.
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("bash".into()),
        id: Some("t1".into()),
        input: None,
        extra: json!({}),
    });

    sink.set_agent_pid(Some(98_765));

    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("bash".into()),
        id: Some("t2".into()),
        input: None,
        extra: json!({}),
    });

    let collected = logged.lock().unwrap().clone();
    assert_eq!(collected.len(), 2);
    assert_eq!(
        collected[0].agent_pid, None,
        "records before set_agent_pid must omit agent_pid"
    );
    assert_eq!(
        collected[1].agent_pid,
        Some(98_765),
        "records after set_agent_pid must carry the spawned child PID"
    );
}

#[test]
fn live_metrics_updated_from_events() {
    let lines = Arc::new(StdMutex::new(Vec::new()));
    let dispatched = Arc::new(StdMutex::new(Vec::new()));
    let mut sink = make_sink(lines, dispatched);
    let metrics = sink.live_metrics();
    sink.on_semantic_event(SemanticEvent::ToolCall {
        name: Some("bash".into()),
        id: Some("t1".into()),
        input: None,
        extra: json!({}),
    });
    let state = metrics.lock().unwrap();
    assert_eq!(state.in_flight.len(), 1);
}
