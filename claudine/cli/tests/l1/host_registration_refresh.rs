//! The launch refresh of Claudine's Darkmatter host registration
//! (`main.rs::refresh_host_registration`): a substantive command publishes
//! the binary's revision under the fixture home when it is missing or stale,
//! writes nothing when it is current, only warns when it cannot publish, and
//! never registers itself in `~/.darkmatter.yaml` (which the startup load
//! creates at its defaults when it is missing). Hooks, `steer`, and completion stay
//! passive. `claudine errors` is the substantive command: it needs an
//! existing configuration and launches nothing.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use biscuit_file::FileResolutionContext;
use claudine::composition::registration::{HostRevision, MANIFEST_REFERENCE, manifest_path, revisions_dir};
use darkmatter::config::load::{ConfigRevision, Registration};
use darkmatter::config::registration::{load_entries, md_owned_names};

use crate::common;
use common::CliProcessFixture;

const REFRESH_WARNING: &str = "host registration was not refreshed";

/// A fixture with a valid Claudine configuration, so no first-run wizard
/// starts.
fn configured(prefix: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(prefix);
    fixture.seed_user_config();
    fixture
}

/// Runs `claudine errors`, requiring success; returns stderr.
fn run_substantive(fixture: &CliProcessFixture) -> String {
    let output = fixture.command().env("COLUMNS", "200").arg("errors").output().expect("claudine runs");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "claudine errors failed:\n{stderr}");
    stderr
}

/// Every file under `dir` with its bytes and modification time; empty when
/// `dir` does not exist.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
    let mut files = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&next) else { continue };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
                files.insert(path.clone(), (std::fs::read(&path).unwrap(), modified));
            }
        }
    }
    files
}

/// The manifest's text when it names `revision`'s first directory.
fn expected_manifest() -> String {
    let revision = HostRevision::claudine();
    revision.manifest_text(&revision.directory_name())
}

/// Loads the published manifest through Darkmatter's registration loader as
/// a user `register` entry, requiring one `claudine` profile.
fn assert_loads(home: &Path) {
    let entry = Registration {
        reference: MANIFEST_REFERENCE.to_string(),
        index: 0,
        declared_in: home.join(".darkmatter.yaml"),
        revision: ConfigRevision::User,
        at: None,
    };
    let context = FileResolutionContext::from_snapshot(home.to_path_buf(), Some(home.to_path_buf()), HashMap::new());
    let loaded = load_entries(&[entry], &context, md_owned_names());
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    assert_eq!(loaded.profiles.len(), 1, "one profile");
    assert_eq!(loaded.profiles[0].host(), "claudine");
    assert_eq!(loaded.profiles[0].version(), HostRevision::claudine().version());
}

/// Whether anything named `darkmatter-host*` exists under `~/.claudine`.
fn publication_exists(home: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(home.join(".claudine")) else { return false };
    entries
        .flatten()
        .any(|entry| entry.file_name().to_string_lossy().starts_with("darkmatter-host"))
}

#[test]
fn host_registration_refresh_first_launch_publishes_and_registers_nothing_in_darkmatter_config() {
    let fixture = configured("host-refresh-first");
    std::fs::remove_file(fixture.home().join(".darkmatter.yaml")).unwrap();

    run_substantive(&fixture);

    let home = fixture.home();
    assert_eq!(std::fs::read_to_string(manifest_path(home)).unwrap(), expected_manifest());
    let revision = revisions_dir(home).join(HostRevision::claudine().directory_name());
    assert!(revision.join("schemas").is_dir(), "{} exists", revision.display());
    assert_loads(home);
    // The startup load creates the missing file at its defaults; the refresh
    // adds nothing to it.
    let config = std::fs::read_to_string(home.join(".darkmatter.yaml")).expect("created at its defaults");
    assert!(config.contains("register: []"), "the refresh never registers in ~/.darkmatter.yaml: {config}");
    assert!(!home.join("schemas").exists(), "nothing is copied into ~/schemas");
}

#[test]
fn host_registration_refresh_second_launch_writes_nothing() {
    let fixture = configured("host-refresh-repeat");
    run_substantive(&fixture);
    let before = snapshot(fixture.home());
    assert!(publication_exists(fixture.home()));

    run_substantive(&fixture);

    assert_eq!(snapshot(fixture.home()), before, "bytes and modification times are unchanged");
}

#[test]
fn host_registration_refresh_replaces_a_stale_manifest() {
    let fixture = configured("host-refresh-stale");
    run_substantive(&fixture);
    let manifest = manifest_path(fixture.home());
    let stale = expected_manifest().replace(
        &format!("version: \"{}\"", HostRevision::claudine().version()),
        "version: \"0.0.1\"",
    );
    assert_ne!(stale, expected_manifest(), "the stale manifest differs");
    std::fs::write(&manifest, &stale).unwrap();

    run_substantive(&fixture);

    assert_eq!(std::fs::read_to_string(&manifest).unwrap(), expected_manifest());
    assert_loads(fixture.home());
}

#[test]
fn host_registration_refresh_failure_warns_and_keeps_the_previous_manifest() {
    let fixture = configured("host-refresh-obstructed");
    run_substantive(&fixture);
    let manifest = manifest_path(fixture.home());
    let previous = std::fs::read(&manifest).unwrap();
    let revisions = revisions_dir(fixture.home());
    std::fs::remove_dir_all(&revisions).unwrap();
    std::fs::write(&revisions, "an obstruction").unwrap();

    let stderr = run_substantive(&fixture);

    assert!(stderr.contains(REFRESH_WARNING), "the warning names the refresh:\n{stderr}");
    assert_eq!(std::fs::read(&manifest).unwrap(), previous, "the previous manifest is unchanged");
    assert_eq!(std::fs::read_to_string(&revisions).unwrap(), "an obstruction");
}

#[test]
fn host_registration_refresh_hooks_steer_and_completion_publish_nothing() {
    let fixture = configured("host-refresh-passive");

    fixture.command().args(["completions", "bash"]).assert().success();
    fixture
        .command()
        .args(["handle", "before_prompt", "--provider", "claude"])
        .write_stdin(r#"{"hook_event_name":"UserPromptSubmit","prompt":"hi","session_id":"refresh-test"}"#)
        .assert()
        .success();
    // Without a daemon the listing fails; it must still publish nothing.
    let _ = fixture
        .command()
        .env("CLAUDE_CONFIG_DIR", fixture.home().join(".claude"))
        .args(["steer", "--list", "--json"])
        .output()
        .expect("claudine runs");

    assert!(!publication_exists(fixture.home()), "no ~/.claudine/darkmatter-host* was written");
}
