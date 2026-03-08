use std::io::{IsTerminal, Read};

use clap::Args;
use color_eyre::eyre::{Result, bail};
use serde_json::Value;
use tracing::debug;

use claudine::events::{PROVIDERS_DISPLAY_ORDER, Provider, detect_environment};

use crate::provider_values::provider_value_parser;

/// Arguments for the handle subcommand.
#[derive(Args)]
pub struct HandleArgs {
    /// Event name (used for logging, the actual event is in the JSON payload).
    pub event: String,
    /// Optional provider hint (auto-detected from payload if not given).
    #[arg(long, value_parser = provider_value_parser())]
    pub provider: Option<String>,

    /// Emit structured JSON output suitable for CI parsing.
    #[arg(long)]
    pub json: bool,
}

/// Handle an incoming event from stdin.
pub async fn run(args: HandleArgs) -> Result<()> {
    let raw = read_stdin_json()?;
    let provider = resolve_provider(args.provider.as_deref(), &raw)?;
    let cwd = std::env::current_dir().unwrap_or_default();
    let env = detect_environment(&cwd);

    debug!(%provider, event = %args.event, "Handling event");
    let outcome = claudine::dispatch::dispatch(&raw, provider, &env).await?;
    if args.json {
        let output = serde_json::json!({
            "provider": provider.as_slug(),
            "event": args.event,
            "response": outcome.response,
            "exit_code": outcome.exit_code,
            "protect_pre": outcome.protect_pre,
            "protect_post": outcome.protect_post,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if let Some(payload) = outcome.response {
        println!("{}", serde_json::to_string(&payload)?);
    }
    if let Some(exit_code) = outcome.exit_code {
        std::process::exit(exit_code);
    }
    Ok(())
}

fn read_stdin_json() -> Result<Value> {
    let mut stdin = std::io::stdin();
    if stdin.is_terminal() {
        bail!(
            "No input provided. Pipe JSON event data to stdin.\n\nExample:\n  echo '{{\"hook_event_name\": \"...\", ...}}' | claudine handle <event>"
        );
    }
    let mut buf = String::new();
    stdin.read_to_string(&mut buf)?;
    let raw: Value = serde_json::from_str(&buf)?;
    Ok(raw)
}

fn resolve_provider(hint: Option<&str>, raw: &Value) -> Result<Provider> {
    resolve_provider_inner(hint, raw, provider_from_wrapper_env())
}

fn provider_from_wrapper_env() -> Option<Provider> {
    std::env::var("AGENT")
        .ok()
        .or_else(|| std::env::var("Agent").ok())
        .as_deref()
        .and_then(parse_wrapper_env_provider)
}

fn parse_provider(name: &str) -> Result<Provider> {
    if let Some(provider) = Provider::parse_cli_name(name) {
        return Ok(provider);
    }

    let supported = PROVIDERS_DISPLAY_ORDER
        .iter()
        .map(Provider::as_slug)
        .collect::<Vec<_>>()
        .join(", ");
    bail!("Unknown provider: {name}. Supported: {supported}")
}

fn parse_wrapper_env_provider(value: &str) -> Option<Provider> {
    Provider::parse_cli_name(value)
}

fn resolve_provider_inner(
    hint: Option<&str>,
    raw: &Value,
    env_provider: Option<Provider>,
) -> Result<Provider> {
    if let Some(name) = hint {
        return parse_provider(name);
    }

    if let Some(provider) = env_provider {
        return Ok(provider);
    }

    if let Some(provider) = Provider::detect_from_payload(raw) {
        return Ok(provider);
    }

    bail!("Could not detect provider from payload. Use --provider to specify.")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn resolve_provider_prefers_explicit_hint() {
        let raw = json!({ "hook_event_name": "BeforeAgent" });
        let provider = resolve_provider_inner(Some("codex"), &raw, Some(Provider::Gemini)).unwrap();
        assert_eq!(provider, Provider::Codex);
    }

    #[test]
    fn resolve_provider_uses_wrapper_env_before_payload_detection() {
        let raw = json!({ "hook_event_name": "BeforeAgent" });
        let provider = resolve_provider_inner(None, &raw, Some(Provider::Gemini)).unwrap();
        assert_eq!(provider, Provider::Gemini);
    }

    #[test]
    fn resolve_provider_falls_back_to_payload_detection() {
        let raw = json!({ "type": "thread.started", "thread_id": "t-1" });
        let provider = resolve_provider_inner(None, &raw, None).unwrap();
        assert_eq!(provider, Provider::Codex);
    }

    #[test]
    fn parse_wrapper_env_provider_accepts_aliases() {
        assert_eq!(
            super::parse_wrapper_env_provider("open-code"),
            Some(Provider::OpenCode)
        );
        assert_eq!(
            super::parse_wrapper_env_provider("gemini"),
            Some(Provider::Gemini)
        );
        assert_eq!(super::parse_wrapper_env_provider("nope"), None);
    }
}
