//! Core types for composition workflows.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposePerfReport, ComposeWarning};
use darkmatter::markdown::hash::ComputedHash;
use serde::{Deserialize, Serialize};

use super::launch_workspace::LaunchWorkspaceContext;
use super::lifecycle::LifecycleConfig;
use crate::diagnostics::DiagnosticSnapshot;
use crate::harness::shell::CachedApprovalDecision;
use crate::provider::Provider;

/// Shared approval cache that can be reused across composition runs
/// (e.g. steps of one sequence) so that previously approved commands
/// do not prompt again.
pub type SharedApprovalCache = Arc<Mutex<HashMap<String, CachedApprovalDecision>>>;

/// Which composition mode to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionMode {
    /// Use the frontmatter `prompt` property as input; replace body with output.
    InlineFrontmatterPrompt,
    /// Compose the full document and send as prompt; no file mutation.
    ChainedDocument,
}

/// A resolved and loaded composition source document.
#[derive(Debug, Clone)]
pub struct ResolvedCompositionSource {
    /// The original file reference string.
    pub original_ref: String,
    /// The resolved absolute path.
    pub resolved_path: PathBuf,
    /// The original on-disk document text.
    pub original_text: String,
    /// The parsed Markdown document.
    pub markdown: Markdown,
}

/// Validated loop configuration parsed from a prompt's `loop` frontmatter.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopConfig {
    /// The condition that decides whether loop execution continues.
    pub condition: LoopCondition,
    /// Actions applied to frontmatter after each completed iteration.
    pub actions: Vec<LoopAction>,
    /// Optional per-document positive iteration cap.
    pub max_iterations: Option<usize>,
    /// Optional per-document failure behavior.
    pub fail_fast: Option<bool>,
    /// Optional per-document policy for what to do when a completed iteration
    /// reports a provider rate-limit signal. `None` falls back to
    /// [`OnRateLimit::Pause`].
    pub on_rate_limit: Option<OnRateLimit>,
}

/// Policy for how the loop engine reacts to a rate-limit signal observed on
/// a completed iteration.
///
/// The signal is read from [`crate::stream::summary::RateLimitInfo`] when
/// `is_throttled == Some(true)`. The policy is resolved from
/// `LoopExecutionOptions.on_rate_limit` (CLI) > `LoopConfig.on_rate_limit`
/// (frontmatter) > [`OnRateLimit::Pause`] (default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnRateLimit {
    /// Sleep until `reset_at` + a small safety margin, then proceed to the
    /// next iteration. If `reset_at` is missing or already past, behave as
    /// [`OnRateLimit::Abort`] (no unbounded sleep).
    #[default]
    Pause,
    /// Halt the loop with a structured [`super::error::CompositionError::LoopRateLimited`].
    Abort,
    /// Proceed to the next iteration without pausing. Reserved for soft
    /// per-request limits that won't recur; not recommended.
    Continue,
}

impl OnRateLimit {
    /// Parse from the frontmatter string form.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "pause" => Ok(Self::Pause),
            "abort" => Ok(Self::Abort),
            "continue" => Ok(Self::Continue),
            other => Err(format!(
                "must be one of `pause`, `abort`, `continue`, got `{other}`"
            )),
        }
    }

    /// Return the canonical string form (matches the frontmatter accepted form).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Abort => "abort",
            Self::Continue => "continue",
        }
    }
}

/// A loop condition. `while` continues while truthy; `until` continues until truthy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopCondition {
    /// Continue while this expression evaluates to true.
    While(String),
    /// Continue until this expression evaluates to true.
    Until(String),
}

/// A frontmatter mutation action executed between loop iterations.
#[derive(Debug, Clone, PartialEq)]
pub enum LoopAction {
    /// Increment a numeric property by one.
    Increment(String),
    /// Decrement a numeric property by one.
    Decrement(String),
    /// Set a property to the provided value.
    Set {
        /// Frontmatter property to update.
        prop: String,
        /// New value.
        value: serde_json::Value,
    },
    /// Append a value to a property.
    Append {
        /// Frontmatter property to update.
        prop: String,
        /// Value to append.
        value: serde_json::Value,
    },
    /// Prepend a value to a property.
    Prepend {
        /// Frontmatter property to update.
        prop: String,
        /// Value to prepend.
        value: serde_json::Value,
    },
    /// Shallow-merge an object into a property.
    Merge {
        /// Frontmatter property to update.
        prop: String,
        /// Object value to merge.
        value: serde_json::Value,
    },
}

