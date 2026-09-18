#![allow(dead_code)]

use std::path::Path;

use publication_spike::SpikeError;
use publication_spike::model::{Snapshot, Verified, fixture, parse_manifest};

pub fn old() -> Snapshot {
    fixture("old", false)
}

pub fn new() -> Snapshot {
    fixture("new", true)
}

pub fn id(snapshot: &Snapshot) -> String {
    parse_manifest(&snapshot.manifest_bytes()).expect("manifest").snapshot_id
}

/// What a verified read observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seen {
    Old,
    New,
    NoSnapshot,
    Refused(String),
    /// A verified snapshot that is neither: must never happen.
    Other(String),
}

pub fn classify(result: Result<Verified, SpikeError>) -> Seen {
    match result {
        Ok(v) if v.manifest.snapshot_id == id(&old()) => {
            assert_eq!(v.files, old().artifacts, "verified bytes must equal the old snapshot exactly");
            Seen::Old
        }
        Ok(v) if v.manifest.snapshot_id == id(&new()) => {
            assert_eq!(v.files, new().artifacts, "verified bytes must equal the new snapshot exactly");
            Seen::New
        }
        Ok(v) => Seen::Other(v.manifest.snapshot_id),
        Err(SpikeError::NoSnapshot) => Seen::NoSnapshot,
        Err(error) => Seen::Refused(error.to_string()),
    }
}

/// Every leftover the protocol might create outside committed content.
pub fn leftovers(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    walk(root, root, &mut found);
    found
}

fn walk(root: &Path, dir: &Path, found: &mut Vec<String>) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    for item in read.flatten() {
        let path = item.path();
        let name = item.file_name().to_string_lossy().into_owned();
        if name.ends_with(".publish-tmp") || name == "journal.json" || (name == "tx" && path.is_dir()) {
            found.push(path.strip_prefix(root).unwrap().display().to_string());
        }
        if path.is_dir() {
            walk(root, &path, found);
        }
    }
}
