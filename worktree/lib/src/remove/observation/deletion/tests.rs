//! The checked deletion engine. The injected-observer tests act at a known
//! batch through the engine's `before_batch` seam, so no test depends on
//! host timing; the `native_*` tests run controlled writers (child processes
//! with readiness handshakes, never sleeps) against this platform's adapter.

use std::cell::Cell;
use std::fs;
use std::io::{BufRead, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use super::*;
use crate::remove::observation::injected::{Injector, Scripted};
use crate::remove::observation::{ObserverOptions, Root, Targets};
use crate::remove::quarantine::rename::rename_no_replace;
use crate::remove::recovery::OperationId;
use crate::remove::recovery::journal::{self, EntryState, JournalLog};
use crate::remove::test_support::link_dir;

/// An in-memory journal that can fail its `n`th append.
#[derive(Clone, Default)]
struct Memory {
    bytes: Arc<Mutex<Vec<u8>>>,
    fail_append: Option<usize>,
    appends: usize,
}

impl JournalSink for Memory {
    fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.appends += 1;
        if self.fail_append == Some(self.appends) {
            return Err(io::Error::other("injected append failure"));
        }
        self.bytes.lock().unwrap().extend_from_slice(bytes);
        Ok(())
    }

    fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn operation() -> OperationId {
    OperationId::parse("0123456789abcdef0123456789abcdef").unwrap()
}

/// A checkout under `<tmp>/parent/checkout`, inventoried, then moved to its
/// quarantine. `outside` is a directory the checkout links to.
struct Fixture {
    _root: tempfile::TempDir,
    original: PathBuf,
    quarantine: PathBuf,
    admin: PathBuf,
    outside: PathBuf,
    inventory: Arc<DeletionInventory>,
}

/// `.git`, `README.md`, `src/{dirty.rs,lib.rs}`, `target/debug/deps/{a.o,b.o}`,
/// and `linked` → `outside`.
fn standard(checkout: &Path, outside: &Path) {
    fs::create_dir_all(checkout.join("src")).unwrap();
    fs::create_dir_all(checkout.join("target").join("debug").join("deps")).unwrap();
    fs::write(checkout.join(".git"), b"gitdir: /repo/.git/worktrees/checkout\n").unwrap();
    fs::write(checkout.join("README.md"), b"readme\n").unwrap();
    fs::write(checkout.join("src").join("dirty.rs"), b"fn main() {}\n").unwrap();
    fs::write(checkout.join("src").join("lib.rs"), b"pub fn f() {}\n").unwrap();
    for name in ["a.o", "b.o"] {
        fs::write(checkout.join("target").join("debug").join("deps").join(name), b"object").unwrap();
    }
    link_dir(outside, &checkout.join("linked"));
}

impl Fixture {
    fn new() -> Self {
        Self::with(standard)
    }

    fn with(build: impl FnOnce(&Path, &Path)) -> Self {
        let root = tempfile::tempdir().unwrap();
        // Resolved, as Git's listing reports paths (`/private/var` on macOS).
        let base = crate::copy_record::canonical_worktree_path(root.path()).unwrap();
        let parent = base.join("parent");
        let original = parent.join("checkout");
        fs::create_dir_all(&original).unwrap();
        let outside = base.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep.txt"), b"never deleted").unwrap();
        let admin = base.join("admin");
        fs::create_dir(&admin).unwrap();
        fs::write(admin.join("index"), b"index").unwrap();
        build(&original, &outside);
        let inventory = Arc::new(DeletionInventory::capture(&original, &[PathBuf::from("src/dirty.rs")]).unwrap());
        let quarantine = parent.join(".wt-remove-checkout-0123456789abcdef");
        Self { _root: root, original, quarantine, admin, outside, inventory }
    }

    fn targets(&self) -> Targets {
        Targets::new(&self.original, &self.quarantine, &self.admin).unwrap()
    }

    fn injected(&self) -> (Observation, Injector) {
        let (mut observation, injector) = Observation::injected(self.targets(), ObserverOptions::default());
        self.move_to_quarantine(&mut observation);
        (observation, injector)
    }

    /// [`Fixture::injected`] whose drains wait for a marker on a virtual
    /// clock; the move's labeling and the first drain are confirmed at once,
    /// later drains only as the test scripts.
    fn confirming(&self) -> (Observation, Injector) {
        let (mut observation, injector) = Observation::injected_confirming(self.targets(), ObserverOptions::default());
        injector.at(Duration::ZERO, Scripted::Acknowledge);
        self.move_to_quarantine(&mut observation);
        injector.at(Duration::ZERO, Scripted::Acknowledge);
        (observation, injector)
    }

    fn move_to_quarantine(&self, observation: &mut Observation) {
        rename_no_replace(&self.original, &self.quarantine).unwrap();
        observation.moved_to_quarantine(&|| false);
    }

    fn journal(&self, sink: Memory) -> JournalWriter<Memory> {
        JournalWriter::start(sink, &operation(), Arc::clone(&self.inventory), journal::limit_for(&self.inventory)).unwrap()
    }

    fn q(&self, relative: &str) -> PathBuf {
        self.quarantine.join(relative)
    }
}

/// A change made to the quarantine's files during deletion.
type Edit = Box<dyn Fn(&Fixture)>;

/// Something the injected observer reports.
type Inject = Box<dyn Fn(&Injector)>;

fn never() -> bool {
    false
}

/// Runs the engine with `hook` before each batch; returns what it did and the
/// journal as recovery would read it.
fn run(
    fixture: &Fixture,
    observation: &mut Observation,
    options: DeletionOptions,
    hook: &mut dyn FnMut(u64),
) -> (Deletion, JournalLog) {
    let sink = Memory::default();
    let mut writer = fixture.journal(sink.clone());
    let deletion = delete_content_with(observation, &mut writer, &never, options, hook);
    let log = journal::parse(&sink.bytes.lock().unwrap(), &operation()).unwrap();
    (deletion, log)
}

fn states(fixture: &Fixture, log: &JournalLog) -> Vec<(String, EntryState)> {
    log.replay(|path| Ok(fs::symlink_metadata(fixture.quarantine.join(path)).is_ok()))
        .unwrap()
        .into_iter()
        .map(|entry| (entry.path.display().into_owned().replace('\\', "/"), entry.state))
        .collect()
}

fn state_of(fixture: &Fixture, log: &JournalLog, path: &str) -> Option<EntryState> {
    states(fixture, log).into_iter().find(|(entry, _)| entry == path).map(|(_, state)| state)
}

/// What a deletion reported changed after the user agreed.
fn warnings(deletion: &Deletion) -> Vec<(ActivityLocation, PathBuf, ActivityKind)> {
    deletion.warnings.iter().map(|activity| (activity.location, activity.path.as_path().to_path_buf(), activity.kind)).collect()
}

/// The deletion finished: no stop, and the quarantine is gone.
fn assert_completed(fixture: &Fixture, deletion: &Deletion, case: &str) {
    assert!(deletion.stop.is_none(), "{case}: {:?}", deletion.stop);
    assert!(!fixture.quarantine.exists(), "{case}: the quarantine is gone");
}

fn lost_watching(deletion: &Deletion) -> bool {
    deletion.warnings.iter().any(|activity| activity.kind == ActivityKind::CoverageLost)
}

/// Every entry was deleted after each of its descendants, and `.git` last.
fn assert_children_first(log: &JournalLog) {
    let order: Vec<PathBuf> = log.batches.iter().flat_map(|batch| batch.entries.iter().map(|entry| entry.path.as_path().to_path_buf())).collect();
    for (at, path) in order.iter().enumerate() {
        let later_child = order[at + 1..].iter().find(|other| other.starts_with(path) && *other != path);
        assert!(later_child.is_none(), "{path:?} was deleted before its descendant {later_child:?}");
    }
    assert_eq!(order.last().map(PathBuf::as_path), Some(Path::new(".git")), "{order:?}");
}

#[test]
fn an_undisturbed_tree_is_deleted_children_first_with_git_last() {
    let fixture = Fixture::new();
    let (mut observation, injector) = fixture.injected();

    let (deletion, log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |_| {});

    assert!(deletion.stop.is_none(), "{:?}", deletion.stop);
    assert_eq!(deletion.deleted.len(), fixture.inventory.entries().len());
    assert!(deletion.externally_absent.is_empty());
    assert!(states(&fixture, &log).iter().all(|(_, state)| *state == EntryState::Deleted), "{:?}", states(&fixture, &log));
    assert_children_first(&log);
    assert!(log.batches.iter().all(|batch| batch.entries.len() <= MAX_BATCH_ENTRIES));
    assert!(!fixture.quarantine.exists() && !fixture.original.exists(), "both locations are gone");
    assert_eq!(fs::read(fixture.outside.join("keep.txt")).unwrap(), b"never deleted", "a link's target is never touched");
    assert!(!injector.admin_released(), "the admin directory stays watched until the unregister");
}

#[test]
fn drains_follow_depth_levels_and_the_bound_not_entries() {
    let fixture = Fixture::with(|checkout, _| {
        fs::write(checkout.join(".git"), b"gitdir: x\n").unwrap();
        let flat = checkout.join("flat");
        fs::create_dir(&flat).unwrap();
        for n in 0..600 {
            fs::write(flat.join(format!("{n:03}.o")), b"").unwrap();
        }
        fs::create_dir_all(checkout.join("a").join("b").join("c").join("d")).unwrap();
        fs::write(checkout.join("a").join("b").join("c").join("d").join("leaf"), b"").unwrap();
    });
    let (mut observation, injector) = fixture.injected();

    let options = DeletionOptions { drain_every: 250, ..DeletionOptions::default() };
    let (deletion, log) = run(&fixture, &mut observation, options, &mut |_| {});

    assert!(deletion.stop.is_none(), "{:?}", deletion.stop);
    // 600 entries in one directory: 3 batches of at most 256, one directory each.
    let flat_batches = log.batches.iter().filter(|batch| batch.entries[0].path.as_path().starts_with("flat")).count();
    assert_eq!(flat_batches, 3);
    // One drain at the start, one per depth level (5 for content, 1 for
    // `.git`), two at the 250-entry bound inside level 2, and one after the
    // quarantine itself is removed; never one per entry or per batch.
    assert_eq!(injector.drains(), 1 + 6 + 2 + 1, "drains");
}

/// `.git` and 600 files in one directory: batches of 256, 256, and 88.
fn flat_600(checkout: &Path, _outside: &Path) {
    fs::write(checkout.join(".git"), b"gitdir: x\n").unwrap();
    let flat = checkout.join("flat");
    fs::create_dir(&flat).unwrap();
    for n in 0..600 {
        fs::write(flat.join(format!("{n:03}.o")), b"").unwrap();
    }
}

/// Deletes `fixture`, whose first drain after the engine's starting one
/// stalls; returns the loss the observer recorded and the deletion, which
/// went on without watching.
fn stall_at_the_second_drain(fixture: &Fixture, options: DeletionOptions) -> (crate::remove::observation::ObservationLoss, Deletion) {
    let (mut observation, _injector) = fixture.confirming();
    let (deletion, _log) = run(fixture, &mut observation, options, &mut |_| {});
    assert_completed(fixture, &deletion, "a stall");
    assert!(lost_watching(&deletion), "{:?}", deletion.warnings);
    (observation.diagnostics().loss.expect("the stall is recorded"), deletion)
}

/// The default interval is 2,048 entries on every platform; the loss a
/// drain receives during deletion carries the entries processed since the
/// previous drain.
#[test]
fn the_default_drain_interval_is_2048_on_every_platform() {
    let fixture = Fixture::with(flat_600);

    let (loss, deletion) = stall_at_the_second_drain(&fixture, DeletionOptions::default());

    assert_eq!(DEFAULT_DRAIN_EVERY, 2048);
    assert_eq!(loss.stage, crate::remove::observation::ObservationStage::Deletion);
    // 600 entries never reach the interval, so only the level's end drains.
    assert_eq!(loss.entries_since_drain, Some(600));
    assert_eq!(deletion.deleted.len(), fixture.inventory.entries().len(), "deletion went on past the stalled drain");
}

/// The interval is checked after each whole batch, so a drain comes at most
/// one batch minus one entry after it.
#[test]
fn a_drain_overshoots_its_interval_by_at_most_one_batch_minus_one_entry() {
    let fixture = Fixture::with(flat_600);
    let drain_every = 257;

    let (loss, _deletion) = stall_at_the_second_drain(&fixture, DeletionOptions { drain_every, ..DeletionOptions::default() });

    // 256 is one short; the drain waits for the second batch, at 512.
    assert_eq!(loss.entries_since_drain, Some((drain_every + MAX_BATCH_ENTRIES - 1) as u64));
}

/// The user agreed to the removal: an approved file that changed after that
/// is deleted as it is now, and reported.
#[test]
fn a_file_changed_without_any_event_is_deleted_and_reported() {
    let cases: Vec<(&str, Edit, ActivityKind)> = vec![
        ("appended", Box::new(|f: &Fixture| fs::write(f.q("README.md"), b"readme\nmore\n").unwrap()), ActivityKind::Modified),
        (
            "same size and time, other bytes, in a hashed file",
            Box::new(|f: &Fixture| {
                let path = f.q("src/dirty.rs");
                let before = fs::metadata(&path).unwrap().modified().unwrap();
                fs::write(&path, b"fn main() {0}").unwrap();
                fs::File::options().write(true).open(&path).unwrap().set_modified(before).unwrap();
            }),
            ActivityKind::Modified,
        ),
        (
            "replaced by a new file",
            // Written beside the original and renamed over it: deleting first
            // lets Linux hand the freed inode straight to the new file, which
            // is then the same `(device, inode)` with other content, so it
            // reads as `Modified` (deleted either way).
            Box::new(|f: &Fixture| {
                fs::write(f.q("README.md.new"), b"readme\n").unwrap();
                fs::rename(f.q("README.md.new"), f.q("README.md")).unwrap();
            }),
            ActivityKind::Replaced,
        ),
    ];
    for (name, change, kind) in cases {
        let fixture = Fixture::new();
        let (mut observation, _injector) = fixture.injected();
        let changed = if name.contains("hashed") { "src/dirty.rs" } else { "README.md" };
        let (deletion, log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
            if batch == 1 {
                change(&fixture);
            }
        });

        assert_completed(&fixture, &deletion, name);
        assert_eq!(warnings(&deletion), vec![(ActivityLocation::Quarantine, fixture.q(changed), kind)], "{name}");
        assert_eq!(state_of(&fixture, &log, changed), Some(EntryState::Deleted), "{name}");
        assert!(deletion.deleted.iter().any(|path| path == Path::new(changed)), "{name}");
    }
}

