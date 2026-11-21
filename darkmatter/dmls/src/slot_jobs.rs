//! Schema-slot resolution off the request path: the slot worker, the jobs it
//! runs, and the snapshot store requests read.
//!
//! A document whose schema is a template (a `$schema` or matched trigger
//! payload holding `{{ … }}` slots) is resolved only here, on a worker thread
//! of its own. The trigger-registry worker is never used: requests wait on
//! that lane, so a slow `load_data` there would stall them.
//!
//! ```text
//! request path (overlay)               slot worker
//! ───────────────────────              ───────────
//! pins miss / entry stale ──demand──▶  queued job (seq n; job n-1 canceled)
//! serves the installed session  ◀──install (dispatch thread, seq checked)──
//!   read-only (pinned snapshots)       resolves with a fresh `SlotSession`
//! ```
//!
//! Each job resolves with a fresh [`SlotSession`] in full mode **without**
//! an approval handler: the configured `permissions.file-read` and the
//! in-memory (this server process) approvals decide every read, and
//! anything else is
//! `dm.schema.slot_approval_required`, never a prompt. The installed session
//! keeps the snapshots it pinned; requests serve them through
//! [`DarkmatterSchemas::with_slot_pins_only`](darkmatter::markdown::schemas::DarkmatterSchemas::with_slot_pins_only),
//! which evaluates and reads nothing a slot names.
//!
//! A change to anything a resolution read (a watched path, an open buffer, a
//! `stat` difference in the rescan fallback, a grant, a configuration change)
//! marks the document's entry stale. Requests keep serving the last-good
//! snapshot, marked stale, until the refresh it demands is installed.
//!
//! The same job also *observes* the document's eligible whole-value
//! properties ([`crate::overlay::properties`]): it evaluates them under the
//! same grants (never prompting, fetching, or writing) and judges the values
//! against the assembled schema. A document whose schema is not a template
//! gets a property-only job, which never resolves a slot; a templated one
//! whose snapshot is still served observes over that snapshot's pins. The
//! observation is one coherent [`PropertyView`]:
//!
//! - it installs only from the newest job, and only when no input it read
//!   changed since the job was dispatched (the store's input generation);
//!   an outdated result is dropped and its document refreshed again;
//! - it is withheld whenever the entry is stale or refreshing, and once the
//!   earliest cached remote response it used expires ([`SlotSnapshots::next_deadline`]
//!   is the dispatch loop's timer for that);
//! - a result already expired when it arrives installs no observation.

use std::collections::{BTreeSet, HashMap};
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::SystemTime;

use biscuit_file::FileResolutionContext;
use crossbeam_channel::{Receiver, Sender};
use darkmatter::markdown::compose::DataRuntime;
use darkmatter::markdown::compose::expression::data::ObservationKind;
use darkmatter::markdown::compose::expression::iteration::IterationLimits;
use simplified_schema::triggers::TriggerRegistry;
use darkmatter::markdown::schemas::slots::SlotSession;
use darkmatter::markdown::schemas::slots::grants::Grant;
use darkmatter::markdown::schemas::slots::observe::{ObserveRequest, PropertyObservations, PropertyOutcome, PropertySite};
use darkmatter::markdown::schemas::ObservedVerdict;
use std::time::Instant;
use lsp_types::Uri;
use simplified_schema::SchemaError;
use simplified_schema::resolve::buffers::SourceBuffers;

use crate::DmlsConfig;

/// The work each slot may do in the editor: a quarter of a command's visit
/// allowance and of its expansion bytes. The slowest realistic slot measured
/// (a 500-member list) uses well under 1 ms; this stops a runaway expression
/// in roughly 25 ms of work with the ordinary `dm.iteration.*` failure at the
/// slot, before a user notices the worker.
pub(crate) const DMLS_SLOT_LIMITS: IterationLimits = IterationLimits::new(
    NonZeroU64::new(25_000).unwrap(),
    NonZeroU64::new(4 * 1024 * 1024).unwrap(),
);

/// One document's resolution, with everything it reads captured on the
/// dispatch thread.
pub(crate) struct SlotJob {
    pub uri: Uri,
    pub path: PathBuf,
    pub seq: u64,
    /// The overlay's schema key (text, configuration, registry, context) the
    /// job was demanded for.
    pub base_key: u64,
    pub text: String,
    pub config: DmlsConfig,
    pub workspace_roots: Vec<PathBuf>,
    pub registry: Option<TriggerRegistry>,
    pub context: FileResolutionContext,
    /// Every open buffer, whatever its extension, which `load_data`, the
    /// Markdown readers, and schema resolution read in place of their files.
    pub buffers: SourceBuffers,
    /// The in-memory grants given in this server process.
    pub once_grants: Vec<Grant>,
    pub cancel: Arc<AtomicBool>,
    /// The document's schema is a template.
    pub templated: bool,
    /// The template's slots are resolved by this job; otherwise the job
    /// observes over `pinned` (or no template at all).
    pub slot_demand: bool,
    /// The installed session whose pins a property-only job assembles over.
    pub pinned: Option<Arc<SlotSession>>,
    /// The installed resolution's failure and reads, carried through a job
    /// that does not resolve slots.
    pub pinned_error: Option<Arc<SchemaError>>,
    pub pinned_reads: Vec<WatchedRead>,
    /// Every whole-value top-level property; empty when none is eligible.
    pub property_sites: Vec<PropertySite>,
    /// The registrations request-side assembly selects the document's host
    /// with.
    pub registered_hosts: Option<Arc<darkmatter::config::registration::RegisteredHosts>>,
    /// The store's input generation when the job was demanded.
    pub input_generation: u64,
    /// Wall and monotonic time sampled together at demand.
    pub dispatched: (SystemTime, Instant),
    /// The cache root remote reads are answered under (a test's temporary
    /// root); `None` is the document's base folder, as `md compose` uses.
    pub cache_root: Option<PathBuf>,
}

/// One observation pass's installed result: what completion, hover, and
/// diagnostics read about the document's eligible properties.
#[derive(Debug, Clone)]
pub struct PropertyView {
    /// What evaluating each property produced.
    pub observations: PropertyObservations,
    /// The eligible keys, in document order.
    pub eligible: Vec<String>,
    /// The destination schema's verdict on each eligible observed value;
    /// empty when no schema was assembled.
    pub verdicts: std::collections::BTreeMap<String, ObservedVerdict>,
    /// The shape of each observed value, for completion and hover.
    pub shapes: std::collections::BTreeMap<String, darkmatter::markdown::compose::expression::scope::Shape>,
    /// Each observed value with its direct-load source, for data positions.
    pub origins: std::collections::BTreeMap<String, darkmatter::markdown::ExpressionMetadata>,
    /// The shape of what each `load_data` call (as written) returned across
    /// the pass, for typing the call site and the callbacks it feeds; a call
    /// that returned more distinct values than are kept is left out, so it
    /// stays provisional.
    pub calls: Vec<(simplified_schema::expression::ast::Expr, darkmatter::markdown::compose::expression::scope::Shape)>,
}

impl PropertyView {
    /// The observed value of `key`, when it was observed.
    pub fn value(&self, key: &str) -> Option<&serde_json::Value> {
        match self.observations.get(key)? {
            PropertyOutcome::Observed { value, .. } => Some(value),
            _ => None,
        }
    }
}

