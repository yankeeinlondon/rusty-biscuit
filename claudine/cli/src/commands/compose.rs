//! Top-level composition commands.
//!
//! - `claudine compose <file>` — chained composition (no file mutation)
//! - `claudine inline-compose <file>` — inline composition (replaces body)
//!
//! Both commands are thin request builders that delegate to
//! [`execute_composition_request`] for wrapper-grade execution.

// `CompositionError` carries variants with several `PathBuf` and other
// owned fields (e.g. `LoopIterationFailed`, `LoopRateLimited`) so the
// enum-on-the-stack is sizable. Boxing the inner data would ripple
// through every existing call site for marginal benefit; the closures
// in this file legitimately propagate the typed error and that's the
// shape they need to keep.
#![allow(clippy::result_large_err)]

use std::collections::BTreeSet;

use clap::Args;
use claudine::composition::{
    self, CompositionExecutionRequest, CompositionMode, OutputFormat as CompositionOutputFormat,
};
use claudine::provider::Provider;
use claudine::system_prompt::SystemPromptArgs;
use color_eyre::eyre::{Result, eyre};
use tracing::info_span;

use super::wrap::composition::execute_composition_request;
use crate::provider_values::provider_value_parser;

/// Shared flags for composition commands.
///
/// ## Notes
///
/// The eight provider boolean fields (`claude`, `codex`, `gemini`, `goose`,
/// `kimicode`, `opencode`, `qwen`, `roo`) are handled entirely by the
/// pre-clap argv normalizer in [`crate::argv`]: Rule 1 rewrites each
/// `--<provider>` token to the canonical `--provider <slug>` pair before
/// clap ever sees it. The struct fields and clap `#[arg(...)]` declarations
/// are retained as user-facing help entries only — their parsed boolean
/// values are never read at runtime (see [`Self::explicit_provider`]).
///
/// Retiring these fields is tracked in the
/// `2026-04-17-cli-pre-processing` spec follow-ups.
#[derive(Debug, Clone, Args)]
pub struct SharedComposeArgs {
    /// Use a specific provider.
    #[arg(long, value_parser = provider_value_parser(), group = "compose_provider")]
    pub provider: Option<Provider>,

    /// Use Claude Code.
    #[arg(long, group = "compose_provider")]
    pub claude: bool,

    /// Use Codex CLI.
    #[arg(long, group = "compose_provider")]
    pub codex: bool,

    /// Use Gemini CLI.
    #[arg(long, group = "compose_provider")]
    pub gemini: bool,

    /// Use Goose.
    #[arg(long, group = "compose_provider")]
    pub goose: bool,

    /// Use Kimi Code.
    #[arg(long = "kimi", group = "compose_provider")]
    pub kimicode: bool,

    /// Use OpenCode.
    #[arg(long, group = "compose_provider")]
    pub opencode: bool,

    /// Use Qwen Code.
    #[arg(long = "qwen", group = "compose_provider")]
    pub qwen: bool,

    /// Use Roo Code.
    #[arg(long = "roo", group = "compose_provider")]
    pub roo: bool,

    /// Exclude providers from automatic selection (repeatable).
    #[arg(long = "exclude", value_name = "PROVIDER", value_parser = provider_value_parser())]
    pub exclude: Vec<Provider>,

    /// Enable provider-specific YOLO/auto-approval mode.
    ///
    /// `CLAUDINE_YOLO=true` (or `=1`, `=yes`, `=on`) in the environment
    /// is equivalent to passing `--yolo` on the command line; both
    /// activate the same single intent signal that drives the provider's
    /// native bypass flag. The legacy short `YOLO` env var is no longer
    /// honored here — the reporter previously read it independently,
    /// producing diverging signals between launch behavior and reporting.
    #[arg(short = 'y', long, env = "CLAUDINE_YOLO")]
    pub yolo: bool,

    /// Run the provider session in interactive mode.
    #[arg(short = 'i', long)]
    pub interactive: bool,

    /// Preserve this env var even when it matches sensitive-name filters.
    #[arg(long = "include", value_name = "ENV_NAME")]
    pub include: Vec<String>,

    /// Override the model used by the provider.
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Set the output format (json, text, stream).
    #[arg(short = 'o', long = "output", value_name = "FORMAT")]
    pub output: Option<CompositionOutputFormat>,

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

    /// Wall-clock timeout (e.g. `30s`, `5m`, `2h`). Sends SIGTERM then SIGKILL.
    /// Only valid in non-interactive mode.
    #[arg(short = 't', long = "timeout", value_name = "DURATION")]
    pub timeout: Option<String>,

    /// Step-silence timeout (e.g. `30s`, `5m`). Kills the child when no stream
    /// event is observed for this long. Only valid in non-interactive
    /// structured-stream mode.
    #[arg(long = "step-timeout", value_name = "DURATION")]
    pub step_timeout: Option<String>,

    /// Set the OPERATION env var for the composed session.
    #[arg(long = "operation", visible_alias = "op", value_name = "OP")]
    pub operation: Option<String>,

    /// Enable provider-specific sandboxing.
    #[arg(long)]
    pub sandbox: bool,

    /// Use only repo-scoped skills, commands, and agents via a shadow HOME.
    #[arg(long)]
    pub repo: bool,

    /// Show what would be executed without launching the child.
    #[arg(long)]
    pub dry_run: bool,

    /// Suppress env details and info messages, but still show the system prompt when set.
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Suppress all output except the composition result.
    #[arg(long, conflicts_with = "quiet")]
    pub silent: bool,

    /// Override frontmatter values as JSON/JSON5 (e.g. `--set '{"key":"val"}'`).
    #[arg(long, value_name = "JSON")]
    pub set: Option<String>,

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

    /// Maximum number of loop iterations (overrides `loop.max` frontmatter).
    #[arg(long = "max-iterations", value_name = "N")]
    pub max_iterations: Option<usize>,

    /// What to do when a completed loop iteration reports a provider
    /// rate-limit signal. Overrides `loop.on_rate_limit` frontmatter.
    ///
    /// `pause` (default) sleeps until the provider's reset time, then runs
    /// the next iteration. `abort` halts the loop with a structured error
    /// (exit code 75 — `EX_TEMPFAIL`). `continue` proceeds without pausing.
    /// If `reset_at` is missing or already past, `pause` falls back to
    /// `abort` to avoid unbounded sleeps.
    #[arg(long = "on-rate-limit", value_name = "POLICY", value_enum)]
    pub on_rate_limit: Option<OnRateLimitArg>,
}

/// CLI-facing wrapper for [`claudine::composition::OnRateLimit`], exposed
/// as a [`clap::ValueEnum`] so that `--on-rate-limit` accepts the canonical
/// `pause | abort | continue` tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OnRateLimitArg {
    /// Pause until the provider's reset time, then continue (default).
    Pause,
    /// Halt the loop with a structured error (exit code 75).
    Abort,
    /// Proceed without pausing.
    Continue,
}

