//! Level 2 real-terminal capture for the file-resolution feature's motivating
//! incident and its paired explicit-relative counter-case.
//!
//! Feature: `claudine/features/2026-07-13-file-resolution/` (Phase 8, AC4).
//!
//! ## The motivating incident
//!
//! A router at `<repo>/prompts/router.md` authored a lifecycle hand-off to the
//! **bare** reference `prompts/_implement/implement-suggestions.md`. The target
//! lives at the repository-root-relative location
//! `<repo>/prompts/_implement/implement-suggestions.md`, but Claudine's old
//! private harness grammar joined every non-absolute, non-`@` value onto the
//! source document's directory — and the source already lived in `prompts/`, so
//! the attempted path doubled to `<repo>/prompts/prompts/_implement/...` and the
//! hand-off failed.
//!
//! This suite drives the **real** `claudine` binary through a real tmux pane
//! against a real git worktree and proves the finalized contract end-to-end:
//!
//! - a repository-scoped `^prompts/_implement/implement-suggestions.md`
//!   reaches the repository target, launches the provider, and never produces
//!   the doubled `prompts/prompts` path;
//! - the paired **explicit** `./prompts/_implement/implement-suggestions.md`
//!   stays pinned to the source directory, so it *does* resolve to the doubled
//!   `<repo>/prompts/prompts/...` path — which is absent — and fails with the
//!   typed `Unresolvable file reference` block rather than silently reaching the
//!   repository-root file (AC2).
//!
//! The two cases share one fixture tree and differ only in the sigil, so the
//! pair isolates anchoring: the same target file, source document, and
//! repository differ only by the explicit sigil.

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::{TestWorkspace, augmented_path, clear_no_color, init_git_repo, write, write_executable};

