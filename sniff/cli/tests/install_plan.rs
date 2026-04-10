use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

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

/// Helper: point HOME at a tempdir so HostCapabilities doesn't touch the real
/// cache file. Returns the tempdir (must stay alive) and a ready Command.
fn cmd_with_tmp_home() -> (TempDir, assert_cmd::Command) {
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = cargo_bin_cmd!("sniff");
    cmd.env("HOME", tmp.path());
    (tmp, cmd)
}

#[cfg(unix)]
#[test]
fn install_plan_populates_cache_file() {
    let (tmp, mut cmd) = cmd_with_tmp_home();
    cmd.args(["editors", "install-plan", "vim"])
        .assert()
        .success();
    let cache = tmp.path().join(".sniff-programs.json");
    assert!(cache.exists(), "cache file should be created");
}

#[cfg(unix)]
#[test]
fn install_plan_force_rebuilds_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join(".sniff-programs.json");
    // Seed a garbage cache.
    std::fs::write(&cache, "garbage").unwrap();

    cargo_bin_cmd!("sniff")
        .env("HOME", tmp.path())
        .args(["editors", "install-plan", "vim", "--force"])
        .assert()
        .success();

    let after = std::fs::read_to_string(&cache).unwrap();
    assert_ne!(after, "garbage", "cache should have been rewritten");
}

#[test]
fn install_plan_no_sudo_never_selects_sudo_method() {
    // We can't force a deterministic host, but we can assert that any
    // selected option has requires_sudo = false when --no-sudo is passed.
    let output = cargo_bin_cmd!("sniff")
        .args([
            "editors",
            "install-plan",
            "vim",
            "--no-sudo",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: Value = serde_json::from_str(&stdout).unwrap();
    if json["successful"] == Value::Bool(true) {
        let options = json["options"].as_array().unwrap();
        let chosen = options.iter().find(|o| o["choose"] == Value::Bool(true)).unwrap();
        assert_eq!(chosen["requires_sudo"], Value::Bool(false));
    }
}
