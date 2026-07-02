use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

fn unchained() -> Command {
    cargo_bin_cmd!("unchained")
}

#[test]
fn test_help() {
    let mut cmd = unchained();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "AI pipeline tools and agent status monitoring",
        ))
        .stdout(predicate::str::contains("limits"));
}

#[test]
fn test_version() {
    let mut cmd = unchained();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn test_limits_help() {
    let mut cmd = unchained();
    cmd.arg("limits")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Show usage limits and cap status"))
        .stdout(predicate::str::contains("--platform"));
}

#[test]
fn test_models_help_shows_flat() {
    let mut cmd = unchained();
    cmd.arg("models")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--flat"));
}

#[test]
fn test_models_flat_provider_outputs_canonical_wire_ids() {
    let mut cmd = unchained();
    cmd.arg("models")
        .arg("--provider")
        .arg("zai")
        .arg("--flat")
        .assert()
        .success()
        .stdout(predicate::str::contains("z-ai/glm-4.5"))
        .stdout(predicate::str::contains("Z.ai").not());
}

#[test]
fn test_models_flat_preserves_aggregator_namespaces() {
    let mut cmd = unchained();
    cmd.arg("models")
        .arg("--provider")
        .arg("zenmux")
        .arg("--flat")
        .assert()
        .success()
        .stdout(predicate::str::contains("zenmux/anthropic/claude-opus-4"))
        .stdout(predicate::str::contains("zenmux/z-ai/glm-4.5"))
        .stdout(predicate::str::contains("ZenMux").not());
}

#[test]
fn test_limits_invalid_platform() {
    let mut cmd = unchained();
    cmd.arg("limits")
        .arg("--platform")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown platform"));
}

#[test]
fn test_no_subcommand() {
    let mut cmd = unchained();
    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("Usage: unchained"));
}

#[test]
fn test_completions_bash() {
    let mut cmd = unchained();
    cmd.arg("--completions")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_unchained()"));
}

#[test]
fn test_completions_zsh() {
    let mut cmd = unchained();
    cmd.arg("--completions")
        .arg("zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef unchained"));
}