impl From<OnRateLimitArg> for claudine::composition::OnRateLimit {
    fn from(value: OnRateLimitArg) -> Self {
        match value {
            OnRateLimitArg::Pause => Self::Pause,
            OnRateLimitArg::Abort => Self::Abort,
            OnRateLimitArg::Continue => Self::Continue,
        }
    }
}

impl SharedComposeArgs {
    /// Explicit provider selected on the command line.
    ///
    /// After [`crate::argv::normalize`], provider boolean flags (`--claude`,
    /// `--gemini`, …) have already been rewritten to `--provider <slug>`,
    /// so runtime selection only needs to read the canonical `provider`
    /// field. The boolean fields remain on the struct as user-facing help
    /// entries but are never set in practice.
    pub(crate) fn explicit_provider(&self) -> Option<Provider> {
        self.provider
    }

    pub(crate) fn excluded(&self) -> BTreeSet<Provider> {
        self.exclude.iter().copied().collect()
    }

    pub(crate) fn system_prompt_args(&self) -> SystemPromptArgs {
        SystemPromptArgs {
            append_file: self.append_system_prompt.clone(),
            replace_file: self.replace_system_prompt.clone(),
        }
    }

    /// Parse `--step-timeout DURATION` into seconds using the same
    /// [`claudine::harness::parse_timeout`] grammar frontmatter uses, so CLI
    /// and frontmatter errors share one vocabulary. Returns `Ok(None)` when
    /// the flag was not supplied.
    pub(crate) fn step_timeout_secs(&self) -> Result<Option<u64>> {
        match self.step_timeout.as_deref() {
            Some(raw) => {
                let duration =
                    claudine::harness::parse_timeout(raw, std::path::Path::new("<--step-timeout>"))
                        .map_err(|e| eyre!("invalid --step-timeout value: {e}"))?;
                Ok(Some(duration.as_secs()))
            }
            None => Ok(None),
        }
    }
}

/// Compose a Markdown document through an agentic CLI.
///
/// Positional tokens are one file reference plus optional `key=value` setters
/// in any order. Inline setters override `--set` on overlapping keys.
#[derive(Debug, Clone, Args)]
pub struct ComposeArgs {
    #[command(flatten)]
    pub shared: SharedComposeArgs,

    /// File reference and/or `key=value` setters.
    #[arg(value_name = "ARG", num_args = 1.., required = true)]
    pub args: Vec<String>,
}

/// Inline composition: use frontmatter `prompt`, replace body with output.
///
/// Positional tokens are one file reference plus optional `key=value` setters
/// in any order. Inline setters override `--set` on overlapping keys.
#[derive(Debug, Clone, Args)]
pub struct InlineComposeArgs {
    #[command(flatten)]
    pub shared: SharedComposeArgs,

    /// File reference and/or `key=value` setters.
    #[arg(value_name = "ARG", num_args = 1.., required = true)]
    pub args: Vec<String>,
}

