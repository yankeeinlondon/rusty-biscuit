pub(crate) mod env;
pub(crate) mod exec;
pub(crate) mod profile;
pub(crate) mod prompt_file;
pub(crate) mod repo_home;

use biscuit_terminal::terminal::Terminal;
use clap::Args;
use claudine::events::{
    AgenticEvent, EnvironmentContext, EventMeta as DispatchEventMeta, Provider,
};
use claudine::stream::parser::{EventMeta as StreamEventMeta, StreamEventSink};
use claudine::stream::stderr::{Verbosity, format_warning};
use color_eyre::eyre::{Result, eyre};
use inquire::Select;
use profile::{OutputFormat, WrapperProfile};
use sniff::programs::InstalledAiClients;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

struct StructuredCodexOutput {
    last_message_path: PathBuf,
}

impl StructuredCodexOutput {
    fn prepare(args: &mut Vec<String>) -> Self {
        let path = std::env::temp_dir().join(format!(
            "claudine-codex-last-message-{}.txt",
            uuid::Uuid::new_v4()
        ));
        args.push("--output-last-message".to_string());
        args.push(path.to_string_lossy().into_owned());
        Self {
            last_message_path: path,
        }
    }

    fn apply_to_summary(&self, summary: &mut claudine::stream::summary::StreamExecutionSummary) {
        if let Ok(text) = fs::read_to_string(&self.last_message_path)
            && !text.trim().is_empty()
        {
            summary.assistant_text = text;
        }
        let _ = fs::remove_file(&self.last_message_path);
    }
}

type StreamDispatchFn = Box<dyn Fn(AgenticEvent, DispatchEventMeta) + Send + Sync + 'static>;

#[derive(Debug, Clone, Default)]
struct StructuredSummaryDetails {
    tool_names: Vec<String>,
}

impl StructuredSummaryDetails {
    fn record_tool_name(&mut self, tool_name: &str) {
        if !tool_name.is_empty() && !self.tool_names.iter().any(|name| name == tool_name) {
            self.tool_names.push(tool_name.to_string());
        }
    }
}

struct LiveStreamSink {
    provider: Provider,
    env: EnvironmentContext,
    verbosity: Verbosity,
    session_id: Option<String>,
    model: Option<String>,
    start_emitted: bool,
    summary_details: Arc<Mutex<StructuredSummaryDetails>>,
    dispatch: StreamDispatchFn,
}

impl LiveStreamSink {
    fn new(
        provider: Provider,
        env: EnvironmentContext,
        verbosity: Verbosity,
        summary_details: Arc<Mutex<StructuredSummaryDetails>>,
    ) -> Self {
        let handle = tokio::runtime::Handle::try_current().ok();
        Self::with_dispatcher(
            provider,
            env,
            verbosity,
            summary_details,
            move |event, meta| {
                if let Some(handle) = handle.as_ref()
                    && let Err(error) = handle.block_on(claudine::dispatch::dispatch_event_meta(
                        provider, event, meta,
                    ))
                {
                    tracing::warn!(%provider, %event, "live stream dispatch failed: {error}");
                }
            },
        )
    }

    fn with_dispatcher<F>(
        provider: Provider,
        env: EnvironmentContext,
        verbosity: Verbosity,
        summary_details: Arc<Mutex<StructuredSummaryDetails>>,
        dispatch: F,
    ) -> Self
    where
        F: Fn(AgenticEvent, DispatchEventMeta) + Send + Sync + 'static,
    {
        Self {
            provider,
            env,
            verbosity,
            session_id: None,
            model: None,
            start_emitted: false,
            summary_details,
            dispatch: Box::new(dispatch),
        }
    }

    fn merge_state(&mut self, meta: &StreamEventMeta) {
        if let Some(session_id) = string_from_extra(&meta.extra, &["session_id", "thread_id", "id"])
        {
            self.session_id = Some(session_id);
        }
        if let Some(model) = string_from_extra(&meta.extra, &["model"]) {
            self.model = Some(model);
        }
    }

    fn emit_start_summary(&mut self) {
        if self.start_emitted || self.verbosity != Verbosity::Normal {
            return;
        }

        if let Some(ref session_id) = self.session_id {
            let line = crate::output::format_session_start(
                self.provider,
                session_id,
                self.model.as_deref(),
            );
            eprintln!("{line}\n"); // blank line after session ID separates from execution output
            self.start_emitted = true;
        }
    }

    fn emit_warning_line(&self, message: &str) {
        if self.verbosity != Verbosity::Silent {
            eprintln!("{}", format_warning(message));
        }
    }

    fn should_surface_warning(message: &str) -> bool {
        !message.starts_with("Malformed JSON on line ")
    }

    fn build_meta(&mut self, event: AgenticEvent, meta: &StreamEventMeta) -> DispatchEventMeta {
        self.merge_state(meta);

        let mut extra = meta.extra.clone();
        extra
            .entry("stream_wrapper".into())
            .or_insert_with(|| serde_json::Value::Bool(true));
        if let Some(model) = &self.model {
            extra
                .entry("model".into())
                .or_insert_with(|| serde_json::Value::String(model.clone()));
        }

        DispatchEventMeta {
            provider: self.provider,
            event,
            timestamp: chrono::Utc::now(),
            session_id: string_from_extra(&meta.extra, &["session_id", "thread_id", "id"])
                .or_else(|| self.session_id.clone()),
            cwd: std::env::current_dir()
                .ok()
                .map(|cwd| cwd.display().to_string()),
            tool_name: string_from_extra(&meta.extra, &["tool_name", "name"]),
            tool_input: value_from_extra(
                &meta.extra,
                &["tool_input", "parameters", "input", "arguments"],
            ),
            tool_response: value_from_extra(
                &meta.extra,
                &["tool_response", "output", "result", "content"],
            ),
            error: string_from_extra(&meta.extra, &["error_message", "message"]).or_else(|| {
                value_from_extra(&meta.extra, &["error"]).and_then(|value| value_to_string(&value))
            }),
            prompt: string_from_extra(&meta.extra, &["prompt"]),
            agent_type: string_from_extra(&meta.extra, &["agent_type"]),
            notification_type: string_from_extra(&meta.extra, &["notification_type"]),
            notification_message: string_from_extra(
                &meta.extra,
                &["notification_message", "message"],
            ),
            extra,
            env: self.env.clone(),
        }
    }

