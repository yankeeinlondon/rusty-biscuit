//! Publication fixture for Playa's detached audio spool.
//!
//! Tests that assert on a durable audio record must not leave that record
//! runnable. Holding `worker.lock` stops the scheduler, but it does not stop a
//! publisher or a preparation helper: `playa::detached` commits and replaces
//! records under `queue.lock`. Cleanup that scans the spool without that lock
//! can iterate past a record another actor is in the middle of committing, and
//! then release worker ownership over work that is still runnable — which is
//! how a "silent" test becomes audible after it ends.
//!
//! [`LockedAudioSpool`] takes both locks in the order the scheduler's
//! final-empty path uses: `worker.lock` for the fixture's whole lifetime, and
//! `queue.lock` around every scan and removal.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;

/// Suffix Playa gives a durable, not-yet-claimed spool record.
const PENDING_SUFFIX: &str = ".pending.json";

/// Owns a private spool root and the worker exclusion over it, and removes
/// every runnable record under `queue.lock` before that ownership is released.
///
/// The fixture blocks until it owns `worker.lock`, so no scheduler can execute
/// a record while it is alive. [`clear_pending`](Self::clear_pending) runs on
/// [`Drop`] as well, including while a test is unwinding, so a failed assertion
/// cannot strand runnable audio.
///
/// ## Examples
///
/// ```no_run
/// # use test_toolkit::LockedAudioSpool;
/// let temp = tempfile::tempdir().unwrap();
/// let spool = LockedAudioSpool::new(&temp.path().join("spool"));
/// // ... publish and assert on a durable record ...
/// spool.clear_pending().expect("pending records removed under the queue lock");
/// ```
///
/// ## Panics
///
/// [`new`](Self::new) panics when the spool root cannot be created or its
/// worker lock cannot be taken. `Drop` panics when cleanup fails and the thread
/// is not already panicking; when it is, the cleanup error is discarded so the
/// original failure is the one reported.
#[derive(Debug)]
pub struct LockedAudioSpool {
    root: PathBuf,
    _worker: File,
}

impl LockedAudioSpool {
    /// Create `root` as a private spool directory and take exclusive worker
    /// ownership of it.
    ///
    /// On Unix the directory is created with mode `0o700`, which Playa's spool
    /// validation requires. An existing directory is reused.
    ///
    /// ## Panics
    ///
    /// Panics when the directory cannot be created or the worker lock cannot be
    /// acquired.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        fs::create_dir_all(root).expect("spool root should create");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(root, fs::Permissions::from_mode(0o700))
                .expect("spool root should become private");
        }
        let worker =
            Self::open_lock(&Self::worker_lock_path(root)).expect("worker lock should open");
        worker
            .lock_exclusive()
            .expect("fixture should own the spool worker lock");
        Self {
            root: root.to_path_buf(),
            _worker: worker,
        }
    }

    /// Spool root this fixture owns.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path of the lock that serializes scheduler execution over `root`.
    #[must_use]
    pub fn worker_lock_path(root: &Path) -> PathBuf {
        root.join("worker.lock")
    }

    /// Path of the lock that serializes record publication and replacement
    /// over `root`.
    #[must_use]
    pub fn queue_lock_path(root: &Path) -> PathBuf {
        root.join("queue.lock")
    }

    /// Open a spool lock file with the flags Playa's detached protocol uses.
    ///
    /// Exposed so a test can take `queue.lock` the same way a publisher does,
    /// or probe `worker.lock` to prove the fixture released it.
    ///
    /// ## Errors
    ///
    /// Returns the underlying [`io::Error`] when the lock file cannot be opened
    /// or created.
    pub fn open_lock(path: &Path) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
    }

    /// Remove every runnable record from the spool while holding both locks.
    ///
    /// `queue.lock` is acquired before the directory is scanned and released
    /// only after the last removal, so a publisher or preparation helper can
    /// neither commit behind the scan nor replace a record during it. The
    /// already-held `worker.lock` keeps execution impossible throughout.
    ///
    /// ## Errors
    ///
    /// Returns the underlying [`io::Error`] when the queue lock cannot be taken
    /// or a record cannot be read or removed.
    pub fn clear_pending(&self) -> io::Result<()> {
        let queue = Self::open_lock(&Self::queue_lock_path(&self.root))?;
        queue.lock_exclusive()?;
        let result = (|| {
            for entry in fs::read_dir(&self.root)? {
                let entry = entry?;
                if entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(PENDING_SUFFIX)
                {
                    fs::remove_file(entry.path())?;
                }
            }
            Ok(())
        })();
        let unlock = FileExt::unlock(&queue);
        result.and(unlock)
    }
}

impl Drop for LockedAudioSpool {
    fn drop(&mut self) {
        let result = self.clear_pending();
        // A cleanup failure while unwinding must not abort the process, and it
        // must not mask the assertion that started the unwind.
        if !std::thread::panicking() {
            result.expect("remove pending audio before releasing the fixture worker lock");
        }
    }
}
