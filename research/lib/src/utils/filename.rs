use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum FilenameError {
    #[error("Gemini API error: {0}")]
    ApiError(String),
    #[error("No API key provided. Set GEMINI_API_KEY environment variable")]
    NoApiKey,
    #[error("Invalid response from API: {0}")]
    InvalidResponse(String),
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: String,
}

/// Generate a semantic filename from a prompt using LLM
///
/// Calls Gemini Flash to summarize prompt into 3-5 word expression,
/// converts to snake_case, validates uniqueness against existing research.
///
/// ## Arguments
/// * `prompt` - The research prompt text
/// * `existing_files` - List of filenames already in use for this research topic
///
/// ## Returns
/// * `Ok(String)` - Snake_case filename with .md extension
/// * `Err(FilenameError)` - If API call fails or validation fails
///
/// ## Collision Resolution
/// If generated filename conflicts with `existing_files`, appends `_1`, `_2`, etc.
/// until unique name found.
///
/// ## Examples
/// ```no_run
/// use research_lib::utils::filename::choose_filename;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let existing = vec!["async_patterns.md".to_string()];
///     let filename = choose_filename(
///         "What are advanced async patterns?",
///         &existing
///     ).await?;
///     // Result: "advanced_async_patterns.md" or "async_patterns_1.md" if collision
///     Ok(())
/// }
/// ```
pub async fn choose_filename(
    prompt: &str,
    existing_files: &[String],
) -> Result<String, FilenameError> {
    // Get API key from environment
    let api_key = std::env::var("GEMINI_API_KEY").map_err(|_| FilenameError::NoApiKey)?;

    // Create HTTP client with timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| FilenameError::ApiError(e.to_string()))?;

    // Build request
    let system_prompt = "Generate a short (2-5 word) kebab-case identifier for the following research prompt. \
                         Respond with ONLY the kebab-case identifier, no explanation, no quotes, no extra text.";

    let request_body = GeminiRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: format!("{}\n\nPrompt: {}", system_prompt, prompt),
            }],
        }],
    };

    // Call Gemini Flash API
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
        api_key
    );

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| FilenameError::ApiError(e.to_string()))?;

    if !response.status().is_success() {
        return Err(FilenameError::ApiError(format!(
            "HTTP {}: {}",
            response.status(),
            response.text().await.unwrap_or_default()
        )));
    }

    let gemini_response: GeminiResponse = response
        .json()
        .await
        .map_err(|e| FilenameError::InvalidResponse(e.to_string()))?;

    // Extract text from response
    let generated_text = gemini_response
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.as_str())
        .ok_or_else(|| FilenameError::InvalidResponse("No text in response".to_string()))?;

    // Sanitize: lowercase, replace spaces with hyphens, filter to alphanumeric + hyphens
    let kebab_case = generated_text
        .trim()
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    // Validate result is not empty
    if kebab_case.is_empty() {
        return Err(FilenameError::InvalidResponse(format!(
            "Sanitized response is empty: '{}'",
            generated_text
        )));
    }

    // Convert kebab-case to snake_case
    let snake_case = kebab_case.replace('-', "_");

    // Check for collision and resolve with _N suffix
    let final_filename = resolve_collision(&snake_case, existing_files);

    tracing::info!("Generated filename: {}", final_filename);

    Ok(final_filename)
}

