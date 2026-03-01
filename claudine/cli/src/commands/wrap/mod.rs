pub(crate) mod env;
mod exec;
pub(crate) mod profile;

use biscuit_terminal::terminal::Terminal;
use clap::Args;
use claudine::events::Provider;
use color_eyre::eyre::{Result, eyre};
use profile::{OutputFormat, WrapperProfile};
use sniff::programs::InstalledAiClients;
use std::path::PathBuf;

use crate::log;

/// Shared wrapper args for provider subcommands.
#[derive(Debug, Clone, Args)]
pub struct WrapperArgs {
    /// Enable provider-specific YOLO/auto-approval mode.
    #[arg(short = 'y', long)]
    pub yolo: bool,

    /// Preserve this env var even when it matches sensitive-name filters.
    #[arg(long = "include", value_name = "ENV_NAME")]
    pub include: Vec<String>,

    /// Force provider-specific non-interactive mode.
    #[arg(short = 'n', long = "non-interactive", visible_alias = "ni")]
    pub non_interactive: bool,

    /// Override the model used by the provider.
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Set the output format (json, text, stream).
    #[arg(short = 'o', long = "output", value_name = "FORMAT")]
    pub output: Option<String>,

    /// Set or append a system prompt (string or file path).
    #[arg(short = 's', long = "system-prompt", value_name = "PROMPT|FILE")]
    pub system_prompt: Option<String>,

    /// Timeout in seconds (sends SIGTERM then SIGKILL). Only valid with -n.
    #[arg(short = 't', long = "timeout", value_name = "SECONDS")]
    pub timeout: Option<u64>,

    /// Show what would be executed without launching the child.
    #[arg(long)]
    pub dry_run: bool,

    /// Suppress the Claudine preflight summary banner.
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Enable provider-specific sandboxing.
    #[arg(long)]
    pub sandbox: bool,

    /// Arguments forwarded to the wrapped provider CLI.
    #[arg(
        value_name = "ARGS",
        num_args = 0..,
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    pub passthrough: Vec<String>,
}

/// Run a wrapped provider command.
pub fn run_provider_wrapper(provider: Provider, args: WrapperArgs) -> Result<()> {
    let code = match run_provider_wrapper_inner(provider, args) {
        Ok(code) => code,
        Err(error) => {
            log::error(&error.to_string());
            1
        }
    };

    std::process::exit(code);
}

