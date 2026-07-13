//! Bridge: feed classified stderr log lines into the shared semantic sink,
//! maintain stderr-side summary counters, and signal early termination when a
//! pre-stream usage-cap error is detected.

pub mod errors;
pub mod format;
pub mod session;
pub mod signals;
pub mod stall_guard;

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use tracing::debug;

use claudine_catalog_types::{Quantity, Unit};

use crate::signals::{CapScope, SignalEvent as TaxonomySignalEvent, SignalHub, SignalSource};
use crate::stream::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use crate::stream::summary::RateLimitInfo;

use crate::stream::logs::opencode::classify::{
    asset_type_as_str, classify, classify_raw, get_http_status_description,
    max_reset_at, merge_rate_limit, render_malformed_asset_message, render_rate_limit_message,
    strip_ansi,
};
use crate::stream::logs::opencode::events::{
    AssetType, LogClassification, LogLevel, OpenCodeLogRecord, ParsedOpenCodeStderrLine,
    ProviderLimitKind,
};
use crate::stream::logs::opencode::state::SharedStderrState;

use errors::{is_stream_error, stream_error_fingerprint, MAX_CONSECUTIVE_STREAM_ERRORS};
use format::{base_extra, format_llm_call_message, format_http_response_message, format_permission_message, format_snapshot_message, non_empty, summarize_snapshot_tags};
use session::ChildSessionInfo;

/// Whether the bridge took ownership of an incoming stderr log line and
/// therefore suppresses raw passthrough.
///
/// A line is `Consumed` whenever the bridge recognized it as a structured
/// OpenCode log record — regardless of whether that record produced a
/// [`SemanticEvent`], was intentionally filtered (e.g. `service=bus`), or
/// was simply Unclassified. The structured-log format is OpenCode's, so any
/// line that parses as one is "ours" to interpret; echoing it to the user
/// proxies internal debug output and is never what we want.
///
/// Only truly unstructured stderr (panics, raw `RawText` content, plain
/// progress preambles) remains `NotConsumed`, preserving the legacy raw
/// passthrough so genuine error output still reaches the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StderrIngestOutcome {
    /// The bridge handled this line — either by emitting a [`SemanticEvent`]
    /// or by parsing it as a structured record we intentionally drop.
    /// Callers must not echo the raw line.
    Consumed,
    /// The bridge did not recognize the line as a structured OpenCode log
    /// record (or as a classifiable raw error). Callers should keep the
    /// existing raw-passthrough behavior.
    NotConsumed,
}

/// Diagnostic enrichment carried by `EarlyTermination::StepTimeout` for
/// subagents that were still outstanding when the stream-silence rule
/// fired. Lives in the lib so the CLI's `WatchdogTermination` can convert
/// from its internal `ActiveSubagentSnapshot` without leaking CLI types
/// across the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StuckSubagentInfo {
    /// Subagent identifier (e.g. an OpenCode session id).
    pub id: String,
    /// Optional human-readable name or title of the subagent.
    pub name: Option<String>,
    /// Wall-clock duration since the subagent last reported progress.
    pub elapsed_since_progress: Duration,
}

/// Safe OpenCode metadata captured at the last attempted generation, carried
/// by [`EarlyTermination::StalledGeneration`] so the rendered error block and
/// `guard_context` can name the run without re-parsing the stream.
///
/// Every field is optional — OpenCode does not always tag every value on a
/// `service=llm` record. The struct deliberately stores **only** safe
/// metadata: there is no field for prompt text, tool inputs/outputs, HTTP
/// URLs, authorization headers, or raw stderr lines, and none must ever be
/// added. The guard needs identity, not payloads.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StalledGenerationContext {
    /// OpenCode session id (`session.id` tag) of the stalled generation.
    pub session_id: Option<String>,
    /// Reasoning step the stall was observed on, from the bridge's per-session
    /// dedup state when known.
    pub step: Option<u32>,
    /// Agent name driving the generation (e.g. `rust-developer`).
    pub agent: Option<String>,
    /// Provider id of the model attempting the generation.
    pub provider_id: Option<String>,
    /// Model id attempting the generation.
    pub model_id: Option<String>,
    /// OpenCode generation mode (e.g. `primary`, `all`).
    pub mode: Option<String>,
}

