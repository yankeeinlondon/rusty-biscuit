//! D10 characterization baseline for the error-propagation transport migration.
//!
//! Feature: `claudine/features/2026-07-13-error-propogation/` (Phase 1).
//!
//! Spec §D10 requires that typed propagation "must not accidentally alter
//! process exit status, lifecycle event ordering, retry/resume/proxy decisions,
//! or whether a failure is emitted once", and that each route be characterized
//! *before* it is changed. This file is that characterization: it drives the
//! real `claudine` binary through every route Phases 4 and 5 will touch and
//! pins the three properties that must survive the migration byte-for-byte.
//!
//! ## What this file pins — and what it deliberately does not
//!
//! Pinned (must be identical before and after every later phase):
//!
//! - **process exit code**
//! - **lifecycle event order** — the ordered `events.log` markers
//! - **emission count** — the failure is surfaced exactly once
//!
//! Deliberately **not** pinned:
//!
//! - **Error text, headline, or block shape.** Phases 4/5 intentionally change
//!   these — that is the entire point of the feature. A test that pinned them
//!   would fail by design and teach the next phase to weaken this baseline.
//! - **`err.msg` content.** D10 lists "the documented `err.msg` change to the
//!   effective diagnostic's concise message projection" as an *intended*
//!   change. `inventory.md` records the current values instead.
//!
//! The distinction matters: this suite's job is to prove the migration was
//! behavior-neutral where it claims to be, not to freeze the behavior it claims
//! to improve.
//!
//! ## Placement
//!
//! Un-prefixed, so it runs under `just test` (the `!level2_` filterset excludes
//! only `level2_`/`level3_` binaries). It spawns the real binary but needs no
//! TTY, tmux, or PTY — every route here reaches its failure headlessly, which
//! is what keeps the baseline non-flaky. Flakiness here would be
//! disqualifying: Checkpoint 1 forbids proceeding on a flaky baseline, because
//! a flaky baseline cannot prove D10 neutrality.

use assert_cmd::cargo::cargo_bin_cmd;
use std::path::{Path, PathBuf};

mod common;
use common::{TestWorkspace, augmented_path, strip_ansi, write, write_executable};

// --- Provider shims -------------------------------------------------------

#[cfg(windows)]
fn shim_name(name: &str) -> String {
    format!("{name}.cmd")
}

#[cfg(not(windows))]
fn shim_name(name: &str) -> String {
    name.to_string()
}

#[cfg(windows)]
fn provider_shim(exit_code: i32) -> String {
    format!("@echo off\r\nexit /b {exit_code}\r\n")
}

#[cfg(not(windows))]
fn provider_shim(exit_code: i32) -> String {
    format!("#!/bin/sh\nexit {exit_code}\n")
}

// --- Harness --------------------------------------------------------------

/// One route's externally observable failure behavior.
struct RouteOutcome {
    exit_code: Option<i32>,
    /// Ordered `events.log` markers; empty when the route failed before any
    /// lifecycle stack ran.
    events: Vec<String>,
    stderr_plain: String,
}

impl RouteOutcome {
    /// How many times the failure was surfaced to the user.
    ///
    /// Counts both surfaces the CLI can use — the generic `color_eyre`
    /// `Error:` line and a rendered `BlockError` status block — because a
    /// route may legitimately migrate from one to the other while still
    /// needing to emit exactly once. Summing them is what makes the
    /// exactly-once assertion survive that migration (§D5).
    ///
    /// A status block is detected **structurally**: an error glyph line
    /// followed by a `┃` body line. The glyph alone is not sufficient — the
    /// wrapper also prints ordinary status lines with it (`⤫ agent exited with
    /// error code 3`), and counting those would conflate a provider's exit
    /// report with an error emission.
    fn emission_count(&self) -> usize {
        let lines: Vec<&str> = self.stderr_plain.lines().collect();

        let generic = lines
            .iter()
            .filter(|line| line.trim_start().starts_with("Error:"))
            .count();

        let blocks = lines
            .windows(2)
            .filter(|pair| {
                pair[0].trim_start().starts_with('\u{292B}')
                    && pair[1].trim_start().starts_with('\u{2503}')
            })
            .count();

        generic + blocks
    }
}