/// A finished job, handed to the dispatch thread to install.
pub(crate) struct SlotResult {
    pub uri: Uri,
    pub path: PathBuf,
    pub seq: u64,
    pub base_key: u64,
    pub session: Arc<SlotSession>,
    /// The resolution's failure (a slot-resolution or other schema error),
    /// or `None` when the schema assembled.
    pub error: Option<Arc<SchemaError>>,
    pub reads: Vec<WatchedRead>,
    /// The reads of the slot resolution alone, carried by a later job that
    /// observes over its pins.
    pub slot_reads: Vec<WatchedRead>,
    pub canceled: bool,
    /// The property observation, when the document has eligible properties.
    pub properties: Option<Arc<PropertyView>>,
    /// The earliest expiry of a cached remote response the observation used.
    pub remote_expiry: Option<SystemTime>,
    pub input_generation: u64,
    pub dispatched: (SystemTime, Instant),
}

/// A path a resolution depended on, with what `stat` said after it ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WatchedRead {
    /// In [`SourceBuffers::key`] form.
    pub path: PathBuf,
    pub stat: Option<FileStat>,
}

/// The `stat` facts the rescan fallback compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileStat {
    len: u64,
    modified: Option<SystemTime>,
    dir: bool,
}

impl FileStat {
    fn of(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        Some(Self { len: metadata.len(), modified: metadata.modified().ok(), dir: metadata.is_dir() })
    }
}

impl WatchedRead {
    fn now(path: PathBuf) -> Self {
        let stat = FileStat::of(&path);
        Self { path, stat }
    }
}

/// What a request serves for one document.
#[derive(Clone, Default)]
pub struct SlotView {
    /// The last installed job's session, whose pinned snapshots requests
    /// read.
    pub session: Option<Arc<SlotSession>>,
    /// The last installed job's failure.
    pub error: Option<Arc<SchemaError>>,
    /// Changes whenever anything below does, for the overlay's cache key.
    pub generation: u64,
    /// A refresh is queued or running, or something the installed
    /// resolution read changed since: what it serves is stale.
    pub refreshing: bool,
    /// The base key of the installed job.
    pub installed_key: Option<u64>,
    /// The installed property observation, only while it is current: `None`
    /// when the entry is stale or refreshing, or a cached response it used
    /// has expired.
    pub properties: Option<Arc<PropertyView>>,
}

struct Installed {
    uri: Uri,
    session: Arc<SlotSession>,
    error: Option<Arc<SchemaError>>,
    reads: Vec<WatchedRead>,
    slot_reads: Vec<WatchedRead>,
    base_key: u64,
    properties: Option<Arc<PropertyView>>,
    /// When the observation's earliest cached remote response expires, on
    /// the monotonic clock; `None` once fired or when none was used.
    deadline: Option<Instant>,
}

/// How [`SlotSnapshots::install_checked`] disposed of a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallOutcome {
    Installed,
    /// Superseded or canceled; nothing changed.
    Rejected,
    /// An input it read changed after it was dispatched: the entry is stale
    /// and the document must refresh again.
    Outdated,
}

struct InFlight {
    seq: u64,
    base_key: u64,
    cancel: Arc<AtomicBool>,
    /// The store's input generation when it was demanded.
    input_generation: u64,
}

#[derive(Default)]
struct Entry {
    installed: Option<Installed>,
    in_flight: Option<InFlight>,
    stale: bool,
    generation: u64,
}

#[derive(Default)]
struct Inner {
    entries: HashMap<PathBuf, Entry>,
    next_seq: u64,
    once: Vec<Grant>,
    demands: Vec<SlotJob>,
    /// Bumped by every change to `once`, so a cached view never outlives a
    /// grant.
    epoch: u64,
    /// Bumped by every invalidation (a path, a buffer, a grant, the
    /// configuration, a refresh), with what it named in `changes`; a result
    /// dispatched before a change to something it read is outdated.
    input_generation: u64,
    /// The changes since the oldest job in flight: the generation each made
    /// and its paths (`None`: everything).
    changes: Vec<(u64, Option<Vec<PathBuf>>)>,
}

impl Inner {
    fn note_change(&mut self, paths: Option<Vec<PathBuf>>) {
        self.input_generation += 1;
        let oldest = self
            .entries
            .values()
            .filter_map(|entry| entry.in_flight.as_ref())
            .map(|in_flight| in_flight.input_generation)
            .min();
        match oldest {
            Some(oldest) => {
                self.changes.retain(|(generation, _)| *generation > oldest);
                self.changes.push((self.input_generation, paths));
            }
            None => self.changes.clear(),
        }
    }

    /// Whether anything `reads` names changed after `generation`.
    fn changed_since(&self, generation: u64, reads: &[WatchedRead]) -> bool {
        self.changes.iter().filter(|(made, _)| *made > generation).any(|(_, paths)| match paths {
            None => true,
            Some(paths) => reads.iter().any(|read| paths.contains(&read.path)),
        })
    }
}

/// Every open document's slot snapshot and the jobs demanded for them.
///
/// Shared by the overlay (which reads views and records demands on the
/// request path, never blocking on the worker) and the dispatch thread
/// (which drains demands to the worker and installs results).
pub struct SlotSnapshots {
    inner: Mutex<Inner>,
    clock: Mutex<Arc<dyn crate::clock::Clock>>,
    /// A test's cache root for remote reads (see [`SlotJob::cache_root`]).
    cache_root: Mutex<Option<PathBuf>>,
}

impl Default for SlotSnapshots {
    fn default() -> Self {
        Self {
            inner: Mutex::default(),
            clock: Mutex::new(Arc::new(crate::clock::SystemClock)),
            cache_root: Mutex::new(None),
        }
    }
}

/// The inputs [`SlotSnapshots::demand`] builds a job from, captured by the
/// overlay.
pub(crate) struct JobInputs {
    pub uri: Uri,
    pub text: String,
    pub config: DmlsConfig,
    pub workspace_roots: Vec<PathBuf>,
    pub registry: Option<TriggerRegistry>,
    pub context: FileResolutionContext,
    pub buffers: SourceBuffers,
    /// The document's schema is a template.
    pub templated: bool,
    /// See [`SlotJob::property_sites`].
    pub property_sites: Vec<PropertySite>,
    pub registered_hosts: Option<Arc<darkmatter::config::registration::RegisteredHosts>>,
}

impl SlotSnapshots {
    /// Reads time from `clock` (wall and monotonic, paired) from now on.
    pub fn set_clock(&self, clock: Arc<dyn crate::clock::Clock>) {
        *self.clock.lock().unwrap_or_else(PoisonError::into_inner) = clock;
    }

    /// Answers every job's remote reads under `root` (a test's temporary
    /// cache) instead of each document's base folder.
    pub fn set_cache_root(&self, root: Option<PathBuf>) {
        *self.cache_root.lock().unwrap_or_else(PoisonError::into_inner) = root;
    }

