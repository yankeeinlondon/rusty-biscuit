//! Type definitions for the compose pipeline.
//!
//! This module contains the core types used by the compose pipeline:
//! - `ComposeOperation` - Enumeration of every discrete pipeline operation
//! - `ComposePhase` - Execution phase grouping for operations
//! - `ComposeOptions` - Configuration for compose execution
//! - `ComposeContext` - Runtime context captured at compose start
//! - `ComposeReport` - Results and diagnostics from compose execution

use super::super::normalize::NormalizationReport;
use super::cache::{CacheAccessMode, CacheFreshnessMode, CacheStats};
use super::shell_expansion::types::ShellTimeoutBehavior;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

/// Every discrete operation in the compose pipeline.
///
/// Operations are grouped into three phases for execution:
/// - **Inline Pre**: serial, runs before transclusion
/// - **Transclusion**: concurrent, recursive document inclusion
/// - **Inline Post**: serial, runs after transclusion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComposeOperation {
    /// Resolves `{{ variable }}` expressions inside frontmatter values
    /// using non-templated frontmatter values, `ctx`, and `env` as inputs.
    /// Runs before the final effective state is built.
    FrontmatterInterpolation,

    /// Executes shell commands embedded in frontmatter values.
    /// Runs before the final effective state is built.
    FrontmatterShellExpansion,

    /// Applies the frontmatter `replace` map to substitute text
    /// patterns throughout the document body.
    TextReplacement,

    /// Evaluates `::block when="..."` / `::end-block` conditional
    /// regions and removes blocks whose conditions are false.
    PageBlocks,

    /// Expands `{{variable}}` handlebars expressions using frontmatter,
    /// environment variables, and context variables.
    Interpolation,

    /// Executes approved `::shell` directives and replaces them
    /// with the command's stdout output.
    ShellExpansion,

    /// Executes approved `::shell-block` directives and replaces them
    /// with the combined output of the block's commands.
    ShellBlocks,

    /// Resolves `::file` and `::url` directives by including the
    /// referenced markdown document (recursively composed).
    BlockTransclusion,

    /// Resolves `prologue` and `epilogue` frontmatter references
    /// by prepending/appending the referenced documents.
    FrontmatterTransclusion,

    /// Resolves `::code` directives by including file content
    /// as a fenced code block.
    CodeTransclusion,

    /// Expands `::toc-linking` directives by generating a linked
    /// table of contents from an external document's headings.
    TocLinking,

    /// Normalizes markdown formatting: injects blank lines between
    /// block elements and aligns table columns.
    Cleanup,

    /// Adjusts heading levels to ensure a valid hierarchy
    /// (e.g., no H3 before an H2).
    Normalization,

    /// Resolves all local link targets (Markdown hyperlinks/images and
    /// supported HTML embeds) to absolute paths during the Inline-Pre stage.
    LinkResolve,

    /// Converts absolute path links back into portable forms during the
    /// Finalization stage (relative paths within the same repo, `~/` for
    /// home-relative paths, `${VAR}` for whitelisted environment-relative
    /// paths).
    LinkNormalization,
}

/// Fixed-size operation set keyed by [`ComposeOperation`] discriminants.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ComposeOperationSet {
    enabled: [bool; ComposeOperation::COUNT],
}

impl ComposeOperationSet {
    /// Creates an empty operation set.
    pub fn empty() -> Self {
        Self {
            enabled: [false; ComposeOperation::COUNT],
        }
    }

    /// Creates a set containing every operation.
    pub fn all() -> Self {
        ComposeOperation::default_order().iter().copied().collect()
    }

    /// Enables an operation.
    pub fn insert(&mut self, op: ComposeOperation) {
        self.enabled[op.index()] = true;
    }

    /// Disables an operation.
    pub fn remove(&mut self, op: ComposeOperation) {
        self.enabled[op.index()] = false;
    }

    /// Returns `true` when the operation is enabled.
    pub fn contains(&self, op: ComposeOperation) -> bool {
        self.enabled[op.index()]
    }

    /// Iterates enabled operations in canonical enum order.
    pub fn iter(&self) -> impl Iterator<Item = ComposeOperation> + '_ {
        ComposeOperation::default_order()
            .iter()
            .copied()
            .filter(|op| self.contains(*op))
    }
}

impl std::fmt::Debug for ComposeOperationSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl Default for ComposeOperationSet {
    fn default() -> Self {
        Self::empty()
    }
}

impl FromIterator<ComposeOperation> for ComposeOperationSet {
    fn from_iter<T: IntoIterator<Item = ComposeOperation>>(iter: T) -> Self {
        let mut set = Self::empty();
        for op in iter {
            set.insert(op);
        }
        set
    }
}

/// The execution phase an operation belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposePhase {
    /// Serial operations before transclusion.
    InlinePre,
    /// Concurrent recursive document inclusion.
    Transclusion,
    /// Serial operations after transclusion.
    InlinePost,
    /// Root-only finalization stage. Runs after Inline-Post completes on the
    /// outermost document and is skipped on transcluded children.
    Finalization,
}

impl ComposeOperation {
    /// Total number of compose operations.
    pub const COUNT: usize = 15;

    /// Stable discriminant index for fixed-size operation sets.
    pub const fn index(self) -> usize {
        match self {
            Self::FrontmatterInterpolation => 0,
            Self::FrontmatterShellExpansion => 1,
            Self::TextReplacement => 2,
            Self::PageBlocks => 3,
            Self::Interpolation => 4,
            Self::ShellExpansion => 5,
            Self::ShellBlocks => 6,
            Self::BlockTransclusion => 7,
            Self::FrontmatterTransclusion => 8,
            Self::CodeTransclusion => 9,
            Self::TocLinking => 10,
            Self::Cleanup => 11,
            Self::Normalization => 12,
            Self::LinkResolve => 13,
            Self::LinkNormalization => 14,
        }
    }

    /// Returns the default phase this operation belongs to.
    pub fn phase(&self) -> ComposePhase {
        match self {
            Self::FrontmatterInterpolation
            | Self::FrontmatterShellExpansion
            | Self::TextReplacement
            | Self::PageBlocks
            | Self::Interpolation
            | Self::ShellExpansion
            | Self::ShellBlocks
            | Self::LinkResolve => ComposePhase::InlinePre,

            Self::BlockTransclusion
            | Self::FrontmatterTransclusion
            | Self::CodeTransclusion
            | Self::TocLinking => ComposePhase::Transclusion,

            Self::Cleanup | Self::Normalization => ComposePhase::InlinePost,

            Self::LinkNormalization => ComposePhase::Finalization,
        }
    }

    /// Returns all operations in their default execution order.
    pub fn default_order() -> &'static [ComposeOperation] {
        &[
            // Inline Pre (serial)
            Self::FrontmatterInterpolation,
            Self::FrontmatterShellExpansion,
            Self::TextReplacement,
            Self::PageBlocks,
            Self::Interpolation,
            Self::ShellExpansion,
            Self::ShellBlocks,
            Self::LinkResolve,
            // Transclusion (concurrent)
            Self::BlockTransclusion,
            Self::FrontmatterTransclusion,
            Self::CodeTransclusion,
            Self::TocLinking,
            // Inline Post (serial)
            Self::Cleanup,
            Self::Normalization,
            // Finalization (root-only)
            Self::LinkNormalization,
        ]
    }

    /// Returns the set of all operations.
    pub fn all() -> ComposeOperationSet {
        ComposeOperationSet::all()
    }
}

impl std::fmt::Display for ComposeOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FrontmatterInterpolation => write!(f, "FrontmatterInterpolation"),
            Self::FrontmatterShellExpansion => write!(f, "Frontmatter Shell Expansion"),
            Self::TextReplacement => write!(f, "TextReplacement"),
            Self::PageBlocks => write!(f, "PageBlocks"),
            Self::Interpolation => write!(f, "Interpolation"),
            Self::ShellExpansion => write!(f, "ShellExpansion"),
            Self::ShellBlocks => write!(f, "ShellBlocks"),
            Self::BlockTransclusion => write!(f, "BlockTransclusion"),
            Self::FrontmatterTransclusion => write!(f, "FrontmatterTransclusion"),
            Self::CodeTransclusion => write!(f, "CodeTransclusion"),
            Self::TocLinking => write!(f, "TocLinking"),
            Self::Cleanup => write!(f, "Cleanup"),
            Self::Normalization => write!(f, "Normalization"),
            Self::LinkResolve => write!(f, "LinkResolve"),
            Self::LinkNormalization => write!(f, "LinkNormalization"),
        }
    }
}

impl std::fmt::Display for ComposePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InlinePre => write!(f, "InlinePre"),
            Self::Transclusion => write!(f, "Transclusion"),
            Self::InlinePost => write!(f, "InlinePost"),
            Self::Finalization => write!(f, "finalization"),
        }
    }
}

/// Configuration for the compose pipeline.
///
/// Controls which operations run, how transclusion resolves references,
/// and provides external state for interpolation and replacement.
///
/// ## Construction
///
/// Always use `ComposeOptions::new()` to construct, which captures
/// runtime context (current time, environment variables) at creation.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::{ComposeOptions, ComposeOperation};
///
/// // Default: all operations enabled
/// let options = ComposeOptions::new();
///
/// // Disable cleanup and normalization
/// let options = ComposeOptions::new()
///     .disable(ComposeOperation::Cleanup)
///     .disable(ComposeOperation::Normalization);
///
/// // Only run specific operations
/// let options = ComposeOptions::new()
///     .only(&[ComposeOperation::TextReplacement, ComposeOperation::Interpolation]);
/// ```
#[derive(Clone)]
pub struct ComposeOptions {
    // ── Operation control ──────────────────────────────────────────
    /// Set of operations to execute.
    ///
    /// Defaults to all operations. Use `disable()` or `only()` to
    /// restrict which operations run.
    pub(crate) enabled_operations: ComposeOperationSet,

    // ── Error handling ─────────────────────────────────────────────
    /// When `true`, the pipeline returns an error on the first failure.
    /// When `false` (default), failures are recorded as warnings and
    /// the pipeline continues with remaining operations.
    pub(crate) fail_fast: bool,

