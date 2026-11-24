//! `claudine doctor` through the shipped binary: help and usage, the exit
//! status of each report shape, the JSON contract against the library's
//! rows, the human report's plain rendering, redaction, and that an
//! inspection of a missing or broken home changes nothing.
//!
//! Each row's own outcomes are the library's (`lib/src/doctor/*_tests.rs`);
//! these tests prove the command reaches them before configuration loads.
//! The `tts` and `path` rows read this host, so only their presence, order,
//! and (for `path`) the deliberate miss or hit are asserted here. On native
//! Windows every run but the healthy-home case skips the real shell probes
//! (`SKIP_SHELL_PROBES`): each costs seconds, and their outcomes are the
//! library's `path` tests.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use claudine::composition::registration::{
    MANIFEST_REFERENCE, RegistrationInventory, legacy_manifest_path, legacy_revisions_dir, publish, registrations_dir,
};
use claudine::doctor::DoctorRow;
use darkmatter::markdown::compose::RequestSnapshot;
use serde_json::Value;

use crate::common;
use common::CliProcessFixture;

const IDS: [&str; 7] = ["config", "registration", "darkmatter-permissions", "repo-config", "stray-config-keys", "tts", "path"];

/// A Claudine configuration without `tts` (hook speech off, so the `tts`
/// row is never above `info`) and with its own `permissions` block.
const CLAUDINE_CONFIG: &str = "# mine\nlogging: false   # quiet\n\npermissions:\n  network: deny\n";

fn darkmatter_config() -> String {
    format!("# mine\npermissions: ~/.claudine/config.yaml\nregister:\n  - {MANIFEST_REFERENCE}\n")
}

/// A home with nothing for doctor to report: both files, the published
/// registration, and a `claudine` on the fixture `PATH`.
fn healthy(prefix: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(prefix);
    common::write(&fixture.home().join(".claudine/config.yaml"), CLAUDINE_CONFIG);
    common::write(&fixture.home().join(".darkmatter.yaml"), &darkmatter_config());
    publish(fixture.home(), RegistrationInventory::claudine()).expect("publish the registration");
    install_claudine_on_path(&fixture);
    fixture
}

/// The `path` row only looks the name up; it never runs what it finds.
fn install_claudine_on_path(fixture: &CliProcessFixture) {
    common::write_executable(&fixture.bin_dir().join(format!("claudine{}", std::env::consts::EXE_SUFFIX)), "#!/bin/sh\n");
}

/// The `test-fixtures` seam that keeps a child from probing the host's
/// Windows shells; it changes nothing on other OSes.
const SKIP_SHELL_PROBES: &str = "CLAUDINE_TEST_DOCTOR_SKIP_SHELL_PROBES";

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn doctor(fixture: &CliProcessFixture, args: &[&str]) -> Run {
    let mut command = fixture.command();
    run(command.env(SKIP_SHELL_PROBES, "1").env("COLUMNS", "300").arg("doctor").args(args))
}

fn run(command: &mut assert_cmd::Command) -> Run {
    let output = command.write_stdin("").output().expect("claudine runs");
    Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn rows(run: &Run) -> Vec<Value> {
    let parsed: Value = serde_json::from_str(&run.stdout).unwrap_or_else(|error| panic!("{error}: {}", run.stdout));
    let rows = parsed.as_array().expect("one JSON array").clone();
    let ids: Vec<&str> = rows.iter().map(|row| row["id"].as_str().unwrap()).collect();
    assert_eq!(ids, IDS, "one row per check, in order");
    for row in &rows {
        let keys: Vec<&str> = row.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys.len(), 5, "{row}");
        for key in ["id", "severity", "summary", "detail", "fixable"] {
            assert!(keys.contains(&key), "{key} missing from {row}");
        }
        assert!(row["fixable"].is_boolean() && row["detail"].is_string(), "{row}");
        assert!(!row["summary"].as_str().unwrap().contains('\n'), "{row}");
    }
    rows
}

fn row<'a>(rows: &'a [Value], id: &str) -> &'a Value {
    rows.iter().find(|row| row["id"] == id).unwrap()
}

fn severity(rows: &[Value], id: &str) -> String {
    row(rows, id)["severity"].as_str().unwrap().to_string()
}

/// Every file and directory under `root`, with each file's bytes. A writer
/// lock sidecar is listed without its (empty) bytes: Windows refuses to read
/// a file another handle holds locked.
fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut entries = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for child in std::fs::read_dir(&dir).unwrap() {
            let path = child.unwrap().path();
            if std::fs::symlink_metadata(&path).unwrap().is_dir() {
                pending.push(path.clone());
                entries.insert(path, None);
            } else if path.to_string_lossy().ends_with(darkmatter::config::write::LOCK_SUFFIX) {
                entries.insert(path, Some(Vec::new()));
            } else {
                entries.insert(path.clone(), Some(std::fs::read(&path).unwrap()));
            }
        }
    }
    entries
}

