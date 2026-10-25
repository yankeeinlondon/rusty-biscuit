//! Real repositories; rename failures a host cannot produce are injected
//! through [`Faults::rename_instead`] and classified as real ones.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::*;
use crate::remove::admin_entry::same_location;
use crate::remove::quarantine::association::plan_move;
use crate::remove::quarantine::rename;
use crate::remove::recovery::OperationId;
use crate::remove::test_support::TestRepo;
use crate::worktree::{WorktreeEntry, parse_worktree_list};
use sniff::filesystem::query::FileIdentity;

fn listed(repo: &TestRepo, path: &Path) -> Vec<WorktreeEntry> {
    parse_worktree_list(&repo.git(&["worktree", "list", "--porcelain"]))
        .into_iter()
        .filter(|entry| same_location(&entry.path, path))
        .collect()
}

fn planned(repo: &TestRepo, checkout: &Path) -> MovePlan {
    let [entry] = listed(repo, checkout).try_into().expect("listed once");
    plan_move(&repo.path(), &entry, &OperationId::generate().unwrap()).unwrap()
}

/// The checkout's directory, its `.git` file, Git's record, and the listing.
#[derive(Debug, PartialEq)]
struct Snapshot {
    identity: FileIdentity,
    dot_git: Vec<u8>,
    gitdir: Vec<u8>,
    listing: String,
}

fn snapshot(repo: &TestRepo, plan: &MovePlan) -> Snapshot {
    Snapshot {
        identity: rename::identity(&plan.original).unwrap(),
        dot_git: fs::read(plan.original.join(".git")).unwrap(),
        gitdir: fs::read(plan.admin.dir.join("gitdir")).unwrap(),
        listing: repo.git(&["worktree", "list", "--porcelain"]),
    }
}

fn assert_in_place(repo: &TestRepo, plan: &MovePlan, before: &Snapshot) {
    assert_eq!(&snapshot(repo, plan), before, "checkout and registration as they were");
    assert_eq!(before.identity, plan.identity);
    assert!(!plan.quarantine.exists(), "nothing at the quarantine");
}

#[test]
fn each_rename_failure_has_one_placement() {
    for (kind, expected) in [
        (RenameFailure::Unsupported, Some(InPlaceReason::NoReplaceUnsupported)),
        (RenameFailure::Sharing, Some(InPlaceReason::InUse)),
        (RenameFailure::Collision, None),
        (RenameFailure::CrossDevice, None),
        (RenameFailure::Permission, None),
        (RenameFailure::Other, None),
    ] {
        assert_eq!(in_place_for_rename(kind), expected, "{kind:?}");
    }
}

/// Only a verified link that cannot be rewritten becomes deletion in place;
/// a `.git` naming another record (`Changed`) stays a refusal.
#[test]
fn only_an_unmovable_link_refusal_is_deleted_in_place() {
    assert_eq!(
        in_place_for_refusal(&MoveRefusal::LinkNotMovable("why".into())),
        Some(InPlaceReason::LinkNotMovable("why".into()))
    );
    for refusal in [
        MoveRefusal::NotARealDirectory(PathBuf::from("x")),
        MoveRefusal::Locked { reason: None },
        MoveRefusal::Unsupported("main checkout".into()),
        MoveRefusal::Association(crate::remove::admin_entry::AssociationError::NotFound),
        MoveRefusal::Changed("its .git file names another record".into()),
        MoveRefusal::Uninspectable("denied".into()),
    ] {
        assert_eq!(in_place_for_refusal(&refusal), None, "{refusal:?}");
    }
}

#[test]
fn a_normal_checkout_is_moved() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/moved", "moved", "main");
    let plan = planned(&repo, &target);

    let placement = place(&repo.path(), &plan, &[]).unwrap();

    assert_eq!(placement, Placement::Moved);
    assert!(!target.exists());
    assert_eq!(rename::identity(&plan.quarantine).unwrap(), plan.identity);
    assert_eq!(listed(&repo, &plan.quarantine).len(), 1);
}

