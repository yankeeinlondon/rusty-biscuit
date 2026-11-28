//! The `darkmatter-permissions` row and its repair over real temporary
//! homes and repositories, plus the shared planner and committer driven
//! directly where a case needs to act between them.
//!
//! The registry here holds only `config` and `darkmatter-permissions`, so no
//! other repair writes into the fixture.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::config::init::{self as dm_init, InitError, default_permissions};
use darkmatter::config::schema::ConfigScope as DarkmatterScope;
use darkmatter::config::write::ConfigWriteError;
use serde_json::{Value, json};
use simplified_schema::yaml_documents::load_yaml_document;

use super::super::config::check_config;
#[cfg(unix)]
use super::super::file_checks;
use super::super::{CheckDescriptor, DoctorRow, Environment, Severity, native_path, run_with};
use super::{check_permissions, repair_permissions};
#[cfg(unix)]
use crate::composition::registration::{
    FileKind, PublishedFile, RegistrationInventory,
};
#[cfg(unix)]
use crate::config::persist::SaveOutcome;
use crate::config::schema::ConfigScope;
use crate::config::setup::{
    ClaudineSide, HandoffCommitError, HandoffMode, ManualReason, HandoffRefusal, commit_permissions_handoff,
    plan_permissions_handoff, read_document,
};

const CLAUDINE: &str = ".claudine/config.yaml";
const DARKMATTER: &str = ".darkmatter.yaml";
const ID: &str = "darkmatter-permissions";
const BLOCK: &str = "permissions:\n  shell-commands:\n    - prefix ls\n  network: deny\n";
const OTHER_BLOCK: &str = "permissions:\n  shell-commands:\n    - prefix git\n";

static REGISTRY: [CheckDescriptor; 2] = [
    CheckDescriptor { id: "config", title: "Configuration files", prerequisites: &[], check: check_config, repair: None },
    CheckDescriptor {
        id: ID,
        title: "Darkmatter permissions handoff",
        prerequisites: &["config"],
        check: check_permissions,
        repair: Some(repair_permissions),
    },
];

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    home: PathBuf,
    repo: PathBuf,
    /// The repository root the context names; `None` outside a repository.
    repository: Option<PathBuf>,
}

impl Fixture {
    /// A home with a valid Claudine file (no `permissions`) and nothing else;
    /// the launch directory is outside any repository.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        let (home, repo) = (root.join("home"), root.join("repo"));
        std::fs::create_dir_all(home.join(".claudine")).unwrap();
        std::fs::create_dir_all(repo.join(".claudine")).unwrap();
        let fixture = Self { _dir: dir, root, home, repo, repository: None };
        fixture.user(CLAUDINE, "# my settings\nlogging: true\n");
        fixture
    }

    /// As [`Self::new`], launched inside the repository.
    fn in_repository() -> Self {
        let mut fixture = Self::new();
        fixture.repository = Some(fixture.repo.clone());
        fixture
    }

    fn user(&self, name: &str, text: &str) -> PathBuf {
        write(&self.home.join(native_path(name)), text)
    }

    fn repository_file(&self, name: &str, text: &str) -> PathBuf {
        write(&self.repo.join(native_path(name)), text)
    }

    fn context(&self) -> FileResolutionContext {
        let cwd = self.repository.clone().unwrap_or_else(|| self.home.clone());
        let context = FileResolutionContext::from_snapshot(cwd, Some(self.home.clone()), HashMap::new());
        match &self.repository {
            Some(root) => context.with_repository_root(root),
            None => context,
        }
    }

    fn environment(&self) -> Environment {
        Environment::for_test(Some(self.home.clone()), Ok(self.context()))
    }

    fn row(&self, fix: bool) -> DoctorRow {
        run_with(&REGISTRY, &self.environment(), fix).into_iter().find(|row| row.id == ID).unwrap()
    }

    /// Every file under the fixture with its bytes and modification time.
    fn tree(&self) -> Vec<(PathBuf, Vec<u8>, SystemTime)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>, SystemTime)>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let metadata = std::fs::symlink_metadata(&path).unwrap();
                if metadata.is_dir() {
                    walk(&path, out);
                } else {
                    let bytes = std::fs::read(&path).unwrap_or_default();
                    out.push((path, bytes, metadata.modified().unwrap()));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out.sort();
        out
    }
}

fn write(path: &Path, text: &str) -> PathBuf {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
    path.to_path_buf()
}

