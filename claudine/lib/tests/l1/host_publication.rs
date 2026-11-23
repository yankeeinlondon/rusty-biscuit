//! Claudine publishes its format 2 `darkmatter-host` registration at fixed
//! paths (`claudine::composition::registration`): every publish is judged by
//! Darkmatter's own verifying reader (`bundle::load_snapshot`) and its
//! registration loader, across no-op republishes, interruptions, cooperating
//! publishers, concurrent readers, leftover temporary files, retired members,
//! and links that obstruct the output tree.

use std::collections::{BTreeMap, HashMap};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Barrier;
use std::time::{Duration, SystemTime};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use claudine::composition::registration::{
    FileKind, MANIFEST_FILE, MANIFEST_REFERENCE, PublishError, PublishHooks, PublishOutcome, PublishStep, PublishedFile,
    RegistrationInventory, TEMP_PREFIX, lock_path, manifest_path, output_root, publish, publish_with,
};
use darkmatter::config::load::{ConfigRevision, Registration};
use darkmatter::config::registration::bundle::{SnapshotRefusal, VerifiedSnapshot, load_snapshot};
use darkmatter::config::registration::{HOST_REGISTRATION_CODE, load_entries, md_owned_names};

/// A temporary home and working directory; nothing is read from the process.
struct Home {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Home {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_simplified(dir.path()).unwrap();
        std::fs::create_dir_all(root.join("home")).unwrap();
        std::fs::create_dir_all(root.join("work")).unwrap();
        Self { _dir: dir, root }
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn output(&self) -> PathBuf {
        output_root(&self.home())
    }

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.root.join("work"), Some(self.home()), HashMap::new())
    }

    /// The installed bundle as Darkmatter's verifying reader sees it.
    fn read(&self) -> Result<VerifiedSnapshot, Box<SnapshotRefusal>> {
        load_snapshot(&manifest_path(&self.home()), &self.context())
    }

    /// Requires the `register` entry [`MANIFEST_REFERENCE`] to load as one
    /// `claudine` profile of `inventory`'s version with no diagnostics.
    fn assert_loads(&self, inventory: &RegistrationInventory) {
        let entry = Registration {
            reference: MANIFEST_REFERENCE.to_string(),
            index: 0,
            declared_in: self.home().join(".darkmatter.yaml"),
            revision: ConfigRevision::User,
            at: None,
        };
        let loaded = load_entries(&[entry], &self.context(), md_owned_names());
        assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
        assert_eq!(loaded.profiles.len(), 1, "one profile");
        assert_eq!(loaded.profiles[0].host(), "claudine");
        assert_eq!(loaded.profiles[0].version(), inventory.version());
        assert_release(&self.read().expect("the installed bundle verifies"), inventory);
    }

    fn manifest_bytes(&self) -> Vec<u8> {
        std::fs::read(manifest_path(&self.home())).unwrap()
    }
}

/// Every file under `dir`, by path, with its bytes and modification time.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
    let mut files = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&next) else { continue };
        for entry in entries {
            let path = entry.unwrap().path();
            let metadata = std::fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() {
                files.insert(path.clone(), (std::fs::read(&path).unwrap(), metadata.modified().unwrap()));
            }
        }
    }
    files
}

/// `snapshot` verified exactly `inventory`'s members, byte for byte, and no
/// other file.
fn assert_release(snapshot: &VerifiedSnapshot, inventory: &RegistrationInventory) {
    assert!(is_release(snapshot, inventory), "the snapshot is the release of {}", inventory.version());
}

fn is_release(snapshot: &VerifiedSnapshot, inventory: &RegistrationInventory) -> bool {
    snapshot.version() == inventory.version()
        && snapshot.bytes().len() == inventory.members().count()
        && inventory.members().all(|file| snapshot.member_bytes(file.path()).is_some_and(|bytes| **bytes == *file.bytes()))
}

/// A refusal is Darkmatter's located registration warning.
fn assert_refused(refusal: &SnapshotRefusal) {
    assert_eq!(refusal.code, HOST_REGISTRATION_CODE, "{refusal:#?}");
    assert!(refusal.line.is_some(), "the refusal is located: {refusal:#?}");
}

