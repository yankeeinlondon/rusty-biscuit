//! Activity observation while a removal moves and deletes a checkout.
//!
//! One [`Observation`] watches three places from before the move until the
//! removal ends:
//!
//! - the original's **parent**, for entries named like the original (its
//!   recreation) or the quarantine; changes to other siblings are dropped;
//! - the checkout **tree**, wherever it currently is, for every descendant;
//! - the worktree's **admin** directory, for the files behind its identity and
//!   index ([`ADMIN_FILES`], plus `sharedindex.*`).
//!
//! Each platform has a native adapter behind the same contract:
//!
//! - [`Observation::drain`] is a barrier: every event caused before the call
//!   is in the returned [`Drain`];
//! - a directory's removal or rename may be reported in place of changes
//!   below it that were not yet delivered (FSEvents coalesces them), so a
//!   caller that must see those changes drains before removing or renaming
//!   the directory;
//! - coverage loss (a kernel overflow, a full queue, a watch lost to anything
//!   but the operation's own deletion) is a typed [`CoverageLost`] that stays
//!   set for the rest of the session, never a silent downgrade;
//! - the queue between the platform and `drain` is bounded
//!   ([`ObserverOptions::queue_capacity`]); a full queue is coverage loss;
//! - handles the session holds inside the checkout are registered with
//!   [`OwnedHandles`], so the decisive usage query does not refuse the
//!   removal over its own watcher;
//! - dropping the session releases every watch and leaves no thread behind.
//!
//! Observation that cannot be established is an [`ObservationError`] before
//! anything is moved; the removal stops with exit 4. Events are facts, not
//! verdicts: deciding which are the operation's own deletions is the deletion
//! engine's job.

pub mod deletion;
#[cfg(target_os = "macos")]
mod fsevents;
#[cfg(test)]
pub(crate) mod injected;
#[cfg(target_os = "linux")]
mod inotify;
pub mod inventory;
#[cfg(windows)]
mod read_directory_changes;

#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use sniff::filesystem::query::EvidenceKind;

use super::usage::OwnedHandles;

/// Admin-directory files whose changes are reported: the worktree's identity
/// (`HEAD`, `gitdir`, `locked`) and its approved index (`index`,
/// `index.lock`). Split-index `sharedindex.*` files are reported too.
pub const ADMIN_FILES: [&str; 5] = ["HEAD", "gitdir", "index", "index.lock", "locked"];

/// Default bound of events held between the platform and [`Observation::drain`].
pub const DEFAULT_QUEUE_CAPACITY: usize = 16_384;

/// What one session watches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Targets {
    original: PathBuf,
    quarantine: PathBuf,
    admin: PathBuf,
}

impl Targets {
    /// `None` unless `original` and `quarantine` are distinct named siblings.
    pub fn new(original: &Path, quarantine: &Path, admin: &Path) -> Option<Self> {
        let (Some(parent), Some(original_name)) = (original.parent(), original.file_name()) else {
            return None;
        };
        let quarantine_name = quarantine.file_name()?;
        (quarantine.parent() == Some(parent) && original_name != quarantine_name).then(|| Self {
            original: original.to_path_buf(),
            quarantine: quarantine.to_path_buf(),
            admin: admin.to_path_buf(),
        })
    }

    #[allow(missing_docs)]
    pub fn original(&self) -> &Path {
        &self.original
    }

    #[allow(missing_docs)]
    pub fn quarantine(&self) -> &Path {
        &self.quarantine
    }

    #[allow(missing_docs)]
    pub fn admin(&self) -> &Path {
        &self.admin
    }

    /// The directory holding the original and the quarantine.
    pub fn parent(&self) -> &Path {
        self.original.parent().expect("checked by Targets::new")
    }

    fn original_name(&self) -> &OsStr {
        self.original.file_name().expect("checked by Targets::new")
    }

    fn quarantine_name(&self) -> &OsStr {
        self.quarantine.file_name().expect("checked by Targets::new")
    }

