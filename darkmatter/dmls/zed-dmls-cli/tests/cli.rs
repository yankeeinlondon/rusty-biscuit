use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn missing_command_uses_clap_exit_two() {
    Command::cargo_bin("zed-dmls").unwrap().assert().code(2);
}

#[test]
fn doctor_is_hermetic_with_path_overrides_and_plain_output() {
    let temp = TempDir::new().unwrap();
    Command::cargo_bin("zed-dmls")
        .unwrap()
        .env("PATH", "")
        .args([
            "doctor",
            "--plain",
            "--staging-dir",
            temp.path().join("stage").to_str().unwrap(),
            "--zed-data-dir",
            temp.path().join("zed").to_str().unwrap(),
            "--zed-log",
            temp.path().join("Zed.log").to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("FAIL: dmls binary missing from PATH"))
        .stdout(predicate::str::contains("\u{1b}[").not());
}

#[test]
fn conditional_doctor_is_silent_when_zed_is_absent() {
    let temp = TempDir::new().unwrap();
    Command::cargo_bin("zed-dmls")
        .unwrap()
        .env("PATH", "")
        .args([
            "doctor",
            "--if-zed-present",
            "--plain",
            "--zed-data-dir",
            temp.path().join("missing-zed").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn stage_returns_three_when_manual_registration_remains() {
    let temp = TempDir::new().unwrap();
    let staging = temp.path().join("stable/zed-dmls");
    Command::cargo_bin("zed-dmls")
        .unwrap()
        .args([
            "stage",
            "--plain",
            "--staging-dir",
            staging.to_str().unwrap(),
            "--zed-data-dir",
            temp.path().join("zed").to_str().unwrap(),
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::contains(staging.display().to_string()))
        .stdout(predicate::str::contains("manual registration required"))
        .stdout(predicate::str::contains("does not exist"));
    assert!(staging.join("extension.toml").exists());
    assert!(staging.join("extension.wasm").exists());
}

#[test]
fn stage_registers_and_returns_zero_when_zed_is_present() {
    let temp = TempDir::new().unwrap();
    let staging = temp.path().join("stable/zed-dmls");
    let zed = temp.path().join("zed");
    std::fs::create_dir_all(&zed).unwrap();
    Command::cargo_bin("zed-dmls")
        .unwrap()
        .args([
            "stage",
            "--plain",
            "--staging-dir",
            staging.to_str().unwrap(),
            "--zed-data-dir",
            zed.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("registered it as Zed's `dmls` dev extension"));
    let registration = zed.join("extensions/installed/dmls");
    assert_eq!(
        std::fs::canonicalize(&registration).unwrap(),
        std::fs::canonicalize(&staging).unwrap()
    );
}

#[test]
fn conditional_stage_is_silent_when_zed_is_absent() {
    let temp = TempDir::new().unwrap();
    let staging = temp.path().join("stable/zed-dmls");
    Command::cargo_bin("zed-dmls")
        .unwrap()
        .args([
            "stage",
            "--if-zed-present",
            "--plain",
            "--staging-dir",
            staging.to_str().unwrap(),
            "--zed-data-dir",
            temp.path().join("missing-zed").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
    assert!(!staging.exists());
}
