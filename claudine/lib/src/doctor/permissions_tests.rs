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
use super::super::{CheckDescriptor, DoctorRow, Environment, Severity, native_path, run_with};
use super::{check_permissions, repair_permissions};
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
    assert_eq!(fixture.tree(), before);
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
    assert!(failed.summary.starts_with("the repair failed: could not replace"), "{failed:#?}");
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

#[cfg(unix)]
#[test]
fn a_linked_destination_or_lock_sidecar_is_manual_and_nothing_is_written() {
    for linked in ["destination", "lock"] {
        let fixture = Fixture::new();
        let elsewhere = fixture.root.join("elsewhere.yaml");
        write(&elsewhere, BLOCK);
        let darkmatter = fixture.home.join(DARKMATTER);
        match linked {
            "destination" => std::os::unix::fs::symlink(&elsewhere, &darkmatter).unwrap(),
            _ => {
                write(&darkmatter, BLOCK);
                let lock = fixture.home.join(format!("{DARKMATTER}{}", darkmatter::config::write::LOCK_SUFFIX));
                std::os::unix::fs::symlink(fixture.root.join("lock-target"), lock).unwrap();
            }
        }
        let before = fixture.tree();
        let row = fixture.row(true);
        assert_row(&row, Severity::Error, false, "is a symbolic link, which the repair does not write through");
        assert_eq!(fixture.tree(), before, "{linked}");
        assert!(!fixture.root.join("lock-target").exists(), "{linked}");
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
