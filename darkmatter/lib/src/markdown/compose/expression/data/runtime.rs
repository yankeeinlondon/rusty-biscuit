//! The request-wide `load_data` runtime: memo, read set, and counters.
//!
//! One [`DataRuntime`] spans a whole command. It is held on
//! `ComposeOptions` and cloned (sharing its state) into every expression
//! context the command builds, so the reference check, pre-flight, the
//! lifecycle, composition, and nested documents read through one memo and
//! report into one read set.
//!
//! The memo has three layers. A raw source (the decoded text, or the offset
//! of its first invalid UTF-8 byte, with the size and BLAKE3 hash of what was
//! read) is keyed by the resolved identity alone; a parsed source by a
//! source key (identity, effective format, effective delimiter); and a
//! bound result by a bind key (that source key, the contract key, and the
//! effective header). So two delimiters over one file read it once and parse
//! it twice, two contracts parse it once and bind it twice, and an omitted
//! default shares its entries with the same default written out. Each entry
//! is single-flight: the first caller loads, and concurrent callers for the
//! same key wait for its result. The map lock is never held while a loader
//! runs, and no loader consults another layer.
//!
//! [`DataRuntime::invalidate_memo`] ends reuse after a command ran: it starts
//! a new generation with every layer empty and leaves the read set alone.
//! Each layer is computed against the generation of the entry below it, so a
//! load already in flight finishes into its old entry for its own caller and
//! never populates the new maps at any layer.
//! [`DataRuntime::memo_generation`] tells a holder of earlier results that
//! this happened.
//!
//! Beside the read set, the runtime keeps an ordered [`Observation`] log:
//! every input an expression function observed (content with the BLAKE3 hash
//! of the bytes read, an existence answer, a missing probe, a listing). The
//! hash of a read lives on its raw entry, so a memo hit at any layer records
//! the same observation again, and a caller that marks the log
//! ([`DataRuntime::read_mark`]) and reads its slice afterward
//! ([`DataRuntime::observations_since`]) sees every input it depended on.

use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use biscuit_file::DataFormat;

use super::contract::PreparedContract;
use super::error::DataError;

/// The resolved identity of a data source: a canonical local path or a
/// normalized URL. It keys the source memo; [`Self::read_set_entry`] is its
/// portable spelling.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataSourceIdentity {
    /// A local file, by its canonical native path.
    Local(PathBuf),
    /// A remote document, by its normalized URL (query and fragment kept).
    /// Boxed so the typed errors that carry an identity stay small.
    Remote(Box<url::Url>),
}

impl DataSourceIdentity {
    /// The identity of the local file at `path`: its canonical path when the
    /// file exists, otherwise its canonical folder joined with its name, and
    /// `path` itself when neither can be canonicalized.
    pub fn local(path: &Path) -> Self {
        Self::Local(canonical_or_normalized(path))
    }

    /// The text this identity contributes to a read set: portable absolute
    /// path text, or [`url::Url::to_string`].
    pub fn read_set_entry(&self) -> String {
        match self {
            Self::Local(path) => biscuit_file::to_portable_string(path),
            Self::Remote(url) => url.to_string(),
        }
    }
}

impl std::fmt::Display for DataSourceIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.read_set_entry())
    }
}

/// A canonical path for comparison, even for a file that does not exist yet:
/// a missing file's folder is canonicalized so it spells the same as the file
/// will once created (`/var` and `/private/var` on macOS).
fn canonical_or_normalized(path: &Path) -> PathBuf {
    if let Ok(canonical) = biscuit_file::canonicalize_simplified(path) {
        return canonical;
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => biscuit_file::canonicalize_simplified(parent)
            .map(|parent| parent.join(name))
            .unwrap_or_else(|_| path.to_path_buf()),
        _ => path.to_path_buf(),
    }
}

/// What kind of input an [`Observation`] records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservationKind {
    /// A local file's bytes were read; the hash is of those bytes.
    File,
    /// A URL's body was fetched; the hash is of the body text.
    Url,
    /// A target was found to exist; nothing was read.
    Exists,
    /// A candidate was probed and absent.
    MissingProbe,
    /// A directory was listed; the hash is of the sorted entry names.
    Listing,
    /// A glob pattern was listed; the entry is the pattern as written and
    /// the hash is of the sorted matched paths.
    Glob,
}

/// One input an expression function observed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Observation {
    /// The input: a read-set entry (portable absolute path or URL), or for a
    /// glob listing the pattern as written.
    pub entry: String,
    /// What was observed.
    pub kind: ObservationKind,
    /// BLAKE3 of what was read; `None` when nothing was read (an existence
    /// answer or a probe) or the read failed, so a failure is still a
    /// dependency.
    pub hash: Option<[u8; 32]>,
}

impl Observation {
    /// Whether [`Self::entry`] names a file, folder, or URL and so also
    /// belongs in the read set; a glob pattern does not.
    fn is_read_set_entry(&self) -> bool {
        self.kind != ObservationKind::Glob
    }
}

/// A data source as read, before any format interprets it.
#[derive(Debug, Clone)]
pub(crate) struct RawSource {
    /// The BOM-stripped text, or the zero-based offset of the first invalid
    /// UTF-8 byte in the original bytes (a byte-order mark counted). The
    /// failure stays format-free here; the parse layer names its format.
    pub(crate) text: Result<Arc<str>, usize>,
    /// The size of the document as read (local bytes, or the fetched text
    /// before its byte-order mark is removed), in bytes.
    pub(crate) len: usize,
    /// BLAKE3 of what was read (local bytes before decoding, or the fetched
    /// text), so a memo hit at any layer records the same observation.
    pub(crate) hash: [u8; 32],
}

/// A data document read, decoded, and parsed, before any contract applies.
#[derive(Debug, Clone)]
pub struct LoadedSource {
    /// The parsed content.
    pub content: SourceContent,
    /// The format it was parsed as.
    pub format: DataFormat,
    /// The BOM-stripped text, local or fetched, for locating failures; shared
    /// with the raw entry and every other interpretation of it.
    pub text: Arc<str>,
    /// The size of the document as read (local bytes or fetched text), in
    /// bytes.
    pub bytes: usize,
}

