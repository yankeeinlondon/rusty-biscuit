//! Copy included entries without replacing destination content.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Seek};
use std::path::{Path, PathBuf};
use cap_std::fs::{Dir, OpenOptions};
use cap_std::ambient_authority;
use crate::compare::{Kind, Observation};
use super::{EntryKind, IncludeSet, guarded_kind, relative_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason { Existing, IndexTracked, LinkUnsupported, UnsafePath }

#[derive(Debug, Default)]
pub struct CopyOutcome {
    pub copied: Vec<(Vec<u8>, Option<Observation>)>,
    pub skipped: Vec<(Vec<u8>, SkipReason)>,
    pub failed: Vec<(Vec<u8>, String)>,
    pub warnings: Vec<String>,
}

pub trait CopyOps {
    fn clone_file(&self, source: &File, parent: &Dir, temp: &Path) -> io::Result<File>;
    fn byte_copy(&self, source: &File, parent: &Dir, temp: &Path) -> io::Result<File>;
    fn symlink(&self, target: &Path, parent: &Dir, dest: &Path) -> io::Result<()>;
    fn publish(&self, parent: &Dir, temp: &Path, dest: &Path) -> io::Result<()>;
}

pub struct RealCopyOps;
impl CopyOps for RealCopyOps {
    fn clone_file(&self, source: &File, parent: &Dir, temp: &Path) -> io::Result<File> {
        #[cfg(target_os = "macos")]
        {
            use std::ffi::CString;
            use std::os::fd::AsRawFd;
            use std::os::unix::ffi::OsStrExt;
            unsafe extern "C" {
                fn fclonefileat(source: std::ffi::c_int, parent: std::ffi::c_int,
                    name: *const std::ffi::c_char, flags: std::ffi::c_int) -> std::ffi::c_int;
            }
            let name = CString::new(temp.as_os_str().as_bytes())?;
            if unsafe { fclonefileat(source.as_raw_fd(), parent.as_raw_fd(), name.as_ptr(), 0) } != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(parent.open(temp)?.into_std())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let dest = create_temp(parent, temp)?;
            let len = source.metadata()?.len();
            let Some(len) = std::num::NonZeroU64::new(len) else {
                return Err(io::ErrorKind::Unsupported.into());
            };
            #[cfg(windows)] dest.set_len(len.get())?;
            reflink_copy::ReflinkBlockBuilder::new(source, &dest, len).reflink_block()?;
            Ok(dest)
        }
    }
    fn byte_copy(&self, source: &File, parent: &Dir, temp: &Path) -> io::Result<File> {
        let mut source = source.try_clone()?;
        source.rewind()?;
        let mut dest = create_temp(parent, temp)?;
        io::copy(&mut source, &mut dest)?;
        Ok(dest)
    }
    fn symlink(&self, target: &Path, parent: &Dir, dest: &Path) -> io::Result<()> {
        #[cfg(unix)] { parent.symlink_contents(target, dest) }
        #[cfg(windows)] { parent.symlink_file(target, dest) }
    }
    fn publish(&self, parent: &Dir, temp: &Path, dest: &Path) -> io::Result<()> {
        #[cfg(unix)]
        { parent.hard_link(temp, parent, dest) }
        #[cfg(windows)]
        {
            parent.hard_link(temp, parent, dest)
        }
    }
}

fn create_temp(parent: &Dir, name: &Path) -> io::Result<File> {
    parent.open_with(name, OpenOptions::new().read(true).write(true).create_new(true))
        .map(|file| file.into_std())
}

fn temp_path() -> io::Result<PathBuf> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| io::Error::other(e.to_string()))?;
    let name: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(PathBuf::from(format!(".wt-copy-{name}.tmp")))
}

fn exists_even_dangling(parent: &Dir, name: &Path) -> bool {
    parent.symlink_metadata(name).is_ok()
}

