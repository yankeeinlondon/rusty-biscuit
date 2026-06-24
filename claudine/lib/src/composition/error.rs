//! Composition-specific error types.

use std::path::PathBuf;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use chrono::{DateTime, Utc};
use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::compose::shell_expansion::ShellExpansionError;

use super::frontmatter_excerpt::FrontmatterExcerpt;
use super::types::{ResolutionMode, ResolvedCompositionSource, SessionInteractivitySource};
use crate::provider::Provider;
use thiserror::Error;

/// Exit code emitted when a loop run halts because a provider rate-limit
/// signal was observed and the configured policy was `abort` (or pause was
/// unsafe due to a missing `reset_at`).
///
/// Mirrors `EX_TEMPFAIL` (`75`) from BSD `sysexits.h` so shell wrappers can
/// distinguish a transient rate-limit halt from a generic non-zero exit
/// (typically `1`).
pub const LOOP_RATE_LIMITED_EXIT_CODE: i32 = 75;

/// Errors that can occur during composition workflows.
#[derive(Error, Debug)]
pub enum CompositionError {
    /// The file reference string could not be parsed.
    #[error("invalid file reference: {0}")]
    InvalidReference(String),

    /// The resolved file does not exist.
    #[error("file not found: {0}")]
    FileNotFound(String),

    /// The file is not a Markdown document.
    #[error("not a Markdown file (expected .md or .markdown): {0}")]
    NotMarkdown(String),

    /// The Markdown file could not be loaded or parsed.
    #[error("failed to load Markdown: {0}")]
    MarkdownLoad(String),

    /// The document's frontmatter (the YAML between the leading `---` markers)
    /// is present but failed to parse.
    ///
    /// Distinct from [`Self::PromptPropertyMissing`]: the frontmatter block
    /// exists but is malformed, so *no* properties — including `prompt` — could
    /// be read. Surfacing this separately stops a YAML syntax error (e.g.
    /// inconsistent block-scalar indentation) from masquerading as a missing
    /// property.
    ///
    /// Carries the typed `MarkdownError` so the CLI's top-level walker renders
    /// Darkmatter's rich frontmatter-parse block (file link, YAML location,
    /// offending-line excerpt) instead of a flat string.
    #[error("failed to parse frontmatter: {0}")]
    FrontmatterParse(#[source] MarkdownError),

    /// Inline composition requires a `prompt` frontmatter property.
    #[error("frontmatter is missing a `prompt` property")]
    PromptPropertyMissing,

    /// The `prompt` frontmatter property is not a string.
    #[error("frontmatter `prompt` must be a string, got {0}")]
    PromptPropertyWrongType(String),

    /// `inline-compose` was run on a document that authors both a non-null
    /// `prompt` and a non-null `sequence` — i.e. an inline sequence. Such a
    /// document must be run with `claudine sequence` so each sequence state
    /// invokes an inline-compose using `prompt`.
    ///
    /// Detected against the *authored* frontmatter (not command-line overrides),
    /// before prompt-type validation, schema processing, composition, provider
    /// selection, or execution. The authored frontmatter is shown as a YAML
    /// block by enriching this error via
    /// [`enrich_frontmatter`](CompositionError::enrich_frontmatter); the block
    /// is TTY-gated by the [`FrontmatterExcerpt`] like every other
    /// frontmatter-rooted error.
    #[error("`inline-compose` cannot run a document configured as a sequence")]
    InlineComposeSequenceMismatch {
        /// The resolved absolute path to the source document.
        source_path: PathBuf,
    },

    /// Darkmatter composition failed for a reason other than a known
    /// structured shell-expansion failure.
    ///
    /// Carries the typed `MarkdownError` so the CLI's top-level walker can
    /// render a rich `BlockError` report (transclusion cycles, reference
    /// errors, etc.) instead of a flat string.
    #[error("compose failed: {0}")]
    ComposeFailed(#[source] MarkdownError),

    /// A shell expansion directive inside the composed document failed in a
    /// structurally-known way. Carries the underlying `ShellExpansionError`
    /// so the CLI layer can render a rich, source-aware error report.
    ///
    /// The `Display` impl of this variant intentionally matches the legacy
    /// `ComposeFailed` wrapper so callers that only use `to_string()` stay
    /// compatible, while callers that want structured data can `match` on
    /// the variant itself.
    #[error("compose failed: Shell expansion failed: {error}")]
    ShellExpansionFailed {
        /// The file being composed when the shell expansion error fired.
        source_path: PathBuf,
        /// The underlying structured shell expansion error.
        #[source]
        error: Box<ShellExpansionError>,
    },

    /// No installed providers can run composition.
    #[error("no runnable providers available (all excluded or uninstalled)")]
    NoRunnableProviders,

    /// The `agent` frontmatter hint does not match any known provider.
    #[error("agent hint `{0}` does not match any known provider")]
    AgentHintInvalid(String),

    /// Agent resolution could not select a runnable provider in the current
    /// session (non-TTY, or the user cancelled the interactive picker).
    #[error("agent resolution failed for {source_path}: {state:?}")]
    AgentResolutionFailed {
        /// The prompt file whose `agent` hint could not be resolved.
        source_path: PathBuf,
        /// Classified state explaining why resolution failed.
        state: super::types::AgentResolutionState,
        /// Installed providers at the time of resolution (for diagnostic lists).
        installed: Vec<Provider>,
    },

    /// The `agent` frontmatter property is not a valid type.
    #[error("frontmatter `agent` must be a string or array of strings, got {0}")]
    AgentHintWrongType(String),

    /// The `model` frontmatter property is not a valid type.
    #[error("frontmatter `model` must be a string or array of strings, got {0}")]
    ModelHintWrongType(String),

    /// The `interactive` frontmatter property is not a boolean.
    #[error("frontmatter `interactive` must be a boolean (true/false), got {0}")]
    InteractiveHintWrongType(String),

    /// The `agent` frontmatter hint matches multiple providers.
    #[error("agent hint `{hint}` is ambiguous; matches: {matches}")]
    AgentHintAmbiguous {
        hint: String,
        matches: String,
        /// The matched providers for interactive disambiguation.
        providers: Vec<Provider>,
    },

    /// Provider selection could not resolve in the current mode.
    #[error(
        "could not resolve provider in {mode} mode; installed: {installed:?}, \
         favorite_agent: {favorite_agent:?}, frontmatter_agent_present: {frontmatter_agent_present}"
    )]
    SelectionUnavailable {
        mode: ResolutionMode,
        installed: Vec<Provider>,
        favorite_agent: Option<Provider>,
        frontmatter_agent_present: bool,
    },

    /// A model is required but none was resolved.
    #[error("model selection failed for {provider}: {reason}")]
    ModelSelectionFailed { provider: Provider, reason: String },

