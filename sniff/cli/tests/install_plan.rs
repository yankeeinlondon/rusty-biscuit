use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn install_plan_vim_renders_text_output() {
    cargo_bin_cmd!("sniff")
        .args(["editors", "install-plan", "vim"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Vim"));
}

#[test]
fn install_plan_vim_json_returns_program_field() {
    let output = cargo_bin_cmd!("sniff")
        .args(["editors", "install-plan", "vim", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["program"], "Vim");
    assert!(json["options"].is_array());
    assert!(json["website"].is_string());
    assert!(json["successful"].is_boolean());
}

#[test]
fn install_plan_unknown_program_errors() {
    cargo_bin_cmd!("sniff")
        .args(["programs", "install-plan", "definitely-not-a-real-thing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown program"));
}

#[test]
fn install_dry_run_does_not_execute() {
    // Dry-run must always succeed because nothing actually runs.
    cargo_bin_cmd!("sniff")
        .args(["editors", "install", "vim", "--dry-run", "-y"])
        .assert()
        .success();
}

#[test]
fn install_via_unknown_manager_errors_with_valid_list() {
    cargo_bin_cmd!("sniff")
        .args(["editors", "install", "vim", "--via", "nonexistent-mgr", "--dry-run", "-y"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("valid manager").or(predicate::str::contains("Unknown manager")));
}
