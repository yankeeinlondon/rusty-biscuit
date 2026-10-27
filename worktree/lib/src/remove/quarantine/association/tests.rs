//! Real repositories, real renames, and Git reading the result back. Faults
//! after the rename are injected through [`Faults`].

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::git::recorder;
use crate::remove::recovery::record::QUARANTINE_PREFIX;
use crate::remove::test_support::TestRepo;

fn listed(repo: &TestRepo, path: &Path) -> WorktreeEntry {
    parse_worktree_list(&repo.git(&["worktree", "list", "--porcelain"]))
        .into_iter()
        .find(|entry| same_location(&entry.path, path))
        .expect("listed")
}

fn listed_at(repo: &TestRepo, path: &Path) -> Vec<WorktreeEntry> {
    parse_worktree_list(&repo.git(&["worktree", "list", "--porcelain"]))
        .into_iter()
        .filter(|entry| same_location(&entry.path, path))
        .collect()
}

fn admin_dir(repo: &TestRepo, checkout: &Path) -> PathBuf {
    PathBuf::from(repo.git_in(checkout, &["rev-parse", "--path-format=absolute", "--git-dir"]))
}

fn plan(repo: &TestRepo, checkout: &Path) -> Result<MovePlan, MoveRefusal> {
    plan_move(&repo.path(), &listed(repo, checkout), &OperationId::generate().unwrap())
}

/// Every Git-side byte the move could touch: each worktree record's files,
/// the target's `.git` file, and the listing.
#[derive(Debug, PartialEq)]
struct GitState {
    records: Vec<(PathBuf, Vec<u8>)>,
    dot_git: Vec<u8>,
    listing: String,
}

fn git_state(repo: &TestRepo, checkout: &Path) -> GitState {
    let records = repo.path().join(".git").join("worktrees");
    let mut files = Vec::new();
    for record in fs::read_dir(&records).unwrap() {
        let record = record.unwrap().path();
        for name in ["gitdir", "locked"] {
            if let Ok(bytes) = fs::read(record.join(name)) {
                files.push((record.join(name), bytes));
            }
        }
    }
    files.sort();
    GitState {
        records: files,
        dot_git: fs::read(checkout.join(".git")).unwrap_or_default(),
        listing: repo.git(&["worktree", "list", "--porcelain"]),
    }
}

/// The checkout is where it was, with its content, identity, and listing.
fn assert_untouched(repo: &TestRepo, plan: &MovePlan, before: &GitState) {
    assert_eq!(rename::identity(&plan.original).unwrap(), plan.identity, "the checkout is at its path");
    assert!(plan.original.join("README.md").is_file());
    assert!(!plan.quarantine.exists(), "no quarantine left behind");
    assert_eq!(&git_state(repo, &plan.original), before);
}

fn forbidden_git(calls: &[Vec<String>]) -> Vec<&Vec<String>> {
    calls
        .iter()
        .filter(|args| {
            args.iter().any(|arg| ["move", "repair", "prune", "unlock", "--force", "-f"].contains(&arg.as_str()))
        })
        .collect()
}

