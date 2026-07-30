//! Level 2 real-terminal capture tests for the `--dry-run` metadata table.
//!
//! Closes the spec acceptance criterion that the metadata table renders with
//! visible styling and a hyperlink:
//!
//! > The metadata table includes Document (with OSC8 link), Description,
//! > Agent, Model, YOLO, and Area … rendered in blue / italic+dim / green+red.
//!
//! The L1 unit tests in `dry_run.rs` and the CLI integration tests in the
//! `wrap_compose_*.rs` binaries assert the table's *semantics* after stripping
//! escape codes, so they cannot catch broken SGR/OSC8 emission. These tests drive the
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
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::{TestWorkspace, clear_no_color, write_executable};

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

/// Fixture with an invalid `agent` frontmatter value.
const FIXTURE_INVALID_AGENT: &str = "\
---
name: Invalid Agent Doc
agent: totally-invalid-agent
---
Just a body.
";

/// Fixture with a valid but not-installed `agent` frontmatter value.
/// The test harness only stubs `goose` on PATH, so `qwen` is not installed.
const FIXTURE_NOT_INSTALLED: &str = "\
---
name: Not Installed Doc
agent: qwen
---
Just a body.
";

/// Fixture with no `agent` frontmatter and no explicit provider flag.
const FIXTURE_NO_AGENT: &str = "\
---
name: No Agent Doc
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
    // Every fixture in this file asserts a *colored* surface (256-color YAML
    // highlighting, bold/italic headings, table cell colors). An ambient
    // `NO_COLOR` out-votes both `FORCE_COLOR=1` and the plain fixture's real
    // capability detection — see `common::clear_no_color`.
    clear_no_color(harness);

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

    let claudine = cargo_bin("claudine").display().to_string();
    // Hermetic PATH: exactly the stub bin dir, so installed/not-installed is
    // a fact of the fixture, never of the host (a host-installed provider
    // must not flip the resolution state under test).
    let path = bin_dir.to_string_lossy().into_owned();
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

/// Stage a workspace and run `claudine compose --dry-run doc.md` with a custom
/// document content and **no** explicit `--<provider>` flag.
///
/// Only `goose` is stubbed on PATH; `qwen` is absent so it resolves as
/// not-installed. This lets frontmatter `agent` values drive the resolution
/// state rendered in the metadata table.
fn run_dry_run_compose_with_doc<H: TerminalHarness>(
    harness: &mut H,
    doc_content: &str,
    workspace_name: &str,
) -> DryRunCapture {
    // Every fixture in this file asserts a *colored* surface (256-color YAML
    // highlighting, bold/italic headings, table cell colors). An ambient
    // `NO_COLOR` out-votes both `FORCE_COLOR=1` and the plain fixture's real
    // capability detection — see `common::clear_no_color`.
    clear_no_color(harness);

    let workspace = TestWorkspace::named(workspace_name);
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    // Only goose is present on PATH.
    write_executable(&bin_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let doc = workspace.path().join("doc.md");
    fs::write(&doc, doc_content).unwrap();

    let claudine = cargo_bin("claudine").display().to_string();
    // Hermetic PATH: exactly the stub bin dir, so installed/not-installed is
    // a fact of the fixture, never of the host (a host-installed provider
    // must not flip the resolution state under test).
    let path = bin_dir.to_string_lossy().into_owned();
    let home = workspace.path().to_string_lossy().into_owned();

    harness
        .send_text(format!("cd {}\n", workspace.path().display()).as_bytes())
        .expect("cd into workspace");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    // No --goose (or any --provider) flag — frontmatter drives resolution.
    let cmd = format!("{claudine} compose --dry-run {}", doc.display());
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", home.as_str()),
                ("PATH", path.as_str()),
                ("FORCE_COLOR", "1"),
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
/// form contains `label` inside the metadata table.
///
/// Matches `│ {label}` so YAML frontmatter values (e.g. `name: Invalid Agent
/// Doc`) or wrapped command lines do not collide with table row labels.
fn row_raw<'a>(frame: &'a CapturedFrame, label: &str) -> Option<&'a str> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let plain_lines: Vec<&str> = frame.plain.lines().collect();
    plain_lines
        .iter()
        .position(|l| l.contains(&format!("│ {label}")))
        .and_then(|i| raw_lines.get(i).copied())
}

/// A single styling attribute decoded from a CSI `m` sequence in a terminal
/// capture line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Attr {
    Bold,
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
            1 => out.push(Attr::Bold),
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

