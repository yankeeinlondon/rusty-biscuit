//! `claudine setup` (`commands/setup.rs`): the repeatable, non-interactive
//! route to Claudine's setup with Darkmatter on a home that is already
//! configured. It is the command Darkmatter's format 1 warning names, so an
//! upgraded install can replace the retired `~/.claudine/darkmatter-host.yaml`
//! entry. The ordering and handshake themselves are the library's
//! (`lib/tests/l1/installed_registration.rs`); these tests prove the CLI
//! reaches them and keeps the user's settings.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use claudine::composition::registration::{
    LEGACY_MANIFEST_REFERENCE, MANIFEST_REFERENCE, RegistrationInventory, legacy_manifest_path, legacy_revisions_dir,
    manifest_path, registrations_dir,
};

use crate::common;
use common::CliProcessFixture;

/// A Claudine configuration with settings of its own, comments, and a
/// `permissions` block, so the handshake has nothing to add.
const CLAUDINE_CONFIG: &str = "# mine\nlogging: false   # quiet\n\npermissions:\n  network: deny\n";

/// `~/.darkmatter.yaml` pointing at Claudine's permissions and listing the
/// retired manifest between two entries of the user's own.
fn darkmatter_config() -> String {
    format!(
        "# mine\npermissions: ~/.claudine/config.yaml\nregister:\n  - ~/hosts/mine.yaml   # kept\n  - {LEGACY_MANIFEST_REFERENCE}   # Claudine's\n  - ~/hosts/gone.yaml\n"
    )
}

