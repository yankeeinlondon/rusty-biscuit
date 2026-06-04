//! Level 2 real-terminal capture tests for the `--dry-run` metadata table.
//!
//! Closes the spec acceptance criterion that the metadata table renders with
//! visible styling and a hyperlink:
//!
//! > The metadata table includes Document (with OSC8 link), Description,
//! > Agent, Model, YOLO, and Area … rendered in blue / italic+dim / green+red.
//!
//! The L1 unit tests in `dry_run.rs` and the CLI integration tests in
//! `wrap_commands.rs` assert the table's *semantics* after stripping escape
//! codes, so they cannot catch broken SGR/OSC8 emission. These tests drive the
//! real `claudine compose --dry-run` binary inside a real terminal emulator and
//! assert against the bytes the terminal actually displayed (`frame.raw`):
//!
//! - `tmux` (portable, headless): the color/dim/italic SGR contract — blue
//!   Document cell, italic+dim Description, red `false` YOLO cell.
//! - `WezTerm` (OSC8 fidelity): the Document cell is a real OSC8 hyperlink.
//!
//! Both run `compose --dry-run` with `FORCE_COLOR=1`, which routes claudine's
//! output through an optimistic terminal (TrueColor + OSC8 link support) so the
//! styling is emitted regardless of the host terminal's own capability
//! detection — the proof is then the *emulator's* capture path, not claudine's
//! byte stream.
//!
//! Skip-clean: each test checks `Harness::available()` first and returns early
//! when the backend is absent (no `#[ignore]`), so CI without the tooling stays
//! green. `BISCUIT_TEST_LEVEL_REQUIRED=2` flips a missing harness into a hard
//! failure.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test-l2
//! ```

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::time::Duration;
use test_toolkit::{Level, require_level};

mod common;
use common::{TestWorkspace, augmented_path, write_executable};

/// The composition fixture: a `name`/`description` frontmatter (so the Document
/// cell uses the name and the Description row is present) and a body with no
/// `::shell` commands (so no approval gate fires).
const FIXTURE_DOC: &str = "\
---
name: Styled Doc
description: a styled description
---
Just a body.
";

/// What the captured frame the driver returns carries alongside the pane
/// capture, kept together so the temp workspace outlives the capture.
struct DryRunCapture {
    frame: CapturedFrame,
    /// The absolute source-document path, used to assert the OSC8 `file://`
    /// link target.
    _workspace: TestWorkspace,
}

/// Stage a workspace and run `claudine compose --goose --dry-run doc.md` inside
/// `harness`, returning the captured pane frame.
///
/// `--goose` resolves the Agent cell to a concrete provider (stubbed on `PATH`
/// so resolution succeeds); under `--dry-run` the provider never launches. The
/// metadata table and frontmatter land on the pane via stderr; the composed
/// body via stdout.
fn run_dry_run_compose<H: TerminalHarness>(harness: &mut H) -> DryRunCapture {
    let workspace = TestWorkspace::named("claudine-dryrun-l2");
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Minimal config so the first-run setup wizard does not intercept.
    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    // Provider stub: present for resolution, never launched under --dry-run.
    write_executable(&bin_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let doc = workspace.path().join("doc.md");
    fs::write(&doc, FIXTURE_DOC).unwrap();

    let claudine = cargo_bin!("claudine").display().to_string();
    let path = augmented_path(&bin_dir);
    let path = path.to_string_lossy().into_owned();
    let home = workspace.path().to_string_lossy().into_owned();

    // Run from the (non-repo) workspace so no monorepo `Area` row appears and
    // the assertions stay focused on the fixed rows.
    harness
        .send_text(format!("cd {}\n", workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let cmd = format!("{claudine} compose --goose --dry-run {}", doc.display());
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", home.as_str()),
                ("PATH", path.as_str()),
                ("FORCE_COLOR", "1"),
                // Deterministic table width well inside any pane.
                ("COLUMNS", "80"),
            ],
        )
        .expect("send compose --dry-run");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    std::thread::sleep(Duration::from_millis(250));

    let frame = harness.capture().expect("capture failed");
    DryRunCapture {
        frame,
        _workspace: workspace,
    }
}

/// Return the raw (escape-bearing) capture line whose plain (escape-stripped)
/// form contains `label`, pairing raw and plain lines by index.
///
/// The metadata-table row labels (`Document`, `Description`, …) are
/// capitalized and unique to the table, so they never collide with the
/// lowercase YAML frontmatter keys also printed to stderr or with the echoed
/// command line.
fn row_raw<'a>(frame: &'a CapturedFrame, label: &str) -> Option<&'a str> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let plain_lines: Vec<&str> = frame.plain.lines().collect();
    plain_lines
        .iter()
        .position(|l| l.contains(label))
        .and_then(|i| raw_lines.get(i).copied())
}

/// A single styling attribute decoded from a CSI `m` sequence in a terminal
/// capture line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Attr {
    Dim,
    Italic,
    /// Basic or bright foreground color (`30`–`37`, `90`–`97`).
    Fg(i64),
}

