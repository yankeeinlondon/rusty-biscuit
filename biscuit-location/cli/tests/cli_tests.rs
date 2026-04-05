use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    let mut c = Command::cargo_bin("where").unwrap();
    // Keep output deterministic across local machines and CI.
    c.env_remove("FORCE_COLOR");
    c.env_remove("RUST_LOG");
    c.env("NO_COLOR", "1");
    c
}

// ---------------------------------------------------------------------------
// Help, version, no-subcommand
// ---------------------------------------------------------------------------

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
fn help_does_not_show_help_subcommand() {
    // The `help` subcommand is disabled (disable_help_subcommand=true).
    let out = cmd().arg("--help").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    // Look for a line that starts with "  help" (the subcommand list entry).
    // We only reject the subcommand line, not general help text.
    for line in stdout.lines() {
        assert!(
            !line.trim_start().starts_with("help  "),
            "help subcommand should not appear in help output, got: {line}"
        );
    }
}

#[test]
fn help_does_not_show_completions_subcommand() {
    // Completions is hidden (#[command(hide = true)]). The word still
    // appears in the "Shell completions:" examples section of AFTER_HELP,
    // but it must NOT appear as a subcommand row (starts with "  completions").
    let out = cmd().arg("--help").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    for line in stdout.lines() {
        assert!(
            !line.trim_start().starts_with("completions  "),
            "hidden `completions` subcommand leaked into help output: {line}"
        );
    }
}

#[test]
fn help_mentions_output_modes() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--plain"));
}

#[test]
fn shows_version() {
    cmd().arg("--version").assert().success();
}

#[test]
fn no_subcommand_shows_help_on_stderr() {
    cmd()
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("Usage"));
}

// ---------------------------------------------------------------------------
// Distance
// ---------------------------------------------------------------------------

#[test]
fn distance_between_coordinates_text() {
    cmd()
        .args(["distance", "34.0522,-118.2437", "40.7128,-74.0060"])
        .assert()
        .success()
        .stdout(predicate::str::contains("km"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn distance_between_coordinates_miles() {
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
fn distance_unit_accepts_short_alias() {
    cmd()
        .args([
            "distance",
            "34.0522,-118.2437",
            "40.7128,-74.0060",
            "--unit",
            "mi",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("miles"));
}

#[test]
fn distance_json_output_shape() {
    let out = cmd()
        .args([
            "distance",
            "34.0522,-118.2437",
            "40.7128,-74.0060",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid JSON on stdout");
    assert!(parsed["value"].as_f64().unwrap() > 3000.0); // LA to NYC > 3000 km
    assert_eq!(parsed["unit"], "kilometers");
}

#[test]
fn distance_json_nautical_miles_snake_case() {
    let out = cmd()
        .args([
            "distance",
            "34.0522,-118.2437",
            "40.7128,-74.0060",
            "--json",
            "--unit",
            "nm",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["unit"], "nautical_miles");
}

#[test]
fn distance_plain_mode_has_no_escape_codes() {
    let out = cmd()
        .args([
            "distance",
            "34.0522,-118.2437",
            "40.7128,-74.0060",
            "--plain",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(
        !text.contains('\x1b'),
        "plain output contained escape codes: {text:?}"
    );
}

// ---------------------------------------------------------------------------
// Validation (exit code 2 = usage error from clap)
// ---------------------------------------------------------------------------

#[test]
fn invalid_coordinates_rejected() {
    cmd()
        .args(["reverse", "999", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error").or(predicate::str::contains("invalid")));
}

#[test]
fn invalid_ip_gives_usage_error() {
    cmd()
        .args(["ip", "not-an-ip"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid value").or(predicate::str::contains("error:")));
}

#[test]
fn distance_invalid_input_gives_usage_error() {
    cmd()
        .args(["distance", "not-a-location", "40.71,-74.01"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn distance_invalid_unit_rejected() {
    cmd()
        .args([
            "distance",
            "34.0522,-118.2437",
            "40.7128,-74.0060",
            "--unit",
            "furlongs",
        ])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn conflicting_json_plain_rejected() {
    cmd()
        .args(["--json", "--plain", "distance", "gps", "gps"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn conflicting_verbose_quiet_rejected() {
    cmd()
        .args([
            "-v",
            "--quiet",
            "distance",
            "34.05,-118.24",
            "40.71,-74.01",
        ])
        .assert()
        .failure()
        .code(2);
}

// ---------------------------------------------------------------------------
// Quiet mode
// ---------------------------------------------------------------------------

#[test]
fn verbose_emits_tracing_to_stderr() {
    cmd()
        .args([
            "-vv",
            "distance",
            "34.0522,-118.2437",
            "40.7128,-74.0060",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("km"))
        .stderr(predicate::str::contains("DEBUG").or(predicate::str::contains("dispatching")));
}

#[test]
fn default_invocation_has_clean_stderr() {
    cmd()
        .args(["distance", "34.0522,-118.2437", "40.7128,-74.0060"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn quiet_suppresses_stdout_on_success() {
    cmd()
        .args([
            "--quiet",
            "distance",
            "34.0522,-118.2437",
            "40.7128,-74.0060",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// ---------------------------------------------------------------------------
// Completions
// ---------------------------------------------------------------------------

#[test]
fn completions_bash_generates_script() {
    cmd()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_where"))
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn completions_zsh_generates_script() {
    cmd()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef where"));
}

#[test]
fn completions_fish_generates_script() {
    cmd()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("where"));
}

#[test]
fn completions_requires_shell_name() {
    cmd().arg("completions").assert().failure().code(2);
}