    fn dispatch_event(&mut self, event: AgenticEvent, meta: &StreamEventMeta) {
        let dispatch_meta = self.build_meta(event, meta);
        if event == AgenticEvent::BeforeTool
            && let Some(tool_name) = dispatch_meta.tool_name.as_deref()
            && let Ok(mut details) = self.summary_details.lock()
        {
            details.record_tool_name(tool_name);
        }
        if event == AgenticEvent::SessionStart {
            self.emit_start_summary();
        }
        if event == AgenticEvent::TurnError
            && let Some(message) = dispatch_meta.error.as_deref()
        {
            self.emit_warning_line(message);
        }
        (self.dispatch)(event, dispatch_meta);
    }
}

impl StreamEventSink for LiveStreamSink {
    fn on_session_start(&mut self, meta: &StreamEventMeta) {
        self.dispatch_event(AgenticEvent::SessionStart, meta);
    }

    fn on_turn_start(&mut self, meta: &StreamEventMeta) {
        self.dispatch_event(AgenticEvent::BeforePrompt, meta);
    }

    fn on_turn_complete(&mut self, meta: &StreamEventMeta) {
        self.dispatch_event(AgenticEvent::TurnComplete, meta);
    }

    fn on_turn_error(&mut self, meta: &StreamEventMeta) {
        self.dispatch_event(AgenticEvent::TurnError, meta);
    }

    fn on_before_tool(&mut self, meta: &StreamEventMeta) {
        self.dispatch_event(AgenticEvent::BeforeTool, meta);
    }

    fn on_after_tool(&mut self, meta: &StreamEventMeta) {
        self.dispatch_event(AgenticEvent::AfterTool, meta);
    }

    fn on_permission_request(&mut self, meta: &StreamEventMeta) {
        self.dispatch_event(AgenticEvent::PermissionRequest, meta);
    }

    fn on_warning(&mut self, message: &str) {
        if Self::should_surface_warning(message) {
            self.emit_warning_line(message);
        } else {
            tracing::debug!("suppressing malformed structured stream warning: {message}");
        }
    }
}

fn string_from_extra(
    extra: &std::collections::HashMap<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter().find_map(|key| {
        extra.get(*key).and_then(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .or_else(|| value_to_string(value))
        })
    })
}

fn value_from_extra(
    extra: &std::collections::HashMap<String, serde_json::Value>,
    keys: &[&str],
) -> Option<serde_json::Value> {
    keys.iter().find_map(|key| extra.get(*key).cloned())
}

fn value_to_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| serde_json::to_string(value).ok())
}

fn structured_verbosity(silent: bool, quiet: bool) -> Verbosity {
    if silent {
        Verbosity::Silent
    } else if quiet {
        Verbosity::Quiet
    } else {
        Verbosity::Normal
    }
}

