//! Provider validation baselines and listing sources.
//!
//! The validation baseline for every provider is the generated
//! expected-offering records ([`expected_baseline`]: ids plus rolling
//! aliases). Listing sources (`ModelCatalogSource`) describe how a
//! provider's live model listing is fetched — a shell command like
//! `opencode models`, or nothing at all — but a fetched listing only
//! feeds the on-disk drift-channel cache, never validation.

use std::collections::HashSet;
use std::process::Stdio;

use tokio::process::Command;

use crate::provider::{ModelCatalogSource, Provider, provider_info};

/// Expected-offering ids for a provider — the drift-comparison baseline.
///
/// Exactly the `id` values of the provider's generated
/// `expected_offerings`, in generated order. The drift channel compares
/// the cached dynamic listing against this set; validation uses
/// [`expected_baseline`] (which adds aliases) instead.
pub fn expected_ids(provider: Provider) -> Vec<String> {
    provider_info(provider)
        .expected_offerings
        .iter()
        .map(|offering| offering.id.to_string())
        .collect()
}

/// Validation baseline for a provider: expected-offering ids plus their
/// rolling aliases.
///
/// Users author aliases like `opus` in frontmatter `model:` hints, so
/// validation must accept them alongside exact ids. Deduplicated; all
/// ids come before any alias.
pub fn expected_baseline(provider: Provider) -> Vec<String> {
    let offerings = provider_info(provider).expected_offerings;
    let mut seen = HashSet::new();
    let mut baseline = Vec::new();
    for offering in offerings {
        if seen.insert(offering.id) {
            baseline.push(offering.id.to_string());
        }
    }
    for alias in offerings.iter().filter_map(|offering| offering.alias) {
        if seen.insert(alias) {
            baseline.push(alias.to_string());
        }
    }
    baseline
}

/// Fetch a dynamic catalog for the given provider.
///
/// Returns `Ok(models)` on success, `Err` if the source is unavailable.
/// `ShellCommand` sources spawn the declared program (e.g. OpenCode's
/// `opencode models`).
pub async fn fetch_provider_catalog(provider: Provider) -> Result<Vec<String>, CatalogFetchError> {
    match provider_info(provider).model_catalog_source {
        ModelCatalogSource::None => Ok(Vec::new()),
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
    fn expected_ids_are_offering_ids_only() {
        let ids = expected_ids(Provider::Claude);
        assert!(
            ids.contains(&"claude-opus-4-8".to_string()),
            "expected claude-opus-4-8 in {ids:?}"
        );
        assert!(
            !ids.contains(&"opus".to_string()),
            "aliases must not appear in the drift baseline: {ids:?}"
        );
    }

    #[test]
    fn expected_baseline_appends_aliases_after_ids() {
        let baseline = expected_baseline(Provider::Claude);
        let id_pos = baseline.iter().position(|m| m == "claude-opus-4-8");
        let alias_pos = baseline.iter().position(|m| m == "opus");
        assert!(id_pos.is_some(), "id missing from {baseline:?}");
        assert!(alias_pos.is_some(), "alias missing from {baseline:?}");
        assert!(id_pos < alias_pos, "ids must precede aliases");
    }

    /// Gemini's `auto` offering carries `auto` as its own alias, and two
    /// offerings share the `flash` alias — both collapse to one entry.
    #[test]
    fn expected_baseline_deduplicates() {
        let baseline = expected_baseline(Provider::Gemini);
        assert_eq!(baseline.iter().filter(|m| *m == "auto").count(), 1);
        assert_eq!(baseline.iter().filter(|m| *m == "flash").count(), 1);
    }

    /// Goose had no compiled model list before the baseline flip; its
    /// generated expected offerings made it validatable for the first
    /// time.
    #[test]
    fn expected_baseline_goose_non_empty() {
        let baseline = expected_baseline(Provider::Goose);
        assert!(
            baseline.contains(&"gpt-5".to_string()),
            "expected gpt-5 in {baseline:?}"
        );
    }

    /// End-to-end smoke over a real subprocess (not `opencode`): spawn,
    /// capture stdout, and parse newline-separated ids. Windows uses `cmd`
    /// because `echo` is shell syntax there, not an executable contract.
    #[tokio::test]
    async fn fetch_shell_command_models_spawns_and_parses() {
        #[cfg(windows)]
        let models = fetch_shell_command_models("cmd", &["/D", "/C", "echo model-a& echo model-b"])
            .await
            .unwrap();
        #[cfg(not(windows))]
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