fn document(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap();
    load_yaml_document(&text, path).unwrap().json
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[track_caller]
fn assert_row(row: &DoctorRow, severity: Severity, fixable: bool, needle: &str) {
    assert_eq!((row.severity, row.fixable), (severity, fixable), "{row:#?}");
    assert!(row.summary.contains(needle) || row.detail.contains(needle), "`{needle}` not in {row:#?}");
}

// ---------------------------------------------------------------- references

#[test]
fn a_reference_to_claudine_passes_in_every_spelling() {
    let fixture = Fixture::new();
    let claudine = fixture.user(CLAUDINE, &format!("logging: true\n{BLOCK}"));
    let absolute = claudine.display().to_string();
    for spelling in ["~/.claudine/config.yaml", absolute.as_str(), ".claudine/config.yaml", "./.claudine/config.yaml"] {
        fixture.user(DARKMATTER, &format!("permissions: {spelling}\n"));
        let row = fixture.row(false);
        assert_row(&row, Severity::Ok, false, "Darkmatter takes its permissions from Claudine's configuration");
        assert!(row.detail.contains("the user `permissions` refers to Claudine's configuration"), "{spelling}: {row:#?}");
    }
}

#[test]
fn the_repository_scope_is_assessed_with_its_policy_note() {
    let fixture = Fixture::in_repository();
    fixture.user(CLAUDINE, BLOCK);
    fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
    fixture.repository_file(CLAUDINE, "permissions:\n  network: deny\n");
    let darkmatter = fixture.repository_file(DARKMATTER, "permissions: ./.claudine/config.yaml\n");

    let row = fixture.row(false);
    assert_row(&row, Severity::Ok, false, "the repository `permissions` refers to Claudine's configuration");
    assert!(row.detail.contains(&darkmatter.display().to_string()), "{row:#?}");
    assert!(row.detail.contains("committed, and working-copy policy"), "{row:#?}");
}

#[test]
fn a_valid_third_party_reference_is_information() {
    let fixture = Fixture::new();
    fixture.user(".acme/config.yaml", "permissions:\n  network: deny\n");
    fixture.user(DARKMATTER, "permissions: ~/.acme/config.yaml\n");
    let row = fixture.row(false);
    assert_row(&row, Severity::Info, false, "refers to another program's file; Claudine leaves it in place");
}

#[test]
fn every_broken_reference_is_an_error_without_a_repair() {
    let cases = [
        ("a missing target", "permissions: ~/.acme/config.yaml\n", None, "a file"),
        ("a target without a mapping", "permissions: ~/.acme/config.yaml\n", Some("other: 1\n"), "a file"),
        ("a null mapping", "permissions: ~/.acme/config.yaml\n", Some("permissions: null\n"), "a file"),
        ("a self-reference", "permissions: ~/.darkmatter.yaml\n", None, "a file"),
        ("Claudine without a block", "permissions: ~/.claudine/config.yaml\n", None, "Claudine's configuration"),
    ];
    for (case, darkmatter, target, names) in cases {
        let fixture = Fixture::new();
        fixture.user(DARKMATTER, darkmatter);
        if let Some(target) = target {
            fixture.user(".acme/config.yaml", target);
        }
        let before = fixture.tree();
        let row = fixture.row(true);
        assert_row(&row, Severity::Error, false, &format!("refers to {names}, which does not supply a valid"));
        assert!(!row.detail.contains("Repair"), "{case}: {row:#?}");
        assert_eq!(fixture.tree(), before, "{case}");
    }
}

// ---------------------------------------------------------------- prerequisites

#[test]
fn a_missing_prerequisite_skips_its_scope_with_information() {
    let fixture = Fixture::new();
    let row = fixture.row(false);
    assert_row(&row, Severity::Info, false, "user scope skipped: this `.darkmatter.yaml` does not exist");

    std::fs::remove_file(fixture.home.join(native_path(CLAUDINE))).unwrap();
    fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
    let row = fixture.row(false);
    assert_row(&row, Severity::Info, false, "user scope skipped: this Claudine configuration does not exist");

    fixture.user(CLAUDINE, "logging: [not, a, boolean]\n");
    let row = fixture.row(false);
    assert_row(&row, Severity::Info, false, "user scope skipped: this file has problems, reported under `config`");
}

#[test]
fn a_user_only_rule_the_repository_replaces_blocks_the_handoff_and_is_explained_there() {
    let routes = "messenger:\n  active_config: SECRET-GONE\n  configurations:\n    team:\n      slack:\n        channel_id: C1\n";
    let fixture = Fixture::in_repository();
    let user_claudine = fixture.user(CLAUDINE, routes);
    let user_darkmatter = fixture.user(DARKMATTER, BLOCK);
    fixture.repository_file(CLAUDINE, "active_messenger: team\n");
    let rows = run_with(&REGISTRY, &fixture.environment(), false);
    let config = rows.iter().find(|row| row.id == "config").unwrap();
    assert_eq!(config.severity, Severity::Ok, "{config:#?}");

    let before = fixture.tree();
    let row = fixture.row(true);
    assert_eq!(fixture.tree(), before, "the refused handoff writes nothing");
    assert_row(&row, Severity::Error, false, "breaks a configuration rule, so the `permissions` handoff cannot edit it");
    assert!(row.detail.contains(&format!("{}: breaks a configuration rule", user_claudine.display())), "{row:#?}");
    assert!(
        row.detail.contains("in the user file alone, the setting `messenger.active_config` breaks a configuration rule"),
        "{row:#?}"
    );
    assert!(!row.detail.contains("reported under `config`"), "{row:#?}");
    assert!(!row.detail.contains("SECRET"), "{row:#?}");
    assert_eq!(read(&user_darkmatter), BLOCK);

    // Outside the repository `config` reports the rule, and the row says so.
    let outside = Fixture::new();
    outside.user(CLAUDINE, routes);
    outside.user(DARKMATTER, BLOCK);
    let row = outside.row(false);
    assert_row(&row, Severity::Error, false, "the rule is reported under `config`");
}

#[test]
fn a_missing_home_skips_the_user_scope() {
    let fixture = Fixture::new();
    let context = FileResolutionContext::from_snapshot(fixture.home.clone(), None, HashMap::new());
    let rows = run_with(&REGISTRY, &Environment::for_test(None, Ok(context)), false);
    let row = rows.iter().find(|row| row.id == ID).unwrap();
    assert_row(row, Severity::Info, false, "user scope skipped: the home directory cannot be located");
}

#[test]
fn a_repository_without_its_claudine_file_skips_only_that_scope() {
    let fixture = Fixture::in_repository();
    fixture.user(CLAUDINE, BLOCK);
    fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
    let darkmatter = fixture.repository_file(DARKMATTER, "permissions:\n  network: deny\n");
    let before = fixture.tree();

    let row = fixture.row(true);
    assert_row(&row, Severity::Info, false, "repository scope skipped: this Claudine configuration does not exist");
    assert!(row.detail.contains("the user `permissions` refers to Claudine's configuration"), "{row:#?}");
    assert_eq!(fixture.tree(), before);
    assert_eq!(read(&darkmatter), "permissions:\n  network: deny\n");
}

#[test]
fn a_repository_that_is_the_home_is_assessed_once() {
    let mut fixture = Fixture::new();
    fixture.repository = Some(fixture.home.clone());
    fixture.user(CLAUDINE, BLOCK);
    fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
    let row = fixture.row(false);
    assert_row(&row, Severity::Info, false, "repository scope skipped: this file is the user file");
    assert!(row.detail.contains("the user `permissions` refers to Claudine's configuration"), "{row:#?}");
}

#[cfg(unix)]
#[test]
fn a_repository_darkmatter_file_aliasing_the_user_one_skips_the_repository_scope() {
    let fixture = Fixture::in_repository();
    fixture.user(CLAUDINE, BLOCK);
    let user = fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
    fixture.repository_file(CLAUDINE, "permissions:\n  network: deny\n");
    std::os::unix::fs::symlink(&user, fixture.repo.join(DARKMATTER)).unwrap();
    let before = fixture.tree();

    let row = fixture.row(true);
    assert_row(&row, Severity::Info, false, "repository scope skipped: this file is the user file");
    assert_eq!(fixture.tree(), before);
}

// ---------------------------------------------------------------- conflicts

#[test]
fn different_inline_blocks_conflict_and_nothing_is_written() {
    let fixture = Fixture::new();
    fixture.user(CLAUDINE, BLOCK);
    fixture.user(DARKMATTER, OTHER_BLOCK);
    let before = fixture.tree();

    let row = fixture.row(true);
    assert_row(&row, Severity::Error, false, "block differs from the one in Claudine's configuration");
    assert!(row.detail.contains("Remedy: choose which setting to keep"), "{row:#?}");
    assert_eq!(fixture.tree(), before);
}

#[test]
fn a_claudine_side_reference_is_manual_in_every_darkmatter_state() {
    for darkmatter in ["register: []\n", "permissions: null\n", BLOCK] {
        let fixture = Fixture::new();
        fixture.user(".acme/config.yaml", BLOCK);
        fixture.user(CLAUDINE, "permissions: ~/.acme/config.yaml\n");
        fixture.user(DARKMATTER, darkmatter);
        let before = fixture.tree();
        let row = fixture.row(true);
        assert_row(&row, Severity::Error, false, "whose own `permissions` is a reference");
        assert!(row.detail.contains("choose which setting to keep"), "{darkmatter:?}: {row:#?}");
        assert_eq!(fixture.tree(), before, "{darkmatter:?}");
    }
}

#[test]
fn a_mapping_that_fails_the_scoped_rules_is_manual() {
    let fixture = Fixture::new();
    // Valid by the schema; the loader refuses an absolute `file-read` entry.
    fixture.user(CLAUDINE, "permissions:\n  file-read:\n    - /etc/SECRET.txt\n");
    fixture.user(DARKMATTER, "register: []\n");
    let before = fixture.tree();
    let row = fixture.row(true);
    assert_row(&row, Severity::Error, false, "fails Darkmatter's checks for the user scope");
    assert!(
        row.detail.contains("Remedy: correct its `permissions` mapping for the user scope, then run `claudine doctor --fix`"),
        "{row:#?}"
    );
    assert!(!row.detail.contains("md compose") && !row.detail.contains("claudine setup"), "{row:#?}");
    assert!(!row.detail.contains("SECRET"), "{row:#?}");
    assert_eq!(fixture.tree(), before);
}

// ---------------------------------------------------------------- remedies
//
// `claudine setup` performs the user handoff only after publishing the host
// registration, so a publisher obstruction stops it before the handoff;
// doctor's repair does not depend on publication, so it is the one remedy
// offered in either scope.

#[test]
fn every_repairable_handoff_offers_doctors_fix_and_never_setup() {
    const FIX: &str = "Remedy: run `claudine doctor --fix`";
    let fixture = Fixture::new();
    fixture.user(DARKMATTER, BLOCK);
    let row = fixture.row(false);
    assert_row(&row, Severity::Error, true, "the user `permissions` is an inline block");
    assert!(row.detail.contains(FIX) && !row.detail.contains("claudine setup"), "{row:#?}");

    let fixture = Fixture::new();
    fixture.user(CLAUDINE, BLOCK);
    fixture.user(DARKMATTER, "permissions: null\n");
    let row = fixture.row(false);
    assert_row(&row, Severity::Error, true, "the user `permissions` is null");
    assert!(row.detail.contains(FIX) && !row.detail.contains("claudine setup"), "{row:#?}");

    let fixture = Fixture::in_repository();
    fixture.user(CLAUDINE, BLOCK);
    fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
    fixture.repository_file(CLAUDINE, "# repo\nactive_messenger: null\n");
    fixture.repository_file(DARKMATTER, "permissions:\n  network: deny\n");
    let row = fixture.row(false);
    assert_row(&row, Severity::Error, true, "the repository `permissions` is an inline block");
    assert!(row.detail.contains(FIX) && !row.detail.contains("claudine setup"), "{row:#?}");
}

/// Moves `path` to `destination` inside the home and links it back, so
/// every reader sees the same bytes through the link.
#[cfg(unix)]
fn link_back(path: &Path, destination: &Path) {
    std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
    std::fs::rename(path, destination).unwrap();
    std::os::unix::fs::symlink(destination, path).unwrap();
}

/// A user handoff beside a published, registered bundle that `obstruct`
/// then makes unpublishable for setup. Proves the premise (setup stops at
/// publication and leaves the inline block), that the row offers doctor's
/// repair alone, and that `--fix` completes the handoff while the
/// `registration` row keeps the severity `registration`.
#[cfg(unix)]
#[track_caller]
fn assert_doctor_fix_hands_off_past(
    case: &str,
    reference_only: bool,
    previous: Option<&RegistrationInventory>,
    registration: Severity,
    obstruct: impl FnOnce(&Fixture),
) {
    use crate::composition::registration::publish;
    use crate::config::setup::{SetupError, setup_claudine};

    let fixture = Fixture::new();
    publish(&fixture.home, previous.unwrap_or_else(|| RegistrationInventory::claudine())).unwrap();
    if reference_only {
        fixture.user(CLAUDINE, &format!("# mine\n{BLOCK}"));
    }
    let darkmatter_text = format!("{BLOCK}register:\n    - ~/.claudine/registrations/darkmatter/manifest.yaml\n");
    let darkmatter = fixture.user(DARKMATTER, &darkmatter_text);
    obstruct(&fixture);

    let setup = setup_claudine(&fixture.context(), ConfigScope::User, |_| Ok(SaveOutcome::default()));
    assert!(matches!(setup, Err(SetupError::Publish { .. })), "{case}: setup must stop at publication: {setup:#?}");
    assert_eq!(read(&darkmatter), darkmatter_text, "{case}: setup left the inline block");

    let rows = |fix| run_with(file_checks(), &fixture.environment(), fix);
    let find = |rows: &[DoctorRow], id: &str| rows.iter().find(|row| row.id == id).cloned().unwrap();
    let inspected = rows(false);
    let row = find(&inspected, ID);
    assert_eq!((row.severity, row.fixable), (Severity::Error, true), "{case}: {row:#?}");
    assert!(
        row.detail.contains("Remedy: run `claudine doctor --fix`") && !row.detail.contains("claudine setup"),
        "{case}: {row:#?}"
    );
    assert_eq!(find(&inspected, "registration").severity, registration, "{case}: {inspected:#?}");

    let fixed = rows(true);
    assert_row(&find(&fixed, ID), Severity::Ok, false, "refers to Claudine's configuration");
    assert_eq!(document(&darkmatter)["permissions"], json!("~/.claudine/config.yaml"), "{case}");
    assert_eq!(find(&rows(false), "registration").severity, registration, "{case}: the obstruction is still reported");
}

/// The first schema member of the compiled inventory.
#[cfg(unix)]
fn a_schema_member() -> &'static str {
    let files = RegistrationInventory::claudine().files();
    files.iter().map(PublishedFile::path).find(|path| path.starts_with("schemas/")).unwrap()
}

