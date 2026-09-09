//! Model-source and prompt resolution helpers for the wrap pipeline.
//!
//! Two related concerns live here: naming where a launched model came from
//! ([`ModelSource`], provider-neutral — the deciding variable or file is
//! carried, never assumed) and extracting a typed
//! [`PromptSource`](super::PromptSource) from raw provider passthrough argv.
//! Both are pure resolution logic with no provider-trait dispatch, so they sit
//! beside the trait module rather than inside it.

use std::path::PathBuf;

use claudine::provider::{COMMON_VALUE_TAKING_FLAGS, PromptArgConventions};
use color_eyre::eyre::{Result, bail, eyre};

use super::{PromptSource, WrapperProfile, has_flag};

/// A provider's own configured default model, as read from its configuration
/// by [`WrapperProfile::configured_default_model`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfiguredModel {
    pub(crate) model: String,
    /// The file the `model` entry was read from, for the preamble and for
    /// error attribution.
    pub(crate) path: PathBuf,
}

/// Where the launched model came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModelSource {
    /// An explicit model: the composition's resolved model or the direct
    /// wrapper's `--model`.
    CliSwitch(String),
    /// One of the provider's catalog `model_env_vars`.
    ProviderEnv { var: &'static str, model: String },
    /// The provider's own configured default; nothing is pushed on argv
    /// because the provider reads it itself.
    ConfigDefault(ConfiguredModel),
}

impl ModelSource {
    pub(crate) fn model(&self) -> &str {
        match self {
            Self::CliSwitch(m) | Self::ProviderEnv { model: m, .. } => m,
            Self::ConfigDefault(configured) => &configured.model,
        }
    }

    pub(crate) fn status_markup(&self) -> String {
        let model = self.model();
        match self {
            Self::CliSwitch(_) => format!(
                "<dim><i>using the </i><yellow>{model}</yellow><i> based on the CLI switch override used by caller</i></dim>"
            ),
            Self::ProviderEnv { var, .. } => format!(
                "<dim><i>using the </i><yellow>{model}</yellow><i> based on the {var} environment variable</i></dim>"
            ),
            Self::ConfigDefault(configured) => format!(
                "<dim><i>using the </i><b>{model}</b><i> because this is the default configured in <blue>{}</blue></i></dim>",
                configured.path.display()
            ),
        }
    }

    pub(crate) fn location_string(&self) -> String {
        match self {
            Self::CliSwitch(_) => "the --model CLI switch".to_string(),
            Self::ProviderEnv { var, .. } => format!("the {var} environment variable"),
            Self::ConfigDefault(configured) => {
                format!("the config file {}", configured.path.display())
            }
        }
    }
}

