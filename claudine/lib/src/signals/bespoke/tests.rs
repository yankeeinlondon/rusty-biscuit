use claudine_catalog_types::SignalKind;

use super::super::{SignalEngine, SignalHub, detection_table};
use super::*;

const CLAUDE_BILLING: &str = include_str!(
    "../../../../docs/research/signals/fixtures/claude/billing-error-synthetic-result.jsonl"
);
const GOOSE_ERROR_THEN_COMPLETE: &str = include_str!(
    "../../../../docs/research/signals/fixtures/goose/stream-error-then-complete.jsonl"
);
const PI_RETRY_EXHAUSTED: &str = include_str!(
    "../../../../docs/research/signals/fixtures/pi/stream-auto-retry-exhausted.jsonl"
);
const QWEN_LOOP: &str =
    include_str!("../../../../docs/research/signals/fixtures/qwen/result-loop-detected.jsonl");

fn payloads(fixture: &str) -> Vec<Value> {
    fixture
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("fixture line is JSON"))
        .collect()
}

fn run_chain(slug: &str, source: SignalSource, payloads: &[Value]) -> Vec<SignalEvent> {
    let mut chain = BespokeChain::for_slug(slug);
    payloads
        .iter()
        .flat_map(|payload| chain.observe(source, payload))
        .collect()
}

#[test]
fn claude_taint_fires_once_on_billing_evidence_with_prior_error_cause() {
    let payloads = payloads(CLAUDE_BILLING);
    let events = run_chain("claude", SignalSource::Stream, &payloads);
    assert_eq!(
        events,
        vec![SignalEvent::SessionTainted {
            cause: "billing_error: Credit balance is too low".to_string(),
        }]
    );
    // Emit-once: replaying the terminal result again stays silent.
    let mut chain = BespokeChain::for_slug("claude");
    for payload in &payloads {
        chain.observe(SignalSource::Stream, payload);
    }
    assert!(chain.observe(SignalSource::Stream, &payloads[1]).is_empty());
}

#[test]
fn claude_taint_ignores_error_subtypes_and_clean_results() {
    // An error subtype already claims the failure declaratively.
    let claimed: Value = serde_json::json!({
        "type": "result", "subtype": "error_during_execution", "is_error": true,
    });
    // A clean success envelope is not tainted.
    let clean: Value = serde_json::json!({
        "type": "result", "subtype": "success", "is_error": false,
    });
    assert!(run_chain("claude", SignalSource::Stream, &[claimed, clean]).is_empty());
}

#[test]
fn goose_taint_fires_on_error_then_complete_with_error_cause() {
    let events = run_chain(
        "goose",
        SignalSource::Stream,
        &payloads(GOOSE_ERROR_THEN_COMPLETE),
    );
    assert_eq!(events.len(), 1);
    match &events[0] {
        SignalEvent::SessionTainted { cause } => {
            assert!(cause.starts_with("Context length exceeded"), "cause: {cause}");
        }
        other => panic!("expected SessionTainted, got {other:?}"),
    }
}

#[test]
fn goose_complete_without_prior_error_is_silent() {
    let complete: Value = serde_json::json!({ "type": "complete", "total_tokens": null });
    assert!(run_chain("goose", SignalSource::Stream, &[complete]).is_empty());
}

#[test]
fn pi_retries_exhausted_fires_on_evidence_with_attempt_and_message() {
    let events = run_chain("pi", SignalSource::Stream, &payloads(PI_RETRY_EXHAUSTED));
    assert_eq!(
        events,
        vec![SignalEvent::RetriesExhausted {
            status_code: None,
            attempts: Some(3),
            message: Some("Provider returned error: 503 service unavailable".to_string()),
        }]
    );
}

#[test]
fn pi_retry_cancel_and_unexhausted_budget_stay_silent() {
    let cancelled: Value = serde_json::json!({
        "type": "auto_retry_end", "success": false, "attempt": 2,
        "finalError": "Retry cancelled",
    });
    assert!(run_chain("pi", SignalSource::Stream, &[cancelled]).is_empty());

    let start: Value = serde_json::json!({
        "type": "auto_retry_start", "attempt": 1, "maxAttempts": 3,
        "delayMs": 1000, "errorMessage": "boom",
    });
    let early_end: Value = serde_json::json!({
        "type": "auto_retry_end", "success": false, "attempt": 1,
        "finalError": "boom",
    });
    assert!(run_chain("pi", SignalSource::Stream, &[start, early_end]).is_empty());
}

