//! Composition-specific error types.

use std::ops::Range;
use std::path::{Path, PathBuf};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use chrono::{DateTime, Utc};
use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::compose::shell_expansion::ShellExpansionError;

use darkmatter::markdown::compose::expression::file_suggestions::DEFAULT_MAX_SUGGESTIONS;
use darkmatter::markdown::compose::expression::{
    ExpressionError, FileRefFailure, FileReferenceDiagnostic, suggest_sibling_files,
};
use serde_json::{Value, json};

use super::frontmatter_excerpt::FrontmatterExcerpt;
use super::types::{ResolutionMode, ResolvedCompositionSource, SessionInteractivitySource};
use crate::diagnostics::{Category, Diagnostic, Disposition, Origin, code_spec, null_detail_for};
use crate::provider::Provider;
use thiserror::Error;

#[allow(missing_docs)]
mod render;
use render::pointer_to_dotted;
#[cfg(test)]
use render::render_agent_resolution_failed_body;

/// Exit code emitted when a loop run halts because a provider rate-limit
/// signal was observed and the configured policy was `abort` (or pause was
/// unsafe due to a missing `reset_at`).
///
/// Mirrors `EX_TEMPFAIL` (`75`) from BSD `sysexits.h` so shell wrappers can
/// distinguish a transient rate-limit halt from a generic non-zero exit
/// (typically `1`).
pub const LOOP_RATE_LIMITED_EXIT_CODE: i32 = 75;

pub(crate) fn indexed_property(property: &str, index: usize) -> String {
    format!("{property}[{index}]")
}

/// Errors that can occur during composition workflows.
#[derive(Error, Debug)]
pub enum CompositionError {
    /// The file reference string could not be parsed.
    #[error("invalid file reference `{reference}`: {source}")]
    InvalidReference {
        /// The raw reference string that failed to resolve.
        reference: String,
        /// The typed resolution failure from `biscuit-file`.
        #[source]
        source: biscuit_file::FileReferenceError,
    },

    /// The resolved file does not exist.
    #[error("file not found: {0}")]
    FileNotFound(String),

    /// The file is not a Markdown document.
    #[error("not a Markdown file (expected .md or .markdown): {0}")]
    NotMarkdown(String),

    /// The Markdown file could not be loaded or parsed.
    #[error("failed to load Markdown: {}: {source}", path.display())]
    MarkdownLoad {
        /// The file whose load/parse failed.
        path: PathBuf,
        /// The typed lower-layer cause.
        #[source]
        source: MarkdownLoadCause,
    },

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
    ///
    /// Carries the file path and the typed [`crate::error::ClaudineError`]
    /// source (boxed because `ClaudineError` can itself contain a
    /// `CompositionError`, which would otherwise make the type infinitely
    /// sized) so the path and underlying cause reach the CLI walker instead of
    /// being flattened to a string.
    #[error("atomic write to {path} failed: {source}")]
    AtomicWriteFailed {
        /// The file whose atomic write failed.
        path: PathBuf,
        /// The underlying typed write failure.
        #[source]
        source: Box<crate::error::ClaudineError>,
    },

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