/// An earlier release: an edited `shared.yaml` (still valid) under another
/// version.
fn earlier() -> RegistrationInventory {
    let shared = RegistrationInventory::claudine().file("schemas/shared.yaml").unwrap();
    let edited = format!("{}\n# an earlier release\n", shared.text().unwrap());
    RegistrationInventory::claudine()
        .clone()
        .with_version("0.0.9")
        .with_member(PublishedFile::new("schemas/shared.yaml", shared.kind(), None::<&'static str>, edited.into_bytes()))
}

/// A types-only library `path` that nothing imports.
fn library(path: &'static str) -> PublishedFile {
    PublishedFile::new(path, FileKind::Library, None::<&'static str>, b"kind: schema\ntypes:\n    retired: string\n".to_vec())
}

fn changes(outcome: &PublishOutcome) -> &claudine::composition::registration::PublishChanges {
    outcome.changes().unwrap_or_else(|| panic!("expected a publication: {outcome:?}"))
}

/// Publishes `inventory`, stopping with an interruption at the first step
/// `stop_at` accepts.
fn publish_interrupted(home: &Path, inventory: &RegistrationInventory, stop_at: fn(&PublishStep) -> bool) -> PublishError {
    let mut observer = |step: &PublishStep| if stop_at(step) { ControlFlow::Break(()) } else { ControlFlow::Continue(()) };
    publish_with(home, inventory, PublishHooks { lock_deadline: Duration::from_secs(5), observer: &mut observer })
        .expect_err("the observer stops the publish")
}

#[test]
fn host_publication_fresh_publish_loads_as_a_format_2_host_with_no_warnings() {
    let home = Home::new();
    let inventory = RegistrationInventory::claudine();

    let outcome = publish(&home.home(), inventory).unwrap();

    assert_eq!(outcome.manifest(), manifest_path(&home.home()));
    let written = &changes(&outcome).written;
    assert_eq!(written.len(), inventory.files().len(), "every file is written once");
    assert_eq!(written.last().map(String::as_str), Some(MANIFEST_FILE), "the manifest is written last");
    for file in inventory.files() {
        let installed = std::fs::read(home.output().join(file.path())).unwrap();
        assert!(installed == file.bytes(), "{} is installed byte for byte", file.path());
    }
    let snapshot = home.read().expect("the bundle verifies");
    assert_eq!(snapshot.host(), "claudine");
    let schemas: Vec<&str> =
        inventory.members().filter(|file| file.kind() != FileKind::Lifecycle).map(PublishedFile::path).collect();
    assert_eq!(snapshot.sorted_files(), schemas);
    assert_eq!(snapshot.lifecycle().path, "globals.lifecycle.json");
    home.assert_loads(inventory);
}

#[test]
fn host_publication_identical_publish_writes_nothing_and_keeps_every_mtime() {
    let home = Home::new();
    let inventory = RegistrationInventory::claudine();
    publish(&home.home(), inventory).unwrap();
    let before = snapshot(&home.home());

    let outcome = publish(&home.home(), inventory).unwrap();

    assert_eq!(outcome, PublishOutcome::Current { manifest: manifest_path(&home.home()) });
    assert_eq!(snapshot(&home.home()), before, "no byte or timestamp changed");
}

#[test]
fn host_publication_changed_member_replaces_only_that_member_and_the_manifest() {
    let home = Home::new();
    publish(&home.home(), &earlier()).unwrap();
    let before = snapshot(&home.output());

    let outcome = publish(&home.home(), RegistrationInventory::claudine()).unwrap();

    assert_eq!(changes(&outcome).written, ["schemas/shared.yaml", MANIFEST_FILE]);
    let after = snapshot(&home.output());
    for (path, entry) in &before {
        if !path.ends_with("shared.yaml") && !path.ends_with(MANIFEST_FILE) {
            assert_eq!(after.get(path), Some(entry), "{} is untouched", path.display());
        }
    }
    home.assert_loads(RegistrationInventory::claudine());
}

#[test]
fn host_publication_interrupted_member_replacement_is_refused_then_converges() {
    let home = Home::new();
    let previous = earlier();
    publish(&home.home(), &previous).unwrap();
    let manifest = home.manifest_bytes();

    let error = publish_interrupted(&home.home(), RegistrationInventory::claudine(), |step| {
        matches!(step, PublishStep::MemberWritten(_))
    });

    assert!(matches!(error, PublishError::Interrupted { .. }), "{error:?}");
    assert_eq!(home.manifest_bytes(), manifest, "the previous manifest stands");
    let refusal = home.read().expect_err("a replaced member no longer matches the previous manifest");
    assert_refused(&refusal);
    assert!(refusal.member.as_deref().is_some_and(|member| member.starts_with("files[")), "{refusal:#?}");

    let outcome = publish(&home.home(), RegistrationInventory::claudine()).unwrap();
    assert!(changes(&outcome).manifest_written());
    home.assert_loads(RegistrationInventory::claudine());
    assert!(publish(&home.home(), RegistrationInventory::claudine()).unwrap().changes().is_none(), "converged");
}

#[test]
fn host_publication_interrupted_before_the_manifest_commit_is_refused_then_converges() {
    let home = Home::new();
    let previous = earlier();
    publish(&home.home(), &previous).unwrap();
    let manifest = home.manifest_bytes();

    let error = publish_interrupted(&home.home(), RegistrationInventory::claudine(), |step| {
        matches!(step, PublishStep::BeforeManifest)
    });

    assert!(matches!(error, PublishError::Interrupted { .. }), "{error:?}");
    assert_eq!(home.manifest_bytes(), manifest, "the previous manifest stands");
    assert_refused(&home.read().expect_err("every member is new and the manifest is old"));

    let outcome = publish(&home.home(), RegistrationInventory::claudine()).unwrap();
    assert_eq!(changes(&outcome).written, [MANIFEST_FILE], "only the manifest was left to write");
    home.assert_loads(RegistrationInventory::claudine());
}

#[test]
fn host_publication_readers_during_publication_see_one_release_or_a_refusal() {
    let home = Home::new();
    let previous = earlier().with_member(library("schemas/retired.yaml"));
    publish(&home.home(), &previous).unwrap();
    let next = RegistrationInventory::claudine();
    let mut seen = Vec::new();

    let mut observer = |step: &PublishStep| {
        match home.read() {
            Ok(snapshot) => {
                assert!(
                    is_release(&snapshot, &previous) || is_release(&snapshot, next),
                    "at {step:?} the reader verified a mix of releases"
                );
                seen.push(format!("{step:?}: {}", snapshot.version()));
            }
            Err(refusal) => {
                assert_refused(&refusal);
                seen.push(format!("{step:?}: refused"));
            }
        }
        ControlFlow::Continue(())
    };
    publish_with(&home.home(), next, PublishHooks { lock_deadline: Duration::from_secs(5), observer: &mut observer })
        .unwrap();

    assert!(seen[0].ends_with("0.0.9"), "the locked publisher has changed nothing yet: {seen:?}");
    assert!(seen.iter().any(|step| step.ends_with("refused")), "{seen:?}");
    assert!(seen.iter().any(|step| step.starts_with("Retired")), "{seen:?}");
    home.assert_loads(next);
}

#[test]
fn host_publication_cooperating_publishers_serialize_and_the_result_verifies() {
    let previous = earlier();
    let next = RegistrationInventory::claudine();
    for pair in [[next, next], [&previous, next]] {
        let home = Home::new();
        let barrier = Barrier::new(2);

        let outcomes: Vec<Result<PublishOutcome, PublishError>> = std::thread::scope(|scope| {
            let workers: Vec<_> = pair
                .iter()
                .map(|inventory| {
                    let (barrier, home) = (&barrier, &home);
                    scope.spawn(move || {
                        barrier.wait();
                        publish(&home.home(), inventory)
                    })
                })
                .collect();
            workers.into_iter().map(|worker| worker.join().unwrap()).collect()
        });

        for outcome in &outcomes {
            assert!(outcome.is_ok(), "{outcome:?}");
        }
        let snapshot = home.read().expect("the result verifies");
        let winner = pair.into_iter().find(|inventory| is_release(&snapshot, inventory)).expect("one whole release");
        home.assert_loads(winner);
        assert!(std::fs::read_dir(home.output().join("schemas")).unwrap().all(|entry| {
            !entry.unwrap().file_name().to_string_lossy().starts_with(TEMP_PREFIX)
        }));
    }
}

#[test]
fn host_publication_lock_held_by_another_publisher_times_out_and_writes_nothing() {
    let home = Home::new();
    let lock = lock_path(&home.home());
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    let holder = std::fs::File::create(&lock).unwrap();
    holder.lock().unwrap();

    let mut observer = |_: &PublishStep| ControlFlow::Continue(());
    let error = publish_with(
        &home.home(),
        RegistrationInventory::claudine(),
        PublishHooks { lock_deadline: Duration::from_millis(50), observer: &mut observer },
    )
    .unwrap_err();

    assert!(matches!(error, PublishError::LockTimeout { .. }), "{error:?}");
    assert!(!home.output().exists(), "nothing was written");
    drop(holder);
    publish(&home.home(), RegistrationInventory::claudine()).unwrap();
}

#[test]
fn host_publication_leftover_temps_are_removed_and_unknown_files_kept() {
    let home = Home::new();
    let inventory = RegistrationInventory::claudine();
    publish(&home.home(), inventory).unwrap();
    let root_temp = home.output().join(format!("{TEMP_PREFIX}root"));
    let member_temp = home.output().join(format!("schemas/{TEMP_PREFIX}member"));
    let temp_named_dir = home.output().join(format!("schemas/{TEMP_PREFIX}folder"));
    let unknown = [home.output().join("notes.txt"), home.output().join("schemas/mine.yaml"), temp_named_dir.join("kept")];
    for path in [&root_temp, &member_temp].into_iter().chain(&unknown) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "leftover").unwrap();
    }
    let manifest = snapshot(&home.output()).remove(&manifest_path(&home.home())).unwrap();

    let outcome = publish(&home.home(), inventory).unwrap();

    let changes = changes(&outcome);
    let mut removed = changes.removed_temps.clone();
    removed.sort();
    assert_eq!(removed, [root_temp.clone(), member_temp.clone()]);
    assert!(changes.written.is_empty() && changes.retired.is_empty(), "{changes:?}");
    assert!(!root_temp.exists() && !member_temp.exists());
    for path in &unknown {
        assert_eq!(std::fs::read_to_string(path).unwrap(), "leftover", "{} is kept", path.display());
    }
    assert_eq!(snapshot(&home.output()).remove(&manifest_path(&home.home())).unwrap(), manifest);
    home.assert_loads(inventory);
}

#[test]
fn host_publication_retired_members_from_the_previous_manifest_are_removed() {
    let home = Home::new();
    let previous = RegistrationInventory::claudine()
        .clone()
        .with_member(library("schemas/retired.yaml"))
        .with_member(library("schemas/old/nested.yaml"));
    publish(&home.home(), &previous).unwrap();
    home.assert_loads(&previous);

    let outcome = publish(&home.home(), RegistrationInventory::claudine()).unwrap();

    let mut retired = changes(&outcome).retired.clone();
    retired.sort();
    assert_eq!(retired, ["schemas/old/nested.yaml", "schemas/retired.yaml"]);
    assert!(!home.output().join("schemas/retired.yaml").exists());
    assert!(!home.output().join("schemas/old").exists(), "the emptied folder is removed");
    home.assert_loads(RegistrationInventory::claudine());
}

#[test]
fn host_publication_removes_nothing_an_invalid_or_escaping_manifest_names() {
    let home = Home::new();
    let inventory = RegistrationInventory::claudine();
    publish(&home.home(), inventory).unwrap();
    let outside = home.home().join(".claudine/registrations/outside.yaml");
    let stale = home.output().join("schemas/stale.yaml");
    for path in [&outside, &stale] {
        std::fs::write(path, "kept").unwrap();
    }
    let manifest = manifest_path(&home.home());
    let text = String::from_utf8(home.manifest_bytes()).unwrap();
    let escaping = text
        .replace("files:\n", "files:\n    - \"./../outside.yaml\"\n")
        .replace("digests:\n", &format!("digests:\n    \"./../outside.yaml\": \"{}\"\n", "0".repeat(64)));

    for previous in [escaping, text.replace("format_version: 2", "format_version: 1").replace("files:\n", "files:\n    - ./schemas/stale.yaml\n")] {
        std::fs::write(&manifest, previous).unwrap();

        let outcome = publish(&home.home(), inventory).unwrap();

        assert_eq!(changes(&outcome).written, [MANIFEST_FILE]);
        assert!(changes(&outcome).retired.is_empty(), "{outcome:?}");
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "kept");
        assert_eq!(std::fs::read_to_string(&stale).unwrap(), "kept");
    }
    home.assert_loads(inventory);
}

