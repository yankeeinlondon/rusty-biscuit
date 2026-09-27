//! Level 2 test for the `sniff repo structure` Lockfiles section rendered in a
//! real terminal (`2026-09-26-lockfile-corroboration`, "CLI and
//! compatibility").
//!
//! Level 1 pins the renderer's text with escapes stripped and runs the CLI with
//! `--plain`, so neither reaches what only an emulator decides: the width the
//! pane reports, where the headline wraps and how its continuation is
//! indented, the list glyphs and nesting a TTY run chooses, and the SGR the
//! status markup lowers to. This test runs the real `sniff` binary in a tmux
//! pane it owns, narrow enough that the layer headline must wrap, against a
//! disposable pnpm workspace whose lockfile records one stale importer and
//! lacks one current member, so the layer reports `mismatch` with both kinds
//! of difference.
//!
//! The pane is owned rather than the broker's shared one because its width is
//! the contract under test and the shared pane is not this test's to resize.
//!
//! Skip-clean: when tmux is unavailable the test prints a skip notice and
//! passes. Gated behind `test-fixtures`, which pulls in `biscuit-test-harness`;
//! the `test-l2` recipe enables it.
use std::fs;
use std::path::Path;
use std::time::Duration;

use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use test_toolkit::{Backend, Level, require_level};

use crate::common;

const RENDER_DEADLINE: Duration = Duration::from_secs(15);
const PANE_COLS: u32 = 56;
const PANE_ROWS: u32 = 40;
const DONE_MARKER: &str = "LOCKFILE-L2-DONE";

const HEADLINE_START: &str = "pnpm workspaces (.): mismatch";
const EXPLANATION: &str = "the lockfile's members differ from the manifest";
const DETAILS: [&str; 3] = [
    "lockfile: pnpm-lock.yaml",
    "missing from the lockfile: packages/delta",
    "only in the lockfile: packages/gamma",
];

