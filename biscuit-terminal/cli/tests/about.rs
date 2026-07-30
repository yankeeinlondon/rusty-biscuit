//! Integration tests for the `bt about [APP]` command.

use predicates::prelude::*;

fn about_kitty_without_config() -> assert_cmd::Command {
    let config_home = tempfile::tempdir().expect("temp config home");
    let mut cmd = about_kitty_as_non_current();
    cmd.env("XDG_CONFIG_HOME", config_home.path())
        .env_remove("KITTY_CONFIG_DIRECTORY");
    cmd
}

fn about_kitty_as_non_current() -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("bt").unwrap();
    cmd.env("TERM_PROGRAM", "WezTerm")
        .env_remove("TERM")
        .env_remove("KITTY_PID")
        .env_remove("KITTY_WINDOW_ID")
        .env_remove("KITTY_PUBLIC_KEY")
        .env_remove("KITTY_LISTEN_ON")
        .env_remove("KITTY_CONFIG_DIRECTORY")
        .env_remove("KITTY_INSTALLATION_DIR");
    cmd
}

fn about_kitty_as_current() -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("bt").unwrap();
    cmd.env("TERM_PROGRAM", "kitty")
        .env_remove("TERM")
        .env("KITTY_PID", "12345")
        .env("KITTY_LISTEN_ON", "unix:/tmp/bt-kitty.sock");
    cmd
}

fn setting_value<'a>(parsed: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    parsed
        .pointer(&format!("/config/settings/{key}/value"))
        .and_then(|value| value.as_str())
}

#[test]
fn test_about_kitty_plain_renders_report() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["about", "kitty", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("About Kitty"))
        .stdout(predicate::str::contains("Identity"))
        .stdout(predicate::str::contains("Config Candidates"));
}