    // ── Context override ──────────────────────────────────────────
    /// When `true`, non-object `ctx` frontmatter is downgraded from an error
    /// to a warning and the runtime context is used instead.
    pub(crate) allow_ctx_override: bool,

    // ── Transclusion set-override permissive flags ─────────────────
    /// When `true`, a `::file` directive whose `set=<value>` RHS fails
    /// to parse as a JSON5 object is downgraded from `InvalidFrontmatterAssignment`
    /// to a `ComposeWarning`; sibling valid set clauses still apply.
    pub(crate) allow_invalid_frontmatter_assignment: bool,

    /// When `true`, a `::file` directive that reassigns the same set
    /// property (or duplicates the object form) is downgraded from
    /// `InvalidReassignedFrontmatterProperty` to a `ComposeWarning`; the
    /// rightmost assignment wins.
    pub(crate) allow_reassigned_frontmatter_property: bool,

    // ── Source context ─────────────────────────────────────────────
    /// Source location of the document being composed.
    ///
    /// Required for transclusion to resolve relative `::file` paths.
    /// Set via `with_source_file()` or `with_source_url()`.
    pub(crate) source: ComposeSource,

    // ── State and data ─────────────────────────────────────────────
    /// External state merged with frontmatter for interpolation and
    /// replacement. Missing or null frontmatter keys are filled from
    /// this value using deep-merge semantics.
    pub(crate) external_state: Option<serde_json::Value>,

    /// Override values that unconditionally overwrite frontmatter keys.
    ///
    /// Unlike `external_state` which only fills missing/null keys,
    /// these values always win regardless of what the frontmatter says.
    pub(crate) set_overrides: Option<serde_json::Value>,

    // ── Transclusion ───────────────────────────────────────────────
    /// Maximum recursive transclusion depth before the pipeline
    /// returns an error. Prevents infinite `::file` chains.
    /// Default: 16.
    pub(crate) max_transclusion_depth: usize,

    /// Whether `::url` remote transclusion is allowed.
    ///
    /// Disabled by default for security. When false, `::url` directives
    /// are skipped (or error if `fail_fast` is true).
    pub(crate) allow_remote_transclusion: bool,

    /// Whether `::file` can include local markdown documents.
    /// Default: true.
    pub(crate) allow_local_markdown: bool,

    /// Whether `::code` can include local text files as code blocks.
    /// Default: true.
    pub(crate) allow_local_code: bool,

    /// Language tag applied to `::code` blocks when the file extension
    /// is unknown or unmapped. Default: `"txt"`.
    pub(crate) code_fallback_language: String,

    /// Overrides the default behavior for invalid transclusion references.
    ///
    /// - `None`: use the document's frontmatter `ignore_invalid` setting
    /// - `Some(true)`: silently skip invalid references
    /// - `Some(false)`: treat invalid references as errors
    pub(crate) ignore_invalid_references: Option<bool>,

    /// Whether `@`-prefixed paths resolve to the git repository root.
    /// Default: true.
    pub(crate) resolve_repo_root: bool,

    /// Custom search roots for `@`-prefixed (magic) file references.
    ///
    /// Each entry is a `(path, position)` pair where `position` controls
    /// whether the path is searched before (`Start`) or after (`End`) the
    /// default roots (git repo root, HOME).
    pub(crate) magic_paths: Vec<(PathBuf, biscuit_file::PathPosition)>,

    // ── Shell expansion ────────────────────────────────────────────
    /// Maximum execution time for a single `::shell` command.
    /// Default: 10 seconds.
    pub(crate) shell_timeout: std::time::Duration,

    /// What happens when a shell command exceeds its timeout.
    /// Default: `Error` (abort compose).
    pub(crate) shell_timeout_behavior: ShellTimeoutBehavior,

    /// Root directory for shell expansion policy files.
    ///
    /// When set, only commands matching an approval policy in this
    /// directory (or its ancestors) are allowed to execute.
    pub(crate) shell_policy_root: Option<PathBuf>,

    /// Working directory for `::shell` command execution.
    ///
    /// When `None`, commands run in the directory of the source file
    /// (if known) or the current working directory.
    pub(crate) shell_working_directory: Option<PathBuf>,

    /// Callback for interactive shell command approval.
    ///
    /// When set, commands that require approval call this handler
    /// before execution. When `None`, unapproved commands are skipped.
    pub(crate) shell_approval_handler:
        Option<std::sync::Arc<dyn super::shell_expansion::ShellApprovalHandler>>,

    /// Pre-approved shell commands (normalized forms).
    ///
    /// When set, the shell expansion stage skips the entire approval flow
    /// (no whitelist check, no blacklist check, no approval handler).
    /// Each directive's normalized command is checked against this set:
    /// - Found: execute immediately (still subject to timeout)
    /// - Not found: immediate Denied error
    ///
    /// Mutually exclusive with `shell_approval_handler`. When this field
    /// is `Some`, the approval handler is ignored.
    pub(crate) pre_approved_commands: Option<std::collections::HashSet<String>>,

    /// Whether to strip ANSI escape codes and set `NO_COLOR=1` for shell commands.
    /// Default: true.
    pub shell_strip_ansi: bool,

    // ── Cleanup ────────────────────────────────────────────────────
    /// Controls how blank lines between list items are handled
    /// during the cleanup operation. Default: `Normal`.
    pub(crate) list_spacing: crate::markdown::cleanup::ListSpacingMode,

    /// Number of spaces per nesting level for list indentation
    /// during cleanup. Default: 4.
    pub(crate) indent_size: usize,

    // ── Caching ───────────────────────────────────────────────────
    /// Controls whether and how caching is used during compose.
    /// Default: `ReadWrite` (full caching with single-flight dedup).
    pub(crate) cache_access_mode: CacheAccessMode,

    /// Controls staleness tolerance for persistent cache entries.
    /// Default: `Strict` (only accept entries whose closure hash matches).
    pub(crate) cache_freshness_mode: CacheFreshnessMode,

    /// Root directory for persistent cache storage.
    /// When `None`, persistent caching is disabled. Set to a path
    /// (typically `<workspace>/.darkmatter/cache/v1/`) to enable.
    pub(crate) cache_root: Option<PathBuf>,

    /// Namespace for cache isolation (e.g., branch name, profile).
    /// When set, cache entries are stored under this namespace to
    /// prevent cross-contamination between different contexts.
    pub(crate) cache_namespace: Option<String>,

    // ── Performance ─────────────────────────────────────────────────
    /// When `true`, the pipeline collects per-stage timing metrics
    /// and populates `ComposeReport::perf`. Default: `false`.
    pub(crate) perf_enabled: bool,

    // ── Internal (crate-private) ───────────────────────────────────
    /// Runtime context captured at construction time (timestamps,
    /// environment variables).
    context: ComposeContext,

    /// When true, external `replace` keys override document `replace`
    /// keys (used during recursive transclusion to inherit parent
    /// replacements).
    pub(crate) replace_parent_wins: bool,

    /// One-off replace map applied only to this document's replacement
    /// stage, never propagated to children.
    pub(crate) one_off_replace: Option<serde_json::Map<String, serde_json::Value>>,

    // ── Interpolation ─────────────────────────────────────────────
    /// When `true`, body interpolation processes `{{ }}` expressions
    /// inside fenced and indented code blocks.
    ///
    /// Inline code spans (single-backticks) are always interpolated —
    /// this flag only governs fenced/indented code blocks, which by
    /// default (`false`) are skipped to preserve literal code examples.
    ///
    /// Can also be set via frontmatter: `interpolate_code_blocks: true`.
    pub(crate) interpolate_code_blocks: bool,

    // ── Schema ────────────────────────────────────────────────────
    /// Optional baseline schema merged with any document `$schema`
    /// before validation runs. When both baseline and document declare
    /// the same property, the document wins.
    pub(crate) baseline_schema: Option<crate::markdown::schemas::SimplifiedSchema>,

    // ── Link normalization ────────────────────────────────────────
    /// Environment variables that may be used as path-prefix abstractions
    /// during the Finalization stage's Link Normalization operation.
    ///
    /// Acts as a strict allowlist: only variables present in this list (or
    /// the built-in default set when this list is empty) are considered
    /// when collapsing absolute paths to portable `${VAR}/...` form.
    ///
    /// Defaults to an empty vector; the Link Normalization operation
    /// applies a built-in default whitelist (`PROJECT_ROOT`, `DOCS_BASE`)
    /// when this field is empty.
    pub(crate) env_path_whitelist: Vec<String>,
}

impl std::fmt::Debug for ComposeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComposeOptions")
            .field("enabled_operations", &self.enabled_operations)
            .field("fail_fast", &self.fail_fast)
            .field("source", &self.source)
            .field("external_state", &self.external_state)
            .field("set_overrides", &self.set_overrides)
            .field("max_transclusion_depth", &self.max_transclusion_depth)
            .field("allow_remote_transclusion", &self.allow_remote_transclusion)
            .field("allow_local_markdown", &self.allow_local_markdown)
            .field("allow_local_code", &self.allow_local_code)
            .field("code_fallback_language", &self.code_fallback_language)
            .field("ignore_invalid_references", &self.ignore_invalid_references)
            .field("resolve_repo_root", &self.resolve_repo_root)
            .field("magic_paths", &self.magic_paths)
            .field("shell_timeout", &self.shell_timeout)
            .field("shell_policy_root", &self.shell_policy_root)
            .field("shell_working_directory", &self.shell_working_directory)
            .field(
                "shell_approval_handler",
                if self.shell_approval_handler.is_some() {
                    &"Some(..)"
                } else {
                    &"None"
                },
            )
            .field(
                "pre_approved_commands",
                &self
                    .pre_approved_commands
                    .as_ref()
                    .map(|s| format!("{} commands", s.len())),
            )
            .field("list_spacing", &self.list_spacing)
            .field("indent_size", &self.indent_size)
            .field("cache_access_mode", &self.cache_access_mode)
            .field("cache_freshness_mode", &self.cache_freshness_mode)
            .field("cache_root", &self.cache_root)
            .field("cache_namespace", &self.cache_namespace)
            .field("perf_enabled", &self.perf_enabled)
            .field("replace_parent_wins", &self.replace_parent_wins)
            .field("one_off_replace", &self.one_off_replace)
            .field("interpolate_code_blocks", &self.interpolate_code_blocks)
            .field(
                "baseline_schema",
                if self.baseline_schema.is_some() {
                    &"Some(..)"
                } else {
                    &"None"
                },
            )
            .field("env_path_whitelist", &self.env_path_whitelist)
            .field(
                "allow_invalid_frontmatter_assignment",
                &self.allow_invalid_frontmatter_assignment,
            )
            .field(
                "allow_reassigned_frontmatter_property",
                &self.allow_reassigned_frontmatter_property,
            )
            .field("context", &self.context)
            .finish()
    }
}

