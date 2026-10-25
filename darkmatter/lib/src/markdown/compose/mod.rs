//! Compose pipeline for markdown document preparation and transclusion.
//!
//! This module provides the `compose()` family of methods on `Markdown`
//! for running operations in four phases:
//!
//! Before any operation, a passive **call check** rejects the document when an
//! authored call on the composed surface is proven wrong (category 5) against
//! its schema's declared types, in any branch; nothing has been evaluated yet.
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
//! ## Frontmatter-surface projection
//!
//! [`ComposeOptions::only_frontmatter_surface`] composes a document's effective
//! frontmatter — interpolation, schema coercion (and verdict, unless deferred),
//! and approved `$(...)` expansion — while leaving the body exactly as authored.
//! It never dereferences a `::file`/`::url`/`::code` transclusion or a
//! prologue/epilogue, never parses or runs `::shell`/`::shell-block`, never
//! evaluates `::block when`, and never prefetches remote transclusions; its
//! pre-approval check uses [`collect_frontmatter_shell_commands`] instead of
//! the condition-blind graph walk. Keys excluded by the caller keep their
//! `{{ }}` spans.
//!
//! It serves staged initialization: a caller reads the frontmatter (for
//! example a lifecycle surface) that drives a step which creates files the body
//! will include, runs that step, and only then composes the full document. The
//! projected body is not a prompt. Full composition and
//! [`Markdown::compose_preflight`] are unchanged and still fail on a missing
//! transclusion target.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::Markdown;
//! use darkmatter::markdown::compose::{
//!     ComposeOperation, ComposeOptions, ComposeRequest, RequestSnapshot,
//! };
//!
//! let content = "# Hello\nWorld";
//! let md: Markdown = content.into();
//! // A library call names its request directory; a binary uses
//! // `RequestSnapshot::from_process()`.
//! let snapshot = RequestSnapshot::new(std::env::temp_dir());
//!
//! // Transform with default options (all operations enabled)
//! let request = ComposeRequest::prepare(ComposeOptions::new(), &snapshot).unwrap();
//! let (composed, report) = md.compose_with(&request).unwrap();
//!
//! // Transform with specific operations disabled
//! let options = ComposeOptions::new()
//!     .disable(ComposeOperation::Cleanup)
//!     .disable(ComposeOperation::Normalization);
//! let request = ComposeRequest::prepare(options, &snapshot).unwrap();
//! let (composed, report) = md.compose_with(&request).unwrap();
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

pub(crate) mod body_origin;
pub(crate) mod cache;
mod call_precheck;
pub mod conditions;
pub mod context;
mod frontmatter_interpolation;
pub(crate) use frontmatter_interpolation::locate_frontmatter_span;
pub(crate) mod frontmatter_shell_expansion;
pub(crate) mod glob_listing;
pub(crate) mod icmp;
pub(crate) mod indent;
pub(crate) mod parse_utils;
pub(crate) mod perf;
pub mod policy;
pub mod report_diagnostics;
pub(crate) mod pipeline;
pub(crate) mod schema_validation;
pub use schema_validation::effective_schema_for;
pub mod subtree;
mod unknown_identifiers;
mod util;
pub(crate) mod value_origin;

#[cfg(test)]
mod type_tests;

pub mod block_pairs;
pub mod directives_api;
pub mod directive_targets;
pub mod expression;
pub mod file_links;
pub(crate) mod inline;
pub mod interpolation;
pub(crate) mod link_normalization;
pub(crate) mod link_resolve;
pub(crate) mod nested;
pub mod page_blocks;
pub mod preflight;
pub(crate) mod remote_fetch;
pub mod remote;
pub mod replacement;
pub mod shell_blocks;
pub mod shell_expansion;
pub mod target_chain;
pub mod toc_linking;
pub mod transclusion;

pub use biscuit_file::PathPosition;
pub use cache::{CacheAccessMode, CacheStats};
pub use context::{
    ContextAuthority, ContextCaptureEvidence, ContextExtension, ContextGroup,
    ContextMergeDiagnostic, ContextRequirements, CurrentAuthority, CurrentProvider, CurrentRefresh,
    DeferredCapabilities,
};
pub use file_links::FileLinksError;
pub use remote::{
    DEFAULT_REMOTE_CONCURRENCY, DiscoveredRemoteUrl, REMOTE_CONCURRENCY_ENV, RemoteFreshnessMode,
    RemoteReadConfig, RemoteReadError, RemoteUrlCatalog, RemoteUrlConsumer,
    resolve_remote_concurrency,
};
pub use remote_fetch::RemoteFetchStats;
pub use frontmatter_shell_expansion::{
    FRONTMATTER_SHELL_SUFFIXES, FrontmatterShellAction, FrontmatterShellBody,
    FrontmatterShellPipeline, FrontmatterShellSuffix, FrontmatterShellTernary,
    FrontmatterShellValue, ResolvedShellValue, ShellResultKind, ShellSuffixDescriptor,
    ShellSuffixError, authored_shell_template, check_frontmatter_shell_value, describe_suffix,
    execute_resolved_shell_values, execute_shell_values_with, expected_suffixes,
    parse_frontmatter_shell_suffixes, parse_frontmatter_shell_value_spanned,
};
pub use icmp::PlannedIcmpProbe;
pub use preflight::{ComposePreflightApprovals, ComposePreflightReport, ExtraShellSources, PreflightApprovalStats, collect_frontmatter_shell_commands, collect_shell_commands};
pub use shell_blocks::ShellBlockError;
pub use shell_expansion::ShellCommandOrigin;
pub use shell_expansion::ShellExpansionError;
pub use shell_expansion::ShellTimeoutBehavior;
pub use context::effective_state::{EffectiveState, EffectiveStateBuilder};
pub(crate) use context::effective_state::ResolvingLookup;
pub use context::options::{
    CallerInputRecord, CallerInputRecords, ComposeOptions, ComposeSource, caller_input_records_for_overrides,
    file_resolution_context_identity,
};
pub use context::request::{
    ComposeRequest, ContextBuildError, RequestSnapshot, build_resolution_context,
    build_resolution_context_with_catalog,
};
#[cfg(test)]
pub(crate) use context::request::test_support::{request as test_request, request_in as test_request_in};
pub use context::repository_scope_catalog;
pub(crate) use context::options::ReferenceGraphOptionsIdentity;
pub use context::report::{ComposeFailure, ComposeReport, ComposeWarning, SourceRange};
pub use context::runtime::ComposeContext;
pub use value_origin::{OverrideLayer, OverrideOrigin};
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
    abbreviate_path, find_target_range, prepare_frontmatter_for_compose,
};

