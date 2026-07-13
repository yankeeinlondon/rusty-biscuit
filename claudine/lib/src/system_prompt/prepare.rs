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
mod tests {
    use super::*;
    use serial_test::serial;
    use std::path::PathBuf;
    use tempfile::TempDir;

    use crate::system_prompt::context::LaunchContext;

    /// Helper: create a temp file and return its path.
    fn write_temp_file(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn shared_context_reuses_known_root_area_without_repo_capture() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let source = SystemPromptSource::ExplicitFile {
            path: write_temp_file(tmp.path(), "prompt.md", "{{ ctx.area }} / {{ ctx.os }}"),
            mode: SystemPromptMode::Append,
        };
        let context = LaunchContext {
            cwd: root.clone(),
            repo_root: Some(root.clone()),
            package_area_root: Some(root),
            package_root: None,
            agent: None,
        };

        let shared = build_shared_compose_context(
            Some((&source, "{{ ctx.area }} / {{ ctx.os }}")),
            None,
            &context,
        );
        assert!(shared.runtime.get("repo").is_none());
        assert!(shared.runtime.get("os").is_none());

        let result = prepare_system_prompt_with_ctx(
            source,
            "{{ ctx.area }} / {{ ctx.os }}",
            Some(&shared),
            None,
        )
        .unwrap();
        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert!(prepared.composed_markdown.starts_with("root / "));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn plain_markdown_composes_as_is() {
        let tmp = TempDir::new().unwrap();
        let path = write_temp_file(tmp.path(), "prompt.md", "# Hello World\n\nSome content.");

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, "# Hello World\n\nSome content.").unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert!(prepared.composed_markdown.contains("Hello World"));
                assert!(prepared.composed_markdown.contains("Some content."));
                assert_eq!(prepared.mode, SystemPromptMode::Append);
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn transclusion_resolves_relative() {
        let tmp = TempDir::new().unwrap();
        let included_path = write_temp_file(tmp.path(), "included.md", "Included content here.");
        let _ = &included_path; // ensure file exists

        let prompt_path = write_temp_file(
            tmp.path(),
            "prompt.md",
            "Before.\n\n::file ./included.md\n\nAfter.",
        );

        let source = SystemPromptSource::StandardDiscovered {
            path: prompt_path,
            scope: StandardPromptScope::Package,
        };

        let result =
            prepare_system_prompt(source, "Before.\n\n::file ./included.md\n\nAfter.").unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert!(
                    prepared
                        .composed_markdown
                        .contains("Included content here."),
                    "Expected transclusion to resolve. Got: {}",
                    prepared.composed_markdown
                );
                assert!(prepared.composed_markdown.contains("Before."));
                assert!(prepared.composed_markdown.contains("After."));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn shell_directive_executes() {
        let tmp = TempDir::new().unwrap();

        // Shell policy resolution walks up to find .git root, so create
        // a fake git root and place the whitelist there.
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        write_temp_file(tmp.path(), ".darkmatter-shell-whitelist", "prefix echo\n");

        let path = write_temp_file(
            tmp.path(),
            "prompt.md",
            "Pre.\n\n::shell echo hello\n\nPost.",
        );

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, "Pre.\n\n::shell echo hello\n\nPost.").unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert!(
                    prepared.composed_markdown.contains("hello"),
                    "Expected shell output. Got: {}",
                    prepared.composed_markdown
                );
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn interpolation_expands() {
        let tmp = TempDir::new().unwrap();
        let path = write_temp_file(tmp.path(), "prompt.md", "Today is {{today}}.");

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, "Today is {{today}}.").unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                // {{today}} should be replaced with an actual date, not the literal
                assert!(
                    !prepared.composed_markdown.contains("{{today}}"),
                    "Expected interpolation to expand. Got: {}",
                    prepared.composed_markdown
                );
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn empty_body_produces_disabled() {
        let tmp = TempDir::new().unwrap();
        let path = write_temp_file(tmp.path(), "prompt.md", "---\ntitle: Empty\n---\n");

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, "---\ntitle: Empty\n---\n").unwrap();

        assert!(
            result.is_disabled(),
            "Expected Disabled for empty body, got {result:?}"
        );
    }

    #[test]
    fn whitespace_only_body_produces_disabled() {
        let tmp = TempDir::new().unwrap();
        let path = write_temp_file(tmp.path(), "prompt.md", "---\ntitle: Blank\n---\n\n   \n\n");

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, "---\ntitle: Blank\n---\n\n   \n\n").unwrap();

        assert!(
            result.is_disabled(),
            "Expected Disabled for whitespace-only body, got {result:?}"
        );
    }

