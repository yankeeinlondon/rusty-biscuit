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
    assert!(row.detail.contains("its bytes do not match the manifest's digest"), "{row:#?}");
    assert_repaired(&fixture, &fixture.fix());
}

#[test]
fn a_changed_manifest_darkmatter_still_loads_is_a_warning_and_one_it_refuses_an_error() {
    let fixture = Fixture::set_up();
    // Another release's manifest over the same members still verifies.
    fixture.publish(&RegistrationInventory::claudine().clone().with_version("0.0.0-elsewhere"));
    assert_row(&fixture.check(), Severity::Warn, true, "Darkmatter still loads the bundle it lists as Claudine's registration");

    std::fs::write(fixture.root().join("manifest.yaml"), "kind: darkmatter-host\nformat_version: 2\nhost: someone-else\n").unwrap();
    assert_row(&fixture.check(), Severity::Error, true, "Darkmatter refuses the bundle it lists as Claudine's registration");

    assert_repaired(&fixture, &fixture.fix());
}

// ---------------------------------------------------------------- edited bundles

/// Publishes Claudine's registration into `home` with the library publisher,
/// then rewrites the published member `member` (`schemas/action.yaml`,
/// `globals.lifecycle.json`) with `edit` of its published bytes and the
/// manifest with that member's new digest, as the inventory derives every
/// manifest. Returns the edited member's path.
///
/// Built on the public publishing API only, so a CLI-process test can share
/// the shape.
pub(crate) fn publish_with_edited_member(home: &Path, member: &str, edit: impl FnOnce(&str) -> String) -> PathBuf {
    publish(home, RegistrationInventory::claudine()).unwrap();
    let root = home.join(native_path(ROOT));
    let path = root.join(native_path(member));
    let published = std::fs::read_to_string(&path).unwrap();
    let original = RegistrationInventory::claudine().file(member).unwrap_or_else(|| panic!("`{member}` is published"));
    let edited = RegistrationInventory::claudine().clone().with_member(PublishedFile::new(
        member.to_string(),
        original.kind(),
        original.name().map(str::to_string),
        edit(&published).into_bytes(),
    ));
    std::fs::write(&path, edited.file(member).unwrap().bytes()).unwrap();
    std::fs::write(root.join("manifest.yaml"), edited.manifest().bytes()).unwrap();
    path
}

const INVALID_YAML: &str = "\nbroken: [\n";
const COMMENT: &str = "\n# a local note\n";

impl Fixture {
    /// [`Fixture::set_up`] with `member` edited by [`publish_with_edited_member`].
    fn with_edited_member(member: &str, edit: impl FnOnce(&str) -> String) -> Self {
        let fixture = Self::set_up();
        publish_with_edited_member(&fixture.home, member, edit);
        fixture
    }
}

/// Asserts the changed manifest is an error because Darkmatter refuses the
/// bundle (an `error` row is what makes `claudine doctor` exit 1), and the
/// member's digest is not alleged to disagree.
#[track_caller]
fn assert_refused_with_correct_digests(fixture: &Fixture, member: &str) -> DoctorRow {
    let row = fixture.check();
    assert_row(&row, Severity::Error, true, "Darkmatter refuses the bundle it lists as Claudine's registration");
    assert!(!row.detail.contains("do not match the manifest's digest"), "{member}: {row:#?}");
    assert!(!row.detail.contains("Darkmatter still loads"), "{member}: {row:#?}");
    row
}

#[rstest::rstest]
#[case::schema_library("schemas/action.yaml")]
#[case::schema_trigger("schemas/composition.yaml")]
#[case::standalone_yaml_schema("schemas/formal-sequence-file.yaml")]
#[case::namespace_schema("schemas/group.yaml")]
fn an_invalid_member_with_an_updated_digest_is_an_error(#[case] member: &str) {
    let fixture = Fixture::with_edited_member(member, |text| format!("{text}{INVALID_YAML}"));
    assert_refused_with_correct_digests(&fixture, member);
}

