pub(crate) mod env;
pub(crate) mod exec;
pub(crate) mod profile;
pub(crate) mod prompt_file;
pub(crate) mod repo_home;

use biscuit_terminal::terminal::Terminal;
use clap::Args;
use claudine::events::Provider;
use color_eyre::eyre::{Result, eyre};
use inquire::Select;
use profile::{OutputFormat, WrapperProfile};
use sniff::programs::InstalledAiClients;
use std::io::IsTerminal;
use std::path::PathBuf;

use crate::log;

#[derive(Debug, Clone, Default)]
pub(crate) struct McpRuntimeInfo {
    pub(crate) servers: Vec<String>,
    pub(crate) default_servers: Vec<String>,
    pub(crate) explicit_servers: Vec<String>,
    pub(crate) tag_servers: Vec<String>,
    pub(crate) resolved_tags: Vec<String>,
    pub(crate) missing_tags: Vec<String>,
    pub(crate) ambiguous_tags: Vec<String>,
    pub(crate) cleaned_prompt: Option<String>,
    pub(crate) env_vars_set: Vec<String>,
    pub(crate) temp_files: Vec<PathBuf>,
    pub(crate) extra_args: Vec<String>,
}

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

    /// Show only the header line; suppress env details and info messages.
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Suppress all Claudine preflight output (header, env, info, warnings).
    #[arg(long, conflicts_with = "quiet")]
    pub silent: bool,

    /// Set the OPERATION env var for the wrapped session.
    #[arg(long = "operation", visible_alias = "op", value_name = "OP")]
    pub operation: Option<String>,

    /// Enable provider-specific sandboxing.
    #[arg(long)]
    pub sandbox: bool,

    /// Use only repo-scoped skills, commands, and agents via a shadow HOME.
    #[arg(long)]
    pub repo: bool,

    /// Source the initial prompt from a Markdown file (composed with Darkmatter).
    #[arg(short = 'p', long = "prompt-file", value_name = "FILE")]
    pub prompt_file: Option<String>,

    /// Inline composition: use frontmatter `prompt` as input, replace body with output.
    #[arg(long = "frontmatter-prompt", visible_alias = "fp", value_name = "FILE", conflicts_with_all = ["prompt_file", "compose"])]
    pub frontmatter_prompt: Option<String>,

    /// Chained composition: compose full document and use as prompt (no file mutation).
    #[arg(long = "compose", value_name = "FILE", conflicts_with_all = ["prompt_file", "frontmatter_prompt"])]
    pub compose: Option<String>,

    /// Enable Claudine-managed MCP session composition.
    #[arg(long)]
    pub mcp: bool,

    /// Activate specific MCP servers by ID or alias (comma-separated).
    #[arg(long = "use", value_name = "ID", value_delimiter = ',')]
    pub mcp_use: Vec<String>,

    /// Treat unresolved or ambiguous MCP tags as hard errors.
    #[arg(long)]
    pub strict: bool,

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
pub fn run_provider_wrapper(
    provider: Provider,
    args: WrapperArgs,
    verbose: u8,
) -> Result<()> {
    let code = match run_provider_wrapper_inner(provider, args, verbose) {
        Ok(code) => code,
        Err(error) => {
            log::error(&error.to_string());
            1
        }
    };

    std::process::exit(code);
}

