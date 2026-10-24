//! `md compose` as Darkmatter's core-only lifecycle host, through the binary.
//!
//! Each document marks its events with `info` lines naming the event, so the
//! order of those lines on stderr is the order the events ran. The engine's
//! routing (every outcome, cancellation at each step, late replies) is
//! covered in-process by `darkmatter::lifecycle::md_host::tests`; these tests
//! prove the invocation path: parsing, the terminal channels, the shell
//! approval, composition, the document write, and exit statuses.

use crate::common;

use common::CliProcessFixture;
use std::path::Path;

/// The run's event markers, in the order they reached stderr.
fn events(stderr: &str) -> Vec<&str> {
    stderr
        .lines()
        .filter_map(|line| line.split("event:").nth(1))
        .map(|rest| rest.split_whitespace().next().unwrap_or_default())
        .collect()
}

struct Ran {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

fn compose(fixture: &CliProcessFixture, document: &Path, extra: &[&str]) -> Ran {
    let output = fixture
        .command_builder()
        .host_path()
        .build()
        .arg("compose")
        .arg(document)
        .args(extra)
        .output()
        .expect("md runs");
    Ran {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// A stack for each of `md`'s events that only announces itself.
const EVERY_EVENT: &str = "\
initialize:
  - info: \"event:initialize\"
composed:
  - info: \"event:composed\"
failed:
  - info: \"event:failed {{ err.msg }}\"
cancelled:
  - info: \"event:cancelled\"
finalize:
  - info: \"event:finalize\"
";

#[test]
fn a_successful_run_fires_initialize_composed_then_finalize() {
    let fixture = CliProcessFixture::named("lifecycle-success");
    let doc = fixture.write_file("cwd/doc.md", &format!("---\ntitle: Report\n{EVERY_EVENT}---\n# {{{{ title }}}}\n"));
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert_eq!(events(&ran.stderr), ["initialize", "composed", "finalize"]);
    assert!(ran.stdout.contains("# Report"), "{}", ran.stdout);
}

#[test]
fn a_shell_audit_denial_runs_failed_then_finalize() {
    // A lifecycle-only command still needs approval, and a non-interactive
    // run with nothing whitelisted refuses it before any shell runs.
    let fixture = CliProcessFixture::named("lifecycle-denial");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\ninitialize:\n  - info: \"event:initialize\"\ncomposed:\n  - shell: \"echo lifecycle-only\"\n\
         failed:\n  - info: \"event:failed\"\nfinalize:\n  - info: \"event:finalize\"\n---\nbody\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(1), "{}", ran.stderr);
    assert_eq!(events(&ran.stderr), ["initialize", "failed", "finalize"], "{}", ran.stderr);
    assert!(ran.stderr.contains("Approval required for 'echo lifecycle-only'"), "{}", ran.stderr);
    assert!(!ran.stdout.contains("lifecycle-only"), "the refused command never ran");
    assert!(!ran.stdout.contains("body"), "no document is written");
}

#[test]
fn a_missing_include_runs_failed_then_finalize() {
    let fixture = CliProcessFixture::named("lifecycle-missing-include");
    let doc = fixture.write_file("cwd/doc.md", &format!("---\n{EVERY_EVENT}---\n::file ./missing.md\n"));
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(2), "{}", ran.stderr);
    assert_eq!(events(&ran.stderr), ["initialize", "failed", "finalize"], "{}", ran.stderr);
    assert!(ran.stderr.contains("missing.md"), "{}", ran.stderr);
}

#[test]
fn skip_writes_no_document_runs_no_finalize_and_exits_zero() {
    let fixture = CliProcessFixture::named("lifecycle-skip");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\ninitialize:\n  - info: \"event:initialize\"\n  - skip\nfinalize:\n  - info: \"event:finalize\"\n---\nbody\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert_eq!(events(&ran.stderr), ["initialize"]);
    assert!(ran.stdout.trim().is_empty(), "{}", ran.stdout);
}

#[test]
fn stdout_actions_frame_the_document_and_other_verbs_use_stderr() {
    let fixture = CliProcessFixture::named("lifecycle-stdout-order");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\ncomposed:\n  - stdout: \"before-document\"\n  - warn: \"warn-line\"\n  - message: \"message-line\"\n\
         finalize:\n  - stdout: \"after-document\"\n  - success: \"success-line\"\n  - stderr: \"stderr-line\"\n---\n\
         the-document\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    let order: Vec<&str> = ran.stdout.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    assert_eq!(order, ["before-document", "the-document", "after-document"]);
    for line in ["warn-line", "message-line", "success-line", "stderr-line"] {
        assert!(ran.stderr.contains(line), "{line} on stderr: {}", ran.stderr);
        assert!(!ran.stdout.contains(line), "{line} not on stdout");
    }
}

#[test]
fn a_piped_stdout_action_writes_its_text_as_authored() {
    let fixture = CliProcessFixture::named("lifecycle-stdout-piped");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\ncomposed:\n  - stdout: \"hello **bold** <!-- note -->\"\n---\nthe-document\n",
    );
    // A capable, colored terminal environment: only the pipe turns styling off.
    let output = fixture
        .command_builder()
        .host_path()
        .rendering_input_removed("NO_COLOR")
        .rendering_input("FORCE_COLOR", "1")
        .rendering_input("TERM", "xterm-256color")
        .build()
        .arg("compose")
        .arg(&doc)
        .output()
        .expect("md runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(!stdout.contains('\u{1b}'), "no escape sequence on a pipe: {stdout:?}");
    let lines: Vec<&str> = stdout.lines().filter(|line| !line.trim().is_empty()).collect();
    assert_eq!(lines, ["hello **bold** <!-- note -->", "the-document"], "{stdout:?}");
}

/// `document` names one action `md` does not run, between two that it does.
fn assert_skipped_once(name: &str, action: &str, verb: &str) {
    let fixture = CliProcessFixture::named(name);
    let doc = fixture.write_file(
        "cwd/doc.md",
        &format!("---\ncomposed:\n  - info: \"event:composed\"\n  - {action}\n  - info: \"after-skipped\"\n---\nbody\n"),
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(0), "the exit is unaffected: {}", ran.stderr);
    let warning = format!("does not run {verb}; skipped it in composed");
    assert_eq!(common::unwrapped(&ran.stderr).replace('`', "").matches(&warning).count(), 1, "{}", ran.stderr);
    assert!(ran.stderr.contains("after-skipped"), "the rest of the stack runs: {}", ran.stderr);
    assert!(ran.stdout.contains("body"));
}

#[test]
fn say_is_skipped_with_one_warning_naming_it_and_the_event() {
    assert_skipped_once("lifecycle-say", "say: \"hello\"", "say");
}

#[test]
fn retry_is_skipped_with_one_warning_naming_it_and_the_event() {
    assert_skipped_once("lifecycle-retry", "retry: 3", "retry");
}

#[test]
fn an_unknown_verb_is_skipped_with_one_warning_naming_it_and_the_event() {
    assert_skipped_once("lifecycle-unknown", "frobnicate: { any: shape }", "frobnicate");
}

#[test]
fn an_unselected_branch_warns_nothing() {
    let fixture = CliProcessFixture::named("lifecycle-unselected");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\ncomposed:\n  - when: \"false\"\n    then:\n      - say: \"never\"\n    else:\n      - info: \"else-ran\"\n---\nbody\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert!(ran.stderr.contains("else-ran"));
    assert!(!ran.stderr.contains("does not run"), "{}", ran.stderr);
}

#[test]
fn host_only_keys_are_plain_frontmatter() {
    let fixture = CliProcessFixture::named("lifecycle-host-only-keys");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\nstart: start-value\nsuccess: success-value\nfinalize:\n  - info: \"event:finalize\"\n---\n\
         {{ start }} {{ success }}\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert!(ran.stdout.contains("start-value success-value"), "{}", ran.stdout);
    assert_eq!(events(&ran.stderr), ["finalize"]);
}

#[test]
fn report_only_shell_mode_lists_lifecycle_commands_and_runs_nothing() {
    let fixture = CliProcessFixture::named("lifecycle-report-only");
    let marker = fixture.cwd().join("ran-marker");
    let doc = fixture.write_file(
        "cwd/doc.md",
        &format!("---\n{EVERY_EVENT}  - shell: \"touch ran-marker\"\n---\nbody\n"),
    );
    let ran = compose(&fixture, &doc, &["--shell"]);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert!(ran.stdout.contains("touch ran-marker"), "{}", ran.stdout);
    assert!(events(&ran.stderr).is_empty(), "no stack fires: {}", ran.stderr);
    assert!(!marker.exists());
}

#[test]
fn a_composed_error_runs_finalize_without_failed_and_exits_nonzero() {
    let fixture = CliProcessFixture::named("lifecycle-composed-error");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\ncomposed:\n  - info: \"event:composed\"\n  - error: \"refused-here\"\nfailed:\n  - info: \"event:failed\"\n\
         finalize:\n  - info: \"event:finalize {{ err.msg }}\"\n---\nbody\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(1), "{}", ran.stderr);
    assert_eq!(events(&ran.stderr), ["composed", "finalize"], "{}", ran.stderr);
    assert!(ran.stderr.contains("refused-here"));
    assert!(!ran.stdout.contains("body"), "{}", ran.stdout);
}

#[test]
fn a_bare_error_in_composed_names_the_md_event() {
    let fixture = CliProcessFixture::named("lifecycle-bare-error");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\ncomposed:\n  - error\nfinalize:\n  - info: \"event:finalize {{ err.msg }}\"\n---\nbody\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(1), "{}", ran.stderr);
    assert!(ran.stderr.contains("lifecycle composed error"), "{}", ran.stderr);
    assert!(!ran.stderr.contains("lifecycle start error"), "{}", ran.stderr);
}

#[test]
fn a_shell_reached_before_preflight_is_refused_naming_the_md_event_where_shells_unlock() {
    let fixture = CliProcessFixture::named("lifecycle-locked-shell");
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "exact touch locked-marker\n");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\nfailed:\n  - info: \"event:failed\"\n  - shell: \"touch locked-marker\"\n\
         finalize:\n  - info: \"event:finalize {{ err.msg }}\"\n---\n::file ./missing.md\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_ne!(ran.status, Some(0), "{}", ran.stderr);
    assert_eq!(events(&ran.stderr), ["failed", "finalize"], "{}", ran.stderr);
    assert!(
        ran.stderr.contains("move the command to composed or a later event"),
        "{}",
        ran.stderr
    );
    assert!(!ran.stderr.contains("move the command to start"), "{}", ran.stderr);
    assert!(!fixture.cwd().join("locked-marker").exists(), "the locked shell never ran");
}

#[test]
fn stop_ends_only_its_stack_and_keeps_the_document_and_finalize() {
    let fixture = CliProcessFixture::named("lifecycle-stop");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\ncomposed:\n  - info: \"event:composed\"\n  - stop\nfinalize:\n  - info: \"event:finalize\"\n---\nbody\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert_eq!(events(&ran.stderr), ["composed", "finalize"]);
    assert!(ran.stdout.contains("body"));
}

#[test]
fn initialize_can_create_the_file_the_body_includes() {
    let fixture = CliProcessFixture::named("lifecycle-initialize-include");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\ninitialize:\n  - ensure_file: [\"part.md\", \"created-by-initialize\"]\n---\n::file ./part.md\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert!(ran.stdout.contains("created-by-initialize"), "{}", ran.stdout);
}

#[test]
fn an_initialize_set_reaches_the_composition() {
    let fixture = CliProcessFixture::named("lifecycle-initialize-set");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\nstamp: authored\ninitialize:\n  - set: { stamp: \"from-initialize\" }\n---\nstamp={{ stamp }}\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert!(ran.stdout.contains("stamp=from-initialize"), "{}", ran.stdout);
}

/// The document fails to parse its lifecycle, so nothing in it ran: neither
/// the `ensure_file` in `initialize` nor any event.
fn assert_refused_before_any_effect(name: &str, stack: &str, expected: &str) {
    let fixture = CliProcessFixture::named(name);
    let marker = fixture.cwd().join("marker.md");
    let doc = fixture.write_file(
        "cwd/doc.md",
        &format!("---\ninitialize:\n  - ensure_file: [\"marker.md\", \"x\"]\n  - info: \"event:initialize\"\n{stack}---\nbody\n"),
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(1), "{}", ran.stderr);
    assert!(common::unwrapped(&ran.stderr).contains(expected), "{}", ran.stderr);
    assert!(events(&ran.stderr).is_empty(), "{}", ran.stderr);
    assert!(!marker.exists(), "no effect ran");
    assert!(!ran.stdout.contains("body"));
}

#[test]
fn a_malformed_provided_action_fails_before_any_effect() {
    assert_refused_before_any_effect(
        "lifecycle-malformed",
        "composed:\n  - info: { bogus: x }\n",
        "`info` requires a `text` parameter",
    );
}

#[test]
fn shell_in_initialize_fails_before_any_effect() {
    assert_refused_before_any_effect("lifecycle-initialize-shell", "  - shell: \"echo no\"\n", "initialize");
}

#[test]
fn skip_in_composed_fails_before_any_effect() {
    assert_refused_before_any_effect("lifecycle-composed-skip", "composed:\n  - skip\n", "skip");
}

#[test]
fn malformed_lifecycle_yaml_shape_fails_before_any_effect() {
    assert_refused_before_any_effect(
        "lifecycle-malformed-when",
        "composed:\n  - then: [{ info: x }]\n",
        "`then` requires `when`",
    );
}

#[test]
fn an_ignored_actions_operand_and_payload_cause_no_effect() {
    let fixture = CliProcessFixture::named("lifecycle-inert");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\ncomposed:\n  - say: \"$(touch say-marker)\"\n  - notify: { title: \"{{ missing.deep }}\", payload: { shell: \"touch notify-marker\" } }\n\
         \x20 - info: \"after-ignored\"\n---\nbody\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert!(ran.stderr.contains("after-ignored"));
    assert!(!ran.stderr.contains("Approval required"), "an ignored payload is never audited: {}", ran.stderr);
    assert!(!fixture.cwd().join("say-marker").exists());
    assert!(!fixture.cwd().join("notify-marker").exists());
}

#[test]
fn an_approved_lifecycle_shell_runs_once_shells_unlock_at_composed() {
    let fixture = CliProcessFixture::named("lifecycle-approved-shell");
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "exact echo lifecycle-said\n");
    let doc = fixture.write_file("cwd/doc.md", "---\ncomposed:\n  - shell: \"echo lifecycle-said\"\n---\nthe-body\n");
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    let order: Vec<&str> = ran.stdout.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    assert_eq!(order, ["lifecycle-said", "the-body"]);
}

#[test]
fn a_childs_lifecycle_keys_never_fire() {
    let fixture = CliProcessFixture::named("lifecycle-child");
    fixture.write_file(
        "cwd/child.md",
        "---\ninitialize:\n  - info: \"event:child-initialize\"\ncomposed:\n  - info: \"event:child-composed\"\n---\nchild-body\n",
    );
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\ncomposed:\n  - info: \"event:composed\"\n---\n::file ./child.md\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert_eq!(events(&ran.stderr), ["composed"], "{}", ran.stderr);
    assert!(ran.stdout.contains("child-body"));
}

#[test]
fn an_unreadable_root_fails_before_any_effect() {
    let fixture = CliProcessFixture::named("lifecycle-unreadable");
    let doc = fixture.cwd().join("doc.md");
    std::fs::write(&doc, b"---\nfinalize:\n  - info: \"event:finalize\"\n---\n\xff\xfe not utf-8\n").unwrap();
    let ran = compose(&fixture, &doc, &[]);
    assert_ne!(ran.status, Some(0), "{}", ran.stderr);
    assert!(events(&ran.stderr).is_empty(), "{}", ran.stderr);
}

#[test]
fn an_error_in_failed_replaces_the_error_without_running_failed_again() {
    let fixture = CliProcessFixture::named("lifecycle-cleanup-error");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\nfailed:\n  - info: \"event:failed\"\n  - error: \"cleanup-broke\"\nfinalize:\n  - info: \"event:finalize {{ err.msg }}\"\n\
         ---\n::file ./missing.md\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(1), "{}", ran.stderr);
    assert_eq!(events(&ran.stderr), ["failed", "finalize"], "{}", ran.stderr);
}

#[test]
fn a_closed_stdout_runs_finalize_and_exits_nonzero_without_failed() {
    let fixture = CliProcessFixture::named("lifecycle-closed-stdout");
    let doc = fixture.write_file(
        "cwd/doc.md",
        &format!("---\n{EVERY_EVENT}---\n{}\n", "document line\n".repeat(64)),
    );
    let mut child = fixture
        .command_builder()
        .build_std()
        .arg("compose")
        .arg(&doc)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("md starts");
    drop(child.stdout.take());
    let output = child.wait_with_output().expect("md ends");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert_eq!(events(&stderr), ["initialize", "composed", "finalize"], "{stderr}");
    assert!(stderr.contains("failed to write the composed document"), "{stderr}");
}

/// The one real-signal test: Ctrl-C reaches `md`'s handler while `composed`
/// runs a command, so the rest of `composed` and the document are dropped,
/// and `cancelled` then `finalize` run before the interrupted exit. Every
/// other cancellation path is covered in-process with a fake interrupt.
/// Unix only: it delivers `SIGINT` to a process group with `kill(2)`.
#[cfg(unix)]
#[test]
fn ctrl_c_runs_cancelled_then_finalize_and_exits_130() {
    use std::io::{BufRead, BufReader, Read};
    use std::os::unix::process::CommandExt as _;

    let fixture = CliProcessFixture::named("lifecycle-ctrl-c");
    // The command announces itself only once it is the running program, so
    // the interrupt cannot land before it starts.
    let started = fixture.cwd().join("started");
    common::write_executable(
        &fixture.bin_dir().join("slow-step"),
        &format!("#!/bin/sh\ntouch '{}'\nexec sleep 5\n", started.display()),
    );
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "exact slow-step\n");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\ncomposed:\n  - info: \"event:composed\"\n  - shell: \"slow-step\"\n  - info: \"event:after-shell\"\n\
         cancelled:\n  - info: \"event:cancelled\"\nfinalize:\n  - info: \"event:finalize\"\n---\nbody\n",
    );
    let mut child = fixture
        .command_builder()
        .host_path()
        .build_std()
        .arg("compose")
        .arg(&doc)
        // Its own process group, so the interrupt reaches `md` and its
        // command together, as a terminal's Ctrl-C does.
        .process_group(0)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("md starts");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    let mut seen = String::new();
    while !seen.contains("event:composed") {
        let mut line = String::new();
        assert_ne!(stderr.read_line(&mut line).unwrap(), 0, "md ended before composed: {seen}");
        seen.push_str(&line);
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(4);
    while !started.exists() {
        assert!(std::time::Instant::now() < deadline, "the command never started: {seen}");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    // SAFETY: `kill` of the child's own process group with SIGINT has no
    // memory effects.
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGINT);
    }
    stderr.read_to_string(&mut seen).unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(130), "{seen}");
    assert_eq!(events(&seen), ["composed", "cancelled", "finalize"], "{seen}");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("body"));
}

/// A lifecycle call the design-time check proves wrong is rejected before any
/// event runs, with the verdict the same call gets in the body, and the
/// document is not written.
#[test]
fn a_provably_wrong_lifecycle_call_rejects_before_any_event() {
    let fixture = CliProcessFixture::named("lifecycle-call-check");
    let doc = fixture.write_file(
        "cwd/doc.md",
        "---\n$schema:\n  flag: boolean(required)\nflag: true\ninitialize:\n  - info: \"event:initialize\"\ncomposed:\n  - when: min(flag, 5) > 1\n    then:\n      - stdout: hi\n---\nbody\n",
    );
    let ran = compose(&fixture, &doc, &[]);
    assert_eq!(ran.status, Some(1), "{}", ran.stderr);
    assert!(events(&ran.stderr).is_empty(), "no event runs: {}", ran.stderr);
    assert!(ran.stdout.is_empty(), "no document is written: {}", ran.stdout);
    assert!(ran.stderr.contains("composed[0].when"), "{}", ran.stderr);
    assert!(
        ran.stderr.contains("min: argument 1 (`a`) is boolean, expected number (category 5: no value of that type can bind)"),
        "{}",
        ran.stderr
    );
}