    /// `raw` with its [`Scope`], or `None` when it is outside what the session
    /// reports (a sibling, the parent itself, an unrelated admin file).
    fn classify(&self, raw: RawEvent) -> Option<ObservedEvent> {
        let scope = match raw.root {
            Root::Tree => Scope::Tree(raw.path),
            Root::Parent => {
                let (name, rest) = split_first(&raw.path)?;
                if name == self.original_name() {
                    Scope::Original(rest)
                } else if name == self.quarantine_name() {
                    Scope::Quarantine(rest)
                } else {
                    return None;
                }
            }
            Root::Admin => {
                let (name, rest) = split_first(&raw.path)?;
                if !rest.as_os_str().is_empty() || !is_admin_file(name) {
                    return None;
                }
                Scope::Admin(name.to_os_string())
            }
        };
        Some(ObservedEvent { scope, kind: raw.kind, directory: raw.directory, received: raw.received })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserverOptions {
    pub queue_capacity: usize,
}

impl Default for ObserverOptions {
    fn default() -> Self {
        Self { queue_capacity: DEFAULT_QUEUE_CAPACITY }
    }
}

/// One of the three watched places.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Root {
    Parent,
    Tree,
    Admin,
}

/// Where an event happened.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Inside the checkout tree, before or after the move; empty is the tree's
    /// root directory itself.
    Tree(PathBuf),
    /// The parent's entry named like the original (empty), or a path below
    /// that entry that is not the tree, such as content written into a
    /// recreated original.
    Original(PathBuf),
    /// The parent's entry named like the quarantine, or a path below it that
    /// is not the tree.
    Quarantine(PathBuf),
    /// One of the admin files.
    Admin(OsString),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    Created,
    Removed,
    /// Content written, or a file opened for writing was closed.
    Modified,
    /// Permissions, ownership, timestamps, or link count.
    Metadata,
    /// Renamed away from this path.
    MovedFrom,
    /// Renamed onto this path.
    MovedTo,
}

/// One notification. `received` is when `wt` read it, not when the change
/// happened; no process is attributed to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedEvent {
    pub scope: Scope,
    pub kind: EventKind,
    /// `None` when the platform does not say.
    pub directory: Option<bool>,
    pub received: SystemTime,
}

/// Why events may be missing. Once lost, coverage stays lost: nothing a
/// later drain returns proves that nothing else happened.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoverageLost {
    #[error("the platform dropped events (its queue overflowed)")]
    Overflow,
    #[error("more than {capacity} events arrived between two checks")]
    QueueFull { capacity: usize },
    #[error("the watch on the {root:?} stopped before the removal ended")]
    WatchLost { root: Root },
    #[error("reading events failed: {message}")]
    ReadFailed { message: String },
    /// The platform did not confirm, within `waited`, that every earlier
    /// change had been delivered.
    #[error("the platform did not confirm delivery within {waited:?}")]
    DrainTimedOut { waited: std::time::Duration },
}

/// What one [`Observation::drain`] returned.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct Drain {
    /// In delivery order, including events received before coverage was lost.
    pub events: Vec<ObservedEvent>,
    /// Set from the first loss onward, on every later drain too.
    pub lost: Option<CoverageLost>,
}

/// Observation could not be established. Nothing has been moved yet; the
/// removal stops as an environment limit (exit 4), keeping the checkout and
/// its registration.
#[derive(Debug, thiserror::Error)]
pub enum ObservationError {
    #[error("activity observation is not available here: {reason}")]
    Unsupported { reason: &'static str },
    #[error("could not watch {}: {source}", path.display())]
    Setup { path: PathBuf, source: std::io::Error },
}

/// An event as a platform adapter reports it: relative to the [`Root`] whose
/// watch produced it. A tree event's path is relative to the tree; a parent or
/// admin event's path starts with the entry's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawEvent {
    pub root: Root,
    pub path: PathBuf,
    pub kind: EventKind,
    pub directory: Option<bool>,
    pub received: SystemTime,
}

