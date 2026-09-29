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

/// The fixture's Claude configuration directory. Pointing `CLAUDE_CONFIG_DIR`
/// at it keeps native Claude discovery inside the fixture on every OS:
/// Windows resolves the user home from the system, not from `USERPROFILE`.
fn claude_dir(fixture: &CliProcessFixture) -> PathBuf {
    fixture.home().join(".claude")
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> (i32, String, String) {
    let output = fixture
        .command()
        .env("CLAUDE_CONFIG_DIR", claude_dir(fixture))
        .args(args)
        .output()
        .expect("claudine runs");
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

fn listing(stdout: &str) -> serde_json::Value {
    serde_json::from_str(stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"))
}

/// Without a daemon the managed source fails, but native discovery (Claude
/// Code's registry, empty here) still answers: a partial listing succeeds and
/// names the missing route.
#[test]
fn listing_without_a_daemon_reports_the_missing_route_and_changes_no_configuration() {
    let fixture = CliProcessFixture::named("steer-list");
    let before = files_under(fixture.home());

    let (code, stdout, stderr) = run(&fixture, &["steer", "--list", "--json"]);
    assert_eq!(code, 0, "partial discovery succeeds; stderr:\n{stderr}");
    let document = listing(&stdout);
    assert_eq!(document["sessions"], serde_json::json!([]));
    let errors = document["discovery_errors"].as_array().unwrap();
    assert_eq!(errors.len(), 1, "{document}");
    assert_eq!(errors[0]["source"], "managed");

    let (code, _, _) = run(&fixture, &["steer", "--session", "managed:00000000-0000-0000-0000-000000000001", "--json", "go"]);
    assert_eq!(code, 1);

    assert_eq!(files_under(fixture.home()), before, "steer never writes configuration or state into the home");
    assert!(!fixture.audio_spool().exists(), "no lifecycle audio was published");
}

/// A Claude Code record for a live process is listed natively, with its
/// reason for being unavailable; a record whose process has exited is not.
#[test]
fn native_claude_sessions_are_listed_from_claudes_registry() {
    let fixture = CliProcessFixture::named("steer-native-claude");
    let registry = claude_dir(&fixture).join("sessions");
    std::fs::create_dir_all(&registry).unwrap();
    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let record = |pid: u32, session: &str| {
        serde_json::json!({
            "pid": pid, "sessionId": session, "cwd": "/work/project", "kind": "interactive", "entrypoint": "cli",
            "status": "busy", "version": "2.1.284", "name": "fixture session", "startedAt": now_ms,
            "procStart": "Mon Sep 28 10:00:00 2026", "messagingSocketPath": "/tmp/cc-socks/0.sock", "peerProtocol": 1,
        })
        .to_string()
    };
    // This test's own process stands in for a running Claude Code.
    let live = std::process::id();
    std::fs::write(registry.join(format!("{live}.json")), record(live, "live-session")).unwrap();
    let mut exited = std::process::Command::new(std::env::current_exe().unwrap()).arg("--list").stdout(std::process::Stdio::null()).spawn().unwrap();
    let gone = exited.id();
    exited.wait().unwrap();
    std::fs::write(registry.join(format!("{gone}.json")), record(gone, "stale-session")).unwrap();

    let (code, stdout, stderr) = run(&fixture, &["steer", "--list", "--json"]);
    assert_eq!(code, 0, "{stderr}");
    let document = listing(&stdout);
    let sessions = document["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1, "only the live session: {document}");
    let row = &sessions[0];
    assert_eq!(row["provider"], "claude");
    assert_eq!(row["origins"], serde_json::json!(["native"]));
    assert_eq!(row["state"], "working");
    assert_eq!(row["launch_profile"], "ordinary-interactive");
    assert_eq!(row["provider_version"], "2.1.284");
    assert_eq!(row["availability"], "unavailable");
    let id = row["id"].as_str().unwrap();
    assert!(id.starts_with(&format!("native:claude:{live}:")) && id.ends_with(":live-session"), "{id}");
    assert!(row["reason"].as_str().unwrap().contains("peer-"), "the reason names the researched mechanism: {row}");

    // An explicit send to it sends nothing.
    let (code, stdout, _) = run(&fixture, &["steer", "--session", id, "--json", "go"]);
    assert_eq!(code, 1);
    assert_eq!(listing(&stdout)["outcome"], "unavailable");
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