/// What a source parsed into.
///
/// CSV and TSV stay a table because their row shape is chosen per contract
/// and header choice when binding, while the parsed source is keyed without
/// either.
#[derive(Debug, Clone)]
pub enum SourceContent {
    /// A value tree (YAML, JSON, JSON Lines, TOML, JSON5).
    Tree(serde_json::Value),
    /// A delimited table (CSV, TSV).
    Table(Arc<biscuit_file::DelimitedData>),
}

/// A data document bound to one contract.
#[derive(Debug, Clone)]
pub struct LoadedData {
    /// The converted value `load_data` returns.
    pub value: serde_json::Value,
    /// The number of top-level items (array elements or object members).
    pub items: usize,
}

/// Counts of the work a [`DataRuntime`] did, for assertions about reuse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DataRuntimeStats {
    /// Raw loaders run (each one reads a file or takes a fetched body). A raw
    /// lookup answered from the memo counts nothing, so raw reuse shows as
    /// this staying flat.
    pub reads: usize,
    /// Documents parsed, as reported by the parse loaders.
    pub parses: usize,
    /// Binders run (each one validates a source against a contract).
    pub binds: usize,
    /// Parsed-source lookups answered from the memo.
    pub source_hits: usize,
    /// Bind lookups answered from the memo.
    pub bind_hits: usize,
    /// Contracts prepared.
    pub contract_preparations: usize,
    /// Contract lookups answered from the memo.
    pub contract_hits: usize,
    /// Times the memo was emptied after a command ran.
    pub invalidations: usize,
}

#[derive(Default)]
struct Counters {
    reads: AtomicUsize,
    parses: AtomicUsize,
    binds: AtomicUsize,
    source_hits: AtomicUsize,
    bind_hits: AtomicUsize,
    contract_preparations: AtomicUsize,
    contract_hits: AtomicUsize,
    invalidations: AtomicUsize,
}

impl Counters {
    fn snapshot(&self) -> DataRuntimeStats {
        let read = |counter: &AtomicUsize| counter.load(Ordering::SeqCst);
        DataRuntimeStats {
            reads: read(&self.reads),
            parses: read(&self.parses),
            binds: read(&self.binds),
            source_hits: read(&self.source_hits),
            bind_hits: read(&self.bind_hits),
            contract_preparations: read(&self.contract_preparations),
            contract_hits: read(&self.contract_hits),
            invalidations: read(&self.invalidations),
        }
    }
}

fn bump(counter: &AtomicUsize) {
    counter.fetch_add(1, Ordering::SeqCst);
}

type Outcome<T> = Result<Arc<T>, Arc<DataError>>;
type Slot<T> = Arc<OnceLock<Outcome<T>>>;
/// Selects one of a generation's maps.
type MapOf<R, S, B, K, T> = fn(&mut Generation<R, S, B>) -> &mut HashMap<K, Slot<T>>;

/// The parse-layer key: one interpretation of one source. The format and
/// delimiter are the effective ones (an alias normalized, a default filled
/// in), so equivalent spellings share an entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SourceKey {
    pub(crate) identity: DataSourceIdentity,
    pub(crate) format: DataFormat,
    pub(crate) delimiter: Option<u8>,
}

/// The bind-layer key: a parsed source, a contract, and the effective
/// header choice (`None` for a format without one).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BindKey {
    source: SourceKey,
    contract: String,
    header: Option<bool>,
}

struct Generation<R, S, B> {
    number: u64,
    raw: HashMap<DataSourceIdentity, Slot<R>>,
    sources: HashMap<SourceKey, Slot<S>>,
    bindings: HashMap<BindKey, Slot<B>>,
    contracts: HashMap<String, Slot<PreparedContract>>,
}

impl<R, S, B> Generation<R, S, B> {
    fn new(number: u64) -> Self {
        Self {
            number,
            raw: HashMap::new(),
            sources: HashMap::new(),
            bindings: HashMap::new(),
            contracts: HashMap::new(),
        }
    }
}

/// A raw source as one caller received it, with the generation it was read
/// in.
pub(crate) struct RawEntry<R> {
    generation: u64,
    value: Arc<R>,
    from_memo: bool,
}

impl<R> RawEntry<R> {
    pub(crate) fn value(&self) -> &R {
        &self.value
    }

    /// Whether the memo answered, so this caller ran no raw loader.
    pub(crate) fn answered_from_memo(&self) -> bool {
        self.from_memo
    }
}

/// A parsed source as one caller received it, with the generation it was
/// loaded in.
pub struct SourceEntry<S> {
    key: SourceKey,
    generation: u64,
    value: Arc<S>,
    from_memo: bool,
}

impl<S> SourceEntry<S> {
    /// The parsed source.
    pub fn value(&self) -> &S {
        &self.value
    }

    /// The source's identity.
    pub fn identity(&self) -> &DataSourceIdentity {
        &self.key.identity
    }

    /// Whether the memo answered, so this caller ran no loader.
    pub fn from_memo(&self) -> bool {
        self.from_memo
    }
}

/// A bound result as one caller received it.
pub struct BindEntry<B> {
    value: Arc<B>,
    from_memo: bool,
}

impl<B> BindEntry<B> {
    /// The bound result.
    pub fn value(&self) -> &B {
        &self.value
    }

    /// Whether the memo answered, so this caller ran no binder.
    pub fn from_memo(&self) -> bool {
        self.from_memo
    }
}

/// The three-layer single-flight memo, generic over the raw source `R`, the
/// parsed source `S`, and the bound result `B`.
pub(crate) struct DataMemo<R, S, B> {
    state: Mutex<Generation<R, S, B>>,
    counters: Counters,
}

