//! Compose pipeline for markdown document preparation and transclusion.
//!
//! This module provides the `compose()` family of methods on `Markdown`
//! for running operations in three phases:
//!
//! **Inline Pre** (serial):
//! 0. **Frontmatter Interpolation** - Resolve `{{variable}}` in frontmatter values.
//!    When shell expansion is enabled, templated keys that reference
//!    shell-pending values (top-level `$(...)`) are deferred for a second
//!    interpolation pass after step 2 completes.
//! 1. **Schema Validation** - Validate frontmatter against `$schema` or
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

pub(crate) mod cache;
pub mod conditions;
pub mod context;
mod frontmatter_interpolation;
pub(crate) mod frontmatter_shell_expansion;
pub(crate) mod indent;
pub(crate) mod parse_utils;
pub(crate) mod perf;
mod schema_validation;
mod state;
mod types;

pub mod block_pairs;
pub mod expression;
pub mod file_links;
pub mod interpolation;
pub(crate) mod link_normalization;
pub(crate) mod link_resolve;
pub mod page_blocks;
pub(crate) mod remote_fetch;
pub mod remote;
pub mod replacement;
pub mod shell_blocks;
pub mod shell_expansion;
pub mod toc_linking;
pub mod transclusion;

pub use biscuit_file::PathPosition;
pub use cache::{CacheAccessMode, CacheFreshnessMode, CacheStats};
pub use context::ContextMergeDiagnostic;
pub use file_links::FileLinksError;
pub use remote::{
    DEFAULT_REMOTE_CONCURRENCY, DiscoveredRemoteUrl, REMOTE_CONCURRENCY_ENV, RemoteFreshnessMode,
    RemoteReadConfig, RemoteReadError, RemoteUrlCatalog, RemoteUrlConsumer,
    resolve_remote_concurrency,
};
pub use remote_fetch::RemoteFetchStats;
pub use shell_blocks::ShellBlockError;
pub use shell_expansion::ShellCommandOrigin;
pub use shell_expansion::ShellExpansionError;
pub use shell_expansion::ShellTimeoutBehavior;
pub use state::{EffectiveState, EffectiveStateBuilder};
pub use toc_linking::TocLinkingError;
pub use transclusion::TransclusionError;
pub use types::{
    ComposeContext, ComposeOperation, ComposeOperationSet, ComposeOptions, ComposePerfMetric,
    ComposePerfReport, ComposePhase, ComposeReport, ComposeSource, ComposeStage, ComposeWarning,
    ShellCommandSpan, SourceRange, redact_shell_command,
};

// Internal re-exports for crate modules that still use TransclusionOptions
pub(crate) use types::TransclusionOptions;

use super::Markdown;
use super::cleanup;
use super::normalize::{self, NormalizationError};
use super::types::{MarkdownError, MarkdownResult};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, instrument, trace, warn};

use cache::operation::CacheableOperation;
use shell_expansion::{apply_replacements_in_reverse, execute_directive_detailed};

/// Shorten an absolute path for display in diagnostics.
///
/// Tries to make the path relative to the git repo root discovered by
/// walking up from the file's parent directory. Falls back to `~/…` when
/// the path is under the user's home directory, or the absolute path
/// otherwise.
fn abbreviate_path(path: &Path) -> String {
    // Try git repo root first (walk up looking for .git)
    if let Some(root) = find_git_root_from(path)
        && let Ok(rel) = path.strip_prefix(&root)
    {
        return rel.display().to_string();
    }

    // Fall back to ~/… for paths under HOME
    if let Some(home) = dirs::home_dir()
        && let Ok(rel) = path.strip_prefix(&home)
    {
        return format!("~/{}", rel.display());
    }

    path.display().to_string()
}

