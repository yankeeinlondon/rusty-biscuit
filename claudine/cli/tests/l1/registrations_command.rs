//! The hidden `claudine registrations` command (`commands/registrations.rs`)
//! and the passivity of every other command toward the published
//! registration.
//!
//! Listing and printing read only the inventory compiled into the binary, so
//! they run in a home with no configuration at all and create nothing there.
//! `publish` installs the same set and prints the same listing. An ordinary
//! command (`claudine errors`, which loads the configuration and launches
//! nothing, and the other commands in the configured-startup table) never
//! reads, creates, or repairs anything under `~/.claudine/registrations`.
//!
//! Unchanged bytes and modification times cannot show a read, so in a
//! `test-fixtures` build every run also records the registration module's
//! file-system calls through the `CLAUDINE_TEST_REGISTRATION_ACCESS` seam
//! (`lib/src/composition/registration/access_log.rs`): an ordinary command
//! must leave that log empty, and the publishing routes (`claudine setup`,
//! `claudine registrations publish`) are the controls that fill it. Darkmatter's
//! loader of an installed bundle is outside that module, so a source scan
//! proves Claudine never calls it.

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

/// Where a run's registration access log is written: in the workspace, so
/// no snapshot of the home sees it.
fn access_log(fixture: &CliProcessFixture) -> PathBuf {
    fixture.workspace_path().join("registration-access.log")
}

/// Takes the access log the runs so far wrote: one `<operation>\t<path>`
/// line per file-system call of the registration module, empty when none.
fn take_accesses(fixture: &CliProcessFixture) -> Vec<String> {
    let log = access_log(fixture);
    let lines = std::fs::read_to_string(&log).unwrap_or_default().lines().map(str::to_string).collect();
    let _ = std::fs::remove_file(&log);
    lines
}

/// A `claudine` command that records the registration module's file-system
/// calls (`test-fixtures` builds only; otherwise the variable is ignored).
fn command(fixture: &CliProcessFixture) -> assert_cmd::Command {
    let mut command = fixture.command();
    command.env("CLAUDINE_TEST_REGISTRATION_ACCESS", access_log(fixture));
    command
}

