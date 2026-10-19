//! The macOS adapter: one FSEvents stream with file-level events over the
//! original, the quarantine, and the admin directory, run on its own run-loop
//! thread.
//!
//! - FSEvents watches paths, not directories, and recursively. Watching both
//!   the original and the quarantine path before the move covers the tree on
//!   either side of it and the original's recreation. The parent is not
//!   watched: a recursive watch on it would deliver every sibling worktree's
//!   activity, and a busy sibling (a build) is enough to make FSEvents drop
//!   events. Only these three subtrees reach the stream.
//! - A drain is a series of marker rounds. A round creates a file in a
//!   private marker directory the stream also watches, and waits for that
//!   file's event, asking the stream to flush (`FSEventStreamFlushAsync`,
//!   which does not block) every millisecond until it arrives. A flush alone
//!   delivers only what fseventsd has already processed (measured: 1 of 50
//!   write-then-flush rounds missed its write), and one confirmed marker is
//!   not enough either: fseventsd does not keep the order changes were made
//!   in. Under a burst of 2,000 creations it assigned a few of them event
//!   IDs after a marker created once all 2,000 had returned, and delivered
//!   them only in a later round (about 1 drain in 15). So the drain repeats
//!   rounds until one delivers no watched-path event, which makes a late
//!   event rare (none in 500 stressed bursts) but not impossible: with 11
//!   streams busy at once, a quiet round still came back with 240 creations
//!   pending. A drain here therefore returns what has been delivered so far;
//!   anything late arrives in a later drain, never lost without a loss being
//!   reported. Correctness rests on the direct checks: the deletion engine's
//!   comparison of each entry with its record before unlinking it, and
//!   unregistration's reread of the index, HEAD, and both paths. The marker
//!   directory is in the per-user temporary directory, outside the checkout
//!   and Git's admin directory.
//! - A drain's rounds share one [`WaitState`] under the session's
//!   [`WaitPolicy`]: a newly acknowledged marker or an in-scope event is
//!   progress (a sibling's, an unreported admin file's, or a marker's own
//!   cleanup event is not). Silence for the stall limit is
//!   `DeliveryStalled`; the ceiling spans every round, so events that keep
//!   arriving cannot extend a drain past it (`DrainCeiling`). A late event
//!   is progress but does not make a round quiet. Startup and the waits
//!   before the move is labeled and before the admin directory is released
//!   are drains of the same kind.
//! - `MustScanSubDirs` and either dropped flag are coverage loss, reported as
//!   `FsEventsDropped` with the event's whole flag mask and every loss flag
//!   it carries.
//! - A moved or deleted parent or admin directory produces no event under the
//!   watched paths, so each drain compares their identity (device and inode)
//!   with what it was at start.
//! - Item flags are cumulative history for a path, so one event may carry
//!   several kinds (a file created and deleted between deliveries reports
//!   both). Each flag is reported; a kind alone never proves who made the
//!   change. A rename does not say which side it is: a path that exists when
//!   the event is read is `MovedTo`, otherwise `MovedFrom`.
//! - The stream is owned by fseventsd, not by a descriptor of this process,
//!   so a usage query of the checkout sees nothing of it and no handle is
//!   registered.

#[cfg(test)]
mod tests;

use std::ffi::{CStr, CString, OsStr, OsString, c_char, c_void};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::os::unix::ffi::OsStringExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};

use fsevent_sys::core_foundation as cf;
use fsevent_sys as fs_events;
use sniff::filesystem::query::EvidenceKind;

use super::diagnostics::RoundTrips;
use super::wait::{Delivery, Outcome, Source, WaitPolicy, WaitState, await_marker};
use super::{
    Backend, CoverageLost, EventKind, EventQueue, FsEventsLossFlag, ObservationBackend, ObservationError, ObserverOptions, RawEvent, Root, Sink,
    Targets, Waited,
};