/// Reason the bridge wants `run_child_stream_semantic(...)` to terminate
/// the child process early.
///
/// Today this fires for pre-stream usage-cap failures and the unified
/// two-rule timeout watchdog (`timeout` and `step_timeout`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EarlyTermination {
    /// The provider reported a rate-limit failure before any stdout
    /// semantic event was observed.
    RateLimit {
        message: String,
        reset_at: Option<DateTime<Utc>>,
    },
    /// The wall-clock budget (`timeout`) elapsed since the child process
    /// was spawned. The wrapper terminates the child process and maps the
    /// outcome to [`crate::harness::ProcessTermination::TimedOut`] so the
    /// lifecycle `failure` stack observes a `Timeout` failure event. The
    /// synthesized summary marks `error_kind = "timeout"`.
    Timeout { message: String },
    /// The stream-silence budget (`step_timeout`) elapsed with no parent
    /// stream event observed. The wrapper terminates the child process and
    /// maps the outcome to [`crate::harness::ProcessTermination::TimedOut`].
    /// The synthesized summary marks `error_kind = "step_timeout"`.
    ///
    /// `outstanding` enumerates any subagents that were still in flight at
    /// the moment of breach so the rendered error block can name them.
    StepTimeout {
        message: String,
        #[allow(dead_code)]
        outstanding: Vec<StuckSubagentInfo>,
    },
    /// A user-authored exit-expression matched a line of streamed provider
    /// output. The wrapper terminates the child process and maps the
    /// outcome to [`crate::harness::ProcessTermination::Aborted`] so
    /// [`crate::harness::classify_failure`] yields
    /// [`crate::harness::FailureEvent::AgentFailure`] (never the
    /// timeout-retry path).
    ///
    /// ## Notes
    ///
    /// Synthesized summary marks `error_kind = "exit_expression"`. The
    /// matched `pattern` (literal substring or regex source) and optional
    /// `scope` (e.g. `opencode/kimi-for-coding/k2p7`) are carried verbatim
    /// into the failure payload.
    ExitExpression {
        pattern: String,
        scope: Option<String>,
    },
    /// The content detector observed a tight group-cycle repetition that
    /// crossed the configured threshold (`repeats` ≥ the runaway detector's
    /// `MAX_REPETITION_ALLOWED` constant, landed in Phase 2). The wrapper
    /// terminates the child process and maps the outcome to
    /// [`crate::harness::ProcessTermination::Aborted`].
    ///
    /// ## Notes
    ///
    /// Synthesized summary marks `error_kind = "runaway_repetition"`.
    /// `cycle_len` is the detected cycle length `L` in `1..=MAX_CYCLE_LENGTH`
    /// and `repeats` is the count of consecutive matching cycles observed at
    /// trip time (always ≥ the threshold). A trip is terminal — the caller
    /// must not feed further output after one is observed.
    RunawayRepetition { cycle_len: usize, repeats: usize },
    /// The per-turn volume cap was exceeded on either the line count or the
    /// byte count. The wrapper terminates the child process and maps the
    /// outcome to [`crate::harness::ProcessTermination::Aborted`]. On the
    /// capture path this additionally bounds the unbounded accumulator
    /// buffer (per-run cap, not per-turn).
    ///
    /// ## Notes
    ///
    /// Synthesized summary marks `error_kind = "runaway_volume"`. `lines`
    /// and `bytes` are the per-turn (streaming) or per-run (capture)
    /// counters at the moment of breach; at least one will be ≥ its
    /// threshold.
    RunawayVolume { lines: u64, bytes: u64 },
    /// Consecutive OpenCode `stream error` records crossed
    /// [`MAX_CONSECUTIVE_STREAM_ERRORS`] with no step advance — the provider is
    /// failing every retry and OpenCode's internal backoff would otherwise spin
    /// forever. The wrapper terminates the child and maps the outcome to
    /// [`crate::harness::ProcessTermination::Aborted`] so
    /// [`crate::harness::classify_failure`] yields
    /// [`crate::harness::FailureEvent::AgentFailure`] (fail-fast, never the
    /// timeout-retry path that would reproduce the failure loop).
    ///
    /// ## Notes
    ///
    /// Synthesized summary marks `error_kind = "repeated_stream_error"`.
    /// `count` is the consecutive-failure count at trip time (≥ the threshold).
    RepeatedStreamError { count: u32 },
    /// OpenCode kept launching fresh generations (`llm_call_start`) without ever
    /// making forward progress — the live-but-dead retry-churn fingerprint. The
    /// provider returned neither a response nor an error envelope, so every
    /// liveness clock (`step_timeout`'s event/byte heartbeats) and the
    /// [`Self::RepeatedStreamError`] backstop are defeated; only repeated
    /// generation attempts with zero intervening progress remain on the wire.
    /// The wrapper terminates the child and maps the outcome to
    /// [`crate::harness::ProcessTermination::Aborted`] so
    /// [`crate::harness::classify_failure`] yields
    /// [`crate::harness::FailureEvent::AgentFailure`] (fail-fast, **never** the
    /// `handle_timeout:` retry path that would reproduce the stall).
    ///
    /// ## Notes
    ///
    /// Synthesized summary marks `error_kind = "stalled_generation"`. Fires
    /// only when **both** conditions hold: retry churn
    /// (`generation_count >= MAX_GENERATIONS_WITHOUT_PROGRESS`) **and**
    /// progress silence (`stall_duration >= stall_timeout`). Either condition
    /// alone never trips, so a single legitimately slow generation is exempt.
    /// `generation_count` is the count of generation attempts since the last
    /// progress-class event, `stall_duration` is the elapsed progress silence
    /// at trip time, and `context` carries only safe identity metadata (no
    /// prompt text or tool payloads).
    StalledGeneration {
        generation_count: u32,
        stall_duration: Duration,
        context: StalledGenerationContext,
    },
}