#[test]
fn a_namespace_comment_with_an_updated_digest_is_an_error_since_the_lifecycle_artifact_disagrees() {
    let fixture = Fixture::with_edited_member("schemas/group.yaml", |text| format!("{text}{COMMENT}"));
    let row = assert_refused_with_correct_digests(&fixture, "schemas/group.yaml");
    let artifact = fixture.root().join("globals.lifecycle.json");
    assert!(row.detail.contains(&format!("Darkmatter's first refusal is in {}", artifact.display())), "{row:#?}");
}

#[test]
fn an_emptied_lifecycle_artifact_with_an_updated_digest_is_an_error() {
    let fixture = Fixture::with_edited_member("globals.lifecycle.json", |_| "{}".to_string());
    assert_refused_with_correct_digests(&fixture, "globals.lifecycle.json");
}

/// The comment-only acceptance control for the cases above: a consistent
/// bundle that still loads is a deviation from the shipped bytes, not a
/// refusal.
#[rstest::rstest]
#[case::schema_library("schemas/action.yaml", COMMENT)]
#[case::schema_trigger("schemas/composition.yaml", COMMENT)]
#[case::standalone_yaml_schema("schemas/formal-sequence-file.yaml", COMMENT)]
#[case::lifecycle_artifact_whitespace("globals.lifecycle.json", "\n\n")]
fn a_loadable_byte_edit_with_an_updated_digest_warns_without_alleging_a_digest_mismatch(
    #[case] member: &str,
    #[case] appended: &str,
) {
    let fixture = Fixture::with_edited_member(member, |text| format!("{text}{appended}"));

    let row = fixture.check();

    let path = fixture.root().join(native_path(member));
    assert_row(&row, Severity::Warn, true, &format!("{}: differs from the file this Claudine publishes", path.display()));
    assert!(row.detail.contains("Darkmatter still loads the bundle it lists"), "{row:#?}");
    assert!(!row.detail.contains("digest") && !row.detail.contains("refuses"), "{row:#?}");
}

// ------------------------------------------------- member reference spellings

impl Fixture {
    /// Rewrites the published manifest's entry and digest key for `member`
    /// from its canonical `./{member}` to `spelling`, keeping every byte and
    /// digest value.
    fn respell_member(&self, member: &str, spelling: &str) {
        let manifest = self.root().join("manifest.yaml");
        let text = std::fs::read_to_string(&manifest).unwrap();
        let canonical = format!("\"./{member}\"");
        assert_eq!(text.matches(&canonical).count(), 2, "an entry and a digest key: {text}");
        std::fs::write(&manifest, text.replace(&canonical, &format!("\"{spelling}\""))).unwrap();
    }
}

/// A spelling Darkmatter's installed reader resolves to the same in-bundle
/// member is a loadable manifest edit (a warning), whatever the member's
/// category; one that ends outside the bundle is refused (an error), as
/// Darkmatter refuses it. Each test builds its own home.
macro_rules! member_spelling_cases {
    ($($category:ident => $member:literal,)*) => {$(
        mod $category {
            use super::*;

            #[test]
            fn the_canonical_spelling_beside_a_manifest_comment_warns() {
                let fixture = Fixture::set_up();
                let manifest = fixture.root().join("manifest.yaml");
                let published = std::fs::read_to_string(&manifest).unwrap();
                assert!(published.contains(&format!("\"./{}\"", $member)), "{published}");
                std::fs::write(&manifest, format!("{published}{COMMENT}")).unwrap();
                assert_row(&fixture.check(), Severity::Warn, true, "Darkmatter still loads the bundle it lists");
            }

            #[test]
            fn a_spelling_that_reenters_the_bundle_warns() {
                let fixture = Fixture::set_up();
                fixture.respell_member($member, &format!("../darkmatter/{}", $member));
                let row = fixture.check();
                assert_row(&row, Severity::Warn, true, "Darkmatter still loads the bundle it lists");
                assert!(!row.detail.contains("digest") && !row.detail.contains("refuses"), "{row:#?}");
            }

            #[test]
            fn a_spelling_that_leaves_the_bundle_is_an_error() {
                let fixture = Fixture::set_up();
                fixture.respell_member($member, &format!("../{}", $member));
                assert_row(&fixture.check(), Severity::Error, true, "Darkmatter refuses the bundle it lists");
            }
        }
    )*};
}

