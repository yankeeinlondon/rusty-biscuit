//! The Windows adapter: `ReadDirectoryChangesW` on the original's parent,
//! with the subtree flag, and on the admin directory, both read by one
//! delivery thread.
//!
//! - One handle on the parent covers the tree on either side of the move and
//!   the original's recreation; the handle does not block the move (S1).
//!   Paths arrive relative to the parent, so an entry named like the tree is
//!   the tree, whatever its name is now. Siblings' changes are dropped before
//!   anything else is done with them.
//! - A drain is a marker: a hidden file created in each watched directory and
//!   deleted when its handle closes, so a crash leaves nothing behind. Delivery
//!   is in order per handle, so once a handle has reported its marker, every
//!   earlier change in that directory has been delivered.
//! - Each handle awaited by a drain has its own stall clock under the
//!   session's [`WaitPolicy`]: its marker or an in-scope event from it is
//!   its progress, and a busy parent never hides a silent admin handle, or
//!   the reverse. Silence for the stall limit is `DeliveryStalled` naming
//!   that watch; the ceiling is shared by the drain (`DrainCeiling`). A
//!   handle whose watch ended is not awaited. Startup and the waits before
//!   the move is labeled and before the admin handle is released are drains
//!   of the same kind.
//! - A zero-byte completion (the system's buffer overflowed) and
//!   `ERROR_NOTIFY_ENUM_DIR` are `Overflow`. Any other completion error is the
//!   watch ending, `WatchLost`; the session's own cancel happens only when
//!   the admin directory is released or the session ends, and is never read
//!   as a completion.
//! - A moved or deleted parent or admin directory may produce no completion,
//!   so each drain also compares their identity with what it was at start.
//! - `FILE_ACTION_MODIFIED` covers content, attributes, and timestamps alike,
//!   so every change in place is `Modified`, never `Metadata`.
//! - Nothing in the watched trees is ever opened, not even to read an entry's
//!   type: any handle inside the tree, however brief, makes the move fail
//!   with `ERROR_ACCESS_DENIED`. The type comes from the extended
//!   notification format's attributes; a volume that refuses that format is
//!   read in the plain one, whose events carry no type.
//! - Neither handle is inside the checkout, so a usage query of the checkout
//!   does not report them and none is registered.

#[cfg(test)]
mod tests;

use std::ffi::{OsStr, OsString};
use std::io;
use std::mem::offset_of;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::{Instant, SystemTime};

use sniff::filesystem::query::{EvidenceKind, FileIdentity};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_INVALID_FUNCTION, ERROR_INVALID_PARAMETER, ERROR_NOT_SUPPORTED, ERROR_NOTIFY_ENUM_DIR, FALSE, HANDLE, INVALID_HANDLE_VALUE, TRUE,
    WAIT_OBJECT_0,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ACTION_ADDED, FILE_ATTRIBUTE_DIRECTORY, FILE_ACTION_REMOVED, FILE_ACTION_RENAMED_NEW_NAME, FILE_ACTION_RENAMED_OLD_NAME,
    FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_TEMPORARY, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_DELETE_ON_CLOSE,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE,
    FILE_NOTIFY_CHANGE_ATTRIBUTES, FILE_NOTIFY_CHANGE_CREATION, FILE_NOTIFY_CHANGE_DIR_NAME,
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SECURITY,
    FILE_NOTIFY_CHANGE_SIZE, FILE_NOTIFY_EXTENDED_INFORMATION, FILE_NOTIFY_INFORMATION, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, READ_DIRECTORY_NOTIFY_INFORMATION_CLASS, ReadDirectoryChangesExW,
    ReadDirectoryNotifyExtendedInformation, ReadDirectoryNotifyInformation,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Threading::{CreateEventW, INFINITE, ResetEvent, SetEvent, WaitForMultipleObjects};

