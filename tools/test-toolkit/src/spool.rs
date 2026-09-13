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
//! [`LockedAudioSpool`] reproduces the ordering of the scheduler's final-empty
//! handoff (`playa::detached::run_scheduler_with`): it owns `worker.lock` for
//! its whole lifetime, and destruction is one `queue.lock` critical section
//! that removes every runnable record, releases `worker.lock` while
//! `queue.lock` is still held, and only then releases `queue.lock`.
//!
//! That ordering is what closes the last window. A publisher commits under
//! `queue.lock` and then probes `worker.lock` to decide whether an active
//! worker will pick the record up. Against this fixture it can therefore only
//! see one of two states: worker ownership held, in which case its record was
//! committed before the fixture's scan and the scan removes it; or worker
//! ownership free, in which case the spool is already empty. A fresh record and
//! released worker ownership never coexist.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;

/// Suffix Playa gives a durable, not-yet-claimed spool record.
const PENDING_SUFFIX: &str = ".pending.json";

/// Callback a test can run inside the destruction critical section.
type HandoffObserver = Box<dyn Fn() + Send + Sync + 'static>;

/// Owns a private spool root and the worker exclusion over it, and removes
/// every runnable record under `queue.lock` before that ownership is released.
///
/// The fixture blocks until it owns `worker.lock`, so no scheduler can execute
/// a record while it is alive. Destruction repeats the cleanup — including
/// while a test is unwinding — as one `queue.lock` critical section ending in
/// the release of worker ownership, so neither a failed assertion nor a
/// concurrent publisher can strand runnable audio.
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
/// worker lock cannot be taken. `Drop` reports a cleanup failure on stderr and
/// then panics, unless the thread is already unwinding — a second panic there
/// would abort the process and destroy the original failure. Either way a
/// failed cleanup keeps worker ownership rather than releasing it over a record
/// it could not remove.
pub struct LockedAudioSpool {
    root: PathBuf,
    /// `None` once worker ownership has been released, which happens inside the
    /// destruction critical section rather than through this field's drop.
    worker: Option<File>,
    handoff: Option<HandoffObserver>,
}

impl fmt::Debug for LockedAudioSpool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LockedAudioSpool")
            .field("root", &self.root)
            .field("owns_worker", &self.worker.is_some())
            .field("observed_handoff", &self.handoff.is_some())
            .finish()
    }
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
            worker: Some(worker),
            handoff: None,
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

    /// Run `observer` inside the destruction critical section, after the last
    /// runnable record has been removed and before worker ownership is
    /// released, with both `queue.lock` and `worker.lock` still held.
    ///
    /// This is a seam for one regression that cannot be written from outside.
    /// The handoff it guards is two adjacent unlock calls, so a thread waiting
    /// on `queue.lock` is granted it and resumed far later than the nanoseconds
    /// the wrong order would leave open: an external observer would report the
    /// broken ordering only occasionally. Asking from inside makes the
    /// observation deterministic.
    ///
    /// `observer` must not block on `queue.lock` or `worker.lock`, both of
    /// which this fixture holds while it runs; probe them instead. It must also
    /// return, since destruction cannot finish until it does — give any wait
    /// for another thread a deadline.
    pub fn observe_handoff(&mut self, observer: impl Fn() + Send + Sync + 'static) {
        self.handoff = Some(Box::new(observer));
    }

    /// Remove every runnable record from the spool while holding both locks.
    ///
    /// `queue.lock` is acquired before the directory is scanned and released
    /// only after the last removal, so a publisher or preparation helper can
    /// neither commit behind the scan nor replace a record during it. The
    /// already-held `worker.lock` keeps execution impossible throughout, and is
    /// retained: this is the mid-test cleanup, not the handoff, which happens
    /// on [`Drop`].
    ///
    /// ## Errors
    ///
    /// Returns the underlying [`io::Error`] when the queue lock cannot be taken
    /// or a record cannot be read or removed.
    pub fn clear_pending(&self) -> io::Result<()> {
        let queue = Self::open_lock(&Self::queue_lock_path(&self.root))?;
        queue.lock_exclusive()?;
        let result = Self::remove_runnable_records(&self.root);
        let unlock = FileExt::unlock(&queue);
        result.and(unlock)
    }

    /// Clear the spool and release worker ownership as one `queue.lock`
    /// critical section.
    ///
    /// On success worker ownership is released and `self.worker` is `None`; on
    /// failure it is untouched, because releasing it would expose whatever the
    /// scan could not remove.
    fn clear_pending_and_release_worker(&mut self) -> io::Result<()> {
        let queue = Self::open_lock(&Self::queue_lock_path(&self.root))?;
        queue.lock_exclusive()?;

        let mut result = Self::remove_runnable_records(&self.root);
        if result.is_ok() {
            if let Some(observer) = self.handoff.take() {
                observer();
            }
            // `queue.lock` is deliberately still held here. A publisher decides
            // whether to spawn a scheduler by probing `worker.lock` while it
            // owns `queue.lock`, so releasing worker ownership outside that
            // critical section would let it commit a record, see this fixture
            // as the worker responsible for it, and leave it runnable. This
            // mirrors `run_scheduler_with`'s final-empty handoff.
            if let Some(worker) = self.worker.take() {
                result = FileExt::unlock(&worker);
            }
        }

        let unlock = FileExt::unlock(&queue);
        result.and(unlock)
    }

    fn remove_runnable_records(root: &Path) -> io::Result<()> {
        for entry in fs::read_dir(root)? {
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
    }
}

impl Drop for LockedAudioSpool {
    fn drop(&mut self) {
        let unwinding = std::thread::panicking();
        let Err(error) = self.clear_pending_and_release_worker() else {
            return;
        };

        // Cleanup failed, so the spool may still hold a runnable record. Two
        // things follow, both counter-intuitive enough to state here.
        //
        // The error is written to stderr rather than only raised: a panic while
        // the thread is already unwinding aborts the process and destroys the
        // failure that started the unwind, and a discarded error would let the
        // handoff fail in silence. Nextest attributes this output to the test.
        //
        // Worker ownership is then leaked instead of released. Dropping the
        // `File` would close its descriptor and hand the spool to the next
        // scheduler, which is exactly the outcome the fixture exists to
        // prevent; keeping it alive for the rest of the process keeps whatever
        // survived unreachable. The cost is Windows-only and confined to this
        // already-failing path: an open handle blocks deletion of the lock
        // file, so the test's spool directory may outlive the run.
        eprintln!(
            "LockedAudioSpool: cleanup of {} failed ({error}); worker ownership retained so any \
             surviving record stays unreachable",
            self.root.display()
        );
        std::mem::forget(self.worker.take());

        if !unwinding {
            panic!(
                "failed to remove pending audio before releasing the fixture worker lock: {error}"
            );
        }
    }
}