fn safe_parent(root: &Dir, relative: &Path, create: bool) -> io::Result<Option<Dir>> {
    let Some(parent) = relative.parent() else { return root.try_clone().map(Some); };
    let mut current = root.try_clone()?;
    for part in parent.components() {
        let name = Path::new(part.as_os_str());
        match current.symlink_metadata(name) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                #[cfg(windows)] {
                    use cap_std::fs::MetadataExt;
                    if metadata.file_attributes() & 0x400 != 0 { return Ok(None); }
                }
            }
            Ok(_) => return Ok(None),
            Err(error) if error.kind() == io::ErrorKind::NotFound && create => current.create_dir(name)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        }
        current = current.open_dir(name)?;
    }
    Ok(Some(current))
}

fn metadata_stable(before: &fs::Metadata, after: &fs::Metadata) -> bool {
    before.len() == after.len() && before.modified().ok() == after.modified().ok()
}

fn unsupported_clone(error: &io::Error) -> bool {
    matches!(error.kind(), io::ErrorKind::Unsupported | io::ErrorKind::PermissionDenied)
        || matches!(error.raw_os_error(), Some(18 | 45 | 95 | 38 | 1 | 17 | 50))
        || error.raw_os_error().is_some_and(|code| {
            let code = code as u32;
            code & 0xffff0000 == 0x80070000 && matches!(code & 0xffff, 1 | 17 | 18 | 50)
        })
}

fn link_unsupported(error: &io::Error) -> bool {
    matches!(error.kind(), io::ErrorKind::Unsupported | io::ErrorKind::PermissionDenied)
        || error.raw_os_error() == Some(1314)
}

fn source_file(parent: &Dir, name: &Path) -> io::Result<File> {
    let before = parent.symlink_metadata(name)?;
    if !before.is_file() || before.file_type().is_symlink() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "source is no longer a regular file"));
    }
    #[cfg(windows)]
    let opened = {
        use cap_std::fs::OpenOptionsExt;
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(0x0020_0000);
        parent.open_with(name, &options)?
    };
    #[cfg(not(windows))]
    let opened = parent.open(name)?;
    let file = opened.into_std();
    let after = file.metadata()?;
    if !after.is_file() || !same_file(&before, &after) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "source changed while opening"));
    }
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        if after.file_attributes() & 0x400 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "source is a reparse point"));
        }
    }
    Ok(file)
}

fn open_checked_root(path: &Path) -> io::Result<Dir> {
    let before = fs::symlink_metadata(path)?;
    if !before.is_dir() || before.file_type().is_symlink() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "copy root is not a directory"));
    }
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        if before.file_attributes() & 0x400 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "copy root is a reparse point"));
        }
    }
    let dir = Dir::open_ambient_dir(path, ambient_authority())?;
    #[cfg(unix)] {
        use cap_std::fs::MetadataExt as _;
        use std::os::unix::fs::MetadataExt as _;
        let after = dir.dir_metadata()?;
        if before.dev() != after.dev() || before.ino() != after.ino() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "copy root changed while opening"));
        }
    }
    Ok(dir)
}

#[cfg(unix)]
fn same_file(before: &cap_std::fs::Metadata, after: &fs::Metadata) -> bool {
    use cap_std::fs::MetadataExt as _;
    use std::os::unix::fs::MetadataExt as _;
    before.dev() == after.dev() && before.ino() == after.ino()
}

#[cfg(windows)]
fn same_file(before: &cap_std::fs::Metadata, after: &fs::Metadata) -> bool {
    before.len() == after.len()
        && before.modified().ok().map(cap_std::time::SystemTime::into_std) == after.modified().ok()
}