/// WSL drvfs answers `EINVAL` (Windows: `ERROR_NOT_SUPPORTED`) to a
/// no-replacement rename (R4).
#[test]
fn an_unsupported_rename_deletes_in_place_and_changes_nothing() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/drvfs", "drvfs", "main");
    let plan = planned(&repo, &target);
    let before = snapshot(&repo, &plan);
    #[cfg(unix)]
    let unsupported: fn() -> io::Error = || io::Error::from_raw_os_error(libc::EINVAL);
    #[cfg(windows)]
    let unsupported: fn() -> io::Error = || io::Error::from_raw_os_error(50); // ERROR_NOT_SUPPORTED
    let faults = Faults { rename_instead: Some(unsupported), ..Faults::default() };

    let placement = place_with(&repo.path(), &plan, &[], &faults).unwrap();

    assert_eq!(placement, Placement::InPlace(InPlaceReason::NoReplaceUnsupported));
    assert_in_place(&repo, &plan, &before);
}

/// Only mounts strictly inside the checkout count, and no rename is tried.
#[test]
fn a_nested_mount_deletes_in_place_without_trying_the_move() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/mounted", "mounted", "main");
    let plan = planned(&repo, &target);
    fs::create_dir(plan.original.join("vendor")).unwrap();
    let before = snapshot(&repo, &plan);
    let mount = |point: &Path, source: &str| Mount { mount_point: point.to_path_buf(), source: source.into(), fs_type: String::new(), super_options: String::new() };
    let inside = mount(&plan.original.join("vendor"), "/dev/disk3");
    let table = [mount(plan.original.parent().unwrap(), "/dev/disk1"), mount(&plan.original, "/dev/disk2"), inside.clone()];
    let attempted: fn() -> io::Error = || panic!("the move was attempted");
    let faults = Faults { rename_instead: Some(attempted), ..Faults::default() };

    let placement = place_with(&repo.path(), &plan, &table, &faults).unwrap();

    assert_eq!(placement, Placement::InPlace(InPlaceReason::NestedMounts(vec![inside])));
    assert_in_place(&repo, &plan, &before);
}

#[test]
fn a_collision_at_the_hidden_name_stays_an_error_and_replaces_nothing() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/collide", "collide", "main");
    let plan = planned(&repo, &target);
    let before = snapshot(&repo, &plan);
    fs::create_dir(&plan.quarantine).unwrap();
    fs::write(plan.quarantine.join("occupant.txt"), "kept\n").unwrap();
    let occupant = rename::identity(&plan.quarantine).unwrap();

    let error = place(&repo.path(), &plan, &[]).unwrap_err();

    assert!(matches!(&error, MoveError::Rename(rename) if rename.kind == RenameFailure::Collision), "{error:?}");
    assert_eq!(rename::identity(&plan.quarantine).unwrap(), occupant, "the occupant stays");
    assert_eq!(fs::read_to_string(plan.quarantine.join("occupant.txt")).unwrap(), "kept\n");
    fs::remove_dir_all(&plan.quarantine).unwrap();
    assert_in_place(&repo, &plan, &before);
}

/// Another volume is an error, not a reason to delete in place.
#[test]
fn a_rename_to_another_volume_stays_an_error() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/volume", "volume", "main");
    let plan = planned(&repo, &target);
    let before = snapshot(&repo, &plan);
    #[cfg(unix)]
    let cross_device: fn() -> io::Error = || io::Error::from_raw_os_error(libc::EXDEV);
    #[cfg(windows)]
    let cross_device: fn() -> io::Error = || io::Error::from_raw_os_error(17); // ERROR_NOT_SAME_DEVICE
    let faults = Faults { rename_instead: Some(cross_device), ..Faults::default() };

    let error = place_with(&repo.path(), &plan, &[], &faults).unwrap_err();

    assert!(matches!(&error, MoveError::Rename(rename) if rename.kind == RenameFailure::CrossDevice), "{error:?}");
    assert_in_place(&repo, &plan, &before);
}

/// Windows refuses to move a directory holding an open file; the checkout
/// is deleted in place while the engine waits on that file.
#[cfg(windows)]
#[test]
fn an_open_file_in_the_checkout_deletes_in_place() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/held", "held", "main");
    let plan = planned(&repo, &target);
    let before = snapshot(&repo, &plan);
    let held = fs::File::open(plan.original.join("README.md")).unwrap();

    let placement = place(&repo.path(), &plan, &[]);

    drop(held);
    assert_eq!(placement.unwrap(), Placement::InPlace(InPlaceReason::InUse));
    assert_in_place(&repo, &plan, &before);
}
