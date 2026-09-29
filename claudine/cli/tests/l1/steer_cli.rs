//! `claudine steer` through the shipped binary: argument handling, exit
//! codes, stdout/stderr separation, and the no-configuration guarantee.
//!
//! The fixture points the local Rendezvous endpoint at a private path
//! nothing listens on, so these runs see "no local daemon" and can never
//! reach the developer's own sessions. Selection, consent, and delivery are
//! covered in-crate (`commands::steer::tests`) with scripted input and a
//! real daemon.

use std::path::{Path, PathBuf};

use crate::common;
use common::CliProcessFixture;

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> (i32, String, String) {
    let output = fixture.command().args(args).output().expect("claudine runs");
    (
        output.status.code().expect("exited normally"),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Line wrapping collapsed, so a phrase matches wherever it was broken.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn listing_without_a_daemon_fails_on_stderr_and_changes_no_configuration() {
    let fixture = CliProcessFixture::named("steer-list");
    let before = files_under(fixture.home());

    let (code, stdout, stderr) = run(&fixture, &["steer", "--list", "--json"]);
    assert_eq!(code, 1, "total discovery failure fails; stderr:\n{stderr}");
    assert!(stdout.is_empty(), "no partial JSON document on stdout: {stdout}");
    assert!(flat(&stderr).contains("session discovery failed for every source"), "{stderr}");

    let (code, _, _) = run(&fixture, &["steer", "--session", "managed:00000000-0000-0000-0000-000000000001", "--json", "go"]);
    assert_eq!(code, 1);

    assert_eq!(files_under(fixture.home()), before, "steer never writes configuration or state into the home");
    assert!(!fixture.audio_spool().exists(), "no lifecycle audio was published");
}

#[test]
fn invalid_invocations_exit_with_usage_errors() {
    let fixture = CliProcessFixture::named("steer-usage");
    let cases: &[(&[&str], &str)] = &[
        (&["steer", " \t "], "empty or whitespace-only"),
        (&["steer", "--session", "managed:00000000", "go"], "not a session ID"),
        (&["steer", "--json", "go"], "--session"),
        // Piped stdin: no terminal to choose a session.
        (&["steer", "go"], "claudine steer --list"),
        (&["steer", "--list", "go"], "cannot be used with"),
        (&["steer"], "required"),
        (&["steer", "--yes", "go"], "unexpected argument"),
    ];
    for (args, needle) in cases {
        let (code, stdout, stderr) = run(&fixture, args);
        assert_eq!(code, 2, "{args:?}: stderr:\n{stderr}");
        assert!(stdout.is_empty(), "{args:?}: usage errors print nothing on stdout: {stdout}");
        assert!(flat(&stderr).contains(needle), "{args:?}: expected `{needle}` in:\n{stderr}");
    }
}

#[test]
fn help_documents_the_forms_without_a_consent_bypass() {
    let fixture = CliProcessFixture::named("steer-help");
    let (code, stdout, stderr) = run(&fixture, &["steer", "--help"]);
    assert_eq!(code, 0, "{stderr}");
    let help = flat(&stdout);
    for flag in ["--list", "--session", "--json", "MESSAGE"] {
        assert!(help.contains(flag), "`{flag}` in help:\n{stdout}");
    }
    for bypass in ["--yes", "--force", "--interrupt"] {
        assert!(!help.contains(bypass), "no `{bypass}` bypass flag:\n{stdout}");
    }
}
