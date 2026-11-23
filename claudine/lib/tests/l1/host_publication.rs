//! Claudine publishes its `darkmatter-host` revision
//! (`claudine::composition::registration`): every published manifest loads
//! through Darkmatter's real registration loader and names one complete,
//! immutable revision directory, across refreshes, upgrades, races,
//! damaged revisions, and failed writes.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::time::SystemTime;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use claudine::composition::registration::{
    CONFIG_SCHEMA_FILE, GLOBALS_FILE, HostRevision, IDENTIFY_FILE, LIFECYCLE_FILE, MANIFEST_REFERENCE,
    PublishError, PublishOutcome, manifest_path, publish, revisions_dir,
};
use darkmatter::config::load::{ConfigRevision, Registration};
use darkmatter::config::registration::{RegisteredHostProfile, load_entries, md_owned_names};

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

    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(self.root.join("work"), Some(self.home()), HashMap::new())
    }

    /// Loads the user-scope entry `reference` and requires exactly one
    /// `claudine` profile with no diagnostics.
    fn load(&self, reference: &str) -> Arc<RegisteredHostProfile> {
        let entry = Registration {
            reference: reference.to_string(),
            index: 0,
            declared_in: self.home().join(".darkmatter.yaml"),
            revision: ConfigRevision::User,
            at: None,
        };
        let loaded = load_entries(&[entry], &self.context(), md_owned_names());
        assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
        assert_eq!(loaded.profiles.len(), 1, "one profile");
        let profile = Arc::clone(&loaded.profiles[0]);
        assert_eq!(profile.host(), "claudine");
        profile
    }

    fn manifest_text(&self) -> String {
        std::fs::read_to_string(manifest_path(&self.home())).unwrap()
    }

    /// The non-staging revision directory names.
    fn revision_names(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(revisions_dir(&self.home()))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert!(!names.iter().any(|name| name.starts_with('.')), "no staging directory is left: {names:?}");
        names
    }
}

/// Every file under `dir`, by path, with its bytes and modification time.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
    let mut files = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
                files.insert(path.clone(), (std::fs::read(&path).unwrap(), modified));
            }
        }
    }
    files
}

/// Every file `revision` holds is in `dir` with exactly its text.
fn assert_complete(dir: &Path, revision: &HostRevision) {
    for (name, text) in revision.files() {
        let bytes = std::fs::read(dir.join("schemas").join(name)).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(bytes == text.as_bytes(), "{name} holds the revision's text");
    }
}

fn published(outcome: PublishOutcome) -> PathBuf {
    match outcome {
        PublishOutcome::Published { revision_dir, .. } => revision_dir,
        other => panic!("expected a publication: {other:?}"),
    }
}

/// A revision whose content differs from the embedded one and still loads.
fn edited_revision() -> HostRevision {
    let (_, document) = HostRevision::claudine().files().find(|(name, _)| *name == "document.yaml").unwrap();
    let edited = format!("{document}\n# edited\n");
    HostRevision::claudine().clone().with_file("document.yaml", edited)
}

#[test]
fn host_publication_first_publish_loads_as_one_claudine_profile() {
    let home = Home::new();
    let revision = HostRevision::claudine();

    let outcome = publish(&home.home(), revision).unwrap();

    assert_eq!(outcome.manifest(), manifest_path(&home.home()));
    let revision_dir = published(outcome);
    assert_eq!(revision_dir, revisions_dir(&home.home()).join(revision.directory_name()));
    assert_complete(&revision_dir, revision);
    assert_eq!(home.manifest_text(), revision.manifest_text(&revision.directory_name()));
    let profile = home.load(MANIFEST_REFERENCE);
    assert_eq!(profile.version(), revision.version());
    assert_eq!(profile.triggers().len(), 3);
    let manifest = manifest_path(&home.home());
    for dependency in profile.dependencies().iter().filter(|dependency| dependency.path != manifest) {
        assert!(dependency.path.starts_with(&revision_dir), "{} lies in the revision", dependency.path.display());
    }
    assert!(profile.namespaces().iter().all(|member| member.location.starts_with(&revision_dir)));
    assert!(profile.lifecycle_path().starts_with(&revision_dir));
}

