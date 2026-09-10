use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt as _;

/// Publication-only fixtures own the scheduler lock until all runnable jobs are removed.
/// The directory remains in place until both lock handles have closed on Windows.
pub(super) struct LockedAudioSpool {
    root: PathBuf,
    _worker: File,
}

impl LockedAudioSpool {
    pub(super) fn new(root: &Path) -> Self {
        fs::create_dir_all(root).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let worker = Self::open_lock(&root.join("worker.lock")).unwrap();
        worker.lock_exclusive().unwrap();
        Self { root: root.to_path_buf(), _worker: worker }
    }

    fn open_lock(path: &Path) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
    }

    pub(super) fn clear_pending(&self) -> io::Result<()> {
        // Match the scheduler's final-empty path: queue lock protects removal,
        // while the already-held worker lock prevents execution throughout.
        let queue = Self::open_lock(&self.root.join("queue.lock"))?;
        queue.lock_exclusive()?;
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().ends_with(".pending.json") {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }
}

impl Drop for LockedAudioSpool {
    fn drop(&mut self) {
        let result = self.clear_pending();
        if !std::thread::panicking() {
            result.expect("remove pending audio before releasing the fixture worker lock");
        }
    }
}

#[test]
fn publication_fixture_clears_jobs_before_unlocking_on_unwind() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("spool");
    let pending = root.join("fixture.pending.json");
    let result = std::panic::catch_unwind(|| {
        let _spool = LockedAudioSpool::new(&root);
        fs::write(&pending, b"inert fixture job").unwrap();
        panic!("simulated assertion failure");
    });
    assert!(result.is_err());
    assert!(!pending.exists());
    let worker = LockedAudioSpool::open_lock(&root.join("worker.lock")).unwrap();
    assert!(worker.try_lock_exclusive().unwrap());
    drop(worker);
    fs::remove_dir_all(&root).unwrap();
}