impl<R, S, B> Default for DataMemo<R, S, B> {
    fn default() -> Self {
        Self { state: Mutex::new(Generation::new(0)), counters: Counters::default() }
    }
}

impl<R, S, B> DataMemo<R, S, B> {
    fn lock(&self) -> MutexGuard<'_, Generation<R, S, B>> {
        // A loader never runs under the lock, so a poisoned lock only means a
        // panic while inserting a slot; the maps are still consistent.
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn stats(&self) -> DataRuntimeStats {
        self.counters.snapshot()
    }

    pub(crate) fn note_parse(&self) {
        bump(&self.counters.parses);
    }

    fn generation(&self) -> u64 {
        self.lock().number
    }

    /// Starts a new generation with every layer empty.
    pub(crate) fn invalidate(&self) {
        let mut state = self.lock();
        let next = state.number + 1;
        *state = Generation::new(next);
        bump(&self.counters.invalidations);
    }

    /// Runs `init` at most once per key and generation, returning its outcome,
    /// the generation the slot belongs to, and whether this caller ran it.
    ///
    /// With `expected` set, a key is memoized only when that is still the
    /// current generation; otherwise `init` runs for this caller alone and
    /// the outcome keeps the stale `expected` generation. A
    /// deferral outcome is removed from the map so a later caller tries again.
    fn single_flight<K: Hash + Eq + Clone, T>(
        &self,
        map: MapOf<R, S, B, K, T>,
        key: K,
        expected: Option<u64>,
        init: impl FnOnce() -> Result<T, Arc<DataError>>,
    ) -> (Outcome<T>, u64, bool) {
        let (slot, generation) = {
            let mut state = self.lock();
            let generation = state.number;
            if let Some(expected) = expected
                && expected != generation
            {
                drop(state);
                // The stale generation is passed on, so the next layer's
                // entry built from this one is not memoized either.
                return (init().map(Arc::new), expected, true);
            }
            (map(&mut state).entry(key.clone()).or_default().clone(), generation)
        };
        let mut ran = false;
        let outcome = slot
            .get_or_init(|| {
                ran = true;
                init().map(Arc::new)
            })
            .clone();
        if let Err(error) = &outcome
            && error.is_deferral()
        {
            let mut state = self.lock();
            if state.number == generation {
                let entries = map(&mut state);
                if entries.get(&key).is_some_and(|current| Arc::ptr_eq(current, &slot)) {
                    entries.remove(&key);
                }
            }
        }
        (outcome, generation, ran)
    }

    /// The raw source for `identity`, running `loader` only when no entry of
    /// the current generation holds it. Only a run counts (`reads`).
    ///
    /// ## Errors
    ///
    /// The loader's error, memoized unless it is a deferral.
    pub(crate) fn load_raw(
        &self,
        identity: &DataSourceIdentity,
        loader: impl FnOnce() -> Result<R, Arc<DataError>>,
    ) -> Result<RawEntry<R>, Arc<DataError>> {
        let (outcome, generation, ran) = self.single_flight(|g| &mut g.raw, identity.clone(), None, || {
            bump(&self.counters.reads);
            loader()
        });
        outcome.map(|value| RawEntry { generation, value, from_memo: !ran })
    }

    /// `raw` interpreted under `key`, running `loader` only when no entry
    /// holds it. A raw source read before the last invalidation is parsed for
    /// this caller alone and never memoized: its bytes may be stale.
    ///
    /// ## Errors
    ///
    /// The loader's error, memoized unless it is a deferral.
    pub(crate) fn load_source(
        &self,
        key: &SourceKey,
        raw: &RawEntry<R>,
        loader: impl FnOnce(&R) -> Result<S, Arc<DataError>>,
    ) -> Result<SourceEntry<S>, Arc<DataError>> {
        let (outcome, generation, ran) =
            self.single_flight(|g| &mut g.sources, key.clone(), Some(raw.generation), || loader(&raw.value));
        if !ran {
            bump(&self.counters.source_hits);
        }
        outcome.map(|value| SourceEntry { key: key.clone(), generation, value, from_memo: !ran })
    }

    /// `source` bound to the contract `contract_key` with the effective
    /// `header`, running `binder` only when no entry holds it. A source
    /// loaded before the last invalidation is bound for this caller alone and
    /// never memoized: its bytes may be stale.
    ///
    /// ## Errors
    ///
    /// The binder's error, memoized unless it is a deferral.
    pub(crate) fn bind(
        &self,
        source: &SourceEntry<S>,
        contract_key: &str,
        header: Option<bool>,
        binder: impl FnOnce(&S) -> Result<B, Arc<DataError>>,
    ) -> Result<BindEntry<B>, Arc<DataError>> {
        let key = BindKey { source: source.key.clone(), contract: contract_key.to_string(), header };
        let (outcome, _, ran) = self.single_flight(|g| &mut g.bindings, key, Some(source.generation), || {
            bump(&self.counters.binds);
            binder(&source.value)
        });
        if !ran {
            bump(&self.counters.bind_hits);
        }
        outcome.map(|value| BindEntry { value, from_memo: !ran })
    }

    /// Drops the prepared contract under `key`, so the next caller prepares it.
    pub(crate) fn forget_contract(&self, key: &str) {
        self.lock().contracts.remove(key);
    }

    /// The prepared contract under `key`, running `prepare` only when no entry
    /// holds it.
    ///
    /// ## Errors
    ///
    /// The preparation's error, memoized unless it is a deferral.
    pub(crate) fn contract(
        &self,
        key: String,
        prepare: impl FnOnce() -> Result<PreparedContract, Box<DataError>>,
    ) -> Result<Arc<PreparedContract>, Arc<DataError>> {
        let (outcome, _, ran) = self.single_flight(|g| &mut g.contracts, key, None, || {
            bump(&self.counters.contract_preparations);
            prepare().map_err(Arc::from)
        });
        if !ran {
            bump(&self.counters.contract_hits);
        }
        outcome
    }
}

