pub mod badges;
pub mod claude_semantic;
pub mod codex_semantic;
pub mod gemini_semantic;
pub mod kimi_semantic;
pub mod logs;
pub mod opencode_semantic;
pub mod parser;
pub mod path_link;
pub mod progress;
pub mod prompt_timing;
pub mod protocol;
pub mod qwen_semantic;
pub mod reporting;
pub mod semantic;
pub mod stderr;
pub mod summary;
pub mod thinking;
pub mod token_usage;
pub mod tool_display;

use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::events::Provider;
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
}

/// Optional configuration for parser construction.
#[derive(Debug, Clone, Default)]
pub struct ParserConfig {
    /// External model name (for providers like OpenCode that don't report it in stream).
    pub model: Option<String>,
}

/// Create a semantic-event parser for a provider.
///
/// Every supported provider has a native [`SemanticStreamParser`]
/// implementation that emits [`SemanticEvent`]s directly. Providers without
/// a structured stream format fall back to the Claude parser as a degenerate
/// no-op.
pub fn create_semantic_parser<S: SemanticEventSink + 'static>(
    provider: Provider,
    sink: S,
    config: ParserConfig,
) -> Box<dyn SemanticStreamParser> {
    match provider {
        Provider::Claude => Box::new(claude_semantic::ClaudeSemanticStreamParser::new(sink)),
        Provider::Codex => Box::new(codex_semantic::CodexSemanticStreamParser::new(
            sink,
            config.model.clone(),
        )),
        Provider::Gemini => Box::new(gemini_semantic::GeminiSemanticStreamParser::new(sink)),
        Provider::OpenCode => Box::new(opencode_semantic::OpenCodeSemanticStreamParser::new(
            sink,
            config.model.clone(),
        )),
        Provider::KimiCode => Box::new(kimi_semantic::KimiSemanticStreamParser::new(sink)),
        Provider::QwenCode => Box::new(qwen_semantic::QwenSemanticStreamParser::new(sink)),
        _ => Box::new(claude_semantic::ClaudeSemanticStreamParser::new(sink)),
    }
}

/// Return the stream protocol used by a provider, if supported.
pub fn stream_protocol_for(provider: Provider) -> Option<StreamProtocol> {
    match provider {
        Provider::Claude => Some(StreamProtocol::StreamJson),
        Provider::Codex => Some(StreamProtocol::Jsonl),
        Provider::Gemini => Some(StreamProtocol::StreamJson),
        Provider::KimiCode => Some(StreamProtocol::StreamJson),
        Provider::OpenCode => Some(StreamProtocol::Ndjson),
        Provider::QwenCode => Some(StreamProtocol::StreamJson),
        _ => None,
    }
}

pub(crate) fn trace_parser_event(provider: Provider, event_type: &str, line_num: usize) {
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
    trace!(
        provider = %provider,
        session_id = session_id.unwrap_or(""),
        model = model.unwrap_or(""),
        "updated stream session metadata"
    );
}

pub(crate) fn trace_tool_event(provider: Provider, tool_calls: u32, tool_name: Option<&str>) {
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
        use crate::events::Provider;

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
                .feed_line(r#"{"type":"init","session_id":"s1","model":"claude"}"#)
                .unwrap();
            parser
                .feed_line(r#"{"type":"assistant","content":[{"type":"text","text":"Hello"}]}"#)
                .unwrap();
            parser
                .feed_line(r#"{"type":"tool_use","id":"t1","name":"bash","input":{"cmd":"ls"}}"#)
                .unwrap();
            parser
                .feed_line(r#"{"type":"tool_result","tool_use_id":"t1","content":"ok"}"#)
                .unwrap();

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

            let result = parser.feed_line("not json {{{");
            assert!(result.is_ok());

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
                )
                .unwrap();

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
                )
                .unwrap();

            let collected = events.lock().unwrap().clone();
            assert!(collected.iter().any(
                |e| matches!(e, SemanticEvent::Warning { message, .. } if message == "Rate limit")
            ));
        }
    }
}
