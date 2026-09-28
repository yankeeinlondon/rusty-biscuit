//! Per-worktree copy baselines bound to a Git registration marker.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::cache::{atomic_write_private, repo_cache_file};
use crate::compare::Observation;
use crate::error::WorktreeError;

pub const FORMAT_VERSION: u32 = 1;
const MARKER: &str = "wt-copy-registration";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordFile {
    pub path_hex: String,
    pub observation: Observation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyRecord {
    pub format_version: u32,
    pub worktree: PathBuf,
    pub admin_dir: PathBuf,
    pub registration: String,
    pub source: PathBuf,
    pub source_label: String,
    pub files: Vec<RecordFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    pub admin_dir: PathBuf,
    pub nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadedRecord {
    Trusted(CopyRecord),
    Absent,
    Untrusted(String),
}

pub fn path_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn record_path(repo_root: &Path, worktree: &Path) -> Result<PathBuf, WorktreeError> {
    let canonical = canonical_worktree_path(worktree)?;
    #[cfg(unix)]
    let bytes = {
        use std::os::unix::ffi::OsStrExt;
        canonical.as_os_str().as_bytes().to_vec()
    };
    #[cfg(windows)]
    let bytes = canonical.to_string_lossy().as_bytes().to_vec();
    let hash = biscuit_hash::blake3_hash_bytes(&bytes);
    repo_cache_file(repo_root, &format!("copy-{}.json", path_hex(&hash[..8])))
}

fn canonical_worktree_path(path: &Path) -> io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut missing = Vec::new();
    let mut existing = absolute.as_path();
    loop {
        match fs::canonicalize(existing) {
            Ok(mut canonical) => {
                for component in missing.into_iter().rev() { canonical.push(component); }
                return Ok(canonical);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let name = existing.file_name().ok_or(error)?;
                missing.push(name.to_os_string());
                existing = existing.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "worktree path has no existing parent"))?;
            }
            Err(error) => return Err(error),
        }
    }
}

pub fn registration(admin_dir: &Path) -> Result<Registration, WorktreeError> {
    let admin_dir = fs::canonicalize(admin_dir)?;
    let marker = admin_dir.join(MARKER);
    let nonce = match fs::read_to_string(&marker) {
        Ok(nonce) => nonce,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let mut bytes = [0_u8; 16];
            getrandom::fill(&mut bytes).map_err(|e| WorktreeError::Io(io::Error::other(e.to_string())))?;
            let nonce = path_hex(&bytes);
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)] {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&marker) {
                Ok(mut file) => { file.write_all(nonce.as_bytes())?; nonce },
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => fs::read_to_string(&marker)?,
                Err(error) => return Err(error.into()),
            }
        }
        Err(error) => return Err(error.into()),
    };
    if nonce.len() != 32 || !nonce.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(WorktreeError::Io(io::Error::new(io::ErrorKind::InvalidData, "invalid copy registration marker")));
    }
    Ok(Registration { admin_dir, nonce })
}

pub fn write_atomic(path: &Path, record: &CopyRecord) -> Result<(), WorktreeError> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    atomic_write_private(path, &serde_json::to_vec_pretty(record)?)
}

pub fn delete(path: &Path) -> Result<(), WorktreeError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Deletes the copy record after a worktree is removed. Cache failures are
/// warnings because the worktree has already gone.
pub fn delete_for(repo_root: &Path, worktree: &Path) -> Option<String> {
    record_path(repo_root, worktree).and_then(|path| delete(&path))
        .err().map(|error| error.to_string())
}

pub fn load(repo_root: &Path, worktree: &Path, expected: &Registration) -> LoadedRecord {
    let path = match record_path(repo_root, worktree) {
        Ok(path) => path,
        Err(error) => return LoadedRecord::Untrusted(error.to_string()),
    };
    load_from(&path, worktree, expected)
}

