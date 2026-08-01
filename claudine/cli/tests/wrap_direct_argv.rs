#![cfg(unix)]

use std::fs;
use tempfile::tempdir;

fn write_executable(path: &std::path::Path, body: &str) {
    fs::write(path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn direct_wrap_opencode_argv() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let args_path = workspace.path().join("args.txt");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .current_dir(workspace.path())
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("PATH", &path_dir)
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args([
            "opencode",
            "--format",
            "json",
            "--yolo",
            "--",
            "- my bullet prompt",
        ])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let collected: Vec<_> = args.lines().collect();

    // assertions
    let run_index = collected
        .iter()
        .position(|arg| *arg == "run")
        .expect("run entrypoint");
    let model_index = collected
        .iter()
        .position(|arg| *arg == "--model")
        .expect("--model");
    let format_index = collected
        .iter()
        .position(|arg| *arg == "--format")
        .expect("--format");
    let yolo_index = collected
        .iter()
        .position(|arg| *arg == "--dangerously-skip-permissions")
        .expect("yolo");
    let sep_index = collected
        .iter()
        .position(|arg| *arg == "--")
        .expect("-- separator");
    let prompt_index = collected
        .iter()
        .position(|arg| *arg == "- my bullet prompt")
        .expect("prompt");

    assert!(model_index < sep_index);
    assert!(format_index < sep_index);
    assert!(yolo_index < sep_index);
    assert!(run_index < sep_index);
    assert!(prompt_index > sep_index);
    assert_eq!(
        prompt_index,
        collected.len() - 1,
        "prompt should be the last argument"
    );
}

#[cfg(unix)]
#[test]
fn direct_wrap_goose_argv() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let args_path = workspace.path().join("args.txt");
    fs::create_dir_all(&path_dir).unwrap();

    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .env("PATH", &path_dir)
        .env("GOOSE_MODEL", "test-model")
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["goose", "--yolo", "--", "my goose prompt"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let collected: Vec<_> = args.lines().collect();

    // goose expects 'run' as entrypoint
    let run_count = collected.iter().filter(|arg| **arg == "run").count();
    assert_eq!(run_count, 1, "exactly one 'run' entrypoint expected");
}