use super::diagnostics::RoundTrips;
use super::wait::{Delivery, Outcome, Source, WaitPolicy, WaitState, await_marker};
use super::{
    Backend, CoverageLost, EventKind, EventQueue, ObservationBackend, ObservationError, ObserverOptions, RawEvent, Root, Sink, Targets, Waited,
};
use crate::remove::quarantine::rename::identity;

/// The parent's buffer. The system keeps a buffer of the same size between
/// reads, so a larger one absorbs a burst (a sibling's build) while the
/// thread is busy.
const LARGE_BUFFER_BYTES: usize = 1024 * 1024;

/// The limit for a directory on a network share; a larger buffer fails there
/// with `ERROR_INVALID_PARAMETER`.
const NETWORK_BUFFER_BYTES: usize = 64 * 1024;

/// Every change but a read (`LAST_ACCESS`).
const FILTER: FILE_NOTIFY_CHANGE = FILE_NOTIFY_CHANGE_FILE_NAME
    | FILE_NOTIFY_CHANGE_DIR_NAME
    | FILE_NOTIFY_CHANGE_ATTRIBUTES
    | FILE_NOTIFY_CHANGE_SIZE
    | FILE_NOTIFY_CHANGE_LAST_WRITE
    | FILE_NOTIFY_CHANGE_CREATION
    | FILE_NOTIFY_CHANGE_SECURITY;

fn setup(path: &Path) -> impl FnOnce(io::Error) -> ObservationError + use<> {
    let path = path.to_path_buf();
    move |source| ObservationError::Setup { path, source }
}

/// A kernel handle closed on drop.
struct OwnedHandle(HANDLE);

// SAFETY: a kernel handle may be used and closed from any thread.
unsafe impl Send for OwnedHandle {}
unsafe impl Sync for OwnedHandle {}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: the handle is open and owned here.
        unsafe { CloseHandle(self.0) };
    }
}

fn wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
}

/// Opens a directory for watching without following a link at it, sharing
/// everything so the watch never blocks a rename or deletion.
fn open_directory(path: &Path) -> io::Result<OwnedHandle> {
    let name = wide(path);
    // SAFETY: `name` is NUL-terminated; the result is checked.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            FILE_LIST_DIRECTORY,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_OVERLAPPED,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE { Err(io::Error::last_os_error()) } else { Ok(OwnedHandle(handle)) }
}

fn event(manual_reset: bool) -> io::Result<OwnedHandle> {
    // SAFETY: no attributes or name; the result is checked.
    let handle = unsafe { CreateEventW(std::ptr::null(), i32::from(manual_reset), FALSE, std::ptr::null()) };
    if handle.is_null() { Err(io::Error::last_os_error()) } else { Ok(OwnedHandle(handle)) }
}

/// A directory that must be a directory itself, not a link to one.
fn require_directory(path: &Path) -> Result<(), ObservationError> {
    let metadata = std::fs::symlink_metadata(path).map_err(setup(path))?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        Ok(())
    } else {
        Err(setup(path)(io::Error::new(io::ErrorKind::NotADirectory, "not a directory (links are not followed)")))
    }
}

/// What a completed read means.
#[derive(Debug, PartialEq, Eq)]
enum Completion {
    /// `bytes` of notifications to parse.
    Notifications(usize),
    /// The system's buffer overflowed; changes were dropped.
    Overflow,
    /// The watch ended.
    Ended,
}

fn completion(result: Result<u32, u32>) -> Completion {
    match result {
        Ok(0) | Err(ERROR_NOTIFY_ENUM_DIR) => Completion::Overflow,
        Ok(bytes) => Completion::Notifications(bytes as usize),
        Err(_) => Completion::Ended,
    }
}

/// The layout a watch asks the system for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    /// `FILE_NOTIFY_EXTENDED_INFORMATION`: each entry carries the entry's
    /// attributes, so its type is known without opening it.
    Extended,
    /// `FILE_NOTIFY_INFORMATION`, for a volume that refuses the extended
    /// class (ReFS does); entries carry no type.
    Plain,
}

