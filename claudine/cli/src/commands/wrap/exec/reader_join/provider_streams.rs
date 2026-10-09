//! Real provider streams through the reader's snapshot publication, then a
//! reader cut off past the cutoff.
//!
//! The reader here is the production pairing: a real provider parser, built by
//! the wrapper's parser plumbing, feeding a [`LiveSemanticSink`] inside a
//! [`RunScope`], each line fed through
//! [`run_scope::feed_line`]. Two stalls are covered:
//!
//! - after the last line the reader blocks on its source the way it blocks on
//!   a pipe a descendant still holds open;
//! - the reader stalls *inside* the completion line, in the sink's logging or
//!   hook callback for the provider's completion event.
//!
//! Either way it never hands its parser back, and settlement has only what was
//! published.

use super::*;
use crate::commands::wrap::live_semantic_sink::LiveSemanticSink;
use crate::commands::wrap::StructuredSummaryDetails;
use claudine::stream::stderr::Verbosity;
use crate::commands::wrap::run_scope::{self, RunScope};
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::ParserConfig;
use crate::commands::wrap::live_semantic_sink::{SemanticDispatchFn, SemanticEventLoggerFn};
use crate::commands::wrap::policy::build_structured_plumbing;
use claudine::events::AgenticEvent;
use claudine::signals::SignalHub;
use claudine::stream::semantic::SemanticEvent;
use std::path::Path;
use std::sync::mpsc;

const BUDGET: ReaderBudget = ReaderBudget {
    pipe_cap: Duration::from_millis(20),
    pipe_settle: Duration::from_millis(20),
    drain_limit: Duration::from_secs(10),
};

/// `provider`'s parser emitting into `sink`, built the way a wrapped run
/// builds it.
fn production_parser(provider: Provider, sink: LiveSemanticSink) -> Box<dyn SemanticStreamParser> {
    let hub = Arc::new(SignalHub::new(claudine::signals::for_provider(provider)));
    let (build_parser, _stderr_bridge, _content) =
        build_structured_plumbing(provider, sink, ParserConfig::default(), None, &hub);
    build_parser(Box::new(|_chunk| {}), Box::new(|_chunk| {}), None)
}

/// Feed `lines` to a reader for `provider`, hold its source open, and settle
/// the run at `exit_code` once the reader is cut off.
fn settle_held_open(
    provider: Provider,
    lines: &[&str],
    exit_code: i32,
) -> (StreamExecutionSummary, Option<String>) {
    let scope = RunScope::default();
    let progress = ReaderProgress::default();
    let slot: ParserSlot = Arc::new(Mutex::new(None));
    let (source, pipe) = mpsc::channel::<String>();
    let (fed_tx, fed) = mpsc::channel::<()>();
    let (reader_scope, reader_progress, reader_slot) =
        (scope.clone(), progress.clone(), Arc::clone(&slot));
    let handle = thread::spawn(move || {
        let _guard = reader_scope.enter();
        let sink = LiveSemanticSink::new(
            provider,
            EnvironmentContext::default(),
            Path::new("/tmp"),
            Verbosity::Quiet,
            Arc::new(Mutex::new(StructuredSummaryDetails::default())),
            Box::new(|_event, _meta| {}),
            Box::new(|_line| {}),
        );
        let mut parser = production_parser(provider, sink);
        for line in reader_progress.track(pipe.into_iter()) {
            run_scope::feed_line(&mut *parser, &line);
            let _ = fed_tx.send(());
        }
        *reader_slot.lock().unwrap() = Some(parser);
    });
    for line in lines {
        source.send((*line).to_string()).unwrap();
        fed.recv_timeout(Duration::from_secs(10)).expect("the reader handles each line");
    }

    let outcome = join_reader(handle, &progress, BUDGET, Instant::now());
    assert!(matches!(outcome, JoinOutcome::TimedOut(ReaderStall::PipeOpen)), "{outcome:?}");
    let (parser, warning) = settle_parser(outcome, &slot, exit_code, BUDGET, &scope.snapshot());
    scope.close();
    // Closing the source lets the detached reader finish, so no thread
    // outlives the test.
    drop(source);
    (parser.finish(exit_code), warning)
}

