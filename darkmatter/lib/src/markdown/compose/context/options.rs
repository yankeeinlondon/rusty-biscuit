//! Compose pipeline configuration (`ComposeOptions`), its source context
//! (`ComposeSource`), and the internal `TransclusionOptions` view.

use super::super::cache::{CacheAccessMode, FileStore};
use super::super::pipeline::operations::{ComposeOperation, ComposeOperationSet};
use super::super::preflight::PreflightGraphNode;
use super::super::remote::{RemoteFreshnessMode, RemoteReadConfig};
use super::super::remote_fetch::RemoteFetchRuntime;
use super::super::shell_expansion::types::ShellTimeoutBehavior;
use super::super::remote_fetch::RemoteFetchWeakId;
use super::super::shell_expansion::ShellApprovalHandler;
use super::runtime::ComposeContext;
use biscuit_hash::{xx_hash, xx_hash_bytes};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use std::time::Duration;
use url::Url;

/// The operations [`ComposeOptions::only_frontmatter_surface`] may keep enabled.
const FRONTMATTER_SURFACE_OPERATIONS: [ComposeOperation; 2] = [
    ComposeOperation::FrontmatterInterpolation,
    ComposeOperation::FrontmatterShellExpansion,
];

/// One immutable caller override paired with the context that authored it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallerInputRecord {
    raw: serde_json::Value,
    origin: biscuit_file::FileResolutionContext,
}

impl CallerInputRecord {
    /// Pair an unmodified caller value with its captured resolution context.
    pub fn new(
        raw: serde_json::Value,
        origin: biscuit_file::FileResolutionContext,
    ) -> Self {
        Self { raw, origin }
    }

    /// The unmodified value supplied by the caller.
    pub fn raw(&self) -> &serde_json::Value {
        &self.raw
    }

    /// The immutable context in which the caller authored the value.
    pub fn origin(&self) -> &biscuit_file::FileResolutionContext {
        &self.origin
    }
}

/// Immutable caller overrides keyed by the winning top-level property.
pub type CallerInputRecords = std::collections::BTreeMap<String, CallerInputRecord>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum SourceDerivation {
    #[default]
    Ordinary,
    TrustedExternal,
}

/// The reference that opened a file source, and the path the opening
/// document's context resolved it to.
///
/// [`ComposeSource::File`] holds the canonical path, which is the source's
/// identity (cycle detection, caches, the pre-flight graph). The source's
/// [`FileResolutionContext`](biscuit_file::FileResolutionContext) is derived
/// from this record instead, so a `~` or `{{VAR}}` anchor survives as the tree
/// root and the source keeps the spelling of the tree it was resolved in
/// (macOS `/var` versus `/private/var`).
#[derive(Debug, Clone)]
pub(crate) struct SourceOpening {
    pub(crate) reference: biscuit_file::FileReference,
    pub(crate) resolved: PathBuf,
}

// `FileReference` has no `PartialEq`; equal raw text parses to an equal
// reference.
impl PartialEq for SourceOpening {
    fn eq(&self, other: &Self) -> bool {
        self.reference.raw() == other.reference.raw() && self.resolved == other.resolved
    }
}

impl Eq for SourceOpening {}

/// Derives a file source's context from the request snapshot.
///
/// The one derivation rule for every Darkmatter surface that resolves a file
/// source's references. With an opening record the source keeps its opening
/// anchor and resolved spelling; without one (a root document given as a
/// path) it is derived from `path`.
pub(crate) fn source_file_context(
    snapshot: &biscuit_file::FileResolutionContext,
    path: &Path,
    derivation: SourceDerivation,
    opening: Option<&SourceOpening>,
) -> biscuit_file::FileResolutionContext {
    match (opening, derivation) {
        (Some(opening), SourceDerivation::Ordinary) => {
            snapshot.for_source_reference(&opening.reference, &opening.resolved)
        }
        (Some(opening), SourceDerivation::TrustedExternal) => {
            snapshot.for_trusted_external_source_reference(&opening.reference, &opening.resolved)
        }
        (None, SourceDerivation::Ordinary) => snapshot.for_source(path),
        (None, SourceDerivation::TrustedExternal) => snapshot.for_trusted_external_source(path),
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
    /// When `false` (default), recoverable failures in stages such as TOC
    /// linking and non-structural transclusion are recorded as warnings and
    /// the pipeline continues. An expression that cannot be parsed or
    /// evaluated is never recoverable: it fails the document either way.
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

    /// Override values that overwrite frontmatter keys as **data**: never
    /// scanned for `{{ … }}`, `{{{ … }}}`, or whole-value `$( … )`. Applied
    /// after [`set_overrides`](Self::set_overrides), so a key present in both
    /// is data.
    pub(crate) data_overrides: Option<serde_json::Value>,

    /// Origin of the values this document receives from the document that
    /// transcludes it. Never propagated to grandchildren.
    pub(crate) inherited_origin: InheritedOrigin,

    /// Raw caller overrides and their per-property authoring contexts.
    pub(crate) caller_input_records: CallerInputRecords,

    /// Schema-selected property/array occurrences mapped back to caller provenance.
    pub(crate) caller_file_provenance:
        std::collections::HashMap<String, super::super::expression::resolve_ctx::CallerFileProvenance>,

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

    /// How the current file source entered this compose run.
    pub(crate) source_derivation: SourceDerivation,

    /// The reference that opened the current file source, when a parent
    /// document resolved it. Cleared whenever `source` is replaced.
    pub(crate) source_opening: Option<SourceOpening>,

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
        Option<std::sync::Arc<dyn super::super::shell_expansion::ShellApprovalHandler>>,

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

    /// Controls whether cleanup collapses incidental single newlines
    /// in prose. Default: `Strip`.
    pub(crate) incidental_newline_mode: crate::markdown::cleanup::IncidentalNewlineMode,

    /// Optional fixed column width for cleanup reflow. Default: `None`.
    pub(crate) fixed_width: Option<usize>,

    /// Number of spaces per nesting level for list indentation
    /// during cleanup. Default: 4.
    pub(crate) indent_size: usize,

    // ── Caching ───────────────────────────────────────────────────
    /// Controls the run-local, in-memory compose cache (never persisted).
    /// Default: `ReadWrite` (full caching with single-flight dedup).
    pub(crate) cache_access_mode: CacheAccessMode,

    /// Root of the remote transport cache, at
    /// `<cache_root>/.darkmatter/cache/v1[/<namespace>]/`. Raw remote URL
    /// response bodies are the only artifact written here; no semantic result
    /// (composed document, `::file` child, operation result, snapshot) ever is.
    /// When `None`, nothing is persisted.
    pub(crate) cache_root: Option<PathBuf>,

    /// Namespace (e.g., branch name, profile) appended to the remote
    /// transport-cache root, isolating one context's fetched bodies from
    /// another's.
    pub(crate) cache_namespace: Option<String>,

    // ── Performance ─────────────────────────────────────────────────
    /// When `true`, the pipeline collects per-stage timing metrics
    /// and populates `ComposeReport::perf`. Default: `false`.
    pub(crate) perf_enabled: bool,

    // ── Internal (crate-private) ───────────────────────────────────
    /// Runtime context captured at construction time (timestamps,
    /// environment variables).
    context: ComposeContext,

    /// Whether composition may grow [`context`](Self::context) when a source
    /// names a `ctx.*` group it has not captured.
    ///
    /// The root document and every transcluded source are extended through
    /// this authority before their first expression stage. A frozen
    /// (`CallerSupplied`) context is never augmented: a pinned test snapshot or
    /// an evidence-backed capture must fail rather than consult the host.
    context_authority: super::authority::ContextAuthority,

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

    /// Whether `baseline_schema` is the authored Darkmatter baseline (set via
    /// [`ComposeOptions::with_darkmatter_baseline_schema`]). When true, schema
    /// validation shares the process-cached compiled JSON Schema instead of
    /// re-converting or cloning it per compose (F8/F9).
    pub(crate) baseline_is_darkmatter_default: bool,

    /// Whether file-backed compose validation discovers trigger schemas from
    /// the source file up through its repository boundary.
    pub(crate) trigger_schemas: bool,

    // ── Remote reads ────────────────────────────────────────────
    /// Configuration for remote URL reads: allowed hosts, concurrency,
    /// TTL, freshness mode, and refresh behavior.
    ///
    /// Defaults to deny-all. Wired into eager prefetch, read-side expression
    /// resolution (`frontmatter(url)`, …), and the remote transport cache.
    pub(crate) remote_read_config: RemoteReadConfig,

    // ── Schema validation ──────────────────────────────────────────
    /// When `true`, the schema-validation stage defers problems on frontmatter
    /// values still holding a `$(...)` shell expression even though
    /// `FrontmatterShellExpansion` is not in the enabled set. Used by the
    /// shell-command discovery pass: it strips shell expansion (to avoid running
    /// commands) but a *later* terminal compose pass re-validates the resolved
    /// frontmatter, so a still-literal `$(...)` value is not yet a final
    /// violation — while a genuinely-bad non-`$(...)` value (e.g. an empty
    /// required string) must still fail fast here. Default: `false`.
    pub(crate) defer_shell_pending_schema_problems: bool,

    /// When `true`, the schema-validation stage still coerces frontmatter to
    /// the declared types but reports no problems: this pass is not the one
    /// that owns the document's schema verdict.
    ///
    /// Two callers need it. A shell-discovery pass composes only to find
    /// commands. And a pre-`initialize` bootstrap read composes a document whose
    /// own `initialize` may still add or repair a schema property — judging it
    /// here would fail the document for a violation the very next stage is about
    /// to fix. In both cases a later terminal pass composes the settled document
    /// and reports the verdict. Default: `false`.
    pub(crate) defer_schema_verdict: bool,

    /// When `true`, body interpolation renders a `ctx.*` variable whose group
    /// the request never captured as `null` instead of failing. Used only by
    /// the shell-command discovery pass, which composes without page blocks:
    /// it would otherwise evaluate content a false `::block` removes before
    /// the terminal pass, which owns the missing-capture verdict. Default:
    /// `false`.
    pub(crate) defer_missing_runtime_context: bool,

    /// When `true`, a body or mixed-text frontmatter expression that cannot be
    /// parsed or evaluated warns and keeps its span instead of failing the
    /// document. Used only by the shell-command discovery pass, which composes
    /// without page blocks and so evaluates content a false `::block` removes;
    /// the terminal pass owns the verdict and is always strict. Default:
    /// `false`.
    pub(crate) defer_expression_failures: bool,

    /// When `true`, `has_alias`, `has_builtin_function`, `has_user_function`,
    /// and the shell half of `can_execute` answer `false` without launching the
    /// login shell. Set only by the shell-command discovery pass, which R8
    /// forbids from running a profile; the terminal pass probes. Default:
    /// `false`.
    pub(crate) suppress_shell_probes: bool,

    /// What `as_markdown` does on this request's expression surfaces. The
    /// pipeline installs [`NestedComposeSlot::Active`] per document; the
    /// shell-command discovery pass sets [`NestedComposeSlot::Discover`] so
    /// preflight never composes nested content. Default: `Unavailable`.
    ///
    /// [`NestedComposeSlot::Active`]: crate::markdown::compose::nested::NestedComposeSlot::Active
    /// [`NestedComposeSlot::Discover`]: crate::markdown::compose::nested::NestedComposeSlot::Discover
    pub(crate) nested_compose: crate::markdown::compose::nested::NestedComposeSlot,

    /// The request's ICMP effect authority behind `ping` and `ping_under`
    /// (R6). It owns the request's denial-warning sink, its transport, and —
    /// during shell-command discovery — the planned-probe sink that lets
    /// preflight report an ICMP effect without sending a packet. The grants
    /// themselves are derived from `remote_read_config.allowed_hosts` each time
    /// the authority is projected, so `--allow-host` stays the single entry
    /// point. Default: no grants, which denies every target.
    pub(crate) icmp: crate::markdown::compose::icmp::IcmpAuthority,

    /// The request's refresh authority behind the lazy `current` root (R30).
    /// It owns the invocation's capability to observe one mutable fact now and
    /// the sink its fail-closed `PartialRuntimeCapture` diagnostics land in.
    /// [`ComposeOptions::new`] installs an anchored ambient refresh; a caller
    /// holding its own launch evidence installs one with
    /// [`with_current_provider`](Self::with_current_provider). Default: no
    /// capability, so every `current.<key>` read fails closed.
    pub(crate) current: crate::markdown::compose::context::CurrentAuthority,

    /// The runtime schema phase the compose-time verdict is judged at, when the
    /// caller is preparing a launch rather than validating an authored
    /// document. `None` keeps the unphased authoring verdict. Default: `None`.
    pub(crate) schema_phase: Option<crate::markdown::schemas::SchemaPhase>,

    // ── Deferred frontmatter keys (DM1) ────────────────────────────
    /// Top-level frontmatter keys deferred from every compose-time value
    /// resolution pass (`{{ }}` interpolation, whole-value expansion,
    /// `$(...)` shell expansion, and schema value interpolation).
    ///
    /// A deferred key survives in `effective_frontmatter` with its authored
    /// `{{ }}` / structure intact — its JSON type and shape are preserved,
    /// only resolution is skipped. The caller owns event-time interpolation
    /// of these subtrees. Default: empty (no behavior change for any caller).
    pub(crate) exclude_keys: std::collections::HashSet<String>,

    // ── Named-object string coercion (Sequence Plus) ───────────────
    /// Frontmatter keys whose OBJECT values render their `name` string field
    /// when interpolated in inline string context (`{{key}}`). Whole-value
    /// spans, dotted paths (`{{key.x}}`), and typed `get()` lookups are
    /// unaffected. Empty by default; Claudine sets `["state","previous","next"]`
    /// for sequence steps.
    pub(crate) name_coercion_keys: Vec<String>,

    // ── Link normalization ────────────────────────────────────────
    /// Variable names Link Normalization may write as a `{{VAR}}/…` anchor,
    /// in addition to those the request environment's
    /// `PORTABLE_ENV_VARIABLES` declares. There is no built-in set.
    pub(crate) portable_env: std::collections::BTreeSet<String>,
    /// Whether Link Normalization warns when a destination keeps its
    /// absolute fallback. On by default.
    pub(crate) absolute_fallback_warning: bool,

    // ── Pre-flight graph reuse ────────────────────────────────────
    /// Optional pre-computed preflight graph to seed block transclusion.
    ///
    /// When the caller has already collected a
    /// [`PreflightGraphNode`](super::super::preflight::PreflightGraphNode) via
    /// [`Markdown::compose_preflight`](crate::markdown::Markdown::compose_preflight),
    /// the transclusion engine can use it to skip its own directive parse and
    /// target-resolution passes for body `::file` / `::url` directives. This
    /// removes the duplicated discovery walk the v2 design calls for.
    ///
    /// Wrapped in `Arc` because `ComposeOptions` is cloned through recursive
    /// child pipelines and the graph may be large.
    pub(crate) preflight_graph: Option<Arc<PreflightGraphNode>>,

    // ── Shared remote-fetch runtime ───────────────────────────────
    /// Optional remote-fetch runtime shared across the pre-flight collection
    /// walk and the terminal compose pass.
    ///
    /// When a caller (e.g. the CLI) runs `compose_preflight` before
    /// `compose_with`, both stages otherwise build their own runtime and fetch
    /// each remote URL twice. The runtime is single-flight (keyed by URL), so
    /// sharing one instance collapses pre-flight + compose into a single network
    /// request per URL. `None` means each stage builds its own.
    pub(crate) remote_fetch: Option<RemoteFetchRuntime>,

    // ── File-reference launch-area anchor (diagnostic only) ────────
    /// The captured launch-area directory a top-level caller (Claudine) was
    /// invoked from.
    ///
    /// Per D2 the launch directory is a base for **top-level** references only;
    /// it is **not** a resolution fallback for references authored inside a
    /// nested document. Darkmatter's document-backed resolution is
    /// document-first then repository-relative and never consults this directory,
    /// so it is retained solely as a diagnostic facet (the `fallback_dir` field
    /// of a file-reference diagnostic) and as an authored `ComposeOptions`
    /// identity input. It participates in neither the resolution candidate order
    /// nor the compiled-validator resolution.
    ///
    /// [`expression_resolution_context`]: Self::expression_resolution_context
    /// [`frontmatter_resolution_context`]: Self::frontmatter_resolution_context
    pub(crate) file_ref_fallback_dir: Option<PathBuf>,
}

impl std::fmt::Debug for ComposeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComposeOptions")
            .field("enabled_operations", &self.enabled_operations)
            .field("fail_fast", &self.fail_fast)
            .field("source", &self.source)
            .field("external_state", &self.external_state)
            .field("set_overrides", &self.set_overrides)
            .field("data_overrides", &self.data_overrides)
            .field("inherited_origin", &self.inherited_origin)
            .field("caller_input_records", &self.caller_input_records)
            .field("max_transclusion_depth", &self.max_transclusion_depth)
            .field("allow_remote_transclusion", &self.allow_remote_transclusion)
            .field("allow_local_markdown", &self.allow_local_markdown)
            .field("allow_local_code", &self.allow_local_code)
            .field("code_fallback_language", &self.code_fallback_language)
            .field("ignore_invalid_references", &self.ignore_invalid_references)
            .field("resolve_repo_root", &self.resolve_repo_root)
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
            .field("incidental_newline_mode", &self.incidental_newline_mode)
            .field("fixed_width", &self.fixed_width)
            .field("indent_size", &self.indent_size)
            .field("cache_access_mode", &self.cache_access_mode)
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
            .field("portable_env", &self.portable_env)
            .field("absolute_fallback_warning", &self.absolute_fallback_warning)
            .field(
                "allow_invalid_frontmatter_assignment",
                &self.allow_invalid_frontmatter_assignment,
            )
            .field(
                "allow_reassigned_frontmatter_property",
                &self.allow_reassigned_frontmatter_property,
            )
            .field("context", &self.context)
            .field("remote_read_config", &self.remote_read_config)
            .field("exclude_keys", &self.exclude_keys)
            .field("name_coercion_keys", &self.name_coercion_keys)
            .field("file_ref_fallback_dir", &self.file_ref_fallback_dir)
            .finish()
    }
}

