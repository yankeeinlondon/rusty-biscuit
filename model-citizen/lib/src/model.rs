//! Core model types for unified model representation across runners.
//!
//! This module provides the `UnifiedModel` struct that normalizes model information
//! from different runners (Ollama, LM Studio, Llama.cpp) into a common format.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A unified representation of a model across different runners.
///
/// This struct normalizes model metadata from Ollama, LM Studio, and Llama.cpp
/// into a common format for display and management.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedModel {
    /// Unique identifier for this model (format: `source:name:quantization`).
    pub id: String,

    /// Human-readable name of the model.
    pub name: String,

    /// Size of the model file in bytes.
    pub size_bytes: u64,

    /// Quantization type (e.g., Q4_0, Q5_1, Q8_0).
    pub quantization: QuantizationType,

    /// Model architecture (e.g., Llama, Mistral, Qwen).
    pub architecture: ModelArchitecture,

    /// Which runner this model came from.
    pub source: ModelSource,

    /// Filesystem path to the model file.
    pub path: PathBuf,
}

impl UnifiedModel {
    /// Creates a new unified model with the given parameters.
    ///
    /// The ID is automatically generated from source, name, and quantization.
    pub fn new(
        name: impl Into<String>,
        size_bytes: u64,
        quantization: QuantizationType,
        architecture: ModelArchitecture,
        source: ModelSource,
        path: impl Into<PathBuf>,
    ) -> Self {
        let name = name.into();
        let id = format!("{}:{}:{}", source.as_str(), name, quantization.as_str());
        Self {
            id,
            name,
            size_bytes,
            quantization,
            architecture,
            source,
            path: path.into(),
        }
    }

    /// Returns the size formatted as a human-readable string (e.g., "4.2 GB").
    #[must_use]
    pub fn size_display(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.size_bytes >= GB {
            format!("{:.1} GB", self.size_bytes as f64 / GB as f64)
        } else if self.size_bytes >= MB {
            format!("{:.1} MB", self.size_bytes as f64 / MB as f64)
        } else if self.size_bytes >= KB {
            format!("{:.1} KB", self.size_bytes as f64 / KB as f64)
        } else {
            format!("{} B", self.size_bytes)
        }
    }
}

/// Quantization type for GGUF models.
///
/// Determines the precision and size tradeoff for model weights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuantizationType {
    /// 4-bit quantization, type 0.
    Q4_0,
    /// 4-bit quantization, type 1.
    Q4_1,
    /// 4-bit K-quant, medium.
    Q4Km,
    /// 4-bit K-quant, small.
    Q4Ks,
    /// 5-bit quantization, type 0.
    Q5_0,
    /// 5-bit quantization, type 1.
    Q5_1,
    /// 5-bit K-quant, medium.
    Q5Km,
    /// 5-bit K-quant, small.
    Q5Ks,
    /// 6-bit K-quant.
    Q6K,
    /// 8-bit quantization, type 0.
    Q8_0,
    /// 16-bit floating point.
    F16,
    /// 32-bit floating point (full precision).
    F32,
    /// IQ1 series (1-bit).
    Iq1S,
    /// IQ2 series (2-bit, extra small).
    Iq2Xxs,
    /// IQ2 series (2-bit, extra small).
    Iq2Xs,
    /// IQ2 series (2-bit, small).
    Iq2S,
    /// IQ3 series (3-bit, extra small).
    Iq3Xxs,
    /// IQ3 series (3-bit, extra small).
    Iq3Xs,
    /// IQ3 series (3-bit, small).
    Iq3S,
    /// IQ4 series (4-bit, extra small).
    Iq4Xs,
    /// IQ4 series (4-bit, non-linear).
    Iq4Nl,
    /// 16-bit brain floating point (MLX).
    Bf16,
    /// 4-bit quantization (MLX).
    Bit4,
    /// 6-bit quantization (MLX).
    Bit6,
    /// 8-bit quantization (MLX).
    Bit8,
    /// Microsoft MXFP4 (MLX).
    Mxfp4,
    /// Unknown or unsupported quantization.
    #[default]
    Unknown,
}

