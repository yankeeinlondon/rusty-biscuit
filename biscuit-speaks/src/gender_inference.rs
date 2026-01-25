//! Gender inference from voice names.
//!
//! This module provides functionality to infer gender from a voice name using
//! the `gender_guesser` crate. This is useful as a fallback when TTS providers
//! do not explicitly provide gender metadata for their voices.
//!
//! ## Accuracy Note
//!
//! Gender inference based on names has limitations, particularly for:
//! - Non-Western names
//! - Unisex names
//! - Technical or synthetic voice names (e.g., "Voice A", "en-US-1")
//!
//! When accuracy is critical, prefer explicit provider metadata over inference.

use crate::types::Gender;
use gender_guesser::{Detector, Gender as GuesserGender};

/// Infer gender from a voice name.
///
/// Uses the `gender_guesser` crate to analyze the given name and map the result
/// to our [`Gender`] enum.
///
/// ## Mapping
///
/// | gender_guesser result | Mapped to |
/// |----------------------|-----------|
/// | `Male`, `MayBeMale` | `Gender::Male` |
/// | `Female`, `MayBeFemale` | `Gender::Female` |
/// | `BothMaleFemale`, `NotSure`, `NotFound` | `Gender::Any` |
///
/// ## Examples
///
/// ```
/// use biscuit_speaks::gender_inference::infer_gender;
/// use biscuit_speaks::Gender;
///
/// assert_eq!(infer_gender("Samantha"), Gender::Female);
/// assert_eq!(infer_gender("Albert"), Gender::Male);
/// assert_eq!(infer_gender(""), Gender::Any);
/// ```
pub fn infer_gender(name: &str) -> Gender {
    // Handle empty strings gracefully
    if name.trim().is_empty() {
        return Gender::Any;
    }

    // Extract the first word as the likely given name
    // (handles names like "Samantha Enhanced" or "Albert Premium")
    let first_word = name.split_whitespace().next().unwrap_or(name);

    // Filter out non-alphabetic characters for cleaner matching
    let cleaned: String = first_word.chars().filter(|c| c.is_alphabetic()).collect();

    if cleaned.is_empty() {
        return Gender::Any;
    }

    let detector = Detector::new();
    let result = detector.get_gender(&cleaned);

    match result {
        GuesserGender::Male | GuesserGender::MayBeMale => Gender::Male,
        GuesserGender::Female | GuesserGender::MayBeFemale => Gender::Female,
        GuesserGender::BothMaleFemale | GuesserGender::NotSure | GuesserGender::NotFound => {
            Gender::Any
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Basic name tests
    // ========================================================================

    #[test]
    fn test_infer_female_names() {
        assert_eq!(infer_gender("Samantha"), Gender::Female);
        assert_eq!(infer_gender("Emily"), Gender::Female);
        assert_eq!(infer_gender("Sarah"), Gender::Female);
        assert_eq!(infer_gender("Jessica"), Gender::Female);
        assert_eq!(infer_gender("Victoria"), Gender::Female);
    }

    #[test]
    fn test_infer_male_names() {
        assert_eq!(infer_gender("Albert"), Gender::Male);
        assert_eq!(infer_gender("Michael"), Gender::Male);
        assert_eq!(infer_gender("David"), Gender::Male);
        assert_eq!(infer_gender("Robert"), Gender::Male);
        assert_eq!(infer_gender("William"), Gender::Male);
    }

    // ========================================================================
    // Edge cases
    // ========================================================================

    #[test]
    fn test_empty_string() {
        assert_eq!(infer_gender(""), Gender::Any);
    }

    #[test]
    fn test_whitespace_only() {
        assert_eq!(infer_gender("   "), Gender::Any);
        assert_eq!(infer_gender("\t\n"), Gender::Any);
    }

    #[test]
    fn test_special_characters_only() {
        assert_eq!(infer_gender("!!!"), Gender::Any);
        assert_eq!(infer_gender("@#$%"), Gender::Any);
        assert_eq!(infer_gender("123"), Gender::Any);
    }

    #[test]
    fn test_mixed_special_characters() {
        // Should extract alphabetic characters and still work
        assert_eq!(infer_gender("Michael123"), Gender::Male);
        assert_eq!(infer_gender("@Sarah"), Gender::Female);
    }

    // ========================================================================
    // Multi-word names (voice descriptors)
    // ========================================================================

    #[test]
    fn test_voice_descriptor_names() {
        // First word should be used for inference
        assert_eq!(infer_gender("Samantha Enhanced"), Gender::Female);
        assert_eq!(infer_gender("Albert Premium"), Gender::Male);
        assert_eq!(infer_gender("Emily (Neural)"), Gender::Female);
    }

    // ========================================================================
    // Unknown/ambiguous names
    // ========================================================================

    #[test]
    fn test_unknown_name() {
        // Made-up names that won't be in the database
        assert_eq!(infer_gender("Xyzzy"), Gender::Any);
        assert_eq!(infer_gender("Qwerty"), Gender::Any);
    }

    #[test]
    fn test_technical_voice_names() {
        // Technical names that aren't human names
        assert_eq!(infer_gender("Voice"), Gender::Any);
        assert_eq!(infer_gender("Default"), Gender::Any);
    }

    #[test]
    fn test_ambiguous_names_return_any() {
        // Names classified as BothMaleFemale by gender_guesser
        // (used in multiple cultures for different genders)
        assert_eq!(infer_gender("Alex"), Gender::Any);
        assert_eq!(infer_gender("John"), Gender::Any); // Surprisingly marked as both
        assert_eq!(infer_gender("Maria"), Gender::Any); // Cultural variation
    }

    #[test]
    fn test_case_insensitive() {
        // gender_guesser is case-insensitive
        assert_eq!(infer_gender("SAMANTHA"), Gender::Female);
        assert_eq!(infer_gender("albert"), Gender::Male);
        assert_eq!(infer_gender("EmIlY"), Gender::Female);
    }
}
