//! The request-wide `load_data` runtime: memo, read set, and counters.
//!
//! One [`DataRuntime`] spans a whole command. It is held on
//! `ComposeOptions` and cloned (sharing its state) into every expression
//! context the command builds, so the reference check, pre-flight, the
//! lifecycle, composition, and nested documents read through one memo and
//! report into one read set.
//!
//! The memo has two levels. A parsed source is keyed by its resolved identity
//! alone, and a bound result by that identity and the contract key, so two
//! contracts over one file read and parse it once and bind it twice. Each
//! entry is single-flight: the first caller loads, and concurrent callers for
//! the same key wait for its result. The map lock is never held while a
//! loader runs.
//!
//! [`DataRuntime::invalidate_memo`] ends reuse after a command ran: it starts
//! a new generation with empty maps and leaves the read set alone. A load
//! already in flight finishes into its old entry for its own caller, and the
//! generation it started in keeps it from populating the new maps.

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

/// A data document read, decoded, and parsed, before any contract applies.
#[derive(Debug, Clone)]
pub struct LoadedSource {
    /// The parsed content.
    pub content: SourceContent,
    /// The format it was parsed as.
    pub format: DataFormat,
    /// The BOM-stripped text, local or fetched, for locating failures.
    pub text: Arc<str>,
    /// The size of the document as read (local bytes or fetched text), in
    /// bytes.
    pub bytes: usize,
}

/// What a source parsed into.
///
/// CSV and TSV stay a table because their row shape is chosen per contract
/// when binding, while the source memo is keyed by identity alone.
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
    /// Source loaders run (each one reads a file or a fetched body).
    pub reads: usize,
    /// Documents parsed, as reported by the loaders.
    pub parses: usize,
    /// Binders run (each one validates a source against a contract).
    pub binds: usize,
    /// Source lookups answered from the memo.
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
type BindKey = (DataSourceIdentity, String);
/// Selects one of a generation's maps.
type MapOf<S, B, K, T> = fn(&mut Generation<S, B>) -> &mut HashMap<K, Slot<T>>;

struct Generation<S, B> {
    number: u64,
    sources: HashMap<DataSourceIdentity, Slot<S>>,
    bindings: HashMap<BindKey, Slot<B>>,
    contracts: HashMap<String, Slot<PreparedContract>>,
}

impl<S, B> Generation<S, B> {
    fn new(number: u64) -> Self {
        Self { number, sources: HashMap::new(), bindings: HashMap::new(), contracts: HashMap::new() }
    }
}

/// A parsed source as one caller received it, with the generation it was
/// loaded in.
pub struct SourceEntry<S> {
    identity: DataSourceIdentity,
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
        &self.identity
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

/// The two-level single-flight memo, generic over the parsed source `S` and
/// the bound result `B`.
pub(crate) struct DataMemo<S, B> {
    state: Mutex<Generation<S, B>>,
    counters: Counters,
}

impl<S, B> Default for DataMemo<S, B> {
    fn default() -> Self {
        Self { state: Mutex::new(Generation::new(0)), counters: Counters::default() }
    }
}

impl<S, B> DataMemo<S, B> {
    fn lock(&self) -> MutexGuard<'_, Generation<S, B>> {
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