/// A native adapter. It reports raw events into a [`Sink`], which filters,
/// classifies, and bounds them.
pub(crate) trait Backend: Send {
    /// Moves every event caused before this call into `sink`.
    fn drain(&mut self, sink: &mut Sink<'_>);
    /// The operation moved the tree to the quarantine. Path-based adapters
    /// map paths differently from here on; handle-based ones need nothing.
    fn moved_to_quarantine(&mut self) {}
    /// Stops watching the admin directory before the operation's own Git step
    /// deletes it. Its later disappearance is not coverage loss.
    fn release_admin(&mut self);
    /// Handles this adapter holds that a usage query of the checkout reports.
    fn handles(&self) -> Vec<(EvidenceKind, u64)>;
}

/// The activity observer for one removal.
pub struct Observation {
    targets: Targets,
    options: ObserverOptions,
    backend: Box<dyn Backend>,
    moved: bool,
    lost: Option<CoverageLost>,
}

impl std::fmt::Debug for Observation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Observation")
            .field("targets", &self.targets)
            .field("moved", &self.moved)
            .field("lost", &self.lost)
            .finish_non_exhaustive()
    }
}

impl Observation {
    /// Starts watching with this platform's native adapter. The tree must
    /// still be at the original path.
    pub fn start(targets: Targets, options: ObserverOptions) -> Result<Self, ObservationError> {
        let backend = native_backend(&targets, options, false)?;
        Ok(Self::with_backend(targets, options, backend))
    }

    /// Starts watching a tree that an earlier, interrupted attempt already
    /// moved to the quarantine, which must be a real directory.
    pub fn start_at_quarantine(targets: Targets, options: ObserverOptions) -> Result<Self, ObservationError> {
        let backend = native_backend(&targets, options, true)?;
        let mut observation = Self::with_backend(targets, options, backend);
        observation.moved = true;
        Ok(observation)
    }

    pub(crate) fn with_backend(targets: Targets, options: ObserverOptions, backend: Box<dyn Backend>) -> Self {
        Self { targets, options, backend, moved: false, lost: None }
    }

    #[allow(missing_docs)]
    pub fn targets(&self) -> &Targets {
        &self.targets
    }

    /// Where the tree is now: the original path, or the quarantine once the
    /// operation reported its move.
    pub fn tree_location(&self) -> &Path {
        if self.moved { &self.targets.quarantine } else { &self.targets.original }
    }

    /// Every event caused before this call, then the coverage state.
    pub fn drain(&mut self) -> Drain {
        let mut sink = Sink {
            targets: &self.targets,
            capacity: self.options.queue_capacity,
            events: Vec::new(),
            lost: self.lost.take(),
        };
        self.backend.drain(&mut sink);
        self.lost = sink.lost;
        Drain { events: sink.events, lost: self.lost.clone() }
    }

    /// Reports the operation's own move: later events in the tree belong to
    /// the quarantine. Call it right after the rename succeeds.
    pub fn moved_to_quarantine(&mut self) {
        self.moved = true;
        self.backend.moved_to_quarantine();
    }

    /// Stops watching the admin directory. Call it after the final index
    /// check and before the step that unregisters the worktree.
    pub fn release_admin(&mut self) {
        self.backend.release_admin();
    }

    /// Registers this session's handles before the decisive usage query.
    pub fn register_handles(&self, own: &mut OwnedHandles) {
        for (kind, descriptor) in self.backend.handles() {
            own.register(kind, descriptor);
        }
    }
}

/// `moved`: the tree is already at the quarantine.
#[cfg(target_os = "linux")]
fn native_backend(targets: &Targets, _options: ObserverOptions, moved: bool) -> Result<Box<dyn Backend>, ObservationError> {
    let tree = if moved { targets.quarantine() } else { targets.original() };
    Ok(Box::new(inotify::Inotify::start(targets, tree)?))
}

#[cfg(target_os = "macos")]
fn native_backend(targets: &Targets, options: ObserverOptions, moved: bool) -> Result<Box<dyn Backend>, ObservationError> {
    Ok(Box::new(fsevents::FsEvents::start(targets, options, moved)?))
}