/// Resolve filename collision by appending _N suffix
///
/// If the base filename conflicts with existing files, appends `_1`, `_2`, etc.
/// until a unique filename is found.
///
/// ## Arguments
/// * `base_name` - Base filename (without .md extension, in snake_case)
/// * `existing_files` - List of existing filenames to check against
///
/// ## Returns
/// Unique filename with .md extension
///
/// ## Examples
/// ```
/// use research_lib::utils::filename::resolve_collision;
///
/// let existing = vec!["async_patterns.md".to_string()];
/// let result = resolve_collision("async_patterns", &existing);
/// assert_eq!(result, "async_patterns_1.md");
///
/// let existing_multiple = vec!["test.md".to_string(), "test_1.md".to_string()];
/// let result = resolve_collision("test", &existing_multiple);
/// assert_eq!(result, "test_2.md");
/// ```
pub fn resolve_collision(base_name: &str, existing_files: &[String]) -> String {
    let candidate = format!("{}.md", base_name);

    // No collision
    if !existing_files.contains(&candidate) {
        return candidate;
    }

    // Collision detected, try _1, _2, _3, etc.
    tracing::debug!("Filename collision detected for: {}", candidate);

    let mut suffix = 1;
    loop {
        let candidate_with_suffix = format!("{}_{}.md", base_name, suffix);

        if !existing_files.contains(&candidate_with_suffix) {
            tracing::debug!("Resolved collision with: {}", candidate_with_suffix);
            return candidate_with_suffix;
        }

        suffix += 1;

        // Safety check to prevent infinite loops (highly unlikely in practice)
        if suffix > 1000 {
            tracing::warn!(
                "Collision resolution exceeded 1000 attempts for: {}",
                base_name
            );
            return format!("{}_{}.md", base_name, suffix);
        }
    }
}

/// Core research document filenames that cannot be used for custom prompts
///
/// These filenames are reserved for the core research pipeline outputs and
/// will be rejected if a user attempts to use them via the `filename -> prompt` syntax.
const CORE_DOCUMENTS: &[&str] = &[
    "overview",
    "similar_libraries",
    "integration_partners",
    "use_cases",
    "changelog",
    "deep_dive",   // legacy
    "deep-dive",   // new directory name
    "brief",
];

/// Extract manual filename from prompt if present
///
/// Parses prompt for `{filename} -> {prompt_text}` pattern.
/// Validates filename doesn't conflict with core research documents.
///
/// ## Arguments
/// * `prompt` - The user-provided research prompt
///
/// ## Returns
/// `(cleaned_prompt, optional_filename)`
/// - `cleaned_prompt`: Prompt with filename prefix removed (or unchanged)
/// - `optional_filename`: Some(filename) if valid, None if invalid or absent
///
/// ## Validation Rules
/// - Filename cannot be empty after trimming
/// - Filename cannot match core documents: `overview`, `similar_libraries`,
///   `integration_partners`, `use_cases`, `changelog`, `deep_dive`, `brief`, `deep-dive`
/// - Filename must not contain path separators (`/`, `\`)
/// - `.md` extension added automatically if omitted
///
/// ## Examples
/// ```
/// use research_lib::utils::filename::extract_prompt_name;
///
/// // Valid filename
/// let (prompt, filename) = extract_prompt_name("async-patterns -> What are best async patterns?");
/// assert_eq!(prompt, "What are best async patterns?");
/// assert_eq!(filename, Some("async-patterns.md".to_string()));
///
/// // Invalid filename (core document)
/// let (prompt, filename) = extract_prompt_name("overview -> What is this library?");
/// assert_eq!(prompt, "What is this library?"); // Prefix stripped
/// assert_eq!(filename, None); // Rejected
///
/// // No filename prefix
/// let (prompt, filename) = extract_prompt_name("What are best practices?");
/// assert_eq!(prompt, "What are best practices?");
/// assert_eq!(filename, None);
/// ```
pub fn extract_prompt_name(prompt: &str) -> (String, Option<String>) {
    // Check if prompt contains "->" separator
    let Some((filename_part, prompt_part)) = prompt.split_once("->") else {
        // No separator found, return prompt as-is
        return (prompt.to_string(), None);
    };

    // Trim whitespace from both parts
    let filename_candidate = filename_part.trim();
    let cleaned_prompt = prompt_part.trim().to_string();

    // Validate filename is not empty
    if filename_candidate.is_empty() {
        return (cleaned_prompt, None);
    }

    // Check for path separators
    if filename_candidate.contains('/') || filename_candidate.contains('\\') {
        eprintln!(
            "- a custom prompt expressed an invalid filename ({}) which contains path separators.",
            filename_candidate
        );
        return (cleaned_prompt, None);
    }

    // Extract filename without extension for core document check
    let filename_without_ext = filename_candidate
        .strip_suffix(".md")
        .unwrap_or(filename_candidate);

    // Check against core documents (case-insensitive)
    let filename_lower = filename_without_ext.to_lowercase();
    if CORE_DOCUMENTS.contains(&filename_lower.as_str()) {
        eprintln!(
            "- a custom prompt expressed an invalid filename ({}) which conflicts with the core research docs being produced.",
            filename_without_ext
        );
        return (cleaned_prompt, None);
    }

    // Add .md extension if not present
    let final_filename = if filename_candidate.ends_with(".md") {
        filename_candidate.to_string()
    } else {
        format!("{}.md", filename_candidate)
    };

    (cleaned_prompt, Some(final_filename))
}