impl ComposeOptions {
    /// Creates new compose options with all operations enabled and captured context.
    ///
    /// Captures runtime context (timestamps, environment variables) at
    /// construction time. If you already have a `ComposeContext`, use
    /// [`new_with_context`](Self::new_with_context) to avoid redundant capture.
    pub fn new() -> Self {
        Self::new_with_context(ComposeContext::capture())
    }

    /// Creates new compose options using a pre-captured context.
    ///
    /// Use this when you have already captured a `ComposeContext` (e.g.,
    /// via `ComposeContext::capture_for_content`) and want to avoid
    /// the cost of a redundant capture.
    pub fn new_with_context(context: ComposeContext) -> Self {
        Self {
            enabled_operations: ComposeOperation::all(),
            fail_fast: false,
            allow_ctx_override: false,
            allow_invalid_frontmatter_assignment: false,
            allow_reassigned_frontmatter_property: false,
            source: ComposeSource::Unknown,
            external_state: None,
            set_overrides: None,
            max_transclusion_depth: 16,
            allow_remote_transclusion: false,
            allow_local_markdown: true,
            allow_local_code: true,
            code_fallback_language: "txt".to_string(),
            ignore_invalid_references: None,
            resolve_repo_root: true,
            magic_paths: Vec::new(),
            shell_timeout: std::time::Duration::from_secs(10),
            shell_timeout_behavior: ShellTimeoutBehavior::Error,
            shell_policy_root: None,
            shell_working_directory: None,
            shell_approval_handler: None,
            pre_approved_commands: None,
            list_spacing: crate::markdown::cleanup::ListSpacingMode::Normal,
            indent_size: crate::markdown::cleanup::DEFAULT_INDENT,
            cache_access_mode: CacheAccessMode::default(),
            cache_freshness_mode: CacheFreshnessMode::default(),
            cache_root: None,
            cache_namespace: None,
            perf_enabled: false,
            context,
            replace_parent_wins: false,
            one_off_replace: None,
            interpolate_code_blocks: false,
            shell_strip_ansi: true,
            env_path_whitelist: Vec::new(),
            baseline_schema: None,
        }
    }

    /// Returns a reference to the captured runtime context.
    pub fn context(&self) -> &ComposeContext {
        &self.context
    }

    /// Disables a single operation.
    #[must_use]
    pub fn disable(mut self, op: ComposeOperation) -> Self {
        self.enabled_operations.remove(op);
        self
    }

    /// Enables only the specified operations, disabling everything else.
    #[must_use]
    pub fn only(mut self, ops: &[ComposeOperation]) -> Self {
        self.enabled_operations = ops.iter().copied().collect();
        self
    }

    /// Returns true if the given operation is enabled.
    pub fn is_enabled(&self, op: ComposeOperation) -> bool {
        self.enabled_operations.contains(op)
    }

