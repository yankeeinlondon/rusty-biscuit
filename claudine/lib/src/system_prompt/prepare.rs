use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use tracing::info_span;

use crate::system_prompt::types::*;

fn source_path(source: &SystemPromptSource) -> Option<&std::path::Path> {
    match source {
        SystemPromptSource::StandardDiscovered { path, .. }
        | SystemPromptSource::ExplicitFile { path, .. }
        | SystemPromptSource::NonInteractiveFile { path, .. } => Some(path.as_path()),
        SystemPromptSource::BuiltInNonInteractive => None,
    }
}

fn mode_for_source(source: &SystemPromptSource) -> SystemPromptMode {
    match source {
        SystemPromptSource::StandardDiscovered { .. }
        | SystemPromptSource::NonInteractiveFile { .. }
        | SystemPromptSource::BuiltInNonInteractive => SystemPromptMode::Append,
        SystemPromptSource::ExplicitFile { mode, .. } => *mode,
    }
}

fn compose_prompt_markdown(
    source: &SystemPromptSource,
    raw_text: &str,
    shared_ctx: Option<&ComposeContext>,
    shell_cwd: Option<&std::path::Path>,
) -> Result<String, crate::error::ClaudineError> {
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
        Some(c) => c.clone(),
        None => {
            let base_dir = match source_path(source).and_then(|p| p.parent()) {
                Some(parent) => parent.to_path_buf(),
                None => std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            };
            ComposeContext::capture_for_content(&base_dir, raw_text)
        }
    };
    let options = match source_path(source) {
        Some(path) => crate::composition::bind_agent_workspace(
            ComposeOptions::new_with_context(ctx),
            path,
            shell_cwd,
        ),
        None => ComposeOptions::new_with_context(ctx),
    };

    let (composed, _report) = md.compose_with(options)?;

    Ok(composed.content().to_string())
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

/// Capture a single `ComposeContext` covering every text body that the
/// caller will compose this session.
///
/// Scans the union of all bodies for `ctx.*` references via
/// [`ComposeContext::capture_for_content`] so each runtime-context
/// group (Repo, OS, Hardware, etc.) is captured at most once even when
/// the system prompt and the non-interactive appendix together
/// reference fields from several groups.
fn build_shared_compose_context(
    primary: Option<(&SystemPromptSource, &str)>,
    appendix_candidates: Option<&[(&SystemPromptSource, &str)]>,
    launch_context: &crate::system_prompt::context::LaunchContext,
) -> ComposeContext {
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

    let mut ctx = ComposeContext::capture_for_content(&base_dir, &combined);
    if let Some(ref agent) = launch_context.agent {
        ctx.env_mut().insert("AGENT".to_string(), agent.clone());
    }
    ctx
}

/// Internal variant of [`prepare_system_prompt`] that accepts a
/// pre-captured shared context. The public entrypoint passes `None`
/// to preserve its original capture semantics.
fn prepare_system_prompt_with_ctx(
    source: SystemPromptSource,
    raw_text: &str,
    shared_ctx: Option<&ComposeContext>,
    shell_cwd: Option<&std::path::Path>,
) -> Result<ResolvedSystemPrompt, crate::error::ClaudineError> {
    let mode = mode_for_source(&source);
    let composed_markdown = compose_prompt_markdown(&source, raw_text, shared_ctx, shell_cwd)?;
    if composed_markdown.trim().is_empty() {
        return Ok(ResolvedSystemPrompt::Disabled { source });
    }
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
    shared_ctx: Option<&ComposeContext>,
    shell_cwd: Option<&std::path::Path>,
) -> Result<PreparedNonInteractiveAppendix, crate::error::ClaudineError> {
    for (source, raw_text) in candidates {
        let composed_markdown = compose_prompt_markdown(&source, &raw_text, shared_ctx, shell_cwd)?;
        let normalized = composed_markdown.trim().to_string();
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
    let mode = mode_for_source(&source);
    // No launch context here, so shell directives fall back to Darkmatter's
    // source-relative default. The session-aware path supplies a working
    // directory via `resolve_and_prepare_for_session`.
    let composed_markdown = compose_prompt_markdown(&source, raw_text, None, None)?;

    // Empty-body check
    if composed_markdown.trim().is_empty() {
        return Ok(ResolvedSystemPrompt::Disabled { source });
    }

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
            let mode = mode_for_source(&source);
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
                assert_eq!(
                    prepared.composed_markdown,
                    DEFAULT_NON_INTERACTIVE_SYSTEM_PROMPT.trim()
                );
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
