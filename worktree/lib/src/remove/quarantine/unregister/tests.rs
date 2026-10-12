//! The whole checked removal at library level against real repositories:
//! inventory, move, checked deletion, and the final unregister.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use super::*;
use crate::git::recorder;
use crate::remove::collect_inventory;
use crate::remove::missing::bind_index;
use crate::remove::observation::deletion::{DeletionOptions, delete_content};
use crate::remove::observation::injected::Injector;
use crate::remove::observation::inventory::DeletionInventory;
use crate::remove::observation::{ObserverOptions, Root, Targets};
use crate::remove::quarantine::association::{MovePlan, move_to_quarantine, plan_move};
use crate::remove::recovery::OperationId;
use crate::remove::recovery::journal::{self, DEFAULT_JOURNAL_LIMIT, EntryState, JournalWriter};
use crate::remove::test_support::{TestRepo, link_dir};
use crate::worktree::WorktreeEntry;

fn listed_at(repo: &TestRepo, path: &Path) -> Vec<WorktreeEntry> {
    parse_worktree_list(&repo.git(&["worktree", "list", "--porcelain"]))
        .into_iter()
        .filter(|entry| same_location(&entry.path, path))
        .collect()
}

/// A repository with two linked worktrees; `gone` holds a dirty file,
/// ignored build output, a nested directory, and a link to a directory
/// outside it.
struct Setup {
    repo: TestRepo,
    keep: PathBuf,
    plan: MovePlan,
    outside: PathBuf,
    sensitive: Vec<PathBuf>,
}

fn setup() -> Setup {
    let repo = TestRepo::new();
    fs::write(repo.path().join(".gitignore"), "target/\n").unwrap();
    repo.git(&["add", ".gitignore"]);
    repo.git(&["commit", "-q", "-m", "ignore"]);
    let gone = repo.add_worktree("feat/gone", "gone", "main");
    let keep = repo.add_worktree("feat/keep", "keep", "main");
    fs::write(gone.join("README.md"), "dirty\n").unwrap();
    fs::create_dir_all(gone.join("target").join("debug").join("deps")).unwrap();
    fs::write(gone.join("target").join("debug").join("deps").join("a.o"), "object").unwrap();
    fs::create_dir_all(gone.join("docs").join("deep")).unwrap();
    fs::write(gone.join("docs").join("deep").join("page.md"), "page\n").unwrap();
    let outside = repo.path().parent().unwrap().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep.txt"), "never deleted").unwrap();
    link_dir(&outside, &gone.join("target").join("linked"));

    // Consent is collected before observation starts, as `wt remove` does.
    let sensitive = collect_inventory(&repo.path(), &gone).unwrap().consent_sensitive_paths();
    let [entry] = listed_at(&repo, &gone).try_into().unwrap();
    let plan = plan_move(&repo.path(), &entry, &OperationId::generate().unwrap()).unwrap();
    Setup { repo, keep, plan, outside, sensitive }
}

impl Setup {
    fn targets(&self) -> Targets {
        Targets::new(&self.plan.original, &self.plan.quarantine, &self.plan.admin.dir).unwrap()
    }

    /// Inventory, approved index binding, the move, and the move's own
    /// events drained; returns the inventory and binding.
    fn prepare_and_move(&self, observation: &mut Observation) -> (Arc<DeletionInventory>, IndexBinding) {
        let inventory = Arc::new(DeletionInventory::capture(&self.plan.original, &self.sensitive).unwrap());
        inventory.confirm(&self.plan.original, &observation.drain()).unwrap();
        let approved = bind_index(&self.repo.path(), &self.plan.admin, self.repo.sha("refs/heads/feat/gone")).unwrap();
        move_to_quarantine(&self.repo.path(), &self.plan, &|| false).unwrap();
        observation.moved_to_quarantine();
        // The move's own events: the original renamed away, the quarantine
        // renamed in, and the rewritten admin `gitdir`. Checking them is the
        // lifecycle's job; here they only must not be loss or tree content.
        let own = observation.drain();
        assert_eq!(own.lost, None);
        for event in &own.events {
            let expected = match &event.scope {
                Scope::Original(relative) | Scope::Quarantine(relative) | Scope::Tree(relative) => relative.as_os_str().is_empty(),
                Scope::Admin(name) => name == "gitdir",
            };
            assert!(expected, "the move caused {event:?}");
        }
        (inventory, approved)
    }

