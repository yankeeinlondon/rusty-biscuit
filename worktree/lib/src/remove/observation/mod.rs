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
//! - [`Observation::drain`] is a barrier on Linux and Windows: every event
//!   caused before the call is in the returned [`Drain`]. FSEvents promises
//!   no ordering, so on macOS a drain settles until a quiet marker round and
//!   an event can still arrive in a later drain (see the `fsevents` adapter);
//!   no event is dropped without a [`CoverageLost`], and the direct checks
//!   before each deletion, not the drain, decide what is safe to delete;
//! - a directory's removal or rename may be reported in place of changes
//!   below it that were not yet delivered (FSEvents coalesces them), so a
//!   caller that must see those changes drains before removing or renaming
//!   the directory;
//! - coverage loss (a kernel overflow, a full queue, a watch lost to anything
//!   but the operation's own deletion) is a typed [`CoverageLost`] that stays
//!   set for the rest of the session, never a silent downgrade;
//! - the queue between the platform and `drain` is bounded
//!   ([`ObserverOptions::queue_capacity`]); a full queue is coverage loss;
//! - confirming delivery (at startup, in each drain, before the move is
//!   labeled, and before the admin directory is released) waits under one
//!   [`WaitPolicy`]: it may take as long as delivery keeps making progress,
//!   stalls after [`WaitPolicy::stall`] without any, and never passes
//!   [`WaitPolicy::ceiling`]; either limit is coverage loss. An interrupt
//!   ends the wait without loss, and the caller stops as it does for any
//!   interrupt (a [`Drain::interrupted`], [`ObservationError::Interrupted`],
//!   or [`WaitInterrupted`]);
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
mod diagnostics;
#[cfg(target_os = "macos")]
mod fsevents;
#[cfg(test)]
pub(crate) mod injected;
#[cfg(target_os = "linux")]
mod inotify;
pub mod inventory;
#[cfg(windows)]
mod read_directory_changes;
// Linux reads until empty and compiles the loop only for its tests.
#[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
mod wait;

pub use diagnostics::{DiagnosticsHandle, ObservationBackend, ObservationDiagnostics, ObservationLoss, ObservationStage};
pub use wait::WaitPolicy;

#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{Instant, SystemTime};

use serde::{Deserialize, Serialize};
use sniff::filesystem::query::EvidenceKind;

use diagnostics::RoundTrips;

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
    /// How long confirming delivery may take; unused on Linux, where reading
    /// until empty needs no wait.
    pub wait: WaitPolicy,
}

impl Default for ObserverOptions {
    fn default() -> Self {
        Self { queue_capacity: DEFAULT_QUEUE_CAPACITY, wait: WaitPolicy::default() }
    }
}

/// One of the three watched places.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
///
/// Persisted in the report's [`ObservationLoss`]; a change to a variant is a
/// change to the report's shape.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CoverageLost {
    /// Linux and Windows: the kernel's event buffer overflowed.
    #[error("the platform dropped events (its queue overflowed)")]
    Overflow,
    /// macOS: FSEvents flagged an event as standing for changes it could not
    /// report. `raw` is the event's whole flag mask, bits without a name
    /// included; `flags` names every loss flag set in it, in
    /// [`FsEventsLossFlag`] declaration order, with no precedence among them.
    /// [`FsEventsLossFlag::MustScanSubDirs`] alone is loss too.
    #[error("FSEvents could not report every change ({}; flags {raw:#x})", flag_names(flags))]
    FsEventsDropped { raw: u32, flags: Vec<FsEventsLossFlag> },
    #[error("more than {capacity} events arrived between two checks")]
    QueueFull { capacity: usize },
    #[error("the watch on the {root:?} stopped before the removal ended")]
    WatchLost { root: Root },
    #[error("reading events failed: {message}")]
    ReadFailed { message: String },
    /// No relevant progress arrived for `waited` while confirming that every
    /// earlier change had been delivered. Confirmation could not be
    /// obtained; this does not show that events were dropped. `watch` names
    /// the stalled watch where the platform has one per place (Windows).
    #[error("could not confirm delivery of earlier changes{}: no progress for {waited:?}", on_watch(*.watch))]
    DeliveryStalled {
        #[serde(rename = "waited_ms", with = "diagnostics::duration_ms")]
        waited: std::time::Duration,
        watch: Option<Root>,
    },
    /// Confirming delivery took `waited`, the whole of one wait's ceiling,
    /// although progress kept arriving. As with a stall, nothing is known to
    /// have been dropped.
    #[error("could not confirm delivery of earlier changes{} within {waited:?}", on_watch(*.watch))]
    DrainCeiling {
        #[serde(rename = "waited_ms", with = "diagnostics::duration_ms")]
        waited: std::time::Duration,
        watch: Option<Root>,
    },
}