    /// Interactive selection is required but no TTY is available.
    #[error(
        "interactive provider selection required but no TTY available; use an explicit provider flag (--claude, --codex, etc.) or add an `agent` frontmatter property"
    )]
    InteractiveSelectionRequired,

    /// Inline composition in interactive mode is not supported for this provider
    /// because it cannot capture the final assistant message.
    #[error(
        "inline-compose in interactive mode (from {source_kind}) is not supported for {provider}; \
         the provider cannot capture the final assistant message"
    )]
    InlineInteractiveUnsupported {
        /// Provider that does not support interactive inline closure.
        provider: String,
        /// Why the session resolved to interactive mode.
        source_kind: SessionInteractivitySource,
    },

    /// The provider returned an invalid response for inline composition.
    #[error("invalid inline composition response: {0}")]
    InvalidInlineResponse(String),

    /// The document's existing `hash` frontmatter property is malformed.
    ///
    /// Carries the typed `MarkdownError` so the CLI's top-level walker renders
    /// Darkmatter's `MalformedStoredHash` block instead of a flat string.
    #[error("malformed stored `hash` property: {0}")]
    InlineHashMalformed(#[source] MarkdownError),

    /// Atomic file write failed during inline composition.
    #[error("atomic write failed: {0}")]
    AtomicWriteFailed(String),

    /// The composition target file lacks required read/write permissions.
    #[error("insufficient file permissions (need read+write): {0}")]
    InsufficientFilePermissions(String),

    /// Pre-flight shell command discovery failed.
    ///
    /// Carries the typed `MarkdownError` so the CLI's top-level walker can
    /// render a rich `BlockError` report (e.g. transclusion cycles or
    /// reference errors encountered while walking the document graph) instead
    /// of a flat string.
    #[error("pre-flight discovery failed: {0}")]
    PreFlightDiscoveryFailed(#[source] MarkdownError),

    /// A general pre-flight failure (blacklisted command, missing handler, etc.).
    #[error("pre-flight shell approval failed: {0}")]
    PreFlightFailed(String),

    /// The user denied a shell command during pre-flight approval.
    #[error(
        "Aborted: shell command '{command}' was denied during pre-flight approval \
         (source: {source_file}, line {line}). No provider session was started."
    )]
    ShellCommandDenied {
        command: String,
        source_file: PathBuf,
        line: usize,
    },

    /// A lifecycle notification property failed to deserialize.
    #[error("invalid lifecycle property `{property}`: {message}")]
    LifecycleInvalid {
        /// The frontmatter property that failed to parse (e.g. `"success"`).
        property: String,
        /// The underlying deserialization error message.
        message: String,
        /// The source file whose frontmatter contained the invalid property.
        source_file: PathBuf,
        /// The unknown field name extracted from the serde error (e.g. `"speak"`).
        unknown_field: Option<String>,
        /// The list of valid field names for the lifecycle notification.
        expected_fields: Vec<String>,
    },

    /// A lifecycle notification property has both `say` and `say_first`.
    #[error("lifecycle property `{0}` has both `say` and `say_first`; only one is allowed")]
    LifecycleSayConflict(String),

    /// A lifecycle notification property references an unknown sound effect.
    #[error("lifecycle property `{0}` references unknown sound effect `{1}`")]
    LifecycleUnknownEffect(String, String),

    /// A rendered lifecycle string still contains a recognized `{{ … }}`
    /// interpolation span after composition.
    ///
    /// This guards against Darkmatter's default lenient behavior
    /// (`fail_fast = false`), which leaves malformed or unresolvable
    /// expressions in place instead of failing composition. Catching the
    /// leak here prevents raw template syntax from reaching user-visible
    /// side effects (Discord, Slack, TTS, stderr, desktop notifications).
    #[error(
        "lifecycle interpolation leaked in `{property}`: {expression} ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleInterpolationLeak {
        /// The composed prompt file whose lifecycle frontmatter leaked.
        source_path: PathBuf,
        /// Dotted lifecycle key path, e.g. `"start.message"`.
        property: String,
        /// Raw offending span text, e.g. `"{{ parent_dir(review)) }}"`.
        expression: String,
        /// Parse/eval failure reason from the compose report's warnings, when
        /// available. Empty when the span is unrecognized entirely.
        reason: String,
    },

    /// A lifecycle string references a bare `{{ variable }}` that is undefined
    /// after composition.
    ///
    /// Darkmatter resolves an unknown bare variable to an empty string with no
    /// warning and no error (even in fail-fast mode), so a message like
    /// `"before {{ missing }} after"` would otherwise dispatch silently as
    /// `"before  after"`. This guard inspects the **raw** (pre-composition)
    /// lifecycle strings — where the span is still visible — and aborts
    /// preparation before any side effect (Discord, Slack, TTS, stderr,
    /// desktop notification) can dispatch the degraded message.
    #[error(
        "lifecycle property `{property}` references undefined variable `{variable}` ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleUndefinedVariable {
        /// The prompt file whose lifecycle frontmatter referenced the variable.
        source_path: PathBuf,
        /// Dotted lifecycle key path, e.g. `"start.message"`.
        property: String,
        /// The undefined bare variable name, e.g. `"missing_lifecycle_var"`.
        variable: String,
    },

    /// A removed harness validation or handler DSL key was found in the
    /// frontmatter.
    ///
    /// The `pre_checks`, `post_checks`, `handle`, `deviate`, and
    /// `handle_<subject>` DSL keys were retired when the lifecycle stack model
    /// replaced the harness validation and handler execution layers. This
    /// variant surfaces a typed, actionable diagnostic that names the offending
    /// key and points to the lifecycle surface that replaces it.
    #[error(
        "removed validation/handler key `{key}` in {source_path}: {replacement}",
        source_path = source_path.display()
    )]
    RemovedValidationKey {
        /// The prompt file whose frontmatter contained the removed key.
        source_path: PathBuf,
        /// The removed key exactly as authored (e.g. `"pre_checks"`).
        key: String,
        /// Human-readable replacement guidance.
        replacement: String,
    },

    /// A lifecycle stack item has an invalid shape (not an object, missing
    /// `action` key, unknown key, etc.).
    ///
    /// Raised at parse time while walking a `stack:` block. The `property`
    /// names the owning event (e.g. `"start"`) so the diagnostic can point at
    /// the right frontmatter block.
    #[error(
        "invalid lifecycle stack item in `{property}` ({source_path}): {message}",
        source_path = source_path.display()
    )]
    LifecycleStackInvalidShape {
        /// The prompt file whose lifecycle frontmatter held the malformed stack.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`, `"success.stack[2]"`).
        property: String,
        /// Human-readable description of the shape problem.
        message: String,
    },

    /// A short-form lifecycle action `verb(args)` failed to parse.
    ///
    /// Covers: missing verb, unbalanced parens, trailing characters, argument
    /// splitter errors, and argument expression parse errors. The bare
    /// "unquoted multi-word literal" case (e.g. `say(using codex)`) is also
    /// reported through this variant with a `message` that names the cause.
    #[error(
        "invalid lifecycle action `{raw}` in `{property}` ({source_path}): {message}",
        source_path = source_path.display()
    )]
    LifecycleActionInvalidShortForm {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// The raw short-form string exactly as authored.
        raw: String,
        /// Human-readable parse failure description.
        message: String,
    },

    /// A long-form lifecycle action supplied an unknown field, wrong value
    /// type, or missing required parameter.
    #[error(
        "invalid lifecycle action `{action}` in `{property}` ({source_path}): {message}",
        source_path = source_path.display()
    )]
    LifecycleActionInvalidLongForm {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// The action verb (e.g. `"set_frontmatter"`, `"shell"`).
        action: String,
        /// Human-readable description of the long-form problem.
        message: String,
    },

    /// A lifecycle control action was used in an event where the spec's
    /// "Where valid" matrix forbids it (e.g. `Skip` outside `initialize`).
    ///
    /// Raised at parse time after the action identity is known, so the
    /// diagnostic can name both the action and the event.
    #[error(
        "lifecycle action `{action}` is not valid in the `{event}` event ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleActionPlacement {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// The rejected action verb (e.g. `"skip"`, `"retry"`).
        action: String,
        /// The event name the action was placed in (e.g. `"start"`).
        event: String,
    },

    /// More than one lifecycle control action appeared in a single stack item.
    ///
    /// The spec's cardinality rule allows at most one lifecycle control action
    /// per `when/action` block (and it must be the last action). This variant
    /// covers the "more than one" half.
    #[error(
        "lifecycle stack item in `{property}` has more than one lifecycle action; \
         at most one is allowed per item ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleMultipleLifecycleActions {
        /// The prompt file whose lifecycle frontmatter held the stack item.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
    },

    /// A lifecycle control action was not the last action in its stack item.
    ///
    /// The spec's cardinality rule allows at most one lifecycle control action
    /// per `when/action` block, and it must be the last action (subsequent
    /// actions would be unreachable).
    #[error(
        "lifecycle action in `{property}` must be the last action in its stack item ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleActionOrder {
        /// The prompt file whose lifecycle frontmatter held the stack item.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
    },

    /// A lifecycle action received the wrong number or type of arguments.
    ///
    /// Distinct from [`Self::LifecycleActionInvalidShortForm`] (which covers
    /// syntax-level failures): this variant covers semantic-argument errors
    /// such as `retry(-1)` (negative max attempts) or `proxy()` (missing the
    /// required target).
    #[error(
        "lifecycle action `{action}` in `{property}` has invalid arguments ({source_path}): {message}",
        source_path = source_path.display()
    )]
    LifecycleInvalidArgs {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// The action verb (e.g. `"retry"`, `"proxy"`).
        action: String,
        /// Human-readable description of the argument problem.
        message: String,
    },

    /// A reference to the lifecycle-stack-only `err` global appeared in a
    /// no-error event (`initialize`, `start`, `success`, `loop`).
    ///
    /// The `err` global is only meaningful when the iteration may have errored
    /// (`blocked`, `failure`, and the optional-error `finalize`). Reading it
    /// elsewhere is faulty logic and is rejected at parse time by walking the
    /// expression surfaces (`when:` clauses, message strings, short-form
    /// action arguments) with the existing Darkmatter AST machinery. The
    /// `doc.err` escape hatch is exempt — only a bare `err` (or `err.*`
    /// member access) triggers this variant.
    #[error(
        "lifecycle property `{property}` references `err` in the `{event}` event, \
         which never carries an error ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleErrNotAvailable {
        /// The prompt file whose lifecycle frontmatter held the reference.
        source_path: PathBuf,
        /// Dotted lifecycle key path, e.g. `"start.stack[1].action"`.
        property: String,
        /// The event name the reference appeared in (e.g. `"start"`).
        event: String,
    },

    /// A `failure` stack requested `resume(...)` but the agentic loop did
    /// not surface a provider session id, so there is nothing to resume.
    ///
    /// Raised at runtime (not parse time): whether a session id exists is
    /// only known after the provider runs. Surfacing it as a typed error
    /// keeps the control from silently no-opping.
    #[error(
        "lifecycle `resume` requires a live provider session to resume, but none \
         was captured (the provider had not launched) ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleResumeWithoutSession {
        /// The prompt file whose stack requested resume.
        source_path: PathBuf,
    },

    /// A flow-control action is valid in the event (placement is universal),
    /// but its runtime effect — run-loop re-entry (`retry`), hand-off
    /// (`proxy`), or the deferred-queue (`requeue`) — is not wired for events
    /// outside the provider run loop: `initialize`, a compose pre-flight
    /// `blocked`, and the `loop` gate. The control is surfaced as a clear error
    /// rather than a silent no-op. (`resume` reports `ResumeWithoutSession`
    /// separately when no session exists.)
    #[error(
        "lifecycle `{action}` at `{event}` is accepted, but its runtime effect is \
         not wired for this event; move the recovery to a post-launch event \
         (`failure`/`finalize`/`success`) ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleSetupPhaseRecoveryUnsupported {
        /// The prompt file whose stack requested the action.
        source_path: PathBuf,
        /// The originating event (`initialize` / `blocked` / `loop`).
        event: String,
        /// The deferred action verb (`retry` / `proxy`).
        action: String,
    },

    /// A lifecycle stack requested `defer(...)` (deferred re-execution). The
    /// action parses and is valid in every event, but its runtime home — the
    /// rendezvous deferred-execution scheduler — is not ready to receive
    /// prompts yet, so the control is surfaced as a clear "not implemented"
    /// error rather than silently dropped or half-enqueued.
    #[error(
        "lifecycle `defer` is not implemented yet: deferred re-execution (running \
         this prompt again later, via the rendezvous scheduler) is not ready to \
         receive prompts — use `retry`/`resume` for in-run recovery for now \
         ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleDeferNotImplemented {
        /// The prompt file whose stack requested `defer`.
        source_path: PathBuf,
    },

    /// A lifecycle stack requested `requeue(...)`, but the runtime could not
    /// record the prompt in the deferred-execution queue.
    #[error(
        "lifecycle `requeue` (delay `{delay}`{reason}) could not enqueue the \
         prompt via rendezvous: {message} ({source_path})",
        reason = match reason {
            Some(r) => format!(", reason `{r}`"),
            None => String::new(),
        },
        source_path = source_path.display()
    )]
    LifecycleRequeueEnqueueFailed {
        /// The prompt file whose stack requested requeue.
        source_path: PathBuf,
        /// The requested delay duration string.
        delay: String,
        /// The optional authored reason.
        reason: Option<String>,
        /// Human-readable enqueue failure.
        message: String,
    },

    /// A lifecycle `proxy(...)` hand-off would re-enter a document already on
    /// the active proxy chain (a self-proxy or an A→B→A cycle), or exceeded the
    /// maximum proxy hop count.
    ///
    /// Surfaced as a typed error rather than looping forever: a `failure`
    /// stack that proxies back to a document whose own `failure` stack proxies
    /// again has no terminal state, so the runtime stops the chain loudly.
    #[error(
        "lifecycle `proxy` hand-off to `{target}` forms a cycle or exceeds the \
         proxy hop limit ({limit}); active chain: {chain} ({source_path})",
        chain = chain.join(" -> "),
        source_path = source_path.display()
    )]
    LifecycleProxyCycle {
        /// The prompt file whose stack requested the cyclic proxy.
        source_path: PathBuf,
        /// The proxy target that would close the cycle or overflow the limit.
        target: String,
        /// The active proxy chain (resolved paths, in hand-off order).
        chain: Vec<String>,
        /// The maximum number of proxy hops permitted.
        limit: usize,
    },

    /// A lifecycle `initialize` stack raised an explicit `error(...)` (or an
    /// unintentional action error routed to `failure`), so the run aborts
    /// before any iteration runs.
    ///
    /// Mirrors the non-loop path's `eyre!("...")` failure at initialize: the
    /// `failure` and `finalize` events fire through the lifecycle guard before
    /// this error is returned, so callers should not double-emit them.
    #[error(
        "lifecycle `initialize` raised an error: {reason} ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleInitializeFailed {
        /// The prompt file whose `initialize` stack raised the error.
        source_path: PathBuf,
        /// Evaluated reason (explicit `error(...)` argument or captured
        /// action-error message).
        reason: String,
    },

    /// A `loop:` gate stack raised an explicit `error(...)`, converting the
    /// loop's final outcome to failure and exiting the loop.
    ///
    /// Only the **explicit** `Error` lifecycle action lands here. The `loop`
    /// gate is a terminal-phase event, so an *unintentional* action error there
    /// leaves the composition outcome unchanged (per the action error-
    /// propagation table) and never produces this variant.
    #[error(
        "lifecycle `loop` gate raised an error: {reason} ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleLoopGateFailed {
        /// The prompt file whose `loop:` gate stack raised the error.
        source_path: PathBuf,
        /// Evaluated reason (explicit `error(...)` argument).
        reason: String,
    },

    // -- Sequence errors -------------------------------------------------------
    /// The `sequence` frontmatter value is not a valid type (must be a list or a string).
    #[error("invalid sequence definition: {0}")]
    SequenceInvalid(String),

    /// The sequence list is empty.
    #[error("sequence list is empty; at least one step is required")]
    SequenceEmpty,

    /// The external YAML file could not be loaded.
    #[error("failed to load external sequence file: {0}")]
    SequenceExternalLoad(String),

    /// The external YAML file has an unexpected root shape.
    #[error("external sequence file has wrong structure: {0}")]
    SequenceExternalWrongType(String),

    /// An object step is missing the required `name` property.
    #[error("sequence step at index {index} is missing required `name` property")]
    SequenceStepNameMissing { index: usize },

    /// The `name` property of an object step is not a string.
    #[error("sequence step at index {index} has `name` of type {found}, expected string")]
    SequenceStepNameWrongType { index: usize, found: String },

    /// A template value is not a string.
    #[error("sequence template key `{key}` has type {found}, expected string")]
    SequenceTemplateWrongType { key: String, found: String },

    /// Templates require all list items to be objects.
    #[error("sequence templates require all list items to be objects (dictionaries)")]
    SequenceTemplateRequiresObjectItems,

    /// A template key collides with a reserved sequence overlay key.
    #[error("sequence template key `{0}` collides with reserved sequence key")]
    SequenceReservedTemplateKey(String),

    /// One or more sequence steps failed provider/model resolution in non-TTY mode.
    #[error("sequence selection failed for {failure_count} step(s): {failures:?}")]
    SequenceSelectionFailed {
        failures: Vec<SequenceSelectionFailure>,
        failure_count: usize,
    },

    /// One or more sequence steps have missing required schema properties
    /// and Interactive Mode is not allowed.
    ///
    /// Aggregated so the user can fix the full sequence in a single edit
    /// pass instead of discovering each step's gaps one at a time.
    #[error(
        "sequence schema validation failed for {failure_count} step(s); missing required properties"
    )]
    SequenceMissingProperties {
        /// Per-step missing-property records in execution order.
        failures: Vec<SequenceMissingPropertiesStep>,
        /// Number of steps that reported missing properties.
        failure_count: usize,
    },

    /// A `sequence` document authored `interactive: true` in its frontmatter.
    ///
    /// Sequences are serial automation; interactive mode must be requested
    /// per-invocation with the `--interactive` flag instead.
    #[error(
        "`interactive: true` is not allowed in a sequence document ({0}); \
         use `compose` or `inline-compose` for dialog-shaped prompts, \
         or pass `--interactive` to override a single sequence run"
    )]
    SequenceInteractiveRejected(PathBuf),

    // -- Loop errors -----------------------------------------------------------
    /// The `loop` frontmatter value is invalid.
    #[error("invalid loop definition: {0}")]
    LoopInvalid(String),

    /// Loop execution exceeded its configured safety cap.
    #[error(
        "loop limit exceeded for {prompt_path} at iteration {iteration}; cap is {cap}",
        prompt_path = prompt_path.display()
    )]
    LoopLimitExceeded {
        /// Maximum allowed iteration count.
        cap: usize,
        /// Prompt file being executed.
        prompt_path: PathBuf,
        /// 1-based iteration that exceeded the limit.
        iteration: usize,
    },

    /// A loop action failed validation or execution.
    #[error(
        "invalid loop action at iteration {iteration}, action {action_index} of {total_actions}: {message}"
    )]
    InvalidAction {
        /// 1-based iteration index.
        iteration: usize,
        /// 1-based action index.
        action_index: usize,
        /// Total actions in this iteration.
        total_actions: usize,
        /// Human-readable failure reason.
        message: String,
    },

    /// Increment targeted a property with an unsupported type.
    #[error(
        "invalid increment at iteration {iteration}, action {action_index} of {total_actions}: property `{property}` has type {found} (value: {value_excerpt})"
    )]
    InvalidIncrementType {
        /// 1-based iteration index.
        iteration: usize,
        /// 1-based action index.
        action_index: usize,
        /// Total actions in this iteration.
        total_actions: usize,
        /// Property that was targeted.
        property: String,
        /// Actual property type.
        found: String,
        /// Excerpt of the offending value, including a stage note when it is an
        /// unresolved template.
        value_excerpt: String,
    },

    /// Decrement targeted a property with an unsupported type.
    #[error(
        "invalid decrement at iteration {iteration}, action {action_index} of {total_actions}: property `{property}` has type {found} (value: {value_excerpt})"
    )]
    InvalidDecrementType {
        /// 1-based iteration index.
        iteration: usize,
        /// 1-based action index.
        action_index: usize,
        /// Total actions in this iteration.
        total_actions: usize,
        /// Property that was targeted.
        property: String,
        /// Actual property type.
        found: String,
        /// Excerpt of the offending value, including a stage note when it is an
        /// unresolved template.
        value_excerpt: String,
    },

    /// Loop execution was interrupted by the user (SIGINT / Ctrl+C).
    #[error(
        "user interrupted looping operation in {prompt_path}",
        prompt_path = prompt_path.display()
    )]
    LoopInterrupted {
        /// Prompt file being executed when the interrupt was observed.
        prompt_path: PathBuf,
    },

    /// A loop iteration's provider exited non-zero.
    ///
    /// This is a *runtime* failure distinct from [`Self::LoopInvalid`] (a
    /// frontmatter parse problem) and from [`Self::LoopInterrupted`]
    /// (Ctrl+C). The `reason` field carries a human-readable cause derived
    /// from the iteration's `session_end` JSONL row (e.g. `step_timeout`,
    /// `wall-clock timeout`, `signal SIGTERM`, `provider exited non-zero`).
    /// The `exit_reason` field carries the structured `error_kind` string
    /// for downstream tooling.
    #[error(
        "loop iteration {iteration} of {prompt_path}: {reason} (exit code {exit_code})",
        prompt_path = prompt_path.display()
    )]
    LoopIterationFailed {
        /// 1-based iteration that failed.
        iteration: usize,
        /// Prompt file being executed.
        prompt_path: PathBuf,
        /// Process-style exit code for the failed iteration.
        exit_code: i32,
        /// Human-readable cause. Never empty.
        reason: String,
        /// Structured `error_kind` from the iteration's session_end row when
        /// one is present (e.g. `step_timeout`, `wall_clock_timeout`,
        /// `signal`, `usage_limit_reached`). `None` when no row was written.
        exit_reason: Option<String>,
    },

    // -- Schema errors --------------------------------------------------------
    /// A `$schema` reference could not be resolved or compiled.
    ///
    /// Covers both file resolution failures (e.g. unsupported `http://` URL,
    /// missing file) and schema compilation failures (invalid SimplifiedSchema
    /// or JSON Schema structure).
    #[error("schema load failed for {}: {message}", source_path.display())]
    SchemaLoad {
        /// The prompt file whose `$schema` reference failed to load.
        source_path: PathBuf,
        /// Human-readable description of the failure.
        message: String,
    },

    /// One or more present frontmatter properties failed schema validation.
    ///
    /// This is a hard error: required and optional properties with the wrong
    /// type are reported together. Optional invalid values get dropped during
    /// run-time validation (see Phase 2) but raw validation surfaces them
    /// here so callers can choose their handling.
    #[error(
        "schema validation failed for {}: {message}",
        source_path.display()
    )]
    SchemaValidation {
        /// The prompt file being validated.
        source_path: PathBuf,
        /// Human-readable failure summary.
        message: String,
        /// JSON pointer paths (or property names) of failures, in
        /// declaration order. Empty when the underlying validator did not
        /// expose structured locations.
        problems: Vec<String>,
    },

    /// Required schema properties are missing and cannot be collected
    /// interactively from the current session.
    ///
    /// Returned both when Interactive Mode is not allowed (non-TTY, silent
    /// flag, etc.) and when the user has opted out via `prompt_for_missing`.
    #[error(
        "missing required schema {plural} for {}: {names}",
        source_path.display(),
        plural = if missing.len() == 1 { "property" } else { "properties" },
        names = format_missing_names(missing)
    )]
    MissingProperties {
        /// The prompt file whose schema declares the missing properties.
        source_path: PathBuf,
        /// Missing properties in declaration order. Empty `missing` is
        /// permitted only when `pointer_paths` is populated.
        missing: Vec<MissingProperty>,
        /// Frontmatter `description` value, if any, for additional context
        /// when rendering the error.
        frontmatter_description: Option<String>,
        /// JSON pointer paths from Darkmatter's validation problems used
        /// when the document has raw JSON Schema and no typed metadata is
        /// available.
        pointer_paths: Vec<String>,
    },

    /// A missing required property cannot be mapped to a `biscuit-tui`
    /// widget, so Interactive Mode would fail.
    ///
    /// Examples: raw JSON Schema documents, root-level unions without a
    /// SimplifiedSchema projection, `object`, `any`.
    #[error(
        "missing required property `{property}` in {} has an unsupported \
         schema shape for interactive collection: {shape}",
        source_path.display()
    )]
    UnsupportedInteractiveSchema {
        /// The prompt file whose schema cannot be collected.
        source_path: PathBuf,
        /// The property name (declaration form).
        property: String,
        /// Describes the unsupported shape (e.g. `"object"`, `"any"`,
        /// `"property-level union"`, `"raw JSON Schema"`).
        shape: String,
    },

    /// A loop iteration completed but reported a provider rate limit, and
    /// either the configured `on_rate_limit` policy was `abort` or no
    /// `reset_at` was available to safely pause.
    ///
    /// Maps to exit code [`LOOP_RATE_LIMITED_EXIT_CODE`] (`75`, `EX_TEMPFAIL`).
    #[error(
        "loop halted at iteration {iteration} of {prompt_path}: provider rate limited{}{}{}",
        provider.as_ref().map(|p| format!(" ({p})")).unwrap_or_default(),
        reset_at.as_ref().map(|r| format!("; resets at {}", r.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S"))).unwrap_or_default(),
        message.as_ref().map(|m| format!("\n  ↳ {m}")).unwrap_or_default(),
        prompt_path = prompt_path.display()
    )]
    LoopRateLimited {
        /// 1-based iteration that produced the rate-limit trailer.
        iteration: usize,
        /// Prompt file being executed.
        prompt_path: PathBuf,
        /// Provider id reported by the throttling source, when known.
        provider: Option<String>,
        /// Model id reported by the throttling source, when known.
        model: Option<String>,
        /// When the cap is reported to reset, when known.
        reset_at: Option<DateTime<Utc>>,
        /// Provider-supplied human-readable message, when known.
        message: Option<String>,
    },

    /// Composition succeeded but produced an empty body.
    ///
    /// The Markdown composed cleanly (no transclusion or shell-expansion
    /// failures), but every block in the body evaluated to falsy or the
    /// document had no body to begin with. Sending the result to a provider
    /// CLI would surface as `"Input must be provided ..."` from the provider
    /// rather than naming the real cause, so the composition layer short-
    /// circuits here with an actionable error.
    #[error(
        "composed prompt body is empty for {}{}",
        source_path.display(),
        if provided_overrides.is_empty() {
            String::new()
        } else {
            format!(" (provided: {})", provided_overrides.join(", "))
        }
    )]
    ComposedBodyEmpty {
        /// The prompt file that produced the empty body.
        source_path: PathBuf,
        /// Composition mode in effect when the body was produced.
        mode: super::types::CompositionMode,
        /// Top-level keys of `key=value` / `--set` overrides applied during
        /// composition, in declaration order. Useful for telling the user
        /// which variables were actually visible to `::block when=…`
        /// conditions when the body collapsed to nothing.
        provided_overrides: Vec<String>,
    },

    /// A frontmatter-rooted error carrying a captured source excerpt for
    /// rendering.
    ///
    /// Produced at the render boundary by
    /// [`enrich_frontmatter`](CompositionError::enrich_frontmatter), never at a
    /// construction site, so control-flow `match`es upstream operate on the
    /// unwrapped variant. `Display` delegates to `inner`. The CLI error walker
    /// recognizes this wrapper, renders the deepest block from `inner`, and
    /// appends the [`FrontmatterExcerpt`] after it.
    #[error("{inner}")]
    WithFrontmatter {
        /// The original frontmatter-rooted error.
        inner: Box<CompositionError>,
        /// The captured frontmatter block, appended after the inner diagnostic.
        excerpt: FrontmatterExcerpt,
    },
}

