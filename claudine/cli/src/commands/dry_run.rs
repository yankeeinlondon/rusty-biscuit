use std::io::{IsTerminal, Read};

use clap::Args;
use color_eyre::eyre::{Result, bail};
use serde_json::{Value, json};

use claudine::adapters;
use claudine::dispatch::template::interpolate;
use claudine::events::{AgenticEvent, HookAction, Provider, detect_environment};

use crate::log;

/// Arguments for the dry-run subcommand.
#[derive(Args)]
#[command(after_help = r#"EXAMPLES:
    # Test with a mock event (no stdin needed)
    claudine dry-run turn_complete
    claudine dry-run permission_request
    claudine dry-run tool_error

    # Test with custom JSON payload
    echo '{"hook_event_name":"Stop","session_id":"test"}' | claudine dry-run turn_complete

    # Specify provider explicitly
    claudine dry-run turn_complete --provider codex

AVAILABLE EVENTS:
    session_start, session_end, before_prompt, before_tool, after_tool,
    tool_error, permission_request, turn_complete, turn_error,
    subagent_start, subagent_stop, before_model, after_model,
    before_compact, notification
"#)]
pub struct DryRunArgs {
    /// Event name to simulate (e.g., turn_complete, permission_request).
    pub event: String,

    /// Provider to simulate (default: claude).
    ///
    /// Supported: claude, codex, gemini, opencode
    #[arg(long, default_value = "claude")]
    pub provider: String,
}

/// Show what would happen for an event without side effects.
pub async fn run(args: DryRunArgs) -> Result<()> {
    let provider = parse_provider(&args.provider)?;
    let event = parse_event(&args.event)?;

    // Use stdin if piped, otherwise use mock payload
    let raw = if std::io::stdin().is_terminal() {
        mock_payload(provider, &event)
    } else {
        read_stdin_json()?
    };

    let cwd = std::env::current_dir().unwrap_or_default();
    let env = detect_environment(&cwd);

    let adapter = adapters::adapter_for(provider);
    let parsed = adapter.parse_event(&raw);

    match parsed {
        Ok((parsed_event, mut meta)) => {
            meta.env = env;
            log::data(&format!("Provider: {provider}"));
            log::data(&format!("Event:    {parsed_event}"));
            if let Some(tool) = &meta.tool_name {
                log::data(&format!("Tool:     {tool}"));
            }
            if let Some(sid) = &meta.session_id {
                log::data(&format!("Session:  {sid}"));
            }
            if let Some(prompt) = &meta.prompt {
                log::data(&format!("Prompt:   {prompt}"));
            }
            if let Some(error) = &meta.error {
                log::data(&format!("Error:    {error}"));
            }
            log::data("");

            // Try to load config and show what would fire
            let config = claudine::dispatch::loader::load_config(None, None);
            match config {
                Ok(cfg) => {
                    if let Some(binding) = cfg.get_binding(provider, &parsed_event) {
                        log::data("Matching binding found:");
                        log::data(&format!("  Enabled: {}", binding.enabled));
                        log::data(&format!("  Actions: {}", binding.actions.len()));
                        for (i, action) in binding.actions.iter().enumerate() {
                            match action {
                                HookAction::Speak { message } => {
                                    let resolved = interpolate(message, &meta);
                                    log::data(&format!("  [{i}] Speak: \"{resolved}\""));
                                }
                                HookAction::Log { target } => {
                                    log::data(&format!("  [{i}] Log: {target:?}"));
                                }
                                HookAction::Report { handler } => {
                                    log::data(&format!("  [{i}] Report: {handler:?}"));
                                }
                                HookAction::FireAndForget { command, args } => {
                                    let args_str =
                                        args.as_ref().map(|a| a.join(" ")).unwrap_or_default();
                                    log::data(&format!("  [{i}] FireAndForget: {command} {args_str}"));
                                }
                                HookAction::Call {
                                    command,
                                    args,
                                    timeout_ms,
                                    mapper,
                                } => {
                                    let args_str =
                                        args.as_ref().map(|a| a.join(" ")).unwrap_or_default();
                                    log::data(&format!(
                                        "  [{i}] Call: {command} {args_str} (timeout={timeout_ms:?}, mapper={mapper:?})"
                                    ));
                                }
                                HookAction::SoundEffect {
                                    name,
                                    volume,
                                    speed,
                                } => {
                                    log::data(&format!(
                                        "  [{i}] SoundEffect: {name} (vol={volume}, speed={speed})"
                                    ));
                                }
                                _ => log::data(&format!("  [{i}] Other action: {action:?}")),
                            }
                        }
                    } else {
                        log::data(&format!("No binding found for {provider}/{parsed_event}"));
                    }
                }
                Err(e) => {
                    log::data(&format!("No config loaded: {e}"));
                    log::data("(Create config with `claudine init`)");
                }
            }
        }
        Err(error) => {
            log::data(&format!("Adapter parse failed: {error}"));
        }
    }

    Ok(())
}