/// A pnpm 9.0-format workspace: the manifest globs `alpha`, `beta`, and
/// `delta`; the lockfile's importers name `alpha`, `beta`, and a stale
/// `gamma`.
fn write_workspace(root: &Path) {
    for member in ["alpha", "beta", "delta"] {
        let dir = root.join("packages").join(member);
        fs::create_dir_all(&dir).expect("create member directory");
        fs::write(
            dir.join("package.json"),
            format!(r#"{{ "name": "@fixture/{member}", "version": "0.0.0" }}"#),
        )
        .expect("write member manifest");
    }
    fs::write(
        root.join("package.json"),
        r#"{ "name": "fixture-root", "version": "0.0.0", "private": true }"#,
    )
    .expect("write root manifest");
    fs::write(
        root.join("pnpm-workspace.yaml"),
        "packages:\n  - \"packages/*\"\n",
    )
    .expect("write pnpm-workspace.yaml");
    fs::write(
        root.join("pnpm-lock.yaml"),
        "lockfileVersion: '9.0'\n\n\
         settings:\n  autoInstallPeers: true\n  excludeLinksFromLockfile: false\n\n\
         importers:\n\n  .: {}\n\n  packages/alpha: {}\n\n  packages/beta: {}\n\n  packages/gamma: {}\n",
    )
    .expect("write pnpm-lock.yaml");
}

/// Everything from the `Lockfiles` title to the completion marker.
fn lockfile_section(frame: &CapturedFrame) -> Option<Vec<&str>> {
    let lines: Vec<&str> = frame.plain.lines().collect();
    let start = lines.iter().position(|line| line.trim() == "Lockfiles")?;
    let end = lines.iter().position(|line| line.trim() == DONE_MARKER)?;
    (start < end).then(|| lines[start..end].to_vec())
}

/// The text after the `- ` bullet, when `line` is a list item at exactly
/// `indent` columns.
fn item_body(line: &str, indent: usize) -> Option<&str> {
    line.strip_prefix(&" ".repeat(indent))?.strip_prefix("- ")
}

fn leading_spaces(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

#[test]
fn level2_lockfile_mismatch_renders_wrapped_and_styled_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = common::SniffCliFixture::named("sniff-l2-lockfile");
    let root = fixture.workspace_path().join("repo");
    write_workspace(&root);

    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("tmux spawn_shell failed");
    harness
        .resize(PANE_COLS, PANE_ROWS)
        .expect("tmux resize failed");
    assert_eq!(
        harness.pane_cols().expect("tmux pane width"),
        PANE_COLS,
        "the pane must report the width this test asserts wrapping against"
    );

    // The environment prefix is part of the command string so it binds to
    // `sniff`, not to `clear`. The marker is printed by `printf`, so the
    // echoed command line (which spells it with `%s`) cannot satisfy the wait.
    let binary = biscuit_test_harness::bin_exe!("sniff");
    let command = format!(
        "clear; HOME='{home}' XDG_CONFIG_HOME='{config}' XDG_CACHE_HOME='{cache}' \
         FORCE_COLOR=1 '{binary}' --base '{root}' repo structure; \
         printf '\\n%s\\n' {DONE_MARKER}",
        home = fixture.home().display(),
        config = fixture.config_dir().display(),
        cache = fixture.cache_dir().display(),
        binary = binary.display(),
        root = root.display(),
    );
    harness
        .send_command_with_env(&command, &[])
        .expect("send_command_with_env failed");

    let frame = common::capture_until(&mut harness, RENDER_DEADLINE, |frame| {
        lockfile_section(frame).is_some_and(|section| {
            section
                .iter()
                .any(|line| line.contains("only in the lockfile"))
        })
    });
    let pane = &frame.plain;
    let section = lockfile_section(&frame)
        .unwrap_or_else(|| panic!("no Lockfiles section before the marker:\n{pane}"));

    // No escape or markup survives as visible text.
    for leaked in [
        "\x1b", "[0m", "[1m", "[31m", "<red>", "</red>", "<b>", "<dim>",
    ] {
        assert!(
            !pane.contains(leaked),
            "`{leaked:?}` is visible in the pane:\n{pane}"
        );
    }

    // The layer headline is a top-level item whose explanation the renderer
    // wrapped under a hanging indent: a terminal soft wrap would leave the
    // continuation at column 0, and truncation would lose words.
    let headline = section
        .iter()
        .position(|line| item_body(line, 0).is_some_and(|body| body.starts_with(HEADLINE_START)))
        .unwrap_or_else(|| panic!("no top-level `{HEADLINE_START}` item:\n{pane}"));
    let first_detail = section
        .iter()
        .position(|line| item_body(line, 4).is_some())
        .unwrap_or_else(|| panic!("no nested detail item:\n{pane}"));
    assert!(
        first_detail > headline + 1,
        "the headline must wrap at {PANE_COLS} columns:\n{pane}"
    );
    let continuation = &section[headline + 1..first_detail];
    let hanging = leading_spaces(section[headline]) + 2;
    for line in continuation {
        assert_eq!(
            leading_spaces(line),
            hanging,
            "a wrapped headline line must hang under the item text: {line:?}\n{pane}"
        );
    }
    let headline_text = std::iter::once(item_body(section[headline], 0).expect("headline body"))
        .chain(continuation.iter().map(|line| line.trim()))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        headline_text.ends_with(EXPLANATION),
        "the wrapped headline must carry the whole explanation, got {headline_text:?}:\n{pane}"
    );
    assert!(
        !headline_text.contains('…'),
        "the headline was truncated:\n{pane}"
    );

    // Selected path, missing, and extra members are items nested one level
    // under the headline, in that order, each on its own line.
    let details: Vec<&str> = section[first_detail..]
        .iter()
        .filter_map(|line| item_body(line, 4))
        .collect();
    assert_eq!(details, DETAILS, "nested detail items:\n{pane}");

    for line in pane.lines() {
        assert!(
            line.chars().count() <= PANE_COLS as usize,
            "a row is wider than the pane: {line:?}"
        );
    }

    // The status and the member-difference labels lower to red SGR, and the
    // layer label to bold, on the rows that carry them.
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let raw_row = |needle: &str| {
        raw_lines
            .iter()
            .find(|line| biscuit_test_harness::strip_ansi(line).contains(needle))
            .copied()
            .unwrap_or_else(|| panic!("no raw row carries `{needle}`:\n{}", frame.raw))
    };
    let is_red = |row: &str| row.contains("[31m") || row.contains(";31m");
    let headline_raw = raw_row(HEADLINE_START);
    assert!(
        is_red(headline_raw),
        "`mismatch` must render red: {headline_raw:?}"
    );
    assert!(
        headline_raw.contains("[1m")
            || headline_raw.contains(";1m")
            || headline_raw.contains("[1;"),
        "the layer label must render bold: {headline_raw:?}"
    );
    for label in ["missing from the lockfile", "only in the lockfile"] {
        let row = raw_row(label);
        assert!(is_red(row), "`{label}` must render red: {row:?}");
    }
    let path_row = raw_row("lockfile: pnpm-lock.yaml");
    assert!(
        !is_red(path_row),
        "the selected path is not an error: {path_row:?}"
    );
}
