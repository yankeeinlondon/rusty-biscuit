//! Bridge: feed classified stderr log lines into the shared semantic sink,
//! maintain stderr-side summary counters, and signal early termination when a
//! pre-stream usage-cap error is detected.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use tracing::{debug, warn};

use crate::stream::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use crate::stream::summary::{RateLimitInfo, StderrDiagnostics};

use crate::stream::logs::opencode::errors::{
    asset_type_as_str, classify, classify_raw, get_http_status_description, max_reset_at,
    merge_rate_limit, render_malformed_asset_message, render_rate_limit_message, strip_ansi,
};
use crate::stream::logs::opencode::events::{
    AssetType, LogClassification, LogLevel, OpenCodeLogRecord, ParsedOpenCodeStderrLine,
    ProviderLimitKind,
};

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
    /// standard `handle_timeout` failure handler runs. The synthesized
    /// summary marks `error_kind = "timeout"`.
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
}

/// Shared stderr-side state accumulated by the bridge as it parses lines.
///
/// Held behind a `Mutex` so the bridge can be cloned cheaply across threads
/// and the main wait loop can read the accumulated diagnostics at the end
/// of the run. The bridge never mutates [`crate::stream::summary::StreamExecutionSummary`]
/// directly; merging is the wrapper layer's responsibility.
#[derive(Debug, Default)]
pub struct SharedStderrState {
    pub diagnostics: StderrDiagnostics,
    pub rate_limit: Option<RateLimitInfo>,
    /// First `providerID` observed on a `service=llm ... mode=primary`
    /// stderr log line. OpenCode routes the primary turn through a single
    /// model provider per session, so the first observation is canonical.
    pub primary_provider_id: Option<String>,
    /// First `modelID` observed on a `service=llm ... mode=primary` stderr
    /// log line. Used to enrich [`crate::stream::summary::StreamExecutionSummary::model`]
    /// when the stdout NDJSON stream does not include an `init` payload.
    pub primary_model_id: Option<String>,
}

impl SharedStderrState {
    /// Convenience accessor: does the shared state contain any parsed
    /// structured log records?
    pub fn any_records(&self) -> bool {
        self.diagnostics.log_records_parsed > 0
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
}

/// Bookkeeping for an OpenCode child session observed on the stderr stream.
///
/// Created when a `service=session ... parentID=...` record is classified
/// as [`LogClassification::SessionCreated`] with a non-empty `parent_id`.
/// `stopped` flips to `true` the first time the child's `exiting loop`
/// fires so subsequent `StepExit` records do not emit duplicate
/// [`SemanticEvent::SubagentStop`] events for the same child.
#[derive(Debug, Clone, Default)]
struct ChildSessionInfo {
    parent_id: String,
    name: Option<String>,
    stopped: bool,
}

impl<S: SemanticEventSink> OpenCodeLogBridge<S> {
    /// Build a new bridge wired to a shared sink, an observation gate, and
    /// an optional early-termination channel.
    pub fn new(
        sink: S,
        stdout_event_seen: Arc<AtomicBool>,
        early_terminate: Option<Sender<EarlyTermination>>,
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
        }
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
            LogClassification::UncaughtError { raw_text } => self.on_uncaught_error(raw_text, None),
            _ => StderrIngestOutcome::NotConsumed,
        }
    }