pub fn load_from(path: &Path, worktree: &Path, expected: &Registration) -> LoadedRecord {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return LoadedRecord::Absent,
        Err(error) => return LoadedRecord::Untrusted(error.to_string()),
    };
    let record: CopyRecord = match serde_json::from_slice(&bytes) {
        Ok(record) => record,
        Err(error) => return LoadedRecord::Untrusted(error.to_string()),
    };
    let current = match fs::read_to_string(expected.admin_dir.join(MARKER)) {
        Ok(current) => current,
        Err(error) => return LoadedRecord::Untrusted(error.to_string()),
    };
    let canonical = match fs::canonicalize(worktree) {
        Ok(canonical) => canonical,
        Err(error) => return LoadedRecord::Untrusted(error.to_string()),
    };
    if record.format_version != FORMAT_VERSION || record.worktree != canonical
        || record.admin_dir != expected.admin_dir || record.registration != expected.nonce
        || current != expected.nonce {
        return LoadedRecord::Untrusted("record does not match this registration".into());
    }
    if record.files.iter().any(|file| file.path_hex.is_empty() || file.path_hex.len() % 2 != 0
        || !file.path_hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))) {
        return LoadedRecord::Untrusted("record contains an invalid path".into());
    }
    LoadedRecord::Trusted(record)
}

/// Deletes this repository's records whose worktree is neither in `live` nor
/// still registered with the marker the record was bound to.
///
/// `live` is a listing's `git worktree list`, which can predate a worktree
/// `wt create` added while the listing waited on the remote; that worktree's
/// registration still matches, so its new record survives.
pub fn prune_from(cache_dir: &Path, repo_root: &Path, live: &HashSet<PathBuf>) -> Result<(), WorktreeError> {
    let prefix_path = record_path(repo_root, repo_root)?;
    let prefix = prefix_path.file_name().unwrap().to_string_lossy();
    let repo_prefix = prefix.split(".copy-").next().unwrap();
    let entries = match fs::read_dir(cache_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(&format!("{repo_prefix}.copy-")) || !name.ends_with(".json") { continue; }
        let path = entry.path();
        let Ok(bytes) = fs::read(&path) else { continue; };
        let Ok(record) = serde_json::from_slice::<CopyRecord>(&bytes) else { continue; };
        if !live.contains(&record.worktree) && !still_registered(&record) { delete(&path)?; }
    }
    Ok(())
}

/// Whether `record`'s Git admin directory still holds the marker it was
/// bound to; Git deletes the directory when the worktree is removed or pruned.
fn still_registered(record: &CopyRecord) -> bool {
    fs::read_to_string(record.admin_dir.join(MARKER)).is_ok_and(|nonce| nonce == record.registration)
}

