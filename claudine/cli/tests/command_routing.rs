use assert_cmd::cargo::cargo_bin_cmd;

mod common;

use common::{TestWorkspace, strip_ansi, write};

fn seed_user_config(home: &std::path::Path) {
    let config = serde_json::json!({
        "preferred_agent": "claude",
        "tts": false,
        "logging": false,
        "protect": { "enabled": false },
        "actions": {
            "session_start": [
                {
                    "type": "report",
                    "handler": { "format": "json" }
                }
            ]
        }
    });

    write(
        &home.join(".claudine/config.json"),
        &serde_json::to_string_pretty(&config).unwrap(),
    );
}

#[test]
fn providers_command_routes_to_stdout() {
    let workspace = TestWorkspace::named("claudine-command-routing");
    let home = workspace.path().join("home");
    seed_user_config(&home);

    let output = cargo_bin_cmd!("claudine")
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .arg("providers")
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("Provider"));
    assert!(stdout.contains("Claude"));
    assert!(
        stderr.trim().is_empty(),
        "expected no stderr, got: {stderr}"
    );
}

#[test]
fn hooks_support_command_routes_without_detected_agents() {
    let workspace = TestWorkspace::named("claudine-command-routing");
    let home = workspace.path().join("home");
    seed_user_config(&home);

    let output = cargo_bin_cmd!("claudine")
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("TERM_WIDTH", "160")
        .args(["hooks", "--support"])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Event") || stdout.contains("Table could not be rendered"),
        "unexpected hooks support output: {stdout}"
    );
    assert!(
        stdout.contains("SessionStart") || stdout.contains("hook support"),
        "unexpected hooks support output: {stdout}"
    );
}

#[test]
fn actions_command_routes_and_reports_configured_events() {
    let workspace = TestWorkspace::named("claudine-command-routing");
    let home = workspace.path().join("home");
    seed_user_config(&home);

    let stdout = String::from_utf8(
        cargo_bin_cmd!("claudine")
            .env("HOME", &home)
            .env("NO_COLOR", "1")
            .arg("actions")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();

    assert!(stdout.contains("Report"));
    assert!(stdout.contains("SessionStart"));
}

#[test]
fn agents_and_commands_route_to_empty_state_messages() {
    let workspace = TestWorkspace::named("claudine-command-routing");
    let home = workspace.path().join("home");
    seed_user_config(&home);

    let agents_stdout = String::from_utf8(
        cargo_bin_cmd!("claudine")
            .env("HOME", &home)
            .env("NO_COLOR", "1")
            .arg("agents")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(
        agents_stdout.contains("No agents found.") || agents_stdout.contains("Agents"),
        "unexpected agents output: {agents_stdout}"
    );

    let commands_stdout = String::from_utf8(
        cargo_bin_cmd!("claudine")
            .env("HOME", &home)
            .env("NO_COLOR", "1")
            .arg("commands")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert!(
        commands_stdout.contains("No slash commands found.")
            || commands_stdout.contains("Slash Commands"),
        "unexpected commands output: {commands_stdout}"
    );
}

#[test]
fn completions_emit_dynamic_bootstrap_snippets() {
    // `claudine completions <shell>` no longer ships a static clap_complete
    // script; it now emits a one-line bootstrap that activates dynamic
    // completion via the `COMPLETE=<shell>` runtime hook. Each expected
    // snippet is exact except that the marker assertions skip trailing
    // whitespace so future formatting tweaks don't break the contract.
    for (shell, expected) in [
        ("bash", "source <(COMPLETE=bash claudine)\n"),
        ("zsh", "source <(COMPLETE=zsh claudine)\n"),
        ("fish", "COMPLETE=fish claudine | source\n"),
        (
            "powershell",
            "& { $env:COMPLETE=\"powershell\"; claudine } | Out-String | Invoke-Expression\n",
        ),
        ("elvish", "eval (E:COMPLETE=elvish claudine | slurp)\n"),
    ] {
        let output = cargo_bin_cmd!("claudine")
            .env("NO_COLOR", "1")
            .args(["completions", shell])
            .assert()
            .success()
            .get_output()
            .clone();

        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_eq!(
            stdout, expected,
            "expected {shell} bootstrap snippet to match exactly",
        );
        assert!(
            stderr.trim().is_empty(),
            "expected no stderr, got: {stderr}"
        );
        assert!(
            !stdout.contains("_clap_complete"),
            "{shell} bootstrap leaked the retired static clap_complete script",
        );
    }
}

#[test]
fn no_color_and_plain_suppress_ansi_output() {
    let workspace = TestWorkspace::named("claudine-command-routing");
    let home = workspace.path().join("home");
    seed_user_config(&home);

    let no_color_stdout = String::from_utf8(
        cargo_bin_cmd!("claudine")
            .env("HOME", &home)
            .env("NO_COLOR", "1")
            .arg("providers")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert_eq!(strip_ansi(&no_color_stdout), no_color_stdout);

    let plain_stdout = String::from_utf8(
        cargo_bin_cmd!("claudine")
            .env("HOME", &home)
            .arg("--plain")
            .arg("providers")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert_eq!(strip_ansi(&plain_stdout), plain_stdout);
}

#[test]
fn force_color_enables_ansi_in_non_tty_context() {
    let output = cargo_bin_cmd!("claudine")
        .env("FORCE_COLOR", "1")
        .args(["sequence", "/tmp/definitely-missing-sequence.md"])
        .assert()
        .failure()
        .get_output()
        .clone();

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains('\u{1b}'),
        "expected ANSI output, got: {stderr}"
    );
    assert!(strip_ansi(&stderr).contains("Error:"));
}

#[test]
fn errors_stay_on_stderr_for_command_failures() {
    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .args(["sequence", "/tmp/definitely-missing-sequence.md"])
        .assert()
        .failure()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stdout.trim().is_empty(),
        "expected no stdout, got: {stdout}"
    );
    assert!(strip_ansi(&stderr).contains("Error:"));
}
