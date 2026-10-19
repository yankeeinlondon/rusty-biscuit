//! The deletion inventory against real files: what it records, what it
//! refuses, and what its finishing check catches.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::*;
use crate::remove::test_support::link_dir;
use crate::remove::observation::{EventKind, Observation, ObserverOptions, Root, Targets};

/// `<root>/parent/checkout` holding a tracked-looking file, a dirty file, an
/// ignored build tree, a nested directory, and a `.git` file; plus a
/// directory outside the checkout that a link points to.
struct Checkout {
    _root: tempfile::TempDir,
    parent: PathBuf,
    path: PathBuf,
    outside: PathBuf,
}

impl Checkout {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join("parent");
        let path = parent.join("checkout");
        fs::create_dir_all(path.join("src")).unwrap();
        fs::create_dir_all(path.join("target").join("debug").join("deps")).unwrap();
        fs::write(path.join(".git"), b"gitdir: /repo/.git/worktrees/checkout\n").unwrap();
        fs::write(path.join("README.md"), b"readme\n").unwrap();
        fs::write(path.join("src").join("dirty.rs"), b"fn main() {}\n").unwrap();
        fs::write(path.join("target").join("debug").join("deps").join("lib.rlib"), b"ignored build output").unwrap();
        let outside = root.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep.txt"), b"not part of the checkout").unwrap();
        Self { _root: root, parent, path, outside }
    }

    fn sensitive() -> Vec<PathBuf> {
        vec![PathBuf::from("src/dirty.rs")]
    }
}

fn paths(inventory: &DeletionInventory) -> Vec<String> {
    inventory.entries().iter().map(|entry| entry.path.display().into_owned().replace('\\', "/")).collect()
}

#[test]
fn every_entry_is_recorded_parents_first_with_identity_and_metadata() {
    let checkout = Checkout::new();
    link_dir(&checkout.outside, &checkout.path.join("linked"));

    let inventory = DeletionInventory::capture(&checkout.path, &Checkout::sensitive()).unwrap();

    assert_eq!(
        paths(&inventory),
        [".git", "README.md", "linked", "src", "src/dirty.rs", "target", "target/debug", "target/debug/deps", "target/debug/deps/lib.rlib"],
        "ignored content is included; a link is listed but never walked"
    );
    assert_eq!(*inventory.root(), crate::remove::quarantine::rename::identity(&checkout.path).unwrap());
    let readme = inventory.get(Path::new("README.md")).unwrap();
    assert_eq!(readme.identity, crate::remove::quarantine::rename::identity(&checkout.path.join("README.md")).unwrap());
    let Shape::File { size, content, .. } = &readme.shape else { panic!("{readme:?}") };
    assert_eq!((*size, content), (7, &Content::Unhashed), "only consent-sensitive files are hashed");
    let Shape::File { content, .. } = &inventory.get(Path::new("src/dirty.rs")).unwrap().shape else { panic!() };
    assert_eq!(*content, Content::Blake3(biscuit_hash::blake3_hash_reader(&mut &b"fn main() {}\n"[..]).unwrap()));
    let Shape::Link { target } = &inventory.get(Path::new("linked")).unwrap().shape else { panic!() };
    assert_eq!(target.as_path(), checkout.outside.as_path());
    assert!(matches!(inventory.get(Path::new("target/debug")).unwrap().shape, Shape::Directory { .. }));
    assert!(inventory.get(Path::new("linked/keep.txt")).is_none());
}

#[test]
fn a_file_below_a_sensitive_directory_is_hashed() {
    let checkout = Checkout::new();
    fs::create_dir(checkout.path.join("nested")).unwrap();
    fs::write(checkout.path.join("nested").join("inner.txt"), b"x").unwrap();
    // `git status` lists an untracked nested repository as one directory,
    // spelled with a trailing slash.
    let inventory = DeletionInventory::capture(&checkout.path, &[PathBuf::from("nested/")]).unwrap();
    let Shape::File { content, .. } = &inventory.get(Path::new("nested/inner.txt")).unwrap().shape else { panic!() };
    assert!(matches!(content, Content::Blake3(_)), "{content:?}");
    let Shape::File { content, .. } = &inventory.get(Path::new("README.md")).unwrap().shape else { panic!() };
    assert_eq!(*content, Content::Unhashed);
}