/// Walk up from `start` (or its parent if it's a file) looking for a `.git`
/// directory, returning the repo root if found.
pub(crate) fn find_git_root_from(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Helper to find target range within content.
pub(crate) fn find_target_range(
    content: &str,
    record: &crate::markdown::reference::ReferenceRecord,
    raw_target: &str,
) -> Option<(usize, usize)> {
    let span = &record.origin.span;
    if span.end > content.len() {
        trace!(
            "find_target_range: span.end {} > content.len {}",
            span.end,
            content.len()
        );
        return None;
    }
    let outer_text = &content[span.clone()];

    // Try attribute-aware search for HTML syntax first
    if let Some(attr_name) = get_attribute_name_for_syntax(&record.origin.syntax) {
        // Try searching for attr="target" or attr = "target" (with optional whitespace)
        let patterns = [
            format!(r#"{}="{}""#, attr_name, raw_target),
            format!("{}='{}'", attr_name, raw_target),
        ];
        for pattern in &patterns {
            if let Some(idx) = outer_text.find(pattern) {
                let mut actual_idx = idx + attr_name.len() + 1; // skip past attr_name=
                // Skip optional whitespace after =
                while actual_idx < outer_text.len() && outer_text[actual_idx..].starts_with(' ') {
                    actual_idx += 1;
                }
                let start = span.start + actual_idx + 1; // skip past quote
                let end = start + raw_target.len();
                trace!(
                    "find_target_range: attribute-aware match for '{}' in {:?} at {}",
                    raw_target, record.origin.syntax, start
                );
                return Some((start, end));
            }
        }

        // Try HTML-encoded form (for entities like &amp;)
        let encoded = html_escape::encode_quoted_attribute(raw_target);
        if encoded != raw_target {
            let patterns = [
                format!(r#"{}="{}""#, attr_name, encoded),
                format!("{}='{}'", attr_name, encoded),
            ];
            for pattern in &patterns {
                if let Some(idx) = outer_text.find(pattern.as_str()) {
                    let actual_idx = idx + attr_name.len() + 1;
                    // Skip optional whitespace after =
                    let mut quote_start = actual_idx;
                    while quote_start < outer_text.len()
                        && outer_text[quote_start..].starts_with(' ')
                    {
                        quote_start += 1;
                    }
                    let start = span.start + quote_start + 1; // skip past quote
                    let end = start + encoded.len();
                    trace!(
                        "find_target_range: HTML-encoded match for '{}' in {:?} at {}",
                        raw_target, record.origin.syntax, start
                    );
                    return Some((start, end));
                }
            }
        }
    }

    // Fallback: search for raw target string with context check
    let mut start_idx = 0;
    while let Some(idx) = outer_text[start_idx..].find(raw_target) {
        let actual_idx = start_idx + idx;

        if actual_idx > 0 {
            let prev_char = outer_text[..actual_idx].chars().next_back();
            trace!(
                "find_target_range: found '{}' at {}, prev_char: {:?}",
                raw_target, actual_idx, prev_char
            );
            if matches!(prev_char, Some('(' | '=' | '"' | '\'' | '<')) {
                let start = span.start + actual_idx;
                let end = start + raw_target.len();
                return Some((start, end));
            }
        } else {
            trace!("find_target_range: found '{}' at 0", raw_target);
            // Edge case: if raw_target is the exact span, match it.
            let start = span.start + actual_idx;
            let end = start + raw_target.len();
            return Some((start, end));
        }

        start_idx = actual_idx + 1;
    }
    trace!(
        "find_target_range: failed to find '{}' in '{}'",
        raw_target, outer_text
    );
    None
}

/// Maps a ReferenceSyntax to its target attribute name for HTML-aware matching.
fn get_attribute_name_for_syntax(
    syntax: &crate::markdown::reference::ReferenceSyntax,
) -> Option<&'static str> {
    use crate::markdown::reference::ReferenceSyntax;
    match syntax {
        ReferenceSyntax::HtmlAnchor | ReferenceSyntax::HtmlLinkTag => Some("href"),
        ReferenceSyntax::HtmlImage
        | ReferenceSyntax::HtmlVideoTag
        | ReferenceSyntax::HtmlAudioTag
        | ReferenceSyntax::HtmlSourceTag
        | ReferenceSyntax::HtmlIframeTag
        | ReferenceSyntax::HtmlScriptTag => Some("src"),
        _ => None,
    }
}

/// Applies pre-effective-state frontmatter preparation shared by runtime
/// compose and shell-command discovery.
///
/// This mutates frontmatter with external-state defaults and `--set`
/// overrides using the same rules the real compose pipeline uses. When
/// requested, it also captures the post-merge/pre-interpolation string
/// snapshot used for frontmatter shell executable provenance checks.
pub(crate) fn prepare_frontmatter_for_compose(
    markdown: &mut Markdown,
    options: &ComposeOptions,
    capture_pre_interpolation_snapshot: bool,
) -> Option<HashMap<String, String>> {
    // Apply external state as defaults using deep-merge: nested keys
    // from external state fill in missing values at every level, not
    // just top-level keys. Frontmatter values take precedence.
    if let Some(external) = options.external_state.as_ref() {
        let fm = markdown.frontmatter_mut().as_map_mut();
        let current = Value::Object(fm.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        let merged = state::deep_merge(external, &current);
        if let Value::Object(map) = merged {
            *fm = map.into_iter().collect();
        }
    }

    // Apply set overrides: unconditionally overwrite frontmatter keys.
    if let Some(overrides) = options.set_overrides.as_ref().and_then(Value::as_object) {
        let fm = markdown.frontmatter_mut().as_map_mut();
        for (key, value) in overrides {
            fm.insert(key.clone(), value.clone());
        }
    }

    capture_pre_interpolation_snapshot.then(|| {
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .filter_map(|(key, value)| value.as_str().map(|s| (key.clone(), s.to_string())))
            .collect()
    })
}

#[derive(Clone)]
enum PreparedTransclusion {
    FixedReplace {
        order: usize,
        span: std::ops::Range<usize>,
        replacement: String,
        report: ComposeReport,
    },
    FixedSection {
        order: usize,
        slot: SectionSlot,
        content: Option<String>,
        report: ComposeReport,
    },
    Markdown {
        order: usize,
        target: ApplyTarget,
        path: PathBuf,
        directive_options: transclusion::BlockOptions,
        insertion_context: Option<(usize, usize)>,
    },
    Code {
        order: usize,
        span: std::ops::Range<usize>,
        path: PathBuf,
        directive_options: transclusion::BlockOptions,
        line: usize,
    },
    RemoteFile {
        order: usize,
        target: ApplyTarget,
        url: url::Url,
        directive_options: transclusion::BlockOptions,
        insertion_context: Option<(usize, usize)>,
    },
    RemoteCode {
        order: usize,
        span: std::ops::Range<usize>,
        url: url::Url,
        directive_options: transclusion::BlockOptions,
        language: String,
    },
    Toc {
        order: usize,
        span: std::ops::Range<usize>,
        directive: toc_linking::TocLinkingDirective,
    },
    FileLinks {
        order: usize,
        span: std::ops::Range<usize>,
        /// The parsed directive. Discovery is deferred to the concurrent
        /// resolve stage so multiple directives' filesystem walks run in
        /// parallel, and each directive's tree is built from its discovered
        /// entries rather than walked a second time for rendering.
        directive: file_links::FileLinksDirective,
    },
}

#[derive(Clone, Copy)]
enum SectionSlot {
    Prologue(usize),
    Epilogue(usize),
}

#[derive(Clone)]
enum ApplyTarget {
    Replace(std::ops::Range<usize>),
    Section(SectionSlot),
}

struct ResolvedTransclusion {
    order: usize,
    target: ApplyTarget,
    content: Option<String>,
    report: ComposeReport,
    /// Source file for file-based transclusions (used for source map).
    source_file: Option<PathBuf>,
}

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

    /// Internal pipeline runner.
    fn run_compose_pipeline(&mut self, options: ComposeOptions) -> MarkdownResult<ComposeReport> {
        // Resolve persistent cache root if configured
        let persistent_root = options.cache_root.as_ref().map(|root| {
            cache::FileStore::resolve_cache_root(Some(root), options.cache_namespace.as_deref())
        });

        // Share a persistent store with the remote-fetch runtime so remote
        // artifacts are cached across runs alongside local compose artifacts.
        let remote_store = persistent_root.as_ref().and_then(|root| {
            cache::FileStore::new(root.clone())
                .map(std::sync::Arc::new)
                .ok()
        });
        let remote_fetch = remote_fetch::RemoteFetchRuntime::with_store(
            &options.remote_read_config,
            remote_store,
        );

        let mut runtime = shell_expansion::types::PipelineRuntime::with_remote_fetch(
            options.max_transclusion_depth,
            options.cache_access_mode,
            persistent_root,
            remote_fetch,
        );

        // Eagerly register discovered remote URLs and start fetching. The two
        // discovery paths gate independently: directive (`::file`/`::code`)
        // discovery requires the explicit remote-transclusion opt-in, while
        // URL-typed expression-function arguments (`frontmatter(url)`, …) are a
        // read-side capability enabled whenever remote reads are configured —
        // so a caller that allows a host but never enables transclusion can
        // still prefetch and read its expression URLs.
        if options.remote_reads_enabled() {
            let mut catalog = remote::RemoteUrlCatalog::new();

            if options.allow_remote_transclusion
                && options.is_enabled(ComposeOperation::BlockTransclusion)
            {
                let directives = transclusion::parse_directives(
                    &self.content,
                    self.source_context_for_errors(),
                )
                .unwrap_or_default();
                for entry in
                    remote::discover_remote_urls_from_directives(&directives, &options.source)
                {
                    catalog.add(entry);
                }
            }

            if options.is_enabled(ComposeOperation::Interpolation) {
                for entry in
                    remote::discover_remote_urls_from_expressions(&self.content, &options.source)
                {
                    catalog.add(entry);
                }
            }

            for url in catalog.urls() {
                runtime.remote_fetch.register_and_fetch(url);
            }
        }

        let mut report = self.run_compose_pipeline_internal(options, &mut runtime)?;
        report.cache_stats = Some(runtime.cache.stats());
        report.remote_fetch_stats = Some(runtime.remote_fetch.stats());
        Ok(report)
    }

    /// Internal recursive pipeline runner shared by root and child documents.
    ///
    /// Executes operations in three phases:
    /// 1. **Inline Pre** (serial): TextReplacement, PageBlocks, Interpolation, ShellExpansion, ShellBlocks
    /// 2. **Transclusion** (prepared serially, resolved concurrently): BlockTransclusion,
    ///    FrontmatterTransclusion, CodeTransclusion, TocLinking, FileLinks
    /// 3. **Inline Post** (serial): Cleanup, Normalization
    #[instrument(skip_all, fields(source = ?options.source))]
    pub(crate) fn run_compose_pipeline_internal(
        &mut self,
        options: ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
    ) -> MarkdownResult<ComposeReport> {
        let source_id = match &options.source {
            ComposeSource::Unknown => None,
            ComposeSource::File(path) => Some(
                std::fs::canonicalize(path)
                    .unwrap_or_else(|_| path.clone())
                    .to_string_lossy()
                    .to_string(),
            ),
            ComposeSource::Url(url) => Some(url.to_string()),
        };

        if let Some(id) = source_id.clone() {
            let path = match &options.source {
                ComposeSource::File(p) => p.clone(),
                ComposeSource::Url(u) => std::path::PathBuf::from(u.to_string()),
                ComposeSource::Unknown => std::path::PathBuf::from("<unknown>"),
            };
            runtime.transclusion.enter(id, path, 1)?;
        }

        let result = (|| {
            let mut report = ComposeReport::new();
            let mut perf = perf::PerfCollector::new(options.perf_enabled);

            let pre_interpolation_snapshot = prepare_frontmatter_for_compose(
                self,
                &options,
                options.is_enabled(ComposeOperation::FrontmatterShellExpansion),
            );

            let shell_expansion_enabled =
                options.is_enabled(ComposeOperation::FrontmatterShellExpansion);

            // Frontmatter Interpolation: resolve {{ }} in frontmatter values
            // before EffectiveState is built, since it mutates frontmatter
            // inputs that drive later stages.
            //
            // When shell expansion is also enabled, defer any templated key
            // that references a shell-pending value (top-level `$(...)`). A
            // second interpolation pass after shell expansion will resolve
            // those keys against the shell-expanded values.
            if options.is_enabled(ComposeOperation::FrontmatterInterpolation) {
                let fm_start = perf.is_enabled().then(std::time::Instant::now);
                let fm_report = frontmatter_interpolation::interpolate_frontmatter(
                    self.frontmatter_mut(),
                    options.context(),
                    options.fail_fast,
                    shell_expansion_enabled,
                )?;
                report.frontmatter_interpolations_applied = fm_report.replacements;
                report.warnings.extend(fm_report.warnings);
                if let Some(start) = fm_start {
                    perf.record(
                        perf::PerfMetricKind::FrontmatterInterpolation,
                        start.elapsed(),
                    );
                }
            }

            // Schema Validation: check frontmatter against $schema or baseline
            // AFTER frontmatter interpolation so template values like
            // `runtime_agent: '{{ env.AGENT }}'` are evaluated to their
            // resolved form before being checked. Runs BEFORE shell
            // expansion so the validator can fail-fast without triggering
            // (potentially expensive or side-effectful) shell commands when
            // the resolved frontmatter is invalid. This stage also coerces
            // schema-recognized scalars (e.g. the string "true" against a
            // boolean field) and writes the real types back into frontmatter,
            // so later stages and the composed output see coerced values.
            //
            // For frontmatter values that depend on shell-expanded inputs,
            // the second interpolation pass below will re-resolve them and
            // the prepare-time consumer (e.g. claudine's `prepare_*_with_schema`)
            // can re-validate the post-shell effective frontmatter.
            // Skipped by internal non-terminal passes (shell-command
            // discovery) that strip FrontmatterShellExpansion: validating a
            // still-literal `$(...)` value there would wrongly report it as a
            // final violation. See `ComposeOptions::skip_schema_validation`.
            if !options.skip_schema_validation {
                let sv_start = perf.is_enabled().then(std::time::Instant::now);
                schema_validation::run(self, &options)?;
                if let Some(start) = sv_start {
                    perf.record(perf::PerfMetricKind::SchemaValidation, start.elapsed());
                }
            }

            // Frontmatter Shell Expansion: execute $(cmd) in frontmatter values
            // before EffectiveState is built, since the expanded values must be
            // visible to all later stages.
            if shell_expansion_enabled {
                let fse_start = perf.is_enabled().then(std::time::Instant::now);
                let fse_ctx = self.source_context_for_errors();
                let fse_report = frontmatter_shell_expansion::execute_frontmatter_shell_expansion(
                    self.frontmatter_mut(),
                    &options,
                    runtime,
                    pre_interpolation_snapshot.as_ref(),
                    &fse_ctx,
                )?;
                report.frontmatter_shell_expansions_applied = fse_report.replacements;
                report.shell_approvals_used += fse_report.approvals_used;
                report.warnings.extend(fse_report.warnings);
                if let Some(start) = fse_start {
                    perf.record(
                        perf::PerfMetricKind::FrontmatterShellExpansion,
                        start.elapsed(),
                    );
                }

                // Second interpolation pass: templated keys that referenced
                // shell-pending values were deferred above. Now that shell
                // expansion has produced concrete values, resolve them.
                if options.is_enabled(ComposeOperation::FrontmatterInterpolation)
                    && fse_report.replacements > 0
                {
                    let fm_start = perf.is_enabled().then(std::time::Instant::now);
                    let fm_report = frontmatter_interpolation::interpolate_frontmatter(
                        self.frontmatter_mut(),
                        options.context(),
                        options.fail_fast,
                        false,
                    )?;
                    report.frontmatter_interpolations_applied += fm_report.replacements;
                    report.warnings.extend(fm_report.warnings);
                    if let Some(start) = fm_start {
                        perf.record(
                            perf::PerfMetricKind::FrontmatterInterpolation,
                            start.elapsed(),
                        );
                    }
                }
            }

            // Build effective state for replacement/interpolation and condition checks.
            let esb_start = perf.is_enabled().then(std::time::Instant::now);
            let effective_state = EffectiveStateBuilder::new()
                .with_frontmatter(
                    self.frontmatter()
                        .as_map()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                )
                .with_external_state(
                    options
                        .external_state
                        .clone()
                        .unwrap_or(Value::Object(Map::new())),
                )
                .with_merge_strategy(super::MergeStrategy::PreferDocument)
                .with_replace_parent_wins(options.replace_parent_wins)
                .with_context(options.context().clone())
                .with_allow_ctx_override(options.allow_ctx_override)
                .build()?;
            if let Some(start) = esb_start {
                perf.record(perf::PerfMetricKind::EffectiveStateBuild, start.elapsed());
            }

            // Convert ctx diagnostics to compose warnings
            let source_display = match &options.source {
                ComposeSource::File(p) => abbreviate_path(p),
                ComposeSource::Url(u) => u.to_string(),
                ComposeSource::Unknown => "unknown".to_string(),
            };
            for diag in effective_state.ctx_diagnostics() {
                let warning = match diag {
                    context::ContextMergeDiagnostic::UserCtxMerged { colliding_keys }
                        if colliding_keys.is_empty() =>
                    {
                        // No warning needed when merge succeeded without collisions
                        continue;
                    }
                    context::ContextMergeDiagnostic::UserCtxMerged { colliding_keys } => {
                        let keys_list = colliding_keys.join(", ");
                        ComposeWarning::new(
                            "context",
                            format!(
                                "the <blue>{source_display}</blue> document <i>defines</i> a <inverse>ctx</inverse> property and keys [<dim>{keys_list}</dim>] in the <inverse>ctx</inverse> dictionary conflict with those provided by Darkmatter's normal context dictionary!"
                            ),
                        )
                    }
                    context::ContextMergeDiagnostic::InvalidUserCtxReplaced => ComposeWarning::new(
                        "context",
                        "Document ctx was not an object; replaced with runtime context",
                    ),
                    context::ContextMergeDiagnostic::PartialRuntimeCapture { area, detail } => {
                        ComposeWarning::new(
                            "context",
                            format!("Partial runtime capture for {area}: {detail}"),
                        )
                    }
                };
                report.warnings.push(warning);
            }

            let mut transclusion_ran = false;
            for operation in ComposeOperation::default_order() {
                trace!(operation = ?operation, enabled = options.is_enabled(*operation), "compose: checking operation");
                if !options.is_enabled(*operation) {
                    continue;
                }

                info!(operation = ?operation, phase = ?operation.phase(), "compose: running operation");
                match operation.phase() {
                    ComposePhase::InlinePre => {
                        let op_start = perf.is_enabled().then(std::time::Instant::now);
                        self.run_inline_pre_operation(
                            *operation,
                            &effective_state,
                            &options,
                            runtime,
                            &mut report,
                            &mut perf,
                        )?;
                        if let Some(start) = op_start {
                            let kind = match operation {
                                ComposeOperation::FrontmatterInterpolation => {
                                    Some(perf::PerfMetricKind::FrontmatterInterpolation)
                                }
                                ComposeOperation::FrontmatterShellExpansion => {
                                    Some(perf::PerfMetricKind::FrontmatterShellExpansion)
                                }
                                ComposeOperation::TextReplacement => {
                                    Some(perf::PerfMetricKind::TextReplacement)
                                }
                                ComposeOperation::PageBlocks => {
                                    Some(perf::PerfMetricKind::PageBlocks)
                                }
                                ComposeOperation::Interpolation => {
                                    Some(perf::PerfMetricKind::Interpolation)
                                }
                                ComposeOperation::ShellExpansion => {
                                    Some(perf::PerfMetricKind::ShellExpansion)
                                }
                                ComposeOperation::ShellBlocks => {
                                    Some(perf::PerfMetricKind::ShellBlocks)
                                }
                                ComposeOperation::LinkResolve => {
                                    Some(perf::PerfMetricKind::LinkResolve)
                                }
                                _ => unreachable!(),
                            };
                            if let Some(kind) = kind {
                                perf.record(kind, start.elapsed());
                            }
                        }
                    }
                    ComposePhase::Transclusion => {
                        if transclusion_ran {
                            continue;
                        }

                        let enabled_transclusion_ops = ComposeOperation::default_order()
                            .iter()
                            .copied()
                            .filter(|op| {
                                op.phase() == ComposePhase::Transclusion && options.is_enabled(*op)
                            })
                            .collect::<Vec<_>>();

                        self.run_transclusion_phase(
                            &enabled_transclusion_ops,
                            &effective_state,
                            &options,
                            runtime,
                            &mut report,
                            &mut perf,
                        )?;
                        transclusion_ran = true;
                    }
                    ComposePhase::InlinePost => {
                        let op_start = perf.is_enabled().then(std::time::Instant::now);
                        self.run_inline_post_operation(*operation, &options, &mut report)?;
                        if let Some(start) = op_start {
                            let kind = match operation {
                                ComposeOperation::Cleanup => perf::PerfMetricKind::Cleanup,
                                ComposeOperation::Normalization => {
                                    perf::PerfMetricKind::Normalization
                                }
                                _ => unreachable!(),
                            };
                            perf.record(kind, start.elapsed());
                        }
                    }
                    ComposePhase::Finalization => {
                        if runtime.transclusion.depth() <= 1 {
                            let op_start = perf.is_enabled().then(std::time::Instant::now);
                            self.run_finalization_operation(*operation, &options, &mut report)?;
                            if let Some(start) = op_start {
                                let kind = match operation {
                                    ComposeOperation::LinkNormalization => {
                                        perf::PerfMetricKind::LinkNormalization
                                    }
                                    _ => unreachable!(),
                                };
                                perf.record(kind, start.elapsed());
                            }
                        }
                    }
                }
            }

            report.max_transclusion_depth = runtime.transclusion.deepest_seen;
            if perf.is_enabled() {
                perf.set_capture_timings(options.context().capture_timings().to_vec());
            }
            report.perf = perf.finish();
            Ok(report)
        })();

        if source_id.is_some() {
            runtime.transclusion.exit();
        }

        result
    }

    fn run_inline_pre_operation(
        &mut self,
        operation: ComposeOperation,
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
        perf: &mut perf::PerfCollector,
    ) -> MarkdownResult<()> {
        match operation {
            // FrontmatterInterpolation is handled before EffectiveState build,
            // not in the generic operation loop.
            ComposeOperation::FrontmatterInterpolation => Ok(()),
            // FrontmatterShellExpansion is handled before EffectiveState build,
            // not in the generic operation loop.
            ComposeOperation::FrontmatterShellExpansion => Ok(()),
            ComposeOperation::TextReplacement => {
                report.replacements_applied = self.run_replacement_stage(state, options);
                Ok(())
            }
            ComposeOperation::PageBlocks => {
                self.run_page_blocks_stage(state, options, runtime, report)
            }
            ComposeOperation::Interpolation => {
                report.interpolations_applied =
                    self.run_interpolation_stage(state, options, runtime, report)?;
                Ok(())
            }
            ComposeOperation::ShellExpansion => {
                self.run_shell_expansion_stage(options, runtime, report, perf)
            }
            ComposeOperation::ShellBlocks => {
                let sb_ctx = self.source_context_for_errors();
                shell_blocks::run_shell_blocks_stage_for_markdown(
                    &mut self.content,
                    options,
                    &mut runtime.shell,
                    report,
                    &sb_ctx,
                )
            }
            ComposeOperation::LinkResolve => link_resolve::link_resolve(self, options, report),
            _ => Ok(()),
        }
    }

    fn run_inline_post_operation(
        &mut self,
        operation: ComposeOperation,
        options: &ComposeOptions,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        match operation {
            ComposeOperation::Cleanup => {
                let original_content = self.content.clone();
                self.content = match options.list_spacing {
                    cleanup::ListSpacingMode::Normal => {
                        cleanup::cleanup_content_with_indent(&self.content, options.indent_size)
                    }
                    cleanup::ListSpacingMode::Compact => {
                        cleanup::cleanup_content_with_indent_compact(
                            &self.content,
                            options.indent_size,
                        )
                    }
                    cleanup::ListSpacingMode::Loose => cleanup::cleanup_content_with_indent_loose(
                        &self.content,
                        options.indent_size,
                    ),
                };
                report.cleanup_changed = self.content != original_content;
                Ok(())
            }
            ComposeOperation::Normalization => match self.run_normalization_stage() {
                Ok(norm_report) => {
                    if norm_report.has_changes() {
                        report.normalization_report = Some(norm_report);
                    }
                    Ok(())
                }
                Err(NormalizationError::LevelOverflow { .. }) if !options.fail_fast => {
                    report.add_warning(ComposeWarning::new(
                        "normalization",
                        "Skipped normalization: would overflow H6",
                    ));
                    Ok(())
                }
                Err(e) => Err(MarkdownError::Transform(format!(
                    "Normalization failed: {}",
                    e
                ))),
            },
            _ => Ok(()),
        }
    }

    fn run_finalization_operation(
        &mut self,
        operation: ComposeOperation,
        options: &ComposeOptions,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        match operation {
            ComposeOperation::LinkNormalization => {
                link_normalization::normalize_links(self, options, report)
            }
            _ => Ok(()),
        }
    }

    fn run_transclusion_phase(
        &mut self,
        operations: &[ComposeOperation],
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
        perf_collector: &mut perf::PerfCollector,
    ) -> MarkdownResult<()> {
        use rayon::prelude::*;

        if operations.is_empty() {
            return Ok(());
        }

        info!(operations = ?operations, "compose: starting transclusion phase");
        let parse_start = perf_collector.is_enabled().then(std::time::Instant::now);

        let parsed_directives = if operations.iter().any(|op| {
            matches!(
                op,
                ComposeOperation::BlockTransclusion | ComposeOperation::CodeTransclusion
            )
        }) {
            Some(transclusion::parse_directives(
                &self.content,
                self.source_context_for_errors(),
            )?)
        } else {
            None
        };

        let frontmatter_refs = if operations.contains(&ComposeOperation::FrontmatterTransclusion) {
            Some(transclusion::parse_frontmatter_refs(
                self.frontmatter().as_map(),
                self.source_context_for_errors(),
            )?)
        } else {
            None
        };

        let toc_directives = if operations.contains(&ComposeOperation::TocLinking) {
            Some(toc_linking::parse_directives(&self.content)?)
        } else {
            None
        };

        let file_links_directives = if operations.contains(&ComposeOperation::FileLinks) {
            Some(file_links::parse_file_links_directives(&self.content)?)
        } else {
            None
        };

        if let Some(start) = parse_start {
            perf_collector.record(perf::PerfMetricKind::TransclusionParse, start.elapsed());
        }

        let prepare_start = perf_collector.is_enabled().then(std::time::Instant::now);

        let mut prepared = Vec::new();
        let mut next_order = 0usize;

        for operation in operations {
            match operation {
                ComposeOperation::BlockTransclusion => {
                    if let Some(directives) = parsed_directives.as_ref() {
                        self.prepare_block_transclusions(
                            directives,
                            transclusion::DirectiveKind::File,
                            state,
                            options,
                            &runtime.remote_fetch,
                            report,
                            &mut prepared,
                            &mut next_order,
                        )?;
                    }
                }
                ComposeOperation::FrontmatterTransclusion => {
                    if let Some(refs) = frontmatter_refs.as_ref() {
                        self.prepare_frontmatter_transclusions(
                            refs,
                            state,
                            options,
                            &runtime.remote_fetch,
                            report,
                            &mut prepared,
                            &mut next_order,
                        )?;
                    }
                }
                ComposeOperation::CodeTransclusion => {
                    if let Some(directives) = parsed_directives.as_ref() {
                        self.prepare_block_transclusions(
                            directives,
                            transclusion::DirectiveKind::Code,
                            state,
                            options,
                            &runtime.remote_fetch,
                            report,
                            &mut prepared,
                            &mut next_order,
                        )?;
                    }
                }
                ComposeOperation::TocLinking => {
                    if let Some(directives) = toc_directives.as_ref() {
                        self.prepare_toc_transclusions(
                            directives,
                            report,
                            &mut prepared,
                            &mut next_order,
                        );
                    }
                }
                ComposeOperation::FileLinks => {
                    if let Some(directives) = file_links_directives.as_ref() {
                        self.prepare_file_links_transclusions(
                            directives,
                            &mut prepared,
                            &mut next_order,
                        );
                    }
                }
                _ => {}
            }
        }

        if let Some(start) = prepare_start {
            perf_collector.record(perf::PerfMetricKind::TransclusionPrepare, start.elapsed());
        }

        if prepared.is_empty() {
            return Ok(());
        }

        let resolve_start = perf_collector.is_enabled().then(std::time::Instant::now);

        let runtime_mutex = std::sync::Mutex::new(runtime);
        let results = prepared
            .into_par_iter()
            .map(|item| self.resolve_prepared_transclusion(item, state, options, &runtime_mutex))
            .collect::<Vec<_>>();

        debug!(
            resolved = results.len(),
            "compose: transclusion resolution complete"
        );
        if let Some(start) = resolve_start {
            perf_collector.record(perf::PerfMetricKind::TransclusionResolve, start.elapsed());
        }

        let apply_start = perf_collector.is_enabled().then(std::time::Instant::now);

        let mut replacements = Vec::new();
        let prologue_count = frontmatter_refs
            .as_ref()
            .map_or(0, |refs| refs.prologue.len());
        let epilogue_count = frontmatter_refs
            .as_ref()
            .map_or(0, |refs| refs.epilogue.len());
        let mut prologue_sections = vec![None; prologue_count];
        let mut epilogue_sections = vec![None; epilogue_count];

        for result in results {
            let resolved = match result {
                Ok(resolved) => resolved,
                Err(error) => {
                    let is_structural = matches!(
                        error,
                        MarkdownError::Transclusion(ref inner)
                            if matches!(
                                inner.as_ref(),
                                transclusion::TransclusionError::CycleDetected { .. }
                                    | transclusion::TransclusionError::MaxDepthExceeded { .. }
                                    | transclusion::TransclusionError::RemoteFetchFailed { .. }
                            )
                    );
                    if is_structural || options.fail_fast {
                        return Err(error);
                    }
                    report.add_warning(ComposeWarning::new("transclusion", error.to_string()));
                    continue;
                }
            };

            report.merge(resolved.report);

            match resolved.target {
                ApplyTarget::Replace(span) => {
                    replacements.push((
                        resolved.order,
                        span,
                        resolved.content.unwrap_or_default(),
                        resolved.source_file,
                    ));
                }
                ApplyTarget::Section(SectionSlot::Prologue(index)) => {
                    prologue_sections[index] = resolved.content;
                }
                ApplyTarget::Section(SectionSlot::Epilogue(index)) => {
                    epilogue_sections[index] = resolved.content;
                }
            }
        }

        if !replacements.is_empty() {
            replacements.sort_by(|left, right| {
                right
                    .1
                    .start
                    .cmp(&left.1.start)
                    .then_with(|| right.0.cmp(&left.0))
            });
            let mut next = self.content.clone();
            for (_, span, replacement, _) in &replacements {
                next.replace_range(span.clone(), replacement);
            }
            self.content = next;

            // Build source map: compute final byte positions for each file transclusion.
            // Sort forward by original span start and track cumulative offset.
            {
                let mut forward: Vec<_> = replacements
                    .iter()
                    .map(|(_, span, content, source)| (span.clone(), content.len(), source.clone()))
                    .collect();
                forward.sort_by_key(|(span, _, _)| span.start);

                let mut offset: isize = 0;
                for (span, content_len, source_file) in forward {
                    let final_start = (span.start as isize + offset) as usize;
                    let final_end = final_start + content_len;

                    if let Some(file) = source_file {
                        report.source_map.push(SourceRange {
                            byte_start: final_start,
                            byte_end: final_end,
                            source_file: file,
                            source_start_line: 1,
                        });
                    }

                    offset += content_len as isize - (span.end - span.start) as isize;
                }
            }
        }

        if prologue_count > 0 || epilogue_count > 0 {
            let mut sections = Vec::new();
            sections.extend(
                prologue_sections
                    .into_iter()
                    .flatten()
                    .filter(|part| !part.trim().is_empty()),
            );
            sections.push(self.content.clone());
            sections.extend(
                epilogue_sections
                    .into_iter()
                    .flatten()
                    .filter(|part| !part.trim().is_empty()),
            );
            self.content = sections.join("\n\n");
        }

        if let Some(start) = apply_start {
            perf_collector.record(perf::PerfMetricKind::TransclusionApply, start.elapsed());
        }

        Ok(())
    }

    /// Runs the text replacement stage.
    ///
    /// Applies text replacements from the `replace` map in effective state.
    /// See [`replacement::apply_replacements`] for algorithm details.
    fn run_replacement_stage(&mut self, state: &EffectiveState, options: &ComposeOptions) -> usize {
        let (new_content, count) = if let Some(one_off) = &options.one_off_replace {
            let merged_replace = state::merge_replace_maps(state.get_replace_map(), Some(one_off));
            let mut frontmatter = HashMap::new();
            frontmatter.insert("replace".to_string(), Value::Object(merged_replace));
            let scoped_state = EffectiveStateBuilder::new()
                .with_frontmatter(frontmatter)
                .with_context(options.context().clone())
                .build()
                .expect("replace-only state has no user ctx");
            replacement::apply_replacements(&self.content, &scoped_state)
        } else {
            replacement::apply_replacements(&self.content, state)
        };
        if count > 0 {
            self.content = new_content;
        }
        debug!(count, "compose: text replacements applied");
        count
    }

    /// Runs the interpolation stage.
    ///
    /// Finds `{{ expression }}` patterns in content and evaluates them
    /// against the effective state. Inline code spans (single backticks)
    /// are always scanned. Fenced and indented code blocks are skipped
    /// by default; set `interpolate_code_blocks` (via options or
    /// frontmatter) to scan them too.
    fn run_interpolation_stage(
        &mut self,
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime: &shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
    ) -> MarkdownResult<usize> {
        use interpolation::{Evaluator, ScanMode, interpolate_text};

        let scan_mode = if self.resolve_interpolate_code_blocks(options) {
            ScanMode::Plain
        } else {
            ScanMode::MarkdownAware
        };

        // Wrap the effective state with a resolution context so read-side
        // expression functions (`frontmatter`, `file_exists`, `markdown_title`,
        // …) resolve filesystem paths and — when remote reads are enabled —
        // HTTP(S) URL arguments through the run's remote-fetch runtime. Bare
        // `EffectiveState` returns no resolution context and those functions
        // never run.
        let lookup = state::ResolvingLookup::new(
            state,
            options.expression_resolution_context(&runtime.remote_fetch),
        );
        let evaluator = Evaluator::new(&lookup);
        let result = interpolate_text(
            &self.content,
            &evaluator,
            scan_mode,
            options.fail_fast,
            "interpolation",
        )?;

        if result.replacements > 0 {
            self.content = result.output;
        }
        report.warnings.extend(result.warnings);
        debug!(
            count = result.replacements,
            "compose: interpolations applied"
        );
        Ok(result.replacements)
    }

    /// Runs the normalization stage.
    ///
    /// Uses `None` as target level, which means headings are not re-leveled
    /// but the document structure is validated.
    fn run_normalization_stage(
        &mut self,
    ) -> Result<normalize::NormalizationReport, NormalizationError> {
        debug!("compose: running normalization");
        let (new_content, report) = normalize::normalize(&self.content, None)?;
        self.content = new_content;
        Ok(report)
    }

    /// Runs Stage 1 shell expansion directives.
    fn run_shell_expansion_stage(
        &mut self,
        options: &ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
        perf: &mut perf::PerfCollector,
    ) -> MarkdownResult<()> {
        let directives =
            shell_expansion::parse_directives(&self.content, self.source_context_for_errors())?;
        debug!(
            directive_count = directives.len(),
            "compose: shell expansion directives found"
        );
        if directives.is_empty() {
            return Ok(());
        }

        let shell_opts = options.shell_options();
        let policy_paths = shell_expansion::resolve_policy_paths(&shell_opts, &options.source)?;
        runtime.shell.ensure_loaded(&policy_paths)?;

        let mut replacements = Vec::new();

        for directive in directives {
            let span_start = perf.is_enabled().then(std::time::Instant::now);
            let execution =
                execute_directive_detailed(&directive, options, &policy_paths, &mut runtime.shell)?;
            if let Some(start) = span_start {
                perf.record_shell_span(ShellCommandSpan {
                    command_display: redact_shell_command(&directive.raw_command),
                    command_hash: format!(
                        "{:016x}",
                        biscuit_hash::xx_hash(&directive.raw_command)
                    ),
                    elapsed: start.elapsed(),
                });
            }
            // Re-indent multi-line output to the directive's column so generated
            // lines stay nested under the surrounding list or block quote.
            let output = indent::indent_text(&execution.combined_output(), &directive.indent, None);
            replacements.push((directive.span.clone(), output));
            report.warnings.extend(execution.warnings);
            report.shell_expansions_applied += 1;
        }

        apply_replacements_in_reverse(&mut self.content, replacements);
        report.shell_approvals_used += runtime.shell.take_recent_approval_count();
        Ok(())
    }

    /// Runs page blocks (conditional content regions).
    fn run_page_blocks_stage(
        &mut self,
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime: &shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        debug!("compose: running page blocks");
        let source = self.source_context_for_errors();
        let regions = page_blocks::parser::parse_page_blocks(&self.content, source.clone())?;
        if regions.is_empty() {
            return Ok(());
        }

        // Warn for unknown options
        fn warn_unknown_options(region: &page_blocks::PageBlockRegion, report: &mut ComposeReport) {
            for unknown in &region.options.unknown_options {
                report.add_warning(
                    ComposeWarning::new(
                        "page_blocks",
                        format!("Unknown page block option: '{}'", unknown),
                    )
                    .at_line(region.start_line),
                );
            }
            for child in &region.children {
                warn_unknown_options(child, report);
            }
        }
        for region in &regions {
            warn_unknown_options(region, report);
        }

        let lookup = state::ResolvingLookup::new(
            state,
            options.expression_resolution_context(&runtime.remote_fetch),
        );
        self.content = page_blocks::engine::render_page_blocks(
            &self.content,
            &regions,
            &lookup,
            report,
            source,
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_block_transclusions(
        &self,
        directives: &[transclusion::BlockDirective],
        kind: transclusion::DirectiveKind,
        state: &EffectiveState,
        options: &ComposeOptions,
        remote_fetch: &remote_fetch::RemoteFetchRuntime,
        report: &mut ComposeReport,
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
    ) -> MarkdownResult<()> {
        let ignore_invalid = self.resolve_ignore_invalid(options);
        let transclusion_opts = options.transclusion_options();

        for directive in directives.iter().filter(|directive| match kind {
            transclusion::DirectiveKind::Code => {
                directive.kind == transclusion::DirectiveKind::Code
            }
            _ => directive.kind != transclusion::DirectiveKind::Code,
        }) {
            for unknown in &directive.options.unknown_options {
                report.add_warning(
                    ComposeWarning::new(
                        "transclusion",
                        format!(
                            "Unknown option '{}' on ::{} directive; ignoring",
                            unknown,
                            directive.kind.as_str()
                        ),
                    )
                    .at_line(directive.line),
                );
            }

            for error in &directive.options.deferred_set_errors {
                match error {
                    transclusion::DeferredSetError::InvalidAssignment { raw, reason, line } => {
                        if options.allow_invalid_frontmatter_assignment {
                            report.add_warning(
                                ComposeWarning::new(
                                    "transclusion",
                                    format!(
                                        "Invalid frontmatter assignment on ::{} directive at line {}: {} (value: {})",
                                        directive.kind.as_str(),
                                        line,
                                        reason,
                                        raw
                                    ),
                                )
                                .at_line(*line),
                            );
                        } else {
                            return Err(
                                transclusion::TransclusionError::InvalidFrontmatterAssignment {
                                    ctx: Box::new(self.source_context_for_errors()),
                                    line: *line,
                                    raw: raw.clone(),
                                    reason: reason.clone(),
                                }
                                .into(),
                            );
                        }
                    }
                    transclusion::DeferredSetError::ReassignedProperty { name } => {
                        if options.allow_reassigned_frontmatter_property {
                            report.add_warning(
                                ComposeWarning::new(
                                    "transclusion",
                                    format!(
                                        "Duplicate set property '{}' on ::{} directive at line {}; rightmost assignment wins",
                                        name,
                                        directive.kind.as_str(),
                                        directive.line
                                    ),
                                )
                                .at_line(directive.line),
                            );
                        } else {
                            return Err(
                                transclusion::TransclusionError::InvalidReassignedFrontmatterProperty {
                                    ctx: Box::new(self.source_context_for_errors()),
                                    line: directive.line,
                                    name: name.clone(),
                                }
                                .into(),
                            );
                        }
                    }
                }
            }

            if let Some(expr) = &directive.options.when_expr {
                let lookup = state::ResolvingLookup::new(
                    state,
                    options.expression_resolution_context(remote_fetch),
                );
                let should_include = transclusion::evaluate_condition(
                    expr,
                    &lookup,
                    directive.line,
                    self.source_context_for_errors(),
                )?;
                if !should_include {
                    let mut fixed_report = ComposeReport::new();
                    fixed_report.transclusions_skipped = 1;
                    prepared.push(PreparedTransclusion::FixedReplace {
                        order: *next_order,
                        span: directive.span.clone(),
                        replacement: String::new(),
                        report: fixed_report,
                    });
                    *next_order += 1;
                    continue;
                }
            }

            let target = transclusion::normalize_reference_token(&directive.raw_target);
            let resolved = match transclusion::resolve_target(
                directive.kind,
                &target,
                &transclusion_opts,
                &options.source,
                directive.line,
                self.source_context_for_errors(),
            ) {
                Ok(resolved) => resolved,
                Err(err) if ignore_invalid => {
                    let mut fixed_report = ComposeReport::new();
                    fixed_report.transclusions_skipped = 1;
                    fixed_report.add_warning(
                        ComposeWarning::new("transclusion", err.to_string())
                            .at_line(directive.line),
                    );
                    prepared.push(PreparedTransclusion::FixedReplace {
                        order: *next_order,
                        span: directive.span.clone(),
                        replacement: String::new(),
                        report: fixed_report,
                    });
                    *next_order += 1;
                    continue;
                }
                Err(err) => return Err(err.into()),
            };

            match resolved {
                transclusion::ResolvedTarget::File { path, .. } => {
                    let item = if directive.kind == transclusion::DirectiveKind::Code {
                        PreparedTransclusion::Code {
                            order: *next_order,
                            span: directive.span.clone(),
                            path,
                            directive_options: directive.options.clone(),
                            line: directive.line,
                        }
                    } else {
                        PreparedTransclusion::Markdown {
                            order: *next_order,
                            target: ApplyTarget::Replace(directive.span.clone()),
                            path,
                            directive_options: directive.options.clone(),
                            insertion_context: Some((directive.span.start, directive.line)),
                        }
                    };
                    prepared.push(item);
                    *next_order += 1;
                }
                transclusion::ResolvedTarget::Url { url, .. }
                    if options.allow_remote_transclusion =>
                {
                    // The eager pre-scan only sees URLs present in the original
                    // content. A directive whose URL was produced by an earlier
                    // compose phase (interpolation, replacement) reaches here
                    // unregistered, so register it now to start its fetch and
                    // keep point-of-use from failing with "not registered".
                    remote_fetch.register_nested(url.clone());
                    if directive.kind == transclusion::DirectiveKind::Code {
                        let language = transclusion::infer_language(
                            std::path::Path::new(url.path()),
                            &options.code_fallback_language,
                        );
                        prepared.push(PreparedTransclusion::RemoteCode {
                            order: *next_order,
                            span: directive.span.clone(),
                            url,
                            directive_options: directive.options.clone(),
                            language,
                        });
                    } else {
                        prepared.push(PreparedTransclusion::RemoteFile {
                            order: *next_order,
                            target: ApplyTarget::Replace(directive.span.clone()),
                            url,
                            directive_options: directive.options.clone(),
                            insertion_context: Some((directive.span.start, directive.line)),
                        });
                    }
                    *next_order += 1;
                }
                transclusion::ResolvedTarget::Url { url, .. } if ignore_invalid => {
                    let mut fixed_report = ComposeReport::new();
                    fixed_report.transclusions_skipped = 1;
                    fixed_report.add_warning(
                        ComposeWarning::new(
                            "transclusion",
                            format!(
                                "Skipping URL transclusion '{}': remote execution disabled",
                                url
                            ),
                        )
                        .at_line(directive.line),
                    );
                    prepared.push(PreparedTransclusion::FixedReplace {
                        order: *next_order,
                        span: directive.span.clone(),
                        replacement: String::new(),
                        report: fixed_report,
                    });
                    *next_order += 1;
                }
                transclusion::ResolvedTarget::Url { url, .. } => {
                    return Err(transclusion::TransclusionError::UrlExecutionDisabled {
                        url: url.to_string(),
                    }
                    .into());
                }
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_frontmatter_transclusions(
        &self,
        refs: &transclusion::FrontmatterRefs,
        _state: &EffectiveState,
        options: &ComposeOptions,
        remote_fetch: &remote_fetch::RemoteFetchRuntime,
        _report: &mut ComposeReport,
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
    ) -> MarkdownResult<()> {
        for (index, reference) in refs.prologue.iter().enumerate() {
            self.prepare_frontmatter_reference(
                reference,
                SectionSlot::Prologue(index),
                options,
                remote_fetch,
                prepared,
                next_order,
            )?;
        }

        for (index, reference) in refs.epilogue.iter().enumerate() {
            self.prepare_frontmatter_reference(
                reference,
                SectionSlot::Epilogue(index),
                options,
                remote_fetch,
                prepared,
                next_order,
            )?;
        }

        Ok(())
    }

    fn prepare_frontmatter_reference(
        &self,
        reference: &str,
        slot: SectionSlot,
        options: &ComposeOptions,
        remote_fetch: &remote_fetch::RemoteFetchRuntime,
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
    ) -> MarkdownResult<()> {
        if !transclusion::is_url_like(reference) && !transclusion::is_file_like_reference(reference)
        {
            prepared.push(PreparedTransclusion::FixedSection {
                order: *next_order,
                slot,
                content: Some(reference.to_string()),
                report: ComposeReport::new(),
            });
            *next_order += 1;
            return Ok(());
        }

        let kind = if transclusion::is_url_like(reference) {
            transclusion::DirectiveKind::Url
        } else {
            transclusion::DirectiveKind::File
        };
        let ignore_invalid = self.resolve_ignore_invalid(options);
        let transclusion_opts = options.transclusion_options();

        let resolved = match transclusion::resolve_target(
            kind,
            reference,
            &transclusion_opts,
            &options.source,
            0,
            self.source_context_for_errors(),
        ) {
            Ok(resolved) => resolved,
            Err(err) if ignore_invalid => {
                let mut fixed_report = ComposeReport::new();
                fixed_report.transclusions_skipped = 1;
                fixed_report.add_warning(ComposeWarning::new("transclusion", err.to_string()));
                prepared.push(PreparedTransclusion::FixedSection {
                    order: *next_order,
                    slot,
                    content: None,
                    report: fixed_report,
                });
                *next_order += 1;
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        };

        match resolved {
            transclusion::ResolvedTarget::File { path, .. } => {
                prepared.push(PreparedTransclusion::Markdown {
                    order: *next_order,
                    target: ApplyTarget::Section(slot),
                    path,
                    directive_options: transclusion::BlockOptions::default(),
                    insertion_context: None,
                });
                *next_order += 1;
            }
            transclusion::ResolvedTarget::Url { url, .. }
                if options.allow_remote_transclusion =>
            {
                // Frontmatter `prologue`/`epilogue` URLs are not seen by the
                // eager pre-scan (it only covers directives and expression
                // arguments), so register the slot here. Without this,
                // `PreparedTransclusion::RemoteFile` fails at point-of-use with
                // "URL was not registered for fetching" — matching the
                // directive path's register-on-discovery behavior.
                remote_fetch.register_nested(url.clone());
                prepared.push(PreparedTransclusion::RemoteFile {
                    order: *next_order,
                    target: ApplyTarget::Section(slot),
                    url,
                    directive_options: transclusion::BlockOptions::default(),
                    insertion_context: None,
                });
                *next_order += 1;
            }
            transclusion::ResolvedTarget::Url { url, .. } if ignore_invalid => {
                let mut fixed_report = ComposeReport::new();
                fixed_report.transclusions_skipped = 1;
                fixed_report.add_warning(ComposeWarning::new(
                    "transclusion",
                    format!(
                        "Skipping URL transclusion '{}': remote execution disabled",
                        url
                    ),
                ));
                prepared.push(PreparedTransclusion::FixedSection {
                    order: *next_order,
                    slot,
                    content: None,
                    report: fixed_report,
                });
                *next_order += 1;
            }
            transclusion::ResolvedTarget::Url { url, .. } => {
                return Err(transclusion::TransclusionError::UrlExecutionDisabled {
                    url: url.to_string(),
                }
                .into());
            }
        }

        Ok(())
    }

    fn prepare_toc_transclusions(
        &self,
        directives: &[toc_linking::TocLinkingDirective],
        _report: &mut ComposeReport,
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
    ) {
        for directive in directives {
            prepared.push(PreparedTransclusion::Toc {
                order: *next_order,
                span: directive.span.clone(),
                directive: directive.clone(),
            });
            *next_order += 1;
        }
    }

    fn prepare_file_links_transclusions(
        &self,
        directives: &[file_links::FileLinksDirective],
        prepared: &mut Vec<PreparedTransclusion>,
        next_order: &mut usize,
    ) {
        // Discovery is intentionally NOT performed here: it runs in the
        // concurrent resolve stage (see `resolve_file_links_transclusion`) so
        // multiple directives' expensive filesystem walks parallelize. This
        // loop only enqueues the parsed directive.
        for directive in directives {
            prepared.push(PreparedTransclusion::FileLinks {
                order: *next_order,
                span: directive.span.clone(),
                directive: directive.clone(),
            });
            *next_order += 1;
        }
    }

    /// Records a fetched remote URL body as a closure-hash dependency.
    ///
    /// The dependency's `closure_hash` is the xxHash of the response body, so a
    /// changed remote document invalidates any parent artifact transcluding it.
    /// No-op when the URL's content hash is unavailable (unregistered or failed).
    fn record_remote_dependency(
        runtime_mutex: &std::sync::Mutex<&mut shell_expansion::types::PipelineRuntime>,
        remote_fetch: &remote_fetch::RemoteFetchRuntime,
        url: &url::Url,
    ) {
        if let Some(content_hash) = remote_fetch.content_hash(url) {
            let sid = cache::hashing::source_id_hash(url.as_str());
            let dependency = cache::types::DependencyRef {
                artifact_class: cache::types::ArtifactClass::RemoteUrl,
                entry_key: sid,
                source_id_hash: sid,
                closure_hash: content_hash,
            };
            runtime_mutex.lock().unwrap().record_dependency(dependency);
        }
    }

    fn resolve_prepared_transclusion(
        &self,
        item: PreparedTransclusion,
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime_mutex: &std::sync::Mutex<&mut shell_expansion::types::PipelineRuntime>,
    ) -> MarkdownResult<ResolvedTransclusion> {
        match item {
            PreparedTransclusion::FixedReplace {
                order,
                span,
                replacement,
                report,
            } => Ok(ResolvedTransclusion {
                order,
                target: ApplyTarget::Replace(span),
                content: Some(replacement),
                report,
                source_file: None,
            }),
            PreparedTransclusion::FixedSection {
                order,
                slot,
                content,
                report,
            } => Ok(ResolvedTransclusion {
                order,
                target: ApplyTarget::Section(slot),
                content,
                report,
                source_file: None,
            }),
            PreparedTransclusion::Markdown {
                order,
                target,
                path,
                directive_options,
                insertion_context,
            } => {
                let mut child_runtime = {
                    let runtime = runtime_mutex.lock().unwrap();
                    runtime.clone_for_child()
                };
                let mut child_report = ComposeReport::new();
                let content = self.render_markdown_transclusion(
                    &path,
                    insertion_context,
                    &directive_options,
                    state,
                    options,
                    &mut child_runtime,
                    &mut child_report,
                )?;
                child_report.transclusions_applied += 1;
                {
                    let mut runtime = runtime_mutex.lock().unwrap();
                    runtime.merge_child(&child_runtime);
                }
                Ok(ResolvedTransclusion {
                    order,
                    target,
                    content: Some(content),
                    report: child_report,
                    source_file: Some(path),
                })
            }
            PreparedTransclusion::Code {
                order,
                span,
                path,
                directive_options,
                line: _line,
            } => {
                let cache_handle = {
                    let runtime = runtime_mutex.lock().unwrap();
                    runtime.cache.clone()
                };
                let (content, dependency) = self.render_code_transclusion(
                    &path,
                    &directive_options,
                    state,
                    options,
                    &cache_handle,
                )?;
                if let Some(dependency) = dependency {
                    let mut runtime = runtime_mutex.lock().unwrap();
                    runtime.record_dependency(dependency);
                }
                let mut code_report = ComposeReport::new();
                code_report.transclusions_applied = 1;
                Ok(ResolvedTransclusion {
                    order,
                    target: ApplyTarget::Replace(span),
                    content: Some(content),
                    report: code_report,
                    source_file: None,
                })
            }
            PreparedTransclusion::RemoteFile {
                order,
                target,
                url,
                directive_options,
                insertion_context,
            } => {
                let remote_fetch = {
                    let runtime = runtime_mutex.lock().unwrap();
                    runtime.remote_fetch.clone()
                };
                let body_text = remote_fetch
                    .get_content(&url)
                    .map_err(|e| {
                        transclusion::TransclusionError::RemoteFetchFailed {
                            url: url.to_string(),
                            reason: e,
                        }
                    })?
                    .ok_or_else(|| transclusion::TransclusionError::RemoteFetchFailed {
                        url: url.to_string(),
                        reason: "URL was not registered for fetching".to_string(),
                    })?;

                Self::record_remote_dependency(runtime_mutex, &remote_fetch, &url);

                // Parse the fetched body as Markdown and recursively compose it.
                let mut child =
                    crate::markdown::Markdown::try_from_content(body_text).map_err(|e| {
                        crate::markdown::types::MarkdownError::Transform(format!(
                            "failed to parse fetched Markdown from '{}': {e}",
                            url
                        ))
                    })?;

                let child_source = ComposeSource::Url(url.clone());

                // Eagerly register any remote URLs the fetched document itself
                // references, so the child pipeline's point-of-use waits land on
                // an already-in-flight slot rather than an unregistered one.
                // Mirror the root pipeline's op-scoping: directive URLs follow
                // block transclusion, expression URLs follow interpolation.
                if options.allow_remote_transclusion {
                    let mut child_catalog = remote::RemoteUrlCatalog::new();

                    if options.is_enabled(ComposeOperation::BlockTransclusion) {
                        let child_directives = transclusion::parse_directives(
                            child.content(),
                            child.source_context_for_errors(),
                        )
                        .unwrap_or_default();
                        for entry in remote::discover_remote_urls_from_directives(
                            &child_directives,
                            &child_source,
                        ) {
                            child_catalog.add(entry);
                        }
                    }

                    if options.is_enabled(ComposeOperation::Interpolation) {
                        for entry in remote::discover_remote_urls_from_expressions(
                            child.content(),
                            &child_source,
                        ) {
                            child_catalog.add(entry);
                        }
                    }

                    for nested in child_catalog.urls() {
                        remote_fetch.register_nested(nested);
                    }
                }

                let mut child_runtime = {
                    let runtime = runtime_mutex.lock().unwrap();
                    runtime.clone_for_child()
                };
                let mut child_options = options.clone();
                child_options.source = child_source;

                let child_report = child
                    .run_compose_pipeline_internal(child_options, &mut child_runtime)?;
                {
                    let mut runtime = runtime_mutex.lock().unwrap();
                    runtime.merge_child(&child_runtime);
                }

                let mut content = child.content().to_string();
                let mut merged_report = child_report;
                merged_report.transclusions_applied += 1;

                if let Some((offset, line)) = insertion_context
                    && let Some(parent_level) =
                        transclusion::find_preceding_heading_level(&self.content, offset)
                {
                    let target_level =
                        super::normalize::HeadingLevel::new((parent_level.as_u8() + 1).min(6))
                            .unwrap_or(super::normalize::HeadingLevel::H6);
                    let (releveled, warnings) =
                        transclusion::relevel_with_overflow(&content, target_level);
                    content = releveled;
                    for warning in warnings {
                        merged_report.add_warning(warning.at_line(line));
                    }
                }

                let result = self.apply_wrappers(content, &directive_options);
                Ok(ResolvedTransclusion {
                    order,
                    target,
                    content: Some(result),
                    report: merged_report,
                    source_file: None,
                })
            }
            PreparedTransclusion::RemoteCode {
                order,
                span,
                url,
                directive_options,
                language,
            } => {
                let remote_fetch = {
                    let runtime = runtime_mutex.lock().unwrap();
                    runtime.remote_fetch.clone()
                };
                let body_text = remote_fetch
                    .get_content(&url)
                    .map_err(|e| {
                        transclusion::TransclusionError::RemoteFetchFailed {
                            url: url.to_string(),
                            reason: e,
                        }
                    })?
                    .ok_or_else(|| transclusion::TransclusionError::RemoteFetchFailed {
                        url: url.to_string(),
                        reason: "URL was not registered for fetching".to_string(),
                    })?;

                Self::record_remote_dependency(runtime_mutex, &remote_fetch, &url);

                let fenced = transclusion::wrap_in_code_block(&body_text, &language);
                let spaced = transclusion::ensure_vertical_spacing(&fenced);
                let result = self.apply_wrappers(spaced, &directive_options);
                let mut code_report = ComposeReport::new();
                code_report.transclusions_applied = 1;
                Ok(ResolvedTransclusion {
                    order,
                    target: ApplyTarget::Replace(span),
                    content: Some(result),
                    report: code_report,
                    source_file: None,
                })
            }
            PreparedTransclusion::Toc {
                order,
                span,
                directive,
            } => {
                let transclusion_opts = options.transclusion_options();
                let replacement = if let Some((display_target, path)) =
                    toc_linking::resolve_target_chain(
                        &directive,
                        &options.source,
                        &transclusion_opts,
                        self.source_context_for_errors(),
                    )? {
                    let cache_handle = {
                        let runtime = runtime_mutex.lock().unwrap();
                        runtime.cache.clone()
                    };
                    let canonical_source = cache::compose_cache_key_for_path(&path);
                    let source_id = cache::hashing::source_id_hash(&canonical_source);
                    let source_bytes =
                        std::fs::read(&path).map_err(toc_linking::TocLinkingError::Io)?;
                    let source_content_hash = cache::hashing::raw_bytes_hash(&source_bytes);
                    let buckets = cache::TocLinkingOperation::split_params(&directive.options);
                    let entry_key =
                        cache::TocLinkingOperation::variant_cache_key(source_id, &buckets);
                    let cache_key =
                        cache::TocLinkingOperation::cache_key_string(&path, &directive.options);
                    let persistent_ctx = cache::OperationPersistentContext {
                        op_kind: "toc-linking",
                        entry_key,
                        source_id,
                        canonical_source,
                        source_content_hash,
                    };
                    let line = directive.line;
                    let options_clone = directive.options.clone();
                    let display_clone = display_target.clone();
                    let path_clone = path.clone();

                    let cached = cache_handle.get_or_compute_operation(
                        &cache_key,
                        Some(&persistent_ctx),
                        options.cache_freshness_mode,
                        || {
                            let headings = {
                                let runtime = runtime_mutex.lock().unwrap();
                                runtime
                                    .load_toc_headings(&path_clone)
                                    .map_err(toc_linking::TocLinkingError::Io)?
                            };
                            // Render with no indentation so the cache entry is
                            // indent-independent. Indentation is caller-local
                            // and is applied below after cache lookup.
                            let content = toc_linking::render_resolved_directive(
                                &display_clone,
                                &headings,
                                &options_clone,
                                line,
                                "",
                                None,
                            )
                            .map_err(crate::markdown::types::MarkdownError::TocLinking)?;
                            Ok(cache::OperationResult { content })
                        },
                    )?;

                    if let Some(dependency) = cache_handle.operation_dependency_ref(&persistent_ctx)
                    {
                        let mut runtime = runtime_mutex.lock().unwrap();
                        runtime.record_dependency(dependency);
                    }

                    toc_linking::indent_text(
                        &cached.content,
                        &directive.indent,
                        directive.inferred_indent.as_deref(),
                    )
                } else {
                    let empty_text = directive.options.empty_text.clone().unwrap_or_default();
                    toc_linking::indent_text(
                        &empty_text,
                        &directive.indent,
                        directive.inferred_indent.as_deref(),
                    )
                };

                let mut toc_report = ComposeReport::new();
                toc_report.toc_links_generated = 1;
                Ok(ResolvedTransclusion {
                    order,
                    target: ApplyTarget::Replace(span),
                    content: Some(replacement),
                    report: toc_report,
                    source_file: None,
                })
            }
            PreparedTransclusion::FileLinks {
                order,
                span,
                directive,
            } => self.resolve_file_links_transclusion(order, span, directive, options),
        }
    }

    /// Resolves a single `::file-links` directive in the concurrent stage.
    ///
    /// Discovery runs here (not during preparation) so directives parallelize.
    /// On a match the [`FileSystem`](biscuit_terminal::components::filesystem::FileSystem)
    /// tree is built directly from the discovered entries — no second
    /// filesystem walk — and its fully-styled render subtree is embedded
    /// losslessly via [`renderable::tree::embed`]. Empty and invalid results
    /// reproduce the strict/permissive behavior the preparation stage used to
    /// apply.
    fn resolve_file_links_transclusion(
        &self,
        order: usize,
        span: std::ops::Range<usize>,
        directive: file_links::FileLinksDirective,
        options: &ComposeOptions,
    ) -> MarkdownResult<ResolvedTransclusion> {
        let skipped_replace = |replacement: String, report: ComposeReport| {
            Ok(ResolvedTransclusion {
                order,
                target: ApplyTarget::Replace(span.clone()),
                content: Some(replacement),
                report,
                source_file: None,
            })
        };

        let render = match file_links::discover(&directive, &options.source) {
            Ok(result) => match result.render {
                Some(render) => render,
                None => {
                    // Empty result: strict mode inserts a subtle notice,
                    // permissive mode removes the directive with a warning.
                    let mut report = ComposeReport::new();
                    report.transclusions_skipped = 1;
                    if options.fail_fast {
                        return skipped_replace("_No matching files_".to_string(), report);
                    }
                    report.add_warning(
                        ComposeWarning::new(
                            "file_links",
                            format!(
                                "No matching files for ::file-links directive at line {}",
                                directive.line
                            ),
                        )
                        .at_line(directive.line),
                    );
                    return skipped_replace(String::new(), report);
                }
            },
            Err(err) => {
                if self.resolve_ignore_invalid(options) {
                    let mut report = ComposeReport::new();
                    report.transclusions_skipped = 1;
                    report.add_warning(
                        ComposeWarning::new("file_links", err.to_string()).at_line(directive.line),
                    );
                    return skipped_replace(String::new(), report);
                }
                return Err(err.into());
            }
        };

        // Build the FileSystem tree directly from the discovered entries and
        // inject it, so the component renders without re-walking the directory.
        let tree = file_links::build_included_tree(&render);
        let mut fs =
            biscuit_terminal::components::filesystem::FileSystem::new(&render.component_root)
                .map_err(|e| {
                    crate::markdown::types::MarkdownError::Transform(format!(
                        "failed to create FileSystem for ::file-links at line {}: {e}",
                        directive.line
                    ))
                })?;
        fs = fs
            .with_prebuilt_tree(tree)
            .with_file_links()
            .italicize_dot_files(true)
            .dim_gitignore(true)
            .show_root(true);
        if !render.dimmed_prefix.is_empty() {
            fs = fs.with_dimmed_root_prefix(&render.dimmed_prefix);
        }
        if !render.target_name.is_empty() {
            fs = fs.with_root_display_name(&render.target_name);
        }
        if render.uses_repo_icon {
            fs = fs.with_root_icon(
                biscuit_terminal::components::filesystem::RootIconKind::Repository,
            );
        }

        // Carry the fully-styled render subtree through the composed document
        // losslessly: the fold splices it back so terminal and browser
        // rendering reproduce the live component (color, dim, icons), while
        // plain-Markdown consumers see the embedded portable fallback.
        use renderable::tree::{TreeRenderable, encode_embedded_subtree};
        let node = fs.render_tree();
        let embedded = encode_embedded_subtree(&node).map_err(|e| {
            crate::markdown::types::MarkdownError::Transform(format!(
                "failed to embed ::file-links render tree at line {}: {e}",
                directive.line
            ))
        })?;
        let replacement = indent::indent_text(
            &embedded,
            &directive.indent,
            directive.inferred_indent.as_deref(),
        );

        let mut report = ComposeReport::new();
        report.transclusions_applied = 1;
        Ok(ResolvedTransclusion {
            order,
            target: ApplyTarget::Replace(span),
            content: Some(replacement),
            report,
            source_file: None,
        })
    }

    // NOTE: `::file` and `::code` directives share the same indentation
    // preservation bug as `::toc-linking` (see spec.md for 2026-05-07).
    // Unlike `::toc-linking`, the fix is not trivially co-located here:
    // `PreparedTransclusion::Markdown` and `PreparedTransclusion::Code`
    // do not capture directive indentation, and the underlying
    // `transclusion::Directive` struct lacks indent fields. Fixing this
    // would require structural changes across the transclusion pipeline.
    // Tracked as part of the same feature but deferred to a follow-up.
    #[allow(clippy::too_many_arguments)]
    fn render_markdown_transclusion(
        &self,
        path: &Path,
        insertion_context: Option<(usize, usize)>,
        directive_options: &transclusion::BlockOptions,
        state: &EffectiveState,
        options: &ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
        report: &mut ComposeReport,
    ) -> MarkdownResult<String> {
        // ── Core compose (cacheable via single-flight) ─────────────
        let overlay_hash = cache::hashing::set_overlay_hash(
            directive_options.set_object.as_ref(),
            &directive_options.set_properties,
        );
        let options_hash = cache::hashing::combine_options_overlay_hash(
            cache::hashing::options_hash(options),
            overlay_hash,
        );
        let persistent_ctx = cache::PersistentContext {
            source_id: cache::hashing::source_id_hash(&cache::compose_cache_key_for_path(path)),
            state_hash: cache::hashing::effective_state_hash(state),
            context_hash: cache::hashing::context_hash(state.context()),
            options_hash,
        };
        let cache_key = format!(
            "compose:{:016x}:{:016x}:{:016x}:{:016x}:{:016x}",
            persistent_ctx.source_id,
            persistent_ctx.state_hash,
            persistent_ctx.context_hash,
            persistent_ctx.options_hash,
            overlay_hash,
        );
        let cache_handle = runtime.cache.clone();

        let inherited = self.build_child_external_state(state);
        let replace_parent_wins = matches!(
            directive_options.replace,
            transclusion::ReplaceOption::ParentWins
        );
        let one_off = match &directive_options.replace {
            transclusion::ReplaceOption::OneOff(one_off) => Some(one_off.clone()),
            _ => None,
        };
        let path_buf = path.to_path_buf();

        // Snapshot the per-directive set overlay. The overlay is applied to
        // the child's authored frontmatter before any of the child's pre-op
        // stages run; it does NOT propagate through `child_options` so
        // grandchildren do not inherit it.
        let set_object = directive_options.set_object.clone();
        let set_properties = directive_options.set_properties.clone();

        let cached = cache_handle.get_or_compute_compose(
            &cache_key,
            Some(&persistent_ctx),
            options.cache_freshness_mode,
            || {
                let mut child_options = options
                    .clone()
                    .with_replace_parent_wins(replace_parent_wins)
                    .with_one_off_replace(one_off.clone());
                child_options.external_state = Some(inherited.clone());
                child_options.source = ComposeSource::File(path_buf.clone());

                let mut compose_runtime = runtime.clone_for_child();
                let mut child = compose_runtime.load_markdown(path)?;

                // Apply the three-layer set overlay on the child's frontmatter
                // before any of its pre-op stages observe it. Keeping this
                // scoped inside the closure preserves the rule that
                // grandchildren referenced by the child's own `::file`
                // directives do NOT inherit this parent-applied overlay.
                if set_object.is_some() || !set_properties.is_empty() {
                    let base_indexmap = std::mem::take(child.frontmatter_mut().as_map_mut());
                    let base_map: serde_json::Map<String, Value> =
                        base_indexmap.into_iter().collect();
                    let overlaid =
                        state::apply_set_overrides(&base_map, set_object.as_ref(), &set_properties);
                    *child.frontmatter_mut().as_map_mut() = overlaid.into_iter().collect();
                }

                let child_report =
                    child.run_compose_pipeline_internal(child_options, &mut compose_runtime)?;
                runtime.merge_child(&compose_runtime);

                Ok(cache::ComposeResult {
                    content: child.content().to_string(),
                    report: child_report,
                    dependencies: compose_runtime.dependencies().to_vec(),
                })
            },
        )?;

        if let Some(dependency) = cache_handle.compose_dependency_ref(&persistent_ctx) {
            runtime.record_dependency(dependency);
        }

        report.merge(cached.report.clone());

        // ── Post-cache transforms (parent-specific, cheap) ─────────
        let mut content = cached.content.clone();

        // Apply exclude patterns to remove heading sections from the child.
        if !directive_options.exclude.is_empty() {
            let mut child_md = Markdown::new(content);
            child_md.remove_sections(&directive_options.exclude);
            content = child_md.into_parts().1;
        }

        if let Some((offset, line)) = insertion_context
            && let Some(parent_level) =
                transclusion::find_preceding_heading_level(&self.content, offset)
        {
            let target_level =
                super::normalize::HeadingLevel::new((parent_level.as_u8() + 1).min(6))
                    .unwrap_or(super::normalize::HeadingLevel::H6);
            let (releveled, warnings) = transclusion::relevel_with_overflow(&content, target_level);
            content = releveled;
            for warning in warnings {
                report.add_warning(warning.at_line(line));
            }
        }

        // For block directives (::file), ensure the final output ends with a
        // blank line so subsequent parent content is not absorbed into the last
        // block element of the child (e.g., a list item or blockquote).
        // This runs AFTER apply_wrappers because wrappers like wrap_quotation
        // use `.lines().join("\n")` which strips trailing newlines.
        // Frontmatter prologue/epilogue transclusion (insertion_context=None)
        // doesn't need this because sections are joined with "\n\n".
        let mut result = self.apply_wrappers(content, directive_options);
        if insertion_context.is_some() && !result.ends_with("\n\n") {
            if !result.ends_with('\n') {
                result.push('\n');
            }
            result.push('\n');
        }

        Ok(result)
    }

    fn render_code_transclusion(
        &self,
        path: &Path,
        directive_options: &transclusion::BlockOptions,
        state: &EffectiveState,
        options: &ComposeOptions,
        cache_handle: &cache::RunLocalCache,
    ) -> MarkdownResult<(String, Option<cache::types::DependencyRef>)> {
        // Compute variant params (needed for both cache key and core)
        let base_map = state.get_replace_map().cloned().unwrap_or_default();
        let effective_map = match &directive_options.replace {
            transclusion::ReplaceOption::InheritDefault => base_map,
            transclusion::ReplaceOption::ParentWins => base_map,
            transclusion::ReplaceOption::OneOff(one_off) => {
                state::merge_replace_maps(Some(&base_map), Some(one_off))
            }
        };
        let language = transclusion::infer_language(path, &options.code_fallback_language);
        let canonical_source = cache::compose_cache_key_for_path(path);
        let source_id = cache::hashing::source_id_hash(&canonical_source);
        let source_bytes = std::fs::read(path)?;
        let source_content_hash = cache::hashing::raw_bytes_hash(&source_bytes);

        let op = cache::CodeOperation;
        let mut buckets = op.split_params(directive_options);
        buckets
            .variant
            .push(("language".to_string(), language.clone()));
        let entry_key = op.variant_cache_key(source_id, &buckets);
        let cache_key = format!("code:{}:{:016x}", canonical_source, entry_key);
        let persistent_ctx = cache::OperationPersistentContext {
            op_kind: "code",
            entry_key,
            source_id,
            canonical_source,
            source_content_hash,
        };

        // Core computation (cacheable via single-flight)
        let context = options.context().clone();
        let path_buf = path.to_path_buf();
        let raw_text = match String::from_utf8(source_bytes) {
            Ok(text) => text,
            Err(_) => {
                return Err(transclusion::TransclusionError::NonTextCodeSource {
                    path: path_buf.clone(),
                }
                .into());
            }
        };
        let cached = cache_handle.get_or_compute_operation(
            &cache_key,
            Some(&persistent_ctx),
            options.cache_freshness_mode,
            || {
                let raw = raw_text.clone();

                let replaced = if effective_map.is_empty() {
                    raw
                } else {
                    let mut frontmatter = HashMap::new();
                    frontmatter.insert("replace".to_string(), Value::Object(effective_map.clone()));
                    let temp_state = EffectiveStateBuilder::new()
                        .with_frontmatter(frontmatter)
                        .with_context(context.clone())
                        .build()
                        .expect("replace-only state has no user ctx");
                    let (replaced, _) = replacement::apply_replacements(&raw, &temp_state);
                    replaced
                };

                let fenced = transclusion::wrap_in_code_block(&replaced, &language);
                let spaced = transclusion::ensure_vertical_spacing(&fenced);
                Ok(cache::OperationResult { content: spaced })
            },
        )?;

        // Post: apply wrappers (cheap, directive-specific)
        Ok((
            self.apply_wrappers(cached.content.clone(), directive_options),
            cache_handle.operation_dependency_ref(&persistent_ctx),
        ))
    }

    fn apply_wrappers(
        &self,
        mut content: String,
        directive_options: &transclusion::BlockOptions,
    ) -> String {
        if let Some(quotation) = &directive_options.quotation {
            let attribution = if quotation.is_empty() {
                None
            } else {
                Some(quotation.as_str())
            };
            content = transclusion::wrap_quotation(&content, attribution);
        }

        if let Some(summary) = &directive_options.disclosure {
            content = transclusion::wrap_disclosure(&content, summary);
        }

        content
    }

    fn build_child_external_state(&self, state: &EffectiveState) -> Value {
        let mut inherited: Map<String, Value> = state.data().clone().into_iter().collect();

        // Prologue/epilogue are scoped to the defining document — never propagate.
        // ctx is captured fresh per-document by EffectiveStateBuilder, so the
        // parent's merged runtime context must not leak into children (it would
        // appear as a document-defined ctx and trigger false collision warnings).
        inherited.remove("prologue");
        inherited.remove("epilogue");
        inherited.remove("ctx");

        Value::Object(inherited)
    }

    fn resolve_ignore_invalid(&self, options: &ComposeOptions) -> bool {
        if let Some(value) = options.ignore_invalid_references {
            return value;
        }

        if let Ok(Some(value)) = self.fm_get::<bool>("ignore_invalid") {
            return value;
        }

        options
            .context()
            .env()
            .get("IGNORE_INVALID")
            .and_then(|raw| parse_bool(raw))
            .unwrap_or(false)
    }

    /// Resolves whether interpolation should process fenced/indented code blocks.
    ///
    /// Inline code spans are always interpolated; this only governs
    /// fenced and indented code blocks.
    ///
    /// Checks (in priority order):
    /// 1. `ComposeOptions::interpolate_code_blocks`
    /// 2. Frontmatter `interpolate_code_blocks` key
    fn resolve_interpolate_code_blocks(&self, options: &ComposeOptions) -> bool {
        if options.interpolate_code_blocks {
            return true;
        }

        if let Ok(Some(value)) = self.fm_get::<bool>("interpolate_code_blocks") {
            return value;
        }

        false
    }
}

fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::HeadingLevel;
    use super::*;

    #[test]
    fn test_compose_returns_unchanged_document() {
        let content = "# Hello\n\nWorld";
        let md: Markdown = content.into();

        let (composed, _report) = md.compose().unwrap();

        // Content should still match (cleanup may add spacing)
        assert!(composed.content().contains("Hello"));
        assert!(composed.content().contains("World"));
    }

    #[test]
    fn test_compose_mut_modifies_in_place() {
        let content = "# Hello\n\nWorld";
        let mut md: Markdown = content.into();

        let _report = md.compose_mut().unwrap();

        assert!(md.content().contains("Hello"));
        assert!(md.content().contains("World"));
    }

    #[test]
    fn test_compose_with_custom_options() {
        let content = "# Hello\n\nWorld";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);

        let (composed, report) = md.compose_with(options).unwrap();

        // With cleanup disabled, content should be unchanged
        assert_eq!(composed.content(), md.content());
        assert!(!report.cleanup_changed);
    }

    #[test]
    fn test_compose_cleanup_stage() {
        // Content without proper spacing
        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Cleanup]);

        let (composed, report) = md.compose_with(options).unwrap();

        // Cleanup should add blank line between header and paragraph
        assert!(composed.content().contains("\n\n"));
        assert!(report.cleanup_changed);
    }

    #[test]
    fn test_compose_normalization_stage_no_change() {
        let content = "# Hello\n\n## World";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Normalization]);

        let (_, report) = md.compose_with(options).unwrap();

        // Well-formed document, no normalization needed
        assert!(report.normalization_report.is_none());
    }

    #[test]
    fn test_compose_preserves_frontmatter() {
        let content = "---\ntitle: Test\n---\n# Hello";
        let md: Markdown = content.into();

        let (composed, _) = md.compose().unwrap();

        let title: Option<String> = composed.fm_get("title").unwrap();
        assert_eq!(title, Some("Test".to_string()));
    }

    #[test]
    fn test_compose_report_summary() {
        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        let (_, report) = md.compose().unwrap();

        // Should have a meaningful summary
        let summary = report.summary();
        assert!(!summary.is_empty());
    }

    #[test]
    fn test_compose_report_has_changes() {
        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        let (_, report) = md.compose().unwrap();

        // Cleanup should have made changes
        assert!(report.has_changes());
        assert!(report.cleanup_changed);
    }

    #[test]
    fn test_compose_stages_all_disabled() {
        let content = "# Header\nParagraph";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[]);

        let (composed, report) = md.compose_with(options).unwrap();

        // No changes should be made
        assert_eq!(composed.content(), md.content());
        assert!(!report.has_changes());
    }

    #[test]
    fn test_compose_stages_run_in_order() {
        let content = "---\nshow: false\n---\nBefore\n\n::block when=\"show\"\n\n::shell echo hidden\n\n::end-block\n\n::code ./example.rs\nAfter";
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let code = dir.path().join("example.rs");
        std::fs::write(&root, content).unwrap();
        std::fs::write(&code, "fn main() {}\n").unwrap();

        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_shell_policy_root(dir.path())
            .only(&[
                ComposeOperation::PageBlocks,
                ComposeOperation::ShellExpansion,
                ComposeOperation::CodeTransclusion,
                ComposeOperation::Cleanup,
            ]);

        let (composed, report) = Markdown::try_from(root.as_path())
            .unwrap()
            .compose_with(options)
            .unwrap();

        assert!(!composed.content().contains("hidden"));
        assert!(composed.content().contains("```rs"));
        assert!(composed.content().contains("\n\n```rs"));
        assert_eq!(report.page_blocks_skipped, 1);
        assert_eq!(report.shell_expansions_applied, 0);
        assert_eq!(report.transclusions_applied, 1);
        assert!(report.cleanup_changed);
    }

    #[test]
    fn test_compose_with_external_state() {
        let content = "# Hello";
        let md: Markdown = content.into();

        let options =
            ComposeOptions::new().with_external_state(serde_json::json!({"key": "value"}));

        // Should not fail
        let result = md.compose_with(options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compose_options_context_captured() {
        let options = ComposeOptions::new();
        let ctx = options.context();

        // Context should have been captured
        assert!(!ctx.today().is_empty());
        assert!(!ctx.year().is_empty());
    }

    #[test]
    fn test_compose_fail_fast_false_continues_on_warning() {
        // Document that would cause normalization warning
        // (but for now normalization doesn't fail with None target)
        let content = "# Hello";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_fail_fast(false);

        let result = md.compose_with(options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_effective_state_available_to_stages() {
        let content = "---\nkey: value\n---\n# Hello";
        let md: Markdown = content.into();

        // External state should merge with frontmatter
        let options =
            ComposeOptions::new().with_external_state(serde_json::json!({"external": "data"}));

        let result = md.compose_with(options);
        assert!(result.is_ok());
    }

    // ============================================
    // Replacement stage integration tests
    // ============================================

    #[test]
    fn test_replacement_stage_with_frontmatter() {
        let content = "---\nreplace:\n  foo: bar\n---\n# Hello foo\n\nContent with foo here.";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Hello bar"));
        assert!(composed.content().contains("Content with bar here."));
        assert_eq!(report.replacements_applied, 2);
    }

    #[test]
    fn test_replacement_stage_overlap_resolution() {
        // Longest key wins: "foobar" before "foo"
        let content = "---\nreplace:\n  foo: short\n  foobar: long\n---\nfoobar and foo";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "long and short");
        assert_eq!(report.replacements_applied, 2);
    }

    #[test]
    fn test_replacement_stage_non_recursive() {
        // Replacement output should NOT be re-scanned
        let content = "---\nreplace:\n  foo: foobar\n  foobar: baz\n---\nfoo";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (composed, report) = md.compose_with(options).unwrap();

        // "foo" -> "foobar" but NOT -> "baz"
        assert_eq!(composed.content(), "foobar");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_null_value() {
        let content = "---\nreplace:\n  remove_me: null\n---\nHello remove_me world";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Hello  world");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_number_value() {
        let content = "---\nreplace:\n  VERSION: 42\n---\nVersion: VERSION";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Version: 42");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_no_replace_in_frontmatter() {
        let content = "---\ntitle: Test\n---\n# Hello foo";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (composed, report) = md.compose_with(options).unwrap();

        // No changes when no replace map
        assert_eq!(composed.content(), md.content());
        assert_eq!(report.replacements_applied, 0);
    }

    #[test]
    fn test_replacement_stage_with_external_state() {
        // External state can provide replace map
        let content = "# Hello foo";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::TextReplacement])
            .with_external_state(serde_json::json!({
                "replace": {"foo": "bar"}
            }));

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "# Hello bar");
        assert_eq!(report.replacements_applied, 1);
    }

    #[test]
    fn test_replacement_stage_frontmatter_overrides_external_with_deep_merge() {
        // External state replace map is merged as defaults; frontmatter wins on conflicts.
        let content = "---\nreplace:\n  foo: from_fm\n  baz: qux\n---\nfoo baz";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::TextReplacement])
            .with_external_state(serde_json::json!({
                "replace": {"foo": "from_external"}
            }));

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "from_fm qux");
        assert_eq!(report.replacements_applied, 2);
    }

    #[test]
    fn test_replacement_stage_report_summary() {
        let content = "---\nreplace:\n  a: b\n---\na a a";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::TextReplacement]);

        let (_, report) = md.compose_with(options).unwrap();

        assert_eq!(report.replacements_applied, 3);
        let summary = report.summary();
        assert!(summary.contains("3 replacement(s)"));
    }

    #[test]
    fn test_replacement_then_cleanup() {
        // Test that replacement runs before cleanup
        let content = "---\nreplace:\n  foo: bar\n---\n# Header\nfoo here";
        let md: Markdown = content.into();

        // Enable both replacement and cleanup
        let options = ComposeOptions::new()
            .only(&[ComposeOperation::TextReplacement, ComposeOperation::Cleanup]);

        let (composed, report) = md.compose_with(options).unwrap();

        // Replacement happened
        assert!(composed.content().contains("bar here"));
        assert_eq!(report.replacements_applied, 1);

        // Cleanup added blank line
        assert!(composed.content().contains("\n\n"));
        assert!(report.cleanup_changed);
    }

    // ============================================
    // Interpolation stage integration tests
    // ============================================

    #[test]
    fn test_interpolation_simple_variable() {
        let content = "---\nname: Alice\n---\n# Hello {{ name }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "# Hello Alice!");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_nested_variable() {
        let content = "---\nuser:\n  name: Bob\n---\nWelcome {{ user.name }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Welcome Bob");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_missing_variable() {
        let content = "---\ntitle: Test\n---\nHello {{ missing }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        // Missing variables become empty string
        assert_eq!(composed.content(), "Hello !");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_fallback_uses_default() {
        let content = "---\ntitle: Test\n---\nColor: {{ color || \"unknown\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Color: unknown");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_fallback_missing_variable_renders_default() {
        let content = "---\ntitle: Test\n---\nValue: {{ missing || \"default\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Value: default");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_fallback_uses_primary() {
        let content = "---\ncolor: blue\n---\nColor: {{ color || \"unknown\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Color: blue");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_ternary_true() {
        let content = "---\nactive: true\n---\nStatus: {{ active ? \"on\" : \"off\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Status: on");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_ternary_false() {
        let content = "---\nactive: false\n---\nStatus: {{ active ? \"on\" : \"off\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Status: off");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_comparison_equal() {
        let content = "---\ncount: 5\n---\n{{ count == 5 ? \"five\" : \"other\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "five");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_comparison_greater_than() {
        let content = "---\ncount: 10\n---\n{{ count > 5 ? \"many\" : \"few\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "many");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_multiple_expressions() {
        let content = "---\nfirst: Alice\nlast: Smith\n---\n{{ first }} {{ last }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Alice Smith");
        assert_eq!(report.interpolations_applied, 2);
    }

    #[test]
    fn test_interpolation_scans_inline_code_spans() {
        // Inline code spans (single backticks) are interpolated by default —
        // common templating pattern e.g. `var_{{ phase }}`.
        let content = "---\nname: Alice\n---\nHello {{ name }}! Code: `{{ name }}`";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Hello Alice! Code: `Alice`");
        assert_eq!(report.interpolations_applied, 2);
    }

    #[test]
    fn test_interpolation_code_blocks_via_option() {
        // Fenced code blocks are skipped unless explicitly opted in.
        let content = "---\nname: Alice\n---\nHello {{ name }}!\n\n```\n{{ name }}\n```";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Interpolation])
            .with_interpolate_code_blocks(true);

        let (composed, report) = md.compose_with(options).unwrap();

        // Both expressions expanded when interpolate_code_blocks is enabled
        assert!(composed.content().contains("Hello Alice!"));
        assert!(composed.content().contains("```\nAlice\n```"));
        assert_eq!(report.interpolations_applied, 2);
    }

    #[test]
    fn test_interpolation_code_blocks_via_frontmatter() {
        let content = "---\nname: Alice\ninterpolate_code_blocks: true\n---\nHello {{ name }}!\n\n```\n{{ name }}\n```";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        // Both expressions expanded when frontmatter flag is set
        assert!(composed.content().contains("Hello Alice!"));
        assert!(composed.content().contains("```\nAlice\n```"));
        assert_eq!(report.interpolations_applied, 2);
    }

    #[test]
    fn test_interpolation_skips_fenced_code() {
        let content = "---\nname: Alice\n---\nHello {{ name }}!\n\n```\n{{ name }}\n```";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        // Only the first expression is expanded, fenced block preserved
        assert!(composed.content().contains("Hello Alice!"));
        assert!(composed.content().contains("```\n{{ name }}\n```"));
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_no_expressions() {
        let content = "---\nname: Alice\n---\n# Just plain text";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), md.content());
        assert_eq!(report.interpolations_applied, 0);
    }

    #[test]
    fn test_interpolation_with_external_state() {
        let content = "# Hello {{ name }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Interpolation])
            .with_external_state(serde_json::json!({"name": "External"}));

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "# Hello External!");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_frontmatter_overrides_external() {
        let content = "---\nname: Frontmatter\n---\n# Hello {{ name }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Interpolation])
            .with_external_state(serde_json::json!({"name": "External"}));

        let (composed, report) = md.compose_with(options).unwrap();

        // Frontmatter wins on conflict
        assert_eq!(composed.content(), "# Hello Frontmatter!");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_chained_fallback() {
        let content = "---\nbackup: second\n---\nValue: {{ missing || backup || \"default\" }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Value: second");
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_interpolation_parse_error_preserves_original() {
        // Malformed expression should be left as-is (not fail_fast)
        let content = "---\nname: Alice\n---\nHello {{ @invalid }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Interpolation])
            .with_fail_fast(false);

        let (composed, report) = md.compose_with(options).unwrap();

        // Invalid expression left unchanged
        assert_eq!(composed.content(), "Hello {{ @invalid }}!");
        assert_eq!(report.interpolations_applied, 0);
    }

    #[test]
    fn test_interpolation_parse_error_fail_fast_returns_error() {
        let content = "---\nname: Alice\n---\nHello {{ @invalid }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Interpolation])
            .with_fail_fast(true);

        let err = md.compose_with(options).unwrap_err();
        assert!(matches!(err, MarkdownError::Transform(_)));
    }

    #[test]
    fn test_interpolation_bare_pipe_produces_parse_error() {
        // Bare `|` in interpolation should produce a clear lexer error
        let content = "---\nname: Alice\n---\nHello {{ name | \"default\" }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Interpolation])
            .with_fail_fast(false);

        let (composed, report) = md.compose_with(options).unwrap();

        // Invalid expression left unchanged
        assert_eq!(composed.content(), "Hello {{ name | \"default\" }}!");
        assert_eq!(report.interpolations_applied, 0);
    }

    #[test]
    fn test_interpolation_bare_pipe_fail_fast_error_message() {
        let content = "---\nname: Alice\n---\nHello {{ name | \"default\" }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::Interpolation])
            .with_fail_fast(true);

        let err = md.compose_with(options).unwrap_err();
        let err_string = format!("{}", err);
        assert!(
            err_string.contains("Unexpected '|'") || err_string.contains("parse"),
            "Expected bare pipe error, got: {}",
            err_string
        );
    }

    #[test]
    fn test_full_compose_with_interpolation() {
        // Integration test: frontmatter + interpolation + cleanup
        let content = "---\nname: Alice\ncount: 3\n---\n# Welcome {{ name }}\nYou have {{ count > 0 ? \"items\" : \"nothing\" }}";
        let md: Markdown = content.into();

        let (composed, report) = md.compose().unwrap();

        assert!(composed.content().contains("Welcome Alice"));
        assert!(composed.content().contains("You have items"));
        assert_eq!(report.interpolations_applied, 2);
        assert!(report.cleanup_changed); // Cleanup adds blank line
    }

    #[test]
    fn test_interpolation_report_summary() {
        let content = "---\na: 1\nb: 2\n---\n{{ a }} {{ b }}";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (_, report) = md.compose_with(options).unwrap();

        assert_eq!(report.interpolations_applied, 2);
        let summary = report.summary();
        assert!(summary.contains("2 interpolation(s)"));
    }

    // ============================================
    // E2E Integration tests
    // ============================================

    #[test]
    fn test_e2e_all_stages_with_external_state() {
        // Full pipeline: replacement -> interpolation -> cleanup -> normalization
        let content = r#"---
replace:
  PLACEHOLDER: actual
name: Alice
count: 5
---
# Welcome {{ name }}
PLACEHOLDER content here.
{{ count > 3 ? "many items" : "few items" }}"#;

        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_external_state(serde_json::json!({
            "extra": "external_value"
        }));

        let (composed, report) = md.compose_with(options).unwrap();

        // Replacement happened
        assert!(composed.content().contains("actual content here"));
        assert_eq!(report.replacements_applied, 1);

        // Interpolation happened
        assert!(composed.content().contains("Welcome Alice"));
        assert!(composed.content().contains("many items"));
        assert_eq!(report.interpolations_applied, 2);

        // Cleanup happened (blank line added)
        assert!(report.cleanup_changed);

        // Full summary
        let summary = report.summary();
        assert!(summary.contains("1 replacement(s)"));
        assert!(summary.contains("2 interpolation(s)"));
    }

    #[test]
    fn test_e2e_unicode_content() {
        // Test Unicode handling in replacement and interpolation
        let content = r#"---
replace:
  ":smile:": "😊"
  ":wave:": "👋"
greeting: こんにちは
---
Hello :wave: {{ greeting }} :smile:"#;

        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::TextReplacement,
            ComposeOperation::Interpolation,
        ]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content(), "Hello 👋 こんにちは 😊");
        assert_eq!(report.replacements_applied, 2);
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_e2e_helper_functions() {
        let content = r#"---
items:
  - a
  - b
  - c
value: "42"
pi: 3.14159
---
Items: {{ length(items) }}
Number: {{ number(value) }}
Rounded: {{ round(pi) }}"#;

        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Items: 3"));
        assert!(composed.content().contains("Number: 42"));
        assert!(composed.content().contains("Rounded: 3"));
        assert_eq!(report.interpolations_applied, 3);
    }

    #[test]
    fn test_stage2_file_transclusion_relevels_to_parent_heading() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "## Parent\n\n::file ./child.md").unwrap();
        std::fs::write(&child, "# Child\n\nBody").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("### Child"));
        assert!(composed.content().contains("Body"));
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn test_stage2_nested_transclusion_counts_recursive_includes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");

        std::fs::write(&root, "::file ./a.md").unwrap();
        std::fs::write(&a, "::file ./b.md").unwrap();
        std::fs::write(&b, "# Leaf").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("# Leaf"));
        assert_eq!(report.transclusions_applied, 2);
        assert!(report.max_transclusion_depth >= 2);
    }

    #[test]
    fn test_stage2_duplicate_sibling_includes_are_not_treated_as_cycles() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "::file ./child.md\n\n::file ./child.md").unwrap();
        std::fs::write(&child, "# Child").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content().matches("# Child").count(), 2);
        assert_eq!(report.transclusions_applied, 2);
    }

    #[test]
    fn test_stage2_diamond_dependency_graph_is_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let left = dir.path().join("left.md");
        let right = dir.path().join("right.md");
        let shared = dir.path().join("shared.md");

        std::fs::write(&root, "::file ./left.md\n\n::file ./right.md").unwrap();
        std::fs::write(&left, "## Left\n\n::file ./shared.md").unwrap();
        std::fs::write(&right, "## Right\n\n::file ./shared.md").unwrap();
        std::fs::write(&shared, "### Shared").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content().matches("### Shared").count(), 2);
        assert_eq!(report.transclusions_applied, 4);
    }

    #[test]
    fn test_stage2_cycle_detection_fails() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");

        std::fs::write(&a, "::file ./b.md").unwrap();
        std::fs::write(&b, "::file ./a.md").unwrap();

        let md = Markdown::try_from(a.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(a);
        let err = md.compose_with(options).unwrap_err();

        assert!(matches!(
            err,
            MarkdownError::Transclusion(ref inner)
                if matches!(inner.as_ref(), transclusion::TransclusionError::CycleDetected { .. })
        ));
    }

    #[test]
    fn test_stage2_code_transclusion_wraps_fenced_block() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let code = dir.path().join("main.rs");

        std::fs::write(&root, "## Code\n\n::code ./main.rs").unwrap();
        std::fs::write(&code, "fn main() {\n    println!(\"hi\");\n}\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("```rs"));
        assert!(composed.content().contains("fn main()"));
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn test_stage2_code_transclusion_uses_fallback_language_for_unknown_extension() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let code = dir.path().join("sample.weird");

        std::fs::write(&root, "::code ./sample.weird").unwrap();
        std::fs::write(&code, "hello").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("```txt"));
        assert!(composed.content().contains("hello"));
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn test_stage2_repeated_code_includes_are_allowed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let code = dir.path().join("main.rs");

        std::fs::write(&root, "::code ./main.rs\n\n::code ./main.rs").unwrap();
        std::fs::write(&code, "fn repeated() {}\n").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content().matches("fn repeated() {}").count(), 2);
        assert_eq!(report.transclusions_applied, 2);
    }

    #[test]
    fn test_stage2_when_false_skips_directive() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "---\nenabled: false\n---\n::file ./child.md when=\"enabled\"",
        )
        .unwrap();
        std::fs::write(&child, "# Child").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(!composed.content().contains("Child"));
        assert_eq!(report.transclusions_skipped, 1);
    }

    #[test]
    fn test_stage2_frontmatter_prologue_epilogue() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let intro = dir.path().join("intro.md");
        let outro = dir.path().join("outro.md");

        std::fs::write(
            &root,
            "---\nprologue: ./intro.md\nepilogue: [\"./outro.md\"]\n---\nBody",
        )
        .unwrap();
        std::fs::write(&intro, "Intro").unwrap();
        std::fs::write(&outro, "Outro").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().starts_with("Intro"));
        assert!(composed.content().contains("Body"));
        assert!(composed.content().trim_end_matches('\n').ends_with("Outro"));
        assert_eq!(report.transclusions_applied, 2);
    }

    #[test]
    fn test_stage2_same_file_can_be_used_in_prologue_and_body() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let shared = dir.path().join("shared.md");

        std::fs::write(&root, "---\nprologue: ./shared.md\n---\n::file ./shared.md").unwrap();
        std::fs::write(&shared, "## Shared").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content().matches("## Shared").count(), 2);
        assert_eq!(report.transclusions_applied, 2);
    }

    #[test]
    fn test_stage2_missing_source_context_for_relative_path() {
        let md: Markdown = "::file ./child.md".into();
        let err = md.compose().unwrap_err();
        assert!(matches!(
            err,
            MarkdownError::Transclusion(ref inner)
                if matches!(
                    inner.as_ref(),
                    transclusion::TransclusionError::MissingSourceContext { .. }
                )
        ));
    }

    #[test]
    fn test_toc_linking_fail_fast_false_becomes_warning() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "::toc-linking ./missing.md").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_fail_fast(false);
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content().trim_end(), "::toc-linking ./missing.md");
        assert_eq!(report.toc_links_generated, 0);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.message.contains("File not found"))
        );
    }

    #[test]
    fn test_toc_linking_fail_fast_true_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "::toc-linking ./missing.md").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_fail_fast(true);
        let err = md.compose_with(options).unwrap_err();

        assert!(matches!(err, MarkdownError::TocLinking(_)));
    }

    #[test]
    fn test_stage2_h6_overflow_converts_to_bold_text() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "###### Root\n\n::file ./child.md").unwrap();
        std::fs::write(&child, "## Child\n\n### Deep").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("###### Child"));
        assert!(composed.content().contains("**Deep**"));
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.message.contains("Heading overflow"))
        );
    }

    #[test]
    fn test_stage2_consecutive_file_directives_separated_by_blank_line() {
        // Regression test: when two ::file directives are consecutive, the second
        // file's content must not be absorbed into the last block element (e.g., a
        // list) of the first file.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let one = dir.path().join("one.md");
        let two = dir.path().join("two.md");

        std::fs::write(&root, "::file ./one.md\n\n::file ./two.md").unwrap();
        std::fs::write(&one, "- Item A\n- Item B").unwrap();
        std::fs::write(&two, "## Section Two\n\nParagraph.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        // Two transclusions should have occurred
        assert_eq!(report.transclusions_applied, 2);
        // The heading from two.md must exist as a proper heading, not inside a list
        assert!(
            composed.content().contains("\n## Section Two\n")
                || composed.content().contains("\n## Section Two"),
            "Second file's heading should not be absorbed into first file's list, got:\n{}",
            composed.content()
        );
    }

    #[test]
    fn test_stage2_frontmatter_inline_string_prologue() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(
            &root,
            "---\nprologue: \"**Draft** document\"\n---\nBody content.",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().starts_with("**Draft** document"));
        assert!(composed.content().contains("Body content."));
        assert_eq!(report.transclusions_applied, 0); // inline string is not a transclusion
    }

    #[test]
    fn test_stage2_frontmatter_inline_string_epilogue() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(&root, "---\nepilogue: \"End of document.\"\n---\nBody.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(
            composed
                .content()
                .trim_end_matches('\n')
                .ends_with("End of document.")
        );
        assert_eq!(report.transclusions_applied, 0);
    }

    #[test]
    fn test_stage2_frontmatter_mixed_file_and_inline() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let intro = dir.path().join("intro.md");

        std::fs::write(
            &root,
            "---\nprologue: [\"./intro.md\", \"Inline note.\"]\n---\nBody.",
        )
        .unwrap();
        std::fs::write(&intro, "File intro.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        let content = composed.content();
        assert!(content.starts_with("File intro."));
        assert!(content.contains("Inline note."));
        assert!(content.contains("Body."));
        assert_eq!(report.transclusions_applied, 1); // only the file counts
    }

    #[test]
    fn test_stage2_frontmatter_bare_filename_is_treated_as_file_reference() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let intro = dir.path().join("intro.md");

        std::fs::write(&root, "---\nprologue: intro.md\n---\nBody.").unwrap();
        std::fs::write(&intro, "Intro text.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        let content = composed.content();
        assert!(content.starts_with("Intro text."));
        assert!(content.contains("Body."));
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn test_stage2_parent_frontmatter_propagates_to_child_interpolation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "---\nauthor: Alice\n---\n::file ./child.md").unwrap();
        std::fs::write(&child, "Written by {{ author }}.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Written by Alice."));
    }

    #[test]
    fn test_stage2_parent_replace_map_propagates_to_child() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "---\nreplace:\n  PLACEHOLDER: actual\n---\n::file ./child.md",
        )
        .unwrap();
        std::fs::write(&child, "Content with PLACEHOLDER here.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Content with actual here."));
    }

    #[test]
    fn test_stage2_replace_parent_wins_inverts_precedence() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "---\nreplace:\n  TOKEN: parent\n---\n::file ./child.md replace=true",
        )
        .unwrap();
        std::fs::write(&child, "---\nreplace:\n  TOKEN: child\n---\nTOKEN").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _report) = md.compose_with(options).unwrap();

        assert_eq!(composed.content().trim(), "parent");
    }

    #[test]
    fn test_stage2_replace_one_off_does_not_propagate_to_grandchildren() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");
        let grand = dir.path().join("grand.md");

        std::fs::write(
            &root,
            "---\nreplace:\n  A: root\n---\n::file ./child.md replace={\"ONE\":\"oneoff\"}",
        )
        .unwrap();
        std::fs::write(&child, "Child: ONE A\n::file ./grand.md").unwrap();
        std::fs::write(&grand, "Grand: ONE A").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _report) = md.compose_with(options).unwrap();

        let content = composed.content();
        assert!(content.contains("Child: oneoff root"));
        assert!(content.contains("Grand: ONE root"));
    }

    #[test]
    fn test_stage2_prologue_epilogue_do_not_propagate_to_children() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "---\nepilogue: \"Root epilogue.\"\n---\n::file ./child.md",
        )
        .unwrap();
        std::fs::write(&child, "Child body.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _report) = md.compose_with(options).unwrap();

        let content = composed.content();
        // "Root epilogue." should appear exactly once — at the end of root, not within child
        assert_eq!(content.matches("Root epilogue.").count(), 1);
        assert!(content.trim_end_matches('\n').ends_with("Root epilogue."));
    }

    #[test]
    fn test_stage2_inline_epilogue_with_markdown_links() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");

        // Epilogue containing markdown link syntax should be treated as inline
        // content, not as a file reference. Previously, the `/` in `](./...)
        // caused `is_file_like_reference` to misidentify it as a path.
        std::fs::write(
            &root,
            "---\nepilogue: \"---\\n\\n- No [animals](./animals.md) were hurt\"\n---\nBody.",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _report) = md.compose_with(options).unwrap();

        let content = composed.content();
        assert!(
            content.contains("[animals](./animals.md)"),
            "Inline epilogue with markdown links should be preserved, got:\n{}",
            content
        );
    }

    #[test]
    fn test_stage2_exclude_removes_section() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "::file ./child.md exclude=\"## Remove Me\"").unwrap();
        std::fs::write(
            &child,
            "## Keep\n\nKept body.\n\n## Remove Me\n\nRemoved body.\n\n## Also Keep\n\nAlso kept.",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("## Keep"));
        assert!(composed.content().contains("Kept body."));
        assert!(!composed.content().contains("Remove Me"));
        assert!(!composed.content().contains("Removed body."));
        assert!(composed.content().contains("## Also Keep"));
        assert_eq!(report.transclusions_applied, 1);
    }

    #[test]
    fn test_stage2_exclude_wildcard() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "::file ./child.md exclude=\"## Remove*\"").unwrap();
        std::fs::write(
            &child,
            "## Keep\n\nKept.\n\n## Remove This\n\nGone.\n\n## Also Keep\n\nStays.",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("## Keep"));
        assert!(!composed.content().contains("Remove This"));
        assert!(composed.content().contains("## Also Keep"));
    }

    #[test]
    fn test_stage2_exclude_prelude() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "::file ./child.md exclude=\"!prelude\"").unwrap();
        std::fs::write(&child, "Prelude text here.\n\n## Heading\n\nBody.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _) = md.compose_with(options).unwrap();

        assert!(!composed.content().contains("Prelude text"));
        assert!(composed.content().contains("## Heading"));
        assert!(composed.content().contains("Body."));
    }

    #[test]
    fn test_stage2_multiple_excludes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "::file ./child.md exclude=\"## A\" exclude=\"## C\"").unwrap();
        std::fs::write(
            &child,
            "## A\n\nA body.\n\n## B\n\nB body.\n\n## C\n\nC body.",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _) = md.compose_with(options).unwrap();

        assert!(!composed.content().contains("## A"));
        assert!(composed.content().contains("## B"));
        assert!(!composed.content().contains("## C"));
    }

    #[test]
    fn test_stage2_quotation_wrapper_does_not_absorb_following_content() {
        // Regression: wrap_quotation consumed trailing \n\n, causing the
        // next paragraph to become a lazy continuation of the blockquote.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "## Section\n\n::file ./child.md quotation=\"Source\"\n\nFollowing paragraph.",
        )
        .unwrap();
        std::fs::write(&child, "Quoted content here.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert_eq!(report.transclusions_applied, 1);
        // The "Following paragraph" must NOT be inside the blockquote
        let content = composed.content();
        assert!(
            content.contains("\n\nFollowing paragraph."),
            "Following content should be separated from blockquote by blank line, got:\n{}",
            content
        );
        // Verify blockquote is present
        assert!(content.contains("> Quoted content here."));
        assert!(content.contains("> — Source"));
    }

    // ============================================
    // Conditional transclusion tests
    // ============================================

    #[test]
    fn test_stage2_when_env_match_includes_directive() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "## Section\n\n::file ./child.md when=\"env.AGENT == 'claude'\"",
        )
        .unwrap();
        std::fs::write(&child, "Claude content.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let mut ctx = types::ComposeContext::capture();
        ctx.env_mut()
            .insert("AGENT".to_string(), "claude".to_string());
        let options = ComposeOptions::new()
            .with_source_file(root)
            .with_context(ctx);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Claude content."));
        assert_eq!(report.transclusions_applied, 1);
        assert_eq!(report.transclusions_skipped, 0);
    }

    #[test]
    fn test_stage2_when_env_mismatch_skips_directive() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "## Section\n\n::file ./child.md when=\"env.AGENT == 'claude'\"",
        )
        .unwrap();
        std::fs::write(&child, "Claude content.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let mut ctx = types::ComposeContext::capture();
        ctx.env_mut()
            .insert("AGENT".to_string(), "opencode".to_string());
        let options = ComposeOptions::new()
            .with_source_file(root)
            .with_context(ctx);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(!composed.content().contains("Claude content."));
        assert_eq!(report.transclusions_applied, 0);
        assert_eq!(report.transclusions_skipped, 1);
    }

    #[test]
    fn test_stage2_when_env_unset_skips_equality() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "## Section\n\n::file ./child.md when=\"env.AGENT == 'claude'\"",
        )
        .unwrap();
        std::fs::write(&child, "Claude content.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        // Use a fixed context with no AGENT env var
        let ctx = types::ComposeContext::fixed_for_testing();
        let options = ComposeOptions::new()
            .with_source_file(root)
            .with_context(ctx);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(!composed.content().contains("Claude content."));
        assert_eq!(report.transclusions_skipped, 1);
    }

    #[test]
    fn test_stage2_mutual_exclusion_conditions() {
        // Three directives with mutually exclusive conditions:
        //   AGENT == 'claude'
        //   AGENT == 'opencode'
        //   !env.AGENT (unset)
        // Only one should match at any time.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let cc = dir.path().join("cc.md");
        let oc = dir.path().join("oc.md");
        let default = dir.path().join("default.md");

        std::fs::write(
            &root,
            "## Section\n\n\
             ::file ./cc.md when=\"env.AGENT == 'claude'\"\n\
             ::file ./oc.md when=\"env.AGENT == 'opencode'\"\n\
             ::file ./default.md when=\"!env.AGENT\"",
        )
        .unwrap();
        std::fs::write(&cc, "CC only.").unwrap();
        std::fs::write(&oc, "OC only.").unwrap();
        std::fs::write(&default, "Default only.").unwrap();

        // Test 1: AGENT=claude → only cc.md included
        let md = Markdown::try_from(root.as_path()).unwrap();
        let mut ctx = types::ComposeContext::capture();
        ctx.env_mut()
            .insert("AGENT".to_string(), "claude".to_string());
        let opts = ComposeOptions::new()
            .with_source_file(&root)
            .with_context(ctx);
        let (out, report) = md.compose_with(opts).unwrap();
        assert!(out.content().contains("CC only."), "Expected CC content");
        assert!(!out.content().contains("OC only."), "Should not contain OC");
        assert!(
            !out.content().contains("Default only."),
            "Should not contain default"
        );
        assert_eq!(report.transclusions_applied, 1);
        assert_eq!(report.transclusions_skipped, 2);

        // Test 2: AGENT=opencode → only oc.md included
        let md = Markdown::try_from(root.as_path()).unwrap();
        let mut ctx = types::ComposeContext::capture();
        ctx.env_mut()
            .insert("AGENT".to_string(), "opencode".to_string());
        let opts = ComposeOptions::new()
            .with_source_file(&root)
            .with_context(ctx);
        let (out, report) = md.compose_with(opts).unwrap();
        assert!(!out.content().contains("CC only."));
        assert!(out.content().contains("OC only."), "Expected OC content");
        assert!(!out.content().contains("Default only."));
        assert_eq!(report.transclusions_applied, 1);
        assert_eq!(report.transclusions_skipped, 2);

        // Test 3: AGENT not set → only default.md included
        let md = Markdown::try_from(root.as_path()).unwrap();
        let ctx = types::ComposeContext::fixed_for_testing();
        let opts = ComposeOptions::new()
            .with_source_file(&root)
            .with_context(ctx);
        let (out, report) = md.compose_with(opts).unwrap();
        assert!(!out.content().contains("CC only."));
        assert!(!out.content().contains("OC only."));
        assert!(
            out.content().contains("Default only."),
            "Expected default content"
        );
        assert_eq!(report.transclusions_applied, 1);
        assert_eq!(report.transclusions_skipped, 2);
    }

    // ============================================
    // Re-leveling tests
    // ============================================

    #[test]
    fn test_stage2_relevel_h1_child_under_h3_parent() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(
            &root,
            "# Title\n\n## Overview\n\n### Details\n\n::file ./child.md",
        )
        .unwrap();
        std::fs::write(&child, "# Child Title\n\n## Child Sub\n\nBody.").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _) = md.compose_with(options).unwrap();

        // Parent heading before directive is H3, so child should be re-leveled:
        // H1 → H4, H2 → H5
        assert!(
            composed.content().contains("#### Child Title"),
            "H1 should become H4, got:\n{}",
            composed.content()
        );
        assert!(
            composed.content().contains("##### Child Sub"),
            "H2 should become H5, got:\n{}",
            composed.content()
        );
    }

    // ── Page block integration tests ────────────────────────────────────

    #[test]
    fn page_block_true_preserves_content_through_pipeline() {
        let content = "---\nflag: true\n---\n\nbefore\n\n::block when=\"flag\"\n\nkept content\n\n::end-block\n\nafter\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            composed.content().contains("kept content"),
            "True block body should be preserved, got:\n{}",
            composed.content()
        );
        assert!(
            composed.content().contains("before"),
            "Content before block should be preserved"
        );
        assert!(
            composed.content().contains("after"),
            "Content after block should be preserved"
        );
        assert_eq!(report.page_blocks_rendered, 1);
    }

    #[test]
    fn page_block_false_removes_content_through_pipeline() {
        let content = "---\nflag: false\n---\n\nbefore\n\n::block when=\"flag\"\n\nremoved\n\n::end-block\n\nafter\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            !composed.content().contains("removed"),
            "False block body should be removed, got:\n{}",
            composed.content()
        );
        assert!(composed.content().contains("before"));
        assert!(composed.content().contains("after"));
        assert_eq!(report.page_blocks_skipped, 1);
    }

    #[test]
    fn page_block_condition_can_read_frontmatter_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let review = dir.path().join("review.md");
        std::fs::write(&review, "---\nready: true\n---\n# Review\n").unwrap();

        let content = concat!(
            "---\nreview_path: review.md\n---\n\n",
            "::block when=\"frontmatter(review_path, 'ready') == true\"\n\n",
            "ready block\n\n",
            "::end-block\n",
        );
        std::fs::write(&root, content).unwrap();
        let md: Markdown = content.into();

        let options = ComposeOptions::new()
            .only(&[ComposeOperation::PageBlocks])
            .with_source_file(&root);

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            composed.content().contains("ready block"),
            "page block should evaluate filesystem expression functions, got:\n{}",
            composed.content()
        );
        assert_eq!(report.page_blocks_rendered, 1);
    }

    #[test]
    fn page_block_coexists_with_interpolation() {
        let content =
            "---\nshow: true\n---\n\n::block when=\"show\"\n\nShown: {{show}}\n\n::end-block\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::Interpolation,
            ComposeOperation::PageBlocks,
        ]);

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            composed.content().contains("Shown: true"),
            "Page blocks and interpolation should both apply, got:\n{}",
            composed.content()
        );
        assert_eq!(report.page_blocks_rendered, 1);
        assert!(report.interpolations_applied > 0);
    }

    #[test]
    fn page_block_report_and_warnings_populated() {
        let content = "---\na: true\nb: false\n---\n\n::block when=\"a\" unknown=\"x\"\n\nA\n\n::end-block\n\n::block when=\"b\"\n\nB\n\n::end-block\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);

        let (_, report) = md.compose_with(options).unwrap();
        assert_eq!(report.page_blocks_rendered, 1);
        assert_eq!(report.page_blocks_skipped, 1);
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.message.contains("unknown")),
            "Should warn about unknown option"
        );
    }

    #[test]
    fn page_block_toggle_disabled_leaves_directives_as_text() {
        let content = "::block when=\"x\"\nbody\n::end-block\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[]);

        let (composed, report) = md.compose_with(options).unwrap();
        assert!(
            composed.content().contains("::block"),
            "With page_blocks disabled, directives should be left as text"
        );
        assert_eq!(report.page_blocks_rendered, 0);
        assert_eq!(report.page_blocks_skipped, 0);
    }

    #[test]
    fn perf_disabled_produces_no_report() {
        let content = "# Test\nSome content";
        let md: Markdown = content.into();

        let options = ComposeOptions::new(); // perf_enabled defaults to false
        let (_, report) = md.compose_with(options).unwrap();
        assert!(report.perf.is_none(), "Perf should be None when disabled");
    }

    #[test]
    fn perf_enabled_produces_report() {
        let content = "# Test\nSome content";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_perf(true);
        let (_, report) = md.compose_with(options).unwrap();
        assert!(
            report.perf.is_some(),
            "Perf should be populated when enabled"
        );

        let perf = report.perf.unwrap();
        assert!(perf.total > std::time::Duration::ZERO);
        assert!(!perf.metrics.is_empty(), "Should have at least one metric");

        // Verify expected stages are present
        let stages: Vec<_> = perf.metrics.iter().map(|m| m.stage).collect();
        assert!(stages.contains(&ComposeStage::EffectiveStateBuild));
        assert!(stages.contains(&ComposeStage::Cleanup));
    }

    #[test]
    fn perf_enabled_with_interpolation() {
        let content = "---\nname: World\n---\nHello {{ name }}!";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().with_perf(true);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(composed.content().contains("Hello World!"));
        let perf = report.perf.unwrap();
        let interp = perf
            .metrics
            .iter()
            .find(|m| m.stage == ComposeStage::Interpolation)
            .unwrap();
        assert_eq!(interp.calls, 1);
    }

    // ── Frontmatter Interpolation Integration Tests ─────────────────

    #[test]
    fn test_frontmatter_interpolation_spec_example() {
        let content = "---\nbase: /path/to/something\nspec: \"{{base}}/spec.md\"\nplan: \"{{base}}/plan.md\"\n---\nThe spec is located at: {{spec}}\nThe plan is located at: {{plan}}";
        let md: Markdown = content.into();
        let (composed, report) = md
            .compose_with(ComposeOptions::new().only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::Interpolation,
            ]))
            .unwrap();

        assert_eq!(report.frontmatter_interpolations_applied, 2);
        assert!(
            composed
                .content()
                .contains("The spec is located at: /path/to/something/spec.md")
        );
        assert!(
            composed
                .content()
                .contains("The plan is located at: /path/to/something/plan.md")
        );
    }

    #[test]
    fn test_frontmatter_interpolation_with_set_overrides() {
        let content = "---\nbase: /original\nspec: \"{{base}}/spec.md\"\n---\nSpec: {{spec}}";
        let md: Markdown = content.into();
        let (composed, report) = md
            .compose_with(
                ComposeOptions::new()
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ])
                    .with_set_overrides(serde_json::json!({"base": "/override"})),
            )
            .unwrap();

        assert_eq!(report.frontmatter_interpolations_applied, 1);
        assert!(composed.content().contains("Spec: /override/spec.md"));
    }

    #[test]
    fn test_frontmatter_interpolation_arrays_and_objects() {
        let content = "---\nbase: /root\npaths:\n  - \"{{base}}/a\"\n  - \"{{base}}/b\"\nmeta:\n  home: \"{{base}}/home\"\n---\n";
        let md: Markdown = content.into();
        let (_, report) = md
            .compose_with(ComposeOptions::new().only(&[ComposeOperation::FrontmatterInterpolation]))
            .unwrap();

        assert!(report.frontmatter_interpolations_applied >= 3);
    }

    #[test]
    fn test_frontmatter_interpolation_disabled() {
        let content = "---\nbase: /path\nspec: \"{{base}}/spec.md\"\n---\n{{spec}}";
        let md: Markdown = content.into();
        let (composed, report) = md
            .compose_with(
                ComposeOptions::new()
                    .disable(ComposeOperation::FrontmatterInterpolation)
                    .only(&[ComposeOperation::Interpolation]),
            )
            .unwrap();

        assert_eq!(report.frontmatter_interpolations_applied, 0);
        // body interpolation resolves {{spec}} to {{base}}/spec.md and then
        // recursively resolves {{base}} in the same pass.
        assert!(composed.content().contains("/path/spec.md"));
    }

    #[test]
    fn test_frontmatter_interpolation_body_still_skips_fenced_code() {
        // Inline code spans interpolate, but fenced blocks remain untouched
        // unless `interpolate_code_blocks` is set.
        let content =
            "---\nname: World\n---\nHello {{ name }}! Code: `{{ name }}`\n\n```\n{{ name }}\n```";
        let md: Markdown = content.into();
        let (composed, _) = md
            .compose_with(ComposeOptions::new().only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::Interpolation,
            ]))
            .unwrap();

        assert!(composed.content().contains("Hello World!"));
        assert!(composed.content().contains("Code: `World`"));
        assert!(composed.content().contains("```\n{{ name }}\n```"));
    }

    #[test]
    fn test_frontmatter_interpolation_report_counted_separately() {
        let content = "---\nbase: /path\nspec: \"{{base}}/spec.md\"\n---\nHello {{ spec }}!";
        let md: Markdown = content.into();
        let (_, report) = md
            .compose_with(ComposeOptions::new().only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::Interpolation,
            ]))
            .unwrap();

        assert_eq!(report.frontmatter_interpolations_applied, 1);
        assert_eq!(report.interpolations_applied, 1);
    }

    #[test]
    fn test_frontmatter_interpolation_summary() {
        let mut report = ComposeReport::new();
        report.frontmatter_interpolations_applied = 2;
        let summary = report.summary();
        assert!(summary.contains("2 frontmatter interpolation(s)"));
    }

    #[test]
    fn test_frontmatter_interpolation_report_merge() {
        let mut r1 = ComposeReport::new();
        r1.frontmatter_interpolations_applied = 3;
        let mut r2 = ComposeReport::new();
        r2.frontmatter_interpolations_applied = 5;
        r1.merge(r2);
        assert_eq!(r1.frontmatter_interpolations_applied, 8);
    }

    // ── Nested external state regression tests ────────────────────────

    #[test]
    fn test_frontmatter_interpolation_nested_external_state() {
        // External state has nested keys; frontmatter references them.
        let content = "---\nmeta:\n  author: Local\nspec: \"{{meta.base}}/spec.md\"\n---\n{{spec}}";
        let md: Markdown = content.into();
        let (composed, report) = md
            .compose_with(
                ComposeOptions::new()
                    .with_external_state(serde_json::json!({
                        "meta": {"base": "/root", "author": "Parent"}
                    }))
                    .only(&[
                        ComposeOperation::FrontmatterInterpolation,
                        ComposeOperation::Interpolation,
                    ]),
            )
            .unwrap();

        // meta.base from external state should be deep-merged in
        assert!(
            composed.content().contains("/root/spec.md"),
            "Expected /root/spec.md but got: {}",
            composed.content()
        );
        // frontmatter author should win over external
        assert_eq!(
            composed
                .frontmatter()
                .as_map()
                .get("meta")
                .and_then(|v| v.get("author")),
            Some(&serde_json::json!("Local"))
        );
        assert!(report.frontmatter_interpolations_applied >= 1);
    }

    #[test]
    fn test_external_state_deep_merge_preserves_frontmatter_values() {
        // Both frontmatter and external have nested objects; frontmatter wins on conflict.
        let content =
            "---\nconfig:\n  theme: dark\n---\ntheme={{config.theme}} lang={{config.lang}}";
        let md: Markdown = content.into();
        let (composed, _) = md
            .compose_with(
                ComposeOptions::new()
                    .with_external_state(serde_json::json!({
                        "config": {"theme": "light", "lang": "en"}
                    }))
                    .only(&[ComposeOperation::Interpolation]),
            )
            .unwrap();

        assert!(
            composed.content().contains("theme=dark"),
            "Frontmatter should win: {}",
            composed.content()
        );
        assert!(
            composed.content().contains("lang=en"),
            "External nested key should fill in: {}",
            composed.content()
        );
    }

    // ── Child document frontmatter from parent state ──────────────────

    #[test]
    fn test_child_frontmatter_interpolation_from_parent_state() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let child = dir.path().join("child.md");

        std::fs::write(&root, "---\nbase: /docs\n---\n::file ./child.md").unwrap();
        std::fs::write(
            &child,
            "---\nspec: \"{{base}}/spec.md\"\n---\nSpec: {{spec}}",
        )
        .unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, _) = md.compose_with(options).unwrap();

        assert!(
            composed.content().contains("Spec: /docs/spec.md"),
            "Child should derive frontmatter from parent state: {}",
            composed.content()
        );
    }

    // ── Interpolated prologue/epilogue paths ──────────────────────────

    #[test]
    fn test_interpolated_prologue_path() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let intro = dir.path().join("intro.md");

        std::fs::write(
            &root,
            "---\nparts: .\nprologue: \"{{parts}}/intro.md\"\n---\nBody",
        )
        .unwrap();
        std::fs::write(&intro, "Prologue content").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(
            composed.content().contains("Prologue content"),
            "Interpolated prologue path should resolve: {}",
            composed.content()
        );
        assert!(report.frontmatter_interpolations_applied >= 1);
    }

    #[test]
    fn test_interpolated_epilogue_path() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root.md");
        let outro = dir.path().join("outro.md");

        std::fs::write(
            &root,
            "---\nparts: .\nepilogue: \"{{parts}}/outro.md\"\n---\nBody",
        )
        .unwrap();
        std::fs::write(&outro, "Epilogue content").unwrap();

        let md = Markdown::try_from(root.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(root);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(
            composed.content().contains("Epilogue content"),
            "Interpolated epilogue path should resolve: {}",
            composed.content()
        );
        assert!(report.frontmatter_interpolations_applied >= 1);
    }

    // ── Page blocks consuming interpolated frontmatter values ─────────

    #[test]
    fn test_page_block_uses_interpolated_frontmatter() {
        // Frontmatter interpolation produces a value that page blocks consume.
        let content = "---\nbase: show\nflag: \"{{base}}\"\n---\n\n::block when=\"flag\"\n\nVisible\n\n::end-block\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::PageBlocks,
        ]);
        let (composed, report) = md.compose_with(options).unwrap();

        assert!(
            composed.content().contains("Visible"),
            "Page block should see interpolated frontmatter value: {}",
            composed.content()
        );
        assert!(report.frontmatter_interpolations_applied >= 1);
        assert!(report.page_blocks_rendered >= 1);
    }

    #[test]
    fn test_page_block_false_from_interpolated_frontmatter() {
        let content = "---\nbase: \"\"\nflag: \"{{base}}\"\n---\n\n::block when=\"flag\"\n\nHidden\n\n::end-block\n\nAfter\n";
        let md: Markdown = content.into();

        let options = ComposeOptions::new().only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::PageBlocks,
        ]);
        let (composed, _) = md.compose_with(options).unwrap();

        assert!(
            !composed.content().contains("Hidden"),
            "Page block with falsy interpolated value should be removed: {}",
            composed.content()
        );
        assert!(composed.content().contains("After"));
    }

    mod frontmatter_shell_expansion_integration {
        use super::*;
        use crate::markdown::compose::shell_expansion::types::{
            ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
            ShellExpansionOptions,
        };
        use std::sync::Arc;
        use std::time::Duration;
        use tempfile::TempDir;

        struct MockApproval;
        impl ShellApprovalHandler for MockApproval {
            fn approve(
                &self,
                _req: ShellApprovalRequest,
            ) -> Result<ShellApprovalDecision, ShellExpansionError> {
                Ok(ShellApprovalDecision::AllowOnce)
            }
        }

        #[test]
        fn frontmatter_shell_output_visible_to_body_interpolation() {
            let temp_dir = TempDir::new().unwrap();
            let content = "---\ngreeting: \"$(echo hello)\"\n---\nMessage: {{greeting}}\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::FrontmatterShellExpansion,
                    ComposeOperation::Interpolation,
                ])
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let (composed, report) = md.compose_with(options).unwrap();
            assert!(
                composed.content().contains("Message: hello"),
                "Expected 'Message: hello' in:\n{}",
                composed.content()
            );
            assert_eq!(report.frontmatter_shell_expansions_applied, 1);
        }

        #[test]
        fn frontmatter_interpolation_feeds_into_shell_expansion() {
            let temp_dir = TempDir::new().unwrap();
            let content = "---\nfile: README.md\ndir: \"$(dirname {{file}})\"\n---\nDir: {{dir}}\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::FrontmatterShellExpansion,
                    ComposeOperation::Interpolation,
                ])
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let (composed, report) = md.compose_with(options).unwrap();
            // dirname README.md returns "."
            assert!(
                composed.content().contains("Dir: ."),
                "Expected 'Dir: .' in:\n{}",
                composed.content()
            );
            assert_eq!(report.frontmatter_shell_expansions_applied, 1);
        }

        #[test]
        fn body_and_frontmatter_shell_coexist() {
            let temp_dir = TempDir::new().unwrap();
            let content =
                "---\nfm_val: \"$(echo from-frontmatter)\"\n---\n::shell echo from-body\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterShellExpansion,
                    ComposeOperation::ShellExpansion,
                ])
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let (composed, report) = md.compose_with(options).unwrap();
            assert_eq!(report.frontmatter_shell_expansions_applied, 1);
            assert_eq!(report.shell_expansions_applied, 1);
            assert!(composed.content().contains("from-body"));
        }

        #[test]
        fn frontmatter_shell_with_no_candidates_is_noop() {
            let content = "---\ntitle: Hello\n---\nBody text\n";
            let md: Markdown = content.into();

            let options =
                ComposeOptions::new().only(&[ComposeOperation::FrontmatterShellExpansion]);

            let (composed, report) = md.compose_with(options).unwrap();
            assert_eq!(report.frontmatter_shell_expansions_applied, 0);
            assert!(composed.content().contains("Body text"));
        }

        #[test]
        fn frontmatter_shell_timeout_empty_emits_warning() {
            let temp_dir = TempDir::new().unwrap();
            let content = "---\nval: \"$(sleep 1)\"\n---\nValue: {{val}}\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterShellExpansion,
                    ComposeOperation::Interpolation,
                ])
                .with_shell(ShellExpansionOptions {
                    timeout: Duration::from_millis(100),
                    timeout_behavior: super::ShellTimeoutBehavior::EmptyString,
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let (composed, report) = md.compose_with(options).unwrap();
            assert!(composed.content().contains("Value: "));
            assert_eq!(report.frontmatter_shell_expansions_applied, 1);
            assert_eq!(report.warnings.len(), 1);
            assert!(report.warnings[0].message.contains("timed out"));
        }

        #[test]
        fn frontmatter_shell_rejects_interpolated_executable() {
            let content = "---\ncmd_name: echo\nval: \"$({{cmd_name}} hello)\"\n---\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new().only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
            ]);

            let err = md.compose_with(options).unwrap_err();
            assert!(
                err.to_string()
                    .contains("Frontmatter shell executable may not come from interpolation")
            );
        }

        #[test]
        fn frontmatter_shell_rejects_pipe_in_command() {
            let temp_dir = TempDir::new().unwrap();
            let content = "---\nval: \"$(echo a | cat)\"\n---\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[ComposeOperation::FrontmatterShellExpansion])
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let err = md.compose_with(options).unwrap_err();
            assert!(
                err.to_string().contains("pipes") || err.to_string().contains("Shell pipes"),
                "Expected shell pipe rejection, got: {}",
                err
            );
        }

        #[test]
        fn frontmatter_shell_or_chain_works() {
            let temp_dir = TempDir::new().unwrap();
            let content = "---\nval: \"$(false || echo fallback)\"\n---\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[ComposeOperation::FrontmatterShellExpansion])
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let (composed, _report) = md.compose_with(options).unwrap();
            assert_eq!(
                composed.frontmatter().as_map().get("val"),
                Some(&serde_json::json!("fallback"))
            );
        }

        #[test]
        fn ternary_motivating_workflow_true_branch_through_full_pipeline() {
            // Review finding 4: exercise the motivating spec_file workflow
            // through the full compose pipeline so frontmatter interpolation,
            // pre-interpolation snapshot capture, and frontmatter shell
            // expansion are all wired together. With `has_spec: true` the
            // then-branch wins and produces the basename of the spec path.
            let temp_dir = TempDir::new().unwrap();
            let content = concat!(
                "---\n",
                "has_spec: true\n",
                "spec: /tmp/example-spec.md\n",
                "spec_file: \"$({{has_spec}} ? basename {{spec}} : '')\"\n",
                "---\n",
                "Spec: {{spec_file}}\n",
            );
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::FrontmatterShellExpansion,
                    ComposeOperation::Interpolation,
                ])
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let (composed, report) = md.compose_with(options).unwrap();
            assert_eq!(report.frontmatter_shell_expansions_applied, 1);
            assert_eq!(
                composed.frontmatter().as_map().get("spec_file"),
                Some(&serde_json::json!("example-spec.md"))
            );
            assert!(
                composed.content().contains("Spec: example-spec.md"),
                "Expected body to interpolate spec_file, got:\n{}",
                composed.content()
            );
        }

        #[test]
        fn ternary_motivating_workflow_false_branch_through_full_pipeline() {
            // Counterpart to the true-branch test: with `has_spec: false`
            // the else-branch (`''`) wins, short-circuiting to an empty
            // string without invoking the shell.
            let temp_dir = TempDir::new().unwrap();
            let content = concat!(
                "---\n",
                "has_spec: false\n",
                "spec: /tmp/example-spec.md\n",
                "spec_file: \"$({{has_spec}} ? basename {{spec}} : '')\"\n",
                "---\n",
                "Spec: {{spec_file}}\n",
            );
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::FrontmatterShellExpansion,
                    ComposeOperation::Interpolation,
                ])
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let (composed, report) = md.compose_with(options).unwrap();
            assert_eq!(report.frontmatter_shell_expansions_applied, 1);
            assert_eq!(
                composed.frontmatter().as_map().get("spec_file"),
                Some(&serde_json::json!(""))
            );
            assert!(
                composed.content().contains("Spec: "),
                "Expected body to render with empty spec_file, got:\n{}",
                composed.content()
            );
        }

        #[test]
        fn ternary_stringified_false_condition_selects_else_branch_in_pipeline() {
            // Review finding 2 at compose level: when an earlier frontmatter
            // interpolation rewrites `has_spec` from a boolean into the
            // string `"false"`, the ternary condition must still resolve
            // to the else-branch.
            let temp_dir = TempDir::new().unwrap();
            let content = concat!(
                "---\n",
                "raw_false: false\n",
                "has_spec: \"{{raw_false}}\"\n",
                "spec_file: \"$({{has_spec}} ? echo present : '')\"\n",
                "---\n",
            );
            let md: Markdown = content.into();

            let options = ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::FrontmatterShellExpansion,
                ])
                .with_shell(ShellExpansionOptions {
                    policy_root: Some(temp_dir.path().to_path_buf()),
                    approval_handler: Some(Arc::new(MockApproval)),
                    ..Default::default()
                });

            let (composed, _report) = md.compose_with(options).unwrap();
            // has_spec is rendered to the string "false"; the ternary must
            // see it as boolean-false and pick the empty branch.
            assert_eq!(
                composed.frontmatter().as_map().get("has_spec"),
                Some(&serde_json::json!("false"))
            );
            assert_eq!(
                composed.frontmatter().as_map().get("spec_file"),
                Some(&serde_json::json!(""))
            );
        }
    }

    mod infix_logic_conditions {
        use super::*;

        fn compose_with_page_blocks(content: &str) -> (String, ComposeReport) {
            let md: Markdown = content.into();
            let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);
            let (composed, report) = md.compose_with(options).unwrap();
            (composed.content().to_string(), report)
        }

        #[test]
        fn page_block_with_infix_and_true() {
            let content =
                "---\na: true\nb: true\n---\n::block when=\"a && b\"\ninside\n::end-block\n";
            let (output, report) = compose_with_page_blocks(content);
            assert!(output.contains("inside"));
            assert_eq!(report.page_blocks_rendered, 1);
            assert_eq!(report.page_blocks_skipped, 0);
        }

        #[test]
        fn page_block_with_infix_and_false() {
            let content =
                "---\na: true\nb: false\n---\n::block when=\"a && b\"\ninside\n::end-block\n";
            let (output, report) = compose_with_page_blocks(content);
            assert!(!output.contains("inside"));
            assert_eq!(report.page_blocks_rendered, 0);
            assert_eq!(report.page_blocks_skipped, 1);
        }

        #[test]
        fn page_block_with_infix_or_one_true() {
            let content =
                "---\na: false\nb: true\n---\n::block when=\"a || b\"\ninside\n::end-block\n";
            let (output, report) = compose_with_page_blocks(content);
            assert!(output.contains("inside"));
            assert_eq!(report.page_blocks_rendered, 1);
        }

        #[test]
        fn page_block_with_infix_or_both_false() {
            let content =
                "---\na: false\nb: false\n---\n::block when=\"a || b\"\ninside\n::end-block\n";
            let (output, _report) = compose_with_page_blocks(content);
            assert!(!output.contains("inside"));
        }

        #[test]
        fn page_block_with_grouped_precedence() {
            // (a || b) && c — grouping overrides default precedence
            let content = "---\na: false\nb: true\nc: true\n---\n::block when=\"(a || b) && c\"\ninside\n::end-block\n";
            let (output, _report) = compose_with_page_blocks(content);
            assert!(output.contains("inside"));

            let content_false = "---\na: false\nb: true\nc: false\n---\n::block when=\"(a || b) && c\"\ninside\n::end-block\n";
            let (output, _report) = compose_with_page_blocks(content_false);
            assert!(!output.contains("inside"));
        }

        #[test]
        fn page_block_with_chained_or() {
            // Chained `||` in condition mode evaluates as logical OR
            let content = "---\na: false\nb: false\nc: true\n---\n::block when=\"a || b || c\"\ninside\n::end-block\n";
            let (output, _report) = compose_with_page_blocks(content);
            assert!(output.contains("inside"));
        }

        #[test]
        fn transclusion_directive_with_mixed_infix_logic() {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let child = dir.path().join("child.md");

            // Child loaded only if (enabled || fallback) && !skip
            std::fs::write(&child, "child body").unwrap();
            std::fs::write(
                &root,
                "---\nenabled: true\nskip: false\n---\nbefore\n\n::file child.md when=\"enabled && !skip\"\n\nafter\n",
            )
            .unwrap();

            let options = ComposeOptions::new()
                .with_source_file(&root)
                .only(&[ComposeOperation::BlockTransclusion]);

            let (composed, _) = Markdown::try_from(root.as_path())
                .unwrap()
                .compose_with(options)
                .unwrap();
            assert!(composed.content().contains("child body"));
        }

        #[test]
        fn transclusion_skipped_when_infix_condition_false() {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let child = dir.path().join("child.md");

            std::fs::write(&child, "child body").unwrap();
            std::fs::write(
                &root,
                "---\nenabled: true\nskip: true\n---\nbefore\n\n::file child.md when=\"enabled && !skip\"\n\nafter\n",
            )
            .unwrap();

            let options = ComposeOptions::new()
                .with_source_file(&root)
                .only(&[ComposeOperation::BlockTransclusion]);

            let (composed, _) = Markdown::try_from(root.as_path())
                .unwrap()
                .compose_with(options)
                .unwrap();
            assert!(!composed.content().contains("child body"));
        }

        #[test]
        fn page_block_with_bare_pipe_fails_parse() {
            // Bare `|` in condition expressions should produce a parse error
            let content = "---\na: true\n---\n::block when=\"a | b\"\ninside\n::end-block\n";
            let md: Markdown = content.into();
            let options = ComposeOptions::new().only(&[ComposeOperation::PageBlocks]);
            let err = md.compose_with(options).unwrap_err();

            let err_string = format!("{}", err);
            assert!(
                err_string.contains("Unexpected '|'") || err_string.contains("logical OR"),
                "Expected bare pipe error in condition, got: {}",
                err_string
            );
        }
    }

    // ============================================
    // Schema Validation integration tests
    // ============================================

    mod schema_validation_integration {
        use super::*;

        #[test]
        fn schema_validation_fails_fast_before_shell_expansion() {
            // Document matching the shape of the failing planner prompt:
            // spec is empty, and dir uses shell expansion that would fail
            // if spec stays empty.
            let content = "---\n$schema:\n  spec: 'file(required)'\nspec: \"\"\ndir: \"$(dirname '{{ spec }}')\"\n---\nBody\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new().only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::Interpolation,
            ]);

            let err = md.compose_with(options).unwrap_err();
            let err_string = format!("{err}");
            assert!(
                err_string.contains("Schema validation failed"),
                "Expected schema validation error, got: {err_string}"
            );
            assert!(
                !err_string.contains("dirname"),
                "Shell expansion should not have run, got: {err_string}"
            );

            // The error variant itself should name the failing property.
            match err {
                MarkdownError::SchemaValidationFailed { problems, .. } => {
                    assert!(
                        problems.iter().any(|p| {
                            p.property.as_deref() == Some("spec") || p.path == "/spec"
                        }),
                        "Error should mention the spec property, got: {problems:?}"
                    );
                }
                other => panic!("Expected SchemaValidationFailed, got {other:?}"),
            }
        }

        #[test]
        fn schema_violation_on_shell_value_reported_when_shell_expansion_disabled() {
            // A `$(...)` frontmatter value violates the schema, but
            // FrontmatterShellExpansion is NOT in the enabled set. Because no
            // later stage will expand or re-validate `spec`, the violation must
            // surface here rather than being deferred and silently accepted.
            let content =
                "---\n$schema:\n  spec: 'number(required)'\nspec: \"$(echo 1)\"\n---\nBody\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new().only(&[ComposeOperation::Interpolation]);

            let err = md.compose_with(options).unwrap_err();
            match err {
                MarkdownError::SchemaValidationFailed { problems, .. } => {
                    assert!(
                        problems
                            .iter()
                            .any(|p| p.property.as_deref() == Some("spec") || p.path == "/spec"),
                        "Error should mention the spec property, got: {problems:?}"
                    );
                }
                other => panic!("Expected SchemaValidationFailed, got {other:?}"),
            }
        }

        #[test]
        fn schema_validation_reports_zero_shell_replacements() {
            let content = "---\n$schema:\n  spec: 'file(required)'\nspec: \"\"\ndir: \"$(dirname '{{ spec }}')\"\n---\nBody\n";
            let md: Markdown = content.into();

            // Even with fail_fast=false, schema validation is a hard error.
            let options = ComposeOptions::new()
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::FrontmatterShellExpansion,
                    ComposeOperation::Interpolation,
                ])
                .with_fail_fast(false);

            let err = md.compose_with(options).unwrap_err();
            match err {
                MarkdownError::SchemaValidationFailed { .. } => {}
                other => panic!("Expected SchemaValidationFailed, got {other:?}"),
            }
        }

        #[test]
        fn coercion_write_back_flows_to_composed_frontmatter() {
            // `has_spec` derives from a ternary, resolves to the string "true"
            // during frontmatter interpolation, and is coerced to a real JSON
            // bool by schema validation. The composed frontmatter must hold the
            // bool, not the string.
            let content = "---\n$schema:\n  spec: string(required)\n  has_spec: boolean\nspec: design.md\nhas_spec: \"{{spec ? true : false}}\"\n---\nBody\n";
            let md: Markdown = content.into();

            let options = ComposeOptions::new().only(&[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::Interpolation,
            ]);

            let (composed, _report) = md.compose_with(options).unwrap();
            assert_eq!(
                composed.frontmatter().as_map().get("has_spec"),
                Some(&serde_json::json!(true))
            );
        }

        #[test]
        fn implement_md_three_arm_union_ternaries_coerce_and_defer_shell() {
            // Faithful reproduction of the original failing `claudine compose
            // prompts/implement.md spec=… --claude` invocation: a 3-arm root
            // union where every arm types the `has_*` trio as strict `boolean`,
            // computed `has_*` ternaries that render into quoted scalars
            // ("true"/"false"), and a `$(...)`-bearing `dir`. A `spec=` value is
            // supplied via --set, so arm 2 (`spec: string(required)`) validates
            // post-coercion. Before this feature the strict `boolean` arms
            // rejected the "false"/"true" strings; now they coerce.
            //
            // Frontmatter shell expansion is left disabled to keep the test
            // hermetic (no real `dirname` invocation). `dir` is typed `string`,
            // so its literal `$(...)` value is already a valid string: coercion
            // skips it and validation raises no type problem, so it survives
            // untouched into the composed output as a deferred shell expression.
            let content = "---\n\
                $schema:\n\
                \x20 - review: string(required)\n\
                \x20   spec: string\n\
                \x20   iteration: number\n\
                \x20   has_plan: boolean\n\
                \x20   has_spec: boolean\n\
                \x20   has_review: boolean\n\
                \x20 - spec: string(required)\n\
                \x20   has_plan: boolean\n\
                \x20   has_spec: boolean\n\
                \x20   has_review: boolean\n\
                \x20 - plan: string(required)\n\
                \x20   spec: string\n\
                \x20   iteration: number\n\
                \x20   has_plan: boolean\n\
                \x20   has_spec: boolean\n\
                \x20   has_review: boolean\n\
                has_spec: \"{{spec ? true : false}}\"\n\
                has_plan: \"{{plan ? true : false}}\"\n\
                has_review: \"{{review ? true : false}}\"\n\
                dir: \"$(dirname '{{spec || plan}}')\"\n\
                ---\nBody\n";
            let md: Markdown = content.into();

            // `spec=` provided via --set; no `plan`/`review` → second arm wins.
            let options = ComposeOptions::new()
                .with_set_overrides(serde_json::json!({ "spec": "features/plan.md" }))
                .only(&[
                    ComposeOperation::FrontmatterInterpolation,
                    ComposeOperation::Interpolation,
                ]);

            let (composed, _report) = md
                .compose_with(options)
                .expect("compose should succeed once the has_* strings coerce");

            let fm = composed.frontmatter();
            let map = fm.as_map();
            // The motivating fix: the ternary-derived strings become real bools.
            assert_eq!(map.get("has_spec"), Some(&serde_json::json!(true)));
            assert_eq!(map.get("has_plan"), Some(&serde_json::json!(false)));
            assert_eq!(map.get("has_review"), Some(&serde_json::json!(false)));
            // The `$(...)` `dir` value is deferred: coercion skips it pre-shell,
            // and the unresolved interpolation/shell template never errored.
            let dir = map.get("dir").and_then(serde_json::Value::as_str).unwrap();
            assert!(
                dir.contains("$(") && dir.contains("dirname"),
                "dir should remain a deferred shell expression, got: {dir}"
            );
        }

        #[test]
        fn parent_set_overlay_satisfies_child_schema() {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let child = dir.path().join("child.md");

            // Child has a schema requiring child_input
            std::fs::write(
                &child,
                "---\n$schema:\n  child_input: 'string(required)'\n---\nChild body\n",
            )
            .unwrap();

            // Parent transcludes child with set.child_input="ok"
            std::fs::write(
                &root,
                "# Parent\n\n::file ./child.md set.child_input=\"ok\"\n",
            )
            .unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new().with_source_file(root);
            let (composed, report) = md.compose_with(options).unwrap();

            assert!(composed.content().contains("Child body"));
            assert_eq!(report.transclusions_applied, 1);
        }

        #[test]
        fn parent_set_overlay_missing_child_schema_fails() {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let child = dir.path().join("child.md");

            // Child has a schema requiring child_input
            std::fs::write(
                &child,
                "---\n$schema:\n  child_input: 'string(required)'\n---\nChild body\n",
            )
            .unwrap();

            // Parent transcludes child WITHOUT the set overlay
            std::fs::write(&root, "# Parent\n\n::file ./child.md\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            // fail_fast=true so the schema validation error propagates rather
            // than being downgraded to a transclusion warning.
            let options = ComposeOptions::new()
                .with_source_file(root)
                .with_fail_fast(true);
            let err = md.compose_with(options).unwrap_err();

            match err {
                MarkdownError::SchemaValidationFailed { problems, .. } => {
                    assert!(
                        problems.iter().any(|p| p.property.as_deref() == Some("child_input")),
                        "Expected problem on child_input, got: {problems:?}"
                    );
                }
                other => panic!("Expected SchemaValidationFailed, got {other:?}"),
            }
        }

        /// Different baseline schemas must not share cache entries for the same
        /// transcluded child. Compose the same parent+child three times against
        /// a shared persistent cache:
        ///
        /// 1. baseline A → cold cache, child is computed and written to the
        ///    persistent store (`persistent_hits == 0`, `persistent_writes >= 1`).
        /// 2. baseline A again → cache is warm and the child compose entry is
        ///    reused (`persistent_hits >= 1`).
        /// 3. baseline B → baseline differs, so the persistent cache key
        ///    differs; the child must be recomputed rather than reuse the
        ///    baseline-A entry (`persistent_hits == 0` again).
        ///
        /// This proves `options_hash` includes `baseline_schema` in a way that
        /// actually invalidates the persistent cache — guarding against the
        /// "stale success keyed without baseline" regression.
        #[test]
        fn baseline_cache_does_not_reuse_across_distinct_baselines() {
            use crate::markdown::compose::CacheAccessMode;
            use crate::markdown::schemas::{
                Constraint, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema,
                SimplifiedType, TypeExpr,
            };
            use indexmap::IndexMap;

            fn baseline_required(prop: &str) -> SimplifiedSchema {
                let mut properties = IndexMap::new();
                properties.insert(
                    prop.into(),
                    PropertyDef::Single(PropertyAtom {
                        ty: TypeExpr::Primitive(SimplifiedType::String),
                        is_array: false,
                        constraints: vec![Constraint::Required],
                        array_constraints: vec![],
                        description: None,
                    }),
                );
                SimplifiedSchema::Single(SchemaShape { properties })
            }

            let dir = tempfile::tempdir().unwrap();
            let cache_root = dir.path().join("cache");
            let root = dir.path().join("root.md");
            let child = dir.path().join("child.md");

            // Parent supplies both `alpha` and `beta` so it (and its effective
            // state inherited by the child) satisfies either baseline under
            // test. Cache invalidation is the contract we care about here, not
            // the validation outcome.
            std::fs::write(&child, "---\nalpha: ok\nbeta: ok\n---\nChild body\n").unwrap();
            std::fs::write(
                &root,
                "---\nalpha: ok\nbeta: ok\n---\n# Parent\n\n::file ./child.md\n",
            )
            .unwrap();

            let mk_options = |baseline_prop: &str| {
                ComposeOptions::new()
                    .with_source_file(&root)
                    .with_baseline_schema(baseline_required(baseline_prop))
                    .with_cache_access_mode(CacheAccessMode::ReadWrite)
                    .with_cache_root(&cache_root)
                    .with_cache_namespace("baseline_cache_regression")
                    .with_fail_fast(true)
            };

            // ── Run 1: cold cache under baseline A ─────────────────────
            let md1 = Markdown::try_from(root.as_path()).unwrap();
            let (_, report1) = md1
                .compose_with(mk_options("alpha"))
                .expect("run 1 (baseline alpha, cold cache) should succeed");
            let stats1 = report1
                .cache_stats
                .expect("expected cache stats with cache enabled");
            assert_eq!(
                stats1.persistent_hits, 0,
                "run 1 should have a cold persistent cache, got {stats1:?}"
            );
            assert!(
                stats1.persistent_writes >= 1,
                "run 1 must write the child compose to the persistent cache, got {stats1:?}"
            );

            // ── Run 2: same baseline A → cache should be warm ──────────
            let md2 = Markdown::try_from(root.as_path()).unwrap();
            let (_, report2) = md2
                .compose_with(mk_options("alpha"))
                .expect("run 2 (baseline alpha, warm cache) should succeed");
            let stats2 = report2
                .cache_stats
                .expect("expected cache stats with cache enabled");
            assert!(
                stats2.persistent_hits >= 1,
                "run 2 must reuse the warmed persistent entry, got {stats2:?}",
            );

            // ── Run 3: baseline B → distinct key, must not reuse run 1 ─
            let md3 = Markdown::try_from(root.as_path()).unwrap();
            let (_, report3) = md3
                .compose_with(mk_options("beta"))
                .expect("run 3 (baseline beta) should succeed");
            let stats3 = report3
                .cache_stats
                .expect("expected cache stats with cache enabled");
            assert_eq!(
                stats3.persistent_hits, 0,
                "run 3 must NOT reuse the baseline-A entry — options_hash must include \
                 baseline_schema. got {stats3:?}",
            );
            assert!(
                stats3.persistent_writes >= 1,
                "run 3 must compute and write a fresh entry under the new baseline, got {stats3:?}"
            );
        }
    }

    mod remote_transclusion_tests {
        use super::*;
        use crate::markdown::compose::remote::RemoteReadConfig;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        use wiremock::matchers::{method, path};

        async fn compose_with_remote(
            content: &str,
            source_file: &std::path::Path,
            _server: &MockServer,
            allowed_hosts: Vec<String>,
        ) -> MarkdownResult<(Markdown, ComposeReport)> {
            let md: Markdown = content.into();
            let config = RemoteReadConfig {
                allowed_hosts,
                ..Default::default()
            };
            let options = ComposeOptions::new()
                .with_source_file(source_file)
                .with_allow_remote_transclusion(true)
                .with_remote_read_config(config)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            md.compose_with(options)
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn remote_file_transclusion_inserts_fetched_body() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/remote.md"))
                .respond_with(ResponseTemplate::new(200).set_body_string("# Remote\n\nHello from remote"))
                .mount(&server)
                .await;

            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let remote_url = format!("{}/remote.md", server.uri());
            let content = format!("# Local\n\n::file {remote_url}\n");
            std::fs::write(&root, &content).unwrap();

            let (composed, report) =
                compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                    .await
                    .unwrap();

            let text = composed.content();
            assert!(text.contains("Hello from remote"), "content: {text}");
            assert!(text.contains("# Local"), "content: {text}");
            assert_eq!(report.transclusions_applied, 1);
            let rf = report.remote_fetch_stats.unwrap();
            assert_eq!(rf.fetched, 1);
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn remote_code_transclusion_inserts_fetched_code() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/snippet.rs"))
                .respond_with(ResponseTemplate::new(200).set_body_string("fn main() {}"))
                .mount(&server)
                .await;

            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let remote_url = format!("{}/snippet.rs", server.uri());
            let content = format!("# Doc\n\n::code {remote_url}\n");
            std::fs::write(&root, &content).unwrap();

            let (composed, report) =
                compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                    .await
                    .unwrap();

            let text = composed.content();
            assert!(text.contains("fn main()"), "content: {text}");
            assert!(text.contains("```rs"), "content: {text}");
            assert_eq!(report.transclusions_applied, 1);
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn remote_transclusion_denied_by_default() {
            let server = MockServer::start().await;

            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let remote_url = format!("{}/blocked.md", server.uri());
            let content = format!("# Doc\n\n::file {remote_url}\n");
            std::fs::write(&root, &content).unwrap();

            let md: Markdown = content.clone().into();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .with_allow_remote_transclusion(true)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let result = md.compose_with(options);
            assert!(
                result.is_err(),
                "Expected error because no allowed hosts configured"
            );
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn remote_file_duplicate_consumers_one_fetch() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/shared.md"))
                .respond_with(ResponseTemplate::new(200).set_body_string("shared content"))
                .mount(&server)
                .await;

            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let remote_url = format!("{}/shared.md", server.uri());
            // Two line-start directives referencing the same URL: directives
            // are line-oriented, so each must begin its own line.
            let content = format!(
                "# Doc\n\n::file {remote_url}\n\n::file {remote_url}\n"
            );
            std::fs::write(&root, &content).unwrap();

            let (composed, report) =
                compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                    .await
                    .unwrap();

            let text = composed.content();
            let count = text.matches("shared content").count();
            assert_eq!(count, 2, "Should appear twice (once per directive)");
            assert_eq!(report.transclusions_applied, 2);
            let rf = report.remote_fetch_stats.unwrap();
            assert_eq!(rf.fetched, 1, "Only one actual network fetch should occur");
        }

        /// Mounts a single document and composes a body that reads it through
        /// the given quoted URL expression, returning the composed text.
        ///
        /// The URL argument is quoted because the interpolation expression
        /// parser only accepts a string literal there; the unquoted
        /// `frontmatter(https://…)` form does not tokenize.
        async fn compose_expr_against_doc(doc_body: &str, expr_template: &str) -> String {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/doc.md"))
                .respond_with(ResponseTemplate::new(200).set_body_string(doc_body.to_string()))
                .mount(&server)
                .await;
            let url = format!("{}/doc.md", server.uri());
            let body = expr_template.replace("{URL}", &url);

            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            std::fs::write(&root, &body).unwrap();

            let (composed, _) =
                compose_with_remote(&body, &root, &server, vec!["127.0.0.1".into()])
                    .await
                    .unwrap();
            composed.content().to_string()
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn remote_frontmatter_expression_reads_url() {
            let text = compose_expr_against_doc(
                "---\ntitle: Remote Title\nstatus: draft\n---\n# H1\n\nBody\n",
                "S: {{ frontmatter(\"{URL}\", \"status\") }}\n",
            )
            .await;
            assert_eq!(text, "S: draft\n");
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn remote_markdown_title_expression_reads_url() {
            let text = compose_expr_against_doc(
                "---\ntitle: Remote Title\n---\n# H1\n\nBody\n",
                "T: {{ markdown_title(\"{URL}\") }}\n",
            )
            .await;
            assert_eq!(text, "T: Remote Title\n");
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn remote_markdown_body_empty_expression_reads_url() {
            let text = compose_expr_against_doc(
                "---\ntitle: Empty Body\n---\n",
                "E: {{ markdown_body_empty(\"{URL}\") }}\n",
            )
            .await;
            assert_eq!(text, "E: true\n");
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn remote_validate_schema_expression_reads_url() {
            // A document without `$schema` always validates as `true`.
            let text = compose_expr_against_doc(
                "---\ntitle: No Schema\n---\n# H1\n",
                "V: {{ validate_schema(\"{URL}\") }}\n",
            )
            .await;
            assert_eq!(text, "V: true\n");
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn remote_file_exists_expression_reads_url() {
            let text = compose_expr_against_doc(
                "# Present\n",
                "X: {{ file_exists(\"{URL}\") }}\n",
            )
            .await;
            assert_eq!(text, "X: true\n");
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn file_exists_false_for_unallowed_remote_host() {
            // Remote reads enabled but no allowed hosts: the URL is denied and
            // never fetched, so `file_exists` reads it as non-existent rather
            // than erroring out of composition.
            let server = MockServer::start().await;
            let url = format!("{}/blocked.md", server.uri());

            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let body = format!("X: {{{{ file_exists(\"{url}\") }}}}\n");
            std::fs::write(&root, &body).unwrap();

            let (composed, _) = compose_with_remote(&body, &root, &server, vec![])
                .await
                .unwrap();
            assert_eq!(composed.content(), "X: false\n");
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn interpolation_only_discovers_and_reads_remote_expression_url() {
            // Review 7: a library caller following the documented API enables
            // remote expression reads via `with_remote_read_config` alone — an
            // allowed host is sufficient. No `with_allow_remote_transclusion`
            // call is required, since expression URL reads are a read-side
            // capability independent of block transclusion.
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/doc.md"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string("---\ntitle: Interp Only\n---\n# H1\n"),
                )
                .mount(&server)
                .await;
            let url = format!("{}/doc.md", server.uri());

            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let body = format!("T: {{{{ markdown_title(\"{url}\") }}}}\n");
            std::fs::write(&root, &body).unwrap();

            let config = RemoteReadConfig {
                allowed_hosts: vec!["127.0.0.1".into()],
                ..Default::default()
            };
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .with_remote_read_config(config)
                .only(&[ComposeOperation::Interpolation]);
            let md: Markdown = body.clone().into();
            let (composed, report) = md.compose_with(options).unwrap();

            assert_eq!(composed.content(), "T: Interp Only\n");
            let rf = report.remote_fetch_stats.unwrap();
            assert_eq!(rf.fetched, 1, "URL must be fetched without BlockTransclusion");
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn local_markdown_title_expression_reads_relative_file() {
            // The same resolution-context wiring resolves local relative paths
            // against the source document's directory.
            let server = MockServer::start().await;
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(
                dir.path().join("other.md"),
                "---\ntitle: Local Title\n---\n# H1\n",
            )
            .unwrap();
            let root = dir.path().join("root.md");
            let body = "T: {{ markdown_title(\"./other.md\") }}\n";
            std::fs::write(&root, body).unwrap();

            let (composed, _) = compose_with_remote(body, &root, &server, vec![])
                .await
                .unwrap();
            assert_eq!(composed.content(), "T: Local Title\n");
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn nested_remote_reference_is_discovered_and_fetched() {
            let server = MockServer::start().await;
            // The parent document itself transcludes a further remote child.
            let child_url = format!("{}/child.md", server.uri());
            Mock::given(method("GET"))
                .and(path("/parent.md"))
                .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                    "# Parent\n\n::file {child_url}\n"
                )))
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path("/child.md"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_string("nested child body"),
                )
                .mount(&server)
                .await;

            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let parent_url = format!("{}/parent.md", server.uri());
            let content = format!("# Local\n\n::file {parent_url}\n");
            std::fs::write(&root, &content).unwrap();

            let (composed, report) =
                compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                    .await
                    .unwrap();

            let text = composed.content();
            assert!(text.contains("nested child body"), "content: {text}");
            let rf = report.remote_fetch_stats.unwrap();
            assert_eq!(rf.fetched, 2, "Both parent and nested child fetched once");
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn interpolated_directive_creates_fetchable_remote_file() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/late.md"))
                .respond_with(ResponseTemplate::new(200).set_body_string("late remote body"))
                .mount(&server)
                .await;

            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let remote_url = format!("{}/late.md", server.uri());
            // The directive's URL only materializes after interpolation expands
            // `{{ remote_ref }}`, so the eager pre-scan never sees it. It must be
            // registered when prepared, or point-of-use fails "not registered".
            let content = format!(
                "---\nremote_ref: \"{remote_url}\"\n---\n# Local\n\n::file {{{{ remote_ref }}}}\n"
            );
            std::fs::write(&root, &content).unwrap();

            let (composed, report) =
                compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                    .await
                    .unwrap();

            let text = composed.content();
            assert!(text.contains("late remote body"), "content: {text}");
            let rf = report.remote_fetch_stats.unwrap();
            assert_eq!(rf.fetched, 1);
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn interpolated_directive_creates_fetchable_remote_code() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/late.rs"))
                .respond_with(ResponseTemplate::new(200).set_body_string("fn main() {}"))
                .mount(&server)
                .await;

            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let remote_url = format!("{}/late.rs", server.uri());
            let content = format!(
                "---\nremote_ref: \"{remote_url}\"\n---\n# Doc\n\n::code {{{{ remote_ref }}}}\n"
            );
            std::fs::write(&root, &content).unwrap();

            let (composed, report) =
                compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                    .await
                    .unwrap();

            let text = composed.content();
            assert!(text.contains("fn main()"), "content: {text}");
            assert!(text.contains("```rs"), "content: {text}");
            let rf = report.remote_fetch_stats.unwrap();
            assert_eq!(rf.fetched, 1);
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn cached_local_child_revalidates_nested_remote_under_refresh() {
            use std::sync::Arc;
            use std::sync::atomic::{AtomicUsize, Ordering};
            use wiremock::{Request, Respond};

            // Serves "remote v1" on the first request and "remote v2" on every
            // later request, so a missed revalidation surfaces as stale output.
            struct Versioned {
                hits: Arc<AtomicUsize>,
            }
            impl Respond for Versioned {
                fn respond(&self, _req: &Request) -> ResponseTemplate {
                    let n = self.hits.fetch_add(1, Ordering::SeqCst);
                    let body = if n == 0 { "remote v1" } else { "remote v2" };
                    ResponseTemplate::new(200).set_body_string(body)
                }
            }

            let server = MockServer::start().await;
            let hits = Arc::new(AtomicUsize::new(0));
            Mock::given(method("GET"))
                .and(path("/remote.md"))
                .respond_with(Versioned {
                    hits: Arc::clone(&hits),
                })
                .mount(&server)
                .await;

            let dir = tempfile::tempdir().unwrap();
            let cache_root = dir.path().join("cache");
            let root = dir.path().join("root.md");
            let child = dir.path().join("child.md");
            let remote_url = format!("{}/remote.md", server.uri());
            std::fs::write(&root, "# Root\n\n::file ./child.md\n").unwrap();
            std::fs::write(&child, format!("# Child\n\n::file {remote_url}\n")).unwrap();

            let mk_options = |refresh: bool| {
                let config = RemoteReadConfig {
                    allowed_hosts: vec!["127.0.0.1".into()],
                    refresh,
                    ..Default::default()
                };
                ComposeOptions::new()
                    .with_source_file(&root)
                    .with_allow_remote_transclusion(true)
                    .with_remote_read_config(config)
                    .with_cache_root(&cache_root)
                    .disable(ComposeOperation::Cleanup)
                    .disable(ComposeOperation::Normalization)
            };

            // Run 1: populate the local-child and remote caches; remote → v1.
            let md1 = Markdown::try_from(root.as_path()).unwrap();
            let (c1, _) = md1.compose_with(mk_options(false)).unwrap();
            assert!(
                c1.content().contains("remote v1"),
                "run 1 should embed the original remote body: {}",
                c1.content()
            );

            // Run 2: the remote body has changed and `--remote-refresh` forces a
            // revalidation. The cached local child must NOT be accepted against
            // the stale remote manifest.
            let md2 = Markdown::try_from(root.as_path()).unwrap();
            let (c2, _) = md2.compose_with(mk_options(true)).unwrap();
            assert!(
                c2.content().contains("remote v2"),
                "cached local child must revalidate its nested remote URL under \
                 --remote-refresh; got: {}",
                c2.content()
            );
            assert!(
                !c2.content().contains("remote v1"),
                "stale remote body served from cache: {}",
                c2.content()
            );
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn remote_prologue_inserts_fetched_body() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/intro.md"))
                .respond_with(ResponseTemplate::new(200).set_body_string("# Intro\n\nFrom prologue"))
                .mount(&server)
                .await;

            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let remote_url = format!("{}/intro.md", server.uri());
            let content = format!("---\nprologue: {remote_url}\n---\n# Local\n\nBody.\n");
            std::fs::write(&root, &content).unwrap();

            let (composed, report) =
                compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                    .await
                    .unwrap();

            let text = composed.content();
            assert!(text.contains("From prologue"), "content: {text}");
            assert!(text.contains("# Local"), "content: {text}");
            assert_eq!(report.transclusions_applied, 1);
            let rf = report.remote_fetch_stats.unwrap();
            assert_eq!(rf.fetched, 1);
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn remote_epilogue_inserts_fetched_body() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/outro.md"))
                .respond_with(ResponseTemplate::new(200).set_body_string("# Outro\n\nFrom epilogue"))
                .mount(&server)
                .await;

            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let remote_url = format!("{}/outro.md", server.uri());
            let content = format!("---\nepilogue: [\"{remote_url}\"]\n---\n# Local\n\nBody.\n");
            std::fs::write(&root, &content).unwrap();

            let (composed, report) =
                compose_with_remote(&content, &root, &server, vec!["127.0.0.1".into()])
                    .await
                    .unwrap();

            let text = composed.content();
            assert!(text.contains("From epilogue"), "content: {text}");
            assert!(text.contains("# Local"), "content: {text}");
            assert_eq!(report.transclusions_applied, 1);
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn remote_prologue_denied_by_policy() {
            // Remote transclusion is enabled but the host is not allowlisted, so
            // the registered fetch fails by policy and surfaces an error rather
            // than a bogus "URL was not registered" message.
            let server = MockServer::start().await;
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            let remote_url = format!("{}/intro.md", server.uri());
            let content = format!("---\nprologue: {remote_url}\n---\n# Local\n\nBody.\n");
            std::fs::write(&root, &content).unwrap();

            let md: Markdown = content.clone().into();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .with_allow_remote_transclusion(true)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let result = md.compose_with(options);
            assert!(
                result.is_err(),
                "Expected error because no allowed hosts configured"
            );
        }
    }

    mod file_links_compose {
        use super::*;

        #[test]
        fn glob_mode_replaces_directive_with_linked_tree() {
            let dir = tempfile::tempdir().unwrap();
            // Fake repo so boundary resolves to temp dir, not CWD.
            std::fs::create_dir(dir.path().join(".git")).unwrap();
            let docs = dir.path().join("docs");
            std::fs::create_dir(&docs).unwrap();
            std::fs::write(docs.join("a.md"), "# A\n").unwrap();
            std::fs::write(docs.join("b.txt"), "B\n").unwrap();

            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links docs/*\n\nFooter.\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let (composed, report) = md.compose_with(options).unwrap();

            let text = composed.content();
            assert!(text.contains("a.md"), "content: {text}");
            assert!(text.contains("b.txt"), "content: {text}");
            assert!(!text.contains("::file-links"), "directive should be replaced: {text}");
            assert_eq!(report.transclusions_applied, 1);
        }

        /// End-to-end: composing a `::file-links` directive through the full
        /// default pipeline embeds the FileSystem render subtree, and rendering
        /// the composed document to a terminal reproduces the styled tree —
        /// OSC8 hyperlinks and dim styling survive the round-trip, and the
        /// embedding marker never leaks into rendered output.
        #[test]
        fn embedded_subtree_round_trips_through_compose_then_terminal_render() {
            use crate::markdown::highlighting::{ColorMode, ThemePair};
            use crate::markdown::output::terminal::{
                ColorDepth, DimMode, HyperlinkMode, ItalicMode, MermaidMode, TerminalImageMode,
                TerminalOptions,
            };

            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join(".git")).unwrap();
            let docs = dir.path().join("docs").join("topics");
            std::fs::create_dir_all(&docs).unwrap();
            std::fs::write(docs.join("alpha.md"), "# Alpha\n").unwrap();
            std::fs::write(docs.join("beta.md"), "# Beta\n").unwrap();

            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links docs/topics/*.md\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            // Full default pipeline (cleanup + normalization enabled) proves the
            // embedded block survives end to end.
            let (composed, _report) = md
                .compose_with(ComposeOptions::new().with_source_file(&root))
                .unwrap();
            assert!(
                composed.content().contains("bt:render-tree"),
                "composed doc should carry the embedded subtree: {}",
                composed.content()
            );

            let options = TerminalOptions {
                code_theme: ThemePair::OneHalf,
                prose_theme: ThemePair::OneHalf,
                color_mode: ColorMode::Dark,
                include_line_numbers: false,
                color_depth: Some(ColorDepth::TrueColor),
                image_mode: TerminalImageMode::Never,
                base_path: None,
                italic_mode: ItalicMode::Always,
                dim_mode: DimMode::Always,
                max_width: Some(100),
                mermaid_mode: MermaidMode::Off,
                hyperlink_mode: HyperlinkMode::Always,
                hr_defaults: None,
            };
            let output = composed.as_terminal(options).unwrap();

            assert!(
                !output.contains("bt:render-tree"),
                "embedding marker leaked into rendered output: {output:?}"
            );
            assert!(output.contains("alpha.md"), "missing file: {output:?}");
            assert!(output.contains("beta.md"), "missing file: {output:?}");
            assert!(
                output.contains("\x1b]8;;"),
                "OSC8 hyperlink did not survive: {output:?}"
            );
            assert!(
                output.contains("file://"),
                "file:// link target did not survive: {output:?}"
            );
            assert!(
                output.contains("\x1b[2m"),
                "dim styling (dimmed root prefix) did not survive: {output:?}"
            );
        }

        /// In-process companion to the Level 2 presentation test: the full
        /// `::file-links` contract (extension glyphs, repository icon, italic
        /// dotfile, dimmed gitignored entry, bold target) is produced by
        /// composing then rendering to a terminal string. This runs in the L1
        /// suite, so the bytes the real-terminal test asserts on are verified
        /// without a WezTerm pane.
        #[test]
        fn rich_fixture_renders_full_presentation_contract() {
            use crate::markdown::output::terminal::{
                ColorDepth, DimMode, HyperlinkMode, ItalicMode, MermaidMode, TerminalImageMode,
                TerminalOptions,
            };

            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join(".git")).unwrap();
            let topics = dir.path().join("docs").join("topics");
            std::fs::create_dir_all(&topics).unwrap();
            std::fs::write(topics.join("alpha.md"), "# Alpha\n").unwrap();
            std::fs::write(topics.join("notes.txt"), "notes\n").unwrap();
            std::fs::write(topics.join("report.pdf"), "pdf\n").unwrap();
            std::fs::write(topics.join("sheet.xlsx"), "xls\n").unwrap();
            std::fs::write(topics.join("memo.docx"), "doc\n").unwrap();
            std::fs::write(topics.join(".hidden.md"), "# Hidden\n").unwrap();
            std::fs::write(topics.join(".gitignore"), "ignored.md\n").unwrap();
            std::fs::write(topics.join("ignored.md"), "# Ignored\n").unwrap();

            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links --dir docs/topics\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let (composed, _report) = md
                .compose_with(ComposeOptions::new().with_source_file(&root))
                .unwrap();

            let options = TerminalOptions {
                color_mode: crate::markdown::highlighting::ColorMode::Dark,
                color_depth: Some(ColorDepth::TrueColor),
                image_mode: TerminalImageMode::Never,
                italic_mode: ItalicMode::Always,
                dim_mode: DimMode::Always,
                hyperlink_mode: HyperlinkMode::Always,
                mermaid_mode: MermaidMode::Off,
                max_width: Some(100),
                ..TerminalOptions::default()
            };
            let output = composed.as_terminal(options).unwrap();

            // Extension-specific Unicode glyphs.
            for glyph in ["📝", "📕", "📗", "📘"] {
                assert!(output.contains(glyph), "missing glyph {glyph:?}: {output:?}");
            }
            // Repository icon, never the ordinary folder icon (no subdirs).
            assert!(output.contains("📦"), "missing repository icon: {output:?}");
            assert!(!output.contains("📂"), "unexpected folder icon: {output:?}");
            // Dotfile italic, gitignored dim, target bold.
            assert!(output.contains("\x1b[3m"), "missing italic dotfile: {output:?}");
            assert!(output.contains("\x1b[2m"), "missing dim entry: {output:?}");
            assert!(output.contains("\x1b[1m"), "missing bold target: {output:?}");
            // The gitignored document is present but dim, the dotfile present.
            assert!(output.contains("ignored.md"), "missing ignored.md: {output:?}");
            assert!(output.contains(".hidden.md"), "missing .hidden.md: {output:?}");
            assert!(
                !output.contains(".gitignore"),
                ".gitignore should not be a tree entry: {output:?}"
            );
        }

        #[test]
        fn dir_mode_with_depth_zero_lists_top_level_only() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join(".git")).unwrap();
            let docs = dir.path().join("docs");
            std::fs::create_dir(&docs).unwrap();
            std::fs::write(docs.join("top.md"), "# Top\n").unwrap();
            let sub = docs.join("sub");
            std::fs::create_dir(&sub).unwrap();
            std::fs::write(sub.join("nested.md"), "# Nested\n").unwrap();

            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links --dir docs\n\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let (composed, report) = md.compose_with(options).unwrap();

            let text = composed.content();
            assert!(text.contains("top.md"), "content: {text}");
            assert!(
                !text.contains("nested.md"),
                "depth 0 should not recurse: {text}"
            );
            assert_eq!(report.transclusions_applied, 1);
        }

        #[test]
        fn dir_mode_with_depth_recovers_nested_files() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join(".git")).unwrap();
            let docs = dir.path().join("docs");
            std::fs::create_dir(&docs).unwrap();
            let sub = docs.join("sub");
            std::fs::create_dir(&sub).unwrap();
            std::fs::write(sub.join("nested.md"), "# Nested\n").unwrap();

            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links --dir docs --depth 2\n\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let (composed, report) = md.compose_with(options).unwrap();

            let text = composed.content();
            assert!(text.contains("nested.md"), "content: {text}");
            assert_eq!(report.transclusions_applied, 1);
        }

        #[test]
        fn self_exclusion_skips_source_document() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join(".git")).unwrap();
            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links *.md\n\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let (composed, _report) = md.compose_with(options).unwrap();

            let text = composed.content();
            assert!(
                !text.contains("root.md"),
                "source doc should be excluded: {text}"
            );
        }

        #[test]
        fn strict_empty_result_inserts_notice() {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links *.nonexistent\n\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .with_fail_fast(true)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let (composed, report) = md.compose_with(options).unwrap();

            let text = composed.content();
            assert!(text.contains("No matching files"), "content: {text}");
            assert_eq!(report.transclusions_skipped, 1);
        }

        #[test]
        fn permissive_empty_result_removes_directive_with_warning() {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links *.nonexistent\n\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .with_fail_fast(false)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let (composed, report) = md.compose_with(options).unwrap();

            let text = composed.content();
            assert!(
                !text.contains("::file-links"),
                "directive should be removed: {text}"
            );
            assert!(!text.contains("No matching files"), "content: {text}");
            assert_eq!(report.transclusions_skipped, 1);
            assert!(
                report.warnings.iter().any(|w| w.message.contains("No matching files")),
                "expected warning: {:?}",
                report.warnings
            );
        }

        #[test]
        fn dir_target_regular_file_errors_strict() {
            // `--dir` pointed at a regular file is a syntax error, not an empty
            // directory: strict mode fails the compose with a clear diagnostic.
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join(".git")).unwrap();
            std::fs::write(dir.path().join("report.pdf"), "pdf\n").unwrap();
            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links --dir report.pdf\n\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .with_fail_fast(true)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let err = md.compose_with(options).unwrap_err();
            assert!(
                err.to_string().contains("not a directory"),
                "expected a not-a-directory error, got: {err}"
            );
        }

        #[test]
        fn dir_target_regular_file_warns_permissive() {
            // Permissive mode removes the directive and records the real
            // not-a-directory diagnostic instead of an empty/misleading warning.
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join(".git")).unwrap();
            std::fs::write(dir.path().join("report.pdf"), "pdf\n").unwrap();
            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links --dir report.pdf\n\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .with_ignore_invalid_references(Some(true))
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let (composed, report) = md.compose_with(options).unwrap();

            let text = composed.content();
            assert!(
                !text.contains("::file-links"),
                "directive should be removed: {text}"
            );
            assert_eq!(report.transclusions_skipped, 1);
            assert!(
                report
                    .warnings
                    .iter()
                    .any(|w| w.message.contains("not a directory")),
                "expected not-a-directory warning: {:?}",
                report.warnings
            );
        }

        #[test]
        fn operation_disabling_leaves_directive_intact() {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links *.md\n\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .disable(ComposeOperation::FileLinks)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let (composed, _report) = md.compose_with(options).unwrap();

            let text = composed.content();
            assert!(text.contains("::file-links"), "directive should remain: {text}");
        }

        #[test]
        fn indented_directive_preserves_container_nesting() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join(".git")).unwrap();
            let docs = dir.path().join("docs");
            std::fs::create_dir(&docs).unwrap();
            std::fs::write(docs.join("a.md"), "# A\n").unwrap();

            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n- Item\n  ::file-links docs/*\n\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let (composed, _report) = md.compose_with(options).unwrap();

            let text = composed.content();
            // Each output line should be indented to match the list item
            for line in text.lines().skip(3) {
                if line.contains("a.md") {
                    assert!(
                        line.starts_with("  "),
                        "expected indent, got: {line}"
                    );
                }
            }
        }

        #[test]
        fn malformed_directive_in_strict_mode_fails() {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links\n\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .with_fail_fast(true)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let result = md.compose_with(options);
            assert!(result.is_err(), "expected parse error in strict mode");
        }

        #[test]
        fn out_of_bound_path_is_ignored() {
            let dir = tempfile::tempdir().unwrap();
            let docs = dir.path().join("docs");
            std::fs::create_dir(&docs).unwrap();
            std::fs::write(docs.join("a.md"), "# A\n").unwrap();

            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links ../*\n\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let (composed, report) = md.compose_with(options).unwrap();

            let text = composed.content();
            assert!(
                !text.contains(".."),
                "out-of-bound paths should be excluded: {text}"
            );
            assert_eq!(report.transclusions_skipped, 1);
        }

        #[test]
        fn mixed_case_extensions_are_included() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join(".git")).unwrap();
            let docs = dir.path().join("docs");
            std::fs::create_dir(&docs).unwrap();
            std::fs::write(docs.join("lower.md"), "# L\n").unwrap();
            std::fs::write(docs.join("UPPER.MD"), "# U\n").unwrap();
            std::fs::write(docs.join("MiXeD.Txt"), "# M\n").unwrap();
            std::fs::write(docs.join("binary.exe"), "binary\n").unwrap();

            let root = dir.path().join("root.md");
            std::fs::write(&root, "# Root\n\n::file-links docs/*\n\n").unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let (composed, _report) = md.compose_with(options).unwrap();

            let text = composed.content();
            assert!(text.contains("lower.md"), "content: {text}");
            assert!(text.contains("UPPER.MD"), "content: {text}");
            assert!(text.contains("MiXeD.Txt"), "content: {text}");
            assert!(
                !text.contains("binary.exe"),
                "unsupported extension should be excluded: {text}"
            );
        }

        #[test]
        fn multiple_directives_produce_deterministic_ordering() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join(".git")).unwrap();
            let docs = dir.path().join("docs");
            std::fs::create_dir(&docs).unwrap();
            std::fs::write(docs.join("a.md"), "# A\n").unwrap();
            let other = dir.path().join("other");
            std::fs::create_dir(&other).unwrap();
            std::fs::write(other.join("b.txt"), "B\n").unwrap();

            let root = dir.path().join("root.md");
            std::fs::write(
                &root,
                "# Root\n\n::file-links docs/*\n\n::file-links other/*\n\n",
            )
            .unwrap();

            let md = Markdown::try_from(root.as_path()).unwrap();
            let options = ComposeOptions::new()
                .with_source_file(&root)
                .disable(ComposeOperation::Cleanup)
                .disable(ComposeOperation::Normalization);
            let (composed, report) = md.compose_with(options).unwrap();

            let text = composed.content();
            // First directive's content should appear before second
            let pos_a = text.find("a.md").expect("a.md present");
            let pos_b = text.find("b.txt").expect("b.txt present");
            assert!(pos_a < pos_b, "deterministic order violated: {text}");
            assert_eq!(report.transclusions_applied, 2);
        }
    }
}