    /// Sets the compose source as a file path.
    #[must_use]
    pub fn with_source_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.source = ComposeSource::File(path.into());
        self
    }

    /// Sets the compose source as a URL.
    #[must_use]
    pub fn with_source_url(mut self, url: Url) -> Self {
        self.source = ComposeSource::Url(url);
        self
    }

    /// Sets the external state for interpolation/replacement.
    #[must_use]
    pub fn with_external_state(mut self, state: serde_json::Value) -> Self {
        self.external_state = Some(state);
        self
    }

    /// Sets override values that overwrite existing frontmatter keys.
    #[must_use]
    pub fn with_set_overrides(mut self, overrides: serde_json::Value) -> Self {
        self.set_overrides = Some(overrides);
        self
    }

    /// Sets the list spacing mode for the cleanup stage.
    #[must_use]
    pub fn with_list_spacing(mut self, mode: crate::markdown::cleanup::ListSpacingMode) -> Self {
        self.list_spacing = mode;
        self
    }

    /// Sets the indentation width for nested list cleanup.
    #[must_use]
    pub fn with_indent_size(mut self, size: usize) -> Self {
        self.indent_size = size.max(1);
        self
    }

    /// Sets the cache access mode.
    #[must_use]
    pub fn with_cache_access_mode(mut self, mode: CacheAccessMode) -> Self {
        self.cache_access_mode = mode;
        self
    }

    /// Sets the cache freshness mode for persistent cache.
    #[must_use]
    pub fn with_cache_freshness_mode(mut self, mode: CacheFreshnessMode) -> Self {
        self.cache_freshness_mode = mode;
        self
    }

    /// Sets the persistent cache root directory, enabling persistent caching.
    #[must_use]
    pub fn with_cache_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.cache_root = Some(root.into());
        self
    }

    /// Sets the cache namespace for isolation.
    #[must_use]
    pub fn with_cache_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.cache_namespace = Some(namespace.into());
        self
    }

    /// Sets fail-fast mode.
    #[must_use]
    pub fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }

    /// Allow non-object ctx frontmatter (downgrade error to warning).
    #[must_use]
    pub fn with_allow_ctx_override(mut self, allow: bool) -> Self {
        self.allow_ctx_override = allow;
        self
    }

    /// Downgrades `InvalidFrontmatterAssignment` to a compose warning.
    ///
    /// When `true`, a `::file` directive whose `set=<value>` RHS is not
    /// a JSON5 object is dropped with a warning instead of failing the
    /// pipeline. Sibling `set.NAME=<value>` clauses on the same directive
    /// line are unaffected and still apply.
    #[must_use]
    pub fn with_allow_invalid_frontmatter_assignment(mut self, allow: bool) -> Self {
        self.allow_invalid_frontmatter_assignment = allow;
        self
    }

    /// Downgrades `InvalidReassignedFrontmatterProperty` to a compose warning.
    ///
    /// When `true`, a duplicate `set.NAME=` assignment (or duplicate
    /// `set=<object>`) on the same `::file` directive emits a warning
    /// instead of failing the pipeline. The rightmost assignment wins.
    #[must_use]
    pub fn with_allow_reassigned_frontmatter_property(mut self, allow: bool) -> Self {
        self.allow_reassigned_frontmatter_property = allow;
        self
    }

    /// Enables interpolation inside fenced and indented code blocks.
    ///
    /// Inline code spans (single-backticks) are always interpolated —
    /// this option only opts fenced/indented code blocks into the
    /// interpolation scan, which is otherwise skipped to preserve
    /// literal code examples.
    #[must_use]
    pub fn with_interpolate_code_blocks(mut self, enabled: bool) -> Self {
        self.interpolate_code_blocks = enabled;
        self
    }

    /// Sets the strict allowlist of environment variables that the
    /// Finalization stage may use as path-prefix abstractions.
    ///
    /// Each entry is the bare variable name (e.g. `"PROJECT_ROOT"`); the
    /// Link Normalization operation reads the corresponding value from the
    /// process environment at evaluation time. Passing an empty vector
    /// restores the built-in default whitelist.
    #[must_use]
    pub fn with_env_path_whitelist(mut self, paths: Vec<String>) -> Self {
        self.env_path_whitelist = paths;
        self
    }

    /// Returns the effective environment-variable allowlist used by Link
    /// Normalization.
    ///
    /// When the user-supplied list (`with_env_path_whitelist`) is empty,
    /// returns the built-in default fallback set (`PROJECT_ROOT`,
    /// `DOCS_BASE`); otherwise returns the user-supplied list.
    pub fn effective_env_path_whitelist(&self) -> Vec<String> {
        if self.env_path_whitelist.is_empty() {
            Self::default_env_path_whitelist()
                .iter()
                .map(|s| (*s).to_string())
                .collect()
        } else {
            self.env_path_whitelist.clone()
        }
    }

    /// Returns the built-in default environment-variable allowlist used
    /// when the caller has not supplied an explicit whitelist.
    pub const fn default_env_path_whitelist() -> &'static [&'static str] {
        &["PROJECT_ROOT", "DOCS_BASE"]
    }

    /// Sets shell expansion options from a `ShellExpansionOptions` struct.
    #[must_use]
    pub fn with_shell(mut self, shell: super::shell_expansion::ShellExpansionOptions) -> Self {
        self.shell_timeout = shell.timeout;
        self.shell_timeout_behavior = shell.timeout_behavior;
        self.shell_policy_root = shell.policy_root;
        self.shell_working_directory = shell.working_directory;
        self.shell_approval_handler = shell.approval_handler;
        self.shell_strip_ansi = shell.strip_ansi;
        self
    }

    /// Sets the shell command timeout directly on flat compose options.
    #[must_use]
    pub fn with_shell_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.shell_timeout = timeout;
        self
    }

    /// Sets the shell timeout behavior directly on flat compose options.
    #[must_use]
    pub fn with_shell_timeout_behavior(mut self, behavior: ShellTimeoutBehavior) -> Self {
        self.shell_timeout_behavior = behavior;
        self
    }

    /// Sets whether to allow shell commands to timeout without aborting compose.
    ///
    /// When `true`, sets `shell_timeout_behavior` to `EmptyString`.
    /// When `false`, sets it to `Error` (default).
    #[must_use]
    pub fn with_allow_shell_timeout(mut self, allow: bool) -> Self {
        self.shell_timeout_behavior = if allow {
            ShellTimeoutBehavior::EmptyString
        } else {
            ShellTimeoutBehavior::Error
        };
        self
    }

    /// Sets the shell policy root directly on flat compose options.
    #[must_use]
    pub fn with_shell_policy_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.shell_policy_root = Some(path.into());
        self
    }

    /// Sets the shell working directory directly on flat compose options.
    #[must_use]
    pub fn with_shell_working_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.shell_working_directory = Some(path.into());
        self
    }

    /// Sets the shell approval handler directly on flat compose options.
    #[must_use]
    pub fn with_shell_approval_handler(
        mut self,
        handler: std::sync::Arc<dyn super::shell_expansion::ShellApprovalHandler>,
    ) -> Self {
        self.shell_approval_handler = Some(handler);
        self
    }

    /// Sets the pre-approved shell commands.
    #[must_use]
    pub fn with_pre_approved_commands(
        mut self,
        commands: std::collections::HashSet<String>,
    ) -> Self {
        self.pre_approved_commands = Some(commands);
        self
    }

    /// Sets whether remote transclusion is allowed.
    #[must_use]
    pub fn with_allow_remote_transclusion(mut self, allow: bool) -> Self {
        self.allow_remote_transclusion = allow;
        self
    }

    /// Sets whether local markdown transclusion is allowed.
    #[must_use]
    pub fn with_allow_local_markdown(mut self, allow: bool) -> Self {
        self.allow_local_markdown = allow;
        self
    }

    /// Sets whether local code transclusion is allowed.
    #[must_use]
    pub fn with_allow_local_code(mut self, allow: bool) -> Self {
        self.allow_local_code = allow;
        self
    }

    /// Sets the maximum recursive transclusion depth.
    #[must_use]
    pub fn with_max_transclusion_depth(mut self, max_depth: usize) -> Self {
        self.max_transclusion_depth = max_depth.max(1);
        self
    }

    /// Sets how invalid transclusion references are handled.
    #[must_use]
    pub fn with_ignore_invalid_references(mut self, ignore: Option<bool>) -> Self {
        self.ignore_invalid_references = ignore;
        self
    }

    /// Sets whether `@` paths resolve relative to the repository root.
    #[must_use]
    pub fn with_resolve_repo_root(mut self, enabled: bool) -> Self {
        self.resolve_repo_root = enabled;
        self
    }

    /// Adds a custom search root for `@`-prefixed file references.
    ///
    /// Paths added with `PathPosition::Start` are searched before the
    /// git repository root; paths with `PathPosition::End` are searched
    /// after HOME.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::compose::{ComposeOptions, PathPosition};
    ///
    /// let options = ComposeOptions::new()
    ///     .with_magic_path("/project/.claudine", PathPosition::Start)
    ///     .with_magic_path("/home/user/.claudine", PathPosition::Start);
    /// ```
    #[must_use]
    pub fn with_magic_path(
        mut self,
        path: impl Into<PathBuf>,
        position: biscuit_file::PathPosition,
    ) -> Self {
        self.magic_paths.push((path.into(), position));
        self
    }

    /// Sets the fallback language for code transclusion.
    #[must_use]
    pub fn with_code_fallback_language(mut self, language: impl Into<String>) -> Self {
        self.code_fallback_language = language.into();
        self
    }

    /// Returns a `TransclusionOptions` view of the transclusion-related fields.
    ///
    /// Used internally to pass transclusion config to resolver and TOC linking
    /// functions without coupling them to the full `ComposeOptions` type.
    pub(crate) fn transclusion_options(&self) -> TransclusionOptions {
        TransclusionOptions {
            source: self.source.clone(),
            max_depth: self.max_transclusion_depth,
            allow_remote: self.allow_remote_transclusion,
            allow_local_markdown: self.allow_local_markdown,
            allow_local_code_text: self.allow_local_code,
            code_fallback_language: self.code_fallback_language.clone(),
            ignore_invalid: self.ignore_invalid_references,
            resolve_repo_root: self.resolve_repo_root,
            magic_paths: self.magic_paths.clone(),
        }
    }

    /// Returns a `ShellExpansionOptions` view of the shell-related fields.
    ///
    /// Used internally to pass shell config to executor and policy functions
    /// without coupling them to the full `ComposeOptions` type.
    pub(crate) fn shell_options(&self) -> super::shell_expansion::ShellExpansionOptions {
        super::shell_expansion::ShellExpansionOptions {
            timeout: self.shell_timeout,
            timeout_behavior: self.shell_timeout_behavior,
            policy_root: self.shell_policy_root.clone(),
            working_directory: self.shell_working_directory.clone(),
            approval_handler: self.shell_approval_handler.clone(),
            strip_ansi: self.shell_strip_ansi,
        }
    }

    /// Sets whether to strip ANSI escape codes for shell commands.
    #[must_use]
    pub fn with_shell_strip_ansi(mut self, enabled: bool) -> Self {
        self.shell_strip_ansi = enabled;
        self
    }

    /// Attach a baseline `SimplifiedSchema` that is merged with any
    /// `$schema` declared in the document before validation runs.
    ///
    /// Callers (e.g. claudine) can register a workspace-wide schema
    /// without editing every prompt file. When both baseline and
    /// document `$schema` declare the same property, the document
    /// side wins — matching the existing `schemas::resolve::merge`
    /// rule.
    #[must_use]
    pub fn with_baseline_schema(
        mut self,
        schema: crate::markdown::schemas::SimplifiedSchema,
    ) -> Self {
        self.baseline_schema = Some(schema);
        self
    }

    /// Internal builder: toggles parent-wins behavior for the `replace` map.
    #[must_use]
    pub(crate) fn with_replace_parent_wins(mut self, enabled: bool) -> Self {
        self.replace_parent_wins = enabled;
        self
    }

    /// Internal builder: sets a one-off replace map for this document only.
    #[must_use]
    pub(crate) fn with_one_off_replace(
        mut self,
        replace: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        self.one_off_replace = replace;
        self
    }

    /// Replaces the captured runtime context.
    ///
    /// Use this to share a single captured context between validation
    /// and compose, avoiding redundant capture work.
    pub fn with_context(mut self, context: ComposeContext) -> Self {
        self.context = context;
        self
    }

    /// Enables or disables performance metric collection.
    #[must_use]
    pub fn with_perf(mut self, enabled: bool) -> Self {
        self.perf_enabled = enabled;
        self
    }

    // ── Getters ────────────────────────────────────────────────────

    /// Returns the maximum transclusion depth.
    #[must_use]
    pub fn max_transclusion_depth(&self) -> usize {
        self.max_transclusion_depth
    }
}

impl Default for ComposeOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Source context for compose execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeSource {
    /// Source is unknown (e.g., in-memory string).
    Unknown,
    /// Source is a local file.
    File(PathBuf),
    /// Source is a URL.
    Url(Url),
}

impl ComposeSource {
    /// Creates a file source from a path reference.
    pub fn infer_from_path(path: impl AsRef<Path>) -> Self {
        Self::File(path.as_ref().to_path_buf())
    }

    /// Returns a human-readable form of the source.
    ///
    /// - `File` → the path's lossy string form.
    /// - `Url`  → the URL's string form.
    /// - `Unknown` → `<stdin>`.
    ///
    /// Use this when surfacing the source in diagnostics or logs so callers
    /// don't have to special-case the URL/Unknown arms.
    pub fn display(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::File(path) => path.to_string_lossy(),
            Self::Url(url) => std::borrow::Cow::Borrowed(url.as_str()),
            Self::Unknown => std::borrow::Cow::Borrowed("<stdin>"),
        }
    }
}

/// Transclusion-specific options (internal convenience type).
///
/// These fields are mirrored on `ComposeOptions` for the public API.
/// This struct exists so internal functions (resolver, toc_linking) can
/// receive only the transclusion-related fields without coupling to
/// the full `ComposeOptions` type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransclusionOptions {
    /// Source context of the document being composed.
    pub source: ComposeSource,

    /// Maximum recursive include depth.
    pub max_depth: usize,

    /// Whether remote transclusion is allowed.
    pub allow_remote: bool,

    /// Whether local markdown files can be included with `::file`.
    pub allow_local_markdown: bool,

    /// Whether local text files can be included with `::code`.
    pub allow_local_code_text: bool,

    /// Fallback code language for unknown extensions.
    pub code_fallback_language: String,

    /// Explicit override for invalid-reference behavior.
    pub ignore_invalid: Option<bool>,

    /// Whether repo-root (`@`) resolution is enabled.
    pub resolve_repo_root: bool,

    /// Custom search roots for `@`-prefixed (magic) file references.
    pub magic_paths: Vec<(PathBuf, biscuit_file::PathPosition)>,
}

impl Default for TransclusionOptions {
    fn default() -> Self {
        Self {
            source: ComposeSource::Unknown,
            max_depth: 16,
            allow_remote: false,
            allow_local_markdown: true,
            allow_local_code_text: true,
            code_fallback_language: "txt".to_string(),
            ignore_invalid: None,
            resolve_repo_root: true,
            magic_paths: Vec::new(),
        }
    }
}

/// Runtime context captured at compose start for deterministic output.
///
/// All date/time values are captured once when the context is created,
/// ensuring consistent values throughout the compose pipeline even
/// if the compose takes significant time.
///
/// Backed by `Arc` for cheap cloning across transclusion graphs.
/// The `values` map is the canonical backing store for all context variables.
/// Legacy public fields are kept for backward compatibility but are also
/// mirrored in the `values` map.
#[derive(Debug, Clone)]
pub struct ComposeContext {
    inner: std::sync::Arc<ComposeContextInner>,
}

/// Inner storage for ComposeContext, shared via Arc.
#[derive(Debug, Clone)]
struct ComposeContextInner {
    now: String,
    now_utc: String,
    today: String,
    yesterday: String,
    tomorrow: String,
    day: String,
    day_abbr: String,
    year: String,
    month: String,
    month_name: String,
    month_name_abbr: String,
    env: HashMap<String, String>,
    values: serde_json::Map<String, serde_json::Value>,
    capture_diagnostics: Vec<super::context::ContextMergeDiagnostic>,
    capture_timings: Vec<(String, Duration)>,
}

