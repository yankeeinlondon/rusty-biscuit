use biscuit_file::serde_yaml_ng;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::schemas::{SimplifiedSchema, parse_yaml_schema};
use tracing::{info_span, warn};

use crate::system_prompt::types::*;

fn source_path(source: &SystemPromptSource) -> Option<&std::path::Path> {
    match source {
        SystemPromptSource::StandardDiscovered { path, .. }
        | SystemPromptSource::ExplicitFile { path, .. }
        | SystemPromptSource::NonInteractiveFile { path, .. } => Some(path.as_path()),
        SystemPromptSource::BuiltInNonInteractive => None,
    }
}

fn invocation_context_error(
    error: crate::invocation_context::InvocationContextError,
) -> crate::error::ClaudineError {
    crate::error::ClaudineError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        error,
    ))
}

struct ResolvedPromptInput {
    source: SystemPromptSource,
    raw_text: String,
    source_context: Option<crate::invocation_context::SourceContext>,
}

impl ResolvedPromptInput {
    fn capture(
        resolved: (SystemPromptSource, String),
        invocation: &crate::invocation_context::InvocationContext,
    ) -> Result<Self, crate::error::ClaudineError> {
        let (source, raw_text) = resolved;
        let source_context = source_path(&source)
            .map(|path| invocation.derive_source(path).map_err(invocation_context_error))
            .transpose()?;
        Ok(Self {
            source,
            raw_text,
            source_context,
        })
    }
}

/// Baseline [`SimplifiedSchema`] claudine attaches to discovered
/// `system-prompt.md` files so the `mode` frontmatter key is validated
/// during Darkmatter compose.
///
/// The schema declares a single optional property:
///
/// - `mode: enum(append, replace; default(append))`
///
/// The `default(append)` is annotation-only — Darkmatter does not
/// backfill an absent key into the composed frontmatter — so
/// [`mode_from_composed`] still treats an absent `mode` as `Append`.
///
/// Constructed once per discovered-source compose call from a
/// compile-controlled literal; a parse failure here is a programmer
/// error, not a runtime condition, hence the panic over the grammar
/// constant.
fn discovered_baseline_schema() -> SimplifiedSchema {
    const MODE_GRAMMAR: &str = "enum(append, replace; default(append))";
    let mut mapping = serde_yaml_ng::Mapping::new();
    mapping.insert(
        serde_yaml_ng::Value::String("mode".to_string()),
        serde_yaml_ng::Value::String(MODE_GRAMMAR.to_string()),
    );
    parse_yaml_schema(&serde_yaml_ng::Value::Mapping(mapping)).unwrap_or_else(|err| {
        panic!(
            "discovered system-prompt baseline schema must parse (grammar: `{MODE_GRAMMAR}`): {err}"
        )
    })
}

/// Read the effective delivery mode for a discovered system-prompt from
/// its **composed** frontmatter.
///
/// The baseline schema attached during compose (`mode` enum of
/// `append`/`replace`) validates the value, so the common path is an
/// enum-validated string. The one way an unexpected value can still
/// arrive is if the document declares its own `$schema` that redefines
/// `mode` (merge rule: document-side-wins). The read-back stays
/// defensive in that override case: any non-`replace` result (including
/// `Ok(None)` for an absent key and `Err(_)` for a non-string that
/// escaped validation) resolves to `Append`, the backwards-compatible
/// default — never a panic, never a bespoke mode.
fn mode_from_composed(composed: &Markdown) -> SystemPromptMode {
    match composed.fm_get::<String>("mode") {
        Ok(Some(value)) if value == "replace" => SystemPromptMode::Replace,
        Ok(_) => SystemPromptMode::Append,
        Err(err) => {
            warn!(
                error = %err,
                "system-prompt `mode` read-back failed; falling back to Append"
            );
            SystemPromptMode::Append
        }
    }
}

/// Resolve the effective delivery mode after compose.
///
/// Discovered files read their mode from the composed frontmatter
/// ([`mode_from_composed`]). Explicit-flag files carry their mode on
/// the source itself, and the non-interactive sources are always
/// `Append` (their content is appended regardless of any frontmatter).
fn effective_mode(source: &SystemPromptSource, composed: &Markdown) -> SystemPromptMode {
    match source {
        SystemPromptSource::StandardDiscovered { .. } => mode_from_composed(composed),
        SystemPromptSource::ExplicitFile { mode, .. } => *mode,
        SystemPromptSource::NonInteractiveFile { .. }
        | SystemPromptSource::BuiltInNonInteractive => SystemPromptMode::Append,
    }
}

