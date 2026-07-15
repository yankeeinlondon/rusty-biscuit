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
    shell_cwd: Option<&std::path::Path>,
) -> Result<Markdown, crate::error::ClaudineError> {
    let md: Markdown = raw_text.into();

    // Demand-driven runtime-context capture (perf, 2026-05-10).
    //
    // The default `ComposeOptions::new()` calls `ComposeContext::capture()`
    // which runs the full `ContextGroup::all()` pipeline: git, repo, file
    // changes, languages, document discovery, OS detection, hardware
    // summary, and a GPU subprocess on macOS. On a representative
    // worktree this measures ~150 ms per call. The system-prompt path
    // does this twice (system prompt + non-interactive appendix), turning
    // a sub-second prep phase into a 300+ ms tax on every compose run.
    //
    // System prompts and non-interactive appendices rarely reference
    // `ctx.*` at all. `ComposeContext::capture_for_content` scans the
    // raw text for `ctx.*` tokens and only captures the groups whose
    // variables are referenced — DateTime is free, the rest is skipped
    // entirely when the file has no `ctx.*` references. When the caller
    // already has a context covering the union of all relevant content
    // (e.g. system-prompt + non-interactive appendix), it can pass it
    // through `shared_ctx` so the per-call capture is skipped.
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
    if let Some(state) = shared_ctx.and_then(|shared| shared.external_state.as_ref()) {
        options = options.with_external_state(state.clone());
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
    external_state: Option<serde_json::Value>,
}

fn mask_ctx_key(content: &str, key: &str) -> String {
    let needle = format!("ctx.{key}");
    let replacement = format!("ctx.__claudine_known_{key}");
    let mut masked = String::with_capacity(content.len());
    let mut cursor = 0;

    for (start, _) in content.match_indices(&needle) {
        let end = start + needle.len();
        if content[end..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        masked.push_str(&content[cursor..start]);
        masked.push_str(&replacement);
        cursor = end;
    }
    masked.push_str(&content[cursor..]);
    masked
}

fn references_ctx_key(content: &str, key: &str) -> bool {
    let needle = format!("ctx.{key}");
    content.match_indices(&needle).any(|(start, _)| {
        let end = start + needle.len();
        !content[end..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

fn detect_os_name() -> Option<String> {
    use sniff::os::OsType;

    match sniff::os::detect_os_type() {
        OsType::Windows => Some("Windows".to_string()),
        OsType::MacOS => Some("macOS".to_string()),
        OsType::Linux => Some("Linux".to_string()),
        _ => None,
    }
}

/// Capture shared composition state covering every text body that the
/// caller will compose this session.
///
/// Scans the union of all bodies for `ctx.*` references via
/// [`ComposeContext::capture_for_content`] so each runtime-context
/// group (Repo, OS, Hardware, etc.) is captured at most once even when
/// the system prompt and the non-interactive appendix together
/// reference fields from several groups. At a known monorepo root, the
/// already-resolved `area` is supplied as external state so Repo capture
/// is omitted when no other Repo field is referenced. An OS-name-only
/// reference similarly uses Sniff's OS-type probe instead of the full OS
/// group, whose package-manager inventory is unnecessary for `ctx.os`.
fn build_shared_compose_context(
    primary: Option<(&SystemPromptSource, &str)>,
    appendix_candidates: Option<&[(&SystemPromptSource, &str)]>,
    launch_context: &crate::system_prompt::context::LaunchContext,
) -> SharedComposeContext {
    let mut combined = String::new();
    if let Some((_, text)) = primary {
        combined.push_str(text);
        combined.push('\n');
    }
    if let Some(candidates) = appendix_candidates {
        for (_, text) in candidates {
            combined.push_str(text);
            combined.push('\n');
        }
    }

    // Anchor the capture to a directory that exists. Prefer the launch
    // context's CWD because resolution is most often rooted there;
    // falling back to the source file's parent or the process CWD
    // mirrors the pre-shared-context behaviour for callers that don't
    // pass a primary source.
    let base_dir = if launch_context.cwd.exists() {
        launch_context.cwd.clone()
    } else if let Some(parent) = primary.and_then(|(s, _)| source_path(s).and_then(|p| p.parent()))
    {
        parent.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    };

    // Mask only exact keys whose values are supplied below; similarly named
    // keys retain Darkmatter's normal capture.
    let known_area = launch_context
        .repo_root
        .as_ref()
        .filter(|root| {
            launch_context.cwd == **root
                && launch_context.package_area_root.as_ref() == Some(*root)
        })
        .map(|_| "root");
    let known_os = (references_ctx_key(&combined, "os")
        && !["os_distro", "os_package_manager", "os_version"]
            .iter()
            .any(|key| references_ctx_key(&combined, key)))
    .then(detect_os_name)
    .flatten();

    let mut capture_content = combined;
    if known_area.is_some() {
        capture_content = mask_ctx_key(&capture_content, "area");
    }
    if known_os.is_some() {
        capture_content = mask_ctx_key(&capture_content, "os");
    }

    let mut runtime = ComposeContext::capture_for_content(&base_dir, &capture_content);
    if let Some(ref agent) = launch_context.agent {
        runtime
            .env_mut()
            .insert("AGENT".to_string(), agent.clone());
    }
    let mut known_context = serde_json::Map::new();
    if let Some(area) = known_area {
        known_context.insert("area".to_string(), serde_json::Value::String(area.to_string()));
    }
    if let Some(os) = known_os {
        known_context.insert("os".to_string(), serde_json::Value::String(os));
    }
    let external_state = (!known_context.is_empty()).then(|| {
        let mut state = serde_json::Map::new();
        state.insert("ctx".to_string(), serde_json::Value::Object(known_context));
        serde_json::Value::Object(state)
    });

    SharedComposeContext {
        runtime,
        external_state,
    }
}

/// Internal variant of [`prepare_system_prompt`] that accepts a
/// pre-captured shared context. The public entrypoint passes `None`
/// to preserve its original capture semantics.
fn prepare_system_prompt_with_ctx(
    source: SystemPromptSource,
    raw_text: &str,
    shared_ctx: Option<&SharedComposeContext>,
    shell_cwd: Option<&std::path::Path>,
) -> Result<ResolvedSystemPrompt, crate::error::ClaudineError> {
    let composed_md = compose_prompt_markdown(&source, raw_text, shared_ctx, shell_cwd)?;
    let composed_markdown = composed_md.content().to_string();
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

/// Variant of [`prepare_non_interactive_appendix`] that consumes an
/// already-resolved candidate list and an optional shared context.
fn prepare_non_interactive_appendix_from(
    candidates: Vec<(SystemPromptSource, String)>,
    shared_ctx: Option<&SharedComposeContext>,
    shell_cwd: Option<&std::path::Path>,
) -> Result<PreparedNonInteractiveAppendix, crate::error::ClaudineError> {
    for (source, raw_text) in candidates {
        let composed_md = compose_prompt_markdown(&source, &raw_text, shared_ctx, shell_cwd)?;
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
    let composed_md = compose_prompt_markdown(&source, raw_text, None, None)?;
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
    let _span = info_span!(
        "system_prompt_prepare",
        non_interactive,
        repo = %context.repo_root.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
    )
    .entered();

    // Resolve sources up front so we can capture the runtime context
    // once over the union of every text body that will go through
    // Darkmatter compose. Capturing per-call paid the demand-driven
    // cost twice (system prompt + appendix); a single pre-capture pays
    // it once.
    let primary = crate::system_prompt::resolve::resolve_system_prompt_source(args, context)?;
    let appendix_candidates = if non_interactive {
        Some(crate::system_prompt::resolve::resolve_non_interactive_candidates(context)?)
    } else {
        None
    };

    let shared_ctx = build_shared_compose_context(
        primary.as_ref().map(|(s, t)| (s, t.as_str())),
        appendix_candidates
            .as_ref()
            .map(|cs| cs.iter().map(|(s, t)| (s, t.as_str())).collect::<Vec<_>>())
            .as_deref(),
        context,
    );

    // Pin `::shell` execution to the agent's launch repo root so directives
    // in a system-prompt template run where the agent does, not next to the
    // template file on disk.
    let shell_cwd = context.repo_root.as_deref();

    let effective = match primary {
        Some((source, raw_text)) => {
            prepare_system_prompt_with_ctx(source, &raw_text, Some(&shared_ctx), shell_cwd)?
        }
        None => ResolvedSystemPrompt::None,
    };

    if !non_interactive {
        return Ok(effective);
    }

    let appendix = prepare_non_interactive_appendix_from(
        appendix_candidates.unwrap_or_default(),
        Some(&shared_ctx),
        shell_cwd,
    )?;

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