/// The request-wide `load_data` runtime and read-set sink.
///
/// Cloning shares the state. A runtime is attached to `ComposeOptions` with
/// [`ComposeOptions::with_shared_data_runtime`] (or
/// [`ComposeOptions::with_data_runtime`] to keep a handle); a root
/// composition with none attached builds its own, so separate compositions
/// never share results. Nothing persists past the runtime.
///
/// [`ComposeOptions::with_shared_data_runtime`]: crate::markdown::compose::ComposeOptions::with_shared_data_runtime
/// [`ComposeOptions::with_data_runtime`]: crate::markdown::compose::ComposeOptions::with_data_runtime
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::DataRuntime;
///
/// let runtime = DataRuntime::new();
/// assert!(runtime.read_set().is_empty());
/// assert_eq!(runtime.stats().reads, 0);
/// ```
#[derive(Clone, Default)]
pub struct DataRuntime {
    inner: Arc<DataRuntimeInner>,
}

#[derive(Default)]
struct DataRuntimeInner {
    memo: DataMemo<RawSource, LoadedSource, LoadedData>,
    read_set: Mutex<BTreeSet<String>>,
    observations: Mutex<Vec<Observation>>,
    /// What each prepared contract observed, by memo key.
    contract_observations: Mutex<HashMap<String, Arc<Vec<Observation>>>>,
    /// What the slots of a contract naming a template observed and consumed
    /// while it was prepared inside a slot, by memo key: a memo hit replays
    /// them, so the enclosing slot's resolution depends on them as a cold
    /// preparation's does.
    contract_slot_inputs: Mutex<HashMap<String, Arc<ContractSlotInputs>>>,
}

/// What a contract's own slots observed and consumed during its preparation.
struct ContractSlotInputs {
    observations: Vec<Observation>,
    dependencies: crate::markdown::schemas::slots::dependencies::SlotDependencies,
}

/// A position in a [`DataRuntime`]'s observation log ([`DataRuntime::read_mark`]).
/// The default mark is the start of the log.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReadMark(usize);

impl std::fmt::Debug for DataRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataRuntime").field("stats", &self.stats()).finish_non_exhaustive()
    }
}

impl DataRuntime {
    /// A runtime with an empty memo and read set.
    pub fn new() -> Self {
        Self::default()
    }

    /// The work this runtime did so far.
    pub fn stats(&self) -> DataRuntimeStats {
        self.inner.memo.stats()
    }

    /// The files and URLs expressions read so far, deduplicated, in lexical
    /// order.
    pub fn read_set(&self) -> Vec<String> {
        self.read_set_guard().iter().cloned().collect()
    }

    /// Whether `self` and `other` share one state.
    pub fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    fn read_set_guard(&self) -> MutexGuard<'_, BTreeSet<String>> {
        self.inner.read_set.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Records a read of `identity`.
    pub(crate) fn record(&self, identity: &DataSourceIdentity) {
        self.read_set_guard().insert(identity.read_set_entry());
    }

    /// Records a read of the local file at `path`; see
    /// [`DataSourceIdentity::local`].
    pub(crate) fn record_local(&self, path: &Path) {
        self.record(&DataSourceIdentity::local(path));
    }

    /// Records a read of `url`. Recording authorizes nothing.
    pub(crate) fn record_url(&self, url: &url::Url) {
        self.record(&DataSourceIdentity::Remote(Box::new(url.clone())));
    }

    /// Adds `entry` to the read set as it is: a cached result replays the
    /// reads its first run recorded, so the command reports what a cold run
    /// would.
    pub fn record_entry(&self, entry: &str) {
        self.read_set_guard().insert(entry.to_string());
    }

    /// Appends `observation` to the log (and to every [`capture_observations`]
    /// scope running on this thread), and its entry to the read set when it
    /// names a file, folder, or URL.
    pub(crate) fn observe(&self, observation: Observation) {
        if observation.is_read_set_entry() {
            self.record_entry(&observation.entry);
        }
        CAPTURES.with_borrow_mut(|frames| frames.iter_mut().for_each(|frame| frame.push(observation.clone())));
        self.observations_guard().push(observation);
    }

    /// Observes `identity` read with `hash` (`None`: the read failed).
    pub(crate) fn observe_read(&self, identity: &DataSourceIdentity, hash: Option<[u8; 32]>) {
        let kind = match identity {
            DataSourceIdentity::Local(_) => ObservationKind::File,
            DataSourceIdentity::Remote(_) => ObservationKind::Url,
        };
        self.observe(Observation { entry: identity.read_set_entry(), kind, hash });
    }

    /// Observes whether the local `path` exists, without reading it.
    pub(crate) fn observe_existence(&self, path: &Path, exists: bool) {
        let kind = if exists { ObservationKind::Exists } else { ObservationKind::MissingProbe };
        self.observe(Observation { entry: DataSourceIdentity::local(path).read_set_entry(), kind, hash: None });
    }

    /// Observes a listing (of the folder `entry`, or of the glob pattern
    /// `entry` for [`ObservationKind::Glob`]) whose names were `names`.
    pub(crate) fn observe_listing(&self, kind: ObservationKind, entry: String, mut names: Vec<String>) {
        names.sort();
        let hash = biscuit_hash::blake3_hash_bytes(names.join("\0").as_bytes());
        self.observe(Observation { entry, kind, hash: Some(hash) });
    }

    fn observations_guard(&self) -> MutexGuard<'_, Vec<Observation>> {
        self.inner.observations.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The current end of the observation log.
    pub fn read_mark(&self) -> ReadMark {
        ReadMark(self.observations_guard().len())
    }

    /// Every observation recorded after `mark`, in order, duplicates kept.
    pub fn observations_since(&self, mark: ReadMark) -> Vec<Observation> {
        self.observations_guard().get(mark.0..).map(<[Observation]>::to_vec).unwrap_or_default()
    }

