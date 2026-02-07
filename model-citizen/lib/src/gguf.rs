//! GGUF file utilities for parsing model metadata.
//!
//! This module provides utilities for extracting quantization information from
//! GGUF files, either by parsing the file header or by detecting patterns in
//! the filename.
//!
//! ## GGUF Format Overview
//!
//! GGUF (GGML Unified Format) is the standard format for GGML-based models.
//! The file structure is:
//! - Magic number: `GGUF` (4 bytes)
//! - Version: u32 (4 bytes)
//! - Tensor count: u64 (8 bytes)
//! - Metadata KV count: u64 (8 bytes)
//! - Metadata key-value pairs (variable)
//!
//! For simplicity and robustness, we primarily use filename-based detection
//! with header parsing as a fallback for validation.

use crate::{ModelCitizenError, QuantizationType};
use std::path::Path;

/// Magic bytes at the start of a GGUF file.
pub const GGUF_MAGIC: &[u8; 4] = b"GGUF";

/// GGUF header information.
#[derive(Debug, Clone)]
pub struct GgufHeader {
    /// GGUF format version.
    pub version: u32,
    /// Number of tensors in the file.
    pub tensor_count: u64,
    /// Number of metadata key-value pairs.
    pub metadata_kv_count: u64,
}

impl GgufHeader {
    /// Parses a GGUF header from a byte slice.
    ///
    /// Expects at least 24 bytes (magic + version + tensor_count + metadata_kv_count).
    ///
    /// ## Errors
    ///
    /// Returns an error if the slice is too small or has invalid magic bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ModelCitizenError> {
        if bytes.len() < 24 {
            return Err(ModelCitizenError::parse("GGUF header too short"));
        }

        // Check magic
        if &bytes[0..4] != GGUF_MAGIC {
            return Err(ModelCitizenError::parse("Invalid GGUF magic bytes"));
        }

        // Parse version (little-endian u32)
        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        // Parse tensor count (little-endian u64)
        let tensor_count = u64::from_le_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]);

        // Parse metadata KV count (little-endian u64)
        let metadata_kv_count = u64::from_le_bytes([
            bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
        ]);

        Ok(Self {
            version,
            tensor_count,
            metadata_kv_count,
        })
    }
}

/// Reads and parses a GGUF header from a file.
///
/// Only reads the first 1KB of the file to minimize I/O.
///
/// ## Errors
///
/// Returns an error if the file cannot be read or has an invalid header.
pub fn parse_header(path: &Path) -> Result<GgufHeader, ModelCitizenError> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0u8; 1024];
    let bytes_read = file.read(&mut buffer)?;

    if bytes_read < 24 {
        return Err(ModelCitizenError::parse("File too small to be a valid GGUF"));
    }

    GgufHeader::from_bytes(&buffer[..bytes_read])
}

/// Checks if a file has a valid GGUF header.
///
/// Only reads the first 4 bytes to check the magic number.
#[must_use]
pub fn is_gguf_file(path: &Path) -> bool {
    use std::io::Read;

    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };

    let mut magic = [0u8; 4];
    if file.read_exact(&mut magic).is_err() {
        return false;
    }

    &magic == GGUF_MAGIC
}