/// Parse event name from string (supports both canonical and native names).
fn parse_event(name: &str) -> Result<AgenticEvent> {
    // Normalize: convert PascalCase to snake_case, handle separators
    let mut normalized = String::new();
    let mut prev_was_lower = false;

    for c in name.chars() {
        if c == '-' || c == '_' {
            if !normalized.ends_with('_') {
                normalized.push('_');
            }
            prev_was_lower = false;
        } else if c.is_uppercase() {
            // Only add underscore if previous char was lowercase (PascalCase boundary)
            if prev_was_lower && !normalized.ends_with('_') {
                normalized.push('_');
            }
            normalized.push(c.to_ascii_lowercase());
            prev_was_lower = false;
        } else {
            normalized.push(c);
            prev_was_lower = c.is_lowercase();
        }
    }

    let event = match normalized.as_str() {
        "session_start" => AgenticEvent::SessionStart,
        "session_end" => AgenticEvent::SessionEnd,
        "before_prompt" | "user_prompt_submit" => AgenticEvent::BeforePrompt,
        "before_tool" | "pre_tool_use" => AgenticEvent::BeforeTool,
        "after_tool" | "post_tool_use" => AgenticEvent::AfterTool,
        "tool_error" | "post_tool_use_failure" => AgenticEvent::ToolError,
        "permission_request" => AgenticEvent::PermissionRequest,
        "turn_complete" | "stop" => AgenticEvent::TurnComplete,
        "turn_error" => AgenticEvent::TurnError,
        "subagent_start" => AgenticEvent::SubagentStart,
        "subagent_stop" => AgenticEvent::SubagentStop,
        "before_model" => AgenticEvent::BeforeModel,
        "after_model" => AgenticEvent::AfterModel,
        "before_compact" | "pre_compact" => AgenticEvent::BeforeCompact,
        "notification" => AgenticEvent::Notification,
        other => bail!(
            "Unknown event: {other}\n\n\
            Available events:\n  \
            session_start, session_end, before_prompt, before_tool, after_tool,\n  \
            tool_error, permission_request, turn_complete, turn_error,\n  \
            subagent_start, subagent_stop, before_model, after_model,\n  \
            before_compact, notification"
        ),
    };
    Ok(event)
}

/// Generate a mock payload for testing an event.
fn mock_payload(provider: Provider, event: &AgenticEvent) -> Value {
    match provider {
        Provider::Claude => mock_claude_payload(event),
        Provider::Codex => mock_codex_payload(event),
        Provider::Gemini => mock_gemini_payload(event),
        Provider::OpenCode => mock_opencode_payload(event),
        Provider::Goose
        | Provider::KimiCode
        | Provider::QwenCode
        | Provider::RooCode => mock_claude_payload(event),
        _ => mock_claude_payload(event),
    }
}