#[test]
fn a_reported_write_is_deleted_and_reported() {
    let fixture = Fixture::new();
    let (mut observation, injector) = fixture.injected();
    let (deletion, log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
        if batch == 1 {
            fs::write(fixture.q("target/new.o"), b"written during deletion").unwrap();
            injector.event(Root::Tree, "target/new.o", EventKind::Created);
        }
    });
    assert_completed(&fixture, &deletion, "a reported write");
    assert_eq!(warnings(&deletion)[0], (ActivityLocation::Quarantine, fixture.q("target/new.o"), ActivityKind::Created));
    assert_eq!(state_of(&fixture, &log, "target"), Some(EntryState::Deleted), "its folder is approved, deleted with the new file");
}

#[test]
fn an_unreported_new_file_is_deleted_with_its_folder_and_reported() {
    let fixture = Fixture::new();
    let (mut observation, _injector) = fixture.injected();
    let (deletion, log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
        if batch == 2 {
            // Below `deps`, whose entries were all deleted in batch 1.
            fs::write(fixture.q("target/debug/deps/late.o"), b"x").unwrap();
        }
    });
    assert_completed(&fixture, &deletion, "an unreported new file");
    assert_eq!(warnings(&deletion), vec![(ActivityLocation::Quarantine, fixture.q("target/debug/deps"), ActivityKind::Created)]);
    assert_eq!(state_of(&fixture, &log, "target/debug/deps"), Some(EntryState::Deleted));
}

