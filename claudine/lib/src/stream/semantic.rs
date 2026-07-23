//! Semantic stream event model.
//!
//! A provider-agnostic, typed event model that represents both output deltas
//! (assistant text, reasoning) and structured metadata (tool calls, subagents,
//! file changes, plan updates, diagnostics) emitted during a wrapped
//! non-interactive session.
//!
//! Every successfully-parsed JSONL line from a provider becomes exactly one
//! `SemanticEvent`. Typed variants carry their own fields plus an `extra`
//! [`serde_json::Value`] object for provider-specific spillover. Unknown but
//! parseable events become [`SemanticEvent::ProviderExtension`] so nothing is
//! dropped.
//!
//! Design context lives in `claudine/features/2026-04-14-more-meta-response/`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::token_usage::NormalizedTokenUsage;
use crate::provider_id::Provider;
use crate::stream::logs::opencode::bridge::StalledGenerationProgress;

/// Typed classification of an error surfaced through [`SemanticEvent::Error`].
///
/// Carries enough information for downstream renderers (live stderr surface,
/// end-of-run error report) to choose a label and color without re-parsing
/// the underlying error message. Replays of older JSONL streams that lack a
/// `kind` field deserialize as [`SemanticErrorKind::Unknown`] via the
/// `#[serde(default)]` attribute on [`SemanticEvent::Error::kind`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SemanticErrorKind {
    /// Local configuration problem (missing API key, bad config file, etc.).
    Configuration,
    /// Native error reported by the agent CLI itself (model not found,
    /// invalid CLI usage, etc.).
    AgentNative,
    /// Error originating from a remote API the agent talks to (rate limits,
    /// billing, upstream API failures).
    ApiRemote,
    /// User or signal-driven interruption (Ctrl-C, SIGTERM).
    Interrupted,
    /// Unclassified error. Default for replay compatibility.
    #[default]
    Unknown,
}

impl SemanticErrorKind {
    /// Stable lowercase identifier for logging and serialization tags.
    pub fn as_str(self) -> &'static str {
        match self {
            SemanticErrorKind::Configuration => "configuration",
            SemanticErrorKind::AgentNative => "agent_native",
            SemanticErrorKind::ApiRemote => "api_remote",
            SemanticErrorKind::Interrupted => "interrupted",
            SemanticErrorKind::Unknown => "unknown",
        }
    }
}

/// Typed cross-provider stream event.
///
/// See the module docs for the fidelity invariants. In short: every typed
/// variant carries an `extra` JSON object for provider-specific fields, and
/// [`SemanticEvent::ProviderExtension`] is the catch-all for kinds that have
/// not (yet) graduated to a typed variant.
///
/// `strum::VariantNames` supplies the PascalCase variant-name set the render
/// dispatch table checks itself against (`render::event_renderer::DISPATCH`),
/// so a new variant that lacks a dispatch entry fails a completeness test
/// rather than silently rendering nothing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum::VariantNames)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SemanticEvent {
    SessionStart {
        session_id: Option<String>,
        model: Option<String>,
        extra: Value,
    },
    TurnStart {
        extra: Value,
    },
    TurnComplete {
        provider_status: Option<String>,
        token_usage: Option<NormalizedTokenUsage>,
        cost_usd: Option<f64>,
        duration_ms: Option<u64>,
        extra: Value,
    },
    OutputText {
        text: String,
        extra: Value,
    },
    Reasoning {
        text: String,
        extra: Value,
    },
    ToolCall {
        name: Option<String>,
        id: Option<String>,
        input: Option<Value>,
        extra: Value,
    },
    ToolResult {
        name: Option<String>,
        id: Option<String>,
        status: Option<String>,
        exit_code: Option<i32>,
        output: Option<Value>,
        extra: Value,
    },
    PermissionRequest {
        kind: Option<String>,
        tool_name: Option<String>,
        extra: Value,
    },
    SubagentStart {
        name: Option<String>,
        id: Option<String>,
        extra: Value,
    },
    SubagentStop {
        name: Option<String>,
        id: Option<String>,
        status: Option<String>,
        extra: Value,
    },
    FileChange {
        path: Option<String>,
        change_kind: Option<String>,
        extra: Value,
    },
    PlanUpdate {
        message: Option<String>,
        extra: Value,
    },
    Info {
        message: String,
        extra: Value,
    },
    Warning {
        message: String,
        extra: Value,
    },
    Error {
        message: String,
        terminal: bool,
        #[serde(default)]
        kind: SemanticErrorKind,
        extra: Value,
    },
    ProviderExtension {
        provider: Provider,
        kind: String,
        payload: Value,
    },
}

