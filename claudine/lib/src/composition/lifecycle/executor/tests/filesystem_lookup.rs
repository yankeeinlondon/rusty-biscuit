//! filesystem lookup executor tests.

use super::*;

#[test]
fn ctx_scan_hint_descends_container_literals() {
    use darkmatter::markdown::compose::expression::parse;

    let array = parse("[ctx.area, ctx.package]").expect("array literal must parse");
    let hint = ctx_scan_hint(&array);
    assert!(hint.contains("ctx.area"), "got: {hint}");
    assert!(hint.contains("ctx.package"), "got: {hint}");

    let object = parse("{ ctx: ctx.agent }").expect("object literal must parse");
    assert_eq!(ctx_scan_hint(&object), "ctx.agent");
}

#[test]
#[serial_test::serial(file_resolution_snapshot)]
fn lifecycle_file_functions_reuse_all_request_resolution_inputs() {
    let request = tempfile::tempdir().unwrap();
    let source_dir = request.path().join("prompts");
    let home = request.path().join("home");
    let magic = request.path().join("magic");
    let package = request.path().join("package");
    for dir in [&source_dir, &home, &magic, &package] {
        std::fs::create_dir_all(dir).unwrap();
    }
    for path in [
        request.path().join("env.flag"),
        home.join("home.flag"),
        magic.join("magic.flag"),
        package.join("package.flag"),
    ] {
        std::fs::write(path, "ready").unwrap();
    }
    let source_path = source_dir.join("prompt.md");
    let mut env = std::collections::HashMap::new();
    env.insert(
        "CLAUDINE_SNAPSHOT_ROOT".to_string(),
        request.path().display().to_string(),
    );
    let snapshot = biscuit_file::FileResolutionContext::from_snapshot(
        request.path(),
        Some(home.clone()),
        env,
    )
    .with_repository_root(request.path())
    .with_package_area(&package)
    .add_magic_path(&magic, biscuit_file::PathPosition::Start);

    let prior_root = std::env::var_os("CLAUDINE_SNAPSHOT_ROOT");
    let ambient = tempfile::tempdir().unwrap();
    // SAFETY: the test is serialized while process-global state is changed.
    unsafe { std::env::set_var("CLAUDINE_SNAPSHOT_ROOT", ambient.path()) };

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = Map::new();
    let context = StackExecutionContext {
        signal: LifecycleSignal::Start,
        frontmatter: &fm,
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        base_dir: Some(&source_dir),
        ctx_base_dir: None,
        prepared_context: None,
        file_resolution_context: Some(&snapshot),
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: Some(request.path()),
        messaging: &harness.messaging,
        settings: &harness.settings,
    };
    for expression in [
        "file_exists('{{CLAUDINE_SNAPSHOT_ROOT}}/env.flag')",
        "file_exists('~/home.flag')",
        "file_exists('@magic.flag')",
        "file_exists('!package.flag')",
    ] {
        let parsed = darkmatter::markdown::compose::expression::parse(expression).unwrap();
        assert_eq!(context.eval_expr(&parsed, &fm).unwrap(), Value::Bool(true));
    }

    match prior_root {
        Some(value) => unsafe { std::env::set_var("CLAUDINE_SNAPSHOT_ROOT", value) },
        None => unsafe { std::env::remove_var("CLAUDINE_SNAPSHOT_ROOT") },
    }
}

/// `ctx.*` capture follows the launch area rather than the prompt's parent.
/// Each temporary directory is its own Git repository, making the selected
/// root deterministic without relying on the workspace layout.
#[test]
fn ctx_capture_follows_ctx_base_dir_not_base_dir() {
    let git_init = |dir: &Path| {
        let ok = std::process::Command::new("git")
            .arg("init")
            .arg("--quiet")
            .current_dir(dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "git init must succeed in {}", dir.display());
    };

    let ctx_dir = tempfile::tempdir().unwrap();
    let base_dir = tempfile::tempdir().unwrap();
    git_init(ctx_dir.path());
    git_init(base_dir.path());

    // Canonicalize: macOS temp dirs are symlinks (`/var` → `/private/var`),
    // and sniff reports the canonical repo root.
    let ctx_root = dunce::canonicalize(ctx_dir.path()).unwrap();
    let base_root = dunce::canonicalize(base_dir.path()).unwrap();

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = Map::new();
    let source_path = ctx_root.join("prompt.md");

    let context = StackExecutionContext {
        signal: LifecycleSignal::Start,
        frontmatter: &fm,
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        // `base_dir` deliberately differs from `ctx_base_dir` so a leak back
        // to `base_dir` would resolve to `base_root` and fail the assert.
        base_dir: Some(base_root.as_path()),
        ctx_base_dir: Some(ctx_root.as_path()),
        // No prepared snapshot: exercise the fallback re-capture path so the
        // assertion proves `ctx_base_dir` (not `base_dir`) roots the capture.
        prepared_context: None,
        file_resolution_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };

    let resolved = context
        .resolve_string_value("{{ctx.repo_root}}", &fm)
        .expect("ctx.repo_root resolves");
    let resolved = resolved.as_str().unwrap_or_default();
    assert_eq!(
        resolved,
        biscuit_file::to_portable_string(&ctx_root),
        "ctx.* must capture against ctx_base_dir (launch area), not base_dir"
    );
    assert_ne!(
        resolved,
        base_root.to_string_lossy(),
        "ctx.* must not leak to base_dir"
    );
}