impl EarlyTermination {
    /// The taxonomy [`TaxonomySignalEvent`] this termination mirrors.
    ///
    /// The variants' synthesized summary `error_kind` strings deliberately
    /// match the corresponding `SignalKind` names; this exhaustive match
    /// pins that correspondence — adding an `EarlyTermination` variant
    /// forces a mapping decision here.
    pub fn to_signal_event(&self) -> TaxonomySignalEvent {
        match self {
            // `RateLimit` is only constructed on the terminal-cap path
            // (`ProviderLimitKind::UsageCap` / `RetriesExhausted`, both
            // classified terminal in `on_provider_limit`), and the summary
            // synthesis maps it to `error_kind = "usage_limit_reached"` —
            // so it is a `usage_capped` signal, not a transient
            // `rate_limited`. A retries-exhausted origin still fires the
            // precise `retries_exhausted` kind through the declarative
            // stderr_promoted record; this bespoke mirror keeps parity with
            // the summary's cap semantics.
            Self::RateLimit { message, reset_at } => TaxonomySignalEvent::UsageCapped {
                // OpenCode's terminal cap payload names no model or window.
                model: CapScope::All,
                timeframe: None,
                // A fired cap has zero capacity left (design convention).
                remaining: Some(Quantity {
                    value: 0.0,
                    unit: Unit::Percent,
                }),
                lifts_at: *reset_at,
                message: Some(message.clone()),
            },
            Self::Timeout { message } => TaxonomySignalEvent::Timeout {
                message: Some(message.clone()),
            },
            Self::StepTimeout { message, .. } => TaxonomySignalEvent::StepTimeout {
                message: Some(message.clone()),
            },
            Self::ExitExpression { pattern, scope } => TaxonomySignalEvent::ExitExpression {
                pattern: pattern.clone(),
                scope: scope.clone(),
            },
            Self::RunawayRepetition { cycle_len, repeats } => {
                TaxonomySignalEvent::RunawayRepetition {
                    cycle_len: u32::try_from(*cycle_len).unwrap_or(u32::MAX),
                    repeats: u32::try_from(*repeats).unwrap_or(u32::MAX),
                }
            }
            Self::RunawayVolume { lines, bytes } => TaxonomySignalEvent::RunawayVolume {
                lines: *lines,
                bytes: *bytes,
            },
            Self::RepeatedStreamError { count } => {
                TaxonomySignalEvent::RepeatedStreamError { count: *count }
            }
            Self::StalledGeneration {
                generation_count,
                stall_duration,
                ..
            } => TaxonomySignalEvent::StalledGeneration {
                message: Some(format!(
                    "{generation_count} generation attempts over {}s with no progress",
                    stall_duration.as_secs()
                )),
            },
        }
    }
}

/// Progress half of the stalled-generation backstop, shared across the two
/// producers that can observe forward progress.
///
/// The stderr [`OpenCodeLogBridge`] owns the counting and trip logic, but
/// genuine forward progress also arrives on **stdout** as semantic events
/// (`OutputText`, `ToolCall`, …). The stdout NDJSON parser runs on a different
/// thread, so the two clocks this guard reads live behind an `Arc<Mutex<…>>`:
/// the bridge resets them on its own progress-class stderr records, and a
/// stdout-side progress observer (see
/// [`StalledProgressObserverSink`](crate::stream::semantic::StalledProgressObserverSink))
/// resets the same cell when a progress-class stdout event is forwarded.
/// Without that shared cell, stdout-origin progress would never clear the
/// counter and a later `llm_call_start` could trip the guard on a run that was
/// in fact progressing.
///
/// Each method takes `now: Instant` so the detector stays testable without
/// real sleeps. The critical section never emits a [`SemanticEvent`], so the
/// lock is always released before any sink call — there is no nested-lock
/// ordering between this `Mutex` and the shared semantic-sink `Mutex`.
#[derive(Debug)]
pub struct StalledGenerationProgress {
    /// Monotonic instant of the last progress-class event observed on either
    /// the stderr bridge or the stdout semantic stream.
    last_progress_at: Instant,
    /// Streamed `llm_call_start` records observed since the last progress-class
    /// event. One half of the two-condition trip.
    generation_count_since_progress: u32,
}

impl StalledGenerationProgress {
    /// Seed both clocks at `now`. Called at bridge construction so the silence
    /// budget starts ticking immediately (the live-but-dead incident occurred
    /// entirely while stderr was active, before any stdout NDJSON).
    fn new(now: Instant) -> Self {
        Self {
            last_progress_at: now,
            generation_count_since_progress: 0,
        }
    }

    /// Mark forward progress: advance the silence clock to `now` and clear the
    /// generation churn counter. Idempotent — a second progress event in the
    /// same instant simply re-stamps the clock.
    ///
    /// Public so the stdout-side progress observer can reset the shared cell
    /// when it forwards a progress-class semantic event.
    pub fn mark_progress(&mut self, now: Instant) {
        self.last_progress_at = now;
        self.generation_count_since_progress = 0;
    }
}