/// The scripted Claude stream: initialization, an answer, a successful result.
const CLAUDE_ANSWER: &[&str] = &[
    r#"{"type":"system","subtype":"init","session_id":"session-1","model":"claude-opus"}"#,
    r#"{"type":"assistant","message":{"content":[{"type":"text","text":"review-answer"}]}}"#,
    r#"{"type":"result","subtype":"success","result":"review-answer","session_id":"session-1","is_error":false,"duration_ms":1200,"num_turns":1,"usage":{"input_tokens":100,"output_tokens":50}}"#,
];

/// The stopped-task regression stream (`tests/common/incomplete_subagents.rs`):
/// a started task, its `stopped` notification, and a parent result that reads
/// as success. The failure exists only in the parser's task ledger.
const CLAUDE_STOPPED_TASK: &[&str] = &[
    r#"{"type":"system","subtype":"init","session_id":"incident","model":"claude-opus"}"#,
    r#"{"type":"task_started","task_id":"sa_1","name":"commit-alpha"}"#,
    r#"{"type":"assistant","message":{"content":[{"type":"text","text":"All done."}]}}"#,
    r#"{"type":"task_notification","task_id":"sa_1","name":"commit-alpha","status":"stopped"}"#,
    r#"{"type":"result","subtype":"success","stop_reason":"end_turn","num_turns":1,"duration_ms":600000}"#,
];

#[test]
fn a_reader_held_open_after_a_claude_answer_keeps_the_answer_session_and_usage() {
    let (summary, warning) = settle_held_open(Provider::Claude, CLAUDE_ANSWER, 0);

    assert!(!summary.is_error, "{summary:?}");
    assert_eq!(summary.exit_code, 0);
    assert!(summary.assistant_text.contains("review-answer"), "{summary:?}");
    assert_eq!(summary.session_id.as_deref(), Some("session-1"));
    assert_eq!(summary.token_usage.and_then(|usage| usage.input), Some(100));
    assert_eq!(summary.duration_ms, Some(1200));
    let warning = warning.expect("the cutoff is announced");
    assert!(warning.contains("result is kept"), "{warning}");
}

#[test]
fn a_reader_cut_off_after_a_stopped_task_keeps_the_incomplete_subagents_failure() {
    let (summary, warning) = settle_held_open(Provider::Claude, CLAUDE_STOPPED_TASK, 0);

    assert!(summary.is_error, "a stopped sub-agent is not a success: {summary:?}");
    assert_eq!(summary.error_kind.as_deref(), Some("incomplete_subagents"));
    assert_eq!(summary.subagent_outcomes.len(), 1, "{summary:?}");
    assert_eq!(summary.exit_code, 0, "the native exit is not rewritten");
    assert_eq!(summary.session_id.as_deref(), Some("incident"));
    let warning = warning.expect("the cutoff is announced");
    assert!(warning.contains("error is kept"), "{warning}");
}

#[test]
fn a_reader_cut_off_after_a_provider_error_keeps_it_with_the_real_exit_code() {
    let lines = [
        CLAUDE_ANSWER[0],
        r#"{"type":"result","subtype":"error_during_execution","is_error":true,"session_id":"session-1","error":{"type":"overloaded_error","message":"Overloaded"}}"#,
    ];
    let (summary, warning) = settle_held_open(Provider::Claude, &lines, 1);

    assert!(summary.is_error, "{summary:?}");
    assert_eq!(summary.exit_code, 1);
    assert_eq!(summary.session_id.as_deref(), Some("session-1"));
    assert!(warning.is_some());
}

#[test]
fn a_reader_held_open_after_a_codex_answer_keeps_the_answer() {
    let lines = [
        r#"{"type":"thread.started","thread_id":"th-1"}"#,
        r#"{"type":"item.completed","item":{"id":"a1","type":"agent_message","text":"codex-answer"}}"#,
        r#"{"type":"turn.completed","usage":{"input_tokens":100,"output_tokens":50},"duration_ms":800,"status":"completed"}"#,
    ];
    let (summary, _warning) = settle_held_open(Provider::Codex, &lines, 0);

    assert!(!summary.is_error, "{summary:?}");
    assert!(summary.assistant_text.contains("codex-answer"), "{summary:?}");
}

#[test]
fn a_reader_cut_off_before_any_result_is_still_an_incomplete_stream() {
    let (summary, _warning) = settle_held_open(Provider::Claude, &CLAUDE_ANSWER[..2], 0);

    assert!(summary.is_error);
    assert_eq!(summary.error_kind.as_deref(), Some("claudine_completion_delayed"));
    assert!(summary.assistant_text.is_empty(), "no result is invented from a partial stream");
}

