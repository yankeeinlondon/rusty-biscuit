//! Level-1 process coverage for the shell approval prompt's no-terminal
//! refusal: `claudine compose` of a document whose command could be approved
//! by its shape shows no prompt when stdin is not a terminal, fails pre-flight
//! as approval unavailable, writes no policy file, and launches no provider.
//! The prompt's choices and what each saves are covered by the scripted
//! sessions beside the handler's install site.

use crate::common;

use common::CliProcessFixture;
use common::write;
#[cfg(unix)]
use common::write_executable;
use std::path::Path;

/// A Claude stub that leaves a marker when it is launched.
fn write_marking_claude(bin_dir: &Path, marker: &Path) {
    #[cfg(unix)]
    write_executable(
        &bin_dir.join("claude"),
        &format!("#!/bin/sh\ntouch '{}'\nexit 0\n", marker.display()),
    );

    #[cfg(windows)]
    write(
        &bin_dir.join("claude.cmd"),
        &format!("@echo off\r\ntype nul > \"{}\"\r\nexit /b 0\r\n", marker.display()),
    );
}

#[test]
fn without_a_terminal_a_command_shape_is_not_prompted_for_and_nothing_runs() {
    let fixture = CliProcessFixture::named("shell-approval-prompt-no-terminal");
    fixture.seed_user_config();
    let marker = fixture.cwd().join("provider-launched");
    write_marking_claude(fixture.bin_dir(), &marker);
    write(
        &fixture.cwd().join("doc.md"),
        "---\nout: dist\n---\n::shell just clean --out {{ out }}/build\n",
    );

    let output = fixture
        .command()
        .args(["compose", "--claude", "doc.md"])
        .output()
        .expect("claudine runs");

    let text = common::strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
    .replace(['┃', '│'], " ")
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ");
    assert!(!output.status.success(), "{text}");
    assert!(
        text.contains("'just clean --out dist/build'") && text.contains("no approval handler is available"),
        "{text}"
    );
    assert!(!text.contains("Shell Approval Required"), "no prompt without a terminal: {text}");
    assert!(!fixture.cwd().join(".darkmatter-shell-whitelist").exists());
    assert!(!marker.exists(), "the provider must not launch");
}