fn run_provider_wrapper_inner(provider: Provider, args: WrapperArgs, verbose: u8) -> Result<i32> {
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
    let repo_requested = args.repo || extracted.repo;
    let quiet_requested = args.quiet || extracted.quiet;
    let silent_requested = args.silent || extracted.silent;
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

    // Universal --operation flag (clap-parsed or extracted from passthrough)
    let effective_operation = args.operation.clone().or(extracted.operation);
    if let Some(ref op) = effective_operation {
        env_overrides.push(("OPERATION".to_string(), op.clone()));
    }

    // Universal --sandbox flag
    if args.sandbox
        && let Some(warn) = profile.apply_sandbox(&mut child_args)
    {
        deferred_warnings.push(warn);
    }

    let needs_mcp_shadow_home = (args.mcp || !args.mcp_use.is_empty())
        && matches!(provider, Provider::Codex | Provider::Gemini);

    let mut env_plan = env::build_child_env(
        profile,
        provider,
        &args.include,
        yolo_enabled,
        !non_interactive_requested,
        &raw_agent_params,
        &cwd,
        &env_overrides,
        repo_requested,
        needs_mcp_shadow_home,
    )?;

    // -- Prompt-file pipeline -------------------------------------------------
    let mut stdin_seed: Option<String> = None;
    let mut prompt_file_dry_run: Option<prompt_file::PromptFileDryRunInfo> = None;

    if let Some(ref prompt_file_input) = args.prompt_file {
        let pf_ctx = prompt_file::PromptResolutionContext {
            cwd: cwd.clone(),
            repo_root: env_plan.repo_root.clone(),
            package_root: env_plan
                .package_context
                .as_ref()
                .and_then(|pc| {
                    // Derive package root from repo_root + package_area
                    env_plan
                        .repo_root
                        .as_ref()
                        .map(|rr| rr.join(&pc.package_area))
                }),
            interactive: std::io::stdin().is_terminal()
                && std::io::stdout().is_terminal()
                && !non_interactive_requested,
        };

        let resolved = prompt_file::resolve_prompt_file(prompt_file_input, &pf_ctx)?;
        let composed = prompt_file::compose_prompt_file(&resolved)?;

        // Detect conflict with existing prompt source
        prompt_file::detect_existing_prompt_source(profile, &child_args, provider)?;

        // Deliver the composed prompt to the provider BEFORE applying
        // non-interactive mode, because some providers (Gemini) validate
        // that a prompt is present in args during apply_non_interactive.
        let delivery_method =
            if matches!(provider, Provider::Claude | Provider::KimiCode)
                || matches!(provider, Provider::Codex | Provider::OpenCode)
            {
                "stdin"
            } else {
                "args"
            };
        profile.apply_prompt_body(
            &mut child_args,
            &mut stdin_seed,
            &composed.body,
            true, // always non-interactive for prompt-file composition
        )?;

        // Force non-interactive for prompt-file composition
        if !non_interactive_requested {
            profile.apply_non_interactive(&mut child_args)?;
            profile.apply_non_interactive_defaults(&mut child_args);
        }

        // Add prompt-file env vars to child environment
        for (key, value) in &composed.env_overrides {
            env_plan
                .env
                .insert(key.clone().into(), value.clone().into());
            env_plan
                .added
                .push((key.clone(), value.clone()));
        }

        prompt_file_dry_run = Some(prompt_file::PromptFileDryRunInfo {
            original: resolved.original.clone(),
            resolved_path: composed.resolved_path.clone(),
            delivery_method: delivery_method.to_string(),
            env_names: composed.env_names.clone(),
        });
    }

    // -- Frontmatter-prompt (inline composition) pipeline --------------------
    let mut inline_composition_source: Option<(
        claudine::composition::ResolvedCompositionSource,
        claudine::composition::PreparedPrompt,
    )> = None;

    if let Some(ref fp_input) = args.frontmatter_prompt {
        let source = claudine::composition::resolve_composition_source(fp_input)
            .map_err(|e| eyre!("frontmatter-prompt: {e}"))?;
        let prepared = claudine::composition::prepare_inline_prompt(&source)
            .map_err(|e| eyre!("frontmatter-prompt: {e}"))?;

        // Detect conflict with existing prompt source
        prompt_file::detect_existing_prompt_source(profile, &child_args, provider)?;

        // Deliver the composed prompt to the provider BEFORE applying
        // non-interactive mode, because some providers (Gemini) validate
        // that a prompt is present in args during apply_non_interactive.
        profile.apply_prompt_body(
            &mut child_args,
            &mut stdin_seed,
            &prepared.prompt,
            true, // always non-interactive for inline composition
        )?;

        // Force non-interactive for inline composition
        if !non_interactive_requested {
            profile.apply_non_interactive(&mut child_args)?;
            profile.apply_non_interactive_defaults(&mut child_args);
        }

        inline_composition_source = Some((source, prepared));
    }

    // -- Chained composition (--compose) pipeline ------------------------------
    let mut chained_composition = false;

    if let Some(ref compose_input) = args.compose {
        let source = claudine::composition::resolve_composition_source(compose_input)
            .map_err(|e| eyre!("compose: {e}"))?;
        let prepared = claudine::composition::prepare_chained_prompt(&source)
            .map_err(|e| eyre!("compose: {e}"))?;

        // Detect conflict with existing prompt source
        prompt_file::detect_existing_prompt_source(profile, &child_args, provider)?;

        // Deliver the composed document to the provider BEFORE applying
        // non-interactive mode, because some providers (Gemini) validate
        // that a prompt is present in args during apply_non_interactive.
        profile.apply_prompt_body(
            &mut child_args,
            &mut stdin_seed,
            &prepared.prompt,
            true, // always non-interactive for chained composition
        )?;

        // Force non-interactive for chained composition
        if !non_interactive_requested {
            profile.apply_non_interactive(&mut child_args)?;
            profile.apply_non_interactive_defaults(&mut child_args);
        }

        chained_composition = true;
    }

    // -- Final argument validation -------------------------------------------
    // All prompt sources (passthrough, --prompt-file, --frontmatter-prompt,
    // --compose) have now been processed. Validate that providers requiring a
    // positional prompt actually have one.
    let effective_non_interactive = non_interactive_requested
        || prompt_file_dry_run.is_some()
        || inline_composition_source.is_some()
        || chained_composition;
    profile.validate_final_args(&child_args, effective_non_interactive, stdin_seed.is_some())?;

    // If a composition pipeline inferred non-interactive mode, update the
    // INTERACTIVE env var that was set before the pipelines ran.
    if effective_non_interactive && !non_interactive_requested {
        env_plan
            .env
            .insert("INTERACTIVE".into(), "false".into());
    }

    let mut mcp_runtime = None;
    let mut mcp_cleanup: Option<(Box<dyn claudine::mcp::inject::McpInjector>, claudine::mcp::inject::InjectionResult)> = None;

    // MCP session composition
    if args.mcp || !args.mcp_use.is_empty() {
        use claudine::mcp::catalog::McpCatalogStore;
        use claudine::mcp::inject::injector_for_provider;
        use claudine::mcp::session::{compute_session_set, lex_tags};

        let repo_root_ref = env_plan.repo_root.as_deref();
        if bootstrap_mcp_state(repo_root_ref)? {
            deferred_messages.push(
                "MCP bootstrap: created Claudine MCP state from discoverable provider configs."
                    .to_string(),
            );
        }
        let catalog =
            McpCatalogStore::load().map_err(|e| eyre!("failed to load MCP catalog: {e}"))?;
        let (cleaned_prompt, prompt_tags) =
            extract_tags_from_child_args(provider, &mut child_args, lex_tags);
        let prompt_is_interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
        let mut session = compute_session_set(
            &catalog,
            repo_root_ref,
            &args.mcp_use,
            &prompt_tags,
            |tag, _tier, candidates| {
                if args.strict || non_interactive_requested || !prompt_is_interactive {
                    return None;
                }
                Select::new(
                    &format!("`#{tag}` matched multiple MCP servers. Choose one:"),
                    candidates.to_vec(),
                )
                .prompt()
                .ok()
            },
        )
        .map_err(|e| eyre!("MCP session error: {e}"))?;
        session.cleaned_prompt = cleaned_prompt.clone();

        for warning in &session.warnings {
            deferred_warnings.push(warning.clone());
        }
        if !session.missing_tags.is_empty() {
            if args.strict {
                return Err(eyre!(
                    "unresolved MCP tag(s): {}",
                    session
                        .missing_tags
                        .iter()
                        .map(|tag| format!("#{tag}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            for tag in &session.missing_tags {
                deferred_warnings.push(format!("tag `#{tag}` was not found in the MCP catalog"));
            }
        }
        if !session.ambiguous_tags.is_empty() {
            if args.strict || non_interactive_requested {
                let message = session
                    .ambiguous_tags
                    .iter()
                    .map(|tag| format!("#{} -> {}", tag.tag, tag.candidates.join(", ")))
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(eyre!("ambiguous MCP tag(s): {message}"));
            }
            // Interactive non-strict: warn and drop ambiguous tags
            for tag in &session.ambiguous_tags {
                deferred_warnings.push(format!(
                    "tag `#{}` is ambiguous ({}); dropped from session",
                    tag.tag,
                    tag.candidates.join(", ")
                ));
            }
            session.ambiguous_tags.clear();
        }

        let mut runtime = McpRuntimeInfo {
            servers: session
                .servers
                .iter()
                .map(|server| server.id.clone())
                .collect(),
            default_servers: session.default_servers.clone(),
            explicit_servers: session.explicit_servers.clone(),
            tag_servers: session.tag_servers.clone(),
            resolved_tags: session
                .resolved_tags
                .iter()
                .map(|tag| format!("#{} -> {} ({:?})", tag.tag, tag.resolved_to, tag.match_tier))
                .collect(),
            missing_tags: session
                .missing_tags
                .iter()
                .map(|tag| format!("#{tag}"))
                .collect(),
            ambiguous_tags: session
                .ambiguous_tags
                .iter()
                .map(|tag| format!("#{} -> {}", tag.tag, tag.candidates.join(", ")))
                .collect(),
            cleaned_prompt: session.cleaned_prompt.clone(),
            ..McpRuntimeInfo::default()
        };

        if let Some(injector) = injector_for_provider(provider) {
            if !session.servers.is_empty() {
                let shadow = env_plan.shadow_home_path.as_deref();
                // Injector works with String env; bridge to OsString env plan
                let mut string_env = std::collections::HashMap::new();
                let result = injector
                    .inject(&session.servers, &mut string_env, shadow)
                    .map_err(|e| eyre!("MCP injection failed: {e}"))?;

                // Merge injected env vars into the OsString env plan
                for (k, v) in string_env {
                    env_plan.env.insert(k.into(), v.into());
                }

                for arg in &result.extra_args {
                    child_args.push(arg.clone());
                }

                runtime.env_vars_set = result.env_vars_set.clone();
                runtime.temp_files = result.temp_files.clone();
                runtime.extra_args = result.extra_args.clone();

                mcp_cleanup = Some((injector, result));
            }
        } else {
            return Err(eyre!(
                "provider {} does not support runtime MCP injection.\n\
                 Use `claudine mcp export {} --apply` to write servers to its native config instead.",
                provider,
                provider.as_slug()
            ));
        }

        deferred_messages.push(if runtime.servers.is_empty() {
            "MCP: no active servers".to_string()
        } else {
            format!("MCP: {}", runtime.servers.join(", "))
        });
        if !runtime.resolved_tags.is_empty() {
            deferred_messages.push(format!("MCP tags: {}", runtime.resolved_tags.join(", ")));
        }
        mcp_runtime = Some(runtime);
    }

    let child_cwd = env_plan.repo_root.as_deref().unwrap_or(&cwd);

    // --dry-run: print what would be executed and exit
    if args.dry_run {
        crate::output::log_dry_run(
            profile,
            &binary_path,
            &child_args,
            repo_requested,
            &env_plan,
            mcp_runtime.as_ref(),
            prompt_file_dry_run.as_ref(),
            child_cwd,
            &term,
        );
        return Ok(0);
    }

    // Determine compose display mode and prompt summary for header
    let compose_display = if inline_composition_source.is_some() {
        Some(crate::output::ComposeDisplay::InlineCompose)
    } else if chained_composition {
        Some(crate::output::ComposeDisplay::Compose)
    } else {
        None
    };

    // For compose modes, the prompt goes to stdin (not child_args), so we need
    // to extract a summary to display in the header. For regular runs the prompt
    // is already in child_args.
    let prompt_summary: Option<String> = if let Some((_, ref prepared)) = inline_composition_source
    {
        Some(prepared.prompt.clone())
    } else if chained_composition {
        stdin_seed.clone()
    } else {
        None
    };

    // Output verbosity: --silent suppresses everything, --quiet shows header only
    if !silent_requested {
        // Header line (shown for both default and --quiet)
        crate::output::log_wrapper_header(
            profile,
            yolo_enabled,
            effective_non_interactive,
            repo_requested,
            compose_display.as_ref(),
            effective_operation.as_deref(),
            &child_args,
            prompt_summary.as_deref(),
            &env_plan,
            &term,
        );

        // Everything below is suppressed by --quiet
        if !quiet_requested {
            crate::output::log_wrapper_env_details(
                &env_plan,
                mcp_runtime.as_ref(),
                &term,
                verbose,
            );

            if let Some(info_message) =
                crate::output::removed_env_info_message(&env_plan.removed, &term)
            {
                log::message(&info_message);
            }
            if repo_requested {
                log::message(&crate::output::repo_flag_info_message(
                    &term,
                    env_plan.shadow_home_path.as_deref(),
                ));
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
    }

    let stdout_noise = if effective_non_interactive {
        profile.stdout_noise_prefixes()
    } else {
        &[]
    };
    let stderr_noise = profile.stderr_noise_prefixes();

    // For captured-output paths (inline compose), let the profile inject
    // structured output flags (e.g. Gemini's --output-format stream-json)
    // so we can reliably extract the assistant response from noisy stdout.
    // Chained compose uses run_child (forwards to terminal), so it relies
    // on prefix-based noise filtering instead.
    if inline_composition_source.is_some() {
        profile.prepare_captured_output(&mut child_args);
    }

    let exit_code = if let Some((source, _prepared)) = inline_composition_source {
        // Inline composition: capture output and update file
        let captured = exec::run_child_capture(
            binary_path.as_path(),
            &child_args,
            &env_plan.env,
            child_cwd,
            args.timeout,
            exec::ChildIoOptions {
                stdout_noise_prefixes: stdout_noise,
                stderr_noise_prefixes: stderr_noise,
                stdin_seed: stdin_seed.as_deref(),
            },
        )?;

        if captured.exit_code == 0 {
            // Build updated document: original frontmatter + last_updated + captured body
            let mut updated_md = source.markdown.clone();
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            updated_md.fm_insert("last_updated", &today)
                .map_err(|e| eyre!("failed to update last_updated: {e}"))?;
            *updated_md.content_mut() = profile.parse_captured_output(&captured.stdout);

            let doc_string = updated_md.as_string();
            claudine::config::atomic::atomic_write(
                &source.resolved_path,
                doc_string.as_bytes(),
            )
            .map_err(|e| eyre!("failed to write inline composition result: {e}"))?;

            if !silent_requested && !quiet_requested {
                log::message(&format!(
                    "  \x1b[32m✓\x1b[0m Updated {}",
                    source.resolved_path.display()
                ));
            }
        } else if !captured.stderr.is_empty() {
            eprintln!("{}", captured.stderr);
        }

        Ok(captured.exit_code)
    } else {
        // Normal execution: forward I/O to terminal
        exec::run_child(
            binary_path.as_path(),
            &child_args,
            &env_plan.env,
            child_cwd,
            args.timeout,
            exec::ChildIoOptions {
                stdout_noise_prefixes: stdout_noise,
                stderr_noise_prefixes: stderr_noise,
                stdin_seed: stdin_seed.as_deref(),
            },
        )
    };

    // MCP injector cleanup: remove temp files written during injection
    if let Some((injector, injection_result)) = mcp_cleanup
        && let Err(e) = injector.cleanup(&injection_result)
    {
        tracing::warn!("MCP injector cleanup failed: {e}");
    }

    exit_code
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptLocation {
    Value(usize),
    Inline { index: usize, prefix: &'static str },
}

fn extract_tags_from_child_args(
    provider: Provider,
    args: &mut [String],
    extract_tags: fn(&str) -> (String, Vec<String>),
) -> (Option<String>, Vec<String>) {
    let Some(location) = find_prompt_location(provider, args) else {
        return (None, Vec::new());
    };

    let prompt = match location {
        PromptLocation::Value(index) => args[index].clone(),
        PromptLocation::Inline { index, prefix } => args[index]
            .strip_prefix(prefix)
            .unwrap_or_default()
            .to_string(),
    };

    let (cleaned_prompt, tags) = extract_tags(&prompt);
    if tags.is_empty() {
        return (None, tags);
    }

    match location {
        PromptLocation::Value(index) => args[index] = cleaned_prompt.clone(),
        PromptLocation::Inline { index, prefix } => {
            args[index] = format!("{prefix}{cleaned_prompt}");
        }
    }

    (Some(cleaned_prompt), tags)
}

fn bootstrap_mcp_state(repo_root: Option<&std::path::Path>) -> Result<bool> {
    use claudine::mcp::defaults::{save_repo_defaults, save_user_defaults};
    use claudine::mcp::import::McpImporter;
    use claudine::mcp::state::McpProviderStateStore;
    use claudine::mcp::types::{McpDefaults, defaults_path, repo_defaults_path};

    let needs_bootstrap = !claudine::mcp::types::catalog_path().exists()
        || !defaults_path().exists()
        || !claudine::mcp::types::provider_state_path().exists()
        || repo_root.is_some_and(|root| !repo_defaults_path(root).exists());
    if !needs_bootstrap {
        return Ok(false);
    }

    let mut catalog = claudine::mcp::catalog::McpCatalogStore::load()
        .map_err(|e| eyre!("failed to load MCP catalog for bootstrap: {e}"))?;
    let mut state = McpProviderStateStore::load()
        .map_err(|e| eyre!("failed to load MCP provider-state for bootstrap: {e}"))?;
    let mut importer = McpImporter::new(&mut catalog, &mut state);
    let _ = importer.import_all(repo_root);
    catalog
        .save()
        .map_err(|e| eyre!("failed to save bootstrapped MCP catalog: {e}"))?;
    state
        .save()
        .map_err(|e| eyre!("failed to save bootstrapped MCP provider-state: {e}"))?;

    if !defaults_path().exists() {
        save_user_defaults(&McpDefaults::default())
            .map_err(|e| eyre!("failed to create bootstrapped MCP defaults: {e}"))?;
    }
    if let Some(repo_root) = repo_root
        && !repo_defaults_path(repo_root).exists()
    {
        save_repo_defaults(repo_root, &McpDefaults::default())
            .map_err(|e| eyre!("failed to create bootstrapped repo MCP defaults: {e}"))?;
    }

    Ok(true)
}

fn find_prompt_location(provider: Provider, args: &[String]) -> Option<PromptLocation> {
    match provider {
        Provider::Gemini => find_gemini_prompt_location(args),
        Provider::Codex => find_positional_prompt_location(args, 1),
        Provider::OpenCode => find_positional_prompt_location(args, 1),
        _ => None,
    }
}

fn find_gemini_prompt_location(args: &[String]) -> Option<PromptLocation> {
    for (index, arg) in args.iter().enumerate() {
        if arg == "--prompt" || arg == "-p" {
            return (index + 1 < args.len()).then_some(PromptLocation::Value(index + 1));
        }
        if arg.starts_with("--prompt=") {
            return Some(PromptLocation::Inline {
                index,
                prefix: "--prompt=",
            });
        }
        if arg.starts_with("-p=") {
            return Some(PromptLocation::Inline {
                index,
                prefix: "-p=",
            });
        }
    }

    find_positional_prompt_location(args, 0)
}

fn find_positional_prompt_location(args: &[String], start_index: usize) -> Option<PromptLocation> {
    let mut skip_next = false;

    for (index, arg) in args.iter().enumerate().skip(start_index) {
        if skip_next {
            skip_next = false;
            continue;
        }

        if index == 0 && (arg == "exec" || arg == "run" || arg == "e") {
            continue;
        }

        if arg == "--" {
            return (index + 1 < args.len()).then_some(PromptLocation::Value(index + 1));
        }

        if takes_value(arg) {
            skip_next = !arg.contains('=');
            continue;
        }

        if !arg.starts_with('-') {
            return Some(PromptLocation::Value(index));
        }
    }

    None
}

fn takes_value(arg: &str) -> bool {
    matches!(
        arg,
        "-m" | "--model"
            | "-o"
            | "--output"
            | "--output-format"
            | "--approval-mode"
            | "--config"
            | "-c"
            | "--profile"
            | "--system-prompt"
            | "--sandbox-image"
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExtractedWrapperFlags {
    yolo: bool,
    non_interactive: bool,
    repo: bool,
    quiet: bool,
    silent: bool,
    operation: Option<String>,
}

fn extract_wrapper_flags_from_passthrough(args: &mut Vec<String>) -> ExtractedWrapperFlags {
    let mut extracted = ExtractedWrapperFlags::default();
    let mut skip_next = false;
    let mut remove_indices = Vec::new();

    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        match arg.as_str() {
            "-y" | "--yolo" => {
                extracted.yolo = true;
                remove_indices.push(i);
            }
            "-n" | "--non-interactive" | "--ni" => {
                extracted.non_interactive = true;
                remove_indices.push(i);
            }
            "--repo" => {
                extracted.repo = true;
                remove_indices.push(i);
            }
            "-q" | "--quiet" => {
                extracted.quiet = true;
                remove_indices.push(i);
            }
            "--silent" => {
                extracted.silent = true;
                remove_indices.push(i);
            }
            "--operation" | "--op" => {
                if let Some(value) = args.get(i + 1) {
                    extracted.operation = Some(value.clone());
                    remove_indices.push(i);
                    remove_indices.push(i + 1);
                    skip_next = true;
                }
            }
            _ => {
                if let Some(value) = arg.strip_prefix("--operation=") {
                    extracted.operation = Some(value.to_string());
                    remove_indices.push(i);
                } else if let Some(value) = arg.strip_prefix("--op=") {
                    extracted.operation = Some(value.to_string());
                    remove_indices.push(i);
                }
            }
        }
    }

    // Remove in reverse order to preserve indices
    for i in remove_indices.into_iter().rev() {
        args.remove(i);
    }

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

    use chrono::Utc;
    use claudine::mcp::session::lex_tags;
    use claudine::mcp::types::{McpServer, McpServerMetadata, McpTransport};

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
            shadow_home_path: None,
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
            shadow_home_path: None,
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
    fn extract_wrapper_flags_lifts_operation_from_passthrough() {
        let mut args = vec![
            "do something".to_string(),
            "--op".to_string(),
            "commit".to_string(),
        ];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args);

        assert_eq!(extracted.operation.as_deref(), Some("commit"));
        assert_eq!(args, vec!["do something"]);
    }

    #[test]
    fn extract_wrapper_flags_lifts_operation_equals_form() {
        let mut args = vec![
            "do something".to_string(),
            "--operation=deploy".to_string(),
        ];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args);

        assert_eq!(extracted.operation.as_deref(), Some("deploy"));
        assert_eq!(args, vec!["do something"]);
    }

    #[test]
    fn extract_wrapper_flags_lifts_op_equals_form() {
        let mut args = vec![
            "do something".to_string(),
            "--op=review".to_string(),
        ];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args);

        assert_eq!(extracted.operation.as_deref(), Some("review"));
        assert_eq!(args, vec!["do something"]);
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

    fn make_catalog_with_servers(names: &[&str]) -> Vec<McpServer> {
        let mut servers = Vec::new();
        for name in names {
            servers.push(McpServer {
                id: (*name).to_string(),
                aliases: Vec::new(),
                transport: McpTransport::Stdio,
                command: Some("npx".into()),
                args: vec!["-y".into(), format!("@test/{name}")],
                cwd: None,
                env: HashMap::new(),
                url: None,
                headers: HashMap::new(),
                enabled_tools: Vec::new(),
                disabled_tools: Vec::new(),
                required: false,
                metadata: McpServerMetadata {
                    description: None,
                    created_from: None,
                    fingerprint: String::new(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
                provider_overrides: HashMap::new(),
            });
        }
        servers
    }

    #[test]
    fn extracts_tags_from_codex_prompt_position() {
        let _ = make_catalog_with_servers(&["calendar"]);
        let mut args = vec![
            "exec".to_string(),
            "--json".to_string(),
            "fix #calendar bugs".to_string(),
        ];

        let (cleaned, tags) = extract_tags_from_child_args(Provider::Codex, &mut args, lex_tags);

        assert_eq!(tags, vec!["calendar"]);
        assert_eq!(cleaned.as_deref(), Some("fix bugs"));
        assert_eq!(args[2], "fix bugs");
    }

    #[test]
    fn extracts_tags_from_gemini_prompt_flag() {
        let _ = make_catalog_with_servers(&["slack"]);
        let mut args = vec!["--prompt".to_string(), "debug #slack auth".to_string()];

        let (cleaned, tags) = extract_tags_from_child_args(Provider::Gemini, &mut args, lex_tags);

        assert_eq!(tags, vec!["slack"]);
        assert_eq!(cleaned.as_deref(), Some("debug auth"));
        assert_eq!(args[1], "debug auth");
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
                flags in prop::collection::vec("-y|--yolo|-n|--non-interactive|--ni|-q|--quiet|--silent", 0..5),
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
