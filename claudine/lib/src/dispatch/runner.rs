use std::io::Write;
use std::time::Duration;

use regex::Regex;
use tokio::process::Command;
use tracing::{debug, warn};

use super::template::interpolate;
use crate::actions::{
    CompiledMapper, HookAction, HookDecision, HookResponse, LogTarget, Mapper, ReportFormat,
    ReportHandler,
};
use crate::error::Result;
use crate::events::{EventMeta, GlobalSettings};

/// Execute hook actions in declaration order.
///
/// Returns the selected blocking response from `call` actions when applicable.
pub async fn execute_actions(
    actions: &[HookAction],
    compiled_mappers: Option<&[Option<CompiledMapper>]>,
    meta: &EventMeta,
    settings: &GlobalSettings,
    can_block: bool,
) -> Result<Option<HookResponse>> {
    let mut selected_response: Option<HookResponse> = None;

    for (index, action) in actions.iter().enumerate() {
        match action {
            HookAction::Speak { message } => execute_speak(message, meta),
            HookAction::Log { target } => {
                let target = resolve_log_target(target, settings);
                execute_log(target, meta).await?
            }
            HookAction::Report { handler } => execute_report(handler.as_ref(), meta),
            HookAction::FireAndForget { command, args } => {
                execute_fire_and_forget(command, args.as_deref(), meta)
            }
            HookAction::Call {
                command,
                args,
                timeout_ms,
                mapper,
            } => {
                let cmd = interpolate(command, meta);
                let rendered_args = args.as_ref().map(|items| {
                    items
                        .iter()
                        .map(|arg| interpolate(arg, meta))
                        .collect::<Vec<_>>()
                });

                let timeout = timeout_ms
                    .map(Duration::from_millis)
                    .unwrap_or(Duration::from_secs(60));

                let compiled_mapper = compiled_mappers
                    .and_then(|mappers| mappers.get(index))
                    .and_then(Option::as_ref);

                match tokio::time::timeout(
                    timeout,
                    run_command_blocking(&cmd, rendered_args.as_deref()),
                )
                .await
                {
                    Ok(Ok(output)) => match apply_mapper(compiled_mapper, mapper.as_ref(), &output)
                    {
                        Ok(response) => {
                            if can_block {
                                if should_replace_selected(selected_response.as_ref(), &response) {
                                    selected_response = Some(response);
                                }
                            } else {
                                debug!(%cmd, "Call response produced on non-blocking event and discarded");
                            }
                        }
                        Err(error) => {
                            warn!(%cmd, %error, "Call mapper failed");
                        }
                    },
                    Ok(Err(error)) => {
                        warn!(%cmd, %error, "Call command failed");
                    }
                    Err(_) => {
                        warn!(%cmd, timeout_ms = timeout.as_millis(), "Call command timed out");
                    }
                }
            }
            HookAction::SoundEffect {
                name,
                volume,
                speed,
            } => execute_sound_effect(name, *volume, *speed),
        }
    }

    Ok(selected_response)
}

fn resolve_log_target<'a>(target: &'a LogTarget, settings: &'a GlobalSettings) -> &'a LogTarget {
    match (target, settings.default_log_target.as_ref()) {
        (LogTarget::File { path: None, .. }, Some(default_target)) => default_target,
        _ => target,
    }
}

fn should_replace_selected(current: Option<&HookResponse>, candidate: &HookResponse) -> bool {
    match current {
        None => true,
        Some(existing) => {
            let existing_is_continue = matches!(existing.decision, Some(HookDecision::Continue));
            let candidate_is_continue = matches!(candidate.decision, Some(HookDecision::Continue));
            existing_is_continue && !candidate_is_continue
        }
    }
}

/// Speak a message via biscuit-speaks TTS (fire-and-forget).
fn execute_speak(message_template: &str, meta: &EventMeta) {
    let text = interpolate(message_template, meta);
    if text.is_empty() {
        return;
    }

    tokio::spawn(async move {
        if let Err(error) = biscuit_speaks::Speak::new(text).play().await {
            warn!(%error, "TTS playback failed");
        }
    });
}

fn execute_sound_effect(name: &str, volume: f32, speed: f32) {
    let effect = match playa::SoundEffect::from_name(name) {
        Some(effect) => effect,
        None => {
            warn!(%name, "Unknown sound effect name");
            return;
        }
    };

    tokio::task::spawn_blocking(move || {
        let playback = match playa::Playa::from_bytes(effect.bytes().to_vec()) {
            Ok(player) => player.volume(volume).speed(speed).play(),
            Err(error) => {
                warn!(%error, "Failed to construct sound effect player");
                return;
            }
        };
        if let Err(error) = playback {
            warn!(%error, "Sound effect playback failed");
        }
    });
}