#[test]
fn qwen_loop_detected_fires_on_evidence_with_zeroed_counts() {
    let events = run_chain("qwen", SignalSource::Stream, &payloads(QWEN_LOOP));
    assert_eq!(
        events,
        vec![SignalEvent::RunawayRepetition {
            cycle_len: 0,
            repeats: 0,
        }]
    );
}

/// Mirror gate: every `QwenLoopType` token the guard scans for must be
/// documented in the qwen research record's `vocabulary:` list, so the
/// enum (the single source) and the corpus cannot drift.
#[test]
fn qwen_loop_type_enum_mirrors_the_research_vocabulary() {
    const QWEN_DOC: &str = include_str!("../../../../docs/research/signals/qwen.md");
    for token in QwenLoopType::iter().map(<&str>::from) {
        assert!(
            QWEN_DOC.contains(token),
            "QwenLoopType::{token} is not documented in qwen.md vocabulary"
        );
    }
}

#[test]
fn qwen_error_result_without_loop_token_is_silent() {
    let payload: Value = serde_json::json!({
        "type": "result", "subtype": "error_during_execution", "is_error": true,
        "error": { "message": "missing api key" },
    });
    assert!(run_chain("qwen", SignalSource::Stream, &[payload]).is_empty());
}

#[test]
fn qwen_exit_codes_map_to_ratified_kinds_and_others_stay_silent() {
    let cases = [
        (53, Some(SignalKind::TurnLimitReached)),
        (55, Some(SignalKind::SessionTimeLimitReached)),
        (130, Some(SignalKind::Interrupted)),
        (0, None),
        (1, None),
        (143, None),
    ];
    for (code, expected) in cases {
        let events = run_chain(
            "qwen",
            SignalSource::Exit,
            &[exit_source_payload(code, "", "tail")],
        );
        let kinds: Vec<SignalKind> = events.iter().map(SignalEvent::kind).collect();
        match expected {
            Some(kind) => assert_eq!(kinds, vec![kind], "exit code {code}"),
            None => assert!(kinds.is_empty(), "exit code {code} fired {kinds:?}"),
        }
    }
}

#[test]
fn kimi_version_window_in_range_silent_out_of_range_fires() {
    let init = |version: &str| -> Value {
        serde_json::json!({
            "jsonrpc": "2.0", "id": "init-1",
            "result": { "protocol_version": version, "server": { "name": "Kimi Code CLI" } },
        })
    };
    for version in SUPPORTED_WIRE_PROTOCOL_VERSIONS {
        assert!(
            run_chain("kimi", SignalSource::Stream, &[init(version)]).is_empty(),
            "{version} is inside the supported window"
        );
    }
    let events = run_chain("kimi", SignalSource::Stream, &[init("2.0")]);
    assert_eq!(
        events,
        vec![SignalEvent::UnsupportedProtocolVersion {
            version: "2.0".to_string(),
            supported: SUPPORTED_WIRE_PROTOCOL_VERSIONS
                .iter()
                .map(|v| v.to_string())
                .collect(),
        }]
    );
}

