//! Phase 6 regression test for the OpenCode "Dual-Source Contract".
//!
//! Replays a captured stderr fixture covering one parent session that
//! spawns two subagents (`task` tool invocations). Verifies that
//! [`OpenCodeLogBridge`] promotes the structured log records to the
//! expected sequence of `SemanticEvent`s — in particular the paired
//! `SubagentStart` / `SubagentStop` events that the wrapper now relies on
//! exclusively (the NDJSON synthesis path was removed in Phase 4 of
//! `fixes/2026-05-12-opencode-stderr-returns/plan.md`).

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use claudine::stream::logs::StderrIngestOutcome;
use claudine::stream::logs::opencode::OpenCodeLogBridge;
use claudine::stream::semantic::{SemanticEvent, SemanticEventSink};

#[derive(Clone)]
struct SharedSink {
    events: Arc<Mutex<Vec<SemanticEvent>>>,
}

impl SemanticEventSink for SharedSink {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        self.events
            .lock()
            .expect("recording sink poisoned")
            .push(event);
    }
}

fn fixture() -> &'static str {
    include_str!("fixtures/logs/opencode-subagent-lifecycle.txt")
}

fn replay_with_events(stdout_seen: bool) -> Vec<SemanticEvent> {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = SharedSink {
        events: Arc::clone(&events),
    };
    let mut bridge = OpenCodeLogBridge::new(sink, Arc::new(AtomicBool::new(stdout_seen)), None, None);
    for line in fixture().lines() {
        let _ = bridge.ingest(line);
    }
    drop(bridge);
    Arc::try_unwrap(events)
        .expect("only one strong ref expected after bridge drop")
        .into_inner()
        .expect("recording sink poisoned")
}

fn kind_of(event: &SemanticEvent) -> &'static str {
    match event {
        SemanticEvent::SessionStart { .. } => "session_start",
        SemanticEvent::SubagentStart { .. } => "subagent_start",
        SemanticEvent::SubagentStop { .. } => "subagent_stop",
        SemanticEvent::Info { .. } => "info",
        SemanticEvent::Warning { .. } => "warning",
        SemanticEvent::Error { .. } => "error",
        _ => "other",
    }
}

#[test]
fn fixture_replay_emits_paired_subagent_lifecycle() {
    let events = replay_with_events(false);

    let session_starts = events
        .iter()
        .filter(|e| matches!(e, SemanticEvent::SessionStart { .. }))
        .count();
    let subagent_starts = events
        .iter()
        .filter(|e| matches!(e, SemanticEvent::SubagentStart { .. }))
        .count();
    let subagent_stops = events
        .iter()
        .filter(|e| matches!(e, SemanticEvent::SubagentStop { .. }))
        .count();

    assert_eq!(
        session_starts,
        1,
        "exactly one primary SessionStart expected; events: {:?}",
        events.iter().map(kind_of).collect::<Vec<_>>(),
    );
    assert_eq!(
        subagent_starts,
        2,
        "expected two SubagentStart events (one per child session); events: {:?}",
        events.iter().map(kind_of).collect::<Vec<_>>(),
    );
    assert_eq!(
        subagent_stops,
        2,
        "expected two SubagentStop events (one per child session); events: {:?}",
        events.iter().map(kind_of).collect::<Vec<_>>(),
    );
}

#[test]
fn fixture_replay_preserves_subagent_start_then_stop_ordering_per_child() {
    let events = replay_with_events(false);

    let mut child_start_index: std::collections::BTreeMap<String, usize> = Default::default();
    let mut child_stop_index: std::collections::BTreeMap<String, usize> = Default::default();
    for (i, event) in events.iter().enumerate() {
        match event {
            SemanticEvent::SubagentStart { id: Some(id), .. } => {
                child_start_index.entry(id.clone()).or_insert(i);
            }
            SemanticEvent::SubagentStop { id: Some(id), .. } => {
                child_stop_index.entry(id.clone()).or_insert(i);
            }
            _ => {}
        }
    }

    for id in ["ses_child_a", "ses_child_b"] {
        let start = child_start_index
            .get(id)
            .copied()
            .unwrap_or_else(|| panic!("missing SubagentStart for {id}"));
        let stop = child_stop_index
            .get(id)
            .copied()
            .unwrap_or_else(|| panic!("missing SubagentStop for {id}"));
        assert!(
            start < stop,
            "SubagentStart for {id} must precede SubagentStop (start={start}, stop={stop})",
        );
    }
}

