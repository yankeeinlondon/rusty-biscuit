//! How a forwarded provider-argument tail is shown: the pre-launch INFO
//! notice, the value-free summary the correlated error report reuses, and
//! the redactor for provider text that may echo a tail value.
//!
//! Composition and the direct provider wrappers both call [`announce`], so
//! the notice reads the same whichever path launched the agent. It never
//! shows an argument value: the implicit prefix is reduced to switch names by
//! [`switch_names_for_display`], and the opaque suffix after an authored `--`
//! is summarized without listing anything.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::prelude::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::{ProviderTail, ProviderTailNotices};
use claudine::provider::Provider;

use crate::commands::wrap::env::{redact_sensitive_args, sensitive_arg_values};
use crate::log;

/// Shown in place of a short token with attached text (`-csecret`, `-yq`).
/// Without researched metadata Claudine cannot tell an attached value from
/// a cluster of switches, so it echoes neither.
const ATTACHED_SHORT_TOKEN: &str = "a short switch with attached text (not shown)";

/// Emit the forwarding notice once per distinct `(provider, tail)` for the
/// command that owns `notices`.
///
/// Renders nothing when the tail is empty or output is quieted. `notices` is
/// claimed before rendering and its lock is never held while rendering.
pub(crate) fn announce(
    provider: Provider,
    tail: &ProviderTail,
    notices: &ProviderTailNotices,
    silent: bool,
    quiet: bool,
    term: &Terminal,
) {
    if silent || quiet {
        return;
    }
    let Some(message) = forwarding_message(provider, tail) else {
        return;
    };
    if !notices.claim(provider, tail) {
        return;
    }
    let status = Status::from_prose(message).state(StatusState::Info);
    log::message(&status.render(term));
}

/// The notice text, or `None` when nothing is forwarded.
fn forwarding_message(provider: Provider, tail: &ProviderTail) -> Option<String> {
    if tail.is_empty() {
        return None;
    }
    let message = match tail_summary(tail) {
        TailSummary::Opaque => {
            format!("Forwarding an opaque argument tail to {provider} (passed after --).")
        }
        TailSummary::OperandsOnly => format!("Forwarding provider arguments to {provider}."),
        TailSummary::Names(names) | TailSummary::NamesThenOpaque(names) => {
            let suffix = if tail.opaque_args().is_some_and(|suffix| !suffix.is_empty()) {
                format!(", {OPAQUE_SUFFIX}.")
            } else {
                String::new()
            };
            format!("Forwarding provider arguments to {provider}: {names}{suffix}")
        }
    };
    Some(message)
}

/// What a non-empty tail can be said to contain without showing a value.
///
/// Names are already sanitized for display: control characters removed and
/// Prose markup escaped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TailSummary {
    /// Switch names from the implicit prefix, comma separated.
    Names(String),
    /// Switch names followed by an opaque suffix after an authored `--`.
    NamesThenOpaque(String),
    /// Only an opaque suffix (fully explicit tail).
    Opaque,
    /// Tokens, but none switch-shaped and none after `--`.
    OperandsOnly,
}

impl TailSummary {
    /// A phrase naming what was forwarded, for a sentence that already says
    /// the arguments were forwarded.
    pub(crate) fn phrase(&self) -> String {
        match self {
            Self::Names(names) => names.clone(),
            Self::NamesThenOpaque(names) => format!("{names}, {OPAQUE_SUFFIX}"),
            Self::Opaque => "an opaque argument tail (passed after --)".to_string(),
            Self::OperandsOnly => "operands with no switch names".to_string(),
        }
    }
}

const OPAQUE_SUFFIX: &str = "followed by an opaque argument tail (passed after --)";

/// Summarize `tail` for display. An empty tail summarizes as operands only;
/// callers decide whether an empty tail is worth mentioning.
pub(crate) fn tail_summary(tail: &ProviderTail) -> TailSummary {
    let names = switch_names_for_display(tail);
    let has_opaque = tail.opaque_args().is_some_and(|suffix| !suffix.is_empty());
    // Control characters would let a token drive the terminal; markup would
    // let it restyle the message.
    let names: String = names.join(", ").chars().filter(|c| !c.is_control()).collect();
    let names = Prose::escape_text(&names);
    match (names.is_empty(), has_opaque) {
        (true, true) => TailSummary::Opaque,
        (true, false) => TailSummary::OperandsOnly,
        (false, false) => TailSummary::Names(names),
        (false, true) => TailSummary::NamesThenOpaque(names),
    }
}

/// A redactor for text that may echo `tail`: it masks recognized secrets and
/// every value [`redact_sensitive_args`] would mask in the tail, wherever the
/// value reappears, even without its flag.
pub(crate) fn tail_redactor(tail: &ProviderTail) -> claudine::secrets::Redactor {
    let args = tail.launch_args();
    claudine::secrets::Redactor::for_message(&args.join(" "))
        .with_known_values(sensitive_arg_values(args))
}

/// Whether `switch`, a name a provider's diagnostic quoted, is one of the
/// tail's tokens: the token itself, its `--name=value` form, or a short
/// switch with attached text.
pub(crate) fn tail_names_switch(tail: &ProviderTail, switch: &str) -> bool {
    let switch = switch.split_once('=').map_or(switch, |(name, _)| name);
    tail.launch_args().iter().any(|token| {
        token == switch
            || token
                .strip_prefix(switch)
                .is_some_and(|rest| rest.starts_with('=') || (is_short_switch(switch) && !rest.is_empty()))
    })
}

fn is_short_switch(switch: &str) -> bool {
    switch.len() == 2 && switch.starts_with('-') && switch != "--"
}

/// The value-free switch names of the implicit prefix, in order.
///
/// Runs the tokens through [`redact_sensitive_args`], keeps switch-shaped
/// tokens, and strips an `=value` suffix. A short token with attached text is
/// described by [`ATTACHED_SHORT_TOKEN`] instead of being split or echoed.
/// Tokens after an authored `--` are never listed.
pub(crate) fn switch_names_for_display(tail: &ProviderTail) -> Vec<String> {
    redact_sensitive_args(tail.implicit_args())
        .into_iter()
        .filter(|token| token.starts_with('-') && token != "-" && token != "--")
        .map(|token| {
            if token.starts_with("--") {
                return match token.split_once('=') {
                    Some((name, _)) => name.to_string(),
                    None => token,
                };
            }
            let name = token.split_once('=').map_or(token.as_str(), |(name, _)| name);
            if name.chars().count() == 2 {
                name.to_string()
            } else {
                ATTACHED_SHORT_TOKEN.to_string()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