impl Format {
    fn class(self) -> READ_DIRECTORY_NOTIFY_INFORMATION_CLASS {
        match self {
            Self::Extended => ReadDirectoryNotifyExtendedInformation,
            Self::Plain => ReadDirectoryNotifyInformation,
        }
    }

    /// Byte offsets of the attributes (extended only), the name's length, and
    /// the name in one entry.
    fn offsets(self) -> (Option<usize>, usize, usize) {
        match self {
            Self::Extended => (
                Some(offset_of!(FILE_NOTIFY_EXTENDED_INFORMATION, FileAttributes)),
                offset_of!(FILE_NOTIFY_EXTENDED_INFORMATION, FileNameLength),
                offset_of!(FILE_NOTIFY_EXTENDED_INFORMATION, FileName),
            ),
            Self::Plain => (None, offset_of!(FILE_NOTIFY_INFORMATION, FileNameLength), offset_of!(FILE_NOTIFY_INFORMATION, FileName)),
        }
    }
}

/// One parsed notification: the action, the entry's attributes when the
/// format carries them, and its name relative to the watched directory.
type Notification = (u32, Option<u32>, OsString);

/// The notifications in a completed buffer; `Err` when an entry runs past the
/// buffer.
fn parse(buffer: &[u8], format: Format) -> Result<Vec<Notification>, String> {
    let (attributes_at, length_at, name_at) = format.offsets();
    let word = |at: usize| u32::from_le_bytes(buffer[at..at + 4].try_into().expect("four bytes"));
    let mut entries = Vec::new();
    let mut offset = 0;
    loop {
        if offset + name_at > buffer.len() {
            return Err(format!("a notification at byte {offset} runs past the {} bytes read", buffer.len()));
        }
        let (next, action, length) = (word(offset) as usize, word(offset + 4), word(offset + length_at) as usize);
        let attributes = attributes_at.map(|at| word(offset + at));
        let name = buffer
            .get(offset + name_at..offset + name_at + length)
            .ok_or_else(|| format!("a name at byte {offset} runs past the {} bytes read", buffer.len()))?;
        let units: Vec<u16> = name.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
        entries.push((action, attributes, OsString::from_wide(&units)));
        if next == 0 {
            return Ok(entries);
        }
        offset += next;
    }
}

fn kind(action: u32) -> EventKind {
    match action {
        FILE_ACTION_ADDED => EventKind::Created,
        FILE_ACTION_REMOVED => EventKind::Removed,
        FILE_ACTION_RENAMED_OLD_NAME => EventKind::MovedFrom,
        FILE_ACTION_RENAMED_NEW_NAME => EventKind::MovedTo,
        // `FILE_ACTION_MODIFIED`, and any action a later Windows adds: a
        // change all the same.
        _ => EventKind::Modified,
    }
}

/// What the delivery thread and the session share.
struct Shared {
    queue: EventQueue,
    original_name: OsString,
    quarantine_name: OsString,
    /// Names `<prefix><number>` are this session's markers.
    marker_prefix: OsString,
    moved: AtomicBool,
    /// Per handle ([`Root::Parent`] or [`Root::Admin`]): its highest marker
    /// (`u64::MAX` once its watch ended or was released, so no drain waits
    /// for it) and its in-scope events.
    delivery: Delivery<Root>,
    /// Held by a test to keep the thread from reading, so the system's
    /// buffer fills.
    #[cfg(test)]
    gate: std::sync::Mutex<()>,
    /// Set by a test, from the start if it asks, to hold every completion,
    /// as a stalled watch would.
    #[cfg(test)]
    held: AtomicBool,
}

impl Shared {
    fn ended(&self, root: Root) {
        self.delivery.end(root);
    }

    /// The marker number `name` carries, if it is one of this session's.
    fn marker(&self, name: &OsStr) -> Option<u64> {
        name.to_str()?.strip_prefix(self.marker_prefix.to_str()?)?.parse().ok()
    }