#[test]
fn test_about_kitty_plain_has_no_ansi_escapes() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["about", "kitty", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[").not());
}

#[test]
fn test_about_kitty_plain_includes_setting_locators_without_config() {
    let output = about_kitty_without_config()
        .args(["about", "kitty", "--plain"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Settings"));
    assert!(stdout.contains("allow_remote_control"));
    assert!(stdout.contains("font_size"));
    assert!(stdout.contains("background_opacity"));
    assert!(stdout.contains("no config file"));
    assert!(!stdout.contains("No settings extracted."));
}

#[test]
fn test_about_alacritty_plain_includes_config_candidates_per_os_target() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .env_remove("TERM")
        .args(["about", "alacritty", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config Candidates"))
        .stdout(predicate::str::contains("Linux"))
        .stdout(predicate::str::contains("MacOS"))
        .stdout(predicate::str::contains("Windows"))
        .stdout(predicate::str::contains("Wsl1"))
        .stdout(predicate::str::contains("Wsl2"))
        .stdout(predicate::str::contains("active"));
}

#[test]
fn test_about_alacritty_plain_reports_xdg_config_home_override() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .env_remove("TERM")
        .env_remove("XDG_CONFIG_HOME")
        .args(["about", "alacritty", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config Overrides"))
        .stdout(predicate::str::contains("XDG_CONFIG_HOME"))
        .stdout(predicate::str::contains("(dir):"))
        .stdout(predicate::str::contains("unset"))
        .stdout(predicate::str::contains("No config-relocating environment variables.").not());
}

#[test]
fn test_about_kitty_json_outputs_valid_report() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["about", "kitty", "--json"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("app").and_then(|v| v.as_str()), Some("Kitty"));
    assert!(parsed.get("variant").is_some());
    assert!(parsed.get("is_current").is_some());
    assert!(parsed.get("installed").is_some());
    assert!(parsed.get("install_status").is_some());
    assert!(parsed.get("os_target").is_some());
    assert!(parsed.get("config").is_some());
    assert!(parsed.get("env").is_some());
    assert!(parsed.get("config_candidates").is_none());
    assert!(parsed.get("env_overrides").is_none());
    assert!(parsed.get("resolved_config").is_none());
    assert!(parsed.get("settings").is_none());
    assert!(parsed.get("env_facts").is_none());
}

#[test]
fn test_about_kitty_json_includes_setting_locators_without_config() {
    let output = about_kitty_without_config()
        .args(["about", "kitty", "--json"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let settings = parsed
        .pointer("/config/settings")
        .and_then(|v| v.as_object())
        .expect("config.settings should be an object");

    let font_size = settings
        .get("font_size")
        .expect("font_size locator should be present");

    assert_eq!(
        font_size.get("path").and_then(|v| v.as_str()),
        Some("font_size")
    );
    assert!(font_size.get("value").is_some_and(serde_json::Value::is_null));

    assert_eq!(
        settings
            .get("ipc")
            .and_then(|setting| setting.get("path"))
            .and_then(|v| v.as_str()),
        Some("allow_remote_control")
    );
    assert_eq!(
        settings
            .get("opacity")
            .and_then(|setting| setting.get("path"))
            .and_then(|v| v.as_str()),
        Some("background_opacity")
    );
}

#[test]
fn test_about_kitty_json_uses_nested_config_contract() {
    let output = about_kitty_without_config()
        .args(["about", "kitty", "--json"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let config = parsed.get("config").expect("config should be present");

    assert_eq!(config.get("format").and_then(|v| v.as_str()), Some("KittyConf"));
    assert_eq!(
        config
            .pointer("/location_env/0/var")
            .and_then(|v| v.as_str()),
        Some("KITTY_CONFIG_DIRECTORY")
    );
    assert_eq!(
        config
            .pointer("/location_env/0/kind")
            .and_then(|v| v.as_str()),
        Some("dir")
    );
    assert!(config
        .pointer("/candidates/macos")
        .and_then(|v| v.as_array())
        .is_some_and(|candidates| !candidates.is_empty()));
    assert!(config.get("resolved_file").is_some_and(serde_json::Value::is_null));
    assert!(config.get("resolved_source").is_some_and(serde_json::Value::is_null));
}

#[test]
fn test_about_alacritty_json_reports_xdg_config_home_location_env() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .env_remove("TERM")
        .env_remove("XDG_CONFIG_HOME")
        .args(["about", "alacritty", "--json"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(
        parsed
            .pointer("/config/location_env/0/var")
            .and_then(|v| v.as_str()),
        Some("XDG_CONFIG_HOME")
    );
    assert_eq!(
        parsed
            .pointer("/config/location_env/0/kind")
            .and_then(|v| v.as_str()),
        Some("dir")
    );
}

#[test]
fn test_about_alacritty_json_extracts_legacy_yaml_candidate() {
    let config_home = tempfile::tempdir().expect("temp config home");
    let alacritty_dir = config_home.path().join("alacritty");
    std::fs::create_dir_all(&alacritty_dir).expect("alacritty config dir");
    let yaml_path = alacritty_dir.join("alacritty.yml");
    std::fs::write(
        &yaml_path,
        "font:\n  size: 15\n  normal:\n    family: CommitMono\nwindow:\n  opacity: 0.92\n",
    )
    .expect("write alacritty yaml config");

    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("XDG_CONFIG_HOME", config_home.path())
        .env("HOME", config_home.path())
        .env_remove("TERM")
        .args(["about", "alacritty", "--json"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(
        parsed.pointer("/config/resolved_file").and_then(|v| v.as_str()),
        Some(yaml_path.to_str().unwrap())
    );
    assert_eq!(setting_value(&parsed, "font_size"), Some("15"));
    assert_eq!(setting_value(&parsed, "font"), Some("CommitMono"));
    assert_eq!(setting_value(&parsed, "opacity"), Some("0.92"));
}

#[test]
fn test_about_warp_json_resolves_directory_without_none_format() {
    let home = tempfile::tempdir().expect("temp home");
    let warp_dir = home.path().join(".warp");
    std::fs::create_dir_all(&warp_dir).expect("warp config dir");

    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("HOME", home.path())
        .env_remove("TERM")
        .args(["about", "warp", "--json"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("unreadable"));

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(
        parsed.pointer("/config/format").and_then(|v| v.as_str()),
        Some("Yaml")
    );
    assert_ne!(
        parsed.pointer("/config/format").and_then(|v| v.as_str()),
        Some("None")
    );
    assert_eq!(
        parsed.pointer("/config/resolved_file").and_then(|v| v.as_str()),
        Some(warp_dir.to_str().unwrap())
    );
    assert_eq!(
        parsed
            .pointer("/config/resolved_source/candidate")
            .and_then(|v| v.as_u64()),
        Some(0)
    );

    for key in ["ipc", "font", "font_size", "theme", "background_color", "opacity"] {
        let value = parsed
            .pointer(&format!("/config/settings/{key}/value"))
            .expect("core Warp setting should be present");
        assert!(
            value.is_null(),
            "Warp setting {key} should remain locator-only"
        );
    }
}

#[test]
fn test_about_prefix_match() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["about", "ki", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("About Kitty"));
}

#[test]
fn test_about_alias_match() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["about", "VSCode", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("About VS Code"));
}

#[test]
fn test_about_invalid_app_exits_with_code_2() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["about", "notreal"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("Unknown terminal app"))
        .stderr(predicate::str::contains("Valid apps"));
}

#[test]
fn test_about_defaults_to_current_terminal() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("TERM_PROGRAM", "kitty")
        .env_remove("TERM")
        .args(["about", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("About Kitty"))
        .stdout(predicate::str::contains("Current Terminal: yes"));
}

#[test]
fn test_about_non_current_terminal_plain_includes_env_fact_candidates() {
    about_kitty_as_non_current()
        .args(["about", "kitty", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Environment Facts"))
        .stdout(predicate::str::contains("KITTY_PID"))
        .stdout(predicate::str::contains("KITTY_LISTEN_ON"))
        .stdout(predicate::str::contains("(not current terminal)"));
}

#[test]
fn test_about_current_terminal_plain_includes_env_fact_candidates_and_values() {
    about_kitty_as_current()
        .args(["about", "kitty", "--plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Current Terminal: yes"))
        .stdout(predicate::str::contains("KITTY_PID"))
        .stdout(predicate::str::contains("12345"))
        .stdout(predicate::str::contains("KITTY_LISTEN_ON"))
        .stdout(predicate::str::contains("unix:/tmp/bt-kitty.sock"));
}

#[test]
fn test_about_non_current_terminal_json_includes_env_fact_candidates_without_values() {
    let output = about_kitty_as_non_current()
        .args(["about", "kitty", "--json"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let pid = parsed
        .pointer("/env/pid")
        .expect("PID fact should be present");
    let vars = pid
        .get("vars")
        .and_then(|v| v.as_array())
        .expect("PID vars should be an array");

    assert_eq!(vars.first().and_then(|v| v.as_str()), Some("KITTY_PID"));
    assert!(pid.get("value").is_some_and(serde_json::Value::is_null));
}

#[test]
fn test_about_current_terminal_json_includes_env_fact_candidates_and_values() {
    let output = about_kitty_as_current()
        .args(["about", "kitty", "--json"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");
    let ipc = parsed
        .pointer("/env/ipc_address")
        .expect("IPC Address fact should be present");
    let vars = ipc
        .get("vars")
        .and_then(|v| v.as_array())
        .expect("IPC Address vars should be an array");

    assert_eq!(vars.first().and_then(|v| v.as_str()), Some("KITTY_LISTEN_ON"));
    assert_eq!(
        ipc.get("value").and_then(|v| v.as_str()),
        Some("unix:/tmp/bt-kitty.sock")
    );
}
