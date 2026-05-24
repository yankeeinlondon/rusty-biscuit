use assert_cmd::cargo::cargo_bin_cmd;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../..").canonicalize().unwrap()
}

#[test]
fn context_default_exits_zero_and_produces_stdout() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.is_empty(), "default context should produce stdout");
    assert!(
        stdout.contains("ctx.today"),
        "should display ctx.today; got: {stdout}"
    );
    assert!(
        stdout.contains("Property"),
        "should have Property column; got: {stdout}"
    );
    assert!(
        stdout.contains("Type"),
        "should have Type column; got: {stdout}"
    );
    assert!(
        stdout.contains("Description"),
        "should have Description column; got: {stdout}"
    );
    assert!(
        stdout.contains("Date and Time Information"),
        "should have H3 grouping; got: {stdout}"
    );
}

#[test]
fn context_default_works_outside_repo() {
    let temp_dir = std::env::temp_dir();
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(&temp_dir)
        .args(["context"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("ctx.today"),
        "should show ctx.today outside repo; got: {stdout}"
    );
    assert!(
        stdout.contains("Property"),
        "should have Property column outside repo; got: {stdout}"
    );
}

#[test]
fn context_values_exits_zero_and_produces_stdout() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--values"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.is_empty(), "--values should produce stdout");
    assert!(
        stdout.contains("ctx.today"),
        "should display ctx.today; got: {stdout}"
    );
    assert!(
        stdout.contains("Value"),
        "should have Value column; got: {stdout}"
    );
    assert!(
        !stdout.contains("Description"),
        "should NOT have Description column; got: {stdout}"
    );
}

#[test]
fn context_expressions_exits_zero_and_produces_stdout() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--expressions"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.is_empty(), "--expressions should produce stdout");
    assert!(
        stdout.contains("Operator Precedence"),
        "should show precedence; got: {stdout}"
    );
    assert!(
        stdout.contains("Truthiness"),
        "should show truthiness; got: {stdout}"
    );
    assert!(
        stdout.contains("Unary Operators"),
        "should show unary operators; got: {stdout}"
    );
    assert!(
        stdout.contains("Comparison Operators"),
        "should show comparison operators; got: {stdout}"
    );
    assert!(
        stdout.contains("Arithmetic Operators"),
        "should show arithmetic operators; got: {stdout}"
    );
    assert!(
        stdout.contains("Variable Access"),
        "should show variable access; got: {stdout}"
    );
    assert!(
        stdout.contains("Null Propagation"),
        "should show null propagation; got: {stdout}"
    );
    assert!(
        stdout.contains("Functions"),
        "should show functions; got: {stdout}"
    );
}

#[test]
fn context_side_effects_exits_zero_and_produces_stdout() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--side-effects"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("not implemented yet"),
        "--side-effects should show placeholder; got: {stdout}"
    );
}

#[test]
fn context_default_writes_footer_to_stderr() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--expressions"),
        "stderr should mention --expressions; got: {stderr}"
    );
    assert!(
        stderr.contains("--side-effects"),
        "stderr should mention --side-effects; got: {stderr}"
    );
}

#[test]
fn context_values_writes_footer_to_stderr() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--values"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--expressions"),
        "stderr should mention --expressions; got: {stderr}"
    );
    assert!(
        stderr.contains("--side-effects"),
        "stderr should mention --side-effects; got: {stderr}"
    );
}

#[test]
fn context_expressions_writes_footer_to_stderr() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--expressions"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--expressions"),
        "stderr should mention --expressions; got: {stderr}"
    );
    assert!(
        stderr.contains("--side-effects"),
        "stderr should mention --side-effects; got: {stderr}"
    );
}

#[test]
fn context_side_effects_writes_footer_to_stderr() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--side-effects"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--expressions"),
        "stderr should mention --expressions; got: {stderr}"
    );
    assert!(
        stderr.contains("--side-effects"),
        "stderr should mention --side-effects; got: {stderr}"
    );
}
