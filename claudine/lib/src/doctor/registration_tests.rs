//! The `registration` row over real temporary homes.
//!
//! Inspections run through the built-in registry. Repairs run through a
//! registry of this row alone, so the permissions repair (which edits the
//! same `.darkmatter.yaml`) cannot change the bytes these tests compare.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};

use super::super::{CheckDescriptor, DoctorRow, Environment, Severity, file_checks, native_path, run_with};
use super::{check_registration, repair_registration};
use crate::composition::registration::{PublishedFile, RegistrationInventory, publish};

/// The `register` entry setup writes, spelled out: the published-tree
/// locators are kept inside the registration module.
const ENTRY: &str = "~/.claudine/registrations/darkmatter/manifest.yaml";
const ROOT: &str = ".claudine/registrations/darkmatter";

static ONLY_REGISTRATION: [CheckDescriptor; 1] = [CheckDescriptor {
    id: "registration",
    title: "Darkmatter host registration",
    prerequisites: &[],
    check: check_registration,
    repair: Some(repair_registration),
}];

struct Fixture {
    _dir: tempfile::TempDir,
    home: PathBuf,
}

impl Fixture {
    /// An empty home: no registration and no `.darkmatter.yaml`.
    fn empty() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let home = canonicalize_simplified(dir.path()).unwrap().join("home");
        std::fs::create_dir_all(home.join(".claudine")).unwrap();
        Self { _dir: dir, home }
    }

    /// The state setup leaves: the tree published and the entry registered.
    fn set_up() -> Self {
        let fixture = Self::empty();
        fixture.publish(RegistrationInventory::claudine());
        fixture.config(&format!("# my settings\nregister:\n    - {ENTRY}\n"));
        fixture
    }

    fn publish(&self, inventory: &RegistrationInventory) {
        publish(&self.home, inventory).unwrap();
    }

    fn config(&self, text: &str) {
        std::fs::write(self.home.join(".darkmatter.yaml"), text).unwrap();
    }

    fn config_text(&self) -> String {
        std::fs::read_to_string(self.home.join(".darkmatter.yaml")).unwrap()
    }

    fn root(&self) -> PathBuf {
        self.home.join(native_path(ROOT))
    }

    fn legacy(&self) {
        std::fs::write(self.home.join(native_path(".claudine/darkmatter-host.yaml")), "kind: darkmatter-host\n").unwrap();
        std::fs::create_dir_all(self.home.join(native_path(".claudine/darkmatter-host/r1"))).unwrap();
        std::fs::write(self.home.join(native_path(".claudine/darkmatter-host/r1/a.yaml")), "kind: schema\n").unwrap();
    }

    fn environment(&self) -> Environment {
        let context = FileResolutionContext::from_snapshot(self.home.clone(), Some(self.home.clone()), HashMap::new());
        Environment::for_test(Some(self.home.clone()), Ok(context))
    }

    /// The `registration` row of an inspection by the built-in registry.
    fn check(&self) -> DoctorRow {
        row(run_with(file_checks(), &self.environment(), false))
    }

    /// The `registration` row after `--fix`.
    fn fix(&self) -> DoctorRow {
        row(run_with(&ONLY_REGISTRATION, &self.environment(), true))
    }

    /// Every entry under the temporary root, read without following links.
    fn snapshot(&self) -> Vec<(PathBuf, Option<Vec<u8>>, SystemTime)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Option<Vec<u8>>, SystemTime)>) {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().path()).collect();
            entries.sort();
            for path in entries {
                let metadata = std::fs::symlink_metadata(&path).unwrap();
                let modified = metadata.modified().unwrap();
                if metadata.is_dir() {
                    out.push((path.clone(), None, modified));
                    walk(&path, out);
                } else if metadata.is_file() {
                    out.push((path.clone(), Some(std::fs::read(&path).unwrap()), modified));
                } else {
                    out.push((path, None, modified));
                }
            }
        }
        let mut out = Vec::new();
        walk(self.home.parent().unwrap(), &mut out);
        out
    }
}