fn run_provider_wrapper_inner(provider: Provider, args: WrapperArgs) -> Result<i32> {
    let profile = profile::profile_for_provider(provider).ok_or_else(|| {
        eyre!(
            "'{}' cannot be wrapped (it is a VS Code extension)",
            provider
        )
    })?;
    let cwd = std::env::current_dir()?;
    let term = Terminal::new();

    let clients = InstalledAiClients::new();
    let binary_path = resolve_binary_path(profile, &clients)?;

    let raw_agent_params: Vec<String> = std::env::args().skip(2).collect();
    let mut child_args = args.passthrough.clone();
    let extracted = extract_wrapper_flags_from_passthrough(&mut child_args);
    let yolo_requested = args.yolo || extracted.yolo;
    let mut yolo_enabled = yolo_requested;
    let non_interactive_requested = args.non_interactive || extracted.non_interactive;
    let mut env_overrides: Vec<(String, String)> = Vec::new();
    let mut deferred_warnings: Vec<String> = Vec::new();
    let mut deferred_messages: Vec<String> = Vec::new();

    // Validate: --timeout requires --non-interactive
    if args.timeout.is_some() && !non_interactive_requested {
        return Err(eyre!(
            "--timeout can only be used with --non-interactive mode"
        ));
    }

    profile.reject_direct_yolo(&child_args)?;

    if yolo_requested && let Some(warn) = profile.apply_yolo(&mut child_args, &mut env_overrides)? {
        deferred_warnings.push(warn);
    }
    if yolo_requested && !profile.has_supported_yolo() {
        yolo_enabled = false;
    }

    if non_interactive_requested {
        profile.apply_non_interactive(&mut child_args)?;
        profile.apply_non_interactive_defaults(&mut child_args);
    }

    // Universal --model flag
    if let Some(ref model) = args.model
        && let Some(warn) = profile.apply_model(&mut child_args, &mut env_overrides, model)
    {
        deferred_warnings.push(warn);
    }

    // OpenCode non-interactive MODEL env var (from passthrough --model)
    if provider == Provider::OpenCode
        && non_interactive_requested
        && args.model.is_none()
        && let Some(model) = model_value_from_args(&child_args)
    {
        env_overrides.push(("MODEL".to_string(), model));
    }

    if provider == Provider::OpenCode && non_interactive_requested {
        deferred_messages.push(crate::output::opencode_non_interactive_model_hint());
    }

    // Universal --output flag
    if let Some(ref output_str) = args.output {
        let format: OutputFormat = output_str.parse().map_err(|e: String| eyre!(e))?;
        if let Some(warn) = profile.apply_output_format(&mut child_args, format) {
            deferred_warnings.push(warn);
        }
    }

    // Universal --system-prompt flag
    if let Some(ref prompt) = args.system_prompt {
        let resolved = resolve_system_prompt(prompt)?;
        if let Some(warn) = profile.apply_system_prompt(&mut child_args, &resolved) {
            deferred_warnings.push(warn);
        }
    }

    // Universal --sandbox flag
    if args.sandbox
        && let Some(warn) = profile.apply_sandbox(&mut child_args)
    {
        deferred_warnings.push(warn);
    }

    let env_plan = env::build_child_env(
        profile,
        &args.include,
        yolo_enabled,
        !non_interactive_requested,
        &raw_agent_params,
        &cwd,
        &env_overrides,
    )?;

    let child_cwd = env_plan.repo_root.as_deref().unwrap_or(&cwd);

    // --dry-run: print what would be executed and exit
    if args.dry_run {
        crate::output::log_dry_run(
            profile,
            &binary_path,
            &child_args,
            &env_plan,
            child_cwd,
            &term,
        );
        return Ok(0);
    }

    // --quiet: skip all summary output
    if !args.quiet {
        crate::output::log_wrapper_summary(
            profile,
            yolo_enabled,
            non_interactive_requested,
            &child_args,
            &env_plan,
            &term,
            args.verbose_level(),
        );

        if let Some(info_message) =
            crate::output::removed_env_info_message(&env_plan.removed, &term)
        {
            log::message(&info_message);
        }
        for warning in &env_plan.warnings {
            log::message(&crate::output::post_env_warning_message(warning, &term));
        }
        for warning in &deferred_warnings {
            log::message(&crate::output::post_env_warning_message(warning, &term));
        }
        for message in &deferred_messages {
            log::message(&crate::output::post_env_message(message, &term));
        }
    }

    exec::run_child(
        binary_path.as_path(),
        &child_args,
        &env_plan.env,
        child_cwd,
        args.timeout,
    )
}

impl WrapperArgs {
    /// Determine the effective verbosity level from the global -v/-vv flag.
    ///
    /// This reads the tracing subscriber level to determine verbosity:
    /// - 0 = default (header + badges)
    /// - 1 = verbose (+ env changes + warnings)
    /// - 2 = debug (+ full command + all debug info)
    fn verbose_level(&self) -> u8 {
        if tracing::enabled!(tracing::Level::DEBUG) {
            2
        } else if tracing::enabled!(tracing::Level::INFO) {
            1
        } else {
            0
        }
    }
}

fn resolve_binary_path(
    profile: &dyn WrapperProfile,
    clients: &InstalledAiClients,
) -> Result<PathBuf> {
    let ai_cli = profile.provider().sniff_ai_cli();
    clients.path(ai_cli).ok_or_else(|| {
        eyre!(
            "cannot run wrapped {} session because '{}' is not installed or not on PATH (docs: {})",
            profile.provider(),
            profile.binary(),
            profile.provider().docs_url()
        )
    })
}

