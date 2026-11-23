//! The hidden `claudine registrations` command (`commands/registrations.rs`)
//! and the passivity of every other command toward the published
//! registration.
//!
//! Listing and printing read only the inventory compiled into the binary, so
//! they run in a home with no configuration at all and create nothing there.
//! `publish` installs the same set and prints the same listing. An ordinary
//! command (`claudine errors`, which loads the configuration and launches
//! nothing) never reads, creates, or repairs anything under
//! `~/.claudine/registrations`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use biscuit_hash::xx_hash_bytes;
use claudine::composition::registration::{
    FileKind, PublishedFile, RegistrationInventory, output_root, registrations_dir,
};
use serde_json::{Value, json};

use crate::common;
use common::CliProcessFixture;

fn inventory() -> &'static RegistrationInventory {
    RegistrationInventory::claudine()
}

fn hash_of(bytes: &[u8]) -> String {
    format!("{:016x}", xx_hash_bytes(bytes))
}

/// The `file` record the listing prints for `file`.
fn expected_entry(file: &PublishedFile) -> Value {
    let mut entry = json!({
        "kind": file.kind().as_str(),
        "filepath": file.path(),
        "hash": hash_of(file.bytes()),
    });
    if let Some(name) = file.name() {
        entry["name"] = json!(name);
    }
    entry
}

fn expected_listing() -> Value {
    Value::Array(inventory().files().iter().map(expected_entry).collect())
}

/// Runs `claudine registrations <args>`; returns the exit success, stdout,
/// and stderr.
fn registrations(fixture: &CliProcessFixture, args: &[&str]) -> (bool, String, String) {
    let output = fixture.command().arg("registrations").args(args).output().expect("claudine runs");
    (
        output.status.success(),
        String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Parses machine output: one pretty JSON document and one trailing newline,
/// nothing else.
fn parse_json(stdout: &str) -> Value {
    let body = stdout.strip_suffix('\n').expect("output ends with a newline");
    assert!(!body.ends_with('\n'), "exactly one trailing newline");
    assert!(body.contains("\n  "), "pretty-printed:\n{stdout}");
    serde_json::from_str(body).unwrap_or_else(|error| panic!("stdout is one JSON document ({error}):\n{stdout}"))
}

/// A fixture with a valid Claudine configuration, so an ordinary command
/// starts no first-run setup.
fn configured(prefix: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(prefix);
    fixture.seed_user_config();
    fixture
}

/// Runs `claudine errors`, requiring success; returns stderr.
fn run_ordinary(fixture: &CliProcessFixture) -> String {
    let output = fixture.command().env("COLUMNS", "200").arg("errors").output().expect("claudine runs");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "claudine errors failed:\n{stderr}");
    stderr
}

/// Every entry under `dir` (not following links) with its bytes, or its link
/// target, and its modification time; empty when `dir` does not exist.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
    let mut entries = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let Ok(children) = std::fs::read_dir(&next) else { continue };
        for child in children {
            let path = child.unwrap().path();
            let metadata = std::fs::symlink_metadata(&path).unwrap();
            let modified = metadata.modified().unwrap();
            if metadata.is_dir() {
                entries.insert(path.clone(), (Vec::new(), modified));
                pending.push(path);
            } else if metadata.file_type().is_symlink() {
                let target = std::fs::read_link(&path).unwrap();
                entries.insert(path, (target.to_string_lossy().into_owned().into_bytes(), modified));
            } else {
                entries.insert(path.clone(), (std::fs::read(&path).unwrap(), modified));
            }
        }
    }
    entries
}