/// How long FSEvents may hold events before delivering them unasked. A drain
/// flushes regardless; a zero latency with `NoDefer` dropped events on bursts
/// (S2), so the stream batches instead.
const LATENCY_SECONDS: f64 = 0.1;

/// How often a waiting round asks for a flush. Each flush delivers only what
/// fseventsd has processed by then; without one, the stream holds a marker
/// for its latency, which made every round about 100 ms slower.
const FLUSH_INTERVAL: Duration = Duration::from_millis(1);

/// Each loss flag, in [`FsEventsLossFlag`] declaration order.
const LOSS_FLAGS: [(u32, FsEventsLossFlag); 3] = [
    (fs_events::kFSEventStreamEventFlagKernelDropped, FsEventsLossFlag::KernelDropped),
    (fs_events::kFSEventStreamEventFlagUserDropped, FsEventsLossFlag::UserDropped),
    (fs_events::kFSEventStreamEventFlagMustScanSubDirs, FsEventsLossFlag::MustScanSubDirs),
];

/// The loss `flags` reports, if any: every named loss flag it carries, and
/// the mask itself.
fn loss(flags: u32) -> Option<CoverageLost> {
    let named: Vec<FsEventsLossFlag> = LOSS_FLAGS.iter().filter(|(bit, _)| flags & bit != 0).map(|(_, flag)| *flag).collect();
    (!named.is_empty()).then_some(CoverageLost::FsEventsDropped { raw: flags, flags: named })
}

const METADATA_FLAGS: u32 = fs_events::kFSEventStreamEventFlagItemInodeMetaMod
    | fs_events::kFSEventStreamEventFlagItemFinderInfoMod
    | fs_events::kFSEventStreamEventFlagItemChangeOwner
    | fs_events::kFSEventStreamEventFlagItemXattrMod;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRetain(cf: cf::CFRef) -> cf::CFRef;
}

/// A directory's identity; a different one at the same path is a different
/// directory.
type Identity = (u64, u64);

fn identity(path: &Path) -> io::Result<Identity> {
    let metadata = std::fs::symlink_metadata(path)?;
    Ok((metadata.dev(), metadata.ino()))
}

fn setup(path: &Path) -> impl FnOnce(io::Error) -> ObservationError + use<> {
    let path = path.to_path_buf();
    move |source| ObservationError::Setup { path, source }
}

/// The watched paths as FSEvents reports them (resolved: `/private/var/...`
/// for `/var/...`, and the on-disk case of each name), with the session's own
/// names for the parent's entries.
#[derive(Debug)]
struct Paths {
    original: PathBuf,
    quarantine: PathBuf,
    admin: PathBuf,
    parent: PathBuf,
    /// The session's private marker directory; see [`MarkerDirectory`].
    markers: PathBuf,
    original_name: OsString,
    quarantine_name: OsString,
}

impl Paths {
    /// `moved` says the tree is already at the quarantine (a resumed
    /// removal); the tree's current path must be a real directory.
    fn resolve(targets: &Targets, markers: &Path, moved: bool) -> Result<Self, ObservationError> {
        let parent = std::fs::canonicalize(targets.parent()).map_err(setup(targets.parent()))?;
        let tree = if moved { targets.quarantine() } else { targets.original() };
        // A link at the tree's path would be watched at its target, which is
        // not what the operation moves or deletes.
        let metadata = std::fs::symlink_metadata(tree).map_err(setup(tree))?;
        if !metadata.is_dir() {
            return Err(setup(tree)(io::Error::new(io::ErrorKind::NotADirectory, "not a directory (links are not followed)")));
        }
        let resolved = std::fs::canonicalize(tree).map_err(setup(tree))?;
        if resolved.parent() != Some(parent.as_path()) {
            return Err(setup(tree)(io::Error::other("the checkout does not resolve beside its quarantine")));
        }
        let admin = std::fs::canonicalize(targets.admin()).map_err(setup(targets.admin()))?;
        if !admin.is_dir() {
            return Err(setup(targets.admin())(io::Error::new(io::ErrorKind::NotADirectory, "not a directory")));
        }
        let (original, quarantine) = if moved {
            (parent.join(targets.original_name()), resolved)
        } else {
            (resolved, parent.join(targets.quarantine_name()))
        };
        Ok(Self {
            quarantine,
            original,
            admin,
            parent,
            markers: std::fs::canonicalize(markers).map_err(setup(markers))?,
            original_name: targets.original_name().to_os_string(),
            quarantine_name: targets.quarantine_name().to_os_string(),
        })
    }
}