// Spelled out rather than located: only the registration module may call its
// locators (`registrations_command::only_the_registration_module_locates_or_loads_an_installed_bundle`).
#[cfg(unix)]
const REGISTRATIONS: &str = ".claudine/registrations";
#[cfg(unix)]
const PUBLISHED_ROOT: &str = ".claudine/registrations/darkmatter";
#[cfg(unix)]
const PUBLISHER_LOCK: &str = ".claudine/registrations/.darkmatter.lock";

#[cfg(unix)]
#[test]
fn a_linked_registration_path_leaves_the_handoff_to_doctors_fix() {
    let member = a_schema_member();
    for (case, relative) in [
        ("registration folder", String::new()),
        ("schema folder", "schemas".to_string()),
        ("manifest", "manifest.yaml".to_string()),
        ("schema member", member.to_string()),
    ] {
        assert_doctor_fix_hands_off_past(case, false, None, Severity::Error, |fixture| {
            let root = fixture.home.join(native_path(PUBLISHED_ROOT));
            let path = if relative.is_empty() { root } else { root.join(native_path(&relative)) };
            link_back(&path, &fixture.home.join("moved").join(case.replace(' ', "-")));
        });
    }
}

#[cfg(unix)]
#[test]
fn a_linked_shared_parent_leaves_the_handoff_to_doctors_fix() {
    // A linked `~/.claudine` holds Claudine's file too; only a handoff that
    // writes Darkmatter's file alone stays repairable through it.
    assert_doctor_fix_hands_off_past("~/.claudine", true, None, Severity::Error, |fixture| {
        link_back(&fixture.home.join(".claudine"), &fixture.home.join("dotfiles/claudine"));
    });
    assert_doctor_fix_hands_off_past("registrations", false, None, Severity::Error, |fixture| {
        link_back(&fixture.home.join(native_path(REGISTRATIONS)), &fixture.home.join("dotfiles/registrations"));
    });
}

#[cfg(unix)]
#[test]
fn a_linked_publisher_lock_hidden_by_a_healthy_registration_row_leaves_the_handoff_to_doctors_fix() {
    assert_doctor_fix_hands_off_past("publisher lock", false, None, Severity::Ok, |fixture| {
        // Publication leaves the lock file in place.
        link_back(&fixture.home.join(native_path(PUBLISHER_LOCK)), &fixture.home.join("dotfiles/lock"));
    });
}

#[cfg(unix)]
#[test]
fn a_linked_former_member_that_publication_retires_leaves_the_handoff_to_doctors_fix() {
    let former = "schemas/former.yaml";
    let previous = RegistrationInventory::claudine().clone().with_member(PublishedFile::new(
        former,
        FileKind::Library,
        None::<&'static str>,
        b"kind: schema\ntypes:\n    former: string\n".to_vec(),
    ));
    assert_doctor_fix_hands_off_past("former member", false, Some(&previous), Severity::Error, |fixture| {
        let path = fixture.home.join(native_path(PUBLISHED_ROOT)).join(native_path(former));
        link_back(&path, &fixture.home.join("dotfiles/former.yaml"));
    });
}

#[test]
fn a_broken_repository_reference_is_corrected_without_a_user_scoped_command() {
    let fixture = Fixture::in_repository();
    fixture.user(CLAUDINE, BLOCK);
    fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
    fixture.repository_file(CLAUDINE, "# repo\nactive_messenger: null\n");
    fixture.repository_file(DARKMATTER, "permissions: ./SECRET-missing.yaml\n");
    let rows = run_with(&REGISTRY, &fixture.environment(), false);
    let (config, row) = (&rows[0], &rows[1]);
    // The handoff row keeps its error and refers to `config`, which locates
    // the reference and says how to correct it.
    assert_row(row, Severity::Error, false, "the repository `permissions` refers to a file, which does not supply a valid");
    assert!(row.detail.contains("the reference is reported under `config`"), "{row:#?}");
    assert!(row.detail.contains("Remedy: correct it as `config` describes, then run `claudine doctor` again"), "{row:#?}");
    assert_row(config, Severity::Error, false, "has a `permissions` reference to a file that does not exist (line 1, column 14)");
    assert!(
        config.detail.contains("Remedy: correct the reference or the file it names, then run `claudine doctor` again"),
        "{config:#?}"
    );
    for row in [config, row] {
        assert!(!row.detail.contains("md init") && !row.detail.contains("md compose"), "{row:#?}");
        assert!(!row.detail.contains("SECRET"), "{row:#?}");
    }

    // The user scope keeps both: `md` prints the message and `md init
    // --inline` rewrites that file.
    let fixture = Fixture::new();
    fixture.user(DARKMATTER, "permissions: ~/.acme/config.yaml\n");
    let config = run_with(&REGISTRY, &fixture.environment(), false).swap_remove(0);
    assert!(config.detail.contains("`md compose` of a document that runs a shell command prints the full message"), "{config:#?}");
    assert!(config.detail.contains("`md init --inline` replaces the reference with a block"), "{config:#?}");
}

// ---------------------------------------------------------------- repairs

#[test]
fn an_omitted_setting_is_repaired_with_the_default_block_and_the_reference() {
    let fixture = Fixture::new();
    let claudine = fixture.home.join(native_path(CLAUDINE));
    let darkmatter = fixture.user(DARKMATTER, "# mine\nregister: []   # none yet\n");

    let before = fixture.row(false);
    assert_row(&before, Severity::Error, true, "does not set `permissions` to Claudine's configuration");

    let row = fixture.row(true);
    assert_row(&row, Severity::Ok, false, "refers to Claudine's configuration");
    assert!(row.detail.contains(&format!("Repaired: {}: set `permissions`", claudine.display())), "{row:#?}");
    assert!(row.detail.contains(&format!("Repaired: {}: set `permissions`", darkmatter.display())), "{row:#?}");
    assert!(read(&claudine).starts_with("# my settings\nlogging: true\n"), "{}", read(&claudine));
    assert_eq!(document(&claudine)["permissions"], default_permissions(DarkmatterScope::User));
    assert_eq!(read(&darkmatter), "# mine\nregister: []   # none yet\npermissions: ~/.claudine/config.yaml\n");
}

#[test]
fn a_null_setting_reuses_claudines_mapping_and_writes_only_the_reference() {
    let fixture = Fixture::new();
    let claudine = fixture.user(CLAUDINE, &format!("# mine\n{BLOCK}"));
    let darkmatter = fixture.user(DARKMATTER, "permissions: null # unset\nregister: []\n");
    let claudine_before = std::fs::read(&claudine).unwrap();

    assert_row(&fixture.row(false), Severity::Error, true, "the user `permissions` is null");
    let row = fixture.row(true);
    assert_row(&row, Severity::Ok, false, "refers to Claudine's configuration");
    assert!(!row.detail.contains(&format!("{}: set", claudine.display())), "{row:#?}");
    assert_eq!(std::fs::read(&claudine).unwrap(), claudine_before);
    assert_eq!(read(&darkmatter), "permissions: ~/.claudine/config.yaml # unset\nregister: []\n");
}

#[test]
fn an_effectively_equal_block_is_repaired_by_writing_only_the_reference() {
    let fixture = Fixture::new();
    // Equal to `{}` once the scope's defaults fill it in.
    let claudine = fixture.user(CLAUDINE, "permissions:\n  network: deny\n");
    let darkmatter = fixture.user(DARKMATTER, "permissions: {}\n");
    let claudine_before = std::fs::read(&claudine).unwrap();

    let row = fixture.row(false);
    assert_row(&row, Severity::Error, true, "already holds an equivalent block; the repair only refers to it");
    let row = fixture.row(true);
    assert_row(&row, Severity::Ok, false, "refers to Claudine's configuration");
    assert_eq!(std::fs::read(&claudine).unwrap(), claudine_before);
    assert_eq!(document(&darkmatter), json!({ "permissions": "~/.claudine/config.yaml" }));
}

