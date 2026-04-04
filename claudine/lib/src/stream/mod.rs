pub mod claude;
pub mod codex;
pub mod gemini;
pub mod kimi;
pub mod opencode;
pub mod parser;
pub mod qwen;
pub mod reporting;
pub mod stderr;
pub mod summary;
pub mod token_usage;

use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::events::Provider;
use parser::{StreamEventSink, StreamParser};

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

/// Create the appropriate parser for a provider.
pub fn create_parser(
    provider: Provider,
    sink: impl StreamEventSink + 'static,
    config: ParserConfig,
) -> Box<dyn StreamParser> {
    match provider {
        Provider::Claude => Box::new(claude::ClaudeStreamParser::new(sink)),
        Provider::Codex => Box::new(codex::CodexStreamParser::new(sink, config.model)),
        Provider::Gemini => Box::new(gemini::GeminiStreamParser::new(sink)),
        Provider::KimiCode => Box::new(kimi::KimiStreamParser::new(sink)),
        Provider::OpenCode => Box::new(opencode::OpenCodeStreamParser::new(sink, config.model)),
        Provider::QwenCode => Box::new(qwen::QwenStreamParser::new(sink)),
        _ => Box::new(claude::ClaudeStreamParser::new(sink)),
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
    use parser::NullSink;

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
    fn create_parser_returns_correct_provider() {
        let providers = [
            Provider::Claude,
            Provider::Codex,
            Provider::Gemini,
            Provider::KimiCode,
            Provider::OpenCode,
            Provider::QwenCode,
        ];
        for provider in &providers {
            let parser = create_parser(*provider, NullSink, ParserConfig::default());
            let summary = parser.finish(0);
            assert_eq!(summary.provider, *provider);
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
}
