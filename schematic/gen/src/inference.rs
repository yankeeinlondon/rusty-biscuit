//! Module path inference from API names.
//!
//! This module provides heuristics for inferring the module path from an API name
//! when no explicit `module_path` is set. The inference is conservative and only
//! applies to specific naming patterns that clearly indicate a variant API.
//!
//! ## Inference Rules
//!
//! The inference only triggers for names with recognizable variant suffixes:
//! - "Native", "Client", "Service", "Hub", "Api", "Sdk"
//!
//! For example:
//! - "OllamaNative" -> "ollama" (has "Native" suffix)
//! - "OpenAI" -> None (no inference, falls back to "openai")
//! - "HuggingFaceHub" -> "huggingface" (has "Hub" suffix)
//!
//! This conservative approach prevents breaking changes for existing APIs
//! while enabling inference for obvious variant patterns.
//!
//! ## Examples
//!
//! ```ignore
//! // Variant suffix detected - extract base name
//! assert_eq!(infer_module_path("OllamaNative"), Some("ollama".to_string()));
//! assert_eq!(infer_module_path("HuggingFaceHub"), Some("huggingface".to_string()));
//!
//! // No variant suffix - returns None (caller should fall back)
//! assert_eq!(infer_module_path("OpenAI"), None);
//! assert_eq!(infer_module_path("HuggingFace"), None);
//! ```

/// Known suffixes that indicate a variant API.
///
/// When an API name ends with one of these suffixes, the inference will
/// extract the base name by removing the suffix.
const VARIANT_SUFFIXES: &[&str] = &["Native", "Client", "Service", "Hub", "Api", "Sdk"];

/// Infers a module path from an API name using conservative CamelCase heuristics.
///
/// Only applies inference when the name clearly indicates a variant API by
/// ending with a known suffix (Native, Client, Service, Hub, Api, Sdk).
/// For other names, returns None to let the caller fall back to a default.
///
/// ## Returns
///
/// - `Some(path)` if the name has a recognized variant suffix
/// - `None` if no inference can be made (caller should use fallback)
///
/// ## Examples
///
/// ```ignore
/// // Variant suffix detected
/// assert_eq!(infer_module_path("OllamaNative"), Some("ollama".to_string()));
/// assert_eq!(infer_module_path("HuggingFaceHub"), Some("huggingface".to_string()));
///
/// // No variant suffix - returns None
/// assert_eq!(infer_module_path("OpenAI"), None);
/// assert_eq!(infer_module_path("ollama"), None);
/// ```
pub fn infer_module_path(api_name: &str) -> Option<String> {
    if api_name.is_empty() {
        return None;
    }

    // Split CamelCase into words
    let words = split_camel_case(api_name);

    if words.len() < 2 {
        // Single word or empty - no inference possible
        return None;
    }

    // Check if the last word is a known variant suffix
    let last_word = words.last()?;
    if !VARIANT_SUFFIXES.contains(last_word) {
        // No recognized suffix - don't infer
        return None;
    }

    // Extract the base name (all words except the suffix)
    let base_words: Vec<&str> = words[..words.len() - 1].to_vec();

    // Join words with underscores and lowercase for the module path
    // e.g., ["Hugging", "Face"] -> "huggingface" (no underscores for module names)
    Some(
        base_words
            .iter()
            .map(|w| w.to_lowercase())
            .collect::<String>(),
    )
}

/// Splits a CamelCase string into individual words.
///
/// Handles various CamelCase patterns:
/// - "OllamaNative" -> ["Ollama", "Native"]
/// - "OpenAI" -> ["Open", "AI"]
/// - "HuggingFace" -> ["Hugging", "Face"]
/// - "ElevenLabs" -> ["Eleven", "Labs"]
/// - "HTTPClient" -> ["HTTP", "Client"]
/// - "ollama" -> ["ollama"]
fn split_camel_case(s: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut word_start = 0;
    let chars: Vec<char> = s.chars().collect();

    for i in 1..chars.len() {
        let current = chars[i];
        let prev = chars[i - 1];

        // Split before uppercase that follows lowercase: "ollamaNext" -> "ollama", "Next"
        // Split before uppercase followed by lowercase when preceded by uppercase: "HTTPClient" -> "HTTP", "Client"
        let is_new_word = current.is_uppercase()
            && (prev.is_lowercase()
                || (i + 1 < chars.len() && chars[i + 1].is_lowercase() && prev.is_uppercase()));

        if is_new_word {
            if i > word_start {
                words.push(&s[word_start..i]);
            }
            word_start = i;
        }
    }

    // Add the final word
    if word_start < s.len() {
        words.push(&s[word_start..]);
    }

    words
}