/// A named FSEvents loss flag, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FsEventsLossFlag {
    KernelDropped,
    UserDropped,
    /// Changes below the path were coalesced and must be rescanned: complete
    /// event coverage is unavailable even though nothing says "dropped".
    MustScanSubDirs,
}

impl std::fmt::Display for FsEventsLossFlag {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::KernelDropped => "KernelDropped",
            Self::UserDropped => "UserDropped",
            Self::MustScanSubDirs => "MustScanSubDirs",
        })
    }
}

fn flag_names(flags: &[FsEventsLossFlag]) -> String {
    flags.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")
}

fn on_watch(watch: Option<Root>) -> String {
    watch.map_or_else(String::new, |root| format!(" on the {root:?} watch"))
}

/// What one [`Observation::drain`] returned.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct Drain {
    /// In delivery order, including events received before coverage was lost.
    pub events: Vec<ObservedEvent>,
    /// Set from the first loss onward, on every later drain too.
    pub lost: Option<CoverageLost>,
    /// The interrupt ended the wait for delivery (in this drain or in the
    /// move's labeling before it): `events` is what arrived so far, and
    /// nothing confirms that it is everything.
    pub interrupted: bool,
}

/// The interrupt ended a wait for delivery before the admin directory was
/// released; it is still watched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the wait for change delivery was interrupted")]
pub struct WaitInterrupted;

/// Observation could not be established. Nothing has been moved yet; the
/// removal stops as an environment limit (exit 4), keeping the checkout and
/// its registration.
#[derive(Debug, thiserror::Error)]
pub enum ObservationError {
    #[error("activity observation is not available here: {reason}")]
    Unsupported { reason: &'static str },
    #[error("could not watch {}: {source}", path.display())]
    Setup { path: PathBuf, source: std::io::Error },
    /// The watch was set up, but confirming that it delivers lost coverage;
    /// the diagnostics' [`ObservationLoss`] says why.
    #[error("the watch did not start delivering changes: {}", loss_reason(.0))]
    NotDelivering(Box<ObservationDiagnostics>),
    /// The interrupt ended the startup wait; the caller stops as it does for
    /// an interrupt before the move, not as an environment limit.
    #[error("the removal was interrupted while observation was starting")]
    Interrupted,
}

fn loss_reason(diagnostics: &ObservationDiagnostics) -> String {
    diagnostics.loss.as_ref().map_or_else(|| "no reason was recorded".into(), |loss| loss.reason.to_string())
}

impl ObservationError {
    /// A startup confirmation that lost coverage with `loss`, after the
    /// marker round trips in `round_trips`.
    #[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code, reason = "Linux confirms nothing at startup"))]
    pub(crate) fn not_delivering(backend: ObservationBackend, loss: CoverageLost, round_trips: RoundTrips) -> Self {
        let mut diagnostics = ObservationDiagnostics::new(backend);
        diagnostics.set_round_trips(round_trips);
        diagnostics.loss = Some(ObservationLoss { stage: ObservationStage::Startup, reason: loss, entries_since_drain: None });
        Self::NotDelivering(Box::new(diagnostics))
    }

    /// The diagnostics a startup failure carries, if it got far enough to
    /// have any.
    pub fn diagnostics(&self) -> Option<&ObservationDiagnostics> {
        match self {
            Self::NotDelivering(diagnostics) => Some(diagnostics),
            _ => None,
        }
    }