impl ComposeOptions {
    /// Creates new compose options with all operations enabled and captured context.
    ///
    /// Captures only the zero-discovery runtime context — date/time and the
    /// environment snapshot. A constructor has no document to consult, so
    /// eagerly probing Git, repository topology, working-tree changes,
    /// languages, documents, OS, hardware, and GPU would be speculative: it
    /// walks the whole working tree once per call (~2s inside this monorepo on
    /// Windows) for values most callers never read. Composition instead
    /// captures the groups the root document names before its first stage;
    /// a `ctx.*` key from any other group fails with
    /// [`ExpressionError::ContextNotCaptured`](crate::markdown::compose::expression::ExpressionError::ContextNotCaptured).
    ///
    /// When the document is already available, prefer
    /// [`for_document`](Self::for_document) — it captures exactly the groups
    /// the document names, which also folds them into the compose cache key,
    /// and fixes the request's repository observation at creation.
    /// [`ComposeContext::capture_for_dir`] remains available for callers that
    /// genuinely want the full snapshot of a directory.
    pub fn new() -> Self {
        Self::new_with_context(ComposeContext::capture_minimal())
            .with_context_authority(super::authority::ContextAuthority::DarkmatterOwned)
    }

    /// Creates the Darkmatter-owned request for `document`, anchored at
    /// `anchor` — the request boundary of decision D3.
    ///
    /// One call captures the runtime context the document names
    /// ([`ComposeContext::capture_for_document`]), lets Darkmatter grow that
    /// context for transcluded sources
    /// ([`DarkmatterOwned`](super::authority::ContextAuthority::DarkmatterOwned)),
    /// and fixes the request's repository observation: the eager `Repo`
    /// capture when the document names one, otherwise one discovery at
    /// `anchor`. Every later phase run with these options — reference
    /// validation, pre-flight, compose, every transcluded child, every
    /// `current.repo*` read — answers from that observation, so a repository
    /// change after this call is invisible to the request. `md compose`
    /// builds its request here.
    ///
    /// The discovery replaces, rather than adds to, the one the root pipeline
    /// entry would otherwise make for the file-resolution snapshot, which
    /// projects the request's observation instead.
    ///
    /// A caller holding its own launch evidence keeps
    /// [`new_with_context`](Self::new_with_context) and
    /// [`with_current_provider`](Self::with_current_provider) instead.
    pub fn for_document(anchor: &Path, document: &crate::markdown::Markdown) -> Self {
        let options = Self::new_with_context(ComposeContext::capture_for_document(anchor, document))
            .with_context_authority(super::authority::ContextAuthority::DarkmatterOwned);
        options.establish_request_repository();
        options
    }

    /// Installs a source's request-epoch context, keeping the authority.
    pub(crate) fn with_request_context(mut self, context: ComposeContext) -> Self {
        self.context = context;
        self
    }

    /// Sets who may grow this request's runtime context.
    ///
    /// [`new_with_context`](Self::new_with_context) freezes the supplied
    /// context. A caller whose context was itself captured from the host (for
    /// example [`ComposeContext::capture_for_document`]) and that accepts the
    /// same discovery for transcluded sources passes
    /// [`ContextAuthority::DarkmatterOwned`](super::authority::ContextAuthority::DarkmatterOwned);
    /// a caller holding its own retained evidence passes
    /// [`ContextAuthority::CallerExtended`](super::authority::ContextAuthority::CallerExtended).
    pub fn with_context_authority(mut self, authority: super::authority::ContextAuthority) -> Self {
        self.context_authority = authority;
        self
    }

    /// Who may grow this request's runtime context.
    pub fn context_authority(&self) -> &super::authority::ContextAuthority {
        &self.context_authority
    }

    /// Extends the context with the groups `document` names, when the context
    /// authority permits.
    ///
    /// A document that names only groups already captured, or options whose
    /// context is frozen, pay nothing and keep what they have.
    pub(crate) fn extend_context_for(&mut self, document: &crate::markdown::Markdown) {
        if !self.context_authority.is_extendable() {
            return;
        }
        // The first document composed under a growable context is the
        // request's root, unless the caller captured the context for one.
        self.context.attach_root_document(document);
        let requirements = super::capture::ContextRequirements::for_document(document);
        self.context_authority.extend(&mut self.context, &requirements);
    }

    /// Whether [`extend_context_for`](Self::extend_context_for) would capture
    /// a group `document` names that these options lack.
    pub(crate) fn needs_extension_for(&self, document: &crate::markdown::Markdown) -> bool {
        let requirements = super::capture::ContextRequirements::for_document(document);
        self.context_authority.is_extendable()
            && self
                .context
                .missing_requirements(&requirements)
                .iter()
                .next()
                .is_some()
    }

    /// Fixes the request's repository observation (decision D3).
    ///
    /// Only a [`DarkmatterOwned`] request without an embedder-supplied
    /// refresh provider observes the repository itself; see
    /// [`CurrentAuthority::establish_ambient_repository`]. It runs when a
    /// request is created ([`for_document`](Self::for_document) and
    /// [`ComposeRequest`](super::request::ComposeRequest)) and is a no-op once
    /// established. A child pipeline never calls it: a descendant must find
    /// the observation already fixed, never establish it late.
    ///
    /// [`CurrentAuthority::establish_ambient_repository`]: crate::markdown::compose::context::CurrentAuthority::establish_ambient_repository
    /// [`DarkmatterOwned`]: super::authority::ContextAuthority::DarkmatterOwned
    pub(super) fn establish_request_repository(&self) {
        if !self.observes_its_own_repository() {
            return;
        }
        self.current.establish_ambient_repository(&self.context);
    }

    fn observes_its_own_repository(&self) -> bool {
        !self.current.has_provider()
            && matches!(self.context_authority, super::authority::ContextAuthority::DarkmatterOwned)
    }

    /// The request's one repository observation, when it holds one: the
    /// ambient observation established for `current.*`, else the eager
    /// `Repo` capture of the context.
    fn request_repository(&self) -> Option<&std::sync::Arc<super::repository_scope::RepositoryObservation>> {
        self.current
            .ambient_repository()
            .and_then(|repository| repository.observation())
            .or_else(|| self.context.observations().repository())
    }

    /// The request's repository observation when it found a repository that
    /// contains `dir`, so the context builder can project it instead of
    /// discovering the repository a second time.
    pub(super) fn request_repository_containing(
        &self,
        dir: &Path,
    ) -> Option<std::sync::Arc<super::repository_scope::RepositoryObservation>> {
        self.request_repository()
            .filter(|repository| repository.contains(dir))
            .cloned()
    }

    /// Makes the context builder's discovery at `dir` the request's
    /// repository observation, when this request observes its own repository
    /// from that same directory and has not yet established one.
    pub(super) fn adopt_request_repository(
        &self,
        dir: &Path,
        discovery: super::request::RepositoryDiscovery,
    ) {
        if self.observes_its_own_repository()
            && self.context.anchor() == dir
            && !self.context.capture_requirements().contains(super::capture::ContextGroup::Repo)
        {
            self.current.adopt_ambient_repository(discovery.values, discovery.observation);
        }
    }

    /// Re-anchors a Darkmatter-owned context on the request directory when it
    /// has captured nothing that depends on its anchor.
    ///
    /// [`new`](Self::new) captures only date and time, so its anchor is a
    /// placeholder; a later capture (`ctx.cwd`, the `Repo` group) then
    /// describes the request directory rather than the process directory.
    pub(super) fn anchor_unanchored_context(&mut self, dir: &Path) {
        if matches!(self.context_authority, super::authority::ContextAuthority::DarkmatterOwned)
            && self.context.anchor() != dir
            && self
                .context
                .capture_requirements()
                .iter()
                .all(|group| group == super::capture::ContextGroup::DateTime)
        {
            self.context = self.context.clone().with_anchor(dir);
        }
    }

    /// Makes `ctx.env` (and the `AGENT`/`MODEL` values derived from it) the
    /// request context's environment.
    pub(super) fn align_context_environment(&mut self, env: &std::collections::HashMap<String, String>) {
        if self.context.env() != env {
            *self.context.env_mut() = env.clone();
        }
    }

    /// Creates new compose options using a pre-captured context.
    ///
    /// Use this when you have already captured a `ComposeContext` (e.g.,
    /// via `ComposeContext::capture_for_content`) and want to avoid
    /// the cost of a redundant capture. The context authority starts as
    /// caller-supplied; a request Darkmatter should own for a document it
    /// has in hand is [`for_document`](Self::for_document), which also fixes
    /// the repository observation at creation rather than at the root
    /// pipeline entry.
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
            data_overrides: None,
            inherited_origin: InheritedOrigin::default(),
            caller_input_records: CallerInputRecords::new(),
            caller_file_provenance: std::collections::HashMap::new(),
            max_transclusion_depth: 16,
            allow_remote_transclusion: false,
            allow_local_markdown: true,
            allow_local_code: true,
            code_fallback_language: "txt".to_string(),
            ignore_invalid_references: None,
            resolve_repo_root: true,
            source_derivation: SourceDerivation::Ordinary,
            source_opening: None,
            shell_timeout: std::time::Duration::from_secs(10),
            shell_timeout_behavior: ShellTimeoutBehavior::Error,
            shell_policy_root: None,
            shell_working_directory: None,
            shell_approval_handler: None,
            pre_approved_commands: None,
            list_spacing: crate::markdown::cleanup::ListSpacingMode::Normal,
            incidental_newline_mode: crate::markdown::cleanup::IncidentalNewlineMode::Strip,
            fixed_width: None,
            indent_size: crate::markdown::cleanup::DEFAULT_INDENT,
            cache_access_mode: CacheAccessMode::default(),
            cache_root: None,
            cache_namespace: None,
            perf_enabled: false,
            context,
            context_authority: super::authority::ContextAuthority::CallerSupplied,
            replace_parent_wins: false,
            one_off_replace: None,
            interpolate_code_blocks: false,
            shell_strip_ansi: true,
            portable_env: std::collections::BTreeSet::new(),
            absolute_fallback_warning: true,
            baseline_schema: None,
            baseline_is_darkmatter_default: false,
            trigger_schemas: false,
            remote_read_config: RemoteReadConfig::default(),
            defer_shell_pending_schema_problems: false,
            defer_schema_verdict: false,
            defer_missing_runtime_context: false,
            defer_expression_failures: false,
            suppress_shell_probes: false,
            nested_compose: Default::default(),
            icmp: Default::default(),
            current: Default::default(),
            schema_phase: None,
            preflight_graph: None,
            remote_fetch: None,
            exclude_keys: std::collections::HashSet::new(),
            name_coercion_keys: Vec::new(),
            file_ref_fallback_dir: None,
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

