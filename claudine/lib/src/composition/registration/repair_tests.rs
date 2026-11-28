//! The repair over real temporary homes: its ordering, its refusals, its
//! failure points, and that it converges and then changes nothing.

use std::collections::HashMap;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};

use super::super::{
    CleanupError, FileKind, MANIFEST_REFERENCE, PublishError, PublishStep, PublishedFile, RegistrationInventory, TEMP_PREFIX,
    fail_removals, inspect_installation, legacy_manifest_path, legacy_revisions_dir, output_root, publish,
    remove_legacy_registration,
};
use super::{
    EditFailure, RegistrationRepair, RepairFailure, RepairHooks, RepairOperation, RepairStep, repair_registration,
    repair_registration_with,
};

const LEGACY_ENTRY: &str = "register:\n    - ~/.claudine/darkmatter-host.yaml\n";

struct Home {
    _dir: tempfile::TempDir,
    home: PathBuf,
    context: FileResolutionContext,
}

impl Home {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let home = canonicalize_simplified(dir.path()).unwrap().join("home");
        std::fs::create_dir_all(home.join(".claudine")).unwrap();
        let context = FileResolutionContext::from_snapshot(home.clone(), Some(home.clone()), HashMap::new());
        Self { _dir: dir, home, context }
    }

    fn config(&self) -> PathBuf {
        self.home.join(".darkmatter.yaml")
    }

    fn write_config(&self, text: &str) {
        std::fs::write(self.config(), text).unwrap();
    }

    fn config_text(&self) -> String {
        std::fs::read_to_string(self.config()).unwrap()
    }

    /// The retired manifest and a revision tree two folders deep.
    fn legacy(&self) {
        std::fs::write(legacy_manifest_path(&self.home), "kind: darkmatter-host\n").unwrap();
        let revision = legacy_revisions_dir(&self.home).join("r1/schemas");
        std::fs::create_dir_all(&revision).unwrap();
        std::fs::write(revision.join("a.yaml"), "kind: schema\n").unwrap();
    }

    fn legacy_present(&self) -> bool {
        legacy_manifest_path(&self.home).exists() && legacy_revisions_dir(&self.home).exists()
    }

    fn repair(&self) -> RegistrationRepair {
        repair_registration(&self.context, RegistrationInventory::claudine())
    }

    fn current(&self) -> bool {
        inspect_installation(&self.home, Some(&self.config()), RegistrationInventory::claudine()).is_current()
    }

    /// Every entry under the temporary root, read without following links:
    /// its path, bytes (files), and modification time.
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

fn index_of(repair: &RegistrationRepair, wanted: &RepairOperation) -> usize {
    repair.operations.iter().position(|operation| operation == wanted).unwrap_or_else(|| panic!("{wanted:?} not in {repair:#?}"))
}

#[test]
fn a_repair_publishes_registers_and_only_then_removes_the_retired_layout() {
    let home = Home::new();
    home.write_config(&format!("# mine\n{LEGACY_ENTRY}"));
    home.legacy();

    let repair = home.repair();

    assert!(repair.failure.is_none(), "{repair:#?}");
    let edited = index_of(&repair, &RepairOperation::Edited(home.config()));
    let manifest = index_of(&repair, &RepairOperation::Replaced(output_root(&home.home).join("manifest.yaml")));
    let revisions = index_of(&repair, &RepairOperation::Removed(legacy_revisions_dir(&home.home)));
    let old = index_of(&repair, &RepairOperation::Removed(legacy_manifest_path(&home.home)));
    assert!(manifest < edited && edited < revisions && edited < old, "{repair:#?}");
    assert_eq!(home.config_text(), format!("# mine\nregister:\n    - {MANIFEST_REFERENCE}\n"));
    assert!(!legacy_manifest_path(&home.home).exists() && !legacy_revisions_dir(&home.home).exists());
    assert!(home.current());
}

#[test]
fn a_second_repair_changes_no_byte_and_no_modification_time() {
    let home = Home::new();
    home.write_config(LEGACY_ENTRY);
    home.legacy();
    assert!(home.repair().failure.is_none());
    let before = home.snapshot();

    let again = home.repair();

    assert!(again.failure.is_none() && again.operations.is_empty(), "{again:#?}");
    assert_eq!(home.snapshot(), before);
}

