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

/// Options for composition preparation.
#[derive(Debug, Default, Clone)]
pub struct PrepareOptions {
    /// Frontmatter `--set` overrides (JSON object).
    pub set_overrides: Option<serde_json::Value>,
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
    /// preflight against this exact snapshot instead of calling
    /// [`ComposeContext::capture`], so the body and the lifecycle reuse one
    /// `ctx.*`/`env.*` capture rooted at the launch area. `env_overrides` are
    /// still applied to it. `None` (the default) preserves the
    /// capture-at-prepare behavior for library-only callers and tests.
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
    /// Frontmatter keys whose object values render their `name` field in inline
    /// string context (`{{state}}` → the name). Sequence preparation sets this
    /// to the reserved overlay keys (`state`/`previous`/`next`); every other
    /// caller leaves it empty, so this is a no-op for `compose`/`inline-compose`.
    pub name_coercion_keys: Vec<String>,
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
    // Reuse the single composition-start snapshot when the caller supplied one
    // (so body, preflight, and lifecycle share one `ctx.*`/`env.*` capture);
    // otherwise capture now for library-only callers and tests.
    let mut ctx = options
        .prepared_context
        .clone()
        .unwrap_or_else(ComposeContext::capture);
    for (key, value) in &options.env_overrides {
        ctx.env_mut().insert(key.clone(), value.clone());
    }
    // Retain the composed context so pre-flight shell resolution (C3) can build
    // an early-binding lookup over the same `ctx.*`/`env.*` state main compose saw.
    let mut compose_opts = bind_agent_workspace(
        ComposeOptions::new_with_context(ctx.clone()),
        &source.resolved_path,
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
    .with_incidental_newline_mode(
        darkmatter::markdown::cleanup::IncidentalNewlineMode::Preserve,
    );
    // Retain the caller's inputs so the harness loop can re-apply them when it
    // re-composes this document after a `retry`/`resume`/`proxy` re-entry (they
    // are otherwise consumed into `compose_opts` below).
    let rematerialize = super::RematerializeInputs {
        set_overrides: options.set_overrides.clone(),
        file_ref_fallback_dir: options.file_ref_fallback_dir.clone(),
        pre_approved_commands: options.pre_approved_commands.clone(),
        env_overrides: options.env_overrides.clone(),
    };
    if let Some(overrides) = options.set_overrides {
        compose_opts = compose_opts.with_set_overrides(overrides);
    }
    if !options.name_coercion_keys.is_empty() {
        compose_opts = compose_opts.with_name_coercion_keys(options.name_coercion_keys.clone());
    }
    if let Some(approved) = options.pre_approved_commands {
        compose_opts = compose_opts.with_pre_approved_commands(approved);
    }
    if let Some(fallback) = options.file_ref_fallback_dir.clone() {
        compose_opts = compose_opts.with_file_ref_fallback_dir(fallback);
    }
    let (composed, report) = source
        .markdown
        .compose_with(compose_opts)
        .map_err(|e| map_compose_error(&source.resolved_path, e))?;

    let prompt = composed.content().to_string();
    if prompt.trim().is_empty() {
        return Err(CompositionError::ComposedBodyEmpty {
            source_path: source.resolved_path.clone(),
            mode: CompositionMode::ChainedDocument,
            provided_overrides: override_keys,
        });
    }

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

    let source_repo_root = options
        .source_repo_root
        .or_else(|| find_git_root_from_path(&source.resolved_path));

    Ok(PreparedComposition {
        mode: CompositionMode::ChainedDocument,
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
        rematerialize,
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

    // Build temporary markdown (frontmatter + prompt as body) and compose
    let temp_md = Markdown::with_frontmatter(fm.clone(), &prompt_text);
    // Reuse the single composition-start snapshot when supplied; see
    // `prepare_direct`.
    let mut ctx = options
        .prepared_context
        .clone()
        .unwrap_or_else(ComposeContext::capture);
    for (key, value) in &options.env_overrides {
        ctx.env_mut().insert(key.clone(), value.clone());
    }
    // Retain the composed context so pre-flight shell resolution (C3) can build
    // an early-binding lookup over the same `ctx.*`/`env.*` state main compose saw.
    let mut compose_opts = bind_agent_workspace(
        ComposeOptions::new_with_context(ctx.clone()),
        &source.resolved_path,
        options.shell_working_directory.as_deref(),
    )
    .with_perf(options.perf_enabled)
    // Defer the seven lifecycle event subtrees so their authored `{{ }}`
    // spans survive raw in `effective_frontmatter` for event-time
    // interpolation (C1). Non-lifecycle keys compose as today.
    .with_exclude_keys(LIFECYCLE_EVENT_KEYS.iter().copied())
    // The composed body is delivered verbatim to the agent and reported as the
    // user prompt. Darkmatter's default strips incidental single newlines, which
    // would collapse an author's line-structured prompt into one paragraph —
    // altering the delivered text and defeating line-count-based report
    // truncation. Preserve the source line breaks for prompt delivery.
    .with_incidental_newline_mode(
        darkmatter::markdown::cleanup::IncidentalNewlineMode::Preserve,
    );
    // Retain the caller's inputs for faithful re-materialization on a
    // `retry`/`resume`/`proxy` re-entry (see `prepare_direct`).
    let rematerialize = super::RematerializeInputs {
        set_overrides: options.set_overrides.clone(),
        file_ref_fallback_dir: options.file_ref_fallback_dir.clone(),
        pre_approved_commands: options.pre_approved_commands.clone(),
        env_overrides: options.env_overrides.clone(),
    };
    if let Some(overrides) = options.set_overrides {
        compose_opts = compose_opts.with_set_overrides(overrides);
    }
    if !options.name_coercion_keys.is_empty() {
        compose_opts = compose_opts.with_name_coercion_keys(options.name_coercion_keys.clone());
    }
    if let Some(approved) = options.pre_approved_commands {
        compose_opts = compose_opts.with_pre_approved_commands(approved);
    }
    if let Some(fallback) = options.file_ref_fallback_dir.clone() {
        compose_opts = compose_opts.with_file_ref_fallback_dir(fallback);
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
        options.file_ref_fallback_dir.as_deref(),
    )?;
    // Deferred lifecycle strings resolve at event-time (C2); the prepare-time
    // leak / undefined-variable scans do not run over them. See `prepare_direct`.
    validate_no_err_in_no_error_events(&lifecycle, &source.resolved_path)?;

    let mut prompt = composed.content().to_string();
    if prompt.trim().is_empty() {
        return Err(CompositionError::ComposedBodyEmpty {
            source_path: source.resolved_path.clone(),
            mode: CompositionMode::InlineFrontmatterPrompt,
            provided_overrides: override_keys,
        });
    }

    let source_repo_root = options
        .source_repo_root
        .or_else(|| find_git_root_from_path(&source.resolved_path));

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
        rematerialize,
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
