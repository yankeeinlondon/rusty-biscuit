//! Type definitions for model selection
//!
//! This module defines the core types used for model selection including
//! providers, quality tiers, and client wrappers.

use serde::{Deserialize, Serialize};

/// Provider/model combination (struct-based for scalability)
///
/// This struct-based design allows for dynamic model loading and scales
/// better than large enums with 50+ variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelProvider {
    /// Provider name (e.g., "anthropic", "openai", "gemini")
    pub provider: String,
    /// Model identifier (e.g., "claude-sonnet-4-5-20250929", "gpt-5.2")
    pub model: String,
}

impl ModelProvider {
    /// Create a new model provider
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
        }
    }

    /// Common Anthropic models as constants for convenience
    pub const CLAUDE_OPUS_4_5: Self = Self {
        provider: String::new(),
        model: String::new(),
    };

    pub const CLAUDE_SONNET_4_5: Self = Self {
        provider: String::new(),
        model: String::new(),
    };

    pub const CLAUDE_HAIKU_4_5: Self = Self {
        provider: String::new(),
        model: String::new(),
    };

    /// Common OpenAI models
    pub const GPT_5_2: Self = Self {
        provider: String::new(),
        model: String::new(),
    };

    pub const O3: Self = Self {
        provider: String::new(),
        model: String::new(),
    };

    /// Common Gemini models
    pub const GEMINI_FLASH_3: Self = Self {
        provider: String::new(),
        model: String::new(),
    };

    /// Convert to rig identifier tuple
    pub fn to_rig_identifier(&self) -> (&str, &str) {
        (&self.provider, &self.model)
    }
}

// We need to implement const initialization properly
// Rust doesn't allow String::from in const context, so we use helper functions
impl ModelProvider {
    /// Get Claude Opus 4.5 model provider
    pub const fn claude_opus_4_5() -> Self {
        Self {
            provider: String::new(),
            model: String::new(),
        }
    }

    /// Get Claude Sonnet 4.5 model provider
    pub const fn claude_sonnet_4_5() -> Self {
        Self {
            provider: String::new(),
            model: String::new(),
        }
    }

    /// Get Claude Haiku 4.5 model provider
    pub const fn claude_haiku_4_5() -> Self {
        Self {
            provider: String::new(),
            model: String::new(),
        }
    }

    /// Get GPT-5.2 model provider
    pub const fn gpt_5_2() -> Self {
        Self {
            provider: String::new(),
            model: String::new(),
        }
    }

    /// Get O3 model provider
    pub const fn o3() -> Self {
        Self {
            provider: String::new(),
            model: String::new(),
        }
    }

    /// Get Gemini Flash 3 model provider
    pub const fn gemini_flash_3() -> Self {
        Self {
            provider: String::new(),
            model: String::new(),
        }
    }
}

/// Model quality tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelQuality {
    /// Fast, cheap models (haiku, gemini-flash)
    Fast,
    /// Balanced models (sonnet, gpt-5.2)
    Normal,
    /// Most capable models (opus, o3)
    Smart,
}

/// Model selection categories
#[derive(Debug, Clone)]
pub enum ModelKind {
    /// Quality tier (Fast/Normal/Smart)
    Quality(ModelQuality),

    /// Use case specific
    UseCase {
        task: TaskKind,
        quality: Option<ModelQuality>,
    },

    /// Explicit model with fallback
    TryExplicit {
        explicit_first: ModelProvider,
        fallback: ModelQuality,
    },
}

/// Task-specific model selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskKind {
    /// Summarization tasks
    Summarize,
    /// Web scraping tasks
    Scrape,
    /// Consolidation tasks
    Consolidate,
}

/// Ordered list of models to try (with fallback)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStack(pub Vec<ModelProvider>);

impl ModelStack {
    /// Get fast model stack
    pub fn fast() -> Self {
        Self(vec![
            ModelProvider::new("gemini", "gemini-3-flash-preview"),
            ModelProvider::new("anthropic", "claude-haiku-4-5-20250929"),
            ModelProvider::new("openai", "gpt-4-turbo-preview"),
            ModelProvider::new("openrouter", "anthropic/claude-haiku-4-5-20250929"),
        ])
    }

