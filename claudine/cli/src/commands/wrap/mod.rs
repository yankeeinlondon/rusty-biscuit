mod env;
mod exec;
pub(crate) mod profile;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{Renderable, RenderableContent};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;
use clap::Args;
use claudine::badges::{NON_INTERACTIVE, YOLO};
use claudine::events::Provider;
use color_eyre::eyre::{Result, eyre};
use profile::{OutputFormat, WrapperProfile};
use sniff::programs::InstalledAiClients;
use std::path::{Path, PathBuf};

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
    let profile = profile::profile_for_provider(provider)
        .ok_or_else(|| eyre!("'{}' cannot be wrapped (it is a VS Code extension)", provider))?;
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
        return Err(eyre!("--timeout can only be used with --non-interactive mode"));
    }

    profile.reject_direct_yolo(&child_args)?;

    if yolo_requested
        && let Some(warn) = profile.apply_yolo(&mut child_args, &mut env_overrides)?
    {
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
        deferred_messages.push(opencode_non_interactive_model_hint());
    }

    // Universal --output flag
    if let Some(ref output_str) = args.output {
        let format: OutputFormat = output_str
            .parse()
            .map_err(|e: String| eyre!(e))?;
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
        log_dry_run(profile, &binary_path, &child_args, &env_plan, child_cwd, &term);
        return Ok(0);
    }

    // --quiet: skip all summary output
    if !args.quiet {
        log_wrapper_summary(
            profile,
            yolo_enabled,
            non_interactive_requested,
            &child_args,
            &env_plan,
            &term,
            args.verbose_level(),
        );

        if let Some(info_message) = removed_env_info_message(&env_plan.removed, &term) {
            log::message(&info_message);
        }
        for warning in &env_plan.warnings {
            log::message(&post_env_warning_message(warning, &term));
        }
        for warning in &deferred_warnings {
            log::message(&post_env_warning_message(warning, &term));
        }
        for message in &deferred_messages {
            log::message(&post_env_message(message, &term));
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

fn log_wrapper_summary(
    profile: &dyn WrapperProfile,
    yolo_requested: bool,
    non_interactive_requested: bool,
    child_args: &[String],
    env_plan: &env::EnvPlan,
    term: &Terminal,
    verbose: u8,
) {
    // Header always includes: Claudine ▸ ProviderName [badges]
    let mut header_parts: Vec<String> = vec![
        Prose::new(format!(
            "<blue><bold>Claudine</bold></blue> <dim>\u{25b8}</dim> <bold>{}</bold>",
            profile.provider()
        ))
        .fallback_render(term),
    ];

    if yolo_requested {
        header_parts.push(YOLO.to_string());
    }

    if non_interactive_requested {
        header_parts.push(NON_INTERACTIVE.to_string());
    }

    if let Some(package_name) = package_name_display(env_plan) {
        header_parts.push(
            Prose::new(format!(
                "<green><bold>PACKAGE_NAME:</bold> {package_name}</green>"
            ))
            .fallback_render(term),
        );
    }

    let remaining = format_passthrough_args(child_args);
    if !remaining.is_empty() {
        header_parts.push(Prose::new(format!("<dim>{remaining}</dim>")).fallback_render(term));
    }

    log::message(&format!("\n{}", header_parts.join(" ")));

    // Verbose level 0: header only (no env details)
    // Verbose level 1+: header + environment changes
    if verbose > 0 || !env_plan.added.is_empty() || !env_plan.removed.is_empty() {
        log::message(
            &Prose::new("<bold>Environment Variables:</bold>").fallback_render(term),
        );

        let mut items: Vec<RenderableContent> = Vec::new();
        for removed in &env_plan.removed {
            items.push(RenderableContent::from(Prose::new(format!(
                "<red><strikethrough>{removed}</strikethrough></red>"
            ))));
        }

        for (key, value) in &env_plan.added {
            items.push(RenderableContent::from(Prose::new(format!(
                "<green>{key}</green><dim>={}</dim>",
                summarize_value(key, value)
            ))));
        }

        if items.is_empty() {
            items.push(RenderableContent::from(Prose::new(
                "<dim>no environment changes</dim>",
            )));
        }

        let rendered = UnorderedList::from(items)
            .with_bullet("• ")
            .fallback_render(term);
        log::message(&rendered);
    }
}

fn log_dry_run(
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_args: &[String],
    env_plan: &env::EnvPlan,
    child_cwd: &Path,
    term: &Terminal,
) {
    log::message(
        &Prose::new(format!(
            "\n<blue><bold>Claudine</bold></blue> <dim>\u{25b8}</dim> <bold>{}</bold> <dim>[DRY RUN]</dim>",
            profile.provider()
        ))
        .fallback_render(term),
    );

    // Working directory
    log::message(
        &Prose::new(format!(
            "<bold>Working directory:</bold> <dim>{}</dim>",
            child_cwd.display()
        ))
        .fallback_render(term),
    );

    // Full command line
    let cmd_parts: Vec<String> = std::iter::once(binary_path.display().to_string())
        .chain(child_args.iter().map(|a| shell_escape(a)))
        .collect();
    log::message(
        &Prose::new(format!("<bold>Command:</bold> <dim>{}</dim>", cmd_parts.join(" ")))
            .fallback_render(term),
    );

    // Environment changes
    log::message(
        &Prose::new("<bold>Environment Changes:</bold>").fallback_render(term),
    );
    let mut items: Vec<RenderableContent> = Vec::new();
    for removed in &env_plan.removed {
        items.push(RenderableContent::from(Prose::new(format!(
            "<red><strikethrough>{removed}</strikethrough></red>"
        ))));
    }
    for (key, value) in &env_plan.added {
        items.push(RenderableContent::from(Prose::new(format!(
            "<green>{key}</green><dim>={value}</dim>"
        ))));
    }
    if items.is_empty() {
        items.push(RenderableContent::from(Prose::new(
            "<dim>no environment changes</dim>",
        )));
    }
    let rendered = UnorderedList::from(items)
        .with_bullet("• ")
        .fallback_render(term);
    log::message(&rendered);
}

fn summarize_value(key: &str, value: &str) -> String {
    if key == "AGENT_PARAMS" && value.len() > 120 {
        return format!("{}...", &value[..117]);
    }
    value.to_string()
}

fn package_name_display(env_plan: &env::EnvPlan) -> Option<String> {
    let package_context = env_plan.package_context.as_ref()?;
    let package = package_context.package.as_deref()?;

    Some(format!(
        "{package} <dim>(area: {})</dim>",
        package_context.package_area
    ))
}

fn opencode_non_interactive_model_hint() -> String {
    "<bold><blue>Info:</blue></bold> Opencode requires a model be specified when run in \
     non-interactive mode. You can specify with the --model switch or set either OPENCODE_MODEL \
     or MODEL environment variables."
        .to_string()
}

fn removed_env_info_message(removed_env: &[String], term: &Terminal) -> Option<String> {
    let example_env = removed_env.first()?;
    Some(
        Prose::new(format!(
            "- <blue><bold>Info:</bold></blue> potentially dangerous ENV variables were removed; \
             if you need one of these to be included use the <blue>--include \
             <dim>{example_env}</dim></blue> CLI switch"
        ))
        .with_word_wrap(WordWrap::WrapProse(Some(8), Some(3)))
        .fallback_render(term),
    )
}

fn post_env_message(message: &str, term: &Terminal) -> String {
    let styled = style_cli_switches(message);
    Prose::new(format!("- {styled}"))
        .with_word_wrap(WordWrap::WrapProse(Some(8), Some(3)))
        .fallback_render(term)
}

fn post_env_warning_message(message: &str, term: &Terminal) -> String {
    let styled = style_cli_switches(message);
    Prose::new(format!(
        "- <orange><bold>Warning:</bold></orange> {styled}"
    ))
    .with_word_wrap(WordWrap::WrapProse(Some(8), Some(3)))
    .fallback_render(term)
}

fn style_cli_switches(message: &str) -> String {
    let bytes = message.as_bytes();
    let mut i = 0usize;
    let mut last = 0usize;
    let mut out = String::with_capacity(message.len() + 32);

    while i < bytes.len() {
        let Some((start, end)) = next_switch_span(bytes, i) else {
            break;
        };
        out.push_str(&message[last..start]);
        out.push_str("<blue>");
        out.push_str(&message[start..end]);
        out.push_str("</blue>");
        i = end;
        last = end;
    }

    out.push_str(&message[last..]);
    out
}

fn next_switch_span(bytes: &[u8], start_at: usize) -> Option<(usize, usize)> {
    let mut i = start_at;
    while i < bytes.len() {
        if bytes[i] == b'-' && is_switch_boundary(bytes, i) {
            if i + 2 < bytes.len() && bytes[i + 1] == b'-' && is_switch_start(bytes[i + 2]) {
                let mut j = i + 3;
                while j < bytes.len() && is_switch_continue(bytes[j]) {
                    j += 1;
                }
                return Some((i, j));
            }
            if i + 1 < bytes.len() && is_switch_start(bytes[i + 1]) {
                let mut j = i + 2;
                while j < bytes.len() && is_switch_continue(bytes[j]) {
                    j += 1;
                }
                return Some((i, j));
            }
        }
        i += 1;
    }
    None
}

fn is_switch_boundary(bytes: &[u8], index: usize) -> bool {
    if index == 0 {
        return true;
    }

    !bytes[index - 1].is_ascii_alphanumeric()
}

fn is_switch_start(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
}

fn is_switch_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
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

fn format_passthrough_args(args: &[String]) -> String {
    args.iter()
        .map(|arg| shell_escape(arg))
        .collect::<Vec<_>>()
        .join(" ")
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

fn shell_escape(arg: &str) -> String {
    if arg.is_empty() {
        return "''".to_string();
    }

    if arg
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '='))
    {
        return arg.to_string();
    }

    format!("'{}'", arg.replace('\'', "'\\''"))
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
            added: Vec::new(),
            repo_root: None,
            package_context: Some(env::PackageContext {
                package_area: "claudine".to_string(),
                package: Some("claudine-cli".to_string()),
                candidates: vec!["claudine-cli".to_string()],
            }),
            warnings: Vec::new(),
        };

        let rendered = package_name_display(&env_plan).unwrap();
        assert!(rendered.contains("claudine-cli"));
        assert!(rendered.contains("area: claudine"));
    }

    #[test]
    fn package_name_display_is_hidden_when_package_is_ambiguous() {
        let env_plan = env::EnvPlan {
            env: HashMap::new(),
            removed: Vec::new(),
            added: Vec::new(),
            repo_root: None,
            package_context: Some(env::PackageContext {
                package_area: "claudine".to_string(),
                package: None,
                candidates: vec!["claudine".to_string(), "claudine-cli".to_string()],
            }),
            warnings: Vec::new(),
        };

        assert!(package_name_display(&env_plan).is_none());
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
}
