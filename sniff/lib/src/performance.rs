use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::Level;

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

    fn record_stage(&self, name: &'static str, duration: Duration) {
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

    fn increment_counter(&self, name: &'static str, delta: u64) {
        COUNTER_BUFFER.with(|buf| {
            *buf.borrow_mut().entry(name).or_default() += delta;
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
pub fn with_current_collector<T>(
    collector: Option<Arc<PerformanceCollector>>,
    f: impl FnOnce() -> T,
) -> T {
    // Drain stale thread-local buffers before switching collectors.
    STAGE_BUFFER.with(|buf| buf.borrow_mut().clear());
    COUNTER_BUFFER.with(|buf| buf.borrow_mut().clear());

    let previous = CURRENT_COLLECTOR.with(|slot| slot.replace(collector));
    let result = f();
    CURRENT_COLLECTOR.with(|slot| {
        slot.replace(previous);
    });
    result
}

/// Returns the currently installed collector for this thread, if any.
pub fn current_collector() -> Option<Arc<PerformanceCollector>> {
    CURRENT_COLLECTOR.with(|slot| slot.borrow().clone())
}

/// Record a performance stage with a static name (zero-allocation hot path).
///
/// For known constant stage names, this avoids per-call string allocation
/// and writes to a thread-local buffer instead of contending on a mutex.
pub fn record_stage(name: &'static str, duration: Duration) {
    if let Some(collector) = current_collector() {
        collector.record_stage(name, duration);
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
    if let Some(collector) = current_collector() {
        collector.increment_counter(name, delta);
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