/// A home configured by an older Claudine whose retired manifest is gone.
fn configured(prefix: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(prefix);
    write(&fixture.home().join(".claudine/config.yaml"), CLAUDINE_CONFIG);
    write(&fixture.home().join(".darkmatter.yaml"), &darkmatter_config());
    fixture
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Runs `claudine <args>` with stdin closed; returns success, stdout, stderr.
fn claudine(fixture: &CliProcessFixture, args: &[&str]) -> (bool, String, String) {
    let output = fixture.command().env("COLUMNS", "300").args(args).write_stdin("").output().expect("claudine runs");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// The configuration files and the published tree, with bytes and
/// modification times (not following links).
fn snapshot(home: &Path) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
    let mut entries = BTreeMap::new();
    let mut pending = vec![registrations_dir(home)];
    for file in [home.join(".darkmatter.yaml"), home.join(".claudine/config.yaml")] {
        let modified = std::fs::metadata(&file).unwrap().modified().unwrap();
        entries.insert(file.clone(), (std::fs::read(&file).unwrap(), modified));
    }
    while let Some(dir) = pending.pop() {
        let Ok(children) = std::fs::read_dir(&dir) else { continue };
        for child in children {
            let path = child.unwrap().path();
            let metadata = std::fs::symlink_metadata(&path).unwrap();
            let modified = metadata.modified().unwrap();
            if metadata.is_dir() {
                pending.push(path.clone());
                entries.insert(path, (Vec::new(), modified));
            } else {
                entries.insert(path.clone(), (std::fs::read(&path).unwrap(), modified));
            }
        }
    }
    entries
}

#[test]
fn setup_replaces_a_retired_entry_whose_manifest_is_gone_and_a_second_run_changes_nothing() {
    let fixture = configured("setup-migrate-missing");
    assert!(!legacy_manifest_path(fixture.home()).exists());

    let (success, stdout, stderr) = claudine(&fixture, &["setup"]);

    assert!(success, "claudine setup failed:\n{stderr}");
    assert!(stdout.is_empty(), "setup reports on stderr only:\n{stdout}");
    assert_eq!(
        read(&fixture.home().join(".darkmatter.yaml")),
        darkmatter_config().replace(LEGACY_MANIFEST_REFERENCE, MANIFEST_REFERENCE),
        "the entry is replaced in place; every other byte is kept"
    );
    assert!(stderr.contains(MANIFEST_REFERENCE) && stderr.contains(LEGACY_MANIFEST_REFERENCE), "{stderr}");
    assert_eq!(read(&fixture.home().join(".claudine/config.yaml")), CLAUDINE_CONFIG, "settings are kept byte for byte");
    let manifest = std::fs::read(manifest_path(fixture.home())).expect("the format 3 manifest is published");
    assert_eq!(manifest, RegistrationInventory::claudine().manifest().bytes());
    let repaired = snapshot(fixture.home());

    let (success, stdout, stderr) = claudine(&fixture, &["setup"]);

    assert!(success, "the second run failed:\n{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(!stderr.contains(LEGACY_MANIFEST_REFERENCE), "nothing is left to replace:\n{stderr}");
    assert_eq!(snapshot(fixture.home()), repaired, "bytes and modification times are unchanged");
}

#[test]
fn setup_removes_the_retired_files_and_leaves_a_repository_entry_to_its_owner() {
    let fixture = configured("setup-migrate-present");
    let home = fixture.home().to_path_buf();
    write(&legacy_manifest_path(&home), "kind: darkmatter-host\nformat_version: 1\n");
    write(&legacy_revisions_dir(&home).join("0123abcd/globals.yaml"), "kind: schema\n");
    assert!(common::init_git_repo(fixture.cwd()), "git init");
    let repository = fixture.cwd().join(".darkmatter.yaml");
    let repository_text = format!("register:\n  - {LEGACY_MANIFEST_REFERENCE}\n");
    write(&repository, &repository_text);

    let (success, _, stderr) = claudine(&fixture, &["setup"]);

    assert!(success, "claudine setup failed:\n{stderr}");
    assert!(!legacy_manifest_path(&home).exists() && !legacy_revisions_dir(&home).exists(), "{stderr}");
    assert!(stderr.contains("Removed the retired host registration"), "{stderr}");
    assert_eq!(read(&repository), repository_text, "a repository's entry is its owners' to edit");
    assert!(read(&home.join(".darkmatter.yaml")).contains(MANIFEST_REFERENCE));
}

/// A writable retired tree below a read-only `.claudine`: its entries go,
/// its root and the retired manifest cannot, and setup says what went.
#[cfg(unix)]
#[test]
fn setup_reports_a_partly_removed_retired_registration() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = configured("setup-migrate-partial");
    let home = fixture.home().to_path_buf();
    let (success, _, stderr) = claudine(&fixture, &["setup"]);
    assert!(success, "the first setup failed:\n{stderr}");
    let revisions = legacy_revisions_dir(&home);
    write(&legacy_manifest_path(&home), "kind: darkmatter-host\nformat_version: 1\n");
    write(&revisions.join("0123abcd/globals.yaml"), "kind: schema\n");
    let claudine_dir = home.join(".claudine");
    let set_mode = |mode| std::fs::set_permissions(&claudine_dir, std::fs::Permissions::from_mode(mode)).unwrap();
    set_mode(0o555);
    // A superuser ignores the mode.
    if std::fs::write(claudine_dir.join("probe"), "").is_ok() {
        set_mode(0o755);
        eprintln!("skipped: the folder mode does not stop this user");
        return;
    }

    let (success, _, stderr) = claudine(&fixture, &["setup"]);

    set_mode(0o755);
    assert!(success, "a cleanup failure is a warning:\n{stderr}");
    let inner = biscuit_file::to_forward_slash_string(&revisions.join("0123abcd"));
    assert!(stderr.contains(&format!("Removed `{inner}` of the retired host registration")), "{stderr}");
    assert!(stderr.contains("The retired host registration was only partly removed"), "{stderr}");
    assert!(!revisions.join("0123abcd").exists() && revisions.is_dir() && legacy_manifest_path(&home).is_file());

    let (success, _, stderr) = claudine(&fixture, &["setup"]);

    assert!(success, "{stderr}");
    assert!(!stderr.contains(&inner), "only what was left is removed:\n{stderr}");
    assert!(!revisions.exists() && !legacy_manifest_path(&home).exists(), "{stderr}");
}

#[test]
fn setup_without_a_configuration_writes_the_defaults_without_asking() {
    let fixture = CliProcessFixture::named("setup-first-run");
    assert!(!fixture.home().join(".claudine/config.yaml").exists());

    let (success, _, stderr) = claudine(&fixture, &["setup"]);

    assert!(success, "claudine setup failed:\n{stderr}");
    assert!(fixture.home().join(".claudine/config.yaml").is_file(), "{stderr}");
    assert!(read(&fixture.home().join(".darkmatter.yaml")).contains(MANIFEST_REFERENCE), "{stderr}");
    assert!(manifest_path(fixture.home()).is_file());
}

#[test]
fn registrations_publish_leaves_a_retired_entry_for_setup() {
    let fixture = configured("setup-publish-separate");
    let before = read(&fixture.home().join(".darkmatter.yaml"));

    let (success, _, stderr) = claudine(&fixture, &["registrations", "publish"]);

    assert!(success, "{stderr}");
    assert!(manifest_path(fixture.home()).is_file());
    assert_eq!(read(&fixture.home().join(".darkmatter.yaml")), before, "publishing edits no configuration");
    assert_eq!(read(&fixture.home().join(".claudine/config.yaml")), CLAUDINE_CONFIG);
}

#[test]
fn setup_keeps_both_blocks_on_a_permissions_conflict_and_still_registers() {
    let fixture = configured("setup-conflict");
    let darkmatter = fixture.home().join(".darkmatter.yaml");
    let text = darkmatter_config().replace("permissions: ~/.claudine/config.yaml\n", "permissions:\n  network: allow\n");
    write(&darkmatter, &text);

    let (success, _, stderr) = claudine(&fixture, &["setup"]);

    assert!(success, "a conflict is a warning:\n{stderr}");
    assert!(stderr.contains("Darkmatter permissions were not set up"), "{stderr}");
    assert_eq!(read(&darkmatter), text.replace(LEGACY_MANIFEST_REFERENCE, MANIFEST_REFERENCE), "only the entry changes");
    assert_eq!(read(&fixture.home().join(".claudine/config.yaml")), CLAUDINE_CONFIG);
}
