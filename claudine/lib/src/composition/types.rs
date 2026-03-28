//! Core types for composition workflows.

use std::collections::BTreeSet;
use std::path::PathBuf;

use darkmatter::markdown::Markdown;
use serde_json;

use crate::events::Provider;

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

/// Why a particular provider was selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionReason {
    /// The caller explicitly specified the provider (wrapper subcommand).
    ExplicitProvider,
    /// Only one installed provider remained after exclusion filtering.
    SingleInstalled,
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

/// A composition prepared with effective (composed) frontmatter.
///
/// This struct carries the full effective frontmatter after Darkmatter
/// composition. All downstream code (harness, MCP, provider selection)
/// must read from `effective_frontmatter`, never from raw source state.
#[derive(Debug, Clone)]
pub struct PreparedComposition {
    /// Which composition mode produced this.
    pub mode: CompositionMode,
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
    /// The `agent` value from effective frontmatter, if present.
    pub effective_agent_hint: Option<serde_json::Value>,
    /// Closure plan for post-execution file updates.
    pub closure: CompositionClosurePlan,
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
    /// Hash of the original body (for unchanged-body detection).
    pub original_body_hash: u64,
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
    pub output: Option<String>,
    /// Set or append a system prompt (string or file path).
    pub system_prompt: Option<String>,
    /// Timeout in seconds for non-interactive mode.
    pub timeout: Option<u64>,
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
    /// Show only header; suppress env details and info.
    pub quiet: bool,
    /// Suppress all preflight output.
    pub silent: bool,
}
