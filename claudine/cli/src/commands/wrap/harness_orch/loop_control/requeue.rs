//! Rendezvous deferred-execution (`requeue`) enqueue machinery: daemon-first
//! with a durable JSONL fallback. Entirely `dead_code` until the rendezvous
//! deferred-execution backend lands (`defer` currently surfaces
//! `LifecycleDeferNotImplemented`).

use super::*;

#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
pub(super) const REQUEUE_SESSION_ID: &str = "claudine-deferred-execution";
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
pub(super) const REQUEUE_SOURCE: &str = "claudine.lifecycle.requeue";
/// Environment variable that overrides the directory used by the
/// rendezvous deferred-queue fallback file. When unset the fallback
/// lives under `<config_dir>/claudine/rendezvous/deferred-queue.jsonl`.
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
pub(super) const REQUEUE_FALLBACK_DIR_ENV: &str = "CLAUDINE_RENDEZVOUS_FALLBACK_DIR";
/// Fallback file name appended to the resolved fallback directory when no
/// rendezvous daemon is reachable. Each line is the JSON serialization of
/// the same `AppendEntryRequest` shape the daemon would have received, so a
/// future daemon can drain it verbatim.
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
pub(super) const REQUEUE_FALLBACK_FILE_NAME: &str = "deferred-queue.jsonl";

/// Errors that can occur while persisting a `requeue(...)` deferred-prompt
/// entry.
///
/// The contract is daemon-first with a durable fallback (see
/// [`enqueue_requeue_entry`]). Only failures that lose the prompt surface
/// here; a daemon connect/append failure that successfully falls back to the
/// JSONL file is `Ok(())`.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
pub(super) enum RequeueEnqueueError {
    #[error("failed to connect to rendezvous daemon at {endpoint}: {source}")]
    Connect {
        endpoint: rendezvous_core::LocalEndpoint,
        #[source]
        source: rendezvous_client::ConnectError,
    },
    #[error("rendezvous append-entry RPC failed: {0}")]
    Rpc(#[from] tonic::Status),
    #[error("failed to serialize requeue metadata: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("no Tokio runtime is available for rendezvous enqueue")]
    NoRuntime,
    /// The daemon was unreachable AND the durable fallback write failed.
    /// The prompt is lost; surface this to the user as a hard failure.
    #[error(
        "rendezvous daemon unreachable ({daemon_error}) and fallback write to {path} failed: {source}"
    )]
    FallbackWrite {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
        daemon_error: String,
    },
}

/// Resolve the durable fallback directory for the deferred-prompt queue.
///
/// Order:
/// 1. `CLAUDINE_RENDEZVOUS_FALLBACK_DIR` env var (test isolation / power
///    users).
/// 2. `<config_dir>/claudine/rendezvous/` via the `dirs` crate (per-user,
///    cross-platform: `~/Library/Application Support` on macOS,
///    `~/.config` on Linux, `%APPDATA%` on Windows).
/// 3. `~/.claudine/rendezvous/` as a last-resort home-dir fallback.
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
pub(super) fn requeue_fallback_dir() -> Option<std::path::PathBuf> {
    if let Some(explicit) = std::env::var_os(REQUEUE_FALLBACK_DIR_ENV)
        && !explicit.is_empty()
    {
        return Some(std::path::PathBuf::from(explicit));
    }
    let base = dirs::config_dir().or_else(dirs::home_dir)?;
    Some(base.join("claudine").join("rendezvous"))
}

/// Resolve the absolute fallback file path (without touching the disk).
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
pub(super) fn requeue_fallback_path() -> Option<std::path::PathBuf> {
    requeue_fallback_dir().map(|d| d.join(REQUEUE_FALLBACK_FILE_NAME))
}