#[test]
fn a_transfer_preserves_every_other_setting_and_comment_and_converges() {
    let fixture = Fixture::new();
    let claudine_text = "# my settings\nlogging: true   # keep\n";
    let claudine = fixture.user(CLAUDINE, claudine_text);
    let darkmatter = fixture.user(
        DARKMATTER,
        "# top\nregister: []   # none\npermissions:\n  shell-commands:\n    - prefix ls\n  network: deny\n# tail\n",
    );

    let row = fixture.row(true);
    assert_row(&row, Severity::Ok, false, "refers to Claudine's configuration");
    let written = read(&claudine);
    assert!(written.starts_with(claudine_text), "{written}");
    assert_eq!(document(&claudine)["permissions"], json!({ "shell-commands": ["prefix ls"], "network": "deny" }));
    assert_eq!(document(&claudine)["logging"], json!(true));
    let referred = read(&darkmatter);
    assert!(referred.starts_with("# top\nregister: []   # none\npermissions: ~/.claudine/config.yaml\n"), "{referred}");
    assert!(referred.contains("# tail"), "{referred}");

    let settled = fixture.tree();
    let again = fixture.row(true);
    assert_row(&again, Severity::Ok, false, "refers to Claudine's configuration");
    assert!(!again.detail.contains("Repair"), "{again:#?}");
    assert_eq!(fixture.tree(), settled);
}

#[test]
fn the_repository_scope_is_repaired_with_its_relative_reference() {
    let fixture = Fixture::in_repository();
    fixture.user(CLAUDINE, BLOCK);
    fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
    let claudine = fixture.repository_file(CLAUDINE, "# repo\nactive_messenger: null\n");
    let darkmatter = fixture.repository_file(DARKMATTER, "permissions:\n  network: deny\n");

    assert_row(&fixture.row(false), Severity::Error, true, "the repository `permissions` is an inline block");
    let row = fixture.row(true);
    assert_row(&row, Severity::Ok, false, "the repository `permissions` refers to Claudine's configuration");
    assert_eq!(document(&claudine)["permissions"], json!({ "network": "deny" }));
    assert_eq!(read(&darkmatter), "permissions: ./.claudine/config.yaml\n");
}

#[test]
fn an_interrupted_transfer_keeps_the_inline_block_and_a_retry_completes_it() {
    let fixture = Fixture::new();
    let claudine = fixture.home.join(native_path(CLAUDINE));
    let darkmatter = fixture.user(DARKMATTER, BLOCK);

    let failed = {
        let _hook = darkmatter::config::write::test_hooks::fail_replacement_after(1);
        fixture.row(true)
    };
    assert_eq!(failed.severity, Severity::Error, "{failed:#?}");
    assert!(failed.summary.starts_with("the repair failed: user scope: could not replace"), "{failed:#?}");
    assert!(failed.detail.contains(&format!("Already changed: {}: set `permissions`", claudine.display())), "{failed:#?}");
    // The Darkmatter file still holds its block, so its policy is unchanged,
    // and the fresh reading still offers the repair.
    assert_eq!(read(&darkmatter), BLOCK);
    assert!(failed.fixable, "{failed:#?}");

    let claudine_bytes = std::fs::read(&claudine).unwrap();
    let claudine_modified = std::fs::metadata(&claudine).unwrap().modified().unwrap();
    let row = fixture.row(true);
    assert_row(&row, Severity::Ok, false, "refers to Claudine's configuration");
    assert!(!row.detail.contains(&format!("{}: set", claudine.display())), "{row:#?}");
    assert_eq!(std::fs::read(&claudine).unwrap(), claudine_bytes);
    assert_eq!(std::fs::metadata(&claudine).unwrap().modified().unwrap(), claudine_modified);
    assert_eq!(read(&darkmatter), "permissions: ~/.claudine/config.yaml\n");
}

/// Both scopes hold an inline block the repair would move.
fn two_inline_scopes() -> (Fixture, [PathBuf; 4]) {
    let fixture = Fixture::in_repository();
    let user_claudine = fixture.home.join(native_path(CLAUDINE));
    let user_darkmatter = fixture.user(DARKMATTER, BLOCK);
    let repo_claudine = fixture.repository_file(CLAUDINE, "# repo\nactive_messenger: null\n");
    let repo_darkmatter = fixture.repository_file(DARKMATTER, OTHER_BLOCK);
    (fixture, [user_claudine, user_darkmatter, repo_claudine, repo_darkmatter])
}

/// Holds Darkmatter's writer lock beside `config` until dropped.
fn hold_writer_lock(config: &Path) -> std::fs::File {
    let mut sidecar = config.as_os_str().to_owned();
    sidecar.push(darkmatter::config::write::LOCK_SUFFIX);
    let lock = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(sidecar).unwrap();
    lock.lock().unwrap();
    lock
}

#[test]
fn a_failed_scope_does_not_stop_the_other_scopes_handoff() {
    let _deadline = darkmatter::config::write::test_hooks::lock_deadline(std::time::Duration::from_millis(50));
    let set = |path: &Path| format!("{}: set `permissions`", path.display());
    for locked_user in [true, false] {
        let (fixture, [user_claudine, user_darkmatter, repo_claudine, repo_darkmatter]) = two_inline_scopes();
        let (locked, failed_scope) = if locked_user { (&user_darkmatter, "user") } else { (&repo_darkmatter, "repository") };
        let (kept, kept_block, done) = if locked_user {
            ((&user_darkmatter, &user_claudine), BLOCK, [(&repo_claudine, &repo_darkmatter, "./.claudine/config.yaml")])
        } else {
            ((&repo_darkmatter, &repo_claudine), OTHER_BLOCK, [(&user_claudine, &user_darkmatter, "~/.claudine/config.yaml")])
        };

        let row = {
            let _lock = hold_writer_lock(locked);
            fixture.row(true)
        };

        assert_eq!(row.severity, Severity::Error, "{row:#?}");
        assert!(row.summary.starts_with(&format!("the repair failed: {failed_scope} scope: another writer held the lock")), "{row:#?}");
        assert!(!row.summary.contains("; user scope") && !row.summary.contains("; repository scope"), "one failure: {row:#?}");
        // The failed scope keeps its inline policy and its Claudine file.
        assert_eq!(read(kept.0), kept_block);
        assert!(document(kept.1).get("permissions").is_none(), "{row:#?}");
        assert!(row.fixable, "the failed scope can still be repaired: {row:#?}");
        // The other scope is handed off in the same invocation, both writes reported.
        for (claudine, darkmatter, reference) in done {
            assert!(row.detail.contains(&format!("Already changed: {}", set(claudine))), "{row:#?}");
            assert!(row.detail.contains(&format!("Already changed: {}", set(darkmatter))), "{row:#?}");
            assert_eq!(read(darkmatter), format!("permissions: {reference}\n"));
        }

        // With the lock released, a retry completes the failed scope only.
        let retried = fixture.row(true);
        assert_row(&retried, Severity::Ok, false, "refers to Claudine's configuration");
        assert_eq!(retried.detail.matches("Repaired: ").count(), 2, "{retried:#?}");
    }
}

#[test]
fn both_scopes_are_handed_off_together_when_nothing_blocks_them() {
    let (fixture, files) = two_inline_scopes();

    let row = fixture.row(true);

    assert_row(&row, Severity::Ok, false, "refers to Claudine's configuration");
    for path in &files {
        assert!(row.detail.contains(&format!("Repaired: {}: set `permissions`", path.display())), "{row:#?}");
    }
    assert_eq!(row.detail.matches("Repaired: ").count(), 4, "{row:#?}");
    assert_eq!(read(&files[1]), "permissions: ~/.claudine/config.yaml\n");
    assert_eq!(read(&files[3]), "permissions: ./.claudine/config.yaml\n");
}

#[test]
fn a_file_changed_between_plan_and_commit_is_refused_without_an_overwrite() {
    let fixture = Fixture::new();
    let claudine = fixture.home.join(native_path(CLAUDINE));
    let darkmatter = fixture.user(DARKMATTER, BLOCK);
    let context = fixture.context();
    let observation = dm_init::observe(&context, DarkmatterScope::User).unwrap();
    let (bytes, document) = read_document(&claudine).unwrap();
    let side = ClaudineSide { path: claudine.clone(), bytes, document, block_before: None };
    let plan = plan_permissions_handoff(&context, ConfigScope::User, HandoffMode::Doctor, side, &observation).unwrap();
    assert!(plan.edits_claudine() && plan.edits_darkmatter());

    let concurrent = "permissions:\n  network: allow\n";
    std::fs::write(&darkmatter, concurrent).unwrap();
    let claudine_before = std::fs::read(&claudine).unwrap();
    let error = commit_permissions_handoff(&context, plan).unwrap_err();

    assert!(
        matches!(&error, HandoffCommitError::Darkmatter(error) if matches!(**error, InitError::Write(ConfigWriteError::Changed { .. }))),
        "{error:?}"
    );
    assert_eq!(read(&darkmatter), concurrent);
    assert_eq!(std::fs::read(&claudine).unwrap(), claudine_before);
}