impl QuantizationType {
    /// Returns the string representation of this quantization type.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Q4_0 => "Q4_0",
            Self::Q4_1 => "Q4_1",
            Self::Q4Km => "Q4_K_M",
            Self::Q4Ks => "Q4_K_S",
            Self::Q5_0 => "Q5_0",
            Self::Q5_1 => "Q5_1",
            Self::Q5Km => "Q5_K_M",
            Self::Q5Ks => "Q5_K_S",
            Self::Q6K => "Q6_K",
            Self::Q8_0 => "Q8_0",
            Self::F16 => "F16",
            Self::F32 => "F32",
            Self::Iq1S => "IQ1_S",
            Self::Iq2Xxs => "IQ2_XXS",
            Self::Iq2Xs => "IQ2_XS",
            Self::Iq2S => "IQ2_S",
            Self::Iq3Xxs => "IQ3_XXS",
            Self::Iq3Xs => "IQ3_XS",
            Self::Iq3S => "IQ3_S",
            Self::Iq4Xs => "IQ4_XS",
            Self::Iq4Nl => "IQ4_NL",
            Self::Bf16 => "BF16",
            Self::Bit4 => "4bit",
            Self::Bit6 => "6bit",
            Self::Bit8 => "8bit",
            Self::Mxfp4 => "MXFP4",
            Self::Unknown => "unknown",
        }
    }

    /// Maps an MLX quantization bits value to a `QuantizationType`.
    ///
    /// MLX models store quantization precision as an integer `bits` field
    /// in their `config.json`.
    #[must_use]
    pub fn from_mlx_bits(bits: u64) -> Self {
        match bits {
            4 => Self::Bit4,
            6 => Self::Bit6,
            8 => Self::Bit8,
            16 => Self::Bf16,
            _ => Self::Unknown,
        }
    }

    /// Parses a quantization type from a string (case-insensitive).
    ///
    /// Handles common variations like "Q4_K_M", "q4km", "Q4-K-M".
    #[must_use]
    pub fn from_str_loose(s: &str) -> Self {
        let normalized = s.to_uppercase().replace(['-', '_'], "");
        match normalized.as_str() {
            "Q40" => Self::Q4_0,
            "Q41" => Self::Q4_1,
            "Q4KM" => Self::Q4Km,
            "Q4KS" => Self::Q4Ks,
            "Q50" => Self::Q5_0,
            "Q51" => Self::Q5_1,
            "Q5KM" => Self::Q5Km,
            "Q5KS" => Self::Q5Ks,
            "Q6K" => Self::Q6K,
            "Q80" => Self::Q8_0,
            "F16" => Self::F16,
            "F32" => Self::F32,
            "IQ1S" => Self::Iq1S,
            "IQ2XXS" => Self::Iq2Xxs,
            "IQ2XS" => Self::Iq2Xs,
            "IQ2S" => Self::Iq2S,
            "IQ3XXS" => Self::Iq3Xxs,
            "IQ3XS" => Self::Iq3Xs,
            "IQ3S" => Self::Iq3S,
            "IQ4XS" => Self::Iq4Xs,
            "IQ4NL" => Self::Iq4Nl,
            "BF16" => Self::Bf16,
            "4BIT" => Self::Bit4,
            "6BIT" => Self::Bit6,
            "8BIT" => Self::Bit8,
            "MXFP4" => Self::Mxfp4,
            _ => Self::Unknown,
        }
    }
}

/// Model architecture family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelArchitecture {
    /// Llama family (Llama, Llama 2, Llama 3, Code Llama).
    Llama,
    /// Mistral family.
    Mistral,
    /// Qwen family.
    Qwen,
    /// Phi family (Microsoft).
    Phi,
    /// Gemma family (Google).
    Gemma,
    /// Command family (Cohere).
    Command,
    /// Yi family (01.AI).
    Yi,
    /// DeepSeek family.
    DeepSeek,
    /// StarCoder family.
    StarCoder,
    /// Unknown or unsupported architecture.
    #[default]
    Unknown,
}

impl ModelArchitecture {
    /// Returns the string representation of this architecture.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Llama => "llama",
            Self::Mistral => "mistral",
            Self::Qwen => "qwen",
            Self::Phi => "phi",
            Self::Gemma => "gemma",
            Self::Command => "command",
            Self::Yi => "yi",
            Self::DeepSeek => "deepseek",
            Self::StarCoder => "starcoder",
            Self::Unknown => "unknown",
        }
    }

    /// Attempts to detect architecture from a model name.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower.contains("llama") || lower.contains("codellama") {
            Self::Llama
        } else if lower.contains("mistral") || lower.contains("mixtral") {
            Self::Mistral
        } else if lower.contains("qwen") {
            Self::Qwen
        } else if lower.contains("phi") {
            Self::Phi
        } else if lower.contains("gemma") {
            Self::Gemma
        } else if lower.contains("command") {
            Self::Command
        } else if lower.contains("yi-") || lower.starts_with("yi") {
            Self::Yi
        } else if lower.contains("deepseek") {
            Self::DeepSeek
        } else if lower.contains("starcoder") {
            Self::StarCoder
        } else {
            Self::Unknown
        }
    }
}

/// Source runner for a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelSource {
    /// Ollama model manager.
    Ollama,
    /// LM Studio application.
    LmStudio,
    /// Raw Llama.cpp directory.
    LlamaCpp,
}

impl ModelSource {
    /// Returns the string representation of this source.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::LmStudio => "lmstudio",
            Self::LlamaCpp => "llamacpp",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unified_model_new_generates_id() {
        let model = UnifiedModel::new(
            "llama3",
            4_000_000_000,
            QuantizationType::Q4Km,
            ModelArchitecture::Llama,
            ModelSource::Ollama,
            "/models/llama3.gguf",
        );
        assert_eq!(model.id, "ollama:llama3:Q4_K_M");
        assert_eq!(model.name, "llama3");
    }