    /// `path`, reported by the handle on `root`, as a [`Root`] and a path
    /// relative to it, or `None` outside the session's scope. An entry of the
    /// parent named like the tree (in any ASCII case, as the file system
    /// matches names) is the tree; the tree's root directory itself stays the
    /// parent's entry, as on Linux. Parent entries are reported under the
    /// session's own names.
    fn locate(&self, root: Root, path: &Path) -> Option<(Root, PathBuf)> {
        if root == Root::Admin {
            return Some((Root::Admin, path.to_path_buf()));
        }
        let mut components = path.components();
        let Some(Component::Normal(name)) = components.next() else {
            return None;
        };
        let rest = components.as_path();
        let (tree, other) = if self.moved.load(Ordering::SeqCst) {
            (&self.quarantine_name, &self.original_name)
        } else {
            (&self.original_name, &self.quarantine_name)
        };
        if name.eq_ignore_ascii_case(tree) {
            Some(if rest.as_os_str().is_empty() {
                (Root::Parent, PathBuf::from(tree))
            } else {
                (Root::Tree, rest.to_path_buf())
            })
        } else if name.eq_ignore_ascii_case(other) {
            Some((Root::Parent, Path::new(other).join(rest)))
        } else {
            None
        }
    }

    /// Turns one notification from the handle on `root` into a raw event, or
    /// records a marker. Nothing is opened: a handle on an entry inside the
    /// tree, however brief, makes the move fail with `ERROR_ACCESS_DENIED`.
    fn deliver(&self, root: Root, action: u32, attributes: Option<u32>, name: &OsStr, received: SystemTime) {
        if let Some(number) = self.marker(name) {
            if action == FILE_ACTION_ADDED {
                self.delivery.acknowledge(root, number);
            }
            return;
        }
        let Some((located, relative)) = self.locate(root, Path::new(name)) else {
            return;
        };
        let kind = kind(action);
        let directory = match kind {
            EventKind::Removed | EventKind::MovedFrom => None,
            _ => attributes.map(|attributes| attributes & FILE_ATTRIBUTE_DIRECTORY != 0),
        };
        if self.queue.push(RawEvent { root: located, path: relative, kind, directory, received }) {
            self.delivery.event(root);
        }
    }
}

/// One watched directory and its outstanding read. The thread owns it; the
/// buffer and `OVERLAPPED` stay put until the read is no longer pending.
struct Watch {
    root: Root,
    directory: PathBuf,
    subtree: bool,
    handle: OwnedHandle,
    event: OwnedHandle,
    overlapped: Box<OVERLAPPED>,
    /// `u64` for the alignment the extended entries require.
    buffer: Vec<u64>,
    format: Format,
    pending: bool,
}

// SAFETY: `OVERLAPPED` holds only the event handle and kernel-written
// fields; a watch is opened on the session's thread and then used by the
// delivery thread alone, which issues its reads.
unsafe impl Send for Watch {}

impl Watch {
    fn open(root: Root, directory: &Path, subtree: bool, buffer_bytes: usize) -> Result<Self, ObservationError> {
        require_directory(directory)?;
        Ok(Self {
            root,
            directory: directory.to_path_buf(),
            subtree,
            handle: open_directory(directory).map_err(setup(directory))?,
            event: event(true).map_err(setup(directory))?,
            // SAFETY: an all-zero `OVERLAPPED` is valid.
            overlapped: Box::new(unsafe { std::mem::zeroed() }),
            buffer: vec![0; buffer_bytes / size_of::<u64>()],
            format: Format::Extended,
            pending: false,
        })
    }