/// Entry point for `claudine compose`.
///
/// Errors returned here bubble up to the top-level walker in `main.rs`,
/// which renders darkmatter `BlockError` reports for typed Markdown
/// failures and falls back to `color_eyre` otherwise.
pub fn run_compose(
    args: ComposeArgs,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<()> {
    let code = run_compose_inner(args, verbose, startup_timings)?;
    std::process::exit(code);
}

/// Entry point for `claudine inline-compose`.
///
/// Errors returned here bubble up to the top-level walker in `main.rs`,
/// which renders darkmatter `BlockError` reports for typed Markdown
/// failures and falls back to `color_eyre` otherwise.
pub fn run_inline_compose(
    args: InlineComposeArgs,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<()> {
    let code = run_inline_compose_inner(args, verbose, startup_timings)?;
    std::process::exit(code);
}

fn run_compose_inner(
    args: ComposeArgs,
    verbose: u8,
    mut startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<i32> {
    let compose_entry = std::time::Instant::now();
    let ComposeArgs { shared, args } = args;
    let parsed = parse_composition_positionals(&args)?;
    let file = parsed.file_ref.ok_or_else(|| {
        eyre!("missing file reference: expected exactly one file reference plus optional key=value setters")
    })?;

    // Receipt banner: immediate feedback that the request was accepted.
    // Suppressed only by `--silent`. Per spec W1, `--quiet` follows the
    // existing execution header's detail rules — which include emitting
    // the brief one-liner — so the banner appears under `--quiet`.
    if !shared.silent {
        use biscuit_terminal::components::renderable::Renderable;
        use biscuit_terminal::components::status::Status;
        let term = crate::log::terminal();
        let status = Status::from_prose(format!("→ Composing {file}…"));
        crate::log::message(&status.render(&term));
    }

    // Install the process-scoped SIGINT handler now so Ctrl+C during the
    // (potentially slow) compose prep phase produces the same INFO notice
    // and exit-130 outcome the loop emits during its iterations.
    let _user_interrupt_guard = install_user_interrupt_guard(&file);
    if shared.timeout.is_some() && shared.interactive {
        return Err(eyre!("--timeout cannot be used with --interactive mode"));
    }
    if shared.step_timeout.is_some() && shared.interactive {
        return Err(eyre!(
            "--step-timeout cannot be used with --interactive mode"
        ));
    }
    // Early validation: both flags share the same duration grammar.
    if let Some(ref raw) = shared.timeout {
        claudine::harness::parse_timeout(raw, std::path::Path::new("<--timeout>"))
            .map_err(|e| eyre!("invalid --timeout value: {e}"))?;
    }
    shared.step_timeout_secs()?;
    let set_overrides = merge_set_overrides(shared.set.as_deref(), parsed.shorthand_setters)?;
    let system_prompt_args = shared.system_prompt_args();

    let source = composition::resolve_composition_source(&file)?;

    let _compose_span = info_span!(
        "compose",
        file = %source.resolved_path.display(),
        provider = ?shared.explicit_provider(),
        interactive = shared.interactive,
    )
    .entered();

    // Phase 2 (2026-05-09-slow-prep): build the per-invocation prep context
    // immediately after source resolution. The context owns the single
    // source-repo-root discovery, the loaded selection config, and the
    // installed-provider snapshot used by every later prep phase.
    let prep_context = super::wrap::composition::CompositionPrepContext::new(
        &file,
        &source.resolved_path,
        &shared.excluded(),
    )?;

    // -- Eager target resolution -------------------------------------------
    // Resolve the execution target *before* composing templates so that
    // `{{env.AGENT}}` in the body resolves to the chosen provider's slug.
    // Hints come from the raw frontmatter (no compose).
    let raw_hints =
        composition::parse_selection_hints_from_frontmatter(source.markdown.frontmatter())?;
    let resolved_target = {
        let _span = info_span!("compose_prep.eager_target").entered();
        super::wrap::composition::eagerly_resolve_target(
            &prep_context,
            &raw_hints,
            shared.explicit_provider(),
            shared.model.as_deref(),
        )?
    };

    let mut env_overrides: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    super::wrap::composition::install_agent_env_for_composition(
        &resolved_target,
        &mut env_overrides,
    );

    // ── Pre-flight shell approval ────────────────────────────────────
    let compose_options = {
        let mut opts = darkmatter::markdown::compose::ComposeOptions::new()
            .with_source_file(&source.resolved_path);
        if let Some(ref overrides) = set_overrides {
            opts = opts.with_set_overrides(overrides.clone());
        }
        opts
    };

    let shared_approval_cache =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

    let approval_options = super::wrap::build_harness_shell_options_with_cache(
        &source.resolved_path,
        prep_context.source_repo_root.as_deref(),
        Some(std::sync::Arc::clone(&shared_approval_cache)),
    );

    let preflight = {
        let _span = info_span!("compose_prep.shell_preflight").entered();
        composition::resolve_shell_approvals(
            Some(&source.markdown),
            Some(&compose_options),
            None,
            &approval_options,
        )?
    };

    // Post-prep interrupt checkpoint. If the user pressed Ctrl+C during
    // any of the prep phases (compose source parse, target resolution,
    // env install, shell preflight) the SIGINT handler has already
    // emitted the INFO notice — short-circuit before launching the agent
    // or entering the loop.
    if crate::output::user_interrupt_observed() {
        return Ok(USER_INTERRUPT_EXIT_CODE);
    }

    // ── Loop detection and execution ─────────────────────────────────────
    let loop_options = claudine::composition::LoopExecutionOptions {
        max_iterations: shared
            .max_iterations
            .or(claudine::composition::resolve_max_iterations_from_env()),
        fail_fast: claudine::composition::resolve_fail_fast_from_env(),
        on_rate_limit: shared.on_rate_limit.map(Into::into),
        // Engine polls this during rate-limit pause sleeps so Ctrl+C
        // surfaces as `LoopInterrupted` instead of waiting out the timer.
        interrupt_check: Some(crate::output::user_interrupt_observed),
    };

    let file_for_loop = file.clone();
    if let Some(loop_result) =
        run_loop_with_overrides(&source, set_overrides.as_ref(), loop_options, |ctx| {
            let prepared = {
                let _span = info_span!("compose_prep.prepare_direct").entered();
                composition::prepare_direct(
                    &source,
                    composition::PrepareOptions {
                        set_overrides: Some(ctx.as_set_overrides()),
                        pre_approved_commands: Some(preflight.approved_commands.clone()),
                        env_overrides: env_overrides.clone(),
                        perf_enabled: shared.perf,
                        source_repo_root: prep_context.source_repo_root.clone(),
                    },
                )?
            };

            let request = CompositionExecutionRequest {
                mode: CompositionMode::ChainedDocument,
                file_ref: file_for_loop.clone(),
                prepared,
                resolved_target: Some(resolved_target.clone()),
                explicit_provider: shared.explicit_provider(),
                excluded: shared.excluded(),
                yolo: shared.yolo,
                include: shared.include.clone(),
                model: shared.model.clone(),
                output: shared.output,
                system_prompt_args: system_prompt_args.clone(),
                timeout: shared.timeout.clone(),
                step_timeout: shared.step_timeout.clone(),
                operation: shared.operation.clone(),
                sandbox: shared.sandbox,
                repo: shared.repo,
                dry_run: shared.dry_run,
                mcp: shared.mcp,
                mcp_use: shared.mcp_use.clone(),
                strict: shared.strict,
                session_interactive: shared.interactive,
                quiet: shared.quiet,
                silent: shared.silent,
                env_overrides: env_overrides.clone(),
                shared_approval_cache: Some(std::sync::Arc::clone(&shared_approval_cache)),
                sequence: false,
                installed_snapshot: Some(prep_context.installed_snapshot.clone()),
                prep_launch_workspace: Some(prep_context.launch_workspace.clone()),
                prep_launch_context: Some(prep_context.launch_context.clone()),
                prep_env_context: Some(prep_context.env_context.clone()),
                prep_launch_detection_error: prep_context.launch_detection_error.clone(),
            };

            let outcome = super::wrap::composition::execute_composition_request_inner(
                request,
                verbose,
                None,
                shared.perf,
            )
            .map_err(|e| {
                // Pre-spawn execution wiring failed (binary lookup, env
                // build, etc.). Surface as an iteration failure — these
                // are runtime problems, not malformed loop frontmatter.
                claudine::composition::CompositionError::LoopIterationFailed {
                    iteration: ctx.iteration,
                    prompt_path: source.resolved_path.clone(),
                    exit_code: 1,
                    reason: e.to_string(),
                    exit_reason: None,
                }
            })?;

            Ok(build_loop_iteration_output(
                ctx.iteration,
                &source.resolved_path,
                outcome,
            ))
        })?
    {
        if let Some(error) = loop_result.error {
            // The interrupt path already announced itself via the INFO
            // status line; suppress the red `Error:` echo and exit with
            // the conventional 130 code so the shell sees a clean Ctrl+C.
            if matches!(
                error,
                claudine::composition::CompositionError::LoopInterrupted { .. }
            ) {
                return Ok(loop_result.final_exit_code);
            }
            // Rate-limit halt has its own conventional exit code
            // (`EX_TEMPFAIL` = 75) so shell wrappers can recognize a
            // transient halt and retry-after-cool-off. Render the styled
            // error block inline, then return `Ok(75)` so the outer
            // process exits with the right code.
            if matches!(
                error,
                claudine::composition::CompositionError::LoopRateLimited { .. }
            ) {
                emit_rate_limit_halt(&error);
                return Ok(claudine::composition::LOOP_RATE_LIMITED_EXIT_CODE);
            }
            return Err(error.into());
        }
        return Ok(loop_result.final_exit_code);
    }

    // ── Single execution path (no loop) ──────────────────────────────────
    let prepared = {
        let _span = info_span!("compose_prep.prepare_direct").entered();
        composition::prepare_direct(
            &source,
            composition::PrepareOptions {
                set_overrides,
                pre_approved_commands: Some(preflight.approved_commands),
                env_overrides: env_overrides.clone(),
                perf_enabled: shared.perf,
                source_repo_root: prep_context.source_repo_root.clone(),
            },
        )?
    };

    let request = CompositionExecutionRequest {
        mode: CompositionMode::ChainedDocument,
        file_ref: file,
        prepared,
        resolved_target: Some(resolved_target),
        explicit_provider: shared.explicit_provider(),
        excluded: shared.excluded(),
        yolo: shared.yolo,
        include: shared.include,
        model: shared.model,
        output: shared.output,
        system_prompt_args,
        timeout: shared.timeout.clone(),
        step_timeout: shared.step_timeout.clone(),
        operation: shared.operation,
        sandbox: shared.sandbox,
        repo: shared.repo,
        dry_run: shared.dry_run,
        mcp: shared.mcp,
        mcp_use: shared.mcp_use,
        strict: shared.strict,
        session_interactive: shared.interactive,
        quiet: shared.quiet,
        silent: shared.silent,
        env_overrides,
        shared_approval_cache: Some(shared_approval_cache),
                sequence: false,
                installed_snapshot: Some(prep_context.installed_snapshot.clone()),
                prep_launch_workspace: Some(prep_context.launch_workspace.clone()),
                prep_launch_context: Some(prep_context.launch_context.clone()),
                prep_env_context: Some(prep_context.env_context.clone()),
                prep_launch_detection_error: prep_context.launch_detection_error.clone(),
            };

    if let Some(ref mut timings) = startup_timings {
        timings.prep_phase = compose_entry.elapsed();
    }

    execute_composition_request(request, verbose, startup_timings, shared.perf)
}

fn run_inline_compose_inner(
    args: InlineComposeArgs,
    verbose: u8,
    mut startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<i32> {
    let inline_compose_entry = std::time::Instant::now();
    let InlineComposeArgs { shared, args } = args;
    let parsed = parse_composition_positionals(&args)?;
    let file = parsed.file_ref.ok_or_else(|| {
        eyre!("missing file reference: expected exactly one file reference plus optional key=value setters")
    })?;

    // Receipt banner: immediate feedback that the request was accepted.
    // Suppressed only by `--silent`. Per spec W1, `--quiet` follows the
    // existing execution header's detail rules — which include emitting
    // the brief one-liner — so the banner appears under `--quiet`.
    if !shared.silent {
        use biscuit_terminal::components::renderable::Renderable;
        use biscuit_terminal::components::status::Status;
        let term = crate::log::terminal();
        let status = Status::from_prose(format!("→ Composing {file}…"));
        crate::log::message(&status.render(&term));
    }

    // Install the process-scoped SIGINT handler now so Ctrl+C during the
    // (potentially slow) compose prep phase produces the same INFO notice
    // and exit-130 outcome the loop emits during its iterations.
    let _user_interrupt_guard = install_user_interrupt_guard(&file);
    if shared.timeout.is_some() && shared.interactive {
        return Err(eyre!("--timeout cannot be used with --interactive mode"));
    }
    if shared.step_timeout.is_some() && shared.interactive {
        return Err(eyre!(
            "--step-timeout cannot be used with --interactive mode"
        ));
    }
    // Early validation: both flags share the same duration grammar.
    if let Some(ref raw) = shared.timeout {
        claudine::harness::parse_timeout(raw, std::path::Path::new("<--timeout>"))
            .map_err(|e| eyre!("invalid --timeout value: {e}"))?;
    }
    shared.step_timeout_secs()?;
    let set_overrides = merge_set_overrides(shared.set.as_deref(), parsed.shorthand_setters)?;
    let system_prompt_args = shared.system_prompt_args();
    let show_checks = !shared.silent;
    let term = if show_checks {
        Some(crate::log::terminal())
    } else {
        None
    };

    let source = match composition::resolve_composition_source(&file) {
        Ok(source) => {
            if let Some(ref t) = term {
                claudine::harness::report::report_source_file(&file, &source.resolved_path, t);
            }
            let _inline_span = info_span!(
                "inline_compose",
                file = %source.resolved_path.display(),
                provider = ?shared.explicit_provider(),
                interactive = shared.interactive,
            )
            .entered();
            source
        }
        Err(e) => {
            if let Some(ref t) = term {
                claudine::harness::report::report_source_file(&file, std::path::Path::new(""), t);
            }
            return Err(e.into());
        }
    };

    // -- Pre-validation: prompt frontmatter property ------------------------

    let prompt_value = source
        .markdown
        .frontmatter()
        .as_map()
        .get("prompt")
        .cloned();
    let has_prompt = prompt_value.is_some();
    let is_non_empty = prompt_value
        .as_ref()
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.trim().is_empty());

    if let Some(ref t) = term {
        claudine::harness::report::report_prompt_property(has_prompt, is_non_empty, t);
    }

    // Phase 2 (2026-05-09-slow-prep): single source-root discovery for the
    // whole inline-compose invocation; downstream prep phases reuse this
    // context rather than rediscovering the repo root.
    let prep_context = super::wrap::composition::CompositionPrepContext::new(
        &file,
        &source.resolved_path,
        &shared.excluded(),
    )?;

    // -- Eager target resolution ------------------------------------------
    let raw_hints =
        composition::parse_selection_hints_from_frontmatter(source.markdown.frontmatter())?;
    let resolved_target = {
        let _span = info_span!("compose_prep.eager_target").entered();
        super::wrap::composition::eagerly_resolve_target(
            &prep_context,
            &raw_hints,
            shared.explicit_provider(),
            shared.model.as_deref(),
        )?
    };

    let mut env_overrides: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    super::wrap::composition::install_agent_env_for_composition(
        &resolved_target,
        &mut env_overrides,
    );

    // ── Pre-flight shell approval ────────────────────────────────────
    let compose_options = {
        let mut opts = darkmatter::markdown::compose::ComposeOptions::new()
            .with_source_file(&source.resolved_path);
        if let Some(ref overrides) = set_overrides {
            opts = opts.with_set_overrides(overrides.clone());
        }
        opts
    };

    let shared_approval_cache =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

    let approval_options = super::wrap::build_harness_shell_options_with_cache(
        &source.resolved_path,
        prep_context.source_repo_root.as_deref(),
        Some(std::sync::Arc::clone(&shared_approval_cache)),
    );

    let preflight = {
        let _span = info_span!("compose_prep.shell_preflight").entered();
        composition::resolve_shell_approvals(
            Some(&source.markdown),
            Some(&compose_options),
            None,
            &approval_options,
        )?
    };

    // Post-prep interrupt checkpoint. If the user pressed Ctrl+C during
    // any of the prep phases (compose source parse, target resolution,
    // env install, shell preflight) the SIGINT handler has already
    // emitted the INFO notice — short-circuit before launching the agent
    // or entering the loop.
    if crate::output::user_interrupt_observed() {
        return Ok(USER_INTERRUPT_EXIT_CODE);
    }

    // ── Loop detection and execution ─────────────────────────────────────
    let loop_options = claudine::composition::LoopExecutionOptions {
        max_iterations: shared
            .max_iterations
            .or(claudine::composition::resolve_max_iterations_from_env()),
        fail_fast: claudine::composition::resolve_fail_fast_from_env(),
        on_rate_limit: shared.on_rate_limit.map(Into::into),
        // Engine polls this during rate-limit pause sleeps so Ctrl+C
        // surfaces as `LoopInterrupted` instead of waiting out the timer.
        interrupt_check: Some(crate::output::user_interrupt_observed),
    };

    let file_for_loop = file.clone();
    if let Some(loop_result) =
        run_loop_with_overrides(&source, set_overrides.as_ref(), loop_options, |ctx| {
            let prepared = {
                let _span = info_span!("compose_prep.prepare_inline").entered();
                composition::prepare_inline(
                    &source,
                    composition::PrepareOptions {
                        set_overrides: Some(ctx.as_set_overrides()),
                        pre_approved_commands: Some(preflight.approved_commands.clone()),
                        env_overrides: env_overrides.clone(),
                        perf_enabled: shared.perf,
                        source_repo_root: prep_context.source_repo_root.clone(),
                    },
                )?
            };

            let request = CompositionExecutionRequest {
                mode: CompositionMode::InlineFrontmatterPrompt,
                file_ref: file_for_loop.clone(),
                prepared,
                resolved_target: Some(resolved_target.clone()),
                explicit_provider: shared.explicit_provider(),
                excluded: shared.excluded(),
                yolo: shared.yolo,
                include: shared.include.clone(),
                model: shared.model.clone(),
                output: shared.output,
                system_prompt_args: system_prompt_args.clone(),
                timeout: shared.timeout.clone(),
                step_timeout: shared.step_timeout.clone(),
                operation: shared.operation.clone(),
                sandbox: shared.sandbox,
                repo: shared.repo,
                dry_run: shared.dry_run,
                mcp: shared.mcp,
                mcp_use: shared.mcp_use.clone(),
                strict: shared.strict,
                session_interactive: shared.interactive,
                quiet: shared.quiet,
                silent: shared.silent,
                env_overrides: env_overrides.clone(),
                shared_approval_cache: Some(std::sync::Arc::clone(&shared_approval_cache)),
                sequence: false,
                installed_snapshot: Some(prep_context.installed_snapshot.clone()),
                prep_launch_workspace: Some(prep_context.launch_workspace.clone()),
                prep_launch_context: Some(prep_context.launch_context.clone()),
                prep_env_context: Some(prep_context.env_context.clone()),
                prep_launch_detection_error: prep_context.launch_detection_error.clone(),
            };

            let outcome = super::wrap::composition::execute_composition_request_inner(
                request,
                verbose,
                None,
                shared.perf,
            )
            .map_err(|e| {
                // Pre-spawn execution wiring failed (binary lookup, env
                // build, etc.). Surface as an iteration failure — these
                // are runtime problems, not malformed loop frontmatter.
                claudine::composition::CompositionError::LoopIterationFailed {
                    iteration: ctx.iteration,
                    prompt_path: source.resolved_path.clone(),
                    exit_code: 1,
                    reason: e.to_string(),
                    exit_reason: None,
                }
            })?;

            Ok(build_loop_iteration_output(
                ctx.iteration,
                &source.resolved_path,
                outcome,
            ))
        })?
    {
        if let Some(error) = loop_result.error {
            // The interrupt path already announced itself via the INFO
            // status line; suppress the red `Error:` echo and exit with
            // the conventional 130 code so the shell sees a clean Ctrl+C.
            if matches!(
                error,
                claudine::composition::CompositionError::LoopInterrupted { .. }
            ) {
                return Ok(loop_result.final_exit_code);
            }
            // Rate-limit halt has its own conventional exit code
            // (`EX_TEMPFAIL` = 75) so shell wrappers can recognize a
            // transient halt and retry-after-cool-off. Render the styled
            // error block inline, then return `Ok(75)` so the outer
            // process exits with the right code.
            if matches!(
                error,
                claudine::composition::CompositionError::LoopRateLimited { .. }
            ) {
                emit_rate_limit_halt(&error);
                return Ok(claudine::composition::LOOP_RATE_LIMITED_EXIT_CODE);
            }
            return Err(error.into());
        }
        return Ok(loop_result.final_exit_code);
    }

    // ── Single execution path (no loop) ──────────────────────────────────
    let prepared = {
        let _span = info_span!("compose_prep.prepare_inline").entered();
        composition::prepare_inline(
            &source,
            composition::PrepareOptions {
                set_overrides,
                pre_approved_commands: Some(preflight.approved_commands),
                env_overrides: env_overrides.clone(),
                perf_enabled: shared.perf,
                source_repo_root: prep_context.source_repo_root.clone(),
            },
        )?
    };

    let request = CompositionExecutionRequest {
        mode: CompositionMode::InlineFrontmatterPrompt,
        file_ref: file,
        prepared,
        resolved_target: Some(resolved_target),
        explicit_provider: shared.explicit_provider(),
        excluded: shared.excluded(),
        yolo: shared.yolo,
        include: shared.include,
        model: shared.model,
        output: shared.output,
        system_prompt_args,
        timeout: shared.timeout.clone(),
        step_timeout: shared.step_timeout.clone(),
        operation: shared.operation,
        sandbox: shared.sandbox,
        repo: shared.repo,
        dry_run: shared.dry_run,
        mcp: shared.mcp,
        mcp_use: shared.mcp_use,
        strict: shared.strict,
        session_interactive: shared.interactive,
        quiet: shared.quiet,
        silent: shared.silent,
        env_overrides,
        shared_approval_cache: Some(shared_approval_cache),
        sequence: false,
        installed_snapshot: Some(prep_context.installed_snapshot.clone()),
        prep_launch_workspace: Some(prep_context.launch_workspace.clone()),
        prep_launch_context: Some(prep_context.launch_context.clone()),
        prep_env_context: Some(prep_context.env_context.clone()),
        prep_launch_detection_error: prep_context.launch_detection_error.clone(),
    };

    if let Some(ref mut timings) = startup_timings {
        timings.prep_phase = inline_compose_entry.elapsed();
    }

    execute_composition_request(request, verbose, startup_timings, shared.perf)
}

/// Render a `LoopRateLimited` error inline so the user sees the styled
/// halt notice before the wrapper exits with `EX_TEMPFAIL`.
fn emit_rate_limit_halt(error: &claudine::composition::CompositionError) {
    use biscuit_terminal::errors::BlockError;
    let term = crate::log::terminal();
    let rendered = error.report_block_error(&term);
    crate::log::message("");
    crate::log::message(&rendered);
    crate::log::message("");
}

/// Translate a [`SingleCompositionOutcome`] into a
/// [`claudine::composition::LoopIterationOutput`] that the loop engine can
/// inspect for rate-limit policy and honest error classification.
///
/// On a non-zero `outcome.exit_code` this builds a
/// [`claudine::composition::CompositionError::LoopIterationFailed`] with a
/// `reason` and `exit_reason` pulled from the iteration's session_end
/// signals (e.g. `step_timeout`) — never the old
/// `LoopInvalid("provider exited with code N")` overload, which was
/// reserved for malformed `loop:` frontmatter.
///
/// Always attaches the iteration's rate-limit trailer and
/// provider/model attribution so the engine can apply the configured
/// [`claudine::composition::OnRateLimit`] policy between iterations.
fn build_loop_iteration_output(
    iteration: usize,
    prompt_path: &std::path::Path,
    outcome: super::wrap::composition::SingleCompositionOutcome,
) -> claudine::composition::LoopIterationOutput {
    let signals = outcome.iteration_signals.unwrap_or_default();
    let rate_limit = signals.rate_limit.clone();
    let exit_reason = signals.exit_reason.clone();
    let provider_id = signals.provider_id.clone();
    let model_id = signals.model_id.clone();

    if outcome.exit_code == 0 {
        claudine::composition::LoopIterationOutput::success("")
            .with_rate_limit(rate_limit)
            .with_exit_reason(exit_reason)
            .with_attribution(provider_id, model_id)
    } else {
        // Build a human-readable cause that surfaces the structured
        // exit_reason at the top. Watchdog detail (e.g. "no stream
        // activity for 30m; terminating due to step_timeout") rides
        // along on the next line.
        let reason = match (&exit_reason, &signals.error_message) {
            (Some(kind), Some(detail)) => format!("{kind}\n  ↳ {detail}"),
            (Some(kind), None) => kind.clone(),
            (None, Some(detail)) => detail.clone(),
            (None, None) => "provider exited non-zero".to_string(),
        };
        let error = claudine::composition::CompositionError::LoopIterationFailed {
            iteration,
            prompt_path: prompt_path.to_path_buf(),
            exit_code: outcome.exit_code,
            reason,
            exit_reason: exit_reason.clone(),
        };
        claudine::composition::LoopIterationOutput::failure("", outcome.exit_code, error)
            .with_rate_limit(rate_limit)
            .with_exit_reason(exit_reason)
            .with_attribution(provider_id, model_id)
    }
}

/// Run a composition loop with CLI `--set` / shorthand setter overrides
/// merged into the loop's initial frontmatter.
///
/// Returns `Ok(None)` when the source has no `loop` frontmatter, matching
/// [`claudine::composition::execute_loop`].
///
/// CLI overrides shadow on-disk frontmatter values from the very first
/// iteration so that the loop condition and templated body see the values
/// the user passed on the command line.
fn run_loop_with_overrides<F>(
    source: &claudine::composition::ResolvedCompositionSource,
    set_overrides: Option<&serde_json::Value>,
    options: claudine::composition::LoopExecutionOptions,
    mut executor: F,
) -> std::result::Result<
    Option<claudine::composition::LoopExecutionResult>,
    claudine::composition::CompositionError,
>
where
    F: FnMut(
        claudine::composition::LoopIterationContext,
    ) -> std::result::Result<
        claudine::composition::LoopIterationOutput,
        claudine::composition::CompositionError,
    >,
{
    let Some(config) = claudine::composition::resolve_loop_config(source)? else {
        return Ok(None);
    };

    let mut initial_frontmatter: serde_json::Map<String, serde_json::Value> = source
        .markdown
        .frontmatter()
        .as_map()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if let Some(serde_json::Value::Object(overrides)) = set_overrides {
        for (k, v) in overrides {
            initial_frontmatter.insert(k.clone(), v.clone());
        }
    }

    let prompt_path = source.resolved_path.clone();

    // The Ctrl+C SIGINT handler is installed at the top of the compose
    // subcommand (see `install_user_interrupt_guard`) so it covers the
    // entire prep window, not just the loop. The wrapped executor below
    // simply observes the process-scoped flag and short-circuits
    // remaining iterations once the user has interrupted.
    let prompt_path_for_executor = prompt_path.clone();
    let wrapped_executor = move |ctx: claudine::composition::LoopIterationContext| {
        if crate::output::user_interrupt_observed() {
            return Ok(claudine::composition::LoopIterationOutput::failure(
                "",
                USER_INTERRUPT_EXIT_CODE,
                claudine::composition::CompositionError::LoopInterrupted {
                    prompt_path: prompt_path_for_executor.clone(),
                },
            ));
        }
        let output = executor(ctx)?;
        if crate::output::user_interrupt_observed() {
            return Ok(claudine::composition::LoopIterationOutput::failure(
                output.output,
                USER_INTERRUPT_EXIT_CODE,
                claudine::composition::CompositionError::LoopInterrupted {
                    prompt_path: prompt_path_for_executor.clone(),
                },
            ));
        }
        Ok(output)
    };

    let mut result = claudine::composition::execute_loop_with_config(
        &source.resolved_path,
        &config,
        initial_frontmatter,
        options,
        wrapped_executor,
    )?;

    if crate::output::user_interrupt_observed() {
        // Force the interrupt outcome regardless of fail-fast: under
        // `fail_fast: false` the engine would have continued past the
        // first short-circuited iteration, so overwrite the result so
        // callers see exit code 130 and a `LoopInterrupted` error.
        result.final_exit_code = USER_INTERRUPT_EXIT_CODE;
        result.error = Some(claudine::composition::CompositionError::LoopInterrupted {
            prompt_path: prompt_path.clone(),
        });
    }

    Ok(Some(result))
}

/// Exit code emitted when Ctrl+C is observed during a compose run.
/// Matches the standard `128 + SIGINT(2)` convention used by shells.
pub(crate) const USER_INTERRUPT_EXIT_CODE: i32 = 130;

/// RAII guard returned by [`install_user_interrupt_guard`]. Drops the
/// underlying `signal_hook` registration when the compose subcommand
/// returns, restoring whatever handler was previously installed.
pub(crate) struct UserInterruptGuard {
    #[cfg(unix)]
    _hook: Option<signal_hook::SigId>,
}

/// Install a process-scoped SIGINT handler that covers the **entire**
/// compose / inline-compose run — including the slow prep phase before
/// the loop is entered. The handler:
///
/// - Marks the process-scoped `USER_INTERRUPTED` flag so any downstream
///   surface (loop executor, live semantic sink, post-prep checkpoints)
///   can branch on it.
/// - Writes a pre-rendered INFO notice to stderr exactly once via
///   async-signal-safe `libc::write(2)`. The notice has a leading `\n`
///   so it lands at column 1 (off the terminal's echoed `^C`) and the
///   prompt is rendered as an OSC8 hyperlink whose visible text is the
///   user's CLI argument verbatim.
///
/// `signal_hook::low_level::register` stacks handlers, so this one
/// composes cleanly with the per-iteration SIGINT handler the wrapper
/// installs around each agent child.
pub(crate) fn install_user_interrupt_guard(prompt_argv: &str) -> UserInterruptGuard {
    let bytes = std::sync::Arc::new(format_user_interrupt_message(prompt_argv).into_bytes());
    let printed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    #[cfg(unix)]
    {
        let bytes_handler = std::sync::Arc::clone(&bytes);
        let printed_handler = std::sync::Arc::clone(&printed);
        let hook = unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
                crate::output::mark_user_interrupted();
                // Print our notice exactly once. `write(2)` on a file
                // descriptor is async-signal-safe; the Rust stdio
                // macros (`eprintln!`, `println!`) are not, and any
                // allocation or `tracing` call would be unsafe here.
                if !printed_handler.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    let buf = bytes_handler.as_slice();
                    libc::write(
                        libc::STDERR_FILENO,
                        buf.as_ptr() as *const libc::c_void,
                        buf.len(),
                    );
                }
            })
        }
        .ok();
        UserInterruptGuard { _hook: hook }
    }
    #[cfg(not(unix))]
    {
        let _ = (bytes, printed);
        UserInterruptGuard {}
    }
}

