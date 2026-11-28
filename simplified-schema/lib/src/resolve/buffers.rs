//! Unsaved editor texts that schema resolution reads in place of files.
//!
//! An editor holds schema sources the author has not saved: a definition, a
//! file it imports, a `$schema` target, or a file that exists only in the
//! editor. [`with`] runs a resolution with a [`SourceBuffers`] snapshot in
//! force on the calling thread, so every schema source the resolver reads (a
//! `$schema` target, a `Name@file` import, an `example(file)`, and the spans
//! an error re-projects) comes from the buffer when one is open at that path,
//! and a reference selects an open buffer at a probed location that holds no
//! file yet. Without a snapshot in force, resolution reads only the disk.
//!
//! The snapshot is plain data (`Send + Sync`): the host captures it once per
//! preparation and may carry it to whichever thread prepares.
//!
//! [`with_reads_retained`] uses the same snapshot for a second purpose: for
//! the length of one host operation, every file read from the disk is kept as
//! if it were open, so separate resolutions in that operation share one read.
//!
//! [`guarded`] lets a host judge each schema source before it is read (from
//! a buffer or the disk): a refused path fails its read as
//! [`std::io::ErrorKind::PermissionDenied`] and nothing is read.
//!
//! [`SharedReads`] is a host's observation of local files that outlives one
//! resolution: with it in force ([`enter_shared_reads`]), each file the
//! resolver reads from the disk is opened once for the whole observation,
//! and host readers outside the resolver ([`shared_reads`]) read the same
//! bytes.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Open texts keyed by [`SourceBuffers::key`]. Cloning shares the map.
#[derive(Clone, Debug, Default)]
pub struct SourceBuffers {
    texts: Arc<HashMap<PathBuf, Arc<str>>>,
}

impl SourceBuffers {
    /// The form every path is compared in: its nearest existing ancestor
    /// canonicalized and the rest kept as written, so a file that exists only
    /// in the editor still matches the spelling resolution probes it under
    /// (`/var/…` and `/private/var/…` alike).
    #[must_use]
    pub fn key(path: &Path) -> PathBuf {
        let mut rest = Vec::new();
        let mut current = path;
        while let (Some(parent), Some(name)) = (current.parent(), current.file_name()) {
            rest.push(name);
            if let Ok(canonical) = biscuit_file::canonicalize_simplified(parent) {
                return rest.iter().rev().fold(canonical, |path, name| path.join(name));
            }
            current = parent;
        }
        path.to_path_buf()
    }

    /// Records `text` as the open buffer at `path`, returning the text it
    /// replaced.
    pub fn insert(&mut self, path: &Path, text: Arc<str>) -> Option<Arc<str>> {
        Arc::make_mut(&mut self.texts).insert(Self::key(path), text)
    }

    /// Forgets the buffer at `path`, returning its text.
    pub fn remove(&mut self, path: &Path) -> Option<Arc<str>> {
        let key = Self::key(path);
        if !self.texts.contains_key(&key) {
            return None;
        }
        Arc::make_mut(&mut self.texts).remove(&key)
    }

    /// The open text at `path`.
    #[must_use]
    pub fn get(&self, path: &Path) -> Option<&Arc<str>> {
        if self.texts.is_empty() {
            return None;
        }
        self.texts.get(&Self::key(path))
    }

    #[allow(missing_docs)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }

    /// Every open path, in [`Self::key`] form.
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.texts.keys().map(PathBuf::as_path)
    }
}

/// A local file's bytes as one open read them, with the modification time
/// taken from the same handle, so the two never describe different
/// revisions.
#[derive(Clone, Debug)]
pub struct ReadSource {
    /// The file's bytes.
    pub bytes: Arc<[u8]>,
    /// The file's modification time, when the platform reports one.
    pub modified: Option<std::time::SystemTime>,
}

/// Opens the local file at `path` once and reads it whole: the one open
/// every reader that can share a [`SharedReads`] goes through, and the one
/// the `work-counters` feature counts per path
/// (`work_counters::source_opens`).
///
/// ## Errors
///
/// The open's or the read's I/O error.
pub fn open_source(path: &Path) -> std::io::Result<ReadSource> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    #[cfg(feature = "work-counters")]
    crate::work_counters::note_source_open(path);
    if let Some(observer) = OPEN_OBSERVER.get() {
        observer(path);
    }
    let modified = file.metadata().and_then(|metadata| metadata.modified()).ok();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(ReadSource { bytes: Arc::from(bytes), modified })
}