#[test]
fn fixture_replay_filters_bus_lines_from_semantic_stream() {
    let events = replay_with_events(false);

    let bus_in_extra = events.iter().any(|e| match e {
        SemanticEvent::Info { extra, .. }
        | SemanticEvent::Warning { extra, .. }
        | SemanticEvent::Error { extra, .. } => {
            extra.get("service").and_then(serde_json::Value::as_str) == Some("bus")
        }
        _ => false,
    });
    assert!(
        !bus_in_extra,
        "service=bus records must be silently dropped; events: {events:?}",
    );
}

#[test]
fn fixture_replay_emits_info_for_llm_calls_and_http_responses() {
    let events = replay_with_events(false);

    let mut llm_call_count = 0_usize;
    let mut http_response_count = 0_usize;
    for event in &events {
        if let SemanticEvent::Info { message, .. } = event {
            // Messages are enriched with inline context (provider/model,
            // method/url/status), so match on the well-known prefix.
            if message.starts_with("llm_call_start") {
                llm_call_count += 1;
            } else if message.starts_with("http_response") {
                http_response_count += 1;
            }
        }
    }
    assert_eq!(
        llm_call_count, 3,
        "expected 3 llm_call_start Info events; got events: {events:?}",
    );
    assert_eq!(
        http_response_count, 1,
        "expected 1 http_response Info event; got events: {events:?}",
    );
}

#[test]
fn structured_log_records_are_always_consumed_even_when_unclassified_or_bus() {
    // Regression: the bridge used to return `NotConsumed` for structured
    // records it parsed but did not promote (e.g. `service=bus`,
    // `service=skill`, boot banner). The reader thread then proxied them
    // straight to the user's terminal as raw debug output. Any line that
    // parses as a structured OpenCode log record is owned by the bridge
    // and must never reach raw stderr passthrough.
    let events: Arc<Mutex<Vec<SemanticEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = SharedSink {
        events: Arc::clone(&events),
    };
    let mut bridge = OpenCodeLogBridge::new(sink, Arc::new(AtomicBool::new(false)), None, None);

    // Each of these is a well-formed structured stderr line that the
    // previous implementation leaked: `bus` (filtered), `skill`
    // (Unclassified), and the boot banner (not promoted).
    let structured_lines = [
        "INFO  2026-05-13T00:38:57 +0ms service=bus type=session.updated publishing",
        "WARN  2026-05-13T00:38:57 +0ms service=skill name=foo existing=/a/b duplicate=/c/d duplicate skill name",
        "INFO  2026-05-13T00:38:57 +0ms service=skill count=132 init",
        "INFO  2026-05-13T00:38:57 +7ms service=tool.registry status=completed duration=21 skill",
        "INFO  2026-05-13T00:38:57 +0ms service=default version=0.1.0 opencode",
    ];
    for line in structured_lines {
        assert_eq!(
            bridge.ingest(line),
            StderrIngestOutcome::Consumed,
            "structured log line must be consumed by bridge: {line}",
        );
    }

    // Truly unstructured stderr stays NotConsumed so genuine panics /
    // raw error output still reach the user.
    let raw_line = "thread 'main' panicked at 'oh no', src/main.rs:1:1";
    assert_eq!(
        bridge.ingest(raw_line),
        StderrIngestOutcome::NotConsumed,
        "non-structured raw line must remain NotConsumed for passthrough",
    );
}

#[test]
fn fixture_replay_does_not_double_emit_primary_session_start_when_stdout_seen() {
    let events = replay_with_events(true);

    // Cross-stream dedup: when the stdout NDJSON parser has already
    // emitted a semantic event, the stderr-derived primary SessionStart
    // must be suppressed. Children still promote because the NDJSON
    // synthesis path was removed in Phase 4.
    let session_starts = events
        .iter()
        .filter(|e| matches!(e, SemanticEvent::SessionStart { .. }))
        .count();
    let subagent_starts = events
        .iter()
        .filter(|e| matches!(e, SemanticEvent::SubagentStart { .. }))
        .count();
    assert_eq!(
        session_starts,
        0,
        "primary SessionStart must be deduped when stdout has already emitted; events: {:?}",
        events.iter().map(kind_of).collect::<Vec<_>>(),
    );
    assert_eq!(
        subagent_starts,
        2,
        "subagent starts must NOT be deduped by stdout_seen; events: {:?}",
        events.iter().map(kind_of).collect::<Vec<_>>(),
    );
}