pub fn prune(repo_root: &Path, live: &HashSet<PathBuf>) -> Result<(), WorktreeError> {
    let path = record_path(repo_root, repo_root)?;
    prune_from(path.parent().unwrap(), repo_root, live)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remove::test_support::TestRepo;

    #[test]
    fn repeated_round_trip_and_identity_failures() {
        let repo = TestRepo::new();
        let admin = repo.path().join(".git");
        let identity = registration(&admin).unwrap();
        let path = repo.cache_path().join("record.json");
        let mut record = CopyRecord { format_version: FORMAT_VERSION,
            worktree: repo.path().canonicalize().unwrap(), admin_dir: identity.admin_dir.clone(),
            registration: identity.nonce.clone(), source: repo.path(), source_label: "base".into(), files: vec![] };
        write_atomic(&path, &record).unwrap();
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        assert_eq!(load_from(&path, &repo.path(), &identity), LoadedRecord::Trusted(record.clone()));
        record.source_label = "branch".into();
        write_atomic(&path, &record).unwrap();
        assert_eq!(load_from(&path, &repo.path(), &identity), LoadedRecord::Trusted(record.clone()));
        let mut wrong = identity.clone(); wrong.nonce = "different".into();
        assert!(matches!(load_from(&path, &repo.path(), &wrong), LoadedRecord::Untrusted(_)));
        fs::write(identity.admin_dir.join(MARKER), b"re-registered").unwrap();
        assert!(matches!(load_from(&path, &repo.path(), &identity), LoadedRecord::Untrusted(_)));
        assert!(registration(&admin).is_err());
        fs::write(identity.admin_dir.join(MARKER), &identity.nonce).unwrap();
        record.format_version += 1;
        write_atomic(&path, &record).unwrap();
        assert!(matches!(load_from(&path, &repo.path(), &identity), LoadedRecord::Untrusted(_)));
        record.format_version = FORMAT_VERSION;
        record.files.push(RecordFile { path_hex: "ZZ".into(), observation: Observation {
            kind: crate::compare::Kind::File, size: 0, digest: None,
        } });
        write_atomic(&path, &record).unwrap();
        assert!(matches!(load_from(&path, &repo.path(), &identity), LoadedRecord::Untrusted(_)));
        fs::write(&path, b"not json").unwrap();
        assert!(matches!(load_from(&path, &repo.path(), &identity), LoadedRecord::Untrusted(_)));
        delete(&path).unwrap();
        assert_eq!(load_from(&path, &repo.path(), &identity), LoadedRecord::Absent);
    }

    #[test]
    fn separate_worktree_records_survive_concurrent_writes_and_prune() {
        let repo = TestRepo::new();
        let first = repo.add_linked_worktree("first");
        let second = repo.add_linked_worktree("second");
        let cache = repo.cache_path();
        fs::create_dir(&cache).unwrap();
        let records = [first.clone(), second.clone()].map(|worktree| {
            let admin = repo.git_in(&worktree, &["rev-parse", "--path-format=absolute", "--git-dir"]);
            let identity = registration(Path::new(&admin)).unwrap();
            let record = CopyRecord { format_version: FORMAT_VERSION,
                worktree: worktree.canonicalize().unwrap(), admin_dir: identity.admin_dir,
                registration: identity.nonce, source: repo.path(), source_label: "base".into(), files: vec![] };
            let name = record_path(&repo.path(), &worktree).unwrap().file_name().unwrap().to_owned();
            (cache.join(name), record)
        });
        std::thread::scope(|scope| {
            for (path, record) in &records {
                scope.spawn(move || write_atomic(path, record).unwrap());
            }
        });
        assert_ne!(records[0].0, records[1].0);
        repo.git(&["worktree", "remove", "--force", second.to_str().unwrap()]);
        let live = HashSet::from([first.canonicalize().unwrap()]);
        prune_from(&cache, &repo.path(), &live).unwrap();
        assert!(records[0].0.exists());
        assert!(!records[1].0.exists());
    }

    /// A listing's worktree list predates a worktree created during its
    /// remote wait; that worktree's new record must survive the prune.
    #[test]
    fn a_record_for_a_worktree_missing_from_the_listing_survives_while_registered() {
        let repo = TestRepo::new();
        let added = repo.add_linked_worktree("added-during-the-wait");
        let cache = repo.cache_path();
        let admin = repo.git_in(&added, &["rev-parse", "--path-format=absolute", "--git-dir"]);
        let identity = registration(Path::new(&admin)).unwrap();
        let record = CopyRecord { format_version: FORMAT_VERSION,
            worktree: added.canonicalize().unwrap(), admin_dir: identity.admin_dir,
            registration: identity.nonce, source: repo.path(), source_label: "base".into(), files: vec![] };
        let path = cache.join(record_path(&repo.path(), &added).unwrap().file_name().unwrap());
        write_atomic(&path, &record).unwrap();

        prune_from(&cache, &repo.path(), &HashSet::from([repo.path().canonicalize().unwrap()])).unwrap();
        assert!(path.exists(), "still registered with the record's marker");

        fs::write(record.admin_dir.join(MARKER), "0123456789abcdef0123456789abcdef").unwrap();
        prune_from(&cache, &repo.path(), &HashSet::new()).unwrap();
        assert!(!path.exists(), "a new registration at that path does not keep the old record");
    }

    #[test]
    fn unreadable_cache_directory_is_an_error() {
        let repo = TestRepo::new();
        let file = repo.cache_path();
        fs::write(&file, b"not a directory").unwrap();
        assert!(prune_from(&file, &repo.path(), &HashSet::new()).is_err());
    }

    #[test]
    fn destination_key_is_stable_before_and_after_creation() {
        let repo = TestRepo::new();
        let destination = repo.path().parent().unwrap().join("new-wts").join("feature");
        let before = record_path(&repo.path(), &destination).unwrap();
        fs::create_dir_all(&destination).unwrap();
        let after = record_path(&repo.path(), &destination).unwrap();
        assert_eq!(before, after);
        fs::remove_dir_all(&destination).unwrap();
        assert_eq!(record_path(&repo.path(), &destination).unwrap(), before);
    }
}
