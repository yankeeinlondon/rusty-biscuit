//! Runtime metadata types for model capabilities and specifications.
//!
//! This module provides types for representing model metadata fetched from
//! external sources like the Parsera LLM Specs API. These types are used
//! at runtime to query model capabilities, context windows, and modalities.

use std::str::FromStr;

/// Input/output modality supported by a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modality {
    /// Text input/output
    Text,
    /// Image input/output
    Image,
    /// Audio input/output
    Audio,
    /// Video input/output
    Video,
    /// Embedding output
    Embeddings,
}

impl FromStr for Modality {
    type Err = ModalityParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "image" => Ok(Self::Image),
            "audio" => Ok(Self::Audio),
            "video" => Ok(Self::Video),
            "embeddings" | "embedding" => Ok(Self::Embeddings),
            _ => Err(ModalityParseError(s.to_string())),
        }
    }
}

impl std::fmt::Display for Modality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Image => write!(f, "image"),
            Self::Audio => write!(f, "audio"),
            Self::Video => write!(f, "video"),
            Self::Embeddings => write!(f, "embeddings"),
        }
    }
}

/// Error returned when parsing an unknown modality string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalityParseError(pub String);

impl std::fmt::Display for ModalityParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown modality: '{}'", self.0)
    }
}

impl std::error::Error for ModalityParseError {}

/// Input and output modalities supported by a model.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModelModalities {
    /// Modalities the model accepts as input.
    pub input: Vec<Modality>,
    /// Modalities the model can produce as output.
    pub output: Vec<Modality>,
}

impl ModelModalities {
    /// Creates a new empty modalities specification.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if the model supports a given input modality.
    #[must_use]
    pub fn supports_input(&self, modality: Modality) -> bool {
        self.input.contains(&modality)
    }

    /// Checks if the model supports a given output modality.
    #[must_use]
    pub fn supports_output(&self, modality: Modality) -> bool {
        self.output.contains(&modality)
    }
}

/// Metadata about a model fetched from external specification sources.
///
/// This struct contains rich metadata that may be populated from sources
/// like the Parsera LLM Specs API. All fields are optional since not all
/// models have complete metadata available.
#[derive(Debug, Clone, Default)]
pub struct ModelMetadata {
    /// Human-readable display name (e.g., "GPT-4o mini").
    pub display_name: Option<String>,

    /// Model family (e.g., "gpt-4o-mini", "claude-3").
    pub family: Option<String>,

    /// Maximum context window size in tokens.
    pub context_window: Option<u32>,

    /// Maximum output tokens the model can generate.
    pub max_output_tokens: Option<u32>,

    /// Input and output modalities supported by this model.
    pub modalities: Option<ModelModalities>,

    /// Capabilities like "function_calling", "structured_output", etc.
    pub capabilities: Vec<String>,
}

impl ModelMetadata {
    /// Creates a new empty metadata instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates metadata with just a display name.
    #[must_use]
    pub fn with_display_name(display_name: impl Into<String>) -> Self {
        Self {
            display_name: Some(display_name.into()),
            ..Default::default()
        }
    }

    /// Checks if this model supports a given input modality.
    #[must_use]
    pub fn supports_input(&self, modality: Modality) -> bool {
        self.modalities
            .as_ref()
            .map(|m| m.supports_input(modality))
            .unwrap_or(false)
    }

    /// Checks if this model supports a given output modality.
    #[must_use]
    pub fn supports_output(&self, modality: Modality) -> bool {
        self.modalities
            .as_ref()
            .map(|m| m.supports_output(modality))
            .unwrap_or(false)
    }

    /// Checks if this model has a specific capability.
    #[must_use]
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities
            .iter()
            .any(|c| c.eq_ignore_ascii_case(capability))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modality_from_str() {
        assert_eq!(Modality::from_str("text").unwrap(), Modality::Text);
        assert_eq!(Modality::from_str("TEXT").unwrap(), Modality::Text);
        assert_eq!(Modality::from_str("Image").unwrap(), Modality::Image);
        assert_eq!(Modality::from_str("audio").unwrap(), Modality::Audio);
        assert_eq!(Modality::from_str("video").unwrap(), Modality::Video);
        assert_eq!(
            Modality::from_str("embeddings").unwrap(),
            Modality::Embeddings
        );
        assert_eq!(
            Modality::from_str("embedding").unwrap(),
            Modality::Embeddings
        );
        assert!(Modality::from_str("unknown").is_err());
    }

    #[test]
    fn test_modality_display() {
        assert_eq!(Modality::Text.to_string(), "text");
        assert_eq!(Modality::Image.to_string(), "image");
        assert_eq!(Modality::Embeddings.to_string(), "embeddings");
    }

    #[test]
    fn test_model_modalities_supports() {
        let modalities = ModelModalities {
            input: vec![Modality::Text, Modality::Image],
            output: vec![Modality::Text],
        };

        assert!(modalities.supports_input(Modality::Text));
        assert!(modalities.supports_input(Modality::Image));
        assert!(!modalities.supports_input(Modality::Audio));
        assert!(modalities.supports_output(Modality::Text));
        assert!(!modalities.supports_output(Modality::Image));
    }

    #[test]
    fn test_model_metadata_supports_modality() {
        let metadata = ModelMetadata {
            modalities: Some(ModelModalities {
                input: vec![Modality::Text, Modality::Image],
                output: vec![Modality::Text],
            }),
            ..Default::default()
        };

        assert!(metadata.supports_input(Modality::Text));
        assert!(metadata.supports_input(Modality::Image));
        assert!(!metadata.supports_input(Modality::Audio));
        assert!(metadata.supports_output(Modality::Text));
        assert!(!metadata.supports_output(Modality::Image));
    }

    #[test]
    fn test_model_metadata_has_capability() {
        let metadata = ModelMetadata {
            capabilities: vec!["function_calling".to_string(), "structured_output".to_string()],
            ..Default::default()
        };

        assert!(metadata.has_capability("function_calling"));
        assert!(metadata.has_capability("FUNCTION_CALLING"));
        assert!(metadata.has_capability("structured_output"));
        assert!(!metadata.has_capability("vision"));
    }

    #[test]
    fn test_model_metadata_empty_returns_false() {
        let metadata = ModelMetadata::new();

        assert!(!metadata.supports_input(Modality::Text));
        assert!(!metadata.supports_output(Modality::Text));
        assert!(!metadata.has_capability("anything"));
    }
}