/// Run `claudine compose --claude <doc>` in an isolated workspace.
fn run_route(doc_body: &str, provider_exit: i32, extra_args: &[&str]) -> RouteOutcome {
    let workspace = TestWorkspace::named("claudine-characterization");
    let root = workspace.path();
    let bin_dir = root.join("bin");
    write_executable(
        &bin_dir.join(shim_name("claude")),
        &provider_shim(provider_exit),
    );

    let doc = root.join("route.md");
    write(&doc, doc_body);

    let mut args: Vec<&str> = vec!["compose", "--claude"];
    args.extend_from_slice(extra_args);
    let doc_arg = doc.to_str().unwrap().to_string();
    args.push(&doc_arg);

    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("PATH", augmented_path(&bin_dir))
        .current_dir(root)
        .args(&args)
        .output()
        .expect("spawn claudine");

    let stderr_plain = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let events = read_events(root);

    RouteOutcome {
        exit_code: output.status.code(),
        events,
        stderr_plain,
    }
}

/// Read the ordered lifecycle markers.
///
/// Searches the workspace rather than assuming a fixed location: the wrapper
/// may switch CWD to a discovered repo root, which moves where a relative
/// `append_line` target lands. Searching keeps the baseline stable on hosts
/// whose temp dir has a `.git` ancestor.
fn read_events(root: &Path) -> Vec<String> {
    let Some(path) = find_events_log(root) else {
        return Vec::new();
    };
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .filter(|line| !line.is_empty())
        .collect()
}

fn find_events_log(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        } else if path.file_name().is_some_and(|n| n == "events.log") {
            return Some(path);
        }
    }
    subdirs.into_iter().find_map(|d| find_events_log(&d))
}

// --- Route 1: `initialize` proxy resolution failure -----------------------

/// The motivating incident (spec §"Motivating Incident").
///
/// `initialize.stack` proxies to a target that does not exist.
/// `composition::resolve_proxy_target` returns a typed
/// `HarnessError::PathResolutionFailed`, which `pipeline.rs:1162` flattens with
/// `eyre!("lifecycle initialize proxy: {e}")`.
///
/// Baseline: the resolution failure aborts the run **before** `start`, so no
/// terminal event fires — `initialize` is the only marker. Phase 4 replaces the
/// `eyre!` with a typed wrapper and Phase 5 renders it as a `StatusBlock`;
/// neither may start running `failure`/`finalize` here, and neither may change
/// the exit code.
#[test]
fn characterize_initialize_proxy_resolution_failure() {
    let outcome = run_route(
        r#"---
title: characterize initialize proxy
initialize:
  stack:
    - action: {append_line: ["events.log", "initialize"]}
    - action: {proxy: "no/such/target.md"}
start:
  stack:
    - action: {append_line: ["events.log", "start"]}
failure:
  stack:
    - action: {append_line: ["events.log", "failure"]}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#,
        0,
        &[],
    );

    assert_eq!(
        outcome.exit_code,
        Some(1),
        "initialize-proxy resolution failure must exit 1; stderr:\n{}",
        outcome.stderr_plain
    );
    assert_eq!(
        outcome.events,
        vec!["initialize"],
        "the resolution failure aborts before `start`, so `initialize` must be \
         the only marker — no terminal event may fire; stderr:\n{}",
        outcome.stderr_plain
    );
    assert_eq!(
        outcome.emission_count(),
        1,
        "the failure must be surfaced exactly once; stderr:\n{}",
        outcome.stderr_plain
    );
}

// --- Route 2: terminal/recovery proxy resolution failure ------------------

