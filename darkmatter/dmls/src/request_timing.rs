//! Per-message timing for the dispatch loop: the slow-request log.
//!
//! The router starts a [`RequestTimings`] when it dequeues a request or
//! notification, the overlay build and the provider chain record their stage
//! times into it, and the router emits exactly one [`CompletionEvent`] when
//! the message finishes. At or above [`SLOW_THRESHOLD`] the event is a `warn`
//! (`slow request` / `slow notification`); below it, a `debug`
//! (`request complete` / `notification complete`).
//!
//! A request is measured from dequeue to sending its response; a notification
//! has no response and is measured through its handler's completion. Work the
//! handler hands to another thread (debounced diagnostics publishing, the
//! startup file-list walk, a trigger-registry preparation it queued) is not
//! part of a notification's figure; a request that waits for such work
//! includes the wait, and records it as a [`Wait`].
//!
//! `trigger_registry_wait_ms` is the time a request spent parked on trigger
//! registry jobs: queueing and preparation together, summed over every
//! interval it was parked (a request requeued behind a stale job parks
//! again). Each interval ends when the request resumes or is answered
//! (cancellation, shutdown), so handler work and index waits between them
//! are not part of it. The field is present only for a request that parked.
//!
//! ## Stage nesting
//!
//! Stage figures are elapsed times, not a breakdown of the total:
//!
//! - `overlay_build_ms` contains `schema_assembly_ms`, which contains
//!   `trigger_registry_ms`. A schema-trigger file being authored times its
//!   registry lookup directly under the overlay build.
//! - `provider_chain_ms` is a sibling of the overlay build.
//!
//! A stage that runs more than once in a message (a notification that
//! refreshes every open document) reports the sum of its runs. A stage that
//! did not run is omitted from the event.
//!
//! The event carries the method, document URI, request id, and outcome. It
//! never carries source text, frontmatter values, or protocol payloads.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::clock::{Clock, SystemClock};

/// The elapsed time at which a completion event becomes a `warn`.
pub const SLOW_THRESHOLD: Duration = Duration::from_millis(500);

/// A measured part of a message's handling. See the module docs for nesting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    OverlayBuild,
    SchemaAssembly,
    TriggerRegistry,
    ProviderChain,
}

impl Stage {
    const ALL: [Stage; 4] = [
        Stage::OverlayBuild,
        Stage::SchemaAssembly,
        Stage::TriggerRegistry,
        Stage::ProviderChain,
    ];

    fn index(self) -> usize {
        self as usize
    }
}

/// A prerequisite a request can block on before its providers run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wait {
    FileList,
    /// The index build: the index store revalidated in this process.
    Index,
    /// Its document's trigger registry, prepared off the dispatch thread.
    TriggerRegistry,
}

/// How a message's handling ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Answered with a result (or, for a notification, handled).
    Ok,
    /// Answered with an error response.
    Error,
    /// The handler panicked and the dispatch boundary caught it.
    Panic,
    /// Answered `RequestCanceled`.
    Cancelled,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Ok => "ok",
            Outcome::Error => "error",
            Outcome::Panic => "panic",
            Outcome::Cancelled => "cancelled",
        }
    }
}

/// The timer and stage record for one dispatched message.
///
/// Shared by reference through the overlay build and provider calls on the
/// dispatch thread; interior mutability lets those `&self` paths record.
pub struct RequestTimings {
    clock: Arc<dyn Clock>,
    started: Instant,
    stages: Mutex<[Option<Duration>; 4]>,
    waited_for_file_list: AtomicBool,
    waited_for_index: AtomicBool,
    waited_for_trigger_registry: AtomicBool,
    registry_wait: Mutex<RegistryWait>,
}

/// The parked intervals of [`RequestTimings::begin_registry_wait`].
#[derive(Default)]
struct RegistryWait {
    /// Closed intervals, summed.
    total: Duration,
    /// The start of the interval in progress.
    parked_since: Option<Instant>,
}

