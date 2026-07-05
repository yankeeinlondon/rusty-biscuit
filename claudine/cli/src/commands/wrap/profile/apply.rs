//! Catalog-driven default bodies for the heavy [`WrapperProfile`](super::WrapperProfile)
//! methods.
//!
//! The trait keeps the method declarations (and their docs) in
//! [`super`](super); the large default implementations live here as free
//! functions taking the resolved [`Provider`] so the trait module stays a
//! readable contract surface. Per-provider overrides remain in the provider
//! submodules and never route through here.

use std::path::Path;

use claudine::provider::{
    EntrypointMode, OutputFormatSelector, Provider, YoloSupport, provider_info,
};
use claudine::system_prompt::{PreparedSystemPrompt, SystemPromptMode};
use color_eyre::eyre::{Result, bail};

use super::super::system_prompt::SystemPromptApplication;
use super::{OutputFormat, YoloOutcome, has_any_flag, has_flag};

/// Catalog-driven default for [`WrapperProfile::apply_yolo`](super::WrapperProfile::apply_yolo).
pub(super) fn apply_yolo(
    provider: Provider,
    args: &mut Vec<String>,
    env_overrides: &mut Vec<(String, String)>,
) -> Result<YoloOutcome> {
    match provider_info(provider).yolo {
        YoloSupport::DirectFlag { native_flag } => {
            if !has_flag(args, native_flag) {
                args.push(native_flag.to_string());
            }
            Ok(YoloOutcome::applied())
        }
        YoloSupport::DirectFlagWithAlias {
            native_flag,
            aliases,
        } => {
            if !has_any_flag(args, native_flag, aliases) {
                args.push(native_flag.to_string());
            }
            Ok(YoloOutcome::applied())
        }
        YoloSupport::EnvVar { env_var, value } => {
            if !env_overrides
                .iter()
                .any(|(k, v)| k == env_var && v == value)
            {
                env_overrides.push((env_var.to_string(), value.to_string()));
            }
            Ok(YoloOutcome::applied())
        }
        YoloSupport::NonInteractiveOnly { .. } => {
            // Mode-unaware path: we don't know if we're interactive,
            // so the conservative default is to NOT apply and emit
            // the warning. The mode-aware variant below will do the
            // right thing when callers thread `interactive` through.
            Ok(YoloOutcome::not_applied(format!(
                "{provider} YOLO mode is non-interactive only"
            )))
        }
        YoloSupport::None => Ok(YoloOutcome::not_applied(format!(
            "{provider} does not support YOLO mode"
        ))),
    }
}

/// Catalog-driven default for
/// [`WrapperProfile::reject_direct_yolo`](super::WrapperProfile::reject_direct_yolo).
pub(super) fn reject_direct_yolo(provider: Provider, args: &[String]) -> Result<()> {
    match provider_info(provider).yolo {
        YoloSupport::DirectFlag { native_flag } if has_flag(args, native_flag) => {
            bail!(
                "do not pass <blue>{native_flag}</blue> directly to claudine {provider}; \
                 use Claudine's <blue>--yolo</blue> or <blue>-y</blue> switches instead. \
                 Claudine uses this CLI convention for all agents it provides a wrapper to."
            );
        }
        YoloSupport::DirectFlagWithAlias {
            native_flag,
            aliases,
        } if has_any_flag(args, native_flag, aliases) => {
            bail!(
                "do not pass <blue>{native_flag}</blue> directly to claudine {provider}; \
                 use Claudine's <blue>--yolo</blue> or <blue>-y</blue> switches instead. \
                 Claudine uses this CLI convention for all agents it provides a wrapper to."
            );
        }
        _ => {}
    }
    Ok(())
}

