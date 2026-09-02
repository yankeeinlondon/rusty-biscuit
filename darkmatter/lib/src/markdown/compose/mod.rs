//! Compose pipeline for markdown document preparation and transclusion.
//!
//! This module provides the `compose()` family of methods on `Markdown`
//! for running operations in four phases:
//!
//! **Inline Pre** (serial):
//! 0. **Frontmatter Interpolation** - Resolve `{{variable}}` in frontmatter values.
//!    When shell expansion is enabled, templated keys that reference
//!    shell-pending values (top-level `$(...)`) are deferred for a second
//!    interpolation pass after step 2 completes.
//! 1. **Schema Validation** (pre-operation stage) - Validate frontmatter against `$schema` or
//!    `ComposeOptions::baseline_schema`. Runs after `--set` / `--state`
//!    overrides and frontmatter interpolation are applied, but before
//!    frontmatter shell expansion.
//! 2. **Frontmatter Shell Expansion** - Execute shell commands in frontmatter
//!    values, then re-run interpolation to resolve any keys deferred above.
//! 3. **Text Replacement** - Replace literal strings from frontmatter `replace` map
//! 4. **Page Blocks** - Evaluate `::block`/`::end-block` conditional regions
//! 5. **Interpolation** - Expand `{{variable}}` expressions in body content
//! 6. **Shell Expansion** - Execute `::shell` directives with security controls
//! 7. **Shell Blocks** - Execute `::shell-block` directives with security controls
//! 8. **Link Resolve** - Resolve local links to absolute paths
//!
//! **Transclusion** (concurrent execution after serial preparation):
//! 9. **Block Transclusion** - Include `::file`/`::url` referenced documents
//! 10. **Frontmatter Transclusion** - Prepend/append `prologue`/`epilogue` documents
//! 11. **Code Transclusion** - Include `::code` file content as fenced blocks
//! 12. **TOC Linking** - Expand `::toc-linking` directives into heading link lists
//! 13. **File Links** - Expand `::file-links` directives into a linked file tree
//!
//! **Inline Post** (serial):
//! 14. **Cleanup** - Normalize markdown formatting
//! 15. **Normalization** - Adjust heading levels
//!
//! **Finalization** (root-only serial):
//! 16. **Link Normalization** - Convert absolute paths back to portable forms
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::Markdown;
//! use darkmatter::markdown::compose::{ComposeOptions, ComposeOperation};
//!
//! let content = "# Hello\nWorld";
//! let mut md: Markdown = content.into();
//!
//! // Transform with default options (all operations enabled)
//! let report = md.compose_mut().unwrap();
//!
//! // Transform with specific operations disabled
//! let options = ComposeOptions::new()
//!     .disable(ComposeOperation::Cleanup)
//!     .disable(ComposeOperation::Normalization);
//! let report = md.compose_with(options).unwrap();
//! ```
//!
//! ## Maintenance
//!
//! Keep this module from becoming a "god file":
//!
//! - New user-toggleable compose stages must add a [`ComposeOperationDescriptor`]
//!   entry in `pipeline/operations.rs` and a dedicated stage module under
//!   `markdown/compose/`.
//! - Non-toggleable pipeline sub-stages (schema validation, effective-state build,
//!   transclusion parse/prepare/resolve/apply) stay in `perf.rs` and are not added
//!   to the operation descriptor table.
//! - New render-tree block extensions extend a dedicated `markdown/render_tree/`
//!   module rather than inline code in the fold.
//! - Large in-file test suites move to a sibling `tests` module when production
//!   code around them changes.

pub(crate) mod cache;
pub mod conditions;
pub mod context;
mod frontmatter_interpolation;
pub(crate) mod frontmatter_shell_expansion;
pub(crate) mod indent;
pub(crate) mod parse_utils;
pub(crate) mod perf;
pub(crate) mod pipeline;
mod schema_validation;
pub mod subtree;
mod util;

#[cfg(test)]
mod type_tests;

pub mod block_pairs;
pub mod directives_api;
pub mod expression;
pub mod file_links;
pub(crate) mod inline;
pub mod interpolation;
pub(crate) mod link_normalization;
pub(crate) mod link_resolve;
pub mod page_blocks;
pub mod preflight;
pub(crate) mod remote_fetch;
pub mod remote;
pub mod replacement;
pub mod shell_blocks;
pub mod shell_expansion;
pub mod toc_linking;
pub mod transclusion;

