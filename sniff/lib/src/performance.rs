use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::Level;

thread_local! {
    static CURRENT_COLLECTOR: RefCell<Option<Arc<PerformanceCollector>>> = const { RefCell::new(None) };
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

#[derive(Debug, Default)]
pub(crate) struct PerformanceCollector {
    state: Mutex<CollectorState>,
}

impl PerformanceCollector {
    pub(crate) fn new_shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub(crate) fn snapshot(&self, total_duration: Duration) -> PerformanceReport {
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

    fn record_stage(&self, name: &str, duration: Duration) {
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

    fn increment_counter(&self, name: &str, delta: u64) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *state.counters.entry(name.to_string()).or_default() += delta;
    }
}

pub(crate) fn with_current_collector<T>(
    collector: Option<Arc<PerformanceCollector>>,
    f: impl FnOnce() -> T,
) -> T {
    let previous = CURRENT_COLLECTOR.with(|slot| slot.replace(collector));
    let result = f();
    CURRENT_COLLECTOR.with(|slot| {
        slot.replace(previous);
    });
    result
}

pub(crate) fn current_collector() -> Option<Arc<PerformanceCollector>> {
    CURRENT_COLLECTOR.with(|slot| slot.borrow().clone())
}

pub(crate) fn record_stage(name: impl Into<String>, duration: Duration) {
    let name = name.into();
    if let Some(collector) = current_collector() {
        collector.record_stage(&name, duration);
    }

    #[cfg(feature = "metrics")]
    {
        metrics::counter!("sniff_stage_calls_total", "stage" => name.clone()).increment(1);
        metrics::histogram!("sniff_stage_duration_ms", "stage" => name.clone())
            .record(duration_ms(duration));
    }
}

pub(crate) fn record_logged_stage(name: impl Into<String>, duration: Duration, level: Level) {
    let name = name.into();
    let elapsed_ms = duration_ms(duration);
    record_stage(name.clone(), duration);
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

pub(crate) fn increment_counter(name: impl Into<String>, delta: u64) {
    let name = name.into();
    if let Some(collector) = current_collector() {
        collector.increment_counter(&name, delta);
    }

    #[cfg(feature = "metrics")]
    {
        metrics::counter!("sniff_counter_total", "counter" => name).increment(delta);
    }
}

pub(crate) fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