/// Which completion callback of the sink the reader stalls in.
#[derive(Debug, Clone, Copy)]
enum Held {
    /// The JSONL event logger.
    Log,
    Answer,
    /// Lifecycle hook dispatch.
    Hook,
}

/// Whether `event` is the one whose callbacks the reader stalls in.
fn completes(event: &SemanticEvent) -> bool {
    matches!(
        event,
        SemanticEvent::TurnComplete { .. } | SemanticEvent::Error { terminal: true, .. }
    )
}

/// What a callback-held run settled as, beside what the same parser finalized
/// to once the callback was released.
struct HeldSettlement {
    settled: StreamExecutionSummary,
    warning: Option<String>,
    unblocked: StreamExecutionSummary,
    observation: run_scope::CompletionObservation,
    completion_dispatches: usize,
}

/// Feed `lines` to a reader for `provider`, built through the production
/// parser plumbing, with the `held` callback of its completion event blocked
/// until teardown. The run is settled at `exit_code` once the reader's drain
/// limit expires while it is inside that callback.
fn settle_held_in_completion(
    provider: Provider,
    lines: &[&str],
    held: Held,
    exit_code: i32,
) -> HeldSettlement {
    let scope = RunScope::default();
    let progress = ReaderProgress::default();
    let slot: ParserSlot = Arc::new(Mutex::new(None));
    let (entered_tx, entered) = mpsc::channel::<()>();
    let (release_tx, release) = mpsc::channel::<()>();
    let release = Arc::new(Mutex::new(release));
    let (finished_tx, finished) = mpsc::channel::<()>();
    let completion_dispatches = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let dispatches = completion_dispatches.clone();
    let owned: Vec<String> = lines.iter().map(|line| (*line).to_string()).collect();
    let (reader_scope, reader_progress, reader_slot) =
        (scope.clone(), progress.clone(), Arc::clone(&slot));
    let reader = thread::spawn(move || {
        let _guard = reader_scope.enter();
        // Blocks until the test releases it or, if the test panicked first,
        // drops the release sender.
        let hold = {
            let release = Arc::clone(&release);
            move || {
                let _ = entered_tx.send(());
                let _ = release.lock().unwrap().recv();
            }
        };
        let (dispatch, logger): (SemanticDispatchFn, SemanticEventLoggerFn) = match held {
            Held::Log => (
                Box::new(move |event, _meta| {
                    if matches!(event, AgenticEvent::TurnComplete | AgenticEvent::TurnError) {
                        dispatches.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }),
                Box::new(move |event, _meta| {
                    if completes(event) {
                        hold();
                    }
                }),
            ),
            Held::Answer => (
                Box::new(|_event, _meta| {}),
                Box::new(move |event, _meta| {
                    if matches!(event, SemanticEvent::OutputText { .. })
                        || matches!(event, SemanticEvent::Reasoning { extra, .. } if extra["origin"] == "agent_message") { hold(); }
                }),
            ),
            Held::Hook => (
                Box::new(move |event, _meta| {
                    if matches!(event, AgenticEvent::TurnComplete | AgenticEvent::TurnError) {
                        hold();
                    }
                }),
                Box::new(|_event, _meta| {}),
            ),
        };
        let sink = LiveSemanticSink::new(
            provider,
            EnvironmentContext::default(),
            Path::new("/tmp"),
            Verbosity::Quiet,
            Arc::new(Mutex::new(StructuredSummaryDetails::default())),
            dispatch,
            Box::new(|_line| {}),
        )
        .with_event_logger(logger);
        let mut parser = production_parser(provider, sink);
        for line in reader_progress.track(owned.into_iter()) {
            run_scope::feed_line(&mut *parser, &line);
        }
        *reader_slot.lock().unwrap() = Some(parser);
        let _ = finished_tx.send(());
    });
    entered
        .recv_timeout(Duration::from_secs(10))
        .unwrap_or_else(|_| panic!("{provider:?}: the completion callback is reached"));

    let budget = ReaderBudget { drain_limit: Duration::from_millis(30), ..BUDGET };
    let outcome = join_reader(reader, &progress, budget, Instant::now());
    assert!(
        matches!(outcome, JoinOutcome::TimedOut(ReaderStall::Processing)),
        "{provider:?}: {outcome:?}"
    );
    let (parser, warning) = settle_parser(outcome, &slot, exit_code, budget, &scope.snapshot());
    let settled = parser.finish(exit_code);
    let observation = scope.freeze();
    release_tx.send(()).expect("the held callback is still waiting");
    finished.recv_timeout(Duration::from_secs(10)).expect("the released reader finishes");
    let unblocked = slot.lock().unwrap().take().expect("the reader hands back its parser");
    assert_eq!(scope.freeze(), observation, "late reader cannot mutate frozen data");
    HeldSettlement {
        settled,
        observation,
        completion_dispatches: completion_dispatches.load(std::sync::atomic::Ordering::Acquire),
        warning,
        unblocked: unblocked.finish(exit_code),
    }
}

const OPENCODE_ANSWER: &[&str] = &[
    r#"{"type":"step_start","sessionID":"ses_1"}"#,
    r#"{"type":"text","text":"opencode-answer"}"#,
    r#"{"type":"step_complete","usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15},"cost_usd":0.001,"duration_ms":200}"#,
];

/// A successful stream and the answer its parser reports.
struct AnswerCase {
    provider: Provider,
    lines: &'static [&'static str],
    answer: &'static str,
}