    fn issue(&mut self) -> io::Result<()> {
        // SAFETY: no read is pending, so nothing else uses `overlapped`,
        // `buffer`, or the event; both stay allocated until the read is no
        // longer pending (`drop` waits for it).
        unsafe {
            *self.overlapped = std::mem::zeroed();
            self.overlapped.hEvent = self.event.0;
            ResetEvent(self.event.0);
            let issued = ReadDirectoryChangesExW(
                self.handle.0,
                self.buffer.as_mut_ptr().cast(),
                (self.buffer.len() * size_of::<u64>()) as u32,
                i32::from(self.subtree),
                FILTER,
                std::ptr::null_mut(),
                &mut *self.overlapped,
                None,
                self.format.class(),
            );
            if issued == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        self.pending = true;
        Ok(())
    }

    /// Issues the first read. A volume that refuses the request is asked
    /// again with a network share's buffer limit, then in the plain format:
    /// a share refuses a larger buffer with `ERROR_INVALID_PARAMETER`, and
    /// ReFS refuses the extended format with `ERROR_INVALID_FUNCTION`.
    fn start(&mut self) -> io::Result<()> {
        let requested = self.buffer.len();
        let network = NETWORK_BUFFER_BYTES / size_of::<u64>();
        let lengths = if requested > network { vec![requested, network] } else { vec![requested] };
        let refused = |error: &io::Error| {
            [ERROR_INVALID_PARAMETER, ERROR_INVALID_FUNCTION, ERROR_NOT_SUPPORTED].iter().any(|code| error.raw_os_error() == Some(*code as i32))
        };
        let mut result = Ok(());
        for format in [Format::Extended, Format::Plain] {
            for &length in &lengths {
                self.format = format;
                if self.buffer.len() != length {
                    self.buffer = vec![0; length];
                }
                result = self.issue();
                match &result {
                    Err(error) if refused(error) => {}
                    _ => return result,
                }
            }
        }
        result
    }

    /// Handles the signaled read and issues the next. `false` once the watch
    /// has ended.
    fn complete(&mut self, shared: &Shared) -> bool {
        #[cfg(test)]
        let _gate = shared.gate.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        #[cfg(test)]
        while shared.held.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let mut bytes = 0;
        // SAFETY: the read was issued on this handle with this `OVERLAPPED`,
        // and its event is signaled, so it is no longer pending.
        let result = unsafe { GetOverlappedResult(self.handle.0, &*self.overlapped, &mut bytes, FALSE) };
        self.pending = false;
        let result = if result == 0 { Err(io::Error::last_os_error().raw_os_error().unwrap_or(0) as u32) } else { Ok(bytes) };
        let received = SystemTime::now();
        match completion(result) {
            Completion::Overflow => shared.queue.lose(CoverageLost::Overflow),
            Completion::Ended => {
                shared.queue.lose(CoverageLost::WatchLost { root: self.root });
                return false;
            }
            Completion::Notifications(length) => {
                // SAFETY: the system wrote `length` bytes (at most the
                // buffer's size) into the buffer.
                let bytes = unsafe { std::slice::from_raw_parts(self.buffer.as_ptr().cast::<u8>(), length) };
                match parse(bytes, self.format) {
                    Ok(entries) => {
                        for (action, attributes, name) in entries {
                            shared.deliver(self.root, action, attributes, &name, received);
                        }
                    }
                    Err(message) => shared.queue.lose(CoverageLost::ReadFailed { message }),
                }
            }
        }
        match self.issue() {
            Ok(()) => true,
            Err(_) => {
                shared.queue.lose(CoverageLost::WatchLost { root: self.root });
                false
            }
        }
    }
}

impl Drop for Watch {
    /// Cancels the outstanding read and waits for it, so the system no longer
    /// writes into the buffer when it is freed.
    fn drop(&mut self) {
        if self.pending {
            let mut bytes = 0;
            // SAFETY: the read is pending on this handle with this
            // `OVERLAPPED`; waiting ends it before the buffer is freed.
            unsafe {
                CancelIoEx(self.handle.0, &*self.overlapped);
                GetOverlappedResult(self.handle.0, &*self.overlapped, &mut bytes, TRUE);
            }
        }
    }
}

/// The session's requests to the delivery thread.
struct Control {
    /// Manual reset: stays set until the thread has gone.
    stop: OwnedHandle,
    /// Auto reset: one release per signal.
    release: OwnedHandle,
}

pub(super) struct ReadDirectoryChanges {
    shared: Arc<Shared>,
    control: Arc<Control>,
    thread: Option<JoinHandle<()>>,
    released: mpsc::Receiver<()>,
    admin_released: bool,
    parent: PathBuf,
    admin: PathBuf,
    parent_identity: FileIdentity,
    admin_identity: FileIdentity,
    /// The last marker written.
    marker: u64,
    policy: WaitPolicy,
    round_trips: RoundTrips,
    /// Dropped by the delivery thread as it ends.
    #[cfg(test)]
    thread_alive: Arc<()>,
}

impl ReadDirectoryChanges {
    /// `moved` says the tree is already at the quarantine (a resumed
    /// removal).
    pub fn start(targets: &Targets, options: ObserverOptions, moved: bool, interrupted: &dyn Fn() -> bool) -> Result<Self, ObservationError> {
        Self::start_with_buffer(targets, options, LARGE_BUFFER_BYTES, moved, interrupted, false)
    }