#[test]
fn exit_source_payload_carries_code_and_last_lines_tail() {
    let long = |prefix: &str| {
        (1..=15)
            .map(|n| format!("{prefix} {n}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let payload = exit_source_payload(53, &long("out"), &long("err"));
    assert_eq!(payload["exit_code"], 53);

    let stdout_tail = payload["stdout_tail"].as_str().unwrap();
    assert!(stdout_tail.starts_with("out 6"), "stdout_tail: {stdout_tail}");
    assert!(stdout_tail.ends_with("out 15"), "stdout_tail: {stdout_tail}");
    assert_eq!(stdout_tail.lines().count(), EXIT_STDOUT_TAIL_LINES);

    let stderr_tail = payload["stderr_tail"].as_str().unwrap();
    assert!(stderr_tail.starts_with("err 6"), "stderr_tail: {stderr_tail}");
    assert!(stderr_tail.ends_with("err 15"), "stderr_tail: {stderr_tail}");
    assert_eq!(stderr_tail.lines().count(), EXIT_STDERR_TAIL_LINES);

    // Short streams pass through whole, each on its own field.
    let short = exit_source_payload(1, "only out", "only err");
    assert_eq!(short["stdout_tail"], "only out");
    assert_eq!(short["stderr_tail"], "only err");
}

/// Shipped-behavior parity: the SAME payload the wrapper synthesizes at
/// exit — Antigravity's `agy` writes its auth errors to stdout, so they
/// arrive in `stdout_tail`, not `stderr_tail` — must fire the production
/// `source: exit` AuthInvalid records. This is the guard the fixtures
/// alone cannot give: it proves the runtime payload shape matches what the
/// generated records key on.
#[test]
fn antigravity_exit_stdout_tail_fires_auth_invalid_records() {
    let table = detection_table("antigravity").expect("antigravity table");
    let cases = [
        (
            "Error: Please sign in to view available models. Launch the CLI without arguments to sign in.",
            "exit-auth_invalid-models-signin",
        ),
        (
            "Error: authentication failed or timed out",
            "exit-auth_invalid-print-timeout",
        ),
    ];
    for (stdout, record_id) in cases {
        let payload = exit_source_payload(1, stdout, "");

        let mut engine = SignalEngine::new(table);
        let kinds: Vec<SignalKind> = engine
            .observe(SignalSource::Exit, &payload)
            .iter()
            .map(SignalEvent::kind)
            .collect();
        assert_eq!(kinds, vec![SignalKind::AuthInvalid], "kind for {record_id}");

        let fired: Vec<&str> = engine
            .observe_detailed(SignalSource::Exit, &payload)
            .iter()
            .map(|obs| obs.record.id)
            .collect();
        assert!(
            fired.contains(&record_id),
            "record {record_id} must fire on its stdout evidence; fired {fired:?}"
        );
    }
}

/// The invariant this finding is about: every compiled
/// [`DetectionMode::Bespoke`] record must have a registered replayer,
/// which proves a runtime detector exists, so `signals check` cannot go
/// green while a bespoke record silently lacks a detector.
#[test]
fn every_bespoke_record_has_a_registered_replayer() {
    use claudine_catalog_types::DetectionMode;

    use super::super::all_detection_tables;

    for table in all_detection_tables() {
        for record in table.records {
            if record.mode == DetectionMode::Bespoke {
                assert!(
                    bespoke_replayer(record.id).is_some(),
                    "bespoke record `{}` (provider `{}`) has no registered replayer — \
                     wire a runtime detector/replayer",
                    record.id,
                    table.slug,
                );
            }
        }
    }
}

#[test]
fn every_bespoke_record_with_evidence_has_a_replayer_that_fires() {
    let cases = [
        ("stream-session_tainted-result-error", CLAUDE_BILLING),
        ("stream-session_tainted-error-then-complete", GOOSE_ERROR_THEN_COMPLETE),
        ("stream-retries_exhausted-auto_retry_end", PI_RETRY_EXHAUSTED),
        ("stream-runaway_repetition-result-loop", QWEN_LOOP),
    ];
    for (record_id, fixture) in cases {
        let replay = bespoke_replayer(record_id).expect(record_id);
        assert!(replay(&payloads(fixture)), "{record_id} must fire on its evidence");
    }
    assert!(bespoke_replayer("no-such-record").is_none());
}

/// Hub integration: the goose error-then-complete sequence through
/// `observe_json` yields `SessionTainted` from the bespoke chain
/// alongside the declarative `tokens_consumed` fire on the trailing
/// `complete` frame, all in one sink.
#[test]
fn hub_observe_json_runs_bespoke_chain_alongside_declarative_engine() {
    let hub = SignalHub::new(detection_table("goose").expect("goose table"));
    for payload in payloads(GOOSE_ERROR_THEN_COMPLETE) {
        hub.observe_json(SignalSource::Stream, &payload);
    }
    let signals = hub.drain();
    let kinds: Vec<SignalKind> = signals.iter().map(|s| s.event.kind()).collect();
    assert!(kinds.contains(&SignalKind::SessionTainted), "kinds: {kinds:?}");
    assert!(kinds.contains(&SignalKind::TokensConsumed), "kinds: {kinds:?}");
}