impl PartialEq for ComposeContext {
    fn eq(&self, other: &Self) -> bool {
        // Same Arc instance = equal; otherwise compare values
        std::sync::Arc::ptr_eq(&self.inner, &other.inner) || self.inner.values == other.inner.values
    }
}

impl Eq for ComposeContext {}

// Legacy public field accessors (backward compatible)
impl ComposeContext {
    /// ISO 8601 local datetime.
    pub fn now(&self) -> &str {
        &self.inner.now
    }
    /// ISO 8601 UTC datetime.
    pub fn now_utc(&self) -> &str {
        &self.inner.now_utc
    }
    /// Local date YYYY-MM-DD.
    pub fn today(&self) -> &str {
        &self.inner.today
    }
    /// Yesterday YYYY-MM-DD.
    pub fn yesterday(&self) -> &str {
        &self.inner.yesterday
    }
    /// Tomorrow YYYY-MM-DD.
    pub fn tomorrow(&self) -> &str {
        &self.inner.tomorrow
    }
    /// Full day of week name.
    pub fn day(&self) -> &str {
        &self.inner.day
    }
    /// Abbreviated day of week.
    pub fn day_abbr(&self) -> &str {
        &self.inner.day_abbr
    }
    /// Four-digit year.
    pub fn year(&self) -> &str {
        &self.inner.year
    }
    /// Two-digit month (01-12).
    pub fn month(&self) -> &str {
        &self.inner.month
    }
    /// Full month name.
    pub fn month_name(&self) -> &str {
        &self.inner.month_name
    }
    /// Abbreviated month name.
    pub fn month_name_abbr(&self) -> &str {
        &self.inner.month_name_abbr
    }
    /// Environment variables snapshot.
    pub fn env(&self) -> &HashMap<String, String> {
        &self.inner.env
    }
    /// Access the values map.
    pub fn values(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.inner.values
    }
}

impl ComposeContext {
    /// Captures the current runtime context using CWD as the base directory.
    ///
    /// This snapshots:
    /// - Current local and UTC time
    /// - Today, yesterday, and tomorrow dates
    /// - Day of week (full and abbreviated)
    /// - Year, month (numeric and named)
    /// - All environment variables
    /// - Repository, monorepo, OS, and hardware information (via sniff)
    pub fn capture() -> Self {
        match std::env::current_dir() {
            Ok(base_dir) => Self::capture_for_dir(&base_dir),
            Err(e) => {
                // CWD discovery failed: populate date/time/env but leave
                // sniff-derived fields null, and record the failure.
                let (mut values, diagnostics) = (
                    serde_json::Map::new(),
                    vec![
                        super::context::ContextMergeDiagnostic::PartialRuntimeCapture {
                            area: "cwd",
                            detail: format!("current_dir() failed: {e}"),
                        },
                    ],
                );
                super::context::capture::populate_datetime(&mut values);
                Self::from_values(values, diagnostics, Vec::new())
            }
        }
    }

    /// Captures the runtime context using the given base directory.
    pub fn capture_for_dir(base_dir: &std::path::Path) -> Self {
        let (values, capture_diagnostics, timings) =
            super::context::capture::capture_runtime_context(base_dir);
        Self::from_values(values, capture_diagnostics, timings)
    }

    /// Demand-driven capture: scans `content` for `ctx.*` references and
    /// only captures the context groups actually needed.
    ///
    /// If the document uses no `ctx.*` variables, only date/time (zero I/O)
    /// is captured. This avoids git, repo, docs, OS, and hardware detection
    /// for documents that don't need them.
    ///
    /// **Note:** This scans only the provided string. If the document has
    /// frontmatter values containing `ctx.*` references, use
    /// [`capture_for_document`](Self::capture_for_document) instead.
    pub fn capture_for_content(base_dir: &std::path::Path, content: &str) -> Self {
        let (values, capture_diagnostics, timings) =
            super::context::capture::capture_runtime_context_for_content(base_dir, content);
        Self::from_values(values, capture_diagnostics, timings)
    }

    /// Demand-driven capture that scans both frontmatter values and body
    /// content for `ctx.*` references.
    ///
    /// This is the correct method when composing a full document, since
    /// frontmatter values may contain `ctx.*` references that are absent
    /// from the body.
    pub fn capture_for_document(
        base_dir: &std::path::Path,
        doc: &crate::markdown::Markdown,
    ) -> Self {
        let fm_json = serde_json::to_string(doc.frontmatter().as_map()).unwrap_or_default();
        let combined = format!("{}\n{}", fm_json, doc.content());
        Self::capture_for_content(base_dir, &combined)
    }

    /// Build a `ComposeContext` from pre-computed values.
    fn from_values(
        values: serde_json::Map<String, serde_json::Value>,
        capture_diagnostics: Vec<super::context::ContextMergeDiagnostic>,
        capture_timings: Vec<(String, Duration)>,
    ) -> Self {
        let env: HashMap<String, String> = std::env::vars().collect();

        let get_str = |key: &str| -> String {
            values
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        Self {
            inner: std::sync::Arc::new(ComposeContextInner {
                now: get_str("now"),
                now_utc: get_str("now_utc"),
                today: get_str("today"),
                yesterday: get_str("yesterday"),
                tomorrow: get_str("tomorrow"),
                day: get_str("day"),
                day_abbr: get_str("day_abbr"),
                year: get_str("year"),
                month: get_str("month"),
                month_name: get_str("month_name"),
                month_name_abbr: get_str("month_name_abbr"),
                env,
                values,
                capture_diagnostics,
                capture_timings,
            }),
        }
    }

    /// Looks up a value from the backing store.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.inner.values.get(key)
    }

    /// Returns the full context as a JSON object.
    pub fn as_object(&self) -> serde_json::Value {
        serde_json::Value::Object(self.inner.values.clone())
    }

    /// Returns diagnostics from the capture phase.
    pub fn diagnostics(&self) -> &[super::context::ContextMergeDiagnostic] {
        &self.inner.capture_diagnostics
    }

    /// Returns per-group timings from the capture phase.
    ///
    /// Each entry is `(group_name, elapsed)`. Empty when no groups
    /// required I/O (e.g., DateTime-only capture).
    pub fn capture_timings(&self) -> &[(String, Duration)] {
        &self.inner.capture_timings
    }

    /// Iterates the exposed key names.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.inner.values.keys().map(String::as_str)
    }

    /// Returns a mutable reference to the inner env map.
    ///
    /// Clones the `Arc` on write if shared. Use this to inject
    /// environment overrides before passing the context to
    /// [`ComposeOptions::new_with_context`].
    pub fn env_mut(&mut self) -> &mut HashMap<String, String> {
        &mut std::sync::Arc::make_mut(&mut self.inner).env
    }

    /// Creates a context with fixed values for testing.
    ///
    /// Uses an empty environment to ensure deterministic test behavior.
    #[cfg(test)]
    pub fn fixed_for_testing() -> Self {
        let mut values = serde_json::Map::new();
        let fields = [
            ("now", "2024-06-15T10:30:00"),
            ("now_utc", "2024-06-15T17:30:00Z"),
            ("today", "2024-06-15"),
            ("yesterday", "2024-06-14"),
            ("tomorrow", "2024-06-16"),
            ("day", "Saturday"),
            ("day_abbr", "Sat"),
            ("year", "2024"),
            ("month", "06"),
            ("month_name", "June"),
            ("month_name_abbr", "Jun"),
        ];
        for (k, v) in &fields {
            values.insert((*k).to_string(), serde_json::Value::String(v.to_string()));
        }

        let get_str = |key: &str| -> String {
            values
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        Self {
            inner: std::sync::Arc::new(ComposeContextInner {
                now: get_str("now"),
                now_utc: get_str("now_utc"),
                today: get_str("today"),
                yesterday: get_str("yesterday"),
                tomorrow: get_str("tomorrow"),
                day: get_str("day"),
                day_abbr: get_str("day_abbr"),
                year: get_str("year"),
                month: get_str("month"),
                month_name: get_str("month_name"),
                month_name_abbr: get_str("month_name_abbr"),
                env: HashMap::new(),
                values,
                capture_diagnostics: Vec::new(),
                capture_timings: Vec::new(),
            }),
        }
    }
}

/// A single timing metric from the compose pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposePerfMetric {
    /// Pipeline stage this metric represents.
    pub stage: ComposeStage,
    /// Accumulated elapsed time for this metric.
    pub elapsed: Duration,
    /// Number of times this metric was recorded.
    pub calls: usize,
}

/// Aggregated performance timings from the compose pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposePerfReport {
    /// Total compose pipeline time.
    pub total: Duration,
    /// Per-stage metrics in deterministic order.
    pub metrics: Vec<ComposePerfMetric>,
}

/// Named compose pipeline stages for type-safe metric identification.
///
/// Variants are listed in pipeline execution order so reports have
/// a deterministic, intuitive ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComposeStage {
    FrontmatterInterpolation,
    SchemaValidation,
    FrontmatterShellExpansion,
    EffectiveStateBuild,
    TextReplacement,
    PageBlocks,
    Interpolation,
    ShellExpansion,
    ShellBlocks,
    TransclusionParse,
    TransclusionPrepare,
    TransclusionResolve,
    TransclusionApply,
    LinkResolve,
    LinkNormalization,
    Cleanup,
    Normalization,
}

impl std::fmt::Display for ComposeStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::FrontmatterInterpolation => "frontmatter interpolation",
            Self::SchemaValidation => "schema validation",
            Self::FrontmatterShellExpansion => "frontmatter shell expansion",
            Self::EffectiveStateBuild => "effective state build",
            Self::TextReplacement => "text replacement",
            Self::PageBlocks => "page blocks",
            Self::Interpolation => "interpolation",
            Self::ShellExpansion => "shell expansion",
            Self::ShellBlocks => "shell blocks",
            Self::TransclusionParse => "transclusion parse",
            Self::TransclusionPrepare => "transclusion prepare",
            Self::TransclusionResolve => "transclusion resolve",
            Self::TransclusionApply => "transclusion apply",
            Self::LinkResolve => "link resolve",
            Self::LinkNormalization => "link normalization",
            Self::Cleanup => "cleanup",
            Self::Normalization => "normalization",
        })
    }
}

