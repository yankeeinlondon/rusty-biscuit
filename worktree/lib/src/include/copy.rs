//! Copy included entries without replacing destination content.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use crate::compare::{observe, FilesystemReader, Observation};
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
    fn clone_file(&self, source: &Path, dest: &Path) -> io::Result<()>;
    fn byte_copy(&self, source: &Path, dest: &Path) -> io::Result<()>;
    fn symlink(&self, target: &Path, dest: &Path) -> io::Result<()>;
    fn publish(&self, temp: &Path, dest: &Path) -> io::Result<()>;
}

pub struct RealCopyOps;
impl CopyOps for RealCopyOps {
    fn clone_file(&self, source: &Path, dest: &Path) -> io::Result<()> {
        reflink_copy::reflink(source, dest)
    }
    fn byte_copy(&self, source: &Path, dest: &Path) -> io::Result<()> {
        fs::copy(source, dest).map(|_| ())
    }
    fn symlink(&self, target: &Path, dest: &Path) -> io::Result<()> {
        #[cfg(unix)] { std::os::unix::fs::symlink(target, dest) }
        #[cfg(windows)] { std::os::windows::fs::symlink_file(target, dest) }
    }
    fn publish(&self, temp: &Path, dest: &Path) -> io::Result<()> {
        #[cfg(unix)]
        { fs::hard_link(temp, dest) }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
            let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = dest.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0) } == 0 {
                Err(io::Error::last_os_error())
            } else { Ok(()) }
        }
    }
}

fn temp_path(dest: &Path) -> io::Result<PathBuf> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| io::Error::other(e.to_string()))?;
    let name: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(dest.with_file_name(format!(".wt-copy-{name}.tmp")))
}

fn exists_even_dangling(path: &Path) -> bool { fs::symlink_metadata(path).is_ok() }

fn safe_parent(root: &Path, relative: &Path) -> io::Result<bool> {
    let Some(parent) = relative.parent() else { return Ok(true); };
    let mut current = root.to_path_buf();
    for part in parent.components() {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                #[cfg(windows)] {
                    use std::os::windows::fs::MetadataExt;
                    if metadata.file_attributes() & 0x400 != 0 { return Ok(false); }
                }
            }
            Ok(_) => return Ok(false),
            Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(&current)?,
            Err(error) => return Err(error),
        }
    }
    Ok(true)
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