pub use biscuit_file::PathPosition;
pub use cache::{CacheAccessMode, CacheFreshnessMode, CacheStats};
pub use context::{
    ContextCaptureEvidence, ContextGroup, ContextMergeDiagnostic, ContextRequirements,
};
pub use file_links::FileLinksError;
pub use remote::{
    DEFAULT_REMOTE_CONCURRENCY, DiscoveredRemoteUrl, REMOTE_CONCURRENCY_ENV, RemoteFreshnessMode,
    RemoteReadConfig, RemoteReadError, RemoteUrlCatalog, RemoteUrlConsumer,
    resolve_remote_concurrency,
};
pub use remote_fetch::RemoteFetchStats;
pub use frontmatter_shell_expansion::{
    FrontmatterShellAction, FrontmatterShellBody, FrontmatterShellPipeline, FrontmatterShellSuffix,
    FrontmatterShellTernary, FrontmatterShellValue, parse_frontmatter_shell_value_spanned,
};
pub use preflight::{ComposePreflightApprovals, ComposePreflightReport, PreflightApprovalStats, collect_shell_commands};
pub use shell_blocks::ShellBlockError;
pub use shell_expansion::ShellCommandOrigin;
pub use shell_expansion::ShellExpansionError;
pub use shell_expansion::ShellTimeoutBehavior;
pub use context::effective_state::{EffectiveState, EffectiveStateBuilder};
pub(crate) use context::effective_state::ResolvingLookup;
pub use context::options::{CallerInputRecord, CallerInputRecords, ComposeOptions, ComposeSource};
pub use context::capture_file_resolution_context;
pub use context::repository_scope_catalog;
pub(crate) use context::options::ReferenceGraphOptionsIdentity;
pub use context::report::{ComposeReport, ComposeWarning, SourceRange};
pub use context::runtime::ComposeContext;
pub use perf::{
    ComposePerfMetric, ComposePerfReport, ComposeStage, ShellCommandSpan, redact_shell_command,
};
pub use pipeline::operations::{
    ComposeOperation, ComposeOperationDescriptor, ComposeOperationPerfMetric, ComposeOperationSet,
    ComposePhase,
};
pub use toc_linking::TocLinkingError;
pub use transclusion::TransclusionError;

// Internal re-export for crate modules that still use TransclusionOptions
pub(crate) use context::options::TransclusionOptions;

// Shared helpers, re-exported so in-crate callers reach them as `compose::<name>`.
pub use util::find_git_root_from;
pub(crate) use util::{
    abbreviate_path, document_resolution_context, find_target_range, prepare_frontmatter_for_compose,
};

use super::Markdown;
use super::types::MarkdownResult;
use tracing::instrument;

// The transclusion preparation/resolution types and engine live in
// `transclusion/engine.rs`; the driver below imports them via `transclusion::`.

// Re-export HeadingLevel for tests
#[cfg(test)]
pub use super::normalize::HeadingLevel;

impl Markdown {
    /// Transforms the document using default options.
    ///
    /// This is equivalent to `compose_with(ComposeOptions::new())`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    ///
    /// let content = "# Test\nContent";
    /// let md: Markdown = content.into();
    /// let (composed, report) = md.compose().unwrap();
    /// ```
    pub fn compose(&self) -> MarkdownResult<(Markdown, ComposeReport)> {
        self.compose_with(ComposeOptions::new())
    }

    /// Transforms the document with custom options.
    ///
    /// Returns a new `Markdown` document and a report of changes made.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions};
    ///
    /// let content = "# Test\nContent";
    /// let md: Markdown = content.into();
    ///
    /// let options = ComposeOptions::new()
    ///     .disable(ComposeOperation::Normalization);
    ///
    /// let (composed, report) = md.compose_with(options).unwrap();
    /// ```
    #[instrument(skip_all, fields(source = ?options.source))]
    pub fn compose_with(
        &self,
        options: ComposeOptions,
    ) -> MarkdownResult<(Markdown, ComposeReport)> {
        let mut result = self.clone();
        let report = result.run_compose_pipeline(options)?;
        Ok((result, report))
    }

    /// Transforms the document in place, returning only the report.
    ///
    /// This is more efficient than `compose()` when you don't need
    /// to preserve the original document.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    ///
    /// let content = "# Test\nContent";
    /// let mut md: Markdown = content.into();
    /// let report = md.compose_mut().unwrap();
    ///
    /// // md is now composed
    /// ```
    pub fn compose_mut(&mut self) -> MarkdownResult<ComposeReport> {
        self.run_compose_pipeline(ComposeOptions::new())
    }

}

#[cfg(test)]
mod tests;