impl SemanticEvent {
    /// Stable short identifier for the variant (used for logging, metrics,
    /// and the `extra.semantic_kind` tag in JSONL/SQLite reporting).
    pub fn kind_str(&self) -> &'static str {
        match self {
            SemanticEvent::SessionStart { .. } => "session_start",
            SemanticEvent::TurnStart { .. } => "turn_start",
            SemanticEvent::TurnComplete { .. } => "turn_complete",
            SemanticEvent::OutputText { .. } => "output_text",
            SemanticEvent::Reasoning { .. } => "reasoning",
            SemanticEvent::ToolCall { .. } => "tool_call",
            SemanticEvent::ToolResult { .. } => "tool_result",
            SemanticEvent::PermissionRequest { .. } => "permission_request",
            SemanticEvent::SubagentStart { .. } => "subagent_start",
            SemanticEvent::SubagentStop { .. } => "subagent_stop",
            SemanticEvent::FileChange { .. } => "file_change",
            SemanticEvent::PlanUpdate { .. } => "plan_update",
            SemanticEvent::Info { .. } => "info",
            SemanticEvent::Warning { .. } => "warning",
            SemanticEvent::Error { .. } => "error",
            SemanticEvent::ProviderExtension { .. } => "provider_extension",
        }
    }

    /// Whether this event counts as "activity" for heartbeat silence tracking.
    ///
    /// Session/turn envelope events are not counted; actual model work
    /// (output, reasoning, tools, subagents, diagnostics, extensions) is.
    pub fn is_activity(&self) -> bool {
        matches!(
            self,
            SemanticEvent::OutputText { .. }
                | SemanticEvent::Reasoning { .. }
                | SemanticEvent::ToolCall { .. }
                | SemanticEvent::ToolResult { .. }
                | SemanticEvent::SubagentStart { .. }
                | SemanticEvent::SubagentStop { .. }
                | SemanticEvent::FileChange { .. }
                | SemanticEvent::PlanUpdate { .. }
                | SemanticEvent::Info { .. }
                | SemanticEvent::Warning { .. }
                | SemanticEvent::Error { .. }
                | SemanticEvent::ProviderExtension { .. }
        )
    }

    /// Whether this event is **progress-class** for the OpenCode
    /// stalled-generation backstop — genuine forward motion that resets the
    /// progress-silence clock and generation churn counter.
    ///
    /// Deliberately narrower than [`Self::is_activity`]: the live-but-dead guard
    /// keys on forward *progress*, not liveness. `Info`/`Warning`/`Error`
    /// diagnostics (which include OpenCode's repeated `llm_call_start` heartbeat)
    /// and the session/turn envelope are liveness-only and must **not** reset the
    /// guard, otherwise the noisy-but-dead retry loop would refresh its own
    /// silence clock forever. This is a separate predicate so it can be tightened
    /// without perturbing `step_timeout`'s `is_activity` silence rule.
    ///
    /// Matches the spec's stdout reset taxonomy: new model output
    /// (`OutputText`, `Reasoning`), tool lifecycle (`ToolCall`, `ToolResult`),
    /// subagent lifecycle (`SubagentStart`, `SubagentStop`), and artifact motion
    /// (`FileChange`, `PlanUpdate`).
    pub fn is_stdout_progress_class(&self) -> bool {
        matches!(
            self,
            SemanticEvent::OutputText { .. }
                | SemanticEvent::Reasoning { .. }
                | SemanticEvent::ToolCall { .. }
                | SemanticEvent::ToolResult { .. }
                | SemanticEvent::SubagentStart { .. }
                | SemanticEvent::SubagentStop { .. }
                | SemanticEvent::FileChange { .. }
                | SemanticEvent::PlanUpdate { .. }
        )
    }
}

/// Sink interface for stream parsers.
///
/// Every successfully-parsed JSONL line produces exactly one [`SemanticEvent`]
/// delivered via `on_semantic_event`. Malformed JSON is surfaced as
/// [`SemanticEvent::Warning`] rather than propagated as an error.
pub trait SemanticEventSink: Send {
    fn on_semantic_event(&mut self, event: SemanticEvent);
}

impl<S: SemanticEventSink + ?Sized> SemanticEventSink for Box<S> {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        (**self).on_semantic_event(event);
    }
}

/// No-op sink that discards all events.
pub struct NullSemanticSink;

impl SemanticEventSink for NullSemanticSink {
    fn on_semantic_event(&mut self, _event: SemanticEvent) {}
}