/// What the callback reads. It outlives the stream: the adapter holds it
/// until the run-loop thread, which releases the stream, has been joined.
struct Shared {
    queue: EventQueue,
    paths: Paths,
    moved: AtomicBool,
    admin_released: AtomicBool,
    /// The highest marker the stream has delivered, and the in-scope events
    /// delivered so far (markers excluded); a round that leaves the count
    /// unchanged was quiet.
    delivery: Delivery<()>,
    /// Set by a test to hold the callback, as a delayed fseventsd would.
    #[cfg(test)]
    held: AtomicBool,
}

impl Shared {
    /// `path` as a [`Root`] and a path relative to it, or `None` outside the
    /// session's scope. The tree's root directory itself is the parent's
    /// entry, as the parent's watch reports it on Linux.
    fn locate(&self, path: &Path) -> Option<(Root, PathBuf)> {
        let paths = &self.paths;
        if let Ok(relative) = path.strip_prefix(&paths.admin) {
            return (!self.admin_released.load(Ordering::SeqCst)).then(|| (Root::Admin, relative.to_path_buf()));
        }
        let (tree, tree_name, other, other_name) = if self.moved.load(Ordering::SeqCst) {
            (&paths.quarantine, &paths.quarantine_name, &paths.original, &paths.original_name)
        } else {
            (&paths.original, &paths.original_name, &paths.quarantine, &paths.quarantine_name)
        };
        if let Ok(relative) = path.strip_prefix(tree) {
            return Some(if relative.as_os_str().is_empty() {
                (Root::Parent, PathBuf::from(tree_name))
            } else {
                (Root::Tree, relative.to_path_buf())
            });
        }
        let relative = path.strip_prefix(other).ok()?;
        Some((Root::Parent, Path::new(other_name).join(relative)))
    }

    /// Turns one FSEvents event into raw events or a loss.
    fn deliver(&self, path: &Path, flags: u32, received: SystemTime) {
        if let Some(loss) = loss(flags) {
            self.queue.lose(loss);
            return;
        }
        if let Ok(marker) = path.strip_prefix(&self.paths.markers) {
            if let Some(number) = marker.to_str().and_then(|name| name.parse().ok()) {
                self.delivery.acknowledge((), number);
            }
            return;
        }
        let Some((root, relative)) = self.locate(path) else {
            return;
        };
        if flags & (fs_events::kFSEventStreamEventFlagMount | fs_events::kFSEventStreamEventFlagUnmount) != 0 {
            // A volume appearing or leaving under a watched path changes
            // content no item event describes.
            self.queue.lose(CoverageLost::WatchLost { root });
            return;
        }
        let directory = (flags
            & (fs_events::kFSEventStreamEventFlagItemIsDir
                | fs_events::kFSEventStreamEventFlagItemIsFile
                | fs_events::kFSEventStreamEventFlagItemIsSymlink)
            != 0)
            .then_some(flags & fs_events::kFSEventStreamEventFlagItemIsDir != 0);
        let renamed = if std::fs::symlink_metadata(path).is_ok() { EventKind::MovedTo } else { EventKind::MovedFrom };
        let kinds = [
            (fs_events::kFSEventStreamEventFlagItemCreated, EventKind::Created),
            (fs_events::kFSEventStreamEventFlagItemModified, EventKind::Modified),
            (METADATA_FLAGS, EventKind::Metadata),
            (fs_events::kFSEventStreamEventFlagItemRenamed, renamed),
            (fs_events::kFSEventStreamEventFlagItemRemoved, EventKind::Removed),
        ];
        let mut in_scope = false;
        for (bits, kind) in kinds {
            if flags & bits != 0 {
                in_scope |= self.queue.push(RawEvent { root, path: relative.clone(), kind, directory, received });
            }
        }
        if in_scope {
            self.delivery.event(());
        }
    }
}