fn row(rows: Vec<DoctorRow>) -> DoctorRow {
    rows.into_iter().find(|row| row.id == "registration").expect("a `registration` row")
}

#[track_caller]
fn assert_row(row: &DoctorRow, severity: Severity, fixable: bool, needle: &str) {
    assert_eq!((row.severity, row.fixable), (severity, fixable), "{row:#?}");
    assert!(row.summary.contains(needle) || row.detail.contains(needle), "`{needle}` not in {row:#?}");
}

/// Asserts a `--fix` repaired the row and the next inspection is clean.
#[track_caller]
fn assert_repaired(fixture: &Fixture, row: &DoctorRow) {
    assert_row(row, Severity::Ok, false, "Repaired:");
    assert_row(&fixture.check(), Severity::Ok, false, "the registration is published and registered");
}

// ---------------------------------------------------------------- control

#[test]
fn the_state_setup_leaves_passes() {
    let fixture = Fixture::set_up();
    let before = fixture.snapshot();

    let row = fixture.check();

    assert_row(&row, Severity::Ok, false, "the registration is published and registered");
    assert_eq!(row.detail, "");
    assert_eq!(fixture.snapshot(), before, "an inspection writes nothing");
}

#[test]
fn without_a_home_the_row_is_informational() {
    let environment = Environment::for_test(None, Err(super::super::ContextUnavailable));
    let row = row(run_with(file_checks(), &environment, false));
    assert_row(&row, Severity::Info, false, "the home directory cannot be located");
}

// ---------------------------------------------------------------- the tree

#[test]
fn a_missing_tree_is_a_fixable_error_and_the_fix_publishes_it() {
    let fixture = Fixture::empty();
    fixture.config(&format!("register:\n    - {ENTRY}\n"));

    assert_row(&fixture.check(), Severity::Error, true, "the published registration is missing");
    let fixed = fixture.fix();

    assert_repaired(&fixture, &fixed);
    assert!(fixed.detail.contains(&format!("Repaired: wrote {}", fixture.root().join("manifest.yaml").display())), "{fixed:#?}");
}

#[test]
fn a_missing_member_is_a_fixable_error() {
    let fixture = Fixture::set_up();
    let member = fixture.root().join(native_path("schemas/shared.yaml"));
    std::fs::remove_file(&member).unwrap();

    let row = fixture.check();

    assert_row(&row, Severity::Error, true, &format!("{}: is missing", member.display()));
    assert_repaired(&fixture, &fixture.fix());
    assert!(member.is_file());
}

#[test]
fn a_stale_member_is_a_fixable_warning_that_explains_the_digest() {
    let fixture = Fixture::set_up();
    let stale = RegistrationInventory::claudine().clone().with_member(PublishedFile::new(
        "schemas/shared.yaml",
        crate::composition::registration::FileKind::Library,
        None::<&'static str>,
        b"kind: schema\ntypes:\n    x: string\n".to_vec(),
    ));
    let member = fixture.root().join(native_path("schemas/shared.yaml"));
    std::fs::write(&member, stale.file("schemas/shared.yaml").unwrap().bytes()).unwrap();

    let row = fixture.check();

    assert_row(&row, Severity::Warn, true, "differs from the file this Claudine publishes");
    assert!(row.detail.contains("refuses a member whose bytes do not match the manifest's digest"), "{row:#?}");
    assert_repaired(&fixture, &fixture.fix());
}

#[test]
fn a_changed_manifest_darkmatter_still_loads_is_a_warning_and_one_it_refuses_an_error() {
    let fixture = Fixture::set_up();
    // Another release's manifest over the same members still verifies.
    fixture.publish(&RegistrationInventory::claudine().clone().with_version("0.0.0-elsewhere"));
    assert_row(&fixture.check(), Severity::Warn, true, "Darkmatter still accepts it as Claudine's registration");

    std::fs::write(fixture.root().join("manifest.yaml"), "kind: darkmatter-host\nformat_version: 2\nhost: someone-else\n").unwrap();
    assert_row(&fixture.check(), Severity::Error, true, "Darkmatter refuses it as Claudine's registration");

    assert_repaired(&fixture, &fixture.fix());
}