/// A router whose `initialize` stack proxies the repository-scoped motivating
/// reference.
///
/// `^` probes the package scopes and repository root, finding
/// `<repo>/prompts/_implement/implement-suggestions.md`.
const REPOSITORY_SCOPED_ROUTER_DOC: &str = "\
---
title: implicit router
initialize:
  stack:
    - action: {proxy: \"^prompts/_implement/implement-suggestions.md\"}
---
Router body (never used — the run proxies away at initialize).
";

/// The same router, but the reference is **explicit** `./`.
///
/// Explicit ⇒ source-directory only: the single candidate is
/// `<repo>/prompts/prompts/_implement/implement-suggestions.md`, which does not
/// exist, so resolution fails without falling back to the repository root.
const EXPLICIT_ROUTER_DOC: &str = "\
---
title: explicit router
initialize:
  stack:
    - action: {proxy: \"./prompts/_implement/implement-suggestions.md\"}
---
Router body (never used — the run proxies away at initialize).
";

/// A router proxying a **bare** reference that exists at neither anchor, so both
/// implicit candidates miss.
///
/// Source candidate `<repo>/prompts/missing/target.md` and repository candidate
/// `<repo>/missing/target.md` are both absent, so the run fails with the typed
/// block and the report enumerates the two attempted candidates in
/// source-then-repository order.
const NO_MATCH_ROUTER_DOC: &str = "\
---
title: no-match router
initialize:
  stack:
    - action: {proxy: \"missing/target.md\"}
---
Router body (never used — the run proxies away at initialize).
";

/// The proxied target. Benign: no lifecycle, so once the hand-off lands the run
/// composes it and launches the provider stub.
const TARGET_DOC: &str = "\
---
title: implement suggestions
---
Implement the suggestions.
";

struct Staged {
    workspace: TestWorkspace,
    bin_dir: PathBuf,
    router: PathBuf,
    launch_count: PathBuf,
}

/// Stage a git worktree with `<repo>/prompts/router.md` holding `router_body`
/// and the real target at `<repo>/prompts/_implement/implement-suggestions.md`.
///
/// The workspace is a git repo so the wrapper's `sniff` discovery finds the
/// worktree root, which supplies the explicit `^` repository scopes used by
/// the motivating case.
fn stage(name: &str, router_body: &str) -> Staged {
    let workspace = TestWorkspace::named(name);
    let root = workspace.path().to_path_buf();

    assert!(
        init_git_repo(&root),
        "the motivating case needs a real git worktree root for repository-scoped \
         resolution; `git init` failed"
    );

    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let launch_count = root.join("provider-launched.txt");
    // Record the launch and exit cleanly so a resolved hand-off ends in success.
    write_executable(
        &bin_dir.join("claude"),
        "#!/bin/sh\nprintf launched >> \"$CLAUDINE_PROVIDER_LAUNCH_COUNT\"\nexit 0\n",
    );

    let claudine_dir = root.join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    write(&claudine_dir.join("config.json"), "{}");

    let router = root.join("prompts/router.md");
    write(&router, router_body);
    write(
        &root.join("prompts/_implement/implement-suggestions.md"),
        TARGET_DOC,
    );

    Staged {
        workspace,
        bin_dir,
        router,
        launch_count,
    }
}

struct Capture {
    frame: CapturedFrame,
    exit_code: i32,
}

/// Bracket- and glob-free exit marker (see `level2_typed_error_render_capture`).
const EXIT_MARKER: &str = "claudine_rc:";

fn parse_exit_marker(plain: &str) -> Option<i32> {
    plain.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix(EXIT_MARKER)?;
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    })
}

fn wait_for_exit_marker(harness: &mut TmuxHarness, deadline: Duration) -> Capture {
    let stop = Instant::now() + deadline;
    loop {
        let frame = harness.capture().expect("capture pane");
        if let Some(exit_code) = parse_exit_marker(&frame.plain) {
            return Capture { frame, exit_code };
        }
        if Instant::now() >= stop {
            panic!(
                "the `{EXIT_MARKER}<code>` exit marker did not appear within \
                 {deadline:?}.\nplain:\n{}",
                frame.plain
            );
        }
        harness.settle();
    }
}

/// Run `claudine compose --claude <router>` from the worktree root in a real
/// pane and capture the result.
fn run_in_pane(harness: &mut TmuxHarness, staged: &Staged) -> Capture {
    let _ = harness.resize(120, 200);

    // This fixture runs under `FORCE_COLOR=1`, which an ambient `NO_COLOR` would
    // out-vote — see `common::clear_no_color`.
    clear_no_color(harness);

    let claudine = cargo_bin("claudine").display().to_string();
    let home = staged.workspace.path().to_string_lossy().into_owned();
    let path = augmented_path(&staged.bin_dir);
    let path = path.to_string_lossy().into_owned();
    let launch_count = staged.launch_count.to_string_lossy().into_owned();

    harness.send_text(b"clear\n").expect("clear pane");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    harness
        .send_text(format!("cd {}\n", staged.workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let cmd = format!(
        "{claudine} compose --claude {}; echo {EXIT_MARKER}$?",
        staged.router.display()
    );
    let full_env: [(&str, &str); 4] = [
        ("HOME", home.as_str()),
        ("PATH", path.as_str()),
        ("FORCE_COLOR", "1"),
        ("CLAUDINE_PROVIDER_LAUNCH_COUNT", launch_count.as_str()),
    ];
    harness
        .send_command_with_env(&cmd, &full_env)
        .expect("send claudine command");

    let capture = wait_for_exit_marker(harness, Duration::from_secs(30));
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    capture
}

fn has_status_block(frame: &CapturedFrame) -> bool {
    // The `Unresolvable file reference` headline is the resolution-failure block.
    frame.plain.contains("Unresolvable file reference")
}

/// The repository-scoped reference reaches the repository-root target and
/// launches the provider without producing a doubled `prompts/prompts` path.
#[test]
#[serial(level2_terminal)]
fn level2_repository_scoped_reference_resolves_from_repository_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage(
        "claudine-l2-fileres-repository-scoped",
        REPOSITORY_SCOPED_ROUTER_DOC,
    );
    let capture = run_in_pane(&mut harness, &staged);

    assert!(
        !has_status_block(&capture.frame),
        "the repository-scoped reference must resolve from the repository, not fail \
         resolution.\nplain:\n{}",
        capture.frame.plain
    );
    // The doubled path from the motivating bug must never be produced.
    assert!(
        !capture.frame.plain.contains("prompts/prompts"),
        "a repository-scoped reference must not double the `prompts/` segment.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        staged.launch_count.exists(),
        "the resolved hand-off must reach the target and launch the provider.\nplain:\n{}",
        capture.frame.plain
    );
    assert_eq!(
        capture.exit_code, 0,
        "a resolved repository-scoped hand-off ends in success.\nplain:\n{}",
        capture.frame.plain
    );
}

/// AC2: the paired explicit `./` reference stays source-relative. Its single
/// candidate is the doubled `<repo>/prompts/prompts/...` path, which is absent,
/// so it fails with the typed block instead of silently reaching the
/// repository-root file the implicit form resolves to.
#[test]
#[serial(level2_terminal)]
fn level2_explicit_reference_stays_source_relative_and_fails_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-fileres-explicit", EXPLICIT_ROUTER_DOC);
    let capture = run_in_pane(&mut harness, &staged);

    assert!(
        has_status_block(&capture.frame),
        "an explicit `./` reference must fail when its source-relative candidate \
         is absent — it must NOT fall back to the repository root.\nplain:\n{}",
        capture.frame.plain
    );
    // The crux of AC2: the SAME file exists at the repository root
    // (`<repo>/prompts/_implement/implement-suggestions.md`, which the implicit
    // case resolves), yet the explicit form failed — proving `./` never fell
    // back to the repository root. The attempted path re-enters `prompts/`,
    // doubling the segment (the authored `./` segment is preserved verbatim in
    // the rendered candidate, so accept either the normalized or literal form).
    assert!(
        capture.frame.plain.contains("prompts/./prompts")
            || capture.frame.plain.contains("prompts/prompts"),
        "the explicit reference must resolve against the doubled source-relative \
         path, not the repository-root twin.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        capture.frame.plain.contains("does not exist"),
        "the failure must state the missing source-relative candidate.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        !staged.launch_count.exists(),
        "a failed hand-off must not launch the provider.\nplain:\n{}",
        capture.frame.plain
    );
    assert_eq!(
        capture.exit_code, 1,
        "an unresolvable explicit reference must exit 1.\nplain:\n{}",
        capture.frame.plain
    );
}

/// AC8 / D8: an implicit reference that misses at both anchors renders the typed
/// block with the ordered candidate plan — the two attempted candidates in
/// source-then-repository order — not just one candidate. This is the
/// user-observable proof that the detailed resolution record reaches the report
/// instead of being discarded before diagnostics.
#[test]
#[serial(level2_terminal)]
fn level2_implicit_no_match_lists_two_ordered_candidates_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-fileres-nomatch", NO_MATCH_ROUTER_DOC);
    let capture = run_in_pane(&mut harness, &staged);

    assert!(
        has_status_block(&capture.frame),
        "an implicit no-match must render the typed resolution block.\nplain:\n{}",
        capture.frame.plain
    );

    let plain = &capture.frame.plain;
    assert!(
        plain.contains("Tried:"),
        "the report must enumerate the ordered candidate plan.\nplain:\n{plain}"
    );

    // Anchor the ordering check inside the "Tried:" list so the earlier
    // reference/`does not exist` lines (which also name the bare path) cannot
    // satisfy it. Within the list, the `prompts/`-nested source candidate must
    // precede the bare repository candidate.
    let tried = plain
        .split("Tried:")
        .nth(1)
        .expect("the report must contain a Tried section");
    let source_idx = tried
        .find("prompts/missing/target.md")
        .expect("the source candidate must be listed");
    let repo_idx = tried
        .rfind("missing/target.md")
        .expect("the repository candidate must be listed");
    assert!(
        source_idx < repo_idx,
        "candidates must be listed source-then-repository.\nplain:\n{plain}"
    );

    assert!(
        !staged.launch_count.exists(),
        "a no-match hand-off must not launch the provider.\nplain:\n{plain}"
    );
    assert_eq!(
        capture.exit_code, 1,
        "an unresolvable implicit reference must exit 1.\nplain:\n{plain}"
    );
}