extern "C" fn callback(
    _stream: fs_events::FSEventStreamRef,
    info: *mut c_void,
    count: usize,
    paths: *mut c_void,
    flags: *const fs_events::FSEventStreamEventFlags,
    _ids: *const fs_events::FSEventStreamEventId,
) {
    // SAFETY: `info` is the `Shared` the adapter keeps alive until the stream
    // is released; without `UseCFTypes`, `paths` is `count` C strings and
    // `flags` holds `count` entries.
    let (shared, paths, flags) = unsafe {
        (
            &*info.cast::<Shared>(),
            std::slice::from_raw_parts(paths.cast::<*const c_char>(), count),
            std::slice::from_raw_parts(flags, count),
        )
    };
    #[cfg(test)]
    while shared.held.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(1));
    }
    let received = SystemTime::now();
    for (path, flags) in paths.iter().zip(flags) {
        // SAFETY: each entry is a NUL-terminated path FSEvents owns for the
        // duration of the callback.
        let path = OsStr::from_bytes(unsafe { CStr::from_ptr(*path) }.to_bytes());
        shared.deliver(Path::new(path), *flags, received);
    }
}

/// A Core Foundation pointer moved to or from the run-loop thread.
struct Handle(*mut c_void);

// SAFETY: run loops and streams may be referenced from any thread;
// `CFRunLoopStop` and the stream's flush requests are made from another
// thread than the one running the loop.
unsafe impl Send for Handle {}

pub(super) struct FsEvents {
    shared: Arc<Shared>,
    stream: Handle,
    /// Retained, so stopping it stays valid after its thread has exited.
    run_loop: Handle,
    thread: Option<JoinHandle<()>>,
    parent: Identity,
    admin: Identity,
    /// Removed with the session.
    _markers: MarkerDirectory,
    /// The last marker written.
    marker: u64,
    policy: WaitPolicy,
    round_trips: RoundTrips,
    /// Dropped by the run-loop thread as it ends.
    #[cfg(test)]
    thread_alive: Arc<()>,
}

impl FsEvents {
    pub fn start(targets: &Targets, options: ObserverOptions, moved: bool, interrupted: &dyn Fn() -> bool) -> Result<Self, ObservationError> {
        Self::start_held(targets, options, moved, interrupted, false)
    }