#[test]
fn the_doctor_planner_reads_null_and_omitted_as_handoff_requests() {
    for darkmatter in ["register: []\n", "permissions: null\n", "permissions: ~\n"] {
        for claudine_text in ["logging: true\n", "permissions: null\n"] {
            let fixture = Fixture::new();
            let claudine = fixture.user(CLAUDINE, claudine_text);
            fixture.user(DARKMATTER, darkmatter);
            let context = fixture.context();
            let observation = dm_init::observe(&context, DarkmatterScope::User).unwrap();
            let (bytes, document) = read_document(&claudine).unwrap();
            let side = ClaudineSide { path: claudine, bytes, document, block_before: None };
            let plan = plan_permissions_handoff(&context, ConfigScope::User, HandoffMode::Doctor, side, &observation)
                .unwrap_or_else(|refusal| panic!("{darkmatter:?} / {claudine_text:?}: {refusal:?}"));
            assert!(plan.edits_claudine() && plan.edits_darkmatter(), "{darkmatter:?} / {claudine_text:?}");
            assert_eq!(plan.block, Some(default_permissions(DarkmatterScope::User)));
        }
    }
}

#[test]
fn the_doctor_planner_refuses_a_missing_darkmatter_file() {
    let fixture = Fixture::new();
    let claudine = fixture.home.join(native_path(CLAUDINE));
    let context = fixture.context();
    let observation = dm_init::observe(&context, DarkmatterScope::User).unwrap();
    let (bytes, document) = read_document(&claudine).unwrap();
    let side = ClaudineSide { path: claudine, bytes, document, block_before: None };
    let refusal = plan_permissions_handoff(&context, ConfigScope::User, HandoffMode::Doctor, side, &observation).unwrap_err();
    assert!(matches!(refusal, HandoffRefusal::Manual(ManualReason::DarkmatterMissing)), "{refusal:?}");
}

/// The lock sidecar Darkmatter's writer takes beside `config`.
#[cfg(unix)]
fn sidecar(config: &Path) -> PathBuf {
    PathBuf::from(format!("{}{}", config.display(), darkmatter::config::write::LOCK_SUFFIX))
}

/// A scope whose `.darkmatter.yaml` holds an inline block the repair would
/// move: the user scope outside a repository, or the repository scope with
/// the user scope already handed off. Returns the scope's root.
fn inline_scope(repository: bool) -> (Fixture, PathBuf) {
    if repository {
        let fixture = Fixture::in_repository();
        fixture.user(CLAUDINE, BLOCK);
        fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
        fixture.repository_file(CLAUDINE, "# repo\nactive_messenger: null\n");
        fixture.repository_file(DARKMATTER, BLOCK);
        let root = fixture.repo.clone();
        (fixture, root)
    } else {
        let fixture = Fixture::new();
        fixture.user(DARKMATTER, BLOCK);
        let root = fixture.home.clone();
        (fixture, root)
    }
}

/// Moves `<root>/.claudine` to a shared folder outside every scope and
/// links it back, so the configuration reads the same bytes through it.
#[cfg(unix)]
fn link_claudine_folder(fixture: &Fixture, root: &Path) -> PathBuf {
    let shared = fixture.root.join("shared");
    std::fs::rename(root.join(".claudine"), &shared).unwrap();
    let link = root.join(".claudine");
    std::os::unix::fs::symlink(&shared, &link).unwrap();
    link
}

#[cfg(unix)]
#[test]
fn a_linked_destination_or_lock_sidecar_is_manual_and_nothing_is_written() {
    for repository in [false, true] {
        for linked in ["claudine", "claudine lock", "darkmatter", "darkmatter lock"] {
            let case = format!("{linked} (repository: {repository})");
            let (fixture, root) = inline_scope(repository);
            let elsewhere = fixture.root.join("elsewhere.yaml");
            let config = root.join(native_path(if linked.starts_with("claudine") { CLAUDINE } else { DARKMATTER }));
            let link = if linked.ends_with("lock") {
                sidecar(&config)
            } else {
                std::fs::copy(&config, &elsewhere).unwrap();
                std::fs::remove_file(&config).unwrap();
                config.clone()
            };
            std::os::unix::fs::symlink(&elsewhere, &link).unwrap();
            let before = fixture.tree();
            let row = fixture.row(true);
            assert_row(&row, Severity::Error, false, &format!("{}: is a symbolic link, which doctor never follows", link.display()));
            assert!(!row.detail.contains("Repair"), "{case}: {row:#?}");
            assert_eq!(fixture.tree(), before, "{case}");
        }
    }
}

#[cfg(unix)]
#[test]
fn a_linked_claudine_folder_is_manual_and_neither_file_nor_sidecar_is_written() {
    for repository in [false, true] {
        // The control: the same layout with an ordinary folder repairs.
        let (control, root) = inline_scope(repository);
        assert_row(&control.row(true), Severity::Ok, false, "refers to Claudine's configuration");
        let reference = if repository { "./.claudine/config.yaml" } else { "~/.claudine/config.yaml" };
        assert_eq!(read(&root.join(DARKMATTER)), format!("permissions: {reference}\n"));

        let (fixture, root) = inline_scope(repository);
        let link = link_claudine_folder(&fixture, &root);
        let claudine = root.join(native_path(CLAUDINE));
        let darkmatter = root.join(DARKMATTER);
        let before = fixture.tree();
        let claudine_before = std::fs::read(&claudine).unwrap();

        assert_row(&fixture.row(false), Severity::Error, false, "is a symbolic link, which doctor never follows");
        let row = fixture.row(true);
        assert_row(&row, Severity::Error, false, &format!("{}: is a symbolic link, which doctor never follows", link.display()));
        assert!(!row.detail.contains("Repair"), "repository: {repository}: {row:#?}");
        assert_eq!(fixture.tree(), before, "repository: {repository}");
        assert_eq!(std::fs::read(&claudine).unwrap(), claudine_before);
        assert_eq!(read(&darkmatter), BLOCK, "the inline policy stays in force");
        for config in [&claudine, &darkmatter] {
            assert!(std::fs::symlink_metadata(sidecar(config)).is_err(), "{} was created", sidecar(config).display());
        }
    }
}

#[cfg(unix)]
#[test]
fn a_folder_linked_after_planning_is_refused_before_the_writer_opens_anything() {
    for repository in [false, true] {
        let (fixture, root) = inline_scope(repository);
        let scope = if repository { ConfigScope::Repository } else { ConfigScope::User };
        let claudine = root.join(native_path(CLAUDINE));
        let context = fixture.context();
        let observation = dm_init::observe(&context, super::darkmatter_scope_of(scope)).unwrap();
        let (bytes, document) = read_document(&claudine).unwrap();
        let side = ClaudineSide { path: claudine.clone(), bytes, document, block_before: None };
        let plan = plan_permissions_handoff(&context, scope, HandoffMode::Doctor, side, &observation).unwrap();
        assert!(plan.edits_claudine() && plan.edits_darkmatter() && plan.obstruction().is_none());

        let link = link_claudine_folder(&fixture, &root);
        let before = fixture.tree();
        let error = commit_permissions_handoff(&context, plan).unwrap_err();

        assert!(matches!(&error, HandoffCommitError::Obstructed(blocker) if blocker.path == link), "{error:?}");
        assert_eq!(fixture.tree(), before, "repository: {repository}");
        assert!(std::fs::symlink_metadata(sidecar(&claudine)).is_err());
    }
}

#[test]
fn a_reference_that_would_not_name_the_planned_file_is_refused_before_any_write() {
    let (fixture, root) = inline_scope(true);
    // A planned target the scope's reference does not spell.
    let claudine = fixture.repository_file(".other/config.yaml", "# repo\nactive_messenger: null\n");
    let context = fixture.context();
    let observation = dm_init::observe(&context, DarkmatterScope::Repository).unwrap();
    let (bytes, document) = read_document(&claudine).unwrap();
    let side = ClaudineSide { path: claudine, bytes, document, block_before: None };
    let plan = plan_permissions_handoff(&context, ConfigScope::Repository, HandoffMode::Doctor, side, &observation).unwrap();
    assert!(plan.edits_darkmatter());

    let before = fixture.tree();
    let error = commit_permissions_handoff(&context, plan).unwrap_err();

    assert!(matches!(error, HandoffCommitError::ReferenceOutOfScope), "{error:?}");
    assert_eq!(fixture.tree(), before);
    assert_eq!(read(&root.join(DARKMATTER)), BLOCK);
}

/// Moves `<root>/.claudine` (`folder`) or its `config.yaml` into
/// `destination` and links it back, so the configuration reads the same
/// bytes through the link. Returns the file the link now resolves to.
#[cfg(unix)]
fn relocate_claudine(root: &Path, destination: &Path, folder: bool) -> PathBuf {
    let (from, to) = if folder {
        (root.join(".claudine"), destination.to_path_buf())
    } else {
        (root.join(native_path(CLAUDINE)), destination.join("config.yaml"))
    };
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::rename(&from, &to).unwrap();
    std::os::unix::fs::symlink(&to, &from).unwrap();
    if folder { to.join("config.yaml") } else { to }
}

