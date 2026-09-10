//! Prompt preparation for composition workflows.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::hash::MdHashKind;
use darkmatter::markdown::{Markdown, MarkdownError};

use super::json_util::json_type_name;

/// Convert a `MarkdownError` into a `CompositionError`, preserving the
/// structured `ShellExpansion` variant so the CLI can render rich errors.
///
/// Shell errors route to the `ShellExpansionFailed` variant so the CLI's
/// renderer has the composing file's path available alongside the structured
/// error. All other `MarkdownError` variants ride through `ComposeFailed`
/// while retaining their typed source in the error chain (via `#[source]`)
/// so the top-level `as_block_error` walker can still produce a rich
/// `BlockError` report.
fn map_compose_error(source_path: &std::path::Path, err: MarkdownError) -> CompositionError {
    match err {
        MarkdownError::ShellExpansion(shell_err) => CompositionError::ShellExpansionFailed {
            source_path: source_path.to_path_buf(),
            error: shell_err,
        },
        other => CompositionError::ComposeFailed(other),
    }
}

/// Bind the agent's workspace onto compose options: the source file (for
/// `::file` transclusion and diagnostic spans) and, when known, the directory
/// the dispatched agent will run in.
///
/// Without `shell_cwd`, Darkmatter defaults `::shell` execution to the source
/// file's parent directory — wrong for `@`-resolved prompts that physically
/// live under `~/.claudine/prompts/` but reason about the repo the agent runs
/// in. Pinning the working directory keeps compose-time shell and the agent on
/// the same root.
pub fn bind_agent_workspace(
    opts: ComposeOptions,
    source_path: &Path,
    shell_cwd: Option<&Path>,
) -> ComposeOptions {
    let opts = opts.with_source_file(source_path);
    match shell_cwd {
        Some(cwd) => opts.with_shell_working_directory(cwd),
        None => opts,
    }
}

mod entry;
mod service;

pub use entry::{DocumentEntryReason, LoopOwnership, PreparationStages, SourceBasis};
pub use service::{DocumentPreparation, PromptSource, SchemaStage, prepare_document};