    /// Restricts composition to the effective frontmatter surface: frontmatter
    /// interpolation and frontmatter `$(...)` expansion, and nothing that reads
    /// the body.
    ///
    /// The result keeps every other option (caller inputs, exclusions, schema
    /// deferral, file resolution, pre-approved commands) and only narrows the
    /// operation set, intersecting with what is already enabled — a frontmatter
    /// operation the caller disabled stays disabled.
    ///
    /// The composed document's body is the unchanged input body: no
    /// transclusion is dereferenced, no body directive or `::block` condition is
    /// evaluated, and no remote transclusion is prefetched. Keys the caller
    /// excluded (for example a deferred lifecycle subtree) keep their `{{ }}`
    /// spans. Its up-front pre-approval check covers frontmatter commands only
    /// (see [`collect_frontmatter_shell_commands`]).
    ///
    /// It exists so a staged run can read the frontmatter that drives an
    /// initialization step before that step creates the files the body
    /// includes; the projected content must never be used as a prompt.
    ///
    /// [`collect_frontmatter_shell_commands`]: crate::markdown::compose::preflight::collect::collect_frontmatter_shell_commands
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot};
    ///
    /// let md: Markdown = "---\nname: \"{{ 'x' }}\"\n---\n::file ./missing.md\n".into();
    /// let options = ComposeOptions::new().only_frontmatter_surface();
    /// assert!(options.is_frontmatter_surface_only());
    ///
    /// let request = ComposeRequest::prepare(options, &RequestSnapshot::new(std::env::temp_dir())).unwrap();
    /// let (projected, _report) = md.compose_with(&request).unwrap();
    /// assert_eq!(projected.frontmatter().as_map()["name"], "x");
    /// assert!(projected.content().contains("::file ./missing.md"));
    /// ```
    #[must_use]
    pub fn only_frontmatter_surface(mut self) -> Self {
        self.enabled_operations = FRONTMATTER_SURFACE_OPERATIONS
            .iter()
            .copied()
            .filter(|op| self.enabled_operations.contains(*op))
            .collect();
        self
    }

    /// Returns true when no enabled operation reads the document body.
    pub fn is_frontmatter_surface_only(&self) -> bool {
        self.enabled_operations
            .iter()
            .all(|op| FRONTMATTER_SURFACE_OPERATIONS.contains(&op))
    }

    /// Sets the compose source as a file path.
    #[must_use]
    pub fn with_source_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.source = ComposeSource::File(path.into());
        self.source_derivation = SourceDerivation::Ordinary;
        self.source_opening = None;
        self
    }

    /// Sets a child source that has already passed file-reference resolution.
    ///
    /// `opening` is the reference that resolved to `path` and the path it
    /// resolved to, when the caller has them; the child's context is then
    /// derived from it (see [`SourceOpening`]).
    /// A source outside the request's tree that only a trusted-external
    /// derivation of `context` admits is marked `TrustedExternal`.
    #[must_use]
    pub(crate) fn with_accepted_source_file_in(
        mut self,
        path: impl Into<PathBuf>,
        opening: Option<SourceOpening>,
        context: &biscuit_file::FileResolutionContext,
    ) -> Self {
        let path = path.into();
        let derive = |derivation| source_file_context(context, &path, derivation, opening.as_ref());
        self.source_derivation = if derive(SourceDerivation::Ordinary).validate().is_err()
            && derive(SourceDerivation::TrustedExternal).validate().is_ok()
        {
            SourceDerivation::TrustedExternal
        } else {
            SourceDerivation::Ordinary
        };
        self.source = ComposeSource::File(path);
        self.source_opening = opening;
        self
    }

    /// Sets the compose source as a URL.
    #[must_use]
    pub fn with_source_url(mut self, url: Url) -> Self {
        self.source = ComposeSource::Url(url);
        self.source_opening = None;
        self
    }

    /// Sets the external state for interpolation/replacement.
    #[must_use]
    pub fn with_external_state(mut self, state: serde_json::Value) -> Self {
        self.external_state = Some(state);
        self
    }

    /// Sets override values that overwrite existing frontmatter keys.
    ///
    /// These values are **authored**: a person wrote them, so they are
    /// templates exactly like the document's own frontmatter. Use
    /// [`with_data_overrides`](Self::with_data_overrides) for values an
    /// operation produced.
    #[must_use]
    pub fn with_set_overrides(mut self, overrides: serde_json::Value) -> Self {
        self.set_overrides = Some(overrides);
        self
    }

    /// Sets override values that overwrite existing frontmatter keys as
    /// **data**.
    ///
    /// A data value is inserted verbatim and never scanned: its `{{ … }}` is
    /// not evaluated, its `{{{ … }}}` is not converted, and a whole-value
    /// `$( … )` is not a shell command. Data overrides apply after
    /// [`with_set_overrides`](Self::with_set_overrides), so a key present in
    /// both is data.
    #[must_use]
    pub fn with_data_overrides(mut self, overrides: serde_json::Value) -> Self {
        self.data_overrides = Some(overrides);
        self
    }

    /// Sets top-level overrides from ordered, origin-tagged layers.
    ///
    /// A later layer's key replaces an earlier layer's key, and each key takes
    /// the origin of the layer that supplied it. Replaces any overrides set
    /// earlier by [`with_set_overrides`](Self::with_set_overrides) or
    /// [`with_data_overrides`](Self::with_data_overrides).
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::Markdown;
    /// use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, OverrideLayer, RequestSnapshot};
    /// use serde_json::json;
    ///
    /// let md: Markdown = "---\ntitle: t\n---\n{{ user }} / {{ output }}\n".into();
    /// let options = ComposeOptions::new().with_override_layers([
    ///     OverrideLayer::authored(json!({ "user": "{{ title }}" })),
    ///     OverrideLayer::data(json!({ "output": "{{ title }}" })),
    /// ]);
    /// let request = ComposeRequest::prepare(options, &RequestSnapshot::new(std::env::temp_dir())).unwrap();
    /// let (composed, _) = md.compose_with(&request).unwrap();
    /// assert_eq!(composed.content().trim(), "t / {{ title }}");
    /// ```
    #[must_use]
    pub fn with_override_layers(
        mut self,
        layers: impl IntoIterator<Item = super::super::value_origin::OverrideLayer>,
    ) -> Self {
        use super::super::value_origin::OverrideOrigin;
        let mut authored = serde_json::Map::new();
        let mut data = serde_json::Map::new();
        for layer in layers {
            let Some(values) = layer.values.as_object() else {
                continue;
            };
            for (key, value) in values {
                authored.remove(key);
                data.remove(key);
                match layer.origin {
                    OverrideOrigin::Authored => authored.insert(key.clone(), value.clone()),
                    OverrideOrigin::Data => data.insert(key.clone(), value.clone()),
                };
            }
        }
        self.set_overrides = Some(serde_json::Value::Object(authored));
        self.data_overrides = Some(serde_json::Value::Object(data));
        self
    }

    /// Clears the caller overrides that target the root document, for a
    /// compose of other content (`as_markdown`) under these options.
    pub(crate) fn clear_root_overrides(&mut self) {
        self.set_overrides = None;
        self.data_overrides = None;
        self.inherited_origin = InheritedOrigin::default();
    }

    /// Installs immutable raw caller values with their per-property origins.
    #[must_use]
    pub fn with_caller_input_records(mut self, records: CallerInputRecords) -> Self {
        self.caller_input_records = records;
        self
    }

    /// Immutable caller values retained independently from effective overrides.
    pub fn caller_input_records(&self) -> &CallerInputRecords {
        &self.caller_input_records
    }

    /// Sets the list spacing mode for the cleanup stage.
    #[must_use]
    pub fn with_list_spacing(mut self, mode: crate::markdown::cleanup::ListSpacingMode) -> Self {
        self.list_spacing = mode;
        self
    }

    /// Sets whether cleanup collapses incidental single newlines in prose.
    #[must_use]
    pub fn with_incidental_newline_mode(
        mut self,
        mode: crate::markdown::cleanup::IncidentalNewlineMode,
    ) -> Self {
        self.incidental_newline_mode = mode;
        self
    }

    /// Sets the fixed column width for cleanup reflow.
    #[must_use]
    pub fn with_fixed_width(mut self, width: usize) -> Self {
        self.fixed_width = Some(width.max(1));
        self
    }

    /// Sets the indentation width for nested list cleanup.
    #[must_use]
    pub fn with_indent_size(mut self, size: usize) -> Self {
        self.indent_size = size.max(1);
        self
    }

    /// Sets the run-local cache access mode. It governs in-memory reuse within
    /// one compose run only; nothing it selects is written to disk.
    #[must_use]
    pub fn with_cache_access_mode(mut self, mode: CacheAccessMode) -> Self {
        self.cache_access_mode = mode;
        self
    }

    /// Sets the root of the remote transport cache.
    ///
    /// Raw remote URL response bodies (transport artifacts) are the only
    /// class it can persist. Semantic results — composed documents, `::file`
    /// children, `::code` / `::toc-linking` results, and document snapshots —
    /// are never persisted until a `ContentPolicy` exists.
    ///
    /// ## Notes
    ///
    /// - Setting a root touches nothing on disk; directories appear only when
    ///   a storable remote response is written.
    /// - `Cache-Control` outranks [`RemoteReadConfig`] freshness settings: a
    ///   `no-store` response is never written and a `no-cache` one is
    ///   revalidated before every reuse.
    /// - A cache root never authorizes a host; the host policy is checked
    ///   before any cache read.
    #[must_use]
    pub fn with_cache_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.cache_root = Some(root.into());
        self
    }

    /// Sets the namespace appended to the remote transport-cache root.
    #[must_use]
    pub fn with_cache_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.cache_namespace = Some(namespace.into());
        self
    }

    /// Sets fail-fast mode for recoverable stages.
    ///
    /// With `false` (the default), failures in recoverable stages such as TOC
    /// linking and non-structural transclusion become warnings. It does not
    /// cover expressions: a body or frontmatter `{{ … }}` that cannot be parsed
    /// or evaluated fails composition whatever this is set to.
    #[must_use]
    pub fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }

    /// The expression-failure policy for this request's body and frontmatter
    /// interpolation: strict, except in the shell-command discovery pass.
    pub(crate) fn expression_failure_policy(
        &self,
    ) -> crate::markdown::compose::interpolation::ExpressionFailurePolicy {
        use crate::markdown::compose::interpolation::ExpressionFailurePolicy;
        if self.defer_expression_failures {
            ExpressionFailurePolicy::Lenient
        } else {
            ExpressionFailurePolicy::Strict
        }
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

    /// Declares environment variables whose value Link Normalization may
    /// write as a `{{VAR}}/…` anchor, forwarded to
    /// [`biscuit_file::PortablePath::with_portable_env`].
    ///
    /// Names accumulate across calls and are deduplicated; they join the names
    /// the request environment's `PORTABLE_ENV_VARIABLES` declares. Values
    /// always come from the request's captured environment. A name that is
    /// not a valid `{{VAR}}` name is skipped and reported as a warning.
    #[must_use]
    pub fn with_portable_env<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.portable_env.extend(names.into_iter().map(Into::into));
        self
    }

    /// The names declared through [`with_portable_env`](Self::with_portable_env).
    pub fn portable_env(&self) -> &std::collections::BTreeSet<String> {
        &self.portable_env
    }

    /// Controls the warning Link Normalization reports when no portable
    /// reference reaches a destination and it keeps its absolute path.
    ///
    /// On by default: such a destination leaves the composed document with a
    /// link tied to this host. Pass `false` when host-specific links are
    /// expected; the destination is kept either way.
    #[must_use]
    pub fn with_absolute_fallback_warning(mut self, enabled: bool) -> Self {
        self.absolute_fallback_warning = enabled;
        self
    }

    /// Whether Link Normalization warns about an absolute fallback; see
    /// [`with_absolute_fallback_warning`](Self::with_absolute_fallback_warning).
    pub fn absolute_fallback_warning(&self) -> bool {
        self.absolute_fallback_warning
    }

    /// Sets shell expansion options from a `ShellExpansionOptions` struct.
    #[must_use]
    pub fn with_shell(mut self, shell: super::super::shell_expansion::ShellExpansionOptions) -> Self {
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
        handler: std::sync::Arc<dyn super::super::shell_expansion::ShellApprovalHandler>,
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

    /// The document these options compose.
    pub fn source(&self) -> &ComposeSource {
        &self.source
    }

    /// Sets the fallback language for code transclusion.
    #[must_use]
    pub fn with_code_fallback_language(mut self, language: impl Into<String>) -> Self {
        self.code_fallback_language = language.into();
        self
    }

    /// Returns a `TransclusionOptions` view of the transclusion-related fields,
    /// resolving through the request's `context`.
    ///
    /// Used internally to pass transclusion config to resolver and TOC linking
    /// functions without coupling them to the full `ComposeOptions` type.
    pub(crate) fn transclusion_options_in(
        &self,
        context: &biscuit_file::FileResolutionContext,
    ) -> TransclusionOptions {
        TransclusionOptions {
            source: self.source.clone(),
            max_depth: self.max_transclusion_depth,
            allow_remote: self.allow_remote_transclusion,
            allow_local_markdown: self.allow_local_markdown,
            allow_local_code_text: self.allow_local_code,
            code_fallback_language: self.code_fallback_language.clone(),
            ignore_invalid: self.ignore_invalid_references,
            resolve_repo_root: self.resolve_repo_root,
            file_resolution_context: context.clone(),
            source_derivation: self.source_derivation,
            source_opening: self.source_opening.clone(),
        }
    }

    /// The current source's context, derived from the request's `context` by
    /// [`source_file_context`].
    ///
    /// A source with no path (a string, stdin, or URL) has no directory of its
    /// own, so it uses the request's context unchanged: its `cwd` is the
    /// request's. Deriving it to `.` would make it depend on the process
    /// directory and put it outside the request's tree.
    pub(crate) fn source_file_resolution_context_in(
        &self,
        context: &biscuit_file::FileResolutionContext,
    ) -> biscuit_file::FileResolutionContext {
        match &self.source {
            ComposeSource::File(path) => {
                source_file_context(context, path, self.source_derivation, self.source_opening.as_ref())
            }
            _ => context.clone(),
        }
    }

    /// Builds the [`ResolutionContext`] used by read-side expression functions
    /// during interpolation.
    ///
    /// Carries the document's directory (so relative/`@` references
    /// resolve where the source lives), the configured magic search paths, and
    /// — only when remote reads are enabled — the run's remote-fetch runtime so
    /// HTTP(S) URL arguments read from the fetch cache rather than disk.
    ///
    /// [`ResolutionContext`]: super::super::expression::ResolutionContext
    pub(crate) fn expression_resolution_context_in(
        &self,
        request_context: &biscuit_file::FileResolutionContext,
        remote_fetch: &super::super::remote_fetch::RemoteFetchRuntime,
    ) -> super::super::expression::ResolutionContext {
        // The derived context decides `cwd`, so expression helpers that compare
        // it with their own `cwd` see the same spelling.
        let mut context = super::super::expression::ResolutionContext::new(
            self.source_file_resolution_context_in(request_context),
        );
        context.file_ref_fallback_dir = self.file_ref_fallback_dir.clone();
        context.remote_fetch = self.remote_reads_enabled().then(|| remote_fetch.clone());
        context.ctx_values = self.context_values_for_resolution();
        context.observations = self.context.observations().clone();
        if !self.suppress_shell_probes {
            context.shell_probe =
                crate::markdown::compose::shell_expansion::probe::ShellProbe::from_environment(self.context.env());
        }
        context.icmp = self.icmp_authority();
        context.current = self.current_authority();
        context.caller_file_provenance = self.caller_file_provenance.clone();
        context.nested_compose = self.nested_compose.clone();
        context
    }

    /// Builds the [`ResolutionContext`] used by frontmatter interpolation and
    /// `$()` evaluation. Authorized remote reads share the compose run's same
    /// single-flight runtime as body interpolation.
    ///
    /// [`expression_resolution_context`]: Self::expression_resolution_context
    /// [`ResolutionContext`]: super::super::expression::ResolutionContext
    pub(crate) fn frontmatter_resolution_context_in(
        &self,
        request_context: &biscuit_file::FileResolutionContext,
    ) -> super::super::expression::ResolutionContext {
        let mut context = self.local_expression_resolution_context_in(request_context);
        context.remote_fetch = self.remote_reads_enabled().then(|| self.remote_fetch_runtime());
        context.nested_compose = self.nested_compose.clone();
        context
    }

    /// Builds the local-only expression context for the configured source.
    ///
    /// Hosts that evaluate document-authored expressions outside the main
    /// compose pipeline use this adapter so they retain the same immutable
    /// file-resolution snapshot, source derivation, and magic roots.
    pub(crate) fn local_expression_resolution_context_in(
        &self,
        request_context: &biscuit_file::FileResolutionContext,
    ) -> super::super::expression::ResolutionContext {
        // The derived context decides `cwd`, so expression helpers that compare
        // it with their own `cwd` see the same spelling.
        let mut context = super::super::expression::ResolutionContext::new(
            self.source_file_resolution_context_in(request_context),
        );
        context.file_ref_fallback_dir = self.file_ref_fallback_dir.clone();
        context.ctx_values = self.context_values_for_resolution();
        context.observations = self.context.observations().clone();
        if !self.suppress_shell_probes {
            context.shell_probe =
                crate::markdown::compose::shell_expansion::probe::ShellProbe::from_environment(self.context.env());
        }
        context.icmp = self.icmp_authority();
        context.current = self.current_authority();
        context.caller_file_provenance = self.caller_file_provenance.clone();
        context
    }

    fn context_values_for_resolution(&self) -> serde_json::Map<String, serde_json::Value> {
        match self.context.as_object() {
            serde_json::Value::Object(values) => values,
            _ => serde_json::Map::new(),
        }
    }

    /// Returns a `ShellExpansionOptions` view of the shell-related fields.
    ///
    /// Used internally to pass shell config to executor and policy functions
    /// without coupling them to the full `ComposeOptions` type.
    pub(crate) fn shell_options(&self) -> super::super::shell_expansion::ShellExpansionOptions {
        super::super::shell_expansion::ShellExpansionOptions {
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
        // A caller-supplied baseline is not the cached Darkmatter default, even
        // if it happens to be structurally equal; clear the fast-path marker.
        self.baseline_is_darkmatter_default = false;
        self
    }

    /// Attaches the Darkmatter baseline frontmatter schema as the baseline.
    ///
    /// This is a convenience wrapper around
    /// [`with_baseline_schema`](Self::with_baseline_schema) that loads the
    /// authored schema from `darkmatter/docs/schemas/darkmatter.yaml`. When both
    /// the baseline and the document `$schema` declare the same property, the
    /// document side wins.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::compose::ComposeOptions;
    ///
    /// let options = ComposeOptions::new().with_darkmatter_baseline_schema();
    /// ```
    #[must_use]
    pub fn with_darkmatter_baseline_schema(self) -> Self {
        let mut opts = self.with_baseline_schema(crate::markdown::schemas::darkmatter_base_schema());
        // Mark the fast path: schema validation reuses the process-cached
        // compiled JSON Schema instead of re-converting per compose (F9).
        opts.baseline_is_darkmatter_default = true;
        opts
    }

    /// Enables repository-scoped trigger-schema discovery for file sources.
    ///
    /// This is opt-in for library hosts. The `md compose` CLI enables it by
    /// default; stdin, URL, and in-memory sources remain discovery-free.
    #[must_use]
    pub fn with_trigger_schemas(mut self, enabled: bool) -> Self {
        self.trigger_schemas = enabled;
        self
    }

    /// Internal builder: toggles parent-wins behavior for the `replace` map.
    #[must_use]
    pub(crate) fn with_replace_parent_wins(mut self, enabled: bool) -> Self {
        self.replace_parent_wins = enabled;
        self
    }

    /// Sets the remote read configuration.
    #[must_use]
    pub fn with_remote_read_config(mut self, config: RemoteReadConfig) -> Self {
        self.remote_read_config = config;
        self
    }

    /// Whether the remote-fetch capability is enabled for this run.
    ///
    /// Read-side expression URL reads (`markdown_title(url)`, `frontmatter(url)`,
    /// …) are a separate capability from `::file`/`::code` block transclusion: a
    /// caller that configures an allowed host via [`with_remote_read_config`]
    /// enables them without also opting into remote transclusion. Block
    /// transclusion keeps its own explicit [`with_allow_remote_transclusion`]
    /// gate. An empty allowlist with the transclusion flag unset means remote
    /// reads stay fully disabled, preserving the deny-all default.
    ///
    /// [`with_remote_read_config`]: Self::with_remote_read_config
    /// [`with_allow_remote_transclusion`]: Self::with_allow_remote_transclusion
    pub(crate) fn remote_reads_enabled(&self) -> bool {
        self.allow_remote_transclusion || self.remote_read_config.http_hosts().next().is_some()
    }

    /// The request's ICMP authority with its grants derived from the
    /// `--allow-host` allowlist.
    ///
    /// Sharing one entry point with HTTP is deliberate (R6), but the two
    /// policies stay disjoint: a CIDR entry grants ICMP and nothing else, and a
    /// hostname or wildcard grants HTTP and nothing else.
    pub(crate) fn icmp_authority(&self) -> crate::markdown::compose::icmp::IcmpAuthority {
        self.icmp.with_grants(&self.remote_read_config.allowed_hosts)
    }

    /// Installs the invocation's refresh capability for the lazy `current` root.
    ///
    /// A caller holding its own launch evidence — Claudine's invocation
    /// authority, or a test's scripted provider — passes one here so
    /// `current.<key>` observes that evidence instead of the host. Without one,
    /// a caller-supplied request fails closed and a Darkmatter-owned request
    /// refreshes at the context's retained anchor.
    #[must_use]
    pub fn with_current_provider(
        mut self,
        provider: std::sync::Arc<dyn crate::markdown::compose::context::CurrentProvider>,
    ) -> Self {
        self.current = self.current.with_provider(provider);
        self
    }

    /// The request's effective refresh authority for the lazy `current` root.
    ///
    /// An explicitly installed provider always wins. Otherwise a
    /// [`DarkmatterOwned`] request — the one authority that already permits
    /// host discovery at the retained anchor — gets the anchored ambient
    /// refresh, and a caller-supplied or caller-extended request gets nothing,
    /// so its unsupplied capabilities fail closed (decision D5).
    ///
    /// [`DarkmatterOwned`]: super::authority::ContextAuthority::DarkmatterOwned
    pub(crate) fn current_authority(&self) -> crate::markdown::compose::context::CurrentAuthority {
        if self.current.has_provider() {
            return self.current.clone();
        }
        match self.context_authority {
            super::authority::ContextAuthority::DarkmatterOwned => {
                self.current.with_ambient_refresh(&self.context)
            }
            _ => self.current.clone(),
        }
    }

    /// Adds a single allowed host for remote URL reads.
    ///
    /// Convenience wrapper that appends to the existing allowlist.
    #[must_use]
    pub fn with_allowed_host(mut self, host: impl Into<String>) -> Self {
        self.remote_read_config.allowed_hosts.push(host.into());
        self
    }

    /// Sets the maximum number of concurrent remote fetches.
    #[must_use]
    pub fn with_remote_concurrency(mut self, cap: usize) -> Self {
        self.remote_read_config.remote_concurrency = cap.max(1);
        self
    }

    /// Sets the remote artifact TTL override.
    ///
    /// When `None` (default), server-provided cache headers are used.
    #[must_use]
    pub fn with_remote_ttl(mut self, ttl: Option<Duration>) -> Self {
        self.remote_read_config.remote_ttl = ttl;
        self
    }

    /// Forces revalidation of remote artifacts even when cached content
    /// is otherwise fresh.
    #[must_use]
    pub fn with_remote_refresh(mut self, refresh: bool) -> Self {
        self.remote_read_config.refresh = refresh;
        self
    }

    /// Sets the freshness mode for remote cache artifacts.
    #[must_use]
    pub fn with_remote_freshness_mode(mut self, mode: RemoteFreshnessMode) -> Self {
        self.remote_read_config.freshness_mode = mode;
        self
    }

    /// Returns a reference to the remote read configuration.
    pub fn remote_read_config(&self) -> &RemoteReadConfig {
        &self.remote_read_config
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
    /// and compose, avoiding redundant capture work. The supplied context is
    /// frozen, as with [`new_with_context`](Self::new_with_context); call
    /// [`with_context_authority`](Self::with_context_authority) afterwards to
    /// let composition grow it.
    pub fn with_context(mut self, context: ComposeContext) -> Self {
        self.context = context;
        self.context_authority = super::authority::ContextAuthority::CallerSupplied;
        self
    }

    /// Enables or disables performance metric collection.
    #[must_use]
    pub fn with_perf(mut self, enabled: bool) -> Self {
        self.perf_enabled = enabled;
        self
    }

    /// Attaches a pre-computed preflight graph to seed block transclusion.
    ///
    /// When set, the transclusion engine reuses the graph's
    /// [`PreflightGraphEdge`](super::super::preflight::PreflightGraphEdge)
    /// resolved targets as a resolution cache, skipping a second
    /// target-resolution pass for `::file` / `::url` directives the preflight
    /// walk already resolved. Directives are still parsed from the current
    /// content so replacement spans are never reused from the preflight walk
    /// (which runs before frontmatter shell expansion and other offset-shifting
    /// stages). The typical flow is:
    ///
    /// 1. Call `Markdown::compose_preflight(&request)` to collect the
    ///    approval set and the graph.
    /// 2. Authorize the approval set with the caller's policy/prompt.
    /// 3. Compose with
    ///    `request.map_options(|o| o.with_pre_approved_commands(...).with_preflight_graph(report.preflight_graph))`.
    ///
    /// `None` (the default) preserves the legacy behavior: the transclusion
    /// engine parses directives and resolves targets itself.
    #[must_use]
    pub fn with_preflight_graph(mut self, graph: PreflightGraphNode) -> Self {
        self.preflight_graph = Some(Arc::new(graph));
        self
    }

    /// Returns a reference to the attached preflight graph, if any.
    pub fn preflight_graph(&self) -> Option<&PreflightGraphNode> {
        self.preflight_graph.as_deref()
    }

    /// Defers the named top-level frontmatter keys from every compose-time
    /// value-resolution pass (`{{ }}` interpolation, whole-value expansion,
    /// `$(...)` shell expansion, and schema value interpolation).
    ///
    /// A deferred key survives in `effective_frontmatter` with its authored
    /// `{{ }}` / structure intact, preserving its JSON type and shape. The
    /// caller owns event-time interpolation of deferred subtrees. A compose-time
    /// (non-deferred) key that *references* a deferred key is rejected during
    /// dependency analysis so a raw lifecycle subtree cannot leak into an
    /// early-bound value.
    ///
    /// Default: empty (no behavior change).
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::compose::ComposeOptions;
    ///
    /// let options = ComposeOptions::new()
    ///     .with_exclude_keys(["failure", "start"]);
    /// ```
    #[must_use]
    pub fn with_exclude_keys(
        mut self,
        keys: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.exclude_keys = keys.into_iter().map(|k| k.into()).collect();
        self
    }

    /// Returns the set of top-level frontmatter keys deferred from
    /// compose-time resolution.
    pub fn exclude_keys(&self) -> &std::collections::HashSet<String> {
        &self.exclude_keys
    }

    /// Sets the frontmatter keys whose OBJECT values render their `name` string
    /// field when interpolated in inline string context (`{{key}}`).
    ///
    /// Whole-value spans, dotted paths (`{{key.x}}`), and typed `get()` lookups
    /// are unaffected. Empty by default; Claudine sets
    /// `["state","previous","next"]` for sequence steps.
    #[must_use]
    pub fn with_name_coercion_keys(mut self, keys: Vec<String>) -> Self {
        self.name_coercion_keys = keys;
        self
    }

    /// Composes without owning the document's schema verdict: frontmatter is
    /// still coerced to the declared types, but no schema problem is reported.
    ///
    /// Set it only when a later terminal pass composes the settled document and
    /// reports the verdict — a shell-discovery pass, or a read taken before an
    /// `initialize` stage that may still add or repair the offending property.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::compose::ComposeOptions;
    ///
    /// let options = ComposeOptions::new().with_deferred_schema_verdict(true);
    /// ```
    #[must_use]
    pub fn with_deferred_schema_verdict(mut self, deferred: bool) -> Self {
        self.defer_schema_verdict = deferred;
        self
    }

    /// Judges the compose-time schema verdict at a runtime phase instead of
    /// the unphased authoring contract.
    ///
    /// `Some(SchemaPhase::Launch)` lets a required-but-not-eager property stay
    /// absent (a later actor is expected to supply it) while still requiring
    /// every `required; eager` property and type-checking every present value.
    /// An eager-only property stays optional: `eager` decides when a supplied
    /// value is validated, not whether it must exist. `None` (the default)
    /// keeps the existing unphased verdict.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::compose::ComposeOptions;
    /// use darkmatter::markdown::schemas::SchemaPhase;
    ///
    /// let options = ComposeOptions::new().with_schema_phase(Some(SchemaPhase::Launch));
    /// ```
    #[must_use]
    pub fn with_schema_phase(
        mut self,
        phase: Option<crate::markdown::schemas::SchemaPhase>,
    ) -> Self {
        self.schema_phase = phase;
        self
    }

    /// The runtime schema phase the compose-time verdict is judged at, if any.
    pub fn schema_phase(&self) -> Option<crate::markdown::schemas::SchemaPhase> {
        self.schema_phase
    }

    /// Sets the explicit fallback directory for caller-supplied file references
    /// (typically the captured launch area).
    ///
    /// Propagated into both [`expression_resolution_context`] and
    /// [`frontmatter_resolution_context`] as
    /// [`ResolutionContext::file_ref_fallback_dir`], and into the
    /// [`DarkmatterSchemas`] builder used by the compose-stage schema
    /// validation. Resolution still tries `base_dir` (the document directory)
    /// first; only when that misses does it consult the fallback.
    ///
    /// [`expression_resolution_context`]: Self::expression_resolution_context
    /// [`frontmatter_resolution_context`]: Self::frontmatter_resolution_context
    /// [`ResolutionContext::file_ref_fallback_dir`]: super::super::expression::ResolutionContext::file_ref_fallback_dir
    /// [`DarkmatterSchemas`]: crate::markdown::schemas::DarkmatterSchemas
    #[must_use]
    pub fn with_file_ref_fallback_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.file_ref_fallback_dir = Some(dir.into());
        self
    }

    /// Attaches a single remote-fetch runtime shared by `compose_preflight` and
    /// the subsequent `compose_with` pass.
    ///
    /// Callers that run pre-flight collection before composing (the CLI's
    /// approval lifecycle) should call this once so both stages fetch each
    /// remote URL exactly once instead of twice. The runtime is built with the
    /// same transport-cache resolution `compose_with` uses, honoring
    /// `cache_root` / `cache_namespace`.
    #[must_use]
    pub fn with_shared_remote_fetch(mut self) -> Self {
        self.remote_fetch = Some(self.build_remote_fetch_runtime());
        self
    }

    /// Returns the shared remote-fetch runtime when one is attached, otherwise
    /// builds a fresh one with the transport cache resolved from `cache_root`.
    pub(crate) fn remote_fetch_runtime(&self) -> RemoteFetchRuntime {
        self.remote_fetch
            .clone()
            .unwrap_or_else(|| self.build_remote_fetch_runtime())
    }

    /// Builds a remote-fetch runtime with the transport cache resolved from
    /// `cache_root` / `cache_namespace` (absent → network-only, no cross-run
    /// cache). Performs no filesystem I/O: the store creates its directories
    /// on the first remote body it writes.
    fn build_remote_fetch_runtime(&self) -> RemoteFetchRuntime {
        let remote_store = self.cache_root.as_ref().map(|root| {
            Arc::new(FileStore::at(FileStore::resolve_cache_root(
                root,
                self.cache_namespace.as_deref(),
            )))
        });
        RemoteFetchRuntime::with_store(&self.remote_read_config, remote_store)
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

impl serde::Serialize for ComposeSource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Unknown => serializer.serialize_none(),
            Self::File(path) => serializer.serialize_str(&path.display().to_string()),
            Self::Url(url) => serializer.serialize_str(url.as_str()),
        }
    }
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

    /// The request's context, inherited by every nested document.
    pub file_resolution_context: biscuit_file::FileResolutionContext,

    /// How the current file source entered the traversal.
    pub(crate) source_derivation: SourceDerivation,

    /// The reference that opened the current file source, if known.
    pub(crate) source_opening: Option<SourceOpening>,
}