#[test]
fn extras_are_removed_children_first_and_expected_folders_are_kept() {
    let home = Home::new();
    publish(&home.home, RegistrationInventory::claudine()).unwrap();
    home.write_config(&format!("register:\n    - {MANIFEST_REFERENCE}\n"));
    let root = output_root(&home.home);
    let deep = root.join("schemas/old/deep");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("x.yaml"), "x").unwrap();
    std::fs::write(root.join("notes.txt"), "mine").unwrap();
    std::fs::create_dir(root.join("empty")).unwrap();
    // A plain publish keeps what is not its own.
    publish(&home.home, RegistrationInventory::claudine()).unwrap();
    assert!(deep.join("x.yaml").exists() && root.join("notes.txt").exists());

    let repair = home.repair();

    assert!(repair.failure.is_none(), "{repair:#?}");
    let file = index_of(&repair, &RepairOperation::Removed(deep.join("x.yaml")));
    let folder = index_of(&repair, &RepairOperation::Removed(deep.clone()));
    let parent = index_of(&repair, &RepairOperation::Removed(root.join("schemas/old")));
    assert!(file < folder && folder < parent, "{repair:#?}");
    for removed in [root.join("notes.txt"), root.join("empty")] {
        index_of(&repair, &RepairOperation::Removed(removed));
    }
    assert!(root.join("schemas").is_dir(), "an expected folder is kept");
    assert!(!repair.operations.iter().any(|operation| matches!(operation, RepairOperation::Edited(_))));
    assert!(home.current());
}

#[test]
fn a_failed_publication_withholds_the_register_edit_and_keeps_the_retired_layout() {
    let home = Home::new();
    home.write_config(LEGACY_ENTRY);
    home.legacy();
    let mut observer = |step: &RepairStep| match step {
        RepairStep::Publish(PublishStep::MemberWritten(_)) => ControlFlow::Break(()),
        _ => ControlFlow::Continue(()),
    };

    let repair = repair_registration_with(
        &home.context,
        RegistrationInventory::claudine(),
        RepairHooks { lock_deadline: Duration::from_secs(5), observer: &mut observer },
    );

    assert!(matches!(repair.failure, Some(RepairFailure::Interrupted { .. })), "{repair:#?}");
    assert_eq!(repair.operations.len(), 1, "the one member written is reported: {repair:#?}");
    assert!(matches!(repair.operations[0], RepairOperation::Replaced(_)));
    assert_eq!(home.config_text(), LEGACY_ENTRY);
    assert!(home.legacy_present());

    let retry = home.repair();
    assert!(retry.failure.is_none(), "{retry:#?}");
    assert!(!home.legacy_present() && home.current());
}

#[test]
fn a_failed_register_write_keeps_the_published_tree_and_the_retired_layout_and_a_retry_converges() {
    let home = Home::new();
    publish(&home.home, RegistrationInventory::claudine()).unwrap();
    home.write_config(LEGACY_ENTRY);
    home.legacy();

    let repair = {
        let _fail = darkmatter::config::write::test_hooks::fail_next_replacement();
        home.repair()
    };

    assert!(
        matches!(repair.failure, Some(RepairFailure::RegisterEdit { ref path, cause: EditFailure::Io(_) }) if *path == home.config()),
        "{repair:#?}"
    );
    assert_eq!(home.config_text(), LEGACY_ENTRY);
    assert!(home.legacy_present() && home.current());

    let retry = home.repair();
    assert!(retry.failure.is_none(), "{retry:#?}");
    assert!(home.config_text().contains(MANIFEST_REFERENCE));
    assert!(!home.legacy_present());
}

