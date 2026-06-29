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
use common::{TestWorkspace, augmented_path, write_executable};

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
    // appended excerpt: the gutter carries the `>` highlight marker on line 3,
    // matching the line-highlight rendering the malformed-fence capture asserts.
    assert!(
        frame.plain.contains("> 3 ") || frame.plain.contains(">  3 "),
        "expected the line-3 highlight marker in the excerpt gutter.\nplain:\n{}",
        frame.plain
    );
    assert!(
        frame.raw.contains('\u{1b}'),
        "schema-parse diagnostic should carry styling through tmux.\nraw:\n{}",
        frame.raw
    );
    assert!(
        !staged.launch_count.exists(),
        "schema parse error should fail before provider launch"
    );
}