/// Decode every [`Attr`] present in a single capture line.
///
/// `38`/`48` (set fg/bg via truecolor or indexed color) consume their trailing
/// parameters so a color component value (e.g. `…;3;…`) is never misread as the
/// italic attribute. Both `;` and ITU `:` separators are accepted because
/// terminal capture paths re-emit attributes in either form.
fn decode_attrs(line: &str) -> Vec<Attr> {
    let mut attrs = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'[') {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len()
                && (bytes[j].is_ascii_digit() || bytes[j] == b';' || bytes[j] == b':')
            {
                j += 1;
            }
            if bytes.get(j) == Some(&b'm') {
                let params: Vec<i64> = line[start..j]
                    .split([';', ':'])
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| s.parse().ok())
                    .collect();
                decode_params(&params, &mut attrs);
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    attrs
}

/// Decode one SGR parameter list into [`Attr`]s, skipping the operands of
/// `38`/`48` color selectors.
fn decode_params(params: &[i64], out: &mut Vec<Attr>) {
    let mut i = 0;
    while i < params.len() {
        match params[i] {
            2 => out.push(Attr::Dim),
            3 => out.push(Attr::Italic),
            38 | 48 => match params.get(i + 1) {
                Some(&2) => {
                    i += 5;
                    continue;
                }
                Some(&5) => {
                    i += 3;
                    continue;
                }
                _ => {}
            },
            n @ (30..=37 | 90..=97) => out.push(Attr::Fg(n)),
            _ => {}
        }
        i += 1;
    }
}

/// Whether `line` selects basic or bright blue foreground (`34`/`94`).
fn has_blue(line: &str) -> bool {
    let attrs = decode_attrs(line);
    attrs.contains(&Attr::Fg(34)) || attrs.contains(&Attr::Fg(94))
}

/// Whether `line` selects basic or bright red foreground (`31`/`91`).
fn has_red(line: &str) -> bool {
    let attrs = decode_attrs(line);
    attrs.contains(&Attr::Fg(31)) || attrs.contains(&Attr::Fg(91))
}

/// Assert the visible table cells and the SGR color/dim/italic contract shared
/// by every backend.
fn assert_styled_rows(frame: &CapturedFrame) {
    // Visible cells: row labels and their values must all reach the pane.
    for needle in [
        "Document",
        "Styled Doc",
        "Description",
        "a styled description",
        "Agent",
        "goose",
        "Model",
        "default",
        "YOLO",
        "false",
    ] {
        assert!(
            frame.plain.contains(needle),
            "expected '{needle}' in the captured pane.\nplain:\n{}",
            frame.plain,
        );
    }

    let document = row_raw(frame, "Document")
        .unwrap_or_else(|| panic!("Document row not found.\nraw:\n{}", frame.raw));
    assert!(
        has_blue(document),
        "expected the Document cell to render blue (SGR 34/94).\nrow: {document:?}",
    );

    let description = row_raw(frame, "Description")
        .unwrap_or_else(|| panic!("Description row not found.\nraw:\n{}", frame.raw));
    let attrs = decode_attrs(description);
    assert!(
        attrs.contains(&Attr::Dim) && attrs.contains(&Attr::Italic),
        "expected the Description cell to render italic + dim (SGR 3 and 2).\n\
         decoded: {attrs:?}\nrow: {description:?}",
    );

    let yolo = row_raw(frame, "YOLO")
        .unwrap_or_else(|| panic!("YOLO row not found.\nraw:\n{}", frame.raw));
    assert!(
        has_red(yolo),
        "expected the YOLO `false` cell to render red (SGR 31/91).\nrow: {yolo:?}",
    );
}

/// tmux is the portable, headless backend and faithfully re-emits SGR, so it
/// covers the color/dim/italic contract on every host that has `tmux`.
#[test]
#[serial(level2_terminal)]
fn level2_dry_run_metadata_table_renders_styled_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let capture = run_dry_run_compose(&mut harness);
    assert_styled_rows(&capture.frame);
}

/// WezTerm preserves OSC8 hyperlinks through its `get-text` capture, so it is
/// the backend that proves the Document cell is a real `file://` link (and,
/// for free, the same SGR contract).
#[test]
#[serial(level2_terminal)]
fn level2_dry_run_document_cell_renders_osc8_link_in_wezterm() {
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );

    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    let capture = run_dry_run_compose(&mut harness);

    // The Document cell is built as `<blue><a href="file://…">…</a></blue>`; a
    // real terminal must surface the OSC8 hyperlink-open sequence pointing at
    // the source document. We assert the stable `file://` prefix rather than the
    // full path (macOS canonicalizes `/tmp` → `/private/tmp`).
    assert!(
        capture.frame.raw.contains("\x1b]8;;file://"),
        "expected an OSC8 `file://` hyperlink for the Document cell.\nraw:\n{}",
        capture.frame.raw,
    );
    // The visible label must still be present (the link wraps, not replaces).
    assert!(
        capture.frame.plain.contains("Styled Doc"),
        "expected the Document label 'Styled Doc' to remain visible.\nplain:\n{}",
        capture.frame.plain,
    );

    // OSC8 fidelity aside, WezTerm also re-emits the SGR contract.
    assert_styled_rows(&capture.frame);
}
