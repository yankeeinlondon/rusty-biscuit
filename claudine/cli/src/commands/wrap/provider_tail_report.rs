//! How a forwarded provider-argument tail is shown: the pre-launch INFO
//! notice, the value-free summary the correlated error report reuses, and
//! the redactor for provider text that may echo a tail value.
//!
//! Composition and the direct provider wrappers both call [`announce`], so
//! the notice reads the same whichever path launched the agent. It never
//! shows an argument value: the implicit prefix is reduced to switch names by
//! [`switch_names_for_display`], and the opaque suffix after an authored `--`
//! is summarized without listing anything.
//!
//! Below the notice, each implicit switch is explained from the compiled
//! switch catalog ([`claudine::provider::lookup_switch`]) at the command path
//! the launch uses: what a researched switch is, or that the catalog
//! establishes no type for it there (no record, or a record typed unknown)
//! and Claudine forwards it anyway. The
//! explanation never claims the provider will reject a switch.

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::prelude::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::{ProviderTail, ProviderTailNotices};
use claudine::provider::{Provider, SwitchLookup, SwitchValue, lookup_switch, match_switch_token};

use crate::commands::wrap::env::{redact_sensitive_args, sensitive_arg_values};
use crate::commands::wrap::profile::WrapperProfile;
use crate::log;

/// Shown in place of a short token with attached text (`-csecret`, `-yq`).
/// Without researched metadata Claudine cannot tell an attached value from
/// a cluster of switches, so it echoes neither.
const ATTACHED_SHORT_TOKEN: &str = "a short switch with attached text (not shown)";

/// Where forwarded switches are read: the provider and the native command
/// path its launch uses (`["exec"]` for a non-interactive Codex run, empty
/// for the root entrypoint).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SwitchContext {
    pub(crate) provider: Provider,
    pub(crate) command_path: Vec<String>,
}

impl SwitchContext {
    /// The context of a launch through `profile`. The path is what the
    /// profile's own [`WrapperProfile::apply_entrypoint`] puts in front of an
    /// empty argv, so it is the entrypoint the launch really uses.
    pub(crate) fn for_launch(profile: &dyn WrapperProfile, non_interactive: bool) -> Self {
        let mut args = Vec::new();
        profile.apply_entrypoint(&mut args, non_interactive);
        let command_path = args
            .iter()
            .take_while(|arg| !arg.starts_with('-'))
            .flat_map(|arg| arg.split_whitespace())
            .map(str::to_string)
            .collect();
        Self {
            provider: profile.provider(),
            command_path,
        }
    }

    /// The context of a resume launch: the words of the profile's resume
    /// entrypoint (`exec resume` for Codex), without the session id or any
    /// flag. The catalog answers a resume entrypoint separately.
    pub(crate) fn for_resume(provider: Provider, entrypoint_args: &[String], session_id: &str) -> Self {
        let command_path = entrypoint_args
            .iter()
            .take_while(|arg| !arg.starts_with('-'))
            .filter(|arg| *arg != session_id)
            .cloned()
            .collect();
        Self {
            provider,
            command_path,
        }
    }

    fn path(&self) -> Vec<&str> {
        self.command_path.iter().map(String::as_str).collect()
    }

    /// Check `tail`'s implicit switch assignments against this provider and
    /// command path (the resolved-provider check). Explicit tails, and a
    /// direct wrapper's tail, carry no assignments and always pass.
    ///
    /// ## Errors
    ///
    /// The [`claudine::composition::TailMismatch`] the researched types
    /// establish; it fails before the spawn and never reaches the native-exit
    /// report.
    pub(crate) fn check(&self, tail: &ProviderTail) -> Result<(), claudine::composition::TailMismatch> {
        claudine::composition::check_launch_tail(tail, self.provider, &self.path())
    }

    /// The command path as a reader names it.
    fn where_phrase(&self) -> String {
        if self.command_path.is_empty() {
            "its root command".to_string()
        } else {
            format!("its `{}` command", self.command_path.join(" "))
        }
    }
}

/// Emit the forwarding notice once per distinct `(provider, tail)` for the
/// command that owns `notices`, with one explanation per implicit switch.
///
/// Renders nothing when the tail is empty or output is quieted. `notices` is
/// claimed before rendering and its lock is never held while rendering.
pub(crate) fn announce(
    context: &SwitchContext,
    tail: &ProviderTail,
    notices: &ProviderTailNotices,
    silent: bool,
    quiet: bool,
    term: &Terminal,
) {
    if silent || quiet {
        return;
    }
    let Some(message) = forwarding_message(context, tail) else {
        return;
    };
    if !notices.claim(context.provider, tail) {
        return;
    }
    let status = Status::from_prose(message).state(StatusState::Info);
    log::message(&status.render(term));
    let explanations = switch_explanations(context, tail);
    if !explanations.is_empty() {
        let mut list = UnorderedList::empty();
        for explanation in explanations {
            list.add(Prose::new(explanation));
        }
        log::message(&list.render(term));
    }
}