<<<<<<< Updated upstream
    #[allow(clippy::too_many_arguments)]
    fn on_provider_limit(
||||||| Stash base
    #[allow(clippy::too_many_arguments)]
    fn on_rate_limit(
=======
    fn on_provider_limit(
>>>>>>> Stashed changes
        &mut self,
        record: &OpenCodeLogRecord,
        status_code: u16,
        kind: ProviderLimitKind,
        reset_at: Option<DateTime<Utc>>,
        provider_id: Option<String>,
        model_id: Option<String>,
        provider_error: String,
    ) -> StderrIngestOutcome {
<<<<<<< Updated upstream
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
||||||| Stash base
        let stdout_seen = self.stdout_event_seen.load(Ordering::SeqCst);
        let rendered_message = render_rate_limit_message(provider_id, model_id, reset_at);
=======
        let stdout_seen = self.stdout_event_seen.load(Ordering::SeqCst);
        let rendered_message = render_rate_limit_message(provider_id, model_id, reset_at);
        let is_terminal = matches!(
            kind,
            ProviderLimitKind::UsageCap | ProviderLimitKind::RetriesExhausted
        );
>>>>>>> Stashed changes

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
        extra_map.insert("status_code".into(), json!(status_code));
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

<<<<<<< Updated upstream
        if is_terminal {
||||||| Stash base
        if stdout_seen || !is_fatal {
=======
        if stdout_seen || !is_terminal {
>>>>>>> Stashed changes
            debug!(
                status_code,
                kind = ?kind,
                reset_at = ?reset_at,
<<<<<<< Updated upstream
                "opencode provider-limit classified as terminal; requesting early termination",
||||||| Stash base
                is_fatal,
                "opencode rate-limit classified after stdout activity or non-fatal; emitting warning",
            );
            self.sink.on_semantic_event(SemanticEvent::Warning {
                message: rendered_message,
                extra: Value::Object(extra_map),
            });
        } else {
            debug!(
                status_code,
                error_name = %error_name,
                reset_at = ?reset_at,
                is_fatal,
                "opencode rate-limit classified before any stdout activity and fatal; requesting early termination",
=======
                "opencode provider-limit classified after stdout activity or non-terminal; emitting warning",
            );
            self.sink.on_semantic_event(SemanticEvent::Warning {
                message: rendered_message,
                extra: Value::Object(extra_map),
            });
        } else {
            debug!(
                status_code,
                kind = ?kind,
                reset_at = ?reset_at,
                "opencode provider-limit classified before any stdout activity and terminal; requesting early termination",
>>>>>>> Stashed changes
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

    fn on_session_created(
        &mut self,
        record: &OpenCodeLogRecord,
        id: String,
        parent_id: Option<String>,
    ) -> StderrIngestOutcome {
        // The bare-value body parser absorbs the trailing ` created`
        // keyword into the last tag's value; in practice that tag is
        // `title=`, so strip the suffix before exposing the title to
        // consumers.
        let title = record
            .tags
            .get("title")
            .map(|s| s.strip_suffix(" created").unwrap_or(s).to_string())
            .filter(|s| !s.is_empty());
        let mut extra_map = base_extra(record, "session_created");
        extra_map.insert("session_id".into(), Value::String(id.clone()));
        if let Some(ref parent) = parent_id {
            extra_map.insert("parent_id".into(), Value::String(parent.clone()));
        }
        if let Some(ref title) = title {
            extra_map.insert("title".into(), Value::String(title.clone()));
        }

        match parent_id {
            Some(parent) => {
                self.child_sessions.insert(
                    id.clone(),
                    ChildSessionInfo {
                        parent_id: parent,
                        name: title.clone(),
                        stopped: false,
                    },
                );
                self.sink.on_semantic_event(SemanticEvent::SubagentStart {
                    name: title,
                    id: Some(id),
                    extra: Value::Object(extra_map),
                });
            }
            None => {
                if self.primary_session_emitted {
                    debug!(
                        session_id = %id,
                        "opencode primary SessionStart already emitted on stderr; skipping duplicate",
                    );
                    return StderrIngestOutcome::Consumed;
                }
                // Cross-stream dedup: if the stdout NDJSON parser has
                // already emitted any semantic event (its `init` /
                // `session_start` payload most likely), the stderr-derived
                // primary SessionStart would be a redundant duplicate.
                // First arrival wins, the stderr session_created is
                // recorded but not re-emitted.
                if self.stdout_event_seen.load(Ordering::SeqCst) {
                    debug!(
                        session_id = %id,
                        "opencode stdout already emitted a semantic event; skipping stderr primary SessionStart",
                    );
                    self.primary_session_emitted = true;
                    return StderrIngestOutcome::Consumed;
                }
                self.primary_session_emitted = true;
                self.sink.on_semantic_event(SemanticEvent::SessionStart {
                    session_id: Some(id),
                    model: None,
                    extra: Value::Object(extra_map),
                });
            }
        }

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
        StderrIngestOutcome::Consumed
    }

    fn on_step_loop(
        &mut self,
        record: &OpenCodeLogRecord,
        session_id: String,
        step: u32,
    ) -> StderrIngestOutcome {
        // OpenCode emits a `service=session.prompt … loop` record at every
        // HTTP-span boundary inside the same step. Suppress repeats so only
        // genuine step transitions surface.
        if self.last_step_per_session.get(&session_id) == Some(&step) {
            return StderrIngestOutcome::Consumed;
        }
        self.last_step_per_session
            .insert(session_id.clone(), step);

        let mut extra_map = base_extra(record, "step_loop");
        extra_map.insert("session_id".into(), Value::String(session_id.clone()));
        extra_map.insert("step".into(), json!(step));

        self.sink.on_semantic_event(SemanticEvent::Info {
            message: format!("step_loop step={step} session={session_id}"),
            extra: Value::Object(extra_map),
        });
        StderrIngestOutcome::Consumed
    }

    fn on_step_exit(
        &mut self,
        record: &OpenCodeLogRecord,
        session_id: String,
    ) -> StderrIngestOutcome {
        // Drop the dedup entry so a follow-up prompt on the same session
        // starts fresh.
        self.last_step_per_session.remove(&session_id);

        let mut extra_map = base_extra(record, "step_exit");
        extra_map.insert("session_id".into(), Value::String(session_id.clone()));

        self.sink.on_semantic_event(SemanticEvent::Info {
            message: format!("exiting_loop session={session_id}"),
            extra: Value::Object(extra_map),
        });

        // If this session was registered as a subagent child, emit a
        // matching SubagentStop. Only fire once per child session so we
        // maintain a 1:1 SubagentStart/SubagentStop pairing even though
        // OpenCode emits an `exiting loop` record at the end of every
        // step within a session.
        if let Some(child) = self.child_sessions.get_mut(&session_id)
            && !child.stopped
        {
            child.stopped = true;
            let name = child.name.clone();
            let parent_id = child.parent_id.clone();
            let mut subagent_extra = Map::new();
            subagent_extra.insert("provider".into(), Value::String("opencode".into()));
            subagent_extra.insert("source".into(), Value::String("stderr_log".into()));
            subagent_extra.insert(
                "classification".into(),
                Value::String("subagent_stop".into()),
            );
            subagent_extra.insert("session_id".into(), Value::String(session_id.clone()));
            subagent_extra.insert("parent_id".into(), Value::String(parent_id));
            if let Some(ref n) = name {
                subagent_extra.insert("name".into(), Value::String(n.clone()));
            }
            self.sink.on_semantic_event(SemanticEvent::SubagentStop {
                name,
                id: Some(session_id),
                status: None,
                extra: Value::Object(subagent_extra),
            });
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

    fn fire_early_termination(&mut self, termination: EarlyTermination) {
        if self.early_terminate_fired {
            return;
        }
        self.early_terminate_fired = true;
        let Some(sender) = self.early_terminate.as_ref() else {
            return;
        };
        if let Err(err) = sender.send(termination) {
            warn!(
                error = %err,
                "failed to deliver OpenCode early-termination signal; receiver dropped",
            );
        }
    }
}

/// Render the inline message string for a `service=llm ... stream` event.
/// Example: `llm_call_start anthropic/claude-opus-4-7 (mode=primary, agent=build)`.
fn format_llm_call_message(
    provider_id: &str,
    model_id: &str,
    mode: &str,
    agent: Option<&str>,
) -> String {
    let identity = match (provider_id, model_id) {
        ("", "") => "(unknown provider/model)".to_string(),
        ("", model) => model.to_string(),
        (provider, "") => provider.to_string(),
        (provider, model) => format!("{provider}/{model}"),
    };
    let mut parts: Vec<String> = Vec::with_capacity(2);
    if !mode.is_empty() {
        parts.push(format!("mode={mode}"));
    }
    if let Some(agent) = agent.filter(|a| !a.is_empty()) {
        parts.push(format!("agent={agent}"));
    }
    if parts.is_empty() {
        format!("llm_call_start {identity}")
    } else {
        format!("llm_call_start {identity} ({})", parts.join(", "))
    }
}

/// Render the inline message string for a permission evaluation. OpenCode
/// emits the resolved `action` either as a bare value or as a JSON object
/// of the shape `{"permission": "...", "pattern": "...", "action": "..."}`;
/// when JSON, we extract the inner `action` ("allow" / "deny") so the
/// rendered text stays scannable.
fn format_permission_message(permission: &str, pattern: &str, action: &str) -> String {
    let action_short = summarize_permission_action(action);
    let subject = match (permission, pattern) {
        ("", "") => String::new(),
        ("", pat) => pat.to_string(),
        (perm, "") => perm.to_string(),
        (perm, pat) => format!("{perm}:{pat}"),
    };
    match (subject.is_empty(), action_short.is_empty()) {
        (true, true) => "permission_evaluated".to_string(),
        (true, false) => format!("permission_evaluated → {action_short}"),
        (false, true) => format!("permission_evaluated {subject}"),
        (false, false) => format!("permission_evaluated {subject} → {action_short}"),
    }
}

fn summarize_permission_action(action: &str) -> String {
    let trimmed = action.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('{')
        && let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed)
        && let Some(Value::String(inner)) = map.get("action")
    {
        return inner.clone();
    }
    trimmed.to_string()
}

/// Render the inline message string for a `Sent HTTP response` event.
fn format_http_response_message(method: &str, url: &str, status: u16, duration_ms: u64) -> String {
    let duration = if duration_ms == 0 {
        String::new()
    } else {
        format!(" ({duration_ms}ms)")
    };
    let target = match (method.is_empty(), url.is_empty()) {
        (true, true) => String::new(),
        (true, false) => url.to_string(),
        (false, true) => method.to_string(),
        (false, false) => format!("{method} {url}"),
    };
    let status_part = if status == 0 {
        String::new()
    } else {
        format!(" {status}")
    };
    if target.is_empty() && status == 0 {
        "http_response".to_string()
    } else if target.is_empty() {
        format!("http_response{status_part}{duration}")
    } else {
        format!("http_response {target}{status_part}{duration}")
    }
}

/// Render the inline message string for a snapshot subsystem log line.
/// `tag_summary` is a one-line, comma-joined view of the most informative
/// non-control tags on the record.
fn format_snapshot_message(message: &str, tag_summary: &str) -> String {
    let base = if message.is_empty() {
        "snapshot".to_string()
    } else {
        format!("snapshot: {message}")
    };
    if tag_summary.is_empty() {
        base
    } else {
        format!("{base} ({tag_summary})")
    }
}

/// Copy snapshot-relevant tags into `extra` and return a comma-joined
/// `key=value` summary suitable for inline rendering. Falls back to all
/// non-`service` tags when the well-known keys are absent.
fn summarize_snapshot_tags(record: &OpenCodeLogRecord, extra: &mut Map<String, Value>) -> String {
    const PREFERRED: &[&str] = &["file", "files", "path", "id", "session.id", "err", "error"];
    let mut parts: Vec<String> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    for key in PREFERRED {
        if let Some(value) = record.tags.get(*key) {
            extra.insert((*key).into(), Value::String(value.clone()));
            parts.push(format!("{key}={}", truncate_for_inline(value)));
            seen.insert((*key).to_string());
        }
    }

    if parts.is_empty() {
        for (key, value) in &record.tags {
            if key == "service" || seen.contains(key) {
                continue;
            }
            extra.insert(key.clone(), Value::String(value.clone()));
            parts.push(format!("{key}={}", truncate_for_inline(value)));
        }
    }

    parts.join(", ")
}

/// Clip a tag value for inline display so a single multi-kilobyte
/// `error=` payload cannot blow up the rendered status line.
fn truncate_for_inline(value: &str) -> String {
    const MAX: usize = 80;
    if value.chars().count() <= MAX {
        return value.to_string();
    }
    let head: String = value.chars().take(MAX).collect();
    format!("{head}…")
}

fn base_extra(record: &OpenCodeLogRecord, classification: &str) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("provider".into(), Value::String("opencode".into()));
    map.insert("source".into(), Value::String("stderr_log".into()));
    map.insert(
        "classification".into(),
        Value::String(classification.into()),
    );
    map.insert("raw".into(), Value::String(record.raw.clone()));
    if let Some(service) = record.tags.get("service") {
        map.insert("service".into(), Value::String(service.clone()));
    }
    map
}

/// Merge the bridge's accumulated [`SharedStderrState`] into a summary.
///
/// Called once by the wrapper layer after the stderr thread has joined.
/// Always sets `summary.stderr_diagnostics` when the bridge parsed at least
/// one structured log record. Always merges `summary.rate_limit` when the
/// bridge accumulated stderr-side rate-limit state. Backfills
/// `summary.model` from the first observed `mode=primary` LLM call when the
/// stdout NDJSON stream did not surface a model. Always recomputes
/// `summary.badges` via [`crate::stream::badges::derive_badges`] so the
/// stderr-derived badge categories (rate-limit resets, malformed-asset
/// warnings) appear in the final output.
pub fn merge_stderr_state_into_summary(
    state: &std::sync::Arc<std::sync::Mutex<SharedStderrState>>,
    summary: &mut crate::stream::summary::StreamExecutionSummary,
) {
    let Ok(state) = state.lock() else {
        return;
    };
    if state.any_records() {
        summary.stderr_diagnostics = Some(state.diagnostics.clone());
    }
    if let Some(stderr_rl) = state.rate_limit.clone() {
        summary.rate_limit = Some(merge_rate_limit(summary.rate_limit.clone(), stderr_rl));
    }
    if summary.model.is_none()
        && let Some(model) = state.primary_model_id.clone()
    {
        summary.model = Some(model);
    }
    drop(state);
    summary.badges = crate::stream::badges::derive_badges(summary, summary.provider);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingSink {
        events: Vec<SemanticEvent>,
    }

    impl SemanticEventSink for RecordingSink {
        fn on_semantic_event(&mut self, event: SemanticEvent) {
            self.events.push(event);
        }
    }

    fn stdout_seen() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(true))
    }

    fn stdout_unseen() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    fn assert_string(extra: &Value, key: &str, expected: &str) {
        let actual = extra
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("missing {key} in extra: {extra}"));
        assert_eq!(actual, expected, "extra.{key} mismatch: {extra}");
    }

    #[test]
    fn unclassified_structured_line_is_consumed_without_emitting_event() {
        // Structured OpenCode log records are owned by the bridge — even
        // when they don't classify into a promoted semantic event we
        // suppress the raw line so it doesn't leak to the user's terminal
        // as debug output.
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let outcome = bridge.ingest("INFO 2026-04-15T21:28:30 +0ms service=default msg=hello");
        assert_eq!(outcome, StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 0);
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.log_records_parsed, 1);
    }

    #[test]
    fn unstructured_noise_returns_not_consumed_and_is_not_counted() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let outcome = bridge.ingest("just some chatter");
        assert_eq!(outcome, StderrIngestOutcome::NotConsumed);
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.log_records_parsed, 0);
        assert_eq!(state.diagnostics.uncaught_errors, 0);
    }

    #[test]
    fn malformed_command_emits_warning_and_consumes() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "ERROR 2026-04-15T21:28:30 +315ms service=config command=/tmp/foo.md err=ENOENT failed to load command";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Warning { message, extra } => {
                assert!(message.contains("command"), "{message}");
                assert!(message.contains("/tmp/foo.md"), "{message}");
                assert_string(extra, "provider", "opencode");
                assert_string(extra, "source", "stderr_log");
                assert_string(extra, "classification", "malformed_asset");
                assert_string(extra, "asset_type", "command");
                assert_string(extra, "path", "/tmp/foo.md");
                assert_string(extra, "service", "config");
                assert!(
                    extra.get("raw").and_then(Value::as_str).is_some(),
                    "raw must be present in extra: {extra}",
                );
            }
            other => panic!("expected Warning, got {other:?}"),
        }
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.malformed_asset_events, 1);
    }

    #[test]
    fn usage_cap_after_stdout_emits_terminal_error_and_early_terminate() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), Some(tx));
        let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2026-04-16 04:18:56\"}}"}]}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                extra,
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::ApiRemote);
                assert!(message.to_lowercase().contains("usage limit"), "{message}");
                // We no longer assert the exact timestamp because it's converted to local time
                assert!(message.contains("2026-04-"), "{message}");
                assert_string(extra, "classification", "rate_limit");