member_spelling_cases! {
    schema_library_spelling => "schemas/action.yaml",
    schema_trigger_spelling => "schemas/composition.yaml",
    standalone_yaml_schema_spelling => "schemas/formal-sequence-file.yaml",
    namespace_schema_spelling => "schemas/group.yaml",
    lifecycle_artifact_spelling => "globals.lifecycle.json",
}

/// A member edited without updating its digest is attributed the mismatch
/// whether the manifest spells it canonically or by re-entering the bundle.
#[rstest::rstest]
#[case::canonical(None)]
#[case::reentry(Some("../darkmatter/schemas/action.yaml"))]
fn a_member_that_disagrees_with_its_digest_is_attributed_under_either_spelling(#[case] spelling: Option<&str>) {
    let fixture = Fixture::set_up();
    if let Some(spelling) = spelling {
        fixture.respell_member("schemas/action.yaml", spelling);
    }
    let member = fixture.root().join(native_path("schemas/action.yaml"));
    let published = std::fs::read_to_string(&member).unwrap();
    std::fs::write(&member, format!("{published}{COMMENT}")).unwrap();

    let row = fixture.check();

    let text = format!("{}\n{}", row.summary, row.detail);
    assert!(text.contains(&format!("{}: differs from the file this Claudine publishes", member.display())), "{row:#?}");
    assert!(text.contains("its bytes do not match the manifest's digest"), "{row:#?}");
}