/// Ambient loop variables injected into each loop iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbientVariable {
    /// 1-based loop iteration number.
    Iteration,
    /// Whether the current iteration is the first.
    IsFirst,
    /// Whether the current iteration is expected to be the last.
    IsLast,
    /// Captured output from the previous iteration.
    LastOutput,
    /// Exit code from the previous iteration.
    LastExitCode,
}

impl AmbientVariable {
    /// Reserved ambient variable names.
    ///
    /// All loop ambient variables are namespaced under the `_loop_` prefix
    /// to avoid shadowing user-defined frontmatter properties named
    /// `iteration`, `is_first`, `last_output`, etc.
    pub const NAMES: &[&str] = super::reserved::AMBIENT_VARIABLE_NAMES;

    /// Returns true when `name` is a reserved ambient variable.
    pub fn is_reserved(name: &str) -> bool {
        Self::NAMES.contains(&name)
    }

    /// Return the frontmatter/template key for this ambient variable.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Iteration => Self::NAMES[0],
            Self::IsFirst => Self::NAMES[1],
            Self::IsLast => Self::NAMES[2],
            Self::LastOutput => Self::NAMES[3],
            Self::LastExitCode => Self::NAMES[4],
        }
    }
}

/// Why a particular provider was selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionReason {
    /// The caller explicitly specified the provider (wrapper subcommand).
    ExplicitProvider,
    /// The source document's `agent` frontmatter selected the provider.
    FrontmatterHint,
    /// The user's config favorite (`settings.linking.preference[0]`).
    ConfigFavorite,
    /// The user chose interactively.
    InteractiveChoice,
}

/// A provider selected for composition execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedProvider {
    /// The selected provider.
    pub provider: Provider,
    /// Why this provider was selected.
    pub reason: SelectionReason,
}

/// Whether the current session is interactive (TTY) or non-interactive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionMode {
    /// Interactive terminal session — picker may be shown.
    Tty,
    /// Non-interactive session — picker is forbidden; only resolving signals.
    NonTty,
}

impl fmt::Display for ResolutionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tty => write!(f, "TTY"),
            Self::NonTty => write!(f, "non-TTY"),
        }
    }
}

/// Why a composition session is interactive or non-interactive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionInteractivitySource {
    /// The `--no-interactive` CLI flag was provided.
    NoInteractiveFlag,
    /// The `-i` / `--interactive` CLI flag was provided.
    InteractiveFlag,
    /// The source document's `interactive` frontmatter property set the mode.
    Frontmatter,
    /// The default non-interactive session mode was used.
    Default,
}

impl fmt::Display for SessionInteractivitySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoInteractiveFlag => write!(f, "--no-interactive"),
            Self::InteractiveFlag => write!(f, "--interactive"),
            Self::Frontmatter => write!(f, "frontmatter"),
            Self::Default => write!(f, "default"),
        }
    }
}

/// Resolved interactivity for a composition session, including its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedSessionInteractivity {
    /// Whether the session should be interactive.
    pub value: bool,
    /// Why `value` was chosen.
    pub source: SessionInteractivitySource,
}

/// Snapshot of installed providers at command start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledProviderSnapshot {
    /// Providers that are installed and not excluded.
    pub runnable: Vec<Provider>,
    /// Providers explicitly excluded by `--exclude`.
    pub excluded: BTreeSet<Provider>,
    /// All installed providers (including excluded ones).
    pub all_installed: Vec<Provider>,
    /// Resolved binary paths for each installed provider.
    pub binary_paths: BTreeMap<Provider, PathBuf>,
}

impl InstalledProviderSnapshot {
    /// Return the resolved binary path for a provider, if known.
    pub fn binary_path(&self, provider: Provider) -> Option<&std::path::Path> {
        self.binary_paths.get(&provider).map(|p| p.as_path())
    }
}

/// Why a provider was chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderResolutionReason {
    /// Explicit `--<provider>` CLI flag.
    ExplicitFlag,
    /// Single frontmatter `agent` value resolved directly.
    FrontmatterSingle,
    /// Frontmatter `agent` list resolved to first installed match.
    FrontmatterList,
    /// Configured favorite agent resolved directly.
    FavoriteAgent,
    /// User confirmed via interactive picker.
    InteractivePicker,
    /// User confirmed via sequence review screen.
    SequenceReview,
}