/// Unit tests that never resolve a file anchor the context at the system
/// temporary directory.
#[cfg(test)]
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
            file_resolution_context: biscuit_file::FileResolutionContext::new(std::env::temp_dir()),
            source_derivation: SourceDerivation::Ordinary,
            source_opening: None,
        }
    }
}

// ── Shared options field-classification authority ──────────────────
//
// One crate-private, exhaustive classification of every `ComposeOptions`
// field. It destructures `ComposeOptions` **without `..`** so a newly added
// field is a compile error here until its identity treatment is chosen. Two
// purpose-specific products are derived from this single inventory:
//
//   * `ReferenceGraphOptionsIdentity::capture` — conservative, fail-closed
//     graph identity covering every field (value fingerprint + weak instance
//     handles for the three stateful `Arc`-backed fields).
//   * `ComposeOptions::compose_cache_fingerprint` — the compose-cache value
//     fingerprint over only the output-affecting subset. `options_hash`
//     delegates to it; there is no third parallel field inventory.
//
// The versioned domain marker prevents collisions with unrelated hashed
// content and lets the encoding evolve deliberately.
const OPTIONS_IDENTITY_DOMAIN: &str = "dm.compose-options.v2";

/// Versioned domain marker for the compose-cache options fingerprint.
///
/// Distinct from [`OPTIONS_IDENTITY_DOMAIN`] (the graph identity) so the two
/// products of the single classification never collide, and distinct from the
/// historical `options_hash` string-join encoding this replaced. Compose-cache
/// keys are run-local, so bumping the version invalidates nothing on disk.
const CACHE_OPTIONS_DOMAIN: &str = "dm.compose-cache-options.v1";

/// Versioned domain marker for the reference-graph's complete runtime-context
/// encoding. Kept distinct from the compose-cache `context_hash` product so
/// the two encodings can evolve independently.
const GRAPH_CONTEXT_DOMAIN: &str = "dm.compose-graph-context.v1";

/// Platform tags for [`GraphIdentityEncoder::path`]. Every path encode writes
/// exactly one of these before its length-prefixed payload, so a Unix byte
/// sequence and a Windows wide-unit sequence can never hash alike. The values
/// are stable wire bytes: change them only alongside a domain-marker bump.
#[cfg(unix)]
const PATH_ENCODING_UNIX_BYTES: u8 = 1;
#[cfg(windows)]
const PATH_ENCODING_WINDOWS_WIDE: u8 = 2;
#[cfg(not(any(unix, windows)))]
const PATH_ENCODING_LOSSY_FALLBACK: u8 = 3;

/// Complete fingerprint of the captured runtime context for the reference-graph
/// identity.
///
/// Unlike the compose-cache `context_hash` — which deliberately drops
/// volatile per-second fields (`now`, `timestamp`, ...) and system-state fields
/// (`memory_used`, `memory_avail`) so a stable document is not re-cached every
/// second — the graph identity must be complete and fail-closed: graph
/// construction interpolates `{{ ctx.* }}` into link/transclusion targets and
/// evaluates transclusion `when=` conditions against these same values, so two
/// contexts differing only in an otherwise-volatile value can yield different
/// references and must not share a graph identity.
///
/// Hashes every entry of the full normalized values map plus every environment
/// value; `canonical_json_sorted` sorts object keys recursively and env pairs
/// are sorted explicitly, so the fingerprint is deterministic. The context is
/// captured once and cloned into every `ComposeOptions::clone`, so it is also
/// clone-stable.
fn graph_context_fingerprint(ctx: &ComposeContext) -> u64 {
    use super::super::cache::hashing::canonical_json_sorted;
    use serde_json::Value;

    let values_canonical = canonical_json_sorted(&Value::Object(ctx.values().clone()));
    let mut parts = vec![GRAPH_CONTEXT_DOMAIN.to_string(), values_canonical];

    let mut env_pairs: Vec<_> = ctx.env().iter().collect();
    env_pairs.sort_by_key(|(k, _)| *k);
    for (k, v) in env_pairs {
        parts.push(format!("env.{k}={v}"));
    }

    xx_hash(&parts.join("\0"))
}

/// Unambiguous, length-prefixed canonical byte encoder for the reference-graph
/// options value fingerprint.
///
/// Every variable-length segment is written as an 8-byte little-endian length
/// prefix followed by its bytes, and every collection as an element count
/// followed by each element, so segment and element boundaries always survive.
/// The historical comma/NUL string join lost them, letting
/// `pre_approved_commands = {"a,b"}` and `{"a", "b"}` hash identically even
/// though the values are not equivalent. Enum discriminants are explicit stable
/// bytes here, never `Debug` output (which the spec prohibits as a canonical
/// encoding). The buffer is xxHashed via `biscuit-hash`.
/// Origin of the values a transcluded document receives from its parent.
///
/// A root compose leaves every field at its default: `--state` is authored,
/// and the document's frontmatter is its own.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct InheritedOrigin {
    /// Origin of [`ComposeOptions::external_state`]. A parent's composed
    /// values are data in its children.
    pub(crate) external_state: super::super::value_origin::OverrideOrigin,
    /// Origin of [`ComposeOptions::one_off_replace`].
    pub(crate) one_off_replace: super::super::value_origin::OverrideOrigin,
    /// Frontmatter leaves a directive `set` overlay wrote as data before this
    /// document was composed.
    pub(crate) frontmatter_data: super::super::value_origin::DataPaths,
}

impl InheritedOrigin {
    fn encode(&self, enc: &mut GraphIdentityEncoder) {
        use super::super::value_origin::{OverrideOrigin, ValuePathSegment};
        let tag = |origin: OverrideOrigin| match origin {
            OverrideOrigin::Authored => 0,
            OverrideOrigin::Data => 1,
        };
        enc.tag(tag(self.external_state));
        enc.tag(tag(self.one_off_replace));
        let paths = self.frontmatter_data.paths();
        enc.count(paths.len());
        for path in paths {
            enc.count(path.len());
            for segment in path {
                match segment {
                    ValuePathSegment::Key(key) => {
                        enc.tag(0);
                        enc.str(key);
                    }
                    ValuePathSegment::Index(index) => {
                        enc.tag(1);
                        enc.u64(*index as u64);
                    }
                }
            }
        }
    }
}

struct GraphIdentityEncoder {
    buf: Vec<u8>,
}

impl GraphIdentityEncoder {
    /// Starts a buffer seeded with the versioned domain marker.
    fn new(domain: &str) -> Self {
        let mut enc = Self { buf: Vec::new() };
        enc.segment(domain.as_bytes());
        enc
    }

    /// Writes a length-prefixed byte segment.
    fn segment(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(&(data.len() as u64).to_le_bytes());
        self.buf.extend_from_slice(data);
    }

    /// Writes a stable field label so two fields whose values encode to the
    /// same bytes stay distinct.
    fn field(&mut self, name: &str) {
        self.segment(name.as_bytes());
    }

    /// Writes a length-prefixed UTF-8 value.
    fn str(&mut self, value: &str) {
        self.segment(value.as_bytes());
    }

