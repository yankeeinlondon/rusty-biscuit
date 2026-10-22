//! Level 1 PTY tests for when Claudine asks for shell approval
//! (`2026-10-27-darkmatter-lifecycle-ownership`, acceptance 9 and spec
//! section 4): every approval prompt a run needs is shown before its first
//! provider launch, in a fixed order, and no prompt follows a launch, even
//! though the lifecycle shells it approved run after the launch.
//!
//! The pinned order is the order the preflight audit collects commands in,
//! which the lifecycle extraction moved but did not change.
//!
//! ## Tier
//!
//! **Level 1**, gated like `level1_dry_run_pty.rs`: `#![cfg(unix)]` because
//! `expectrl`'s `OsSession` is Unix-only, and `expect_level!` fails when the
//! PTY is missing. No terminal emulator participates.

use expectrl::Session;
use expectrl::process::Healthcheck as _;
use expectrl::session::OsSession;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};
use test_toolkit::{Level, expect_level};

use crate::common;
use common::pty::{read_for, stage_default_config};
use common::{CliProcessFixture, pty_available, write_executable};

/// The last static line the approval handler writes before it reads a choice.
const PROMPT_END: &str = "the blacklist; stops the run)";

/// One approval prompt: the token of the command it asked about, and whether
/// any provider had launched when it appeared.
#[derive(Debug, PartialEq)]
struct Prompt {
    token: String,
    after_launch: bool,
}

/// Stage a `goose` stub that appends `launch` to `trace.log`.
fn stage_provider(fixture: &CliProcessFixture) {
    stage_default_config(fixture.home());
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!("#!/bin/sh\nprintf 'launch\\n' >> '{}'\nexit 0\n", fixture.cwd().join("trace.log").display()),
    );
}

/// Run `claudine <args>` under a PTY, answering every approval prompt with
/// "Allow once". Returns each prompt, in order, with the command token (one
/// of `tokens`) it named, and the run's `trace.log`.
fn approve_every_prompt(fixture: &CliProcessFixture, args: &[&str], tokens: &[&str]) -> (Vec<Prompt>, String) {
    let trace = fixture.cwd().join("trace.log");
    let mut command = fixture.command_std();
    command.args(args);
    command.env("TERM", "xterm-256color");
    command.env_remove("NO_COLOR");
    command.env_remove("CLAUDINE_PLAIN");
    command.env_remove("CI");
    let mut session: OsSession = Session::spawn(command).expect("spawn PTY session");

    let mut prompts = Vec::new();
    let mut transcript = String::new();
    let mut unanswered = String::new();
    let stop = Instant::now() + Duration::from_secs(20);
    while Instant::now() < stop {
        let chunk = read_for(&mut session, Duration::from_millis(100));
        transcript.push_str(&chunk);
        unanswered.push_str(&chunk);
        let plain = common::strip_ansi(&unanswered);
        if let Some(end) = plain.find(PROMPT_END) {
            let region = &plain[plain[..end].rfind("Shell Approval Required").unwrap_or(0)..end];
            let named: Vec<&str> = tokens.iter().copied().filter(|token| region.contains(token)).collect();
            assert_eq!(named.len(), 1, "a prompt names exactly one staged command; prompt:\n{region}");
            let launched = fs::read_to_string(&trace).is_ok_and(|text| text.contains("launch"));
            prompts.push(Prompt { token: named[0].to_string(), after_launch: launched });
            session.write_all(b"3\n").expect("answer Allow once");
            session.flush().ok();
            unanswered.clear();
            continue;
        }
        if !chunk.is_empty() {
            continue;
        }
        if !session.is_alive().unwrap_or(false) {
            break;
        }
    }
    assert!(!session.is_alive().unwrap_or(false), "the run did not finish; transcript:\n{}", common::strip_ansi(&transcript));
    (prompts, fs::read_to_string(&trace).unwrap_or_default())
}

fn write(path: &Path, text: &str) {
    fs::write(path, text).expect("write fixture document");
}

fn before_launch(tokens: &[&str]) -> Vec<Prompt> {
    tokens.iter().map(|token| Prompt { token: token.to_string(), after_launch: false }).collect()
}

/// A document's body and lifecycle shells are all approved before its
/// provider launches; the `success` and `finalize` shells then run after the
/// launch without a second prompt.
#[test]
fn level1_pty_a_documents_approvals_all_precede_its_launch() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
    let fixture = CliProcessFixture::named("level1-approval-order-document");
    stage_provider(&fixture);
    write(
        &fixture.cwd().join("doc.md"),
        "---\n\
         start:\n  - shell: \"printf 'tok-start\\\\n' >> trace.log\"\n\
         success:\n  - shell: \"printf 'tok-success\\\\n' >> trace.log\"\n\
         finalize:\n  - shell: \"printf 'tok-finalize\\\\n' >> trace.log\"\n\
         ---\n\
         ::shell echo tok-body\n",
    );
    let tokens = ["tok-body", "tok-start", "tok-success", "tok-finalize"];

    let (prompts, trace) = approve_every_prompt(&fixture, &["compose", "--goose", "doc.md"], &tokens);

    assert_eq!(prompts, before_launch(&["tok-body", "tok-start", "tok-success", "tok-finalize"]));
    assert_eq!(trace, "tok-start\nlaunch\ntok-success\ntok-finalize\n");
}

/// A sequence front-loads approval for every task and every lifecycle shell
/// before its first task launches, and asks nothing once a task has
/// launched. The order is the sequence preflight's: the sequence graph's own
/// commands (its lifecycle, then its shell tasks), then each prompt task's
/// document in declaration order, body before lifecycle.
#[test]
fn level1_pty_a_sequence_front_loads_every_approval_in_order() {
    expect_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");
    let fixture = CliProcessFixture::named("level1-approval-order-sequence");
    stage_provider(&fixture);
    write(
        &fixture.cwd().join("seq.md"),
        "---\n\
         finalize:\n  - shell: \"printf 'tok-seq-finalize\\\\n' >> trace.log\"\n\
         sequence:\n\
         \x20 - name: first\n    prompt: ./first.md\n\
         \x20 - name: middle\n    shell: \"printf 'tok-task\\\\n' >> trace.log\"\n\
         \x20 - name: last\n    prompt: ./last.md\n\
         ---\n",
    );
    write(
        &fixture.cwd().join("first.md"),
        "---\nsuccess:\n  - shell: \"printf 'tok-first-success\\\\n' >> trace.log\"\n---\nFirst\n",
    );
    write(
        &fixture.cwd().join("last.md"),
        "---\nstart:\n  - shell: \"printf 'tok-last-start\\\\n' >> trace.log\"\n---\n::shell echo tok-last-body\n",
    );
    let tokens = ["tok-seq-finalize", "tok-first-success", "tok-task", "tok-last-start", "tok-last-body"];

    let (prompts, trace) = approve_every_prompt(&fixture, &["sequence", "--goose", "seq.md"], &tokens);

    assert_eq!(
        prompts,
        before_launch(&["tok-seq-finalize", "tok-task", "tok-first-success", "tok-last-body", "tok-last-start"])
    );
    assert_eq!(trace, "launch\ntok-first-success\ntok-task\ntok-last-start\nlaunch\ntok-seq-finalize\n");
}
