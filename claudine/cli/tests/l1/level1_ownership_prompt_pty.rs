//! Level 1 PTY test for the ownership ambiguity prompt.
//!
//! The document lists Claude and Codex as candidates, so `-c foo` is read two
//! ways: Claude's `-c` takes no value, Codex's takes one. Under a TTY Claudine
//! asks which agent the arguments are for. The answer decides who owns `foo`
//! and nothing else: the run still resolves its own provider (Codex, the only
//! one installed), and the resolved-provider check judges the result.
//!
//! ## Tier
//!
//! **Level 1**, gated like `level1_provider_picker_pty.rs`: `#![cfg(unix)]`
//! because `expectrl`'s `OsSession` is Unix-only, and `expect_level!` fails
//! when the PTY is missing. No terminal emulator participates.

use expectrl::Session;
use expectrl::process::Healthcheck as _;
use expectrl::session::OsSession;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};
use test_toolkit::{Level, expect_level};

use crate::common;
use common::pty::*;
use common::{CliProcessFixture, pty_available, write_executable};

/// Run `compose plan.md -c foo` under a PTY, answer the prompt with `keys`,
/// and return the transcript and the Codex argv (empty when it never ran).
fn answer(name: &str, keys: &[u8]) -> (String, Vec<String>) {
    let fixture = CliProcessFixture::named(name);
    stage_default_config(fixture.home());
    let argv_file = fixture.cwd().join("codex.argv");
    write_executable(
        &fixture.bin_dir().join("codex"),
        &format!(
            "#!/bin/sh\nfor arg in \"$@\"; do printf '%s\\n' \"$arg\"; done > '{}'\nexit 0\n",
            argv_file.display()
        ),
    );
    let md_file = fixture.cwd().join("plan.md");
    fs::write(&md_file, "---\nagent: [claude, codex]\n---\nPlan body.\n").unwrap();

    // `expectrl` needs a live `std::process::Command`; the builder's raw
    // surface hands one over carrying the same policy.
    let mut cmd = fixture.command_std();
    cmd.args(["compose", md_file.to_str().unwrap(), "-c", "foo"]);
    cmd.env("TERM", "xterm-256color");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "Codex", Duration::from_secs(10));
    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
    assert!(!argv_file.exists(), "Codex launched before the prompt was answered");
    session.write_all(keys).expect("answer the prompt");
    session.flush().ok();

    let stop = Instant::now() + Duration::from_secs(15);
    let mut transcript = pre;
    while Instant::now() < stop && !argv_file.exists() && session.is_alive().unwrap_or(false) {
        transcript.push_str(&read_for(&mut session, Duration::from_millis(200)));
    }
    transcript.push_str(&read_for(&mut session, Duration::from_millis(200)));
    (common::strip_ansi(&transcript), read_argv(&argv_file))
}

fn read_argv(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

#[test]
fn level1_pty_choosing_codex_forwards_the_word_as_its_value() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    // Claude is listed first and is the default; `j` moves to Codex.
    let (transcript, argv) = answer("level1-ownership-prompt-codex", b"j\r");

    assert!(
        transcript.contains("Resolving how the arguments after the composition file are read"),
        "{transcript}"
    );
    assert!(transcript.contains("not which agent runs"), "{transcript}");
    let at = argv.iter().position(|arg| arg == "-c").unwrap_or_else(|| panic!("{argv:?}\n{transcript}"));
    assert_eq!(argv.get(at + 1).map(String::as_str), Some("foo"), "{argv:?}");
}

#[test]
fn level1_pty_choosing_claude_decides_ownership_only() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    // Claude's reading leaves `foo` to Claudine, but the run resolves to
    // Codex, whose `-c` then has no value: the check refuses the spawn.
    let (transcript, argv) = answer("level1-ownership-prompt-claude", b"\r");

    assert!(argv.is_empty(), "Codex must not be spawned: {argv:?}");
    let flat = transcript.split_whitespace().collect::<Vec<_>>().join(" ").replace("┃ ", "");
    assert!(flat.contains("takes a value for Codex"), "{transcript}");
}