<<<<<<< Updated upstream
                assert_string(extra, "kind", "UsageCap");
||||||| Stash base
                assert_string(extra, "error_name", "AI_RetryError");
=======
                assert_string(extra, "kind", "RetriesExhausted");
>>>>>>> Stashed changes
            }
            other => panic!("expected Error, got {other:?}"),
        }
        assert!(
            rx.try_recv().is_ok(),
            "early-termination signal expected for UsageCap even when stdout already seen",
        );
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.rate_limit_events, 1);
        assert!(state.diagnostics.rate_limit_reset_at.is_some());
        assert_eq!(state.rate_limit.as_ref().unwrap().is_throttled, Some(true));
    }

    #[test]
    fn usage_cap_before_stdout_emits_terminal_error_and_early_terminate() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge =
            OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx));
        let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2026-04-16 04:18:56\"}}"}]}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                extra,
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::ApiRemote);
                assert!(message.to_lowercase().contains("usage limit"));
                assert_string(extra, "classification", "rate_limit");
                assert_string(extra, "kind", "RetriesExhausted");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        match rx.try_recv() {
            Ok(EarlyTermination::RateLimit { message, reset_at }) => {
                assert!(message.to_lowercase().contains("usage limit"));
                let reset = reset_at.expect("reset_at should be forwarded");
                assert_eq!(
                    reset.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "2026-04-16 04:18:56",
                );
            }
            other => panic!("expected EarlyTermination::RateLimit, got {other:?}"),
        }
    }

    #[test]
    fn provider_limit_fires_early_termination_only_once() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge =
            OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx));
        let line = r#"ERROR 2026-04-15T19:26:02 +10ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429}]}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert!(rx.try_recv().is_ok(), "first rate-limit fires channel");
        assert!(
            rx.try_recv().is_err(),
            "second rate-limit must not re-fire the channel",
        );
    }

    #[test]
    fn auth_failure_emits_terminal_error() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = r#"ERROR 2026-04-15T19:26:02 +5ms service=llm error={"error":{"name":"AuthenticationError","message":"Invalid API key"}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                extra,
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::ApiRemote);
                assert_string(extra, "classification", "auth_failure");
                assert_eq!(message, "AuthenticationError: Invalid API key");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.auth_failures, 1);
    }

    #[test]
    fn api_failure_emits_warning_if_not_fatal() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","message":"upstream boom","statusCode":500}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Warning { message, extra } => {
                assert_string(extra, "classification", "api_failure");
                assert_string(extra, "error_name", "AI_APICallError");
                assert_eq!(extra.get("status_code"), Some(&json!(500)));
                assert_eq!(
                    message,
                    "AI_APICallError (500: Internal Server Error): upstream boom"
                );
            }
            other => panic!("expected Warning, got {other:?}"),
        }
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.api_failures, 1);
    }

    #[test]
    fn uncaught_structured_error_emits_unknown_error_event() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "ERROR 2026-04-15T21:28:30 +33ms service=default name=TypeError message=U.split is not a function fatal";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                extra,
                ..
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::Unknown);
                assert_string(extra, "classification", "uncaught_error");
                assert_string(extra, "error_name", "TypeError");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.uncaught_errors, 1);
    }

    #[test]
    fn raw_ansi_error_line_emits_unknown_error_event() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "\u{1b}[91m\u{1b}[1mError: \u{1b}[0mUnexpected error, check log file";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                extra,
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::Unknown);
                assert!(
                    !message.contains('\u{1b}'),
                    "ANSI must be stripped: {message}"
                );
                assert_string(extra, "classification", "uncaught_error");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn early_termination_channel_works_without_receiver_dropped_panic() {
        // Receiver is dropped before the bridge ingests; bridge should not
        // panic, just log a warning and continue.
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        drop(rx);
        let mut bridge =
            OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx));
        let line = r#"ERROR 2026-04-15T19:26:02 +10ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[]}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    }

    #[test]
    fn merge_stderr_state_applies_diagnostics_without_emitting_config_badge() {
        use crate::provider_id::Provider;
        use crate::stream::badges::BadgeCategory;
        use crate::stream::summary::StreamExecutionSummary;

        let state = Arc::new(Mutex::new(SharedStderrState {
            diagnostics: StderrDiagnostics {
                log_records_parsed: 3,
                malformed_asset_events: 2,
                ..Default::default()
            },
            rate_limit: None,
            ..Default::default()
        }));
        let mut summary = StreamExecutionSummary {
            provider: Provider::OpenCode,
            ..Default::default()
        };

        merge_stderr_state_into_summary(&state, &mut summary);

        // Per the 2026-04-18 OpenCode reporting contract, the malformed
        // asset counter is preserved on the summary (so JSONL/dashboards
        // can observe it) but the trailer Config badge is removed —
        // each malformed asset is already surfaced as a per-line Warning.
        let diagnostics = summary
            .stderr_diagnostics
            .as_ref()
            .expect("diagnostics should be attached");
        assert_eq!(diagnostics.log_records_parsed, 3);
        assert_eq!(diagnostics.malformed_asset_events, 2);
        assert!(
            !summary
                .badges
                .iter()
                .any(|b| b.category == BadgeCategory::Config),
            "Config trailer badge must NOT be emitted for malformed assets: {:?}",
            summary.badges,
        );
    }

    #[test]
    fn merge_stderr_state_merges_rate_limit_and_yields_rate_limit_badge() {
        use crate::provider_id::Provider;
        use crate::stream::badges::BadgeCategory;
        use crate::stream::summary::StreamExecutionSummary;
        use chrono::TimeZone;

        let reset = Utc.with_ymd_and_hms(2026, 4, 16, 4, 18, 56).unwrap();
        let state = Arc::new(Mutex::new(SharedStderrState {
            diagnostics: StderrDiagnostics {
                log_records_parsed: 1,
                rate_limit_events: 1,
                rate_limit_reset_at: Some(reset),
                ..Default::default()
            },
            rate_limit: Some(RateLimitInfo {
                is_throttled: Some(true),
                retry_after_ms: None,
                message: Some("Usage limit reached".into()),
                reset_at: Some(reset),
            }),
            ..Default::default()
        }));
        let mut summary = StreamExecutionSummary {
            provider: Provider::OpenCode,
            ..Default::default()
        };

        merge_stderr_state_into_summary(&state, &mut summary);

        let rate_limit = summary
            .rate_limit
            .as_ref()
            .expect("rate_limit should be merged onto summary");
        assert_eq!(rate_limit.is_throttled, Some(true));
        assert_eq!(rate_limit.reset_at, Some(reset));
        assert!(
            summary
                .badges
                .iter()
                .any(|b| b.category == BadgeCategory::RateLimit),
            "RateLimit badge should be recomputed after merge",
        );
    }

    #[test]
    fn merge_stderr_state_without_records_does_not_attach_diagnostics() {
        use crate::provider_id::Provider;
        use crate::stream::summary::StreamExecutionSummary;

        let state = Arc::new(Mutex::new(SharedStderrState::default()));
        let mut summary = StreamExecutionSummary {
            provider: Provider::OpenCode,
            ..Default::default()
        };

        merge_stderr_state_into_summary(&state, &mut summary);
        assert!(summary.stderr_diagnostics.is_none());
        assert!(summary.rate_limit.is_none());
        assert!(summary.badges.is_empty());
    }

    #[test]
    fn bus_line_is_consumed_silently_and_counts_as_parsed() {
        // `service=bus` is the noisiest source on the stderr stream
        // (~70-75% of INFO volume) and carries nothing the user cares
        // about. Bus lines are counted in diagnostics but produce no
        // semantic event and must NEVER leak to the user as raw stderr —
        // so the bridge returns `Consumed` to suppress raw passthrough.
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "INFO 2026-04-15T21:28:30 +5ms service=bus msg=internal chatter";
        let outcome = bridge.ingest(line);
        assert_eq!(outcome, StderrIngestOutcome::Consumed);
        assert_eq!(
            bridge.sink.events.len(),
            0,
            "bus lines must not emit semantic events"
        );
        let state = bridge.state.lock().unwrap();
        assert_eq!(
            state.diagnostics.log_records_parsed, 1,
            "bus lines must still be counted as parsed"
        );
    }

    #[test]
    fn non_bus_line_classifies_normally_after_bus_filter() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        // First, a bus line that is silently dropped (but still consumed
        // so it never reaches raw stderr passthrough).
        let bus_line = "INFO 2026-04-15T21:28:30 +5ms service=bus msg=ignored";
        assert_eq!(bridge.ingest(bus_line), StderrIngestOutcome::Consumed);
        // Then, a malformed asset line that should classify normally.
        let real_line = "ERROR 2026-04-15T21:28:30 +315ms service=config command=/tmp/foo.md err=ENOENT failed to load command";
        assert_eq!(bridge.ingest(real_line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.log_records_parsed, 2);
        assert_eq!(state.diagnostics.malformed_asset_events, 1);
    }

    // ------------------------------------------------------------------
    // Phase-3 semantic event promotion
    // ------------------------------------------------------------------

    #[test]
    fn session_created_without_parent_emits_session_start() {
        // Phase 4 dedup gate: bridge emits a primary SessionStart only
        // when stdout has not yet produced any semantic event.
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), None);
        let line = "INFO  2026-05-12T20:00:12 +20ms service=session id=ses_primary slug=happy-panda version=1.14.48 title=New session created";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::SessionStart {
                session_id,
                model,
                extra,
            } => {
                assert_eq!(session_id.as_deref(), Some("ses_primary"));
                assert!(model.is_none());
                assert_string(extra, "classification", "session_created");
                assert_string(extra, "session_id", "ses_primary");
                assert_string(extra, "title", "New session");
            }
            other => panic!("expected SessionStart, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_primary_session_created_is_not_re_emitted() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), None);
        let line = "INFO  2026-05-12T20:00:12 +20ms service=session id=ses_primary title=A created";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        let line2 =
            "INFO  2026-05-12T20:00:13 +21ms service=session id=ses_primary_other title=B created";
        assert_eq!(bridge.ingest(line2), StderrIngestOutcome::Consumed);
        assert_eq!(
            bridge.sink.events.len(),
            1,
            "second primary session-created must be suppressed",
        );
    }

    #[test]
    fn session_created_with_parent_emits_subagent_start() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "INFO  2026-05-12T20:05:26 +1ms service=session id=ses_child slug=lucky-orchid version=1.14.48 parentID=ses_parent title=Count letters in 'banana' created";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::SubagentStart { name, id, extra } => {
                assert_eq!(id.as_deref(), Some("ses_child"));
                assert!(name.is_some());
                assert_string(extra, "classification", "session_created");
                assert_string(extra, "session_id", "ses_child");
                assert_string(extra, "parent_id", "ses_parent");
            }
            other => panic!("expected SubagentStart, got {other:?}"),
        }
        assert!(bridge.child_sessions.contains_key("ses_child"));
    }

    #[test]
    fn llm_call_emits_info_event_with_provider_model_mode() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_a small=false agent=build mode=primary stream";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Info { message, extra } => {
                assert_eq!(
                    message,
                    "llm_call_start kimi-for-coding/k2p6 (mode=primary, agent=build)"
                );
                assert_string(extra, "provider_id", "kimi-for-coding");
                assert_string(extra, "model_id", "k2p6");
                assert_string(extra, "mode", "primary");
                assert_string(extra, "agent", "build");
                assert_string(extra, "session_id", "ses_a");
                assert_eq!(extra.get("is_stream"), Some(&json!(true)));
            }
            other => panic!("expected Info, got {other:?}"),
        }
    }

    #[test]
    fn step_loop_emits_info_event() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "INFO  2026-05-12T20:00:12 +0ms service=session.prompt session.id=ses_a step=3 logSpan.http.span.4=55ms loop";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Info { message, extra } => {
                assert_eq!(message, "step_loop step=3 session=ses_a");
                assert_string(extra, "session_id", "ses_a");
                assert_eq!(extra.get("step"), Some(&json!(3)));
            }
            other => panic!("expected Info, got {other:?}"),
        }
    }

    #[test]
    fn step_loop_dedups_repeated_step_in_same_session() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let same_step = "INFO  2026-05-12T20:00:12 +0ms service=session.prompt session.id=ses_a step=0 logSpan.http.span.4=55ms loop";
        let later_span = "INFO  2026-05-12T20:00:13 +0ms service=session.prompt session.id=ses_a step=0 logSpan.http.span.4=2143ms loop";
        let new_step = "INFO  2026-05-12T20:00:14 +0ms service=session.prompt session.id=ses_a step=1 logSpan.http.span.4=2200ms loop";

        assert_eq!(bridge.ingest(same_step), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.ingest(later_span), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.ingest(new_step), StderrIngestOutcome::Consumed);

        let step_loops: Vec<_> = bridge
            .sink
            .events
            .iter()
            .filter_map(|e| match e {
                SemanticEvent::Info { message, .. } if message.starts_with("step_loop ") => {
                    Some(message.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            step_loops,
            vec![
                "step_loop step=0 session=ses_a".to_string(),
                "step_loop step=1 session=ses_a".to_string(),
            ],
        );
    }

    #[test]
    fn step_loop_dedup_resets_after_step_exit() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let first = "INFO  2026-05-12T20:00:12 +0ms service=session.prompt session.id=ses_a step=0 logSpan.http.span.4=55ms loop";
        let exit = "INFO  2026-05-12T20:00:19 +0ms service=session.prompt session.id=ses_a logSpan.http.span.4=7437ms exiting loop";
        let after = "INFO  2026-05-12T20:01:00 +0ms service=session.prompt session.id=ses_a step=0 logSpan.http.span.5=10ms loop";

        assert_eq!(bridge.ingest(first), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.ingest(exit), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.ingest(after), StderrIngestOutcome::Consumed);

        let step_loops = bridge
            .sink
            .events
            .iter()
            .filter(|e| matches!(e, SemanticEvent::Info { message, .. } if message.starts_with("step_loop ")))
            .count();
        assert_eq!(step_loops, 2, "exit should reset dedup so the next step=0 emits again");
    }

    #[test]
    fn step_exit_for_non_child_session_emits_only_info_event() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "INFO  2026-05-12T20:00:19 +1ms service=session.prompt session.id=ses_a logSpan.http.span.4=7437ms exiting loop";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Info { message, extra } => {
                assert_eq!(message, "exiting_loop session=ses_a");
                assert_string(extra, "session_id", "ses_a");
            }
            other => panic!("expected Info, got {other:?}"),
        }
    }

    #[test]
    fn step_exit_for_child_session_emits_info_then_subagent_stop() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);

        let start = "INFO  2026-05-12T20:05:26 +1ms service=session id=ses_child parentID=ses_parent title=Count created";
        assert_eq!(bridge.ingest(start), StderrIngestOutcome::Consumed);
        let exit = "INFO  2026-05-12T20:05:30 +0ms service=session.prompt session.id=ses_child logSpan.http.span.4=4000ms exiting loop";
        assert_eq!(bridge.ingest(exit), StderrIngestOutcome::Consumed);

        assert_eq!(bridge.sink.events.len(), 3);
        match &bridge.sink.events[0] {
            SemanticEvent::SubagentStart { id, .. } => {
                assert_eq!(id.as_deref(), Some("ses_child"));
            }
            other => panic!("expected SubagentStart first, got {other:?}"),
        }
        match &bridge.sink.events[1] {
            SemanticEvent::Info { message, .. } => {
                assert!(
                    message.starts_with("exiting_loop session="),
                    "expected enriched exiting_loop message, got {message:?}",
                );
            }
            other => panic!("expected Info second, got {other:?}"),
        }
        match &bridge.sink.events[2] {
            SemanticEvent::SubagentStop { id, extra, .. } => {
                assert_eq!(id.as_deref(), Some("ses_child"));
                assert_string(extra, "parent_id", "ses_parent");
                assert_string(extra, "classification", "subagent_stop");
            }
            other => panic!("expected SubagentStop third, got {other:?}"),
        }
    }

    #[test]
    fn step_exit_for_child_session_emits_subagent_stop_only_once() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);

        let start = "INFO  2026-05-12T20:05:26 +1ms service=session id=ses_child parentID=ses_parent title=Count created";
        assert_eq!(bridge.ingest(start), StderrIngestOutcome::Consumed);
        let exit = "INFO  2026-05-12T20:05:30 +0ms service=session.prompt session.id=ses_child logSpan.http.span.4=4000ms exiting loop";
        assert_eq!(bridge.ingest(exit), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.ingest(exit), StderrIngestOutcome::Consumed);

        let subagent_stops = bridge
            .sink
            .events
            .iter()
            .filter(|e| matches!(e, SemanticEvent::SubagentStop { .. }))
            .count();
        assert_eq!(
            subagent_stops, 1,
            "SubagentStop must fire only once per child session",
        );
    }

    #[test]
    fn permission_evaluated_emits_info_event() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = r#"INFO  2026-05-12T20:05:26 +160ms service=permission permission=task pattern=general action={"permission":"*","action":"allow","pattern":"*"} evaluated"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Info { message, extra } => {
                assert_eq!(message, "permission_evaluated task:general → allow");
                assert_string(extra, "permission", "task");
                assert_string(extra, "pattern", "general");
            }
            other => panic!("expected Info, got {other:?}"),
        }
    }

    #[test]
    fn http_response_emits_info_event() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "INFO  2026-05-12T20:05:54 +0ms service=default http.method=POST http.url=/session/x/message http.status=500 logSpan.http.span.4=99ms Sent HTTP response";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Info { message, extra } => {
                assert_eq!(message, "http_response POST /session/x/message 500 (99ms)");
                assert_string(extra, "method", "POST");
                assert_string(extra, "url", "/session/x/message");
                assert_eq!(extra.get("status"), Some(&json!(500)));
                assert_eq!(extra.get("duration_ms"), Some(&json!(99)));
            }
            other => panic!("expected Info, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_warn_line_emits_warning_with_message_context() {
        // OpenCode's stderr body parser absorbs trailing message text into
        // the last bare-valued tag (a known quirk handled elsewhere via
        // `has_trailing_keyword`). Either shape — a clean trailing message
        // OR an absorbed tag value — must surface enough context for the
        // user to know which snapshot subsystem operation failed.
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        // JSON-array `files=[…]` form: parser cleanly separates trailing message.
        let line = r#"WARN  2026-05-12T20:05:26 +0ms service=snapshot session.id=ses_a files=["/repo/.env",".npmrc"] failed to add snapshot files"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Warning { message, extra } => {
                assert!(
                    message.starts_with("snapshot: failed to add snapshot files"),
                    "expected snapshot message prefix, got {message:?}",
                );
                assert!(
                    message.contains("files=") || message.contains("session.id="),
                    "expected tag summary in message, got {message:?}",
                );
                assert_string(extra, "level", "WARN");
                assert_string(extra, "classification", "snapshot");
                assert!(
                    extra.get("files").is_some() || extra.get("session.id").is_some(),
                    "expected files or session.id in extra, got {extra:?}",
                );
            }
            other => panic!("expected Warning, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_warn_line_with_absorbed_trailing_still_carries_context() {
        // Even when the body parser absorbs the trailing message into a
        // bare-valued tag (e.g. `file=/repo/.env failed to add snapshot files`),
        // the rendered Warning must still surface enough information for
        // the operator to know what was being snapshotted — the file path
        // and the absorbed diagnostic both ride along in the tag summary.
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "WARN  2026-05-12T20:05:26 +0ms service=snapshot session.id=ses_a file=/repo/.env failed to add snapshot files";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Warning { message, extra } => {
                assert!(
                    message.contains("/repo/.env")
                        && message.contains("failed to add snapshot files"),
                    "expected file path and diagnostic in message, got {message:?}",
                );
                assert_string(extra, "level", "WARN");
            }
            other => panic!("expected Warning, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_info_line_is_silently_consumed() {
        // Routine snapshot maintenance — INFO/DEBUG `taking snapshot`,
        // `prune=7.days cleanup`, etc. — is parsed and counted but emits
        // no semantic event. They dominate snapshot-line volume and
        // carry no user-actionable signal; only WARN/ERROR-level
        // snapshot lines (`failed to add snapshot files`, etc.) surface
        // as Warning events.
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let lines = [
            r#"INFO  2026-05-12T20:05:26 +0ms service=snapshot id=snap_abc files=["/repo/x.rs"] taking snapshot"#,
            "INFO  2026-05-12T20:05:27 +0ms service=snapshot prune=7.days cleanup",
            "DEBUG 2026-05-12T20:05:28 +0ms service=snapshot id=snap_xyz noop",
        ];
        for line in lines {
            assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        }
        assert_eq!(
            bridge.sink.events.len(),
            0,
            "routine snapshot lines must not emit events; got {:?}",
            bridge.sink.events,
        );
        let state = bridge.state.lock().unwrap();
        assert_eq!(
            state.diagnostics.log_records_parsed, 3,
            "INFO/DEBUG snapshot lines must still count as parsed records"
        );
    }

    #[test]
    fn boot_banner_is_parsed_and_consumed_without_emitting_event() {
        // The boot banner is parsed and counted but Phase 3 deliberately
        // does not promote it to a `SessionStart` (the NDJSON stream's
        // own session event remains the anchor). It still must NOT be
        // proxied to the user's terminal — return `Consumed` so the raw
        // stderr passthrough does not echo it as debug output.
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "INFO  2026-05-12T20:00:11 +97ms service=default version=1.14.48 args=[\"run\"] opencode";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 0);
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.log_records_parsed, 1);
    }

    // ------------------------------------------------------------------
    // Phase-4 cross-stream dedup and summary enrichment
    // ------------------------------------------------------------------

    #[test]
    fn primary_llm_call_captures_provider_and_model_on_first_observation() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_a small=false agent=build mode=primary stream";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);

        let state = bridge.state.lock().unwrap();
        assert_eq!(
            state.primary_provider_id.as_deref(),
            Some("kimi-for-coding")
        );
        assert_eq!(state.primary_model_id.as_deref(), Some("k2p6"));
    }

    #[test]
    fn primary_llm_call_only_captures_first_mode_primary_observation() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let first = "INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_a mode=primary stream";
        let second = "INFO  2026-05-12T20:01:00 +0ms service=llm providerID=anthropic modelID=claude-4 session.id=ses_a mode=primary stream";
        assert_eq!(bridge.ingest(first), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.ingest(second), StderrIngestOutcome::Consumed);

        let state = bridge.state.lock().unwrap();
        assert_eq!(
            state.primary_provider_id.as_deref(),
            Some("kimi-for-coding")
        );
        assert_eq!(state.primary_model_id.as_deref(), Some("k2p6"));
    }

    #[test]
    fn non_primary_llm_call_does_not_capture_provider_and_model() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6-small session.id=ses_a mode=subagent stream";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);

        let state = bridge.state.lock().unwrap();
        assert!(state.primary_provider_id.is_none());
        assert!(state.primary_model_id.is_none());
    }

    #[test]
    fn merge_stderr_state_backfills_summary_model_from_primary_llm_call() {
        use crate::provider_id::Provider;
        use crate::stream::summary::StreamExecutionSummary;

        let state = Arc::new(Mutex::new(SharedStderrState {
            diagnostics: StderrDiagnostics {
                log_records_parsed: 1,
                ..Default::default()
            },
            rate_limit: None,
            primary_provider_id: Some("kimi-for-coding".into()),
            primary_model_id: Some("k2p6".into()),
        }));
        let mut summary = StreamExecutionSummary {
            provider: Provider::OpenCode,
            model: None,
            ..Default::default()
        };

        merge_stderr_state_into_summary(&state, &mut summary);
        assert_eq!(summary.model.as_deref(), Some("k2p6"));
    }

    #[test]
    fn merge_stderr_state_does_not_overwrite_existing_summary_model() {
        use crate::provider_id::Provider;
        use crate::stream::summary::StreamExecutionSummary;

        let state = Arc::new(Mutex::new(SharedStderrState {
            diagnostics: StderrDiagnostics {
                log_records_parsed: 1,
                ..Default::default()
            },
            rate_limit: None,
            primary_provider_id: Some("kimi-for-coding".into()),
            primary_model_id: Some("k2p6".into()),
        }));
        let mut summary = StreamExecutionSummary {
            provider: Provider::OpenCode,
            model: Some("preexisting-model".into()),
            ..Default::default()
        };

        merge_stderr_state_into_summary(&state, &mut summary);
        assert_eq!(
            summary.model.as_deref(),
            Some("preexisting-model"),
            "stdout-derived model must win over the stderr backfill",
        );
    }

    #[test]
    fn primary_session_start_is_suppressed_when_stdout_already_emitted() {
        // Cross-stream dedup: if stdout has already emitted any semantic
        // event, the stderr-derived primary SessionStart is redundant and
        // must be dropped. The `primary_session_emitted` flag is still set
        // so subsequent stderr session_created lines for other ids are
        // also suppressed.
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line =
            "INFO  2026-05-12T20:00:12 +20ms service=session id=ses_primary title=Primary created";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(
            bridge.sink.events.len(),
            0,
            "stderr SessionStart must be suppressed when stdout has already emitted",
        );
        assert!(
            bridge.primary_session_emitted,
            "primary_session_emitted must be set so future duplicates are also skipped",
        );
    }

    #[test]
    fn subagent_start_is_not_dedup_gated_by_stdout_event_seen() {
        // The dedup gate only applies to the primary SessionStart;
        // child session_created lines must always promote to SubagentStart
        // because the stdout NDJSON stream no longer synthesizes them
        // (Phase 4 removed the synthesis path).
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "INFO  2026-05-12T20:05:26 +1ms service=session id=ses_child parentID=ses_parent title=Count created";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        assert!(matches!(
            bridge.sink.events[0],
            SemanticEvent::SubagentStart { .. }
        ));
    }

    #[test]
    fn usage_cap_without_retry_error_still_terminates() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge =
            OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx));

        // This is a 1308 but NOT wrapped in AI_RetryError
        let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm error={"error":{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached.\"}}"}}"#;

        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);

        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                extra,
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::ApiRemote);
                assert!(message.to_lowercase().contains("usage limit"), "{message}");
                assert_string(extra, "classification", "rate_limit");
                assert_string(extra, "kind", "UsageCap");
            }
            other => panic!("expected Error, got {other:?}"),
        }

        assert!(
            rx.try_recv().is_ok(),
            "early-termination signal expected for UsageCap before stdout",
        );
    }

    #[test]
    fn overload_emits_warning_no_early_terminate() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = r#"ERROR 2026-05-15T19:26:02 +3054ms service=llm providerID=kimi-for-coding modelID=k2p6 error={"error":{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"type\":\"rate_limit_error\",\"message\":\"The engine is currently overloaded, please try again later\"}}","isRetryable":true}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Warning { message, extra } => {
                assert_eq!(message, "server overloaded; will retry");
                assert_string(extra, "classification", "rate_limit");
                assert_string(extra, "kind", "Overloaded");
            }
            other => panic!("expected Warning, got {other:?}"),
        }
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.rate_limit_events, 1);
        assert!(
            state.rate_limit.is_none(),
            "overload must not set state.rate_limit"
        );
    }

    #[test]
    fn throttled_emits_warning_no_early_terminate() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","statusCode":429,"message":"Too many requests"}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Warning { message, extra } => {
                assert_eq!(message, "request throttled; will retry");
                assert_string(extra, "classification", "rate_limit");
                assert_string(extra, "kind", "RateLimited");
            }
            other => panic!("expected Warning, got {other:?}"),
        }
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.rate_limit_events, 1);
        assert!(
            state.rate_limit.is_none(),
            "throttle must not set state.rate_limit"
        );
    }

    #[test]
    fn retries_exhausted_emits_terminal_error_and_early_terminate() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge =
            OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx));
        let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429}]}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                extra,
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::ApiRemote);
                assert_eq!(message, "provider 429s did not clear after retries");
                assert_string(extra, "classification", "rate_limit");
                assert_string(extra, "kind", "RetriesExhausted");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        assert!(
            rx.try_recv().is_ok(),
            "early-termination signal expected for RetriesExhausted",
        );
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.rate_limit_events, 1);
        assert!(
            state.rate_limit.is_some(),
            "RetriesExhausted must set state.rate_limit"
        );
    }

    #[test]
    fn exceeded_quota_emits_terminal_error_and_early_terminate() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge =
            OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx));
        let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"type\":\"exceeded_current_quota_error\",\"message\":\"Quota exceeded\"}}"}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                extra,
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::ApiRemote);
                assert!(message.to_lowercase().contains("usage limit"), "{message}");
                assert_string(extra, "classification", "rate_limit");
                assert_string(extra, "kind", "UsageCap");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        assert!(
            rx.try_recv().is_ok(),
            "early-termination signal expected for UsageCap",
        );
    }

    #[test]
    fn cap_phrase_without_error_tag_emits_advisory_warning_no_terminate() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "ERROR 2026-05-15T19:26:02 +100ms service=llm dummy={} Usage limit reached for k2p6";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Warning { message, extra } => {
                assert_eq!(message, "Usage limit reached for k2p6");
                assert_string(extra, "classification", "api_failure");
            }
            other => panic!("expected Warning, got {other:?}"),
        }
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.api_failures, 1);
        assert!(
            state.rate_limit.is_none(),
            "advisory cap must not set state.rate_limit"
        );
    }

    #[test]
    fn cap_wins_over_retries_exhausted_in_bridge() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge =
            OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx));
        let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached\"}}"}]}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                extra,
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::ApiRemote);
                assert!(message.to_lowercase().contains("usage limit"), "{message}");
                assert_string(extra, "classification", "rate_limit");
                assert_string(extra, "kind", "UsageCap");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        assert!(
            rx.try_recv().is_ok(),
            "early-termination signal expected for UsageCap",
        );
    }
}
