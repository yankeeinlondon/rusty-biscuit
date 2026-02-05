mod claude;
mod codex;
mod gemini;
mod opencode;

use serde_json::Value;

use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider};

/// Trait for provider-specific event adapters.
///
/// Each adapter translates raw provider JSON into normalized
/// `AgenticEvent` + `EventMeta` pairs.
pub trait ProviderAdapter {
    /// Which provider this adapter handles.
    fn provider(&self) -> Provider;

    /// Parse a raw provider event payload into a normalized event.
    ///
    /// Returns `None` for unknown or unsupported event types.
    fn parse_event(
        &self,
        raw: &Value,
        env: &EnvironmentContext,
    ) -> Option<(AgenticEvent, EventMeta)>;
}

/// Create the appropriate adapter for a given provider.
///
/// ## Panics
///
/// Panics if called with `Provider::Goose`, `Provider::KimiCode`, or
/// `Provider::QwenCode` as these providers do not yet have adapter implementations.
pub fn adapter_for(provider: Provider) -> Box<dyn ProviderAdapter> {
    match provider {
        Provider::Claude => Box::new(claude::ClaudeAdapter),
        Provider::Codex => Box::new(codex::CodexAdapter),
        Provider::Gemini => Box::new(gemini::GeminiAdapter),
        Provider::Goose => unimplemented!("Goose adapter not yet implemented"),
        Provider::KimiCode => unimplemented!("Kimi Code adapter not yet implemented"),
        Provider::OpenCode => Box::new(opencode::OpenCodeAdapter),
        Provider::QwenCode => unimplemented!("Qwen Code adapter not yet implemented"),
    }
}
