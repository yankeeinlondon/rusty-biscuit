//! Darkmatter's user configuration at Claudine's startup: a file that does not
//! load stops the run before any provider launches or command runs, with a
//! located error; a missing file is one warning per invocation naming
//! `claudine init`.
//!
//! The provider is the compiled `claudine-fake-goose` in its marker mode (one
//! line per launch in `PROVIDER_MARKER`), and the document's frontmatter
//! command is the compiled `claudine-argv-recorder`, so a run that composed or
//! launched anything leaves a record.

use std::fs;

use test_toolkit::argv_recorder::ARGV_RECORDER_FILE;

use crate::common::{self, CliProcessFixture};

const DOCUMENT: &str = "---\nprompt: Say hello\nprobe: \"$(argv-recorder ran)\"\n---\nHello.\n";

struct Run {
    success: bool,
    stderr: String,
    launches: usize,
    commands_ran: bool,
}

/// Composes the document with `args` before the file; the user's permissions
/// approve its command where the fixture keeps a user configuration.
fn run(fixture: &CliProcessFixture, args: &[&str]) -> Run {
    let suffix = std::env::consts::EXE_SUFFIX;
    for (binary, name) in [
        (biscuit_test_harness::bin_exe!("claudine-fake-goose"), "goose"),
        (biscuit_test_harness::bin_exe!("claudine-argv-recorder"), "argv-recorder"),
    ] {
        fs::copy(binary, fixture.bin_dir().join(format!("{name}{suffix}")))
            .expect("fixture binaries are built with `test-fixtures`");
    }
    let document = fixture.cwd().join("hello.md");
    fs::write(&document, DOCUMENT).unwrap();
    let marker = fixture.cwd().join("provider-runs.log");
    let records = fixture.cwd().join("argv.bin");
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", &marker)
        .env(ARGV_RECORDER_FILE, &records)
        .args(["compose", "--goose"])
        .args(args)
        .arg(&document)
        .output()
        .unwrap();
    Run {
        success: output.status.success(),
        stderr: common::strip_ansi(&String::from_utf8_lossy(&output.stderr)),
        launches: fs::read_to_string(&marker).map_or(0, |log| log.lines().count()),
        commands_ran: records.exists(),
    }
}

#[test]
fn a_user_file_that_does_not_load_stops_before_anything_is_dispatched() {
    let fixture = CliProcessFixture::named("dm-config-broken");
    fixture.seed_user_config();
    let config = fixture.home().join(".darkmatter.yaml");
    fs::write(&config, "register: []\nnot-a-setting: 1\n").unwrap();

    let run = run(&fixture, &[]);

    assert!(!run.success, "{}", run.stderr);
    assert_eq!(run.launches, 0, "no provider launched");
    assert!(!run.commands_ran, "no command ran");
    let squeezed: String = run.stderr.split_whitespace().collect();
    let located: String = format!("{}:2:1", config.display()).split_whitespace().collect();
    assert!(squeezed.contains(&located), "file, line, and column: {}", run.stderr);
}

#[test]
fn a_missing_user_file_is_reported_once_naming_claudine_init() {
    let fixture = CliProcessFixture::named("dm-config-missing");
    fixture.seed_user_config();
    let fixture = fixture.without_darkmatter_config();

    // A missing file pre-approves no command, so `--yolo` approves this one.
    let run = run(&fixture, &["--yolo"]);

    assert!(run.success, "{}", run.stderr);
    assert_eq!(run.launches, 1);
    assert_eq!(run.stderr.matches("does not exist").count(), 1, "{}", run.stderr);
    assert!(run.stderr.contains("claudine init"), "{}", run.stderr);
}