    /// Writes native path data exactly, with no UTF-8 presentation conversion.
    ///
    /// `Path::display()` must never reach an identity product: it is lossy. On
    /// Unix an `OsStr` may hold arbitrary non-UTF-8 bytes, and distinct invalid
    /// sequences all collapse to U+FFFD, so two paths that select different
    /// files would share a fingerprint — a false-match that lets prebuilt-graph
    /// validation accept a graph built from different resolution inputs, and
    /// lets the compose cache select an unrelated entry.
    ///
    /// The leading platform tag keeps the Unix byte encoding and the Windows
    /// wide-unit encoding in disjoint spaces, so the same fingerprint can never
    /// be produced by two different platform representations.
    fn path(&mut self, value: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            self.tag(PATH_ENCODING_UNIX_BYTES);
            self.segment(value.as_os_str().as_bytes());
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            self.tag(PATH_ENCODING_WINDOWS_WIDE);
            // Fixed-width little-endian u16 units: exact and self-delimiting
            // inside the length-prefixed segment.
            let mut units = Vec::new();
            for unit in value.as_os_str().encode_wide() {
                units.extend_from_slice(&unit.to_le_bytes());
            }
            self.segment(&units);
        }
        #[cfg(not(any(unix, windows)))]
        {
            // No target in this workspace lacks both `OsStrExt`s; this arm only
            // keeps the encoder compiling on exotic targets. It is the one
            // lossy representation, so it carries its own tag and can never be
            // mistaken for an exact one.
            self.tag(PATH_ENCODING_LOSSY_FALLBACK);
            self.segment(value.as_os_str().to_string_lossy().as_bytes());
        }
    }

    /// Writes an explicit stable enum discriminant byte.
    fn tag(&mut self, discriminant: u8) {
        self.buf.push(discriminant);
    }

    fn bool(&mut self, value: bool) {
        self.buf.push(u8::from(value));
    }

    fn u64(&mut self, value: u64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    fn u128(&mut self, value: u128) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Writes a collection element count; each element follows via `str`.
    fn count(&mut self, n: usize) {
        self.u64(n as u64);
    }

    fn finish(self) -> u64 {
        xx_hash_bytes(&self.buf)
    }
}

/// Result of the single exhaustive `ComposeOptions` field classification.
///
/// The two fingerprints and the weak handles are the raw material of the two
/// derived products: `ReferenceGraphOptionsIdentity::capture` reads
/// `graph_value_fingerprint` plus the weak handles; `compose_cache_fingerprint`
/// reads `cache_value_fingerprint`.
struct ComposeOptionsClassification {
    /// Canonical, versioned value fingerprint over every value-representable
    /// field — the conservative reference-graph identity input. Built by
    /// [`GraphIdentityEncoder`] so element and field boundaries are
    /// unambiguous.
    graph_value_fingerprint: u64,
    /// Typed, length-delimited fingerprint over the output-affecting field
    /// subset — the compose-cache identity input. Built by the same
    /// [`GraphIdentityEncoder`] under the distinct [`CACHE_OPTIONS_DOMAIN`]
    /// marker, so it carries no `Debug`-based encoding and never equals a value
    /// of the historical string-join `options_hash` (the domain and value both
    /// differ).
    cache_value_fingerprint: u64,
    /// Weak instance handle for the shell approval handler (stateful; not
    /// value-representable).
    shell_approval_handler: Option<Weak<dyn ShellApprovalHandler>>,
    /// Weak instance handle for the shared preflight graph.
    preflight_graph: Option<Weak<PreflightGraphNode>>,
    /// Weak instance handle for the shared remote-fetch runtime.
    remote_fetch: Option<RemoteFetchWeakId>,
}

/// Process cache for the Darkmatter default baseline schema's canonical JSON.
static DEFAULT_BASELINE_CANONICAL_JSON: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Canonical JSON encoding of a baseline schema, for identity fingerprinting.
///
/// `is_darkmatter_default` selects a process-cached encoding derived from the
/// process-cached compiled baseline (`schemas::darkmatter_base_json_schema_ref`)
/// rather than re-running `to_json_schema`, which dominates this whole
/// classification (~534 µs, vs ~74 µs to canonicalize and ~2 µs for every other
/// option field combined). Compose already populates that cache for the default
/// baseline via `schema_validation`'s F9 fast path, so the marked arm adds no
/// conversion at all.
///
/// The marker is only ever set by `with_darkmatter_baseline_schema`, which
/// assigns `schemas::darkmatter_base_schema()` in the same call;
/// `with_baseline_schema` clears it unconditionally. So the marker implies the
/// schema *is* the default, and the cached bytes are exactly what converting it
/// would produce — hence the same fingerprint.
///
/// ## Notes
///
/// A caller-supplied schema that happens to equal the default takes the uncached
/// arm and still fingerprints identically; the marker is a fast path, never a
/// semantic distinction.
fn baseline_canonical_json(
    schema: &crate::markdown::schemas::SimplifiedSchema,
    is_darkmatter_default: bool,
) -> std::borrow::Cow<'static, str> {
    use super::super::cache::hashing::canonical_json_sorted;

    if is_darkmatter_default {
        return std::borrow::Cow::Borrowed(DEFAULT_BASELINE_CANONICAL_JSON.get_or_init(|| {
            canonical_json_sorted(crate::markdown::schemas::darkmatter_base_json_schema_ref())
        }));
    }

    let json = crate::markdown::schemas::to_json_schema(schema)
        .unwrap_or_else(|_| serde_json::json!({"baseline_schema_error": true}));
    std::borrow::Cow::Owned(canonical_json_sorted(&json))
}

/// Encodes every file-resolution input that can change candidate construction.
///
/// Besides the authoring anchors, this covers the three inputs that can change
/// the winning `@` candidate without touching any other field: `package_root`
/// (an intrinsic local-tier root), the captured launch `@` scope (which anchors
/// the whole `@` chain independently of the source anchors), and each
/// configured magic root's tier policy (a `User` override reorders the chain
/// even when the root path is unchanged).
fn encode_file_resolution_context(
    enc: &mut GraphIdentityEncoder,
    context: &biscuit_file::FileResolutionContext,
) {
    for path in [
        context.source_path(),
        Some(context.cwd()),
        Some(context.request_cwd()),
        context.repository_root(),
        context.package_root(),
        context.package_area(),
        context.home_dir(),
    ] {
        match path {
            Some(path) => {
                enc.tag(1);
                enc.path(path);
            }
            None => enc.tag(0),
        }
    }
    enc.bool(context.is_trusted_external_authoring_cwd());

    let scope = context.launch_magic_scope();
    enc.field("launch_magic_scope");
    for path in [
        Some(scope.request_dir()),
        scope.repository_root(),
        scope.package_root(),
        scope.package_area(),
    ] {
        match path {
            Some(path) => {
                enc.tag(1);
                enc.path(path);
            }
            None => enc.tag(0),
        }
    }

    let mut env: Vec<(&str, &str)> = context
        .env()
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    env.sort_unstable();
    enc.count(env.len());
    for (key, value) in env {
        enc.str(key);
        enc.str(value);
    }

    // Tier-aware registrations: position alone no longer fixes where a root
    // sits in the chain, so the tier policy is part of the identity.
    enc.field("magic_path_registrations");
    let registrations = context.magic_path_registrations();
    enc.count(registrations.len());
    for registration in &registrations {
        enc.path(registration.path());
        enc.tag(match registration.position() {
            biscuit_file::PathPosition::Start => 0,
            biscuit_file::PathPosition::End => 1,
        });
        enc.tag(match registration.tier() {
            biscuit_file::MagicPathTier::Inferred => 0,
            biscuit_file::MagicPathTier::User => 1,
        });
    }

    enc.field("vault_roots");
    enc.count(context.vault_roots().len());
    for path in context.vault_roots() {
        enc.path(path);
    }

    // The tree root bounds relative references, so two snapshots that differ
    // only in it resolve differently.
    enc.field("base_dir");
    enc.path(context.base_dir());
    match context.base_dir_origin() {
        biscuit_file::BaseDirOrigin::Repository => enc.tag(0),
        biscuit_file::BaseDirOrigin::Explicit => enc.tag(1),
        biscuit_file::BaseDirOrigin::Vault => enc.tag(2),
        biscuit_file::BaseDirOrigin::Home => enc.tag(3),
        biscuit_file::BaseDirOrigin::Environment { name } => {
            enc.tag(4);
            enc.str(name);
        }
        biscuit_file::BaseDirOrigin::Fallback => enc.tag(5),
    }
    enc.bool(context.external_relative_allowed());
}

fn encode_source_opening(enc: &mut GraphIdentityEncoder, opening: &Option<SourceOpening>) {
    match opening {
        Some(opening) => {
            enc.tag(1);
            enc.str(opening.reference.raw());
            enc.path(&opening.resolved);
        }
        None => enc.tag(0),
    }
}