/// A single required schema property that is missing from frontmatter.
///
/// Carries enough metadata to render an actionable error (declaration name,
/// type label, optional description) without holding a Darkmatter schema
/// reference. Constructed by the validation layer after consulting the
/// effective SimplifiedSchema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingProperty {
    /// Property name as it appears in the schema.
    pub name: String,
    /// Schema type label (e.g. `string`, `enum(a|b|c)`, `file`, `number`).
    /// `None` when the underlying schema is raw JSON Schema and no typed
    /// metadata is available.
    pub type_label: Option<String>,
    /// Optional description from the schema for the property.
    pub description: Option<String>,
    /// Interactive widget shape used by the CLI to drive a `biscuit-tui`
    /// prompt. `None` when the property has a shape that cannot be
    /// collected interactively (raw JSON Schema, property-level union,
    /// `object`, `any`, etc.).
    pub interactive_shape: Option<InteractiveShape>,
}

/// Widget mapping for interactively collecting a missing property value.
///
/// Produced by the validation layer from the property's
/// [`SimplifiedType`][darkmatter::markdown::schemas::SimplifiedType] plus
/// any enum members. The CLI layer chooses the concrete `biscuit-tui`
/// widget from this enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractiveShape {
    /// Plain text input (string, date, datetime, time, url, email, file).
    Text {
        /// Hint about the expected text format. Used for placeholder /
        /// help text only — Darkmatter handles validation post-submission.
        format: TextFormat,
    },
    /// Numeric input with parse-and-retry validation.
    Number {
        /// `true` when the property is constrained to integer values
        /// (via the SimplifiedSchema `integer` constraint).
        integer: bool,
    },
    /// Boolean on/off toggle (covers both `boolean` and `boolish`).
    Boolean,
    /// Single-choice enum from a fixed member list.
    EnumOne {
        /// Enum member names in declaration order.
        members: Vec<String>,
    },
    /// Multiple-choice enum from a fixed member list.
    EnumMany {
        /// Enum member names in declaration order.
        members: Vec<String>,
    },
}

