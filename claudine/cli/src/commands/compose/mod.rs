//! Top-level composition commands.
//!
//! - `claudine compose <file>` — chained composition (no file mutation)
//! - `claudine inline-compose <file>` — inline composition (replaces body)
//!
//! Both commands are thin request builders that delegate to
//! [`crate::commands::wrap::composition::execute_composition_request_inner`]
//! for wrapper-grade execution.

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
    CompositionError, CompositionMode, OutputFormat as CompositionOutputFormat, PrepareOptions,
    PreparedComposition, ResolvedCompositionSource, ResolvedSessionInteractivity,
};
use claudine::provider::Provider;
use claudine::system_prompt::SystemPromptArgs;
use color_eyre::eyre::{Result, eyre};

use crate::provider_values::provider_value_parser;

mod interrupt;
mod loop_run;
pub(crate) mod prep;
mod setters;

pub(crate) use setters::{json_type_name, merge_set_overrides, parse_composition_positionals};

/// Shared flags for composition commands.
///
/// ## Notes
///
/// The seven provider boolean fields (`claude`, `codex`, `gemini`, `goose`,
/// `kimicode`, `opencode`, `qwen`) are handled entirely by the
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

    /// OpenCode stalled-generation backstop budget (e.g. `10m`). Aborts when
    /// OpenCode repeatedly drops generations with no progress for this long.
    /// OpenCode-scoped, structured-stream only; `0s` disables. Default `10m`.
    #[arg(long = "stall-timeout", value_name = "DURATION")]
    pub stall_timeout: Option<String>,

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

    /// Provider-argument tail forwarded to the underlying agent, captured by
    /// the pre-clap ownership partition ([`crate::argv::partition_composition_tail`]).
    /// Not parsed by clap — populated in `main` after the parse from the
    /// partitioned argv, so it is deliberately excluded from the CLI surface.
    #[arg(skip)]
    pub provider_args: Vec<String>,

    /// `true` when [`Self::provider_args`] came from an explicit `--` boundary
    /// (opaque, unclassified) rather than an implicit non-Claudine switch.
    #[arg(skip)]
    pub provider_args_explicit: bool,
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

    /// Parse `--stall-timeout DURATION` into seconds using the same
    /// [`claudine::harness::parse_timeout_allow_zero`] grammar frontmatter
    /// uses, so CLI and frontmatter errors share one vocabulary. A `0s` literal
    /// is the disable sentinel and parses to `Some(0)`; a fractional value such
    /// as `0.5s` is a valid 500ms budget (rounded down to `Some(0)` seconds
    /// only by integer seconds truncation, never disabled). Returns `Ok(None)`
    /// when the flag was not supplied.
    pub(crate) fn stall_timeout_secs(&self) -> Result<Option<u64>> {
        match self.stall_timeout.as_deref() {
            Some(raw) => {
                let duration = claudine::harness::parse_timeout_allow_zero(
                    raw,
                    std::path::Path::new("<--stall-timeout>"),
                )
                .map_err(|e| eyre!("invalid --stall-timeout value: {e}"))?;
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

/// Identifies which composition command is running so the shared scaffold
/// can branch on the small set of command-specific differences.
#[derive(Debug, Clone, Copy)]
pub(crate) enum CompositionKind {
    /// `claudine compose` — chained document, no file mutation.
    Direct,
    /// `claudine inline-compose` — frontmatter prompt replaces body.
    Inline,
}

/// Prompt-property state captured during inline-compose source validation
/// and consumed by deferred post-header reporting.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InlinePromptState {
    /// Whether the frontmatter contained any `prompt` key.
    pub has_prompt: bool,
    /// Whether the `prompt` value is a non-empty string.
    pub is_non_empty: bool,
}

impl CompositionKind {
    /// Returns `true` for `inline-compose`.
    pub(crate) fn is_inline(self) -> bool {
        matches!(self, Self::Inline)
    }

    /// Execution mode passed to the wrapper pipeline.
    pub(crate) fn mode(self) -> CompositionMode {
        match self {
            Self::Direct => CompositionMode::ChainedDocument,
            Self::Inline => CompositionMode::InlineFrontmatterPrompt,
        }
    }

    /// Schema-aware prepare function for this command.
    pub(crate) fn prepare_with_schema(
        self,
        source: &ResolvedCompositionSource,
        options: PrepareOptions,
    ) -> Result<PreparedComposition, CompositionError> {
        match self {
            Self::Direct => claudine::composition::prepare_direct_with_schema(source, options),
            Self::Inline => claudine::composition::prepare_inline_with_schema(source, options),
        }
    }

    /// Prepare through the canonical document service at an explicit schema
    /// stage.
    ///
    /// Preferred over [`prepare_with_schema`](Self::prepare_with_schema) on the
    /// single-execution path: a document whose own `initialize` may add or repair
    /// a schema property must not be judged before that event runs (R4), and the
    /// canonical service records the deferral on the prepared composition so the
    /// harness knows a stabilized reread is still owed.
    ///
    /// ## Errors
    ///
    /// See [`prepare_with_schema`](Self::prepare_with_schema). A deferred stage
    /// surfaces every non-schema preparation failure and no schema verdict.
    pub(crate) fn prepare_staged(
        self,
        source: &ResolvedCompositionSource,
        options: PrepareOptions,
        entry: claudine::composition::DocumentEntryReason,
        schema: claudine::composition::SchemaStage,
    ) -> Result<PreparedComposition, CompositionError> {
        claudine::composition::prepare_document(claudine::composition::DocumentPreparation {
            entry,
            mode: self.mode(),
            source,
            prompt_source: claudine::composition::PromptSource::ComposedBody,
            schema,
            options,
        })
    }

    /// Schema-agnostic prepare function for this command.
    ///
    /// Used on loop iterations after the first, where the seed pass has
    /// already validated the frontmatter against `$schema` and we only need
    /// to re-compose with the current override state.
    pub(crate) fn prepare_without_schema(
        self,
        source: &ResolvedCompositionSource,
        options: PrepareOptions,
    ) -> Result<PreparedComposition, CompositionError> {
        match self {
            Self::Direct => claudine::composition::prepare_direct(source, options),
            Self::Inline => claudine::composition::prepare_inline(source, options),
        }
    }

    /// Command-specific validation that runs immediately after source
    /// resolution.
    ///
    /// For `inline-compose` this enforces the inline/sequence contract and
    /// the prompt-property contract, returning prompt state for deferred
    /// reporting. For `compose` this is a no-op.
    ///
    /// ## Errors
    ///
    /// Returns [`CompositionError::InlineComposeSequenceMismatch`],
    /// [`CompositionError::PromptPropertyMissing`], or
    /// [`CompositionError::PromptPropertyWrongType`] for inline-compose
    /// contract violations.
    pub(crate) fn on_source_resolved(
        self,
        source: ResolvedCompositionSource,
        shared: &SharedComposeArgs,
    ) -> Result<(ResolvedCompositionSource, Option<InlinePromptState>), CompositionError> {
        match self {
            Self::Direct => Ok((source, None)),
            Self::Inline => {
                // Captured once for frontmatter-excerpt enrichment of any inline
                // contract error returned below; gates whether the YAML block is
                // shown (TTY or FORCE_COLOR) or withheld (pipe/CI/NO_COLOR).
                let stderr_is_tty = std::io::stderr().is_terminal()
                    || std::env::var_os("FORCE_COLOR").is_some();

                // -- Fail-fast: inline-compose / sequence mismatch ------------------
                //
                // A document authoring both a non-null `prompt` and a non-null
                // `sequence` defines an inline sequence and must run under
                // `claudine sequence`. Reject it here — after load/parse (so
                // `FrontmatterParse` keeps precedence) but before the
                // prompt-property pre-validation, schema scrubbing, overrides,
                // composition, provider selection, and execution.
                if claudine::composition::is_inline_sequence_mismatch(&source) {
                    return Err(CompositionError::InlineComposeSequenceMismatch {
                        source_path: source.resolved_path.clone(),
                    }
                    .enrich_frontmatter(&source, stderr_is_tty));
                }

                // -- Pre-validation: prompt frontmatter property --------------------
                //
                // This MUST run before the schema-aware pre-validation.
                // Otherwise a schema like `$schema: { prompt: string }` with
                // `prompt: 123` would be dropped by the invalid-optional scrub,
                // and inline-compose would then incorrectly report
                // `PromptPropertyMissing` instead of `PromptPropertyWrongType`
                // — masking the real problem.
                let prompt_value =
                    source.markdown.frontmatter().as_map().get("prompt").cloned();
                let has_prompt = prompt_value.is_some();
                let is_non_empty = prompt_value
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| !s.trim().is_empty());

                // Fail-fast diagnostics (missing / present-but-empty) print before
                // the header; the success line is deferred to the reporting block
                // after the early header so nothing precedes the execution line.
                if !(shared.silent || (has_prompt && is_non_empty)) {
                    let term = crate::log::terminal();
                    claudine::harness::report::report_prompt_property(
                        has_prompt, is_non_empty, &term,
                    );
                }

                // Drive the inline-specific contract eagerly so a wrong-type or
                // missing `prompt` produces the right typed error before any
                // schema scrubbing kicks in.
                if !has_prompt {
                    return Err(CompositionError::PromptPropertyMissing
                        .enrich_frontmatter(&source, stderr_is_tty));
                }
                if let Some(ref value) = prompt_value
                    && !matches!(value, serde_json::Value::String(_))
                {
                    return Err(CompositionError::PromptPropertyWrongType(
                        json_type_name(value).to_string(),
                    )
                    .enrich_frontmatter(&source, stderr_is_tty));
                }

                Ok((
                    source,
                    Some(InlinePromptState {
                        has_prompt,
                        is_non_empty,
                    }),
                ))
            }
        }
    }

    /// Command-specific reporting that runs after the execution header is
    /// emitted.
    ///
    /// For `inline-compose` this emits the deferred source-file and
    /// prompt-property success lines after the header. For `compose` this
    /// is a no-op.
    pub(crate) fn on_header_emitted(
        self,
        source: &ResolvedCompositionSource,
        shared: &SharedComposeArgs,
        file: &str,
        inline_state: Option<InlinePromptState>,
    ) -> Result<(), CompositionError> {
        match self {
            Self::Direct => Ok(()),
            Self::Inline => {
                // Deferred success reports: the execution header has now been
                // emitted, so the source-file and prompt-property confirmations
                // belong here in the reporting section (success messages must
                // not precede the header). The missing / present-but-empty
                // variants already printed fail-fast in `on_source_resolved`.
                if !shared.silent {
                    let term = crate::log::terminal();
                    claudine::harness::report::report_source_file(file, &source.resolved_path, &term);
                    if let Some(state) = inline_state
                        && state.has_prompt
                        && state.is_non_empty
                    {
                        claudine::harness::report::report_prompt_property(
                            state.has_prompt,
                            state.is_non_empty,
                            &term,
                        );
                    }
                }
                Ok(())
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
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<i32> {
    let ComposeArgs { shared, args } = args;
    prep::run_composition_inner(
        shared,
        args,
        verbose,
        startup_timings,
        CompositionKind::Direct,
    )
}

fn run_inline_compose_inner(
    args: InlineComposeArgs,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<i32> {
    let InlineComposeArgs { shared, args } = args;
    prep::run_composition_inner(
        shared,
        args,
        verbose,
        startup_timings,
        CompositionKind::Inline,
    )
}

#[cfg(test)]
mod tests;
