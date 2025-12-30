//! Type definitions for provider discovery

use serde::{Deserialize, Serialize};

/// Represents a single LLM provider and model combination
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LlmEntry {
    /// Provider name (e.g., "openai", "anthropic")
    pub provider: String,
    /// Model identifier (e.g., "gpt-5.2", "claude-sonnet-4.5")
    pub model: String,
}

impl LlmEntry {
    /// Create a new LLM entry
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
        }
    }

    /// Get the combined provider/model identifier
    pub fn identifier(&self) -> String {
        format!("{}/{}", self.provider, self.model)
    }
}

/// Format for the generated provider list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProviderListFormat {
    /// JSON array of string literals: ["openai/gpt-5.2", "anthropic/claude-opus-4.5"]
    #[default]
    StringLiterals,
    /// Rust enum variants: Openai_Gpt_5_2, Anthropic_Claude_Opus_4_5
    RustEnum,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_entry_new() {
        let entry = LlmEntry::new("openai", "gpt-4");
        assert_eq!(entry.provider, "openai");
        assert_eq!(entry.model, "gpt-4");
    }

    #[test]
    fn llm_entry_identifier() {
        let entry = LlmEntry::new("anthropic", "claude-opus-4.5");
        assert_eq!(entry.identifier(), "anthropic/claude-opus-4.5");
    }

    #[test]
    fn provider_list_format_default() {
        assert_eq!(ProviderListFormat::default(), ProviderListFormat::StringLiterals);
    }
}