/// Stderr-side integration object for the OpenCode structured wrapper path.
///
/// Responsibilities:
/// - parse and classify one stderr line at a time via
///   [`parse_line`](super::events::parse_line) +
///   [`classify`] / [`classify_raw`]
/// - emit [`SemanticEvent`]s through a shared sink so the live renderer and
///   JSONL reporting surface stderr diagnostics alongside stdout events
/// - update shared stderr summary counters so the wrapper layer can merge
///   them into the final [`crate::stream::summary::StreamExecutionSummary`]
/// - optionally send an [`EarlyTermination`] signal when a rate limit is
///   observed before any stdout activity
pub struct OpenCodeLogBridge<S: SemanticEventSink> {
    sink: S,
    state: Arc<Mutex<SharedStderrState>>,
    stdout_event_seen: Arc<AtomicBool>,
    early_terminate: Option<Sender<EarlyTermination>>,
    early_terminate_fired: bool,
    /// Whether a primary [`SemanticEvent::SessionStart`] has been emitted
    /// from the stderr stream. Used to suppress duplicates when multiple
    /// session-created records arrive without a parent id (first match wins).
    primary_session_emitted: bool,
    /// Tracks child sessions discovered from `service=session ... parentID=...`
    /// records so the bridge can convert subsequent `service=session.prompt
    /// ... exiting loop` records into [`SemanticEvent::SubagentStop`] events.
    child_sessions: BTreeMap<String, ChildSessionInfo>,
    /// Last `step` value emitted per session for `step_loop` records.
    /// OpenCode emits a `service=session.prompt … loop` record at every
    /// HTTP-span boundary inside the same reasoning step, so we suppress
    /// repeats and only re-emit when `step` actually advances.
    last_step_per_session: BTreeMap<String, u32>,
    /// Count of consecutive *identical* `message="stream error"` records
    /// observed with no intervening step advance. Bounds OpenCode's unbounded
    /// backoff retries of the **same** terminal failure so an error vocabulary
    /// the classifier does not recognize as terminal still degrades to a
    /// fail-fast abort instead of an indefinite hang. Only repeats whose
    /// fingerprint (see [`stream_error_fingerprint`]) matches
    /// [`Self::last_stream_error_fingerprint`] accumulate; a genuinely different
    /// stream error resets the run to 1. Reset to 0 on any genuine step
    /// transition (see [`Self::on_step_loop`]).
    consecutive_stream_errors: u32,
    /// Fingerprint of the most recent `stream error` record, used to decide
    /// whether the next stream error continues the current run (matching
    /// fingerprint → increment) or starts a fresh one (changed fingerprint →
    /// reset to 1). `None` until the first stream error and after a step
    /// transition. See [`stream_error_fingerprint`] for what the fingerprint
    /// captures (and why the per-line timestamp is excluded).
    last_stream_error_fingerprint: Option<String>,
    /// Stall-timeout budget for the stalled-generation backstop. `None`
    /// disables the guard entirely (the `0s`-disables contract resolves to
    /// `None` upstream); when `Some(d)`, a trip additionally requires progress
    /// silence to reach `d`.
    stall_timeout: Option<Duration>,
    /// Progress clocks for the stalled-generation backstop (last progress
    /// instant + generation churn count), shared with the stdout-side progress
    /// observer. Seeded at construction so the silence budget starts ticking at
    /// bridge creation, not at first stdout NDJSON. Behind an `Arc<Mutex<…>>`
    /// because stdout-origin progress arrives on a different thread; see
    /// [`StalledGenerationProgress`].
    progress: Arc<Mutex<StalledGenerationProgress>>,
    /// Safe metadata from the most recent streamed `LlmCall`. Retained across a
    /// progress reset so a later trip message can still name the last attempted
    /// generation. Bridge-local (only the stderr side knows generation
    /// identity), so it stays outside the shared progress cell.
    last_generation_context: Option<StalledGenerationContext>,
    /// Glue-mode shim into the run's signal pipeline (E5): every
    /// non-`Unclassified` classification is re-serialized through
    /// [`LogClassification::to_signal_payload`] and observed as
    /// [`SignalSource::StderrPromoted`], and early terminations emit their
    /// bespoke [`TaxonomySignalEvent`] mirror. `None` on legacy/test
    /// constructions leaves the bridge signal-free.
    signal_hub: Option<Arc<SignalHub>>,
}

impl<S: SemanticEventSink> OpenCodeLogBridge<S> {
    /// Build a new bridge wired to a shared sink, an observation gate, an
    /// optional early-termination channel, and the stalled-generation
    /// backstop budget.
    ///
    /// `stall_timeout` is `None` to disable the live-but-dead guard (the
    /// `0s`-disables contract resolves to `None` upstream); `Some(d)` arms it
    /// with a `d` progress-silence threshold. The progress clock
    /// (`last_progress_at`) is seeded to `Instant::now()` here so the silence
    /// budget starts ticking at bridge creation, not at first stdout NDJSON.
    pub fn new(
        sink: S,
        stdout_event_seen: Arc<AtomicBool>,
        early_terminate: Option<Sender<EarlyTermination>>,
        stall_timeout: Option<Duration>,
    ) -> Self {
        Self {
            sink,
            state: Arc::new(Mutex::new(SharedStderrState::default())),
            stdout_event_seen,
            early_terminate,
            early_terminate_fired: false,
            primary_session_emitted: false,
            child_sessions: BTreeMap::new(),
            last_step_per_session: BTreeMap::new(),
            consecutive_stream_errors: 0,
            last_stream_error_fingerprint: None,
            stall_timeout,
            progress: Arc::new(Mutex::new(StalledGenerationProgress::new(Instant::now()))),
            last_generation_context: None,
            signal_hub: None,
        }
    }

    /// Attach the run's shared signal hub so classified stderr records and
    /// early terminations feed the signal pipeline. See the `signal_hub`
    /// field for the emission contract.
    pub fn with_signal_hub(mut self, hub: Arc<SignalHub>) -> Self {
        self.signal_hub = Some(hub);
        self
    }

