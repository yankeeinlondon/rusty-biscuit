//! requeue harness-loop tests.

use super::*;

use super::*;
use indexmap::IndexMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// Build a `requeue(...)`-shaped prompt state pointing at `source`.
fn requeue_prompt_state(source: &Path) -> HarnessPromptState {
    HarnessPromptState {
        mode: HarnessPromptMode::Compose,
        source_path: source.to_path_buf(),
        original_ref: source.display().to_string(),
        base_prompt: None,
        overlay: IndexMap::new(),
        prompt_tail: Vec::new(),
        next_prompt_override: None,
        next_resume_session_id: None,
        rematerialize: Default::default(),
    }
}

/// Build a materialized prompt with the deferred-prompt body the requeue
/// action is supposed to persist.
fn requeue_materialized(prompt: &str) -> MaterializedHarnessPrompt {
    let frontmatter = serde_json::json!({"title": "deferred"});
    let live_frontmatter = MaterializedHarnessPrompt::live_cell_from(&frontmatter);
    MaterializedHarnessPrompt {
        frontmatter,
        prompt: prompt.to_string(),
        env_overrides: Vec::new(),
        inline_closure_plan: None,
        live_frontmatter,
    }
}

/// The cross-platform Windows-facing contract: when the rendezvous daemon
/// is unreachable, `enqueue_requeue_entry` must NOT abort — it must
/// return `Ok(())` and append exactly one durable fallback entry whose
/// shape matches what the daemon would have received. This is the exact
/// code path a Windows user takes (no daemon runs there), proven on the
/// macOS host by pointing `RENDEZVOUS_SOCKET` at a non-existent socket.
#[tokio::test]
#[serial_test::serial(requeue_fallback)]
async fn enqueue_requeue_entry_falls_back_to_durable_file_when_daemon_unreachable() {
    let fallback_dir = TempDir::new().expect("tempdir");
    let fallback_path: PathBuf = fallback_dir.path().join(REQUEUE_FALLBACK_FILE_NAME);
    let _socket_env =
        test_toolkit::EnvGuard::set_safe("RENDEZVOUS_SOCKET", "/tmp/does-not-exist-rs.sock");
    let _fallback_env =
        test_toolkit::EnvGuard::set_safe(REQUEUE_FALLBACK_DIR_ENV, fallback_dir.path());

    let workspace = TempDir::new().expect("workspace tempdir");
    let source_path = workspace.path().join("deferred.md");
    std::fs::write(&source_path, "defer body").expect("write source");
    let prompt_state = requeue_prompt_state(&source_path);
    let materialized = requeue_materialized("Body to defer through rendezvous\n");

    let result = enqueue_requeue_entry_async(
        Provider::Goose,
        &prompt_state,
        &materialized,
        Some(workspace.path()),
        "5m",
        Some("provider failed"),
    )
    .await;
    assert!(
        result.is_ok(),
        "daemon-unreachable requeue must succeed via fallback; got {:?}",
        result.err()
    );

    // Exactly one JSONL line was appended.
    let contents = std::fs::read_to_string(&fallback_path).expect("fallback file written");
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines.len(), 1, "exactly one fallback entry; got {lines:?}");
    let entry: serde_json::Value =
        serde_json::from_str(lines[0]).expect("fallback line is valid JSON");

    // The entry carries the same shape as AppendEntryRequest.
    assert_eq!(entry["source"], REQUEUE_SOURCE);
    assert_eq!(entry["level"], "info");
    assert_eq!(entry["session_id"], REQUEUE_SESSION_ID);
    assert_eq!(entry["owner_node_id"], "");
    let message = entry["message"].as_str().expect("message is a string");
    assert!(
        message.contains("deferred.md") && message.contains("5m"),
        "entry message should identify the prompt and delay; got {message:?}"
    );

    // `metadata_json` is embedded as a parsed object — its inner shape is
    // the contract a future daemon drain depends on.
    let metadata = &entry["metadata_json"];
    assert_eq!(metadata["kind"], "claudine.lifecycle.requeue");
    assert_eq!(metadata["provider"], "goose");
    assert_eq!(metadata["delay"], "5m");
    assert_eq!(metadata["reason"], "provider failed");
    assert_eq!(metadata["prompt"], "Body to defer through rendezvous\n");
    assert!(
        metadata["source_path"]
            .as_str()
            .is_some_and(|p| p.ends_with("deferred.md")),
        "metadata should record source_path; got {metadata}"
    );
}

/// A second requeue on the same fallback file appends rather than
/// overwriting — the queue is durable and accumulates across runs.
#[tokio::test]
#[serial_test::serial(requeue_fallback)]
async fn enqueue_requeue_entry_fallback_appends_across_calls() {
    let fallback_dir = TempDir::new().expect("tempdir");
    let fallback_path: PathBuf = fallback_dir.path().join(REQUEUE_FALLBACK_FILE_NAME);
    let _socket_env =
        test_toolkit::EnvGuard::set_safe("RENDEZVOUS_SOCKET", "/tmp/does-not-exist-rs.sock");
    let _fallback_env =
        test_toolkit::EnvGuard::set_safe(REQUEUE_FALLBACK_DIR_ENV, fallback_dir.path());

    let workspace = TempDir::new().expect("workspace tempdir");
    let source_path = workspace.path().join("deferred.md");
    std::fs::write(&source_path, "defer body").expect("write source");
    let prompt_state = requeue_prompt_state(&source_path);
    let materialized = requeue_materialized("body\n");

    enqueue_requeue_entry_async(
        Provider::Goose,
        &prompt_state,
        &materialized,
        Some(workspace.path()),
        "1m",
        None,
    )
    .await
    .expect("first enqueue");
    enqueue_requeue_entry_async(
        Provider::Goose,
        &prompt_state,
        &materialized,
        Some(workspace.path()),
        "2m",
        None,
    )
    .await
    .expect("second enqueue");

    let contents = std::fs::read_to_string(&fallback_path).expect("fallback file");
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines.len(), 2, "fallback file accumulates entries");
    let first: serde_json::Value = serde_json::from_str(lines[0]).expect("first entry parses");
    let second: serde_json::Value =
        serde_json::from_str(lines[1]).expect("second entry parses");
    assert_eq!(first["metadata_json"]["delay"], "1m");
    assert_eq!(second["metadata_json"]["delay"], "2m");
}