    /// [`start`](Self::start); `held` (tests only) holds the callback from
    /// the start, so the startup wait sees no delivery.
    fn start_held(
        targets: &Targets,
        options: ObserverOptions,
        moved: bool,
        interrupted: &dyn Fn() -> bool,
        #[cfg_attr(not(test), expect(unused_variables, reason = "only a test holds delivery"))] held: bool,
    ) -> Result<Self, ObservationError> {
        let markers = MarkerDirectory::create().map_err(|source| ObservationError::Setup { path: PathBuf::from("<per-user temporary directory>"), source })?;
        let paths = Paths::resolve(targets, &markers.0, moved)?;
        let parent = identity(&paths.parent).map_err(setup(targets.parent()))?;
        let admin = identity(&paths.admin).map_err(setup(targets.admin()))?;
        let watched = [paths.original.clone(), paths.quarantine.clone(), paths.admin.clone(), paths.markers.clone()];
        let shared = Arc::new(Shared {
            queue: EventQueue::new(targets.clone(), options.queue_capacity),
            paths,
            moved: AtomicBool::new(moved),
            admin_released: AtomicBool::new(false),
            delivery: Delivery::new([()]),
            #[cfg(test)]
            held: AtomicBool::new(held),
        });
        let stream = create_stream(&shared, &watched)?;
        #[cfg(test)]
        let thread_alive = Arc::new(());
        #[cfg(test)]
        let alive = Arc::clone(&thread_alive);

        let (sender, receiver) = mpsc::sync_channel(1);
        let thread_stream = Handle(stream.0);
        let spawned = std::thread::Builder::new().name("wt-fsevents".into()).spawn(move || {
            #[cfg(test)]
            let _alive = alive;
            run(thread_stream, &sender);
        });
        let thread = match spawned {
            Ok(thread) => thread,
            Err(error) => {
                // SAFETY: the stream was never scheduled or started.
                unsafe { fs_events::FSEventStreamRelease(stream.0) };
                return Err(setup(targets.original())(error));
            }
        };
        let Ok(Some(run_loop)) = receiver.recv() else {
            let _ = thread.join();
            return Err(setup(targets.original())(io::Error::other("FSEventStreamStart failed")));
        };
        let mut adapter = Self {
            shared,
            stream,
            run_loop,
            thread: Some(thread),
            parent,
            admin,
            _markers: markers,
            marker: 0,
            policy: options.wait,
            round_trips: RoundTrips::default(),
            #[cfg(test)]
            thread_alive,
        };
        // `FSEventStreamStart` returns before fseventsd routes events to the
        // stream; a change made right after it was missed. Once a marker
        // written after the start has arrived, every later change is covered.
        if adapter.barrier(interrupted) == Waited::Interrupted {
            return Err(ObservationError::Interrupted);
        }
        if let Some(loss) = adapter.shared.queue.take_loss() {
            return Err(ObservationError::not_delivering(ObservationBackend::FsEvents, loss, adapter.round_trips));
        }
        Ok(adapter)
    }

    /// Runs marker rounds until one delivers no in-scope event, or records
    /// why that could not be confirmed. See the module docs for why one
    /// round is not enough.
    fn barrier(&mut self, interrupted: &dyn Fn() -> bool) -> Waited {
        let mut wait = WaitState::new(self.policy, Instant::now(), [()]);
        loop {
            let before = self.shared.delivery.delivered(()).events;
            match self.marker_round(&mut wait, interrupted) {
                Round::Confirmed => {}
                Round::Ended(waited) => return waited,
            }
            if self.shared.delivery.delivered(()).events == before {
                return Waited::Done;
            }
            wait.expect((), Instant::now());
        }
    }

    /// Writes the next marker, asks for a flush, and waits for the stream to
    /// deliver the marker; a stall or the ceiling is recorded as loss.
    fn marker_round(&mut self, wait: &mut WaitState<()>, interrupted: &dyn Fn() -> bool) -> Round {
        self.marker += 1;
        let marker = self.shared.paths.markers.join(self.marker.to_string());
        if let Err(error) = std::fs::File::create(&marker) {
            self.shared.queue.lose(CoverageLost::ReadFailed { message: format!("writing the drain marker failed: {error}") });
            return Round::Ended(Waited::Done);
        }
        let queue = &self.shared.queue;
        let source = Flushing { delivery: &self.shared.delivery, stream: &self.stream };
        let outcome = await_marker(&source, wait, self.marker, &|| queue.has_loss(), interrupted);
        let _ = std::fs::remove_file(&marker);
        match outcome {
            Outcome::Confirmed { round_trip } => {
                self.round_trips.record(round_trip);
                Round::Confirmed
            }
            Outcome::Lost => Round::Ended(Waited::Done),
            Outcome::Interrupted => Round::Ended(Waited::Interrupted),
            Outcome::Stalled { waited, .. } => {
                queue.lose(CoverageLost::DeliveryStalled { waited, watch: None });
                Round::Ended(Waited::Done)
            }
            Outcome::Ceiling { waited, .. } => {
                queue.lose(CoverageLost::DrainCeiling { waited, watch: None });
                Round::Ended(Waited::Done)
            }
        }
    }