/// Hint about the expected text format for a [`InteractiveShape::Text`]
/// prompt.
///
/// Used for placeholder text and help hints only — Darkmatter runs the
/// authoritative validation after the user submits a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFormat {
    /// Plain string with no special format.
    Plain,
    /// ISO-8601 date (`YYYY-MM-DD`).
    Date,
    /// ISO-8601 datetime.
    DateTime,
    /// Time-of-day with optional timezone.
    Time,
    /// Absolute URL.
    Url,
    /// Email address.
    Email,
    /// File reference (resolved via `biscuit-file::FileReference`).
    File,
}

impl TextFormat {
    /// One-line human-readable label for the format.
    pub fn label(self) -> &'static str {
        match self {
            TextFormat::Plain => "string",
            TextFormat::Date => "date (YYYY-MM-DD)",
            TextFormat::DateTime => "datetime (ISO-8601)",
            TextFormat::Time => "time",
            TextFormat::Url => "URL",
            TextFormat::Email => "email",
            TextFormat::File => "file path or reference",
        }
    }
}

/// Pipeline stage where a [`DroppedOptional`] was elided.
///
/// Schema validation runs in three places. Each stage surfaces dropped
/// optionals so the CLI can attribute the warning to its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DroppedOptionalStage {
    /// Dropped during pre-flight pre-validation against the raw
    /// frontmatter + setter overrides.
    PreValidation,
    /// Dropped during the prepare-time retry against the post-interpolation
    /// effective frontmatter (Darkmatter compose stage).
    Composition,
    /// Dropped during the post-shell-expansion re-validation against the
    /// final effective frontmatter.
    PostShellExpansion,
}

impl DroppedOptionalStage {
    /// Short human-readable label used in CLI warnings.
    pub fn label(self) -> &'static str {
        match self {
            DroppedOptionalStage::PreValidation => "pre-validation",
            DroppedOptionalStage::Composition => "composition",
            DroppedOptionalStage::PostShellExpansion => "post-shell expansion",
        }
    }
}

/// A schema-optional frontmatter property whose value failed validation
/// and was elided from the run.
///
/// Carries enough metadata for the CLI to render an actionable warning
/// to stderr so users notice when their supplied value silently dropped
/// out of the prompt context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedOptional {
    /// Property name as declared in the schema.
    pub property: String,
    /// Source of the invalid value: file frontmatter, setter override,
    /// or composed effective frontmatter.
    pub source: DroppedOptionalSource,
    /// Pipeline stage where the property was dropped.
    pub stage: DroppedOptionalStage,
    /// Human-readable reason (the underlying validator message).
    pub reason: String,
}

/// Where the invalid value originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DroppedOptionalSource {
    /// File-authored frontmatter value.
    Frontmatter,
    /// CLI `key=value` setter or `--set` JSON override.
    Override,
    /// Composed effective frontmatter (post template / shell expansion).
    Composed,
}

impl DroppedOptionalSource {
    /// Short human-readable label used in CLI warnings.
    pub fn label(self) -> &'static str {
        match self {
            DroppedOptionalSource::Frontmatter => "frontmatter",
            DroppedOptionalSource::Override => "override",
            DroppedOptionalSource::Composed => "composed",
        }
    }
}

fn format_missing_names(missing: &[MissingProperty]) -> String {
    missing
        .iter()
        .map(|m| m.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Per-step missing-property record for [`CompositionError::SequenceMissingProperties`].
///
/// Carries the same fields as a single-step [`CompositionError::MissingProperties`]
/// so the CLI can render each step's report without having to re-validate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceMissingPropertiesStep {
    /// 1-based step number.
    pub step: usize,
    /// Display name of the step.
    pub step_name: String,
    /// The prompt file whose schema declared the missing properties.
    pub source_path: PathBuf,
    /// Missing properties in declaration order.
    pub missing: Vec<MissingProperty>,
    /// Frontmatter `description` value, if any.
    pub frontmatter_description: Option<String>,
    /// JSON pointer paths from Darkmatter's validation problems used
    /// when the document has raw JSON Schema and no typed metadata.
    pub pointer_paths: Vec<String>,
}

/// Per-step failure information for sequence selection errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceSelectionFailure {
    /// 1-based step number.
    pub step: usize,
    /// Display name of the step.
    pub step_name: String,
    /// Why the step failed resolution.
    pub reason: String,
    /// Providers that were installed at the time of resolution.
    pub installed: Vec<Provider>,
}

impl CompositionError {
    /// Attach a captured frontmatter excerpt to a frontmatter-rooted error.
    ///
    /// Called at the render boundary — after all control-flow `match`es on the
    /// unwrapped variant — so the wrapper never interferes with upstream
    /// decision-making. For errors that do not relate to frontmatter, or when
    /// `source` has no parseable frontmatter block, the error is returned
    /// unchanged. Idempotent: an already-wrapped error is returned as-is.
    pub fn enrich_frontmatter(
        self,
        source: &ResolvedCompositionSource,
        stderr_is_tty: bool,
    ) -> Self {
        if matches!(self, CompositionError::WithFrontmatter { .. }) {
            return self;
        }
        let Some(property) = self.frontmatter_block_spec() else {
            return self;
        };
        match FrontmatterExcerpt::capture(&source.original_text, property.as_deref(), stderr_is_tty)
        {
            Some(excerpt) => CompositionError::WithFrontmatter {
                inner: Box::new(self),
                excerpt,
            },
            None => self,
        }
    }

    /// The captured frontmatter excerpt, when this is a wrapped error.
    pub fn frontmatter_excerpt(&self) -> Option<&FrontmatterExcerpt> {
        match self {
            CompositionError::WithFrontmatter { excerpt, .. } => Some(excerpt),
            _ => None,
        }
    }

    /// Whether this error should carry a frontmatter excerpt, and the dotted
    /// property to highlight within it.
    ///
    /// Returns `None` to skip the excerpt entirely; `Some(None)` to show the
    /// block with no highlighted line; `Some(Some(prop))` to highlight the
    /// `prop` key. Body-composition errors (`ComposeFailed`,
    /// `ShellExpansionFailed`) show the frontmatter as context with no
    /// highlight, since their anchor is in the body.
    fn frontmatter_block_spec(&self) -> Option<Option<String>> {
        match self {
            CompositionError::LifecycleInterpolationLeak { property, .. }
            | CompositionError::LifecycleUndefinedVariable { property, .. }
            | CompositionError::LifecycleInvalid { property, .. }
            | CompositionError::LifecycleStackInvalidShape { property, .. }
            | CompositionError::LifecycleActionInvalidShortForm { property, .. }
            | CompositionError::LifecycleActionInvalidLongForm { property, .. }
            | CompositionError::LifecycleActionPlacement { property, .. }
            | CompositionError::LifecycleMultipleLifecycleActions { property, .. }
            | CompositionError::LifecycleActionOrder { property, .. }
            | CompositionError::LifecycleInvalidArgs { property, .. }
            | CompositionError::LifecycleErrNotAvailable { property, .. } => {
                Some(Some(property.clone()))
            }
            CompositionError::LifecycleSayConflict(property)
            | CompositionError::LifecycleUnknownEffect(property, _) => Some(Some(property.clone())),
            CompositionError::RemovedValidationKey { key, .. } => Some(Some(key.clone())),
            CompositionError::PromptPropertyMissing
            | CompositionError::PromptPropertyWrongType(_) => Some(Some("prompt".to_string())),
            CompositionError::AgentHintWrongType(_)
            | CompositionError::AgentResolutionFailed { .. } => Some(Some("agent".to_string())),
            CompositionError::ModelHintWrongType(_) => Some(Some("model".to_string())),
            CompositionError::InteractiveHintWrongType(_) => {
                Some(Some("interactive".to_string()))
            }
            CompositionError::SchemaLoad { .. } => Some(Some("$schema".to_string())),
            CompositionError::InlineHashMalformed(_) => Some(Some("hash".to_string())),
            CompositionError::UnsupportedInteractiveSchema { property, .. } => {
                Some(Some(property.clone()))
            }
            CompositionError::MissingProperties {
                missing,
                pointer_paths,
                ..
            } => match (missing.split_first(), pointer_paths.split_first()) {
                (Some((only, [])), _) => Some(Some(only.name.clone())),
                (_, Some((only, []))) => Some(Some(pointer_to_dotted(only))),
                _ => Some(None),
            },
            CompositionError::SchemaValidation { problems, .. } => match problems.split_first() {
                Some((only, [])) => Some(Some(pointer_to_dotted(only))),
                _ => Some(None),
            },
            CompositionError::InlineComposeSequenceMismatch { .. }
            | CompositionError::ComposeFailed(_)
            | CompositionError::ShellExpansionFailed { .. } => Some(None),
            _ => None,
        }
    }
}