/// End-to-end of the exact layout that let the bug regress: a prompt living
/// OUTSIDE any area (`<repo>/prompts`) while the run was launched FROM a
/// different area. The single composition-start snapshot is captured against
/// the launch area and threaded as `prepared_context`; the lifecycle event
/// reuses it for `{{ctx.*}}` instead of re-capturing against the prompt's
/// parent (`base_dir`).
///
/// Probes `ctx.repo_root` (directory-sensitive, only needs `git init`).
/// The snapshot is rooted at `launch_root`; `base_dir` points at the
/// prompt's parentless-of-area `prompts/` dir inside a *different* repo, so
/// the pre-fix re-capture would have produced `base_root`, not `launch_root`.
#[test]
fn lifecycle_reuses_prepared_snapshot_for_prompt_outside_launch_area() {
    let git_init = |dir: &Path| {
        let ok = std::process::Command::new("git")
            .arg("init")
            .arg("--quiet")
            .current_dir(dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(ok, "git init must succeed in {}", dir.display());
    };

    // The launch area: the package area the caller launched from.
    let launch_dir = tempfile::tempdir().unwrap();
    git_init(launch_dir.path());
    let launch_root = dunce::canonicalize(launch_dir.path()).unwrap();

    // A separate repo whose `prompts/` subdir holds the prompt file — the
    // "prompt outside any area" shape. `base_dir` points here.
    let prompt_repo = tempfile::tempdir().unwrap();
    git_init(prompt_repo.path());
    let prompt_repo_root = dunce::canonicalize(prompt_repo.path()).unwrap();
    let prompts_dir = prompt_repo_root.join("prompts");
    std::fs::create_dir(&prompts_dir).unwrap();
    let source_path = prompts_dir.join("implement-plan.md");

    // The single composition-start snapshot, captured ONCE against the
    // launch area (mirrors what the CLI does in `compose/prep.rs`).
    let prepared = ComposeContext::capture_for_content(
        launch_root.as_path(),
        "{{ ctx.repo_root }}",
    );

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = Map::new();

    let context = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: &fm,
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        // The prompt's parent — inside a different repo, no area.
        base_dir: Some(prompts_dir.as_path()),
        ctx_base_dir: Some(launch_root.as_path()),
        // The reused snapshot is the source of truth.
        prepared_context: Some(&prepared),
        file_resolution_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };

    let resolved = context
        .resolve_string_value("{{ctx.repo_root}}", &fm)
        .expect("ctx.repo_root resolves");
    let resolved = resolved.as_str().unwrap_or_default();
    assert_eq!(
        resolved,
        biscuit_file::to_portable_string(&launch_root),
        "lifecycle must reuse the launch-area snapshot, not the prompt dir"
    );
    assert_ne!(
        resolved,
        prompt_repo_root.to_string_lossy(),
        "lifecycle ctx.* must not resolve against the prompt's own repo"
    );
}