impl RequestTimings {
    /// Starts timing a message now, on `clock`.
    pub fn start(clock: Arc<dyn Clock>) -> Self {
        let started = clock.now();
        Self {
            clock,
            started,
            stages: Mutex::new([None; 4]),
            waited_for_file_list: AtomicBool::new(false),
            waited_for_index: AtomicBool::new(false),
            waited_for_trigger_registry: AtomicBool::new(false),
            registry_wait: Mutex::new(RegistryWait::default()),
        }
    }

    /// A record nobody reads, for an overlay build outside the dispatch loop.
    pub fn untimed() -> Self {
        Self::start(Arc::new(SystemClock))
    }

    /// Runs `body`, adding its elapsed time to `stage`.
    pub fn time<R>(&self, stage: Stage, body: impl FnOnce() -> R) -> R {
        let start = self.clock.now();
        let result = body();
        let elapsed = self.clock.now().saturating_duration_since(start);
        let mut stages = self.stages.lock().expect("request timings poisoned");
        let slot = &mut stages[stage.index()];
        *slot = Some(slot.unwrap_or_default() + elapsed);
        result
    }

    /// Records that the message blocked on `wait`.
    pub fn record_wait(&self, wait: Wait) {
        let flag = match wait {
            Wait::FileList => &self.waited_for_file_list,
            Wait::Index => &self.waited_for_index,
            Wait::TriggerRegistry => &self.waited_for_trigger_registry,
        };
        flag.store(true, Ordering::Relaxed);
    }

    /// Records [`Wait::TriggerRegistry`] and starts a parked interval, unless
    /// one is already in progress.
    pub fn begin_registry_wait(&self) {
        self.record_wait(Wait::TriggerRegistry);
        let mut wait = self.registry_wait.lock().expect("request timings poisoned");
        if wait.parked_since.is_none() {
            wait.parked_since = Some(self.clock.now());
        }
    }

    /// Ends the parked interval in progress, if any, adding it to the total.
    pub fn end_registry_wait(&self) {
        let now = self.clock.now();
        let mut wait = self.registry_wait.lock().expect("request timings poisoned");
        if let Some(since) = wait.parked_since.take() {
            wait.total += now.saturating_duration_since(since);
        }
    }

    /// The summed parked time on trigger registry jobs, including an
    /// interval still in progress, or `None` when the message never parked.
    pub fn registry_wait(&self) -> Option<Duration> {
        if !self.waited(Wait::TriggerRegistry) {
            return None;
        }
        let wait = self.registry_wait.lock().expect("request timings poisoned");
        let open = wait
            .parked_since
            .map(|since| self.clock.now().saturating_duration_since(since))
            .unwrap_or_default();
        Some(wait.total + open)
    }

    /// The time spent in `stage`, or `None` when it did not run.
    pub fn stage(&self, stage: Stage) -> Option<Duration> {
        self.stages.lock().expect("request timings poisoned")[stage.index()]
    }

    pub fn waited(&self, wait: Wait) -> bool {
        match wait {
            Wait::FileList => self.waited_for_file_list.load(Ordering::Relaxed),
            Wait::Index => self.waited_for_index.load(Ordering::Relaxed),
            Wait::TriggerRegistry => self.waited_for_trigger_registry.load(Ordering::Relaxed),
        }
    }

    /// Time since [`RequestTimings::start`].
    pub fn elapsed(&self) -> Duration {
        self.clock.now().saturating_duration_since(self.started)
    }
}

impl std::fmt::Debug for RequestTimings {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = formatter.debug_struct("RequestTimings");
        for stage in Stage::ALL {
            debug.field(&format!("{stage:?}"), &self.stage(stage));
        }
        debug
            .field("waited_for_file_list", &self.waited(Wait::FileList))
            .field("waited_for_index", &self.waited(Wait::Index))
            .field("waited_for_trigger_registry", &self.waited(Wait::TriggerRegistry))
            .field("registry_wait", &self.registry_wait())
            .finish()
    }
}

/// What finished: a request (with its id) or a notification.
#[derive(Clone, Copy, Debug)]
pub enum MessageKind<'a> {
    Request { id: &'a lsp_server::RequestId },
    Notification,
}