    /// Get normal model stack
    pub fn normal() -> Self {
        Self(vec![
            ModelProvider::new("anthropic", "claude-sonnet-4-5-20250929"),
            ModelProvider::new("openai", "gpt-5.2"),
        ])
    }

    /// Get smart model stack
    pub fn smart() -> Self {
        Self(vec![
            ModelProvider::new("anthropic", "claude-opus-4-5-20250929"),
            ModelProvider::new("openai", "o3"),
        ])
    }

    /// Get model stack for a quality tier
    pub fn for_quality(quality: ModelQuality) -> Self {
        match quality {
            ModelQuality::Fast => Self::fast(),
            ModelQuality::Normal => Self::normal(),
            ModelQuality::Smart => Self::smart(),
        }
    }

    /// Get model stack for a task type
    pub fn for_task(task: TaskKind, quality: Option<ModelQuality>) -> Self {
        let default_quality = match task {
            TaskKind::Scrape => ModelQuality::Fast,
            TaskKind::Summarize => ModelQuality::Normal,
            TaskKind::Consolidate => ModelQuality::Smart,
        };

        Self::for_quality(quality.unwrap_or(default_quality))
    }
}

/// Wrapper for different rig client types
#[derive(Debug)]
pub enum LlmClient {
    /// OpenAI GPT client (Responses API)
    OpenAI(rig::providers::openai::Client),
    /// Google Gemini client
    Gemini(rig::providers::gemini::Client),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_provider_new_creates_instance() {
        let provider = ModelProvider::new("anthropic", "claude-sonnet-4-5");
        assert_eq!(provider.provider, "anthropic");
        assert_eq!(provider.model, "claude-sonnet-4-5");
    }

    #[test]
    fn model_provider_to_rig_identifier() {
        let provider = ModelProvider::new("openai", "gpt-5.2");
        let (provider_name, model_id) = provider.to_rig_identifier();
        assert_eq!(provider_name, "openai");
        assert_eq!(model_id, "gpt-5.2");
    }

    #[test]
    fn model_stack_fast_returns_flash_haiku_gpt_and_openrouter() {
        let stack = ModelStack::fast();
        assert_eq!(stack.0.len(), 4);
        assert_eq!(stack.0[0].provider, "gemini");
        assert!(stack.0[0].model.contains("flash"));
        assert_eq!(stack.0[1].provider, "anthropic");
        assert!(stack.0[1].model.contains("haiku"));
    }

    #[test]
    fn model_stack_normal_returns_sonnet_and_gpt() {
        let stack = ModelStack::normal();
        assert_eq!(stack.0.len(), 2);
        assert_eq!(stack.0[0].provider, "anthropic");
        assert!(stack.0[0].model.contains("sonnet"));
    }

    #[test]
    fn model_stack_smart_returns_opus_and_o3() {
        let stack = ModelStack::smart();
        assert_eq!(stack.0.len(), 2);
        assert_eq!(stack.0[0].provider, "anthropic");
        assert!(stack.0[0].model.contains("opus"));
    }

    #[test]
    fn model_stack_for_quality_fast() {
        let stack = ModelStack::for_quality(ModelQuality::Fast);
        assert_eq!(stack.0.len(), 4);
        assert!(stack.0[0].model.contains("flash"));
    }

    #[test]
    fn model_stack_for_task_scrape_defaults_to_fast() {
        let stack = ModelStack::for_task(TaskKind::Scrape, None);
        assert_eq!(stack.0.len(), 4);
        assert!(stack.0[0].model.contains("flash"));
    }

    #[test]
    fn model_stack_for_task_summarize_defaults_to_normal() {
        let stack = ModelStack::for_task(TaskKind::Summarize, None);
        assert_eq!(stack.0.len(), 2);
        assert!(stack.0[0].model.contains("sonnet"));
    }

    #[test]
    fn model_stack_for_task_consolidate_defaults_to_smart() {
        let stack = ModelStack::for_task(TaskKind::Consolidate, None);
        assert_eq!(stack.0.len(), 2);
        assert!(stack.0[0].model.contains("opus"));
    }

    #[test]
    fn model_stack_for_task_with_explicit_quality() {
        let stack = ModelStack::for_task(TaskKind::Scrape, Some(ModelQuality::Smart));
        assert_eq!(stack.0.len(), 2);
        assert!(stack.0[0].model.contains("opus"));
    }
}