/// Why a model was chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelResolutionReason {
    /// Explicit `--model` CLI flag.
    ExplicitCli,
    /// Provider-specific environment variable (e.g. `OPENCODE_MODEL`).
    ProviderEnv(&'static str),
    /// Generic `MODEL` environment variable.
    GenericEnv,
    /// Single frontmatter `model` value validated against catalog.
    FrontmatterSingle,
    /// Frontmatter `model` list resolved to first valid match.
    FrontmatterList,
    /// Provider's built-in default (no explicit model chosen).
    ProviderDefault,
    /// User confirmed via sequence review screen.
    SequenceReview,
}

/// Fully resolved provider and model for a composition run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExecutionTarget {
    /// The selected provider.
    pub provider: Provider,
    /// Why this provider was selected.
    pub provider_reason: ProviderResolutionReason,
    /// The selected model, if any (`None` means use provider default).
    pub model: Option<String>,
    /// Why this model was selected.
    pub model_reason: ModelResolutionReason,
}

/// What influenced a picker's default or ordering for a given option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerInfluence {
    /// This option matches a singular frontmatter `agent` hint.
    FrontmatterSingle,
    /// This option appears in a list-valued frontmatter `agent` hint.
    FrontmatterList,
    /// This option matches the configured favorite agent.
    FavoriteAgent,
}

/// One row in the provider picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderPickerOption {
    /// The provider for this row.
    pub provider: Provider,
    /// What signal caused this provider to be ranked here, if any.
    pub rank_reason: Option<PickerInfluence>,
}

/// A plan for showing the one-shot provider picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderPickerPlan {
    /// Options in display order (installed providers, reordered by frontmatter).
    pub options: Vec<ProviderPickerOption>,
    /// Index of the initially-highlighted option.
    pub default_index: usize,
}

/// Per-step draft for sequence review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceStepDraft {
    /// Zero-based step index.
    pub step_index: usize,
    /// Display name for the step.
    pub step_name: String,
    /// Provider picker plan for this step.
    pub provider_plan: ProviderPickerPlan,
    /// Proposed model for this step (may be `None`).
    pub proposed_model: Option<String>,
    /// Why the proposed model was chosen.
    pub model_reason: ModelResolutionReason,
    /// Whether the provider is locked by an explicit CLI flag.
    pub provider_locked: bool,
    /// Whether the model is locked by an explicit CLI flag.
    pub model_locked: bool,
    /// The resolved provider when locked (`provider_locked=true`), otherwise `None`.
    pub resolved_provider: Option<Provider>,
}

/// A typed hint for which agent(s) to use, parsed from frontmatter `agent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentHint {
    /// A single provider.
    Single(Provider),
    /// An ordered list of providers (author preference order).
    List(Vec<Provider>),
}

/// Classified outcome of resolving a frontmatter `agent` value against the
/// known provider catalog and the installed-provider snapshot.
///
/// This is the data model for both the dry-run metadata table and the live
/// execution path: every variant corresponds to a specific user-facing
/// message and a specific TTY/no-TTY behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentResolutionState {
    /// No `agent` hint was provided by the caller or the document.
    NoAgent,
    /// A single valid, installed provider was selected.
    Selected {
        /// The provider that will run the composition.
        provider: Provider,
    },
    /// A single value was given but it does not match any known provider.
    SingleInvalid {
        /// The invalid value exactly as it appeared in frontmatter.
        hint: String,
    },
    /// A single known provider was requested but is not installed on this host.
    SingleNotInstalled {
        /// The requested provider.
        provider: Provider,
    },
    /// A list-valued hint resolved to two or more installed providers.
    ListMultipleInstalled {
        /// Installed providers from the suggestion list, in declaration order.
        installed: Vec<Provider>,
        /// Known-but-not-installed providers from the list, in declaration order.
        not_installed: Vec<Provider>,
        /// Values that did not match the provider catalog, in declaration order.
        invalid: Vec<String>,
    },
    /// A list-valued hint resolved to exactly one installed provider.
    ListOneInstalled {
        /// The single installed provider that will be auto-selected.
        selected: Provider,
        /// Known-but-not-installed providers from the list, in declaration order.
        not_installed: Vec<Provider>,
        /// Values that did not match the provider catalog, in declaration order.
        invalid: Vec<String>,
    },
    /// A list-valued hint resolved to zero installed providers (all invalid,
    /// all not-installed, or a mix with nothing runnable).
    ZeroInstalledList {
        /// Known-but-not-installed providers from the list, in declaration order.
        not_installed: Vec<Provider>,
        /// Values that did not match the provider catalog, in declaration order.
        invalid: Vec<String>,
    },
}

