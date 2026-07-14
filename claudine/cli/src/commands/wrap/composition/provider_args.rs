//! Pre-launch INFO status for the forwarded provider-argument tail.
//!
//! Phase 1 renders a **generic** notice: it reports that a tail is being
//! forwarded to the resolved provider without claiming per-switch
//! recognition (the compiled switch catalog arrives in Phase 2). The notice
//! is suppressed by `--quiet` and `--silent`, deduplicated so a sequence or
//! loop does not repeat it every attempt, and never exposes an argument
//! *value* — only switch names, with any `=value` suffix stripped, run
//! through the shared [`redact_sensitive_args`] policy.

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::prelude::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use claudine::provider::Provider;

use crate::commands::wrap::env::redact_sensitive_args;
use crate::log;

/// Distinct `(provider, tail)` pairs already announced this process. Bounds
/// the notice to "once per distinct pair per top-level command" — a sequence
/// forwarding the same tail to the same provider on every step announces once.
static ANNOUNCED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// Emit the generic forwarding notice for `args` once per `(provider, args)`.
///
/// No-ops when the tail is empty or output is quieted.
pub(crate) fn announce_forwarded_tail(
    provider: Provider,
    args: &[String],
    explicit: bool,
    silent: bool,
    quiet: bool,
    term: &Terminal,
) {
    if args.is_empty() || silent || quiet {
        return;
    }

    let key = format!("{}\u{0}{}", provider.as_slug(), args.join("\u{0}"));
    {
        let mut announced = ANNOUNCED.lock().unwrap();
        if !announced.insert(key) {
            return;
        }
    }

    let message = forwarding_message(provider, args, explicit);
    let status = Status::from_prose(message).state(StatusState::Info);
    log::message(&status.render(term));
}

/// Build the generic notice text (switch names only, values redacted/stripped).
fn forwarding_message(provider: Provider, args: &[String], explicit: bool) -> String {
    if explicit {
        return format!(
            "Forwarding an opaque argument tail to {provider} (passed after `--`)."
        );
    }
    let switches = switch_names_for_display(args);
    if switches.is_empty() {
        format!("Forwarding provider arguments to {provider}.")
    } else {
        format!(
            "Forwarding provider arguments to {provider}: {} \
             (not recognized by Claudine; passed through to the agent).",
            switches.join(" ")
        )
    }
}

/// Extract the switch-shaped tokens for display: redact sensitive values, keep
/// only flag tokens, and strip any `=value` suffix so no value is revealed.
fn switch_names_for_display(args: &[String]) -> Vec<String> {
    redact_sensitive_args(args)
        .into_iter()
        .filter(|token| token.starts_with('-') && token != "-" && token != "--")
        .map(|token| match token.split_once('=') {
            Some((flag, _)) => flag.to_string(),
            None => token,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_hides_values_and_keeps_switch_names() {
        let args = vec![
            "-c".to_string(),
            "model_reasoning_effort=low".to_string(),
            "--flag=secretvalue".to_string(),
        ];
        // The bare operand (`model_reasoning_effort=low`) is not switch-shaped
        // and is dropped; `--flag=secretvalue` keeps only `--flag`.
        assert_eq!(switch_names_for_display(&args), vec!["-c", "--flag"]);
    }

    #[test]
    fn explicit_tail_message_names_no_switches() {
        let msg = forwarding_message(Provider::Codex, &["-c".to_string(), "x".to_string()], true);
        assert!(msg.contains("opaque"));
        assert!(!msg.contains("-c"));
    }

    #[test]
    fn implicit_tail_message_lists_switch_names_only() {
        let msg = forwarding_message(
            Provider::Codex,
            &["-c".to_string(), "model_reasoning_effort=low".to_string()],
            false,
        );
        assert!(msg.contains("-c"));
        assert!(!msg.contains("model_reasoning_effort"));
    }
}
