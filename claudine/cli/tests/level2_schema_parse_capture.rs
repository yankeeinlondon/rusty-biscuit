//! Level 2 real-terminal capture for the `$schema` parse (`SchemaParse`) report.
//!
//! Drives `claudine compose` through a real tmux pane against a prompt whose
//! inline `$schema` mapping has a grammar error (a `,` constraint separator
//! where `;` is required) and asserts the user-visible diagnostic — the
//! `invalid schema` headline, the appended frontmatter excerpt with the
//! offending `$schema.spec` line highlighted, and the OSC8-linked prompt file —
//! reaches the rendered terminal surface with styling.

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use test_toolkit::{Level, require_level};

mod common;
use common::{
    TestWorkspace, assert_row_is_styled, augmented_path, clear_no_color, write_executable,
};

// A bad constraint separator (`,` instead of `;`) in the `spec` type string is
// a grammar error attributed to `$schema.spec`, on file line 3.
const SCHEMA_DOC: &str = "\
---
$schema:
    spec: file(required, match(**/*spec*.md))
spec: \"x\"
---
Plan.
";

struct Staged {
    workspace: TestWorkspace,
    bin_dir: PathBuf,
    doc: PathBuf,
    launch_count: PathBuf,
}

fn stage(name: &str) -> Staged {
    let workspace = TestWorkspace::named(name);
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let claudine_dir = workspace.path().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();

    let launch_count = workspace.path().join("provider-launched.txt");
    write_provider_stub(&bin_dir);

    let doc = workspace.path().join("plan.md");
    fs::write(&doc, SCHEMA_DOC).unwrap();

    Staged {
        workspace,
        bin_dir,
        doc,
        launch_count,
    }
}

fn write_provider_stub(bin_dir: &Path) {
    let script = "#!/bin/sh\nprintf launched >> \"$CLAUDINE_PROVIDER_LAUNCH_COUNT\"\nexit 0\n";
    write_executable(&bin_dir.join("goose"), script);
}

fn wait_for_pane_marker(
    harness: &mut TmuxHarness,
    marker: &str,
    deadline: Duration,
) -> CapturedFrame {
    let stop = Instant::now() + deadline;
    loop {
        let frame = harness.capture().expect("capture pane");
        if frame.plain.contains(marker) {
            return frame;
        }
        if Instant::now() >= stop {
            panic!(
                "marker {marker:?} did not appear within {deadline:?}.\nplain:\n{}",
                frame.plain
            );
        }
        harness.settle();
    }
}

fn capture_command(harness: &mut TmuxHarness, staged: &Staged) -> CapturedFrame {
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

    let cmd = format!("{claudine} compose --goose {}", staged.doc.display());
    harness
        .send_command_with_env(
            &cmd,
            &[
                ("HOME", home.as_str()),
                ("PATH", path.as_str()),
                ("FORCE_COLOR", "1"),
                ("COLUMNS", "100"),
                ("CLAUDINE_PROVIDER_LAUNCH_COUNT", launch_count.as_str()),
            ],
        )
        .expect("send claudine command");

    let frame = wait_for_pane_marker(harness, "invalid schema", Duration::from_secs(15));
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    frame
}

/// The `48;2;R;G;B` truecolor background SGR codes present on the single captured
/// pane row whose visible text contains `needle`.
///
/// The frontmatter excerpt's offending line is highlighted by painting a
/// distinct background color — the `CodeBlock` appendix does not use a `>` gutter
/// glyph — so a highlighted row carries a background its unhighlighted siblings
/// do not. The row is located by its escape-stripped text, then the original
/// (escapes-intact) row is scanned for background codes.
fn row_truecolor_backgrounds(raw: &str, needle: &str) -> Vec<String> {
    let row = raw
        .lines()
        .find(|line| biscuit_test_harness::strip_ansi(line).contains(needle))
        .unwrap_or_else(|| panic!("no captured row contains {needle:?}.\nraw:\n{raw}"));

    let mut backgrounds = Vec::new();
    let mut rest = row;
    while let Some(pos) = rest.find("\u{1b}[48;2;") {
        // Skip the `\x1b[` prefix; capture the `48;2;…` body up to the `m`.
        let body = &rest[pos + 2..];
        let Some(end) = body.find('m') else { break };
        backgrounds.push(body[..end].to_string());
        rest = &body[end + 1..];
    }
    backgrounds
}

#[test]
#[serial(level2_terminal)]
fn level2_schema_parse_renders_highlighted_excerpt_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-schema-parse-l2");
    let frame = capture_command(&mut harness, &staged);

    for needle in [
        "invalid schema",
        "spec",
        "spec: file(required, match(**/*spec*.md))",
        "yaml",
    ] {
        assert!(
            frame.plain.contains(needle),
            "schema-parse diagnostic missing {needle:?}.\nplain:\n{}",
            frame.plain
        );
    }

    // The offending `$schema.spec` line (file line 3) is highlighted in the
    // appended excerpt by painting a distinct background — the `CodeBlock`
    // appendix highlights with color, not a gutter glyph. So the `spec:`
    // type-string row carries a truecolor background its unhighlighted
    // `spec: "x"` sibling row does not.
    let offending_backgrounds =
        row_truecolor_backgrounds(&frame.raw, "spec: file(required, match(**/*spec*.md))");
    let sibling_backgrounds = row_truecolor_backgrounds(&frame.raw, "spec: \"x\"");
    assert!(
        !offending_backgrounds.is_empty(),
        "the offending `$schema.spec` row should carry a code-block background.\nraw:\n{}",
        frame.raw
    );
    assert!(
        offending_backgrounds
            .iter()
            .any(|bg| !sibling_backgrounds.contains(bg)),
        "the offending `$schema.spec` row (file line 3) should be highlighted with a \
         background distinct from its unhighlighted `spec: \"x\"` sibling row.\n\
         offending: {offending_backgrounds:?}\nsibling: {sibling_backgrounds:?}\nraw:\n{}",
        frame.raw
    );
    assert_row_is_styled(&frame.raw, "invalid schema", "schema-parse diagnostic");
    assert!(
        !staged.launch_count.exists(),
        "schema parse error should fail before provider launch"
    );
}