    /// A lifecycle action verb is not recognized at parse time.
    ///
    /// Raised for single-key positional objects whose only key is not a known
    /// lifecycle verb.
    #[error(
        "unknown lifecycle action `{verb}` in `{property}` ({source_path}){rewrite}",
        source_path = source_path.display()
    )]
    LifecycleUnknownVerb {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// The unrecognized verb.
        verb: String,
        /// Did-you-mean rewrite, empty when no suggestion is available.
        rewrite: String,
    },

    /// A lifecycle stack item is ambiguous between positional and key/value
    /// forms because it is a multi-key object without an explicit `action:` key.
    #[error(
        "ambiguous lifecycle stack item in `{property}` ({source_path}): {message}",
        source_path = source_path.display()
    )]
    LifecycleStackAmbiguous {
        /// The prompt file whose lifecycle frontmatter held the stack item.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// Human-readable description with did-you-mean rewrites.
        message: String,
    },

    /// A positional lifecycle action received a direct YAML map value.
    ///
    /// Object-valued side-effect arguments must be passed through a whole-value
    /// `{{ … }}` interpolation span; direct nested YAML objects are rejected.
    #[error(
        "lifecycle action `{verb}` in `{property}` received an object value where a scalar or array is expected; \
         pass object data through a whole-value `{{{{ ... }}}}` interpolation ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleObjectDataThroughInterpolationPositional {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// The action verb.
        verb: String,
    },

    /// A key/value lifecycle action parameter received a direct YAML map value.
    ///
    /// Object-valued side-effect arguments must be passed through a whole-value
    /// `{{ … }}` interpolation span; direct nested YAML objects are rejected.
    #[error(
        "lifecycle action `{verb}` parameter `{param}` in `{property}` received an object value where a scalar is expected; \
         pass object data through a whole-value `{{{{ ... }}}}` interpolation ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleObjectDataThroughInterpolationParameter {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// The action verb.
        verb: String,
        /// The parameter name.
        param: String,
    },

    /// A positional lifecycle action received the wrong number of arguments.
    #[error(
        "lifecycle action `{verb}` in `{property}` has the wrong arity ({source_path}): {message}",
        source_path = source_path.display()
    )]
    LifecycleWrongArity {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// The action verb.
        verb: String,
        /// Human-readable arity description.
        message: String,
    },

    /// The short-form lifecycle action grammar `verb(args)` has been removed.
    ///
    /// Authors must use positional (`verb: value`) or key/value
    /// (`{ action: verb, ... }`) form. The `rewrite` field carries the
    /// positional equivalent for the did-you-mean message.
    #[error(
        "short-form lifecycle action `{raw}` in `{property}` has been removed ({source_path}).\n\n\
         Use the positional or key/value form instead: {rewrite}",
        source_path = source_path.display()
    )]
    LifecycleShortFormRemoved {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// The raw short-form string exactly as authored.
        raw: String,
        /// Positional-form did-you-mean rewrite.
        rewrite: String,
    },

    /// A variadic expression-function action was authored in key/value form.
    ///
    /// Variadic functions (`and`, `or`) accept a variable number of positional
    /// arguments and have no deterministic named-parameter mapping, so they
    /// reject the `{ action: verb, ... }` shape. Authors must use the positional
    /// array form (`and: [a, b, c]`).
    #[error(
        "lifecycle expression function `{verb}` in `{property}` does not support key/value form; \
         use the positional array form (`{verb}: [...]`) ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleExpressionFunctionKeyValueUnsupported {
        /// The prompt file whose frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// The variadic expression-function verb.
        verb: String,
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

    /// A lifecycle shell command failed pre-flight resolution via DM2 (C3).
    ///
    /// Shell commands live inside the deferred lifecycle subtree, so they are
    /// unresolved after main compose. Pre-flight resolves each one via DM2
    /// with an early-binding-only lookup (`doc.*`, `ctx.*`, `env.*`, read-side
    /// functions) and stamps the resolved bytes back so the approved command
    /// equals the executed command. This variant covers every failure DM2
    /// surfaces: malformed expressions, unknown roots (typos), unknown
    /// functions, and late-binding references (`err`/`timing`/`current`),
    /// which are rejected because shell commands are approved at pre-flight
    /// time, before any event fires.
    #[error(
        "lifecycle shell command at `{property}` failed pre-flight resolution: {message} \
         (raw: `{raw}`) ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleShellResolution {
        /// The prompt file whose lifecycle frontmatter held the command.
        source_path: PathBuf,
        /// Dotted lifecycle key path, e.g. `"start.stack[0].action.command"`.
        property: String,
        /// The raw command string exactly as authored (pre-resolution).
        raw: String,
        /// Human-readable resolution failure (parse error, unknown root,
        /// late-binding reference, etc.).
        message: String,
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

    /// A late-binding lifecycle expression *raised* at event time — a `when:`
    /// guard, a top-level communication string, or an action-value
    /// interpolation that threw (an unknown root under DM2 strict mode, a
    /// malformed `{{ … }}` span, or a read-side function error).
    ///
    /// Unlike a side-effect **dispatch** failure (which honors `no_error: true`
    /// and the per-phase routing policy), an evaluation error halts on **every**
    /// event phase and is surfaced to the user. A terminal-phase event
    /// (`success`/`failure`/`finalize`/`loop`) produces this and exits non-zero
    /// without retroactively firing `failure`; a setup-phase event
    /// (`initialize`/`start`/`blocked`) routes it through `failure`/`finalize`
    /// like any other setup failure.
    #[error(
        "lifecycle `{event}` evaluation error in `{surface}`: {message} ({source_path})",
        source_path = source_path.display()
    )]
    LifecycleEvaluationError {
        /// The prompt file whose lifecycle event raised the error.
        source_path: PathBuf,
        /// The lifecycle event name (e.g. `success`, `finalize`, `loop`).
        event: String,
        /// The offending surface — `when`, `interpolation`, or an action verb
        /// (taken from the raised [`super::lifecycle_context::LifecycleErrorInfo::variant`]).
        surface: String,
        /// The raised expression's message.
        message: String,
    },

    // -- Sequence errors -------------------------------------------------------
    /// The `sequence` frontmatter value is not a valid type (must be a list or a string).
    #[error("invalid sequence definition: {0}")]
    SequenceInvalid(String),

    /// The sequence list is empty.
    #[error("sequence list is empty; at least one step is required")]
    SequenceEmpty,

    /// The external YAML file could not be loaded.
    #[error("failed to load external sequence file: {context}: {source}")]
    SequenceExternalLoad {
        /// Display context: the raw reference (formatted as `` `raw` ``) for the
        /// reference-resolution sites, or the resolved YAML path for the load
        /// sites.
        context: String,
        /// The typed lower-layer cause.
        #[source]
        source: SequenceLoadCause,
    },

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
    /// A `$schema` reference could not be *resolved* — a missing file, an
    /// unsupported `http://` / `https://` URL, or a referenced file that is
    /// neither a SimplifiedSchema nor a JSON Schema.
    ///
    /// This is strictly a reference-resolution failure. A *syntax* error in the
    /// schema body itself is [`SchemaParse`], whose remediation is about
    /// constraint grammar rather than file paths.
    ///
    /// [`SchemaParse`]: CompositionError::SchemaParse
    #[error("schema load failed for {}: {message}", source_path.display())]
    SchemaLoad {
        /// The prompt file whose `$schema` reference failed to load.
        source_path: PathBuf,
        /// Human-readable description of the failure.
        message: String,
    },

    /// A `$schema` declaration could not be *parsed* or lowered to a JSON Schema.
    ///
    /// Distinct from [`SchemaLoad`] (reference resolution): this is a syntax
    /// error in the inline SimplifiedSchema body — a malformed type-and-constraint
    /// expression, an invalid constraint conversion, or an unsupported `$schema`
    /// shape. The remediation names the constraint grammar, not the file path,
    /// because the path is fine and the body is wrong.
    ///
    /// [`SchemaLoad`]: CompositionError::SchemaLoad
    #[error("schema parse failed for {}: {message}", source_path.display())]
    SchemaParse {
        /// The prompt file whose `$schema` body failed to parse.
        source_path: PathBuf,
        /// The schema property the failure is attributed to, when the typed
        /// cause identifies one (`None` for whole-shape failures).
        property: Option<String>,
        /// The grammar / conversion error message from the schema subsystem.
        message: String,
        /// Byte span of the offending token within the type-and-constraint
        /// string, when the typed cause carries one. Retained for the focused
        /// excerpt; `None` for conversion / shape failures that have no span.
        span: Option<Range<usize>>,
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

    /// A **provided** `file`/`file[]` value with non-empty `match(...)`
    /// patterns did not resolve to an existing path.
    ///
    /// Distinct from [`Self::MissingProperties`] (the value was *absent*) and
    /// from the generic [`Self::SchemaValidation`] (a wrong-type value): the
    /// user supplied a value that is best interpreted as a **partial** — a
    /// substring to match against the property's `match(...)` glob candidates.
    /// The CLI catches this variant and, when interactive, offers a
    /// confirmation dialog (single glob+substring match) or chooser (multiple).
    /// Zero matches / non-interactive re-surface this error unchanged, whose
    /// `reason` preserves the original file-reference failure text.
    #[error(
        "unresolved file reference for `{property}` in {}: {reason}",
        source_path.display()
    )]
    UnresolvedFileReference {
        /// The prompt file whose schema declares the property.
        source_path: PathBuf,
        /// Property name as declared in the schema.
        property: String,
        /// The value the user provided (frontmatter or `key=value`/`--set`),
        /// used as a case-insensitive path substring against the candidates.
        provided: String,
        /// Glob patterns from the property's `match(...)` constraint. Always
        /// non-empty for this variant (a bare `file` has no glob to walk).
        patterns: Vec<String>,
        /// `true` when the property is declared `file[]`.
        is_array: bool,
        /// The original file-reference failure message (e.g. `no existing file
        /// matched reference …`), preserved so the non-interactive / zero-match
        /// fall-through shows the same actionable text as before.
        reason: String,
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

    // -- Autocomplete errors --------------------------------------------------
    /// No files matched the autocomplete query.
    #[error("no files matched autocomplete query `{query}`")]
    AutocompleteNoMatches {
        /// The user's typed query token.
        query: String,
    },

    /// Too many files matched the autocomplete query; the user must narrow it.
    #[error(
        "more than {cap} files matched autocomplete query `{query}`; narrow your query"
    )]
    AutocompleteOverCap {
        /// The user's typed query token.
        query: String,
        /// The candidate cap that was exceeded.
        cap: usize,
    },

    /// Autocomplete requires an interactive terminal.
    #[error("autocomplete requires an interactive terminal")]
    AutocompleteNotInteractive,

    /// The user cancelled the autocomplete dialog or chooser.
    #[error("autocomplete cancelled for query `{query}`")]
    AutocompleteCancelled {
        /// The user's typed query token.
        query: String,
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

    /// A lifecycle evaluation error whose styled block was already rendered to
    /// stderr at its catch point (Decision #2), before any catch events
    /// (`failure`/`finalize`) fired.
    ///
    /// Constructed by [`already_emitted`](CompositionError::already_emitted)
    /// once the early emit has happened, so the outer CLI renderer recognizes
    /// the error has been surfaced and suppresses the duplicate styled block —
    /// while still propagating a non-zero exit. `Display` delegates to `inner`
    /// so any plain-text fallback keeps the original message.
    #[error("{inner}")]
    LifecycleEvaluationAlreadyEmitted {
        /// The original evaluation error (a
        /// [`Self::LifecycleEvaluationError`]).
        inner: Box<CompositionError>,
    },
}

