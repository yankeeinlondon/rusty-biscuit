//! Level-1 process coverage for the missing-value rule: `claudine compose`
//! refuses, at pre-flight and before anything runs, a document whose
//! `::shell`, frontmatter `$( … )`, lifecycle `shell`, or lifecycle `set`
//! command reads a `null` or absent value; an empty string is present and runs.
//!
//! The marker command is `git init --quiet ran`, which needs no repository and
//! behaves the same on every host: the `ran/.git` directory proves it ran.

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

struct Run {
    code: Option<i32>,
    output: String,
    ran: bool,
}

/// Composes `document` as `doc.md` with `git` and `ls` approved by prefix.
fn compose(name: &str, document: &str) -> Run {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    write_succeeding_claude(fixture.bin_dir());
    write(&fixture.cwd().join("doc.md"), document);
    write(&fixture.cwd().join(".darkmatter-shell-whitelist"), "prefix git\nprefix ls\n");
    let output = fixture
        .command_builder()
        // `git` is the marker command; Git for Windows is outside the minimal
        // system set.
        .host_path()
        .build()
        .args(["compose", "--claude", "doc.md"])
        .output()
        .expect("claudine runs");
    let output_text = common::strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ));
    Run {
        code: output.status.code(),
        output: output_text.replace(['┃', '│'], " ").split_whitespace().collect::<Vec<_>>().join(" "),
        ran: fixture.cwd().join("ran").join(".git").exists(),
    }
}

#[test]
fn a_missing_value_fails_pre_flight_on_every_shell_surface() {
    for (row, document, command) in [
        (
            "body",
            "---\ntarget: null\nstart:\n  - shell: \"git init --quiet ran\"\n---\n::shell ls {{ target }}\n",
            "ls {{ target }}",
        ),
        (
            "frontmatter",
            "---\nlisting: $(ls {{ target }})\nstart:\n  - shell: \"git init --quiet ran\"\n---\nBody.\n",
            "ls {{ target }}",
        ),
        (
            "lifecycle shell",
            "---\ntarget: null\nstart:\n  - shell: \"git init --quiet ran\"\n  - shell: \"ls {{ target }}\"\n---\nBody.\n",
            "ls {{ target }}",
        ),
        (
            "lifecycle set",
            "---\nstart:\n  - shell: \"git init --quiet ran\"\n  - set:\n      listing: \"$(ls {{ fm.target }})\"\n---\nBody.\n",
            "ls {{ fm.target }}",
        ),
    ] {
        let run = compose(&format!("shell-missing-{}", row.replace(' ', "-")), document);
        assert_ne!(run.code, Some(0), "{row}: {}", run.output);
        assert!(run.output.contains(command), "{row}: {}", run.output);
        assert!(run.output.contains("which is missing"), "{row}: {}", run.output);
        assert!(!run.ran, "{row}: a command ran before the missing value was reported");
    }
}

#[test]
fn an_empty_string_value_is_present_and_runs() {
    let run = compose(
        "shell-missing-empty",
        "---\nsuffix: \"\"\nstart:\n  - shell: \"git init --quiet ran{{ suffix }}\"\n---\nBody.\n",
    );
    assert_eq!(run.code, Some(0), "{}", run.output);
    assert!(run.ran, "{}", run.output);
}