pub fn copy_include_set(source_root: &Path, dest_root: &Path, set: &IncludeSet,
    dest_index: &HashSet<Vec<u8>>, ops: &dyn CopyOps) -> CopyOutcome {
    let mut outcome = CopyOutcome::default();
    let roots = open_checked_root(source_root)
        .and_then(|source| open_checked_root(dest_root)
            .map(|dest| (source, dest)));
    let (source_dir, dest_dir) = match roots {
        Ok(roots) => roots,
        Err(error) => {
            for entry in &set.entries { outcome.failed.push((entry.path.clone(), error.to_string())); }
            return outcome;
        }
    };
    for entry in &set.entries {
        let path = &entry.path;
        if dest_index.contains(path) {
            outcome.skipped.push((path.clone(), SkipReason::IndexTracked)); continue;
        }
        let relative = match relative_path(path) {
            Ok(relative) => relative,
            Err(error) => { outcome.failed.push((path.clone(), error.to_string())); continue; }
        };
        let name = Path::new(relative.file_name().expect("validated relative path has a filename"));
        match guarded_kind(source_root, &relative) {
            Ok(Some(kind)) if kind == entry.kind => {},
            Ok(_) => { outcome.skipped.push((path.clone(), SkipReason::UnsafePath)); continue; },
            Err(error) => { outcome.failed.push((path.clone(), error.to_string())); continue; },
        }
        let source_parent = match safe_parent(&source_dir, &relative, false) {
            Ok(Some(parent)) => parent,
            Ok(None) => { outcome.skipped.push((path.clone(), SkipReason::UnsafePath)); continue; },
            Err(error) => { outcome.failed.push((path.clone(), error.to_string())); continue; },
        };
        let dest_parent = match safe_parent(&dest_dir, &relative, true) {
            Ok(Some(parent)) => parent,
            Ok(None) => { outcome.skipped.push((path.clone(), SkipReason::UnsafePath)); continue; },
            Err(error) => { outcome.failed.push((path.clone(), error.to_string())); continue; },
        };
        if exists_even_dangling(&dest_parent, name) {
            outcome.skipped.push((path.clone(), SkipReason::Existing)); continue;
        }
        match entry.kind {
            EntryKind::Symlink => {
                let target = source_parent.read_link_contents(name);
                let result = target.as_ref().map_err(|error| io::Error::new(error.kind(), error.to_string()))
                    .and_then(|target| ops.symlink(target, &dest_parent, name));
                match result {
                    Ok(()) => match dest_parent.read_link_contents(name).and_then(|target| link_observation(&target)) {
                        Ok(observation) => {
                            let stable = source_parent.read_link_contents(name).ok().as_ref() == target.as_ref().ok();
                            if !stable { outcome.warnings.push(format!("source changed while copying {}", relative.display())); }
                            outcome.copied.push((path.clone(), if stable { Some(observation) } else { None }));
                        }
                        Err(error) => outcome.failed.push((path.clone(), error.to_string())),
                    },
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists =>
                        outcome.skipped.push((path.clone(), SkipReason::Existing)),
                    Err(error) if link_unsupported(&error) =>
                        outcome.skipped.push((path.clone(), SkipReason::LinkUnsupported)),
                    Err(error) => outcome.failed.push((path.clone(), error.to_string())),
                }
            }
            EntryKind::File => {
                let result = source_file(&source_parent, name)
                    .and_then(|source_file| copy_file(&source_file, &source_parent, &dest_parent, name, ops));
                match result {
                    Ok((observation, stable)) => {
                        if !stable { outcome.warnings.push(format!("source changed while copying {}", relative.display())); }
                        outcome.copied.push((path.clone(), if stable { Some(observation) } else { None }));
                    }
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists =>
                        outcome.skipped.push((path.clone(), SkipReason::Existing)),
                    Err(error) => outcome.failed.push((path.clone(), error.to_string())),
                }
            }
            EntryKind::Other => outcome.skipped.push((path.clone(), SkipReason::UnsafePath)),
        }
    }
    outcome
}

fn copy_file(source: &File, source_parent: &Dir, parent: &Dir, dest: &Path,
    ops: &dyn CopyOps) -> io::Result<(Observation, bool)> {
    let before = source.metadata()?;
    let mut temp = temp_path()?;
    let result = (|| {
        let copied = match ops.clone_file(source, parent, &temp) {
            Ok(file) => file,
            Err(error) if unsupported_clone(&error) => {
                let _ = parent.remove_file(&temp);
                temp = temp_path()?;
                ops.byte_copy(source, parent, &temp)?
            }
            Err(error) => return Err(error),
        };
        copied.set_permissions(before.permissions())?;
        let observation = observe_open_file(&copied)?;
        let after = source.metadata()?;
        let still_named = source_parent.symlink_metadata(dest).is_ok_and(|entry| same_file(&entry, &after));
        ops.publish(parent, &temp, dest)?;
        Ok((observation, still_named && metadata_stable(&before, &after)))
    })();
    let _ = parent.remove_file(&temp);
    result
}