/// Heterogeneous lower-layer cause of a Markdown source-load failure.
///
/// Carried as the typed `#[source]` of [`CompositionError::MarkdownLoad`] so a
/// programmatic handler can recover the concrete lower-layer type via
/// [`std::error::Error::source`] instead of inspecting a flattened string.
#[derive(Error, Debug)]
pub enum MarkdownLoadCause {
    /// The file could not be read.
    #[error(transparent)]
    Read(#[from] std::io::Error),
    /// A non-frontmatter Markdown parse failure.
    ///
    /// Boxed to keep `MarkdownLoadCause` (and thus the `MarkdownLoad`
    /// `Err` variant) small — `MarkdownError` is the heavy member here,
    /// mirroring how `MarkdownError` itself boxes its own large children.
    #[error(transparent)]
    Parse(#[from] Box<MarkdownError>),
    /// A YAML load/convert failure (CLI sequence YAML path).
    #[error(transparent)]
    Yaml(#[from] biscuit_file::YamlError),
}

/// Heterogeneous lower-layer cause of an external sequence-file load failure.
///
/// Carried as the typed `#[source]` of
/// [`CompositionError::SequenceExternalLoad`] so a programmatic handler can
/// recover the concrete lower-layer type via [`std::error::Error::source`]
/// instead of inspecting a flattened string.
#[derive(Error, Debug)]
pub enum SequenceLoadCause {
    /// A file-reference resolution failure.
    #[error(transparent)]
    Reference(#[from] biscuit_file::FileReferenceError),
    /// A YAML load/convert failure.
    #[error(transparent)]
    Yaml(#[from] biscuit_file::YamlError),
    /// The reference resolved but no file exists at the resolved path.
    #[error("file not found")]
    NotFound,
    /// `~` expansion failed because no home directory is known.
    #[error("unable to resolve home directory")]
    HomeDir,
}

/// A single required schema property that is missing from frontmatter.
///
/// Carries enough metadata to render an actionable error (declaration name,
/// type label, optional description) without holding a Darkmatter schema
/// reference. Constructed by the validation layer after consulting the
/// effective SimplifiedSchema.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub enum InteractiveShape {
    /// Plain text input (string, date, datetime, time, url, email).
    Text {
        /// Hint about the expected text format. Used for placeholder /
        /// help text only — Darkmatter handles validation post-submission.
        format: TextFormat,
        /// Optional minimum string length (Unicode code points) from the
        /// SimplifiedSchema `min` constraint.
        min_len: Option<usize>,
        /// Optional maximum string length (Unicode code points) from the
        /// SimplifiedSchema `max` constraint.
        max_len: Option<usize>,
    },
    /// Numeric input with parse-and-retry validation.
    Number {
        /// `true` when the property is constrained to integer values
        /// (via the SimplifiedSchema `integer` constraint).
        integer: bool,
        /// Optional inclusive minimum from the SimplifiedSchema `min`
        /// constraint.
        min: Option<f64>,
        /// Optional inclusive maximum from the SimplifiedSchema `max`
        /// constraint.
        max: Option<f64>,
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
    /// File reference chooser.
    File {
        /// `true` when the property is declared as `file[]`.
        is_array: bool,
        /// Glob patterns from `match(...)`, empty for a bare `file`/`file[]`
        /// property (uses the default glob).
        patterns: Vec<String>,
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
#[derive(Debug, Clone, PartialEq)]
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
    /// Build a [`Self::LifecycleEvaluationError`] from the raised lifecycle
    /// error snapshot.
    ///
    /// `event` is the lifecycle event whose stack raised (e.g. `success`); the
    /// offending surface and message are lifted from `info` so a single
    /// snapshot constructed at the executor layer renders consistently
    /// regardless of which orchestrator caught it.
    pub fn lifecycle_evaluation(
        event: impl Into<String>,
        source_path: impl Into<PathBuf>,
        info: &super::lifecycle_context::LifecycleErrorInfo,
    ) -> Self {
        Self::LifecycleEvaluationError {
            source_path: source_path.into(),
            event: event.into(),
            surface: info.variant.clone(),
            message: info.msg.clone(),
        }
    }

    /// Surface the evaluation error that halts the run after the catch events
    /// (`failure` and/or `finalize`) ran for an earlier raise.
    ///
    /// Precedence: a raise inside `finalize` beats a raise inside `failure`
    /// beats the original error — the user sees the *latest* lifecycle crash,
    /// not the one that triggered the catch. The callers are responsible for
    /// threading the active error into `finalize` (a `failure` raise becomes
    /// the `err` carried into `finalize`); this helper only decides which
    /// outcome surfaces.
    ///
    /// `failure_outcome`/`finalize_outcome` are `None` when the corresponding
    /// catch event did not run (e.g. a terminal-phase catch skips `failure`).
    pub fn catch_evaluation_error(
        source_path: &Path,
        original_event: &str,
        original_info: &super::lifecycle_context::LifecycleErrorInfo,
        failure_outcome: Option<&super::lifecycle_executor::LifecycleEventOutcome>,
        finalize_outcome: Option<&super::lifecycle_executor::LifecycleEventOutcome>,
    ) -> Self {
        if let Some(fin) = finalize_outcome
            && let Some(fin_info) = fin.evaluation_error.as_ref()
        {
            return Self::lifecycle_evaluation("finalize", source_path, fin_info);
        }
        if let Some(fail) = failure_outcome
            && let Some(fail_info) = fail.evaluation_error.as_ref()
        {
            return Self::lifecycle_evaluation("failure", source_path, fail_info);
        }
        Self::lifecycle_evaluation(original_event, source_path, original_info)
    }

    /// Mark this evaluation error as already styled-emitted to stderr at its
    /// catch point (Decision #2).
    ///
    /// Wraps `self` in [`Self::LifecycleEvaluationAlreadyEmitted`] so the outer
    /// CLI renderer suppresses the duplicate styled block while preserving the
    /// non-zero exit. Idempotent: an already-marked error is returned as-is.
    /// `self` is expected to be a [`Self::LifecycleEvaluationError`]; any other
    /// variant is wrapped unchanged (suppression is keyed on the wrapper, not
    /// the inner shape).
    pub fn already_emitted(self) -> Self {
        if matches!(self, CompositionError::LifecycleEvaluationAlreadyEmitted { .. }) {
            return self;
        }
        CompositionError::LifecycleEvaluationAlreadyEmitted {
            inner: Box::new(self),
        }
    }

    /// Whether this error was already styled-emitted at its catch point, so the
    /// outer renderer must not re-render it.
    pub fn is_already_emitted(&self) -> bool {
        matches!(self, CompositionError::LifecycleEvaluationAlreadyEmitted { .. })
    }

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
        self.enrich_frontmatter_text(&source.original_text, stderr_is_tty)
    }

    /// Attach a captured frontmatter excerpt from raw source text.
    ///
    /// This is used for source-load failures where parsing failed before a
    /// [`ResolvedCompositionSource`] could be built.
    pub fn enrich_frontmatter_text(self, source_text: &str, stderr_is_tty: bool) -> Self {
        if matches!(self, CompositionError::WithFrontmatter { .. }) {
            return self;
        }
        let Some(spec) = self.frontmatter_block_spec() else {
            return self;
        };
        let excerpt = match spec {
            FrontmatterHighlight::Line(line) => {
                FrontmatterExcerpt::capture_line(source_text, line, stderr_is_tty)
            }
            FrontmatterHighlight::Property(property) => {
                FrontmatterExcerpt::capture(source_text, Some(&property), stderr_is_tty)
            }
            FrontmatterHighlight::SchemaSpan {
                property,
                span_start,
            } => FrontmatterExcerpt::capture_schema_span(
                source_text,
                property.as_deref(),
                span_start,
                stderr_is_tty,
            ),
            FrontmatterHighlight::BlockOnly => {
                FrontmatterExcerpt::capture(source_text, None, stderr_is_tty)
            }
        };
        match excerpt {
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

    /// How a frontmatter-rooted error should be excerpted.
    fn frontmatter_block_spec(&self) -> Option<FrontmatterHighlight> {
        match self {
            CompositionError::FrontmatterParse(md_err) => match md_err {
                MarkdownError::FrontmatterFenceMismatch { line, .. } => {
                    Some(FrontmatterHighlight::Line(*line))
                }
                _ => Some(FrontmatterHighlight::BlockOnly),
            },
            CompositionError::LifecycleInterpolationLeak { property, .. }
            | CompositionError::LifecycleUndefinedVariable { property, .. }
            | CompositionError::LifecycleInvalid { property, .. }
            | CompositionError::LifecycleStackInvalidShape { property, .. }
            | CompositionError::LifecycleActionInvalidShortForm { property, .. }
            | CompositionError::LifecycleActionInvalidLongForm { property, .. }
            | CompositionError::LifecycleUnknownVerb { property, .. }
            | CompositionError::LifecycleStackAmbiguous { property, .. }
            | CompositionError::LifecycleObjectDataThroughInterpolationPositional { property, .. }
            | CompositionError::LifecycleObjectDataThroughInterpolationParameter { property, .. }
            | CompositionError::LifecycleWrongArity { property, .. }
            | CompositionError::LifecycleShortFormRemoved { property, .. }
            | CompositionError::LifecycleActionPlacement { property, .. }
            | CompositionError::LifecycleMultipleLifecycleActions { property, .. }
            | CompositionError::LifecycleActionOrder { property, .. }
            | CompositionError::LifecycleInvalidArgs { property, .. }
            | CompositionError::LifecycleErrNotAvailable { property, .. } => {
                Some(FrontmatterHighlight::Property(property.clone()))
            }
            CompositionError::LifecycleSayConflict(property)
            | CompositionError::LifecycleUnknownEffect(property, _) => {
                Some(FrontmatterHighlight::Property(property.clone()))
            }
            CompositionError::RemovedValidationKey { key, .. } => {
                Some(FrontmatterHighlight::Property(key.clone()))
            }
            CompositionError::PromptPropertyMissing
            | CompositionError::PromptPropertyWrongType(_) => {
                Some(FrontmatterHighlight::Property("prompt".to_string()))
            }
            CompositionError::AgentHintWrongType(_)
            | CompositionError::AgentResolutionFailed { .. } => {
                Some(FrontmatterHighlight::Property("agent".to_string()))
            }
            CompositionError::ModelHintWrongType(_) => {
                Some(FrontmatterHighlight::Property("model".to_string()))
            }
            CompositionError::InteractiveHintWrongType(_) => {
                Some(FrontmatterHighlight::Property("interactive".to_string()))
            }
            CompositionError::SchemaLoad { .. } => {
                Some(FrontmatterHighlight::Property("$schema".to_string()))
            }
            CompositionError::SchemaParse {
                property, span, ..
            } => match span {
                // A grammar `span` lets the excerpt land on the exact line of a
                // multi-line schema value; the property name scopes it to the
                // right `$schema.<prop>` entry (or the `$schema` parent when the
                // failure is structural and carries no real property name).
                Some(range) => Some(FrontmatterHighlight::SchemaSpan {
                    property: property
                        .as_deref()
                        .map(|prop| format!("$schema.{prop}")),
                    span_start: range.start,
                }),
                // Convert / shape failures carry no span; highlight the property
                // line when named, else the `$schema` parent.
                None => match property {
                    Some(prop) => Some(FrontmatterHighlight::Property(format!("$schema.{prop}"))),
                    None => Some(FrontmatterHighlight::Property("$schema".to_string())),
                },
            },
            CompositionError::InlineHashMalformed(_) => {
                Some(FrontmatterHighlight::Property("hash".to_string()))
            }
            CompositionError::UnsupportedInteractiveSchema { property, .. } => {
                Some(FrontmatterHighlight::Property(property.clone()))
            }
            CompositionError::MissingProperties {
                missing,
                pointer_paths,
                ..
            } => match (missing.split_first(), pointer_paths.split_first()) {
                (Some((only, [])), _) => Some(FrontmatterHighlight::Property(only.name.clone())),
                (_, Some((only, []))) => {
                    Some(FrontmatterHighlight::Property(pointer_to_dotted(only)))
                }
                _ => Some(FrontmatterHighlight::BlockOnly),
            },
            CompositionError::SchemaValidation { problems, .. } => match problems.split_first() {
                Some((only, [])) => Some(FrontmatterHighlight::Property(pointer_to_dotted(only))),
                _ => Some(FrontmatterHighlight::BlockOnly),
            },
            CompositionError::UnresolvedFileReference { property, .. } => {
                Some(FrontmatterHighlight::Property(property.clone()))
            }
            // A whole-value frontmatter interpolation failure names its receiving
            // key — focus the excerpt on that line rather than dumping the whole
            // block. Body interpolation (key `None`) falls through to BlockOnly.
            CompositionError::ComposeFailed(MarkdownError::Interpolation {
                key: Some(key),
                ..
            }) => Some(FrontmatterHighlight::Property(key.clone())),
            CompositionError::InlineComposeSequenceMismatch { .. }
            | CompositionError::ComposeFailed(_)
            | CompositionError::ShellExpansionFailed { .. } => Some(FrontmatterHighlight::BlockOnly),
            _ => None,
        }
    }
}

/// How a frontmatter-rooted error should be highlighted in the captured excerpt.
enum FrontmatterHighlight {
    /// Highlight a dotted frontmatter property key.
    Property(String),
    /// Highlight a 1-based document line (used for delimiter-level errors).
    Line(usize),
    /// Highlight a schema property whose type-and-constraint string failed to
    /// parse, using the grammar `span` to land on the right line of a multi-line
    /// value. `property` is the dotted key (`None` for whole-shape failures);
    /// `span_start` is the byte offset into the property's type string.
    SchemaSpan {
        property: Option<String>,
        span_start: usize,
    },
    /// Show the frontmatter block with no line highlighted.
    BlockOnly,
}

#[cfg(test)]
mod tests;