fn compose_prompt_markdown(
    source: &SystemPromptSource,
    raw_text: &str,
    shared_ctx: Option<&SharedComposeContext>,
    source_context: Option<&crate::invocation_context::SourceContext>,
    shell_cwd: Option<&std::path::Path>,
) -> Result<Markdown, crate::error::ClaudineError> {
    let md: Markdown = raw_text.into();

    // Canonical session preparation supplies one context covering the union of
    // the primary and appendix requirements. The ambient branch remains for
    // library compatibility and captures only groups referenced by this file.
    let ctx = match shared_ctx {
        Some(c) => c.runtime.clone(),
        None => {
            let base_dir = match source_path(source).and_then(|p| p.parent()) {
                Some(parent) => parent.to_path_buf(),
                None => std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            };
            ComposeContext::capture_for_content(&base_dir, raw_text)
        }
    };
    let mut options = match source_path(source) {
        Some(path) => crate::composition::bind_agent_workspace(
            ComposeOptions::new_with_context(ctx),
            path,
            shell_cwd,
        ),
        None => ComposeOptions::new_with_context(ctx),
    };
    if let Some(invocation) = shared_ctx.and_then(|shared| shared.invocation.as_ref()) {
        invocation.record_compose_operation();
    }
    if let Some(source_context) = source_context {
        options = options.with_file_resolution_context(
            source_context.file_resolution_context().clone(),
        );
    }
    // Attach the baseline schema for discovered `system-prompt.md` files
    // only — explicit flags carry their own mode and the non-interactive
    // appendix is always appended, so neither runs `mode` validation.
    // The returned `Markdown` carries the composed frontmatter, from
    // which the discovered mode is read back by `mode_from_composed`.
    let options = if matches!(source, SystemPromptSource::StandardDiscovered { .. }) {
        options.with_baseline_schema(discovered_baseline_schema())
    } else {
        options
    };

    let (composed, _report) = md.compose_with(options)?;

    Ok(composed)
}

fn merge_prompt_sections(base: &str, appendix: &str) -> String {
    let base = base.trim_end();
    let appendix = appendix.trim();

    if base.is_empty() {
        appendix.to_string()
    } else if appendix.is_empty() {
        base.to_string()
    } else {
        format!("{base}\n\n{appendix}")
    }
}

struct SharedComposeContext {
    runtime: ComposeContext,
    invocation: Option<crate::invocation_context::InvocationContext>,
}

fn build_shared_compose_context_with_invocation(
    primary: Option<&ResolvedPromptInput>,
    appendix_candidates: Option<&[ResolvedPromptInput]>,
    launch_context: &crate::system_prompt::context::LaunchContext,
    invocation: &crate::invocation_context::InvocationContext,
) -> Result<SharedComposeContext, crate::error::ClaudineError> {
    let mut combined = String::new();
    if let Some(input) = primary {
        combined.push_str(&input.raw_text);
        combined.push('\n');
    }
    if let Some(candidates) = appendix_candidates {
        for input in candidates {
            combined.push_str(&input.raw_text);
            combined.push('\n');
        }
    }

    let source_context = primary
        .and_then(|input| input.source_context.clone())
        .or_else(|| {
            appendix_candidates.and_then(|candidates| {
                candidates
                    .iter()
                    .find_map(|input| input.source_context.clone())
            })
        })
        .map(Ok)
        .unwrap_or_else(|| {
            let anchor = invocation
                .launch_cwd()
                .join(".claudine-built-in-system-prompt.md");
            invocation
                .derive_source(&anchor)
                .map_err(invocation_context_error)
        })?;
    let requirements = darkmatter::markdown::compose::ContextRequirements::for_content(&combined);
    let evidence = invocation.runtime_evidence(&source_context, &requirements);
    let mut runtime = ComposeContext::capture_with_evidence(
        source_context.base_dir(),
        &requirements,
        &evidence,
    );
    if let Some(agent) = launch_context.agent.as_ref() {
        runtime.env_mut().insert("AGENT".to_string(), agent.clone());
    }

    Ok(SharedComposeContext {
        runtime,
        invocation: Some(invocation.clone()),
    })
}