    pub(crate) fn clock(&self) -> Arc<dyn crate::clock::Clock> {
        Arc::clone(&self.clock.lock().unwrap_or_else(PoisonError::into_inner))
    }

    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// What requests for the document at `path` serve. A lookup only.
    pub fn view(&self, path: &Path) -> SlotView {
        let inner = self.inner();
        let Some(entry) = inner.entries.get(path) else {
            return SlotView { generation: inner.epoch, ..SlotView::default() };
        };
        let refreshing = entry.stale || entry.in_flight.is_some();
        let now = self.clock().now();
        let properties = entry
            .installed
            .as_ref()
            .filter(|installed| !refreshing && installed.deadline.is_none_or(|deadline| deadline > now))
            .and_then(|installed| installed.properties.clone());
        SlotView {
            session: entry.installed.as_ref().map(|installed| Arc::clone(&installed.session)),
            error: entry.installed.as_ref().and_then(|installed| installed.error.clone()),
            generation: entry.generation ^ (inner.epoch << 32),
            refreshing,
            installed_key: entry.installed.as_ref().map(|installed| installed.base_key),
            properties,
        }
    }

    /// Records that the document at `path` needs a resolution for
    /// `base_key`: its pinned snapshot missed (`pins_missed`), or what the
    /// installed one read changed. A job already demanded for the same key
    /// is not repeated; one for another key is canceled and replaced.
    ///
    /// ## Returns
    ///
    /// Whether a job was demanded.
    #[cfg(test)]
    pub(crate) fn demand(&self, path: &Path, base_key: u64, pins_missed: bool, inputs: impl FnOnce() -> JobInputs) -> bool {
        self.demand_with(path, base_key, pins_missed, false, inputs)
    }

    /// [`Self::demand`] for a document that may also have eligible
    /// properties to observe (`observes`): a job is demanded for a key no
    /// installed job ran for even when the pins serve, and it resolves the
    /// template's slots only when they missed or went stale.
    pub(crate) fn demand_with(
        &self,
        path: &Path,
        base_key: u64,
        pins_missed: bool,
        observes: bool,
        inputs: impl FnOnce() -> JobInputs,
    ) -> bool {
        let mut inner = self.inner();
        let seq = inner.next_seq + 1;
        let once = inner.once.clone();
        let entry = inner.entries.entry(path.to_path_buf()).or_default();
        let resolved_for_key = entry.installed.as_ref().is_some_and(|installed| installed.base_key == base_key);
        if !(entry.stale || ((pins_missed || observes) && !resolved_for_key)) {
            return false;
        }
        if let Some(in_flight) = &entry.in_flight {
            if in_flight.base_key == base_key {
                return false;
            }
            in_flight.cancel.store(true, Ordering::Release);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let input_generation = inner.input_generation;
        let entry = inner.entries.get_mut(path).expect("the entry was just created");
        let stale = entry.stale;
        entry.in_flight = Some(InFlight { seq, base_key, cancel: Arc::clone(&cancel), input_generation });
        entry.stale = false;
        entry.generation += 1;
        let installed = entry.installed.as_ref();
        let (pinned, pinned_error, pinned_reads) = match installed {
            Some(installed) => (Some(Arc::clone(&installed.session)), installed.error.clone(), installed.slot_reads.clone()),
            None => (None, None, Vec::new()),
        };
        inner.next_seq = seq;
        let inputs = inputs();
        // A template's slots are resolved again when no snapshot serves it,
        // or something its resolution read changed; otherwise the job only
        // observes over the snapshot it pinned.
        let slot_demand = inputs.templated && (pins_missed || stale || pinned.is_none());
        let dispatched = self.clock().pair();
        let cache_root = self.cache_root.lock().unwrap_or_else(PoisonError::into_inner).clone();
        inner.demands.push(SlotJob {
            uri: inputs.uri,
            path: path.to_path_buf(),
            seq,
            base_key,
            text: inputs.text,
            config: inputs.config,
            workspace_roots: inputs.workspace_roots,
            registry: inputs.registry,
            context: inputs.context,
            buffers: inputs.buffers,
            once_grants: once,
            cancel,
            templated: inputs.templated,
            slot_demand,
            pinned,
            pinned_error,
            pinned_reads,
            property_sites: inputs.property_sites,
            registered_hosts: inputs.registered_hosts,
            input_generation,
            dispatched,
            cache_root,
        });
        true
    }

    /// The jobs demanded since the last call, in order.
    pub(crate) fn take_demands(&self) -> Vec<SlotJob> {
        std::mem::take(&mut self.inner().demands)
    }

    /// Installs a finished job unless a newer one was demanded for its
    /// document since, or it was canceled.
    ///
    /// ## Returns
    ///
    /// Whether it was installed.
    #[cfg(test)]
    pub(crate) fn install(&self, result: SlotResult) -> bool {
        self.install_checked(result) == InstallOutcome::Installed
    }

    /// [`Self::install`], saying why a result was not installed. A result
    /// whose inputs changed after its dispatch is [`InstallOutcome::Outdated`]:
    /// the entry goes stale, so the document's next request demands it
    /// again. A result whose cached remote response has already expired
    /// installs without its observation. After installing, every path the
    /// result read is `stat`ed again: a difference from what the job saw
    /// (a change no watch could have reported yet) marks the entry stale.
    pub(crate) fn install_checked(&self, result: SlotResult) -> InstallOutcome {
        let wall = self.clock().wall();
        let mut inner = self.inner();
        let outdated = inner.changed_since(result.input_generation, &result.reads);
        let Some(entry) = inner.entries.get_mut(&result.path) else {
            return InstallOutcome::Rejected;
        };
        if result.canceled || entry.in_flight.as_ref().is_none_or(|in_flight| in_flight.seq != result.seq) {
            return InstallOutcome::Rejected;
        }
        entry.in_flight = None;
        if outdated {
            entry.stale = true;
            entry.generation += 1;
            return InstallOutcome::Outdated;
        }
        let expired = result.remote_expiry.is_some_and(|expiry| expiry <= wall);
        let deadline = result.remote_expiry.filter(|_| !expired).map(|expiry| {
            let (dispatched_wall, dispatched_mono) = result.dispatched;
            dispatched_mono + expiry.duration_since(dispatched_wall).unwrap_or_default()
        });
        let changed = result.reads.iter().any(|read| FileStat::of(&read.path) != read.stat);
        entry.installed = Some(Installed {
            uri: result.uri,
            session: result.session,
            error: result.error,
            reads: result.reads,
            slot_reads: result.slot_reads,
            base_key: result.base_key,
            properties: if expired { None } else { result.properties },
            deadline,
        });
        if changed {
            entry.stale = true;
        }
        entry.generation += 1;
        InstallOutcome::Installed
    }

    /// The earliest monotonic instant an installed observation's cached
    /// remote response expires, for the dispatch loop's timer.
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.inner()
            .entries
            .values()
            .filter_map(|entry| entry.installed.as_ref().and_then(|installed| installed.deadline))
            .min()
    }

    /// Marks stale every entry whose observation's deadline has passed and
    /// returns their documents, for the dispatch loop to refresh.
    pub(crate) fn expire_due(&self) -> Vec<Uri> {
        let now = self.clock().now();
        let mut due = Vec::new();
        for entry in self.inner().entries.values_mut() {
            let Some(installed) = entry.installed.as_mut() else {
                continue;
            };
            if installed.deadline.is_some_and(|deadline| deadline <= now) {
                installed.deadline = None;
                installed.properties = None;
                entry.stale = true;
                entry.generation += 1;
                due.push(installed.uri.clone());
            }
        }
        due
    }