/// One message's completion, emitted once per terminal path.
#[derive(Clone, Copy, Debug)]
pub struct CompletionEvent<'a> {
    pub kind: MessageKind<'a>,
    pub method: &'a str,
    /// The document the message names; `None` omits the field.
    pub uri: Option<&'a str>,
    pub outcome: Outcome,
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

macro_rules! completion_event {
    ($level:expr, $message:literal, $completion:expr, $id:expr, $elapsed:expr, $timings:expr) => {
        tracing::event!(
            $level,
            method = $completion.method,
            uri = $completion.uri,
            id = $id,
            outcome = $completion.outcome.as_str(),
            cancelled = ($completion.outcome == Outcome::Cancelled).then_some(true),
            elapsed_ms = millis($elapsed),
            overlay_build_ms = $timings.stage(Stage::OverlayBuild).map(millis),
            schema_assembly_ms = $timings.stage(Stage::SchemaAssembly).map(millis),
            trigger_registry_ms = $timings.stage(Stage::TriggerRegistry).map(millis),
            provider_chain_ms = $timings.stage(Stage::ProviderChain).map(millis),
            waited_for_file_list = $timings.waited(Wait::FileList),
            waited_for_index = $timings.waited(Wait::Index),
            waited_for_trigger_registry = $timings.waited(Wait::TriggerRegistry),
            trigger_registry_wait_ms = $timings.registry_wait().map(millis),
            $message
        )
    };
}

impl CompletionEvent<'_> {
    /// Emits the completion event, its level and text chosen by the elapsed
    /// time since `timings` started.
    pub fn emit(&self, timings: &RequestTimings) {
        use tracing::Level;
        let elapsed = timings.elapsed();
        let slow = elapsed >= SLOW_THRESHOLD;
        match self.kind {
            MessageKind::Request { id } => {
                let id = Some(tracing::field::display(id));
                if slow {
                    completion_event!(Level::WARN, "slow request", self, id, elapsed, timings);
                } else {
                    completion_event!(Level::DEBUG, "request complete", self, id, elapsed, timings);
                }
            }
            MessageKind::Notification => {
                let id: Option<tracing::field::DisplayValue<&str>> = None;
                if slow {
                    completion_event!(Level::WARN, "slow notification", self, id, elapsed, timings);
                } else {
                    completion_event!(
                        Level::DEBUG,
                        "notification complete",
                        self,
                        id,
                        elapsed,
                        timings
                    );
                }
            }
        }
    }
}

