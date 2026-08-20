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

/// Build an executable in `dir` that prints its working directory, and return
/// its portable path text for use as both a `::shell` command and a whitelist
/// `prefix` rule.
///
/// The probe reports the portable spelling because its output lands in a
/// Markdown document: a native Windows path would lose the separator of any
/// segment beginning with punctuation to CommonMark backslash escaping.
fn compile_cwd_probe(dir: &std::path::Path) -> String {
    std::fs::create_dir_all(dir).unwrap();
    let source = write_temp_file(
        dir,
        "cwd_probe.rs",
        "fn main() {\n    let cwd = std::env::current_dir().unwrap();\n    \
         println!(\"{}\", cwd.display().to_string().replace('\\\\', \"/\"));\n}\n",
    );
    let executable = dir.join(format!("cwd-probe{}", std::env::consts::EXE_SUFFIX));
    let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let compilation = std::process::Command::new(compiler)
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compilation.status.success(),
        "failed to compile cwd probe: {}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    biscuit_file::to_portable_string(&executable)
}

struct ScopedHome {
    original: Option<std::ffi::OsString>,
}

impl ScopedHome {
    fn set(path: &std::path::Path) -> Self {
        let original = std::env::var_os("HOME");
        // SAFETY: Callers use `#[serial]`, and the guard restores the exact
        // prior value even if the test unwinds.
        unsafe { std::env::set_var("HOME", path) };
        Self { original }
    }
}

impl Drop for ScopedHome {
    fn drop(&mut self) {
        // SAFETY: The owning test is serialized for this process-global value.
        unsafe {
            match self.original.as_ref() {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }
    }
}

#[test]
fn shared_context_uses_request_owned_repo_and_os_evidence() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .status()
        .unwrap();
    assert!(status.success());
    let source = SystemPromptSource::ExplicitFile {
        path: write_temp_file(tmp.path(), "prompt.md", "{{ ctx.area }} / {{ ctx.os }}"),
        mode: SystemPromptMode::Append,
    };
    let invocation = crate::invocation_context::InvocationContext::capture_at(&root);
    let context = invocation.launch_context();
    let input = ResolvedPromptInput::capture(
        (source, "{{ ctx.area }} / {{ ctx.os }}".to_string()),
        &invocation,
    )
    .unwrap();

    let shared = build_shared_compose_context_with_invocation(
        Some(&input),
        None,
        &context,
        &invocation,
    )
    .unwrap();
    assert!(shared.runtime.get("repo").is_some());
    assert!(shared.runtime.get("os").is_some());

    let result = prepare_system_prompt_with_ctx(input, Some(&shared), None).unwrap();
    match result {
        ResolvedSystemPrompt::Ready(prepared) => {
            assert!(
                prepared.composed_markdown.starts_with("/ "),
                "got {:?}",
                prepared.composed_markdown
            );
            assert!(!prepared.composed_markdown.contains("{{ ctx."));
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
        "---\nmode: replace\n---\n\nReplacement base.\n\n::file ./primary-one.md\n\n::file ./primary-two.md",
    )
    .unwrap();
    std::fs::write(repo.join("primary-one.md"), "Primary one.").unwrap();
    std::fs::write(repo.join("primary-two.md"), "Primary two.").unwrap();
    std::fs::write(
        repo.join(".claudine").join("non-interactive.md"),
        "Repo appendix.\n\n::file ./appendix-fragment.md",
    )
    .unwrap();
    std::fs::write(
        repo.join(".claudine").join("appendix-fragment.md"),
        "Appendix fragment.",
    )
    .unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&repo)
        .status()
        .unwrap();
    assert!(status.success());

    let _home = ScopedHome::set(&home);

    let args = SystemPromptArgs::default();
    let invocation = crate::invocation_context::InvocationContext::capture_at(&repo);
    let context = invocation.launch_context();

