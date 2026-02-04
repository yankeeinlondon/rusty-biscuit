mod claude;
mod codex;
mod gemini;
mod opencode;
mod roo;

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
pub fn adapter_for(provider: Provider) -> Box<dyn ProviderAdapter> {
    match provider {
        Provider::Claude => Box::new(claude::ClaudeAdapter),
        Provider::Codex => Box::new(codex::CodexAdapter),
        Provider::Gemini => Box::new(gemini::GeminiAdapter),
        Provider::OpenCode => Box::new(opencode::OpenCodeAdapter),
        Provider::RooCode => Box::new(roo::RooAdapter),
    }
}
