use predicates::prelude::*;
use std::path::PathBuf;

/// Returns a temporary HOME directory with no homey.json config.
/// This ensures tests don't accidentally use the user's real config.
fn empty_home() -> PathBuf {
    let dir = std::env::temp_dir().join("homey-cli-test");
    std::fs::create_dir_all(&dir).unwrap();
    // Remove any leftover config from previous test runs
    let _ = std::fs::remove_file(dir.join("homey.json"));
    dir
}

// =============================================================================
//                              GENERAL CLI TESTS
// =============================================================================

#[test]
fn test_main_help() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Homelab automation CLI"))
        .stdout(predicate::str::contains("arcam"))
        .stdout(predicate::str::contains("sony"));
}

#[test]
fn test_no_command_shows_error() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .env_remove("COMPLETE")
        .assert()
        .failure();
}

#[test]
fn test_completions_help_section() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .args(["completions"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SHELL COMPLETIONS"))
        .stdout(predicate::str::contains("COMPLETE=bash homey"));
}

// =============================================================================
//                              DYNAMIC COMPLETIONS TESTS
// =============================================================================

#[test]
fn test_dynamic_completions_bash() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .env("COMPLETE", "bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_clap_complete_homey"));
}

#[test]
fn test_dynamic_completions_zsh() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .env("COMPLETE", "zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("homey"));
}

#[test]
fn test_dynamic_completions_fish() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .env("COMPLETE", "fish")
        .assert()
        .success()
        .stdout(predicate::str::contains("homey"));
}

// =============================================================================
//                              ARCAM TESTS
// =============================================================================

#[test]
fn test_arcam_missing_host_error() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .env_remove("ARCAM_AMP")
        .env("HOME", empty_home())
        .args(["arcam", "on"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Host required"));
}

#[test]
fn test_arcam_help_shows_actions() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .args(["arcam", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("on"))
        .stdout(predicate::str::contains("off"))
        .stdout(predicate::str::contains("power-status"))
        .stdout(predicate::str::contains("mute-status"))
        .stdout(predicate::str::contains("mute-toggle"));
}

// =============================================================================
//                              SONY TESTS
// =============================================================================

#[test]
fn test_sony_missing_host_error() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .env_remove("SONY_RECEIVER")
        .env("HOME", empty_home())
        .args(["sony", "system", "power-status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Host required"));
}

#[test]
fn test_sony_help_shows_subcommands() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .args(["sony", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("system"))
        .stdout(predicate::str::contains("audio"))
        .stdout(predicate::str::contains("input"))
        .stdout(predicate::str::contains("playback"))
        .stdout(predicate::str::contains("debug"))
        .stdout(predicate::str::contains("--name"))
        .stdout(predicate::str::contains("--host"))
        .stdout(predicate::str::contains("--port"));
}

#[test]
fn test_sony_system_help() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .args(["sony", "system", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("power-status"))
        .stdout(predicate::str::contains("on"))
        .stdout(predicate::str::contains("off"))
        .stdout(predicate::str::contains("info"))
        .stdout(predicate::str::contains("update-check"));
}

#[test]
fn test_sony_audio_help() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .args(["sony", "audio", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("volume"))
        .stdout(predicate::str::contains("set-volume"))
        .stdout(predicate::str::contains("mute-status"))
        .stdout(predicate::str::contains("mute"))
        .stdout(predicate::str::contains("unmute"));
}

#[test]
fn test_sony_input_help() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .args(["sony", "input", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("current"))
        .stdout(predicate::str::contains("set"))
        .stdout(predicate::str::contains("schemes"));
}

#[test]
fn test_sony_playback_help() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .args(["sony", "playback", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("now-playing"))
        .stdout(predicate::str::contains("stop"))
        .stdout(predicate::str::contains("pause"))
        .stdout(predicate::str::contains("next"));
}

#[test]
fn test_sony_debug_help() {
    assert_cmd::Command::cargo_bin("homey")
        .unwrap()
        .args(["sony", "debug", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("methods"))
        .stdout(predicate::str::contains("probe"));
}
