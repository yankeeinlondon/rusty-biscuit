use std::io::Read;

use clap::Args;
use color_eyre::eyre::{Result, bail};
use serde_json::Value;

use claudine_lib::adapters;
use claudine_lib::dispatch::template::interpolate;
use claudine_lib::events::{EventAction, Provider, detect_environment};

use crate::log;

/// Arguments for the dry-run subcommand.
#[derive(Args)]
pub struct DryRunArgs {
    /// Event name to simulate.
    pub event: String,
    /// Optional provider hint.
    #[arg(long)]
    pub provider: Option<String>,
}

/// Show what would happen for an event without side effects.
pub async fn run(args: DryRunArgs) -> Result<()> {
    let raw = read_stdin_json()?;
    let provider = resolve_provider(args.provider.as_deref(), &raw)?;
    let cwd = std::env::current_dir().unwrap_or_default();
    let env = detect_environment(&cwd);

    let adapter = adapters::adapter_for(provider);
    let parsed = adapter.parse_event(&raw, &env);

    match parsed {
        Some((event, meta)) => {
            log::data(&format!("Provider: {provider}"));
            log::data(&format!("Event:    {event}"));
            if let Some(tool) = &meta.tool_name {
                log::data(&format!("Tool:     {tool}"));
            }
            if let Some(sid) = &meta.session_id {
                log::data(&format!("Session:  {sid}"));
            }
            log::data("");

            // Try to load config and show what would fire
            let config = claudine_lib::dispatch::loader::load_config(None, None);
            match config {
                Ok(cfg) => {
                    if let Some(binding) = cfg.events.get(&event) {
                        log::data("Matching binding found:");
                        log::data(&format!("  Enabled: {}", binding.enabled));
                        log::data(&format!("  Actions: {}", binding.actions.len()));
                        for (i, action) in binding.actions.iter().enumerate() {
                            match action {
                                EventAction::Speak { message } => {
                                    let resolved = interpolate(message, &meta);
                                    log::data(&format!("  [{i}] Speak: \"{resolved}\""));
                                }
                                EventAction::Log { target } => {
                                    log::data(&format!("  [{i}] Log: {target:?}"));
                                }
                                EventAction::Report { handler } => {
                                    log::data(&format!("  [{i}] Report: {handler:?}"));
                                }
                                EventAction::SoundEffect { name, .. } => {
                                    log::data(&format!("  [{i}] SoundEffect: {name}"));
                                }
                            }
                        }
                    } else {
                        log::data(&format!("No binding found for event '{event}'"));
                    }
                }
                Err(e) => {
                    log::data(&format!("No config loaded: {e}"));
                    log::data("(Create config with `claudine init`)");
                }
            }
        }
        None => {
            log::data("Adapter returned None for this payload (unknown event).");
        }
    }

    Ok(())
}

fn read_stdin_json() -> Result<Value> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let raw: Value = serde_json::from_str(&buf)?;
    Ok(raw)
}

fn resolve_provider(hint: Option<&str>, raw: &Value) -> Result<Provider> {
    if let Some(name) = hint {
        return parse_provider(name);
    }
    if raw.get("hook_event_name").is_some() {
        return Ok(Provider::Claude);
    }
    if raw.get("type").is_some() && raw.get("thread_id").is_some() {
        return Ok(Provider::Codex);
    }
    if raw.get("event_type").is_some() {
        return Ok(Provider::OpenCode);
    }
    if raw.get("event_name").is_some() {
        return Ok(Provider::Gemini);
    }
    bail!("Could not detect provider from payload. Use --provider to specify.")
}

fn parse_provider(name: &str) -> Result<Provider> {
    match name.to_lowercase().as_str() {
        "claude" => Ok(Provider::Claude),
        "codex" => Ok(Provider::Codex),
        "gemini" => Ok(Provider::Gemini),
        "opencode" | "open_code" => Ok(Provider::OpenCode),
        "roo" | "roo_code" | "roocode" => Ok(Provider::RooCode),
        other => bail!("Unknown provider: {other}"),
    }
}