#[test]
fn registrations_lists_every_published_file_sorted_with_its_hash_and_declared_name() {
    let fixture = CliProcessFixture::named("registrations-list").without_darkmatter_config();
    let before = snapshot(fixture.home());

    let (success, stdout, stderr) = registrations(&fixture, &[]);

    assert!(success, "listing failed:\n{stderr}");
    assert!(stderr.is_empty(), "no warning or log line accompanies the listing:\n{stderr}");
    let listed = parse_json(&stdout);
    assert_eq!(listed, expected_listing());
    let entries = listed.as_array().unwrap();
    let paths: Vec<&str> = entries.iter().map(|entry| entry["filepath"].as_str().unwrap()).collect();
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    assert_eq!(paths, sorted, "sorted by filepath");
    let kinds: Vec<&str> = entries.iter().map(|entry| entry["kind"].as_str().unwrap()).collect();
    for kind in [FileKind::Manifest, FileKind::Lifecycle] {
        assert!(kinds.contains(&kind.as_str()), "the listing includes the {} file", kind.as_str());
    }
    for entry in entries {
        let hash = entry["hash"].as_str().unwrap();
        assert!(hash.len() == 16 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')), "{hash}");
        assert!(entry.get("name").is_none_or(Value::is_string), "{entry}");
    }
    assert!(entries.iter().any(|entry| entry.get("name").is_some()), "some member declares a name");
    assert!(entries.iter().any(|entry| entry.get("name").is_none()), "`name` is omitted where none is declared");
    // No configuration is read or created, and nothing is installed.
    assert_eq!(snapshot(fixture.home()), before);
}

#[test]
fn registrations_detail_prints_the_exact_contents_of_a_listed_file() {
    let fixture = CliProcessFixture::named("registrations-detail").without_darkmatter_config();
    let named = inventory().files().iter().find(|file| file.name().is_some()).expect("a named member");
    let manifest = inventory().manifest();
    let lifecycle = inventory()
        .files()
        .iter()
        .find(|file| file.kind() == FileKind::Lifecycle)
        .expect("the lifecycle artifact");

    for file in [manifest, lifecycle, named] {
        let (success, stdout, stderr) = registrations(&fixture, &[file.path()]);

        assert!(success, "printing {} failed:\n{stderr}", file.path());
        assert!(stderr.is_empty(), "{stderr}");
        let details = parse_json(&stdout);
        let contents = details["contents"].as_str().expect("contents is text");
        assert_eq!(contents.as_bytes(), file.bytes(), "{} round-trips to its exact bytes", file.path());
        let mut expected = expected_entry(file);
        expected["contents"] = json!(contents);
        assert_eq!(details, expected);
    }
    assert!(fixture.home().read_dir().unwrap().next().is_none(), "nothing is written to the home");
}

#[test]
fn registrations_refuses_a_path_not_spelled_exactly_as_listed() {
    let fixture = CliProcessFixture::named("registrations-unknown");
    assert!(inventory().file("manifest.yaml").is_some());
    // The publish token is never a member, so it can only mean publish.
    assert!(inventory().file("publish").is_none());

    let (success, stdout, stderr) = registrations(&fixture, &["./manifest.yaml"]);

    assert!(!success, "an unknown path exits nonzero");
    assert!(stdout.is_empty(), "nothing on stdout:\n{stdout}");
    assert!(stderr.contains("./manifest.yaml"), "stderr names the path:\n{stderr}");
}

#[test]
fn registrations_publish_installs_the_listing_and_a_second_publish_changes_nothing() {
    let fixture = CliProcessFixture::named("registrations-publish").without_darkmatter_config();

    let (success, first, stderr) = registrations(&fixture, &["publish"]);

    assert!(success, "publish failed:\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(parse_json(&first), expected_listing());
    let root = output_root(fixture.home());
    for file in inventory().files() {
        let installed = std::fs::read(root.join(file.path()))
            .unwrap_or_else(|error| panic!("{} is installed: {error}", file.path()));
        assert_eq!(installed, file.bytes(), "{} holds the listed bytes", file.path());
    }
    // Publishing changes no configuration.
    assert!(!fixture.home().join(".darkmatter.yaml").exists());
    assert!(!fixture.home().join(".claudine").join("config.yaml").exists());
    let installed = snapshot(fixture.home());

    let (success, second, stderr) = registrations(&fixture, &["publish"]);

    assert!(success, "the second publish failed:\n{stderr}");
    assert_eq!(second, first, "an already current installation prints the same listing");
    assert_eq!(snapshot(fixture.home()), installed, "bytes and modification times are unchanged");
}

#[test]
fn registrations_publish_failure_exits_nonzero_with_the_cause_on_stderr() {
    let fixture = CliProcessFixture::named("registrations-publish-obstructed");
    let obstruction = registrations_dir(fixture.home());
    std::fs::create_dir_all(obstruction.parent().unwrap()).unwrap();
    std::fs::write(&obstruction, "an obstruction").unwrap();

    let (success, stdout, stderr) = registrations(&fixture, &["publish"]);

    assert!(!success, "an obstructed publish exits nonzero");
    assert!(stdout.is_empty(), "nothing on stdout:\n{stdout}");
    assert!(stderr.contains("not a directory"), "stderr reports the cause:\n{stderr}");
    assert_eq!(std::fs::read_to_string(&obstruction).unwrap(), "an obstruction");
}

#[test]
fn ordinary_commands_leave_an_obstructed_registration_path_untouched() {
    let fixture = configured("registrations-passive-file");
    let obstruction = registrations_dir(fixture.home());
    std::fs::write(&obstruction, "a sentinel the publisher would refuse").unwrap();
    let before = snapshot(&fixture.home().join(".claudine"));

    let stderr = run_ordinary(&fixture);

    assert!(!stderr.contains("registration"), "nothing reports on the registration:\n{stderr}");
    assert_eq!(
        snapshot(&fixture.home().join(".claudine")),
        before,
        "no file under ~/.claudine was created or changed"
    );
}

#[cfg(unix)]
#[test]
fn ordinary_commands_never_follow_a_registration_symlink() {
    let fixture = configured("registrations-passive-link");
    let sentinel = fixture.workspace_path().join("sentinel");
    std::fs::create_dir(&sentinel).unwrap();
    std::os::unix::fs::symlink(&sentinel, registrations_dir(fixture.home())).unwrap();
    let modified = std::fs::metadata(&sentinel).unwrap().modified().unwrap();

    let stderr = run_ordinary(&fixture);

    assert!(!stderr.contains("registration"), "{stderr}");
    assert!(sentinel.read_dir().unwrap().next().is_none(), "nothing was written through the link");
    assert_eq!(std::fs::metadata(&sentinel).unwrap().modified().unwrap(), modified);
    assert!(std::fs::symlink_metadata(registrations_dir(fixture.home())).unwrap().file_type().is_symlink());
}

#[test]
fn ordinary_commands_repair_neither_missing_nor_stale_members() {
    let fixture = configured("registrations-passive-damage");
    let (success, _, stderr) = registrations(&fixture, &["publish"]);
    assert!(success, "{stderr}");
    let root = output_root(fixture.home());
    let mut members = inventory().members();
    let missing = root.join(members.next().expect("a member").path());
    let stale = root.join(members.next().expect("a second member").path());
    std::fs::remove_file(&missing).unwrap();
    std::fs::write(&stale, "stale: true\n").unwrap();
    let damaged = snapshot(&registrations_dir(fixture.home()));

    let stderr = run_ordinary(&fixture);

    assert!(!stderr.contains("registration"), "{stderr}");
    assert_eq!(snapshot(&registrations_dir(fixture.home())), damaged, "the damage is left as it was");
    assert!(!missing.exists());
    assert_eq!(std::fs::read_to_string(&stale).unwrap(), "stale: true\n");
}