#[cfg(test)]
mod tests {
    use super::*;

    // === split_camel_case tests ===

    #[test]
    fn split_camel_case_multi_word() {
        assert_eq!(split_camel_case("OllamaNative"), vec!["Ollama", "Native"]);
    }

    #[test]
    fn split_camel_case_with_acronym() {
        assert_eq!(split_camel_case("OpenAI"), vec!["Open", "AI"]);
    }

    #[test]
    fn split_camel_case_two_words() {
        assert_eq!(split_camel_case("HuggingFace"), vec!["Hugging", "Face"]);
        assert_eq!(split_camel_case("ElevenLabs"), vec!["Eleven", "Labs"]);
    }

    #[test]
    fn split_camel_case_acronym_at_start() {
        assert_eq!(split_camel_case("HTTPClient"), vec!["HTTP", "Client"]);
    }

    #[test]
    fn split_camel_case_single_word() {
        assert_eq!(split_camel_case("Ollama"), vec!["Ollama"]);
    }

    #[test]
    fn split_camel_case_lowercase() {
        assert_eq!(split_camel_case("ollama"), vec!["ollama"]);
    }

    #[test]
    fn split_camel_case_empty() {
        assert_eq!(split_camel_case(""), Vec::<&str>::new());
    }

    // === infer_module_path tests ===

    #[test]
    fn infer_module_path_ollama_native() {
        // OllamaNative has "Native" suffix, so infers to "ollama"
        assert_eq!(
            infer_module_path("OllamaNative"),
            Some("ollama".to_string())
        );
    }

    #[test]
    fn infer_module_path_openai_no_inference() {
        // OpenAI has no recognized variant suffix, returns None
        // Caller will fall back to "openai" via to_lowercase()
        assert_eq!(infer_module_path("OpenAI"), None);
    }

    #[test]
    fn infer_module_path_huggingface_no_inference() {
        // HuggingFace has no recognized variant suffix, returns None
        // Caller will fall back to "huggingface" via to_lowercase()
        assert_eq!(infer_module_path("HuggingFace"), None);
    }

    #[test]
    fn infer_module_path_huggingface_hub() {
        // HuggingFaceHub has "Hub" suffix, so infers to "huggingface"
        assert_eq!(
            infer_module_path("HuggingFaceHub"),
            Some("huggingface".to_string())
        );
    }

    #[test]
    fn infer_module_path_elevenlabs_no_inference() {
        // ElevenLabs has no recognized variant suffix (Labs != Hub), returns None
        // Caller will fall back to "elevenlabs" via to_lowercase()
        assert_eq!(infer_module_path("ElevenLabs"), None);
    }

    #[test]
    fn infer_module_path_simple_lowercase() {
        // Single word: no inference possible
        assert_eq!(infer_module_path("ollama"), None);
    }

    #[test]
    fn infer_module_path_simple_capitalized() {
        // Single word: no inference possible
        assert_eq!(infer_module_path("Ollama"), None);
    }

    #[test]
    fn infer_module_path_empty() {
        assert_eq!(infer_module_path(""), None);
    }

    #[test]
    fn infer_module_path_all_caps() {
        // Single word (all caps): no inference possible
        assert_eq!(infer_module_path("API"), None);
    }

    #[test]
    fn infer_module_path_http_client() {
        // HTTPClient has "Client" suffix, so infers to "http"
        assert_eq!(infer_module_path("HTTPClient"), Some("http".to_string()));
    }

    #[test]
    fn infer_module_path_with_service_suffix() {
        // MyService has "Service" suffix, so infers to "my"
        assert_eq!(infer_module_path("MyService"), Some("my".to_string()));
    }

    #[test]
    fn infer_module_path_with_api_suffix() {
        // TestApi has "Api" suffix, so infers to "test"
        assert_eq!(infer_module_path("TestApi"), Some("test".to_string()));
    }

    #[test]
    fn infer_module_path_with_sdk_suffix() {
        // AwsSdk has "Sdk" suffix, so infers to "aws"
        assert_eq!(infer_module_path("AwsSdk"), Some("aws".to_string()));
    }

    #[test]
    fn infer_module_path_multi_word_with_suffix() {
        // OpenAINative has "Native" suffix, so infers to "openai"
        assert_eq!(
            infer_module_path("OpenAINative"),
            Some("openai".to_string())
        );
    }
}