/// Every structured provider identity.
const STRUCTURED_ANSWERS: &[AnswerCase] = &[
    AnswerCase { provider: Provider::Claude, lines: CLAUDE_ANSWER, answer: "review-answer" },
    AnswerCase {
        provider: Provider::Codex,
        lines: &[
            r#"{"type":"thread.started","thread_id":"th-1"}"#,
            r#"{"type":"item.completed","item":{"id":"a1","type":"agent_message","text":"codex-answer"}}"#,
            r#"{"type":"turn.completed","usage":{"input_tokens":100,"output_tokens":50},"duration_ms":800,"status":"completed"}"#,
        ],
        answer: "codex-answer",
    },
    AnswerCase {
        provider: Provider::Gemini,
        lines: &[
            r#"{"type":"init","session_id":"g1","model":"gemini-2.5"}"#,
            r#"{"type":"message","role":"assistant","delta":true,"content":"gemini-answer"}"#,
            r#"{"type":"result","status":"success","stats":{"input_tokens":100,"output_tokens":50,"duration_ms":1000}}"#,
        ],
        answer: "gemini-answer",
    },
    AnswerCase { provider: Provider::OpenCode, lines: OPENCODE_ANSWER, answer: "opencode-answer" },
    AnswerCase { provider: Provider::Kilo, lines: OPENCODE_ANSWER, answer: "opencode-answer" },
    AnswerCase {
        provider: Provider::QwenCode,
        lines: &[
            r#"{"type":"init","session_id":"q1","model":"qwen-coder"}"#,
            r#"{"type":"message","role":"assistant","content":[{"text":"qwen-answer"}]}"#,
            r#"{"type":"summary","duration_ms":3000,"token_usage":{"input_tokens":100,"output_tokens":50}}"#,
        ],
        answer: "qwen-answer",
    },
    AnswerCase {
        provider: Provider::Pi,
        lines: &[
            r#"{"type":"session","version":3,"id":"s-1","cwd":"/work"}"#,
            r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"pi-answer"}}"#,
            r#"{"type":"message_end","message":{"stopReason":"stop","usage":{"input":10,"output":5,"totalTokens":15}}}"#,
            r#"{"type":"agent_end","willRetry":false}"#,
        ],
        answer: "pi-answer",
    },
    AnswerCase {
        provider: Provider::KimiCode,
        lines: &[
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnBegin","payload":{"user_input":"hi"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"ContentPart","payload":{"type":"text","text":"kimi-answer"}}}"#,
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"TurnEnd","payload":{}}}"#,
        ],
        answer: "kimi-answer",
    },
    AnswerCase {
        provider: Provider::Antigravity,
        lines: &[
            r#"{"conversation_id":"a1","status":"SUCCESS","response":"agy-answer","duration_seconds":1.7,"num_turns":1,"usage":{"input_tokens":100,"output_tokens":6}}"#,
        ],
        answer: "agy-answer",
    },
];

/// The settled summary carries what the parser itself concluded.
fn assert_matches_unblocked(label: &str, run: &HeldSettlement) {
    assert!(run.observation.verdict_received);
    let expected = if run.settled.is_error && run.settled.assistant_text.is_empty() { None }
        else { Some(run.settled.assistant_text.as_str()) };
    assert_eq!(run.observation.retained.response_text.as_deref(), expected);
    let (settled, unblocked) = (&run.settled, &run.unblocked);
    assert_eq!(settled.assistant_text, unblocked.assistant_text, "{label}");
    assert_eq!(settled.is_error, unblocked.is_error, "{label}");
    assert_eq!(settled.error_kind, unblocked.error_kind, "{label}");
    assert_eq!(settled.session_id, unblocked.session_id, "{label}");
    assert_eq!(settled.token_usage, unblocked.token_usage, "{label}");
    assert_eq!(settled.provider_status, unblocked.provider_status, "{label}");
}