/// Build the rendered interrupt notice (with a leading newline so the
/// terminal's echoed `^C` does not share a line) for async-signal-safe
/// `libc::write(2)` emission from the SIGINT handler.
///
/// At install time we have only the user's CLI argument (e.g. the
/// relative path they typed). We use that verbatim as the OSC8 visible
/// text, and best-effort canonicalise it against the current working
/// directory to produce an absolute `file://` link target. If
/// canonicalisation fails (path doesn't exist yet, permission denied,
/// etc.) we fall back to a plain (non-hyperlinked) prose line.
fn format_user_interrupt_message(prompt_argv: &str) -> String {
    use biscuit_terminal::components::renderable::Renderable;
    use biscuit_terminal::components::status::{Status, StatusState};

    let absolute = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join(prompt_argv))
        .and_then(|p| p.canonicalize().ok())
        .map(|p| p.display().to_string());

    let prose = if let Some(absolute) = absolute {
        format!("User interrupted compose operation in [{prompt_argv}](file://{absolute})")
    } else {
        format!("User interrupted compose operation in <yellow>{prompt_argv}</yellow>")
    };

    let term = crate::log::terminal();
    let body = Status::from_prose(prose)
        .state(StatusState::Info)
        .render(&term);

    format!("\n{body}")
}

