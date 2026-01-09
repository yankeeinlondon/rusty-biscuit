use ai_pipeline::rigging::providers::models::build::enum_name::enum_variant_name_from_wire_id;
use std::collections::HashSet;

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

        format!(
            r#"//! Auto-generated provider model enum
//!
//! Generated: {timestamp}
//! Generator: gen-models v{version}
//! Provider: {provider}
//!
//! Do not edit manually.

use model_id::ModelId;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
pub enum {enum_name} {{
{variants}
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}}
"#,
            timestamp = chrono::Utc::now().to_rfc3339(),
            version = env!("CARGO_PKG_VERSION"),
            provider = self.provider_name,
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
}