struct SurfaceStaged {
    workspace: TestWorkspace,
    bin_dir: PathBuf,
    home: PathBuf,
    provider_log: PathBuf,
    launch_log: PathBuf,
}

fn stage_surface(name: &str) -> SurfaceStaged {
    let workspace = TestWorkspace::named(name);
    let root = workspace.path();
    assert!(init_git_repo(root), "the L2 surface fixture needs a git repository");

    write(
        &root.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"alpha/lib\"]\n",
    );
    write(
        &root.join("alpha/lib/Cargo.toml"),
        "[package]\nname = \"alpha-lib\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    );
    write(&root.join("alpha/lib/src/lib.rs"), "");

    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let provider_log = root.join("provider.log");
    let launch_log = root.join("launches.log");
    write_executable(
        &bin_dir.join("claude"),
        "#!/bin/sh\nprintf 'launch\\n' >> \"$CLAUDINE_L2_LAUNCH_LOG\"\nprintf '%s\\n' \"$*\" >> \"$CLAUDINE_L2_PROVIDER_LOG\"\ncat >> \"$CLAUDINE_L2_PROVIDER_LOG\"\nexit 0\n",
    );

    let home = root.to_path_buf();
    fs::create_dir_all(root.join(".claudine")).unwrap();
    write(&root.join(".claudine/config.json"), "{}");

    SurfaceStaged {
        workspace,
        bin_dir,
        home,
        provider_log,
        launch_log,
    }
}

fn run_surface_command(
    harness: &mut TmuxHarness,
    staged: &SurfaceStaged,
    cwd: &std::path::Path,
    command: &str,
) -> Capture {
    let _ = harness.resize(120, 240);
    clear_no_color(harness);

    harness.send_text(b"clear\n").expect("clear pane");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    harness
        .send_text(format!("cd {}\n", cwd.display()).as_bytes())
        .expect("cd into fixture directory");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let path = augmented_path(&staged.bin_dir);
    let path = path.to_string_lossy().into_owned();
    let home = staged.home.to_string_lossy().into_owned();
    let provider_log = staged.provider_log.to_string_lossy().into_owned();
    let launch_log = staged.launch_log.to_string_lossy().into_owned();
    let full_command = format!("{command}; echo {EXIT_MARKER}$?");
    let full_env: [(&str, &str); 5] = [
        ("HOME", home.as_str()),
        ("PATH", path.as_str()),
        ("FORCE_COLOR", "1"),
        ("CLAUDINE_L2_PROVIDER_LOG", provider_log.as_str()),
        ("CLAUDINE_L2_LAUNCH_LOG", launch_log.as_str()),
    ];
    harness
        .send_command_with_env(&full_command, &full_env)
        .expect("send Claudine surface command");

    let capture = wait_for_exit_marker(harness, Duration::from_secs(45));
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    capture
}

#[test]
#[serial(level2_terminal)]
fn level2_nested_package_compose_resolves_all_finalized_reference_forms_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let staged = stage_surface("claudine-l2-finalized-compose");
    let root = staged.workspace.path();
    let nested = root.join("alpha/lib/prompts/nested");
    write(&root.join("repo-only.md"), "REPOSITORY-ROOT-MARKER\n");
    write(
        &root.join("alpha/lib/package-only.md"),
        "PACKAGE-ROOT-MARKER\n",
    );
    write(
        &root.join("alpha/lib/prompts/magic.md"),
        "MAGIC-PROMPT-MARKER\n",
    );
    write(&nested.join("implicit.md"), "IMPLICIT-SOURCE-MARKER\n");
    let document = nested.join("router.md");
    write(
        &document,
        concat!(
            "---\ntitle: finalized references\n---\n",
            "::file &repo-only.md\n\n",
            "::file ^package-only.md\n\n",
            "::file @magic.md\n\n",
            "::file implicit.md\n",
        ),
    );

    let claudine = cargo_bin("claudine").display().to_string();
    let command = format!("{claudine} compose --claude {}", document.display());
    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let capture = run_surface_command(&mut harness, &staged, root, &command);

    assert_eq!(capture.exit_code, 0, "plain:\n{}", capture.frame.plain);
    let prompt = fs::read_to_string(&staged.provider_log).unwrap_or_default();
    for marker in [
        "REPOSITORY-ROOT-MARKER",
        "PACKAGE-ROOT-MARKER",
        "MAGIC-PROMPT-MARKER",
        "IMPLICIT-SOURCE-MARKER",
    ] {
        assert!(
            prompt.contains(marker),
            "the real compose prompt must contain {marker}; prompt:\n{prompt}\nterminal:\n{}",
            capture.frame.plain
        );
    }
    assert_eq!(
        fs::read_to_string(&staged.launch_log)
            .unwrap_or_default()
            .lines()
            .count(),
        1,
        "compose must launch exactly one provider"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_proxy_keeps_caller_file_parameter_anchored_to_launch_directory_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let staged = stage_surface("claudine-l2-finalized-proxy-parameter");
    let root = staged.workspace.path();
    let launch_dir = root.join("alpha/lib");
    let launch_spec = launch_dir.join("spec.md");
    write(&launch_spec, "LAUNCH-SPEC\n");
    write(&root.join("spec.md"), "REPOSITORY-DECOY\n");
    write(
        &root.join("target.md"),
        "---\n$schema:\n  spec: 'file(eager; required)'\n---\nTARGET SPEC={{ spec }}\n",
    );
    let router = root.join("router.md");
    write(
        &router,
        "---\n$schema:\n  spec: 'file(eager; required)'\ninitialize:\n  stack:\n    - action: {proxy: target.md}\n---\nROUTER\n",
    );

    let claudine = cargo_bin("claudine").display().to_string();
    let command = format!(
        "{claudine} compose --claude {} spec=spec.md",
        router.display()
    );
    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let capture = run_surface_command(&mut harness, &staged, &launch_dir, &command);

    assert_eq!(capture.exit_code, 0, "plain:\n{}", capture.frame.plain);
    let expected = biscuit_file::to_portable_string(
        &launch_spec.canonicalize().expect("canonical launch spec"),
    );
    let prompt = fs::read_to_string(&staged.provider_log).unwrap_or_default();
    assert!(
        prompt.contains(&format!("TARGET SPEC={expected}")),
        "the proxy target must receive the launch-anchored value; prompt:\n{prompt}"
    );
    assert!(
        !prompt.contains("REPOSITORY-DECOY"),
        "the repository decoy must not replace the caller's file value"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_sequence_task_file_parameter_is_anchored_to_sequence_document_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let staged = stage_surface("claudine-l2-finalized-sequence-parameter");
    let root = staged.workspace.path();
    let sequence_dir = root.join("alpha/lib/sequences/nested");
    let authored_spec = sequence_dir.join("task-spec.md");
    write(&authored_spec, "SEQUENCE-AUTHORED-SPEC\n");
    write(&root.join("task-spec.md"), "REPOSITORY-DECOY\n");
    let task = root.join("shared-task.md");
    write(
        &task,
        "---\n$schema:\n  spec: 'file(eager; required)'\n---\nTASK SPEC={{ spec }}\n",
    );
    let sequence = sequence_dir.join("sequence.md");
    write(
        &sequence,
        "---\nsequence:\n  - name: anchored\n    prompt: ^shared-task.md\n    params:\n      spec: task-spec.md\n---\nSEQUENCE\n",
    );

    let claudine = cargo_bin("claudine").display().to_string();
    let command = format!("{claudine} sequence --claude {}", sequence.display());
    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let capture = run_surface_command(&mut harness, &staged, root, &command);

    assert_eq!(capture.exit_code, 0, "plain:\n{}", capture.frame.plain);
    let prompt = fs::read_to_string(&staged.provider_log).unwrap_or_default();
    let actual = prompt
        .lines()
        .find_map(|line| line.strip_prefix("TASK SPEC="))
        .map(PathBuf::from)
        .expect("provider prompt must contain the task spec path");
    assert_eq!(
        actual.canonicalize().expect("canonical provider task spec"),
        authored_spec
            .canonicalize()
            .expect("canonical sequence-authored spec"),
        "the task must receive the sequence-authored file value; prompt:\n{prompt}",
    );
    assert_eq!(
        fs::read_to_string(&staged.launch_log)
            .unwrap_or_default()
            .lines()
            .count(),
        1,
        "the one-task sequence must launch exactly one provider"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_completion_candidate_executes_unchanged_against_the_same_magic_scope_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let staged = stage_surface("claudine-l2-finalized-completion-parity");
    let root = staged.workspace.path();
    let launch_dir = root.join("alpha/lib");
    write(
        &launch_dir.join("prompts/parity.md"),
        "PACKAGE-COMPLETION-MARKER\n",
    );
    write(
        &root.join("prompts/parity.md"),
        "REPOSITORY-COMPLETION-DECOY\n",
    );

    let claudine = cargo_bin("claudine").display().to_string();
    let command = format!(
        "sh -c 'candidate=$({claudine} __complete --current 2 -- claudine compose @parity); test \"$candidate\" = @parity.md && {claudine} compose --claude \"$candidate\"'"
    );
    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let capture = run_surface_command(&mut harness, &staged, &launch_dir, &command);

    assert_eq!(capture.exit_code, 0, "plain:\n{}", capture.frame.plain);
    let launches = fs::read_to_string(&staged.launch_log).unwrap_or_default();
    assert!(
        capture.frame.plain.contains("PACKAGE-COMPLETION-MARKER"),
        "the emitted completion must execute against the package magic root; \
         launches:\n{launches}\nterminal:\n{}",
        capture.frame.plain
    );
    assert!(
        !capture.frame.plain.contains("REPOSITORY-COMPLETION-DECOY"),
        "completion and execution must agree on the closest candidate"
    );
    assert_eq!(launches.lines().count(), 1, "completion execution must launch once");
}
