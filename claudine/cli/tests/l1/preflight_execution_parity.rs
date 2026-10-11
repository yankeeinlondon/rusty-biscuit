//! Level-1 process coverage for pre-flight shell approval and execution
//! agreeing on every command's bytes.
//!
//! Every row composes a document whose shell command interpolates a value and
//! changes only where the command lives and where the value comes from. The
//! approval set is exactly the commands the row whitelists by their full
//! bytes, so a run that succeeds and shows the command's output proves the
//! command that launched was the command pre-flight approved. The command is
//! `git check-ref-format --branch ran/<value>`, which prints `ran/<value>`
//! without a repository.

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

const PART: (&str, &str) = ("part.md", "::shell git check-ref-format --branch ran/{{ base }}\n");

/// The partial of the spec's report: a fact read through a `::shell-block`
/// with a `when_error` fallback.
const FACTS_PART: (&str, &str) = (
    "part.md",
    "::shell-block when_error=\"(unavailable)\"\ngit check-ref-format --branch ran/{{ base }}\n::end-block\n",
);

fn command_for(value: &str) -> String {
    format!("git check-ref-format --branch ran/{value}")
}

struct Run {
    code: Option<i32>,
    output: String,
}

impl Run {
    /// The output with the error box's border glyphs and hard wraps removed,
    /// so a wrapped diagnostic reads as one line.
    fn collapsed(&self) -> String {
        self.output
            .replace('┃', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    write_succeeding_claude(fixture.bin_dir());
    fixture
}

/// Write `files` and a whitelist holding exactly `approved`, then run
/// `claudine <args>`.
fn run(
    fixture: &CliProcessFixture,
    files: &[(&str, &str)],
    approved: &[String],
    args: &[&str],
) -> Run {
    for (name, content) in files {
        write(&fixture.cwd().join(name), content);
    }
    let whitelist: String = approved.iter().map(|command| format!("exact {command}\n")).collect();
    // Outside a repository, a document's shell policy is read from its own
    // directory, so every directory a row writes a document into gets the list.
    let mut policy_dirs: Vec<std::path::PathBuf> = vec![fixture.cwd().to_path_buf()];
    for (name, _) in files {
        let dir = fixture.cwd().join(name).parent().expect("a file has a parent").to_path_buf();
        if !policy_dirs.contains(&dir) {
            policy_dirs.push(dir);
        }
    }
    for dir in policy_dirs {
        write(&dir.join(".darkmatter-shell-whitelist"), &whitelist);
    }
    let output = fixture
        .command_builder()
        // `git` runs the command under test; Git for Windows is outside the
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

struct Row {
    name: &'static str,
    files: &'static [(&'static str, &'static str)],
    args: &'static [&'static str],
    /// The value the command interpolates when it runs.
    value: &'static str,
    /// Whether the command runs (a command in an untaken branch does not).
    executes: bool,
}

const ROUTER_OVERLAY: &str = "---\nstart:\n  stack:\n    - action: {action: proxy, target: './target.md', with: {base: overlay}}\n---\nrouter body\n";
const ROUTER_PLAIN: &str =
    "---\nstart:\n  stack:\n    - action: {action: proxy, target: './target.md'}\n---\nrouter body\n";
const TARGET_REQUIRED: &str = "---\nbase: string(required)\n---\n::file ./part.md\n";

const ROWS: &[Row] = &[
    Row {
        name: "command inline in the target, proxy overlay",
        files: &[
            ("router.md", ROUTER_OVERLAY),
            (
                "target.md",
                "---\nbase: string(required)\n---\n::shell git check-ref-format --branch ran/{{ base }}\n",
            ),
        ],
        args: &["compose", "--claude", "router.md"],
        value: "overlay",
        executes: true,
    },
    Row {
        name: "command in a partial, caller value through the proxy",
        files: &[("router.md", ROUTER_PLAIN), ("target.md", TARGET_REQUIRED), PART],
        args: &["compose", "--claude", "router.md", "base=caller"],
        value: "caller",
        executes: true,
    },
    Row {
        name: "command in a partial, no interpolation",
        files: &[
            ("router.md", ROUTER_OVERLAY),
            ("target.md", TARGET_REQUIRED),
            ("part.md", "::shell git check-ref-format --branch ran/literal\n"),
        ],
        args: &["compose", "--claude", "router.md"],
        value: "literal",
        executes: true,
    },
    Row {
        name: "command in a partial, proxy overlay (the reported row)",
        files: &[("router.md", ROUTER_OVERLAY), ("target.md", TARGET_REQUIRED), FACTS_PART],
        args: &["compose", "--claude", "router.md"],
        value: "overlay",
        executes: true,
    },
    Row {
        name: "command in a partial, target invoked directly with a caller value",
        files: &[("target.md", TARGET_REQUIRED), FACTS_PART],
        args: &["compose", "--claude", "target.md", "base=direct"],
        value: "direct",
        executes: true,
    },
    Row {
        name: "command in a partial, target default",
        files: &[("target.md", "---\nbase: default\n---\n::file ./part.md\n"), FACTS_PART],
        args: &["compose", "--claude", "target.md"],
        value: "default",
        executes: true,
    },
    Row {
        name: "nested conditional partial, value derived in the target",
        files: &[
            (
                "target.md",
                "---\nb: derived\nbase: \"{{ b }}-x\"\nwanted: true\n---\n::file ./mid.md when=\"wanted\"\n",
            ),
            ("mid.md", "::file ./part.md\n"),
            PART,
        ],
        args: &["compose", "--claude", "target.md"],
        value: "derived-x",
        executes: true,
    },
    Row {
        name: "transclusion-local set over the target default",
        files: &[
            ("target.md", "---\nbase: default\n---\n::file ./part.md set.base=\"local\"\n"),
            PART,
        ],
        args: &["compose", "--claude", "target.md"],
        value: "local",
        executes: true,
    },
    Row {
        name: "reserved sequence input",
        files: &[
            ("seq.md", "---\nsequence:\n  - alpha\n---\n::file ./part-state.md\n"),
            ("part-state.md", "::shell git check-ref-format --branch ran/{{ state }}\n"),
        ],
        args: &["sequence", "--claude", "seq.md"],
        value: "alpha",
        executes: true,
    },
    Row {
        name: "sequence prompt task in a subdirectory, document-relative read",
        files: &[
            (
                "seq-task.md",
                "---\nsequence:\n    - name: first\n      prompt: \"./docs/task.md\"\n---\n",
            ),
            ("docs/task.md", "::file ./task-part.md\n"),
            (
                "docs/task-part.md",
                "::shell git check-ref-format --branch ran/{{ file_exists('note.md') }}\n",
            ),
            ("docs/note.md", "note\n"),
        ],
        args: &["sequence", "--claude", "seq-task.md"],
        value: "true",
        executes: true,
    },
    Row {
        name: "untaken branch",
        files: &[
            (
                "target.md",
                "---\nbase: skipped\nwanted: false\n---\ntarget body\n\n::file ./part.md when=\"wanted\"\n",
            ),
            PART,
        ],
        args: &["compose", "--claude", "target.md"],
        value: "skipped",
        executes: false,
    },
];

fn assert_row_launches_approved_bytes(row: &Row) {
    let fixture = fixture("preflight-parity");
    let result = run(&fixture, row.files, &[command_for(row.value)], row.args);
    assert_eq!(result.code, Some(0), "{}: {}", row.name, result.output);
    assert_eq!(
        result.output.contains(&format!("ran/{}", row.value)),
        row.executes,
        "{}: {}",
        row.name,
        result.output
    );
    assert!(
        !result.output.contains("pre-approved") && !result.output.contains("Could not transclude"),
        "{}: {}",
        row.name,
        result.output
    );
}

/// One test per `ROWS` entry, so each process spawn runs in parallel under
/// nextest instead of serially in one test.
macro_rules! row_tests {
    ($($test:ident => $index:expr,)*) => {
        $(
            #[test]
            fn $test() {
                assert_row_launches_approved_bytes(&ROWS[$index]);
            }
        )*
    };
}

row_tests! {
    every_launched_command_is_approved_inline_proxy_overlay => 0,
    every_launched_command_is_approved_partial_caller_value => 1,
    every_launched_command_is_approved_partial_no_interpolation => 2,
    every_launched_command_is_approved_partial_reported_row => 3,
    every_launched_command_is_approved_partial_direct_target => 4,
    every_launched_command_is_approved_partial_target_default => 5,
    every_launched_command_is_approved_nested_conditional_partial => 6,
    every_launched_command_is_approved_transclusion_local_set => 7,
    every_launched_command_is_approved_reserved_sequence_input => 8,
    every_launched_command_is_approved_sequence_task_subdirectory => 9,
    every_launched_command_is_approved_untaken_branch => 10,
}

#[test]
fn every_launched_command_rows_are_all_tested() {
    assert_eq!(ROWS.len(), 11, "add a row_tests! entry for each ROWS row");
}

/// Discovery is condition-blind: the untaken branch's command still needs
/// approval, which is how the row above proves it was discovered.
#[test]
fn an_untaken_branch_is_discovered_without_executing() {
    let fixture = fixture("preflight-parity-untaken");
    let untaken = ROWS
        .iter()
        .find(|row| !row.executes)
        .expect("the table carries an untaken-branch row");
    let result = run(&fixture, untaken.files, &[], untaken.args);
    assert_ne!(result.code, Some(0), "{}", result.output);
    assert!(result.collapsed().contains("requires approval"), "{}", result.output);
    assert!(result.collapsed().contains("ran/skipped"), "{}", result.output);
}

/// An approval covers the bytes it was granted for. A loop iteration composes
/// a changed command from its own state, so the command is audited again, and
/// refused non-interactively when only the first iteration's bytes are
/// approved — for a body command and for a lifecycle `shell` action alike.
const LOOP_BODY: &str = "---\nn: 0\nloop:\n    while: \"n < 2\"\n    action: \"increment(n)\"\n---\n::file ./loop-part.md\n";
const LOOP_LIFECYCLE: &str = "---\nn: 0\nloop:\n    while: \"n < 2\"\n    action: \"increment(n)\"\nsuccess:\n  stack:\n    - action: {action: shell, command: \"git check-ref-format --branch ran/{{ n }}\"}\n---\nbody\n";

fn loop_files(document: &'static str) -> [(&'static str, &'static str); 2] {
    [("loop.md", document), ("loop-part.md", "::shell git check-ref-format --branch ran/{{ n }}\n")]
}

#[test]
fn a_loop_iteration_audits_the_bytes_its_own_state_produces() {
    let every = [command_for("0"), command_for("1"), command_for("2")];

    let accepted = fixture("preflight-parity-loop");
    let result = run(&accepted, &loop_files(LOOP_BODY), &every, &["compose", "--claude", "loop.md"]);
    assert_eq!(result.code, Some(0), "{}", result.output);
    for value in ["ran/0", "ran/1", "ran/2"] {
        assert!(result.output.contains(value), "{value}: {}", result.output);
    }
}

fn assert_loop_refused_beyond_first_iteration(name: &str, document: &'static str) {
    let refused = fixture("preflight-parity-loop-refused");
    let result = run(&refused, &loop_files(document), &[command_for("0")], &["compose", "--claude", "loop.md"]);
    assert_ne!(result.code, Some(0), "{name}: {}", result.output);
    assert!(result.collapsed().contains("requires approval"), "{name}: {}", result.output);
    assert!(result.collapsed().contains("ran/1"), "{name}: {}", result.output);
    assert!(!result.output.contains("pre-approved"), "{name}: {}", result.output);
}

#[test]
fn a_loop_body_command_is_refused_when_only_the_first_iteration_is_approved() {
    assert_loop_refused_beyond_first_iteration("body", LOOP_BODY);
}

#[test]
fn a_loop_lifecycle_shell_is_refused_when_only_the_first_iteration_is_approved() {
    assert_loop_refused_beyond_first_iteration("lifecycle", LOOP_LIFECYCLE);
}

/// A command whose bytes discovery cannot know is a hard preparation failure.
/// The partial's `when_error` covers a command that ran and failed, and the
/// lifecycle's `no_error` covers a failed action; neither turns this into a
/// warning, a notice in place of the partial, or a missing fact.
#[test]
fn an_unknowable_command_fails_preparation_despite_when_error_and_no_error() {
    let fixture = fixture("preflight-parity-hard");
    let result = run(
        &fixture,
        &[
            (
                "target.md",
                "---\nflag: \"{{ has_alias('ll') }}\"\nfailure:\n  stack:\n    - action: {info: 'failure fired'}\n      no_error: true\nfinalize:\n  stack:\n    - action: {info: 'finalize fired'}\n      no_error: true\n---\n::file ./part.md\n",
            ),
            (
                "part.md",
                "::shell-block when_error=\"(unavailable)\"\ngit check-ref-format --branch ran/{{ flag }}\n::end-block\n",
            ),
        ],
        &[command_for("false"), command_for("true")],
        &["compose", "--claude", "target.md"],
    );
    assert_ne!(result.code, Some(0), "{}", result.output);
    assert!(result.collapsed().contains("does not observe"), "{}", result.output);
    assert!(!result.output.contains("Could not transclude"), "{}", result.output);
    assert!(
        !result.output.contains("ran/false") && !result.output.contains("ran/true"),
        "{}",
        result.output
    );
}

/// Lifecycle `no_error` covers a shell action that ran and failed. A command
/// whose bytes were never approved did not run, so `no_error` must not turn
/// the refusal into a skipped action: iteration 2 re-stamps the command with
/// its own state and the run still fails. `git init` leaves a directory behind
/// when it runs, which is how the row proves the refused bytes never launched.
#[test]
fn a_lifecycle_no_error_shell_does_not_absorb_an_unapproved_command() {
    const LIFECYCLE: &str = "---\nn: 0\nloop:\n    while: \"n < 2\"\n    action: \"increment(n)\"\nsuccess:\n  stack:\n    - action: {action: shell, command: \"git init --quiet made-{{ n }}\", no_error: true}\n---\nbody\n";
    let fixture = fixture("preflight-parity-no-error");
    let result = run(
        &fixture,
        &[("loop.md", LIFECYCLE)],
        &["git init --quiet made-0".to_string()],
        &["compose", "--claude", "loop.md"],
    );

    assert!(
        fixture.cwd().join("made-0").is_dir(),
        "fixture check: iteration 1's approved command runs: {}",
        result.output
    );
    assert!(!fixture.cwd().join("made-1").exists(), "{}", result.output);
    assert_ne!(result.code, Some(0), "{}", result.output);
    let collapsed = result.collapsed();
    assert!(collapsed.contains("requires approval"), "{}", result.output);
    // The terminal may break the command after its hyphen, so the refused
    // bytes are matched with every space removed.
    assert!(collapsed.replace(' ', "").contains("made-1"), "{}", result.output);
}
