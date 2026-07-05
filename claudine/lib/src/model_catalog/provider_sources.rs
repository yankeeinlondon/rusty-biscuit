//! Provider-specific model catalog sources.
//!
//! Most providers have static, research-fed model lists compiled into the
//! generated catalog. OpenCode sources dynamically by shelling out to its
//! own CLI (`ShellCommand`). Providers without a source return an empty
//! list and rely entirely on user overrides.

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
/// `ShellCommand` sources spawn the declared program (e.g. OpenCode's
/// `opencode models`).
pub async fn fetch_provider_catalog(provider: Provider) -> Result<Vec<String>, CatalogFetchError> {
    match provider_info(provider).model_catalog_source {
        ModelCatalogSource::None => Ok(Vec::new()),
        ModelCatalogSource::Static => Ok(static_catalog_for_provider(provider)),
        ModelCatalogSource::ShellCommand { program, args } => {
            fetch_shell_command_models(program, args).await
        }
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

/// Fetch a model catalog by spawning `program` with `args`.
///
/// The generic `ShellCommand` fetcher: stdout is parsed by
/// [`parse_shell_command_models`]. Exposed within the crate so
/// [`super::service::ModelCatalogService`] can deduplicate concurrent
/// fetches of the same command in-process.
///
/// Cancellable: the child is spawned with `kill_on_drop(true)` and the
/// await on its completion races a poll of the process-scoped
/// [`crate::interrupt`] flag, so a Ctrl+C during a slow or hung subprocess
/// returns within ~50 ms and the child is killed instead of orphaned.
pub(super) async fn fetch_shell_command_models(
    program: &'static str,
    args: &'static [&'static str],
) -> Result<Vec<String>, CatalogFetchError> {
    let child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CatalogFetchError::CliNotFound(program.into())
            } else {
                CatalogFetchError::CliFailed {
                    exit_code: None,
                    stderr: e.to_string(),
                }
            }
        })?;

    let output = tokio::select! {
        result = child.wait_with_output() => {
            result.map_err(|e| CatalogFetchError::CliFailed {
                exit_code: None,
                stderr: e.to_string(),
            })?
        }
        _ = wait_for_user_interrupt() => {
            // `child` was moved into `wait_with_output`; cancelling that
            // future drops the child and `kill_on_drop(true)` reaps the
            // subprocess so we do not leak it on Ctrl+C.
            return Err(CatalogFetchError::CliFailed {
                exit_code: None,
                stderr: "interrupted by user".into(),
            });
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(CatalogFetchError::CliFailed {
            exit_code: output.status.code(),
            stderr,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_shell_command_models(&stdout)
}

/// Resolves once the process-scoped user-interrupt flag is set.
///
/// Polled every 50 ms so the worst-case interrupt-to-return latency for
/// callers racing this in a `select!` is bounded by the poll cadence.
async fn wait_for_user_interrupt() {
    loop {
        if crate::interrupt::interrupted() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

/// Parse shell-command catalog output into normalized model IDs.
///
/// Tries multiple strategies:
/// 1. JSON array of strings
/// 2. JSON array of objects with `id` or `name` field
/// 3. Plain text, one model per line (trimmed, empty lines dropped)
fn parse_shell_command_models(output: &str) -> Result<Vec<String>, CatalogFetchError> {
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
        assert!(
            models.contains(&"gpt-5.5".to_string()),
            "expected gpt-5.5 in {models:?}"
        );
        assert!(models.contains(&"gpt-5.4-mini".to_string()));
    }

    #[test]
    fn static_catalog_claude_contains_known_models() {
        let models = static_catalog_for_provider(Provider::Claude);
        assert!(
            models.contains(&"claude-opus-4-8".to_string()),
            "expected claude-opus-4-8 in {models:?}"
        );
    }

    /// Gemini's static catalog is research-fed as of generator v1 (the
    /// agent-models `default_models` list; previously empty).
    #[test]
    fn static_catalog_gemini_contains_known_models() {
        let models = static_catalog_for_provider(Provider::Gemini);
        assert!(
            models.contains(&"gemini-2.5-pro".to_string()),
            "expected gemini-2.5-pro in {models:?}"
        );
    }

    /// Qwen left shell sourcing (2026-07-05 ruling): its research-fed
    /// `static_models` list is the catalog.
    #[test]
    fn static_catalog_qwen_contains_known_models() {
        let models = static_catalog_for_provider(Provider::QwenCode);
        assert!(
            models.contains(&"qwen3-coder-plus".to_string()),
            "expected qwen3-coder-plus in {models:?}"
        );
    }

    /// End-to-end smoke over a real subprocess (`echo`, not `opencode`):
    /// spawn, capture stdout, parse newline-separated ids.
    #[tokio::test]
    async fn fetch_shell_command_models_spawns_and_parses() {
        let models = fetch_shell_command_models("echo", &["model-a\nmodel-b"])
            .await
            .unwrap();
        assert_eq!(models, vec!["model-a", "model-b"]);
    }

    #[tokio::test]
    async fn fetch_shell_command_models_missing_program_is_cli_not_found() {
        let err = fetch_shell_command_models("claudine-test-no-such-program", &[])
            .await
            .unwrap_err();
        assert_eq!(
            err,
            CatalogFetchError::CliNotFound("claudine-test-no-such-program".into())
        );
    }

    #[test]
    fn parse_shell_command_json_string_array() {
        let input = r#"["gpt-4o", "o3-mini", "claude-sonnet-4"]"#;
        let models = parse_shell_command_models(input).unwrap();
        assert_eq!(models, vec!["gpt-4o", "o3-mini", "claude-sonnet-4"]);
    }

    #[test]
    fn parse_shell_command_json_object_array() {
        let input = r#"[{"id":"gpt-4o"},{"id":"o3-mini","name":"o3 Mini"}]"#;
        let models = parse_shell_command_models(input).unwrap();
        assert_eq!(models, vec!["gpt-4o", "o3-mini"]);
    }

    #[test]
    fn parse_shell_command_plain_text() {
        let input = "gpt-4o\no3-mini\nclaude-sonnet-4\n";
        let models = parse_shell_command_models(input).unwrap();
        assert_eq!(models, vec!["gpt-4o", "o3-mini", "claude-sonnet-4"]);
    }

    #[test]
    fn parse_shell_command_empty_returns_empty() {
        let models = parse_shell_command_models("").unwrap();
        assert!(models.is_empty());
    }

    #[test]
    fn parse_shell_command_whitespace_only_returns_empty() {
        let models = parse_shell_command_models("   \n\n   ").unwrap();
        assert!(models.is_empty());
    }
}