pub fn copy_include_set(source_root: &Path, dest_root: &Path, set: &IncludeSet,
    dest_index: &HashSet<Vec<u8>>, ops: &dyn CopyOps) -> CopyOutcome {
    let mut outcome = CopyOutcome::default();
    for entry in &set.entries {
        let path = &entry.path;
        if dest_index.contains(path) {
            outcome.skipped.push((path.clone(), SkipReason::IndexTracked)); continue;
        }
        let relative = match relative_path(path) {
            Ok(relative) => relative,
            Err(error) => { outcome.failed.push((path.clone(), error.to_string())); continue; }
        };
        let source = source_root.join(&relative);
        let dest = dest_root.join(&relative);
        match guarded_kind(source_root, &relative) {
            Ok(Some(kind)) if kind == entry.kind => {},
            Ok(_) => { outcome.skipped.push((path.clone(), SkipReason::UnsafePath)); continue; },
            Err(error) => { outcome.failed.push((path.clone(), error.to_string())); continue; },
        }
        match safe_parent(dest_root, &relative) {
            Ok(true) => {},
            Ok(false) => { outcome.skipped.push((path.clone(), SkipReason::UnsafePath)); continue; },
            Err(error) => { outcome.failed.push((path.clone(), error.to_string())); continue; },
        }
        if exists_even_dangling(&dest) {
            outcome.skipped.push((path.clone(), SkipReason::Existing)); continue;
        }
        match entry.kind {
            EntryKind::Symlink => {
                let target = fs::read_link(&source);
                let result = target.as_ref().map_err(|error| io::Error::new(error.kind(), error.to_string()))
                    .and_then(|target| ops.symlink(target, &dest));
                match result {
                    Ok(()) => match observe(&dest, &FilesystemReader) {
                        Ok(observation) => {
                            let stable = fs::read_link(&source).ok().as_ref() == target.as_ref().ok();
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
                let result = copy_file(&source, &dest, ops);
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

fn copy_file(source: &Path, dest: &Path, ops: &dyn CopyOps) -> io::Result<(Observation, bool)> {
    let before = fs::symlink_metadata(source)?;
    let mut temp = temp_path(dest)?;
    let result = (|| {
        if let Err(error) = ops.clone_file(source, &temp) {
            if !unsupported_clone(&error) { return Err(error); }
            let _ = fs::remove_file(&temp);
            temp = temp_path(dest)?;
            ops.byte_copy(source, &temp)?;
        }
        fs::set_permissions(&temp, before.permissions())?;
        let observation = observe(&temp, &FilesystemReader)?;
        let after = fs::symlink_metadata(source)?;
        ops.publish(&temp, dest)?;
        Ok((observation, metadata_stable(&before, &after)))
    })();
    let _ = fs::remove_file(&temp);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fallback;
    impl CopyOps for Fallback {
        fn clone_file(&self, _: &Path, _: &Path) -> io::Result<()> { Err(io::ErrorKind::Unsupported.into()) }
        fn byte_copy(&self, source: &Path, dest: &Path) -> io::Result<()> { RealCopyOps.byte_copy(source, dest) }
        fn symlink(&self, target: &Path, dest: &Path) -> io::Result<()> { RealCopyOps.symlink(target, dest) }
        fn publish(&self, temp: &Path, dest: &Path) -> io::Result<()> { RealCopyOps.publish(temp, dest) }
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
            fn clone_file(&self, _: &Path, dest: &Path) -> io::Result<()> {
                fs::write(dest, b"partial")?;
                Err(io::Error::other("failed clone"))
            }
            fn byte_copy(&self, _: &Path, _: &Path) -> io::Result<()> { panic!("no fallback") }
            fn symlink(&self, _: &Path, _: &Path) -> io::Result<()> { unreachable!() }
            fn publish(&self, _: &Path, _: &Path) -> io::Result<()> { unreachable!() }
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
            fn clone_file(&self, source: &Path, dest: &Path) -> io::Result<()> { fs::copy(source, dest).map(|_| ()) }
            fn byte_copy(&self, _: &Path, _: &Path) -> io::Result<()> { panic!("unneeded byte copy") }
            fn symlink(&self, _: &Path, _: &Path) -> io::Result<()> { unreachable!() }
            fn publish(&self, temp: &Path, dest: &Path) -> io::Result<()> { RealCopyOps.publish(temp, dest) }
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
        struct ChangingSource;
        impl CopyOps for ChangingSource {
            fn clone_file(&self, source: &Path, dest: &Path) -> io::Result<()> {
                fs::copy(source, dest)?;
                fs::write(source, b"newer-longer")
            }
            fn byte_copy(&self, _: &Path, _: &Path) -> io::Result<()> { unreachable!() }
            fn symlink(&self, _: &Path, _: &Path) -> io::Result<()> { unreachable!() }
            fn publish(&self, temp: &Path, dest: &Path) -> io::Result<()> { RealCopyOps.publish(temp, dest) }
        }
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source"); let dest = dir.path().join("dest");
        fs::create_dir(&source).unwrap(); fs::create_dir(&dest).unwrap();
        fs::write(source.join(".env"), b"secret").unwrap();
        let set = IncludeSet { entries: vec![super::super::IncludedEntry { path: b".env".to_vec(), kind: EntryKind::File }], unsupported: vec![] };
        let outcome = copy_include_set(&source, &dest, &set, &HashSet::new(), &ChangingSource);
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
            fn clone_file(&self, _: &Path, _: &Path) -> io::Result<()> { unreachable!() }
            fn byte_copy(&self, _: &Path, _: &Path) -> io::Result<()> { unreachable!() }
            fn symlink(&self, _: &Path, _: &Path) -> io::Result<()> {
                Err(io::Error::from_raw_os_error(1314))
            }
            fn publish(&self, _: &Path, _: &Path) -> io::Result<()> { unreachable!() }
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
}