    /// Clone handle into the shared stalled-generation progress clocks.
    ///
    /// The stdout NDJSON parser runs on a separate thread and cannot reach the
    /// bridge directly, so it resets these clocks through a
    /// [`StalledProgressObserverSink`](crate::stream::semantic::StalledProgressObserverSink)
    /// built from this handle. Returns the same cell the bridge mutates, so a
    /// stdout-origin progress event and a stderr-bridge progress event clear one
    /// shared counter.
    pub fn stalled_generation_progress(&self) -> Arc<Mutex<StalledGenerationProgress>> {
        Arc::clone(&self.progress)
    }

    /// Generation churn count currently held in the shared progress cell.
    #[cfg(test)]
    fn generation_count_since_progress(&self) -> u32 {
        self.progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .generation_count_since_progress
    }

    /// Last-progress instant currently held in the shared progress cell.
    #[cfg(test)]
    fn last_progress_at(&self) -> Instant {
        self.progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .last_progress_at
    }

    /// Create a new early-termination channel. Returns the sender for the
    /// bridge plus the receiver the wait loop should poll.
    pub fn new_early_terminate_channel() -> (Sender<EarlyTermination>, Receiver<EarlyTermination>) {
        mpsc::channel()
    }

    /// Clone handle into the shared stderr state for post-run merging.
    pub fn shared_state(&self) -> Arc<Mutex<SharedStderrState>> {
        Arc::clone(&self.state)
    }

    /// Consume one stderr line and return whether the bridge absorbed it.
    ///
    /// A `Consumed` result means the bridge emitted a [`SemanticEvent`];
    /// the caller should suppress raw passthrough for that line.
    pub fn ingest(&mut self, line: &str) -> StderrIngestOutcome {
        match crate::stream::logs::opencode::events::parse_line(line) {
            ParsedOpenCodeStderrLine::Structured(record) => self.handle_structured(record),
            ParsedOpenCodeStderrLine::RawText(raw) => self.handle_raw(&raw),
        }
    }

    fn handle_structured(&mut self, record: OpenCodeLogRecord) -> StderrIngestOutcome {
        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.log_records_parsed =
                state.diagnostics.log_records_parsed.saturating_add(1);
        }

        // Aggressive noise filter: `service=bus` lines carry internal
        // cross-module chatter that is not meaningful to operators.
        // They are still counted as parsed (so the summary is accurate)
        // but they do not emit semantic events. Return `Consumed` so the
        // raw line is suppressed from the user's terminal — the
        // byte-heartbeat refresh in the stderr reader thread (spawn.rs)
        // already happened before this line reached the bridge, so the
        // silence watchdog still sees activity.
        if record.tags.get("service").map(|s| s.as_str()) == Some("bus") {
            return StderrIngestOutcome::Consumed;
        }

        let classification = classify(&record);

        // Glue-mode signal shim (E5, ratified): re-serialize the
        // classification and let the declarative stderr_promoted records
        // match it. A `BootBanner` flowing through narrows version-scoped
        // record selection for the whole run — intended (the opencode
        // provider_version record extracts it). `Unclassified` records
        // carry no classification payload worth observing.
        if !matches!(classification, LogClassification::Unclassified)
            && let Some(hub) = &self.signal_hub
        {
            hub.observe_json(
                SignalSource::StderrPromoted,
                &classification.to_signal_payload(),
            );
        }

        // Defense-in-depth backstop. A `stream error` record that the
        // classifier already deemed terminal fires early-termination through
        // its own handler below (idempotent via `early_terminate_fired`). This
        // guard catches the residual case: repeated *identical* stream errors
        // the classifier treats as non-terminal (or an unknown future shape),
        // which OpenCode would retry the same failure under unbounded backoff.
        // Only consecutive identical failures accumulate — a genuinely
        // different stream error starts a fresh run (count = 1), so five
        // distinct one-off errors in a step do not trip the abort.
        if is_stream_error(&record) {
            let fingerprint = stream_error_fingerprint(&record);
            if self.last_stream_error_fingerprint.as_ref() == Some(&fingerprint) {
                self.consecutive_stream_errors =
                    self.consecutive_stream_errors.saturating_add(1);
            } else {
                // New (or first) fingerprint: this error is the first of a new
                // run rather than a continuation of the previous one.
                self.consecutive_stream_errors = 1;
            }
            self.last_stream_error_fingerprint = Some(fingerprint);
            if self.consecutive_stream_errors >= MAX_CONSECUTIVE_STREAM_ERRORS
                && !self.early_terminate_fired
            {
                return self.on_repeated_stream_error(&record);
            }
        }

