//! One command's schema-slot resolutions: the grants its reads are decided
//! by, the snapshots it pinned, and the templates it is resolving.
//!
//! A [`SlotSession`] belongs to one command (one `md` invocation, one
//! library composition, one editor resolution job). Every pass of the command that
//! validates a document against a template resolves it through the same
//! session, so:
//!
//! - **grants are decided once per run.** The session builds one
//!   [`SlotGrants`] on first use (the user's grant store, the run-only grants
//!   pre-flight gave, the command's `--allow host=HOST` values, and the
//!   approval handler), and its automatic decisions are kept for the run: a
//!   file the run's own approved step later changes keeps the grant it had.
//! - **one snapshot is pinned per template and context.** A later validation
//!   in the run gets the same concrete schema. When the command's data memo
//!   moved on (a shell command or lifecycle effect ran), the pin's inputs are
//!   rehashed first; any change drops the pin and the template resolves
//!   again, under the grants the run already decided. Nothing validates
//!   against a stale snapshot.
//! - **a resolution in progress is never re-entered.** A slot whose
//!   `load_data` contract names a template being resolved on this thread is
//!   the typed cycle `dm.schema.slot_cycle`, positioned at the outer slot.
//!
//! Across commands, resolutions are shared through the process-wide
//! [`cache`](super::cache), which re-checks what each entry depended on and
//! that this session's grants cover every read it made.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use biscuit_file::FileResolutionContext;
use simplified_schema::SchemaError;
use simplified_schema::expression::parse_spanned;
use simplified_schema::resolve::ResolvedSchema;
use simplified_schema::simplified::{DefinitionTable, SchemaRoot, Slot};
use simplified_schema::slots::{
    PendingSchema, SlotFailure, SlotHost, SlotKey, SlotResolutionError, SlotRunRequest, SlotValues,
};
use simplified_schema::{SlotResolutionFailure, UnresolvedSlots};

use super::admission::slot_class_report;
use super::cache::{self, CacheEntry, ContextCheck, GrantedRead, InputCheck, Outcome, ResolutionKey};
use super::codes::{SLOT_CYCLE_CODE, SLOT_GRANT_STORE_INVALID_CODE};
use super::evaluator::{ContextDigest, SlotEvaluator, SlotGates, context_digest_now, context_inputs, slot_context};
use super::grants::gate::{RunGates, SlotGrants};
use super::grants::{Grant, GrantBook, GrantStore, GrantStoreError};
use super::host::DarkmatterSlotHost;
use crate::markdown::compose::DataRuntime;
use crate::markdown::compose::context::capture::CapturedObservations;
use crate::markdown::compose::expression::functions::SlotClass;
use crate::markdown::compose::expression::iteration::IterationLimits;
use crate::markdown::compose::expression::read_gate::{GateRefusal, ReadGate, ReadTarget};
use crate::markdown::compose::remote_fetch::RemoteFetchRuntime;
use crate::markdown::compose::shell_expansion::ShellApprovalHandler;

/// How far a session may go to resolve a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionMode {
    /// Resolve with the automatic, persisted, and run-only grants, asking
    /// the approval handler (when there is one) about anything else.
    #[default]
    Full,
    /// Resolve with the automatic, persisted, and run-only grants only;
    /// never ask. A template a read nothing grants (or a deny) stops is
    /// left unresolved ([`SchemaError::UnresolvedSlots`]), which a
    /// reference check defers like any value only composition can judge.
    ReferenceCheck,
    /// Never resolve: a template stays unresolved
    /// ([`SchemaError::UnresolvedSlots`]).
    Passive,
}

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

/// One command's schema-slot resolutions; see the module documentation.
///
/// Build one per command and share it (it is cheap to clone behind an
/// `Arc`) through
/// [`ComposeOptions::with_slot_session`](crate::markdown::compose::ComposeOptions::with_slot_session)
/// or [`DarkmatterSchemas::with_slot_session`](crate::markdown::schemas::DarkmatterSchemas::with_slot_session).
/// Nothing is read until a template needs resolving.
pub struct SlotSession {
    id: u64,
    data: DataRuntime,
    observations: CapturedObservations,
    remote: Option<RemoteFetchRuntime>,
    hosts: Vec<String>,
    repository_root: Option<PathBuf>,
    handler: Option<Arc<dyn ShellApprovalHandler>>,
    run_grants: Vec<Grant>,
    mode: SessionMode,
    home: Option<PathBuf>,
    cancel: Option<Arc<AtomicBool>>,
    limits: Option<IterationLimits>,
    grants: OnceLock<Result<Arc<SlotGrants>, Arc<GrantStoreError>>>,
    pinned: Mutex<HashMap<ResolutionKey, Pin>>,
}