/// Whether `line` selects basic or bright yellow foreground (`33`/`93`).
fn has_yellow(line: &str) -> bool {
    let attrs = decode_attrs(line);
    attrs.contains(&Attr::Fg(33)) || attrs.contains(&Attr::Fg(93))
}

/// Whether `line` carries the dim attribute (`2`).
fn has_dim(line: &str) -> bool {
    decode_attrs(line).contains(&Attr::Dim)
}

/// Whether `line` carries the bold attribute (`1`).
fn has_bold(line: &str) -> bool {
    decode_attrs(line).contains(&Attr::Bold)
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
        "Goose",
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

/// Assert the `Frontmatter (resolved):` heading is present, bold on
/// "Frontmatter", italic on "resolved", and followed by a blank line (bottom
/// margin of 1).
fn assert_frontmatter_heading_and_spacing(frame: &CapturedFrame) {
    let plain_lines: Vec<&str> = frame.plain.lines().collect();
    let heading_idx = plain_lines
        .iter()
        .position(|l| l.contains("Frontmatter") && l.contains("resolved"))
        .unwrap_or_else(|| {
            panic!(
                "expected 'Frontmatter (resolved):' heading.\nplain:\n{}",
                frame.plain
            )
        });

    // The line after the heading must be blank (bottom margin of 1).
    assert!(
        plain_lines
            .get(heading_idx + 1)
            .is_some_and(|l| l.trim().is_empty()),
        "expected blank line after 'Frontmatter (resolved):' heading (bottom margin 1).\nplain:\n{}",
        frame.plain,
    );

    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let raw_heading = raw_lines.get(heading_idx).unwrap_or_else(|| {
        panic!(
            "raw heading line missing at index {heading_idx}.\nraw:\n{}",
            frame.raw
        )
    });
    assert!(
        has_bold(raw_heading),
        "expected 'Frontmatter' to render bold (SGR 1).\nrow: {raw_heading:?}",
    );
    assert!(
        decode_attrs(raw_heading).contains(&Attr::Italic),
        "expected 'resolved' to render italic (SGR 3).\nrow: {raw_heading:?}",
    );
}

/// Assert that the full-width horizontal rule renders as visible text in a
/// real terminal.
///
/// The dry-run hr is a `RuleStyle::Dashes` / `RuleAlignment::Full` rule. Under
/// a non-image terminal (the plain, no-`FORCE_COLOR` capture path) it renders
/// as a run of `╌` glyphs (ASCII `-` in a restricted terminal), which tmux
/// faithfully captures — unlike the Kitty-graphics image tier the optimistic
/// `FORCE_COLOR` terminal would select.
fn assert_horizontal_rule(frame: &CapturedFrame) {
    let has_rule = frame.plain.lines().any(|line| {
        let trimmed = line.trim();
        let dash_count = trimmed.chars().filter(|&c| c == '╌' || c == '-').count();
        dash_count >= 10
            && trimmed
                .chars()
                .all(|c| c == '╌' || c == '-' || c.is_whitespace())
    });
    assert!(
        has_rule,
        "expected a full-width horizontal rule (dashes) in the captured pane.\nplain:\n{}",
        frame.plain,
    );
}

/// Assert that the YAML frontmatter block carries syntax-highlighting SGR
/// codes (proving inverse-theme highlighting was applied).
fn assert_inverse_theme_yaml(frame: &CapturedFrame) {
    let plain_lines: Vec<&str> = frame.plain.lines().collect();
    let heading_idx = plain_lines
        .iter()
        .position(|l| l.contains("Frontmatter") && l.contains("resolved"))
        .unwrap_or_else(|| {
            panic!(
                "expected 'Frontmatter (resolved):' heading.\nplain:\n{}",
                frame.plain
            )
        });

    // The YAML block sits between the heading+blank-line and the metadata table.
    // Look for the first table border (`┌`) after the heading.
    let table_start = plain_lines
        .iter()
        .skip(heading_idx)
        .position(|l| l.contains('┌'))
        .map(|i| heading_idx + i)
        .unwrap_or_else(|| {
            panic!(
                "expected metadata table start after heading.\nplain:\n{}",
                frame.plain
            )
        });

    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let yaml_lines = &raw_lines[heading_idx + 2..table_start];

    // At least one YAML line must carry SGR escape codes (syntax highlighting).
    let highlighted_count = yaml_lines
        .iter()
        .filter(|l| l.contains('\x1b'))
        .count();
    assert!(
        highlighted_count >= 1,
        "expected at least one YAML line with syntax-highlighting SGR.\n\
         yaml raw lines:\n{}",
        yaml_lines.join("\n"),
    );
}

/// Assert that a multi-line `Agent` cell preserves two-column table alignment:
/// every `│` column separator inside the metadata table appears at the same
/// positions across all table lines.
fn assert_agent_cell_alignment(frame: &CapturedFrame, agent_label: &str) {
    let plain_lines: Vec<&str> = frame.plain.lines().collect();

    // Find the table start by the top-left border character.
    let table_start = plain_lines
        .iter()
        .position(|l| l.contains('┌'))
        .unwrap_or_else(|| {
            panic!(
                "expected table top border (`┌`) in pane.\nplain:\n{}",
                frame.plain
            )
        });

    // Find the table end by the bottom-left border character.
    let table_end = plain_lines
        .iter()
        .skip(table_start)
        .position(|l| l.contains('└'))
        .map(|i| table_start + i)
        .unwrap_or_else(|| {
            panic!(
                "expected table bottom border (`└`) in pane.\nplain:\n{}",
                frame.plain
            )
        });

    // Confirm the Agent label is inside the table so we're testing the right
    // region.
    let agent_in_table = plain_lines[table_start..=table_end]
        .iter()
        .any(|l| l.contains(agent_label));
    assert!(
        agent_in_table,
        "expected '{agent_label}' inside the metadata table.\nplain:\n{}",
        frame.plain,
    );

    // Gather the column positions of every `│` on each table line.
    let mut expected_positions: Option<Vec<usize>> = None;
    for line in &plain_lines[table_start..=table_end] {
        let positions: Vec<usize> = line
            .char_indices()
            .filter(|(_, c)| *c == '│')
            .map(|(i, _)| i)
            .collect();
        // Borders have at least two `│` separators; skip stray characters.
        if positions.len() >= 2 {
            if let Some(ref expected) = expected_positions {
                assert_eq!(
                    positions, *expected,
                    "table column separators misaligned.\n\
                     line: {line:?}\nexpected positions: {expected:?}\nplain:\n{}",
                    frame.plain,
                );
            } else {
                expected_positions = Some(positions);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Existing tests (selected-provider fixture)
// ---------------------------------------------------------------------------

/// tmux is the portable, headless backend and faithfully re-emits SGR, so it
/// covers the color/dim/italic contract on every host that has `tmux`.
#[test]
#[serial(level2_terminal)]
fn level2_dry_run_metadata_table_renders_styled_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

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
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

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

/// Stage a workspace and run `claudine compose --goose --dry-run doc.md` with
/// image support disabled (empty `TERM_PROGRAM`) and **without** `FORCE_COLOR`.
///
/// `biscuit-terminal` keys image-protocol detection off `TERM_PROGRAM` / `TERM`
/// ([`biscuit_terminal::discovery::detection::image_support`]). Inside the
/// harness the child otherwise inherits an image-capable `TERM_PROGRAM` (e.g.
/// `WezTerm`), so the horizontal rule renders as a Kitty graphics image that
/// tmux strips from `capture-pane`. Clearing `TERM_PROGRAM` drops image support
/// to `None`, so the rule renders as visible dash glyphs that tmux captures;
/// 256-color SGR (YAML highlighting, bold/italic heading) is keyed off `TERM` /
/// `COLORTERM` and is unaffected.
fn run_dry_run_compose_plain<H: TerminalHarness>(harness: &mut H) -> DryRunCapture {
    // Every fixture in this file asserts a *colored* surface (256-color YAML
    // highlighting, bold/italic headings, table cell colors). An ambient
    // `NO_COLOR` out-votes both `FORCE_COLOR=1` and the plain fixture's real
    // capability detection — see `common::clear_no_color`.
    clear_no_color(harness);

    let workspace = TestWorkspace::named("claudine-dryrun-l2-plain");
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    write_executable(&bin_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let doc = workspace.path().join("doc.md");
    fs::write(&doc, FIXTURE_DOC).unwrap();

    let claudine = cargo_bin("claudine").display().to_string();
    // Hermetic PATH: exactly the stub bin dir, so installed/not-installed is
    // a fact of the fixture, never of the host (a host-installed provider
    // must not flip the resolution state under test).
    let path = bin_dir.to_string_lossy().into_owned();
    let home = workspace.path().to_string_lossy().into_owned();

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
                ("COLUMNS", "80"),
                // Neutralize the harness's session-level `FORCE_COLOR=1`, which
                // otherwise routes claudine through an optimistic terminal that
                // claims Kitty image support (the hr would then emit as a Kitty
                // image that tmux strips). With it off, real detection applies:
                // the tmux pane still reports 256-color (YAML highlighting kept)
                // but no image protocol, so the hr renders as text dashes.
                ("FORCE_COLOR", "0"),
                ("CLICOLOR_FORCE", "0"),
                // Belt-and-braces: also clear the image-capable TERM_PROGRAM.
                ("TERM_PROGRAM", ""),
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

// ---------------------------------------------------------------------------
// Phase 6 — structural formatting (hr, heading, YAML) in a real terminal
// ---------------------------------------------------------------------------

/// Structural dry-run formatting: the full-width horizontal rule, the
/// `Frontmatter (resolved):` heading with bottom margin 1, and inverse-theme
/// YAML syntax highlighting.
///
/// The capture omits `FORCE_COLOR` (see [`run_dry_run_compose_plain`]) so the
/// terminal does not advertise Kitty graphics; the hr therefore renders as
/// visible dash glyphs that tmux captures, rather than as a Kitty image (which
/// tmux strips from `capture-pane`).
#[test]
#[serial(level2_terminal)]
fn level2_dry_run_yaml_heading_structure_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let capture = run_dry_run_compose_plain(&mut harness);
    assert_horizontal_rule(&capture.frame);
    assert_frontmatter_heading_and_spacing(&capture.frame);
    assert_inverse_theme_yaml(&capture.frame);
}

// ---------------------------------------------------------------------------
// Phase 6 — agent-state styling captures
// ---------------------------------------------------------------------------

/// An invalid `agent` frontmatter value renders the `Invalid Agent` header in
/// red (SGR 31/91) inside the Agent table cell.
#[test]
#[serial(level2_terminal)]
fn level2_dry_run_invalid_agent_renders_red_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let capture = run_dry_run_compose_with_doc(
        &mut harness,
        FIXTURE_INVALID_AGENT,
        "claudine-dryrun-invalid-l2",
    );

    let plain = &capture.frame.plain;
    assert!(
        plain.contains("Invalid Agent"),
        "expected 'Invalid Agent' in Agent cell.\nplain:\n{plain}",
    );
    assert!(
        plain.contains("totally-invalid-agent"),
        "expected the invalid hint in Agent cell.\nplain:\n{plain}",
    );

    let agent_raw = row_raw(&capture.frame, "Agent")
        .unwrap_or_else(|| panic!("Agent row not found.\nraw:\n{}", capture.frame.raw));
    assert!(
        has_red(agent_raw),
        "expected the Invalid Agent header to render red (SGR 31/91).\nrow: {agent_raw:?}",
    );
}

/// A not-installed `agent` frontmatter value renders the
/// `Agent Not Installed:` header in yellow (SGR 33/93) and the provider name
/// in dim (SGR 2) inside the Agent table cell.
#[test]
#[serial(level2_terminal)]
fn level2_dry_run_not_installed_renders_yellow_dim_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let capture = run_dry_run_compose_with_doc(
        &mut harness,
        FIXTURE_NOT_INSTALLED,
        "claudine-dryrun-notinst-l2",
    );

    let plain = &capture.frame.plain;
    assert!(
        plain.contains("Agent Not Installed"),
        "expected 'Agent Not Installed' in Agent cell.\nplain:\n{plain}",
    );
    assert!(
        plain.contains("Qwen"),
        "expected the not-installed provider name in Agent cell.\nplain:\n{plain}",
    );

    let agent_raw = row_raw(&capture.frame, "Agent")
        .unwrap_or_else(|| panic!("Agent row not found.\nraw:\n{}", capture.frame.raw));
    assert!(
        has_yellow(agent_raw),
        "expected the 'Agent Not Installed:' header to render yellow (SGR 33/93).\n\
         row: {agent_raw:?}",
    );
    assert!(
        has_dim(agent_raw),
        "expected the dimmed provider name in the Agent cell (SGR 2).\nrow: {agent_raw:?}",
    );
}

/// The no-agent state produces a multi-line Agent cell. In a real terminal the
/// table's two-column alignment and the `1ch` left offset must be preserved
/// across all continuation lines.
#[test]
#[serial(level2_terminal)]
fn level2_dry_run_no_agent_multiline_alignment_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let capture = run_dry_run_compose_with_doc(
        &mut harness,
        FIXTURE_NO_AGENT,
        "claudine-dryrun-noagent-l2",
    );

    let plain = &capture.frame.plain;
    assert!(
        plain.contains("didn't specify the Agent"),
        "expected no-agent unordered list in Agent cell.\nplain:\n{plain}",
    );

    assert_agent_cell_alignment(&capture.frame, "Agent");
}
