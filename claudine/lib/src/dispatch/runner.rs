use std::io::Write;
use std::time::Duration;

use tracing::{debug, warn};

use super::template::interpolate;
use crate::error::Result;
use crate::events::{EventAction, EventMeta, GlobalSettings, LogTarget, ReportFormat};

/// Execute a list of actions for a matched event.
///
/// Audio actions (Speak, SoundEffect) are fire-and-forget via `tokio::spawn`.
/// Log and Report actions are executed inline.
///
/// ## Errors
///
/// Returns an error only for fatal failures (e.g. file I/O in Log/LocalFile).
/// Network and audio errors are logged but not propagated.
pub async fn execute_actions(
    actions: &[EventAction],
    meta: &EventMeta,
    settings: &GlobalSettings,
) -> Result<()> {
    for action in actions {
        match action {
            EventAction::Speak { message } => execute_speak(message, meta),
            EventAction::Log { target } => execute_log(target, meta, settings).await?,
            EventAction::Report { handler } => execute_report(handler.as_ref(), meta),
            EventAction::Run {
                command,
                args,
                blocking,
            } => {
                execute_run(command, args.as_deref(), *blocking).await?;
            }
            EventAction::SoundEffect { name, .. } => execute_sound_effect(name),
        }
    }
    Ok(())
}

/// Speak a message via biscuit-speaks TTS (fire-and-forget).
fn execute_speak(message_template: &str, meta: &EventMeta) {
    let text = interpolate(message_template, meta);
    if text.is_empty() {
        return;
    }
    debug!(%text, "Speaking via TTS");
    tokio::spawn(async move {
        if let Err(e) = biscuit_speaks::Speak::new(text).play().await {
            warn!(%e, "TTS playback failed");
        }
    });
}

/// Log event metadata to a file or remote server.
async fn execute_log(
    target: &LogTarget,
    meta: &EventMeta,
    _settings: &GlobalSettings,
) -> Result<()> {
    match target {
        LogTarget::LocalFile { path } => write_jsonl(path, meta),
        LogTarget::Server { url } => {
            post_to_server(url, meta).await;
            Ok(())
        }
    }
}

/// Append a JSONL entry to a local file, creating parent dirs as needed.
fn write_jsonl(path: &std::path::Path, meta: &EventMeta) -> Result<()> {
    let expanded = if path.starts_with("~") {
        dirs::home_dir()
            .map(|h| h.join(path.strip_prefix("~").unwrap_or(path)))
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.to_path_buf()
    };
    if let Some(parent) = expanded.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(meta)?;
    line.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&expanded)?
        .write_all(line.as_bytes())?;
    debug!(?expanded, "Wrote JSONL log entry");
    Ok(())
}

/// POST event metadata to a remote logging server (10s timeout, non-fatal).
async fn post_to_server(url: &url::Url, meta: &EventMeta) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!(%e, "Failed to build HTTP client");
            return;
        }
    };
    match client.post(url.as_str()).json(meta).send().await {
        Ok(resp) => debug!(status = %resp.status(), %url, "Log server POST complete"),
        Err(e) => warn!(%e, %url, "Log server POST failed"),
    }
}

/// Write a report line to stdout.
fn execute_report(handler: Option<&crate::events::ReportHandler>, meta: &EventMeta) {
    let output = match handler {
        Some(h) => format_report(h, meta),
        None => format!(
            "[{}] {} ({})",
            meta.event,
            meta.tool_name.as_deref().unwrap_or("-"),
            meta.provider
        ),
    };
    println!("{output}");
}

/// Format a report line according to the handler's configuration.
fn format_report(handler: &crate::events::ReportHandler, meta: &EventMeta) -> String {
    if let Some(template) = &handler.template {
        let mut output = interpolate(template, meta);
        if handler.include_metadata
            && let Ok(json) = serde_json::to_string(meta)
        {
            output.push(' ');
            output.push_str(&json);
        }
        return output;
    }
    match handler.format {
        ReportFormat::Json => serde_json::to_string(meta).unwrap_or_else(|_| "{}".to_string()),
        ReportFormat::Compact => format!(
            "[{}] {}",
            meta.event,
            meta.tool_name.as_deref().unwrap_or("-")
        ),
        ReportFormat::Text => format!(
            "Event: {}, Provider: {}, Tool: {}",
            meta.event,
            meta.provider,
            meta.tool_name.as_deref().unwrap_or("-")
        ),
    }
}