/// A typed hint for which model(s) to use, parsed from frontmatter `model`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelHint {
    /// A single model identifier.
    Single(String),
    /// An ordered list of model identifiers (author preference order).
    List(Vec<String>),
}

/// Typed selection hints parsed from effective frontmatter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveSelectionHints {
    /// Parsed `agent` frontmatter property.
    pub agent: Option<AgentHint>,
    /// Parsed `model` frontmatter property.
    pub model: Option<ModelHint>,
    /// Parsed `interactive` frontmatter property.
    ///
    /// `None` means absent or explicitly `null`, preserving today's default
    /// non-interactive behavior.
    pub interactive: Option<bool>,
    /// `agent` strings that did not match the known provider catalog.
    ///
    /// Kept separate from [`Self::agent`] so existing resolution code can
    /// continue to operate on valid providers only while the new rendering
    /// and live-path code surfaces invalid values as styled non-fatal state.
    pub agent_invalid: Vec<String>,
    /// Whether the source `agent` frontmatter value was an array.
    ///
    /// Preserved independently of [`Self::agent`] because a list whose only
    /// entries are invalid produces `agent: None`, which would otherwise be
    /// indistinguishable from a single invalid scalar. Classification needs
    /// this to route a single-entry all-invalid list to the zero-installed
    /// list state rather than the single-invalid state.
    pub agent_was_list: bool,
}

/// The caller's input layers, reapplied at every canonical preparation of
/// every document in one invocation.
///
/// These are invocation-scoped, not document-scoped: the caller stays
/// authoritative at every document, so a proxy target, a retry, and a loop
/// refresh all prepare with the same layers the caller supplied. Drop them and
/// a re-prepared document loses the `--set` params, the launch-area file-ref
/// anchor, and the approved shell set — so a `$schema`-bearing proxy target
/// validates against inputs it was never handed.
///
/// This is an **input** of the canonical preparation service
/// ([`prepare_document`][super::prepare_document]), and only these four layers.
/// A source-specific value does not belong here: it belongs to the prepared
/// document, which is rebuilt per target.
#[derive(Debug, Clone, Default)]
pub struct CallerInputLayers {
    /// Frontmatter `--set` overrides (JSON object) the caller supplied.
    pub set_overrides: Option<serde_json::Value>,
    /// Launch-area directory that anchors caller-supplied file references.
    pub file_ref_fallback_dir: Option<PathBuf>,
    /// Immutable request-scoped resolution inputs captured before the first
    /// source is resolved.
    pub file_resolution_context: Option<biscuit_file::FileResolutionContext>,
    /// Shell commands approved during the original pre-flight discovery.
    /// Shell commands approved so far in this invocation.
    ///
    /// Grows as a fresh document's own pre-flight approves its own commands
    /// (see [`add_approved_commands`][Self::add_approved_commands]); an exact
    /// already-approved command never re-prompts.
    pub pre_approved_commands: Option<HashSet<String>>,
    /// Composition env overrides (e.g. `AGENT`, `MODEL`, `YOLO`) injected into
    /// the [`ComposeContext`][darkmatter::markdown::compose::ComposeContext] for
    /// the run.
    ///
    /// These back `ctx.agent`/`ctx.model`; without them a re-preparation's
    /// context resolves both to the `"unknown"`/`"default"` fallbacks even
    /// though the run has a resolved provider.
    pub env_overrides: BTreeMap<String, String>,
}

impl CallerInputLayers {
    /// The layers `options` carries, retained so a later canonical preparation
    /// reapplies exactly them.
    pub fn from_options(options: &super::PrepareOptions) -> Self {
        Self {
            set_overrides: options.set_overrides.clone(),
            file_ref_fallback_dir: options.file_ref_fallback_dir.clone(),
            file_resolution_context: options.file_resolution_context.clone(),
            pre_approved_commands: options.pre_approved_commands.clone(),
            env_overrides: options.env_overrides.clone(),
        }
    }

    /// Apply these layers onto `options` — the one assembly point every
    /// canonical preparation goes through.
    pub fn apply_to(&self, mut options: super::PrepareOptions) -> super::PrepareOptions {
        options.set_overrides = self.set_overrides.clone();
        options.file_ref_fallback_dir = self.file_ref_fallback_dir.clone();
        options.file_resolution_context = self.file_resolution_context.clone();
        options.pre_approved_commands = self.pre_approved_commands.clone();
        options.env_overrides = self.env_overrides.clone();
        options
    }