#[test]
fn the_retired_layout_is_removed_only_after_the_entry_verifies() {
    let home = Home::new();
    home.write_config(LEGACY_ENTRY);
    home.legacy();
    let mut seen_entry = None;
    let config = home.config();
    let mut observer = |step: &RepairStep| {
        if *step == RepairStep::BeforeLegacyCleanup {
            seen_entry = Some(std::fs::read_to_string(&config).unwrap());
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    };

    let repair = repair_registration_with(
        &home.context,
        RegistrationInventory::claudine(),
        RepairHooks { lock_deadline: Duration::from_secs(5), observer: &mut observer },
    );

    assert!(matches!(repair.failure, Some(RepairFailure::Interrupted { .. })), "{repair:#?}");
    assert!(seen_entry.unwrap().contains(MANIFEST_REFERENCE));
    assert!(home.legacy_present());
}

#[test]
fn a_missing_or_invalid_user_file_refuses_before_anything_is_written() {
    for (text, expected) in [
        (None, super::EntryPrecondition::Missing),
        (Some("register: 5\n"), super::EntryPrecondition::RegisterInvalid),
        (Some("register: [\n"), super::EntryPrecondition::Invalid),
    ] {
        let home = Home::new();
        if let Some(text) = text {
            home.write_config(text);
        }
        home.legacy();
        let before = home.snapshot();

        let repair = home.repair();

        assert!(matches!(repair.failure, Some(RepairFailure::Precondition(found)) if found == expected), "{text:?}: {repair:#?}");
        assert!(repair.operations.is_empty());
        assert_eq!(home.snapshot(), before, "{text:?}");
    }
}

/// R4-F2.1/.2: an entry edit the format-preserving editor cannot make in
/// place stops the repair before the publisher runs, so nothing is published
/// and the retired layout stays.
#[rstest::rstest]
#[case::inline_list_append("register: [~/.acme/manifest.yaml]\n", crate::composition::registration::EditLayout::Setting("register"))]
#[case::inline_list_replace(
    "register: [~/.acme/manifest.yaml, ~/.claudine/darkmatter-host.yaml]\n",
    crate::composition::registration::EditLayout::Setting("register")
)]
#[case::inline_list_remove(
    "register: [~/.claudine/registrations/darkmatter/manifest.yaml, ~/.claudine/darkmatter-host.yaml]\n",
    crate::composition::registration::EditLayout::Setting("register")
)]
#[case::inline_root("{register: [], permissions: {network: deny}}\n", crate::composition::registration::EditLayout::Root)]
fn an_entry_edit_the_editor_refuses_stops_the_repair_before_publication(
    #[case] text: &str,
    #[case] layout: crate::composition::registration::EditLayout,
) {
    let home = Home::new();
    home.write_config(text);
    home.legacy();
    let before = home.snapshot();

    let repair = home.repair();

    let expected = super::EntryPrecondition::Layout(layout);
    assert!(matches!(repair.failure, Some(RepairFailure::Precondition(found)) if found == expected), "{repair:#?}");
    assert!(repair.operations.is_empty(), "{repair:#?}");
    assert_eq!(home.snapshot(), before);
}

/// The writer's own check stays: a file rewritten inline after the repair's
/// checks (here, once the tree is published) is refused by the transaction,
/// the file is kept, and so is the retired layout.
#[test]
fn a_list_rewritten_inline_after_the_checks_is_still_refused_by_the_writer() {
    let home = Home::new();
    home.write_config(LEGACY_ENTRY);
    home.legacy();
    let config = home.config();
    let inline = "register: [~/.acme/manifest.yaml, ~/.claudine/darkmatter-host.yaml]\n";
    let mut observer = |step: &RepairStep| {
        if *step == RepairStep::BeforeRegister {
            std::fs::write(&config, inline).unwrap();
        }
        ControlFlow::Continue(())
    };

    let repair = repair_registration_with(
        &home.context,
        RegistrationInventory::claudine(),
        RepairHooks { lock_deadline: Duration::from_secs(5), observer: &mut observer },
    );

    assert!(
        matches!(repair.failure, Some(RepairFailure::RegisterEdit { cause: EditFailure::Invalid, .. })),
        "{repair:#?}"
    );
    assert_eq!(home.config_text(), inline);
    assert!(home.legacy_present() && home.current());
}

