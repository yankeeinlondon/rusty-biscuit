//! Content comparison for copied include entries.

use std::fs::{self, File};
use std::io;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    File,
    Symlink { target_bytes: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub kind: Kind,
    pub size: u64,
    pub digest: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    Included,
    DirtyHandoff,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
    Unchanged,
    Changed,
    Missing,
    Unknown(String),
}

pub trait ReadCounter {
    fn open(&self, path: &Path) -> io::Result<File>;
}

pub struct FilesystemReader;
impl ReadCounter for FilesystemReader {
    fn open(&self, path: &Path) -> io::Result<File> { File::open(path) }
}

pub fn digest_file(path: &Path, reader: &dyn ReadCounter) -> io::Result<[u8; 32]> {
    let mut file = reader.open(path)?;
    let hex = biscuit_hash::blake3_hash_reader(&mut file)?;
    let mut digest = [0; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    }
    Ok(digest)
}

pub fn observe(path: &Path, reader: &dyn ReadCounter) -> io::Result<Observation> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(path)?;
        let target_bytes = os_bytes(target.as_os_str())?;
        return Ok(Observation { size: target_bytes.len() as u64,
            kind: Kind::Symlink { target_bytes }, digest: None });
    }
    if !metadata.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "unsupported file kind"));
    }
    Ok(Observation { kind: Kind::File, size: metadata.len(),
        digest: Some(digest_file(path, reader)?) })
}

pub fn compare(baseline: &Observation, path: &Path, _policy: Policy,
    reader: &dyn ReadCounter) -> Comparison {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Comparison::Missing,
        Err(error) => return Comparison::Unknown(error.to_string()),
    };
    match (&baseline.kind, metadata.file_type()) {
        (Kind::File, kind) if kind.is_file() => {
            if baseline.digest.is_none() { return Comparison::Unknown("copy has no trusted digest".into()); }
            if baseline.size != metadata.len() { return Comparison::Changed; }
            match digest_file(path, reader) {
                Ok(digest) if Some(digest) == baseline.digest => Comparison::Unchanged,
                Ok(_) => Comparison::Changed,
                Err(error) => Comparison::Unknown(error.to_string()),
            }
        }
        (Kind::Symlink { target_bytes }, kind) if kind.is_symlink() => {
            match fs::read_link(path).and_then(|p| os_bytes(p.as_os_str())) {
                Ok(current) if &current == target_bytes => Comparison::Unchanged,
                Ok(_) => Comparison::Changed,
                Err(error) => Comparison::Unknown(error.to_string()),
            }
        }
        _ => Comparison::Changed,
    }
}

#[cfg(unix)]
fn os_bytes(value: &std::ffi::OsStr) -> io::Result<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;
    Ok(value.as_bytes().to_vec())
}

#[cfg(windows)]
fn os_bytes(value: &std::ffi::OsStr) -> io::Result<Vec<u8>> {
    value.to_str().map(|s| s.as_bytes().to_vec())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "path is not UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct CountingReader(Cell<usize>);
    impl ReadCounter for CountingReader {
        fn open(&self, path: &Path) -> io::Result<File> {
            self.0.set(self.0.get() + 1);
            File::open(path)
        }
    }

    #[test]
    fn size_change_avoids_read_and_same_size_edit_is_hashed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        fs::write(&path, b"secret1").unwrap();
        let reader = CountingReader(Cell::new(0));
        let baseline = observe(&path, &reader).unwrap();
        let original_time = fs::metadata(&path).unwrap().modified().unwrap();
        reader.0.set(0);
        fs::write(&path, b"different-length").unwrap();
        assert_eq!(compare(&baseline, &path, Policy::Included, &reader), Comparison::Changed);
        assert_eq!(reader.0.get(), 0);
        fs::write(&path, b"secret2").unwrap();
        File::options().write(true).open(&path).unwrap()
            .set_times(fs::FileTimes::new().set_modified(original_time)).unwrap();
        assert_eq!(compare(&baseline, &path, Policy::Included, &reader), Comparison::Changed);
        assert_eq!(reader.0.get(), 1);
        fs::write(&path, b"secret1").unwrap();
        assert_eq!(compare(&baseline, &path, Policy::Included, &reader), Comparison::Unchanged);
        assert_eq!(reader.0.get(), 2);
        fs::remove_file(&path).unwrap();
        assert_eq!(compare(&baseline, &path, Policy::Included, &reader), Comparison::Missing);
    }

    #[test]
    fn large_same_size_file_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.env");
        fs::write(&path, vec![b'a'; 2_000_000]).unwrap();
        let reader = CountingReader(Cell::new(0));
        let baseline = observe(&path, &reader).unwrap();
        let original_time = fs::metadata(&path).unwrap().modified().unwrap();
        fs::write(&path, vec![b'b'; 2_000_000]).unwrap();
        File::options().write(true).open(&path).unwrap()
            .set_times(fs::FileTimes::new().set_modified(original_time)).unwrap();
        reader.0.set(0);
        assert_eq!(compare(&baseline, &path, Policy::Included, &reader), Comparison::Changed);
        assert_eq!(reader.0.get(), 1);
    }

    #[test]
    fn read_failure_is_unknown_and_kind_change_is_changed() {
        struct Denied;
        impl ReadCounter for Denied {
            fn open(&self, _: &Path) -> io::Result<File> {
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied"))
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        fs::write(&path, b"x").unwrap();
        let baseline = observe(&path, &FilesystemReader).unwrap();
        assert!(matches!(compare(&baseline, &path, Policy::Included, &Denied), Comparison::Unknown(_)));
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert_eq!(compare(&baseline, &path, Policy::Included, &Denied), Comparison::Changed);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_target_change_is_changed_without_following() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        symlink("outside-a", &path).unwrap();
        let baseline = observe(&path, &FilesystemReader).unwrap();
        fs::remove_file(&path).unwrap();
        symlink("outside-b", &path).unwrap();
        assert_eq!(compare(&baseline, &path, Policy::Included, &FilesystemReader), Comparison::Changed);
    }

    #[test]
    fn untrusted_copy_baseline_is_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        fs::write(&path, b"secret").unwrap();
        let mut baseline = observe(&path, &FilesystemReader).unwrap();
        baseline.digest = None;
        assert!(matches!(compare(&baseline, &path, Policy::Included, &FilesystemReader), Comparison::Unknown(_)));
    }
}