impl ComposeOptions {
    /// Exhaustively classifies every field into the two identity products.
    ///
    /// The no-`..` destructure is the compile-time maintenance guard: adding a
    /// field to `ComposeOptions` fails to compile here until its graph- and
    /// cache-identity treatment is chosen. The request's `file_resolution_context`
    /// is encoded beside the fields, since a request holds it outside the
    /// options.
    fn classify_options(
        &self,
        file_resolution_context: &biscuit_file::FileResolutionContext,
    ) -> ComposeOptionsClassification {
        let ComposeOptions {
            enabled_operations,
            fail_fast,
            allow_ctx_override,
            allow_invalid_frontmatter_assignment,
            allow_reassigned_frontmatter_property,
            source,
            external_state,
            set_overrides,
            data_overrides,
            inherited_origin,
            caller_input_records,
            caller_file_provenance,
            max_transclusion_depth,
            allow_remote_transclusion,
            allow_local_markdown,
            allow_local_code,
            code_fallback_language,
            ignore_invalid_references,
            resolve_repo_root,
            source_derivation,
            source_opening,
            shell_timeout,
            shell_timeout_behavior,
            shell_policy_root,
            shell_working_directory,
            shell_approval_handler,
            pre_approved_commands,
            shell_strip_ansi,
            list_spacing,
            incidental_newline_mode,
            fixed_width,
            indent_size,
            cache_access_mode,
            cache_root,
            cache_namespace,
            perf_enabled,
            context,
            context_authority,
            replace_parent_wins,
            one_off_replace,
            interpolate_code_blocks,
            baseline_schema,
            baseline_is_darkmatter_default,
            trigger_schemas,
            remote_read_config,
            defer_shell_pending_schema_problems,
            defer_schema_verdict,
            defer_missing_runtime_context,
            defer_expression_failures,
            suppress_shell_probes,
            nested_compose,
            icmp,
            current,
            schema_phase,
            exclude_keys,
            name_coercion_keys,
            portable_env,
            absolute_fallback_warning,
            preflight_graph,
            remote_fetch,
            file_ref_fallback_dir,
        } = self;

        use super::super::cache::hashing::canonical_json_sorted;
        use serde_json::Value;

        // Both identity domains below encode the baseline schema, and the
        // conversion-plus-canonicalization dominates this whole function
        // (~600 µs vs ~2 µs for every other field combined). Compute it once
        // here and lend it to both encoders rather than paying it per domain.
        let baseline_canonical: Option<std::borrow::Cow<'static, str>> = baseline_schema
            .as_ref()
            .map(|schema| baseline_canonical_json(schema, *baseline_is_darkmatter_default));

        // ── Graph identity: conservative, covers every value-representable
        //    field (stateful fields are handled by weak instance handles).
        //
        //    Encoded as a length-prefixed, tagged byte buffer so element and
        //    field boundaries are unambiguous and every enum contributes an
        //    explicit stable discriminant rather than `Debug` output.
        let mut enc = GraphIdentityEncoder::new(OPTIONS_IDENTITY_DOMAIN);

        // Bounded operation set: encode the canonical-order index of each
        // enabled operation (never the `Debug` name).
        enc.field("enabled_operations");
        let enabled_indices: Vec<usize> = enabled_operations.iter().map(|op| op.index()).collect();
        enc.count(enabled_indices.len());
        for index in enabled_indices {
            enc.u64(index as u64);
        }

        enc.field("fail_fast");
        enc.bool(*fail_fast);
        enc.field("allow_ctx_override");
        enc.bool(*allow_ctx_override);
        enc.field("allow_invalid_frontmatter_assignment");
        enc.bool(*allow_invalid_frontmatter_assignment);
        enc.field("allow_reassigned_frontmatter_property");
        enc.bool(*allow_reassigned_frontmatter_property);

        enc.field("source");
        match source {
            ComposeSource::Unknown => enc.tag(0),
            ComposeSource::File(p) => {
                enc.tag(1);
                enc.path(p);
            }
            ComposeSource::Url(u) => {
                enc.tag(2);
                enc.str(u.as_str());
            }
        }

        enc.field("external_state");
        match external_state {
            Some(v) => {
                enc.tag(1);
                enc.str(&canonical_json_sorted(v));
            }
            None => enc.tag(0),
        }
        enc.field("set_overrides");
        match set_overrides {
            Some(v) => {
                enc.tag(1);
                enc.str(&canonical_json_sorted(v));
            }
            None => enc.tag(0),
        }
        enc.field("data_overrides");
        match data_overrides {
            Some(v) => {
                enc.tag(1);
                enc.str(&canonical_json_sorted(v));
            }
            None => enc.tag(0),
        }
        enc.field("inherited_origin");
        inherited_origin.encode(&mut enc);
        enc.field("caller_input_records");
        enc.count(caller_input_records.len());
        for (property, record) in caller_input_records {
            enc.str(property);
            enc.str(&canonical_json_sorted(record.raw()));
            encode_file_resolution_context(&mut enc, record.origin());
        }
        enc.field("caller_file_provenance");
        let mut projected: Vec<_> = caller_file_provenance.iter().collect();
        projected.sort_by_key(|(occurrence, _)| *occurrence);
        enc.count(projected.len());
        for (occurrence, provenance) in projected {
            enc.str(occurrence);
            enc.str(&provenance.property);
            enc.str(&provenance.reference);
            encode_file_resolution_context(&mut enc, &provenance.origin);
            enc.path(&provenance.candidate);
            enc.tag(match provenance.candidate_provenance {
                biscuit_file::RootProvenance::Repository => 0,
                biscuit_file::RootProvenance::Source => 1,
                biscuit_file::RootProvenance::PackageRoot => 2,
                biscuit_file::RootProvenance::PackageArea => 7,
                biscuit_file::RootProvenance::Home => 3,
                biscuit_file::RootProvenance::Magic => 4,
                biscuit_file::RootProvenance::Vault => 5,
                biscuit_file::RootProvenance::Absolute => 6,
                // Appended after the historical codes; never renumber them —
                // the codes are persisted with graph identities.
                biscuit_file::RootProvenance::LocalRoot => 8,
            });
        }

        enc.field("max_transclusion_depth");
        enc.u64(*max_transclusion_depth as u64);
        enc.field("allow_remote_transclusion");
        enc.bool(*allow_remote_transclusion);
        enc.field("allow_local_markdown");
        enc.bool(*allow_local_markdown);
        enc.field("allow_local_code");
        enc.bool(*allow_local_code);
        enc.field("code_fallback_language");
        enc.str(code_fallback_language);

        enc.field("ignore_invalid_references");
        match ignore_invalid_references {
            None => enc.tag(0),
            Some(false) => enc.tag(1),
            Some(true) => enc.tag(2),
        }

        enc.field("resolve_repo_root");
        enc.bool(*resolve_repo_root);

        enc.field("file_resolution_context");
        encode_file_resolution_context(&mut enc, file_resolution_context);
        enc.field("source_derivation");
        enc.tag(match source_derivation {
            SourceDerivation::Ordinary => 0,
            SourceDerivation::TrustedExternal => 1,
        });
        enc.field("source_opening");
        encode_source_opening(&mut enc, source_opening);

        enc.field("shell_timeout_ns");
        enc.u128(shell_timeout.as_nanos());
        enc.field("shell_timeout_behavior");
        enc.tag(match shell_timeout_behavior {
            ShellTimeoutBehavior::Error => 0,
            ShellTimeoutBehavior::EmptyString => 1,
        });
        enc.field("shell_policy_root");
        match shell_policy_root {
            Some(p) => {
                enc.tag(1);
                enc.path(p);
            }
            None => enc.tag(0),
        }
        enc.field("shell_working_directory");
        match shell_working_directory {
            Some(p) => {
                enc.tag(1);
                enc.path(p);
            }
            None => enc.tag(0),
        }

        // Unordered set: sort for canonical order.
        enc.field("pre_approved_commands");
        match pre_approved_commands {
            Some(set) => {
                enc.tag(1);
                let mut entries: Vec<&str> = set.iter().map(String::as_str).collect();
                entries.sort_unstable();
                enc.count(entries.len());
                for entry in entries {
                    enc.str(entry);
                }
            }
            None => enc.tag(0),
        }

        enc.field("shell_strip_ansi");
        enc.bool(*shell_strip_ansi);

        enc.field("list_spacing");
        enc.tag(match list_spacing {
            crate::markdown::cleanup::ListSpacingMode::Normal => 0,
            crate::markdown::cleanup::ListSpacingMode::Compact => 1,
            crate::markdown::cleanup::ListSpacingMode::Loose => 2,
        });
        enc.field("incidental_newline_mode");
        enc.tag(match incidental_newline_mode {
            crate::markdown::cleanup::IncidentalNewlineMode::Strip => 0,
            crate::markdown::cleanup::IncidentalNewlineMode::Preserve => 1,
        });
        enc.field("fixed_width");
        match fixed_width {
            Some(width) => {
                enc.tag(1);
                enc.u64(*width as u64);
            }
            None => enc.tag(0),
        }
        enc.field("indent_size");
        enc.u64(*indent_size as u64);

        enc.field("cache_access_mode");
        enc.tag(match cache_access_mode {
            CacheAccessMode::Off => 0,
            CacheAccessMode::ReadOnly => 1,
            CacheAccessMode::ReadWrite => 2,
            CacheAccessMode::Refresh => 3,
        });
        enc.field("cache_root");
        match cache_root {
            Some(p) => {
                enc.tag(1);
                enc.path(p);
            }
            None => enc.tag(0),
        }
        enc.field("cache_namespace");
        match cache_namespace {
            Some(n) => {
                enc.tag(1);
                enc.str(n);
            }
            None => enc.tag(0),
        }

        enc.field("perf_enabled");
        enc.bool(*perf_enabled);

        // Runtime context: complete encoding of every captured value and env
        // entry — including the volatile per-second and system-state fields the
        // compose-cache `context_hash` drops. Graph construction interpolates
        // `{{ ctx.* }}` into link/transclusion targets and evaluates `when=`
        // conditions against these values, so the graph identity must be
        // complete and fail closed rather than reuse the cache product.
        enc.field("context");
        enc.u64(graph_context_fingerprint(context));

        // Who may grow `context` above. Encoded because graph identity is
        // conservative and fails closed: an extendable context can gain groups
        // once composed, so two option sets whose contexts compare equal today
        // can still diverge.
        enc.field("context_authority");
        enc.tag(context_authority.fingerprint_tag());

        enc.field("replace_parent_wins");
        enc.bool(*replace_parent_wins);
        enc.field("one_off_replace");
        match one_off_replace {
            Some(m) => {
                enc.tag(1);
                enc.str(&canonical_json_sorted(&Value::Object(m.clone())));
            }
            None => enc.tag(0),
        }
        enc.field("interpolate_code_blocks");
        enc.bool(*interpolate_code_blocks);

        enc.field("baseline_schema");
        match &baseline_canonical {
            Some(canonical) => {
                enc.tag(1);
                enc.str(canonical);
            }
            None => enc.tag(0),
        }
        enc.field("baseline_is_darkmatter_default");
        enc.bool(*baseline_is_darkmatter_default);
        enc.field("trigger_schemas");
        enc.bool(*trigger_schemas);

        // Remote read config: allowed_hosts is an unordered allowlist → sort.
        enc.field("remote_read_config.allowed_hosts");
        {
            let mut hosts: Vec<&str> = remote_read_config
                .allowed_hosts
                .iter()
                .map(String::as_str)
                .collect();
            hosts.sort_unstable();
            enc.count(hosts.len());
            for host in hosts {
                enc.str(host);
            }
        }
        enc.field("remote_read_config.remote_concurrency");
        enc.u64(remote_read_config.remote_concurrency as u64);
        enc.field("remote_read_config.remote_ttl");
        match remote_read_config.remote_ttl {
            Some(ttl) => {
                enc.tag(1);
                enc.u128(ttl.as_nanos());
            }
            None => enc.tag(0),
        }
        enc.field("remote_read_config.refresh");
        enc.bool(remote_read_config.refresh);
        enc.field("remote_read_config.freshness_mode");
        enc.tag(match remote_read_config.freshness_mode {
            RemoteFreshnessMode::Optimistic => 0,
            RemoteFreshnessMode::Strict => 1,
            RemoteFreshnessMode::Fallback => 2,
        });

        enc.field("defer_shell_pending_schema_problems");
        enc.bool(*defer_shell_pending_schema_problems);
        enc.field("defer_schema_verdict");
        enc.bool(*defer_schema_verdict);
        enc.field("defer_missing_runtime_context");
        enc.bool(*defer_missing_runtime_context);
        enc.field("defer_expression_failures");
        enc.bool(*defer_expression_failures);
        enc.field("suppress_shell_probes");
        enc.bool(*suppress_shell_probes);
        // An installed `Active` handle is per-document pipeline state, never
        // caller configuration; only the discovery mode changes what
        // evaluation produces.
        enc.field("nested_compose_discovery");
        enc.bool(nested_compose.is_discovery());
        // The grants are already encoded through `remote_read_config`; what is
        // left is whether this surface sends at all and through whose
        // transport.
        enc.field("icmp_discovery");
        enc.bool(icmp.is_discovery());
        enc.field("icmp_injected_transport");
        enc.bool(icmp.has_injected_transport());
        // The lazy roots are evaluated per expression and never cached, so only
        // whether a refresh capability exists can change a composed result.
        enc.field("current_provider");
        enc.bool(current.has_provider());
        enc.field("schema_phase");
        enc.tag(schema_phase_tag(*schema_phase));

        // Unordered set: sort for canonical order.
        enc.field("exclude_keys");
        {
            let mut keys: Vec<&str> = exclude_keys.iter().map(String::as_str).collect();
            keys.sort_unstable();
            enc.count(keys.len());
            for key in keys {
                enc.str(key);
            }
        }
        // Ordered vector: preserve the caller's coercion precedence.
        enc.field("name_coercion_keys");
        enc.count(name_coercion_keys.len());
        for key in name_coercion_keys {
            enc.str(key);
        }
        // Ordered set: iteration is already canonical.
        enc.field("portable_env");
        enc.count(portable_env.len());
        for name in portable_env {
            enc.str(name);
        }
        enc.field("absolute_fallback_warning");
        enc.bool(*absolute_fallback_warning);

        enc.field("file_ref_fallback_dir");
        match file_ref_fallback_dir {
            Some(dir) => {
                enc.tag(1);
                enc.path(dir);
            }
            None => enc.tag(0),
        }

        let graph_value_fingerprint = enc.finish();

        // ── Compose-cache fingerprint: output-affecting subset only, encoded by
        //    the same typed, length-delimited [`GraphIdentityEncoder`] under the
        //    distinct [`CACHE_OPTIONS_DOMAIN`]. This drops the historical
        //    `Debug`/string-join encoding entirely (no `{:?}`, no `,`/NUL joins)
        //    so element and field boundaries are unambiguous, and the domain
        //    marker keeps its values disjoint from the old encoding's.
        //    Same field subset as the historical hash → cache-reuse semantics
        //    are preserved (equal options still share a key); only the encoding
        //    and value changed.
        let mut cenc = GraphIdentityEncoder::new(CACHE_OPTIONS_DOMAIN);

        cenc.field("enabled_operations");
        let cache_op_indices: Vec<usize> = enabled_operations.iter().map(|op| op.index()).collect();
        cenc.count(cache_op_indices.len());
        for index in cache_op_indices {
            cenc.u64(index as u64);
        }

        cenc.field("fail_fast");
        cenc.bool(*fail_fast);
        cenc.field("max_transclusion_depth");
        cenc.u64(*max_transclusion_depth as u64);
        cenc.field("allow_remote_transclusion");
        cenc.bool(*allow_remote_transclusion);
        cenc.field("allow_local_markdown");
        cenc.bool(*allow_local_markdown);
        cenc.field("allow_local_code");
        cenc.bool(*allow_local_code);
        cenc.field("code_fallback_language");
        cenc.str(code_fallback_language);

        cenc.field("ignore_invalid_references");
        match ignore_invalid_references {
            None => cenc.tag(0),
            Some(false) => cenc.tag(1),
            Some(true) => cenc.tag(2),
        }

        cenc.field("resolve_repo_root");
        cenc.bool(*resolve_repo_root);

        cenc.field("file_resolution_context");
        encode_file_resolution_context(&mut cenc, file_resolution_context);
        cenc.field("source_derivation");
        cenc.tag(match source_derivation {
            SourceDerivation::Ordinary => 0,
            SourceDerivation::TrustedExternal => 1,
        });
        // The opening anchor can change the source's tree root, and with it
        // what the source's relative references resolve to.
        cenc.field("source_opening");
        encode_source_opening(&mut cenc, source_opening);

        cenc.field("list_spacing");
        cenc.tag(match list_spacing {
            crate::markdown::cleanup::ListSpacingMode::Normal => 0,
            crate::markdown::cleanup::ListSpacingMode::Compact => 1,
            crate::markdown::cleanup::ListSpacingMode::Loose => 2,
        });
        cenc.field("indent_size");
        cenc.u64(*indent_size as u64);
        cenc.field("replace_parent_wins");
        cenc.bool(*replace_parent_wins);

        cenc.field("one_off_replace");
        match one_off_replace {
            Some(m) => {
                cenc.tag(1);
                cenc.str(&canonical_json_sorted(&Value::Object(m.clone())));
            }
            None => cenc.tag(0),
        }
        cenc.field("external_state");
        match external_state {
            Some(v) => {
                cenc.tag(1);
                cenc.str(&canonical_json_sorted(v));
            }
            None => cenc.tag(0),
        }
        cenc.field("set_overrides");
        match set_overrides {
            Some(v) => {
                cenc.tag(1);
                cenc.str(&canonical_json_sorted(v));
            }
            None => cenc.tag(0),
        }
        cenc.field("data_overrides");
        match data_overrides {
            Some(v) => {
                cenc.tag(1);
                cenc.str(&canonical_json_sorted(v));
            }
            None => cenc.tag(0),
        }
        cenc.field("inherited_origin");
        inherited_origin.encode(&mut cenc);
        cenc.field("caller_input_records");
        cenc.count(caller_input_records.len());
        for (property, record) in caller_input_records {
            cenc.str(property);
            cenc.str(&canonical_json_sorted(record.raw()));
            encode_file_resolution_context(&mut cenc, record.origin());
        }
        cenc.field("baseline_schema");
        match &baseline_canonical {
            Some(canonical) => {
                cenc.tag(1);
                cenc.str(canonical);
            }
            None => cenc.tag(0),
        }
        cenc.field("trigger_schemas");
        cenc.bool(*trigger_schemas);
        cenc.field("defer_schema_verdict");
        cenc.bool(*defer_schema_verdict);
        cenc.field("schema_phase");
        cenc.tag(schema_phase_tag(*schema_phase));
        cenc.field("name_coercion_keys");
        cenc.count(name_coercion_keys.len());
        for key in name_coercion_keys {
            cenc.str(key);
        }

        cenc.field("file_ref_fallback_dir");
        match file_ref_fallback_dir {
            Some(dir) => {
                cenc.tag(1);
                cenc.path(dir);
            }
            None => cenc.tag(0),
        }

        let cache_value_fingerprint = cenc.finish();

        ComposeOptionsClassification {
            graph_value_fingerprint,
            cache_value_fingerprint,
            shell_approval_handler: shell_approval_handler.as_ref().map(Arc::downgrade),
            preflight_graph: preflight_graph.as_ref().map(Arc::downgrade),
            remote_fetch: remote_fetch.as_ref().map(RemoteFetchRuntime::weak_id),
        }
    }

    /// Compose-cache value fingerprint over the output-affecting option subset.
    ///
    /// Derived from the single [`classify_options`](Self::classify_options)
    /// inventory — the same field authority the reference-graph identity uses,
    /// so there is no parallel field list. `options_hash` delegates here.
    /// Encoded by the typed, length-delimited [`GraphIdentityEncoder`] under
    /// [`CACHE_OPTIONS_DOMAIN`]; it carries no `Debug` encoding and is not
    /// value-compatible with the historical string-join hash.
    pub(crate) fn compose_cache_fingerprint(
        &self,
        context: &biscuit_file::FileResolutionContext,
    ) -> u64 {
        self.classify_options(context).cache_value_fingerprint
    }
}

/// Conservative, fail-closed identity of the graph-affecting compose options.
///
/// Covers **every** `ComposeOptions` field via the shared classification: a
/// compact value fingerprint over all value-representable fields plus weak
/// instance handles for the three stateful `Arc`-backed fields. Equality is
/// clone-stable — a clone shares each `Arc`, so weak handles compare equal —
/// and fails closed: a dropped or independently constructed stateful instance
/// is never equivalent even when the visible configuration matches. This is an
/// in-process correctness guard, **not** a cryptographically unforgeable token.
///
/// Consumed by the reference-graph provenance: `from_build` captures it and
/// prebuilt-graph validation compares it.
#[derive(Clone)]
pub(crate) struct ReferenceGraphOptionsIdentity {
    value_fingerprint: u64,
    shell_approval_handler: Option<Weak<dyn ShellApprovalHandler>>,
    preflight_graph: Option<Weak<PreflightGraphNode>>,
    remote_fetch: Option<RemoteFetchWeakId>,
}

impl ReferenceGraphOptionsIdentity {
    /// Captures the identity of `request`'s graph-affecting configuration.
    pub(crate) fn capture(request: &super::request::ComposeRequest) -> Self {
        let classification = request.options().classify_options(request.resolution_context());
        Self {
            value_fingerprint: classification.graph_value_fingerprint,
            shell_approval_handler: classification.shell_approval_handler,
            preflight_graph: classification.preflight_graph,
            remote_fetch: classification.remote_fetch,
        }
    }
}

impl std::fmt::Debug for ReferenceGraphOptionsIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Only the value fingerprint is reported; the retained weak handles are
        // never presented so provenance cannot leak stateful identity.
        f.debug_struct("ReferenceGraphOptionsIdentity")
            .field("value_fingerprint", &format_args!("{:016x}", self.value_fingerprint))
            .finish_non_exhaustive()
    }
}

impl PartialEq for ReferenceGraphOptionsIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.value_fingerprint == other.value_fingerprint
            && weak_opt_same_instance(&self.shell_approval_handler, &other.shell_approval_handler)
            && weak_opt_same_instance(&self.preflight_graph, &other.preflight_graph)
            && remote_fetch_opt_same_instance(&self.remote_fetch, &other.remote_fetch)
    }
}

impl Eq for ReferenceGraphOptionsIdentity {}

/// Compares two optional weak handles by live allocation identity.
///
/// `None`/`None` is equal; `Some`/`Some` is equal only while both weak handles
/// still upgrade to the same allocation (a dropped instance is never equal);
/// `Some`/`None` is never equal.
#[allow(dead_code)]
fn weak_opt_same_instance<T: ?Sized>(a: &Option<Weak<T>>, b: &Option<Weak<T>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => match (a.upgrade(), b.upgrade()) {
            (Some(a), Some(b)) => Arc::ptr_eq(&a, &b),
            _ => false,
        },
        _ => false,
    }
}

#[allow(dead_code)]
fn remote_fetch_opt_same_instance(
    a: &Option<RemoteFetchWeakId>,
    b: &Option<RemoteFetchWeakId>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.same_instance(b),
        _ => false,
    }
}