/// A scope where Claudine's file already holds [`BLOCK`] and Darkmatter's
/// file sets `permissions` as `darkmatter` spells it, so the repair only
/// writes the reference. The repository scope has the user scope handed off.
fn reference_only_scope(repository: bool, darkmatter: &str) -> (Fixture, PathBuf) {
    let fixture = if repository { Fixture::in_repository() } else { Fixture::new() };
    fixture.user(CLAUDINE, &format!("# mine\n{BLOCK}"));
    if repository {
        fixture.user(DARKMATTER, "permissions: ~/.claudine/config.yaml\n");
        fixture.repository_file(CLAUDINE, &format!("# repo\n{BLOCK}"));
        fixture.repository_file(DARKMATTER, darkmatter);
    } else {
        fixture.user(DARKMATTER, darkmatter);
    }
    let root = if repository { fixture.repo.clone() } else { fixture.home.clone() };
    (fixture, root)
}

/// The Darkmatter settings the planner reads as a reference-only handoff:
/// an omitted setting, an explicit `null`, and an equal inline block.
const REFERENCE_ONLY: [(&str, &str); 3] =
    [("omitted", "register: []\n"), ("null", "permissions: null\nregister: []\n"), ("equal", BLOCK)];

#[cfg(unix)]
#[test]
fn a_reference_only_handoff_through_a_link_repairs_inside_its_scope_and_is_manual_outside() {
    for repository in [false, true] {
        for folder in [false, true] {
            for (setting, darkmatter_text) in REFERENCE_ONLY {
                for inside in [true, false] {
                    let case = format!("repository: {repository}, folder: {folder}, {setting}, inside: {inside}");
                    let (fixture, root) = reference_only_scope(repository, darkmatter_text);
                    let destination = if inside { root.join("dotfiles") } else { fixture.root.join("dotfiles") };
                    let target = relocate_claudine(&root, &destination, folder);
                    let claudine = root.join(native_path(CLAUDINE));
                    let darkmatter = root.join(DARKMATTER);
                    let claudine_before = std::fs::read(&target).unwrap();

                    let row = fixture.row(false);
                    if inside {
                        assert_row(&row, Severity::Error, true, "the repair only refers to it");
                        let fixed = fixture.row(true);
                        assert_row(&fixed, Severity::Ok, false, "refers to Claudine's configuration");
                        let reference = if repository { "./.claudine/config.yaml" } else { "~/.claudine/config.yaml" };
                        assert_eq!(document(&darkmatter)["permissions"], json!(reference), "{case}");
                        assert_eq!(std::fs::read(&target).unwrap(), claudine_before, "{case}");
                        continue;
                    }
                    let scope_root = if repository { "the repository" } else { "your home" };
                    let named = format!(
                        "{} resolves to {}, outside {scope_root}",
                        display_path(&claudine),
                        display_path(&target)
                    );
                    assert_row(&row, Severity::Error, false, &named);
                    assert!(row.detail.contains("replace the link with the file it names"), "{case}: {row:#?}");
                    let before = fixture.tree();
                    let fixed = fixture.row(true);
                    assert_row(&fixed, Severity::Error, false, &named);
                    assert!(!fixed.detail.contains("Repair"), "{case}: nothing was attempted: {fixed:#?}");
                    assert_eq!(fixture.tree(), before, "{case}");
                    assert_eq!(read(&darkmatter), darkmatter_text, "{case}");
                }
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn a_repository_link_outside_its_scope_leaves_the_user_handoff_repairable() {
    let fixture = Fixture::in_repository();
    let user_darkmatter = fixture.user(DARKMATTER, BLOCK);
    fixture.repository_file(CLAUDINE, &format!("# repo\n{BLOCK}"));
    let repo_darkmatter = fixture.repository_file(DARKMATTER, BLOCK);
    let target = relocate_claudine(&fixture.repo, &fixture.root.join("dotfiles"), false);
    let target_before = std::fs::read(&target).unwrap();

    let fixed = fixture.row(true);

    assert_row(&fixed, Severity::Error, false, "outside the repository");
    assert!(fixed.detail.contains(&format!("Repaired: {}: set `permissions`", user_darkmatter.display())), "{fixed:#?}");
    assert_eq!(read(&user_darkmatter), "permissions: ~/.claudine/config.yaml\n");
    assert_eq!(read(&repo_darkmatter), BLOCK);
    assert_eq!(std::fs::read(&target).unwrap(), target_before);
}

#[cfg(unix)]
#[test]
fn a_handoff_that_writes_claudines_file_through_a_link_is_manual_inside_and_outside_its_scope() {
    for repository in [false, true] {
        for folder in [false, true] {
            for inside in [true, false] {
                let case = format!("repository: {repository}, folder: {folder}, inside: {inside}");
                let (fixture, root) = inline_scope(repository);
                let destination = if inside { root.join("dotfiles") } else { fixture.root.join("dotfiles") };
                relocate_claudine(&root, &destination, folder);
                let link = if folder { root.join(".claudine") } else { root.join(native_path(CLAUDINE)) };
                let before = fixture.tree();

                let row = fixture.row(true);

                assert_row(&row, Severity::Error, false, &format!("{}: is a symbolic link, which doctor never follows", link.display()));
                assert!(!row.detail.contains("Repair"), "{case}: {row:#?}");
                assert_eq!(fixture.tree(), before, "{case}");
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn an_authored_reference_to_a_linked_file_keeps_darkmatters_own_verdict() {
    for repository in [false, true] {
        for inside in [true, false] {
            let case = format!("repository: {repository}, inside: {inside}");
            let reference = if repository { "./.claudine/config.yaml" } else { "~/.claudine/config.yaml" };
            let (fixture, root) = reference_only_scope(repository, &format!("permissions: {reference}\n"));
            let destination = if inside { root.join("dotfiles") } else { fixture.root.join("dotfiles") };
            relocate_claudine(&root, &destination, false);
            let before = fixture.tree();

            let row = fixture.row(true);

            // Darkmatter's user policy follows a link out of the home; its
            // repository policy refuses one out of the repository.
            if inside || !repository {
                assert_row(&row, Severity::Ok, false, "refers to Claudine's configuration");
            } else {
                assert_row(&row, Severity::Error, false, "does not supply a valid `permissions` mapping");
            }
            assert!(!row.detail.contains("Repair"), "{case}: {row:#?}");
            assert_eq!(fixture.tree(), before, "{case}");
        }
    }
}

// ---------------------------------------------------------------- redaction

#[test]
fn no_reference_or_block_value_reaches_the_report() {
    let cases = [
        ("logging: true\n", "permissions: ~/.SECRET-reference.yaml\n"),
        ("permissions:\n  shell-commands:\n    - prefix SECRET-mine\n", "permissions:\n  shell-commands:\n    - prefix SECRET-theirs\n"),
        ("permissions: ~/.SECRET-claudine.yaml\n", "register: []\n"),
    ];
    for (claudine, darkmatter) in cases {
        let fixture = Fixture::new();
        fixture.user(".SECRET-claudine.yaml", BLOCK);
        fixture.user(CLAUDINE, claudine);
        fixture.user(DARKMATTER, darkmatter);
        for fix in [false, true] {
            let rows = run_with(&REGISTRY, &fixture.environment(), fix);
            let row = rows.iter().find(|row| row.id == ID).unwrap();
            assert_eq!(row.severity, Severity::Error, "{row:#?}");
            let shown = serde_json::to_string(row).unwrap();
            assert!(!shown.contains("SECRET"), "{shown}");
        }
    }
}

#[test]
fn a_refused_candidate_reaches_the_report_without_its_refusal_text() {
    let darkmatter = PathBuf::from("/home/me/.darkmatter.yaml");
    let shown = |path: &Path| path.display().to_string();
    let edited = |path: &Path| format!("{}: set `permissions`", path.display());
    let refusals = [
        HandoffCommitError::Claudine(crate::error::ClaudineError::ConfigValidation("`network: SECRET-value` is invalid".into())),
        HandoffCommitError::Darkmatter(Box::new(InitError::Write(ConfigWriteError::ValidationFailed {
            path: darkmatter.clone(),
            refusal: darkmatter::config::write::ValidationRefusal::new("expected a mapping, found `SECRET-value`"),
        }))),
    ];
    for refusal in refusals {
        let (changes, reason) = super::failure(refusal, &shown, &edited);
        assert!(changes.is_empty());
        let attempt = crate::doctor::RepairOutcome { changes, failure: Some(reason) };
        let row = super::super::report::row(ID, crate::doctor::CheckOutcome::new("ok"), Some(&attempt));
        assert_eq!(row.severity, Severity::Error, "{row:#?}");
        assert!(row.detail.contains("would be invalid; nothing was saved"), "{row:#?}");
        let shown = serde_json::to_string(&row).unwrap();
        assert!(!shown.contains("SECRET"), "{shown}");
    }
}

// ------------------------------------------------- the file the writer judges

/// Darkmatter's writer judges the whole edited `.darkmatter.yaml`, so a
/// handoff that edits a file with an invalid `register` setting is manual:
/// the row is not fixable, `--fix` leaves every byte in place, and the
/// writer itself refuses the same plan. A valid or unwritten setting keeps
/// the handoff repairable.
macro_rules! register_shape_cases {
    ($($name:ident: repository $repository:expr, $register:expr => $repairable:expr;)*) => {$(
        #[test]
        fn $name() {
            let (fixture, root) = inline_scope($repository);
            let darkmatter = write(&root.join(DARKMATTER), &format!("{BLOCK}{}", $register));
            let claudine = root.join(native_path(CLAUDINE));
            let label = if $repository { "repository" } else { "user" };

            let row = fixture.row(false);

            assert_row(&row, Severity::Error, $repairable, &format!("the {label} `permissions` is an inline block"));
            let manual = "the repair cannot edit this file while its `register` setting is invalid";
            assert_eq!(row.detail.contains(manual), !$repairable, "{row:#?}");
            if $repairable {
                let fixed = fixture.row(true);
                assert_row(&fixed, Severity::Ok, false, "refers to Claudine's configuration");
                assert_eq!(document(&claudine)["permissions"], document_of(BLOCK)["permissions"]);
                assert!(read(&darkmatter).ends_with(&format!("{}", $register)), "{}", read(&darkmatter));
                return;
            }
            assert!(row.detail.contains(&display_path(&darkmatter)), "names the file: {row:#?}");
            assert!(row.detail.contains("Remedy: correct the `register` setting, then run `claudine doctor --fix`"), "{row:#?}");
            let before = fixture.tree();
            let fixed = fixture.row(true);
            assert_row(&fixed, Severity::Error, false, "is an inline block");
            assert!(!fixed.detail.contains("Repaired") && !fixed.summary.contains("the repair failed"), "{fixed:#?}");
            assert_eq!(fixture.tree(), before, "the writer never opened: no byte, time, or lock sidecar changed");

            // The premise: the writer refuses this plan on every attempt.
            let context = fixture.context();
            let scope = if $repository { DarkmatterScope::Repository } else { DarkmatterScope::User };
            let config_scope = if $repository { ConfigScope::Repository } else { ConfigScope::User };
            let observation = dm_init::observe(&context, scope).unwrap();
            let (bytes, document) = read_document(&claudine).unwrap();
            let side = ClaudineSide { path: claudine.clone(), bytes, document, block_before: None };
            let plan = plan_permissions_handoff(&context, config_scope, HandoffMode::Doctor, side, &observation).unwrap();
            let error = commit_permissions_handoff(&context, plan).unwrap_err();
            assert!(
                matches!(&error, HandoffCommitError::Darkmatter(error)
                    if matches!(**error, InitError::Write(ConfigWriteError::ValidationFailed { .. }))),
                "{error:?}"
            );
            assert_eq!(read(&darkmatter), format!("{BLOCK}{}", $register), "the original inline block stays");
        }
    )*};
}

register_shape_cases! {
    user_handoff_with_an_omitted_register_is_repairable: repository false, "" => true;
    user_handoff_with_a_null_register_is_repairable: repository false, "register: null\n" => true;
    user_handoff_with_an_empty_register_is_repairable: repository false, "register: []\n" => true;
    user_handoff_with_a_valid_register_is_repairable: repository false, "register:\n  - ~/hosts/mine.yaml\n" => true;
    user_handoff_with_a_scalar_register_is_manual: repository false, "register: 3\n" => false;
    user_handoff_with_a_mixed_register_is_manual: repository false, "register:\n  - ~/hosts/mine.yaml\n  - 3\n" => false;
    user_handoff_with_an_all_invalid_register_is_manual: repository false, "register:\n  - 3\n  - ''\n" => false;
    repository_handoff_with_an_omitted_register_is_repairable: repository true, "" => true;
    repository_handoff_with_a_null_register_is_repairable: repository true, "register: null\n" => true;
    repository_handoff_with_an_empty_register_is_repairable: repository true, "register: []\n" => true;
    repository_handoff_with_a_valid_register_is_repairable: repository true, "register:\n  - ./hosts/mine.yaml\n" => true;
    repository_handoff_with_a_scalar_register_is_manual: repository true, "register: 3\n" => false;
    repository_handoff_with_a_mixed_register_is_manual: repository true, "register:\n  - ./hosts/mine.yaml\n  - 3\n" => false;
    repository_handoff_with_an_all_invalid_register_is_manual: repository true, "register:\n  - 3\n  - ''\n" => false;
}

fn document_of(text: &str) -> Value {
    load_yaml_document(text, Path::new("inline.yaml")).unwrap().json
}

fn display_path(path: &Path) -> String {
    super::super::report::display_path(path)
}

#[test]
fn an_invalid_repository_register_leaves_the_user_handoff_repairable() {
    let (fixture, [user_claudine, user_darkmatter, repo_claudine, repo_darkmatter]) = two_inline_scopes();
    write(&repo_darkmatter, &format!("{OTHER_BLOCK}register: 3\n"));
    let repo_before = (std::fs::read(&repo_claudine).unwrap(), std::fs::read(&repo_darkmatter).unwrap());

    let row = fixture.row(false);
    assert_row(&row, Severity::Error, true, "the repository `permissions` is an inline block");

    let fixed = fixture.row(true);
    assert_eq!((fixed.severity, fixed.fixable), (Severity::Error, false), "{fixed:#?}");
    assert!(!fixed.summary.contains("the repair failed"), "nothing was attempted for the repository: {fixed:#?}");
    for path in [&user_claudine, &user_darkmatter] {
        assert!(fixed.detail.contains(&format!("Repaired: {}: set `permissions`", path.display())), "{fixed:#?}");
    }
    assert_eq!(fixed.detail.matches("Repaired: ").count(), 2, "{fixed:#?}");
    assert_eq!(read(&user_darkmatter), "permissions: ~/.claudine/config.yaml\n");
    assert_eq!((std::fs::read(&repo_claudine).unwrap(), std::fs::read(&repo_darkmatter).unwrap()), repo_before);
}

#[test]
fn an_existing_reference_keeps_its_verdict_beside_an_invalid_register() {
    for repository in [false, true] {
        let fixture = if repository { Fixture::in_repository() } else { Fixture::new() };
        fixture.user(CLAUDINE, BLOCK);
        let user_register = if repository { "" } else { "register: 3\n" };
        fixture.user(DARKMATTER, &format!("permissions: ~/.claudine/config.yaml\n{user_register}"));
        if repository {
            fixture.repository_file(CLAUDINE, &format!("# repo\n{OTHER_BLOCK}"));
            fixture.repository_file(DARKMATTER, "permissions: ./.claudine/config.yaml\nregister: 3\n");
        }
        let before = fixture.tree();

        let row = fixture.row(true);

        assert_row(&row, Severity::Ok, false, "refers to Claudine's configuration");
        assert!(!row.detail.contains("`register`"), "{row:#?}");
        assert_eq!(fixture.tree(), before, "no edit was planned");
    }
}

// ------------------------------------------------- the editor's layout (R4-F2)

/// Which file of the pair a layout case concerns.
#[derive(Debug, Clone, Copy)]
enum Side {
    Claudine,
    Darkmatter,
}

/// What a layout case expects of the handoff.
#[derive(Debug, Clone, Copy)]
enum LayoutOutcome {
    /// The planned edit of this file meets the editor's layout refusal at its
    /// root: the scope is manual and neither file changes.
    ManualRoot(Side),
    /// The same, at this file's `permissions` setting.
    ManualSetting(Side),
    /// The handoff is repaired; `claudine_kept` when Claudine's file needs
    /// no edit and keeps every byte.
    Repaired { claudine_kept: bool },
}

/// Writes one scope's pair (the repository scope with the user scope
/// already handed off) and checks the row, `--fix`, and the writer.
fn assert_layout_case(repository: bool, claudine_text: &str, darkmatter_text: &str, outcome: LayoutOutcome) {
    let (fixture, root) = inline_scope(repository);
    let claudine = write(&root.join(native_path(CLAUDINE)), claudine_text);
    let darkmatter = write(&root.join(DARKMATTER), darkmatter_text);
    let label = if repository { "repository" } else { "user" };
    let reference = if repository { "./.claudine/config.yaml" } else { "~/.claudine/config.yaml" };

    let row = fixture.row(false);

    match outcome {
        LayoutOutcome::ManualRoot(side) | LayoutOutcome::ManualSetting(side) => {
            let file = match side {
                Side::Claudine => &claudine,
                Side::Darkmatter => &darkmatter,
            };
            let shown = display_path(file);
            let (problem, part) = match outcome {
                LayoutOutcome::ManualRoot(_) => (
                    format!("the root mapping of {shown} is written inline, which Darkmatter's format-preserving editor cannot change in place"),
                    format!("the root mapping of {shown}"),
                ),
                _ => (
                    format!("cannot change the `permissions` setting of {shown} in place, because the edit reaches an inline `{{…}}` or `[…]` collection, or a value with an anchor, alias, or tag"),
                    format!("the `permissions` setting of {shown}"),
                ),
            };
            assert_row(&row, Severity::Error, false, &format!("the {label} `permissions`"));
            assert!(row.detail.contains(&problem), "{row:#?}");
            assert!(!row.detail.contains("not written in block form"), "{row:#?}");
            assert!(row.detail.contains(&format!("Remedy: rewrite {part} in block form")), "{row:#?}");
            assert!(!row.detail.contains("network") && !row.detail.contains("claude,"), "no authored value: {row:#?}");
            let before = fixture.tree();
            let fixed = fixture.row(true);
            assert!(!fixed.fixable && !fixed.detail.contains("Repaired"), "{fixed:#?}");
            assert_eq!(fixture.tree(), before, "neither file, time, nor lock sidecar changed");

            // The premise: the writer refuses the same plan and saves neither file.
            let context = fixture.context();
            let (scope, config_scope) = if repository {
                (DarkmatterScope::Repository, ConfigScope::Repository)
            } else {
                (DarkmatterScope::User, ConfigScope::User)
            };
            let observation = dm_init::observe(&context, scope).unwrap();
            let (bytes, document) = read_document(&claudine).unwrap();
            let side = ClaudineSide { path: claudine.clone(), bytes, document, block_before: None };
            let plan = plan_permissions_handoff(&context, config_scope, HandoffMode::Doctor, side, &observation).unwrap();
            let error = commit_permissions_handoff(&context, plan).unwrap_err();
            assert!(
                matches!(&error, HandoffCommitError::Darkmatter(error)
                    if matches!(**error, InitError::Write(ConfigWriteError::UnsupportedEdit { .. }))),
                "{error:?}"
            );
            assert_eq!((read(&claudine).as_str(), read(&darkmatter).as_str()), (claudine_text, darkmatter_text));
        }
        LayoutOutcome::Repaired { claudine_kept } => {
            assert_row(&row, Severity::Error, true, &format!("the {label} `permissions`"));
            let fixed = fixture.row(true);
            assert_row(&fixed, Severity::Ok, false, "refers to Claudine's configuration");
            assert_eq!(document(&darkmatter)["permissions"], json!(reference));
            assert_eq!(document(&claudine)["permissions"]["network"], json!("deny"));
            if claudine_kept {
                assert_eq!(read(&claudine), claudine_text);
            }
        }
    }
}

macro_rules! layout_cases {
    ($($name:ident: repository $repository:expr, claudine $claudine:expr, darkmatter $darkmatter:expr => $outcome:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_layout_case($repository, $claudine, $darkmatter, $outcome);
        }
    )*};
}

layout_cases! {
    // R4-F2.3: a transfer into an inline Claudine root, and its block control.
    user_transfer_into_an_inline_claudine_root_is_manual:
        repository false, claudine "{canonical_provider: claude}\n", darkmatter "permissions: {network: deny}\n"
        => LayoutOutcome::ManualRoot(Side::Claudine);
    repository_transfer_into_an_inline_claudine_root_is_manual:
        repository true, claudine "{active_messenger: null}\n", darkmatter "permissions: {network: deny}\n"
        => LayoutOutcome::ManualRoot(Side::Claudine);
    user_transfer_into_a_block_claudine_root_is_repaired:
        repository false, claudine "canonical_provider: claude\n", darkmatter "permissions: {network: deny}\n"
        => LayoutOutcome::Repaired { claudine_kept: false };
    repository_transfer_into_a_block_claudine_root_is_repaired:
        repository true, claudine "active_messenger: null\n", darkmatter "permissions: {network: deny}\n"
        => LayoutOutcome::Repaired { claudine_kept: false };
    // R4-F2.4: an inline Darkmatter root, with an equal Claudine block
    // (reference only) and with none (both files).
    user_reference_only_into_an_inline_darkmatter_root_is_manual:
        repository false, claudine "permissions:\n  network: deny\n", darkmatter "{register: [], permissions: {network: deny}}\n"
        => LayoutOutcome::ManualRoot(Side::Darkmatter);
    repository_reference_only_into_an_inline_darkmatter_root_is_manual:
        repository true, claudine "permissions:\n  network: deny\n", darkmatter "{register: [], permissions: {network: deny}}\n"
        => LayoutOutcome::ManualRoot(Side::Darkmatter);
    user_two_file_handoff_into_an_inline_darkmatter_root_is_manual:
        repository false, claudine "# my settings\nlogging: true\n", darkmatter "{register: [], permissions: {network: deny}}\n"
        => LayoutOutcome::ManualRoot(Side::Darkmatter);
    repository_two_file_handoff_into_an_inline_darkmatter_root_is_manual:
        repository true, claudine "# repo\nactive_messenger: null\n", darkmatter "{register: [], permissions: {network: deny}}\n"
        => LayoutOutcome::ManualRoot(Side::Darkmatter);
    // R5-F3.1: a block `permissions` mapping with an anchor or a tag, with an
    // equal Claudine block (reference only) and with none (both files).
    user_reference_only_over_an_anchored_darkmatter_block_is_manual:
        repository false, claudine "permissions:\n  network: deny\n", darkmatter "permissions: &policy\n  network: deny\n"
        => LayoutOutcome::ManualSetting(Side::Darkmatter);
    repository_reference_only_over_an_anchored_darkmatter_block_is_manual:
        repository true, claudine "permissions:\n  network: deny\n", darkmatter "permissions: &policy\n  network: deny\n"
        => LayoutOutcome::ManualSetting(Side::Darkmatter);
    user_two_file_handoff_of_an_anchored_darkmatter_block_is_manual:
        repository false, claudine "# my settings\nlogging: true\n", darkmatter "permissions: &policy\n  network: deny\n"
        => LayoutOutcome::ManualSetting(Side::Darkmatter);
    repository_two_file_handoff_of_an_anchored_darkmatter_block_is_manual:
        repository true, claudine "# repo\nactive_messenger: null\n", darkmatter "permissions: &policy\n  network: deny\n"
        => LayoutOutcome::ManualSetting(Side::Darkmatter);
    user_reference_only_over_a_tagged_darkmatter_block_is_manual:
        repository false, claudine "permissions:\n  network: deny\n", darkmatter "permissions: !!map\n  network: deny\n"
        => LayoutOutcome::ManualSetting(Side::Darkmatter);
    repository_reference_only_over_a_tagged_darkmatter_block_is_manual:
        repository true, claudine "permissions:\n  network: deny\n", darkmatter "permissions: !!map\n  network: deny\n"
        => LayoutOutcome::ManualSetting(Side::Darkmatter);
    user_two_file_handoff_of_a_tagged_darkmatter_block_is_manual:
        repository false, claudine "# my settings\nlogging: true\n", darkmatter "permissions: !!map\n  network: deny\n"
        => LayoutOutcome::ManualSetting(Side::Darkmatter);
    repository_two_file_handoff_of_a_tagged_darkmatter_block_is_manual:
        repository true, claudine "# repo\nactive_messenger: null\n", darkmatter "permissions: !!map\n  network: deny\n"
        => LayoutOutcome::ManualSetting(Side::Darkmatter);
    // R4-F2.c2: edits the editor makes, whatever inline collection is nearby.
    user_empty_inline_claudine_root_takes_its_first_key:
        repository false, claudine "{}\n", darkmatter "permissions:\n  network: deny\n"
        => LayoutOutcome::Repaired { claudine_kept: false };
    repository_empty_inline_claudine_root_takes_its_first_key:
        repository true, claudine "{}\n", darkmatter "permissions:\n  network: deny\n"
        => LayoutOutcome::Repaired { claudine_kept: false };
    user_reference_only_leaves_an_inline_claudine_root_alone:
        repository false, claudine "{canonical_provider: claude, permissions: {network: deny}}\n", darkmatter "permissions:\n  network: deny\n"
        => LayoutOutcome::Repaired { claudine_kept: true };
    repository_reference_only_leaves_an_inline_claudine_root_alone:
        repository true, claudine "{active_messenger: null, permissions: {network: deny}}\n", darkmatter "permissions:\n  network: deny\n"
        => LayoutOutcome::Repaired { claudine_kept: true };
}

/// R4-F2.3/.4: a scope the editor's layout blocks does not stop the other
/// scope's handoff in the same `--fix`.
#[test]
fn a_scope_blocked_by_its_layout_leaves_the_other_scope_repairable() {
    let (fixture, [user_claudine, user_darkmatter, repo_claudine, repo_darkmatter]) = two_inline_scopes();
    let inline = "{canonical_provider: claude}\n";
    write(&user_claudine, inline);

    let row = fixture.row(true);

    assert!(row.detail.contains(&format!("the root mapping of {}", display_path(&user_claudine))), "{row:#?}");
    assert!(row.detail.contains(&format!("Repaired: {}: set `permissions`", repo_darkmatter.display())), "{row:#?}");
    assert_eq!((read(&user_claudine).as_str(), read(&user_darkmatter).as_str()), (inline, BLOCK));
    assert_eq!(document(&repo_claudine)["permissions"], document_of(OTHER_BLOCK)["permissions"]);
    assert_eq!(read(&repo_darkmatter), "permissions: ./.claudine/config.yaml\n");
}