    /// The parent and (until released) the admin directory must still be the
    /// directories the session started on; the stream would not report their
    /// move or deletion.
    fn check_identities(&self) {
        let paths = &self.shared.paths;
        if identity(&paths.parent).ok() != Some(self.parent) {
            self.shared.queue.lose(CoverageLost::WatchLost { root: Root::Parent });
        }
        if !self.shared.admin_released.load(Ordering::SeqCst) && identity(&paths.admin).ok() != Some(self.admin) {
            self.shared.queue.lose(CoverageLost::WatchLost { root: Root::Admin });
        }
    }
}

/// The stream's deliveries, flushed whenever a round would block.
struct Flushing<'a> {
    delivery: &'a Delivery<()>,
    stream: &'a Handle,
}

impl Source<()> for Flushing<'_> {
    fn now(&self) -> Instant {
        self.delivery.now()
    }

    fn generation(&self) -> u64 {
        self.delivery.generation()
    }

    fn delivered(&self, key: ()) -> super::wait::Delivered {
        self.delivery.delivered(key)
    }

    fn park(&self, seen: u64, timeout: Duration) {
        // SAFETY: the stream is started and is released only after the
        // run-loop thread is joined, in `drop`.
        unsafe { fs_events::FSEventStreamFlushAsync(self.stream.0) };
        self.delivery.park(seen, timeout.min(FLUSH_INTERVAL));
    }
}

/// How one marker round ended.
enum Round {
    Confirmed,
    /// The barrier ends here: a loss was recorded, or the wait interrupted.
    Ended(Waited),
}

/// A directory only this session writes, in the per-user temporary directory
/// (`confstr(_CS_DARWIN_USER_TEMP_DIR)`, owner-only), removed with the
/// session. A session that crashed leaves an empty directory there, which the
/// system's temporary-file cleanup removes.
struct MarkerDirectory(PathBuf);

