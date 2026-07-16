use std::io::{IsTerminal, Read};
use std::time::Duration;

use clap::Args;
use color_eyre::eyre::{Result, bail};
use serde_json::Value;
use tracing::{debug, info_span};

use claudine::events::{AgenticEvent, detect_environment_fast};

use crate::cli_utils::parse_provider;
use crate::provider_values::provider_value_parser;
use claudine::provider::Provider;

/// Kill switch shared with the wrapper's presence reporter (default on).
const RENDEZVOUS_ENABLE_ENV: &str = "CLAUDINE_RENDEZVOUS_REPORT";

/// Env key the wrapper injects into every child; reused as the session
/// identity so hook-driven status agrees with presence/sink reports.
const SESSION_ID_ENV: &str = "CLAUDINE_SESSION_ID";

/// Env key the wrapper injects to advertise interactiveness to the hook
/// subprocess. Trigger 2's idle producer gates on `"1"`: a non-interactive
/// turn-complete is the agent auto-proceeding, not waiting on a human, so
/// it must never be flagged idle.
const INTERACTIVE_ENV: &str = "CLAUDINE_INTERACTIVE";

/// Hard cap on the best-effort status report to the rendezvous daemon.
const STATUS_REPORT_TIMEOUT: Duration = Duration::from_millis(250);

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

    // Normalize the payload to a canonical event so a permission-ask or
    // turn boundary can drive the dashboard status slot. Best-effort: an
    // unrecognized payload simply reports nothing. This is the primary
    // Trigger-1 producer — hooks fire out-of-band on BOTH the PTY and
    // the structured launch, where the stream sink may not run.
    let canonical_event = claudine::hook_adapters::adapter_for(provider)
        .parse_event(&raw)
        .ok()
        .map(|(event, _meta)| event);

    let outcome = {
        let span = info_span!(
            "handle_dispatch_canonical",
            %provider,
            event = %event_label,
        );
        let _enter = span.enter();
        claudine::dispatch::dispatch_canonical(&raw, provider, &env).await?
    };

    if let Some(contribution) = canonical_event.and_then(status_contribution_for) {
        report_session_status(contribution).await;
    }

    // Trigger 2 — interactive idle producer. Only a wrapped INTERACTIVE
    // session's turn boundary means the agent is waiting on a human: a turn
    // completing marks the `IdleHook` slot `idle`, the next user prompt clears
    // it back to `active`. Writes a distinct producer slot from the Trigger-1
    // PermissionHook report above, so the weaker `idle` never clobbers an
    // unresolved `waiting_on_user` (the daemon reducer's precedence decides).
    let interactive_raw = std::env::var(INTERACTIVE_ENV).ok();
    if let Some(status) =
        canonical_event.and_then(|event| idle_report_for(interactive_raw.as_deref(), event))
        && let Some(session_id) = std::env::var(SESSION_ID_ENV)
            .ok()
            .filter(|value| !value.is_empty())
    {
        crate::commands::wrap::session_report::report_status(&session_id, status).await;
    }

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

/// Map a canonical event onto the PermissionHook slot's status
/// contribution, or `None` when the event is status-irrelevant.
///
/// A permission-ask class event raises `WaitingOnUser`; a turn boundary
/// clears the slot back to `Active`. Both write the `PermissionHook`
/// slot (distinct from Trigger 2's `IdleHook` slot), so the clear only
/// ever lowers this producer's own opinion.
fn status_contribution_for(
    event: AgenticEvent,
) -> Option<rendezvous_core::proto::StatusContribution> {
    use rendezvous_core::{SessionStatusBasis as Basis, SessionStatusState as State};

    let (state, basis) = match event {
        AgenticEvent::PermissionRequest | AgenticEvent::HumanInTheLoop => {
            (State::WaitingOnUser, Basis::PermissionAsk)
        }
        AgenticEvent::TurnComplete | AgenticEvent::BeforePrompt => {
            (State::Active, Basis::Lifecycle)
        }
        _ => return None,
    };
    Some(rendezvous_core::proto::StatusContribution {
        state: state as i32,
        producer: rendezvous_core::StatusProducer::PermissionHook as i32,
        basis: basis as i32,
        revision: revision_now(),
    })
}

