use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

// =============================================================================
//                              GENERAL CLI TESTS
// =============================================================================

#[test]
fn test_main_help() {
    cargo_bin_cmd!("homey")
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Homelab automation CLI"))
        .stdout(predicate::str::contains("arcam"))
        .stdout(predicate::str::contains("sony"));
}

#[test]
fn test_no_command_shows_error() {
    cargo_bin_cmd!("homey")
        .env_remove("COMPLETE")
        .assert()
        .failure();
}

#[test]
fn test_completions_help_section() {
    cargo_bin_cmd!("homey")
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
    cargo_bin_cmd!("homey")
        .env("COMPLETE", "bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_clap_complete_homey"));
}

#[test]
fn test_dynamic_completions_zsh() {
    cargo_bin_cmd!("homey")
        .env("COMPLETE", "zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("homey"));
}

#[test]
fn test_dynamic_completions_fish() {
    cargo_bin_cmd!("homey")
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
    cargo_bin_cmd!("homey")
        .env_remove("ARCAM_AMP")
        .args(["arcam", "on"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Host required"));
}

#[test]
fn test_arcam_help_shows_actions() {
    cargo_bin_cmd!("homey")
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
    cargo_bin_cmd!("homey")
        .env_remove("SONY_RECEIVER")
        .args(["sony", "system", "power-status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Host required"));
}

#[test]
fn test_sony_help_shows_subcommands() {
    cargo_bin_cmd!("homey")
        .args(["sony", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("system"))
        .stdout(predicate::str::contains("audio"))
        .stdout(predicate::str::contains("input"))
        .stdout(predicate::str::contains("playback"))
        .stdout(predicate::str::contains("debug"))
        .stdout(predicate::str::contains("--host"))
        .stdout(predicate::str::contains("--port"));
}

#[test]
fn test_sony_system_help() {
    cargo_bin_cmd!("homey")
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
    cargo_bin_cmd!("homey")
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
    cargo_bin_cmd!("homey")
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
    cargo_bin_cmd!("homey")
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
    cargo_bin_cmd!("homey")
        .args(["sony", "debug", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("methods"))
        .stdout(predicate::str::contains("probe"));
}