#[test]
fn an_absolute_record_moves_and_git_reads_the_quarantine() {
    let repo = TestRepo::new();
    let keep = repo.add_worktree("feat/keep", "keep", "main");
    let gone = repo.add_worktree("feat/gone", "gone", "main");
    fs::remove_dir_all(&gone).unwrap();
    let target = repo.add_worktree("feat/x", "x", "main");
    fs::write(target.join("scratch.txt"), "untracked\n").unwrap();
    let admin = admin_dir(&repo, &target);
    let recorded = fs::read(admin.join("gitdir")).unwrap();
    assert!(Path::new(&*String::from_utf8_lossy(&recorded)).is_absolute(), "Git wrote an absolute record");
    let dot_git = fs::read(target.join(".git")).unwrap();
    let others = [&keep, &gone].map(|checkout| fs::read(admin.parent().unwrap().join(checkout.file_name().unwrap()).join("gitdir")).unwrap());
    let plan = plan(&repo, &target).unwrap();
    let name = plan.quarantine.file_name().unwrap().to_string_lossy().into_owned();
    assert_eq!(plan.quarantine.parent(), plan.original.parent(), "a sibling");
    assert!(same_location(&plan.original, &target));
    assert!(name.starts_with(&format!("{QUARANTINE_PREFIX}x-")), "{name}");

    recorder::start_recording();
    move_with(&repo.path(), &plan, &Faults::default()).unwrap();
    let calls = recorder::finish_recording();

    assert!(!target.exists(), "nothing at the original path");
    assert_eq!(rename::identity(&plan.quarantine).unwrap(), plan.identity);
    assert_eq!(fs::read_to_string(plan.quarantine.join("scratch.txt")).unwrap(), "untracked\n");
    let expected = String::from_utf8(recorded).unwrap().replace("/x/.git", &format!("/{name}/.git"));
    assert_eq!(fs::read_to_string(admin.join("gitdir")).unwrap(), expected, "only the name changed");
    assert_eq!(fs::read(plan.quarantine.join(".git")).unwrap(), dot_git, ".git file unchanged");
    let now = listed_at(&repo, &plan.quarantine);
    assert_eq!(now.len(), 1);
    assert_eq!((now[0].branch.as_deref(), now[0].prunable.as_deref()), (Some("feat/x"), None));
    assert!(listed_at(&repo, &target).is_empty());
    assert_eq!(repo.git_in(&plan.quarantine, &["status", "--porcelain"]), "?? scratch.txt");
    let pruned = repo.git(&["worktree", "prune", "--dry-run", "--verbose"]);
    assert!(!pruned.contains("worktrees/x"), "prune leaves the quarantine alone:\n{pruned}");
    let others_now = [&keep, &gone].map(|checkout| fs::read(admin.parent().unwrap().join(checkout.file_name().unwrap()).join("gitdir")).unwrap());
    assert_eq!(others_now, others, "other records untouched, the prunable one included");
    assert!(forbidden_git(&calls).is_empty(), "{calls:?}");
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const HIDDEN: u32 = 0x2;
        assert_ne!(fs::symlink_metadata(&plan.quarantine).unwrap().file_attributes() & HIDDEN, 0, "hidden");
    }
}

/// Git 2.48 and later can write links relative to the admin directory;
/// the record keeps that format. Older Git skips with a message.
#[test]
fn a_relative_record_keeps_its_format() {
    let repo = TestRepo::new();
    let target = repo.path().parent().unwrap().join("wts").join("rel");
    let added = repo.try_git(&["-c", "worktree.useRelativePaths=true", "worktree", "add", "-q", "-b", "feat/rel", target.to_str().unwrap(), "main"]);
    added.unwrap();
    let admin = admin_dir(&repo, &target);
    let recorded = fs::read_to_string(admin.join("gitdir")).unwrap();
    if Path::new(recorded.trim_end()).is_absolute() {
        eprintln!("skipped: this Git writes absolute worktree links only");
        return;
    }
    let dot_git = fs::read(target.join(".git")).unwrap();
    let plan = plan(&repo, &target).unwrap();
    let name = plan.quarantine.file_name().unwrap().to_string_lossy().into_owned();

    move_with(&repo.path(), &plan, &Faults::default()).unwrap();

    let rewritten = fs::read_to_string(admin.join("gitdir")).unwrap();
    assert_eq!(rewritten, recorded.replace("/rel/.git", &format!("/{name}/.git")));
    assert!(!Path::new(rewritten.trim_end()).is_absolute(), "still relative: {rewritten}");
    assert_eq!(fs::read(plan.quarantine.join(".git")).unwrap(), dot_git, ".git file unchanged");
    let now = listed_at(&repo, &plan.quarantine);
    assert_eq!((now.len(), now[0].prunable.as_deref()), (1, None));
    repo.git_in(&plan.quarantine, &["status", "--porcelain"]);
    let pruned = repo.git(&["worktree", "prune", "--dry-run", "--verbose"]);
    assert!(!pruned.contains("worktrees/rel"), "{pruned}");
}

