//! A root document Claudine cannot read, or whose frontmatter or lifecycle is
//! malformed, fails before any lifecycle effect
//! (`2026-10-27-darkmatter-lifecycle-ownership`, acceptance 21; spec "Run
//! preparation and context lifetime"): no stack runs, the well-formed cleanup
//! stacks beside a malformed one included, and no provider launches.

use crate::common;

use common::{CliProcessFixture, strip_ansi, write};

/// Every lifecycle stack a run could reach before or instead of a launch,
/// each appending its name to `events.log` (resolved against the repository
/// root, which is the fixture's working directory).
const STACKS: &str = "initialize:\n    - {append_line: ['events.log', 'initialize']}\n\
                      blocked:\n    - {append_line: ['events.log', 'blocked']}\n\
                      failure:\n    - {append_line: ['events.log', 'failure']}\n\
                      finalize:\n    - {append_line: ['events.log', 'finalize']}\n";

/// Runs `claudine compose <root> --claude` and asserts it failed with no
/// lifecycle effect and no launch. Returns stderr.
fn assert_fails_before_effects(name: &str, stage: impl FnOnce(&CliProcessFixture) -> String) -> String {
    let fixture = CliProcessFixture::named(name);
    fixture.initialize_repository();
    let records = common::launch_recorder::install(&fixture, "claude", "exit 0");
    let root = stage(&fixture);

    let output = fixture.command().args(["compose", &root, "--claude"]).output().unwrap();

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(common::launch_recorder::launches(&records).is_empty(), "a provider launched:\n{stderr}");
    let events = std::fs::read_to_string(fixture.cwd().join("events.log")).unwrap_or_default();
    assert_eq!(events, "", "a lifecycle stack ran:\n{stderr}");
    stderr
}

/// The stacks [`assert_fails_before_effects`] looks for do run from a
/// well-formed root, so their absence there is evidence.
#[test]
fn the_same_stacks_run_from_a_well_formed_root() {
    let fixture = CliProcessFixture::named("root-preparation-control");
    fixture.initialize_repository();
    let records = common::launch_recorder::install(&fixture, "claude", "exit 0");
    write(&fixture.cwd().join("doc.md"), &format!("---\n{STACKS}---\nBody.\n"));

    let output = fixture.command().args(["compose", "doc.md", "--claude"]).output().unwrap();

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "{stderr}");
    assert_eq!(common::launch_recorder::launches(&records).len(), 1, "{stderr}");
    let events = std::fs::read_to_string(fixture.cwd().join("events.log")).unwrap_or_default();
    assert_eq!(events, "initialize\nfinalize\n", "{stderr}");
}

#[test]
fn a_root_without_read_permission_runs_nothing() {
    use std::os::unix::fs::PermissionsExt as _;

    let stderr = assert_fails_before_effects("root-preparation-unreadable", |fixture| {
        let root = fixture.cwd().join("doc.md");
        write(&root, &format!("---\n{STACKS}---\nBody.\n"));
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o000)).unwrap();
        // Root ignores file modes; this test needs an unprivileged runner.
        assert!(std::fs::read(&root).is_err(), "the root stayed readable after chmod 000");
        "doc.md".into()
    });
    // The error box wraps at a width the temp path's length decides, so its
    // continuation lines are joined before matching.
    let joined = stderr.split('\n').map(|line| line.trim_start_matches(['┃', ' '])).collect::<Vec<_>>().join(" ");
    assert!(joined.contains("failed to load Markdown") && joined.contains("Permission denied"), "{stderr}");
}

#[test]
fn a_root_that_is_not_valid_utf8_runs_no_stack() {
    let stderr = assert_fails_before_effects("root-preparation-not-utf8", |fixture| {
        let mut bytes = format!("---\n{STACKS}---\nBody ").into_bytes();
        bytes.extend_from_slice(&[0xff, 0xfe, b'\n']);
        std::fs::write(fixture.cwd().join("doc.md"), bytes).unwrap();
        "doc.md".into()
    });
    assert!(stderr.contains("failed to load Markdown") && stderr.contains("valid UTF-8"), "{stderr}");
}

#[test]
fn malformed_frontmatter_runs_none_of_its_stacks() {
    let stderr = assert_fails_before_effects("root-preparation-bad-yaml", |fixture| {
        write(&fixture.cwd().join("doc.md"), &format!("---\n{STACKS}title: [unclosed\n---\nBody.\n"));
        "doc.md".into()
    });
    assert!(stderr.contains("frontmatter parse failed"), "{stderr}");
}

#[test]
fn a_malformed_stack_keeps_the_well_formed_cleanup_stacks_beside_it_from_running() {
    let stderr = assert_fails_before_effects("root-preparation-bad-stack", |fixture| {
        write(&fixture.cwd().join("doc.md"), &format!("---\n{STACKS}start: \"not a list\"\n---\nBody.\n"));
        "doc.md".into()
    });
    assert!(stderr.contains("invalid lifecycle item") && stderr.contains("`start`"), "{stderr}");
}