#[test]
fn host_publication_misplaced_entries_stop_before_any_write() {
    let home = Home::new();
    publish(&home.home(), &earlier()).unwrap();
    let action = home.output().join("schemas/action.yaml");
    std::fs::remove_file(&action).unwrap();
    std::fs::create_dir(&action).unwrap();
    let before = snapshot(&home.home());

    let error = publish(&home.home(), RegistrationInventory::claudine()).unwrap_err();

    assert!(matches!(&error, PublishError::NotAFile { path } if *path == action), "{error:?}");
    assert_eq!(snapshot(&home.home()), before, "the preflight wrote nothing");
    std::fs::remove_dir(&action).unwrap();
    publish(&home.home(), RegistrationInventory::claudine()).unwrap();
    home.assert_loads(RegistrationInventory::claudine());
}

/// Links that obstruct the output tree: each is refused before anything is
/// written, and the out-of-tree sentinel it points at is untouched.
#[cfg(unix)]
mod obstructions {
    use std::os::unix::fs::symlink;

    use super::*;

    struct Sentinel {
        _dir: tempfile::TempDir,
        dir: PathBuf,
        file: PathBuf,
    }

    impl Sentinel {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let path = canonicalize_simplified(dir.path()).unwrap();
            let file = path.join("sentinel.yaml");
            std::fs::write(&file, "sentinel").unwrap();
            Self { dir: path, file, _dir: dir }
        }

