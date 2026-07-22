//! Structured stream parsing and per-provider protocol handling.
//!
//! See [`claudine/docs/topics/stream-parsing.md`](../../../docs/topics/stream-parsing.md)
//! for the protocol model, semantic event taxonomy, per-provider
//! adapters, and the wrapper-time rendering pipeline.

pub mod badges;
pub mod logs;
pub mod parser;
pub mod path_link;
pub mod progress;
pub mod prompt_timing;
pub mod protocol;
pub mod providers;
pub mod reporting;
pub mod semantic;
pub mod stderr;
pub mod summary;
pub mod thinking;
pub mod token_usage;
pub mod tool_display;

use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::provider_id::Provider;
use parser::SemanticStreamParser;

pub use semantic::{NullSemanticSink, SemanticEvent, SemanticEventSink};

/// Ensure text from a full assistant message ends with a newline.
///
/// When the stream renderer receives text from consecutive assistant messages
/// (e.g. between tool calls), it concatenates them in its line buffer. Without
/// a trailing newline, sentences run together without whitespace separation
/// (e.g. "files.Let me" instead of proper paragraph flow).
///
/// This must only be applied to complete message text, not streaming deltas.
pub(crate) fn ensure_message_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// The structured stream format used by a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StreamProtocol {
    StreamJson,
    Ndjson,
    Jsonl,
    /// JSON-RPC 2.0 over line-delimited JSON. Used by Kimi Code's `--wire`
    /// mode where stdout carries `{"jsonrpc":"2.0",…}` envelopes (events,
    /// requests, responses) instead of provider-specific stream-json shapes.
    WireJsonRpc,
    /// A single buffered JSON document for the entire response (not
    /// line-delimited). Used by Antigravity's `--output-format json` print
    /// mode, whose stdout is one `{conversation_id,status,response,usage,…}`
    /// object emitted after the run completes.
    Json,
}

/// Optional configuration for parser construction.
#[derive(Debug, Clone, Default)]
pub struct ParserConfig {
    /// External model name (for providers like OpenCode that don't report it in stream).
    pub model: Option<String>,
}

/// Create a semantic-event parser for a provider.
///
/// Dispatches through the provider's
/// [`ProviderBehavior::create_semantic_parser`](crate::provider::ProviderBehavior::create_semantic_parser)
/// implementation. Providers without a native semantic parser fall back
/// through the trait default to a degenerate Claude-shaped parser.
pub fn create_semantic_parser<S: SemanticEventSink + Send + 'static>(
    provider: Provider,
    sink: S,
    config: ParserConfig,
) -> Box<dyn SemanticStreamParser> {
    let boxed: Box<dyn SemanticEventSink + Send + 'static> = Box::new(sink);
    crate::provider::provider_info(provider)
        .behavior
        .create_semantic_parser(boxed, config)
}

/// Return the stream protocol used by a provider, if supported.
pub fn stream_protocol_for(provider: Provider) -> Option<StreamProtocol> {
    crate::provider::provider_info(provider).stream_protocol
}

pub(crate) fn trace_parser_event(provider: Provider, event_type: &str, line_num: usize) {
    let _span = tracing::info_span!(
        "stream_parse_event",
        provider = %provider,
        event_type,
        line_num,
    )
    .entered();
    trace!(
        provider = %provider,
        event_type,
        line_num,
        "parsed structured stream event"
    );
}

pub(crate) fn trace_session_metadata(
    provider: Provider,
    session_id: Option<&str>,
    model: Option<&str>,
) {
    let _span = tracing::info_span!(
        "stream_session_metadata",
        provider = %provider,
        session_id = session_id.unwrap_or(""),
        model = model.unwrap_or(""),
    )
    .entered();
    trace!(
        provider = %provider,
        session_id = session_id.unwrap_or(""),
        model = model.unwrap_or(""),
        "updated stream session metadata"
    );
}

pub(crate) fn trace_tool_event(provider: Provider, tool_calls: u32, tool_name: Option<&str>) {
    let _span = tracing::info_span!(
        "stream_tool_event",
        provider = %provider,
        tool_calls,
        tool_name = tool_name.unwrap_or(""),
    )
    .entered();
    trace!(
        provider = %provider,
        tool_calls,
        tool_name = tool_name.unwrap_or(""),
        "updated stream tool state"
    );
}

pub(crate) fn trace_summary_update(
    provider: Provider,
    provider_status: Option<&str>,
    duration_ms: Option<u64>,
    cost_usd: Option<f64>,
) {
    let _span = tracing::info_span!(
        "stream_summary",
        provider = %provider,
        duration_ms = duration_ms.unwrap_or(0),
    )
    .entered();
    trace!(
        provider = %provider,
        provider_status = provider_status.unwrap_or(""),
        duration_ms = duration_ms.unwrap_or(0),
        cost_usd = cost_usd.unwrap_or(0.0),
        "updated stream summary state"
    );
}

pub(crate) fn trace_parser_finish(
    provider: Provider,
    exit_code: i32,
    tool_calls: u32,
    num_turns: u32,
    provider_status: Option<&str>,
) {
    let _span = tracing::info_span!(
        "stream_parser_finish",
        provider = %provider,
        exit_code,
        tool_calls,
        num_turns,
    )
    .entered();
    trace!(
        provider = %provider,
        exit_code,
        tool_calls,
        num_turns,
        provider_status = provider_status.unwrap_or(""),
        "finished structured stream parser"
    );
}

