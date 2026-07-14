use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tracing::warn;

use crate::events::{AgenticEvent, EnvironmentContext, EventMeta};

use super::wrapper_flags::{wrapper_interactive_flag, wrapper_yolo_flag};

/// Write a single [`EventMeta`] as a JSONL line to the given path.
///
/// Creates parent directories if needed. The line is appended to the file
/// (or created if it does not exist) and terminated with `\n`.
///
/// ## Examples
///
/// ```no_run
/// # use claudine::dispatch::write_dispatch_event_to;
/// # use claudine::events::{AgenticEvent, EventMeta};
/// # use claudine::provider::Provider;
/// # use std::path::Path;
/// let meta = EventMeta::new(Provider::Claude, AgenticEvent::SessionStart);
/// write_dispatch_event_to(&meta, Path::new("/tmp/test.jsonl")).unwrap();
/// ```
pub fn write_dispatch_event_to(meta: &EventMeta, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut line = serde_json::to_string(meta)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    line.push('\n');

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(line.as_bytes())?;

    Ok(())
}

/// Write an [`EventMeta`] to the default daily-rotated JSONL log file.
///
/// Resolves the path via [`crate::reporting::paths::resolve_file_log_path`].
/// Errors are logged as warnings and swallowed so that logging failures
/// never abort dispatch.
pub fn log_dispatch_event(meta: &EventMeta) {
    let path: PathBuf = match crate::reporting::paths::resolve_file_log_path(None, true) {
        Ok(path) => path,
        Err(e) => {
            warn!(error = %e, "Failed to resolve dispatch log path");
            return;
        }
    };

    if let Err(e) = write_dispatch_event_to(meta, &path) {
        warn!(error = %e, path = %path.display(), "Failed to write dispatch event to JSONL log");
    }
}

pub(super) fn prepare_meta_for_dispatch(meta: &mut EventMeta, env: &EnvironmentContext) {
    meta.env = env.clone();

    if meta.session_id.is_none()
        && let Ok(wrapper_sid) = std::env::var("CLAUDINE_SESSION_ID")
        && !wrapper_sid.trim().is_empty()
    {
        meta.session_id = Some(wrapper_sid);
    }

    if let Some(interactive) = wrapper_interactive_flag() {
        meta.extra
            .entry("interactive".to_string())
            .or_insert_with(|| Value::String(interactive));
    }
    if let Some(yolo) = wrapper_yolo_flag() {
        meta.extra
            .entry("yolo".to_string())
            .or_insert_with(|| Value::String(yolo));
    }

    if let Some(pid) = meta.env.claudine_pid {
        meta.extra
            .entry("claudine_pid".to_string())
            .or_insert(Value::Number(serde_json::Number::from(pid)));
    }
    if let Some(pid) = meta.agent_pid {
        meta.extra
            .entry("agent_pid".to_string())
            .or_insert(Value::Number(serde_json::Number::from(pid)));
    }
}

pub(super) fn tool_detail_for_log(event: AgenticEvent, meta: &EventMeta) -> Option<String> {
    match event {
        AgenticEvent::BeforeTool | AgenticEvent::PermissionRequest => meta
            .tool_input
            .as_ref()
            .map(|value| compact_value_for_log(value, 120)),
        AgenticEvent::AfterTool => {
            let mut parts = Vec::new();

            if let Some(tool_id) = meta.extra.get("tool_id").and_then(|value| value.as_str()) {
                parts.push(format!("id={tool_id}"));
            }
            if let Some(status) = meta.extra.get("status").and_then(|value| value.as_str()) {
                parts.push(format!("status={status}"));
            }
            let error = meta.error.clone().or_else(|| {
                meta.extra
                    .get("error")
                    .and_then(|value| compact_scalar_for_log(value, 80))
            });
            if let Some(error) = error {
                parts.push(format!("error={error}"));
            }
            if let Some(response) = meta.tool_response.as_ref() {
                parts.push(format!("result={}", compact_value_for_log(response, 120)));
            }

            (!parts.is_empty()).then(|| parts.join(" "))
        }
        AgenticEvent::ToolError => meta
            .error
            .as_deref()
            .map(|error| format!("error={}", truncate_for_log(error, 80))),
        _ => None,
    }
}

fn compact_value_for_log(value: &Value, max_chars: usize) -> String {
    let rendered = serde_json::to_string(value).unwrap_or_else(|_| value.to_string());
    truncate_for_log(&rendered, max_chars)
}

fn compact_scalar_for_log(value: &Value, max_chars: usize) -> Option<String> {
    match value {
        Value::String(text) => Some(truncate_for_log(text, max_chars)),
        Value::Null | Value::Bool(_) | Value::Number(_) => {
            Some(compact_value_for_log(value, max_chars))
        }
        _ => None,
    }
}

fn truncate_for_log(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }

    let truncated: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    format!("{truncated}...")
}

#[cfg(test)]
mod logging_tests {
    use super::*;
    use crate::provider::Provider;
    use tempfile::TempDir;

    #[test]
    fn log_dispatch_event_writes_jsonl_line() {
        let tmp = TempDir::new().unwrap();
        let log_path = tmp.path().join("test.jsonl");

        let mut meta = EventMeta::new(Provider::Claude, AgenticEvent::SessionStart);
        meta.session_id = Some("test-sess".into());

        write_dispatch_event_to(&meta, &log_path).unwrap();

        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("test-sess"));
        assert!(content.ends_with('\n'));
    }
}