/// Parse `--set` JSON/JSON5, validate it's an object, return as `serde_json::Value`.
pub(crate) fn parse_set_json(raw: Option<&str>) -> Result<Option<serde_json::Value>> {
    let Some(json_str) = raw else {
        return Ok(None);
    };
    let parsed = biscuit_file::Json5::from_str(json_str)
        .map_err(|e| eyre!("Invalid JSON/JSON5 in --set argument: {e}"))?;
    let value = parsed.value().clone();
    if !value.is_object() {
        return Err(eyre!(
            "Invalid --set argument: expected a JSON object like {{\"name\":\"Alice\"}}"
        ));
    }
    Ok(Some(value))
}

/// Parse an inline shorthand setter value: JSON5 first, string fallback.
pub(crate) fn parse_shorthand_value(raw: &str) -> serde_json::Value {
    if raw.is_empty() {
        return serde_json::Value::String(String::new());
    }
    match biscuit_file::Json5::from_str(raw) {
        Ok(parsed) => parsed.value().clone(),
        Err(_) => serde_json::Value::String(raw.to_string()),
    }
}

/// Classify a positional token as a shorthand setter.
///
/// ## Returns
/// - `None` — token is not a setter (pass through as file candidate)
/// - `Some(Err)` — setter syntax recognized but invalid (empty key)
/// - `Some(Ok((key, value)))` — valid setter
pub(crate) fn parse_compose_setter(
    token: &str,
) -> Option<std::result::Result<(String, serde_json::Value), String>> {
    let eq_pos = token.find('=')?;
    let key = &token[..eq_pos];
    let raw_value = &token[eq_pos + 1..];

    if key.is_empty() {
        return Some(Err("setter key must not be empty".to_string()));
    }

    let mut chars = key.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }

    for ch in chars {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            continue;
        }
        return None;
    }

    let value = parse_shorthand_value(raw_value);
    Some(Ok((key.to_string(), value)))
}

