pub(crate) mod catalog_helpers;
pub(crate) mod env;
pub(crate) mod exec;
pub(crate) mod live_semantic_sink;
pub(crate) mod profile;
pub(crate) mod repo_home;
pub(crate) mod section;
pub(crate) mod stream_io;
pub(crate) mod system_prompt;
pub(crate) mod wire_io;

use biscuit_terminal::terminal::Terminal;
use clap::Args;
use claudine::composition::lifecycle::LifecycleSignal;
use claudine::events::{EnvironmentContext, Provider};
use claudine::stream::stderr::Verbosity;
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
use tracing::{debug, info_span};

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
pub(crate) mod selection_ui;
pub(crate) mod sequence;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HarnessPromptMode {
    Passthrough,
    Inline,
    Compose,
}

fn harness_prompt_mode_label(mode: HarnessPromptMode) -> &'static str {
    match mode {
        HarnessPromptMode::Passthrough => "passthrough",
        HarnessPromptMode::Inline => "inline",
        HarnessPromptMode::Compose => "compose",
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HarnessPromptState {
    pub(crate) mode: HarnessPromptMode,
    pub(crate) source_path: PathBuf,
    /// The original file reference string (for reporting).
    pub(crate) original_ref: String,
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
    /// Wire-mode JSON-RPC prompt body, when the provider's prompt
    /// delivery requested transport via [`super::wire_io::run_kimi_wire_session`]
    /// instead of stdin / argv. Mutually exclusive with `stdin_seed` for
    /// the same launch.
    pub(crate) wire_prompt: Option<String>,
    pub(crate) timeout: Option<u64>,
    /// Silence-detection step timeout in seconds, if configured.
    ///
    /// Enforced by the streaming wait loop against `LiveMetrics.last_event_at`.
    /// Dropped (with a stderr warning) for capture and passthrough paths
    /// because those modes have no stream events to observe.
    pub(crate) step_timeout: Option<u64>,
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
    build_harness_shell_options_with_cache(source_path, repo_root, None)
}

/// Build shell approval options, optionally reusing a shared approval
/// cache. Callers like the sequence orchestrator pass a shared cache so
/// that "allow once" approvals from earlier steps carry over to later
/// ones for the duration of the sequence run.
///
/// The interactive approval handler is installed whenever the process
/// can actually prompt — i.e. stdin and stderr are both TTYs. This is
/// independent of whether the spawned agent runs in interactive mode:
/// shell approval happens during preflight, before any agent is launched,
/// so there is no TTY contention. Non-TTY environments (CI, piped input)
/// get no handler and unapproved commands hard-fail as before.
pub(crate) fn build_harness_shell_options_with_cache(
    source_path: &Path,
    repo_root: Option<&Path>,
    shared_cache: Option<claudine::composition::SharedApprovalCache>,
) -> claudine::harness::ShellApprovalOptions {
    let approval_handler: Option<
        std::sync::Arc<dyn darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler>,
    > = if darkmatter_cli::approval::can_prompt_interactively() {
        Some(std::sync::Arc::new(
            darkmatter_cli::approval::CliShellApprovalHandler,
        ))
    } else {
        None
    };
    let mut opts = claudine::harness::ShellApprovalOptions {
        policy_root: harness_policy_root(source_path, repo_root),
        approval_handler,
        ..Default::default()
    };
    if let Some(cache) = shared_cache {
        opts.approval_cache = cache;
    }
    opts
}

#[derive(Debug, Clone)]
pub(crate) struct WrapperHarnessPermissionProbe {
    provider: Provider,
    child_args: Vec<String>,
    repo_root: Option<PathBuf>,
}

impl WrapperHarnessPermissionProbe {
    pub(crate) fn new(
        provider: Provider,
        child_args: Vec<String>,
        repo_root: Option<&Path>,
    ) -> Self {
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
    fn with_shell_options(
        source_path: &Path,
        repo_root: Option<&Path>,
        shell_options: claudine::harness::ShellApprovalOptions,
    ) -> Self {
        Self {
            source_path: source_path.to_path_buf(),
            repo_root: repo_root.map(Path::to_path_buf),
            shell_options,
        }
    }

    fn refresh(&mut self, source_path: &Path, repo_root: Option<&Path>) {
        let repo_root = repo_root.map(Path::to_path_buf);
        if self.source_path != source_path || self.repo_root != repo_root {
            self.source_path = source_path.to_path_buf();
            self.repo_root = repo_root;
            self.shell_options.policy_root =
                harness_policy_root(&self.source_path, self.repo_root.as_deref());
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

    /// Strip the interactive approval handler so subsequent harness-loop
    /// iterations operate in deny-only mode.  Cached and whitelisted
    /// commands still pass; new uncached commands are denied without
    /// prompting.  This enforces the spec contract: "all shell approvals
    /// are resolved before the provider workflow begins."
    fn freeze_shell_approvals(&mut self) {
        self.shell_options.approval_handler = None;
    }
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

/// Build the structured-stream parser builder plus an optional stderr bridge.
///
/// All providers share the same stdout parser construction pattern, but
/// OpenCode additionally wires an [`OpenCodeLogBridge`] into the stderr
/// reader thread so classified `--print-logs` records flow through the
/// same semantic sink as stdout events. When a bridge is returned, the
/// caller is responsible for threading it through
/// [`exec::run_child_stream_semantic`] so the stderr thread can consume
/// classified lines and the final summary can merge the bridge's
/// accumulated diagnostics.
///
/// ## Returns
///
/// * `build_parser` — parser-builder closure passed to
///   [`exec::run_child_stream_semantic`].
/// * `stderr_bridge` — `Some` for OpenCode, `None` otherwise. The bridge
///   owns its own shared state clone so the finalizer closure can merge
///   stderr-derived diagnostics into the summary after the reader threads
///   join.
///
/// [`OpenCodeLogBridge`]: claudine::stream::logs::opencode::OpenCodeLogBridge
fn build_structured_plumbing(
    provider: Provider,
    sink: live_semantic_sink::LiveSemanticSink,
    parser_config: claudine::stream::ParserConfig,
) -> (
    exec::SemanticParserBuilder,
    Option<claudine::stream::logs::StderrBridgeHandle>,
) {
    use claudine::stream::logs::StderrBridgeHandle;
    use claudine::stream::logs::codex::CodexLogBridge;
    use claudine::stream::logs::opencode::{OpenCodeLogBridge, merge_stderr_state_into_summary};
    use claudine::stream::semantic::{ObservedSemanticSink, SharedSemanticSink};
    use std::sync::atomic::AtomicBool;

    if provider == Provider::OpenCode {
        let shared = SharedSemanticSink::new(sink);
        let live_sink_inner = Arc::clone(shared.inner());
        let stdout_seen = Arc::new(AtomicBool::new(false));

        let (early_tx, early_rx) = std::sync::mpsc::channel();
        let bridge =
            OpenCodeLogBridge::new(shared.clone(), Arc::clone(&stdout_seen), Some(early_tx));
        let bridge_state = bridge.shared_state();
        let finalize: claudine::stream::logs::SummaryFinalizer = Box::new(move |summary| {
            merge_stderr_state_into_summary(&bridge_state, summary);
        });
        let stderr_bridge = Some(StderrBridgeHandle {
            bridge: Box::new(bridge),
            finalize,
            early_terminate: Some(early_rx),
        });

        let stdout_sink = ObservedSemanticSink::new(shared, stdout_seen);
        let build_parser: exec::SemanticParserBuilder =
            Box::new(move |output_cb, _reasoning_cb| {
                if let Ok(mut inner) = live_sink_inner.lock() {
                    inner.set_output_text_sink(output_cb);
                }
                claudine::stream::create_semantic_parser(provider, stdout_sink, parser_config)
            });
        (build_parser, stderr_bridge)
    } else if provider == Provider::Codex {
        // Codex emits `tracing-subscriber` records on stderr that we'd
        // rather render inline through the live sink (as an orange
        // BlockQuote) than leak raw to the terminal. Share the sink so the
        // stdout parser and the stderr bridge feed one rendering pipeline.
        let shared = SharedSemanticSink::new(sink);
        let live_sink_inner = Arc::clone(shared.inner());
        let bridge = CodexLogBridge::new(shared.clone());
        let stderr_bridge = Some(StderrBridgeHandle {
            bridge: Box::new(bridge),
            finalize: Box::new(|_summary| {}),
            early_terminate: None,
        });

        let stdout_sink = shared;
        let build_parser: exec::SemanticParserBuilder =
            Box::new(move |output_cb, _reasoning_cb| {
                if let Ok(mut inner) = live_sink_inner.lock() {
                    inner.set_output_text_sink(output_cb);
                }
                claudine::stream::create_semantic_parser(provider, stdout_sink, parser_config)
            });
        (build_parser, stderr_bridge)
    } else {
        let build_parser: exec::SemanticParserBuilder =
            Box::new(move |output_cb, _reasoning_cb| {
                let sink = sink.with_output_text_sink(output_cb);
                claudine::stream::create_semantic_parser(provider, sink, parser_config)
            });
        (build_parser, None)
    }
}

pub(crate) fn wrap_terminal() -> Terminal {
    crate::log::terminal()
}

/// Shared startup detection results for the direct wrap path.
///
/// Populated by a single `sniff::detect_with_plan` call and consumed by
/// three independent downstream structures that would otherwise each
/// trigger their own full repo walk.
pub(crate) struct WrapStartupDetection {
    pub(crate) env_context: EnvironmentContext,
    pub(crate) launch_context: claudine::system_prompt::LaunchContext,
    pub(crate) launch_workspace: env::LaunchWorkspaceContext,
}

/// Run one sniff-based filesystem scan and build every startup context
/// the direct wrap path needs from the shared result.
///
/// On a cold filesystem cache in a large monorepo the previous pipeline
/// walked the tree 3-5 times (once in `detect_environment_fast`, once in
/// `LaunchContext::from_cwd`, and twice inside `build_child_env`). This
/// helper collapses that into a single scan and then builds the three
/// consumer contexts from borrowed data.
pub(crate) fn detect_wrap_startup(cwd: &Path) -> Result<WrapStartupDetection> {
    use sniff::request::*;

    let plan = DetectionPlan::new()
        .base_dir(cwd.to_path_buf())
        .without_os()
        .without_hardware()
        .without_network()
        .filesystem(
            FilesystemRequest::new()
                .git(GitRequest::summary())
                .repo(RepoRequest::structure())
                .without_file_inventory()
                .without_docs()
                .without_formatting(),
        );

    let result = sniff::detect_with_plan(plan)
        .map_err(|e| eyre!("startup detection failed for '{}': {e}", cwd.display()))?;

    let launch_context = claudine::system_prompt::LaunchContext::from_sniff_result(&result, cwd);

    let (git_root, repo) = result
        .filesystem
        .as_ref()
        .map(|f| {
            (
                f.git.as_ref().map(|g| g.repo_root.clone()),
                f.repo.as_ref().cloned(),
            )
        })
        .unwrap_or((None, None));

    let launch_workspace =
        env::launch_workspace_context_from_repo_info(cwd, git_root.as_deref(), repo.as_ref());

    let env_context = claudine::events::environment_context_from_sniff_result(result);

    Ok(WrapStartupDetection {
        env_context,
        launch_context,
        launch_workspace,
    })
}

fn fallback_wrap_startup(cwd: &Path) -> WrapStartupDetection {
    WrapStartupDetection {
        env_context: EnvironmentContext::default(),
        launch_context: claudine::system_prompt::LaunchContext {
            cwd: cwd.to_path_buf(),
            repo_root: None,
            package_area_root: None,
            package_root: None,
        },
        launch_workspace: env::LaunchWorkspaceContext {
            launch_cwd: cwd.to_path_buf(),
            repo_root: None,
            child_cwd: cwd.to_path_buf(),
            package_context: None,
            warnings: Vec::new(),
        },
    }
}

pub(crate) fn switch_process_cwd(child_cwd: &Path) -> Result<()> {
    let current = std::env::current_dir()?;
    if current != child_cwd {
        std::env::set_current_dir(child_cwd)?;
    }
    Ok(())
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

/// Reject retired composition flags that should no longer be forwarded to
/// wrapped providers. Users should migrate to `claudine compose` or
/// `claudine inline-compose`.
fn reject_retired_composition_flags(args: &[String]) -> Result<()> {
    const RETIRED: &[(&str, &str)] = &[
        ("--compose", "claudine compose --<provider> <file>"),
        (
            "--frontmatter-prompt",
            "claudine inline-compose --<provider> <file>",
        ),
        (
            "--prompt-file",
            "the provider CLI directly (claudine compose has different semantics)",
        ),
    ];

    for (flag, replacement) in RETIRED {
        if args
            .iter()
            .any(|a| a == flag || a.starts_with(&format!("{flag}=")))
        {
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
///
/// Boolean flags like `--yolo`, `--interactive`, `--quiet`, `--silent`,
/// `--verbose`, and `--repo` are declared here for clap to parse AND also
/// extracted from the passthrough bucket by `extract_wrapper_flags_from_passthrough`.
/// The two sources are OR-merged so flags work whether placed on either side of
/// the first positional argument. This avoids bug #2.2 (dual-source truth) by
/// keeping clap as the primary parser while the passthrough extractor serves as
/// a fallback for flags that land after `trailing_var_arg` has started capturing.
///
/// The extractor honours the POSIX `--` convention: anything on or after the
/// first `--` separator is treated as opaque agent arguments and is never
/// rewritten by Claudine, even when it collides with a Claudine flag name. See
/// `find_passthrough_dash_boundary` for the detection strategy.
///
/// Unknown flags (belonging to the underlying agent) flow into `passthrough`
/// thanks to `ignore_errors(true)` on wrapper subcommands (see `parse_cli`).
#[derive(Debug, Clone, Args)]
pub struct WrapperArgs {
    /// Print help for this wrapper command.
    #[arg(short, long)]
    pub help: bool,

    /// Enable provider-specific YOLO/auto-approval mode.
    #[arg(short = 'y', long)]
    pub yolo: bool,

    /// Preserve this env var even when it matches sensitive-name filters.
    #[arg(long = "include", value_name = "ENV_NAME")]
    pub include: Vec<String>,

    /// Force interactive mode even when a prompt string is provided.
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,

    /// Open the prompt in an external editor before launching the provider.
    #[arg(long, conflicts_with = "interactive")]
    pub edit: bool,

    /// Override the model used by the provider.
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Set the output format (json, text, stream).
    #[arg(short = 'o', long = "output", value_name = "FORMAT")]
    pub output: Option<String>,

    /// Append a system prompt from a file.
    #[arg(
        long = "append-system-prompt",
        visible_alias = "asp",
        value_name = "FILE",
        conflicts_with = "replace_system_prompt"
    )]
    pub append_system_prompt: Option<String>,

    /// Replace the provider's system prompt with contents from a file.
    #[arg(
        long = "replace-system-prompt",
        visible_alias = "rsp",
        value_name = "FILE",
        conflicts_with = "append_system_prompt"
    )]
    pub replace_system_prompt: Option<String>,

    /// Timeout in seconds (sends SIGTERM then SIGKILL). Only valid in non-interactive mode.
    #[arg(short = 't', long = "timeout", value_name = "SECONDS")]
    pub timeout: Option<u64>,

    /// Step-silence timeout (e.g. `30s`, `5m`). Kills the child when no stream
    /// event is observed for this long. Only valid in non-interactive
    /// structured-stream mode.
    #[arg(long = "step-timeout", value_name = "DURATION")]
    pub step_timeout: Option<String>,

    /// Show what would be executed without launching the child.
    #[arg(long)]
    pub dry_run: bool,

    /// Suppress env details and info messages, but still show the system prompt when set.
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

    /// Emit a performance report to stderr after command completion.
    #[arg(long)]
    pub perf: bool,

    /// Arguments forwarded to the wrapped provider CLI.
    ///
    /// Because wrapper subcommands use `ignore_errors(true)` (see `parse_cli`
    /// in main.rs), unknown flags from the underlying agent CLI land here
    /// instead of causing a clap error.
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
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<()> {
    if args.help {
        print_wrapper_help(provider);
        return Ok(());
    }

    let wrapper_start = std::time::Instant::now();
    let mut perf_collector =
        startup_timings.map(|timings| crate::perf::CommandPerfCollector::new("Wrapper", timings));

    let (code, stderr_capture, model_source) =
        run_provider_wrapper_inner(provider, args, verbose, perf_collector.as_mut())?;

    if code != 0 {
        let term = wrap_terminal();
        let report = crate::output::error_report::AgentErrorReport::from_exit_code_with_source(
            provider,
            code,
            stderr_capture.as_deref(),
            model_source.as_ref(),
        );
        report.render(&term);
    }

    // `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
    // The perf report is always emitted to stderr when requested.
    if let Some(collector) = perf_collector {
        let total = wrapper_start.elapsed();
        let report = collector.into_report(total);
        eprint!("{}", crate::perf::render_perf_report(&report));
    }

    std::process::exit(code);
}

fn maybe_edit_prompt_source(
    prompt_source: profile::PromptSource,
    silent_requested: bool,
) -> Result<Option<profile::PromptSource>> {
    maybe_edit_prompt_source_with(
        prompt_source,
        silent_requested,
        std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
        darkmatter::editor::resolve_editor_command,
        darkmatter::editor::edit_text,
    )
}

fn maybe_edit_prompt_source_with<Resolve, Edit>(
    prompt_source: profile::PromptSource,
    silent_requested: bool,
    terminal_ready: bool,
    resolve_editor: Resolve,
    edit_text_fn: Edit,
) -> Result<Option<profile::PromptSource>>
where
    Resolve: FnOnce() -> std::result::Result<String, darkmatter::editor::EditorError>,
    Edit:
        FnOnce(&str, &str) -> std::result::Result<Option<String>, darkmatter::editor::EditorError>,
{
    if !terminal_ready || matches!(prompt_source, profile::PromptSource::InheritStdin) {
        return Err(eyre!("--edit requires an interactive terminal"));
    }

    let seed = match prompt_source {
        profile::PromptSource::None => String::new(),
        profile::PromptSource::Inline(text) => text,
        profile::PromptSource::InheritStdin => unreachable!("handled by terminal preflight"),
    };

    let editor_command = resolve_editor()?;
    let editor_name = editor_command
        .split_whitespace()
        .next()
        .unwrap_or(editor_command.as_str());

    if !silent_requested {
        log::info(&format!("opening {editor_name} for prompt..."));
    }

    match edit_text_fn(&seed, ".md")? {
        Some(edited_text) => Ok(Some(profile::PromptSource::Inline(edited_text))),
        None => {
            if !silent_requested {
                log::info("prompt empty; aborted");
            }
            Ok(None)
        }
    }
}

fn run_provider_wrapper_inner(
    provider: Provider,
    args: WrapperArgs,
    verbose: u8,
    mut perf_collector: Option<&mut crate::perf::CommandPerfCollector>,
) -> Result<(i32, Option<String>, Option<profile::OpenCodeModelSource>)> {
    let profile = profile::profile_for_provider(provider).ok_or_else(|| {
        eyre!(
            "'{}' cannot be wrapped (it is a VS Code extension)",
            provider
        )
    })?;
    let cwd = std::env::current_dir()?;

    let term = wrap_terminal();

    let binary_path = info_span!(
        "wrapper_binary_resolution",
        provider = %provider,
    )
    .in_scope(|| resolve_binary_path_direct(profile))?;

    let raw_agent_params: Vec<String> = std::env::args().skip(2).collect();
    let mut child_args = args.passthrough.clone();
    let extracted = extract_wrapper_flags_from_passthrough(&mut child_args)?;
    let yolo_requested = args.yolo || extracted.yolo;
    let mut yolo_enabled = yolo_requested;
    let interactive_requested = args.interactive || extracted.interactive;
    let edit_requested = args.edit || extracted.edit;
    let repo_requested = args.repo || extracted.repo;
    let quiet_requested = args.quiet || extracted.quiet;
    let silent_requested = args.silent || extracted.silent;
    let detail_requested = verbose > 0 || extracted.verbose;
    let mut env_overrides: Vec<(String, String)> = Vec::new();
    let mut deferred_warnings: Vec<String> = Vec::new();
    let mut deferred_messages: Vec<String> = Vec::new();

    // One sniff-based scan produces the raw git + repo-structure data that
    // the rest of startup needs. The result is consumed by three separate
    // consumers (EnvironmentContext, LaunchContext, LaunchWorkspaceContext)
    // without ever walking the filesystem again — this avoids the 3-5
    // redundant tree walks the earlier pipeline performed.
    let startup = match detect_wrap_startup(&cwd) {
        Ok(startup) => startup,
        Err(error) => {
            if repo_requested {
                return Err(eyre!(
                    "--repo requires startup repo detection, but startup detection failed: {error}"
                ));
            }
            deferred_warnings.push(format!(
                "startup detection failed; continuing without repo/package context: {error}"
            ));
            fallback_wrap_startup(&cwd)
        }
    };
    let env_context = startup.env_context;
    let launch_context = startup.launch_context;
    let launch_workspace = startup.launch_workspace;

    // In the direct-wrap path the child inherits stdin automatically —
    // we don't need to detect or seed it. Pass false so that a non-tty
    // test environment (or any shell running without an attached terminal)
    // doesn't accidentally classify the session as non-interactive.
    // Composition (Task 14) is where InheritStdin / stdin_seed matters.
    let has_piped_stdin = false;

    // Extract the prompt up-front into a typed PromptSource, leaving
    // child_args free of any prompt characters. Downstream apply_*
    // methods see clean args; prompt_delivery is the only code path
    // that places the prompt back in.
    let (extracted_args, mut prompt_source) =
        profile::extract_prompt_source_from_passthrough(profile, &child_args, has_piped_stdin)?;
    child_args = extracted_args;

    if edit_requested {
        let Some(edited_prompt) = maybe_edit_prompt_source(prompt_source, silent_requested)? else {
            return Ok((0, None, None));
        };
        prompt_source = edited_prompt;
    }

    // Default: non-interactive when a prompt reaches the child, interactive
    // otherwise. --interactive/-i overrides the default back to interactive.
    let has_prompt = prompt_source.has_prompt_or_stdin();
    let non_interactive_requested = if interactive_requested {
        false
    } else {
        has_prompt
    };
    let wrapper_span = info_span!(
        "wrapper_session",
        binary_path = %binary_path.display(),
        has_prompt,
        interactive_requested,
        edit_requested,
        yolo_requested,
        model_override = %args.model.as_deref().unwrap_or(""),
        provider = %provider,
        session_id = tracing::field::Empty,
        child_pid = tracing::field::Empty,
        structured_mode = tracing::field::Empty,
    );
    let _wrapper_guard = wrapper_span.enter();

    // Early check: --timeout + --interactive is always an error
    if args.timeout.is_some() && interactive_requested {
        return Err(eyre!("--timeout cannot be used with --interactive mode"));
    }

    // Early check: --step-timeout + --interactive is always an error
    if args.step_timeout.is_some() && interactive_requested {
        return Err(eyre!(
            "--step-timeout cannot be used with --interactive mode"
        ));
    }

    // clap catches direct `--edit` + `--interactive` conflicts, but once a
    // prompt token starts passthrough capture either flag can arrive from the
    // fallback extractor instead. Keep the merged behavior consistent.
    if edit_requested && interactive_requested {
        return Err(eyre!("--edit cannot be used with --interactive"));
    }

    // Early check: --timeout requires non-interactive mode
    if args.timeout.is_some() && !non_interactive_requested {
        return Err(eyre!(
            "--timeout can only be used in non-interactive mode \
             (provide a prompt or use a composition switch)"
        ));
    }

    // Early check: --step-timeout requires non-interactive mode
    if args.step_timeout.is_some() && !non_interactive_requested {
        return Err(eyre!(
            "--step-timeout can only be used in non-interactive mode \
             (provide a prompt or use a composition switch)"
        ));
    }

    // Parse `--step-timeout DURATION` once via the same parser frontmatter
    // uses so CLI and frontmatter errors share one grammar. The `Path`
    // argument is only used to decorate `HarnessError`; the CLI flag has no
    // source file, so we use a synthetic label.
    let cli_step_timeout_secs: Option<u64> = match args.step_timeout.as_deref() {
        Some(raw) => Some(
            claudine::harness::parse_timeout(raw, std::path::Path::new("<--step-timeout>"))
                .map_err(|e| eyre!("invalid --step-timeout value: {e}"))?
                .as_secs(),
        ),
        None => None,
    };

    // The effective interactivity state is determined solely by the explicit flag.
    let effective_non_interactive = non_interactive_requested;

    profile.reject_direct_yolo(&child_args)?;
    reject_retired_composition_flags(&child_args)?;

    if yolo_requested
        && let Some(warn) = profile.apply_yolo_for_mode(
            &mut child_args,
            &mut env_overrides,
            !non_interactive_requested,
        )?
    {
        deferred_warnings.push(warn);
        // A returned warning means yolo was NOT actually applied for this
        // invocation (e.g. OpenCode interactive mode), so the summary and
        // header badge should reflect the disabled state.
        yolo_enabled = false;
    }
    if yolo_requested && !profile.has_supported_yolo() {
        yolo_enabled = false;
    }

    profile.apply_entrypoint(&mut child_args, non_interactive_requested);

    if non_interactive_requested {
        profile.apply_non_interactive_flags(&mut child_args)?;
    }

    // OpenCode model resolution (replaces apply_non_interactive_defaults +
    // validate_non_interactive_requirements).
    let opencode_model_source: Option<profile::OpenCodeModelSource> =
        if provider == Provider::OpenCode {
            let has_model = env_overrides.iter().any(|(k, _)| k == "MODEL");
            match profile::apply_opencode_model_resolution(
                &mut child_args,
                &mut |k, v| env_overrides.push((k, v)),
                has_model,
                args.model.as_deref(),
                non_interactive_requested,
                &profile::OpenCodeEnvSnapshot::from_system(),
            ) {
                Ok(source) => source,
                Err(_) => {
                    let term = wrap_terminal();
                    let report =
                        crate::output::error_report::AgentErrorReport::no_model_provided(provider);
                    report.render(&term);
                    std::process::exit(1);
                }
            }
        } else {
            None
        };

    // Universal --model flag (non-OpenCode providers, and OpenCode when
    // the user passed --model explicitly but we already handled it above).
    if provider != Provider::OpenCode {
        if let Some(ref model) = args.model
            && let Some(warn) = profile.apply_model(&mut child_args, &mut env_overrides, model)
        {
            deferred_warnings.push(warn);
        }
    } else if let Some(ref model) = args.model
        && !non_interactive_requested
    {
        // Interactive OpenCode with --model: use the standard apply_model path.
        if let Some(warn) = profile.apply_model(&mut child_args, &mut env_overrides, model) {
            deferred_warnings.push(warn);
        }
    }

    // Non-OpenCode providers still use the trait-based validation.
    if provider != Provider::OpenCode && non_interactive_requested {
        profile.validate_non_interactive_requirements(&child_args)?;
    }

    // Universal --output flag
    if let Some(ref output_str) = args.output {
        let format: OutputFormat = output_str.parse().map_err(|e: String| eyre!(e))?;
        if let Some(warn) = profile.apply_output_format(&mut child_args, format) {
            deferred_warnings.push(warn);
        }
    }

    let prompt_display = prompt_source.as_inline().map(|s| s.to_string());
    let interactive_override = interactive_requested && has_prompt;
    let effective_operation = args.operation.clone().or(extracted.operation);

    if let Some(ref op) = effective_operation {
        env_overrides.push(("OPERATION".to_string(), op.clone()));
    }

    if !silent_requested {
        let header_env_plan = env::EnvPlan {
            package_context: launch_workspace.package_context.clone(),
            ..Default::default()
        };

        crate::output::log_wrapper_header(
            profile,
            yolo_enabled,
            effective_non_interactive,
            interactive_override,
            detail_requested,
            repo_requested,
            None,
            false, // not a sequence
            effective_operation.as_deref(),
            prompt_display.as_deref(),
            None, // no compose source hint for direct wrapper
            &header_env_plan,
            &term,
        );
    }

    let sp_args = claudine::system_prompt::SystemPromptArgs {
        append_file: args.append_system_prompt.clone(),
        replace_file: args.replace_system_prompt.clone(),
    };
    let effective_sp = claudine::system_prompt::resolve_and_prepare_for_session(
        &sp_args,
        &launch_context,
        non_interactive_requested,
    )?;

    let mut sp_artifacts: Vec<super::wrap::system_prompt::SystemPromptArtifact> = Vec::new();

    match &effective_sp {
        claudine::system_prompt::EffectiveSystemPrompt::None
        | claudine::system_prompt::EffectiveSystemPrompt::Disabled { .. } => {}
        claudine::system_prompt::EffectiveSystemPrompt::Ready(prepared) => {
            let application =
                profile.apply_system_prompt(prepared, !non_interactive_requested, &cwd)?;
            child_args.extend(application.args);
            env_overrides.extend(
                application
                    .env
                    .into_iter()
                    .map(|(k, v)| (k.to_string_lossy().into(), v.to_string_lossy().into())),
            );
            sp_artifacts = application.artifacts;
            for warn in application.warnings {
                deferred_warnings.push(warn);
            }
        }
    }
    let _ = &sp_artifacts;

    if args.sandbox
        && let Some(warn) = profile.apply_sandbox(&mut child_args)
    {
        deferred_warnings.push(warn);
    }

    let needs_mcp_shadow_home = (args.mcp || !args.mcp_use.is_empty())
        && matches!(provider, Provider::Codex | Provider::Gemini);

    let mut env_plan = env::build_child_env_with_launch(
        profile,
        provider,
        &args.include,
        yolo_enabled,
        !non_interactive_requested,
        &raw_agent_params,
        &env_overrides,
        repo_requested,
        needs_mcp_shadow_home,
        launch_workspace,
    )?;

    if !silent_requested && !quiet_requested {
        use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
        use biscuit_terminal::prelude::Renderable as _;

        let status = Status::from_prose("Starting pre-flight checks".to_string())
            .state(StatusState::Info)
            .theme(StatusTheme::Circular);
        crate::log::message(&status.render(&term));
    }

    if args.timeout.is_some() && !effective_non_interactive {
        return Err(eyre!(
            "--timeout can only be used in non-interactive mode \
             (provide a prompt)"
        ));
    }

    if args.step_timeout.is_some() && !effective_non_interactive {
        return Err(eyre!(
            "--step-timeout can only be used in non-interactive mode \
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
            extract_tags_from_prompt(prompt_source.as_inline(), lex_tags);
        if let Some(ref cleaned) = cleaned_prompt {
            prompt_source = profile::PromptSource::Inline(cleaned.clone());
        }
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

    let child_cwd = env_plan.child_cwd.as_path();

    if effective_non_interactive && !silent_requested {
        log::info(&crate::output::format_launch_directory(child_cwd));
    }

    if let Some(collector) = perf_collector.as_mut() {
        collector.mark_env_setup_complete();
    }

    // --dry-run: print what would be executed and exit
    let sp_display_lines = crate::commands::wrap::system_prompt::describe_effective(&effective_sp);
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
            sp_display_lines.as_deref(),
        );
        if let Some(collector) = perf_collector.as_mut() {
            collector.set_dry_run();
        }
        return Ok((0, None, None));
    }

    switch_process_cwd(child_cwd)?;

    let dispatch_context = HashMap::new();

    // Output verbosity: --silent suppresses everything, --quiet hides env/info preamble
    // but still shows the system prompt when one is active.
    if !silent_requested {
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

            if let Some(ref source) = opencode_model_source {
                use biscuit_terminal::components::status::Status;
                use biscuit_terminal::prelude::Renderable as _;
                let status = Status::from_prose(source.status_markup());
                log::message(&status.render(&term));
            }
        }

        crate::output::log_system_prompt(
            &effective_sp,
            detail_requested,
            silent_requested,
            quiet_requested,
            &term,
        );

        // Blank line to separate preamble from execution output. Keep it aligned with
        // whichever preamble blocks were actually emitted.
        if !quiet_requested
            || matches!(
                effective_sp,
                claudine::system_prompt::EffectiveSystemPrompt::Ready(_)
            )
        {
            log::message("");
        }
    }

    let stdout_noise = if effective_non_interactive {
        profile.stdout_noise_prefixes()
    } else {
        &[]
    };
    // Interactive TUIs (Codex, OpenCode, etc.) must inherit stderr directly.
    // A non-empty stderr filter causes `exec::run_child` to pipe stderr,
    // which flips `isolate_process_group` on and leaves the child in a
    // background pgroup — it then hangs on SIGTTIN when reading the TTY.
    let stderr_noise = if effective_non_interactive {
        profile.stderr_noise_prefixes()
    } else {
        &[]
    };

    // Decide whether to use internal structured stream parsing.
    // Conditions: provider supports it, non-interactive, no explicit output format.
    let use_structured = profile.supports_structured_stream()
        && effective_non_interactive
        && args.output.is_none()
        && !has_explicit_native_output_request(provider, &child_args);
    wrapper_span.record("structured_mode", use_structured);
    let stream_verbosity = structured_verbosity(silent_requested, quiet_requested);

    // `step_timeout` is only enforceable in structured-stream mode because
    // it gates on `LiveMetrics::last_event_at`. Warn and drop it otherwise so
    // the downstream exec path never has to re-check.
    let cli_step_timeout_secs = if !use_structured && cli_step_timeout_secs.is_some() {
        if !silent_requested {
            log::warn(
                "--step-timeout is only enforced in structured-stream mode; \
                 this run does not qualify, so the flag will be ignored",
            );
        }
        None
    } else {
        cli_step_timeout_secs
    };

    if use_structured {
        profile.apply_structured_stream(&mut child_args);
    }

    let structured_codex_output = if use_structured && provider == Provider::Codex {
        Some(StructuredCodexOutput::prepare(&mut child_args))
    } else {
        None
    };

    let mut wire_prompt: Option<String> = None;
    let stdin_seed: Option<String> = if let Some(prompt) = prompt_source.as_inline() {
        let delivery = profile.prompt_delivery(&child_args, prompt, effective_non_interactive)?;
        if let Some(body) = delivery.as_wire_rpc() {
            wire_prompt = Some(body.to_string());
        }
        delivery.apply_to(&mut child_args)
    } else {
        None
    };

    profile::require_prompt_present(profile.binary(), effective_non_interactive, &prompt_source)?;

    if tracing::enabled!(tracing::Level::WARN) {
        profile::validate_argv_flags_before_separator(profile.binary(), &child_args);
    }

    let wrapper_harness = {
        let base_prompt = prompt_source
            .as_inline()
            .map(|s| s.to_string())
            .or_else(|| stdin_seed.clone());
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
                let plan = claudine::harness::parse_harness_plan(
                    &seed.frontmatter,
                    &source_path,
                    &resolve_ctx,
                )
                .map_err(|e| eyre!("{e}"))?;

                // Pre-flight harness shell commands
                let _harness_preflight = claudine::composition::resolve_shell_approvals(
                    None,
                    None,
                    Some(&plan),
                    &shell_options,
                )
                .map_err(|e| eyre!("{e}"))?;

                drop(plan);

                Some((source_path, base_prompt, seed, shell_options))
            } else {
                None
            }
        } else {
            None
        }
    };

    if !silent_requested && !quiet_requested {
        use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
        use biscuit_terminal::prelude::Renderable as _;

        let status = Status::from_prose("Pre-flight checks have passed".to_string())
            .state(StatusState::Success)
            .theme(StatusTheme::Circular);
        crate::log::message(&status.render(&term));
    }

    // Execute the provider. Composition and harness execution are handled by
    // `claudine compose` / `claudine inline-compose` through the wrapper-grade
    // composition executor; the wrapper path handles plain prompt passthrough.
    let (exit_code, stderr_capture) =
        if let Some((source_path, base_prompt, initial_materialized, shell_options)) =
            wrapper_harness
        {
            let mut prompt_state = HarnessPromptState {
                mode: HarnessPromptMode::Passthrough,
                original_ref: source_path.display().to_string(),
                source_path,
                base_prompt: Some(base_prompt),
                overlay: indexmap::IndexMap::new(),
                prompt_tail: Vec::new(),
                next_prompt_override: None,
                next_resume_session_id: None,
            };

            let mut harness_base_args = child_args.clone();
            if !use_structured {
                profile.prepare_captured_output(&mut harness_base_args);
            }

            let source_path_for_lifecycle = prompt_state.source_path.clone();
            let default_lifecycle = claudine::composition::LifecycleConfig::default();
            let default_lifecycle_settings = claudine::events::GlobalSettings::default();
            let default_lifecycle_messaging = claudine::messaging::RuntimeMessagingSettings {
                user: None,
                repo: None,
            };
            let default_lifecycle_ctx = claudine::composition::LifecycleRuntimeContext {
                settings: &default_lifecycle_settings,
                messaging: &default_lifecycle_messaging,
                term: &term,
                source_path: &source_path_for_lifecycle,
                repo_root: env_plan.repo_root.as_deref(),
            };
            let default_lifecycle_emitter = claudine::composition::DefaultLifecycleEmitter;

            let (harness_code, harness_perf) = run_harness_loop(
                provider,
                profile,
                binary_path.as_path(),
                child_cwd,
                effective_non_interactive,
                args.timeout,
                cli_step_timeout_secs,
                &harness_base_args,
                &env_plan.env,
                &mut prompt_state,
                env_plan.repo_root.as_deref(),
                shell_options,
                use_structured,
                structured_codex_output.as_ref(),
                stdout_noise,
                stderr_noise,
                profile.suppress_structured_stderr_on_success(),
                !silent_requested,
                stream_verbosity,
                detail_requested,
                &env_context,
                &dispatch_context,
                Some(initial_materialized),
                &term,
                &default_lifecycle,
                &default_lifecycle_ctx,
                &default_lifecycle_emitter,
                true,
            )?;
            if let (Some(collector), Some(perf)) = (perf_collector.as_mut(), harness_perf) {
                collector.set_agent_perf(perf);
            }
            (harness_code, None)
        } else if use_structured {
            let summary_details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
            let parser_config = claudine::stream::ParserConfig {
                model: args.model.clone(),
            };
            let sink = live_semantic_sink::LiveSemanticSink::with_default_wiring(
                provider,
                env_context.clone(),
                child_cwd,
                stream_verbosity,
                summary_details.clone(),
            )
            .with_context_extra(dispatch_context.clone());
            let live_metrics = sink.live_metrics();
            let stream_output = sink.stream_output();
            // Snapshot the sink's section-stream handle before the sink is
            // moved into the parser closure. Post-stream trailer and
            // Codex-final-stdout emission uses this handle so every section
            // transition shares the same tracker state.
            let section_stream = sink.section_stream();
            let (build_parser, stderr_bridge) =
                build_structured_plumbing(provider, sink, parser_config);
            let mut _spawned = false;
            let stream_result = if let Some(wire_prompt) = wire_prompt.clone() {
                let runtime_context =
                    match claudine::dispatch::DispatchRuntimeContext::load_for_env(&env_context) {
                        Ok(runtime) => runtime,
                        Err(error) => {
                            tracing::warn!(%provider, "failed to preload wire runtime config: {error}");
                            claudine::dispatch::DispatchRuntimeContext::default()
                        }
                    };
                let _ = stderr_bridge;
                wire_io::run_kimi_wire_session(
                    wire_io::WireSessionConfig {
                        binary: binary_path.as_path(),
                        args: &child_args,
                        env: &env_plan.env,
                        cwd: child_cwd,
                        prompt: wire_prompt,
                        timeout: args.timeout,
                        client_name: env!("CARGO_PKG_NAME"),
                        client_version: env!("CARGO_PKG_VERSION"),
                        capabilities: wire_io::WireClientCapabilities::default_for_claudine(),
                        env_context: env_context.clone(),
                    },
                    wire_io::WireSessionWiring {
                        build_parser,
                        stream_output,
                        live_metrics,
                        runtime_context,
                    },
                    &mut _spawned,
                )?
            } else {
                exec::run_child_stream_semantic(
                    binary_path.as_path(),
                    &child_args,
                    &env_plan.env,
                    child_cwd,
                    args.timeout,
                    cli_step_timeout_secs,
                    stderr_noise,
                    profile.suppress_structured_stderr_on_success(),
                    stream_verbosity != Verbosity::Silent,
                    stdin_seed.as_deref(),
                    build_parser,
                    &mut _spawned,
                    live_metrics,
                    stream_output,
                    stderr_bridge,
                    None,
                )?
            };
            let mut summary = stream_result.data;
            let api_duration_ms = summary.duration_ms;
            if let Some(collector) = perf_collector.as_mut() {
                collector.set_agent_perf(stream_result.telemetry.into_agent_perf(api_duration_ms));
            }
            if let Some(ref sid) = summary.session_id {
                wrapper_span.record("session_id", tracing::field::display(sid));
            }
            if let Some(codex_output) = structured_codex_output.as_ref() {
                codex_output.apply_to_summary(&mut summary);
            }
            if provider == Provider::Codex && !summary.assistant_text.is_empty() {
                section_stream.enter_final_stdout();
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
                detail_requested,
                &summary_details.lock().unwrap().clone(),
                Some(&section_stream),
            );

            let stderr_text = summary.stderr_text.clone();
            (summary.exit_code, stderr_text)
        } else {
            // Legacy path: forward I/O to terminal
            let mut _spawned = false;
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
                &mut _spawned,
            )?;
            if let Some(collector) = perf_collector.as_mut() {
                collector.set_agent_perf(result.telemetry.into_agent_perf(None));
            }
            (result.data, None)
        };

    // MCP injector cleanup: remove temp files written during injection
    if let Some((injector, injection_result)) = mcp_cleanup
        && let Err(e) = injector.cleanup(&injection_result)
    {
        tracing::warn!("MCP injector cleanup failed: {e}");
    }

    Ok((exit_code, stderr_capture, opencode_model_source))
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
            "--json" | "--verbose" if !resume_args.iter().any(|arg| arg == &base_args[index]) => {
                resume_args.push(base_args[index].clone());
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
    let (composed, _report) = source_markdown.compose_with(options)?;

    Ok(MaterializedHarnessPrompt {
        frontmatter: frontmatter_map_to_value(composed.frontmatter()),
        prompt,
        env_overrides: Vec::new(),
        inline_closure_plan: None,
    })
}

fn find_wrapper_harness_source(
    provider: Provider,
    repo_root: Option<&Path>,
    cwd: &Path,
) -> Option<PathBuf> {
    let agent = claudine::agents::agent_for(provider);
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
            let (composed, _report) = effective_markdown.compose_with(options)?;
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
            let (composed, _report) = effective_markdown.compose_with(options)?;
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
            let prepared = claudine::composition::prepare_inline(
                &source,
                claudine::composition::PrepareOptions::default(),
            )?;
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

/// Resolved timeouts for a single harness attempt.
///
/// Combines optional CLI overrides with frontmatter-declared values using the
/// standard "CLI wins" precedence rule. Each field is an `Option<u64>` of
/// seconds so the wait loop can skip enforcement when neither source supplies
/// a value.
#[derive(Debug, Clone, Copy, Default)]
struct LaunchTimeouts {
    timeout: Option<u64>,
    step_timeout: Option<u64>,
}

impl LaunchTimeouts {
    fn timeout_secs_for_span(self) -> u64 {
        self.timeout.unwrap_or(0)
    }

    fn step_timeout_secs_for_span(self) -> u64 {
        self.step_timeout.unwrap_or(0)
    }
}

fn resolve_launch_timeouts(
    cli_timeout: Option<u64>,
    plan_timeout: Option<std::time::Duration>,
    cli_step_timeout: Option<u64>,
    plan_step_timeout: Option<std::time::Duration>,
) -> LaunchTimeouts {
    LaunchTimeouts {
        timeout: cli_timeout.or_else(|| plan_timeout.map(|timeout| timeout.as_secs())),
        step_timeout: cli_step_timeout
            .or_else(|| plan_step_timeout.map(|timeout| timeout.as_secs())),
    }
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
    cli_step_timeout: Option<u64>,
    plan_step_timeout: Option<std::time::Duration>,
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
    let prompt_source = profile::PromptSource::Inline(prompt.clone());
    let delivery = profile.prompt_delivery(&args, &prompt, effective_non_interactive)?;
    let wire_prompt = delivery.as_wire_rpc().map(str::to_string);
    let stdin_seed = delivery.apply_to(&mut args);
    profile::require_prompt_present(profile.binary(), effective_non_interactive, &prompt_source)?;

    let mut env = base_env.clone();
    for (key, value) in &materialized.env_overrides {
        env.insert(key.clone().into(), value.clone().into());
    }

    let timeouts = resolve_launch_timeouts(
        cli_timeout,
        plan_timeout,
        cli_step_timeout,
        plan_step_timeout,
    );

    Ok(AttemptLaunch {
        args,
        env,
        stdin_seed,
        wire_prompt,
        timeout: timeouts.timeout,
        step_timeout: timeouts.step_timeout,
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
    detail_requested: bool,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    term: &Terminal,
    child_spawned: &mut bool,
    prompt_timing: Option<claudine::stream::prompt_timing::PromptTimingContext>,
) -> Result<(
    claudine::harness::AttemptOutcome,
    Option<crate::perf::AgentExecutionPerf>,
)> {
    let _attempt_span = info_span!(
        "harness_attempt",
        provider = %provider,
        attempt,
        prompt_mode = harness_prompt_mode_label(prompt_mode),
        source_path = %prompt_state.source_path.display(),
        use_structured,
    )
    .entered();
    // Step-silence enforcement requires live `SemanticEvent` ticks, which only
    // exist in the structured-stream path. Capture and passthrough attempts
    // drop the value with a warning rather than silently ignoring it.
    let launch = if !use_structured && launch.step_timeout.is_some() {
        use biscuit_terminal::components::renderable::Renderable;
        use biscuit_terminal::components::status::{Status, StatusState};
        let rendered = Status::new(
            "step_timeout is only enforced in structured-stream mode; \
             ignoring for this capture/passthrough attempt"
                .to_string(),
        )
        .state(StatusState::Warning)
        .render(term);
        eprintln!("{rendered}");
        AttemptLaunch {
            step_timeout: None,
            ..launch.clone()
        }
    } else {
        launch.clone()
    };
    let launch = &launch;
    let (exit_code, termination, session_id, final_response, stderr_text, perf) = if use_structured
    {
        let summary_details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
        let parser_config = claudine::stream::ParserConfig::default();
        let sink = live_semantic_sink::LiveSemanticSink::with_default_wiring(
            provider,
            env_context.clone(),
            child_cwd,
            stream_verbosity,
            summary_details.clone(),
        )
        .with_context_extra(dispatch_context.clone());
        let live_metrics = sink.live_metrics();
        let stream_output = sink.stream_output();
        let section_stream = sink.section_stream();
        let (build_parser, stderr_bridge) =
            build_structured_plumbing(provider, sink, parser_config);
        let stream_result = if let Some(wire_prompt) = launch.wire_prompt.clone() {
            let runtime_context =
                match claudine::dispatch::DispatchRuntimeContext::load_for_env(env_context) {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        tracing::warn!(%provider, "failed to preload wire runtime config: {error}");
                        claudine::dispatch::DispatchRuntimeContext::default()
                    }
                };
            let _ = stderr_bridge;
            let _ = prompt_timing;
            wire_io::run_kimi_wire_session(
                wire_io::WireSessionConfig {
                    binary: binary_path,
                    args: &launch.args,
                    env: &launch.env,
                    cwd: child_cwd,
                    prompt: wire_prompt,
                    timeout: launch.timeout,
                    client_name: env!("CARGO_PKG_NAME"),
                    client_version: env!("CARGO_PKG_VERSION"),
                    capabilities: wire_io::WireClientCapabilities::default_for_claudine(),
                    env_context: env_context.clone(),
                },
                wire_io::WireSessionWiring {
                    build_parser,
                    stream_output,
                    live_metrics,
                    runtime_context,
                },
                child_spawned,
            )?
        } else {
            exec::run_child_stream_semantic(
                binary_path,
                &launch.args,
                &launch.env,
                child_cwd,
                launch.timeout,
                launch.step_timeout,
                stderr_noise,
                suppress_stderr_on_success,
                stream_verbosity != Verbosity::Silent,
                launch.stdin_seed.as_deref(),
                build_parser,
                child_spawned,
                live_metrics,
                stream_output,
                stderr_bridge,
                prompt_timing,
            )?
        };
        let api_duration_ms = stream_result.data.duration_ms;
        let perf = Some(stream_result.telemetry.into_agent_perf(api_duration_ms));
        let termination = stream_result.termination;
        let mut summary = stream_result.data;
        if let Some(codex_output) = structured_codex_output {
            codex_output.apply_to_summary(&mut summary);
        }
        if provider == Provider::Codex && !summary.assistant_text.is_empty() {
            section_stream.enter_final_stdout();
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
            detail_requested,
            &summary_details.lock().unwrap().clone(),
            Some(&section_stream),
        );

        (
            summary.exit_code,
            termination,
            summary.session_id.clone(),
            summary.assistant_text.clone(),
            summary.stderr_text.clone(),
            perf,
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
            child_spawned,
        )?;
        let perf = Some(capture.telemetry.into_agent_perf(None));
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
            perf,
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

    Ok((
        claudine::harness::AttemptOutcome {
            attempt,
            session_id,
            final_response,
            exit_code,
            termination,
            stderr_text,
        },
        perf,
    ))
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
    use claudine::harness::{FailurePhase, ValidationEvent, ValidationFailure, ValidationRuleId};

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

    // Read post-run frontmatter for comparison (best-effort)
    let post_run_fm = std::fs::read_to_string(source_path).ok().map(|text| {
        let md: darkmatter::markdown::Markdown = text.into();
        md.frontmatter().as_map().clone()
    });

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match claudine::composition::closure::apply_inline_closure(
        closure_plan,
        &replacement_body,
        source_path,
        &today,
        post_run_fm.as_ref(),
    ) {
        Ok(result) => {
            if show_checks {
                use biscuit_terminal::components::status::{Status, StatusState};
                use biscuit_terminal::prelude::Renderable;

                log::message(&crate::output::fm_check_ok(
                    "Applied the captured replacement body to the target document",
                    term,
                ));
                log::message(&crate::output::fm_check_ok(
                    "Preserved original frontmatter and updated <bold>last_updated</bold>",
                    term,
                ));

                for key in &result.new_properties {
                    log::message(&crate::output::fm_check_ok(
                        &format!("Merged new frontmatter property <bold>\"{key}\"</bold>"),
                        term,
                    ));
                }

                for key in &result.reverted_properties {
                    let status = Status::from_prose(format!(
                        "Agent modified frontmatter property <b>\"{key}\"</b> — reverted to original value"
                    ))
                    .state(StatusState::Warning);
                    log::message(&status.render(term));
                }
            }
            Ok(())
        }
        Err(error) => {
            let is_unchanged = error.to_string().contains("unchanged");
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
        // Update the reporting reference so status output reflects the
        // redirected file rather than the original.
        state.original_ref = path.display().to_string();
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

/// Try to resolve a handler for each failure context and build a recovery
/// plan. Returns `Some(plan)` on the first successful resolution, or `None`
/// if no handler produced an actionable plan.
///
/// The handler-engagement banner is emitted exactly once, only after a
/// concrete `NextAttemptPlan` has been produced.
#[allow(clippy::too_many_arguments)]
fn try_resolve_handler(
    contexts: &[claudine::harness::FailureContext],
    plan: &claudine::harness::HarnessPlan,
    attempt: u32,
    default_max_retries: u32,
    profile: &dyn WrapperProfile,
    session_id: Option<&str>,
    source_path: &Path,
    repo_root: Option<&Path>,
    show_checks: bool,
    term: &Terminal,
) -> Result<Option<NextAttemptPlan>> {
    let _handler_span = info_span!(
        "harness_handler_resolution",
        attempt,
        failure_count = contexts.len(),
        source_path = %source_path.display(),
    )
    .entered();
    for failure_ctx in contexts {
        match claudine::harness::resolve_handler(
            failure_ctx,
            &plan.handlers,
            plan.programmatic_handler.as_ref(),
        ) {
            Ok(Some(action)) => {
                if let Some(next_plan) = build_next_attempt_plan(
                    &action,
                    attempt,
                    default_max_retries,
                    &failure_ctx.message,
                    profile,
                    session_id,
                    source_path,
                    repo_root,
                    failure_ctx,
                    term,
                )? {
                    // Emit the engagement banner exactly once, only when
                    // a concrete recovery plan has been produced.
                    if show_checks {
                        claudine::harness::report::report_handler_engagement(
                            &source_path.display().to_string(),
                            term,
                        );
                    }
                    return Ok(Some(next_plan));
                }
            }
            Ok(None) => {}
            Err(e) => return Err(eyre!("{e}")),
        }
    }
    Ok(None)
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
    let _decision_span = info_span!(
        "harness_handler_decision",
        current_attempt,
        action = %handler_action_name(action),
        source_path = %source_path.display(),
    )
    .entered();

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
            debug!(
                next_attempt = current_attempt + 1,
                "Handler requested retry"
            );
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
            debug!(
                next_attempt = current_attempt + 1,
                "Handler requested resume"
            );
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
            debug!(
                next_attempt = current_attempt + 1,
                redirect_source = %redirect_source.display(),
                "Handler requested redirect"
            );
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
                    debug!(
                        next_attempt = current_attempt + 1,
                        exit_code = deviate_exit,
                        "Handler requested deviate command"
                    );
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

fn handler_action_name(action: &claudine::harness::HandlerAction) -> &'static str {
    match action {
        claudine::harness::HandlerAction::Retry { .. } => "retry",
        claudine::harness::HandlerAction::Resume { .. } => "resume",
        claudine::harness::HandlerAction::Redirect { .. } => "redirect",
        claudine::harness::HandlerAction::Deviate { .. } => "deviate",
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_assignments)]
pub(crate) fn run_harness_loop(
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_cwd: &Path,
    effective_non_interactive: bool,
    cli_timeout: Option<u64>,
    cli_step_timeout: Option<u64>,
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    prompt_state: &mut HarnessPromptState,
    repo_root: Option<&Path>,
    shell_options: claudine::harness::ShellApprovalOptions,
    use_structured: bool,
    structured_codex_output: Option<&StructuredCodexOutput>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    suppress_stderr_on_success: bool,
    show_checks: bool,
    stream_verbosity: Verbosity,
    detail_requested: bool,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    initial_materialized: Option<MaterializedHarnessPrompt>,
    term: &Terminal,
    lifecycle: &claudine::composition::LifecycleConfig,
    lifecycle_ctx: &claudine::composition::LifecycleRuntimeContext<'_>,
    lifecycle_emitter: &dyn claudine::composition::LifecycleEmitter,
    // When `true`, every structured-stream attempt in the harness loop
    // emits the prompt-scoped timing header and — if the parsed plan
    // carries `timeout_warn` / `step_timeout_warn` — their fire-once
    // warning lines. Wrapper passthrough callers with no prompt file
    // pass `false` to suppress the header entirely; composition callers
    // pass `true`.
    emit_prompt_timing: bool,
) -> Result<(i32, Option<crate::perf::AgentExecutionPerf>)> {
    const DEFAULT_MAX_RETRIES: u32 = 3;
    let mut guard =
        claudine::composition::LifecycleRunGuard::new(lifecycle, lifecycle_ctx, lifecycle_emitter);
    let permission_probe =
        WrapperHarnessPermissionProbe::new(provider, base_args.to_vec(), repo_root);
    let mut harness_context = CachedHarnessLoopContext::with_shell_options(
        &prompt_state.source_path,
        repo_root,
        shell_options,
    );
    let mut attempt = 1u32;
    let mut initial_materialized = initial_materialized;
    let mut harness_perf: Option<crate::perf::AgentExecutionPerf> = None;
    let mut _harness_attempts: usize = 0;

    loop {
        let _attempt_cycle_span = info_span!(
            "harness_attempt_cycle",
            provider = %provider,
            attempt,
            prompt_mode = harness_prompt_mode_label(prompt_state.mode),
            source_path = %prompt_state.source_path.display(),
        )
        .entered();
        harness_context.refresh(&prompt_state.source_path, repo_root);
        let materialized = if let Some(seed) = initial_materialized.take() {
            seed
        } else {
            info_span!(
                "harness_materialize_prompt",
                attempt,
                source_path = %prompt_state.source_path.display(),
            )
            .in_scope(|| materialize_harness_prompt(prompt_state, repo_root))
            .map_err(|e| guard.emit_blocked_or_err(e))?
        };
        let resolve_ctx = harness_context.resolve_context();
        let mut plan = info_span!(
            "harness_plan_parse",
            attempt,
            source_path = %prompt_state.source_path.display(),
        )
        .in_scope(|| {
            claudine::harness::parse_harness_plan(
                &materialized.frontmatter,
                &prompt_state.source_path,
                &resolve_ctx,
            )
        })
        .map_err(|e| guard.emit_blocked_or_err(e))?;

        // Source-file existence reporting
        if show_checks {
            claudine::harness::report::report_source_file(
                &prompt_state.original_ref,
                &prompt_state.source_path,
                term,
            );
        }
        if !prompt_state.source_path.exists() {
            if show_checks {
                claudine::harness::report::report_unhandled_failure(
                    "source file does not exist — cannot proceed",
                    term,
                );
            }
            guard.emit_blocked_or_failure();
            return Err(eyre!(
                "source file does not exist: {}",
                prompt_state.source_path.display()
            ));
        }

        // For inline composition, prepend a system-owned writability
        // pre-check so handler recovery paths can respond to permission
        // failures.
        if matches!(prompt_state.mode, HarnessPromptMode::Inline) {
            plan.pre_checks.insert(
                0,
                claudine::harness::inline_writability_pre_check(&prompt_state.source_path),
            );
        }

        // Shell audit preflight.
        //
        // Composition flows (Compose/Inline) preflight all shell commands
        // before the provider starts — template directives during composition
        // and harness commands in execute_composition_request.  The per-
        // attempt audit below is redundant for those modes because:
        //
        //   1. source_text is None, so source-page ::shell directives are
        //      excluded (they were discovered via Darkmatter's graph walker
        //      during composition, which respects ::block when="false").
        //   2. Harness commands were approved and cached during the
        //      composition preflight pass.
        //   3. The approval handler is frozen after attempt 1, so no new
        //      interactive prompts are possible.
        //
        // Only Passthrough mode needs the per-attempt audit because it reads
        // raw source text and the source file may change between
        // redirect/retry iterations.
        if matches!(prompt_state.mode, HarnessPromptMode::Passthrough) {
            let source_text = std::fs::read_to_string(&prompt_state.source_path).ok();

            let auditable =
                claudine::harness::collect_auditable_commands(&plan, source_text.as_deref())?;

            let audit_report = info_span!(
                "harness_shell_audit",
                attempt,
                command_count = auditable.len(),
            )
            .in_scope(|| {
                claudine::harness::audit_shell_commands(&auditable, harness_context.shell_options())
            });

            if show_checks {
                claudine::harness::report::report_shell_audit_header(
                    audit_report.outcomes.len(),
                    term,
                );
                claudine::harness::report::report_shell_audit_outcomes(&audit_report, term);
            }

            if !audit_report.all_passed() {
                let failed = audit_report.failures();
                let (source_failures, harness_failures): (Vec<_>, Vec<_>) =
                    failed.into_iter().partition(|o| {
                        matches!(
                            o.command.source,
                            claudine::harness::AuditedCommandSource::ComposeSourceLine { .. }
                        )
                    });

                // Source-page ::shell failures are terminal in v1 — no recovery.
                if !source_failures.is_empty() {
                    if show_checks {
                        claudine::harness::report::report_unhandled_failure(
                            "shell audit failed for source-page directives — cannot proceed",
                            term,
                        );
                    }
                    guard.emit_blocked_or_failure();
                    return Err(eyre!(
                        "shell audit failed: {} denied directive(s) in source page",
                        source_failures.len()
                    ));
                }

                // Non-source failures flow through handler resolution.
                if !harness_failures.is_empty() {
                    let contexts = claudine::harness::build_audit_failure_context(
                        &harness_failures,
                        provider.as_slug(),
                        plan.source_path.as_path(),
                        attempt,
                    );
                    if let Some(next_plan) = try_resolve_handler(
                        &contexts,
                        &plan,
                        attempt,
                        DEFAULT_MAX_RETRIES,
                        profile,
                        None,
                        &prompt_state.source_path,
                        repo_root,
                        show_checks,
                        term,
                    )? {
                        attempt = next_plan.next_attempt;
                        continue;
                    }

                    let msg = format!(
                        "shell audit failed: {} command(s) denied. \
                         No handler available to resolve.",
                        harness_failures.len()
                    );
                    if show_checks {
                        claudine::harness::report::report_unhandled_failure(&msg, term);
                    }
                    guard.emit_blocked_or_failure();
                    return Err(eyre!("shell audit failed"));
                }
            }
        }

        // Composition flows resolved all shell approvals during preflight.
        // Freeze the approval set so redirect/retry iterations cannot
        // trigger new interactive prompts — only cached/whitelisted
        // commands pass; new uncached commands are denied.  Passthrough
        // mode has no prior preflight so its handler stays active.
        if attempt == 1 && !matches!(prompt_state.mode, HarnessPromptMode::Passthrough) {
            harness_context.freeze_shell_approvals();
        }

        let pre_report = info_span!(
            "harness_pre_validation",
            attempt,
            rule_count = plan.pre_checks.len(),
        )
        .in_scope(|| claudine::harness::evaluate_pre_checks(&plan, Some(&permission_probe)));

        if show_checks {
            claudine::harness::report::report_phase_discovery(
                claudine::harness::FailurePhase::PreCheck,
                pre_report.count(),
                term,
            );
            claudine::harness::report::report_check_outcomes(&pre_report, term);
        }

        if !pre_report.all_passed() {
            let failures = pre_report.failures();
            let contexts = claudine::harness::build_validation_failure_context(
                &failures,
                provider.as_slug(),
                plan.source_path.as_path(),
                attempt,
                None,
                None,
            );
            if let Some(next_plan) = try_resolve_handler(
                &contexts,
                &plan,
                attempt,
                DEFAULT_MAX_RETRIES,
                profile,
                None,
                &prompt_state.source_path,
                repo_root,
                show_checks,
                term,
            )? {
                attempt = next_plan.next_attempt;
                apply_next_attempt_plan(prompt_state, &next_plan);
                continue;
            }
            let fail_msg = format!(
                "pre-check validation failed ({} {})",
                failures.len(),
                if failures.len() == 1 {
                    "failure"
                } else {
                    "failures"
                }
            );
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&fail_msg, term);
            }
            guard.emit_blocked_or_failure();
            return Err(eyre!("{fail_msg}"));
        }

        // Emit start lifecycle signal before the first provider launch.
        guard.emit_start_once();

        let snapshot = info_span!(
            "harness_pre_snapshot",
            attempt,
            rule_count = plan.post_checks.len(),
        )
        .in_scope(|| claudine::harness::capture_pre_run_snapshot(&plan))
        .map_err(|e| eyre!("harness snapshot: {e}"))?;
        let resolved_timeouts = resolve_launch_timeouts(
            cli_timeout,
            plan.timeout,
            cli_step_timeout,
            plan.step_timeout,
        );
        let launch = info_span!(
            "harness_launch_plan",
            attempt,
            timeout_secs = resolved_timeouts.timeout_secs_for_span(),
            step_timeout_secs = resolved_timeouts.step_timeout_secs_for_span(),
        )
        .in_scope(|| {
            build_harness_launch(
                provider,
                profile,
                base_args,
                base_env,
                prompt_state,
                &materialized,
                effective_non_interactive,
                cli_timeout,
                plan.timeout,
                cli_step_timeout,
                plan.step_timeout,
            )
        })?;

        // Build the prompt-scoped timing context for this attempt. The
        // warn thresholds are re-read from each parsed plan so a
        // handler that redirects to a different source document picks
        // up the replacement document's warn values, not the original's.
        let prompt_timing = if emit_prompt_timing {
            Some(
                crate::commands::wrap::composition::build_prompt_timing_context(
                    &prompt_state.source_path,
                    repo_root,
                    plan.timeout_warn,
                    plan.step_timeout_warn,
                ),
            )
        } else {
            None
        };

        let mut child_spawned = false;
        let attempt_result = execute_harness_attempt(
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
            detail_requested,
            env_context,
            dispatch_context,
            term,
            &mut child_spawned,
            prompt_timing,
        );

        // Mark launched as soon as spawn succeeded — before propagating
        // any post-spawn error — so the guard correctly classifies
        // subsequent failures as `Failure` rather than `Blocked`.
        if child_spawned {
            guard.mark_provider_launched();
        }
        let (outcome, perf) = attempt_result?;
        if let Some(p) = perf {
            _harness_attempts += 1;
            match harness_perf.as_mut() {
                Some(acc) => {
                    acc.launches += p.launches;
                    acc.total_elapsed += p.total_elapsed;
                    if acc.first_response_latency.is_none() && p.first_response_latency.is_some() {
                        acc.first_response_latency = p.first_response_latency;
                    }
                    if let Some(api) = p.provider_api_duration {
                        acc.provider_api_duration = Some(
                            acc.provider_api_duration
                                .unwrap_or(std::time::Duration::ZERO)
                                + api,
                        );
                    }
                }
                None => {
                    harness_perf = Some(p);
                }
            }
        }

        if outcome.termination == claudine::harness::ProcessTermination::Interrupted {
            // Surface the interrupt to the user before we let the guard
            // close: without this the wrapper would silently return 130
            // and the operator has no feedback that Claudine noticed.
            eprintln!("{}", crate::output::format_user_interrupt_status());
            guard.emit_terminal(LifecycleSignal::Failure);
            return Ok((outcome.exit_code, harness_perf));
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
            if let Some(next_plan) = try_resolve_handler(
                &[ctx],
                &plan,
                attempt,
                DEFAULT_MAX_RETRIES,
                profile,
                outcome.session_id.as_deref(),
                &prompt_state.source_path,
                repo_root,
                show_checks,
                term,
            )? {
                attempt = next_plan.next_attempt;
                apply_next_attempt_plan(prompt_state, &next_plan);
                continue;
            }
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&message, term);
            }
            guard.emit_terminal(LifecycleSignal::Failure);
            return Err(eyre!("{message}"));
        }

        // For inline mode, apply closure BEFORE post-checks so that
        // file-state checks (file_changed, frontmatter comparisons, etc.)
        // observe the final rewritten document rather than the pre-closure
        // source file.
        if let Some(closure_plan) = materialized.inline_closure_plan.as_ref()
            && outcome.exit_code == 0
            && let Err(failures) = try_inline_closure(
                closure_plan,
                &outcome.final_response,
                &prompt_state.source_path,
                child_cwd,
                show_checks,
                term,
            )
        {
            let contexts = claudine::harness::build_validation_failure_context(
                &failures,
                provider.as_slug(),
                plan.source_path.as_path(),
                attempt,
                outcome.session_id.clone(),
                Some(outcome.clone()),
            );
            if let Some(next_plan) = try_resolve_handler(
                &contexts,
                &plan,
                attempt,
                DEFAULT_MAX_RETRIES,
                profile,
                outcome.session_id.as_deref(),
                &prompt_state.source_path,
                repo_root,
                show_checks,
                term,
            )? {
                attempt = next_plan.next_attempt;
                apply_next_attempt_plan(prompt_state, &next_plan);
                continue;
            }
            let fail_msg = format!(
                "inline closure validation failed ({} {})",
                failures.len(),
                if failures.len() == 1 {
                    "failure"
                } else {
                    "failures"
                }
            );
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&fail_msg, term);
            }
            guard.emit_terminal(LifecycleSignal::Failure);
            return Err(eyre!("{fail_msg}"));
        }

        // Evaluate post-checks. In inline mode this now runs against the
        // post-closure document so file-state checks see the final artifact.
        let post_report = info_span!(
            "harness_post_validation",
            attempt,
            rule_count = plan.post_checks.len(),
        )
        .in_scope(|| {
            claudine::harness::evaluate_post_checks(
                &plan,
                &snapshot,
                &outcome,
                Some(&permission_probe),
            )
        });

        if show_checks {
            claudine::harness::report::report_phase_discovery(
                claudine::harness::FailurePhase::PostCheck,
                post_report.count(),
                term,
            );
            claudine::harness::report::report_check_outcomes(&post_report, term);
        }

        if post_report.all_passed() {
            guard.emit_terminal(LifecycleSignal::Success);
            return Ok((outcome.exit_code, harness_perf));
        }

        let failures = post_report.failures();
        let contexts = claudine::harness::build_validation_failure_context(
            &failures,
            provider.as_slug(),
            plan.source_path.as_path(),
            attempt,
            outcome.session_id.clone(),
            Some(outcome.clone()),
        );
        if let Some(next_plan) = try_resolve_handler(
            &contexts,
            &plan,
            attempt,
            DEFAULT_MAX_RETRIES,
            profile,
            outcome.session_id.as_deref(),
            &prompt_state.source_path,
            repo_root,
            show_checks,
            term,
        )? {
            attempt = next_plan.next_attempt;
            apply_next_attempt_plan(prompt_state, &next_plan);
            continue;
        }
        let fail_msg = format!(
            "post-check validation failed ({} {})",
            failures.len(),
            if failures.len() == 1 {
                "failure"
            } else {
                "failures"
            }
        );
        if show_checks {
            claudine::harness::report::report_unhandled_failure(&fail_msg, term);
        }
        guard.emit_terminal(LifecycleSignal::Failure);
        return Err(eyre!("{fail_msg}"));
    }
}

pub(crate) fn resolve_binary_path(
    profile: &dyn WrapperProfile,
    clients: &InstalledAiClients,
) -> Result<PathBuf> {
    let ai_cli = profile.provider().sniff_ai_cli();
    clients
        .path(ai_cli)
        .ok_or_else(|| binary_missing_error(profile))
}

/// Resolve the child binary path directly via `which`, without scanning the
/// entire set of known AI CLIs. Used on the hot path of the direct wrapper
/// so we don't pay for a full PATH walk over ~9 binaries when only one is
/// needed.
pub(crate) fn resolve_binary_path_direct(profile: &dyn WrapperProfile) -> Result<PathBuf> {
    which::which(profile.binary()).map_err(|_| binary_missing_error(profile))
}

fn binary_missing_error(profile: &dyn WrapperProfile) -> color_eyre::eyre::Error {
    eyre!(
        "cannot run wrapped {} session because '{}' is not installed or not on PATH (docs: {})",
        profile.provider(),
        profile.binary(),
        profile.provider().docs_url()
    )
}

#[allow(dead_code)]
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
///
/// When a [`SectionStream`](live_semantic_sink::SectionStream) handle is
/// supplied, every trailer line is routed through it as
/// [`Section::TrailerMetadata`] so the section-separator blank between the
/// final stdout and the trailer is inserted exactly once (and only when
/// the prior section actually emitted non-blank content). When the handle
/// is absent (legacy / test call sites), emission falls back to plain
/// `eprintln!`.
pub(crate) struct StreamSummaryContext<'a> {
    summary: &'a claudine::stream::summary::StreamExecutionSummary,
    profile: &'a dyn WrapperProfile,
    env_context: &'a EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    details: &'a StructuredSummaryDetails,
    section_stream: Option<&'a super::wrap::section::SectionStream>,
}

pub(crate) fn emit_stream_summary(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    details: &StructuredSummaryDetails,
    section_stream: Option<&super::wrap::section::SectionStream>,
) {
    emit_stream_summary_inner(
        StreamSummaryContext {
            summary,
            profile,
            env_context,
            verbosity,
            verbose,
            details,
            section_stream,
        },
        None,
    );
}

pub(crate) fn emit_stream_summary_with_context(
    ctx: StreamSummaryContext<'_>,
    context_extra: &HashMap<String, serde_json::Value>,
) {
    emit_stream_summary_inner(ctx, Some(context_extra));
}

fn emit_stream_summary_inner(
    ctx: StreamSummaryContext<'_>,
    context_extra: Option<&HashMap<String, serde_json::Value>>,
) {
    let StreamSummaryContext {
        summary,
        profile,
        env_context,
        verbosity,
        verbose,
        details,
        section_stream,
    } = ctx;
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
        use super::wrap::section::Section;
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::renderable::Renderable;

        let term = crate::log::terminal();
        if let Some(section_stream) = section_stream {
            // Route every trailer line through the shared section tracker.
            // The tracker inserts the section-separator blank exactly once
            // when transitioning into `TrailerMetadata`, so callers do not
            // need any ad-hoc newline bookkeeping.
            if let Some(markup) = primary_markup {
                let rendered = Prose::new(markup).render(&term);
                section_stream.emit_stderr(Section::TrailerMetadata, &rendered);
            }
            if let Some(markup) = secondary_markup {
                let rendered = Prose::new(markup).render(&term);
                section_stream.emit_stderr(Section::TrailerMetadata, &format!("  {rendered}"));
            }
        } else {
            // Legacy / test fallback: keep the original spacing heuristic
            // so callers that do not own a section stream still emit a
            // reasonable separator between stdout text and the trailer.
            if !summary.assistant_text.is_empty() {
                if summary.assistant_text.ends_with("\n\n") {
                    // Already has a trailing blank line — no separator needed.
                } else if summary.assistant_text.ends_with('\n') {
                    eprintln!();
                } else {
                    eprint!("\n\n");
                }
            }
            if let Some(markup) = primary_markup {
                let rendered = Prose::new(markup).render(&term);
                eprintln!("{rendered}");
            }
            if let Some(markup) = secondary_markup {
                let rendered = Prose::new(markup).render(&term);
                eprintln!("  {rendered}");
            }
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

    if let Some(pp) = summary.permission_prompts {
        parts.push(format!(
            "{pp} <i>permission prompt{}</i>",
            if pp == 1 { "" } else { "s" }
        ));
    }

    if let Some(uip) = summary.user_input_prompts {
        parts.push(format!(
            "{uip} <i>user input prompt{}</i>",
            if uip == 1 { "" } else { "s" }
        ));
    }

    if parts.is_empty() {
        return None;
    }

    let mut out = format!("<dim>{prefix} {}</dim>", parts.join(" \u{00b7} "));
    for badge in &summary.badges {
        let color = match badge.severity {
            claudine::stream::badges::BadgeSeverity::Error => "red",
            claudine::stream::badges::BadgeSeverity::Warning => "yellow",
            claudine::stream::badges::BadgeSeverity::Info => "cyan",
        };
        out.push('\n');
        out.push_str(&format!(
            "<{color}>\u{26a0} <bold>{}</bold> \u{2014} {}</{color}>",
            badge.label, badge.message
        ));
        if let Some(url) = &badge.remediation_url {
            out.push('\n');
            out.push_str(&format!("  <dim>\u{2192} {url}</dim>"));
        }
    }
    Some(out)
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

fn extract_tags_from_prompt(
    prompt: Option<&str>,
    extract_tags: fn(&str) -> (String, Vec<String>),
) -> (Option<String>, Vec<String>) {
    let Some(prompt) = prompt else {
        return (None, Vec::new());
    };
    let (cleaned, tags) = extract_tags(prompt);
    if tags.is_empty() {
        (None, Vec::new())
    } else {
        (Some(cleaned), tags)
    }
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

fn print_wrapper_help(provider: Provider) {
    let slug = provider.as_slug();
    println!(
        "Wrap {provider} with Claudine preflight/env handling\n\
         \n\
         Usage: claudine {slug} [OPTIONS] [ARGS]...\n\
         \n\
         Arguments:\n\
         \x20 [ARGS]...  Arguments forwarded to the wrapped provider CLI\n\
         \n\
         Options:\n\
         \x20 -y, --yolo               Enable provider-specific YOLO/auto-approval mode\n\
         \x20     --include <ENV_NAME>  Preserve this env var even when it matches sensitive-name filters\n\
         \x20 -i, --interactive         Force interactive mode even when a prompt string is provided\n\
         \x20     --edit                Open the prompt in an external editor before launching the provider\n\
         \x20 -m, --model <MODEL>       Override the model used by the provider\n\
         \x20 -o, --output <FORMAT>     Set the output format (json, text, stream)\n\
          \x20     --asp <FILE>             Append a system prompt from a file\n\
          \x20     --rsp <FILE>             Replace the provider's system prompt with contents from a file\n\
          \x20 -t, --timeout <SECONDS>   Timeout in seconds (non-interactive only)\n\
         \x20     --dry-run             Show what would be executed without launching the child\n\
         \x20 -q, --quiet              Suppress env details and info; still show the system prompt when set\n\
         \x20     --silent              Suppress all Claudine preflight output\n\
         \x20     --operation <OP>      Set the OPERATION env var for the wrapped session\n\
         \x20     --sandbox             Enable provider-specific sandboxing\n\
         \x20     --repo                Use only repo-scoped skills, commands, and agents\n\
         \x20     --mcp                 Enable Claudine-managed MCP session composition\n\
         \x20     --use <ID>            Activate specific MCP servers by ID or alias\n\
         \x20     --strict              Treat unresolved or ambiguous MCP tags as hard errors\n\
         \x20     --perf                Emit a performance report to stderr after command completion\n\
         \x20 -h, --help               Print help"
    );
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExtractedWrapperFlags {
    yolo: bool,
    interactive: bool,
    edit: bool,
    repo: bool,
    quiet: bool,
    silent: bool,
    verbose: bool,
    operation: Option<String>,
    perf: bool,
}

/// Collector for wrapper-level performance telemetry.
///
/// Locate the POSIX `--` separator in the wrapper passthrough vector.
///
/// Returns the index of the first `--` that delimits agent-only arguments.
/// Two cases are handled:
///
/// 1. The `--` literal is present in the passthrough vector itself (clap
///    preserves it when it appears after the first positional, thanks to
///    `trailing_var_arg`). The boundary is at that index.
/// 2. The `--` was consumed by clap as a separator (it appeared before any
///    positional argument) and is therefore absent from the passthrough. We
///    fall back to the raw process arguments: count the tokens that followed
///    `--` in the original command line and mark the corresponding tail of
///    the passthrough as protected.
///
/// Returns `None` when no `--` was provided on the command line at all.
fn find_passthrough_dash_boundary(passthrough: &[String]) -> Option<usize> {
    let raw: Vec<String> = std::env::args().collect();
    find_passthrough_dash_boundary_with_raw(passthrough, &raw)
}

fn find_passthrough_dash_boundary_with_raw(
    passthrough: &[String],
    raw_args: &[String],
) -> Option<usize> {
    if let Some(pos) = passthrough.iter().position(|arg| arg == "--") {
        return Some(pos);
    }

    let raw_pos = raw_args.iter().position(|arg| arg == "--")?;
    let tail_count = raw_args.len() - raw_pos - 1;
    Some(passthrough.len().saturating_sub(tail_count))
}

fn extract_wrapper_flags_from_passthrough(args: &mut Vec<String>) -> Result<ExtractedWrapperFlags> {
    let boundary = find_passthrough_dash_boundary(args).unwrap_or(args.len());
    extract_wrapper_flags_from_passthrough_with_boundary(args, boundary)
}

fn extract_wrapper_flags_from_passthrough_with_boundary(
    args: &mut Vec<String>,
    boundary: usize,
) -> Result<ExtractedWrapperFlags> {
    let boundary = boundary.min(args.len());
    let mut extracted = ExtractedWrapperFlags::default();
    let mut skip_next = false;
    let mut remove_indices = Vec::new();

    for i in 0..boundary {
        if skip_next {
            skip_next = false;
            continue;
        }
        let arg = &args[i];
        match arg.as_str() {
            "-y" | "--yolo" => {
                extracted.yolo = true;
                remove_indices.push(i);
            }
            "-i" | "--interactive" => {
                extracted.interactive = true;
                remove_indices.push(i);
            }
            "--edit" => {
                extracted.edit = true;
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
            "--perf" => {
                extracted.perf = true;
                remove_indices.push(i);
            }
            "--operation" | "--op" => {
                let next = args.get(i + 1);
                let value_within_boundary = i + 1 < boundary;
                let next_is_separator = next.map(|v| v == "--").unwrap_or(false);

                if next.is_none() || !value_within_boundary || next_is_separator {
                    return Err(eyre!(
                        "missing value for `{arg}`; pass a value like `{arg} <OP>` \
                         before any `--` separator"
                    ));
                }

                extracted.operation = Some(next.unwrap().clone());
                remove_indices.push(i);
                remove_indices.push(i + 1);
                skip_next = true;
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

    // Remove in reverse order to preserve indices.
    for i in remove_indices.into_iter().rev() {
        args.remove(i);
    }

    Ok(extracted)
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
            child_cwd: PathBuf::from("/tmp"),
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
            child_cwd: PathBuf::from("/tmp"),
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
    fn maybe_edit_prompt_source_rejects_non_tty_sessions() {
        let err = maybe_edit_prompt_source_with(
            profile::PromptSource::Inline("seed".to_string()),
            false,
            false,
            || Ok("code".to_string()),
            |_, _| Ok(Some("edited".to_string())),
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "--edit requires an interactive terminal");
    }

    #[test]
    fn maybe_edit_prompt_source_rejects_inherited_stdin() {
        let err = maybe_edit_prompt_source_with(
            profile::PromptSource::InheritStdin,
            false,
            true,
            || Ok("code".to_string()),
            |_, _| Ok(Some("edited".to_string())),
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "--edit requires an interactive terminal");
    }

    #[test]
    fn maybe_edit_prompt_source_replaces_seed_with_edited_text() {
        let edited = maybe_edit_prompt_source_with(
            profile::PromptSource::Inline("seed prompt".to_string()),
            true,
            true,
            || Ok("code".to_string()),
            |seed, suffix| {
                assert_eq!(seed, "seed prompt");
                assert_eq!(suffix, ".md");
                Ok(Some("edited prompt".to_string()))
            },
        )
        .unwrap();

        assert_eq!(
            edited,
            Some(profile::PromptSource::Inline("edited prompt".to_string()))
        );
    }

    #[test]
    fn maybe_edit_prompt_source_uses_empty_seed_when_no_prompt_exists() {
        let edited = maybe_edit_prompt_source_with(
            profile::PromptSource::None,
            true,
            true,
            || Ok("code".to_string()),
            |seed, suffix| {
                assert_eq!(seed, "");
                assert_eq!(suffix, ".md");
                Ok(Some("typed from scratch".to_string()))
            },
        )
        .unwrap();

        assert_eq!(
            edited,
            Some(profile::PromptSource::Inline(
                "typed from scratch".to_string()
            ))
        );
    }

    #[test]
    fn maybe_edit_prompt_source_returns_clean_abort_for_empty_buffer() {
        let edited = maybe_edit_prompt_source_with(
            profile::PromptSource::Inline("seed".to_string()),
            true,
            true,
            || Ok("code".to_string()),
            |_, _| Ok(None),
        )
        .unwrap();

        assert_eq!(edited, None);
    }

    #[test]
    fn extract_wrapper_flags_lifts_reserved_aliases_from_passthrough() {
        let mut args = vec![
            "--json".to_string(),
            "-i".to_string(),
            "task".to_string(),
            "-y".to_string(),
        ];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

        assert!(extracted.yolo);
        assert!(extracted.interactive);
        assert_eq!(args, vec!["--json", "task"]);
    }

    #[test]
    fn extract_wrapper_flags_lifts_interactive_long_form() {
        let mut args = vec!["--interactive".to_string(), "do something".to_string()];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

        assert!(extracted.interactive);
        assert_eq!(args, vec!["do something"]);
    }

    #[test]
    fn extract_wrapper_flags_lifts_edit_long_form() {
        let mut args = vec!["do something".to_string(), "--edit".to_string()];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

        assert!(extracted.edit);
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

        let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

        // Old flags should NOT be consumed by Claudine
        assert!(!extracted.interactive);
        assert_eq!(args, vec!["-n", "--non-interactive", "--ni", "task"]);
    }

    #[test]
    fn extract_wrapper_flags_lifts_operation_from_passthrough() {
        let mut args = vec![
            "do something".to_string(),
            "--op".to_string(),
            "commit".to_string(),
        ];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

        assert_eq!(extracted.operation.as_deref(), Some("commit"));
        assert_eq!(args, vec!["do something"]);
    }

    #[test]
    fn extract_wrapper_flags_lifts_operation_equals_form() {
        let mut args = vec!["do something".to_string(), "--operation=deploy".to_string()];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

        assert_eq!(extracted.operation.as_deref(), Some("deploy"));
        assert_eq!(args, vec!["do something"]);
    }

    #[test]
    fn extract_wrapper_flags_lifts_op_equals_form() {
        let mut args = vec!["do something".to_string(), "--op=review".to_string()];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

        assert_eq!(extracted.operation.as_deref(), Some("review"));
        assert_eq!(args, vec!["do something"]);
    }

    #[test]
    fn extract_wrapper_flags_lifts_perf_from_passthrough() {
        let mut args = vec!["do something".to_string(), "--perf".to_string()];

        let extracted = extract_wrapper_flags_from_passthrough(&mut args).unwrap();

        assert!(extracted.perf);
        assert_eq!(args, vec!["do something"]);
    }

    #[test]
    fn extract_wrapper_flags_respects_double_dash_in_passthrough() {
        // User typed: claudine claude prompt -- --silent -y
        //
        // clap collects the tail verbatim because `trailing_var_arg` began
        // capturing at `prompt`, so the passthrough literally contains `--`.
        // Anything at or after that `--` must be opaque to Claudine.
        let mut args = vec![
            "prompt".to_string(),
            "--".to_string(),
            "--silent".to_string(),
            "-y".to_string(),
        ];

        let extracted = extract_wrapper_flags_from_passthrough_with_boundary(&mut args, 1).unwrap();

        assert!(!extracted.silent);
        assert!(!extracted.yolo);
        assert_eq!(args, vec!["prompt", "--", "--silent", "-y"]);
    }

    #[test]
    fn extract_wrapper_flags_respects_double_dash_consumed_by_clap() {
        // User typed: claudine claude -- prompt --silent
        //
        // clap consumed `--` as the positional separator so it is absent from
        // the passthrough vector. Boundary detection must recover the tail
        // count from the raw process arguments.
        let mut args = vec!["prompt".to_string(), "--silent".to_string()];

        let raw = vec![
            "claudine".to_string(),
            "claude".to_string(),
            "--".to_string(),
            "prompt".to_string(),
            "--silent".to_string(),
        ];
        let boundary = find_passthrough_dash_boundary_with_raw(&args, &raw).unwrap();
        let extracted =
            extract_wrapper_flags_from_passthrough_with_boundary(&mut args, boundary).unwrap();

        assert!(!extracted.silent);
        assert_eq!(args, vec!["prompt", "--silent"]);
    }

    #[test]
    fn extract_wrapper_flags_extracts_before_dash_but_not_after() {
        // User typed: claudine claude -y prompt -- --yolo
        //
        // `-y` BEFORE the prompt is consumed by clap (not present in
        // passthrough). The trailing `--yolo` after `--` must remain untouched
        // so it can collide with an agent-owned flag without being stolen.
        let mut args = vec!["prompt".to_string(), "--".to_string(), "--yolo".to_string()];

        let extracted = extract_wrapper_flags_from_passthrough_with_boundary(&mut args, 1).unwrap();

        assert!(!extracted.yolo);
        assert_eq!(args, vec!["prompt", "--", "--yolo"]);
    }

    #[test]
    fn extract_wrapper_flags_extracts_edit_before_dash_but_not_after() {
        let mut args = vec!["prompt".to_string(), "--".to_string(), "--edit".to_string()];

        let extracted = extract_wrapper_flags_from_passthrough_with_boundary(&mut args, 1).unwrap();

        assert!(!extracted.edit);
        assert_eq!(args, vec!["prompt", "--", "--edit"]);
    }

    #[test]
    fn find_passthrough_dash_boundary_detects_literal_separator() {
        let passthrough = vec!["prompt".to_string(), "--".to_string(), "rest".to_string()];
        let raw = vec![
            "claudine".to_string(),
            "claude".to_string(),
            "prompt".to_string(),
            "--".to_string(),
            "rest".to_string(),
        ];

        assert_eq!(
            find_passthrough_dash_boundary_with_raw(&passthrough, &raw),
            Some(1)
        );
    }

    #[test]
    fn find_passthrough_dash_boundary_uses_raw_tail_when_clap_strips_dash() {
        let passthrough = vec!["prompt".to_string(), "--silent".to_string()];
        let raw = vec![
            "claudine".to_string(),
            "claude".to_string(),
            "--".to_string(),
            "prompt".to_string(),
            "--silent".to_string(),
        ];

        assert_eq!(
            find_passthrough_dash_boundary_with_raw(&passthrough, &raw),
            Some(0)
        );
    }

    #[test]
    fn find_passthrough_dash_boundary_returns_none_without_dash() {
        let passthrough = vec!["prompt".to_string()];
        let raw = vec![
            "claudine".to_string(),
            "claude".to_string(),
            "prompt".to_string(),
        ];

        assert_eq!(
            find_passthrough_dash_boundary_with_raw(&passthrough, &raw),
            None
        );
    }

    #[test]
    fn extract_wrapper_flags_errors_on_dangling_operation_flag() {
        let mut args = vec!["prompt".to_string(), "--operation".to_string()];
        let boundary = args.len();

        let err =
            extract_wrapper_flags_from_passthrough_with_boundary(&mut args, boundary).unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("--operation"),
            "expected error to mention --operation, got: {message}"
        );
        assert!(
            message.to_lowercase().contains("missing value"),
            "expected error to describe the missing value, got: {message}"
        );
    }

    #[test]
    fn extract_wrapper_flags_errors_on_dangling_op_alias() {
        let mut args = vec!["prompt".to_string(), "--op".to_string()];
        let boundary = args.len();

        let err =
            extract_wrapper_flags_from_passthrough_with_boundary(&mut args, boundary).unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("--op"),
            "expected error to mention --op, got: {message}"
        );
    }

    #[test]
    fn extract_wrapper_flags_errors_when_operation_value_is_dash_separator() {
        // User typed: claudine claude --operation -- prompt
        //
        // `--operation` would otherwise greedily consume `--` as its value,
        // which is nonsensical. Require a real value before the separator.
        let mut args = vec![
            "--operation".to_string(),
            "--".to_string(),
            "prompt".to_string(),
        ];

        let err = extract_wrapper_flags_from_passthrough_with_boundary(&mut args, 1).unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("--operation"),
            "expected error to mention --operation, got: {message}"
        );
    }

    #[test]
    fn model_value_from_args_supports_short_and_long_forms() {
        let long_inline = vec!["--model=foo".to_string()];
        let short_next = vec!["-m".to_string(), "bar".to_string()];

        assert_eq!(model_value_from_args(&long_inline), Some("foo".to_string()));
        assert_eq!(model_value_from_args(&short_next), Some("bar".to_string()));
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
        let prompt = "fix #calendar bugs";

        let (cleaned, tags) = extract_tags_from_prompt(Some(prompt), lex_tags);

        assert_eq!(tags, vec!["calendar"]);
        assert_eq!(cleaned.as_deref(), Some("fix bugs"));
    }

    #[test]
    fn extracts_tags_from_gemini_prompt_flag() {
        let _ = make_catalog_with_servers(&["slack"]);
        let prompt = "debug #slack auth";

        let (cleaned, tags) = extract_tags_from_prompt(Some(prompt), lex_tags);

        assert_eq!(tags, vec!["slack"]);
        assert_eq!(cleaned.as_deref(), Some("debug auth"));
    }

    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn proptest_extract_wrapper_flags_preserves_others(
                flags in prop::collection::vec("-y|--yolo|-i|--interactive|--edit|-q|--quiet|--silent", 0..5),
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
                // Pass a boundary equal to args.len() so std::env::args() is
                // not consulted inside the proptest runner.
                let boundary = args.len();
                let extracted =
                    extract_wrapper_flags_from_passthrough_with_boundary(&mut args, boundary)
                        .unwrap();

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
                if flags.iter().any(|f| f == "--edit") {
                    assert!(extracted.edit);
                }
            }
        }
    }

    #[test]
    fn format_summary_prose_appends_badge_markup() {
        use claudine::events::Provider;
        use claudine::stream::badges::{BadgeCategory, BadgeSeverity, SessionBadge};
        use claudine::stream::summary::StreamExecutionSummary;
        let summary = StreamExecutionSummary {
            provider: Provider::Claude,
            duration_ms: Some(1000),
            badges: vec![SessionBadge {
                category: BadgeCategory::Billing,
                severity: BadgeSeverity::Error,
                label: "Billing".into(),
                message: "Insufficient credits".into(),
                remediation_url: Some("https://console.anthropic.com/settings/billing".into()),
            }],
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("Billing"));
        assert!(rendered.contains("Insufficient credits"));
        assert!(rendered.contains("https://console.anthropic.com/settings/billing"));
    }

    #[test]
    fn format_summary_prose_without_badges_has_no_badge_markup() {
        use claudine::events::Provider;
        use claudine::stream::summary::StreamExecutionSummary;
        let summary = StreamExecutionSummary {
            provider: Provider::Claude,
            duration_ms: Some(1000),
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(!rendered.contains("Billing"));
        assert!(!rendered.contains("\u{26a0}"));
    }

    #[test]
    fn format_summary_prose_renders_permission_prompts_singular() {
        let summary = claudine::stream::summary::StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            permission_prompts: Some(1),
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("1 <i>permission prompt</i>"));
        assert!(!rendered.contains("permission prompts"));
    }

    #[test]
    fn format_summary_prose_renders_permission_prompts_plural() {
        let summary = claudine::stream::summary::StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            permission_prompts: Some(3),
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("3 <i>permission prompts</i>"));
    }

    #[test]
    fn format_summary_prose_renders_user_input_prompts_singular() {
        let summary = claudine::stream::summary::StreamExecutionSummary {
            duration_ms: Some(18_000),
            user_input_prompts: Some(1),
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("1 <i>user input prompt</i>"));
        assert!(!rendered.contains("user input prompts"));
    }

    #[test]
    fn format_summary_prose_renders_both_counters() {
        let summary = claudine::stream::summary::StreamExecutionSummary {
            duration_ms: Some(41_000),
            tool_calls: Some(12),
            permission_prompts: Some(2),
            user_input_prompts: Some(1),
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(rendered.contains("2 <i>permission prompts</i>"));
        assert!(rendered.contains("1 <i>user input prompt</i>"));
    }

    #[test]
    fn format_summary_prose_omits_permission_clauses_when_unset() {
        let summary = claudine::stream::summary::StreamExecutionSummary {
            duration_ms: Some(18_000),
            tool_calls: Some(4),
            ..Default::default()
        };
        let rendered = super::format_summary_prose(&summary).unwrap();
        assert!(!rendered.contains("permission"));
        assert!(!rendered.contains("user input"));
    }
}