    /// Marks stale every entry that read one of `paths`.
    ///
    /// ## Returns
    ///
    /// Whether any entry was marked.
    pub fn invalidate_paths(&self, paths: &[PathBuf]) -> bool {
        if paths.is_empty() {
            return false;
        }
        let keys: Vec<PathBuf> = paths.iter().map(|path| SourceBuffers::key(path)).collect();
        let mut marked = false;
        let mut inner = self.inner();
        inner.note_change(Some(keys.clone()));
        for entry in inner.entries.values_mut() {
            let reads = entry.installed.as_ref().map(|installed| installed.reads.as_slice()).unwrap_or_default();
            if reads.iter().any(|read| keys.contains(&read.path)) {
                entry.stale = true;
                entry.generation += 1;
                marked = true;
            }
        }
        marked
    }

    /// Marks every entry stale (a refresh command, a grant, a configuration
    /// change).
    pub fn invalidate_all(&self) {
        let mut inner = self.inner();
        inner.note_change(None);
        for entry in inner.entries.values_mut() {
            entry.stale = true;
            entry.generation += 1;
        }
    }

    /// The rescan fallback: re-`stat`s every path an installed resolution
    /// read and marks stale each entry with a difference. File contents are
    /// not compared, so a rewrite that keeps a file's size and timestamp goes
    /// unnoticed until something else refreshes it.
    ///
    /// ## Returns
    ///
    /// Whether any entry was marked.
    pub fn recheck_stats(&self) -> bool {
        let mut marked = false;
        for entry in self.inner().entries.values_mut() {
            let changed = entry
                .installed
                .as_ref()
                .is_some_and(|installed| installed.reads.iter().any(|read| FileStat::of(&read.path) != read.stat));
            if changed {
                entry.stale = true;
                entry.generation += 1;
                marked = true;
            }
        }
        marked
    }

    /// Every path the installed resolutions read, for the slot watches.
    pub fn watched_paths(&self) -> BTreeSet<PathBuf> {
        self.inner()
            .entries
            .values()
            .filter_map(|entry| entry.installed.as_ref())
            .flat_map(|installed| installed.reads.iter().map(|read| read.path.clone()))
            .collect()
    }

    /// Adds an in-memory grant for the rest of this server process and
    /// marks every entry stale.
    pub fn add_once(&self, grant: Grant) {
        {
            let mut inner = self.inner();
            if !inner.once.contains(&grant) {
                inner.once.push(grant);
            }
            inner.epoch += 1;
        }
        self.invalidate_all();
    }

    /// The in-memory grants given so far.
    pub fn once_grants(&self) -> Vec<Grant> {
        self.inner().once.clone()
    }

    /// Drops a closed document's entry, canceling its job.
    pub fn forget(&self, path: &Path) {
        if let Some(entry) = self.inner().entries.remove(path)
            && let Some(in_flight) = entry.in_flight
        {
            in_flight.cancel.store(true, Ordering::Release);
        }
    }
}

/// Test seams over the slot worker.
#[derive(Debug, Default)]
pub struct SlotWorkerHooks {
    /// When set, the worker waits before resolving each job (after
    /// reporting [`crate::StartupEvent::SlotPreparing`]) until this receives
    /// a message or disconnects.
    pub gate: Option<Receiver<()>>,
}

/// Starts the slot worker. Each finished job is sent to `done`; the worker
/// exits when the returned sender (and every clone) is dropped.
pub(crate) fn spawn_slot_worker(
    done: impl Fn(SlotResult) + Send + 'static,
    before_job: impl Fn() + Send + 'static,
    hooks: SlotWorkerHooks,
    dispatch: tracing::Dispatch,
) -> Sender<SlotJob> {
    let (jobs_tx, jobs_rx) = crossbeam_channel::unbounded::<SlotJob>();
    std::thread::Builder::new()
        .name("dmls-slots".to_string())
        .spawn(move || {
            tracing::dispatcher::with_default(&dispatch, || {
                for job in jobs_rx {
                    if job.cancel.load(Ordering::Acquire) {
                        continue;
                    }
                    before_job();
                    if let Some(gate) = &hooks.gate {
                        let _ = gate.recv();
                    }
                    done(run_job(job));
                }
            });
        })
        .expect("spawn the slot worker");
    jobs_tx
}

/// Runs one job: resolves the template's slots with a fresh session when
/// the job demands it (else assembles over the installed pins, or with no
/// template), then observes the eligible properties and judges their values
/// against the assembled schema.
pub(crate) fn run_job(job: SlotJob) -> SlotResult {
    let started = std::time::Instant::now();
    let data = DataRuntime::new();
    let session = Arc::new(
        SlotSession::for_context(data.clone(), &job.context)
            .with_run_grants(job.once_grants.iter().cloned())
            .with_cancel(Arc::clone(&job.cancel))
            .with_slot_limits(DMLS_SLOT_LIMITS),
    );
    // A job that does not resolve slots must never read a slot's inputs:
    // it serves the installed session's pins, which evaluate nothing.
    let slot_use = match (&job.pinned, job.templated, job.slot_demand) {
        (_, true, true) => crate::overlay::schema::SlotUse::Resolve(&session),
        (Some(pinned), true, false) => crate::overlay::schema::SlotUse::Pins(pinned),
        _ => crate::overlay::schema::SlotUse::None,
    };
    let resolves = matches!(slot_use, crate::overlay::schema::SlotUse::Resolve(_));
    let assembled = simplified_schema::resolve::buffers::with(&job.buffers, || {
        crate::overlay::resolve_slot_templates(
            &job.path,
            &job.text,
            &job.config,
            &job.workspace_roots,
            job.registry.clone(),
            job.registered_hosts.as_ref(),
            &job.context,
            slot_use,
        )
    });
    let (bundle, error) = match assembled {
        Ok(bundle) => (bundle, None),
        Err(error) => (None, Some(Arc::new(error))),
    };
    let (session_out, error, slot_reads) = if resolves {
        (Arc::clone(&session), error, slot_reads(&data, job.context.repository_root()))
    } else {
        let served = job.pinned.clone().filter(|_| job.templated).unwrap_or_else(|| Arc::clone(&session));
        let error = if job.templated { job.pinned_error.clone() } else { error };
        (served, error, job.pinned_reads.clone())
    };
    let properties = (!job.property_sites.is_empty() && !job.cancel.load(Ordering::Acquire)).then(|| {
        observe(&job, &session, bundle.as_ref())
    });
    let canceled = job.cancel.load(Ordering::Acquire) || properties.as_ref().is_some_and(|view| view.observations.canceled);
    let mut reads = slot_reads.clone();
    if !resolves || properties.is_some() {
        for read in self::slot_reads(&data, job.context.repository_root()) {
            if !reads.contains(&read) {
                reads.push(read);
            }
        }
    }
    let remote_expiry = properties.as_ref().and_then(|view| view.observations.remote_expiry);
    tracing::debug!(
        path = %job.path.display(),
        seq = job.seq,
        reads = reads.len(),
        failed = error.is_some(),
        properties = properties.as_ref().map_or(0, |view| view.observations.properties.len()),
        // The job's one data runtime: how often its inputs were read and
        // parsed, whichever properties shared them.
        data_reads = data.stats().reads,
        parses = data.stats().parses,
        exhausted = properties.as_ref().is_some_and(|view| view.observations.exhausted),
        canceled,
        elapsed_ms = started.elapsed().as_millis(),
        "schema slots resolved"
    );
    SlotResult {
        uri: job.uri,
        path: job.path,
        seq: job.seq,
        base_key: job.base_key,
        session: session_out,
        error,
        reads,
        slot_reads,
        canceled,
        properties: properties.map(Arc::new),
        remote_expiry,
        input_generation: job.input_generation,
        dispatched: job.dispatched,
    }
}