#[cfg(windows)]
fn native_backend(targets: &Targets, options: ObserverOptions, moved: bool) -> Result<Box<dyn Backend>, ObservationError> {
    Ok(Box::new(read_directory_changes::ReadDirectoryChanges::start(targets, options, moved)?))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn native_backend(_targets: &Targets, _options: ObserverOptions, _moved: bool) -> Result<Box<dyn Backend>, ObservationError> {
    Err(ObservationError::Unsupported { reason: "no native adapter exists for this platform yet" })
}

/// Whether `name` is an admin file whose changes are reported.
pub fn is_admin_file(name: &OsStr) -> bool {
    ADMIN_FILES.iter().any(|file| name == OsStr::new(file))
        || name.as_encoded_bytes().starts_with(b"sharedindex.")
}

/// Where a [`Backend`] delivers raw events during one drain.
pub(crate) struct Sink<'a> {
    #[cfg_attr(not(target_os = "linux"), expect(dead_code, reason = "read by `push`"))]
    targets: &'a Targets,
    capacity: usize,
    events: Vec<ObservedEvent>,
    lost: Option<CoverageLost>,
}

impl Sink<'_> {
    /// Classifies and keeps `raw`, dropping events outside the watched scope.
    /// Past the capacity, coverage is lost and nothing more is kept.
    #[cfg_attr(
        not(target_os = "linux"),
        expect(dead_code, reason = "only an adapter without a delivery thread pushes raw events into the sink")
    )]
    pub fn push(&mut self, raw: RawEvent) {
        if let Some(event) = self.targets.classify(raw) {
            self.keep(event);
        }
    }

    fn keep(&mut self, event: ObservedEvent) {
        if self.events.len() >= self.capacity {
            self.lose(CoverageLost::QueueFull { capacity: self.capacity });
        } else {
            self.events.push(event);
        }
    }

    /// Records a loss; the first one is kept.
    pub fn lose(&mut self, loss: CoverageLost) {
        self.lost.get_or_insert(loss);
    }

}

fn split_first(path: &Path) -> Option<(&OsStr, PathBuf)> {
    let mut components = path.components();
    match components.next()? {
        Component::Normal(name) => Some((name, components.as_path().to_path_buf())),
        _ => None,
    }
}

/// A bounded queue between an adapter's delivery thread and
/// [`Backend::drain`]. Pushing never waits for a reader: past the capacity
/// the event is dropped and coverage is lost. Events outside the session's
/// scope are dropped before they count, so busy siblings cannot fill it.
#[cfg_attr(
    not(any(test, target_os = "macos", windows)),
    allow(dead_code, reason = "only adapters with a delivery thread use it")
)]
#[derive(Debug)]
pub(crate) struct EventQueue {
    state: Mutex<QueueState>,
}

#[derive(Debug)]
struct QueueState {
    targets: Targets,
    events: VecDeque<ObservedEvent>,
    capacity: usize,
    lost: Option<CoverageLost>,
}

#[cfg_attr(
    not(any(test, target_os = "macos", windows)),
    allow(dead_code, reason = "only adapters with a delivery thread use it")
)]
impl EventQueue {
    pub fn new(targets: Targets, capacity: usize) -> Self {
        Self { state: Mutex::new(QueueState { targets, events: VecDeque::new(), capacity, lost: None }) }
    }

    pub fn push(&self, raw: RawEvent) {
        let mut state = self.lock();
        let Some(event) = state.targets.classify(raw) else {
            return;
        };
        if state.events.len() >= state.capacity {
            let capacity = state.capacity;
            state.lost.get_or_insert(CoverageLost::QueueFull { capacity });
        } else {
            state.events.push_back(event);
        }
    }

    pub fn lose(&self, loss: CoverageLost) {
        self.lock().lost.get_or_insert(loss);
    }

    /// Removes and returns the recorded loss, if any.
    #[cfg(any(target_os = "macos", windows))]
    pub fn take_loss(&self) -> Option<CoverageLost> {
        self.lock().lost.take()
    }

    /// Moves everything queued into `sink`, then any loss.
    pub fn take_into(&self, sink: &mut Sink<'_>) {
        let (events, lost) = {
            let mut state = self.lock();
            (std::mem::take(&mut state.events), state.lost.take())
        };
        for event in events {
            sink.keep(event);
        }
        if let Some(loss) = lost {
            sink.lose(loss);
        }
    }

    // A delivery thread that panicked mid-push leaves the queue usable; the
    // session then sees whatever it recorded.
    fn lock(&self) -> std::sync::MutexGuard<'_, QueueState> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