/// The process's [`observe_opens`] observer.
static OPEN_OBSERVER: std::sync::OnceLock<fn(&Path)> = std::sync::OnceLock::new();

/// Installs `observer`, called with the path of every file
/// [`open_source`] opens, for the rest of the process: a host's test seam
/// that counts a binary's opens from outside it. The first installation
/// wins; `false` when one was already installed.
pub fn observe_opens(observer: fn(&Path)) -> bool {
    OPEN_OBSERVER.set(observer).is_ok()
}

/// One invocation's observation of local source files: the first
/// successful read of each file, by [`SourceBuffers::key`], served to every
/// later reader that shares it. Cloning shares the observation.
///
/// A host that checks one invocation in several passes (a dry run) gives
/// every pass the same `SharedReads`, so a prompt, an included file, or a
/// schema the passes all read is opened once and every pass judges the same
/// bytes. A failed read is not kept: the next reader tries the file again.
/// Nothing a reader is refused reaches it: a host's permission check, and
/// any [`guarded`] scope, run before [`SharedReads::read`] is asked. In
/// memory only, for the life of the observation.
#[derive(Clone, Debug, Default)]
pub struct SharedReads {
    held: Arc<std::sync::Mutex<HashMap<PathBuf, ReadSource>>>,
    memos: Arc<Memos>,
}

/// The host memos one [`SharedReads`] carries ([`SharedReads::memo`]), one
/// per type.
#[derive(Default)]
struct Memos(std::sync::Mutex<Vec<Arc<dyn std::any::Any + Send + Sync>>>);

impl std::fmt::Debug for Memos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Memos").finish_non_exhaustive()
    }
}

impl SharedReads {
    /// The bytes of the file at `path`: those an earlier reader of this
    /// observation read, else one [`open_source`] whose result is kept.
    ///
    /// ## Errors
    ///
    /// The open's or the read's I/O error, which is not kept.
    pub fn read(&self, path: &Path) -> std::io::Result<ReadSource> {
        let key = SourceBuffers::key(path);
        if let Some(held) = self.lock().get(&key) {
            return Ok(held.clone());
        }
        let read = open_source(path)?;
        // A concurrent reader that kept its read first wins, so every
        // reader of this observation sees one revision.
        Ok(self.lock().entry(key).or_insert(read).clone())
    }

    /// Whether this observation already holds the file at `path`, so a
    /// [`Self::read`] of it opens nothing.
    #[must_use]
    pub fn holds(&self, path: &Path) -> bool {
        self.lock().contains_key(&SourceBuffers::key(path))
    }

