//! Composition-specific error types.

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

    /// Darkmatter composition failed.
    #[error("compose failed: {0}")]
    ComposeFailed(String),

    /// No installed providers can run composition.
    #[error("no runnable providers available (all excluded or uninstalled)")]
    NoRunnableProviders,

    /// The `agent` frontmatter hint does not match any known provider.
    #[error("agent hint `{0}` does not match any known provider")]
    AgentHintInvalid(String),

    /// The `agent` frontmatter hint matches multiple providers.
    #[error("agent hint `{hint}` is ambiguous; matches: {matches}")]
    AgentHintAmbiguous {
        hint: String,
        matches: String,
        /// The matched providers for interactive disambiguation.
        providers: Vec<Provider>,
    },

    /// Interactive selection is required but no TTY is available.
    #[error("interactive provider selection required but no TTY available; set AGENT env var or add an `agent` frontmatter property")]
    InteractiveSelectionRequired,

    /// The provider child process could not be launched.
    #[error("failed to launch provider: {0}")]
    ProviderLaunchFailed(String),

    /// The provider child process exited with a non-zero code.
    #[error("provider exited with code {0}")]
    ProviderRunFailed(i32),

    /// Atomic file write failed during inline composition.
    #[error("atomic write failed: {0}")]
    AtomicWriteFailed(String),
}