#[test]
fn host_publication_refresh_of_a_current_registration_writes_nothing() {
    let home = Home::new();
    let revision = HostRevision::claudine();
    let first = published(publish(&home.home(), revision).unwrap());
    let before = snapshot(&home.home());

    let outcome = publish(&home.home(), revision).unwrap();

    assert_eq!(
        outcome,
        PublishOutcome::Current { manifest: manifest_path(&home.home()), revision_dir: first }
    );
    assert_eq!(snapshot(&home.home()), before, "no file was written");
}

#[test]
fn host_publication_version_and_content_changes_publish_new_revisions() {
    let home = Home::new();
    let original = HostRevision::claudine();
    let original_dir = published(publish(&home.home(), original).unwrap());
    let original_files = snapshot(&original_dir);

    for revision in [original.clone().with_version("0.1.1"), edited_revision()] {
        let revision_dir = published(publish(&home.home(), &revision).unwrap());

        assert_ne!(revision_dir, original_dir);
        assert_complete(&revision_dir, &revision);
        assert_eq!(home.manifest_text(), revision.manifest_text(&revision.directory_name()));
        assert_eq!(home.load(MANIFEST_REFERENCE).version(), revision.version());
        assert_eq!(snapshot(&original_dir), original_files, "the old revision is retained unchanged");
    }
    assert_eq!(home.revision_names().len(), 3);
}

#[test]
fn host_publication_concurrent_publishers_of_one_revision_share_it() {
    let home = Home::new();
    let barrier = Barrier::new(4);
    let revision = HostRevision::claudine();

    let outcomes: Vec<Result<PublishOutcome, PublishError>> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    publish(&home.home(), revision)
                })
            })
            .collect();
        workers.into_iter().map(|worker| worker.join().unwrap()).collect()
    });

    for outcome in outcomes {
        assert_eq!(outcome.unwrap().revision_dir(), revisions_dir(&home.home()).join(revision.directory_name()));
    }
    assert_eq!(home.revision_names(), vec![revision.directory_name()]);
    home.load(MANIFEST_REFERENCE);
}