/// Prepare a resolved input with its request-owned source and compose context.
fn prepare_system_prompt_with_ctx(
    input: ResolvedPromptInput,
    shared_ctx: Option<&SharedComposeContext>,
    shell_cwd: Option<&std::path::Path>,
) -> Result<ResolvedSystemPrompt, crate::error::ClaudineError> {
    let ResolvedPromptInput {
        source,
        raw_text,
        source_context,
    } = input;
    let composed_md = compose_prompt_markdown(
        &source,
        &raw_text,
        shared_ctx,
        source_context.as_ref(),
        shell_cwd,
    )?;
    let composed_markdown = composed_md.content().to_string();
    if composed_markdown.trim().is_empty() {
        return Ok(ResolvedSystemPrompt::Disabled { source });
    }
    let mode = effective_mode(&source, &composed_md);
    Ok(ResolvedSystemPrompt::Ready(PreparedSystemPrompt {
        mode,
        source,
        raw_text,
        composed_markdown,
        non_interactive_appendix: None,
    }))
}

/// Variant of [`prepare_non_interactive_appendix`] that consumes an
/// already-resolved candidate list and an optional shared context.
fn prepare_non_interactive_appendix_from(
    candidates: Vec<ResolvedPromptInput>,
    shared_ctx: Option<&SharedComposeContext>,
    shell_cwd: Option<&std::path::Path>,
) -> Result<PreparedNonInteractiveAppendix, crate::error::ClaudineError> {
    for input in candidates {
        let ResolvedPromptInput {
            source,
            raw_text,
            source_context,
        } = input;
        let composed_md = compose_prompt_markdown(
            &source,
            &raw_text,
            shared_ctx,
            source_context.as_ref(),
            shell_cwd,
        )?;
        let normalized = composed_md.content().trim().to_string();
        if normalized.is_empty() {
            continue;
        }
        return Ok(PreparedNonInteractiveAppendix {
            source,
            raw_text,
            composed_markdown: normalized,
        });
    }
    unreachable!("non-interactive prompt candidates must include a built-in fallback")
}

/// Compose a resolved system prompt source through Darkmatter and
/// return the effective result.
///
/// If the composed body is empty after trimming, returns
/// `ResolvedSystemPrompt::Disabled`. Otherwise returns `Ready`.
pub fn prepare_system_prompt(
    source: SystemPromptSource,
    raw_text: &str,
) -> Result<ResolvedSystemPrompt, crate::error::ClaudineError> {
    // No launch context here, so shell directives fall back to Darkmatter's
    // source-relative default. The session-aware path supplies a working
    // directory via `resolve_and_prepare_for_session`.
    let composed_md = compose_prompt_markdown(&source, raw_text, None, None, None)?;
    let composed_markdown = composed_md.content().to_string();

    // Empty-body check
    if composed_markdown.trim().is_empty() {
        return Ok(ResolvedSystemPrompt::Disabled { source });
    }

    let mode = effective_mode(&source, &composed_md);
    Ok(ResolvedSystemPrompt::Ready(PreparedSystemPrompt {
        mode,
        source,
        raw_text: raw_text.to_string(),
        composed_markdown,
        non_interactive_appendix: None,
    }))
}

/// Top-level convenience: resolve + compose in one call.
pub fn resolve_and_prepare(
    args: &SystemPromptArgs,
    context: &crate::system_prompt::context::LaunchContext,
) -> Result<ResolvedSystemPrompt, crate::error::ClaudineError> {
    resolve_and_prepare_for_session(args, context, false)
}

/// Session-aware convenience: resolve + compose and optionally append
/// non-interactive safety instructions.
pub fn resolve_and_prepare_for_session(
    args: &SystemPromptArgs,
    context: &crate::system_prompt::context::LaunchContext,
    non_interactive: bool,
) -> Result<ResolvedSystemPrompt, crate::error::ClaudineError> {
    let invocation = crate::invocation_context::InvocationContext::capture_at(&context.cwd);
    resolve_and_prepare_for_session_with_context(args, context, non_interactive, &invocation)
}