    /// Empties the memo after a command ran; see the module documentation.
    pub(crate) fn invalidate_memo(&self) {
        self.inner.memo.invalidate();
    }

    /// The memo's generation: it starts at 0 and grows by one each time
    /// [`Self::invalidate_memo`] ends reuse after a command ran. A holder of a
    /// result computed in an earlier generation re-checks what it observed.
    pub fn memo_generation(&self) -> u64 {
        self.inner.memo.generation()
    }

    /// Counts one parse; called by a parse loader.
    pub(crate) fn note_parse(&self) {
        self.inner.memo.note_parse();
    }

    /// The raw source for `identity`; see [`DataMemo::load_raw`]. The caller
    /// records the read before calling, so a failed read is recorded too.
    pub(crate) fn load_raw(
        &self,
        identity: &DataSourceIdentity,
        loader: impl FnOnce() -> Result<RawSource, Arc<DataError>>,
    ) -> Result<RawEntry<RawSource>, Arc<DataError>> {
        self.inner.memo.load_raw(identity, loader)
    }

    /// `raw` parsed under `key`; see [`DataMemo::load_source`].
    pub(crate) fn load_source(
        &self,
        key: &SourceKey,
        raw: &RawEntry<RawSource>,
        loader: impl FnOnce(&RawSource) -> Result<LoadedSource, Arc<DataError>>,
    ) -> Result<SourceEntry<LoadedSource>, Arc<DataError>> {
        self.inner.memo.load_source(key, raw, loader)
    }

    /// `source` bound to the contract `contract_key` with the effective
    /// `header`; see [`DataMemo::bind`].
    pub(crate) fn bind(
        &self,
        source: &SourceEntry<LoadedSource>,
        contract_key: &str,
        header: Option<bool>,
        binder: impl FnOnce(&LoadedSource) -> Result<LoadedData, Arc<DataError>>,
    ) -> Result<BindEntry<LoadedData>, Arc<DataError>> {
        self.inner.memo.bind(source, contract_key, header, binder)
    }

    /// The contract `written` describes in `ctx`'s document context,
    /// prepared once per context identity and text. The schema files the
    /// preparation read (imports at any depth, including the one that made
    /// it fail) are observed with the hash of the text read, and every other
    /// location an import probed (a missing import) as a missing probe,
    /// whether it succeeds or fails. A memo hit observes the same again, and
    /// also replays what the contract's own slots observed and consumed (a
    /// contract naming a template, prepared inside a slot), including each
    /// read their gates allowed with the contract file that defined it, so
    /// every resolution that loads through the contract depends on them and
    /// is served to another command only under grants covering those reads.
    ///
    /// In a schema slot, `ctx`'s gate judges each schema file before it is
    /// opened. A memo hit opens nothing; the inputs it observes are judged by
    /// the dispatch's check of what the call observed.
    ///
    /// ## Errors
    ///
    /// As [`prepare_contract`](super::contract::prepare_contract), and
    /// [`ExpressionError::SlotReadRefused`](crate::markdown::compose::expression::ExpressionError::SlotReadRefused)
    /// for the first import a slot may not read; that preparation is not kept.
    pub(crate) fn prepare_contract(
        &self,
        written: &str,
        ctx: &crate::markdown::compose::expression::ResolutionContext,
    ) -> Result<Arc<PreparedContract>, crate::markdown::compose::expression::ExpressionError> {
        let document = &ctx.file_context();
        let identity = crate::markdown::compose::file_resolution_context_identity(document);
        // A contract prepared inside a slot's evaluation may resolve a
        // template through that slot's session, so it is kept apart.
        let session = crate::markdown::schemas::slots::session::active_session_id()
            .map_or_else(String::new, |session| format!("\0slots:{session}"));
        let key = format!("{identity:016x}\0{written}{session}");
        let in_slot = !session.is_empty();
        let mut refused = None;
        let mut ran = false;
        let prepared = self.inner.memo.contract(key.clone(), || {
            ran = true;
            let prepare = || super::contract::prepare_contract(written, document);
            let (((((prepared, refusal), texts), observed), slot_observations), consumed) =
                crate::markdown::schemas::slots::dependencies::collect(|| {
                    capture_observations(|| {
                        simplified_schema::resolve::observed::collect(|| {
                            simplified_schema::resolve::buffers::recording(|| {
                                ctx.gate_schema_reads(super::load::FUNCTION, prepare)
                            })
                        })
                    })
                });
            // Kept per key, before the memo slot is filled, so every caller
            // the slot answers (one waiting on this preparation included)
            // observes the same inputs.
            self.contract_observations_guard().insert(key.clone(), Arc::new(contract_observations(&observed, &texts)));
            if in_slot {
                let inputs = ContractSlotInputs { observations: slot_observations, dependencies: consumed };
                self.contract_slot_inputs_guard().insert(key.clone(), Arc::new(inputs));
            }
            refused = refusal;
            prepared
        });
        if let Some(refusal) = refused {
            // A refusal is this gate's answer, not the contract's: a later
            // caller with other grants prepares it again.
            self.inner.memo.forget_contract(&key);
            self.contract_observations_guard().remove(&key);
            self.contract_slot_inputs_guard().remove(&key);
            return Err(refusal);
        }
        if !ran && let Some(inputs) = self.contract_slot_inputs_guard().get(&key).cloned() {
            // A cold preparation observed these as its slots ran.
            inputs.observations.iter().cloned().for_each(|observation| self.observe(observation));
            crate::markdown::schemas::slots::dependencies::replay(&inputs.dependencies);
        }
        let observations = self.contract_observations_guard().get(&key).cloned();
        observations.iter().flat_map(|observations| observations.iter()).cloned().for_each(|observation| self.observe(observation));
        prepared.map_err(Into::into)
    }