fn execute_fire_and_forget(command: &str, args: Option<&[String]>, meta: &EventMeta) {
    let cmd = interpolate(command, meta);
    let rendered_args: Option<Vec<String>> = args.map(|items| {
        items
            .iter()
            .map(|arg| interpolate(arg, meta))
            .collect::<Vec<_>>()
    });

    tokio::spawn(async move {
        if let Err(error) = run_command_blocking(&cmd, rendered_args.as_deref()).await {
            warn!(%cmd, %error, "Fire-and-forget command failed");
        }
    });
}

async fn execute_log(target: &LogTarget, meta: &EventMeta) -> Result<()> {
    match target {
        LogTarget::File { path, rotate_daily } => {
            let resolved = resolve_file_log_path(path.as_deref(), *rotate_daily);
            write_jsonl(&resolved, meta)
        }
        LogTarget::Server {
            url,
            timeout_ms,
            headers,
        } => {
            post_to_server(url, *timeout_ms, headers.as_ref(), meta).await;
            Ok(())
        }
    }
}

fn resolve_file_log_path(path: Option<&std::path::Path>, rotate_daily: bool) -> std::path::PathBuf {
    if let Some(path) = path {
        return expand_tilde(path);
    }

    let base = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~"))
        .join(".claudine")
        .join("logs");

    if rotate_daily {
        let file = format!("{}.jsonl", chrono::Local::now().format("%Y-%m-%d"));
        base.join(file)
    } else {
        base.join("events.jsonl")
    }
}

fn expand_tilde(path: &std::path::Path) -> std::path::PathBuf {
    if path.starts_with("~")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(path.strip_prefix("~").unwrap_or(path));
    }
    path.to_path_buf()
}

fn write_jsonl(path: &std::path::Path, meta: &EventMeta) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut line = serde_json::to_string(meta)?;
    line.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(line.as_bytes())?;

    Ok(())
}

async fn post_to_server(
    url: &str,
    timeout_ms: u64,
    headers: Option<&std::collections::HashMap<String, String>>,
    meta: &EventMeta,
) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            warn!(%error, "Failed to build HTTP client for log target");
            return;
        }
    };

    let mut request = client.post(url).json(meta);
    if let Some(headers) = headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }

    if let Err(error) = request.send().await {
        warn!(%error, %url, "Log server POST failed");
    }
}

fn execute_report(handler: Option<&ReportHandler>, meta: &EventMeta) {
    let output = match handler {
        Some(handler) => format_report(handler, meta),
        None => format!(
            "[{}] {} ({})",
            meta.event,
            meta.tool_name.as_deref().unwrap_or("-"),
            meta.provider
        ),
    };
    println!("{output}");
}