impl BlockError for CompositionError {
    fn status_block(&self, term: &Terminal) -> StatusBlock {
        match self {
            CompositionError::WithFrontmatter { inner, .. } => inner.status_block(term),
            CompositionError::LifecycleInvalid {
                property,
                source_file,
                unknown_field,
                expected_fields,
                ..
            } => {
                let dotted_property = match unknown_field {
                    Some(field) => format!("{property}.{field}"),
                    None => property.clone(),
                };

                let file_display = source_file.display().to_string();
                let escaped = escape_prose_path(&file_display);
                let file_link = format!(
                    "<a href=\"{escaped}\">{}</a>",
                    escape_prose_path(&source_file.file_name().map_or_else(
                        || file_display.to_string(),
                        |n| n.to_string_lossy().to_string()
                    ))
                );

                let mut body =
                    format!("Unknown property <cyan>`{dotted_property}`</cyan> in {file_link}");
                if !expected_fields.is_empty() {
                    body.push_str("\n\n<b>Expected one of:</b>");
                    for field in expected_fields {
                        body.push_str(&format!("\n- <cyan>`{field}`</cyan>"));
                    }
                }

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "invalid lifecycle property",
                    ))
                    .body(body)
                    .hint("Check the lifecycle frontmatter section in your prompt file.")
            }
            CompositionError::LifecycleInterpolationLeak {
                source_path,
                property,
                expression,
                reason,
            } => {
                let file_link = render_file_link(source_path);
                let mut body = format!(
                    "Interpolation span leaked in lifecycle property \
                     <cyan>`{property}`</cyan> in {file_link}.\n\n\
                     <b>Expression:</b> <cyan>`{}`</cyan>",
                    escape_prose_path(expression)
                );
                if !reason.is_empty() {
                    body.push_str("\n\n<b>Reason:</b> ");
                    body.push_str(&escape_prose_path(reason));
                }

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "lifecycle interpolation leaked",
                    ))
                    .body(body)
                    .hint(
                        "Fix the expression grammar or define the referenced variable in the \
                         lifecycle frontmatter section of your prompt file.",
                    )
            }
            CompositionError::LifecycleUndefinedVariable {
                source_path,
                property,
                variable,
            } => {
                let file_link = render_file_link(source_path);
                let body = format!(
                    "Lifecycle property <cyan>`{property}`</cyan> in {file_link} references \
                     undefined variable <cyan>`{}`</cyan>, which composition resolves to an \
                     empty string.",
                    escape_prose_path(variable)
                );

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "undefined lifecycle variable",
                    ))
                    .body(body)
                    .hint(
                        "Define the variable in frontmatter, prefix a runtime value with \
                         `ctx.`/`env.`, or supply a fallback (`{{ var || 'default' }}`).",
                    )
            }
            CompositionError::RemovedValidationKey {
                source_path,
                key,
                replacement,
            } => {
                let file_link = render_file_link(source_path);
                let body = format!(
                    "The validation/handler key <cyan>`{}`</cyan> in {file_link} has been \
                     removed. Use the lifecycle stack model instead.\n\n\
                     <b>Replacement:</b> {}",
                    escape_prose_path(key),
                    escape_prose_path(replacement)
                );

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "removed validation/handler key",
                    ))
                    .body(body)
                    .hint(
                        "See the lifecycle documentation for `initialize`, `start`, `success`, \
                         `blocked`, `failure`, `finalize`, and `loop` stacks.",
                    )
            }
            CompositionError::LifecycleStackInvalidShape {
                source_path,
                property,
                message,
            } => {
                let file_link = render_file_link(source_path);
                let body = format!(
                    "Lifecycle stack item in <cyan>`{property}`</cyan> in {file_link} has an \
                     invalid shape.\n\n{message}"
                );
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "invalid lifecycle stack item",
                    ))
                    .body(body)
                    .hint(
                        "A stack item is an object with an optional `when:` condition string \
                         and an `action:` (scalar or array). Remove any extra keys.",
                    )
            }
            CompositionError::LifecycleActionInvalidShortForm {
                source_path,
                property,
                raw,
                message,
            } => {
                let file_link = render_file_link(source_path);
                let body = format!(
                    "Short-form lifecycle action <cyan>`{}`</cyan> in <cyan>`{property}`</cyan> \
                     in {file_link} could not be parsed.\n\n{message}",
                    escape_prose_path(raw)
                );
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "invalid lifecycle action",
                    ))
                    .body(body)
                    .hint(
                        "Short-form actions use `verb(args)` grammar where args are Darkmatter \
                         expressions. Multi-word literals must be quoted: \
                         `say('using codex')`, not `say(using codex)`.",
                    )
            }
            CompositionError::LifecycleActionInvalidLongForm {
                source_path,
                property,
                action,
                message,
            } => {
                let file_link = render_file_link(source_path);
                let body = format!(
                    "Long-form lifecycle action <cyan>`{action}`</cyan> in <cyan>`{property}`</cyan> \
                     in {file_link} could not be parsed.\n\n{message}"
                );
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "invalid lifecycle action",
                    ))
                    .body(body)
            }
            CompositionError::LifecycleActionPlacement {
                source_path,
                property: _,
                action,
                event,
            } => {
                let file_link = render_file_link(source_path);
                let body = format!(
                    "Lifecycle action <cyan>`{action}`</cyan> is not valid in the \
                     <cyan>`{event}`</cyan> event in {file_link}."
                );
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "lifecycle action not valid here",
                    ))
                    .body(body)
                    .hint(
                        "Check the \"Where valid\" matrix in the lifecycle spec: only certain \
                         control actions are allowed in each event.",
                    )
            }
            CompositionError::LifecycleMultipleLifecycleActions {
                source_path, property, ..
            } => {
                let file_link = render_file_link(source_path);
                let body = format!(
                    "Stack item in <cyan>`{property}`</cyan> in {file_link} contains more than \
                     one lifecycle control action."
                );
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "multiple lifecycle actions",
                    ))
                    .body(body)
                    .hint(
                        "Split the actions across separate stack items, or remove the extra \
                         lifecycle action. At most one lifecycle control action is allowed per \
                         `when/action` block.",
                    )
            }
            CompositionError::LifecycleActionOrder {
                source_path, property, ..
            } => {
                let file_link = render_file_link(source_path);
                let body = format!(
                    "Lifecycle action in <cyan>`{property}`</cyan> in {file_link} must be the \
                     last action in its stack item."
                );
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "lifecycle action must be last",
                    ))
                    .body(body)
                    .hint(
                        "A lifecycle control action terminates stack processing, so any actions \
                         after it would never run. Move it to the end of the `action:` array.",
                    )
            }
            CompositionError::LifecycleInvalidArgs {
                source_path,
                property,
                action,
                message,
            } => {
                let file_link = render_file_link(source_path);
                let body = format!(
                    "Lifecycle action <cyan>`{action}`</cyan> in <cyan>`{property}`</cyan> in \
                     {file_link} has invalid arguments.\n\n{message}"
                );
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "invalid lifecycle action arguments",
                    ))
                    .body(body)
            }
            CompositionError::LifecycleErrNotAvailable {
                source_path,
                property,
                event,
            } => {
                let file_link = render_file_link(source_path);
                let body = format!(
                    "Lifecycle property <cyan>`{property}`</cyan> in {file_link} references the \
                     <cyan>`err`</cyan> global in the <cyan>`{event}`</cyan> event, which never \
                     carries an error."
                );
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "`err` not available in this event",
                    ))
                    .body(body)
                    .hint(
                        "Use `err` only in `blocked`, `failure`, or `finalize` (where it is \
                         optional). To reference a frontmatter property named `err`, write \
                         `doc.err` explicitly.",
                    )
            }
            CompositionError::LoopIterationFailed {
                iteration,
                exit_code,
                reason,
                exit_reason,
                ..
            } => {
                // Surface the actionable cause (`step_timeout`,
                // `wall-clock timeout`, signal, …) in the header instead of
                // the generic `composition failed` line. The cause comes
                // from the iteration's session_end JSONL row's
                // `extra.exit_reason` — not from `LoopInvalid` (which is
                // reserved for frontmatter parse errors).
                let title = exit_reason
                    .clone()
                    .unwrap_or_else(|| "iteration failed".to_string());
                let body =
                    format!("Iteration {iteration} exited with code {exit_code}.\n\n{reason}");
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("CompositionError", &title))
                    .body(body)
            }
            CompositionError::LoopRateLimited { .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "rate limited"))
                .body(self.to_string())
                .hint(
                    "Re-run after the listed reset time, or use \
                         `--on-rate-limit pause` to wait automatically.",
                ),
            CompositionError::SchemaLoad {
                source_path,
                message,
            } => {
                let file_link = render_file_link(source_path);
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("CompositionError", "schema load failed"))
                    .body(format!(
                        "Could not load the `$schema` referenced by {file_link}.\n\n{message}"
                    ))
                    .hint(
                        "Verify the `$schema` path is correct, relative to the prompt's parent \
                         directory. Remote `http://` / `https://` references are not supported.",
                    )
            }
            CompositionError::SchemaValidation {
                source_path,
                message,
                problems,
            } => {
                let file_link = render_file_link(source_path);
                let mut body = format!("Schema validation failed for {file_link}.\n\n{message}");
                if !problems.is_empty() {
                    body.push_str("\n\n<b>Problems:</b>");
                    for problem in problems {
                        body.push_str(&format!("\n- <cyan>`{problem}`</cyan>"));
                    }
                }
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("CompositionError", "schema validation"))
                    .body(body)
            }
            CompositionError::MissingProperties {
                source_path,
                missing,
                frontmatter_description,
                pointer_paths,
            } => render_missing_properties_block(
                source_path,
                missing,
                frontmatter_description.as_deref(),
                pointer_paths,
            ),
            CompositionError::SequenceMissingProperties { failures, .. } => {
                render_sequence_missing_properties_block(failures)
            }
            CompositionError::SequenceInteractiveRejected(source_path) => {
                let file_link = render_file_link(source_path);
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "interactive rejected for sequence",
                    ))
                    .body(format!(
                        "The document {file_link} sets <cyan>`interactive: true`</cyan> in its \
                         frontmatter, but a <cyan>`sequence`</cyan> is serial automation and does \
                         not support interactive sessions.\n\n\
                         Use <cyan>`claudine compose`</cyan> or <cyan>`claudine inline-compose`</cyan> \
                         for dialog-shaped prompts. To run an individual sequence step \
                         interactively, use the <cyan>`--interactive`</cyan> CLI flag — this remains \
                         the only explicit override."
                    ))
            }
            CompositionError::UnsupportedInteractiveSchema {
                source_path,
                property,
                shape,
            } => {
                let file_link = render_file_link(source_path);
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "unsupported interactive schema",
                    ))
                    .body(format!(
                        "Required property <cyan>`{property}`</cyan> in {file_link} has shape \
                         <i>{shape}</i>, which cannot be collected interactively."
                    ))
                    .hint(
                        "Pass the value with key=value or --set, or provide it in the prompt's \
                         frontmatter.",
                    )
            }
            CompositionError::InlineComposeSequenceMismatch { source_path } => {
                render_inline_sequence_mismatch_block(source_path)
            }
            CompositionError::AgentResolutionFailed {
                source_path,
                state,
                installed,
            } => {
                let file_link = render_file_link(source_path);
                let body = render_agent_resolution_failed_body(state, installed, &file_link);
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("CompositionError", "agent resolution failed"))
                    .body(body)
                    .hint(
                        "Specify an installed provider with --claude, --codex, etc., run in an \
                         interactive terminal, or correct the `agent` frontmatter property."
                    )
            }
            CompositionError::ComposedBodyEmpty {
                source_path,
                mode,
                provided_overrides,
            } => {
                let file_link = render_file_link(source_path);
                let mode_label = match mode {
                    super::types::CompositionMode::ChainedDocument => "chained (compose)",
                    super::types::CompositionMode::InlineFrontmatterPrompt => {
                        "inline (inline-compose)"
                    }
                };
                let mut body = format!(
                    "Composition produced an <b>empty prompt body</b> for {file_link}.\n\n\
                     Mode: <i>{mode_label}</i>"
                );
                if provided_overrides.is_empty() {
                    body.push_str("\n\nNo `key=value` overrides were provided.");
                } else {
                    body.push_str("\n\n<b>Provided overrides:</b>");
                    for key in provided_overrides {
                        body.push_str(&format!("\n- <cyan>`{key}`</cyan>"));
                    }
                }
                body.push_str(
                    "\n\nThe document composed without error, but every block in the body \
                     was stripped by its `when` condition (or the body was empty to begin with). \
                     The provider CLI would otherwise reject this as \"Input must be provided …\" \
                     without naming the real cause.",
                );
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "composed prompt is empty",
                    ))
                    .body(body)
                    .hint(
                        "Check that the variables you passed match the `::block when=…` \
                         conditions in the prompt, or verify there is body content outside any \
                         conditional block.",
                    )
            }
            CompositionError::ShellExpansionFailed { error, .. } => {
                // Delegate to the structured shell-expansion block so the
                // linked source path, source excerpt, composed frontmatter
                // block, and captured stderr/stdout all survive the claudine
                // boundary instead of being flattened by the catch-all arm.
                error.status_block(term)
            }
            _ => {
                let msg = self.to_string();
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("CompositionError", "composition failed"))
                    .body(msg)
            }
        }
    }

    /// Strips escape bytes from the rendered status block when the terminal has
    /// no color depth.
    ///
    /// `StatusBlock`'s bespoke path (entered whenever the error header carries
    /// `<b>` markup, i.e. every variant here) emits SGR styling and OSC 8 links
    /// even at [`ColorDepth::None`]. On a piped / `NO_COLOR` / JSON terminal
    /// those bytes must be removed so pipeable output stays plain text per the
    /// error-formatting contract. Frontmatter YAML blocks are appended
    /// separately by the CLI error walker (see `output::error_walker`).
    ///
    /// [`ColorDepth::None`]: biscuit_terminal::discovery::detection::ColorDepth::None
    fn report_block_error(&self, term: &Terminal) -> String {
        let out = self.status_block(term).render(term);
        if matches!(
            term.color_depth,
            biscuit_terminal::discovery::detection::ColorDepth::None
        ) {
            biscuit_terminal::utils::escape_codes::strip_escape_codes(&out)
        } else {
            out
        }
    }
}