/// A lock does not refuse planning (the caller unlocks after it), but the
/// move itself still refuses one and keeps it.
#[test]
fn a_locked_worktree_plans_but_refuses_the_move() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/locked", "locked", "main");
    repo.git(&["worktree", "lock", "--reason", "in use by CI", target.to_str().unwrap()]);
    let plan = plan(&repo, &target).unwrap();
    let before = git_state(&repo, &target);

    let result = move_with(&repo.path(), &plan, &Faults::default());

    assert!(
        matches!(&result, Err(MoveError::Refused(MoveRefusal::Locked { reason: Some(reason) })) if reason == "in use by CI"),
        "{result:?}"
    );
    assert_eq!(result.unwrap_err().class(), MoveErrorClass::Refused);
    assert_untouched(&repo, &plan, &before);
    assert!(admin_dir(&repo, &target).join("locked").exists(), "the lock is kept");
}

/// Submodules are deleted with the checkout, so neither a gitlink in the
/// index nor a `modules` directory in the record stops the move, though Git
/// itself refuses to move either.
#[test]
fn a_checkout_with_a_submodule_moves() {
    let repo = TestRepo::new();
    let gitlink = repo.add_worktree("feat/gitlink", "gitlink", "main");
    let head = repo.sha("HEAD");
    repo.git_in(&gitlink, &["update-index", "--add", "--cacheinfo", &format!("160000,{head},vendor/lib")]);
    let modules = repo.add_worktree("feat/modules", "modules", "main");
    let module_dir = admin_dir(&repo, &modules).join("modules");
    fs::create_dir_all(module_dir.join("lib")).unwrap();

    for checkout in [&gitlink, &modules] {
        let plan = plan(&repo, checkout).unwrap();
        move_with(&repo.path(), &plan, &Faults::default()).unwrap();
        assert!(!checkout.exists() && plan.quarantine.join("README.md").is_file(), "{}", checkout.display());
        assert_eq!(listed_at(&repo, &plan.quarantine).len(), 1);
    }
    assert!(module_dir.join("lib").is_dir(), "the record's modules stay with it");
}

#[test]
fn unsupported_shapes_refuse_before_anything_moves() {
    let repo = TestRepo::new();
    let main = listed(&repo, &repo.path());
    let refused = plan_move(&repo.path(), &main, &OperationId::generate().unwrap());
    assert!(matches!(&refused, Err(MoveRefusal::Unsupported(why)) if why.contains("main checkout")), "{refused:?}");

    // A relative link with only leading `..` survives the move unchanged.
    let target = repo.add_worktree("feat/lead", "lead", "main");
    let link = b"gitdir: ../../repo/.git/worktrees/lead\n";
    fs::write(target.join(".git"), link).unwrap();
    let plan = plan(&repo, &target).unwrap();
    move_with(&repo.path(), &plan, &Faults::default()).unwrap();
    repo.git_in(&plan.quarantine, &["status", "--porcelain"]);
    assert_eq!(fs::read(plan.quarantine.join(".git")).unwrap(), link);
}

/// Git reads `../bent/..` through the checkout's own name, which is gone
/// after the rename; the move replaces the link with an absolute one (R8).
#[test]
fn a_link_through_the_checkout_is_rewritten_for_the_move() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/bent", "bent", "main");
    fs::write(target.join(".git"), "gitdir: ../bent/../../repo/.git/worktrees/bent\r\n").unwrap();
    fs::write(target.join("scratch.txt"), "untracked\n").unwrap();
    let admin = admin_dir(&repo, &target);
    let plan = plan(&repo, &target).unwrap();

    move_with(&repo.path(), &plan, &Faults::default()).unwrap();

    let link = fs::read_to_string(plan.quarantine.join(".git")).unwrap();
    let named = link.strip_prefix("gitdir: ").and_then(|rest| rest.strip_suffix("\r\n")).expect(&link);
    assert!(Path::new(named).is_absolute() && same_location(Path::new(named), &admin), "{link:?}");
    assert!(same_location(&read_back_reference(&admin).unwrap(), &plan.quarantine.join(".git")));
    let now = listed_at(&repo, &plan.quarantine);
    assert_eq!((now.len(), now[0].branch.as_deref(), now[0].prunable.as_deref()), (1, Some("feat/bent"), None));
    assert!(listed_at(&repo, &target).is_empty());
    assert_eq!(repo.git_in(&plan.quarantine, &["status", "--porcelain"]), "?? scratch.txt");
}