    /// Starts a new generation with empty maps.
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
    /// current generation; otherwise `init` runs for this caller alone. A
    /// deferral outcome is removed from the map so a later caller tries again.
    fn single_flight<K: Hash + Eq + Clone, T>(
        &self,
        map: MapOf<S, B, K, T>,
        key: K,
        expected: Option<u64>,
        init: impl FnOnce() -> Result<T, Arc<DataError>>,
    ) -> (Outcome<T>, u64, bool) {
        let (slot, generation) = {
            let mut state = self.lock();
            let generation = state.number;
            if expected.is_some_and(|expected| expected != generation) {
                drop(state);
                return (init().map(Arc::new), generation, true);
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

    /// The parsed source for `identity`, running `loader` only when no entry
    /// of the current generation holds it.
    ///
    /// ## Errors
    ///
    /// The loader's error, memoized unless it is a deferral.
    pub(crate) fn load_source(
        &self,
        identity: &DataSourceIdentity,
        loader: impl FnOnce() -> Result<S, Arc<DataError>>,
    ) -> Result<SourceEntry<S>, Arc<DataError>> {
        let (outcome, generation, ran) = self.single_flight(|g| &mut g.sources, identity.clone(), None, || {
            bump(&self.counters.reads);
            loader()
        });
        if !ran {
            bump(&self.counters.source_hits);
        }
        outcome.map(|value| SourceEntry { identity: identity.clone(), generation, value, from_memo: !ran })
    }

    /// `source` bound to the contract `contract_key`, running `binder` only
    /// when no entry holds it. A source loaded before the last invalidation
    /// is bound for this caller alone and never memoized: its bytes may be
    /// stale.
    ///
    /// ## Errors
    ///
    /// The binder's error, memoized unless it is a deferral.
    pub(crate) fn bind(
        &self,
        source: &SourceEntry<S>,
        contract_key: &str,
        binder: impl FnOnce(&S) -> Result<B, Arc<DataError>>,
    ) -> Result<BindEntry<B>, Arc<DataError>> {
        let key = (source.identity.clone(), contract_key.to_string());
        let (outcome, _, ran) = self.single_flight(|g| &mut g.bindings, key, Some(source.generation), || {
            bump(&self.counters.binds);
            binder(&source.value)
        });
        if !ran {
            bump(&self.counters.bind_hits);
        }
        outcome.map(|value| BindEntry { value, from_memo: !ran })
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
    memo: DataMemo<LoadedSource, LoadedData>,
    read_set: Mutex<BTreeSet<String>>,
}

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

    /// Empties the memo after a command ran; see the module documentation.
    pub(crate) fn invalidate_memo(&self) {
        self.inner.memo.invalidate();
    }

    /// Counts one parse; called by a source loader.
    pub(crate) fn note_parse(&self) {
        self.inner.memo.note_parse();
    }

    /// The parsed source for `identity`; see [`DataMemo::load_source`]. The
    /// caller records the read before calling, so a failed read is recorded
    /// too.
    pub(crate) fn load_source(
        &self,
        identity: &DataSourceIdentity,
        loader: impl FnOnce() -> Result<LoadedSource, Arc<DataError>>,
    ) -> Result<SourceEntry<LoadedSource>, Arc<DataError>> {
        self.inner.memo.load_source(identity, loader)
    }

    /// `source` bound to the contract `contract_key`; see [`DataMemo::bind`].
    pub(crate) fn bind(
        &self,
        source: &SourceEntry<LoadedSource>,
        contract_key: &str,
        binder: impl FnOnce(&LoadedSource) -> Result<LoadedData, Arc<DataError>>,
    ) -> Result<BindEntry<LoadedData>, Arc<DataError>> {
        self.inner.memo.bind(source, contract_key, binder)
    }

    /// The contract `written` describes in the document context `document`,
    /// prepared once per context identity and text. Every call records the
    /// contract's imports, so a memo hit keeps its dependencies.
    ///
    /// ## Errors
    ///
    /// As [`prepare_contract`](super::contract::prepare_contract).
    pub(crate) fn prepare_contract(
        &self,
        written: &str,
        document: &biscuit_file::FileResolutionContext,
    ) -> Result<Arc<PreparedContract>, Arc<DataError>> {
        let identity = crate::markdown::compose::file_resolution_context_identity(document);
        let key = format!("{identity:016x}\0{written}");
        let contract = self
            .inner
            .memo
            .contract(key, || super::contract::prepare_contract(written, document))?;
        for import in &contract.imports {
            self.record_local(import);
        }
        Ok(contract)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::sync::atomic::AtomicBool;

    fn identity(name: &str) -> DataSourceIdentity {
        DataSourceIdentity::Local(PathBuf::from(format!("/data/{name}")))
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
        let memo = DataMemo::<String, usize>::default();
        let file = identity("hosts.yaml");
        for contract in ["a", "b", "a"] {
            let source = memo.load_source(&file, || Ok("hosts".to_string())).unwrap();
            let bound = memo.bind(&source, contract, |text| Ok(text.len() + contract.len())).unwrap();
            assert_eq!(*bound.value(), 6);
        }
        let stats = memo.stats();
        assert_eq!((stats.reads, stats.source_hits), (1, 2));
        assert_eq!((stats.binds, stats.bind_hits), (2, 1));
    }

    #[test]
    fn each_level_says_whether_the_memo_answered() {
        let memo = DataMemo::<String, usize>::default();
        let file = identity("flags.yaml");
        let first = memo.load_source(&file, || Ok("x".to_string())).unwrap();
        let first_bind = memo.bind(&first, "c", |text| Ok(text.len())).unwrap();
        assert!(!first.from_memo() && !first_bind.from_memo());
        let second = memo.load_source(&file, || Ok("x".to_string())).unwrap();
        let second_bind = memo.bind(&second, "c", |text| Ok(text.len())).unwrap();
        assert!(second.from_memo() && second_bind.from_memo());
    }

    #[test]
    fn concurrent_callers_share_one_load() {
        let memo = Arc::new(DataMemo::<String, usize>::default());
        let file = identity("shared.yaml");
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let first = {
            let (memo, file, entered, release) = (memo.clone(), file.clone(), entered.clone(), release.clone());
            std::thread::spawn(move || {
                memo.load_source(&file, || {
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
            std::thread::spawn(move || {
                memo.load_source(&file, || Ok("second loader".to_string())).map(|entry| entry.value().clone())
            })
        };
        release.wait();
        assert_eq!(first.join().unwrap().unwrap(), "loaded");
        assert_eq!(second.join().unwrap().unwrap(), "loaded");
        assert_eq!(memo.stats().reads, 1);
    }

    #[test]
    fn invalidation_reads_again_and_a_late_old_load_never_populates_the_new_maps() {
        let memo = Arc::new(DataMemo::<String, usize>::default());
        let file = identity("regenerated.yaml");
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let old = {
            let (memo, file, entered, release) = (memo.clone(), file.clone(), entered.clone(), release.clone());
            std::thread::spawn(move || {
                let source = memo
                    .load_source(&file, || {
                        entered.wait();
                        release.wait();
                        Ok("old".to_string())
                    })
                    .unwrap();
                *memo.bind(&source, "contract", |text| Ok(text.len())).unwrap().value()
            })
        };
        entered.wait();
        memo.invalidate();
        release.wait();
        assert_eq!(old.join().unwrap(), 3, "the in-flight caller still gets its own result");

        let source = memo.load_source(&file, || Ok("newer".to_string())).unwrap();
        assert_eq!(source.value(), "newer", "the old load is not reused after invalidation");
        let bound = memo.bind(&source, "contract", |text| Ok(text.len())).unwrap();
        assert_eq!(*bound.value(), 5, "the old binding did not populate the new generation");
        let stats = memo.stats();
        assert_eq!((stats.reads, stats.binds, stats.invalidations), (2, 2, 1));
    }

    #[test]
    fn a_source_loaded_before_invalidation_binds_without_memoizing() {
        let memo = DataMemo::<String, usize>::default();
        let file = identity("stale.yaml");
        let stale = memo.load_source(&file, || Ok("stale".to_string())).unwrap();
        memo.invalidate();
        memo.bind(&stale, "c", |text| Ok(text.len())).unwrap();
        let fresh = memo.load_source(&file, || Ok("fresh!".to_string())).unwrap();
        assert_eq!(*memo.bind(&fresh, "c", |text| Ok(text.len())).unwrap().value(), 6);
        assert_eq!(memo.stats().bind_hits, 0);
    }

    #[test]
    fn a_failure_is_memoized_but_a_remote_deferral_is_not() {
        use crate::markdown::compose::expression::FileRefFailure;
        let memo = DataMemo::<String, usize>::default();
        let file = identity("absent.json");
        for _ in 0..2 {
            assert!(memo.load_source(&file, || Err(missing(FileRefFailure::NotFound))).is_err());
        }
        assert_eq!(memo.stats().reads, 1);

        let remote = DataSourceIdentity::Remote(Box::new(url::Url::parse("https://example.com/a.json").unwrap()));
        assert!(memo.load_source(&remote, || Err(missing(FileRefFailure::RemoteNotEnabled))).is_err());
        let fetched = AtomicBool::new(false);
        let entry = memo
            .load_source(&remote, || {
                fetched.store(true, Ordering::SeqCst);
                Ok("body".to_string())
            })
            .unwrap();
        assert!(fetched.load(Ordering::SeqCst), "the deferral was not remembered");
        assert_eq!(entry.value(), "body");
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
        let document = biscuit_file::FileResolutionContext::new(dir.path());
        let runtime = DataRuntime::new();
        for _ in 0..2 {
            runtime.prepare_contract("{ port: Port@./types.yaml }[]", &document).unwrap();
        }
        let stats = runtime.stats();
        assert_eq!((stats.contract_preparations, stats.contract_hits), (1, 1));
        let canonical = biscuit_file::canonicalize_simplified(&dir.path().join("types.yaml")).unwrap();
        assert_eq!(runtime.read_set(), vec![biscuit_file::to_portable_string(&canonical)]);
    }
}