impl ComposePerfReport {
    /// Creates an empty perf report.
    pub fn new() -> Self {
        Self {
            total: Duration::ZERO,
            metrics: Vec::new(),
        }
    }

    /// Merges another perf report into this one by summing matching
    /// metric durations and call counts.
    pub fn merge(&mut self, other: &ComposePerfReport) {
        self.total += other.total;

        for other_metric in &other.metrics {
            if let Some(existing) = self
                .metrics
                .iter_mut()
                .find(|m| m.stage == other_metric.stage)
            {
                existing.elapsed += other_metric.elapsed;
                existing.calls += other_metric.calls;
            } else {
                self.metrics.push(*other_metric);
            }
        }
    }
}

impl Default for ComposePerfReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Report of changes made during compose execution.
///
/// Contains counts of changes made by each stage and any warnings
/// generated during processing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComposeReport {
    /// Number of frontmatter interpolation expressions resolved.
    pub frontmatter_interpolations_applied: usize,

    /// Number of frontmatter shell expansions applied.
    pub frontmatter_shell_expansions_applied: usize,

    /// Number of text replacements applied.
    pub replacements_applied: usize,

    /// Number of interpolations resolved.
    pub interpolations_applied: usize,

    /// Number of toc-linking directives expanded.
    pub toc_links_generated: usize,

    /// Number of shell expansions applied.
    pub shell_expansions_applied: usize,

    /// Number of shell blocks applied.
    pub shell_blocks_applied: usize,

    /// Number of shell approvals used.
    pub shell_approvals_used: usize,

    /// Whether the cleanup stage modified the content.
    pub cleanup_changed: bool,

    /// Normalization report if normalization was performed.
    pub normalization_report: Option<NormalizationReport>,

    /// Number of page blocks that evaluated to true and were rendered.
    pub page_blocks_rendered: usize,

    /// Number of page blocks that evaluated to false and were skipped.
    pub page_blocks_skipped: usize,

    /// Number of transclusions applied.
    pub transclusions_applied: usize,

    /// Number of transclusions skipped (conditions/invalid ignored).
    pub transclusions_skipped: usize,

    /// Number of local links resolved to absolute paths.
    pub link_resolves_applied: usize,

    /// Number of absolute paths normalized to portable forms.
    pub link_normalizations_applied: usize,

    /// Maximum recursive transclusion depth observed.
    pub max_transclusion_depth: usize,

    /// Warnings generated during compose (non-fatal issues).
    pub warnings: Vec<ComposeWarning>,

    /// Cache statistics from this compose run.
    pub cache_stats: Option<CacheStats>,

    /// Performance timings when `ComposeOptions::perf_enabled` is `true`.
    pub perf: Option<ComposePerfReport>,

    /// Source map tracking which byte ranges came from transcluded files.
    pub source_map: Vec<SourceRange>,
}

/// Maps a byte range in composed output to its originating source file.
///
/// Populated by `BlockTransclusion` when file content replaces a
/// `::file` directive. Byte positions refer to the final composed content.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceRange {
    /// Start byte offset in the composed output (inclusive).
    pub byte_start: usize,
    /// End byte offset in the composed output (exclusive).
    pub byte_end: usize,
    /// The source file whose content occupies this range.
    pub source_file: PathBuf,
    /// The starting line number in the source file (1-based).
    pub source_start_line: usize,
}

impl ComposeReport {
    /// Creates a new empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if any changes were made by any stage.
    pub fn has_changes(&self) -> bool {
        self.frontmatter_interpolations_applied > 0
            || self.frontmatter_shell_expansions_applied > 0
            || self.replacements_applied > 0
            || self.interpolations_applied > 0
            || self.toc_links_generated > 0
            || self.shell_expansions_applied > 0
            || self.shell_blocks_applied > 0
            || self.link_resolves_applied > 0
            || self.link_normalizations_applied > 0
            || self.cleanup_changed
            || self.page_blocks_rendered > 0
            || self.transclusions_applied > 0
            || self
                .normalization_report
                .as_ref()
                .is_some_and(|r| r.has_changes())
    }

    /// Returns a summary of changes made.
    pub fn summary(&self) -> String {
        if !self.has_changes() {
            return "No changes made".to_string();
        }

        let mut parts = Vec::new();

        if self.frontmatter_interpolations_applied > 0 {
            parts.push(format!(
                "{} frontmatter interpolation(s)",
                self.frontmatter_interpolations_applied
            ));
        }

        if self.frontmatter_shell_expansions_applied > 0 {
            parts.push(format!(
                "{} frontmatter shell expansion(s)",
                self.frontmatter_shell_expansions_applied
            ));
        }

        if self.replacements_applied > 0 {
            parts.push(format!("{} replacement(s)", self.replacements_applied));
        }

        if self.interpolations_applied > 0 {
            parts.push(format!("{} interpolation(s)", self.interpolations_applied));
        }

        if self.toc_links_generated > 0 {
            parts.push(format!("{} toc-link(s)", self.toc_links_generated));
        }

        if self.shell_expansions_applied > 0 {
            parts.push(format!(
                "{} shell expansion(s)",
                self.shell_expansions_applied
            ));
        }

        if self.shell_blocks_applied > 0 {
            parts.push(format!("{} shell block(s)", self.shell_blocks_applied));
        }

        if self.shell_approvals_used > 0 {
            parts.push(format!("{} shell approval(s)", self.shell_approvals_used));
        }

        if self.link_resolves_applied > 0 {
            parts.push(format!("{} link resolve(s)", self.link_resolves_applied));
        }

        if self.link_normalizations_applied > 0 {
            parts.push(format!(
                "{} link normalization(s)",
                self.link_normalizations_applied
            ));
        }

        if self.cleanup_changed {
            parts.push("cleanup applied".to_string());
        }

        if self.page_blocks_rendered > 0 {
            parts.push(format!(
                "{} page block(s) rendered",
                self.page_blocks_rendered
            ));
        }

        if self.page_blocks_skipped > 0 {
            parts.push(format!(
                "{} page block(s) skipped",
                self.page_blocks_skipped
            ));
        }

        if self.transclusions_applied > 0 {
            parts.push(format!("{} transclusion(s)", self.transclusions_applied));
        }

        if self.transclusions_skipped > 0 {
            parts.push(format!(
                "{} transclusion(s) skipped",
                self.transclusions_skipped
            ));
        }

        if let Some(ref norm) = self.normalization_report
            && norm.has_changes()
        {
            parts.push(format!("normalization: {}", norm.summary()));
        }

        if let Some(ref stats) = self.cache_stats
            && stats.has_activity()
        {
            parts.push(format!(
                "cache: {} hit(s), {} miss(es)",
                stats.hits, stats.misses
            ));
        }

        parts.join(", ")
    }

    /// Adds a warning to the report.
    pub fn add_warning(&mut self, warning: ComposeWarning) {
        self.warnings.push(warning);
    }

    /// Merges another report into this one.
    pub fn merge(&mut self, mut other: ComposeReport) {
        self.frontmatter_interpolations_applied += other.frontmatter_interpolations_applied;
        self.frontmatter_shell_expansions_applied += other.frontmatter_shell_expansions_applied;
        self.replacements_applied += other.replacements_applied;
        self.interpolations_applied += other.interpolations_applied;
        self.toc_links_generated += other.toc_links_generated;
        self.shell_expansions_applied += other.shell_expansions_applied;
        self.shell_blocks_applied += other.shell_blocks_applied;
        self.shell_approvals_used += other.shell_approvals_used;
        self.link_resolves_applied += other.link_resolves_applied;
        self.link_normalizations_applied += other.link_normalizations_applied;
        self.cleanup_changed |= other.cleanup_changed;
        self.page_blocks_rendered += other.page_blocks_rendered;
        self.page_blocks_skipped += other.page_blocks_skipped;
        self.transclusions_applied += other.transclusions_applied;
        self.transclusions_skipped += other.transclusions_skipped;
        self.max_transclusion_depth = self
            .max_transclusion_depth
            .max(other.max_transclusion_depth);

        if self.normalization_report.is_none() {
            self.normalization_report = other.normalization_report.take();
        }

        self.warnings.append(&mut other.warnings);

        // Merge cache stats
        match (&mut self.cache_stats, other.cache_stats) {
            (Some(self_stats), Some(ref other_stats)) => self_stats.merge(other_stats),
            (None, Some(other_stats)) => self.cache_stats = Some(other_stats),
            _ => {}
        }

        // Merge perf reports
        match (&mut self.perf, other.perf) {
            (Some(self_perf), Some(ref other_perf)) => self_perf.merge(other_perf),
            (None, Some(other_perf)) => self.perf = Some(other_perf),
            _ => {}
        }
    }
}

/// A warning generated during compose processing.
///
/// Warnings indicate non-fatal issues that did not prevent the
/// transform from completing (when `fail_fast = false`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeWarning {
    /// The stage that generated this warning.
    pub stage: String,

    /// Human-readable description of the issue.
    pub message: String,

    /// Line number where the issue occurred (1-indexed), if applicable.
    pub line_number: Option<usize>,
}