    fn contract_observations_guard(&self) -> MutexGuard<'_, HashMap<String, Arc<Vec<Observation>>>> {
        self.inner.contract_observations.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn contract_slot_inputs_guard(&self) -> MutexGuard<'_, HashMap<String, Arc<ContractSlotInputs>>> {
        self.inner.contract_slot_inputs.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

thread_local! {
    static CAPTURES: std::cell::RefCell<Vec<Vec<Observation>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Runs `run`, returning its result with every observation any runtime made
/// on this thread meanwhile, in order. Unlike a [`ReadMark`] slice of one
/// runtime's log, it never includes what another thread observed.
pub(crate) fn capture_observations<T>(run: impl FnOnce() -> T) -> (T, Vec<Observation>) {
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            CAPTURES.with_borrow_mut(Vec::pop);
        }
    }
    CAPTURES.with_borrow_mut(|frames| frames.push(Vec::new()));
    let pop = Pop;
    let result = run();
    let observed = CAPTURES.with_borrow_mut(|frames| frames.last_mut().map(std::mem::take)).unwrap_or_default();
    drop(pop);
    (result, observed)
}

/// What a contract preparation observed: each schema file read, with the
/// hash of the text the resolver read, and each other location an import
/// probed (a missing import), so creating it is a change.
fn contract_observations(
    observed: &simplified_schema::resolve::observed::Observed,
    texts: &simplified_schema::resolve::buffers::ReadSources,
) -> Vec<Observation> {
    let text_hash = |file: &Path| {
        let key = DataSourceIdentity::local(file);
        texts
            .iter()
            .find(|(path, _)| DataSourceIdentity::local(path) == key)
            .map(|(_, text)| biscuit_hash::blake3_hash_bytes(text.as_bytes()))
    };
    let mut observations: Vec<Observation> = observed
        .read
        .iter()
        .map(|file| Observation {
            entry: DataSourceIdentity::local(file).read_set_entry(),
            kind: ObservationKind::File,
            hash: text_hash(file),
        })
        .collect();
    observations.extend(observed.searched.iter().filter(|probe| !observed.read.contains(*probe)).map(|probe| {
        Observation { entry: DataSourceIdentity::local(probe).read_set_entry(), kind: ObservationKind::MissingProbe, hash: None }
    }));
    observations
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::sync::atomic::AtomicBool;

    /// Raw text, parsed text, and a bound length stand in for the real types.
    type Memo = DataMemo<String, String, usize>;

    fn identity(name: &str) -> DataSourceIdentity {
        DataSourceIdentity::Local(PathBuf::from(format!("/data/{name}")))
    }

    fn key(file: &DataSourceIdentity, format: DataFormat, delimiter: Option<u8>) -> SourceKey {
        SourceKey { identity: file.clone(), format, delimiter }
    }

    fn json(file: &DataSourceIdentity) -> SourceKey {
        key(file, DataFormat::Json, None)
    }

    /// Reads `text` through the raw layer and parses it unchanged under `key`.
    fn load(memo: &Memo, key: &SourceKey, text: &str) -> SourceEntry<String> {
        let raw = memo.load_raw(&key.identity, || Ok(text.to_string())).unwrap();
        memo.load_source(key, &raw, |raw| Ok(raw.clone())).unwrap()
    }

    fn missing(kind: crate::markdown::compose::expression::FileRefFailure) -> Arc<DataError> {
        Arc::new(DataError::FileReference(crate::markdown::compose::expression::FileReferenceDiagnostic {
            function: "load_data",
            reference: "a.json".to_string(),
            kind,
            cwd: PathBuf::from("/data"),
            fallback_dir: None,
            source: None,
            caller: None,
        }))
    }

    #[test]
    fn a_source_is_read_once_and_two_contracts_bind_it_twice() {
        let memo = Memo::default();
        let file = json(&identity("hosts.yaml"));
        for contract in ["a", "b", "a"] {
            let source = load(&memo, &file, "hosts");
            let bound = memo.bind(&source, contract, None, |text| Ok(text.len() + contract.len())).unwrap();
            assert_eq!(*bound.value(), 6);
        }
        let stats = memo.stats();
        assert_eq!((stats.reads, stats.source_hits), (1, 2));
        assert_eq!((stats.binds, stats.bind_hits), (2, 1));
    }

    #[test]
    fn each_level_says_whether_the_memo_answered() {
        let memo = Memo::default();
        let file = identity("flags.yaml");
        let first_raw = memo.load_raw(&file, || Ok("x".to_string())).unwrap();
        let first = memo.load_source(&json(&file), &first_raw, |raw| Ok(raw.clone())).unwrap();
        let first_bind = memo.bind(&first, "c", None, |text| Ok(text.len())).unwrap();
        assert!(!first_raw.answered_from_memo() && !first.from_memo() && !first_bind.from_memo());
        let second_raw = memo.load_raw(&file, || Ok("x".to_string())).unwrap();
        let second = memo.load_source(&json(&file), &second_raw, |raw| Ok(raw.clone())).unwrap();
        let second_bind = memo.bind(&second, "c", None, |text| Ok(text.len())).unwrap();
        assert!(second_raw.answered_from_memo() && second.from_memo() && second_bind.from_memo());
    }

    #[test]
    fn raw_source_is_read_once_for_two_parse_keys() {
        let memo = Memo::default();
        let file = identity("rows.txt");
        let parses = AtomicUsize::new(0);
        for delimiter in *b",;,;" {
            let raw = memo.load_raw(&file, || Ok("a;b".to_string())).unwrap();
            let parsed = memo
                .load_source(&key(&file, DataFormat::Csv, Some(delimiter)), &raw, |raw| {
                    parses.fetch_add(1, Ordering::SeqCst);
                    Ok(raw.split(delimiter as char).count().to_string())
                })
                .unwrap();
            assert_eq!(parsed.value(), if delimiter == b',' { "1" } else { "2" });
            assert_eq!(parsed.identity(), &file);
        }
        assert_eq!(parses.load(Ordering::SeqCst), 2);
        let stats = memo.stats();
        assert_eq!((stats.reads, stats.source_hits), (1, 2), "{stats:?}");
    }

    #[test]
    fn a_raw_failure_is_shared_by_two_parse_keys() {
        use crate::markdown::compose::expression::FileRefFailure;
        let memo = Memo::default();
        let file = identity("absent.csv");
        for _ in 0..2 {
            let failed = memo.load_raw(&file, || Err(missing(FileRefFailure::NotFound)));
            assert!(failed.is_err());
        }
        let stats = memo.stats();
        assert_eq!((stats.reads, stats.source_hits, stats.parses), (1, 0, 0), "{stats:?}");
    }

    #[test]
    fn a_header_choice_is_its_own_binding_over_one_parse() {
        let memo = Memo::default();
        let file = key(&identity("rows.csv"), DataFormat::Csv, Some(b','));
        for header in [Some(true), Some(false), Some(true)] {
            let source = load(&memo, &file, "a,b");
            let bound = memo.bind(&source, "c", header, |text| Ok(text.len() + usize::from(header == Some(true)))).unwrap();
            assert_eq!(*bound.value(), 3 + usize::from(header == Some(true)));
        }
        let stats = memo.stats();
        assert_eq!((stats.reads, stats.source_hits, stats.binds, stats.bind_hits), (1, 2, 2, 1), "{stats:?}");
    }

    #[test]
    fn concurrent_callers_share_one_load() {
        let memo = Arc::new(Memo::default());
        let file = identity("shared.yaml");
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let first = {
            let (memo, file, entered, release) = (memo.clone(), file.clone(), entered.clone(), release.clone());
            std::thread::spawn(move || {
                memo.load_raw(&file, || {
                    entered.wait();
                    release.wait();
                    Ok("loaded".to_string())
                })
                .map(|entry| entry.value().clone())
            })
        };
        entered.wait();
        let second = {
            let (memo, file) = (memo.clone(), file.clone());
            std::thread::spawn(move || memo.load_raw(&file, || Ok("second loader".to_string())).map(|entry| entry.value().clone()))
        };
        release.wait();
        assert_eq!(first.join().unwrap().unwrap(), "loaded");
        assert_eq!(second.join().unwrap().unwrap(), "loaded");
        assert_eq!(memo.stats().reads, 1);
    }

    #[test]
    fn concurrent_callers_share_one_raw_load_and_one_parse() {
        let memo = Arc::new(Memo::default());
        let file = key(&identity("shared.csv"), DataFormat::Csv, Some(b';'));
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let parses = Arc::new(AtomicUsize::new(0));
        let caller = |gate: Option<(Arc<Barrier>, Arc<Barrier>)>| {
            let (memo, file, parses) = (memo.clone(), file.clone(), parses.clone());
            std::thread::spawn(move || {
                let raw = memo
                    .load_raw(&file.identity, || {
                        if let Some((entered, release)) = &gate {
                            entered.wait();
                            release.wait();
                        }
                        Ok("a;b".to_string())
                    })
                    .unwrap();
                let parsed = memo
                    .load_source(&file, &raw, |raw| {
                        parses.fetch_add(1, Ordering::SeqCst);
                        Ok(raw.to_uppercase())
                    })
                    .unwrap();
                parsed.value().clone()
            })
        };
        let first = caller(Some((entered.clone(), release.clone())));
        entered.wait();
        let second = caller(None);
        release.wait();
        assert_eq!(first.join().unwrap(), "A;B");
        assert_eq!(second.join().unwrap(), "A;B");
        assert_eq!(memo.stats().reads, 1);
        assert_eq!(parses.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn invalidation_reads_again_and_a_late_old_load_never_populates_the_new_maps() {
        let memo = Arc::new(Memo::default());
        let file = json(&identity("regenerated.yaml"));
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let old = {
            let (memo, file, entered, release) = (memo.clone(), file.clone(), entered.clone(), release.clone());
            std::thread::spawn(move || {
                let raw = memo
                    .load_raw(&file.identity, || {
                        entered.wait();
                        release.wait();
                        Ok("old".to_string())
                    })
                    .unwrap();
                let source = memo.load_source(&file, &raw, |raw| Ok(raw.clone())).unwrap();
                *memo.bind(&source, "contract", None, |text| Ok(text.len())).unwrap().value()
            })
        };
        entered.wait();
        memo.invalidate();
        release.wait();
        assert_eq!(old.join().unwrap(), 3, "the in-flight caller still gets its own result");

        let source = load(&memo, &file, "newer");
        assert_eq!(source.value(), "newer", "the old load is not reused after invalidation");
        let bound = memo.bind(&source, "contract", None, |text| Ok(text.len())).unwrap();
        assert_eq!(*bound.value(), 5, "the old binding did not populate the new generation");
        let stats = memo.stats();
        assert_eq!((stats.reads, stats.binds, stats.invalidations), (2, 2, 1));
    }

    #[test]
    fn invalidation_clears_all_three_layers_and_a_late_raw_load_never_populates_them() {
        let memo = Arc::new(Memo::default());
        let file = key(&identity("late.csv"), DataFormat::Csv, Some(b','));
        // Every layer is warm before the in-flight read starts.
        let warm = load(&memo, &file, "warm");
        memo.bind(&warm, "c", Some(true), |text| Ok(text.len())).unwrap();
        memo.invalidate();

        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let late = {
            let (memo, file, entered, release) = (memo.clone(), file.clone(), entered.clone(), release.clone());
            std::thread::spawn(move || {
                let raw = memo
                    .load_raw(&file.identity, || {
                        entered.wait();
                        release.wait();
                        Ok("late".to_string())
                    })
                    .unwrap();
                let source = memo.load_source(&file, &raw, |raw| Ok(raw.clone())).unwrap();
                *memo.bind(&source, "c", Some(true), |text| Ok(text.len() * 10)).unwrap().value()
            })
        };
        entered.wait();
        memo.invalidate();
        release.wait();
        assert_eq!(late.join().unwrap(), 40, "the in-flight caller still gets its own result");

        let source = load(&memo, &file, "fresh!");
        assert_eq!(source.value(), "fresh!", "neither the warm nor the late raw entry survived");
        assert!(!source.from_memo(), "the late parse did not populate the new generation");
        let bound = memo.bind(&source, "c", Some(true), |text| Ok(text.len())).unwrap();
        assert!(!bound.from_memo(), "the late binding did not populate the new generation");
        assert_eq!(*bound.value(), 6);
        let stats = memo.stats();
        assert_eq!((stats.reads, stats.parses, stats.source_hits, stats.bind_hits), (3, 0, 0, 0), "{stats:?}");
        assert_eq!(stats.invalidations, 2);
    }

    #[test]
    fn a_parse_from_an_older_raw_generation_is_not_memoized() {
        let memo = Memo::default();
        let file = json(&identity("stale.json"));
        let stale = memo.load_raw(&file.identity, || Ok("stale".to_string())).unwrap();
        memo.invalidate();
        let parsed = memo.load_source(&file, &stale, |raw| Ok(raw.clone())).unwrap();
        assert_eq!(parsed.value(), "stale", "the stale caller still parses for itself");
        let fresh = load(&memo, &file, "fresh");
        assert_eq!(fresh.value(), "fresh");
        assert!(!fresh.from_memo());
        assert_eq!(memo.stats().source_hits, 0);
    }

    #[test]
    fn a_source_loaded_before_invalidation_binds_without_memoizing() {
        let memo = Memo::default();
        let file = json(&identity("stale.yaml"));
        let stale = load(&memo, &file, "stale");
        memo.invalidate();
        memo.bind(&stale, "c", None, |text| Ok(text.len())).unwrap();
        let fresh = load(&memo, &file, "fresh!");
        assert_eq!(*memo.bind(&fresh, "c", None, |text| Ok(text.len())).unwrap().value(), 6);
        assert_eq!(memo.stats().bind_hits, 0);
    }

    #[test]
    fn memo_generation_advances_once_per_invalidation() {
        let runtime = DataRuntime::new();
        assert_eq!(runtime.memo_generation(), 0);
        runtime.invalidate_memo();
        assert_eq!(runtime.memo_generation(), 1);
        runtime.invalidate_memo();
        assert_eq!((runtime.memo_generation(), runtime.stats().invalidations), (2, 2));
    }

    #[test]
    fn a_failure_is_memoized_but_a_remote_deferral_is_not() {
        use crate::markdown::compose::expression::FileRefFailure;
        let memo = Memo::default();
        let file = identity("absent.json");
        for _ in 0..2 {
            assert!(memo.load_raw(&file, || Err(missing(FileRefFailure::NotFound))).is_err());
        }
        assert_eq!(memo.stats().reads, 1);

        let remote = DataSourceIdentity::Remote(Box::new(url::Url::parse("https://example.com/a.json").unwrap()));
        assert!(memo.load_raw(&remote, || Err(missing(FileRefFailure::RemoteNotEnabled))).is_err());
        let fetched = AtomicBool::new(false);
        let entry = memo
            .load_raw(&remote, || {
                fetched.store(true, Ordering::SeqCst);
                Ok("body".to_string())
            })
            .unwrap();
        assert!(fetched.load(Ordering::SeqCst), "the deferral was not remembered");
        assert_eq!(entry.value(), "body");

        let parsed = json(&identity("bad.json"));
        let raw = memo.load_raw(&parsed.identity, || Ok("{".to_string())).unwrap();
        for _ in 0..2 {
            let failed = memo.load_source(&parsed, &raw, |_| Err(missing(FileRefFailure::NotFound)));
            assert!(failed.is_err());
        }
        assert_eq!(memo.stats().source_hits, 1, "a parse failure is memoized too");
    }

    #[test]
    fn the_read_set_is_deduplicated_sorted_and_survives_invalidation() {
        let runtime = DataRuntime::new();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.md"), "").unwrap();
        runtime.record_local(&dir.path().join("b.md"));
        runtime.record_local(&dir.path().join("./b.md"));
        runtime.record_local(&dir.path().join("a-missing.md"));
        runtime.record_url(&url::Url::parse("https://example.com/x.json?v=1#frag").unwrap());
        runtime.invalidate_memo();
        let read_set = runtime.read_set();
        let canonical = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        assert_eq!(
            read_set,
            vec![
                biscuit_file::to_portable_string(&canonical.join("a-missing.md")),
                biscuit_file::to_portable_string(&canonical.join("b.md")),
                "https://example.com/x.json?v=1#frag".to_string(),
            ]
        );
        assert_eq!(runtime.stats().invalidations, 1);
    }

    #[test]
    fn clones_share_state_and_new_runtimes_do_not() {
        let runtime = DataRuntime::new();
        let clone = runtime.clone();
        clone.record_url(&url::Url::parse("https://example.com/").unwrap());
        assert_eq!(runtime.read_set().len(), 1);
        assert!(runtime.same_instance(&clone));
        assert!(!runtime.same_instance(&DataRuntime::new()));
        assert!(DataRuntime::new().read_set().is_empty());
    }

    #[test]
    fn a_contract_is_prepared_once_per_context_and_records_its_imports() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("types.yaml"), "kind: schema\ntypes:\n  Port: number\n").unwrap();
        let ctx = crate::markdown::compose::expression::ResolutionContext::new(biscuit_file::FileResolutionContext::new(dir.path()));
        let runtime = ctx.data.clone();
        for _ in 0..2 {
            runtime.prepare_contract("{ port: Port@./types.yaml }[]", &ctx).unwrap();
        }
        let stats = runtime.stats();
        assert_eq!((stats.contract_preparations, stats.contract_hits), (1, 1));
        let canonical = biscuit_file::canonicalize_simplified(&dir.path().join("types.yaml")).unwrap();
        assert_eq!(runtime.read_set(), vec![biscuit_file::to_portable_string(&canonical)]);
    }
}