/// Observes `job`'s properties with `session` and judges the eligible
/// observed values against `bundle`'s effective schema (no verdict without
/// one: the observations still type completion and hover).
fn observe(job: &SlotJob, session: &Arc<SlotSession>, bundle: Option<&crate::overlay::schema::SchemaBundle>) -> PropertyView {
    let markdown: darkmatter::markdown::Markdown = job.text.as_str().into();
    let seed = match darkmatter::markdown::schemas::frontmatter_as_json(&markdown) {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    let request = ObserveRequest {
        document: job.path.clone(),
        context: job.context.clone(),
        seed: seed.clone(),
        sites: job.property_sites.clone(),
        cache_root: job.cache_root.clone(),
        now: job.dispatched.0,
    };
    let observations =
        simplified_schema::resolve::buffers::with(&job.buffers, || session.observe_properties(&request));
    let eligible: Vec<String> =
        job.property_sites.iter().filter(|site| site.eligible).map(|site| site.key.clone()).collect();
    let mut replacements = std::collections::BTreeMap::new();
    let mut shapes = std::collections::BTreeMap::new();
    let mut origins = std::collections::BTreeMap::new();
    for property in &observations.properties {
        // An executed uncontracted headerless read stays `unknown[][]` in
        // the editor: no observed shape types a value that flows through it
        // and no verdict judges its cells (composition judges every realized
        // cell). A read in a branch that did not run never counts.
        if property.headerless_uncontracted {
            continue;
        }
        if let PropertyOutcome::Observed { value, load } = &property.outcome {
            replacements.insert(property.key.clone(), value.clone());
            shapes.insert(property.key.clone(), darkmatter::markdown::compose::expression::scope::Shape::observed(value));
            origins.insert(
                property.key.clone(),
                darkmatter::markdown::ExpressionMetadata { span: None, load: load.clone(), value: value.clone() },
            );
        }
    }
    let verdicts = match bundle {
        Some(bundle) if !replacements.is_empty() => {
            let mut verdicts = bundle
                .effective
                .validate_observed_frontmatter(&serde_json::Value::Object(seed), &replacements)
                .properties;
            verdicts.retain(|key, _| eligible.contains(key));
            verdicts
        }
        _ => std::collections::BTreeMap::new(),
    };
    let calls = observed_call_shapes(&observations);
    PropertyView { observations, eligible, verdicts, shapes, origins, calls }
}

/// The shape each `load_data` call (as written) returned across every
/// observed property: the union of its distinct values' shapes, with
/// `unknown[][]` for any execution that was a headerless uncontracted read
/// (whose cells are never typed from what it loaded), or nothing when,
/// together, more than eight distinct values were returned.
fn observed_call_shapes(
    observations: &darkmatter::markdown::schemas::slots::observe::PropertyObservations,
) -> Vec<(simplified_schema::expression::ast::Expr, darkmatter::markdown::compose::expression::scope::Shape)> {
    use darkmatter::markdown::compose::expression::scope::Shape;
    const ALTERNATIVES: usize = 8;
    struct Group {
        call: simplified_schema::expression::ast::Expr,
        values: Vec<serde_json::Value>,
        overflow: bool,
        headerless: bool,
    }
    let mut grouped: Vec<Group> = Vec::new();
    for call in observations.properties.iter().flat_map(|property| &property.calls) {
        let index = match grouped.iter().position(|group| group.call == call.call) {
            Some(index) => index,
            None => {
                grouped.push(Group { call: call.call.clone(), values: Vec::new(), overflow: false, headerless: false });
                grouped.len() - 1
            }
        };
        let group = &mut grouped[index];
        group.overflow |= call.overflow;
        group.headerless |= call.headerless_uncontracted;
        for value in &call.values {
            if !group.values.contains(value) {
                group.values.push(value.clone());
            }
        }
        group.overflow |= group.values.len() > ALTERNATIVES;
    }
    let unknown_rows = Shape::array_of(&Shape::array_of(&Shape::Unknown { non_null: false }));
    grouped
        .into_iter()
        .filter(|group| !group.overflow)
        .filter_map(|group| {
            let headerless = group.headerless.then(|| unknown_rows.clone());
            let shape = group.values.iter().map(Shape::observed).chain(headerless).reduce(|left, right| left.union(&right))?;
            Some((group.call, shape))
        })
        .collect()
}

/// Every local path the resolution observed (content reads, existence
/// answers, missing probes, listings), plus, for a document in a
/// repository, that repository's index and `HEAD`: its `permissions` decided
/// every read and count only as committed at `HEAD` (the commit rule), so a
/// commit can admit a read the resolution was refused and never observed.
fn slot_reads(data: &DataRuntime, repository: Option<&Path>) -> Vec<WatchedRead> {
    let mut paths: BTreeSet<PathBuf> = BTreeSet::new();
    for observation in data.observations_since(Default::default()) {
        let local = matches!(
            observation.kind,
            ObservationKind::File | ObservationKind::Exists | ObservationKind::MissingProbe | ObservationKind::Listing
        );
        if local && !observation.entry.contains("://") {
            paths.insert(SourceBuffers::key(Path::new(&observation.entry)));
        }
    }
    if let Some(root) = repository
        && let Some(git_dir) = git_dir(root)
    {
        paths.insert(SourceBuffers::key(&git_dir.join("index")));
        paths.insert(SourceBuffers::key(&git_dir.join("HEAD")));
    }
    paths.into_iter().map(WatchedRead::now).collect()
}

/// The repository at `root`'s `HEAD` and the ref it names (a branch's ref
/// file, which a commit rewrites while `HEAD` stays the same), in the
/// common Git directory for a linked worktree. Empty outside a repository.
pub(crate) fn head_files(root: &Path) -> Vec<PathBuf> {
    let Some(git_dir) = git_dir(root) else {
        return Vec::new();
    };
    let head = git_dir.join("HEAD");
    let mut files = vec![SourceBuffers::key(&head)];
    let named = std::fs::read_to_string(&head).ok().and_then(|text| {
        text.lines().next().and_then(|line| line.strip_prefix("ref:")).map(|name| name.trim().to_string())
    });
    if let Some(name) = named.filter(|name| !name.is_empty()) {
        let common = std::fs::read_to_string(git_dir.join("commondir"))
            .ok()
            .map(|text| {
                let named = Path::new(text.trim());
                if named.is_absolute() { named.to_path_buf() } else { git_dir.join(named) }
            })
            .unwrap_or_else(|| git_dir.clone());
        files.push(SourceBuffers::key(&common.join(name)));
    }
    files
}

/// The Git directory of the repository at `root`: `.git` itself, or the
/// folder a linked worktree's `.git` file names.
fn git_dir(root: &Path) -> Option<PathBuf> {
    let dot_git = root.join(".git");
    let metadata = std::fs::metadata(&dot_git).ok()?;
    if metadata.is_dir() {
        return Some(dot_git);
    }
    let text = std::fs::read_to_string(&dot_git).ok()?;
    let named = text.lines().find_map(|line| line.strip_prefix("gitdir:"))?.trim();
    let named = Path::new(named);
    Some(if named.is_absolute() { named.to_path_buf() } else { root.join(named) })
}

/// The [`UnresolvedSlots`](simplified_schema::UnresolvedSlots) a schema
/// error reports (through a referenced schema or an aggregate): a template
/// no snapshot resolved.
pub(crate) fn find_unresolved(error: &SchemaError) -> Option<&simplified_schema::UnresolvedSlots> {
    match error {
        SchemaError::UnresolvedSlots(unresolved) => Some(unresolved),
        SchemaError::ReferencedSchema { source, .. } => find_unresolved(source),
        SchemaError::Aggregate { errors } => errors.iter().find_map(find_unresolved),
        _ => None,
    }
}

/// Whether `error` is a template's failure: a slot resolution that failed,
/// or a template left unresolved.
pub(crate) fn is_slot_failure(error: &SchemaError) -> bool {
    slot_error(error).is_some()
}

/// The template failure inside `error`, if any (see [`is_slot_failure`]).
pub(crate) fn slot_error(error: &SchemaError) -> Option<&SchemaError> {
    match error {
        SchemaError::SlotResolution(_) | SchemaError::UnresolvedSlots(_) => Some(error),
        SchemaError::ReferencedSchema { source, .. } => slot_error(source),
        SchemaError::Aggregate { errors } => errors.iter().find_map(slot_error),
        _ => None,
    }
}

/// The literal shape of an unresolved template whose root is one object
/// shape: what completion offers until a snapshot resolves it.
pub(crate) fn template_shape(
    unresolved: &simplified_schema::UnresolvedSlots,
) -> Option<Arc<simplified_schema::simplified::SchemaShape>> {
    match unresolved.pending.as_ref()?.template()? {
        (simplified_schema::simplified::SchemaRoot::Single(shape), _) => Some(Arc::new(shape.clone())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The context of a document in `dir`, whose home (`dir/.home`) holds a
    /// configuration pre-approving no read, so only once approvals allow one.
    fn context(dir: &Path) -> FileResolutionContext {
        let home = dir.join(".home");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(".darkmatter.yaml"), "permissions:\n    file-read: []\n").unwrap();
        FileResolutionContext::from_snapshot(dir, Some(home), std::collections::HashMap::new())
    }

    fn inputs(dir: &Path) -> JobInputs {
        JobInputs {
            uri: "file:///doc.md".parse().unwrap(),
            text: String::new(),
            config: DmlsConfig::default(),
            workspace_roots: Vec::new(),
            registry: None,
            context: context(dir),
            buffers: SourceBuffers::default(),
            templated: true,
            property_sites: Vec::new(),
            registered_hosts: None,
        }
    }

    const OBSERVED_DOC: &str = "---\n$schema:\n  direct: \"{ name: string(required), age: number }[]\"\ndirect: \"{{ load_data('./data.yaml') }}\"\n---\n";

    /// The inputs of [`OBSERVED_DOC`] at `dir/doc.md`: no template, one
    /// eligible property.
    fn observed_inputs(dir: &Path, text: &str) -> JobInputs {
        JobInputs {
            text: text.to_string(),
            templated: false,
            property_sites: crate::overlay::properties::sites_to_observe(text),
            ..inputs(dir)
        }
    }

    /// A store whose process approved reading `dir/data.yaml`.
    fn observing_store(dir: &Path) -> SlotSnapshots {
        let store = SlotSnapshots::default();
        store.add_once(Grant {
            decision: darkmatter::markdown::schemas::slots::grants::GrantDecision::Allow,
            target: darkmatter::markdown::schemas::slots::grants::GrantTarget::File(dir.join("data.yaml")),
            scope: darkmatter::markdown::schemas::slots::grants::GrantScope::Once,
        });
        store
    }

    fn canonical_dir() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let dir = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
        (temp, dir)
    }

    #[test]
    fn a_newer_demand_cancels_the_job_in_flight_and_only_the_newest_installs() {
        let dir = tempfile::tempdir().unwrap();
        let store = SlotSnapshots::default();
        let doc = dir.path().join("doc.md");
        assert!(store.demand(&doc, 1, true, || inputs(dir.path())));
        assert!(!store.demand(&doc, 1, true, || inputs(dir.path())), "the same key is demanded once");
        assert!(store.demand(&doc, 2, true, || inputs(dir.path())));
        let jobs = store.take_demands();
        assert_eq!(jobs.len(), 2);
        assert!(jobs[0].cancel.load(Ordering::Acquire), "the superseded job is canceled");
        assert!(!jobs[1].cancel.load(Ordering::Acquire));
        let mut jobs = jobs.into_iter();
        let older = run_job(jobs.next().unwrap());
        let newer = run_job(jobs.next().unwrap());
        assert!(!store.install(older), "an older seq is rejected");
        assert!(store.view(&doc).refreshing);
        assert!(store.install(newer));
        let view = store.view(&doc);
        assert!(!view.refreshing);
        assert_eq!(view.installed_key, Some(2));
    }

    #[test]
    fn a_key_already_resolved_is_not_demanded_again_unless_stale() {
        let dir = tempfile::tempdir().unwrap();
        let store = SlotSnapshots::default();
        let doc = dir.path().join("doc.md");
        store.demand(&doc, 7, true, || inputs(dir.path()));
        let job = store.take_demands().pop().unwrap();
        assert!(store.install(run_job(job)));
        assert!(!store.demand(&doc, 7, true, || inputs(dir.path())));
        store.invalidate_all();
        assert!(store.view(&doc).refreshing, "a stale entry serves its snapshot marked stale");
        assert!(store.demand(&doc, 7, false, || inputs(dir.path())));
    }

    #[test]
    fn the_rescan_fallback_marks_an_entry_stale_when_a_read_file_changes_size() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("values.yaml");
        std::fs::write(&data, "- a\n").unwrap();
        let store = SlotSnapshots::default();
        let doc = dir.path().join("doc.md");
        store.demand(&doc, 1, true, || inputs(dir.path()));
        let job = store.take_demands().pop().unwrap();
        let mut result = run_job(job);
        result.reads = vec![WatchedRead::now(SourceBuffers::key(&data))];
        assert!(store.install(result));
        assert!(!store.recheck_stats(), "nothing changed");
        std::fs::write(&data, "- a\n- b\n").unwrap();
        assert!(store.recheck_stats());
        assert!(store.view(&doc).refreshing);
    }

    #[test]
    fn a_missing_file_that_appears_marks_the_entry_stale() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("later.yaml");
        let store = SlotSnapshots::default();
        let doc = dir.path().join("doc.md");
        store.demand(&doc, 1, true, || inputs(dir.path()));
        let mut result = run_job(store.take_demands().pop().unwrap());
        result.reads = vec![WatchedRead::now(SourceBuffers::key(&data))];
        assert_eq!(result.reads[0].stat, None);
        store.install(result);
        std::fs::write(&data, "- a\n").unwrap();
        assert!(store.invalidate_paths(std::slice::from_ref(&data)));
        assert!(store.view(&doc).refreshing);
    }

    /// A job for `text` at `dir/doc.md`, reading with once approvals for
    /// the folder's `os.yaml` and `types.yaml`.
    fn granted_job(dir: &Path, text: &str) -> SlotJob {
        let store = SlotSnapshots::default();
        let doc = dir.join("doc.md");
        for name in ["os.yaml", "types.yaml"] {
            store.add_once(Grant {
                decision: darkmatter::markdown::schemas::slots::grants::GrantDecision::Allow,
                target: darkmatter::markdown::schemas::slots::grants::GrantTarget::File(dir.join(name)),
                scope: darkmatter::markdown::schemas::slots::grants::GrantScope::Once,
            });
        }
        store.demand(&doc, 1, true, || JobInputs { text: text.to_string(), ..inputs(dir) });
        store.take_demands().pop().unwrap()
    }

    fn failure_codes(result: &SlotResult) -> Vec<String> {
        let Some(error) = &result.error else {
            return Vec::new();
        };
        darkmatter::markdown::compose::report_diagnostics::template_slot_records(slot_error(error).unwrap_or_else(|| panic!("not a slot failure: {error:?}")))
            .unwrap_or_default()
            .into_iter()
            .map(|(_, record)| record.code)
            .collect()
    }

    /// The editor's work bound: a 500-member list (the largest realistic
    /// list measured) resolves; a list the default command limits allow but
    /// the editor's do not fails with the ordinary work-limit code at the
    /// slot, and nothing is installed as a resolved schema.
    #[test]
    fn the_editor_work_bound_resolves_a_realistic_list_and_stops_a_runaway_one() {
        let dir = tempfile::tempdir().unwrap();
        let dir = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let text = "---\n$schema:\n  os: enum({{ load_data(\"./os.yaml\", \"string[]\") }})\nos: m0\n---\n";
        let list = |count: usize| (0..count).map(|index| format!("- m{index}\n")).collect::<String>();

        std::fs::write(dir.join("os.yaml"), list(500)).unwrap();
        let realistic = run_job(granted_job(&dir, text));
        assert!(realistic.error.is_none(), "{:?}", realistic.error);
        assert!(realistic.reads.iter().any(|read| read.path == SourceBuffers::key(&dir.join("os.yaml"))));

        // Each `map` item is one visit: 200 + 200² is within a command's
        // allowance and past the editor's.
        let runaway = "---\n$schema:\n  os: enum({{ map(load_data(\"./os.yaml\", \"string[]\"), x => first(map(load_data(\"./os.yaml\", \"string[]\"), y => x))) }})\nos: m0\n---\n";
        std::fs::write(dir.join("os.yaml"), list(200)).unwrap();
        let job = granted_job(&dir, runaway);
        let (context, grants) = (job.context.clone(), job.once_grants.clone());
        let stopped = run_job(job);
        let codes = failure_codes(&stopped);
        assert!(codes.iter().any(|code| code.starts_with("dm.iteration.")), "{codes:?} / {:?}", stopped.error);
        // Control, after the editor's failure: a command's default allowance
        // resolves the same slot, so the bound decided the failure and the
        // failure was not cached for the command.
        let command = Arc::new(SlotSession::for_context(DataRuntime::new(), &context).with_run_grants(grants));
        crate::overlay::resolve_slot_templates(&dir.join("doc.md"), runaway, &DmlsConfig::default(), &[], None, None, &context, crate::overlay::schema::SlotUse::Resolve(&command))
            .expect("within the command allowance");
    }

    /// A template type's returned import is one of the job's watched reads,
    /// read under the same grants as the slot's own reads.
    #[test]
    fn a_template_type_job_watches_the_import_its_returned_definition_names() {
        let dir = tempfile::tempdir().unwrap();
        let dir = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        std::fs::write(dir.join("types.yaml"), "kind: schema\ntypes:\n  Person:\n    role: enum(admin)\n").unwrap();
        let text = "---\n$schema:\n  owner: \"{{ 'Person@./types.yaml' }}\"\nowner:\n  role: admin\n---\n";
        let result = run_job(granted_job(&dir, text));
        assert!(result.error.is_none(), "{:?}", result.error);
        assert!(result.reads.iter().any(|read| read.path == SourceBuffers::key(&dir.join("types.yaml"))), "{:?}", result.reads);
    }

    /// A returned import that does not exist fails the job, and its missing
    /// candidate is still watched, so creating it re-resolves.
    #[test]
    fn a_template_type_job_with_a_missing_import_fails_and_watches_the_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let dir = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        let text = "---\n$schema:\n  owner: \"{{ 'Person@./types.yaml' }}\"\nowner:\n  role: admin\n---\n";
        let result = run_job(granted_job(&dir, text));
        assert!(result.error.as_deref().is_some_and(is_slot_failure), "{:?}", result.error);
        let candidate = result.reads.iter().find(|read| read.path == SourceBuffers::key(&dir.join("types.yaml")));
        assert_eq!(candidate.map(|read| read.stat), Some(None), "the missing candidate is watched: {:?}", result.reads);
    }

    /// Without a grant the returned import is refused before it is read:
    /// the job fails with approval required and nothing was read.
    #[test]
    fn a_template_type_job_without_a_grant_needs_approval_and_reads_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let dir = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        std::fs::write(dir.join("types.yaml"), "kind: schema\ntypes:\n  Person:\n    role: enum(admin)\n").unwrap();
        let text = "---\n$schema:\n  owner: \"{{ 'Person@./types.yaml' }}\"\nowner:\n  role: admin\n---\n";
        let store = SlotSnapshots::default();
        let doc = dir.join("doc.md");
        store.demand(&doc, 1, true, || JobInputs { text: text.to_string(), ..inputs(&dir) });
        let result = run_job(store.take_demands().pop().unwrap());
        assert_eq!(failure_codes(&result), [darkmatter::markdown::schemas::slots::codes::SLOT_APPROVAL_REQUIRED_CODE], "{:?}", result.error);
        assert!(result.reads.is_empty(), "nothing was read: {:?}", result.reads);
    }

    #[test]
    fn a_once_grant_changes_every_view_generation() {
        let dir = tempfile::tempdir().unwrap();
        let store = SlotSnapshots::default();
        let doc = dir.path().join("doc.md");
        let before = store.view(&doc).generation;
        store.add_once(Grant {
            decision: darkmatter::markdown::schemas::slots::grants::GrantDecision::Allow,
            target: darkmatter::markdown::schemas::slots::grants::GrantTarget::File(dir.path().join("x.yaml")),
            scope: darkmatter::markdown::schemas::slots::grants::GrantScope::Once,
        });
        assert_ne!(store.view(&doc).generation, before);
        assert_eq!(store.once_grants().len(), 1);
    }

    /// A document with an eligible property and no template gets a job that
    /// resolves no slot, observes the property, and judges it.
    #[test]
    fn a_property_only_job_observes_and_judges_without_resolving_slots() {
        let (_temp, dir) = canonical_dir();
        std::fs::write(dir.join("data.yaml"), "- name: Ada\n  age: 36\n- name: Grace\n  age: old\n").unwrap();
        let store = observing_store(&dir);
        let doc = dir.join("doc.md");
        assert!(store.demand(&doc, 1, true, || observed_inputs(&dir, OBSERVED_DOC)));
        let job = store.take_demands().pop().unwrap();
        assert!(!job.slot_demand && !job.templated);
        let result = run_job(job);
        assert!(result.error.is_none(), "{:?}", result.error);
        assert!(result.slot_reads.is_empty(), "no slot was resolved: {:?}", result.slot_reads);
        let view = result.properties.clone().expect("observed");
        assert!(matches!(view.observations.get("direct"), Some(PropertyOutcome::Observed { .. })), "{:?}", view.observations);
        match view.verdicts.get("direct") {
            Some(ObservedVerdict::Judged(problems)) => {
                assert_eq!(problems.len(), 1, "{problems:?}");
                assert_eq!(problems[0].path, "/direct/1/age");
            }
            other => panic!("{other:?}"),
        }
        assert!(result.reads.iter().any(|read| read.path == SourceBuffers::key(&dir.join("data.yaml"))));
        assert!(store.install(result));
        assert!(store.view(&doc).properties.is_some());
    }

    /// A templated document whose snapshot is still served observes over
    /// the installed session's pins: the job resolves nothing and keeps the
    /// served session and its reads.
    #[test]
    fn a_job_over_served_pins_keeps_the_installed_session() {
        let (_temp, dir) = canonical_dir();
        std::fs::write(dir.join("os.yaml"), "- linux\n").unwrap();
        std::fs::write(dir.join("data.yaml"), "- name: Ada\n  age: 36\n").unwrap();
        let text = "---\n$schema:\n  os: enum({{ load_data(\"./os.yaml\", \"string[]\") }})\n  direct: \"{ name: string }[]\"\nos: linux\ndirect: \"{{ load_data('./data.yaml') }}\"\n---\n";
        let store = observing_store(&dir);
        store.add_once(Grant {
            decision: darkmatter::markdown::schemas::slots::grants::GrantDecision::Allow,
            target: darkmatter::markdown::schemas::slots::grants::GrantTarget::File(dir.join("os.yaml")),
            scope: darkmatter::markdown::schemas::slots::grants::GrantScope::Once,
        });
        let doc = dir.join("doc.md");
        let templated = || JobInputs { templated: true, ..observed_inputs(&dir, text) };
        store.demand(&doc, 1, true, templated);
        let first = store.take_demands().pop().unwrap();
        assert!(first.slot_demand);
        let first = run_job(first);
        assert!(first.error.is_none(), "{:?}", first.error);
        let session = Arc::clone(&first.session);
        assert!(store.install(first));
        // An edit of the property only: the pins still serve.
        assert!(store.demand_with(&doc, 2, false, true, templated));
        let second = store.take_demands().pop().unwrap();
        assert!(!second.slot_demand, "the served pins are not resolved again");
        let second = run_job(second);
        assert!(Arc::ptr_eq(&second.session, &session));
        assert!(second.slot_reads.iter().any(|read| read.path == SourceBuffers::key(&dir.join("os.yaml"))));
        assert!(second.properties.is_some());
    }

    /// An installed observation is withheld while its entry is stale.
    #[test]
    fn a_stale_view_withholds_its_observation() {
        let (_temp, dir) = canonical_dir();
        std::fs::write(dir.join("data.yaml"), "- name: Ada\n  age: 36\n").unwrap();
        let store = observing_store(&dir);
        let doc = dir.join("doc.md");
        store.demand(&doc, 1, true, || observed_inputs(&dir, OBSERVED_DOC));
        assert!(store.install(run_job(store.take_demands().pop().unwrap())));
        assert!(store.view(&doc).properties.is_some());
        store.invalidate_paths(&[dir.join("data.yaml")]);
        assert!(store.view(&doc).properties.is_none());
    }

    /// A result dispatched before a change to a file it read is outdated:
    /// it is not installed, and its entry is stale so the document refreshes.
    #[test]
    fn a_result_from_an_older_input_generation_is_discarded() {
        let (_temp, dir) = canonical_dir();
        std::fs::write(dir.join("data.yaml"), "- name: Ada\n  age: 36\n").unwrap();
        let store = observing_store(&dir);
        let doc = dir.join("doc.md");
        store.demand(&doc, 1, true, || observed_inputs(&dir, OBSERVED_DOC));
        let job = store.take_demands().pop().unwrap();
        let result = run_job(job);
        // An unrelated path changing does not outdate it…
        store.invalidate_paths(&[dir.join("other.yaml")]);
        // …but the file it read does, even though nothing installed watched it.
        store.invalidate_paths(&[dir.join("data.yaml")]);
        assert_eq!(store.install_checked(result), InstallOutcome::Outdated);
        let view = store.view(&doc);
        assert!(view.refreshing && view.properties.is_none() && view.installed_key.is_none());
        assert!(store.demand(&doc, 1, true, || observed_inputs(&dir, OBSERVED_DOC)), "the document is demanded again");
    }

    /// A result whose cached remote response expired before it arrived
    /// installs without its observation.
    #[test]
    fn a_result_already_expired_at_install_installs_no_observation() {
        let (_temp, dir) = canonical_dir();
        std::fs::write(dir.join("data.yaml"), "- name: Ada\n  age: 36\n").unwrap();
        let store = observing_store(&dir);
        let doc = dir.join("doc.md");
        store.demand(&doc, 1, true, || observed_inputs(&dir, OBSERVED_DOC));
        let mut result = run_job(store.take_demands().pop().unwrap());
        result.remote_expiry = Some(SystemTime::now() - std::time::Duration::from_secs(1));
        assert!(store.install(result));
        let view = store.view(&doc);
        assert_eq!(view.installed_key, Some(1));
        assert!(view.properties.is_none());
        assert_eq!(store.next_deadline(), None);
    }

    /// The expiry deadline is the remote wall-clock expiry carried onto the
    /// monotonic clock by the job's paired sample: before it the view is
    /// served, after it it is withheld and the document is due a refresh.
    #[test]
    fn a_remote_expiry_becomes_a_monotonic_deadline_from_the_paired_clock() {
        let (_temp, dir) = canonical_dir();
        std::fs::write(dir.join("data.yaml"), "- name: Ada\n  age: 36\n").unwrap();
        let clock = Arc::new(crate::clock::ManualClock::new());
        let store = observing_store(&dir);
        store.set_clock(clock.clone());
        let doc = dir.join("doc.md");
        store.demand(&doc, 1, true, || observed_inputs(&dir, OBSERVED_DOC));
        let job = store.take_demands().pop().unwrap();
        let (wall, mono) = job.dispatched;
        let mut result = run_job(job);
        result.remote_expiry = Some(wall + std::time::Duration::from_secs(10));
        assert!(store.install(result));
        assert_eq!(store.next_deadline(), Some(mono + std::time::Duration::from_secs(10)));
        clock.advance(std::time::Duration::from_secs(9));
        assert!(store.view(&doc).properties.is_some());
        assert!(store.expire_due().is_empty());
        clock.advance(std::time::Duration::from_secs(2));
        assert!(store.view(&doc).properties.is_none(), "an expired observation is withheld on access");
        assert_eq!(store.expire_due().len(), 1);
        assert!(store.view(&doc).refreshing);
        assert_eq!(store.next_deadline(), None);
    }

    /// A file that changes between the job's read and the install, before
    /// any watch could report it, leaves the installed entry stale.
    #[test]
    fn a_read_that_changed_before_install_marks_the_entry_stale() {
        let (_temp, dir) = canonical_dir();
        std::fs::write(dir.join("data.yaml"), "- name: Ada\n  age: 36\n").unwrap();
        let store = observing_store(&dir);
        let doc = dir.join("doc.md");
        store.demand(&doc, 1, true, || observed_inputs(&dir, OBSERVED_DOC));
        let result = run_job(store.take_demands().pop().unwrap());
        std::fs::write(dir.join("data.yaml"), "- name: Ada\n  age: 36\n- name: Grace\n  age: 85\n").unwrap();
        assert!(store.install(result));
        let view = store.view(&doc);
        assert!(view.refreshing && view.properties.is_none());
    }
}