/// Helper function to generate fallback filename with incrementing number
///
/// # Arguments
/// * `existing_files` - List of existing prompt files in the directory
///
/// # Returns
/// Next available `question_N.md` filename
pub fn fallback_filename(existing_files: &[String]) -> String {
    let mut n = 1;
    loop {
        let candidate = format!("question_{}.md", n);
        if !existing_files.contains(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_valid_filename() {
        let (prompt, filename) =
            extract_prompt_name("async-patterns -> What are best async patterns?");
        assert_eq!(prompt, "What are best async patterns?");
        assert_eq!(filename, Some("async-patterns.md".to_string()));
    }

    #[test]
    fn test_core_document_conflict_overview() {
        let (prompt, filename) = extract_prompt_name("overview -> What is this library?");
        assert_eq!(prompt, "What is this library?");
        assert_eq!(filename, None);
    }

    #[test]
    fn test_core_document_conflict_similar_libraries() {
        let (prompt, filename) = extract_prompt_name("similar_libraries -> Compare to others");
        assert_eq!(prompt, "Compare to others");
        assert_eq!(filename, None);
    }

    #[test]
    fn test_no_prefix_passthrough() {
        let (prompt, filename) = extract_prompt_name("What are best practices?");
        assert_eq!(prompt, "What are best practices?");
        assert_eq!(filename, None);
    }

    #[test]
    fn test_empty_filename() {
        let (prompt, filename) = extract_prompt_name(" -> What is this about?");
        assert_eq!(prompt, "What is this about?");
        assert_eq!(filename, None);
    }

    #[test]
    fn test_path_separator_forward_slash() {
        let (prompt, filename) = extract_prompt_name("foo/bar -> Some prompt text");
        assert_eq!(prompt, "Some prompt text");
        assert_eq!(filename, None);
    }

    #[test]
    fn test_path_separator_backslash() {
        let (prompt, filename) = extract_prompt_name("foo\\bar -> Some prompt text");
        assert_eq!(prompt, "Some prompt text");
        assert_eq!(filename, None);
    }

    #[test]
    fn test_whitespace_trimming() {
        let (prompt, filename) = extract_prompt_name("  test-file  ->  Some prompt text  ");
        assert_eq!(prompt, "Some prompt text");
        assert_eq!(filename, Some("test-file.md".to_string()));
    }

    #[test]
    fn test_case_insensitive_overview() {
        let (prompt, filename) = extract_prompt_name("Overview -> Some text");
        assert_eq!(prompt, "Some text");
        assert_eq!(filename, None);
    }

    #[test]
    fn test_case_insensitive_uppercase() {
        let (prompt, filename) = extract_prompt_name("CHANGELOG -> Some text");
        assert_eq!(prompt, "Some text");
        assert_eq!(filename, None);
    }

    #[test]
    fn test_case_insensitive_mixed() {
        let (prompt, filename) = extract_prompt_name("DeEp_DiVe -> Some text");
        assert_eq!(prompt, "Some text");
        assert_eq!(filename, None);
    }

    #[test]
    fn test_md_extension_already_present() {
        let (prompt, filename) = extract_prompt_name("test.md -> Some prompt text");
        assert_eq!(prompt, "Some prompt text");
        assert_eq!(filename, Some("test.md".to_string()));
    }

    #[test]
    fn test_md_extension_missing() {
        let (prompt, filename) = extract_prompt_name("test -> Some prompt text");
        assert_eq!(prompt, "Some prompt text");
        assert_eq!(filename, Some("test.md".to_string()));
    }

    #[test]
    fn test_arrow_without_space() {
        let (prompt, filename) = extract_prompt_name("filename->Some prompt text");
        assert_eq!(prompt, "Some prompt text");
        assert_eq!(filename, Some("filename.md".to_string()));
    }

    #[test]
    fn test_multiple_arrows() {
        let (prompt, filename) = extract_prompt_name("first -> second -> third");
        assert_eq!(prompt, "second -> third");
        assert_eq!(filename, Some("first.md".to_string()));
    }

    #[test]
    fn test_unicode_filename() {
        let (prompt, filename) = extract_prompt_name("тест -> Some prompt text");
        assert_eq!(prompt, "Some prompt text");
        assert_eq!(filename, Some("тест.md".to_string()));
    }

    #[test]
    fn test_all_core_documents() {
        // Test each core document to ensure they're all rejected
        for core_doc in CORE_DOCUMENTS {
            let input = format!("{} -> Test prompt", core_doc);
            let (prompt, filename) = extract_prompt_name(&input);
            assert_eq!(prompt, "Test prompt", "Failed for core doc: {}", core_doc);
            assert_eq!(filename, None, "Core doc {} was not rejected", core_doc);
        }
    }

    // Tests for resolve_collision function
    #[test]
    fn test_resolve_collision_no_conflict() {
        let existing = vec!["other_file.md".to_string()];
        let result = resolve_collision("test", &existing);
        assert_eq!(result, "test.md");
    }

    #[test]
    fn test_resolve_collision_empty_existing() {
        let existing: Vec<String> = vec![];
        let result = resolve_collision("test", &existing);
        assert_eq!(result, "test.md");
    }

    #[test]
    fn test_resolve_collision_single_conflict() {
        let existing = vec!["async_patterns.md".to_string()];
        let result = resolve_collision("async_patterns", &existing);
        assert_eq!(result, "async_patterns_1.md");
    }

    #[test]
    fn test_resolve_collision_multiple_conflicts() {
        let existing = vec![
            "test.md".to_string(),
            "test_1.md".to_string(),
            "test_2.md".to_string(),
        ];
        let result = resolve_collision("test", &existing);
        assert_eq!(result, "test_3.md");
    }

    #[test]
    fn test_resolve_collision_gap_in_sequence() {
        // If test.md and test_2.md exist, but not test_1.md, should use test_1.md
        let existing = vec!["test.md".to_string(), "test_2.md".to_string()];
        let result = resolve_collision("test", &existing);
        assert_eq!(result, "test_1.md");
    }

    #[test]
    fn test_resolve_collision_snake_case_preserved() {
        let existing = vec!["async_patterns.md".to_string()];
        let result = resolve_collision("async_patterns", &existing);
        assert_eq!(result, "async_patterns_1.md");
        assert!(result.contains('_'));
        assert!(!result.contains('-'));
    }

    #[test]
    fn test_resolve_collision_large_sequence() {
        // Test with many collisions (edge case)
        let mut existing = vec!["test.md".to_string()];
        for i in 1..100 {
            existing.push(format!("test_{}.md", i));
        }
        let result = resolve_collision("test", &existing);
        assert_eq!(result, "test_100.md");
    }

    #[test]
    fn test_resolve_collision_different_base_names() {
        // Ensure collision detection only applies to same base name
        let existing = vec![
            "async_patterns.md".to_string(),
            "async_patterns_1.md".to_string(),
        ];
        let result = resolve_collision("sync_patterns", &existing);
        assert_eq!(result, "sync_patterns.md");
    }

    #[test]
    fn test_fallback_filename_incrementing() {
        let existing = vec!["question_1.md".to_string(), "question_2.md".to_string()];
        let result = fallback_filename(&existing);
        assert_eq!(result, "question_3.md");
    }

    #[test]
    fn test_fallback_filename_empty_list() {
        let existing: Vec<String> = vec![];
        let result = fallback_filename(&existing);
        assert_eq!(result, "question_1.md");
    }
}
