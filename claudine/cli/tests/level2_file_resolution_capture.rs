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
//! against a real git worktree and proves the ratified contract end-to-end:
//!
//! - a bare reference is **implicit** — it resolves repository-root first, so the
//!   hand-off reaches `<repo>/prompts/_implement/...`, the provider launches, and
//!   the doubled `prompts/prompts` path never appears (AC3, AC4);
//! - the paired **explicit** `./prompts/_implement/implement-suggestions.md`
//!   stays pinned to the source directory, so it *does* resolve to the doubled
//!   `<repo>/prompts/prompts/...` path — which is absent — and fails with the
//!   typed `Unresolvable file reference` block rather than silently reaching the
//!   repository-root file (AC2).
//!
//! The two cases share one fixture tree and differ only in the sigil, so the
//! pair isolates precedence: the same target file, the same source document, the
//! same repository — only implicit-vs-explicit changes the outcome.

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

/// A router whose `initialize` stack proxies the **bare** motivating reference.
///
/// Bare ⇒ implicit ⇒ repository-first: candidate 1 is
/// `<repo>/prompts/_implement/implement-suggestions.md` (exists); the
/// source-relative candidate `<repo>/prompts/prompts/_implement/...` is never
/// reached.
const IMPLICIT_ROUTER_DOC: &str = "\
---
title: implicit router
initialize:
  stack:
    - action: {proxy: \"prompts/_implement/implement-suggestions.md\"}
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
/// Repository candidate `<repo>/missing/target.md` and source candidate
/// `<repo>/prompts/missing/target.md` are both absent, so the run fails with the
/// typed block and the report enumerates the two attempted candidates in
/// repository-then-source order (spec §D8).
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
/// worktree root — implicit references anchor on it first, which is the entire
/// point of the motivating case.
fn stage(name: &str, router_body: &str) -> Staged {
    let workspace = TestWorkspace::named(name);
    let root = workspace.path().to_path_buf();

    assert!(
        init_git_repo(&root),
        "the motivating case needs a real git worktree root for repository-first \
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

    let claudine = cargo_bin!("claudine").display().to_string();
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

/// AC4 / AC3: the bare (implicit) reference resolves repository-first, so the
/// hand-off reaches the repository-root target, the provider launches, and the
/// doubled `prompts/prompts` path never appears.
#[test]
#[serial(level2_terminal)]
fn level2_implicit_reference_resolves_repository_first_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-l2-fileres-implicit", IMPLICIT_ROUTER_DOC);
    let capture = run_in_pane(&mut harness, &staged);

    assert!(
        !has_status_block(&capture.frame),
        "the bare motivating reference must resolve repository-first, not fail \
         resolution.\nplain:\n{}",
        capture.frame.plain
    );
    // The doubled path from the motivating bug must never be produced.
    assert!(
        !capture.frame.plain.contains("prompts/prompts"),
        "an implicit reference must not double the `prompts/` segment.\nplain:\n{}",
        capture.frame.plain
    );
    assert!(
        staged.launch_count.exists(),
        "the resolved hand-off must reach the target and launch the provider.\nplain:\n{}",
        capture.frame.plain
    );
    assert_eq!(
        capture.exit_code, 0,
        "a resolved repository-first hand-off ends in success.\nplain:\n{}",
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
/// repository-then-source order — not just the repository winner. This is the
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
    // satisfy it. Within the list, the bare repository candidate must precede
    // the `prompts/`-nested source candidate.
    let tried = plain
        .split("Tried:")
        .nth(1)
        .expect("the report must contain a Tried section");
    let repo_idx = tried
        .find("missing/target.md")
        .expect("the repository candidate must be listed");
    let source_idx = tried
        .find("prompts/missing/target.md")
        .expect("the source candidate must be listed");
    assert!(
        repo_idx < source_idx,
        "candidates must be listed repository-then-source.\nplain:\n{plain}"
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