/// The rollback puts the original link back before the checkout returns.
#[test]
fn a_rewritten_link_is_restored_when_the_move_is_rolled_back() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/bent", "bent", "main");
    fs::write(target.join(".git"), "gitdir: ../bent/../../repo/.git/worktrees/bent\n").unwrap();
    let plan = plan(&repo, &target).unwrap();
    let before = git_state(&repo, &target);

    let faults = Faults { fail_write: true, ..Faults::default() };
    let error = move_with(&repo.path(), &plan, &faults).unwrap_err();

    assert!(matches!(&error, MoveError::RolledBack { cause: AssociationFailure::Write(_) }), "{error:?}");
    assert_untouched(&repo, &plan, &before);
    let leftovers: Vec<_> = fs::read_dir(&target).unwrap().map(|entry| entry.unwrap().file_name()).filter(|name| name.to_string_lossy().ends_with(".tmp")).collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

/// A link through the checkout that names another worktree's record is a
/// refusal, never a rewrite.
#[test]
fn a_link_through_the_checkout_to_another_record_refuses() {
    let repo = TestRepo::new();
    repo.add_worktree("feat/other", "other", "main");
    let target = repo.add_worktree("feat/bent", "bent", "main");
    fs::write(target.join(".git"), "gitdir: ../bent/../../repo/.git/worktrees/other\n").unwrap();
    let before = git_state(&repo, &target);

    let refused = plan(&repo, &target);

    assert!(matches!(&refused, Err(MoveRefusal::Changed(why)) if why.contains("other")), "{refused:?}");
    assert_eq!(git_state(&repo, &target), before);
}

#[test]
fn the_link_rewrite_names_the_record_absolutely() {
    let admin = std::env::temp_dir().join("repo").join(".git").join("worktrees").join("x");
    let spelled = admin.to_str().unwrap().replace('\\', "/");
    for ending in [&b"\n"[..], b"\r\n", b""] {
        let mut expected = format!("gitdir: {spelled}").into_bytes();
        expected.extend_from_slice(ending);
        assert_eq!(absolute_link(&admin, ending), Some(expected));
    }
    assert_eq!(absolute_link(Path::new("repo/.git/worktrees/x"), b"\n"), None, "relative");
    assert_eq!(absolute_link(&admin.join("a\nb"), b"\n"), None, "not one line");
}

#[test]
fn state_that_changed_since_planning_refuses_the_move() {
    let repo = TestRepo::new();

    let switched = repo.add_worktree("feat/switched", "switched", "main");
    let plan_switched = plan(&repo, &switched).unwrap();
    repo.git_in(&switched, &["switch", "-q", "-c", "feat/other"]);
    let before = git_state(&repo, &switched);
    let result = move_with(&repo.path(), &plan_switched, &Faults::default());
    assert!(matches!(&result, Err(MoveError::Refused(MoveRefusal::Changed(_)))), "{result:?}");
    assert_untouched(&repo, &plan_switched, &before);

    // Another directory with the same content now stands at the path.
    let replaced = repo.add_worktree("feat/replaced", "replaced", "main");
    let plan_replaced = plan(&repo, &replaced).unwrap();
    let saved = replaced.with_file_name("saved");
    fs::rename(&replaced, &saved).unwrap();
    fs::create_dir(&replaced).unwrap();
    for name in [".git", "README.md"] {
        fs::copy(saved.join(name), replaced.join(name)).unwrap();
    }
    let result = move_with(&repo.path(), &plan_replaced, &Faults::default());
    assert!(matches!(&result, Err(MoveError::Refused(MoveRefusal::Changed(why))) if why.contains("different directory")), "{result:?}");
    assert!(replaced.join(".git").is_file() && saved.join(".git").is_file() && !plan_replaced.quarantine.exists());

    let linked = repo.add_worktree("feat/linked", "linked", "main");
    let plan_linked = plan(&repo, &linked).unwrap();
    let real = crate::remove::test_support::replace_with_link(&linked, "real");
    let result = move_with(&repo.path(), &plan_linked, &Faults::default());
    assert!(matches!(&result, Err(MoveError::Refused(MoveRefusal::NotARealDirectory(_)))), "{result:?}");
    assert!(real.join("README.md").is_file() && !plan_linked.quarantine.exists());
}

