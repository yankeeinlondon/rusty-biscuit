//! `ShellExpansionError` variant snapshots.

use std::path::PathBuf;
use std::time::Duration;

use darkmatter::markdown::compose::{ShellCommandOrigin, ShellExpansionError};

use crate::helpers::{assert_contains_all, render, test_ctx};

fn body_origin() -> ShellCommandOrigin {
    ShellCommandOrigin::Body { line: 17 }
}

fn frontmatter_origin() -> ShellCommandOrigin {
    ShellCommandOrigin::Frontmatter {
        key: "today".into(),
        line: None,
    }
}

#[test]
fn parse_directive_shows_origin_and_syntax_hint() {
    let err = ShellExpansionError::ParseDirective {
        ctx: Box::new(test_ctx("", "doc.md")),
        origin: body_origin(),
        message: "unterminated quote".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ShellExpansionError",
            "directive parse failed",
            "line 17",
            "unterminated quote",
            "::shell",
        ],
    );
}

#[test]
fn command_not_found_has_path_hint() {
    let err = ShellExpansionError::CommandNotFound {
        command: "nx".into(),
        origin: body_origin(),
        ctx: Box::new(test_ctx("", "doc.md")),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ShellExpansionError",
            "command not found",
            "nx",
            "line 17",
            "PATH",
        ],
    );
}

#[test]
fn blacklisted_shows_reason() {
    let err = ShellExpansionError::Blacklisted {
        command: "rm -rf /".into(),
        reason: "destructive".into(),
        origin: body_origin(),
        ctx: Box::new(test_ctx("", "doc.md")),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ShellExpansionError",
            "command blacklisted",
            "rm -rf /",
            "destructive",
        ],
    );
}

#[test]
fn approval_required_names_whitelist_paths() {
    let err = ShellExpansionError::ApprovalRequired {
        command: "gh repo list".into(),
        whitelist_path: Box::new(PathBuf::from("/tmp/wl")),
        blacklist_path: Box::new(PathBuf::from("/tmp/bl")),
        origin: body_origin(),
        ctx: Box::new(test_ctx("", "doc.md")),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ShellExpansionError",
            "approval required",
            "gh repo list",
            "/tmp/wl",
            "/tmp/bl",
            "--approve-shell",
        ],
    );
}

#[test]
fn denied_shows_command_and_origin() {
    let err = ShellExpansionError::Denied {
        command: "git push".into(),
        origin: frontmatter_origin(),
        ctx: Box::new(test_ctx("", "doc.md")),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ShellExpansionError",
            "denied",
            "git push",
            "frontmatter.today",
        ],
    );
}

#[test]
fn not_pre_approved_surfaces_command() {
    let err = ShellExpansionError::NotPreApproved {
        ctx: Box::new(test_ctx("", "doc.md")),
        command: "pnpm run build".into(),
        origin: body_origin(),
        source_desc: " (from whitelist)".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["ShellExpansionError", "not pre-approved", "pnpm run build"],
    );
}

#[test]
fn timeout_includes_duration() {
    let err = ShellExpansionError::Timeout {
        command: "sleep 30".into(),
        timeout: Duration::from_secs(5),
        origin: body_origin(),
        ctx: Box::new(test_ctx("", "doc.md")),
    };
    let out = render(&err);
    assert_contains_all(&out, &["ShellExpansionError", "timed out", "sleep 30"]);
}

#[test]
fn execution_failed_includes_stderr() {
    let err = ShellExpansionError::ExecutionFailed {
        command: "ls --bogus".into(),
        code: 2,
        stdout: String::new(),
        stderr: "ls: unrecognized option".into(),
        origin: body_origin(),
        ctx: Box::new(test_ctx("", "doc.md")),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ShellExpansionError",
            "execution failed",
            "ls --bogus",
            "Exit code:",
            "ls: unrecognized option",
        ],
    );
}

#[test]
fn policy_io_includes_path_and_kind() {
    let err = ShellExpansionError::PolicyIo {
        path: PathBuf::from("/etc/shell.policy"),
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ShellExpansionError",
            "/etc/shell.policy",
            "PermissionDenied",
        ],
    );
}