#[test]
fn a_reader_stalled_in_a_completion_callback_keeps_every_providers_answer_and_verdict() {
    for &AnswerCase { provider, lines, answer } in STRUCTURED_ANSWERS {
        for held in [Held::Log, Held::Hook] {
            let label = format!("{provider:?}, {held:?} held");
            let run = settle_held_in_completion(provider, lines, held, 0);

            assert!(!run.settled.is_error, "{label}: {:?}", run.settled);
            assert_eq!(run.settled.exit_code, 0, "{label}");
            assert!(run.settled.assistant_text.contains(answer), "{label}: {:?}", run.settled);
            assert_matches_unblocked(&label, &run);
            let warning = run.warning.unwrap_or_else(|| panic!("{label}: no warning"));
            assert!(warning.contains("result is kept"), "{label}: {warning}");
        }
    }
}

#[test]
fn a_reader_stalled_in_a_completion_callback_keeps_the_incomplete_subagents_failure() {
    for held in [Held::Log, Held::Hook] {
        let run = settle_held_in_completion(Provider::Claude, CLAUDE_STOPPED_TASK, held, 0);

        let label = format!("{held:?} held");
        assert!(run.settled.is_error, "{label}: a stopped sub-agent is not a success");
        assert_eq!(run.settled.error_kind.as_deref(), Some("incomplete_subagents"), "{label}");
        assert!(run.settled.assistant_text.contains("All done."), "{label}: {:?}", run.settled);
        assert_eq!(run.settled.exit_code, 0, "{label}: the native exit is not rewritten");
        assert_matches_unblocked(&label, &run);
        let warning = run.warning.unwrap_or_else(|| panic!("{label}: no warning"));
        assert!(warning.contains("error is kept"), "{label}: {warning}");
    }
}

#[test]
fn a_reader_stalled_in_a_completion_callback_keeps_the_provider_error() {
    let lines = [
        CLAUDE_ANSWER[0],
        r#"{"type":"result","subtype":"error_during_execution","is_error":true,"session_id":"session-1","error":{"type":"overloaded_error","message":"Overloaded"}}"#,
    ];
    for held in [Held::Log, Held::Hook] {
        let run = settle_held_in_completion(Provider::Claude, &lines, held, 1);

        let label = format!("{held:?} held");
        assert!(run.settled.is_error, "{label}: {:?}", run.settled);
        assert_eq!(run.settled.exit_code, 1, "{label}");
        assert_eq!(run.settled.session_id.as_deref(), Some("session-1"), "{label}");
        assert_matches_unblocked(&label, &run);
        assert!(run.warning.is_some(), "{label}");
    }
}

fn assert_early_answer(provider: Provider, lines: &[&str]) {
    let run = settle_held_in_completion(provider, lines, Held::Answer, 0);
    assert!(run.settled.is_error);
    assert_eq!(run.settled.error_kind.as_deref(), Some("claudine_completion_delayed"));
    assert_eq!(run.settled.exit_code, 0);
    assert!(run.warning.is_some());
    assert!(!run.observation.verdict_received);
    assert_eq!(run.observation.retained.response_text.as_deref(), Some("review-answer"));
    assert_eq!(run.observation.retained.response_complete, Some(false));
}

#[test]
fn an_answer_callback_before_the_verdict_retains_partial_text_without_success() {
    assert_early_answer(Provider::Claude, CLAUDE_ANSWER);
    assert_early_answer(Provider::Codex, &[
        r#"{"type":"thread.started","thread_id":"th-1"}"#,
        r#"{"type":"item.completed","item":{"id":"a1","type":"agent_message","text":"review-answer"}}"#,
        r#"{"type":"turn.completed","usage":{"input_tokens":100,"output_tokens":50}}"#,
    ]);
}

#[test]
fn released_completion_logger_cannot_dispatch_into_a_settled_run() {
    let run = settle_held_in_completion(Provider::Claude, CLAUDE_ANSWER, Held::Log, 0);
    assert!(!run.settled.is_error);
    assert_eq!(run.completion_dispatches, 0);
}
