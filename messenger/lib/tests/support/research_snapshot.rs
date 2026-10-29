//! Immutable fixture snapshots generated once per executable and input revision.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("fixture directory");
    for entry in fs::read_dir(from).expect("fixture entries") {
        let entry = entry.expect("fixture entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("fixture type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy fixture file");
        }
    }
}

fn inputs(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
    for entry in fs::read_dir(dir).expect("input directory") {
        let entry = entry.expect("input entry");
        let path = entry.path();
        if entry.file_type().expect("input type").is_dir() {
            inputs(root, &path, files);
        } else {
            files.insert(path.strip_prefix(root).unwrap().to_path_buf(), fs::read(path).expect("input bytes"));
        }
    }
}

/// The seed includes every schema and fleet fixture, so editing a runtime-read
/// input invalidates the snapshot even when Cargo does not rebuild the binary.
/// Only setup is reused: callers copy it before exercising mutable behavior.
pub fn snapshot(seed: &Path, label: &str, prepare: impl FnOnce(&Path)) -> PathBuf {
    let exe = std::env::current_exe().expect("test executable");
    let metadata = fs::metadata(&exe).expect("executable metadata");
    let mut key = format!("{exe:?}:{:?}:{}:{label}", metadata.modified().unwrap(), metadata.len()).into_bytes();
    let mut files = BTreeMap::new();
    inputs(seed, seed, &mut files);
    for (path, bytes) in files {
        key.extend(format!("{path:?}:{}:", bytes.len()).as_bytes());
        key.extend(bytes);
    }
    let parent = exe.parent().unwrap().join("messenger-research-fixtures");
    fs::create_dir_all(&parent).expect("snapshot cache");
    let name = format!("{:016x}", biscuit_hash::xx_hash_bytes(&key));
    let snapshot = parent.join(&name);
    let lock = fs::OpenOptions::new().create(true).truncate(false).write(true)
        .open(parent.join(format!("{name}.lock"))).expect("snapshot lock");
    lock.lock().expect("lock snapshot setup");
    if !snapshot.is_dir() {
        let building = tempfile::tempdir_in(&parent).expect("snapshot staging");
        copy_tree(seed, building.path());
        prepare(building.path());
        fs::rename(building.path(), &snapshot).expect("publish immutable snapshot");
    }
    snapshot
}