#[test]
fn host_publication_concurrent_different_revisions_leave_one_complete_one() {
    let home = Home::new();
    let barrier = Barrier::new(2);
    let first = HostRevision::claudine().clone();
    let second = edited_revision().with_version("0.2.0");

    std::thread::scope(|scope| {
        let workers: Vec<_> = [&first, &second]
            .into_iter()
            .map(|revision| {
                let barrier = &barrier;
                let home = &home;
                scope.spawn(move || {
                    barrier.wait();
                    publish(&home.home(), revision).unwrap()
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });

    let text = home.manifest_text();
    let winner = [&first, &second]
        .into_iter()
        .find(|revision| text == revision.manifest_text(&revision.directory_name()))
        .expect("the manifest names one of the two revisions");
    assert_complete(&revisions_dir(&home.home()).join(winner.directory_name()), winner);
    assert_eq!(home.load(MANIFEST_REFERENCE).version(), winner.version());
}

#[test]
fn host_publication_corrupt_revision_is_left_and_a_suffixed_one_published() {
    let home = Home::new();
    let revision = HostRevision::claudine();
    let damaged = published(publish(&home.home(), revision).unwrap());
    let globals = damaged.join("schemas").join(GLOBALS_FILE);
    let mut bytes = std::fs::read(&globals).unwrap();
    bytes[0] ^= 0x20;
    std::fs::write(&globals, bytes).unwrap();
    let damaged_files = snapshot(&damaged);

    let suffixed = published(publish(&home.home(), revision).unwrap());

    assert_eq!(suffixed, revisions_dir(&home.home()).join(format!("{}-2", revision.directory_name())));
    assert_complete(&suffixed, revision);
    assert_eq!(snapshot(&damaged), damaged_files, "the damaged revision is untouched");
    assert_eq!(home.manifest_text(), revision.manifest_text(&format!("{}-2", revision.directory_name())));
    home.load(MANIFEST_REFERENCE);
    let again = publish(&home.home(), revision).unwrap();
    assert!(matches!(again, PublishOutcome::Current { .. }), "a suffixed revision is current: {again:?}");
}

#[test]
fn host_publication_failed_staging_keeps_the_previous_manifest() {
    let home = Home::new();
    publish(&home.home(), HostRevision::claudine()).unwrap();
    let manifest = home.manifest_text();
    let revisions = revisions_dir(&home.home());
    std::fs::remove_dir_all(&revisions).unwrap();
    std::fs::write(&revisions, "not a directory").unwrap();

    let error = publish(&home.home(), &HostRevision::claudine().clone().with_version("0.2.0")).unwrap_err();

    assert!(matches!(error, PublishError::RevisionsDir { .. }), "{error:?}");
    assert_eq!(home.manifest_text(), manifest, "the previous manifest is unchanged");
}

#[test]
fn host_publication_failed_manifest_replacement_leaves_a_complete_revision() {
    let home = Home::new();
    let manifest = manifest_path(&home.home());
    std::fs::create_dir_all(&manifest).unwrap();
    std::fs::write(manifest.join("occupant"), "kept").unwrap();
    let revision = HostRevision::claudine();

    let error = publish(&home.home(), revision).unwrap_err();

    assert!(matches!(error, PublishError::Manifest { .. }), "{error:?}");
    assert_eq!(std::fs::read_to_string(manifest.join("occupant")).unwrap(), "kept", "the old manifest path is unchanged");
    assert_complete(&revisions_dir(&home.home()).join(revision.directory_name()), revision);
}

#[test]
fn host_publication_reader_holding_the_old_manifest_still_loads() {
    let home = Home::new();
    let old = HostRevision::claudine().clone().with_version("0.0.9");
    publish(&home.home(), &old).unwrap();
    let held = home.manifest_text();

    publish(&home.home(), &edited_revision()).unwrap();
    std::fs::write(home.home().join(".claudine").join("held-host.yaml"), held).unwrap();

    assert_eq!(home.load("~/.claudine/held-host.yaml").version(), "0.0.9");
    assert_eq!(home.load(MANIFEST_REFERENCE).version(), HostRevision::claudine().version());
}

#[test]
fn host_publication_embedded_sources_match_the_repository_files() {
    let expected: BTreeMap<&str, &str> = BTreeMap::from([
        ("action.yaml", include_str!("../../../schemas/action.yaml")),
        ("claudine.yaml", include_str!("../../../schemas/claudine.yaml")),
        ("document.yaml", include_str!("../../../schemas/document.yaml")),
        ("inline-compose.yaml", include_str!("../../../schemas/inline-compose.yaml")),
        ("lifecycle.yaml", include_str!("../../../schemas/lifecycle.yaml")),
        ("linking.yaml", include_str!("../../../schemas/linking.yaml")),
        ("sequence-types.yaml", include_str!("../../../schemas/sequence-types.yaml")),
        ("sequence.yaml", include_str!("../../../schemas/sequence.yaml")),
        (GLOBALS_FILE, include_str!("../../../schemas/globals.yaml")),
        (LIFECYCLE_FILE, include_str!("../../../schemas/globals.lifecycle.json")),
        (IDENTIFY_FILE, include_str!("../../../schemas/claudine-document.yaml")),
        (CONFIG_SCHEMA_FILE, include_str!("../../../schemas/claudine-config.yaml")),
    ]);

    let embedded: BTreeMap<&str, &str> = HostRevision::claudine().files().collect();

    assert_eq!(embedded.keys().collect::<Vec<_>>(), expected.keys().collect::<Vec<_>>());
    for (name, text) in expected {
        assert!(embedded[name] == text, "{name} is embedded byte for byte");
    }
}
