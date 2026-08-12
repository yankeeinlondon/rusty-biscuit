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
use darkmatter::markdown::compose::context::merge::CtxMergeError;
use darkmatter::markdown::compose::shell_expansion::ShellExpansionError;

use darkmatter::markdown::compose::expression::file_suggestions::DEFAULT_MAX_SUGGESTIONS;
use darkmatter::markdown::compose::expression::{
    ExpressionError, FileRefFailure, FileReferenceDiagnostic, ParseError, suggest_sibling_files,
};
use serde_json::{Value, json};

use super::frontmatter_excerpt::FrontmatterExcerpt;
use super::sequence::task::RunawayTrip;
use super::types::{ResolutionMode, ResolvedCompositionSource, SessionInteractivitySource};
use crate::diagnostics::{
    Category, Diagnostic, DiagnosticRole, DiagnosticSnapshot, Disposition, Origin, code_spec,
    null_detail_for,
};
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

/// Why a shell command could not be approved at pre-flight.
///
/// The `reason` facet of `composition.shell_approval`. Deliberately a closed
/// enum rather than the prose it replaced: an author who wants to react to a
/// blacklist hit but not to a missing approval handler needs a value to match,
/// and re-deriving one by parsing `Display` is what this feature exists to stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellApprovalFailure {
    /// The command matched Claudine's blacklist; carries the catalog's reason.
    Blacklisted(String),
    /// Approval was required, but no approval handler was available — a
    /// non-interactive run with an unwhitelisted command.
    NoHandler,
    /// `--dry-run` cannot obtain the interactive approval the command needs.
    DryRun,
}