/// A file reference authored inside a document resolves against the document's
/// own directory (`base_dir`) after the ambient process CWD has moved away —
/// the core post-`chdir` independence contract (D2).
///
/// Layout: `spec.md` lives under `base_dir` (the prompt's parent); a same-named
/// file under the launch area is NOT a resolution candidate — the launch-area
/// fallback is diagnostic-only for in-document references (D2). Pre-flip this
/// test asserted the launch area drove resolution; repository-first replaced
/// that with document-dir anchoring.
#[serial_test::serial]
#[test]
fn file_exists_resolves_against_base_dir_after_chdir() {
    let launch_dir = tempfile::tempdir().unwrap();
    let prompt_dir = tempfile::tempdir().unwrap();
    let unrelated_dir = tempfile::tempdir().unwrap();

    // The reference resolves against the document's own directory.
    std::fs::write(prompt_dir.path().join("spec.md"), "# spec\n").unwrap();
    // A file present ONLY under the launch area must NOT resolve.
    std::fs::write(launch_dir.path().join("launch_only.md"), "# launch\n").unwrap();

    // Move the ambient CWD to an unrelated dir (mirrors the wrapper's
    // `switch_process_cwd` to the repo root before lifecycle events fire).
    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(unrelated_dir.path()).unwrap();

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = map(json!({ "spec": "spec.md", "launch_only": "launch_only.md" }));
    let source_path = prompt_dir.path().join("prompt.md");

    let context = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: &fm,
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        // The prompt's parent — the document base holding spec.md.
        base_dir: Some(prompt_dir.path()),
        // The launch area — carried for diagnostics only, never a candidate.
        ctx_base_dir: Some(launch_dir.path()),
        prepared_context: None,
        file_resolution_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };

    let resolved = context
        .resolve_string_value("{{file_exists(spec)}}", &fm)
        .expect("file_exists(spec) must resolve");
    assert_eq!(
        resolved,
        Value::Bool(true),
        "file_exists must anchor on base_dir (document dir), independent of ambient CWD"
    );

    // The launch-area fallback is diagnostic-only: a file present only there
    // is not a resolution candidate.
    let launch_only = context
        .resolve_string_value("{{file_exists(launch_only)}}", &fm)
        .expect("file_exists(launch_only) must resolve to a verdict");
    assert_eq!(
        launch_only,
        Value::Bool(false),
        "a launch-area-only file must NOT resolve; the launch fallback is diagnostic-only"
    );

    // Restore the ambient CWD so other tests are unaffected.
    std::env::set_current_dir(&original_cwd).unwrap();
}

/// Prepare-time and event-time resolution agree for the same caller-supplied
/// path: both anchor on `base_dir` (the document directory) and both carry the
/// same diagnostic launch-area fallback. This asserts the event-time
/// `StackExecutionContext` path (`file_exists` → `true`) matches what the
/// `ResolutionContext` builder alone produces — the two paths share one
/// explicit anchor instead of diverging on ambient-CWD timing.
#[serial_test::serial]
#[test]
fn prepare_time_and_event_time_agree_on_file_reference() {
    let launch_dir = tempfile::tempdir().unwrap();
    let prompt_dir = tempfile::tempdir().unwrap();

    // The reference resolves against the document's own directory (base_dir).
    std::fs::write(prompt_dir.path().join("plan.md"), "# plan\n").unwrap();

    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(prompt_dir.path()).unwrap();

    // Prepare-time resolution context (mirrors what ComposeOptions builds):
    let prepare_ctx = ResolutionContext::new(prompt_dir.path().to_path_buf())
        .with_file_ref_fallback_dir(launch_dir.path().to_path_buf());

    // Event-time resolution context (built by StackExecutionContext):
    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = map(json!({ "spec": "plan.md" }));
    let source_path = prompt_dir.path().join("prompt.md");
    let context = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: &fm,
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        base_dir: Some(prompt_dir.path()),
        ctx_base_dir: Some(launch_dir.path()),
        prepared_context: None,
        file_resolution_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };
    let event_ctx = context.resolution_context();

    // Both contexts carry the same fallback directory.
    assert_eq!(
        prepare_ctx.file_ref_fallback_dir, event_ctx.file_ref_fallback_dir,
        "prepare-time and event-time must share the launch-area fallback"
    );
    // Both base dirs point at the prompt's parent.
    assert_eq!(prepare_ctx.base_dir, event_ctx.base_dir);

    // The event-time file_exists agrees with the prepare-time anchor.
    let resolved = context
        .resolve_string_value("{{file_exists(spec)}}", &fm)
        .expect("file_exists(spec) resolves");
    assert_eq!(resolved, Value::Bool(true));

    std::env::set_current_dir(&original_cwd).unwrap();
}

/// `frontmatter(spec, review_iterations)` resolves against `base_dir` (the
/// document directory) — the mechanism behind `iteration` derivation in prompts
/// like `review-feature.md`. Under repository-first resolution the spec is read
/// from the document's own directory, not the launch-area fallback.
#[serial_test::serial]
#[test]
fn frontmatter_reads_resolve_against_base_dir() {
    let launch_dir = tempfile::tempdir().unwrap();
    let prompt_dir = tempfile::tempdir().unwrap();

    // The spec file carries a `review_iterations` frontmatter property and
    // lives under the document's own directory (base_dir).
    std::fs::write(
        prompt_dir.path().join("spec.md"),
        "---\nreview_iterations: 3\n---\n# spec\n",
    )
    .unwrap();

    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(prompt_dir.path()).unwrap();

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = map(json!({ "spec": "spec.md" }));
    let source_path = prompt_dir.path().join("prompt.md");

    let context = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: &fm,
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        base_dir: Some(prompt_dir.path()),
        ctx_base_dir: Some(launch_dir.path()),
        prepared_context: None,
        file_resolution_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };

    let resolved = context
        .resolve_string_value("{{frontmatter(spec, 'review_iterations')}}", &fm)
        .expect("frontmatter(spec, 'review_iterations') resolves");
    assert_eq!(
        resolved,
        Value::Number(3.into()),
        "frontmatter() must read the spec from base_dir (the document directory)"
    );

    std::env::set_current_dir(&original_cwd).unwrap();
}