    fn started_after(mut self, elapsed: std::time::Duration) -> Self {
        if let Self::NotDelivering(diagnostics) = &mut self {
            diagnostics.startup_ms = Some(diagnostics::millis(elapsed));
        }
        self
    }
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

/// Whether a [`Backend`]'s wait for delivery ran to its end: confirmed, or a
/// loss recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Waited {
    Done,
    /// `interrupted` held; no loss was recorded and nothing is confirmed.
    Interrupted,
}

/// A native adapter. It reports raw events into a [`Sink`], which filters,
/// classifies, and bounds them. Every wait polls `interrupted`.
pub(crate) trait Backend: Send {
    /// Moves every event caused before this call into `sink`; when
    /// interrupted, whatever has arrived.
    fn drain(&mut self, sink: &mut Sink<'_>, interrupted: &dyn Fn() -> bool) -> Waited;
    /// The operation moved the tree to the quarantine. Path-based adapters
    /// map paths differently from here on; handle-based ones need nothing.
    fn moved_to_quarantine(&mut self, _interrupted: &dyn Fn() -> bool) -> Waited {
        Waited::Done
    }
    /// Stops watching the admin directory before the operation's own Git step
    /// deletes it. Its later disappearance is not coverage loss. When
    /// interrupted, the directory stays watched.
    fn release_admin(&mut self, interrupted: &dyn Fn() -> bool) -> Waited;
    /// Handles this adapter holds that a usage query of the checkout reports.
    fn handles(&self) -> Vec<(EvidenceKind, u64)>;
    #[allow(missing_docs)]
    fn backend(&self) -> ObservationBackend;
    /// The marker round trips confirmed so far; none for an adapter that
    /// confirms delivery without markers.
    fn round_trips(&self) -> RoundTrips {
        RoundTrips::default()
    }
}

/// The activity observer for one removal.
pub struct Observation {
    targets: Targets,
    options: ObserverOptions,
    backend: Box<dyn Backend>,
    moved: bool,
    lost: Option<CoverageLost>,
    /// The move's labeling was interrupted; the next drain says so.
    interrupted: bool,
    /// Set by the caller once it is past the move's checks; until then the
    /// stage follows `moved`.
    stage: Option<ObservationStage>,
    /// Entries the deletion engine processed since its previous drain.
    entries_since_drain: Option<u64>,
    diagnostics: DiagnosticsHandle,
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
    /// still be at the original path. `interrupted` is polled while the
    /// adapter confirms that its watch delivers.
    pub fn start(targets: Targets, options: ObserverOptions, interrupted: &dyn Fn() -> bool) -> Result<Self, ObservationError> {
        let started = Instant::now();
        let backend = native_backend(&targets, options, false, interrupted).map_err(|error| error.started_after(started.elapsed()))?;
        Ok(Self::with_backend(targets, options, backend).started_after(started.elapsed()))
    }

    /// Starts watching a tree that an earlier, interrupted attempt already
    /// moved to the quarantine, which must be a real directory.
    pub fn start_at_quarantine(
        targets: Targets,
        options: ObserverOptions,
        interrupted: &dyn Fn() -> bool,
    ) -> Result<Self, ObservationError> {
        let started = Instant::now();
        let backend = native_backend(&targets, options, true, interrupted).map_err(|error| error.started_after(started.elapsed()))?;
        let mut observation = Self::with_backend(targets, options, backend).started_after(started.elapsed());
        observation.moved = true;
        Ok(observation)
    }

    pub(crate) fn with_backend(targets: Targets, options: ObserverOptions, backend: Box<dyn Backend>) -> Self {
        let mut diagnostics = ObservationDiagnostics::new(backend.backend());
        diagnostics.set_round_trips(backend.round_trips());
        Self {
            targets,
            options,
            backend,
            moved: false,
            lost: None,
            interrupted: false,
            stage: None,
            entries_since_drain: None,
            diagnostics: DiagnosticsHandle::new(diagnostics),
        }
    }

    fn started_after(self, elapsed: std::time::Duration) -> Self {
        self.diagnostics.update(|diagnostics| diagnostics.startup_ms = Some(diagnostics::millis(elapsed)));
        self
    }

    /// What this session has measured so far.
    pub fn diagnostics(&self) -> ObservationDiagnostics {
        self.diagnostics.snapshot()
    }

    /// A view of [`diagnostics`](Self::diagnostics) that stays readable after
    /// the session is dropped.
    pub fn diagnostics_handle(&self) -> DiagnosticsHandle {
        self.diagnostics.clone()
    }

    /// Labels the losses later drains receive with `stage`, from deletion on.
    pub fn enter_stage(&mut self, stage: ObservationStage) {
        self.stage = Some(stage);
        self.entries_since_drain = None;
    }

    /// A [`drain`](Self::drain) by the deletion engine, which processed
    /// `entries` since its previous one.
    pub fn drain_after(&mut self, entries: u64, interrupted: &dyn Fn() -> bool) -> Drain {
        self.entries_since_drain = Some(entries);
        let drain = self.drain(interrupted);
        self.entries_since_drain = None;
        drain
    }

    fn current_stage(&self) -> ObservationStage {
        self.stage.unwrap_or(if self.moved { ObservationStage::PostMove } else { ObservationStage::PreMove })
    }