/// Map a canonical event onto the interactive idle producer's status
/// string, or `None` when this invocation must not contribute an idle
/// signal.
///
/// Gated on `CLAUDINE_INTERACTIVE=1` (`interactive_raw`): a non-interactive
/// turn-complete is the agent auto-proceeding, not waiting on a human, so it
/// reports nothing. For an interactive session a turn completing maps to
/// `idle` and the next user prompt to `active`; every other event is
/// status-irrelevant here. The returned string feeds the `IdleHook` slot via
/// [`report_status`](crate::commands::wrap::session_report::report_status).
fn idle_report_for(interactive_raw: Option<&str>, event: AgenticEvent) -> Option<&'static str> {
    if interactive_raw.map(str::trim) != Some("1") {
        return None;
    }
    match event {
        AgenticEvent::TurnComplete => Some("idle"),
        AgenticEvent::BeforePrompt => Some("active"),
        _ => None,
    }
}

/// Producer-captured unix-ns wall clock; the daemon LWW-guards the slot
/// against a reordered earlier report.
fn revision_now() -> i64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

/// Best-effort report of a hook-driven status transition to the local
/// rendezvous daemon. Reported as `UPDATED`, so a hook firing for a
/// session the wrapper never STARTED (or already ENDED) is a daemon-side
/// no-op — it never resurrects a phantom session. Every failure degrades
/// to a `debug!`; a missing/wedged daemon must never delay a hook.
async fn report_session_status(contribution: rendezvous_core::proto::StatusContribution) {
    if !rendezvous_reporting_enabled() {
        return;
    }
    let Some(session_id) = std::env::var(SESSION_ID_ENV)
        .ok()
        .filter(|value| !value.is_empty())
    else {
        return;
    };

    let send = async {
        let endpoint = rendezvous_core::socket::legacy_local_endpoint(
            rendezvous_core::socket::default_socket_path(),
        );
        let mut client = rendezvous_client::connect(&endpoint).await?;
        client
            .report_session_event(rendezvous_core::ReportSessionEventRequest {
                session_id,
                kind: rendezvous_core::SessionEventKind::Updated as i32,
                details_json: String::new(),
                status: Some(contribution),
            })
            .await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    };
    match tokio::time::timeout(STATUS_REPORT_TIMEOUT, send).await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => debug!(
            target: "claudine::handle",
            %err,
            "hook status report skipped (daemon unreachable)",
        ),
        Err(_) => debug!(
            target: "claudine::handle",
            "hook status report timed out",
        ),
    }
}

fn rendezvous_reporting_enabled() -> bool {
    match std::env::var(RENDEZVOUS_ENABLE_ENV) {
        Ok(raw) => !matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "false" | "0" | "no" | "off"
        ),
        Err(_) => true,
    }
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
    fn permission_ask_events_raise_waiting_on_the_permission_hook_slot() {
        for event in [AgenticEvent::PermissionRequest, AgenticEvent::HumanInTheLoop] {
            let c = super::status_contribution_for(event).expect("permission ask maps");
            assert_eq!(c.state, rendezvous_core::SessionStatusState::WaitingOnUser as i32);
            assert_eq!(c.producer, rendezvous_core::StatusProducer::PermissionHook as i32);
            assert_eq!(c.basis, rendezvous_core::SessionStatusBasis::PermissionAsk as i32);
        }
    }

    #[test]
    fn turn_boundaries_clear_the_permission_hook_slot() {
        for event in [AgenticEvent::TurnComplete, AgenticEvent::BeforePrompt] {
            let c = super::status_contribution_for(event).expect("turn boundary maps");
            assert_eq!(c.state, rendezvous_core::SessionStatusState::Active as i32);
            assert_eq!(c.producer, rendezvous_core::StatusProducer::PermissionHook as i32);
        }
    }

    #[test]
    fn interactive_turn_boundaries_map_to_idle_producer_states() {
        assert_eq!(
            super::idle_report_for(Some("1"), AgenticEvent::TurnComplete),
            Some("idle")
        );
        assert_eq!(
            super::idle_report_for(Some("1"), AgenticEvent::BeforePrompt),
            Some("active")
        );
    }

    #[test]
    fn non_interactive_sessions_never_report_idle() {
        for raw in [Some("0"), Some("false"), Some(""), None] {
            assert_eq!(
                super::idle_report_for(raw, AgenticEvent::TurnComplete),
                None,
                "raw {raw:?} must not flag idle"
            );
        }
    }

    #[test]
    fn idle_producer_ignores_non_turn_boundary_events() {
        assert_eq!(
            super::idle_report_for(Some("1"), AgenticEvent::AfterTool),
            None
        );
        assert_eq!(
            super::idle_report_for(Some("1"), AgenticEvent::PermissionRequest),
            None
        );
    }

    #[test]
    fn status_irrelevant_events_report_nothing() {
        assert!(super::status_contribution_for(AgenticEvent::AfterTool).is_none());
        assert!(super::status_contribution_for(AgenticEvent::Notification).is_none());
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