pub(crate) fn trace_malformed_line(provider: Provider, line_num: usize, message: &str) {
    let _span = tracing::info_span!(
        "stream_malformed",
        provider = %provider,
        line_num,
    )
    .entered();
    trace!(
        provider = %provider,
        line_num,
        %message,
        "skipping malformed structured stream line"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_protocol_serde_round_trip() {
        let protocols = [
            (StreamProtocol::StreamJson, "\"stream-json\""),
            (StreamProtocol::Ndjson, "\"ndjson\""),
            (StreamProtocol::Jsonl, "\"jsonl\""),
            (StreamProtocol::WireJsonRpc, "\"wire-json-rpc\""),
            (StreamProtocol::Json, "\"json\""),
        ];
        for (proto, expected_json) in &protocols {
            let json = serde_json::to_string(proto).unwrap();
            assert_eq!(&json, expected_json);
            let restored: StreamProtocol = serde_json::from_str(&json).unwrap();
            assert_eq!(proto, &restored);
        }
    }

    #[test]
    fn stream_protocol_for_supported_providers() {
        assert_eq!(
            stream_protocol_for(Provider::Claude),
            Some(StreamProtocol::StreamJson)
        );
        assert_eq!(
            stream_protocol_for(Provider::Codex),
            Some(StreamProtocol::Jsonl)
        );
        assert_eq!(
            stream_protocol_for(Provider::OpenCode),
            Some(StreamProtocol::Ndjson)
        );
        assert_eq!(
            stream_protocol_for(Provider::KimiCode),
            Some(StreamProtocol::WireJsonRpc)
        );
    }

    #[test]
    fn stream_protocol_for_unsupported_providers() {
        assert_eq!(stream_protocol_for(Provider::Goose), None);
    }

    mod semantic_parser {
        use std::sync::{Arc, Mutex};

        use super::super::{
            ParserConfig, SemanticEvent, SemanticEventSink, create_semantic_parser,
        };
        use crate::provider_id::Provider;
        struct RecordingSemanticSink {
            events: Arc<Mutex<Vec<SemanticEvent>>>,
        }

        impl SemanticEventSink for RecordingSemanticSink {
            fn on_semantic_event(&mut self, event: SemanticEvent) {
                self.events.lock().unwrap().push(event);
            }
        }

        fn kinds_of(events: &[SemanticEvent]) -> Vec<&'static str> {
            events.iter().map(|e| e.kind_str()).collect()
        }

        #[test]
        fn claude_parser_emits_semantic_events() {
            let events = Arc::new(Mutex::new(Vec::new()));
            let sink = RecordingSemanticSink {
                events: events.clone(),
            };
            let mut parser =
                create_semantic_parser(Provider::Claude, sink, ParserConfig::default());

            parser
                .feed_line(r#"{"type":"init","session_id":"s1","model":"claude"}"#);
            parser
                .feed_line(r#"{"type":"assistant","content":[{"type":"text","text":"Hello"}]}"#);
            parser
                .feed_line(r#"{"type":"tool_use","id":"t1","name":"bash","input":{"cmd":"ls"}}"#);
            parser
                .feed_line(r#"{"type":"tool_result","tool_use_id":"t1","content":"ok"}"#);

            let collected = events.lock().unwrap().clone();
            let kinds = kinds_of(&collected);
            assert!(kinds.contains(&"session_start"), "kinds = {kinds:?}");
            assert!(kinds.contains(&"output_text"), "kinds = {kinds:?}");
            assert!(kinds.contains(&"tool_call"), "kinds = {kinds:?}");
            assert!(kinds.contains(&"tool_result"), "kinds = {kinds:?}");

            let summary = parser.finish(0);
            assert_eq!(summary.provider, Provider::Claude);
        }

        #[test]
        fn malformed_json_emits_warning_instead_of_error() {
            let events = Arc::new(Mutex::new(Vec::new()));
            let sink = RecordingSemanticSink {
                events: events.clone(),
            };
            let mut parser =
                create_semantic_parser(Provider::Claude, sink, ParserConfig::default());

            parser.feed_line("not json {{{");

            let collected = events.lock().unwrap().clone();
            assert!(
                collected
                    .iter()
                    .any(|e| matches!(e, SemanticEvent::Warning { .. })),
                "expected a Warning event, got {:?}",
                kinds_of(&collected)
            );
        }

        #[test]
        fn thinking_delta_becomes_reasoning_event() {
            let events = Arc::new(Mutex::new(Vec::new()));
            let sink = RecordingSemanticSink {
                events: events.clone(),
            };
            let mut parser =
                create_semantic_parser(Provider::Claude, sink, ParserConfig::default());

            parser
                .feed_line(
                    r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"pondering"}}"#,
                );

            let collected = events.lock().unwrap().clone();
            assert!(collected.iter().any(
                |e| matches!(e, SemanticEvent::Reasoning { text, .. } if text == "pondering")
            ));
        }

        #[test]
        fn rate_limit_becomes_warning_event() {
            let events = Arc::new(Mutex::new(Vec::new()));
            let sink = RecordingSemanticSink {
                events: events.clone(),
            };
            let mut parser =
                create_semantic_parser(Provider::Claude, sink, ParserConfig::default());

            parser
                .feed_line(
                    r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"Rate limit"}"#,
                );

            let collected = events.lock().unwrap().clone();
            assert!(collected.iter().any(
                |e| matches!(e, SemanticEvent::Warning { message, .. } if message == "Rate limit")
            ));
        }
    }
}