/// Options for composition preparation.
#[derive(Debug, Default, Clone)]
pub struct PrepareOptions {
    /// Request owner used for invocation-local composition work accounting.
    ///
    /// Canonical command paths supply this alongside `prepared_context`.
    /// Compatibility callers may omit it without changing composition
    /// behavior.
    pub invocation_context: Option<crate::invocation_context::InvocationContext>,
    /// Intrinsically attributable work recorder for this document epoch.
    ///
    /// Canonical command paths supply this with `prepared_context`; library
    /// compatibility callers may omit both.
    pub document_epoch: Option<crate::invocation_context::DocumentEpoch>,
    /// Frontmatter `--set` overrides (JSON object).
    pub set_overrides: Option<serde_json::Value>,
    /// Immutable raw caller overrides paired with their launch-time origins.
    pub caller_input_records: darkmatter::markdown::compose::CallerInputRecords,
    /// Commands pre-approved during pre-flight shell discovery.
    pub pre_approved_commands: Option<std::collections::HashSet<String>>,
    /// Extra environment variables to inject into the composition context.
    pub env_overrides: BTreeMap<String, String>,
    /// Enable Darkmatter composition performance collection.
    pub perf_enabled: bool,
    /// Pre-computed source repo root.
    ///
    /// When `Some`, [`prepare_direct`] and [`prepare_inline`] use this value
    /// instead of walking up from the source path. CLI callers populate this
    /// from a shared `CompositionPrepContext` so the same repo-root value is
    /// reused across eager target resolution, shell preflight, and
    /// composition preparation. When `None`, the fallback walk
    /// (`find_git_root_from_path`) preserves the original behavior for
    /// library-only callers and tests.
    pub source_repo_root: Option<PathBuf>,
    /// Directory the dispatched agent will run in.
    ///
    /// When `Some`, `::shell` directives execute here instead of the prompt
    /// file's parent, keeping compose-time shell expansion and the agent on
    /// one working directory. CLI callers populate this from
    /// `CompositionPrepContext::launch_workspace.child_cwd`. `None` (the
    /// default) preserves Darkmatter's source-relative fallback for
    /// library-only callers and tests.
    pub shell_working_directory: Option<PathBuf>,
    /// Pre-captured early-binding context snapshot.
    ///
    /// When `Some`, [`prepare_direct`]/[`prepare_inline`] compose and run shell
    /// preflight against this exact snapshot, so the body and the lifecycle
    /// reuse one `ctx.*`/`env.*` capture rooted at the launch area.
    /// `env_overrides` are still applied to it. When `None`,
    /// [`derive_compose_context`] captures one anchored on the launch area (or
    /// the document's own directory) — never on the process CWD, which the
    /// wrapper deliberately mutates to the repo root before any prepared path
    /// runs.
    ///
    /// Whichever way it is obtained, the exact snapshot used is stored on
    /// [`PreparedComposition::compose_context`] (R5).
    pub prepared_context: Option<ComposeContext>,
    /// Explicit fallback directory for caller-supplied file references.
    ///
    /// When `Some`, body interpolation and schema validation resolve
    /// caller-supplied paths (e.g. a CLI-supplied `spec`) against this
    /// directory after the document dir misses, so prepare-time resolution
    /// is independent of the ambient process CWD and agrees with event-time
    /// resolution. CLI callers populate this from
    /// `CompositionPrepContext::launch_workspace.launch_cwd`. `None` (the
    /// default) preserves the legacy ambient-CWD behavior for library-only
    /// callers and tests.
    pub file_ref_fallback_dir: Option<PathBuf>,
    /// Immutable request-scoped file-resolution snapshot shared with
    /// Darkmatter and retained across harness re-materialization.
    pub file_resolution_context: Option<biscuit_file::FileResolutionContext>,
    /// Frontmatter keys whose object values render their `name` field in inline
    /// string context (`{{state}}` → the name). Sequence preparation sets this
    /// to the reserved overlay keys (`state`/`previous`/`next`); every other
    /// caller leaves it empty, so this is a no-op for `compose`/`inline-compose`.
    pub name_coercion_keys: Vec<String>,
    /// Accept a composition whose body is empty instead of raising
    /// [`CompositionError::ComposedBodyEmpty`].
    ///
    /// Set only by a sequence step that declares an executable field: such a
    /// step runs its task *instead of* the document body, and a directly
    /// invoked `kind: sequence` YAML file has no body at all. Every other
    /// caller leaves this `false`, because an empty prompt reaching a provider
    /// is otherwise a bug the provider would report far less usefully.
    pub allow_empty_body: bool,
    /// Compose without reaching this document's schema verdict.
    ///
    /// Set by the stages that read a document they do not judge: the
    /// shell-discovery pre-flight, and the bootstrap read taken *before* a
    /// document's own `initialize` runs. `initialize` may add or repair a
    /// schema property, so a verdict reached before it would fail the document
    /// for a violation the next stage is about to fix (R4). Frontmatter is
    /// still coerced to the declared types; only the verdict is withheld, and
    /// the post-`initialize` stabilized reread reports it.
    pub defer_schema_verdict: bool,
}