/// Mock payload in Claude format.
fn mock_claude_payload(event: &AgenticEvent) -> Value {
    let hook_event_name = match event {
        AgenticEvent::SessionStart => "SessionStart",
        AgenticEvent::SessionEnd => "SessionEnd",
        AgenticEvent::BeforePrompt => "UserPromptSubmit",
        AgenticEvent::BeforeTool => "PreToolUse",
        AgenticEvent::AfterTool => "PostToolUse",
        AgenticEvent::ToolError => "PostToolUseFailure",
        AgenticEvent::PermissionRequest => "PermissionRequest",
        AgenticEvent::TurnComplete => "Stop",
        AgenticEvent::SubagentStart => "SubagentStart",
        AgenticEvent::SubagentStop => "SubagentStop",
        AgenticEvent::BeforeCompact => "PreCompact",
        AgenticEvent::Notification => "Notification",
        _ => "Stop", // Fallback for unsupported events
    };

    let mut payload = json!({
        "hook_event_name": hook_event_name,
        "session_id": "mock-session-001",
        "cwd": "/home/user/project"
    });

    // Add event-specific fields
    match event {
        AgenticEvent::BeforePrompt => {
            payload["prompt"] = json!("Fix the bug in main.rs");
        }
        AgenticEvent::BeforeTool | AgenticEvent::AfterTool | AgenticEvent::PermissionRequest => {
            payload["tool_name"] = json!("Bash");
            payload["tool_input"] = json!({"command": "cargo build"});
        }
        AgenticEvent::ToolError => {
            payload["tool_name"] = json!("Bash");
            payload["error"] = json!("command failed with exit code 1");
        }
        AgenticEvent::SubagentStart | AgenticEvent::SubagentStop => {
            payload["agent_type"] = json!("task");
        }
        AgenticEvent::Notification => {
            payload["notification"] = json!({
                "type": "info",
                "message": "Context window 80% full"
            });
        }
        _ => {}
    }

    payload
}

/// Mock payload in Codex format.
fn mock_codex_payload(event: &AgenticEvent) -> Value {
    let type_field = match event {
        AgenticEvent::BeforeTool => "item.started",
        AgenticEvent::AfterTool => "item.completed",
        AgenticEvent::BeforePrompt => "turn.started",
        AgenticEvent::TurnError => "turn.failed",
        _ => "turn.completed",
    };

    let mut payload = json!({
        "type": type_field,
        "thread_id": "mock-thread-001",
        "session_id": "mock-session-001"
    });

    if type_field == "item.started" {
        payload["item"] = json!({
            "type": "command_execution",
            "name": "shell"
        });
    }
    if type_field == "item.completed" {
        payload["item"] = json!({
            "type": "command_execution",
            "name": "shell",
            "output": {"exit_code": 0}
        });
    }

    payload
}

/// Mock payload in Gemini format.
fn mock_gemini_payload(event: &AgenticEvent) -> Value {
    let hook_event_name = match event {
        AgenticEvent::BeforePrompt => "BeforeAgent",
        AgenticEvent::BeforeTool => "BeforeTool",
        AgenticEvent::AfterTool => "AfterTool",
        AgenticEvent::BeforeModel => "BeforeModel",
        AgenticEvent::AfterModel => "AfterModel",
        AgenticEvent::BeforeCompact => "PreCompress",
        _ => "AfterAgent",
    };

    json!({
        "hook_event_name": hook_event_name,
        "session_id": "mock-session-001"
    })
}

