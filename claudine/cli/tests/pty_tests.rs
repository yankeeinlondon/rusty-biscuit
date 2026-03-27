#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use expectrl::{Expect, Session};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[cfg(unix)]
fn write_executable(path: &std::path::Path, content: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, content).unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(unix)]
#[test]
#[ignore = "PTY smoke tests are timing-sensitive in this harness; wrapper behavior is covered by non-PTY integration tests"]
fn pty_wrapper_summary_shows_badges() {
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    write_executable(&bin_dir.join("goose"), "#!/bin/sh\necho 'child-output'\n");

    let mut cmd = Command::new(cargo_bin!("claudine"));
    cmd.args(["goose", "-y", "-n", "--", "hi"]);
    cmd.env("NO_COLOR", "1");
    cmd.env("TERM_WIDTH", "80");
    cmd.env("PATH", bin_dir);

    let mut p = Session::spawn(cmd).expect("failed to spawn PTY");
    p.expect("Claudine").unwrap();
    p.expect("Goose").unwrap();
    p.expect("YOLO").unwrap();
    p.expect("Non-Interactive").unwrap();
    p.expect("child-output").unwrap();
}

#[cfg(unix)]
#[test]
#[ignore = "PTY smoke tests are timing-sensitive in this harness; wrapper behavior is covered by non-PTY integration tests"]
fn pty_non_interactive_detection() {
    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    write_executable(&bin_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let mut cmd = Command::new(cargo_bin!("claudine"));
    cmd.args(["goose", "--", "hi"]);
    cmd.env("NO_COLOR", "1");
    cmd.env("TERM_WIDTH", "80");
    cmd.env("PATH", bin_dir);

    let mut p = Session::spawn(cmd).expect("failed to spawn PTY");
    p.expect("Claudine").unwrap();
    p.expect("Goose").unwrap();
}