impl std::fmt::Debug for SlotSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlotSession")
            .field("id", &self.id)
            .field("mode", &self.mode)
            .field("repository_root", &self.repository_root)
            .field("handler", &self.handler.is_some())
            .finish_non_exhaustive()
    }
}

/// A snapshot pinned for the command, with the memo generation it was last
/// confirmed at and the inputs that confirm it.
struct Pin {
    outcome: Outcome,
    generation: u64,
    inputs: Vec<InputCheck>,
}

thread_local! {
    /// The templates being resolved on this thread, by session, innermost
    /// last; and the session whose slot is evaluating, for a nested
    /// contract to resolve through.
    static RESOLVING: RefCell<Vec<(Arc<SlotSession>, String, FileResolutionContext)>> = const { RefCell::new(Vec::new()) };
}

/// Pops [`RESOLVING`] when a resolution ends, however it ends.
struct Resolving;

impl Resolving {
    fn enter(session: &Arc<SlotSession>, template: String, context: &FileResolutionContext) -> Self {
        RESOLVING.with_borrow_mut(|stack| stack.push((Arc::clone(session), template, context.clone())));
        Self
    }
}

impl Drop for Resolving {
    fn drop(&mut self) {
        RESOLVING.with_borrow_mut(Vec::pop);
    }
}

/// Whether `template` is being resolved by `session` on this thread.
fn in_progress(session: u64, template: &str) -> bool {
    RESOLVING.with_borrow(|stack| stack.iter().any(|(owner, key, _)| owner.id == session && key == template))
}

/// The session whose slot is evaluating on this thread, and that
/// resolution's request context.
fn active() -> Option<(Arc<SlotSession>, FileResolutionContext)> {
    RESOLVING.with_borrow(|stack| stack.last().map(|(session, _, context)| (Arc::clone(session), context.clone())))
}

/// The id of the session whose slot is evaluating on this thread; the data
/// memo keys a contract prepared there apart from one prepared outside.
pub(crate) fn active_session_id() -> Option<u64> {
    RESOLVING.with_borrow(|stack| stack.last().map(|(session, _, _)| session.id))
}

/// The cycle a nested resolution met: the template it needed is already
/// being resolved.
#[derive(Debug, Clone)]
pub(crate) struct SlotCycle {
    template: String,
}

impl std::fmt::Display for SlotCycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "resolving this slot needs `{}`, whose slots are being resolved", self.template)
    }
}

impl std::error::Error for SlotCycle {}

/// Whether `error` is, or wraps, a [`SlotCycle`].
pub(crate) fn is_cycle(error: &SchemaError) -> bool {
    let SchemaError::SlotResolution(failure) = error else {
        return false;
    };
    match failure.error.as_ref() {
        SlotResolutionError::Failed(failed) => failed.iter().any(|(_, failure)| failure.code == Some(SLOT_CYCLE_CODE)),
        SlotResolutionError::Schema(schema) => is_cycle(schema),
        _ => false,
    }
}

impl SlotSession {
    /// A session for one command reading through `data` (the command's
    /// `load_data` runtime, whose memo and read set the slots share), with
    /// automatic grants judged against no repository, no handler, and no
    /// grant store until [`Self::with_home`] names a home.
    pub fn new(data: DataRuntime) -> Self {
        Self {
            id: NEXT_SESSION.fetch_add(1, Ordering::Relaxed),
            data,
            observations: CapturedObservations::default(),
            remote: None,
            hosts: Vec::new(),
            repository_root: None,
            handler: None,
            run_grants: Vec::new(),
            mode: SessionMode::Full,
            home: None,
            cancel: None,
            limits: None,
            grants: OnceLock::new(),
            pinned: Mutex::new(HashMap::new()),
        }
    }