#[test]
fn a_collision_at_the_quarantine_is_never_replaced() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/collide", "collide", "main");
    let plan = plan(&repo, &target).unwrap();
    let before = git_state(&repo, &target);
    fs::create_dir(&plan.quarantine).unwrap();
    let occupant = rename::identity(&plan.quarantine).unwrap();

    let error = move_with(&repo.path(), &plan, &Faults::default()).unwrap_err();

    assert!(matches!(&error, MoveError::Rename(rename) if rename.kind == RenameFailure::Collision), "{error:?}");
    assert_eq!((error.class(), error.nothing_changed()), (MoveErrorClass::Failed, true));
    assert_eq!(rename::identity(&plan.quarantine).unwrap(), occupant, "the occupant stays");
    assert_eq!(fs::read_dir(&plan.quarantine).unwrap().count(), 0);
    fs::remove_dir(&plan.quarantine).unwrap();
    assert_untouched(&repo, &plan, &before);
}

#[test]
fn a_failed_record_rewrite_is_rolled_back() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/write", "write", "main");
    let plan = plan(&repo, &target).unwrap();
    let before = git_state(&repo, &target);

    let faults = Faults { fail_write: true, ..Faults::default() };
    let error = move_with(&repo.path(), &plan, &faults).unwrap_err();

    assert!(matches!(&error, MoveError::RolledBack { cause: AssociationFailure::Write(_) }), "{error:?}");
    assert_eq!((error.class(), error.nothing_changed()), (MoveErrorClass::Failed, true));
    assert_untouched(&repo, &plan, &before);
}

/// The record was written but Git does not read it as the quarantine; the
/// original record is restored along with the directory.
#[test]
fn an_unverified_record_rewrite_is_rolled_back_and_restored() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/verify", "verify", "main");
    let plan = plan(&repo, &target).unwrap();
    let before = git_state(&repo, &target);
    let elsewhere = repo.path().parent().unwrap().join("elsewhere").join(".git");

    let faults = Faults { write_instead: Some(format!("{}\n", elsewhere.display()).into_bytes()), ..Faults::default() };
    let error = move_with(&repo.path(), &plan, &faults).unwrap_err();

    assert!(matches!(&error, MoveError::RolledBack { cause: AssociationFailure::Unverified(_) }), "{error:?}");
    assert_untouched(&repo, &plan, &before);
}

#[test]
fn a_recreated_original_is_never_overwritten() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/recreated", "recreated", "main");
    let plan = plan(&repo, &target).unwrap();
    let original = plan.original.clone();

    let faults = Faults {
        fail_write: true,
        after_rename: Some(Box::new(move || {
            fs::create_dir(&original).unwrap();
            fs::write(original.join("new.txt"), "written after the move\n").unwrap();
        })),
        ..Faults::default()
    };
    let error = move_with(&repo.path(), &plan, &faults).unwrap_err();

    let MoveError::Stranded { reason, state, .. } = &error else { panic!("{error:?}") };
    assert!(reason.contains("original path"), "{reason}");
    let expected = MoveState { checkout: CheckoutLocation::Quarantine, original_recreated: true, registration: RegistrationState::AtOriginal };
    assert_eq!(*state, expected);
    assert_eq!(fs::read_to_string(target.join("new.txt")).unwrap(), "written after the move\n");
    assert_eq!(fs::read_dir(&target).unwrap().count(), 1, "nothing merged into it");
    assert!(plan.quarantine.join("README.md").is_file());
}