/// Convert a JSON pointer (`/success/message`) or a bare property name into the
/// dotted form (`success.message`) that
/// [`locate_property_line`](super::frontmatter_excerpt::locate_property_line)
/// expects. A leading `/` and any pointer escaping (`~1` → `/`, `~0` → `~`) are
/// normalized; a value without a leading `/` is treated as already-dotted.
fn pointer_to_dotted(pointer: &str) -> String {
    let trimmed = pointer.trim_start_matches('/');
    if pointer.starts_with('/') {
        trimmed
            .split('/')
            .map(|seg| seg.replace("~1", "/").replace("~0", "~"))
            .collect::<Vec<_>>()
            .join(".")
    } else {
        trimmed.to_string()
    }
}

/// Render an absolute OSC8 hyperlink to `path` showing its relative form
/// where possible (falling back to the full display).
///
/// The Prose layer downgrades `<a href>` to plain text when the terminal
/// does not support OSC8.
fn render_file_link(path: &std::path::Path) -> String {
    let abs = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
    let abs_display = abs.display().to_string();
    let label = path.display().to_string();
    format!(
        "<a href=\"{}\">{}</a>",
        escape_prose_path(&abs_display),
        escape_prose_path(&label)
    )
}

/// Render the diagnostic block for
/// [`CompositionError::InlineComposeSequenceMismatch`].
///
/// Builds the blank-line-separated paragraph sequence: the opening statement;
/// the explanation (document link, both property names, what `sequence` does,
/// and the `claudine sequence` directive); and the upcoming-`sections` note.
/// The authored frontmatter YAML block is appended after this diagnostic by the
/// CLI error walker when the error is enriched with a [`FrontmatterExcerpt`].
fn render_inline_sequence_mismatch_block(source_path: &std::path::Path) -> StatusBlock {
    let file_link = render_file_link(source_path);

    let opening =
        Prose::new("You tried to run an inline-compose operation on a document configured as a sequence.");

    let explanation = Prose::new(format!(
        "The document {file_link} defines both <cyan>`prompt`</cyan> and <cyan>`sequence`</cyan>. \
         A <cyan>`sequence`</cyan> makes each state invoke an inline-compose operation using \
         <cyan>`prompt`</cyan>, so run it with <cyan>`claudine sequence`</cyan> instead."
    ));

    let sections_note = Prose::new(
        "Note: the upcoming <cyan>`sections`</cyan> feature may be a better fit when each \
         operation should update a particular section of the document. It may not suit every \
         sequence workflow and is not available yet.",
    );

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new(
            "CompositionError",
            "inline-compose on a sequence",
        ))
        .body(vec![opening, explanation, sections_note])
        .hint("Run the document with `claudine sequence <file>`.")
}

/// Render the human-facing body for [`CompositionError::AgentResolutionFailed`].
///
/// The no-TTY abort body must be the **same** styled message the TTY path
/// would show for the state, so it shares one source of truth with the
/// dry-run table cell and the live TTY pre-prompt — see
/// [`super::agent_message`]. The only state with a distinct (imperative)
/// live message is [`AgentResolutionState::SingleInvalid`], which is built
/// here from [`super::agent_message::invalid_agent_message`] plus the
/// installed-agent list the TTY picker would offer.
fn render_agent_resolution_failed_body(
    state: &super::types::AgentResolutionState,
    installed: &[Provider],
    file_link: &str,
) -> String {
    use super::agent_message::{agent_state_breakdown, invalid_agent_message};
    use super::types::AgentResolutionState;

    match state {
        AgentResolutionState::SingleInvalid { hint } => {
            let mut body = invalid_agent_message(hint, file_link);
            if installed.is_empty() {
                body.push_str("\n\n<i><dim>(no agents are installed)</dim></i>");
            } else {
                for provider in installed {
                    body.push_str(&format!("\n- {provider}"));
                }
            }
            body
        }
        // Auto-selecting states never abort; keep a diagnostic if they
        // somehow reach this path.
        AgentResolutionState::ListOneInstalled { .. } => format!(
            "Agent resolution unexpectedly aborted for {file_link} despite an auto-selectable suggestion."
        ),
        AgentResolutionState::Selected { provider } => format!(
            "Agent resolution unexpectedly aborted for {file_link} when <b>{provider}</b> was already selected."
        ),
        // Every other prompting state aborts with the same breakdown the
        // dry-run table predicts and the TTY path shows.
        other => agent_state_breakdown(other),
    }
}

fn render_sequence_missing_properties_block(
    failures: &[SequenceMissingPropertiesStep],
) -> StatusBlock {
    let plural = if failures.len() == 1 { "step" } else { "steps" };
    let mut body = format!(
        "Missing required schema properties in {} {plural} of the sequence.",
        failures.len()
    );

    for failure in failures {
        let file_link = render_file_link(&failure.source_path);
        body.push_str(&format!(
            "\n\n<b>Step {}: <cyan>{}</cyan></b> ({file_link})",
            failure.step,
            escape_prose_path(&failure.step_name),
        ));
        if let Some(desc) = failure
            .frontmatter_description
            .as_deref()
            .filter(|d| !d.trim().is_empty())
        {
            body.push_str(&format!("\n  <i><dim>{}</dim></i>", escape_prose_path(desc)));
        }
        if !failure.missing.is_empty() {
            for prop in &failure.missing {
                let type_label = prop
                    .type_label
                    .as_deref()
                    .filter(|t| !t.is_empty())
                    .unwrap_or("(unknown type)");
                let mut line = format!("\n  - <cyan>`{}`</cyan>: {}", prop.name, type_label);
                if let Some(desc) = prop.description.as_deref().filter(|d| !d.trim().is_empty()) {
                    line.push_str(&format!(" <i><dim>— {}</dim></i>", escape_prose_path(desc)));
                }
                body.push_str(&line);
            }
        } else if !failure.pointer_paths.is_empty() {
            for pointer in &failure.pointer_paths {
                body.push_str(&format!("\n  - <cyan>`{pointer}`</cyan>"));
            }
        }
    }

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new(
            "CompositionError",
            "sequence missing properties",
        ))
        .body(body)
        .hint(
            "Fix the missing values in the sequence document (or pass them via --set) and re-run; \
             every step is validated before the first provider session starts.",
        )
}

fn render_missing_properties_block(
    source_path: &std::path::Path,
    missing: &[MissingProperty],
    frontmatter_description: Option<&str>,
    pointer_paths: &[String],
) -> StatusBlock {
    let file_link = render_file_link(source_path);

    let mut body = format!("Required {plural} missing in {file_link}.",
        plural = if missing.len() == 1 { "property is" } else { "properties are" });

    if let Some(desc) = frontmatter_description.filter(|d| !d.trim().is_empty()) {
        body.push_str(&format!("\n\n<i><dim>{}</dim></i>", escape_prose_path(desc)));
    }

    if !missing.is_empty() {
        body.push_str("\n\n<b>Missing:</b>");
        for prop in missing {
            let type_label = prop
                .type_label
                .as_deref()
                .filter(|t| !t.is_empty())
                .unwrap_or("(unknown type)");
            let mut line = format!("\n- <cyan>`{}`</cyan>: {}", prop.name, type_label);
            if let Some(desc) = prop.description.as_deref().filter(|d| !d.trim().is_empty()) {
                line.push_str(&format!(" <i><dim>— {}</dim></i>", escape_prose_path(desc)));
            }
            body.push_str(&line);
        }
    } else if !pointer_paths.is_empty() {
        body.push_str("\n\n<b>Validation problems:</b>");
        for pointer in pointer_paths {
            body.push_str(&format!("\n- <cyan>`{pointer}`</cyan>"));
        }
    }

    StatusBlock::new(StatusState::Error)
        .error_header(ErrorHeader::new("CompositionError", "missing properties"))
        .body(body)
        .hint(
            "Pass key=value, use --set, or set prompt_for_missing to true in an interactive \
             terminal.",
        )
}