    /// The session a composition request builds when its caller attached
    /// none: the request's data runtime, captured observations, remote
    /// runtime, `--allow host=HOST` values, repository, home (for the grant
    /// store), and approval handler. A command that composes more than once
    /// builds it once from its first request and attaches it to every pass
    /// with [`ComposeOptions::with_slot_session`](crate::markdown::compose::ComposeOptions::with_slot_session).
    /// Attach the command's data runtime to the request first
    /// (`ComposeOptions::with_data_runtime`): a request without one gives the
    /// session a runtime of its own, whose reads never reach the command's
    /// read set.
    pub fn for_request(request: &crate::markdown::compose::ComposeRequest) -> Self {
        let options = request.options();
        let mut session = Self::new(options.data_runtime())
            .with_home(request.resolution_context().home_dir().map(Path::to_path_buf))
            .with_repository_root(options.slot_repository_root())
            .with_allowed_hosts(options.remote_read_config.http_hosts().cloned().collect());
        session.observations = options.context().observations().clone();
        session.remote = options.remote_fetch.clone();
        if let Some(handler) = &options.shell_approval_handler {
            session = session.with_handler(Arc::clone(handler));
        }
        session
    }

    /// A session for a command that validates without composing (`md schema
    /// validate`, `md clean`): reading through `data`, judging automatic
    /// grants against `context`'s repository, and reading the grant store
    /// below `context`'s home. It has no captured observations, so a slot
    /// probing `PATH` (`has_command`) sees an empty one.
    pub fn for_context(data: DataRuntime, context: &FileResolutionContext) -> Self {
        Self::new(data)
            .with_home(context.home_dir().map(Path::to_path_buf))
            .with_repository_root(context.repository_root().map(Path::to_path_buf))
    }