    /// `held` (tests only) holds every completion from the start, so the
    /// startup wait sees no delivery.
    fn start_with_buffer(
        targets: &Targets,
        options: ObserverOptions,
        parent_buffer: usize,
        moved: bool,
        interrupted: &dyn Fn() -> bool,
        #[cfg_attr(not(test), expect(unused_variables, reason = "only a test holds delivery"))] held: bool,
    ) -> Result<Self, ObservationError> {
        require_directory(if moved { targets.quarantine() } else { targets.original() })?;
        let parent = Watch::open(Root::Parent, targets.parent(), true, parent_buffer)?;
        let admin = Watch::open(Root::Admin, targets.admin(), false, NETWORK_BUFFER_BYTES)?;
        let parent_identity = identity(targets.parent()).map_err(setup(targets.parent()))?;
        let admin_identity = identity(targets.admin()).map_err(setup(targets.admin()))?;
        let mut random = [0_u8; 8];
        getrandom::fill(&mut random).map_err(|error| setup(targets.original())(io::Error::other(error)))?;
        let shared = Arc::new(Shared {
            queue: EventQueue::new(targets.clone(), options.queue_capacity),
            original_name: targets.original_name().to_os_string(),
            quarantine_name: targets.quarantine_name().to_os_string(),
            marker_prefix: format!(".wt-observe-{:016x}-", u64::from_ne_bytes(random)).into(),
            moved: AtomicBool::new(moved),
            delivery: Delivery::new([Root::Parent, Root::Admin]),
            #[cfg(test)]
            gate: std::sync::Mutex::new(()),
            #[cfg(test)]
            held: AtomicBool::new(held),
        });
        let control = Arc::new(Control {
            stop: event(true).map_err(setup(targets.original()))?,
            release: event(false).map_err(setup(targets.original()))?,
        });
        #[cfg(test)]
        let thread_alive = Arc::new(());
        #[cfg(test)]
        let alive = Arc::clone(&thread_alive);

        let (started_sender, started) = mpsc::sync_channel(1);
        let (released_sender, released) = mpsc::sync_channel(1);
        let (thread_shared, thread_control) = (Arc::clone(&shared), Arc::clone(&control));
        // Reads are issued on the thread that waits for them: Windows cancels
        // a thread's outstanding reads when that thread exits.
        let thread = std::thread::Builder::new()
            .name("wt-read-directory-changes".into())
            .spawn(move || {
                #[cfg(test)]
                let _alive = alive;
                run(parent, admin, &thread_shared, &thread_control, &started_sender, &released_sender);
            })
            .map_err(setup(targets.original()))?;
        match started.recv() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                let _ = thread.join();
                return Err(error);
            }
            Err(_) => {
                let _ = thread.join();
                return Err(setup(targets.original())(io::Error::other("the watch thread ended before starting")));
            }
        }
        let mut adapter = Self {
            shared,
            control,
            thread: Some(thread),
            released,
            admin_released: false,
            parent: targets.parent().to_path_buf(),
            admin: targets.admin().to_path_buf(),
            parent_identity,
            admin_identity,
            marker: 0,
            policy: options.wait,
            round_trips: RoundTrips::default(),
            #[cfg(test)]
            thread_alive,
        };
        // Confirms that this volume delivers notifications at all (a network
        // share may accept the call and never complete it).
        if adapter.barrier(interrupted) == Waited::Interrupted {
            return Err(ObservationError::Interrupted);
        }
        if let Some(loss) = adapter.shared.queue.take_loss() {
            return Err(ObservationError::not_delivering(ObservationBackend::ReadDirectoryChanges, loss, adapter.round_trips));
        }
        Ok(adapter)
    }

    /// Delivers every change made in a watched directory before the call into
    /// the queue, or records why that cannot be confirmed.
    fn barrier(&mut self, interrupted: &dyn Fn() -> bool) -> Waited {
        self.marker += 1;
        let number = self.marker;
        let name = {
            let mut name = self.shared.marker_prefix.clone();
            name.push(number.to_string());
            name
        };
        let mut directories = vec![(Root::Parent, &self.parent)];
        if !self.admin_released {
            directories.push((Root::Admin, &self.admin));
        }
        let mut markers = Vec::new();
        let mut awaited = Vec::new();
        for (root, directory) in directories {
            if self.shared.delivery.delivered(root).marker == u64::MAX {
                continue;
            }
            let created = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .attributes(FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_TEMPORARY)
                .custom_flags(FILE_FLAG_DELETE_ON_CLOSE)
                .open(directory.join(&name));
            match created {
                Ok(marker) => {
                    markers.push(marker);
                    awaited.push(root);
                }
                Err(error) => self.shared.queue.lose(CoverageLost::ReadFailed { message: format!("writing the drain marker failed: {error}") }),
            }
        }
        let mut wait = WaitState::new(self.policy, Instant::now(), awaited);
        let queue = &self.shared.queue;
        let outcome = await_marker(&self.shared.delivery, &mut wait, number, &|| queue.has_loss(), interrupted);
        // Closing deletes them.
        drop(markers);
        match outcome {
            Outcome::Confirmed { round_trip } => {
                self.round_trips.record(round_trip);
                Waited::Done
            }
            Outcome::Lost => Waited::Done,
            Outcome::Interrupted => Waited::Interrupted,
            Outcome::Stalled { waited, key } => {
                queue.lose(CoverageLost::DeliveryStalled { waited, watch: Some(key) });
                Waited::Done
            }
            Outcome::Ceiling { waited, key } => {
                queue.lose(CoverageLost::DrainCeiling { waited, watch: key });
                Waited::Done
            }
        }
    }

    /// The parent and (until released) the admin directory must still be the
    /// directories the session started on; a handle reports neither its own
    /// directory's move nor, always, its deletion.
    fn check_identities(&self) {
        if identity(&self.parent).ok().as_ref() != Some(&self.parent_identity) {
            self.shared.queue.lose(CoverageLost::WatchLost { root: Root::Parent });
        }
        if !self.admin_released && identity(&self.admin).ok().as_ref() != Some(&self.admin_identity) {
            self.shared.queue.lose(CoverageLost::WatchLost { root: Root::Admin });
        }
    }
}