fn escape_prose_path(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '<' | '>' | '{' | '"' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_from(text: &str) -> ResolvedCompositionSource {
        ResolvedCompositionSource {
            original_ref: "review.md".to_string(),
            resolved_path: PathBuf::from("review.md"),
            original_text: text.to_string(),
            markdown: text.to_string().into(),
        }
    }

    #[test]
    fn enrich_wraps_lifecycle_leak_with_excerpt() {
        let source = source_from(
            "---\nreview_file: x\nsuccess:\n    message: \"at {{review-file}}\"\n---\nbody\n",
        );
        let err = CompositionError::LifecycleInterpolationLeak {
            source_path: PathBuf::from("review.md"),
            property: "success.message".to_string(),
            expression: "review-file".to_string(),
            reason: String::new(),
        }
        .enrich_frontmatter(&source, true);

        assert!(matches!(err, CompositionError::WithFrontmatter { .. }));
        assert!(err.frontmatter_excerpt().is_some());
        // Display still delegates to the inner leak diagnostic.
        assert!(err.to_string().contains("interpolation leaked"), "got: {err}");
    }

    #[test]
    fn enrich_is_noop_for_unrelated_error() {
        let source = source_from("---\ntitle: x\n---\nbody\n");
        let err = CompositionError::NoRunnableProviders.enrich_frontmatter(&source, true);
        assert!(matches!(err, CompositionError::NoRunnableProviders));
    }

    #[test]
    fn enrich_is_idempotent() {
        let source = source_from("---\ntitle: x\n---\nbody\n");
        let err = CompositionError::PromptPropertyMissing
            .enrich_frontmatter(&source, true)
            .enrich_frontmatter(&source, true);
        // Wrapped exactly once — the inner is the bare missing-prompt error.
        match err {
            CompositionError::WithFrontmatter { inner, .. } => {
                assert!(matches!(*inner, CompositionError::PromptPropertyMissing));
            }
            other => panic!("expected WithFrontmatter, got: {other:?}"),
        }
    }

    #[test]
    fn loop_iteration_failed_display_surfaces_reason_and_iteration() {
        let err = CompositionError::LoopIterationFailed {
            iteration: 2,
            prompt_path: PathBuf::from("fixes/plan.md"),
            exit_code: 1,
            reason: "step_timeout after 30m of stream silence".to_string(),
            exit_reason: Some("step_timeout".to_string()),
        };
        let rendered = err.to_string();
        assert!(
            rendered.contains("loop iteration 2 of fixes/plan.md"),
            "got: {rendered}"
        );
        assert!(
            rendered.contains("step_timeout after 30m of stream silence"),
            "got: {rendered}"
        );
        assert!(
            !rendered.contains("invalid loop definition"),
            "got: {rendered}"
        );
    }

    #[test]
    fn loop_rate_limited_display_includes_reset_time_when_present() {
        let reset = DateTime::parse_from_rfc3339("2026-05-12T18:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let err = CompositionError::LoopRateLimited {
            iteration: 1,
            prompt_path: PathBuf::from("plan.md"),
            provider: Some("k2p6".to_string()),
            model: Some("kimi-for-coding".to_string()),
            reset_at: Some(reset),
            message: Some("Usage limit reached for k2p6".to_string()),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("k2p6"), "got: {rendered}");
        assert!(rendered.contains("resets at"), "got: {rendered}");
        assert!(rendered.contains("Usage limit reached"), "got: {rendered}");
    }

    #[test]
    fn loop_rate_limited_display_omits_optional_fields_when_absent() {
        let err = CompositionError::LoopRateLimited {
            iteration: 3,
            prompt_path: PathBuf::from("plan.md"),
            provider: None,
            model: None,
            reset_at: None,
            message: None,
        };
        let rendered = err.to_string();
        assert!(
            rendered.starts_with("loop halted at iteration 3 of plan.md"),
            "got: {rendered}"
        );
        assert!(
            rendered.contains("provider rate limited"),
            "got: {rendered}"
        );
        // No reset clause when reset_at is absent
        assert!(!rendered.contains("resets at"), "got: {rendered}");
    }

    #[test]
    fn loop_iteration_failed_falls_back_when_no_exit_reason() {
        let err = CompositionError::LoopIterationFailed {
            iteration: 4,
            prompt_path: PathBuf::from("plan.md"),
            exit_code: 1,
            reason: "provider exited non-zero".to_string(),
            exit_reason: None,
        };
        let rendered = err.to_string();
        assert!(
            rendered.contains("provider exited non-zero"),
            "got: {rendered}"
        );
    }

    #[test]
    fn loop_invalid_still_reserved_for_frontmatter_problems() {
        // Sanity: LoopInvalid is still the right variant for malformed
        // frontmatter and renders distinctly from the runtime-fault
        // variants above.
        let err = CompositionError::LoopInvalid("`loop.max` must be greater than zero".to_string());
        let rendered = err.to_string();
        assert!(
            rendered.starts_with("invalid loop definition:"),
            "got: {rendered}"
        );
    }

    // -------------------------------------------------------------------------
    // Schema errors
    // -------------------------------------------------------------------------

    #[test]
    fn schema_load_display_includes_path_and_message() {
        let err = CompositionError::SchemaLoad {
            source_path: PathBuf::from("prompts/plan.md"),
            message: "unsupported `http://` schema reference".to_string(),
        };
        let rendered = err.to_string();
        assert!(
            rendered.contains("prompts/plan.md"),
            "expected path in display: {rendered}"
        );
        assert!(
            rendered.contains("unsupported `http://`"),
            "expected message in display: {rendered}"
        );
    }

    #[test]
    fn schema_validation_display_includes_message() {
        let err = CompositionError::SchemaValidation {
            source_path: PathBuf::from("prompts/plan.md"),
            message: "expected number, got string".to_string(),
            problems: vec!["/properties/count".to_string()],
        };
        let rendered = err.to_string();
        assert!(
            rendered.contains("expected number, got string"),
            "got: {rendered}"
        );
    }

    #[test]
    fn missing_properties_display_lists_names_in_order() {
        let err = CompositionError::MissingProperties {
            source_path: PathBuf::from("prompts/plan.md"),
            missing: vec![
                MissingProperty {
                    name: "target".to_string(),
                    type_label: Some("string".to_string()),
                    description: None,
                    interactive_shape: None,
                },
                MissingProperty {
                    name: "count".to_string(),
                    type_label: Some("number".to_string()),
                    description: None,
                    interactive_shape: None,
                },
            ],
            frontmatter_description: None,
            pointer_paths: Vec::new(),
        };
        let rendered = err.to_string();
        assert!(
            rendered.contains("target, count"),
            "expected declaration-order names: {rendered}"
        );
        assert!(
            rendered.contains("properties"),
            "expected plural form for >1 missing: {rendered}"
        );
    }

    #[test]
    fn missing_properties_display_uses_singular_for_one() {
        let err = CompositionError::MissingProperties {
            source_path: PathBuf::from("prompts/plan.md"),
            missing: vec![MissingProperty {
                name: "target".to_string(),
                type_label: Some("string".to_string()),
                description: None,
                interactive_shape: None,
            }],
            frontmatter_description: None,
            pointer_paths: Vec::new(),
        };
        let rendered = err.to_string();
        assert!(
            rendered.contains("property") && !rendered.contains("properties"),
            "expected singular form: {rendered}"
        );
    }

    #[test]
    fn unsupported_interactive_schema_display_mentions_shape() {
        let err = CompositionError::UnsupportedInteractiveSchema {
            source_path: PathBuf::from("prompts/plan.md"),
            property: "config".to_string(),
            shape: "object".to_string(),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("`config`"), "got: {rendered}");
        assert!(rendered.contains("object"), "got: {rendered}");
    }

    #[test]
    fn missing_properties_status_block_includes_remediation_hint() {
        use biscuit_terminal::prelude::TerminalRenderable;
        use biscuit_terminal::terminal::Terminal;
        let err = CompositionError::MissingProperties {
            source_path: PathBuf::from("prompts/plan.md"),
            missing: vec![MissingProperty {
                name: "target".to_string(),
                type_label: Some("string".to_string()),
                description: Some("the target to act on".to_string()),
                interactive_shape: None,
            }],
            frontmatter_description: Some("Plan a feature".to_string()),
            pointer_paths: Vec::new(),
        };
        let block = err.status_block(&Terminal::default());
        let rendered = block.render(&Terminal::default());
        assert!(
            rendered.contains("Pass key=value")
                || rendered.contains("prompt_for_missing"),
            "expected remediation hint in rendered output: {rendered}"
        );
    }

    #[test]
    fn sequence_missing_properties_status_block_lists_each_step() {
        use biscuit_terminal::prelude::TerminalRenderable;
        use biscuit_terminal::terminal::Terminal;
        let err = CompositionError::SequenceMissingProperties {
            failure_count: 2,
            failures: vec![
                SequenceMissingPropertiesStep {
                    step: 1,
                    step_name: "research".to_string(),
                    source_path: PathBuf::from("prompts/seq.md"),
                    missing: vec![MissingProperty {
                        name: "topic".to_string(),
                        type_label: Some("string".to_string()),
                        description: None,
                        interactive_shape: None,
                    }],
                    frontmatter_description: None,
                    pointer_paths: Vec::new(),
                },
                SequenceMissingPropertiesStep {
                    step: 2,
                    step_name: "summarize".to_string(),
                    source_path: PathBuf::from("prompts/seq.md"),
                    missing: vec![MissingProperty {
                        name: "tone".to_string(),
                        type_label: Some("enum(formal|casual)".to_string()),
                        description: Some("the desired tone".to_string()),
                        interactive_shape: None,
                    }],
                    frontmatter_description: None,
                    pointer_paths: Vec::new(),
                },
            ],
        };
        let block = err.status_block(&Terminal::default());
        let rendered = block.render(&Terminal::default());
        assert!(rendered.contains("Step 1"), "got: {rendered}");
        assert!(rendered.contains("Step 2"), "got: {rendered}");
        assert!(rendered.contains("topic"), "got: {rendered}");
        assert!(rendered.contains("tone"), "got: {rendered}");
        assert!(
            rendered.contains("research") && rendered.contains("summarize"),
            "got: {rendered}"
        );
    }

    #[test]
    fn sequence_missing_properties_display_includes_failure_count() {
        let err = CompositionError::SequenceMissingProperties {
            failure_count: 3,
            failures: Vec::new(),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("3 step(s)"), "got: {rendered}");
    }

    // -------------------------------------------------------------------------
    // Agent-resolution no-TTY abort body parity with the dry-run / TTY message
    // -------------------------------------------------------------------------

    use super::super::agent_message::{agent_state_breakdown, invalid_agent_message};
    use super::super::types::AgentResolutionState;

    const FILE_LINK: &str = "<a href=\"file:///doc.md\">doc.md</a>";

    #[test]
    fn no_tty_no_agent_body_matches_canonical_breakdown() {
        let state = AgentResolutionState::NoAgent;
        let body = render_agent_resolution_failed_body(&state, &[], FILE_LINK);
        assert_eq!(body, agent_state_breakdown(&state));
    }

    #[test]
    fn no_tty_not_installed_body_matches_canonical_breakdown() {
        let state = AgentResolutionState::SingleNotInstalled {
            provider: Provider::Gemini,
        };
        let body = render_agent_resolution_failed_body(&state, &[Provider::Claude], FILE_LINK);
        assert_eq!(body, agent_state_breakdown(&state));
    }

    #[test]
    fn no_tty_list_multiple_body_matches_canonical_breakdown() {
        // Regression: the old body used "the interactive picker would ask …",
        // which drifted from the dry-run cell wording.
        let state = AgentResolutionState::ListMultipleInstalled {
            installed: vec![Provider::Claude, Provider::Codex],
            not_installed: vec![Provider::Gemini],
            invalid: vec!["bad".into()],
        };
        let body = render_agent_resolution_failed_body(&state, &[Provider::Claude], FILE_LINK);
        assert_eq!(body, agent_state_breakdown(&state));
        assert!(
            body.contains("choose interactively between suggested Agents"),
            "got: {body}"
        );
        assert!(!body.contains("the interactive picker would ask"), "got: {body}");
    }

    #[test]
    fn no_tty_zero_installed_body_matches_canonical_breakdown() {
        // Regression: the old body appended "the current session is not
        // interactive", which the TTY/dry-run message never showed.
        let state = AgentResolutionState::ZeroInstalledList {
            not_installed: vec![Provider::Gemini],
            invalid: vec!["bad".into()],
        };
        let body = render_agent_resolution_failed_body(&state, &[Provider::Claude], FILE_LINK);
        assert_eq!(body, agent_state_breakdown(&state));
        assert!(!body.contains("not interactive"), "got: {body}");
    }

    #[test]
    fn no_tty_single_invalid_body_is_imperative_message_plus_installed_list() {
        let state = AgentResolutionState::SingleInvalid {
            hint: "nope".into(),
        };
        let body = render_agent_resolution_failed_body(&state, &[Provider::Claude], FILE_LINK);
        assert!(body.starts_with(&invalid_agent_message("nope", FILE_LINK)), "got: {body}");
        assert!(body.contains(&format!("- {}", Provider::Claude)), "got: {body}");
    }

    #[test]
    fn no_tty_single_invalid_body_notes_no_agents_when_none_installed() {
        let state = AgentResolutionState::SingleInvalid {
            hint: "nope".into(),
        };
        let body = render_agent_resolution_failed_body(&state, &[], FILE_LINK);
        assert!(body.contains("no agents are installed"), "got: {body}");
    }

    #[test]
    fn missing_properties_status_block_lists_pointer_paths_when_no_typed_metadata() {
        use biscuit_terminal::prelude::TerminalRenderable;
        use biscuit_terminal::terminal::Terminal;
        let err = CompositionError::MissingProperties {
            source_path: PathBuf::from("prompts/plan.md"),
            missing: Vec::new(),
            frontmatter_description: None,
            pointer_paths: vec!["/properties/target".to_string()],
        };
        let block = err.status_block(&Terminal::default());
        let rendered = block.render(&Terminal::default());
        assert!(
            rendered.contains("/properties/target"),
            "expected JSON pointer in rendered output: {rendered}"
        );
    }

    // -------------------------------------------------------------------------
    // Inline-compose / sequence mismatch diagnostic (spec criteria 11-16)
    // -------------------------------------------------------------------------

    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    fn mismatch_err() -> CompositionError {
        CompositionError::InlineComposeSequenceMismatch {
            source_path: PathBuf::from("prompts/greeting.md"),
        }
    }

    #[test]
    fn mismatch_render_includes_diagnostic() {
        // The diagnostic names the document, both properties, the `claudine
        // sequence` directive, and the `sections` note. The authored YAML
        // block is appended separately by the CLI walker (tested there).
        let err = mismatch_err();
        let rendered = strip_escape_codes(err.report_block_error_optimistic(Some(80)));
        assert!(rendered.contains("greeting.md"), "document name: {rendered}");
        assert!(rendered.contains("prompt"), "names prompt: {rendered}");
        assert!(rendered.contains("sequence"), "names sequence: {rendered}");
        assert!(
            rendered.contains("claudine sequence"),
            "sequence directive: {rendered}"
        );
        assert!(rendered.contains("sections"), "sections note: {rendered}");
    }

    #[test]
    fn mismatch_plain_terminal_render_has_no_escape_bytes() {
        // A terminal with no color depth cannot display SGR styling or OSC 8
        // hyperlinks, so the rendered diagnostic must contain no escape byte at
        // all — otherwise redirected / `NO_COLOR` output is polluted.
        let mut term = Terminal::builder()
            .width(80)
            .color_depth(biscuit_terminal::discovery::detection::ColorDepth::None)
            .build();
        term.is_nerd_font = Some(false);
        let err = mismatch_err();
        let rendered = err.report_block_error(&term);
        assert!(
            !rendered.contains('\x1b'),
            "plain render must contain no escape byte; got: {rendered:?}"
        );
        assert!(rendered.contains("greeting.md"), "got: {rendered}");
        assert!(rendered.contains("claudine sequence"), "got: {rendered}");
    }

    #[test]
    fn mismatch_display_message_is_plain() {
        // The `#[error(...)]` summary is plain text with no rendering markup.
        let err = mismatch_err();
        let rendered = err.to_string();
        assert!(
            rendered.contains("cannot run a document configured as a sequence"),
            "got: {rendered}"
        );
        assert!(!rendered.contains('<'), "no markup in Display: {rendered}");
    }

    // -------------------------------------------------------------------------
    // Phase 4: regression — hint inside block quote for composition errors
    // -------------------------------------------------------------------------

    #[test]
    fn unsupported_interactive_schema_hint_appears_inside_block_quote_border() {
        use biscuit_terminal::utils::escape_codes::strip_escape_codes;

        let err = CompositionError::UnsupportedInteractiveSchema {
            source_path: PathBuf::from("prompts/review.md"),
            property: "spec".to_string(),
            shape: "(unknown)".to_string(),
        };
        let term = Terminal::new_optimistic(80);
        let rendered = strip_escape_codes(err.report_block_error(&term));

        let hint_token = "Pass the value with key=value";
        let hint_lines: Vec<&str> = rendered
            .lines()
            .filter(|l| l.contains(hint_token))
            .collect();
        assert!(
            !hint_lines.is_empty(),
            "hint text must appear in rendered output: {rendered}"
        );
        for hint_line in &hint_lines {
            assert!(
                hint_line.contains('┃'),
                "regression: hint must appear inside block quote border, got: {hint_line:?}\nfull output:\n{rendered}"
            );
        }

        let body_token = "cannot be collected interactively";
        assert!(
            rendered.contains(body_token),
            "body text must appear: {rendered}"
        );
    }

    #[test]
    fn missing_properties_hint_appears_inside_block_quote_border() {
        use biscuit_terminal::utils::escape_codes::strip_escape_codes;

        let err = CompositionError::MissingProperties {
            source_path: PathBuf::from("prompts/plan.md"),
            missing: vec![MissingProperty {
                name: "target".to_string(),
                type_label: Some("string".to_string()),
                description: None,
                interactive_shape: None,
            }],
            frontmatter_description: None,
            pointer_paths: Vec::new(),
        };
        let term = Terminal::new_optimistic(80);
        let rendered = strip_escape_codes(err.report_block_error(&term));

        let hint_token = "Pass key=value";
        let hint_lines: Vec<&str> = rendered
            .lines()
            .filter(|l| l.contains(hint_token))
            .collect();
        assert!(
            !hint_lines.is_empty(),
            "hint text must appear in rendered output: {rendered}"
        );
        for hint_line in &hint_lines {
            assert!(
                hint_line.contains('┃'),
                "regression: hint must appear inside block quote border, got: {hint_line:?}\nfull output:\n{rendered}"
            );
        }
    }

    #[test]
    fn schema_load_hint_appears_inside_block_quote_border() {
        use biscuit_terminal::utils::escape_codes::strip_escape_codes;

        let err = CompositionError::SchemaLoad {
            source_path: PathBuf::from("prompts/deploy.md"),
            message: "unsupported protocol".to_string(),
        };
        let term = Terminal::new_optimistic(80);
        let rendered = strip_escape_codes(err.report_block_error(&term));

        let hint_token = "Verify the `$schema` path";
        let hint_lines: Vec<&str> = rendered
            .lines()
            .filter(|l| l.contains(hint_token))
            .collect();
        assert!(
            !hint_lines.is_empty(),
            "hint text must appear in rendered output: {rendered}"
        );
        for hint_line in &hint_lines {
            assert!(
                hint_line.contains('┃'),
                "regression: hint must appear inside block quote border, got: {hint_line:?}\nfull output:\n{rendered}"
            );
        }
    }

    // -------------------------------------------------------------------------
    // Phase 4: shell-expansion failure boundary fidelity
    // -------------------------------------------------------------------------

    fn shell_expansion_failed_err() -> CompositionError {
        use darkmatter::markdown::compose::ShellCommandOrigin;

        let content = "---\ntitle: Test\n---\n# Body\n\n::shell \"cmd-that-fails\"\n";
        let ctx = biscuit_terminal::errors::SourceContext::new(
            PathBuf::from("/repo/prompts/test.md"),
            PathBuf::from("prompts/test.md"),
            content,
        );
        let shell = ShellExpansionError::ExecutionFailed {
            ctx: Box::new(ctx),
            command: "cmd-that-fails".to_string(),
            code: 2,
            stdout: "".to_string(),
            stderr: "this command failed\nunknown flag --whatever".to_string(),
            origin: ShellCommandOrigin::Body { line: 6 },
        };
        CompositionError::ShellExpansionFailed {
            source_path: PathBuf::from("prompts/test.md"),
            error: Box::new(shell),
        }
    }

    #[test]
    fn shell_expansion_failed_status_block_delegates_to_shell_error() {
        use biscuit_terminal::prelude::TerminalRenderable;

        let err = shell_expansion_failed_err();
        let term = Terminal::new_optimistic(80);
        let rendered = err.status_block(&term).render(&term);
        assert!(
            rendered.contains("ShellExpansionError"),
            "expected delegated shell-expansion header: {rendered}"
        );
        assert!(
            !rendered.contains("CompositionError"),
            "must not use the generic composition header: {rendered}"
        );
    }

    #[test]
    fn shell_expansion_failed_preserves_rich_diagnostic() {
        use biscuit_terminal::utils::escape_codes::strip_escape_codes;

        let err = shell_expansion_failed_err();
        let term = Terminal::new_optimistic(80);
        let rendered = strip_escape_codes(err.report_block_error(&term));

        assert!(
            rendered.contains("line 6"),
            "expected file-relative line in diagnostic: {rendered}"
        );
        assert!(
            rendered.contains("this command failed"),
            "expected stderr text in diagnostic: {rendered}"
        );
        assert!(
            rendered.contains("::shell"),
            "expected source excerpt in diagnostic: {rendered}"
        );
        assert!(
            rendered.contains("cmd-that-fails"),
            "expected command name in diagnostic: {rendered}"
        );
    }

    #[test]
    fn shell_expansion_failed_plain_terminal_has_no_escape_bytes() {
        let mut term = Terminal::builder()
            .width(80)
            .color_depth(biscuit_terminal::discovery::detection::ColorDepth::None)
            .build();
        term.is_nerd_font = Some(false);

        let err = shell_expansion_failed_err();
        let rendered = err.report_block_error(&term);

        assert!(
            !rendered.contains('\x1b'),
            "plain render must contain no escape byte; got: {rendered:?}"
        );
        assert!(rendered.contains("line 6"), "got: {rendered}");
        assert!(rendered.contains("this command failed"), "got: {rendered}");
        assert!(rendered.contains("::shell"), "got: {rendered}");
    }

    /// Exercise the full Markdown → `map_compose_error` → `report_block_error`
    /// path with a real failing `::shell` directive.
    ///
    /// This complements the hand-built `shell_expansion_failed_err` tests by
    /// proving that a captured `ExecutionFailed` from an actual subprocess
    /// survives through `prepare_direct` and renders with file-relative line
    /// numbers, the command's stderr, a source excerpt, and the composed
    /// frontmatter block.
    #[test]
    fn shell_expansion_failed_via_real_markdown_preserves_rich_diagnostic() {
        use std::collections::{BTreeMap, HashSet};

        use biscuit_terminal::terminal::Terminal;
        use biscuit_terminal::utils::escape_codes::strip_escape_codes;

        use super::super::prepare::{PrepareOptions, prepare_direct};
        use super::super::resolve::resolve_composition_source;

        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        let content = "---\ntitle: Shell demo\n---\n\nPre.\n\n::shell rustc --edition=invalid\n\nPost.\n";
        std::fs::write(&file_path, content).unwrap();

        let source = resolve_composition_source(file_path.to_str().unwrap()).unwrap();

        let mut approved = HashSet::new();
        approved.insert("rustc --edition=invalid".to_string());
        let options = PrepareOptions {
            set_overrides: None,
            pre_approved_commands: Some(approved),
            env_overrides: BTreeMap::new(),
            perf_enabled: false,
            source_repo_root: None,
            shell_working_directory: None,
        };

        let err = prepare_direct(&source, options).unwrap_err();
        let term = Terminal::new_optimistic(80);
        let rendered = strip_escape_codes(err.report_block_error(&term));

        assert!(
            rendered.contains("line 7"),
            "expected file-relative line in diagnostic: {rendered}"
        );
        assert!(
            rendered.contains("error:"),
            "expected captured rustc stderr text in diagnostic: {rendered}"
        );
        assert!(
            rendered.contains("::shell"),
            "expected source excerpt in diagnostic: {rendered}"
        );
        assert!(
            rendered.contains("title:") || rendered.contains("---"),
            "expected frontmatter block in diagnostic: {rendered}"
        );
    }
}