    /// Fold commands a document's own pre-flight just approved into the
    /// invocation-wide approved set.
    pub fn add_approved_commands(&mut self, commands: impl IntoIterator<Item = String>) {
        let mut commands = commands.into_iter().peekable();
        if commands.peek().is_none() {
            return;
        }
        self.pre_approved_commands
            .get_or_insert_with(HashSet::new)
            .extend(commands);
    }
}

/// A composition prepared with effective (composed) frontmatter.
///
/// This struct carries the full effective frontmatter after Darkmatter
/// composition. All downstream code (harness, MCP, provider selection)
/// must read from `effective_frontmatter`, never from raw source state.
#[derive(Debug, Clone)]
pub struct PreparedComposition {
    /// Which composition mode produced this.
    pub mode: CompositionMode,
    /// Why this document entered preparation — the row of the stage matrix
    /// this document's surrounding stages follow.
    ///
    /// Recorded by the canonical service so no downstream layer re-derives it.
    /// Preparing a document outside the service leaves this
    /// [`Direct`][super::DocumentEntryReason::Direct].
    pub entry: super::DocumentEntryReason,
    /// Whether this preparation withheld the document's schema verdict.
    ///
    /// `true` means the read ran before the document's own `initialize` (R4),
    /// so someone downstream still owes the stabilized reread that judges it.
    /// Nothing may treat such a preparation as validated.
    pub schema_verdict_deferred: bool,
    /// Resolved absolute path to the source file.
    pub resolved_path: PathBuf,
    /// Git repo root derived from the source document's location.
    ///
    /// Used for favorite-provider lookup, guardrails, MCP defaults,
    /// and harness path resolution. `None` when the source document
    /// is not inside a git repository.
    pub source_repo_root: Option<PathBuf>,
    /// The composed prompt text.
    pub prompt: String,
    /// Full frontmatter after Darkmatter composition.
    pub effective_frontmatter: serde_json::Value,
    /// Typed selection hints parsed from effective frontmatter.
    pub selection_hints: EffectiveSelectionHints,
    /// Closure plan for post-execution file updates.
    pub closure: CompositionClosurePlan,
    /// Parsed lifecycle notification config from effective frontmatter.
    pub lifecycle: LifecycleConfig,
    /// Darkmatter composition performance report, when enabled.
    pub compose_perf: Option<ComposePerfReport>,
    /// Optional schema properties whose effective value failed validation
    /// and were elided during prepare (pre-validation, drop-and-retry, or
    /// post-shell re-validation). The CLI renders one user-visible warning
    /// per entry so silently dropped values are surfaced before launch.
    pub dropped_optionals: Vec<super::error::DroppedOptional>,
    /// Non-fatal warnings emitted by Darkmatter during composition,
    /// including parser-aware `ctx.*` typo diagnostics.
    pub warnings: Vec<ComposeWarning>,
    /// Lifecycle event keys that Darkmatter intentionally deferred from
    /// compose-time resolution (DM1 metadata).
    ///
    /// Sorted, and limited to keys actually present in the source
    /// frontmatter. These keys keep their authored `{{ }}` spans in
    /// `effective_frontmatter` because they interpolate at event-time, not
    /// during the initial compose. Dry-run output consumes this so a raw
    /// span reads as intentional rather than as an unresolved-variable bug.
    pub deferred_lifecycle_keys: Vec<String>,
    /// The caller's input layers, retained so a later canonical preparation of
    /// this or a proxied document reapplies exactly them.
    pub input_layers: CallerInputLayers,
    /// The exact early-binding snapshot this document was composed against
    /// (R5).
    ///
    /// Body interpolation, effective frontmatter, lifecycle DM2 lookup,
    /// schema/file evaluation, and shell preflight all read this one snapshot.
    /// Storing it is what makes that checkable: a consumer that re-captured
    /// instead would silently answer `ctx.area` from wherever the process CWD
    /// had drifted to by the time it asked.
    pub compose_context: darkmatter::markdown::compose::ComposeContext,
}

/// How the composition result should be applied after provider execution.
#[derive(Debug, Clone)]
pub enum CompositionClosurePlan {
    /// No file mutation; provider output goes to stdout.
    Direct,
    /// Rewrite the source file with the provider's body output.
    Inline(InlineClosurePlan),
}