/// Whatever replaced the moved directory at the quarantine is not moved to
/// the original path.
#[test]
fn a_replaced_quarantine_is_not_moved_back() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/swapped", "swapped", "main");
    let plan = plan(&repo, &target).unwrap();
    let quarantine = plan.quarantine.clone();
    let aside = target.with_file_name("aside");
    let moved_aside = aside.clone();

    let faults = Faults {
        after_rename: Some(Box::new(move || {
            fs::rename(&quarantine, &moved_aside).unwrap();
            fs::create_dir(&quarantine).unwrap();
        })),
        ..Faults::default()
    };
    let error = move_with(&repo.path(), &plan, &faults).unwrap_err();

    let MoveError::Stranded { cause, state, .. } = &error else { panic!("{error:?}") };
    assert_eq!(*cause, AssociationFailure::IdentityChanged);
    assert_eq!((state.checkout, state.original_recreated), (CheckoutLocation::Unknown, false));
    assert!(!target.exists(), "the stranger stayed at the quarantine");
    assert_eq!(fs::read_dir(&plan.quarantine).unwrap().count(), 0);
    assert!(aside.join("README.md").is_file());
}

#[test]
fn rename_failures_map_to_their_exit_classes() {
    let rename_error = |kind| MoveError::Rename(RenameError { kind, source: io::Error::other("x") });
    for (kind, class) in [
        (RenameFailure::Sharing, MoveErrorClass::BlockedByEnvironment),
        (RenameFailure::Unsupported, MoveErrorClass::BlockedByEnvironment),
        (RenameFailure::Collision, MoveErrorClass::Failed),
        (RenameFailure::CrossDevice, MoveErrorClass::Failed),
        (RenameFailure::Permission, MoveErrorClass::Failed),
        (RenameFailure::Other, MoveErrorClass::Failed),
    ] {
        let error = rename_error(kind);
        assert_eq!((error.class(), error.nothing_changed()), (class, true), "{kind:?}");
    }
    let refused = MoveError::Refused(MoveRefusal::Locked { reason: None });
    assert_eq!((refused.class(), refused.nothing_changed()), (MoveErrorClass::Refused, true));
}

/// A Windows process holding a file inside the checkout blocks the rename;
/// that is "in use" (exit 4), and nothing moved.
#[cfg(windows)]
#[test]
fn an_open_file_in_the_checkout_blocks_the_move_as_in_use() {
    let repo = TestRepo::new();
    let target = repo.add_worktree("feat/held", "held", "main");
    let plan = plan(&repo, &target).unwrap();
    let before = git_state(&repo, &target);
    let held = fs::File::open(target.join("README.md")).unwrap();

    let error = move_with(&repo.path(), &plan, &Faults::default()).unwrap_err();

    drop(held);
    assert!(matches!(&error, MoveError::Rename(rename) if rename.kind == RenameFailure::Sharing), "{error:?}");
    assert_eq!(error.class(), MoveErrorClass::BlockedByEnvironment);
    assert_untouched(&repo, &plan, &before);
}

#[test]
fn the_record_rewrite_changes_only_the_checkout_name() {
    let from = OsStr::new("x");
    let to = OsStr::new(".wt-remove-x-0123456789abcdef");
    for (recorded, expected) in [
        (&b"/w/x/.git\n"[..], Some(&b"/w/.wt-remove-x-0123456789abcdef/.git\n"[..])),
        (b"../../../wts/x/.git\n", Some(b"../../../wts/.wt-remove-x-0123456789abcdef/.git\n")),
        (b"/w/x/.git\r\n", Some(b"/w/.wt-remove-x-0123456789abcdef/.git\r\n")),
        (b"/w/x/.git", Some(b"/w/.wt-remove-x-0123456789abcdef/.git")),
        (b"x/.git\n", Some(b".wt-remove-x-0123456789abcdef/.git\n")),
        (b"/w/X/.git\n", None),
        (b"/w/xx/.git\n", None),
        (b"/w/x/git\n", None),
        (b"/w/x.git\n", None),
        (b"\n", None),
    ] {
        assert_eq!(rewrite_back_reference(recorded, from, to).as_deref(), expected, "{}", String::from_utf8_lossy(recorded));
    }
    #[cfg(windows)]
    assert_eq!(
        rewrite_back_reference(b"C:\\w\\x\\.git\n", from, to).as_deref(),
        Some(&b"C:\\w\\.wt-remove-x-0123456789abcdef\\.git\n"[..])
    );
}