#[test]
fn an_entry_deleted_by_someone_else_is_recorded_absent_and_reported() {
    let fixture = Fixture::new();
    let (mut observation, _injector) = fixture.injected();
    let (deletion, log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
        if batch == 1 {
            fs::remove_file(fixture.q("target/debug/deps/a.o")).unwrap();
        }
    });
    assert_completed(&fixture, &deletion, "an entry deleted elsewhere");
    assert_eq!(warnings(&deletion), vec![(ActivityLocation::Quarantine, fixture.q("target/debug/deps/a.o"), ActivityKind::Vanished)]);
    assert_eq!(deletion.externally_absent, [PathBuf::from("target/debug/deps/a.o")]);
    assert!(deletion.deleted.iter().all(|path| path != Path::new("target/debug/deps/a.o")), "never claimed as this operation's deletion");
    assert_eq!(state_of(&fixture, &log, "target/debug/deps/a.o"), Some(EntryState::ExternallyAbsent));
    assert_eq!(state_of(&fixture, &log, "target/debug/deps/b.o"), Some(EntryState::Deleted));
}

/// A folder replaced by a link is deleted as the link it now is; the link's
/// target, outside the checkout, is never reached.
#[test]
fn a_directory_replaced_by_a_link_is_never_descended() {
    let fixture = Fixture::new();
    let (mut observation, _injector) = fixture.injected();
    let decoy = fixture.outside.join("src");
    fs::create_dir(&decoy).unwrap();
    for name in ["dirty.rs", "lib.rs"] {
        fs::write(decoy.join(name), b"outside the checkout").unwrap();
    }
    let (deletion, _log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
        if batch == 1 {
            fs::rename(fixture.q("src"), fixture.outside.join("moved-src")).unwrap();
            link_dir(&decoy, &fixture.q("src"));
        }
    });
    assert_completed(&fixture, &deletion, "a folder replaced by a link");
    assert_eq!(warnings(&deletion), vec![(ActivityLocation::Quarantine, fixture.q("src"), ActivityKind::Replaced)]);
    for name in ["dirty.rs", "lib.rs"] {
        assert_eq!(fs::read(decoy.join(name)).unwrap(), b"outside the checkout");
    }
    assert!(fixture.outside.join("moved-src").join("lib.rs").is_file(), "what was moved out is outside too");
}