    /// Whether two handles share one observation.
    #[must_use]
    pub fn same_reads(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.held, &other.held)
    }

    /// The observation's one `T`, made on first request: a host's memo of
    /// work derived from the bytes this observation serves (a prepared
    /// schema), which lives exactly as long as those bytes are the ones
    /// every reader sees. Every handle of the observation shares it.
    #[must_use]
    pub fn memo<T: std::any::Any + Send + Sync + Default>(&self) -> Arc<T> {
        let mut memos = self.memos.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(held) = memos.iter().find_map(|memo| Arc::clone(memo).downcast::<T>().ok()) {
            return held;
        }
        let memo = Arc::new(T::default());
        memos.push(Arc::clone(&memo) as Arc<dyn std::any::Any + Send + Sync>);
        memo
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<PathBuf, ReadSource>> {
        self.held.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Restores the [`SharedReads`] in force before [`enter_shared_reads`] when
/// dropped.
#[must_use = "the scope ends when the guard is dropped"]
pub struct SharedReadsScope {
    previous: Option<SharedReads>,
}

impl Drop for SharedReadsScope {
    fn drop(&mut self) {
        let previous = self.previous.take();
        SHARED.with_borrow_mut(|shared| *shared = previous);
    }
}

/// Puts `reads` in force on this thread until the returned guard drops:
/// every schema source the resolver reads from the disk, and every host
/// reader that asks [`shared_reads`], reads through it.
pub fn enter_shared_reads(reads: SharedReads) -> SharedReadsScope {
    SharedReadsScope { previous: SHARED.with_borrow_mut(|shared| shared.replace(reads)) }
}

/// Runs `resolve` with `reads` in force on this thread ([`enter_shared_reads`]).
pub fn with_shared_reads<T>(reads: &SharedReads, resolve: impl FnOnce() -> T) -> T {
    let _scope = enter_shared_reads(reads.clone());
    resolve()
}

/// The [`SharedReads`] in force on this thread, when one is.
#[must_use]
pub fn shared_reads() -> Option<SharedReads> {
    SHARED.with_borrow(Clone::clone)
}

/// The [`SharedReads`] in force on this thread when every schema source a
/// resolution reads here now comes from it: no [`guarded`] or
/// [`bundle`](super::bundle) scope is running, and every open buffer in
/// force is a copy of bytes the observation holds (a
/// [`with_reads_retained`] scope's), never an editor's text. A host may
/// then reuse work it derived from the observation's bytes ([`SharedReads::memo`]),
/// since a resolution would read the same bytes again.
#[must_use]
pub fn shared_reads_alone() -> Option<SharedReads> {
    let shared = shared_reads()?;
    if GUARDS.with_borrow(|guards| !guards.is_empty()) || super::bundle::current().is_some() {
        return None;
    }
    let copies = ACTIVE.with_borrow(|active| {
        active.texts.iter().all(|(path, text)| {
            shared.lock().get(path).is_some_and(|held| *held.bytes == *text.as_bytes())
        })
    });
    copies.then_some(shared)
}

/// The schema texts one [`recording`] scope has read: path and text, in
/// read order.
pub type ReadSources = Vec<(PathBuf, Arc<str>)>;

thread_local! {
    static ACTIVE: RefCell<SourceBuffers> = RefCell::new(SourceBuffers::default());
    /// The texts each running [`recording`] scope has read, in read order.
    static RECORDING: RefCell<Vec<ReadSources>> = const { RefCell::new(Vec::new()) };
    /// How many [`with_reads_retained`] scopes are running on this thread.
    static RETAINING: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// The guards of the running [`guarded`] scopes, outermost first.
    static GUARDS: RefCell<Vec<ReadGuard>> = const { RefCell::new(Vec::new()) };
    /// The [`SharedReads`] [`enter_shared_reads`] put in force.
    static SHARED: RefCell<Option<SharedReads>> = const { RefCell::new(None) };
}

/// A host's judgment of one schema source read: `true` lets it happen.
pub type ReadGuard = std::rc::Rc<dyn Fn(&Path) -> bool>;

/// Runs `resolve` with `guard` asked before every schema source it reads,
/// from a buffer or the disk, at any import depth. A path any running guard
/// refuses is not read: the read fails with
/// [`std::io::ErrorKind::PermissionDenied`], which the resolver reports as
/// it reports any unreadable file. The guard is removed when `resolve`
/// returns, even when it panics.
pub fn guarded<T>(guard: ReadGuard, resolve: impl FnOnce() -> T) -> T {
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            GUARDS.with_borrow_mut(Vec::pop);
        }
    }
    GUARDS.with_borrow_mut(|guards| guards.push(guard));
    let _pop = Pop;
    resolve()
}

/// Runs `resolve` with no [`guarded`] scope's guard in force, restoring them
/// after, even when it panics: for work a host judges by other means (a
/// schema slot evaluated with its own approvals inside a resolution another
/// slot's guard covers).
pub fn unguarded<T>(resolve: impl FnOnce() -> T) -> T {
    struct Restore(Vec<ReadGuard>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let outer = std::mem::take(&mut self.0);
            GUARDS.with_borrow_mut(|guards| *guards = outer);
        }
    }
    let _restore = Restore(GUARDS.with_borrow_mut(std::mem::take));
    resolve()
}

/// Whether every running guard lets `path` be read. The guards are cloned
/// out first, so a guard may itself read through this module.
fn permitted(path: &Path) -> bool {
    let guards = GUARDS.with_borrow(Clone::clone);
    guards.iter().all(|guard| guard(path))
}

/// Runs `resolve` with the snapshot in force, also keeping each schema
/// source it reads from the disk as an open buffer for the rest of `resolve`:
/// one host operation that runs several separate resolutions (a catalog's
/// globals, say) reads each file once. The kept texts are dropped when
/// `resolve` returns; nothing persists past the call. Inside a running scope
/// this joins it, so what an inner operation reads stays kept for the outer
/// one.
pub fn with_reads_retained<T>(resolve: impl FnOnce() -> T) -> T {
    if RETAINING.get() > 0 {
        return resolve();
    }
    struct Leave;
    impl Drop for Leave {
        fn drop(&mut self) {
            RETAINING.set(RETAINING.get() - 1);
        }
    }
    let current = ACTIVE.with_borrow(Clone::clone);
    with(&current, || {
        RETAINING.set(RETAINING.get() + 1);
        let _leave = Leave;
        resolve()
    })
}