impl ShellApprovalFailure {
    /// The stable snake_case slug projected to `err.detail.reason`.
    pub fn as_str(&self) -> &'static str {
        match self {
            ShellApprovalFailure::Blacklisted(_) => "blacklisted",
            ShellApprovalFailure::NoHandler => "no_handler",
            ShellApprovalFailure::DryRun => "dry_run",
        }
    }

    /// The human-facing message, byte-identical to the prose each of these
    /// failures carried inside `PreFlightFailed(String)` before the approval
    /// family was typed — the text is a user-visible surface, and typing the
    /// error is not a licence to reword it.
    fn message(&self, command: &str, source_file: &Path, line: usize) -> String {
        let location = if line > 0 {
            format!("{}:{line}", biscuit_file::to_portable_string(source_file))
        } else {
            biscuit_file::to_portable_string(source_file)
        };
        match self {
            ShellApprovalFailure::Blacklisted(reason) => {
                format!("Shell command '{command}' at {location} is blacklisted: {reason}")
            }
            ShellApprovalFailure::NoHandler => format!(
                "Shell command '{command}' at {location} requires approval but no approval \
                 handler is available. Add to whitelist or run interactively."
            ),
            ShellApprovalFailure::DryRun => format!(
                "Cannot dry-run: shell command '{command}' requires interactive approval. \
                 Run with --yolo to auto-approve, or pre-approve the command in your \
                 configuration."
            ),
        }
    }
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

    /// A file reference the author wrote in their prompt document could not be
    /// resolved to a usable file.
    ///
    /// The semantic boundary between a lower-layer resolution failure and the
    /// authoring surface that asked for it: the typed [`source`] says *why*
    /// resolution failed, and this wrapper says *which authored value* asked,
    /// *where* it was written, and *which lifecycle event* was running.
    ///
    /// It owns `composition.invalid_file_reference` for **every** such surface
    /// — proxying, expressions, schemas, transclusion. A surface is told apart
    /// by the `event` and `property` keys in structured detail, never by a
    /// surface-specific code, so one `when: err.code == "…"` clause keeps
    /// matching as new surfaces adopt it.
    ///
    #[error("{}: cannot resolve `{}`: {source}", context.property, context.reference)]
    InvalidFileReference {
        /// Where the reference was authored. Boxed so the context's five
        /// fields do not widen every `Result<_, CompositionError>` in the
        /// crate — the *context* is the safe thing to box, not the `source`.
        context: Box<FileReferenceContext>,
        /// The typed resolution failure.
        ///
        /// Deliberately **not** boxed: `#[source]` on a `Box<ConcreteError>`
        /// publishes `Box<ConcreteError>` to the cause chain, and `Box`'s own
        /// `Error::source` skips straight to the inner error's *source* — so
        /// the concrete error would never be a chain member and
        /// [`as_diagnostic`](crate::diagnostics::as_diagnostic) could never
        /// downcast to it.
        #[source]
        source: crate::harness::HarnessError,
    },

    /// The resolved file does not exist.
    #[error("file not found: {0}")]
    FileNotFound(String),

    /// The file is not a Markdown document.
    #[error("not a Markdown file (expected .md or .markdown): {0}")]
    NotMarkdown(String),

    /// The Markdown file could not be loaded or parsed.
    #[error(
        "failed to load Markdown: {}: {source}",
        biscuit_file::to_portable_string(path)
    )]
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

    /// Stamping `last_updated` into the rebuilt inline document failed.
    ///
    /// The typed sibling of [`InvalidInlineResponse`] for the one inline-closure
    /// step that has a concrete cause in hand: `rewrite_inline_document`'s
    /// fallback path parses the source as Markdown and re-inserts
    /// `last_updated`, and `fm_insert` is its only fallible call. `Display` is
    /// byte-identical to the `InvalidInlineResponse(format!(…))` it replaced —
    /// the prose is a user-visible surface, and typing the error is not a
    /// licence to reword it.
    ///
    /// [`InvalidInlineResponse`]: CompositionError::InvalidInlineResponse
    #[error("invalid inline composition response: failed to update last_updated: {0}")]
    InlineRewriteFailed(#[source] MarkdownError),

    /// The document's existing `hash` frontmatter property is malformed.
    ///
    /// Carries the typed `MarkdownError` so the CLI's top-level walker renders
    /// Darkmatter's `MalformedStoredHash` block instead of a flat string.
    #[error("malformed stored `hash` property: {0}")]
    InlineHashMalformed(#[source] MarkdownError),

    /// Atomic file write failed during inline composition.
    ///
    /// Carries the file path and the unboxed [`std::io::Error`] that
    /// `atomic_write` raised, so both reach the CLI walker instead of being
    /// flattened to a string.
    ///
    /// The source was a `Box<ClaudineError>` until `atomic_write` was narrowed
    /// to `io::Result`. The box was mandatory then — `ClaudineError` can itself
    /// hold a `CompositionError`, so an unboxed field made the type infinitely
    /// sized — and it made the cause invisible to `as_diagnostic`, which is a
    /// downcast list over concrete types and cannot see through a `Box`. An
    /// `io::Error` source has neither problem.
    #[error("atomic write to {path} failed: {source}")]
    AtomicWriteFailed {
        /// The file whose atomic write failed.
        path: PathBuf,
        /// The underlying typed write failure.
        #[source]
        source: std::io::Error,
    },

    /// The composition target file lacks required read/write permissions.
    ///
    /// Carries the unboxed [`std::io::Error`] the read+write open probe raised,
    /// so the OS-level reason (`permission denied`, `is a directory`, a broken
    /// symlink) reaches a handler instead of only the flattened `Display` text.
    #[error(
        "insufficient file permissions (need read+write): {}: {source}",
        biscuit_file::to_portable_string(path)
    )]
    InsufficientFilePermissions {
        /// The file whose read+write probe failed.
        path: PathBuf,
        /// The typed OS failure the probe raised.
        #[source]
        source: std::io::Error,
    },

    /// Pre-flight shell command discovery failed.
    ///
    /// Carries the typed `MarkdownError` so the CLI's top-level walker can
    /// render a rich `BlockError` report (e.g. transclusion cycles or
    /// reference errors encountered while walking the document graph) instead
    /// of a flat string.
    #[error("pre-flight discovery failed: {0}")]
    PreFlightDiscoveryFailed(#[source] MarkdownError),

    /// A pre-flight failure carrying only prose.
    ///
    /// This variant is **not** the shell-approval surface: it carries prose, so
    /// it cannot claim `composition.shell_approval` without parsing its own
    /// `Display` to find out which failure it is. The approval failures have
    /// [`ShellApprovalUnavailable`] and [`ShellCommandDenied`] instead.
    ///
    /// Both of its former production constructors are now typed —
    /// [`PreFlightShellAuditFailed`] and [`PreFlightStateBuildFailed`] — so
    /// nothing in `lib/src` builds this variant today. It is retained as the
    /// enum's documented prose anchor: `error-architecture.md` cites it as the
    /// worked example of why a prose error must not claim a code, and its
    /// `composition.failed` mapping is pinned by
    /// `preflight_failed_does_not_claim_the_approval_code`.
    ///
    /// [`ShellApprovalUnavailable`]: CompositionError::ShellApprovalUnavailable
    /// [`ShellCommandDenied`]: CompositionError::ShellCommandDenied
    /// [`PreFlightShellAuditFailed`]: CompositionError::PreFlightShellAuditFailed
    /// [`PreFlightStateBuildFailed`]: CompositionError::PreFlightStateBuildFailed
    #[error("pre-flight shell approval failed: {0}")]
    PreFlightFailed(String),

    /// A shell-audit failure outside the approval family, raised while checking
    /// a discovered command against policy.
    ///
    /// Split out of [`PreFlightFailed`]'s prose so the [`HarnessError`] the
    /// audit raised stays a downcastable member of the cause chain. The two
    /// approval-family `HarnessError` variants are destructured before this arm
    /// and become [`ShellCommandDenied`] / [`ShellApprovalUnavailable`]; every
    /// other variant lands here.
    ///
    /// The source is deliberately **not** boxed, for the reason
    /// [`InvalidFileReference`] records: a `#[source]` on a `Box<HarnessError>`
    /// publishes the *box* to the chain, so
    /// [`as_diagnostic`](crate::diagnostics::as_diagnostic) — a downcast list
    /// over concrete types — could never resolve the `HarnessError` behind it.
    ///
    /// `Display` is byte-identical to the `PreFlightFailed(e.to_string())` it
    /// replaced, and the variant keeps the `composition.failed` catch-all:
    /// giving the now-typed failure a code of its own is an `err.code` change,
    /// which spec §D10 reserves for a separate specification.
    ///
    /// [`PreFlightFailed`]: CompositionError::PreFlightFailed
    /// [`HarnessError`]: crate::harness::HarnessError
    /// [`ShellCommandDenied`]: CompositionError::ShellCommandDenied
    /// [`ShellApprovalUnavailable`]: CompositionError::ShellApprovalUnavailable
    /// [`InvalidFileReference`]: CompositionError::InvalidFileReference
    #[error("pre-flight shell approval failed: {source}")]
    PreFlightShellAuditFailed {
        /// The typed shell-audit failure.
        #[source]
        source: crate::harness::HarnessError,
    },

    /// Building the early-binding state for lifecycle shell resolution failed.
    ///
    /// Split out of [`PreFlightFailed`]'s prose so the [`CtxMergeError`] stays a
    /// chain member. `Display` is byte-identical to the `format!` it replaced.
    ///
    /// [`PreFlightFailed`]: CompositionError::PreFlightFailed
    #[error(
        "pre-flight shell approval failed: lifecycle shell pre-flight: \
         building early-binding state failed: {source}"
    )]
    PreFlightStateBuildFailed {
        /// The typed `ctx` merge failure from Darkmatter's state builder.
        #[source]
        source: CtxMergeError,
    },

    /// A shell command the author wrote could not be approved at pre-flight,
    /// for a reason other than the user declining it.
    ///
    /// Split out of [`PreFlightFailed`]'s prose so the approval family can own
    /// `composition.shell_approval` and project `command` / `source_path` /
    /// `line` / `reason` as structured detail. `failure` is what tells a
    /// blacklist hit from a missing handler, so no surface needs its own code.
    ///
    /// [`PreFlightFailed`]: CompositionError::PreFlightFailed
    #[error("pre-flight shell approval failed: {}", failure.message(command, source_file, *line))]
    ShellApprovalUnavailable {
        /// The authored command that could not be approved.
        command: String,
        /// The document the command was written in.
        source_file: PathBuf,
        /// 1-based line of the command; `0` when the source carries no line.
        line: usize,
        /// Why approval could not be obtained.
        failure: ShellApprovalFailure,
    },

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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleStackInvalidShape {
        /// The prompt file whose lifecycle frontmatter held the malformed stack.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`, `"success.stack[2]"`).
        property: String,
        /// Human-readable description of the shape problem.
        message: String,
    },

    /// A lifecycle stack item's `when:` condition failed to parse.
    ///
    /// The typed sibling of [`LifecycleStackInvalidShape`] for the one shape
    /// site that holds a concrete [`ParseError`]; every other site rejects a
    /// value on its own JSON shape and has no typed error to retain. It shares
    /// that variant's `Display`, code, `detail`, and status block — `message`
    /// keeps the prose so all four render from one string — and adds only the
    /// recoverable source.
    ///
    /// Staged here for the lifecycle-execution burn-down batch, which owns
    /// `lifecycle/parse.rs`; see `features/2026-07-13-error-propogation/burndown-triage.md`.
    ///
    /// [`LifecycleStackInvalidShape`]: CompositionError::LifecycleStackInvalidShape
    #[error(
        "invalid lifecycle stack item in `{property}` ({source_path}): {message}",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleWhenExpressionInvalid {
        /// The prompt file whose lifecycle frontmatter held the condition.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// Human-readable description of the parse failure.
        message: String,
        /// The typed expression parse failure.
        #[source]
        source: ParseError,
    },

    /// A short-form lifecycle action `verb(args)` failed to parse.
    ///
    /// Covers: missing verb, unbalanced parens, trailing characters, argument
    /// splitter errors, and argument expression parse errors. The bare
    /// "unquoted multi-word literal" case (e.g. `say(using codex)`) is also
    /// reported through this variant with a `message` that names the cause.
    #[error(
        "invalid lifecycle action `{raw}` in `{property}` ({source_path}): {message}",
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        /// Typed cause when the failure came from converting an action value
        /// into an expression (`action_value_to_expr`). `None` for the
        /// shape failures — missing parameter, unknown key, wrong value type —
        /// that never had a lower cause. The enum is carried unboxed so
        /// `Error::source()` publishes [`ActionExprError`] directly rather than
        /// an undowncastable `Box` (decisions.md §D-7); the heavy [`ParseError`]
        /// is boxed *inside* the enum instead, keeping `CompositionError` under
        /// `clippy::result_large_err`.
        #[source]
        source: Option<ActionExprError>,
    },

    /// A lifecycle action verb is not recognized at parse time.
    ///
    /// Raised for single-key positional objects whose only key is not a known
    /// lifecycle verb.
    #[error(
        "unknown lifecycle action `{verb}` in `{property}` ({source_path}){rewrite}",
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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

    /// `proxy.with` was authored with something other than a YAML mapping.
    #[error(
        "`{property}.{path}` must be a mapping, got {actual} ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleProxyWithNotMapping {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`); the stack annotator upgrades
        /// this to `"{event}.stack[{i}]"`.
        property: String,
        /// The `with`-rooted path within the stack item (e.g.
        /// `"action[0].with"`). Joined to `property` for the rendered
        /// property path.
        path: String,
        /// The JSON type name of the authored value.
        actual: String,
    },

    /// `proxy.with` was supplied as a single whole-mapping interpolation
    /// (`with: "{{ payload }}"`) rather than explicit keys.
    ///
    /// Named out-of-scope for v1: authors write the mapping explicitly and may
    /// inject typed object or array values at individual keys.
    #[error(
        "`{property}.{path}` cannot be supplied as a whole-mapping interpolation `{raw}` ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleProxyWithWholeMapping {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name; upgraded to `"{event}.stack[{i}]"` by the stack
        /// annotator.
        property: String,
        /// The `with`-rooted path within the stack item.
        path: String,
        /// The authored string, verbatim.
        raw: String,
    },

    /// A `proxy.with` key carries an interpolation span. Keys name target
    /// frontmatter properties and are never interpolated.
    #[error(
        "`{property}.{path}` has a dynamic key `{key}`; `with:` keys must be static strings ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleProxyWithDynamicKey {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name; upgraded to `"{event}.stack[{i}]"` by the stack
        /// annotator.
        property: String,
        /// The `with`-rooted path within the stack item. It gains a trailing
        /// `.{key}` segment only when the key is a safe path segment — a
        /// dotted path built from an unrepresentable key would point at a
        /// property that does not exist.
        path: String,
        /// The offending key, verbatim.
        key: String,
    },

    /// A `proxy.with` value could not be resolved at event time.
    ///
    /// The message carries the underlying expression-layer reason. Neither it
    /// nor any other field echoes a resolved overlay value: an overlay may
    /// carry secrets, so a diagnostic names properties, not contents.
    #[error(
        "`{property}.{path}` could not be resolved for the proxy to `{target}`: {message} \
         ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleProxyWithEvaluationFailed {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// The `"{event}.stack[{i}]"` property that fired.
        property: String,
        /// The failing path within the stack item, rooted at the action and
        /// carried down to the exact nested value (e.g.
        /// `"action[0].with.metadata.area"`).
        path: String,
        /// The evaluated proxy target the overlay was being built for.
        target: String,
        /// The expression-layer reason.
        message: String,
    },

    /// A parameter that only `proxy` accepts was authored on another action.
    #[error(
        "lifecycle action `{verb}` in `{property}` does not accept a `{param}` parameter; \
         `{param}` is only valid on `proxy` ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleProxyOnlyParameter {
        /// The prompt file whose lifecycle frontmatter held the action.
        source_path: PathBuf,
        /// Owning event name (e.g. `"start"`).
        property: String,
        /// The action verb that received the parameter.
        verb: String,
        /// The proxy-only parameter name.
        param: String,
    },

    /// A positional lifecycle action received the wrong number of arguments.
    #[error(
        "lifecycle action `{verb}` in `{property}` has the wrong arity ({source_path}): {message}",
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        /// The typed DM2 failure, when one exists.
        ///
        /// `None` for the late-binding rejection, which is this layer's own
        /// pre-resolution guard and never calls Darkmatter — there is no typed
        /// error to retain. Boxed because `MarkdownError` is heavy and this
        /// variant already carries four fields; nothing downcasts to it, since
        /// a Darkmatter error is not a registered Claudine diagnostic.
        #[source]
        source: Option<Box<MarkdownError>>,
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
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleResumeWithoutSession {
        /// The prompt file whose stack requested resume.
        source_path: PathBuf,
    },

    /// A `resume` retained a live provider session, but a canonical refresh of
    /// the document changed a launch property the provider fixed when the
    /// session was opened (spec R8). Feeding the refreshed launch plan into the
    /// old session would run it under a stale contract, so the resume refuses.
    ///
    /// Raised at runtime, after the refresh and before any provider attempt:
    /// the session-compatibility key of the launch that opened the session no
    /// longer matches the key of the freshly prepared plan.
    #[error(
        "lifecycle `resume` cannot reuse the live session: a canonical refresh \
         changed incompatible launch propert{plural} ({facets}); retry instead \
         to start a fresh session ({source_path})",
        plural = if facets.len() == 1 { "y" } else { "ies" },
        facets = facets.join(", "),
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleResumeIncompatible {
        /// The prompt file whose stack requested resume.
        source_path: PathBuf,
        /// The launch facets that changed across the refresh, named for the
        /// operator.
        facets: Vec<String>,
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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

    /// A lifecycle stack returned a transition that is real and well-formed,
    /// but that no coordinator at this point in the run can consume.
    ///
    /// `retry`, `resume`, and `defer` all need a provider attempt to act on —
    /// there is nothing to retry before one has run. Reaching this means the
    /// authored control is valid everywhere (`is_valid_for` makes every control
    /// except `skip` universally placeable) but meaningless *here*, so the
    /// diagnostic names the stage rather than claiming the action is invalid.
    #[error(
        "lifecycle `{verb}` cannot be honored at `{stage}`: {reason} ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleTransitionUnownedAtStage {
        /// The document whose stack returned the transition.
        source_path: PathBuf,
        /// The dotted `"{event}.stack[{i}].action[{j}]"` location.
        property: String,
        /// The control verb, as authored (`retry`, `resume`, `defer`).
        verb: &'static str,
        /// The run stage that has no coordinator for it.
        stage: &'static str,
        /// Why this stage cannot own it.
        reason: String,
    },

    /// A lifecycle `proxy` hand-off is well-formed, but the command that
    /// launched this run owns no active-document coordinator able to consume
    /// it.
    ///
    /// The direct provider wrappers (`claudine claude`, `claudine goose`, …)
    /// run a provider memory file as their harness document and take the prompt
    /// from argv or stdin. They carry no run ledger and no composition
    /// coordinator, so there is nothing to re-enter the canonical
    /// selection/MCP/argv pipeline with. Adopting the target in place instead
    /// would run it against the *invocation's* launch bundle — the reduced path
    /// R3 forbids and R6/AC10 require rebuilt — so the hand-off is refused
    /// rather than silently mis-run. The composition commands (`compose`,
    /// `inline-compose`, `sequence`) do own a coordinator and consume the same
    /// hand-off normally.
    #[error(
        "lifecycle `proxy` hand-off to `{target}` has no owning coordinator: \
         `{command}` prepares no active document to hand off to ({source_path})",
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleProxyWithoutOwningCoordinator {
        /// The document whose stack authored the proxy.
        source_path: PathBuf,
        /// The dotted `"{event}.stack[{i}].action[{j}]"` location.
        property: String,
        /// The proxy target, as the source authored it.
        target: String,
        /// The invoked command that owns no coordinator (e.g. `claudine claude`).
        command: String,
    },

    /// A proxy target could not be brought up far enough to run: the
    /// coordinator committed the hand-off, but the target's staged boot could
    /// not produce the surface the next stage requires.
    ///
    /// Carries the source and the action location so a bootstrap failure is
    /// attributable to the `proxy` that requested it — the target itself is
    /// often blameless, and without provenance the user sees a document they
    /// never named failing for a reason they cannot trace. This is the typed
    /// replacement for what was a bare `eyre!` string on the adoption path.
    #[error(
        "proxy target `{target_path}` could not be prepared: {reason} \
         (requested by {property} in {source_path})",
        target_path = biscuit_file::to_portable_string(target_path),
        source_path = biscuit_file::to_portable_string(source_path)
    )]
    LifecycleProxyTargetBootstrapFailed {
        /// The resolved target that failed to bootstrap.
        target_path: PathBuf,
        /// The document whose lifecycle requested the hand-off.
        source_path: PathBuf,
        /// The dotted `"{event}.stack[{i}].action[{j}]"` location of the
        /// `proxy` action.
        property: String,
        /// What the staged boot could not produce.
        reason: String,
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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
        source_path = biscuit_file::to_portable_string(source_path)
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

    /// A template key collides with a reserved sequence overlay key.
    #[error("sequence template key `{0}` collides with reserved sequence key")]
    SequenceReservedTemplateKey(String),

    /// A step declared more than one executable field.
    ///
    /// A step runs the document body (no executable) or exactly one of
    /// `prompt`/`shell`/`side_effect`/`group`/`task` — never several.
    #[error(
        "sequence step at index {index} declares multiple executable fields ({}); \
         a step may declare at most one of prompt/shell/side_effect/group/task",
        fields.join(", ")
    )]
    SequenceExclusiveExecutable {
        /// Zero-based step index.
        index: usize,
        /// The executable field names found on the step.
        fields: Vec<String>,
    },

    /// An authored step state key collides with a generated or root reserved key.
    #[error(
        "sequence step at index {index} uses reserved key `{key}` as state; \
         it is generated by Claudine and cannot be authored"
    )]
    SequenceReservedStateKey {
        /// Zero-based step index.
        index: usize,
        /// The offending reserved key.
        key: String,
    },

    /// A task option is meaningless for the step's chosen executable.
    #[error(
        "sequence step at index {index} sets `{field}`, which is not valid for a \
         `{executable}` task"
    )]
    SequenceInvalidTaskField {
        /// Zero-based step index.
        index: usize,
        /// The offending task-option field.
        field: String,
        /// The executable the step declared.
        executable: String,
    },

    /// The `<file-ref> [-> offset] [::op(args)]` suffix could not be parsed.
    #[error("invalid sequence source `{authored}`: {problem}")]
    SequenceSourceSyntax {
        /// The full authored source string.
        authored: String,
        /// What specifically was wrong with it.
        problem: String,
    },

    /// An operator name is not one of `map`, `name`, or `template`.
    #[error("unknown sequence source operator `{0}`; expected `map`, `name`, or `template`")]
    SequenceUnknownOperator(String),

    /// An operator was given the wrong number of arguments.
    #[error("sequence source operator `{operator}` takes {expected} argument(s), got {found}")]
    SequenceOperatorArity {
        /// The operator name.
        operator: String,
        /// How many arguments the operator requires.
        expected: usize,
        /// How many were supplied.
        found: usize,
    },

    /// An offset path segment does not exist in the source document.
    #[error(
        "sequence offset `{path}` does not exist: `{failed_at}` is not present \
         (found {found} at that point)"
    )]
    SequenceOffsetMissing {
        /// The full dot-notation path attempted.
        path: String,
        /// The path prefix at which traversal failed.
        failed_at: String,
        /// The JSON type of the value the failing segment was looked up on.
        found: String,
    },

    /// An offset path resolved to something that is not a list.
    #[error("sequence offset `{path}` resolved to {found}, expected a list")]
    SequenceOffsetNotAList {
        /// The dot-notation path (or `<root>` for the document root).
        path: String,
        /// The JSON type actually found.
        found: String,
    },

    /// A `->` offset was used against a line-delimited file.
    ///
    /// JSONL/NDJSON documents are always a list at the root, so there is
    /// nothing for an offset to select.
    #[error(
        "`->` offsets are not supported for {format} files ({path}); \
         their root is always the list"
    )]
    SequenceOffsetUnsupported {
        /// The resolved data-file path.
        path: PathBuf,
        /// The rejected format's display name (`JSONL`/`NDJSON`).
        format: String,
    },

    /// An operator was applied to an item that is not an object.
    #[error(
        "sequence operator `{operator}` requires object items, but item {index} is {found}"
    )]
    SequenceOperatorItemNotObject {
        /// The operator name.
        operator: String,
        /// Zero-based item index.
        index: usize,
        /// The JSON type actually found.
        found: String,
    },

    /// An operator's source field is missing from an item.
    #[error("sequence operator `{operator}` cannot read `{field}`: item {index} does not have it")]
    SequenceOperatorMissingField {
        /// The operator name.
        operator: String,
        /// The field the operator reads.
        field: String,
        /// Zero-based item index.
        index: usize,
    },

    /// A `template(expr)` operator produced an empty or null name.
    #[error(
        "sequence operator `template({expression})` produced an empty name for item {index}; \
         a name must be a non-empty string"
    )]
    SequenceOperatorEmptyName {
        /// Zero-based item index.
        index: usize,
        /// The template expression source.
        expression: String,
    },

    /// A dynamic `{{ … }}` or `template(...)` expression failed.
    #[error("sequence expression `{expression}` failed: {source}")]
    SequenceExpressionFailed {
        /// The expression source.
        expression: String,
        /// Whether it failed to parse or to evaluate, and why.
        #[source]
        source: SequenceExpressionCause,
    },

    /// A `$( … )` sequence source could not be expanded.
    #[error("sequence shell source `$({command})` failed: {source}")]
    SequenceShellFailed {
        /// The command inside the `$( … )` span.
        command: String,
        /// Where in approval or execution it failed.
        #[source]
        source: SequenceShellCause,
    },

    /// A lenient (foreign-data) item was `null`.
    #[error("sequence item {index} is null; every item must be a scalar or an object")]
    SequenceNullItem {
        /// Zero-based item index.
        index: usize,
    },

    /// A step's normalized state failed the sequence document's `$schema`.
    #[error(
        "sequence step {index} (`{id}`) failed schema validation at `{property}`: {message}"
    )]
    SequenceStateSchemaViolation {
        /// Zero-based step index.
        index: usize,
        /// The step's generated id.
        id: String,
        /// The failing property path.
        property: String,
        /// The validation message.
        message: String,
    },

    // -- Sequence preflight graph (phase 5) ------------------------------------
    /// A task/group/prompt reference chain returned to a document already on the
    /// ancestry stack. The chain is reported whole so the author can see which
    /// hop closed the loop rather than only the repeated file.
    #[error("sequence reference cycle: {}", render_reference_chain(chain))]
    SequenceReferenceCycle {
        /// The full canonical chain, from the entry document to the repeat.
        chain: Vec<PathBuf>,
    },

    /// A referenced document could not be loaded or is not the expected `kind`.
    #[error("{context} ({path}): {problem}")]
    SequenceReferenceInvalid {
        /// What the reference was being loaded as (`task file`, `group file`, …).
        context: String,
        /// The resolved path.
        path: PathBuf,
        /// What was wrong with it.
        problem: String,
    },

    /// A `prompt:` task referenced a document that itself declares `sequence:`.
    ///
    /// Nested sequences are out of scope in v1 (spec → *Static Preflight*).
    #[error(
        "nested sequences are not supported: `{path}` declares `sequence:` but is \
         referenced as a prompt task from `{referenced_from}`"
    )]
    SequenceNestedSequence {
        /// The referenced prompt document.
        path: PathBuf,
        /// The document that referenced it.
        referenced_from: PathBuf,
    },

    /// A construct that preflight recognizes but v1 deliberately does not run.
    ///
    /// Covers nested groups, group `loop` (commit semantics unratified — spec →
    /// *Open Questions*), and direct execution of a `kind: group` document.
    #[error("{construct} is not supported: {detail}")]
    SequenceUnsupportedConstruct {
        /// The rejected construct, named as the author wrote it.
        construct: String,
        /// Why it is rejected and what to do instead.
        detail: String,
    },

    /// A `prompt:` task's document could not be launched at all.
    ///
    /// Distinct from [`Self::SequenceTaskPromptFailed`], which reports a
    /// provider that ran and exited non-zero: this is the wrapper failing before
    /// or around the run, so the message is the wrapper's, not the agent's.
    #[error("task `{task}` could not run prompt `{path}`: {message}")]
    SequenceTaskPromptLaunch {
        /// The task's name or generated label.
        task: String,
        /// The resolved document path.
        path: PathBuf,
        /// What the wrapper reported.
        message: String,
        /// The launch failure's own diagnostic identity, when the wrapper's
        /// error chain carried one.
        ///
        /// The wrapper returns an erased `Report`, which cannot become a
        /// `#[source]`. Projecting the chain's selected diagnostic here keeps
        /// `code`, `category`, `detail`, and the one-level cause that `message`
        /// alone would discard (spec §D9). `None` when the chain held only
        /// prose.
        /// Boxed to keep `CompositionError` small — it is the `Err` type of
        /// most composition `Result`s.
        snapshot: Option<Box<DiagnosticSnapshot>>,
    },

    /// A step has neither an executable nor a body to run.
    ///
    /// A step without an executable field runs the sequence document's own
    /// body. When that body composes to nothing there is no work to send, and
    /// silently launching a provider with an empty prompt is worse than saying
    /// so — the author either meant to write a body or to declare a task.
    #[error(
        "step {step} (`{name}`) has nothing to run: `{path}` composed to an empty body and the \
         step declares no `prompt`, `shell`, `side_effect`, `task`, or `group`"
    )]
    SequenceStepNotExecutable {
        /// One-based step position.
        step: usize,
        /// The step's display name.
        name: String,
        /// The sequence document whose body composed empty.
        path: PathBuf,
    },

    /// A `{group}@{file-ref}` catalog reference did not resolve to one group.
    #[error(
        "group `{name}` {problem} in catalog `{path}`{}",
        render_available_groups(available)
    )]
    SequenceGroupCatalogLookup {
        /// The requested group name.
        name: String,
        /// The resolved catalog path.
        path: PathBuf,
        /// `is not defined` or `is defined more than once`.
        problem: String,
        /// The names the catalog does define, for a did-you-mean list.
        available: Vec<String>,
    },

    /// A group definition violates the group schema.
    #[error("group `{group}` ({origin}) is invalid: {problem}")]
    SequenceGroupInvalid {
        /// The group's name, or `<inline>` when it has none yet.
        group: String,
        /// The document the group was authored in.
        origin: PathBuf,
        /// The specific schema violation.
        problem: String,
    },

    /// Two tasks in the same parallel group write back to the same document.
    ///
    /// Racing inline-compose write-backs are never legal, and preflight holds
    /// the whole graph, so the collision is caught before anything launches.
    #[error(
        "parallel group `{group}` would write back to `{target}` from more than one task \
         (`{first}` and `{second}`); inline-compose targets must be distinct"
    )]
    SequenceWriteBackCollision {
        /// The owning parallel group.
        group: String,
        /// The canonical target path both tasks would rewrite.
        target: PathBuf,
        /// The first task's label.
        first: String,
        /// The second task's label.
        second: String,
    },

    /// A task's shell command references a value that does not exist at
    /// preflight, so its approved bytes could never equal its executed bytes.
    #[error(
        "shell command `{command}` in {task} references `{root}`, which is not available at \
         preflight; shell commands are approved before the sequence starts, so only \
         early-binding values (`state`, `params`, `doc.*`, `ctx.*`, `env.*`) may be used. \
         Route work that consumes prior output through a `prompt` or `side_effect` task"
    )]
    SequenceShellLateBinding {
        /// The authored command.
        command: String,
        /// The offending root (`outputs`, `err`, `timing`, `current`).
        root: String,
        /// A label locating the task.
        task: String,
    },

    /// A sequence graph command references target identity before the task's
    /// provider and model have been selected.
    #[error(
        "shell command `{command}` in {task} references `{root}` before a task target is \
         available during graph preflight; move target-dependent commands into a task-scoped \
         prompt or lifecycle stack"
    )]
    SequenceShellTargetIdentity {
        /// The authored command.
        command: String,
        /// The offending target-dependent path.
        root: String,
        /// A label locating the task.
        task: String,
    },

    /// A task's shell command failed early-binding resolution at preflight.
    #[error("shell command `{command}` in {task} could not be resolved at preflight: {message}")]
    SequenceShellResolution {
        /// The authored command.
        command: String,
        /// A label locating the task.
        task: String,
        /// The underlying resolution failure.
        message: String,
        /// The typed lower-layer failure, when one was raised.
        #[source]
        source: Option<Box<MarkdownError>>,
    },

    // -- Atomic task execution (phase 7) ---------------------------------------
    /// A task's `timeout:` value is not a duration the repository's parser
    /// accepts.
    ///
    /// A bare integer is deliberately rejected along with `0s`: the unit is part
    /// of the grammar, and an unbounded shell command has no expression here
    /// (spec → *Task Resolution and Lifecycle Semantics*).
    #[error("`timeout` in {} is not a valid duration: {source}", context.task)]
    SequenceTaskTimeoutInvalid {
        /// Where it was authored and what it said. Boxed — the *context*, never
        /// the cause — so the variant stays under the `result_large_err` floor
        /// while the `HarnessError` remains an unboxed, downcastable chain
        /// member (the rule [`InvalidFileReference`] records).
        ///
        /// [`InvalidFileReference`]: CompositionError::InvalidFileReference
        context: Box<TaskTimeoutContext>,
        /// The parser's typed rejection.
        #[source]
        source: crate::harness::HarnessError,
    },

    /// A task's shell command finished with a non-zero exit code.
    #[error("command `{command}` in {task} exited with code {code}")]
    SequenceTaskShellExit {
        /// A label locating the task.
        task: String,
        /// The approved command bytes that ran.
        command: String,
        /// The process exit code.
        code: i32,
    },

    /// A task's shell command exceeded its per-command timeout and was killed.
    #[error("command `{command}` in {task} timed out after {seconds}s")]
    SequenceTaskShellTimeout {
        /// A label locating the task.
        task: String,
        /// The approved command bytes that ran.
        command: String,
        /// The elapsed budget, in seconds.
        seconds: f64,
    },

    /// A task's shell command was killed because it breached the per-task
    /// runaway output-volume guard.
    ///
    /// Distinct from [`SequenceTaskShellTimeout`]: that command was too slow,
    /// this one was too loud. A flood is deterministic in a way a deadline
    /// breach is not, so this fails the task outright rather than inviting a
    /// retry on a quieter machine.
    ///
    /// [`SequenceTaskShellTimeout`]: CompositionError::SequenceTaskShellTimeout
    #[error("command `{command}` in {task} produced runaway output ({trip}) and was stopped")]
    SequenceTaskShellRunaway {
        /// A label locating the task.
        task: String,
        /// The approved command bytes that ran.
        command: String,
        /// Which capture limit tripped, the observed count, and the configured
        /// limit — what separates a slightly oversized command from a flood.
        trip: RunawayTrip,
    },

    /// A task's shell command could not be spawned at all.
    #[error("command `{command}` in {task} failed to run: {source}")]
    SequenceTaskShellSpawn {
        /// A label locating the task.
        task: String,
        /// The approved command bytes that were attempted.
        command: String,
        /// The underlying spawn failure.
        #[source]
        source: std::io::Error,
    },

    /// A task's shell command spawned and ran, but waiting on it failed, so
    /// its exit status is unknown.
    ///
    /// Distinct from [`SequenceTaskShellSpawn`]: that command never started,
    /// this one did — its output may already be on screen — so reporting it
    /// as "failed to run" would misstate what the user watched happen.
    ///
    /// [`SequenceTaskShellSpawn`]: CompositionError::SequenceTaskShellSpawn
    #[error("command `{command}` in {task} ran, but waiting for it failed: {source}")]
    SequenceTaskShellWait {
        /// A label locating the task.
        task: String,
        /// The approved command bytes that ran.
        command: String,
        /// The underlying wait failure.
        #[source]
        source: std::io::Error,
    },

    /// A task's shell command spawned, but exclusive ownership of its process
    /// tree could not be established, so it was killed without running.
    ///
    /// Fail-closed by design: the per-command deadline and the runaway guards
    /// are only enforceable against an owned tree (a Unix process group, a
    /// Windows Job Object), so a command that could only be given
    /// direct-child cleanup is refused rather than run degraded.
    #[error("command `{command}` in {task} could not be isolated into an owned process tree: {source}")]
    SequenceTaskShellIsolation {
        /// A label locating the task.
        task: String,
        /// The approved command bytes that were attempted.
        command: String,
        /// The underlying ownership failure.
        #[source]
        source: std::io::Error,
    },

    /// A task's `prompt:` document ran to completion but reported failure.
    #[error("prompt `{path}` in {task} exited with code {code}")]
    SequenceTaskPromptFailed {
        /// A label locating the task.
        task: String,
        /// The composed document.
        path: PathBuf,
        /// The provider's exit code.
        code: i32,
    },

    /// A task's `side_effect:` value is not a side-effect action.
    ///
    /// The value uses the standard lifecycle action grammar, which also admits
    /// communication and flow-control verbs; only a side effect is executable
    /// work, so anything else is an authoring error rather than a silent no-op.
    #[error("`side_effect` in {task} is not a side-effect action: {problem}")]
    SequenceTaskInvalidSideEffect {
        /// A label locating the task.
        task: String,
        /// What the value was instead.
        problem: String,
    },

    /// A task parameter targets a reserved key.
    #[error(
        "parameter `{key}` in {task} targets a reserved key; `state`, `previous`, `next`, \
         `outputs`, and `sequence_id` are executor-owned views"
    )]
    SequenceTaskParamReserved {
        /// A label locating the task.
        task: String,
        /// The refused parameter name.
        key: String,
    },

    /// A task value failed just-in-time evaluation against the effective state.
    #[error("`{field}` in {task} could not be evaluated: {message}")]
    SequenceTaskValueResolution {
        /// A label locating the task.
        task: String,
        /// The task field being evaluated (`params.topic`, `timeout`).
        field: String,
        /// The underlying evaluation failure.
        message: String,
        /// The typed lower-layer failure.
        #[source]
        source: Box<MarkdownError>,
    },

    /// A task shape reached execution that only a later phase can schedule.
    #[error("{construct} in {task} is not executable yet: {detail}")]
    SequenceTaskUnsupported {
        /// A label locating the task.
        task: String,
        /// The construct that was reached.
        construct: String,
        /// What the author should do instead.
        detail: String,
    },

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

    /// A `loop.while` / `loop.until` condition failed to parse or evaluate.
    ///
    /// The typed sibling of [`LoopInvalid`], which is a bare newtype with no
    /// slot for a cause. Unlike the lifecycle shape family, this variant derives
    /// its prose at `Display` time from `kind`, `condition`, and the cause's
    /// stage rather than storing a `message`: it is the only renderer of that
    /// prose, so a second copy would be a field that can drift from the one
    /// string that reads it. Byte-identical to the `LoopInvalid(format!(…))` it
    /// replaces.
    ///
    /// Staged here for the looping burn-down batch, which owns
    /// `looping/expression.rs`; see `features/2026-07-13-error-propogation/burndown-triage.md`.
    ///
    /// [`LoopInvalid`]: CompositionError::LoopInvalid
    #[error(
        "invalid loop definition: failed to {stage} loop.{kind} `{condition}`: {source}",
        stage = source.stage()
    )]
    LoopExpressionInvalid {
        /// The condition keyword — `while` or `until`.
        kind: String,
        /// The condition expression exactly as authored.
        condition: String,
        /// The typed parse-or-evaluate failure.
        #[source]
        source: LoopExpressionCause,
    },

    /// Loop execution exceeded its configured safety cap.
    #[error(
        "loop limit exceeded for {prompt_path} at iteration {iteration}; cap is {cap}",
        prompt_path = biscuit_file::to_portable_string(prompt_path)
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

    /// A loop action's `{{ … }}` template failed to parse or evaluate.
    ///
    /// The typed sibling of [`InvalidAction`] for the two template sites that
    /// hold a concrete cause. `InvalidAction` stays as-is because its shared
    /// `invalid_action` constructor also serves sites with nothing to retain (a
    /// reserved property name, a rejected value shape), so a mandatory source
    /// field there would have no value to take.
    ///
    /// Staged here for the looping burn-down batch, which owns
    /// `looping/actions.rs`; see `features/2026-07-13-error-propogation/burndown-triage.md`.
    ///
    /// [`InvalidAction`]: CompositionError::InvalidAction
    #[error(
        "invalid loop action at iteration {iteration}, action {action_index} \
         of {total_actions}: {}",
        source.action_message(expression)
    )]
    LoopActionExpressionInvalid {
        /// 1-based iteration index.
        iteration: usize,
        /// 1-based action index.
        action_index: usize,
        /// Total actions in this iteration.
        total_actions: usize,
        /// The template's inner expression text, trimmed, without its braces.
        expression: String,
        /// The typed parse-or-evaluate failure.
        #[source]
        source: LoopExpressionCause,
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
        prompt_path = biscuit_file::to_portable_string(prompt_path)
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
        prompt_path = biscuit_file::to_portable_string(prompt_path)
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
        /// The iteration failure's own diagnostic identity, when the wiring
        /// error chain carried one.
        ///
        /// Pre-spawn wiring returns an erased `Report`, which cannot become a
        /// `#[source]`. Projecting the chain's selected diagnostic here keeps
        /// `code`, `category`, `detail`, and the one-level cause that `reason`
        /// alone would discard (spec §D9). `None` for the provider-exited
        /// path, whose `reason` is assembled from session_end fields rather
        /// than from a typed error.
        /// Boxed to keep `CompositionError` small — it is the `Err` type of
        /// most composition `Result`s.
        snapshot: Option<Box<DiagnosticSnapshot>>,
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
    #[error(
        "schema load failed for {}: {message}",
        biscuit_file::to_portable_string(source_path)
    )]
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
    #[error(
        "schema parse failed for {}: {message}",
        biscuit_file::to_portable_string(source_path)
    )]
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
        biscuit_file::to_portable_string(source_path)
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
        biscuit_file::to_portable_string(source_path),
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
        biscuit_file::to_portable_string(source_path)
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
        biscuit_file::to_portable_string(source_path)
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
        prompt_path = biscuit_file::to_portable_string(prompt_path)
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
        biscuit_file::to_portable_string(source_path),
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