/// A caller-supplied file that exists ONLY under the launch area does NOT
/// resolve: the launch-area fallback is diagnostic-only and is never a
/// resolution candidate for a reference authored inside a document (D2). This
/// is the inverse of the pre-flip contract, which resolved such a file through
/// the fallback.
///
/// Three distinct anchors are materialized so the test cannot pass by
/// accident: `prompt_dir` (base_dir, empty), `repo_root_dir` (the
/// post-`chdir` ambient CWD, empty), and `launch_dir` (ctx_base_dir /
/// fallback, holds the only copy — yet must not be consulted).
#[serial_test::serial]
#[test]
fn regression_path_only_under_launch_area_does_not_resolve() {
    let launch_dir = tempfile::tempdir().unwrap();
    let prompt_dir = tempfile::tempdir().unwrap();
    let repo_root_dir = tempfile::tempdir().unwrap();

    // The caller-supplied file lives ONLY under the launch area.
    std::fs::write(launch_dir.path().join("unique.md"), "# unique\n").unwrap();
    // Defensive sanity: neither the base dir nor the ambient CWD holds it.
    assert!(!prompt_dir.path().join("unique.md").exists());
    assert!(!repo_root_dir.path().join("unique.md").exists());

    // The wrapper's `switch_process_cwd` repositions the process to the
    // repo root before lifecycle events fire.
    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(repo_root_dir.path()).unwrap();

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = map(json!({ "spec": "unique.md" }));
    let source_path = prompt_dir.path().join("prompt.md");

    let context = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: &fm,
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        base_dir: Some(prompt_dir.path()),
        ctx_base_dir: Some(launch_dir.path()),
        prepared_context: None,
        file_resolution_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };

    let resolved = context
        .resolve_string_value("{{file_exists(spec)}}", &fm)
        .expect("file_exists(spec) must resolve to a verdict");
    assert_eq!(
        resolved,
        Value::Bool(false),
        "a file present only under the launch area must NOT resolve; the launch-area fallback \
         is diagnostic-only, never a candidate (neither prompt dir nor repo root holds it)",
    );

    std::env::set_current_dir(&original_cwd).unwrap();
}

/// A same-named launch-area file does not displace the source-local candidate
/// when the request snapshot has no repository candidate.
///
/// Each copy carries a distinct `title` frontmatter property so the
/// `frontmatter(spec, 'title')` value identifies which file won.
#[serial_test::serial]
#[test]
fn source_local_candidate_ignores_same_named_launch_file() {
    let launch_dir = tempfile::tempdir().unwrap();
    let prompt_dir = tempfile::tempdir().unwrap();
    let repo_root_dir = tempfile::tempdir().unwrap();

    // Both anchors hold a same-named spec.md with distinct titles.
    std::fs::write(
        prompt_dir.path().join("spec.md"),
        "---\ntitle: from-prompt-dir\n---\n# prompt\n",
    )
    .unwrap();
    std::fs::write(
        launch_dir.path().join("spec.md"),
        "---\ntitle: from-launch-area\n---\n# launch\n",
    )
    .unwrap();

    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(repo_root_dir.path()).unwrap();

    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let fm = map(json!({ "spec": "spec.md" }));
    let source_path = prompt_dir.path().join("prompt.md");

    let context = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: &fm,
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        base_dir: Some(prompt_dir.path()),
        ctx_base_dir: Some(launch_dir.path()),
        prepared_context: None,
        file_resolution_context: None,
        effect_engine: &engine,
        shell_runner: &shell,
        emitter: &recorder,
        term: &harness.term,
        source_path: &source_path,
        repo_root: None,
        messaging: &harness.messaging,
        settings: &harness.settings,
    };

    let resolved = context
        .resolve_string_value("{{frontmatter(spec, 'title')}}", &fm)
        .expect("frontmatter(spec, 'title') must resolve");
    assert_eq!(
        resolved,
        Value::String("from-prompt-dir".to_string()),
        "the source-local candidate must win because launch metadata is not searched",
    );

    std::env::set_current_dir(&original_cwd).unwrap();
}