/// The `textDocument.uri` a message's params name, if any.
pub fn document_uri(params: &serde_json::Value) -> Option<String> {
    params.get("textDocument")?.get("uri")?.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::ManualClock;
    use crate::log_capture::CapturedLogs;

    fn manual() -> Arc<ManualClock> {
        Arc::new(ManualClock::new())
    }

    fn request<'a>(id: &'a lsp_server::RequestId, uri: Option<&'a str>) -> CompletionEvent<'a> {
        CompletionEvent {
            kind: MessageKind::Request { id },
            method: "textDocument/hover",
            uri,
            outcome: Outcome::Ok,
        }
    }

    /// Emits one hover completion after `elapsed` on a manual clock.
    fn emit_after(elapsed: Duration) -> Vec<String> {
        let clock = manual();
        let timings = RequestTimings::start(clock.clone());
        clock.advance(elapsed);
        let id = lsp_server::RequestId::from(7);
        let ((), logs) = CapturedLogs::scoped(|| {
            request(&id, Some("file:///w/a.md")).emit(&timings);
        });
        logs.lines()
    }

    #[test]
    fn below_the_threshold_is_one_debug_request_complete() {
        let lines = emit_after(Duration::from_millis(499));
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with("DEBUG "), "{lines:?}");
        assert!(lines[0].contains("request complete"), "{lines:?}");
        assert!(lines[0].contains("elapsed_ms=499"), "{lines:?}");
        assert!(!lines[0].contains("slow"), "{lines:?}");
    }

    #[test]
    fn exactly_at_the_threshold_is_one_warn_slow_request() {
        let lines = emit_after(Duration::from_millis(500));
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with(" WARN "), "{lines:?}");
        assert!(lines[0].contains("slow request"), "{lines:?}");
        assert!(lines[0].contains("elapsed_ms=500"), "{lines:?}");
    }

    #[test]
    fn above_the_threshold_is_one_warn_slow_request_with_every_field() {
        let lines = emit_after(Duration::from_millis(1234));
        assert_eq!(lines.len(), 1, "{lines:?}");
        let line = &lines[0];
        assert!(line.starts_with(" WARN "), "{line}");
        for field in [
            "slow request",
            "method=\"textDocument/hover\"",
            "uri=\"file:///w/a.md\"",
            "id=7",
            "outcome=\"ok\"",
            "elapsed_ms=1234",
            "waited_for_file_list=false",
            "waited_for_index=false",
        ] {
            assert!(line.contains(field), "missing {field}: {line}");
        }
        // No stage ran, so none is reported, and an answered request is not
        // marked cancelled.
        for stage in ["overlay_build_ms", "schema_assembly_ms", "trigger_registry_ms", "provider_chain_ms"] {
            assert!(!line.contains(stage), "{stage} did not run: {line}");
        }
        assert!(!line.contains("cancelled"), "{line}");
    }

    #[test]
    fn a_request_without_a_document_omits_uri() {
        let timings = RequestTimings::start(manual());
        let id = lsp_server::RequestId::from(3);
        let ((), logs) = CapturedLogs::scoped(|| {
            CompletionEvent {
                kind: MessageKind::Request { id: &id },
                method: "workspace/symbol",
                uri: None,
                outcome: Outcome::Ok,
            }
            .emit(&timings);
        });
        let lines = logs.lines();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("method=\"workspace/symbol\""), "{lines:?}");
        assert!(!lines[0].contains("uri="), "{lines:?}");
    }

    #[test]
    fn stages_are_reported_separately_and_nested_not_summed() {
        let clock = manual();
        let timings = RequestTimings::start(clock.clone());
        timings.time(Stage::OverlayBuild, || {
            clock.advance(Duration::from_millis(100));
            timings.time(Stage::SchemaAssembly, || {
                clock.advance(Duration::from_millis(200));
                timings.time(Stage::TriggerRegistry, || {
                    clock.advance(Duration::from_millis(300));
                });
            });
        });
        timings.time(Stage::ProviderChain, || clock.advance(Duration::from_millis(50)));
        timings.record_wait(Wait::FileList);
        let id = lsp_server::RequestId::from(1);
        let ((), logs) = CapturedLogs::scoped(|| request(&id, None).emit(&timings));
        let line = &logs.lines()[0];
        for field in [
            "slow request",
            "elapsed_ms=650",
            "overlay_build_ms=600",
            "schema_assembly_ms=500",
            "trigger_registry_ms=300",
            "provider_chain_ms=50",
            "waited_for_file_list=true",
            "waited_for_index=false",
        ] {
            assert!(line.contains(field), "missing {field}: {line}");
        }
    }

    #[test]
    fn a_stage_that_runs_twice_reports_its_sum() {
        let clock = manual();
        let timings = RequestTimings::start(clock.clone());
        for _ in 0..2 {
            timings.time(Stage::ProviderChain, || clock.advance(Duration::from_millis(30)));
        }
        assert_eq!(timings.stage(Stage::ProviderChain), Some(Duration::from_millis(60)));
        assert_eq!(timings.stage(Stage::OverlayBuild), None);
    }

    #[test]
    fn a_cancelled_request_logs_elapsed_and_cancelled_true() {
        let clock = manual();
        let timings = RequestTimings::start(clock.clone());
        clock.advance(Duration::from_millis(12));
        let id = lsp_server::RequestId::from(9);
        let ((), logs) = CapturedLogs::scoped(|| {
            CompletionEvent { outcome: Outcome::Cancelled, ..request(&id, None) }.emit(&timings);
        });
        let lines = logs.lines();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("cancelled=true"), "{lines:?}");
        assert!(lines[0].contains("outcome=\"cancelled\""), "{lines:?}");
        assert!(lines[0].contains("elapsed_ms=12"), "{lines:?}");
    }

    #[test]
    fn notifications_use_their_own_text_and_carry_no_request_id() {
        for (elapsed, level, text) in [
            (Duration::from_millis(499), "DEBUG ", "notification complete"),
            (Duration::from_millis(500), " WARN ", "slow notification"),
        ] {
            let clock = manual();
            let timings = RequestTimings::start(clock.clone());
            clock.advance(elapsed);
            let ((), logs) = CapturedLogs::scoped(|| {
                CompletionEvent {
                    kind: MessageKind::Notification,
                    method: "textDocument/didChange",
                    uri: Some("file:///w/a.md"),
                    outcome: Outcome::Ok,
                }
                .emit(&timings);
            });
            let lines = logs.lines();
            assert_eq!(lines.len(), 1, "{lines:?}");
            assert!(lines[0].starts_with(level), "{lines:?}");
            assert!(lines[0].contains(text), "{lines:?}");
            assert!(!lines[0].contains("request complete") && !lines[0].contains("slow request"), "{lines:?}");
            assert!(!lines[0].contains(" id="), "{lines:?}");
        }
    }

    #[test]
    fn registry_wait_sums_parked_intervals_and_excludes_the_time_between_them() {
        let clock = manual();
        let timings = RequestTimings::start(clock.clone());
        assert_eq!(timings.registry_wait(), None, "never parked");
        timings.begin_registry_wait();
        clock.advance(Duration::from_millis(100));
        timings.end_registry_wait();
        // Handler work or an index wait between two parked intervals.
        clock.advance(Duration::from_millis(40));
        timings.begin_registry_wait();
        clock.advance(Duration::from_millis(50));
        timings.end_registry_wait();
        clock.advance(Duration::from_millis(7));
        assert_eq!(timings.registry_wait(), Some(Duration::from_millis(150)));

        let id = lsp_server::RequestId::from(4);
        let ((), logs) = CapturedLogs::scoped(|| request(&id, None).emit(&timings));
        let line = &logs.lines()[0];
        for field in ["elapsed_ms=197", "waited_for_trigger_registry=true", "trigger_registry_wait_ms=150"] {
            assert!(line.contains(field), "missing {field}: {line}");
        }
    }

    #[test]
    fn a_second_begin_keeps_the_interval_in_progress_and_an_end_without_one_does_nothing() {
        let clock = manual();
        let timings = RequestTimings::start(clock.clone());
        timings.end_registry_wait();
        assert_eq!(timings.registry_wait(), None);
        timings.begin_registry_wait();
        clock.advance(Duration::from_millis(30));
        timings.begin_registry_wait();
        clock.advance(Duration::from_millis(20));
        // Still parked: the open interval counts up to now.
        assert_eq!(timings.registry_wait(), Some(Duration::from_millis(50)));
        timings.end_registry_wait();
        timings.end_registry_wait();
        clock.advance(Duration::from_millis(500));
        assert_eq!(timings.registry_wait(), Some(Duration::from_millis(50)));
    }

    #[test]
    fn a_message_that_never_parked_omits_the_wait_figure() {
        let lines = emit_after(Duration::from_millis(600));
        assert!(lines[0].contains("waited_for_trigger_registry=false"), "{lines:?}");
        assert!(!lines[0].contains("trigger_registry_wait_ms"), "{lines:?}");
    }

    #[test]
    fn document_uri_reads_only_the_text_document_uri() {
        let params = serde_json::json!({ "textDocument": { "uri": "file:///w/a.md" } });
        assert_eq!(document_uri(&params).as_deref(), Some("file:///w/a.md"));
        assert_eq!(document_uri(&serde_json::json!({ "query": "x" })), None);
        assert_eq!(document_uri(&serde_json::Value::Null), None);
    }
}
