//! Integration tests for the resolved-interactive ↔ timeout conflict.
//!
//! Part of the `2026-06-14-interactive` feature (review-1 High finding).
//! Frontmatter `interactive: true` selects an interactive session without a
//! CLI flag, so the timeout conflict must be evaluated against the *resolved*
//! session mode and the *resolved* timeout plan (CLI + frontmatter + env),
//! not just the raw `--interactive` flag. These tests drive the compiled
//! `claudine` binary end-to-end with a provider stub that records launches:
//!
//! - `interactive: true` + `--step-timeout` → conflict, no provider launch.
//! - `interactive: true` + effective frontmatter `timeout` → conflict, no
//!   provider launch.
//! - `--no-interactive` overriding `interactive: true` → timeouts are valid
//!   again and the provider launches.

#![cfg(unix)]

use std::fs;
use tempfile::tempdir;

mod common;
use common::{augmented_path, strip_ansi, write_executable};

/// Stage a `goose` stub that records every launch by appending to
/// `count_path`, so a test can prove the provider was (or was not) launched.
fn stage_goose_counter(bin_dir: &std::path::Path, count_path: &std::path::Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );
}

#[test]
fn compose_frontmatter_interactive_rejects_step_timeout() {
    // `interactive: true` frontmatter + `--step-timeout` must conflict even
    // though `--interactive` was never passed. This is the exact gap the
    // review found: the early CLI guards only see `shared.interactive`, so the
    // resolved-mode executor check is the one that has to fire.
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");
    stage_goose_counter(&bin_dir, &count_path);

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        "---\ninteractive: true\n---\nOpen a dialog.\n",
    )
    .unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&bin_dir))
        .current_dir(workspace.path())
        .args([
            "compose",
            "--goose",
            "--step-timeout",
            "30s",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("interactive mode (from frontmatter)")
            && plain.contains("--step-timeout"),
        "conflict must name the frontmatter source and the --step-timeout flag; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should launch when the timeout conflict fires"
    );
}

#[test]
fn compose_frontmatter_interactive_rejects_frontmatter_timeout() {
    // The wall-clock `timeout` comes from composed frontmatter (a harness
    // key), not the CLI. With `interactive: true` also authored, the resolved
    // mode is interactive and the resolved timeout plan includes the
    // frontmatter timeout — so the conflict must fire.
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");
    stage_goose_counter(&bin_dir, &count_path);

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        "---\ninteractive: true\ntimeout: 5m\n---\nOpen a dialog.\n",
    )
    .unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&bin_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("interactive mode (from frontmatter)") && plain.contains("--timeout"),
        "conflict must name the frontmatter source and the wall-clock timeout; stderr:\n{plain}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should launch when the frontmatter timeout conflicts"
    );
}

#[test]
fn compose_frontmatter_interactive_header_shows_interactive_badge() {
    // The eager execution header renders before prepare/execution. It must
    // describe the *resolved* session mode, not the raw `--interactive` flag:
    // a document with `interactive: true` and no `-i` runs interactively, so
    // the header must carry the `Interactive` badge. Pre-fix the header was
    // built from `shared.interactive` (false here), so no badge appeared and
    // the first status line contradicted the actual run.
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");
    stage_goose_counter(&bin_dir, &count_path);

    let md_file = workspace.path().join("plan.md");
    fs::write(&md_file, "---\ninteractive: true\n---\nOpen a dialog.\n").unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&bin_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("Interactive"),
        "the eager header must show the Interactive badge for `interactive: true` \
         frontmatter even without `-i`; stderr:\n{plain}"
    );
}

#[test]
fn compose_default_non_interactive_header_omits_interactive_badge() {
    // Control for the badge test above: a document with no `interactive`
    // frontmatter and no `-i` flag resolves to the default non-interactive
    // mode, which renders NO interactivity badge. This pins the contrast so a
    // future regression that always emits the badge is caught.
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");
    stage_goose_counter(&bin_dir, &count_path);

    let md_file = workspace.path().join("plan.md");
    fs::write(&md_file, "---\n---\nOpen a dialog.\n").unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&bin_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        !plain.contains("Interactive"),
        "the default non-interactive header must NOT show an Interactive badge; \
         stderr:\n{plain}"
    );
}

#[test]
fn compose_no_interactive_overrides_frontmatter_and_allows_timeout() {
    // `--no-interactive` wins over `interactive: true`, so the resolved mode
    // is non-interactive and timeouts are valid again. The provider must
    // launch normally with `--step-timeout` applied.
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");
    stage_goose_counter(&bin_dir, &count_path);

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        "---\ninteractive: true\n---\nOpen a dialog.\n",
    )
    .unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&bin_dir))
        .current_dir(workspace.path())
        .args([
            "compose",
            "--no-interactive",
            "--goose",
            "--step-timeout",
            "30s",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        count_path.exists(),
        "provider should launch when --no-interactive overrides interactive: true \
         frontmatter, making the timeout valid again"
    );
}