/// New content inside the checkout is deleted with it, through every depth;
/// a link in it is removed as a link.
#[test]
fn new_folders_and_links_inside_are_deleted_and_reported() {
    let fixture = Fixture::new();
    let (mut observation, _injector) = fixture.injected();
    let (deletion, _log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
        if batch == 1 {
            fs::create_dir_all(fixture.q("fresh/deeper")).unwrap();
            fs::write(fixture.q("fresh/deeper/notes.txt"), b"new").unwrap();
            link_dir(&fixture.outside, &fixture.q("fresh/escape"));
        }
    });
    assert_completed(&fixture, &deletion, "new folders");
    assert_eq!(warnings(&deletion), vec![(ActivityLocation::Quarantine, fixture.q("fresh"), ActivityKind::Created)]);
    assert_eq!(fs::read(fixture.outside.join("keep.txt")).unwrap(), b"never deleted", "the link's target is untouched");
}

#[test]
fn recreation_at_the_original_is_reported_and_never_touched() {
    let fixture = Fixture::new();
    let (mut observation, injector) = fixture.injected();
    let (deletion, _log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
        if batch == 1 {
            fs::create_dir_all(fixture.original.join(".gitnexus")).unwrap();
            fs::write(fixture.original.join(".gitnexus").join("index.db"), b"recreated").unwrap();
            injector.event(Root::Parent, "checkout", EventKind::Created);
        }
    });
    assert_completed(&fixture, &deletion, "recreation at the original");
    assert!(warnings(&deletion).contains(&(ActivityLocation::Original, fixture.original.clone(), ActivityKind::Created)), "{:?}", deletion.warnings);
    assert_eq!(fs::read(fixture.original.join(".gitnexus").join("index.db")).unwrap(), b"recreated");
}