/// Mock payload in OpenCode format.
fn mock_opencode_payload(event: &AgenticEvent) -> Value {
    let event_type = match event {
        AgenticEvent::SessionStart => "session.created",
        AgenticEvent::SessionEnd => "session.deleted",
        AgenticEvent::BeforePrompt => "chat.message",
        AgenticEvent::BeforeTool => "tool.execute.before",
        AgenticEvent::AfterTool => "tool.execute.after",
        AgenticEvent::PermissionRequest => "permission.ask",
        AgenticEvent::HumanInTheLoop => "permission.asked",
        AgenticEvent::TurnError => "session.error",
        AgenticEvent::BeforeCompact => "session.compacted",
        AgenticEvent::BeforeModel => "chat.params",
        AgenticEvent::AfterModel => "experimental.text.complete",
        AgenticEvent::Notification => "event",
        _ => "session.idle",
    };

    json!({
        "event_type": event_type,
        "session_id": "mock-session-001"
    })
}

fn read_stdin_json() -> Result<Value> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let raw: Value = serde_json::from_str(&buf)?;
    Ok(raw)
}

fn parse_provider(name: &str) -> Result<Provider> {
    match name.to_lowercase().as_str() {
        "claude" => Ok(Provider::Claude),
        "codex" => Ok(Provider::Codex),
        "gemini" => Ok(Provider::Gemini),
        "goose" => Ok(Provider::Goose),
        "kimi" | "kimicode" | "kimi_code" => Ok(Provider::KimiCode),
        "opencode" | "open_code" => Ok(Provider::OpenCode),
        "qwen" | "qwencode" | "qwen_code" => Ok(Provider::QwenCode),
        "roo" | "roo_code" | "roocode" => Ok(Provider::RooCode),
        other => bail!(
            "Unknown provider: {other}\n\nSupported: claude, codex, gemini, goose, kimi, opencode, qwen, roo"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_event_canonical_names() {
        assert_eq!(
            parse_event("turn_complete").unwrap(),
            AgenticEvent::TurnComplete
        );
        assert_eq!(
            parse_event("permission_request").unwrap(),
            AgenticEvent::PermissionRequest
        );
        assert_eq!(parse_event("tool_error").unwrap(), AgenticEvent::ToolError);
        assert_eq!(
            parse_event("session_start").unwrap(),
            AgenticEvent::SessionStart
        );
    }

    #[test]
    fn parse_event_native_names() {
        // Claude native names
        assert_eq!(parse_event("Stop").unwrap(), AgenticEvent::TurnComplete);
        assert_eq!(parse_event("PreToolUse").unwrap(), AgenticEvent::BeforeTool);
        assert_eq!(
            parse_event("PostToolUseFailure").unwrap(),
            AgenticEvent::ToolError
        );
    }

    #[test]
    fn parse_event_case_insensitive() {
        assert_eq!(
            parse_event("TURN_COMPLETE").unwrap(),
            AgenticEvent::TurnComplete
        );
        assert_eq!(
            parse_event("Turn_Complete").unwrap(),
            AgenticEvent::TurnComplete
        );
    }

    #[test]
    fn parse_event_hyphen_or_underscore() {
        assert_eq!(
            parse_event("turn-complete").unwrap(),
            AgenticEvent::TurnComplete
        );
        assert_eq!(
            parse_event("before-tool").unwrap(),
            AgenticEvent::BeforeTool
        );
    }

    #[test]
    fn parse_event_unknown_fails() {
        assert!(parse_event("not_an_event").is_err());
    }

    #[test]
    fn mock_claude_payload_has_required_fields() {
        let payload = mock_claude_payload(&AgenticEvent::TurnComplete);
        assert_eq!(payload["hook_event_name"], "Stop");
        assert!(payload["session_id"].is_string());
    }

    #[test]
    fn mock_claude_payload_tool_events_have_tool_name() {
        let payload = mock_claude_payload(&AgenticEvent::BeforeTool);
        assert_eq!(payload["tool_name"], "Bash");
        assert!(payload["tool_input"].is_object());
    }

    #[test]
    fn mock_claude_payload_error_has_error_field() {
        let payload = mock_claude_payload(&AgenticEvent::ToolError);
        assert!(payload["error"].is_string());
    }
}
