pub(crate) mod env;
pub(crate) mod exec;
pub(crate) mod profile;
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
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
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

pub(crate) struct StructuredCodexOutput {
    pub(crate) last_message_path: PathBuf,
}

impl StructuredCodexOutput {
    pub(crate) fn prepare(args: &mut Vec<String>) -> Self {
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

    pub(crate) fn apply_to_summary(
        &self,
        summary: &mut claudine::stream::summary::StreamExecutionSummary,
    ) {
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
pub(crate) struct StructuredSummaryDetails {
    pub(crate) tool_names: Vec<String>,
}

impl StructuredSummaryDetails {
    fn record_tool_name(&mut self, tool_name: &str) {
        if !tool_name.is_empty() && !self.tool_names.iter().any(|name| name == tool_name) {
            self.tool_names.push(tool_name.to_string());
        }
    }
}

pub(crate) mod composition;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HarnessPromptMode {
    Passthrough,
    Inline,
    Compose,
}

#[derive(Debug, Clone)]
pub(crate) struct HarnessPromptState {
    pub(crate) mode: HarnessPromptMode,
    pub(crate) source_path: PathBuf,
    pub(crate) base_prompt: Option<String>,
    pub(crate) overlay: indexmap::IndexMap<String, serde_json::Value>,
    pub(crate) prompt_tail: Vec<String>,
    pub(crate) next_prompt_override: Option<String>,
    pub(crate) next_resume_session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct MaterializedHarnessPrompt {
    pub(crate) frontmatter: serde_json::Value,
    pub(crate) prompt: String,
    pub(crate) env_overrides: Vec<(String, String)>,
    pub(crate) inline_closure_plan: Option<claudine::composition::InlineClosurePlan>,
}

#[derive(Debug, Clone)]
pub(crate) struct AttemptLaunch {
    pub(crate) args: Vec<String>,
    pub(crate) env: HashMap<OsString, OsString>,
    pub(crate) stdin_seed: Option<String>,
    pub(crate) timeout: Option<u64>,
}

#[derive(Debug, Clone)]
struct NextAttemptPlan {
    next_attempt: u32,
    prompt_append: Option<String>,
    prompt_override: Option<String>,
    set_overlay: Option<indexmap::IndexMap<String, serde_json::Value>>,
    redirect_source: Option<PathBuf>,
    resume_session_id: Option<String>,
    clear_prompt_tail: bool,
}

fn harness_policy_root(source_path: &Path, repo_root: Option<&Path>) -> Option<PathBuf> {
    let source_dir = source_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())?;

    if let Some(source_repo_root) = find_git_root(source_dir) {
        return Some(source_repo_root);
    }

    if let Some(repo_root) = repo_root
        && source_path.starts_with(repo_root)
    {
        return Some(repo_root.to_path_buf());
    }

    Some(source_dir.to_path_buf())
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start;

    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    None
}

pub(crate) fn build_harness_shell_options(
    source_path: &Path,
    repo_root: Option<&Path>,
) -> claudine::harness::ShellApprovalOptions {
    claudine::harness::ShellApprovalOptions {
        policy_root: harness_policy_root(source_path, repo_root),
        approval_handler: None,
        ..Default::default()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WrapperHarnessPermissionProbe {
    provider: Provider,
    child_args: Vec<String>,
    repo_root: Option<PathBuf>,
}

impl WrapperHarnessPermissionProbe {
    fn new(provider: Provider, child_args: Vec<String>, repo_root: Option<&Path>) -> Self {
        Self {
            provider,
            child_args,
            repo_root: repo_root.map(Path::to_path_buf),
        }
    }

    fn sandbox_value(&self) -> Option<&str> {
        self.child_args
            .iter()
            .position(|arg| arg == "--sandbox")
            .and_then(|index| self.child_args.get(index + 1))
            .map(String::as_str)
    }

    fn workspace_root<'a>(&'a self, source_path: &'a Path) -> Option<&'a Path> {
        self.repo_root.as_deref().or_else(|| {
            source_path
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
        })
    }
}

impl claudine::harness::HarnessPermissionProbe for WrapperHarnessPermissionProbe {
    fn can_write(
        &self,
        path: &Path,
        source_path: &Path,
    ) -> claudine::harness::PermissionAssessment {
        use claudine::harness::PermissionAssessment;

        if self.provider != Provider::Codex {
            return PermissionAssessment::Allowed;
        }

        if self
            .child_args
            .iter()
            .any(|arg| arg == "--dangerously-bypass-approvals-and-sandbox" || arg == "--yolo")
        {
            return PermissionAssessment::Allowed;
        }

        match self.sandbox_value() {
            Some("danger-full-access") => PermissionAssessment::Allowed,
            Some("read-only") => PermissionAssessment::Denied {
                reason: "Codex is running in read-only sandbox mode".to_string(),
            },
            Some("workspace-write") => {
                let Some(root) = self.workspace_root(source_path) else {
                    return PermissionAssessment::Unknown {
                        reason: "workspace-write mode is active, but no workspace root could be determined".to_string(),
                    };
                };
                if path.starts_with(root) {
                    PermissionAssessment::Allowed
                } else {
                    PermissionAssessment::Denied {
                        reason: format!(
                            "Codex workspace-write sandbox only allows writes under {}",
                            root.display()
                        ),
                    }
                }
            }
            Some(mode) => PermissionAssessment::Unknown {
                reason: format!("unrecognized Codex sandbox mode '{mode}'"),
            },
            None => PermissionAssessment::Allowed,
        }
    }
}

#[derive(Clone)]
pub(crate) struct CachedHarnessLoopContext {
    source_path: PathBuf,
    repo_root: Option<PathBuf>,
    shell_options: claudine::harness::ShellApprovalOptions,
}

impl CachedHarnessLoopContext {
    fn new(source_path: &Path, repo_root: Option<&Path>) -> Self {
        Self {
            source_path: source_path.to_path_buf(),
            repo_root: repo_root.map(Path::to_path_buf),
            shell_options: build_harness_shell_options(source_path, repo_root),
        }
    }

    fn refresh(&mut self, source_path: &Path, repo_root: Option<&Path>) {
        let repo_root = repo_root.map(Path::to_path_buf);
        if self.source_path != source_path || self.repo_root != repo_root {
            self.source_path = source_path.to_path_buf();
            self.repo_root = repo_root;
            self.shell_options =
                build_harness_shell_options(&self.source_path, self.repo_root.as_deref());
        }
    }

    fn resolve_context(&self) -> claudine::harness::HarnessResolutionContext<'_> {
        claudine::harness::HarnessResolutionContext {
            source_path: &self.source_path,
            repo_root: self.repo_root.as_deref(),
        }
    }

    fn shell_options(&self) -> &claudine::harness::ShellApprovalOptions {
        &self.shell_options
    }
}

pub(crate) struct LiveStreamSink {
    provider: Provider,
    env: EnvironmentContext,
    verbosity: Verbosity,
    session_id: Option<String>,
    model: Option<String>,
    start_emitted: bool,
    summary_details: Arc<Mutex<StructuredSummaryDetails>>,
    context_extra: HashMap<String, serde_json::Value>,
    dispatch: StreamDispatchFn,
}

impl LiveStreamSink {
    pub(crate) fn new(
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

    pub(crate) fn with_context_extra(
        mut self,
        context_extra: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.context_extra = context_extra;
        self
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
            context_extra: HashMap::new(),
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
        for (key, value) in &self.context_extra {
            extra.entry(key.clone()).or_insert_with(|| value.clone());
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

pub(crate) fn structured_verbosity(silent: bool, quiet: bool) -> Verbosity {
    if silent {
        Verbosity::Silent
    } else if quiet {
        Verbosity::Quiet
    } else {
        Verbosity::Normal
    }
}

pub(crate) fn wrap_terminal() -> Terminal {
    crate::log::terminal()
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

/// Reject retired composition flags that should no longer be forwarded to
/// wrapped providers. Users should migrate to `claudine compose` or
/// `claudine inline-compose`.
fn reject_retired_composition_flags(args: &[String]) -> Result<()> {
    const RETIRED: &[(&str, &str)] = &[
        (
            "--compose",
            "claudine compose --<provider> <file>",
        ),
        (
            "--frontmatter-prompt",
            "claudine inline-compose --<provider> <file>",
        ),
        (
            "--prompt-file",
            "claudine compose --<provider> <file>",
        ),
    ];

    for (flag, replacement) in RETIRED {
        if args.iter().any(|a| a == flag || a.starts_with(&format!("{flag}="))) {
            return Err(eyre!(
                "{flag} has been retired; use `{replacement}` instead"
            ));
        }
    }

    Ok(())
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
    let term = wrap_terminal();

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
    let has_prompt = has_prompt_source(&child_args, None);

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
    reject_retired_composition_flags(&child_args)?;

    if yolo_requested && let Some(warn) = profile.apply_yolo(&mut child_args, &mut env_overrides)? {
        deferred_warnings.push(warn);
    }
    if yolo_requested && !profile.has_supported_yolo() {
        yolo_enabled = false;
    }

    if non_interactive_requested {
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
    let stdin_seed: Option<String> = None;

    // -- Final argument validation -------------------------------------------
    let effective_non_interactive = non_interactive_requested;
    profile.validate_final_args(&child_args, effective_non_interactive, stdin_seed.is_some())?;

    if args.timeout.is_some() && !effective_non_interactive {
        return Err(eyre!(
            "--timeout can only be used in non-interactive mode \
             (provide a prompt)"
        ));
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
            child_cwd,
            &term,
        );
        return Ok(0);
    }

    let prompt_display = extract_user_prompt(&args.passthrough);
    let dispatch_context = HashMap::new();

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
            None,
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

    let wrapper_harness = if effective_non_interactive {
        let base_prompt =
            extract_prompt_from_child_args(provider, &child_args, stdin_seed.as_deref());
        let harness_source = base_prompt.as_ref().and_then(|_| {
            find_wrapper_harness_source(provider, env_plan.repo_root.as_deref(), &cwd)
        });

        if let (Some(base_prompt), Some(source_path)) = (base_prompt, harness_source) {
            let seed = materialize_passthrough_harness_seed(&source_path, base_prompt.clone())?;
            let harness_enabled = claudine::harness::has_harness_properties(&seed.frontmatter);
            if harness_enabled {
                let resolve_ctx = claudine::harness::HarnessResolutionContext {
                    source_path: &source_path,
                    repo_root: env_plan.repo_root.as_deref(),
                };
                let shell_options =
                    build_harness_shell_options(&source_path, env_plan.repo_root.as_deref());
                claudine::harness::parse_harness_plan_with_shell(
                    &seed.frontmatter,
                    &source_path,
                    &resolve_ctx,
                    Some(&shell_options),
                )
                .map_err(|e| eyre!("{e}"))?;

                Some((source_path, base_prompt, seed))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Execute the provider. Composition and harness execution are handled by
    // `claudine compose` / `claudine inline-compose` through the wrapper-grade
    // composition executor; the wrapper path handles plain prompt passthrough.
    let exit_code = if let Some((source_path, base_prompt, initial_materialized)) = wrapper_harness
    {
        let mut prompt_state = HarnessPromptState {
            mode: HarnessPromptMode::Passthrough,
            source_path,
            base_prompt: Some(base_prompt),
            overlay: indexmap::IndexMap::new(),
            prompt_tail: Vec::new(),
            next_prompt_override: None,
            next_resume_session_id: None,
        };

        let mut harness_base_args = child_args.clone();
        strip_prompt_from_args(provider, &mut harness_base_args);
        if !use_structured {
            profile.prepare_captured_output(&mut harness_base_args);
        }

        run_harness_loop(
            provider,
            profile,
            binary_path.as_path(),
            child_cwd,
            effective_non_interactive,
            args.timeout,
            &harness_base_args,
            &env_plan.env,
            &mut prompt_state,
            env_plan.repo_root.as_deref(),
            use_structured,
            structured_codex_output.as_ref(),
            stdout_noise,
            stderr_noise,
            profile.suppress_structured_stderr_on_success(),
            !silent_requested,
            stream_verbosity,
            verbose_requested,
            &env_context,
            &dispatch_context,
            Some(initial_materialized),
            &term,
        )?
    } else if use_structured {
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
            )
            .with_context_extra(dispatch_context.clone()),
            parser_config,
        );
        let stream_result = exec::run_child_stream(
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
        let mut summary = stream_result.data;
        if let Some(codex_output) = structured_codex_output.as_ref() {
            codex_output.apply_to_summary(&mut summary);
        }
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

        emit_stream_summary(
            &summary,
            profile,
            &env_context,
            stream_verbosity,
            verbose_requested,
            &summary_details.lock().unwrap().clone(),
        );

        summary.exit_code
    } else {
        // Legacy path: forward I/O to terminal
        let result = exec::run_child(
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
        result.data
    };

    // MCP injector cleanup: remove temp files written during injection
    if let Some((injector, injection_result)) = mcp_cleanup
        && let Err(e) = injector.cleanup(&injection_result)
    {
        tracing::warn!("MCP injector cleanup failed: {e}");
    }

    Ok(exit_code)
}

fn merge_frontmatter_overlay(
    overlay: &mut indexmap::IndexMap<String, serde_json::Value>,
    update: &indexmap::IndexMap<String, serde_json::Value>,
) {
    for (key, value) in update {
        if value.is_null() {
            overlay.shift_remove(key);
        } else {
            overlay.insert(key.clone(), value.clone());
        }
    }
}

pub(crate) fn strip_prompt_from_args(provider: Provider, args: &mut Vec<String>) {
    match provider {
        Provider::Gemini | Provider::QwenCode => {
            let mut index = 0;
            while index < args.len() {
                if args[index] == "--prompt" || args[index] == "-p" {
                    if index + 1 < args.len() {
                        args.drain(index..=index + 1);
                    } else {
                        args.remove(index);
                    }
                    return;
                }
                if args[index].starts_with("--prompt=") || args[index].starts_with("-p=") {
                    args.remove(index);
                    return;
                }
                index += 1;
            }
        }
        Provider::Goose => {
            if let Some(index) = args.iter().position(|arg| arg == "-t" || arg == "--text") {
                if index + 1 < args.len() {
                    args.drain(index..=index + 1);
                } else {
                    args.remove(index);
                }
            }
        }
        Provider::Codex | Provider::OpenCode => {
            if let Some(location) = find_prompt_location(provider, args) {
                match location {
                    PromptLocation::Value(index) => {
                        if index < args.len() {
                            args.remove(index);
                        }
                    }
                    PromptLocation::Inline { index, .. } => {
                        if index < args.len() {
                            args.remove(index);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn strip_prompt_tags_for_provider(provider: Provider, prompt: &str) -> String {
    if matches!(
        provider,
        Provider::Codex | Provider::Gemini | Provider::OpenCode
    ) {
        claudine::mcp::session::lex_tags(prompt).0
    } else {
        prompt.to_string()
    }
}

fn normalize_resume_args(profile: &dyn WrapperProfile, mut args: Vec<String>) -> Vec<String> {
    if args.first().is_some_and(|arg| arg == profile.binary()) {
        args.remove(0);
    }
    args
}

fn append_resume_passthrough_args(resume_args: &mut Vec<String>, base_args: &[String]) {
    let mut index = 0;
    while index < base_args.len() {
        match base_args[index].as_str() {
            "--json" | "--verbose" => {
                if !resume_args.iter().any(|arg| arg == &base_args[index]) {
                    resume_args.push(base_args[index].clone());
                }
            }
            "--output-format" | "--format" | "--output-last-message" => {
                if index + 1 < base_args.len()
                    && !resume_args.iter().any(|arg| arg == &base_args[index])
                {
                    resume_args.push(base_args[index].clone());
                    resume_args.push(base_args[index + 1].clone());
                }
                index += 1;
            }
            _ => {}
        }
        index += 1;
    }
}

fn frontmatter_map_to_value(frontmatter: &darkmatter::markdown::Frontmatter) -> serde_json::Value {
    serde_json::Value::Object(
        frontmatter
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    )
}

pub(crate) fn materialized_harness_prompt_from_prepared(
    prepared: &claudine::composition::PreparedComposition,
) -> MaterializedHarnessPrompt {
    let inline_closure_plan = match &prepared.closure {
        claudine::composition::CompositionClosurePlan::Inline(plan) => Some(plan.clone()),
        claudine::composition::CompositionClosurePlan::Direct => None,
    };

    MaterializedHarnessPrompt {
        frontmatter: prepared.effective_frontmatter.clone(),
        prompt: prepared.prompt.clone(),
        env_overrides: Vec::new(),
        inline_closure_plan,
    }
}

fn materialize_passthrough_harness_seed(
    source_path: &Path,
    prompt: String,
) -> Result<MaterializedHarnessPrompt> {
    let source_text = fs::read_to_string(source_path)
        .map_err(|e| eyre!("failed to read '{}': {e}", source_path.display()))?;
    let source_markdown: darkmatter::markdown::Markdown = source_text.into();
    let options =
        darkmatter::markdown::compose::ComposeOptions::new().with_source_file(source_path);
    let (composed, _report) = source_markdown.compose_with(options).map_err(|e| {
        eyre!(
            "Darkmatter compose failed for '{}': {e}",
            source_path.display()
        )
    })?;

    Ok(MaterializedHarnessPrompt {
        frontmatter: frontmatter_map_to_value(composed.frontmatter()),
        prompt,
        env_overrides: Vec::new(),
        inline_closure_plan: None,
    })
}

fn provider_agent_id(provider: Provider) -> Option<claudine::agents::AgentId> {
    match provider {
        Provider::Claude => Some(claudine::agents::AgentId::ClaudeCode),
        Provider::Codex => Some(claudine::agents::AgentId::Codex),
        Provider::Gemini => Some(claudine::agents::AgentId::GeminiCli),
        Provider::Goose => Some(claudine::agents::AgentId::Goose),
        Provider::KimiCode => Some(claudine::agents::AgentId::KimiCode),
        Provider::OpenCode => Some(claudine::agents::AgentId::OpenCode),
        Provider::QwenCode => Some(claudine::agents::AgentId::QwenCli),
        Provider::RooCode => Some(claudine::agents::AgentId::RooCode),
        _ => None,
    }
}

fn find_wrapper_harness_source(
    provider: Provider,
    repo_root: Option<&Path>,
    cwd: &Path,
) -> Option<PathBuf> {
    let agent = claudine::agents::agent_for(provider_agent_id(provider)?);
    let search_root = repo_root.unwrap_or(cwd);

    agent
        .capabilities()
        .runtime
        .system_prompt
        .memory_files
        .iter()
        .filter(|path| !path.starts_with('~'))
        .map(PathBuf::from)
        .find_map(|relative| {
            let candidate = search_root.join(relative);
            candidate.is_file().then_some(candidate)
        })
}

fn materialize_harness_prompt(
    state: &HarnessPromptState,
    _repo_root: Option<&Path>,
) -> Result<MaterializedHarnessPrompt> {
    let source_text = fs::read_to_string(&state.source_path)
        .map_err(|e| eyre!("failed to read '{}': {e}", state.source_path.display()))?;
    let mut effective_markdown: darkmatter::markdown::Markdown = source_text.clone().into();
    merge_frontmatter_overlay(
        effective_markdown.frontmatter_mut().as_map_mut(),
        &state.overlay,
    );

    let (mut prompt, frontmatter, env_overrides, inline_closure_plan) = match state.mode {
        HarnessPromptMode::Passthrough => {
            let options = darkmatter::markdown::compose::ComposeOptions::new()
                .with_source_file(&state.source_path);
            let (composed, _report) = effective_markdown.compose_with(options).map_err(|e| {
                eyre!(
                    "Darkmatter compose failed for '{}': {e}",
                    state.source_path.display()
                )
            })?;
            let prompt = state.base_prompt.clone().ok_or_else(|| {
                eyre!(
                    "missing passthrough prompt seed for '{}'",
                    state.source_path.display()
                )
            })?;
            (
                prompt,
                frontmatter_map_to_value(composed.frontmatter()),
                Vec::new(),
                None,
            )
        }
        HarnessPromptMode::Compose => {
            let options = darkmatter::markdown::compose::ComposeOptions::new()
                .with_source_file(&state.source_path);
            let (composed, _report) = effective_markdown.compose_with(options).map_err(|e| {
                eyre!(
                    "Darkmatter compose failed for '{}': {e}",
                    state.source_path.display()
                )
            })?;
            let body = composed.content().to_string();

            let env_overrides = Vec::new();

            (
                body,
                frontmatter_map_to_value(composed.frontmatter()),
                env_overrides,
                None,
            )
        }
        HarnessPromptMode::Inline => {
            claudine::composition::validate_file_permissions(&state.source_path)
                .map_err(|e| eyre!("frontmatter-prompt: {e}"))?;
            let source = claudine::composition::ResolvedCompositionSource {
                original_ref: state.source_path.display().to_string(),
                resolved_path: state.source_path.clone(),
                original_text: source_text.clone(),
                markdown: effective_markdown.clone(),
            };
            let prepared = claudine::composition::prepare_inline(&source)
                .map_err(|e| eyre!("frontmatter-prompt: {e}"))?;
            (
                prepared.prompt,
                prepared.effective_frontmatter,
                Vec::new(),
                match prepared.closure {
                    claudine::composition::CompositionClosurePlan::Inline(plan) => Some(plan),
                    claudine::composition::CompositionClosurePlan::Direct => None,
                },
            )
        }
    };

    if let Some(ref override_prompt) = state.next_prompt_override {
        prompt = override_prompt.clone();
    } else {
        for tail in &state.prompt_tail {
            prompt.push_str("\n\n");
            prompt.push_str(tail);
        }
    }

    Ok(MaterializedHarnessPrompt {
        frontmatter,
        prompt,
        env_overrides,
        inline_closure_plan,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_harness_launch(
    provider: Provider,
    profile: &dyn WrapperProfile,
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    state: &mut HarnessPromptState,
    materialized: &MaterializedHarnessPrompt,
    effective_non_interactive: bool,
    cli_timeout: Option<u64>,
    plan_timeout: Option<std::time::Duration>,
) -> Result<AttemptLaunch> {
    let mut args = if let Some(session_id) = state.next_resume_session_id.take() {
        let mut args = normalize_resume_args(profile, profile.build_resume_args(&session_id)?);
        append_resume_passthrough_args(&mut args, base_args);
        args
    } else {
        base_args.to_vec()
    };
    state.next_prompt_override = None;

    let prompt = strip_prompt_tags_for_provider(provider, &materialized.prompt);
    let mut stdin_seed = None;
    profile.apply_prompt_body(
        &mut args,
        &mut stdin_seed,
        &prompt,
        effective_non_interactive,
    )?;
    profile.validate_final_args(&args, effective_non_interactive, stdin_seed.is_some())?;

    let mut env = base_env.clone();
    for (key, value) in &materialized.env_overrides {
        env.insert(key.clone().into(), value.clone().into());
    }

    Ok(AttemptLaunch {
        args,
        env,
        stdin_seed,
        timeout: cli_timeout.or_else(|| plan_timeout.map(|timeout| timeout.as_secs())),
    })
}

#[allow(clippy::too_many_arguments)]
fn execute_harness_attempt(
    attempt: u32,
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_cwd: &Path,
    launch: &AttemptLaunch,
    prompt_mode: HarnessPromptMode,
    prompt_state: &HarnessPromptState,
    _materialized: &MaterializedHarnessPrompt,
    use_structured: bool,
    structured_codex_output: Option<&StructuredCodexOutput>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    suppress_stderr_on_success: bool,
    show_checks: bool,
    stream_verbosity: Verbosity,
    verbose_requested: bool,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    term: &Terminal,
) -> Result<claudine::harness::AttemptOutcome> {
    let (exit_code, termination, session_id, final_response, stderr_text) = if use_structured {
        let summary_details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
        let parser_config = claudine::stream::ParserConfig::default();
        let parser = claudine::stream::create_parser(
            provider,
            LiveStreamSink::new(
                provider,
                env_context.clone(),
                stream_verbosity,
                summary_details.clone(),
            )
            .with_context_extra(dispatch_context.clone()),
            parser_config,
        );
        let stream_result = exec::run_child_stream(
            binary_path,
            &launch.args,
            &launch.env,
            child_cwd,
            launch.timeout,
            stderr_noise,
            suppress_stderr_on_success,
            launch.stdin_seed.as_deref(),
            parser,
        )?;
        let termination = stream_result.termination;
        let mut summary = stream_result.data;
        if let Some(codex_output) = structured_codex_output {
            codex_output.apply_to_summary(&mut summary);
        }
        if provider == Provider::Codex && !summary.assistant_text.is_empty() {
            let text = &summary.assistant_text;
            if std::io::stdout().is_terminal() {
                let rendered = crate::output::render_assistant_markdown(text, term);
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

        emit_stream_summary(
            &summary,
            profile,
            env_context,
            stream_verbosity,
            verbose_requested,
            &summary_details.lock().unwrap().clone(),
        );

        (
            summary.exit_code,
            termination,
            summary.session_id.clone(),
            summary.assistant_text.clone(),
            summary.stderr_text.clone(),
        )
    } else {
        let capture = exec::run_child_capture(
            binary_path,
            &launch.args,
            &launch.env,
            child_cwd,
            launch.timeout,
            exec::ChildIoOptions {
                stdout_noise_prefixes: stdout_noise,
                stderr_noise_prefixes: stderr_noise,
                stdin_seed: launch.stdin_seed.as_deref(),
            },
        )?;
        let termination = capture.termination;
        let stdout = capture.data.stdout;
        let stderr = capture.data.stderr;
        let response = profile.parse_captured_output(&stdout);

        if !response.trim().is_empty() {
            if std::io::stdout().is_terminal() {
                let rendered = crate::output::render_assistant_markdown(&response, term);
                std::io::stdout().write_all(rendered.as_bytes())?;
                if !rendered.ends_with('\n') {
                    std::io::stdout().write_all(b"\n")?;
                }
            } else {
                std::io::stdout().write_all(response.as_bytes())?;
                if !response.ends_with('\n') {
                    std::io::stdout().write_all(b"\n")?;
                }
            }
            std::io::stdout().flush()?;
        }

        if !stderr.trim().is_empty() {
            eprintln!("{stderr}");
        }

        (
            capture.data.exit_code,
            termination,
            None,
            response,
            (!stderr.trim().is_empty()).then_some(stderr),
        )
    };

    if prompt_mode == HarnessPromptMode::Inline {
        report_inline_agent_status(
            provider,
            &prompt_state.source_path,
            &final_response,
            exit_code,
            termination,
            child_cwd,
            show_checks,
            term,
        );
    }

    Ok(claudine::harness::AttemptOutcome {
        attempt,
        session_id,
        final_response,
        exit_code,
        termination,
        stderr_text,
    })
}

#[allow(clippy::too_many_arguments)]
fn report_inline_agent_status(
    provider: Provider,
    source_path: &Path,
    final_response: &str,
    agent_exit: i32,
    termination: claudine::harness::ProcessTermination,
    child_cwd: &Path,
    show_checks: bool,
    term: &Terminal,
) {
    let provider_name = crate::output::capitalize_provider(provider);
    let display_path = source_path
        .strip_prefix(child_cwd)
        .unwrap_or(source_path)
        .display();
    let was_interrupted = matches!(
        termination,
        claudine::harness::ProcessTermination::Interrupted
    ) || agent_exit == 130
        || agent_exit == 143;

    if show_checks {
        if was_interrupted {
            log::message(&crate::output::fm_check_fail(
                &format!("{provider_name} agent was interrupted by the user (code {agent_exit})"),
                term,
            ));
        } else if agent_exit == 0 {
            log::message(&crate::output::fm_check_ok(
                &format!("{provider_name} agent completed successfully"),
                term,
            ));
        } else {
            log::message(&crate::output::fm_check_fail(
                &format!("{provider_name} agent exited with error (code {agent_exit})"),
                term,
            ));
        }
    }

    if was_interrupted && show_checks {
        if final_response.trim().is_empty() {
            log::message(&crate::output::fm_check_fail(
                &format!(
                    "<b>User interrupted the agent with CTRL+C; the body of \
                     <blue-500>{display_path}</blue-500> is empty so it appears no work was accomplished.</b>"
                ),
                term,
            ));
        } else {
            log::message(&crate::output::fm_check_fail(
                &format!(
                    "<b>User interrupted the agent with CTRL+C; the body of \
                     <blue-500>{display_path}</blue-500> has been at least partially filled:</b>"
                ),
                term,
            ));
            eprintln!();
            for line in final_response.lines() {
                eprintln!("  {line}");
            }
        }
    }
}

/// Attempt inline closure validation and application.
///
/// Returns `Ok(())` on success (file rewritten), or a list of
/// `ValidationFailure`s that should be routed through the harness handler
/// system for potential retry/resume/redirect recovery.
fn try_inline_closure(
    closure_plan: &claudine::composition::InlineClosurePlan,
    final_response: &str,
    source_path: &Path,
    child_cwd: &Path,
    show_checks: bool,
    term: &Terminal,
) -> Result<(), Vec<claudine::harness::ValidationFailure>> {
    use claudine::harness::{ValidationEvent, ValidationFailure, ValidationRuleId, FailurePhase};

    let display_path = source_path
        .strip_prefix(child_cwd)
        .unwrap_or(source_path)
        .display();

    let replacement_body = match claudine::composition::closure::extract_replacement_body(
        final_response,
    ) {
        Ok(body) => body,
        Err(error) => {
            let message = format!(
                "the referenced file -- {display_path} -- did not receive a valid replacement body: {error}"
            );
            if show_checks {
                log::message(&crate::output::fm_check_fail(&message, term));
            }
            return Err(vec![ValidationFailure {
                rule_id: ValidationRuleId(9000),
                event: ValidationEvent::InlineResponseEmpty,
                phase: FailurePhase::PostCheck,
                subject_key: Some(source_path.display().to_string()),
                message,
            }]);
        }
    };

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match claudine::composition::closure::apply_inline_closure(
        closure_plan,
        &replacement_body,
        source_path,
        &today,
    ) {
        Ok(()) => {
            if show_checks {
                log::message(&crate::output::fm_check_ok(
                    "Applied the captured replacement body to the target document",
                    term,
                ));
                log::message(&crate::output::fm_check_ok(
                    "Preserved original frontmatter and updated <bold>last_updated</bold>",
                    term,
                ));
            }
            Ok(())
        }
        Err(error) => {
            let is_unchanged = error
                .to_string()
                .contains("unchanged");
            let event = if is_unchanged {
                ValidationEvent::InlineBodyUnchanged
            } else {
                ValidationEvent::InlineResponseEmpty
            };
            let message = format!("failed to rewrite {display_path}: {error}");
            if show_checks {
                log::message(&crate::output::fm_check_fail(&message, term));
            }
            Err(vec![ValidationFailure {
                rule_id: ValidationRuleId(9001),
                event,
                phase: FailurePhase::PostCheck,
                subject_key: Some(source_path.display().to_string()),
                message,
            }])
        }
    }
}

fn apply_next_attempt_plan(state: &mut HarnessPromptState, plan: &NextAttemptPlan) {
    if plan.clear_prompt_tail {
        state.prompt_tail.clear();
    }
    if let Some(ref path) = plan.redirect_source {
        state.source_path = path.clone();
    }
    if let Some(ref overlay) = plan.set_overlay {
        merge_frontmatter_overlay(&mut state.overlay, overlay);
    }
    if let Some(ref append) = plan.prompt_append {
        state.prompt_tail.push(append.clone());
    }
    if let Some(ref prompt) = plan.prompt_override {
        state.next_prompt_override = Some(prompt.clone());
    }
    state.next_resume_session_id = plan.resume_session_id.clone();
}

#[allow(clippy::too_many_arguments)]
fn build_next_attempt_plan(
    action: &claudine::harness::HandlerAction,
    current_attempt: u32,
    default_max_retries: u32,
    failure_message: &str,
    profile: &dyn WrapperProfile,
    session_id: Option<&str>,
    source_path: &Path,
    repo_root: Option<&Path>,
    failure_ctx: &claudine::harness::FailureContext,
    term: &Terminal,
) -> Result<Option<NextAttemptPlan>> {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::prelude::Renderable;

    let dispatch_handler_feedback = |msg: &Option<String>, say: &Option<String>| {
        if let Some(text) = say.as_deref() {
            claudine::harness::speak_when_able(text);
        }
        if let Some(msg_text) = msg {
            let rendered = Prose::new(msg_text).render(term);
            eprintln!("{rendered}");
        }
    };

    match action {
        claudine::harness::HandlerAction::Retry {
            prompt_suffix,
            set,
            msg,
            say,
            retries,
        } => {
            let max = retries.unwrap_or(default_max_retries);
            if current_attempt >= max {
                log::warn(&format!(
                    "retry ceiling reached ({current_attempt}/{max}): {failure_message}"
                ));
                return Ok(None);
            }
            dispatch_handler_feedback(msg, say);
            let prompt_append = prompt_suffix.clone().or_else(|| {
                Some(format!(
                    "The previous attempt failed: {failure_message}. Please correct the issue and try again."
                ))
            });
            Ok(Some(NextAttemptPlan {
                next_attempt: current_attempt + 1,
                prompt_append,
                prompt_override: None,
                set_overlay: set.clone(),
                redirect_source: None,
                resume_session_id: None,
                clear_prompt_tail: false,
            }))
        }
        claudine::harness::HandlerAction::Resume {
            prompt,
            set,
            msg,
            say,
            retries,
        } => {
            let max = retries.unwrap_or(default_max_retries);
            if current_attempt >= max {
                log::warn(&format!(
                    "resume retry ceiling reached ({current_attempt}/{max}): {failure_message}"
                ));
                return Ok(None);
            }
            claudine::harness::validate_resume(
                &profile.provider().to_string(),
                profile.supports_resume(),
                session_id,
            )?;
            dispatch_handler_feedback(msg, say);
            Ok(Some(NextAttemptPlan {
                next_attempt: current_attempt + 1,
                prompt_append: None,
                prompt_override: Some(prompt.clone()),
                set_overlay: set.clone(),
                redirect_source: None,
                resume_session_id: session_id.map(|id| id.to_string()),
                clear_prompt_tail: false,
            }))
        }
        claudine::harness::HandlerAction::Redirect {
            file,
            set,
            msg,
            say,
            resume,
        } => {
            if *resume {
                claudine::harness::validate_resume(
                    &profile.provider().to_string(),
                    profile.supports_resume(),
                    session_id,
                )?;
            }
            dispatch_handler_feedback(msg, say);
            let resolve_ctx = claudine::harness::HarnessResolutionContext {
                source_path,
                repo_root,
            };
            let redirect_source = claudine::harness::resolve_harness_path(file, &resolve_ctx)?;
            let resume_session_id = if *resume {
                session_id.map(|id| id.to_string())
            } else {
                None
            };
            Ok(Some(NextAttemptPlan {
                next_attempt: current_attempt + 1,
                prompt_append: None,
                prompt_override: None,
                set_overlay: set.clone(),
                redirect_source: Some(redirect_source),
                resume_session_id,
                clear_prompt_tail: true,
            }))
        }
        claudine::harness::HandlerAction::Deviate {
            command,
            set,
            msg,
            say,
        } => {
            dispatch_handler_feedback(msg, say);
            match claudine::harness::execute_deviate_command(
                command,
                failure_ctx,
                Some(source_path),
            ) {
                Ok(deviate_exit) => {
                    if deviate_exit != 0 {
                        log::warn(&format!(
                            "deviate command '{}' exited with code {deviate_exit}",
                            command.raw
                        ));
                    }
                    Ok(Some(NextAttemptPlan {
                        next_attempt: current_attempt + 1,
                        prompt_append: None,
                        prompt_override: None,
                        set_overlay: set.clone(),
                        redirect_source: None,
                        resume_session_id: None,
                        clear_prompt_tail: false,
                    }))
                }
                Err(e) => Err(eyre!("deviate failed: {e}")),
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_harness_loop(
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_cwd: &Path,
    effective_non_interactive: bool,
    cli_timeout: Option<u64>,
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    prompt_state: &mut HarnessPromptState,
    repo_root: Option<&Path>,
    use_structured: bool,
    structured_codex_output: Option<&StructuredCodexOutput>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    suppress_stderr_on_success: bool,
    show_checks: bool,
    stream_verbosity: Verbosity,
    verbose_requested: bool,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    initial_materialized: Option<MaterializedHarnessPrompt>,
    term: &Terminal,
) -> Result<i32> {
    const DEFAULT_MAX_RETRIES: u32 = 3;
    let permission_probe =
        WrapperHarnessPermissionProbe::new(provider, base_args.to_vec(), repo_root);
    let mut harness_context = CachedHarnessLoopContext::new(&prompt_state.source_path, repo_root);
    let mut attempt = 1u32;
    let mut initial_materialized = initial_materialized;

    loop {
        harness_context.refresh(&prompt_state.source_path, repo_root);
        let materialized = if let Some(seed) = initial_materialized.take() {
            seed
        } else {
            materialize_harness_prompt(prompt_state, repo_root)?
        };
        let resolve_ctx = harness_context.resolve_context();
        let mut plan = claudine::harness::parse_harness_plan_with_shell(
            &materialized.frontmatter,
            &prompt_state.source_path,
            &resolve_ctx,
            Some(harness_context.shell_options()),
        )?;

        // For inline composition, prepend a system-owned writability
        // pre-check so handler recovery paths can respond to permission
        // failures.
        if matches!(prompt_state.mode, HarnessPromptMode::Inline) {
            plan.pre_checks.insert(
                0,
                claudine::harness::inline_writability_pre_check(&prompt_state.source_path),
            );
        }

        match claudine::harness::evaluate_pre_checks(&plan, Some(&permission_probe), term) {
            Ok(()) => {}
            Err(claudine::harness::HarnessError::PreCheckFailed { ref failures }) => {
                let contexts = claudine::harness::build_validation_failure_context(
                    failures,
                    provider.as_slug(),
                    plan.source_path.as_path(),
                    attempt,
                    None,
                    None,
                );
                let mut next_plan = None;
                for failure_ctx in &contexts {
                    match claudine::harness::resolve_handler(
                        failure_ctx,
                        &plan.handlers,
                        plan.programmatic_handler.as_ref(),
                    ) {
                        Ok(Some(action)) => {
                            next_plan = build_next_attempt_plan(
                                &action,
                                attempt,
                                DEFAULT_MAX_RETRIES,
                                &failure_ctx.message,
                                profile,
                                None,
                                &prompt_state.source_path,
                                repo_root,
                                failure_ctx,
                                term,
                            )?;
                            if next_plan.is_some() {
                                break;
                            }
                        }
                        Ok(None) => {}
                        Err(e) => return Err(eyre!("{e}")),
                    }
                }
                if let Some(plan) = next_plan {
                    attempt = plan.next_attempt;
                    apply_next_attempt_plan(prompt_state, &plan);
                    continue;
                }
                return Err(eyre!(
                    "pre-check validation failed ({} {})",
                    failures.len(),
                    if failures.len() == 1 {
                        "failure"
                    } else {
                        "failures"
                    }
                ));
            }
            Err(e) => return Err(eyre!("{e}")),
        }

        let snapshot = claudine::harness::capture_pre_run_snapshot(&plan)
            .map_err(|e| eyre!("harness snapshot: {e}"))?;
        let launch = build_harness_launch(
            provider,
            profile,
            base_args,
            base_env,
            prompt_state,
            &materialized,
            effective_non_interactive,
            cli_timeout,
            plan.timeout,
        )?;

        let outcome = execute_harness_attempt(
            attempt,
            provider,
            profile,
            binary_path,
            child_cwd,
            &launch,
            prompt_state.mode,
            prompt_state,
            &materialized,
            use_structured,
            structured_codex_output,
            stdout_noise,
            stderr_noise,
            suppress_stderr_on_success,
            show_checks,
            stream_verbosity,
            verbose_requested,
            env_context,
            dispatch_context,
            term,
        )?;

        if outcome.termination == claudine::harness::ProcessTermination::Interrupted {
            return Ok(outcome.exit_code);
        }

        if let Some(failure_event) = claudine::harness::classify_failure(&outcome) {
            let message = match failure_event {
                claudine::harness::FailureEvent::Timeout => {
                    format!("provider timed out (attempt {attempt})")
                }
                claudine::harness::FailureEvent::AgentFailure => {
                    format!(
                        "agent exited with error code {} (attempt {attempt})",
                        outcome.exit_code
                    )
                }
                _ => format!("failure on attempt {attempt}"),
            };
            let ctx = claudine::harness::build_agent_failure_context(
                provider.as_slug(),
                plan.source_path.as_path(),
                failure_event,
                message.clone(),
                attempt,
                outcome.session_id.clone(),
                Some(outcome.clone()),
            );
            match claudine::harness::resolve_handler(
                &ctx,
                &plan.handlers,
                plan.programmatic_handler.as_ref(),
            ) {
                Ok(Some(action)) => {
                    if let Some(next_plan) = build_next_attempt_plan(
                        &action,
                        attempt,
                        DEFAULT_MAX_RETRIES,
                        &ctx.message,
                        profile,
                        outcome.session_id.as_deref(),
                        &prompt_state.source_path,
                        repo_root,
                        &ctx,
                        term,
                    )? {
                        attempt = next_plan.next_attempt;
                        apply_next_attempt_plan(prompt_state, &next_plan);
                        continue;
                    }
                    return Err(eyre!("{message}"));
                }
                Ok(None) => return Err(eyre!("{message}")),
                Err(e) => return Err(eyre!("{e}")),
            }
        }

        // For inline mode, apply closure BEFORE post-checks so that
        // file-state checks (file_changed, frontmatter comparisons, etc.)
        // observe the final rewritten document rather than the pre-closure
        // source file.
        if let Some(closure_plan) = materialized.inline_closure_plan.as_ref() {
            if outcome.exit_code == 0 {
                if let Err(failures) = try_inline_closure(
                    closure_plan,
                    &outcome.final_response,
                    &prompt_state.source_path,
                    child_cwd,
                    show_checks,
                    term,
                ) {
                    let contexts =
                        claudine::harness::build_validation_failure_context(
                            &failures,
                            provider.as_slug(),
                            plan.source_path.as_path(),
                            attempt,
                            outcome.session_id.clone(),
                            Some(outcome.clone()),
                        );

                    let mut next_plan = None;
                    for failure_ctx in &contexts {
                        match claudine::harness::resolve_handler(
                            failure_ctx,
                            &plan.handlers,
                            plan.programmatic_handler.as_ref(),
                        ) {
                            Ok(Some(action)) => {
                                next_plan = build_next_attempt_plan(
                                    &action,
                                    attempt,
                                    DEFAULT_MAX_RETRIES,
                                    &failure_ctx.message,
                                    profile,
                                    outcome.session_id.as_deref(),
                                    &prompt_state.source_path,
                                    repo_root,
                                    failure_ctx,
                                    term,
                                )?;
                                if next_plan.is_some() {
                                    break;
                                }
                            }
                            Ok(None) => {}
                            Err(e) => return Err(eyre!("{e}")),
                        }
                    }

                    if let Some(plan) = next_plan {
                        attempt = plan.next_attempt;
                        apply_next_attempt_plan(prompt_state, &plan);
                        continue;
                    }

                    return Err(eyre!(
                        "inline closure validation failed ({} {})",
                        failures.len(),
                        if failures.len() == 1 {
                            "failure"
                        } else {
                            "failures"
                        }
                    ));
                }
            }
        }

        // Evaluate post-checks. In inline mode this now runs against the
        // post-closure document so file-state checks see the final artifact.
        match claudine::harness::evaluate_post_checks(
            &plan,
            &snapshot,
            &outcome,
            Some(&permission_probe),
            term,
        ) {
            Ok(()) => {
                return Ok(outcome.exit_code);
            }
            Err(claudine::harness::HarnessError::PostCheckFailed { ref failures }) => {
                let contexts = claudine::harness::build_validation_failure_context(
                    failures,
                    provider.as_slug(),
                    plan.source_path.as_path(),
                    attempt,
                    outcome.session_id.clone(),
                    Some(outcome.clone()),
                );

                let mut next_plan = None;
                for failure_ctx in &contexts {
                    match claudine::harness::resolve_handler(
                        failure_ctx,
                        &plan.handlers,
                        plan.programmatic_handler.as_ref(),
                    ) {
                        Ok(Some(action)) => {
                            next_plan = build_next_attempt_plan(
                                &action,
                                attempt,
                                DEFAULT_MAX_RETRIES,
                                &failure_ctx.message,
                                profile,
                                outcome.session_id.as_deref(),
                                &prompt_state.source_path,
                                repo_root,
                                failure_ctx,
                                term,
                            )?;
                            if next_plan.is_some() {
                                break;
                            }
                        }
                        Ok(None) => {}
                        Err(e) => return Err(eyre!("{e}")),
                    }
                }

                if let Some(plan) = next_plan {
                    attempt = plan.next_attempt;
                    apply_next_attempt_plan(prompt_state, &plan);
                    continue;
                }

                return Err(eyre!(
                    "post-check validation failed ({} {})",
                    failures.len(),
                    if failures.len() == 1 {
                        "failure"
                    } else {
                        "failures"
                    }
                ));
            }
            Err(e) => return Err(eyre!("{e}")),
        }
    }
}

/// Apply a resolved handler action and return the next attempt number if the
/// loop should continue, or `None` if the action cannot be applied (e.g.,
/// retry ceiling reached, resume unsupported).
///
/// Returns `Ok(Some(next_attempt))` to continue the harness loop, or
pub(crate) fn resolve_binary_path(
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
pub(crate) fn emit_stream_summary(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    details: &StructuredSummaryDetails,
) {
    emit_stream_summary_inner(summary, profile, env_context, verbosity, verbose, details, None);
}

pub(crate) fn emit_stream_summary_with_context(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    details: &StructuredSummaryDetails,
    context_extra: &HashMap<String, serde_json::Value>,
) {
    emit_stream_summary_inner(
        summary,
        profile,
        env_context,
        verbosity,
        verbose,
        details,
        Some(context_extra),
    );
}

fn emit_stream_summary_inner(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    details: &StructuredSummaryDetails,
    context_extra: Option<&HashMap<String, serde_json::Value>>,
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
            let term = crate::log::terminal();
            let rendered = Prose::new(markup).render(&term);
            eprintln!("{rendered}");
        }
        if let Some(markup) = secondary_markup {
            let term = crate::log::terminal();
            let rendered = Prose::new(markup).render(&term);
            eprintln!("  {rendered}");
        }
    }

    // Write synthetic summary event to JSONL (best-effort)
    if let Some(protocol) = profile.stream_protocol() {
        let meta = claudine::stream::reporting::summary_to_event_meta_with_context(
            summary,
            protocol,
            env_context,
            context_extra,
        );
        if let Err(e) = claudine::stream::reporting::write_summary_event(&meta) {
            tracing::warn!("Failed to write stream summary event: {e}");
        }
    }
}

/// Like `emit_stream_summary` but without the automatic separator logic.
/// Used when the caller manages spacing (e.g. inline composition validation output).
#[allow(dead_code)]
pub(crate) fn emit_stream_summary_no_separator(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    details: &StructuredSummaryDetails,
) {
    emit_stream_summary_no_separator_with_context(
        summary,
        profile,
        env_context,
        verbosity,
        verbose,
        details,
        None,
    );
}

pub(crate) fn emit_stream_summary_no_separator_with_context(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    details: &StructuredSummaryDetails,
    context_extra: Option<&HashMap<String, serde_json::Value>>,
) {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::Renderable;

    if verbosity != Verbosity::Silent
        && let Some(markup) = format_summary_prose(summary)
    {
        let term = crate::log::terminal();
        let rendered = Prose::new(markup).render(&term);
        eprintln!("{rendered}");
    }
    if verbosity != Verbosity::Silent
        && verbose
        && let Some(markup) = format_verbose_summary_details_prose(summary, details)
    {
        let term = crate::log::terminal();
        let rendered = Prose::new(markup).render(&term);
        eprintln!("  {rendered}");
    }

    // Write synthetic summary event to JSONL (best-effort)
    if let Some(protocol) = profile.stream_protocol() {
        let meta = claudine::stream::reporting::summary_to_event_meta_with_context(
            summary,
            protocol,
            env_context,
            context_extra,
        );
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

fn extract_prompt_from_child_args(
    provider: Provider,
    child_args: &[String],
    stdin_seed: Option<&str>,
) -> Option<String> {
    if let Some(seed) = stdin_seed {
        return Some(seed.to_string());
    }

    find_prompt_location(provider, child_args)
        .and_then(|location| match location {
            PromptLocation::Value(index) => child_args.get(index).cloned(),
            PromptLocation::Inline { index, prefix } => child_args
                .get(index)
                .and_then(|value| value.strip_prefix(prefix))
                .map(ToOwned::to_owned),
        })
        .or_else(|| extract_user_prompt(child_args))
}

/// Returns true if a prompt string is present — either as a remaining
/// non-switch arg in `child_args` or via stdin.
fn has_prompt_source(child_args: &[String], stdin_seed: Option<&str>) -> bool {
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
pub(crate) fn resolve_system_prompt(prompt_or_file: &str) -> Result<String> {
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
        // No prompt → no prompt source
        assert!(!has_prompt_source(&[], None));

        // Non-switch arg in child_args → prompt source
        assert!(has_prompt_source(&["fix the bug".to_string()], None));

        // Switch-only args → no prompt source
        assert!(!has_prompt_source(&["--json".to_string()], None));

        // stdin_seed → prompt source
        assert!(has_prompt_source(&[], Some("hello")));
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
    fn live_stream_sink_merges_context_extra_into_dispatch_meta() {
        let recorded: Arc<Mutex<Vec<DispatchEventMeta>>> = Arc::new(Mutex::new(Vec::new()));
        let sink_events = recorded.clone();
        let mut context = HashMap::new();
        context.insert(
            "composition_file_ref".into(),
            serde_json::Value::String("docs/brief.md".into()),
        );
        context.insert(
            "composition_mode".into(),
            serde_json::Value::String("inline".into()),
        );

        let mut sink = LiveStreamSink::with_dispatcher(
            Provider::Codex,
            EnvironmentContext::default(),
            Verbosity::Silent,
            Arc::new(Mutex::new(StructuredSummaryDetails::default())),
            move |_event, meta| {
                sink_events.lock().unwrap().push(meta);
            },
        )
        .with_context_extra(context);

        sink.on_turn_start(&StreamEventMeta::default());

        let metas = recorded.lock().unwrap().clone();
        assert_eq!(
            metas[0].extra["composition_file_ref"],
            serde_json::Value::String("docs/brief.md".into())
        );
        assert_eq!(
            metas[0].extra["composition_mode"],
            serde_json::Value::String("inline".into())
        );
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