use super::Markdown;
use super::types::MarkdownResult;
use tracing::instrument;

// The transclusion preparation/resolution types and engine live in
// `transclusion/engine.rs`; the driver below imports them via `transclusion::`.

// Re-export HeadingLevel for tests
#[cfg(test)]
pub use super::normalize::HeadingLevel;

/// The host callback of [`Markdown::compose_with_frontmatter_hook`]: the
/// root document's resolved frontmatter and the environment its body will
/// render with, which the callback may change.
pub type ResolvedFrontmatterHook<'h> = dyn FnMut(
        &crate::markdown::Frontmatter,
        &mut std::collections::HashMap<String, String>,
    ) -> MarkdownResult<()>
    + 'h;

impl Markdown {
    /// Composes the document for `request`.
    ///
    /// Returns a new `Markdown` document and a report of changes made.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::compose::{
    ///     ComposeOperation, ComposeOptions, ComposeRequest, RequestSnapshot,
    /// };
    ///
    /// let md: Markdown = "# Test\nContent".into();
    /// let options = ComposeOptions::new().disable(ComposeOperation::Normalization);
    /// let request =
    ///     ComposeRequest::prepare(options, &RequestSnapshot::new(std::env::temp_dir())).unwrap();
    ///
    /// let (composed, report) = md.compose_with(&request).unwrap();
    /// ```
    #[instrument(skip_all, fields(source = ?request.options().source))]
    pub fn compose_with(
        &self,
        request: &ComposeRequest,
    ) -> MarkdownResult<(Markdown, ComposeReport)> {
        let mut result = self.clone();
        let report = result.run_compose_pipeline(request)?;
        Ok((result, report))
    }

    /// [`compose_with`](Self::compose_with), keeping on failure the warnings
    /// the composition had already reported.
    ///
    /// ## Errors
    ///
    /// A boxed [`ComposeFailure`]: the error [`compose_with`](Self::compose_with)
    /// returns, with the failed documents' warnings, the outer document's
    /// first. A child that had already succeeded reported into its parent, so
    /// its warnings appear there.
    pub fn compose_with_warnings(&self, request: &ComposeRequest) -> Result<(Markdown, ComposeReport), Box<ComposeFailure>> {
        let mut result = self.clone();
        match result.run_root_pipeline_collecting(request.clone(), None) {
            (Ok(report), _) => Ok((result, report)),
            (Err(error), warnings) => Err(Box::new(ComposeFailure { error, warnings })),
        }
    }

    /// [`compose_with`](Self::compose_with), letting the host choose the
    /// environment the body renders with from the resolved frontmatter.
    ///
    /// `hook` is called once, for the request's root document only, after its
    /// frontmatter is fully resolved (both interpolation passes and every
    /// frontmatter `$( … )` command) and before any body stage runs. It
    /// receives that frontmatter and the request context's environment (the
    /// `env` root, and the `AGENT`/`MODEL` behind `ctx.agent`/`ctx.model`);
    /// whatever it leaves there is what the body, its transclusions, and its
    /// `as_markdown()` content read. Frontmatter values already resolved keep
    /// the environment they resolved with. Nothing runs twice: the hook adds
    /// no compose pass.
    ///
    /// ## Errors
    ///
    /// As [`compose_with`](Self::compose_with), plus any error `hook` returns,
    /// which ends the compose before the body renders.
    pub fn compose_with_frontmatter_hook(
        &self,
        request: &ComposeRequest,
        hook: &mut ResolvedFrontmatterHook<'_>,
    ) -> MarkdownResult<(Markdown, ComposeReport)> {
        let mut result = self.clone();
        let report = result.run_root_pipeline_hooked(request.clone(), Some(hook))?;
        Ok((result, report))
    }

    /// Composes the document over a request derived from an entry point's
    /// request, for an inline pass inside that entry point.
    pub(crate) fn compose_with_options(
        &self,
        options: ComposeRequest,
    ) -> MarkdownResult<(Markdown, ComposeReport)> {
        let mut result = self.clone();
        let report = result.run_root_pipeline(options)?;
        Ok((result, report))
    }
}

#[cfg(test)]
mod tests;