/// Catalog-driven default for
/// [`WrapperProfile::apply_entrypoint`](super::WrapperProfile::apply_entrypoint).
pub(super) fn apply_entrypoint(provider: Provider, args: &mut Vec<String>, non_interactive: bool) {
    let info = provider_info(provider);
    let target_mode = if non_interactive {
        EntrypointMode::NonInteractive
    } else {
        EntrypointMode::Interactive
    };
    if let Some(ep) = info
        .entrypoints
        .iter()
        .find(|ep| matches!(ep.mode, EntrypointMode::Both) || ep.mode == target_mode)
    {
        if let Some(sub) = ep.subcommand
            && args.first().is_none_or(|first| first != sub)
        {
            args.insert(0, sub.to_string());
        }
        for flag in ep.required_flags {
            if !has_flag(args, flag) {
                args.push(flag.to_string());
            }
        }
    }
}

/// Catalog-driven default for
/// [`WrapperProfile::apply_output_format`](super::WrapperProfile::apply_output_format).
pub(super) fn apply_output_format(
    provider: Provider,
    args: &mut Vec<String>,
    format: OutputFormat,
) -> Option<String> {
    let info = provider_info(provider);
    let provider_format = match format {
        OutputFormat::Json => claudine::provider::OutputFormat::Json,
        OutputFormat::Text => claudine::provider::OutputFormat::Text,
        OutputFormat::Stream => claudine::provider::OutputFormat::Stream,
    };
    let Some(support) = info
        .output_formats
        .iter()
        .find(|s| s.format == provider_format)
    else {
        return Some(format!(
            "{provider} does not support --output {format}; this flag was skipped"
        ));
    };

    match support.selector {
        OutputFormatSelector::Default | OutputFormatSelector::Positional { .. } => None,
        OutputFormatSelector::Flag { flag } | OutputFormatSelector::TransportFlag { flag } => {
            if !has_flag(args, flag) {
                args.push(flag.to_string());
            }
            None
        }
        OutputFormatSelector::FlagValue { flag } => {
            if !has_flag(args, flag)
                && !args.iter().any(|arg| arg.starts_with(&format!("{flag}=")))
            {
                args.push(flag.to_string());
                args.push(support.native_name.to_string());
            }
            None
        }
    }
}

/// Catalog-driven default for
/// [`WrapperProfile::apply_structured_stream`](super::WrapperProfile::apply_structured_stream).
///
/// Keyed on the provider's `OutputFormat::Stream` record: the record's
/// `companion_flags` are pushed first, then its selector. `FlagValue`
/// pushes are unconditional — structured-stream setup only runs when the
/// user did not explicitly request a native output format, and this
/// reproduces the retired `push_stream_json_flags` overrides
/// byte-for-byte. `Flag`/`TransportFlag` selectors keep the `has_flag`
/// idempotence guard the retired Codex/Kimi overrides carried.
/// Providers without a `Stream` record are a no-op.
pub(super) fn apply_structured_stream(provider: Provider, args: &mut Vec<String>) {
    let Some(support) = provider_info(provider)
        .output_formats
        .iter()
        .find(|s| s.format == claudine::provider::OutputFormat::Stream)
    else {
        return;
    };
    for flag in support.companion_flags {
        args.push((*flag).to_string());
    }
    match support.selector {
        OutputFormatSelector::Flag { flag } | OutputFormatSelector::TransportFlag { flag } => {
            if !has_flag(args, flag) {
                args.push(flag.to_string());
            }
        }
        OutputFormatSelector::FlagValue { flag } => {
            args.push(flag.to_string());
            args.push(support.native_name.to_string());
        }
        OutputFormatSelector::Default | OutputFormatSelector::Positional { .. } => {}
    }
}

/// Catalog-driven default for
/// [`WrapperProfile::apply_system_prompt`](super::WrapperProfile::apply_system_prompt).
pub(super) fn apply_system_prompt(
    provider: Provider,
    prompt: &PreparedSystemPrompt,
    _interactive: bool,
    _cwd: &Path,
    _scoped_tmp: &Path,
) -> Result<SystemPromptApplication> {
    Ok(SystemPromptApplication {
        args: vec![],
        env: vec![],
        artifacts: vec![],
        warnings: vec![format!(
            "{provider} does not support {} system prompt; this flag was skipped",
            match prompt.mode {
                SystemPromptMode::Append => "append",
                SystemPromptMode::Replace => "replace",
            },
        )],
    })
}