/// State captured before inline composition for deterministic closure.
#[derive(Debug, Clone)]
pub struct InlineClosurePlan {
    /// The original on-disk document text (frontmatter + body).
    pub original_document_text: String,
    /// Full pre-run hash of the document, always [`MdHashKind::Simple`].
    ///
    /// Computed with `hash` and `last_updated` excluded via
    /// [`inline_hash_options`][crate::composition::closure::inline_hash_options],
    /// so the managed properties cannot influence the unchanged-body check.
    pub original_hash: ComputedHash,
}

/// Universal output format for composed provider execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Text,
    Stream,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Text => write!(f, "text"),
            Self::Stream => write!(f, "stream"),
        }
    }
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "text" => Ok(Self::Text),
            "stream" | "stream-json" => Ok(Self::Stream),
            _ => Err(format!(
                "unknown output format '{value}'; expected json, text, or stream"
            )),
        }
    }
}

/// A fully-specified request to execute a composition through the
/// wrapper-grade pipeline.
#[derive(Debug, Clone)]
pub struct CompositionExecutionRequest {
    /// Which composition mode.
    pub mode: CompositionMode,
    /// The raw file reference string (for display/logging).
    pub file_ref: String,
    /// The prepared composition with effective frontmatter.
    pub prepared: PreparedComposition,
    /// Resolved provider and model, when pre-computed (e.g. by sequence
    /// review or non-TTY resolution). When `Some`, the executor skips
    /// resolution and uses this target directly.
    pub resolved_target: Option<ResolvedExecutionTarget>,
    /// Explicitly chosen provider (from `--claude`, `--codex`, etc.).
    pub explicit_provider: Option<Provider>,
    /// Providers to exclude from automatic selection.
    pub excluded: BTreeSet<Provider>,
    /// Enable provider-specific YOLO/auto-approval mode.
    pub yolo: bool,
    /// Preserve these env vars even when they match sensitive-name filters.
    pub include: Vec<String>,
    /// Override the model used by the provider.
    pub model: Option<String>,
    /// Set the output format (json, text, stream).
    pub output: Option<OutputFormat>,
    /// Parsed system prompt CLI args for the session.
    pub system_prompt_args: crate::system_prompt::SystemPromptArgs,
    /// Wall-clock timeout as a duration string (e.g. `30s`, `5m`, `2h`).
    /// Parsed by the composition executor. Only valid in non-interactive mode.
    pub timeout: Option<String>,
    /// Step-silence timeout as a duration string (e.g. `30s`, `5m`).
    ///
    /// Resets on every stream event; when silence exceeds this budget the
    /// child is killed with `TimedOut`. Ignored in capture and passthrough
    /// modes (warning emitted). `None` means no silence deadline is applied.
    pub step_timeout: Option<String>,
    /// OpenCode stalled-generation backstop budget as a duration string
    /// (e.g. `10m`). Honored only by the OpenCode bridge in structured-stream
    /// mode; provider-neutral so portable prompt files may carry it. `None`
    /// falls through to the built-in `10m` default during resolution; a `0s`
    /// literal disables the backstop.
    pub stall_timeout: Option<String>,
    /// OPERATION env var value for the composed session.
    pub operation: Option<String>,
    /// Enable provider-specific sandboxing.
    pub sandbox: bool,
    /// Use only repo-scoped resources via a shadow HOME.
    pub repo: bool,
    /// Show what would be executed without launching the child.
    pub dry_run: bool,
    /// Enable Claudine-managed MCP session composition.
    pub mcp: bool,
    /// Explicit MCP server IDs or aliases to activate.
    pub mcp_use: Vec<String>,
    /// Treat unresolved or ambiguous MCP tags as hard errors.
    pub strict: bool,
    /// Whether the provider session should be interactive (`-i`).
    pub session_interactive: bool,
    /// Why the session is interactive or non-interactive.
    pub session_interactive_source: SessionInteractivitySource,
    /// Show only header; suppress env details and info.
    pub quiet: bool,
    /// Suppress all preflight output.
    pub silent: bool,
    /// Extra environment variables to inject into both the composition
    /// context (used for preflight and prompt interpolation) and the
    /// spawned child process. Currently used by sequence execution to
    /// propagate `FAIL_FAST`.
    pub env_overrides: BTreeMap<String, String>,
    /// Optional shared shell-approval cache. When provided, harness
    /// shell preflight reuses previously approved commands from this
    /// cache instead of prompting again. Used by sequence execution to
    /// honour "allow once" for the whole run, not just one step.
    pub shared_approval_cache: Option<SharedApprovalCache>,
    /// Whether this request is part of a sequence run. When `true`, the
    /// execution header shows a `Sequence` badge.
    pub sequence: bool,
    /// Pre-computed installed-provider snapshot, supplied by callers that
    /// already ran host detection during prep (e.g. `CompositionPrepContext`).
    /// When `Some`, the executor skips the `InstalledAiClients::new()` scan
    /// and uses this snapshot for both provider selection and binary path
    /// resolution.
    pub installed_snapshot: Option<InstalledProviderSnapshot>,
    /// Pre-computed launch-CWD `LaunchWorkspaceContext`, supplied by callers
    /// that already ran a shared `sniff::detect_with_plan` scan during prep
    /// (e.g. `CompositionPrepContext`). When `Some`, the executor reuses it
    /// instead of calling `resolve_launch_workspace_context` again for both
    /// the header env plan and the child env build.
    pub prep_launch_workspace: Option<LaunchWorkspaceContext>,
    /// Pre-computed launch-CWD `LaunchContext`, supplied by callers that
    /// already ran a shared `sniff::detect_with_plan` scan during prep
    /// (e.g. `CompositionPrepContext`). When `Some`, the executor reuses
    /// it instead of calling `LaunchContext::from_cwd` again.
    pub prep_launch_context: Option<crate::system_prompt::LaunchContext>,
    /// Pre-computed launch-CWD `EnvironmentContext` derived from the
    /// same shared sniff scan. When `Some` and the effective env-detect
    /// root matches the launch CWD, the executor reuses it instead of
    /// calling `detect_environment_fast` again.
    pub prep_env_context: Option<crate::events::EnvironmentContext>,
    /// Diagnostic snapshot of the shared prep-time sniff scan failure, if
    /// it failed. The shared scan defaults to an empty
    /// [`crate::system_prompt::LaunchContext`] on failure to keep
    /// best-effort callers happy, so the executor reads this field to
    /// preserve the legacy hard-fail contract for `--repo`. `None` (or
    /// no prep context at all) means no failure happened during prep.
    ///
    /// A [`DiagnosticSnapshot`](crate::diagnostics::DiagnosticSnapshot)
    /// rather than a `String` so the typed `sniff::SniffError` retained
    /// upstream survives into this `Clone` record.
    pub prep_launch_detection_error: Option<crate::diagnostics::DiagnosticSnapshot>,
    /// Whether the caller already emitted the execution header up front.
    ///
    /// `compose` / `inline-compose` resolve the agent eagerly (prompting
    /// when ambiguous) and render the execution line *before* the
    /// expensive prepare/compose work, so the user sees it immediately.
    /// When `true`, the executor must not re-emit the header. When
    /// `false` (the dry-run-unresolved corner and sequence steps) the
    /// executor emits it after resolving the target itself.
    pub header_emitted: bool,