        fn assert_untouched(&self) {
            assert_eq!(std::fs::read_to_string(&self.file).unwrap(), "sentinel");
            let names: Vec<_> = std::fs::read_dir(&self.dir).unwrap().map(|entry| entry.unwrap().file_name()).collect();
            assert_eq!(names, ["sentinel.yaml"], "nothing was written through the link");
        }
    }

    /// Publishes `earlier()` when `prepublish`, links `link` to `target`,
    /// then requires the publish of Claudine's inventory to be refused as an
    /// obstruction at `link` with nothing under the home changed.
    fn assert_obstructed(prepublish: bool, link: &Path, target: &Path, sentinel: &Sentinel, home: &Home) {
        if prepublish {
            publish(&home.home(), &earlier()).unwrap();
        }
        if link.exists() {
            std::fs::remove_file(link).unwrap();
        }
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink(target, link).unwrap();
        let before = snapshot(&home.home());

        let error = publish(&home.home(), RegistrationInventory::claudine()).unwrap_err();

        assert!(matches!(&error, PublishError::Obstructed { path, .. } if path == link), "{error:?}");
        assert_eq!(snapshot(&home.home()), before, "nothing under the home changed");
        assert_eq!(std::fs::read_link(link).unwrap(), target, "the link is left alone");
        sentinel.assert_untouched();
    }

    #[test]
    fn host_publication_symlinked_output_root_is_refused() {
        let (home, sentinel) = (Home::new(), Sentinel::new());
        assert_obstructed(false, &home.output(), &sentinel.dir, &sentinel, &home);
    }

    #[test]
    fn host_publication_symlinked_path_component_is_refused() {
        let (home, sentinel) = (Home::new(), Sentinel::new());
        assert_obstructed(false, &home.home().join(".claudine/registrations"), &sentinel.dir, &sentinel, &home);
    }

    #[test]
    fn host_publication_symlinked_member_destination_is_refused() {
        let (home, sentinel) = (Home::new(), Sentinel::new());
        let member = home.output().join("schemas/shared.yaml");
        assert_obstructed(true, &member, &sentinel.file, &sentinel, &home);
    }

    #[test]
    fn host_publication_symlinked_lock_file_is_refused() {
        let (home, sentinel) = (Home::new(), Sentinel::new());
        assert_obstructed(false, &lock_path(&home.home()), &sentinel.file, &sentinel, &home);
    }

    #[test]
    fn host_publication_symlinked_retired_entry_is_refused() {
        let (home, sentinel) = (Home::new(), Sentinel::new());
        publish(&home.home(), &RegistrationInventory::claudine().clone().with_member(library("schemas/retired.yaml")))
            .unwrap();
        let retired = home.output().join("schemas/retired.yaml");
        assert_obstructed(false, &retired, &sentinel.file, &sentinel, &home);
    }

    #[test]
    fn host_publication_symlinked_manifest_is_refused() {
        let (home, sentinel) = (Home::new(), Sentinel::new());
        assert_obstructed(true, &manifest_path(&home.home()), &sentinel.file, &sentinel, &home);
    }
}