/// Append one deferred-prompt entry to the durable fallback JSONL file as a
/// single line. Creates the parent directory if needed. Each line carries
/// the same shape as the `AppendEntryRequest` the daemon would have
/// received so a future daemon can drain the file verbatim.
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
pub(super) fn write_requeue_fallback(
    path: &Path,
    request: &rendezvous_core::AppendEntryRequest,
) -> std::result::Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut entry = serde_json::Map::new();
    entry.insert(
        "owner_node_id".to_string(),
        serde_json::Value::String(request.owner_node_id.clone()),
    );
    entry.insert(
        "session_id".to_string(),
        serde_json::Value::String(request.session_id.clone()),
    );
    entry.insert(
        "source".to_string(),
        serde_json::Value::String(request.source.clone()),
    );
    entry.insert(
        "level".to_string(),
        serde_json::Value::String(request.level.clone()),
    );
    entry.insert(
        "message".to_string(),
        serde_json::Value::String(request.message.clone()),
    );
    // `metadata_json` arrives as a JSON-encoded string; embed it as a parsed
    // object so the line is human-readable and round-trips cleanly. Fall
    // back to the raw string if the daemon-side producer emitted non-object
    // JSON.
    let metadata_value = serde_json::from_str::<serde_json::Value>(&request.metadata_json)
        .unwrap_or_else(|_| serde_json::Value::String(request.metadata_json.clone()));
    entry.insert("metadata_json".to_string(), metadata_value);
    let line = serde_json::Value::Object(entry);
    let mut serialized = serde_json::to_string(&line)?;
    serialized.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    use std::io::Write;
    file.write_all(serialized.as_bytes())?;
    file.flush()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
pub(super) async fn enqueue_requeue_entry_async(
    provider: Provider,
    prompt_state: &HarnessPromptState,
    materialized: &MaterializedHarnessPrompt,
    repo_root: Option<&Path>,
    delay: &str,
    reason: Option<&str>,
) -> std::result::Result<(), RequeueEnqueueError> {
    let endpoint =
        rendezvous_core::socket::legacy_local_endpoint(rendezvous_core::socket::default_socket_path());
    let metadata = serde_json::json!({
        "kind": "claudine.lifecycle.requeue",
        "provider": provider.as_slug(),
        "prompt_mode": harness_prompt_mode_label(prompt_state.mode),
        "source_path": prompt_state.source_path,
        "original_ref": prompt_state.original_ref,
        "repo_root": repo_root,
        "delay": delay,
        "reason": reason,
        "prompt": materialized.prompt,
        "frontmatter": materialized.frontmatter,
    });
    let request = rendezvous_core::AppendEntryRequest {
        owner_node_id: String::new(),
        session_id: REQUEUE_SESSION_ID.to_string(),
        source: REQUEUE_SOURCE.to_string(),
        level: "info".to_string(),
        message: format!(
            "deferred {} for {}",
            prompt_state.source_path.display(),
            delay
        ),
        metadata_json: serde_json::to_string(&metadata)?,
    };
    // Daemon-first: on any connect or append failure, durably persist the
    // entry to the local fallback file so the prompt is never lost. Only a
    // fallback write failure surfaces.
    match try_enqueue_via_daemon(&endpoint, &request).await {
        Ok(()) => Ok(()),
        Err(daemon_err) => {
            let Some(fallback_path) = requeue_fallback_path() else {
                // No writable fallback location: surface the daemon error.
                return Err(daemon_err);
            };
            let daemon_error = daemon_err.to_string();
            write_requeue_fallback(&fallback_path, &request).map_err(|source| {
                RequeueEnqueueError::FallbackWrite {
                    path: fallback_path.clone(),
                    source,
                    daemon_error: daemon_error.clone(),
                }
            })?;
            tracing::warn!(
                target: "claudine::lifecycle::requeue",
                daemon_error = %daemon_error,
                fallback_path = %fallback_path.display(),
                "rendezvous daemon unreachable; deferred prompt persisted to fallback file",
            );
            Ok(())
        }
    }
}

/// Attempt the live-daemon append-entry RPC.
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
pub(super) async fn try_enqueue_via_daemon(
    endpoint: &rendezvous_core::LocalEndpoint,
    request: &rendezvous_core::AppendEntryRequest,
) -> std::result::Result<(), RequeueEnqueueError> {
    let mut client = rendezvous_client::connect(endpoint)
        .await
        .map_err(|source| RequeueEnqueueError::Connect {
            endpoint: endpoint.clone(),
            source,
        })?;
    client.append_entry(request.clone()).await?;
    Ok(())
}

#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
pub(super) fn enqueue_requeue_entry(
    provider: Provider,
    prompt_state: &HarnessPromptState,
    materialized: &MaterializedHarnessPrompt,
    repo_root: Option<&Path>,
    delay: &str,
    reason: Option<&str>,
) -> std::result::Result<(), RequeueEnqueueError> {
    let handle =
        tokio::runtime::Handle::try_current().map_err(|_| RequeueEnqueueError::NoRuntime)?;
    tokio::task::block_in_place(|| {
        handle.block_on(enqueue_requeue_entry_async(
            provider,
            prompt_state,
            materialized,
            repo_root,
            delay,
            reason,
        ))
    })
}
