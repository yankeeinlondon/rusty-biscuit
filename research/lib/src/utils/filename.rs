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

/// Generate a kebab-case filename from a prompt using Gemini Flash
///
/// # Arguments
/// * `prompt` - The research prompt text
///
/// # Returns
/// * `Ok(String)` - Kebab-case filename with .md extension (e.g., "async-best-practices.md")
/// * `Err(FilenameError)` - If API call fails, API key missing, or response invalid
///
/// # Environment Variables
/// * `GEMINI_API_KEY` - Required. Your Google Gemini API key
///
/// # Example
/// ```no_run
/// use research_lib::utils::filename::choose_filename;
///
/// #[tokio::main]
/// async fn main() {
///     let filename = choose_filename("What are the best practices for async Rust?").await;
///     match filename {
///         Ok(name) => println!("Generated: {}", name),
///         Err(e) => eprintln!("Fallback to question_1.md: {}", e),
///     }
/// }
/// ```
pub async fn choose_filename(prompt: &str) -> Result<String, FilenameError> {
    // Get API key from environment
    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| FilenameError::NoApiKey)?;

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
        return Err(FilenameError::ApiError(
            format!("HTTP {}: {}", response.status(), response.text().await.unwrap_or_default())
        ));
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
    let sanitized = generated_text
        .trim()
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    // Validate result is not empty
    if sanitized.is_empty() {
        return Err(FilenameError::InvalidResponse(
            format!("Sanitized response is empty: '{}'", generated_text)
        ));
    }

    Ok(format!("{}.md", sanitized))
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