fn optimistic_terminal() -> Terminal {
    let width = std::env::var("TERM_WIDTH")
        .ok()
        .or_else(|| std::env::var("COLUMNS").ok())
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|width| *width > 0);
    crate::log::optimistic_terminal(width)
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn has_explicit_native_output_request(provider: Provider, args: &[String]) -> bool {
    match provider {
        Provider::Codex => has_flag(args, "--json"),
        Provider::Claude | Provider::Gemini | Provider::KimiCode | Provider::QwenCode => {
            has_flag(args, "--output-format")
                || args.iter().any(|arg| arg.starts_with("--output-format="))
        }
        Provider::OpenCode => args.iter().any(|arg| arg == "json"),
        _ => false,
    }
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

    /// Force interactive mode even when a prompt string is provided.
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,

    /// Override the model used by the provider.
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Set the output format (json, text, stream).
    #[arg(short = 'o', long = "output", value_name = "FORMAT")]
    pub output: Option<String>,

    /// Set or append a system prompt (string or file path).
    #[arg(short = 's', long = "system-prompt", value_name = "PROMPT|FILE")]
    pub system_prompt: Option<String>,

    /// Timeout in seconds (sends SIGTERM then SIGKILL). Only valid in non-interactive mode.
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
pub fn run_provider_wrapper(provider: Provider, args: WrapperArgs, verbose: u8) -> Result<()> {
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
    let env_context = claudine::events::detect_environment_fast(&cwd);
    let term = optimistic_terminal();

    let clients = InstalledAiClients::new();
    let binary_path = resolve_binary_path(profile, &clients)?;

    let raw_agent_params: Vec<String> = std::env::args().skip(2).collect();
    let mut child_args = args.passthrough.clone();
    let extracted = extract_wrapper_flags_from_passthrough(&mut child_args);
    let yolo_requested = args.yolo || extracted.yolo;
    let mut yolo_enabled = yolo_requested;
    let interactive_requested = args.interactive || extracted.interactive;
    let repo_requested = args.repo || extracted.repo;
    let quiet_requested = args.quiet || extracted.quiet;
    let silent_requested = args.silent || extracted.silent;
    let verbose_requested = verbose > 0 || extracted.verbose;
    let mut env_overrides: Vec<(String, String)> = Vec::new();
    let mut deferred_warnings: Vec<String> = Vec::new();
    let mut deferred_messages: Vec<String> = Vec::new();

    // Determine if a prompt is present (implies non-interactive by default)
    let has_prompt = has_prompt_source(&args, &child_args, None);

    // Default: interactive when no prompt, non-interactive when prompt present
    // --interactive/-i overrides the default back to interactive
    let non_interactive_requested = if interactive_requested {
        false
    } else {
        has_prompt
    };

    // Early check: --timeout + --interactive is always an error
    if args.timeout.is_some() && interactive_requested {
        return Err(eyre!("--timeout cannot be used with --interactive mode"));
    }

    // Early check: --timeout requires non-interactive mode
    if args.timeout.is_some() && !non_interactive_requested {
        return Err(eyre!(
            "--timeout can only be used in non-interactive mode \
             (provide a prompt or use a composition switch)"
        ));
    }

    profile.reject_direct_yolo(&child_args)?;

    if yolo_requested && let Some(warn) = profile.apply_yolo(&mut child_args, &mut env_overrides)? {
        deferred_warnings.push(warn);
    }
    if yolo_requested && !profile.has_supported_yolo() {
        yolo_enabled = false;
    }

    // Composition pipelines (--prompt-file, --frontmatter-prompt, --compose)
    // deliver the prompt to child_args themselves, then call apply_non_interactive.
    // Only run the early call for passthrough prompts (bare positional args).
    let has_composition_prompt = args.prompt_file.is_some()
        || args.frontmatter_prompt.is_some()
        || args.compose.is_some();

    if non_interactive_requested && !has_composition_prompt {
        profile.apply_non_interactive(&mut child_args)?;
        // Only apply default model if the user didn't pass --model explicitly
        // (apply_model handles it below when args.model is Some).
        if args.model.is_none() {
            profile.apply_non_interactive_defaults(&mut child_args);
        }
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
            package_root: env_plan.package_context.as_ref().and_then(|pc| {
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
        let delivery_method = if matches!(provider, Provider::Claude | Provider::KimiCode)
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
            !interactive_requested, // non-interactive unless --interactive
        )?;

        // Force non-interactive for prompt-file composition (unless --interactive).
        // The early apply_non_interactive block is skipped for composition pipelines,
        // so this is the only call site when a composition switch is used.
        if !interactive_requested {
            profile.apply_non_interactive(&mut child_args)?;
            profile.apply_non_interactive_defaults(&mut child_args);
        }

        // Add prompt-file env vars to child environment
        for (key, value) in &composed.env_overrides {
            env_plan
                .env
                .insert(key.clone().into(), value.clone().into());
            env_plan.added.push((key.clone(), value.clone()));
        }

        prompt_file_dry_run = Some(prompt_file::PromptFileDryRunInfo {
            original: resolved.original.clone(),
            resolved_path: composed.resolved_path.clone(),
            delivery_method: delivery_method.to_string(),
            env_names: composed.env_names.clone(),
            body: composed.body.clone(),
        });
    }

    // -- Frontmatter-prompt (inline composition) pipeline --------------------
    // Tuple: (source, prepared_prompt, pre_fm_hash, pre_body_hash)
    let mut inline_composition_source: Option<(
        claudine::composition::ResolvedCompositionSource,
        claudine::composition::PreparedPrompt,
        u64,
        u64,
    )> = None;

    if let Some(ref fp_input) = args.frontmatter_prompt {
        let source = claudine::composition::resolve_composition_source(fp_input)
            .map_err(|e| eyre!("frontmatter-prompt: {e}"))?;

        // Validate file read/write permissions before proceeding
        // (success message is deferred to the reporting section below)
        if let Err(e) = claudine::composition::validate_file_permissions(&source.resolved_path) {
            log::message(&crate::output::fm_check_fail(
                "the agent does not have read and write permissions required to finish the task",
                &term,
            ));
            return Err(eyre!("frontmatter-prompt: {e}"));
        }

        let prepared =
            claudine::composition::prepare_inline_prompt(&source, env_plan.repo_root.as_deref())
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
            !interactive_requested, // non-interactive unless --interactive
        )?;

        // Force non-interactive for inline composition (unless --interactive).
        // The early apply_non_interactive block is skipped for composition pipelines,
        // so this is the only call site when a composition switch is used.
        if !interactive_requested {
            profile.apply_non_interactive(&mut child_args)?;
            profile.apply_non_interactive_defaults(&mut child_args);
        }

        // Capture pre-execution hashes for post-run validation
        let pre_fm_hash = source.markdown.hash_frontmatter(false);
        let pre_body_hash = source.markdown.hash_body(false);

        inline_composition_source = Some((source, prepared, pre_fm_hash, pre_body_hash));
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
            !interactive_requested, // non-interactive unless --interactive
        )?;

        // Force non-interactive for chained composition (unless --interactive).
        // The early apply_non_interactive block is skipped for composition pipelines,
        // so this is the only call site when a composition switch is used.
        if !interactive_requested {
            profile.apply_non_interactive(&mut child_args)?;
            profile.apply_non_interactive_defaults(&mut child_args);
        }

        chained_composition = true;
    }

    // -- Final argument validation -------------------------------------------
    // All prompt sources (passthrough, --prompt-file, --frontmatter-prompt,
    // --compose) have now been processed. Validate that providers requiring a
    // positional prompt actually have one.
    let effective_non_interactive = if interactive_requested {
        false
    } else {
        non_interactive_requested
            || prompt_file_dry_run.is_some()
            || inline_composition_source.is_some()
            || chained_composition
    };
    profile.validate_final_args(&child_args, effective_non_interactive, stdin_seed.is_some())?;

    // Late --timeout validation: composition may have changed interactivity
    if args.timeout.is_some() && !effective_non_interactive {
        return Err(eyre!(
            "--timeout can only be used in non-interactive mode \
             (provide a prompt or use a composition switch)"
        ));
    }

    // If a composition pipeline inferred non-interactive mode, update the
    // INTERACTIVE env var that was set before the pipelines ran.
    if effective_non_interactive && !non_interactive_requested {
        env_plan.env.insert("INTERACTIVE".into(), "false".into());
    }

    let mut mcp_runtime = None;
    let mut mcp_cleanup: Option<(
        Box<dyn claudine::mcp::inject::McpInjector>,
        claudine::mcp::inject::InjectionResult,
    )> = None;

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
        let prompt_is_interactive =
            std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
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
    } else if prompt_file_dry_run.is_some() {
        Some(crate::output::ComposeDisplay::PromptFile)
    } else {
        None
    };

    // Extract the user's prompt for display in the header line.
    // Both prompt-file and frontmatter-prompt show the actual prompt text;
    // other modes show the file path; regular runs show the prompt text.
    let prompt_display: Option<String> = if let Some(ref pf_info) = prompt_file_dry_run {
        Some(pf_info.body.clone())
    } else if inline_composition_source.is_some() {
        inline_composition_source
            .as_ref()
            .map(|(_, prepared, _, _)| prepared.prompt.clone())
    } else if let Some(ref c) = args.compose {
        Some(format!("--compose {c}"))
    } else {
        extract_user_prompt(&args.passthrough)
    };

    // Interactive override: user explicitly forced -i with a prompt present
    let interactive_override = interactive_requested && has_prompt;

    // Output verbosity: --silent suppresses everything, --quiet shows header only
    if !silent_requested {
        // Header line (shown for both default and --quiet)
        crate::output::log_wrapper_header(
            profile,
            yolo_enabled,
            effective_non_interactive,
            interactive_override,
            verbose_requested,
            repo_requested,
            compose_display.as_ref(),
            effective_operation.as_deref(),
            prompt_display.as_deref(),
            &env_plan,
            &term,
        );

        // Everything below is suppressed by --quiet
        if !quiet_requested {
            crate::output::log_wrapper_env_details(&env_plan, mcp_runtime.as_ref(), &term, verbose);

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

            // Prompt-file: show file resolution and prompt blockquote.
            if let Some(ref pf_info) = prompt_file_dry_run {
                let display_path = pf_info
                    .resolved_path
                    .strip_prefix(child_cwd)
                    .unwrap_or(&pf_info.resolved_path);
                log::message(""); // blank line separating Info section from validation
                log::message(&crate::output::fm_check_ok(
                    &format!(
                        "resolved the file reference to <a href=\"{}\">{}</a>",
                        pf_info.resolved_path.display(),
                        display_path.display()
                    ),
                    &term,
                ));
                log::message("");
                crate::output::render_prompt_blockquote(
                    &pf_info.body,
                    &term,
                    verbose_requested,
                );
            }

            // Frontmatter-prompt: show validation, file resolution, and prompt blockquote.
            // These are grouped together after the Info/warning lines with a blank
            // line separator so the output sections are visually distinct.
            if let Some(ref ics) = inline_composition_source {
                let (source, prepared, _, _) = ics;
                let display_path = source
                    .resolved_path
                    .strip_prefix(child_cwd)
                    .unwrap_or(&source.resolved_path);
                log::message(""); // blank line separating Info section from validation
                log::message(&crate::output::fm_check_ok(
                    "validated that agent has read and write permissions to the referenced file",
                    &term,
                ));
                log::message(&crate::output::fm_check_ok(
                    &format!(
                        "resolved the file reference to <a href=\"{}\">{}</a>",
                        source.resolved_path.display(),
                        display_path.display()
                    ),
                    &term,
                ));
                crate::output::render_prompt_blockquote(
                    &prepared.prompt,
                    &term,
                    verbose_requested,
                );
            }

            // Blank line to separate preamble from execution output
            log::message("");
        }
    }

    let stdout_noise = if effective_non_interactive {
        profile.stdout_noise_prefixes()
    } else {
        &[]
    };
    let stderr_noise = profile.stderr_noise_prefixes();

    // Decide whether to use internal structured stream parsing.
    // Conditions: provider supports it, non-interactive, no explicit output format.
    let use_structured = profile.supports_structured_stream()
        && effective_non_interactive
        && args.output.is_none()
        && !has_explicit_native_output_request(provider, &child_args);
    let stream_verbosity = structured_verbosity(silent_requested, quiet_requested);

    if use_structured {
        profile.apply_structured_stream(&mut child_args);
    }

    let structured_codex_output = if use_structured && provider == Provider::Codex {
        Some(StructuredCodexOutput::prepare(&mut child_args))
    } else {
        None
    };

    let exit_code = if let Some((source, _prepared, pre_fm_hash, pre_body_hash)) =
        inline_composition_source
    {
        // Inline composition: the agent is responsible for writing to the file.
        // Capture structured assistant text so we can render it with terminal-aware
        // wrapping before running validation checks and emitting the metadata line.
        let (agent_exit, deferred_summary) = if use_structured {
            let summary_details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
            let parser_config = claudine::stream::ParserConfig {
                model: args.model.clone(),
            };
            let parser = claudine::stream::create_parser(
                provider,
                LiveStreamSink::new(
                    provider,
                    env_context.clone(),
                    stream_verbosity,
                    summary_details.clone(),
                ),
                parser_config,
            );
            let mut summary = exec::run_child_stream(
                binary_path.as_path(),
                &child_args,
                &env_plan.env,
                child_cwd,
                args.timeout,
                stderr_noise,
                profile.suppress_structured_stderr_on_success(),
                stdin_seed.as_deref(),
                parser,
            )?;
            // Codex never emits text via feed_line; its accumulated text is
            // fallback-only, so it doesn't count as "streamed to stdout".
            let had_streamed_assistant = provider != Provider::Codex
                && !summary.assistant_text.trim().is_empty();
            if let Some(codex_output) = structured_codex_output.as_ref() {
                codex_output.apply_to_summary(&mut summary);
            }
            if !had_streamed_assistant && !summary.assistant_text.trim().is_empty() {
                let text = &summary.assistant_text;
                if std::io::stdout().is_terminal() {
                    let rendered = crate::output::render_assistant_markdown(text, &term);
                    std::io::stdout().write_all(rendered.as_bytes())?;
                    if !rendered.ends_with('\n') {
                        std::io::stdout().write_all(b"\n")?;
                    }
                } else {
                    std::io::stdout().write_all(text.as_bytes())?;
                    if !text.ends_with('\n') {
                        std::io::stdout().write_all(b"\n")?;
                    }
                }
                std::io::stdout().flush()?;
            }

            // Warn if agent provided no summary text to stdout
            if summary.exit_code == 0 && summary.assistant_text.trim().is_empty() {
                log::warn(
                    "the agent did not provide a summarized message on their completed work!",
                );
            }

            let exit = summary.exit_code;
            let details = summary_details.lock().unwrap().clone();
            (exit, Some((summary, details, had_streamed_assistant)))
        } else {
            // Legacy path: forward I/O to terminal
            let exit = exec::run_child(
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
            (exit, None)
        };

        // Post-execution validation: always check the file, even on agent error.
        // The agent may have successfully updated the file before an API error occurred.
        let mut final_exit = agent_exit;
        let show_checks = !silent_requested && !quiet_requested;
        let provider_name = crate::output::capitalize_provider(provider);
        let should_separate_checks = deferred_summary
            .as_ref()
            .is_some_and(|(summary, _, _)| !summary.assistant_text.trim().is_empty());

        if show_checks && should_separate_checks {
            eprintln!();
            eprintln!();
        }

        if agent_exit == 0 && show_checks {
            log::message(&crate::output::fm_check_ok(
                &format!("{provider_name} agent completed successfully"),
                &term,
            ));
        } else if agent_exit != 0 && show_checks {
            log::message(&crate::output::fm_check_fail(
                &format!("{provider_name} agent exited with error (code {agent_exit})"),
                &term,
            ));
        }

        let display_path = source
            .resolved_path
            .strip_prefix(child_cwd)
            .unwrap_or(&source.resolved_path)
            .display();

        // Read the file from disk to see what the agent did
        match fs::read_to_string(source.resolved_path.as_path()) {
            Ok(disk_text) => {
                let on_disk: darkmatter::markdown::Markdown = disk_text.clone().into();
                // Check if the body was updated on disk
                let disk_body_hash = on_disk.hash_body(false);
                let body_updated = disk_body_hash != pre_body_hash;

                if body_updated {
                    if show_checks {
                        log::message(&crate::output::fm_check_ok(
                            "Agent updated the target document's body",
                            &term,
                        ));
                    }

                    // Agent updated the file — if it also exited with an error,
                    // the file update takes precedence (e.g. API error after writing).
                    if agent_exit != 0 && show_checks {
                        log::warn(
                            "agent reported an error but the target file was updated; \
                             treating as success",
                        );
                    }
                    final_exit = 0;

                    // Only check frontmatter when the agent actually did work
                    let disk_fm_hash = on_disk.hash_frontmatter(false);
                    let fm_tampered = disk_fm_hash != pre_fm_hash;
                    if fm_tampered {
                        if show_checks {
                            log::message(&crate::output::fm_check_fail(
                                "Agent ignored instruction to leave frontmatter untouched \
                                 (<i>we have reverted their changes</i>)",
                                &term,
                            ));
                        }
                        // Restore original frontmatter but keep the agent's body content
                        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                        let doc_string = rewrite_markdown_preserving_frontmatter(
                            &source.original_text,
                            on_disk.content(),
                            &today,
                        )
                        .map_err(|e| eyre!("failed to restore frontmatter: {e}"))?;
                        claudine::config::atomic::atomic_write(
                            &source.resolved_path,
                            doc_string.as_bytes(),
                        )
                        .map_err(|e| eyre!("failed to write restored frontmatter: {e}"))?;
                    } else {
                        if show_checks {
                            log::message(&crate::output::fm_check_ok(
                                "Agent left frontmatter untouched (<i>as instructed</i>)",
                                &term,
                            ));
                        }
                        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                        let doc_string = rewrite_markdown_preserving_frontmatter(
                            &disk_text,
                            on_disk.content(),
                            &today,
                        )
                        .map_err(|e| eyre!("failed to update last_updated: {e}"))?;
                        claudine::config::atomic::atomic_write(
                            &source.resolved_path,
                            doc_string.as_bytes(),
                        )
                        .map_err(|e| eyre!("failed to write last_updated: {e}"))?;
                    }

                    if show_checks {
                        log::message(&crate::output::fm_check_ok(
                            "Updated <bold>last_updated</bold> property to today's date",
                            &term,
                        ));
                    }
                } else if agent_exit == 0 {
                    // Agent reported success but didn't update the file
                    if show_checks {
                        log::message(&crate::output::fm_check_fail(
                            &format!(
                                "the referenced file -- {display_path} -- did not get \
                                 updated even though the Agent reported a successful outcome!"
                            ),
                            &term,
                        ));
                    }
                    final_exit = 1;
                }
                // If agent errored AND body wasn't updated, no further checks —
                // the agent failed before completing its work.
            }
            Err(e) => {
                log::error(&format!(
                    "failed to read {display_path} after agent completion: {e}"
                ));
                final_exit = 1;
            }
        }

        // Emit the metadata summary line last, after validation checks.
        // Blank line before metadata to visually separate from checks.
        if let Some((summary, details, _)) = deferred_summary {
            if stream_verbosity != Verbosity::Silent {
                eprintln!();
            }
            emit_stream_summary_no_separator(
                &summary,
                profile,
                &env_context,
                stream_verbosity,
                verbose_requested,
                &details,
            );
        }

        Ok(final_exit)
    } else if use_structured {
        let summary_details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
        // Normal execution with structured stream parsing
        let parser_config = claudine::stream::ParserConfig {
            model: args.model.clone(),
        };
        let parser = claudine::stream::create_parser(
            provider,
            LiveStreamSink::new(
                provider,
                env_context.clone(),
                stream_verbosity,
                summary_details.clone(),
            ),
            parser_config,
        );
        let mut summary = exec::run_child_stream(
            binary_path.as_path(),
            &child_args,
            &env_plan.env,
            child_cwd,
            args.timeout,
            stderr_noise,
            profile.suppress_structured_stderr_on_success(),
            stdin_seed.as_deref(),
            parser,
        )?;
        if let Some(codex_output) = structured_codex_output.as_ref() {
            codex_output.apply_to_summary(&mut summary);
        }
        // Codex never emits assistant text live (feed_line always returns None);
        // the authoritative text comes from --output-last-message. Write it now.
        if provider == Provider::Codex && !summary.assistant_text.is_empty() {
            let text = &summary.assistant_text;
            if std::io::stdout().is_terminal() {
                let rendered = crate::output::render_assistant_markdown(text, &term);
                std::io::stdout().write_all(rendered.as_bytes())?;
                if !rendered.ends_with('\n') {
                    std::io::stdout().write_all(b"\n")?;
                }
            } else {
                std::io::stdout().write_all(text.as_bytes())?;
                if !text.ends_with('\n') {
                    std::io::stdout().write_all(b"\n")?;
                }
            }
            std::io::stdout().flush()?;
        }

        // Emit stderr summaries and write synthetic event
        emit_stream_summary(
            &summary,
            profile,
            &env_context,
            stream_verbosity,
            verbose_requested,
            &summary_details.lock().unwrap().clone(),
        );

        Ok(summary.exit_code)
    } else {
        // Normal execution: forward I/O to terminal (legacy path)
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

/// Emit stderr summaries and write synthetic JSONL event after a structured stream session.
fn emit_stream_summary(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    details: &StructuredSummaryDetails,
) {
    let primary_markup = if verbosity == Verbosity::Silent {
        None
    } else {
        format_summary_prose(summary)
    };
    let secondary_markup = if verbosity == Verbosity::Silent || !verbose {
        None
    } else {
        format_verbose_summary_details_prose(summary, details)
    };
    if primary_markup.is_some() || secondary_markup.is_some() {
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::renderable::Renderable;

        let separator = if summary.assistant_text.is_empty() {
            ""
        } else if summary.assistant_text.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        eprint!("{separator}");
        if let Some(markup) = primary_markup {
            let term = crate::log::optimistic_terminal(None);
            let rendered = Prose::new(markup).render(&term);
            eprint!("{rendered}\n");
        }
        if let Some(markup) = secondary_markup {
            let term = crate::log::optimistic_terminal(None);
            let rendered = Prose::new(markup).render(&term);
            eprint!("  {rendered}\n");
        }
    }

    // Write synthetic summary event to JSONL (best-effort)
    if let Some(protocol) = profile.stream_protocol() {
        let meta =
            claudine::stream::reporting::summary_to_event_meta(summary, protocol, env_context);
        if let Err(e) = claudine::stream::reporting::write_summary_event(&meta) {
            tracing::warn!("Failed to write stream summary event: {e}");
        }
    }
}

/// Like `emit_stream_summary` but without the automatic separator logic.
/// Used when the caller manages spacing (e.g. inline composition validation output).
fn emit_stream_summary_no_separator(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    details: &StructuredSummaryDetails,
) {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::Renderable;

    if verbosity != Verbosity::Silent {
        if let Some(markup) = format_summary_prose(summary) {
            let term = crate::log::optimistic_terminal(None);
            let rendered = Prose::new(markup).render(&term);
            eprintln!("{rendered}");
        }
    }
    if verbosity != Verbosity::Silent && verbose {
        if let Some(markup) = format_verbose_summary_details_prose(summary, details) {
            let term = crate::log::optimistic_terminal(None);
            let rendered = Prose::new(markup).render(&term);
            eprintln!("  {rendered}");
        }
    }

    // Write synthetic summary event to JSONL (best-effort)
    if let Some(protocol) = profile.stream_protocol() {
        let meta =
            claudine::stream::reporting::summary_to_event_meta(summary, protocol, env_context);
        if let Err(e) = claudine::stream::reporting::write_summary_event(&meta) {
            tracing::warn!("Failed to write stream summary event: {e}");
        }
    }
}

fn format_summary_prose(
    summary: &claudine::stream::summary::StreamExecutionSummary,
) -> Option<String> {
    use claudine::stream::stderr::{format_cost, format_duration, format_number};

    let prefix = if summary.is_error {
        "\u{2717}"
    } else {
        "\u{2713}"
    };
    let mut parts = Vec::new();

    if let Some(ms) = summary.duration_ms {
        parts.push(format_duration(ms));
    }

    if let Some(usage) = &summary.token_usage {
        if let Some(input) = usage.input {
            parts.push(format!("{} <i>input tokens</i>", format_number(input)));
        }
        if let Some(output) = usage.output {
            parts.push(format!("{} <i>output tokens</i>", format_number(output)));
        }
        if let Some(cache) = usage.cache_read
            && cache > 0
        {
            parts.push(format!("{} <i>cached tokens</i>", format_number(cache)));
        }
    }

    if let Some(cost) = summary.cost_usd {
        parts.push(format!("{} <i>cost basis</i>", format_cost(cost)));
    }

    match summary.tool_calls {
        Some(tc) => parts.push(format!(
            "{tc} <i>tool call{}</i>",
            if tc == 1 { "" } else { "s" }
        )),
        None => parts.push("<i>no tool calls</i>".to_string()),
    }

    if parts.is_empty() {
        return None;
    }

    Some(format!("<dim>{prefix} {}</dim>", parts.join(" \u{00b7} ")))
}

fn format_verbose_summary_details_prose(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    details: &StructuredSummaryDetails,
) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(sid) = &summary.session_id {
        parts.push(format!("<i>session</i>: {sid}"));
    }

    if !details.tool_names.is_empty() {
        parts.push(format!(
            "<i>tools used</i>: {}",
            details.tool_names.join(", ")
        ));
    }

    if let Some(model) = &summary.model {
        parts.push(format!("<i>model</i>: {model}"));
    }

    if let Some(turns) = summary.num_turns {
        parts.push(format!("<i>turns</i>: {turns}",));
    }

    if let Some(stop_reason) = &summary.provider_status {
        parts.push(format!("<i>stop reason</i>: {stop_reason}"));
    }

    if summary.is_error
        && let Some(msg) = &summary.error_message
    {
        parts.push(format!("<red>{msg}</red>"));
    }

    if parts.is_empty() {
        return None;
    }

    Some(format!("<dim>{}</dim>", parts.join(" \u{00b7} ")))
}

fn rewrite_markdown_preserving_frontmatter(
    frontmatter_source: &str,
    body: &str,
    today: &str,
) -> Result<String> {
    if let Some(parts) = split_frontmatter_parts(frontmatter_source) {
        let newline = detect_newline(frontmatter_source);
        let yaml = upsert_last_updated_in_frontmatter(parts.yaml, today, newline);
        let mut document = String::with_capacity(
            parts.opening.len() + yaml.len() + parts.closing.len() + body.len(),
        );
        document.push_str(parts.opening);
        document.push_str(&yaml);
        document.push_str(parts.closing);
        document.push_str(body);
        return Ok(document);
    }

    let mut markdown: darkmatter::markdown::Markdown = frontmatter_source.to_string().into();
    markdown
        .fm_insert("last_updated", today)
        .map_err(|e| eyre!("failed to update last_updated: {e}"))?;
    *markdown.content_mut() = body.to_string();
    Ok(markdown.as_string())
}

struct FrontmatterParts<'a> {
    opening: &'a str,
    yaml: &'a str,
    closing: &'a str,
}