/// Result of classifying composition positional tokens.
#[derive(Debug, Default)]
pub(crate) struct ParsedCompositionPositionals {
    pub file_ref: Option<String>,
    pub shorthand_setters: serde_json::Map<String, serde_json::Value>,
}

/// Classify positional tokens into an optional file reference and setter map.
///
/// ## Errors
/// - empty-key setter (`=foo`)
/// - multiple non-setter tokens (more than one file-ref candidate)
pub(crate) fn parse_composition_positionals(
    args: &[String],
) -> Result<ParsedCompositionPositionals> {
    let mut file_ref: Option<String> = None;
    let mut shorthand_setters = serde_json::Map::new();

    for token in args {
        match parse_compose_setter(token) {
            Some(Ok((key, value))) => {
                shorthand_setters.insert(key, value);
            }
            Some(Err(e)) => {
                return Err(eyre!("Invalid setter '{}': {}", token, e));
            }
            None => {
                if file_ref.is_some() {
                    let candidates: Vec<&str> = args
                        .iter()
                        .filter(|t| parse_compose_setter(t).is_none())
                        .map(|t| t.as_str())
                        .collect();
                    return Err(eyre!(
                        "expected at most one file reference, but got multiple: {}",
                        candidates.join(", ")
                    ));
                }
                file_ref = Some(token.clone());
            }
        }
    }

    Ok(ParsedCompositionPositionals {
        file_ref,
        shorthand_setters,
    })
}

