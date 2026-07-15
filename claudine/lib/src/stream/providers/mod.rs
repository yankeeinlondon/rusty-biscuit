pub mod antigravity;
pub mod claude;
mod common;
pub mod codex;
pub mod gemini;
pub mod kimi;
pub mod opencode;
pub mod pi;
pub mod qwen;
/// Generated per-provider error-classification vocabulary (`claudine-gen`).
/// Every parser's `classify_error*` consults [`vocabulary::error_keywords`]
/// for its runtime provider identity.
mod vocabulary;
#[cfg(test)]
mod vocabulary_tests;

// Re-exports so the moved parser files can keep their original `super::`
// references without modification.  `super` inside `providers/{name}.rs`
// resolves to this module, so we mirror the items that previously lived
// directly in `stream/`.
pub(crate) use super::{
    ensure_message_newline, parser, protocol, semantic, summary, token_usage, trace_malformed_line,
    trace_parser_event, trace_parser_finish, trace_session_metadata, trace_summary_update,
    trace_tool_event,
};

use crate::provider::BoxedSemanticEventSink;
use crate::provider_id::Provider;
use crate::stream::ParserConfig;
use crate::stream::parser::SemanticStreamParser;

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
        Provider::OpenCode => Box::new(opencode::OpenCodeSemanticStreamParser::new(
            sink,
            config.model,
            Provider::OpenCode,
        )
        .expect("OpenCode parser accepts the OpenCode identity")),
        Provider::QwenCode => Box::new(qwen::QwenSemanticStreamParser::new(sink)),
        // Kilo Code is an OpenCode fork; its `--format json` stream is
        // OpenCode-shaped NDJSON, parsed by the OpenCode parser unchanged —
        // but stamped with Kilo identity so it selects Kilo's own error
        // vocabulary rather than OpenCode's.
        Provider::Kilo => Box::new(opencode::OpenCodeSemanticStreamParser::new(
            sink,
            config.model,
            Provider::Kilo,
        )
        .expect("OpenCode parser accepts the Kilo identity")),
        // Pi is a bespoke provider with its own `--mode json` NDJSON format.
        Provider::Pi => Box::new(pi::PiSemanticStreamParser::new(sink, config.model)),
        // Antigravity's `--output-format json` print mode emits ONE buffered
        // JSON envelope (not a line stream); its bespoke parser accumulates and
        // parses that single object.
        Provider::Antigravity => Box::new(
            antigravity::AntigravitySemanticStreamParser::new(sink, config.model),
        ),
        _ => Box::new(claude::ClaudeSemanticStreamParser::new(sink)),
    }
}
