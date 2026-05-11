//! Provider-specific model catalog sources.
//!
//! Some providers have static model lists compiled from generated enums
//! (Claude, Codex). Others require dynamic sourcing by shelling out to
//! the provider CLI (OpenCode, Qwen). Providers without a source in v1
//! return an empty list and rely entirely on user overrides.

use std::process::Stdio;

use tokio::process::Command;

use crate::provider::{ModelCatalogSource, Provider, provider_info};
/// Return a static catalog for providers with known model enums.
///
/// These lists are derived from the generated enums in `unchained-ai/lib`.
pub fn static_catalog_for_provider(provider: Provider) -> Vec<String> {
    provider_info(provider)
        .static_models
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Fetch a dynamic catalog for the given provider.
///
/// Returns `Ok(models)` on success, `Err` if the source is unavailable.
/// For OpenCode and Qwen this shells out to `opencode models`.
pub async fn fetch_provider_catalog(provider: Provider) -> Result<Vec<String>, CatalogFetchError> {
    match provider_info(provider).dynamic_source {
        ModelCatalogSource::None => Ok(Vec::new()),
        ModelCatalogSource::Static => Ok(static_catalog_for_provider(provider)),
        ModelCatalogSource::OpencodeCli => fetch_opencode_models().await,
        ModelCatalogSource::OpencodeCliQwenFiltered => fetch_qwen_models().await,
    }
}

/// Error fetching a provider catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogFetchError {
    /// The provider CLI is not installed or not on PATH.
    CliNotFound(String),
    /// The CLI exited with a non-zero status.
    CliFailed {
        exit_code: Option<i32>,
        stderr: String,
    },
    /// The CLI output could not be parsed.
    ParseFailed(String),
}

impl std::fmt::Display for CatalogFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CliNotFound(cmd) => write!(f, "provider CLI not found: {cmd}"),
            Self::CliFailed { exit_code, stderr } => {
                write!(
                    f,
                    "provider CLI failed (exit={}): {stderr}",
                    exit_code.map_or("?".to_string(), |c| c.to_string())
                )
            }
            Self::ParseFailed(reason) => write!(f, "failed to parse provider output: {reason}"),
        }
    }
}

impl std::error::Error for CatalogFetchError {}

// ============================================================================
// Static sources
// ============================================================================

#[allow(dead_code)]
fn openai_models() -> Vec<String> {
    vec![
        "gpt-3.5-turbo".into(),
        "gpt-5-search-api".into(),
        "gpt-5.1-codex".into(),
        "gpt-5.2".into(),
        "gpt-5.2-chat-latest".into(),
        "o3".into(),
        "o3-mini".into(),
        "o3-mini-2025-01-31".into(),
        "o4-mini".into(),
    ]
}

#[allow(dead_code)]
fn anthropic_models() -> Vec<String> {
    vec![
        "claude-3-5-haiku-20241022".into(),
        "claude-3-7-sonnet-20250219".into(),
        "claude-3-haiku-20240307".into(),
        "claude-haiku-4-5-20251001".into(),
        "claude-opus-4-1-20250805".into(),
        "claude-opus-4-20250514".into(),
        "claude-opus-4-5-20251101".into(),
        "claude-sonnet-4-20250514".into(),
        "claude-sonnet-4-5-20250929".into(),
    ]
}

// ============================================================================
// Dynamic sources
// ============================================================================

async fn fetch_opencode_models() -> Result<Vec<String>, CatalogFetchError> {
    let output = Command::new("opencode")
        .arg("models")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CatalogFetchError::CliNotFound("opencode".into())
            } else {
                CatalogFetchError::CliFailed {
                    exit_code: None,
                    stderr: e.to_string(),
                }
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(CatalogFetchError::CliFailed {
            exit_code: output.status.code(),
            stderr,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_opencode_models(&stdout)
}

async fn fetch_qwen_models() -> Result<Vec<String>, CatalogFetchError> {
    let all = fetch_opencode_models().await?;
    let qwen: Vec<String> = all
        .into_iter()
        .filter(|m| m.to_ascii_lowercase().contains("qwen"))
        .collect();
    Ok(qwen)
}

/// Parse `opencode models` output into normalized model IDs.
///
/// Tries multiple strategies:
/// 1. JSON array of strings
/// 2. JSON array of objects with `id` or `name` field
/// 3. Plain text, one model per line
fn parse_opencode_models(output: &str) -> Result<Vec<String>, CatalogFetchError> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    // Strategy 1: JSON array of strings
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(trimmed) {
        return Ok(arr);
    }

    // Strategy 2: JSON array of objects
    if trimmed.starts_with('[')
        && let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(trimmed)
    {
        let models: Vec<String> = arr
            .into_iter()
            .filter_map(|v| {
                v.get("id")
                    .and_then(|v| v.as_str())
                    .map(|id| id.to_string())
                    .or_else(|| {
                        v.get("name")
                            .and_then(|v| v.as_str())
                            .map(|name| name.to_string())
                    })
            })
            .collect();
        if !models.is_empty() {
            return Ok(models);
        }
    }

    // Strategy 3: plain text, one per line
    let models: Vec<String> = trimmed
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    if models.is_empty() {
        return Err(CatalogFetchError::ParseFailed(
            "no models found in output".into(),
        ));
    }

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_catalog_codex_contains_known_models() {
        let models = static_catalog_for_provider(Provider::Codex);
        assert!(models.contains(&"gpt-4o".to_string()) || models.contains(&"gpt-5.2".to_string()));
        assert!(models.contains(&"o3-mini".to_string()));
    }

    #[test]
    fn static_catalog_claude_contains_known_models() {
        let models = static_catalog_for_provider(Provider::Claude);
        assert!(
            models.contains(&"claude-3-7-sonnet-20250219".to_string()),
            "expected claude-3-7-sonnet-20250219 in {models:?}"
        );
    }

    #[test]
    fn static_catalog_gemini_is_empty() {
        let models = static_catalog_for_provider(Provider::Gemini);
        assert!(models.is_empty());
    }

    #[test]
    fn parse_opencode_json_string_array() {
        let input = r#"["gpt-4o", "o3-mini", "claude-sonnet-4"]"#;
        let models = parse_opencode_models(input).unwrap();
        assert_eq!(models, vec!["gpt-4o", "o3-mini", "claude-sonnet-4"]);
    }

    #[test]
    fn parse_opencode_json_object_array() {
        let input = r#"[{"id":"gpt-4o"},{"id":"o3-mini","name":"o3 Mini"}]"#;
        let models = parse_opencode_models(input).unwrap();
        assert_eq!(models, vec!["gpt-4o", "o3-mini"]);
    }

    #[test]
    fn parse_opencode_plain_text() {
        let input = "gpt-4o\no3-mini\nclaude-sonnet-4\n";
        let models = parse_opencode_models(input).unwrap();
        assert_eq!(models, vec!["gpt-4o", "o3-mini", "claude-sonnet-4"]);
    }

    #[test]
    fn parse_opencode_empty_returns_empty() {
        let models = parse_opencode_models("").unwrap();
        assert!(models.is_empty());
    }

    #[test]
    fn parse_opencode_whitespace_only_returns_empty() {
        let models = parse_opencode_models("   \n\n   ").unwrap();
        assert!(models.is_empty());
    }
}