#[test]
fn manifest_edits_are_judged_by_whether_the_bundle_loads() {
    let fixture = Fixture::set_up();
    let manifest = fixture.root().join("manifest.yaml");
    let published = std::fs::read_to_string(&manifest).unwrap();

    std::fs::write(&manifest, format!("{published}{COMMENT}")).unwrap();
    assert_row(&fixture.check(), Severity::Warn, true, "Darkmatter still loads the bundle it lists");

    assert!(published.contains("host: claudine\n"), "{published}");
    std::fs::write(&manifest, published.replace("host: claudine\n", "host: other\n")).unwrap();
    assert_row(&fixture.check(), Severity::Error, true, "Darkmatter refuses the bundle it lists");
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

/// Asserts the first `--fix` failed at the retired tree's root and listed
/// the folder it removed from it, and that a second `--fix`, with the root
/// removable again, removes and reports only what was left.
#[track_caller]
fn assert_partial_cleanup_then_retry(fixture: &Fixture, first: &DoctorRow) {
    let revisions = fixture.home.join(native_path(".claudine/darkmatter-host"));
    let manifest = fixture.home.join(native_path(".claudine/darkmatter-host.yaml"));
    let inner = revisions.join("r1");
    assert_eq!(first.severity, Severity::Error, "{first:#?}");
    assert!(first.detail.contains(&format!("Repair failed: could not remove {}", revisions.display())), "{first:#?}");
    assert!(first.detail.contains(&format!("Already changed: removed {}", inner.display())), "{first:#?}");
    assert!(!first.detail.contains("No change was made"), "{first:#?}");
    assert!(!inner.exists() && revisions.is_dir() && manifest.is_file());

    let again = fixture.fix();

    assert_repaired(fixture, &again);
    let repaired: Vec<&str> = again.detail.lines().filter(|line| line.starts_with("Repaired:")).collect();
    assert_eq!(
        repaired,
        [format!("Repaired: removed {}", revisions.display()), format!("Repaired: removed {}", manifest.display())],
        "{again:#?}"
    );
}

#[test]
fn a_failed_retired_tree_removal_reports_the_removed_entries_and_a_retry_only_the_rest() {
    let fixture = Fixture::set_up();
    fixture.config("register:\n    - ~/.claudine/darkmatter-host.yaml\n");
    fixture.legacy();
    let revisions = fixture.home.join(native_path(".claudine/darkmatter-host"));

    let first = {
        let _fail = crate::composition::registration::fail_removals([revisions]);
        fixture.fix()
    };

    assert_partial_cleanup_then_retry(&fixture, &first);
}

/// The review's shape: a writable retired tree below a read-only
/// `.claudine`, so its entries can go but its root cannot.
#[cfg(unix)]
#[test]
fn a_read_only_claudine_folder_keeps_the_retired_root_and_the_report_lists_what_went() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::set_up();
    fixture.config("register:\n    - ~/.claudine/darkmatter-host.yaml\n");
    fixture.legacy();
    let claudine = fixture.home.join(".claudine");
    let set_mode = |mode| std::fs::set_permissions(&claudine, std::fs::Permissions::from_mode(mode)).unwrap();
    set_mode(0o555);
    // A superuser ignores the mode; the portable case above covers it.
    if std::fs::write(claudine.join("probe"), "").is_ok() {
        set_mode(0o755);
        eprintln!("skipped: the folder mode does not stop this user");
        return;
    }

    let first = fixture.fix();

    set_mode(0o755);
    assert_partial_cleanup_then_retry(&fixture, &first);
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

/// The registration repair judges the `register` setting it edits as the
/// permissions handoff does: an omitted, `null`, or empty list is edited,
/// a mis-typed one blocks before anything is written.
#[test]
fn only_a_valid_register_setting_admits_the_entry_repair() {
    for (text, repairable) in [
        ("permissions:\n  network: deny\n", true),
        ("register: null\n", true),
        ("register: []\n", true),
        ("register: 3\n", false),
        ("register:\n    - 3\n", false),
    ] {
        let fixture = Fixture::empty();
        fixture.publish(RegistrationInventory::claudine());
        fixture.config(text);
        let before = fixture.snapshot();

        let row = fixture.check();
        let fixed = fixture.fix();

        assert_row(&row, Severity::Error, repairable, "no `register` entry names Claudine's manifest");
        if repairable {
            assert_repaired(&fixture, &fixed);
            assert!(fixture.config_text().contains(ENTRY), "{text:?}: {}", fixture.config_text());
        } else {
            assert_eq!(fixed, row, "{text:?}: no repair is attempted");
            assert_eq!(fixture.snapshot(), before, "{text:?}");
        }
    }
}

#[test]
fn a_current_tree_with_a_missing_user_file_reports_the_entry_as_blocked() {
    let fixture = Fixture::empty();
    fixture.publish(RegistrationInventory::claudine());
    assert_row(&fixture.check(), Severity::Info, false, "the `register` entry cannot be verified: the file does not exist");
}

// ------------------------------------------------- the editor's layout (R4-F2)

const LAYOUT_REMEDY: &str = "in block form, one `key: value` or `- item` per line";

/// R4-F2.1/.2: when the entry edit the repair needs reaches an inline list or
/// an inline root, the whole repair is manual: the row names the file and
/// the setting, offers no fix, and `--fix` publishes and removes nothing.
#[rstest::rstest]
#[case::inline_list_append("register: [~/.acme/manifest.yaml]\n", false, "the `register` setting of `~/.darkmatter.yaml`")]
#[case::inline_list_replace(
    "register: [~/.acme/manifest.yaml, ~/.claudine/darkmatter-host.yaml]\n",
    true,
    "the `register` setting of `~/.darkmatter.yaml`"
)]
#[case::inline_list_remove(
    "register: [~/.claudine/registrations/darkmatter/manifest.yaml, ~/.claudine/darkmatter-host.yaml]\n",
    true,
    "the `register` setting of `~/.darkmatter.yaml`"
)]
#[case::inline_root("{register: [], permissions: {network: deny}}\n", false, "the root mapping of `~/.darkmatter.yaml`")]
fn an_entry_edit_the_editor_cannot_make_leaves_the_registration_manual(
    #[case] text: &str,
    #[case] legacy: bool,
    #[case] names: &str,
) {
    let fixture = Fixture::empty();
    fixture.config(text);
    if legacy {
        fixture.legacy();
    }
    let before = fixture.snapshot();

    let row = fixture.check();
    let fixed = fixture.fix();

    assert_row(&row, Severity::Error, false, "the published registration is missing");
    assert!(row.detail.contains(&format!("{names} is not written in block form")), "{row:#?}");
    assert!(row.detail.contains(LAYOUT_REMEDY), "{row:#?}");
    assert!(!row.detail.contains("~/.acme/manifest.yaml"), "no authored value is quoted: {row:#?}");
    assert_eq!(fixed, row, "no repair is attempted");
    assert_eq!(fixture.snapshot(), before);
}

