//! Structured performance collection: stage timings and work counters.
//!
//! Work counters are the primary acceptance evidence for Sniff's performance
//! work: one counter corresponds to one meaningful unit of Sniff-owned work
//! (a directory entry, a manifest parse, a status walk, a subprocess spawn),
//! so an optimization is judged by work removed rather than by wall time on a
//! noisy host. [`counters`] holds the stable names.
//!
//! ## Notes
//!
//! Recording is gated on [`is_collecting`], a process-wide count of installed
//! collectors. When nothing is collecting and the `metrics` feature is off,
//! [`increment_counter`], [`record_stage`], and [`StageTimer`] touch neither
//! the clock nor thread-local storage, so per-entry instrumentation costs a
//! single relaxed atomic load on the default path.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::Level;

pub mod counters;

/// Number of collectors currently installed anywhere in the process.
///
/// Read on every instrumentation call, so it is deliberately a plain relaxed
/// counter rather than a lock: a stale `true` costs one wasted thread-local
/// probe, and a stale `false` can only occur before a collector's own
/// installation is visible to the recording thread.
static ACTIVE_COLLECTORS: AtomicUsize = AtomicUsize::new(0);

/// Whether any performance data would be recorded by this call.
///
/// Callers that must set up instrumentation state — reading a clock, sampling
/// a length, formatting a name — should check this first so the default path
/// pays nothing.
#[inline]
pub fn is_collecting() -> bool {
    cfg!(feature = "metrics") || ACTIVE_COLLECTORS.load(Ordering::Relaxed) > 0
}

/// Whether the *calling thread* has a collector installed.
///
/// Thread-local buffers are only safe to write when a collector on this thread
/// will later drain them; otherwise the data would leak into whichever
/// collector next snapshots on this thread.
#[inline]
fn collector_installed() -> bool {
    CURRENT_COLLECTOR.with(|slot| slot.borrow().is_some())
}

thread_local! {
    static CURRENT_COLLECTOR: RefCell<Option<Arc<PerformanceCollector>>> = const { RefCell::new(None) };
    /// Thread-local buffer for stage timings. Each thread accumulates into its own
    /// HashMap, eliminating mutex contention during hot-path recording.
    static STAGE_BUFFER: RefCell<HashMap<&'static str, PerformanceStage>> = RefCell::new(HashMap::new());
    /// Thread-local buffer for counters. Each thread accumulates into its own
    /// HashMap, eliminating mutex contention during hot-path recording.
    static COUNTER_BUFFER: RefCell<HashMap<&'static str, u64>> = RefCell::new(HashMap::new());
}

/// Structured performance data collected during a single sniff run.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PerformanceReport {
    /// Total wall-clock time spent in the request.
    pub total_duration_ms: f64,
    /// Aggregated timings for instrumented stages.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub stages: BTreeMap<String, PerformanceStage>,
    /// Aggregated counters for hot-path activity and cache usage.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub counters: BTreeMap<String, u64>,
}

/// Aggregate timing statistics for a named stage.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PerformanceStage {
    /// Number of times the stage was observed.
    pub calls: u64,
    /// Sum of all observed durations for the stage.
    pub total_duration_ms: f64,
    /// Slowest single observation for the stage.
    pub max_duration_ms: f64,
    /// Most recent observed duration for the stage.
    pub last_duration_ms: f64,
}

#[derive(Debug, Default)]
struct CollectorState {
    stages: BTreeMap<String, PerformanceStage>,
    counters: BTreeMap<String, u64>,
}

/// Collects performance timings and counters during a sniff run.
///
/// Uses thread-local buffers for lock-free hot-path recording.
/// Call [`snapshot`](Self::snapshot) to merge buffers and produce
/// a [`PerformanceReport`].
#[derive(Debug, Default)]
pub struct PerformanceCollector {
    state: Mutex<CollectorState>,
}

