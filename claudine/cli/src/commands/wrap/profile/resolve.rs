//! Model and prompt resolution helpers for the wrap pipeline.
//!
//! Two related concerns live here: resolving the effective OpenCode model from
//! the CLI / env / config precedence chain, and extracting a typed
//! [`PromptSource`](super::PromptSource) from raw provider passthrough argv.
//! Both are pure resolution logic with no provider-trait dispatch, so they sit
//! beside the trait module rather than inside it.

use std::path::PathBuf;

use claudine::provider::{COMMON_VALUE_TAKING_FLAGS, PromptArgConventions};
use color_eyre::eyre::{Result, bail, eyre};

use super::{PromptSource, WrapperProfile, has_flag, non_empty_env_var};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpenCodeModelSource {
    CliSwitch(String),
    OpenCodeModelEnv(String),
    ConfigDefault(String),
}

impl OpenCodeModelSource {
    pub(crate) fn model(&self) -> &str {
        match self {
            Self::CliSwitch(m) | Self::OpenCodeModelEnv(m) | Self::ConfigDefault(m) => m,
        }
    }

    pub(crate) fn status_markup(&self) -> String {
        let model = self.model();
        match self {
            Self::CliSwitch(_) => format!(
                "<dim><i>using the </i><yellow>{model}</yellow><i> based on the CLI switch override used by caller</i></dim>"
            ),
            Self::OpenCodeModelEnv(_) => format!(
                "<dim><i>using the </i><yellow>{model}</yellow><i> based on the OPENCODE_MODEL environment variable</i></dim>"
            ),
            Self::ConfigDefault(_) => format!(
                "<dim><i>using the </i><b>{model}</b><i> because this is the default configured in <blue>~/.config/opencode/config.json</blue></i></dim>"
            ),
        }
    }

    pub(crate) fn location_string(&self) -> &'static str {
        match self {
            Self::CliSwitch(_) => "the --model CLI switch",
            Self::OpenCodeModelEnv(_) => "the OPENCODE_MODEL environment variable",
            Self::ConfigDefault(_) => "the config file ~/.config/opencode/config.json",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoModelProvided;

impl std::fmt::Display for NoModelProvided {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no model provided")
    }
}

impl std::error::Error for NoModelProvided {}

#[derive(Debug, Default, Clone)]
pub(crate) struct OpenCodeEnvSnapshot {
    pub opencode_model_env: Option<String>,
    pub opencode_config_model: Option<String>,
}

impl OpenCodeEnvSnapshot {
    pub(crate) fn from_system() -> Self {
        Self {
            opencode_model_env: non_empty_env_var("OPENCODE_MODEL"),
            opencode_config_model: read_opencode_config_model(),
        }
    }
}

pub(crate) fn resolve_opencode_model(
    cli_model: Option<&str>,
    snapshot: &OpenCodeEnvSnapshot,
) -> std::result::Result<OpenCodeModelSource, NoModelProvided> {
    if let Some(m) = cli_model {
        return Ok(OpenCodeModelSource::CliSwitch(m.to_string()));
    }

    if let Some(m) = &snapshot.opencode_model_env {
        return Ok(OpenCodeModelSource::OpenCodeModelEnv(m.clone()));
    }

    if let Some(m) = &snapshot.opencode_config_model {
        return Ok(OpenCodeModelSource::ConfigDefault(m.clone()));
    }

    Err(NoModelProvided)
}

fn opencode_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".config/opencode/config.json"))
}

fn read_opencode_config_model() -> Option<String> {
    let path = opencode_config_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    let model = value.get("model")?.as_str()?;
    if model.is_empty() {
        return None;
    }
    Some(model.to_string())
}

pub(crate) fn apply_opencode_model_resolution(
    child_args: &mut Vec<String>,
    env_setter: &mut dyn FnMut(String, String),
    has_model_env: bool,
    cli_model: Option<&str>,
    non_interactive: bool,
    snapshot: &OpenCodeEnvSnapshot,
) -> Result<Option<OpenCodeModelSource>> {
    if !non_interactive {
        return Ok(None);
    }

    let opencode_model_source = match resolve_opencode_model(cli_model, snapshot) {
        Ok(source) => {
            let model = source.model().to_string();
            match &source {
                OpenCodeModelSource::CliSwitch(_) | OpenCodeModelSource::OpenCodeModelEnv(_) => {
                    if !has_flag(child_args, "--model") && !has_flag(child_args, "-m") {
                        child_args.push("--model".to_string());
                        child_args.push(model.clone());
                    }
                    env_setter("MODEL".to_string(), model);
                }
                OpenCodeModelSource::ConfigDefault(_) => {
                    env_setter("MODEL".to_string(), model);
                }
            }
            Some(source)
        }
        Err(NoModelProvided) => None,
    };

    let has_model_arg = has_flag(child_args, "--model") || has_flag(child_args, "-m");
    if !has_model_arg && !has_model_env && opencode_model_source.is_none() {
        return Err(eyre!(
            "No model specified! OpenCode by default does not specify a model but you can\n\
             change this behavior by adding a model property to ~/.config/opencode/config.json.\n\
             You can override/set the default model with any of the following methods:\n\n\
             \x20\x20• set OPENCODE_MODEL to a valid model name\n\
             \x20\x20• use the CLI switch --model <model>\n\n\
             Running `opencode models` will give you a list of all valid models."
        ));
    }

    Ok(opencode_model_source)
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