#[test]
fn a_recreated_original_found_only_by_the_final_check_is_reported() {
    let fixture = Fixture::new();
    let (mut observation, _injector) = fixture.injected();
    let (deletion, _log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
        if batch == 1 {
            fs::create_dir(&fixture.original).unwrap();
        }
    });
    assert_completed(&fixture, &deletion, "a recreated original");
    assert_eq!(warnings(&deletion), vec![(ActivityLocation::Original, fixture.original.clone(), ActivityKind::Created)]);
    assert!(fixture.original.is_dir());
}

#[test]
fn lost_coverage_and_admin_changes_are_reported_and_deletion_goes_on() {
    let cases: Vec<(&str, Inject)> = vec![
        ("overflow", Box::new(|injector: &Injector| injector.overflow())),
        ("lost parent watch", Box::new(|injector: &Injector| injector.lose_watch(Root::Parent))),
        ("lost admin watch", Box::new(|injector: &Injector| injector.lose_watch(Root::Admin))),
        ("admin index write", Box::new(|injector: &Injector| injector.event(Root::Admin, "index", EventKind::Modified))),
    ];
    for (name, inject) in cases {
        let fixture = Fixture::new();
        let (mut observation, injector) = fixture.injected();
        let (deletion, _log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
            if batch == 1 {
                inject(&injector);
            }
        });
        assert_completed(&fixture, &deletion, name);
        if name.starts_with("admin") {
            assert_eq!(warnings(&deletion), vec![(ActivityLocation::Admin, fixture.admin.join("index"), ActivityKind::Modified)], "{name}");
        } else {
            assert!(lost_watching(&deletion), "{name}: {:?}", deletion.warnings);
            assert_eq!(deletion.warnings.iter().filter(|activity| activity.kind == ActivityKind::CoverageLost).count(), 1, "{name}: once");
        }
    }
}