// ------------------------------------------------------------- help, usage

#[test]
fn help_names_both_options_and_needs_no_configuration() {
    let fixture = CliProcessFixture::named("doctor-help").without_darkmatter_config();

    let help = doctor(&fixture, &["--help"]);

    assert_eq!(help.code, Some(0), "{}", help.stderr);
    for option in ["--fix", "--json"] {
        assert!(help.stdout.contains(option), "{option}:\n{}", help.stdout);
    }
    let overview = run(fixture.command().env("COLUMNS", "300").arg("--help"));
    assert_eq!(overview.code, Some(0), "{}", overview.stderr);
    let administration = overview.stdout.find("Administration:").expect(&overview.stdout);
    let listed = overview.stdout.find("doctor").expect(&overview.stdout);
    assert!(listed > administration, "{}", overview.stdout);
    assert!(tree(fixture.home()).is_empty(), "help writes nothing");
}

#[test]
fn an_unknown_option_is_a_usage_error_with_nothing_reported() {
    let fixture = CliProcessFixture::named("doctor-usage");

    let refused = doctor(&fixture, &["--repair"]);

    assert_eq!(refused.code, Some(2), "{}", refused.stderr);
    assert!(refused.stdout.is_empty(), "{}", refused.stdout);
    assert!(refused.stderr.contains("--repair"), "{}", refused.stderr);
}

// ---------------------------------------------------------- report shapes

#[test]
fn a_healthy_home_reports_every_row_and_exits_zero() {
    let fixture = healthy("doctor-healthy");
    fixture.initialize_repository();
    let before = tree(fixture.home());

    // The one spawned case with the host's real Windows shell probes; their
    // `ok` or `warn` findings never fail the run.
    let mut command = fixture.command();
    let report = run(command.env("COLUMNS", "300").args(["doctor", "--json"]));

    assert_eq!(report.code, Some(0), "{}\n{}", report.stdout, report.stderr);
    let rows = rows(&report);
    for id in ["config", "registration", "stray-config-keys"] {
        assert_eq!(severity(&rows, id), "ok", "{}", row(&rows, id));
    }
    // Without repository files, only the user scope's handoff is assessed.
    assert_eq!(severity(&rows, "darkmatter-permissions"), "info", "{}", row(&rows, "darkmatter-permissions"));
    assert!(row(&rows, "darkmatter-permissions")["detail"].as_str().unwrap().contains("refers to Claudine's configuration"));
    assert_eq!(severity(&rows, "repo-config"), "info", "an absent repository file is informational");
    assert!(["ok", "info"].contains(&severity(&rows, "tts").as_str()), "hook speech is off: {}", row(&rows, "tts"));
    assert!(["ok", "warn"].contains(&severity(&rows, "path").as_str()), "{}", row(&rows, "path"));
    assert!(rows.iter().all(|row| row["fixable"] == false), "{rows:#?}");
    // A probed PowerShell refreshes its own startup cache under
    // `%LOCALAPPDATA%` (here the fixture home); doctor itself writes nothing.
    let mut after = tree(fixture.home());
    after.retain(|path, _| !path.starts_with(fixture.home().join("AppData")));
    assert_eq!(after, before, "an inspection writes nothing");
}

#[test]
fn warnings_alone_exit_zero_and_both_formats_show_the_library_rows() {
    let fixture = healthy("doctor-warnings");
    common::write(&legacy_manifest_path(fixture.home()), "kind: darkmatter-host\nformat_version: 1\n");

    let report = doctor(&fixture, &["--json"]);

    assert_eq!(report.code, Some(0), "{}\n{}", report.stdout, report.stderr);
    let rows = rows(&report);
    assert_eq!(severity(&rows, "registration"), "warn", "{}", row(&rows, "registration"));
    assert_eq!(row(&rows, "registration")["fixable"], true, "the leftover can be cleaned up");
    assert!(rows.iter().all(|row| row["severity"] != "error"), "{rows:#?}");

    // The file rows are this host's library result exactly; `tts` and
    // `path` read the process environment, which differs in the child.
    let snapshot = RequestSnapshot::new(fixture.cwd()).with_home(Some(fixture.home().to_path_buf()));
    let library: Vec<DoctorRow> = claudine::doctor::run(&snapshot, false);
    let library: Value = serde_json::to_value(&library).unwrap();
    assert_eq!(&rows[..5], &library.as_array().unwrap()[..5]);

    // The human report: the same rows, plain when redirected even with
    // color forced, and plain under `--plain`.
    for args in [&[][..], &["--plain"][..]] {
        let mut command = fixture.command();
        let human = run(command.env(SKIP_SHELL_PROBES, "1").env("COLUMNS", "300").env_remove("NO_COLOR").env("FORCE_COLOR", "1").arg("doctor").args(args));
        assert_eq!(human.code, Some(0), "{args:?}: {}", human.stderr);
        assert!(!human.stdout.contains('\u{1b}'), "{args:?} has ANSI:\n{}", human.stdout);
        let mut at = 0;
        for row in &rows {
            let head = format!("[{}] {} ({}): ", row["severity"].as_str().unwrap(), claudine::doctor::title_of(row["id"].as_str().unwrap()).unwrap(), row["id"].as_str().unwrap());
            let found = human.stdout[at..].find(&head).unwrap_or_else(|| panic!("{head} after byte {at}:\n{}", human.stdout));
            at += found + head.len();
        }
        let registration = row(&rows, "registration");
        for line in registration["detail"].as_str().unwrap().lines() {
            assert!(human.stdout.contains(line), "{line}:\n{}", human.stdout);
        }
        assert!(human.stdout.trim_end().ends_with("0 errors, 1 warning."), "{}", human.stdout);
    }
}