    let result = resolve_and_prepare_for_session_with_context(
        &args,
        &context,
        true,
        &invocation,
    )
    .unwrap();
    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 1);
    assert_eq!(work.topology_probes, 1);
    assert_eq!(work.topology_reuses, 2);
    assert_eq!(work.system_prompt_lookups, 1);
    assert_eq!(work.compose_operations, 2);
    assert_eq!(work.ambient_fallbacks, 0);
    for stage in [
        "lookup",
        "runtime capture",
        "primary compose",
        "appendix compose",
    ] {
        assert!(work.system_prompt_timings.contains_key(stage));
    }

    match result {
        ResolvedSystemPrompt::Ready(prepared) => {
            assert_eq!(prepared.mode, SystemPromptMode::Replace);
            assert!(prepared.composed_markdown.contains("Replacement base."));
            assert!(prepared.composed_markdown.contains("Primary one."));
            assert!(prepared.composed_markdown.contains("Primary two."));
            assert!(prepared.composed_markdown.contains("Repo appendix."));
            assert!(prepared.composed_markdown.contains("Appendix fragment."));
        }
        other => panic!("Expected Ready, got {other:?}"),
    }
}

#[test]
fn session_reuses_resolved_external_source_context() {
    let tmp = TempDir::new().unwrap();
    let launch = tmp.path().join("launch");
    let source_repo = tmp.path().join("source-repo");
    std::fs::create_dir_all(&launch).unwrap();
    std::fs::create_dir_all(&source_repo).unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&source_repo)
        .status()
        .unwrap();
    assert!(status.success());
    let prompt = write_temp_file(&source_repo, "prompt.md", "External prompt.");

    let invocation = crate::invocation_context::InvocationContext::capture_at(&launch);
    let context = invocation.launch_context();
    let args = SystemPromptArgs {
        append_file: Some(prompt.display().to_string()),
        ..Default::default()
    };

    let result = resolve_and_prepare_for_session_with_context(
        &args,
        &context,
        false,
        &invocation,
    )
    .unwrap();
    assert!(matches!(result, ResolvedSystemPrompt::Ready(_)));

    let work = invocation.work_snapshot();
    assert_eq!(work.git_root_discoveries, 2);
    assert_eq!(work.topology_probes, 1);
    assert_eq!(work.topology_reuses, 0);
    assert_eq!(work.compose_operations, 1);
    assert_eq!(work.ambient_fallbacks, 0);
}

#[test]
fn non_repository_session_runs_shell_in_launch_cwd() {
    let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    if !std::process::Command::new(compiler)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        eprintln!("skipping cwd probe test because rustc is unavailable");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let launch = tmp.path().join("launch");
    let source_repo = tmp.path().join("source");
    std::fs::create_dir_all(&launch).unwrap();
    std::fs::create_dir_all(&source_repo).unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&source_repo)
        .status()
        .unwrap();
    assert!(status.success());
    // A compiled probe rather than `pwd`: the built-in shell blacklist rejects
    // every Windows shell that could report a working directory, and no
    // whitelist can override it.
    let probe = compile_cwd_probe(&tmp.path().join("probe"));
    write_temp_file(
        &source_repo,
        ".darkmatter-shell-whitelist",
        &format!("prefix {probe}\n"),
    );
    let prompt = write_temp_file(
        &source_repo,
        "prompt.md",
        &format!("Before.\n\n::shell \"{probe}\"\n\nAfter."),
    );

    let invocation = crate::invocation_context::InvocationContext::capture_at(&launch);
    let context = invocation.launch_context();
    assert!(context.repo_root.is_none());
    let args = SystemPromptArgs {
        append_file: Some(prompt.display().to_string()),
        ..Default::default()
    };

    let result = resolve_and_prepare_for_session_with_context(
        &args,
        &context,
        false,
        &invocation,
    )
    .unwrap();
    let ResolvedSystemPrompt::Ready(prepared) = result else {
        panic!("expected a prepared system prompt");
    };
    assert!(
        prepared
            .composed_markdown
            .contains(biscuit_file::to_portable_string(&launch).as_str()),
        "shell did not run in launch cwd: {}",
        prepared.composed_markdown
    );
    assert!(
        !prepared
            .composed_markdown
            .contains(biscuit_file::to_portable_string(&source_repo).as_str()),
        "shell ran beside the prompt source: {}",
        prepared.composed_markdown
    );
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