#[test]
fn events_of_its_own_deletions_are_explained_by_path_kind_and_absence() {
    let fixture = Fixture::new();
    let (mut observation, injector) = fixture.injected();
    let (deletion, _log) = run(&fixture, &mut observation, DeletionOptions { cumulative_kinds: false, ..DeletionOptions::default() }, &mut |batch| {
        if batch == 2 {
            // Batch 1 deleted `deps`' files; Windows also reports the
            // directory as modified, and Linux a link-count change.
            injector.event(Root::Tree, "target/debug/deps/a.o", EventKind::Removed);
            injector.event(Root::Tree, "target/debug/deps/b.o", EventKind::Metadata);
            injector.event(Root::Tree, "target/debug/deps/b.o", EventKind::Removed);
            injector.event(Root::Tree, "target/debug/deps", EventKind::Modified);
            // An approved entry not yet reached, unchanged.
            injector.event(Root::Tree, "README.md", EventKind::Metadata);
            // The operation's own move.
            injector.event(Root::Parent, "checkout", EventKind::MovedFrom);
            injector.event(Root::Parent, ".wt-remove-checkout-0123456789abcdef", EventKind::MovedTo);
        }
    });
    assert!(deletion.stop.is_none(), "{:?}", deletion.stop);
}

#[test]
fn a_write_like_event_on_a_deleted_file_is_uncertain_unless_kinds_are_cumulative() {
    for cumulative_kinds in [false, true] {
        let fixture = Fixture::new();
        let (mut observation, injector) = fixture.injected();
        let options = DeletionOptions { cumulative_kinds, ..DeletionOptions::default() };
        let (deletion, _log) = run(&fixture, &mut observation, options, &mut |batch| {
            if batch == 2 {
                injector.event(Root::Tree, "target/debug/deps/a.o", EventKind::Modified);
                injector.event(Root::Tree, "target/debug/deps/a.o", EventKind::Removed);
            }
        });
        assert_completed(&fixture, &deletion, "a write-like event");
        if cumulative_kinds {
            // FSEvents reports history: `Modified` beside `Removed` is the
            // same deletion.
            assert!(deletion.warnings.is_empty(), "{:?}", deletion.warnings);
        } else {
            assert_eq!(warnings(&deletion), vec![(ActivityLocation::Quarantine, fixture.q("target/debug/deps/a.o"), ActivityKind::Modified)]);
        }
    }
}

#[test]
fn an_interrupt_stops_at_the_next_entry_boundary() {
    let fixture = Fixture::new();
    let (mut observation, _injector) = fixture.injected();
    let sink = Memory::default();
    let mut writer = fixture.journal(sink.clone());
    // The batch check, then a.o's check pass; b.o's check sees the interrupt.
    let polls = Cell::new(0);
    let interrupted = || {
        polls.set(polls.get() + 1);
        polls.get() > 2
    };
    let deletion = delete_content(&mut observation, &mut writer, &interrupted, DeletionOptions::default());
    assert!(matches!(deletion.stop, Some(Stop::Interrupted)), "{:?}", deletion.stop);
    assert_eq!(deletion.deleted, [PathBuf::from("target/debug/deps/a.o")]);
    let log = journal::parse(&sink.bytes.lock().unwrap(), &operation()).unwrap();
    assert_eq!(state_of(&fixture, &log, "target/debug/deps/b.o"), Some(EntryState::Kept), "the batch's results are durable");
    assert!(fixture.q("target/debug/deps/b.o").is_file());
}

/// The interrupt ends a drain's wait for delivery at once, not at the stall
/// limit, and nothing after that drain is deleted.
#[test]
fn an_interrupt_ends_a_drain_wait_and_nothing_more_is_deleted() {
    let fixture = Fixture::new();
    let (mut observation, injector) = fixture.confirming();
    injector.at(Duration::from_secs(5), Scripted::Interrupt);
    let sink = Memory::default();
    let mut writer = fixture.journal(sink.clone());

    let deletion = delete_content(&mut observation, &mut writer, &|| injector.interrupted(), DeletionOptions::default());

    assert!(matches!(deletion.stop, Some(Stop::Interrupted)), "{:?}", deletion.stop);
    assert!(!deletion.deleted.is_empty(), "the first level was deleted before the drain that waited");
    assert!(injector.elapsed() < Duration::from_secs(6), "ended at the interrupt, not a limit: {:?}", injector.elapsed());
    assert!(fixture.q("README.md").is_file() && fixture.q(".git").is_file(), "later levels are kept");
}