#[test]
fn inspecting_a_missing_home_reports_errors_and_creates_nothing() {
    let fixture = CliProcessFixture::named("doctor-missing").without_darkmatter_config();
    let mut command = fixture.command_builder()
        // The deliberate miss: nothing but the empty fixture `bin` is searched.
        .fake_only_path()
        .build();

    let report = run(command.env(SKIP_SHELL_PROBES, "1").env("COLUMNS", "300").args(["doctor", "--json"]));

    assert_eq!(report.code, Some(1), "{}\n{}", report.stdout, report.stderr);
    let rows = rows(&report);
    for id in ["config", "registration", "path"] {
        assert_eq!(severity(&rows, id), "error", "{}", row(&rows, id));
    }
    assert!(row(&rows, "config")["detail"].as_str().unwrap().contains("claudine setup"), "{rows:#?}");
    assert!(tree(fixture.home()).is_empty(), "no configuration, backup, or registration is created");
    assert!(!fixture.audio_spool().exists(), "no worker or audio job is started");
    assert!(!report.stderr.contains("setup"), "no setup guidance or prompt:\n{}", report.stderr);
}

#[test]
fn inspecting_a_broken_home_leaves_every_file_as_found() {
    let fixture = CliProcessFixture::named("doctor-broken");
    install_claudine_on_path(&fixture);
    // A legacy JSON configuration with no YAML beside it, and an invalid
    // Darkmatter file: neither is converted, rewritten, or refreshed.
    common::write(&fixture.home().join(".claudine/config.json"), "{ \"logging\": true }\n");
    common::write(&fixture.home().join(".darkmatter.yaml"), "register: [\n");
    let before = tree(fixture.home());

    let report = doctor(&fixture, &["--json"]);

    assert_eq!(report.code, Some(1), "{}\n{}", report.stdout, report.stderr);
    let rows = rows(&report);
    assert_eq!(severity(&rows, "config"), "error", "{}", row(&rows, "config"));
    assert!(row(&rows, "config")["detail"].as_str().unwrap().contains("config.json"), "{}", row(&rows, "config"));
    assert_eq!(tree(fixture.home()), before, "nothing is converted, created, or published");
    assert!(!registrations_dir(fixture.home()).exists());
    assert!(!fixture.audio_spool().exists());
}

#[test]
fn no_authored_value_reaches_either_report_or_stderr() {
    // A schema error quoting its value, and a broken permissions reference.
    let cases = [
        ("config", "logging: SECRET-SENTINEL-value\npermissions:\n  network: deny\n".to_string(), darkmatter_config()),
        (
            "darkmatter-permissions",
            CLAUDINE_CONFIG.to_string(),
            format!("permissions: ~/.SECRET-SENTINEL-reference.yaml\nregister:\n  - {MANIFEST_REFERENCE}\n"),
        ),
    ];
    for (id, claudine_config, darkmatter_text) in cases {
        let fixture = healthy("doctor-redaction");
        common::write(&fixture.home().join(".claudine/config.yaml"), &claudine_config);
        common::write(&fixture.home().join(".darkmatter.yaml"), &darkmatter_text);
        for args in [&["--json"][..], &[][..]] {
            let report = doctor(&fixture, args);
            assert_eq!(report.code, Some(1), "{id} {args:?}: {}\n{}", report.stdout, report.stderr);
            assert!(!report.stdout.contains("SECRET"), "{id} {args:?}:\n{}", report.stdout);
            assert!(!report.stderr.contains("SECRET"), "{id} {args:?}:\n{}", report.stderr);
            if args == ["--json"] {
                let rows = rows(&report);
                assert_eq!(severity(&rows, id), "error", "{}", row(&rows, id));
            }
        }
    }
}