    /// Resolves no further than `mode`.
    #[must_use]
    pub fn with_mode(mut self, mode: SessionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Asks `handler` about a read nothing else decides.
    #[must_use]
    pub fn with_handler(mut self, handler: Arc<dyn ShellApprovalHandler>) -> Self {
        self.handler = Some(handler);
        self
    }

    /// Adds grants that last for this run (pre-flight's
    /// [`slot_grants`](crate::markdown::compose::preflight::ComposePreflightApprovals::slot_grants)).
    #[must_use]
    pub fn with_run_grants(mut self, grants: impl IntoIterator<Item = Grant>) -> Self {
        self.run_grants.extend(grants);
        self
    }

    /// The command's `--allow host=HOST` values: each is also a site grant
    /// for this run.
    #[must_use]
    pub fn with_allowed_hosts(mut self, hosts: Vec<String>) -> Self {
        self.hosts = hosts;
        self
    }

    /// Judges automatic grants against the repository at `root`: the
    /// request's own observation, never a new discovery.
    #[must_use]
    pub fn with_repository_root(mut self, root: Option<PathBuf>) -> Self {
        self.repository_root = root;
        self
    }

    /// Reads persisted grants from (and persists them to) the grant store
    /// below `home`; without a home nothing persisted applies.
    #[must_use]
    pub fn with_home(mut self, home: Option<PathBuf>) -> Self {
        self.home = home;
        self
    }

    /// Stops a resolution in progress once `cancel` is set: the slot being
    /// evaluated finishes, no further slot starts, and the resolution fails
    /// with `dm.schema.slot_canceled`, which is never cached or pinned. An
    /// editor sets it when a newer edit supersedes the resolution.
    #[must_use]
    pub fn with_cancel(mut self, cancel: Arc<AtomicBool>) -> Self {
        self.cancel = Some(cancel);
        self
    }

    /// Gives each slot `limits` instead of the default work allowances, so
    /// a slot that does more work fails with the ordinary `dm.iteration.*`
    /// code at the slot. An editor passes smaller limits than a command. A
    /// failure these limits caused is never cached for another session; a
    /// resolution that succeeded is shared as usual.
    #[must_use]
    pub fn with_slot_limits(mut self, limits: IterationLimits) -> Self {
        self.limits = Some(limits);
        self
    }

    /// How far this session resolves.
    pub fn mode(&self) -> SessionMode {
        self.mode
    }

    /// The grants in force now, for a check that a snapshot's reads are
    /// covered; `None` until a template needed them, or when the grant store
    /// is invalid.
    pub fn grants(&self) -> Option<GrantBook> {
        self.grants.get().and_then(|grants| grants.as_ref().ok()).map(|grants| grants.book())
    }

    /// Asks the session's approval handler once, in one batch, about every
    /// read the slots of `templates` name before evaluation (a literal or
    /// pure target) that no grant or automatic grant decides yet, and keeps
    /// its answers for the run (a persisted answer is also written to the
    /// grant store). Returns how many reads were asked about.
    ///
    /// This is the collect-and-approve step of a command that validates
    /// without the compose pre-flight; call it before the first validation.
    /// Nothing is asked in a [`SessionMode::ReferenceCheck`] or
    /// [`SessionMode::Passive`] session, or without a handler; an undecided
    /// read then stays `dm.schema.slot_approval_required` when its slot
    /// resolves. A computed target is judged when its slot evaluates. An
    /// invalid grant store asks nothing here and fails the resolution with
    /// `dm.schema.slot_grant_store_invalid`.
    ///
    /// ## Errors
    ///
    /// The handler's own failure.
    pub fn approve_reads(
        &self,
        templates: &[crate::markdown::schemas::PendingTemplate],
        context: &FileResolutionContext,
    ) -> Result<usize, crate::markdown::compose::ShellExpansionError> {
        if self.mode != SessionMode::Full || templates.is_empty() {
            return Ok(0);
        }
        let Some(handler) = self.handler.as_ref() else {
            return Ok(0);
        };
        let Ok(grants) = self.run_grants() else {
            return Ok(0);
        };
        let requests = grants.undecided_requests(templates, context);
        if requests.is_empty() {
            return Ok(0);
        }
        let decisions = handler.approve_slot_reads(&requests)?;
        grants.apply_all(&requests, &decisions);
        Ok(requests.len())
    }

    /// The run's grants, built on first use.
    fn run_grants(&self) -> Result<Arc<SlotGrants>, Arc<GrantStoreError>> {
        self.grants
            .get_or_init(|| {
                let store = GrantStore::load(GrantStore::path_for_home(self.home.as_deref()).as_deref())?;
                let mut book = GrantBook::new(store, self.hosts.clone());
                for grant in &self.run_grants {
                    book.add_once(grant.decision, grant.target.clone());
                }
                let handler = match self.mode {
                    SessionMode::Full => self.handler.clone(),
                    SessionMode::ReferenceCheck | SessionMode::Passive => None,
                };
                Ok(Arc::new(SlotGrants::new(book, self.repository_root.as_deref(), handler)))
            })
            .clone()
    }

    /// Resolves `pending` in `context`: the pinned snapshot, a cached
    /// resolution that still holds, or a fresh one. `template` identifies
    /// the template beyond its files (the `$schema` value or payload and
    /// where it was written). With `pins_only`, only a pinned snapshot is
    /// used (a pass that must not read anything a slot names).
    ///
    /// ## Errors
    ///
    /// [`SchemaError::UnresolvedSlots`] when the session is passive (or only
    /// pins may be used and none is pinned); [`SchemaError::SlotResolution`]
    /// when resolving failed, with the slots' typed causes.
    pub(crate) fn resolve(
        self: &Arc<Self>,
        pending: &PendingSchema,
        template: &str,
        context: &FileResolutionContext,
        pins_only: bool,
    ) -> Result<Arc<ResolvedSchema>, SchemaError> {
        let unresolved = || unresolved_template(pending);
        if self.mode == SessionMode::Passive {
            return Err(unresolved());
        }
        let resolved = self.resolve_in_mode(pending, template, context, pins_only);
        match resolved {
            // A reference check never asks: a slot whose read nobody
            // granted leaves the template unresolved for it to defer.
            Err(SchemaError::SlotResolution(failure))
                if self.mode == SessionMode::ReferenceCheck && needs_grant(&failure.error) =>
            {
                Err(unresolved())
            }
            other => other,
        }
    }

    fn resolve_in_mode(
        self: &Arc<Self>,
        pending: &PendingSchema,
        template: &str,
        context: &FileResolutionContext,
        pins_only: bool,
    ) -> Result<Arc<ResolvedSchema>, SchemaError> {
        let unresolved = || unresolved_template(pending);
        let key = cache::resolution_key(pending, template, context, self.repository_root.as_deref());
        if let Some(outcome) = self.pinned(&key) {
            return failure_of(outcome, pending);
        }
        if pins_only {
            return Err(unresolved());
        }
        let template_key = template_key(pending, template);
        if in_progress(self.id, &template_key) {
            return Err(cycle_error(pending.slots(), &template_key));
        }
        if let Some(entry) = cache::lookup(&key)
            && self.serves(&entry, context)
        {
            // Replay the reads so the command's read set is what a cold run
            // reports.
            for check in &entry.inputs {
                self.data.observe(check.observation.clone());
            }
            self.pin(key, entry.outcome.clone(), entry.inputs.clone());
            return failure_of(entry.outcome.clone(), pending);
        }
        let outcome = self.resolve_fresh(key, pending, &template_key, context);
        failure_of(outcome, pending)
    }

    /// Resolves `pending` now, storing and pinning the outcome.
    fn resolve_fresh(
        self: &Arc<Self>,
        key: ResolutionKey,
        pending: &PendingSchema,
        template_key: &str,
        context: &FileResolutionContext,
    ) -> Outcome {
        let grants = match self.run_grants() {
            Ok(grants) => grants,
            Err(error) => return Err(Arc::new(store_failure(pending.slots(), &error))),
        };
        let recorder = Arc::new(RecordingGates { inner: RunGates(Arc::clone(&grants)), reads: Arc::default() });
        let host = DarkmatterSlotHost::new(self.evaluator(&grants, Arc::clone(&recorder) as Arc<dyn SlotGates>))
            .with_cancel(self.cancel.clone());
        let (result, observed) = {
            let _resolving = Resolving::enter(self, template_key.to_string(), context);
            crate::markdown::compose::expression::data::runtime::capture_observations(|| {
                pending.resolve(&host, &SlotRunRequest { context: context.clone() })
            })
        };
        let outcome: Outcome = result.map(Arc::new).map_err(Arc::new);
        let cacheable = match &outcome {
            Ok(_) => true,
            // A session with its own (smaller) work limits must not hand
            // the failure they caused to a command allowed more work.
            Err(error) => cache::caches_failure(error) && !(self.limits.is_some() && exhausted_work(error)),
        };
        if !cacheable {
            return outcome;
        }
        let mut inputs: Vec<InputCheck> = Vec::new();
        for observation in observed {
            if !inputs.iter().any(|known| known.observation == observation) {
                inputs.push(InputCheck::capture(observation));
            }
        }
        let volatile = pending.slots().iter().any(is_volatile);
        let only_for = (volatile || inputs.iter().any(|input| !input.rechecks())).then_some(self.id);
        let entry = CacheEntry {
            outcome: outcome.clone(),
            inputs: inputs.clone(),
            context: context_checks(pending.slots(), context),
            reads: recorder.reads.lock().unwrap_or_else(PoisonError::into_inner).clone(),
            only_for,
        };
        cache::store(key, Arc::new(entry));
        self.pin(key, outcome.clone(), inputs);
        outcome
    }

    fn evaluator(&self, grants: &Arc<SlotGrants>, gates: Arc<dyn SlotGates>) -> SlotEvaluator {
        let evaluator = SlotEvaluator::new(self.data.clone())
            .with_gates(gates)
            .with_remote(grants.remote_runtime(self.remote.as_ref()))
            .with_observations(self.observations.clone());
        match self.limits {
            Some(limits) => evaluator.with_limits(limits),
            None => evaluator,
        }
    }

    /// Whether the cached `entry` may be served to this session in
    /// `context`: it belongs to any session or this one, this session's
    /// grants cover every read it made, and its inputs and context inputs are
    /// unchanged. Coverage is judged first, because confirming an input
    /// rereads it.
    fn serves(&self, entry: &CacheEntry, context: &FileResolutionContext) -> bool {
        if entry.only_for.is_some_and(|session| session != self.id) {
            return false;
        }
        if !entry.reads.is_empty() {
            let Ok(grants) = self.run_grants() else {
                return false;
            };
            if !entry
                .reads
                .iter()
                .all(|read| matches!(grants.decide_without_asking(&read.defining, &read.target), Some(Ok(()))))
            {
                return false;
            }
        }
        entry.inputs.iter().all(|input| input.holds(true))
            && entry.context.iter().all(|check| cache::context_holds(check, context))
    }

    /// The pinned outcome for `key`, re-confirmed when the command's data
    /// memo moved on since it was pinned; a pin whose inputs changed is
    /// dropped.
    fn pinned(&self, key: &ResolutionKey) -> Option<Outcome> {
        let generation = self.data.memo_generation();
        let mut pinned = self.pinned.lock().unwrap_or_else(PoisonError::into_inner);
        let pin = pinned.get_mut(key)?;
        if pin.generation != generation {
            if !pin.inputs.iter().all(|input| input.holds(false)) {
                pinned.remove(key);
                return None;
            }
            pin.generation = generation;
        }
        Some(pin.outcome.clone())
    }

    fn pin(&self, key: ResolutionKey, outcome: Outcome, inputs: Vec<InputCheck>) {
        let generation = self.data.memo_generation();
        self.pinned.lock().unwrap_or_else(PoisonError::into_inner).insert(key, Pin { outcome, generation, inputs });
    }
}

/// `pending` left unresolved, keeping its resolution handle so a caller can
/// still read the template's literal parts.
fn unresolved_template(pending: &PendingSchema) -> SchemaError {
    let mut unresolved = UnresolvedSlots::new(pending.slots().to_vec());
    unresolved.pending = Some(pending.clone());
    SchemaError::UnresolvedSlots(Box::new(unresolved))
}

/// `outcome` as [`DarkmatterSchemas`](crate::markdown::schemas::DarkmatterSchemas)
/// reports it.
fn failure_of(outcome: Outcome, pending: &PendingSchema) -> Result<Arc<ResolvedSchema>, SchemaError> {
    outcome.map_err(|error| {
        SchemaError::SlotResolution(Box::new(SlotResolutionFailure::new(error, pending.read_sources().to_vec().into())))
    })
}

/// Whether a slot of `error` stopped on a work limit (`dm.iteration.*`).
fn exhausted_work(error: &SlotResolutionError) -> bool {
    match error {
        SlotResolutionError::Failed(failed) => {
            failed.iter().any(|(_, failure)| failure.code.is_some_and(|code| code.starts_with("dm.iteration.")))
        }
        _ => false,
    }
}

/// Whether `error` failed only because a read was not granted (or denied),
/// so a session that never asks leaves its template unresolved.
fn needs_grant(error: &SlotResolutionError) -> bool {
    match error {
        SlotResolutionError::Failed(failed) => {
            !failed.is_empty()
                && failed.iter().all(|(_, failure)| {
                    matches!(failure.code, Some(super::codes::SLOT_APPROVAL_REQUIRED_CODE | super::codes::SLOT_DENIED_CODE))
                })
        }
        _ => false,
    }
}

/// The key a template is known by on the resolution stack: the schema file
/// that holds it (the first file the resolution read), else its identity.
fn template_key(pending: &PendingSchema, template: &str) -> String {
    match pending.read_sources().first() {
        Some((path, _)) => biscuit_file::to_portable_string(path),
        None => template.to_string(),
    }
}

/// The cycle error for `slots`, whose template `template` is already being
/// resolved, positioned at the first slot.
fn cycle_error(slots: &[Slot], template: &str) -> SchemaError {
    let failed = slots
        .first()
        .map(|slot| {
            (slot.clone(), SlotFailure::new(SlotCycle { template: template.to_string() }, Some(SLOT_CYCLE_CODE)))
        })
        .into_iter()
        .collect();
    SchemaError::SlotResolution(Box::new(SlotResolutionFailure::new(
        Arc::new(SlotResolutionError::Failed(failed)),
        Vec::new().into(),
    )))
}

/// The failure of every slot of a template whose reads cannot be decided
/// because the user's grant store is invalid.
fn store_failure(slots: &[Slot], error: &Arc<GrantStoreError>) -> SlotResolutionError {
    SlotResolutionError::Failed(
        slots
            .iter()
            .map(|slot| {
                (slot.clone(), SlotFailure::from_shared(Arc::clone(error) as _, Some(SLOT_GRANT_STORE_INVALID_CODE)))
            })
            .collect(),
    )
}

/// Whether `slot` reads live state, so its resolution is never served to
/// another command.
fn is_volatile(slot: &Slot) -> bool {
    parse_spanned(&slot.expression).is_ok_and(|parsed| slot_class_report(&parsed).class == SlotClass::Volatile)
}

/// The context inputs each slot reads, with the digest each has now in its
/// own context.
fn context_checks(slots: &[Slot], request: &FileResolutionContext) -> Vec<ContextCheck> {
    let mut checks = Vec::new();
    for slot in slots {
        let Ok(parsed) = parse_spanned(&slot.expression) else {
            continue;
        };
        let context = slot_context(request, slot.origin.as_deref());
        for input in context_inputs(&parsed) {
            let check = ContextCheck {
                defining: slot.origin.clone(),
                digest: ContextDigest { digest: context_digest_now(&input, &context), input },
            };
            if !checks.contains(&check) {
                checks.push(check);
            }
        }
    }
    checks
}

/// The run's gates, remembering every read they allowed so a cached result
/// is served only to a session whose grants cover the same reads.
#[derive(Debug)]
struct RecordingGates {
    inner: RunGates,
    reads: Arc<Mutex<Vec<GrantedRead>>>,
}

impl SlotGates for RecordingGates {
    fn gate_for(&self, defining: &Path) -> Arc<dyn ReadGate> {
        Arc::new(RecordingGate {
            inner: self.inner.gate_for(defining),
            defining: defining.to_path_buf(),
            reads: Arc::clone(&self.reads),
        })
    }
}

/// One slot's gate, recording into its [`RecordingGates`].
#[derive(Debug)]
struct RecordingGate {
    inner: Arc<dyn ReadGate>,
    defining: PathBuf,
    reads: Arc<Mutex<Vec<GrantedRead>>>,
}

impl ReadGate for RecordingGate {
    fn check(&self, target: &ReadTarget) -> Result<(), GateRefusal> {
        self.inner.check(target)?;
        let read = GrantedRead { defining: self.defining.clone(), target: ReadTarget { observed_hash: None, ..target.clone() } };
        let mut reads = self.reads.lock().unwrap_or_else(PoisonError::into_inner);
        if !reads.contains(&read) {
            reads.push(read);
        }
        Ok(())
    }
}

/// Resolves the slots of a `load_data` contract that names a template,
/// through the session whose slot is evaluating on this thread, and
/// substitutes their values into `schema` and `definitions`.
///
/// Returns `None` outside a slot evaluation: there a contract naming a
/// template stays unresolved and lowering it fails with
/// [`SchemaError::UnresolvedSlots`].
///
/// ## Errors
///
/// The cycle `dm.schema.slot_cycle` when the template is the one being
/// resolved (or encloses it); otherwise the slots' refusals, failures, or
/// unusable values.
pub(crate) fn resolve_contract_slots(
    schema: &mut SchemaRoot,
    definitions: &mut DefinitionTable,
) -> Option<Result<(), SchemaError>> {
    let (session, context) = active()?;
    let slots = simplified_schema::slots::collect_slots(schema, definitions);
    if slots.is_empty() {
        return Some(Ok(()));
    }
    let template = slots
        .iter()
        .find_map(|slot| slot.origin.as_deref().map(biscuit_file::to_portable_string))
        .unwrap_or_else(|| "load_data contract".to_string());
    if in_progress(session.id, &template) {
        return Some(Err(cycle_error(&slots, &template)));
    }
    let grants = match session.run_grants() {
        Ok(grants) => grants,
        Err(error) => return Some(Err(contract_failure(store_failure(&slots, &error)))),
    };
    let host = DarkmatterSlotHost::new(session.evaluator(&grants, Arc::new(RunGates(Arc::clone(&grants)))));
    let _resolving = Resolving::enter(&session, template, &context);
    let mut values = SlotValues::default();
    let mut unique: Vec<&Slot> = Vec::new();
    for slot in &slots {
        if !unique.iter().any(|seen| SlotKey::of(seen) == SlotKey::of(slot)) {
            unique.push(slot);
        }
    }
    let refused: Vec<_> =
        unique.iter().filter_map(|slot| host.admit(slot).err().map(|error| ((*slot).clone(), error))).collect();
    if !refused.is_empty() {
        return Some(Err(contract_failure(SlotResolutionError::Refused(refused))));
    }
    let mut failed = Vec::new();
    // These slots read under their own approvals, never the guard of the
    // slot whose contract named the template.
    simplified_schema::resolve::buffers::unguarded(|| {
        for slot in unique {
            let request = simplified_schema::slots::SlotRequest { slot, defining_file: slot.origin.as_deref(), context: &context };
            match host.resolve(&request) {
                Ok(value) => {
                    values.insert(slot, value);
                }
                Err(failure) => failed.push((slot.clone(), failure)),
            }
        }
    });
    if !failed.is_empty() {
        return Some(Err(contract_failure(SlotResolutionError::Failed(failed))));
    }
    Some(simplified_schema::slots::substitute(schema, definitions, &values))
}

fn contract_failure(error: SlotResolutionError) -> SchemaError {
    SchemaError::SlotResolution(Box::new(SlotResolutionFailure::new(Arc::new(error), Vec::new().into())))
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