/// Find a model for a provider that must have one and was given none:
/// the catalog's `model_env_vars` in order, then the provider's configured
/// default. `None` when neither names a model.
///
/// Pure: the environment and the configuration read arrive as closures so
/// the precedence is testable without touching the process.
pub(crate) fn resolve_model_source(
    env_vars: &[&'static str],
    env_lookup: impl Fn(&str) -> Option<String>,
    configured_default: impl FnOnce() -> Option<ConfiguredModel>,
) -> Option<ModelSource> {
    env_vars
        .iter()
        .find_map(|var| {
            env_lookup(var)
                .filter(|value| !value.is_empty())
                .map(|model| ModelSource::ProviderEnv { var, model })
        })
        .or_else(|| configured_default().map(ModelSource::ConfigDefault))
}

/// The pre-launch error for a provider that requires a model in
/// non-interactive mode when no source supplied one.
pub(crate) fn no_model_error(provider: claudine::provider::Provider) -> color_eyre::eyre::Error {
    let env_vars = claudine::provider::provider_info(provider).model_env_vars;
    let env_hint = if env_vars.is_empty() {
        String::new()
    } else {
        format!(
            "\n  • set {} to a valid model name",
            env_vars.join(" or ")
        )
    };
    eyre!(
        "No model specified! {provider} requires a model in non-interactive mode.\n\
         You can set one with any of the following methods:\n\
         \n  • use the CLI switch --model <model>{env_hint}\
         \n  • add a model entry to {provider}'s own configuration"
    )
}

/// Extract a prompt from raw passthrough args, returning the cleaned
/// args and the typed `PromptSource`.
///
/// This is the *single* place in the codebase that knows how to locate
/// a prompt inside provider passthrough arguments. It replaces the
/// previous per-provider extractors (`extract_user_prompt`,
/// `find_prompt_location`, `strip_prompt_from_args`) and the inline
/// positional-to-flag shuffling that used to live in
/// `apply_non_interactive` for Gemini and Qwen.
///
/// Precedence (highest wins):
/// 1. A prompt-carrying flag from `prompt_arg_conventions().prompt_flags`
///    (e.g. `--prompt VALUE`, `-p=VALUE`)
/// 2. A bare positional arg (after skipping the entrypoint subcommand
///    and any value-taking flags)
/// 3. `has_piped_stdin == true` → `PromptSource::InheritStdin`
/// 4. Otherwise → `PromptSource::None`
///
/// Whenever a flag or positional is returned as the prompt, it is
/// removed from the returned `Vec<String>` so downstream trait methods
/// see clean args with zero prompt characters.
///
/// ## Errors
///
/// Returns an error if a prompt-carrying flag appears in `passthrough`
/// without a following value (e.g. a bare trailing `--prompt`). Silent
/// fall-through in that case would drop the user's intent — piped
/// stdin, if present, would take its place. Surface the problem at
/// extraction time instead.
pub(crate) fn extract_prompt_source_from_passthrough(
    profile: &dyn WrapperProfile,
    passthrough: &[String],
    has_piped_stdin: bool,
) -> Result<(Vec<String>, PromptSource)> {
    let conv = profile.prompt_arg_conventions();
    let mut args: Vec<String> = passthrough.to_vec();

    // 1. Look for a prompt-carrying flag.
    if let Some((prompt, indices)) = find_prompt_flag(&args, conv.prompt_flags)? {
        // Remove the matched indices in reverse order so earlier
        // indices stay valid while splicing.
        for idx in indices.iter().rev() {
            args.remove(*idx);
        }
        return Ok((args, PromptSource::Inline(prompt)));
    }

    // 2. Look for a positional prompt, skipping the entrypoint (if any)
    //    and any value-taking flags.
    if let Some(idx) = find_positional_prompt_index(&args, &conv) {
        let prompt = args.remove(idx);
        if idx > 0 && args[idx - 1] == "--" {
            args.remove(idx - 1);
        }
        return Ok((args, PromptSource::Inline(prompt)));
    }

    // 3. Piped stdin.
    if has_piped_stdin {
        return Ok((args, PromptSource::InheritStdin));
    }

    // 4. No prompt.
    Ok((args, PromptSource::None))
}

/// Find a prompt delivered via one of `prompt_flags`. Returns the prompt
/// text and the argv indices to remove.
///
/// Supports four shapes:
/// - `--prompt VALUE`      → two indices
/// - `--prompt=VALUE`      → one index
/// - `-p VALUE`            → two indices
/// - `-p=VALUE`            → one index
fn find_prompt_flag(
    args: &[String],
    prompt_flags: &[&str],
) -> Result<Option<(String, Vec<usize>)>> {
    for (idx, arg) in args.iter().enumerate() {
        for flag in prompt_flags {
            if arg == flag {
                let value = args.get(idx + 1).cloned().ok_or_else(|| {
                    eyre!("prompt flag `{flag}` requires a value but none was provided")
                })?;
                return Ok(Some((value, vec![idx, idx + 1])));
            }
            let inline_prefix = format!("{flag}=");
            if let Some(value) = arg.strip_prefix(&inline_prefix) {
                return Ok(Some((value.to_string(), vec![idx])));
            }
        }
    }
    Ok(None)
}

/// Find the index of the first positional prompt candidate in `args`,
/// honoring the entrypoint skip and the set of value-taking flags.
fn find_positional_prompt_index(args: &[String], conv: &PromptArgConventions) -> Option<usize> {
    let mut skip_next = false;
    for (idx, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        // Skip the entrypoint subcommand if it matches at index 0.
        if idx == 0
            && let Some(entry) = conv.entrypoint
            && arg == entry
        {
            continue;
        }

        if arg == "--" {
            return (idx + 1 < args.len()).then_some(idx + 1);
        }

        // Skip value-taking flags so their values are not mistaken for
        // positional prompts. Handle both `--flag value` and
        // `--flag=value` shapes. The shared union list is deliberate
        // (OQ7a ruling): over-skipping an unknown flag's value is harmless.
        if let Some(eq_idx) = arg.find('=')
            && COMMON_VALUE_TAKING_FLAGS
                .iter()
                .any(|flag| arg[..eq_idx] == **flag)
        {
            continue;
        }
        if COMMON_VALUE_TAKING_FLAGS.iter().any(|flag| arg == *flag) {
            skip_next = true;
            continue;
        }

        if !arg.starts_with('-') {
            return Some(idx);
        }
    }
    None
}

/// Generic "is the prompt requirement satisfied?" check for the wrap
/// pipeline. Called from every call site after all `apply_*` methods
/// have run and `prompt_delivery` has placed any inline prompt.
///
/// Returns `Ok(())` when any of the following holds:
/// - `non_interactive == false` (interactive sessions never require a
///   preloaded prompt — the user will type one)
/// - `source.has_prompt_or_stdin()` is true (inline prompt or piped
///   stdin reaches the child)
///
/// Otherwise bails with a provider-agnostic error message that
/// interpolates `provider_name` so the user knows which wrap failed.
pub(crate) fn require_prompt_present(
    provider_name: &str,
    non_interactive: bool,
    source: &PromptSource,
) -> Result<()> {
    if !non_interactive {
        return Ok(());
    }
    if source.has_prompt_or_stdin() {
        return Ok(());
    }
    bail!(
        "--non-interactive for {provider_name} requires a prompt \
         (positional, via a prompt flag, or piped on stdin)"
    );
}