impl ComposeWarning {
    /// Creates a new warning.
    pub fn new(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            message: message.into(),
            line_number: None,
        }
    }

    /// Adds a line number to this warning.
    #[must_use]
    pub fn at_line(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_source_display_covers_all_variants() {
        assert_eq!(ComposeSource::Unknown.display(), "<stdin>");
        assert_eq!(
            ComposeSource::File(PathBuf::from("/tmp/doc.md")).display(),
            "/tmp/doc.md"
        );
        let url = url::Url::parse("https://example.com/doc.md").unwrap();
        assert_eq!(
            ComposeSource::Url(url).display(),
            "https://example.com/doc.md"
        );
    }

    #[test]
    fn test_compose_options_new_captures_context() {
        let options = ComposeOptions::new();
        let ctx = options.context();

        // Context should have captured current date
        assert!(!ctx.today().is_empty());
        assert!(!ctx.year().is_empty());
        assert!(!ctx.day().is_empty());
    }

    #[test]
    fn test_compose_options_default_stages() {
        let options = ComposeOptions::new();

        assert!(options.is_enabled(ComposeOperation::FrontmatterInterpolation));
        assert!(options.is_enabled(ComposeOperation::TextReplacement));
        assert!(options.is_enabled(ComposeOperation::Interpolation));
        assert!(options.is_enabled(ComposeOperation::Cleanup));
        assert!(options.is_enabled(ComposeOperation::Normalization));
        assert!(options.is_enabled(ComposeOperation::BlockTransclusion));
        assert!(options.is_enabled(ComposeOperation::FrontmatterTransclusion));
    }

    #[test]
    fn test_transclusion_options_defaults() {
        let options = ComposeOptions::new();
        assert_eq!(options.max_transclusion_depth, 16);
        assert!(matches!(options.source, ComposeSource::Unknown));
        assert_eq!(options.code_fallback_language, "txt");
    }

    #[test]
    fn test_compose_options_builder_pattern() {
        let mut options = ComposeOptions::new()
            .disable(ComposeOperation::Cleanup)
            .only(&[
                ComposeOperation::BlockTransclusion,
                ComposeOperation::CodeTransclusion,
            ])
            .with_fail_fast(true)
            .with_external_state(serde_json::json!({"key": "value"}));

        options.max_transclusion_depth = 8;

        assert!(!options.is_enabled(ComposeOperation::Cleanup));
        assert!(options.is_enabled(ComposeOperation::BlockTransclusion));
        assert!(!options.is_enabled(ComposeOperation::FrontmatterTransclusion));
        assert_eq!(options.max_transclusion_depth, 8);
        assert!(options.fail_fast);
        assert!(options.external_state.is_some());
    }

    #[test]
    fn test_compose_options_with_context() {
        let fixed_ctx = ComposeContext::fixed_for_testing();
        let options = ComposeOptions::new().with_context(fixed_ctx.clone());

        assert_eq!(options.context().today(), "2024-06-15");
        assert_eq!(options.context().year(), "2024");
    }

    #[test]
    fn test_compose_options_enable_disable() {
        let mut options = ComposeOptions::new();

        // All operations enabled by default
        assert!(options.is_enabled(ComposeOperation::TextReplacement));
        assert!(options.is_enabled(ComposeOperation::Cleanup));

        // Disable cleanup
        options = options.disable(ComposeOperation::Cleanup);
        assert!(!options.is_enabled(ComposeOperation::Cleanup));
        assert!(options.is_enabled(ComposeOperation::TextReplacement));
    }

    #[test]
    fn test_compose_operation_default_order_exact() {
        assert_eq!(
            ComposeOperation::default_order(),
            &[
                ComposeOperation::FrontmatterInterpolation,
                ComposeOperation::FrontmatterShellExpansion,
                ComposeOperation::TextReplacement,
                ComposeOperation::PageBlocks,
                ComposeOperation::Interpolation,
                ComposeOperation::ShellExpansion,
                ComposeOperation::ShellBlocks,
                ComposeOperation::LinkResolve,
                ComposeOperation::BlockTransclusion,
                ComposeOperation::FrontmatterTransclusion,
                ComposeOperation::CodeTransclusion,
                ComposeOperation::TocLinking,
                ComposeOperation::Cleanup,
                ComposeOperation::Normalization,
                ComposeOperation::LinkNormalization,
            ]
        );
    }

    #[test]
    fn test_compose_operation_phase_mapping_is_complete() {
        let expectations = [
            (
                ComposeOperation::FrontmatterInterpolation,
                ComposePhase::InlinePre,
            ),
            (
                ComposeOperation::FrontmatterShellExpansion,
                ComposePhase::InlinePre,
            ),
            (ComposeOperation::TextReplacement, ComposePhase::InlinePre),
            (ComposeOperation::PageBlocks, ComposePhase::InlinePre),
            (ComposeOperation::Interpolation, ComposePhase::InlinePre),
            (ComposeOperation::ShellExpansion, ComposePhase::InlinePre),
            (ComposeOperation::ShellBlocks, ComposePhase::InlinePre),
            (
                ComposeOperation::BlockTransclusion,
                ComposePhase::Transclusion,
            ),
            (
                ComposeOperation::FrontmatterTransclusion,
                ComposePhase::Transclusion,
            ),
            (
                ComposeOperation::CodeTransclusion,
                ComposePhase::Transclusion,
            ),
            (ComposeOperation::TocLinking, ComposePhase::Transclusion),
            (ComposeOperation::Cleanup, ComposePhase::InlinePost),
            (ComposeOperation::Normalization, ComposePhase::InlinePost),
            (ComposeOperation::LinkResolve, ComposePhase::InlinePre),
            (
                ComposeOperation::LinkNormalization,
                ComposePhase::Finalization,
            ),
        ];

        for (operation, expected_phase) in expectations {
            assert_eq!(operation.phase(), expected_phase, "{operation:?}");
        }
    }

    #[test]
    fn test_compose_operation_all_is_complete_and_ordered() {
        let all = ComposeOperation::all();
        let collected = all.iter().collect::<Vec<_>>();
        assert_eq!(collected, ComposeOperation::default_order());
        assert_eq!(collected.len(), ComposeOperation::COUNT);
    }

    #[test]
    fn test_compose_options_only() {
        let options = ComposeOptions::new().only(&[
            ComposeOperation::TextReplacement,
            ComposeOperation::Interpolation,
        ]);

        assert!(options.is_enabled(ComposeOperation::TextReplacement));
        assert!(options.is_enabled(ComposeOperation::Interpolation));
        assert!(!options.is_enabled(ComposeOperation::Cleanup));
        assert!(!options.is_enabled(ComposeOperation::Normalization));
        assert!(!options.is_enabled(ComposeOperation::BlockTransclusion));
    }

    #[test]
    fn test_compose_options_only_empty() {
        let options = ComposeOptions::new().only(&[]);

        assert!(!options.is_enabled(ComposeOperation::TextReplacement));
        assert!(!options.is_enabled(ComposeOperation::Cleanup));
        assert!(!options.is_enabled(ComposeOperation::BlockTransclusion));
    }

    #[test]
    fn test_compose_options_flat_builders() {
        let handler: std::sync::Arc<dyn super::super::shell_expansion::ShellApprovalHandler> =
            std::sync::Arc::new(TestApprovalHandler);
        let options = ComposeOptions::new()
            .with_shell_timeout(std::time::Duration::from_secs(3))
            .with_shell_policy_root("/tmp/policy")
            .with_shell_working_directory("/tmp/work")
            .with_shell_approval_handler(handler)
            .with_allow_remote_transclusion(true)
            .with_allow_local_markdown(false)
            .with_allow_local_code(false)
            .with_max_transclusion_depth(4)
            .with_ignore_invalid_references(Some(true))
            .with_resolve_repo_root(false)
            .with_code_fallback_language("md");

        assert_eq!(options.shell_timeout, std::time::Duration::from_secs(3));
        assert_eq!(
            options.shell_policy_root,
            Some(PathBuf::from("/tmp/policy"))
        );
        assert_eq!(
            options.shell_working_directory,
            Some(PathBuf::from("/tmp/work"))
        );
        assert!(options.shell_approval_handler.is_some());
        assert!(options.allow_remote_transclusion);
        assert!(!options.allow_local_markdown);
        assert!(!options.allow_local_code);
        assert_eq!(options.max_transclusion_depth, 4);
        assert_eq!(options.ignore_invalid_references, Some(true));
        assert!(!options.resolve_repo_root);
        assert_eq!(options.code_fallback_language, "md");
    }

    #[test]
    fn test_compose_context_capture() {
        let ctx = ComposeContext::capture();

        // Should have reasonable values
        assert!(ctx.year().parse::<i32>().is_ok());
        assert!(ctx.month().len() == 2);
        assert!(!ctx.today().is_empty());
        assert!(!ctx.yesterday().is_empty());
        assert!(!ctx.tomorrow().is_empty());
    }

    #[test]
    fn test_compose_context_fixed_for_testing() {
        let ctx = ComposeContext::fixed_for_testing();

        assert_eq!(ctx.today(), "2024-06-15");
        assert_eq!(ctx.yesterday(), "2024-06-14");
        assert_eq!(ctx.tomorrow(), "2024-06-16");
        assert_eq!(ctx.day(), "Saturday");
        assert_eq!(ctx.year(), "2024");
        assert_eq!(ctx.month(), "06");
    }

    #[test]
    fn test_compose_report_new() {
        let report = ComposeReport::new();

        assert_eq!(report.replacements_applied, 0);
        assert_eq!(report.interpolations_applied, 0);
        assert!(!report.cleanup_changed);
        assert!(report.normalization_report.is_none());
        assert_eq!(report.transclusions_applied, 0);
        assert_eq!(report.transclusions_skipped, 0);
        assert_eq!(report.max_transclusion_depth, 0);
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn test_compose_report_has_changes() {
        let mut report = ComposeReport::new();
        assert!(!report.has_changes());

        report.replacements_applied = 1;
        assert!(report.has_changes());
    }

    #[test]
    fn test_compose_report_summary() {
        let mut report = ComposeReport::new();
        assert_eq!(report.summary(), "No changes made");

        report.replacements_applied = 2;
        report.interpolations_applied = 3;
        report.cleanup_changed = true;
        report.transclusions_applied = 1;
        report.transclusions_skipped = 1;

        let summary = report.summary();
        assert!(summary.contains("2 replacement(s)"));
        assert!(summary.contains("3 interpolation(s)"));
        assert!(summary.contains("cleanup applied"));
        assert!(summary.contains("1 transclusion(s)"));
        assert!(summary.contains("1 transclusion(s) skipped"));
    }

    #[test]
    fn test_compose_warning() {
        let warning = ComposeWarning::new("interpolation", "Missing variable: foo");
        assert_eq!(warning.stage, "interpolation");
        assert_eq!(warning.message, "Missing variable: foo");
        assert!(warning.line_number.is_none());

        let warning_with_line = warning.at_line(42);
        assert_eq!(warning_with_line.line_number, Some(42));
    }

    #[test]
    fn test_compose_report_add_warning() {
        let mut report = ComposeReport::new();
        report.add_warning(ComposeWarning::new("test", "test warning"));

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].message, "test warning");
    }

    struct TestApprovalHandler;

    impl super::super::shell_expansion::ShellApprovalHandler for TestApprovalHandler {
        fn approve(
            &self,
            _request: super::super::shell_expansion::ShellApprovalRequest,
        ) -> Result<
            super::super::shell_expansion::ShellApprovalDecision,
            super::super::ShellExpansionError,
        > {
            Ok(super::super::shell_expansion::ShellApprovalDecision::Deny)
        }
    }

    #[test]
    fn with_magic_path_accumulates_entries() {
        use biscuit_file::PathPosition;

        let options = ComposeOptions::new()
            .with_magic_path("/project/.claudine", PathPosition::Start)
            .with_magic_path("/home/user/.claudine", PathPosition::Start)
            .with_magic_path("/fallback", PathPosition::End);

        assert_eq!(options.magic_paths.len(), 3);
        assert_eq!(
            options.magic_paths[0].0,
            PathBuf::from("/project/.claudine")
        );
        assert_eq!(options.magic_paths[0].1, PathPosition::Start);
        assert_eq!(
            options.magic_paths[1].0,
            PathBuf::from("/home/user/.claudine")
        );
        assert_eq!(options.magic_paths[1].1, PathPosition::Start);
        assert_eq!(options.magic_paths[2].0, PathBuf::from("/fallback"));
        assert_eq!(options.magic_paths[2].1, PathPosition::End);
    }

    #[test]
    fn magic_paths_appear_in_transclusion_options() {
        use biscuit_file::PathPosition;

        let options = ComposeOptions::new().with_magic_path("/custom/root", PathPosition::Start);

        let transclusion = options.transclusion_options();
        assert_eq!(transclusion.magic_paths.len(), 1);
        assert_eq!(transclusion.magic_paths[0].0, PathBuf::from("/custom/root"));
        assert_eq!(transclusion.magic_paths[0].1, PathPosition::Start);
    }

    #[test]
    fn magic_paths_default_empty() {
        let options = ComposeOptions::new();
        assert!(options.magic_paths.is_empty());

        let transclusion = options.transclusion_options();
        assert!(transclusion.magic_paths.is_empty());
    }

    #[test]
    fn perf_disabled_by_default() {
        let options = ComposeOptions::new();
        assert!(!options.perf_enabled);
    }

    #[test]
    fn with_perf_enables_collection() {
        let options = ComposeOptions::new().with_perf(true);
        assert!(options.perf_enabled);
    }

    #[test]
    fn compose_report_perf_none_by_default() {
        let report = ComposeReport::new();
        assert!(report.perf.is_none());
    }

    #[test]
    fn compose_perf_report_merge_sums_durations() {
        let mut parent = ComposePerfReport {
            total: Duration::from_millis(10),
            metrics: vec![
                ComposePerfMetric {
                    stage: ComposeStage::Cleanup,
                    elapsed: Duration::from_millis(3),
                    calls: 1,
                },
                ComposePerfMetric {
                    stage: ComposeStage::Interpolation,
                    elapsed: Duration::from_millis(5),
                    calls: 2,
                },
            ],
        };
        let child = ComposePerfReport {
            total: Duration::from_millis(7),
            metrics: vec![
                ComposePerfMetric {
                    stage: ComposeStage::Cleanup,
                    elapsed: Duration::from_millis(2),
                    calls: 1,
                },
                ComposePerfMetric {
                    stage: ComposeStage::Normalization,
                    elapsed: Duration::from_millis(4),
                    calls: 1,
                },
            ],
        };

        parent.merge(&child);

        assert_eq!(parent.total, Duration::from_millis(17));

        let cleanup = parent
            .metrics
            .iter()
            .find(|m| m.stage == ComposeStage::Cleanup)
            .unwrap();
        assert_eq!(cleanup.elapsed, Duration::from_millis(5));
        assert_eq!(cleanup.calls, 2);

        let interp = parent
            .metrics
            .iter()
            .find(|m| m.stage == ComposeStage::Interpolation)
            .unwrap();
        assert_eq!(interp.elapsed, Duration::from_millis(5));
        assert_eq!(interp.calls, 2);

        let norm = parent
            .metrics
            .iter()
            .find(|m| m.stage == ComposeStage::Normalization)
            .unwrap();
        assert_eq!(norm.elapsed, Duration::from_millis(4));
        assert_eq!(norm.calls, 1);
    }

    #[test]
    fn compose_report_merge_with_perf_none_and_some() {
        let mut report_a = ComposeReport::new();
        assert!(report_a.perf.is_none());

        let mut report_b = ComposeReport::new();
        report_b.perf = Some(ComposePerfReport {
            total: Duration::from_millis(5),
            metrics: vec![ComposePerfMetric {
                stage: ComposeStage::Cleanup,
                elapsed: Duration::from_millis(5),
                calls: 1,
            }],
        });

        report_a.merge(report_b);
        assert!(report_a.perf.is_some());
        assert_eq!(
            report_a.perf.as_ref().unwrap().total,
            Duration::from_millis(5)
        );
    }

    #[test]
    fn compose_report_merge_with_both_perf() {
        let mut report_a = ComposeReport::new();
        report_a.perf = Some(ComposePerfReport {
            total: Duration::from_millis(10),
            metrics: vec![ComposePerfMetric {
                stage: ComposeStage::Cleanup,
                elapsed: Duration::from_millis(3),
                calls: 1,
            }],
        });

        let mut report_b = ComposeReport::new();
        report_b.perf = Some(ComposePerfReport {
            total: Duration::from_millis(7),
            metrics: vec![ComposePerfMetric {
                stage: ComposeStage::Cleanup,
                elapsed: Duration::from_millis(2),
                calls: 1,
            }],
        });

        report_a.merge(report_b);
        let perf = report_a.perf.as_ref().unwrap();
        assert_eq!(perf.total, Duration::from_millis(17));
        assert_eq!(perf.metrics[0].elapsed, Duration::from_millis(5));
        assert_eq!(perf.metrics[0].calls, 2);
    }

    #[test]
    fn compose_options_default_timeout_behavior_is_error() {
        let options = ComposeOptions::new();
        assert_eq!(options.shell_timeout_behavior, ShellTimeoutBehavior::Error);
    }

    #[test]
    fn with_shell_timeout_behavior_sets_value() {
        let options =
            ComposeOptions::new().with_shell_timeout_behavior(ShellTimeoutBehavior::EmptyString);
        assert_eq!(
            options.shell_timeout_behavior,
            ShellTimeoutBehavior::EmptyString
        );
    }

    #[test]
    fn with_allow_shell_timeout_sets_empty_string() {
        let options = ComposeOptions::new().with_allow_shell_timeout(true);
        assert_eq!(
            options.shell_timeout_behavior,
            ShellTimeoutBehavior::EmptyString
        );
    }

    #[test]
    fn with_allow_shell_timeout_false_keeps_error() {
        let options = ComposeOptions::new().with_allow_shell_timeout(false);
        assert_eq!(options.shell_timeout_behavior, ShellTimeoutBehavior::Error);
    }

    #[test]
    fn frontmatter_shell_expansion_is_inline_pre() {
        assert_eq!(
            ComposeOperation::FrontmatterShellExpansion.phase(),
            ComposePhase::InlinePre
        );
    }

    #[test]
    fn frontmatter_shell_expansion_follows_interpolation_in_default_order() {
        let order = ComposeOperation::default_order();
        let fm_interp_pos = order
            .iter()
            .position(|op| *op == ComposeOperation::FrontmatterInterpolation)
            .unwrap();
        let fm_shell_pos = order
            .iter()
            .position(|op| *op == ComposeOperation::FrontmatterShellExpansion)
            .unwrap();
        assert_eq!(fm_shell_pos, fm_interp_pos + 1);
    }

    #[test]
    fn compose_report_tracks_frontmatter_shell_expansions() {
        let report = ComposeReport::new();
        assert_eq!(report.frontmatter_shell_expansions_applied, 0);
    }

    #[test]
    fn default_order_has_fifteen_operations() {
        assert_eq!(ComposeOperation::default_order().len(), 15);
    }

    #[test]
    fn link_resolve_is_last_inline_pre_operation() {
        let order = ComposeOperation::default_order();
        let inline_pre = order
            .iter()
            .copied()
            .filter(|op| op.phase() == ComposePhase::InlinePre)
            .collect::<Vec<_>>();
        assert_eq!(
            inline_pre.last().copied(),
            Some(ComposeOperation::LinkResolve)
        );
    }

    #[test]
    fn link_normalization_is_sole_finalization_operation() {
        let finalization = ComposeOperation::default_order()
            .iter()
            .copied()
            .filter(|op| op.phase() == ComposePhase::Finalization)
            .collect::<Vec<_>>();
        assert_eq!(finalization, vec![ComposeOperation::LinkNormalization]);
    }

    #[test]
    fn finalization_phase_displays_lowercase() {
        assert_eq!(format!("{}", ComposePhase::Finalization), "finalization");
    }

    #[test]
    fn env_path_whitelist_default_is_empty_with_known_fallbacks() {
        let options = ComposeOptions::new();
        assert!(options.env_path_whitelist.is_empty());
        assert_eq!(
            options.effective_env_path_whitelist(),
            vec!["PROJECT_ROOT".to_string(), "DOCS_BASE".to_string()]
        );
    }

    #[test]
    fn with_env_path_whitelist_overrides_default() {
        let options = ComposeOptions::new()
            .with_env_path_whitelist(vec!["MY_VAR".to_string(), "OTHER".to_string()]);
        assert_eq!(
            options.effective_env_path_whitelist(),
            vec!["MY_VAR".to_string(), "OTHER".to_string()]
        );
    }
}