// ----------------------------------------------------------------- repairs

#[test]
fn a_repaired_home_exits_zero_and_a_second_run_changes_nothing() {
    let fixture = healthy("doctor-repair");
    // Both repairs in one run: the registration is unpublished and
    // unregistered, and the permissions block still sits in Darkmatter's file.
    std::fs::remove_dir_all(registrations_dir(fixture.home())).unwrap();
    common::write(&fixture.home().join(".claudine/config.yaml"), "# mine\nlogging: false   # quiet\n");
    common::write(&fixture.home().join(".darkmatter.yaml"), "# theirs\npermissions:\n  network: deny\nregister: []\n");

    let inspected = doctor(&fixture, &["--json"]);
    assert_eq!(inspected.code, Some(1), "{}", inspected.stdout);
    let before = rows(&inspected);
    for id in ["registration", "darkmatter-permissions"] {
        assert_eq!(severity(&before, id), "error", "{}", row(&before, id));
        assert_eq!(row(&before, id)["fixable"], true, "{}", row(&before, id));
    }

    let repaired = doctor(&fixture, &["--fix", "--json"]);

    assert_eq!(repaired.code, Some(0), "{}\n{}", repaired.stdout, repaired.stderr);
    let after = rows(&repaired);
    for id in ["registration", "darkmatter-permissions"] {
        assert_eq!(severity(&after, id), "ok", "{}", row(&after, id));
        assert!(row(&after, id)["detail"].as_str().unwrap().contains("Repaired: "), "{}", row(&after, id));
    }
    let claudine_text = std::fs::read_to_string(fixture.home().join(".claudine/config.yaml")).unwrap();
    assert!(claudine_text.starts_with("# mine\nlogging: false   # quiet\n") && claudine_text.contains("network: deny"), "{claudine_text}");
    let darkmatter_text = std::fs::read_to_string(fixture.home().join(".darkmatter.yaml")).unwrap();
    assert!(darkmatter_text.starts_with("# theirs\n") && darkmatter_text.contains(MANIFEST_REFERENCE), "{darkmatter_text}");
    let settled = tree(fixture.home());

    let again = doctor(&fixture, &["--fix", "--json"]);

    assert_eq!(again.code, Some(0), "{}", again.stdout);
    assert!(rows(&again).iter().all(|row| !row["detail"].as_str().unwrap().contains("Repaired: ")), "{}", again.stdout);
    assert_eq!(tree(fixture.home()), settled, "a second repair is a no-op");
}

#[test]
fn a_repair_blocked_by_a_held_lock_fails_and_keeps_the_entry_and_the_legacy_files() {
    let fixture = healthy("doctor-repair-locked");
    let darkmatter = fixture.home().join(".darkmatter.yaml");
    common::write(&darkmatter, "permissions: ~/.claudine/config.yaml\nregister: []\n");
    common::write(&legacy_manifest_path(fixture.home()), "kind: darkmatter-host\nformat_version: 1\n");
    common::write(&legacy_revisions_dir(fixture.home()).join("0123abcd/globals.yaml"), "kind: schema\n");
    let mut sidecar = darkmatter.clone().into_os_string();
    sidecar.push(darkmatter::config::write::LOCK_SUFFIX);
    let lock = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&sidecar).unwrap();
    lock.lock().expect("the parent holds Darkmatter's writer lock");
    let before = tree(fixture.home());

    let report = doctor(&fixture, &["--fix", "--json"]);
    lock.unlock().unwrap();
    drop(lock);

    assert_eq!(report.code, Some(1), "{}\n{}", report.stdout, report.stderr);
    let rows = rows(&report);
    let registration = row(&rows, "registration");
    assert_eq!(registration["severity"], "error", "{registration}");
    assert!(registration["summary"].as_str().unwrap().starts_with("the repair failed: "), "{registration}");
    assert!(registration["detail"].as_str().unwrap().contains("lock"), "{registration}");
    assert_eq!(registration["fixable"], true, "a retry can still repair it: {registration}");
    assert_eq!(tree(fixture.home()), before, "the entry and the legacy files are kept");
}

// ------------------------------------------------------------ write errors

#[cfg(target_os = "linux")]
#[test]
fn a_failed_write_to_stdout_is_an_error() {
    let fixture = healthy("doctor-stdout-full");
    let full = std::fs::OpenOptions::new().write(true).open("/dev/full").unwrap();
    let mut command = fixture.command_std();
    let output = command.args(["doctor", "--json"]).env(SKIP_SHELL_PROBES, "1").stdout(full).stdin(std::process::Stdio::null()).output().unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error:") && stderr.to_lowercase().contains("space"), "{stderr}");
}