    /// Provider-argument tail forwarded verbatim to the underlying agent,
    /// captured by the CLI's pre-clap ownership partition. Seeds the child
    /// argv at the same stage as direct-wrapper passthrough, ahead of
    /// Claudine's entrypoint / model / transport / prompt-delivery
    /// injections. Distinct from MCP arguments. Empty when no tail was given.
    pub provider_args: Vec<String>,

    /// `true` when [`Self::provider_args`] came from an explicit `--` boundary
    /// (opaque, unclassified) rather than an implicit non-Claudine switch.
    /// Drives the INFO status wording only; never affects forwarding.
    pub provider_args_explicit: bool,

    /// The committed proxy handoff's evaluated `with:` overlay for this
    /// document, when it was reached through a proxy. Re-applied over the
    /// target's authored frontmatter on every re-materialization
    /// (retry/resume), so the immutable pre-schema handoff input survives
    /// refresh exactly as canonical preparation left it. Empty for a
    /// directly-invoked document.
    pub proxy_overlay: indexmap::IndexMap<String, serde_json::Value>,

    /// The invocation-wide run ledger, shared with the provider harness so a
    /// terminal-event proxy can be committed against the one chain while the
    /// source document's stacks are still live to catch a refused hop.
    /// `None` for callers with no active-document coordinator (the direct
    /// wrapper passthrough, whose empty lifecycle can never select a proxy).
    pub handoff_ledger: Option<crate::composition::SharedRunLedger>,