impl MarkerDirectory {
    fn create() -> io::Result<Self> {
        let mut buffer = vec![0_u8; libc::PATH_MAX as usize];
        // SAFETY: `buffer` holds `buffer.len()` bytes; `confstr` writes at
        // most that many and returns the length the value needs, NUL included.
        let needed = unsafe { libc::confstr(libc::_CS_DARWIN_USER_TEMP_DIR, buffer.as_mut_ptr().cast(), buffer.len()) };
        if needed == 0 || needed > buffer.len() {
            return Err(io::Error::last_os_error());
        }
        buffer.truncate(needed - 1);
        let mut random = [0_u8; 8];
        getrandom::fill(&mut random).map_err(io::Error::other)?;
        let name = format!("wt-observe-{:016x}", u64::from_ne_bytes(random));
        let path = PathBuf::from(OsString::from_vec(buffer)).join(name);
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for MarkerDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn create_stream(shared: &Arc<Shared>, watched: &[PathBuf]) -> Result<Handle, ObservationError> {
    let mut strings = Vec::with_capacity(watched.len());
    for path in watched {
        let text = CString::new(path.as_os_str().as_bytes()).map_err(|error| setup(path)(io::Error::new(io::ErrorKind::InvalidInput, error)))?;
        // SAFETY: `text` is NUL-terminated; a null result is checked.
        let string = unsafe { cf::CFStringCreateWithCString(cf::kCFAllocatorDefault, text.as_ptr(), cf::kCFStringEncodingUTF8) };
        if string.is_null() {
            strings.into_iter().for_each(|string| unsafe { cf::CFRelease(string) });
            return Err(setup(path)(io::Error::new(io::ErrorKind::InvalidData, "the path is not UTF-8")));
        }
        strings.push(string);
    }
    // SAFETY: the array retains each string, so they are released here; the
    // stream copies the array. `shared` outlives the stream (see `Shared`).
    unsafe {
        let array = cf::CFArrayCreateMutable(cf::kCFAllocatorDefault, 0, &cf::kCFTypeArrayCallBacks);
        for string in strings {
            cf::CFArrayAppendValue(array, string);
            cf::CFRelease(string);
        }
        let context = fs_events::FSEventStreamContext {
            version: 0,
            info: Arc::as_ptr(shared).cast_mut().cast(),
            retain: None,
            release: None,
            copy_description: None,
        };
        let stream = fs_events::FSEventStreamCreate(
            cf::kCFAllocatorDefault,
            callback,
            &context,
            array,
            fs_events::kFSEventStreamEventIdSinceNow,
            LATENCY_SECONDS,
            fs_events::kFSEventStreamCreateFlagFileEvents,
        );
        cf::CFRelease(array);
        if stream.is_null() {
            return Err(setup(&watched[0])(io::Error::other("FSEventStreamCreate failed")));
        }
        Ok(Handle(stream))
    }
}

/// The run-loop thread: schedules and starts the stream, reports its run loop
/// (or `None` if the stream would not start), runs until stopped, then stops,
/// invalidates, and releases the stream.
fn run(stream: Handle, started: &mpsc::SyncSender<Option<Handle>>) {
    // SAFETY: the stream is valid and used by this thread alone until it is
    // started; afterwards only `FSEventStreamFlushAsync` is called elsewhere,
    // and never after `drop` has stopped this loop.
    unsafe {
        let run_loop = cf::CFRunLoopGetCurrent();
        fs_events::FSEventStreamScheduleWithRunLoop(stream.0, run_loop, cf::kCFRunLoopDefaultMode);
        if fs_events::FSEventStreamStart(stream.0) == 0 {
            fs_events::FSEventStreamInvalidate(stream.0);
            fs_events::FSEventStreamRelease(stream.0);
            let _ = started.send(None);
            return;
        }
        let _ = started.send(Some(Handle(CFRetain(run_loop))));
        cf::CFRunLoopRun();
        fs_events::FSEventStreamStop(stream.0);
        fs_events::FSEventStreamInvalidate(stream.0);
        fs_events::FSEventStreamRelease(stream.0);
    }
}

impl Backend for FsEvents {
    fn drain(&mut self, sink: &mut Sink<'_>, interrupted: &dyn Fn() -> bool) -> Waited {
        // Lost coverage stays lost: there is nothing left to confirm.
        let waited = if sink.lost.is_some() { Waited::Done } else { self.barrier(interrupted) };
        self.check_identities();
        self.shared.queue.take_into(sink);
        waited
    }

    /// Delivers first, so a change made before the move is still labeled with
    /// the tree; from here on, the quarantine path is the tree.
    fn moved_to_quarantine(&mut self, interrupted: &dyn Fn() -> bool) -> Waited {
        let waited = self.barrier(interrupted);
        self.shared.moved.store(true, Ordering::SeqCst);
        waited
    }

    fn release_admin(&mut self, interrupted: &dyn Fn() -> bool) -> Waited {
        let waited = self.barrier(interrupted);
        if waited == Waited::Done {
            self.shared.admin_released.store(true, Ordering::SeqCst);
        }
        waited
    }

    fn handles(&self) -> Vec<(EvidenceKind, u64)> {
        Vec::new()
    }

    fn backend(&self) -> ObservationBackend {
        ObservationBackend::FsEvents
    }

    fn round_trips(&self) -> RoundTrips {
        self.round_trips
    }
}

impl Drop for FsEvents {
    fn drop(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        #[cfg(test)]
        self.shared.held.store(false, Ordering::SeqCst);
        // A stop sent before the loop is running is lost, so repeat it until
        // the thread ends. The queue never makes the callback wait, so the
        // loop always comes back to its stop check.
        while !thread.is_finished() {
            // SAFETY: the run loop is retained until below.
            unsafe { cf::CFRunLoopStop(self.run_loop.0) };
            std::thread::sleep(Duration::from_millis(1));
        }
        let _ = thread.join();
        // SAFETY: retained in `run`; the loop no longer runs.
        unsafe { cf::CFRelease(self.run_loop.0) };
    }
}