fn model_value_from_args(args: &[String]) -> Option<String> {
    for (index, arg) in args.iter().enumerate() {
        if arg == "--model" || arg == "-m" {
            return args.get(index + 1).cloned();
        }
        if let Some(value) = arg.strip_prefix("--model=") {
            return Some(value.to_string());
        }
        if let Some(value) = arg.strip_prefix("-m=") {
            return Some(value.to_string());
        }
    }
    None
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ExtractedWrapperFlags {
    yolo: bool,
    non_interactive: bool,
}

fn extract_wrapper_flags_from_passthrough(args: &mut Vec<String>) -> ExtractedWrapperFlags {
    let mut extracted = ExtractedWrapperFlags::default();
    args.retain(|arg| match arg.as_str() {
        "-y" | "--yolo" => {
            extracted.yolo = true;
            false
        }
        "-n" | "--non-interactive" | "--ni" => {
            extracted.non_interactive = true;
            false
        }
        _ => true,
    });
    extracted
}

/// Resolve the `--system-prompt` value: if it looks like a file path and exists,
/// read its contents; otherwise treat it as a literal prompt string.
fn resolve_system_prompt(prompt_or_file: &str) -> Result<String> {
    let path = std::path::Path::new(prompt_or_file);
    if path.exists() && path.is_file() {
        Ok(std::fs::read_to_string(path)?)
    } else {
        Ok(prompt_or_file.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn missing_binary_preflight_has_actionable_message() {
        let clients = InstalledAiClients::default();
        let profile = profile::profile_for_provider(Provider::Codex).unwrap();

        let error = resolve_binary_path(profile, &clients).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("cannot run wrapped Codex session"));
        assert!(message.contains("docs:"));
    }

    #[test]
    fn package_name_display_shows_resolved_package_and_area() {
        let env_plan = env::EnvPlan {
            env: HashMap::new(),
            removed: Vec::new(),
            included: Vec::new(),
            added: Vec::new(),
            repo_root: None,
            package_context: Some(env::PackageContext {
                package_area: "claudine".to_string(),
                package: Some("claudine-cli".to_string()),
                candidates: vec!["claudine-cli".to_string()],
            }),
            warnings: Vec::new(),
        };

        let rendered = crate::output::package_name_display(&env_plan).unwrap();
        assert!(rendered.contains("claudine-cli"));
        assert!(rendered.contains("area: claudine"));
    }

    #[test]
    fn package_name_display_is_hidden_when_package_is_ambiguous() {
        let env_plan = env::EnvPlan {
            env: HashMap::new(),
            removed: Vec::new(),
            included: Vec::new(),
            added: Vec::new(),
            repo_root: None,
            package_context: Some(env::PackageContext {
                package_area: "claudine".to_string(),
                package: None,
                candidates: vec!["claudine".to_string(), "claudine-cli".to_string()],
            }),
            warnings: Vec::new(),
        };

        assert!(crate::output::package_name_display(&env_plan).is_none());
    }

    #[test]
    fn extract_wrapper_flags_lifts_reserved_aliases_from_passthrough() {
        let mut args = vec![
            "--json".to_string(),
            "--ni".to_string(),
            "task".to_string(),
            "-y".to_string(),
        ];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args);

        assert!(extracted.yolo);
        assert!(extracted.non_interactive);
        assert_eq!(args, vec!["--json", "task"]);
    }

    #[test]
    fn model_value_from_args_supports_short_and_long_forms() {
        let long_inline = vec!["--model=foo".to_string()];
        let short_next = vec!["-m".to_string(), "bar".to_string()];

        assert_eq!(model_value_from_args(&long_inline), Some("foo".to_string()));
        assert_eq!(model_value_from_args(&short_next), Some("bar".to_string()));
    }

    #[test]
    fn resolve_system_prompt_returns_literal_for_non_file() {
        let result = resolve_system_prompt("You are a helpful assistant.").unwrap();
        assert_eq!(result, "You are a helpful assistant.");
    }

    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn proptest_resolve_system_prompt_never_panics(s in "\\PC*") {
                let _ = resolve_system_prompt(&s);
            }

            #[test]
            fn proptest_extract_wrapper_flags_preserves_others(
                flags in prop::collection::vec("-y|--yolo|-n|--non-interactive|--ni", 0..5),
                others in prop::collection::vec("[a-z0-9]+", 0..10)
            ) {
                let mut args = Vec::new();
                for o in &others {
                    args.push(o.clone());
                }
                for f in &flags {
                    args.push(f.clone());
                }

                // Shuffle manually or just accept order for now
                let extracted = extract_wrapper_flags_from_passthrough(&mut args);

                // All 'others' should still be there
                assert_eq!(args.len(), others.len());
                for o in others {
                    assert!(args.contains(&o));
                }

                if flags.iter().any(|f| f == "-y" || f == "--yolo") {
                    assert!(extracted.yolo);
                }
                if flags.iter().any(|f| f == "-n" || f == "--non-interactive" || f == "--ni") {
                    assert!(extracted.non_interactive);
                }
            }
        }
    }
}
