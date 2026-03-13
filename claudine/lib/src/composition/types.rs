//! Core types for composition workflows.

use std::collections::BTreeSet;
use std::path::PathBuf;

use darkmatter::markdown::Markdown;

use crate::events::Provider;

/// Which composition mode to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionMode {
    /// Use the frontmatter `prompt` property as input; replace body with output.
    InlineFrontmatterPrompt,
    /// Compose the full document and send as prompt; no file mutation.
    ChainedDocument,
}

/// A request to perform composition.
#[derive(Debug, Clone)]
pub struct CompositionRequest {
    /// Which composition mode.
    pub mode: CompositionMode,
    /// The raw file reference string.
    pub file_ref: String,
    /// Explicitly chosen provider (from wrapper subcommand).
    pub explicit_provider: Option<Provider>,
    /// Providers to exclude from automatic selection.
    pub excluded: BTreeSet<Provider>,
    /// Force interactive provider selection.
    pub force_interactive_selection: bool,
}

/// A resolved and loaded composition source document.
#[derive(Debug, Clone)]
pub struct ResolvedCompositionSource {
    /// The original file reference string.
    pub original_ref: String,
    /// The resolved absolute path.
    pub resolved_path: PathBuf,
    /// The parsed Markdown document.
    pub markdown: Markdown,
}

/// A prompt prepared for provider execution.
#[derive(Debug, Clone)]
pub struct PreparedPrompt {
    /// Which composition mode produced this prompt.
    pub mode: CompositionMode,
    /// Path to the source file.
    pub resolved_path: PathBuf,
    /// The composed prompt text.
    pub prompt: String,
    /// The raw `agent` frontmatter value, if present.
    pub source_agent_hint: Option<serde_json::Value>,
}

/// Why a particular provider was selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionReason {
    /// The caller explicitly specified the provider (wrapper subcommand).
    ExplicitProvider,
    /// The `AGENT` environment variable selected the provider.
    EnvironmentOverride,
    /// The source document's `agent` frontmatter selected the provider.
    FrontmatterHint,
    /// The user chose interactively.
    InteractiveChoice,
    /// Automatic selection from ranked provider preferences.
    PreferenceFallback,
}

/// A provider selected for composition execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedProvider {
    /// The selected provider.
    pub provider: Provider,
    /// Why this provider was selected.
    pub reason: SelectionReason,
}