    /// Refreshes the diagnostics after a wait; `lost` is what the caller
    /// receives, so a first loss is recorded at the stage that received it.
    fn record(&self, lost: Option<&CoverageLost>) {
        let round_trips = self.backend.round_trips();
        let stage = self.current_stage();
        let entries_since_drain = self.entries_since_drain.filter(|_| stage == ObservationStage::Deletion);
        self.diagnostics.update(|diagnostics| {
            diagnostics.set_round_trips(round_trips);
            if diagnostics.loss.is_none()
                && let Some(reason) = lost
            {
                diagnostics.loss = Some(ObservationLoss { stage, reason: reason.clone(), entries_since_drain });
            }
        });
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

    /// Every event caused before this call, then the coverage state;
    /// [`Drain::interrupted`] when `interrupted` ended the wait first.
    pub fn drain(&mut self, interrupted: &dyn Fn() -> bool) -> Drain {
        let mut sink = Sink {
            targets: &self.targets,
            capacity: self.options.queue_capacity,
            events: Vec::new(),
            lost: self.lost.take(),
        };
        let waited = self.backend.drain(&mut sink, interrupted);
        self.lost = sink.lost;
        self.record(self.lost.as_ref());
        let interrupted = std::mem::take(&mut self.interrupted) || waited == Waited::Interrupted;
        Drain { events: sink.events, lost: self.lost.clone(), interrupted }
    }

    /// Reports the operation's own move: later events in the tree belong to
    /// the quarantine. Call it right after the rename succeeds. An
    /// interrupted wait here is reported by the next [`drain`](Self::drain).
    pub fn moved_to_quarantine(&mut self, interrupted: &dyn Fn() -> bool) {
        self.moved = true;
        if self.backend.moved_to_quarantine(interrupted) == Waited::Interrupted {
            self.interrupted = true;
        }
        self.record(None);
    }

    /// Stops watching the admin directory. Call it after the final index
    /// check and before the step that unregisters the worktree.
    ///
    /// ## Errors
    ///
    /// [`WaitInterrupted`] when the interrupt ended the wait for the admin
    /// directory's last changes; it is still watched, and the caller must
    /// not go on to unregister.
    pub fn release_admin(&mut self, interrupted: &dyn Fn() -> bool) -> Result<(), WaitInterrupted> {
        let waited = self.backend.release_admin(interrupted);
        self.record(None);
        match waited {
            Waited::Done => Ok(()),
            Waited::Interrupted => Err(WaitInterrupted),
        }
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
fn native_backend(
    targets: &Targets,
    _options: ObserverOptions,
    moved: bool,
    _interrupted: &dyn Fn() -> bool,
) -> Result<Box<dyn Backend>, ObservationError> {
    let tree = if moved { targets.quarantine() } else { targets.original() };
    Ok(Box::new(inotify::Inotify::start(targets, tree)?))
}

#[cfg(target_os = "macos")]
fn native_backend(
    targets: &Targets,
    options: ObserverOptions,
    moved: bool,
    interrupted: &dyn Fn() -> bool,
) -> Result<Box<dyn Backend>, ObservationError> {
    Ok(Box::new(fsevents::FsEvents::start(targets, options, moved, interrupted)?))
}

#[cfg(windows)]
fn native_backend(
    targets: &Targets,
    options: ObserverOptions,
    moved: bool,
    interrupted: &dyn Fn() -> bool,
) -> Result<Box<dyn Backend>, ObservationError> {
    Ok(Box::new(read_directory_changes::ReadDirectoryChanges::start(targets, options, moved, interrupted)?))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn native_backend(
    _targets: &Targets,
    _options: ObserverOptions,
    _moved: bool,
    _interrupted: &dyn Fn() -> bool,
) -> Result<Box<dyn Backend>, ObservationError> {
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

    /// Whether `raw` is in the session's scope (kept, or past the capacity
    /// and so lost): only such an event is progress for a wait.
    pub fn push(&self, raw: RawEvent) -> bool {
        let mut state = self.lock();
        let Some(event) = state.targets.classify(raw) else {
            return false;
        };
        if state.events.len() >= state.capacity {
            let capacity = state.capacity;
            state.lost.get_or_insert(CoverageLost::QueueFull { capacity });
        } else {
            state.events.push_back(event);
        }
        true
    }

    pub fn lose(&self, loss: CoverageLost) {
        self.lock().lost.get_or_insert(loss);
    }

    /// Whether a loss is recorded and not yet taken; a wait ends on it.
    pub fn has_loss(&self) -> bool {
        self.lock().lost.is_some()
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
