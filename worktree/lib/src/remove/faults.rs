//! Filesystem failures injected into removal's deletion, so tests can make
//! an entry read-only, access-denied, held open by another program, or a
//! directory that refills, on any host and without privileges.

use std::io;
use std::path::Path;

/// A filesystem operation deletion performs on one entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FsOp {
    ReadDir,
    RemoveFile,
    RemoveDir,
    /// Clearing a read-only entry's flag so it can be deleted.
    MakeWritable,
    Rename,
}

/// Consulted before each [`FsOp`]. An injected error is returned in place of
/// the operation's own result, and the operation is not performed.
pub trait FsFaults: Sync {
    fn inject(&self, op: FsOp, path: &Path) -> Option<io::Error>;
}

/// [`FsFaults`] that injects nothing: the production value.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFaults;

impl FsFaults for NoFaults {
    fn inject(&self, _op: FsOp, _path: &Path) -> Option<io::Error> {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::remove::test_support::ScriptedFaults;

    #[test]
    fn no_faults_injects_nothing() {
        assert!(NoFaults.inject(FsOp::RemoveFile, Path::new("a")).is_none());
    }

    /// A temporary fault clears after its count; a permanent one never does,
    /// and neither touches another operation or path.
    #[test]
    fn scripted_faults_fire_for_their_operation_and_path_only() {
        let held = PathBuf::from("held.txt");
        let locked = PathBuf::from("locked.txt");
        let faults = ScriptedFaults::new()
            .times(FsOp::RemoveFile, &held, 2, || io::Error::from(io::ErrorKind::ResourceBusy))
            .always(FsOp::RemoveFile, &locked, || io::Error::from(io::ErrorKind::PermissionDenied));

        let kinds = |path: &Path| -> Vec<Option<io::ErrorKind>> { (0..3).map(|_| faults.inject(FsOp::RemoveFile, path).map(|error| error.kind())).collect() };

        assert_eq!(kinds(&held), [Some(io::ErrorKind::ResourceBusy), Some(io::ErrorKind::ResourceBusy), None]);
        assert_eq!(kinds(&locked), [Some(io::ErrorKind::PermissionDenied); 3]);
        assert!(faults.inject(FsOp::RemoveDir, &locked).is_none());
        assert!(faults.inject(FsOp::RemoveFile, Path::new("other.txt")).is_none());
        assert_eq!(faults.fired(FsOp::RemoveFile, &held), 2);
    }
}
