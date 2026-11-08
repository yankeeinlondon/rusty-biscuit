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
//! an approval handler: the automatic, persisted, and in-memory (this server
//! process) grants decide every read, and anything else is
//! `dm.schema.slot_approval_required`, never a prompt. The installed session
//! keeps the snapshots it pinned; requests serve them through
//! [`DarkmatterSchemas::with_slot_pins_only`](darkmatter::markdown::schemas::DarkmatterSchemas::with_slot_pins_only),
//! which evaluates and reads nothing a slot names.
//!
//! A change to anything a resolution read (a watched path, an open buffer, a
//! `stat` difference in the rescan fallback, a grant, a configuration change)
//! marks the document's entry stale. Requests keep serving the last-good
//! snapshot, marked stale, until the refresh it demands is installed.

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
use darkmatter::markdown::schemas::TriggerRegistry;
use darkmatter::markdown::schemas::slots::SlotSession;
use darkmatter::markdown::schemas::slots::grants::Grant;
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
    /// The open YAML and JSON buffers, which `load_data` and schema
    /// resolution read in place of their files.
    pub buffers: SourceBuffers,
    /// The in-memory grants given in this server process.
    pub once_grants: Vec<Grant>,
    pub cancel: Arc<AtomicBool>,
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
    pub canceled: bool,
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
}

struct Installed {
    session: Arc<SlotSession>,
    error: Option<Arc<SchemaError>>,
    reads: Vec<WatchedRead>,
    base_key: u64,
}

struct InFlight {
    seq: u64,
    base_key: u64,
    cancel: Arc<AtomicBool>,
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
}

/// Every open document's slot snapshot and the jobs demanded for them.
///
/// Shared by the overlay (which reads views and records demands on the
/// request path, never blocking on the worker) and the dispatch thread
/// (which drains demands to the worker and installs results).
#[derive(Default)]
pub struct SlotSnapshots {
    inner: Mutex<Inner>,
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
}

impl SlotSnapshots {
    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// What requests for the document at `path` serve. A lookup only.
    pub fn view(&self, path: &Path) -> SlotView {
        let inner = self.inner();
        let Some(entry) = inner.entries.get(path) else {
            return SlotView { generation: inner.epoch, ..SlotView::default() };
        };
        SlotView {
            session: entry.installed.as_ref().map(|installed| Arc::clone(&installed.session)),
            error: entry.installed.as_ref().and_then(|installed| installed.error.clone()),
            generation: entry.generation ^ (inner.epoch << 32),
            refreshing: entry.stale || entry.in_flight.is_some(),
            installed_key: entry.installed.as_ref().map(|installed| installed.base_key),
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
    pub(crate) fn demand(&self, path: &Path, base_key: u64, pins_missed: bool, inputs: impl FnOnce() -> JobInputs) -> bool {
        let mut inner = self.inner();
        let seq = inner.next_seq + 1;
        let once = inner.once.clone();
        let entry = inner.entries.entry(path.to_path_buf()).or_default();
        let resolved_for_key = entry.installed.as_ref().is_some_and(|installed| installed.base_key == base_key);
        if !(entry.stale || (pins_missed && !resolved_for_key)) {
            return false;
        }
        if let Some(in_flight) = &entry.in_flight {
            if in_flight.base_key == base_key {
                return false;
            }
            in_flight.cancel.store(true, Ordering::Release);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        entry.in_flight = Some(InFlight { seq, base_key, cancel: Arc::clone(&cancel) });
        entry.stale = false;
        entry.generation += 1;
        inner.next_seq = seq;
        let inputs = inputs();
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
    pub(crate) fn install(&self, result: SlotResult) -> bool {
        let mut inner = self.inner();
        let Some(entry) = inner.entries.get_mut(&result.path) else {
            return false;
        };
        if result.canceled || entry.in_flight.as_ref().is_none_or(|in_flight| in_flight.seq != result.seq) {
            return false;
        }
        entry.in_flight = None;
        entry.installed = Some(Installed {
            session: result.session,
            error: result.error,
            reads: result.reads,
            base_key: result.base_key,
        });
        entry.generation += 1;
        true
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
        for entry in self.inner().entries.values_mut() {
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
        for entry in self.inner().entries.values_mut() {
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

/// Resolves one job's templates with a fresh session.
pub(crate) fn run_job(job: SlotJob) -> SlotResult {
    let started = std::time::Instant::now();
    let data = DataRuntime::new();
    let session = Arc::new(
        SlotSession::for_context(data.clone(), &job.context)
            .with_run_grants(job.once_grants.iter().cloned())
            .with_cancel(Arc::clone(&job.cancel))
            .with_slot_limits(DMLS_SLOT_LIMITS),
    );
    let assembled = simplified_schema::resolve::buffers::with(&job.buffers, || {
        crate::overlay::resolve_slot_templates(
            &job.path,
            &job.text,
            &job.config,
            &job.workspace_roots,
            job.registry.clone(),
            &job.context,
            &session,
        )
    });
    let error = assembled.err().map(Arc::new);
    let canceled = job.cancel.load(Ordering::Acquire);
    let reads = slot_reads(&data, job.context.repository_root());
    tracing::debug!(
        path = %job.path.display(),
        seq = job.seq,
        reads = reads.len(),
        failed = error.is_some(),
        canceled,
        elapsed_ms = started.elapsed().as_millis(),
        "schema slots resolved"
    );
    SlotResult {
        uri: job.uri,
        path: job.path,
        seq: job.seq,
        base_key: job.base_key,
        session,
        error,
        reads,
        canceled,
    }
}

/// Every local path the resolution observed (content reads, existence
/// answers, missing probes, listings), plus the repository's index and
/// `HEAD` when any of them lies inside it: an automatic grant depends on
/// what the index and `HEAD` record for the file.
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
        && paths.iter().any(|path| path.starts_with(root))
        && let Some(git_dir) = git_dir(root)
    {
        paths.insert(SourceBuffers::key(&git_dir.join("index")));
        paths.insert(SourceBuffers::key(&git_dir.join("HEAD")));
    }
    paths.into_iter().map(WatchedRead::now).collect()
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

    fn context(dir: &Path) -> FileResolutionContext {
        FileResolutionContext::from_snapshot(dir, None, std::collections::HashMap::new())
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
        }
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

    /// A job for `text` at `dir/doc.md`, reading with a once grant for the
    /// whole folder.
    fn granted_job(dir: &Path, text: &str) -> SlotJob {
        let store = SlotSnapshots::default();
        let doc = dir.join("doc.md");
        store.add_once(Grant {
            decision: darkmatter::markdown::schemas::slots::grants::GrantDecision::Allow,
            target: darkmatter::markdown::schemas::slots::grants::GrantTarget::Dir(dir.to_path_buf()),
            scope: darkmatter::markdown::schemas::slots::grants::GrantScope::Once,
        });
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
        crate::overlay::resolve_slot_templates(&dir.join("doc.md"), runaway, &DmlsConfig::default(), &[], None, &context, &command)
            .expect("within the command allowance");
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
}
