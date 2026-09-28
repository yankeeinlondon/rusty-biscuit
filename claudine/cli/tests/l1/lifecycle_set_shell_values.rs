//! Level-1 process coverage for a lifecycle `set` that runs a command (R9): the
//! command is approved at pre-flight under its bare bytes, runs when `start`
//! executes, and a later `when:` reads its typed `::result`.
//!
//! The commands are `git` invocations that need no repository, so they behave
//! the same on every host: `git check-ref-format --branch ran/x` exits 0 and
//! prints `ran/x`; `git rev-parse --verify nope` exits 128 with a `fatal:`
//! message whether or not the directory is a repository.

use crate::common;

use common::CliProcessFixture;
use common::write;
#[cfg(unix)]
use common::write_executable;
use std::path::Path;

/// A Claude stub that reports one successful result and exits 0.
fn write_succeeding_claude(bin_dir: &Path) {
    #[cfg(unix)]
    write_executable(
        &bin_dir.join("claude"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"init","session_id":"session-1","model":"claude-test"}'
printf '%s\n' '{"type":"result","subtype":"success","result":"done","session_id":"session-1","is_error":false}'
exit 0
"#,
    );

    #[cfg(windows)]
    write(
        &bin_dir.join("claude.cmd"),
        "@echo off\r\n\
echo {\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"session-1\",\"model\":\"claude-test\"}\r\n\
echo {\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"session-1\",\"is_error\":false}\r\n\
exit /b 0\r\n",
    );
}

const FAILING: &str = "git rev-parse --verify nope";
const SUCCEEDING: &str = "git check-ref-format --branch ran/x";

const DOCUMENT: &str = r#"---
start:
  stack:
    - action:
        - set:
            diff: "$(git rev-parse --verify nope)::result"
            named: "$(git check-ref-format --branch ran/x)::ok"
    - when: "!diff.ok && diff.code == 128 && named"
      action:
        - stderr: "diff code={{ diff.code }} ok={{ diff.ok }} fatal={{ starts_with(diff.stderr, 'fatal:') }}"
    - when: "diff.ok"
      action:
        - stderr: "diff unexpectedly ok"
---
Body.
"#;

struct Run {
    code: Option<i32>,
    output: String,
}

fn run(name: &str, whitelist: &[&str], args: &[&str]) -> Run {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    write_succeeding_claude(fixture.bin_dir());
    write(&fixture.cwd().join("doc.md"), DOCUMENT);
    let policy: String = whitelist.iter().map(|command| format!("exact {command}\n")).collect();
    write(&fixture.cwd().join(".darkmatter-shell-whitelist"), &policy);
    let output = fixture
        .command_builder()
        // `git` runs the commands under test; Git for Windows is outside the
        // minimal system set.
        .host_path()
        .build()
        .args(args)
        .output()
        .expect("claudine runs");
    Run {
        code: output.status.code(),
        output: common::strip_ansi(&format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )),
    }
}

#[test]
fn a_later_when_reads_the_typed_result_of_a_start_set() {
    let run = run("set-shell-result", &[FAILING, SUCCEEDING], &["compose", "--claude", "doc.md"]);
    assert_eq!(run.code, Some(0), "{}", run.output);
    assert!(
        run.output.contains("diff code=128 ok=false fatal=true"),
        "{}",
        run.output
    );
    assert!(!run.output.contains("unexpectedly"), "{}", run.output);
}

#[test]
fn pre_flight_approves_the_bare_command_before_anything_runs() {
    // Only one of the two commands is approved: pre-flight refuses the run
    // and names the other by its bare bytes, with no suffix.
    let run = run("set-shell-unapproved", &[SUCCEEDING], &["compose", "--claude", "doc.md"]);
    assert_ne!(run.code, Some(0), "{}", run.output);
    // Drop the error box's border glyphs and hard wraps so the command reads
    // as one line.
    let collapsed = run.output.replace('┃', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(collapsed.contains(FAILING), "{collapsed}");
    assert!(!collapsed.contains("::result"), "{collapsed}");
    assert!(!collapsed.contains("diff code=128"), "{collapsed}");
}

#[test]
fn a_dry_run_executes_no_lifecycle_assignment() {
    let run = run(
        "set-shell-dry-run",
        &[FAILING, SUCCEEDING],
        &["compose", "--claude", "--dry-run", "doc.md"],
    );
    assert_eq!(run.code, Some(0), "{}", run.output);
    // The authored stack is shown as written; the event never runs.
    assert!(!run.output.contains("diff code=128"), "{}", run.output);
}