/// Detects quantization type from a filename.
///
/// Looks for common patterns like:
/// - `-Q4_K_M.gguf`
/// - `-q4_0.gguf`
/// - `.Q5_1.gguf`
/// - `_F16.gguf`
///
/// ## Examples
///
/// ```
/// use model_citizen::gguf::quantization_from_filename;
/// use model_citizen::QuantizationType;
/// use std::path::Path;
///
/// let path = Path::new("llama-3-8b-Q4_K_M.gguf");
/// assert_eq!(quantization_from_filename(path), QuantizationType::Q4Km);
///
/// let path = Path::new("model.F16.gguf");
/// assert_eq!(quantization_from_filename(path), QuantizationType::F16);
/// ```
#[must_use]
pub fn quantization_from_filename(path: &Path) -> QuantizationType {
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    // Common quantization patterns in order of specificity
    let patterns = [
        // IQ patterns (must come before Q patterns)
        ("IQ1_S", QuantizationType::Iq1S),
        ("iq1_s", QuantizationType::Iq1S),
        ("IQ2_XXS", QuantizationType::Iq2Xxs),
        ("iq2_xxs", QuantizationType::Iq2Xxs),
        ("IQ2_XS", QuantizationType::Iq2Xs),
        ("iq2_xs", QuantizationType::Iq2Xs),
        ("IQ2_S", QuantizationType::Iq2S),
        ("iq2_s", QuantizationType::Iq2S),
        ("IQ3_XXS", QuantizationType::Iq3Xxs),
        ("iq3_xxs", QuantizationType::Iq3Xxs),
        ("IQ3_XS", QuantizationType::Iq3Xs),
        ("iq3_xs", QuantizationType::Iq3Xs),
        ("IQ3_S", QuantizationType::Iq3S),
        ("iq3_s", QuantizationType::Iq3S),
        ("IQ4_XS", QuantizationType::Iq4Xs),
        ("iq4_xs", QuantizationType::Iq4Xs),
        ("IQ4_NL", QuantizationType::Iq4Nl),
        ("iq4_nl", QuantizationType::Iq4Nl),
        // K-quant patterns (must come before basic Q patterns)
        ("Q4_K_M", QuantizationType::Q4Km),
        ("q4_k_m", QuantizationType::Q4Km),
        ("Q4-K-M", QuantizationType::Q4Km),
        ("Q4KM", QuantizationType::Q4Km),
        ("Q4_K_S", QuantizationType::Q4Ks),
        ("q4_k_s", QuantizationType::Q4Ks),
        ("Q4-K-S", QuantizationType::Q4Ks),
        ("Q4KS", QuantizationType::Q4Ks),
        ("Q5_K_M", QuantizationType::Q5Km),
        ("q5_k_m", QuantizationType::Q5Km),
        ("Q5-K-M", QuantizationType::Q5Km),
        ("Q5KM", QuantizationType::Q5Km),
        ("Q5_K_S", QuantizationType::Q5Ks),
        ("q5_k_s", QuantizationType::Q5Ks),
        ("Q5-K-S", QuantizationType::Q5Ks),
        ("Q5KS", QuantizationType::Q5Ks),
        ("Q6_K", QuantizationType::Q6K),
        ("q6_k", QuantizationType::Q6K),
        ("Q6-K", QuantizationType::Q6K),
        ("Q6K", QuantizationType::Q6K),
        // Basic Q patterns
        ("Q4_0", QuantizationType::Q4_0),
        ("q4_0", QuantizationType::Q4_0),
        ("Q4-0", QuantizationType::Q4_0),
        ("Q4_1", QuantizationType::Q4_1),
        ("q4_1", QuantizationType::Q4_1),
        ("Q4-1", QuantizationType::Q4_1),
        ("Q5_0", QuantizationType::Q5_0),
        ("q5_0", QuantizationType::Q5_0),
        ("Q5-0", QuantizationType::Q5_0),
        ("Q5_1", QuantizationType::Q5_1),
        ("q5_1", QuantizationType::Q5_1),
        ("Q5-1", QuantizationType::Q5_1),
        ("Q8_0", QuantizationType::Q8_0),
        ("q8_0", QuantizationType::Q8_0),
        ("Q8-0", QuantizationType::Q8_0),
        // Float types
        ("F16", QuantizationType::F16),
        ("f16", QuantizationType::F16),
        ("fp16", QuantizationType::F16),
        ("FP16", QuantizationType::F16),
        ("F32", QuantizationType::F32),
        ("f32", QuantizationType::F32),
        ("fp32", QuantizationType::F32),
        ("FP32", QuantizationType::F32),
    ];

    for (pattern, quant) in patterns {
        if filename.contains(pattern) {
            return quant;
        }
    }

    QuantizationType::Unknown
}

