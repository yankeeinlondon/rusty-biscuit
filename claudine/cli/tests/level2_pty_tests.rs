#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use expectrl::{Expect, Session};
use std::fs;
use std::process::Command;
use tempfile::tempdir;
use test_toolkit::{Level, require_level};
mod common;
use common::{pty_available, write_executable};

#[test]
#[serial_test::serial(pty)]
fn level2_pty_wrapper_summary_shows_badges() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    write_executable(&bin_dir.join("goose"), "#!/bin/sh\necho 'child-output'\n");
    let home = workspace.path().join("home");
    common::wrap::seed_minimal_config(&home);

    let mut cmd = Command::new(cargo_bin("claudine"));
    cmd.current_dir(workspace.path());
    cmd.args(["goose", "-y", "-n", "--", "hi"]);
    cmd.env("NO_COLOR", "1");
    cmd.env("TERM_WIDTH", "80");
    cmd.env("HOME", home);
    cmd.env("PATH", bin_dir);

    let mut p = Session::spawn(cmd).expect("failed to spawn PTY");
    p.expect("Claudine").unwrap();
    p.expect("Goose").unwrap();
    p.expect("YOLO").unwrap();
    // `-n` records `INTERACTIVE=false` in the environment-variables
    // table the wrapper prints before delegating to the child binary.
    p.expect("INTERACTIVE").unwrap();
    p.expect("child-output").unwrap();
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_non_interactive_detection() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    write_executable(&bin_dir.join("goose"), "#!/bin/sh\nexit 0\n");
    let home = workspace.path().join("home");
    common::wrap::seed_minimal_config(&home);

    let mut cmd = Command::new(cargo_bin("claudine"));
    cmd.current_dir(workspace.path());
    cmd.args(["goose", "--", "hi"]);
    cmd.env("NO_COLOR", "1");
    cmd.env("TERM_WIDTH", "80");
    cmd.env("HOME", home);
    cmd.env("PATH", bin_dir);

    let mut p = Session::spawn(cmd).expect("failed to spawn PTY");
    p.expect("Claudine").unwrap();
    p.expect("Goose").unwrap();
}