/// The delivery thread: issues the first reads, reports whether they started,
/// then handles completions until stopped. Watches are dropped (their reads
/// canceled and awaited) as it returns.
fn run(
    parent: Watch,
    admin: Watch,
    shared: &Shared,
    control: &Control,
    started: &mpsc::SyncSender<Result<(), ObservationError>>,
    released: &mpsc::SyncSender<()>,
) {
    let mut parent = Some(parent);
    let mut admin = Some(admin);
    for watch in [&mut parent, &mut admin].into_iter().flatten() {
        if let Err(error) = watch.start() {
            let _ = started.send(Err(setup(&watch.directory)(error)));
            return;
        }
    }
    let _ = started.send(Ok(()));
    loop {
        let mut waits = vec![control.stop.0, control.release.0];
        let watched: Vec<Root> = [&parent, &admin].into_iter().flatten().map(|watch| watch.root).collect();
        waits.extend([&parent, &admin].into_iter().flatten().map(|watch| watch.event.0));
        // SAFETY: every handle in `waits` stays open for the call.
        let signaled = unsafe { WaitForMultipleObjects(waits.len() as u32, waits.as_ptr(), FALSE, INFINITE) };
        match signaled.wrapping_sub(WAIT_OBJECT_0) {
            0 => return,
            1 => {
                admin = None;
                shared.ended(Root::Admin);
                let _ = released.send(());
            }
            index if (index as usize) < waits.len() => {
                let root = watched[index as usize - 2];
                let slot = if root == Root::Admin { &mut admin } else { &mut parent };
                if let Some(watch) = slot
                    && !watch.complete(shared)
                {
                    *slot = None;
                    shared.ended(root);
                }
            }
            _ => {
                shared.queue.lose(CoverageLost::ReadFailed { message: format!("waiting for notifications failed: {}", io::Error::last_os_error()) });
                shared.ended(Root::Parent);
                shared.ended(Root::Admin);
                // Wait for the stop rather than spin.
                // SAFETY: the stop event stays open for the call.
                unsafe { windows_sys::Win32::System::Threading::WaitForSingleObject(control.stop.0, INFINITE) };
                return;
            }
        }
    }
}

