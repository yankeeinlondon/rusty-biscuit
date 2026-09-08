#![cfg(unix)]

use std::fs;

mod common;
use common::{CliProcessFixture, write_executable};

#[cfg(unix)]
#[test]
fn direct_wrap_opencode_argv() {
    let fixture = CliProcessFixture::named("direct-wrap-opencode-argv");
    let args_path = fixture.cwd().join("args.txt");

    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    fixture
        .command()
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
    let fixture = CliProcessFixture::named("direct-wrap-goose-argv");
    let args_path = fixture.cwd().join("args.txt");

    write_executable(
        &fixture.bin_dir().join("goose"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    fixture
        .command()
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