/// The block-form controls of the same three edits and of the root repair,
/// keeping the other host's entry.
#[rstest::rstest]
#[case::block_list_append("register:\n    - ~/.acme/manifest.yaml\n", false)]
#[case::block_list_replace("register:\n    - ~/.acme/manifest.yaml\n    - ~/.claudine/darkmatter-host.yaml\n", true)]
#[case::block_list_remove(
    "register:\n    - ~/.acme/manifest.yaml\n    - ~/.claudine/registrations/darkmatter/manifest.yaml\n    - ~/.claudine/darkmatter-host.yaml\n",
    true
)]
#[case::block_root("register: []\npermissions:\n    network: deny\n", false)]
fn the_same_entry_edit_in_block_form_is_repaired(#[case] text: &str, #[case] legacy: bool) {
    let fixture = Fixture::empty();
    fixture.config(text);
    if legacy {
        fixture.legacy();
    }

    assert_row(&fixture.check(), Severity::Error, true, "the published registration is missing");
    assert_repaired(&fixture, &fixture.fix());

    let config = fixture.config_text();
    assert_eq!(config.matches(ENTRY).count(), 1, "{config}");
    assert_eq!(config.contains("~/.acme/manifest.yaml"), text.contains("~/.acme/"), "{config}");
    assert!(!config.contains("darkmatter-host.yaml"), "{config}");
    assert!(!fixture.home.join(native_path(".claudine/darkmatter-host.yaml")).exists());
}

/// R4-F2.c1: an inline list that already names the manifest needs no entry
/// edit, so the layout is no obstacle: the retired layout is removed and the
/// list is left byte for byte.
#[test]
fn an_inline_list_that_needs_no_edit_is_kept_byte_for_byte_while_the_retired_layout_goes() {
    let fixture = Fixture::empty();
    fixture.publish(RegistrationInventory::claudine());
    let text = format!("register: [~/.acme/manifest.yaml, {ENTRY}]\n");
    fixture.config(&text);
    fixture.legacy();

    let row = fixture.check();
    assert_eq!((row.severity, row.fixable), (Severity::Warn, true), "{row:#?}");
    assert_repaired(&fixture, &fixture.fix());

    assert_eq!(fixture.config_text(), text);
    assert!(!fixture.home.join(native_path(".claudine/darkmatter-host.yaml")).exists());
}