/// The early-binding snapshot a canonical preparation composes against.
///
/// Returns the caller's snapshot when it supplied one; otherwise captures a
/// fresh one anchored on the caller's launch area, falling back to the
/// document's own directory.
///
/// Canonical command paths always supply the invocation-owned snapshot. The
/// capture branch preserves compatibility for library callers.
///
/// The anchor is never the process CWD. The wrapper changes the parent CWD to
/// the repo root before dispatch by design, so an ambient capture on a prepared
/// path would resolve `ctx.area` against whatever the process was last pointed
/// at rather than where the caller launched — and would answer differently
/// depending on *when* it ran. Anchoring on immutable launch inputs plus the
/// target's own location makes the snapshot a function of its arguments.
fn derive_compose_context(
    source: &ResolvedCompositionSource,
    options: &PrepareOptions,
) -> ComposeContext {
    if let Some(prepared) = options.prepared_context.clone() {
        return prepared;
    }
    if let Some(invocation) = options.invocation_context.as_ref() {
        if let Some(epoch) = options.document_epoch.as_ref() {
            epoch.record_ambient_fallback();
        } else {
            invocation.record_ambient_fallback();
        }
    }
    let anchor = options
        .file_ref_fallback_dir
        .clone()
        .or_else(|| source.resolved_path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    // Demand-driven over this document's frontmatter and body, so a document
    // that never mentions `ctx.*` pays for no host scan.
    ComposeContext::capture_for_document(&anchor, &source.markdown)
}

fn observe_prepared_context(
    options: &PrepareOptions,
    consumer: crate::invocation_context::PreparedContextConsumer,
) {
    if options.prepared_context.is_some() {
        if let Some(epoch) = options.document_epoch.as_ref() {
            epoch.record_prepared_context_consumer(consumer);
        } else if let Some(invocation) = options.invocation_context.as_ref() {
            invocation.record_prepared_context_consumer(consumer);
        }
    }
}

/// The one `ComposeOptions` shape every canonical preparation stage composes
/// with.
///
/// Body preparation, inline preparation, and the proxy target's pre-flight
/// shell audit all build their options here. They used to build them
/// separately, three times, and the copies disagreed — the audit discovered
/// commands against one option set while the compose that executed them used
/// another.
fn canonical_compose_options(
    source_path: &Path,
    ctx: &ComposeContext,
    options: &PrepareOptions,
) -> ComposeOptions {
    let mut compose_opts = bind_agent_workspace(
        ComposeOptions::new_with_context(ctx.clone()),
        source_path,
        options.shell_working_directory.as_deref(),
    )
    .with_perf(options.perf_enabled)
    // Defer the seven lifecycle event subtrees so their authored `{{ }}`
    // spans survive raw in `effective_frontmatter` for event-time
    // interpolation (C1). Non-lifecycle keys compose as today, so
    // variable *values* (`phase`, `pass_icon`, …) are composed before
    // launch and may still be mutated by lifecycle/loop side effects
    // during the run.
    .with_exclude_keys(LIFECYCLE_EVENT_KEYS.iter().copied())
    // The composed body is delivered verbatim to the agent and reported as the
    // user prompt. Darkmatter's default strips incidental single newlines, which
    // would collapse an author's line-structured prompt into one paragraph —
    // altering the delivered text and defeating line-count-based report
    // truncation. Preserve the source line breaks for prompt delivery.
    .with_incidental_newline_mode(darkmatter::markdown::cleanup::IncidentalNewlineMode::Preserve);
    compose_opts = compose_opts.with_set_overrides(
        super::runtime_state::with_initialized_outputs(options.set_overrides.clone()),
    );
    compose_opts = compose_opts.with_caller_input_records(options.caller_input_records.clone());
    if !options.name_coercion_keys.is_empty() {
        compose_opts = compose_opts.with_name_coercion_keys(options.name_coercion_keys.clone());
    }
    if let Some(approved) = options.pre_approved_commands.clone() {
        compose_opts = compose_opts.with_pre_approved_commands(approved);
    }
    if let Some(fallback) = options.file_ref_fallback_dir.clone() {
        compose_opts = compose_opts.with_file_ref_fallback_dir(fallback);
    }
    if let Some(context) = options.file_resolution_context.clone() {
        compose_opts = compose_opts.with_file_resolution_context(context);
    }
    compose_opts.with_deferred_schema_verdict(options.defer_schema_verdict)
}

/// Discover and approve one document's own compose-time shell surfaces, ahead
/// of preparing it.
///
/// A `proxy` hand-off is a fresh prompt run, pre-flight included: the target's
/// frontmatter `$(...)` and template `::shell` commands are the target's own,
/// and the source's pre-flight never saw them. Without this the target's
/// preparation expands those commands against the source's approved set and
/// fails `NotPreApproved` — even for commands the audit would auto-approve.
///
/// Returns the commands the audit approved, for the caller to fold into the
/// input layers it will prepare with.
///
/// ## Errors
///
/// Propagates a shell-audit denial (blacklisted command, denied approval, or —
/// once the approval handler is frozen — an un-whitelisted, un-cached command).
/// A discovery-walk failure is **not** a denial: there are no commands to
/// approve, and the preparation that follows surfaces the authoritative typed
/// error (e.g. a styled `TransclusionError`), so it returns an empty set rather
/// than preempting with a coarser one.
pub fn preflight_document_shell(
    source: &ResolvedCompositionSource,
    options: &PrepareOptions,
    approval_options: &crate::harness::ShellApprovalOptions,
) -> Result<std::collections::HashSet<String>, CompositionError> {
    observe_prepared_context(
        options,
        crate::invocation_context::PreparedContextConsumer::Preflight,
    );
    let ctx = derive_compose_context(source, options);
    let compose_opts = canonical_compose_options(&source.resolved_path, &ctx, options);
    match super::resolve_shell_approvals(
        Some(&source.markdown),
        Some(&compose_opts),
        approval_options,
        None,
        None,
    ) {
        Ok(result) => Ok(result.approved_commands),
        Err(CompositionError::PreFlightDiscoveryFailed(_)) => {
            Ok(std::collections::HashSet::new())
        }
        Err(e) => Err(e),
    }
}

/// Walk up from a file path to find the nearest `.git` directory.
fn find_git_root_from_path(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() { path.parent()? } else { path };
    let mut dir = start;
    loop {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

fn effective_source_repo_root(
    configured: Option<PathBuf>,
    file_resolution_context: Option<&biscuit_file::FileResolutionContext>,
    source_path: &Path,
) -> Option<PathBuf> {
    configured.or_else(|| {
        file_resolution_context
            .is_none()
            .then(|| find_git_root_from_path(source_path))
            .flatten()
    })
}

use super::error::CompositionError;
use super::guardrails::load_or_create_guardrails;
use super::lifecycle::{
    LIFECYCLE_EVENT_KEYS, parse_lifecycle_config, validate_no_err_in_no_error_events,
};
use super::hints::{ParsedAgentHint, parse_agent_hint_full, parse_interactive_hint, parse_model_hint};
use super::types::{
    CompositionClosurePlan, CompositionMode, EffectiveSelectionHints, InlineClosurePlan,
    PreparedComposition, ResolvedCompositionSource,
};
/// Prepare a direct (chained) composition with effective frontmatter.
///
/// Composes the entire document through Darkmatter and extracts the
/// effective frontmatter from the composed state. The closure is
/// [`CompositionClosurePlan::Direct`] — no file mutation occurs.
pub fn prepare_direct(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
) -> Result<PreparedComposition, CompositionError> {
    prepare_direct_with_prompt(source, options, PromptSource::ComposedBody)
}

/// [`prepare_direct`] with an explicit prompt-source stage.
///
/// Split out for the canonical service's passthrough entry, where the document
/// is composed for its effective frontmatter but the prompt came from argv or
/// stdin.
pub(super) fn prepare_direct_with_prompt(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    prompt_source: PromptSource,
) -> Result<PreparedComposition, CompositionError> {
    let override_keys = top_level_override_keys(options.set_overrides.as_ref());
    if let Some((key, replacement)) =
        super::lifecycle::scan_removed_validation_keys(&frontmatter_to_value(source.markdown.frontmatter()))
    {
        return Err(CompositionError::RemovedValidationKey {
            source_path: source.resolved_path.clone(),
            key,
            replacement: replacement.to_string(),
        });
    }
    if matches!(&prompt_source, PromptSource::ComposedBody) {
        observe_prepared_context(
            &options,
            crate::invocation_context::PreparedContextConsumer::Body,
        );
    }
    observe_prepared_context(
        &options,
        crate::invocation_context::PreparedContextConsumer::EffectiveFrontmatter,
    );
    // Reuse the single composition-start snapshot when the caller supplied one
    // (so body, preflight, and lifecycle share one `ctx.*`/`env.*` capture);
    // otherwise derive one from the launch anchor.
    let mut ctx = derive_compose_context(source, &options);
    for (key, value) in &options.env_overrides {
        ctx.env_mut().insert(key.clone(), value.clone());
    }
    let compose_opts = canonical_compose_options(&source.resolved_path, &ctx, &options);
    // Retain the caller's inputs so a later canonical preparation of this or a
    // proxied document re-applies exactly them.
    let input_layers = super::CallerInputLayers::from_options(&options);
    if let Some(invocation) = options.invocation_context.as_ref() {
        invocation.record_compose_operation();
    }
    let (composed, report) = source
        .markdown
        .compose_with(compose_opts)
        .map_err(|e| map_compose_error(&source.resolved_path, e))?;

    let prompt = match prompt_source {
        PromptSource::ComposedBody => {
            let prompt = composed.content().to_string();
            if prompt.trim().is_empty() && !options.allow_empty_body {
                return Err(CompositionError::ComposedBodyEmpty {
                    source_path: source.resolved_path.clone(),
                    mode: CompositionMode::ChainedDocument,
                    provided_overrides: override_keys,
                });
            }
            prompt
        }
        // The composed body is not the prompt here, so its emptiness says
        // nothing about whether the caller supplied a request.
        PromptSource::Supplied(prompt) => prompt,
    };

    let effective_frontmatter = frontmatter_to_value(composed.frontmatter());
    if let Some((key, replacement)) =
        super::lifecycle::scan_removed_validation_keys(&effective_frontmatter)
    {
        return Err(CompositionError::RemovedValidationKey {
            source_path: source.resolved_path.clone(),
            key,
            replacement: replacement.to_string(),
        });
    }
    let agent_full = composed
        .frontmatter()
        .as_map()
        .get("agent")
        .map_or(Ok(ParsedAgentHint::default()), parse_agent_hint_full)?;
    let agent_hint = agent_full.to_agent_hint();
    let model_hint = composed
        .frontmatter()
        .as_map()
        .get("model")
        .map_or(Ok(None), parse_model_hint)?;
    let interactive_hint = composed
        .frontmatter()
        .as_map()
        .get("interactive")
        .map_or(Ok(None), parse_interactive_hint)?;
    let selection_hints = EffectiveSelectionHints {
        agent: agent_hint,
        model: model_hint,
        interactive: interactive_hint,
        agent_invalid: agent_full.invalid,
        agent_was_list: agent_full.is_list,
    };
    let mut lifecycle =
        parse_lifecycle_config(&effective_frontmatter, &source.resolved_path)?;
    // Pre-flight shell resolution (C3): resolve each shell command in the
    // deferred lifecycle subtree via DM2 with an early-binding-only lookup
    // and stamp the resolved bytes back so the approved command equals the
    // executed command. Late-binding references (`err`/`timing`/`current`)
    // are rejected with a typed error.
    super::preflight::resolve_lifecycle_shell_commands(
        &mut lifecycle,
        &effective_frontmatter,
        &ctx,
        &source.resolved_path,
        options.file_resolution_context.as_ref(),
        options.file_ref_fallback_dir.as_deref(),
    )?;
    // Lifecycle communication/action strings are deferred by design (C1): they
    // keep their `{{ }}` spans through prepare and resolve at event-time via
    // DM2 (C2), where strict mode fails closed on undefined roots and malformed
    // expressions, and the post-DM2 dispatch-time leak guard (C4) backstops a
    // surviving span before any side effect is sent. The prepare-time leak and
    // undefined-variable scans therefore no longer run over these deferred
    // strings — they would flag the authored spans as bugs. The `err`-placement
    // scan stays: a bare `err` in a no-error event is invalid regardless of
    // binding time.
    validate_no_err_in_no_error_events(&lifecycle, &source.resolved_path)?;

    let source_repo_root = effective_source_repo_root(
        options.source_repo_root,
        options.file_resolution_context.as_ref(),
        &source.resolved_path,
    );

    Ok(PreparedComposition {
        mode: CompositionMode::ChainedDocument,
        entry: DocumentEntryReason::Direct,
        schema_verdict_deferred: options.defer_schema_verdict,
        resolved_path: source.resolved_path.clone(),
        source_repo_root,
        prompt,
        effective_frontmatter,
        selection_hints,
        closure: CompositionClosurePlan::Direct,
        lifecycle,
        deferred_lifecycle_keys: sorted_deferred_keys(&report),
        compose_perf: report.perf,
        dropped_optionals: Vec::new(),
        warnings: report.warnings.clone(),
        input_layers,
        compose_context: ctx,
        document_epoch: options.document_epoch,
    })
}

/// Prepare an inline composition with effective frontmatter.
///
/// Extracts the `prompt` frontmatter property, builds a temporary
/// document, composes through Darkmatter, and captures closure state
/// for deterministic post-execution rewrite.
pub fn prepare_inline(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
) -> Result<PreparedComposition, CompositionError> {
    let override_keys = top_level_override_keys(options.set_overrides.as_ref());
    let fm = source.markdown.frontmatter();
    if let Some((key, replacement)) =
        super::lifecycle::scan_removed_validation_keys(&frontmatter_to_value(fm))
    {
        return Err(CompositionError::RemovedValidationKey {
            source_path: source.resolved_path.clone(),
            key,
            replacement: replacement.to_string(),
        });
    }

    let prompt_value = fm
        .as_map()
        .get("prompt")
        .ok_or(CompositionError::PromptPropertyMissing)?;

    let prompt_text = match prompt_value {
        serde_json::Value::String(s) => s.clone(),
        other => {
            return Err(CompositionError::PromptPropertyWrongType(
                json_type_name(other).to_string(),
            ));
        }
    };

    observe_prepared_context(
        &options,
        crate::invocation_context::PreparedContextConsumer::Body,
    );
    observe_prepared_context(
        &options,
        crate::invocation_context::PreparedContextConsumer::EffectiveFrontmatter,
    );
    // Build temporary markdown (frontmatter + prompt as body) and compose
    let temp_md = Markdown::with_frontmatter(fm.clone(), &prompt_text);
    // Reuse the single composition-start snapshot when supplied; see
    // `prepare_direct`.
    let mut ctx = derive_compose_context(source, &options);
    for (key, value) in &options.env_overrides {
        ctx.env_mut().insert(key.clone(), value.clone());
    }
    // Retain the composed context so pre-flight shell resolution (C3) can build
    // an early-binding lookup over the same `ctx.*`/`env.*` state main compose saw.
    let compose_opts = canonical_compose_options(&source.resolved_path, &ctx, &options);
    let input_layers = super::CallerInputLayers::from_options(&options);
    if let Some(invocation) = options.invocation_context.as_ref() {
        invocation.record_compose_operation();
    }
    let (composed, report) = temp_md
        .compose_with(compose_opts)
        .map_err(|e| map_compose_error(&source.resolved_path, e))?;

    let effective_frontmatter = frontmatter_to_value(composed.frontmatter());
    if let Some((key, replacement)) =
        super::lifecycle::scan_removed_validation_keys(&effective_frontmatter)
    {
        return Err(CompositionError::RemovedValidationKey {
            source_path: source.resolved_path.clone(),
            key,
            replacement: replacement.to_string(),
        });
    }
    let agent_full = composed
        .frontmatter()
        .as_map()
        .get("agent")
        .map_or(Ok(ParsedAgentHint::default()), parse_agent_hint_full)?;
    let agent_hint = agent_full.to_agent_hint();
    let model_hint = composed
        .frontmatter()
        .as_map()
        .get("model")
        .map_or(Ok(None), parse_model_hint)?;
    let interactive_hint = composed
        .frontmatter()
        .as_map()
        .get("interactive")
        .map_or(Ok(None), parse_interactive_hint)?;
    let selection_hints = EffectiveSelectionHints {
        agent: agent_hint,
        model: model_hint,
        interactive: interactive_hint,
        agent_invalid: agent_full.invalid,
        agent_was_list: agent_full.is_list,
    };
    let mut lifecycle =
        parse_lifecycle_config(&effective_frontmatter, &source.resolved_path)?;
    // Pre-flight shell resolution (C3): see `prepare_direct`.
    super::preflight::resolve_lifecycle_shell_commands(
        &mut lifecycle,
        &effective_frontmatter,
        &ctx,
        &source.resolved_path,
        options.file_resolution_context.as_ref(),
        options.file_ref_fallback_dir.as_deref(),
    )?;
    // Deferred lifecycle strings resolve at event-time (C2); the prepare-time
    // leak / undefined-variable scans do not run over them. See `prepare_direct`.
    validate_no_err_in_no_error_events(&lifecycle, &source.resolved_path)?;

    let mut prompt = composed.content().to_string();
    if prompt.trim().is_empty() && !options.allow_empty_body {
        return Err(CompositionError::ComposedBodyEmpty {
            source_path: source.resolved_path.clone(),
            mode: CompositionMode::InlineFrontmatterPrompt,
            provided_overrides: override_keys,
        });
    }

    let source_repo_root = effective_source_repo_root(
        options.source_repo_root,
        options.file_resolution_context.as_ref(),
        &source.resolved_path,
    );

    // Append guardrails with the new inline contract
    let guardrails = load_or_create_guardrails(source_repo_root.as_deref());
    prompt.push_str("\n\n");
    prompt.push_str(&guardrails);

    // Capture pre-execution hash for closure
    let original_hash = source.markdown.compute_hash(
        MdHashKind::Simple,
        &super::closure::inline_hash_options(),
    );

    Ok(PreparedComposition {
        mode: CompositionMode::InlineFrontmatterPrompt,
        entry: DocumentEntryReason::Direct,
        schema_verdict_deferred: options.defer_schema_verdict,
        resolved_path: source.resolved_path.clone(),
        source_repo_root,
        prompt,
        effective_frontmatter,
        selection_hints,
        closure: CompositionClosurePlan::Inline(InlineClosurePlan {
            original_document_text: source.original_text.clone(),
            original_hash,
        }),
        lifecycle,
        deferred_lifecycle_keys: sorted_deferred_keys(&report),
        compose_perf: report.perf,
        dropped_optionals: Vec::new(),
        warnings: report.warnings.clone(),
        input_layers,
        compose_context: ctx,
        document_epoch: options.document_epoch,
    })
}

/// Convert a `Frontmatter` to a `serde_json::Value::Object`.
fn frontmatter_to_value(fm: &darkmatter::markdown::Frontmatter) -> serde_json::Value {
    serde_json::Value::Object(
        fm.as_map()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    )
}

/// Collect Darkmatter's intentionally-deferred (DM1) lifecycle keys from a
/// compose report into a stable, sorted list for dry-run labeling (C5).
///
/// The report already limits the set to keys present in the source
/// frontmatter; sorting makes the dry-run output deterministic.
fn sorted_deferred_keys(report: &darkmatter::markdown::compose::ComposeReport) -> Vec<String> {
    let mut keys: Vec<String> = report.deferred_frontmatter_keys.iter().cloned().collect();
    keys.sort();
    keys
}

/// Extract the top-level keys from a `set_overrides` JSON object, in the
/// underlying `serde_json::Map` iteration order (alphabetical without the
/// `preserve_order` feature). Returns an empty `Vec` when overrides are
/// absent or the value is not an object. Used to surface "what the user
/// provided" in the `ComposedBodyEmpty` error so the diagnostic can name
/// the variables that were visible to `::block when=…` conditions.
fn top_level_override_keys(overrides: Option<&serde_json::Value>) -> Vec<String> {
    match overrides {
        Some(serde_json::Value::Object(map)) => map.keys().cloned().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