/// Proxy hand-off from a terminal event (`failure`).
///
/// Records the resolver divergence documented in `decisions.md` D-2: this route
/// uses `harness::resolve_harness_path`, which **succeeds** on a missing target.
/// The run therefore announces the hand-off, runs its full terminal lifecycle,
/// and only fails later in pre-flight (`prompt.rs:196`).
///
/// Baseline: the complete `start` → `failure` → `finalize` order fires. Phase 4
/// must not collapse this into the route-1 shape — that would be a routing
/// change, which D10 requires be split into a separate spec.
#[test]
fn characterize_terminal_proxy_resolution_failure() {
    let outcome = run_route(
        r#"---
title: characterize failure proxy
start:
  stack:
    - action: {append_line: ["events.log", "start"]}
failure:
  stack:
    - action: {append_line: ["events.log", "failure"]}
    - action: {proxy: "no/such/target.md"}
finalize:
  stack:
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#,
        3,
        &[],
    );

    assert_eq!(
        outcome.exit_code,
        Some(1),
        "terminal-proxy resolution failure must exit 1; stderr:\n{}",
        outcome.stderr_plain
    );
    assert_eq!(
        outcome.events,
        vec!["start", "failure", "finalize"],
        "the terminal proxy route resolves the missing target successfully and \
         fails later in pre-flight, so the full terminal lifecycle must fire; \
         stderr:\n{}",
        outcome.stderr_plain
    );
    assert_eq!(
        outcome.emission_count(),
        1,
        "the failure must be surfaced exactly once; stderr:\n{}",
        outcome.stderr_plain
    );
}

// --- Route 3: post-start setup failure (`from_action_failure`) ------------

/// The D7 `from_action_failure` route (`loop_control.rs:954`).
///
/// A malformed `exit_expressions` regex validates-and-aborts inside
/// `execute_harness_attempt` — after `start`, before the provider spawns.
///
/// Baseline: `start` fires, the provider never runs, and `failure` then
/// `finalize` fire with an `err` present. Phase 5 migrates this site to pass a
/// typed diagnostic, which flips `err.kind`/`err.variant` from the facet-less
/// labels to the faceted `category`/`code` values — an *intended* change, so the
/// marker values are not pinned here, only their **order and presence**.
#[test]
fn characterize_post_start_setup_failure() {
    let outcome = run_route(
        r#"---
title: characterize post-start setup failure
exit_expressions:
  - patterns:
      - "["
    kind: regex
start:
  stack:
    - action: {append_line: ["events.log", "start"]}
failure:
  stack:
    - action: {append_line: ["events.log", "failure"]}
finalize:
  stack:
    - when: "err"
      action: {append_line: ["events.log", "finalize-has-err"]}
    - action: {append_line: ["events.log", "finalize"]}
---
Body
"#,
        0,
        &[],
    );

    assert_eq!(
        outcome.exit_code,
        Some(1),
        "post-start setup failure must exit 1; stderr:\n{}",
        outcome.stderr_plain
    );
    assert_eq!(
        outcome.events,
        vec!["start", "failure", "finalize-has-err", "finalize"],
        "the pre-spawn guard failure must fire start → failure → finalize with \
         a truthy `when: err` guard; stderr:\n{}",
        outcome.stderr_plain
    );
    assert_eq!(
        outcome.emission_count(),
        1,
        "the failure must be surfaced exactly once; stderr:\n{}",
        outcome.stderr_plain
    );
}

// --- Route 4: harness pre-flight shell denial -----------------------------