#[cfg(unix)]
#[test]
fn a_link_at_the_writers_lock_sidecar_refuses_before_anything_is_written() {
    let home = Home::new();
    home.write_config("register: []\n");
    let outside = home.home.parent().unwrap().join("outside");
    std::fs::write(&outside, "not yours").unwrap();
    std::os::unix::fs::symlink(&outside, home.home.join(".darkmatter.yaml.darkmatter-lock")).unwrap();
    let before = home.snapshot();

    let repair = home.repair();

    assert!(
        matches!(&repair.failure, Some(RepairFailure::Blocked(blocker)) if blocker.path.ends_with(".darkmatter.yaml.darkmatter-lock")),
        "{repair:#?}"
    );
    assert_eq!(home.snapshot(), before);
}

#[test]
fn concurrent_repairs_of_one_home_serialize_and_converge() {
    let home = Home::new();
    home.write_config(LEGACY_ENTRY);
    home.legacy();

    let repairs: Vec<RegistrationRepair> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..2).map(|_| scope.spawn(|| home.repair())).collect();
        workers.into_iter().map(|worker| worker.join().unwrap()).collect()
    });

    for repair in &repairs {
        assert!(repair.failure.is_none(), "{repair:#?}");
    }
    assert_eq!(home.config_text(), format!("register:\n    - {MANIFEST_REFERENCE}\n"));
    assert!(!home.legacy_present() && home.current());
}

// ------------------------------------------- failures after earlier changes