    fn delete(&self, observation: &mut Observation, inventory: Arc<DeletionInventory>) -> Vec<u8> {
        let sink = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut writer = JournalWriter::start(SharedSink(Arc::clone(&sink)), &OperationId::parse(&"0".repeat(32)).unwrap(), inventory, DEFAULT_JOURNAL_LIMIT).unwrap();
        let deletion = delete_content(observation, &mut writer, &|| false, DeletionOptions::default());
        assert!(deletion.stop.is_none(), "{:?}", deletion.stop);
        sink.lock().unwrap().clone()
    }

    fn registered_at_quarantine(&self) -> bool {
        listed_at(&self.repo, &self.plan.quarantine).len() == 1
    }
}

struct SharedSink(Arc<std::sync::Mutex<Vec<u8>>>);

impl crate::remove::recovery::journal::JournalSink for SharedSink {
    fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(())
    }

    fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn native_a_checked_removal_deletes_the_content_then_unregisters_only_this_worktree() {
    let setup = setup();
    let mut observation = Observation::start(setup.targets(), ObserverOptions::default()).unwrap();
    let (inventory, approved) = setup.prepare_and_move(&mut observation);
    let journal_bytes = setup.delete(&mut observation, Arc::clone(&inventory));

    recorder::start_recording();
    let warning = unregister_quarantine(&setup.repo.path(), &mut observation, &approved).unwrap();
    let calls = recorder::finish_recording();

    assert_eq!(warning, None);
    assert!(listed_at(&setup.repo, &setup.plan.quarantine).is_empty() && listed_at(&setup.repo, &setup.plan.original).is_empty());
    assert!(!setup.plan.admin.dir.exists(), "Git deleted the worktree's record");
    assert!(!setup.plan.quarantine.exists() && !setup.plan.original.exists());
    assert_eq!(listed_at(&setup.repo, &setup.keep).len(), 1, "the other worktree is untouched");
    assert!(setup.keep.join("README.md").is_file());
    setup.repo.git(&["rev-parse", "--verify", "refs/heads/feat/gone"]);
    assert_eq!(fs::read(setup.outside.join("keep.txt")).unwrap(), b"never deleted");
    let removes: Vec<_> = calls.iter().filter(|args| args.starts_with(&["worktree".into(), "remove".into()])).collect();
    assert_eq!(removes.len(), 1, "{calls:?}");
    let forbidden = ["--force", "-f", "prune", "repair", "move", "unlock"];
    assert!(!calls.iter().flatten().any(|arg| forbidden.contains(&arg.as_str())), "{calls:?}");

    // Ignored output and the dirty file were both approved and deleted, and
    // the journal says so.
    let log = journal::parse(&journal_bytes, &OperationId::parse(&"0".repeat(32)).unwrap()).unwrap();
    let entries = log.replay(|_| Ok(false)).unwrap();
    assert_eq!(entries.len(), inventory.entries().len());
    assert!(entries.iter().all(|entry| entry.state == EntryState::Deleted));
    for path in ["README.md", "target/debug/deps/a.o", "target/linked", ".git"] {
        assert!(inventory.get(Path::new(path)).is_some(), "{path} was inventoried");
    }
}

fn injected(setup: &Setup) -> (Observation, Injector) {
    Observation::injected(setup.targets(), ObserverOptions::default())
}