/// Run a command on the host system.
///
/// Validates the command exists on PATH via `sniff::programs::find_program`.
/// Non-fatal: logs a warning and returns `Ok(())` if the command is not found.
async fn execute_run(command: &str, args: Option<&[String]>, blocking: bool) -> Result<()> {
    if sniff::programs::find_program(command).is_none() {
        warn!(%command, "Run action skipped: command not found on PATH");
        return Ok(());
    }

    let mut cmd = tokio::process::Command::new(command);
    if let Some(args) = args {
        cmd.args(args);
    }

    if blocking {
        debug!(%command, "Running command (blocking)");
        match cmd.output().await {
            Ok(output) => {
                if !output.status.success() {
                    warn!(%command, status = %output.status, "Command exited with non-zero status");
                }
            }
            Err(e) => warn!(%command, %e, "Command execution failed"),
        }
    } else {
        debug!(%command, "Running command (fire-and-forget)");
        tokio::spawn(async move {
            match cmd.output().await {
                Ok(output) if !output.status.success() => {
                    warn!(status = %output.status, "Background command exited with non-zero status");
                }
                Err(e) => warn!(%e, "Background command execution failed"),
                _ => {}
            }
        });
    }

    Ok(())
}

/// Play a sound effect via playa (fire-and-forget).
fn execute_sound_effect(name: &str) {
    let effect = match playa::SoundEffect::from_name(name) {
        Some(e) => e,
        None => {
            warn!(%name, "Unknown sound effect name");
            return;
        }
    };
    debug!(%name, "Playing sound effect");
    tokio::spawn(async move {
        if let Err(e) = effect.play_async().await {
            warn!(%e, "Sound effect playback failed");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn meta() -> EventMeta {
        EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::BeforeTool,
            timestamp: Utc::now(),
            session_id: Some("test-session".to_string()),
            cwd: Some("/tmp".to_string()),
            tool_name: Some("Bash".to_string()),
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        }
    }

    fn log_action(path: std::path::PathBuf) -> Vec<EventAction> {
        vec![EventAction::Log {
            target: LogTarget::LocalFile { path },
        }]
    }

    #[tokio::test]
    async fn log_local_file_writes_jsonl() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("events.jsonl");
        execute_actions(
            &log_action(path.clone()),
            &meta(),
            &GlobalSettings::default(),
        )
        .await
        .unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: EventMeta = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(parsed.provider, Provider::Claude);
        assert_eq!(parsed.tool_name.as_deref(), Some("Bash"));
    }

    #[tokio::test]
    async fn log_local_file_creates_parent_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nested/deep/events.jsonl");
        execute_actions(
            &log_action(path.clone()),
            &meta(),
            &GlobalSettings::default(),
        )
        .await
        .unwrap();
        assert!(path.exists());
    }

    #[tokio::test]
    async fn log_local_file_appends() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("events.jsonl");
        let actions = log_action(path.clone());
        let m = meta();
        let s = GlobalSettings::default();
        execute_actions(&actions, &m, &s).await.unwrap();
        execute_actions(&actions, &m, &s).await.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.trim().split('\n').collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn report_default_format() {
        execute_report(None, &meta()); // verifies no panic
    }

    #[test]
    fn report_compact_with_template() {
        let h = ReportHandler {
            format: ReportFormat::Compact,
            template: Some("[TOOL] {tool_name}: executing".to_string()),
            include_metadata: false,
        };
        assert_eq!(format_report(&h, &meta()), "[TOOL] Bash: executing");
    }

    #[test]
    fn report_json_format() {
        let h = ReportHandler {
            format: ReportFormat::Json,
            template: None,
            include_metadata: false,
        };
        let _: serde_json::Value = serde_json::from_str(&format_report(&h, &meta())).unwrap();
    }

    #[test]
    fn report_text_format() {
        let h = ReportHandler {
            format: ReportFormat::Text,
            template: None,
            include_metadata: false,
        };
        let out = format_report(&h, &meta());
        assert!(out.contains("before_tool") && out.contains("Claude") && out.contains("Bash"));
    }

    #[tokio::test]
    async fn run_action_with_nonexistent_command() {
        let actions = vec![EventAction::Run {
            command: "this_command_surely_does_not_exist_12345".to_string(),
            args: None,
            blocking: false,
        }];
        // Should not panic — warns and returns Ok.
        execute_actions(&actions, &meta(), &GlobalSettings::default())
            .await
            .unwrap();
    }

    #[test]
    fn report_with_metadata() {
        let h = ReportHandler {
            format: ReportFormat::Compact,
            template: Some("{event}".to_string()),
            include_metadata: true,
        };
        let out = format_report(&h, &meta());
        assert!(out.starts_with("before_tool "));
        let _: serde_json::Value = serde_json::from_str(&out["before_tool ".len()..]).unwrap();
    }
}