fn split_frontmatter_parts(text: &str) -> Option<FrontmatterParts<'_>> {
    let mut lines = text.split_inclusive('\n');
    let opening = lines.next()?;
    if trim_line_ending(opening) != "---" {
        return None;
    }

    let yaml_start = opening.len();
    let mut offset = yaml_start;
    for line in lines {
        let next_offset = offset + line.len();
        if trim_line_ending(line) == "---" {
            return Some(FrontmatterParts {
                opening: &text[..yaml_start],
                yaml: &text[yaml_start..offset],
                closing: &text[offset..next_offset],
            });
        }
        offset = next_offset;
    }

    None
}

fn upsert_last_updated_in_frontmatter(yaml: &str, today: &str, newline: &str) -> String {
    let mut updated = String::with_capacity(yaml.len() + today.len() + 32);
    let mut found = false;
    let mut had_trailing_newline = yaml.is_empty();

    for line in yaml.split_inclusive('\n') {
        let line_ending = if line.ends_with("\r\n") {
            "\r\n"
        } else if line.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let content = trim_line_ending(line);

        if let Some(rewritten) = rewrite_last_updated_line(content, today) {
            updated.push_str(&rewritten);
            updated.push_str(line_ending);
            found = true;
        } else {
            updated.push_str(line);
        }

        had_trailing_newline = !line_ending.is_empty();
    }

    if !found {
        if !updated.is_empty() && !had_trailing_newline {
            updated.push_str(newline);
        }
        updated.push_str("last_updated: ");
        updated.push_str(today);
        updated.push_str(newline);
    }

    updated
}