/// Runs `claudine registrations <args>`; returns the exit success, stdout,
/// and stderr.
fn registrations(fixture: &CliProcessFixture, args: &[&str]) -> (bool, String, String) {
    let output = command(fixture).arg("registrations").args(args).output().expect("claudine runs");
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

/// Runs `claudine errors`, requiring success and, in a `test-fixtures`
/// build, no file-system call of the registration module; returns stderr.
fn run_ordinary(fixture: &CliProcessFixture) -> String {
    run_passive(fixture, &["errors"])
}

/// Runs `claudine <args>`, requiring success and, in a `test-fixtures`
/// build, an empty registration access log; returns stderr.
fn run_passive(fixture: &CliProcessFixture, args: &[&str]) -> String {
    let _ = take_accesses(fixture);
    let output = command(fixture).env("COLUMNS", "200").args(args).output().expect("claudine runs");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "claudine {args:?} failed:\n{stderr}");
    // The publishing controls below prove the seam records in this binary,
    // so an empty log means no call was made.
    #[cfg(feature = "test-fixtures")]
    assert_eq!(take_accesses(fixture), Vec::<String>::new(), "claudine {args:?} touched the registration");
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
    let _ = take_accesses(&fixture);

    let (success, second, stderr) = registrations(&fixture, &["publish"]);

    assert!(success, "the second publish failed:\n{stderr}");
    assert_eq!(second, first, "an already current installation prints the same listing");
    assert_eq!(snapshot(fixture.home()), installed, "bytes and modification times are unchanged");
    #[cfg(feature = "test-fixtures")]
    {
        let accesses = take_accesses(&fixture);
        for file in inventory().files() {
            let installed = file.path().split('/').fold(root.clone(), |path, name| path.join(name));
            let read = format!("read\t{}", installed.display());
            assert!(accesses.contains(&read), "the current publish reads `{}`:\n{accesses:#?}", file.path());
        }
        assert!(!accesses.iter().any(|line| line.starts_with("write\t")), "{accesses:#?}");
    }
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

/// A configured home after `claudine setup`: the bundle published and
/// registered in `~/.darkmatter.yaml`, as an installed Claudine leaves it.
/// Setup is the first control: it must record calls into the bundle.
#[cfg(feature = "test-fixtures")]
fn published_and_registered(prefix: &str) -> CliProcessFixture {
    let fixture = configured(prefix);
    let output = command(&fixture).arg("setup").write_stdin("").output().expect("claudine runs");
    assert!(output.status.success(), "setup failed:\n{}", String::from_utf8_lossy(&output.stderr));
    let registered = std::fs::read_to_string(fixture.home().join(".darkmatter.yaml")).unwrap();
    assert!(
        registered.contains(claudine::composition::registration::MANIFEST_REFERENCE),
        "setup registers the manifest:\n{registered}"
    );
    let manifest = claudine::composition::registration::manifest_path(fixture.home());
    let accesses = take_accesses(&fixture);
    assert!(
        accesses.contains(&format!("read\t{}", manifest.display())),
        "setup's publish reads the manifest:\n{accesses:#?}"
    );
    fixture
}

/// Configured startup reaches no registration code: no row's run makes a
/// single file-system call of the registration module, while setup on the
/// same home and a later explicit publish do.
#[cfg(feature = "test-fixtures")]
#[rstest::rstest]
#[case::errors(&["errors"])]
#[case::compose_dry_run(&["compose", "--dry-run", "prompt.md"])]
#[case::wrapper_dry_run(&["goose", "--dry-run", "hi"])]
#[case::completions(&["completions", "bash"])]
fn configured_startup_never_reads_the_published_bundle(#[case] args: &[&str]) {
    let fixture = published_and_registered("passive-startup-read");
    std::fs::write(fixture.cwd().join("prompt.md"), "---\nagent: goose\n---\nSay hi.\n").unwrap();
    let before = snapshot(&registrations_dir(fixture.home()));

    let stderr = run_passive(&fixture, args);

    assert!(!stderr.contains("registration"), "{stderr}");
    assert_eq!(snapshot(&registrations_dir(fixture.home())), before);
    let (success, _, stderr) = registrations(&fixture, &["publish"]);
    assert!(success, "{stderr}");
    assert!(
        take_accesses(&fixture).iter().any(|line| line.starts_with("read\t")),
        "the explicit publish, the second control, reads the installed bundle"
    );
}

/// Darkmatter loads an installed bundle outside Claudine's registration
/// module, where the access log cannot see it. Claudine's sources must
/// therefore never call that loader, and must locate the published tree only
/// inside the module (and setup, which writes its `register` entry).
#[test]
fn only_the_registration_module_locates_or_loads_an_installed_bundle() {
    const DARKMATTER_LOADERS: &[&str] =
        &["RegisteredHosts::load", "load_registrations", "load_entries", "load_snapshot", "load_snapshot_with"];
    // A locator function counts where it is called, so a field of the same
    // name elsewhere (`provider_overlay/lease.rs`'s `lock_path`) is not one.
    const LOCATORS: &[&str] =
        &["registrations_dir(", "output_root(", "manifest_path(", "lock_path(", "MANIFEST_REFERENCE"];
    const SETUP: &[&str] = &["lib/src/config/setup.rs", "cli/src/commands/init_wizard.rs"];

    let area = biscuit_test_harness::manifest_dir!().parent().expect("cli crate has a package area").to_path_buf();
    let mut files = Vec::new();
    for sub in ["lib/src", "cli/src"] {
        let mut pending = vec![area.join(sub)];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display())) {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.extension().is_some_and(|extension| extension == "rs") {
                    files.push(path);
                }
            }
        }
    }
    assert!(files.len() > 100, "the scan reached both crates: {} files", files.len());

    let mut found = Vec::new();
    for path in files {
        let relative = path.strip_prefix(&area).unwrap().to_string_lossy().replace('\\', "/");
        let in_module = relative == "lib/src/composition/registration.rs"
            || relative.starts_with("lib/src/composition/registration/");
        let source = std::fs::read_to_string(&path).unwrap();
        for (number, line) in source.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for name in DARKMATTER_LOADERS.iter().chain(if in_module { &[][..] } else { LOCATORS }) {
                if *name == "MANIFEST_REFERENCE" && SETUP.contains(&relative.as_str()) {
                    continue;
                }
                if names_identifier(line, name) {
                    found.push(format!("{relative}:{}: `{name}`", number + 1));
                }
            }
        }
    }
    assert!(found.is_empty(), "an installed bundle is located or loaded outside the registration module:\n{}", found.join("\n"));
}

/// Whether `line` holds `name` with no identifier character on either side
/// (on the left only, when `name` ends in its call's parenthesis).
fn names_identifier(line: &str, name: &str) -> bool {
    let is_identifier = |c: char| c.is_alphanumeric() || c == '_';
    line.match_indices(name).any(|(start, _)| {
        !line[..start].chars().next_back().is_some_and(is_identifier)
            && (name.ends_with('(') || !line[start + name.len()..].chars().next().is_some_and(is_identifier))
    })
}
