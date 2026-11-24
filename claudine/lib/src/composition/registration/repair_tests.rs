//! The repair over real temporary homes: its ordering, its refusals, its
//! failure points, and that it converges and then changes nothing.

use std::collections::HashMap;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};

use super::super::{
    MANIFEST_REFERENCE, PublishStep, RegistrationInventory, inspect_installation, legacy_manifest_path,
    legacy_revisions_dir, output_root, publish,
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