    #[test]
    fn frontmatter_not_forwarded() {
        let tmp = TempDir::new().unwrap();
        let raw = "---\ntitle: Secret\n---\n\nVisible body.";
        let path = write_temp_file(tmp.path(), "prompt.md", raw);

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, raw).unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert!(
                    !prepared.composed_markdown.contains("---"),
                    "Frontmatter delimiters should not appear in composed output. Got: {}",
                    prepared.composed_markdown
                );
                assert!(
                    !prepared.composed_markdown.contains("title: Secret"),
                    "Frontmatter content should not appear. Got: {}",
                    prepared.composed_markdown
                );
                assert!(prepared.composed_markdown.contains("Visible body."));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn standard_file_always_append() {
        let tmp = TempDir::new().unwrap();
        let path = write_temp_file(tmp.path(), "prompt.md", "Content.");

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Package,
        };

        let result = prepare_system_prompt(source, "Content.").unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Append);
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn explicit_replace_preserves_mode() {
        let tmp = TempDir::new().unwrap();
        let path = write_temp_file(tmp.path(), "prompt.md", "Replacement content.");

        let source = SystemPromptSource::ExplicitFile {
            path,
            mode: SystemPromptMode::Replace,
        };

        let result = prepare_system_prompt(source, "Replacement content.").unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Replace);
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    // --- System Prompt Mode (frontmatter-driven replace) -------------------
    //
    // The following tests pin the spec at `claudine/features/2026-06-14-system-prompt-mode/spec.md`.
    // Discovered `system-prompt.md` files may declare `mode: append|replace`
    // in frontmatter; the value is validated by the baseline `SimplifiedSchema`
    // attached in `compose_prompt_markdown` and read back from the composed
    // frontmatter by `mode_from_composed`.

    #[test]
    fn discovered_absent_mode_defaults_to_append() {
        // Spec test 2.1: absent `mode` frontmatter resolves to Append.
        // Also pins that the JSON Schema `default(append)` annotation is NOT
        // backfilled into composed frontmatter — `fm_get` returns `Ok(None)`
        // and the read-back maps that to `Append`.
        let tmp = TempDir::new().unwrap();
        let raw = "Discovered body with no mode key.";
        let path = write_temp_file(tmp.path(), "prompt.md", raw);

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, raw).unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Append);
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn discovered_null_mode_defaults_to_append() {
        let tmp = TempDir::new().unwrap();
        let raw = "---\nmode: null\n---\n\nBody.";
        let path = write_temp_file(tmp.path(), "prompt.md", raw);

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, raw).unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Append);
                assert!(prepared.composed_markdown.contains("Body."));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn discovered_explicit_append_mode_resolves_append() {
        // Spec test 2.2: explicit `mode: append` resolves to Append.
        let tmp = TempDir::new().unwrap();
        let raw = "---\nmode: append\n---\n\nDiscovered body.";
        let path = write_temp_file(tmp.path(), "prompt.md", raw);

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, raw).unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Append);
                assert!(prepared.composed_markdown.contains("Discovered body."));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn discovered_explicit_replace_mode_resolves_replace() {
        // Spec test 2.3: explicit `mode: replace` resolves to Replace.
        // This is the headline new capability.
        let tmp = TempDir::new().unwrap();
        let raw = "---\nmode: replace\n---\n\nReplacement body.";
        let path = write_temp_file(tmp.path(), "prompt.md", raw);

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, raw).unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Replace);
                assert!(prepared.composed_markdown.contains("Replacement body."));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn discovered_invalid_string_mode_rejected_at_compose() {
        // Spec test 2.4: `mode: overwrite` fails schema validation during
        // compose and surfaces via `ClaudineError::SystemPromptComposition`
        // wrapping `MarkdownError::SchemaValidationFailed` — NOT a bespoke
        // `InvalidSystemPromptMode` variant.
        let tmp = TempDir::new().unwrap();
        let raw = "---\nmode: overwrite\n---\n\nBody.";
        let path = write_temp_file(tmp.path(), "prompt.md", raw);

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        match prepare_system_prompt(source, raw) {
            Err(crate::error::ClaudineError::SystemPromptComposition(
                darkmatter::markdown::MarkdownError::SchemaValidationFailed { problems, .. },
            )) => {
                // The generic summary does not name the property; the
                // structured `problems` vector does. The enum is encoded
                // as an `anyOf` of const schemas, so the human message is
                // also generic — the JSON pointer at `/mode` is the stable
                // signal that the `mode` property failed validation.
                assert!(
                    problems.iter().any(|p| p.path == "/mode"),
                    "expected a problem at `/mode`, got: {problems:?}"
                );
            }
            other => panic!(
                "expected SystemPromptComposition(SchemaValidationFailed), got {other:?}"
            ),
        }
    }

    #[test]
    fn discovered_non_string_mode_rejected_at_compose() {
        // Spec test 2.5: a non-string `mode: 42` fails schema validation
        // during compose and surfaces via the same `SystemPromptComposition`
        // path as an invalid string.
        let tmp = TempDir::new().unwrap();
        let raw = "---\nmode: 42\n---\n\nBody.";
        let path = write_temp_file(tmp.path(), "prompt.md", raw);

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        match prepare_system_prompt(source, raw) {
            Err(crate::error::ClaudineError::SystemPromptComposition(
                darkmatter::markdown::MarkdownError::SchemaValidationFailed { .. },
            )) => {}
            other => panic!(
                "expected SystemPromptComposition(SchemaValidationFailed), got {other:?}"
            ),
        }
    }

    #[test]
    #[serial]
    fn discovered_replace_mode_flows_through_full_pipeline() {
        // Spec test 2.6: `resolve_and_prepare_for_session` with a discovered
        // `mode: replace` file produces a `PreparedSystemPrompt` with
        // `mode: Replace` that flows through to provider delivery.
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        let home = tmp.path().join("home");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(
            repo.join("system-prompt.md"),
            "---\nmode: replace\n---\n\nReplacement body.",
        )
        .unwrap();

        unsafe {
            std::env::set_var("HOME", &home);
        }

        let args = SystemPromptArgs::default();
        let context = LaunchContext {
            agent: None,
            cwd: repo.clone(),
            repo_root: Some(repo),
            package_area_root: None,
            package_root: None,
        };

        let result = resolve_and_prepare_for_session(&args, &context, false).unwrap();

        unsafe {
            std::env::remove_var("HOME");
        }

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Replace);
                assert!(prepared.composed_markdown.contains("Replacement body."));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn explicit_replace_flag_ignores_frontmatter_mode() {
        // Spec test 2.7: `--replace-system-prompt` pointing at a file that
        // contains `mode: append` in frontmatter still uses Replace. The
        // flag wins because the explicit path composes WITHOUT the baseline
        // schema and `resolve_system_prompt_source` returns early before
        // discovery, so the discovered-file frontmatter is structurally
        // never read.
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        let home = tmp.path().join("home");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        let replace_path = write_temp_file(
            repo.as_path(),
            "replace.md",
            "---\nmode: append\n---\n\nReplacement content.",
        );

        unsafe {
            std::env::set_var("HOME", &home);
        }

        let args = SystemPromptArgs {
            replace_file: Some(replace_path.display().to_string()),
            ..Default::default()
        };
        let context = LaunchContext {
            agent: None,
            cwd: repo.clone(),
            repo_root: Some(repo),
            package_area_root: None,
            package_root: None,
        };

        let result = resolve_and_prepare_for_session(&args, &context, false).unwrap();

        unsafe {
            std::env::remove_var("HOME");
        }

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Replace);
                assert!(prepared.composed_markdown.contains("Replacement content."));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn non_interactive_session_preserves_discovered_replace_mode() {
        // Spec test 2.8: a discovered `mode: replace` file in a non-interactive
        // session preserves Replace mode after the safety appendix is appended
        // (the appendix is content, not a mode change).
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        let home = tmp.path().join("home");
        std::fs::create_dir_all(repo.join(".claudine")).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(
            repo.join("system-prompt.md"),
            "---\nmode: replace\n---\n\nReplacement base.",
        )
        .unwrap();
        std::fs::write(
            repo.join(".claudine").join("non-interactive.md"),
            "Repo appendix.",
        )
        .unwrap();

        unsafe {
            std::env::set_var("HOME", &home);
        }

        let args = SystemPromptArgs::default();
        let context = LaunchContext {
            agent: None,
            cwd: repo.clone(),
            repo_root: Some(repo.clone()),
            package_area_root: None,
            package_root: None,
        };

        let result = resolve_and_prepare_for_session(&args, &context, true).unwrap();

        unsafe {
            std::env::remove_var("HOME");
        }

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Replace);
                assert!(prepared.composed_markdown.contains("Replacement base."));
                assert!(prepared.composed_markdown.contains("Repo appendix."));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn discovered_frontmatter_only_replace_mode_resolves_disabled() {
        // Spec test 2.9: a discovered file that is frontmatter-only
        // (`mode: replace`, no body) composes to an empty body and resolves
        // to `ResolvedSystemPrompt::Disabled` regardless of the declared
        // mode — not an error, not an empty replacement.
        let tmp = TempDir::new().unwrap();
        let raw = "---\nmode: replace\n---\n";
        let path = write_temp_file(tmp.path(), "prompt.md", raw);

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, raw).unwrap();

        assert!(
            result.is_disabled(),
            "Expected Disabled for empty body with declared replace mode, got {result:?}"
        );
    }

    #[test]
    fn discovered_schema_conflict_falls_back_to_append() {
        // Spec test 2.10: a document `$schema` that redefines `mode` (e.g.
        // as a free `string`) to allow another value (e.g. `mode: overwrite`)
        // composes successfully (document-side-wins merge relaxes the
        // baseline enum), but claudine resolves the effective delivery mode
        // to Append rather than panicking or inventing a mode.
        let tmp = TempDir::new().unwrap();
        let raw = "---\n$schema:\n  mode: string\nmode: overwrite\n---\n\nBody.";
        let path = write_temp_file(tmp.path(), "prompt.md", raw);

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, raw).unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(
                    prepared.mode,
                    SystemPromptMode::Append,
                    "document-side schema override of `mode` must fall back to Append"
                );
                assert!(prepared.composed_markdown.contains("Body."));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn discovered_replace_mode_propagates_to_prepared_mode_field() {
        // Spec test 2.11: `PreparedSystemPrompt.mode` (the field
        // `describe_effective` and prompt reports read) reflects the
        // composed-frontmatter decision for a discovered `mode: replace`
        // file. `verbosity` frontmatter continues to control only report
        // verbosity and stays orthogonal to delivery mode.
        let tmp = TempDir::new().unwrap();
        let raw = "---\nmode: replace\nverbosity: terse\n---\n\nBody.";
        let path = write_temp_file(tmp.path(), "prompt.md", raw);

        let source = SystemPromptSource::StandardDiscovered {
            path,
            scope: StandardPromptScope::Repo,
        };

        let result = prepare_system_prompt(source, raw).unwrap();

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Replace);
                assert!(prepared.composed_markdown.contains("Body."));
                // Frontmatter (mode, verbosity) is not forwarded into the
                // composed prompt body — like all frontmatter, it is metadata.
                assert!(
                    !prepared.composed_markdown.contains("verbosity"),
                    "frontmatter should not leak into composed body: {}",
                    prepared.composed_markdown
                );
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn non_interactive_session_uses_builtin_when_no_prompt_exists() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let cwd = tmp.path().join("workspace");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&cwd).unwrap();

        unsafe {
            std::env::set_var("HOME", &home);
        }

        let args = SystemPromptArgs::default();
        let context = LaunchContext {
            agent: None,
            cwd: cwd.clone(),
            repo_root: Some(cwd),
            package_area_root: None,
            package_root: None,
        };

        let result = resolve_and_prepare_for_session(&args, &context, true).unwrap();

        unsafe {
            std::env::remove_var("HOME");
        }

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Append);
                // The built-in prompt is delivered verbatim modulo
                // Darkmatter's markdown reflow, which joins soft-wrapped
                // lines (single newline between sentences) into one
                // paragraph. Assert each section is present rather than
                // exact equality so the test is robust to that reflow.
                let body = &prepared.composed_markdown;
                assert!(body.contains("**IMPORTANT:**"));
                assert!(body.contains("## Shell restrictions"));
                assert!(body.contains("follow-up stdin input"));
                assert!(body.contains("Avoid REPLs"));
                assert!(body.contains("Prefer one-shot commands"));
                assert!(body.contains("choose a different approach"));
                assert!(matches!(
                    prepared.source,
                    SystemPromptSource::BuiltInNonInteractive
                ));
                assert!(prepared.non_interactive_appendix.is_none());
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn non_interactive_session_appends_repo_prompt() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        let home = tmp.path().join("home");
        std::fs::create_dir_all(repo.join(".claudine")).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(repo.join("system-prompt.md"), "Base prompt.").unwrap();
        std::fs::write(
            repo.join(".claudine").join("non-interactive.md"),
            "Repo appendix.",
        )
        .unwrap();

        unsafe {
            std::env::set_var("HOME", &home);
        }

        let args = SystemPromptArgs::default();
        let context = LaunchContext {
            agent: None,
            cwd: repo.clone(),
            repo_root: Some(repo.clone()),
            package_area_root: None,
            package_root: None,
        };

        let result = resolve_and_prepare_for_session(&args, &context, true).unwrap();

        unsafe {
            std::env::remove_var("HOME");
        }

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Append);
                assert_eq!(prepared.composed_markdown, "Base prompt.\n\nRepo appendix.");
                let appendix = prepared
                    .non_interactive_appendix
                    .expect("missing non-interactive appendix metadata");
                assert_eq!(appendix.composed_markdown, "Repo appendix.");
                assert!(matches!(
                    appendix.source,
                    SystemPromptSource::NonInteractiveFile {
                        scope: StandardPromptScope::Repo,
                        ..
                    }
                ));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn non_interactive_session_preserves_replace_mode() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        let home = tmp.path().join("home");
        std::fs::create_dir_all(repo.join(".claudine")).unwrap();
        std::fs::create_dir_all(&home).unwrap();

        let replace_path = write_temp_file(repo.as_path(), "replace.md", "Replacement prompt.");
        std::fs::write(
            repo.join(".claudine").join("non-interactive.md"),
            "Repo appendix.",
        )
        .unwrap();

        unsafe {
            std::env::set_var("HOME", &home);
        }

        let args = SystemPromptArgs {
            replace_file: Some(replace_path.display().to_string()),
            ..Default::default()
        };
        let context = LaunchContext {
            agent: None,
            cwd: repo.clone(),
            repo_root: Some(repo.clone()),
            package_area_root: None,
            package_root: None,
        };

        let result = resolve_and_prepare_for_session(&args, &context, true).unwrap();

        unsafe {
            std::env::remove_var("HOME");
        }

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Replace);
                assert_eq!(
                    prepared.composed_markdown,
                    "Replacement prompt.\n\nRepo appendix."
                );
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn non_interactive_session_ignores_empty_base_prompt() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        let home = tmp.path().join("home");
        std::fs::create_dir_all(repo.join(".claudine")).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(repo.join("system-prompt.md"), "---\ntitle: Empty\n---\n").unwrap();
        std::fs::write(
            repo.join(".claudine").join("non-interactive.md"),
            "Repo appendix.",
        )
        .unwrap();

        unsafe {
            std::env::set_var("HOME", &home);
        }

        let args = SystemPromptArgs::default();
        let context = LaunchContext {
            agent: None,
            cwd: repo.clone(),
            repo_root: Some(repo.clone()),
            package_area_root: None,
            package_root: None,
        };

        let result = resolve_and_prepare_for_session(&args, &context, true).unwrap();

        unsafe {
            std::env::remove_var("HOME");
        }

        match result {
            ResolvedSystemPrompt::Ready(prepared) => {
                assert_eq!(prepared.mode, SystemPromptMode::Append);
                assert_eq!(prepared.composed_markdown, "Repo appendix.");
                assert!(matches!(
                    prepared.source,
                    SystemPromptSource::NonInteractiveFile {
                        scope: StandardPromptScope::Repo,
                        ..
                    }
                ));
            }
            other => panic!("Expected Ready, got {other:?}"),
        }
    }

    #[test]
    fn resolve_and_prepare_none() {
        let tmp = TempDir::new().unwrap();
        let cwd = tmp.path().join("empty");
        std::fs::create_dir_all(&cwd).unwrap();

        let args = SystemPromptArgs::default();
        let context = LaunchContext {
            agent: None,
            cwd: cwd.clone(),
            repo_root: Some(cwd.clone()),
            package_area_root: None,
            package_root: None,
        };

        let result = resolve_and_prepare(&args, &context).unwrap();

        // Should be None unless ~/.claudine/system-prompt.md exists on the host
        match result {
            ResolvedSystemPrompt::None => {} // expected in clean test environments
            ResolvedSystemPrompt::Ready(_) => {
                // Acceptable if user has ~/.claudine/system-prompt.md
            }
            ResolvedSystemPrompt::Disabled { .. } => {
                // Acceptable if user has an empty ~/.claudine/system-prompt.md
            }
        }
    }
}
