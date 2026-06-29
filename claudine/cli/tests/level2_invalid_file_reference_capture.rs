//! Level 2 real-terminal capture for the invalid-file-reference report.
//!
//! Drives `claudine compose` through a real tmux pane against a prompt whose
//! `iteration` frontmatter key references a MISSING file through
//! `frontmatter(spec, 'review_iterations')`. A present-but-unresolvable file
//! reference is authoring-fatal (real-errors ratified decision), so composition
//! aborts before the provider launches and renders the cause-driven report.
//!
//! Asserts the user-visible report reaches the rendered terminal surface: the
//! root-cause "invalid file path" headline (NOT the mechanism word
//! "transform"/"interpolation"), the receiving `iteration` key, the focused
//! excerpt surfacing the involved keys (`spec:` and `iteration:`), an OSC8 link
//! to the prompt file plus the filename in plain text, and styling in `raw`.
//!
//! This mirrors the Darkmatter `md compose` capture
//! (`darkmatter/cli/tests/level2_errors.rs`) so the same failure renders
//! identically through both binaries, per the real-errors spec.

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

// `iteration` evaluates `frontmatter(spec, …)` against a file that does not
// exist; the present-but-unresolvable reference is authoring-fatal. `spec` is a
// sibling key the expression references, so the focused excerpt surfaces both
// `spec` and the receiving `iteration` key. `agent` is unrelated and excluded.
// Top-level keys (not nested under `$schema`) are used because `$schema` values
// are type specifications, not interpolation expressions.
const INVALID_FILE_REFERENCE_DOC: &str = "\
---
agent: \"codex\"
spec: \"does-not-exist-spec.md\"
iteration: \"{{ frontmatter(spec, 'review_iterations') ? frontmatter(spec, 'review_iterations') : 1 }}\"
---
# Body
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
    fs::write(&doc, INVALID_FILE_REFERENCE_DOC).unwrap();

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

    let frame = wait_for_pane_marker(harness, "invalid file path", Duration::from_secs(15));
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    frame
}

#[test]
#[serial(level2_terminal)]
fn level2_invalid_file_reference_renders_headline_excerpt_and_osc8_link_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    let staged = stage("claudine-invalid-file-ref-l2");
    let frame = capture_command(&mut harness, &staged);

    // Root-cause headline — the mechanism word "interpolation"/"transform" must
    // NOT be the headline; the cause ("invalid file path") is.
    assert!(
        frame.plain.contains("invalid file path"),
        "expected root-cause headline 'invalid file path'. plain:\n{}",
        frame.plain
    );

    // Names the receiving frontmatter key and surfaces the involved keys in the
    // focused excerpt (`spec` is referenced by the expression; `iteration` is the
    // receiving key).
    for needle in ["iteration", "spec:"] {
        assert!(
            frame.plain.contains(needle),
            "invalid-file-reference report missing {needle:?}.\nplain:\n{}",
            frame.plain
        );
    }

    // OSC8 hyperlink to the prompt file. macOS aliases `/var` to `/private/var`;
    // the binary embeds whichever spelling reaches it, so accept either form.
    let canonical = staged.doc.canonicalize().expect("canonicalize prompt path");
    let canonical_url = format!("file://{}", canonical.to_string_lossy());
    let aliased_url = canonical_url.replacen("file:///private", "file://", 1);
    let osc8_canonical = format!("\x1b]8;;{}", canonical_url);
    let osc8_aliased = format!("\x1b]8;;{}", aliased_url);
    assert!(
        frame.raw.contains(&osc8_canonical) || frame.raw.contains(&osc8_aliased),
        "expected OSC8 hyperlink for {canonical_url} (or aliased {aliased_url}).\nraw:\n{}",
        frame.raw
    );
    assert!(
        frame.plain.contains("plan.md"),
        "expected the linked prompt filename in plain text.\nplain:\n{}",
        frame.plain
    );

    assert!(
        frame.raw.contains('\u{1b}'),
        "invalid-file-reference report should carry styling through tmux.\nraw:\n{}",
        frame.raw
    );
    assert!(
        !staged.launch_count.exists(),
        "invalid file reference should fail before provider launch"
    );
}