/// Thread-safe sink wrapper backed by `Arc<Mutex<S>>`.
///
/// Lets multiple producers (stdout parser, stderr log bridge) share a single
/// downstream sink without rewriting it for lock-free fanout. Events are
/// serialized by the mutex so the downstream sink always sees one event at a
/// time. Cloning produces another handle to the same inner sink; dropping a
/// clone does not affect the others.
pub struct SharedSemanticSink<S: SemanticEventSink> {
    inner: Arc<Mutex<S>>,
}

impl<S: SemanticEventSink> SharedSemanticSink<S> {
    /// Wrap a sink so it can be shared across threads.
    pub fn new(sink: S) -> Self {
        Self {
            inner: Arc::new(Mutex::new(sink)),
        }
    }

    pub fn inner(&self) -> &Arc<Mutex<S>> {
        &self.inner
    }
}

impl<S: SemanticEventSink> Clone for SharedSemanticSink<S> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<S: SemanticEventSink> SemanticEventSink for SharedSemanticSink<S> {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.on_semantic_event(event);
    }
}

/// Sink wrapper that flips a shared `AtomicBool` the first time any stdout
/// semantic event is forwarded.
///
/// Used by the OpenCode structured wrapper path to tell stderr-derived
/// diagnostics whether stdout activity has already begun — for example, to
/// decide whether a rate-limit classification should emit a
/// [`SemanticEvent::Warning`] (stdout already seen) or
/// [`SemanticEvent::Error`] with early-termination (no stdout activity yet).
pub struct ObservedSemanticSink<S: SemanticEventSink> {
    inner: S,
    stdout_event_seen: Arc<AtomicBool>,
}

impl<S: SemanticEventSink> ObservedSemanticSink<S> {
    /// Build a new observed wrapper around `sink` that sets
    /// `stdout_event_seen` the first time any semantic event is forwarded.
    pub fn new(sink: S, stdout_event_seen: Arc<AtomicBool>) -> Self {
        Self {
            inner: sink,
            stdout_event_seen,
        }
    }

    /// Clone handle into the shared `stdout_event_seen` gate so other
    /// producers (for example the stderr log bridge) can read the flag.
    pub fn stdout_event_seen(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stdout_event_seen)
    }
}

impl<S: SemanticEventSink> SemanticEventSink for ObservedSemanticSink<S> {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        self.stdout_event_seen.store(true, Ordering::SeqCst);
        self.inner.on_semantic_event(event);
    }
}

/// Sink wrapper that resets the OpenCode stalled-generation progress clocks
/// whenever a progress-class stdout semantic event is forwarded.
///
/// The live-but-dead guard's counters live in the stderr
/// [`OpenCodeLogBridge`](crate::stream::logs::opencode::OpenCodeLogBridge), but
/// genuine forward progress also arrives on stdout (`OutputText`, `ToolCall`,
/// …) on a different thread. Without resetting from the stdout side, a run that
/// makes real stdout progress and then emits another `llm_call_start` could
/// trip the guard even though it was progressing. This observer closes that gap:
/// built from the bridge's shared progress cell, it calls
/// [`StalledGenerationProgress::mark_progress`] for each progress-class event
/// (per [`SemanticEvent::is_stdout_progress_class`]) before forwarding it.
///
/// Liveness-only stdout events (`Info`/`Warning`/`Error`, the session/turn
/// envelope) are forwarded untouched, so the guard still only resets on real
/// progress. The progress lock is dropped before forwarding, so it never nests
/// with the downstream semantic-sink lock.
pub struct StalledProgressObserverSink<S: SemanticEventSink> {
    inner: S,
    progress: Arc<Mutex<StalledGenerationProgress>>,
}

impl<S: SemanticEventSink> StalledProgressObserverSink<S> {
    /// Wrap `sink` so progress-class stdout events reset the shared
    /// stalled-generation progress cell before they are forwarded downstream.
    pub fn new(sink: S, progress: Arc<Mutex<StalledGenerationProgress>>) -> Self {
        Self {
            inner: sink,
            progress,
        }
    }
}

impl<S: SemanticEventSink> SemanticEventSink for StalledProgressObserverSink<S> {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        if event.is_stdout_progress_class() {
            // Short critical section: stamp progress, then release before any
            // downstream sink call so this lock never nests with the shared
            // semantic-sink mutex.
            self.progress
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .mark_progress(Instant::now());
        }
        self.inner.on_semantic_event(event);
    }
}

#[cfg(test)]
mod tests;