/// Harness pre-flight failure carrying a typed `HarnessError`.
///
/// A blacklisted `::shell` directive is rejected at pre-flight. The typed
/// `HarnessError` is wrapped by the pre-flight boundary and surfaces today as a
/// `CompositionError` block whose body is the *flattened* harness text — the
/// `HarnessError`'s own richer block never renders, because `error_walker.rs`
/// cannot discover `HarnessError` (inventory §F).
///
/// Baseline: exit 1, no lifecycle markers (pre-flight precedes `start`), one
/// emission. Phase 5 makes the inner diagnostic discoverable, which changes the
/// block's *content* but must not change any of the three pinned properties.
#[cfg(unix)]
#[test]
fn characterize_preflight_shell_denial() {
    let outcome = run_route(
        r#"---
title: characterize preflight shell denial
start:
  stack:
    - action: {append_line: ["events.log", "start"]}
---

::shell rm -rf /
"#,
        0,
        &[],
    );

    assert_eq!(
        outcome.exit_code,
        Some(1),
        "pre-flight shell denial must exit 1; stderr:\n{}",
        outcome.stderr_plain
    );
    assert!(
        outcome.events.is_empty(),
        "pre-flight precedes `start`, so no lifecycle marker may fire; got {:?}; \
         stderr:\n{}",
        outcome.events,
        outcome.stderr_plain
    );
    assert_eq!(
        outcome.emission_count(),
        1,
        "the failure must be surfaced exactly once; stderr:\n{}",
        outcome.stderr_plain
    );
}

// --- Route 5: pre-document argument failure -------------------------------

/// An unparsable `--timeout` fails during argument preparation, before any
/// document or lifecycle work.
///
/// Phase 1 filed this as the "deliberately unstructured" control on the
/// assumption that an argument-shape failure carries no typed error. That was
/// **wrong**: it carries a typed `HarnessError::InvalidTimeout`, which Phase 5
/// makes discoverable, so it correctly renders a `StatusBlock` rather than the
/// generic `Error:` line. The three pinned properties below are unaffected —
/// emission count sums both surfaces precisely so a route may migrate between
/// them (§D5) — so this route keeps its baseline value; only the mistaken
/// rationale is corrected.
///
/// The genuine unstructured control lives in `effective_diagnostic_render.rs`,
/// which asserts the render contract this file deliberately does not pin.
#[test]
fn characterize_pre_document_argument_failure() {
    let outcome = run_route(
        r#"---
title: characterize unstructured fallback
---
Body
"#,
        0,
        &["--timeout", "not-a-duration"],
    );

    assert_eq!(
        outcome.exit_code,
        Some(1),
        "an invalid --timeout must exit 1; stderr:\n{}",
        outcome.stderr_plain
    );
    assert!(
        outcome.events.is_empty(),
        "argument preparation precedes every lifecycle event; got {:?}",
        outcome.events
    );
    assert_eq!(
        outcome.emission_count(),
        1,
        "the failure must be surfaced exactly once; stderr:\n{}",
        outcome.stderr_plain
    );
}

// --- Cross-route invariants ----------------------------------------------

/// Every characterized failure route exits 1 and emits exactly once.
///
/// Stated once, over all routes, so a later phase that regresses the invariant
/// on a route this file does not enumerate individually still trips a gate.
#[test]
fn characterize_all_failure_routes_exit_one_and_emit_once() {
    let routes: Vec<(&str, RouteOutcome)> = vec![
        (
            "initialize-proxy",
            run_route(
                "---\ntitle: t\ninitialize:\n  stack:\n    - action: {proxy: \"no/such/target.md\"}\n---\nBody\n",
                0,
                &[],
            ),
        ),
        (
            "terminal-proxy",
            run_route(
                "---\ntitle: t\nfailure:\n  stack:\n    - action: {proxy: \"no/such/target.md\"}\n---\nBody\n",
                3,
                &[],
            ),
        ),
        (
            "pre-document-argument-failure",
            run_route(
                "---\ntitle: t\n---\nBody\n",
                0,
                &["--timeout", "not-a-duration"],
            ),
        ),
    ];

    for (name, outcome) in &routes {
        assert_eq!(
            outcome.exit_code,
            Some(1),
            "route `{name}` must exit 1; stderr:\n{}",
            outcome.stderr_plain
        );
        assert_eq!(
            outcome.emission_count(),
            1,
            "route `{name}` must surface its failure exactly once; stderr:\n{}",
            outcome.stderr_plain
        );
    }
}