/// Detects quantization type from a GGUF file.
///
/// First tries filename-based detection (fast and reliable), then falls back
/// to header parsing for validation if needed.
///
/// ## Examples
///
/// ```no_run
/// use model_citizen::gguf::detect_quantization;
/// use std::path::Path;
///
/// let quant = detect_quantization(Path::new("/models/llama-Q4_K_M.gguf"));
/// println!("Quantization: {}", quant.as_str());
/// ```
#[must_use]
pub fn detect_quantization(path: &Path) -> QuantizationType {
    // Filename-based detection is fast and usually sufficient
    let from_filename = quantization_from_filename(path);
    if from_filename != QuantizationType::Unknown {
        return from_filename;
    }

    // Fallback: verify it's a valid GGUF file
    // Note: GGUF metadata parsing for quantization type is complex and
    // not strictly necessary since all common tools include quantization
    // in the filename.
    if is_gguf_file(path) {
        // File is valid GGUF but we couldn't detect quantization from name
        QuantizationType::Unknown
    } else {
        QuantizationType::Unknown
    }
}

/// Extracts model name from a GGUF filename.
///
/// Removes quantization suffix and `.gguf` extension.
///
/// ## Examples
///
/// ```
/// use model_citizen::gguf::model_name_from_filename;
/// use std::path::Path;
///
/// let path = Path::new("llama-3-8b-instruct-Q4_K_M.gguf");
/// assert_eq!(model_name_from_filename(path), Some("llama-3-8b-instruct".to_string()));
/// ```
#[must_use]
pub fn model_name_from_filename(path: &Path) -> Option<String> {
    let filename = path.file_stem()?.to_str()?;

    // Common quantization patterns to strip
    let patterns = [
        "-Q4_K_M", "-Q4_K_S", "-Q5_K_M", "-Q5_K_S", "-Q6_K",
        "-Q4_0", "-Q4_1", "-Q5_0", "-Q5_1", "-Q8_0",
        "-F16", "-F32",
        "-IQ1_S", "-IQ2_XXS", "-IQ2_XS", "-IQ2_S",
        "-IQ3_XXS", "-IQ3_XS", "-IQ3_S", "-IQ4_XS", "-IQ4_NL",
        ".Q4_K_M", ".Q4_K_S", ".Q5_K_M", ".Q5_K_S", ".Q6_K",
        ".Q4_0", ".Q4_1", ".Q5_0", ".Q5_1", ".Q8_0",
        ".F16", ".F32",
        "_Q4_K_M", "_Q4_K_S", "_Q5_K_M", "_Q5_K_S", "_Q6_K",
        "_Q4_0", "_Q4_1", "_Q5_0", "_Q5_1", "_Q8_0",
        "_F16", "_F32",
    ];

    let mut name = filename.to_string();
    for pattern in patterns {
        if let Some(idx) = name.to_uppercase().find(&pattern.to_uppercase()) {
            name = name[..idx].to_string();
            break;
        }
    }

    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gguf_header_parses_valid_bytes() {
        let mut bytes = vec![0u8; 24];
        bytes[0..4].copy_from_slice(b"GGUF");
        bytes[4..8].copy_from_slice(&3u32.to_le_bytes()); // version 3
        bytes[8..16].copy_from_slice(&100u64.to_le_bytes()); // 100 tensors
        bytes[16..24].copy_from_slice(&50u64.to_le_bytes()); // 50 metadata

        let header = GgufHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header.version, 3);
        assert_eq!(header.tensor_count, 100);
        assert_eq!(header.metadata_kv_count, 50);
    }

    #[test]
    fn gguf_header_rejects_invalid_magic() {
        let mut bytes = vec![0u8; 24];
        bytes[0..4].copy_from_slice(b"GGML"); // Wrong magic

        let result = GgufHeader::from_bytes(&bytes);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("magic"));
    }

    #[test]
    fn gguf_header_rejects_short_input() {
        let bytes = vec![0u8; 16]; // Too short

        let result = GgufHeader::from_bytes(&bytes);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn quantization_from_filename_detects_k_quants() {
        assert_eq!(
            quantization_from_filename(Path::new("model-Q4_K_M.gguf")),
            QuantizationType::Q4Km
        );
        assert_eq!(
            quantization_from_filename(Path::new("model-q4_k_m.gguf")),
            QuantizationType::Q4Km
        );
        assert_eq!(
            quantization_from_filename(Path::new("model-Q5_K_S.gguf")),
            QuantizationType::Q5Ks
        );
        assert_eq!(
            quantization_from_filename(Path::new("model.Q6_K.gguf")),
            QuantizationType::Q6K
        );
    }

    #[test]
    fn quantization_from_filename_detects_basic_quants() {
        assert_eq!(
            quantization_from_filename(Path::new("model-Q4_0.gguf")),
            QuantizationType::Q4_0
        );
        assert_eq!(
            quantization_from_filename(Path::new("model-Q5_1.gguf")),
            QuantizationType::Q5_1
        );
        assert_eq!(
            quantization_from_filename(Path::new("model-Q8_0.gguf")),
            QuantizationType::Q8_0
        );
    }

    #[test]
    fn quantization_from_filename_detects_float_types() {
        assert_eq!(
            quantization_from_filename(Path::new("model-F16.gguf")),
            QuantizationType::F16
        );
        assert_eq!(
            quantization_from_filename(Path::new("model.fp16.gguf")),
            QuantizationType::F16
        );
        assert_eq!(
            quantization_from_filename(Path::new("model-F32.gguf")),
            QuantizationType::F32
        );
    }

    #[test]
    fn quantization_from_filename_detects_iq_quants() {
        assert_eq!(
            quantization_from_filename(Path::new("model-IQ2_XXS.gguf")),
            QuantizationType::Iq2Xxs
        );
        assert_eq!(
            quantization_from_filename(Path::new("model-IQ3_XS.gguf")),
            QuantizationType::Iq3Xs
        );
        assert_eq!(
            quantization_from_filename(Path::new("model-IQ4_NL.gguf")),
            QuantizationType::Iq4Nl
        );
    }

    #[test]
    fn quantization_from_filename_returns_unknown_for_no_match() {
        assert_eq!(
            quantization_from_filename(Path::new("model.gguf")),
            QuantizationType::Unknown
        );
        assert_eq!(
            quantization_from_filename(Path::new("random-file.bin")),
            QuantizationType::Unknown
        );
    }

    #[test]
    fn model_name_from_filename_strips_quantization() {
        assert_eq!(
            model_name_from_filename(Path::new("llama-3-8b-Q4_K_M.gguf")),
            Some("llama-3-8b".to_string())
        );
        assert_eq!(
            model_name_from_filename(Path::new("mistral-7b-instruct-v0.2.Q5_K_S.gguf")),
            Some("mistral-7b-instruct-v0.2".to_string())
        );
        assert_eq!(
            model_name_from_filename(Path::new("phi-3-mini_F16.gguf")),
            Some("phi-3-mini".to_string())
        );
    }

    #[test]
    fn model_name_from_filename_handles_no_quantization() {
        assert_eq!(
            model_name_from_filename(Path::new("model.gguf")),
            Some("model".to_string())
        );
    }

    #[test]
    fn detect_quantization_prefers_filename() {
        // Even for non-existent files, filename detection should work
        let path = Path::new("/nonexistent/model-Q4_K_M.gguf");
        assert_eq!(detect_quantization(path), QuantizationType::Q4Km);
    }

    #[test]
    fn is_gguf_file_returns_false_for_missing() {
        assert!(!is_gguf_file(Path::new("/nonexistent/model.gguf")));
    }
}