        match classification {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                reset_at,
                ref provider_id,
                ref model_id,
                ref provider_error,
            } => self.on_provider_limit(
                &record,
                status_code,
                kind,
                reset_at,
                provider_id.clone(),
                model_id.clone(),
                provider_error.clone(),
            ),
            LogClassification::MalformedAsset {
                asset_type,
                ref path,
                ref error,
            } => self.on_malformed_asset(&record, asset_type, path.clone(), error.clone()),
            LogClassification::ApiFailure {
                status_code,
                ref error_name,
                ref message,
                is_fatal,
            } => self.on_api_failure(
                &record,
                status_code,
                error_name.clone(),
                message.clone(),
                is_fatal,
            ),
            LogClassification::AuthFailure { ref message } => {
                self.on_auth_failure(&record, message.clone())
            }
            LogClassification::UncaughtError { ref raw_text } => {
                self.on_uncaught_error(raw_text.clone(), Some(&record))
            }
            // Boot banner is parsed and counted but not promoted to a
            // semantic event in Phase 3; the NDJSON stream's own session
            // event remains the SessionStart anchor when present. Suppress
            // from raw passthrough — it's a structured log record we own.
            LogClassification::BootBanner { .. } => StderrIngestOutcome::Consumed,
            LogClassification::SessionCreated { id, parent_id } => {
                self.on_session_created(&record, id, parent_id)
            }
            LogClassification::LlmCall {
                provider_id,
                model_id,
                mode,
                is_stream,
            } => self.on_llm_call(&record, provider_id, model_id, mode, is_stream),
            LogClassification::StepLoop { session_id, step } => {
                self.on_step_loop(&record, session_id, step)
            }
            LogClassification::StepExit { session_id } => self.on_step_exit(&record, session_id),
            LogClassification::PermissionEvaluated {
                permission,
                pattern,
                action,
            } => self.on_permission_evaluated(&record, permission, pattern, action),
            LogClassification::HttpResponse {
                method,
                url,
                status,
                duration_ms,
            } => self.on_http_response(&record, method, url, status, duration_ms),
            LogClassification::Snapshot { message, level } => {
                self.on_snapshot(&record, message, level)
            }
            // An Unclassified line still parsed as a well-formed OpenCode
            // structured log record (level + timestamp + tags + message).
            // We have already counted it and refreshed the byte heartbeat;
            // it is ours to drop rather than proxy as raw debug output.
            LogClassification::Unclassified => StderrIngestOutcome::Consumed,
        }
    }

    fn handle_raw(&mut self, line: &str) -> StderrIngestOutcome {
        match classify_raw(line) {
            classification @ LogClassification::UncaughtError { .. } => {
                // Same glue-mode shim as `handle_structured`: raw-line
                // classifications are stderr-promoted evidence too.
                if let Some(hub) = &self.signal_hub {
                    hub.observe_json(
                        SignalSource::StderrPromoted,
                        &classification.to_signal_payload(),
                    );
                }
                let LogClassification::UncaughtError { raw_text } = classification else {
                    unreachable!("matched UncaughtError above");
                };
                self.on_uncaught_error(raw_text, None)
            }
            _ => StderrIngestOutcome::NotConsumed,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn on_provider_limit(
        &mut self,
        record: &OpenCodeLogRecord,
        status_code: Option<u16>,
        kind: ProviderLimitKind,
        reset_at: Option<DateTime<Utc>>,
        provider_id: Option<String>,
        model_id: Option<String>,
        provider_error: String,
    ) -> StderrIngestOutcome {
        let rendered_message = match kind {
            ProviderLimitKind::Overloaded => "server overloaded; will retry".to_string(),
            ProviderLimitKind::RateLimited => "request throttled; will retry".to_string(),
            ProviderLimitKind::UsageCap => {
                render_rate_limit_message(provider_id, model_id, reset_at)
            }
            ProviderLimitKind::RetriesExhausted => {
                "provider 429s did not clear after retries".to_string()
            }
        };
        let is_terminal = matches!(
            kind,
            ProviderLimitKind::UsageCap | ProviderLimitKind::RetriesExhausted
        );

        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.rate_limit_events =
                state.diagnostics.rate_limit_events.saturating_add(1);
            state.diagnostics.rate_limit_reset_at =
                max_reset_at(state.diagnostics.rate_limit_reset_at, reset_at);
            if is_terminal {
                state.rate_limit = Some(merge_rate_limit(
                    state.rate_limit.take(),
                    RateLimitInfo {
                        is_throttled: Some(true),
                        retry_after_ms: None,
                        message: Some(rendered_message.clone()),
                        reset_at,
                    },
                ));
            }
        }

        let mut extra_map = base_extra(record, "rate_limit");
        if let Some(code) = status_code {
            extra_map.insert("status_code".into(), json!(code));
        }
        extra_map.insert("kind".into(), Value::String(format!("{kind:?}")));
        if let Some(reset) = reset_at {
            extra_map.insert(
                "reset_at".into(),
                Value::String(reset.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            );
        }
        if !provider_error.is_empty() {
            extra_map.insert("provider_error".into(), Value::String(provider_error));
        }

        if is_terminal {
            debug!(
                status_code,
                kind = ?kind,
                reset_at = ?reset_at,
                "opencode provider-limit classified as terminal; requesting early termination",
            );
            self.sink.on_semantic_event(SemanticEvent::Error {
                message: rendered_message.clone(),
                terminal: true,
                kind: SemanticErrorKind::ApiRemote,
                extra: Value::Object(extra_map),
            });
            self.fire_early_termination(EarlyTermination::RateLimit {
                message: rendered_message,
                reset_at,
            });
        } else {
            debug!(
                status_code,
                kind = ?kind,
                reset_at = ?reset_at,
                "opencode provider-limit classified as non-terminal; emitting warning",
            );
            self.sink.on_semantic_event(SemanticEvent::Warning {
                message: rendered_message,
                extra: Value::Object(extra_map),
            });
        }

        StderrIngestOutcome::Consumed
    }

    fn on_malformed_asset(
        &mut self,
        record: &OpenCodeLogRecord,
        asset_type: AssetType,
        path: Option<String>,
        error: String,
    ) -> StderrIngestOutcome {
        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.malformed_asset_events =
                state.diagnostics.malformed_asset_events.saturating_add(1);
        }

        let mut extra_map = base_extra(record, "malformed_asset");
        extra_map.insert(
            "asset_type".into(),
            Value::String(asset_type_as_str(asset_type).into()),
        );
        if let Some(ref path) = path {
            extra_map.insert("path".into(), Value::String(path.clone()));
        }
        if !error.is_empty() {
            extra_map.insert("error".into(), Value::String(error.clone()));
        }

        let message = render_malformed_asset_message(asset_type, path.as_deref());
        self.sink.on_semantic_event(SemanticEvent::Warning {
            message,
            extra: Value::Object(extra_map),
        });

        StderrIngestOutcome::Consumed
    }

    fn on_api_failure(
        &mut self,
        record: &OpenCodeLogRecord,
        status_code: Option<u16>,
        error_name: String,
        message: String,
        is_fatal: bool,
    ) -> StderrIngestOutcome {
        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.api_failures = state.diagnostics.api_failures.saturating_add(1);
        }

        let mut extra_map = base_extra(record, "api_failure");
        extra_map.insert("error_name".into(), Value::String(error_name.clone()));
        extra_map.insert("is_fatal".into(), json!(is_fatal));
        if let Some(code) = status_code {
            extra_map.insert("status_code".into(), json!(code));
        }
        if let Some(raw_error) = record.tags.get("error") {
            extra_map.insert("raw_error".into(), Value::String(raw_error.clone()));
        }

        let rendered = if !message.is_empty() {
            message
        } else {
            match status_code {
                Some(code) => {
                    let desc = get_http_status_description(code);
                    if !desc.is_empty() {
                        format!("OpenCode API failure ({error_name}: {code} {desc})")
                    } else {
                        format!("OpenCode API failure ({error_name}: {code})")
                    }
                }
                None => format!("OpenCode API failure ({error_name})"),
            }
        };

        if is_fatal {
            self.sink.on_semantic_event(SemanticEvent::Error {
                message: rendered,
                terminal: true,
                kind: SemanticErrorKind::ApiRemote,
                extra: Value::Object(extra_map),
            });
        } else {
            self.sink.on_semantic_event(SemanticEvent::Warning {
                message: rendered,
                extra: Value::Object(extra_map),
            });
        }

        StderrIngestOutcome::Consumed
    }

    fn on_auth_failure(
        &mut self,
        record: &OpenCodeLogRecord,
        message: String,
    ) -> StderrIngestOutcome {
        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.auth_failures = state.diagnostics.auth_failures.saturating_add(1);
        }

        let mut extra_map = base_extra(record, "auth_failure");
        if !message.is_empty() {
            extra_map.insert("detail".into(), Value::String(message.clone()));
        }

        let rendered = if message.is_empty() {
            "OpenCode authentication failed".to_string()
        } else {
            message
        };

        self.sink.on_semantic_event(SemanticEvent::Error {
            message: rendered,
            terminal: true,
            kind: SemanticErrorKind::ApiRemote,
            extra: Value::Object(extra_map),
        });

        StderrIngestOutcome::Consumed
    }

    fn on_uncaught_error(
        &mut self,
        raw_text: String,
        record: Option<&OpenCodeLogRecord>,
    ) -> StderrIngestOutcome {
        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.uncaught_errors = state.diagnostics.uncaught_errors.saturating_add(1);
        }

        let mut extra_map = Map::new();
        extra_map.insert("provider".into(), Value::String("opencode".into()));
        extra_map.insert("source".into(), Value::String("stderr_log".into()));
        extra_map.insert(
            "classification".into(),
            Value::String("uncaught_error".into()),
        );
        extra_map.insert("raw".into(), Value::String(raw_text.clone()));

        if let Some(record) = record {
            if let Some(service) = record.tags.get("service") {
                extra_map.insert("service".into(), Value::String(service.clone()));
            }
            if let Some(name) = record.tags.get("name") {
                extra_map.insert("error_name".into(), Value::String(name.clone()));
            }
        }

        let rendered = strip_ansi(&raw_text).trim().to_string();
        let rendered = if rendered.is_empty() {
            "OpenCode produced an uncaught error".to_string()
        } else {
            rendered
        };

        self.sink.on_semantic_event(SemanticEvent::Error {
            message: rendered,
            terminal: true,
            kind: SemanticErrorKind::Unknown,
            extra: Value::Object(extra_map),
        });

        StderrIngestOutcome::Consumed
    }

    fn on_llm_call(
        &mut self,
        record: &OpenCodeLogRecord,
        provider_id: String,
        model_id: String,
        mode: String,
        is_stream: bool,
    ) -> StderrIngestOutcome {
        // Capture the first `mode=primary` LLM call so the final
        // [`crate::stream::summary::StreamExecutionSummary`] can attribute
        // the primary provider/model even when the stdout NDJSON stream
        // omits the `init` payload (e.g. OpenCode's "DONE-only" stream).
        if mode == "primary" {
            let mut state = self.state.lock().expect("stderr state poisoned");
            if state.primary_provider_id.is_none() {
                state.primary_provider_id = Some(provider_id.clone());
            }
            if state.primary_model_id.is_none() {
                state.primary_model_id = Some(model_id.clone());
            }
        }

        let mut extra_map = base_extra(record, "llm_call");
        extra_map.insert("provider_id".into(), Value::String(provider_id.clone()));
        extra_map.insert("model_id".into(), Value::String(model_id.clone()));
        extra_map.insert("mode".into(), Value::String(mode.clone()));
        extra_map.insert("is_stream".into(), json!(is_stream));
        let agent = record.tags.get("agent").cloned();
        if let Some(ref agent) = agent {
            extra_map.insert("agent".into(), Value::String(agent.clone()));
        }
        if let Some(small) = record.tags.get("small") {
            extra_map.insert("small".into(), Value::String(small.clone()));
        }
        if let Some(session_id) = record.tags.get("session.id") {
            extra_map.insert("session_id".into(), Value::String(session_id.clone()));
        }

        let rendered_message =
            format_llm_call_message(&provider_id, &model_id, &mode, agent.as_deref());
        self.sink.on_semantic_event(SemanticEvent::Info {
            message: rendered_message,
            extra: Value::Object(extra_map),
        });

        // Stalled-generation backstop: only streamed generations count toward
        // the retry-churn fingerprint. A non-streamed call (e.g. a title/summary
        // helper) is not the live-but-dead loop.
        if is_stream {
            let session_id = record.tags.get("session.id").cloned();
            let step = session_id
                .as_ref()
                .and_then(|sid| self.last_step_per_session.get(sid).copied());
            let ctx = StalledGenerationContext {
                session_id,
                step,
                agent,
                provider_id: non_empty(provider_id),
                model_id: non_empty(model_id),
                mode: non_empty(mode),
            };
            let now = Instant::now();
            if let Some(termination) = self.record_llm_call_and_check_trip(now, ctx) {
                return self.on_stalled_generation(record, termination);
            }
        }
        StderrIngestOutcome::Consumed
    }

    fn on_permission_evaluated(
        &mut self,
        record: &OpenCodeLogRecord,
        permission: String,
        pattern: String,
        action: String,
    ) -> StderrIngestOutcome {
        let mut extra_map = base_extra(record, "permission_evaluated");
        extra_map.insert("permission".into(), Value::String(permission.clone()));
        extra_map.insert("pattern".into(), Value::String(pattern.clone()));
        extra_map.insert("action".into(), Value::String(action.clone()));

        let rendered_message = format_permission_message(&permission, &pattern, &action);
        self.sink.on_semantic_event(SemanticEvent::Info {
            message: rendered_message,
            extra: Value::Object(extra_map),
        });
        StderrIngestOutcome::Consumed
    }

    fn on_http_response(
        &mut self,
        record: &OpenCodeLogRecord,
        method: String,
        url: String,
        status: u16,
        duration_ms: u64,
    ) -> StderrIngestOutcome {
        let mut extra_map = base_extra(record, "http_response");
        extra_map.insert("method".into(), Value::String(method.clone()));
        extra_map.insert("url".into(), Value::String(url.clone()));
        extra_map.insert("status".into(), json!(status));
        extra_map.insert("duration_ms".into(), json!(duration_ms));

        let rendered_message = format_http_response_message(&method, &url, status, duration_ms);
        self.sink.on_semantic_event(SemanticEvent::Info {
            message: rendered_message,
            extra: Value::Object(extra_map),
        });
        StderrIngestOutcome::Consumed
    }

    /// Emit a Warning describing a `service=snapshot` log line at WARN or
    /// ERROR level. Tag values such as `file=`, `files=`, `path=`, `error=`,
    /// and `err=` are surfaced alongside the message so operators have
    /// enough context to act on snapshot subsystem failures.
    ///
    /// INFO and DEBUG snapshot lines (routine maintenance — `taking snapshot`,
    /// `prune=7.days cleanup`, etc.) are intentionally silent: they're
    /// counted as parsed structured records and the byte heartbeat fires,
    /// but no semantic event is emitted because they would dominate the
    /// rendered output with no user-actionable information.
    fn on_snapshot(
        &mut self,
        record: &OpenCodeLogRecord,
        message: String,
        level: LogLevel,
    ) -> StderrIngestOutcome {
        if !matches!(level, LogLevel::Warn | LogLevel::Error) {
            return StderrIngestOutcome::Consumed;
        }

        let mut extra_map = base_extra(record, "snapshot");
        let tag_summary = summarize_snapshot_tags(record, &mut extra_map);
        extra_map.insert(
            "level".into(),
            Value::String(format!("{level:?}").to_uppercase()),
        );

        let rendered_message = format_snapshot_message(&message, &tag_summary);
        self.sink.on_semantic_event(SemanticEvent::Warning {
            message: rendered_message,
            extra: Value::Object(extra_map),
        });
        StderrIngestOutcome::Consumed
    }
}


#[cfg(test)]
mod tests;