#[test]
fn the_inventory_round_trips_through_json() {
    let checkout = Checkout::new();
    let inventory = DeletionInventory::capture(&checkout.path, &Checkout::sensitive()).unwrap();
    let text = serde_json::to_string(&inventory).unwrap();
    let back: DeletionInventory = serde_json::from_str(&text).unwrap();
    assert_eq!(back, inventory);
    assert_eq!(serde_json::to_string(&back).unwrap(), text, "write, read, write is stable");
}

#[cfg(unix)]
#[test]
fn a_fifo_or_socket_is_ineligible_and_never_opened() {
    for (name, expected) in [("pipe", Ineligibility::Fifo), ("socket", Ineligibility::Socket)] {
        let checkout = Checkout::new();
        let path = checkout.path.join("target").join(name);
        let _listener;
        if expected == Ineligibility::Fifo {
            let c_path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
        } else {
            _listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        }
        // Marked sensitive, so hashing would open it; a FIFO with no writer
        // would block that open forever.
        let sensitive = vec![PathBuf::from("target").join(name)];
        match DeletionInventory::capture(&checkout.path, &sensitive) {
            Err(InventoryRefusal::Ineligible { path: refused, reason }) => {
                assert_eq!((refused, reason), (path, expected));
            }
            other => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn a_link_at_the_root_is_refused() {
    let checkout = Checkout::new();
    let link = checkout.parent.join("link");
    link_dir(&checkout.path, &link);
    match DeletionInventory::capture(&link, &[]) {
        Err(InventoryRefusal::Changed { change: Change::Replaced, .. }) => {}
        other => panic!("{other:?}"),
    }
}

/// An injected observation over `checkout`, still at its original path.
fn observed(checkout: &Checkout) -> (Observation, crate::remove::observation::injected::Injector) {
    let admin = checkout.parent.join("admin");
    fs::create_dir_all(&admin).unwrap();
    let targets = Targets::new(&checkout.path, &checkout.parent.join(".wt-remove-checkout-0123"), &admin).unwrap();
    Observation::injected(targets, ObserverOptions::default())
}

fn set_modified(path: &Path, when: SystemTime) {
    fs::File::options().write(true).open(path).unwrap().set_modified(when).unwrap();
}

/// A change made to a checkout's files.
type Edit = Box<dyn Fn(&Path)>;

#[test]
fn the_finishing_check_refuses_each_kind_of_change_at_the_reported_path() {
    let cases: Vec<(&str, Edit, &str, Change)> = vec![
        ("appended", Box::new(|root: &Path| fs::write(root.join("README.md"), b"readme\nmore\n").unwrap()), "README.md", Change::Modified),
        (
            "same size, same time, other bytes in a hashed file",
            Box::new(|root: &Path| {
                let path = root.join("src").join("dirty.rs");
                let before = fs::metadata(&path).unwrap().modified().unwrap();
                fs::write(&path, b"fn main() {0}").unwrap();
                set_modified(&path, before);
            }),
            "src/dirty.rs",
            Change::Modified,
        ),
        ("new file", Box::new(|root: &Path| fs::write(root.join("target").join("new.o"), b"x").unwrap()), "target/new.o", Change::Appeared),
        ("deleted", Box::new(|root: &Path| fs::remove_file(root.join("README.md")).unwrap()), "README.md", Change::Vanished),
        (
            "replaced",
            Box::new(|root: &Path| {
                fs::remove_file(root.join("README.md")).unwrap();
                fs::create_dir(root.join("README.md")).unwrap();
            }),
            "README.md",
            Change::Replaced,
        ),
        (
            "directory replaced by a link",
            Box::new(|root: &Path| {
                fs::remove_dir_all(root.join("src")).unwrap();
                link_dir(&root.join("target"), &root.join("src"));
            }),
            "src/dirty.rs",
            Change::Replaced,
        ),
    ];
    for (name, change_it, reported, expected) in cases {
        let checkout = Checkout::new();
        let (mut observation, injector) = observed(&checkout);
        let inventory = DeletionInventory::capture(&checkout.path, &Checkout::sensitive()).unwrap();
        change_it(&checkout.path);
        injector.event(Root::Tree, reported, EventKind::Modified);
        match inventory.confirm(&checkout.path, &observation.drain(&|| false)) {
            Err(InventoryRefusal::Changed { change, .. }) => assert_eq!(change, expected, "{name}"),
            other => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn an_unchanged_path_or_no_event_passes_and_unreported_metadata_preserving_writes_are_the_stated_limit() {
    let checkout = Checkout::new();
    let (mut observation, injector) = observed(&checkout);
    let inventory = DeletionInventory::capture(&checkout.path, &Checkout::sensitive()).unwrap();
    injector.event(Root::Tree, "README.md", EventKind::Modified);
    injector.event(Root::Tree, "src", EventKind::Metadata);
    inventory.confirm(&checkout.path, &observation.drain(&|| false)).unwrap();

    // Other bytes, same size and time, in a file that is not hashed, and no
    // event: nothing can see it, which the report states as its limit.
    let path = checkout.path.join("README.md");
    let before = fs::metadata(&path).unwrap().modified().unwrap();
    fs::write(&path, b"README\n").unwrap();
    set_modified(&path, before);
    inventory.confirm(&checkout.path, &observation.drain(&|| false)).unwrap();
    // The same edit reported: the hash-free check still passes, since the
    // inspected metadata did not change.
    injector.event(Root::Tree, "README.md", EventKind::Modified);
    inventory.confirm(&checkout.path, &observation.drain(&|| false)).unwrap();
    // A different time is detected.
    set_modified(&path, before + Duration::from_secs(5));
    injector.event(Root::Tree, "README.md", EventKind::Modified);
    assert!(matches!(
        inventory.confirm(&checkout.path, &observation.drain(&|| false)),
        Err(InventoryRefusal::Changed { change: Change::Modified, .. })
    ));
}

#[cfg(unix)]
#[test]
fn a_permission_change_is_a_modification() {
    use std::os::unix::fs::PermissionsExt;
    let checkout = Checkout::new();
    let (mut observation, injector) = observed(&checkout);
    let inventory = DeletionInventory::capture(&checkout.path, &[]).unwrap();
    fs::set_permissions(checkout.path.join("README.md"), fs::Permissions::from_mode(0o755)).unwrap();
    injector.event(Root::Tree, "README.md", EventKind::Metadata);
    assert!(matches!(
        inventory.confirm(&checkout.path, &observation.drain(&|| false)),
        Err(InventoryRefusal::Changed { change: Change::Modified, .. })
    ));
}

#[test]
fn lost_coverage_or_a_replaced_root_refuses() {
    let checkout = Checkout::new();
    let (mut observation, injector) = observed(&checkout);
    let inventory = DeletionInventory::capture(&checkout.path, &[]).unwrap();
    injector.overflow();
    assert!(matches!(inventory.confirm(&checkout.path, &observation.drain(&|| false)), Err(InventoryRefusal::Coverage(CoverageLost::Overflow))));

    let checkout = Checkout::new();
    let (mut observation, _injector) = observed(&checkout);
    let inventory = DeletionInventory::capture(&checkout.path, &[]).unwrap();
    fs::rename(&checkout.path, checkout.parent.join("elsewhere")).unwrap();
    fs::create_dir(&checkout.path).unwrap();
    assert!(matches!(
        inventory.confirm(&checkout.path, &observation.drain(&|| false)),
        Err(InventoryRefusal::Changed { change: Change::Replaced, .. })
    ));
}

/// The capture itself (including hashing) raises no event that the
/// finishing check would count as a change.
#[test]
fn native_capturing_an_inventory_raises_nothing_the_check_refuses() {
    let checkout = Checkout::new();
    let admin = checkout.parent.join("admin");
    fs::create_dir_all(&admin).unwrap();
    let targets = Targets::new(&checkout.path, &checkout.parent.join(".wt-remove-checkout-0123"), &admin).unwrap();
    let mut observation = Observation::start(targets, ObserverOptions::default(), &|| false).unwrap();
    let inventory = DeletionInventory::capture(&checkout.path, &Checkout::sensitive()).unwrap();
    inventory.confirm(&checkout.path, &observation.drain(&|| false)).unwrap();

    fs::write(checkout.path.join("target").join("late.o"), b"x").unwrap();
    match inventory.confirm(&checkout.path, &observation.drain(&|| false)) {
        Err(InventoryRefusal::Changed { path, change: Change::Appeared }) => {
            assert!(path.ends_with(Path::new("target").join("late.o")), "{path:?}");
        }
        other => panic!("{other:?}"),
    }
}
