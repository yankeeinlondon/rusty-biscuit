use std::io::{IsTerminal, Read};

use clap::Args;
use color_eyre::eyre::{Result, bail};
use serde_json::Value;
use tracing::debug;

use claudine::events::{Provider, detect_environment};

/// Arguments for the handle subcommand.
#[derive(Args)]
pub struct HandleArgs {
    /// Event name (used for logging, the actual event is in the JSON payload).
    pub event: String,
    /// Optional provider hint (auto-detected from payload if not given).
    #[arg(long)]
    pub provider: Option<String>,
}

/// Handle an incoming event from stdin.
pub async fn run(args: HandleArgs) -> Result<()> {
    let raw = read_stdin_json()?;
    let provider = resolve_provider(args.provider.as_deref(), &raw)?;
    let cwd = std::env::current_dir().unwrap_or_default();
    let env = detect_environment(&cwd);

    debug!(%provider, event = %args.event, "Handling event");
    claudine::dispatch::dispatch(&raw, provider, &env).await?;
    Ok(())
}

fn read_stdin_json() -> Result<Value> {
    let mut stdin = std::io::stdin();
    if stdin.is_terminal() {
        bail!("No input provided. Pipe JSON event data to stdin.\n\nExample:\n  echo '{{\"hook_event_name\": \"...\", ...}}' | claudine handle <event>");
    }
    let mut buf = String::new();
    stdin.read_to_string(&mut buf)?;
    let raw: Value = serde_json::from_str(&buf)?;
    Ok(raw)
}

fn resolve_provider(hint: Option<&str>, raw: &Value) -> Result<Provider> {
    if let Some(name) = hint {
        return parse_provider(name);
    }

    // Auto-detect from payload structure
    if raw.get("hook_event_name").is_some() {
        return Ok(Provider::Claude);
    }
    if raw.get("type").is_some() && raw.get("thread_id").is_some() {
        return Ok(Provider::Codex);
    }
    if raw.get("event_type").is_some() {
        return Ok(Provider::OpenCode);
    }
    // Check for Gemini-style events
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
        other => bail!("Unknown provider: {other}"),
    }
}
