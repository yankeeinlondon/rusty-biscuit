//! Bridge: feed classified stderr log lines into the shared semantic sink,
//! maintain stderr-side summary counters, and signal early termination when a
//! pre-stream usage-cap error is detected.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use tracing::{debug, warn};

use crate::stream::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use crate::stream::summary::{RateLimitInfo, StderrDiagnostics};

use crate::stream::logs::opencode::events::{
    AssetType, LogClassification, OpenCodeLogRecord, ParsedOpenCodeStderrLine,
};
use crate::stream::logs::opencode::errors::{
    asset_type_as_str, classify, classify_raw, max_reset_at, merge_rate_limit,
    render_malformed_asset_message, render_rate_limit_message, strip_ansi,
};

/// Whether an incoming stderr log line was converted into a semantic event
/// and should therefore be suppressed from raw stderr passthrough.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StderrIngestOutcome {
    /// The bridge classified and emitted a [`SemanticEvent`] for this line.
    /// Callers should not also echo the raw line to the user.
    Consumed,
    /// The bridge did not recognize the line as a meaningful diagnostic.
    /// Callers should keep the existing raw-passthrough behavior.
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
/// Today this fires for pre-stream usage-cap failures, wrapper-driven
/// post-stop hang recovery in OpenCode's structured non-interactive path,
/// and the unified two-rule timeout watchdog (`timeout` and `step_timeout`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EarlyTermination {
    /// The provider reported a rate-limit failure before any stdout
    /// semantic event was observed.
    RateLimit {
        message: String,
        reset_at: Option<DateTime<Utc>>,
    },
    /// OpenCode reported a terminal stop condition (`reason = "stop"`) but
    /// the process never exited. The wrapper terminates the hung process and
    /// treats the run as successful because the semantic stream had already
    /// finished.
    CompletedButHung { message: String },
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
/// - parse and classify one stderr line at a time via [`parse_line`] +
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

        let classification = classify(&record);
        match classification {
            LogClassification::RateLimit {
                status_code,
                ref error_name,
                reset_at,
                ref provider_id,
                ref model_id,
                ref provider_error,
                is_fatal,
            } => self.on_rate_limit(
                &record,
                status_code,
                error_name.clone(),
                reset_at,
                provider_id.clone(),
                model_id.clone(),
                provider_error.clone(),
                is_fatal,
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
            LogClassification::Unclassified => StderrIngestOutcome::NotConsumed,
        }
    }

    fn handle_raw(&mut self, line: &str) -> StderrIngestOutcome {
        match classify_raw(line) {
            LogClassification::UncaughtError { raw_text } => self.on_uncaught_error(raw_text, None),
            _ => StderrIngestOutcome::NotConsumed,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn on_rate_limit(
        &mut self,
        record: &OpenCodeLogRecord,
        status_code: u16,
        error_name: String,
        reset_at: Option<DateTime<Utc>>,
        provider_id: Option<String>,
        model_id: Option<String>,
        provider_error: String,
        is_fatal: bool,
    ) -> StderrIngestOutcome {
        let stdout_seen = self.stdout_event_seen.load(Ordering::SeqCst);
        let rendered_message = render_rate_limit_message(provider_id, model_id, reset_at);

        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.rate_limit_events =
                state.diagnostics.rate_limit_events.saturating_add(1);
            state.diagnostics.rate_limit_reset_at =
                max_reset_at(state.diagnostics.rate_limit_reset_at, reset_at);
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

        let mut extra_map = base_extra(record, "rate_limit");
        extra_map.insert("status_code".into(), json!(status_code));
        extra_map.insert("error_name".into(), Value::String(error_name.clone()));
        extra_map.insert("is_fatal".into(), json!(is_fatal));
        if let Some(reset) = reset_at {
            extra_map.insert(
                "reset_at".into(),
                Value::String(reset.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            );
        }
        if !provider_error.is_empty() {
            extra_map.insert("provider_error".into(), Value::String(provider_error));
        }

        if stdout_seen || !is_fatal {
            debug!(
                status_code,
                error_name = %error_name,
                reset_at = ?reset_at,
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
            format!("OpenCode API failure ({error_name})")
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
/// bridge accumulated stderr-side rate-limit state. Always recomputes
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
    fn unclassified_structured_line_returns_not_consumed() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let outcome = bridge.ingest("INFO 2026-04-15T21:28:30 +0ms service=default msg=hello");
        assert_eq!(outcome, StderrIngestOutcome::NotConsumed);
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
    fn rate_limit_after_stdout_emits_warning_no_early_terminate() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), Some(tx));
        let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2026-04-16 04:18:56\"}}"}]}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Warning { message, extra } => {
                assert!(message.to_lowercase().contains("usage limit"), "{message}");
                // We no longer assert the exact timestamp because it's converted to local time
                assert!(message.contains("2026-04-"), "{message}");
                assert_string(extra, "classification", "rate_limit");
                assert_string(extra, "error_name", "AI_RetryError");
            }
            other => panic!("expected Warning, got {other:?}"),
        }
        assert!(
            rx.try_recv().is_err(),
            "no early-termination signal expected when stdout already seen",
        );
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.rate_limit_events, 1);
        assert!(state.diagnostics.rate_limit_reset_at.is_some());
        assert_eq!(state.rate_limit.as_ref().unwrap().is_throttled, Some(true));
    }

    #[test]
    fn rate_limit_before_stdout_emits_terminal_error_and_early_terminate() {
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
    fn rate_limit_fires_early_termination_only_once() {
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
                assert_eq!(message, "AI_APICallError (500): upstream boom");
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
    fn rate_limit_without_retry_error_is_warning_even_before_stdout() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge =
            OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx));

        // This is a 1308 but NOT wrapped in AI_RetryError
        let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm error={"error":{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached.\"}}"}}"#;

        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);

        match &bridge.sink.events[0] {
            SemanticEvent::Warning { message, .. } => {
                assert!(message.to_lowercase().contains("usage limit"), "{message}");
            }
            other => panic!("expected Warning, got {other:?}"),
        }

        assert!(
            rx.try_recv().is_err(),
            "early-termination signal NOT expected for non-fatal rate limit",
        );
    }
}
