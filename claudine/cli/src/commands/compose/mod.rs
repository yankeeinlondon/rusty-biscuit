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
use std::io::IsTerminal;

use clap::Args;
use claudine::composition::{
    self, CompositionError, CompositionExecutionRequest, CompositionMode,
    OutputFormat as CompositionOutputFormat, ResolvedSessionInteractivity,
};
use claudine::provider::Provider;
use claudine::system_prompt::SystemPromptArgs;
use color_eyre::eyre::{Result, eyre};
use tracing::info_span;

use super::wrap::composition::execute_composition_request;
use crate::provider_values::provider_value_parser;

mod interrupt;
mod loop_run;
mod setters;

pub(crate) use interrupt::{USER_INTERRUPT_EXIT_CODE, install_user_interrupt_guard};
pub(crate) use loop_run::{
    build_loop_iteration_output, emit_compose_warnings, emit_rate_limit_halt, record_prep_substage,
    run_loop_with_overrides,
};
pub(crate) use setters::{json_type_name, merge_set_overrides, parse_composition_positionals};

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

    /// Run the provider session in non-interactive mode.
    #[arg(long, conflicts_with = "interactive")]
    pub no_interactive: bool,

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

    /// Resolve the session interactivity from CLI flags and frontmatter.
    ///
    /// Precedence (highest → lowest): `--no-interactive` flag, `-i` / `--interactive`
    /// flag, authored `interactive` frontmatter value, then default non-interactive.
    pub(crate) fn resolve_session_interactivity(
        &self,
        frontmatter_interactive: Option<bool>,
    ) -> ResolvedSessionInteractivity {
        if self.no_interactive {
            ResolvedSessionInteractivity {
                value: false,
                source: claudine::composition::SessionInteractivitySource::NoInteractiveFlag,
            }
        } else if self.interactive {
            ResolvedSessionInteractivity {
                value: true,
                source: claudine::composition::SessionInteractivitySource::InteractiveFlag,
            }
        } else if let Some(value) = frontmatter_interactive {
            ResolvedSessionInteractivity {
                value,
                source: claudine::composition::SessionInteractivitySource::Frontmatter,
            }
        } else {
            ResolvedSessionInteractivity {
                value: false,
                source: claudine::composition::SessionInteractivitySource::Default,
            }
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
    let perf_enabled = shared.perf;
    let mut prep_substages: Vec<crate::perf::SubstageTiming> = Vec::new();
    let parsed = parse_composition_positionals(&args)?;
    let file = parsed.file_ref.ok_or_else(|| {
        eyre!("missing file reference: expected exactly one file reference plus optional key=value setters")
    })?;

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

    let prep_t = std::time::Instant::now();
    let source = composition::resolve_composition_source(&file)?;
    record_prep_substage(&mut prep_substages, perf_enabled, "frontmatter load", prep_t);

    // Schema-aware pre-prepare validation. Runs BEFORE the preflight
    // compose pass so the user-visible error surface is Claudine's
    // typed `CompositionError` rather than Darkmatter's raw
    // `MarkdownError::SchemaValidationFailed`. Drives interactive
    // collection of missing required values when allowed and merges the
    // collected values into `set_overrides`. Invalid optional values are
    // dropped in place. Schema-load failures, invalid required values,
    // and missing required values (non-interactive) surface as typed
    // errors here, never as Darkmatter raw errors.
    let schema_t = std::time::Instant::now();
    let (source, set_overrides) = {
        let interactive_opts =
            super::schema_interactive::resolve_interactive_options(shared.silent);
        let term = crate::log::terminal();
        let pre = super::schema_interactive::pre_validate_with_interactive_collection(
            &source,
            set_overrides.as_ref(),
            interactive_opts,
            &term,
        )?;
        super::schema_interactive::emit_dropped_optional_warnings(&pre.dropped_optionals);
        (pre.source, pre.set_overrides)
    };
    // Includes any interactive collection wait when stdin is a TTY; in the
    // common non-interactive / dry-run `--perf` case this is pure validation.
    record_prep_substage(&mut prep_substages, perf_enabled, "schema validation", schema_t);

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
    let prep_ctx_t = std::time::Instant::now();
    let prep_context = super::wrap::composition::CompositionPrepContext::new(
        &file,
        &source.resolved_path,
        &shared.excluded(),
    )?;
    record_prep_substage(&mut prep_substages, perf_enabled, "prep context", prep_ctx_t);

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
            shared.dry_run,
            &source.resolved_path,
        )?
    };

    let mut env_overrides: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    if let Some(ref target) = resolved_target {
        super::wrap::composition::install_agent_env_for_composition(target, shared.yolo, &mut env_overrides);
    }

    // Render the execution line the moment the agent is known. Eager
    // resolution above prompts the interactive picker when the agent is
    // ambiguous, so by this point the target is settled for every real
    // run — and this lands *before* the expensive prepare/compose work, so
    // the user sees immediate feedback. The lone case without a target is
    // a `--dry-run` with an unresolved agent; the executor emits the
    // header itself there after rendering the unresolved state.
    //
    // The header must describe the *resolved* session mode, not the raw
    // `-i` flag: a document with `interactive: true` and no flag still runs
    // interactively. `raw_hints.interactive` carries the authored
    // frontmatter value, so resolving it here keeps the eager header in
    // agreement with the executor's `session_interactive`.
    let header_interactive =
        shared.resolve_session_interactivity(raw_hints.interactive).value;
    let header_emitted = match (shared.silent, resolved_target.as_ref()) {
        (false, Some(target)) => super::wrap::composition::emit_execution_header(
            target.provider,
            shared.yolo,
            header_interactive,
            verbose > 0,
            shared.repo,
            false, // is_inline
            false, // sequence
            shared.operation.as_deref(),
            &file,
            prep_context.launch_workspace.package_context.clone(),
            &crate::log::terminal(),
        ),
        _ => false,
    };

    // ── Pre-flight shell approval ────────────────────────────────────
    //
    // `ComposeContext::capture()` + `env_mut().insert(...)` installs the
    // resolved `AGENT` (and any other composition env overrides) into
    // the same Darkmatter compose pipeline that the preflight pass
    // walks. Without this, frontmatter values that derive from env
    // (e.g. `runtime_agent: '{{ env.AGENT }}'`) fail Darkmatter's
    // built-in schema validation during preflight, before reaching
    // `prepare_direct_with_schema`.
    let compose_options = {
        let mut ctx = darkmatter::markdown::compose::ComposeContext::capture();
        for (key, value) in &env_overrides {
            ctx.env_mut().insert(key.clone(), value.clone());
        }
        let mut opts = darkmatter::markdown::compose::ComposeOptions::new_with_context(ctx)
            .with_source_file(&source.resolved_path);
        if let Some(ref overrides) = set_overrides {
            opts = opts.with_set_overrides(overrides.clone());
        }
        opts
    };

    let shared_approval_cache =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

    let approval_options = super::wrap::apply_composition_shell_overrides(
        super::wrap::build_harness_shell_options_with_cache(
            &source.resolved_path,
            prep_context.source_repo_root.as_deref(),
            Some(std::sync::Arc::clone(&shared_approval_cache)),
        ),
        shared.dry_run,
        shared.yolo,
    );

    let shell_approval_t = std::time::Instant::now();
    let preflight = {
        let _span = info_span!("compose_prep.shell_preflight").entered();
        composition::resolve_shell_approvals(
            Some(&source.markdown),
            Some(&compose_options),
            None,
            &approval_options,
        )?
    };
    record_prep_substage(&mut prep_substages, perf_enabled, "shell approval", shell_approval_t);

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
        pause_reset_margin: claudine::composition::resolve_pause_reset_margin_from_env(),
    };

    // `--dry-run` bypasses the iteration engine entirely: a single
    // composition + single render (Decision 4). Loop detection is skipped so
    // a doc with `loop:` frontmatter renders once rather than iterating.
    let file_for_loop = file.clone();
    let loop_prepare_options = composition::PrepareOptions {
        set_overrides: set_overrides.clone(),
        pre_approved_commands: Some(preflight.approved_commands.clone()),
        env_overrides: env_overrides.clone(),
        perf_enabled: shared.perf,
        source_repo_root: prep_context.source_repo_root.clone(),
        shell_working_directory: Some(
            prep_context.launch_workspace.child_cwd.clone(),
        ),
    };
    if !shared.dry_run && let Some(loop_result) =
        run_loop_with_overrides(&source, &loop_prepare_options, loop_options, |ctx| {
            let prepared = {
                let _span = info_span!("compose_prep.prepare_direct").entered();
                // Schema-aware variant: typed `SchemaLoad` /
                // `SchemaValidation` / `MissingProperties` errors and
                // drop-and-retry for invalid optionals apply per
                // iteration. Interactive collection is NOT driven inside
                // loops; missing required values surface as
                // `MissingProperties` on the first iteration.
                let mut iteration_options = loop_prepare_options.clone();
                iteration_options.set_overrides = Some(ctx.as_set_overrides());
                composition::prepare_direct_with_schema(
                    &source,
                    iteration_options,
                )?
            };

            emit_compose_warnings(&prepared.warnings, shared.silent);

            let request = {
                let resolved = shared.resolve_session_interactivity(prepared.selection_hints.interactive);
                CompositionExecutionRequest {
                    mode: CompositionMode::ChainedDocument,
                    file_ref: file_for_loop.clone(),
                    prepared,
                    resolved_target: resolved_target.clone(),
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
                    session_interactive: resolved.value,
                    session_interactive_source: resolved.source,
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
                    header_emitted,
                }
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
    // Pre-validation (with interactive collection) has already run, so
    // schema requirements are satisfied. Call the schema-aware prepare
    // directly — typed errors still surface for any residual issues
    // (e.g. drop-and-retry on a latent optional).
    let prepared = {
        let _span = info_span!("compose_prep.prepare_direct").entered();
        composition::prepare_direct_with_schema(
            &source,
            composition::PrepareOptions {
                set_overrides,
                pre_approved_commands: Some(preflight.approved_commands),
                env_overrides: env_overrides.clone(),
                perf_enabled: shared.perf,
                source_repo_root: prep_context.source_repo_root.clone(),
                shell_working_directory: Some(prep_context.launch_workspace.child_cwd.clone()),
            },
        )?
    };
    super::schema_interactive::emit_dropped_optional_warnings(&prepared.dropped_optionals);
    emit_compose_warnings(&prepared.warnings, shared.silent);

    let request = {
        let resolved = shared.resolve_session_interactivity(prepared.selection_hints.interactive);
        CompositionExecutionRequest {
            mode: CompositionMode::ChainedDocument,
            file_ref: file,
            prepared,
            resolved_target,
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
            session_interactive: resolved.value,
            session_interactive_source: resolved.source,
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
            header_emitted,
        }
    };

    if let Some(ref mut timings) = startup_timings {
        timings.prep_phase = compose_entry.elapsed();
        timings.prep_substages = prep_substages;
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
    let perf_enabled = shared.perf;
    let mut prep_substages: Vec<crate::perf::SubstageTiming> = Vec::new();
    let parsed = parse_composition_positionals(&args)?;
    let file = parsed.file_ref.ok_or_else(|| {
        eyre!("missing file reference: expected exactly one file reference plus optional key=value setters")
    })?;

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

    let frontmatter_load_t = std::time::Instant::now();
    let source = match composition::resolve_composition_source(&file) {
        Ok(source) => {
            // The "resolved source file" success line is deferred until
            // after the execution header (see the reporting block below the
            // early header) so nothing prints before the header. The
            // not-found error path stays fail-fast in the `Err` arm.
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
            // Only a genuine reference-resolution failure means the file was
            // not found. A file that resolved but failed to load/parse (e.g.
            // malformed frontmatter) must not be reported as "no match" — its
            // own typed error already names the file and the cause.
            if let Some(ref t) = term
                && matches!(
                    e,
                    CompositionError::FileNotFound(_) | CompositionError::InvalidReference(_)
                )
            {
                claudine::harness::report::report_source_file(&file, std::path::Path::new(""), t);
            }
            return Err(e.into());
        }
    };
    record_prep_substage(
        &mut prep_substages,
        perf_enabled,
        "frontmatter load",
        frontmatter_load_t,
    );

    // -- Fail-fast: inline-compose / sequence mismatch ----------------------
    //
    // A document authoring both a non-null `prompt` and a non-null `sequence`
    // defines an inline sequence and must run under `claudine sequence`. Reject
    // it here — after load/parse (so `FrontmatterParse` keeps precedence) but
    // before the prompt-property pre-validation, schema scrubbing, overrides,
    // composition, provider selection, and execution. Detection reads authored
    // frontmatter only; `set_overrides` never participate.
    if composition::is_inline_sequence_mismatch(&source) {
        let raw_yaml =
            composition::capture_frontmatter_yaml(&source.original_text).unwrap_or_default();
        let stderr_is_tty = std::io::stderr().is_terminal();
        return Err(CompositionError::InlineComposeSequenceMismatch {
            source_path: source.resolved_path.clone(),
            raw_yaml,
            stderr_is_tty,
        }
        .into());
    }

    // -- Pre-validation: prompt frontmatter property ------------------------
    //
    // This MUST run before the schema-aware pre-validation. Otherwise a
    // schema like `$schema: { prompt: string }` with `prompt: 123` would
    // be dropped by the invalid-optional scrub, and inline-compose would
    // then incorrectly report `PromptPropertyMissing` instead of
    // `PromptPropertyWrongType` — masking the real problem.
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

    // Fail-fast diagnostics (missing / present-but-empty) print before the
    // header; the success line is deferred to the reporting block after the
    // early header so nothing precedes the execution line.
    if let Some(ref t) = term
        && !(has_prompt && is_non_empty)
    {
        claudine::harness::report::report_prompt_property(has_prompt, is_non_empty, t);
    }

    // Drive the inline-specific contract eagerly so a wrong-type or
    // missing `prompt` produces the right typed error before any schema
    // scrubbing kicks in.
    if !has_prompt {
        return Err(CompositionError::PromptPropertyMissing.into());
    }
    if let Some(value) = prompt_value.as_ref()
        && !matches!(value, serde_json::Value::String(_))
    {
        return Err(CompositionError::PromptPropertyWrongType(
            json_type_name(value).to_string(),
        )
        .into());
    }

    // Schema-aware pre-prepare validation. Runs AFTER the prompt-property
    // check so the inline-compose contract takes precedence over the
    // generic schema check. See the same call in `run_compose_inner` for
    // the full rationale.
    let schema_t = std::time::Instant::now();
    let (source, set_overrides) = {
        let interactive_opts =
            super::schema_interactive::resolve_interactive_options(shared.silent);
        let term_for_schema = crate::log::terminal();
        let pre = super::schema_interactive::pre_validate_with_interactive_collection(
            &source,
            set_overrides.as_ref(),
            interactive_opts,
            &term_for_schema,
        )?;
        super::schema_interactive::emit_dropped_optional_warnings(&pre.dropped_optionals);
        (pre.source, pre.set_overrides)
    };
    record_prep_substage(&mut prep_substages, perf_enabled, "schema validation", schema_t);

    // Phase 2 (2026-05-09-slow-prep): single source-root discovery for the
    // whole inline-compose invocation; downstream prep phases reuse this
    // context rather than rediscovering the repo root.
    let prep_ctx_t = std::time::Instant::now();
    let prep_context = super::wrap::composition::CompositionPrepContext::new(
        &file,
        &source.resolved_path,
        &shared.excluded(),
    )?;
    record_prep_substage(&mut prep_substages, perf_enabled, "prep context", prep_ctx_t);

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
            shared.dry_run,
            &source.resolved_path,
        )?
    };

    let mut env_overrides: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    if let Some(ref target) = resolved_target {
        super::wrap::composition::install_agent_env_for_composition(target, shared.yolo, &mut env_overrides);
    }

    // Render the execution line the moment the agent is known — see the
    // matching block in `run_compose_inner` for the rationale. `is_inline`
    // is `true` here so the header shows the inline-compose badge. As in the
    // direct-compose block, resolve the session mode from the raw
    // frontmatter so the header reflects `interactive: true` documents.
    let header_interactive =
        shared.resolve_session_interactivity(raw_hints.interactive).value;
    let header_emitted = match (shared.silent, resolved_target.as_ref()) {
        (false, Some(target)) => super::wrap::composition::emit_execution_header(
            target.provider,
            shared.yolo,
            header_interactive,
            verbose > 0,
            shared.repo,
            true, // is_inline
            false, // sequence
            shared.operation.as_deref(),
            &file,
            prep_context.launch_workspace.package_context.clone(),
            &crate::log::terminal(),
        ),
        _ => false,
    };

    // Deferred success reports: the execution header has now been emitted,
    // so the source-file and prompt-property confirmations belong here in
    // the reporting section (success messages must not precede the header).
    // The missing / present-but-empty variants already printed fail-fast
    // above.
    if let Some(ref t) = term {
        claudine::harness::report::report_source_file(&file, &source.resolved_path, t);
        if has_prompt && is_non_empty {
            claudine::harness::report::report_prompt_property(has_prompt, is_non_empty, t);
        }
    }

    // ── Pre-flight shell approval ────────────────────────────────────
    //
    // See the matching block in `run_compose_inner` for why we install
    // `env_overrides` into the compose context: it lets template values
    // like `{{ env.AGENT }}` resolve during preflight's schema check.
    let compose_options = {
        let mut ctx = darkmatter::markdown::compose::ComposeContext::capture();
        for (key, value) in &env_overrides {
            ctx.env_mut().insert(key.clone(), value.clone());
        }
        let mut opts = darkmatter::markdown::compose::ComposeOptions::new_with_context(ctx)
            .with_source_file(&source.resolved_path);
        if let Some(ref overrides) = set_overrides {
            opts = opts.with_set_overrides(overrides.clone());
        }
        opts
    };

    let shared_approval_cache =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

    let approval_options = super::wrap::apply_composition_shell_overrides(
        super::wrap::build_harness_shell_options_with_cache(
            &source.resolved_path,
            prep_context.source_repo_root.as_deref(),
            Some(std::sync::Arc::clone(&shared_approval_cache)),
        ),
        shared.dry_run,
        shared.yolo,
    );

    let shell_approval_t = std::time::Instant::now();
    let preflight = {
        let _span = info_span!("compose_prep.shell_preflight").entered();
        composition::resolve_shell_approvals(
            Some(&source.markdown),
            Some(&compose_options),
            None,
            &approval_options,
        )?
    };
    record_prep_substage(&mut prep_substages, perf_enabled, "shell approval", shell_approval_t);

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
        pause_reset_margin: claudine::composition::resolve_pause_reset_margin_from_env(),
    };

    // `--dry-run` bypasses the iteration engine entirely: a single
    // composition + single render (Decision 4). Loop detection is skipped so
    // a doc with `loop:` frontmatter renders once rather than iterating. The
    // single-render path (below) reaches the post-preflight dry-run seam,
    // which returns before the provider launches — so the source file is
    // never mutated (Decision 2).
    let file_for_loop = file.clone();
    let loop_prepare_options = composition::PrepareOptions {
        set_overrides: set_overrides.clone(),
        pre_approved_commands: Some(preflight.approved_commands.clone()),
        env_overrides: env_overrides.clone(),
        perf_enabled: shared.perf,
        source_repo_root: prep_context.source_repo_root.clone(),
        shell_working_directory: Some(
            prep_context.launch_workspace.child_cwd.clone(),
        ),
    };
    if !shared.dry_run && let Some(loop_result) =
        run_loop_with_overrides(&source, &loop_prepare_options, loop_options, |ctx| {
            let prepared = {
                let _span = info_span!("compose_prep.prepare_inline").entered();
                // Schema-aware variant: typed `SchemaLoad` /
                // `SchemaValidation` / `MissingProperties` errors and
                // drop-and-retry for invalid optionals apply per
                // iteration. Interactive collection is NOT driven inside
                // loops; missing required values surface as
                // `MissingProperties` on the first iteration.
                let mut iteration_options = loop_prepare_options.clone();
                iteration_options.set_overrides = Some(ctx.as_set_overrides());
                composition::prepare_inline_with_schema(
                    &source,
                    iteration_options,
                )?
            };

            emit_compose_warnings(&prepared.warnings, shared.silent);

            let request = {
                let resolved = shared.resolve_session_interactivity(prepared.selection_hints.interactive);
                CompositionExecutionRequest {
                    mode: CompositionMode::InlineFrontmatterPrompt,
                    file_ref: file_for_loop.clone(),
                    prepared,
                    resolved_target: resolved_target.clone(),
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
                    session_interactive: resolved.value,
                    session_interactive_source: resolved.source,
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
                    header_emitted,
                }
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
    // Pre-validation (with interactive collection) has already run, so
    // schema requirements are satisfied. Call the schema-aware prepare
    // directly for typed-error fidelity on residual issues.
    let prepared = {
        let _span = info_span!("compose_prep.prepare_inline").entered();
        composition::prepare_inline_with_schema(
            &source,
            composition::PrepareOptions {
                set_overrides,
                pre_approved_commands: Some(preflight.approved_commands),
                env_overrides: env_overrides.clone(),
                perf_enabled: shared.perf,
                source_repo_root: prep_context.source_repo_root.clone(),
                shell_working_directory: Some(prep_context.launch_workspace.child_cwd.clone()),
            },
        )?
    };
    super::schema_interactive::emit_dropped_optional_warnings(&prepared.dropped_optionals);
    emit_compose_warnings(&prepared.warnings, shared.silent);

    let request = {
        let resolved = shared.resolve_session_interactivity(prepared.selection_hints.interactive);
        CompositionExecutionRequest {
            mode: CompositionMode::InlineFrontmatterPrompt,
            file_ref: file,
            prepared,
            resolved_target,
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
            session_interactive: resolved.value,
            session_interactive_source: resolved.source,
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
            header_emitted,
        }
    };

    if let Some(ref mut timings) = startup_timings {
        timings.prep_phase = inline_compose_entry.elapsed();
        timings.prep_substages = prep_substages;
    }

    execute_composition_request(request, verbose, startup_timings, shared.perf)
}

#[cfg(test)]
mod tests;
