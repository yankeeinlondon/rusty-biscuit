//! Composition-specific error types.

use std::path::PathBuf;

use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::compose::shell_expansion::ShellExpansionError;

use super::types::ResolutionMode;
use crate::events::Provider;
use thiserror::Error;

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

    /// Inline composition requires a `prompt` frontmatter property.
    #[error("frontmatter is missing a `prompt` property")]
    PromptPropertyMissing,

    /// The `prompt` frontmatter property is not a string.
    #[error("frontmatter `prompt` must be a string, got {0}")]
    PromptPropertyWrongType(String),

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

    /// The `agent` frontmatter property is not a valid type.
    #[error("frontmatter `agent` must be a string or array of strings, got {0}")]
    AgentHintWrongType(String),

    /// The `model` frontmatter property is not a valid type.
    #[error("frontmatter `model` must be a string or array of strings, got {0}")]
    ModelHintWrongType(String),

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

    /// Inline composition with `-i` is not supported for this provider
    /// because it cannot capture the final assistant message.
    #[error(
        "inline-compose with --interactive is not supported for {0}; the provider cannot capture the final assistant message"
    )]
    InlineInteractiveUnsupported(String),

    /// The provider returned an invalid response for inline composition.
    #[error("invalid inline composition response: {0}")]
    InvalidInlineResponse(String),

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
        /// The frontmatter property that failed to parse.
        property: String,
        /// The underlying deserialization error message.
        message: String,
    },

    /// A lifecycle notification property has both `say` and `say_first`.
    #[error("lifecycle property `{0}` has both `say` and `say_first`; only one is allowed")]
    LifecycleSayConflict(String),

    /// A lifecycle notification property references an unknown sound effect.
    #[error("lifecycle property `{0}` references unknown sound effect `{1}`")]
    LifecycleUnknownEffect(String, String),

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
