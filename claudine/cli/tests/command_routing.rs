
mod common;

use common::{CliProcessFixture, strip_ansi, write};
use claudine::actions::HookAction;
use claudine::events::AgenticEvent;

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
    let fixture = CliProcessFixture::named("command-routing-providers");
    seed_user_config(fixture.home());

    let output = fixture
        .command()
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
    let fixture = CliProcessFixture::named("command-routing-hooks-support");
    seed_user_config(fixture.home());

    let output = fixture
        .command()
        .args(["hooks", "--support"])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Event"),
        "unexpected hooks support output: {stdout}"
    );
    assert!(
        !stdout.contains("Table could not be rendered"),
        "support view must chunk instead of refusing: {stdout}"
    );
    assert!(
        stdout.contains("SessionStart"),
        "unexpected hooks support output: {stdout}"
    );
}

#[test]
fn explicit_config_path_loads_configured_actions() {
    let fixture = CliProcessFixture::named("command-routing-explicit-config");
    let config_path = fixture.cwd().join("config.json");
    write(
        &config_path,
        r#"{
  "preferred_agent": "claude",
  "tts": false,
  "logging": false,
  "protect": { "enabled": false },
  "actions": {
    "session_start": [
      { "type": "report", "handler": { "format": "json" } }
    ]
  }
}"#,
    );

    let config = claudine::dispatch::loader::load_claudine_config(Some(&config_path), None)
        .expect("explicit fixture config should load");
    assert!(
        config
            .actions
            .get(&AgenticEvent::SessionStart)
            .is_some_and(|actions| {
                actions
                    .iter()
                    .any(|action| matches!(action, HookAction::Report { .. }))
            })
    );

    fixture
        .command()
        .args(["actions", "--help"])
        .assert()
        .success();
}

#[test]
fn agents_and_commands_route_to_empty_state_messages() {
    let fixture = CliProcessFixture::named("command-routing-empty-state");
    seed_user_config(fixture.home());

    let agents_stdout = String::from_utf8(
        fixture
            .command()
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
        fixture
            .command()
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
fn completions_emit_supplement_aware_bash_zsh_fish_scripts() {
    // Phase 5 of the `2026-04-18-file-completion-supplement` feature
    // flips `claudine completions bash|zsh|fish` from the old one-line
    // `COMPLETE=<shell>` bootstrap to full shell-specific scripts that
    // shell out to the hidden `claudine __complete` subcommand. This
    // test pins the three script contracts so unintended regressions
    // surface at install time rather than at runtime on a user's
    // shell.
    let fixture = CliProcessFixture::named("command-routing-completions");

    for (shell, markers) in [
        (
            "bash",
            vec![
                "_claudine_complete()",
                "claudine __complete --current",
                "\"${COMP_WORDS[@]}\"",
                "$COMP_CWORD",
                "-F _claudine_complete claudine",
                "-o bashdefault",
                "-o default",
            ],
        ),
        (
            "zsh",
            vec![
                "#compdef claudine",
                "_claudine()",
                "claudine __complete --current",
                "\"${words[@]}\"",
                "compdef _claudine claudine",
                "_files",
            ],
        ),
        (
            "fish",
            vec![
                "function __claudine_complete",
                "claudine __complete --current",
                "commandline -opc",
                "commandline -ct",
                "set -l candidates",
                "__fish_complete_path",
                "complete -c claudine",
                "-f",
                "-a '(__claudine_complete)'",
            ],
        ),
    ] {
        let output = fixture
            .command()
            .args(["completions", shell])
            .assert()
            .success()
            .get_output()
            .clone();

        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.trim().is_empty(),
            "expected no stderr for {shell}, got: {stderr}"
        );
        assert!(
            stdout.ends_with('\n'),
            "{shell} script should end with a newline; output was: {stdout}",
        );
        for marker in markers {
            assert!(
                stdout.contains(marker),
                "{shell} script missing marker `{marker}`; output was:\n{stdout}",
            );
        }
        // The supplement scripts must not use the legacy `COMPLETE=<shell>`
        // bootstrap — that would silently route the user back onto the
        // old `CompleteEnv` engine.
        assert!(
            !stdout.contains("COMPLETE="),
            "{shell} script must not fall back to the legacy `COMPLETE=<shell>` \
             bootstrap; output was:\n{stdout}",
        );
    }
}

#[test]
fn completions_retain_legacy_bootstrap_for_powershell_and_elvish() {
    // The supplement acceptance matrix covers bash/zsh/fish only.
    // PowerShell and Elvish keep the previous one-line `COMPLETE=<shell>`
    // bootstrap so stale installations and future shell additions don't
    // regress.
    let fixture = CliProcessFixture::named("command-routing-legacy-completions");

    for (shell, expected) in [
        (
            "powershell",
            "& { $env:COMPLETE=\"powershell\"; claudine } | Out-String | Invoke-Expression\n",
        ),
        ("elvish", "eval (E:COMPLETE=elvish claudine | slurp)\n"),
    ] {
        let output = fixture
            .command()
            .args(["completions", shell])
            .assert()
            .success()
            .get_output()
            .clone();

        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_eq!(
            stdout, expected,
            "expected {shell} legacy bootstrap snippet to match exactly",
        );
        assert!(
            stderr.trim().is_empty(),
            "expected no stderr, got: {stderr}"
        );
    }
}

#[test]
fn no_color_and_plain_suppress_ansi_output() {
    let fixture = CliProcessFixture::named("command-routing-plain-output");
    seed_user_config(fixture.home());

    let no_color_stdout = String::from_utf8(
        fixture
            .command()
            .arg("providers")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone(),
    )
    .unwrap();
    assert_eq!(strip_ansi(&no_color_stdout), no_color_stdout);

    // `--plain` is the subject here, so the fixture's default `NO_COLOR` has to
    // go: leaving it set would suppress color on its own and the assertion
    // would hold whether `--plain` worked or not.
    let plain_stdout = String::from_utf8(
        fixture
            .command()
            .env_remove("NO_COLOR")
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
    let fixture = CliProcessFixture::named("command-routing-force-color");
    let missing = fixture.cwd().join("definitely-missing-sequence.md");

    let output = fixture
        .command()
        .env("FORCE_COLOR", "1")
        .env_remove("NO_COLOR")
        .args(["sequence", missing.to_str().unwrap()])
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
    let fixture = CliProcessFixture::named("command-routing-error-stream");
    let missing = fixture.cwd().join("definitely-missing-sequence.md");

    let output = fixture
        .command()
        .args(["sequence", missing.to_str().unwrap()])
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
