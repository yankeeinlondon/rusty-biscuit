use std::io::{IsTerminal, Read};
use std::time::Duration;

use clap::Args;
use color_eyre::eyre::{Result, bail};
use serde_json::Value;
use tracing::{debug, info_span};

use claudine::events::detect_environment_fast;

use claudine::provider::Provider;
use crate::cli_utils::parse_provider;
use crate::provider_values::provider_value_parser;

/// Default overall execution deadline for a single `claudine handle` invocation.
///
/// Hook handlers run inside the parent agent's event pipeline. Claude Code,
/// Gemini CLI, and others kill the handler at ~30s, so we cap ourselves well
/// below that to stop blocking the agent session. The value is deliberately
/// above the default 60s `Call` action timeout would imply — callers who run a
/// long `Call` must raise the env override below.
const DEFAULT_HANDLE_DEADLINE_SECONDS: u64 = 15;

/// Exit code for deadline timeouts.
///
/// Matches the `coreutils timeout` convention so any shell or agent inspecting
/// the exit code sees a recognizable "operation timed out" signal.
const EXIT_CODE_DEADLINE_EXCEEDED: i32 = 124;

/// Arguments for the handle subcommand.
#[derive(Args)]
pub struct HandleArgs {
    /// Event name (used for logging, the actual event is in the JSON payload).
    ///
    /// Optional — when omitted (e.g. Codex `notify` hooks), the event label
    /// defaults to `"event"` and the actual event type is derived from the
    /// JSON payload by the provider adapter.
    pub event: Option<String>,
    /// Optional provider hint (auto-detected from payload if not given).
    #[arg(long, value_parser = provider_value_parser())]
    pub provider: Option<Provider>,

    /// Emit structured JSON output suitable for CI parsing.
    #[arg(long)]
    pub json: bool,
}

fn resolve_deadline() -> Duration {
    let secs = std::env::var("CLAUDINE_HANDLE_DEADLINE_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_HANDLE_DEADLINE_SECONDS);
    Duration::from_secs(secs)
}

/// Handle an incoming event from stdin under a hard execution deadline.
///
/// ## Deadline semantics
///
/// Real work runs inside [`run_inner`], wrapped in [`tokio::time::timeout`]
/// sized by `CLAUDINE_HANDLE_DEADLINE_SECONDS` (default 15s). When the
/// deadline elapses, the handler prints a one-line diagnostic to stderr and
/// exits with code 124 (`coreutils timeout` convention) so the parent agent
/// classifies the handler as "failed" instead of waiting for its own ~30s
/// hook timeout.
///
/// ## Exit discipline
///
/// Hook handlers run as short-lived children of an interactive agent. The
/// top-level command owns stdout/stderr flushing plus the final
/// [`std::process::exit`] so inner async helpers never bypass buffered
/// machine-readable output.
pub async fn run(args: HandleArgs) -> Result<()> {
    let deadline = resolve_deadline();

    match tokio::time::timeout(deadline, run_inner(args)).await {
        Ok(Ok(exit_code)) => {
            flush_streams();
            std::process::exit(exit_code);
        }
        Ok(Err(error)) => {
            flush_streams();
            Err(error)
        }
        Err(_elapsed) => {
            eprintln!(
                "claudine handle: deadline exceeded after {}s; aborting hook handler \
                 to prevent blocking the agent session (set \
                 CLAUDINE_HANDLE_DEADLINE_SECONDS to override)",
                deadline.as_secs()
            );
            flush_streams();
            std::process::exit(EXIT_CODE_DEADLINE_EXCEEDED);
        }
    }
}

fn flush_streams() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}

async fn run_inner(args: HandleArgs) -> Result<i32> {
    // Run the sync stdin read on a blocking-pool thread so the outer
    // `tokio::time::timeout` can fire even if the parent agent never closes
    // its end of the pipe. If we ran this on the async runtime thread,
    // `stdin.read_to_string` would block without yielding and the timer
    // would never be polled.
    let raw = tokio::task::spawn_blocking(|| {
        let _span = info_span!("handle_stdin_read").entered();
        read_stdin_json()
    })
    .await
    .map_err(|e| color_eyre::eyre::eyre!("stdin read task panicked: {e}"))??;

    let provider = {
        let _span = info_span!("handle_provider_resolve").entered();
        resolve_provider(args.provider, &raw)?
    };
    let env = {
        let _span = info_span!("handle_env_detect").entered();
        let cwd = std::env::current_dir().unwrap_or_default();
        detect_environment_fast(&cwd)
    };

    let event_label = args.event.as_deref().unwrap_or("event");
    debug!(%provider, event = %event_label, "Handling event");

    let outcome = {
        let span = info_span!(
            "handle_dispatch_canonical",
            %provider,
            event = %event_label,
        );
        let _enter = span.enter();
        claudine::dispatch::dispatch_canonical(&raw, provider, &env).await?
    };

    if args.json {
        let output = serde_json::json!({
            "provider": provider.as_slug(),
            "event": event_label,
            "response": outcome.response,
            "exit_code": outcome.exit_code,
            "protect_pre": outcome.protect_pre,
            "protect_post": outcome.protect_post,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if let Some(payload) = outcome.response {
        println!("{}", serde_json::to_string(&payload)?);
    }

    Ok(outcome.exit_code.unwrap_or(0))
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

fn resolve_provider(hint: Option<Provider>, raw: &Value) -> Result<Provider> {
    resolve_provider_inner(hint, raw, provider_from_wrapper_env())
}

fn provider_from_wrapper_env() -> Option<Provider> {
    std::env::var("AGENT")
        .ok()
        .or_else(|| std::env::var("Agent").ok())
        .as_deref()
        .and_then(parse_wrapper_env_provider)
}

fn parse_wrapper_env_provider(value: &str) -> Option<Provider> {
    parse_provider(value).ok()
}

fn resolve_provider_inner(
    hint: Option<Provider>,
    raw: &Value,
    env_provider: Option<Provider>,
) -> Result<Provider> {
    if let Some(provider) = hint {
        return Ok(provider);
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
        let provider =
            resolve_provider_inner(Some(Provider::Codex), &raw, Some(Provider::Gemini)).unwrap();
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