fn rewrite_last_updated_line(line: &str, today: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("last_updated:")?;
    let indent = &line[..line.len() - trimmed.len()];
    if !indent.is_empty() {
        return None;
    }
    let quote = rest
        .trim_start()
        .chars()
        .next()
        .filter(|quote| matches!(quote, '"' | '\''));

    let mut rewritten = String::from(indent);
    rewritten.push_str("last_updated: ");
    match quote {
        Some(quote) => {
            rewritten.push(quote);
            rewritten.push_str(today);
            rewritten.push(quote);
        }
        None => rewritten.push_str(today),
    }
    Some(rewritten)
}

fn detect_newline(text: &str) -> &str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
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
        Provider::Codex => find_positional_prompt_location(args, 0),
        Provider::OpenCode => find_positional_prompt_location(args, 0),
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

/// Extract the user's prompt string from the raw passthrough args.
/// Returns the first non-switch argument, if any.
fn extract_user_prompt(passthrough: &[String]) -> Option<String> {
    passthrough
        .iter()
        .find(|arg| !arg.starts_with('-'))
        .cloned()
}

/// Returns true if a prompt string is present — either as a remaining
/// non-switch arg in `child_args`, from a composition source, or via stdin.
fn has_prompt_source(args: &WrapperArgs, child_args: &[String], stdin_seed: Option<&str>) -> bool {
    // Composition switches provide a prompt
    if args.prompt_file.is_some() || args.frontmatter_prompt.is_some() || args.compose.is_some() {
        return true;
    }
    if stdin_seed.is_some() {
        return true;
    }
    // Check for a non-switch positional arg in passthrough
    child_args.iter().any(|arg| !arg.starts_with('-'))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExtractedWrapperFlags {
    yolo: bool,
    interactive: bool,
    repo: bool,
    quiet: bool,
    silent: bool,
    verbose: bool,
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
            "-i" | "--interactive" => {
                extracted.interactive = true;
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
            "-v" | "--verbose" => {
                extracted.verbose = true;
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
    use std::sync::{Arc, Mutex};

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
            "-i".to_string(),
            "task".to_string(),
            "-y".to_string(),
        ];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args);

        assert!(extracted.yolo);
        assert!(extracted.interactive);
        assert_eq!(args, vec!["--json", "task"]);
    }

    #[test]
    fn extract_wrapper_flags_lifts_interactive_long_form() {
        let mut args = vec!["--interactive".to_string(), "do something".to_string()];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args);

        assert!(extracted.interactive);
        assert_eq!(args, vec!["do something"]);
    }

    #[test]
    fn old_non_interactive_flags_pass_through_to_provider() {
        let mut args = vec![
            "-n".to_string(),
            "--non-interactive".to_string(),
            "--ni".to_string(),
            "task".to_string(),
        ];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args);

        // Old flags should NOT be consumed by Claudine
        assert!(!extracted.interactive);
        assert_eq!(args, vec!["-n", "--non-interactive", "--ni", "task"]);
    }

    #[test]
    fn has_prompt_source_detects_positional_arg() {
        let args = WrapperArgs {
            yolo: false,
            interactive: false,
            model: None,
            output: None,
            system_prompt: None,
            timeout: None,
            dry_run: false,
            quiet: false,
            silent: false,
            operation: None,
            sandbox: false,
            repo: false,
            prompt_file: None,
            frontmatter_prompt: None,
            compose: None,
            mcp: false,
            mcp_use: Vec::new(),
            strict: false,
            include: Vec::new(),
            passthrough: Vec::new(),
        };

        // No prompt, no composition → no prompt source
        assert!(!has_prompt_source(&args, &[], None));

        // Non-switch arg in child_args → prompt source
        assert!(has_prompt_source(&args, &["fix the bug".to_string()], None));

        // Switch-only args → no prompt source
        assert!(!has_prompt_source(&args, &["--json".to_string()], None));

        // stdin_seed → prompt source
        assert!(has_prompt_source(&args, &[], Some("hello")));
    }

    #[test]
    fn has_prompt_source_detects_composition_switches() {
        let mut args = WrapperArgs {
            yolo: false,
            interactive: false,
            model: None,
            output: None,
            system_prompt: None,
            timeout: None,
            dry_run: false,
            quiet: false,
            silent: false,
            operation: None,
            sandbox: false,
            repo: false,
            prompt_file: None,
            frontmatter_prompt: None,
            compose: None,
            mcp: false,
            mcp_use: Vec::new(),
            strict: false,
            include: Vec::new(),
            passthrough: Vec::new(),
        };

        args.prompt_file = Some("file.md".to_string());
        assert!(has_prompt_source(&args, &[], None));

        args.prompt_file = None;
        args.frontmatter_prompt = Some("file.md".to_string());
        assert!(has_prompt_source(&args, &[], None));

        args.frontmatter_prompt = None;
        args.compose = Some("file.md".to_string());
        assert!(has_prompt_source(&args, &[], None));
    }

    #[test]
    fn extract_user_prompt_finds_first_non_switch() {
        assert_eq!(
            extract_user_prompt(&["--json".to_string(), "fix bug".to_string()]),
            Some("fix bug".to_string())
        );
        assert_eq!(
            extract_user_prompt(&["--json".to_string(), "--verbose".to_string()]),
            None
        );
        assert_eq!(
            extract_user_prompt(&["hello world".to_string()]),
            Some("hello world".to_string())
        );
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
        let mut args = vec!["do something".to_string(), "--operation=deploy".to_string()];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args);

        assert_eq!(extracted.operation.as_deref(), Some("deploy"));
        assert_eq!(args, vec!["do something"]);
    }

    #[test]
    fn extract_wrapper_flags_lifts_op_equals_form() {
        let mut args = vec!["do something".to_string(), "--op=review".to_string()];

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

    #[test]
    fn live_stream_sink_maps_coarse_events_into_dispatch_meta() {
        let recorded: Arc<Mutex<Vec<DispatchEventMeta>>> = Arc::new(Mutex::new(Vec::new()));
        let sink_events = recorded.clone();
        let mut sink = LiveStreamSink::with_dispatcher(
            Provider::Codex,
            EnvironmentContext::default(),
            Verbosity::Silent,
            Arc::new(Mutex::new(StructuredSummaryDetails::default())),
            move |_event, meta| {
                sink_events.lock().unwrap().push(meta);
            },
        );

        let mut session_meta = StreamEventMeta::default();
        session_meta.extra.insert(
            "session_id".into(),
            serde_json::Value::String("thread-1".into()),
        );
        session_meta.extra.insert(
            "model".into(),
            serde_json::Value::String("codex-mini".into()),
        );
        sink.on_session_start(&session_meta);

        sink.on_turn_start(&StreamEventMeta::default());

        let mut tool_meta = StreamEventMeta::default();
        tool_meta.extra.insert(
            "tool_name".into(),
            serde_json::Value::String("search".into()),
        );
        tool_meta
            .extra
            .insert("tool_input".into(), serde_json::json!({"query": "rust"}));
        sink.on_before_tool(&tool_meta);

        let mut after_tool_meta = StreamEventMeta::default();
        after_tool_meta.extra.insert(
            "tool_name".into(),
            serde_json::Value::String("search".into()),
        );
        after_tool_meta
            .extra
            .insert("tool_response".into(), serde_json::json!({"hits": 3}));
        sink.on_after_tool(&after_tool_meta);

        let mut error_meta = StreamEventMeta::default();
        error_meta.extra.insert(
            "error_message".into(),
            serde_json::Value::String("boom".into()),
        );
        sink.on_turn_error(&error_meta);

        sink.on_turn_complete(&StreamEventMeta::default());

        let metas = recorded.lock().unwrap().clone();
        assert_eq!(metas[0].event, AgenticEvent::SessionStart);
        assert_eq!(metas[0].session_id.as_deref(), Some("thread-1"));
        assert_eq!(
            metas[0].extra["model"],
            serde_json::Value::String("codex-mini".into())
        );

        assert_eq!(metas[1].event, AgenticEvent::BeforePrompt);
        assert_eq!(metas[1].session_id.as_deref(), Some("thread-1"));

        assert_eq!(metas[2].event, AgenticEvent::BeforeTool);
        assert_eq!(metas[2].tool_name.as_deref(), Some("search"));
        assert_eq!(metas[2].tool_input.as_ref().unwrap()["query"], "rust");

        assert_eq!(metas[3].event, AgenticEvent::AfterTool);
        assert_eq!(metas[3].tool_response.as_ref().unwrap()["hits"], 3);

        assert_eq!(metas[4].event, AgenticEvent::TurnError);
        assert_eq!(metas[4].error.as_deref(), Some("boom"));

        assert_eq!(metas[5].event, AgenticEvent::TurnComplete);
        assert_eq!(metas[5].session_id.as_deref(), Some("thread-1"));
    }

    #[test]
    fn rewrite_markdown_preserves_block_scalar_frontmatter_layout() {
        let original = concat!(
            "---\n",
            "prompt: |-\n",
            "  First line\n",
            "  Second line\n",
            "last_updated: 2026-03-18\n",
            "---\n",
            "Old body\n",
        );

        let rewritten =
            rewrite_markdown_preserving_frontmatter(original, "Fresh body\n", "2026-03-19")
                .unwrap();

        assert!(rewritten.contains("prompt: |-"));
        assert!(rewritten.contains("  First line\n  Second line\n"));
        assert!(rewritten.contains("last_updated: 2026-03-19"));
        assert!(rewritten.ends_with("---\nFresh body\n"));
    }

    #[test]
    fn rewrite_markdown_adds_last_updated_without_reserializing_frontmatter() {
        let original = concat!(
            "---\n",
            "prompt: |-\n",
            "  Keep this formatting\n",
            "---\n",
            "Body\n",
        );

        let rewritten =
            rewrite_markdown_preserving_frontmatter(original, "Updated body\n", "2026-03-19")
                .unwrap();

        assert!(rewritten.contains("prompt: |-"));
        assert!(rewritten.contains("  Keep this formatting\n"));
        assert!(rewritten.contains("last_updated: 2026-03-19\n---\nUpdated body\n"));
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
                flags in prop::collection::vec("-y|--yolo|-i|--interactive|-q|--quiet|--silent", 0..5),
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
                if flags.iter().any(|f| f == "-i" || f == "--interactive") {
                    assert!(extracted.interactive);
                }
            }
        }
    }
}