/// A drain whose delivery stalls is lost coverage: reported once, and
/// deletion goes on without waiting again.
#[test]
fn a_stalled_drain_is_reported_and_deletion_goes_on() {
    let fixture = Fixture::new();
    let (mut observation, injector) = fixture.confirming();

    let (deletion, _log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |_| {});

    assert_completed(&fixture, &deletion, "a stall");
    assert!(lost_watching(&deletion), "{:?}", deletion.warnings);
    match observation.diagnostics().loss.map(|loss| loss.reason) {
        Some(crate::remove::observation::CoverageLost::DeliveryStalled { waited, watch: None }) => assert_eq!(waited, Duration::from_secs(30)),
        other => panic!("expected a stall, got {other:?}"),
    }
    assert_eq!(injector.elapsed(), Duration::from_secs(30), "one wait, never another after the loss");
}

#[test]
fn a_journal_that_cannot_record_an_intent_stops_before_its_batch() {
    let fixture = Fixture::new();
    let (mut observation, _injector) = fixture.injected();
    // Append 1 is the header and inventory, 2 batch 1's intent, 3 its
    // results, 4 batch 2's intent.
    let sink = Memory { fail_append: Some(4), ..Memory::default() };
    let mut writer = fixture.journal(sink.clone());
    let deletion = delete_content(&mut observation, &mut writer, &never, DeletionOptions::default());
    assert!(matches!(deletion.stop, Some(Stop::Journal(_))), "{:?}", deletion.stop);
    assert_eq!(deletion.deleted.len(), 2, "only batch 1, whose intent and results are durable");
    assert!(fixture.q("target/debug").is_dir(), "batch 2 never started");
}

#[test]
fn the_report_names_deleted_entries_and_states_the_concurrent_write_limit() {
    let fixture = Fixture::new();
    let (mut observation, _injector) = fixture.injected();
    let (deletion, _log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
        if batch == 2 {
            fs::write(fixture.q("README.md"), b"changed").unwrap();
        }
    });
    assert!(deletion.stop.is_none(), "{:?}", deletion.stop);
    let mut report = OperationReport::new(Some(operation()), RemovalOutcome::Removed, NativeString::from(fixture.original.as_path()));
    report.record_deletion(&fixture.quarantine, &deletion);
    assert_eq!(report.notes, [ReportNote::ConcurrentWriteLimit]);
    assert_eq!(
        report.deleted.iter().map(|path| path.as_path().to_path_buf()).collect::<Vec<_>>(),
        deletion.deleted.iter().map(|path| fixture.quarantine.join(path)).collect::<Vec<_>>()
    );
    assert_eq!(report.activity.len(), 1);
    assert_eq!(report.activity[0].path.as_path(), fixture.q("README.md"));

    let untouched = Deletion { deleted: Vec::new(), externally_absent: Vec::new(), warnings: Vec::new(), stop: Some(Stop::Interrupted) };
    let mut report = OperationReport::new(None, RemovalOutcome::CleanupFailed, NativeString::from(fixture.original.as_path()));
    report.record_deletion(&fixture.quarantine, &untouched);
    assert!(report.notes.is_empty(), "nothing deleted, nothing to qualify");
}

// ---- native adapters and controlled writers ----

/// A child process that does one thing when told to and says when it has.
struct Writer {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Writer {
    /// Runs `script` (sh on Unix, cmd on Windows) in `cwd`. The script reads
    /// one line, acts, prints `done`, then waits for its input to close.
    fn spawn(cwd: &Path, script: &str) -> Self {
        #[cfg(unix)]
        let mut command = {
            let mut command = Command::new("sh");
            command.arg("-c").arg(format!("read go; {script}; echo done; read end"));
            command
        };
        #[cfg(windows)]
        let mut command = {
            use std::os::windows::process::CommandExt;
            let mut command = Command::new("cmd");
            command.arg("/D").arg("/Q").arg("/C").raw_arg(format!("\"set /p go= & {script} & echo done& set /p end=\""));
            command
        };
        let mut child = command.current_dir(cwd).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self { child, stdin, stdout }
    }