/// The removals of `repair`, in order.
fn removed(repair: &RegistrationRepair) -> Vec<PathBuf> {
    repair
        .operations
        .iter()
        .filter_map(|operation| match operation {
            RepairOperation::Removed(path) => Some(path.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_failed_retired_tree_root_removal_reports_what_went_and_a_retry_removes_only_the_rest() {
    let home = Home::new();
    home.write_config(LEGACY_ENTRY);
    home.legacy();
    let revisions = legacy_revisions_dir(&home.home);

    let repair = {
        let _fail = fail_removals([revisions.clone()]);
        home.repair()
    };

    assert!(
        matches!(&repair.failure, Some(RepairFailure::Legacy(CleanupError::Io { path, .. })) if *path == revisions),
        "{repair:#?}"
    );
    // The revision folder and everything in it went before the root failed.
    assert!(removed(&repair).contains(&revisions.join("r1")), "{repair:#?}");
    assert!(!revisions.join("r1").exists() && revisions.is_dir() && legacy_manifest_path(&home.home).is_file());

    let retry = home.repair();

    assert!(retry.failure.is_none(), "{retry:#?}");
    assert_eq!(retry.operations, [RepairOperation::Removed(revisions), RepairOperation::Removed(legacy_manifest_path(&home.home))]);
    assert!(!home.legacy_present() && home.current());
}

#[test]
fn a_failed_retired_manifest_removal_still_reports_the_removed_tree() {
    let home = Home::new();
    home.write_config(LEGACY_ENTRY);
    home.legacy();
    let manifest = legacy_manifest_path(&home.home);

    let repair = {
        let _fail = fail_removals([manifest.clone()]);
        home.repair()
    };

    assert!(
        matches!(&repair.failure, Some(RepairFailure::Legacy(CleanupError::Io { path, .. })) if *path == manifest),
        "{repair:#?}"
    );
    let revisions = legacy_revisions_dir(&home.home);
    assert_eq!(removed(&repair).last(), Some(&revisions), "{repair:#?}");
    assert!(!revisions.exists() && manifest.is_file());

    let retry = home.repair();

    assert!(retry.failure.is_none(), "{retry:#?}");
    assert_eq!(retry.operations, [RepairOperation::Removed(manifest)]);
}

#[test]
fn a_failed_temporary_file_removal_keeps_the_earlier_ones_in_the_journal() {
    let home = Home::new();
    publish(&home.home, RegistrationInventory::claudine()).unwrap();
    home.write_config(&format!("register:\n    - {MANIFEST_REFERENCE}\n"));
    let root = output_root(&home.home);
    for name in ["a", "b", "c"] {
        std::fs::write(root.join(format!("{TEMP_PREFIX}{name}")), "partial").unwrap();
    }
    // The cleanup lists the folder in this order too: nothing changes it
    // between the two listings.
    let listed: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.file_name().unwrap().to_string_lossy().starts_with(TEMP_PREFIX))
        .collect();
    let failing = listed[2].clone();

    let repair = {
        let _fail = fail_removals([failing.clone()]);
        home.repair()
    };

    assert!(
        matches!(&repair.failure, Some(RepairFailure::Publish(PublishError::Io { path, .. })) if *path == failing),
        "{repair:#?}"
    );
    assert_eq!(removed(&repair), listed[..2], "{repair:#?}");
    assert!(listed[..2].iter().all(|path| !path.exists()) && failing.is_file());

    let retry = home.repair();

    assert!(retry.failure.is_none(), "{retry:#?}");
    assert_eq!(retry.operations, [RepairOperation::Removed(failing)]);
}

#[test]
fn folders_emptied_by_retirement_are_reported_when_a_later_step_fails() {
    let home = Home::new();
    let retired = "schemas/old/deep/retired.yaml";
    let previous = RegistrationInventory::claudine().clone().with_member(PublishedFile::new(
        retired,
        FileKind::Library,
        None::<&'static str>,
        b"kind: schema\ntypes:\n    retired: string\n".to_vec(),
    ));
    publish(&home.home, &previous).unwrap();
    home.write_config(LEGACY_ENTRY);
    home.legacy();
    let root = output_root(&home.home);
    let revisions = legacy_revisions_dir(&home.home);

    let repair = {
        let _fail = fail_removals([revisions.clone()]);
        home.repair()
    };

    assert!(matches!(repair.failure, Some(RepairFailure::Legacy(_))), "{repair:#?}");
    let removed = removed(&repair);
    let file = removed.iter().position(|path| *path == root.join("schemas/old/deep/retired.yaml"));
    let deep = removed.iter().position(|path| *path == root.join("schemas/old/deep"));
    let old = removed.iter().position(|path| *path == root.join("schemas/old"));
    assert!(file.is_some() && file < deep && deep < old, "{repair:#?}");
    assert!(!root.join("schemas/old").exists());
}

// ── case-only aliases ───────────────────────────────────────────────────────
//
// Each test asserts the case-insensitive outcome where the temporary volume
// resolves another case of a name to the same entry (APFS and NTFS by
// default) and the case-sensitive one elsewhere (ext4), so one test proves
// the fix on macOS and Windows and the distinct-name control on Linux.

/// Whether `dir`'s volume resolves a name spelled in another case to the
/// entry it holds.
fn case_insensitive(dir: &Path) -> bool {
    let probe = dir.join("case-probe");
    std::fs::write(&probe, "").unwrap();
    let insensitive = std::fs::symlink_metadata(dir.join("CASE-PROBE")).is_ok();
    std::fs::remove_file(&probe).unwrap();
    // Names the branch a run proved; shown with `--success-output`.
    eprintln!("case-insensitive volume at {}: {insensitive}", dir.display());
    insensitive
}

/// The names `dir` lists, as stored.
fn listed(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        std::fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

/// The inventory with every member path respelled by `respell` (same
/// bytes), plus `extra` members.
fn respelled(respell: impl Fn(&str) -> String, extra: impl IntoIterator<Item = PublishedFile>) -> RegistrationInventory {
    let inventory = RegistrationInventory::claudine();
    let members = inventory
        .members()
        .map(|file| PublishedFile::new(respell(file.path()), file.kind(), file.name().map(str::to_string), file.bytes().to_vec()));
    RegistrationInventory::from_members(inventory.version(), members.chain(extra))
}

fn action_as_upper(path: &str) -> String {
    if path == "schemas/action.yaml" { "schemas/Action.yaml".to_string() } else { path.to_string() }
}

fn schemas_as_upper(path: &str) -> String {
    path.strip_prefix("schemas/").map_or_else(|| path.to_string(), |rest| format!("Schemas/{rest}"))
}

#[test]
fn a_case_only_renamed_member_is_the_expected_file_where_the_volume_resolves_it_and_an_extra_where_it_does_not() {
    let home = Home::new();
    publish(&home.home, RegistrationInventory::claudine()).unwrap();
    home.write_config(&format!("register:\n    - {MANIFEST_REFERENCE}\n"));
    let schemas = output_root(&home.home).join("schemas");
    std::fs::rename(schemas.join("action.yaml"), schemas.join("Action.yaml")).unwrap();
    let insensitive = case_insensitive(&home.home);

    let found = inspect_installation(&home.home, Some(&home.config()), RegistrationInventory::claudine());
    let repair = home.repair();

    assert!(repair.failure.is_none(), "{repair:#?}");
    let names = listed(&schemas);
    if insensitive {
        assert!(found.is_current(), "both spellings reach one entry: {found:#?}");
        assert!(repair.operations.is_empty(), "nothing is replaced or removed: {repair:#?}");
        assert!(names.contains(&"Action.yaml".to_string()), "{names:?}");
    } else {
        assert!(found.extras.iter().any(|extra| extra.path == schemas.join("Action.yaml")), "{found:#?}");
        assert_eq!(
            repair.operations,
            [RepairOperation::Replaced(schemas.join("action.yaml")), RepairOperation::Removed(schemas.join("Action.yaml"))]
        );
        assert!(names.contains(&"action.yaml".to_string()) && !names.contains(&"Action.yaml".to_string()), "{names:?}");
    }
    assert!(home.current());
}

#[test]
fn a_case_only_renamed_folder_is_the_expected_subtree_where_the_volume_resolves_it_and_an_extra_where_it_does_not() {
    let home = Home::new();
    publish(&home.home, RegistrationInventory::claudine()).unwrap();
    home.write_config(&format!("register:\n    - {MANIFEST_REFERENCE}\n"));
    let root = output_root(&home.home);
    std::fs::rename(root.join("schemas"), root.join("Schemas")).unwrap();
    let insensitive = case_insensitive(&home.home);

    let found = inspect_installation(&home.home, Some(&home.config()), RegistrationInventory::claudine());
    let repair = home.repair();

    assert!(repair.failure.is_none(), "{repair:#?}");
    let names = listed(&root);
    if insensitive {
        assert!(found.is_current(), "the folder and its children are expected: {found:#?}");
        assert!(repair.operations.is_empty(), "nothing is replaced or removed: {repair:#?}");
        assert!(names.contains(&"Schemas".to_string()), "{names:?}");
    } else {
        assert!(found.extras.iter().any(|extra| extra.path == root.join("Schemas") && extra.directory), "{found:#?}");
        let action = index_of(&repair, &RepairOperation::Replaced(root.join("schemas/action.yaml")));
        let child = index_of(&repair, &RepairOperation::Removed(root.join("Schemas/action.yaml")));
        let folder = index_of(&repair, &RepairOperation::Removed(root.join("Schemas")));
        assert!(action < child && child < folder, "{repair:#?}");
        assert!(names.contains(&"schemas".to_string()) && !names.contains(&"Schemas".to_string()), "{names:?}");
    }
    assert!(home.current());
}

#[test]
fn a_previous_manifest_spelling_of_a_current_member_in_another_case_is_retired_only_where_it_is_a_distinct_entry() {
    let home = Home::new();
    publish(&home.home, &respelled(action_as_upper, [])).unwrap();
    let schemas = output_root(&home.home).join("schemas");
    let insensitive = case_insensitive(&home.home);

    let outcome = publish(&home.home, RegistrationInventory::claudine()).unwrap();

    let changes = outcome.changes().expect("the manifest changed");
    if insensitive {
        assert_eq!(changes.written, ["manifest.yaml"], "the member already holds its bytes: {changes:#?}");
        assert!(changes.retired.is_empty(), "the current member is not retired: {changes:#?}");
        assert!(listed(&schemas).contains(&"Action.yaml".to_string()));
    } else {
        assert_eq!(changes.written, ["schemas/action.yaml", "manifest.yaml"], "{changes:#?}");
        assert_eq!(changes.retired, ["schemas/Action.yaml"], "{changes:#?}");
        assert!(!listed(&schemas).contains(&"Action.yaml".to_string()));
    }
    assert!(inspect_installation(&home.home, None, RegistrationInventory::claudine()).is_current());
}

#[test]
fn a_previous_manifest_folder_in_another_case_keeps_its_current_members_and_the_folder_where_it_is_one_entry() {
    let home = Home::new();
    let retired = PublishedFile::new(
        "Schemas/retired.yaml",
        FileKind::Library,
        None::<&'static str>,
        b"kind: schema\ntypes:\n    retired: string\n".to_vec(),
    );
    let previous = respelled(schemas_as_upper, [retired]);
    publish(&home.home, &previous).unwrap();
    let root = output_root(&home.home);
    let insensitive = case_insensitive(&home.home);

    let outcome = publish(&home.home, RegistrationInventory::claudine()).unwrap();

    let changes = outcome.changes().expect("the manifest changed");
    if insensitive {
        assert_eq!(changes.written, ["manifest.yaml"], "every member already holds its bytes: {changes:#?}");
        assert_eq!(changes.retired, ["Schemas/retired.yaml"], "only the retired member goes: {changes:#?}");
        assert!(changes.removed_folders.is_empty(), "`Schemas` is the members' folder: {changes:#?}");
        assert!(listed(&root).contains(&"Schemas".to_string()));
    } else {
        let members = RegistrationInventory::claudine().members().filter(|file| file.path().starts_with("schemas/")).count();
        assert_eq!(changes.written.len(), members + 1, "{changes:#?}");
        assert_eq!(changes.retired.len(), members + 1, "{changes:#?}");
        assert_eq!(changes.removed_folders, ["Schemas"], "{changes:#?}");
        assert!(!listed(&root).contains(&"Schemas".to_string()));
    }
    assert!(inspect_installation(&home.home, None, RegistrationInventory::claudine()).is_current());
}

#[test]
fn a_digest_listed_under_another_case_is_checked_where_the_volume_resolves_it() {
    let home = Home::new();
    publish(&home.home, &respelled(action_as_upper, [])).unwrap();
    std::fs::write(output_root(&home.home).join("schemas/Action.yaml"), "kind: schema\n").unwrap();
    let insensitive = case_insensitive(&home.home);

    let mut found = inspect_installation(&home.home, None, RegistrationInventory::claudine());
    found.judge_manifest(&home.home, &home.context);

    let action = found.files.iter().find(|file| file.path.ends_with("action.yaml")).unwrap();
    if insensitive {
        assert_eq!(action.state, super::super::ExpectedState::Changed, "{action:#?}");
        assert!(action.digest_mismatch, "the listed digest is the alias's: {action:#?}");
    } else {
        assert_eq!(action.state, super::super::ExpectedState::Missing, "{action:#?}");
        assert!(!action.digest_mismatch, "{action:#?}");
    }
}

#[test]
fn the_retired_layout_is_found_and_removed_by_the_same_lookup_in_any_case() {
    let home = Home::new();
    let claudine = home.home.join(".claudine");
    std::fs::write(claudine.join("Darkmatter-Host.yaml"), "kind: darkmatter-host\n").unwrap();
    std::fs::create_dir_all(claudine.join("Darkmatter-Host/r1")).unwrap();
    std::fs::write(claudine.join("Darkmatter-Host/r1/a.yaml"), "kind: schema\n").unwrap();
    let insensitive = case_insensitive(&home.home);

    let found = inspect_installation(&home.home, None, RegistrationInventory::claudine());
    let cleanup = remove_legacy_registration(&home.home).unwrap();

    let names = listed(&claudine);
    if insensitive {
        let expected = [legacy_manifest_path(&home.home), legacy_revisions_dir(&home.home)];
        assert_eq!(found.legacy, expected, "{found:#?}");
        assert_eq!(cleanup.removed.len(), 2, "{cleanup:#?}");
        assert!(!names.iter().any(|name| name.eq_ignore_ascii_case("darkmatter-host.yaml")), "{names:?}");
        assert!(!names.iter().any(|name| name.eq_ignore_ascii_case("darkmatter-host")), "{names:?}");
    } else {
        assert!(found.legacy.is_empty(), "another spelling is not the retired layout: {found:#?}");
        assert!(cleanup.removed.is_empty(), "{cleanup:#?}");
        assert!(names.contains(&"Darkmatter-Host.yaml".to_string()) && names.contains(&"Darkmatter-Host".to_string()));
    }
}