#[test]
fn extras_are_removable_warnings_and_the_fix_removes_only_them() {
    let fixture = Fixture::set_up();
    let nested = fixture.root().join(native_path("schemas/old"));
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("stale.yaml"), "x").unwrap();
    std::fs::write(fixture.root().join("notes.txt"), "x").unwrap();

    let row = fixture.check();

    assert_row(&row, Severity::Warn, true, &format!("{}: is not part of the published registration", nested.join("stale.yaml").display()));
    assert!(row.detail.contains(&format!("{}: is a folder that is not part", nested.display())), "{row:#?}");
    let fixed = fixture.fix();
    assert_repaired(&fixture, &fixed);
    let line = |wanted: String| fixed.detail.lines().position(|line| line == wanted).unwrap_or_else(|| panic!("{wanted}: {fixed:#?}"));
    let file = line(format!("Repaired: removed {}", nested.join("stale.yaml").display()));
    let folder = line(format!("Repaired: removed {}", nested.display()));
    assert!(file < folder, "deepest first: {fixed:#?}");
    assert!(fixture.root().join("schemas").is_dir() && !nested.exists());
}

#[test]
fn the_retired_layout_is_a_warning_and_the_fix_removes_it_after_moving_the_entry() {
    let fixture = Fixture::set_up();
    fixture.config("register:\n    - ~/.claudine/darkmatter-host.yaml\n");
    fixture.legacy();

    let row = fixture.check();

    assert_row(&row, Severity::Error, true, "no `register` entry names Claudine's manifest");
    let legacy = native_path(".claudine/darkmatter-host.yaml");
    assert!(row.detail.contains(&format!("{}: is left over from the retired format 1 registration", legacy.display())), "{row:#?}");
    let fixed = fixture.fix();
    assert_repaired(&fixture, &fixed);
    let edited = fixed.detail.find("updated the `register` entry").unwrap();
    let removed = fixed.detail.find(&format!("removed {}", fixture.home.join(native_path(".claudine/darkmatter-host.yaml")).display())).unwrap();
    assert!(edited < removed, "{fixed:#?}");
    assert_eq!(fixture.config_text(), format!("register:\n    - {ENTRY}\n"));
}

// ---------------------------------------------------------------- the entry

#[test]
fn an_equivalent_spelling_of_the_entry_passes() {
    let fixture = Fixture::set_up();
    fixture.config(&format!("register:\n    - {}\n", fixture.root().join("manifest.yaml").display()));
    assert_row(&fixture.check(), Severity::Ok, false, "the registration is published and registered");
}

#[test]
fn a_missing_entry_is_a_fixable_error_and_other_hosts_entries_survive_the_fix() {
    let fixture = Fixture::set_up();
    let before = "# my settings\nregister:\n    # the acme host\n    - ~/.acme/manifest.yaml # pinned\n";
    fixture.config(before);

    assert_row(&fixture.check(), Severity::Error, true, "no `register` entry names Claudine's manifest");
    assert_repaired(&fixture, &fixture.fix());

    assert_eq!(fixture.config_text(), format!("{before}    - {ENTRY}\n"));
}

#[test]
fn a_missing_or_invalid_user_file_blocks_the_repair_and_nothing_is_written() {
    for text in [None, Some("register: [\n"), Some("register: 5\n")] {
        let fixture = Fixture::empty();
        if let Some(text) = text {
            fixture.config(text);
        }
        let before = fixture.snapshot();

        let row = fixture.check();
        let fixed = fixture.fix();

        assert_row(&row, Severity::Error, false, "the published registration is missing");
        assert!(row.detail.contains("the automatic repair is unavailable"), "{text:?}: {row:#?}");
        assert_eq!(fixed, row, "{text:?}: no repair is attempted");
        assert_eq!(fixture.snapshot(), before, "{text:?}");
    }
}