fn observe_open_file(file: &File) -> io::Result<Observation> {
    let mut file = file.try_clone()?;
    file.rewind()?;
    let hex = biscuit_hash::blake3_hash_reader(&mut file)?;
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    }
    Ok(Observation { kind: Kind::File, size: file.metadata()?.len(), digest: Some(digest) })
}

fn link_observation(target: &Path) -> io::Result<Observation> {
    #[cfg(unix)]
    let bytes = {
        use std::os::unix::ffi::OsStrExt;
        target.as_os_str().as_bytes().to_vec()
    };
    #[cfg(windows)]
    let bytes = target.as_os_str().to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "link target is not UTF-8"))?
        .as_bytes().to_vec();
    Ok(Observation { kind: Kind::Symlink { target_bytes: bytes.clone() }, size: bytes.len() as u64, digest: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fallback;
    impl CopyOps for Fallback {
        fn clone_file(&self, _: &File, _: &Dir, _: &Path) -> io::Result<File> { Err(io::ErrorKind::Unsupported.into()) }
        fn byte_copy(&self, source: &File, parent: &Dir, dest: &Path) -> io::Result<File> { RealCopyOps.byte_copy(source, parent, dest) }
        fn symlink(&self, target: &Path, parent: &Dir, dest: &Path) -> io::Result<()> { RealCopyOps.symlink(target, parent, dest) }
        fn publish(&self, parent: &Dir, temp: &Path, dest: &Path) -> io::Result<()> { RealCopyOps.publish(parent, temp, dest) }
    }

    #[test]
    fn fallback_copies_without_replacing_and_preserves_independence() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        fs::create_dir(&source).unwrap(); fs::create_dir(&dest).unwrap();
        fs::write(source.join(".env"), b"secret").unwrap();
        let set = IncludeSet { entries: vec![super::super::IncludedEntry { path: b".env".to_vec(), kind: EntryKind::File }], unsupported: vec![] };
        let first = copy_include_set(&source, &dest, &set, &HashSet::new(), &Fallback);
        assert_eq!(first.copied.len(), 1);
        assert!(first.copied[0].1.as_ref().unwrap().digest.is_some());
        fs::write(dest.join(".env"), b"changed").unwrap();
        assert_eq!(fs::read(source.join(".env")).unwrap(), b"secret");
        let second = copy_include_set(&source, &dest, &set, &HashSet::new(), &Fallback);
        assert_eq!(second.skipped, vec![(b".env".to_vec(), SkipReason::Existing)]);
        assert_eq!(fs::read(dest.join(".env")).unwrap(), b"changed");
    }

    #[test]
    fn failed_clone_cleans_partial_temp_and_keeps_destination_absent() {
        struct PartialFailure;
        impl CopyOps for PartialFailure {
            fn clone_file(&self, _: &File, parent: &Dir, dest: &Path) -> io::Result<File> {
                parent.write(dest, b"partial")?;
                Err(io::Error::other("failed clone"))
            }
            fn byte_copy(&self, _: &File, _: &Dir, _: &Path) -> io::Result<File> { panic!("no fallback") }
            fn symlink(&self, _: &Path, _: &Dir, _: &Path) -> io::Result<()> { unreachable!() }
            fn publish(&self, _: &Dir, _: &Path, _: &Path) -> io::Result<()> { unreachable!() }
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        fs::create_dir(&source).unwrap(); fs::create_dir(&dest).unwrap();
        fs::write(source.join(".env"), b"secret").unwrap();
        let set = IncludeSet { entries: vec![super::super::IncludedEntry { path: b".env".to_vec(), kind: EntryKind::File }], unsupported: vec![] };
        let outcome = copy_include_set(&source, &dest, &set, &HashSet::new(), &PartialFailure);
        assert_eq!(outcome.failed.len(), 1);
        assert!(!dest.join(".env").exists());
        assert_eq!(fs::read_dir(&dest).unwrap().count(), 0);
    }

    #[test]
    fn index_and_ancestor_conflicts_skip_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        fs::create_dir_all(source.join("nested")).unwrap(); fs::create_dir(&dest).unwrap();
        fs::write(source.join("nested/.env"), b"secret").unwrap();
        let path = b"nested/.env".to_vec();
        let set = IncludeSet { entries: vec![super::super::IncludedEntry { path: path.clone(), kind: EntryKind::File }], unsupported: vec![] };
        let tracked = copy_include_set(&source, &dest, &set, &HashSet::from([path.clone()]), &Fallback);
        assert_eq!(tracked.skipped, vec![(path.clone(), SkipReason::IndexTracked)]);
        fs::write(dest.join("nested"), b"conflict").unwrap();
        let blocked = copy_include_set(&source, &dest, &set, &HashSet::new(), &Fallback);
        assert_eq!(blocked.skipped, vec![(path, SkipReason::UnsafePath)]);
    }

    #[cfg(unix)]
    #[test]
    fn modes_and_dangling_links_are_preserved() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        fs::create_dir(&source).unwrap(); fs::create_dir(&dest).unwrap();
        fs::write(source.join(".env"), b"secret").unwrap();
        fs::set_permissions(source.join(".env"), fs::Permissions::from_mode(0o600)).unwrap();
        symlink("outside", source.join("link.env")).unwrap();
        let set = IncludeSet { entries: vec![
            super::super::IncludedEntry { path: b".env".to_vec(), kind: EntryKind::File },
            super::super::IncludedEntry { path: b"link.env".to_vec(), kind: EntryKind::Symlink },
        ], unsupported: vec![] };
        let outcome = copy_include_set(&source, &dest, &set, &HashSet::new(), &Fallback);
        assert_eq!(outcome.copied.len(), 2);
        assert_eq!(fs::symlink_metadata(dest.join(".env")).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(fs::read_link(dest.join("link.env")).unwrap(), Path::new("outside"));
        let again = copy_include_set(&source, &dest, &set, &HashSet::new(), &Fallback);
        assert!(again.skipped.contains(&(b"link.env".to_vec(), SkipReason::Existing)));
    }

    #[test]
    fn clone_success_does_not_call_byte_copy() {
        struct CloneSuccess;
        impl CopyOps for CloneSuccess {
            fn clone_file(&self, source: &File, parent: &Dir, dest: &Path) -> io::Result<File> { RealCopyOps.byte_copy(source, parent, dest) }
            fn byte_copy(&self, _: &File, _: &Dir, _: &Path) -> io::Result<File> { panic!("unneeded byte copy") }
            fn symlink(&self, _: &Path, _: &Dir, _: &Path) -> io::Result<()> { unreachable!() }
            fn publish(&self, parent: &Dir, temp: &Path, dest: &Path) -> io::Result<()> { RealCopyOps.publish(parent, temp, dest) }
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        fs::create_dir(&source).unwrap(); fs::create_dir(&dest).unwrap();
        fs::write(source.join(".env"), b"secret").unwrap();
        let set = IncludeSet { entries: vec![super::super::IncludedEntry { path: b".env".to_vec(), kind: EntryKind::File }], unsupported: vec![] };
        let outcome = copy_include_set(&source, &dest, &set, &HashSet::new(), &CloneSuccess);
        assert_eq!(outcome.copied.len(), 1);
        assert_eq!(fs::read(dest.join(".env")).unwrap(), b"secret");
    }

    #[test]
    fn filesystem_copy_succeeds_or_reports_clone_capability() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        fs::create_dir(&source).unwrap(); fs::create_dir(&dest).unwrap();
        fs::write(source.join(".env"), b"secret").unwrap();
        let clone_capability = reflink_copy::reflink(source.join(".env"), dir.path().join("probe"));
        if let Err(error) = &clone_capability { eprintln!("clone capability unavailable: {error}"); }
        let set = IncludeSet { entries: vec![super::super::IncludedEntry { path: b".env".to_vec(), kind: EntryKind::File }], unsupported: vec![] };
        let outcome = copy_include_set(&source, &dest, &set, &HashSet::new(), &RealCopyOps);
        assert!(outcome.failed.is_empty(), "{outcome:?}");
        assert_eq!(fs::read(dest.join(".env")).unwrap(), b"secret");
    }

    #[test]
    fn windows_privilege_error_is_a_link_skip() {
        assert!(link_unsupported(&io::Error::from_raw_os_error(1314)));
        assert!(unsupported_clone(&io::Error::from_raw_os_error(-2147024895)));
    }

    #[cfg(unix)]
    #[test]
    fn source_ancestor_link_is_never_followed() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("outside");
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        fs::create_dir(&outside).unwrap(); fs::create_dir(&source).unwrap(); fs::create_dir(&dest).unwrap();
        fs::write(outside.join(".env"), b"secret").unwrap();
        symlink(&outside, source.join("linked")).unwrap();
        let set = IncludeSet { entries: vec![super::super::IncludedEntry { path: b"linked/.env".to_vec(), kind: EntryKind::File }], unsupported: vec![] };
        let outcome = copy_include_set(&source, &dest, &set, &HashSet::new(), &Fallback);
        assert_eq!(outcome.skipped, vec![(b"linked/.env".to_vec(), SkipReason::UnsafePath)]);
        assert!(!dest.join("linked/.env").exists());
    }

    #[test]
    fn source_change_during_copy_has_no_trusted_baseline() {
        struct ChangingSource(PathBuf);
        impl CopyOps for ChangingSource {
            fn clone_file(&self, source: &File, parent: &Dir, dest: &Path) -> io::Result<File> {
                let copied = RealCopyOps.byte_copy(source, parent, dest)?;
                fs::write(&self.0, b"newer-longer")?;
                Ok(copied)
            }
            fn byte_copy(&self, _: &File, _: &Dir, _: &Path) -> io::Result<File> { unreachable!() }
            fn symlink(&self, _: &Path, _: &Dir, _: &Path) -> io::Result<()> { unreachable!() }
            fn publish(&self, parent: &Dir, temp: &Path, dest: &Path) -> io::Result<()> { RealCopyOps.publish(parent, temp, dest) }
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        fs::create_dir(&source).unwrap(); fs::create_dir(&dest).unwrap();
        fs::write(source.join(".env"), b"secret").unwrap();
        let set = IncludeSet { entries: vec![super::super::IncludedEntry { path: b".env".to_vec(), kind: EntryKind::File }], unsupported: vec![] };
        let outcome = copy_include_set(&source, &dest, &set, &HashSet::new(), &ChangingSource(source.join(".env")));
        assert_eq!(outcome.copied, vec![(b".env".to_vec(), None)]);
        assert_eq!(fs::read(dest.join(".env")).unwrap(), b"secret");
        assert_eq!(outcome.warnings.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn denied_link_creation_is_skipped() {
        use std::os::unix::fs::symlink;
        struct DeniedLink;
        impl CopyOps for DeniedLink {
            fn clone_file(&self, _: &File, _: &Dir, _: &Path) -> io::Result<File> { unreachable!() }
            fn byte_copy(&self, _: &File, _: &Dir, _: &Path) -> io::Result<File> { unreachable!() }
            fn symlink(&self, _: &Path, _: &Dir, _: &Path) -> io::Result<()> {
                Err(io::Error::from_raw_os_error(1314))
            }
            fn publish(&self, _: &Dir, _: &Path, _: &Path) -> io::Result<()> { unreachable!() }
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        fs::create_dir(&source).unwrap(); fs::create_dir(&dest).unwrap();
        symlink("outside", source.join("link.env")).unwrap();
        let set = IncludeSet { entries: vec![super::super::IncludedEntry { path: b"link.env".to_vec(), kind: EntryKind::Symlink }], unsupported: vec![] };
        let outcome = copy_include_set(&source, &dest, &set, &HashSet::new(), &DeniedLink);
        assert_eq!(outcome.skipped, vec![(b"link.env".to_vec(), SkipReason::LinkUnsupported)]);
        assert!(!dest.join("link.env").exists());
    }

    #[cfg(unix)]
    #[test]
    fn destination_ancestor_link_is_never_followed() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        let outside = dir.path().join("outside");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::create_dir(&dest).unwrap(); fs::create_dir(&outside).unwrap();
        fs::write(source.join("nested/.env"), b"secret").unwrap();
        symlink(&outside, dest.join("nested")).unwrap();
        let set = IncludeSet { entries: vec![super::super::IncludedEntry { path: b"nested/.env".to_vec(), kind: EntryKind::File }], unsupported: vec![] };
        let outcome = copy_include_set(&source, &dest, &set, &HashSet::new(), &Fallback);
        assert_eq!(outcome.skipped, vec![(b"nested/.env".to_vec(), SkipReason::UnsafePath)]);
        assert!(!outside.join(".env").exists());
    }

    #[cfg(unix)]
    #[test]
    fn source_swap_after_validation_copies_only_opened_file() {
        use std::os::unix::fs::symlink;
        struct SwapSource { path: PathBuf, outside: PathBuf }
        impl CopyOps for SwapSource {
            fn clone_file(&self, source: &File, parent: &Dir, temp: &Path) -> io::Result<File> {
                fs::rename(&self.path, self.path.with_extension("original"))?;
                symlink(&self.outside, &self.path)?;
                RealCopyOps.byte_copy(source, parent, temp)
            }
            fn byte_copy(&self, _: &File, _: &Dir, _: &Path) -> io::Result<File> { unreachable!() }
            fn symlink(&self, _: &Path, _: &Dir, _: &Path) -> io::Result<()> { unreachable!() }
            fn publish(&self, parent: &Dir, temp: &Path, dest: &Path) -> io::Result<()> { RealCopyOps.publish(parent, temp, dest) }
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        let outside = dir.path().join("outside-secret");
        fs::create_dir(&source).unwrap(); fs::create_dir(&dest).unwrap();
        fs::write(source.join(".env"), b"inside").unwrap();
        fs::write(&outside, b"outside").unwrap();
        let set = IncludeSet { entries: vec![super::super::IncludedEntry { path: b".env".to_vec(), kind: EntryKind::File }], unsupported: vec![] };
        let outcome = copy_include_set(&source, &dest, &set, &HashSet::new(),
            &SwapSource { path: source.join(".env"), outside: outside.clone() });
        assert!(outcome.failed.is_empty(), "{outcome:?}");
        assert_eq!(outcome.copied, vec![(b".env".to_vec(), None)]);
        assert_eq!(outcome.warnings.len(), 1);
        assert_eq!(fs::read(dest.join(".env")).unwrap(), b"inside");
        assert_eq!(fs::read(outside).unwrap(), b"outside");
    }

    #[cfg(unix)]
    #[test]
    fn destination_parent_swap_never_creates_an_outside_file() {
        use std::os::unix::fs::symlink;
        struct SwapParent { parent: PathBuf, outside: PathBuf }
        impl CopyOps for SwapParent {
            fn clone_file(&self, source: &File, parent: &Dir, temp: &Path) -> io::Result<File> {
                fs::rename(&self.parent, self.parent.with_file_name("moved"))?;
                symlink(&self.outside, &self.parent)?;
                RealCopyOps.byte_copy(source, parent, temp)
            }
            fn byte_copy(&self, _: &File, _: &Dir, _: &Path) -> io::Result<File> { unreachable!() }
            fn symlink(&self, _: &Path, _: &Dir, _: &Path) -> io::Result<()> { unreachable!() }
            fn publish(&self, parent: &Dir, temp: &Path, dest: &Path) -> io::Result<()> { RealCopyOps.publish(parent, temp, dest) }
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        let outside = dir.path().join("outside");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::create_dir_all(dest.join("nested")).unwrap(); fs::create_dir(&outside).unwrap();
        fs::write(source.join("nested/.env"), b"inside").unwrap();
        let set = IncludeSet { entries: vec![super::super::IncludedEntry { path: b"nested/.env".to_vec(), kind: EntryKind::File }], unsupported: vec![] };
        let outcome = copy_include_set(&source, &dest, &set, &HashSet::new(),
            &SwapParent { parent: dest.join("nested"), outside: outside.clone() });
        assert!(outcome.failed.is_empty(), "{outcome:?}");
        assert!(!outside.join(".env").exists());
        assert_eq!(fs::read(dest.join("moved/.env")).unwrap(), b"inside");
    }
}
