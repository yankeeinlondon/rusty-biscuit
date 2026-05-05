pub mod claude;
pub mod codex;
pub mod gemini;
pub mod kimi;
pub mod opencode;
pub mod qwen;

// Re-exports so the moved parser files can keep their original `super::`
// references without modification.  `super` inside `providers/{name}.rs`
// resolves to this module, so we mirror the items that previously lived
// directly in `stream/`.
pub(crate) use super::{
    ensure_message_newline,
    parser,
    protocol,
    semantic,
    summary,
    token_usage,
    trace_malformed_line,
    trace_parser_event,
    trace_parser_finish,
    trace_session_metadata,
    trace_summary_update,
    trace_tool_event,
};

use crate::provider_id::Provider;
use crate::stream::parser::SemanticStreamParser;
use crate::provider::BoxedSemanticEventSink;
use crate::stream::ParserConfig;

/// Marker trait for semantic stream parsers.
///
/// All provider-specific parsers implement [`SemanticStreamParser`]; this
/// trait is a thin marker so callers can work with `Box<dyn SemanticParser>`.
pub trait SemanticParser: SemanticStreamParser + Send + 'static {}

impl<T: SemanticStreamParser + Send + 'static> SemanticParser for T {}

/// Construct a boxed semantic parser for the given provider.
pub fn for_provider(
    p: Provider,
    sink: BoxedSemanticEventSink,
    config: ParserConfig,
) -> Box<dyn SemanticParser> {
    match p {
        Provider::Claude => Box::new(claude::ClaudeSemanticStreamParser::new(sink)),
        Provider::Codex => Box::new(codex::CodexSemanticStreamParser::new(sink, config.model)),
        Provider::Gemini => Box::new(gemini::GeminiSemanticStreamParser::new(sink)),
        Provider::KimiCode => Box::new(kimi::KimiSemanticStreamParser::new(sink)),
        Provider::OpenCode => {
            Box::new(opencode::OpenCodeSemanticStreamParser::new(sink, config.model))
        }
        Provider::QwenCode => Box::new(qwen::QwenSemanticStreamParser::new(sink)),
        _ => Box::new(claude::ClaudeSemanticStreamParser::new(sink)),
    }
}