impl PerformanceCollector {
    /// Creates a new collector wrapped in an [`Arc`] for sharing.
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Merges thread-local buffers and returns the current report.
    pub fn snapshot(&self, total_duration: Duration) -> PerformanceReport {
        // Merge thread-local buffers into the central state before snapshotting.
        self.merge_thread_local_buffers();

        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        PerformanceReport {
            total_duration_ms: duration_ms(total_duration),
            stages: state.stages.clone(),
            counters: state.counters.clone(),
        }
    }

    /// Drain all thread-local buffers into the central mutex-protected state.
    ///
    /// This is called once at snapshot time, after all worker threads have
    /// finished. It serializes the merge but keeps the hot path lock-free.
    fn merge_thread_local_buffers(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        // Merge stage buffers from the calling thread.
        STAGE_BUFFER.with(|buf| {
            let mut buf = buf.borrow_mut();
            for (name, incoming) in buf.drain() {
                let stage = state.stages.entry(name.to_string()).or_default();
                stage.calls += incoming.calls;
                stage.total_duration_ms += incoming.total_duration_ms;
                stage.max_duration_ms = stage.max_duration_ms.max(incoming.max_duration_ms);
                stage.last_duration_ms = incoming.last_duration_ms;
            }
        });

        // Merge counter buffers from the calling thread.
        COUNTER_BUFFER.with(|buf| {
            let mut buf = buf.borrow_mut();
            for (name, incoming) in buf.drain() {
                *state.counters.entry(name.to_string()).or_default() += incoming;
            }
        });
    }

    /// Drain the thread-local buffers of the **calling thread** into the
    /// central state.  This is intended to be called by worker threads
    /// (e.g. Rayon tasks) before they exit, so that their data is not lost
    /// when the thread is parked or reused by the pool.
    pub fn flush_thread_local(&self) {
        self.merge_thread_local_buffers();
    }
}

/// Runs `f` with the given collector installed as the thread-local current
/// collector, restoring the previous collector afterwards.
///
/// Also drains any stale data from the thread-local buffers before installing
/// the new collector, so that leftover data from a previous run does not leak
/// into the current collector's snapshot.
///
/// ## Notes
///
/// `f`'s recordings are flushed into `collector` before returning. Recording
/// writes to a thread-local buffer, and the only other things that drain it are
/// [`PerformanceCollector::snapshot`] — which merges the *snapshotting* thread —
/// and [`WorkerCollector`]'s drop. A caller that installs a collector on a
/// scoped thread and lets that thread exit has neither, so without this flush
/// every counter `f` recorded is silently discarded and the request reports
/// zero for work it demonstrably performed.
pub fn with_current_collector<T>(
    collector: Option<Arc<PerformanceCollector>>,
    f: impl FnOnce() -> T,
) -> T {
    // Drain stale thread-local buffers before switching collectors.
    STAGE_BUFFER.with(|buf| buf.borrow_mut().clear());
    COUNTER_BUFFER.with(|buf| buf.borrow_mut().clear());

    #[cfg(test)]
    let _test_read_guard = collector.is_some().then(CollectorTestReadGuard::acquire);

    // The active count must survive a panic in `f`, or every later request in
    // the process would keep paying for instrumentation that records nothing.
    let _active = collector.is_some().then(ActiveGuard::acquire);
    let installed = collector.clone();
    let previous = CURRENT_COLLECTOR.with(|slot| slot.replace(collector));
    let result = f();
    if let Some(installed) = installed.as_ref() {
        installed.flush_thread_local();
    }
    CURRENT_COLLECTOR.with(|slot| {
        slot.replace(previous);
    });
    result
}

/// Holds [`ACTIVE_COLLECTORS`] above zero for its lifetime.
struct ActiveGuard;

impl ActiveGuard {
    fn acquire() -> Self {
        ACTIVE_COLLECTORS.fetch_add(1, Ordering::Release);
        Self
    }
}

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        ACTIVE_COLLECTORS.fetch_sub(1, Ordering::Release);
    }
}

#[cfg(test)]
static COLLECTOR_TEST_LOCK: std::sync::RwLock<()> = std::sync::RwLock::new(());