/// Runs `git` from the base checkout with `GIT_INDEX_FILE` set.
fn git_with_index(setup: &Setup, index: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(setup.repo.path())
        .args(args)
        .env("GIT_INDEX_FILE", index)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Deletes the checkout under an injected observer, applies `change`, and
/// expects the unregister to refuse with `IndexChanged`, keeping the
/// registration and its record.
fn assert_index_changed_keeps_the_registration(change: impl FnOnce(&Setup)) {
    let setup = setup();
    let (mut observation, _injector) = injected(&setup);
    let (inventory, approved) = setup.prepare_and_move(&mut observation);
    setup.delete(&mut observation, inventory);
    change(&setup);

    match unregister_quarantine(&setup.repo.path(), &mut observation, &approved) {
        Err(error @ UnregisterError::IndexChanged) => assert_eq!(error.outcome(), RemovalOutcome::ActivityDetected),
        other => panic!("{other:?}"),
    }
    assert!(setup.registered_at_quarantine(), "the registration is kept");
    assert!(setup.plan.admin.dir.is_dir());
}

#[test]
fn an_entry_staged_after_approval_keeps_the_registration() {
    assert_index_changed_keeps_the_registration(|setup| {
        let blob = setup.repo.git(&["hash-object", "-w", ".gitignore"]);
        git_with_index(setup, &setup.plan.admin.index(), &["update-index", "--add", "--cacheinfo", &format!("100644,{blob},late.txt")]);
    });
}

#[test]
fn a_branch_moved_after_approval_keeps_the_registration() {
    assert_index_changed_keeps_the_registration(|setup| {
        let other = setup.repo.commit("other.txt");
        setup.repo.git(&["update-ref", "refs/heads/feat/gone", &other]);
    });
}

/// Deletes the checkout under an injected observer, then `act`s; the
/// unregister must report activity at `location` and keep the registration
/// and the admin watch.
fn assert_activity_keeps_the_registration(act: impl FnOnce(&Setup, &Injector), location: ActivityLocation) {
    let setup = setup();
    let (mut observation, injector) = injected(&setup);
    let (inventory, approved) = setup.prepare_and_move(&mut observation);
    setup.delete(&mut observation, inventory);
    act(&setup, &injector);

    match unregister_quarantine(&setup.repo.path(), &mut observation, &approved) {
        Err(UnregisterError::Activity(list)) => assert!(list.iter().any(|activity| activity.location == location), "{list:?}"),
        other => panic!("{other:?}"),
    }
    assert!(!injector.admin_released(), "the admin watch is kept");
    assert!(setup.registered_at_quarantine(), "the registration is kept");
}

#[test]
fn a_recreated_original_keeps_the_registration() {
    assert_activity_keeps_the_registration(
        |setup, injector| {
            fs::create_dir(&setup.plan.original).unwrap();
            injector.event(Root::Parent, setup.plan.original.file_name().unwrap(), EventKind::Created);
        },
        ActivityLocation::Original,
    );
}

#[test]
fn an_index_write_keeps_the_registration() {
    assert_activity_keeps_the_registration(
        |_, injector| injector.event(Root::Admin, "index.lock", EventKind::Created),
        ActivityLocation::Admin,
    );
}

#[test]
fn an_unreported_recreated_quarantine_keeps_the_registration() {
    assert_activity_keeps_the_registration(|setup, _| fs::create_dir(&setup.plan.quarantine).unwrap(), ActivityLocation::Quarantine);
}

#[test]
fn lost_coverage_keeps_the_registration() {
    let setup = setup();
    let (mut observation, injector) = injected(&setup);
    let (inventory, approved) = setup.prepare_and_move(&mut observation);
    setup.delete(&mut observation, inventory);
    injector.overflow();
    assert!(matches!(
        unregister_quarantine(&setup.repo.path(), &mut observation, &approved),
        Err(UnregisterError::CoverageLost(CoverageLost::Overflow))
    ));
    assert!(setup.registered_at_quarantine());
}