/// Stable identity tag for the compose-time schema phase: `0` unphased,
/// `1` launch, `2` completion.
fn schema_phase_tag(phase: Option<crate::markdown::schemas::SchemaPhase>) -> u8 {
    match phase {
        None => 0,
        Some(crate::markdown::schemas::SchemaPhase::Launch) => 1,
        Some(crate::markdown::schemas::SchemaPhase::Completion) => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::{test_request, test_request_in, ComposeRequest};
    use std::path::PathBuf;

    /// `ComposeOptions::new()` must not perform host or repository discovery.
    ///
    /// A constructor has no document to justify probing Git, repository
    /// topology, working-tree changes, languages, documents, OS, hardware, or
    /// GPU. Reintroducing that capture is not a visible failure — every test
    /// still passes, the suite just silently gets a working-tree walk per
    /// construction (~2s inside this monorepo on Windows, across ~400 call
    /// sites). That is precisely the regression this asserts against, so it is
    /// pinned on the group identity rather than on a duration, which would be
    /// flaky and would not say what broke.
    ///
    /// The groups stay reachable: composition extends the request context with
    /// the groups a document names before that document's first expression
    /// stage, never during evaluation.
    #[test]
    fn new_captures_no_discovery_derived_group() {
        use crate::markdown::compose::context::capture::ContextGroup;

        let options = ComposeOptions::new();

        let discovered: Vec<(&String, ContextGroup)> = options
            .context
            .values()
            .iter()
            .filter_map(|(key, _)| {
                ContextGroup::for_key(key)
                    .filter(|group| *group != ContextGroup::DateTime)
                    .map(|group| (key, group))
            })
            .collect();

        assert!(
            discovered.is_empty(),
            "ComposeOptions::new() captured discovery-derived keys: {discovered:?}. \
             Construct with `new_with_context(ComposeContext::capture_for_document(..))` \
             when a document is in hand, or `ComposeContext::capture_for_dir(..)` for a deliberate \
             full snapshot.",
        );
    }

    /// A request prepared at the launch directory anchors the unanchored
    /// default context there; a later source elsewhere does not move it.
    #[test]
    fn request_context_upgrade_keeps_the_request_launch_anchor() {
        let launch = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let expected = launch.path().to_path_buf();
        let mut request = ComposeRequest::prepare(
            ComposeOptions::new(),
            &super::super::request::RequestSnapshot::new(&expected),
        )
        .unwrap();

        request.source = ComposeSource::File(target.path().join("prompt.md"));
        let document: crate::markdown::Markdown = "{{ ctx.cwd }}".into();
        request.extend_context_for(&document);

        assert_eq!(request.context.anchor(), expected);
        assert_eq!(
            request.context.get("cwd"),
            Some(&serde_json::json!(biscuit_file::to_portable_string(&expected))),
        );
    }

    #[test]
    fn file_ref_fallback_dir_defaults_to_none() {
        let options = ComposeOptions::new();
        assert!(options.file_ref_fallback_dir.is_none());
    }

    #[test]
    fn with_file_ref_fallback_dir_sets_the_field() {
        let options = ComposeOptions::new().with_file_ref_fallback_dir("/tmp/launch");
        assert_eq!(
            options.file_ref_fallback_dir.as_deref(),
            Some(std::path::Path::new("/tmp/launch")),
        );
    }

    /// A `ComposeOptions` with a fallback produces a `ResolutionContext`
    /// carrying it — verifying the builder threads through to both resolution
    /// contexts (Phase 2 Track A verification goal).
    #[test]
    fn frontmatter_resolution_context_carries_fallback() {
        let options = test_request(ComposeOptions::new().with_file_ref_fallback_dir("/tmp/launch"));
        let ctx = options.frontmatter_resolution_context();
        assert_eq!(
            ctx.file_ref_fallback_dir.as_deref(),
            Some(std::path::Path::new("/tmp/launch")),
        );
    }

    /// Without a fallback set, the resolution context leaves the field as
    /// `None` — preserving the legacy document-only resolution behavior.
    #[test]
    fn frontmatter_resolution_context_without_fallback_is_none() {
        let options = test_request(ComposeOptions::new());
        let ctx = options.frontmatter_resolution_context();
        assert!(ctx.file_ref_fallback_dir.is_none());
    }

    #[test]
    #[serial_test::serial(darkmatter_file_cwd)]
    fn expression_and_frontmatter_adapters_reuse_the_request_snapshot() {
        use biscuit_file::file_reference::fetch::FetchPolicy;
        use biscuit_file::FileReference;

        let request_root = tempfile::tempdir().unwrap();
        let nested = request_root.path().join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let target = request_root.path().join("captured.md");
        std::fs::write(&target, "captured").unwrap();
        let ambient = tempfile::tempdir().unwrap();
        let prior = std::env::var_os("DARKMATTER_SNAPSHOT_ROOT");

        let mut env = std::collections::HashMap::new();
        env.insert(
            "DARKMATTER_SNAPSHOT_ROOT".to_string(),
            request_root.path().display().to_string(),
        );
        let snapshot = biscuit_file::FileResolutionContext::new(request_root.path()).with_env(env);
        let options = test_request_in(ComposeOptions::new().with_source_file(nested.join("child.md")), snapshot);

        // SAFETY: this test is serialized while mutating process-global state.
        unsafe { std::env::set_var("DARKMATTER_SNAPSHOT_ROOT", ambient.path()) };
        let remote_fetch = crate::markdown::compose::remote_fetch::RemoteFetchRuntime::with_policy(
            FetchPolicy::deny_all(),
        );
        let expression = options.expression_resolution_context(&remote_fetch);
        let frontmatter = options.frontmatter_resolution_context();
        let file_ref = FileReference::new("{{DARKMATTER_SNAPSHOT_ROOT}}/captured.md").unwrap();
        let resolved = [&expression, &frontmatter].map(|ctx| {
            crate::markdown::compose::expression::resolve_ctx::resolve_document_file_ref(
                &file_ref,
                &ctx.cwd,
                &ctx.file_resolution_context,
            )
            .unwrap()
        });
        match prior {
            Some(value) => unsafe { std::env::set_var("DARKMATTER_SNAPSHOT_ROOT", value) },
            None => unsafe { std::env::remove_var("DARKMATTER_SNAPSHOT_ROOT") },
        }

        assert_eq!(expression.cwd, nested);
        assert_eq!(frontmatter.cwd, nested);
        for path in resolved {
            assert_eq!(path.as_deref(), Some(target.as_path()));
        }
    }

    /// The fallback appears in the `Debug` output so diagnostics surface it.
    #[test]
    fn debug_impl_includes_file_ref_fallback_dir() {
        let options = ComposeOptions::new().with_file_ref_fallback_dir("/tmp/launch");
        let debug = format!("{options:?}");
        assert!(
            debug.contains("file_ref_fallback_dir"),
            "expected Debug to include file_ref_fallback_dir, got: {debug}",
        );
        assert!(
            debug.contains("/tmp/launch"),
            "expected Debug to include the path, got: {debug}",
        );
    }

    /// `PathBuf` type assertion — the field accepts any `Into<PathBuf>`.
    #[test]
    #[allow(dead_code)]
    fn file_ref_fallback_dir_accepts_pathbuf() {
        let _: PathBuf = PathBuf::from("/tmp/x");
    }

    /// `with_darkmatter_baseline_schema()` injects the authored baseline schema
    /// into the compose options (Phase 4).
    #[test]
    fn with_darkmatter_baseline_schema_injects_baseline() {
        let options = ComposeOptions::new().with_darkmatter_baseline_schema();
        assert!(
            options.baseline_schema.is_some(),
            "baseline schema must be injected"
        );
    }

    /// The baseline-injected options still allow unknown user frontmatter keys
    /// (Non-Goal 1; spec testing requirement 5) and preserve document `$schema`
    /// precedence (Non-Goal 5; spec testing requirement 6).
    #[test]
    fn darkmatter_baseline_compose_allows_unknown_keys_and_preserves_document_schema() {
        use crate::markdown::schemas::DarkmatterSchemas;
        use crate::markdown::Markdown;

        let options = ComposeOptions::new().with_darkmatter_baseline_schema();
        let baseline = options
            .baseline_schema
            .expect("baseline schema must be set");
        let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()))
            .with_baseline(baseline)
            .expect("baseline must convert");

        // Unknown keys are allowed by the baseline.
        let md_unknown: Markdown = "---\ncustom_key: 42\n---\nbody\n".into();
        let report = api.validate(&md_unknown).expect("validate");
        assert!(
            report.valid,
            "unknown user keys must remain accepted: {:?}",
            report.problems
        );

        // Document `$schema` wins over baseline.
        let md_override: Markdown = "---\n$schema:\n  title: number\ntitle: 42\n---\nbody\n".into();
        let report = api.validate(&md_override).expect("validate");
        assert!(
            report.valid,
            "document schema should override baseline title type: {:?}",
            report.problems
        );
    }

    // ── Reference-graph options identity (Phase 1) ─────────────────────

    use super::super::super::shell_expansion::types::{
        ShellApprovalDecision, ShellApprovalHandler as ShellApprovalHandlerTrait,
        ShellApprovalRequest,
    };
    use crate::markdown::compose::ShellExpansionError;
    use std::sync::Arc;

    struct DummyApproval;
    impl ShellApprovalHandlerTrait for DummyApproval {
        fn approve(
            &self,
            _request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(ShellApprovalDecision::Deny)
        }
    }

    /// A process-independent request context for the identity fixtures, so
    /// two options built alike compare only their own fields.
    fn neutral_context() -> biscuit_file::FileResolutionContext {
        biscuit_file::FileResolutionContext::from_snapshot(
            fixture_root(""),
            None,
            std::collections::HashMap::new(),
        )
    }

    /// An absolute fixture path under a fixed root on every OS.
    fn fixture_root(relative: &str) -> PathBuf {
        let root = std::env::temp_dir().join("identity-fixture");
        if relative.is_empty() { root } else { root.join(relative) }
    }

    fn req(options: ComposeOptions) -> ComposeRequest {
        test_request_in(options, neutral_context())
    }

    fn req_in(options: ComposeOptions, context: biscuit_file::FileResolutionContext) -> ComposeRequest {
        test_request_in(options, context)
    }

    /// The graph identity of `options` under the neutral context.
    fn id(options: &ComposeOptions) -> ReferenceGraphOptionsIdentity {
        rid(&req(options.clone()))
    }

    fn rid(request: &ComposeRequest) -> ReferenceGraphOptionsIdentity {
        ReferenceGraphOptionsIdentity::capture(request)
    }

    /// The compose-cache fingerprint of `options` under the neutral context.
    fn fp(options: &ComposeOptions) -> u64 {
        options.compose_cache_fingerprint(&neutral_context())
    }

    /// Options over a deterministic fixed context.
    ///
    /// The graph identity folds in the complete runtime context (including
    /// volatile `now`/`timestamp`/`memory_*`), so comparing options built from
    /// two independent `ComposeContext::capture_for_dir(..)` calls would differ on that
    /// capture drift alone. A shared fixed context isolates the option field
    /// each identity test actually targets.
    fn fixed_opts() -> ComposeOptions {
        ComposeOptions::new_with_context(ComposeContext::fixed_for_testing())
    }

    #[test]
    fn options_identity_ignores_unordered_set_insertion_order() {
        // `exclude_keys` (HashSet) and `pre_approved_commands` (HashSet) are
        // genuinely unordered: identity must be insensitive to insertion order.
        let a = fixed_opts()
            .with_exclude_keys(["alpha", "beta", "gamma"])
            .with_pre_approved_commands(
                ["ls", "cat", "echo"].iter().map(|s| s.to_string()).collect(),
            );
        let b = fixed_opts()
            .with_exclude_keys(["gamma", "alpha", "beta"])
            .with_pre_approved_commands(
                ["echo", "ls", "cat"].iter().map(|s| s.to_string()).collect(),
            );
        assert_eq!(id(&a), id(&b));

        // `portable_env` is a set too: declaration order and repeats are not
        // behavior, but a name is.
        let c = fixed_opts().with_portable_env(["A", "B"]);
        let d = fixed_opts().with_portable_env(["B", "A", "B"]);
        assert_eq!(id(&c), id(&d));
        assert_ne!(id(&c), id(&fixed_opts().with_portable_env(["A"])));
        // Suppressing the absolute-fallback warning changes the report.
        assert_ne!(
            id(&fixed_opts()),
            id(&fixed_opts().with_absolute_fallback_warning(false))
        );
    }

    /// The snapshot's tree root and reader opt-in, and the reference that
    /// opened the source, decide what relative references resolve to, so
    /// options that differ only in one of them share neither identity.
    #[test]
    fn file_tree_and_source_opening_participate_in_graph_and_cache_identity() {
        let temp = tempfile::tempdir().unwrap();
        let snapshot = || {
            biscuit_file::FileResolutionContext::from_snapshot(
                temp.path().join("docs"),
                None,
                std::collections::HashMap::new(),
            )
        };
        let base = req_in(fixed_opts(), snapshot());
        let variants = [
            req_in(fixed_opts(), snapshot().with_base_dir(temp.path())),
            req_in(fixed_opts(), snapshot().allow_external_relative()),
            base.clone().with_accepted_source_file(
                temp.path().join("docs/a.md"),
                Some(SourceOpening {
                    reference: biscuit_file::FileReference::new("./a.md").unwrap(),
                    resolved: temp.path().join("docs/a.md"),
                }),
            ),
        ];
        let unopened = base.clone().with_accepted_source_file(temp.path().join("docs/a.md"), None);
        for variant in &variants {
            assert_ne!(rid(&base), rid(variant));
            assert_ne!(base.compose_cache_fingerprint(), variant.compose_cache_fingerprint());
        }
        assert_ne!(rid(&unopened), rid(&variants[2]));
        assert_ne!(unopened.compose_cache_fingerprint(), variants[2].compose_cache_fingerprint());
    }

    #[test]
    fn caller_input_origin_participates_in_graph_and_cache_identity() {
        let records = |base: &str| {
            [(
                "spec".to_string(),
                CallerInputRecord::new(
                    serde_json::json!("fixes/case/spec.md"),
                    biscuit_file::FileResolutionContext::from_snapshot(
                        fixture_root(base),
                        None,
                        std::collections::HashMap::new(),
                    ),
                ),
            )]
            .into_iter()
            .collect()
        };
        let a = fixed_opts().with_caller_input_records(records("repo/one"));
        let b = fixed_opts().with_caller_input_records(records("repo/two"));
        let cloned = a.clone();

        assert_ne!(id(&a), id(&b));
        assert_ne!(fp(&a), fp(&b));
        assert_eq!(id(&a), id(&cloned));
        assert_eq!(fp(&a), fp(&cloned));
        assert_eq!(
            a.caller_input_records()["spec"].raw(),
            &serde_json::json!("fixes/case/spec.md")
        );
    }

    /// A bare source context for the identity fixtures below.
    /// Fixed options whose context registers `roots` as start-position `@`
    /// roots, in order.
    fn with_magic_roots(roots: &[&str]) -> ComposeRequest {
        let context = roots.iter().fold(identity_source_context(), |context, root| {
            context.add_magic_path(*root, biscuit_file::PathPosition::Start)
        });
        req_in(fixed_opts(), context)
    }

    fn identity_source_context() -> biscuit_file::FileResolutionContext {
        biscuit_file::FileResolutionContext::from_snapshot(
            fixture_root("repo/docs"),
            Some(fixture_root("repo")),
            std::collections::HashMap::new(),
        )
        .with_repository_root(fixture_root("repo"))
    }

    /// The captured launch `@` scope anchors the whole `@` chain independently
    /// of the source anchors: two otherwise-identical options whose contexts
    /// differ only in that scope can resolve the same `@` reference to
    /// different files, so both identity products must distinguish them.
    #[test]
    fn identities_distinguish_launch_magic_scope() {
        let launch_a = biscuit_file::FileResolutionContext::from_snapshot(
            fixture_root("repo/area-a"),
            None,
            std::collections::HashMap::new(),
        )
        .with_repository_root(fixture_root("repo"));
        let launch_b = biscuit_file::FileResolutionContext::from_snapshot(
            fixture_root("repo/area-b"),
            None,
            std::collections::HashMap::new(),
        )
        .with_repository_root(fixture_root("repo"));
        assert_ne!(
            launch_a.launch_magic_scope(),
            launch_b.launch_magic_scope(),
            "fixture must vary only the launch scope"
        );

        let source = identity_source_context();
        let a = req_in(
            fixed_opts(),
            source
                .clone()
                .with_launch_magic_scope(launch_a.launch_magic_scope().clone()),
        );
        let b = req_in(
            fixed_opts(),
            source
                .clone()
                .with_launch_magic_scope(launch_b.launch_magic_scope().clone()),
        );

        assert_ne!(rid(&a), rid(&b));
        assert_ne!(
            a.compose_cache_fingerprint(),
            b.compose_cache_fingerprint()
        );
    }

    /// A configured magic root's tier policy can reorder the `@` chain without
    /// changing the root path, its position, or any other field — a `User`
    /// override moves a local-looking root behind every local-tier candidate.
    #[test]
    fn identities_distinguish_magic_tier_override() {
        let base = identity_source_context();
        let inferred = req_in(
            fixed_opts(),
            base.clone()
                .add_magic_path(fixture_root("repo/configs"), biscuit_file::PathPosition::Start),
        );
        let user = req_in(
            fixed_opts(),
            base.add_magic_path_with_tier(
                fixture_root("repo/configs"),
                biscuit_file::PathPosition::Start,
                biscuit_file::MagicPathTier::User,
            ),
        );

        assert_ne!(rid(&inferred), rid(&user));
        assert_ne!(
            inferred.compose_cache_fingerprint(),
            user.compose_cache_fingerprint()
        );
    }

    /// `package_root` is an intrinsic local-tier `@` root; a context that
    /// supplies it resolves `@x.md` differently from one that does not, so the
    /// field participates in both identity products.
    #[test]
    fn identities_distinguish_context_package_root() {
        let without = req_in(fixed_opts(), identity_source_context());
        let with = req_in(
            fixed_opts(),
            identity_source_context().with_package_root(fixture_root("repo/darkmatter/lib")),
        );

        assert_ne!(rid(&without), rid(&with));
        assert_ne!(
            without.compose_cache_fingerprint(),
            with.compose_cache_fingerprint()
        );
    }

    #[test]
    fn caller_file_occurrences_each_participate_in_request_identity() {
        let origin = biscuit_file::FileResolutionContext::from_snapshot(
            fixture_root("repo/launch"),
            Some(fixture_root("repo")),
            std::collections::HashMap::new(),
        );
        let candidate = fixture_root("repo/launch/missing.md");
        let provenance = |property: &str, reference: &str| {
            crate::markdown::compose::expression::resolve_ctx::CallerFileProvenance {
                property: property.to_string(),
                reference: reference.to_string(),
                origin: origin.clone(),
                candidate: candidate.clone(),
                candidate_provenance: biscuit_file::RootProvenance::Source,
            }
        };
        let mut complete = fixed_opts();
        complete.caller_file_provenance = [
            ("/first".to_string(), provenance("first", "missing.md")),
            ("/second".to_string(), provenance("second", "./missing.md")),
        ]
        .into_iter()
        .collect();
        let mut collapsed = complete.clone();
        collapsed.caller_file_provenance.remove("/first");

        assert_eq!(complete.caller_file_provenance.len(), 2);
        assert_eq!(complete.caller_file_provenance["/first"].property, "first");
        assert_eq!(complete.caller_file_provenance["/second"].property, "second");
        assert_eq!(
            complete.caller_file_provenance["/first"].candidate,
            complete.caller_file_provenance["/second"].candidate
        );
        assert_ne!(id(&complete), id(&collapsed));
    }

    #[test]
    fn options_identity_sensitive_to_ordered_vector_reorder() {
        // Magic-root order is behavioral (search-root precedence); reordering
        // must change identity.
        let a = with_magic_roots(&["/one", "/two"]);
        let b = with_magic_roots(&["/two", "/one"]);
        assert_ne!(rid(&a), rid(&b));
    }

    /// The length-prefixed encoding keeps set-element boundaries: a single
    /// element that embeds the historical `,` delimiter must not collide with
    /// two separate elements. The old comma/NUL join collapsed both to the same
    /// fingerprint even though the values are not equivalent (they change which
    /// commands are pre-approved).
    #[test]
    fn options_identity_pre_approved_commands_element_boundaries_are_injective() {
        let merged = fixed_opts()
            .with_pre_approved_commands(["a,b"].iter().map(|s| s.to_string()).collect());
        let split = fixed_opts()
            .with_pre_approved_commands(["a", "b"].iter().map(|s| s.to_string()).collect());
        assert_ne!(id(&merged), id(&split));
    }

    /// Same boundary guarantee for the `exclude_keys` set.
    #[test]
    fn options_identity_exclude_keys_element_boundaries_are_injective() {
        let merged = fixed_opts().with_exclude_keys(["a,b"]);
        let split = fixed_opts().with_exclude_keys(["a", "b"]);
        assert_ne!(id(&merged), id(&split));
    }

    /// Same boundary guarantee for the `portable_env` set and
    /// the sorted `allowed_hosts` allowlist: a delimiter embedded in one element
    /// stays distinct from that delimiter splitting two elements.
    #[test]
    fn options_identity_portable_env_and_host_element_boundaries_are_injective() {
        let merged_env = fixed_opts().with_portable_env(["A,B"]);
        let split_env = fixed_opts().with_portable_env(["A", "B"]);
        assert_ne!(id(&merged_env), id(&split_env));

        let merged_host = fixed_opts().with_allowed_host("a.example,b.example");
        let split_host = fixed_opts()
            .with_allowed_host("a.example")
            .with_allowed_host("b.example");
        assert_ne!(id(&merged_host), id(&split_host));
    }

    /// The typed encoder distinguishes an absent optional field from a present
    /// but empty one: `None` writes a `0` tag while `Some(<empty>)` writes a `1`
    /// tag plus a zero count, so a dropped value and an empty value never share
    /// a graph identity.
    #[test]
    fn options_identity_distinguishes_none_from_empty_collection() {
        let base = fixed_opts();

        // `pre_approved_commands: Option<HashSet>` — None vs Some(empty set).
        let empty_pre_approved =
            fixed_opts().with_pre_approved_commands(std::collections::HashSet::new());
        assert_ne!(id(&base), id(&empty_pre_approved));

        // `external_state: Option<Value>` — None vs Some(empty object).
        let empty_state = fixed_opts().with_external_state(serde_json::json!({}));
        assert_ne!(id(&base), id(&empty_state));
    }

    /// Two non-UTF-8 Unix paths that `Path::display()` renders identically must
    /// not share a graph identity. `display()` maps every distinct invalid byte
    /// sequence to the same U+FFFD, so encoding paths through it let unequal
    /// `PathBuf`s — which select different files — produce one fingerprint, and
    /// prebuilt-graph validation would then accept a graph built from different
    /// resolution inputs.
    #[cfg(unix)]
    #[test]
    fn options_identity_distinguishes_non_utf8_paths_that_display_identically() {
        let (a, b) = lossy_twin_paths();

        for (label, build) in path_field_builders() {
            assert_ne!(
                rid(&build(a.clone())),
                rid(&build(b.clone())),
                "graph identity collided on non-UTF-8 `{label}` paths"
            );
        }
    }

    /// The same guarantee for the compose-cache product: a collision there
    /// reuses an unrelated run-local entry. Only the path fields the cache
    /// fingerprint actually covers are asserted.
    #[cfg(unix)]
    #[test]
    fn cache_fingerprint_distinguishes_non_utf8_paths_that_display_identically() {
        let (a, b) = lossy_twin_paths();

        let magic = |p: PathBuf| {
            req_in(
                fixed_opts(),
                identity_source_context().add_magic_path(p, biscuit_file::PathPosition::Start),
            )
        };
        assert_ne!(
            magic(a.clone()).compose_cache_fingerprint(),
            magic(b.clone()).compose_cache_fingerprint()
        );

        let fallback = |p: PathBuf| fixed_opts().with_file_ref_fallback_dir(p);
        assert_ne!(fp(&fallback(a)), fp(&fallback(b)));
    }

    /// Two `PathBuf`s whose `OsStr` bytes differ only in an invalid continuation
    /// byte. Both render as `"f\u{FFFD}"`, which is exactly the collision the
    /// exact encoder exists to prevent.
    #[cfg(unix)]
    fn lossy_twin_paths() -> (PathBuf, PathBuf) {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let a = PathBuf::from(OsStr::from_bytes(&[0x66, 0x80]));
        let b = PathBuf::from(OsStr::from_bytes(&[0x66, 0x81]));

        assert_ne!(a, b, "the two paths must be genuinely unequal");
        assert_eq!(
            a.display().to_string(),
            b.display().to_string(),
            "precondition: `display()` must render both identically, otherwise \
             this fixture no longer exercises the lossy-encoding collision"
        );
        (a, b)
    }

    /// Every path-valued field that feeds the graph identity, as a builder from
    /// a path to the options carrying it.
    #[cfg(unix)]
    #[allow(clippy::type_complexity)]
    fn path_field_builders() -> Vec<(&'static str, Box<dyn Fn(PathBuf) -> ComposeRequest>)> {
        vec![
            (
                "magic_paths",
                Box::new(|p| {
                    req_in(
                        fixed_opts(),
                        identity_source_context().add_magic_path(p, biscuit_file::PathPosition::Start),
                    )
                }) as Box<dyn Fn(PathBuf) -> ComposeRequest>,
            ),
            (
                "shell_policy_root",
                Box::new(|p| req(fixed_opts().with_shell_policy_root(p))),
            ),
            (
                "shell_working_directory",
                Box::new(|p| req(fixed_opts().with_shell_working_directory(p))),
            ),
            ("cache_root", Box::new(|p| req(fixed_opts().with_cache_root(p)))),
            (
                "file_ref_fallback_dir",
                Box::new(|p| req(fixed_opts().with_file_ref_fallback_dir(p))),
            ),
            (
                "source_file",
                Box::new(|p| req(fixed_opts().with_source_file(p))),
            ),
        ]
    }

    /// Portable separator case: a separator inside one collection element must
    /// not collide with that separator splitting two elements. The path encoder
    /// keeps its payload length-prefixed, so element boundaries survive on every
    /// platform — a naive concatenation would fold both shapes into `a/b`.
    #[test]
    fn options_identity_magic_path_separator_does_not_cross_element_boundary() {
        let merged = with_magic_roots(&["a/b"]);
        let split = with_magic_roots(&["a", "b"]);

        assert_ne!(rid(&merged), rid(&split));
        assert_ne!(
            merged.compose_cache_fingerprint(),
            split.compose_cache_fingerprint()
        );
    }

    /// The same guarantee across a *field* boundary: one path spanning a
    /// separator must not collide with its halves landing in two adjacent
    /// path-valued fields.
    #[test]
    fn options_identity_separator_does_not_cross_shell_root_field_boundary() {
        let merged = fixed_opts().with_shell_policy_root("a/b");
        let split = fixed_opts()
            .with_shell_policy_root("a")
            .with_shell_working_directory("b");
        assert_ne!(id(&merged), id(&split));
    }

    /// Path encoding is deterministic, so the Finding 18 reuse guard — which
    /// validates a graph built from `options.clone()` against `options` — keeps
    /// matching for path-bearing options.
    #[test]
    fn options_identity_path_encoding_is_clone_stable() {
        let opts = with_magic_roots(&["/one"]).derive(|options| {
            options
                .with_shell_policy_root("/policy")
                .with_shell_working_directory("/work")
                .with_cache_root("/cache")
                .with_file_ref_fallback_dir("/fallback")
                .with_source_file("/doc.md")
        });

        assert_eq!(rid(&opts), rid(&opts.clone()));
        assert_eq!(
            opts.compose_cache_fingerprint(),
            opts.clone().compose_cache_fingerprint()
        );
    }

    #[test]
    fn options_identity_sensitive_across_representative_families() {
        let base = fixed_opts();
        // scalar
        assert_ne!(id(&base), id(&fixed_opts().with_max_transclusion_depth(3)));
        // collection
        assert_ne!(id(&base), id(&fixed_opts().with_exclude_keys(["k"])));
        // transclusion
        assert_ne!(id(&base), id(&fixed_opts().with_allow_local_markdown(false)));
        // remote
        assert_ne!(id(&base), id(&fixed_opts().with_allowed_host("example.com")));
        // shell
        assert_ne!(id(&base), id(&fixed_opts().with_shell_strip_ansi(false)));
        // schema
        assert_ne!(id(&base), id(&fixed_opts().with_darkmatter_baseline_schema()));
    }

    #[test]
    fn options_identity_sensitive_to_volatile_context_value() {
        use super::super::super::cache::hashing::context_hash;

        // Two contexts identical except for `timestamp` — a field the
        // compose-cache `context_hash` deliberately drops. Graph
        // construction interpolates `{{ ctx.timestamp }}` into link and
        // transclusion targets, so distinct timestamps can yield distinct
        // references and must not share a graph identity.
        let ctx_a =
            ComposeContext::fixed_for_testing_with([("timestamp", serde_json::json!("1000"))]);
        let ctx_b =
            ComposeContext::fixed_for_testing_with([("timestamp", serde_json::json!("2000"))]);

        // Precondition: the cache product genuinely collides here (this is why
        // reusing it for the graph identity was unsound).
        assert_eq!(
            context_hash(&ctx_a),
            context_hash(&ctx_b),
            "cache context_hash is expected to drop the volatile timestamp"
        );

        let a = ComposeOptions::new_with_context(ctx_a);
        let b = ComposeOptions::new_with_context(ctx_b);
        assert_ne!(
            id(&a),
            id(&b),
            "graph identity must distinguish contexts differing only in a volatile value"
        );
    }

    #[test]
    fn options_identity_sensitive_to_volatile_system_state() {
        // A `when=` transclusion condition can reference volatile system-state
        // values such as `memory_used`, which `context_hash` also drops. The
        // graph identity must still separate them so a prebuilt graph is not
        // reused across a changed condition outcome.
        let ctx_a =
            ComposeContext::fixed_for_testing_with([("memory_used", serde_json::json!(1024))]);
        let ctx_b =
            ComposeContext::fixed_for_testing_with([("memory_used", serde_json::json!(2048))]);
        let a = ComposeOptions::new_with_context(ctx_a);
        let b = ComposeOptions::new_with_context(ctx_b);
        assert_ne!(id(&a), id(&b));
    }

    #[test]
    fn options_identity_clone_stable_across_volatile_context() {
        // Invariant: the context is captured once and cloned into every
        // `ComposeOptions::clone`, so a complete encoding stays clone-stable
        // even though it now includes volatile fields.
        let ctx =
            ComposeContext::fixed_for_testing_with([("timestamp", serde_json::json!("1000"))]);
        let opts = ComposeOptions::new_with_context(ctx);
        assert_eq!(id(&opts), id(&opts.clone()));
    }

    #[test]
    fn options_identity_clone_stable_including_shared_stateful_arc() {
        let handler: Arc<dyn ShellApprovalHandlerTrait> = Arc::new(DummyApproval);
        let opts = fixed_opts().with_shell_approval_handler(handler);
        // A clone shares the same `Arc`, so identity is stable.
        assert_eq!(id(&opts), id(&opts.clone()));
    }

    #[test]
    fn options_identity_unequal_for_fresh_stateful_instance() {
        let handler_a: Arc<dyn ShellApprovalHandlerTrait> = Arc::new(DummyApproval);
        let handler_b: Arc<dyn ShellApprovalHandlerTrait> = Arc::new(DummyApproval);
        let a = fixed_opts().with_shell_approval_handler(handler_a);
        let b = fixed_opts().with_shell_approval_handler(handler_b);
        // Distinct instances with identical visible configuration are not equal.
        assert_ne!(id(&a), id(&b));
    }

    #[test]
    fn options_identity_rejects_dropped_then_recreated_instance() {
        let captured = {
            let handler: Arc<dyn ShellApprovalHandlerTrait> = Arc::new(DummyApproval);
            let opts = fixed_opts().with_shell_approval_handler(Arc::clone(&handler));
            let strong_before = Arc::strong_count(&handler);
            let identity = id(&opts);
            // Identity capture and graph ownership do not add strong references.
            assert_eq!(Arc::strong_count(&handler), strong_before);
            identity
            // `opts` and `handler` drop here.
        };

        // Allocator churn, then a fresh instance with identical configuration.
        let _churn: Vec<Box<[u8]>> = (0..64).map(|_| vec![0u8; 128].into_boxed_slice()).collect();
        let fresh: Arc<dyn ShellApprovalHandlerTrait> = Arc::new(DummyApproval);
        let fresh_opts = fixed_opts().with_shell_approval_handler(fresh);
        assert_ne!(captured, id(&fresh_opts));
    }

    #[test]
    fn options_identity_clone_stable_with_shared_remote_fetch_runtime() {
        let opts = fixed_opts().with_shared_remote_fetch();
        // A clone shares the same runtime `Arc`, so identity is stable; a
        // separately built runtime would not be.
        assert_eq!(id(&opts), id(&opts.clone()));
        let other = fixed_opts().with_shared_remote_fetch();
        assert_ne!(id(&opts), id(&other));
    }

    #[test]
    fn options_identity_preflight_weak_does_not_extend_lifetime() {
        use super::super::super::preflight::PreflightGraphNode;

        // Own the preflight `Arc` externally so its strong count is observable
        // (the public `with_preflight_graph` builder wraps a private `Arc`).
        let preflight = Arc::new(PreflightGraphNode::default());
        let strong_before = Arc::strong_count(&preflight);
        let mut opts = fixed_opts();
        opts.preflight_graph = Some(Arc::clone(&preflight));
        let strong_with_opts = Arc::strong_count(&preflight);

        let captured = id(&opts);
        // Identity capture downgrades the preflight `Arc` → no new strong ref.
        assert_eq!(Arc::strong_count(&preflight), strong_with_opts);

        drop(opts);
        // The captured identity holds only a `Weak`, so releasing the owning
        // options returns the strong count to baseline — graph provenance never
        // pins the preflight instance alive.
        assert_eq!(Arc::strong_count(&preflight), strong_before);

        // The now-dead weak can never match a fresh preflight (fail-closed).
        let mut fresh = fixed_opts();
        fresh.preflight_graph = Some(Arc::new(PreflightGraphNode::default()));
        assert_ne!(captured, id(&fresh));
    }

    #[test]
    fn options_identity_rejects_dropped_then_recreated_remote_fetch() {
        let captured = {
            let opts = fixed_opts().with_shared_remote_fetch();
            id(&opts)
            // `opts`, the sole owner of the runtime, drops here — expiring the
            // captured weak identity handle.
        };
        // Allocator churn, then a fresh runtime with identical configuration.
        let _churn: Vec<Box<[u8]>> = (0..64).map(|_| vec![0u8; 128].into_boxed_slice()).collect();
        let fresh = fixed_opts().with_shared_remote_fetch();
        // A fresh runtime built after the build-time one dropped can never match
        // the expired handle, exercising the drop path the two-live-instances
        // test does not.
        assert_ne!(captured, id(&fresh));
    }

    #[test]
    fn cache_fingerprint_shares_classification_and_matches_options_hash() {
        // Value-equivalent options (unordered-set order aside) share the cache
        // fingerprint, and the fingerprint is exactly what `options_hash`
        // delegates to.
        let a = ComposeOptions::new().with_exclude_keys(["x", "y"]);
        let b = ComposeOptions::new().with_exclude_keys(["y", "x"]);
        // `exclude_keys` is not part of the cache fingerprint, so both match.
        assert_eq!(fp(&a), fp(&b));
        assert_eq!(fp(&a), super::super::super::cache::hashing::options_hash(&req(a)));
    }

    /// The cached-default fast path must be a pure speedup: an identical schema
    /// supplied by hand (marker cleared, uncached arm) must fingerprint exactly
    /// as the marked default does. If `with_darkmatter_baseline_schema` ever
    /// stops assigning `darkmatter_base_schema()`, the marker would start lying
    /// about which schema is encoded and this fails.
    #[test]
    fn default_baseline_fast_path_fingerprints_as_the_uncached_schema() {
        let marked = fixed_opts().with_darkmatter_baseline_schema();
        let by_hand = fixed_opts().with_baseline_schema(crate::markdown::schemas::darkmatter_base_schema());

        assert!(marked.baseline_is_darkmatter_default);
        assert!(!by_hand.baseline_is_darkmatter_default);

        // The marker itself is encoded into the graph identity, so only the
        // cache fingerprint (which omits it) can compare the schema encoding.
        assert_eq!(
            fp(&marked),
            fp(&by_hand),
            "cached default encoding must equal the freshly converted one"
        );
        assert_eq!(
            baseline_canonical_json(&crate::markdown::schemas::darkmatter_base_schema(), true),
            baseline_canonical_json(&crate::markdown::schemas::darkmatter_base_schema(), false),
            "cached and uncached arms must produce identical canonical bytes"
        );
    }

    /// A non-default baseline must not collide with the cached default.
    #[test]
    fn non_default_baseline_does_not_read_the_default_cache() {
        let default = fixed_opts().with_darkmatter_baseline_schema();
        let custom = fixed_opts().with_baseline_schema(
            crate::markdown::schemas::parse_yaml_schema(
                &serde_yaml_ng::from_str::<serde_yaml_ng::Value>("zzz_probe: string").unwrap(),
            )
            .unwrap(),
        );
        assert_ne!(fp(&default), fp(&custom));
    }
}