    /// The already-committed proxy handoff this request re-prepares, when the
    /// document was reached through a proxy. `Some` marks an *adopted target*:
    /// the command coordinator has already committed the hop against the shared
    /// [`handoff_ledger`][Self::handoff_ledger], so the executor must **not**
    /// route the target's `initialize` a second time through the setup pipeline.
    /// Instead the harness loop's staged bootstrap (narrow initialize-shell gate
    /// → the target's own `initialize` → stabilized reread → full audit) owns the
    /// target's `initialize`, exactly as an in-harness adoption does — the one
    /// canonical R4 staging for every route. `None` for a directly-invoked
    /// document and for the caller's first document.
    pub adopted_handoff: Option<Box<crate::composition::ProxyHandoff>>,

    /// The invocation-local runtime state cell shared with the caller.
    ///
    /// Supplied by callers that execute more than one composition in one
    /// logical run — the `--loop` engine across iterations, and sequence
    /// execution across steps — so accumulated `set` mutations and the
    /// `outputs` array survive from one execution to the next. `None` makes the
    /// executor mint a fresh cell, which is the correct lifetime for a
    /// standalone `compose` / `inline-compose`.
    pub runtime_state: Option<std::sync::Arc<super::runtime_state::RuntimeState>>,

    /// Suppress this execution's own commit to `outputs`.
    ///
    /// Set by a caller that owns output timing itself — the sequence task
    /// executor, which must not publish an entry until `teardown` has completed
    /// (a failing teardown converts the task to failure and owes no output).
    /// The captured text still reaches the caller through
    /// `SingleCompositionOutcome::final_output`.
    pub suppress_output_commit: bool,
    /// Frames this run's live provider stdout under one task's bar.
    ///
    /// Set by the sequence task executor so a group member's assistant text is
    /// attributed line by line, the same way its shell output is. `None` — every
    /// non-sequence run — leaves the stream undecorated.
    ///
    /// Owned rather than borrowed because the stdout reader runs on its own
    /// thread; owned rather than a handle on the process-wide coordinator
    /// because attribution is per task and several sessions share that
    /// coordinator at once.
    pub task_frame_writer: Option<crate::render::TaskFrameWriter>,
}

/// Options for sequence execution at the CLI level.
#[derive(Debug, Clone)]
pub struct SequenceExecutionOptions {
    /// CLI `--fail-fast` override. `None` means use the document default.
    pub fail_fast_override: Option<bool>,
}

/// Summary of a completed sequence run.
#[derive(Debug, Clone)]
pub struct SequenceRunSummary {
    /// Total steps in the sequence.
    pub total_steps: usize,
    /// Number of steps that succeeded.
    pub succeeded: usize,
    /// Number of steps that failed.
    pub failed: usize,
    /// Per-step results.
    pub steps: Vec<SequenceStepResult>,
}

/// Result of a single sequence step.
#[derive(Debug, Clone)]
pub struct SequenceStepResult {
    /// 1-based step number.
    pub step: usize,
    /// Display name of the step.
    pub name: String,
    /// Whether the step succeeded (exit code 0).
    pub success: bool,
    /// Error message if the step failed.
    ///
    /// Load-bearing beyond display: `run_sequence_steps` compares it against
    /// the `interrupted by SIGINT` sentinel to select exit code 130, so its
    /// text is a contract, not prose.
    pub error: Option<String>,
    /// The failure's diagnostic identity, when the step's error chain carried
    /// one.
    ///
    /// Beside [`error`](Self::error) rather than replacing it: the prose is
    /// matched for the interrupt sentinel and rendered verbatim, while this
    /// carries the `code`, `category`, `detail`, and one-level cause the
    /// string discards (spec §D9). `None` for an interrupt, a success, or a
    /// chain that held only prose.
    /// Boxed to match the `StepOutcome` variant it is moved from.
    pub error_snapshot: Option<Box<DiagnosticSnapshot>>,
    /// Wall-clock duration of the step.
    pub duration: std::time::Duration,
    /// When the step ran a group, one entry per member task that ran, in
    /// declaration order. Empty for every other step.
    pub tasks: Vec<SequenceTaskResult>,
}

/// One group member task's contribution to a step summary.
///
/// Deliberately name + outcome + timing only: the removed group `output` and
/// task `passthrough` fields are not reintroduced here — `outputs` is the only
/// output accumulator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceTaskResult {
    /// The task's authored `name:`, or its generated label.
    pub name: String,
    /// Whether the task succeeded.
    pub success: bool,
    /// Whether the task was interrupted rather than failed.
    pub interrupted: bool,
    /// Wall-clock duration of the task.
    pub duration: std::time::Duration,
}