/// `path`'s text, read as the resolver reads a schema source: the open
/// buffer in force, else the file's, kept for the rest of a
/// [`with_reads_retained`] scope. For a host reading a schema file it then
/// resolves (a baseline, a catalog) before handing the text on.
///
/// ## Errors
///
/// The read's I/O error; [`std::io::ErrorKind::InvalidData`] when the file
/// is not UTF-8.
pub fn read_to_string(path: &Path) -> std::io::Result<String> {
    String::from_utf8(read(path)?).map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

/// Runs `resolve` with `buffers` in force on this thread, restoring the
/// snapshot that was in force before, even when `resolve` panics.
pub fn with<T>(buffers: &SourceBuffers, resolve: impl FnOnce() -> T) -> T {
    struct Restore(Option<SourceBuffers>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some(previous) = self.0.take() {
                ACTIVE.with_borrow_mut(|active| *active = previous);
            }
        }
    }
    let previous = ACTIVE.with_borrow_mut(|active| std::mem::replace(active, buffers.clone()));
    let _restore = Restore(Some(previous));
    resolve()
}

/// The open text at `path` in the snapshot in force on this thread.
#[must_use]
pub fn text(path: &Path) -> Option<Arc<str>> {
    ACTIVE.with_borrow(|active| active.get(path).cloned())
}

/// Whether an open buffer is at `path` in the snapshot in force: a file
/// selection treats it as present at a location that holds no file.
#[cfg_attr(not(feature = "file"), allow(dead_code))]
pub(super) fn is_open(path: &Path) -> bool {
    ACTIVE.with_borrow(|active| active.get(path).is_some())
}

/// `path`'s bytes, read as the resolver reads a schema source: the open
/// buffer's text in force on this thread, else the file's, through the
/// [`SharedReads`] in force when there is one (after every guard). Outside
/// both scopes this is one [`open_source`]. A host reading a file that a
/// resolution depends on (a data file a slot loads) reads it here, so an
/// editor's unsaved text is what it sees.
///
/// ## Errors
///
/// The file read's I/O error; [`std::io::ErrorKind::PermissionDenied`] when
/// a [`guarded`] scope refuses `path`, or a [`bundle`](super::bundle) scope
/// in force lists no member at it (a listed member's verified bytes are
/// served instead of any buffer or file).
pub fn read(path: &Path) -> std::io::Result<Vec<u8>> {
    if !permitted(path) {
        return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "the host refused the read"));
    }
    if let Some(served) = super::bundle::read(path) {
        let bytes = served?;
        if RECORDING.with_borrow(|frames| !frames.is_empty())
            && let Ok(text) = std::str::from_utf8(&bytes)
        {
            record(path, &Arc::from(text));
        }
        return Ok(bytes);
    }
    if let Some(text) = text(path) {
        record(path, &text);
        return Ok(text.as_bytes().to_vec());
    }
    let bytes = match shared_reads() {
        Some(shared) => shared.read(path)?.bytes.to_vec(),
        None => open_source(path)?.bytes.to_vec(),
    };
    super::observed::opened(path);
    // Non-UTF-8 bytes fail the read's caller, so there is nothing to keep.
    if let Ok(text) = std::str::from_utf8(&bytes)
        && (RETAINING.get() > 0 || RECORDING.with_borrow(|frames| !frames.is_empty()))
    {
        let text: Arc<str> = Arc::from(text);
        record(path, &text);
        if RETAINING.get() > 0 {
            ACTIVE.with_borrow_mut(|active| active.insert(path, text));
        }
    }
    Ok(bytes)
}

/// Adds the text read at `path` to every running [`recording`] scope that
/// has not read it yet.
fn record(path: &Path, text: &Arc<str>) {
    RECORDING.with_borrow_mut(|frames| {
        for frame in frames.iter_mut() {
            if !frame.iter().any(|(seen, _)| seen == path) {
                frame.push((path.to_path_buf(), Arc::clone(text)));
            }
        }
    });
}