/// With the tree already published, the entry finding itself carries the
/// layout remedy.
#[test]
fn a_missing_entry_in_an_inline_list_carries_the_block_form_remedy() {
    let fixture = Fixture::empty();
    fixture.publish(RegistrationInventory::claudine());
    let text = "register: [~/.acme/manifest.yaml]\n";
    fixture.config(text);

    let row = fixture.check();

    assert_row(&row, Severity::Error, false, "no `register` entry names Claudine's manifest");
    assert!(row.detail.contains("rewrite the `register` list of `~/.darkmatter.yaml` in block form"), "{row:#?}");
    assert_eq!(fixture.fix(), row);
    assert_eq!(fixture.config_text(), text);
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

// ---------------------------------------------------------------- the user file seen from a repository

/// Where doctor runs relative to the user file.
#[derive(Debug, Clone, Copy)]
enum Launch {
    /// A separate repository with no `.darkmatter.yaml` (control).
    SeparateRepository,
    /// A separate repository whose `.darkmatter.yaml` links to the user file.
    #[cfg(unix)]
    LinkedRepositoryFile,
    /// Inside the home, which is itself the repository root.
    HomeIsRepository,
}

fn launches() -> Vec<Launch> {
    let mut launches = vec![Launch::SeparateRepository, Launch::HomeIsRepository];
    #[cfg(unix)]
    launches.push(Launch::LinkedRepositoryFile);
    launches
}

/// A user-only grant (a home reference no repository file may hold) and a
/// policy both scopes accept.
const USER_ONLY_POLICY: &str = "permissions:\n    file-read:\n        - ~/notes/*.md\n";
const SHARED_POLICY: &str = "permissions:\n    network: deny\n";

impl Fixture {
    /// An environment launched in the repository `launch` lays out.
    fn environment_in(&self, launch: Launch) -> Environment {
        let root = match launch {
            Launch::SeparateRepository => self.separate_repository(),
            #[cfg(unix)]
            Launch::LinkedRepositoryFile => {
                let root = self.separate_repository();
                std::os::unix::fs::symlink(self.home.join(".darkmatter.yaml"), root.join(".darkmatter.yaml")).unwrap();
                root
            }
            Launch::HomeIsRepository => self.home.clone(),
        };
        let context = FileResolutionContext::from_snapshot(root.clone(), Some(self.home.clone()), HashMap::new())
            .with_repository_root(&root);
        Environment::for_test(Some(self.home.clone()), Ok(context))
    }

    fn separate_repository(&self) -> PathBuf {
        let root = self.home.parent().unwrap().join("repo");
        std::fs::create_dir_all(&root).unwrap();
        root
    }
}

#[test]
fn the_entry_repair_edits_the_user_file_wherever_doctor_runs() {
    for launch in launches() {
        for policy in [USER_ONLY_POLICY, SHARED_POLICY] {
            let case = format!("{launch:?}, {policy:?}");
            let fixture = Fixture::set_up();
            let before = format!("register:\n    # the acme host\n    - ~/.acme/manifest.yaml\n{policy}");
            fixture.config(&before);
            // A different inline policy, so the permissions handoff of the
            // full registry leaves both files alone.
            std::fs::write(
                fixture.home.join(native_path(".claudine/config.yaml")),
                "permissions:\n    file-read:\n        - ~/elsewhere/*.txt\n",
            )
            .unwrap();
            let environment = fixture.environment_in(launch);

            let fixed = row(run_with(file_checks(), &environment, true));
            assert_eq!(fixed.severity, Severity::Ok, "{case}: {fixed:#?}");
            assert!(fixed.summary.contains("Repaired:") || fixed.detail.contains("Repaired:"), "{case}: {fixed:#?}");
            let after = row(run_with(file_checks(), &environment, false));
            assert_eq!(after.severity, Severity::Ok, "{case}: the entry verifies: {after:#?}");

            let expected = format!("register:\n    # the acme host\n    - ~/.acme/manifest.yaml\n    - {ENTRY}\n{policy}");
            assert_eq!(fixture.config_text(), expected, "{case}");
        }
    }
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

    /// The published tree and the retired layout both sit below `.claudine`:
    /// the same bytes reached through a linked `.claudine` are refused.
    #[test]
    fn a_linked_claudine_folder_holding_the_tree_and_the_retired_layout() {
        let fixture = Fixture::set_up();
        fixture.legacy();
        fixture.config("register: []\n");
        let shared = outside(&fixture).join("claudine");
        let link = fixture.home.join(".claudine");
        std::fs::rename(&link, &shared).unwrap();
        std::os::unix::fs::symlink(&shared, &link).unwrap();
        assert_manual(&fixture, &link, &shared);
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
