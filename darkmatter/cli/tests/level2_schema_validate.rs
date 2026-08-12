//! Level-2 tests for `md schema validate` terminal rendering.
//!
//! These tests drive the actual `md` binary inside a shared WezTerm pane and
//! capture the terminal's rendered cells. The Level-1 tests prove the CLI
//! emits the per-problem description sub-line; this file proves the dimmed
//! sub-line reaches a real terminal with its SGR styling intact — the
//! user-visible styling plain-text Level-1 snapshots cannot see.

mod common;

use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::CapturedFrame;
use common::level2::{SHARED_HARNESS, md_shim, run_with_sentinel_env, wezterm_decision};
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use test_toolkit::LevelDecision;

// Document with an inline `$schema` whose required `title` property declares
// a description via the `->` arrow syntax and is OMITTED from the frontmatter
// so `md schema validate` reports a missing-required failure. `md schema
// validate` does not run the compose pipeline, so no scalar coercion can mask
// the failure; omitting the property is coercion-proof by construction. The
// declared description surfaces as a dimmed sub-line beneath the problem
// bullet (Decision #7), the path under test here.
const MISSING_DESCRIBED_REQUIRED: &str =
    "---\n$schema:\n  title: 'string(required) -> The headline shown in listing pages'\n---\nBody\n";

/// Writes the fixture, runs `md schema validate` in the shared WezTerm pane,
/// and returns the captured frame plus the canonical fixture path. Returns
/// `None` when WezTerm is unavailable so the caller can skip cleanly. Uses
/// [`md_shim`] so the suite verifies the code under review rather than a stale
/// `md` installed on the host.
fn run_md_schema_validate(file_name: &str, file_body: &str) -> Option<(CapturedFrame, PathBuf)> {
    match wezterm_decision() {
        LevelDecision::Run => {}
        LevelDecision::Skip(msg) => {
            eprintln!("{msg}");
            return None;
        }
        LevelDecision::Panic(msg) => panic!("{msg}"),
    }

    let dir = tempdir().unwrap();
    let file_path = dir.path().join(file_name);
    fs::write(&file_path, file_body).unwrap();
    let canonical = file_path.canonicalize().expect("canonicalize failed");

    let mut guard = SHARED_HARNESS
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().unwrap();

    // Reset the visible region so a previous test's output does not bleed
    // into this capture.
    run_with_sentinel_env(harness, "clear", &[]);

    let cmd = format!("{} schema validate {}", md_shim(), file_path.display());
    let frame = run_with_sentinel_env(harness, &cmd, &[]);
    // Keep tempdir alive past capture by returning canonical path to caller.
    drop(dir);
    Some((frame, canonical))
}

/// Level-2 capture for the per-problem description sub-line in `md schema
/// validate` pretty output: drives the real binary against a fixture whose
/// failing property declares a description via the `->` arrow syntax, then
/// verifies the description text and its dim SGR survive the real terminal
/// path. Unlike the `md compose` schema-validation block (which renders the
/// description as `<i><dim>`), this path renders only `<dim>`.
#[test]
#[serial(level2_terminal)]
fn level2_schema_validate_pretty_renders_dimmed_description_sub_line() {
    let Some((frame, _)) =
        run_md_schema_validate("post-schema.md", MISSING_DESCRIBED_REQUIRED)
    else {
        return;
    };

    // The failing-property bullet must be visible in plain so the description
    // assertions below read the validation report, not stray output.
    assert!(
        frame.plain.contains("title"),
        "expected failing property `title` to appear on the problem bullet. plain:\n{}",
        frame.plain
    );

    // The per-problem description text (declared via `-> {description}`) must
    // survive to plain — the path NOT covered by plain-text Level-1 snapshots
    // that cannot see styling.
    assert!(
        frame.plain.contains("The headline shown in listing pages"),
        "expected per-problem description text in validation report. plain:\n{}",
        frame.plain
    );

    // The per-problem description renders as `<dim>…</dim>` (no italic, unlike
    // the `md compose` schema-validation block). Assert dim SGR survives the
    // real terminal so a regression that drops the styling (while keeping the
    // text) is caught at Level 2.
    let has_dim = frame.raw.contains("\x1b[2m") || frame.raw.contains("\x1b[0;2m");
    assert!(
        has_dim,
        "expected dim SGR for the per-problem description sub-line. raw:\n{}",
        frame.raw
    );
}