impl Backend for ReadDirectoryChanges {
    fn drain(&mut self, sink: &mut Sink<'_>, interrupted: &dyn Fn() -> bool) -> Waited {
        self.check_identities();
        // Lost coverage stays lost: there is nothing left to confirm.
        let waited = if sink.lost.is_some() { Waited::Done } else { self.barrier(interrupted) };
        self.shared.queue.take_into(sink);
        waited
    }

    /// Delivers first, so a change made before the move is still labeled with
    /// the tree; from here on, the quarantine's entry is the tree.
    fn moved_to_quarantine(&mut self, interrupted: &dyn Fn() -> bool) -> Waited {
        let waited = self.barrier(interrupted);
        self.shared.moved.store(true, Ordering::SeqCst);
        waited
    }

    /// Delivers first, then closes the admin handle before returning, so the
    /// operation's own Git step can delete the directory. An interrupted wait
    /// keeps the handle open.
    fn release_admin(&mut self, interrupted: &dyn Fn() -> bool) -> Waited {
        if self.admin_released {
            return Waited::Done;
        }
        if self.barrier(interrupted) == Waited::Interrupted {
            return Waited::Interrupted;
        }
        self.admin_released = true;
        // SAFETY: the event stays open while the session exists.
        unsafe { SetEvent(self.control.release.0) };
        let _ = self.released.recv();
        Waited::Done
    }

    fn queued_loss(&self) -> Option<CoverageLost> {
        self.shared.queue.loss()
    }

    fn handles(&self) -> Vec<(EvidenceKind, u64)> {
        Vec::new()
    }

    fn backend(&self) -> ObservationBackend {
        ObservationBackend::ReadDirectoryChanges
    }

    fn round_trips(&self) -> RoundTrips {
        self.round_trips
    }
}

impl Drop for ReadDirectoryChanges {
    fn drop(&mut self) {
        #[cfg(test)]
        self.shared.held.store(false, Ordering::SeqCst);
        // SAFETY: the event stays open until `control` is dropped, after the
        // join.
        unsafe { SetEvent(self.control.stop.0) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