#[test]
fn a_current_tree_with_a_missing_user_file_reports_the_entry_as_blocked() {
    let fixture = Fixture::empty();
    fixture.publish(RegistrationInventory::claudine());
    assert_row(&fixture.check(), Severity::Info, false, "the `register` entry cannot be verified: the file does not exist");
}

// ---------------------------------------------------------------- idempotence

#[test]
fn a_second_fix_changes_nothing() {
    let fixture = Fixture::empty();
    fixture.config("register:\n    - ~/.claudine/darkmatter-host.yaml\n");
    fixture.legacy();
    assert_repaired(&fixture, &fixture.fix());
    let before = fixture.snapshot();

    let again = fixture.fix();

    assert_row(&again, Severity::Ok, false, "the registration is published and registered");
    assert!(!again.detail.contains("Repaired"), "{again:#?}");
    assert_eq!(fixture.snapshot(), before);
}

// ---------------------------------------------------------------- redaction

#[test]
fn rows_never_show_an_authored_reference() {
    let fixture = Fixture::empty();
    fixture.config("register:\n    - ~/SECRET/manifest.yaml\n    - https://SECRET.example/manifest.yaml\n");

    let inspected = serde_json::to_string(&fixture.check()).unwrap();
    let fixed = serde_json::to_string(&fixture.fix()).unwrap();

    assert!(!inspected.contains("SECRET") && !fixed.contains("SECRET"), "{inspected}\n{fixed}");
}

// ---------------------------------------------------------------- obstructions

#[cfg(unix)]
mod obstructions {
    use super::*;

    /// Asserts the row is a manual error naming `path`, that `--fix` writes
    /// nothing, and that the link's target is untouched.
    #[track_caller]
    fn assert_manual(fixture: &Fixture, path: &Path, target: &Path) {
        let target_before = std::fs::read_dir(target).map(|entries| entries.count()).ok();
        let before = fixture.snapshot();

        let row = fixture.check();
        let fixed = fixture.fix();

        assert_row(&row, Severity::Error, false, &format!("{}: is a symbolic link, which doctor never follows", path.display()));
        assert_eq!(fixed, row, "no repair is attempted");
        assert_eq!(fixture.snapshot(), before);
        assert_eq!(std::fs::read_dir(target).map(|entries| entries.count()).ok(), target_before);
    }

    fn outside(fixture: &Fixture) -> PathBuf {
        let outside = fixture.home.parent().unwrap().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        outside
    }

    #[test]
    fn a_link_at_the_output_root() {
        let fixture = Fixture::empty();
        fixture.config(&format!("register:\n    - {ENTRY}\n"));
        let outside = outside(&fixture);
        std::fs::create_dir_all(fixture.home.join(native_path(".claudine/registrations"))).unwrap();
        std::os::unix::fs::symlink(&outside, fixture.root()).unwrap();
        assert_manual(&fixture, &fixture.root(), &outside);
    }

    #[test]
    fn a_link_among_the_extras() {
        let fixture = Fixture::set_up();
        let outside = outside(&fixture);
        std::fs::write(fixture.root().join("notes.txt"), "x").unwrap();
        let link = fixture.root().join(native_path("schemas/elsewhere"));
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        assert_manual(&fixture, &link, &outside);
    }

    #[test]
    fn a_link_in_the_retired_layout() {
        let fixture = Fixture::set_up();
        let outside = outside(&fixture);
        let link = fixture.home.join(native_path(".claudine/darkmatter-host"));
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        assert_manual(&fixture, &link, &outside);
    }

    #[test]
    fn a_link_at_the_config_writers_lock_sidecar_when_a_repair_is_needed() {
        let fixture = Fixture::set_up();
        fixture.config("register: []\n");
        let outside = outside(&fixture);
        let link = fixture.home.join(".darkmatter.yaml.darkmatter-lock");
        std::os::unix::fs::symlink(outside.join("lock"), &link).unwrap();
        assert_manual(&fixture, &link, &outside);
    }
}
