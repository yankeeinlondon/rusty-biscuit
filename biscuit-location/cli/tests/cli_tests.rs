use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("where").unwrap()
}

#[test]
fn shows_help() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("gps"))
        .stdout(predicate::str::contains("ip"))
        .stdout(predicate::str::contains("reverse"))
        .stdout(predicate::str::contains("distance"));
}

#[test]
fn shows_version() {
    cmd().arg("--version").assert().success();
}

#[test]
fn distance_between_coordinates() {
    cmd()
        .args(["distance", "34.0522,-118.2437", "40.7128,-74.0060"])
        .assert()
        .success()
        .stdout(predicate::str::contains("km"));
}

#[test]
fn distance_with_miles() {
    cmd()
        .args([
            "distance",
            "34.0522,-118.2437",
            "40.7128,-74.0060",
            "--unit",
            "miles",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("miles"));
}

#[test]
fn invalid_coordinates_rejected() {
    cmd()
        .args(["reverse", "999", "0"])
        .assert()
        .failure();
}

#[test]
fn invalid_ip_rejected() {
    cmd()
        .args(["ip", "not-an-ip"])
        .assert()
        .failure();
}

#[test]
fn distance_invalid_input() {
    cmd()
        .args(["distance", "not-a-location", "40.71,-74.01"])
        .assert()
        .failure();
}

#[test]
fn no_subcommand_shows_help() {
    cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}