/// The notice text, or `None` when nothing is forwarded.
fn forwarding_message(context: &SwitchContext, tail: &ProviderTail) -> Option<String> {
    if tail.is_empty() {
        return None;
    }
    let provider = context.provider;
    let message = match summarize(tail, Some(context)) {
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
    summarize(tail, None)
}

fn summarize(tail: &ProviderTail, context: Option<&SwitchContext>) -> TailSummary {
    let names = switch_names_for_display(tail, context);
    let has_opaque = tail.opaque_args().is_some_and(|suffix| !suffix.is_empty());
    let names = display_safe(&names.join(", "));
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
/// tokens, and strips an `=value` suffix. A short token with attached text
/// is split only where `context`'s catalog researched that form for the
/// switch (`-csecret` → `-c`); otherwise it is described by
/// [`ATTACHED_SHORT_TOKEN`] instead of being split or echoed. Tokens after an
/// authored `--` are never listed.
pub(crate) fn switch_names_for_display(
    tail: &ProviderTail,
    context: Option<&SwitchContext>,
) -> Vec<String> {
    switch_tokens(tail)
        .into_iter()
        .map(|token| display_name(&token, context))
        .collect()
}

/// The redacted, switch-shaped tokens of the implicit prefix.
fn switch_tokens(tail: &ProviderTail) -> Vec<String> {
    redact_sensitive_args(tail.implicit_args())
        .into_iter()
        .filter(|token| token.starts_with('-') && token != "-" && token != "--")
        .collect()
}

fn display_name(token: &str, context: Option<&SwitchContext>) -> String {
    if let Some(context) = context
        && let Some(matched) = match_switch_token(context.provider, &context.path(), token)
    {
        return matched.spelling.to_string();
    }
    if token.starts_with("--") {
        return token.split_once('=').map_or(token, |(name, _)| name).to_string();
    }
    let name = token.split_once('=').map_or(token, |(name, _)| name);
    if name.chars().count() == 2 {
        name.to_string()
    } else {
        ATTACHED_SHORT_TOKEN.to_string()
    }
}

/// One sentence per distinct implicit switch, in order, saying what the
/// compiled catalog establishes about it at `context`'s command path. A
/// record whose type is unknown gets the same sentence as no record.
fn switch_explanations(context: &SwitchContext, tail: &ProviderTail) -> Vec<String> {
    let provider = context.provider;
    let path = context.path();
    let mut seen = Vec::new();
    let mut explanations = Vec::new();
    for token in switch_tokens(tail) {
        let name = display_name(&token, Some(context));
        if seen.contains(&name) {
            continue;
        }
        let shown = display_safe(&name);
        let explanation = match lookup_switch(provider, &path, &name) {
            // A record typed `Unknown` establishes nothing, so it reads like an
            // absent one, as it does for ownership (rule 5).
            SwitchLookup::Known(switch) if switch.value != SwitchValue::Unknown => {
                let what = display_safe(&first_sentence(switch.description));
                if name == switch.flag {
                    format!("{shown} is one of {provider}'s switches ({what}); forwarding to {provider}.")
                } else {
                    let flag = display_safe(switch.flag);
                    format!("{shown} is {provider}'s {flag} switch ({what}); forwarding to {provider}.")
                }
            }
            SwitchLookup::Known(_) | SwitchLookup::NotInCatalog | SwitchLookup::CatalogGap { .. } => format!(
                "{shown}: Claudine's compiled {provider} switch catalog has no established type \
                 for it at {}; Claudine forwards it anyway.",
                context.where_phrase()
            ),
        };
        seen.push(name);
        explanations.push(explanation);
    }
    explanations
}

/// The first clause of a researched description (up to its first `. ` or
/// `; `), without its full stop and with a leading capital lowered unless it
/// starts an acronym, so it reads inside parentheses.
fn first_sentence(description: &str) -> String {
    let end = [". ", "; "]
        .iter()
        .filter_map(|stop| description.find(stop))
        .min()
        .unwrap_or(description.len());
    let sentence = description[..end].trim();
    let sentence = sentence.strip_suffix('.').unwrap_or(sentence);
    let mut chars = sentence.chars();
    match (chars.next(), chars.next()) {
        (Some(first), Some(second)) if first.is_uppercase() && !second.is_uppercase() => {
            first.to_lowercase().chain(sentence.chars().skip(1)).collect()
        }
        _ => sentence.to_string(),
    }
}

/// Text from a token or from research, safe to put in a Prose message:
/// control characters would let it drive the terminal, and markup would let
/// it restyle the message.
fn display_safe(text: &str) -> String {
    let text: String = text.chars().filter(|c| !c.is_control()).collect();
    Prose::escape_text(&text)
}

#[cfg(test)]
mod tests;