/// Runs `resolve`, returning its result with every schema source text it
/// read (from a buffer or the disk), each path once, in read order: the
/// exact bytes a template's resolution depended on. A host that hashes what
/// a resolution read (a data contract's imports) takes the texts from here
/// rather than reading the files again.
pub fn recording<T>(resolve: impl FnOnce() -> T) -> (T, ReadSources) {
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            RECORDING.with_borrow_mut(Vec::pop);
        }
    }
    RECORDING.with_borrow_mut(|frames| frames.push(Vec::new()));
    let pop = Pop;
    let result = resolve();
    let read = RECORDING.with_borrow_mut(|frames| frames.last_mut().map(std::mem::take)).unwrap_or_default();
    drop(pop);
    (result, read)
}

/// The snapshot in force on this thread.
pub(crate) fn current() -> SourceBuffers {
    ACTIVE.with_borrow(Clone::clone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_reads_serve_the_first_read_and_keep_no_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.yaml");
        let shared = SharedReads::default();
        assert!(shared.read(&path).is_err(), "a missing file fails");
        assert!(!shared.holds(&path), "and its failure is not kept");

        std::fs::write(&path, "first").unwrap();
        assert_eq!(&*shared.read(&path).unwrap().bytes, b"first");
        std::fs::write(&path, "second").unwrap();

        // Every reader of the observation sees the first read, through the
        // resolver's reader too; one sharing nothing reads the file now.
        assert_eq!(&*shared.clone().read(&path).unwrap().bytes, b"first");
        assert_eq!(with_shared_reads(&shared, || read(&path).unwrap()), b"first");
        assert_eq!(read(&path).unwrap(), b"second");
        assert_eq!(&*SharedReads::default().read(&path).unwrap().bytes, b"second");
    }

    #[test]
    fn a_guard_refuses_a_path_the_shared_reads_already_hold() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.yaml");
        std::fs::write(&path, "held").unwrap();
        let shared = SharedReads::default();
        shared.read(&path).unwrap();

        let refused = with_shared_reads(&shared, || guarded(std::rc::Rc::new(|_: &Path| false), || read(&path)));

        assert_eq!(refused.unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(with_shared_reads(&shared, || read(&path).unwrap()), b"held");
    }

    #[test]
    fn a_snapshot_applies_only_inside_with_and_is_restored_after() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("types.yaml");
        std::fs::write(&file, "disk").unwrap();
        let mut buffers = SourceBuffers::default();
        buffers.insert(&file, Arc::from("buffer"));
        buffers.insert(&dir.path().join("new.yaml"), Arc::from("unsaved"));

        assert_eq!(read(&file).unwrap(), b"disk");
        let (inside, found) = with(&buffers, || {
            (read(&file).unwrap(), [dir.path().join("absent.yaml"), dir.path().join("new.yaml")].map(|path| is_open(&path)))
        });
        assert_eq!(inside, b"buffer");
        assert_eq!(found, [false, true]);
        assert_eq!(read(&file).unwrap(), b"disk");
        assert!(!is_open(&dir.path().join("new.yaml")));
    }

    /// A refused path is not read from the disk or a buffer, an allowed one
    /// is, and the guard ends with its scope.
    #[test]
    fn a_guard_is_asked_before_every_read_and_a_refusal_reads_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (open, closed, buffered) = (dir.path().join("open.yaml"), dir.path().join("closed.yaml"), dir.path().join("buffered.yaml"));
        std::fs::write(&open, "open").unwrap();
        std::fs::write(&closed, "closed").unwrap();
        let mut buffers = SourceBuffers::default();
        buffers.insert(&buffered, Arc::from("unsaved"));
        let asked = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let guard: ReadGuard = std::rc::Rc::new({
            let (asked, open) = (std::rc::Rc::clone(&asked), open.clone());
            move |path: &Path| {
                asked.borrow_mut().push(path.to_path_buf());
                path == open
            }
        });

        let ((allowed, refused, unsaved), recorded) = with(&buffers, || {
            recording(|| guarded(guard, || (read(&open), read(&closed), read(&buffered))))
        });
        assert_eq!(allowed.unwrap(), b"open");
        assert_eq!(refused.unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(unsaved.unwrap_err().kind(), std::io::ErrorKind::PermissionDenied, "a buffer is content too");
        assert_eq!(*asked.borrow(), vec![open.clone(), closed.clone(), buffered]);
        assert_eq!(recorded.iter().map(|(path, _)| path.clone()).collect::<Vec<_>>(), vec![open], "only the allowed read");
        assert_eq!(read(&closed).unwrap(), b"closed", "the guard ended with its scope");
    }

    /// An unguarded scope inside a guarded one reads without asking, and the
    /// guard is back in force after it.
    #[test]
    fn an_unguarded_scope_suspends_the_guards_around_it() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("types.yaml");
        std::fs::write(&file, "types").unwrap();
        let refuse_all: ReadGuard = std::rc::Rc::new(|_: &Path| false);
        let (inside, after) = guarded(refuse_all, || (unguarded(|| read(&file)), read(&file)));
        assert_eq!(inside.unwrap(), b"types");
        assert_eq!(after.unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);
    }

    /// A retaining scope serves a file's first read for the rest of the scope,
    /// joins an enclosing scope, and keeps nothing once it ends.
    #[test]
    fn a_retaining_scope_reads_each_file_once_and_forgets_after() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("types.yaml");
        std::fs::write(&file, "first").unwrap();

        let (outer, inner) = with_reads_retained(|| {
            let outer = read(&file).unwrap();
            std::fs::write(&file, "second").unwrap();
            (outer, with_reads_retained(|| read_to_string(&file).unwrap()))
        });
        assert_eq!((outer.as_slice(), inner.as_str()), (b"first".as_slice(), "first"));
        assert_eq!(read(&file).unwrap(), b"second", "nothing is kept past the scope");
        assert!(text(&file).is_none());
    }

    /// A recording scope lists each source once, in read order, from a
    /// buffer or the disk, with the exact text read; nothing is recorded
    /// outside one.
    #[test]
    fn a_recording_scope_lists_every_text_read_once_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let (disk, open) = (dir.path().join("disk.yaml"), dir.path().join("open.yaml"));
        std::fs::write(&disk, "on disk").unwrap();
        let mut buffers = SourceBuffers::default();
        buffers.insert(&open, Arc::from("in the editor"));

        read(&disk).unwrap();
        let ((), recorded) = recording(|| {
            with(&buffers, || {
                read(&open).unwrap();
                read(&disk).unwrap();
                read(&open).unwrap();
            })
        });
        let recorded: Vec<(PathBuf, String)> = recorded.into_iter().map(|(path, text)| (path, text.to_string())).collect();
        assert_eq!(recorded, vec![(open, "in the editor".to_string()), (disk.clone(), "on disk".to_string())]);
        let ((), outside) = recording(|| ());
        assert!(outside.is_empty());
    }

    /// The resolver's reads, through the public entry point: an unsaved
    /// import replaces the file's text, an unsaved target that exists only in
    /// the editor is selected, and outside [`with`] the disk is read again.
    #[cfg(feature = "file")]
    #[test]
    fn resolution_reads_unsaved_imports_and_selects_unsaved_targets_only_inside_with() {
        use crate::embedded::ShippedSources;
        use serde_json::json;

        let dir = tempfile::tempdir().unwrap();
        let context = biscuit_file::FileResolutionContext::new(dir.path());
        let library = dir.path().join("types.yaml");
        std::fs::write(&library, "kind: schema\ntypes:\n  Owner: string\n").unwrap();
        let reference: serde_yaml_ng::Value = serde_yaml_ng::from_str("owner: Owner@./types.yaml").unwrap();
        let target: serde_yaml_ng::Value = serde_yaml_ng::from_str("./contract.yaml").unwrap();
        let accepts = |schema: &serde_json::Value, owner: serde_json::Value| {
            jsonschema::validator_for(schema).unwrap().is_valid(&json!({ "owner": owner }))
        };
        let resolve = |value: &serde_yaml_ng::Value| super::super::resolve_yaml_schema(value, dir.path(), &context, ShippedSources::NONE);

        let mut buffers = SourceBuffers::default();
        buffers.insert(&library, Arc::from("kind: schema\ntypes:\n  Owner: number\n"));
        buffers.insert(&dir.path().join("contract.yaml"), Arc::from("$schema:\n  owner: number\n"));

        let saved = resolve(&reference).unwrap();
        assert!(accepts(&saved.json_schema, json!("ops")));
        let (unsaved, contract) = with(&buffers, || (resolve(&reference).unwrap(), resolve(&target).unwrap()));
        assert!(accepts(&unsaved.json_schema, json!(1)) && !accepts(&unsaved.json_schema, json!("ops")));
        assert!(accepts(&contract.json_schema, json!(1)) && !accepts(&contract.json_schema, json!("ops")));
        assert!(accepts(&resolve(&reference).unwrap().json_schema, json!("ops")));
        assert!(resolve(&target).is_err(), "the unsaved target is unknown outside `with`");
    }
}