#[cfg(test)]
thread_local! {
    static COLLECTOR_TEST_LOCK_HELD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` while no collector is installed anywhere in the test binary.
///
/// The "uncollected path" tests assert on the process-global
/// [`ACTIVE_COLLECTORS`] count, but that precondition is impossible to hold
/// while sibling tests install collectors on other threads. Taking the write
/// side of [`COLLECTOR_TEST_LOCK`] serializes against every
/// [`with_current_collector`] call so the assertion observes a true negative.
#[cfg(test)]
pub(crate) fn without_any_collector<T>(f: impl FnOnce() -> T) -> T {
    // Spin on try_write rather than queueing a blocking writer: a queued
    // writer makes later read acquisitions block, and collector tests spawn
    // scoped threads that call with_current_collector while their parent
    // holds a read — a blocked child plus a joining parent is a deadlock.
    let guard = loop {
        match COLLECTOR_TEST_LOCK.try_write() {
            Ok(guard) => break guard,
            Err(std::sync::TryLockError::Poisoned(poisoned)) => break poisoned.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => std::thread::yield_now(),
        }
    };
    let _guard = guard;
    f()
}

/// Read side of [`COLLECTOR_TEST_LOCK`], reentrant per thread because
/// production code nests [`with_current_collector`] calls.
#[cfg(test)]
struct CollectorTestReadGuard(Option<std::sync::RwLockReadGuard<'static, ()>>);

#[cfg(test)]
impl CollectorTestReadGuard {
    fn acquire() -> Self {
        let already_held = COLLECTOR_TEST_LOCK_HELD.with(|held| held.replace(true));
        if already_held {
            Self(None)
        } else {
            Self(Some(
                COLLECTOR_TEST_LOCK
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()),
            ))
        }
    }
}

#[cfg(test)]
impl Drop for CollectorTestReadGuard {
    fn drop(&mut self) {
        if self.0.is_some() {
            COLLECTOR_TEST_LOCK_HELD.with(|held| held.set(false));
        }
    }
}

/// Carries the spawning thread's collector into a worker thread.
///
/// Parallel walkers (`ignore::build_parallel`) and pool-backed workers build
/// their per-worker state on the *spawning* thread but run the work on another
/// thread, so [`current_collector`] is unavailable where the work happens. A
/// `WorkerCollector` is constructed with [`inherit`](Self::inherit) on the
/// spawning thread and [`activate`](Self::activate)d from the first callback
/// that runs on the worker thread.
///
/// ## Notes
///
/// Dropping the value flushes the worker thread's buffers into the collector.
/// It must therefore be owned by per-worker state that is dropped on the worker
/// thread — otherwise the worker's counts are lost when a pooled thread parks
/// instead of exiting.
pub struct WorkerCollector {
    collector: Option<Arc<PerformanceCollector>>,
    installed: bool,
}

impl WorkerCollector {
    /// Captures the calling thread's collector for later use on a worker thread.
    pub fn inherit() -> Self {
        Self {
            collector: current_collector(),
            installed: false,
        }
    }

    /// Installs the inherited collector on the calling thread, once.
    ///
    /// Call this from worker code before recording. Repeat calls are a single
    /// predictable branch.
    #[inline]
    pub fn activate(&mut self) {
        if !self.installed {
            self.install();
        }
    }

    #[cold]
    fn install(&mut self) {
        self.installed = true;
        let Some(collector) = self.collector.as_ref() else {
            return;
        };
        // A pooled thread may still hold buffers from an earlier request.
        STAGE_BUFFER.with(|buf| buf.borrow_mut().clear());
        COUNTER_BUFFER.with(|buf| buf.borrow_mut().clear());
        CURRENT_COLLECTOR.with(|slot| slot.replace(Some(Arc::clone(collector))));
    }
}

impl Drop for WorkerCollector {
    fn drop(&mut self) {
        if !self.installed {
            return;
        }
        if let Some(collector) = self.collector.as_ref() {
            collector.flush_thread_local();
        }
        CURRENT_COLLECTOR.with(|slot| {
            slot.replace(None);
        });
    }
}

/// Attributes work done inside a pooled worker to the spawning request.
///
/// Rayon threads park between requests rather than exiting, so work recorded
/// on one has nowhere to flush to and would silently read as zero — which is
/// worse than no counter, because a later phase could "prove" it removed work
/// that was merely never counted. `collector` comes from [`current_collector`]
/// on the spawning thread; hold the returned guard for the body of the
/// parallel closure.
///
/// ## Examples
///
/// ```ignore
/// let collector = performance::current_collector();
/// items
///     .par_iter()
///     .map(|item| {
///         let _worker = performance::pooled_worker(collector.as_ref());
///         probe(item)
///     })
///     .collect()
/// ```
///
/// ## Notes
///
/// The guard flushes on drop, costing one mutex acquisition per closure call,
/// so it is only worth taking around a body that performs a counted unit of
/// work — a file open, a subprocess, a repository open — never around a
/// trivial map. When nothing is collecting, `collector` is `None` and the
/// guard is inert.
#[must_use = "the guard must live for the body of the parallel closure"]
pub fn pooled_worker(collector: Option<&Arc<PerformanceCollector>>) -> PooledWorkerGuard {
    // Rayon runs some items on the *calling* thread, which already has the
    // request's collector and its live buffers. Installing over that would
    // clear buffers the caller has not snapshotted yet, so defer to whoever
    // installed first — they own the flush.
    let Some(collector) = collector.filter(|_| !collector_installed()) else {
        return PooledWorkerGuard(None);
    };
    let mut worker = WorkerCollector {
        collector: Some(Arc::clone(collector)),
        installed: false,
    };
    worker.activate();
    PooledWorkerGuard(Some(worker))
}

/// Flushes a pooled worker's buffers into the request's collector on drop.
///
/// See [`pooled_worker`].
// The payload is never read: it exists only so `WorkerCollector::drop` runs at
// the end of the guarded scope.
pub struct PooledWorkerGuard(#[allow(dead_code)] Option<WorkerCollector>);

/// A stage timer that reads no clock when nothing is collecting.
///
/// ## Examples
///
/// ```
/// use sniff::performance::StageTimer;
///
/// let timer = StageTimer::start("example.stage");
/// // ... work ...
/// timer.finish();
/// ```
#[must_use = "a StageTimer records nothing unless finished"]
pub struct StageTimer {
    name: &'static str,
    started: Option<Instant>,
}

impl StageTimer {
    /// Starts timing `name`, or does nothing if no collector is installed.
    #[inline]
    pub fn start(name: &'static str) -> Self {
        Self {
            name,
            started: is_collecting().then(Instant::now),
        }
    }

    /// Records the elapsed time against the stage.
    #[inline]
    pub fn finish(self) {
        if let Some(started) = self.started {
            record_stage(self.name, started.elapsed());
        }
    }
}

/// Returns the currently installed collector for this thread, if any.
pub fn current_collector() -> Option<Arc<PerformanceCollector>> {
    CURRENT_COLLECTOR.with(|slot| slot.borrow().clone())
}

fn record_stage_buffered(name: &'static str, duration: Duration) {
    let elapsed_ms = duration_ms(duration);
    STAGE_BUFFER.with(|buf| {
        let mut buf = buf.borrow_mut();
        let stage = buf.entry(name).or_default();
        stage.calls += 1;
        stage.total_duration_ms += elapsed_ms;
        stage.max_duration_ms = stage.max_duration_ms.max(elapsed_ms);
        stage.last_duration_ms = elapsed_ms;
    });
}

/// Record a performance stage with a static name (zero-allocation hot path).
///
/// For known constant stage names, this avoids per-call string allocation
/// and writes to a thread-local buffer instead of contending on a mutex.
pub fn record_stage(name: &'static str, duration: Duration) {
    if is_collecting() && collector_installed() {
        record_stage_buffered(name, duration);
    }

    #[cfg(feature = "metrics")]
    {
        metrics::counter!("sniff_stage_calls_total", "stage" => name).increment(1);
        metrics::histogram!("sniff_stage_duration_ms", "stage" => name)
            .record(duration_ms(duration));
    }
}

/// Record a performance stage with a dynamic name (may allocate).
///
/// Prefer [`record_stage`] with a `&'static str` for known constant names.
/// This variant is for computed / dynamic stage names only.
pub fn record_stage_dynamic(name: impl Into<String>, duration: Duration) {
    if !is_collecting() {
        return;
    }
    let name = name.into();
    if let Some(collector) = current_collector() {
        // For dynamic names, fall back to the mutex path via a temporary static
        // allocation. This is rare; most callers use record_stage with static names.
        collector.record_stage_dynamic(&name, duration);
    }

    #[cfg(feature = "metrics")]
    {
        metrics::counter!("sniff_stage_calls_total", "stage" => name.clone()).increment(1);
        metrics::histogram!("sniff_stage_duration_ms", "stage" => name)
            .record(duration_ms(duration));
    }
}

/// Record a performance stage and emit a tracing event at the given level.
pub fn record_logged_stage(name: &'static str, duration: Duration, level: Level) {
    let elapsed_ms = duration_ms(duration);
    record_stage(name, duration);
    match level {
        Level::ERROR => {
            tracing::event!(
                Level::ERROR,
                stage = %name,
                duration_ms = elapsed_ms,
                "performance stage complete"
            );
        }
        Level::WARN => {
            tracing::event!(
                Level::WARN,
                stage = %name,
                duration_ms = elapsed_ms,
                "performance stage complete"
            );
        }
        Level::INFO => {
            tracing::event!(
                Level::INFO,
                stage = %name,
                duration_ms = elapsed_ms,
                "performance stage complete"
            );
        }
        Level::DEBUG => {
            tracing::event!(
                Level::DEBUG,
                stage = %name,
                duration_ms = elapsed_ms,
                "performance stage complete"
            );
        }
        Level::TRACE => {
            tracing::event!(
                Level::TRACE,
                stage = %name,
                duration_ms = elapsed_ms,
                "performance stage complete"
            );
        }
    }
}

/// Increment a counter with a static name (zero-allocation hot path).
///
/// For known constant counter names, this avoids per-call string allocation
/// and writes to a thread-local buffer instead of contending on a mutex.
pub fn increment_counter(name: &'static str, delta: u64) {
    if is_collecting() && collector_installed() {
        COUNTER_BUFFER.with(|buf| {
            *buf.borrow_mut().entry(name).or_default() += delta;
        });
    }

    #[cfg(feature = "metrics")]
    {
        metrics::counter!("sniff_counter_total", "counter" => name).increment(delta);
    }
}

/// Increment a counter with a dynamic name (may allocate).
///
/// Prefer [`increment_counter`] with a `&'static str` for known constant names.
/// This variant is for computed / dynamic counter names only.
pub fn increment_counter_dynamic(name: impl Into<String>, delta: u64) {
    if !is_collecting() {
        return;
    }
    let name = name.into();
    if let Some(collector) = current_collector() {
        collector.increment_counter_dynamic(&name, delta);
    }

    #[cfg(feature = "metrics")]
    {
        metrics::counter!("sniff_counter_total", "counter" => name).increment(delta);
    }
}

pub fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// Work-count assertions for in-crate tests.
///
/// Built on the ordinary scoped-collector API, so no production type carries
/// test-only state and nothing is cached process-globally: each [`measure`]
/// call owns a private collector that dies with the call.
#[cfg(test)]
pub(crate) mod testing {
    use super::{Duration, PerformanceCollector, PerformanceReport, with_current_collector};

    /// Runs `f` under a private collector and returns its value with the
    /// work it performed.
    ///
    /// ## Notes
    ///
    /// Work performed on threads `f` spawns is included only when those
    /// threads install the collector and flush before exiting — which is what
    /// [`super::WorkerCollector`] exists to guarantee. A count that is lower
    /// than expected therefore indicates either less work or a worker that
    /// failed to flush; [`WorkCounts::total`] over a serial equivalent
    /// distinguishes the two.
    pub(crate) fn measure<T>(f: impl FnOnce() -> T) -> (T, WorkCounts) {
        let collector = PerformanceCollector::new_shared();
        let value = with_current_collector(Some(collector.clone()), f);
        let report = collector.snapshot(Duration::ZERO);
        (value, WorkCounts { report })
    }

    /// Counts recorded by one [`measure`] call.
    pub(crate) struct WorkCounts {
        report: PerformanceReport,
    }

    impl WorkCounts {
        /// Value of `name`, or zero when the counter was never incremented.
        pub(crate) fn get(&self, name: &str) -> u64 {
            self.report.counters.get(name).copied().unwrap_or(0)
        }

        /// Every recorded counter, for diagnosing an unexpected assertion.
        pub(crate) fn all(&self) -> &std::collections::BTreeMap<String, u64> {
            &self.report.counters
        }

        /// Whether any stage timing was recorded.
        pub(crate) fn recorded_any_stage(&self) -> bool {
            !self.report.stages.is_empty()
        }

        /// Whether `name` was observed at least once.
        pub(crate) fn recorded_stage(&self, name: &str) -> bool {
            self.report.stages.contains_key(name)
        }

        /// Every recorded stage name, for diagnosing an unexpected assertion.
        pub(crate) fn stage_names(&self) -> Vec<&str> {
            self.report.stages.keys().map(String::as_str).collect()
        }
    }
}

impl PerformanceCollector {
    fn record_stage_dynamic(&self, name: &str, duration: Duration) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let stage = state.stages.entry(name.to_string()).or_default();
        let elapsed_ms = duration_ms(duration);
        stage.calls += 1;
        stage.total_duration_ms += elapsed_ms;
        stage.max_duration_ms = stage.max_duration_ms.max(elapsed_ms);
        stage.last_duration_ms = elapsed_ms;
    }

    fn increment_counter_dynamic(&self, name: &str, delta: u64) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *state.counters.entry(name.to_string()).or_default() += delta;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COUNTER: &str = "test.performance.unit";
    const STAGE: &str = "test.performance.stage";

    #[test]
    fn nothing_is_collecting_by_default() {
        without_any_collector(|| {
            assert!(
                !is_collecting(),
                "the default path must not pay for instrumentation"
            );
        });
    }

    #[test]
    fn recording_outside_a_collector_is_dropped_not_buffered() {
        increment_counter(COUNTER, 9);
        StageTimer::start(STAGE).finish();

        // If the calls above had buffered into thread-local storage, this
        // collector would adopt work it never scoped.
        let ((), counts) = testing::measure(|| {});
        assert_eq!(counts.get(COUNTER), 0);
        assert!(!counts.recorded_any_stage());
    }

    #[test]
    fn stage_timer_reads_no_clock_when_inactive() {
        without_any_collector(|| {
            assert!(StageTimer::start(STAGE).started.is_none());
        });
        testing::measure(|| {
            assert!(StageTimer::start(STAGE).started.is_some());
        });
    }

    #[test]
    fn measure_scopes_counts_to_its_own_call() {
        let ((), first) = testing::measure(|| increment_counter(COUNTER, 3));
        assert_eq!(first.get(COUNTER), 3);

        let ((), second) = testing::measure(|| increment_counter(COUNTER, 1));
        assert_eq!(second.get(COUNTER), 1, "counts must not accumulate across calls");
    }

    #[test]
    fn pooled_worker_attributes_thread_work_to_the_request() {
        let ((), counts) = testing::measure(|| {
            let collector = current_collector();
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let collector = collector.clone();
                    std::thread::spawn(move || {
                        let _worker = pooled_worker(collector.as_ref());
                        increment_counter(COUNTER, 5);
                    })
                })
                .collect();
            for handle in handles {
                handle.join().expect("worker");
            }
        });

        assert_eq!(
            counts.get(COUNTER),
            20,
            "each of 4 workers must contribute exactly once"
        );
    }

    #[test]
    fn pooled_worker_on_the_calling_thread_does_not_discard_pending_counts() {
        // Rayon runs some items inline on the caller. Taking a guard there
        // must not clear the caller's not-yet-snapshotted buffer.
        let ((), counts) = testing::measure(|| {
            increment_counter(COUNTER, 2);
            let collector = current_collector();
            let _worker = pooled_worker(collector.as_ref());
            increment_counter(COUNTER, 3);
        });

        assert_eq!(counts.get(COUNTER), 5);
    }
}