fn format_report(handler: &ReportHandler, meta: &EventMeta) -> String {
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

#[derive(Debug)]
struct CommandOutput {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

async fn run_command_blocking(command: &str, args: Option<&[String]>) -> Result<CommandOutput> {
    if sniff::programs::find_program(command).is_none() {
        return Err(crate::error::ClaudineError::LinkingError(format!(
            "command not found on PATH: {command}"
        )));
    }

    let mut cmd = Command::new(command);
    if let Some(args) = args {
        cmd.args(args);
    }

    let output = cmd.output().await?;
    Ok(CommandOutput {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

fn apply_mapper(
    compiled_mapper: Option<&CompiledMapper>,
    fallback_mapper: Option<&Mapper>,
    output: &CommandOutput,
) -> Result<HookResponse> {
    if let Some(compiled_mapper) = compiled_mapper {
        return match compiled_mapper {
            CompiledMapper::JsonField { field } => map_json_field(field, output),
            CompiledMapper::JsonObject => map_json_object(output),
            CompiledMapper::ExitCode => Ok(map_exit_code(output)),
            CompiledMapper::Regex { pattern } => map_regex_with_compiled(pattern, output),
        };
    }

    match fallback_mapper.unwrap_or(&Mapper::ExitCode) {
        Mapper::ExitCode => Ok(map_exit_code(output)),
        Mapper::JsonField { field } => map_json_field(field, output),
        Mapper::JsonObject => map_json_object(output),
        Mapper::Regex { pattern } => {
            let regex = Regex::new(pattern)?;
            map_regex_with_compiled(&regex, output)
        }
    }
}

fn map_exit_code(output: &CommandOutput) -> HookResponse {
    let code = output.status.code().unwrap_or(1);
    let decision = match code {
        0 => Some(HookDecision::Allow),
        2 => Some(HookDecision::Deny),
        _ => Some(HookDecision::Allow),
    };

    let reason = if !output.stdout.is_empty() {
        Some(output.stdout.clone())
    } else if !output.stderr.is_empty() {
        Some(output.stderr.clone())
    } else {
        None
    };

    HookResponse {
        decision,
        reason,
        ..HookResponse::default()
    }
}

fn map_json_field(field: &str, output: &CommandOutput) -> Result<HookResponse> {
    let parsed: serde_json::Value = serde_json::from_str(&output.stdout)?;
    let value = dot_lookup(&parsed, field).ok_or_else(|| {
        crate::error::ClaudineError::TemplateError(format!("mapper field not found: {field}"))
    })?;

    let decision = parse_decision(value);
    let reason = parsed
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);

    Ok(HookResponse {
        decision,
        reason,
        ..HookResponse::default()
    })
}

fn map_json_object(output: &CommandOutput) -> Result<HookResponse> {
    if output.stdout.is_empty() {
        return Ok(HookResponse::default());
    }

    if let Ok(response) = serde_json::from_str::<HookResponse>(&output.stdout) {
        return Ok(response);
    }

    let raw = serde_json::from_str::<serde_json::Value>(&output.stdout)?;
    Ok(HookResponse {
        raw: Some(raw),
        ..HookResponse::default()
    })
}

fn map_regex_with_compiled(regex: &Regex, output: &CommandOutput) -> Result<HookResponse> {
    let captures = regex.captures(&output.stdout).ok_or_else(|| {
        crate::error::ClaudineError::TemplateError("regex mapper produced no match".to_string())
    })?;

    let decision = captures
        .name("decision")
        .map(|capture| parse_decision(&serde_json::Value::String(capture.as_str().to_string())))
        .unwrap_or(None);
    let reason = captures
        .name("reason")
        .map(|capture| capture.as_str().to_string());

    Ok(HookResponse {
        decision,
        reason,
        additional_context: captures
            .name("context")
            .map(|capture| capture.as_str().to_string()),
        ..HookResponse::default()
    })
}

fn parse_decision(value: &serde_json::Value) -> Option<HookDecision> {
    let text = value.as_str()?.to_ascii_lowercase();
    match text.as_str() {
        "allow" | "approved" | "approve" => Some(HookDecision::Allow),
        "deny" | "denied" | "reject" | "rejected" => Some(HookDecision::Deny),
        "ask" => Some(HookDecision::Ask),
        "continue" => Some(HookDecision::Continue),
        _ => None,
    }
}

fn dot_lookup<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    path.split('.').try_fold(value, |acc, key| acc.get(key))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::os::unix::process::ExitStatusExt;

    use chrono::Utc;

    use super::*;
    use crate::events::{EnvironmentContext, Provider};

    fn meta() -> EventMeta {
        EventMeta {
            provider: Provider::Claude,
            event: crate::events::AgenticEvent::BeforeTool,
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

    #[tokio::test]
    async fn log_file_writes_jsonl() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("events.jsonl");

        let actions = vec![HookAction::Log {
            target: LogTarget::File {
                path: Some(path.clone()),
                rotate_daily: false,
            },
        }];

        execute_actions(&actions, None, &meta(), &GlobalSettings::default(), false)
            .await
            .unwrap();

        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("before_tool"));
    }

    #[test]
    fn mapper_exit_code_deny() {
        let output = CommandOutput {
            status: std::process::ExitStatus::from_raw(2 << 8),
            stdout: "blocked".to_string(),
            stderr: String::new(),
        };

        let mapped = apply_mapper(None, None, &output).unwrap();
        assert_eq!(mapped.decision, Some(HookDecision::Deny));
        assert_eq!(mapped.reason.as_deref(), Some("blocked"));
    }

    #[test]
    fn mapper_json_field() {
        let output = CommandOutput {
            status: std::process::ExitStatus::from_raw(0),
            stdout: r#"{"decision":"deny","reason":"nope"}"#.to_string(),
            stderr: String::new(),
        };

        let mapped = apply_mapper(
            None,
            Some(&Mapper::JsonField {
                field: "decision".to_string(),
            }),
            &output,
        )
        .unwrap();

        assert_eq!(mapped.decision, Some(HookDecision::Deny));
        assert_eq!(mapped.reason.as_deref(), Some("nope"));
    }

    #[test]
    fn stop_overrides_continue() {
        let continue_response = HookResponse {
            decision: Some(HookDecision::Continue),
            ..HookResponse::default()
        };
        let deny_response = HookResponse {
            decision: Some(HookDecision::Deny),
            ..HookResponse::default()
        };

        assert!(should_replace_selected(
            Some(&continue_response),
            &deny_response
        ));
        assert!(!should_replace_selected(
            Some(&deny_response),
            &continue_response
        ));
    }
}
