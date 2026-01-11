use ai_pipeline::rigging::providers::models::build::enum_name::enum_variant_name_from_wire_id;
use std::collections::HashSet;

use std::borrow::Cow;

/// Provider metadata for documentation generation.
struct ProviderMeta<'a> {
    /// User-facing name for documentation
    display_name: Cow<'a, str>,
    /// Provider's website URL
    url: &'static str,
}

/// Returns provider metadata for documentation purposes.
fn provider_meta(provider_name: &str) -> ProviderMeta<'_> {
    match provider_name {
        "Anthropic" => ProviderMeta {
            display_name: Cow::Borrowed("Anthropic"),
            url: "https://anthropic.com",
        },
        "Deepseek" => ProviderMeta {
            display_name: Cow::Borrowed("DeepSeek"),
            url: "https://deepseek.com",
        },
        "Gemini" => ProviderMeta {
            display_name: Cow::Borrowed("Google Gemini"),
            url: "https://ai.google.dev",
        },
        "Groq" => ProviderMeta {
            display_name: Cow::Borrowed("Groq"),
            url: "https://groq.com",
        },
        "HuggingFace" => ProviderMeta {
            display_name: Cow::Borrowed("Hugging Face"),
            url: "https://huggingface.co",
        },
        "Mistral" => ProviderMeta {
            display_name: Cow::Borrowed("Mistral AI"),
            url: "https://mistral.ai",
        },
        "MoonshotAi" => ProviderMeta {
            display_name: Cow::Borrowed("Moonshot AI (Kimi)"),
            url: "https://moonshot.ai",
        },
        "Ollama" => ProviderMeta {
            display_name: Cow::Borrowed("Ollama"),
            url: "https://ollama.ai",
        },
        "OpenAi" => ProviderMeta {
            display_name: Cow::Borrowed("OpenAI"),
            url: "https://openai.com",
        },
        "OpenRouter" => ProviderMeta {
            display_name: Cow::Borrowed("OpenRouter"),
            url: "https://openrouter.ai",
        },
        "Xai" => ProviderMeta {
            display_name: Cow::Borrowed("xAI"),
            url: "https://x.ai",
        },
        "Zai" => ProviderMeta {
            display_name: Cow::Borrowed("Zhipu AI (Z.ai)"),
            url: "https://zhipuai.cn",
        },
        "ZenMux" => ProviderMeta {
            display_name: Cow::Borrowed("ZenMux"),
            url: "https://zenmux.ai",
        },
        _ => ProviderMeta {
            display_name: Cow::Owned(provider_name.to_string()),
            url: "#",
        },
    }
}

/// Generator for provider model enum files.
pub struct ModelEnumGenerator {
    provider_name: String,
    models: Vec<String>,
}

impl ModelEnumGenerator {
    /// Create a new generator for a provider.
    ///
    /// Models are deduplicated and sorted alphabetically.
    pub fn new(provider_name: String, models: Vec<String>) -> Self {
        // Deduplicate and sort
        let unique: HashSet<_> = models.into_iter().collect();
        let mut models: Vec<_> = unique.into_iter().collect();
        models.sort();
        Self {
            provider_name,
            models,
        }
    }

    /// Generate the Rust source code for the provider model enum.
    pub fn generate(&self) -> String {
        let enum_name = format!("ProviderModel{}", self.provider_name);
        let variants = self.generate_variants();
        let meta = provider_meta(&self.provider_name);

        format!(
            r#"//! Auto-generated provider model enum
//!
//! Generated: {timestamp}
//! Generator: gen-models v{version}
//! Provider: {provider}
//!
//! Do not edit manually.

use model_id::ModelId;

/// Models provided by [{display_name}](<{url}>).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
#[model_id_metadata(
    lookup = "super::metadata_generated::MODEL_METADATA",
    returns = "crate::models::model_metadata::ModelMetadata"
)]
pub enum {enum_name} {{
{variants}
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}}
"#,
            timestamp = chrono::Utc::now().to_rfc3339(),
            version = env!("CARGO_PKG_VERSION"),
            provider = self.provider_name,
            display_name = meta.display_name,
            url = meta.url,
            enum_name = enum_name,
            variants = variants,
        )
    }

    /// Generate variant declarations for all models.
    fn generate_variants(&self) -> String {
        self.models
            .iter()
            .map(|model_id| {
                let variant = enum_variant_name_from_wire_id(model_id);
                format!("    /// Model: `{}`\n    {},", model_id, variant)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns the number of models in this generator.
    pub fn model_count(&self) -> usize {
        self.models.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variant_generation() {
        let generator = ModelEnumGenerator::new(
            "OpenAi".to_string(),
            vec!["gpt-4o".to_string(), "gpt-4o-mini".to_string()],
        );
        let code = generator.generate();

        assert!(code.contains("Gpt__4o,"));
        assert!(code.contains("Gpt__4o__Mini,"));
        assert!(code.contains("Bespoke(String)"));
    }

    #[test]
    fn test_deduplication() {
        let generator = ModelEnumGenerator::new(
            "Test".to_string(),
            vec!["model-a".to_string(), "model-a".to_string()],
        );

        assert_eq!(generator.model_count(), 1);
    }

    #[test]
    fn test_sorting() {
        let generator = ModelEnumGenerator::new(
            "Test".to_string(),
            vec![
                "z-model".to_string(),
                "a-model".to_string(),
                "m-model".to_string(),
            ],
        );
        let code = generator.generate();

        // Verify alphabetical order in output
        let a_pos = code.find("A__Model").unwrap();
        let m_pos = code.find("M__Model").unwrap();
        let z_pos = code.find("Z__Model").unwrap();

        assert!(a_pos < m_pos);
        assert!(m_pos < z_pos);
    }

    /// Regression test: Generated enums must include provider documentation.
    ///
    /// Verifies that:
    /// 1. Known providers get a doc comment with display name and URL
    /// 2. Unknown providers get a fallback doc comment
    #[test]
    fn test_enum_has_provider_doc_comment() {
        // Test known provider
        let generator = ModelEnumGenerator::new(
            "OpenAi".to_string(),
            vec!["gpt-4o".to_string()],
        );
        let code = generator.generate();

        assert!(
            code.contains("/// Models provided by [OpenAI](<https://openai.com>)."),
            "Generated code should have provider doc comment with display name and URL"
        );

        // Test another known provider
        let generator = ModelEnumGenerator::new(
            "Anthropic".to_string(),
            vec!["claude-3".to_string()],
        );
        let code = generator.generate();

        assert!(
            code.contains("/// Models provided by [Anthropic](<https://anthropic.com>)."),
            "Generated code should have Anthropic doc comment"
        );

        // Test unknown provider falls back gracefully
        let generator = ModelEnumGenerator::new(
            "UnknownProvider".to_string(),
            vec!["model-x".to_string()],
        );
        let code = generator.generate();

        assert!(
            code.contains("/// Models provided by [UnknownProvider](<#>)."),
            "Unknown providers should use provider name with # URL"
        );
    }
}