    /// Tells the writer to act and waits until it has.
    fn act(&mut self) {
        self.stdin.write_all(b"go\n").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        assert_eq!(line.trim(), "done", "the writer acted");
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn native(fixture: &Fixture, options: ObserverOptions) -> Observation {
    let mut observation = Observation::start(fixture.targets(), options, &|| false).expect("observation starts");
    fixture.move_to_quarantine(&mut observation);
    observation
}

#[test]
fn native_a_tree_is_deleted_with_its_own_events_explained() {
    let fixture = Fixture::new();
    let mut observation = native(&fixture, ObserverOptions::default());
    let (deletion, log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |_| {});
    assert!(deletion.stop.is_none(), "{:?}", deletion.stop);
    assert_children_first(&log);
    assert!(!fixture.quarantine.exists() && !fixture.original.exists());
    assert_eq!(fs::read(fixture.outside.join("keep.txt")).unwrap(), b"never deleted");
}

/// A process whose working directory is inside the checkout keeps writing
/// into the quarantine after the move (on Windows that directory blocks the
/// move, so the writer there uses the quarantine's path).
#[test]
fn native_a_writer_in_the_moved_checkout_is_reported_and_its_file_deleted() {
    let fixture = Fixture::new();
    #[cfg(unix)]
    let mut writer = Writer::spawn(&fixture.original.join("target"), "echo data > written.txt");
    let mut observation = native(&fixture, ObserverOptions::default());
    #[cfg(windows)]
    let mut writer = Writer::spawn(&fixture.q("target"), "echo data> written.txt");

    let (deletion, log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
        if batch == 1 {
            writer.act();
        }
    });

    let found = warnings(&deletion);
    assert!(
        found.iter().any(|(location, path, _)| *location == ActivityLocation::Quarantine && path.starts_with(fixture.q("target"))),
        "{found:?}"
    );
    assert_completed(&fixture, &deletion, "a writer in the moved checkout");
    assert_eq!(state_of(&fixture, &log, "target"), Some(EntryState::Deleted));
}

#[test]
fn native_recreation_at_the_original_path_is_reported_and_kept() {
    let fixture = Fixture::new();
    let parent = fixture.original.parent().unwrap().to_path_buf();
    #[cfg(unix)]
    let script = "mkdir checkout && echo data > checkout/index.db";
    #[cfg(windows)]
    let script = "mkdir checkout && echo data> checkout\\index.db";
    let mut writer = Writer::spawn(&parent, script);
    let mut observation = native(&fixture, ObserverOptions::default());

    let (deletion, _log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
        if batch == 1 {
            writer.act();
        }
    });

    let found = warnings(&deletion);
    assert!(found.iter().any(|(location, _, _)| *location == ActivityLocation::Original), "{found:?}");
    assert!(fs::read_to_string(fixture.original.join("index.db")).unwrap().starts_with("data"), "the recreated tree is untouched");
    assert_completed(&fixture, &deletion, "recreation at the original");
}

#[test]
fn native_a_full_queue_is_reported_and_deletion_goes_on() {
    let fixture = Fixture::with(|checkout, _| {
        fs::write(checkout.join(".git"), b"gitdir: x\n").unwrap();
        fs::create_dir(checkout.join("flat")).unwrap();
        for n in 0..200 {
            fs::write(checkout.join("flat").join(n.to_string()), b"").unwrap();
        }
    });
    let mut observation = native(&fixture, ObserverOptions { queue_capacity: 8, ..ObserverOptions::default() });
    let options = DeletionOptions { drain_every: usize::MAX, ..DeletionOptions::default() };
    let (deletion, _log) = run(&fixture, &mut observation, options, &mut |_| {});
    assert_completed(&fixture, &deletion, "a full queue");
    assert!(lost_watching(&deletion), "{:?}", deletion.warnings);
    assert!(matches!(
        observation.diagnostics().loss.map(|loss| loss.reason),
        Some(crate::remove::observation::CoverageLost::QueueFull { capacity: 8 })
    ));
}

#[test]
fn received_times_are_recorded_for_reported_activity() {
    let fixture = Fixture::new();
    let (mut observation, injector) = fixture.injected();
    let before = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
    let (deletion, _log) = run(&fixture, &mut observation, DeletionOptions::default(), &mut |batch| {
        if batch == 1 {
            fs::write(fixture.q("new.txt"), b"x").unwrap();
            injector.event(Root::Tree, "new.txt", EventKind::Created);
        }
    });
    assert!(deletion.warnings[0].received_at_ms >= before, "{:?}", deletion.warnings);
}