    #[test]
    fn size_display_formats_correctly() {
        let make_model = |size_bytes| {
            UnifiedModel::new(
                "test",
                size_bytes,
                QuantizationType::Q4_0,
                ModelArchitecture::Unknown,
                ModelSource::LlamaCpp,
                "/test.gguf",
            )
        };

        assert_eq!(make_model(500).size_display(), "500 B");
        assert_eq!(make_model(1024).size_display(), "1.0 KB");
        assert_eq!(make_model(1_500_000).size_display(), "1.4 MB");
        assert_eq!(make_model(4_500_000_000).size_display(), "4.2 GB");
    }

    #[test]
    fn quantization_from_str_loose_handles_variations() {
        assert_eq!(QuantizationType::from_str_loose("Q4_K_M"), QuantizationType::Q4Km);
        assert_eq!(QuantizationType::from_str_loose("q4km"), QuantizationType::Q4Km);
        assert_eq!(QuantizationType::from_str_loose("Q4-K-M"), QuantizationType::Q4Km);
        assert_eq!(QuantizationType::from_str_loose("Q5_0"), QuantizationType::Q5_0);
        assert_eq!(QuantizationType::from_str_loose("f16"), QuantizationType::F16);
        assert_eq!(QuantizationType::from_str_loose("garbage"), QuantizationType::Unknown);
    }

    #[test]
    fn architecture_from_name_detects_families() {
        assert_eq!(ModelArchitecture::from_name("llama-3-8b"), ModelArchitecture::Llama);
        assert_eq!(ModelArchitecture::from_name("codellama-7b"), ModelArchitecture::Llama);
        assert_eq!(ModelArchitecture::from_name("mistral-7b-v0.1"), ModelArchitecture::Mistral);
        assert_eq!(ModelArchitecture::from_name("qwen2-7b"), ModelArchitecture::Qwen);
        assert_eq!(ModelArchitecture::from_name("phi-3-mini"), ModelArchitecture::Phi);
        assert_eq!(ModelArchitecture::from_name("gemma-2b"), ModelArchitecture::Gemma);
        assert_eq!(ModelArchitecture::from_name("some-random-model"), ModelArchitecture::Unknown);
    }

    #[test]
    fn model_source_as_str() {
        assert_eq!(ModelSource::Ollama.as_str(), "ollama");
        assert_eq!(ModelSource::LmStudio.as_str(), "lmstudio");
        assert_eq!(ModelSource::LlamaCpp.as_str(), "llamacpp");
    }

    #[test]
    fn unified_model_serializes_to_json() {
        let model = UnifiedModel::new(
            "test",
            1024,
            QuantizationType::Q8_0,
            ModelArchitecture::Llama,
            ModelSource::Ollama,
            "/test.gguf",
        );
        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("\"quantization\":\"Q8_0\""));
        assert!(json.contains("\"architecture\":\"llama\""));
        assert!(json.contains("\"source\":\"ollama\""));
    }

    #[test]
    fn quantization_from_str_loose_handles_mlx() {
        assert_eq!(QuantizationType::from_str_loose("BF16"), QuantizationType::Bf16);
        assert_eq!(QuantizationType::from_str_loose("bf16"), QuantizationType::Bf16);
        assert_eq!(QuantizationType::from_str_loose("4bit"), QuantizationType::Bit4);
        assert_eq!(QuantizationType::from_str_loose("6BIT"), QuantizationType::Bit6);
        assert_eq!(QuantizationType::from_str_loose("8bit"), QuantizationType::Bit8);
        assert_eq!(QuantizationType::from_str_loose("MXFP4"), QuantizationType::Mxfp4);
        assert_eq!(QuantizationType::from_str_loose("mxfp4"), QuantizationType::Mxfp4);
    }

    #[test]
    fn from_mlx_bits_maps_correctly() {
        assert_eq!(QuantizationType::from_mlx_bits(4), QuantizationType::Bit4);
        assert_eq!(QuantizationType::from_mlx_bits(6), QuantizationType::Bit6);
        assert_eq!(QuantizationType::from_mlx_bits(8), QuantizationType::Bit8);
        assert_eq!(QuantizationType::from_mlx_bits(16), QuantizationType::Bf16);
        assert_eq!(QuantizationType::from_mlx_bits(3), QuantizationType::Unknown);
        assert_eq!(QuantizationType::from_mlx_bits(0), QuantizationType::Unknown);
    }

    #[test]
    fn mlx_quantization_as_str() {
        assert_eq!(QuantizationType::Bf16.as_str(), "BF16");
        assert_eq!(QuantizationType::Bit4.as_str(), "4bit");
        assert_eq!(QuantizationType::Bit6.as_str(), "6bit");
        assert_eq!(QuantizationType::Bit8.as_str(), "8bit");
        assert_eq!(QuantizationType::Mxfp4.as_str(), "MXFP4");
    }

    #[test]
    fn unified_model_deserializes_from_json() {
        let json = r#"{
            "id": "ollama:test:Q8_0",
            "name": "test",
            "size_bytes": 1024,
            "quantization": "Q8_0",
            "architecture": "llama",
            "source": "ollama",
            "path": "/test.gguf"
        }"#;
        let model: UnifiedModel = serde_json::from_str(json).unwrap();
        assert_eq!(model.name, "test");
        assert_eq!(model.quantization, QuantizationType::Q8_0);
    }
}