/// Resolve and compose session prompts using request-owned discovery evidence.
pub fn resolve_and_prepare_for_session_with_context(
    args: &SystemPromptArgs,
    context: &crate::system_prompt::context::LaunchContext,
    non_interactive: bool,
    invocation: &crate::invocation_context::InvocationContext,
) -> Result<ResolvedSystemPrompt, crate::error::ClaudineError> {
    let _span = info_span!(
        "system_prompt_prepare",
        non_interactive,
        repo = %context.repo_root.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
    )
    .entered();
    invocation.record_system_prompt_lookup();

    // Resolve sources up front so we can capture the runtime context
    // once over the union of every text body that will go through
    // Darkmatter compose. Capturing per-call paid the demand-driven
    // cost twice (system prompt + appendix); a single pre-capture pays
    // it once.
    let lookup_started = std::time::Instant::now();
    let primary_result = crate::system_prompt::resolve::resolve_system_prompt_source_with_invocation(
        args,
        context,
        invocation,
    );
    let appendix_candidates_result = if non_interactive {
        Some(
            crate::system_prompt::resolve::resolve_non_interactive_candidates_with_invocation(
                context,
                invocation,
            ),
        )
    } else {
        None
    };
    invocation.record_system_prompt_timing("lookup", lookup_started.elapsed());
    let primary = primary_result?
        .map(|resolved| ResolvedPromptInput::capture(resolved, invocation))
        .transpose()?;
    let appendix_candidates = appendix_candidates_result
        .transpose()?
        .map(|candidates| {
            candidates
                .into_iter()
                .map(|resolved| ResolvedPromptInput::capture(resolved, invocation))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;

    let runtime_capture_started = std::time::Instant::now();
    let shared_ctx_result = build_shared_compose_context_with_invocation(
        primary.as_ref(),
        appendix_candidates.as_deref(),
        context,
        invocation,
    );
    invocation.record_system_prompt_timing(
        "runtime capture",
        runtime_capture_started.elapsed(),
    );
    let shared_ctx = shared_ctx_result?;

    // Pin `::shell` execution to the agent's launch workspace so directives
    // run where the agent does, not next to the template file on disk.
    let shell_cwd = Some(
        context
            .repo_root
            .as_deref()
            .unwrap_or_else(|| invocation.launch_cwd()),
    );

    let primary_compose_started = std::time::Instant::now();
    let effective_result = match primary {
        Some(input) => prepare_system_prompt_with_ctx(input, Some(&shared_ctx), shell_cwd),
        None => Ok(ResolvedSystemPrompt::None),
    };
    invocation.record_system_prompt_timing(
        "primary compose",
        primary_compose_started.elapsed(),
    );
    let effective = effective_result?;

    if !non_interactive {
        return Ok(effective);
    }

    let appendix_compose_started = std::time::Instant::now();
    let appendix_result = prepare_non_interactive_appendix_from(
        appendix_candidates.unwrap_or_default(),
        Some(&shared_ctx),
        shell_cwd,
    );
    invocation.record_system_prompt_timing(
        "appendix compose",
        appendix_compose_started.elapsed(),
    );
    let appendix = appendix_result?;

    Ok(match effective {
        ResolvedSystemPrompt::Ready(mut prepared) => {
            prepared.raw_text = merge_prompt_sections(&prepared.raw_text, &appendix.raw_text);
            prepared.composed_markdown =
                merge_prompt_sections(&prepared.composed_markdown, &appendix.composed_markdown);
            prepared.non_interactive_appendix = Some(appendix);
            ResolvedSystemPrompt::Ready(prepared)
        }
        ResolvedSystemPrompt::Disabled { source } => {
            let mode = match &source {
                SystemPromptSource::ExplicitFile { mode, .. } => *mode,
                // An empty discovered/non-interactive body makes the
                // declared mode moot — the effective prompt becomes the
                // appendix, which is Append-style content (matches the
                // spec's empty-body edge case where `mode` is
                // irrelevant). `NonInteractiveFile` / `BuiltInNonInteractive`
                // cannot reach this arm via `resolve_system_prompt_source`
                // but are listed for exhaustiveness.
                SystemPromptSource::StandardDiscovered { .. }
                | SystemPromptSource::NonInteractiveFile { .. }
                | SystemPromptSource::BuiltInNonInteractive => SystemPromptMode::Append,
            };
            ResolvedSystemPrompt::Ready(PreparedSystemPrompt {
                mode,
                source: appendix.source.clone(),
                raw_text: appendix.raw_text.clone(),
                composed_markdown: appendix.composed_markdown.clone(),
                non_interactive_appendix: None,
            })
        }
        ResolvedSystemPrompt::None => ResolvedSystemPrompt::Ready(PreparedSystemPrompt {
            mode: SystemPromptMode::Append,
            source: appendix.source.clone(),
            raw_text: appendix.raw_text.clone(),
            composed_markdown: appendix.composed_markdown.clone(),
            non_interactive_appendix: None,
        }),
    })
}

#[cfg(test)]
mod tests;