/// Merge `--set` JSON with shorthand setters. Shorthand wins on overlapping keys.
///
/// ## Returns
/// - `Ok(None)` when both sources are empty
/// - `Ok(Some(Value::Object(...)))` otherwise
pub(crate) fn merge_set_overrides(
    raw_set: Option<&str>,
    shorthand: serde_json::Map<String, serde_json::Value>,
) -> Result<Option<serde_json::Value>> {
    let base = parse_set_json(raw_set)?;
    let mut map = match base {
        Some(serde_json::Value::Object(m)) => m,
        Some(_) => unreachable!("parse_set_json enforces object shape"),
        None => serde_json::Map::new(),
    };
    for (key, value) in shorthand {
        map.insert(key, value);
    }
    if map.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::Value::Object(map)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn s(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    // ── parse_shorthand_value ────────────────────────────────────────

    #[test]
    fn shorthand_value_empty_string() {
        assert_eq!(parse_shorthand_value(""), Value::String(String::new()));
    }

    #[test]
    fn shorthand_value_number() {
        assert_eq!(parse_shorthand_value("3"), json!(3));
    }

    #[test]
    fn shorthand_value_boolean() {
        assert_eq!(parse_shorthand_value("true"), json!(true));
    }

    #[test]
    fn shorthand_value_array() {
        assert_eq!(parse_shorthand_value(r#"["a","b"]"#), json!(["a", "b"]));
    }

    #[test]
    fn shorthand_value_object_json5() {
        assert_eq!(
            parse_shorthand_value(r#"{mode:"fast"}"#),
            json!({"mode": "fast"})
        );
    }

    #[test]
    fn shorthand_value_plain_string_fallback() {
        assert_eq!(
            parse_shorthand_value("review.md"),
            Value::String("review.md".into())
        );
    }

    #[test]
    fn shorthand_value_url_fallback() {
        assert_eq!(
            parse_shorthand_value("https://x/?a=b"),
            Value::String("https://x/?a=b".into())
        );
    }

    // ── parse_compose_setter ─────────────────────────────────────────

    #[test]
    fn setter_valid_string() {
        let res = parse_compose_setter("review=review.md").unwrap().unwrap();
        assert_eq!(res, ("review".into(), Value::String("review.md".into())));
    }

    #[test]
    fn setter_underscore_key() {
        let res = parse_compose_setter("_private=true").unwrap().unwrap();
        assert_eq!(res, ("_private".into(), json!(true)));
    }

    #[test]
    fn setter_hyphen_key() {
        let res = parse_compose_setter("my-key=value").unwrap().unwrap();
        assert_eq!(res, ("my-key".into(), Value::String("value".into())));
    }

    #[test]
    fn setter_empty_value() {
        let res = parse_compose_setter("key=").unwrap().unwrap();
        assert_eq!(res, ("key".into(), Value::String("".into())));
    }

    #[test]
    fn setter_first_eq_split() {
        let res = parse_compose_setter("url=https://x/?a=b").unwrap().unwrap();
        assert_eq!(res, ("url".into(), Value::String("https://x/?a=b".into())));
    }

    #[test]
    fn setter_empty_key_errors() {
        let res = parse_compose_setter("=foo").unwrap();
        assert!(res.is_err());
    }

    #[test]
    fn setter_digit_start_rejected() {
        assert!(parse_compose_setter("9key=value").is_none());
    }

    #[test]
    fn setter_dot_path_rejected() {
        assert!(parse_compose_setter("foo.bar=baz").is_none());
    }

    #[test]
    fn setter_slash_key_rejected() {
        assert!(parse_compose_setter("/path=val").is_none());
    }

    #[test]
    fn setter_no_equals_rejected() {
        assert!(parse_compose_setter("file.md").is_none());
    }

    // ── parse_composition_positionals ────────────────────────────────

    #[test]
    fn positionals_file_only() {
        let parsed = parse_composition_positionals(&s(&["file.md"])).unwrap();
        assert_eq!(parsed.file_ref.as_deref(), Some("file.md"));
        assert!(parsed.shorthand_setters.is_empty());
    }

    #[test]
    fn positionals_setter_only() {
        let parsed = parse_composition_positionals(&s(&["key=val"])).unwrap();
        assert!(parsed.file_ref.is_none());
        assert_eq!(
            parsed.shorthand_setters.get("key"),
            Some(&Value::String("val".into()))
        );
    }

    #[test]
    fn positionals_file_then_setter() {
        let parsed = parse_composition_positionals(&s(&["file.md", "key=val"])).unwrap();
        assert_eq!(parsed.file_ref.as_deref(), Some("file.md"));
        assert_eq!(
            parsed.shorthand_setters.get("key"),
            Some(&Value::String("val".into()))
        );
    }

    #[test]
    fn positionals_setter_then_file() {
        let parsed = parse_composition_positionals(&s(&["key=val", "file.md"])).unwrap();
        assert_eq!(parsed.file_ref.as_deref(), Some("file.md"));
        assert_eq!(
            parsed.shorthand_setters.get("key"),
            Some(&Value::String("val".into()))
        );
    }

    #[test]
    fn positionals_multiple_setters_around_file() {
        let parsed = parse_composition_positionals(&s(&["a=1", "file.md", "b=2"])).unwrap();
        assert_eq!(parsed.file_ref.as_deref(), Some("file.md"));
        assert_eq!(parsed.shorthand_setters.get("a"), Some(&json!(1)));
        assert_eq!(parsed.shorthand_setters.get("b"), Some(&json!(2)));
    }

    #[test]
    fn positionals_duplicate_setter_last_wins() {
        let parsed = parse_composition_positionals(&s(&["k=old", "k=new"])).unwrap();
        assert_eq!(
            parsed.shorthand_setters.get("k"),
            Some(&Value::String("new".into()))
        );
    }

    #[test]
    fn positionals_two_files_errors() {
        let err = parse_composition_positionals(&s(&["a.md", "b.md"])).unwrap_err();
        assert!(err.to_string().contains("multiple"));
    }

    #[test]
    fn positionals_empty_key_errors() {
        let err = parse_composition_positionals(&s(&["=foo"])).unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn positionals_dot_path_is_file_candidate() {
        let parsed = parse_composition_positionals(&s(&["foo.bar=baz"])).unwrap();
        assert_eq!(parsed.file_ref.as_deref(), Some("foo.bar=baz"));
        assert!(parsed.shorthand_setters.is_empty());
    }

    // ── merge_set_overrides ──────────────────────────────────────────

    #[test]
    fn merge_both_empty() {
        let result = merge_set_overrides(None, serde_json::Map::new()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn merge_set_only() {
        let result = merge_set_overrides(Some(r#"{"a":"b"}"#), serde_json::Map::new()).unwrap();
        assert_eq!(result, Some(json!({"a": "b"})));
    }

    #[test]
    fn merge_shorthand_only() {
        let mut short = serde_json::Map::new();
        short.insert("k".into(), Value::String("v".into()));
        let result = merge_set_overrides(None, short).unwrap();
        assert_eq!(result, Some(json!({"k": "v"})));
    }

    #[test]
    fn merge_shorthand_wins() {
        let mut short = serde_json::Map::new();
        short.insert("k".into(), Value::String("new".into()));
        let result = merge_set_overrides(Some(r#"{"k":"old"}"#), short).unwrap();
        assert_eq!(result, Some(json!({"k": "new"})));
    }

    #[test]
    fn merge_disjoint() {
        let mut short = serde_json::Map::new();
        short.insert("b".into(), json!(2));
        let result = merge_set_overrides(Some(r#"{"a":"1"}"#), short).unwrap();
        assert_eq!(result, Some(json!({"a": "1", "b": 2})));
    }

    // ── SIGINT / Ctrl+C during prep (Phase 5) ────────────────────────

    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn sigint_during_prep_sets_interrupt_flag_and_renders_notice() {
        // Ensure the global flag starts clean and will be restored on exit.
        crate::output::clear_user_interrupt_for_tests();
        assert!(!crate::output::user_interrupt_observed());

        let prompt = "prompts/test.md";
        let _guard = install_user_interrupt_guard(prompt);

        // Deliver SIGINT to ourselves. The handler must be async-signal-safe
        // and may run on this thread or a signal-delivery thread.
        unsafe {
            libc::kill(libc::getpid(), libc::SIGINT);
        }

        // Give the kernel a moment to deliver the signal.
        std::thread::sleep(std::time::Duration::from_millis(50));

        assert!(
            crate::output::user_interrupt_observed(),
            "SIGINT should set the user-interrupt flag"
        );

        // The notice formatting should produce a non-empty string containing
        // the prompt argument.
        let notice = format_user_interrupt_message(prompt);
        assert!(
            notice.contains("User interrupted compose operation"),
            "notice should contain the interrupt message"
        );
        assert!(
            notice.contains(prompt),
            "notice should reference the prompt file"
        );

        // Clean up so later tests in the same process see a clean flag.
        crate::output::clear_user_interrupt_for_tests();
    }
}