/// Where a file reference was authored, for
/// [`CompositionError::InvalidFileReference`].
///
/// `event` and `property` are what tell one authoring surface from another in
/// `err.detail.*`, which is why the wrapper needs no surface-specific code.
#[derive(Debug, Clone)]
pub struct FileReferenceContext {
    /// The document that authored the reference.
    pub source_path: PathBuf,
    /// The lifecycle event that was running, when the reference was authored
    /// inside a lifecycle stack. `None` for a non-lifecycle surface.
    pub event: Option<String>,
    /// Dotted property path naming the authored value, e.g.
    /// `"initialize.stack[0].proxy"`.
    pub property: String,
    /// The reference exactly as authored, before any resolution.
    pub reference: String,
    /// Remediation naming the contract the reference violated.
    pub hint: String,
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
    /// A JSON/JSON5 load/convert failure.
    #[error(transparent)]
    Json5(#[from] biscuit_file::Json5Error),
    /// A line-delimited (JSONL/NDJSON) entry failed to parse.
    ///
    /// The line number is the whole point of this arm: a bare `serde_json`
    /// error locates itself within the single line it was handed, which is
    /// useless for finding the offending record in the file.
    #[error("line {line}: {source}")]
    JsonLine {
        /// One-based line number within the file.
        line: usize,
        /// The underlying parse failure.
        #[source]
        source: serde_json::Error,
    },
    /// Building or running the step-state schema validator failed.
    ///
    /// Boxed to keep the enum small — `SchemaError` is by far its heaviest
    /// member.
    #[error(transparent)]
    Schema(#[from] Box<darkmatter::markdown::schemas::SchemaError>),
    /// A generated step-state frontmatter could not be assembled for
    /// validation.
    #[error(transparent)]
    Frontmatter(#[from] Box<darkmatter::markdown::MarkdownError>),
    /// The file could not be read from disk.
    #[error(transparent)]
    Read(#[from] std::io::Error),
    /// The reference resolved but no file exists at the resolved path.
    #[error("file not found")]
    NotFound,
    /// `~` expansion failed because no home directory is known.
    #[error("unable to resolve home directory")]
    HomeDir,
}

/// Which stage of a sequence-source expression failed.
///
/// The sequence source layer evaluates expressions at three sites (a
/// whole-value `{{ … }}` source, a `::template(expr)` operator, and a formal
/// document's `template:` values); all three share this cause so a handler can
/// recover the concrete Darkmatter error by downcasting once.
#[derive(Error, Debug)]
pub enum SequenceExpressionCause {
    /// The expression could not be parsed.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// The expression parsed but could not be evaluated.
    ///
    /// Boxed to keep the enum small, matching [`LoopExpressionCause`].
    #[error(transparent)]
    Evaluate(#[from] Box<ExpressionError>),
}

/// Where a `$( … )` sequence source failed.
#[derive(Error, Debug)]
pub enum SequenceShellCause {
    /// The command was rejected by the shell-approval gate.
    ///
    /// Carried unboxed on purpose: `as_diagnostic` cannot downcast through a
    /// `Box`, so boxing this would render an approval denial as a generic
    /// `Error:` line instead of its own diagnostic.
    #[error(transparent)]
    Approval(#[from] crate::harness::HarnessError),
    /// The approved command could not be spawned.
    #[error(transparent)]
    Spawn(#[from] std::io::Error),
    /// The command ran but reported failure.
    #[error("exited with {status}: {stderr}")]
    Exited {
        /// The rendered exit status.
        status: String,
        /// The command's trimmed stderr.
        stderr: String,
    },
    /// No approval-capable runner was available to expand the source.
    ///
    /// This arm genuinely has no lower cause: nothing was attempted.
    #[error("shell sequence sources are not available in this context")]
    Unavailable,
}

/// Heterogeneous lower-layer cause of a loop expression failure.
///
/// Carried as the typed `#[source]` of [`CompositionError::LoopExpressionInvalid`]
/// and [`CompositionError::LoopActionExpressionInvalid`]. Both variants can fail
/// at either of two stages against the same authored expression, and the stage
/// is what their `Display` prose turns on — so the cause carries it rather than
/// each variant re-declaring a discriminant beside its source.
///
/// Like [`MarkdownLoadCause`], the arms are `transparent`: they replace the
/// concrete error in the chain rather than adding a hop to it, so a handler
/// recovers the stage by downcasting to this enum and matching, not by walking
/// one level deeper.
#[derive(Error, Debug)]
pub enum LoopExpressionCause {
    /// The expression could not be parsed.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// The expression parsed but could not be evaluated.
    ///
    /// Boxed to keep the enum small: `ExpressionError` is the heavy member,
    /// mirroring how `MarkdownError::Interpolation` boxes the same type.
    #[error(transparent)]
    Evaluate(#[from] Box<ExpressionError>),
}

/// Lower-layer cause of an `action_value_to_expr` failure.
///
/// Carried as the optional typed `#[source]` of
/// [`CompositionError::LifecycleActionInvalidLongForm`] so a handler can
/// recover the concrete Darkmatter [`ParseError`] through
/// [`std::error::Error::source`] instead of re-parsing the flattened message.
/// The [`Invalid`](ActionExprError::Invalid) arm carries the genuinely-prose
/// rejections (a null, an unsupported number, or object/array scalar data)
/// that never had a typed lower cause.
///
/// `Display` is byte-identical to the strings `action_value_to_expr` built
/// before the cause was typed — typing an error is not a licence to reword it.
#[derive(Error, Debug)]
pub enum ActionExprError {
    /// A whole-value `{{ … }}` interpolation span did not parse.
    ///
    /// Boxed to keep `ActionExprError` (and thus the unboxed
    /// `LifecycleActionInvalidLongForm` source slot) small — [`ParseError`]
    /// is the heavy member, mirroring how [`MarkdownLoadCause::Parse`] boxes
    /// its own. The unboxed enum keeps the field downcastable while the boxed
    /// payload keeps `CompositionError` under `clippy::result_large_err`.
    #[error("whole-value expression is not valid: {0}")]
    Parse(#[from] Box<ParseError>),
    /// A scalar shape that cannot become an action expression at all.
    #[error("{0}")]
    Invalid(String),
}

impl LoopExpressionCause {
    /// The failing stage, as the verb both loop variants' `Display` reads.
    pub fn stage(&self) -> &'static str {
        match self {
            LoopExpressionCause::Parse(_) => "parse",
            LoopExpressionCause::Evaluate(_) => "evaluate",
        }
    }

    /// The loop-action prose for `expression`, byte-identical to the two
    /// `format!` strings [`CompositionError::InvalidAction`] carried before the
    /// stage was typed — the text is a user-visible surface, and typing the
    /// error is not a licence to reword it.
    fn action_message(&self, expression: &str) -> String {
        match self {
            LoopExpressionCause::Parse(error) => {
                format!("invalid template `{{{{{expression}}}}}` in loop action: {error}")
            }
            LoopExpressionCause::Evaluate(error) => {
                format!("failed to evaluate template `{{{{{expression}}}}}`: {error}")
            }
        }
    }
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

/// Render a reference-cycle chain as `a.md → b.yaml → a.md`.
fn render_reference_chain(chain: &[PathBuf]) -> String {
    chain
        .iter()
        .map(|path| biscuit_file::to_portable_string(path))
        .collect::<Vec<_>>()
        .join(" → ")
}

/// Render the "; catalog defines: …" tail of a catalog-lookup failure, or an
/// empty string when the catalog defines nothing to suggest.
fn render_available_groups(available: &[String]) -> String {
    if available.is_empty() {
        String::new()
    } else {
        format!("; catalog defines: {}", available.join(", "))
    }
}

fn format_missing_names(missing: &[MissingProperty]) -> String {
    missing
        .iter()
        .map(|m| m.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Where a rejected task `timeout:` was authored, for
/// [`CompositionError::SequenceTaskTimeoutInvalid`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTimeoutContext {
    /// A label locating the task.
    pub task: String,
    /// The authored value, rendered.
    pub raw: String,
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
            | CompositionError::LifecycleWhenExpressionInvalid { property, .. }
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
            | CompositionError::LifecycleErrNotAvailable { property, .. }
            | CompositionError::LifecycleProxyOnlyParameter { property, .. }
            // The target failed, but the `proxy` that named it is the line the
            // user can act on — and the only one in a document they authored.
            | CompositionError::LifecycleProxyTargetBootstrapFailed { property, .. }
            | CompositionError::LifecycleProxyWithoutOwningCoordinator { property, .. }
            | CompositionError::LifecycleTransitionUnownedAtStage { property, .. } => {
                Some(FrontmatterHighlight::Property(property.clone()))
            }
            CompositionError::InvalidFileReference { context, .. } => {
                Some(FrontmatterHighlight::Property(context.property.clone()))
            }
            // The `with`-rooted family names the deepest representable path;
            // the excerpt renderer walks back to the most specific line it can
            // locate.
            CompositionError::LifecycleProxyWithNotMapping { property, path, .. }
            | CompositionError::LifecycleProxyWithWholeMapping { property, path, .. }
            | CompositionError::LifecycleProxyWithDynamicKey { property, path, .. }
            | CompositionError::LifecycleProxyWithEvaluationFailed { property, path, .. } => {
                Some(FrontmatterHighlight::Property(format!("{property}.{path}")))
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
