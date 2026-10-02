//! Type-aware ownership for the composition commands: the CLI half.
//!
//! [`claudine::composition::own_arguments`] decides who owns each argument
//! after the composition file. This module supplies what it needs from the
//! file **as authored**: the parameter names its literal `$schema` declares,
//! and the candidate providers (the command-line provider, else the literal
//! frontmatter `agent`, else every provider), each at the native command path
//! its launch would use. Caller setters never change either, so `agent=codex`
//! on the command line does not narrow the candidates.
//!
//! When candidates disagree over a word, an eligible session is asked which
//! agent the arguments are for. The answer decides ownership only; each run,
//! `sequence` step, and proxy target still resolves its own provider, and the
//! resolved-provider check catches one that disagrees.

use std::path::Path;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_tui::prelude::*;
use claudine::composition::{
    OwnershipCandidate, OwnershipError, ProviderTail, ResolvedCompositionSource, SchemaParameters,
    authored_schema_parameters, own_arguments, parse_selection_hints_from_frontmatter, setter_key,
};
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};
use color_eyre::eyre::{Result, eyre};

use super::SharedComposeArgs;
use crate::commands::schema_interactive::resolve_interactive_options;
use crate::commands::wrap::profile::profile_for_provider;
use crate::commands::wrap::provider_tail_report::SwitchContext;
use crate::log;

/// Own `shared.caller_arguments` against the authored `source`: store the
/// provider tail on `shared` and return Claudine's setter and positional
/// tokens, in order, for [`super::parse_composition_positionals`].
///
/// Reads nothing when there is nothing to classify; reads the `$schema` only
/// when a setter-shaped token makes its names matter.
///
/// ## Errors
///
/// An unreadable `$schema`, an [`OwnershipError`], or a cancelled prompt.
pub(crate) fn own_caller_arguments(
    shared: &mut SharedComposeArgs,
    source: &ResolvedCompositionSource,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    launch_fallback: Option<&Path>,
) -> Result<Vec<String>> {
    let arguments = std::mem::take(&mut shared.caller_arguments);
    let tokens: Vec<&str> = arguments
        .arguments()
        .iter()
        .filter_map(|argument| match argument {
            claudine::composition::CallerArgument::Token(token) => Some(token.as_str()),
            claudine::composition::CallerArgument::ClaudineOption => None,
        })
        .collect();
    if tokens.is_empty() {
        shared.provider_tail = ProviderTail::new(Vec::new(), arguments.opaque().map(<[String]>::to_vec));
        return Ok(Vec::new());
    }

    let schema = if tokens.iter().any(|token| setter_key(token).is_some()) {
        authored_schema_parameters(source, launch_fallback, Some(file_resolution_context))?
    } else {
        SchemaParameters::NoSchema
    };
    let mut candidates = candidates(shared, source);
    let owned = loop {
        match own_arguments(&arguments, &schema, &candidates) {
            Ok(owned) => break owned,
            Err(OwnershipError::Ambiguous(ambiguous))
                if candidates.len() > 1 && resolve_interactive_options(shared.silent).allowed() =>
            {
                let chosen = ask_which_agent(&ambiguous)?;
                candidates.retain(|candidate| candidate.provider == chosen);
            }
            Err(err) => return Err(color_eyre::eyre::Report::new(err)),
        }
    };
    shared.provider_tail = owned.tail;
    Ok(owned.claudine)
}

/// The command-line provider, else the literal frontmatter `agent`
/// providers, else every provider; each at its launch's command path.
/// Shell completion reads the same candidates.
pub(crate) fn candidates(shared: &SharedComposeArgs, source: &ResolvedCompositionSource) -> Vec<OwnershipCandidate> {
    let frontmatter = source.markdown.frontmatter();
    let hints = parse_selection_hints_from_frontmatter(frontmatter).ok();
    let providers: Vec<Provider> = match shared.explicit_provider() {
        Some(provider) => vec![provider],
        None if agent_is_templated(frontmatter.as_map().get("agent")) => PROVIDERS_DISPLAY_ORDER.to_vec(),
        None => match hints.as_ref().and_then(|hints| hints.agent.as_ref()) {
            Some(claudine::composition::AgentHint::Single(provider)) => vec![*provider],
            Some(claudine::composition::AgentHint::List(providers)) => providers.clone(),
            None => PROVIDERS_DISPLAY_ORDER.to_vec(),
        },
    };
    let interactive = if shared.no_interactive {
        false
    } else {
        shared.interactive || hints.and_then(|hints| hints.interactive) == Some(true)
    };
    providers
        .into_iter()
        .map(|provider| OwnershipCandidate {
            provider,
            command_path: profile_for_provider(provider)
                .map(|profile| SwitchContext::for_launch(profile, !interactive).command_path)
                .unwrap_or_default(),
        })
        .collect()
}

/// An `agent` written as an expression cannot narrow the candidates, even in
/// a list beside literal providers: it may resolve to any provider.
fn agent_is_templated(agent: Option<&serde_json::Value>) -> bool {
    let templated = |text: &str| text.contains("{{") || text.contains("$(");
    match agent {
        Some(serde_json::Value::String(text)) => templated(text),
        Some(serde_json::Value::Array(items)) => items.iter().any(|item| item.as_str().is_some_and(templated)),
        _ => false,
    }
}

/// Ask which agent the arguments are intended for, offering the providers
/// that read the switch differently.
fn ask_which_agent(ambiguous: &claudine::composition::ownership::AmbiguousSwitch) -> Result<Provider> {
    let intro = format!(
        "Resolving how the arguments after the composition file are read: the candidate \
         agents read the word after `{}` differently. Which agent are these arguments for? \
         (This decides how the arguments are read, not which agent runs.)",
        ambiguous.switch
    );
    log::message(&Prose::new(intro).render(&log::terminal()));
    let options: Vec<ChoiceOption<Provider>> = ambiguous
        .readings
        .iter()
        .map(|reading| ChoiceOption::new(reading.provider.as_slug().to_string(), reading.provider.to_string(), reading.provider))
        .collect();
    let height = crate::commands::schema_interactive::chooser_height(options.len(), false);
    let selected: Option<Provider> = run_standalone(ChooseOne::new(), ChooseOneState::from_options(options), height)?;
    selected.ok_or_else(|| eyre!("no agent chosen for reading the provider arguments"))
}
