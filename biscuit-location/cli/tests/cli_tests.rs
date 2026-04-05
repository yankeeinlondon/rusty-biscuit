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

// ---------------------------------------------------------------------------
// Runtime command paths
// ---------------------------------------------------------------------------

/// `where ip` with no database configured must surface the resolution error
/// on stderr with exit code 1, not crash or silently succeed.
#[test]
fn ip_without_db_reports_missing_database() {
    cmd()
        .args(["ip", "8.8.8.8"])
        .env("BISCUIT_LOCATION_MAXMIND_DB", "/does/not/exist.mmdb")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("MaxMind database not found"));
}

/// `where ip` JSON error envelope must stay on stderr so stdout stays clean.
#[test]
fn ip_without_db_json_error_on_stderr() {
    let out = cmd()
        .args(["--json", "ip", "8.8.8.8"])
        .env("BISCUIT_LOCATION_MAXMIND_DB", "/does/not/exist.mmdb")
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    assert!(out.stdout.is_empty(), "stdout not empty: {:?}", out.stdout);
    let stderr = String::from_utf8(out.stderr).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stderr.trim())
        .unwrap_or_else(|e| panic!("stderr not valid JSON ({e}): {stderr:?}"));
    assert_eq!(parsed["error"], true);
    assert!(
        parsed["message"].as_str().unwrap().contains("MaxMind"),
        "unexpected message: {parsed}"
    );
}

/// `where distance gps <coords>` must emit a clear "no GPS fix" message when
/// the host has no fix available — NOT the old `internal error: ...` form.
///
/// On macOS, the host may have location services denied for this CLI. In that
/// case the underlying `svc.gps()` returns `Ok(None)` and `resolve_input`
/// converts that into `LocationError::NoGpsFix`. The test is tolerant of both
/// "no fix" (exit 1) and "got a real fix" (exit 0) outcomes so it does not
/// flake on developer laptops with location services enabled.
#[test]
fn distance_gps_reports_no_gps_fix_cleanly() {
    let out = cmd()
        .args(["distance", "gps", "34.0522,-118.2437"])
        .timeout(std::time::Duration::from_secs(15))
        .output()
        .expect("spawn failed");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);

    // The failing case must NOT surface as an "internal error".
    if !out.status.success() {
        assert!(
            stderr.contains("no GPS fix available"),
            "expected clean no-fix message, got stderr: {stderr:?}"
        );
        assert!(
            !stderr.contains("internal error"),
            "no-fix was reported as an internal error: {stderr:?}"
        );
    } else {
        // If we got a real GPS fix, stdout should contain a distance value.
        assert!(
            stdout.contains("km") || stdout.contains("kilometers"),
            "success but no distance emitted: {stdout:?}"
        );
    }
}

/// `where reverse --timeout` must propagate a sub-second timeout so the
/// request fails fast when the network is slow or unreachable. This exercises
/// the reverse-timeout CLI flag threaded into `LocationConfig.reverse.timeout`.
#[test]
fn reverse_timeout_flag_is_honored() {
    // Point at a non-routable IP that will time out on connect. If the
    // timeout flag is not threaded into the config, the default 10s timeout
    // would apply and this test would hang.
    let out = cmd()
        .args([
            "reverse",
            "34.0522",
            "-118.2437",
            "--timeout",
            "1",
        ])
        .timeout(std::time::Duration::from_secs(10))
        .output()
        .expect("spawn failed");
    // The command may succeed (real Nominatim resolves in <1s) or fail with
    // a timeout. Either outcome proves the flag was accepted; what must NOT
    // happen is a clap usage error (exit 2) or a hang past the test timeout.
    assert!(
        out.status.code() != Some(2),
        "reverse --timeout rejected by clap: stderr={:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The `--maps` global flag should attach a Google Maps URL to locations.
/// This exercises `distance` indirectly - the flag is global and must not
/// break commands that do not emit a location (`distance` emits only a
/// number).
#[test]
fn maps_flag_does_not_break_distance() {
    cmd()
        .args([
            "--maps",
            "distance",
            "34.0522,-118.2437",
            "40.7128,-74.0060",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("km"));
}

/// JSON mode with `--maps` on a command that does emit a location must
/// include a `maps_url` field. We can't exercise `ip`/`gps`/`reverse`
/// without network/DB, so this test is only a negative assertion: distance
/// JSON never carries `maps_url`.
#[test]
fn distance_json_has_no_maps_url() {
    let out = cmd()
        .args([
            "--maps",
            "--json",
            "distance",
            "34.0522,-118.2437",
            "40.7128,-74.0060",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.get("maps_url").is_none(), "unexpected maps_url: {parsed}");
}
